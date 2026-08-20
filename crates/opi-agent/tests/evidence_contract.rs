//! Phase 17 task 17.3 evidence contract.
//!
//! Exercises the Agent Core evidence vocabulary and storage-neutral sink
//! lifecycle as an additive substrate: opaque call-graph identities, versioned
//! health, the four distinguishable failure outcomes, runtime-input binding,
//! measurements (unknown != zero), classified payload-free artifact references,
//! the producer redaction boundary, and the no-op / in-memory adapter
//! conformance. This task owns the P17-FAL-001 evidence public-contract slice
//! — the closed failure classes a caller distinguishes by variant, without
//! string parsing — and defines the types the 17.4/17.6/17.7 consumers depend
//! on. It does not wire evidence into the agent loop.

// opi-phase17-acceptance

use opi_agent::diagnostic::{RedactionMode, Severity};
use opi_agent::evidence::*;
use opi_ai::WireApi;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn digest(s: &str) -> ContentDigest {
    let byte = s.as_bytes().iter().fold(0_u8, |acc, byte| acc ^ byte);
    ContentDigest::from_hex(format!("{byte:02x}").repeat(32)).expect("valid test digest")
}

fn assembly(identity: &str) -> AssemblyIdentity {
    AssemblyIdentity::new(identity).expect("valid test assembly identity")
}

#[test]
fn embedder_assembly_and_policy_facts_use_validated_opaque_identities() {
    let assembly = AssemblyIdentity::new("acme.desktop").expect("valid embedder identity");
    let capability =
        CapabilityIdentity::new("acme.documents.review").expect("valid capability identity");
    let permission =
        PermissionReference::new("acme.permission.review").expect("valid permission reference");
    let scope = PermissionScope::new("document:quarterly-report").expect("valid scope");
    let grant = ScopedGrantReference::new("acme.grant.session-42").expect("valid grant reference");

    assert_eq!(assembly.as_str(), "acme.desktop");
    assert_eq!(capability.as_str(), "acme.documents.review");
    assert!(AssemblyIdentity::new("").is_err());
    assert!(CapabilityIdentity::new(" acme.documents.review").is_err());
    assert!(PermissionReference::new("acme.permission\nreview").is_err());

    let facts = UserPolicyFacts {
        policy_digest: digest("policy"),
        capability: Some(capability),
        permission_ref: Some(permission),
        permission_scope: Some(scope),
        scoped_grant_ref: Some(grant),
    };
    let serialized = serde_json::to_value(facts).expect("policy facts serialize");
    assert_eq!(serialized["capability"], "acme.documents.review");
    assert_eq!(serialized["permission_ref"], "acme.permission.review");
    assert_eq!(serialized["permission_scope"], "document:quarterly-report");
    assert_eq!(serialized["scoped_grant_ref"], "acme.grant.session-42");
}

#[test]
fn opaque_identities_round_trip_through_json_without_bypassing_validation() {
    fn assert_round_trip<T>(identity: T, expected: &str)
    where
        T: serde::Serialize
            + serde::de::DeserializeOwned
            + std::fmt::Debug
            + std::fmt::Display
            + PartialEq,
    {
        let json = serde_json::to_string(&identity).expect("identity serializes");
        let restored: T = serde_json::from_str(&json).expect("valid identity deserializes");
        assert_eq!(restored, identity);
        assert_eq!(restored.to_string(), expected);
    }

    assert_round_trip(
        AssemblyIdentity::new("acme.desktop").unwrap(),
        "acme.desktop",
    );
    assert_round_trip(
        CapabilityIdentity::new("acme.documents.review").unwrap(),
        "acme.documents.review",
    );
    assert_round_trip(
        PolicyReference::new("acme.policy.v1").unwrap(),
        "acme.policy.v1",
    );
    assert_round_trip(
        PermissionReference::new("acme.permission.review").unwrap(),
        "acme.permission.review",
    );
    assert_round_trip(
        PermissionScope::new("document:quarterly-report").unwrap(),
        "document:quarterly-report",
    );
    assert_round_trip(
        ScopedGrantReference::new("acme.grant.session-42").unwrap(),
        "acme.grant.session-42",
    );
}

#[test]
fn opaque_identity_json_rejects_empty_untrimmed_and_control_character_values() {
    macro_rules! assert_invalid_json {
        ($identity:ty) => {
            for invalid in [r#""""#, r#"" padded""#, r#""line\nbreak""#] {
                assert!(
                    serde_json::from_str::<$identity>(invalid).is_err(),
                    "{} accepted invalid JSON identity {invalid:?}",
                    stringify!($identity)
                );
            }
        };
    }

    assert_invalid_json!(AssemblyIdentity);
    assert_invalid_json!(CapabilityIdentity);
    assert_invalid_json!(PolicyReference);
    assert_invalid_json!(PermissionReference);
    assert_invalid_json!(PermissionScope);
    assert_invalid_json!(ScopedGrantReference);
}

#[test]
fn provider_invocation_facts_distinguish_calls_from_standalone_compaction() {
    let applicable = ProviderInvocationFacts::applicable(
        RouteFacts::new(
            RequestedRoute::new("acme", "requested").unwrap(),
            RouteSelection::new("acme", "resolved", WireApi::OpenAiCompletions).unwrap(),
            ActualRoute::wire_unknown("acme", "actual", UnknownReason::NotReported).unwrap(),
        ),
        ProvenanceFacts::from_auth(&opi_ai::auth::AuthProvenance::default()).unwrap(),
    );
    assert!(matches!(
        applicable,
        ProviderInvocationFacts::Applicable { .. }
    ));

    let not_applicable =
        ProviderInvocationFacts::not_applicable(ProviderNotApplicableReason::StandaloneCompaction);
    assert!(matches!(
        not_applicable,
        ProviderInvocationFacts::NotApplicable {
            reason: ProviderNotApplicableReason::StandaloneCompaction,
        }
    ));
    let json = serde_json::to_value(not_applicable).unwrap();
    assert_eq!(json["kind"], "not_applicable");
    assert_eq!(json["reason"], "standalone_compaction");
}

#[test]
fn standalone_compaction_manifest_requires_explicit_not_applicable_provider_facts() {
    let binding = RuntimeInputBinding::direct(digest("standalone"), assembly("opi.product.cli"));
    let mut alloc = IdentityAllocator::new();
    let run = alloc.run_id();
    let turn = alloc.next_turn();
    let call = alloc.next_call();
    let started = EvidenceRecord {
        run,
        turn: Some(turn),
        call,
        parent: None,
        sequence: alloc.next_sequence(),
        kind: CallKind::Compaction,
        payload: EvidencePayload::Compaction(CompactionEvidenceFacts::started(
            CompactionTrigger::Manual,
        )),
    };
    let terminal = EvidenceRecord {
        run,
        turn: Some(turn),
        call,
        parent: None,
        sequence: alloc.next_sequence(),
        kind: CallKind::Compaction,
        payload: EvidencePayload::Compaction(CompactionEvidenceFacts::terminal(
            CompactionTrigger::Manual,
            CompactionOutcome::Succeeded,
        )),
    };
    let records = [started, terminal];
    let mut candidate = candidate_for_observed_lifecycle(binding.clone(), &records, vec![]);
    candidate.provider =
        ProviderInvocationFacts::not_applicable(ProviderNotApplicableReason::StandaloneCompaction);
    candidate.environment.trigger = ExecutionTrigger::Compaction {
        reason: CompactionTrigger::Manual,
    };
    assert!(
        candidate
            .clone()
            .validate(EvidenceRunObservation::new(&binding, &records, &[]))
            .is_ok(),
        "standalone compaction must finalize without fabricated provider/auth facts"
    );

    candidate.provider = ProviderInvocationFacts::applicable(
        RouteFacts::new(
            RequestedRoute::new("mock", "model").unwrap(),
            RouteSelection::new("mock", "model", WireApi::OpenAiResponses).unwrap(),
            ActualRoute::unknown(UnknownReason::NotReported),
        ),
        ProvenanceFacts::from_auth(&opi_ai::auth::AuthProvenance::default()).unwrap(),
    );
    assert!(
        candidate
            .validate(EvidenceRunObservation::new(&binding, &records, &[]))
            .is_err(),
        "compaction-only evidence cannot validate fabricated provider facts"
    );
}

