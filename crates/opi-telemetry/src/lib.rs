//! Backend-neutral, explicitly owned telemetry. No global initialization is required.
//!
//! [`TelemetryContext::start_span`] starts synchronously. The returned [`SpanOwner`]
//! alone can finish the span; cloned observation handles and future wrappers cannot.
//! Dropping an unfinished owner leaves the observation unfinished, never successful.
//! See the crate README for the boundary with execution, cancellation and exporters.
#![forbid(unsafe_code)]

pub mod backend;
pub mod memory;
pub mod schema;
#[cfg(feature = "tracing")]
pub mod tracing;

mod context;
mod metrics;
mod noop;

use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;

pub use context::{Instrumented, SpanOwner, TelemetryContext, TelemetrySpan};
pub use metrics::{Counter, Histogram, InstrumentDefinition, MetricKind, Metrics, UpDownCounter};

/// Owned values: arrays are copied when an adapter records borrowed attributes.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(untagged)]
pub enum AttributeValue {
    String(String),
    Number(f64),
    Boolean(bool),
    Strings(Vec<String>),
    Numbers(Vec<f64>),
    Booleans(Vec<bool>),
}

impl From<&str> for AttributeValue {
    fn from(value: &str) -> Self {
        Self::String(value.into())
    }
}

impl From<String> for AttributeValue {
    fn from(value: String) -> Self {
        Self::String(value)
    }
}

/// Missing keys are absent; updates merge without removing existing keys.
pub type Attributes = BTreeMap<String, AttributeValue>;

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ErrorStatus {
    pub name: String,
    pub message: String,
}

/// Domain owners choose status. Cancellation is not automatically an error.
#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum SpanStatus {
    #[default]
    Ok,
    Error {
        error: Option<ErrorStatus>,
    },
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct SpanOptions {
    pub name: String,
    pub attributes: Attributes,
}

impl SpanOptions {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            ..Self::default()
        }
    }
}

/// Adapter failure, retaining the original error as its source. Business errors
/// belong to the caller and are never converted to this type automatically.
#[derive(Debug)]
pub struct TelemetryError(Box<dyn Error + Send + Sync>);

impl TelemetryError {
    pub fn new(error: impl Error + Send + Sync + 'static) -> Self {
        Self(Box::new(error))
    }
}

impl fmt::Display for TelemetryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl Error for TelemetryError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(self.0.as_ref())
    }
}

pub type Result<T> = std::result::Result<T, TelemetryError>;
