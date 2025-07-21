use tokio::runtime::Runtime;

static REGISTRY: &str = "registry-1.docker.io";

fn get_env() -> Option<(String, String)> {
  let user = ::std::env::var("DOCKER_REGISTRY_DOCKER_USER");
  let password = ::std::env::var("DOCKER_REGISTRY_DOCKER_PASSWD");
  match (user, password) {
    (Ok(u), Ok(t)) => Some((u, t)),
    _ => None,
  }
}

#[test]
fn test_dockerio_getenv() {
  if get_env().is_none() {
    println!("[WARN] {REGISTRY}: missing DOCKER_REGISTRY_DOCKER_USER / DOCKER_REGISTRY_DOCKER_PASSWD");
  }
}

#[test]
fn test_dockerio_base() {
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
fn test_dockerio_insecure() {
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
fn test_dockerio_anonymous_auth() {
  let runtime = Runtime::new().unwrap();
  let image = "library/alpine";
  let version = "latest";
  let login_scope = format!("repository:{image}:pull");
  let scopes = vec![login_scope.as_str()];
  let client_future = docker_registry::v2::Client::configure()
    .registry(REGISTRY)
    .insecure_registry(false)
    .username(None)
    .password(None)
    .build()
    .unwrap()
    .authenticate(scopes.as_slice());

  let client = runtime.block_on(client_future).unwrap();
  let futcheck = client.get_manifest(image, version);

  let res = runtime.block_on(futcheck);
  assert!(res.is_ok());
}

/// Check that when requesting an image that does not exist
/// we get an Api error.
#[test]
fn test_dockerio_anonymous_non_existent_image() {
  let runtime = Runtime::new().unwrap();
  let image = "bad/image";
  let version = "latest";
  let login_scope = format!("repository:{image}:pull");
  let scopes = vec![login_scope.as_str()];
  let dclient_future = docker_registry::v2::Client::configure()
    .registry(REGISTRY)
    .insecure_registry(false)
    .username(None)
    .password(None)
    .build()
    .unwrap()
    .authenticate(scopes.as_slice());

  let dclient = runtime.block_on(dclient_future).unwrap();
  let futcheck = dclient.get_manifest(image, version);

  let res = runtime.block_on(futcheck);
  assert!(res.is_err());
  assert!(matches!(res, Err(docker_registry::errors::Error::Api(_))));
}

/// Test that we can deserialize OCI image manifest, as is
/// returned for s390x/ubuntu image.
#[test]
fn test_dockerio_anonymous_auth_oci_manifest() {
  let runtime = Runtime::new().unwrap();
  let image = "s390x/ubuntu";
  let version = "latest";
  let login_scope = format!("repository:{image}:pull");
  let scopes = vec![login_scope.as_str()];
  let dclient_future = docker_registry::v2::Client::configure()
    .registry(REGISTRY)
    .insecure_registry(false)
    .username(None)
    .password(None)
    .build()
    .unwrap()
    .authenticate(scopes.as_slice());

  let dclient = runtime.block_on(dclient_future).unwrap();
  let futcheck = dclient.get_manifest(image, version);

  let res = runtime.block_on(futcheck);
  assert!(res.is_ok());
}
