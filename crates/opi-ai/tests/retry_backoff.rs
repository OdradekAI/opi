//! Public retry contracts exercised at the policy and acquisition boundaries.

#[path = "retry_backoff/acquisition.rs"]
mod acquisition;
#[path = "retry_backoff/backoff_policy.rs"]
mod backoff_policy;
#[path = "retry_backoff/response_headers.rs"]
mod response_headers;
#[path = "retry_backoff/smithy.rs"]
mod smithy;
