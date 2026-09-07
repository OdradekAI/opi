//! Canonical resolved-experiment identity.
//!
//! [`ResolvedExperiment::resolve`] freezes the experiment contract - schema
//! identity, experiment id and manifest digest, benchmark descriptor,
//! N-harness subject set with directed comparison edges, shared model
//! controls, environment, and declared trials - into one immutable,
//! digest-addressed value before any Agent process starts. Resolution is
//! fail-closed: a document with a missing identity field, an implicit model
//! control, an unknown edge endpoint, or a duplicate trial id is rejected with
//! a typed error instead of being silently normalized.

use std::collections::BTreeMap;
use std::fmt;

use serde::Serialize;
use sha2::{Digest, Sha256};
use thiserror::Error;

/// Schema identity accepted by [`ResolvedExperiment::resolve`].
pub const EXPERIMENT_SCHEMA: &str = "opi-eval-experiment/2";

/// Fail-closed resolution failures for an experiment document.
#[derive(Debug, Error)]
pub enum ResolveError {
    /// The document is not valid TOML.
    #[error("experiment document is not valid TOML: {0}")]
    Parse(#[from] toml::de::Error),
    /// The resolved contract cannot be canonicalized (for example a
    /// non-finite TOML float control).
    #[error("experiment contract cannot be canonicalized: {0}")]
    Canonicalize(String),
    /// The document root is not a table.
    #[error("experiment document root must be a table")]
    NotATable,
    /// The schema identity is missing or unsupported.
    #[error("unsupported experiment schema: {0}")]
    UnsupportedSchema(String),
    /// A required scalar field is missing or empty.
    #[error("missing or empty field: {0}")]
    MissingField(String),
    /// The subject set is empty.
    #[error("experiment must declare at least one subject")]
    MissingSubjects,
    /// The comparison edge set is empty.
    #[error("experiment must declare at least one comparison edge")]
    MissingEdges,
    /// The declared trial set is empty.
    #[error("experiment must declare at least one trial")]
    MissingTrials,
    /// Two subjects share an id.
    #[error("duplicate subject id: {0}")]
    DuplicateSubject(String),
    /// Two edges share an id, or the same directed baseline->candidate pair.
    #[error("duplicate comparison edge: {0}")]
    DuplicateEdge(String),
    /// An edge names a subject that does not exist.
    #[error("edge {edge} references unknown {role} subject {subject}")]
    UnknownEdgeEndpoint {
        edge: String,
        role: &'static str,
        subject: String,
    },
    /// An edge uses the same subject as baseline and candidate.
    #[error("edge {0} uses the same subject as baseline and candidate")]
    SelfEdge(String),
    /// A shared model control is absent; controls must be explicit.
    #[error("missing shared model control: {0}")]
    MissingModelControl(String),
    /// A control marker is not `omitted` or `unknown`.
    #[error("invalid control marker for {control}: {marker}")]
    InvalidControlMarker { control: String, marker: String },
    /// A control has a value of the wrong TOML type.
    #[error("invalid control value for {control}: expected {expected}")]
    InvalidControlValue {
        control: String,
        expected: &'static str,
    },
    /// A trial id is duplicated.
    #[error("duplicate trial id: {0}")]
    DuplicateTrial(String),
    /// A trial names a subject that does not exist.
    #[error("trial {trial} references unknown subject {subject}")]
    UnknownTrialSubject { trial: String, subject: String },
    /// An array entry is not a table.
    #[error("{0} entries must be tables")]
    EntryNotATable(&'static str),
    /// The top-level connection selection is missing.
    #[error("experiment must select one [[connections]] id")]
    MissingConnectionSelection,
    /// The selected connection id does not exist.
    #[error("unknown connection selection: {0}")]
    UnknownConnection(String),
    /// Two connections share an id.
    #[error("duplicate connection id: {0}")]
    DuplicateConnection(String),
    /// A connection credential reference is absent, doubled, or empty.
    #[error(
        "connection {connection} needs exactly one non-empty credential_env or credential_file reference"
    )]
    InvalidCredentialReference {
        /// Owning connection id.
        connection: String,
    },
    /// A connection api is not one of the three admitted protocols.
    #[error("unsupported connection api: {0}")]
    InvalidConnectionApi(String),
    /// A connection base URL is not a parseable http(s) URL.
    #[error("connection base_url is not a parseable http(s) URL: {0}")]
    InvalidBaseUrl(String),
    /// A remote base URL requires the explicit live execution mode.
    #[error(
        "remote base URL {base_url} rejected: execution mode {mode} admits only loopback routes"
    )]
    RemoteRouteNotLive {
        /// The rejected base URL.
        base_url: String,
        /// The declared (or defaulted) execution mode.
        mode: String,
    },
    /// The execution mode is not one of the admitted values.
    #[error("invalid execution mode: {0}")]
    InvalidExecutionMode(String),
    /// The capture policy is not one of the admitted values.
    #[error("invalid storage capture_policy: {0}")]
    InvalidCapturePolicy(String),
    /// A requested cap cannot be enforced on the native Agent route.
    #[error("requested execution cap {cap} cannot be enforced on the native route: {reason}")]
    UnenforceableCap {
        /// The requested cap key.
        cap: String,
        /// Why the route cannot enforce it.
        reason: String,
    },
    /// A required control is unsupported, omitted, or unknown by name.
    #[error("required control {control} is not an enforced value: {reason}")]
    RequiredControlUnsupported {
        /// The required control name.
        control: String,
        /// Why the requirement is not met.
        reason: String,
    },
    /// Both `[selection]` and explicit `[[trials]]` were declared.
    #[error("selection and explicit trials are mutually exclusive")]
    SelectionAndTrials,
    /// The selection scope is not `full` or `subset`.
    #[error("invalid selection scope: {0} (expected full or subset)")]
    InvalidSelectionScope(String),
    /// A subset selection pinned no task ids.
    #[error("scope=subset requires a non-empty task_ids list")]
    EmptySubsetSelection,
    /// A full selection also pinned task ids.
    #[error("scope=full cannot pin task_ids; full selects the whole corpus")]
    FullScopePinsTasks,
    /// A task id appears twice in the selection.
    #[error("duplicate selection task id: {0}")]
    DuplicateSelectionTask(String),
    /// The repetition count is not positive.
    #[error("selection repetitions must be positive, found {0}")]
    InvalidSelectionRepetitions(u64),
    /// The concurrency bound is not positive.
    #[error("execution concurrency must be positive, found {0}")]
    InvalidConcurrency(u64),
    /// A selection seed names a task outside the selection.
    #[error("selection seed names task outside the selection: {0}")]
    UnknownSelectionSeed(String),
    /// A requested execution bound has no enforcement route here.
    #[error("requested execution bound {bound} cannot be enforced: {reason}")]
    UnenforceableExecutionBound {
        /// The requested bound key.
        bound: String,
        /// Why it cannot be enforced.
        reason: String,
    },
}

/// The corpus selection scope of a `[selection]` block.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum SelectionScope {
    /// Every task in the pinned corpus inventory.
    Full,
    /// The explicitly listed task ids.
    Subset,
}

impl SelectionScope {
    /// The exact document spelling of this scope.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Full => "full",
            Self::Subset => "subset",
        }
    }
}

/// A resolved `[selection]` block: the user's corpus-selection intent.
/// The exact trial expansion is corpus-dependent and happens only at
/// freeze time (plan/run), never inside document resolution.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ResolvedSelection {
    /// Corpus scope.
    pub scope: SelectionScope,
    /// Explicit task ids (empty for `full`).
    pub task_ids: Vec<String>,
    /// Recorded reason for the selection.
    pub reason: String,
    /// Repetitions per task (positive).
    pub repetitions: u32,
    /// Optional per-task deterministic seeds.
    pub seeds: std::collections::BTreeMap<String, u64>,
}

