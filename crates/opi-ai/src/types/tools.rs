//! Tool constrained-sampling intent values.

/// Per-tool constrained-sampling intent (AIR-040).
///
/// `JsonSchema` selects provider-side structured outputs: `Prefer` projects
/// strict when the wire and the schema support it and silently stays
/// non-strict otherwise; `Require` fails typed before dispatch when strict
/// mode is unavailable or the schema cannot be converted.
///
/// `Grammar` selects OpenAI-style grammar-constrained custom tools; the
/// variants carry lark and regex definitions, lark preferred.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", tag = "type")]
pub enum ConstrainedSampling {
    #[serde(rename = "json_schema")]
    JsonSchema { strict: StrictPreference },
    #[serde(rename_all = "camelCase")]
    Grammar {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        openai_lark: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        openai_regex: Option<String>,
    },
}

/// Whether a tool's JSON-schema constrained sampling is preferred or
/// required.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum StrictPreference {
    Prefer,
    Require,
}
