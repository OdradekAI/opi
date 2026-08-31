//! Crate-private provisional trajectory and causal-span projection
//! (Phase 18 task 18.11).
//!
//! [`ProjectionPipeline`] projects the settled, pre-seal facts of one trial —
//! the Agent's native events, the optional native verification lifecycle, the
//! collected native artifacts, the final-workspace capture, and the trial
//! lifecycle ladder — into one provisional trajectory of [`Node`]s connected
//! by causal [`Edge`]s and grouped into [`Span`]s.
//!
//! Invariants carried from `P18-TRJ-001..005`:
//!
//! - every node and span retains the native artifact and adapter rule that
//!   produced it ([`Provenance`]);
//! - no adapter invents facts: measured values are [`Fact::Known`] only when
//!   the native evidence carries them, and stay [`Fact::Unknown`] with a
//!   typed reason otherwise;
//! - ordering is per source: cross-source ordering is never claimed, and
//!   parallel activity stays parent/causal ordered, never a fabricated
//!   global total order;
//! - the trajectory is pre-seal by construction: sealing happens after
//!   projection and its result is recorded only in the outer
//!   [`TrajectoryReceipt`], never as a trajectory node;
//! - everything here stays provisional and crate-private until the complete
//!   Phase 18 conformance matrix passes.
//!
//! The sole later production caller is the crate's experiment runner
//! (task 18.12); this module never links an Agent or benchmark runtime.

use std::collections::BTreeMap;
use std::path::Path;

use crate::agent::opi::sha256_hex;
use crate::agent::process::{AgentCompletion, AgentRecord, Fact, NativeArtifact};
use crate::benchmark::process::BenchmarkRecord;
use crate::runner::lifecycle::{TrialLifecycle, TrialPhase};
use thiserror::Error;

/// Schema identity of the provisional projection (`P18-TRJ-005`: the
/// trajectory hypothesis itself stays provisional until the conformance
/// matrix passes).
pub(crate) const TRAJECTORY_SCHEMA: &str = "phase18-provisional-trajectory/1";

/// The settled pre-seal inputs of exactly one trial.
pub(crate) struct TrialInputs<'a> {
    /// The settled Agent record (one per trial, on every path).
    pub(crate) agent: &'a AgentRecord,
    /// The settled native verification record, when this trial was verified.
    pub(crate) benchmark: Option<&'a BenchmarkRecord>,
    /// The trial lifecycle machine, which must still be pre-seal.
    pub(crate) lifecycle: &'a TrialLifecycle,
    /// The final Agent output workspace the verifier grades, when captured.
    pub(crate) workspace: Option<&'a Path>,
}

/// Stable node identity inside one trajectory.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct NodeId(pub(crate) u32);

/// Which native source produced a node. Ordering claims are only ever made
/// inside one source (`P18-TRJ-003`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NodeSource {
    /// The durable trial lifecycle ladder.
    Lifecycle,
    /// The supervised Agent process execution.
    AgentExecution,
    /// One native Agent event line.
    AgentEvents,
    /// One collected native Agent artifact.
    AgentArtifacts,
    /// The final-workspace capture.
    Workspace,
    /// The supervised verifier process execution.
    VerifierExecution,
    /// The settled grader verdict.
    Grader,
}

/// The projected shape of one native fact.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum NodeKind {
    /// One reached rung of the pre-seal lifecycle ladder.
    Lifecycle { phase: TrialPhase },
    /// The supervised Agent execution itself (exit, wall time, usage).
    AgentExecution,
    /// One native Agent event line (Opi evidence record or pi session event).
    AgentEvent { event_type: String },
    /// One collected native artifact reference.
    Artifact { role: String },
    /// The final-workspace capture digest.
    WorkspaceCapture,
    /// The supervised verifier execution (exit, wall time).
    VerifierExecution,
    /// The settled native grader verdict.
    Grader { family: String, revision: String },
}

impl NodeKind {
    /// Stable tag used by the canonical content digest.
    pub(crate) fn tag(&self) -> String {
        match self {
            NodeKind::Lifecycle { phase } => format!("lifecycle/{phase:?}"),
            NodeKind::AgentExecution => "agent-execution".to_owned(),
            NodeKind::AgentEvent { event_type } => format!("agent-event/{event_type}"),
            NodeKind::Artifact { role } => format!("artifact/{role}"),
            NodeKind::WorkspaceCapture => "workspace-capture".to_owned(),
            NodeKind::VerifierExecution => "verifier-execution".to_owned(),
            NodeKind::Grader { family, revision } => format!("grader/{family}/{revision}"),
        }
    }
}

/// `P18-TRJ-001`: every projected fact retains the native artifact and the
/// adapter rule that produced it. Lifecycle facts have no native artifact;
/// their reference is the durable trial reservation itself.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Provenance {
    pub(crate) artifact_role: String,
    pub(crate) artifact_sha256: Option<String>,
    pub(crate) rule: String,
}

/// One projected trajectory node.
#[derive(Debug, Clone)]
pub(crate) struct Node {
    pub(crate) id: NodeId,
    pub(crate) source: NodeSource,
    pub(crate) kind: NodeKind,
    pub(crate) provenance: Provenance,
    /// Per-source order (native sequence or line index), never a global
    /// total order claim.
    pub(crate) source_order: Option<u64>,
    /// Retained measured facts with typed unknowns.
    pub(crate) facts: BTreeMap<String, Fact>,
}

/// How two nodes relate. There is deliberately no total-order relation: a
/// global order across sources is not evidence this crate possesses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EdgeRelation {
    /// Native parent correlation inside one Agent event stream.
    Parent,
    /// One node caused another across sources.
    Caused,
    /// Order inside one source only.
    SourceOrder,
}

#[derive(Debug, Clone)]
pub(crate) struct Edge {
    pub(crate) from: NodeId,
    pub(crate) to: NodeId,
    pub(crate) relation: EdgeRelation,
}

/// The supplemental causal span kinds; each exists only where the native
/// evidence supports it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SpanKind {
    /// The supervised Agent process.
    Process,
    /// The supervised verifier process.
    Grader,
    /// One Agent turn.
    Turn,
    /// One LLM message exchange.
    Llm,
    /// One tool execution.
    Tool,
    /// One retry.
    Retry,
    /// One compaction.
    Compaction,
}

#[derive(Debug, Clone)]
pub(crate) struct Span {
    pub(crate) kind: SpanKind,
    pub(crate) covers: Vec<NodeId>,
    pub(crate) provenance: Provenance,
    pub(crate) facts: BTreeMap<String, Fact>,
}

/// The provisional trajectory of one trial: nodes, causal edges, and spans.
#[derive(Debug, Clone)]
pub(crate) struct ProvisionalTrajectory {
    pub(crate) schema: &'static str,
    /// The Agent product whose native evidence this trajectory projects.
    pub(crate) product: String,
    pub(crate) nodes: Vec<Node>,
    pub(crate) edges: Vec<Edge>,
    pub(crate) spans: Vec<Span>,
}

impl ProvisionalTrajectory {
    /// The node with `id`, if present.
    pub(crate) fn node(&self, id: NodeId) -> Option<&Node> {
        self.nodes.iter().find(|n| n.id == id)
    }

    /// Canonical content digest of the pre-seal projection. Deterministic:
    /// BTreeMap facts and stable node ids keep the serialized form canonical
    /// for identical inputs.
    pub(crate) fn content_digest(&self) -> String {
        sha256_hex(&serde_json::to_vec(&self.digest_value()).expect("canonical JSON serializes"))
    }

    /// Canonical byte form of the whole pre-seal projection: nodes with
    /// their kinds, provenance, and facts, edges, and spans. Deterministic
    /// for identical inputs; staged as sealed trial evidence so a sealed
    /// bundle retains the trajectory itself, not only its digest
    /// (`P18-BND-001`).
    pub(crate) fn canonical_bytes(&self) -> Vec<u8> {
        serde_json::to_vec(&self.canonical_value()).expect("canonical JSON serializes")
    }

