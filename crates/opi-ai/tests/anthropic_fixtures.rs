//! Anthropic protocol fixture test target.
//!
//! The target keeps one process-level owner while exposing independent topic
//! selectors for SSE mapping, strict wire projection, and assembled adapter
//! behavior.

#[path = "anthropic_fixtures/fixtures.rs"]
mod fixtures;
pub(crate) use fixtures::text_fixture;

#[path = "anthropic_fixtures/adapter.rs"]
mod adapter;
#[path = "anthropic_fixtures/constrained_sampling.rs"]
mod constrained_sampling;
#[path = "anthropic_fixtures/sse.rs"]
mod sse;
