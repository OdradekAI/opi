//! Product-neutral evidence contract: opaque call-graph identities, versioned
//! health, the storage-neutral sink lifecycle, and the resolved-execution
//! manifest value types.
//!
//! This is the Agent Core evidence vocabulary consumed by authorization, the
//! Agent runtime, and the Reference Product file adapter. The loop emits records
//! through this contract; file storage, exporters, Eval, and an `ActiveSnapshot`
//! (Promotion Controller) remain outside Agent Core.
//!
//! ## Redaction boundary
//!
//! Evidence values crossing into a sink are typed structural values, digests,
//! redacted diagnostics, and classified artifact references — never raw
//! credentials, environment values, prompts, tool arguments, tool results, or
//! provider error bodies. The producer classifies and redacts content *before*
//! constructing an evidence value. [`RedactedValue`] is the only constructor
//! for the structured-content channel and it applies [`crate::redact`], so a
//! caller cannot place unredacted structured content into the sink contract.
//! Sink adapters are not the redaction boundary.
//!
//! ## Lifecycle
//!
//! [`EvidenceSink`] distinguishes setup, ordered emission, artifact
//! finalization, and run finalization. A finalized [`FinalizedManifest`] is
//! immutable. A sink failure advances [`EvidenceHealth`] to
//! [`EvidenceHealth::Incomplete`] and cannot be hidden by emitting a normal
//! finalized record through another path: once a lifecycle phase fails, the
//! run's evidence is incomplete and the completed manifest is withheld.

use serde::Serialize;

use crate::diagnostic::RedactionMode;

// ===========================================================================
// Opaque call-graph identities
// ===========================================================================

/// Monotonic run-local sequence number assigned at emission time.
///
/// Ordering is structural (`Ord`); the inner value is opaque and is not reused
/// outside evidence correlation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(transparent)]
pub struct Sequence(u64);

/// Opaque stable run identifier. Persisted as a canonical UUID version 7 to
/// provide a collision-resistant, process-independent identity without a
/// shared process-local counter. Minted by [`IdentityAllocator`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RunId(uuid::Uuid);

/// Opaque stable turn identifier within a run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(transparent)]
pub struct TurnId(u64);

/// Opaque stable call identifier. Provider, tool, retry, compaction, and other
/// call-like activity each receive one, correlated to its parent via an
/// optional [`ParentCallId`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(transparent)]
pub struct CallId(u64);

/// Optional parent-call correlation for retry and nested-call relationships
/// without requiring a future Eval call-graph schema.
pub type ParentCallId = CallId;

/// Explicit kind for a [`CallId`]. `#[non_exhaustive]` because the runtime
/// mapping is not frozen and "other call-like activity" may add kinds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum CallKind {
    /// A provider/model dispatch attempt.
    Provider,
    /// A tool execution.
    Tool,
    /// A retry of a prior call (carries the retried call as its parent).
    Retry,
    /// A context compaction.
    Compaction,
    /// A diagnostic-linked observation.
    Diagnostic,
}

/// Validation failure for a persisted [`RunId`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("run id must be a canonical UUID version 7")]
pub struct RunIdParseError;

impl std::str::FromStr for RunId {
    type Err = RunIdParseError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let uuid = uuid::Uuid::parse_str(value).map_err(|_| RunIdParseError)?;
        if uuid.get_version() != Some(uuid::Version::SortRand)
            || uuid.get_variant() != uuid::Variant::RFC4122
            || uuid.hyphenated().to_string() != value
        {
            return Err(RunIdParseError);
        }
        Ok(Self(uuid))
    }
}

impl std::fmt::Display for RunId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&self.0.hyphenated(), f)
    }
}

impl Serialize for RunId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.collect_str(self)
    }
}

impl<'de> serde::Deserialize<'de> for RunId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = <String as serde::Deserialize>::deserialize(deserializer)?;
        value.parse().map_err(serde::de::Error::custom)
    }
}

/// Mints opaque identities and a monotonic run-local [`Sequence`] for one run.
///
/// One allocator belongs to one run. The loop (17.6) holds it; identities are
/// minted immediately before the corresponding lifecycle evidence is emitted so
/// correlation precedes any external effect. `RunId` collision resistance
/// across processes comes from UUID version 7; run-internal `TurnId`, `CallId`,
/// and `Sequence` are minted monotonically by this allocator.
pub struct IdentityAllocator {
    run: RunId,
    next_turn: u64,
    next_call: u64,
    next_sequence: u64,
}

impl IdentityAllocator {
    /// Begin a new run with a fresh opaque [`RunId`].
    pub fn new() -> Self {
        Self {
            run: RunId(uuid::Uuid::now_v7()),
            next_turn: 1,
            next_call: 1,
            next_sequence: 0,
        }
    }

    /// The stable run identifier for this run.
    pub fn run_id(&self) -> RunId {
        self.run
    }

    /// Mint the next turn identifier.
    pub fn next_turn(&mut self) -> TurnId {
        let t = self.next_turn;
        self.next_turn += 1;
        TurnId(t)
    }

    /// Mint the next call identifier.
    pub fn next_call(&mut self) -> CallId {
        let c = self.next_call;
        self.next_call += 1;
        CallId(c)
    }

    /// Mint the next monotonic run-local sequence number.
    pub fn next_sequence(&mut self) -> Sequence {
        let s = self.next_sequence;
        self.next_sequence += 1;
        Sequence(s)
    }
}

impl Default for IdentityAllocator {
    fn default() -> Self {
        Self::new()
    }
}

// ===========================================================================
// Versioned health
// ===========================================================================

/// Monotonic health generation. Advanced every time [`EvidenceHealth`] changes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(transparent)]
pub struct EvidenceGeneration(u64);

impl EvidenceGeneration {
    /// The initial healthy generation.
    pub const INITIAL: EvidenceGeneration = EvidenceGeneration(0);

    /// Advance to the next generation.
    pub fn next(self) -> EvidenceGeneration {
        EvidenceGeneration(self.0 + 1)
    }
}

/// Lifecycle phase whose failure makes evidence incomplete. Closed: these are
/// the three distinguishable failure origins carried by
/// [`EvidenceHealth::Incomplete`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceFailureCode {
    /// Setup (before-run) failure.
    Setup,
    /// Emission (during-run) failure.
    Emission,
    /// Artifact or run finalization (after-run) failure.
    Finalization,
}

/// Closed, versioned, run-local evidence health. Agent Core advances it
/// immediately when setup, emission, or finalization fails: the loop owns
/// during-run transitions and [`crate::agent::AgentRunResult`] owns post-loop
/// compaction/finalization transitions. Sinks do not expose a mutable health
/// handle; authorizers receive a *copy* in each request (17.4), so
/// authorization never shares mutable health with the sink.
///
/// The two variants are exhaustive (not `#[non_exhaustive]`): health is either
/// [`EvidenceHealth::Healthy`] or [`EvidenceHealth::Incomplete`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum EvidenceHealth {
    /// Evidence is healthy as of `generation`.
    Healthy {
        /// Current health generation.
        generation: EvidenceGeneration,
    },
    /// Evidence became incomplete at `generation` because of `first_failure_code`.
    /// Later failures do not overwrite the first failure code; only the
    /// generation advances.
    Incomplete {
        /// Generation at which the first failure was observed.
        generation: EvidenceGeneration,
        /// The lifecycle phase of the first failure.
        first_failure_code: EvidenceFailureCode,
    },
}

impl EvidenceHealth {
    /// Start a healthy run at the initial generation.
    pub fn healthy() -> Self {
        EvidenceHealth::Healthy {
            generation: EvidenceGeneration::INITIAL,
        }
    }

    /// Advance health on a lifecycle failure. The first failure fixes
    /// `first_failure_code`; subsequent failures only advance the generation.
    /// A [`EvidenceHealth::Healthy`] value becomes incomplete.
    pub fn advance_on_failure(&mut self, code: EvidenceFailureCode) {
        let (generation, first_failure_code) = match self {
            EvidenceHealth::Healthy { generation } => (generation.next(), code),
            EvidenceHealth::Incomplete {
                generation,
                first_failure_code,
            } => (generation.next(), *first_failure_code),
        };
        *self = EvidenceHealth::Incomplete {
            generation,
            first_failure_code,
        };
    }

    /// Whether evidence is currently healthy.
    pub fn is_healthy(&self) -> bool {
        matches!(self, EvidenceHealth::Healthy { .. })
    }

    /// The current generation, regardless of variant.
    pub fn generation(&self) -> EvidenceGeneration {
        match self {
            EvidenceHealth::Healthy { generation } => *generation,
            EvidenceHealth::Incomplete { generation, .. } => *generation,
        }
    }
}

// ===========================================================================
// Typed lifecycle failure outcomes
// ===========================================================================

/// Closed typed evidence-lifecycle error returned by [`EvidenceSink`] methods.
///
/// Together with [`EvidenceHealth::Incomplete`], these are the four
/// distinguishable evidence-contract outcomes a caller matches by variant
/// without parsing strings:
///
/// - [`EvidenceError::Setup`] — setup (before-run) failure;
/// - [`EvidenceError::Emission`] — emission (during-run) failure;
/// - [`EvidenceError::Finalization`] — artifact or run finalization failure;
/// - [`EvidenceHealth::Incomplete`] — the run's evidence is incomplete.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum EvidenceError {
    /// Setup failed before the run could begin.
    #[error("evidence setup failed: {detail}")]
    Setup {
        /// Redacted, non-secret reason.
        detail: String,
    },
    /// A record emission failed during the run.
    #[error("evidence emission failed: {detail}")]
    Emission {
        /// Redacted, non-secret reason.
        detail: String,
    },
    /// Artifact or run finalization failed.
    #[error("evidence finalization failed: {detail}")]
    Finalization {
        /// Redacted, non-secret reason.
        detail: String,
    },
}

