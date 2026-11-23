//! Public TLS and mTLS contracts for HttpClient and the Models chat path.

#![forbid(unsafe_code)]

#[path = "tls_contract/cancellation.rs"]
mod cancellation;
#[path = "tls_contract/fixtures.rs"]
mod fixtures;
#[path = "tls_contract/mtls.rs"]
mod mtls;
#[path = "tls_contract/negotiation_and_trust.rs"]
mod negotiation_and_trust;
#[path = "tls_contract/provider_isolation.rs"]
mod provider_isolation;
#[path = "support/tls_fixtures.rs"]
mod tls_fixtures;