/// An explicitly declared shared model control.
///
/// A control carries either a value or an explicit `omitted`/`unknown`
/// marker. An absent key is a resolution failure: there is no implicit
/// default and no fallback. Serialized form mirrors the document form, so the
/// canonical digest is stable across document formatting.
#[derive(Debug, Clone, PartialEq)]
pub enum ControlValue<T> {
    /// The harness expresses the control with this exact value.
    Value(T),
    /// The control is explicitly not configured for this experiment.
    Omitted,
    /// The control exists upstream but this harness cannot express it.
    Unknown,
}

impl<T> Serialize for ControlValue<T>
where
    T: Serialize,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            ControlValue::Value(value) => value.serialize(serializer),
            ControlValue::Omitted => serializer.serialize_str("omitted"),
            ControlValue::Unknown => serializer.serialize_str("unknown"),
        }
    }
}

impl<T: fmt::Display> fmt::Display for ControlValue<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ControlValue::Value(value) => write!(f, "value({value})"),
            ControlValue::Omitted => write!(f, "omitted"),
            ControlValue::Unknown => write!(f, "unknown"),
        }
    }
}

/// Frozen benchmark descriptor of a resolved experiment.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ResolvedBenchmark {
    /// Benchmark family name, for example `terminal-bench`.
    pub name: String,
    /// Immutable benchmark revision identity.
    pub revision: String,
    /// Dataset reference owned by the benchmark revision.
    pub dataset: String,
    /// Integrity-record digest once the revision is admitted. It may be
    /// absent for fixture-only or otherwise unadmitted revisions.
    pub integrity_digest: Option<String>,
}

/// One identified harness subject of the experiment.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ResolvedSubject {
    /// Subject id, unique inside the experiment and referenced by edges and
    /// trials. The contract is Agent-neutral: any id is accepted and no
    /// harness name is hard-coded.
    pub id: String,
    /// Product identity of the subject, attributable to the evaluated Agent.
    pub product: String,
    /// Product version identity.
    pub version: String,
}

/// One directed baseline-subject -> candidate-subject comparison edge.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ComparisonEdge {
    /// Edge id, unique inside the experiment.
    pub id: String,
    /// Baseline subject id.
    pub baseline: String,
    /// Candidate subject id.
    pub candidate: String,
}

/// Frozen shared model controls.
///
/// Every control that can change outcome interpretation is explicit: either a
/// value or an `omitted`/`unknown` marker. Fallbacks are prohibited. Provider,
/// protocol, base URL, and model identity live on the selected
/// [`Connection`], never here.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ModelControls {
    /// Sampling temperature.
    pub temperature: ControlValue<f64>,
    /// Nucleus sampling control.
    pub top_p: ControlValue<f64>,
    /// Output token limit.
    pub max_output_tokens: ControlValue<u64>,
    /// Context window limit.
    pub context_window: ControlValue<u64>,
    /// Reasoning effort control.
    pub reasoning: ControlValue<String>,
    /// Control names that must be enforced values: a listed control marked
    /// `omitted`/`unknown` rejects the experiment before any model effect.
    pub required: Vec<String>,
}

/// The three admitted provider protocols. Protocol names do not rename the
/// provider: identity stays on the connection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum ConnectionApi {
    /// Anthropic Messages wire.
    #[serde(rename = "anthropic-messages")]
    AnthropicMessages,
    /// OpenAI Chat Completions wire.
    #[serde(rename = "openai-completions")]
    OpenAiCompletions,
    /// OpenAI Responses wire.
    #[serde(rename = "openai-responses")]
    OpenAiResponses,
}

impl ConnectionApi {
    /// The exact document spelling of this protocol.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::AnthropicMessages => "anthropic-messages",
            Self::OpenAiCompletions => "openai-completions",
            Self::OpenAiResponses => "openai-responses",
        }
    }
}

/// Where the selected connection's credential comes from. Exactly one
/// reference is declared per connection; the value is resolved only inside
/// the supervised Agent launch and never enters exportable artifacts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum CredentialReference {
    /// An environment variable resolved at Agent launch.
    Env {
        /// Variable name.
        variable: String,
    },
    /// A private local file read at Agent launch.
    File {
        /// Absolute file path.
        path: String,
    },
}

/// One declared named connection.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Connection {
    /// Unique connection id; the top-level `connection` selects it.
    pub id: String,
    /// Provider identity. Protocol names do not rename the provider.
    pub provider: String,
    /// Admitted request protocol.
    pub api: ConnectionApi,
    /// Provider base URL. Only loopback routes are admitted without the
    /// explicit live execution mode.
    pub base_url: String,
    /// Exact model identity.
    pub model: String,
    /// Exactly one credential reference.
    pub credential: CredentialReference,
}

/// Declared execution mode and the explicitly enforceable bounds.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ResolvedExecution {
    /// Execution mode. `live` is never defaulted.
    pub mode: ExecutionMode,
    /// Enforceable wall-clock supervision limit per Agent process.
    pub time_limit_seconds: Option<u64>,
    /// Enforceable captured-output limit per Agent process.
    pub max_output_bytes: Option<u64>,
    /// Bounded parallel trials (positive; 1 keeps the serial order).
    pub concurrency: u32,
    /// Declared maximum trial count; a larger expansion rejects at freeze.
    pub max_trials: Option<u64>,
    /// Per-Agent-process supervision limit (narrows `time_limit_seconds`).
    pub agent_timeout_seconds: Option<u64>,
    /// Per-verifier-process supervision limit.
    pub verifier_timeout_seconds: Option<u64>,
    /// Output-token bound; enforceable only through the pinned model
    /// control of the same value (the projection carries it to the Agent).
    pub max_output_tokens: Option<u64>,
}

/// Execution mode. Credential presence never selects it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ExecutionMode {
    /// Fixture trials against deterministic helpers (the default).
    Hermetic,
    /// Native trials against a loopback fixture provider.
    NativeFixture,
    /// Real provider execution; explicit only, operator evidence runs.
    Live,
}

impl ExecutionMode {
    /// The exact document spelling of this mode.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Hermetic => "hermetic",
            Self::NativeFixture => "native-fixture",
            Self::Live => "live",
        }
    }
}

/// Declared storage identity and evidence-capture policy.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ResolvedStorage {
    /// Local material cache directory name.
    pub cache: String,
    /// Local results directory name.
    pub results: String,
    /// Evidence-capture policy for trial artifacts.
    pub capture_policy: CapturePolicy,
}

/// Evidence-capture policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum CapturePolicy {
    /// Capture every capturable evidence stream (the reviewed default).
    Full,
    /// Capture only terminal observations and binding-required artifacts.
    TerminalOnly,
}

impl CapturePolicy {
    /// The exact document spelling of this policy.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Full => "full",
            Self::TerminalOnly => "terminal-only",
        }
    }
}

/// Frozen execution environment identity.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ResolvedEnvironment {
    /// Host platform identity.
    pub platform: String,
    /// Host architecture identity.
    pub architecture: String,
    /// Working-directory policy for Agent processes.
    pub cwd_policy: String,
}

/// One declared trial of the frozen experiment.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct DeclaredTrial {
    /// Trial id, unique inside the experiment. Fresh identities for retries
    /// and re-runs are assigned by the runner, not by the document.
    pub id: String,
    /// Owning subject id.
    pub subject: String,
    /// Benchmark task id.
    pub task: String,
    /// Trial group used later for baseline/candidate pairing.
    pub group: String,
}

