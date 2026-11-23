//! Tests of public HTTP acceptance through HttpClient and Models Chat calls.

#![forbid(unsafe_code)]

#[path = "http_acceptance/fixtures.rs"]
mod fixtures;
#[path = "http_acceptance/https_proxy.rs"]
mod https_proxy;
#[path = "http_acceptance/redirects_and_disconnects.rs"]
mod redirects_and_disconnects;
#[path = "http_acceptance/response_parsing.rs"]
mod response_parsing;

// The proxy topic uses the CA and server identity from the shared TLS corpus.
#[allow(dead_code)]
#[path = "support/tls_fixtures.rs"]
mod tls_fixtures;
