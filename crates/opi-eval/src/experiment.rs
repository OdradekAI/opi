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

use std::fmt;

use serde::Serialize;
use sha2::{Digest, Sha256};
use thiserror::Error;

/// Schema identity accepted by [`ResolvedExperiment::resolve`].
pub const EXPERIMENT_SCHEMA: &str = "opi-eval-experiment/1";

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
/// value or an `omitted`/`unknown` marker. Fallbacks are prohibited.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ModelControls {
    /// Provider identity used by every subject of the experiment.
    pub provider: String,
    /// Exact model identity.
    pub model: String,
    /// Endpoint class, for example `local` or `chat-completions`.
    pub endpoint_class: String,
    /// Sampling temperature.
    pub temperature: ControlValue<f64>,
    /// Output token limit.
    pub max_output_tokens: ControlValue<u64>,
    /// Reasoning effort control.
    pub reasoning: ControlValue<String>,
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
    model_controls: ModelControls,
    environment: ResolvedEnvironment,
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
        if edges.is_empty() {
            return Err(ResolveError::MissingEdges);
        }
        edges.sort_by(|a, b| a.id.cmp(&b.id));

        let controls_table = required_table(table, "model_controls")?;
        let model_controls = ModelControls {
            provider: require_str(controls_table, "model_controls.provider")?,
            model: require_str(controls_table, "model_controls.model")?,
            endpoint_class: require_str(controls_table, "model_controls.endpoint_class")?,
            temperature: float_control(controls_table, "temperature")?,
            max_output_tokens: u64_control(controls_table, "max_output_tokens")?,
            reasoning: string_control(controls_table, "reasoning")?,
        };

        let environment_table = required_table(table, "environment")?;
        let environment = ResolvedEnvironment {
            platform: require_str(environment_table, "environment.platform")?,
            architecture: require_str(environment_table, "environment.architecture")?,
            cwd_policy: require_str(environment_table, "environment.cwd_policy")?,
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
        if trials.is_empty() {
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
            model_controls,
            environment,
            trials,
        };
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

    /// Frozen shared model controls.
    pub fn model_controls(&self) -> &ModelControls {
        &self.model_controls
    }

    /// Frozen environment identity.
    pub fn environment(&self) -> &ResolvedEnvironment {
        &self.environment
    }

    /// Declared trials, sorted by id.
    pub fn trials(&self) -> &[DeclaredTrial] {
        &self.trials
    }
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
