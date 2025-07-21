use futures::StreamExt;
use tokio::runtime::Runtime;

#[test]
fn test_dockerv2_tags_simple() {
  let name = "repo";
  let tags = r#"{"name": "repo", "tags": [ "t1", "t2" ]}"#;
  let ep = format!("/v2/{name}/tags/list");

  let mut server = mockito::Server::new();
  let addr = server.host_with_port();

  let mock = server
    .mock("GET", ep.as_str())
    .with_status(200)
    .with_header("Content-Type", "application/json")
    .with_body(tags)
    .create();

  let runtime = Runtime::new().unwrap();
  let client = docker_registry::v2::Client::configure()
    .registry(&addr)
    .insecure_registry(true)
    .username(None)
    .password(None)
    .build()
    .unwrap();

  let futcheck = client.get_tags(name, None);

  let res = runtime.block_on(futcheck.map(Result::unwrap).collect::<Vec<_>>());
  mock.assert();
  assert_eq!(res.first().unwrap(), &String::from("t1"));
  assert_eq!(res.get(1).unwrap(), &String::from("t2"));
}

#[test]
fn test_dockerv2_tags_paginate() {
  let name = "repo";
  let tags_p1 = r#"{"name": "repo", "tags": [ "t1" ]}"#;
  let tags_p2 = r#"{"name": "repo", "tags": [ "t2" ]}"#;
  let ep1 = format!("/v2/{name}/tags/list?n=1");
  let ep2 = format!("/v2/{name}/tags/list?n=1&last=t1");

  let mut server = mockito::Server::new();
  let addr = server.host_with_port();

  let mock1 = server
    .mock("GET", ep1.as_str())
    .with_status(200)
    .with_header("Link", &format!(r#"<{addr}/v2/_tags?n=1&last=t1>; rel="next""#))
    .with_header("Content-Type", "application/json")
    .with_body(tags_p1)
    .create();
  let mock2 = server
    .mock("GET", ep2.as_str())
    .with_status(200)
    .with_header("Content-Type", "application/json")
    .with_body(tags_p2)
    .create();

  let runtime = Runtime::new().unwrap();
  let client = docker_registry::v2::Client::configure()
    .registry(&addr)
    .insecure_registry(true)
    .username(None)
    .password(None)
    .build()
    .unwrap();

  let next = Box::pin(client.get_tags(name, Some(1)).map(Result::unwrap));

  let (first_tag, stream_rest) = runtime.block_on(next.into_future());
  assert_eq!(first_tag.unwrap(), "t1".to_owned());

  let (second_tag, stream_rest) = runtime.block_on(stream_rest.into_future());
  assert_eq!(second_tag.unwrap(), "t2".to_owned());

  let (end, _) = runtime.block_on(stream_rest.into_future());
  if end.is_some() {
    panic!("end is some: {end:?}");
  }

  mock1.assert();
  mock2.assert();
}

#[test]
fn test_dockerv2_tags_404() {
  let name = "repo";
  let ep = format!("/v2/{name}/tags/list");

  let mut server = mockito::Server::new();
  let addr = server.host_with_port();

  let mock = server
    .mock("GET", ep.as_str())
    .with_status(404)
    .with_header("Content-Type", "application/json")
    .create();

  let runtime = Runtime::new().unwrap();
  let client = docker_registry::v2::Client::configure()
    .registry(&addr)
    .insecure_registry(true)
    .username(None)
    .password(None)
    .build()
    .unwrap();

  let futcheck = client.get_tags(name, None);

  let res = runtime.block_on(futcheck.collect::<Vec<_>>());
  mock.assert();
  assert!(res.first().unwrap().is_err());
}

#[test]
fn test_dockerv2_tags_missing_header() {
  let name = "repo";
  let tags = r#"{"name": "repo", "tags": [ "t1", "t2" ]}"#;
  let ep = format!("/v2/{name}/tags/list");

  let mut server = mockito::Server::new();
  let addr = server.host_with_port();

  let mock = server
    .mock("GET", ep.as_str())
    .with_status(200)
    .with_body(tags)
    .create();

  let runtime = Runtime::new().unwrap();
  let client = docker_registry::v2::Client::configure()
    .registry(&addr)
    .insecure_registry(true)
    .username(None)
    .password(None)
    .build()
    .unwrap();

  let futcheck = client.get_tags(name, None);

  let res = runtime.block_on(futcheck.map(Result::unwrap).collect::<Vec<_>>());
  mock.assert();
  assert_eq!(vec!["t1", "t2"], res);
}
