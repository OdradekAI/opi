# Phase 18 Audit

**Audit run ID**: `phase18-codex-gpt56-08bc61d-20260830t201731z-b59d9710`
**Audit head**: `08bc61d87146a83ae6e00dc9638a8b8c89fae8d7`
**Reviewer ID**: `codex`
**Model ID**: `gpt56`
**Reviewer identity**: Codex
**Reviewer model ID**: `gpt56`
**Model identity source**: operator-declared
**Independence**: fresh-context-same-family; committed sources were audited without reading active/history audit or remediation conclusions
**Baseline policy**: latest-committed-spec
**Verdict**: FAIL

## Baseline Sources

| Path | SHA-256 | Registration note |
|---|---|---|
| `.opi-impl-state.json` | `9d2ecf977f940f03db3c5d3b17437ad4a3afbca6ad409fcebf306727848a358e` | current committed source |
| `docs/snapshots/phase18/opi-impl-state.json` | `cea5031074ac0d5667357863fbdf03bc76494295a6c38fac304dc1c851d7b42c` | current committed source |
| `docs/opi-spec.md` | `cc7f8898f60c0d8abaa667f4b49b7affc721412e75dd3a67dcde37a783e1bc4c` | current committed source |
| `docs/superpowers/specs/2026-08-25-phase18-independent-cross-agent-eval-seam-validation-design.md` | `43b2759d327cbf0af8d35d4eba50839eef7aac473978b58fcb707b335dad8265` | current committed source |

Registered supplemental-source hashes match the latest committed bytes. The Phase 18 snapshot supplies the immutable task graph; behavior is assessed at audit_head.

## Requirement Conformance

Sealed set: 217 records (`acaeb0ebd0cf9660063062e922ec615e0013b088a29ca3e5f8ad949fc051319a`). States: 179 met, 15 not met, 23 not assessable, 0 partially met.

