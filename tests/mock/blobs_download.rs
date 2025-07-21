use futures::{FutureExt, stream::StreamExt};
use sha2::Digest;

type Fallible<T> = Result<T, Box<dyn std::error::Error>>;

#[tokio::test]
async fn test_blobs_has_layer() {
  let name = "my-repo/my-image";
  let digest = "fakedigest";
  let binary_digest = "binarydigest";
  let ep = format!("/v2/{name}/blobs/{digest}");

  let mut server = mockito::Server::new_async().await;
  let addr = server.host_with_port();

  let mock = server
    .mock("HEAD", ep.as_str())
    .with_status(200)
    .with_header("Content-Length", "0")
    .with_header("Docker-Content-Digest", binary_digest)
    .create();

  let client = docker_registry::v2::Client::configure()
    .registry(&addr)
    .insecure_registry(true)
    .username(None)
    .password(None)
    .build()
    .unwrap();

  let res = client.has_blob(name, digest).await.unwrap();

  mock.assert_async().await;
  assert!(res);
}

#[tokio::test]
async fn test_blobs_hasnot_layer() {
  let name = "my-repo/my-image";
  let digest = "fakedigest";
  let ep = format!("/v2/{name}/blobs/{digest}");

  let mut server = mockito::Server::new_async().await;
  let addr = server.host_with_port();

  let mock = server.mock("HEAD", ep.as_str()).with_status(404).create();

  let client = docker_registry::v2::Client::configure()
    .registry(&addr)
    .insecure_registry(true)
    .username(None)
    .password(None)
    .build()
    .unwrap();

  let res = client.has_blob(name, digest).await.unwrap();

  mock.assert_async().await;
  assert!(!res);
}

#[tokio::test]
async fn get_blobs_succeeds_with_consistent_layer() -> Fallible<()> {
  let name = "my-repo/my-image";
  let blob = b"hello";
  let digest = format!("sha256:{:x}", sha2::Sha256::digest(blob));
  let ep = format!("/v2/{name}/blobs/{digest}");

  let mut server = mockito::Server::new_async().await;
  let addr = server.host_with_port();

  let mock = server
    .mock("GET", ep.as_str())
    .with_status(200)
    .with_body(blob)
    .create();

  let client = docker_registry::v2::Client::configure()
    .registry(&addr)
    .insecure_registry(true)
    .username(None)
    .password(None)
    .build()
    .unwrap();

  let res = client.get_blob(name, &digest).await.unwrap();

  mock.assert_async().await;
  assert_eq!(blob, res.as_slice());

  Ok(())
}

#[tokio::test]
async fn get_blobs_fails_with_inconsistent_layer() -> Fallible<()> {
  let name = "my-repo/my-image";
  let blob = b"hello";
  let blob2 = b"hello2";
  let digest = format!("sha256:{:x}", sha2::Sha256::digest(blob));
  let ep = format!("/v2/{name}/blobs/{digest}");

  let mut server = mockito::Server::new_async().await;
  let addr = server.host_with_port();

  let mock = server
    .mock("GET", ep.as_str())
    .with_status(200)
    .with_body(blob2)
    .create();

  let client = docker_registry::v2::Client::configure()
    .registry(&addr)
    .insecure_registry(true)
    .username(None)
    .password(None)
    .build()
    .unwrap();

  match client.get_blob(name, &digest).await {
    Ok(_) => panic!("Expected error"),
    Err(e) => assert_eq!(e.to_string(), "content digest error"),
  }

  mock.assert_async().await;

  Ok(())
}

#[tokio::test]
async fn get_blobs_stream() -> Fallible<()> {
  let name = "my-repo/my-image";
  let blob = b"hello";
  let digest = format!("sha256:{:x}", sha2::Sha256::digest(blob));
  let ep = format!("/v2/{name}/blobs/{digest}");

  let mut server = mockito::Server::new_async().await;
  let addr = server.host_with_port();

  let mock = server
    .mock("GET", ep.as_str())
    .with_status(200)
    .with_body(blob)
    .create();

  let client = docker_registry::v2::Client::configure()
    .registry(&addr)
    .insecure_registry(true)
    .username(None)
    .password(None)
    .build()
    .unwrap();

  let res = client.get_blob_response(name, &digest).await.unwrap();

  mock.assert_async().await;
  assert_eq!(res.size(), Some(5));
  let stream_output = res.stream().next().now_or_never();
  let output = stream_output.unwrap_or_else(|| panic!("No stream output"));
  let received_blob = output.unwrap_or_else(|| panic!("No blob data"))?;
  assert_eq!(blob.to_vec(), received_blob);

  Ok(())
}
