//! Public Deferred contracts with the minimum SDK feature set.

#[path = "deferred_contract/cancellation.rs"]
mod cancellation;
#[path = "deferred_contract/fixtures.rs"]
mod fixtures;
#[path = "deferred_contract/handle_validation.rs"]
mod handle_validation;
#[path = "deferred_contract/reopen.rs"]
mod reopen;
#[path = "deferred_contract/submission_and_fetch.rs"]
mod submission_and_fetch;
#[path = "deferred_contract/telemetry.rs"]
mod telemetry;

#[path = "support/pending_auth.rs"]
mod registration_pending_auth;
