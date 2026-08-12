//! Product-neutral evidence contract: opaque call-graph identities, versioned
//! health, the storage-neutral sink lifecycle, and the resolved-execution
//! manifest value types (Phase 17 task 17.3).
//!
//! This is an **additive substrate**. It defines the Agent Core evidence
//! vocabulary that authorization (17.4), the Agent runtime (17.6), and the
//! Reference Product file-adapter cutover (17.7) consume. It does **not**
//! activate evidence in [`crate::agent_loop`], add file storage/exporters/Eval,
//! fabricate an `ActiveSnapshot`, or remove the existing [`crate::trace`]
//! `TraceSink` contract (the expand-contract migration lives in 17.6/17.7).
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

use std::sync::atomic::{AtomicU64, Ordering};

use serde::Serialize;

use crate::diagnostic::RedactionMode;

// ===========================================================================
// Opaque call-graph identities (P17-EVD-001)
// ===========================================================================

/// Monotonic run-local sequence number assigned at emission time.
///
/// Ordering is structural (`Ord`); the inner value is opaque and is not reused
/// outside evidence correlation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(transparent)]
pub struct Sequence(u64);

/// Opaque stable run identifier. Non-reused: distinct from the trace
/// envelope's loose run strings. Uniqueness is minted by [`IdentityAllocator`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(transparent)]
pub struct RunId(u64);

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

impl RunId {
    /// Opaque inner value, available only for serialization-adjacent internal
    /// construction. Callers must not depend on the numeric representation.
    pub(crate) fn from_inner(inner: u64) -> Self {
        Self(inner)
    }
}

/// Mints opaque identities and a monotonic run-local [`Sequence`] for one run.
///
/// One allocator belongs to one run. The loop (17.6) holds it; identities are
/// minted immediately before the corresponding lifecycle evidence is emitted so
/// correlation precedes any external effect. `RunId` uniqueness across runs is
/// sourced from a process-wide monotonic counter; run-internal `TurnId`,
/// `CallId`, and `Sequence` are minted monotonically by this allocator.
pub struct IdentityAllocator {
    run: RunId,
    next_turn: u64,
    next_call: u64,
    next_sequence: u64,
}

impl IdentityAllocator {
    /// Begin a new run with a fresh opaque [`RunId`].
    pub fn new() -> Self {
        let next = RUN_ID_COUNTER.fetch_add(1, Ordering::SeqCst) + 1;
        Self {
            run: RunId::from_inner(next),
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

/// Process-wide source of `RunId` uniqueness. Starts at zero; the first run
/// receives `RunId(1)` so a minted id is never zero.
static RUN_ID_COUNTER: AtomicU64 = AtomicU64::new(0);

// ===========================================================================
// Versioned health (P17-EVD-001 evidence owner slice)
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

/// Lifecycle phase whose failure makes evidence incomplete (P17-FAL-001
/// evidence slice). Closed: these are the three distinguishable failure
/// origins carried by [`EvidenceHealth::Incomplete`].
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

/// Closed, versioned, run-local evidence health. Owned by Agent Core; only the
/// loop advances it, immediately when setup, emission, or finalization fails.
/// Sinks do not expose a mutable health handle; authorizers receive a *copy* in
/// each request (17.4), so authorization never shares mutable health with the
/// sink.
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
// Typed lifecycle failure outcomes (P17-FAL-001 evidence slice)
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
// Runtime input binding (P17-EVD-003)
// ===========================================================================

/// Opaque content digest over resolved material. Producers compute the digest;
/// the sink stores only the digest, never the payload.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
#[serde(transparent)]
pub struct ContentDigest(String);

impl ContentDigest {
    /// Construct a digest from its hex rendering. The producer is responsible
    /// for computing the digest; this only carries it.
    pub fn from_hex(hex: impl Into<String>) -> Self {
        Self(hex.into())
    }
}

/// Where direct runtime assembly originated. Closed over the current Reference
/// Product assembly modes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AssemblySource {
    /// Interactive / non-interactive CLI assembly.
    Cli,
    /// SDK embedder assembly.
    Sdk,
    /// JSON/RPC server assembly.
    Rpc,
}

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
/// Current direct CLI/SDK/RPC assembly uses
/// [`RuntimeInputBinding::DirectRuntimeInput`], whose digest covers the
/// resolved material runtime inputs. [`RuntimeInputBinding::ActiveSnapshot`] is
/// accepted only when a future trusted Promotion Controller supplies its
/// reference.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum RuntimeInputBinding {
    /// Direct assembly: the material runtime inputs are resolved and digested
    /// at assembly time.
    DirectRuntimeInput {
        /// Digest over the resolved material runtime inputs.
        digest: ContentDigest,
        /// Where the direct assembly originated.
        assembly_source: AssemblySource,
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
    pub fn direct(digest: ContentDigest, assembly_source: AssemblySource) -> Self {
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
// Measurements (P17-EVD-004)
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
/// unknown measurement is never converted to zero (P17-EVD-004).
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
// Classified artifact references (P17-EVD-005)
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
/// finalization state. It **never** embeds the artifact payload (P17-EVD-005).
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
// Resolved-execution manifest values (P17-EVD-003)
// ===========================================================================

/// Opaque stable identifier for a session branch.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
#[serde(transparent)]
pub struct SessionBranchRef(String);

impl SessionBranchRef {
    /// Construct a session-branch reference.
    pub fn new(reference: impl Into<String>) -> Self {
        Self(reference.into())
    }
}

/// Canonical provider:model plus wire selection at one stage of resolution.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RouteSelection {
    /// Provider identifier.
    pub provider_id: String,
    /// Model identifier within the provider.
    pub model_id: String,
    /// Wire protocol used for the call.
    pub wire: opi_ai::WireApi,
}

/// Requested, resolved, and actual route facts. The three stages are distinct
/// fields and cannot be conflated (P17-EVD-004).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RouteFacts {
    /// What the caller requested.
    pub requested: RouteSelection,
    /// What route selection resolved.
    pub resolved: RouteSelection,
    /// What route the dispatch actually used.
    pub actual: RouteSelection,
}

/// Non-secret authentication, fallback, and source provenance. Secret-bearing
/// detail never crosses; only the classification and redacted summary do.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ProvenanceFacts {
    /// Non-secret authentication source classification.
    pub auth_source: AuthProvenanceSource,
    /// Whether an authentication fallback was allowed, when known.
    pub fallback_allowed: Option<bool>,
}

/// Closed non-secret authentication provenance source.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum AuthProvenanceSource {
    /// A static configured key.
    Static,
    /// An environment-provided credential.
    Environment,
    /// A credential store (e.g. OS keyring) credential.
    CredentialStore,
    /// An OAuth token.
    Oauth,
}

