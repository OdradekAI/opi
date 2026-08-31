# Phase 18 Audit

**Audit run ID**: `phase18-pi-glm53-25d0e68-20260831t124752z`
**Audit head**: `25d0e6823ea13537702e79bd3c94064bd7c67197`
**Reviewer ID**: `pi`
**Model ID**: `glm53`
**Reviewer identity**: pi coding agent (earendil-works pi)
**Reviewer model ID**: `glm-5.3`
**Model identity source**: runtime-attested
**Independence**: fresh-context-same-family — new task context; no prior Phase 18 audit, remediation, or sibling-peer content was read; model identity attested by the pi runtime session record (provider zai-coding-cn, model glm-5.3)
**Baseline policy**: latest-committed-spec
**Verdict**: PASS-WITH-FINDINGS

## Baseline Sources

| Path | SHA-256 | Registration note |
|---|---|---|
| `.opi-impl-state.json` | `9d2ecf977f940f03db3c5d3b17437ad4a3afbca6ad409fcebf306727848a358e` | root implementation ledger (committed) |
| `docs/snapshots/phase18/opi-impl-state.json` | `cea5031074ac0d5667357863fbdf03bc76494295a6c38fac304dc1c851d7b42c` | pointed Phase 18 state (committed) |
| `docs/opi-spec.md` | `cc7f8898f60c0d8abaa667f4b49b7affc721412e75dd3a67dcde37a783e1bc4c` | normative spec (registered) |
| `docs/superpowers/specs/2026-08-25-phase18-independent-cross-agent-eval-seam-validation-design.md` | `43b2759d327cbf0af8d35d4eba50839eef7aac473978b58fcb707b335dad8265` | pointed Phase 18 state (committed) |

## Requirement Conformance