impl EvidenceError {
    /// The closed failure code for this error.
    pub fn failure_code(&self) -> EvidenceFailureCode {
        match self {
            EvidenceError::Setup { .. } => EvidenceFailureCode::Setup,
            EvidenceError::Emission { .. } => EvidenceFailureCode::Emission,
            EvidenceError::Finalization { .. } => EvidenceFailureCode::Finalization,
        }
    }
}

// ===========================================================================
// Runtime input binding
// ===========================================================================

/// Opaque content digest over resolved material. Producers compute the digest;
/// the sink stores only the digest, never the payload.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
#[serde(transparent)]
pub struct ContentDigest(String);

/// Validation failure for a canonical SHA-256 content digest.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("content digest must be exactly 64 lowercase hexadecimal characters")]
pub struct ContentDigestError;

impl ContentDigest {
    /// Construct a digest from its canonical SHA-256 hex rendering.
    ///
    /// # Errors
    ///
    /// Returns [`ContentDigestError`] unless the input is exactly 64 lowercase
    /// hexadecimal characters.
    pub fn from_hex(hex: impl Into<String>) -> Result<Self, ContentDigestError> {
        let hex = hex.into();
        if hex.len() != 64
            || !hex
                .as_bytes()
                .iter()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(byte))
        {
            return Err(ContentDigestError);
        }
        Ok(Self(hex))
    }

    /// The hex rendering of the digest.
    pub fn as_hex(&self) -> &str {
        &self.0
    }
}

/// Validation failure for a product- or embedder-owned opaque identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("opaque identity must be non-empty, trimmed, and contain no control characters")]
pub struct OpaqueIdentityError;

macro_rules! opaque_identity {
    ($(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
        #[serde(transparent)]
        pub struct $name(String);

        impl $name {
            /// Construct a validated opaque identity owned by trusted assembly.
            ///
            /// # Errors
            ///
            /// Returns [`OpaqueIdentityError`] when the value is empty, has
            /// leading or trailing whitespace, or contains control characters.
            pub fn new(value: impl Into<String>) -> Result<Self, OpaqueIdentityError> {
                let value = value.into();
                if value.is_empty()
                    || value.trim() != value
                    || value.chars().any(char::is_control)
                {
                    return Err(OpaqueIdentityError);
                }
                Ok(Self(value))
            }

            /// Return the opaque identity as a string slice.
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str(&self.0)
            }
        }

        impl<'de> serde::Deserialize<'de> for $name {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                let value = <String as serde::Deserialize>::deserialize(deserializer)?;
                Self::new(value).map_err(serde::de::Error::custom)
            }
        }
    };
}

opaque_identity!(
    /// Product- or embedder-owned identity for the trusted assembly path that
    /// resolved a direct run. Agent Core assigns no product mode constants.
    AssemblyIdentity
);

opaque_identity!(
    /// Product- or embedder-owned capability identity assigned by trusted
    /// registration. Agent Core assigns no built-in permission families.
    CapabilityIdentity
);

opaque_identity!(
    /// Opaque reference to the effective policy used for an authorization.
    PolicyReference
);

opaque_identity!(
    /// Opaque reference to a capability permission used for an authorization.
    PermissionReference
);

opaque_identity!(
    /// Opaque permission scope fixed by trusted policy assembly.
    PermissionScope
);

opaque_identity!(
    /// Opaque reference to a separately versioned scoped grant.
    ScopedGrantReference
);

opaque_identity!(
    /// Trusted provider-visible tool identity retained in typed evidence.
    ToolIdentity
);

opaque_identity!(
    /// Trusted tool-registration reference retained in typed evidence.
    ToolRegistrationReference
);

opaque_identity!(
    /// Opaque trusted session invocation reference used by tool evidence.
    InvocationSessionReference
);

opaque_identity!(
    /// Stable controlled authorization outcome code.
    AuthorizationCode
);

/// Opaque reference to a future Promotion-Controller-selected snapshot.
/// Reserved: the current Reference Product has no Promotion Controller and must
/// not fabricate this authority. [`RuntimeInputBinding`] exposes no constructor
/// that produces an [`RuntimeInputBinding::ActiveSnapshot`] for a direct run.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
#[serde(transparent)]
pub struct SnapshotRef(String);

impl SnapshotRef {
    /// Construct a snapshot reference. This is intentionally not surfaced
    /// through a direct-run binding constructor; it exists for the future
    /// trusted Promotion Controller path.
    pub fn new(reference: impl Into<String>) -> Self {
        Self(reference.into())
    }
}

/// Closed runtime-input binding carried by every run. The two variants are
/// distinguishable in evidence and cannot be normalized into one another.
///
/// Direct product or embedder assembly uses
/// [`RuntimeInputBinding::DirectRuntimeInput`], whose digest covers the
/// resolved material runtime inputs and whose assembly identity remains opaque
/// to Agent Core. [`RuntimeInputBinding::ActiveSnapshot`] is accepted only when
/// a future trusted Promotion Controller supplies its reference.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum RuntimeInputBinding {
    /// Direct assembly: the material runtime inputs are resolved and digested
    /// at assembly time.
    DirectRuntimeInput {
        /// Digest over the resolved material runtime inputs.
        digest: ContentDigest,
        /// Trusted product- or embedder-owned assembly identity.
        assembly_source: AssemblyIdentity,
    },
    /// A Promotion-Controller-selected immutable snapshot. Reserved for a
    /// future trusted authority; not produced by any current assembly path.
    ActiveSnapshot {
        /// Opaque reference to the selected snapshot.
        snapshot_ref: SnapshotRef,
    },
}

impl RuntimeInputBinding {
    /// The only direct-run constructor: produces a
    /// [`RuntimeInputBinding::DirectRuntimeInput`]. No constructor here
    /// fabricates an [`RuntimeInputBinding::ActiveSnapshot`].
    pub fn direct(digest: ContentDigest, assembly_source: AssemblyIdentity) -> Self {
        RuntimeInputBinding::DirectRuntimeInput {
            digest,
            assembly_source,
        }
    }

    /// Whether this binding is a direct (non-snapshot) assembly.
    pub fn is_direct(&self) -> bool {
        matches!(self, RuntimeInputBinding::DirectRuntimeInput { .. })
    }
}

// ===========================================================================
// Measurements
// ===========================================================================

/// Origin of a measured value. Requested, resolved, and actual *route* facts
/// are kept distinct by [`RouteFacts`]; provider-reported usage is kept
/// distinct from estimated, quota, and billed values by tagging each
/// [`Measurement`] with one of these origins.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum MeasurementOrigin {
    /// Reported by the provider on the wire.
    ProviderReported,
    /// Estimated locally (not reported).
    Estimated,
    /// An account/service quota fact.
    Quota,
    /// A billing-system fact.
    Billed,
}

/// Typed reason a measurement is unknown. Distinct from a measured zero: an
/// unknown measurement is never converted to zero.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum UnknownReason {
    /// The provider did not report the value.
    NotReported,
    /// The value was withheld for safety/privacy.
    Withheld,
    /// The value is not yet known (e.g. pending finalization).
    PendingFinalization,
}

/// A measured or unknown value. A measured zero is [`Measurement::Known`] with
/// `value == 0`; an unknown value is [`Measurement::Unknown`] with a reason and
/// is never collapsed to zero.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum Measurement {
    /// A known measured value with its origin.
    Known {
        /// The measured magnitude.
        value: u64,
        /// Where the measurement came from.
        origin: MeasurementOrigin,
    },
    /// An unknown value with a typed reason.
    Unknown {
        /// Why the value is unknown.
        reason: UnknownReason,
    },
}

impl Measurement {
    /// Convenience: a provider-reported measured value.
    pub fn provider_reported(value: u64) -> Self {
        Measurement::Known {
            value,
            origin: MeasurementOrigin::ProviderReported,
        }
    }

    /// Whether this measurement is unknown (as opposed to a measured value,
    /// including a measured zero).
    pub fn is_unknown(&self) -> bool {
        matches!(self, Measurement::Unknown { .. })
    }
}

// ===========================================================================
// Classified artifact references
// ===========================================================================

/// Logical role an artifact plays in the run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum ArtifactRole {
    /// A user prompt.
    Prompt,
    /// A system instruction.
    SystemInstruction,
    /// A tool definition schema.
    ToolSchema,
    /// A tool invocation input.
    ToolInput,
    /// A tool invocation result.
    ToolResult,
    /// A provider request or response body.
    ProviderBody,
}

/// Sensitivity classification of an artifact's content. Determines producer
/// redaction before the reference crosses into the sink.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum SensitivityClassification {
    /// Non-sensitive structured content.
    Public,
    /// Potentially sensitive; redacted before reference construction.
    Sensitive,
    /// Secret-bearing; only a digest ever crosses, never content.
    Secret,
}

/// Finalization state of an artifact reference.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum FinalizationState {
    /// Still being produced.
    Pending,
    /// Finalized and immutable.
    Finalized,
}

/// Reference to a finalized-or-pending artifact. Contains a logical role, media
/// type, content digest, location/reference, sensitivity classification, and
/// finalization state. It **never** embeds the artifact payload.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ArtifactReference {
    /// Logical role.
    pub role: ArtifactRole,
    /// Media type of the artifact content.
    pub media_type: MediaType,
    /// Digest of the artifact content (payload is not embedded).
    pub content_digest: ContentDigest,
    /// Location or opaque reference to the artifact.
    pub location: ArtifactLocation,
    /// Sensitivity classification governing producer redaction.
    pub sensitivity: SensitivityClassification,
    /// Finalization state.
    pub finalization: FinalizationState,
}

/// Media type of an artifact. Opaque token; producers assert correctness.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
#[serde(transparent)]
pub struct MediaType(String);

impl MediaType {
    /// Construct a media type token (e.g. `"application/json"`).
    pub fn new(media_type: impl Into<String>) -> Self {
        Self(media_type.into())
    }
}

/// Location or opaque reference to an artifact's stored content.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
#[serde(transparent)]
pub struct ArtifactLocation(String);

impl ArtifactLocation {
    /// Construct an artifact location/reference.
    pub fn new(location: impl Into<String>) -> Self {
        Self(location.into())
    }
}