    /// The digest payload: schema, product, node identities and facts, and
    /// edges.
    fn digest_value(&self) -> serde_json::Value {
        let nodes: Vec<serde_json::Value> = self
            .nodes
            .iter()
            .map(|n| {
                serde_json::json!({
                    "id": n.id.0,
                    "kind": n.kind.tag(),
                    "source_order": n.source_order,
                    "facts": facts_json(&n.facts),
                })
            })
            .collect();
        let edges: Vec<serde_json::Value> = self
            .edges
            .iter()
            .map(|e| {
                serde_json::json!({
                    "from": e.from.0,
                    "to": e.to.0,
                    "relation": format!("{:?}", e.relation),
                })
            })
            .collect();
        serde_json::json!({
            "schema": self.schema,
            "product": self.product,
            "nodes": nodes,
            "edges": edges,
        })
    }

    /// The canonical retention form: the digest payload plus every node's
    /// provenance and the span layer.
    fn canonical_value(&self) -> serde_json::Value {
        let mut value = self.digest_value();
        let nodes: Vec<serde_json::Value> = self
            .nodes
            .iter()
            .map(|n| {
                serde_json::json!({
                    "id": n.id.0,
                    "kind": n.kind.tag(),
                    "source": format!("{:?}", n.source),
                    "source_order": n.source_order,
                    "provenance": {
                        "artifact_role": n.provenance.artifact_role,
                        "artifact_sha256": n.provenance.artifact_sha256,
                        "rule": n.provenance.rule,
                    },
                    "facts": facts_json(&n.facts),
                })
            })
            .collect();
        let spans: Vec<serde_json::Value> = self
            .spans
            .iter()
            .map(|s| {
                serde_json::json!({
                    "kind": format!("{:?}", s.kind),
                    "covers": s.covers.iter().map(|id| id.0).collect::<Vec<_>>(),
                    "provenance": {
                        "artifact_role": s.provenance.artifact_role,
                        "artifact_sha256": s.provenance.artifact_sha256,
                        "rule": s.provenance.rule,
                    },
                    "facts": facts_json(&s.facts),
                })
            })
            .collect();
        value["nodes"] = serde_json::json!(nodes);
        value["spans"] = serde_json::json!(spans);
        value
    }

    /// Validate the invariants this module owns. `project` runs this before
    /// returning; it stays callable so later maintainers and tests can
    /// re-check a trajectory after construction.
    pub(crate) fn validate(&self) -> Result<(), ProjectionError> {
        if self.schema != TRAJECTORY_SCHEMA {
            return Err(ProjectionError::SchemaDrift {
                found: self.schema.to_owned(),
            });
        }
        if self.product.is_empty() {
            return Err(ProjectionError::MissingProduct);
        }
        for node in &self.nodes {
            // The trajectory is pre-seal by construction: sealing consumes
            // the trajectory, so a seal-phase or seal-result node would be
            // self-referential.
            if let NodeKind::Lifecycle {
                phase: TrialPhase::Sealed | TrialPhase::Graded | TrialPhase::Reported,
            } = node.kind
            {
                return Err(ProjectionError::SelfReferentialSeal { node: node.id });
            }
            if let NodeKind::Artifact { role } = &node.kind
                && (role.starts_with("seal") || role.starts_with("bundle/"))
            {
                return Err(ProjectionError::SelfReferentialSeal { node: node.id });
            }
            // Synthetic parity: a Known fact whose origin names the other
            // Agent product is a comparison claim, and comparisons belong to
            // the pairing/comparison seam, never to one product's trajectory.
            for (name, fact) in &node.facts {
                if let Fact::Known { origin, .. } = fact
                    && names_other_product(&self.product, origin)
                {
                    return Err(ProjectionError::SyntheticParity {
                        node: node.id,
                        fact: name.clone(),
                    });
                }
            }
        }
        for edge in &self.edges {
            let (from, to) = match (self.node(edge.from), self.node(edge.to)) {
                (Some(from), Some(to)) => (from, to),
                _ => {
                    return Err(ProjectionError::DanglingEdge {
                        from: edge.from,
                        to: edge.to,
                    });
                }
            };
            match edge.relation {
                EdgeRelation::SourceOrder if from.source != to.source => {
                    return Err(ProjectionError::InventedChronology {
                        from: edge.from,
                        to: edge.to,
                    });
                }
                EdgeRelation::Parent
                    if from.source != NodeSource::AgentEvents
                        || to.source != NodeSource::AgentEvents =>
                {
                    return Err(ProjectionError::InventedChronology {
                        from: edge.from,
                        to: edge.to,
                    });
                }
                _ => {}
            }
        }
        Ok(())
    }
}

/// The outer receipt: the later runner seals after projection and records
/// the actual seal result only here (`P18-DUR` ladder: the trajectory itself
/// is an input to sealing, so it cannot contain the seal result).
#[derive(Debug, Clone)]
pub(crate) struct TrajectoryReceipt {
    /// Digest of the pre-seal trajectory this receipt wraps.
    pub(crate) pre_seal_digest: String,
    /// The actual seal outcome, recorded after sealing by the runner.
    pub(crate) seal_result: Option<SealOutcome>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum SealOutcome {
    Sealed {
        bundle_digest: String,
    },
    #[expect(
        dead_code,
        reason = "required closed post-seal receipt outcome for failed sealing"
    )]
    SealFailed {
        reason: String,
    },
}

impl TrajectoryReceipt {
    /// Wraps a pre-seal trajectory digest.
    pub(crate) fn for_trajectory(trajectory: &ProvisionalTrajectory) -> Self {
        Self {
            pre_seal_digest: trajectory.content_digest(),
            seal_result: None,
        }
    }

    /// Records the actual seal outcome. The only place a seal result lives;
    /// the wrapped trajectory keeps no seal representation.
    pub(crate) fn record_seal(&mut self, outcome: SealOutcome) {
        self.seal_result = Some(outcome);
    }
}

/// Typed projection failure.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub(crate) enum ProjectionError {
    /// The lifecycle has already moved past settlement: the trajectory is a
    /// pre-seal projection input, not a post-seal view.
    #[error("lifecycle is beyond the pre-seal window")]
    LifecycleBeyondPreSeal,
    /// A referenced native artifact changed since settlement.
    #[error("native artifact {role:?} drifted since settlement")]
    ArtifactDrift { role: String },
    /// The final-workspace capture root does not exist.
    #[error("final-workspace capture root is missing")]
    WorkspaceMissing,
    /// A projection input could not be parsed as its native schema.
    #[error("native {role:?} bytes failed the {schema:?} projection parse")]
    NativeParse { role: String, schema: &'static str },
    /// A projected node would claim a fabricated global order.
    #[error("edge {from:?}->{to:?} invents chronology across sources")]
    InventedChronology { from: NodeId, to: NodeId },
    /// A projected fact would claim parity with the other Agent product.
    #[error("fact {fact:?} on node {node:?} claims synthetic parity")]
    SyntheticParity { node: NodeId, fact: String },
    /// A projected node would reference the trajectory's own seal result.
    #[error("node {node:?} is a self-referential seal-result node")]
    SelfReferentialSeal { node: NodeId },
    /// An edge references a node the trajectory does not contain.
    #[error("edge {from:?}->{to:?} dangles")]
    DanglingEdge { from: NodeId, to: NodeId },
    /// The trajectory schema identity drifted.
    #[error("trajectory schema drifted: {found:?}")]
    SchemaDrift { found: String },
    /// The product identity is absent.
    #[error("trajectory has no product identity")]
    MissingProduct,
}

/// The projection pipeline: stateless; every projection reads only its
/// inputs and the native artifact bytes they reference.
pub(crate) struct ProjectionPipeline;

impl ProjectionPipeline {
    /// Project one trial's settled pre-seal facts into a provisional
    /// trajectory. Fails closed on artifact drift, missing captures, and
    /// post-settlement lifecycles.
    pub(crate) fn project(
        inputs: &TrialInputs<'_>,
    ) -> Result<ProvisionalTrajectory, ProjectionError> {
        if !matches!(
            inputs.lifecycle.phase(),
            TrialPhase::Planned
                | TrialPhase::IntentPublished
                | TrialPhase::ProcessEffectPending
                | TrialPhase::Settled
        ) {
            return Err(ProjectionError::LifecycleBeyondPreSeal);
        }
        let mut builder = TrajectoryBuilder::new(&inputs.agent.identity.product);

        let lifecycle_ids = builder.project_lifecycle(inputs.lifecycle.phase());
        let agent_id = builder.project_agent_execution(inputs.agent);
        // The Agent process runs while the trial is effect-pending; the
        // observed execution is what settles it.
        if let Some(&effect_pending) = lifecycle_ids.get(2) {
            builder.push_edge(effect_pending, agent_id, EdgeRelation::Caused);
        }

        match inputs.agent.identity.product.as_str() {
            "opi" => project_opi_events(&mut builder, inputs.agent)?,
            "pi" => project_pi_events(&mut builder, inputs.agent)?,
            // No other product has an admitted event importer yet; its
            // artifacts still project below.
            _ => {}
        }

        builder.project_agent_artifacts(inputs.agent, agent_id);
        let workspace_id = builder.project_workspace(inputs.workspace, agent_id)?;

        if let Some(benchmark) = inputs.benchmark {
            let caused_by = workspace_id.unwrap_or(agent_id);
            builder.project_benchmark(benchmark, caused_by);
        }

        builder.project_spans();

        let trajectory = builder.finish();
        trajectory.validate()?;
        Ok(trajectory)
    }
}

