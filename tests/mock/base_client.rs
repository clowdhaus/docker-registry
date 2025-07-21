static API_VERSION_K: &str = "Docker-Distribution-API-Version";
static API_VERSION_V: &str = "registry/2.0";

#[tokio::test]
#[ignore]
async fn test_base_no_insecure() {
  let mut server = mockito::Server::new_async().await;
  let addr = server.host_with_port();

  let mock = server
    .mock("GET", "/v2/")
    .with_status(200)
    .with_header(API_VERSION_K, API_VERSION_V)
    .create();

  let client = docker_registry::v2::Client::configure()
    .registry(&addr)
    .insecure_registry(false)
    .username(None)
    .password(None)
    .build()
    .unwrap();

  let res = client.is_v2_supported().await.unwrap();

  // This relies on the fact that mockito is HTTP-only and
  // trying to speak TLS to it results in garbage/errors.
  mock.assert_async().await;
  assert!(res);
}

#[tokio::test]
async fn test_base_useragent() {
  let mut server = mockito::Server::new_async().await;
  let addr = server.host_with_port();

  let mock = server
    .mock("GET", "/v2/")
    .match_header("user-agent", docker_registry::USER_AGENT)
    .with_status(200)
    .with_header(API_VERSION_K, API_VERSION_V)
    .create();

  let client = docker_registry::v2::Client::configure()
    .registry(&addr)
    .insecure_registry(true)
    .username(None)
    .password(None)
    .build()
    .unwrap();

  let res = client.is_v2_supported().await.unwrap();

  mock.assert_async().await;
  assert!(res);
}

#[tokio::test]
async fn test_base_custom_useragent() {
  let ua = "custom-ua/1.0";

  let mut server = mockito::Server::new_async().await;
  let addr = server.host_with_port();

  let mock = server
    .mock("GET", "/v2/")
    .match_header("user-agent", ua)
    .with_status(200)
    .with_header(API_VERSION_K, API_VERSION_V)
    .create();

  let client = docker_registry::v2::Client::configure()
    .registry(&addr)
    .insecure_registry(true)
    .user_agent(Some(ua.to_string()))
    .username(None)
    .password(None)
    .build()
    .unwrap();

  let res = client.is_v2_supported().await.unwrap();

  mock.assert_async().await;
  assert!(res);
}

/// Test that we properly deserialize API error payload and can access error contents.
#[test_case::test_case("tests/fixtures/api_error_fixture_with_detail.json".to_string() ; "API error with detail")]
#[test_case::test_case("tests/fixtures/api_error_fixture_without_detail.json".to_string() ; "API error without detail")]
fn test_base_api_error(fixture: String) {
  let ua = "custom-ua/1.0";
  let image = "fake/image";
  let version = "fakeversion";

  let mut server = mockito::Server::new();
  let addr = server.host_with_port();

  let mock = server
    .mock("GET", format!("/v2/{image}/manifests/{version}").as_str())
    .match_header("user-agent", ua)
    .with_status(404)
    .with_header(API_VERSION_K, API_VERSION_V)
    .with_body_from_file(fixture)
    .create();

  let runtime = tokio::runtime::Runtime::new().unwrap();
  let dclient = docker_registry::v2::Client::configure()
    .registry(&addr)
    .insecure_registry(true)
    .user_agent(Some(ua.to_string()))
    .username(None)
    .password(None)
    .build()
    .unwrap();

  let futcheck = dclient.get_manifest(image, version);

  let res = runtime.block_on(futcheck);
  assert!(res.is_err());

  assert!(matches!(res, Err(docker_registry::errors::Error::Api(_))));
  if let docker_registry::errors::Error::Api(e) = res.unwrap_err() {
    assert_eq!(e.errors().as_ref().unwrap()[0].code(), "UNAUTHORIZED");
    assert_eq!(
      e.errors().as_ref().unwrap()[0].message().unwrap(),
      "authentication required"
    );
  }
  mock.assert();
}

mod test_custom_root_certificate {
  use std::{error::Error, net::TcpListener};

  use docker_registry::v2::Client;
  use native_tls::{HandshakeError, Identity, TlsStream};
  use reqwest::Certificate;
  use rustls_cert_gen::CertificateBuilder;