// ===========================================================================
// Resolved-execution manifest values
// ===========================================================================

/// Opaque stable identifier for a session branch.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
#[serde(transparent)]
pub struct SessionBranchRef(String);

impl SessionBranchRef {
    fn new(reference: impl Into<String>) -> Self {
        Self(reference.into())
    }

    /// Read the opaque session-branch reference.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Required session binding for a finalized manifest. A run either binds an
/// exact session branch or explicitly attests that it was non-session; absence
/// is not representable.
///
/// ```compile_fail
/// use opi_agent::evidence::{SessionBinding, SessionBranchRef};
/// let _ = SessionBinding::Branch {
///     reference: SessionBranchRef::new(""),
/// };
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SessionBinding {
    /// Exact session branch that produced the run.
    Branch {
        /// Opaque branch reference.
        reference: SessionBranchRef,
    },
    /// Trusted assembly explicitly ran without a session.
    NoSession,
}

impl SessionBinding {
    /// Construct a non-empty, trimmed session-branch binding.
    pub fn branch(reference: impl Into<String>) -> Result<Self, EvidenceFactError> {
        let reference = reference.into();
        validate_fact_identity(&reference, "session.branch")?;
        Ok(Self::Branch {
            reference: SessionBranchRef::new(reference),
        })
    }
}

/// Canonical provider:model plus wire selection at one stage of resolution.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RouteSelection {
    /// Provider identifier.
    provider_id: String,
    /// Model identifier within the provider.
    model_id: String,
    /// Wire protocol used for the call.
    wire: opi_ai::WireApi,
}

/// A malformed typed evidence fact. Construction fails rather than inventing
/// a configured/default fact at an evidence boundary.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum EvidenceFactError {
    /// A provider, model, tool, registration, session, or code identity was
    /// empty, untrimmed, or contained control characters.
    #[error("{field} must be non-empty, trimmed, and contain no control characters")]
    InvalidIdentity {
        /// Name of the invalid evidence field.
        field: &'static str,
    },
    /// A requested model selection was not canonical `provider:model`.
    #[error("requested route is not canonical provider:model")]
    InvalidRequestedRoute,
    /// `opi-ai` added a provenance variant that this evidence contract cannot
    /// represent exactly yet.
    #[error("authentication provenance is not representable by this evidence contract")]
    UnsupportedAuthProvenance,
    /// A used fallback contradicts the selected source or does not change
    /// source.
    #[error("authentication fallback provenance is inconsistent: {reason}")]
    InconsistentAuthFallback {
        /// Exact structural contradiction.
        reason: AuthFallbackInconsistency,
    },
}

/// Structural contradiction in used authentication fallback provenance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum AuthFallbackInconsistency {
    /// The selected auth source differs from the fallback target.
    #[error("fallback target does not match the selected auth source")]
    TargetDoesNotMatchSelectedSource,
    /// A claimed fallback did not move between distinct sources.
    #[error("fallback source and target are identical")]
    SourceEqualsTarget,
}

fn validate_fact_identity(value: &str, field: &'static str) -> Result<(), EvidenceFactError> {
    if value.is_empty() || value.trim() != value || value.chars().any(char::is_control) {
        return Err(EvidenceFactError::InvalidIdentity { field });
    }
    Ok(())
}

impl RouteSelection {
    /// Construct one validated resolved or actual route.
    pub fn new(
        provider_id: impl Into<String>,
        model_id: impl Into<String>,
        wire: opi_ai::WireApi,
    ) -> Result<Self, EvidenceFactError> {
        let provider_id = provider_id.into();
        let model_id = model_id.into();
        validate_fact_identity(&provider_id, "provider_id")?;
        validate_fact_identity(&model_id, "model_id")?;
        Ok(Self {
            provider_id,
            model_id,
            wire,
        })
    }

    /// Provider identifier.
    pub fn provider_id(&self) -> &str {
        &self.provider_id
    }

    /// Model identifier within the provider.
    pub fn model_id(&self) -> &str {
        &self.model_id
    }

    /// Exact wire protocol selected for the route.
    pub fn wire(&self) -> opi_ai::WireApi {
        self.wire
    }
}

/// Exact provider/model selection supplied by the caller. A request does not
/// independently select a wire, so wire truth begins at [`RouteSelection`]
/// after collection resolution.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RequestedRoute {
    provider_id: String,
    model_id: String,
}

impl RequestedRoute {
    /// Construct a validated canonical requested route.
    pub fn new(
        provider_id: impl Into<String>,
        model_id: impl Into<String>,
    ) -> Result<Self, EvidenceFactError> {
        let provider_id = provider_id.into();
        let model_id = model_id.into();
        validate_fact_identity(&provider_id, "requested.provider_id")?;
        validate_fact_identity(&model_id, "requested.model_id")?;
        Ok(Self {
            provider_id,
            model_id,
        })
    }

    /// Provider identifier requested by the caller.
    pub fn provider_id(&self) -> &str {
        &self.provider_id
    }

    /// Model identifier requested by the caller.
    pub fn model_id(&self) -> &str {
        &self.model_id
    }
}

/// Provider-reported actual route. The response can report provider/model
/// without reporting its exact wire; that state remains distinct from a fully
/// reported route and retains a typed reason.
///
/// ```compile_fail
/// use opi_agent::evidence::{ActualRoute, UnknownReason};
/// let _ = ActualRoute::WireUnknown {
///     provider_id: String::new(),
///     model_id: "model".to_owned(),
///     reason: UnknownReason::NotReported,
/// };
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ActualRoute {
    /// Provider, model, and wire were all reported.
    Reported {
        /// Fully reported route.
        route: RouteSelection,
    },
    /// Provider/model were reported but the exact wire was not.
    WireUnknown {
        /// Validated provider/model reported by the response.
        #[serde(flatten)]
        route: RequestedRoute,
        /// Typed reason the wire is unknown.
        reason: UnknownReason,
    },
    /// No actual route fact was reported.
    Unknown {
        /// Typed reason the route is unknown.
        reason: UnknownReason,
    },
}

impl ActualRoute {
    /// Convert provider-reported provider/model fields into actual-route
    /// evidence. An empty field means the provider did not report a complete
    /// route; any non-empty field must still be well formed.
    pub fn from_reported_provider_model(
        provider_id: impl Into<String>,
        model_id: impl Into<String>,
        unknown_reason: UnknownReason,
    ) -> Result<Self, EvidenceFactError> {
        let provider_id = provider_id.into();
        let model_id = model_id.into();
        if !provider_id.is_empty() {
            validate_fact_identity(&provider_id, "actual.provider_id")?;
        }
        if !model_id.is_empty() {
            validate_fact_identity(&model_id, "actual.model_id")?;
        }
        if provider_id.is_empty() || model_id.is_empty() {
            return Ok(Self::unknown(unknown_reason));
        }
        Ok(Self::WireUnknown {
            route: RequestedRoute {
                provider_id,
                model_id,
            },
            reason: unknown_reason,
        })
    }

    /// Construct an actual provider/model fact whose wire is unknown.
    pub fn wire_unknown(
        provider_id: impl Into<String>,
        model_id: impl Into<String>,
        reason: UnknownReason,
    ) -> Result<Self, EvidenceFactError> {
        let provider_id = provider_id.into();
        let model_id = model_id.into();
        validate_fact_identity(&provider_id, "actual.provider_id")?;
        validate_fact_identity(&model_id, "actual.model_id")?;
        Ok(Self::WireUnknown {
            route: RequestedRoute {
                provider_id,
                model_id,
            },
            reason,
        })
    }

    /// Construct a fully provider-reported actual route.
    pub fn reported(route: RouteSelection) -> Self {
        Self::Reported { route }
    }

    /// Construct a typed unknown actual route.
    pub fn unknown(reason: UnknownReason) -> Self {
        Self::Unknown { reason }
    }
}

/// Requested, resolved, and actual route facts. The three stages are distinct
/// fields and cannot be conflated; unknown actual facts are
/// represented by [`ActualRoute`] rather than empty strings or a fabricated
/// configured wire.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RouteFacts {
    /// What the caller requested.
    requested: RequestedRoute,
    /// What route selection resolved.
    resolved: RouteSelection,
    /// What route the dispatch actually used.
    actual: ActualRoute,
}

impl RouteFacts {
    /// Construct exact requested, resolved, and actual route facts.
    pub fn new(requested: RequestedRoute, resolved: RouteSelection, actual: ActualRoute) -> Self {
        Self {
            requested,
            resolved,
            actual,
        }
    }

    /// Requested provider/model.
    pub fn requested(&self) -> &RequestedRoute {
        &self.requested
    }

    /// Resolved provider/model/wire.
    pub fn resolved(&self) -> &RouteSelection {
        &self.resolved
    }

    /// Provider-reported actual route or typed unknown state.
    pub fn actual(&self) -> &ActualRoute {
        &self.actual
    }
}

/// Typed, non-secret authentication source. Supplementary environment/store/
/// OAuth/AWS labels are retained exactly; no secret value crosses.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AuthSourceFacts {
    /// A static configured credential.
    Static,
    /// A credential read from a named environment variable.
    Environment {
        /// Non-secret variable name, never its value.
        name: String,
    },
    /// A credential read from a named store kind.
    CredentialStore {
        /// Non-secret store kind.
        kind: String,
    },
    /// A credential obtained through a named OAuth integration.
    Oauth {
        /// Non-secret OAuth kind.
        kind: String,
    },
    /// AWS SigV4 credential-chain provenance.
    AwsSigV4 {
        /// Exact non-secret AWS source.
        source: AwsCredentialSourceFacts,
    },
}

/// Exact non-secret AWS credential-chain source.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AwsCredentialSourceFacts {
    /// Complete credentials supplied directly by configuration.
    ExplicitConfig,
    /// Configured access-key input paired with named environment variables.
    ConfiguredEnvironment {
        /// Secret-access-key environment variable name.
        secret_access_key_env: String,
        /// Optional session-token environment variable name.
        session_token_env: Option<String>,
    },
    /// Standard AWS environment variables.
    Environment,
    /// AWS shared credentials profile.
    ProfileFile,
    /// AWS shared config profile.
    ConfigFile,
    /// AWS shared config `credential_process`.
    CredentialProcess,
}