fn standalone_compaction_observation(
    trigger: CompactionTrigger,
) -> (RuntimeInputBinding, IdentityAllocator, [EvidenceRecord; 2]) {
    let binding = RuntimeInputBinding::direct(digest("standalone"), assembly("opi.product.cli"));
    let mut alloc = IdentityAllocator::new();
    let run = alloc.run_id();
    let turn = alloc.next_turn();
    let call = alloc.next_call();
    let started = EvidenceRecord {
        run,
        turn: Some(turn),
        call,
        parent: None,
        sequence: alloc.next_sequence(),
        kind: CallKind::Compaction,
        payload: EvidencePayload::Compaction(CompactionEvidenceFacts::started(trigger)),
    };
    let terminal = EvidenceRecord {
        run,
        turn: Some(turn),
        call,
        parent: None,
        sequence: alloc.next_sequence(),
        kind: CallKind::Compaction,
        payload: EvidencePayload::Compaction(CompactionEvidenceFacts::terminal(
            trigger,
            CompactionOutcome::Succeeded,
        )),
    };
    (binding, alloc, [started, terminal])
}

#[test]
fn standalone_compaction_manifest_rejects_a_mixed_record_graph() {
    let (binding, mut alloc, [started, terminal]) =
        standalone_compaction_observation(CompactionTrigger::Manual);
    let diagnostic = EvidenceRecord {
        run: terminal.run,
        turn: terminal.turn,
        call: alloc.next_call(),
        parent: None,
        sequence: alloc.next_sequence(),
        kind: CallKind::Diagnostic,
        payload: EvidencePayload::Diagnostic(RedactedDiagnostic {
            severity: Severity::Warning,
            code: "standalone_compaction_diagnostic",
        }),
    };
    let records = [started, terminal, diagnostic];
    let mut candidate = candidate_for_observed_lifecycle(binding.clone(), &records, vec![]);
    candidate.provider =
        ProviderInvocationFacts::not_applicable(ProviderNotApplicableReason::StandaloneCompaction);
    candidate.environment.trigger = ExecutionTrigger::Compaction {
        reason: CompactionTrigger::Manual,
    };

    assert!(
        candidate
            .validate(EvidenceRunObservation::new(&binding, &records, &[]))
            .is_err(),
        "standalone compaction cannot contain non-compaction records"
    );
}

#[test]
fn standalone_compaction_manifest_rejects_a_non_compaction_environment_trigger() {
    let (binding, _, records) = standalone_compaction_observation(CompactionTrigger::Manual);
    let mut candidate = candidate_for_observed_lifecycle(binding.clone(), &records, vec![]);
    candidate.environment.trigger = ExecutionTrigger::Invocation;

    assert!(
        candidate
            .validate(EvidenceRunObservation::new(&binding, &records, &[]))
            .is_err(),
        "standalone compaction requires a compaction environment trigger"
    );
}

#[test]
fn standalone_compaction_manifest_rejects_a_mismatched_compaction_reason() {
    let (binding, _, records) = standalone_compaction_observation(CompactionTrigger::Manual);
    let mut candidate = candidate_for_observed_lifecycle(binding.clone(), &records, vec![]);
    candidate.environment.trigger = ExecutionTrigger::Compaction {
        reason: CompactionTrigger::Threshold,
    };

    assert!(
        candidate
            .validate(EvidenceRunObservation::new(&binding, &records, &[]))
            .is_err(),
        "standalone compaction environment and lifecycle triggers must agree"
    );
}

#[test]
fn manifest_rejects_both_provider_kind_payload_inverses_in_mixed_graphs() {
    let binding = RuntimeInputBinding::direct(digest("kind-payload"), assembly("opi.embedder"));
    let mut alloc = IdentityAllocator::new();
    let valid = fresh_record(&mut alloc, CallKind::Provider, sample_provider_payload());

    let mut provider_payload_under_tool_kind = valid.clone();
    provider_payload_under_tool_kind.kind = CallKind::Tool;
    assert!(
        candidate_for_observed_lifecycle(
            binding.clone(),
            std::slice::from_ref(&provider_payload_under_tool_kind),
            vec![],
        )
        .validate(EvidenceRunObservation::new(
            &binding,
            std::slice::from_ref(&provider_payload_under_tool_kind),
            &[],
        ))
        .is_err(),
        "a Provider payload cannot be relabeled as a non-Provider call"
    );

    let malformed_terminal = EvidenceRecord {
        run: valid.run,
        turn: valid.turn,
        call: alloc.next_call(),
        parent: None,
        sequence: alloc.next_sequence(),
        kind: CallKind::Provider,
        payload: EvidencePayload::Digest(digest("wrong-provider-payload")),
    };
    let records = [valid, malformed_terminal];
    assert!(
        candidate_for_observed_lifecycle(binding.clone(), &records, vec![])
            .validate(EvidenceRunObservation::new(&binding, &records, &[]))
            .is_err(),
        "a mixed graph cannot hide a Provider-kind record with a non-Provider payload"
    );
}

#[test]
fn core_sources_do_not_close_over_reference_product_identity_constants() {
    let evidence_source = include_str!("../src/evidence.rs");
    let authority_source = include_str!("../src/authority.rs");
    let tool_source = include_str!("../src/tool.rs");
    let agent_loop_source = include_str!("../src/agent_loop.rs");
    let core_sources =
        format!("{evidence_source}\n{authority_source}\n{tool_source}\n{agent_loop_source}");

    for forbidden in [
        "pub enum AssemblySource",
        "AssemblySource::Cli",
        "AssemblySource::Sdk",
        "AssemblySource::Rpc",
        "CapabilityClass",
        "Capability::Builtin",
        "WorkspaceRead",
        "WorkspaceWrite",
        "CommandExecute",
    ] {
        assert!(
            !core_sources.contains(forbidden),
            "Agent Core must not define Reference Product identity constant {forbidden:?}"
        );
    }
}

fn artifact() -> ArtifactReference {
    ArtifactReference {
        role: ArtifactRole::ToolInput,
        media_type: MediaType::new("application/json"),
        content_digest: digest("art"),
        location: ArtifactLocation::new("memory://art"),
        sensitivity: SensitivityClassification::Public,
        finalization: FinalizationState::Finalized,
    }
}

fn sample_provider_facts() -> ProviderEvidenceFacts {
    ProviderEvidenceFacts {
        route: RouteFacts::new(
            RequestedRoute::new("mock", "model").unwrap(),
            RouteSelection::new("mock", "model", WireApi::OpenAiResponses).unwrap(),
            ActualRoute::wire_unknown("mock", "model", UnknownReason::NotReported).unwrap(),
        ),
        provenance: ProvenanceFacts::from_auth(&opi_ai::auth::AuthProvenance::default()).unwrap(),
    }
}

fn sample_provider_payload() -> EvidencePayload {
    EvidencePayload::Provider(sample_provider_facts())
}

fn fresh_record(
    alloc: &mut IdentityAllocator,
    kind: CallKind,
    payload: EvidencePayload,
) -> EvidenceRecord {
    let payload = if kind == CallKind::Provider && matches!(payload, EvidencePayload::Digest(_)) {
        sample_provider_payload()
    } else {
        payload
    };
    EvidenceRecord {
        run: alloc.run_id(),
        turn: Some(alloc.next_turn()),
        call: alloc.next_call(),
        parent: None,
        sequence: alloc.next_sequence(),
        kind,
        payload,
    }
}

fn sample_manifest() -> FinalizedManifest {
    validate_with_candidate_observation(sample_manifest_candidate(
        SessionBinding::branch("main").unwrap(),
    ))
    .expect("sample manifest is complete")
}

fn validate_with_candidate_observation(
    candidate: ManifestCandidate,
) -> Result<FinalizedManifest, EvidenceError> {
    let payload = match &candidate.provider {
        ProviderInvocationFacts::Applicable { route, provenance } => {
            EvidencePayload::Provider(ProviderEvidenceFacts {
                route: route.clone(),
                provenance: provenance.as_ref().clone(),
            })
        }
        ProviderInvocationFacts::NotApplicable { .. } => {
            EvidencePayload::Compaction(CompactionEvidenceFacts::started(CompactionTrigger::Manual))
        }
    };
    let record = EvidenceRecord {
        run: candidate.correlation.run,
        turn: candidate.correlation.turn,
        call: candidate
            .correlation
            .call
            .expect("test candidate carries a terminal call"),
        parent: candidate.correlation.parent,
        sequence: candidate.correlation.sequence,
        kind: if matches!(&payload, EvidencePayload::Compaction(_)) {
            CallKind::Compaction
        } else {
            CallKind::Provider
        },
        payload,
    };
    let binding = candidate.binding.clone();
    let artifacts = candidate.artifacts.clone();
    candidate.validate(EvidenceRunObservation::new(
        &binding,
        std::slice::from_ref(&record),
        &artifacts,
    ))
}

