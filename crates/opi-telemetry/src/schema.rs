//! Serializable description data. These values do not validate runtime spans.
//! Domain crates own typed instrumentation and their schema; this crate contains
//! no AI span names, token/cost policies or automatic error interpretation.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum AttributeType {
    #[serde(rename = "string")]
    String {
        values: Option<Vec<String>>,
        examples: Option<Vec<String>>,
    },
    #[serde(rename = "number")]
    Number {
        values: Option<Vec<f64>>,
        examples: Option<Vec<f64>>,
    },
    #[serde(rename = "boolean")]
    Boolean {
        values: Option<Vec<bool>>,
        examples: Option<Vec<bool>>,
    },
    #[serde(rename = "string[]")]
    Strings {
        #[serde(rename = "elementValues")]
        element_values: Option<Vec<String>>,
        examples: Option<Vec<Vec<String>>>,
    },
    #[serde(rename = "number[]")]
    Numbers {
        #[serde(rename = "elementValues")]
        element_values: Option<Vec<f64>>,
        examples: Option<Vec<Vec<f64>>>,
    },
    #[serde(rename = "boolean[]")]
    Booleans {
        #[serde(rename = "elementValues")]
        element_values: Option<Vec<bool>>,
        examples: Option<Vec<Vec<bool>>>,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Cardinality {
    Low,
    High,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AttributeDefinition {
    pub description: String,
    pub sensitive: Option<bool>,
    pub cardinality: Option<Cardinality>,
    #[serde(flatten)]
    pub value_type: AttributeType,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RequiredAttributeDefinition {
    #[serde(flatten)]
    pub definition: AttributeDefinition,
    pub required: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EventDefinition {
    pub description: String,
    pub attributes: BTreeMap<String, RequiredAttributeDefinition>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ParentDefinition {
    Any,
    RootOrExternal,
    Spans { spans: Vec<String> },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StatusDefinition {
    pub default: DefaultStatus,
    pub error_when: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DefaultStatus {
    Ok,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpanDefinition {
    pub description: String,
    pub parents: ParentDefinition,
    pub start_attributes: BTreeMap<String, RequiredAttributeDefinition>,
    pub end_attributes: BTreeMap<String, AttributeDefinition>,
    pub events: Option<BTreeMap<String, EventDefinition>>,
    pub status: StatusDefinition,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum MetricType {
    Counter,
    UpDownCounter,
    Histogram { boundaries: Option<Vec<f64>> },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MetricDefinition {
    pub description: String,
    pub unit: Option<String>,
    pub attributes: BTreeMap<String, RequiredAttributeDefinition>,
    #[serde(flatten)]
    pub metric_type: MetricType,
}

impl MetricDefinition {
    /// Pass the definition's metadata to a synchronous adapter factory. This
    /// does not validate measurements or impose new schema restrictions.
    pub fn instrument(&self, name: impl Into<String>) -> crate::InstrumentDefinition {
        let (kind, boundaries) = match &self.metric_type {
            MetricType::Counter => (crate::MetricKind::Counter, None),
            MetricType::UpDownCounter => (crate::MetricKind::UpDownCounter, None),
            MetricType::Histogram { boundaries } => {
                (crate::MetricKind::Histogram, boundaries.clone())
            }
        };
        crate::InstrumentDefinition {
            name: name.into(),
            kind,
            description: Some(self.description.clone()),
            unit: self.unit.clone(),
            boundaries,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TelemetrySchema {
    pub version: u32,
    pub spans: BTreeMap<String, SpanDefinition>,
    pub metrics: Option<BTreeMap<String, MetricDefinition>>,
}