/// Redacted text that is safe to serialize across an evidence sink boundary.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct RedactedEvidenceText(String);

impl RedactedEvidenceText {
    /// Redact a non-secret-controlled summary defensively before evidence
    /// construction. Query credentials and secret-like tokens are removed.
    pub fn new(value: &str) -> Self {
        Self(crate::redact_text(value, RedactionMode::Summary))
    }

    /// Read the already-redacted text.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Exact authentication fallback attempt state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum AuthFallbackFacts {
    /// No fallback was attempted.
    NotAttempted,
    /// An explicitly allowed fallback was used.
    Used {
        /// Source attempted first.
        from: AuthSourceFacts,
        /// Source that resolved authentication.
        to: AuthSourceFacts,
        /// Stable reason, defensively redacted before construction.
        stable_reason: RedactedEvidenceText,
    },
}

/// Non-secret authentication and fallback provenance. Construction from
/// `opi-ai` is fallible so future unsupported variants cannot silently become
/// static/no-fallback facts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ProvenanceFacts {
    auth_source: AuthSourceFacts,
    fallback: AuthFallbackFacts,
}

impl ProvenanceFacts {
    /// Convert the exact prepared-auth provenance into evidence facts.
    pub fn from_auth(provenance: &opi_ai::auth::AuthProvenance) -> Result<Self, EvidenceFactError> {
        Ok(Self {
            auth_source: auth_source_facts(&provenance.source)?,
            fallback: match &provenance.fallback {
                opi_ai::auth::AuthFallback::NotAttempted => AuthFallbackFacts::NotAttempted,
                opi_ai::auth::AuthFallback::Used { from, to, reason } => {
                    if to != &provenance.source {
                        return Err(EvidenceFactError::InconsistentAuthFallback {
                            reason: AuthFallbackInconsistency::TargetDoesNotMatchSelectedSource,
                        });
                    }
                    if from == to {
                        return Err(EvidenceFactError::InconsistentAuthFallback {
                            reason: AuthFallbackInconsistency::SourceEqualsTarget,
                        });
                    }
                    AuthFallbackFacts::Used {
                        from: auth_source_facts(from)?,
                        to: auth_source_facts(to)?,
                        stable_reason: RedactedEvidenceText::new(reason),
                    }
                }
                _ => return Err(EvidenceFactError::UnsupportedAuthProvenance),
            },
        })
    }

    /// Exact non-secret authentication source.
    pub fn auth_source(&self) -> &AuthSourceFacts {
        &self.auth_source
    }

    /// Exact fallback attempt state.
    pub fn fallback(&self) -> &AuthFallbackFacts {
        &self.fallback
    }
}

fn auth_source_facts(
    source: &opi_ai::auth::AuthProvenanceSource,
) -> Result<AuthSourceFacts, EvidenceFactError> {
    use opi_ai::auth::{AuthProvenanceSource, AwsCredentialSource};
    let facts = match source {
        AuthProvenanceSource::Static => AuthSourceFacts::Static,
        AuthProvenanceSource::Environment { name } => {
            validate_fact_identity(name, "auth.environment.name")?;
            AuthSourceFacts::Environment { name: name.clone() }
        }
        AuthProvenanceSource::CredentialStore { kind } => {
            validate_fact_identity(kind, "auth.credential_store.kind")?;
            AuthSourceFacts::CredentialStore { kind: kind.clone() }
        }
        AuthProvenanceSource::OAuth { kind } => {
            validate_fact_identity(kind, "auth.oauth.kind")?;
            AuthSourceFacts::Oauth { kind: kind.clone() }
        }
        AuthProvenanceSource::AwsSigV4 { source } => AuthSourceFacts::AwsSigV4 {
            source: match source {
                AwsCredentialSource::ExplicitConfig => AwsCredentialSourceFacts::ExplicitConfig,
                AwsCredentialSource::ConfiguredEnvironment {
                    secret_access_key_env,
                    session_token_env,
                } => {
                    validate_fact_identity(
                        secret_access_key_env,
                        "auth.aws.secret_access_key_env",
                    )?;
                    if let Some(session_token_env) = session_token_env {
                        validate_fact_identity(session_token_env, "auth.aws.session_token_env")?;
                    }
                    AwsCredentialSourceFacts::ConfiguredEnvironment {
                        secret_access_key_env: secret_access_key_env.clone(),
                        session_token_env: session_token_env.clone(),
                    }
                }
                AwsCredentialSource::Environment => AwsCredentialSourceFacts::Environment,
                AwsCredentialSource::ProfileFile => AwsCredentialSourceFacts::ProfileFile,
                AwsCredentialSource::ConfigFile => AwsCredentialSourceFacts::ConfigFile,
                AwsCredentialSource::CredentialProcess => {
                    AwsCredentialSourceFacts::CredentialProcess
                }
            },
        },
        _ => return Err(EvidenceFactError::UnsupportedAuthProvenance),
    };
    Ok(facts)
}

/// Complete typed provider evidence for one logical call. Before dispatch the
/// actual route is [`ActualRoute::Unknown`]; a terminal record can replace it
/// with exact provider-reported facts without parsing JSON tokens.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ProviderEvidenceFacts {
    /// Requested/resolved/actual route facts.
    pub route: RouteFacts,
    /// Exact non-secret prepared-auth provenance.
    pub provenance: ProvenanceFacts,
}

impl ProviderEvidenceFacts {
    /// Build exact typed provider evidence from one prepared logical call.
    pub fn from_prepared(
        requested_spec: &str,
        route: &opi_ai::PreparedRoute,
        provenance: &opi_ai::auth::AuthProvenance,
    ) -> Result<Self, EvidenceFactError> {
        let (requested_provider, requested_model) =
            opi_ai::registry::parse_model_spec(requested_spec)
                .map_err(|_| EvidenceFactError::InvalidRequestedRoute)?;
        Ok(Self {
            route: RouteFacts::new(
                RequestedRoute::new(requested_provider, requested_model)?,
                RouteSelection::new(
                    route.provider_id.clone(),
                    route.model_id.clone(),
                    route.wire_api,
                )?,
                ActualRoute::unknown(UnknownReason::NotReported),
            ),
            provenance: ProvenanceFacts::from_auth(provenance)?,
        })
    }

    /// Replace the pre-dispatch unknown actual route with provider-reported
    /// terminal facts while preserving the exact requested/resolved route and
    /// prepared-auth provenance.
    pub fn with_actual(mut self, actual: ActualRoute) -> Self {
        self.route.actual = actual;
        self
    }
}

/// Why provider route and authentication facts do not apply to a resolved
/// execution manifest.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderNotApplicableReason {
    /// The execution contained only standalone context compaction and made no
    /// provider call.
    StandaloneCompaction,
    /// The run was cancelled before provider preparation or dispatch began.
    CancelledBeforeProvider,
}

/// Provider facts for a resolved execution.
///
/// A provider-backed run retains its validated route and prepared-auth
/// provenance together. A standalone compaction uses an explicit closed
/// not-applicable reason, so product assembly never fabricates a configured
/// route or static authentication source for activity that made no provider
/// call. A run cancelled before provider preparation uses a distinct closed
/// reason for the same purpose.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ProviderInvocationFacts {
    /// A provider call occurred.
    Applicable {
        /// Requested, resolved, and provider-reported actual route.
        route: RouteFacts,
        /// Exact prepared-auth provenance.
        provenance: Box<ProvenanceFacts>,
    },
    /// No provider call applied to this execution.
    NotApplicable {
        /// Closed reason provider facts do not apply.
        reason: ProviderNotApplicableReason,
    },
}

impl ProviderInvocationFacts {
    /// Construct facts for a provider-backed execution.
    pub fn applicable(route: RouteFacts, provenance: ProvenanceFacts) -> Self {
        Self::Applicable {
            route,
            provenance: Box::new(provenance),
        }
    }

    /// Construct explicit facts for an execution with no provider call.
    pub fn not_applicable(reason: ProviderNotApplicableReason) -> Self {
        Self::NotApplicable { reason }
    }

    /// Route facts when a provider call applied.
    pub fn route(&self) -> Option<&RouteFacts> {
        match self {
            Self::Applicable { route, .. } => Some(route),
            Self::NotApplicable { .. } => None,
        }
    }

    /// Prepared-auth provenance when a provider call applied.
    pub fn provenance(&self) -> Option<&ProvenanceFacts> {
        match self {
            Self::Applicable { provenance, .. } => Some(provenance.as_ref()),
            Self::NotApplicable { .. } => None,
        }
    }
}

/// Explicit invocation/session binding carried by typed tool evidence. A
/// non-session invocation is a first-class fact, not a missing optional value.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum InvocationBinding {
    /// Trusted assembly supplied no session context.
    NoSession,
    /// Trusted assembly supplied an opaque session context.
    Session {
        /// Opaque session reference.
        reference: InvocationSessionReference,
    },
}

impl InvocationBinding {
    /// Construct a validated session invocation binding.
    pub fn session(reference: impl Into<String>) -> Result<Self, OpaqueIdentityError> {
        Ok(Self::Session {
            reference: InvocationSessionReference::new(reference)?,
        })
    }
}

/// Exact typed authorization outcome retained for a tool call.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolAuthorizationFacts {
    /// This outcome-only record carries no authorization decision.
    NotReached,
    /// A fresh allow carried the exact effective policy bindings.
    Allowed {
        /// Effective policy reference.
        policy_ref: PolicyReference,
        /// Permission reference.
        permission_ref: PermissionReference,
        /// Permission scope.
        permission_scope: PermissionScope,
        /// Optional separately versioned scoped grant.
        scoped_grant_ref: Option<ScopedGrantReference>,
    },
    /// Authorization denied with a stable code and redacted reason.
    Denied {
        /// Stable controlled code.
        stable_code: AuthorizationCode,
        /// Redacted reason safe for evidence.
        redacted_reason: RedactedEvidenceText,
    },
}