| Requirement ID | Criterion | Current evidence | State | Finding IDs |
|---|---|---|---|---|
| P18-AUTH-001 | P18-AUTH | python3 scripts/opi-doc-check.py | met |  |
| P18-AUTH-002 | P18-AUTH | python3 scripts/opi-doc-check.py | met |  |
| P18-AUTH-003 | P18-AUTH | python3 scripts/opi-doc-check.py | met |  |
| P18-AUTH-004 | P18-AUTH | python3 scripts/opi-doc-check.py | met |  |
| P18-AUTH-005 | P18-AUTH | python3 scripts/opi-doc-check.py | met |  |
| P18-OUT-001 | P18-OUT | crates/opi-eval/tests/{bundle_recompute,report_contract,phase18_assembled_smoke,end_to_end_report}.rs | met |  |
| P18-OUT-002 | P18-OUT | crates/opi-eval/tests/{bundle_recompute,report_contract,phase18_assembled_smoke,end_to_end_report}.rs | met |  |
| P18-OUT-003 | P18-OUT | crates/opi-eval/tests/{bundle_recompute,report_contract,phase18_assembled_smoke,end_to_end_report}.rs | met |  |
| P18-OUT-004 | P18-OUT | crates/opi-eval/tests/{bundle_recompute,report_contract,phase18_assembled_smoke,end_to_end_report}.rs | met |  |
| P18-OUT-005 | P18-OUT | crates/opi-eval/tests/{bundle_recompute,report_contract,phase18_assembled_smoke,end_to_end_report}.rs | met |  |
| P18-OUT-006 | P18-OUT | crates/opi-eval/tests/{bundle_recompute,report_contract,phase18_assembled_smoke,end_to_end_report}.rs | met |  |
| P18-PLC-001 | P18-PLC | crates/opi-eval/tests/experiment_contract.rs (p18_a01); crates/opi-eval/tests/agent_integration_conformance.rs (no-fallback cases) | met |  |
| P18-PLC-002 | P18-PLC | crates/opi-eval/tests/experiment_contract.rs (p18_a01); crates/opi-eval/tests/agent_integration_conformance.rs (no-fallback cases) | met |  |
| P18-PLC-003 | P18-PLC | crates/opi-eval/tests/experiment_contract.rs (p18_a01); crates/opi-eval/tests/agent_integration_conformance.rs (no-fallback cases) | met |  |
| P18-PLC-004 | P18-PLC | crates/opi-eval/tests/experiment_contract.rs (p18_a01); crates/opi-eval/tests/agent_integration_conformance.rs (no-fallback cases) | met |  |
| P18-PLC-005 | P18-PLC | crates/opi-eval/tests/experiment_contract.rs (p18_a01); crates/opi-eval/tests/agent_integration_conformance.rs (no-fallback cases) | met |  |
| P18-PLC-006 | P18-PLC | crates/opi-eval/tests/experiment_contract.rs (p18_a01); crates/opi-eval/tests/agent_integration_conformance.rs (no-fallback cases) | met |  |
| P18-SEAM-001 | P18-SEAM | crates/opi-eval/tests/phase18_acceptance.rs (a20); crates/opi-eval/tests/report_contract.rs (a16 asymmetric unknowns) | met | P18-AUD-002 |
| P18-SEAM-002 | P18-SEAM | crates/opi-eval/tests/phase18_acceptance.rs (a20); crates/opi-eval/tests/report_contract.rs (a16 asymmetric unknowns) | met |  |
| P18-SEAM-003 | P18-SEAM | crates/opi-eval/tests/phase18_acceptance.rs (a20); crates/opi-eval/tests/report_contract.rs (a16 asymmetric unknowns) | met |  |
| P18-SEAM-004 | P18-SEAM | crates/opi-eval/tests/phase18_acceptance.rs (a20); crates/opi-eval/tests/report_contract.rs (a16 asymmetric unknowns) | met |  |
| P18-SEAM-005 | P18-SEAM | crates/opi-eval/tests/phase18_acceptance.rs (a20); crates/opi-eval/tests/report_contract.rs (a16 asymmetric unknowns) | met |  |
| P18-EXP-001 | P18-EXP | crates/opi-eval/tests/experiment_contract.rs (14 tests); crates/opi-eval/tests/pairing_and_integrity.rs (a13) | met |  |
| P18-EXP-002 | P18-EXP | crates/opi-eval/tests/experiment_contract.rs (14 tests); crates/opi-eval/tests/pairing_and_integrity.rs (a13) | met |  |
| P18-EXP-003 | P18-EXP | crates/opi-eval/tests/experiment_contract.rs (14 tests); crates/opi-eval/tests/pairing_and_integrity.rs (a13) | met |  |
| P18-EXP-004 | P18-EXP | crates/opi-eval/tests/experiment_contract.rs (14 tests); crates/opi-eval/tests/pairing_and_integrity.rs (a13) | met |  |
| P18-EXP-005 | P18-EXP | crates/opi-eval/tests/experiment_contract.rs (14 tests); crates/opi-eval/tests/pairing_and_integrity.rs (a13) | met |  |
| P18-EXP-006 | P18-EXP | crates/opi-eval/tests/experiment_contract.rs (14 tests); crates/opi-eval/tests/pairing_and_integrity.rs (a13) | met |  |
| P18-EXP-007 | P18-EXP | crates/opi-eval/tests/experiment_contract.rs (14 tests); crates/opi-eval/tests/pairing_and_integrity.rs (a13) | met |  |
| P18-EXP-008 | P18-EXP | crates/opi-eval/tests/experiment_contract.rs (14 tests); crates/opi-eval/tests/pairing_and_integrity.rs (a13) | met |  |
| P18-DUR-001 | P18-DUR | crates/opi-eval/src/runner/lifecycle.rs unit tests (crash-after-intent = effect-unknown); crates/opi-eval/tests/bundle_recompute.rs | met |  |
| P18-DUR-002 | P18-DUR | crates/opi-eval/src/runner/lifecycle.rs unit tests (crash-after-intent = effect-unknown); crates/opi-eval/tests/bundle_recompute.rs | met |  |
| P18-DUR-003 | P18-DUR | crates/opi-eval/src/runner/lifecycle.rs unit tests (crash-after-intent = effect-unknown); crates/opi-eval/tests/bundle_recompute.rs | met |  |
| P18-DUR-004 | P18-DUR | crates/opi-eval/src/runner/lifecycle.rs unit tests (crash-after-intent = effect-unknown); crates/opi-eval/tests/bundle_recompute.rs | met |  |
| P18-DUR-005 | P18-DUR | crates/opi-eval/src/runner/lifecycle.rs unit tests (crash-after-intent = effect-unknown); crates/opi-eval/tests/bundle_recompute.rs | met |  |
| P18-AGT-001 | P18-AGT | crates/opi-eval/tests/agent_integration_conformance.rs (2 tests, pinned settlement truth table); crates/opi-eval/tests/phase18_assembled_smoke.rs | met |  |
| P18-AGT-002 | P18-AGT | crates/opi-eval/tests/agent_integration_conformance.rs (2 tests, pinned settlement truth table); crates/opi-eval/tests/phase18_assembled_smoke.rs | met |  |
| P18-AGT-003 | P18-AGT | crates/opi-eval/tests/agent_integration_conformance.rs (2 tests, pinned settlement truth table); crates/opi-eval/tests/phase18_assembled_smoke.rs | met |  |
| P18-AGT-004 | P18-AGT | crates/opi-eval/tests/agent_integration_conformance.rs (2 tests, pinned settlement truth table); crates/opi-eval/tests/phase18_assembled_smoke.rs | met |  |
| P18-AGT-005 | P18-AGT | crates/opi-eval/tests/agent_integration_conformance.rs (2 tests, pinned settlement truth table); crates/opi-eval/tests/phase18_assembled_smoke.rs | met |  |
| P18-AGT-006 | P18-AGT | crates/opi-eval/tests/agent_integration_conformance.rs (2 tests, pinned settlement truth table); crates/opi-eval/tests/phase18_assembled_smoke.rs | met |  |
| P18-AGT-007 | P18-AGT | crates/opi-eval/tests/agent_integration_conformance.rs (2 tests, pinned settlement truth table); crates/opi-eval/tests/phase18_assembled_smoke.rs | met |  |
| P18-AGT-008 | P18-AGT | crates/opi-eval/tests/agent_integration_conformance.rs (2 tests, pinned settlement truth table); crates/opi-eval/tests/phase18_assembled_smoke.rs | met |  |
| P18-AGT-009 | P18-AGT | crates/opi-eval/tests/agent_integration_conformance.rs (2 tests, pinned settlement truth table); crates/opi-eval/tests/phase18_assembled_smoke.rs | met |  |
| P18-BMK-001 | P18-BMK | crates/opi-eval/tests/benchmark_integration_conformance.rs (2 tests); crates/opi-eval/tests/native_driver.rs (8 tests) | met |  |
| P18-BMK-002 | P18-BMK | crates/opi-eval/tests/benchmark_integration_conformance.rs (2 tests); crates/opi-eval/tests/native_driver.rs (8 tests) | met |  |
| P18-BMK-003 | P18-BMK | crates/opi-eval/tests/benchmark_integration_conformance.rs (2 tests); crates/opi-eval/tests/native_driver.rs (8 tests) | met |  |
| P18-BMK-004 | P18-BMK | crates/opi-eval/tests/benchmark_integration_conformance.rs (2 tests); crates/opi-eval/tests/native_driver.rs (8 tests) | met |  |
| P18-BMK-005 | P18-BMK | crates/opi-eval/tests/benchmark_integration_conformance.rs (2 tests); crates/opi-eval/tests/native_driver.rs (8 tests) | met |  |
| P18-BMK-006 | P18-BMK | crates/opi-eval/tests/benchmark_integration_conformance.rs (2 tests); crates/opi-eval/tests/native_driver.rs (8 tests) | met |  |
| P18-BMK-007 | P18-BMK | crates/opi-eval/tests/benchmark_integration_conformance.rs (2 tests); crates/opi-eval/tests/native_driver.rs (8 tests) | met |  |
| P18-BMK-008 | P18-BMK | crates/opi-eval/tests/benchmark_integration_conformance.rs (2 tests); crates/opi-eval/tests/native_driver.rs (8 tests) | met |  |
| P18-BMK-009 | P18-BMK | crates/opi-eval/tests/benchmark_integration_conformance.rs (2 tests); crates/opi-eval/tests/native_driver.rs (8 tests) | met |  |
| P18-BMK-010 | P18-BMK | crates/opi-eval/tests/benchmark_integration_conformance.rs (2 tests); crates/opi-eval/tests/native_driver.rs (8 tests) | met |  |
| P18-BMK-011 | P18-BMK | crates/opi-eval/tests/benchmark_integration_conformance.rs (2 tests); crates/opi-eval/tests/native_driver.rs (8 tests) | met |  |
| P18-RDM-001 | P18-RDM | crates/opi-eval/tests/phase18_acceptance.rs (a20 generic 3-subject/fourth-benchmark fixture, a21 roadmap) | met |  |
| P18-RDM-002 | P18-RDM | crates/opi-eval/tests/phase18_acceptance.rs (a20 generic 3-subject/fourth-benchmark fixture, a21 roadmap) | met |  |
| P18-RDM-003 | P18-RDM | crates/opi-eval/tests/phase18_acceptance.rs (a20 generic 3-subject/fourth-benchmark fixture, a21 roadmap) | met |  |
| P18-RDM-004 | P18-RDM | crates/opi-eval/tests/phase18_acceptance.rs (a20 generic 3-subject/fourth-benchmark fixture, a21 roadmap) | met |  |
| P18-RDM-005 | P18-RDM | crates/opi-eval/tests/phase18_acceptance.rs (a20 generic 3-subject/fourth-benchmark fixture, a21 roadmap) | met |  |
| P18-RDM-006 | P18-RDM | crates/opi-eval/tests/phase18_acceptance.rs (a20 generic 3-subject/fourth-benchmark fixture, a21 roadmap) | met |  |
| P18-INT-001 | P18-INT | crates/opi-eval/tests/pairing_and_integrity.rs (a14 exclusion + reclassification digest change + identity-reuse refusal) | met |  |
| P18-INT-002 | P18-INT | crates/opi-eval/tests/pairing_and_integrity.rs (a14 exclusion + reclassification digest change + identity-reuse refusal) | met |  |
| P18-INT-003 | P18-INT | crates/opi-eval/tests/pairing_and_integrity.rs (a14 exclusion + reclassification digest change + identity-reuse refusal) | met |  |
| P18-INT-004 | P18-INT | crates/opi-eval/tests/pairing_and_integrity.rs (a14 exclusion + reclassification digest change + identity-reuse refusal) | met | P18-AUD-001 |
| P18-INT-005 | P18-INT | crates/opi-eval/tests/pairing_and_integrity.rs (a14 exclusion + reclassification digest change + identity-reuse refusal) | met |  |
| P18-BND-001 | P18-BND | crates/opi-eval/tests/bundle_recompute.rs (a15 mutation-detected/digest-mismatch, bnd001 unmanifested-file + sidecar-drift); crates/opi-eval/tests/... | met |  |
| P18-BND-002 | P18-BND | crates/opi-eval/tests/bundle_recompute.rs (a15 mutation-detected/digest-mismatch, bnd001 unmanifested-file + sidecar-drift); crates/opi-eval/tests/... | met |  |
| P18-BND-003 | P18-BND | crates/opi-eval/tests/bundle_recompute.rs (a15 mutation-detected/digest-mismatch, bnd001 unmanifested-file + sidecar-drift); crates/opi-eval/tests/... | met |  |
| P18-BND-004 | P18-BND | crates/opi-eval/tests/bundle_recompute.rs (a15 mutation-detected/digest-mismatch, bnd001 unmanifested-file + sidecar-drift); crates/opi-eval/tests/... | met |  |
| P18-BND-005 | P18-BND | crates/opi-eval/tests/bundle_recompute.rs (a15 mutation-detected/digest-mismatch, bnd001 unmanifested-file + sidecar-drift); crates/opi-eval/tests/... | met |  |
| P18-BND-006 | P18-BND | crates/opi-eval/tests/bundle_recompute.rs (a15 mutation-detected/digest-mismatch, bnd001 unmanifested-file + sidecar-drift); crates/opi-eval/tests/... | met |  |
| P18-TRJ-001 | P18-TRJ | crates/opi-eval/tests/report_contract.rs (a16 pi usage unknown:pi-usage-not-native, no fabricated value); crates/opi-eval/tests/end_to_end_report.rs | met |  |
| P18-TRJ-002 | P18-TRJ | crates/opi-eval/tests/report_contract.rs (a16 pi usage unknown:pi-usage-not-native, no fabricated value); crates/opi-eval/tests/end_to_end_report.rs | met |  |
| P18-TRJ-003 | P18-TRJ | crates/opi-eval/tests/report_contract.rs (a16 pi usage unknown:pi-usage-not-native, no fabricated value); crates/opi-eval/tests/end_to_end_report.rs | met |  |
| P18-TRJ-004 | P18-TRJ | crates/opi-eval/tests/report_contract.rs (a16 pi usage unknown:pi-usage-not-native, no fabricated value); crates/opi-eval/tests/end_to_end_report.rs | met |  |
| P18-TRJ-005 | P18-TRJ | crates/opi-eval/tests/report_contract.rs (a16 pi usage unknown:pi-usage-not-native, no fabricated value); crates/opi-eval/tests/end_to_end_report.rs | met |  |
| P18-FAL-001 | P18-FAL | crates/opi-eval/tests/authority_boundaries.rs (4 tests: scored Agent failures dispatch grader; boundary failures stop transitions via executed() co... | met | P18-AUD-001 |
| P18-FAL-002 | P18-FAL | crates/opi-eval/tests/authority_boundaries.rs (4 tests: scored Agent failures dispatch grader; boundary failures stop transitions via executed() co... | met |  |
| P18-FAL-003 | P18-FAL | crates/opi-eval/tests/authority_boundaries.rs (4 tests: scored Agent failures dispatch grader; boundary failures stop transitions via executed() co... | met |  |
| P18-FAL-004 | P18-FAL | crates/opi-eval/tests/authority_boundaries.rs (4 tests: scored Agent failures dispatch grader; boundary failures stop transitions via executed() co... | met |  |
| P18-FAL-005 | P18-FAL | crates/opi-eval/tests/authority_boundaries.rs (4 tests: scored Agent failures dispatch grader; boundary failures stop transitions via executed() co... | met |  |
| P18-RPT-001 | P18-RPT | crates/opi-eval/tests/bundle_recompute.rs (a17 byte-stable regrade/report, identity-reuse refusal, offline reproduction); crates/opi-eval/tests/rep... | met |  |
| P18-RPT-002 | P18-RPT | crates/opi-eval/tests/bundle_recompute.rs (a17 byte-stable regrade/report, identity-reuse refusal, offline reproduction); crates/opi-eval/tests/rep... | met |  |
| P18-RPT-003 | P18-RPT | crates/opi-eval/tests/bundle_recompute.rs (a17 byte-stable regrade/report, identity-reuse refusal, offline reproduction); crates/opi-eval/tests/rep... | met |  |
| P18-RPT-004 | P18-RPT | crates/opi-eval/tests/bundle_recompute.rs (a17 byte-stable regrade/report, identity-reuse refusal, offline reproduction); crates/opi-eval/tests/rep... | met |  |
| P18-RPT-005 | P18-RPT | crates/opi-eval/tests/bundle_recompute.rs (a17 byte-stable regrade/report, identity-reuse refusal, offline reproduction); crates/opi-eval/tests/rep... | met |  |
| P18-RPT-006 | P18-RPT | crates/opi-eval/tests/bundle_recompute.rs (a17 byte-stable regrade/report, identity-reuse refusal, offline reproduction); crates/opi-eval/tests/rep... | met |  |
| P18-SEC-001 | P18-SEC | crates/opi-eval/tests/authority_boundaries.rs; crates/opi-eval/tests/report_contract.rs (a18 canary leakage blocks sealing/publication) | met |  |
| P18-SEC-002 | P18-SEC | crates/opi-eval/tests/authority_boundaries.rs; crates/opi-eval/tests/report_contract.rs (a18 canary leakage blocks sealing/publication) | met |  |
| P18-SEC-003 | P18-SEC | crates/opi-eval/tests/authority_boundaries.rs; crates/opi-eval/tests/report_contract.rs (a18 canary leakage blocks sealing/publication) | met | P18-AUD-001 |
| P18-SEC-004 | P18-SEC | crates/opi-eval/tests/authority_boundaries.rs; crates/opi-eval/tests/report_contract.rs (a18 canary leakage blocks sealing/publication) | met |  |
| P18-SEC-005 | P18-SEC | crates/opi-eval/tests/authority_boundaries.rs; crates/opi-eval/tests/report_contract.rs (a18 canary leakage blocks sealing/publication) | met |  |
| P18-SEC-006 | P18-SEC | crates/opi-eval/tests/authority_boundaries.rs; crates/opi-eval/tests/report_contract.rs (a18 canary leakage blocks sealing/publication) | met |  |
| P18-MIG-001 | P18-MIG | crates/opi-eval/tests/rollback_contract.rs (3 tests); crates/opi-eval/tests/phase18_acceptance.rs (a19 baseline rejection) | met |  |
| P18-MIG-002 | P18-MIG | crates/opi-eval/tests/rollback_contract.rs (3 tests); crates/opi-eval/tests/phase18_acceptance.rs (a19 baseline rejection) | met |  |
| P18-MIG-003 | P18-MIG | crates/opi-eval/tests/rollback_contract.rs (3 tests); crates/opi-eval/tests/phase18_acceptance.rs (a19 baseline rejection) | met |  |
| P18-MIG-004 | P18-MIG | crates/opi-eval/tests/rollback_contract.rs (3 tests); crates/opi-eval/tests/phase18_acceptance.rs (a19 baseline rejection) | met |  |
| P18-MIG-005 | P18-MIG | crates/opi-eval/tests/rollback_contract.rs (3 tests); crates/opi-eval/tests/phase18_acceptance.rs (a19 baseline rejection) | met |  |
| P18-MIG-006 | P18-MIG | crates/opi-eval/tests/rollback_contract.rs (3 tests); crates/opi-eval/tests/phase18_acceptance.rs (a19 baseline rejection) | met |  |
| P18-PLT-001 | P18-PLT | three-platform CI jobs (fmt/clippy/test/acceptance/doctest/doc) | met |  |
| P18-PLT-002 | P18-PLT | three-platform CI jobs (fmt/clippy/test/acceptance/doctest/doc) | met |  |
| P18-PLT-003 | P18-PLT | three-platform CI jobs (fmt/clippy/test/acceptance/doctest/doc) | met |  |
| P18-PLT-004 | P18-PLT | three-platform CI jobs (fmt/clippy/test/acceptance/doctest/doc) | met |  |
| P18-A01 | P18-A | experiment_contract.rs p18_a01 + cargo tree | met |  |
| P18-A02 | P18-A | agent_integration_conformance.rs (both adapters, same suite) + native smoke artifact (6 trials) | met |  |
| P18-A03 | P18-A | agent conformance cases + seam matrix native/evidence/{manifest,records} | met |  |
| P18-A04 | P18-A | agent conformance + seam matrix native/events/stdout, unknown telemetry typed | met |  |
| P18-A05 | P18-A | agent_integration_conformance.rs invalid-output/parse-failure/missing-terminal/bounded-output truth table | met |  |
| P18-A06 | P18-A | conformance timeout/cancellation cases; lifecycle settlement retention | met |  |
| P18-A07 | P18-A | crash_after_durable_intent_before_settlement_is_effect_unknown unit test | met |  |
| P18-A08 | P18-A | native smoke artifact (task openssl-selfsigned-cert) + benchmark conformance | met |  |
| P18-A09 | P18-A | native smoke artifact + benchmark conformance | met |  |
| P18-A10 | P18-A | native smoke artifact + benchmark conformance | met |  |
| P18-A11 | P18-A | benchmark_integration_conformance.rs incomplete-package/verifier-failure cases (no cached score/heuristic/LLM fallback) | met |  |
| P18-A12 | P18-A | pairing_and_integrity.rs + assembled smoke: 3 tasks x 2 agents = 6 trials, one edge per group, conformance-only label | met |  |
| P18-A13 | P18-A | pairing_and_integrity.rs p18_a13 (missing/duplicate/mismatch typed refusals, exit codes) | met |  |
| P18-A14 | P18-A | pairing_and_integrity.rs p18_a14 (exclusion visible, reclassification changes digest, identity reuse refused) | met |  |
| P18-A15 | P18-A | bundle_recompute.rs p18_a15 (mutation-detected, digest-mismatch, kind/trial named; restore re-verifies) | met |  |
| P18-A16 | P18-A | report_contract.rs p18_a16 (opi measured w/ artifact+digest; pi unknown:pi-usage-not-native, no fabricated parity) | met |  |
| P18-A17 | P18-A | bundle_recompute.rs p18_a17 (byte-stable outputs, no agent/provider, identity-reuse refusal) | met |  |
| P18-A18 | P18-A | report_contract.rs p18_a18 (canary fixture; leaky run fails, seal failed:at-evidence) | met |  |
| P18-A19 | P18-A | phase18_acceptance.rs p18_a19_rejects_hand_authored_or_late_baseline PASSED; before/after runtime re-verify anchored to green three-platform CI rec... | met |  |
| P18-A20 | P18-A | phase18_acceptance.rs p18_a20 (subjects=3, edges=2 resolve; no Opi/pi hard-coding) + seam matrix fourth-benchmark descriptor | met |  |
| P18-A21 | P18-A | phase18_acceptance.rs a21 roadmap inspection | met |  |
| P18-A22 | P18-A | receipt 27/27 jobs; local gates rerun at audit_head | met |  |
| P18-RBK-001 | P18-RBK | crates/opi-eval/tests/rollback_contract.rs (3 tests: coherent removal, artifact immutability, no runtime modification) | met |  |
| P18-RBK-002 | P18-RBK | crates/opi-eval/tests/rollback_contract.rs (3 tests: coherent removal, artifact immutability, no runtime modification) | met |  |
| P18-RBK-003 | P18-RBK | crates/opi-eval/tests/rollback_contract.rs (3 tests: coherent removal, artifact immutability, no runtime modification) | met |  |
| P18-RBK-004 | P18-RBK | crates/opi-eval/tests/rollback_contract.rs (3 tests: coherent removal, artifact immutability, no runtime modification) | met |  |
| P18-RBK-005 | P18-RBK | crates/opi-eval/tests/rollback_contract.rs (3 tests: coherent removal, artifact immutability, no runtime modification) | met |  |

All 131 sealed requirements (109 `P18-*` clauses across 18 families plus 22 `P18-A*` acceptance scenarios) are `met` on current audit_head evidence; the four records linked to advisory findings remain `met` because the findings record residual hygiene, not failed acceptance behavior. No optional obligations exist in the registered tables: every registered clause is `MUST`-scoped.

## Standards Review

- Dependency direction: `cargo tree -p opi-eval --edges normal` contains no `opi-*` package; the reverse scan finds no Opi crate, script, or product workflow depending on or activating `opi-eval`. Workspace placement matches AGENTS.md topology (no internal deps, not in `[workspace.dependencies]`).
- Rust correctness: typed errors (`thiserror`) throughout; `unsafe` confined to `process/tree.rs`, the single documented FFI home behind `#![deny(unsafe_code)]` with a local override; closed enums for lifecycle and classification states; `cargo clippy -p opi-eval --all-targets -- -D warnings`, `cargo fmt --check --all`, and `RUSTDOCFLAGS="-D warnings" cargo doc -p opi-eval --no-deps` are clean at audit_head.
- Documentation lockstep: `scripts/opi-doc-check.py` PASS; crate README/README.zh synchronized; CHANGELOG `[Unreleased]` carries the user-visible Phase 18 entries.
- Residual finding P18-AUD-001 (masked dead surface behind blanket `allow(dead_code)`) and P18-AUD-002 (lib.rs entry-seam enumeration) are recorded below as Minor/advisory standards-residual gaps.

## Spec Review

STRAT-002 seam validation is delivered as specified: the N-harness experiment contract resolves with a frozen digest before any process starts (`validate` prints `digest=4eaf540c... subjects=2 edges=1 trials=2`); pairing/comparability fail closed with typed refusals (A13); the three pinned revisions each carry a complete per-file task-package table with digest-pinned verifier identities (Harbor v0.22.0 by commit, `uv --locked`, DeepSWE pier job results); the artifact-derived seam matrix binds the native smoke (run 33271354427, 6 paired trials, two independently owned verifier contracts) and keeps unproved fields provisional/rejected; offline regrade/report are byte-stable and identity-reuse is refused (A17); reports carry the single `conformance-evidence` classification and never a composite score.

## Security, Invariants, Integration, Test Quality, and Residuals

- Security/authority: no HTTP client dependency exists, so nothing can call a paid provider or leaderboard; the only endpoints are the localhost scripted provider; activation is explicit profile-driven; canary leakage blocks sealing (`failed:at-evidence`, A18); external invocation uses structured `SpawnSpec` argv/cwd/env with typed spawn refusals; the `/bin/sh` bodies in the tree are static hermetic fixture helpers labelled "never the real product".
- Invariants/integration: the seven-phase trial machine enforces ordering (`require(...)`), crash-after-intent classifies `EffectUnknown` and never not-started/success (DUR); bundle writes follow write/file-sync/rename/parent-sync and sealing rejects mutation, unmanifested files, and sidecar drift (BND); scored Agent failures still dispatch the native grader while authority-boundary failures stop every later transition (FAL, via `executed()` counts in `authority_boundaries.rs`).
- Test quality: 148 unit + 52 integration tests pass in the audit export (200 total passed); assertions discriminate behavior (mutation kinds and trial IDs, byte-stability of repeated offline operations, asymmetric telemetry as typed unknowns with no fabricated value, exit-code refusals). One test, `p18_a19_ordinary_opi_minimal_runtime_before_after`, cannot run in a repo-less `git archive` export: it re-derives the pre-Phase baseline through `git archive <commit>`; its baseline-rejection sibling passes, product sources and `Cargo.lock` are byte-identical to the three-platform-green receipt head `0f5a3fa`, and the terminal receipt (27/27 jobs, run 33305179715) is a committed ancestor — recorded as an explicit environment limitation, not a defect.
- Residuals: see P18-AUD-001 — about one hundred never-used items are masked by fourteen blanket `#[allow(dead_code)]` suppressions, led by the fully unconsumed 2238-line `external_lock` module whose lib.rs comment still claims in-crate consumption; lock validation actually runs in the Python verify scripts, so no requirement behavior is lost.

## Minimum-change Conformance

| Task | Recorded surface claim vs current code | Status |
|---|---|---|
| 18.1-18.1x (19 tasks) | recorded reuse_search/surface_necessity/simplification_ceiling match current code: all subsystems stay crate-private behind the single provisional entry seam; concrete `ProcessSupervisor` with no trait/feature flag; no compatibility aliases or dual paths found | conforming |
| 18.2 (external-lock contract) | recorded claim "runner and adapter modules inside this crate consume it" is false at audit_head — the Rust module has no in-crate consumer; enforcement lives in `scripts/verify-phase18-*.py` | drifted (routed as P18-AUD-001) |

Introduced public seam: `crates/opi-eval` library entry — production_consumers: `src/main.rs` (validate, validate_native, run, regrade, report, conformance dispatch) and same-package integration tests (`experiment_contract.rs`, `phase18_acceptance.rs`); nonproduction_consumers: none outside the package; net_deletion: none (additive provisional crate); residual_glue: the masked dead surface recorded in P18-AUD-001.

## Findings

### P18-AUD-001: Masked dead surface in opi-eval behind blanket allow(dead_code)

- Axis: residuals
- Severity: Minor
- Conformance effect: advisory
- Requirement IDs: P18-FAL-001, P18-INT-004, P18-SEC-003
- Claim: At audit_head, opi-eval retains roughly one hundred never-used items - including the complete 2238-line external_lock module with no in-crate consumer, FailureBoundaryCode::{Experiment,PairReport}, RevisionStatus::{NotAdmitted,Retired}, four TaskClassification variants, and integrity/authority helper methods - hidden from cargo clippy -D warnings by fourteen blanket #[allow(dead_code)] suppressions, while lib.rs still documents external_lock as consumed by runner and adapter modules.
- Evidence: `crates/opi-eval/src/external_lock.rs` — No in-crate consumer exists at audit_head; the only reference is a rustdoc link in benchmark/process.rs:56. Lock validation actually runs in scripts/verify-phase18-*.py. `crates/opi-eval/src/lib.rs:32-35` — Module comment claims 'runner and adapter modules inside this crate consume it', which is false at audit_head. `crates/opi-eval/src/failure.rs + integrity.rs + authority.rs + trajectory/mod.rs + process.rs` — Removing the 14 suppressions in a scratch copy surfaced ~100 never-used/never-constructed/never-read items; the tree was restored byte-identical afterwards. `crates/opi-eval/src/lib.rs (13 suppressions) + failure.rs (1)` — cargo clippy -p opi-eval --all-targets -- -D warnings passes only because of the blanket suppressions.
- Refutation attempted: in-crate reference scan found zero non-doc consumers; observable acceptance behavior for FAL/INT/SEC still passes (A13/A14 canary and boundary tests); lock enforcement verified present in the Python scripts; crate is provisional `publish = false` with documented rename latitude — provisional status explains churn but not suppression-masked accumulation with stale contract comments, so the residual claim stands
- Reproduction: `cp crates/opi-eval/src/lib.rs /tmp/b; sed -i '/^#\[allow(dead_code)\]$/d' crates/opi-eval/src/lib.rs crates/opi-eval/src/failure.rs; cargo clippy -p opi-eval --all-targets 2>&1 | grep -c 'never used\|never constructed'; cp /tmp/b crates/opi-eval/src/lib.rs`
- Suggested closure: delete or wire the dead units and drop the blanket suppressions so `clippy -D warnings` polices the crate; correct the external_lock and authority module comments

### P18-AUD-002: lib.rs entry-seam enumeration understates the actual public surface

- Axis: standards
- Severity: Minor
- Conformance effect: advisory
- Requirement IDs: P18-SEAM-001
- Claim: The lib.rs documentation sentence 'The library currently exposes the minimum entry seam required by its same-package CLI and integration tests: experiment::ResolvedExperiment ... and cli::validate' enumerates only two items, while the public seam at audit_head also exposes cli::{validate_native, run, regrade, report, conformance} and the full experiment type set, all consumed by src/main.rs and the integration tests.
- Evidence: `crates/opi-eval/src/lib.rs:14-17` — The colon enumeration names only ResolvedExperiment and cli::validate. `crates/opi-eval/src/main.rs:124-227` — The bin consumes cli::validate_native, cli::run, cli::regrade, cli::report, and cli::conformance, which are therefore necessarily public. `crates/opi-eval/tests/{experiment_contract,phase18_acceptance}.rs` — Integration tests additionally import ControlValue, EXPERIMENT_SCHEMA, ResolveError, and cli.
- Refutation attempted: every pub item was confirmed consumed by `src/main.rs` or the integration tests, so the seam itself is minimal; the defect is the incomplete enumeration in the lib.rs sentence, which survives as documentation drift
- Reproduction: `grep -n 'pub mod\|pub fn\|pub struct\|pub enum' crates/opi-eval/src/cli/mod.rs crates/opi-eval/src/experiment.rs; grep -n 'cli::' crates/opi-eval/src/main.rs`
- Suggested closure: complete the lib.rs entry-seam enumeration to name the full same-package-consumed surface

## Verification Commands

| Command | Result | Requirement/finding |
|---|---|---|
| `python3 scripts/opi-doc-check.py` | PASS | AUTH-004, doc lockstep |
| `cargo tree -p opi-eval --edges normal (grep opi-)` | PASS (no opi-*) | P18-PLC-001, P18-A01 |
| `reverse-dependency grep (all file types)` | PASS (no product activation) | P18-PLC-002, P18-MIG-004, P18-OUT-006 |
| `cargo test -p opi-eval --all-targets` | 200 passed / 1 env-limited (A19 git re-derivation) | all hermetic families |
| `cargo run -p opi-eval -- validate --config .../minimal.toml` | PASS (digest/subjects/edges/trials) | P18-EXP-001 |
| `cargo fmt --check --all; clippy -p opi-eval --all-targets -D warnings; cargo doc -p opi-eval -D warnings; cargo test -p opi-eval --doc` | all PASS | P18-A22 local gates |
| `git merge-base --is-ancestor <receipt/ledger heads> HEAD; ci-receipt inspect` | PASS (27/27 jobs, three platforms) | P18-PLT-001/002, P18-A19, P18-A22 |
| `clippy with the 14 allow(dead_code) removed (scratch copy, restored)` | ~100 dead items surface | P18-AUD-001 |
| `grep pub items vs lib.rs enumeration` | enumeration incomplete | P18-AUD-002 |

## Verdict Rationale

All 131 mandatory sealed requirements are `met` on current audit_head evidence, so no member-blocking condition exists. Two actionable Minor advisory findings (P18-AUD-001 masked dead surface; P18-AUD-002 entry-seam enumeration) make the member verdict PASS-WITH-FINDINGS. Native-smoke and real-process evidence is anchored to the committed artifact-derived seam matrix and the verified three-platform terminal receipt; the single export-environment limitation (A19 runtime re-derivation) is covered by the byte-identical product-source chain to the green receipt head and current reverse-dependency scans.
