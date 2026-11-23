//! Classifier declarations, model metadata, answers and result values.

use super::ApiId;
use super::model::ModelPricing;
use super::usage::{CostBreakdown, Usage};
use crate::errors::ProviderError;

/// One input modality a classifier model accepts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClassifierInputModality {
    /// Text state (always supported).
    Text,
    /// Image input judged together with the state.
    Image,
}

/// One base64-encoded image judged together with the state.
#[derive(Debug, Clone, PartialEq)]
pub struct ClassifierImage {
    /// MIME type (for example `image/png`).
    pub mime_type: String,
    /// Base64-encoded image bytes, exactly as supplied.
    pub data: String,
}

/// Named classifier declarations in source-defined enumeration order.
///
/// Canonical decimal array-index keys from `0` through `4294967294` appear
/// first in numeric order. Other string keys retain first-insertion order;
/// for example, `01`, `-0` and `4294967295` are ordinary string keys.
/// Replacing a key changes its value without moving it. Removing and then
/// reinserting an ordinary string key places it after the other ordinary
/// keys. Collection from an iterator applies these same replacement rules.
///
/// This order determines question wire order, Llama question execution,
/// choice labels and the first choice selected when Llama probabilities tie.
/// Serialization emits a JSON object in the same enumeration order.
#[derive(Debug, Clone, PartialEq)]
pub struct ClassifierDeclarations<T> {
    entries: Vec<(String, T)>,
}

impl<T> Default for ClassifierDeclarations<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> ClassifierDeclarations<T> {
    /// Construct an empty declaration collection.
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    /// Insert a declaration, returning its previous value when replaced.
    /// Replacement retains the key's existing position.
    pub fn insert(&mut self, key: String, value: T) -> Option<T> {
        if let Some((_, current)) = self.entries.iter_mut().find(|(name, _)| name == &key) {
            return Some(std::mem::replace(current, value));
        }
        let position = declaration_index(&key)
            .and_then(|index| {
                self.entries
                    .iter()
                    .position(|(name, _)| declaration_index(name).is_none_or(|other| other > index))
            })
            .unwrap_or(self.entries.len());
        self.entries.insert(position, (key, value));
        None
    }

    /// Remove a declaration, preserving the other keys' relative order.
    pub fn remove(&mut self, key: &str) -> Option<T> {
        let position = self.entries.iter().position(|(name, _)| name == key)?;
        Some(self.entries.remove(position).1)
    }

    /// Look up a declaration by its exact key.
    pub fn get(&self, key: &str) -> Option<&T> {
        self.entries
            .iter()
            .find(|(name, _)| name == key)
            .map(|(_, value)| value)
    }

    /// The number of named declarations.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the collection contains no declarations.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Borrow keys and values in declaration enumeration order.
    pub fn iter(&self) -> impl Iterator<Item = (&String, &T)> {
        self.entries.iter().map(|(key, value)| (key, value))
    }

    /// Borrow keys in declaration enumeration order.
    pub fn keys(&self) -> impl Iterator<Item = &String> {
        self.entries.iter().map(|(key, _)| key)
    }

    /// Borrow values in declaration enumeration order.
    pub fn values(&self) -> impl Iterator<Item = &T> {
        self.entries.iter().map(|(_, value)| value)
    }
}

fn declaration_index(key: &str) -> Option<u32> {
    let index = key.parse::<u32>().ok()?;
    (index != u32::MAX && index.to_string() == key).then_some(index)
}

impl<T> FromIterator<(String, T)> for ClassifierDeclarations<T> {
    fn from_iter<I: IntoIterator<Item = (String, T)>>(iter: I) -> Self {
        let mut declarations = Self::new();
        for (key, value) in iter {
            declarations.insert(key, value);
        }
        declarations
    }
}

impl<T, const N: usize> From<[(String, T); N]> for ClassifierDeclarations<T> {
    fn from(entries: [(String, T); N]) -> Self {
        entries.into_iter().collect()
    }
}

impl<T> IntoIterator for ClassifierDeclarations<T> {
    type Item = (String, T);
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.entries.into_iter()
    }
}