impl ToolAuthorizationFacts {
    /// Construct a typed denial, validating its stable code and redacting its
    /// explanatory text before it can enter evidence.
    pub fn denied(
        stable_code: impl Into<String>,
        reason: &str,
    ) -> Result<Self, OpaqueIdentityError> {
        Ok(Self::Denied {
            stable_code: AuthorizationCode::new(stable_code)?,
            redacted_reason: RedactedEvidenceText::new(reason),
        })
    }
}

/// Typed lifecycle phase represented by one tool evidence record.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolEvidencePhase {
    /// Pre-execution authorization decision.
    Authorization,
    /// Terminal execution outcome.
    Outcome,
    /// A single post-execution record retaining authorization and terminal outcome.
    Combined,
}

/// Typed terminal facts of a tool call. Raw tool output is deliberately absent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct ToolOutcomeFacts {
    /// Exact lower-boundary execution outcome.
    pub execution: ToolExecutionOutcome,
}

impl ToolOutcomeFacts {
    /// Whether the terminal outcome is an error rather than success.
    pub fn is_error(self) -> bool {
        self.execution != ToolExecutionOutcome::Succeeded
    }
}

/// Closed lower-boundary outcome of one tool execution.
///
/// Partial external effects and unconfirmed cleanup remain distinct from an
/// ordinary failure so the enclosing run cannot later report unqualified
/// success.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolExecutionOutcome {
    /// The tool completed successfully.
    Succeeded,
    /// The tool failed and confirmed that no uncertain external effect remains.
    Failed,
    /// The tool execution was cancelled with no stronger uncertain outcome.
    Cancelled,
    /// An external effect may have occurred before the tool failed.
    PartialSideEffect,
    /// Cleanup of an external effect could not be confirmed.
    CleanupUnknown,
}

/// A malformed typed tool evidence fact.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum ToolEvidenceFactError {
    /// An opaque tool, registration, or related identity is invalid.
    #[error(transparent)]
    InvalidIdentity(#[from] OpaqueIdentityError),
    /// Registration and capability resolution must both be present or absent.
    #[error("tool registration and capability resolution are incomplete")]
    IncompleteResolution,
    /// Authorization/combined records require a reached authorization result.
    #[error("tool authorization was not reached for an authorization-bearing record")]
    AuthorizationNotReached,
}

/// Typed tool evidence. Registration/capability/policy/session facts are
/// structural values, not strings embedded in an ad-hoc JSON protocol.
///
/// ```compile_fail
/// use opi_agent::evidence::ToolEvidenceFacts;
/// let _ = ToolEvidenceFacts {
///     phase: unimplemented!(),
///     tool: unimplemented!(),
///     registration: None,
///     capability: None,
///     invocation: unimplemented!(),
///     authorization: unimplemented!(),
///     outcome: None,
/// };
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ToolEvidenceFacts {
    /// Lifecycle phase represented by this record.
    phase: ToolEvidencePhase,
    /// Trusted provider-visible tool identity.
    tool: ToolIdentity,
    /// Resolved trusted registration, when resolution succeeded.
    registration: Option<ToolRegistrationReference>,
    /// Registration-derived capability, when resolution succeeded.
    capability: Option<CapabilityIdentity>,
    /// Explicit invocation/session binding.
    invocation: InvocationBinding,
    /// Exact authorization state and policy bindings.
    authorization: ToolAuthorizationFacts,
    /// Terminal outcome for outcome/combined records.
    outcome: Option<ToolOutcomeFacts>,
}

impl ToolEvidenceFacts {
    /// Construct one resolved authorization record.
    pub fn authorization(
        tool: impl Into<String>,
        registration: impl Into<String>,
        capability: CapabilityIdentity,
        invocation: InvocationBinding,
        authorization: ToolAuthorizationFacts,
    ) -> Result<Self, ToolEvidenceFactError> {
        if matches!(authorization, ToolAuthorizationFacts::NotReached) {
            return Err(ToolEvidenceFactError::AuthorizationNotReached);
        }
        Ok(Self {
            phase: ToolEvidencePhase::Authorization,
            tool: ToolIdentity::new(tool)?,
            registration: Some(ToolRegistrationReference::new(registration)?),
            capability: Some(capability),
            invocation,
            authorization,
            outcome: None,
        })
    }

    /// Construct a terminal tool outcome record. Unknown registrations remain
    /// explicitly absent rather than receiving a fabricated identity.
    pub fn outcome(
        tool: impl Into<String>,
        registration: Option<&str>,
        capability: Option<CapabilityIdentity>,
        invocation: InvocationBinding,
        execution: ToolExecutionOutcome,
    ) -> Result<Self, ToolEvidenceFactError> {
        let (registration, capability) = tool_resolution(registration, capability)?;
        Ok(Self {
            phase: ToolEvidencePhase::Outcome,
            tool: ToolIdentity::new(tool)?,
            registration,
            capability,
            invocation,
            authorization: ToolAuthorizationFacts::NotReached,
            outcome: Some(ToolOutcomeFacts { execution }),
        })
    }

    /// Construct one post-execution record retaining authorization and outcome.
    pub fn combined(
        tool: impl Into<String>,
        registration: Option<&str>,
        capability: Option<CapabilityIdentity>,
        invocation: InvocationBinding,
        authorization: ToolAuthorizationFacts,
        execution: ToolExecutionOutcome,
    ) -> Result<Self, ToolEvidenceFactError> {
        if matches!(authorization, ToolAuthorizationFacts::NotReached) {
            return Err(ToolEvidenceFactError::AuthorizationNotReached);
        }
        let (registration, capability) = tool_resolution(registration, capability)?;
        Ok(Self {
            phase: ToolEvidencePhase::Combined,
            tool: ToolIdentity::new(tool)?,
            registration,
            capability,
            invocation,
            authorization,
            outcome: Some(ToolOutcomeFacts { execution }),
        })
    }

    /// Lifecycle phase represented by this record.
    pub fn phase(&self) -> ToolEvidencePhase {
        self.phase
    }

    /// Trusted provider-visible tool identity.
    pub fn tool(&self) -> &ToolIdentity {
        &self.tool
    }

    /// Resolved trusted registration, when resolution succeeded.
    pub fn registration(&self) -> Option<&ToolRegistrationReference> {
        self.registration.as_ref()
    }

    /// Registration-derived capability, when resolution succeeded.
    pub fn capability(&self) -> Option<&CapabilityIdentity> {
        self.capability.as_ref()
    }

    /// Explicit invocation/session binding.
    pub fn invocation(&self) -> &InvocationBinding {
        &self.invocation
    }

    /// Exact authorization state and policy bindings.
    pub fn authorization_facts(&self) -> &ToolAuthorizationFacts {
        &self.authorization
    }

    /// Terminal outcome for outcome/combined records.
    pub fn outcome_facts(&self) -> Option<ToolOutcomeFacts> {
        self.outcome
    }
}

fn tool_resolution(
    registration: Option<&str>,
    capability: Option<CapabilityIdentity>,
) -> Result<
    (
        Option<ToolRegistrationReference>,
        Option<CapabilityIdentity>,
    ),
    ToolEvidenceFactError,
> {
    match (registration, capability) {
        (Some(registration), Some(capability)) => Ok((
            Some(ToolRegistrationReference::new(registration)?),
            Some(capability),
        )),
        (None, None) => Ok((None, None)),
        _ => Err(ToolEvidenceFactError::IncompleteResolution),
    }
}

/// Effective user-policy facts snapshotted for the run. The digest addresses
/// the immutable policy; capability/scope/grant are referenced, not embedded.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct UserPolicyFacts {
    /// Digest over the immutable effective user policy.
    pub policy_digest: ContentDigest,
    /// Product- or embedder-owned capability identity, when applicable.
    pub capability: Option<CapabilityIdentity>,
    /// Permission reference selected by the effective policy, when applicable.
    pub permission_ref: Option<PermissionReference>,
    /// Permission scope selected by the effective policy, when applicable.
    pub permission_scope: Option<PermissionScope>,
    /// Separately versioned scoped-grant reference, when applicable.
    pub scoped_grant_ref: Option<ScopedGrantReference>,
}

/// Identity of resolved material runtime inputs (prompt, system instruction,
/// tool schema) via digest or classified artifact reference.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct InputIdentity {
    /// Digest over the resolved prompt.
    pub prompt_digest: ContentDigest,
    /// Digest over the system instruction.
    pub system_digest: Option<ContentDigest>,
    /// Digests over the visible tool schemas.
    pub tool_schema_digests: Vec<ContentDigest>,
}

/// Resolved configuration identity for harness/runtime/adapter/material facts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ConfigIdentity {
    /// Digest over the resolved harness configuration.
    pub harness_digest: ContentDigest,
    /// Digest over the resolved runtime configuration.
    pub runtime_digest: ContentDigest,
    /// Digest over the resolved adapter configuration.
    pub adapter_digest: ContentDigest,
    /// Digest over the resolved material configuration.
    pub material_digest: ContentDigest,
}

/// Typed trigger provenance for one resolved execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ExecutionTrigger {
    /// New caller input initiated the run.
    Invocation,
    /// An explicit retry initiated the run.
    Retry,
    /// A continuation without new caller input initiated the run.
    Continuation,
    /// Context compaction initiated the run/activity.
    Compaction {
        /// Exact compaction cause.
        reason: CompactionTrigger,
    },
}

/// Typed compaction trigger provenance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CompactionTrigger {
    /// Explicit manual compaction.
    Manual,
    /// Configured token/size threshold.
    Threshold,
    /// Provider/context overflow recovery.
    Overflow,
}