| Requirement ID | Criterion | Current evidence | State | Finding IDs |
|---|---|---|---|---|
| `P18-AUTH-001` | `docs/superpowers/specs/2026-08-25-phase18-independent-cross-agent-eval-seam-validation-design.md#P18-AUTH-001` | docs/opi-spec.md, docs/superpowers/specs/2026-08-25-phase18-independent-cross-agent-eval-seam-validation-design.md | not-met | P18-AUD-001, P18-AUD-003 |
| `P18-AUTH-002` | `docs/superpowers/specs/2026-08-25-phase18-independent-cross-agent-eval-seam-validation-design.md#P18-AUTH-002` | docs/opi-spec.md, docs/superpowers/specs/2026-08-25-phase18-independent-cross-agent-eval-seam-validation-design.md | met | — |
| `P18-AUTH-003` | `docs/superpowers/specs/2026-08-25-phase18-independent-cross-agent-eval-seam-validation-design.md#P18-AUTH-003` | docs/opi-spec.md, docs/superpowers/specs/2026-08-25-phase18-independent-cross-agent-eval-seam-validation-design.md | met | — |
| `P18-AUTH-004` | `docs/superpowers/specs/2026-08-25-phase18-independent-cross-agent-eval-seam-validation-design.md#P18-AUTH-004` | docs/opi-spec.md, docs/superpowers/specs/2026-08-25-phase18-independent-cross-agent-eval-seam-validation-design.md | met | — |
| `P18-AUTH-005` | `docs/superpowers/specs/2026-08-25-phase18-independent-cross-agent-eval-seam-validation-design.md#P18-AUTH-005` | docs/opi-spec.md, docs/superpowers/specs/2026-08-25-phase18-independent-cross-agent-eval-seam-validation-design.md | met | — |
| `P18-OUT-001` | `docs/superpowers/specs/2026-08-25-phase18-independent-cross-agent-eval-seam-validation-design.md#P18-OUT-001` | crates/opi-eval/src/runner/experiment.rs, crates/opi-eval/src/bundle/mod.rs | not-assessable | P18-AUD-006 |
| `P18-OUT-002` | `docs/superpowers/specs/2026-08-25-phase18-independent-cross-agent-eval-seam-validation-design.md#P18-OUT-002` | crates/opi-eval/src/runner/experiment.rs, crates/opi-eval/src/bundle/mod.rs | not-assessable | P18-AUD-006 |
| `P18-OUT-003` | `docs/superpowers/specs/2026-08-25-phase18-independent-cross-agent-eval-seam-validation-design.md#P18-OUT-003` | crates/opi-eval/src/runner/experiment.rs, crates/opi-eval/src/bundle/mod.rs | met | — |
| `P18-OUT-004` | `docs/superpowers/specs/2026-08-25-phase18-independent-cross-agent-eval-seam-validation-design.md#P18-OUT-004` | crates/opi-eval/src/runner/experiment.rs, crates/opi-eval/src/bundle/mod.rs | met | — |
| `P18-OUT-005` | `docs/superpowers/specs/2026-08-25-phase18-independent-cross-agent-eval-seam-validation-design.md#P18-OUT-005` | crates/opi-eval/src/runner/experiment.rs, crates/opi-eval/src/bundle/mod.rs | not-assessable | P18-AUD-006 |
| `P18-OUT-006` | `docs/superpowers/specs/2026-08-25-phase18-independent-cross-agent-eval-seam-validation-design.md#P18-OUT-006` | crates/opi-eval/src/runner/experiment.rs, crates/opi-eval/src/bundle/mod.rs | met | — |
| `P18-PLC-001` | `docs/superpowers/specs/2026-08-25-phase18-independent-cross-agent-eval-seam-validation-design.md#P18-PLC-001` | Cargo.toml, crates/opi-eval/Cargo.toml | met | — |
| `P18-PLC-002` | `docs/superpowers/specs/2026-08-25-phase18-independent-cross-agent-eval-seam-validation-design.md#P18-PLC-002` | Cargo.toml, crates/opi-eval/Cargo.toml | met | — |
| `P18-PLC-003` | `docs/superpowers/specs/2026-08-25-phase18-independent-cross-agent-eval-seam-validation-design.md#P18-PLC-003` | Cargo.toml, crates/opi-eval/Cargo.toml | met | — |
| `P18-PLC-004` | `docs/superpowers/specs/2026-08-25-phase18-independent-cross-agent-eval-seam-validation-design.md#P18-PLC-004` | Cargo.toml, crates/opi-eval/Cargo.toml | met | — |
| `P18-PLC-005` | `docs/superpowers/specs/2026-08-25-phase18-independent-cross-agent-eval-seam-validation-design.md#P18-PLC-005` | Cargo.toml, crates/opi-eval/Cargo.toml | met | — |
| `P18-PLC-006` | `docs/superpowers/specs/2026-08-25-phase18-independent-cross-agent-eval-seam-validation-design.md#P18-PLC-006` | Cargo.toml, crates/opi-eval/Cargo.toml | met | — |
| `P18-SEAM-001` | `docs/superpowers/specs/2026-08-25-phase18-independent-cross-agent-eval-seam-validation-design.md#P18-SEAM-001` | crates/opi-eval/src/lib.rs, crates/opi-eval/docs/seam-evidence-matrix.md | met | — |
| `P18-SEAM-002` | `docs/superpowers/specs/2026-08-25-phase18-independent-cross-agent-eval-seam-validation-design.md#P18-SEAM-002` | crates/opi-eval/src/lib.rs, crates/opi-eval/docs/seam-evidence-matrix.md | met | — |
| `P18-SEAM-003` | `docs/superpowers/specs/2026-08-25-phase18-independent-cross-agent-eval-seam-validation-design.md#P18-SEAM-003` | crates/opi-eval/src/lib.rs, crates/opi-eval/docs/seam-evidence-matrix.md | met | — |
| `P18-SEAM-004` | `docs/superpowers/specs/2026-08-25-phase18-independent-cross-agent-eval-seam-validation-design.md#P18-SEAM-004` | crates/opi-eval/src/lib.rs, crates/opi-eval/docs/seam-evidence-matrix.md | met | — |
| `P18-SEAM-005` | `docs/superpowers/specs/2026-08-25-phase18-independent-cross-agent-eval-seam-validation-design.md#P18-SEAM-005` | crates/opi-eval/src/lib.rs, crates/opi-eval/docs/seam-evidence-matrix.md | met | — |
| `P18-EXP-001` | `docs/superpowers/specs/2026-08-25-phase18-independent-cross-agent-eval-seam-validation-design.md#P18-EXP-001` | crates/opi-eval/src/experiment.rs, crates/opi-eval/src/comparison.rs | met | — |
| `P18-EXP-002` | `docs/superpowers/specs/2026-08-25-phase18-independent-cross-agent-eval-seam-validation-design.md#P18-EXP-002` | crates/opi-eval/src/experiment.rs, crates/opi-eval/src/comparison.rs | met | — |
| `P18-EXP-003` | `docs/superpowers/specs/2026-08-25-phase18-independent-cross-agent-eval-seam-validation-design.md#P18-EXP-003` | crates/opi-eval/src/experiment.rs, crates/opi-eval/src/comparison.rs | met | — |
| `P18-EXP-004` | `docs/superpowers/specs/2026-08-25-phase18-independent-cross-agent-eval-seam-validation-design.md#P18-EXP-004` | crates/opi-eval/src/experiment.rs, crates/opi-eval/src/comparison.rs | met | — |
| `P18-EXP-005` | `docs/superpowers/specs/2026-08-25-phase18-independent-cross-agent-eval-seam-validation-design.md#P18-EXP-005` | crates/opi-eval/src/experiment.rs, crates/opi-eval/src/comparison.rs | met | — |
| `P18-EXP-006` | `docs/superpowers/specs/2026-08-25-phase18-independent-cross-agent-eval-seam-validation-design.md#P18-EXP-006` | crates/opi-eval/src/experiment.rs, crates/opi-eval/src/comparison.rs | met | — |
| `P18-EXP-007` | `docs/superpowers/specs/2026-08-25-phase18-independent-cross-agent-eval-seam-validation-design.md#P18-EXP-007` | crates/opi-eval/src/experiment.rs, crates/opi-eval/src/comparison.rs | not-assessable | P18-AUD-006 |
| `P18-EXP-008` | `docs/superpowers/specs/2026-08-25-phase18-independent-cross-agent-eval-seam-validation-design.md#P18-EXP-008` | crates/opi-eval/src/experiment.rs, crates/opi-eval/src/comparison.rs | met | — |
| `P18-DUR-001` | `docs/superpowers/specs/2026-08-25-phase18-independent-cross-agent-eval-seam-validation-design.md#P18-DUR-001` | crates/opi-eval/src/bundle/mod.rs, crates/opi-eval/src/runner/lifecycle.rs | not-met | P18-AUD-002 |
| `P18-DUR-002` | `docs/superpowers/specs/2026-08-25-phase18-independent-cross-agent-eval-seam-validation-design.md#P18-DUR-002` | crates/opi-eval/src/bundle/mod.rs, crates/opi-eval/src/runner/lifecycle.rs | met | — |
| `P18-DUR-003` | `docs/superpowers/specs/2026-08-25-phase18-independent-cross-agent-eval-seam-validation-design.md#P18-DUR-003` | crates/opi-eval/src/bundle/mod.rs, crates/opi-eval/src/runner/lifecycle.rs | met | — |
| `P18-DUR-004` | `docs/superpowers/specs/2026-08-25-phase18-independent-cross-agent-eval-seam-validation-design.md#P18-DUR-004` | crates/opi-eval/src/bundle/mod.rs, crates/opi-eval/src/runner/lifecycle.rs | met | — |
| `P18-DUR-005` | `docs/superpowers/specs/2026-08-25-phase18-independent-cross-agent-eval-seam-validation-design.md#P18-DUR-005` | crates/opi-eval/src/bundle/mod.rs, crates/opi-eval/src/runner/lifecycle.rs | met | — |
| `P18-AGT-001` | `docs/superpowers/specs/2026-08-25-phase18-independent-cross-agent-eval-seam-validation-design.md#P18-AGT-001` | crates/opi-eval/src/agent/process.rs, crates/opi-eval/src/agent/opi.rs | met | — |
| `P18-AGT-002` | `docs/superpowers/specs/2026-08-25-phase18-independent-cross-agent-eval-seam-validation-design.md#P18-AGT-002` | crates/opi-eval/src/agent/process.rs, crates/opi-eval/src/agent/opi.rs | not-assessable | P18-AUD-006 |
| `P18-AGT-003` | `docs/superpowers/specs/2026-08-25-phase18-independent-cross-agent-eval-seam-validation-design.md#P18-AGT-003` | crates/opi-eval/src/agent/process.rs, crates/opi-eval/src/agent/opi.rs | not-met | P18-AUD-003 |
| `P18-AGT-004` | `docs/superpowers/specs/2026-08-25-phase18-independent-cross-agent-eval-seam-validation-design.md#P18-AGT-004` | crates/opi-eval/src/agent/process.rs, crates/opi-eval/src/agent/opi.rs | not-met | P18-AUD-003 |
| `P18-AGT-005` | `docs/superpowers/specs/2026-08-25-phase18-independent-cross-agent-eval-seam-validation-design.md#P18-AGT-005` | crates/opi-eval/src/agent/process.rs, crates/opi-eval/src/agent/opi.rs | met | — |
| `P18-AGT-006` | `docs/superpowers/specs/2026-08-25-phase18-independent-cross-agent-eval-seam-validation-design.md#P18-AGT-006` | crates/opi-eval/src/agent/process.rs, crates/opi-eval/src/agent/opi.rs | met | — |
| `P18-AGT-007` | `docs/superpowers/specs/2026-08-25-phase18-independent-cross-agent-eval-seam-validation-design.md#P18-AGT-007` | crates/opi-eval/src/agent/process.rs, crates/opi-eval/src/agent/opi.rs | met | — |
| `P18-AGT-008` | `docs/superpowers/specs/2026-08-25-phase18-independent-cross-agent-eval-seam-validation-design.md#P18-AGT-008` | crates/opi-eval/src/agent/process.rs, crates/opi-eval/src/agent/opi.rs | met | — |
| `P18-AGT-009` | `docs/superpowers/specs/2026-08-25-phase18-independent-cross-agent-eval-seam-validation-design.md#P18-AGT-009` | crates/opi-eval/src/agent/process.rs, crates/opi-eval/src/agent/opi.rs | met | — |
| `P18-BMK-001` | `docs/superpowers/specs/2026-08-25-phase18-independent-cross-agent-eval-seam-validation-design.md#P18-BMK-001` | crates/opi-eval/src/benchmark/process.rs, crates/opi-eval/src/benchmark/terminal_bench_21.rs | not-assessable | P18-AUD-006 |
| `P18-BMK-002` | `docs/superpowers/specs/2026-08-25-phase18-independent-cross-agent-eval-seam-validation-design.md#P18-BMK-002` | crates/opi-eval/src/benchmark/process.rs, crates/opi-eval/src/benchmark/terminal_bench_21.rs | met | — |
| `P18-BMK-003` | `docs/superpowers/specs/2026-08-25-phase18-independent-cross-agent-eval-seam-validation-design.md#P18-BMK-003` | crates/opi-eval/src/benchmark/process.rs, crates/opi-eval/src/benchmark/terminal_bench_21.rs | not-assessable | P18-AUD-006 |
| `P18-BMK-004` | `docs/superpowers/specs/2026-08-25-phase18-independent-cross-agent-eval-seam-validation-design.md#P18-BMK-004` | crates/opi-eval/src/benchmark/process.rs, crates/opi-eval/src/benchmark/terminal_bench_21.rs | met | — |
| `P18-BMK-005` | `docs/superpowers/specs/2026-08-25-phase18-independent-cross-agent-eval-seam-validation-design.md#P18-BMK-005` | crates/opi-eval/src/benchmark/process.rs, crates/opi-eval/src/benchmark/terminal_bench_21.rs | not-assessable | P18-AUD-006 |
| `P18-BMK-006` | `docs/superpowers/specs/2026-08-25-phase18-independent-cross-agent-eval-seam-validation-design.md#P18-BMK-006` | crates/opi-eval/src/benchmark/process.rs, crates/opi-eval/src/benchmark/terminal_bench_21.rs | met | — |
| `P18-BMK-007` | `docs/superpowers/specs/2026-08-25-phase18-independent-cross-agent-eval-seam-validation-design.md#P18-BMK-007` | crates/opi-eval/src/benchmark/process.rs, crates/opi-eval/src/benchmark/terminal_bench_21.rs | met | — |
| `P18-BMK-008` | `docs/superpowers/specs/2026-08-25-phase18-independent-cross-agent-eval-seam-validation-design.md#P18-BMK-008` | crates/opi-eval/src/benchmark/process.rs, crates/opi-eval/src/benchmark/terminal_bench_21.rs | met | — |
| `P18-BMK-009` | `docs/superpowers/specs/2026-08-25-phase18-independent-cross-agent-eval-seam-validation-design.md#P18-BMK-009` | crates/opi-eval/src/benchmark/process.rs, crates/opi-eval/src/benchmark/terminal_bench_21.rs | not-assessable | P18-AUD-006 |
| `P18-BMK-010` | `docs/superpowers/specs/2026-08-25-phase18-independent-cross-agent-eval-seam-validation-design.md#P18-BMK-010` | crates/opi-eval/src/benchmark/process.rs, crates/opi-eval/src/benchmark/terminal_bench_21.rs | met | — |
| `P18-BMK-011` | `docs/superpowers/specs/2026-08-25-phase18-independent-cross-agent-eval-seam-validation-design.md#P18-BMK-011` | crates/opi-eval/src/benchmark/process.rs, crates/opi-eval/src/benchmark/terminal_bench_21.rs | not-assessable | P18-AUD-006 |
| `P18-RDM-001` | `docs/superpowers/specs/2026-08-25-phase18-independent-cross-agent-eval-seam-validation-design.md#P18-RDM-001` | docs/superpowers/specs/2026-08-25-phase18-independent-cross-agent-eval-seam-validation-design.md, crates/opi-eval/docs/seam-evidence-matrix.md | met | — |
| `P18-RDM-002` | `docs/superpowers/specs/2026-08-25-phase18-independent-cross-agent-eval-seam-validation-design.md#P18-RDM-002` | docs/superpowers/specs/2026-08-25-phase18-independent-cross-agent-eval-seam-validation-design.md, crates/opi-eval/docs/seam-evidence-matrix.md | met | — |
| `P18-RDM-003` | `docs/superpowers/specs/2026-08-25-phase18-independent-cross-agent-eval-seam-validation-design.md#P18-RDM-003` | docs/superpowers/specs/2026-08-25-phase18-independent-cross-agent-eval-seam-validation-design.md, crates/opi-eval/docs/seam-evidence-matrix.md | met | — |
| `P18-RDM-004` | `docs/superpowers/specs/2026-08-25-phase18-independent-cross-agent-eval-seam-validation-design.md#P18-RDM-004` | docs/superpowers/specs/2026-08-25-phase18-independent-cross-agent-eval-seam-validation-design.md, crates/opi-eval/docs/seam-evidence-matrix.md | met | — |
| `P18-RDM-005` | `docs/superpowers/specs/2026-08-25-phase18-independent-cross-agent-eval-seam-validation-design.md#P18-RDM-005` | docs/superpowers/specs/2026-08-25-phase18-independent-cross-agent-eval-seam-validation-design.md, crates/opi-eval/docs/seam-evidence-matrix.md | met | — |
| `P18-RDM-006` | `docs/superpowers/specs/2026-08-25-phase18-independent-cross-agent-eval-seam-validation-design.md#P18-RDM-006` | docs/superpowers/specs/2026-08-25-phase18-independent-cross-agent-eval-seam-validation-design.md, crates/opi-eval/docs/seam-evidence-matrix.md | met | — |
| `P18-INT-001` | `docs/superpowers/specs/2026-08-25-phase18-independent-cross-agent-eval-seam-validation-design.md#P18-INT-001` | crates/opi-eval/src/integrity.rs, crates/opi-eval/src/comparison.rs | met | — |
| `P18-INT-002` | `docs/superpowers/specs/2026-08-25-phase18-independent-cross-agent-eval-seam-validation-design.md#P18-INT-002` | crates/opi-eval/src/integrity.rs, crates/opi-eval/src/comparison.rs | met | — |
| `P18-INT-003` | `docs/superpowers/specs/2026-08-25-phase18-independent-cross-agent-eval-seam-validation-design.md#P18-INT-003` | crates/opi-eval/src/integrity.rs, crates/opi-eval/src/comparison.rs | met | — |
| `P18-INT-004` | `docs/superpowers/specs/2026-08-25-phase18-independent-cross-agent-eval-seam-validation-design.md#P18-INT-004` | crates/opi-eval/src/integrity.rs, crates/opi-eval/src/comparison.rs | met | — |
| `P18-INT-005` | `docs/superpowers/specs/2026-08-25-phase18-independent-cross-agent-eval-seam-validation-design.md#P18-INT-005` | crates/opi-eval/src/integrity.rs, crates/opi-eval/src/comparison.rs | met | — |
| `P18-BND-001` | `docs/superpowers/specs/2026-08-25-phase18-independent-cross-agent-eval-seam-validation-design.md#P18-BND-001` | crates/opi-eval/src/bundle/mod.rs, crates/opi-eval/src/regrade.rs | met | — |
| `P18-BND-002` | `docs/superpowers/specs/2026-08-25-phase18-independent-cross-agent-eval-seam-validation-design.md#P18-BND-002` | crates/opi-eval/src/bundle/mod.rs, crates/opi-eval/src/regrade.rs | not-met | P18-AUD-001 |
| `P18-BND-003` | `docs/superpowers/specs/2026-08-25-phase18-independent-cross-agent-eval-seam-validation-design.md#P18-BND-003` | crates/opi-eval/src/bundle/mod.rs, crates/opi-eval/src/regrade.rs | met | — |
| `P18-BND-004` | `docs/superpowers/specs/2026-08-25-phase18-independent-cross-agent-eval-seam-validation-design.md#P18-BND-004` | crates/opi-eval/src/bundle/mod.rs, crates/opi-eval/src/regrade.rs | not-met | P18-AUD-005 |
| `P18-BND-005` | `docs/superpowers/specs/2026-08-25-phase18-independent-cross-agent-eval-seam-validation-design.md#P18-BND-005` | crates/opi-eval/src/bundle/mod.rs, crates/opi-eval/src/regrade.rs | met | — |
| `P18-BND-006` | `docs/superpowers/specs/2026-08-25-phase18-independent-cross-agent-eval-seam-validation-design.md#P18-BND-006` | crates/opi-eval/src/bundle/mod.rs, crates/opi-eval/src/regrade.rs | met | — |
| `P18-TRJ-001` | `docs/superpowers/specs/2026-08-25-phase18-independent-cross-agent-eval-seam-validation-design.md#P18-TRJ-001` | crates/opi-eval/src/trajectory/mod.rs, crates/opi-eval/src/runner/experiment.rs | met | — |
| `P18-TRJ-002` | `docs/superpowers/specs/2026-08-25-phase18-independent-cross-agent-eval-seam-validation-design.md#P18-TRJ-002` | crates/opi-eval/src/trajectory/mod.rs, crates/opi-eval/src/runner/experiment.rs | met | — |
| `P18-TRJ-003` | `docs/superpowers/specs/2026-08-25-phase18-independent-cross-agent-eval-seam-validation-design.md#P18-TRJ-003` | crates/opi-eval/src/trajectory/mod.rs, crates/opi-eval/src/runner/experiment.rs | met | — |
| `P18-TRJ-004` | `docs/superpowers/specs/2026-08-25-phase18-independent-cross-agent-eval-seam-validation-design.md#P18-TRJ-004` | crates/opi-eval/src/trajectory/mod.rs, crates/opi-eval/src/runner/experiment.rs | met | — |
| `P18-TRJ-005` | `docs/superpowers/specs/2026-08-25-phase18-independent-cross-agent-eval-seam-validation-design.md#P18-TRJ-005` | crates/opi-eval/src/trajectory/mod.rs, crates/opi-eval/src/runner/experiment.rs | met | — |
| `P18-FAL-001` | `docs/superpowers/specs/2026-08-25-phase18-independent-cross-agent-eval-seam-validation-design.md#P18-FAL-001` | crates/opi-eval/src/failure.rs, crates/opi-eval/src/authority.rs | met | — |
| `P18-FAL-002` | `docs/superpowers/specs/2026-08-25-phase18-independent-cross-agent-eval-seam-validation-design.md#P18-FAL-002` | crates/opi-eval/src/failure.rs, crates/opi-eval/src/authority.rs | met | — |
| `P18-FAL-003` | `docs/superpowers/specs/2026-08-25-phase18-independent-cross-agent-eval-seam-validation-design.md#P18-FAL-003` | crates/opi-eval/src/failure.rs, crates/opi-eval/src/authority.rs | met | — |
| `P18-FAL-004` | `docs/superpowers/specs/2026-08-25-phase18-independent-cross-agent-eval-seam-validation-design.md#P18-FAL-004` | crates/opi-eval/src/failure.rs, crates/opi-eval/src/authority.rs | met | — |
| `P18-FAL-005` | `docs/superpowers/specs/2026-08-25-phase18-independent-cross-agent-eval-seam-validation-design.md#P18-FAL-005` | crates/opi-eval/src/failure.rs, crates/opi-eval/src/authority.rs | met | — |
| `P18-RPT-001` | `docs/superpowers/specs/2026-08-25-phase18-independent-cross-agent-eval-seam-validation-design.md#P18-RPT-001` | crates/opi-eval/src/regrade.rs, crates/opi-eval/src/report.rs | not-met | P18-AUD-005 |
| `P18-RPT-002` | `docs/superpowers/specs/2026-08-25-phase18-independent-cross-agent-eval-seam-validation-design.md#P18-RPT-002` | crates/opi-eval/src/regrade.rs, crates/opi-eval/src/report.rs | met | — |
| `P18-RPT-003` | `docs/superpowers/specs/2026-08-25-phase18-independent-cross-agent-eval-seam-validation-design.md#P18-RPT-003` | crates/opi-eval/src/regrade.rs, crates/opi-eval/src/report.rs | met | — |
| `P18-RPT-004` | `docs/superpowers/specs/2026-08-25-phase18-independent-cross-agent-eval-seam-validation-design.md#P18-RPT-004` | crates/opi-eval/src/regrade.rs, crates/opi-eval/src/report.rs | met | — |
| `P18-RPT-005` | `docs/superpowers/specs/2026-08-25-phase18-independent-cross-agent-eval-seam-validation-design.md#P18-RPT-005` | crates/opi-eval/src/regrade.rs, crates/opi-eval/src/report.rs | met | — |
| `P18-RPT-006` | `docs/superpowers/specs/2026-08-25-phase18-independent-cross-agent-eval-seam-validation-design.md#P18-RPT-006` | crates/opi-eval/src/regrade.rs, crates/opi-eval/src/report.rs | met | — |
| `P18-SEC-001` | `docs/superpowers/specs/2026-08-25-phase18-independent-cross-agent-eval-seam-validation-design.md#P18-SEC-001` | crates/opi-eval/src/process.rs, crates/opi-eval/src/process/tree.rs | met | — |
| `P18-SEC-002` | `docs/superpowers/specs/2026-08-25-phase18-independent-cross-agent-eval-seam-validation-design.md#P18-SEC-002` | crates/opi-eval/src/process.rs, crates/opi-eval/src/process/tree.rs | not-met | P18-AUD-001 |
| `P18-SEC-003` | `docs/superpowers/specs/2026-08-25-phase18-independent-cross-agent-eval-seam-validation-design.md#P18-SEC-003` | crates/opi-eval/src/process.rs, crates/opi-eval/src/process/tree.rs | met | — |
| `P18-SEC-004` | `docs/superpowers/specs/2026-08-25-phase18-independent-cross-agent-eval-seam-validation-design.md#P18-SEC-004` | crates/opi-eval/src/process.rs, crates/opi-eval/src/process/tree.rs | met | — |
| `P18-SEC-005` | `docs/superpowers/specs/2026-08-25-phase18-independent-cross-agent-eval-seam-validation-design.md#P18-SEC-005` | crates/opi-eval/src/process.rs, crates/opi-eval/src/process/tree.rs | met | — |
| `P18-SEC-006` | `docs/superpowers/specs/2026-08-25-phase18-independent-cross-agent-eval-seam-validation-design.md#P18-SEC-006` | crates/opi-eval/src/process.rs, crates/opi-eval/src/process/tree.rs | met | — |
| `P18-MIG-001` | `docs/superpowers/specs/2026-08-25-phase18-independent-cross-agent-eval-seam-validation-design.md#P18-MIG-001` | crates/opi-eval/src/agent/opi.rs, crates/opi-eval/src/agent/pi.rs | met | — |
| `P18-MIG-002` | `docs/superpowers/specs/2026-08-25-phase18-independent-cross-agent-eval-seam-validation-design.md#P18-MIG-002` | crates/opi-eval/src/agent/opi.rs, crates/opi-eval/src/agent/pi.rs | met | — |
| `P18-MIG-003` | `docs/superpowers/specs/2026-08-25-phase18-independent-cross-agent-eval-seam-validation-design.md#P18-MIG-003` | crates/opi-eval/src/agent/opi.rs, crates/opi-eval/src/agent/pi.rs | met | — |
| `P18-MIG-004` | `docs/superpowers/specs/2026-08-25-phase18-independent-cross-agent-eval-seam-validation-design.md#P18-MIG-004` | crates/opi-eval/src/agent/opi.rs, crates/opi-eval/src/agent/pi.rs | met | — |
| `P18-MIG-005` | `docs/superpowers/specs/2026-08-25-phase18-independent-cross-agent-eval-seam-validation-design.md#P18-MIG-005` | crates/opi-eval/src/agent/opi.rs, crates/opi-eval/src/agent/pi.rs | met | — |
| `P18-MIG-006` | `docs/superpowers/specs/2026-08-25-phase18-independent-cross-agent-eval-seam-validation-design.md#P18-MIG-006` | crates/opi-eval/src/agent/opi.rs, crates/opi-eval/src/agent/pi.rs | met | — |
| `P18-PLT-001` | `docs/superpowers/specs/2026-08-25-phase18-independent-cross-agent-eval-seam-validation-design.md#P18-PLT-001` | crates/opi-eval/src/process.rs, crates/opi-eval/src/process/tree.rs | not-assessable | P18-AUD-006 |
| `P18-PLT-002` | `docs/superpowers/specs/2026-08-25-phase18-independent-cross-agent-eval-seam-validation-design.md#P18-PLT-002` | crates/opi-eval/src/process.rs, crates/opi-eval/src/process/tree.rs | not-assessable | P18-AUD-006 |
| `P18-PLT-003` | `docs/superpowers/specs/2026-08-25-phase18-independent-cross-agent-eval-seam-validation-design.md#P18-PLT-003` | crates/opi-eval/src/process.rs, crates/opi-eval/src/process/tree.rs | met | — |
| `P18-PLT-004` | `docs/superpowers/specs/2026-08-25-phase18-independent-cross-agent-eval-seam-validation-design.md#P18-PLT-004` | crates/opi-eval/src/process.rs, crates/opi-eval/src/process/tree.rs | met | — |
| `P18-A01` | `docs/superpowers/specs/2026-08-25-phase18-independent-cross-agent-eval-seam-validation-design.md#P18-A01` | crates/opi-eval/src/runner/experiment.rs, crates/opi-eval/tests/phase18_acceptance.rs | met | — |
| `P18-A02` | `docs/superpowers/specs/2026-08-25-phase18-independent-cross-agent-eval-seam-validation-design.md#P18-A02` | crates/opi-eval/src/runner/experiment.rs, crates/opi-eval/tests/phase18_acceptance.rs | not-assessable | P18-AUD-006 |
| `P18-A03` | `docs/superpowers/specs/2026-08-25-phase18-independent-cross-agent-eval-seam-validation-design.md#P18-A03` | crates/opi-eval/src/runner/experiment.rs, crates/opi-eval/tests/phase18_acceptance.rs | not-met | P18-AUD-003, P18-AUD-006 |
| `P18-A04` | `docs/superpowers/specs/2026-08-25-phase18-independent-cross-agent-eval-seam-validation-design.md#P18-A04` | crates/opi-eval/src/runner/experiment.rs, crates/opi-eval/tests/phase18_acceptance.rs | not-assessable | P18-AUD-006 |
| `P18-A05` | `docs/superpowers/specs/2026-08-25-phase18-independent-cross-agent-eval-seam-validation-design.md#P18-A05` | crates/opi-eval/src/runner/experiment.rs, crates/opi-eval/tests/phase18_acceptance.rs | not-met | P18-AUD-003 |
| `P18-A06` | `docs/superpowers/specs/2026-08-25-phase18-independent-cross-agent-eval-seam-validation-design.md#P18-A06` | crates/opi-eval/src/runner/experiment.rs, crates/opi-eval/tests/phase18_acceptance.rs | met | — |
| `P18-A07` | `docs/superpowers/specs/2026-08-25-phase18-independent-cross-agent-eval-seam-validation-design.md#P18-A07` | crates/opi-eval/src/runner/experiment.rs, crates/opi-eval/tests/phase18_acceptance.rs | met | — |
| `P18-A08` | `docs/superpowers/specs/2026-08-25-phase18-independent-cross-agent-eval-seam-validation-design.md#P18-A08` | crates/opi-eval/src/runner/experiment.rs, crates/opi-eval/tests/phase18_acceptance.rs | not-assessable | P18-AUD-006 |
| `P18-A09` | `docs/superpowers/specs/2026-08-25-phase18-independent-cross-agent-eval-seam-validation-design.md#P18-A09` | crates/opi-eval/src/runner/experiment.rs, crates/opi-eval/tests/phase18_acceptance.rs | not-assessable | P18-AUD-006 |
| `P18-A10` | `docs/superpowers/specs/2026-08-25-phase18-independent-cross-agent-eval-seam-validation-design.md#P18-A10` | crates/opi-eval/src/runner/experiment.rs, crates/opi-eval/tests/phase18_acceptance.rs | not-assessable | P18-AUD-006 |
| `P18-A11` | `docs/superpowers/specs/2026-08-25-phase18-independent-cross-agent-eval-seam-validation-design.md#P18-A11` | crates/opi-eval/src/runner/experiment.rs, crates/opi-eval/tests/phase18_acceptance.rs | met | — |
| `P18-A12` | `docs/superpowers/specs/2026-08-25-phase18-independent-cross-agent-eval-seam-validation-design.md#P18-A12` | crates/opi-eval/src/runner/experiment.rs, crates/opi-eval/tests/phase18_acceptance.rs | not-assessable | P18-AUD-006 |
| `P18-A13` | `docs/superpowers/specs/2026-08-25-phase18-independent-cross-agent-eval-seam-validation-design.md#P18-A13` | crates/opi-eval/src/runner/experiment.rs, crates/opi-eval/tests/phase18_acceptance.rs | met | — |
| `P18-A14` | `docs/superpowers/specs/2026-08-25-phase18-independent-cross-agent-eval-seam-validation-design.md#P18-A14` | crates/opi-eval/src/runner/experiment.rs, crates/opi-eval/tests/phase18_acceptance.rs | met | — |
| `P18-A15` | `docs/superpowers/specs/2026-08-25-phase18-independent-cross-agent-eval-seam-validation-design.md#P18-A15` | crates/opi-eval/src/runner/experiment.rs, crates/opi-eval/tests/phase18_acceptance.rs | met | — |
| `P18-A16` | `docs/superpowers/specs/2026-08-25-phase18-independent-cross-agent-eval-seam-validation-design.md#P18-A16` | crates/opi-eval/src/runner/experiment.rs, crates/opi-eval/tests/phase18_acceptance.rs | met | — |
| `P18-A17` | `docs/superpowers/specs/2026-08-25-phase18-independent-cross-agent-eval-seam-validation-design.md#P18-A17` | crates/opi-eval/src/runner/experiment.rs, crates/opi-eval/tests/phase18_acceptance.rs | met | — |
| `P18-A18` | `docs/superpowers/specs/2026-08-25-phase18-independent-cross-agent-eval-seam-validation-design.md#P18-A18` | crates/opi-eval/src/runner/experiment.rs, crates/opi-eval/tests/phase18_acceptance.rs | met | — |
| `P18-A19` | `docs/superpowers/specs/2026-08-25-phase18-independent-cross-agent-eval-seam-validation-design.md#P18-A19` | crates/opi-eval/src/runner/experiment.rs, crates/opi-eval/tests/phase18_acceptance.rs | met | — |
| `P18-A20` | `docs/superpowers/specs/2026-08-25-phase18-independent-cross-agent-eval-seam-validation-design.md#P18-A20` | crates/opi-eval/src/runner/experiment.rs, crates/opi-eval/tests/phase18_acceptance.rs | not-assessable | P18-AUD-006 |
| `P18-A21` | `docs/superpowers/specs/2026-08-25-phase18-independent-cross-agent-eval-seam-validation-design.md#P18-A21` | crates/opi-eval/src/runner/experiment.rs, crates/opi-eval/tests/phase18_acceptance.rs | met | — |
| `P18-A22` | `docs/superpowers/specs/2026-08-25-phase18-independent-cross-agent-eval-seam-validation-design.md#P18-A22` | crates/opi-eval/src/runner/experiment.rs, crates/opi-eval/tests/phase18_acceptance.rs | not-assessable | P18-AUD-006 |
| `P18-RBK-001` | `docs/superpowers/specs/2026-08-25-phase18-independent-cross-agent-eval-seam-validation-design.md#P18-RBK-001` | crates/opi-eval/tests/rollback_contract.rs, crates/opi-eval/Cargo.toml | met | — |
| `P18-RBK-002` | `docs/superpowers/specs/2026-08-25-phase18-independent-cross-agent-eval-seam-validation-design.md#P18-RBK-002` | crates/opi-eval/tests/rollback_contract.rs, crates/opi-eval/Cargo.toml | met | — |
| `P18-RBK-003` | `docs/superpowers/specs/2026-08-25-phase18-independent-cross-agent-eval-seam-validation-design.md#P18-RBK-003` | crates/opi-eval/tests/rollback_contract.rs, crates/opi-eval/Cargo.toml | met | — |
| `P18-RBK-004` | `docs/superpowers/specs/2026-08-25-phase18-independent-cross-agent-eval-seam-validation-design.md#P18-RBK-004` | crates/opi-eval/tests/rollback_contract.rs, crates/opi-eval/Cargo.toml | met | — |
| `P18-RBK-005` | `docs/superpowers/specs/2026-08-25-phase18-independent-cross-agent-eval-seam-validation-design.md#P18-RBK-005` | crates/opi-eval/tests/rollback_contract.rs, crates/opi-eval/Cargo.toml | met | — |
| `GOAL-001` | `docs/opi-spec.md#GOAL-001` | docs/opi-spec.md, docs/superpowers/specs/2026-08-25-phase18-independent-cross-agent-eval-seam-validation-design.md | met | — |
| `GOAL-004` | `docs/opi-spec.md#GOAL-004` | docs/opi-spec.md, docs/superpowers/specs/2026-08-25-phase18-independent-cross-agent-eval-seam-validation-design.md | met | — |
| `PRIN-001` | `docs/opi-spec.md#PRIN-001` | docs/opi-spec.md, docs/superpowers/specs/2026-08-25-phase18-independent-cross-agent-eval-seam-validation-design.md | met | — |
| `PRIN-002` | `docs/opi-spec.md#PRIN-002` | docs/opi-spec.md, docs/superpowers/specs/2026-08-25-phase18-independent-cross-agent-eval-seam-validation-design.md | met | — |
| `PRIN-003` | `docs/opi-spec.md#PRIN-003` | docs/opi-spec.md, docs/superpowers/specs/2026-08-25-phase18-independent-cross-agent-eval-seam-validation-design.md | met | — |
| `PRIN-004` | `docs/opi-spec.md#PRIN-004` | docs/opi-spec.md, docs/superpowers/specs/2026-08-25-phase18-independent-cross-agent-eval-seam-validation-design.md | not-met | P18-AUD-001, P18-AUD-003 |
| `PRIN-005` | `docs/opi-spec.md#PRIN-005` | docs/opi-spec.md, docs/superpowers/specs/2026-08-25-phase18-independent-cross-agent-eval-seam-validation-design.md | met | — |
| `PLACE-001` | `docs/opi-spec.md#PLACE-001` | docs/opi-spec.md, docs/superpowers/specs/2026-08-25-phase18-independent-cross-agent-eval-seam-validation-design.md | met | — |
| `PLACE-002` | `docs/opi-spec.md#PLACE-002` | docs/opi-spec.md, docs/superpowers/specs/2026-08-25-phase18-independent-cross-agent-eval-seam-validation-design.md | met | — |
| `PLACE-003` | `docs/opi-spec.md#PLACE-003` | docs/opi-spec.md, docs/superpowers/specs/2026-08-25-phase18-independent-cross-agent-eval-seam-validation-design.md | met | — |
| `PLACE-004` | `docs/opi-spec.md#PLACE-004` | docs/opi-spec.md, docs/superpowers/specs/2026-08-25-phase18-independent-cross-agent-eval-seam-validation-design.md | met | — |
| `CAP-003` | `docs/opi-spec.md#CAP-003` | docs/opi-spec.md, docs/superpowers/specs/2026-08-25-phase18-independent-cross-agent-eval-seam-validation-design.md | met | — |
| `CAP-006` | `docs/opi-spec.md#CAP-006` | docs/opi-spec.md, docs/superpowers/specs/2026-08-25-phase18-independent-cross-agent-eval-seam-validation-design.md | met | — |
| `CTRL-001` | `docs/opi-spec.md#CTRL-001` | docs/opi-spec.md, docs/superpowers/specs/2026-08-25-phase18-independent-cross-agent-eval-seam-validation-design.md | met | — |
| `CTRL-002` | `docs/opi-spec.md#CTRL-002` | docs/opi-spec.md, docs/superpowers/specs/2026-08-25-phase18-independent-cross-agent-eval-seam-validation-design.md | met | — |
| `CTRL-003` | `docs/opi-spec.md#CTRL-003` | docs/opi-spec.md, docs/superpowers/specs/2026-08-25-phase18-independent-cross-agent-eval-seam-validation-design.md | met | — |
| `CTRL-004` | `docs/opi-spec.md#CTRL-004` | docs/opi-spec.md, docs/superpowers/specs/2026-08-25-phase18-independent-cross-agent-eval-seam-validation-design.md | met | — |
| `CTRL-005` | `docs/opi-spec.md#CTRL-005` | docs/opi-spec.md, docs/superpowers/specs/2026-08-25-phase18-independent-cross-agent-eval-seam-validation-design.md | met | — |
| `CTRL-006` | `docs/opi-spec.md#CTRL-006` | docs/opi-spec.md, docs/superpowers/specs/2026-08-25-phase18-independent-cross-agent-eval-seam-validation-design.md | met | — |
| `CTRL-007` | `docs/opi-spec.md#CTRL-007` | docs/opi-spec.md, docs/superpowers/specs/2026-08-25-phase18-independent-cross-agent-eval-seam-validation-design.md | met | — |
| `CTRL-008` | `docs/opi-spec.md#CTRL-008` | docs/opi-spec.md, docs/superpowers/specs/2026-08-25-phase18-independent-cross-agent-eval-seam-validation-design.md | met | — |
| `CTRL-009` | `docs/opi-spec.md#CTRL-009` | docs/opi-spec.md, docs/superpowers/specs/2026-08-25-phase18-independent-cross-agent-eval-seam-validation-design.md | met | — |
| `CTRL-010` | `docs/opi-spec.md#CTRL-010` | docs/opi-spec.md, docs/superpowers/specs/2026-08-25-phase18-independent-cross-agent-eval-seam-validation-design.md | met | — |
| `INV-006` | `docs/opi-spec.md#INV-006` | docs/opi-spec.md, docs/superpowers/specs/2026-08-25-phase18-independent-cross-agent-eval-seam-validation-design.md | met | — |
| `INV-007` | `docs/opi-spec.md#INV-007` | docs/opi-spec.md, docs/superpowers/specs/2026-08-25-phase18-independent-cross-agent-eval-seam-validation-design.md | met | — |
| `INV-008` | `docs/opi-spec.md#INV-008` | docs/opi-spec.md, docs/superpowers/specs/2026-08-25-phase18-independent-cross-agent-eval-seam-validation-design.md | met | — |
| `GATE-001` | `docs/opi-spec.md#GATE-001` | docs/opi-spec.md, docs/superpowers/specs/2026-08-25-phase18-independent-cross-agent-eval-seam-validation-design.md | met | — |
| `GATE-002` | `docs/opi-spec.md#GATE-002` | docs/opi-spec.md, docs/superpowers/specs/2026-08-25-phase18-independent-cross-agent-eval-seam-validation-design.md | met | — |
| `PHASE-001` | `docs/opi-spec.md#PHASE-001` | docs/opi-spec.md, docs/superpowers/specs/2026-08-25-phase18-independent-cross-agent-eval-seam-validation-design.md | met | — |
| `PHASE-002` | `docs/opi-spec.md#PHASE-002` | docs/opi-spec.md, docs/superpowers/specs/2026-08-25-phase18-independent-cross-agent-eval-seam-validation-design.md | met | — |
| `PHASE-003` | `docs/opi-spec.md#PHASE-003` | docs/opi-spec.md, docs/superpowers/specs/2026-08-25-phase18-independent-cross-agent-eval-seam-validation-design.md | met | — |
| `PHASE-004` | `docs/opi-spec.md#PHASE-004` | docs/opi-spec.md, docs/superpowers/specs/2026-08-25-phase18-independent-cross-agent-eval-seam-validation-design.md | met | — |
| `PHASE-005` | `docs/opi-spec.md#PHASE-005` | docs/opi-spec.md, docs/superpowers/specs/2026-08-25-phase18-independent-cross-agent-eval-seam-validation-design.md | met | — |
| `PHASE-006` | `docs/opi-spec.md#PHASE-006` | docs/opi-spec.md, docs/superpowers/specs/2026-08-25-phase18-independent-cross-agent-eval-seam-validation-design.md | met | — |
| `STRAT-002` | `docs/opi-spec.md#STRAT-002` | docs/opi-spec.md, docs/superpowers/specs/2026-08-25-phase18-independent-cross-agent-eval-seam-validation-design.md | met | — |
| `P18-NG-001` | `docs/superpowers/specs/2026-08-25-phase18-independent-cross-agent-eval-seam-validation-design.md#Non-goals` | crates/opi-eval/tests/rollback_contract.rs, crates/opi-eval/src/lib.rs | met | — |
| `P18-NG-002` | `docs/superpowers/specs/2026-08-25-phase18-independent-cross-agent-eval-seam-validation-design.md#Non-goals` | crates/opi-eval/tests/rollback_contract.rs, crates/opi-eval/src/lib.rs | met | — |
| `P18-NG-003` | `docs/superpowers/specs/2026-08-25-phase18-independent-cross-agent-eval-seam-validation-design.md#Non-goals` | crates/opi-eval/tests/rollback_contract.rs, crates/opi-eval/src/lib.rs | met | — |
| `P18-NG-004` | `docs/superpowers/specs/2026-08-25-phase18-independent-cross-agent-eval-seam-validation-design.md#Non-goals` | crates/opi-eval/tests/rollback_contract.rs, crates/opi-eval/src/lib.rs | met | — |
| `P18-NG-005` | `docs/superpowers/specs/2026-08-25-phase18-independent-cross-agent-eval-seam-validation-design.md#Non-goals` | crates/opi-eval/tests/rollback_contract.rs, crates/opi-eval/src/lib.rs | met | — |
| `P18-NG-006` | `docs/superpowers/specs/2026-08-25-phase18-independent-cross-agent-eval-seam-validation-design.md#Non-goals` | crates/opi-eval/tests/rollback_contract.rs, crates/opi-eval/src/lib.rs | met | — |
| `P18-NG-007` | `docs/superpowers/specs/2026-08-25-phase18-independent-cross-agent-eval-seam-validation-design.md#Non-goals` | crates/opi-eval/tests/rollback_contract.rs, crates/opi-eval/src/lib.rs | met | — |
| `P18-NG-008` | `docs/superpowers/specs/2026-08-25-phase18-independent-cross-agent-eval-seam-validation-design.md#Non-goals` | crates/opi-eval/tests/rollback_contract.rs, crates/opi-eval/src/lib.rs | met | — |
| `P18-NG-009` | `docs/superpowers/specs/2026-08-25-phase18-independent-cross-agent-eval-seam-validation-design.md#Non-goals` | crates/opi-eval/tests/rollback_contract.rs, crates/opi-eval/src/lib.rs | met | — |
| `P18-NG-010` | `docs/superpowers/specs/2026-08-25-phase18-independent-cross-agent-eval-seam-validation-design.md#Non-goals` | crates/opi-eval/tests/rollback_contract.rs, crates/opi-eval/src/lib.rs | met | — |
| `P18-NG-011` | `docs/superpowers/specs/2026-08-25-phase18-independent-cross-agent-eval-seam-validation-design.md#Non-goals` | crates/opi-eval/tests/rollback_contract.rs, crates/opi-eval/src/lib.rs | met | — |
| `P18-NG-012` | `docs/superpowers/specs/2026-08-25-phase18-independent-cross-agent-eval-seam-validation-design.md#Non-goals` | crates/opi-eval/tests/rollback_contract.rs, crates/opi-eval/src/lib.rs | met | — |
| `P18-NG-013` | `docs/superpowers/specs/2026-08-25-phase18-independent-cross-agent-eval-seam-validation-design.md#Non-goals` | crates/opi-eval/tests/rollback_contract.rs, crates/opi-eval/src/lib.rs | met | — |
| `P18-RISK-001` | `docs/superpowers/specs/2026-08-25-phase18-independent-cross-agent-eval-seam-validation-design.md#Risk thresholds and rollback` | crates/opi-eval/src, scripts | met | — |
| `P18-RISK-002` | `docs/superpowers/specs/2026-08-25-phase18-independent-cross-agent-eval-seam-validation-design.md#Risk thresholds and rollback` | crates/opi-eval/src, scripts | met | — |
| `P18-RISK-003` | `docs/superpowers/specs/2026-08-25-phase18-independent-cross-agent-eval-seam-validation-design.md#Risk thresholds and rollback` | crates/opi-eval/src, scripts | met | — |
| `P18-RISK-004` | `docs/superpowers/specs/2026-08-25-phase18-independent-cross-agent-eval-seam-validation-design.md#Risk thresholds and rollback` | crates/opi-eval/src, scripts | met | — |
| `P18-RISK-005` | `docs/superpowers/specs/2026-08-25-phase18-independent-cross-agent-eval-seam-validation-design.md#Risk thresholds and rollback` | crates/opi-eval/src, scripts | met | — |
| `P18-RISK-006` | `docs/superpowers/specs/2026-08-25-phase18-independent-cross-agent-eval-seam-validation-design.md#Risk thresholds and rollback` | crates/opi-eval/src, scripts | met | — |
| `P18-RISK-007` | `docs/superpowers/specs/2026-08-25-phase18-independent-cross-agent-eval-seam-validation-design.md#Risk thresholds and rollback` | crates/opi-eval/src, scripts | met | — |
| `P18-RISK-008` | `docs/superpowers/specs/2026-08-25-phase18-independent-cross-agent-eval-seam-validation-design.md#Risk thresholds and rollback` | crates/opi-eval/src, scripts | met | — |
| `P18-RISK-009` | `docs/superpowers/specs/2026-08-25-phase18-independent-cross-agent-eval-seam-validation-design.md#Risk thresholds and rollback` | crates/opi-eval/src, scripts | met | — |
| `P18-RISK-010` | `docs/superpowers/specs/2026-08-25-phase18-independent-cross-agent-eval-seam-validation-design.md#Risk thresholds and rollback` | crates/opi-eval/src, scripts | met | — |
| `P18-RISK-011` | `docs/superpowers/specs/2026-08-25-phase18-independent-cross-agent-eval-seam-validation-design.md#Risk thresholds and rollback` | crates/opi-eval/src, scripts | met | — |
| `P18-RISK-012` | `docs/superpowers/specs/2026-08-25-phase18-independent-cross-agent-eval-seam-validation-design.md#Risk thresholds and rollback` | crates/opi-eval/src, scripts | met | — |
| `P18-RISK-013` | `docs/superpowers/specs/2026-08-25-phase18-independent-cross-agent-eval-seam-validation-design.md#Risk thresholds and rollback` | crates/opi-eval/src, scripts | met | — |
| `P18-RISK-014` | `docs/superpowers/specs/2026-08-25-phase18-independent-cross-agent-eval-seam-validation-design.md#Risk thresholds and rollback` | crates/opi-eval/src, scripts | met | — |
| `P18-RISK-015` | `docs/superpowers/specs/2026-08-25-phase18-independent-cross-agent-eval-seam-validation-design.md#Risk thresholds and rollback` | crates/opi-eval/src, scripts | met | — |
| `P18-RISK-016` | `docs/superpowers/specs/2026-08-25-phase18-independent-cross-agent-eval-seam-validation-design.md#Risk thresholds and rollback` | crates/opi-eval/src, scripts | met | — |
| `P18-RISK-017` | `docs/superpowers/specs/2026-08-25-phase18-independent-cross-agent-eval-seam-validation-design.md#Risk thresholds and rollback` | crates/opi-eval/src, scripts | met | — |
| `P18-RISK-018` | `docs/superpowers/specs/2026-08-25-phase18-independent-cross-agent-eval-seam-validation-design.md#Risk thresholds and rollback` | crates/opi-eval/src, scripts | met | — |
| `P18-TASK-18-1-DOD` | `$.tasks[?(@.id=='18.1')].definition_of_done` | Cargo.toml, Cargo.lock | met | — |
| `P18-TASK-18-2-DOD` | `$.tasks[?(@.id=='18.2')].definition_of_done` | crates/opi-eval/src/lib.rs, crates/opi-eval/src/external_lock.rs | met | — |
| `P18-TASK-18-3-DOD` | `$.tasks[?(@.id=='18.3')].definition_of_done` | crates/opi-eval/external-locks/resolved/linux-x86_64.json, crates/opi-eval/tests/fixtures/external-locks/materialization/** | met | — |
| `P18-TASK-18-4-DOD` | `$.tasks[?(@.id=='18.4')].definition_of_done` | crates/opi-eval/src/lib.rs, crates/opi-eval/src/process.rs | met | — |
| `P18-TASK-18-5-DOD` | `$.tasks[?(@.id=='18.5')].definition_of_done` | crates/opi-eval/src/lib.rs, crates/opi-eval/src/bundle/** | not-met | P18-AUD-002 |
| `P18-TASK-18-5-1-DOD` | `$.tasks[?(@.id=='18.5.1')].definition_of_done` | crates/opi-eval/src/lib.rs, crates/opi-eval/src/integrity.rs | met | — |
| `P18-TASK-18-6-DOD` | `$.tasks[?(@.id=='18.6')].definition_of_done` | crates/opi-eval/src/lib.rs, crates/opi-eval/src/agent/mod.rs | met | — |
| `P18-TASK-18-7-DOD` | `$.tasks[?(@.id=='18.7')].definition_of_done` | crates/opi-eval/src/agent/mod.rs, crates/opi-eval/src/agent/pi.rs | met | — |
| `P18-TASK-18-8-DOD` | `$.tasks[?(@.id=='18.8')].definition_of_done` | crates/opi-eval/src/lib.rs, crates/opi-eval/src/benchmark/mod.rs | met | — |
| `P18-TASK-18-9-DOD` | `$.tasks[?(@.id=='18.9')].definition_of_done` | crates/opi-eval/src/benchmark/mod.rs, crates/opi-eval/src/benchmark/terminal_bench_30.rs | not-met | P18-AUD-003 |
| `P18-TASK-18-10-DOD` | `$.tasks[?(@.id=='18.10')].definition_of_done` | crates/opi-eval/src/benchmark/mod.rs, crates/opi-eval/src/benchmark/deepswe.rs | met | — |
| `P18-TASK-18-10-1-DOD` | `$.tasks[?(@.id=='18.10.1')].definition_of_done` | crates/opi-eval/src/main.rs, crates/opi-eval/src/cli/mod.rs | met | — |
| `P18-TASK-18-11-DOD` | `$.tasks[?(@.id=='18.11')].definition_of_done` | crates/opi-eval/src/lib.rs, crates/opi-eval/src/trajectory/** | met | — |
| `P18-TASK-18-12-DOD` | `$.tasks[?(@.id=='18.12')].definition_of_done` | crates/opi-eval/src/lib.rs, crates/opi-eval/src/main.rs | not-met | P18-AUD-004 |
| `P18-TASK-18-13-DOD` | `$.tasks[?(@.id=='18.13')].definition_of_done` | crates/opi-eval/src/lib.rs, crates/opi-eval/src/main.rs | not-met | P18-AUD-004, P18-AUD-005 |
| `P18-TASK-18-14-DOD` | `$.tasks[?(@.id=='18.14')].definition_of_done` | .github/workflows/phase18-native-smoke.yml, scripts/phase18-native-smoke.sh | met | — |
| `P18-TASK-18-14-1-DOD` | `$.tasks[?(@.id=='18.14.1')].definition_of_done` | crates/opi-eval/src/**, crates/opi-eval/tests/** | met | — |
| `P18-TASK-18-15-DOD` | `$.tasks[?(@.id=='18.15')].definition_of_done` | scripts/verify-phase18-native-artifact.py, scripts/test_verify_phase18_native_artifact.py | not-assessable | P18-AUD-006 |
| `P18-TASK-18-16-DOD` | `$.tasks[?(@.id=='18.16')].definition_of_done` | crates/opi-eval/docs/seam-evidence-matrix.md, crates/opi-eval/tests/phase18_acceptance.rs | not-assessable | P18-AUD-006 |
| `P18-TASK-18-16-1-DOD` | `$.tasks[?(@.id=='18.16.1')].definition_of_done` | docs/snapshots/phase18/ci-receipt.json, .gitattributes | not-assessable | P18-AUD-006 |

## Standards Review

The Companion remains publish-disabled, has no Opi crate dependency or reverse product dependency, and preserves crate-private runtime seams. Focused formatting and clippy pass. Typed failures and supervision are generally strong, but P18-AUD-001/P18-AUD-002 violate authority and durability standards and P18-AUD-004 contradicts the registered cross-platform wrapper contract.

The documentation check could not traverse the archive-extracted `.claude/skills` symlink on Windows. Full-workspace fmt hit path-length error 206 in the long export; `cargo fmt -p opi-eval --check` passed. These are evidence limitations, not product findings.

## Spec Review

The seal covers all Phase 18 normative rows, parent routes, non-goals, risks, acceptance scenarios, and 20 task definitions of done. Most static contracts and focused negative tests are supported. Six blocking findings leave mandatory requirements non-met or not assessable; exact provenance means older native/CI artifacts cannot prove later current-head runtime changes.

## Security, Invariants, Integration, Test Quality, and Residuals

- Security: ancestor aliases bypass bundle staging and report output containment (P18-AUD-001, P18-AUD-005).
- Invariants: intent publication grants process authority before directory durability is complete (P18-AUD-002).
- Integration: Opi import is weaker than its producer and the Windows wrapper cannot execute its happy path (P18-AUD-003, P18-AUD-004).
- Runtime fidelity: exact native and three-platform evidence is bound to older candidates (P18-AUD-006).
- Test quality: 119 unit tests pass, but Unix-gated assembled/bundle/report/native binaries run zero tests on Windows. Scripted-provider tests pass with subprocess ResourceWarnings.
- Residuals: no plugin SDK, feature flag, reverse activation edge, or ordinary-Opi capability path was introduced.

## Minimum-change Conformance

| Task | Current status | Evidence |
|---|---|---|
| `18.1` | conforming | recorded reuse/placement/surface ceiling matches current crate-private implementation |
| `18.2` | conforming | recorded reuse/placement/surface ceiling matches current crate-private implementation |
| `18.3` | conforming | recorded reuse/placement/surface ceiling matches current crate-private implementation |
| `18.4` | conforming | recorded reuse/placement/surface ceiling matches current crate-private implementation |
| `18.5` | drifted | P18-AUD-002 |
| `18.5.1` | conforming | recorded reuse/placement/surface ceiling matches current crate-private implementation |
| `18.6` | conforming | recorded reuse/placement/surface ceiling matches current crate-private implementation |
| `18.7` | conforming | recorded reuse/placement/surface ceiling matches current crate-private implementation |
| `18.8` | conforming | recorded reuse/placement/surface ceiling matches current crate-private implementation |
| `18.9` | drifted | P18-AUD-003 |
| `18.10` | conforming | recorded reuse/placement/surface ceiling matches current crate-private implementation |
| `18.10.1` | conforming | recorded reuse/placement/surface ceiling matches current crate-private implementation |
| `18.11` | conforming | recorded reuse/placement/surface ceiling matches current crate-private implementation |
| `18.12` | drifted | P18-AUD-004 |
| `18.13` | drifted | P18-AUD-004, P18-AUD-005 |
| `18.14` | conforming | recorded reuse/placement/surface ceiling matches current crate-private implementation |
| `18.14.1` | conforming | recorded reuse/placement/surface ceiling matches current crate-private implementation |
| `18.15` | not-assessable | P18-AUD-006 |
| `18.16` | not-assessable | P18-AUD-006 |
| `18.16.1` | not-assessable | P18-AUD-006 |

No additional public runtime seam exists beyond the provisional unpublished lib-to-bin entry. Production consumers remain the same-package CLI/runner; conformance tests are nonproduction consumers. Drift is routed through findings.

## Findings

### P18-AUD-001: Ancestor symlinks can redirect staged bundle writes

- Axis: security
- Severity: Major
- Conformance effect: blocks
- Requirement IDs: P18-BND-002, P18-SEC-002, PRIN-004, P18-AUTH-001
- Claim: An Agent that creates a symlink at a not-yet-created bundle artifact directory can redirect later RunBundle insertion outside the bundle root, and seal-time validation accepts the resulting regular file through that symlinked ancestor.
- Evidence: `crates/opi-eval/src/bundle/mod.rs:1103` — Insertion derives the target with artifact_path and writes it with atomic_write without checking ancestor components.; `crates/opi-eval/src/bundle/mod.rs:1338` — read_covered applies symlink_metadata only to the final path; filesystem lookup has already followed any symlinked ancestor.; `crates/opi-eval/src/bundle/mod.rs:1352` — artifact_path joins the key and create_dir_all(parent), but never canonicalizes or rejects an existing symlink in the ancestor chain.; `crates/opi-eval/src/runner/experiment.rs:1133` — The Agent runs before native and normalized artifact directories are populated and can reach the sibling bundle tree from its workspace.
- Refutation attempted: ArtifactKey rejects absolute paths and parent traversal, and post-seal collection rejects a final symlink. Those controls do not inspect ancestor components; the only symlink test replaces the final file.
- Suggested closure: Open every artifact path beneath a trusted directory handle without following symlinks; reject any alias ancestor before writing and sealing, with an Agent-created ancestor test.

### P18-AUD-002: Intent publication is atomic but not crash-durable

- Axis: invariants
- Severity: Major
- Conformance effect: blocks
- Requirement IDs: P18-DUR-001, P18-TASK-18-5-DOD
- Claim: publish_intent returns DurableIntentProof after syncing the temporary file and renaming it, but without synchronizing the containing directory, so a system crash can lose the intent directory entry after process effects have begun.
- Evidence: `crates/opi-eval/src/bundle/mod.rs:1389` — The API promises a durably reserved intent and returns proof after atomic_write.; `crates/opi-eval/src/bundle/mod.rs:1415` — publish_intent delegates durability entirely to atomic_write before returning DurableIntentProof.; `crates/opi-eval/src/bundle/mod.rs:1522` — atomic_write syncs the temporary file and renames it, but performs no parent-directory fsync or platform-equivalent durable rename.; `crates/opi-eval/src/runner/experiment.rs:1069` — The returned proof authorizes entry into process-effect-pending.
- Refutation attempted: The temporary file is fully synced and rename prevents partial final bytes. That proves atomic publication, not persistence of the renamed directory entry.
- Suggested closure: Use a platform-appropriate durable atomic write that syncs the file and containing directory before returning DurableIntentProof, with fault injection.

### P18-AUD-003: Opi importer accepts Phase 17-invalid evidence graphs

- Axis: integration
- Severity: Major
- Conformance effect: blocks
- Requirement IDs: P18-AGT-003, P18-AGT-004, P18-A03, P18-A05, P18-TASK-18-9-DOD, PRIN-004, P18-AUTH-001
- Claim: The Opi importer can classify a trace as complete when records use mixed run identities, non-increasing sequences, inconsistent call graphs, or a kind/payload mismatch, even though the producing Phase 17 contract rejects each graph.
- Evidence: `crates/opi-eval/src/agent/opi.rs:387` — Import completion compares only manifest sequence to the last evidence-line sequence.; `crates/opi-eval/src/agent/opi.rs:454` — validate_evidence_records validates field names, UUID shape, independent vocabularies, and the last sequence but not run/call/parent correlation or monotonic ordering.; `crates/opi-agent/src/evidence.rs:1886` — The authoritative producer rejects mixed runs and sequences that are not strictly increasing.; `crates/opi-agent/src/evidence.rs:1904` — The producer also validates kind/payload agreement, stable call identity, and earlier-parent linkage.; `crates/opi-eval/src/agent/opi.rs:650` — Importer tests omit mixed-run, ordering, parent, and kind/payload adversaries.
- Refutation attempted: The importer checks exact fields, UUID shape, known vocabularies, completeness, and terminal sequence. None is an alternate implementation of the producer's graph invariants.
- Suggested closure: Validate single run, strict sequence, kind/payload, call stability, parent linkage, and terminal correlation; add one negative fixture per invariant.

### P18-AUD-004: The committed Windows smoke wrapper cannot execute the hermetic runner

- Axis: integration
- Severity: Major
- Conformance effect: blocks
- Requirement IDs: P18-TASK-18-12-DOD, P18-TASK-18-13-DOD
- Claim: The task 18.12/18.13 cross-platform PowerShell wrapper fails its happy path on Windows because the production hermetic runner always emits POSIX sh helpers and has no Windows execution branch.
- Evidence: `crates/opi-eval/src/runner/experiment.rs:243` — agent_helper_script emits POSIX commands, parameter syntax, and a /bin/sh shebang.; `crates/opi-eval/src/runner/experiment.rs:327` — make_executable is a no-op on non-Unix while run_trial directly spawns helper-agent.sh.; `crates/opi-eval/tests/report_contract.rs:14` — The assembled/report black-box suites are compiled out with cfg(unix).; `scripts/test_phase18_eval_smoke.py:45` — All three wrapper tests failed on the audited Windows host because happy did not return success.; `crates/opi-eval/src/runner/experiment.rs:1259` — A direct Windows run settled without answer.txt and failed with expected agent output is unreadable.
- Refutation attempted: The real native smoke is deliberately Linux-only, but the sealed task separately requires cross-platform hermetic wrappers and commits a PowerShell wrapper.
- Suggested closure: Implement Windows hermetic helpers/launch or narrow the registered task and remove the unsupported PowerShell claim; make the wrapper test pass on Windows CI.

### P18-AUD-005: Report output containment is bypassed through an ancestor alias

- Axis: security
- Severity: Major
- Conformance effect: blocks
- Requirement IDs: P18-BND-004, P18-RPT-001, P18-TASK-18-13-DOD
- Claim: An --out path lexically outside the run root but reached through a symlink or junction into a sealed bundle passes open_output containment and creates an unmanifested report inside that bundle.
- Evidence: `crates/opi-eval/src/cli/report.rs:69` — open_output canonicalizes the existing root but only lexically absolutizes the non-existing output target.; `crates/opi-eval/src/cli/report.rs:75` — Containment is starts_with against the unresolved output path.; `crates/opi-eval/src/cli/report.rs:91` — create_new follows existing ancestor aliases and creates the final file at their resolved destination.; `crates/opi-eval/tests/report_contract.rs:71` — Tests cover direct in-root and existing external targets but no symlinked or junction ancestor.
- Refutation attempted: create_new prevents overwrite and direct lexical in-root targets are rejected. It still follows an existing ancestor alias to create a new file inside the sealed tree.
- Suggested closure: Resolve and validate the output parent against canonical run/bundle roots without aliases; add symlink and Windows junction tests.

### P18-AUD-006: Phase-exit runtime evidence does not bind the audited implementation

- Axis: runtime-fidelity
- Severity: Major
- Conformance effect: blocks
- Requirement IDs: P18-OUT-001, P18-OUT-002, P18-OUT-005, P18-EXP-007, P18-AGT-002, P18-BMK-001, P18-BMK-003, P18-BMK-005, P18-BMK-009, P18-BMK-011, P18-PLT-001, P18-PLT-002, P18-A02, P18-A03, P18-A04, P18-A08, P18-A09, P18-A10, P18-A12, P18-A20, P18-A22, P18-TASK-18-15-DOD, P18-TASK-18-16-DOD, P18-TASK-18-16-1-DOD
- Claim: The recorded real Opi/pi three-benchmark artifact is bound to candidate 27344e3 and the three-platform receipt to 0f5a3fa, while audit_head materially changes the adapters, supervisors, benchmark importers, runner, bundle, report, and native producer they exercised; current-head real-runtime claims are therefore not assessable from those artifacts.
- Evidence: `docs/snapshots/phase18/opi-impl-state.json:3760` — The ledger records native run 33271354427 against 27344e3 and a revisit trigger when an admitted Agent, benchmark, runner, provider, adapter, authority, or host lock changes.; `docs/snapshots/phase18/ci-receipt.json:7` — The terminal three-platform receipt binds candidate_head 0f5a3fa152b12d7be4036b2a08ae7a195f8c2107.; `git diff --name-status 27344e3..08bc61d -- crates/opi-eval/src` — Current changes include Agent, benchmark, bundle, process, report, runner, and trajectory surfaces.; `git diff --name-status 27344e3..08bc61d -- scripts/phase18-native-smoke.sh scripts/phase18-scripted-provider.py` — Native producer and provider bytes changed after the artifact candidate.; `cargo test -p opi-eval --all-targets` — Current Windows tests pass, but real-process/native Unix integration binaries execute zero tests.
- Refutation attempted: The prior artifacts were valid for their candidates and current unit/clippy checks pass. Exact provenance prevents them from proving later runtime and producer bytes; the ledger records this revisit trigger.
- Suggested closure: Run the exact current candidate through pinned Linux native and three-platform CI, independently verify the artifacts, and bind current-head receipts.

## Verification Commands

| Command | Result | Requirement/finding |
|---|---|---|
| `rotation validator` | PASS | independent input boundary |
| `cargo test -p opi-eval --all-targets` | PASS: 119 unit tests and Windows-visible integrations; Unix-gated binaries run 0 | broad Rust evidence |
| `cargo clippy -p opi-eval --all-targets -- -D warnings` | PASS | standards |
| `cargo fmt -p opi-eval --check` | PASS | standards |
| `cargo tree -p opi-eval --all-features --target all --edges normal,build,dev` | PASS: no Opi dependency | placement |
| `cargo tree --workspace --invert opi-eval` | PASS: no reverse consumer | placement |
| `python scripts/test_capture_phase18_minimal_runtime_baseline.py` | PASS: 18 tests, 11 skips | A19 |
| `python scripts/test_derive_phase18_seam_matrix.py` | PASS: 7 tests | A20 contract |
| `python scripts/test_phase18_eval_smoke.py` | FAIL: 3/3 Windows wrapper tests | P18-AUD-004 |
| `python scripts/test_phase18_scripted_provider.py` | PASS: 15; ResourceWarnings | security |
| `python scripts/test_verify_phase18_ci.py` | PASS: 26 | CI contract |
| `python scripts/test_verify_phase18_materialization_artifact.py` | PASS: 53 | external locks |
| `python scripts/test_verify_phase18_materialization_ci.py` | PASS: 15 | external locks |
| `python scripts/test_verify_phase18_native_ci.py` | PASS: 34 | native producer contract |
| `python scripts/test_verify_phase18_native_artifact.py` | ENVIRONMENTAL FAIL: python3 returned Windows 9009 | limitation |
| `native artifact tests with python3 mapped to current interpreter` | FAIL: Windows separators reject valid synthetic artifact | limitation; not current native proof |
| `python scripts/opi-doc-check.py` | ENVIRONMENTAL FAIL: archive symlink access denied | limitation |
| `cargo fmt --check --all` | ENVIRONMENTAL FAIL: path error 206 | limitation |
| `git diff --name-status 27344e3..08bc61d -- Phase 18 native surfaces` | FAIL current-head binding | P18-AUD-006 |

## Verdict Rationale

The member verdict is mechanically **FAIL** because 38 mandatory records are not `met` (15 not met and 23 not assessable). All six findings are Major and blocking. Passing unit, clippy, dependency, and verifier-contract tests do not close the filesystem/durability/importer defects or replace current-head native evidence.

Test impact: `none` — assurance artifacts only; no production or test source was edited.