/// One pi message exchange pairing state: start/end nodes and the native
/// timestamps each endpoint carried, when present.
#[derive(Debug, Clone, Copy, Default)]
struct PairedMessage {
    start: Option<NodeId>,
    end: Option<NodeId>,
    start_timestamp_ms: Option<u64>,
    end_timestamp_ms: Option<u64>,
}

/// Incremental trajectory construction shared by every projection step.
struct TrajectoryBuilder {
    product: String,
    nodes: Vec<Node>,
    edges: Vec<Edge>,
    next_id: u32,
    /// Opi evidence call-id correlation, retained for parent edges and spans.
    opi_calls: BTreeMap<u64, NodeId>,
    /// Opi per-turn event node ids, retained for turn spans.
    opi_turns: BTreeMap<u64, Vec<NodeId>>,
    /// pi turn/message pairing state, retained for spans.
    pi_open_turn: Option<NodeId>,
    pi_messages: BTreeMap<String, PairedMessage>,
    pi_event_nodes: Vec<NodeId>,
    agent_side: Vec<NodeId>,
    grader_side: Vec<NodeId>,
    agent_execution: Option<NodeId>,
    events_provenance: Option<Provenance>,
    spans: Vec<Span>,
}

impl TrajectoryBuilder {
    fn new(product: &str) -> Self {
        Self {
            product: product.to_owned(),
            nodes: Vec::new(),
            edges: Vec::new(),
            next_id: 0,
            opi_calls: BTreeMap::new(),
            opi_turns: BTreeMap::new(),
            pi_open_turn: None,
            pi_messages: BTreeMap::new(),
            pi_event_nodes: Vec::new(),
            agent_side: Vec::new(),
            grader_side: Vec::new(),
            agent_execution: None,
            events_provenance: None,
            spans: Vec::new(),
        }
    }

    fn push_node(
        &mut self,
        source: NodeSource,
        kind: NodeKind,
        provenance: Provenance,
        source_order: Option<u64>,
        facts: BTreeMap<String, Fact>,
    ) -> NodeId {
        let id = NodeId(self.next_id);
        self.next_id += 1;
        self.nodes.push(Node {
            id,
            source,
            kind,
            provenance,
            source_order,
            facts,
        });
        id
    }

    fn push_edge(&mut self, from: NodeId, to: NodeId, relation: EdgeRelation) {
        self.edges.push(Edge { from, to, relation });
    }

    /// The pre-seal lifecycle ladder reached so far, as ordered nodes.
    fn project_lifecycle(&mut self, phase: TrialPhase) -> Vec<NodeId> {
        let ladder: [TrialPhase; 4] = [
            TrialPhase::Planned,
            TrialPhase::IntentPublished,
            TrialPhase::ProcessEffectPending,
            TrialPhase::Settled,
        ];
        let reached: Vec<TrialPhase> = ladder
            .into_iter()
            .take_while(|p| *p != phase)
            .chain(std::iter::once(phase))
            .collect();
        let provenance = Provenance {
            artifact_role: "trial/lifecycle".to_owned(),
            artifact_sha256: None,
            rule: "runner-lifecycle/1".to_owned(),
        };
        let mut ids = Vec::new();
        for (index, phase) in reached.into_iter().enumerate() {
            let id = self.push_node(
                NodeSource::Lifecycle,
                NodeKind::Lifecycle { phase },
                provenance.clone(),
                Some(index as u64),
                BTreeMap::new(),
            );
            if let Some(&previous) = ids.last() {
                self.push_edge(previous, id, EdgeRelation::SourceOrder);
            }
            ids.push(id);
            self.agent_side.push(id);
        }
        ids
    }

    /// The supervised Agent execution: exit, wall time, and usage facts.
    fn project_agent_execution(&mut self, agent: &AgentRecord) -> NodeId {
        let provenance = Provenance {
            artifact_role: "native/stdout".to_owned(),
            artifact_sha256: None,
            rule: "agent-execution/1".to_owned(),
        };
        let mut facts = BTreeMap::new();
        if let crate::process::ExitState::Exited { code } = agent.exit {
            facts.insert(
                "exit_code".to_owned(),
                Fact::Known {
                    value: code.unsigned_abs() as u64,
                    origin: "supervisor-exit".to_owned(),
                },
            );
        }
        facts.insert(
            "wall_time_ms".to_owned(),
            Fact::Known {
                value: agent.wall_time.as_millis() as u64,
                origin: "supervisor-wall-clock".to_owned(),
            },
        );
        for (name, fact) in [
            ("input_tokens", &agent.usage.input_tokens),
            ("output_tokens", &agent.usage.output_tokens),
        ] {
            if let Some(fact) = fact {
                facts.insert(name.to_owned(), fact.clone());
            }
        }
        let id = self.push_node(
            NodeSource::AgentExecution,
            NodeKind::AgentExecution,
            provenance,
            None,
            facts,
        );
        self.agent_execution = Some(id);
        self.agent_side.push(id);
        id
    }

    /// Content-addressed native Agent artifact references.
    fn project_agent_artifacts(&mut self, agent: &AgentRecord, agent_id: NodeId) -> Vec<NodeId> {
        let rule = format!("{}-adapter", agent.identity.product);
        let mut ids = Vec::new();
        if let AgentCompletion::Completed { artifacts } = &agent.completion {
            for artifact in artifacts {
                let provenance = Provenance {
                    artifact_role: artifact.role.clone(),
                    artifact_sha256: Some(artifact.sha256.clone()),
                    rule: rule.clone(),
                };
                let id = self.push_node(
                    NodeSource::AgentArtifacts,
                    NodeKind::Artifact {
                        role: artifact.role.clone(),
                    },
                    provenance,
                    None,
                    BTreeMap::new(),
                );
                self.push_edge(agent_id, id, EdgeRelation::Caused);
                self.agent_side.push(id);
                ids.push(id);
            }
        }
        ids
    }

    /// The final-workspace capture: one digest over the captured file set.
    /// Pre-seal by construction; the immutable content-addressed bundle is
    /// sealed after this projection.
    fn project_workspace(
        &mut self,
        workspace: Option<&Path>,
        agent_id: NodeId,
    ) -> Result<Option<NodeId>, ProjectionError> {
        let Some(root) = workspace else {
            return Ok(None);
        };
        if !root.is_dir() {
            return Err(ProjectionError::WorkspaceMissing);
        }
        let mut files: Vec<(String, String, u64)> = Vec::new();
        collect_capture_entries(root, root, &mut files);
        files.sort();
        let mut digest_input = String::new();
        let mut total_bytes: u64 = 0;
        for (relative, file_sha, file_len) in &files {
            digest_input.push_str(relative);
            digest_input.push('\u{0}');
            digest_input.push_str(file_sha);
            digest_input.push('\n');
            total_bytes += file_len;
        }
        let provenance = Provenance {
            artifact_role: "final-workspace/capture".to_owned(),
            artifact_sha256: Some(sha256_hex(digest_input.as_bytes())),
            rule: "workspace-capture/1".to_owned(),
        };
        let mut facts = BTreeMap::new();
        facts.insert(
            "files".to_owned(),
            Fact::Known {
                value: files.len() as u64,
                origin: "workspace-capture".to_owned(),
            },
        );
        facts.insert(
            "bytes".to_owned(),
            Fact::Known {
                value: total_bytes,
                origin: "workspace-capture".to_owned(),
            },
        );
        let id = self.push_node(
            NodeSource::Workspace,
            NodeKind::WorkspaceCapture,
            provenance,
            None,
            facts,
        );
        self.push_edge(agent_id, id, EdgeRelation::Caused);
        self.agent_side.push(id);
        Ok(Some(id))
    }