fn manifest_for_observed_lifecycle(
    binding: RuntimeInputBinding,
    records: &[EvidenceRecord],
    artifacts: Vec<ArtifactReference>,
) -> FinalizedManifest {
    candidate_for_observed_lifecycle(binding.clone(), records, artifacts.clone())
        .validate(EvidenceRunObservation::new(&binding, records, &artifacts))
        .expect("observed manifest validates")
}

fn candidate_for_observed_lifecycle(
    binding: RuntimeInputBinding,
    records: &[EvidenceRecord],
    artifacts: Vec<ArtifactReference>,
) -> ManifestCandidate {
    let last = records.last().expect("observed lifecycle has a record");
    let mut candidate = sample_manifest_candidate(SessionBinding::NoSession);
    candidate.binding = binding;
    candidate.correlation = ManifestCorrelation {
        run: records[0].run,
        turn: last.turn,
        call: Some(last.call),
        parent: last.parent,
        sequence: last.sequence,
    };
    if let Some(facts) = records
        .iter()
        .rev()
        .find_map(|record| match &record.payload {
            EvidencePayload::Provider(facts) => Some(facts),
            _ => None,
        })
    {
        candidate.provider =
            ProviderInvocationFacts::applicable(facts.route.clone(), facts.provenance.clone());
    } else if records
        .iter()
        .all(|record| record.kind == CallKind::Compaction)
    {
        candidate.provider = ProviderInvocationFacts::not_applicable(
            ProviderNotApplicableReason::StandaloneCompaction,
        );
        if let Some(trigger) = records.iter().find_map(|record| match &record.payload {
            EvidencePayload::Compaction(facts) => Some(facts.trigger()),
            _ => None,
        }) {
            candidate.environment.trigger = ExecutionTrigger::Compaction { reason: trigger };
        }
    }
    candidate.artifacts = artifacts;
    candidate
}

fn validate_record_graph(records: &[EvidenceRecord]) -> Result<FinalizedManifest, EvidenceError> {
    let binding = RuntimeInputBinding::direct(digest("graph"), assembly("opi.embedder"));
    candidate_for_observed_lifecycle(binding.clone(), records, vec![])
        .validate(EvidenceRunObservation::new(&binding, records, &[]))
}

fn repeated_record(
    first: &EvidenceRecord,
    sequence: Sequence,
    payload: &'static str,
) -> EvidenceRecord {
    EvidenceRecord {
        run: first.run,
        turn: first.turn,
        call: first.call,
        parent: first.parent,
        sequence,
        kind: first.kind,
        payload: if first.kind == CallKind::Provider {
            sample_provider_payload()
        } else {
            EvidencePayload::Digest(digest(payload))
        },
    }
}

// ===========================================================================
// Identities (P17-EVD-001)
// ===========================================================================

#[test]
fn run_ids_are_unique_across_allocators() {
    let a = IdentityAllocator::new().run_id();
    let b = IdentityAllocator::new().run_id();
    let c = IdentityAllocator::new().run_id();
    assert_ne!(a, b);
    assert_ne!(b, c);
    assert_ne!(a, c);
}

#[test]
fn call_ids_and_sequence_are_unique_and_monotonic_within_run() {
    let mut alloc = IdentityAllocator::new();
    let c1 = alloc.next_call();
    let c2 = alloc.next_call();
    assert_ne!(c1, c2);
    let s1 = alloc.next_sequence();
    let s2 = alloc.next_sequence();
    let s3 = alloc.next_sequence();
    assert!(s1 < s2);
    assert!(s2 < s3);
}

#[test]
fn parent_call_link_correlates_retry_to_origin() {
    let mut alloc = IdentityAllocator::new();
    let origin = alloc.next_call();
    let retry = alloc.next_call();
    let record = EvidenceRecord {
        run: alloc.run_id(),
        turn: None,
        call: retry,
        parent: Some(origin),
        sequence: alloc.next_sequence(),
        kind: CallKind::Retry,
        payload: EvidencePayload::Digest(digest("retry")),
    };
    assert_eq!(record.parent, Some(origin));
    assert_eq!(record.kind, CallKind::Retry);
    assert_ne!(record.parent.unwrap(), record.call);
}

// ===========================================================================
// Versioned health
// ===========================================================================

#[test]
fn health_starts_healthy_at_initial_generation() {
    let h = EvidenceHealth::healthy();
    assert!(h.is_healthy());
    assert_eq!(h.generation(), EvidenceGeneration::INITIAL);
    assert!(matches!(
        h,
        EvidenceHealth::Healthy { generation } if generation == EvidenceGeneration::INITIAL
    ));
}

#[test]
fn advance_on_failure_makes_incomplete_and_advances_generation() {
    let mut h = EvidenceHealth::healthy();
    h.advance_on_failure(EvidenceFailureCode::Emission);
    assert!(!h.is_healthy());
    match h {
        EvidenceHealth::Incomplete {
            generation,
            first_failure_code,
        } => {
            assert_eq!(generation, EvidenceGeneration::INITIAL.next());
            assert_eq!(first_failure_code, EvidenceFailureCode::Emission);
        }
        other => panic!("expected Incomplete, got {other:?}"),
    }
}

#[test]
fn first_failure_code_is_sticky_while_generation_advances() {
    let mut h = EvidenceHealth::healthy();
    h.advance_on_failure(EvidenceFailureCode::Setup);
    h.advance_on_failure(EvidenceFailureCode::Emission);
    h.advance_on_failure(EvidenceFailureCode::Finalization);
    match h {
        EvidenceHealth::Incomplete {
            generation,
            first_failure_code,
        } => {
            assert_eq!(
                first_failure_code,
                EvidenceFailureCode::Setup,
                "first failure code is sticky"
            );
            assert_eq!(generation, EvidenceGeneration::INITIAL.next().next().next());
        }
        other => panic!("got {other:?}"),
    }
}

#[test]
fn health_is_a_copy_not_a_shared_mutable_handle() {
    // Authorizers receive a copy in each request; advancing a copy must not
    // mutate the original the loop owns (no shared mutable health handle).
    let original = EvidenceHealth::healthy();
    let mut authorizer_copy = original.clone();
    authorizer_copy.advance_on_failure(EvidenceFailureCode::Emission);
    assert!(!authorizer_copy.is_healthy());
    assert!(
        original.is_healthy(),
        "original health must be unchanged by a copied advance"
    );
}

// ===========================================================================
// Typed failure outcomes (P17-FAL-001 evidence slice)
// ===========================================================================

#[test]
fn evidence_failure_outcomes_are_distinguishable_without_strings() {
    let setup = EvidenceError::Setup {
        detail: "x".to_owned(),
    };
    let emission = EvidenceError::Emission {
        detail: "x".to_owned(),
    };
    let finalization = EvidenceError::Finalization {
        detail: "x".to_owned(),
    };
    let incomplete = EvidenceHealth::Incomplete {
        generation: EvidenceGeneration::INITIAL.next(),
        first_failure_code: EvidenceFailureCode::Emission,
    };

    fn classify(err: &EvidenceError) -> &'static str {
        match err {
            EvidenceError::Setup { .. } => "setup",
            EvidenceError::Emission { .. } => "emission",
            EvidenceError::Finalization { .. } => "finalization",
        }
    }
    assert_eq!(classify(&setup), "setup");
    assert_eq!(classify(&emission), "emission");
    assert_eq!(classify(&finalization), "finalization");
    assert_ne!(setup, emission);
    assert_ne!(emission, finalization);
    assert_ne!(setup, finalization);
    assert!(matches!(incomplete, EvidenceHealth::Incomplete { .. }));
    assert!(!incomplete.is_healthy());
}

#[test]
fn evidence_error_maps_to_failure_code() {
    assert_eq!(
        EvidenceError::Setup {
            detail: "x".to_owned()
        }
        .failure_code(),
        EvidenceFailureCode::Setup
    );
    assert_eq!(
        EvidenceError::Emission {
            detail: "x".to_owned()
        }
        .failure_code(),
        EvidenceFailureCode::Emission
    );
    assert_eq!(
        EvidenceError::Finalization {
            detail: "x".to_owned()
        }
        .failure_code(),
        EvidenceFailureCode::Finalization
    );
}

// ===========================================================================
// Runtime-input binding (P17-EVD-003)
// ===========================================================================

#[test]
fn direct_constructor_yields_direct_runtime_input() {
    let b = RuntimeInputBinding::direct(digest("inputs"), assembly("opi.cli"));
    assert!(b.is_direct());
    assert!(matches!(b, RuntimeInputBinding::DirectRuntimeInput { .. }));
}