/// A canonical, immutable, digest-addressed experiment contract.
///
/// The value is constructed only through [`ResolvedExperiment::resolve`],
/// exposes read-only accessors, and canonicalizes its collections (subjects,
/// edges, trials sorted by id) so equal semantics always compare equal and
/// produce the same [`manifest_digest`](Self::manifest_digest).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ResolvedExperiment {
    schema: String,
    experiment_id: String,
    manifest_digest: String,
    benchmark: ResolvedBenchmark,
    subjects: Vec<ResolvedSubject>,
    edges: Vec<ComparisonEdge>,
    connections: Vec<Connection>,
    connection_id: String,
    model_controls: ModelControls,
    execution: ResolvedExecution,
    storage: ResolvedStorage,
    environment: ResolvedEnvironment,
    selection: Option<ResolvedSelection>,
    trials: Vec<DeclaredTrial>,
}

impl ResolvedExperiment {
    /// Resolve an experiment document into a frozen contract.
    ///
    /// Fails closed on any missing identity field, implicit model control,
    /// unknown or self-referencing edge endpoint, duplicate id, or trial
    /// referencing an unknown subject. Deterministic: the same document
    /// semantics always resolve to an equal value with the same digest.
    pub fn resolve(source: &str) -> Result<Self, ResolveError> {
        let root: toml::Value = toml::from_str(source)?;
        let table = root.as_table().ok_or(ResolveError::NotATable)?;

        let schema = match table.get("schema") {
            Some(toml::Value::String(value)) if value == EXPERIMENT_SCHEMA => value.clone(),
            Some(toml::Value::String(value)) => {
                return Err(ResolveError::UnsupportedSchema(value.clone()));
            }
            _ => return Err(ResolveError::UnsupportedSchema("<missing>".to_owned())),
        };
        let experiment_id = require_str(table, "experiment_id")?;

        let benchmark_table = required_table(table, "benchmark")?;
        let benchmark = ResolvedBenchmark {
            name: require_str(benchmark_table, "benchmark.name")?,
            revision: require_str(benchmark_table, "benchmark.revision")?,
            dataset: require_str(benchmark_table, "benchmark.dataset")?,
            integrity_digest: match benchmark_table.get("integrity_digest") {
                Some(toml::Value::String(value)) if !value.is_empty() => Some(value.clone()),
                _ => None,
            },
        };

        let mut subjects: Vec<ResolvedSubject> = Vec::new();
        for entry in array_entries(table, "subjects", "subjects")? {
            let subject = ResolvedSubject {
                id: require_str(entry, "subjects[].id")?,
                product: require_str(entry, "subjects[].product")?,
                version: require_str(entry, "subjects[].version")?,
            };
            if subjects.iter().any(|s| s.id == subject.id) {
                return Err(ResolveError::DuplicateSubject(subject.id));
            }
            subjects.push(subject);
        }
        if subjects.is_empty() {
            return Err(ResolveError::MissingSubjects);
        }
        subjects.sort_by(|a, b| a.id.cmp(&b.id));

        let mut edges: Vec<ComparisonEdge> = Vec::new();
        for entry in array_entries(table, "edges", "edges")? {
            let edge = ComparisonEdge {
                id: require_str(entry, "edges[].id")?,
                baseline: require_str(entry, "edges[].baseline")?,
                candidate: require_str(entry, "edges[].candidate")?,
            };
            if edge.baseline == edge.candidate {
                return Err(ResolveError::SelfEdge(edge.id));
            }
            for role in [("baseline", &edge.baseline), ("candidate", &edge.candidate)] {
                if !subjects.iter().any(|s| s.id == *role.1) {
                    return Err(ResolveError::UnknownEdgeEndpoint {
                        edge: edge.id.clone(),
                        role: role.0,
                        subject: role.1.clone(),
                    });
                }
            }
            if edges.iter().any(|e| e.id == edge.id) {
                return Err(ResolveError::DuplicateEdge(edge.id));
            }
            if edges
                .iter()
                .any(|e| e.baseline == edge.baseline && e.candidate == edge.candidate)
            {
                return Err(ResolveError::DuplicateEdge(format!(
                    "{}->{}",
                    edge.baseline, edge.candidate
                )));
            }
            edges.push(edge);
        }
        // A single-subject experiment needs no artificial comparison
        // edge (P19-D05/§6): single-subject reports stand on their own.
        if edges.is_empty() && subjects.len() > 1 {
            return Err(ResolveError::MissingEdges);
        }
        edges.sort_by(|a, b| a.id.cmp(&b.id));

        // Named connections: the top-level selection is required, ids are
        // unique, every entry carries the exact admitted identity, and only
        // a parseable loopback route is admitted without the explicit live
        // execution mode.
        let selection = match table.get("connection") {
            Some(toml::Value::String(value)) if !value.trim().is_empty() => value.clone(),
            _ => return Err(ResolveError::MissingConnectionSelection),
        };
        let mut connections: Vec<Connection> = Vec::new();
        for entry in array_entries(table, "connections", "connections")? {
            let id = require_str(entry, "connections[].id")?;
            let api = match entry.get("api") {
                Some(toml::Value::String(value)) => match value.as_str() {
                    "anthropic-messages" => ConnectionApi::AnthropicMessages,
                    "openai-completions" => ConnectionApi::OpenAiCompletions,
                    "openai-responses" => ConnectionApi::OpenAiResponses,
                    other => {
                        return Err(ResolveError::InvalidConnectionApi(other.to_owned()));
                    }
                },
                _ => return Err(ResolveError::InvalidConnectionApi("<missing>".to_owned())),
            };
            let base_url = require_str(entry, "connections[].base_url")?;
            let credential_env = match entry.get("credential_env") {
                Some(toml::Value::String(value)) if !value.trim().is_empty() => Some(value.clone()),
                _ => None,
            };
            let credential_file = match entry.get("credential_file") {
                Some(toml::Value::String(value)) if !value.trim().is_empty() => Some(value.clone()),
                _ => None,
            };
            let credential = match (credential_env, credential_file) {
                (Some(variable), None) => CredentialReference::Env { variable },
                (None, Some(path)) => CredentialReference::File { path },
                _ => {
                    return Err(ResolveError::InvalidCredentialReference { connection: id });
                }
            };
            let connection = Connection {
                id: id.clone(),
                provider: require_str(entry, "connections[].provider")?,
                api,
                base_url,
                model: require_str(entry, "connections[].model")?,
                credential,
            };
            if connections.iter().any(|c| c.id == connection.id) {
                return Err(ResolveError::DuplicateConnection(connection.id));
            }
            connections.push(connection);
        }
        if connections.is_empty() {
            return Err(ResolveError::UnknownConnection(selection.clone()));
        }
        connections.sort_by(|a, b| a.id.cmp(&b.id));
        if !connections.iter().any(|c| c.id == selection) {
            return Err(ResolveError::UnknownConnection(selection));
        }

        // Execution mode and enforceable bounds. `[execution]` is optional
        // and defaults to hermetic; `live` is never defaulted. Requested
        // monetary/usage caps are rejected: the native Agent/provider route
        // cannot enforce them, and a bound that cannot be enforced is never
        // silently ignored.
        let execution = match table.get("execution") {
            Some(toml::Value::Table(execution_table)) => {
                let mode = match execution_table.get("mode") {
                    None => ExecutionMode::Hermetic,
                    Some(toml::Value::String(value)) => match value.as_str() {
                        "hermetic" => ExecutionMode::Hermetic,
                        "native-fixture" => ExecutionMode::NativeFixture,
                        "live" => ExecutionMode::Live,
                        other => {
                            return Err(ResolveError::InvalidExecutionMode(other.to_owned()));
                        }
                    },
                    _ => {
                        return Err(ResolveError::InvalidExecutionMode(
                            "<non-string>".to_owned(),
                        ));
                    }
                };
                if let Some(toml::Value::Table(caps)) = execution_table.get("caps")
                    && let Some((cap, _)) = caps.iter().next()
                {
                    return Err(ResolveError::UnenforceableCap {
                        cap: cap.clone(),
                        reason: "native Agent/provider route cannot enforce it".to_owned(),
                    });
                }
                let concurrency = match execution_table.get("concurrency") {
                    None => 1,
                    Some(toml::Value::Integer(value)) => u32::try_from(*value)
                        .ok()
                        .filter(|parsed| *parsed > 0)
                        .ok_or(ResolveError::InvalidConcurrency(*value as u64))?,
                    _ => return Err(ResolveError::InvalidConcurrency(0)),
                };
                let max_trials = optional_u64(execution_table, "max_trials")?;
                let max_total_tokens = optional_u64(execution_table, "max_total_tokens")?;
                if max_total_tokens.is_some() {
                    return Err(ResolveError::UnenforceableExecutionBound {
                        bound: "max_total_tokens".to_owned(),
                        reason: "provider usage accounting is not an enforcement route in this build; declare an enforceable bound or omit it".to_owned(),
                    });
                }
                if execution_table.contains_key("max_cost") {
                    return Err(ResolveError::UnenforceableExecutionBound {
                        bound: "max_cost".to_owned(),
                        reason: "unknown prices cannot support a guaranteed currency cap"
                            .to_owned(),
                    });
                }
                ResolvedExecution {
                    mode,
                    time_limit_seconds: optional_u64(execution_table, "time_limit_seconds")?,
                    max_output_bytes: optional_u64(execution_table, "max_output_bytes")?,
                    concurrency,
                    max_trials,
                    agent_timeout_seconds: optional_u64(execution_table, "agent_timeout_seconds")?,
                    verifier_timeout_seconds: optional_u64(
                        execution_table,
                        "verifier_timeout_seconds",
                    )?,
                    max_output_tokens: optional_u64(execution_table, "max_output_tokens")?,
                }
            }
            _ => ResolvedExecution {
                mode: ExecutionMode::Hermetic,
                time_limit_seconds: None,
                max_output_bytes: None,
                concurrency: 1,
                max_trials: None,
                agent_timeout_seconds: None,
                verifier_timeout_seconds: None,
                max_output_tokens: None,
            },
        };
        for connection in &connections {
            if execution.mode != ExecutionMode::Live && !is_loopback_base_url(&connection.base_url)
            {
                return Err(ResolveError::RemoteRouteNotLive {
                    base_url: connection.base_url.clone(),
                    mode: execution.mode.as_str().to_owned(),
                });
            }
        }

        let controls_table = required_table(table, "model_controls")?;
        let model_controls = ModelControls {
            temperature: float_control(controls_table, "temperature")?,
            top_p: float_control(controls_table, "top_p")?,
            max_output_tokens: u64_control(controls_table, "max_output_tokens")?,
            context_window: u64_control(controls_table, "context_window")?,
            reasoning: string_control(controls_table, "reasoning")?,
            required: match controls_table.get("required") {
                Some(toml::Value::Array(entries)) => entries
                    .iter()
                    .map(|entry| {
                        entry.as_str().map(str::to_owned).ok_or_else(|| {
                            ResolveError::InvalidControlValue {
                                control: "required".to_owned(),
                                expected: "an array of control names",
                            }
                        })
                    })
                    .collect::<Result<Vec<String>, ResolveError>>()?,
                _ => Vec::new(),
            },
        };
        for name in &model_controls.required {
            let supported = match name.as_str() {
                "temperature" => matches!(model_controls.temperature, ControlValue::Value(_)),
                "top_p" => matches!(model_controls.top_p, ControlValue::Value(_)),
                "max_output_tokens" => {
                    matches!(model_controls.max_output_tokens, ControlValue::Value(_))
                }
                "context_window" => {
                    matches!(model_controls.context_window, ControlValue::Value(_))
                }
                "reasoning" => matches!(model_controls.reasoning, ControlValue::Value(_)),
                other => {
                    return Err(ResolveError::RequiredControlUnsupported {
                        control: other.to_owned(),
                        reason: "not a model control name".to_owned(),
                    });
                }
            };
            if !supported {
                return Err(ResolveError::RequiredControlUnsupported {
                    control: name.clone(),
                    reason: "control is omitted or unknown".to_owned(),
                });
            }
        }

        // Storage identity: cache and results locations plus the
        // evidence-capture policy, frozen into the contract.
        let storage_table = required_table(table, "storage")?;
        let storage = ResolvedStorage {
            cache: require_str(storage_table, "storage.cache")?,
            results: require_str(storage_table, "storage.results")?,
            capture_policy: match storage_table.get("capture_policy") {
                Some(toml::Value::String(value)) => match value.as_str() {
                    "full" => CapturePolicy::Full,
                    "terminal-only" => CapturePolicy::TerminalOnly,
                    other => {
                        return Err(ResolveError::InvalidCapturePolicy(other.to_owned()));
                    }
                },
                _ => {
                    return Err(ResolveError::MissingField(
                        "storage.capture_policy".to_owned(),
                    ));
                }
            },
        };

        let environment_table = required_table(table, "environment")?;
        let environment = ResolvedEnvironment {
            platform: require_str(environment_table, "environment.platform")?,
            architecture: require_str(environment_table, "environment.architecture")?,
            cwd_policy: require_str(environment_table, "environment.cwd_policy")?,
        };

        // Task selection (P19-D05): `[selection]` derives the trial set
        // from the corpus inventory at freeze time; it is mutually
        // exclusive with explicit `[[trials]]` (ambiguous intent
        // rejects). Without either, the document has no trial set.
        let task_selection = match table.get("selection") {
            Some(toml::Value::Table(selection_table)) => {
                if table.contains_key("trials") {
                    return Err(ResolveError::SelectionAndTrials);
                }
                let scope = match selection_table.get("scope") {
                    Some(toml::Value::String(value)) => match value.as_str() {
                        "full" => SelectionScope::Full,
                        "subset" => SelectionScope::Subset,
                        other => {
                            return Err(ResolveError::InvalidSelectionScope(other.to_owned()));
                        }
                    },
                    _ => {
                        return Err(ResolveError::MissingField("selection.scope".to_owned()));
                    }
                };
                let task_ids: Vec<String> = match selection_table.get("task_ids") {
                    Some(toml::Value::Array(entries)) => {
                        let mut ids = Vec::with_capacity(entries.len());
                        for entry in entries {
                            let Some(id) = entry.as_str() else {
                                return Err(ResolveError::InvalidSelectionScope(
                                    "<non-string task id>".to_owned(),
                                ));
                            };
                            if id.trim().is_empty() {
                                return Err(ResolveError::EmptySubsetSelection);
                            }
                            if ids.iter().any(|known: &String| known == id) {
                                return Err(ResolveError::DuplicateSelectionTask(id.to_owned()));
                            }
                            ids.push(id.to_owned());
                        }
                        ids
                    }
                    _ => Vec::new(),
                };
                match scope {
                    SelectionScope::Full if !task_ids.is_empty() => {
                        return Err(ResolveError::FullScopePinsTasks);
                    }
                    SelectionScope::Subset if task_ids.is_empty() => {
                        return Err(ResolveError::EmptySubsetSelection);
                    }
                    _ => {}
                }
                let reason = require_str(selection_table, "selection.reason")?;
                let repetitions = match selection_table.get("repetitions") {
                    None => 1,
                    Some(toml::Value::Integer(value)) => u32::try_from(*value)
                        .ok()
                        .filter(|parsed| *parsed > 0)
                        .ok_or(ResolveError::InvalidSelectionRepetitions(*value as u64))?,
                    _ => return Err(ResolveError::InvalidSelectionRepetitions(0)),
                };
                let mut seeds = std::collections::BTreeMap::new();
                if let Some(toml::Value::Table(seed_table)) = selection_table.get("seeds") {
                    for (task, value) in seed_table {
                        let Some(seed) = value.as_integer() else {
                            return Err(ResolveError::UnknownSelectionSeed(task.clone()));
                        };
                        if scope == SelectionScope::Subset && !task_ids.iter().any(|id| id == task)
                        {
                            return Err(ResolveError::UnknownSelectionSeed(task.clone()));
                        }
                        seeds.insert(task.clone(), seed as u64);
                    }
                }
                Some(ResolvedSelection {
                    scope,
                    task_ids,
                    reason,
                    repetitions,
                    seeds,
                })
            }
            _ => None,
        };

        let mut trials: Vec<DeclaredTrial> = Vec::new();
        for entry in array_entries(table, "trials", "trials")? {
            let trial = DeclaredTrial {
                id: require_str(entry, "trials[].id")?,
                subject: require_str(entry, "trials[].subject")?,
                task: require_str(entry, "trials[].task")?,
                group: require_str(entry, "trials[].group")?,
            };
            if trials.iter().any(|t| t.id == trial.id) {
                return Err(ResolveError::DuplicateTrial(trial.id));
            }
            if !subjects.iter().any(|s| s.id == trial.subject) {
                return Err(ResolveError::UnknownTrialSubject {
                    trial: trial.id.clone(),
                    subject: trial.subject.clone(),
                });
            }
            trials.push(trial);
        }
        if trials.is_empty() && task_selection.is_none() {
            return Err(ResolveError::MissingTrials);
        }
        trials.sort_by(|a, b| a.id.cmp(&b.id));

        let resolved = ResolvedExperiment {
            schema,
            experiment_id,
            manifest_digest: String::new(),
            benchmark,
            subjects,
            edges,
            connections,
            connection_id: selection,
            model_controls,
            execution,
            storage,
            environment,
            selection: task_selection,
            trials,
        };
        if let Some(bound) = resolved.execution.max_output_tokens {
            let enforceable = matches!(
                resolved.model_controls.max_output_tokens,
                ControlValue::Value(value) if value == bound
            );
            if !enforceable {
                let reason = match resolved.model_controls.max_output_tokens {
                    ControlValue::Value(value) => {
                        format!("contradicts the pinned max_output_tokens control {value}")
                    }
                    _ => "no enforcement route without a pinned max_output_tokens model control"
                        .to_owned(),
                };
                return Err(ResolveError::UnenforceableExecutionBound {
                    bound: "max_output_tokens".to_owned(),
                    reason,
                });
            }
        }
        let mut resolved = resolved;
        let canonical = serde_json::to_string(&resolved)
            .map_err(|error| ResolveError::Canonicalize(error.to_string()))?;
        let digest = Sha256::digest(canonical.as_bytes());
        resolved.manifest_digest = digest.iter().map(|b| format!("{b:02x}")).collect();
        Ok(resolved)
    }