  fn run_server(listener: TcpListener, identity: Identity) -> Result<(), std::io::Error> {
    println!("Will accept tls connections at {}", listener.local_addr()?);

    let mut incoming = listener.incoming();
    let test_server = native_tls::TlsAcceptor::new(identity).unwrap();

    if let Some(stream_result) = incoming.next() {
      println!("Incoming");
      let stream = stream_result?;

      println!("Accepting incoming as tls");
      let accept_result = test_server.accept(stream);

      if let Err(e) = map_tls_io_error(accept_result) {
        eprintln!("Accept failed: {e:?}");
      }

      println!("Done with stream");
    } else {
      panic!("Never received an incoming connection");
    }

    println!("No longer accepting connections");

    Ok(())
  }

  async fn run_client(ca_certificate: Option<Certificate>, client_host: String) {
    println!("Client creating");
    let mut config = Client::configure().registry(&client_host);

    if let Some(ca) = &ca_certificate {
      config = config.add_root_certificate(ca.clone());
    }

    let registry = config.build().unwrap();
    let err = registry.is_auth().await.unwrap_err();

    if let docker_registry::errors::Error::Reqwest(r) = err {
      if let Some(s) = r.source() {
        let oh: Option<&hyper::Error> = s.downcast_ref();

        if let Some(he) = oh {
          println!("Hyper error: {he:?}");

          if ca_certificate.is_some() {
            assert!(he.is_closed(), "is a ChannelClosed error, not a certificate error");
          } else {
            let hec = he.source().unwrap();

            let message = format!("{hec}");
            assert!(
              message.contains("certificate verify failed"),
              "'certificate verify failed' contained in: {message}"
            );
          }
        }
      }
    } else {
      eprintln!("Unexpected error: {err:?}");
    }
  }

  fn map_tls_io_error<S>(tls_result: Result<TlsStream<S>, HandshakeError<S>>) -> Result<TlsStream<S>, String>
  where
    S: std::io::Read + std::io::Write,
  {
    match tls_result {
      Ok(stream) => Ok(stream),
      Err(he) => {
        match he {
          HandshakeError::Failure(e) => Err(format!("{e:#?}")),
          // Can't directly unwrap because TlsStream doesn't implement Debug trait
          HandshakeError::WouldBlock(_) => Err("Would block".into()),
        }
      }
    }
  }

  #[derive(Debug)]
  struct CertData {
    ca_cert: Vec<u8>,
    localhost_cert: Vec<u8>,
    localhost_key: Vec<u8>,
  }

  fn get_certs() -> CertData {
    let ca = CertificateBuilder::new()
      .certificate_authority()
      .country_name("USA")
      .expect("Failed to set country name")
      .organization_name("Automated Testing CA")
      .build()
      .expect("Failed to build CA");
    let ca_key = ca.serialize_pem();

    let mut entity = CertificateBuilder::new().end_entity().common_name("localhost");
    entity.server_auth();
    let entity_key = entity.build(&ca).expect("Failed to build entity").serialize_pem();

    CertData {
      ca_cert: ca_key.cert_pem.as_bytes().to_vec(),
      localhost_cert: entity_key.cert_pem.as_bytes().to_vec(),
      localhost_key: entity_key.private_key_pem.as_bytes().to_vec(),
    }
  }

  #[tokio::test]
  async fn without_ca() {
    with_ca_cert(false).await
  }

  #[tokio::test]
  pub async fn with_ca() {
    with_ca_cert(true).await;
  }

  // ToDo: fails on macos - https://github.com/seanmonstar/reqwest/issues/2321
  async fn with_ca_cert(with_ca: bool) {
    let certs = get_certs();
    let mut ca_cert = None;
    if with_ca {
      ca_cert = Some(Certificate::from_pem(&certs.ca_cert).expect("Failed to create CA certificate"));
    }

    let registry_identity =
      Identity::from_pkcs8(&certs.localhost_cert, &certs.localhost_key).expect("Failed to create registry identity");

    let listener = TcpListener::bind("localhost:0").unwrap();

    // local_addr returns an IP address, but we need to use a name for TLS, so extract only the port number.
    let listener_port = listener.local_addr().unwrap().port();
    let client_host = format!("localhost:{listener_port}");
    let t_server = std::thread::spawn(move || run_server(listener, registry_identity));
    let t_client = tokio::task::spawn(async move { run_client(ca_cert, client_host).await });

    println!("Joining client");
    t_client.await.unwrap();

    println!("Joining server");
    t_server.join().unwrap().unwrap();

    println!("Done");
  }
}