#[test]
fn binding_variants_are_distinguishable_and_not_normalizable() {
    let direct = RuntimeInputBinding::direct(digest("d"), assembly("opi.sdk"));
    let snapshot = RuntimeInputBinding::ActiveSnapshot {
        snapshot_ref: SnapshotRef::new("future-promotion-snapshot"),
    };
    assert!(direct.is_direct());
    assert!(!snapshot.is_direct());
    fn kind(b: &RuntimeInputBinding) -> u8 {
        match b {
            RuntimeInputBinding::DirectRuntimeInput { .. } => 0,
            RuntimeInputBinding::ActiveSnapshot { .. } => 1,
        }
    }
    assert_ne!(kind(&direct), kind(&snapshot));
}

#[test]
fn direct_run_never_fabricates_active_snapshot() {
    // The only binding constructor is direct(); ActiveSnapshot is reserved for
    // a future trusted Promotion Controller and is never produced for a run.
    for source in [
        assembly("opi.cli"),
        assembly("opi.sdk"),
        assembly("opi.rpc"),
    ] {
        let b = RuntimeInputBinding::direct(digest("x"), source.clone());
        assert!(
            b.is_direct(),
            "direct() must never fabricate ActiveSnapshot for {source:?}"
        );
    }
}

// ===========================================================================
// Measurements (P17-EVD-004)
// ===========================================================================

#[test]
fn measured_zero_is_not_unknown() {
    let zero = Measurement::Known {
        value: 0,
        origin: MeasurementOrigin::ProviderReported,
    };
    let unknown = Measurement::Unknown {
        reason: UnknownReason::NotReported,
    };
    assert!(!zero.is_unknown(), "a measured zero is known");
    assert!(unknown.is_unknown());
    assert_ne!(zero, unknown);
}

#[test]
fn measurement_origins_are_distinct() {
    let origins = [
        MeasurementOrigin::ProviderReported,
        MeasurementOrigin::Estimated,
        MeasurementOrigin::Quota,
        MeasurementOrigin::Billed,
    ];
    let rendered: Vec<String> = origins
        .iter()
        .map(|o| serde_json::to_string(o).unwrap())
        .collect();
    let unique: std::collections::BTreeSet<&String> = rendered.iter().collect();
    assert_eq!(
        unique.len(),
        origins.len(),
        "measurement origins must be distinct: {rendered:?}"
    );
}

#[test]
fn unknown_measurement_serializes_with_reason_not_zero() {
    let unknown = Measurement::Unknown {
        reason: UnknownReason::Withheld,
    };
    let s = serde_json::to_string(&unknown).unwrap().to_lowercase();
    assert!(
        s.contains("unknown"),
        "serialized unknown must carry the unknown variant: {s}"
    );
    assert!(
        s.contains("withheld"),
        "serialized unknown must carry its reason: {s}"
    );
    assert!(
        !s.contains("\"value\":0"),
        "unknown must not be converted to a zero value: {s}"
    );
}

// ===========================================================================
// Classified artifact references (P17-EVD-005)
// ===========================================================================

#[test]
fn artifact_reference_carries_no_payload() {
    let art = ArtifactReference {
        role: ArtifactRole::Prompt,
        media_type: MediaType::new("text/plain"),
        content_digest: digest("prompt-digest"),
        location: ArtifactLocation::new("memory://prompt"),
        sensitivity: SensitivityClassification::Sensitive,
        finalization: FinalizationState::Finalized,
    };
    let s = serde_json::to_string(&art).unwrap();
    assert!(s.contains("content_digest"), "carries a content digest");
    assert!(
        !s.contains("\"payload\""),
        "artifact reference must not embed a payload: {s}"
    );
    assert!(
        !s.contains("\"content\""),
        "artifact reference must not embed raw content: {s}"
    );
    let secret = ArtifactReference {
        role: ArtifactRole::ProviderBody,
        media_type: MediaType::new("application/json"),
        content_digest: digest("secret"),
        location: ArtifactLocation::new("redacted"),
        sensitivity: SensitivityClassification::Secret,
        finalization: FinalizationState::Finalized,
    };
    assert_eq!(secret.sensitivity, SensitivityClassification::Secret);
}

// ===========================================================================
// Redaction boundary (P17-EVD-005 / P17-EVD-011)
// ===========================================================================

#[test]
fn structured_payload_is_redacted_at_the_producer_boundary() {
    // The only RedactedValue constructor applies redaction; raw prompt content
    // is scrubbed before it can cross into the sink contract.
    let raw = serde_json::json!({ "prompt": "super-secret-user-prompt-XYZ" });
    let redacted = RedactedValue::redacted(raw, RedactionMode::Summary);
    let recorded = serde_json::to_string(redacted.as_value()).unwrap();
    assert!(
        !recorded.contains("super-secret-user-prompt-XYZ"),
        "raw secret-bearing prompt content must not cross into the sink contract: {recorded}"
    );
    assert!(
        recorded.contains("REDACTED") || recorded.contains("redacted"),
        "content-sensitive field must be scrubbed: {recorded}"
    );
}

#[test]
fn evidence_record_payload_channels_are_all_typed() {
    let mut alloc = IdentityAllocator::new();
    let channels = [
        EvidencePayload::Digest(digest("route")),
        EvidencePayload::Diagnostic(RedactedDiagnostic {
            severity: Severity::Warning,
            code: "evidence_emission_failed",
        }),
        EvidencePayload::Artifact(ArtifactReference {
            role: ArtifactRole::ToolResult,
            media_type: MediaType::new("application/json"),
            content_digest: digest("tool-out"),
            location: ArtifactLocation::new("memory://tool"),
            sensitivity: SensitivityClassification::Sensitive,
            finalization: FinalizationState::Finalized,
        }),
        EvidencePayload::Structured(RedactedValue::redacted(
            serde_json::json!({ "command": "rm -rf /" }),
            RedactionMode::Summary,
        )),
    ];
    for payload in channels {
        let rec = fresh_record(&mut alloc, CallKind::Provider, payload);
        let s = serde_json::to_string(&rec).unwrap();
        // No raw user-content channel exists; command content was redacted.
        assert!(
            !s.contains("rm -rf /"),
            "raw command content must not cross into the sink: {s}"
        );
    }
}

// ===========================================================================
// Finalized manifest immutability (P17-EVD-003)
// ===========================================================================

#[test]
fn finalized_manifest_serializes_with_direct_binding() {
    let manifest = sample_manifest();
    // Immutability is a type-level guarantee — finalize_run borrows
    // &FinalizedManifest with no &mut surface (evidence.rs) — not something a
    // runtime assertion can prove for a plain struct, so it is not asserted
    // here. This test verifies the manifest serializes to its completeness
    // field and that a direct run binds DirectRuntimeInput (never
    // ActiveSnapshot).
    let json = serde_json::to_value(&manifest).unwrap();
    assert!(json.get("completeness").is_some());
    assert!(manifest.binding.is_direct());
}

#[test]
fn content_digest_rejects_noncanonical_sha256_hex() {
    for invalid in [
        "",
        "abcd",
        "g000000000000000000000000000000000000000000000000000000000000000",
        "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA",
        "00000000000000000000000000000000000000000000000000000000000000000",
    ] {
        assert!(
            ContentDigest::from_hex(invalid).is_err(),
            "accepted invalid SHA-256 digest: {invalid}"
        );
    }
}

#[test]
fn strict_manifest_rejects_incomplete_and_pending_artifacts() {
    let mut incomplete = sample_manifest_candidate(SessionBinding::NoSession);
    incomplete.completeness = EvidenceCompleteness::Incomplete;
    assert!(validate_with_candidate_observation(incomplete).is_err());

    let mut pending = sample_manifest_candidate(SessionBinding::NoSession);
    let mut pending_artifact = artifact();
    pending_artifact.finalization = FinalizationState::Pending;
    pending.artifacts.push(pending_artifact);
    assert!(validate_with_candidate_observation(pending).is_err());
}