    /// Schema identity of the resolved contract.
    pub fn schema(&self) -> &str {
        &self.schema
    }

    /// Experiment id.
    pub fn experiment_id(&self) -> &str {
        &self.experiment_id
    }

    /// SHA-256 digest over the canonical serialization of the resolved
    /// contract. Formatting-only changes to the document leave it unchanged.
    pub fn manifest_digest(&self) -> &str {
        &self.manifest_digest
    }

    /// Frozen benchmark descriptor.
    pub fn benchmark(&self) -> &ResolvedBenchmark {
        &self.benchmark
    }

    /// Subject set, sorted by id.
    pub fn subjects(&self) -> &[ResolvedSubject] {
        &self.subjects
    }

    /// Directed comparison edges, sorted by id.
    pub fn edges(&self) -> &[ComparisonEdge] {
        &self.edges
    }

    /// Declared connections, sorted by id.
    pub fn connections(&self) -> &[Connection] {
        &self.connections
    }

    /// The selected connection: the single identity every subject's Agent
    /// process uses, without provider, protocol, model, or billing fallback.
    pub fn selected_connection(&self) -> &Connection {
        self.connections
            .iter()
            .find(|connection| connection.id == self.connection_id)
            .expect("resolution admits only a selected connection")
    }

    /// Frozen shared model controls.
    pub fn model_controls(&self) -> &ModelControls {
        &self.model_controls
    }