    /// The verifier execution, the grader verdict, and the grader's native
    /// artifacts, for whichever benchmark revision family settled this run.
    fn project_benchmark(&mut self, record: &BenchmarkRecord, caused_by: NodeId) {
        let verifier_provenance = Provenance {
            artifact_role: "native/verifier".to_owned(),
            artifact_sha256: None,
            rule: format!("{}-verifier", record.identity.adapter),
        };
        let mut verifier_facts = BTreeMap::new();
        if let crate::process::ExitState::Exited { code } = record.exit {
            verifier_facts.insert(
                "exit_code".to_owned(),
                Fact::Known {
                    value: code.unsigned_abs() as u64,
                    origin: "supervisor-exit".to_owned(),
                },
            );
        }
        verifier_facts.insert(
            "wall_time_ms".to_owned(),
            Fact::Known {
                value: record.wall_time.as_millis() as u64,
                origin: "supervisor-wall-clock".to_owned(),
            },
        );
        let verifier_id = self.push_node(
            NodeSource::VerifierExecution,
            NodeKind::VerifierExecution,
            verifier_provenance,
            None,
            verifier_facts,
        );
        // The verifier grades the trial's captured final Agent output.
        self.push_edge(caused_by, verifier_id, EdgeRelation::Caused);

        let grader_provenance = Provenance {
            artifact_role: "native/grader".to_owned(),
            artifact_sha256: None,
            rule: record.identity.adapter.clone(),
        };
        let mut grader_facts = BTreeMap::new();
        grader_facts.insert("reward".to_owned(), record.reward.clone());
        let completion_kind = match &record.completion {
            crate::benchmark::process::BenchmarkCompletion::Verified { metrics, .. } => {
                for (name, fact) in [
                    ("tests", &metrics.tests),
                    ("passed", &metrics.passed),
                    ("failed", &metrics.failed),
                    ("skipped", &metrics.skipped),
                    ("pending", &metrics.pending),
                    ("other", &metrics.other),
                ] {
                    if let Some(fact) = fact {
                        grader_facts.insert(name.to_owned(), fact.clone());
                    }
                }
                "verified"
            }
            crate::benchmark::process::BenchmarkCompletion::Failed(_) => "failed",
        };
        grader_facts.insert(
            "completion".to_owned(),
            Fact::Known {
                value: if completion_kind == "verified" { 1 } else { 0 },
                origin: "native-completion".to_owned(),
            },
        );
        let grader_id = self.push_node(
            NodeSource::Grader,
            NodeKind::Grader {
                family: record.identity.benchmark.clone(),
                revision: record.identity.revision.clone(),
            },
            grader_provenance,
            None,
            grader_facts,
        );
        self.push_edge(verifier_id, grader_id, EdgeRelation::Caused);
        self.grader_side.push(verifier_id);
        self.grader_side.push(grader_id);

        if let crate::benchmark::process::BenchmarkCompletion::Verified { artifacts, .. } =
            &record.completion
        {
            for artifact in artifacts {
                let provenance = Provenance {
                    artifact_role: artifact.role.clone(),
                    artifact_sha256: Some(artifact.sha256.clone()),
                    rule: record.identity.adapter.clone(),
                };
                let id = self.push_node(
                    NodeSource::Grader,
                    NodeKind::Artifact {
                        role: artifact.role.clone(),
                    },
                    provenance,
                    None,
                    BTreeMap::new(),
                );
                self.push_edge(grader_id, id, EdgeRelation::Caused);
                self.grader_side.push(id);
            }
        }
    }

    fn project_spans(&mut self) {
        // The process span covers every agent-side node.
        if let Some(agent_id) = self.agent_execution {
            let covers = self.agent_side.clone();
            let provenance = Provenance {
                artifact_role: "native/stdout".to_owned(),
                artifact_sha256: None,
                rule: "agent-execution/1".to_owned(),
            };
            self.spans.push(Span {
                kind: SpanKind::Process,
                covers,
                provenance,
                facts: BTreeMap::new(),
            });
            let _ = agent_id;
        }
        // Opi native spans: one per call kind plus one per turn.
        if self.events_provenance.is_some() && !self.opi_calls.is_empty() {
            let provenance = self.events_provenance.clone().expect("checked above");
            for (turn, nodes) in &self.opi_turns {
                self.spans.push(Span {
                    kind: SpanKind::Turn,
                    covers: nodes.clone(),
                    provenance: provenance.clone(),
                    facts: BTreeMap::from([(
                        "turn".to_owned(),
                        Fact::Known {
                            value: *turn,
                            origin: "evidence-records".to_owned(),
                        },
                    )]),
                });
            }
            for node in &self.nodes {
                if let NodeKind::AgentEvent { event_type } = &node.kind {
                    let kind = match event_type.as_str() {
                        "provider" => Some(SpanKind::Llm),
                        "tool" => Some(SpanKind::Tool),
                        "retry" => Some(SpanKind::Retry),
                        "compaction" => Some(SpanKind::Compaction),
                        _ => None,
                    };
                    if let Some(kind) = kind {
                        self.spans.push(Span {
                            kind,
                            covers: vec![node.id],
                            provenance: provenance.clone(),
                            facts: BTreeMap::new(),
                        });
                    }
                }
            }
        }
        // pi native spans: turns and message exchanges.
        if self.events_provenance.is_some() && !self.pi_event_nodes.is_empty() {
            let provenance = self.events_provenance.clone().expect("checked above");
            for paired in self.pi_messages.values() {
                let (start, end, start_ts, end_ts) = (
                    paired.start,
                    paired.end,
                    paired.start_timestamp_ms,
                    paired.end_timestamp_ms,
                );
                let mut covers = Vec::new();
                if let Some(start) = start {
                    covers.push(start);
                }
                if let Some(end) = end {
                    covers.push(end);
                }
                if covers.is_empty() {
                    continue;
                }
                let duration = match (start_ts, end_ts) {
                    (Some(start), Some(end)) => Fact::Known {
                        value: end.saturating_sub(start),
                        origin: "session-jsonl".to_owned(),
                    },
                    _ => Fact::Unknown {
                        reason: "native-timing-absent".to_owned(),
                    },
                };
                self.spans.push(Span {
                    kind: SpanKind::Llm,
                    covers,
                    provenance: provenance.clone(),
                    facts: BTreeMap::from([("duration_ms".to_owned(), duration)]),
                });
            }
        }
        // The grader span covers the verifier, verdict, and grader artifacts.
        if !self.grader_side.is_empty() {
            let covers = self.grader_side.clone();
            self.spans.push(Span {
                kind: SpanKind::Grader,
                covers,
                provenance: Provenance {
                    artifact_role: "native/grader".to_owned(),
                    artifact_sha256: None,
                    rule: "benchmark-verifier/1".to_owned(),
                },
                facts: BTreeMap::new(),
            });
        }
    }

    fn finish(self) -> ProvisionalTrajectory {
        ProvisionalTrajectory {
            schema: TRAJECTORY_SCHEMA,
            product: self.product,
            nodes: self.nodes,
            edges: self.edges,
            spans: self.spans,
        }
    }
}

