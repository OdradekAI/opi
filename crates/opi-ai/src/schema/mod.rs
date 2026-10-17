//! Private TypeBox JSON-dialect schema engine (DEL-04 substrate).
//!
//! This module implements the fixed private JSON dialect selected by
//! `docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-schema.md`: the
//! semantics of the pinned TypeBox 1.3.27 `Compile` evaluation path over
//! trusted JSON schemas, evaluated by an explicit work-stack machine with
//! typed no-progress recursion failure and no implicit I/O or source-free
//! size/fuel limits. It is deliberately **not** a general JSON Schema
//! validator and not a public conformance API — task 20.10 owns the public
//! normalizer/validator seam built on this engine, and the strict/grammar
//! wire projection is task 20.11.
//!
//! Backend identities: regex leaves compile with `regress` 0.12.0 (the
//! research-pinned feature profile); the Unicode-16 data identity of the
//! regex engine is the registry-published `opi-vendor-regress` dependency
//! resolved at packaging time (tasks 20.8/20.57), not a workspace
//! substitution. URL-family formats use the WHATWG `url` implementation.
//!
//! Known acceptance-set boundaries (documented, not hidden): structurally
//! selected invalid regexes fail admission even in branches the upstream
//! would never visit; URL parsing divergences between the `url` crate and
//! the WHATWG reference parser follow the `url` crate; exact Unicode-16
//! agreement of `\p{...}` classes and IDN domains is proven at packaging
//! time with the vendor family, not by the workspace's stock backend.

mod ecma_number;
mod eval;
mod format_patterns;
mod formats;
mod grapheme;
mod guards;
mod hashing;
mod ir;
pub(crate) mod js;
mod normalize;
mod punycode;
mod refs;

use std::sync::Arc;

use serde_json::Value;

/// Typed failures of the dialect engine. Invalid instances are `Ok(false)`,
/// never errors: errors distinguish compile failure, URI/fragment decode
/// failure and no-progress recursion from ordinary non-matches.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum SchemaError {
    /// A structurally-selected regex leaf failed to compile at admission.
    /// `path` is the JSON-pointer location of the leaf.
    #[error("schema regex at `{path}` failed to compile: {message}")]
    Regex { path: String, message: String },
    /// A WHATWG URL operation or fragment percent-decode failed (the
    /// upstream throws for these).
    #[error("schema URI operation failed: {detail}")]
    Uri { detail: String },
    /// The same schema node, instance location and reference frame was
    /// re-entered without progress; the upstream would overflow the
    /// JavaScript stack here.
    #[error("no-progress recursion in schema evaluation")]
    NoProgress,
    /// An internal evaluator invariant was violated (a bug, not a schema
    /// property).
    #[error("internal evaluator invariant violated: {0}")]
    Internal(&'static str),
}

/// An admitted schema: the trusted schema value plus the compiled regex
/// leaves and the evaluation mode. Cheap to clone for caching.
#[derive(Clone)]
pub struct CompiledSchema {
    inner: Arc<ir::Compiled>,
}

impl std::fmt::Debug for CompiledSchema {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CompiledSchema")
            .field("reducer", &self.inner.reducer)
            .field("regex_nodes", &self.inner.regexes.len())
            .finish_non_exhaustive()
    }
}

pub use normalize::{
    ToolArgumentFailure, ToolArgumentOutcome, validate_tool_arguments, validate_tool_call,
};

/// Admit a trusted schema document into the dialect IR.
///
/// Fails closed on structurally-selected invalid regex leaves (see the
/// module documentation for the divergence from the upstream's lazy
/// compile).
pub fn compile(schema: Value) -> Result<CompiledSchema, SchemaError> {
    Ok(CompiledSchema {
        inner: Arc::new(ir::compile(schema)?),
    })
}

impl CompiledSchema {
    /// The trusted schema value this IR was admitted from (the root of the
    /// reference graph; normalization walks it structurally).
    pub(crate) fn root_value(&self) -> &Value {
        &self.inner.schema
    }

    /// Validate one instance against the admitted schema.
    ///
    /// Returns `Ok(true)` for a valid instance, `Ok(false)` for an invalid
    /// one, and `Err` for typed engine failures (no-progress recursion, URI
    /// failures during reference resolution).
    pub fn check(&self, value: &Value) -> Result<bool, SchemaError> {
        let mut evaluator = eval::Evaluator::new(&self.inner);
        evaluator.run(value)
    }

    /// [`Self::check`] plus the first observed failure site for invalid
    /// instances (redacted; observation only — the verdict is identical).
    pub(crate) fn check_detailed(
        &self,
        value: &Value,
    ) -> Result<(bool, Option<eval::FailureSite>), SchemaError> {
        let mut evaluator = eval::Evaluator::new(&self.inner);
        let valid = evaluator.run(value)?;
        Ok((valid, evaluator.first_failure.take()))
    }
}