#[test]
fn manifest_validation_requires_matching_lifecycle_observation() {
    let setup_binding = RuntimeInputBinding::direct(digest("setup"), assembly("opi.cli"));
    let mut alloc = IdentityAllocator::new();
    let record = fresh_record(
        &mut alloc,
        CallKind::Provider,
        EvidencePayload::Digest(digest("record")),
    );
    let mut candidate = sample_manifest_candidate(SessionBinding::NoSession);
    candidate.binding = RuntimeInputBinding::direct(digest("other"), assembly("opi.cli"));
    candidate.correlation = ManifestCorrelation {
        run: record.run,
        turn: record.turn,
        call: Some(record.call),
        parent: record.parent,
        sequence: record.sequence,
    };

    assert!(
        candidate
            .validate(EvidenceRunObservation::new(
                &setup_binding,
                std::slice::from_ref(&record),
                &[],
            ))
            .is_err()
    );
}

#[test]
fn repeated_call_records_with_stable_metadata_are_valid() {
    let mut alloc = IdentityAllocator::new();
    let first = fresh_record(
        &mut alloc,
        CallKind::Provider,
        EvidencePayload::Digest(digest("prepared")),
    );
    let terminal = repeated_record(&first, alloc.next_sequence(), "terminal");

    assert!(validate_record_graph(&[first, terminal]).is_ok());
}

#[test]
fn repeated_call_record_cannot_self_parent() {
    let mut alloc = IdentityAllocator::new();
    let first = fresh_record(
        &mut alloc,
        CallKind::Provider,
        EvidencePayload::Digest(digest("prepared")),
    );
    let mut terminal = repeated_record(&first, alloc.next_sequence(), "terminal");
    terminal.parent = Some(terminal.call);

    assert!(validate_record_graph(&[first, terminal]).is_err());
}

#[test]
fn repeated_call_record_cannot_change_kind() {
    let mut alloc = IdentityAllocator::new();
    let first = fresh_record(
        &mut alloc,
        CallKind::Provider,
        EvidencePayload::Digest(digest("prepared")),
    );
    let mut terminal = repeated_record(&first, alloc.next_sequence(), "terminal");
    terminal.kind = CallKind::Tool;

    assert!(validate_record_graph(&[first, terminal]).is_err());
}

#[test]
fn repeated_call_record_cannot_change_turn() {
    let mut alloc = IdentityAllocator::new();
    let first = fresh_record(
        &mut alloc,
        CallKind::Provider,
        EvidencePayload::Digest(digest("prepared")),
    );
    let mut terminal = repeated_record(&first, alloc.next_sequence(), "terminal");
    terminal.turn = Some(alloc.next_turn());

    assert!(validate_record_graph(&[first, terminal]).is_err());
}

#[test]
fn repeated_call_record_cannot_change_parent() {
    let mut alloc = IdentityAllocator::new();
    let origin = fresh_record(
        &mut alloc,
        CallKind::Provider,
        EvidencePayload::Digest(digest("origin")),
    );
    let child_call = alloc.next_call();
    let child = EvidenceRecord {
        run: origin.run,
        turn: origin.turn,
        call: child_call,
        parent: Some(origin.call),
        sequence: alloc.next_sequence(),
        kind: CallKind::Tool,
        payload: EvidencePayload::Digest(digest("authorization")),
    };
    let mut terminal = repeated_record(&child, alloc.next_sequence(), "outcome");
    terminal.parent = None;

    assert!(validate_record_graph(&[origin, child, terminal]).is_err());
}

#[test]
fn compaction_graph_requires_one_start_followed_by_one_same_call_terminal() {
    let mut alloc = IdentityAllocator::new();
    let run = alloc.run_id();
    let call = alloc.next_call();
    let start = EvidenceRecord {
        run,
        turn: None,
        call,
        parent: None,
        sequence: alloc.next_sequence(),
        kind: CallKind::Compaction,
        payload: EvidencePayload::Compaction(CompactionEvidenceFacts::started(
            CompactionTrigger::Threshold,
        )),
    };
    let terminal = EvidenceRecord {
        run,
        turn: None,
        call,
        parent: None,
        sequence: alloc.next_sequence(),
        kind: CallKind::Compaction,
        payload: EvidencePayload::Compaction(CompactionEvidenceFacts::terminal(
            CompactionTrigger::Threshold,
            CompactionOutcome::Failed,
        )),
    };

    assert!(
        validate_record_graph(std::slice::from_ref(&start)).is_err(),
        "an unmatched compaction start cannot enter a finalized graph"
    );
    assert!(
        validate_record_graph(std::slice::from_ref(&terminal)).is_err(),
        "a terminal compaction without a start cannot enter a finalized graph"
    );
    assert!(validate_record_graph(&[start.clone(), terminal.clone()]).is_ok());
    assert!(
        validate_record_graph(&[start, terminal.clone(), terminal]).is_err(),
        "a compaction call has exactly one terminal outcome"
    );
}

// ===========================================================================
// Sink lifecycle and adapters (P17-EVD-008 / P17-EVD-010 / P17-EVD-011)
// ===========================================================================

#[test]
fn noop_sink_is_default_and_captures_nothing() {
    let sink = NoopEvidenceSink::new();
    let binding = RuntimeInputBinding::direct(digest("i"), assembly("opi.cli"));
    assert!(sink.setup(&binding).is_ok());
    let mut alloc = IdentityAllocator::new();
    let rec = fresh_record(
        &mut alloc,
        CallKind::Provider,
        EvidencePayload::Digest(digest("d")),
    );
    assert!(sink.emit(&rec).is_ok());
    assert!(sink.finalize_artifact(&artifact()).is_ok());
    assert!(sink.finalize_run(&sample_manifest()).is_ok());
}

#[test]
fn in_memory_sink_records_lifecycle_in_order() {
    let sink = InMemoryEvidenceSink::new();
    let binding = RuntimeInputBinding::direct(digest("i"), assembly("opi.cli"));
    sink.setup(&binding).unwrap();
    let mut alloc = IdentityAllocator::new();
    let r1 = fresh_record(
        &mut alloc,
        CallKind::Provider,
        EvidencePayload::Digest(digest("1")),
    );
    let r2 = fresh_record(
        &mut alloc,
        CallKind::Diagnostic,
        EvidencePayload::Digest(digest("2")),
    );
    let compaction_call = alloc.next_call();
    let r3 = EvidenceRecord {
        run: alloc.run_id(),
        turn: None,
        call: compaction_call,
        parent: None,
        sequence: alloc.next_sequence(),
        kind: CallKind::Compaction,
        payload: EvidencePayload::Compaction(CompactionEvidenceFacts::started(
            CompactionTrigger::Threshold,
        )),
    };
    let r4 = EvidenceRecord {
        run: alloc.run_id(),
        turn: None,
        call: compaction_call,
        parent: None,
        sequence: alloc.next_sequence(),
        kind: CallKind::Compaction,
        payload: EvidencePayload::Compaction(CompactionEvidenceFacts::terminal(
            CompactionTrigger::Threshold,
            CompactionOutcome::Succeeded,
        )),
    };
    sink.emit(&r1).unwrap();
    sink.emit(&r2).unwrap();
    sink.emit(&r3).unwrap();
    sink.emit(&r4).unwrap();
    let finalized_artifact = artifact();
    sink.finalize_artifact(&finalized_artifact).unwrap();
    let manifest = manifest_for_observed_lifecycle(
        binding,
        &[r1.clone(), r2.clone(), r3.clone(), r4.clone()],
        vec![finalized_artifact],
    );
    sink.finalize_run(&manifest).unwrap();

    let records = sink.records();
    assert_eq!(records.len(), 4, "four records recorded in order");
    assert_eq!(records[0].sequence, r1.sequence);
    assert_eq!(records[3].sequence, r4.sequence);
    assert_eq!(sink.artifacts().len(), 1);
    assert!(!sink.has_failure());
    assert!(
        sink.completed_manifest().is_some(),
        "a clean run yields a completed manifest"
    );
}

#[test]
fn in_memory_sink_rejects_kind_payload_mismatch_during_emission() {
    let sink = InMemoryEvidenceSink::new();
    let binding = RuntimeInputBinding::direct(digest("emit-kind"), assembly("opi.embedder"));
    sink.setup(&binding).unwrap();
    let mut alloc = IdentityAllocator::new();
    let mut malformed = fresh_record(&mut alloc, CallKind::Provider, sample_provider_payload());
    malformed.kind = CallKind::Tool;

    assert!(matches!(
        sink.emit(&malformed),
        Err(EvidenceError::Emission { .. })
    ));
    assert!(sink.has_failure());
    assert!(
        sink.records().is_empty(),
        "a malformed record must not be accepted into the adapter"
    );
}