/// Project Opi's native evidence records: one node per line, native
/// `sequence` as per-source order, and native `parent` call correlation as
/// Parent edges (P18-TRJ-003).
fn project_opi_events(
    builder: &mut TrajectoryBuilder,
    agent: &AgentRecord,
) -> Result<(), ProjectionError> {
    let Some(artifact) = completion_artifacts(agent)
        .iter()
        .find(|a| a.role == "evidence/records")
        .cloned()
    else {
        return Ok(());
    };
    let bytes = std::fs::read(&artifact.path).map_err(|_| ProjectionError::ArtifactDrift {
        role: artifact.role.clone(),
    })?;
    if sha256_hex(&bytes) != artifact.sha256 {
        return Err(ProjectionError::ArtifactDrift {
            role: artifact.role.clone(),
        });
    }
    let provenance = Provenance {
        artifact_role: artifact.role.clone(),
        artifact_sha256: Some(artifact.sha256.clone()),
        rule: format!("{}-adapter/1", agent.identity.product),
    };
    builder.events_provenance = Some(provenance.clone());

    let text = std::str::from_utf8(&bytes).map_err(|_| ProjectionError::NativeParse {
        role: artifact.role.clone(),
        schema: "opi-evidence-records/1",
    })?;
    let mut pending_parents: Vec<(u64, NodeId)> = Vec::new();
    for line in text.lines().filter(|l| !l.trim().is_empty()) {
        let value: serde_json::Value =
            serde_json::from_str(line).map_err(|_| ProjectionError::NativeParse {
                role: artifact.role.clone(),
                schema: "opi-evidence-records/1",
            })?;
        let kind = value["kind"].as_str().ok_or(ProjectionError::NativeParse {
            role: artifact.role.clone(),
            schema: "opi-evidence-records/1",
        })?;
        let turn = value["turn"].as_u64().ok_or(ProjectionError::NativeParse {
            role: artifact.role.clone(),
            schema: "opi-evidence-records/1",
        })?;
        let call = value["call"].as_u64().ok_or(ProjectionError::NativeParse {
            role: artifact.role.clone(),
            schema: "opi-evidence-records/1",
        })?;
        let sequence = value["sequence"]
            .as_u64()
            .ok_or(ProjectionError::NativeParse {
                role: artifact.role.clone(),
                schema: "opi-evidence-records/1",
            })?;
        let mut facts = BTreeMap::new();
        for (name, value) in [("turn", turn), ("call", call)] {
            facts.insert(
                name.to_owned(),
                Fact::Known {
                    value,
                    origin: "evidence-records".to_owned(),
                },
            );
        }
        let node_id = builder.push_node(
            NodeSource::AgentEvents,
            NodeKind::AgentEvent {
                event_type: kind.to_owned(),
            },
            provenance.clone(),
            Some(sequence),
            facts,
        );
        builder.opi_calls.insert(call, node_id);
        builder.opi_turns.entry(turn).or_default().push(node_id);
        builder.agent_side.push(node_id);
        if let Some(parent) = value["parent"].as_u64() {
            pending_parents.push((parent, node_id));
        }
    }
    // Parent edges resolve only against observed native call correlation;
    // an unobserved parent id invents chronology and is never fabricated.
    for (parent, child) in pending_parents {
        if let Some(&parent_node) = builder.opi_calls.get(&parent) {
            builder.push_edge(parent_node, child, EdgeRelation::Parent);
        }
    }
    Ok(())
}

/// Project pi's native session stream: one node per line, line index as
/// per-source order, and message pairing only where the native stream pairs
/// it. Timing stays a typed unknown unless the native event carries a
/// timestamp (P18-TRJ-002).
fn project_pi_events(
    builder: &mut TrajectoryBuilder,
    agent: &AgentRecord,
) -> Result<(), ProjectionError> {
    let Some(artifact) = completion_artifacts(agent)
        .iter()
        .find(|a| a.role == "events/stdout")
        .cloned()
    else {
        return Ok(());
    };
    let bytes = std::fs::read(&artifact.path).map_err(|_| ProjectionError::ArtifactDrift {
        role: artifact.role.clone(),
    })?;
    if sha256_hex(&bytes) != artifact.sha256 {
        return Err(ProjectionError::ArtifactDrift {
            role: artifact.role.clone(),
        });
    }
    let provenance = Provenance {
        artifact_role: artifact.role.clone(),
        artifact_sha256: Some(artifact.sha256.clone()),
        rule: format!("{}-adapter/1", agent.identity.product),
    };
    builder.events_provenance = Some(provenance.clone());

    let text = std::str::from_utf8(&bytes).map_err(|_| ProjectionError::NativeParse {
        role: artifact.role.clone(),
        schema: "pi-session-events/1",
    })?;
    let mut turn_nodes: Vec<NodeId> = Vec::new();
    for (index, line) in text.lines().filter(|l| !l.trim().is_empty()).enumerate() {
        let value: serde_json::Value =
            serde_json::from_str(line).map_err(|_| ProjectionError::NativeParse {
                role: artifact.role.clone(),
                schema: "pi-session-events/1",
            })?;
        let event_type = value["type"].as_str().ok_or(ProjectionError::NativeParse {
            role: artifact.role.clone(),
            schema: "pi-session-events/1",
        })?;
        let mut facts = BTreeMap::new();
        let message_timestamp = value["message"]["timestamp"].as_u64();
        if let Some(timestamp) = message_timestamp {
            facts.insert(
                "timestamp_ms".to_owned(),
                Fact::Known {
                    value: timestamp,
                    origin: "session-jsonl".to_owned(),
                },
            );
        }
        let node_id = builder.push_node(
            NodeSource::AgentEvents,
            NodeKind::AgentEvent {
                event_type: event_type.to_owned(),
            },
            provenance.clone(),
            Some(index as u64),
            facts,
        );
        builder.pi_event_nodes.push(node_id);
        builder.agent_side.push(node_id);
        match event_type {
            "turn_start" => {
                builder.pi_open_turn = Some(node_id);
                turn_nodes.clear();
                turn_nodes.push(node_id);
            }
            "turn_end" => {
                turn_nodes.push(node_id);
                let covers = std::mem::take(&mut turn_nodes);
                builder.spans.push(Span {
                    kind: SpanKind::Turn,
                    covers,
                    provenance: provenance.clone(),
                    facts: BTreeMap::new(),
                });
                builder.pi_open_turn = None;
            }
            "message_start" | "message_end" => {
                if let Some(message_id) = value["message"]["id"].as_str() {
                    let entry = builder
                        .pi_messages
                        .entry(message_id.to_owned())
                        .or_default();
                    let timestamp = value["message"]["timestamp"].as_u64();
                    if event_type == "message_start" {
                        entry.start = Some(node_id);
                        entry.start_timestamp_ms = timestamp;
                    } else {
                        entry.end = Some(node_id);
                        entry.end_timestamp_ms = timestamp;
                    }
                }
            }
            _ => {}
        }
        if builder.pi_open_turn.is_some() && event_type != "turn_start" && event_type != "turn_end"
        {
            turn_nodes.push(node_id);
        }
    }
    Ok(())
}

/// The native artifacts a settled Agent completion imported (empty on a
/// failed completion: nothing was imported to project).
fn completion_artifacts(record: &AgentRecord) -> &[NativeArtifact] {
    match &record.completion {
        AgentCompletion::Completed { artifacts } => artifacts,
        AgentCompletion::Failed(_) => &[],
    }
}

/// Collect the capture file set as (relative path, file sha256) pairs.
fn collect_capture_entries(root: &Path, dir: &Path, files: &mut Vec<(String, String, u64)>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_capture_entries(root, &path, files);
        } else if let Ok(bytes) = std::fs::read(&path) {
            let relative = path
                .strip_prefix(root)
                .map(|p| p.to_string_lossy().replace('\\', "/"))
                .unwrap_or_default();
            files.push((relative, sha256_hex(&bytes), bytes.len() as u64));
        }
    }
}

/// Whether a fact origin names the other Agent product (an opi trajectory
/// carrying a `pi-…` origin, or the reverse). Product tokens deliberately
/// include the trailing dash so native grader origins like `pier-report`
/// never match.
fn names_other_product(product: &str, origin: &str) -> bool {
    match product {
        "opi" => origin.starts_with("pi-"),
        "pi" => origin.starts_with("opi-"),
        _ => false,
    }
}

