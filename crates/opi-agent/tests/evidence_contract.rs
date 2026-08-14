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

fn route(provider: &str, model: &str) -> RouteSelection {
    RouteSelection {
        provider_id: provider.to_owned(),
        model_id: model.to_owned(),
        wire: WireApi::OpenAiCompletions,
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

fn fresh_record(
    alloc: &mut IdentityAllocator,
    kind: CallKind,
    payload: EvidencePayload,
) -> EvidenceRecord {
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
    let mut alloc = IdentityAllocator::new();
    FinalizedManifest {
        correlation: ManifestCorrelation {
            run: alloc.run_id(),
            turn: Some(alloc.next_turn()),
            call: Some(alloc.next_call()),
            parent: None,
            sequence: alloc.next_sequence(),
        },
        outcome: TerminalOutcome::Success,
        session_branch: Some(SessionBranchRef::new("main")),
        binding: RuntimeInputBinding::direct(digest("inputs"), AssemblySource::Cli),
        config: ConfigIdentity {
            harness_digest: digest("h"),
            runtime_digest: digest("r"),
            adapter_digest: digest("a"),
            material_digest: digest("m"),
        },
        route: RouteFacts {
            requested: route("anthropic", "claude"),
            resolved: route("anthropic", "claude"),
            actual: route("anthropic", "claude"),
            actual_reason: None,
        },
        provenance: ProvenanceFacts {
            auth_source: AuthProvenanceSource::CredentialStore,
            fallback_allowed: Some(false),
        },
        policy: UserPolicyFacts {
            policy_digest: digest("policy"),
            capability: Some(CapabilityClass::CommandExecute),
        },
        input_identity: InputIdentity {
            prompt_digest: digest("prompt"),
            system_digest: None,
            tool_schema_digests: vec![],
        },
        environment: EnvironmentFacts {
            budget: Measurement::provider_reported(1000),
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
    let b = RuntimeInputBinding::direct(digest("inputs"), AssemblySource::Cli);
    assert!(b.is_direct());
    assert!(matches!(b, RuntimeInputBinding::DirectRuntimeInput { .. }));
}

#[test]
fn binding_variants_are_distinguishable_and_not_normalizable() {
    let direct = RuntimeInputBinding::direct(digest("d"), AssemblySource::Sdk);
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
        AssemblySource::Cli,
        AssemblySource::Sdk,
        AssemblySource::Rpc,
    ] {
        let b = RuntimeInputBinding::direct(digest("x"), source);
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
    let mut incomplete = sample_manifest();
    incomplete.completeness = EvidenceCompleteness::Incomplete;
    assert!(incomplete.require_complete().is_err());

    let mut pending = sample_manifest();
    let mut pending_artifact = artifact();
    pending_artifact.finalization = FinalizationState::Pending;
    pending.artifacts.push(pending_artifact);
    assert!(pending.require_complete().is_err());
}

// ===========================================================================
// Sink lifecycle and adapters (P17-EVD-008 / P17-EVD-010 / P17-EVD-011)
// ===========================================================================

#[test]
fn noop_sink_is_default_and_captures_nothing() {
    let sink = NoopEvidenceSink::new();
    let binding = RuntimeInputBinding::direct(digest("i"), AssemblySource::Cli);
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
    let binding = RuntimeInputBinding::direct(digest("i"), AssemblySource::Cli);
    sink.setup(&binding).unwrap();
    let mut alloc = IdentityAllocator::new();
    let r1 = fresh_record(
        &mut alloc,
        CallKind::Provider,
        EvidencePayload::Digest(digest("1")),
    );
    let r2 = fresh_record(
        &mut alloc,
        CallKind::Tool,
        EvidencePayload::Digest(digest("2")),
    );
    let r3 = fresh_record(
        &mut alloc,
        CallKind::Compaction,
        EvidencePayload::Digest(digest("3")),
    );
    sink.emit(&r1).unwrap();
    sink.emit(&r2).unwrap();
    sink.emit(&r3).unwrap();
    sink.finalize_artifact(&artifact()).unwrap();
    sink.finalize_run(&sample_manifest()).unwrap();

    let records = sink.records();
    assert_eq!(records.len(), 3, "three records recorded in order");
    assert_eq!(records[0].sequence, r1.sequence);
    assert_eq!(records[2].sequence, r3.sequence);
    assert_eq!(sink.artifacts().len(), 1);
    assert!(!sink.has_failure());
    assert!(
        sink.completed_manifest().is_some(),
        "a clean run yields a completed manifest"
    );
}

#[test]
fn in_memory_setup_resets_all_prior_run_state() {
    let sink = InMemoryEvidenceSink::new();
    let binding = RuntimeInputBinding::direct(digest("first"), AssemblySource::Cli);
    sink.setup(&binding).unwrap();
    let mut alloc = IdentityAllocator::new();
    sink.emit(&fresh_record(
        &mut alloc,
        CallKind::Provider,
        EvidencePayload::Digest(digest("record")),
    ))
    .unwrap();
    sink.finalize_artifact(&artifact()).unwrap();
    sink.finalize_run(&sample_manifest()).unwrap();

    let second = RuntimeInputBinding::direct(digest("second"), AssemblySource::Sdk);
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
    let binding = RuntimeInputBinding::direct(digest("i"), AssemblySource::Cli);
    let err = sink.setup(&binding).unwrap_err();
    assert!(matches!(err, EvidenceError::Setup { .. }));
    assert!(sink.has_failure());
    // A failure cannot be hidden by emitting a finalized manifest another way:
    // even if finalize_run runs, the public accessor withholds it.
    sink.finalize_run(&sample_manifest()).unwrap();
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
        AssemblySource::Cli,
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
    sink.finalize_run(&sample_manifest()).unwrap();
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
        AssemblySource::Cli,
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
        let binding = RuntimeInputBinding::direct(digest("i"), AssemblySource::Sdk);
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
    exercise(&NoopEvidenceSink::new());
    exercise(&InMemoryEvidenceSink::new());
}
