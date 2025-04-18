# Docker Registry

A pure-Rust asynchronous library for the Docker Registry HTTP API v2.

`docker-registry` provides support for asynchronous interaction with container registries
conformant to the [Docker Registry HTTP API v2](https://docs.docker.com/registry/spec/api/) specification.

## Configurable features

The following is a list of [Cargo features](https://doc.rust-lang.org/stable/cargo/reference/manifest.html#the-features-section) that consumers can enable or disable:

* **reqwest-default-tls** *(enabled by default)*: provides TLS support via [system-specific library](https://docs.rs/native-tls) (OpenSSL on Linux)
* **reqwest-rustls**: provides TLS support via the [rustls](https://docs.rs/rustls) library

## Testing

### Integration tests

This library relies on the [mockito](https://github.com/lipanski/mockito) framework for mocking.

### Interoperability tests

This library includes additional interoperability tests against some of the most common registries.

Those tests are not run by default as they require network access and registry credentials.

They are gated behind a dedicated "test-net-private" feature and can be run as:

```sh
cargo test --features test-net-private
```

Credentials for those registries must be provided via environmental flags.

## License

Licensed under either of

* Apache License, Version 2.0
   ([LICENSE-APACHE](LICENSE-APACHE) or <http://www.apache.org/licenses/LICENSE-2.0>)
* MIT license
   ([LICENSE-MIT](LICENSE-MIT) or <http://opensource.org/licenses/MIT>)

at your option.

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.
