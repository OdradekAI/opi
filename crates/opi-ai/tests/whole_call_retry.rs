//! Public whole-call retry classification, callback and cancellation contracts.

#[path = "whole_call_retry/backoff_and_cancellation.rs"]
mod backoff_and_cancellation;
#[path = "whole_call_retry/call_and_callbacks.rs"]
mod call_and_callbacks;
#[path = "whole_call_retry/classification.rs"]
mod classification;
#[path = "whole_call_retry/fixtures.rs"]
mod fixtures;