#[test]
fn in_memory_sink_rejects_manifest_binding_or_run_mismatch() {
    let setup_binding = RuntimeInputBinding::direct(digest("setup"), assembly("opi.cli"));
    let mut alloc = IdentityAllocator::new();
    let record = fresh_record(
        &mut alloc,
        CallKind::Provider,
        EvidencePayload::Digest(digest("record")),
    );

    let sink = InMemoryEvidenceSink::new();
    let without_setup = manifest_for_observed_lifecycle(
        setup_binding.clone(),
        std::slice::from_ref(&record),
        vec![],
    );
    assert!(matches!(
        sink.finalize_run(&without_setup),
        Err(EvidenceError::Finalization { .. })
    ));

    let sink = InMemoryEvidenceSink::new();
    sink.setup(&setup_binding).unwrap();
    sink.emit(&record).unwrap();
    let wrong_binding = manifest_for_observed_lifecycle(
        RuntimeInputBinding::direct(digest("other"), assembly("opi.cli")),
        std::slice::from_ref(&record),
        vec![],
    );
    assert!(matches!(
        sink.finalize_run(&wrong_binding),
        Err(EvidenceError::Finalization { .. })
    ));
    assert!(sink.has_failure());
    assert!(sink.completed_manifest().is_none());

    let sink = InMemoryEvidenceSink::new();
    sink.setup(&setup_binding).unwrap();
    sink.emit(&record).unwrap();
    let mut wrong_run = manifest_for_observed_lifecycle(
        setup_binding.clone(),
        std::slice::from_ref(&record),
        vec![],
    )
    .facts()
    .clone();
    wrong_run.correlation.run = IdentityAllocator::new().run_id();
    let wrong_run = validate_with_candidate_observation(wrong_run).unwrap();
    assert!(matches!(
        sink.finalize_run(&wrong_run),
        Err(EvidenceError::Finalization { .. })
    ));
}

#[test]
fn in_memory_sink_rejects_manifest_terminal_correlation_mismatch() {
    fn assert_rejected(mutate: impl FnOnce(&mut ManifestCorrelation, &mut IdentityAllocator)) {
        let binding = RuntimeInputBinding::direct(digest("setup"), assembly("opi.cli"));
        let mut alloc = IdentityAllocator::new();
        let record = fresh_record(
            &mut alloc,
            CallKind::Provider,
            EvidencePayload::Digest(digest("record")),
        );
        let sink = InMemoryEvidenceSink::new();
        sink.setup(&binding).unwrap();
        sink.emit(&record).unwrap();
        let mut candidate =
            manifest_for_observed_lifecycle(binding, std::slice::from_ref(&record), vec![])
                .facts()
                .clone();
        mutate(&mut candidate.correlation, &mut alloc);
        let manifest = validate_with_candidate_observation(candidate).unwrap();
        assert!(matches!(
            sink.finalize_run(&manifest),
            Err(EvidenceError::Finalization { .. })
        ));
    }

    assert_rejected(|correlation, _| correlation.turn = None);
    assert_rejected(|correlation, alloc| correlation.call = Some(alloc.next_call()));
    assert_rejected(|correlation, alloc| correlation.sequence = alloc.next_sequence());

    let binding = RuntimeInputBinding::direct(digest("setup"), assembly("opi.cli"));
    let mut alloc = IdentityAllocator::new();
    let record = fresh_record(
        &mut alloc,
        CallKind::Provider,
        EvidencePayload::Digest(digest("record")),
    );
    let sink = InMemoryEvidenceSink::new();
    sink.setup(&binding).unwrap();
    sink.emit(&record).unwrap();
    let parent_call = alloc.next_call();
    let terminal_sequence = alloc.next_sequence();
    let mut candidate = sample_manifest_candidate(SessionBinding::NoSession);
    candidate.binding = binding.clone();
    candidate.correlation = ManifestCorrelation {
        run: record.run,
        turn: record.turn,
        call: Some(record.call),
        parent: Some(parent_call),
        sequence: terminal_sequence,
    };
    let parent_record = EvidenceRecord {
        run: record.run,
        turn: record.turn,
        call: parent_call,
        parent: None,
        sequence: record.sequence,
        kind: CallKind::Provider,
        payload: sample_provider_payload(),
    };
    let terminal_record = EvidenceRecord {
        run: record.run,
        turn: record.turn,
        call: record.call,
        parent: Some(parent_call),
        sequence: terminal_sequence,
        kind: CallKind::Provider,
        payload: sample_provider_payload(),
    };
    let fake_records = [parent_record, terminal_record];
    let manifest = candidate
        .validate(EvidenceRunObservation::new(&binding, &fake_records, &[]))
        .unwrap();
    assert!(matches!(
        sink.finalize_run(&manifest),
        Err(EvidenceError::Finalization { .. })
    ));
}

#[test]
fn in_memory_sink_rejects_finalized_artifact_set_mismatch() {
    let binding = RuntimeInputBinding::direct(digest("setup"), assembly("opi.cli"));
    let mut alloc = IdentityAllocator::new();
    let record = fresh_record(
        &mut alloc,
        CallKind::Provider,
        EvidencePayload::Digest(digest("record")),
    );

    let sink = InMemoryEvidenceSink::new();
    sink.setup(&binding).unwrap();
    sink.emit(&record).unwrap();
    sink.finalize_artifact(&artifact()).unwrap();
    let missing =
        manifest_for_observed_lifecycle(binding.clone(), std::slice::from_ref(&record), vec![]);
    assert!(matches!(
        sink.finalize_run(&missing),
        Err(EvidenceError::Finalization { .. })
    ));

    let sink = InMemoryEvidenceSink::new();
    sink.setup(&binding).unwrap();
    sink.emit(&record).unwrap();
    let invented =
        manifest_for_observed_lifecycle(binding, std::slice::from_ref(&record), vec![artifact()]);
    assert!(matches!(
        sink.finalize_run(&invented),
        Err(EvidenceError::Finalization { .. })
    ));
}

#[test]
fn in_memory_setup_resets_all_prior_run_state() {
    let sink = InMemoryEvidenceSink::new();
    let binding = RuntimeInputBinding::direct(digest("first"), assembly("opi.cli"));
    sink.setup(&binding).unwrap();
    let mut alloc = IdentityAllocator::new();
    sink.emit(&fresh_record(
        &mut alloc,
        CallKind::Provider,
        EvidencePayload::Digest(digest("record")),
    ))
    .unwrap();
    let finalized_artifact = artifact();
    sink.finalize_artifact(&finalized_artifact).unwrap();
    let records = sink.records();
    let manifest =
        manifest_for_observed_lifecycle(binding.clone(), &records, vec![finalized_artifact]);
    sink.finalize_run(&manifest).unwrap();

    let second = RuntimeInputBinding::direct(digest("second"), assembly("opi.sdk"));
    sink.setup(&second).unwrap();

    assert!(
        sink.records().is_empty(),
        "prior-run records survived setup"
    );
    assert!(
        sink.artifacts().is_empty(),
        "prior-run artifacts survived setup"
    );
    assert!(
        sink.completed_manifest().is_none(),
        "prior-run manifest survived setup"
    );
    assert!(
        !sink.has_failure(),
        "prior-run failure state survived setup"
    );
}

#[test]
fn setup_failure_withholds_manifest_and_marks_incomplete() {
    let sink = InMemoryEvidenceSink::new();
    sink.inject_failure(EvidenceError::Setup {
        detail: "no destination".to_owned(),
    });
    let binding = RuntimeInputBinding::direct(digest("i"), assembly("opi.cli"));
    let err = sink.setup(&binding).unwrap_err();
    assert!(matches!(err, EvidenceError::Setup { .. }));
    assert!(sink.has_failure());
    // A failure cannot be hidden by emitting a finalized manifest another way:
    // even if finalize_run runs, the public accessor withholds it.
    assert!(sink.finalize_run(&sample_manifest()).is_err());
    assert!(
        sink.completed_manifest().is_none(),
        "a failed run withholds the finalized manifest"
    );
}

#[test]
fn emission_failure_withholds_manifest_and_marks_incomplete() {
    let sink = InMemoryEvidenceSink::new();
    sink.setup(&RuntimeInputBinding::direct(
        digest("i"),
        assembly("opi.cli"),
    ))
    .unwrap();
    sink.inject_failure(EvidenceError::Emission {
        detail: "write error".to_owned(),
    });
    let mut alloc = IdentityAllocator::new();
    let rec = fresh_record(
        &mut alloc,
        CallKind::Provider,
        EvidencePayload::Digest(digest("d")),
    );
    let err = sink.emit(&rec).unwrap_err();
    assert!(matches!(err, EvidenceError::Emission { .. }));
    assert!(sink.has_failure());
    assert!(sink.finalize_run(&sample_manifest()).is_err());
    assert!(
        sink.completed_manifest().is_none(),
        "emission failure withholds the manifest"
    );
}

