//! Telemetry context accepted by AI operations.
//!
//! The backend-neutral carrier is owned by `opi-telemetry`. These existing
//! paths remain available for request callers. `TelemetryContext::from_span`
//! uses its optional tracing adapter; no-op preserves ambient host tracing.
//! AI-specific schema, instrumentation and request settlement belong here,
//! independently of context cloning, future polling or consumer Drop.

pub use opi_telemetry::{Instrumented, TelemetryContext};