    /// Frozen execution mode and enforceable bounds.
    pub fn execution(&self) -> &ResolvedExecution {
        &self.execution
    }

    /// Frozen storage identity and capture policy.
    pub fn storage(&self) -> &ResolvedStorage {
        &self.storage
    }

    /// Frozen environment identity.
    pub fn environment(&self) -> &ResolvedEnvironment {
        &self.environment
    }

    /// Declared trials, sorted by id. Empty when the document selects
    /// tasks through `[selection]`; the corpus-aware expansion happens
    /// at freeze time (plan/run).
    pub fn trials(&self) -> &[DeclaredTrial] {
        &self.trials
    }

    /// The resolved `[selection]` block, when present.
    pub fn selection(&self) -> Option<&ResolvedSelection> {
        self.selection.as_ref()
    }

    /// Runner-only post-resolution trial expansion: replaces the
    /// declared-trial projection with the corpus-aware expansion of the
    /// document's `[selection]`. The manifest digest stays the document
    /// identity (the expansion is corpus-dependent and lives in the
    /// digest-bound run plan, never in the experiment digest).
    pub(crate) fn replace_trials(mut self, trials: Vec<DeclaredTrial>) -> Self {
        self.trials = trials;
        self
    }

    /// Project the selected connection for one pinned native client
    /// (P19-D02). The credential material must match the connection's
    /// declared reference; it is consumed here - inside the supervised
    /// Agent launch boundary - to render the exact client configuration
    /// and agent environment, without fallback to any ambient user value.
    pub fn project_connection(
        &self,
        client: NativeClient,
        credential: ResolvedCredential,
    ) -> Result<ConnectionProjection, ProjectionError> {
        project_connection(
            self.selected_connection(),
            self.model_controls(),
            client,
            credential,
        )
    }
}

/// Projects one selected connection for one pinned native client. The
/// credential material must match the connection's declared reference; it
/// is consumed here - inside the supervised Agent launch boundary - to
/// render the exact client configuration and agent environment, without
/// fallback to any ambient user value.
pub fn project_connection(
    connection: &Connection,
    controls: &ModelControls,
    client: NativeClient,
    credential: ResolvedCredential,
) -> Result<ConnectionProjection, ProjectionError> {
    if !credential.matches(&connection.credential) {
        return Err(ProjectionError::CredentialMismatch);
    }
    let controls = project_controls(controls, client, connection);
    let (config_filename, config_bytes, agent_env) = match client {
        NativeClient::Opi => {
            let text = render_opi_config(connection, &controls, &credential)?;
            let mut agent_env = BTreeMap::new();
            if let Some(variable) = credential.variable() {
                agent_env.insert(variable.to_owned(), credential.value().to_owned());
            }
            ("config.toml", text.into_bytes(), agent_env)
        }
        NativeClient::Pi => {
            let bytes = render_pi_config(connection, &controls, &credential)?;
            ("models.json", bytes, BTreeMap::new())
        }
    };
    Ok(ConnectionProjection {
        client,
        connection: connection.clone(),
        controls,
        config_filename,
        config_bytes,
        agent_env,
    })
}

