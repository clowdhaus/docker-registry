use futures::stream::StreamExt;
use tokio::runtime::Runtime;

static REGISTRY: &str = "gcr.io";

fn get_env() -> Option<(String, String)> {
  let user = ::std::env::var("DOCKER_REGISTRY_GCR_USER");
  let password = ::std::env::var("DOCKER_REGISTRY_GCR_PASSWD");
  match (user, password) {
    (Ok(u), Ok(t)) => Some((u, t)),
    _ => None,
  }
}

#[test]
fn test_dockerio_getenv() {
  if get_env().is_none() {
    println!("[WARN] {REGISTRY}: missing DOCKER_REGISTRY_GCR_USER / DOCKER_REGISTRY_GCR_PASSWD");
  }
}

#[test]
fn test_gcrio_base() {
  let (user, password) = match get_env() {
    Some(t) => t,
    None => return,
  };

  let runtime = Runtime::new().unwrap();
  let client = docker_registry::v2::Client::configure()
    .registry(REGISTRY)
    .insecure_registry(false)
    .username(Some(user))
    .password(Some(password))
    .build()
    .unwrap();

  let futcheck = client.is_v2_supported();

  let res = runtime.block_on(futcheck).unwrap();
  assert!(res);
}

#[test]
fn test_gcrio_insecure() {
  let runtime = Runtime::new().unwrap();
  let client = docker_registry::v2::Client::configure()
    .registry(REGISTRY)
    .insecure_registry(true)
    .username(None)
    .password(None)
    .build()
    .unwrap();

  let futcheck = client.is_v2_supported();

  let res = runtime.block_on(futcheck).unwrap();
  assert!(res);
}

#[test]
fn test_gcrio_get_tags() {
  let runtime = Runtime::new().unwrap();
  let client = docker_registry::v2::Client::configure()
    .registry(REGISTRY)
    .insecure_registry(false)
    .username(None)
    .password(None)
    .build()
    .unwrap();

  let image = "google_containers/mounttest";
  let fut_tags = client.get_tags(image, None);
  let tags = runtime.block_on(fut_tags.collect::<Vec<_>>());
  let has_version = tags.iter().map(|t| t.as_ref().unwrap()).any(|t| t == "0.2");

  assert!(has_version);
}

#[test]
fn test_gcrio_has_manifest() {
  let runtime = Runtime::new().unwrap();
  let client = docker_registry::v2::Client::configure()
    .registry(REGISTRY)
    .insecure_registry(false)
    .username(None)
    .password(None)
    .build()
    .unwrap();

  let image = "google_containers/mounttest";
  let tag = "0.2";
  let manifest_type = docker_registry::mediatypes::MediaTypes::ManifestV2S1Signed.to_string();
  let manifest_type_str = manifest_type.as_str();
  let manifest_type_vec = vec![manifest_type_str];
  let fut = client.has_manifest(image, tag, Some(manifest_type_vec.as_slice()));
  let has_manifest = runtime.block_on(fut).unwrap();

  assert_eq!(
    has_manifest,
    Some(docker_registry::mediatypes::MediaTypes::ManifestV2S1Signed)
  );
}

#[test]
fn test_gcrio_get_manifest() {
  let runtime = Runtime::new().unwrap();
  let client = docker_registry::v2::Client::configure()
    .registry(REGISTRY)
    .insecure_registry(false)
    .username(None)
    .password(None)
    .build()
    .unwrap();

  let image = "google_containers/mounttest";
  let tag = "0.2";

  let fut = client.get_manifest(image, tag);
  runtime
    .block_on(fut)
    .expect("check that manifest was downloaded successfully");
}
