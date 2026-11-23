//! Request options, wire enrichment, and callbacks at the provider boundary.

#[path = "request_enrichment/callbacks.rs"]
mod callbacks;
#[path = "request_enrichment/fixtures.rs"]
mod fixtures;
#[path = "request_enrichment/protected_headers.rs"]
mod protected_headers;
#[path = "request_enrichment/scalars_and_timeout.rs"]
mod scalars_and_timeout;
#[path = "request_enrichment/session_affinity.rs"]
mod session_affinity;