fn facts_json(facts: &BTreeMap<String, Fact>) -> serde_json::Value {
    let map: serde_json::Map<String, serde_json::Value> = facts
        .iter()
        .map(|(name, fact)| {
            let value = match fact {
                Fact::Known { value, origin } => serde_json::json!({
                    "Known": {"value": value, "origin": origin}
                }),
                Fact::Unknown { reason } => serde_json::json!({
                    "Unknown": {"reason": reason}
                }),
            };
            (name.clone(), value)
        })
        .collect();
    serde_json::Value::Object(map)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::opi::sha256_hex;
    use crate::agent::process::{AgentCompletion, AgentIdentity, UsageProjection};
    use crate::process::{CleanupEvidence, ExitState, OutputCapture};
    use std::path::PathBuf;
    use std::time::Duration;

    fn fixture(relative: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(relative)
    }

    fn settled_lifecycle() -> (TrialLifecycle, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let trial = crate::bundle::TrialIdentity::new("trial-1").unwrap();
        let intent = crate::bundle::IntentRecord {
            trial: trial.clone(),
            pair: crate::bundle::PairIdentity::new("pair-1").unwrap(),
            artifacts: vec![crate::bundle::ArtifactKey::new("native/stdout").unwrap()],
            expected_output: crate::bundle::ArtifactKey::new("normalized/expected-output").unwrap(),
        };
        let mut bundle = crate::bundle::RunBundle::create(dir.path()).unwrap();
        let proof = bundle.publish_intent(&intent).unwrap();
        let mut lifecycle = TrialLifecycle::plan(trial);
        lifecycle.publish_intent(intent).unwrap();
        lifecycle.enter_process_effect_pending(proof).unwrap();
        lifecycle
            .settle(crate::runner::lifecycle::ObservedOutcome {
                kind: crate::runner::lifecycle::SettlementKind::Exited { code: 0 },
            })
            .unwrap();
        (lifecycle, dir)
    }

    fn agent_record(product: &str, artifacts: Vec<NativeArtifact>) -> AgentRecord {
        AgentRecord {
            identity: AgentIdentity {
                product: product.to_owned(),
                package: format!("{product}-fixture"),
                adapter: format!("{product}-adapter (fixture-schema/1)"),
                executable: PathBuf::from("/tmp/fake-agent"),
            },
            workspace: PathBuf::from("/tmp/ws"),
            exit: ExitState::Exited { code: 0 },
            stdout: OutputCapture {
                bytes: Vec::new(),
                truncated: false,
            },
            stderr: OutputCapture {
                bytes: Vec::new(),
                truncated: false,
            },
            cleanup: CleanupEvidence::NotRequired,
            wall_time: Duration::from_millis(1200),
            usage: UsageProjection::default(),
            completion: AgentCompletion::Completed { artifacts },
        }
    }

    fn artifact(role: &str, path: PathBuf) -> NativeArtifact {
        let bytes = std::fs::read(&path).expect("fixture bytes readable");
        NativeArtifact {
            role: role.to_owned(),
            sha256: sha256_hex(&bytes),
            path,
        }
    }

    #[test]
    fn projects_pi_native_events_with_turn_and_llm_spans_and_typed_unknown_timing() {
        let stream = artifact(
            "events/stdout",
            fixture("tests/fixtures/trajectories/pi/session.jsonl"),
        );
        let agent = agent_record("pi", vec![stream]);
        let (lifecycle, _intent_dir) = settled_lifecycle();

        let trajectory = ProjectionPipeline::project(&TrialInputs {
            agent: &agent,
            benchmark: None,
            lifecycle: &lifecycle,
            workspace: None,
        })
        .unwrap();
        trajectory.validate().unwrap();
        assert_eq!(trajectory.product, "pi");

        // One node per native session line, in native line order.
        let events: Vec<(&str, Option<u64>, bool)> = trajectory
            .nodes
            .iter()
            .filter_map(|n| match &n.kind {
                NodeKind::AgentEvent { event_type } => Some((
                    event_type.as_str(),
                    n.source_order,
                    n.facts.contains_key("timestamp_ms"),
                )),
                _ => None,
            })
            .collect();
        assert_eq!(
            events,
            vec![
                ("session", Some(0), false),
                ("agent_start", Some(1), false),
                ("turn_start", Some(2), false),
                ("message_start", Some(3), true),
                ("message_end", Some(4), true),
                ("turn_end", Some(5), false),
                ("session_shutdown", Some(6), false),
            ]
        );

        // pi exposes no call correlation: no Parent edge is fabricated.
        assert!(
            !trajectory
                .edges
                .iter()
                .any(|e| e.relation == EdgeRelation::Parent)
        );

        // One native turn span covering turn_start..turn_end.
        let turn_spans: Vec<&Span> = trajectory
            .spans
            .iter()
            .filter(|s| s.kind == SpanKind::Turn)
            .collect();
        assert_eq!(turn_spans.len(), 1);
        assert_eq!(turn_spans[0].covers.len(), 4);

        // One LLM span for the paired message, with native timing retained.
        let llm_spans: Vec<&Span> = trajectory
            .spans
            .iter()
            .filter(|s| s.kind == SpanKind::Llm)
            .collect();
        assert_eq!(llm_spans.len(), 1);
        assert_eq!(
            llm_spans[0].facts.get("duration_ms"),
            Some(&Fact::Known {
                value: 1000,
                origin: "session-jsonl".to_owned(),
            })
        );

        // Without native timestamps the same pairing stays a typed unknown
        // (P18-TRJ-002/004: no invented timing).
        let dir = tempfile::tempdir().unwrap();
        let sparse = dir.path().join("session.jsonl");
        std::fs::write(
            &sparse,
            "{\"type\":\"session\",\"version\":2,\"id\":\"x\"}\n\
             {\"type\":\"turn_start\"}\n\
             {\"type\":\"message_start\",\"message\":{\"id\":\"m-1\",\"role\":\"assistant\"}}\n\
             {\"type\":\"message_end\",\"message\":{\"id\":\"m-1\",\"role\":\"assistant\"}}\n\
             {\"type\":\"turn_end\"}\n",
        )
        .unwrap();
        let sparse_agent = agent_record("pi", vec![artifact("events/stdout", sparse.clone())]);
        let sparse_trajectory = ProjectionPipeline::project(&TrialInputs {
            agent: &sparse_agent,
            benchmark: None,
            lifecycle: &settled_lifecycle().0,
            workspace: None,
        })
        .unwrap();
        let sparse_llm: Vec<&Span> = sparse_trajectory
            .spans
            .iter()
            .filter(|s| s.kind == SpanKind::Llm)
            .collect();
        assert_eq!(sparse_llm.len(), 1);
        assert_eq!(
            sparse_llm[0].facts.get("duration_ms"),
            Some(&Fact::Unknown {
                reason: "native-timing-absent".to_owned(),
            })
        );
    }

    #[test]
    fn projects_grader_and_artifact_nodes_for_every_revision_family() {
        use crate::benchmark::process::{
            BenchmarkCompletion, BenchmarkIdentity, BenchmarkProvenance, NativeMetrics,
        };

        struct Family {
            benchmark: &'static str,
            revision: &'static str,
            adapter: &'static str,
            artifact_role: &'static str,
            fixture: &'static str,
            reward: Fact,
            metrics: NativeMetrics,
        }
        let ctrf_metrics = NativeMetrics {
            tests: Some(Fact::Known {
                value: 6,
                origin: "ctrf-summary".to_owned(),
            }),
            passed: Some(Fact::Known {
                value: 6,
                origin: "ctrf-summary".to_owned(),
            }),
            failed: Some(Fact::Known {
                value: 0,
                origin: "ctrf-summary".to_owned(),
            }),
            skipped: None,
            pending: None,
            other: None,
        };
        let families = [
            Family {
                benchmark: "terminal-bench",
                revision: "2.1",
                adapter: "opi-eval-terminal-bench-21-adapter/1",
                artifact_role: "native/ctrf-report",
                fixture: "tests/fixtures/benchmarks/terminal-bench-2.1/ctrf/ok-six-passed.json",
                reward: Fact::Unknown {
                    reason: "native-reward-pending-18-15-smoke".to_owned(),
                },
                metrics: ctrf_metrics.clone(),
            },
            Family {
                benchmark: "terminal-bench",
                revision: "3.0",
                adapter: "opi-eval-terminal-bench-30-adapter/1",
                artifact_role: "native/ctrf-report",
                fixture: "tests/fixtures/benchmarks/terminal-bench-3.0/ctrf/ok-six-passed.json",
                reward: Fact::Unknown {
                    reason: "native-reward-pending-18-15-smoke".to_owned(),
                },
                metrics: ctrf_metrics,
            },
            Family {
                benchmark: "deepswe",
                revision: "v1.1",
                adapter: "opi-eval-deepswe-adapter/1",
                artifact_role: "native/pier-report",
                fixture: "tests/fixtures/benchmarks/deepswe-v1.1/pier-report/resolved.json",
                reward: Fact::Known {
                    value: 1,
                    origin: "pier-report".to_owned(),
                },
                metrics: NativeMetrics::default(),
            },
        ];

        let evidence = artifact(
            "evidence/records",
            fixture("tests/fixtures/trajectories/opi/evidence.jsonl"),
        );
        let agent = agent_record("opi", vec![evidence]);
        let (lifecycle, _intent_dir) = settled_lifecycle();
        let workspace = tempfile::tempdir().unwrap();
        std::fs::write(workspace.path().join("answer.txt"), b"result\n").unwrap();

        for family in &families {
            let native = artifact(family.artifact_role, fixture(family.fixture));
            let record = crate::benchmark::process::BenchmarkRecord {
                identity: BenchmarkIdentity {
                    benchmark: family.benchmark.to_owned(),
                    revision: family.revision.to_owned(),
                    adapter: family.adapter.to_owned(),
                },
                exit: ExitState::Exited { code: 0 },
                stdout: OutputCapture {
                    bytes: Vec::new(),
                    truncated: false,
                },
                stderr: OutputCapture {
                    bytes: Vec::new(),
                    truncated: false,
                },
                cleanup: CleanupEvidence::NotRequired,
                wall_time: Duration::from_millis(800),
                reward: family.reward.clone(),
                completion: BenchmarkCompletion::Verified {
                    metrics: family.metrics.clone(),
                    artifacts: vec![native],
                },
                provenance: BenchmarkProvenance {
                    admitted_lock_digest: "a".repeat(64),
                    integrity_digest: "b".repeat(64),
                    revision: family.revision.to_owned(),
                    task_id: "fixture-task".to_owned(),
                },
            };

            let trajectory = ProjectionPipeline::project(&TrialInputs {
                agent: &agent,
                benchmark: Some(&record),
                lifecycle: &lifecycle,
                workspace: Some(workspace.path()),
            })
            .unwrap();
            trajectory.validate().unwrap();

            // Verifier execution, grader verdict, and grader artifact nodes.
            let verifier: Vec<&Node> = trajectory
                .nodes
                .iter()
                .filter(|n| matches!(n.kind, NodeKind::VerifierExecution))
                .collect();
            assert_eq!(verifier.len(), 1, "{}", family.adapter);
            assert_eq!(
                verifier[0].facts.get("wall_time_ms"),
                Some(&Fact::Known {
                    value: 800,
                    origin: "supervisor-wall-clock".to_owned(),
                })
            );

            let grader: Vec<&Node> = trajectory
                .nodes
                .iter()
                .filter(|n| {
                    n.kind.tag() == format!("grader/{}/{}", family.benchmark, family.revision)
                })
                .collect();
            assert_eq!(grader.len(), 1, "{}", family.adapter);
            assert_eq!(grader[0].facts.get("reward"), Some(&family.reward));
            assert_eq!(
                grader[0].facts.get("completion"),
                Some(&Fact::Known {
                    value: 1,
                    origin: "native-completion".to_owned(),
                })
            );
            assert_eq!(grader[0].provenance.rule, family.adapter);

            let native_artifact: Vec<&Node> = trajectory
                .nodes
                .iter()
                .filter(|n| n.kind.tag() == format!("artifact/{}", family.artifact_role))
                .collect();
            assert_eq!(native_artifact.len(), 1, "{}", family.adapter);

            // Causal chain: workspace capture -> verifier -> grader -> report.
            let caused: Vec<(String, String)> = trajectory
                .edges
                .iter()
                .filter(|e| e.relation == EdgeRelation::Caused)
                .filter_map(|e| {
                    Some((
                        trajectory.node(e.from)?.kind.tag(),
                        trajectory.node(e.to)?.kind.tag(),
                    ))
                })
                .collect();
            for (from, to) in [
                (
                    "workspace-capture".to_owned(),
                    "verifier-execution".to_owned(),
                ),
                (
                    "verifier-execution".to_owned(),
                    format!("grader/{}/{}", family.benchmark, family.revision),
                ),
                (
                    format!("grader/{}/{}", family.benchmark, family.revision),
                    format!("artifact/{}", family.artifact_role),
                ),
            ] {
                assert!(
                    caused.contains(&(from.clone(), to.clone())),
                    "{from} -> {to}"
                );
            }

            // The grader span covers verifier, verdict, and native report.
            let grader_span: Vec<&Span> = trajectory
                .spans
                .iter()
                .filter(|s| s.kind == SpanKind::Grader)
                .collect();
            assert_eq!(grader_span.len(), 1, "{}", family.adapter);
            assert_eq!(grader_span[0].covers.len(), 3, "{}", family.adapter);
        }
    }

    #[test]
    fn projects_the_pre_seal_lifecycle_ladder_and_workspace_capture() {
        let evidence = artifact(
            "evidence/records",
            fixture("tests/fixtures/trajectories/opi/evidence.jsonl"),
        );
        let agent = agent_record("opi", vec![evidence]);
        let workspace = tempfile::tempdir().unwrap();
        std::fs::write(workspace.path().join("answer.txt"), b"result\n").unwrap();
        std::fs::create_dir_all(workspace.path().join("nested")).unwrap();
        std::fs::write(workspace.path().join("nested/patch.diff"), b"diff\n").unwrap();

        let (lifecycle, _intent_dir) = settled_lifecycle();
        let trajectory = ProjectionPipeline::project(&TrialInputs {
            agent: &agent,
            benchmark: None,
            lifecycle: &lifecycle,
            workspace: Some(workspace.path()),
        })
        .unwrap();
        trajectory.validate().unwrap();

        // The reached pre-seal ladder projects as ordered lifecycle nodes.
        let ladder: Vec<&Node> = trajectory
            .nodes
            .iter()
            .filter(|n| matches!(n.kind, NodeKind::Lifecycle { .. }))
            .collect();
        assert_eq!(ladder.len(), 4);
        let phases: Vec<TrialPhase> = ladder
            .iter()
            .map(|n| match n.kind {
                NodeKind::Lifecycle { phase } => phase,
                _ => unreachable!(),
            })
            .collect();
        assert_eq!(
            phases,
            vec![
                TrialPhase::Planned,
                TrialPhase::IntentPublished,
                TrialPhase::ProcessEffectPending,
                TrialPhase::Settled,
            ]
        );
        let ladder_ids: Vec<NodeId> = ladder.iter().map(|n| n.id).collect();
        let order_edges: Vec<(NodeId, NodeId)> = trajectory
            .edges
            .iter()
            .filter(|e| e.relation == EdgeRelation::SourceOrder)
            .map(|e| (e.from, e.to))
            .collect();
        for pair in ladder_ids.windows(2) {
            assert!(order_edges.contains(&(pair[0], pair[1])));
        }

        // The process phase causes the Agent execution.
        let caused: Vec<(NodeId, NodeId)> = trajectory
            .edges
            .iter()
            .filter(|e| e.relation == EdgeRelation::Caused)
            .map(|e| (e.from, e.to))
            .collect();
        let execution: Vec<NodeId> = trajectory
            .nodes
            .iter()
            .filter(|n| matches!(n.kind, NodeKind::AgentExecution))
            .map(|n| n.id)
            .collect();
        assert_eq!(execution.len(), 1);
        assert!(caused.contains(&(ladder_ids[2], execution[0])));

        // The final-workspace capture projects over the captured file set.
        let capture: Vec<&Node> = trajectory
            .nodes
            .iter()
            .filter(|n| matches!(n.kind, NodeKind::WorkspaceCapture))
            .collect();
        assert_eq!(capture.len(), 1);
        assert_eq!(
            capture[0].facts.get("files"),
            Some(&Fact::Known {
                value: 2,
                origin: "workspace-capture".to_owned(),
            })
        );
        assert!(capture[0].provenance.artifact_sha256.is_some());
        assert!(caused.contains(&(execution[0], capture[0].id)));

        // A fresh planned lifecycle projects only its reached rung.
        let planned = TrialLifecycle::plan(crate::bundle::TrialIdentity::new("t-2").unwrap());
        let planned_trajectory = ProjectionPipeline::project(&TrialInputs {
            agent: &agent,
            benchmark: None,
            lifecycle: &planned,
            workspace: None,
        })
        .unwrap();
        assert_eq!(
            planned_trajectory
                .nodes
                .iter()
                .filter(|n| matches!(n.kind, NodeKind::Lifecycle { .. }))
                .count(),
            1
        );
    }

    #[test]
    fn rejects_invented_chronology_synthetic_parity_and_seal_result_nodes() {
        let evidence = artifact(
            "evidence/records",
            fixture("tests/fixtures/trajectories/opi/evidence.jsonl"),
        );
        let agent = agent_record("opi", vec![evidence]);
        let base = ProjectionPipeline::project(&TrialInputs {
            agent: &agent,
            benchmark: None,
            lifecycle: &settled_lifecycle().0,
            workspace: None,
        })
        .unwrap();

        // A cross-source ordering edge invents chronology (P18-TRJ-003).
        let lifecycle_id = base
            .nodes
            .iter()
            .find(|n| matches!(n.kind, NodeKind::Lifecycle { .. }))
            .unwrap()
            .id;
        let event_id = base
            .nodes
            .iter()
            .find(|n| matches!(n.kind, NodeKind::AgentEvent { .. }))
            .unwrap()
            .id;
        let mut invented = base.clone();
        invented.edges.push(Edge {
            from: lifecycle_id,
            to: event_id,
            relation: EdgeRelation::SourceOrder,
        });
        assert_eq!(
            invented.validate(),
            Err(ProjectionError::InventedChronology {
                from: lifecycle_id,
                to: event_id,
            })
        );

        // A Parent edge outside one event stream invents chronology too.
        let mut invented_parent = base.clone();
        invented_parent.edges.push(Edge {
            from: lifecycle_id,
            to: event_id,
            relation: EdgeRelation::Parent,
        });
        assert!(matches!(
            invented_parent.validate(),
            Err(ProjectionError::InventedChronology { .. })
        ));

        // A fact naming the other Agent product claims synthetic parity.
        let mut parity = base.clone();
        let node = parity
            .nodes
            .iter_mut()
            .find(|n| matches!(n.kind, NodeKind::AgentEvent { .. }))
            .unwrap();
        node.facts.insert(
            "latency_ms".to_owned(),
            Fact::Known {
                value: 7,
                origin: "pi-fabricated".to_owned(),
            },
        );
        assert!(matches!(
            parity.validate(),
            Err(ProjectionError::SyntheticParity { fact, .. }) if fact == "latency_ms"
        ));

        // A sealed lifecycle rung would reference the trajectory's own
        // sealing: self-referential.
        let mut sealed = base.clone();
        let sealed_node = sealed
            .nodes
            .iter_mut()
            .find(|n| matches!(n.kind, NodeKind::Lifecycle { .. }))
            .unwrap();
        sealed_node.kind = NodeKind::Lifecycle {
            phase: TrialPhase::Sealed,
        };
        assert!(matches!(
            sealed.validate(),
            Err(ProjectionError::SelfReferentialSeal { .. })
        ));

        // So does a seal-result artifact node.
        let mut seal_artifact = base.clone();
        seal_artifact.nodes.push(Node {
            id: NodeId(9000),
            source: NodeSource::AgentArtifacts,
            kind: NodeKind::Artifact {
                role: "seal/result".to_owned(),
            },
            provenance: Provenance {
                artifact_role: "seal/result".to_owned(),
                artifact_sha256: None,
                rule: "test/1".to_owned(),
            },
            source_order: None,
            facts: BTreeMap::new(),
        });
        assert!(matches!(
            seal_artifact.validate(),
            Err(ProjectionError::SelfReferentialSeal { .. })
        ));

        // A dangling edge is structurally invalid.
        let mut dangling = base.clone();
        dangling.edges.push(Edge {
            from: NodeId(9000),
            to: event_id,
            relation: EdgeRelation::Caused,
        });
        assert!(matches!(
            dangling.validate(),
            Err(ProjectionError::DanglingEdge { .. })
        ));
    }

    #[test]
    fn the_actual_seal_result_lives_only_in_the_outer_receipt() {
        let evidence = artifact(
            "evidence/records",
            fixture("tests/fixtures/trajectories/opi/evidence.jsonl"),
        );
        let agent = agent_record("opi", vec![evidence]);
        let trajectory = ProjectionPipeline::project(&TrialInputs {
            agent: &agent,
            benchmark: None,
            lifecycle: &settled_lifecycle().0,
            workspace: None,
        })
        .unwrap();
        let digest_before = trajectory.content_digest();
        assert_eq!(digest_before, trajectory.content_digest());
        assert!(
            !trajectory
                .nodes
                .iter()
                .any(|n| n.kind.tag().contains("seal"))
        );

        let mut receipt = TrajectoryReceipt::for_trajectory(&trajectory);
        assert_eq!(receipt.pre_seal_digest, digest_before);
        assert_eq!(receipt.seal_result, None);
        receipt.record_seal(SealOutcome::Sealed {
            bundle_digest: "c".repeat(64),
        });
        assert_eq!(
            receipt.seal_result,
            Some(SealOutcome::Sealed {
                bundle_digest: "c".repeat(64),
            })
        );

        // The trajectory keeps no seal representation and stays valid.
        assert_eq!(trajectory.content_digest(), digest_before);
        trajectory.validate().unwrap();

        // A lifecycle that already sealed cannot be projected pre-seal.
        let dir = tempfile::tempdir().unwrap();
        let mut bundle = crate::bundle::RunBundle::create(dir.path()).unwrap();
        let trial = crate::bundle::TrialIdentity::new("t-3").unwrap();
        let proof = bundle
            .publish_intent(&crate::bundle::IntentRecord {
                trial: trial.clone(),
                pair: crate::bundle::PairIdentity::new("p-1").unwrap(),
                artifacts: vec![crate::bundle::ArtifactKey::new("native/stdout").unwrap()],
                expected_output: crate::bundle::ArtifactKey::new("normalized/expected-output")
                    .unwrap(),
            })
            .unwrap();
        let mut lifecycle = TrialLifecycle::plan(trial);
        lifecycle
            .publish_intent(crate::bundle::IntentRecord {
                trial: crate::bundle::TrialIdentity::new("t-3").unwrap(),
                pair: crate::bundle::PairIdentity::new("p-1").unwrap(),
                artifacts: vec![crate::bundle::ArtifactKey::new("native/stdout").unwrap()],
                expected_output: crate::bundle::ArtifactKey::new("normalized/expected-output")
                    .unwrap(),
            })
            .unwrap();
        lifecycle.enter_process_effect_pending(proof).unwrap();
        lifecycle
            .settle(crate::runner::lifecycle::ObservedOutcome {
                kind: crate::runner::lifecycle::SettlementKind::Exited { code: 0 },
            })
            .unwrap();
        lifecycle.mark_sealed().unwrap();
        assert_eq!(
            ProjectionPipeline::project(&TrialInputs {
                agent: &agent,
                benchmark: None,
                lifecycle: &lifecycle,
                workspace: None,
            })
            .unwrap_err(),
            ProjectionError::LifecycleBeyondPreSeal
        );
    }

    #[test]
    fn projects_opi_native_events_with_parent_edges_and_source_order() {
        let evidence = artifact(
            "evidence/records",
            fixture("tests/fixtures/trajectories/opi/evidence.jsonl"),
        );
        let agent = agent_record("opi", vec![evidence]);
        let (lifecycle, _intent_dir) = settled_lifecycle();

        let trajectory = ProjectionPipeline::project(&TrialInputs {
            agent: &agent,
            benchmark: None,
            lifecycle: &lifecycle,
            workspace: None,
        })
        .unwrap();

        trajectory.validate().unwrap();
        assert_eq!(trajectory.product, "opi");

        // One node per native evidence line, in native sequence order.
        let events: Vec<&Node> = trajectory
            .nodes
            .iter()
            .filter(|n| matches!(n.kind, NodeKind::AgentEvent { .. }))
            .collect();
        assert_eq!(events.len(), 6);
        assert_eq!(
            events.iter().map(|n| n.source_order).collect::<Vec<_>>(),
            vec![Some(1), Some(2), Some(3), Some(4), Some(5), Some(6)]
        );
        let kinds: Vec<&str> = events
            .iter()
            .map(|n| match &n.kind {
                NodeKind::AgentEvent { event_type } => event_type.as_str(),
                _ => unreachable!(),
            })
            .collect();
        assert_eq!(
            kinds,
            vec![
                "provider",
                "tool",
                "retry",
                "provider",
                "compaction",
                "diagnostic"
            ]
        );

        // Native parent correlation becomes Parent edges: call 1 -> calls 2
        // and 3, call 5 -> call 6.
        let parent_edges: Vec<(u32, u32)> = trajectory
            .edges
            .iter()
            .filter(|e| e.relation == EdgeRelation::Parent)
            .map(|e| (e.from.0, e.to.0))
            .collect();
        assert_eq!(parent_edges.len(), 3);

        // Native agent nodes retain their adapter rule (P18-TRJ-001).
        for node in &trajectory.nodes {
            if matches!(
                node.source,
                NodeSource::AgentEvents | NodeSource::AgentArtifacts
            ) {
                assert!(
                    node.provenance.rule.starts_with("opi-adapter"),
                    "rule {} lost its adapter identity",
                    node.provenance.rule
                );
            }
        }
    }
}