/// Closed terminal outcome of one compaction operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CompactionOutcome {
    /// Context replacement completed.
    Succeeded,
    /// Compaction was explicitly aborted before completion.
    Aborted,
    /// Compaction failed without an uncertain external effect.
    Failed,
    /// Compaction may have partially changed an external or durable boundary.
    PartialSideEffect,
    /// Cleanup after compaction could not be confirmed.
    CleanupUnknown,
}

/// Typed compaction lifecycle facts. A start has no outcome; its terminal
/// record carries exactly one [`CompactionOutcome`]. Both records reuse the
/// same evidence call identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct CompactionEvidenceFacts {
    trigger: CompactionTrigger,
    outcome: Option<CompactionOutcome>,
}

impl CompactionEvidenceFacts {
    /// Construct the pre-mutation lifecycle record.
    pub fn started(trigger: CompactionTrigger) -> Self {
        Self {
            trigger,
            outcome: None,
        }
    }

    /// Construct the terminal lifecycle record.
    pub fn terminal(trigger: CompactionTrigger, outcome: CompactionOutcome) -> Self {
        Self {
            trigger,
            outcome: Some(outcome),
        }
    }

    /// Exact compaction trigger.
    pub fn trigger(&self) -> CompactionTrigger {
        self.trigger
    }

    /// `None` for a start and the exact terminal outcome otherwise.
    pub fn outcome(&self) -> Option<CompactionOutcome> {
        self.outcome
    }
}

/// Budget, trigger, time, and platform/environment identity plus measurement
/// origin. Measurements preserve unknown versus zero; trigger is always an
/// explicit typed fact.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct EnvironmentFacts {
    /// Token/turn budget fact, when applicable.
    pub budget: Measurement,
    /// Invocation/environment trigger provenance.
    pub trigger: ExecutionTrigger,
    /// Wall-clock time fact, when applicable.
    pub time: Measurement,
    /// Platform/environment identity.
    pub platform: PlatformIdentity,
}

/// Opaque platform/environment identity.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
#[serde(transparent)]
pub struct PlatformIdentity(String);

impl PlatformIdentity {
    /// Construct a platform identity token.
    pub fn new(identity: impl Into<String>) -> Self {
        Self(identity.into())
    }
}

/// Provider usage separated by origin. Provider-reported values stay distinct
/// from estimated, quota, and billed values.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct UsageFacts {
    /// Input tokens for the call.
    pub input_tokens: Measurement,
    /// Output tokens for the call.
    pub output_tokens: Measurement,
}

/// Run/turn/call/parent/sequence correlation retained by the manifest.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ManifestCorrelation {
    /// Owning run.
    pub run: RunId,
    /// Terminal turn, when applicable.
    pub turn: Option<TurnId>,
    /// Terminal call, when applicable.
    pub call: Option<CallId>,
    /// Parent call of the terminal call, when applicable.
    pub parent: Option<ParentCallId>,
    /// Final sequence number observed.
    pub sequence: Sequence,
}

/// Closed terminal outcome of a run.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum TerminalOutcome {
    /// The run completed normally.
    Success,
    /// The run was cancelled.
    Cancelled,
    /// A provider/tool/retry/compaction error terminated the run.
    Failed,
    /// The run ended with a partial external side effect.
    PartialSideEffect,
    /// Cleanup outcome could not be confirmed.
    CleanupUnknown,
}

/// Closed evidence completeness state. Required-complete-evidence policy
/// (17.7) treats [`EvidenceCompleteness::Incomplete`] as fail-closed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceCompleteness {
    /// All required evidence was captured and finalized.
    Complete,
    /// Evidence is incomplete (a lifecycle phase failed).
    Incomplete,
}

/// Candidate resolved-execution manifest assembled from static and dynamic run
/// facts. It cannot cross [`EvidenceSink::finalize_run`] until
/// [`ManifestCandidate::validate`] consumes it and returns an opaque
/// [`FinalizedManifest`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ManifestCandidate {
    /// Run/turn/call/parent/sequence correlation.
    pub correlation: ManifestCorrelation,
    /// Terminal outcome.
    pub outcome: TerminalOutcome,
    /// Required branch or explicit non-session binding.
    pub session: SessionBinding,
    /// Exact runtime-input binding variant (direct runs are never
    /// [`RuntimeInputBinding::ActiveSnapshot`]).
    pub binding: RuntimeInputBinding,
    /// Resolved configuration identity.
    pub config: ConfigIdentity,
    /// Provider route and authentication facts, or an explicit reason they do
    /// not apply to this execution.
    pub provider: ProviderInvocationFacts,
    /// Effective user-policy facts.
    pub policy: UserPolicyFacts,
    /// Prompt/system/tool-schema identity.
    pub input_identity: InputIdentity,
    /// Budget/time/platform facts.
    pub environment: EnvironmentFacts,
    /// Provider usage by origin.
    pub usage: UsageFacts,
    /// Finalized artifact references.
    pub artifacts: Vec<ArtifactReference>,
    /// Evidence completeness.
    pub completeness: EvidenceCompleteness,
}

/// Validated immutable resolved-execution manifest accepted by
/// [`EvidenceSink::finalize_run`]. Its inner candidate is private, so a caller
/// cannot mutate validated facts or bypass [`ManifestCandidate::validate`].
///
/// ```compile_fail
/// use opi_agent::evidence::{FinalizedManifest, ManifestCandidate};
/// # fn candidate() -> ManifestCandidate { unimplemented!() }
/// let _ = FinalizedManifest(candidate());
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct FinalizedManifest(ManifestCandidate);

impl FinalizedManifest {
    /// Read the required branch-or-non-session binding.
    pub fn session(&self) -> &SessionBinding {
        &self.0.session
    }

    /// Read the validated manifest facts without exposing mutation.
    pub fn facts(&self) -> &ManifestCandidate {
        &self.0
    }

    /// Validate this manifest against the exact lifecycle facts observed by a
    /// recording sink. Recording adapters call this before publishing the
    /// manifest so setup, emission, and artifact facts cannot diverge.
    pub fn validate_observation(
        &self,
        observation: EvidenceRunObservation<'_>,
    ) -> Result<(), EvidenceError> {
        if observation.binding != &self.binding {
            return Err(finalization_error(
                "manifest runtime-input binding does not match evidence setup",
            ));
        }
        let Some(terminal) = observation.records.last() else {
            return Err(finalization_error(
                "manifest correlation requires at least one emitted record",
            ));
        };
        if observation
            .records
            .iter()
            .any(|record| record.run != self.correlation.run)
        {
            return Err(finalization_error(
                "manifest run does not match every emitted record",
            ));
        }
        if observation
            .records
            .windows(2)
            .any(|records| records[0].sequence >= records[1].sequence)
        {
            return Err(finalization_error(
                "emitted evidence sequences are not strictly increasing",
            ));
        }
        for (index, record) in observation.records.iter().enumerate() {
            record.validate_kind_payload()?;
            if record.parent == Some(record.call) {
                return Err(finalization_error(
                    "emitted evidence call cannot parent itself",
                ));
            }
            if observation.records[..index].iter().any(|prior| {
                prior.call == record.call
                    && (prior.kind != record.kind
                        || prior.turn != record.turn
                        || prior.parent != record.parent)
            }) {
                return Err(finalization_error(
                    "records sharing an evidence call must retain kind, turn, and parent",
                ));
            }
            if let Some(parent) = record.parent
                && !observation.records[..index]
                    .iter()
                    .any(|prior| prior.run == record.run && prior.call == parent)
            {
                return Err(finalization_error(
                    "emitted evidence parent does not reference an earlier call",
                ));
            }
        }
        let last_provider = observation.records.iter().rev().find_map(|record| {
            if let EvidencePayload::Provider(facts) = &record.payload {
                Some(facts)
            } else {
                None
            }
        });
        match (&self.provider, last_provider) {
            (ProviderInvocationFacts::Applicable { route, provenance }, Some(observed))
                if route == &observed.route && provenance.as_ref() == &observed.provenance => {}
            (ProviderInvocationFacts::Applicable { .. }, Some(_)) => {
                return Err(finalization_error(
                    "manifest provider facts do not match the last emitted provider record",
                ));
            }
            (ProviderInvocationFacts::Applicable { .. }, None) => {
                return Err(finalization_error(
                    "manifest claims provider facts without an emitted provider record",
                ));
            }
            (
                ProviderInvocationFacts::NotApplicable {
                    reason: ProviderNotApplicableReason::StandaloneCompaction,
                },
                None,
            ) => validate_standalone_compaction_observation(
                &self.environment.trigger,
                observation.records,
            )?,
            (
                ProviderInvocationFacts::NotApplicable {
                    reason: ProviderNotApplicableReason::CancelledBeforeProvider,
                },
                None,
            ) => {
                if self.outcome != TerminalOutcome::Cancelled
                    || !observation.records.iter().any(|record| {
                        matches!(
                            &record.payload,
                            EvidencePayload::Diagnostic(diagnostic)
                                if diagnostic.code
                                    == crate::diagnostic::code::CODE_AGENT_CANCELLED
                        )
                    })
                {
                    return Err(finalization_error(
                        "cancelled-before-provider requires a cancelled terminal outcome and typed cancellation evidence",
                    ));
                }
            }
            (ProviderInvocationFacts::NotApplicable { .. }, Some(_)) => {
                return Err(finalization_error(
                    "provider-not-applicable contradicts emitted provider evidence",
                ));
            }
        }
        validate_compaction_lifecycles(observation.records)?;
        if self.correlation.turn != terminal.turn
            || self.correlation.call != Some(terminal.call)
            || self.correlation.parent != terminal.parent
            || self.correlation.sequence != terminal.sequence
        {
            return Err(finalization_error(
                "manifest terminal correlation does not match the last emitted record",
            ));
        }
        if !same_artifact_set(&self.artifacts, observation.artifacts) {
            return Err(finalization_error(
                "manifest artifacts do not match finalized artifacts",
            ));
        }
        Ok(())
    }
}