/// Effective user-policy facts snapshotted for the run. The digest addresses
/// the immutable policy; capability/scope/grant are referenced, not embedded.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct UserPolicyFacts {
    /// Digest over the immutable effective user policy.
    pub policy_digest: ContentDigest,
    /// Granted capability permission class, when known.
    pub capability: Option<CapabilityClass>,
}

/// Closed capability permission class for a tool.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum CapabilityClass {
    /// Read-only workspace access.
    WorkspaceRead,
    /// Mutating workspace access.
    WorkspaceWrite,
    /// Command execution.
    CommandExecute,
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

/// Budget, trigger, time, and platform/environment identity plus measurement
/// origin. Each fact is a [`Measurement`] so unknown stays distinct from zero.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct EnvironmentFacts {
    /// Token/turn budget fact, when applicable.
    pub budget: Measurement,
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
/// from estimated, quota, and billed values (P17-EVD-004).
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

/// Immutable finalized resolved-execution manifest. Constructed once from the
/// run's final facts and passed by reference to [`EvidenceSink::finalize_run`].
/// It exposes no mutating methods after construction; immutability is the
/// contract (P17-EVD-003).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FinalizedManifest {
    /// Run/turn/call/parent/sequence correlation.
    pub correlation: ManifestCorrelation,
    /// Terminal outcome.
    pub outcome: TerminalOutcome,
    /// Session branch reference, when applicable.
    pub session_branch: Option<SessionBranchRef>,
    /// Exact runtime-input binding variant (direct runs are never
    /// [`RuntimeInputBinding::ActiveSnapshot`]).
    pub binding: RuntimeInputBinding,
    /// Resolved configuration identity.
    pub config: ConfigIdentity,
    /// Requested/resolved/actual route facts.
    pub route: RouteFacts,
    /// Non-secret provenance.
    pub provenance: ProvenanceFacts,
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

// ===========================================================================
// Redacted structured value (producer-boundary enforcement)
// ===========================================================================

/// Structured content channel for evidence. The only constructor applies
/// [`crate::redact`], so unredacted structured content cannot enter the sink
/// contract through this type (P17-EVD-005).
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

/// Storage-neutral evidence sink lifecycle (P17-EVD-008, P17-EVD-011).
///
/// The four phases are setup (before the run), ordered emission (during the
/// run), artifact finalization, and run finalization (after the run). A failure
/// in any phase is surfaced as a typed [`EvidenceError`]; the caller advances
/// [`EvidenceHealth`] and, under required-complete-evidence policy (17.7),
/// withholds the finalized manifest.
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
}

// ===========================================================================
// Adapters: no-op (default, capture disabled) and in-memory (recording oracle)
// ===========================================================================

/// No-op evidence sink. It is the default and enables no content capture: it
/// records nothing and succeeds at every lifecycle phase (P17-EVD-006,
/// P17-EVD-010). With the no-op adapter, execution behavior is unchanged and no
/// capture is implied.
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
    records: std::sync::Mutex<Vec<EvidenceRecord>>,
    artifacts: std::sync::Mutex<Vec<ArtifactReference>>,
    manifest: std::sync::Mutex<Option<FinalizedManifest>>,
    failure: std::sync::Mutex<Option<EvidenceError>>,
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
    fn setup(&self, _binding: &RuntimeInputBinding) -> Result<(), EvidenceError> {
        if let Some(err) = Self::lock(&self.inject).setup.clone() {
            *Self::lock(&self.failure) = Some(err.clone());
            return Err(err);
        }
        Ok(())
    }

    fn emit(&self, record: &EvidenceRecord) -> Result<(), EvidenceError> {
        if let Some(err) = Self::lock(&self.inject).emission.clone() {
            *Self::lock(&self.failure) = Some(err.clone());
            return Err(err);
        }
        Self::lock(&self.records).push(record.clone());
        Ok(())
    }

    fn finalize_artifact(&self, artifact: &ArtifactReference) -> Result<(), EvidenceError> {
        if let Some(err) = Self::lock(&self.inject).finalization.clone() {
            *Self::lock(&self.failure) = Some(err.clone());
            return Err(err);
        }
        Self::lock(&self.artifacts).push(artifact.clone());
        Ok(())
    }

    fn finalize_run(&self, manifest: &FinalizedManifest) -> Result<(), EvidenceError> {
        if let Some(err) = Self::lock(&self.inject).finalization.clone() {
            *Self::lock(&self.failure) = Some(err.clone());
            return Err(err);
        }
        *Self::lock(&self.manifest) = Some(manifest.clone());
        Ok(())
    }
}
