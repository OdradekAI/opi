//! Protocol types, bounded codecs, JSON schemas, and fixtures for opi command
//! execution.
//!
//! The current wire protocol is [`execution::v1`], whose wire identity is
//! `command-execution-jsonl-v1`. That module documents the state machine, frame
//! and cumulative bounds, wire identity, version negotiation, compatibility
//! rules, and the request-id invariant.
//!
//! # Dependency neutrality
//!
//! This crate contains only protocol types, bounded codecs, schemas, and
//! fixtures. It has no dependency on `opi-agent` or `opi-coding-agent` and owns
//! no process launch, package policy, routing, permission, or sandbox behavior.
//! Runtime supervision (deadline enforcement, process-tree kill, cleanup) and
//! the live handshake are responsibilities of the execution host and the
//! execution backend, not this crate. Capabilities here are phrased as
//! "rejects frames that exceed a bound" (behavior when invoked), not as runtime
//! guarantees.

#![forbid(unsafe_code)]

pub mod execution;