fn validate_standalone_compaction_observation(
    environment_trigger: &ExecutionTrigger,
    records: &[EvidenceRecord],
) -> Result<(), EvidenceError> {
    if records.is_empty()
        || !records
            .iter()
            .all(|record| record.kind == CallKind::Compaction)
    {
        return Err(finalization_error(
            "provider-not-applicable requires an exclusively compaction evidence graph",
        ));
    }
    let ExecutionTrigger::Compaction { reason } = environment_trigger else {
        return Err(finalization_error(
            "standalone compaction requires a compaction environment trigger",
        ));
    };
    if records.iter().any(|record| {
        !matches!(
            &record.payload,
            EvidencePayload::Compaction(facts) if facts.trigger() == *reason
        )
    }) {
        return Err(finalization_error(
            "standalone compaction environment trigger does not match lifecycle facts",
        ));
    }
    Ok(())
}

fn validate_compaction_lifecycles(records: &[EvidenceRecord]) -> Result<(), EvidenceError> {
    let mut pending: Vec<(CallId, CompactionTrigger, bool)> = Vec::new();
    for record in records {
        if record.kind != CallKind::Compaction {
            continue;
        }
        let EvidencePayload::Compaction(facts) = &record.payload else {
            return Err(finalization_error(
                "compaction call does not carry compaction lifecycle facts",
            ));
        };
        match facts.outcome() {
            None => {
                if pending.iter().any(|(call, _, _)| *call == record.call) {
                    return Err(finalization_error(
                        "compaction call contains more than one start record",
                    ));
                }
                pending.push((record.call, facts.trigger(), false));
            }
            Some(_) => {
                let Some((_, trigger, terminal_seen)) =
                    pending.iter_mut().find(|(call, _, _)| *call == record.call)
                else {
                    return Err(finalization_error(
                        "compaction terminal has no preceding start record",
                    ));
                };
                if *trigger != facts.trigger() {
                    return Err(finalization_error(
                        "compaction terminal trigger does not match its start",
                    ));
                }
                if *terminal_seen {
                    return Err(finalization_error(
                        "compaction call contains more than one terminal record",
                    ));
                }
                *terminal_seen = true;
            }
        }
    }
    if pending.iter().any(|(_, _, terminal_seen)| !terminal_seen) {
        return Err(finalization_error(
            "compaction start has no terminal outcome record",
        ));
    }
    Ok(())
}

/// Exact setup, emission, and artifact facts observed by a recording evidence
/// sink. Product adapters can reuse this validation input without duplicating
/// correlation rules.
#[derive(Debug, Clone, Copy)]
pub struct EvidenceRunObservation<'a> {
    binding: &'a RuntimeInputBinding,
    records: &'a [EvidenceRecord],
    artifacts: &'a [ArtifactReference],
}

impl<'a> EvidenceRunObservation<'a> {
    /// Borrow one sink's complete pre-finalization lifecycle state.
    pub fn new(
        binding: &'a RuntimeInputBinding,
        records: &'a [EvidenceRecord],
        artifacts: &'a [ArtifactReference],
    ) -> Self {
        Self {
            binding,
            records,
            artifacts,
        }
    }
}

fn finalization_error(detail: &str) -> EvidenceError {
    EvidenceError::Finalization {
        detail: detail.to_owned(),
    }
}

fn same_artifact_set(expected: &[ArtifactReference], observed: &[ArtifactReference]) -> bool {
    if expected.len() != observed.len() {
        return false;
    }
    let mut matched = vec![false; observed.len()];
    expected.iter().all(|expected| {
        observed
            .iter()
            .enumerate()
            .find(|(index, observed)| !matched[*index] && expected == *observed)
            .map(|(index, _)| matched[index] = true)
            .is_some()
    })
}

impl std::ops::Deref for FinalizedManifest {
    type Target = ManifestCandidate;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

// ===========================================================================
// Redacted structured value (producer-boundary enforcement)
// ===========================================================================

/// Structured content channel for evidence. The only constructor applies
/// [`crate::redact`], so unredacted structured content cannot enter the sink
/// contract through this type.
#[derive(Debug, Clone, Serialize)]
pub struct RedactedValue(serde_json::Value);

impl RedactedValue {
    /// Construct a redacted structured value. Redaction is applied here, at the
    /// producer boundary, using `mode`; the sink adapter never performs
    /// redaction.
    pub fn redacted(value: serde_json::Value, mode: RedactionMode) -> Self {
        Self(crate::redact(&value, mode))
    }

    /// Read access to the redacted value (already safe).
    pub fn as_value(&self) -> &serde_json::Value {
        &self.0
    }
}

/// A redacted diagnostic linked into evidence. Carries only severity and the
/// stable diagnostic code; no raw diagnostic message crosses.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RedactedDiagnostic {
    /// Diagnostic severity.
    pub severity: crate::diagnostic::Severity,
    /// Stable diagnostic code.
    pub code: &'static str,
}

// ===========================================================================
// Evidence record and sink lifecycle
// ===========================================================================

/// Typed payload emitted through an [`EvidenceSink`]. Every channel is typed:
/// structural content is redacted at construction ([`RedactedValue`]); content
/// references are digests ([`ContentDigest`]) or payload-free classified
/// references ([`ArtifactReference`]); diagnostics are redacted
/// ([`RedactedDiagnostic`]). There is no channel for raw user content.
#[derive(Debug, Clone, Serialize)]
pub enum EvidencePayload {
    /// Exact provider route and prepared-auth provenance.
    Provider(ProviderEvidenceFacts),
    /// Exact tool registration, invocation, authorization, and outcome facts.
    Tool(ToolEvidenceFacts),
    /// Typed compaction start or terminal facts.
    Compaction(CompactionEvidenceFacts),
    /// Redacted structured value.
    Structured(RedactedValue),
    /// A bare content digest.
    Digest(ContentDigest),
    /// A redacted diagnostic.
    Diagnostic(RedactedDiagnostic),
    /// A classified, payload-free artifact reference.
    Artifact(ArtifactReference),
}

/// One emitted evidence record with full call-graph correlation.
#[derive(Debug, Clone, Serialize)]
pub struct EvidenceRecord {
    /// Owning run.
    pub run: RunId,
    /// Owning turn, when applicable.
    pub turn: Option<TurnId>,
    /// Owning call.
    pub call: CallId,
    /// Parent call, for retry/nested-call correlation.
    pub parent: Option<ParentCallId>,
    /// Monotonic run-local sequence.
    pub sequence: Sequence,
    /// Call kind.
    pub kind: CallKind,
    /// Typed, redacted payload.
    pub payload: EvidencePayload,
}

impl EvidenceRecord {
    /// Validate that the call kind agrees with the closed typed payload
    /// channel. Generic redacted observations use the diagnostic kind; retry
    /// attempts carry their structured attempt facts.
    pub fn validate_kind_payload(&self) -> Result<(), EvidenceError> {
        let matches = matches!(
            (&self.kind, &self.payload),
            (CallKind::Provider, EvidencePayload::Provider(_))
                | (CallKind::Tool, EvidencePayload::Tool(_))
                | (CallKind::Compaction, EvidencePayload::Compaction(_))
                | (CallKind::Retry, EvidencePayload::Structured(_))
                | (
                    CallKind::Diagnostic,
                    EvidencePayload::Structured(_)
                        | EvidencePayload::Digest(_)
                        | EvidencePayload::Diagnostic(_)
                        | EvidencePayload::Artifact(_),
                )
        );
        if matches {
            Ok(())
        } else {
            Err(finalization_error(
                "evidence record kind does not match its typed payload",
            ))
        }
    }
}

/// Storage-neutral evidence sink lifecycle.
///
/// The four phases are setup (before the run), ordered emission (during the
/// run), artifact finalization, and run finalization (after the run). A failure
/// in any phase is surfaced as a typed [`EvidenceError`]; the caller advances
/// [`EvidenceHealth`] and, under required-complete-evidence policy, withholds
/// the finalized manifest.
///
/// ```compile_fail
/// use opi_agent::evidence::{EvidenceSink, ManifestCandidate};
/// fn bypass(sink: &dyn EvidenceSink, candidate: &ManifestCandidate) {
///     let _ = sink.finalize_run(candidate);
/// }
/// ```
pub trait EvidenceSink: Send + Sync {
    /// Prepare the sink before the run, given the runtime-input binding.
    /// Failures are [`EvidenceError::Setup`].
    fn setup(&self, binding: &RuntimeInputBinding) -> Result<(), EvidenceError>;

    /// Emit one record during the run. Records arrive in monotonic
    /// [`Sequence`] order. Failures are [`EvidenceError::Emission`].
    fn emit(&self, record: &EvidenceRecord) -> Result<(), EvidenceError>;

    /// Finalize a single artifact reference. Failures are
    /// [`EvidenceError::Finalization`].
    fn finalize_artifact(&self, artifact: &ArtifactReference) -> Result<(), EvidenceError>;

    /// Finalize the immutable run manifest. The manifest is borrowed and cannot
    /// be mutated by the sink. Failures are [`EvidenceError::Finalization`].
    fn finalize_run(&self, manifest: &FinalizedManifest) -> Result<(), EvidenceError>;

    /// Abandon an unfinalizable run and clean up any provisional sink state.
    ///
    /// Agent Core invokes this after an emission or finalization failure. A
    /// failure means cleanup could not be confirmed and is reported as a
    /// [`TerminalOutcome::CleanupUnknown`] without replacing the lifecycle
    /// error that caused abandonment.
    fn abandon_run(&self, outcome: &TerminalOutcome) -> Result<(), EvidenceError>;
}

// ===========================================================================
// Adapters: no-op (default, capture disabled) and in-memory (recording oracle)
// ===========================================================================

/// No-op evidence sink. It is the default and enables no content capture: it
/// records nothing and succeeds at every lifecycle phase. With the no-op
/// adapter, execution behavior is unchanged and no capture is implied.
#[derive(Debug, Default, Clone, Copy)]
pub struct NoopEvidenceSink;

impl NoopEvidenceSink {
    /// Create a no-op evidence sink.
    pub fn new() -> Self {
        Self
    }
}