#[test]
fn finalization_failure_withholds_manifest() {
    let sink = InMemoryEvidenceSink::new();
    sink.setup(&RuntimeInputBinding::direct(
        digest("i"),
        assembly("opi.cli"),
    ))
    .unwrap();
    let mut alloc = IdentityAllocator::new();
    sink.emit(&fresh_record(
        &mut alloc,
        CallKind::Provider,
        EvidencePayload::Digest(digest("d")),
    ))
    .unwrap();
    sink.inject_failure(EvidenceError::Finalization {
        detail: "flush error".to_owned(),
    });
    let err = sink.finalize_run(&sample_manifest()).unwrap_err();
    assert!(matches!(err, EvidenceError::Finalization { .. }));
    assert!(sink.has_failure());
    assert!(
        sink.completed_manifest().is_none(),
        "finalization failure withholds the manifest"
    );
}

#[test]
fn noop_and_in_memory_satisfy_one_lifecycle_conformance_contract() {
    fn exercise<S: EvidenceSink>(sink: &S) {
        let binding = RuntimeInputBinding::direct(digest("i"), assembly("opi.sdk"));
        assert!(sink.setup(&binding).is_ok());
        let mut alloc = IdentityAllocator::new();
        let rec = fresh_record(
            &mut alloc,
            CallKind::Provider,
            EvidencePayload::Digest(digest("d")),
        );
        assert!(sink.emit(&rec).is_ok());
        let finalized_artifact = artifact();
        assert!(sink.finalize_artifact(&finalized_artifact).is_ok());
        let manifest = manifest_for_observed_lifecycle(
            binding,
            std::slice::from_ref(&rec),
            vec![finalized_artifact],
        );
        assert!(sink.finalize_run(&manifest).is_ok());
    }
    exercise(&NoopEvidenceSink::new());
    exercise(&InMemoryEvidenceSink::new());
}

// ===========================================================================
// Typed facts and fail-closed finalization
// ===========================================================================

#[test]
fn provider_facts_preserve_the_full_redacted_fallback_decision() {
    let route = opi_ai::PreparedRoute {
        provider_id: "mock".to_owned(),
        model_id: "model".to_owned(),
        wire_api: WireApi::OpenAiResponses,
    };
    let provenance = opi_ai::auth::AuthProvenance {
        source: opi_ai::auth::AuthProvenanceSource::Environment {
            name: "MOCK_API_KEY".to_owned(),
        },
        fallback: opi_ai::auth::AuthFallback::Used {
            from: opi_ai::auth::AuthProvenanceSource::CredentialStore {
                kind: "system-keyring".to_owned(),
            },
            to: opi_ai::auth::AuthProvenanceSource::Environment {
                name: "MOCK_API_KEY".to_owned(),
            },
            reason: "primary unavailable at https://example.test?token=phase17-secret-canary"
                .to_owned(),
        },
    };

    let facts = ProviderEvidenceFacts::from_prepared("mock:model", &route, &provenance)
        .expect("well-formed prepared facts");
    let mut alloc = IdentityAllocator::new();
    let sink = InMemoryEvidenceSink::new();
    sink.setup(&RuntimeInputBinding::direct(
        digest("fallback"),
        assembly("opi.embedder"),
    ))
    .unwrap();
    sink.emit(&fresh_record(
        &mut alloc,
        CallKind::Provider,
        EvidencePayload::Provider(facts),
    ))
    .unwrap();
    let recorded = sink.records().pop().expect("provider fact was recorded");
    let EvidencePayload::Provider(recorded_facts) = recorded.payload else {
        panic!("typed provider payload survives the sink")
    };
    match recorded_facts.provenance.fallback() {
        AuthFallbackFacts::Used {
            from,
            to,
            stable_reason,
        } => {
            assert!(matches!(
                from,
                AuthSourceFacts::CredentialStore { kind } if kind == "system-keyring"
            ));
            assert!(matches!(
                to,
                AuthSourceFacts::Environment { name } if name == "MOCK_API_KEY"
            ));
            assert!(!stable_reason.as_str().contains("phase17-secret-canary"));
        }
        other => panic!("expected full used-fallback facts, got {other:?}"),
    }

    let json = serde_json::to_value(EvidencePayload::Provider(recorded_facts)).unwrap();
    let rendered = json.to_string();

    assert_eq!(
        json["Provider"]["route"]["requested"]["provider_id"],
        "mock"
    );
    assert_eq!(json["Provider"]["route"]["requested"]["model_id"], "model");
    assert_eq!(
        json["Provider"]["route"]["resolved"]["wire"],
        "openai-responses"
    );
    assert_eq!(
        json["Provider"]["route"]["actual"]["reason"],
        "not_reported"
    );
    assert_eq!(
        json["Provider"]["provenance"]["fallback"]["from"]["credential_store"]["kind"],
        "system-keyring"
    );
    assert_eq!(
        json["Provider"]["provenance"]["fallback"]["to"]["environment"]["name"],
        "MOCK_API_KEY"
    );
    assert!(!rendered.contains("phase17-secret-canary"));
    assert!(rendered.contains("REDACTED"));
}

#[test]
fn provider_facts_reject_contradictory_auth_fallback_provenance() {
    use opi_ai::auth::{AuthFallback, AuthProvenance, AuthProvenanceSource};

    let selected = AuthProvenanceSource::Environment {
        name: "SELECTED_KEY".to_owned(),
    };
    let wrong_target = ProvenanceFacts::from_auth(&AuthProvenance {
        source: selected.clone(),
        fallback: AuthFallback::Used {
            from: AuthProvenanceSource::Static,
            to: AuthProvenanceSource::CredentialStore {
                kind: "keyring".to_owned(),
            },
            reason: "fallback".to_owned(),
        },
    });
    assert!(matches!(
        wrong_target,
        Err(EvidenceFactError::InconsistentAuthFallback {
            reason: AuthFallbackInconsistency::TargetDoesNotMatchSelectedSource
        })
    ));

    let same_source = ProvenanceFacts::from_auth(&AuthProvenance {
        source: selected.clone(),
        fallback: AuthFallback::Used {
            from: selected.clone(),
            to: selected,
            reason: "fallback".to_owned(),
        },
    });
    assert!(matches!(
        same_source,
        Err(EvidenceFactError::InconsistentAuthFallback {
            reason: AuthFallbackInconsistency::SourceEqualsTarget
        })
    ));
}

#[test]
fn provider_facts_reject_malformed_requested_routes() {
    let route = opi_ai::PreparedRoute {
        provider_id: "mock".to_owned(),
        model_id: "model".to_owned(),
        wire_api: WireApi::OpenAiResponses,
    };
    assert!(
        ProviderEvidenceFacts::from_prepared(
            "bare-model",
            &route,
            &opi_ai::auth::AuthProvenance::default(),
        )
        .is_err()
    );
    assert!(RouteSelection::new("", "model", WireApi::OpenAiResponses).is_err());
    assert!(RouteSelection::new("mock", " model", WireApi::OpenAiResponses).is_err());
    assert!(ActualRoute::wire_unknown("", "model", UnknownReason::NotReported).is_err());
    assert!(
        ProvenanceFacts::from_auth(&opi_ai::auth::AuthProvenance {
            source: opi_ai::auth::AuthProvenanceSource::Environment {
                name: String::new(),
            },
            fallback: opi_ai::auth::AuthFallback::NotAttempted,
        })
        .is_err(),
        "malformed supplementary provenance must fail closed"
    );
}

#[test]
fn actual_provider_and_model_can_be_known_while_wire_retains_an_unknown_reason() {
    let actual = ActualRoute::wire_unknown("mock", "provider-model", UnknownReason::NotReported)
        .expect("valid actual route");
    let json = serde_json::to_value(actual).unwrap();
    assert_eq!(json["kind"], "wire_unknown");
    assert_eq!(json["provider_id"], "mock");
    assert_eq!(json["model_id"], "provider-model");
    assert_eq!(json["reason"], "not_reported");
    assert!(json.get("wire").is_none());

    let absent =
        ActualRoute::from_reported_provider_model("", "provider-model", UnknownReason::NotReported)
            .expect("an absent provider makes the whole actual route unknown");
    assert!(matches!(
        absent,
        ActualRoute::Unknown {
            reason: UnknownReason::NotReported
        }
    ));
    assert!(
        ActualRoute::from_reported_provider_model("", " malformed", UnknownReason::NotReported,)
            .is_err(),
        "an absent field must not hide another malformed non-empty field"
    );
}

