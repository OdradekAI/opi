//! Provider wire and transport lifecycle contracts exercised against local peers.

#[path = "provider_lifecycle/cancellation_and_release.rs"]
mod cancellation_and_release;
#[path = "provider_lifecycle/fixtures.rs"]
mod fixtures;
#[path = "provider_lifecycle/request_and_errors.rs"]
mod request_and_errors;
#[path = "provider_lifecycle/sse_decoding.rs"]
mod sse_decoding;