impl EvidenceSink for NoopEvidenceSink {
    fn setup(&self, _binding: &RuntimeInputBinding) -> Result<(), EvidenceError> {
        Ok(())
    }
    fn emit(&self, _record: &EvidenceRecord) -> Result<(), EvidenceError> {
        Ok(())
    }
    fn finalize_artifact(&self, _artifact: &ArtifactReference) -> Result<(), EvidenceError> {
        Ok(())
    }
    fn finalize_run(&self, _manifest: &FinalizedManifest) -> Result<(), EvidenceError> {
        Ok(())
    }
    fn abandon_run(&self, _outcome: &TerminalOutcome) -> Result<(), EvidenceError> {
        Ok(())
    }
}

/// Injectable failure trigger for [`InMemoryEvidenceSink`] lifecycle phases.
/// `None` means the phase succeeds; `Some` injects a typed failure so the
/// conformance tests can exercise each failure class.
#[derive(Debug, Default, Clone)]
struct FailureInjection {
    setup: Option<EvidenceError>,
    emission: Option<EvidenceError>,
    finalization: Option<EvidenceError>,
}

/// In-memory evidence sink: the recording oracle and conformance fixture. It
/// records every emitted record, finalized artifact, and finalized manifest in
/// order, and tracks whether any lifecycle phase failed. Once a phase fails the
/// run is incomplete and [`InMemoryEvidenceSink::completed_manifest`] returns
/// `None` — a failure cannot be hidden by emitting a finalized manifest through
/// another path.
#[derive(Debug, Default)]
pub struct InMemoryEvidenceSink {
    binding: std::sync::Mutex<Option<RuntimeInputBinding>>,
    records: std::sync::Mutex<Vec<EvidenceRecord>>,
    artifacts: std::sync::Mutex<Vec<ArtifactReference>>,
    manifest: std::sync::Mutex<Option<FinalizedManifest>>,
    failure: std::sync::Mutex<Option<EvidenceError>>,
    finalized: std::sync::Mutex<bool>,
    inject: std::sync::Mutex<FailureInjection>,
}

impl InMemoryEvidenceSink {
    /// Create an empty in-memory evidence sink.
    pub fn new() -> Self {
        Self::default()
    }

    fn lock<T>(lock: &std::sync::Mutex<T>) -> std::sync::MutexGuard<'_, T> {
        lock.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    /// Inject a failure for a lifecycle phase, replacing any prior injection
    /// for that phase.
    pub fn inject_failure(&self, error: EvidenceError) {
        let mut inject = Self::lock(&self.inject);
        match error.failure_code() {
            EvidenceFailureCode::Setup => inject.setup = Some(error),
            EvidenceFailureCode::Emission => inject.emission = Some(error),
            EvidenceFailureCode::Finalization => inject.finalization = Some(error),
        }
    }

    /// Snapshot of emitted records in emission order.
    pub fn records(&self) -> Vec<EvidenceRecord> {
        Self::lock(&self.records).clone()
    }

    /// Snapshot of finalized artifacts in finalization order.
    pub fn artifacts(&self) -> Vec<ArtifactReference> {
        Self::lock(&self.artifacts).clone()
    }

    /// Whether any lifecycle phase failed.
    pub fn has_failure(&self) -> bool {
        Self::lock(&self.failure).is_some()
    }

    /// The completed manifest, but only if no lifecycle phase failed. Returns
    /// `None` while incomplete or after a failure (the manifest is withheld).
    pub fn completed_manifest(&self) -> Option<FinalizedManifest> {
        if Self::lock(&self.failure).is_some() {
            return None;
        }
        Self::lock(&self.manifest).clone()
    }
}

impl EvidenceSink for InMemoryEvidenceSink {
    fn setup(&self, binding: &RuntimeInputBinding) -> Result<(), EvidenceError> {
        *Self::lock(&self.binding) = None;
        Self::lock(&self.records).clear();
        Self::lock(&self.artifacts).clear();
        *Self::lock(&self.manifest) = None;
        *Self::lock(&self.failure) = None;
        *Self::lock(&self.finalized) = false;
        if let Some(err) = Self::lock(&self.inject).setup.clone() {
            *Self::lock(&self.failure) = Some(err.clone());
            return Err(err);
        }
        *Self::lock(&self.binding) = Some(binding.clone());
        Ok(())
    }

    fn emit(&self, record: &EvidenceRecord) -> Result<(), EvidenceError> {
        if *Self::lock(&self.finalized) {
            return Err(EvidenceError::Emission {
                detail: "evidence run is already finalized".to_owned(),
            });
        }
        if record.validate_kind_payload().is_err() {
            let error = EvidenceError::Emission {
                detail: "evidence record kind does not match its typed payload".to_owned(),
            };
            *Self::lock(&self.failure) = Some(error.clone());
            return Err(error);
        }
        if let Some(err) = Self::lock(&self.inject).emission.clone() {
            *Self::lock(&self.failure) = Some(err.clone());
            return Err(err);
        }
        Self::lock(&self.records).push(record.clone());
        Ok(())
    }

    fn finalize_artifact(&self, artifact: &ArtifactReference) -> Result<(), EvidenceError> {
        if *Self::lock(&self.finalized) {
            return Err(EvidenceError::Finalization {
                detail: "evidence run is already finalized".to_owned(),
            });
        }
        if let Some(err) = Self::lock(&self.inject).finalization.clone() {
            *Self::lock(&self.failure) = Some(err.clone());
            return Err(err);
        }
        Self::lock(&self.artifacts).push(artifact.clone());
        Ok(())
    }

    fn finalize_run(&self, manifest: &FinalizedManifest) -> Result<(), EvidenceError> {
        if *Self::lock(&self.finalized) {
            return Err(EvidenceError::Finalization {
                detail: "evidence run is already finalized".to_owned(),
            });
        }
        if let Some(err) = Self::lock(&self.inject).finalization.clone() {
            *Self::lock(&self.failure) = Some(err.clone());
            return Err(err);
        }
        if Self::lock(&self.failure).is_some() {
            return Err(finalization_error(
                "evidence lifecycle is already incomplete",
            ));
        }
        let Some(binding) = Self::lock(&self.binding).clone() else {
            let error = finalization_error("evidence setup was not observed");
            *Self::lock(&self.failure) = Some(error.clone());
            return Err(error);
        };
        let records = Self::lock(&self.records).clone();
        let artifacts = Self::lock(&self.artifacts).clone();
        if let Err(error) = manifest
            .validate_observation(EvidenceRunObservation::new(&binding, &records, &artifacts))
        {
            *Self::lock(&self.failure) = Some(error.clone());
            return Err(error);
        }
        *Self::lock(&self.manifest) = Some(manifest.clone());
        *Self::lock(&self.finalized) = true;
        Ok(())
    }

    fn abandon_run(&self, _outcome: &TerminalOutcome) -> Result<(), EvidenceError> {
        if *Self::lock(&self.finalized) {
            return Err(EvidenceError::Finalization {
                detail: "evidence run is already finalized".to_owned(),
            });
        }
        *Self::lock(&self.manifest) = None;
        *Self::lock(&self.finalized) = true;
        Ok(())
    }
}

// ===========================================================================
// Recording introspection
// ===========================================================================

/// Introspection over a recording [`EvidenceSink`]: the ordered emitted
/// records, whether any lifecycle phase failed, and the finalized manifest (if
/// any). The Reference Product harness holds a recorder handle to assemble the
/// [`FinalizedManifest`] from the recorded dynamic facts (correlation, route)
/// plus its own static product facts, then calls [`EvidenceSink::finalize_run`].
///
/// This is a read-only query seam for recording adapters (in-memory oracle and
/// the product file adapter); it is not part of the capture lifecycle and the
/// no-op adapter does not implement it. It lives in Agent Core because the
/// recording contract and the in-memory conformance oracle already do.
pub trait EvidenceRecorder: EvidenceSink {
    /// Snapshot of emitted records in emission order.
    fn records(&self) -> Vec<EvidenceRecord>;
    /// Whether any lifecycle phase failed (evidence is incomplete).
    fn has_failure(&self) -> bool;
    /// The finalized manifest, but only if no lifecycle phase failed. `None`
    /// while incomplete or before [`EvidenceSink::finalize_run`] succeeds.
    fn completed_manifest(&self) -> Option<FinalizedManifest>;
}

impl EvidenceRecorder for InMemoryEvidenceSink {
    fn records(&self) -> Vec<EvidenceRecord> {
        Self::lock(&self.records).clone()
    }
    fn has_failure(&self) -> bool {
        Self::lock(&self.failure).is_some()
    }
    fn completed_manifest(&self) -> Option<FinalizedManifest> {
        if Self::lock(&self.failure).is_some() {
            return None;
        }
        Self::lock(&self.manifest).clone()
    }
}

impl ManifestCandidate {
    /// Validate completeness and exact observed lifecycle correlation, then
    /// consume this candidate into the only manifest type accepted by
    /// [`EvidenceSink::finalize_run`]. Invalid, incomplete, or uncorrelated
    /// facts never reach a sink.
    pub fn validate(
        self,
        observation: EvidenceRunObservation<'_>,
    ) -> Result<FinalizedManifest, EvidenceError> {
        if self.completeness == EvidenceCompleteness::Incomplete {
            return Err(EvidenceError::Finalization {
                detail: "manifest evidence is incomplete".to_owned(),
            });
        }
        let RuntimeInputBinding::DirectRuntimeInput { .. } = &self.binding else {
            return Err(EvidenceError::Finalization {
                detail: "manifest binding is not DirectRuntimeInput".to_owned(),
            });
        };
        if self
            .artifacts
            .iter()
            .any(|artifact| artifact.finalization != FinalizationState::Finalized)
        {
            return Err(EvidenceError::Finalization {
                detail: "manifest contains an artifact that is not finalized".to_owned(),
            });
        }
        let manifest = FinalizedManifest(self);
        manifest.validate_observation(observation)?;
        Ok(manifest)
    }
}