#[test]
fn typed_tool_authorization_retains_invocation_and_policy_bindings() {
    let allowed = ToolAuthorizationFacts::Allowed {
        policy_ref: PolicyReference::new("policy:v1").unwrap(),
        permission_ref: PermissionReference::new("permission:execute").unwrap(),
        permission_scope: PermissionScope::new("workspace:current").unwrap(),
        scoped_grant_ref: Some(ScopedGrantReference::new("grant:42").unwrap()),
    };
    let facts = ToolEvidenceFacts::authorization(
        "bash",
        "builtin-bash",
        CapabilityIdentity::new("opi.command.execute").unwrap(),
        InvocationBinding::NoSession,
        allowed,
    )
    .expect("valid authorization facts");
    let json = serde_json::to_value(EvidencePayload::Tool(facts)).unwrap();

    assert_eq!(json["Tool"]["invocation"]["kind"], "no_session");
    assert_eq!(
        json["Tool"]["authorization"]["allowed"]["policy_ref"],
        "policy:v1"
    );
    assert_eq!(
        json["Tool"]["authorization"]["allowed"]["permission_ref"],
        "permission:execute"
    );
    assert_eq!(
        json["Tool"]["authorization"]["allowed"]["permission_scope"],
        "workspace:current"
    );
    assert_eq!(
        json["Tool"]["authorization"]["allowed"]["scoped_grant_ref"],
        "grant:42"
    );
}

#[test]
fn tool_facts_reject_incomplete_or_phase_invalid_facts() {
    let capability = CapabilityIdentity::new("opi.command.execute").unwrap();

    assert!(
        ToolEvidenceFacts::outcome(
            "bash",
            None,
            Some(capability.clone()),
            InvocationBinding::NoSession,
            ToolExecutionOutcome::Succeeded,
        )
        .is_err(),
        "capability without a trusted registration must fail closed"
    );
    assert!(
        ToolEvidenceFacts::combined(
            "bash",
            Some("builtin-bash"),
            None,
            InvocationBinding::NoSession,
            ToolAuthorizationFacts::NotReached,
            ToolExecutionOutcome::Failed,
        )
        .is_err(),
        "registration without its capability must fail closed"
    );

    assert!(
        ToolEvidenceFacts::authorization(
            "bash",
            "builtin-bash",
            capability.clone(),
            InvocationBinding::NoSession,
            ToolAuthorizationFacts::NotReached,
        )
        .is_err(),
        "an authorization-phase record cannot claim authorization was not reached"
    );
    assert!(
        ToolEvidenceFacts::combined(
            "bash",
            Some("builtin-bash"),
            Some(capability),
            InvocationBinding::NoSession,
            ToolAuthorizationFacts::NotReached,
            ToolExecutionOutcome::Failed,
        )
        .is_err(),
        "a combined authorization record cannot claim authorization was not reached"
    );
    assert!(matches!(
        ToolEvidenceFacts::outcome(
            "missing-tool",
            None,
            None,
            InvocationBinding::NoSession,
            ToolExecutionOutcome::Failed,
        )
        .unwrap()
        .authorization_facts(),
        ToolAuthorizationFacts::NotReached
    ));
}

#[test]
fn trigger_variants_and_explicit_session_bindings_are_distinguishable() {
    let triggers = [
        ExecutionTrigger::Invocation,
        ExecutionTrigger::Retry,
        ExecutionTrigger::Continuation,
        ExecutionTrigger::Compaction {
            reason: CompactionTrigger::Manual,
        },
        ExecutionTrigger::Compaction {
            reason: CompactionTrigger::Threshold,
        },
        ExecutionTrigger::Compaction {
            reason: CompactionTrigger::Overflow,
        },
    ];
    let rendered: std::collections::BTreeSet<String> = triggers
        .into_iter()
        .map(|trigger| serde_json::to_string(&trigger).unwrap())
        .collect();
    assert_eq!(rendered.len(), 6);

    let branch = SessionBinding::branch("main").expect("valid branch");
    let no_session = SessionBinding::NoSession;
    assert_ne!(
        serde_json::to_string(&branch).unwrap(),
        serde_json::to_string(&no_session).unwrap()
    );
    assert!(SessionBinding::branch("").is_err());
}

#[test]
fn only_a_validated_manifest_can_cross_the_sink_boundary() {
    let binding = RuntimeInputBinding::direct(digest("i"), assembly("opi.embedder"));
    let mut alloc = IdentityAllocator::new();
    let record = fresh_record(
        &mut alloc,
        CallKind::Provider,
        EvidencePayload::Digest(digest("record")),
    );
    let valid =
        manifest_for_observed_lifecycle(binding.clone(), std::slice::from_ref(&record), vec![]);
    let sink = InMemoryEvidenceSink::new();
    sink.setup(&binding).unwrap();
    sink.emit(&record).unwrap();
    sink.finalize_run(&valid).unwrap();
    assert_eq!(sink.completed_manifest(), Some(valid));

    let branch = validate_with_candidate_observation(sample_manifest_candidate(
        SessionBinding::branch("main").unwrap(),
    ))
    .expect("complete branch-bound manifest validates");
    assert!(matches!(branch.session(), SessionBinding::Branch { .. }));
}

fn sample_manifest_candidate(session: SessionBinding) -> ManifestCandidate {
    let mut alloc = IdentityAllocator::new();
    ManifestCandidate {
        correlation: ManifestCorrelation {
            run: alloc.run_id(),
            turn: Some(alloc.next_turn()),
            call: Some(alloc.next_call()),
            parent: None,
            sequence: alloc.next_sequence(),
        },
        outcome: TerminalOutcome::Success,
        session,
        binding: RuntimeInputBinding::direct(digest("inputs"), assembly("opi.embedder")),
        config: ConfigIdentity {
            harness_digest: digest("h"),
            runtime_digest: digest("r"),
            adapter_digest: digest("a"),
            material_digest: digest("m"),
        },
        provider: {
            let facts = sample_provider_facts();
            ProviderInvocationFacts::applicable(facts.route, facts.provenance)
        },
        policy: UserPolicyFacts {
            policy_digest: digest("policy"),
            capability: None,
            permission_ref: None,
            permission_scope: None,
            scoped_grant_ref: None,
        },
        input_identity: InputIdentity {
            prompt_digest: digest("prompt"),
            system_digest: None,
            tool_schema_digests: vec![],
        },
        environment: EnvironmentFacts {
            budget: Measurement::provider_reported(1000),
            trigger: ExecutionTrigger::Invocation,
            time: Measurement::Unknown {
                reason: UnknownReason::NotReported,
            },
            platform: PlatformIdentity::new("linux"),
        },
        usage: UsageFacts {
            input_tokens: Measurement::provider_reported(42),
            output_tokens: Measurement::Unknown {
                reason: UnknownReason::NotReported,
            },
        },
        artifacts: vec![],
        completeness: EvidenceCompleteness::Complete,
    }
}

#[test]
fn manifest_retains_every_typed_terminal_outcome() {
    for (outcome, serialized) in [
        (TerminalOutcome::Success, "success"),
        (TerminalOutcome::Cancelled, "cancelled"),
        (TerminalOutcome::Failed, "failed"),
        (TerminalOutcome::PartialSideEffect, "partial_side_effect"),
        (TerminalOutcome::CleanupUnknown, "cleanup_unknown"),
    ] {
        let manifest = validate_with_candidate_observation({
            let mut candidate = sample_manifest_candidate(SessionBinding::NoSession);
            candidate.outcome = outcome.clone();
            candidate
        })
        .expect("every closed terminal outcome is a valid manifest fact");
        assert_eq!(manifest.facts().outcome, outcome);
        assert_eq!(
            serde_json::to_value(&manifest).unwrap()["outcome"],
            serialized
        );
    }
}

#[test]
fn in_memory_abandon_withholds_manifest_and_closes_lifecycle() {
    let binding = RuntimeInputBinding::direct(digest("abandon"), assembly("opi.embedder"));
    let sink = InMemoryEvidenceSink::new();
    sink.setup(&binding).unwrap();

    sink.abandon_run(&TerminalOutcome::Failed).unwrap();

    assert!(sink.completed_manifest().is_none());
    let mut allocator = IdentityAllocator::new();
    let record = fresh_record(
        &mut allocator,
        CallKind::Provider,
        EvidencePayload::Digest(digest("after-abandon")),
    );
    assert!(matches!(
        sink.emit(&record),
        Err(EvidenceError::Emission { .. })
    ));
}