/// The per-control capability projection for one client. Origins are
/// recorded, never inferred from requests: every effective value comes
/// from the frozen requested control, and every unsupported verdict names
/// the pinned client surface's reason.
fn project_controls(
    requested: &ModelControls,
    client: NativeClient,
    connection: &Connection,
) -> ControlProjection {
    // Both pinned clients express context/output limits natively.
    let context_window = map_numeric(
        requested.context_window.clone(),
        "the requested control is not an enforced value",
    );
    let max_output_tokens = map_numeric(
        requested.max_output_tokens.clone(),
        "the requested control is not an enforced value",
    );
    // Reasoning effort flows through each client's thinking-level map.
    let reasoning = match &requested.reasoning {
        ControlValue::Value(value) => ProjectedControl::Effective {
            value: value.clone(),
        },
        ControlValue::Omitted => ProjectedControl::unsupported_from(
            ControlValue::Omitted,
            "the requested control is not an enforced value",
        ),
        ControlValue::Unknown => ProjectedControl::unsupported_from(
            ControlValue::Unknown,
            "the requested control is not an enforced value",
        ),
    };
    // Sampling controls: only the pi config surface expresses them,
    // and only on OpenAI-compatible wires.
    let expressible_sampling = matches!(
        (client, connection.api),
        (
            NativeClient::Pi,
            ConnectionApi::OpenAiCompletions | ConnectionApi::OpenAiResponses
        )
    );
    let sampling_reason = match (client, connection.api) {
        (NativeClient::Opi, _) => "the pinned Opi custom-model config has no sampling value field",
        (NativeClient::Pi, ConnectionApi::AnthropicMessages) => {
            "pi applies sampling parameters only on OpenAI-compatible APIs"
        }
        (NativeClient::Pi, _) => "pi model sampling parameters",
    };
    let map_sampling = |control: ControlValue<f64>| {
        if expressible_sampling {
            match control {
                ControlValue::Value(value) => ProjectedControl::Effective { value },
                ControlValue::Omitted => ProjectedControl::unsupported_from(
                    ControlValue::Omitted,
                    "the requested control is not an enforced value",
                ),
                ControlValue::Unknown => ProjectedControl::unsupported_from(
                    ControlValue::Unknown,
                    "the requested control is not an enforced value",
                ),
            }
        } else {
            ProjectedControl::unsupported_from(control, sampling_reason)
        }
    };
    ControlProjection {
        temperature: map_sampling(requested.temperature.clone()),
        top_p: map_sampling(requested.top_p.clone()),
        max_output_tokens,
        context_window,
        reasoning,
    }
}

/// Maps a numeric control: effective values pass through; markers record
/// the requested marker with a static reason.
fn map_numeric<T>(control: ControlValue<T>, reason: &'static str) -> ProjectedControl<T> {
    match control {
        ControlValue::Value(value) => ProjectedControl::Effective { value },
        ControlValue::Omitted => ProjectedControl::unsupported_from(ControlValue::Omitted, reason),
        ControlValue::Unknown => ProjectedControl::unsupported_from(ControlValue::Unknown, reason),
    }
}

/// The exact Opi TOML configuration for the selected connection. The
/// credential value never enters this file: `api_key_env` names the
/// variable and the agent environment carries the value.
fn render_opi_config(
    connection: &Connection,
    controls: &ControlProjection,
    credential: &ResolvedCredential,
) -> Result<String, ProjectionError> {
    let guard = |value: &str| {
        if value.contains('"') || value.contains('\n') {
            Err(ProjectionError::ConfigRender(
                "connection identity contains a quote or newline".to_owned(),
            ))
        } else {
            Ok(value.to_owned())
        }
    };
    let provider = guard(&connection.provider)?;
    let base_url = guard(&connection.base_url)?;
    let model = guard(&connection.model)?;
    let auth_scheme = match connection.api {
        ConnectionApi::AnthropicMessages => "api-key",
        ConnectionApi::OpenAiCompletions | ConnectionApi::OpenAiResponses => "bearer",
    };
    let key_env = credential.variable().unwrap_or("OPI_EVAL_AGENT_KEY");
    let mut body = format!(
        "# opi-eval projected selected connection: {id} ({api}).\n\n\
[providers.custom.{provider}]\n\
name = \"opi-eval {provider}\"\n\
base_url = \"{base_url}\"\n\
api_key_env = \"{key_env}\"\n\
auth_scheme = \"{auth_scheme}\"\n\
api = \"{api}\"\n\n\
[[providers.custom.{provider}.models]]\n\
id = \"{model}\"\n\
display_name = \"opi-eval {provider}\"\n",
        id = connection.id,
        api = connection.api.as_str(),
        provider = provider,
        base_url = base_url,
        key_env = key_env,
        auth_scheme = auth_scheme,
        model = model,
    );
    if let ProjectedControl::Effective { value } = controls.context_window {
        body.push_str(&format!("context_window = {value}\n"));
    }
    if let ProjectedControl::Effective { value } = controls.max_output_tokens {
        body.push_str(&format!("max_output_tokens = {value}\n"));
    }
    if let ProjectedControl::Effective { value } = &controls.reasoning {
        body.push_str("supports_thinking = true\n");
        body.push_str(&format!(
            "thinking_level_map = {{ {value} = \"{value}\" }}\n"
        ));
    }
    Ok(body)
}

/// The exact pi `models.json` for the selected connection. pi's config
/// surface resolves the credential inline, so the resolved value is
/// embedded here and lives only in the private launch path.
fn render_pi_config(
    connection: &Connection,
    controls: &ControlProjection,
    credential: &ResolvedCredential,
) -> Result<Vec<u8>, ProjectionError> {
    let mut model = serde_json::json!({
        "id": connection.model,
    });
    if let ProjectedControl::Effective { value } = controls.context_window {
        model["contextWindow"] = value.into();
    }
    if let ProjectedControl::Effective { value } = controls.max_output_tokens {
        model["maxTokens"] = value.into();
    }
    let mut sampling = serde_json::Map::new();
    if let ProjectedControl::Effective { value } = controls.temperature {
        sampling.insert("temperature".to_owned(), value.into());
    }
    if let ProjectedControl::Effective { value } = controls.top_p {
        sampling.insert("top_p".to_owned(), value.into());
    }
    if !sampling.is_empty() {
        model["samplingParams"] = serde_json::Value::Object(sampling);
    }
    if let ProjectedControl::Effective { value } = &controls.reasoning {
        model["reasoning"] = true.into();
        model["thinkingLevelMap"] = serde_json::json!({ value.clone(): value });
    }
    let config = serde_json::json!({
        "providers": {
            connection.provider.clone(): {
                "baseUrl": connection.base_url,
                "api": connection.api.as_str(),
                "apiKey": credential.value(),
                "models": [model],
            },
        },
    });
    serde_json::to_vec(&config).map_err(|error| ProjectionError::ConfigRender(error.to_string()))
}

fn require_str(
    table: &toml::map::Map<String, toml::Value>,
    field: &str,
) -> Result<String, ResolveError> {
    match table.get(field.rsplit('.').next().unwrap_or(field)) {
        Some(toml::Value::String(value)) if !value.trim().is_empty() => Ok(value.clone()),
        _ => Err(ResolveError::MissingField(field.to_owned())),
    }
}

fn required_table<'a>(
    table: &'a toml::map::Map<String, toml::Value>,
    key: &str,
) -> Result<&'a toml::map::Map<String, toml::Value>, ResolveError> {
    table
        .get(key)
        .and_then(toml::Value::as_table)
        .ok_or_else(|| ResolveError::MissingField(key.to_owned()))
}

fn array_entries<'a>(
    table: &'a toml::map::Map<String, toml::Value>,
    key: &str,
    context: &'static str,
) -> Result<Vec<&'a toml::map::Map<String, toml::Value>>, ResolveError> {
    match table.get(key) {
        Some(toml::Value::Array(entries)) => entries
            .iter()
            .map(|entry| {
                entry
                    .as_table()
                    .ok_or(ResolveError::EntryNotATable(context))
            })
            .collect(),
        _ => Ok(Vec::new()),
    }
}