impl<'a, T> IntoIterator for &'a ClassifierDeclarations<T> {
    type Item = (&'a String, &'a T);
    type IntoIter = std::iter::Map<
        std::slice::Iter<'a, (String, T)>,
        fn(&'a (String, T)) -> (&'a String, &'a T),
    >;

    fn into_iter(self) -> Self::IntoIter {
        self.entries.iter().map(|(key, value)| (key, value))
    }
}

impl<T: serde::Serialize> serde::Serialize for ClassifierDeclarations<T> {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeMap;
        let mut map = serializer.serialize_map(Some(self.len()))?;
        for (key, value) in self {
            map.serialize_entry(key, value)?;
        }
        map.end()
    }
}

/// One typed question about the state.
#[derive(Debug, Clone, PartialEq)]
pub enum ClassifierQuestion {
    /// Pick one of the named options.
    Choice {
        /// What the model is asked to classify.
        instructions: String,
        /// Option keys and descriptions in declaration enumeration order.
        criteria: ClassifierDeclarations<String>,
    },
    /// Score the state against ordered levels.
    Score {
        /// What the model is asked to score.
        instructions: String,
        /// Ordered level labels, lowest first.
        levels: Vec<String>,
    },
    /// A yes/no question.
    Bool {
        /// What the model is asked to judge.
        instructions: String,
        /// Meaning of a true answer.
        true_criteria: String,
        /// Meaning of a false answer.
        false_criteria: String,
    },
}

impl ClassifierQuestion {
    /// The question's instructions.
    pub fn instructions(&self) -> &str {
        match self {
            ClassifierQuestion::Choice { instructions, .. }
            | ClassifierQuestion::Score { instructions, .. }
            | ClassifierQuestion::Bool { instructions, .. } => instructions,
        }
    }
}

/// The complete classifier input.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ClassifierContext {
    /// The state object being judged (any JSON object).
    pub state: serde_json::Value,
    /// Typed questions keyed by caller id, in declaration enumeration order.
    pub questions: ClassifierDeclarations<ClassifierQuestion>,
    /// Images judged together with `state`. Adapters that cannot accept
    /// images reject them typed before any request.
    pub images: Option<Vec<ClassifierImage>>,
}

/// One typed answer to a question.
#[derive(Debug, Clone, PartialEq)]
pub enum ClassifierAnswer {
    /// A chosen option with its probability distribution and confidence.
    Choice {
        /// The chosen option key.
        choice: String,
        /// Per-option probabilities, finite numbers exactly as reported.
        probabilities: std::collections::BTreeMap<String, f64>,
        /// Reported confidence.
        confidence: f64,
    },
    /// A score against the question's levels.
    Score {
        /// Reported score.
        score: f64,
        /// Reported confidence.
        confidence: f64,
    },
    /// A yes/no answer with its true-probability.
    Bool {
        /// Probability of a true answer.
        probability: f64,
    },
}

/// Terminal state of one classifier call.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClassifierStopReason {
    /// The call completed with answers for every question.
    Stop,
    /// The provider call failed; `error`/`error_message` carry the cause.
    Error,
    /// The caller cancelled the call.
    Aborted,
}

/// Image transport metadata for a System One classifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClassifierImageInput {
    /// TypeSafe's asset map and named state entries.
    Assets,
    /// Cloudflare's data-URL image array.
    Images,
    /// OpenRouter's ordered `image_url` state parts.
    StateImageUrls,
}

/// Provider-specific classifier transport metadata from the published catalog.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ClassifierCompat {
    /// How a System One route accepts image input.
    pub image_input: Option<ClassifierImageInput>,
    /// Whether image requests require TypeSafe's opt-in header.
    pub send_image_opt_in_header: Option<bool>,
}

