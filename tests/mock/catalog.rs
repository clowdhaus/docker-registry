use futures::StreamExt;
use tokio::runtime::Runtime;

#[test]
fn test_catalog_simple() {
  let repos = r#"{"repositories": ["r1/i1", "r2"]}"#;
  let ep = "/v2/_catalog".to_string();

  let mut server = mockito::Server::new();
  let addr = server.host_with_port();

  let mock = server
    .mock("GET", ep.as_str())
    .with_status(200)
    .with_body(repos)
    .create();

  let runtime = Runtime::new().unwrap();
  let client = docker_registry::v2::Client::configure()
    .registry(&addr)
    .insecure_registry(true)
    .username(None)
    .password(None)
    .build()
    .unwrap();

  let futcheck = client.get_catalog(None);
  let res = runtime.block_on(futcheck.map(Result::unwrap).collect::<Vec<_>>());

  mock.assert();
  assert_eq!(res, vec!["r1/i1", "r2"]);
}

#[test]
fn test_catalog_paginate() {
  let repos_p1 = r#"{"repositories": ["r1/i1"]}"#;

  let mut server = mockito::Server::new();
  let addr = server.host_with_port();

  let mock = server
    .mock("GET", "/v2/_catalog?n=1")
    .with_status(200)
    .with_header("Link", &format!(r#"<{addr}/v2/_catalog?n=21&last=r1/i1>; rel="next""#))
    .with_header("Content-Type", "application/json")
    .with_body(repos_p1)
    .create();

  let runtime = Runtime::new().unwrap();
  let client = docker_registry::v2::Client::configure()
    .registry(&addr)
    .insecure_registry(true)
    .username(None)
    .password(None)
    .build()
    .unwrap();

  let next = Box::pin(client.get_catalog(Some(1)));

  let (page1, next) = runtime.block_on(next.into_future());
  assert_eq!(page1.unwrap().unwrap(), "r1/i1".to_owned());

  let (page2, next) = runtime.block_on(next.into_future());
  // TODO(lucab): implement pagination
  if page2.is_some() {
    panic!("end is some: {page2:?}");
  }

  let (end, _) = runtime.block_on(next.into_future());
  if end.is_some() {
    panic!("end is some: {end:?}");
  }

  mock.assert();
}
