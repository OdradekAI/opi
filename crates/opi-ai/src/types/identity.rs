//! API identities preserve their existing exact matching rules.

/// Exact, caller-chosen chat API identity.
///
/// Preserves the exact string identity (no normalization, length, or grammar
/// rules); two instances agree on an ApiId only when the strings are equal.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ApiId(String);

impl ApiId {
    /// Wrap an API identity string.
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// The exact identity string.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ApiKind {
    Anthropic,
    OpenAi,
    Google,
    Mistral,
}