/// Optional non-negative integer field; `None` when absent or empty.
fn optional_u64(
    table: &toml::map::Map<String, toml::Value>,
    field: &str,
) -> Result<Option<u64>, ResolveError> {
    match table.get(field) {
        None | Some(toml::Value::String(_)) => Ok(None),
        Some(toml::Value::Integer(value)) if *value > 0 => Ok(Some(*value as u64)),
        Some(_) => Err(ResolveError::InvalidControlValue {
            control: field.to_owned(),
            expected: "a positive integer",
        }),
    }
}

/// Whether a base URL is a parseable http(s) loopback route. Anything
/// unparsable fails closed: it is not a loopback route.
fn is_loopback_base_url(base_url: &str) -> bool {
    let rest = base_url
        .strip_prefix("http://")
        .or_else(|| base_url.strip_prefix("https://"))
        .unwrap_or("");
    let authority = rest.split(['/', '?']).next().unwrap_or("");
    let host = authority.rsplit('@').next().unwrap_or("");
    let host = if host.starts_with('[') {
        host.split(']').next().unwrap_or("")
    } else {
        host.rsplit_once(':').map(|(h, _)| h).unwrap_or(host)
    };
    matches!(
        host.to_ascii_lowercase().as_str(),
        "127.0.0.1" | "localhost" | "[::1]" | "::1"
    )
}

fn control_marker(value: &str) -> Option<ControlValue<()>> {
    match value {
        "omitted" => Some(ControlValue::Omitted),
        "unknown" => Some(ControlValue::Unknown),
        _ => None,
    }
}

fn float_control(
    table: &toml::map::Map<String, toml::Value>,
    control: &str,
) -> Result<ControlValue<f64>, ResolveError> {
    let value = table
        .get(control)
        .ok_or_else(|| ResolveError::MissingModelControl(control.to_owned()))?;
    match value {
        toml::Value::Float(value) => Ok(ControlValue::Value(*value)),
        toml::Value::String(text) => match control_marker(text.as_str()) {
            Some(ControlValue::Omitted) => Ok(ControlValue::Omitted),
            Some(ControlValue::Unknown) => Ok(ControlValue::Unknown),
            Some(ControlValue::Value(())) | None => Err(ResolveError::InvalidControlMarker {
                control: control.to_owned(),
                marker: text.clone(),
            }),
        },
        _ => Err(ResolveError::InvalidControlValue {
            control: control.to_owned(),
            expected: "a float",
        }),
    }
}

fn u64_control(
    table: &toml::map::Map<String, toml::Value>,
    control: &str,
) -> Result<ControlValue<u64>, ResolveError> {
    let value = table
        .get(control)
        .ok_or_else(|| ResolveError::MissingModelControl(control.to_owned()))?;
    match value {
        toml::Value::Integer(value) if *value >= 0 => Ok(ControlValue::Value(*value as u64)),
        toml::Value::String(text) => match control_marker(text.as_str()) {
            Some(ControlValue::Omitted) => Ok(ControlValue::Omitted),
            Some(ControlValue::Unknown) => Ok(ControlValue::Unknown),
            Some(ControlValue::Value(())) | None => Err(ResolveError::InvalidControlMarker {
                control: control.to_owned(),
                marker: text.clone(),
            }),
        },
        _ => Err(ResolveError::InvalidControlValue {
            control: control.to_owned(),
            expected: "a non-negative integer",
        }),
    }
}

fn string_control(
    table: &toml::map::Map<String, toml::Value>,
    control: &str,
) -> Result<ControlValue<String>, ResolveError> {
    let value = table
        .get(control)
        .ok_or_else(|| ResolveError::MissingModelControl(control.to_owned()))?;
    match value {
        toml::Value::String(value) => match value.as_str() {
            "omitted" => Ok(ControlValue::Omitted),
            "unknown" => Ok(ControlValue::Unknown),
            other => Ok(ControlValue::Value(other.to_owned())),
        },
        _ => Err(ResolveError::InvalidControlValue {
            control: control.to_owned(),
            expected: "a string",
        }),
    }
}

/// Which pinned native client a connection projection targets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeClient {
    /// The exact built Opi binary.
    Opi,
    /// The exact pinned pi bundle.
    Pi,
}

impl NativeClient {
    /// Product name of this client.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Opi => "opi",
            Self::Pi => "pi",
        }
    }
}

/// Credential material resolved for one selected connection at the
/// supervised Agent launch boundary. The value never enters exportable
/// artifacts: it lives only in the private launch handoff and the task
/// environment's removed-before-verifier staging directory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResolvedCredential {
    /// Value read from the declared environment variable.
    FromEnv {
        /// Declared variable name.
        variable: String,
        /// Resolved value.
        value: String,
    },
    /// Value read from the declared private file.
    FromFile {
        /// Declared file path.
        path: String,
        /// Resolved value.
        value: String,
    },
}

impl ResolvedCredential {
    /// Whether this material matches the connection's declared reference.
    fn matches(&self, reference: &CredentialReference) -> bool {
        match (self, reference) {
            (Self::FromEnv { variable, .. }, CredentialReference::Env { variable: declared }) => {
                variable == declared
            }
            (Self::FromFile { path, .. }, CredentialReference::File { path: declared }) => {
                path == declared
            }
            _ => false,
        }
    }

    /// The resolved secret value.
    fn value(&self) -> &str {
        match self {
            Self::FromEnv { value, .. } | Self::FromFile { value, .. } => value,
        }
    }

    /// The declared environment variable name, when the reference is an
    /// environment reference.
    fn variable(&self) -> Option<&str> {
        match self {
            Self::FromEnv { variable, .. } => Some(variable),
            Self::FromFile { .. } => None,
        }
    }
}

/// One projected model control: the exact effective value, or an explicit
/// unsupported verdict with its reason. Origins are recorded, never
/// inferred from requests: effective values originate from the frozen
/// requested control, and nothing else.
#[derive(Debug, Clone, PartialEq)]
pub enum ProjectedControl<T> {
    /// The projected client configuration expresses exactly this value.
    Effective {
        /// Effective value (origin: the frozen requested control).
        value: T,
    },
    /// The pinned client configuration surface cannot express this
    /// control; the requested value or marker is retained with the reason.
    Unsupported {
        /// The frozen requested value or marker.
        requested: ControlValue<T>,
        /// Why the client cannot express it.
        reason: String,
    },
}

impl<T> ProjectedControl<T> {
    fn unsupported_from(requested: ControlValue<T>, reason: &str) -> Self {
        Self::Unsupported {
            requested,
            reason: reason.to_owned(),
        }
    }
}

/// The per-control projection for one client and connection.
#[derive(Debug, Clone, PartialEq)]
pub struct ControlProjection {
    /// Sampling temperature.
    pub temperature: ProjectedControl<f64>,
    /// Nucleus sampling control.
    pub top_p: ProjectedControl<f64>,
    /// Output token limit.
    pub max_output_tokens: ProjectedControl<u64>,
    /// Context window limit.
    pub context_window: ProjectedControl<u64>,
    /// Reasoning effort control.
    pub reasoning: ProjectedControl<String>,
}

/// Fail-closed projection failures.
#[derive(Debug, Error)]
pub enum ProjectionError {
    /// The credential material does not match the selected connection's
    /// declared reference.
    #[error("credential material does not match the selected connection reference")]
    CredentialMismatch,
    /// The projected client configuration could not be rendered from the
    /// frozen identity.
    #[error("projected configuration cannot be rendered: {0}")]
    ConfigRender(String),
}

/// The typed selected-connection projection for one pinned native client
/// (P19-D02): the exact configuration bytes the client consumes, the agent
/// environment additions, and the per-control effective/unsupported
/// verdicts. The projection is the single owner of request identity;
/// adapters and integrations apply it verbatim and never improvise or
/// fall back.
#[derive(Debug, Clone)]
pub struct ConnectionProjection {
    /// Target client.
    pub client: NativeClient,
    /// The selected connection this projection freezes.
    pub connection: Connection,
    /// Per-control projection with recorded origins.
    pub controls: ControlProjection,
    /// Config file name the client expects.
    pub config_filename: &'static str,
    /// Exact projected configuration bytes (private: may embed credential
    /// material for clients whose config surface requires inline values).
    pub config_bytes: Vec<u8>,
    /// Agent-process environment additions (credential values live here
    /// for clients whose config references an environment variable).
    pub agent_env: BTreeMap<String, String>,
}