/// The complete classifier-model record one classify call binds to.
///
/// The record carries the exact classifier API identity (pi string, e.g.
/// `typesafe-system-one`), the owning provider, the endpoint base URL, the
/// accepted input modalities, the context window and the catalog pricing
/// (tiers included: the classifier path prices usage like chat, unlike the
/// image path's base-rate exception). It is the operation-qualified
/// counterpart of the chat seam's complete record: chat models never satisfy
/// classifier selection, and classifier records never enter the chat
/// catalog.
#[derive(Debug, Clone, PartialEq)]
pub struct ClassifierModelRecord {
    /// Owning provider id (for example `typesafe`).
    pub provider_id: String,
    /// Bare model id.
    pub id: String,
    /// Human-readable name.
    pub name: String,
    /// Exact classifier API identity (pi string).
    pub api: ApiId,
    /// Endpoint base URL (trailing slashes are stripped when joining).
    pub base_url: String,
    /// Ordered ordinary header overrides; `None` removes an earlier value.
    pub headers: Vec<(String, Option<String>)>,
    /// Source input-limit metadata; it does not impose SDK resize policy.
    pub input_limits: Option<serde_json::Value>,
    /// Provider-specific classifier transport metadata from the catalog.
    pub compat: Option<ClassifierCompat>,
    /// Accepted input modalities.
    pub input: Vec<ClassifierInputModality>,
    /// Context window in tokens.
    pub context_window: u64,
    /// Catalog pricing with tiers, or `None` when absent. Finite negative
    /// source placeholders remain metadata; an unknown selected rate yields
    /// no calculated cost.
    pub cost: Option<ModelPricing>,
}

#[cfg(any(feature = "builtins", feature = "provider-protocol-common"))]
pub(crate) fn cost_for_usage(
    record: &ClassifierModelRecord,
    usage: &Usage,
) -> Option<CostBreakdown> {
    if !usage.reported {
        return None;
    }
    let pricing = record.cost.as_ref()?;
    pricing.validate_source_metadata().ok()?;
    let rates = pricing.effective_for_usage(usage);
    crate::types::model::validate_pricing(&rates).ok()?;
    let cost = crate::types::usage::calculate_cost(usage, &rates);
    cost.total_cost().is_finite().then_some(cost)
}

/// The classifier result: pi `ClassifierResult` semantics.
///
/// Provider failures never surface as `Err`: they keep the record identity,
/// empty answers, `stop_reason = Error`/`Aborted`, a sanitized display
/// message and the typed [`ProviderError`] cause. A request that was billed
/// keeps its reported usage: parsing failures after a successful response
/// retain the usage the transport already recorded.
#[derive(Debug, Clone)]
pub struct ClassifierResult {
    /// Classifier API identity the record named.
    pub api: String,
    /// Owning provider id.
    pub provider: String,
    /// Model id.
    pub model: String,
    /// Answers keyed by question id. Empty on failure.
    pub answers: std::collections::BTreeMap<String, ClassifierAnswer>,
    /// Reported token usage, when the service reported token counts (also
    /// retained when later answer parsing failed: the request was billed).
    pub usage: Option<Usage>,
    /// Chat-style tiered cost computed from the record's catalog pricing,
    /// when usage exists.
    pub cost: Option<CostBreakdown>,
    /// Terminal state.
    pub stop_reason: ClassifierStopReason,
    /// Sanitized display message for `Error`/`Aborted`.
    pub error_message: Option<String>,
    /// Typed cause for `Error`/`Aborted`.
    pub error: Option<ProviderError>,
}

impl ClassifierResult {
    /// A pre-dispatch or provider failure result for the given record
    /// identity: empty answers, typed cause and display message.
    pub fn failure(
        record: &ClassifierModelRecord,
        stop_reason: ClassifierStopReason,
        error: ProviderError,
        message: String,
    ) -> Self {
        Self {
            api: record.api.as_str().to_owned(),
            provider: record.provider_id.clone(),
            model: record.id.clone(),
            answers: std::collections::BTreeMap::new(),
            usage: None,
            cost: None,
            stop_reason,
            error_message: Some(message),
            error: Some(error),
        }
    }

    /// An in-progress success result for the given record identity: answers
    /// and usage/cost are filled by the adapter as it parses.
    pub fn in_progress(record: &ClassifierModelRecord) -> Self {
        Self {
            api: record.api.as_str().to_owned(),
            provider: record.provider_id.clone(),
            model: record.id.clone(),
            answers: std::collections::BTreeMap::new(),
            usage: None,
            cost: None,
            stop_reason: ClassifierStopReason::Stop,
            error_message: None,
            error: None,
        }
    }
}