#[cfg(test)]
mod projection_tests {
    use super::*;

    fn doc(api: &str) -> String {
        CONNECTION_DOC_TEMPLATE.replace("__API__", api)
    }

    const CONNECTION_DOC_TEMPLATE: &str = r#"
schema = "opi-eval-experiment/2"
experiment_id = "projection-tests"
connection = "primary"

[benchmark]
name = "terminal-bench"
revision = "2.1"
dataset = "terminal-bench-2.1"

[[subjects]]
id = "opi"
product = "opi"
version = "0.8.2"

[[subjects]]
id = "pi"
product = "pi"
version = "0.84.1"

[[edges]]
id = "opi-baseline-vs-pi-candidate"
baseline = "opi"
candidate = "pi"

[[connections]]
id = "primary"
provider = "scripted"
api = "__API__"
base_url = "http://127.0.0.1:48127/v1"
model = "eval"
credential_env = "OPI_EVAL_SCRIPTED_KEY"

[model_controls]
temperature = 0.25
top_p = 0.9
max_output_tokens = 4096
context_window = 128000
reasoning = "high"
required = ["max_output_tokens", "context_window", "reasoning"]

[execution]
mode = "native-fixture"

[storage]
cache = "cache"
results = "results"
capture_policy = "full"

[environment]
platform = "linux"
architecture = "x86_64"
cwd_policy = "workspace-root"

[[trials]]
id = "trial-opi-hello"
subject = "opi"
task = "hello"
group = "hello"

[[trials]]
id = "trial-pi-hello"
subject = "pi"
task = "hello"
group = "hello"
"#;

    fn resolved(api: &str) -> ResolvedExperiment {
        ResolvedExperiment::resolve(&doc(api)).unwrap()
    }

    fn credential() -> ResolvedCredential {
        ResolvedCredential::FromEnv {
            variable: "OPI_EVAL_SCRIPTED_KEY".to_owned(),
            value: "dummy-scripted-credential".to_owned(),
        }
    }

    #[test]
    fn projection_maps_every_control_for_both_clients_on_every_api() {
        for api in [
            "anthropic-messages",
            "openai-completions",
            "openai-responses",
        ] {
            for client in [NativeClient::Opi, NativeClient::Pi] {
                let experiment = resolved(api);
                let projection = experiment
                    .project_connection(client, credential())
                    .unwrap_or_else(|e| panic!("{api}/{client:?}: {e}"));
                // Limits both clients express natively.
                assert_eq!(
                    projection.controls.context_window,
                    ProjectedControl::Effective { value: 128_000 },
                    "{api}/{client:?}"
                );
                assert_eq!(
                    projection.controls.max_output_tokens,
                    ProjectedControl::Effective { value: 4096 },
                    "{api}/{client:?}"
                );
                // Reasoning effort flows through each client's thinking map.
                assert_eq!(
                    projection.controls.reasoning,
                    ProjectedControl::Effective {
                        value: "high".to_owned()
                    },
                    "{api}/{client:?}"
                );
                // Sampling controls: only pi, and only on OpenAI-compatible
                // wires, can express them.
                match (client, api) {
                    (NativeClient::Pi, "openai-completions")
                    | (NativeClient::Pi, "openai-responses") => {
                        assert_eq!(
                            projection.controls.temperature,
                            ProjectedControl::Effective { value: 0.25 },
                            "{api}/{client:?}"
                        );
                        assert_eq!(
                            projection.controls.top_p,
                            ProjectedControl::Effective { value: 0.9 },
                            "{api}/{client:?}"
                        );
                    }
                    _ => {
                        for control in
                            [&projection.controls.temperature, &projection.controls.top_p]
                        {
                            assert!(
                                matches!(control, ProjectedControl::Unsupported { .. }),
                                "{api}/{client:?}"
                            );
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn projected_config_bytes_carry_the_exact_request_identity() {
        let experiment = resolved("anthropic-messages");

        let opi = experiment
            .project_connection(NativeClient::Opi, credential())
            .unwrap();
        assert_eq!(opi.config_filename, "config.toml");
        let text = String::from_utf8(opi.config_bytes.clone()).unwrap();
        assert!(
            text.contains("base_url = \"http://127.0.0.1:48127/v1\""),
            "{text}"
        );
        assert!(text.contains("api = \"anthropic-messages\""), "{text}");
        assert!(
            text.contains("api_key_env = \"OPI_EVAL_SCRIPTED_KEY\""),
            "{text}"
        );
        assert!(text.contains("id = \"eval\""), "{text}");
        // The credential VALUE never enters the Opi config; the agent
        // environment carries it instead.
        assert!(!text.contains("dummy-scripted-credential"), "{text}");
        assert_eq!(
            opi.agent_env
                .get("OPI_EVAL_SCRIPTED_KEY")
                .map(String::as_str),
            Some("dummy-scripted-credential")
        );

        let pi = experiment
            .project_connection(NativeClient::Pi, credential())
            .unwrap();
        assert_eq!(pi.config_filename, "models.json");
        let value: serde_json::Value = serde_json::from_slice(&pi.config_bytes).unwrap();
        let provider = &value["providers"]["scripted"];
        assert_eq!(provider["baseUrl"], "http://127.0.0.1:48127/v1");
        assert_eq!(provider["api"], "anthropic-messages");
        assert_eq!(provider["apiKey"], "dummy-scripted-credential");
        assert_eq!(provider["models"][0]["id"], "eval");
        assert_eq!(provider["models"][0]["contextWindow"], 128000);
        assert_eq!(provider["models"][0]["maxTokens"], 4096);
        // pi reads the credential from its isolated config; no agent env.
        assert!(pi.agent_env.is_empty());
    }

    #[test]
    fn projection_rejects_credential_not_matching_the_selected_reference() {
        let experiment = resolved("openai-completions");
        let mismatched = ResolvedCredential::FromEnv {
            variable: "SOME_OTHER_VARIABLE".to_owned(),
            value: "value".to_owned(),
        };
        for client in [NativeClient::Opi, NativeClient::Pi] {
            assert!(matches!(
                experiment.project_connection(client, mismatched.clone()),
                Err(ProjectionError::CredentialMismatch)
            ));
        }
    }

    #[test]
    fn marker_controls_project_as_unsupported_with_their_reason() {
        let source = doc("openai-completions")
            .replace("temperature = 0.25", "temperature = \"unknown\"")
            .replace("top_p = 0.9", "top_p = \"omitted\"")
            .replace("reasoning = \"high\"", "reasoning = \"unknown\"")
            .replace(
                "required = [\"max_output_tokens\", \"context_window\", \"reasoning\"]",
                "required = [\"max_output_tokens\", \"context_window\"]",
            );
        let experiment = ResolvedExperiment::resolve(&source).unwrap();
        let projection = experiment
            .project_connection(NativeClient::Pi, credential())
            .unwrap();
        assert!(matches!(
            projection.controls.temperature,
            ProjectedControl::Unsupported {
                requested: ControlValue::Unknown,
                ..
            }
        ));
        assert!(matches!(
            projection.controls.top_p,
            ProjectedControl::Unsupported {
                requested: ControlValue::Omitted,
                ..
            }
        ));
        assert!(matches!(
            projection.controls.reasoning,
            ProjectedControl::Unsupported {
                requested: ControlValue::Unknown,
                ..
            }
        ));
    }
}
