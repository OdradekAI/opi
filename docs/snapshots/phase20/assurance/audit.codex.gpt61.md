# Phase 20 Audit

**Audit run ID**: `phase20-codex-gpt61-b3f1810-20261103t142820z`
**Audit head**: `b3f1810801195d4e4746d9c9f0ac63d39da57313`
**Reviewer ID**: `codex`
**Model ID**: `gpt61`
**Reviewer identity**: Codex
**Reviewer model ID**: `gpt61`
**Model identity source**: operator-declared
**Independence**: fresh-context-same-family; fresh independent context, no assertion of a different model family. Bounded assistants contributed to this one peer, not separate member reports.
**Baseline policy**: latest-committed-spec
**Verdict**: FAIL

## Baseline Sources

Rotation admitted before source reading. The allowlisted reader sealed the head once; raw ledgers, prior audit/remediation material and sibling member output were excluded. All source/code checks ran in the git-archive export. The model slug is transcribed from the invocation and is not an inferred runtime model identity. No production file or implementation ledger was changed.

| Path | SHA-256 | Registration note |
|---|---|---|
| .opi-impl-state.json | `15c7f977d0c2cd7aee098836d5d47c195b402e1e2fb9a82a16fb69c89f52f975` | current committed full bytes |
| docs/snapshots/phase20/opi-impl-state.json | `222372715140f4d66ae43ed6c39779daca50f167e0407503d252536bcbd86bb8` | current committed full bytes |
| docs/opi-spec.md | `b698133fbc2c0347726447159259c18effa8a1091796aae559c4bb04cb5612f3` | current committed full bytes |
| docs/opi-spec.zh.md | `e9da7696be0772e2ca72f74943c0b3fbba398850abc751d58726d31068cbb13c` | current committed full bytes |
| docs/research/pi-ai-opi-ai-alignment/spec-draft.md | `8c6cd66f7771e026210f06e49d037da54029c74258a1d45925cdbddd710f5f21` | current committed full bytes |
| docs/research/pi-ai-opi-ai-alignment/spec-draft.zh.md | `a841575a91ab4a3895648c1c66ce2c6c7d9afd4685fb1d408e550d39b3f62415` | current committed full bytes |
| docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md | `9e3d6ef690c3a794bc9c47ddc2dce04840f98d8881649be1ab2e8db0df287fd5` | current committed full bytes |
| docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-rust-differences.md | `411d2f6d262b1afc22e84c73213dffd0eb890ed70e210dfa497d2a5d021cd31d` | current committed full bytes |
| docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-rust-backends.md | `acb7f5f798b3384c63d36ad9af844b580755e2ea792ce3e2d32d9d3af2f15e81` | current committed full bytes |
| docs/research/pi-ai-opi-ai-alignment/acceptance-matrix.md | `c831586339910f9496a64e6b3c9390a0b7a2a366d37230222874a5f399099a5d` | stored-hash mismatch; current committed bytes govern |
| docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-design-readiness.md | `b74c0b09afd31bc06cf8d5d1df834a6af9d53f2d08c3653aa492ed36396c4eba` | current committed full bytes |
| docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-schema.md | `878d15231e734dc03772e1f1bd48cb8ffe2252a2818748e4b8630af43a2e8a76` | current committed full bytes |
| docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-ws.md | `61547ccfc0549018d2fd4042ade8edac3547599b412e12ea2479821b40f67a8f` | current committed full bytes |
| docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-transport-auth.md | `42e1bc90c02e958ada4d2a01db999cd906ae33ee971496c50c07ed38067cdcfb` | current committed full bytes |
| docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-runtime.md | `1328a81e561e1e1bfb2c35df788ce6d08fec2f71e14eb8abba9d57e1f03f1c7b` | current committed full bytes |
| docs/research/pi-ai-opi-ai-alignment/transcript-target-proposal.md | `25a4234507ab1c4c12d483c56bb4083172bf4d5c35dfec1dc92bc98c77a5d94e` | current committed full bytes |
| docs/research/pi-ai-opi-ai-alignment/replay-contracts.md | `223b21e1aa590d0e4646d99cd7f6792a694c7d8bf0f34b31dd6380979d324fa4` | current committed full bytes |
| docs/research/pi-ai-opi-ai-alignment/sdk-target-proposal.md | `4f84b93681a6dce3bea494d0553e81a1f5cb3aec5ad62635eda93a8ee1f5a75c` | current committed full bytes |

The matrix stored hash `0c05c6a7fbc2a15bf1c727687af35848032e48ca3969588066c6a6777ff267e0` differs in both projected ledgers from current `c831586339910f9496a64e6b3c9390a0b7a2a366d37230222874a5f399099a5d`. It is metadata evidence; no claim that human reconciliation never occurred follows.

## Requirement Conformance

942 mandatory identities were sealed before production inspection:905 fixture,26 native,11 platform. Current states: met=710, not-assessable=155, not-met=50, partially-met=27. Historical host-only/retired duties were excluded under the current source-adoption rules. The requirements sidecar carries complete criteria, immutable source digests, behaviors, consumer/assertion/check evidence and reciprocal findings.

| Requirement ID | Criterion | Current evidence | State | Finding IDs |
|---|---|---|---|---|
| P20-V-01 | docs/research/pi-ai-opi-ai-alignment/acceptance-matrix.md V-01, line 56 | crates/opi-ai/src/models.rs (one frozen Models driver and complete-record binding) | not-assessable | P20-AUD-018 |
| P20-V-02 | docs/research/pi-ai-opi-ai-alignment/acceptance-matrix.md V-02, line 57 | crates/opi-ai/src/models.rs (one frozen Models driver and complete-record binding) | met |  |
| P20-V-03 | docs/research/pi-ai-opi-ai-alignment/acceptance-matrix.md V-03, line 58 | crates/opi-ai/src/models.rs (one frozen Models driver and complete-record binding) | partially-met | P20-AUD-005 |
| P20-V-04 | docs/research/pi-ai-opi-ai-alignment/acceptance-matrix.md V-04, line 59 | crates/opi-ai/src/models.rs (one frozen Models driver and complete-record binding) | met |  |
| P20-V-05 | docs/research/pi-ai-opi-ai-alignment/acceptance-matrix.md V-05, line 60 | crates/opi-ai/src/models.rs (one frozen Models driver and complete-record binding) | partially-met | P20-AUD-006 |
| P20-V-06 | docs/research/pi-ai-opi-ai-alignment/acceptance-matrix.md V-06, line 61 | crates/opi-ai/src/models.rs (one frozen Models driver and complete-record binding) | met |  |
| P20-V-07 | docs/research/pi-ai-opi-ai-alignment/acceptance-matrix.md V-07, line 62 | crates/opi-ai/src/models.rs (one frozen Models driver and complete-record binding) | met |  |
| P20-V-08 | docs/research/pi-ai-opi-ai-alignment/acceptance-matrix.md V-08, line 63 | crates/opi-ai/src/models.rs (one frozen Models driver and complete-record binding) | met |  |
| P20-V-09 | docs/research/pi-ai-opi-ai-alignment/acceptance-matrix.md V-09, line 64 | crates/opi-ai/src/models.rs (one frozen Models driver and complete-record binding) | not-assessable | P20-AUD-018 |
| P20-V-10 | docs/research/pi-ai-opi-ai-alignment/acceptance-matrix.md V-10, line 65 | crates/opi-ai/src/models.rs (one frozen Models driver and complete-record binding) | met |  |
| P20-V-11 | docs/research/pi-ai-opi-ai-alignment/acceptance-matrix.md V-11, line 66 | crates/opi-ai/src/models.rs (one frozen Models driver and complete-record binding) | partially-met | P20-AUD-009 |
| P20-V-12 | docs/research/pi-ai-opi-ai-alignment/acceptance-matrix.md V-12, line 67 | crates/opi-ai/src/models.rs (one frozen Models driver and complete-record binding) | partially-met | P20-AUD-001 |
| P20-V-13 | docs/research/pi-ai-opi-ai-alignment/acceptance-matrix.md V-13, line 68 | crates/opi-ai/src/models.rs (one frozen Models driver and complete-record binding) | partially-met | P20-AUD-003 |
| P20-V-14 | docs/research/pi-ai-opi-ai-alignment/acceptance-matrix.md V-14, line 69 | crates/opi-ai/src/models.rs (one frozen Models driver and complete-record binding) | met |  |
| P20-V-15 | docs/research/pi-ai-opi-ai-alignment/acceptance-matrix.md V-15, line 70 | crates/opi-ai/src/models.rs (one frozen Models driver and complete-record binding) | partially-met | P20-AUD-015 |
| P20-V-16 | docs/research/pi-ai-opi-ai-alignment/acceptance-matrix.md V-16, line 71 | crates/opi-ai/src/models.rs (one frozen Models driver and complete-record binding) | not-assessable | P20-AUD-018 |
| P20-V-20 | docs/research/pi-ai-opi-ai-alignment/acceptance-matrix.md V-20, line 75 | crates/opi-ai/src/models.rs (one frozen Models driver and complete-record binding) | partially-met | P20-AUD-004, P20-AUD-016 |
| P20-V-21 | docs/research/pi-ai-opi-ai-alignment/acceptance-matrix.md V-21, line 76 | crates/opi-ai/src/models.rs (one frozen Models driver and complete-record binding) | met |  |
| P20-V-22 | docs/research/pi-ai-opi-ai-alignment/acceptance-matrix.md V-22, line 77 | crates/opi-ai/src/models.rs (one frozen Models driver and complete-record binding) | partially-met | P20-AUD-014 |
| P20-V-23 | docs/research/pi-ai-opi-ai-alignment/acceptance-matrix.md V-23, line 78 | crates/opi-ai/src/models.rs (one frozen Models driver and complete-record binding) | partially-met | P20-AUD-013 |
| P20-V-24 | docs/research/pi-ai-opi-ai-alignment/acceptance-matrix.md V-24, line 79 | crates/opi-ai/src/models.rs (one frozen Models driver and complete-record binding) | partially-met | P20-AUD-011 |
| P20-V-25 | docs/research/pi-ai-opi-ai-alignment/acceptance-matrix.md V-25, line 80 | crates/opi-ai/src/models.rs (one frozen Models driver and complete-record binding) | partially-met | P20-AUD-014 |
| P20-V-26 | docs/research/pi-ai-opi-ai-alignment/acceptance-matrix.md V-26, line 81 | crates/opi-ai/src/models.rs (one frozen Models driver and complete-record binding) | not-assessable | P20-AUD-018 |
| P20-V-27 | docs/research/pi-ai-opi-ai-alignment/acceptance-matrix.md V-27, line 89 | crates/opi-ai/src/models.rs (one frozen Models driver and complete-record binding) | met |  |
| P20-V-28 | docs/research/pi-ai-opi-ai-alignment/acceptance-matrix.md V-28, line 90 | crates/opi-ai/src/models.rs (one frozen Models driver and complete-record binding) | met |  |
| P20-V-29 | docs/research/pi-ai-opi-ai-alignment/acceptance-matrix.md V-29, line 91 | crates/opi-ai/src/models.rs (one frozen Models driver and complete-record binding) | met |  |
| P20-V-30 | docs/research/pi-ai-opi-ai-alignment/acceptance-matrix.md V-30, line 92 | crates/opi-ai/src/models.rs (one frozen Models driver and complete-record binding) | met |  |
| P20-V-31 | docs/research/pi-ai-opi-ai-alignment/acceptance-matrix.md V-31, line 93 | crates/opi-ai/src/models.rs (one frozen Models driver and complete-record binding) | not-assessable | P20-AUD-018 |
| P20-V-32 | docs/research/pi-ai-opi-ai-alignment/acceptance-matrix.md V-32, line 94 | crates/opi-ai/src/models.rs (one frozen Models driver and complete-record binding) | not-assessable | P20-AUD-018 |
| P20-V-34 | docs/research/pi-ai-opi-ai-alignment/acceptance-matrix.md V-34, line 96 | crates/opi-ai/src/models.rs (one frozen Models driver and complete-record binding) | not-assessable | P20-AUD-018 |
| P20-V-35 | docs/research/pi-ai-opi-ai-alignment/acceptance-matrix.md V-35, line 97 | crates/opi-ai/src/models.rs (one frozen Models driver and complete-record binding) | partially-met | P20-AUD-015 |
| P20-V-36 | docs/research/pi-ai-opi-ai-alignment/acceptance-matrix.md V-36, line 98 | crates/opi-ai/src/models.rs (one frozen Models driver and complete-record binding) | partially-met | P20-AUD-014 |
| P20-V-37 | docs/research/pi-ai-opi-ai-alignment/acceptance-matrix.md V-37, line 99 | crates/opi-ai/src/models.rs (one frozen Models driver and complete-record binding) | met |  |
| P20-V-38 | docs/research/pi-ai-opi-ai-alignment/acceptance-matrix.md V-38, line 100 | crates/opi-ai/src/models.rs (one frozen Models driver and complete-record binding) | not-assessable | P20-AUD-018 |
| P20-V-39 | docs/research/pi-ai-opi-ai-alignment/acceptance-matrix.md V-39, line 182 | crates/opi-ai/src/models.rs (one frozen Models driver and complete-record binding) | partially-met | P20-AUD-001, P20-AUD-008, P20-AUD-012 |
| P20-V-40 | docs/research/pi-ai-opi-ai-alignment/acceptance-matrix.md V-40, line 183 | crates/opi-ai/src/models.rs (one frozen Models driver and complete-record binding) | partially-met | P20-AUD-005 |
| P20-V-41 | docs/research/pi-ai-opi-ai-alignment/acceptance-matrix.md V-41, line 184 | crates/opi-ai/src/models.rs (one frozen Models driver and complete-record binding) | partially-met | P20-AUD-005 |
| P20-V-42 | docs/research/pi-ai-opi-ai-alignment/acceptance-matrix.md V-42, line 185 | crates/opi-ai/src/models.rs (one frozen Models driver and complete-record binding) | not-assessable | P20-AUD-004, P20-AUD-018 |
| P20-V-43 | docs/research/pi-ai-opi-ai-alignment/acceptance-matrix.md V-43, line 191 | crates/opi-ai/src/models.rs (one frozen Models driver and complete-record binding) | met |  |
| P20-V-44 | docs/research/pi-ai-opi-ai-alignment/acceptance-matrix.md V-44, line 192 | crates/opi-ai/src/models.rs (one frozen Models driver and complete-record binding) | met |  |
| P20-V-45 | docs/research/pi-ai-opi-ai-alignment/acceptance-matrix.md V-45, line 193 | crates/opi-ai/src/models.rs (one frozen Models driver and complete-record binding) | partially-met | P20-AUD-008 |
| P20-V-46 | docs/research/pi-ai-opi-ai-alignment/acceptance-matrix.md V-46, line 202 | crates/opi-ai/src/models.rs (one frozen Models driver and complete-record binding) | met |  |
| P20-V-47 | docs/research/pi-ai-opi-ai-alignment/acceptance-matrix.md V-47, line 203 | crates/opi-ai/src/models.rs (one frozen Models driver and complete-record binding) | met |  |
| P20-V-48 | docs/research/pi-ai-opi-ai-alignment/acceptance-matrix.md V-48, line 204 | crates/opi-ai/src/models.rs (one frozen Models driver and complete-record binding) | met |  |
| P20-V-49 | docs/research/pi-ai-opi-ai-alignment/acceptance-matrix.md V-49, line 212 | crates/opi-ai/src/models.rs (one frozen Models driver and complete-record binding) | partially-met | P20-AUD-009 |
| P20-V-50 | docs/research/pi-ai-opi-ai-alignment/acceptance-matrix.md V-50, line 213 | crates/opi-ai/src/models.rs (one frozen Models driver and complete-record binding) | partially-met | P20-AUD-003, P20-AUD-011, P20-AUD-013 |
| P20-V-51 | docs/research/pi-ai-opi-ai-alignment/acceptance-matrix.md V-51, line 214 | crates/opi-ai/src/models.rs (one frozen Models driver and complete-record binding) | partially-met | P20-AUD-014 |
| P20-V-53 | docs/research/pi-ai-opi-ai-alignment/acceptance-matrix.md V-53, line 216 | crates/opi-ai/src/models.rs (one frozen Models driver and complete-record binding) | not-assessable | P20-AUD-018 |
| P20-V-54 | docs/research/pi-ai-opi-ai-alignment/acceptance-matrix.md V-54, line 217 | crates/opi-ai/src/models.rs (one frozen Models driver and complete-record binding) | met |  |
| P20-V-55 | docs/research/pi-ai-opi-ai-alignment/acceptance-matrix.md V-55, line 218 | crates/opi-ai/src/models.rs (one frozen Models driver and complete-record binding) | partially-met | P20-AUD-020 |
| P20-V-56 | docs/research/pi-ai-opi-ai-alignment/acceptance-matrix.md V-56, line 219 | crates/opi-ai/src/models.rs (one frozen Models driver and complete-record binding) | not-assessable | P20-AUD-018 |
| P20-V-57 | docs/research/pi-ai-opi-ai-alignment/acceptance-matrix.md V-57, line 220 | crates/opi-ai/src/models.rs (one frozen Models driver and complete-record binding) | partially-met | P20-AUD-007 |
| P20-V-58 | docs/research/pi-ai-opi-ai-alignment/acceptance-matrix.md V-58, line 221 | crates/opi-ai/src/models.rs (one frozen Models driver and complete-record binding) | not-assessable | P20-AUD-018 |
| P20-V-59 | docs/research/pi-ai-opi-ai-alignment/acceptance-matrix.md V-59, line 222 | crates/opi-ai/src/models.rs (one frozen Models driver and complete-record binding) | not-assessable | P20-AUD-018 |
| P20-AUTH-003 | docs/opi-spec.md AUTH-003, line 36 | Cargo.toml:1 (seven members and inward workspace dependencies) | not-assessable | P20-AUD-018 |
| P20-AUTH-004 | docs/opi-spec.md AUTH-004, line 37 | Cargo.toml:1 (seven members and inward workspace dependencies) | met |  |
| P20-GOAL-001 | docs/opi-spec.md GOAL-001, line 72 | Cargo.toml:1 (seven members and inward workspace dependencies) | partially-met | P20-AUD-004 |
| P20-GOAL-002 | docs/opi-spec.md GOAL-002, line 73 | Cargo.toml:1 (seven members and inward workspace dependencies) | met |  |
| P20-GOAL-004 | docs/opi-spec.md GOAL-004, line 75 | Cargo.toml:1 (seven members and inward workspace dependencies) | met |  |
| P20-PRIN-001 | docs/opi-spec.md PRIN-001, line 120 | Cargo.toml:1 (seven members and inward workspace dependencies) | met |  |
| P20-PRIN-002 | docs/opi-spec.md PRIN-002, line 121 | Cargo.toml:1 (seven members and inward workspace dependencies) | not-assessable | P20-AUD-018 |
| P20-PRIN-003 | docs/opi-spec.md PRIN-003, line 122 | Cargo.toml:1 (seven members and inward workspace dependencies) | met |  |
| P20-PRIN-004 | docs/opi-spec.md PRIN-004, line 123 | Cargo.toml:1 (seven members and inward workspace dependencies) | partially-met | P20-AUD-001 |
| P20-PRIN-005 | docs/opi-spec.md PRIN-005, line 124 | Cargo.toml:1 (seven members and inward workspace dependencies) | not-assessable | P20-AUD-018 |
| P20-PLACE-001 | docs/opi-spec.md PLACE-001, line 192 | Cargo.toml:1 (seven members and inward workspace dependencies) | met |  |
| P20-PLACE-004 | docs/opi-spec.md PLACE-004, line 195 | Cargo.toml:1 (seven members and inward workspace dependencies) | met |  |
| P20-CTRL-001 | docs/opi-spec.md CTRL-001, line 278 | Cargo.toml:1 (seven members and inward workspace dependencies) | met |  |
| P20-CTRL-003 | docs/opi-spec.md CTRL-003, line 280 | Cargo.toml:1 (seven members and inward workspace dependencies) | partially-met | P20-AUD-003 |
| P20-INV-001 | docs/opi-spec.md INV-001, line 405 | Cargo.toml:1 (seven members and inward workspace dependencies) | partially-met | P20-AUD-001 |
| P20-INV-002 | docs/opi-spec.md INV-002, line 406 | Cargo.toml:1 (seven members and inward workspace dependencies) | not-assessable | P20-AUD-018 |
| P20-INV-005 | docs/opi-spec.md INV-005, line 442 | Cargo.toml:1 (seven members and inward workspace dependencies) | met |  |
| P20-INV-006 | docs/opi-spec.md INV-006, line 443 | Cargo.toml:1 (seven members and inward workspace dependencies) | partially-met | P20-AUD-005 |
| P20-GATE-001 | docs/opi-spec.md GATE-001, line 551 | Cargo.toml:1 (seven members and inward workspace dependencies) | not-assessable | P20-AUD-018 |
| P20-GATE-002 | docs/opi-spec.md GATE-002, line 552 | Cargo.toml:1 (seven members and inward workspace dependencies) | met |  |
| P20-PHASE-001 | docs/opi-spec.md PHASE-001, line 744 | Cargo.toml:1 (seven members and inward workspace dependencies) | met |  |
| P20-PHASE-002 | docs/opi-spec.md PHASE-002, line 745 | Cargo.toml:1 (seven members and inward workspace dependencies) | met |  |
| P20-PHASE-003 | docs/opi-spec.md PHASE-003, line 746 | Cargo.toml:1 (seven members and inward workspace dependencies) | met |  |
| P20-PHASE-004 | docs/opi-spec.md PHASE-004, line 747 | Cargo.toml:1 (seven members and inward workspace dependencies) | met |  |
| P20-PHASE-005 | docs/opi-spec.md PHASE-005, line 748 | Cargo.toml:1 (seven members and inward workspace dependencies) | met |  |
| P20-PHASE-006 | docs/opi-spec.md PHASE-006, line 749 | Cargo.toml:1 (seven members and inward workspace dependencies) | met |  |
| P20-SDK-CORE-001 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L19 | {"path":"crates/opi-ai/src/models.rs","sha256":"f9fc8b51c90282772b6b8812079772ad05c264c0b7d80efbd51314a8b46e914b","citation":"609-635,734-1015,1572-2025,4053-4250,4894-5150"} | met |  |
| P20-SDK-CORE-002 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L19 | {"path":"crates/opi-ai/src/models.rs","sha256":"f9fc8b51c90282772b6b8812079772ad05c264c0b7d80efbd51314a8b46e914b","citation":"609-635,734-1015,1572-2025,4053-4250,4894-5150"} | met |  |
| P20-SDK-CORE-003 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L19 | {"path":"crates/opi-ai/src/models.rs","sha256":"f9fc8b51c90282772b6b8812079772ad05c264c0b7d80efbd51314a8b46e914b","citation":"609-635,734-1015,1572-2025,4053-4250,4894-5150"} | met |  |
| P20-SDK-CORE-004 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L19 | {"path":"crates/opi-ai/src/models.rs","sha256":"f9fc8b51c90282772b6b8812079772ad05c264c0b7d80efbd51314a8b46e914b","citation":"609-635,734-1015,1572-2025,4053-4250,4894-5150"} | not-met | P20-AUD-012 |
| P20-SDK-CORE-005 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L20 | {"path":"crates/opi-ai/src/models.rs","sha256":"f9fc8b51c90282772b6b8812079772ad05c264c0b7d80efbd51314a8b46e914b","citation":"609-635,734-1015,1572-2025,4053-4250,4894-5150"} | met |  |
| P20-SDK-CORE-006 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L20 | {"path":"crates/opi-ai/src/models.rs","sha256":"f9fc8b51c90282772b6b8812079772ad05c264c0b7d80efbd51314a8b46e914b","citation":"609-635,734-1015,1572-2025,4053-4250,4894-5150"} | met |  |
| P20-SDK-CORE-007 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L20 | {"path":"crates/opi-ai/src/models.rs","sha256":"f9fc8b51c90282772b6b8812079772ad05c264c0b7d80efbd51314a8b46e914b","citation":"609-635,734-1015,1572-2025,4053-4250,4894-5150"} | met |  |
| P20-SDK-CORE-008 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L20 | {"path":"crates/opi-ai/src/models.rs","sha256":"f9fc8b51c90282772b6b8812079772ad05c264c0b7d80efbd51314a8b46e914b","citation":"609-635,734-1015,1572-2025,4053-4250,4894-5150"} | met |  |
| P20-SDK-CORE-009 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L20 | {"path":"crates/opi-ai/src/models.rs","sha256":"f9fc8b51c90282772b6b8812079772ad05c264c0b7d80efbd51314a8b46e914b","citation":"609-635,734-1015,1572-2025,4053-4250,4894-5150"} | met |  |
| P20-SDK-CORE-010 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L21 | {"path":"crates/opi-ai/src/models.rs","sha256":"f9fc8b51c90282772b6b8812079772ad05c264c0b7d80efbd51314a8b46e914b","citation":"609-635,734-1015,1572-2025,4053-4250,4894-5150"} | met |  |
| P20-SDK-CORE-011 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L21 | {"path":"crates/opi-ai/src/models.rs","sha256":"f9fc8b51c90282772b6b8812079772ad05c264c0b7d80efbd51314a8b46e914b","citation":"609-635,734-1015,1572-2025,4053-4250,4894-5150"} | met |  |
| P20-SDK-CORE-012 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L21 | {"path":"crates/opi-ai/src/models.rs","sha256":"f9fc8b51c90282772b6b8812079772ad05c264c0b7d80efbd51314a8b46e914b","citation":"609-635,734-1015,1572-2025,4053-4250,4894-5150"} | met |  |
| P20-SDK-CORE-013 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L21 | {"path":"crates/opi-ai/src/models.rs","sha256":"f9fc8b51c90282772b6b8812079772ad05c264c0b7d80efbd51314a8b46e914b","citation":"609-635,734-1015,1572-2025,4053-4250,4894-5150"} | met |  |
| P20-SDK-CORE-014 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L21 | {"path":"crates/opi-ai/src/models.rs","sha256":"f9fc8b51c90282772b6b8812079772ad05c264c0b7d80efbd51314a8b46e914b","citation":"609-635,734-1015,1572-2025,4053-4250,4894-5150"} | met |  |
| P20-SDK-CORE-015 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L21 | {"path":"crates/opi-ai/src/models.rs","sha256":"f9fc8b51c90282772b6b8812079772ad05c264c0b7d80efbd51314a8b46e914b","citation":"609-635,734-1015,1572-2025,4053-4250,4894-5150"} | met |  |
| P20-SDK-CORE-016 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L22 | {"path":"crates/opi-ai/src/models.rs","sha256":"f9fc8b51c90282772b6b8812079772ad05c264c0b7d80efbd51314a8b46e914b","citation":"609-635,734-1015,1572-2025,4053-4250,4894-5150"} | met |  |
| P20-SDK-CORE-017 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L22 | {"path":"crates/opi-ai/src/models.rs","sha256":"f9fc8b51c90282772b6b8812079772ad05c264c0b7d80efbd51314a8b46e914b","citation":"609-635,734-1015,1572-2025,4053-4250,4894-5150"} | met |  |
| P20-SDK-CORE-018 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L23 | {"path":"crates/opi-ai/src/models.rs","sha256":"f9fc8b51c90282772b6b8812079772ad05c264c0b7d80efbd51314a8b46e914b","citation":"609-635,734-1015,1572-2025,4053-4250,4894-5150"} | met |  |
| P20-SDK-CORE-019 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L23 | {"path":"crates/opi-ai/src/models.rs","sha256":"f9fc8b51c90282772b6b8812079772ad05c264c0b7d80efbd51314a8b46e914b","citation":"609-635,734-1015,1572-2025,4053-4250,4894-5150"} | met |  |
| P20-SDK-CORE-020 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L23 | {"path":"crates/opi-ai/src/models.rs","sha256":"f9fc8b51c90282772b6b8812079772ad05c264c0b7d80efbd51314a8b46e914b","citation":"609-635,734-1015,1572-2025,4053-4250,4894-5150"} | met |  |
| P20-SDK-CORE-021 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L23 | {"path":"crates/opi-ai/src/models.rs","sha256":"f9fc8b51c90282772b6b8812079772ad05c264c0b7d80efbd51314a8b46e914b","citation":"609-635,734-1015,1572-2025,4053-4250,4894-5150"} | met |  |
| P20-SDK-CORE-022 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L23 | {"path":"crates/opi-ai/src/models.rs","sha256":"f9fc8b51c90282772b6b8812079772ad05c264c0b7d80efbd51314a8b46e914b","citation":"609-635,734-1015,1572-2025,4053-4250,4894-5150"} | met |  |
| P20-SDK-CORE-023 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L24 | {"path":"crates/opi-ai/src/stream.rs","sha256":"5d5866af19cd6f5fb2d9f5ab9b09e2d5c78bfdac6e9a245b9971710fdc976605","citation":"549-675,680-887,904-1068"} | met |  |
| P20-SDK-CORE-024 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L24 | {"path":"crates/opi-ai/src/stream.rs","sha256":"5d5866af19cd6f5fb2d9f5ab9b09e2d5c78bfdac6e9a245b9971710fdc976605","citation":"549-675,680-887,904-1068"} | met |  |
| P20-SDK-CORE-025 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L24 | {"path":"crates/opi-ai/src/stream.rs","sha256":"5d5866af19cd6f5fb2d9f5ab9b09e2d5c78bfdac6e9a245b9971710fdc976605","citation":"549-675,680-887,904-1068"} | met |  |
| P20-SDK-CORE-026 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L24 | {"path":"crates/opi-ai/src/stream.rs","sha256":"5d5866af19cd6f5fb2d9f5ab9b09e2d5c78bfdac6e9a245b9971710fdc976605","citation":"549-675,680-887,904-1068"} | met |  |
| P20-SDK-CORE-027 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L159 | {"path":"crates/opi-ai/src/model_info.rs","sha256":"d1fe2274564f1a5da80950b7e11ee6e545c886b0d6fb617c607029dd8246b4d9","citation":"1168-1195"} | not-assessable | P20-AUD-018 |
| P20-SDK-CORE-028 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L159 | {"path":"crates/opi-ai/src/model_info.rs","sha256":"d1fe2274564f1a5da80950b7e11ee6e545c886b0d6fb617c607029dd8246b4d9","citation":"1168-1195"} | not-assessable | P20-AUD-018 |
| P20-SDK-CORE-029 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L210 | {"path":"crates/opi-ai/src/models.rs","sha256":"f9fc8b51c90282772b6b8812079772ad05c264c0b7d80efbd51314a8b46e914b","citation":"609-635,4053-4250,4894-5050"} | met |  |
| P20-SDK-CORE-030 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L212 | {"path":"crates/opi-ai/src/models.rs","sha256":"f9fc8b51c90282772b6b8812079772ad05c264c0b7d80efbd51314a8b46e914b","citation":"609-635,4053-4250,4894-5050"} | met |  |
| P20-SDK-CORE-031 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-rust-differences.md L56 | {"path":"crates/opi-ai/src/models.rs","sha256":"f9fc8b51c90282772b6b8812079772ad05c264c0b7d80efbd51314a8b46e914b","citation":"609-635,4053-4250,4894-5050"} | met |  |
| P20-SDK-CORE-032 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-rust-differences.md L56 | {"path":"crates/opi-ai/src/models.rs","sha256":"f9fc8b51c90282772b6b8812079772ad05c264c0b7d80efbd51314a8b46e914b","citation":"609-635,4053-4250,4894-5050"} | met |  |
| P20-SDK-CORE-033 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-rust-differences.md L56 | {"path":"crates/opi-ai/src/models.rs","sha256":"f9fc8b51c90282772b6b8812079772ad05c264c0b7d80efbd51314a8b46e914b","citation":"609-635,4053-4250,4894-5050"} | met |  |
| P20-SDK-CORE-034 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-rust-differences.md L56 | {"path":"crates/opi-ai/src/models.rs","sha256":"f9fc8b51c90282772b6b8812079772ad05c264c0b7d80efbd51314a8b46e914b","citation":"609-635,4053-4250,4894-5050"} | met |  |
| P20-SDK-CORE-035 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-rust-differences.md L65 | {"path":"crates/opi-ai/src/models.rs","sha256":"f9fc8b51c90282772b6b8812079772ad05c264c0b7d80efbd51314a8b46e914b","citation":"609-635,4053-4250,4894-5050"} | met |  |
| P20-SDK-CORE-036 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-rust-differences.md L65 | {"path":"crates/opi-ai/src/models.rs","sha256":"f9fc8b51c90282772b6b8812079772ad05c264c0b7d80efbd51314a8b46e914b","citation":"609-635,4053-4250,4894-5050"} | met |  |
| P20-SDK-CORE-037 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-rust-differences.md L65 | {"path":"crates/opi-ai/src/models.rs","sha256":"f9fc8b51c90282772b6b8812079772ad05c264c0b7d80efbd51314a8b46e914b","citation":"609-635,4053-4250,4894-5050"} | met |  |
| P20-SDK-CORE-038 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-rust-differences.md L74 | {"path":"crates/opi-ai/src/stream.rs","sha256":"5d5866af19cd6f5fb2d9f5ab9b09e2d5c78bfdac6e9a245b9971710fdc976605","citation":"549-675,780-887,904-1068"} | not-met | P20-AUD-005 |
| P20-SDK-CORE-039 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-rust-differences.md L74 | {"path":"crates/opi-ai/src/stream.rs","sha256":"5d5866af19cd6f5fb2d9f5ab9b09e2d5c78bfdac6e9a245b9971710fdc976605","citation":"549-675,780-887,904-1068"} | met |  |
| P20-SDK-CORE-040 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-rust-differences.md L74 | {"path":"crates/opi-ai/src/stream.rs","sha256":"5d5866af19cd6f5fb2d9f5ab9b09e2d5c78bfdac6e9a245b9971710fdc976605","citation":"549-675,780-887,904-1068"} | met |  |
| P20-SDK-CORE-041 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-rust-differences.md L83 | {"path":"crates/opi-ai/src/stream.rs","sha256":"5d5866af19cd6f5fb2d9f5ab9b09e2d5c78bfdac6e9a245b9971710fdc976605","citation":"549-675,780-887,904-1068"} | met |  |
| P20-SDK-CORE-042 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-rust-differences.md L83 | {"path":"crates/opi-ai/src/stream.rs","sha256":"5d5866af19cd6f5fb2d9f5ab9b09e2d5c78bfdac6e9a245b9971710fdc976605","citation":"549-675,780-887,904-1068"} | met |  |
| P20-SDK-CORE-043 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-rust-differences.md L83 | {"path":"crates/opi-ai/src/stream.rs","sha256":"5d5866af19cd6f5fb2d9f5ab9b09e2d5c78bfdac6e9a245b9971710fdc976605","citation":"549-675,780-887,904-1068"} | met |  |
| P20-SDK-CORE-044 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-rust-differences.md L92 | {"path":"crates/opi-ai/src/stream.rs","sha256":"5d5866af19cd6f5fb2d9f5ab9b09e2d5c78bfdac6e9a245b9971710fdc976605","citation":"549-675,780-887,904-1068"} | met |  |
| P20-SDK-CORE-045 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-rust-differences.md L99 | {"path":"crates/opi-ai/src/stream.rs","sha256":"5d5866af19cd6f5fb2d9f5ab9b09e2d5c78bfdac6e9a245b9971710fdc976605","citation":"549-675,780-887,904-1068"} | not-assessable | P20-AUD-018 |
| P20-SDK-CORE-046 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-rust-differences.md L108 | {"path":"crates/opi-ai/src/stream.rs","sha256":"5d5866af19cd6f5fb2d9f5ab9b09e2d5c78bfdac6e9a245b9971710fdc976605","citation":"549-675,780-887,904-1068"} | met |  |
| P20-SDK-CORE-047 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-rust-differences.md L108 | {"path":"crates/opi-ai/src/stream.rs","sha256":"5d5866af19cd6f5fb2d9f5ab9b09e2d5c78bfdac6e9a245b9971710fdc976605","citation":"549-675,780-887,904-1068"} | met |  |
| P20-SDK-CORE-048 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-rust-differences.md L219 | {"path":"crates/opi-ai/src/stream.rs","sha256":"5d5866af19cd6f5fb2d9f5ab9b09e2d5c78bfdac6e9a245b9971710fdc976605","citation":"549-675,780-887,904-1068"} | met |  |
| P20-SDK-FACTORY-001 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L62 | {"path":"crates/opi-ai/src/providers/mod.rs","sha256":"a8ca80084bb44e4910c8c72a98fd2ea4fc223e3291b83fe22772621652e6142f","citation":"builtin_provider_profiles/builtin_profile; exac | met |  |
| P20-SDK-FACTORY-002 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L62 | {"path":"crates/opi-ai/src/provider_collection.rs","sha256":"fce9b8186f864933ce0cf54e30a1c9612625845c7e3419ca05ffc7fecf51de32","citation":"L586–696,L719–962,L1064–1237; generic per | met |  |
| P20-SDK-FACTORY-003 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L62 | {"path":"crates/opi-ai/src/provider_collection.rs","sha256":"fce9b8186f864933ce0cf54e30a1c9612625845c7e3419ca05ffc7fecf51de32","citation":"L586–696,L719–962,L1064–1237; generic per | met |  |
| P20-SDK-FACTORY-004 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L62 | {"path":"crates/opi-ai/src/provider_collection.rs","sha256":"fce9b8186f864933ce0cf54e30a1c9612625845c7e3419ca05ffc7fecf51de32","citation":"L586–696,L719–962,L1064–1237; generic per | met |  |
| P20-SDK-FACTORY-005 | docs/research/pi-ai-opi-ai-alignment/acceptance-matrix.md L255 | {"path":"crates/opi-ai/src/provider_collection.rs","sha256":"fce9b8186f864933ce0cf54e30a1c9612625845c7e3419ca05ffc7fecf51de32","citation":"L586–696,L719–962,L1064–1237; generic per | met |  |
| P20-SDK-FACTORY-006 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L63 | {"path":"crates/opi-ai/src/providers/mod.rs","sha256":"a8ca80084bb44e4910c8c72a98fd2ea4fc223e3291b83fe22772621652e6142f","citation":"builtin_provider_profiles/builtin_profile; exac | met |  |
| P20-SDK-FACTORY-007 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L63 | {"path":"crates/opi-ai/src/provider_collection.rs","sha256":"fce9b8186f864933ce0cf54e30a1c9612625845c7e3419ca05ffc7fecf51de32","citation":"L586–696,L719–962,L1064–1237; generic per | met |  |
| P20-SDK-FACTORY-008 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L63 | {"path":"crates/opi-ai/src/provider_collection.rs","sha256":"fce9b8186f864933ce0cf54e30a1c9612625845c7e3419ca05ffc7fecf51de32","citation":"L586–696,L719–962,L1064–1237; generic per | met |  |
| P20-SDK-FACTORY-009 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L63 | {"path":"crates/opi-ai/src/provider_collection.rs","sha256":"fce9b8186f864933ce0cf54e30a1c9612625845c7e3419ca05ffc7fecf51de32","citation":"L586–696,L719–962,L1064–1237; generic per | met |  |
| P20-SDK-FACTORY-010 | docs/research/pi-ai-opi-ai-alignment/acceptance-matrix.md L255 | {"path":"crates/opi-ai/src/provider_collection.rs","sha256":"fce9b8186f864933ce0cf54e30a1c9612625845c7e3419ca05ffc7fecf51de32","citation":"L586–696,L719–962,L1064–1237; generic per | met |  |
| P20-SDK-FACTORY-011 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L64 | {"path":"crates/opi-ai/src/providers/mod.rs","sha256":"a8ca80084bb44e4910c8c72a98fd2ea4fc223e3291b83fe22772621652e6142f","citation":"builtin_provider_profiles/builtin_profile; exac | met |  |
| P20-SDK-FACTORY-012 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L64 | {"path":"crates/opi-ai/src/provider_collection.rs","sha256":"fce9b8186f864933ce0cf54e30a1c9612625845c7e3419ca05ffc7fecf51de32","citation":"L586–696,L719–962,L1064–1237; generic per | met |  |
| P20-SDK-FACTORY-013 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L64 | {"path":"crates/opi-ai/src/provider_collection.rs","sha256":"fce9b8186f864933ce0cf54e30a1c9612625845c7e3419ca05ffc7fecf51de32","citation":"L586–696,L719–962,L1064–1237; generic per | met |  |
| P20-SDK-FACTORY-014 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L64 | {"path":"crates/opi-ai/src/provider_collection.rs","sha256":"fce9b8186f864933ce0cf54e30a1c9612625845c7e3419ca05ffc7fecf51de32","citation":"L586–696,L719–962,L1064–1237; generic per | met |  |
| P20-SDK-FACTORY-015 | docs/research/pi-ai-opi-ai-alignment/acceptance-matrix.md L255 | {"path":"crates/opi-ai/src/provider_collection.rs","sha256":"fce9b8186f864933ce0cf54e30a1c9612625845c7e3419ca05ffc7fecf51de32","citation":"L586–696,L719–962,L1064–1237; generic per | not-met | P20-AUD-009 |
| P20-SDK-FACTORY-016 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L65 | {"path":"crates/opi-ai/src/providers/mod.rs","sha256":"a8ca80084bb44e4910c8c72a98fd2ea4fc223e3291b83fe22772621652e6142f","citation":"builtin_provider_profiles/builtin_profile; exac | met |  |
| P20-SDK-FACTORY-017 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L65 | {"path":"crates/opi-ai/src/provider_collection.rs","sha256":"fce9b8186f864933ce0cf54e30a1c9612625845c7e3419ca05ffc7fecf51de32","citation":"L586–696,L719–962,L1064–1237; generic per | met |  |
| P20-SDK-FACTORY-018 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L65 | {"path":"crates/opi-ai/src/provider_collection.rs","sha256":"fce9b8186f864933ce0cf54e30a1c9612625845c7e3419ca05ffc7fecf51de32","citation":"L586–696,L719–962,L1064–1237; generic per | met |  |
| P20-SDK-FACTORY-019 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L65 | {"path":"crates/opi-ai/src/provider_collection.rs","sha256":"fce9b8186f864933ce0cf54e30a1c9612625845c7e3419ca05ffc7fecf51de32","citation":"L586–696,L719–962,L1064–1237; generic per | met |  |
| P20-SDK-FACTORY-020 | docs/research/pi-ai-opi-ai-alignment/acceptance-matrix.md L255 | {"path":"crates/opi-ai/src/provider_collection.rs","sha256":"fce9b8186f864933ce0cf54e30a1c9612625845c7e3419ca05ffc7fecf51de32","citation":"L586–696,L719–962,L1064–1237; generic per | met |  |
| P20-SDK-FACTORY-021 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L66 | {"path":"crates/opi-ai/src/providers/mod.rs","sha256":"a8ca80084bb44e4910c8c72a98fd2ea4fc223e3291b83fe22772621652e6142f","citation":"builtin_provider_profiles/builtin_profile; exac | met |  |
| P20-SDK-FACTORY-022 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L66 | {"path":"crates/opi-ai/src/provider_collection.rs","sha256":"fce9b8186f864933ce0cf54e30a1c9612625845c7e3419ca05ffc7fecf51de32","citation":"L586–696,L719–962,L1064–1237; generic per | met |  |
| P20-SDK-FACTORY-023 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L66 | {"path":"crates/opi-ai/src/provider_collection.rs","sha256":"fce9b8186f864933ce0cf54e30a1c9612625845c7e3419ca05ffc7fecf51de32","citation":"L586–696,L719–962,L1064–1237; generic per | met |  |
| P20-SDK-FACTORY-024 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L66 | {"path":"crates/opi-ai/src/provider_collection.rs","sha256":"fce9b8186f864933ce0cf54e30a1c9612625845c7e3419ca05ffc7fecf51de32","citation":"L586–696,L719–962,L1064–1237; generic per | met |  |
| P20-SDK-FACTORY-025 | docs/research/pi-ai-opi-ai-alignment/acceptance-matrix.md L255 | {"path":"crates/opi-ai/src/provider_collection.rs","sha256":"fce9b8186f864933ce0cf54e30a1c9612625845c7e3419ca05ffc7fecf51de32","citation":"L586–696,L719–962,L1064–1237; generic per | met |  |
| P20-SDK-FACTORY-026 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L67 | {"path":"crates/opi-ai/src/providers/mod.rs","sha256":"a8ca80084bb44e4910c8c72a98fd2ea4fc223e3291b83fe22772621652e6142f","citation":"builtin_provider_profiles/builtin_profile; exac | met |  |
| P20-SDK-FACTORY-027 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L67 | {"path":"crates/opi-ai/src/provider_collection.rs","sha256":"fce9b8186f864933ce0cf54e30a1c9612625845c7e3419ca05ffc7fecf51de32","citation":"L586–696,L719–962,L1064–1237; generic per | met |  |
| P20-SDK-FACTORY-028 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L67 | {"path":"crates/opi-ai/src/provider_collection.rs","sha256":"fce9b8186f864933ce0cf54e30a1c9612625845c7e3419ca05ffc7fecf51de32","citation":"L586–696,L719–962,L1064–1237; generic per | met |  |
| P20-SDK-FACTORY-029 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L67 | {"path":"crates/opi-ai/src/provider_collection.rs","sha256":"fce9b8186f864933ce0cf54e30a1c9612625845c7e3419ca05ffc7fecf51de32","citation":"L586–696,L719–962,L1064–1237; generic per | met |  |
| P20-SDK-FACTORY-030 | docs/research/pi-ai-opi-ai-alignment/acceptance-matrix.md L255 | {"path":"crates/opi-ai/src/provider_collection.rs","sha256":"fce9b8186f864933ce0cf54e30a1c9612625845c7e3419ca05ffc7fecf51de32","citation":"L586–696,L719–962,L1064–1237; generic per | met |  |
| P20-SDK-FACTORY-031 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L68 | {"path":"crates/opi-ai/src/providers/mod.rs","sha256":"a8ca80084bb44e4910c8c72a98fd2ea4fc223e3291b83fe22772621652e6142f","citation":"builtin_provider_profiles/builtin_profile; exac | met |  |
| P20-SDK-FACTORY-032 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L68 | {"path":"crates/opi-ai/src/provider_collection.rs","sha256":"fce9b8186f864933ce0cf54e30a1c9612625845c7e3419ca05ffc7fecf51de32","citation":"L586–696,L719–962,L1064–1237; generic per | met |  |
| P20-SDK-FACTORY-033 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L68 | {"path":"crates/opi-ai/src/provider_collection.rs","sha256":"fce9b8186f864933ce0cf54e30a1c9612625845c7e3419ca05ffc7fecf51de32","citation":"L586–696,L719–962,L1064–1237; generic per | met |  |
| P20-SDK-FACTORY-034 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L68 | {"path":"crates/opi-ai/src/provider_collection.rs","sha256":"fce9b8186f864933ce0cf54e30a1c9612625845c7e3419ca05ffc7fecf51de32","citation":"L586–696,L719–962,L1064–1237; generic per | met |  |
| P20-SDK-FACTORY-035 | docs/research/pi-ai-opi-ai-alignment/acceptance-matrix.md L255 | {"path":"crates/opi-ai/src/provider_collection.rs","sha256":"fce9b8186f864933ce0cf54e30a1c9612625845c7e3419ca05ffc7fecf51de32","citation":"L586–696,L719–962,L1064–1237; generic per | met |  |
| P20-SDK-FACTORY-036 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L69 | {"path":"crates/opi-ai/src/providers/mod.rs","sha256":"a8ca80084bb44e4910c8c72a98fd2ea4fc223e3291b83fe22772621652e6142f","citation":"builtin_provider_profiles/builtin_profile; exac | met |  |
| P20-SDK-FACTORY-037 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L69 | {"path":"crates/opi-ai/src/provider_collection.rs","sha256":"fce9b8186f864933ce0cf54e30a1c9612625845c7e3419ca05ffc7fecf51de32","citation":"L586–696,L719–962,L1064–1237; generic per | met |  |
| P20-SDK-FACTORY-038 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L69 | {"path":"crates/opi-ai/src/provider_collection.rs","sha256":"fce9b8186f864933ce0cf54e30a1c9612625845c7e3419ca05ffc7fecf51de32","citation":"L586–696,L719–962,L1064–1237; generic per | met |  |
| P20-SDK-FACTORY-039 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L69 | {"path":"crates/opi-ai/src/provider_collection.rs","sha256":"fce9b8186f864933ce0cf54e30a1c9612625845c7e3419ca05ffc7fecf51de32","citation":"L586–696,L719–962,L1064–1237; generic per | met |  |
| P20-SDK-FACTORY-040 | docs/research/pi-ai-opi-ai-alignment/acceptance-matrix.md L255 | {"path":"crates/opi-ai/src/provider_collection.rs","sha256":"fce9b8186f864933ce0cf54e30a1c9612625845c7e3419ca05ffc7fecf51de32","citation":"L586–696,L719–962,L1064–1237; generic per | met |  |
| P20-SDK-FACTORY-041 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L70 | {"path":"crates/opi-ai/src/providers/mod.rs","sha256":"a8ca80084bb44e4910c8c72a98fd2ea4fc223e3291b83fe22772621652e6142f","citation":"builtin_provider_profiles/builtin_profile; exac | met |  |
| P20-SDK-FACTORY-042 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L70 | {"path":"crates/opi-ai/src/provider_collection.rs","sha256":"fce9b8186f864933ce0cf54e30a1c9612625845c7e3419ca05ffc7fecf51de32","citation":"L586–696,L719–962,L1064–1237; generic per | met |  |
| P20-SDK-FACTORY-043 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L70 | {"path":"crates/opi-ai/src/provider_collection.rs","sha256":"fce9b8186f864933ce0cf54e30a1c9612625845c7e3419ca05ffc7fecf51de32","citation":"L586–696,L719–962,L1064–1237; generic per | met |  |
| P20-SDK-FACTORY-044 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L70 | {"path":"crates/opi-ai/src/provider_collection.rs","sha256":"fce9b8186f864933ce0cf54e30a1c9612625845c7e3419ca05ffc7fecf51de32","citation":"L586–696,L719–962,L1064–1237; generic per | met |  |
| P20-SDK-FACTORY-045 | docs/research/pi-ai-opi-ai-alignment/acceptance-matrix.md L255 | {"path":"crates/opi-ai/src/provider_collection.rs","sha256":"fce9b8186f864933ce0cf54e30a1c9612625845c7e3419ca05ffc7fecf51de32","citation":"L586–696,L719–962,L1064–1237; generic per | met |  |
| P20-SDK-FACTORY-046 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L71 | {"path":"crates/opi-ai/src/providers/mod.rs","sha256":"a8ca80084bb44e4910c8c72a98fd2ea4fc223e3291b83fe22772621652e6142f","citation":"builtin_provider_profiles/builtin_profile; exac | met |  |
| P20-SDK-FACTORY-047 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L71 | {"path":"crates/opi-ai/src/provider_collection.rs","sha256":"fce9b8186f864933ce0cf54e30a1c9612625845c7e3419ca05ffc7fecf51de32","citation":"L586–696,L719–962,L1064–1237; generic per | met |  |
| P20-SDK-FACTORY-048 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L71 | {"path":"crates/opi-ai/src/provider_collection.rs","sha256":"fce9b8186f864933ce0cf54e30a1c9612625845c7e3419ca05ffc7fecf51de32","citation":"L586–696,L719–962,L1064–1237; generic per | met |  |
| P20-SDK-FACTORY-049 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L71 | {"path":"crates/opi-ai/src/provider_collection.rs","sha256":"fce9b8186f864933ce0cf54e30a1c9612625845c7e3419ca05ffc7fecf51de32","citation":"L586–696,L719–962,L1064–1237; generic per | met |  |
| P20-SDK-FACTORY-050 | docs/research/pi-ai-opi-ai-alignment/acceptance-matrix.md L255 | {"path":"crates/opi-ai/src/provider_collection.rs","sha256":"fce9b8186f864933ce0cf54e30a1c9612625845c7e3419ca05ffc7fecf51de32","citation":"L586–696,L719–962,L1064–1237; generic per | not-met | P20-AUD-009 |
| P20-SDK-FACTORY-051 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L72 | {"path":"crates/opi-ai/src/providers/mod.rs","sha256":"a8ca80084bb44e4910c8c72a98fd2ea4fc223e3291b83fe22772621652e6142f","citation":"builtin_provider_profiles/builtin_profile; exac | met |  |
| P20-SDK-FACTORY-052 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L72 | {"path":"crates/opi-ai/src/provider_collection.rs","sha256":"fce9b8186f864933ce0cf54e30a1c9612625845c7e3419ca05ffc7fecf51de32","citation":"L586–696,L719–962,L1064–1237; generic per | met |  |
| P20-SDK-FACTORY-053 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L72 | {"path":"crates/opi-ai/src/provider_collection.rs","sha256":"fce9b8186f864933ce0cf54e30a1c9612625845c7e3419ca05ffc7fecf51de32","citation":"L586–696,L719–962,L1064–1237; generic per | met |  |
| P20-SDK-FACTORY-054 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L72 | {"path":"crates/opi-ai/src/provider_collection.rs","sha256":"fce9b8186f864933ce0cf54e30a1c9612625845c7e3419ca05ffc7fecf51de32","citation":"L586–696,L719–962,L1064–1237; generic per | met |  |
| P20-SDK-FACTORY-055 | docs/research/pi-ai-opi-ai-alignment/acceptance-matrix.md L255 | {"path":"crates/opi-ai/src/provider_collection.rs","sha256":"fce9b8186f864933ce0cf54e30a1c9612625845c7e3419ca05ffc7fecf51de32","citation":"L586–696,L719–962,L1064–1237; generic per | met |  |
| P20-SDK-FACTORY-056 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L73 | {"path":"crates/opi-ai/src/providers/mod.rs","sha256":"a8ca80084bb44e4910c8c72a98fd2ea4fc223e3291b83fe22772621652e6142f","citation":"builtin_provider_profiles/builtin_profile; exac | met |  |
| P20-SDK-FACTORY-057 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L73 | {"path":"crates/opi-ai/src/provider_collection.rs","sha256":"fce9b8186f864933ce0cf54e30a1c9612625845c7e3419ca05ffc7fecf51de32","citation":"L586–696,L719–962,L1064–1237; generic per | met |  |
| P20-SDK-FACTORY-058 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L73 | {"path":"crates/opi-ai/src/provider_collection.rs","sha256":"fce9b8186f864933ce0cf54e30a1c9612625845c7e3419ca05ffc7fecf51de32","citation":"L586–696,L719–962,L1064–1237; generic per | met |  |
| P20-SDK-FACTORY-059 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L73 | {"path":"crates/opi-ai/src/provider_collection.rs","sha256":"fce9b8186f864933ce0cf54e30a1c9612625845c7e3419ca05ffc7fecf51de32","citation":"L586–696,L719–962,L1064–1237; generic per | met |  |
| P20-SDK-FACTORY-060 | docs/research/pi-ai-opi-ai-alignment/acceptance-matrix.md L255 | {"path":"crates/opi-ai/src/provider_collection.rs","sha256":"fce9b8186f864933ce0cf54e30a1c9612625845c7e3419ca05ffc7fecf51de32","citation":"L586–696,L719–962,L1064–1237; generic per | met |  |
| P20-SDK-FACTORY-061 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L74 | {"path":"crates/opi-ai/src/providers/mod.rs","sha256":"a8ca80084bb44e4910c8c72a98fd2ea4fc223e3291b83fe22772621652e6142f","citation":"builtin_provider_profiles/builtin_profile; exac | met |  |
| P20-SDK-FACTORY-062 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L74 | {"path":"crates/opi-ai/src/provider_collection.rs","sha256":"fce9b8186f864933ce0cf54e30a1c9612625845c7e3419ca05ffc7fecf51de32","citation":"L586–696,L719–962,L1064–1237; generic per | met |  |
| P20-SDK-FACTORY-063 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L74 | {"path":"crates/opi-ai/src/provider_collection.rs","sha256":"fce9b8186f864933ce0cf54e30a1c9612625845c7e3419ca05ffc7fecf51de32","citation":"L586–696,L719–962,L1064–1237; generic per | met |  |
| P20-SDK-FACTORY-064 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L74 | {"path":"crates/opi-ai/src/provider_collection.rs","sha256":"fce9b8186f864933ce0cf54e30a1c9612625845c7e3419ca05ffc7fecf51de32","citation":"L586–696,L719–962,L1064–1237; generic per | met |  |
| P20-SDK-FACTORY-065 | docs/research/pi-ai-opi-ai-alignment/acceptance-matrix.md L255 | {"path":"crates/opi-ai/src/provider_collection.rs","sha256":"fce9b8186f864933ce0cf54e30a1c9612625845c7e3419ca05ffc7fecf51de32","citation":"L586–696,L719–962,L1064–1237; generic per | met |  |
| P20-SDK-FACTORY-066 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L75 | {"path":"crates/opi-ai/src/providers/mod.rs","sha256":"a8ca80084bb44e4910c8c72a98fd2ea4fc223e3291b83fe22772621652e6142f","citation":"builtin_provider_profiles/builtin_profile; exac | met |  |
| P20-SDK-FACTORY-067 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L75 | {"path":"crates/opi-ai/src/provider_collection.rs","sha256":"fce9b8186f864933ce0cf54e30a1c9612625845c7e3419ca05ffc7fecf51de32","citation":"L586–696,L719–962,L1064–1237; generic per | met |  |
| P20-SDK-FACTORY-068 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L75 | {"path":"crates/opi-ai/src/provider_collection.rs","sha256":"fce9b8186f864933ce0cf54e30a1c9612625845c7e3419ca05ffc7fecf51de32","citation":"L586–696,L719–962,L1064–1237; generic per | met |  |
| P20-SDK-FACTORY-069 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L75 | {"path":"crates/opi-ai/src/provider_collection.rs","sha256":"fce9b8186f864933ce0cf54e30a1c9612625845c7e3419ca05ffc7fecf51de32","citation":"L586–696,L719–962,L1064–1237; generic per | met |  |
| P20-SDK-FACTORY-070 | docs/research/pi-ai-opi-ai-alignment/acceptance-matrix.md L255 | {"path":"crates/opi-ai/src/provider_collection.rs","sha256":"fce9b8186f864933ce0cf54e30a1c9612625845c7e3419ca05ffc7fecf51de32","citation":"L586–696,L719–962,L1064–1237; generic per | met |  |
| P20-SDK-FACTORY-071 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L76 | {"path":"crates/opi-ai/src/providers/mod.rs","sha256":"a8ca80084bb44e4910c8c72a98fd2ea4fc223e3291b83fe22772621652e6142f","citation":"builtin_provider_profiles/builtin_profile; exac | met |  |
| P20-SDK-FACTORY-072 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L76 | {"path":"crates/opi-ai/src/provider_collection.rs","sha256":"fce9b8186f864933ce0cf54e30a1c9612625845c7e3419ca05ffc7fecf51de32","citation":"L586–696,L719–962,L1064–1237; generic per | met |  |
| P20-SDK-FACTORY-073 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L76 | {"path":"crates/opi-ai/src/provider_collection.rs","sha256":"fce9b8186f864933ce0cf54e30a1c9612625845c7e3419ca05ffc7fecf51de32","citation":"L586–696,L719–962,L1064–1237; generic per | met |  |
| P20-SDK-FACTORY-074 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L76 | {"path":"crates/opi-ai/src/provider_collection.rs","sha256":"fce9b8186f864933ce0cf54e30a1c9612625845c7e3419ca05ffc7fecf51de32","citation":"L586–696,L719–962,L1064–1237; generic per | met |  |
| P20-SDK-FACTORY-075 | docs/research/pi-ai-opi-ai-alignment/acceptance-matrix.md L255 | {"path":"crates/opi-ai/src/provider_collection.rs","sha256":"fce9b8186f864933ce0cf54e30a1c9612625845c7e3419ca05ffc7fecf51de32","citation":"L586–696,L719–962,L1064–1237; generic per | met |  |
| P20-SDK-FACTORY-076 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L77 | {"path":"crates/opi-ai/src/providers/mod.rs","sha256":"a8ca80084bb44e4910c8c72a98fd2ea4fc223e3291b83fe22772621652e6142f","citation":"builtin_provider_profiles/builtin_profile; exac | met |  |
| P20-SDK-FACTORY-077 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L77 | {"path":"crates/opi-ai/src/provider_collection.rs","sha256":"fce9b8186f864933ce0cf54e30a1c9612625845c7e3419ca05ffc7fecf51de32","citation":"L586–696,L719–962,L1064–1237; generic per | met |  |
| P20-SDK-FACTORY-078 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L77 | {"path":"crates/opi-ai/src/provider_collection.rs","sha256":"fce9b8186f864933ce0cf54e30a1c9612625845c7e3419ca05ffc7fecf51de32","citation":"L586–696,L719–962,L1064–1237; generic per | met |  |
| P20-SDK-FACTORY-079 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L77 | {"path":"crates/opi-ai/src/provider_collection.rs","sha256":"fce9b8186f864933ce0cf54e30a1c9612625845c7e3419ca05ffc7fecf51de32","citation":"L586–696,L719–962,L1064–1237; generic per | met |  |
| P20-SDK-FACTORY-080 | docs/research/pi-ai-opi-ai-alignment/acceptance-matrix.md L255 | {"path":"crates/opi-ai/src/provider_collection.rs","sha256":"fce9b8186f864933ce0cf54e30a1c9612625845c7e3419ca05ffc7fecf51de32","citation":"L586–696,L719–962,L1064–1237; generic per | met |  |
| P20-SDK-FACTORY-081 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L78 | {"path":"crates/opi-ai/src/providers/mod.rs","sha256":"a8ca80084bb44e4910c8c72a98fd2ea4fc223e3291b83fe22772621652e6142f","citation":"builtin_provider_profiles/builtin_profile; exac | met |  |
| P20-SDK-FACTORY-082 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L78 | {"path":"crates/opi-ai/src/provider_collection.rs","sha256":"fce9b8186f864933ce0cf54e30a1c9612625845c7e3419ca05ffc7fecf51de32","citation":"L586–696,L719–962,L1064–1237; generic per | met |  |
| P20-SDK-FACTORY-083 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L78 | {"path":"crates/opi-ai/src/provider_collection.rs","sha256":"fce9b8186f864933ce0cf54e30a1c9612625845c7e3419ca05ffc7fecf51de32","citation":"L586–696,L719–962,L1064–1237; generic per | met |  |
| P20-SDK-FACTORY-084 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L78 | {"path":"crates/opi-ai/src/provider_collection.rs","sha256":"fce9b8186f864933ce0cf54e30a1c9612625845c7e3419ca05ffc7fecf51de32","citation":"L586–696,L719–962,L1064–1237; generic per | met |  |
| P20-SDK-FACTORY-085 | docs/research/pi-ai-opi-ai-alignment/acceptance-matrix.md L255 | {"path":"crates/opi-ai/src/provider_collection.rs","sha256":"fce9b8186f864933ce0cf54e30a1c9612625845c7e3419ca05ffc7fecf51de32","citation":"L586–696,L719–962,L1064–1237; generic per | met |  |
| P20-SDK-FACTORY-086 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L79 | {"path":"crates/opi-ai/src/providers/mod.rs","sha256":"a8ca80084bb44e4910c8c72a98fd2ea4fc223e3291b83fe22772621652e6142f","citation":"builtin_provider_profiles/builtin_profile; exac | met |  |
| P20-SDK-FACTORY-087 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L79 | {"path":"crates/opi-ai/src/provider_collection.rs","sha256":"fce9b8186f864933ce0cf54e30a1c9612625845c7e3419ca05ffc7fecf51de32","citation":"L586–696,L719–962,L1064–1237; generic per | met |  |
| P20-SDK-FACTORY-088 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L79 | {"path":"crates/opi-ai/src/provider_collection.rs","sha256":"fce9b8186f864933ce0cf54e30a1c9612625845c7e3419ca05ffc7fecf51de32","citation":"L586–696,L719–962,L1064–1237; generic per | met |  |
| P20-SDK-FACTORY-089 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L79 | {"path":"crates/opi-ai/src/provider_collection.rs","sha256":"fce9b8186f864933ce0cf54e30a1c9612625845c7e3419ca05ffc7fecf51de32","citation":"L586–696,L719–962,L1064–1237; generic per | met |  |
| P20-SDK-FACTORY-090 | docs/research/pi-ai-opi-ai-alignment/acceptance-matrix.md L255 | {"path":"crates/opi-ai/src/provider_collection.rs","sha256":"fce9b8186f864933ce0cf54e30a1c9612625845c7e3419ca05ffc7fecf51de32","citation":"L586–696,L719–962,L1064–1237; generic per | met |  |
| P20-SDK-FACTORY-091 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L80 | {"path":"crates/opi-ai/src/providers/mod.rs","sha256":"a8ca80084bb44e4910c8c72a98fd2ea4fc223e3291b83fe22772621652e6142f","citation":"builtin_provider_profiles/builtin_profile; exac | met |  |
| P20-SDK-FACTORY-092 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L80 | {"path":"crates/opi-ai/src/provider_collection.rs","sha256":"fce9b8186f864933ce0cf54e30a1c9612625845c7e3419ca05ffc7fecf51de32","citation":"L586–696,L719–962,L1064–1237; generic per | met |  |
| P20-SDK-FACTORY-093 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L80 | {"path":"crates/opi-ai/src/provider_collection.rs","sha256":"fce9b8186f864933ce0cf54e30a1c9612625845c7e3419ca05ffc7fecf51de32","citation":"L586–696,L719–962,L1064–1237; generic per | met |  |
| P20-SDK-FACTORY-094 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L80 | {"path":"crates/opi-ai/src/provider_collection.rs","sha256":"fce9b8186f864933ce0cf54e30a1c9612625845c7e3419ca05ffc7fecf51de32","citation":"L586–696,L719–962,L1064–1237; generic per | met |  |
| P20-SDK-FACTORY-095 | docs/research/pi-ai-opi-ai-alignment/acceptance-matrix.md L255 | {"path":"crates/opi-ai/src/provider_collection.rs","sha256":"fce9b8186f864933ce0cf54e30a1c9612625845c7e3419ca05ffc7fecf51de32","citation":"L586–696,L719–962,L1064–1237; generic per | met |  |
| P20-SDK-FACTORY-096 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L81 | {"path":"crates/opi-ai/src/providers/mod.rs","sha256":"a8ca80084bb44e4910c8c72a98fd2ea4fc223e3291b83fe22772621652e6142f","citation":"builtin_provider_profiles/builtin_profile; exac | met |  |
| P20-SDK-FACTORY-097 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L81 | {"path":"crates/opi-ai/src/provider_collection.rs","sha256":"fce9b8186f864933ce0cf54e30a1c9612625845c7e3419ca05ffc7fecf51de32","citation":"L586–696,L719–962,L1064–1237; generic per | met |  |
| P20-SDK-FACTORY-098 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L81 | {"path":"crates/opi-ai/src/provider_collection.rs","sha256":"fce9b8186f864933ce0cf54e30a1c9612625845c7e3419ca05ffc7fecf51de32","citation":"L586–696,L719–962,L1064–1237; generic per | met |  |
| P20-SDK-FACTORY-099 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L81 | {"path":"crates/opi-ai/src/provider_collection.rs","sha256":"fce9b8186f864933ce0cf54e30a1c9612625845c7e3419ca05ffc7fecf51de32","citation":"L586–696,L719–962,L1064–1237; generic per | met |  |
| P20-SDK-FACTORY-100 | docs/research/pi-ai-opi-ai-alignment/acceptance-matrix.md L255 | {"path":"crates/opi-ai/src/provider_collection.rs","sha256":"fce9b8186f864933ce0cf54e30a1c9612625845c7e3419ca05ffc7fecf51de32","citation":"L586–696,L719–962,L1064–1237; generic per | met |  |
| P20-SDK-FACTORY-101 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L82 | {"path":"crates/opi-ai/src/providers/mod.rs","sha256":"a8ca80084bb44e4910c8c72a98fd2ea4fc223e3291b83fe22772621652e6142f","citation":"builtin_provider_profiles/builtin_profile; exac | met |  |
| P20-SDK-FACTORY-102 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L82 | {"path":"crates/opi-ai/src/provider_collection.rs","sha256":"fce9b8186f864933ce0cf54e30a1c9612625845c7e3419ca05ffc7fecf51de32","citation":"L586–696,L719–962,L1064–1237; generic per | met |  |
| P20-SDK-FACTORY-103 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L82 | {"path":"crates/opi-ai/src/provider_collection.rs","sha256":"fce9b8186f864933ce0cf54e30a1c9612625845c7e3419ca05ffc7fecf51de32","citation":"L586–696,L719–962,L1064–1237; generic per | met |  |
| P20-SDK-FACTORY-104 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L82 | {"path":"crates/opi-ai/src/provider_collection.rs","sha256":"fce9b8186f864933ce0cf54e30a1c9612625845c7e3419ca05ffc7fecf51de32","citation":"L586–696,L719–962,L1064–1237; generic per | met |  |
| P20-SDK-FACTORY-105 | docs/research/pi-ai-opi-ai-alignment/acceptance-matrix.md L255 | {"path":"crates/opi-ai/src/provider_collection.rs","sha256":"fce9b8186f864933ce0cf54e30a1c9612625845c7e3419ca05ffc7fecf51de32","citation":"L586–696,L719–962,L1064–1237; generic per | met |  |
| P20-SDK-FACTORY-106 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L83 | {"path":"crates/opi-ai/src/providers/mod.rs","sha256":"a8ca80084bb44e4910c8c72a98fd2ea4fc223e3291b83fe22772621652e6142f","citation":"builtin_provider_profiles/builtin_profile; exac | met |  |
| P20-SDK-FACTORY-107 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L83 | {"path":"crates/opi-ai/src/provider_collection.rs","sha256":"fce9b8186f864933ce0cf54e30a1c9612625845c7e3419ca05ffc7fecf51de32","citation":"L586–696,L719–962,L1064–1237; generic per | met |  |
| P20-SDK-FACTORY-108 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L83 | {"path":"crates/opi-ai/src/provider_collection.rs","sha256":"fce9b8186f864933ce0cf54e30a1c9612625845c7e3419ca05ffc7fecf51de32","citation":"L586–696,L719–962,L1064–1237; generic per | met |  |
| P20-SDK-FACTORY-109 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L83 | {"path":"crates/opi-ai/src/provider_collection.rs","sha256":"fce9b8186f864933ce0cf54e30a1c9612625845c7e3419ca05ffc7fecf51de32","citation":"L586–696,L719–962,L1064–1237; generic per | met |  |
| P20-SDK-FACTORY-110 | docs/research/pi-ai-opi-ai-alignment/acceptance-matrix.md L255 | {"path":"crates/opi-ai/src/provider_collection.rs","sha256":"fce9b8186f864933ce0cf54e30a1c9612625845c7e3419ca05ffc7fecf51de32","citation":"L586–696,L719–962,L1064–1237; generic per | met |  |
| P20-SDK-FACTORY-111 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L84 | {"path":"crates/opi-ai/src/providers/mod.rs","sha256":"a8ca80084bb44e4910c8c72a98fd2ea4fc223e3291b83fe22772621652e6142f","citation":"builtin_provider_profiles/builtin_profile; exac | met |  |
| P20-SDK-FACTORY-112 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L84 | {"path":"crates/opi-ai/src/provider_collection.rs","sha256":"fce9b8186f864933ce0cf54e30a1c9612625845c7e3419ca05ffc7fecf51de32","citation":"L586–696,L719–962,L1064–1237; generic per | met |  |
| P20-SDK-FACTORY-113 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L84 | {"path":"crates/opi-ai/src/provider_collection.rs","sha256":"fce9b8186f864933ce0cf54e30a1c9612625845c7e3419ca05ffc7fecf51de32","citation":"L586–696,L719–962,L1064–1237; generic per | met |  |
| P20-SDK-FACTORY-114 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L84 | {"path":"crates/opi-ai/src/provider_collection.rs","sha256":"fce9b8186f864933ce0cf54e30a1c9612625845c7e3419ca05ffc7fecf51de32","citation":"L586–696,L719–962,L1064–1237; generic per | met |  |
| P20-SDK-FACTORY-115 | docs/research/pi-ai-opi-ai-alignment/acceptance-matrix.md L255 | {"path":"crates/opi-ai/src/provider_collection.rs","sha256":"fce9b8186f864933ce0cf54e30a1c9612625845c7e3419ca05ffc7fecf51de32","citation":"L586–696,L719–962,L1064–1237; generic per | met |  |
| P20-SDK-FACTORY-116 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L85 | {"path":"crates/opi-ai/src/providers/mod.rs","sha256":"a8ca80084bb44e4910c8c72a98fd2ea4fc223e3291b83fe22772621652e6142f","citation":"builtin_provider_profiles/builtin_profile; exac | met |  |
| P20-SDK-FACTORY-117 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L85 | {"path":"crates/opi-ai/src/provider_collection.rs","sha256":"fce9b8186f864933ce0cf54e30a1c9612625845c7e3419ca05ffc7fecf51de32","citation":"L586–696,L719–962,L1064–1237; generic per | met |  |
| P20-SDK-FACTORY-118 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L85 | {"path":"crates/opi-ai/src/provider_collection.rs","sha256":"fce9b8186f864933ce0cf54e30a1c9612625845c7e3419ca05ffc7fecf51de32","citation":"L586–696,L719–962,L1064–1237; generic per | met |  |
| P20-SDK-FACTORY-119 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L85 | {"path":"crates/opi-ai/src/provider_collection.rs","sha256":"fce9b8186f864933ce0cf54e30a1c9612625845c7e3419ca05ffc7fecf51de32","citation":"L586–696,L719–962,L1064–1237; generic per | met |  |
| P20-SDK-FACTORY-120 | docs/research/pi-ai-opi-ai-alignment/acceptance-matrix.md L255 | {"path":"crates/opi-ai/src/provider_collection.rs","sha256":"fce9b8186f864933ce0cf54e30a1c9612625845c7e3419ca05ffc7fecf51de32","citation":"L586–696,L719–962,L1064–1237; generic per | met |  |
| P20-SDK-FACTORY-121 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L86 | {"path":"crates/opi-ai/src/providers/mod.rs","sha256":"a8ca80084bb44e4910c8c72a98fd2ea4fc223e3291b83fe22772621652e6142f","citation":"builtin_provider_profiles/builtin_profile; exac | met |  |
| P20-SDK-FACTORY-122 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L86 | {"path":"crates/opi-ai/src/provider_collection.rs","sha256":"fce9b8186f864933ce0cf54e30a1c9612625845c7e3419ca05ffc7fecf51de32","citation":"L586–696,L719–962,L1064–1237; generic per | met |  |
| P20-SDK-FACTORY-123 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L86 | {"path":"crates/opi-ai/src/provider_collection.rs","sha256":"fce9b8186f864933ce0cf54e30a1c9612625845c7e3419ca05ffc7fecf51de32","citation":"L586–696,L719–962,L1064–1237; generic per | met |  |
| P20-SDK-FACTORY-124 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L86 | {"path":"crates/opi-ai/src/provider_collection.rs","sha256":"fce9b8186f864933ce0cf54e30a1c9612625845c7e3419ca05ffc7fecf51de32","citation":"L586–696,L719–962,L1064–1237; generic per | met |  |
| P20-SDK-FACTORY-125 | docs/research/pi-ai-opi-ai-alignment/acceptance-matrix.md L255 | {"path":"crates/opi-ai/src/provider_collection.rs","sha256":"fce9b8186f864933ce0cf54e30a1c9612625845c7e3419ca05ffc7fecf51de32","citation":"L586–696,L719–962,L1064–1237; generic per | met |  |
| P20-SDK-FACTORY-126 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L87 | {"path":"crates/opi-ai/src/providers/mod.rs","sha256":"a8ca80084bb44e4910c8c72a98fd2ea4fc223e3291b83fe22772621652e6142f","citation":"builtin_provider_profiles/builtin_profile; exac | met |  |
| P20-SDK-FACTORY-127 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L87 | {"path":"crates/opi-ai/src/provider_collection.rs","sha256":"fce9b8186f864933ce0cf54e30a1c9612625845c7e3419ca05ffc7fecf51de32","citation":"L586–696,L719–962,L1064–1237; generic per | met |  |
| P20-SDK-FACTORY-128 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L87 | {"path":"crates/opi-ai/src/provider_collection.rs","sha256":"fce9b8186f864933ce0cf54e30a1c9612625845c7e3419ca05ffc7fecf51de32","citation":"L586–696,L719–962,L1064–1237; generic per | met |  |
| P20-SDK-FACTORY-129 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L87 | {"path":"crates/opi-ai/src/provider_collection.rs","sha256":"fce9b8186f864933ce0cf54e30a1c9612625845c7e3419ca05ffc7fecf51de32","citation":"L586–696,L719–962,L1064–1237; generic per | met |  |
| P20-SDK-FACTORY-130 | docs/research/pi-ai-opi-ai-alignment/acceptance-matrix.md L255 | {"path":"crates/opi-ai/src/provider_collection.rs","sha256":"fce9b8186f864933ce0cf54e30a1c9612625845c7e3419ca05ffc7fecf51de32","citation":"L586–696,L719–962,L1064–1237; generic per | not-met | P20-AUD-009 |
| P20-SDK-FACTORY-131 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L88 | {"path":"crates/opi-ai/src/providers/mod.rs","sha256":"a8ca80084bb44e4910c8c72a98fd2ea4fc223e3291b83fe22772621652e6142f","citation":"builtin_provider_profiles/builtin_profile; exac | met |  |
| P20-SDK-FACTORY-132 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L88 | {"path":"crates/opi-ai/src/provider_collection.rs","sha256":"fce9b8186f864933ce0cf54e30a1c9612625845c7e3419ca05ffc7fecf51de32","citation":"L586–696,L719–962,L1064–1237; generic per | met |  |
| P20-SDK-FACTORY-133 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L88 | {"path":"crates/opi-ai/src/provider_collection.rs","sha256":"fce9b8186f864933ce0cf54e30a1c9612625845c7e3419ca05ffc7fecf51de32","citation":"L586–696,L719–962,L1064–1237; generic per | met |  |
| P20-SDK-FACTORY-134 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L88 | {"path":"crates/opi-ai/src/provider_collection.rs","sha256":"fce9b8186f864933ce0cf54e30a1c9612625845c7e3419ca05ffc7fecf51de32","citation":"L586–696,L719–962,L1064–1237; generic per | met |  |
| P20-SDK-FACTORY-135 | docs/research/pi-ai-opi-ai-alignment/acceptance-matrix.md L255 | {"path":"crates/opi-ai/src/provider_collection.rs","sha256":"fce9b8186f864933ce0cf54e30a1c9612625845c7e3419ca05ffc7fecf51de32","citation":"L586–696,L719–962,L1064–1237; generic per | not-met | P20-AUD-009 |
| P20-SDK-FACTORY-136 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L89 | {"path":"crates/opi-ai/src/providers/mod.rs","sha256":"a8ca80084bb44e4910c8c72a98fd2ea4fc223e3291b83fe22772621652e6142f","citation":"builtin_provider_profiles/builtin_profile; exac | met |  |
| P20-SDK-FACTORY-137 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L89 | {"path":"crates/opi-ai/src/provider_collection.rs","sha256":"fce9b8186f864933ce0cf54e30a1c9612625845c7e3419ca05ffc7fecf51de32","citation":"L586–696,L719–962,L1064–1237; generic per | met |  |
| P20-SDK-FACTORY-138 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L89 | {"path":"crates/opi-ai/src/provider_collection.rs","sha256":"fce9b8186f864933ce0cf54e30a1c9612625845c7e3419ca05ffc7fecf51de32","citation":"L586–696,L719–962,L1064–1237; generic per | met |  |
| P20-SDK-FACTORY-139 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L89 | {"path":"crates/opi-ai/src/provider_collection.rs","sha256":"fce9b8186f864933ce0cf54e30a1c9612625845c7e3419ca05ffc7fecf51de32","citation":"L586–696,L719–962,L1064–1237; generic per | met |  |
| P20-SDK-FACTORY-140 | docs/research/pi-ai-opi-ai-alignment/acceptance-matrix.md L255 | {"path":"crates/opi-ai/src/provider_collection.rs","sha256":"fce9b8186f864933ce0cf54e30a1c9612625845c7e3419ca05ffc7fecf51de32","citation":"L586–696,L719–962,L1064–1237; generic per | not-met | P20-AUD-009 |
| P20-SDK-FACTORY-141 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L90 | {"path":"crates/opi-ai/src/providers/mod.rs","sha256":"a8ca80084bb44e4910c8c72a98fd2ea4fc223e3291b83fe22772621652e6142f","citation":"builtin_provider_profiles/builtin_profile; exac | met |  |
| P20-SDK-FACTORY-142 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L90 | {"path":"crates/opi-ai/src/provider_collection.rs","sha256":"fce9b8186f864933ce0cf54e30a1c9612625845c7e3419ca05ffc7fecf51de32","citation":"L586–696,L719–962,L1064–1237; generic per | met |  |
| P20-SDK-FACTORY-143 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L90 | {"path":"crates/opi-ai/src/provider_collection.rs","sha256":"fce9b8186f864933ce0cf54e30a1c9612625845c7e3419ca05ffc7fecf51de32","citation":"L586–696,L719–962,L1064–1237; generic per | met |  |
| P20-SDK-FACTORY-144 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L90 | {"path":"crates/opi-ai/src/provider_collection.rs","sha256":"fce9b8186f864933ce0cf54e30a1c9612625845c7e3419ca05ffc7fecf51de32","citation":"L586–696,L719–962,L1064–1237; generic per | met |  |
| P20-SDK-FACTORY-145 | docs/research/pi-ai-opi-ai-alignment/acceptance-matrix.md L255 | {"path":"crates/opi-ai/src/provider_collection.rs","sha256":"fce9b8186f864933ce0cf54e30a1c9612625845c7e3419ca05ffc7fecf51de32","citation":"L586–696,L719–962,L1064–1237; generic per | met |  |
| P20-SDK-FACTORY-146 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L91 | {"path":"crates/opi-ai/src/providers/mod.rs","sha256":"a8ca80084bb44e4910c8c72a98fd2ea4fc223e3291b83fe22772621652e6142f","citation":"builtin_provider_profiles/builtin_profile; exac | met |  |
| P20-SDK-FACTORY-147 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L91 | {"path":"crates/opi-ai/src/provider_collection.rs","sha256":"fce9b8186f864933ce0cf54e30a1c9612625845c7e3419ca05ffc7fecf51de32","citation":"L586–696,L719–962,L1064–1237; generic per | met |  |
| P20-SDK-FACTORY-148 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L91 | {"path":"crates/opi-ai/src/provider_collection.rs","sha256":"fce9b8186f864933ce0cf54e30a1c9612625845c7e3419ca05ffc7fecf51de32","citation":"L586–696,L719–962,L1064–1237; generic per | met |  |
| P20-SDK-FACTORY-149 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L91 | {"path":"crates/opi-ai/src/provider_collection.rs","sha256":"fce9b8186f864933ce0cf54e30a1c9612625845c7e3419ca05ffc7fecf51de32","citation":"L586–696,L719–962,L1064–1237; generic per | met |  |
| P20-SDK-FACTORY-150 | docs/research/pi-ai-opi-ai-alignment/acceptance-matrix.md L255 | {"path":"crates/opi-ai/src/provider_collection.rs","sha256":"fce9b8186f864933ce0cf54e30a1c9612625845c7e3419ca05ffc7fecf51de32","citation":"L586–696,L719–962,L1064–1237; generic per | met |  |
| P20-SDK-FACTORY-151 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L92 | {"path":"crates/opi-ai/src/providers/mod.rs","sha256":"a8ca80084bb44e4910c8c72a98fd2ea4fc223e3291b83fe22772621652e6142f","citation":"builtin_provider_profiles/builtin_profile; exac | met |  |
| P20-SDK-FACTORY-152 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L92 | {"path":"crates/opi-ai/src/provider_collection.rs","sha256":"fce9b8186f864933ce0cf54e30a1c9612625845c7e3419ca05ffc7fecf51de32","citation":"L586–696,L719–962,L1064–1237; generic per | met |  |
| P20-SDK-FACTORY-153 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L92 | {"path":"crates/opi-ai/src/provider_collection.rs","sha256":"fce9b8186f864933ce0cf54e30a1c9612625845c7e3419ca05ffc7fecf51de32","citation":"L586–696,L719–962,L1064–1237; generic per | met |  |
| P20-SDK-FACTORY-154 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L92 | {"path":"crates/opi-ai/src/provider_collection.rs","sha256":"fce9b8186f864933ce0cf54e30a1c9612625845c7e3419ca05ffc7fecf51de32","citation":"L586–696,L719–962,L1064–1237; generic per | met |  |
| P20-SDK-FACTORY-155 | docs/research/pi-ai-opi-ai-alignment/acceptance-matrix.md L255 | {"path":"crates/opi-ai/src/provider_collection.rs","sha256":"fce9b8186f864933ce0cf54e30a1c9612625845c7e3419ca05ffc7fecf51de32","citation":"L586–696,L719–962,L1064–1237; generic per | met |  |
| P20-SDK-FACTORY-156 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L93 | {"path":"crates/opi-ai/src/providers/mod.rs","sha256":"a8ca80084bb44e4910c8c72a98fd2ea4fc223e3291b83fe22772621652e6142f","citation":"builtin_provider_profiles/builtin_profile; exac | met |  |
| P20-SDK-FACTORY-157 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L93 | {"path":"crates/opi-ai/src/provider_collection.rs","sha256":"fce9b8186f864933ce0cf54e30a1c9612625845c7e3419ca05ffc7fecf51de32","citation":"L586–696,L719–962,L1064–1237; generic per | met |  |
| P20-SDK-FACTORY-158 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L93 | {"path":"crates/opi-ai/src/provider_collection.rs","sha256":"fce9b8186f864933ce0cf54e30a1c9612625845c7e3419ca05ffc7fecf51de32","citation":"L586–696,L719–962,L1064–1237; generic per | met |  |
| P20-SDK-FACTORY-159 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L93 | {"path":"crates/opi-ai/src/provider_collection.rs","sha256":"fce9b8186f864933ce0cf54e30a1c9612625845c7e3419ca05ffc7fecf51de32","citation":"L586–696,L719–962,L1064–1237; generic per | met |  |
| P20-SDK-FACTORY-160 | docs/research/pi-ai-opi-ai-alignment/acceptance-matrix.md L255 | {"path":"crates/opi-ai/src/provider_collection.rs","sha256":"fce9b8186f864933ce0cf54e30a1c9612625845c7e3419ca05ffc7fecf51de32","citation":"L586–696,L719–962,L1064–1237; generic per | met |  |
| P20-SDK-FACTORY-161 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L94 | {"path":"crates/opi-ai/src/providers/mod.rs","sha256":"a8ca80084bb44e4910c8c72a98fd2ea4fc223e3291b83fe22772621652e6142f","citation":"builtin_provider_profiles/builtin_profile; exac | met |  |
| P20-SDK-FACTORY-162 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L94 | {"path":"crates/opi-ai/src/provider_collection.rs","sha256":"fce9b8186f864933ce0cf54e30a1c9612625845c7e3419ca05ffc7fecf51de32","citation":"L586–696,L719–962,L1064–1237; generic per | met |  |
| P20-SDK-FACTORY-163 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L94 | {"path":"crates/opi-ai/src/provider_collection.rs","sha256":"fce9b8186f864933ce0cf54e30a1c9612625845c7e3419ca05ffc7fecf51de32","citation":"L586–696,L719–962,L1064–1237; generic per | met |  |
| P20-SDK-FACTORY-164 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L94 | {"path":"crates/opi-ai/src/provider_collection.rs","sha256":"fce9b8186f864933ce0cf54e30a1c9612625845c7e3419ca05ffc7fecf51de32","citation":"L586–696,L719–962,L1064–1237; generic per | met |  |
| P20-SDK-FACTORY-165 | docs/research/pi-ai-opi-ai-alignment/acceptance-matrix.md L255 | {"path":"crates/opi-ai/src/provider_collection.rs","sha256":"fce9b8186f864933ce0cf54e30a1c9612625845c7e3419ca05ffc7fecf51de32","citation":"L586–696,L719–962,L1064–1237; generic per | met |  |
| P20-SDK-FACTORY-166 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L95 | {"path":"crates/opi-ai/src/providers/mod.rs","sha256":"a8ca80084bb44e4910c8c72a98fd2ea4fc223e3291b83fe22772621652e6142f","citation":"builtin_provider_profiles/builtin_profile; exac | met |  |
| P20-SDK-FACTORY-167 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L95 | {"path":"crates/opi-ai/src/provider_collection.rs","sha256":"fce9b8186f864933ce0cf54e30a1c9612625845c7e3419ca05ffc7fecf51de32","citation":"L586–696,L719–962,L1064–1237; generic per | met |  |
| P20-SDK-FACTORY-168 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L95 | {"path":"crates/opi-ai/src/provider_collection.rs","sha256":"fce9b8186f864933ce0cf54e30a1c9612625845c7e3419ca05ffc7fecf51de32","citation":"L586–696,L719–962,L1064–1237; generic per | met |  |
| P20-SDK-FACTORY-169 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L95 | {"path":"crates/opi-ai/src/provider_collection.rs","sha256":"fce9b8186f864933ce0cf54e30a1c9612625845c7e3419ca05ffc7fecf51de32","citation":"L586–696,L719–962,L1064–1237; generic per | met |  |
| P20-SDK-FACTORY-170 | docs/research/pi-ai-opi-ai-alignment/acceptance-matrix.md L255 | {"path":"crates/opi-ai/src/provider_collection.rs","sha256":"fce9b8186f864933ce0cf54e30a1c9612625845c7e3419ca05ffc7fecf51de32","citation":"L586–696,L719–962,L1064–1237; generic per | met |  |
| P20-SDK-FACTORY-171 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L96 | {"path":"crates/opi-ai/src/providers/mod.rs","sha256":"a8ca80084bb44e4910c8c72a98fd2ea4fc223e3291b83fe22772621652e6142f","citation":"builtin_provider_profiles/builtin_profile; exac | met |  |
| P20-SDK-FACTORY-172 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L96 | {"path":"crates/opi-ai/src/provider_collection.rs","sha256":"fce9b8186f864933ce0cf54e30a1c9612625845c7e3419ca05ffc7fecf51de32","citation":"L586–696,L719–962,L1064–1237; generic per | met |  |
| P20-SDK-FACTORY-173 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L96 | {"path":"crates/opi-ai/src/provider_collection.rs","sha256":"fce9b8186f864933ce0cf54e30a1c9612625845c7e3419ca05ffc7fecf51de32","citation":"L586–696,L719–962,L1064–1237; generic per | met |  |
| P20-SDK-FACTORY-174 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L96 | {"path":"crates/opi-ai/src/provider_collection.rs","sha256":"fce9b8186f864933ce0cf54e30a1c9612625845c7e3419ca05ffc7fecf51de32","citation":"L586–696,L719–962,L1064–1237; generic per | met |  |
| P20-SDK-FACTORY-175 | docs/research/pi-ai-opi-ai-alignment/acceptance-matrix.md L255 | {"path":"crates/opi-ai/src/provider_collection.rs","sha256":"fce9b8186f864933ce0cf54e30a1c9612625845c7e3419ca05ffc7fecf51de32","citation":"L586–696,L719–962,L1064–1237; generic per | not-met | P20-AUD-009 |
| P20-SDK-FACTORY-176 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L97 | {"path":"crates/opi-ai/src/providers/mod.rs","sha256":"a8ca80084bb44e4910c8c72a98fd2ea4fc223e3291b83fe22772621652e6142f","citation":"builtin_provider_profiles/builtin_profile; exac | met |  |
| P20-SDK-FACTORY-177 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L97 | {"path":"crates/opi-ai/src/provider_collection.rs","sha256":"fce9b8186f864933ce0cf54e30a1c9612625845c7e3419ca05ffc7fecf51de32","citation":"L586–696,L719–962,L1064–1237; generic per | met |  |
| P20-SDK-FACTORY-178 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L97 | {"path":"crates/opi-ai/src/provider_collection.rs","sha256":"fce9b8186f864933ce0cf54e30a1c9612625845c7e3419ca05ffc7fecf51de32","citation":"L586–696,L719–962,L1064–1237; generic per | met |  |
| P20-SDK-FACTORY-179 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L97 | {"path":"crates/opi-ai/src/provider_collection.rs","sha256":"fce9b8186f864933ce0cf54e30a1c9612625845c7e3419ca05ffc7fecf51de32","citation":"L586–696,L719–962,L1064–1237; generic per | met |  |
| P20-SDK-FACTORY-180 | docs/research/pi-ai-opi-ai-alignment/acceptance-matrix.md L255 | {"path":"crates/opi-ai/src/provider_collection.rs","sha256":"fce9b8186f864933ce0cf54e30a1c9612625845c7e3419ca05ffc7fecf51de32","citation":"L586–696,L719–962,L1064–1237; generic per | met |  |
| P20-SDK-FACTORY-181 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L98 | {"path":"crates/opi-ai/src/providers/mod.rs","sha256":"a8ca80084bb44e4910c8c72a98fd2ea4fc223e3291b83fe22772621652e6142f","citation":"builtin_provider_profiles/builtin_profile; exac | met |  |
| P20-SDK-FACTORY-182 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L98 | {"path":"crates/opi-ai/src/provider_collection.rs","sha256":"fce9b8186f864933ce0cf54e30a1c9612625845c7e3419ca05ffc7fecf51de32","citation":"L586–696,L719–962,L1064–1237; generic per | met |  |
| P20-SDK-FACTORY-183 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L98 | {"path":"crates/opi-ai/src/provider_collection.rs","sha256":"fce9b8186f864933ce0cf54e30a1c9612625845c7e3419ca05ffc7fecf51de32","citation":"L586–696,L719–962,L1064–1237; generic per | met |  |
| P20-SDK-FACTORY-184 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L98 | {"path":"crates/opi-ai/src/provider_collection.rs","sha256":"fce9b8186f864933ce0cf54e30a1c9612625845c7e3419ca05ffc7fecf51de32","citation":"L586–696,L719–962,L1064–1237; generic per | met |  |
| P20-SDK-FACTORY-185 | docs/research/pi-ai-opi-ai-alignment/acceptance-matrix.md L255 | {"path":"crates/opi-ai/src/provider_collection.rs","sha256":"fce9b8186f864933ce0cf54e30a1c9612625845c7e3419ca05ffc7fecf51de32","citation":"L586–696,L719–962,L1064–1237; generic per | met |  |
| P20-SDK-FACTORY-186 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L99 | {"path":"crates/opi-ai/src/providers/mod.rs","sha256":"a8ca80084bb44e4910c8c72a98fd2ea4fc223e3291b83fe22772621652e6142f","citation":"builtin_provider_profiles/builtin_profile; exac | met |  |
| P20-SDK-FACTORY-187 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L99 | {"path":"crates/opi-ai/src/provider_collection.rs","sha256":"fce9b8186f864933ce0cf54e30a1c9612625845c7e3419ca05ffc7fecf51de32","citation":"L586–696,L719–962,L1064–1237; generic per | met |  |
| P20-SDK-FACTORY-188 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L99 | {"path":"crates/opi-ai/src/provider_collection.rs","sha256":"fce9b8186f864933ce0cf54e30a1c9612625845c7e3419ca05ffc7fecf51de32","citation":"L586–696,L719–962,L1064–1237; generic per | met |  |
| P20-SDK-FACTORY-189 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L99 | {"path":"crates/opi-ai/src/provider_collection.rs","sha256":"fce9b8186f864933ce0cf54e30a1c9612625845c7e3419ca05ffc7fecf51de32","citation":"L586–696,L719–962,L1064–1237; generic per | met |  |
| P20-SDK-FACTORY-190 | docs/research/pi-ai-opi-ai-alignment/acceptance-matrix.md L255 | {"path":"crates/opi-ai/src/provider_collection.rs","sha256":"fce9b8186f864933ce0cf54e30a1c9612625845c7e3419ca05ffc7fecf51de32","citation":"L586–696,L719–962,L1064–1237; generic per | met |  |
| P20-SDK-FACTORY-191 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L100 | {"path":"crates/opi-ai/src/providers/mod.rs","sha256":"a8ca80084bb44e4910c8c72a98fd2ea4fc223e3291b83fe22772621652e6142f","citation":"builtin_provider_profiles/builtin_profile; exac | met |  |
| P20-SDK-FACTORY-192 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L100 | {"path":"crates/opi-ai/src/provider_collection.rs","sha256":"fce9b8186f864933ce0cf54e30a1c9612625845c7e3419ca05ffc7fecf51de32","citation":"L586–696,L719–962,L1064–1237; generic per | met |  |
| P20-SDK-FACTORY-193 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L100 | {"path":"crates/opi-ai/src/provider_collection.rs","sha256":"fce9b8186f864933ce0cf54e30a1c9612625845c7e3419ca05ffc7fecf51de32","citation":"L586–696,L719–962,L1064–1237; generic per | met |  |
| P20-SDK-FACTORY-194 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L100 | {"path":"crates/opi-ai/src/provider_collection.rs","sha256":"fce9b8186f864933ce0cf54e30a1c9612625845c7e3419ca05ffc7fecf51de32","citation":"L586–696,L719–962,L1064–1237; generic per | met |  |
| P20-SDK-FACTORY-195 | docs/research/pi-ai-opi-ai-alignment/acceptance-matrix.md L255 | {"path":"crates/opi-ai/src/provider_collection.rs","sha256":"fce9b8186f864933ce0cf54e30a1c9612625845c7e3419ca05ffc7fecf51de32","citation":"L586–696,L719–962,L1064–1237; generic per | met |  |
| P20-SDK-FACTORY-196 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L101 | {"path":"crates/opi-ai/src/providers/mod.rs","sha256":"a8ca80084bb44e4910c8c72a98fd2ea4fc223e3291b83fe22772621652e6142f","citation":"builtin_provider_profiles/builtin_profile; exac | met |  |
| P20-SDK-FACTORY-197 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L101 | {"path":"crates/opi-ai/src/provider_collection.rs","sha256":"fce9b8186f864933ce0cf54e30a1c9612625845c7e3419ca05ffc7fecf51de32","citation":"L586–696,L719–962,L1064–1237; generic per | met |  |
| P20-SDK-FACTORY-198 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L101 | {"path":"crates/opi-ai/src/provider_collection.rs","sha256":"fce9b8186f864933ce0cf54e30a1c9612625845c7e3419ca05ffc7fecf51de32","citation":"L586–696,L719–962,L1064–1237; generic per | met |  |
| P20-SDK-FACTORY-199 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L101 | {"path":"crates/opi-ai/src/provider_collection.rs","sha256":"fce9b8186f864933ce0cf54e30a1c9612625845c7e3419ca05ffc7fecf51de32","citation":"L586–696,L719–962,L1064–1237; generic per | met |  |
| P20-SDK-FACTORY-200 | docs/research/pi-ai-opi-ai-alignment/acceptance-matrix.md L255 | {"path":"crates/opi-ai/src/provider_collection.rs","sha256":"fce9b8186f864933ce0cf54e30a1c9612625845c7e3419ca05ffc7fecf51de32","citation":"L586–696,L719–962,L1064–1237; generic per | met |  |
| P20-SDK-FACTORY-201 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L102 | {"path":"crates/opi-ai/src/providers/mod.rs","sha256":"a8ca80084bb44e4910c8c72a98fd2ea4fc223e3291b83fe22772621652e6142f","citation":"builtin_provider_profiles/builtin_profile; exac | met |  |
| P20-SDK-FACTORY-202 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L102 | {"path":"crates/opi-ai/src/provider_collection.rs","sha256":"fce9b8186f864933ce0cf54e30a1c9612625845c7e3419ca05ffc7fecf51de32","citation":"L586–696,L719–962,L1064–1237; generic per | met |  |
| P20-SDK-FACTORY-203 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L102 | {"path":"crates/opi-ai/src/provider_collection.rs","sha256":"fce9b8186f864933ce0cf54e30a1c9612625845c7e3419ca05ffc7fecf51de32","citation":"L586–696,L719–962,L1064–1237; generic per | met |  |
| P20-SDK-FACTORY-204 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L102 | {"path":"crates/opi-ai/src/provider_collection.rs","sha256":"fce9b8186f864933ce0cf54e30a1c9612625845c7e3419ca05ffc7fecf51de32","citation":"L586–696,L719–962,L1064–1237; generic per | met |  |
| P20-SDK-FACTORY-205 | docs/research/pi-ai-opi-ai-alignment/acceptance-matrix.md L255 | {"path":"crates/opi-ai/src/provider_collection.rs","sha256":"fce9b8186f864933ce0cf54e30a1c9612625845c7e3419ca05ffc7fecf51de32","citation":"L586–696,L719–962,L1064–1237; generic per | met |  |
| P20-SDK-FACTORY-206 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L103 | {"path":"crates/opi-ai/src/providers/mod.rs","sha256":"a8ca80084bb44e4910c8c72a98fd2ea4fc223e3291b83fe22772621652e6142f","citation":"builtin_provider_profiles/builtin_profile; exac | met |  |
| P20-SDK-FACTORY-207 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L103 | {"path":"crates/opi-ai/src/provider_collection.rs","sha256":"fce9b8186f864933ce0cf54e30a1c9612625845c7e3419ca05ffc7fecf51de32","citation":"L586–696,L719–962,L1064–1237; generic per | met |  |
| P20-SDK-FACTORY-208 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L103 | {"path":"crates/opi-ai/src/provider_collection.rs","sha256":"fce9b8186f864933ce0cf54e30a1c9612625845c7e3419ca05ffc7fecf51de32","citation":"L586–696,L719–962,L1064–1237; generic per | met |  |
| P20-SDK-FACTORY-209 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L103 | {"path":"crates/opi-ai/src/provider_collection.rs","sha256":"fce9b8186f864933ce0cf54e30a1c9612625845c7e3419ca05ffc7fecf51de32","citation":"L586–696,L719–962,L1064–1237; generic per | met |  |
| P20-SDK-FACTORY-210 | docs/research/pi-ai-opi-ai-alignment/acceptance-matrix.md L255 | {"path":"crates/opi-ai/src/provider_collection.rs","sha256":"fce9b8186f864933ce0cf54e30a1c9612625845c7e3419ca05ffc7fecf51de32","citation":"L586–696,L719–962,L1064–1237; generic per | met |  |
| P20-SDK-PROFILE-001 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L58 | {"path":"crates/opi-ai/src/auth/builtin.rs","sha256":"df76187e83211602bb5229d0284263d0c1022decfc38ca926859a52a9b8e297e","citation":"L164–358 selected key/OAuth/header/federation an | met |  |
| P20-SDK-PROFILE-002 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L58 | {"path":"crates/opi-ai/src/auth/builtin.rs","sha256":"df76187e83211602bb5229d0284263d0c1022decfc38ca926859a52a9b8e297e","citation":"L164–358 selected key/OAuth/header/federation an | met |  |
| P20-SDK-PROFILE-003 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L68 | {"path":"crates/opi-ai/src/auth/builtin.rs","sha256":"df76187e83211602bb5229d0284263d0c1022decfc38ca926859a52a9b8e297e","citation":"L164–358 selected key/OAuth/header/federation an | met |  |
| P20-SDK-PROFILE-004 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L72 | {"path":"crates/opi-ai/src/auth/builtin.rs","sha256":"df76187e83211602bb5229d0284263d0c1022decfc38ca926859a52a9b8e297e","citation":"L164–358 selected key/OAuth/header/federation an | met |  |
| P20-SDK-PROFILE-005 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L85 | {"path":"crates/opi-ai/src/auth/builtin.rs","sha256":"df76187e83211602bb5229d0284263d0c1022decfc38ca926859a52a9b8e297e","citation":"L164–358 selected key/OAuth/header/federation an | met |  |
| P20-SDK-PROFILE-006 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L86 | {"path":"crates/opi-ai/src/auth/builtin.rs","sha256":"df76187e83211602bb5229d0284263d0c1022decfc38ca926859a52a9b8e297e","citation":"L164–358 selected key/OAuth/header/federation an | met |  |
| P20-SDK-PROFILE-007 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L87 | {"path":"crates/opi-ai/src/auth/builtin.rs","sha256":"df76187e83211602bb5229d0284263d0c1022decfc38ca926859a52a9b8e297e","citation":"L164–358 selected key/OAuth/header/federation an | met |  |
| P20-SDK-PROFILE-008 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L88 | {"path":"crates/opi-ai/src/auth/builtin.rs","sha256":"df76187e83211602bb5229d0284263d0c1022decfc38ca926859a52a9b8e297e","citation":"L164–358 selected key/OAuth/header/federation an | met |  |
| P20-SDK-PROFILE-009 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L89 | {"path":"crates/opi-ai/src/auth/builtin.rs","sha256":"df76187e83211602bb5229d0284263d0c1022decfc38ca926859a52a9b8e297e","citation":"L164–358 selected key/OAuth/header/federation an | met |  |
| P20-SDK-PROFILE-010 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L93 | {"path":"crates/opi-ai/src/providers/mod.rs","sha256":"a8ca80084bb44e4910c8c72a98fd2ea4fc223e3291b83fe22772621652e6142f","citation":"owning production module; full per-case trace n | not-assessable | P20-AUD-018 |
| P20-SDK-PROFILE-011 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L95 | {"path":"crates/opi-ai/src/auth/builtin.rs","sha256":"df76187e83211602bb5229d0284263d0c1022decfc38ca926859a52a9b8e297e","citation":"L164–358 selected key/OAuth/header/federation an | met |  |
| P20-SDK-PROFILE-012 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L108 | {"path":"crates/opi-ai/src/auth/builtin.rs","sha256":"df76187e83211602bb5229d0284263d0c1022decfc38ca926859a52a9b8e297e","citation":"L164–358 selected key/OAuth/header/federation an | met |  |
| P20-SDK-PROFILE-013 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L108 | {"path":"crates/opi-ai/src/auth/builtin.rs","sha256":"df76187e83211602bb5229d0284263d0c1022decfc38ca926859a52a9b8e297e","citation":"L106–110,L183–225"} | not-met | P20-AUD-001 |
| P20-SDK-PROFILE-014 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L108 | {"path":"crates/opi-ai/src/auth/builtin.rs","sha256":"df76187e83211602bb5229d0284263d0c1022decfc38ca926859a52a9b8e297e","citation":"L164–358 selected key/OAuth/header/federation an | met |  |
| P20-SDK-PROFILE-015 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L108 | {"path":"crates/opi-ai/src/auth/builtin.rs","sha256":"df76187e83211602bb5229d0284263d0c1022decfc38ca926859a52a9b8e297e","citation":"L164–358 selected key/OAuth/header/federation an | met |  |
| P20-SDK-PROFILE-016 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L109 | {"path":"crates/opi-ai/src/auth/builtin.rs","sha256":"df76187e83211602bb5229d0284263d0c1022decfc38ca926859a52a9b8e297e","citation":"L164–358 selected key/OAuth/header/federation an | met |  |
| P20-SDK-PROFILE-017 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L109 | {"path":"crates/opi-ai/src/auth/builtin.rs","sha256":"df76187e83211602bb5229d0284263d0c1022decfc38ca926859a52a9b8e297e","citation":"L164–358 selected key/OAuth/header/federation an | met |  |
| P20-SDK-PROFILE-018 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L109 | {"path":"crates/opi-ai/src/auth/builtin.rs","sha256":"df76187e83211602bb5229d0284263d0c1022decfc38ca926859a52a9b8e297e","citation":"L164–358 selected key/OAuth/header/federation an | met |  |
| P20-SDK-PROFILE-019 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L110 | {"path":"crates/opi-ai/src/auth/builtin.rs","sha256":"df76187e83211602bb5229d0284263d0c1022decfc38ca926859a52a9b8e297e","citation":"L164–358 selected key/OAuth/header/federation an | met |  |
| P20-SDK-PROFILE-020 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L110 | {"path":"crates/opi-ai/src/auth/builtin.rs","sha256":"df76187e83211602bb5229d0284263d0c1022decfc38ca926859a52a9b8e297e","citation":"L164–358 selected key/OAuth/header/federation an | met |  |
| P20-SDK-PROFILE-021 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L110 | {"path":"crates/opi-ai/src/auth/builtin.rs","sha256":"df76187e83211602bb5229d0284263d0c1022decfc38ca926859a52a9b8e297e","citation":"L164–358 selected key/OAuth/header/federation an | met |  |
| P20-SDK-PROFILE-022 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L110 | {"path":"crates/opi-ai/src/auth/builtin.rs","sha256":"df76187e83211602bb5229d0284263d0c1022decfc38ca926859a52a9b8e297e","citation":"L164–358 selected key/OAuth/header/federation an | met |  |
| P20-SDK-PROFILE-023 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L111 | {"path":"crates/opi-ai/src/auth/builtin.rs","sha256":"df76187e83211602bb5229d0284263d0c1022decfc38ca926859a52a9b8e297e","citation":"L164–358 selected key/OAuth/header/federation an | met |  |
| P20-SDK-PROFILE-024 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L111 | {"path":"crates/opi-ai/src/auth/builtin.rs","sha256":"df76187e83211602bb5229d0284263d0c1022decfc38ca926859a52a9b8e297e","citation":"L164–358 selected key/OAuth/header/federation an | met |  |
| P20-SDK-PROFILE-025 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L111 | {"path":"crates/opi-ai/src/auth/builtin.rs","sha256":"df76187e83211602bb5229d0284263d0c1022decfc38ca926859a52a9b8e297e","citation":"L164–358 selected key/OAuth/header/federation an | met |  |
| P20-SDK-PROFILE-026 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L112 | {"path":"crates/opi-ai/src/auth/builtin.rs","sha256":"df76187e83211602bb5229d0284263d0c1022decfc38ca926859a52a9b8e297e","citation":"L164–358 selected key/OAuth/header/federation an | met |  |
| P20-SDK-PROFILE-027 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L112 | {"path":"crates/opi-ai/src/auth/builtin.rs","sha256":"df76187e83211602bb5229d0284263d0c1022decfc38ca926859a52a9b8e297e","citation":"L164–358 selected key/OAuth/header/federation an | met |  |
| P20-SDK-PROFILE-028 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L112 | {"path":"crates/opi-ai/src/auth/builtin.rs","sha256":"df76187e83211602bb5229d0284263d0c1022decfc38ca926859a52a9b8e297e","citation":"L164–358 selected key/OAuth/header/federation an | met |  |
| P20-SDK-PROFILE-029 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L113 | {"path":"crates/opi-ai/src/auth/builtin.rs","sha256":"df76187e83211602bb5229d0284263d0c1022decfc38ca926859a52a9b8e297e","citation":"L164–358 selected key/OAuth/header/federation an | met |  |
| P20-SDK-PROFILE-030 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L113 | {"path":"crates/opi-ai/src/providers/mod.rs","sha256":"a8ca80084bb44e4910c8c72a98fd2ea4fc223e3291b83fe22772621652e6142f","citation":"owning production module; full per-case trace n | not-assessable | P20-AUD-018 |
| P20-SDK-PROFILE-031 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L117 | {"path":"crates/opi-ai/src/auth/builtin.rs","sha256":"df76187e83211602bb5229d0284263d0c1022decfc38ca926859a52a9b8e297e","citation":"L164–358 selected key/OAuth/header/federation an | met |  |
| P20-SDK-PROFILE-032 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L117 | {"path":"crates/opi-ai/src/auth/builtin.rs","sha256":"df76187e83211602bb5229d0284263d0c1022decfc38ca926859a52a9b8e297e","citation":"L164–358 selected key/OAuth/header/federation an | met |  |
| P20-SDK-PROFILE-033 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L117 | {"path":"crates/opi-ai/src/providers/mod.rs","sha256":"a8ca80084bb44e4910c8c72a98fd2ea4fc223e3291b83fe22772621652e6142f","citation":"owning production module; full per-case trace n | not-assessable | P20-AUD-018 |
| P20-SDK-PROFILE-034 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L242 | {"path":"crates/opi-ai/src/auth/builtin.rs","sha256":"df76187e83211602bb5229d0284263d0c1022decfc38ca926859a52a9b8e297e","citation":"L164–358 selected key/OAuth/header/federation an | met |  |
| P20-SDK-AUTH-001 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L135 | {"path":"crates/opi-ai/src/auth.rs","sha256":"bff7ff1a5e2575e62119861762393b8752be99898a45510d6685aed1ed462019","citation":"L551–608 actual expiry/default skew;L890 onward injected | met |  |
| P20-SDK-AUTH-002 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L135 | {"path":"crates/opi-ai/src/auth.rs","sha256":"bff7ff1a5e2575e62119861762393b8752be99898a45510d6685aed1ed462019","citation":"L551–608 actual expiry/default skew;L890 onward injected | met |  |
| P20-SDK-AUTH-003 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L135 | {"path":"crates/opi-ai/src/auth.rs","sha256":"bff7ff1a5e2575e62119861762393b8752be99898a45510d6685aed1ed462019","citation":"L551–608 actual expiry/default skew;L890 onward injected | met |  |
| P20-SDK-AUTH-004 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L135 | {"path":"crates/opi-ai/src/auth.rs","sha256":"bff7ff1a5e2575e62119861762393b8752be99898a45510d6685aed1ed462019","citation":"L551–608 actual expiry/default skew;L890 onward injected | met |  |
| P20-SDK-AUTH-005 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L151 | {"path":"crates/opi-ai/src/auth.rs","sha256":"bff7ff1a5e2575e62119861762393b8752be99898a45510d6685aed1ed462019","citation":"L551–608 actual expiry/default skew;L890 onward injected | met |  |
| P20-SDK-AUTH-006 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L151 | {"path":"crates/opi-ai/src/models.rs","sha256":"f9fc8b51c90282772b6b8812079772ad05c264c0b7d80efbd51314a8b46e914b","citation":"owning production module; full per-case trace not asse | not-assessable | P20-AUD-018 |
| P20-SDK-AUTH-007 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L151 | {"path":"crates/opi-ai/src/models.rs","sha256":"f9fc8b51c90282772b6b8812079772ad05c264c0b7d80efbd51314a8b46e914b","citation":"owning production module; full per-case trace not asse | not-met | P20-AUD-020 |
| P20-SDK-AUTH-008 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L152 | {"path":"crates/opi-ai/src/auth.rs","sha256":"bff7ff1a5e2575e62119861762393b8752be99898a45510d6685aed1ed462019","citation":"L551–608 actual expiry/default skew;L890 onward injected | met |  |
| P20-SDK-AUTH-009 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L152 | {"path":"crates/opi-ai/src/auth.rs","sha256":"bff7ff1a5e2575e62119861762393b8752be99898a45510d6685aed1ed462019","citation":"L551–608 actual expiry/default skew;L890 onward injected | met |  |
| P20-SDK-AUTH-010 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L152 | {"path":"crates/opi-ai/src/auth.rs","sha256":"bff7ff1a5e2575e62119861762393b8752be99898a45510d6685aed1ed462019","citation":"L551–608 actual expiry/default skew;L890 onward injected | met |  |
| P20-SDK-AUTH-011 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L152 | {"path":"crates/opi-ai/src/auth.rs","sha256":"bff7ff1a5e2575e62119861762393b8752be99898a45510d6685aed1ed462019","citation":"L551–608 actual expiry/default skew;L890 onward injected | met |  |
| P20-SDK-AUTH-012 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L152 | {"path":"crates/opi-ai/src/auth.rs","sha256":"bff7ff1a5e2575e62119861762393b8752be99898a45510d6685aed1ed462019","citation":"L551–608 actual expiry/default skew;L890 onward injected | met |  |
| P20-SDK-AUTH-013 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L152 | {"path":"crates/opi-ai/src/auth.rs","sha256":"bff7ff1a5e2575e62119861762393b8752be99898a45510d6685aed1ed462019","citation":"L551–608 actual expiry/default skew;L890 onward injected | met |  |
| P20-SDK-AUTH-014 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L152 | {"path":"crates/opi-ai/src/auth.rs","sha256":"bff7ff1a5e2575e62119861762393b8752be99898a45510d6685aed1ed462019","citation":"L551–608 actual expiry/default skew;L890 onward injected | met |  |
| P20-SDK-AUTH-015 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L153 | {"path":"crates/opi-ai/src/auth.rs","sha256":"bff7ff1a5e2575e62119861762393b8752be99898a45510d6685aed1ed462019","citation":"L551–608 actual expiry/default skew;L890 onward injected | met |  |
| P20-SDK-AUTH-016 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L153 | {"path":"crates/opi-ai/src/auth.rs","sha256":"bff7ff1a5e2575e62119861762393b8752be99898a45510d6685aed1ed462019","citation":"L551–608 actual expiry/default skew;L890 onward injected | met |  |
| P20-SDK-AUTH-017 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L153 | {"path":"crates/opi-ai/src/auth.rs","sha256":"bff7ff1a5e2575e62119861762393b8752be99898a45510d6685aed1ed462019","citation":"L551–608 actual expiry/default skew;L890 onward injected | met |  |
| P20-SDK-AUTH-018 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L153 | {"path":"crates/opi-ai/src/models.rs","sha256":"f9fc8b51c90282772b6b8812079772ad05c264c0b7d80efbd51314a8b46e914b","citation":"owning production module; full per-case trace not asse | not-assessable | P20-AUD-018 |
| P20-SDK-AUTH-019 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L208 | {"path":"crates/opi-ai/src/auth.rs","sha256":"bff7ff1a5e2575e62119861762393b8752be99898a45510d6685aed1ed462019","citation":"L551–608 actual expiry/default skew;L890 onward injected | met |  |
| P20-SDK-AUTH-020 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L208 | {"path":"crates/opi-ai/src/auth.rs","sha256":"bff7ff1a5e2575e62119861762393b8752be99898a45510d6685aed1ed462019","citation":"L551–608 actual expiry/default skew;L890 onward injected | met |  |
| P20-SDK-AUTH-021 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-rust-differences.md L135 | {"path":"crates/opi-ai/src/auth.rs","sha256":"bff7ff1a5e2575e62119861762393b8752be99898a45510d6685aed1ed462019","citation":"L551–608 actual expiry/default skew;L890 onward injected | met |  |
| P20-SDK-AUTH-022 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-rust-differences.md L135 | {"path":"crates/opi-ai/src/auth.rs","sha256":"bff7ff1a5e2575e62119861762393b8752be99898a45510d6685aed1ed462019","citation":"L551–608 actual expiry/default skew;L890 onward injected | met |  |
| P20-SDK-AUTH-023 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-rust-differences.md L144 | {"path":"crates/opi-ai/src/auth.rs","sha256":"bff7ff1a5e2575e62119861762393b8752be99898a45510d6685aed1ed462019","citation":"L551–608 actual expiry/default skew;L890 onward injected | met |  |
| P20-SDK-AUTH-024 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-rust-differences.md L144 | {"path":"crates/opi-ai/src/auth.rs","sha256":"bff7ff1a5e2575e62119861762393b8752be99898a45510d6685aed1ed462019","citation":"L551–608 actual expiry/default skew;L890 onward injected | met |  |
| P20-SDK-AUTH-025 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-rust-differences.md L144 | {"path":"crates/opi-ai/src/auth.rs","sha256":"bff7ff1a5e2575e62119861762393b8752be99898a45510d6685aed1ed462019","citation":"L551–608 actual expiry/default skew;L890 onward injected | met |  |
| P20-SDK-AUTH-026 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-rust-differences.md L144 | {"path":"crates/opi-ai/src/models.rs","sha256":"f9fc8b51c90282772b6b8812079772ad05c264c0b7d80efbd51314a8b46e914b","citation":"owning production module; full per-case trace not asse | not-assessable | P20-AUD-018 |
| P20-SDK-AUTH-027 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-rust-differences.md L145 | {"path":"crates/opi-ai/src/models.rs","sha256":"f9fc8b51c90282772b6b8812079772ad05c264c0b7d80efbd51314a8b46e914b","citation":"owning production module; full per-case trace not asse | not-assessable | P20-AUD-018 |
| P20-SDK-AUTH-028 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-rust-differences.md L156 | {"path":"crates/opi-ai/src/auth.rs","sha256":"bff7ff1a5e2575e62119861762393b8752be99898a45510d6685aed1ed462019","citation":"L551–608 actual expiry/default skew;L890 onward injected | met |  |
| P20-SDK-OAUTH-ANTHROPIC-001 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L139 | {"path":"crates/opi-ai/src/auth/oauth/anthropic.rs","sha256":"2dbe09b3724d18d5be8f9a5a7bcc31e4ccb09ca03cf7950f394386f4792b144e","citation":"OAuthProvider::login,refresh,to_auth; pr | met |  |
| P20-SDK-OAUTH-ANTHROPIC-002 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L139 | {"path":"crates/opi-ai/src/auth/oauth/anthropic.rs","sha256":"2dbe09b3724d18d5be8f9a5a7bcc31e4ccb09ca03cf7950f394386f4792b144e","citation":"OAuthProvider::login,refresh,to_auth; pr | met |  |
| P20-SDK-OAUTH-ANTHROPIC-003 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L139 | {"path":"crates/opi-ai/src/auth/oauth/anthropic.rs","sha256":"2dbe09b3724d18d5be8f9a5a7bcc31e4ccb09ca03cf7950f394386f4792b144e","citation":"OAuthProvider::login,refresh,to_auth; pr | met |  |
| P20-SDK-OAUTH-ANTHROPIC-004 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L139 | {"path":"crates/opi-ai/src/auth/oauth/anthropic.rs","sha256":"2dbe09b3724d18d5be8f9a5a7bcc31e4ccb09ca03cf7950f394386f4792b144e","citation":"OAuthProvider::login,refresh,to_auth; pr | met |  |
| P20-SDK-OAUTH-ANTHROPIC-005 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L139 | {"path":"crates/opi-ai/src/auth/oauth/anthropic.rs","sha256":"2dbe09b3724d18d5be8f9a5a7bcc31e4ccb09ca03cf7950f394386f4792b144e","citation":"OAuthProvider::login,refresh,to_auth; pr | met |  |
| P20-SDK-OAUTH-ANTHROPIC-006 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L139 | {"path":"crates/opi-ai/src/auth/oauth/anthropic.rs","sha256":"2dbe09b3724d18d5be8f9a5a7bcc31e4ccb09ca03cf7950f394386f4792b144e","citation":"OAuthProvider::login,refresh,to_auth; pr | met |  |
| P20-SDK-OAUTH-ANTHROPIC-007 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L139 | {"path":"crates/opi-ai/src/auth/oauth/anthropic.rs","sha256":"2dbe09b3724d18d5be8f9a5a7bcc31e4ccb09ca03cf7950f394386f4792b144e","citation":"OAuthProvider::login,refresh,to_auth; pr | met |  |
| P20-SDK-OAUTH-CODEX-001 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L140 | {"path":"crates/opi-ai/src/auth/oauth/codex.rs","sha256":"052aef0d1eac94e3d4d217477c8d82bb913e0e4b5668d2bfa0dc865ce5e0d264","citation":"OAuthProvider::login,refresh,to_auth; produc | met |  |
| P20-SDK-OAUTH-CODEX-002 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L140 | {"path":"crates/opi-ai/src/auth/oauth/codex.rs","sha256":"052aef0d1eac94e3d4d217477c8d82bb913e0e4b5668d2bfa0dc865ce5e0d264","citation":"OAuthProvider::login,refresh,to_auth; produc | met |  |
| P20-SDK-OAUTH-CODEX-003 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L140 | {"path":"crates/opi-ai/src/auth/oauth/codex.rs","sha256":"052aef0d1eac94e3d4d217477c8d82bb913e0e4b5668d2bfa0dc865ce5e0d264","citation":"OAuthProvider::login,refresh,to_auth; produc | met |  |
| P20-SDK-OAUTH-CODEX-004 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L140 | {"path":"crates/opi-ai/src/auth/oauth/codex.rs","sha256":"052aef0d1eac94e3d4d217477c8d82bb913e0e4b5668d2bfa0dc865ce5e0d264","citation":"OAuthProvider::login,refresh,to_auth; produc | met |  |
| P20-SDK-OAUTH-CODEX-005 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L140 | {"path":"crates/opi-ai/src/auth/oauth/codex.rs","sha256":"052aef0d1eac94e3d4d217477c8d82bb913e0e4b5668d2bfa0dc865ce5e0d264","citation":"OAuthProvider::login,refresh,to_auth; produc | met |  |
| P20-SDK-OAUTH-CODEX-006 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L140 | {"path":"crates/opi-ai/src/auth/oauth/codex.rs","sha256":"052aef0d1eac94e3d4d217477c8d82bb913e0e4b5668d2bfa0dc865ce5e0d264","citation":"OAuthProvider::login,refresh,to_auth; produc | met |  |
| P20-SDK-OAUTH-CODEX-007 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L140 | {"path":"crates/opi-ai/src/auth/oauth/codex.rs","sha256":"052aef0d1eac94e3d4d217477c8d82bb913e0e4b5668d2bfa0dc865ce5e0d264","citation":"OAuthProvider::login,refresh,to_auth; produc | met |  |
| P20-SDK-OAUTH-CODEX-008 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L140 | {"path":"crates/opi-ai/src/auth/oauth/codex.rs","sha256":"052aef0d1eac94e3d4d217477c8d82bb913e0e4b5668d2bfa0dc865ce5e0d264","citation":"OAuthProvider::login,refresh,to_auth; produc | met |  |
| P20-SDK-OAUTH-CHATGPT-001 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L141 | {"path":"crates/opi-ai/src/auth/oauth/chatgpt.rs","sha256":"5de88bcdafd302e6fb4bbd74ad11668a5fdeb959ccc9ddb15846ab6ac6b659fd","citation":"OAuthProvider::login,refresh,to_auth; prod | met |  |
| P20-SDK-OAUTH-CHATGPT-002 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L141 | {"path":"crates/opi-ai/src/auth/oauth/chatgpt.rs","sha256":"5de88bcdafd302e6fb4bbd74ad11668a5fdeb959ccc9ddb15846ab6ac6b659fd","citation":"OAuthProvider::login,refresh,to_auth; prod | met |  |
| P20-SDK-OAUTH-CHATGPT-003 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L141 | {"path":"crates/opi-ai/src/auth/oauth/chatgpt.rs","sha256":"5de88bcdafd302e6fb4bbd74ad11668a5fdeb959ccc9ddb15846ab6ac6b659fd","citation":"OAuthProvider::login,refresh,to_auth; prod | met |  |
| P20-SDK-OAUTH-CHATGPT-004 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L141 | {"path":"crates/opi-ai/src/auth/oauth/chatgpt.rs","sha256":"5de88bcdafd302e6fb4bbd74ad11668a5fdeb959ccc9ddb15846ab6ac6b659fd","citation":"OAuthProvider::login,refresh,to_auth; prod | met |  |
| P20-SDK-OAUTH-CHATGPT-005 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L141 | {"path":"crates/opi-ai/src/auth/oauth/chatgpt.rs","sha256":"5de88bcdafd302e6fb4bbd74ad11668a5fdeb959ccc9ddb15846ab6ac6b659fd","citation":"OAuthProvider::login,refresh,to_auth; prod | met |  |
| P20-SDK-OAUTH-CHATGPT-006 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L141 | {"path":"crates/opi-ai/src/auth/oauth/chatgpt.rs","sha256":"5de88bcdafd302e6fb4bbd74ad11668a5fdeb959ccc9ddb15846ab6ac6b659fd","citation":"OAuthProvider::login,refresh,to_auth; prod | met |  |
| P20-SDK-OAUTH-CHATGPT-007 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L141 | {"path":"crates/opi-ai/src/auth/oauth/chatgpt.rs","sha256":"5de88bcdafd302e6fb4bbd74ad11668a5fdeb959ccc9ddb15846ab6ac6b659fd","citation":"OAuthProvider::login,refresh,to_auth; prod | met |  |
| P20-SDK-OAUTH-CHATGPT-008 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L141 | {"path":"crates/opi-ai/src/auth/oauth/chatgpt.rs","sha256":"5de88bcdafd302e6fb4bbd74ad11668a5fdeb959ccc9ddb15846ab6ac6b659fd","citation":"OAuthProvider::login,refresh,to_auth; prod | met |  |
| P20-SDK-OAUTH-COPILOT-001 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L142 | {"path":"crates/opi-ai/src/auth/oauth/copilot.rs","sha256":"fd3eb40f8b7862e1aca919e98604cca458dd00bca592a5635f2c2b38135a70df","citation":"OAuthProvider::login,refresh,to_auth; prod | met |  |
| P20-SDK-OAUTH-COPILOT-002 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L142 | {"path":"crates/opi-ai/src/auth/oauth/copilot.rs","sha256":"fd3eb40f8b7862e1aca919e98604cca458dd00bca592a5635f2c2b38135a70df","citation":"OAuthProvider::login,refresh,to_auth; prod | met |  |
| P20-SDK-OAUTH-COPILOT-003 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L142 | {"path":"crates/opi-ai/src/auth/oauth/copilot.rs","sha256":"fd3eb40f8b7862e1aca919e98604cca458dd00bca592a5635f2c2b38135a70df","citation":"OAuthProvider::login,refresh,to_auth; prod | met |  |
| P20-SDK-OAUTH-COPILOT-004 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L142 | {"path":"crates/opi-ai/src/auth/oauth/copilot.rs","sha256":"fd3eb40f8b7862e1aca919e98604cca458dd00bca592a5635f2c2b38135a70df","citation":"OAuthProvider::login,refresh,to_auth; prod | met |  |
| P20-SDK-OAUTH-COPILOT-005 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L142 | {"path":"crates/opi-ai/src/auth/oauth/copilot.rs","sha256":"fd3eb40f8b7862e1aca919e98604cca458dd00bca592a5635f2c2b38135a70df","citation":"OAuthProvider::login,refresh,to_auth; prod | met |  |
| P20-SDK-OAUTH-COPILOT-006 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L142 | {"path":"crates/opi-ai/src/auth/oauth/copilot.rs","sha256":"fd3eb40f8b7862e1aca919e98604cca458dd00bca592a5635f2c2b38135a70df","citation":"OAuthProvider::login,refresh,to_auth; prod | met |  |
| P20-SDK-OAUTH-OPENROUTER-001 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L143 | {"path":"crates/opi-ai/src/auth/oauth/openrouter.rs","sha256":"16a26a03a049802d9598c02db3eac796d709186063512dbd3efb5207e7189b51","citation":"OAuthProvider::login,refresh,to_auth; p | met |  |
| P20-SDK-OAUTH-OPENROUTER-002 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L143 | {"path":"crates/opi-ai/src/auth/oauth/openrouter.rs","sha256":"16a26a03a049802d9598c02db3eac796d709186063512dbd3efb5207e7189b51","citation":"OAuthProvider::login,refresh,to_auth; p | met |  |
| P20-SDK-OAUTH-OPENROUTER-003 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L143 | {"path":"crates/opi-ai/src/auth/oauth/openrouter.rs","sha256":"16a26a03a049802d9598c02db3eac796d709186063512dbd3efb5207e7189b51","citation":"OAuthProvider::login,refresh,to_auth; p | met |  |
| P20-SDK-OAUTH-OPENROUTER-004 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L143 | {"path":"crates/opi-ai/src/auth/oauth/openrouter.rs","sha256":"16a26a03a049802d9598c02db3eac796d709186063512dbd3efb5207e7189b51","citation":"owning production module; full per-case | not-met | P20-AUD-020 |
| P20-SDK-OAUTH-OPENROUTER-005 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L143 | {"path":"crates/opi-ai/src/auth/oauth/openrouter.rs","sha256":"16a26a03a049802d9598c02db3eac796d709186063512dbd3efb5207e7189b51","citation":"OAuthProvider::login,refresh,to_auth; p | met |  |
| P20-SDK-OAUTH-OPENROUTER-006 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L143 | {"path":"crates/opi-ai/src/auth/oauth/openrouter.rs","sha256":"16a26a03a049802d9598c02db3eac796d709186063512dbd3efb5207e7189b51","citation":"OAuthProvider::login,refresh,to_auth; p | met |  |
| P20-SDK-OAUTH-KIMI-001 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L144 | {"path":"crates/opi-ai/src/auth/oauth/kimi_coding.rs","sha256":"1e81bbbb2371351832f82673670704b1e3bcb6f14f379ba855ef9d2a91cd1e3b","citation":"OAuthProvider::login,refresh,to_auth;  | met |  |
| P20-SDK-OAUTH-KIMI-002 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L144 | {"path":"crates/opi-ai/src/auth/oauth/kimi_coding.rs","sha256":"1e81bbbb2371351832f82673670704b1e3bcb6f14f379ba855ef9d2a91cd1e3b","citation":"OAuthProvider::login,refresh,to_auth;  | met |  |
| P20-SDK-OAUTH-KIMI-003 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L144 | {"path":"crates/opi-ai/src/auth/oauth/kimi_coding.rs","sha256":"1e81bbbb2371351832f82673670704b1e3bcb6f14f379ba855ef9d2a91cd1e3b","citation":"OAuthProvider::login,refresh,to_auth;  | met |  |
| P20-SDK-OAUTH-KIMI-004 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L144 | {"path":"crates/opi-ai/src/auth/oauth/kimi_coding.rs","sha256":"1e81bbbb2371351832f82673670704b1e3bcb6f14f379ba855ef9d2a91cd1e3b","citation":"OAuthProvider::login,refresh,to_auth;  | met |  |
| P20-SDK-OAUTH-KIMI-005 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L144 | {"path":"crates/opi-ai/src/auth/oauth/kimi_coding.rs","sha256":"1e81bbbb2371351832f82673670704b1e3bcb6f14f379ba855ef9d2a91cd1e3b","citation":"OAuthProvider::login,refresh,to_auth;  | met |  |
| P20-SDK-OAUTH-KIMI-006 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L144 | {"path":"crates/opi-ai/src/auth/oauth/kimi_coding.rs","sha256":"1e81bbbb2371351832f82673670704b1e3bcb6f14f379ba855ef9d2a91cd1e3b","citation":"OAuthProvider::login,refresh,to_auth;  | met |  |
| P20-SDK-OAUTH-META-001 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L145 | {"path":"crates/opi-ai/src/auth/oauth/meta.rs","sha256":"fb36d3e5177f4f792f5c1e2c4451d5d2d4f85624fc720cb27c1d6fa3941996ce","citation":"OAuthProvider::login,refresh,to_auth; product | met |  |
| P20-SDK-OAUTH-META-002 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L145 | {"path":"crates/opi-ai/src/auth/oauth/meta.rs","sha256":"fb36d3e5177f4f792f5c1e2c4451d5d2d4f85624fc720cb27c1d6fa3941996ce","citation":"OAuthProvider::login,refresh,to_auth; product | met |  |
| P20-SDK-OAUTH-META-003 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L145 | {"path":"crates/opi-ai/src/auth/oauth/meta.rs","sha256":"fb36d3e5177f4f792f5c1e2c4451d5d2d4f85624fc720cb27c1d6fa3941996ce","citation":"OAuthProvider::login,refresh,to_auth; product | met |  |
| P20-SDK-OAUTH-META-004 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L145 | {"path":"crates/opi-ai/src/auth/oauth/meta.rs","sha256":"fb36d3e5177f4f792f5c1e2c4451d5d2d4f85624fc720cb27c1d6fa3941996ce","citation":"OAuthProvider::login,refresh,to_auth; product | met |  |
| P20-SDK-OAUTH-META-005 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L145 | {"path":"crates/opi-ai/src/auth/oauth/meta.rs","sha256":"fb36d3e5177f4f792f5c1e2c4451d5d2d4f85624fc720cb27c1d6fa3941996ce","citation":"OAuthProvider::login,refresh,to_auth; product | met |  |
| P20-SDK-OAUTH-META-006 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L145 | {"path":"crates/opi-ai/src/auth/oauth/meta.rs","sha256":"fb36d3e5177f4f792f5c1e2c4451d5d2d4f85624fc720cb27c1d6fa3941996ce","citation":"OAuthProvider::login,refresh,to_auth; product | met |  |
| P20-SDK-OAUTH-XAI-001 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L146 | {"path":"crates/opi-ai/src/auth/oauth/xai.rs","sha256":"7ba3bf84cd628e276f1b133240104ec05a4d27d5401360d4672e13782903e9cb","citation":"OAuthProvider::login,refresh,to_auth; producti | met |  |
| P20-SDK-OAUTH-XAI-002 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L146 | {"path":"crates/opi-ai/src/auth/oauth/xai.rs","sha256":"7ba3bf84cd628e276f1b133240104ec05a4d27d5401360d4672e13782903e9cb","citation":"OAuthProvider::login,refresh,to_auth; producti | met |  |
| P20-SDK-OAUTH-XAI-003 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L146 | {"path":"crates/opi-ai/src/auth/oauth/xai.rs","sha256":"7ba3bf84cd628e276f1b133240104ec05a4d27d5401360d4672e13782903e9cb","citation":"OAuthProvider::login,refresh,to_auth; producti | met |  |
| P20-SDK-OAUTH-XAI-004 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L146 | {"path":"crates/opi-ai/src/auth/oauth/xai.rs","sha256":"7ba3bf84cd628e276f1b133240104ec05a4d27d5401360d4672e13782903e9cb","citation":"OAuthProvider::login,refresh,to_auth; producti | met |  |
| P20-SDK-OAUTH-XAI-005 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L146 | {"path":"crates/opi-ai/src/auth/oauth/xai.rs","sha256":"7ba3bf84cd628e276f1b133240104ec05a4d27d5401360d4672e13782903e9cb","citation":"OAuthProvider::login,refresh,to_auth; producti | met |  |
| P20-SDK-OAUTH-RADIUS-001 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L147 | {"path":"crates/opi-ai/src/auth/oauth/radius.rs","sha256":"07f483d756e9ed2b00f09c072f992eecbae52cffc98a310314a8e46d300a646a","citation":"OAuthProvider::login,refresh,to_auth; produ | met |  |
| P20-SDK-OAUTH-RADIUS-002 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L147 | {"path":"crates/opi-ai/src/auth/oauth/radius.rs","sha256":"07f483d756e9ed2b00f09c072f992eecbae52cffc98a310314a8e46d300a646a","citation":"OAuthProvider::login,refresh,to_auth; produ | met |  |
| P20-SDK-OAUTH-RADIUS-003 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L147 | {"path":"crates/opi-ai/src/auth/oauth/radius.rs","sha256":"07f483d756e9ed2b00f09c072f992eecbae52cffc98a310314a8e46d300a646a","citation":"OAuthProvider::login,refresh,to_auth; produ | met |  |
| P20-SDK-OAUTH-RADIUS-004 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L147 | {"path":"crates/opi-ai/src/auth/oauth/radius.rs","sha256":"07f483d756e9ed2b00f09c072f992eecbae52cffc98a310314a8e46d300a646a","citation":"OAuthProvider::login,refresh,to_auth; produ | met |  |
| P20-SDK-OAUTH-RADIUS-005 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L147 | {"path":"crates/opi-ai/src/auth/oauth/radius.rs","sha256":"07f483d756e9ed2b00f09c072f992eecbae52cffc98a310314a8e46d300a646a","citation":"OAuthProvider::login,refresh,to_auth; produ | met |  |
| P20-SDK-OAUTH-RADIUS-006 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L147 | {"path":"crates/opi-ai/src/auth/oauth/radius.rs","sha256":"07f483d756e9ed2b00f09c072f992eecbae52cffc98a310314a8e46d300a646a","citation":"OAuthProvider::login,refresh,to_auth; produ | met |  |
| P20-SDK-OAUTH-RADIUS-007 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L147 | {"path":"crates/opi-ai/src/auth/oauth/radius.rs","sha256":"07f483d756e9ed2b00f09c072f992eecbae52cffc98a310314a8e46d300a646a","citation":"OAuthProvider::login,refresh,to_auth; produ | met |  |
| P20-SDK-IMAGE-001 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L123 | {"path":"crates/opi-ai/src/api/openrouter_images.rs","sha256":"f65ea5abbd5b17673de4784652afea86a2921183f7eb14ccb80cbc5bac665aea","citation":"L160–186,L229–462,L595–719"} | met |  |
| P20-SDK-IMAGE-002 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L123 | {"path":"crates/opi-ai/src/images.rs","sha256":"1b17938fe0f3c5f337155ca689d03e8ff65bca2f9e1ec0937b35a37f951e3bb9","citation":"owning production module; full per-case trace not asse | not-met | P20-AUD-013 |
| P20-SDK-IMAGE-003 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L123 | {"path":"crates/opi-ai/src/api/openrouter_images.rs","sha256":"f65ea5abbd5b17673de4784652afea86a2921183f7eb14ccb80cbc5bac665aea","citation":"L160–186,L229–462,L595–719"} | met |  |
| P20-SDK-IMAGE-004 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L123 | {"path":"crates/opi-ai/src/api/openrouter_images.rs","sha256":"f65ea5abbd5b17673de4784652afea86a2921183f7eb14ccb80cbc5bac665aea","citation":"L160–186,L229–462,L595–719"} | met |  |
| P20-SDK-IMAGE-005 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L123 | {"path":"crates/opi-ai/src/api/openrouter_images.rs","sha256":"f65ea5abbd5b17673de4784652afea86a2921183f7eb14ccb80cbc5bac665aea","citation":"L160–186,L229–462,L595–719"} | met |  |
| P20-SDK-IMAGE-006 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L123 | {"path":"crates/opi-ai/src/api/openrouter_images.rs","sha256":"f65ea5abbd5b17673de4784652afea86a2921183f7eb14ccb80cbc5bac665aea","citation":"L160–186,L229–462,L595–719"} | met |  |
| P20-SDK-IMAGE-007 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L123 | {"path":"crates/opi-ai/src/images.rs","sha256":"1b17938fe0f3c5f337155ca689d03e8ff65bca2f9e1ec0937b35a37f951e3bb9","citation":"owning production module; full per-case trace not asse | not-met | P20-AUD-014 |
| P20-SDK-IMAGE-008 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L123 | {"path":"crates/opi-ai/src/api/openrouter_images.rs","sha256":"f65ea5abbd5b17673de4784652afea86a2921183f7eb14ccb80cbc5bac665aea","citation":"L160–186,L229–462,L595–719"} | met |  |
| P20-SDK-IMAGE-009 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L123 | {"path":"crates/opi-ai/src/api/openrouter_images.rs","sha256":"f65ea5abbd5b17673de4784652afea86a2921183f7eb14ccb80cbc5bac665aea","citation":"L160–186,L229–462,L595–719"} | met |  |
| P20-SDK-IMAGE-010 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L123 | {"path":"crates/opi-ai/src/api/openrouter_images.rs","sha256":"f65ea5abbd5b17673de4784652afea86a2921183f7eb14ccb80cbc5bac665aea","citation":"L160–186,L229–462,L595–719"} | met |  |
| P20-SDK-CLASSIFIER-001 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L124 | {"path":"crates/opi-ai/src/classifier.rs","sha256":"777f5d93091940d9304588fd481dacbf43c5d6d0c1fe802aee446ed88c7bcec3","citation":"L43–92,L202–264"} | met |  |
| P20-SDK-CLASSIFIER-002 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L124 | {"path":"crates/opi-ai/src/classifier.rs","sha256":"777f5d93091940d9304588fd481dacbf43c5d6d0c1fe802aee446ed88c7bcec3","citation":"L43–92,L202–264"} | met |  |
| P20-SDK-CLASSIFIER-003 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L124 | {"path":"crates/opi-ai/src/classifier.rs","sha256":"777f5d93091940d9304588fd481dacbf43c5d6d0c1fe802aee446ed88c7bcec3","citation":"L43–92,L202–264"} | met |  |
| P20-SDK-CLASSIFIER-004 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L124 | {"path":"crates/opi-ai/src/classifier.rs","sha256":"777f5d93091940d9304588fd481dacbf43c5d6d0c1fe802aee446ed88c7bcec3","citation":"L43–92,L202–264"} | met |  |
| P20-SDK-CLASSIFIER-005 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L124 | {"path":"crates/opi-ai/src/classifier.rs","sha256":"777f5d93091940d9304588fd481dacbf43c5d6d0c1fe802aee446ed88c7bcec3","citation":"L43–92,L202–264"} | met |  |
| P20-SDK-CLASSIFIER-006 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L124 | {"path":"crates/opi-ai/src/classifier.rs","sha256":"777f5d93091940d9304588fd481dacbf43c5d6d0c1fe802aee446ed88c7bcec3","citation":"L43–92,L202–264"} | met |  |
| P20-SDK-CLASSIFIER-007 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L125 | {"path":"crates/opi-ai/src/classifier.rs","sha256":"777f5d93091940d9304588fd481dacbf43c5d6d0c1fe802aee446ed88c7bcec3","citation":"L43–92,L202–264"} | met |  |
| P20-SDK-CLASSIFIER-008 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L125 | {"path":"crates/opi-ai/src/classifier.rs","sha256":"777f5d93091940d9304588fd481dacbf43c5d6d0c1fe802aee446ed88c7bcec3","citation":"L43–92,L202–264"} | met |  |
| P20-SDK-CLASSIFIER-009 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L125 | {"path":"crates/opi-ai/src/classifier.rs","sha256":"777f5d93091940d9304588fd481dacbf43c5d6d0c1fe802aee446ed88c7bcec3","citation":"L43–92,L202–264"} | met |  |
| P20-SDK-CLASSIFIER-010 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L126 | {"path":"crates/opi-ai/src/classifier.rs","sha256":"777f5d93091940d9304588fd481dacbf43c5d6d0c1fe802aee446ed88c7bcec3","citation":"L43–92,L202–264"} | met |  |
| P20-SDK-CLASSIFIER-011 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L126 | {"path":"crates/opi-ai/src/classifier.rs","sha256":"777f5d93091940d9304588fd481dacbf43c5d6d0c1fe802aee446ed88c7bcec3","citation":"L43–92,L202–264"} | met |  |
| P20-SDK-CLASSIFIER-012 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L126 | {"path":"crates/opi-ai/src/classifier.rs","sha256":"777f5d93091940d9304588fd481dacbf43c5d6d0c1fe802aee446ed88c7bcec3","citation":"L43–92,L202–264"} | met |  |
| P20-SDK-CLASSIFIER-013 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L126 | {"path":"crates/opi-ai/src/classifier.rs","sha256":"777f5d93091940d9304588fd481dacbf43c5d6d0c1fe802aee446ed88c7bcec3","citation":"L43–92,L202–264"} | met |  |
| P20-SDK-CLASSIFIER-014 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L126 | {"path":"crates/opi-ai/src/classifier.rs","sha256":"777f5d93091940d9304588fd481dacbf43c5d6d0c1fe802aee446ed88c7bcec3","citation":"L43–92,L202–264"} | met |  |
| P20-SDK-CLASSIFIER-015 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L126 | {"path":"crates/opi-ai/src/classifier.rs","sha256":"777f5d93091940d9304588fd481dacbf43c5d6d0c1fe802aee446ed88c7bcec3","citation":"L43–92,L202–264"} | met |  |
| P20-SDK-CLASSIFIER-016 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L127 | {"path":"crates/opi-ai/src/classifier.rs","sha256":"777f5d93091940d9304588fd481dacbf43c5d6d0c1fe802aee446ed88c7bcec3","citation":"L43–92,L202–264"} | met |  |
| P20-SDK-CLASSIFIER-017 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L127 | {"path":"crates/opi-ai/src/classifier.rs","sha256":"777f5d93091940d9304588fd481dacbf43c5d6d0c1fe802aee446ed88c7bcec3","citation":"L43–92,L202–264"} | met |  |
| P20-SDK-CLASSIFIER-018 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L127 | {"path":"crates/opi-ai/src/classifier.rs","sha256":"777f5d93091940d9304588fd481dacbf43c5d6d0c1fe802aee446ed88c7bcec3","citation":"L43–92,L202–264"} | met |  |
| P20-SDK-CLASSIFIER-019 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L127 | {"path":"crates/opi-ai/src/classifier.rs","sha256":"777f5d93091940d9304588fd481dacbf43c5d6d0c1fe802aee446ed88c7bcec3","citation":"L43–92,L202–264"} | met |  |
| P20-SDK-CLASSIFIER-020 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L127 | {"path":"crates/opi-ai/src/classifier.rs","sha256":"777f5d93091940d9304588fd481dacbf43c5d6d0c1fe802aee446ed88c7bcec3","citation":"L43–92,L202–264"} | met |  |
| P20-SDK-CLASSIFIER-021 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L127 | {"path":"crates/opi-ai/src/classifier.rs","sha256":"777f5d93091940d9304588fd481dacbf43c5d6d0c1fe802aee446ed88c7bcec3","citation":"L43–92,L202–264"} | met |  |
| P20-SDK-CLASSIFIER-022 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L129 | {"path":"crates/opi-ai/src/classifier.rs","sha256":"777f5d93091940d9304588fd481dacbf43c5d6d0c1fe802aee446ed88c7bcec3","citation":"L43–92,L202–264"} | met |  |
| P20-SDK-CLASSIFIER-023 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L129 | {"path":"crates/opi-ai/src/classifier.rs","sha256":"777f5d93091940d9304588fd481dacbf43c5d6d0c1fe802aee446ed88c7bcec3","citation":"L43–92,L202–264"} | not-assessable | P20-AUD-018 |
| P20-SDK-CLASSIFIER-024 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L129 | {"path":"crates/opi-ai/src/classifier.rs","sha256":"777f5d93091940d9304588fd481dacbf43c5d6d0c1fe802aee446ed88c7bcec3","citation":"L43–92,L202–264"} | not-assessable | P20-AUD-018 |
| P20-SDK-CLASSIFIER-025 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L129 | {"path":"crates/opi-ai/src/classifier.rs","sha256":"777f5d93091940d9304588fd481dacbf43c5d6d0c1fe802aee446ed88c7bcec3","citation":"L43–92,L202–264"} | met |  |
| P20-SDK-CLASSIFIER-026 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L129 | {"path":"crates/opi-ai/src/classifier.rs","sha256":"777f5d93091940d9304588fd481dacbf43c5d6d0c1fe802aee446ed88c7bcec3","citation":"L43–92,L202–264"} | met |  |
| P20-SDK-CLASSIFIER-027 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L129 | {"path":"crates/opi-ai/src/classifier.rs","sha256":"777f5d93091940d9304588fd481dacbf43c5d6d0c1fe802aee446ed88c7bcec3","citation":"L43–92,L202–264"} | met |  |
| P20-SDK-CLASSIFIER-028 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L129 | {"path":"crates/opi-ai/src/classifier.rs","sha256":"777f5d93091940d9304588fd481dacbf43c5d6d0c1fe802aee446ed88c7bcec3","citation":"L43–92,L202–264"} | met |  |
| P20-SDK-CLASSIFIER-029 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L129 | {"path":"crates/opi-ai/src/classifier.rs","sha256":"777f5d93091940d9304588fd481dacbf43c5d6d0c1fe802aee446ed88c7bcec3","citation":"L43–92,L202–264"} | met |  |
| P20-SDK-CLASSIFIER-030 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L131 | {"path":"crates/opi-ai/src/classifier.rs","sha256":"777f5d93091940d9304588fd481dacbf43c5d6d0c1fe802aee446ed88c7bcec3","citation":"L43–92,L202–264"} | met |  |
| P20-SDK-CLASSIFIER-031 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L131 | {"path":"crates/opi-ai/src/classifier.rs","sha256":"777f5d93091940d9304588fd481dacbf43c5d6d0c1fe802aee446ed88c7bcec3","citation":"L43–92,L202–264"} | met |  |
| P20-SDK-CLASSIFIER-032 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L131 | {"path":"crates/opi-ai/src/classifier.rs","sha256":"777f5d93091940d9304588fd481dacbf43c5d6d0c1fe802aee446ed88c7bcec3","citation":"L43–92"} | not-met | P20-AUD-011 |
| P20-SDK-CLASSIFIER-033 | docs/research/pi-ai-opi-ai-alignment/acceptance-matrix.md L248 | {"path":"crates/opi-ai/src/classifier.rs","sha256":"777f5d93091940d9304588fd481dacbf43c5d6d0c1fe802aee446ed88c7bcec3","citation":"L43–92,L202–264"} | met |  |
| P20-SDK-CLASSIFIER-034 | docs/research/pi-ai-opi-ai-alignment/acceptance-matrix.md L248 | {"path":"crates/opi-ai/src/classifier.rs","sha256":"777f5d93091940d9304588fd481dacbf43c5d6d0c1fe802aee446ed88c7bcec3","citation":"L43–92,L202–264"} | met |  |
| P20-SDK-CLASSIFIER-035 | docs/research/pi-ai-opi-ai-alignment/acceptance-matrix.md L249 | {"path":"crates/opi-ai/src/classifier.rs","sha256":"777f5d93091940d9304588fd481dacbf43c5d6d0c1fe802aee446ed88c7bcec3","citation":"L43–92,L202–264"} | met |  |
| P20-SDK-MODEL-001 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L159 | {"path":"crates/opi-ai/src/model_info.rs","sha256":"d1fe2274564f1a5da80950b7e11ee6e545c886b0d6fb617c607029dd8246b4d9","citation":"L1176–1195"} | not-met | P20-AUD-019 |
| P20-SDK-MODEL-002 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L160 | {"path":"crates/opi-ai/src/model_info.rs","sha256":"d1fe2274564f1a5da80950b7e11ee6e545c886b0d6fb617c607029dd8246b4d9","citation":"L217–331 thinking map;L919–1008 pricing;L1074–1097 | met |  |
| P20-SDK-MODEL-003 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L160 | {"path":"crates/opi-ai/src/model_info.rs","sha256":"d1fe2274564f1a5da80950b7e11ee6e545c886b0d6fb617c607029dd8246b4d9","citation":"owning production module; full per-case trace not  | not-assessable | P20-AUD-018 |
| P20-SDK-MODEL-004 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L160 | {"path":"crates/opi-ai/src/model_info.rs","sha256":"d1fe2274564f1a5da80950b7e11ee6e545c886b0d6fb617c607029dd8246b4d9","citation":"owning production module; full per-case trace not  | not-assessable | P20-AUD-018 |
| P20-SDK-MODEL-005 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L160 | {"path":"crates/opi-ai/src/model_info.rs","sha256":"d1fe2274564f1a5da80950b7e11ee6e545c886b0d6fb617c607029dd8246b4d9","citation":"L217–331 thinking map;L919–1008 pricing;L1074–1097 | met |  |
| P20-SDK-MODEL-006 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L160 | {"path":"crates/opi-ai/src/model_info.rs","sha256":"d1fe2274564f1a5da80950b7e11ee6e545c886b0d6fb617c607029dd8246b4d9","citation":"L217–331 thinking map;L919–1008 pricing;L1074–1097 | met |  |
| P20-SDK-MODEL-007 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L161 | {"path":"crates/opi-ai/src/model_info.rs","sha256":"d1fe2274564f1a5da80950b7e11ee6e545c886b0d6fb617c607029dd8246b4d9","citation":"L217–331 thinking map;L919–1008 pricing;L1074–1097 | met |  |
| P20-SDK-MODEL-008 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L161 | {"path":"crates/opi-ai/src/model_info.rs","sha256":"d1fe2274564f1a5da80950b7e11ee6e545c886b0d6fb617c607029dd8246b4d9","citation":"L217–331 thinking map;L919–1008 pricing;L1074–1097 | met |  |
| P20-SDK-MODEL-009 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L161 | {"path":"crates/opi-ai/src/model_info.rs","sha256":"d1fe2274564f1a5da80950b7e11ee6e545c886b0d6fb617c607029dd8246b4d9","citation":"L217–331 thinking map;L919–1008 pricing;L1074–1097 | met |  |
| P20-SDK-MODEL-010 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L161 | {"path":"crates/opi-ai/src/model_info.rs","sha256":"d1fe2274564f1a5da80950b7e11ee6e545c886b0d6fb617c607029dd8246b4d9","citation":"L217–331 thinking map;L919–1008 pricing;L1074–1097 | met |  |
| P20-SDK-MODEL-011 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L161 | {"path":"crates/opi-ai/src/model_info.rs","sha256":"d1fe2274564f1a5da80950b7e11ee6e545c886b0d6fb617c607029dd8246b4d9","citation":"L217–331 thinking map;L919–1008 pricing;L1074–1097 | met |  |
| P20-SDK-MODEL-012 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L162 | {"path":"crates/opi-ai/src/model_info.rs","sha256":"d1fe2274564f1a5da80950b7e11ee6e545c886b0d6fb617c607029dd8246b4d9","citation":"L217–331 thinking map;L919–1008 pricing;L1074–1097 | met |  |
| P20-SDK-MODEL-013 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L162 | {"path":"crates/opi-ai/src/model_info.rs","sha256":"d1fe2274564f1a5da80950b7e11ee6e545c886b0d6fb617c607029dd8246b4d9","citation":"L217–331 thinking map;L919–1008 pricing;L1074–1097 | met |  |
| P20-SDK-MODEL-014 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L162 | {"path":"crates/opi-ai/src/model_info.rs","sha256":"d1fe2274564f1a5da80950b7e11ee6e545c886b0d6fb617c607029dd8246b4d9","citation":"L217–331 thinking map;L919–1008 pricing;L1074–1097 | met |  |
| P20-SDK-MODEL-015 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L163 | {"path":"crates/opi-ai/src/model_info.rs","sha256":"d1fe2274564f1a5da80950b7e11ee6e545c886b0d6fb617c607029dd8246b4d9","citation":"L217–331 thinking map;L919–1008 pricing;L1074–1097 | met |  |
| P20-SDK-MODEL-016 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L163 | {"path":"crates/opi-ai/src/model_info.rs","sha256":"d1fe2274564f1a5da80950b7e11ee6e545c886b0d6fb617c607029dd8246b4d9","citation":"L1176–1195"} | not-met | P20-AUD-019 |
| P20-SDK-MODEL-017 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L164 | {"path":"crates/opi-ai/src/model_info.rs","sha256":"d1fe2274564f1a5da80950b7e11ee6e545c886b0d6fb617c607029dd8246b4d9","citation":"L217–331 thinking map;L919–1008 pricing;L1074–1097 | met |  |
| P20-SDK-MODEL-018 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L164 | {"path":"crates/opi-ai/src/model_info.rs","sha256":"d1fe2274564f1a5da80950b7e11ee6e545c886b0d6fb617c607029dd8246b4d9","citation":"L217–331 thinking map;L919–1008 pricing;L1074–1097 | met |  |
| P20-SDK-MODEL-019 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L164 | {"path":"crates/opi-ai/src/model_info.rs","sha256":"d1fe2274564f1a5da80950b7e11ee6e545c886b0d6fb617c607029dd8246b4d9","citation":"L217–331 thinking map;L919–1008 pricing;L1074–1097 | met |  |
| P20-SDK-MODEL-020 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L165 | {"path":"crates/opi-ai/src/model_info.rs","sha256":"d1fe2274564f1a5da80950b7e11ee6e545c886b0d6fb617c607029dd8246b4d9","citation":"L217–331 thinking map;L919–1008 pricing;L1074–1097 | met |  |
| P20-SDK-MODEL-021 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L165 | {"path":"crates/opi-ai/src/model_info.rs","sha256":"d1fe2274564f1a5da80950b7e11ee6e545c886b0d6fb617c607029dd8246b4d9","citation":"owning production module; full per-case trace not  | not-assessable | P20-AUD-018 |
| P20-SDK-MODEL-022 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L165 | {"path":"crates/opi-ai/src/model_info.rs","sha256":"d1fe2274564f1a5da80950b7e11ee6e545c886b0d6fb617c607029dd8246b4d9","citation":"owning production module; full per-case trace not  | not-assessable | P20-AUD-018 |
| P20-SDK-MODEL-023 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L165 | {"path":"crates/opi-ai/src/models.rs","sha256":"f9fc8b51c90282772b6b8812079772ad05c264c0b7d80efbd51314a8b46e914b","citation":"L3930–3985"} | not-met | P20-AUD-008 |
| P20-SDK-MODEL-024 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L165 | {"path":"crates/opi-ai/src/model_info.rs","sha256":"d1fe2274564f1a5da80950b7e11ee6e545c886b0d6fb617c607029dd8246b4d9","citation":"L217–331 thinking map;L919–1008 pricing;L1074–1097 | met |  |
| P20-SDK-MODEL-025 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L165 | {"path":"crates/opi-ai/src/model_info.rs","sha256":"d1fe2274564f1a5da80950b7e11ee6e545c886b0d6fb617c607029dd8246b4d9","citation":"L217–331 thinking map;L919–1008 pricing;L1074–1097 | met |  |
| P20-SDK-MODEL-026 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L165 | {"path":"crates/opi-ai/src/model_info.rs","sha256":"d1fe2274564f1a5da80950b7e11ee6e545c886b0d6fb617c607029dd8246b4d9","citation":"owning production module; full per-case trace not  | not-assessable | P20-AUD-018 |
| P20-SDK-MODEL-027 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L166 | {"path":"crates/opi-ai/src/model_info.rs","sha256":"d1fe2274564f1a5da80950b7e11ee6e545c886b0d6fb617c607029dd8246b4d9","citation":"L217–331 thinking map;L919–1008 pricing;L1074–1097 | met |  |
| P20-SDK-MODEL-028 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L166 | {"path":"crates/opi-ai/src/model_info.rs","sha256":"d1fe2274564f1a5da80950b7e11ee6e545c886b0d6fb617c607029dd8246b4d9","citation":"L217–331 thinking map;L919–1008 pricing;L1074–1097 | met |  |
| P20-SDK-MODEL-029 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L166 | {"path":"crates/opi-ai/src/model_info.rs","sha256":"d1fe2274564f1a5da80950b7e11ee6e545c886b0d6fb617c607029dd8246b4d9","citation":"owning production module; full per-case trace not  | not-assessable | P20-AUD-018 |
| P20-SDK-MODEL-030 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L168 | {"path":"crates/opi-ai/src/model_info.rs","sha256":"d1fe2274564f1a5da80950b7e11ee6e545c886b0d6fb617c607029dd8246b4d9","citation":"owning production module; full per-case trace not  | not-assessable | P20-AUD-018 |
| P20-SDK-MODEL-031 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L168 | {"path":"crates/opi-ai/src/model_info.rs","sha256":"d1fe2274564f1a5da80950b7e11ee6e545c886b0d6fb617c607029dd8246b4d9","citation":"L217–331 thinking map;L919–1008 pricing;L1074–1097 | met |  |
| P20-SDK-MODEL-032 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L172 | {"path":"crates/opi-ai/src/model_info.rs","sha256":"d1fe2274564f1a5da80950b7e11ee6e545c886b0d6fb617c607029dd8246b4d9","citation":"L217–331 thinking map;L919–1008 pricing;L1074–1097 | met |  |
| P20-SDK-MODEL-033 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L173 | {"path":"crates/opi-ai/src/model_info.rs","sha256":"d1fe2274564f1a5da80950b7e11ee6e545c886b0d6fb617c607029dd8246b4d9","citation":"L217–331 thinking map;L919–1008 pricing;L1074–1097 | met |  |
| P20-SDK-MODEL-034 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L173 | {"path":"crates/opi-ai/src/model_info.rs","sha256":"d1fe2274564f1a5da80950b7e11ee6e545c886b0d6fb617c607029dd8246b4d9","citation":"L217–331 thinking map;L919–1008 pricing;L1074–1097 | met |  |
| P20-SDK-MODEL-035 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L173 | {"path":"crates/opi-ai/src/model_info.rs","sha256":"d1fe2274564f1a5da80950b7e11ee6e545c886b0d6fb617c607029dd8246b4d9","citation":"L217–331 thinking map;L919–1008 pricing;L1074–1097 | met |  |
| P20-SDK-MODEL-036 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L174 | {"path":"crates/opi-ai/src/model_info.rs","sha256":"d1fe2274564f1a5da80950b7e11ee6e545c886b0d6fb617c607029dd8246b4d9","citation":"L217–331 thinking map;L919–1008 pricing;L1074–1097 | met |  |
| P20-SDK-MODEL-037 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L174 | {"path":"crates/opi-ai/src/model_info.rs","sha256":"d1fe2274564f1a5da80950b7e11ee6e545c886b0d6fb617c607029dd8246b4d9","citation":"L217–331 thinking map;L919–1008 pricing;L1074–1097 | met |  |
| P20-SDK-MODEL-038 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L175 | {"path":"crates/opi-ai/src/model_info.rs","sha256":"d1fe2274564f1a5da80950b7e11ee6e545c886b0d6fb617c607029dd8246b4d9","citation":"L217–331 thinking map;L919–1008 pricing;L1074–1097 | not-assessable | P20-AUD-018 |
| P20-SDK-MODEL-039 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L175 | {"path":"crates/opi-ai/src/model_info.rs","sha256":"d1fe2274564f1a5da80950b7e11ee6e545c886b0d6fb617c607029dd8246b4d9","citation":"L217–331 thinking map;L919–1008 pricing;L1074–1097 | met |  |
| P20-SDK-MODEL-040 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L176 | {"path":"crates/opi-ai/src/model_info.rs","sha256":"d1fe2274564f1a5da80950b7e11ee6e545c886b0d6fb617c607029dd8246b4d9","citation":"L349–364"} | not-met | P20-AUD-009 |
| P20-SDK-MODEL-041 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L176 | {"path":"crates/opi-ai/src/model_info.rs","sha256":"d1fe2274564f1a5da80950b7e11ee6e545c886b0d6fb617c607029dd8246b4d9","citation":"L349–364"} | not-met | P20-AUD-009 |
| P20-SDK-MODEL-042 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L177 | {"path":"crates/opi-ai/src/model_info.rs","sha256":"d1fe2274564f1a5da80950b7e11ee6e545c886b0d6fb617c607029dd8246b4d9","citation":"L217–331 thinking map;L919–1008 pricing;L1074–1097 | not-assessable | P20-AUD-018 |
| P20-SDK-UTILITY-001 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L183 | {"path":"crates/opi-ai/src/callbacks.rs","sha256":"29189c6c2c33016ff85cc99212e2f6027f1a689624cd35952dc69b84be5df4f8","citation":"1-35,141-230,313-500"} | met |  |
| P20-SDK-UTILITY-002 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L183 | {"path":"crates/opi-ai/src/callbacks.rs","sha256":"29189c6c2c33016ff85cc99212e2f6027f1a689624cd35952dc69b84be5df4f8","citation":"1-35,141-230,313-500"} | met |  |
| P20-SDK-UTILITY-003 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L183 | {"path":"crates/opi-ai/src/callbacks.rs","sha256":"29189c6c2c33016ff85cc99212e2f6027f1a689624cd35952dc69b84be5df4f8","citation":"1-35,141-230,313-500"} | met |  |
| P20-SDK-UTILITY-004 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L184 | {"path":"crates/opi-ai/src/callbacks.rs","sha256":"29189c6c2c33016ff85cc99212e2f6027f1a689624cd35952dc69b84be5df4f8","citation":"1-35,141-230,313-500"} | met |  |
| P20-SDK-UTILITY-005 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L184 | {"path":"crates/opi-ai/src/callbacks.rs","sha256":"29189c6c2c33016ff85cc99212e2f6027f1a689624cd35952dc69b84be5df4f8","citation":"1-35,141-230,313-500"} | met |  |
| P20-SDK-UTILITY-006 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L184 | {"path":"crates/opi-ai/src/callbacks.rs","sha256":"29189c6c2c33016ff85cc99212e2f6027f1a689624cd35952dc69b84be5df4f8","citation":"1-35,141-230,313-500"} | met |  |
| P20-SDK-UTILITY-007 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L185 | {"path":"crates/opi-ai/src/callbacks.rs","sha256":"29189c6c2c33016ff85cc99212e2f6027f1a689624cd35952dc69b84be5df4f8","citation":"1-35,141-230,313-500"} | met |  |
| P20-SDK-UTILITY-008 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L185 | {"path":"crates/opi-ai/src/callbacks.rs","sha256":"29189c6c2c33016ff85cc99212e2f6027f1a689624cd35952dc69b84be5df4f8","citation":"1-35,141-230,313-500"} | met |  |
| P20-SDK-UTILITY-009 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L186 | {"path":"crates/opi-ai/src/retry.rs","sha256":"6ee0b8b45005f37866caab4c8191607bbc89607929dca1503a033a56f2ec430c","citation":"AcquisitionPolicy/state and WholeCallRetryPolicy;lines7 | met |  |
| P20-SDK-UTILITY-010 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L186 | {"path":"crates/opi-ai/src/retry.rs","sha256":"6ee0b8b45005f37866caab4c8191607bbc89607929dca1503a033a56f2ec430c","citation":"AcquisitionPolicy/state and WholeCallRetryPolicy;lines7 | met |  |
| P20-SDK-UTILITY-011 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L186 | {"path":"crates/opi-ai/src/retry.rs","sha256":"6ee0b8b45005f37866caab4c8191607bbc89607929dca1503a033a56f2ec430c","citation":"AcquisitionPolicy/state and WholeCallRetryPolicy;lines7 | not-met | P20-AUD-006 |
| P20-SDK-UTILITY-012 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L186 | {"path":"crates/opi-ai/src/retry.rs","sha256":"6ee0b8b45005f37866caab4c8191607bbc89607929dca1503a033a56f2ec430c","citation":"AcquisitionPolicy/state and WholeCallRetryPolicy;lines7 | met |  |
| P20-SDK-UTILITY-013 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L187 | {"path":"crates/opi-ai/src/retry.rs","sha256":"6ee0b8b45005f37866caab4c8191607bbc89607929dca1503a033a56f2ec430c","citation":"AcquisitionPolicy/state and WholeCallRetryPolicy;lines7 | met |  |
| P20-SDK-UTILITY-014 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L187 | {"path":"crates/opi-ai/src/retry.rs","sha256":"6ee0b8b45005f37866caab4c8191607bbc89607929dca1503a033a56f2ec430c","citation":"AcquisitionPolicy/state and WholeCallRetryPolicy;lines7 | met |  |
| P20-SDK-UTILITY-015 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L187 | {"path":"crates/opi-ai/src/retry.rs","sha256":"6ee0b8b45005f37866caab4c8191607bbc89607929dca1503a033a56f2ec430c","citation":"AcquisitionPolicy/state and WholeCallRetryPolicy;lines7 | met |  |
| P20-SDK-UTILITY-016 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L187 | {"path":"crates/opi-ai/src/retry.rs","sha256":"6ee0b8b45005f37866caab4c8191607bbc89607929dca1503a033a56f2ec430c","citation":"AcquisitionPolicy/state and WholeCallRetryPolicy;lines7 | met |  |
| P20-SDK-UTILITY-017 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L187 | {"path":"crates/opi-ai/src/retry.rs","sha256":"6ee0b8b45005f37866caab4c8191607bbc89607929dca1503a033a56f2ec430c","citation":"AcquisitionPolicy/state and WholeCallRetryPolicy;lines7 | met |  |
| P20-SDK-UTILITY-018 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L188 | {"path":"crates/opi-ai/src/deferred.rs","sha256":"dce0171b8766a6fe24f3ec74c28e37822d25de4135041650dde270c592fd2578","citation":"32-156,201-end"} | met |  |
| P20-SDK-UTILITY-019 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L188 | {"path":"crates/opi-ai/src/deferred.rs","sha256":"dce0171b8766a6fe24f3ec74c28e37822d25de4135041650dde270c592fd2578","citation":"32-156,201-end"} | met |  |
| P20-SDK-UTILITY-020 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L188 | {"path":"crates/opi-ai/src/deferred.rs","sha256":"dce0171b8766a6fe24f3ec74c28e37822d25de4135041650dde270c592fd2578","citation":"32-156,201-end"} | met |  |
| P20-SDK-UTILITY-021 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L188 | {"path":"crates/opi-ai/src/deferred.rs","sha256":"dce0171b8766a6fe24f3ec74c28e37822d25de4135041650dde270c592fd2578","citation":"32-156,201-end"} | met |  |
| P20-SDK-UTILITY-022 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L189 | {"path":"crates/opi-ai/src/message.rs","sha256":"e3ed384c972dae5acadc5a41eca7db30500c90ec26437ab32f2c10492a2501b9","citation":"166-203,308-441,648-666,684-826"} | not-met | P20-AUD-003 |
| P20-SDK-UTILITY-023 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L189 | {"path":"crates/opi-ai/src/message.rs","sha256":"e3ed384c972dae5acadc5a41eca7db30500c90ec26437ab32f2c10492a2501b9","citation":"166-203,308-441,648-666,684-826"} | met |  |
| P20-SDK-UTILITY-024 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L189 | {"path":"crates/opi-ai/src/message.rs","sha256":"e3ed384c972dae5acadc5a41eca7db30500c90ec26437ab32f2c10492a2501b9","citation":"166-203,308-441,648-666,684-826"} | met |  |
| P20-SDK-UTILITY-025 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L190 | {"path":"crates/opi-ai/src/message.rs","sha256":"e3ed384c972dae5acadc5a41eca7db30500c90ec26437ab32f2c10492a2501b9","citation":"166-203,308-441,648-666,684-826"} | met |  |
| P20-SDK-UTILITY-026 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L190 | {"path":"crates/opi-ai/src/message.rs","sha256":"e3ed384c972dae5acadc5a41eca7db30500c90ec26437ab32f2c10492a2501b9","citation":"166-203,308-441,648-666,684-826"} | met |  |
| P20-SDK-UTILITY-027 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L190 | {"path":"crates/opi-ai/src/message.rs","sha256":"e3ed384c972dae5acadc5a41eca7db30500c90ec26437ab32f2c10492a2501b9","citation":"166-203,308-441,648-666,684-826"} | met |  |
| P20-SDK-UTILITY-028 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L190 | {"path":"crates/opi-ai/src/message.rs","sha256":"e3ed384c972dae5acadc5a41eca7db30500c90ec26437ab32f2c10492a2501b9","citation":"308-441"} | not-assessable | P20-AUD-018 |
| P20-SDK-UTILITY-029 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L191 | {"path":"crates/opi-ai/src/message.rs","sha256":"e3ed384c972dae5acadc5a41eca7db30500c90ec26437ab32f2c10492a2501b9","citation":"308-441"} | met |  |
| P20-SDK-UTILITY-030 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L191 | {"path":"crates/opi-ai/src/message.rs","sha256":"e3ed384c972dae5acadc5a41eca7db30500c90ec26437ab32f2c10492a2501b9","citation":"308-441"} | met |  |
| P20-SDK-UTILITY-031 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L191 | {"path":"crates/opi-ai/src/message.rs","sha256":"e3ed384c972dae5acadc5a41eca7db30500c90ec26437ab32f2c10492a2501b9","citation":"308-441"} | not-assessable | P20-AUD-018 |
| P20-SDK-UTILITY-032 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L191 | {"path":"crates/opi-ai/src/message.rs","sha256":"e3ed384c972dae5acadc5a41eca7db30500c90ec26437ab32f2c10492a2501b9","citation":"308-441"} | not-assessable | P20-AUD-018 |
| P20-SDK-UTILITY-033 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L192 | {"path":"crates/opi-ai/src/provider.rs","sha256":"3174b522aea1586b2356feabe519804527f1a94ac6884237234e4d96b26bba92","citation":"336-367"} | met |  |
| P20-SDK-UTILITY-034 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L192 | {"path":"crates/opi-ai/src/provider.rs","sha256":"3174b522aea1586b2356feabe519804527f1a94ac6884237234e4d96b26bba92","citation":"336-367"} | not-assessable | P20-AUD-018 |
| P20-SDK-UTILITY-035 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L193 | {"path":"crates/opi-ai/src/provider_env.rs","sha256":"7398bb718b503fd4f93fb4ac08129d93f82667662a9a441e50cefd97ad8f6b71","citation":"1-34"} | not-met | P20-AUD-010 |
| P20-SDK-UTILITY-036 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L193 | {"path":"crates/opi-ai/src/provider_env.rs","sha256":"7398bb718b503fd4f93fb4ac08129d93f82667662a9a441e50cefd97ad8f6b71","citation":"1-34"} | not-assessable | P20-AUD-018 |
| P20-SDK-UTILITY-037 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L193 | {"path":"crates/opi-ai/src/provider_env.rs","sha256":"7398bb718b503fd4f93fb4ac08129d93f82667662a9a441e50cefd97ad8f6b71","citation":"1-34"} | met |  |
| P20-SDK-UTILITY-038 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L194 | {"path":"crates/opi-ai/src/cancel.rs","sha256":"f23cfdcb4f78cdc2dfccf0e31c46174afbabc7cb4d78f79425fba0b7cebfd230","citation":"1-69"} | met |  |
| P20-SDK-UTILITY-039 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L194 | {"path":"crates/opi-ai/src/cancel.rs","sha256":"f23cfdcb4f78cdc2dfccf0e31c46174afbabc7cb4d78f79425fba0b7cebfd230","citation":"1-69"} | met |  |
| P20-SDK-UTILITY-040 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L194 | {"path":"crates/opi-ai/src/cancel.rs","sha256":"f23cfdcb4f78cdc2dfccf0e31c46174afbabc7cb4d78f79425fba0b7cebfd230","citation":"1-69"} | met |  |
| P20-SDK-UTILITY-041 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L195 | {"path":"crates/opi-ai/src/test_support.rs","sha256":"6a90151986b732fe697d661e3bf4ff4dc2ffc505c0e21ea5e37f5d7c8cececc2","citation":"MockProvider/ScriptedDeferredBackend"} | met |  |
| P20-SDK-UTILITY-042 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L195 | {"path":"crates/opi-ai/src/test_support.rs","sha256":"6a90151986b732fe697d661e3bf4ff4dc2ffc505c0e21ea5e37f5d7c8cececc2","citation":"MockProvider/ScriptedDeferredBackend"} | met |  |
| P20-SDK-UTILITY-043 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L195 | {"path":"crates/opi-ai/src/test_support.rs","sha256":"6a90151986b732fe697d661e3bf4ff4dc2ffc505c0e21ea5e37f5d7c8cececc2","citation":"MockProvider/ScriptedDeferredBackend"} | met |  |
| P20-SDK-UTILITY-044 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L212 | {"path":"crates/opi-ai/src/message.rs","sha256":"e3ed384c972dae5acadc5a41eca7db30500c90ec26437ab32f2c10492a2501b9","citation":"308-441,648-666"} | met |  |
| P20-SDK-UTILITY-045 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L238 | {"path":"crates/opi-ai/src/message.rs","sha256":"e3ed384c972dae5acadc5a41eca7db30500c90ec26437ab32f2c10492a2501b9","citation":"308-441,648-666"} | met |  |
| P20-SDK-UTILITY-046 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L238 | {"path":"crates/opi-ai/src/message.rs","sha256":"e3ed384c972dae5acadc5a41eca7db30500c90ec26437ab32f2c10492a2501b9","citation":"308-441,648-666"} | met |  |
| P20-SDK-UTILITY-047 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L238 | {"path":"crates/opi-ai/src/message.rs","sha256":"e3ed384c972dae5acadc5a41eca7db30500c90ec26437ab32f2c10492a2501b9","citation":"308-441,648-666"} | met |  |
| P20-SDK-UTILITY-048 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L244 | {"path":"crates/opi-ai/src/message.rs","sha256":"e3ed384c972dae5acadc5a41eca7db30500c90ec26437ab32f2c10492a2501b9","citation":"308-441,648-666"} | met |  |
| P20-SDK-RESOURCE-001 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-rust-differences.md L163 | crates/opi-ai/src/models.rs:2204 (first-poll cutoff, exact scope, all-owner cleanup aggregation) | met |  |
| P20-SDK-RESOURCE-002 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-rust-differences.md L163 | crates/opi-ai/src/models.rs:2204 (first-poll cutoff, exact scope, all-owner cleanup aggregation) | met |  |
| P20-SDK-RESOURCE-003 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-rust-differences.md L163 | crates/opi-ai/src/models.rs:2204 (first-poll cutoff, exact scope, all-owner cleanup aggregation) | met |  |
| P20-SDK-RESOURCE-004 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-rust-differences.md L164 | crates/opi-ai/src/models.rs:2204 (first-poll cutoff, exact scope, all-owner cleanup aggregation) | met |  |
| P20-SDK-RESOURCE-005 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-rust-differences.md L164 | crates/opi-ai/src/models.rs:2204 (first-poll cutoff, exact scope, all-owner cleanup aggregation) | met |  |
| P20-SDK-RESOURCE-006 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-rust-differences.md L165 | crates/opi-ai/src/models.rs:2204 (first-poll cutoff, exact scope, all-owner cleanup aggregation) | met |  |
| P20-SDK-RESOURCE-007 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-rust-differences.md L181 | crates/opi-ai/src/models.rs:2204 (first-poll cutoff, exact scope, all-owner cleanup aggregation) | met |  |
| P20-SDK-RESOURCE-008 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-rust-differences.md L183 | crates/opi-ai/src/models.rs:2204 (first-poll cutoff, exact scope, all-owner cleanup aggregation) | met |  |
| P20-SDK-CATALOG-001 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L201 | {"path":"crates/opi-ai/src/models.rs","sha256":"f9fc8b51c90282772b6b8812079772ad05c264c0b7d80efbd51314a8b46e914b","citation":"L2992–3231 restore-before-network, sequential bounded  | met |  |
| P20-SDK-CATALOG-002 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L201 | {"path":"crates/opi-ai/src/models.rs","sha256":"f9fc8b51c90282772b6b8812079772ad05c264c0b7d80efbd51314a8b46e914b","citation":"L2992–3231 restore-before-network, sequential bounded  | met |  |
| P20-SDK-CATALOG-003 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L201 | {"path":"crates/opi-ai/src/models.rs","sha256":"f9fc8b51c90282772b6b8812079772ad05c264c0b7d80efbd51314a8b46e914b","citation":"L2992–3231 restore-before-network, sequential bounded  | met |  |
| P20-SDK-CATALOG-004 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L201 | {"path":"crates/opi-ai/src/models.rs","sha256":"f9fc8b51c90282772b6b8812079772ad05c264c0b7d80efbd51314a8b46e914b","citation":"L2992–3231 restore-before-network, sequential bounded  | met |  |
| P20-SDK-CATALOG-005 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L201 | {"path":"crates/opi-ai/src/models.rs","sha256":"f9fc8b51c90282772b6b8812079772ad05c264c0b7d80efbd51314a8b46e914b","citation":"L2992–3231 restore-before-network, sequential bounded  | met |  |
| P20-SDK-CATALOG-006 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L202 | {"path":"crates/opi-ai/src/models.rs","sha256":"f9fc8b51c90282772b6b8812079772ad05c264c0b7d80efbd51314a8b46e914b","citation":"L2992–3231 restore-before-network, sequential bounded  | met |  |
| P20-SDK-CATALOG-007 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L202 | {"path":"crates/opi-ai/src/models.rs","sha256":"f9fc8b51c90282772b6b8812079772ad05c264c0b7d80efbd51314a8b46e914b","citation":"L2992–3231 restore-before-network, sequential bounded  | met |  |
| P20-SDK-CATALOG-008 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L202 | {"path":"crates/opi-ai/src/models.rs","sha256":"f9fc8b51c90282772b6b8812079772ad05c264c0b7d80efbd51314a8b46e914b","citation":"L2992–3231 restore-before-network, sequential bounded  | met |  |
| P20-SDK-CATALOG-009 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L202 | {"path":"crates/opi-ai/src/models.rs","sha256":"f9fc8b51c90282772b6b8812079772ad05c264c0b7d80efbd51314a8b46e914b","citation":"L2992–3231 restore-before-network, sequential bounded  | met |  |
| P20-SDK-CATALOG-010 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L203 | {"path":"crates/opi-ai/src/models.rs","sha256":"f9fc8b51c90282772b6b8812079772ad05c264c0b7d80efbd51314a8b46e914b","citation":"L2992–3231 restore-before-network, sequential bounded  | met |  |
| P20-SDK-CATALOG-011 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L204 | {"path":"crates/opi-ai/src/models.rs","sha256":"f9fc8b51c90282772b6b8812079772ad05c264c0b7d80efbd51314a8b46e914b","citation":"L2992–3231 restore-before-network, sequential bounded  | met |  |
| P20-SDK-CATALOG-012 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L204 | {"path":"crates/opi-ai/src/models.rs","sha256":"f9fc8b51c90282772b6b8812079772ad05c264c0b7d80efbd51314a8b46e914b","citation":"L2992–3231 restore-before-network, sequential bounded  | met |  |
| P20-SDK-CATALOG-013 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L205 | {"path":"crates/opi-ai/src/models.rs","sha256":"f9fc8b51c90282772b6b8812079772ad05c264c0b7d80efbd51314a8b46e914b","citation":"L2992–3231 restore-before-network, sequential bounded  | met |  |
| P20-SDK-CATALOG-014 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L205 | {"path":"crates/opi-ai/src/models.rs","sha256":"f9fc8b51c90282772b6b8812079772ad05c264c0b7d80efbd51314a8b46e914b","citation":"L2992–3231 restore-before-network, sequential bounded  | met |  |
| P20-SDK-CATALOG-015 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L206 | {"path":"crates/opi-ai/src/models.rs","sha256":"f9fc8b51c90282772b6b8812079772ad05c264c0b7d80efbd51314a8b46e914b","citation":"L2992–3231 restore-before-network, sequential bounded  | met |  |
| P20-SDK-CATALOG-016 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L206 | {"path":"crates/opi-ai/src/catalog/mod.rs","sha256":"f8fdc6d1ab6a1401583552030bb95a99540c998d7f1da48ad7b48a784c3af588","citation":"owning production module; full per-case trace not | not-assessable | P20-AUD-018 |
| P20-SDK-CATALOG-017 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L206 | {"path":"crates/opi-ai/src/models.rs","sha256":"f9fc8b51c90282772b6b8812079772ad05c264c0b7d80efbd51314a8b46e914b","citation":"L2992–3231 restore-before-network, sequential bounded  | met |  |
| P20-SDK-CATALOG-018 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L246 | {"path":"crates/opi-ai/src/models.rs","sha256":"f9fc8b51c90282772b6b8812079772ad05c264c0b7d80efbd51314a8b46e914b","citation":"L2992–3231 restore-before-network, sequential bounded  | met |  |
| P20-SDK-CATALOG-019 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-rust-differences.md L190 | {"path":"crates/opi-ai/src/models.rs","sha256":"f9fc8b51c90282772b6b8812079772ad05c264c0b7d80efbd51314a8b46e914b","citation":"L2992–3231 restore-before-network, sequential bounded  | met |  |
| P20-SDK-CATALOG-020 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-rust-differences.md L192 | {"path":"crates/opi-ai/src/models.rs","sha256":"f9fc8b51c90282772b6b8812079772ad05c264c0b7d80efbd51314a8b46e914b","citation":"L2992–3231 restore-before-network, sequential bounded  | met |  |
| P20-SDK-CATALOG-021 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-rust-differences.md L228 | {"path":"crates/opi-ai/src/models.rs","sha256":"f9fc8b51c90282772b6b8812079772ad05c264c0b7d80efbd51314a8b46e914b","citation":"L2992–3231 restore-before-network, sequential bounded  | met |  |
| P20-SDK-TELEMETRY-001 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L209 | crates/opi-ai/src/telemetry.rs:66 (matched Span/Dispatch, noop, per-poll/drop synchronous scope) | met |  |
| P20-SDK-TELEMETRY-002 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L209 | crates/opi-ai/src/telemetry.rs:66 (matched Span/Dispatch, noop, per-poll/drop synchronous scope) | met |  |
| P20-SDK-TELEMETRY-003 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L209 | crates/opi-ai/src/telemetry.rs:66 (matched Span/Dispatch, noop, per-poll/drop synchronous scope) | met |  |
| P20-SDK-TELEMETRY-004 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L209 | crates/opi-ai/src/telemetry.rs:66 (matched Span/Dispatch, noop, per-poll/drop synchronous scope) | met |  |
| P20-SDK-TELEMETRY-005 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-rust-differences.md L172 | crates/opi-ai/src/telemetry.rs:66 (matched Span/Dispatch, noop, per-poll/drop synchronous scope) | met |  |
| P20-SDK-TELEMETRY-006 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-rust-differences.md L172 | crates/opi-ai/src/telemetry.rs:66 (matched Span/Dispatch, noop, per-poll/drop synchronous scope) | met |  |
| P20-SDK-TELEMETRY-007 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-rust-differences.md L174 | crates/opi-ai/src/telemetry.rs:66 (matched Span/Dispatch, noop, per-poll/drop synchronous scope) | met |  |
| P20-SDK-TELEMETRY-008 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-rust-differences.md L173 | crates/opi-ai/src/telemetry.rs:66 (matched Span/Dispatch, noop, per-poll/drop synchronous scope) | met |  |
| P20-SDK-PACKAGE-001 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L211 | {"path":"crates/opi-ai/src/lib.rs","sha256":"c8e9578fe152e477741979a64fb7c298f71513d4a53638223d711d631459ffdf","citation":"owning production module; full per-case trace not asserte | not-assessable | P20-AUD-018 |
| P20-SDK-PACKAGE-002 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L211 | {"path":"crates/opi-ai/src/lib.rs","sha256":"c8e9578fe152e477741979a64fb7c298f71513d4a53638223d711d631459ffdf","citation":"owning production module; full per-case trace not asserte | not-assessable | P20-AUD-018 |
| P20-SDK-PACKAGE-003 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L211 | {"path":"crates/opi-ai/src/lib.rs","sha256":"c8e9578fe152e477741979a64fb7c298f71513d4a53638223d711d631459ffdf","citation":"owning production module; full per-case trace not asserte | not-assessable | P20-AUD-018 |
| P20-SDK-PACKAGE-004 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L212 | {"path":"crates/opi-ai/src/lib.rs","sha256":"c8e9578fe152e477741979a64fb7c298f71513d4a53638223d711d631459ffdf","citation":"owning production module; full per-case trace not asserte | not-assessable | P20-AUD-018 |
| P20-SDK-PACKAGE-005 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L216 | {"path":"crates/opi-ai/src/lib.rs","sha256":"c8e9578fe152e477741979a64fb7c298f71513d4a53638223d711d631459ffdf","citation":"owning production module; full per-case trace not asserte | not-assessable | P20-AUD-018 |
| P20-SDK-PACKAGE-006 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L218 | {"path":"crates/opi-ai/src/lib.rs","sha256":"c8e9578fe152e477741979a64fb7c298f71513d4a53638223d711d631459ffdf","citation":"owning production module; full per-case trace not asserte | not-assessable | P20-AUD-018 |
| P20-SDK-PACKAGE-007 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L218 | {"path":"crates/opi-ai/src/lib.rs","sha256":"c8e9578fe152e477741979a64fb7c298f71513d4a53638223d711d631459ffdf","citation":"owning production module; full per-case trace not asserte | not-assessable | P20-AUD-018 |
| P20-SDK-PACKAGE-008 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L220 | {"path":"crates/opi-ai/src/lib.rs","sha256":"c8e9578fe152e477741979a64fb7c298f71513d4a53638223d711d631459ffdf","citation":"owning production module; full per-case trace not asserte | not-assessable | P20-AUD-018 |
| P20-SDK-PACKAGE-009 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md L218 | {"path":"crates/opi-ai/src/lib.rs","sha256":"c8e9578fe152e477741979a64fb7c298f71513d4a53638223d711d631459ffdf","citation":"owning production module; full per-case trace not asserte | not-assessable | P20-AUD-018 |
| P20-SDK-PACKAGE-010 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-rust-differences.md L211 | {"path":"crates/opi-ai/src/lib.rs","sha256":"c8e9578fe152e477741979a64fb7c298f71513d4a53638223d711d631459ffdf","citation":"owning production module; full per-case trace not asserte | not-assessable | P20-AUD-018 |
| P20-SDK-PACKAGE-011 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-rust-differences.md L274 | {"path":"crates/opi-ai/src/lib.rs","sha256":"c8e9578fe152e477741979a64fb7c298f71513d4a53638223d711d631459ffdf","citation":"owning production module; full per-case trace not asserte | not-assessable | P20-AUD-018 |
| P20-MECH-SCHEMA-001 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-schema.md L9,L14,L17 | {"path":"crates/opi-ai/src/schema/mod.rs","sha256":"816f780293f0426f0e063fae63111baf6a050d60999d99c1bad52be204d8af70","citation":"L138-L175","observed":"Public compile/check enter  | met |  |
| P20-MECH-SCHEMA-002 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-schema.md L14,L80 | {"path":"Cargo.toml","sha256":"11a2d459067c679bc06a1bc7af5fe67f4e72b9d856d34d2da83241ac0bb47f83","citation":"L36,L47,L56,L60","observed":"Root owns preserve_order, ordinary url and | met |  |
| P20-MECH-SCHEMA-003 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-schema.md L36 | {"path":"crates/opi-ai/src/schema/normalize.rs","sha256":"10491c6d01a2ce7aab562e805c7317e0b40bd6bae42b70ab62ceae475ae16b92","citation":"L66-L103","observed":"After optional-null/co | met |  |
| P20-MECH-SCHEMA-004 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-schema.md L36 | {"path":"crates/opi-ai/src/schema/ecma_number.rs","sha256":"bcbbcfde7b56062a89b7e810f0504690b2dc4d19aeeb55add065f2eb71b88a64","citation":"L224-L239","observed":"Precise JSON intege | met |  |
| P20-MECH-SCHEMA-005 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-schema.md L36 | {"path":"crates/opi-ai/src/message.rs","sha256":"e3ed384c972dae5acadc5a41eca7db30500c90ec26437ab32f2c10492a2501b9","citation":"ToolCall raw arguments and final argument parse bound | not-assessable | P20-AUD-018 |
| P20-MECH-SCHEMA-006 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-schema.md L44 | {"path":"crates/opi-ai/src/schema/mod.rs","sha256":"816f780293f0426f0e063fae63111baf6a050d60999d99c1bad52be204d8af70","citation":"L138-L175","observed":"Public compile/check enter  | met |  |
| P20-MECH-SCHEMA-007 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-schema.md L44 | {"path":"crates/opi-ai/src/schema/mod.rs","sha256":"816f780293f0426f0e063fae63111baf6a050d60999d99c1bad52be204d8af70","citation":"L138-L175","observed":"Public compile/check enter  | met |  |
| P20-MECH-SCHEMA-008 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-schema.md L50 | {"path":"crates/opi-ai/src/schema/mod.rs","sha256":"816f780293f0426f0e063fae63111baf6a050d60999d99c1bad52be204d8af70","citation":"L138-L175","observed":"Public compile/check enter  | met |  |
| P20-MECH-SCHEMA-009 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-schema.md L50 | {"path":"crates/opi-ai/src/schema/js.rs","sha256":"3af39d356d312550aaf07f9e2d6885a5204fcacb4c2eba1ef2a11947dac12842","citation":"js_keys L43; array_index tests L148-L174","observed | met |  |
| P20-MECH-SCHEMA-010 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-schema.md L54 | {"path":"crates/opi-ai/src/schema/mod.rs","sha256":"816f780293f0426f0e063fae63111baf6a050d60999d99c1bad52be204d8af70","citation":"SchemaError L87-L113; CompiledSchema::check L157-L | met |  |
| P20-MECH-SCHEMA-011 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-schema.md L54 | {"path":"crates/opi-ai/src/schema/eval.rs","sha256":"60ae4e2aa91e85b28f3f5281fda05f9feb77a87729467ec586e7fb95e716fa75","citation":"L440-L470,L1340-L1355","observed":"Every frame st | not-assessable | P20-AUD-018 |
| P20-MECH-SCHEMA-012 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-schema.md L54 | {"path":"crates/opi-ai/src/schema/mod.rs","sha256":"816f780293f0426f0e063fae63111baf6a050d60999d99c1bad52be204d8af70","citation":"L138-L175","observed":"Public compile/check enter  | met |  |
| P20-MECH-SCHEMA-013 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-schema.md L54 | {"path":"crates/opi-ai/src/schema/mod.rs","sha256":"816f780293f0426f0e063fae63111baf6a050d60999d99c1bad52be204d8af70","citation":"L138-L175","observed":"Public compile/check enter  | met |  |
| P20-MECH-SCHEMA-014 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-schema.md L56 | {"path":"crates/opi-ai/src/schema/mod.rs","sha256":"816f780293f0426f0e063fae63111baf6a050d60999d99c1bad52be204d8af70","citation":"L138-L175","observed":"Public compile/check enter  | met |  |
| P20-MECH-SCHEMA-015 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-schema.md L56 | {"path":"crates/opi-ai/src/schema/mod.rs","sha256":"816f780293f0426f0e063fae63111baf6a050d60999d99c1bad52be204d8af70","citation":"L138-L175","observed":"Public compile/check enter  | met |  |
| P20-MECH-SCHEMA-016 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-schema.md L60 | {"path":"crates/opi-ai/src/schema/ir.rs","sha256":"c4aee8ca7342ad8c31647b5de66a55bc18ec8d0d85cf87f17f302c63411a9ebe","citation":"PlanNode/RefBinding L55-L82","observed":"Compiled p | not-assessable | P20-AUD-018 |
| P20-MECH-SCHEMA-017 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-schema.md L60,L62 | {"path":"crates/opi-ai/src/schema/mod.rs","sha256":"816f780293f0426f0e063fae63111baf6a050d60999d99c1bad52be204d8af70","citation":"L138-L175","observed":"Public compile/check enter  | not-assessable | P20-AUD-018 |
| P20-MECH-SCHEMA-018 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-schema.md L60 | {"path":"crates/opi-ai/src/schema/mod.rs","sha256":"816f780293f0426f0e063fae63111baf6a050d60999d99c1bad52be204d8af70","citation":"L138-L175","observed":"Public compile/check enter  | met |  |
| P20-MECH-SCHEMA-019 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-schema.md L66 | {"path":"crates/opi-ai/src/schema/grapheme.rs","sha256":"dbd5da5dfd11e9c9b2fd878448fd14ade80e8c9dd06ea0b10880d7d1900aacde","citation":"L150-L188","observed":"Source UTF16 fast path | met |  |
| P20-MECH-SCHEMA-020 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-schema.md L66 | {"path":"crates/opi-ai/src/schema/eval.rs","sha256":"60ae4e2aa91e85b28f3f5281fda05f9feb77a87729467ec586e7fb95e716fa75","citation":"is_multiple_of L557-L595; dispatch L983-L986","ob | met |  |
| P20-MECH-SCHEMA-021 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-schema.md L68 | {"path":"crates/opi-ai/src/schema/formats.rs","sha256":"a7f00daf4f9da0a47e76ff48db1351204d8c5900cbe03a56b9915b4bd547ee6c","citation":"format_test L517-L555","observed":"Twenty-one  | not-assessable | P20-AUD-018 |
| P20-MECH-SCHEMA-022 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-schema.md L68 | {"path":"crates/opi-ai/src/schema/formats.rs","sha256":"a7f00daf4f9da0a47e76ff48db1351204d8c5900cbe03a56b9915b4bd547ee6c","citation":"L453-L555","observed":"URL/IRI/URI-reference p | met |  |
| P20-MECH-SCHEMA-023 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-schema.md L68 | {"path":"crates/opi-ai/src/schema/formats.rs","sha256":"a7f00daf4f9da0a47e76ff48db1351204d8c5900cbe03a56b9915b4bd547ee6c","citation":"L453-L555","observed":"URL/IRI/URI-reference p | met |  |
| P20-MECH-SCHEMA-024 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-schema.md L68 | {"path":"crates/opi-ai/src/schema/formats.rs","sha256":"a7f00daf4f9da0a47e76ff48db1351204d8c5900cbe03a56b9915b4bd547ee6c","citation":"L453-L555","observed":"URL/IRI/URI-reference p | met |  |
| P20-MECH-SCHEMA-025 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-schema.md L70,L72 | {"path":"crates/opi-ai/src/schema/mod.rs","sha256":"816f780293f0426f0e063fae63111baf6a050d60999d99c1bad52be204d8af70","citation":"L138-L175","observed":"Public compile/check enter  | not-assessable | P20-AUD-018 |
| P20-MECH-SCHEMA-026 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-schema.md L70,L80 | {"path":"crates/opi-ai/src/schema/mod.rs","sha256":"816f780293f0426f0e063fae63111baf6a050d60999d99c1bad52be204d8af70","citation":"L138-L175","observed":"Public compile/check enter  | met |  |
| P20-MECH-SCHEMA-027 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-schema.md L74 | {"path":"crates/opi-ai/src/schema/normalize.rs","sha256":"10491c6d01a2ce7aab562e805c7317e0b40bd6bae42b70ab62ceae475ae16b92","citation":"L66-L103,L197-L313,L406-L535,L547-L616","obs | met |  |
| P20-MECH-SCHEMA-028 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-schema.md L74 | {"path":"crates/opi-ai/src/schema/normalize.rs","sha256":"10491c6d01a2ce7aab562e805c7317e0b40bd6bae42b70ab62ceae475ae16b92","citation":"L66-L103,L197-L313,L406-L535,L547-L616","obs | not-met | P20-AUD-015 |
| P20-MECH-SCHEMA-029 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-schema.md L74 | {"path":"crates/opi-ai/src/schema/ecma_number.rs","sha256":"bcbbcfde7b56062a89b7e810f0504690b2dc4d19aeeb55add065f2eb71b88a64","citation":"L19-L215","observed":"Classifier recognize | met |  |
| P20-MECH-SCHEMA-030 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-schema.md L74 | {"path":"crates/opi-ai/src/schema/ecma_number.rs","sha256":"bcbbcfde7b56062a89b7e810f0504690b2dc4d19aeeb55add065f2eb71b88a64","citation":"L217-L239","observed":"ryu-js Buffer forma | met |  |
| P20-MECH-SCHEMA-031 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-schema.md L78 | {"path":"crates/opi-ai/src/schema/mod.rs","sha256":"816f780293f0426f0e063fae63111baf6a050d60999d99c1bad52be204d8af70","citation":"L138-L175","observed":"Public compile/check enter  | not-assessable | P20-AUD-018 |
| P20-MECH-SCHEMA-032 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-schema.md L78 | {"path":"crates/opi-ai/src/schema/mod.rs","sha256":"816f780293f0426f0e063fae63111baf6a050d60999d99c1bad52be204d8af70","citation":"L138-L175","observed":"Public compile/check enter  | not-assessable | P20-AUD-018 |
| P20-MECH-SCHEMA-033 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-schema.md L78 | {"path":"crates/opi-ai/src/schema/mod.rs","sha256":"816f780293f0426f0e063fae63111baf6a050d60999d99c1bad52be204d8af70","citation":"L138-L175","observed":"Public compile/check enter  | not-assessable | P20-AUD-018 |
| P20-MECH-SCHEMA-034 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-schema.md L78 | {"path":"crates/opi-ai/src/schema/mod.rs","sha256":"816f780293f0426f0e063fae63111baf6a050d60999d99c1bad52be204d8af70","citation":"L138-L175","observed":"Public compile/check enter  | not-assessable | P20-AUD-018 |
| P20-MECH-SCHEMA-035 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-schema.md L80,L86,L88 | {"path":"crates/opi-ai/src/schema/mod.rs","sha256":"816f780293f0426f0e063fae63111baf6a050d60999d99c1bad52be204d8af70","citation":"L138-L175","observed":"Public compile/check enter  | not-assessable | P20-AUD-018 |
| P20-MECH-SCHEMA-036 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-schema.md L86,L88 | {"path":"crates/opi-ai/src/schema/mod.rs","sha256":"816f780293f0426f0e063fae63111baf6a050d60999d99c1bad52be204d8af70","citation":"L138-L175","observed":"Public compile/check enter  | not-assessable | P20-AUD-018 |
| P20-MECH-WS-001 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-ws.md L94,L96,L98 | {"path":"crates/opi-ai/src/ws.rs","sha256":"c7f416ce886a590fd752ca8113c1fe7f1e7a0536497b3daa028bb95ebbca0254","citation":"L20-L27,L81-L102,L105-L135","observed":"Existing public Up | met |  |
| P20-MECH-WS-002 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-ws.md L94 | {"path":"crates/opi-ai/src/ws.rs","sha256":"c7f416ce886a590fd752ca8113c1fe7f1e7a0536497b3daa028bb95ebbca0254","citation":"L20-L27,L81-L102,L105-L135","observed":"Existing public Up | met |  |
| P20-MECH-WS-003 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-ws.md L96 | {"path":"crates/opi-ai/src/ws.rs","sha256":"c7f416ce886a590fd752ca8113c1fe7f1e7a0536497b3daa028bb95ebbca0254","citation":"L20-L27,L81-L102,L105-L135","observed":"Existing public Up | met |  |
| P20-MECH-WS-004 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-ws.md L100 | {"path":"crates/opi-ai/src/ws.rs","sha256":"c7f416ce886a590fd752ca8113c1fe7f1e7a0536497b3daa028bb95ebbca0254","citation":"L20-L27,L81-L102,L105-L135","observed":"Existing public Up | met |  |
| P20-MECH-WS-005 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-ws.md L100 | {"path":"crates/opi-ai/src/ws.rs","sha256":"c7f416ce886a590fd752ca8113c1fe7f1e7a0536497b3daa028bb95ebbca0254","citation":"L20-L27,L81-L102,L105-L135","observed":"Existing public Up | met |  |
| P20-MECH-WS-006 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-ws.md L100,L130,L134 | {"path":"crates/opi-ai/src/ws.rs","sha256":"c7f416ce886a590fd752ca8113c1fe7f1e7a0536497b3daa028bb95ebbca0254","citation":"L20-L27,L81-L102,L105-L135","observed":"Existing public Up | not-assessable | P20-AUD-018 |
| P20-MECH-WS-007 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-ws.md L104 | {"path":"crates/opi-ai/src/ws.rs","sha256":"c7f416ce886a590fd752ca8113c1fe7f1e7a0536497b3daa028bb95ebbca0254","citation":"L20-L27,L81-L102,L105-L135","observed":"Existing public Up | met |  |
| P20-MECH-WS-008 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-ws.md L104 | {"path":"crates/opi-ai/src/ws.rs","sha256":"c7f416ce886a590fd752ca8113c1fe7f1e7a0536497b3daa028bb95ebbca0254","citation":"L20-L27,L81-L102,L105-L135","observed":"Existing public Up | met |  |
| P20-MECH-WS-009 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-ws.md L104 | {"path":"crates/opi-ai/src/ws.rs","sha256":"c7f416ce886a590fd752ca8113c1fe7f1e7a0536497b3daa028bb95ebbca0254","citation":"L20-L27,L81-L102,L105-L135","observed":"Existing public Up | met |  |
| P20-MECH-WS-010 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-ws.md L104 | {"path":"crates/opi-ai/src/ws.rs","sha256":"c7f416ce886a590fd752ca8113c1fe7f1e7a0536497b3daa028bb95ebbca0254","citation":"L20-L27,L81-L102,L105-L135","observed":"Existing public Up | met |  |
| P20-MECH-WS-011 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-ws.md L104 | {"path":"crates/opi-ai/src/ws.rs","sha256":"c7f416ce886a590fd752ca8113c1fe7f1e7a0536497b3daa028bb95ebbca0254","citation":"L20-L27,L81-L102,L105-L135","observed":"Existing public Up | met |  |
| P20-MECH-WS-012 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-ws.md L106 | {"path":"crates/opi-ai/src/ws.rs","sha256":"c7f416ce886a590fd752ca8113c1fe7f1e7a0536497b3daa028bb95ebbca0254","citation":"L20-L27,L81-L102,L105-L135","observed":"Existing public Up | met |  |
| P20-MECH-WS-013 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-ws.md L106 | {"path":"crates/opi-ai/src/ws.rs","sha256":"c7f416ce886a590fd752ca8113c1fe7f1e7a0536497b3daa028bb95ebbca0254","citation":"L20-L27,L81-L102,L105-L135","observed":"Existing public Up | met |  |
| P20-MECH-WS-014 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-ws.md L106 | {"path":"crates/opi-ai/src/ws.rs","sha256":"c7f416ce886a590fd752ca8113c1fe7f1e7a0536497b3daa028bb95ebbca0254","citation":"L20-L27,L81-L102,L105-L135","observed":"Existing public Up | met |  |
| P20-MECH-WS-015 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-ws.md L108 | {"path":"crates/opi-ai/src/ws.rs","sha256":"c7f416ce886a590fd752ca8113c1fe7f1e7a0536497b3daa028bb95ebbca0254","citation":"L20-L27,L81-L102,L105-L135","observed":"Existing public Up | met |  |
| P20-MECH-WS-016 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-ws.md L108 | {"path":"crates/opi-ai/src/ws.rs","sha256":"c7f416ce886a590fd752ca8113c1fe7f1e7a0536497b3daa028bb95ebbca0254","citation":"L20-L27,L81-L102,L105-L135","observed":"Existing public Up | met |  |
| P20-MECH-WS-017 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-ws.md L108 | {"path":"crates/opi-ai/src/ws.rs","sha256":"c7f416ce886a590fd752ca8113c1fe7f1e7a0536497b3daa028bb95ebbca0254","citation":"L20-L27,L81-L102,L105-L135","observed":"Existing public Up | met |  |
| P20-MECH-WS-018 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-ws.md L108 | {"path":"crates/opi-ai/src/ws.rs","sha256":"c7f416ce886a590fd752ca8113c1fe7f1e7a0536497b3daa028bb95ebbca0254","citation":"L20-L27,L81-L102,L105-L135","observed":"Existing public Up | met |  |
| P20-MECH-WS-019 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-ws.md L108 | {"path":"crates/opi-ai/src/ws.rs","sha256":"c7f416ce886a590fd752ca8113c1fe7f1e7a0536497b3daa028bb95ebbca0254","citation":"L20-L27,L81-L102,L105-L135","observed":"Existing public Up | met |  |
| P20-MECH-WS-020 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-ws.md L108 | {"path":"crates/opi-ai/src/ws.rs","sha256":"c7f416ce886a590fd752ca8113c1fe7f1e7a0536497b3daa028bb95ebbca0254","citation":"L20-L27,L81-L102,L105-L135","observed":"Existing public Up | met |  |
| P20-MECH-WS-021 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-ws.md L110 | {"path":"crates/opi-ai/src/ws.rs","sha256":"c7f416ce886a590fd752ca8113c1fe7f1e7a0536497b3daa028bb95ebbca0254","citation":"L20-L27,L81-L102,L105-L135","observed":"Existing public Up | met |  |
| P20-MECH-WS-022 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-ws.md L110 | {"path":"crates/opi-ai/src/ws.rs","sha256":"c7f416ce886a590fd752ca8113c1fe7f1e7a0536497b3daa028bb95ebbca0254","citation":"L20-L27,L81-L102,L105-L135","observed":"Existing public Up | met |  |
| P20-MECH-WS-023 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-ws.md L110 | {"path":"crates/opi-ai/src/ws.rs","sha256":"c7f416ce886a590fd752ca8113c1fe7f1e7a0536497b3daa028bb95ebbca0254","citation":"L20-L27,L81-L102,L105-L135","observed":"Existing public Up | not-assessable | P20-AUD-018 |
| P20-MECH-WS-024 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-ws.md L110 | {"path":"crates/opi-ai/src/ws.rs","sha256":"c7f416ce886a590fd752ca8113c1fe7f1e7a0536497b3daa028bb95ebbca0254","citation":"L20-L27,L81-L102,L105-L135","observed":"Existing public Up | met |  |
| P20-MECH-WS-025 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-ws.md L110 | {"path":"crates/opi-ai/src/ws.rs","sha256":"c7f416ce886a590fd752ca8113c1fe7f1e7a0536497b3daa028bb95ebbca0254","citation":"L20-L27,L81-L102,L105-L135","observed":"Existing public Up | met |  |
| P20-MECH-WS-026 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-ws.md L112 | {"path":"crates/opi-ai/src/ws.rs","sha256":"c7f416ce886a590fd752ca8113c1fe7f1e7a0536497b3daa028bb95ebbca0254","citation":"L20-L27,L81-L102,L105-L135","observed":"Existing public Up | met |  |
| P20-MECH-WS-027 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-ws.md L112 | {"path":"crates/opi-ai/src/ws.rs","sha256":"c7f416ce886a590fd752ca8113c1fe7f1e7a0536497b3daa028bb95ebbca0254","citation":"L20-L27,L81-L102,L105-L135","observed":"Existing public Up | met |  |
| P20-MECH-WS-028 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-ws.md L116 | {"path":"crates/opi-ai/src/ws.rs","sha256":"c7f416ce886a590fd752ca8113c1fe7f1e7a0536497b3daa028bb95ebbca0254","citation":"L20-L27,L81-L102,L105-L135","observed":"Existing public Up | met |  |
| P20-MECH-WS-029 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-ws.md L116,L132 | {"path":"crates/opi-ai/src/ws.rs","sha256":"c7f416ce886a590fd752ca8113c1fe7f1e7a0536497b3daa028bb95ebbca0254","citation":"L20-L27,L81-L102,L105-L135","observed":"Existing public Up | not-assessable | P20-AUD-018 |
| P20-MECH-WS-030 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-ws.md L118 | {"path":"crates/opi-ai/src/ws.rs","sha256":"c7f416ce886a590fd752ca8113c1fe7f1e7a0536497b3daa028bb95ebbca0254","citation":"L20-L27,L81-L102,L105-L135","observed":"Existing public Up | met |  |
| P20-MECH-WS-031 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-ws.md L118 | {"path":"crates/opi-ai/src/ws.rs","sha256":"c7f416ce886a590fd752ca8113c1fe7f1e7a0536497b3daa028bb95ebbca0254","citation":"L20-L27,L81-L102,L105-L135","observed":"Existing public Up | met |  |
| P20-MECH-WS-032 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-ws.md L118 | {"path":"crates/opi-ai/src/ws.rs","sha256":"c7f416ce886a590fd752ca8113c1fe7f1e7a0536497b3daa028bb95ebbca0254","citation":"L20-L27,L81-L102,L105-L135","observed":"Existing public Up | met |  |
| P20-MECH-WS-033 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-ws.md L120 | {"path":"crates/opi-ai/src/ws.rs","sha256":"c7f416ce886a590fd752ca8113c1fe7f1e7a0536497b3daa028bb95ebbca0254","citation":"L20-L27,L81-L102,L105-L135","observed":"Existing public Up | not-assessable | P20-AUD-018 |
| P20-MECH-WS-034 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-ws.md L120 | {"path":"crates/opi-ai/src/ws.rs","sha256":"c7f416ce886a590fd752ca8113c1fe7f1e7a0536497b3daa028bb95ebbca0254","citation":"L20-L27,L81-L102,L105-L135","observed":"Existing public Up | met |  |
| P20-MECH-WS-035 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-ws.md L122 | {"path":"crates/opi-ai/src/ws.rs","sha256":"c7f416ce886a590fd752ca8113c1fe7f1e7a0536497b3daa028bb95ebbca0254","citation":"L20-L27,L81-L102,L105-L135","observed":"Existing public Up | met |  |
| P20-MECH-WS-036 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-ws.md L122,L132 | {"path":"crates/opi-ai/src/ws.rs","sha256":"c7f416ce886a590fd752ca8113c1fe7f1e7a0536497b3daa028bb95ebbca0254","citation":"L20-L27,L81-L102,L105-L135","observed":"Existing public Up | not-assessable | P20-AUD-018 |
| P20-MECH-WS-037 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-ws.md L122 | {"path":"crates/opi-ai/src/ws.rs","sha256":"c7f416ce886a590fd752ca8113c1fe7f1e7a0536497b3daa028bb95ebbca0254","citation":"L20-L27,L81-L102,L105-L135","observed":"Existing public Up | met |  |
| P20-MECH-WS-038 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-ws.md L126 | {"path":"crates/opi-ai/src/ws.rs","sha256":"c7f416ce886a590fd752ca8113c1fe7f1e7a0536497b3daa028bb95ebbca0254","citation":"L20-L27,L81-L102,L105-L135","observed":"Existing public Up | met |  |
| P20-MECH-WS-039 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-ws.md L130 | {"path":"crates/opi-ai/src/ws.rs","sha256":"c7f416ce886a590fd752ca8113c1fe7f1e7a0536497b3daa028bb95ebbca0254","citation":"L20-L27,L81-L102,L105-L135","observed":"Existing public Up | not-assessable | P20-AUD-018 |
| P20-MECH-WS-040 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-ws.md L130 | {"path":"crates/opi-ai/src/ws.rs","sha256":"c7f416ce886a590fd752ca8113c1fe7f1e7a0536497b3daa028bb95ebbca0254","citation":"L20-L27,L81-L102,L105-L135","observed":"Existing public Up | not-assessable | P20-AUD-018 |
| P20-MECH-WS-041 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-ws.md L134 | {"path":"crates/opi-ai/src/ws.rs","sha256":"c7f416ce886a590fd752ca8113c1fe7f1e7a0536497b3daa028bb95ebbca0254","citation":"L20-L27,L81-L102,L105-L135","observed":"Existing public Up | not-assessable | P20-AUD-018 |
| P20-MECH-HTTP-001 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-transport-auth.md L11,L37 | {"path":"crates/opi-ai/src/http.rs","sha256":"9c836d8f660f6fd69a0cd4706c4c01f47bd1827c17256566b4ebd4a6a23f4d7c","citation":"L35-L70,L370-L410","observed":"Builder delegates source  | not-met | P20-AUD-002 |
| P20-MECH-HTTP-002 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-transport-auth.md L37 | {"path":"crates/opi-ai/src/http.rs","sha256":"9c836d8f660f6fd69a0cd4706c4c01f47bd1827c17256566b4ebd4a6a23f4d7c","citation":"L35-L70,L370-L410","observed":"Builder delegates source  | not-met | P20-AUD-002 |
| P20-MECH-HTTP-003 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-transport-auth.md L37 | {"path":"crates/opi-ai/src/http.rs","sha256":"9c836d8f660f6fd69a0cd4706c4c01f47bd1827c17256566b4ebd4a6a23f4d7c","citation":"L35-L70,L370-L410","observed":"Builder delegates source  | not-assessable | P20-AUD-018 |
| P20-MECH-HTTP-004 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-transport-auth.md L39 | {"path":"crates/opi-ai/src/http.rs","sha256":"9c836d8f660f6fd69a0cd4706c4c01f47bd1827c17256566b4ebd4a6a23f4d7c","citation":"L35-L70,L370-L410","observed":"Builder delegates source  | not-met | P20-AUD-010 |
| P20-MECH-HTTP-005 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-transport-auth.md L39 | {"path":"crates/opi-ai/src/http.rs","sha256":"9c836d8f660f6fd69a0cd4706c4c01f47bd1827c17256566b4ebd4a6a23f4d7c","citation":"L35-L70,L370-L410","observed":"Builder delegates source  | not-assessable | P20-AUD-018 |
| P20-MECH-HTTP-006 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-transport-auth.md L47 | {"path":"crates/opi-ai/src/http.rs","sha256":"9c836d8f660f6fd69a0cd4706c4c01f47bd1827c17256566b4ebd4a6a23f4d7c","citation":"L35-L70,L370-L410","observed":"Builder delegates source  | not-assessable | P20-AUD-018 |
| P20-MECH-HTTP-007 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-transport-auth.md L47 | {"path":"crates/opi-ai/src/http.rs","sha256":"9c836d8f660f6fd69a0cd4706c4c01f47bd1827c17256566b4ebd4a6a23f4d7c","citation":"L35-L70,L370-L410","observed":"Builder delegates source  | not-assessable | P20-AUD-018 |
| P20-MECH-HTTP-008 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-transport-auth.md L47 | {"path":"crates/opi-ai/src/http.rs","sha256":"9c836d8f660f6fd69a0cd4706c4c01f47bd1827c17256566b4ebd4a6a23f4d7c","citation":"L35-L70,L370-L410","observed":"Builder delegates source  | not-assessable | P20-AUD-018 |
| P20-MECH-HTTP-009 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-transport-auth.md L47 | {"path":"crates/opi-ai/src/http.rs","sha256":"9c836d8f660f6fd69a0cd4706c4c01f47bd1827c17256566b4ebd4a6a23f4d7c","citation":"L35-L70,L370-L410","observed":"Builder delegates source  | not-assessable | P20-AUD-018 |
| P20-MECH-HTTP-010 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-transport-auth.md L51 | {"path":"crates/opi-ai/src/http.rs","sha256":"9c836d8f660f6fd69a0cd4706c4c01f47bd1827c17256566b4ebd4a6a23f4d7c","citation":"L35-L70,L370-L410","observed":"Builder delegates source  | not-assessable | P20-AUD-018 |
| P20-MECH-TLS-001 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-transport-auth.md L61,L87 | {"path":"crates/opi-ai/src/tls/mod.rs","sha256":"ba96a12a89ae10faeefc42385d37277eefee10bbc1a406e7ad0e22ad029b3090","citation":"L193-L269,L297-L380","observed":"Explicit pinned trus | met |  |
| P20-MECH-TLS-002 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-transport-auth.md L61 | {"path":"crates/opi-ai/src/tls/mod.rs","sha256":"ba96a12a89ae10faeefc42385d37277eefee10bbc1a406e7ad0e22ad029b3090","citation":"L193-L269,L297-L380","observed":"Explicit pinned trus | met |  |
| P20-MECH-TLS-003 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-transport-auth.md L63,L87 | {"path":"crates/opi-ai/src/tls/mod.rs","sha256":"ba96a12a89ae10faeefc42385d37277eefee10bbc1a406e7ad0e22ad029b3090","citation":"L193-L269,L297-L380","observed":"Explicit pinned trus | met |  |
| P20-MECH-TLS-004 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-transport-auth.md L63 | {"path":"crates/opi-ai/src/tls/mod.rs","sha256":"ba96a12a89ae10faeefc42385d37277eefee10bbc1a406e7ad0e22ad029b3090","citation":"L193-L269,L297-L380","observed":"Explicit pinned trus | met |  |
| P20-MECH-TLS-005 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-transport-auth.md L65 | {"path":"crates/opi-ai/src/tls/mod.rs","sha256":"ba96a12a89ae10faeefc42385d37277eefee10bbc1a406e7ad0e22ad029b3090","citation":"L193-L269,L297-L380","observed":"Explicit pinned trus | met |  |
| P20-MECH-TLS-006 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-transport-auth.md L65 | {"path":"crates/opi-ai/src/tls/mod.rs","sha256":"ba96a12a89ae10faeefc42385d37277eefee10bbc1a406e7ad0e22ad029b3090","citation":"L193-L269,L297-L380","observed":"Explicit pinned trus | not-assessable | P20-AUD-018 |
| P20-MECH-TLS-007 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-transport-auth.md L65 | {"path":"crates/opi-ai/src/tls/mod.rs","sha256":"ba96a12a89ae10faeefc42385d37277eefee10bbc1a406e7ad0e22ad029b3090","citation":"L193-L269,L297-L380","observed":"Explicit pinned trus | met |  |
| P20-MECH-TLS-008 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-transport-auth.md L89 | {"path":"crates/opi-ai/src/tls/mod.rs","sha256":"ba96a12a89ae10faeefc42385d37277eefee10bbc1a406e7ad0e22ad029b3090","citation":"L193-L269,L297-L380","observed":"Explicit pinned trus | met |  |
| P20-MECH-TLS-009 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-transport-auth.md L89 | {"path":"crates/opi-ai/src/tls/mod.rs","sha256":"ba96a12a89ae10faeefc42385d37277eefee10bbc1a406e7ad0e22ad029b3090","citation":"L193-L269,L297-L380","observed":"Explicit pinned trus | met |  |
| P20-MECH-TLS-010 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-transport-auth.md L89 | {"path":"crates/opi-ai/src/tls/mod.rs","sha256":"ba96a12a89ae10faeefc42385d37277eefee10bbc1a406e7ad0e22ad029b3090","citation":"L193-L269,L297-L380","observed":"Explicit pinned trus | met |  |
| P20-MECH-TLS-011 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-transport-auth.md L89; SDK token-to-Bearer mechanism, not historical L97 nonexecution list | {"path":"crates/opi-ai/src/tls/mod.rs","sha256":"ba96a12a89ae10faeefc42385d37277eefee10bbc1a406e7ad0e22ad029b3090","citation":"L193-L269,L297-L380","observed":"Explicit pinned trus | met |  |
| P20-MECH-TLS-012 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-transport-auth.md L91 | {"path":"crates/opi-ai/src/tls/mod.rs","sha256":"ba96a12a89ae10faeefc42385d37277eefee10bbc1a406e7ad0e22ad029b3090","citation":"L193-L269,L297-L380","observed":"Explicit pinned trus | met |  |
| P20-MECH-TLS-013 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-transport-auth.md L93 | {"path":"crates/opi-ai/src/tls/mod.rs","sha256":"ba96a12a89ae10faeefc42385d37277eefee10bbc1a406e7ad0e22ad029b3090","citation":"L193-L269,L297-L380","observed":"Explicit pinned trus | not-assessable | P20-AUD-018 |
| P20-MECH-TLS-014 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-transport-auth.md L93 | {"path":"crates/opi-ai/src/tls/mod.rs","sha256":"ba96a12a89ae10faeefc42385d37277eefee10bbc1a406e7ad0e22ad029b3090","citation":"L193-L269,L297-L380","observed":"Explicit pinned trus | not-assessable | P20-AUD-018 |
| P20-MECH-TLS-015 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-transport-auth.md L95 | {"path":"crates/opi-ai/src/tls/mod.rs","sha256":"ba96a12a89ae10faeefc42385d37277eefee10bbc1a406e7ad0e22ad029b3090","citation":"L193-L269,L297-L380","observed":"Explicit pinned trus | met |  |
| P20-MECH-TLS-016 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-transport-auth.md L95 | {"path":"crates/opi-ai/src/tls/mod.rs","sha256":"ba96a12a89ae10faeefc42385d37277eefee10bbc1a406e7ad0e22ad029b3090","citation":"L193-L269,L297-L380","observed":"Explicit pinned trus | met |  |
| P20-MECH-AUTH-001 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-transport-auth.md L13,L101 | {"path":"crates/opi-ai/src/auth/builtin_aws.rs","sha256":"4017f7c14c83ddcae5348b825b8ea6ce3e3b70c0056c43269e1010e1e3ea947a","citation":"L57-L82,L161-L188,L229-L318","observed":"Bui | not-assessable | P20-AUD-018 |
| P20-MECH-AUTH-002 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-transport-auth.md L101 | {"path":"crates/opi-ai/src/auth/builtin_aws.rs","sha256":"4017f7c14c83ddcae5348b825b8ea6ce3e3b70c0056c43269e1010e1e3ea947a","citation":"L57-L82,L161-L188,L229-L318","observed":"Bui | not-assessable | P20-AUD-018 |
| P20-MECH-AUTH-003 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-transport-auth.md L105 | {"path":"crates/opi-ai/src/auth/builtin_aws.rs","sha256":"4017f7c14c83ddcae5348b825b8ea6ce3e3b70c0056c43269e1010e1e3ea947a","citation":"L57-L82,L161-L188,L229-L318","observed":"Bui | not-assessable | P20-AUD-018 |
| P20-MECH-AUTH-004 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-transport-auth.md L105 | {"path":"crates/opi-ai/src/auth/store_oauth.rs","sha256":"ede148b6ccf172e1ffc26fc2ff75848f56cb7da03e101863b093fb7c2381a55a","citation":"refresh/store mutation owner","observed":"Ow | met |  |
| P20-MECH-AUTH-005 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-transport-auth.md L105 | {"path":"crates/opi-ai/src/auth/builtin_aws.rs","sha256":"4017f7c14c83ddcae5348b825b8ea6ce3e3b70c0056c43269e1010e1e3ea947a","citation":"L57-L82,L161-L188,L229-L318","observed":"Bui | met |  |
| P20-MECH-AUTH-006 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-transport-auth.md L106 | {"path":"crates/opi-ai/src/auth/builtin_aws.rs","sha256":"4017f7c14c83ddcae5348b825b8ea6ce3e3b70c0056c43269e1010e1e3ea947a","citation":"L57-L82,L161-L188,L229-L318","observed":"Bui | met |  |
| P20-MECH-AUTH-007 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-transport-auth.md L106 | {"path":"crates/opi-ai/src/auth/builtin_aws.rs","sha256":"4017f7c14c83ddcae5348b825b8ea6ce3e3b70c0056c43269e1010e1e3ea947a","citation":"L57-L82,L161-L188,L229-L318","observed":"Bui | not-assessable | P20-AUD-018 |
| P20-MECH-AUTH-008 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-transport-auth.md L107 | {"path":"crates/opi-ai/src/auth/builtin_aws.rs","sha256":"4017f7c14c83ddcae5348b825b8ea6ce3e3b70c0056c43269e1010e1e3ea947a","citation":"L57-L82,L161-L188,L229-L318","observed":"Bui | met |  |
| P20-MECH-AUTH-009 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-transport-auth.md L107 | {"path":"crates/opi-ai/src/auth/builtin_aws.rs","sha256":"4017f7c14c83ddcae5348b825b8ea6ce3e3b70c0056c43269e1010e1e3ea947a","citation":"L57-L82,L161-L188,L229-L318","observed":"Bui | not-assessable | P20-AUD-018 |
| P20-MECH-AUTH-010 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-transport-auth.md L108 | {"path":"crates/opi-ai/src/auth/builtin_aws.rs","sha256":"4017f7c14c83ddcae5348b825b8ea6ce3e3b70c0056c43269e1010e1e3ea947a","citation":"L57-L82,L161-L188,L229-L318","observed":"Bui | not-assessable | P20-AUD-018 |
| P20-MECH-AUTH-011 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-transport-auth.md L108 | {"path":"crates/opi-ai/src/auth/builtin_aws.rs","sha256":"4017f7c14c83ddcae5348b825b8ea6ce3e3b70c0056c43269e1010e1e3ea947a","citation":"L57-L82,L161-L188,L229-L318","observed":"Bui | not-assessable | P20-AUD-018 |
| P20-MECH-AUTH-012 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-transport-auth.md L109 | {"path":"crates/opi-ai/src/auth/builtin_aws.rs","sha256":"4017f7c14c83ddcae5348b825b8ea6ce3e3b70c0056c43269e1010e1e3ea947a","citation":"L57-L82,L161-L188,L229-L318","observed":"Bui | met |  |
| P20-MECH-AUTH-013 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-transport-auth.md L109 | {"path":"crates/opi-ai/src/auth/builtin_aws.rs","sha256":"4017f7c14c83ddcae5348b825b8ea6ce3e3b70c0056c43269e1010e1e3ea947a","citation":"L57-L82,L161-L188,L229-L318","observed":"Bui | met |  |
| P20-MECH-AUTH-014 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-transport-auth.md L110 | {"path":"crates/opi-ai/src/auth/builtin_aws.rs","sha256":"4017f7c14c83ddcae5348b825b8ea6ce3e3b70c0056c43269e1010e1e3ea947a","citation":"L57-L82,L161-L188,L229-L318","observed":"Bui | not-assessable | P20-AUD-018 |
| P20-MECH-AUTH-015 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-transport-auth.md L110 | {"path":"crates/opi-ai/src/auth/builtin_aws.rs","sha256":"4017f7c14c83ddcae5348b825b8ea6ce3e3b70c0056c43269e1010e1e3ea947a","citation":"L57-L82,L161-L188,L229-L318","observed":"Bui | not-assessable | P20-AUD-018 |
| P20-MECH-AUTH-016 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-transport-auth.md L111 | {"path":"crates/opi-ai/src/auth/builtin_aws.rs","sha256":"4017f7c14c83ddcae5348b825b8ea6ce3e3b70c0056c43269e1010e1e3ea947a","citation":"L57-L82,L161-L188,L229-L318","observed":"Bui | not-assessable | P20-AUD-018 |
| P20-MECH-AUTH-017 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-transport-auth.md L112 | {"path":"crates/opi-ai/src/auth/builtin_aws.rs","sha256":"4017f7c14c83ddcae5348b825b8ea6ce3e3b70c0056c43269e1010e1e3ea947a","citation":"L57-L82,L161-L188,L229-L318","observed":"Bui | not-assessable | P20-AUD-018 |
| P20-MECH-AUTH-018 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-transport-auth.md L112 | {"path":"crates/opi-ai/src/auth/builtin_aws.rs","sha256":"4017f7c14c83ddcae5348b825b8ea6ce3e3b70c0056c43269e1010e1e3ea947a","citation":"L57-L82,L161-L188,L229-L318","observed":"Bui | not-assessable | P20-AUD-018 |
| P20-MECH-AUTH-019 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-transport-auth.md L113 | {"path":"crates/opi-ai/src/auth/builtin_aws.rs","sha256":"4017f7c14c83ddcae5348b825b8ea6ce3e3b70c0056c43269e1010e1e3ea947a","citation":"L57-L82,L161-L188,L229-L318","observed":"Bui | not-met | P20-AUD-002 |
| P20-MECH-AUTH-020 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-transport-auth.md L113 | {"path":"crates/opi-ai/src/auth/builtin_aws.rs","sha256":"4017f7c14c83ddcae5348b825b8ea6ce3e3b70c0056c43269e1010e1e3ea947a","citation":"L57-L82,L161-L188,L229-L318","observed":"Bui | not-assessable | P20-AUD-018 |
| P20-MECH-AUTH-021 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-transport-auth.md L114 | {"path":"crates/opi-ai/src/auth/builtin_aws.rs","sha256":"4017f7c14c83ddcae5348b825b8ea6ce3e3b70c0056c43269e1010e1e3ea947a","citation":"L57-L82,L161-L188,L229-L318","observed":"Bui | not-assessable | P20-AUD-018 |
| P20-MECH-AUTH-022 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-transport-auth.md L115 | {"path":"crates/opi-ai/src/auth/builtin_aws.rs","sha256":"4017f7c14c83ddcae5348b825b8ea6ce3e3b70c0056c43269e1010e1e3ea947a","citation":"L57-L82,L161-L188,L229-L318","observed":"Bui | not-assessable | P20-AUD-018 |
| P20-MECH-AUTH-023 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-transport-auth.md L115 | {"path":"crates/opi-ai/src/auth/builtin_aws.rs","sha256":"4017f7c14c83ddcae5348b825b8ea6ce3e3b70c0056c43269e1010e1e3ea947a","citation":"L57-L82,L161-L188,L229-L318","observed":"Bui | not-assessable | P20-AUD-018 |
| P20-MECH-AUTH-024 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-transport-auth.md L116,L126 | {"path":"crates/opi-ai/src/auth/builtin_aws.rs","sha256":"4017f7c14c83ddcae5348b825b8ea6ce3e3b70c0056c43269e1010e1e3ea947a","citation":"L57-L82,L161-L188,L229-L318","observed":"Bui | not-assessable | P20-AUD-018 |
| P20-MECH-AUTH-025 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-transport-auth.md L122,L126 | {"path":"crates/opi-ai/src/auth/builtin_aws.rs","sha256":"4017f7c14c83ddcae5348b825b8ea6ce3e3b70c0056c43269e1010e1e3ea947a","citation":"L57-L82,L161-L188,L229-L318","observed":"Bui | not-assessable | P20-AUD-018 |
| P20-MECH-AUTH-026 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-transport-auth.md L124,L126 | {"path":"crates/opi-ai/src/auth/builtin_aws.rs","sha256":"4017f7c14c83ddcae5348b825b8ea6ce3e3b70c0056c43269e1010e1e3ea947a","citation":"L57-L82,L161-L188,L229-L318","observed":"Bui | not-assessable | P20-AUD-018 |
| P20-MECH-TELEMETRY-001 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-runtime.md L13 | {"path":"crates/opi-ai/src/telemetry.rs","sha256":"9de4f0925ca6717eef05fe3fc78ec354b0be9507cdc937d4128ba8ca5bd97503","citation":"L66-L120,L131-L155","observed":"Frozen source Span+ | met |  |
| P20-MECH-TELEMETRY-002 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-runtime.md L13 | {"path":"crates/opi-ai/src/telemetry.rs","sha256":"9de4f0925ca6717eef05fe3fc78ec354b0be9507cdc937d4128ba8ca5bd97503","citation":"L66-L120,L131-L155","observed":"Frozen source Span+ | met |  |
| P20-MECH-TELEMETRY-003 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-runtime.md L13 | {"path":"crates/opi-ai/src/telemetry.rs","sha256":"9de4f0925ca6717eef05fe3fc78ec354b0be9507cdc937d4128ba8ca5bd97503","citation":"L66-L120,L131-L155","observed":"Frozen source Span+ | met |  |
| P20-MECH-TELEMETRY-004 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-runtime.md L17-L23 | {"path":"crates/opi-ai/src/telemetry.rs","sha256":"9de4f0925ca6717eef05fe3fc78ec354b0be9507cdc937d4128ba8ca5bd97503","citation":"L66-L120,L131-L155","observed":"Frozen source Span+ | met |  |
| P20-MECH-TELEMETRY-005 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-runtime.md L20-L23 | {"path":"crates/opi-ai/src/telemetry.rs","sha256":"9de4f0925ca6717eef05fe3fc78ec354b0be9507cdc937d4128ba8ca5bd97503","citation":"L66-L120,L131-L155","observed":"Frozen source Span+ | met |  |
| P20-MECH-TELEMETRY-006 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-runtime.md L23 | {"path":"crates/opi-ai/src/telemetry.rs","sha256":"9de4f0925ca6717eef05fe3fc78ec354b0be9507cdc937d4128ba8ca5bd97503","citation":"L66-L120,L131-L155","observed":"Frozen source Span+ | met |  |
| P20-MECH-TELEMETRY-007 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-runtime.md L23 | {"path":"crates/opi-ai/src/telemetry.rs","sha256":"9de4f0925ca6717eef05fe3fc78ec354b0be9507cdc937d4128ba8ca5bd97503","citation":"L66-L120,L131-L155","observed":"Frozen source Span+ | met |  |
| P20-MECH-DELIVERY-001 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-runtime.md L29 | {"path":"Cargo.toml","sha256":"11a2d459067c679bc06a1bc7af5fe67f4e72b9d856d34d2da83241ac0bb47f83","citation":"L2-L16,L36-L60,L129-L140","observed":"Root declares existing members, M | not-assessable | P20-AUD-018 |
| P20-MECH-DELIVERY-002 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-runtime.md L29,L33 | {"path":"Cargo.toml","sha256":"11a2d459067c679bc06a1bc7af5fe67f4e72b9d856d34d2da83241ac0bb47f83","citation":"L2-L16,L36-L60,L129-L140","observed":"Root declares existing members, M | not-assessable | P20-AUD-018 |
| P20-MECH-DELIVERY-003 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-runtime.md L31,L33 | {"path":"Cargo.toml","sha256":"11a2d459067c679bc06a1bc7af5fe67f4e72b9d856d34d2da83241ac0bb47f83","citation":"L2-L16,L36-L60,L129-L140","observed":"Root declares existing members, M | not-assessable | P20-AUD-018 |
| P20-MECH-DELIVERY-004 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-runtime.md L33 | {"path":"Cargo.toml","sha256":"11a2d459067c679bc06a1bc7af5fe67f4e72b9d856d34d2da83241ac0bb47f83","citation":"L2-L16,L36-L60,L129-L140","observed":"Root declares existing members, M | not-assessable | P20-AUD-018 |
| P20-MECH-DELIVERY-005 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-runtime.md L39 | {"path":"Cargo.toml","sha256":"11a2d459067c679bc06a1bc7af5fe67f4e72b9d856d34d2da83241ac0bb47f83","citation":"L2-L16,L36-L60,L129-L140","observed":"Root declares existing members, M | not-assessable | P20-AUD-018 |
| P20-MECH-DELIVERY-006 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-runtime.md L39 | {"path":"Cargo.toml","sha256":"11a2d459067c679bc06a1bc7af5fe67f4e72b9d856d34d2da83241ac0bb47f83","citation":"L2-L16,L36-L60,L129-L140","observed":"Root declares existing members, M | not-assessable | P20-AUD-018 |
| P20-MECH-DELIVERY-007 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-runtime.md L39 | {"path":"Cargo.toml","sha256":"11a2d459067c679bc06a1bc7af5fe67f4e72b9d856d34d2da83241ac0bb47f83","citation":"L2-L16,L36-L60,L129-L140","observed":"Root declares existing members, M | not-assessable | P20-AUD-018 |
| P20-MECH-DELIVERY-008 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-runtime.md L43 | {"path":"Cargo.toml","sha256":"11a2d459067c679bc06a1bc7af5fe67f4e72b9d856d34d2da83241ac0bb47f83","citation":"L2-L16,L36-L60,L129-L140","observed":"Root declares existing members, M | not-assessable | P20-AUD-018 |
| P20-MECH-DELIVERY-009 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-runtime.md L43,L45 | {"path":"Cargo.toml","sha256":"11a2d459067c679bc06a1bc7af5fe67f4e72b9d856d34d2da83241ac0bb47f83","citation":"L2-L16,L36-L60,L129-L140","observed":"Root declares existing members, M | not-assessable | P20-AUD-018 |
| P20-MECH-DESIGN-001 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-design-readiness.md L19,L35,L43,L47 | {"path":"Cargo.toml","sha256":"11a2d459067c679bc06a1bc7af5fe67f4e72b9d856d34d2da83241ac0bb47f83","citation":"L2-L16,L47-L60,L129-L140","observed":"Current topology remains seven me | not-assessable | P20-AUD-018 |
| P20-MECH-DESIGN-002 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-design-readiness.md L39,L41 | {"path":"Cargo.toml","sha256":"11a2d459067c679bc06a1bc7af5fe67f4e72b9d856d34d2da83241ac0bb47f83","citation":"L2-L16,L47-L60,L129-L140","observed":"Current topology remains seven me | not-assessable | P20-AUD-018 |
| P20-MECH-DESIGN-003 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-design-readiness.md L43,L45 | {"path":"Cargo.toml","sha256":"11a2d459067c679bc06a1bc7af5fe67f4e72b9d856d34d2da83241ac0bb47f83","citation":"L2-L16,L47-L60,L129-L140","observed":"Current topology remains seven me | not-assessable | P20-AUD-018 |
| P20-MECH-DESIGN-004 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-design-readiness.md L47 | {"path":"Cargo.toml","sha256":"11a2d459067c679bc06a1bc7af5fe67f4e72b9d856d34d2da83241ac0bb47f83","citation":"L2-L16,L47-L60,L129-L140","observed":"Current topology remains seven me | not-assessable | P20-AUD-018 |
| P20-MECH-TRANSCRIPT-001 | docs/research/pi-ai-opi-ai-alignment/transcript-target-proposal.md TP-02,L26,L45-L47 | {"path":"crates/opi-ai/src/transcript.rs","sha256":"bfc33852cd44a966cf0df2fa62741b5627b87c5e3866c40a599effceeee3bf95","citation":"L175-L220,L306-L315,L344-L422,L846-L924","observed | met |  |
| P20-MECH-TRANSCRIPT-002 | docs/research/pi-ai-opi-ai-alignment/transcript-target-proposal.md TP-02,L26,L47 | {"path":"crates/opi-ai/src/transcript.rs","sha256":"bfc33852cd44a966cf0df2fa62741b5627b87c5e3866c40a599effceeee3bf95","citation":"L175-L220,L306-L315,L344-L422,L846-L924","observed | met |  |
| P20-MECH-TRANSCRIPT-003 | docs/research/pi-ai-opi-ai-alignment/transcript-target-proposal.md TP-02,L26,L47 | {"path":"crates/opi-ai/src/transcript.rs","sha256":"bfc33852cd44a966cf0df2fa62741b5627b87c5e3866c40a599effceeee3bf95","citation":"L175-L220,L306-L315,L344-L422,L846-L924","observed | met |  |
| P20-MECH-TRANSCRIPT-004 | docs/research/pi-ai-opi-ai-alignment/transcript-target-proposal.md TP-03,L27,L49 | {"path":"crates/opi-ai/src/transcript.rs","sha256":"bfc33852cd44a966cf0df2fa62741b5627b87c5e3866c40a599effceeee3bf95","citation":"L175-L220,L306-L315,L344-L422,L846-L924","observed | met |  |
| P20-MECH-TRANSCRIPT-005 | docs/research/pi-ai-opi-ai-alignment/transcript-target-proposal.md TP-03,L27,L49 | {"path":"crates/opi-ai/src/transcript.rs","sha256":"bfc33852cd44a966cf0df2fa62741b5627b87c5e3866c40a599effceeee3bf95","citation":"L175-L220,L306-L315,L344-L422,L846-L924","observed | met |  |
| P20-MECH-TRANSCRIPT-006 | docs/research/pi-ai-opi-ai-alignment/transcript-target-proposal.md TP-13,L37,L57 | {"path":"crates/opi-ai/src/transcript.rs","sha256":"bfc33852cd44a966cf0df2fa62741b5627b87c5e3866c40a599effceeee3bf95","citation":"L175-L220,L306-L315,L344-L422,L846-L924","observed | met |  |
| P20-MECH-TRANSCRIPT-007 | docs/research/pi-ai-opi-ai-alignment/transcript-target-proposal.md TP-13,L58 | {"path":"crates/opi-ai/src/transcript.rs","sha256":"bfc33852cd44a966cf0df2fa62741b5627b87c5e3866c40a599effceeee3bf95","citation":"L175-L220,L306-L315,L344-L422,L846-L924","observed | met |  |
| P20-MECH-TRANSCRIPT-008 | docs/research/pi-ai-opi-ai-alignment/transcript-target-proposal.md TP-13,L59 | {"path":"crates/opi-ai/src/transcript.rs","sha256":"bfc33852cd44a966cf0df2fa62741b5627b87c5e3866c40a599effceeee3bf95","citation":"L175-L220,L306-L315,L344-L422,L846-L924","observed | met |  |
| P20-MECH-TRANSCRIPT-009 | docs/research/pi-ai-opi-ai-alignment/transcript-target-proposal.md TP-13,L60 | {"path":"crates/opi-ai/src/transcript.rs","sha256":"bfc33852cd44a966cf0df2fa62741b5627b87c5e3866c40a599effceeee3bf95","citation":"L175-L220,L306-L315,L344-L422,L846-L924","observed | met |  |
| P20-MECH-TRANSCRIPT-010 | docs/research/pi-ai-opi-ai-alignment/transcript-target-proposal.md TP-13,L61 | {"path":"crates/opi-ai/src/transcript.rs","sha256":"bfc33852cd44a966cf0df2fa62741b5627b87c5e3866c40a599effceeee3bf95","citation":"L175-L220,L306-L315,L344-L422,L846-L924","observed | not-met | P20-AUD-015 |
| P20-MECH-TRANSCRIPT-011 | docs/research/pi-ai-opi-ai-alignment/transcript-target-proposal.md TP-13,L62 | {"path":"crates/opi-ai/src/transcript.rs","sha256":"bfc33852cd44a966cf0df2fa62741b5627b87c5e3866c40a599effceeee3bf95","citation":"L175-L220,L306-L315,L344-L422,L846-L924","observed | met |  |
| P20-MECH-TRANSCRIPT-012 | docs/research/pi-ai-opi-ai-alignment/transcript-target-proposal.md TP-13,L63 | {"path":"crates/opi-ai/src/transcript.rs","sha256":"bfc33852cd44a966cf0df2fa62741b5627b87c5e3866c40a599effceeee3bf95","citation":"L175-L220,L306-L315,L344-L422,L846-L924","observed | met |  |
| P20-MECH-TRANSCRIPT-013 | docs/research/pi-ai-opi-ai-alignment/transcript-target-proposal.md TP-13,L64 | {"path":"crates/opi-ai/src/transcript.rs","sha256":"bfc33852cd44a966cf0df2fa62741b5627b87c5e3866c40a599effceeee3bf95","citation":"L175-L220,L306-L315,L344-L422,L846-L924","observed | met |  |
| P20-MECH-TRANSCRIPT-014 | docs/research/pi-ai-opi-ai-alignment/transcript-target-proposal.md TP-13,L65,L67 | {"path":"crates/opi-ai/src/transcript.rs","sha256":"bfc33852cd44a966cf0df2fa62741b5627b87c5e3866c40a599effceeee3bf95","citation":"L175-L220,L306-L315,L344-L422,L846-L924","observed | met |  |
| P20-MECH-TRANSCRIPT-015 | docs/research/pi-ai-opi-ai-alignment/transcript-target-proposal.md TP-14,L38,L73 | {"path":"crates/opi-ai/src/transcript.rs","sha256":"bfc33852cd44a966cf0df2fa62741b5627b87c5e3866c40a599effceeee3bf95","citation":"L175-L220,L306-L315,L344-L422,L846-L924","observed | met |  |
| P20-MECH-TRANSCRIPT-016 | docs/research/pi-ai-opi-ai-alignment/transcript-target-proposal.md TP-14,L73 | {"path":"crates/opi-ai/src/transcript.rs","sha256":"bfc33852cd44a966cf0df2fa62741b5627b87c5e3866c40a599effceeee3bf95","citation":"L175-L220,L306-L315,L344-L422,L846-L924","observed | met |  |
| P20-MECH-TRANSCRIPT-017 | docs/research/pi-ai-opi-ai-alignment/transcript-target-proposal.md TP-14,L75 | {"path":"crates/opi-ai/src/transcript.rs","sha256":"bfc33852cd44a966cf0df2fa62741b5627b87c5e3866c40a599effceeee3bf95","citation":"L175-L220,L306-L315,L344-L422,L846-L924","observed | met |  |
| P20-MECH-TRANSCRIPT-018 | docs/research/pi-ai-opi-ai-alignment/transcript-target-proposal.md TP-14,L75 | {"path":"crates/opi-ai/src/transcript.rs","sha256":"bfc33852cd44a966cf0df2fa62741b5627b87c5e3866c40a599effceeee3bf95","citation":"L175-L220,L306-L315,L344-L422,L846-L924","observed | met |  |
| P20-MECH-TRANSCRIPT-019 | docs/research/pi-ai-opi-ai-alignment/transcript-target-proposal.md TP-14,L38,L77 | {"path":"crates/opi-ai/src/transcript.rs","sha256":"bfc33852cd44a966cf0df2fa62741b5627b87c5e3866c40a599effceeee3bf95","citation":"L175-L220,L306-L315,L344-L422,L846-L924","observed | met |  |
| P20-MECH-REPLAY-001 | docs/research/pi-ai-opi-ai-alignment/replay-contracts.md RP-01,L104 | {"path":"crates/opi-ai/src/replay.rs","sha256":"522ee27e143767e0722dc0ff572e674f4abc78aa00ce209b52c017d8862e95f8","citation":"L210-L214,L280-L367,L379-L465,L467-L535,L627-L778","ob | met |  |
| P20-MECH-REPLAY-002 | docs/research/pi-ai-opi-ai-alignment/replay-contracts.md RP-02,L105 | {"path":"crates/opi-ai/src/replay.rs","sha256":"522ee27e143767e0722dc0ff572e674f4abc78aa00ce209b52c017d8862e95f8","citation":"L210-L214,L280-L367,L379-L465,L467-L535,L627-L778","ob | met |  |
| P20-MECH-REPLAY-003 | docs/research/pi-ai-opi-ai-alignment/replay-contracts.md RP-02,L105 | {"path":"crates/opi-ai/src/replay.rs","sha256":"522ee27e143767e0722dc0ff572e674f4abc78aa00ce209b52c017d8862e95f8","citation":"L210-L214,L280-L367,L379-L465,L467-L535,L627-L778","ob | met |  |
| P20-MECH-REPLAY-004 | docs/research/pi-ai-opi-ai-alignment/replay-contracts.md RP-04,L107 | {"path":"crates/opi-ai/src/replay.rs","sha256":"522ee27e143767e0722dc0ff572e674f4abc78aa00ce209b52c017d8862e95f8","citation":"L210-L214,L280-L367,L379-L465,L467-L535,L627-L778","ob | met |  |
| P20-MECH-REPLAY-005 | docs/research/pi-ai-opi-ai-alignment/replay-contracts.md RP-04,L107 | {"path":"crates/opi-ai/src/replay.rs","sha256":"522ee27e143767e0722dc0ff572e674f4abc78aa00ce209b52c017d8862e95f8","citation":"L210-L214,L280-L367,L379-L465,L467-L535,L627-L778","ob | met |  |
| P20-MECH-REPLAY-006 | docs/research/pi-ai-opi-ai-alignment/replay-contracts.md RP-05,L108 | {"path":"crates/opi-ai/src/replay.rs","sha256":"522ee27e143767e0722dc0ff572e674f4abc78aa00ce209b52c017d8862e95f8","citation":"L210-L214,L280-L367,L379-L465,L467-L535,L627-L778","ob | met |  |
| P20-MECH-REPLAY-007 | docs/research/pi-ai-opi-ai-alignment/replay-contracts.md RP-06,L109 | {"path":"crates/opi-ai/src/replay.rs","sha256":"522ee27e143767e0722dc0ff572e674f4abc78aa00ce209b52c017d8862e95f8","citation":"L210-L214,L280-L367,L379-L465,L467-L535,L627-L778","ob | met |  |
| P20-MECH-REPLAY-008 | docs/research/pi-ai-opi-ai-alignment/replay-contracts.md RP-06,L109 | {"path":"crates/opi-ai/src/replay.rs","sha256":"522ee27e143767e0722dc0ff572e674f4abc78aa00ce209b52c017d8862e95f8","citation":"L210-L214,L280-L367,L379-L465,L467-L535,L627-L778","ob | met |  |
| P20-MECH-REPLAY-009 | docs/research/pi-ai-opi-ai-alignment/replay-contracts.md RP-07,L110 | {"path":"crates/opi-ai/src/replay.rs","sha256":"522ee27e143767e0722dc0ff572e674f4abc78aa00ce209b52c017d8862e95f8","citation":"L210-L214,L280-L367,L379-L465,L467-L535,L627-L778","ob | met |  |
| P20-MECH-REPLAY-010 | docs/research/pi-ai-opi-ai-alignment/replay-contracts.md RP-08,L111,L116 | {"path":"crates/opi-ai/src/replay.rs","sha256":"522ee27e143767e0722dc0ff572e674f4abc78aa00ce209b52c017d8862e95f8","citation":"L210-L214,L280-L367,L379-L465,L467-L535,L627-L778","ob | met |  |
| P20-MECH-REPLAY-011 | docs/research/pi-ai-opi-ai-alignment/replay-contracts.md RP-08,L111,L116 | {"path":"crates/opi-ai/src/replay.rs","sha256":"522ee27e143767e0722dc0ff572e674f4abc78aa00ce209b52c017d8862e95f8","citation":"L210-L214,L280-L367,L379-L465,L467-L535,L627-L778","ob | met |  |
| P20-MECH-REPLAY-012 | docs/research/pi-ai-opi-ai-alignment/replay-contracts.md RP-09,L112,L116 | {"path":"crates/opi-ai/src/replay.rs","sha256":"522ee27e143767e0722dc0ff572e674f4abc78aa00ce209b52c017d8862e95f8","citation":"L210-L214,L280-L367,L379-L465,L467-L535,L627-L778","ob | met |  |
| P20-MECH-REPLAY-013 | docs/research/pi-ai-opi-ai-alignment/replay-contracts.md RP-09,L112 | {"path":"crates/opi-ai/src/replay.rs","sha256":"522ee27e143767e0722dc0ff572e674f4abc78aa00ce209b52c017d8862e95f8","citation":"L210-L214,L280-L367,L379-L465,L467-L535,L627-L778","ob | met |  |
| P20-MECH-REPLAY-014 | docs/research/pi-ai-opi-ai-alignment/replay-contracts.md RP-10,L113,L122 | {"path":"crates/opi-ai/src/replay.rs","sha256":"522ee27e143767e0722dc0ff572e674f4abc78aa00ce209b52c017d8862e95f8","citation":"L210-L214,L280-L367,L379-L465,L467-L535,L627-L778","ob | met |  |
| P20-MECH-REPLAY-015 | docs/research/pi-ai-opi-ai-alignment/replay-contracts.md RP-11,L114,L118 | {"path":"crates/opi-ai/src/replay.rs","sha256":"522ee27e143767e0722dc0ff572e674f4abc78aa00ce209b52c017d8862e95f8","citation":"L210-L214,L280-L367,L379-L465,L467-L535,L627-L778","ob | met |  |
| P20-MECH-REPLAY-016 | docs/research/pi-ai-opi-ai-alignment/replay-contracts.md RP-11,L114,L148 | {"path":"crates/opi-ai/src/replay.rs","sha256":"522ee27e143767e0722dc0ff572e674f4abc78aa00ce209b52c017d8862e95f8","citation":"L210-L214,L280-L367,L379-L465,L467-L535,L627-L778","ob | not-assessable | P20-AUD-018 |
| P20-MECH-SCHEMA-037 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-schema.md L48; TypeBox JSON mechanism: boolean schema | {"path":"crates/opi-ai/src/schema/mod.rs","sha256":"816f780293f0426f0e063fae63111baf6a050d60999d99c1bad52be204d8af70","citation":"L138-L175","observed":"Public compile/check enter  | met |  |
| P20-MECH-SCHEMA-038 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-schema.md L48; TypeBox JSON mechanism: type | {"path":"crates/opi-ai/src/schema/mod.rs","sha256":"816f780293f0426f0e063fae63111baf6a050d60999d99c1bad52be204d8af70","citation":"L138-L175","observed":"Public compile/check enter  | met |  |
| P20-MECH-SCHEMA-039 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-schema.md L48; TypeBox JSON mechanism: const | {"path":"crates/opi-ai/src/schema/mod.rs","sha256":"816f780293f0426f0e063fae63111baf6a050d60999d99c1bad52be204d8af70","citation":"L138-L175","observed":"Public compile/check enter  | met |  |
| P20-MECH-SCHEMA-040 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-schema.md L48; TypeBox JSON mechanism: enum | {"path":"crates/opi-ai/src/schema/mod.rs","sha256":"816f780293f0426f0e063fae63111baf6a050d60999d99c1bad52be204d8af70","citation":"L138-L175","observed":"Public compile/check enter  | met |  |
| P20-MECH-SCHEMA-041 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-schema.md L48; TypeBox JSON mechanism: required | {"path":"crates/opi-ai/src/schema/mod.rs","sha256":"816f780293f0426f0e063fae63111baf6a050d60999d99c1bad52be204d8af70","citation":"L138-L175","observed":"Public compile/check enter  | met |  |
| P20-MECH-SCHEMA-042 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-schema.md L48; TypeBox JSON mechanism: additionalProperties | {"path":"crates/opi-ai/src/schema/mod.rs","sha256":"816f780293f0426f0e063fae63111baf6a050d60999d99c1bad52be204d8af70","citation":"L138-L175","observed":"Public compile/check enter  | met |  |
| P20-MECH-SCHEMA-043 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-schema.md L48; TypeBox JSON mechanism: dependencies | {"path":"crates/opi-ai/src/schema/mod.rs","sha256":"816f780293f0426f0e063fae63111baf6a050d60999d99c1bad52be204d8af70","citation":"L138-L175","observed":"Public compile/check enter  | met |  |
| P20-MECH-SCHEMA-044 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-schema.md L48; TypeBox JSON mechanism: dependentRequired | {"path":"crates/opi-ai/src/schema/mod.rs","sha256":"816f780293f0426f0e063fae63111baf6a050d60999d99c1bad52be204d8af70","citation":"L138-L175","observed":"Public compile/check enter  | met |  |
| P20-MECH-SCHEMA-045 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-schema.md L48; TypeBox JSON mechanism: dependentSchemas | {"path":"crates/opi-ai/src/schema/mod.rs","sha256":"816f780293f0426f0e063fae63111baf6a050d60999d99c1bad52be204d8af70","citation":"L138-L175","observed":"Public compile/check enter  | met |  |
| P20-MECH-SCHEMA-046 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-schema.md L48; TypeBox JSON mechanism: patternProperties | {"path":"crates/opi-ai/src/schema/mod.rs","sha256":"816f780293f0426f0e063fae63111baf6a050d60999d99c1bad52be204d8af70","citation":"L138-L175","observed":"Public compile/check enter  | met |  |
| P20-MECH-SCHEMA-047 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-schema.md L48; TypeBox JSON mechanism: properties | {"path":"crates/opi-ai/src/schema/mod.rs","sha256":"816f780293f0426f0e063fae63111baf6a050d60999d99c1bad52be204d8af70","citation":"L138-L175","observed":"Public compile/check enter  | met |  |
| P20-MECH-SCHEMA-048 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-schema.md L48; TypeBox JSON mechanism: propertyNames | {"path":"crates/opi-ai/src/schema/mod.rs","sha256":"816f780293f0426f0e063fae63111baf6a050d60999d99c1bad52be204d8af70","citation":"L138-L175","observed":"Public compile/check enter  | met |  |
| P20-MECH-SCHEMA-049 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-schema.md L48; TypeBox JSON mechanism: minProperties | {"path":"crates/opi-ai/src/schema/mod.rs","sha256":"816f780293f0426f0e063fae63111baf6a050d60999d99c1bad52be204d8af70","citation":"L138-L175","observed":"Public compile/check enter  | met |  |
| P20-MECH-SCHEMA-050 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-schema.md L48; TypeBox JSON mechanism: maxProperties | {"path":"crates/opi-ai/src/schema/mod.rs","sha256":"816f780293f0426f0e063fae63111baf6a050d60999d99c1bad52be204d8af70","citation":"L138-L175","observed":"Public compile/check enter  | met |  |
| P20-MECH-SCHEMA-051 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-schema.md L48; TypeBox JSON mechanism: unevaluatedProperties | {"path":"crates/opi-ai/src/schema/mod.rs","sha256":"816f780293f0426f0e063fae63111baf6a050d60999d99c1bad52be204d8af70","citation":"L138-L175","observed":"Public compile/check enter  | met |  |
| P20-MECH-SCHEMA-052 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-schema.md L48; TypeBox JSON mechanism: additionalItems | {"path":"crates/opi-ai/src/schema/mod.rs","sha256":"816f780293f0426f0e063fae63111baf6a050d60999d99c1bad52be204d8af70","citation":"L138-L175","observed":"Public compile/check enter  | met |  |
| P20-MECH-SCHEMA-053 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-schema.md L48; TypeBox JSON mechanism: contains | {"path":"crates/opi-ai/src/schema/mod.rs","sha256":"816f780293f0426f0e063fae63111baf6a050d60999d99c1bad52be204d8af70","citation":"L138-L175","observed":"Public compile/check enter  | met |  |
| P20-MECH-SCHEMA-054 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-schema.md L48; TypeBox JSON mechanism: items | {"path":"crates/opi-ai/src/schema/mod.rs","sha256":"816f780293f0426f0e063fae63111baf6a050d60999d99c1bad52be204d8af70","citation":"L138-L175","observed":"Public compile/check enter  | met |  |
| P20-MECH-SCHEMA-055 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-schema.md L48; TypeBox JSON mechanism: maxContains | {"path":"crates/opi-ai/src/schema/mod.rs","sha256":"816f780293f0426f0e063fae63111baf6a050d60999d99c1bad52be204d8af70","citation":"L138-L175","observed":"Public compile/check enter  | met |  |
| P20-MECH-SCHEMA-056 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-schema.md L48; TypeBox JSON mechanism: maxItems | {"path":"crates/opi-ai/src/schema/mod.rs","sha256":"816f780293f0426f0e063fae63111baf6a050d60999d99c1bad52be204d8af70","citation":"L138-L175","observed":"Public compile/check enter  | met |  |
| P20-MECH-SCHEMA-057 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-schema.md L48; TypeBox JSON mechanism: minContains | {"path":"crates/opi-ai/src/schema/mod.rs","sha256":"816f780293f0426f0e063fae63111baf6a050d60999d99c1bad52be204d8af70","citation":"L138-L175","observed":"Public compile/check enter  | met |  |
| P20-MECH-SCHEMA-058 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-schema.md L48; TypeBox JSON mechanism: minItems | {"path":"crates/opi-ai/src/schema/mod.rs","sha256":"816f780293f0426f0e063fae63111baf6a050d60999d99c1bad52be204d8af70","citation":"L138-L175","observed":"Public compile/check enter  | met |  |
| P20-MECH-SCHEMA-059 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-schema.md L48; TypeBox JSON mechanism: prefixItems | {"path":"crates/opi-ai/src/schema/mod.rs","sha256":"816f780293f0426f0e063fae63111baf6a050d60999d99c1bad52be204d8af70","citation":"L138-L175","observed":"Public compile/check enter  | met |  |
| P20-MECH-SCHEMA-060 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-schema.md L48; TypeBox JSON mechanism: uniqueItems | {"path":"crates/opi-ai/src/schema/mod.rs","sha256":"816f780293f0426f0e063fae63111baf6a050d60999d99c1bad52be204d8af70","citation":"L138-L175","observed":"Public compile/check enter  | met |  |
| P20-MECH-SCHEMA-061 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-schema.md L48; TypeBox JSON mechanism: unevaluatedItems | {"path":"crates/opi-ai/src/schema/mod.rs","sha256":"816f780293f0426f0e063fae63111baf6a050d60999d99c1bad52be204d8af70","citation":"L138-L175","observed":"Public compile/check enter  | met |  |
| P20-MECH-SCHEMA-062 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-schema.md L48; TypeBox JSON mechanism: minLength | {"path":"crates/opi-ai/src/schema/mod.rs","sha256":"816f780293f0426f0e063fae63111baf6a050d60999d99c1bad52be204d8af70","citation":"L138-L175","observed":"Public compile/check enter  | met |  |
| P20-MECH-SCHEMA-063 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-schema.md L48; TypeBox JSON mechanism: maxLength | {"path":"crates/opi-ai/src/schema/mod.rs","sha256":"816f780293f0426f0e063fae63111baf6a050d60999d99c1bad52be204d8af70","citation":"L138-L175","observed":"Public compile/check enter  | met |  |
| P20-MECH-SCHEMA-064 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-schema.md L48; TypeBox JSON mechanism: format | {"path":"crates/opi-ai/src/schema/mod.rs","sha256":"816f780293f0426f0e063fae63111baf6a050d60999d99c1bad52be204d8af70","citation":"L138-L175","observed":"Public compile/check enter  | met |  |
| P20-MECH-SCHEMA-065 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-schema.md L48; TypeBox JSON mechanism: pattern | {"path":"crates/opi-ai/src/schema/mod.rs","sha256":"816f780293f0426f0e063fae63111baf6a050d60999d99c1bad52be204d8af70","citation":"L138-L175","observed":"Public compile/check enter  | met |  |
| P20-MECH-SCHEMA-066 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-schema.md L48; TypeBox JSON mechanism: minimum | {"path":"crates/opi-ai/src/schema/mod.rs","sha256":"816f780293f0426f0e063fae63111baf6a050d60999d99c1bad52be204d8af70","citation":"L138-L175","observed":"Public compile/check enter  | met |  |
| P20-MECH-SCHEMA-067 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-schema.md L48; TypeBox JSON mechanism: maximum | {"path":"crates/opi-ai/src/schema/mod.rs","sha256":"816f780293f0426f0e063fae63111baf6a050d60999d99c1bad52be204d8af70","citation":"L138-L175","observed":"Public compile/check enter  | met |  |
| P20-MECH-SCHEMA-068 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-schema.md L48; TypeBox JSON mechanism: exclusiveMinimum | {"path":"crates/opi-ai/src/schema/mod.rs","sha256":"816f780293f0426f0e063fae63111baf6a050d60999d99c1bad52be204d8af70","citation":"L138-L175","observed":"Public compile/check enter  | met |  |
| P20-MECH-SCHEMA-069 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-schema.md L48; TypeBox JSON mechanism: exclusiveMaximum | {"path":"crates/opi-ai/src/schema/mod.rs","sha256":"816f780293f0426f0e063fae63111baf6a050d60999d99c1bad52be204d8af70","citation":"L138-L175","observed":"Public compile/check enter  | met |  |
| P20-MECH-SCHEMA-070 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-schema.md L48; TypeBox JSON mechanism: multipleOf | {"path":"crates/opi-ai/src/schema/mod.rs","sha256":"816f780293f0426f0e063fae63111baf6a050d60999d99c1bad52be204d8af70","citation":"L138-L175","observed":"Public compile/check enter  | met |  |
| P20-MECH-SCHEMA-071 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-schema.md L48; TypeBox JSON mechanism: ref | {"path":"crates/opi-ai/src/schema/mod.rs","sha256":"816f780293f0426f0e063fae63111baf6a050d60999d99c1bad52be204d8af70","citation":"L138-L175","observed":"Public compile/check enter  | met |  |
| P20-MECH-SCHEMA-072 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-schema.md L48; TypeBox JSON mechanism: recursiveRef | {"path":"crates/opi-ai/src/schema/mod.rs","sha256":"816f780293f0426f0e063fae63111baf6a050d60999d99c1bad52be204d8af70","citation":"L138-L175","observed":"Public compile/check enter  | met |  |
| P20-MECH-SCHEMA-073 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-schema.md L48; TypeBox JSON mechanism: dynamicRef | {"path":"crates/opi-ai/src/schema/mod.rs","sha256":"816f780293f0426f0e063fae63111baf6a050d60999d99c1bad52be204d8af70","citation":"L138-L175","observed":"Public compile/check enter  | met |  |
| P20-MECH-SCHEMA-074 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-schema.md L48; TypeBox JSON mechanism: if | {"path":"crates/opi-ai/src/schema/mod.rs","sha256":"816f780293f0426f0e063fae63111baf6a050d60999d99c1bad52be204d8af70","citation":"L138-L175","observed":"Public compile/check enter  | met |  |
| P20-MECH-SCHEMA-075 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-schema.md L48; TypeBox JSON mechanism: not | {"path":"crates/opi-ai/src/schema/mod.rs","sha256":"816f780293f0426f0e063fae63111baf6a050d60999d99c1bad52be204d8af70","citation":"L138-L175","observed":"Public compile/check enter  | met |  |
| P20-MECH-SCHEMA-076 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-schema.md L48; TypeBox JSON mechanism: allOf | {"path":"crates/opi-ai/src/schema/mod.rs","sha256":"816f780293f0426f0e063fae63111baf6a050d60999d99c1bad52be204d8af70","citation":"L138-L175","observed":"Public compile/check enter  | met |  |
| P20-MECH-SCHEMA-077 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-schema.md L48; TypeBox JSON mechanism: anyOf | {"path":"crates/opi-ai/src/schema/mod.rs","sha256":"816f780293f0426f0e063fae63111baf6a050d60999d99c1bad52be204d8af70","citation":"L138-L175","observed":"Public compile/check enter  | met |  |
| P20-MECH-SCHEMA-078 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-schema.md L48; TypeBox JSON mechanism: oneOf | {"path":"crates/opi-ai/src/schema/mod.rs","sha256":"816f780293f0426f0e063fae63111baf6a050d60999d99c1bad52be204d8af70","citation":"L138-L175","observed":"Public compile/check enter  | met |  |
| P20-DEL-01 | docs/research/pi-ai-opi-ai-alignment/spec-draft.md DEL-01, line 40; lines 27,57,67,73,79,583 | {"path":"Cargo.toml","sha256":"11a2d459067c679bc06a1bc7af5fe67f4e72b9d856d34d2da83241ac0bb47f83","citation":"lines 1-26,46-60,115-140","observed":"Seven workspace members, edition2 | not-assessable | P20-AUD-018 |
| P20-DEL-02 | docs/research/pi-ai-opi-ai-alignment/spec-draft.md DEL-02, line 41; lines 210,274,535-536 | {"path":"Cargo.toml","sha256":"11a2d459067c679bc06a1bc7af5fe67f4e72b9d856d34d2da83241ac0bb47f83","citation":"lines 1-26,46-60,115-140","observed":"Seven workspace members, edition2 | not-met | P20-AUD-005 |
| P20-DEL-03 | docs/research/pi-ai-opi-ai-alignment/spec-draft.md DEL-03, line 42; lines 129,214,585 | {"path":"Cargo.toml","sha256":"11a2d459067c679bc06a1bc7af5fe67f4e72b9d856d34d2da83241ac0bb47f83","citation":"lines 1-26,46-60,115-140","observed":"Seven workspace members, edition2 | not-assessable | P20-AUD-018 |
| P20-DEL-04 | docs/research/pi-ai-opi-ai-alignment/spec-draft.md DEL-04, line 43; lines 177,399,504-510 | {"path":"Cargo.toml","sha256":"11a2d459067c679bc06a1bc7af5fe67f4e72b9d856d34d2da83241ac0bb47f83","citation":"lines 1-26,46-60,115-140","observed":"Seven workspace members, edition2 | not-met | P20-AUD-015 |
| P20-DEL-05 | docs/research/pi-ai-opi-ai-alignment/spec-draft.md DEL-05, line 44; lines 468-478 | {"path":"Cargo.toml","sha256":"11a2d459067c679bc06a1bc7af5fe67f4e72b9d856d34d2da83241ac0bb47f83","citation":"lines 1-26,46-60,115-140","observed":"Seven workspace members, edition2 | not-met | P20-AUD-002 |
| P20-DEL-06 | docs/research/pi-ai-opi-ai-alignment/spec-draft.md DEL-06, line 45; lines 482-488 | {"path":"Cargo.toml","sha256":"11a2d459067c679bc06a1bc7af5fe67f4e72b9d856d34d2da83241ac0bb47f83","citation":"lines 1-26,46-60,115-140","observed":"Seven workspace members, edition2 | not-assessable | P20-AUD-018 |
| P20-DEL-07 | docs/research/pi-ai-opi-ai-alignment/spec-draft.md DEL-07, line 46; lines 492-502 | {"path":"Cargo.toml","sha256":"11a2d459067c679bc06a1bc7af5fe67f4e72b9d856d34d2da83241ac0bb47f83","citation":"lines 1-26,46-60,115-140","observed":"Seven workspace members, edition2 | not-assessable | P20-AUD-018 |
| P20-DEL-08 | docs/research/pi-ai-opi-ai-alignment/spec-draft.md DEL-08, line 47; lines 176,338,411-415 | {"path":"Cargo.toml","sha256":"11a2d459067c679bc06a1bc7af5fe67f4e72b9d856d34d2da83241ac0bb47f83","citation":"lines 1-26,46-60,115-140","observed":"Seven workspace members, edition2 | not-met | P20-AUD-001, P20-AUD-002 |
| P20-DEL-09 | docs/research/pi-ai-opi-ai-alignment/spec-draft.md DEL-09, line 48; lines 216,242,345-349,462-464 | {"path":"Cargo.toml","sha256":"11a2d459067c679bc06a1bc7af5fe67f4e72b9d856d34d2da83241ac0bb47f83","citation":"lines 1-26,46-60,115-140","observed":"Seven workspace members, edition2 | not-met | P20-AUD-012 |
| P20-DEL-10 | docs/research/pi-ai-opi-ai-alignment/spec-draft.md DEL-10, line 49; lines 243,373,450 | {"path":"crates/opi-ai/resources/catalog/builtin/catalog.json","sha256":"ffa3e98626230f378f80200debb736c4a2238b5f2ba246a92cfc7d8e19604678","citation":"whole artifact","observed":"9 | not-met | P20-AUD-008, P20-AUD-016 |
| P20-DEL-11 | docs/research/pi-ai-opi-ai-alignment/spec-draft.md DEL-11, line 50; lines 89,185,236-245,587 | {"path":"Cargo.toml","sha256":"11a2d459067c679bc06a1bc7af5fe67f4e72b9d856d34d2da83241ac0bb47f83","citation":"lines 1-26,46-60,115-140","observed":"Seven workspace members, edition2 | not-met | P20-AUD-003, P20-AUD-011, P20-AUD-013, P20-AUD-014 |
| P20-DEL-12 | docs/research/pi-ai-opi-ai-alignment/spec-draft.md DEL-12, line 51; lines 226-228,306-316 | {"path":"Cargo.toml","sha256":"11a2d459067c679bc06a1bc7af5fe67f4e72b9d856d34d2da83241ac0bb47f83","citation":"lines 1-26,46-60,115-140","observed":"Seven workspace members, edition2 | met |  |
| P20-DEL-13 | docs/research/pi-ai-opi-ai-alignment/spec-draft.md DEL-13, line 52; lines 518-539,589 | {"path":"Cargo.toml","sha256":"11a2d459067c679bc06a1bc7af5fe67f4e72b9d856d34d2da83241ac0bb47f83","citation":"lines 1-26,46-60,115-140","observed":"Seven workspace members, edition2 | not-met | P20-AUD-004 |
| P20-DEL-14 | docs/research/pi-ai-opi-ai-alignment/spec-draft.md DEL-14, line 53; lines 29,588-596 | {"path":"Cargo.toml","sha256":"11a2d459067c679bc06a1bc7af5fe67f4e72b9d856d34d2da83241ac0bb47f83","citation":"lines 1-26,46-60,115-140","observed":"Seven workspace members, edition2 | not-met | P20-AUD-004, P20-AUD-016, P20-AUD-018 |
| P20-DEL-SCOPE-01 | docs/research/pi-ai-opi-ai-alignment/spec-draft.md lines 24-26,75,97,122,129 | {"path":"Cargo.toml","sha256":"11a2d459067c679bc06a1bc7af5fe67f4e72b9d856d34d2da83241ac0bb47f83","citation":"lines 1-26,46-60,115-140","observed":"Seven workspace members, edition2 | met |  |
| P20-DEL-SCOPE-02 | docs/research/pi-ai-opi-ai-alignment/spec-draft.md lines 26,492-494 | {"path":"Cargo.toml","sha256":"11a2d459067c679bc06a1bc7af5fe67f4e72b9d856d34d2da83241ac0bb47f83","citation":"lines 1-26,46-60,115-140","observed":"Seven workspace members, edition2 | met |  |
| P20-DEL-SCOPE-03 | docs/research/pi-ai-opi-ai-alignment/spec-draft.md lines 57,61-67,77 | {"path":"Cargo.toml","sha256":"11a2d459067c679bc06a1bc7af5fe67f4e72b9d856d34d2da83241ac0bb47f83","citation":"lines 1-26,46-60,115-140","observed":"Seven workspace members, edition2 | met |  |
| P20-DEL-SCOPE-04 | docs/research/pi-ai-opi-ai-alignment/spec-draft.md lines 79,97,239,592 | {"path":"Cargo.toml","sha256":"11a2d459067c679bc06a1bc7af5fe67f4e72b9d856d34d2da83241ac0bb47f83","citation":"lines 1-26,46-60,115-140","observed":"Seven workspace members, edition2 | met |  |
| P20-DEL-SCOPE-05 | docs/research/pi-ai-opi-ai-alignment/spec-draft.md lines 32,36,57,73 | {"path":"Cargo.toml","sha256":"11a2d459067c679bc06a1bc7af5fe67f4e72b9d856d34d2da83241ac0bb47f83","citation":"lines 1-26,46-60,115-140","observed":"Seven workspace members, edition2 | not-assessable | P20-AUD-018 |
| P20-DEL-SCOPE-06 | docs/research/pi-ai-opi-ai-alignment/spec-draft.md lines 30-31,353 | {"path":"Cargo.toml","sha256":"11a2d459067c679bc06a1bc7af5fe67f4e72b9d856d34d2da83241ac0bb47f83","citation":"lines 1-26,46-60,115-140","observed":"Seven workspace members, edition2 | not-assessable | P20-AUD-018 |
| P20-DEL-EXIT-01 | docs/research/pi-ai-opi-ai-alignment/spec-draft.md lines 29,17,452,613 | {"path":"Cargo.toml","sha256":"11a2d459067c679bc06a1bc7af5fe67f4e72b9d856d34d2da83241ac0bb47f83","citation":"lines 1-26,46-60,115-140","observed":"Seven workspace members, edition2 | not-met | P20-AUD-001, P20-AUD-003, P20-AUD-005 |
| P20-DEL-EXIT-02 | docs/research/pi-ai-opi-ai-alignment/spec-draft.md lines 29,583,590,592 | {"path":"Cargo.toml","sha256":"11a2d459067c679bc06a1bc7af5fe67f4e72b9d856d34d2da83241ac0bb47f83","citation":"lines 1-26,46-60,115-140","observed":"Seven workspace members, edition2 | not-met | P20-AUD-021 |
| P20-DEL-EXIT-03 | docs/research/pi-ai-opi-ai-alignment/spec-draft.md lines 29,53,560,590 | {"path":"Cargo.toml","sha256":"11a2d459067c679bc06a1bc7af5fe67f4e72b9d856d34d2da83241ac0bb47f83","citation":"lines 1-26,46-60,115-140","observed":"Seven workspace members, edition2 | not-assessable | P20-AUD-018 |
| P20-DEL-EXIT-04 | docs/research/pi-ai-opi-ai-alignment/spec-draft.md lines 520,528-539,589 | {"path":"Cargo.toml","sha256":"11a2d459067c679bc06a1bc7af5fe67f4e72b9d856d34d2da83241ac0bb47f83","citation":"lines 1-26,46-60,115-140","observed":"Seven workspace members, edition2 | not-met | P20-AUD-004 |
| P20-DEL-EXIT-05 | docs/research/pi-ai-opi-ai-alignment/spec-draft.md lines 246,539,594-596; AIR-067 | {"path":"Cargo.toml","sha256":"11a2d459067c679bc06a1bc7af5fe67f4e72b9d856d34d2da83241ac0bb47f83","citation":"lines 1-26,46-60,115-140","observed":"Seven workspace members, edition2 | not-assessable | P20-AUD-018 |
| P20-DEL-EXIT-06 | docs/research/pi-ai-opi-ai-alignment/spec-draft.md lines 594-596 | {"path":"Cargo.toml","sha256":"11a2d459067c679bc06a1bc7af5fe67f4e72b9d856d34d2da83241ac0bb47f83","citation":"lines 1-26,46-60,115-140","observed":"Seven workspace members, edition2 | not-assessable | P20-AUD-018 |
| P20-DEL-EXIT-07 | docs/research/pi-ai-opi-ai-alignment/acceptance-matrix.md lines 470-474,477,493 | {"path":"Cargo.toml","sha256":"11a2d459067c679bc06a1bc7af5fe67f4e72b9d856d34d2da83241ac0bb47f83","citation":"lines 1-26,46-60,115-140","observed":"Seven workspace members, edition2 | not-met | P20-AUD-016 |
| P20-DEL-EXIT-08 | docs/research/pi-ai-opi-ai-alignment/acceptance-matrix.md lines 468,472,475; spec-draft.md lines 568-570,594-596 | {"path":"Cargo.toml","sha256":"11a2d459067c679bc06a1bc7af5fe67f4e72b9d856d34d2da83241ac0bb47f83","citation":"lines 1-26,46-60,115-140","observed":"Seven workspace members, edition2 | met |  |
| P20-DEL-EXIT-09 | docs/research/pi-ai-opi-ai-alignment/acceptance-matrix.md lines 558-588,590-603 | {"path":"Cargo.toml","sha256":"11a2d459067c679bc06a1bc7af5fe67f4e72b9d856d34d2da83241ac0bb47f83","citation":"lines 1-26,46-60,115-140","observed":"Seven workspace members, edition2 | met |  |
| P20-DEL-REPLACE-01 | docs/research/pi-ai-opi-ai-alignment/spec-draft.md lines 524-525 | {"path":"Cargo.toml","sha256":"11a2d459067c679bc06a1bc7af5fe67f4e72b9d856d34d2da83241ac0bb47f83","citation":"lines 1-26,46-60,115-140","observed":"Seven workspace members, edition2 | met |  |
| P20-DEL-REPLACE-02 | docs/research/pi-ai-opi-ai-alignment/spec-draft.md lines 528-529,535 | {"path":"Cargo.toml","sha256":"11a2d459067c679bc06a1bc7af5fe67f4e72b9d856d34d2da83241ac0bb47f83","citation":"lines 1-26,46-60,115-140","observed":"Seven workspace members, edition2 | not-assessable | P20-AUD-018 |
| P20-DEL-REPLACE-03 | docs/research/pi-ai-opi-ai-alignment/spec-draft.md lines 531-534 | {"path":"Cargo.toml","sha256":"11a2d459067c679bc06a1bc7af5fe67f4e72b9d856d34d2da83241ac0bb47f83","citation":"lines 1-26,46-60,115-140","observed":"Seven workspace members, edition2 | not-met | P20-AUD-004 |
| P20-DEL-REPLACE-04 | docs/research/pi-ai-opi-ai-alignment/spec-draft.md lines 536-538 | {"path":"Cargo.toml","sha256":"11a2d459067c679bc06a1bc7af5fe67f4e72b9d856d34d2da83241ac0bb47f83","citation":"lines 1-26,46-60,115-140","observed":"Seven workspace members, edition2 | not-met | P20-AUD-004, P20-AUD-016 |
| P20-DEL-REPLACE-05 | docs/research/pi-ai-opi-ai-alignment/acceptance-matrix.md lines 431-448 | {"path":"Cargo.toml","sha256":"11a2d459067c679bc06a1bc7af5fe67f4e72b9d856d34d2da83241ac0bb47f83","citation":"lines 1-26,46-60,115-140","observed":"Seven workspace members, edition2 | not-assessable | P20-AUD-018 |
| P20-DEL-PKG-01 | docs/research/pi-ai-opi-ai-alignment/spec-draft.md AIR-050 line 215; lines 274,478,488; matrix lines 473,532 | {"path":"Cargo.toml","sha256":"11a2d459067c679bc06a1bc7af5fe67f4e72b9d856d34d2da83241ac0bb47f83","citation":"lines 1-26,46-60,115-140","observed":"Seven workspace members, edition2 | not-assessable | P20-AUD-018 |
| P20-DEL-PKG-02 | docs/research/pi-ai-opi-ai-alignment/spec-draft.md lines 215,274,588; matrix line 473 | {"path":"Cargo.toml","sha256":"11a2d459067c679bc06a1bc7af5fe67f4e72b9d856d34d2da83241ac0bb47f83","citation":"lines 1-26,46-60,115-140","observed":"Seven workspace members, edition2 | not-met | P20-AUD-017 |
| P20-DEL-PKG-03 | docs/research/pi-ai-opi-ai-alignment/spec-draft.md lines 474-478,510 | {"path":"Cargo.toml","sha256":"11a2d459067c679bc06a1bc7af5fe67f4e72b9d856d34d2da83241ac0bb47f83","citation":"lines 1-26,46-60,115-140","observed":"Seven workspace members, edition2 | not-assessable | P20-AUD-018 |
| P20-DEL-PKG-04 | docs/research/pi-ai-opi-ai-alignment/spec-draft.md lines 482-488 | {"path":"Cargo.toml","sha256":"11a2d459067c679bc06a1bc7af5fe67f4e72b9d856d34d2da83241ac0bb47f83","citation":"lines 1-26,46-60,115-140","observed":"Seven workspace members, edition2 | not-assessable | P20-AUD-018 |
| P20-DEL-PKG-05 | docs/research/pi-ai-opi-ai-alignment/spec-draft.md lines 496,500-502; backend lines 90-100,212-216 | {"path":"Cargo.toml","sha256":"11a2d459067c679bc06a1bc7af5fe67f4e72b9d856d34d2da83241ac0bb47f83","citation":"lines 1-26,46-60,115-140","observed":"Seven workspace members, edition2 | not-assessable | P20-AUD-018 |
| P20-DEL-PKG-06 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-rust-backends.md lines 92,212,214; spec-draft.md lines 492-500 | {"path":"Cargo.toml","sha256":"11a2d459067c679bc06a1bc7af5fe67f4e72b9d856d34d2da83241ac0bb47f83","citation":"lines 1-26,46-60,115-140","observed":"Seven workspace members, edition2 | not-assessable | P20-AUD-018 |
| P20-DEL-PKG-07 | docs/research/pi-ai-opi-ai-alignment/spec-draft.md lines 27,49,450; backend lines 186,206; matrix lines 507,550 | {"path":"crates/opi-ai/resources/catalog/builtin/catalog.json","sha256":"ffa3e98626230f378f80200debb736c4a2238b5f2ba246a92cfc7d8e19604678","citation":"whole artifact","observed":"9 | not-assessable | P20-AUD-018 |
| P20-DEL-PROV-01 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-rust-backends.md lines 186-188,212-216; spec-draft.md lines 450,476,478,486 | {"path":"Cargo.toml","sha256":"11a2d459067c679bc06a1bc7af5fe67f4e72b9d856d34d2da83241ac0bb47f83","citation":"lines 1-26,46-60,115-140","observed":"Seven workspace members, edition2 | not-assessable | P20-AUD-018 |
| P20-DEL-PROV-02 | docs/research/pi-ai-opi-ai-alignment/spec-draft.md lines 482,486,488; backend lines 188,214-216 | {"path":"Cargo.toml","sha256":"11a2d459067c679bc06a1bc7af5fe67f4e72b9d856d34d2da83241ac0bb47f83","citation":"lines 1-26,46-60,115-140","observed":"Seven workspace members, edition2 | not-assessable | P20-AUD-018 |
| P20-DEL-PROV-03 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-rust-backends.md lines 188,214-216; spec-draft.md lines 496 | {"path":"Cargo.toml","sha256":"11a2d459067c679bc06a1bc7af5fe67f4e72b9d856d34d2da83241ac0bb47f83","citation":"lines 1-26,46-60,115-140","observed":"Seven workspace members, edition2 | not-assessable | P20-AUD-018 |
| P20-DEL-PROV-04 | docs/research/pi-ai-opi-ai-alignment/spec-draft.md lines 496,498; backend lines 66,98,216 | {"path":"Cargo.toml","sha256":"11a2d459067c679bc06a1bc7af5fe67f4e72b9d856d34d2da83241ac0bb47f83","citation":"lines 1-26,46-60,115-140","observed":"Seven workspace members, edition2 | not-assessable | P20-AUD-018 |
| P20-DEL-PROV-05 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-rust-backends.md lines 104-116,126,146,152-154 | {"path":"Cargo.toml","sha256":"11a2d459067c679bc06a1bc7af5fe67f4e72b9d856d34d2da83241ac0bb47f83","citation":"lines 1-26,46-60,115-140","observed":"Seven workspace members, edition2 | not-assessable | P20-AUD-018 |
| P20-DEL-PROV-06 | docs/research/pi-ai-opi-ai-alignment/spec-draft.md lines 510; backend lines 64,66,216 | {"path":"Cargo.toml","sha256":"11a2d459067c679bc06a1bc7af5fe67f4e72b9d856d34d2da83241ac0bb47f83","citation":"lines 1-26,46-60,115-140","observed":"Seven workspace members, edition2 | not-assessable | P20-AUD-018 |
| P20-DEL-PLAT-01 | docs/research/pi-ai-opi-ai-alignment/spec-draft.md lines 28-29,502; backend lines 186,188,206 | {"path":"Cargo.toml","sha256":"11a2d459067c679bc06a1bc7af5fe67f4e72b9d856d34d2da83241ac0bb47f83","citation":"lines 1-26,46-60,115-140","observed":"Seven workspace members, edition2 | not-assessable | P20-AUD-018 |
| P20-DEL-PLAT-02 | docs/research/pi-ai-opi-ai-alignment/spec-draft.md lines 28,502; backend lines 186,206 | {"path":"Cargo.toml","sha256":"11a2d459067c679bc06a1bc7af5fe67f4e72b9d856d34d2da83241ac0bb47f83","citation":"lines 1-26,46-60,115-140","observed":"Seven workspace members, edition2 | not-assessable | P20-AUD-018 |
| P20-DEL-PLAT-03 | docs/research/pi-ai-opi-ai-alignment/spec-draft.md lines 486; backend lines 188,216 | {"path":"Cargo.toml","sha256":"11a2d459067c679bc06a1bc7af5fe67f4e72b9d856d34d2da83241ac0bb47f83","citation":"lines 1-26,46-60,115-140","observed":"Seven workspace members, edition2 | not-assessable | P20-AUD-018 |
| P20-DEL-PLAT-04 | docs/research/pi-ai-opi-ai-alignment/spec-draft.md lines 94,97,215,236,245 | {"path":"Cargo.toml","sha256":"11a2d459067c679bc06a1bc7af5fe67f4e72b9d856d34d2da83241ac0bb47f83","citation":"lines 1-26,46-60,115-140","observed":"Seven workspace members, edition2 | not-assessable | P20-AUD-018 |
| P20-DEL-ONB-01-A | docs/research/pi-ai-opi-ai-alignment/spec-draft.md ONB-01 line 554; matrix line 531 | {"path":"crates/opi-ai/README.md","sha256":"fca48944ae272e38055e6831e13c3fba81e5da77063656739c9fe89f09c1e0e1","citation":"lines 42-526","observed":"Ordinary installation, complete  | met |  |
| P20-DEL-ONB-01-B | docs/research/pi-ai-opi-ai-alignment/acceptance-matrix.md ONB-01 line 531; spec-draft.md line 554 | {"path":"crates/opi-ai/README.md","sha256":"fca48944ae272e38055e6831e13c3fba81e5da77063656739c9fe89f09c1e0e1","citation":"lines 42-526","observed":"Ordinary installation, complete  | not-assessable | P20-AUD-018 |
| P20-DEL-ONB-01-C | docs/research/pi-ai-opi-ai-alignment/spec-draft.md ONB-01 line 554 | {"path":"crates/opi-ai/README.md","sha256":"fca48944ae272e38055e6831e13c3fba81e5da77063656739c9fe89f09c1e0e1","citation":"lines 42-526","observed":"Ordinary installation, complete  | met |  |
| P20-DEL-ONB-02-A | docs/research/pi-ai-opi-ai-alignment/spec-draft.md ONB-02 line 555; matrix line 532 | {"path":"crates/opi-ai/README.md","sha256":"fca48944ae272e38055e6831e13c3fba81e5da77063656739c9fe89f09c1e0e1","citation":"lines 42-526","observed":"Ordinary installation, complete  | not-assessable | P20-AUD-018 |
| P20-DEL-ONB-02-B | docs/research/pi-ai-opi-ai-alignment/spec-draft.md ONB-02 line 555; matrix line 532 | {"path":"crates/opi-ai/README.md","sha256":"fca48944ae272e38055e6831e13c3fba81e5da77063656739c9fe89f09c1e0e1","citation":"lines 42-526","observed":"Ordinary installation, complete  | not-assessable | P20-AUD-018 |
| P20-DEL-ONB-02-C | docs/research/pi-ai-opi-ai-alignment/spec-draft.md ONB-02 line 555 | {"path":"crates/opi-ai/README.md","sha256":"fca48944ae272e38055e6831e13c3fba81e5da77063656739c9fe89f09c1e0e1","citation":"lines 42-526","observed":"Ordinary installation, complete  | met |  |
| P20-DEL-ONB-02-D | docs/research/pi-ai-opi-ai-alignment/spec-draft.md ONB-02 line 555; lines 211,296 | {"path":"crates/opi-ai/README.md","sha256":"fca48944ae272e38055e6831e13c3fba81e5da77063656739c9fe89f09c1e0e1","citation":"lines 42-526","observed":"Ordinary installation, complete  | not-met | P20-AUD-012 |
| P20-DEL-ONB-03-A | docs/research/pi-ai-opi-ai-alignment/spec-draft.md ONB-03 line 556; matrix line 533 | {"path":"crates/opi-ai/README.md","sha256":"fca48944ae272e38055e6831e13c3fba81e5da77063656739c9fe89f09c1e0e1","citation":"lines 42-526","observed":"Ordinary installation, complete  | met |  |
| P20-DEL-ONB-03-B | docs/research/pi-ai-opi-ai-alignment/acceptance-matrix.md ONB-03 line 533; spec-draft.md line 556 | {"path":"crates/opi-ai/README.md","sha256":"fca48944ae272e38055e6831e13c3fba81e5da77063656739c9fe89f09c1e0e1","citation":"lines 42-526","observed":"Ordinary installation, complete  | not-met | P20-AUD-013 |
| P20-DEL-ONB-03-C | docs/research/pi-ai-opi-ai-alignment/spec-draft.md ONB-03 line 556 | {"path":"crates/opi-ai/README.md","sha256":"fca48944ae272e38055e6831e13c3fba81e5da77063656739c9fe89f09c1e0e1","citation":"lines 42-526","observed":"Ordinary installation, complete  | not-assessable | P20-AUD-018 |
| P20-DEL-ONB-03-D | docs/research/pi-ai-opi-ai-alignment/spec-draft.md ONB-03 line 556; matrix line 533 | {"path":"crates/opi-ai/README.md","sha256":"fca48944ae272e38055e6831e13c3fba81e5da77063656739c9fe89f09c1e0e1","citation":"lines 42-526","observed":"Ordinary installation, complete  | not-met | P20-AUD-008 |
| P20-DEL-ONB-04-A | docs/research/pi-ai-opi-ai-alignment/spec-draft.md ONB-04 line 557; matrix line 534 | {"path":"crates/opi-ai/README.md","sha256":"fca48944ae272e38055e6831e13c3fba81e5da77063656739c9fe89f09c1e0e1","citation":"lines 42-526","observed":"Ordinary installation, complete  | met |  |
| P20-DEL-ONB-04-B | docs/research/pi-ai-opi-ai-alignment/spec-draft.md ONB-04 line 557; matrix line 534 | {"path":"crates/opi-ai/README.md","sha256":"fca48944ae272e38055e6831e13c3fba81e5da77063656739c9fe89f09c1e0e1","citation":"lines 42-526","observed":"Ordinary installation, complete  | not-met | P20-AUD-014 |
| P20-DEL-ONB-04-C | docs/research/pi-ai-opi-ai-alignment/spec-draft.md ONB-04 line 557; matrix line 534 | {"path":"crates/opi-ai/README.md","sha256":"fca48944ae272e38055e6831e13c3fba81e5da77063656739c9fe89f09c1e0e1","citation":"lines 42-526","observed":"Ordinary installation, complete  | not-assessable | P20-AUD-018 |
| P20-DEL-ONB-05-A | docs/research/pi-ai-opi-ai-alignment/acceptance-matrix.md ONB-05 line 535; spec-draft.md line 558 | {"path":"crates/opi-ai/README.md","sha256":"fca48944ae272e38055e6831e13c3fba81e5da77063656739c9fe89f09c1e0e1","citation":"lines 42-526","observed":"Ordinary installation, complete  | not-met | P20-AUD-016 |
| P20-DEL-ONB-05-B | docs/research/pi-ai-opi-ai-alignment/spec-draft.md ONB-05 line 558; matrix line 535 | {"path":"crates/opi-ai/README.md","sha256":"fca48944ae272e38055e6831e13c3fba81e5da77063656739c9fe89f09c1e0e1","citation":"lines 42-526","observed":"Ordinary installation, complete  | not-met | P20-AUD-002 |
| P20-DEL-ONB-05-C | docs/research/pi-ai-opi-ai-alignment/spec-draft.md ONB-05 line 558; lines 562-579 | {"path":"crates/opi-ai/README.md","sha256":"fca48944ae272e38055e6831e13c3fba81e5da77063656739c9fe89f09c1e0e1","citation":"lines 42-526","observed":"Ordinary installation, complete  | met |  |
| P20-DEL-ONB-06-A | docs/research/pi-ai-opi-ai-alignment/spec-draft.md ONB-06 line 559; matrix line 536 | {"path":"crates/opi-ai/README.md","sha256":"fca48944ae272e38055e6831e13c3fba81e5da77063656739c9fe89f09c1e0e1","citation":"lines 42-526","observed":"Ordinary installation, complete  | not-assessable | P20-AUD-018 |
| P20-DEL-ONB-06-B | docs/research/pi-ai-opi-ai-alignment/spec-draft.md ONB-06 line 559; matrix line 536 | {"path":"crates/opi-ai/README.md","sha256":"fca48944ae272e38055e6831e13c3fba81e5da77063656739c9fe89f09c1e0e1","citation":"lines 42-526","observed":"Ordinary installation, complete  | met |  |
| P20-DEL-ONB-07-A | docs/research/pi-ai-opi-ai-alignment/spec-draft.md ONB-07 line 560; matrix line 537 | {"path":"crates/opi-ai/README.md","sha256":"fca48944ae272e38055e6831e13c3fba81e5da77063656739c9fe89f09c1e0e1","citation":"lines 42-526","observed":"Ordinary installation, complete  | not-assessable | P20-AUD-018 |
| P20-DEL-ONB-07-B | docs/research/pi-ai-opi-ai-alignment/spec-draft.md ONB-07 line 560; matrix lines 537,558-603 | {"path":"crates/opi-ai/README.md","sha256":"fca48944ae272e38055e6831e13c3fba81e5da77063656739c9fe89f09c1e0e1","citation":"lines 42-526","observed":"Ordinary installation, complete  | not-assessable | P20-AUD-018 |
| P20-DEL-ONB-07-C | docs/research/pi-ai-opi-ai-alignment/spec-draft.md ONB-07 line 560; lines 562-570; matrix line 537 | {"path":"crates/opi-ai/README.md","sha256":"fca48944ae272e38055e6831e13c3fba81e5da77063656739c9fe89f09c1e0e1","citation":"lines 42-526","observed":"Ordinary installation, complete  | not-assessable | P20-AUD-018 |
| P20-DEL-ONB-08-SOURCE | docs/research/pi-ai-opi-ai-alignment/spec-draft.md lines 543-550,572-579; matrix lines 524-527 | {"path":"crates/opi-ai/README.md","sha256":"fca48944ae272e38055e6831e13c3fba81e5da77063656739c9fe89f09c1e0e1","citation":"lines 42-526","observed":"Ordinary installation, complete  | met |  |

## Standards Review

Workspace topology remains seven members; opi-ai has no internal Opi dependency. Metadata/dependencies are workspace-owned. Models is the single mutable assembly/driver owner; provider_collection contains builtin registration rather than a parallel collection facade. Ordinary, advanced and direct consumers compile together. cargo fmt, workspace all-target compilation and documentation contracts passed. The three stale SDK assertions and one real auth-consumer regression prevent passing runtime gates.

## Spec Review

Independent coverage follows the latest registered English/Chinese delivery text, source SDK contracts, current RDF overrides and the sole matrix. Historical TP/SP sections were adopted only where expressly selected. Current behavior is grounded in actual adapters and public consumers, not only registered profile counts. Native/default/C-zlib package graphs, three native platforms, MSRV1.97 and several exact finite-case/source acceptance obligations remain explicitly unestablished; these are reciprocal limitations, not silent passes. The report does not certify Phase exit or authorize remediation.

## Security, Invariants, Integration, Test Quality, and Residuals

The real request probes distinguish selected versus actually dispatched auth, actual redirect target headers, immutable terminal outcomes, already acquired usage and source budget/cap arithmetic. All material was synthetic and local. Error-canary observations concern default result diagnostics; trusted explicit raw callbacks remain a different contract. The ADC probe observes new query forwarding through Referer, not an actual remote compromise or Authorization leakage.

The exact locked reqwest0.12.28 and serde_json1.0.149 archive/checksum/source files were copied as read-only primary proof before inspection. The production byte/CRC/proxy/TLS/WS/schema/normalizer consumers and current discriminating fixtures were traced by their owning slice. Tests that observe only metadata, an internal helper or a consumer-shaped in-workspace call were not promoted to native/package proof.

A cancellation candidate was withdrawn after refutation: cancelling from on_retry_attempt_start happens after the registered cancellable wait. Its failed stronger probe is retained as an observation and supplies no blocking finding. The separate public overflow helper and usage bucket algebra meet their own precise criteria; the whole-call overflow retry and image observer fact loss are assessed only at their exact owners.

## Minimum-change Conformance

All68 admitted tasks received a current structural placement/reuse/consumer/ceiling review, merged with the behavioral owning-surface evidence above. 67 structural rows conform; the crypto worker/verify test-only production helpers trigger the recorded simplification revisit. Historical net-deletion counts remain not-assessable without importing forbidden history. This structural status does not close failed runtime or missing acceptance proof.

| Task | Current structural status | Production/nonproduction seam evidence | Residual / net deletion |
|---|---|---|
| 20.1 | conforming | Lexical current-file references recorded per representative symbol; not asserted dynamically real consumers. | []; historical net deletion unknown |
| 20.2 | conforming | [{'path': 'crates/opi-ai/src/stream.rs', 'sha256': '5d5866af19cd6f5fb2d9f5ab9b09e2d5c78bfdac6e9a245b9971710fdc976605', 'header': ['//! Streaming response events (S7.3).', '', 'use std::future::Future;', 'use std::pin::Pin;', 'use  | []; historical net deletion unknown |
| 20.3 | conforming | [{'path': 'crates/opi-ai/src/models.rs', 'sha256': 'f9fc8b51c90282772b6b8812079772ad05c264c0b7d80efbd51314a8b46e914b', 'header': ['//! Single `Models` front door: catalog, redacted auth status, and chat calls.', '//!', '//! [`Mode | ['Current provider_collection.rs is builtin registration glue over Models, not a parallel state owner. ProviderCollectio; historical net deletion unknown |
| 20.4 | conforming | [{'path': 'crates/opi-ai/src/retry.rs', 'sha256': '6ee0b8b45005f37866caab4c8191607bbc89607929dca1503a033a56f2ec430c', 'header': ['//! Retry/backoff/rate-limit support.', '//!', '//! Task 2.15 introduced the header parsing, exponen | []; historical net deletion unknown |
| 20.5 | conforming | [{'path': 'crates/opi-ai/src/text_decode.rs', 'sha256': '12a9eee7da5b68d20884549bcf7a97f9b2d5668f9ded2cf93a3a6ae7047a588e', 'header': ['//! Incremental byte-to-text decoding for streaming adapters (AIR-006).', '//!', '//! Rust equ | []; historical net deletion unknown |
| 20.6 | conforming | [{'path': 'crates/opi-ai/src/transcript.rs', 'sha256': 'bfc33852cd44a966cf0df2fa62741b5627b87c5e3866c40a599effceeee3bf95', 'header': ['//! Canonical context/transcript chronology (DEL-03, AIR-008/011/020).', '//!', '//! The public | []; historical net deletion unknown |
| 20.7 | conforming | [{'path': 'crates/opi-ai/src/replay.rs', 'sha256': '522ee27e143767e0722dc0ff572e674f4abc78aa00ce209b52c017d8862e95f8', 'header': ['//! Canonical replay projection (AIR-010/012/013/014, task 20.7).', '//!', '//! A replay projection | []; historical net deletion unknown |
| 20.8 | conforming | [{'path': 'crates/opi-ai/src/schema/mod.rs', 'sha256': '816f780293f0426f0e063fae63111baf6a050d60999d99c1bad52be204d8af70', 'header': ['//! Private TypeBox JSON-dialect schema engine (DEL-04 substrate).', '//!', '//! This module im | []; historical net deletion unknown |
| 20.9 | conforming | [{'path': 'crates/opi-ai/src/schema/ir.rs', 'sha256': 'c4aee8ca7342ad8c31647b5de66a55bc18ec8d0d85cf87f17f302c63411a9ebe', 'header': ['//! Admission of a trusted JSON schema into the dialect IR.', '//!', '//! The schema stays a `se | []; historical net deletion unknown |
| 20.10 | conforming | [{'path': 'crates/opi-ai/src/schema/normalize.rs', 'sha256': '10491c6d01a2ce7aab562e805c7317e0b40bd6bae42b70ab62ceae475ae16b92', 'header': ['//! Public tool-argument normalizer/validator seam (DEL-04, task 20.10).', '//!', '//! Po | []; historical net deletion unknown |
| 20.11 | conforming | [{'path': 'crates/opi-ai/src/constrained_sampling.rs', 'sha256': '96828767b35a388563c0573ca928bf921fd27b1b1ca1fea579eb6585f1be2833', 'header': ['//! Constrained sampling: strict JSON-schema wire projection and grammar', '//! tools | []; historical net deletion unknown |
| 20.12 | conforming | [{'path': 'crates/opi-ai/src/http.rs', 'sha256': '9c836d8f660f6fd69a0cd4706c4c01f47bd1827c17256566b4ebd4a6a23f4d7c', 'header': ['//! Shared HTTP client with connection pooling and proxy support (tasks 3.13, 3.12).', '//!', '//! Pr | []; historical net deletion unknown |
| 20.13 | triggered | [{'path': 'crates/opi-ai/src/crypto/mod.rs', 'sha256': '59c3f45c154e484e4dc20c9ccd0b8453d4cfde375657d1bf3dbc0b93da8ba39c', 'header': ['//! Private upstream key-admission and signing owner (RDF-23 substrate).', '//!', "//! Owns opi | ['lib.rs23-28 still carries blanket allow(dead_code) and pre-consumer task-history comment despite real SigningOwner con; historical net deletion unknown |
| 20.14 | conforming | [{'path': 'crates/opi-ai/src/tls/mod.rs', 'sha256': 'ba96a12a89ae10faeefc42385d37277eefee10bbc1a406e7ad0e22ad029b3090', 'header': ['//! Source-scoped TLS configuration for [`crate::http::HttpClient`] (RDF-23).', '//!', '//! This m | ['lib.rs23-28 still carries blanket allow(dead_code) and pre-consumer task-history comment despite real SigningOwner con; historical net deletion unknown |
| 20.15 | conforming | [{'path': 'crates/opi-ai/src/ws.rs', 'sha256': 'c7f416ce886a590fd752ca8113c1fe7f1e7a0536497b3daa028bb95ebbca0254', 'header': ['//! Strict HTTP Upgrade bridge and private WebSocket client protocol state.', '//!', '//! The public br | ['Public ws Upgrade preserved with pub(crate) frame/io/pmd. Public SessionResourceOwner provides namespace/register/clea; historical net deletion unknown |
| 20.16 | conforming | [{'path': 'crates/opi-ai/src/openai_codex_responses.rs', 'sha256': '1d5fc91a735b941d9d4aafa71c22808002cc3aaa18727145881418e156b77337', 'header': ['//! Dedicated OpenAI Codex Responses wire.', '//!', '//! This subscription-specific | ['Public ws Upgrade preserved with pub(crate) frame/io/pmd. Public SessionResourceOwner provides namespace/register/clea; historical net deletion unknown |
| 20.17 | conforming | [{'path': 'crates/opi-ai/src/codex_resources.rs', 'sha256': 'bda5772be7b8d75a95037041f731767e9de3e2130b804ec68146949b64581a7d', 'header': ['//! Instance resource generation, lease tracking and session cleanup', '//! (task 20.17; P | ['Public ws Upgrade preserved with pub(crate) frame/io/pmd. Public SessionResourceOwner provides namespace/register/clea; historical net deletion unknown |
| 20.18 | conforming | [{'path': 'crates/opi-ai/src/auth.rs', 'sha256': 'bff7ff1a5e2575e62119861762393b8752be99898a45510d6685aed1ed462019', 'header': ['//! Per-request auth resolution and store-owned OAuth coordination.', '//!', '//! The SDK owns source | []; historical net deletion unknown |
| 20.19.1 | conforming | [{'path': 'crates/opi-ai/src/auth/aws/local.rs', 'sha256': 'e6cc868edff9db3b5c1782e3085e1ee99d7214f898ce57f8f34435539cde6ec2', 'header': ['//! Local AWS credential-source chain for Bedrock.', '//!', '//! One coordinator resolves o | []; historical net deletion unknown |
| 20.19.2 | conforming | [{'path': 'crates/opi-ai/src/auth/aws/role.rs', 'sha256': 'f3c0cfb04be9c9ecefbaac4e7d35f065e9f6c7b7f5b2dc5756031e909ed8109c', 'header': ['//! STS `AssumeRole` source exchanges.', '//!', '//! The owned wire: `POST /` with `applicat | []; historical net deletion unknown |
| 20.19.3 | conforming | [{'path': 'crates/opi-ai/src/auth/aws/sso.rs', 'sha256': 'f2a4bbfe7e07fb77f4139efb7d5fde75713ccd60b8575c1978d7bb7e14c2f2b1', 'header': ['//! Owned AWS SSO source family (task 20.19.3): modern `sso_session` and', '//! legacy inline | []; historical net deletion unknown |
| 20.20 | conforming | [{'path': 'crates/opi-ai/src/retry.rs', 'sha256': '6ee0b8b45005f37866caab4c8191607bbc89607929dca1503a033a56f2ec430c', 'header': ['//! Retry/backoff/rate-limit support.', '//!', '//! Task 2.15 introduced the header parsing, exponen | []; historical net deletion unknown |
| 20.21.1 | conforming | [{'path': 'crates/opi-ai/src/auth/adc/native.rs', 'sha256': 'ee4a688608e406e673495f1b47e192909bc85c1b27c493138c28fe47a17aad94', 'header': ['//! ADC native credential families for Vertex generation auth (task 20.21.1).', '//!', "// | ['lib.rs23-28 still carries blanket allow(dead_code) and pre-consumer task-history comment despite real SigningOwner con; historical net deletion unknown |
| 20.21.2 | conforming | [{'path': 'crates/opi-ai/src/auth/adc/external.rs', 'sha256': '91166b2483d3abd7559c0086bb8eb828023978dc0a597524cdc9041605419104', 'header': ['//! ADC `external_account` family coordinator (task 20.21.2).', '//!', '//! Pinned from  | []; historical net deletion unknown |
| 20.21.3 | conforming | [{'path': 'crates/opi-ai/src/auth/adc/impersonation.rs', 'sha256': '242e1f0a1b104c72947d7367bd75f97f556f0f45be59ff95d432287438dbf0d4', 'header': ['//! ADC `impersonated_service_account` files and the IAM', '//! `generateAccessToke | []; historical net deletion unknown |
| 20.22 | conforming | [{'path': 'crates/opi-ai/src/mutation.rs', 'sha256': 'd30dd4aa2244808a0044d8831fb58f6f9cfc9fad0a61fc8922a924e84295b6d2', 'header': ['//! Store-owned serialized credential mutations (P20-STORE / task 20.22).', '//!', '//! Mutations | []; historical net deletion unknown |
| 20.23 | conforming | [{'path': 'crates/opi-ai/src/auth/oauth/anthropic.rs', 'sha256': '2dbe09b3724d18d5be8f9a5a7bcc31e4ccb09ca03cf7950f394386f4792b144e', 'header': ['//! Anthropic OAuth flow (Claude Pro/Max), pinned to pi 1.3.0', '//! `packages/ai/src | []; historical net deletion unknown |
| 20.24 | conforming | [{'path': 'crates/opi-ai/src/auth/oauth/codex.rs', 'sha256': '052aef0d1eac94e3d4d217477c8d82bb913e0e4b5668d2bfa0dc865ce5e0d264', 'header': ['//! OpenAI Codex (ChatGPT OAuth) legacy flow, pinned to pi 1.3.0', '//! `packages/ai/src/ | []; historical net deletion unknown |
| 20.25 | conforming | [{'path': 'crates/opi-ai/src/auth/oauth/chatgpt.rs', 'sha256': '5de88bcdafd302e6fb4bbd74ad11668a5fdeb959ccc9ddb15846ab6ac6b659fd', 'header': ['//! OpenAI "Sign in with ChatGPT" OAuth (Responses API token sharing).', '//!', '//! Pi | []; historical net deletion unknown |
| 20.26 | conforming | [{'path': 'crates/opi-ai/src/auth/oauth/copilot.rs', 'sha256': 'fd3eb40f8b7862e1aca919e98604cca458dd00bca592a5635f2c2b38135a70df', 'header': ['//! GitHub Copilot OAuth flow, pinned to pi 1.3.0', '//! `packages/ai/src/auth/oauth/gi | []; historical net deletion unknown |
| 20.27 | conforming | [{'path': 'crates/opi-ai/src/auth/oauth/openrouter.rs', 'sha256': '16a26a03a049802d9598c02db3eac796d709186063512dbd3efb5207e7189b51', 'header': ['//! OpenRouter OAuth PKCE flow, pinned to pi 1.3.0', '//! `packages/ai/src/auth/oaut | []; historical net deletion unknown |
| 20.28 | conforming | [{'path': 'crates/opi-ai/src/auth/oauth/kimi_coding.rs', 'sha256': '1e81bbbb2371351832f82673670704b1e3bcb6f14f379ba855ef9d2a91cd1e3b', 'header': ['//! Kimi Code (subscription) OAuth device flow, pinned to pi 1.3.0', '//! `packages | []; historical net deletion unknown |
| 20.29 | conforming | [{'path': 'crates/opi-ai/src/auth/oauth/meta.rs', 'sha256': 'fb36d3e5177f4f792f5c1e2c4451d5d2d4f85624fc720cb27c1d6fa3941996ce', 'header': ['//! Meta (Muse subscription) OAuth device flow, pinned to pi 1.3.0', '//! `packages/ai/src | []; historical net deletion unknown |
| 20.30 | conforming | [{'path': 'crates/opi-ai/src/auth/oauth/xai.rs', 'sha256': '7ba3bf84cd628e276f1b133240104ec05a4d27d5401360d4672e13782903e9cb', 'header': ['//! xAI (Grok/X subscription) OAuth device flow, pinned to pi 1.3.0', '//! `packages/ai/src | []; historical net deletion unknown |
| 20.31 | conforming | [{'path': 'crates/opi-ai/src/auth/oauth/radius.rs', 'sha256': '07f483d756e9ed2b00f09c072f992eecbae52cffc98a310314a8e46d300a646a', 'header': ['//! Radius gateway OAuth flow, pinned to pi 1.3.0', '//! `packages/ai/src/auth/oauth/rad | []; historical net deletion unknown |
| 20.32 | conforming | [{'path': 'crates/opi-ai/src/catalog/mod.rs', 'sha256': 'f8fdc6d1ab6a1401583552030bb95a99540c998d7f1da48ad7b48a784c3af588', 'header': ['//! Offline bundled model catalog: authentic artifact, loader, and getters.', '//!', '//! The  | []; historical net deletion unknown |
| 20.33 | conforming | [{'path': 'crates/opi-ai/src/catalog/store.rs', 'sha256': 'a77a69ea258051b3e01982351db6656a9254c9b9bdf662e0fb992fd06ffdde7c', 'header': ['//! Persisted dynamic-catalog store with a single revision-commit owner.', '//!', '//! [`Mod | []; historical net deletion unknown |
| 20.34 | conforming | [{'path': 'crates/opi-ai/src/provider_collection.rs', 'sha256': 'fce9b8186f864933ce0cf54e30a1c9612625845c7e3419ca05ffc7fecf51de32', 'header': ['//! The builtin provider collection (task 20.34).', '//!', '//! Turns the pinned [`cra | ['Current provider_collection.rs is builtin registration glue over Models, not a parallel state owner. ProviderCollectio; historical net deletion unknown |
| 20.35 | conforming | [{'path': 'crates/opi-ai/src/context.rs', 'sha256': '800984e0b24068bd8b6d3829345a74efaba03f8593a6c1240ff298eda06b5198', 'header': ['//! Context occupancy estimates for request budgets and compaction decisions.', '//!', '//! Estima | []; historical net deletion unknown |
| 20.36 | conforming | [{'path': 'crates/opi-ai/src/openai_chat.rs', 'sha256': '43a9a83be25317b570cf2d6acc82ad17f8bb052058ad6786d7e19872d772d80d', 'header': ['//! OpenAI-compatible chat completions SSE provider (S8.1).', '//!', '//! Implements streaming | []; historical net deletion unknown |
| 20.37 | conforming | [{'path': 'crates/opi-ai/src/openai_responses.rs', 'sha256': '7355e7a8fcd39a691a8d5fc530fdd7449be5a71ca3236e85cd0753e99e8a11cf', 'header': ['//! Standard OpenAI Responses provider.', '//!', "//! `previous_response_id` is intention | []; historical net deletion unknown |
| 20.38 | conforming | [{'path': 'crates/opi-ai/src/azure_openai.rs', 'sha256': 'a36664f322e61a00fbf05c87d22bf6b191b4e05639d5cd828d6df9ec548baad3', 'header': ['//! Azure OpenAI provider profile (task 20.38: explicit dual wires).', '//!', '//! - Chat wir | []; historical net deletion unknown |
| 20.39 | conforming | [{'path': 'crates/opi-ai/src/anthropic.rs', 'sha256': '984c5edfe29ac026c14654d03fbc3cd55ff3095dd2cd69c396a55995b4e40599', 'header': ['//! Anthropic Messages SSE provider (S8.1).', '', 'use std::sync::Arc;', '', 'use futures_util:: | []; historical net deletion unknown |
| 20.40 | conforming | [{'path': 'crates/opi-ai/src/gemini.rs', 'sha256': '9a04958dc62ebf390982c60e2a64ed403ced53974978ba809777539037490fd6', 'header': ['//! Google Gemini `streamGenerateContent` SSE provider (S8.1).', '//!', '//! Implements streaming f | []; historical net deletion unknown |
| 20.41 | conforming | [{'path': 'crates/opi-ai/src/vertex.rs', 'sha256': '54bee130a52166e38510e3f74ff52f6784cf05dfde44f153b736a7beaf66373c', 'header': ['//! Google Vertex AI provider.', '//!', '//! Routes through the Gemini `streamGenerateContent` adap | []; historical net deletion unknown |
| 20.42 | conforming | [{'path': 'crates/opi-ai/src/bedrock/mod.rs', 'sha256': 'da4339b929f4a133e083919d00c2b3bf27444361cba1b24c7d42f97b0f3c62fc', 'header': ['//! AWS Bedrock provider.', '//!', '//! Implements the Bedrock Converse API with SigV4 or Bear | []; historical net deletion unknown |
| 20.43 | conforming | [{'path': 'crates/opi-ai/src/mistral.rs', 'sha256': 'e02f3d56987e07bdc01d620e68a879278d4dd4b87e841d1173ae03a1c7c67a14', 'header': ['//! Mistral provider profile  - routes through the OpenAI-compatible adapter.', '//!', '//! Mistra | []; historical net deletion unknown |
| 20.44 | conforming | [{'path': 'crates/opi-ai/src/pi_messages.rs', 'sha256': '31f17d0f5ea6202cb06669956456f6d090431cba3705732390afff1225a106d5', 'header': ['//! pi-messages direct adapter (task 20.44, DEL-11).', '//!', "//! Pins `.repo/pi-1.3.0/packag | []; historical net deletion unknown |
| 20.45 | conforming | [{'path': 'crates/opi-ai/src/images.rs', 'sha256': '1b17938fe0f3c5f337155ca689d03e8ff65bca2f9e1ec0937b35a37f951e3bb9', 'header': ['//! Image-generation contracts: public context, result and model-record types.', '//!', '//! The im | []; historical net deletion unknown |
| 20.46 | conforming | [{'path': 'crates/opi-ai/src/classifier.rs', 'sha256': '777f5d93091940d9304588fd481dacbf43c5d6d0c1fe802aee446ed88c7bcec3', 'header': ['//! Classifier contracts: public context, result and model-record types.', '//!', '//! The clas | []; historical net deletion unknown |
| 20.47 | conforming | [{'path': 'crates/opi-ai/src/api/cloudflare.rs', 'sha256': '509e06c9e141908105ce5bb41a33bedd548bfdedfece62a41a3a0f02adff6da0', 'header': ['//! Cloudflare Workers AI System One adapter (task 20.47).', '//!', "//! Serves System One  | []; historical net deletion unknown |
| 20.48 | conforming | [{'path': 'crates/opi-ai/src/api/llama.rs', 'sha256': '2c551c540271c81e4381fbfda0fc0b99b03f992ddc63d9ec52a155bcc5e01b2c', 'header': ["//! Classification with a chat model served by llama.cpp's `llama-server`.", '//!', '//! The mod | []; historical net deletion unknown |
| 20.49 | conforming | [{'path': 'crates/opi-ai/src/api/decisions.rs', 'sha256': '29c871f79b3ebca29fba677a31b7283c627558426e2de689ea8cf12f4f792f06', 'header': ["//! OpenAI's Decisions API (task 20.49).", '//!', '//! One `POST <base>/decisions` with `{mo | []; historical net deletion unknown |
| 20.50 | conforming | [{'path': 'crates/opi-ai/src/deferred.rs', 'sha256': 'dce0171b8766a6fe24f3ec74c28e37822d25de4135041650dde270c592fd2578', 'header': ['//! Deferred-response contracts (task 20.50, AIR-043/AIR-044).', '//!', '//! Deferred submission  | []; historical net deletion unknown |
| 20.51 | conforming | [{'path': 'crates/opi-ai/src/callbacks.rs', 'sha256': '29189c6c2c33016ff85cc99212e2f6027f1a689624cd35952dc69b84be5df4f8', 'header': ['//! Functional request callbacks (task 20.51, AIR-059/V-51 local slice).', '//!', '//! Four call | []; historical net deletion unknown |
| 20.52 | conforming | [{'path': 'crates/opi-ai/src/retry.rs', 'sha256': '6ee0b8b45005f37866caab4c8191607bbc89607929dca1503a033a56f2ec430c', 'header': ['//! Retry/backoff/rate-limit support.', '//!', '//! Task 2.15 introduced the header parsing, exponen | []; historical net deletion unknown |
| 20.53.1 | conforming | [{'path': 'crates/opi-ai/src/message.rs', 'sha256': 'e3ed384c972dae5acadc5a41eca7db30500c90ec26437ab32f2c10492a2501b9', 'header': ['//! Provider-facing message types (S7.1).', '', 'use serde::{Deserialize, Serialize};', '', '/// D | []; historical net deletion unknown |
| 20.53.2 | conforming | [{'path': 'crates/opi-ai/src/message.rs', 'sha256': 'e3ed384c972dae5acadc5a41eca7db30500c90ec26437ab32f2c10492a2501b9', 'header': ['//! Provider-facing message types (S7.1).', '', 'use serde::{Deserialize, Serialize};', '', '/// D | []; historical net deletion unknown |
| 20.53.3 | conforming | [{'path': 'crates/opi-ai/src/provider.rs', 'sha256': '3174b522aea1586b2356feabe519804527f1a94ac6884237234e4d96b26bba92', 'header': ['//! LLM provider abstraction.', '', 'use std::pin::Pin;', 'use std::str::FromStr;', '', 'use futu | []; historical net deletion unknown |
| 20.53.4 | conforming | [{'path': 'crates/opi-ai/src/identity.rs', 'sha256': '5ddc204df7ffb05151211b7dfb33c195a7357a55d2986ce0c52d234c739801ca', 'header': ['//! Compatibility identity utilities (pinned sources:', '//! `.repo/pi-1.3.0/packages/ai/src/util | []; historical net deletion unknown |
| 20.53.6 | conforming | [{'path': 'crates/opi-ai/src/provider_headers.rs', 'sha256': '8a44eda415b95a2ebf55edf8fd9fa9c61d14825d302833bc2412e29f7d4de582', 'header': ['//! Header validation and separation for mapped provider routes.', '', 'use std::collecti | []; historical net deletion unknown |
| 20.53.8 | conforming | [{'path': 'crates/opi-ai/src/test_support.rs', 'sha256': '6a90151986b732fe697d661e3bf4ff4dc2ffc505c0e21ea5e37f5d7c8cececc2', 'header': ['//! Shared test utilities for mock-provider testing.', '//!', '//! Provides `MockProvider` fo | []; historical net deletion unknown |
| 20.54 | conforming | [{'path': 'crates/opi-ai/src/telemetry.rs', 'sha256': '9de4f0925ca6717eef05fe3fc78ec354b0be9507cdc937d4128ba8ca5bd97503', 'header': ['//! Process-local telemetry carrier: a frozen parent [`Span`] plus the', '//! source [`Dispatch` | []; historical net deletion unknown |
| 20.55 | conforming | [{'path': 'crates/opi-ai/src/telemetry.rs', 'sha256': '9de4f0925ca6717eef05fe3fc78ec354b0be9507cdc937d4128ba8ca5bd97503', 'header': ['//! Process-local telemetry carrier: a frozen parent [`Span`] plus the', '//! source [`Dispatch` | []; historical net deletion unknown |
| 20.56 | conforming | [{'path': 'crates/opi-ai/src/lib.rs', 'sha256': 'c8e9578fe152e477741979a64fb7c298f71513d4a53638223d711d631459ffdf', 'header': ['//! Unified multi-provider LLM API with streaming support.', '//!', '//! Provides a standardized inter | ['Current provider_collection.rs is builtin registration glue over Models, not a parallel state owner. ProviderCollectio; historical net deletion unknown |
| 20.57 | conforming | Lexical current-file references recorded per representative symbol; not asserted dynamically real consumers. | ['Current direct full-file review: external_sdk is in-workspace consumer-shaped; actual package runners are narrower fam; historical net deletion unknown |
| 20.58 | conforming | Lexical current-file references recorded per representative symbol; not asserted dynamically real consumers. | ['Current direct full-file review: external_sdk is in-workspace consumer-shaped; actual package runners are narrower fam; historical net deletion unknown |
| 20.59 | conforming | Lexical current-file references recorded per representative symbol; not asserted dynamically real consumers. | []; historical net deletion unknown |

## Findings

### P20-AUD-001: Builtin stored scoped key is ignored in favor of another ambient account

- Axis: security
- Severity: Major
- Conformance effect: blocks
- Requirement IDs: P20-DEL-08, P20-DEL-EXIT-01, P20-INV-001, P20-PRIN-004, P20-SDK-PROFILE-013, P20-V-12, P20-V-39
- Claim: BuiltinResolver::select keeps Credential::ApiKey.env but iterates candidate keys through self.env, which reads only base AuthContext. Actual Models→Groq Chat request sent Bearer synthetic-other-account when stored scoped GROQ_API_KEY selected synthetic-selected-account. Empty scoped values likewise fail to mask ambient.
- Evidence: `crates/opi-ai/src/auth/builtin.rs:L106–110,L183–225` - Current committed source SHA256 df76187e83211602bb5229d0284263d0c1022decfc38ca926859a52a9b8e297e
- Evidence: `Current refutation review` - Stored key itself wins (provider_collection4849), Cloudflare account/gateway scoped merging is tested, and explicit per-call keys win. Those paths avoid the env-only stored-key branch and therefore do not refute the observed wrong identity. The current counter-evidence does not observe the violating input or establish the missing complete criterion.
- Refutation attempted: Stored key itself wins (provider_collection4849), Cloudflare account/gateway scoped merging is tested, and explicit per-call keys win. Those paths avoid the env-only stored-key branch and therefore do not refute the observed wrong identity.
- Suggested closure: Establish the complete linked behavior or applicable evidence at the owning public boundary, preserving the source and authority contract.

### P20-AUD-002: Default ADC URL redirects disclose the full source query in an unsolicited Referer

- Axis: security
- Severity: Major
- Conformance effect: blocks
- Requirement IDs: P20-DEL-05, P20-DEL-08, P20-DEL-ONB-05-B, P20-MECH-AUTH-019, P20-MECH-HTTP-001, P20-MECH-HTTP-002
- Claim: Two actual loopback peers on the same host and different ports: ReqwestAdcTransport GET sends configured Authorization to source, removes it at target, and inserts Referer containing the complete source URL including subject_token=synthetic-query-canary. The public probe fails its no-unsolicited-Referer assertion.
- Evidence: `crates/opi-ai/src/auth/adc/native.rs:338-340,352-379,393-408` - HttpClientBuilder sets redirect Policy::limited but leaves reqwest0.12.28 referer=true. ReqwestAdcTransport reuses that client for every GET. Exact locked archive shows FollowRedirect adds Referer from the previous URL with userinfo/fragment removed but query retained; sensitive-header removal also includes any port change.
- Evidence: `crates/opi-ai/src/http.rs:382-399` - HttpClientBuilder sets redirect Policy::limited but leaves reqwest0.12.28 referer=true. ReqwestAdcTransport reuses that client for every GET. Exact locked archive shows FollowRedirect adds Referer from the previous URL with userinfo/fragment removed but query retained; sensitive-header removal also includes any port change.
- Evidence: `crates/opi-ai/src/auth/adc/subject.rs:111-130` - HttpClientBuilder sets redirect Policy::limited but leaves reqwest0.12.28 referer=true. ReqwestAdcTransport reuses that client for every GET. Exact locked archive shows FollowRedirect adds Referer from the previous URL with userinfo/fragment removed but query retained; sensitive-header removal also includes any port change.
- Evidence: `crates/opi-ai/src/auth/adc/external.rs:200-217,560-569` - HttpClientBuilder sets redirect Policy::limited but leaves reqwest0.12.28 referer=true. ReqwestAdcTransport reuses that client for every GET. Exact locked archive shows FollowRedirect adds Referer from the previous URL with userinfo/fragment removed but query retained; sensitive-header removal also includes any port change.
- Evidence: `Current refutation review` - adc_owner_regressions::default_url_supplier_get_follows_redirects_and_strips_cross_origin_auth passes and explicitly expects cross-port Authorization removal; http_acceptance verifies exact20-hop limits and protected-header stripping. Source security contracts prohibit credentials to unauthorized destinations. Those assertions omit Referer entirely. They prove Authorization is absent, not that no source secret escapes through a new header. Registered source contract explicitly distinguishes Gaxios/node-fetch from reqwest and RDF22 accepts parser/storage differences only; no blanket source redirect/header narrowing authorizes unsolicited Referer. The authority claim is limited to observed synthetic source-query forwarding, without alleging remote exploitation or Authorization leakage.
- Refutation attempted: adc_owner_regressions::default_url_supplier_get_follows_redirects_and_strips_cross_origin_auth passes and explicitly expects cross-port Authorization removal; http_acceptance verifies exact20-hop limits and protected-header stripping. Source security contracts prohibit credentials to unauthorized destinations.
- Suggested closure: Establish the complete linked behavior or applicable evidence at the owning public boundary, preserving the source and authority contract.

### P20-AUD-003: Non-chat results expose raw provider errors in default diagnostics

- Axis: security
- Severity: Major
- Conformance effect: blocks
- Requirement IDs: P20-CTRL-003, P20-DEL-11, P20-DEL-EXIT-01, P20-SDK-UTILITY-022, P20-V-13, P20-V-50
- Claim: Both public loopback probes reproduced default-result Debug exposure: image HTTP400 error.message echoes synthetic-private-canary; Cloudflare success:false envelope echoes synthetic-classifier-private-canary. ImagesResult.error_message and ClassifierResult.error_message copy untrusted provider text separately from their typed causes.
- Evidence: `crates/opi-ai/src/api/openrouter_images.rs:L335–341,L469–486,L509–520` - Current committed source SHA256 f65ea5abbd5b17673de4784652afea86a2921183f7eb14ccb80cbc5bac665aea
- Evidence: `crates/opi-ai/src/images.rs:L221–238` - Current committed source SHA256 1b17938fe0f3c5f337155ca689d03e8ff65bca2f9e1ec0937b35a37f951e3bb9
- Evidence: `Current refutation review` - ProviderErrorSummary::sanitized applies safe_excerpt to the typed cause (openrouter_images529/provider670–674). Normal image and protected-header tests cover wire privacy. This does not redact the separate ImagesResult.error_message assigned verbatim at images236. Five SDK probes actually executed in remaining-tests.log, all five failed. Only synthetic canaries were used.
- Refutation attempted: ProviderErrorSummary::sanitized applies safe_excerpt to the typed cause (openrouter_images529/provider670–674). Normal image and protected-header tests cover wire privacy. This does not redact the separate ImagesResult.error_message assigned verbatim at images236.
- Suggested closure: Construct default result/error diagnostics from a closed redacted summary and preserve any intentionally accessible raw data only through a separately explicit API.

### P20-AUD-004: Handle wrapped prepared-auth failures in the product remediation classifier

- Axis: integration
- Severity: Major
- Conformance effect: blocks
- Requirement IDs: P20-DEL-13, P20-DEL-14, P20-DEL-EXIT-04, P20-DEL-REPLACE-03, P20-DEL-REPLACE-04, P20-GOAL-001, P20-V-20, P20-V-42
- Claim: Prepared auth failures now carry ProviderError::StreamFailure(CredentialNeeded), but provider_auth_failure matches only direct credential variants. The common TUI/RPC/CLI remediation route therefore returns None instead of arming its existing login retry or emitting typed remediation.
- Evidence: `crates/opi-ai/src/models.rs:1843` - Auth setup errors are wrapped with projection as StreamFailure.
- Evidence: `crates/opi-coding-agent/src/harness.rs:111` - Shared provider_auth_failure does not recurse through StreamFailure.
- Evidence: `crates/opi-coding-agent/src/interactive.rs:766` - Pre-output login retry is armed only when that shared classifier returns a credential failure.
- Evidence: `Current refutation review` - Diagnostic category/code and interactive_auth::present_provider_error do recurse, and direct provider stream failures are unwrapped by ChatStream. That does not repair this separate shared remediation classifier for preparation errors. The failing continuation test stops before its subsequent steering-retention assertion; no claim that steering was lost is made. The current counter-evidence does not observe the violating input or establish the missing complete criterion.
- Refutation attempted: Diagnostic category/code and interactive_auth::present_provider_error do recurse, and direct provider stream failures are unwrapped by ChatStream. That does not repair this separate shared remediation classifier for preparation errors. The failing continuation test stops before its subsequent steering-retention assertion; no claim that steering was lost is made.
- Suggested closure: Establish the complete linked behavior or applicable evidence at the owning public boundary, preserving the source and authority contract.

### P20-AUD-005: Reject contradictory terminal stop reasons at the public collector

- Axis: invariants
- Severity: Major
- Conformance effect: blocks
- Requirement IDs: P20-DEL-02, P20-DEL-EXIT-01, P20-INV-006, P20-SDK-CORE-038, P20-V-03, P20-V-40, P20-V-41
- Claim: ChatStream accepts Done.reason=Stop with message.stop_reason=Error as Ok(AssistantMessage(Error)), violating the single typed failure contract. The same invalid boundary can flow through ChatCall.
- Evidence: `crates/opi-ai/src/stream.rs:850` - Success is classified only from event.reason and returns message unchanged; there is no consistency guard.
- Evidence: `Current refutation review` - Current stream_events tests exercise consistent Done(Error/Aborted), first-terminal absorption and collectors. Production builtins normally create consistent events; neither those tests nor that fact refutes malformed public/custom producer acceptance. The public constructor accepts the raw stream. The current counter-evidence does not observe the violating input or establish the missing complete criterion.
- Refutation attempted: Current stream_events tests exercise consistent Done(Error/Aborted), first-terminal absorption and collectors. Production builtins normally create consistent events; neither those tests nor that fact refutes malformed public/custom producer acceptance. The public constructor accepts the raw stream.
- Suggested closure: Establish the complete linked behavior or applicable evidence at the owning public boundary, preserving the source and authority contract.

### P20-AUD-006: Refuse server-required waits above the acquisition cap

- Axis: spec
- Severity: Major
- Conformance effect: blocks
- Requirement IDs: P20-SDK-UTILITY-011, P20-V-05
- Claim: A real OpenAI Chat 429 with Retry-After:120 is admitted for retry after60000ms. The required helper rule fails immediately when the server-required delay exceeds the default60s cap; clamping sends earlier than the server allows.
- Evidence: `crates/opi-ai/src/retry.rs:723` - retry_delay_ms returns hint.min(max_retry_delay_ms), then note_failure publishes a RetryAdmission.
- Evidence: `Current refutation review` - Image and classifier helpers have their own reject-over-cap implementation and tests; Smithy has an intentionally different bounded-hint rule. Neither is the generic Chat/advanced acquisition owner exercised here, and explicit cap0 must still preserve an unlimited server hint. The current counter-evidence does not observe the violating input or establish the missing complete criterion.
- Refutation attempted: Image and classifier helpers have their own reject-over-cap implementation and tests; Smithy has an intentionally different bounded-hint rule. Neither is the generic Chat/advanced acquisition owner exercised here, and explicit cap0 must still preserve an unlimited server hint.
- Suggested closure: Establish the complete linked behavior or applicable evidence at the owning public boundary, preserving the source and authority contract.

### P20-AUD-007: Veto context overflow before transient string matching

- Axis: spec
- Severity: Major
- Conformance effect: blocks
- Requirement IDs: P20-V-57
- Claim: The public whole-call classifier treats maximum context length is 500000 tokens as Transient because the broad 500 pattern matches the token limit. An enabled helper repeats an input that the required overflow veto declares terminal.
- Evidence: `crates/opi-ai/src/retry.rs:1010` - Only quota/billing/usage patterns veto before the broad transient pattern; no overflow veto is consulted.
- Evidence: `Current refutation review` - Quota-veto and source-transient fixtures pass (whole_call_retry18 current tests). The source string list contains 500, but the current selected V57 explicitly preserves overflow/auth/config exclusions, so a token count must not substitute for an HTTP status. The current counter-evidence does not observe the violating input or establish the missing complete criterion.
- Refutation attempted: Quota-veto and source-transient fixtures pass (whole_call_retry18 current tests). The source string list contains 500, but the current selected V57 explicitly preserves overflow/auth/config exclusions, so a token count must not substitute for an HTTP status.
- Suggested closure: Establish the complete linked behavior or applicable evidence at the owning public boundary, preserving the source and authority contract.

### P20-AUD-008: Simple numeric Claude omits thinking-budget addition to requested answer ceiling

- Axis: spec
- Severity: Major
- Conformance effect: blocks
- Requirement IDs: P20-DEL-10, P20-DEL-ONB-03-D, P20-SDK-MODEL-023, P20-V-39, P20-V-45
- Claim: Compiler clamps budget against the base answer ceiling without computing F=context-clamp(min(C+B,modelcap)). Current public real-Anthropic probe sends4096 where C4096+B2048 requires6144 under SDK165.
- Evidence: `crates/opi-ai/src/models.rs:L3930–3985` - Current committed source SHA256 f9fc8b51c90282772b6b8812079772ad05c264c0b7d80efbd51314a8b46e914b
- Evidence: `crates/opi-ai/src/anthropic.rs:L2554–2567` - Current committed source SHA256 984c5edfe29ac026c14654d03fbc3cd55ff3095dd2cd69c396a55995b4e40599
- Evidence: `Current refutation review` - Existing provider_collection1939–1981 budget tests cover cases already clipped to the model/context ceiling and therefore do not distinguish adding B from keeping C. Native defaults are separate and do not fix the simple path. The current counter-evidence does not observe the violating input or establish the missing complete criterion.
- Refutation attempted: Existing provider_collection1939–1981 budget tests cover cases already clipped to the model/context ceiling and therefore do not distinguish adding B from keeping C. Native defaults are separate and do not fix the simple path.
- Suggested closure: Establish the complete linked behavior or applicable evidence at the owning public boundary, preserving the source and authority contract.

### P20-AUD-009: Anthropic complete records discard active source compatibility branches

- Axis: spec
- Severity: Major
- Conformance effect: blocks
- Requirement IDs: P20-SDK-FACTORY-015, P20-SDK-FACTORY-050, P20-SDK-FACTORY-130, P20-SDK-FACTORY-135, P20-SDK-FACTORY-140, P20-SDK-FACTORY-175, P20-SDK-MODEL-040, P20-SDK-MODEL-041, P20-V-11, P20-V-49
- Claim: AnthropicMessagesCompat has only6flags; source metadata decoding omits allowEmptySignature,supportsCacheControlOnTools,supportsLongCacheRetention,supportsMidConvoEffort,allowedFallbackModels and affinity settings. Current authentic bundle contains true empty-signature overrides, false Fireworks cache overrides, nine true effort overrides and one fallback-model list. No public typed field or production caller implements these source branches.
- Evidence: `crates/opi-ai/src/model_info.rs:L349–364` - Current committed source SHA256 d1fe2274564f1a5da80950b7e11ee6e545c886b0d6fb617c607029dd8246b4d9
- Evidence: `crates/opi-ai/src/catalog/wire.rs:L844–856` - Current committed source SHA256 2eba345014122774445c938c77f3556a9d1a449098d615f5384c0a047996db17
- Evidence: `Current refutation review` - Anthropic116currenttests cover eager/adaptive/temperature/strict/inline/native options and signatures, but only the six existing compat flags. No allowedFallbackModels or mid-effort/cache-false/empty-signature override branch exists, so those passing tests cannot prove the missing capability. Custom ChatApi can implement new behavior but does not implement builtin source semantics. The current counter-evidence does not observe the violating input or establish the missing complete criterion.
- Refutation attempted: Anthropic116currenttests cover eager/adaptive/temperature/strict/inline/native options and signatures, but only the six existing compat flags. No allowedFallbackModels or mid-effort/cache-false/empty-signature override branch exists, so those passing tests cannot prove the missing capability. Custom ChatApi can implement new behavior but does not implement builtin source semantics.
- Suggested closure: Establish the complete linked behavior or applicable evidence at the owning public boundary, preserving the source and authority contract.

### P20-AUD-010: Prepared SDK sources do not resolve their scoped proxy environment

- Axis: spec
- Severity: Major
- Conformance effect: blocks
- Requirement IDs: P20-MECH-HTTP-004, P20-SDK-UTILITY-035
- Claim: Only explicit caller ProxyConfig is consumed. The sole environment helper reads process env, prefers uppercase, globally chooses HTTPS_PROXY before HTTP_PROXY and cannot select by target protocol or ALL_PROXY. Prepared auth/source environment is not connected to any alternate proxy owner; reqwest ambient discovery is disabled.
- Evidence: `crates/opi-ai/src/http.rs:370-399,426-478` - Only explicit caller ProxyConfig is consumed. The sole environment helper reads process env, prefers uppercase, globally chooses HTTPS_PROXY before HTTP_PROXY and cannot select by target protocol or ALL_PROXY. Prepared auth/source environment is not connected to any alternate proxy owner; reqwest ambient discovery is disabled.
- Evidence: `crates/opi-ai/src/provider_env.rs:5-8,24-33` - Only explicit caller ProxyConfig is consumed. The sole environment helper reads process env, prefers uppercase, globally chooses HTTPS_PROXY before HTTP_PROXY and cannot select by target protocol or ALL_PROXY. Prepared auth/source environment is not connected to any alternate proxy owner; reqwest ambient discovery is disabled.
- Evidence: `crates/opi-ai/src/provider_collection.rs:780-833` - Only explicit caller ProxyConfig is consumed. The sole environment helper reads process env, prefers uppercase, globally chooses HTTPS_PROXY before HTTP_PROXY and cannot select by target protocol or ALL_PROXY. Prepared auth/source environment is not connected to any alternate proxy owner; reqwest ambient discovery is disabled.
- Evidence: `Current refutation review` - Explicit injected proxy/no_proxy clients are supported and tested. provider_env_value can read scoped overrides but intentionally delegates proxy policy to http. These mechanisms do not implement actual source-scoped proxy resolution. Full ai/src alternate-path search found no ALL_PROXY or source-qualified proxy integration. This is a missing required production path, not merely a deficient unused convenience helper.
- Refutation attempted: Explicit injected proxy/no_proxy clients are supported and tested. provider_env_value can read scoped overrides but intentionally delegates proxy policy to http.
- Suggested closure: Establish the complete linked behavior or applicable evidence at the owning public boundary, preserving the source and authority contract.

### P20-AUD-011: Classifier declarations use sorted maps instead of caller declaration order

- Axis: spec
- Severity: Major
- Conformance effect: blocks
- Requirement IDs: P20-DEL-11, P20-SDK-CLASSIFIER-032, P20-V-24, P20-V-50
- Claim: ClassifierContext.questions and ClassifierQuestion::Choice.criteria are BTreeMaps. Source declaration order cannot be represented: llama iterates those keys to assign A/B/... and sequential question work. First-probability tie thus follows lexical keys rather than the declared first criterion.
- Evidence: `crates/opi-ai/src/classifier.rs:L43–92` - Current committed source SHA256 777f5d93091940d9304588fd481dacbf43c5d6d0c1fe802aee446ed88c7bcec3
- Evidence: `crates/opi-ai/src/api/llama.rs:question_labels/render_overview andL1047–1083` - Current committed source SHA256 2c551c540271c81e4381fbfda0fc0b99b03f992ddc63d9ec52a155bcc5e01b2c
- Evidence: `Current refutation review` - classifier_contract2110–2217 asserts correct math and wire protocol using deliberately alphabetic keys (comment2122). This proves numeric mapping for sorted inputs and does not distinguish source insertion order. Score Vec preserves order and is unaffected. The current counter-evidence does not observe the violating input or establish the missing complete criterion.
- Refutation attempted: classifier_contract2110–2217 asserts correct math and wire protocol using deliberately alphabetic keys (comment2122). This proves numeric mapping for sorted inputs and does not distinguish source insertion order. Score Vec preserves order and is unaffected.
- Suggested closure: Establish the complete linked behavior or applicable evidence at the owning public boundary, preserving the source and authority contract.

### P20-AUD-012: Fresh Models has no default in-memory credential store

- Axis: spec
- Severity: Major
- Conformance effect: blocks
- Requirement IDs: P20-DEL-09, P20-DEL-ONB-02-D, P20-SDK-CORE-004, P20-V-39
- Claim: Models::new and from_registry set credential_store=None. Registering a fake OAuth flow and invoking Models.login fails no credential store attached before interaction, contrary to the specified memory default.
- Evidence: `crates/opi-ai/src/models.rs:L609–636,L2441–2457` - Current committed source SHA256 f9fc8b51c90282772b6b8812079772ad05c264c0b7d80efbd51314a8b46e914b
- Evidence: `crates/opi-ai/src/provider_collection.rs:L719–774` - Current committed source SHA256 fce9b8186f864933ce0cf54e30a1c9612625845c7e3419ca05ffc7fecf51de32
- Evidence: `Current refutation review` - InMemoryCredentialStore is implemented (mutation592 onwards), and injected FakeStore OAuth tests establish acknowledged login/refresh semantics. Every tested successful Models OAuth flow explicitly calls set_credential_store; no constructor or register_builtin path installs that memory adapter. The dynamic_catalogs map may already supply an effective in-memory catalog, so this finding is confined to the credential-store portion rather than asserting catalog data loss.
- Refutation attempted: InMemoryCredentialStore is implemented (mutation592 onwards), and injected FakeStore OAuth tests establish acknowledged login/refresh semantics. Every tested successful Models OAuth flow explicitly calls set_credential_store; no constructor or register_builtin path installs that memory adapter.
- Suggested closure: Install the existing in-memory credential adapter by default while keeping explicit injected persistence semantics.

### P20-AUD-013: Image modalities are copied from model output instead of the source wire rule

- Axis: spec
- Severity: Major
- Conformance effect: blocks
- Requirement IDs: P20-DEL-11, P20-DEL-ONB-03-B, P20-SDK-IMAGE-002, P20-V-23, P20-V-50
- Claim: A complete public custom ImageModelRecord with output=[Text] is accepted and emits modalities=[text]. The governing wire rule requires [image,text] when text output is supported and [image] otherwise, independent of order/omission in metadata.
- Evidence: `crates/opi-ai/src/api/openrouter_images.rs:L160–186` - Current committed source SHA256 f65ea5abbd5b17673de4784652afea86a2921183f7eb14ccb80cbc5bac665aea
- Evidence: `crates/opi-ai/src/images.rs:L119–142,L179–185` - Current committed source SHA256 1b17938fe0f3c5f337155ca689d03e8ff65bca2f9e1ec0937b35a37f951e3bb9
- Evidence: `Current refutation review` - openrouter_fixtures679 verifies image-only and1335 verifies [Image,Text]. Both use input metadata already in the desired wire form, so they do not distinguish normalization from copying. Header-only preflight images179–185 imposes no output restriction. The current counter-evidence does not observe the violating input or establish the missing complete criterion.
- Refutation attempted: openrouter_fixtures679 verifies image-only and1335 verifies [Image,Text]. Both use input metadata already in the desired wire form, so they do not distinguish normalization from copying. Header-only preflight images179–185 imposes no output restriction.
- Suggested closure: Derive the wire modalities from presence of text capability and always include image in the prescribed order.

### P20-AUD-014: Response observer failure discards an already received image result and usage

- Axis: spec
- Severity: Major
- Conformance effect: blocks
- Requirement IDs: P20-DEL-11, P20-DEL-ONB-04-B, P20-SDK-IMAGE-007, P20-V-22, P20-V-25, P20-V-36, P20-V-51
- Claim: Successful nonstream JSON is fully received and parsed before onResponse. If onResponse fails or is cancelled, failure_from_error constructs an empty result with no usage,output,responseId instead of preserving received content/billing facts.
- Evidence: `crates/opi-ai/src/api/openrouter_images.rs:L350–401,L490–504` - Current committed source SHA256 f65ea5abbd5b17673de4784652afea86a2921183f7eb14ccb80cbc5bac665aea
- Evidence: `crates/opi-ai/src/images.rs:L221–238` - Current committed source SHA256 1b17938fe0f3c5f337155ca689d03e8ff65bca2f9e1ec0937b35a37f951e3bb9
- Evidence: `Current refutation review` - openrouter_fixtures1721 proves callback follows JSON and does not inherit attempt timeout, and image usage normal-path tests preserve billing data. Its valid callback fixture has choices=[] and does not assert a failing callback retains any received facts. No retry occurs on this failure, which is correct but does not recover the discarded result. The current counter-evidence does not observe the violating input or establish the missing complete criterion.
- Refutation attempted: openrouter_fixtures1721 proves callback follows JSON and does not inherit attempt timeout, and image usage normal-path tests preserve billing data. Its valid callback fixture has choices=[] and does not assert a failing callback retains any received facts. No retry occurs on this failure, which is correct but does not recover the discarded result.
- Suggested closure: Retain parsed result metadata before the observer and overlay failure/cancel state without erasing acquired facts.

### P20-AUD-015: Plain-JSON null coercion rejects floating-point zero

- Axis: spec
- Severity: Major
- Conformance effect: blocks
- Requirement IDs: P20-DEL-04, P20-MECH-SCHEMA-028, P20-MECH-TRANSCRIPT-010, P20-V-15, P20-V-35
- Claim: The current public compile/validate_tool_arguments probe accepts integer0, false and empty-string controls, then returns Invalid(type null) for0.0 and fails. The branch checks only Number::as_u64/as_i64. Exact locked serde_json1.0.149 returns None for Float from both accessors.
- Evidence: `crates/opi-ai/src/schema/normalize.rs:608-612` - The current public compile/validate_tool_arguments probe accepts integer0, false and empty-string controls, then returns Invalid(type null) for0.0 and fails. The branch checks only Number::as_u64/as_i64. Exact locked serde_json1.0.149 returns None for Float from both accessors.
- Evidence: `Current refutation review` - schema_public25 and schema_engine71 pass; the public normalizer correctly validates every normalized root, preserves raw input, and handles other primitive/optional-null/union controls. Integer0/false/empty controls pass the current audit probe. Those positive controls and ordinary fixtures do not exercise the Float Number representation of zero. This is a reachable public JSON-value case, and source TP13 explicitly includes0/-0 without restricting Number representation. The correct final validator turns the missing conversion into a visible rejection rather than hiding it.
- Refutation attempted: schema_public25 and schema_engine71 pass; the public normalizer correctly validates every normalized root, preserves raw input, and handles other primitive/optional-null/union controls. Integer0/false/empty controls pass the current audit probe.
- Suggested closure: Establish the complete linked behavior or applicable evidence at the owning public boundary, preserving the source and authority contract.

### P20-AUD-016: Update the stale compatibility and cache assertions in the current suite

- Axis: test-quality
- Severity: Major
- Conformance effect: blocks
- Requirement IDs: P20-DEL-10, P20-DEL-14, P20-DEL-EXIT-07, P20-DEL-ONB-05-A, P20-DEL-REPLACE-04, P20-V-20
- Claim: The committed opi-ai default suite has three failing tests: both legacy profile tests expect strict:true without constrained intent, and the enrichment test expects cache Disabled to suppress separately configured affinity headers. The current verification gate cannot pass.
- Evidence: `crates/opi-ai/tests/mistral_fixtures.rs:537` - mistral_profile_inherits_shared_compat_flags fails.
- Evidence: `crates/opi-ai/tests/openrouter_fixtures.rs:490` - openrouter_profile_inherits_shared_compat_flags fails.
- Evidence: `crates/opi-ai/tests/request_enrichment.rs:435` - session_affinity_wire_mappings sees session-disabled rather than None.
- Evidence: `Current refutation review` - Native Mistral strict tests pass and Chat compat documents capability versus requested intent; current SDK explicitly separates affinity from cache key. These facts refute labeling the three failures a native Mistral wire defect. They instead require meaningful assertion migration under the existing behavioral contract, without weakening required coverage. The current counter-evidence does not observe the violating input or establish the missing complete criterion.
- Refutation attempted: Native Mistral strict tests pass and Chat compat documents capability versus requested intent; current SDK explicitly separates affinity from cache key. These facts refute labeling the three failures a native Mistral wire defect. They instead require meaningful assertion migration under the existing behavioral contract, without weakening required coverage.
- Suggested closure: Establish the complete linked behavior or applicable evidence at the owning public boundary, preserving the source and authority contract.

### P20-AUD-017: Run the assembled SDK matrix as an actual external package consumer

- Axis: spec
- Severity: Major
- Conformance effect: blocks
- Requirement IDs: P20-DEL-PKG-02
- Claim: external_sdk is an in-workspace test binary and calls itself consumer-shaped. None of the package runners copies or executes that suite as an outside-workspace consumer. README package consumer covers three Anthropic simple calls, Schema consumer covers validation, HTTP/crypto consumers use advanced preparation, and WS consumer covers Codex stream_simple/cleanup. The mandated full external four-entrance/direct-custom/typed operation/serde/Send/two-instance/shared-store matrix has no current source execution path.
- Evidence: `crates/opi-ai/tests/external_sdk.rs:1` - external_sdk is an in-workspace test binary and calls itself consumer-shaped. None of the package runners copies or executes that suite as an outside-workspace consumer. README package consumer covers three Anthropic simple calls, Schema consumer covers validation, HTTP/crypto consumers use advanced preparation, and WS consumer covers Codex stream_simple/cleanup. The mandated full external four-entrance/direct-custom/typed operation/serde/Send/two-instance/shared-store matrix has no current source execution path.
- Evidence: `Current refutation review` - The current code does contain genuine external .crate consumers for README,Schema,HTTP,crypto,WS; their roles were inspected and not dismissed. Per-adapter and external_sdk internal tests can establish behavior, but source says a single manifest/internal test or unrelated complex consumer is not actual external consumption of all required seams. Searched all current scripts/pi-sdk-conformance Python/Rust/manifests for external_sdk, four ordinary entries, custom/typed operations and include/copy paths. No adapter-specific public consumer beyond the listed families supplies the missing assembled matrix.
- Refutation attempted: The current code does contain genuine external .crate consumers for README,Schema,HTTP,crypto,WS; their roles were inspected and not dismissed. Per-adapter and external_sdk internal tests can establish behavior, but source says a single manifest/internal test or unrelated complex consumer is not actual external consumption of all required seams.
- Suggested closure: Establish the complete linked behavior or applicable evidence at the owning public boundary, preserving the source and authority contract.

### P20-AUD-018: Required acceptance evidence remains unestablished at the sealed head

- Axis: runtime-fidelity
- Severity: Major
- Conformance effect: blocks
- Requirement IDs: P20-AUTH-003, P20-DEL-01, P20-DEL-03, P20-DEL-06, P20-DEL-07, P20-DEL-14, P20-DEL-EXIT-03, P20-DEL-EXIT-05, P20-DEL-EXIT-06, P20-DEL-ONB-01-B, P20-DEL-ONB-02-A, P20-DEL-ONB-02-B, P20-DEL-ONB-03-C, P20-DEL-ONB-04-C, P20-DEL-ONB-06-A, P20-DEL-ONB-07-A, P20-DEL-ONB-07-B, P20-DEL-ONB-07-C, P20-DEL-PKG-01, P20-DEL-PKG-03, P20-DEL-PKG-04, P20-DEL-PKG-05, P20-DEL-PKG-06, P20-DEL-PKG-07, P20-DEL-PLAT-01, P20-DEL-PLAT-02, P20-DEL-PLAT-03, P20-DEL-PLAT-04, P20-DEL-PROV-01, P20-DEL-PROV-02, P20-DEL-PROV-03, P20-DEL-PROV-04, P20-DEL-PROV-05, P20-DEL-PROV-06, P20-DEL-REPLACE-02, P20-DEL-REPLACE-05, P20-DEL-SCOPE-05, P20-DEL-SCOPE-06, P20-GATE-001, P20-INV-002, P20-MECH-AUTH-001, P20-MECH-AUTH-002, P20-MECH-AUTH-003, P20-MECH-AUTH-007, P20-MECH-AUTH-009, P20-MECH-AUTH-010, P20-MECH-AUTH-011, P20-MECH-AUTH-014, P20-MECH-AUTH-015, P20-MECH-AUTH-016, P20-MECH-AUTH-017, P20-MECH-AUTH-018, P20-MECH-AUTH-020, P20-MECH-AUTH-021, P20-MECH-AUTH-022, P20-MECH-AUTH-023, P20-MECH-AUTH-024, P20-MECH-AUTH-025, P20-MECH-AUTH-026, P20-MECH-DELIVERY-001, P20-MECH-DELIVERY-002, P20-MECH-DELIVERY-003, P20-MECH-DELIVERY-004, P20-MECH-DELIVERY-005, P20-MECH-DELIVERY-006, P20-MECH-DELIVERY-007, P20-MECH-DELIVERY-008, P20-MECH-DELIVERY-009, P20-MECH-DESIGN-001, P20-MECH-DESIGN-002, P20-MECH-DESIGN-003, P20-MECH-DESIGN-004, P20-MECH-HTTP-003, P20-MECH-HTTP-005, P20-MECH-HTTP-006, P20-MECH-HTTP-007, P20-MECH-HTTP-008, P20-MECH-HTTP-009, P20-MECH-HTTP-010, P20-MECH-REPLAY-016, P20-MECH-SCHEMA-005, P20-MECH-SCHEMA-011, P20-MECH-SCHEMA-016, P20-MECH-SCHEMA-017, P20-MECH-SCHEMA-021, P20-MECH-SCHEMA-025, P20-MECH-SCHEMA-031, P20-MECH-SCHEMA-032, P20-MECH-SCHEMA-033, P20-MECH-SCHEMA-034, P20-MECH-SCHEMA-035, P20-MECH-SCHEMA-036, P20-MECH-TLS-006, P20-MECH-TLS-013, P20-MECH-TLS-014, P20-MECH-WS-006, P20-MECH-WS-023, P20-MECH-WS-029, P20-MECH-WS-033, P20-MECH-WS-036, P20-MECH-WS-039, P20-MECH-WS-040, P20-MECH-WS-041, P20-PRIN-002, P20-PRIN-005, P20-SDK-AUTH-006, P20-SDK-AUTH-018, P20-SDK-AUTH-026, P20-SDK-AUTH-027, P20-SDK-CATALOG-016, P20-SDK-CLASSIFIER-023, P20-SDK-CLASSIFIER-024, P20-SDK-CORE-027, P20-SDK-CORE-028, P20-SDK-CORE-045, P20-SDK-MODEL-003, P20-SDK-MODEL-004, P20-SDK-MODEL-021, P20-SDK-MODEL-022, P20-SDK-MODEL-026, P20-SDK-MODEL-029, P20-SDK-MODEL-030, P20-SDK-MODEL-038, P20-SDK-MODEL-042, P20-SDK-PACKAGE-001, P20-SDK-PACKAGE-002, P20-SDK-PACKAGE-003, P20-SDK-PACKAGE-004, P20-SDK-PACKAGE-005, P20-SDK-PACKAGE-006, P20-SDK-PACKAGE-007, P20-SDK-PACKAGE-008, P20-SDK-PACKAGE-009, P20-SDK-PACKAGE-010, P20-SDK-PACKAGE-011, P20-SDK-PROFILE-010, P20-SDK-PROFILE-030, P20-SDK-PROFILE-033, P20-SDK-UTILITY-028, P20-SDK-UTILITY-031, P20-SDK-UTILITY-032, P20-SDK-UTILITY-034, P20-SDK-UTILITY-036, P20-V-01, P20-V-09, P20-V-16, P20-V-26, P20-V-31, P20-V-32, P20-V-34, P20-V-38, P20-V-42, P20-V-53, P20-V-56, P20-V-58, P20-V-59
- Claim: The linked mandatory criteria lack complete applicable proof. This includes current full external package consumers, native/feature-union/source-build identities, native Linux/macOS/Windows and MSRV1.97 execution, formal admission/exit receipts, and named finite-case conformance gaps detailed in each requirement record. Substantial current code and passing fixtures establish only the portions they actually observe.
- Evidence: `scripts/pi-sdk-conformance/evidence/readme-examples-proof.json` - Recorded package d4672099... and Windows rustc1.99.1 facts were inventoried and projected before reading; original artifact/log/native seals and current full closure are not present in the committed export.
- Evidence: `crates/opi-ai/tests/platform_delivery.rs:507` - Current test asserts host/metadata facts; it is not native platform execution of each required graph.
- Evidence: `scripts/pi-sdk-conformance/run_readme_examples.py:564` - A fresh audit rerun refused before packaging because the existing persistent Cargo cache marker belongs to the original worktree rather than the sealed export. No cache identity or harness rule was changed to bypass this.
- Evidence: `Current refutation review` - All73 opi-ai integration targets and library tests executed; owning fixture/assertion evidence is preserved per record. Selected prior package records bind current example/runner bytes, package guard tests82pass, and real package runner families exist. Those facts do not establish the missing original/current artifact applicability, platform/MSRV/union execution or the specifically unproved branches. No claim that historical human/native execution never occurred is made. The current counter-evidence does not observe the violating input or establish the missing complete criterion.
- Refutation attempted: All73 opi-ai integration targets and library tests executed; owning fixture/assertion evidence is preserved per record. Selected prior package records bind current example/runner bytes, package guard tests82pass, and real package runner families exist. Those facts do not establish the missing original/current artifact applicability, platform/MSRV/union execution or the specifically unproved branches. No claim that historical human/native execution never occurred is made.
- Suggested closure: Establish the complete linked behavior or applicable evidence at the owning public boundary, preserving the source and authority contract.

### P20-AUD-019: Complete chat metadata drops promptCache short/long durations

- Axis: spec
- Severity: Minor
- Conformance effect: blocks
- Requirement IDs: P20-SDK-MODEL-001, P20-SDK-MODEL-016
- Claim: ModelInfo has only cache-control capability booleans and no promptCache short/long seconds. Catalog raw retains the source object, but conversion/store encoding into an executable complete ModelInfo does not represent those durations; reading a complete owned model or restoring its typed store record cannot preserve required metadata.
- Evidence: `crates/opi-ai/src/model_info.rs:L1176–1195` - Current committed source SHA256 d1fe2274564f1a5da80950b7e11ee6e545c886b0d6fb617c607029dd8246b4d9
- Evidence: `crates/opi-ai/src/catalog/wire.rs:L748–862,L868–1015` - Current committed source SHA256 2eba345014122774445c938c77f3556a9d1a449098d615f5384c0a047996db17
- Evidence: `Current refutation review` - CatalogModel.raw keeps source bytes and source inputLimits are retained; boolean cache capability and actual cache wire behavior exist. They do not expose or roundtrip short/long cache-duration metadata in the independently usable complete chat record. The current counter-evidence does not observe the violating input or establish the missing complete criterion.
- Refutation attempted: CatalogModel.raw keeps source bytes and source inputLimits are retained; boolean cache capability and actual cache wire behavior exist. They do not expose or roundtrip short/long cache-duration metadata in the independently usable complete chat record.
- Suggested closure: Establish the complete linked behavior or applicable evidence at the owning public boundary, preserving the source and authority contract.

### P20-AUD-020: Permanent OpenRouter key is represented as an invented year9999 expiry

- Axis: spec
- Severity: Minor
- Conformance effect: blocks
- Requirement IDs: P20-SDK-AUTH-007, P20-SDK-OAUTH-OPENROUTER-004, P20-V-55
- Claim: The public credential reports Some(9999-12-31) for a permanent key and has no typed non-expiring variant. The target explicitly replaces the maximum-time sentinel with typed permanence.
- Evidence: `crates/opi-ai/src/auth/oauth/openrouter.rs:L43–53,L327` - Current committed source SHA256 16a26a03a049802d9598c02db3eac796d709186063512dbd3efb5207e7189b51
- Evidence: `crates/opi-ai/src/auth.rs:L554–580,L598–606` - Current committed source SHA256 bff7ff1a5e2575e62119861762393b8752be99898a45510d6685aed1ed462019
- Evidence: `Current refutation review` - OpenRouter refresh is correctly identity/no-network and the sentinel lies outside practical refresh windows. Thus no immediate credential loss is alleged; the defect is false expiry metadata/type contract. Existing tests assert permanence operationally and therefore do not prove the required representation. The current counter-evidence does not observe the violating input or establish the missing complete criterion.
- Refutation attempted: OpenRouter refresh is correctly identity/no-network and the sentinel lies outside practical refresh windows. Thus no immediate credential loss is alleged; the defect is false expiry metadata/type contract. Existing tests assert permanence operationally and therefore do not prove the required representation.
- Suggested closure: Represent permanent versus expiring versus unknown lifetime explicitly, or obtain an explicit contract revision for the sentinel.

### P20-AUD-021: Include below-cost quota rejection in the Chinese Bedrock contract

- Axis: standards
- Severity: Minor
- Conformance effect: blocks
- Requirement IDs: P20-DEL-EXIT-02
- Claim: English specifies that retry is blocked when capacity is below required cost5/transient10. Chinese only states that zero capacity blocks retry, omitting positive-but-insufficient capacities1..4/1..9. This is incomplete bilingual boundary wording; it does not prove Chinese explicitly permits those retries or that production is wrong.
- Evidence: `docs/research/pi-ai-opi-ai-alignment/spec-draft.zh.md:334` - English specifies that retry is blocked when capacity is below required cost5/transient10. Chinese only states that zero capacity blocks retry, omitting positive-but-insufficient capacities1..4/1..9. This is incomplete bilingual boundary wording; it does not prove Chinese explicitly permits those retries or that production is wrong.
- Evidence: `Current refutation review` - Chinese shares initial500/cost5or10 and per-call ownership and does not explicitly say only zero is rejected. The narrower sentence remains less precise than the English acceptance boundary; the finding is documentary completeness only. Read both complete current bodies. This is a direct current-source mismatch, not a product implementation inference or historical audit claim.
- Refutation attempted: Chinese shares initial500/cost5or10 and per-call ownership and does not explicitly say only zero is rejected. The narrower sentence remains less precise than the English acceptance boundary; the finding is documentary completeness only.
- Suggested closure: Establish the complete linked behavior or applicable evidence at the owning public boundary, preserving the source and authority contract.

## Verification Commands

| Command / evidence | Result | Scope |
|---|---|---|
| rotation validator; allowlisted scope reader; git archive; ancestor check | PASS | Clean admission, full committed baseline and isolated export |
| cargo test -p opi-ai --all-targets --offline --locked, then all42 remaining targets with --no-fail-fast | FAIL, original exit101 | All73 integration targets +lib:2766passed,3failed,0ignored |
| cargo test -p opi-agent --all-targets --offline --locked --no-fail-fast | FAIL, original exit101 | 34 target sections:613passed,1failed,1 intentional ignored helper |
| cargo check --workspace --all-targets --offline --locked | PASS, exit0 | Current necessary cross-crate callers compile |
| cargo fmt --check --all | PASS, exit0 | Original committed sources after removing export-only probe files |
| python scripts/opi-doc-check.py | PASS, exit0 | Export-only Windows relative alias separator adaptation; initial missing-alias failure retained separately |
| Python package/conformance guard unit suite | PASS |82passed,0failed; synthetic harness guards, not package/native proof |
|13 distinct private public probes |13failed assertions observed |12 normative counterexample cases; one stronger post-callback cancellation expectation excluded after refutation |
| run_readme_examples.py --offline --scratch <new-private> --target-dir <persistent-cache> | NOT EXECUTED past preflight, exit2 | Cache workspace identity differs for sealed export; no bypass/new session cache |
| Full workspace runtime/clippy/doc gates, feature-union native package runs, native Linux/macOS/Windows matrix and MSRV1.97 | NOT RUN as complete matrix | No universal acceptance claimed |

All current Rust acceptance commands were unfiltered; actual named target summaries/counts were inspected, including nested helper-output boundaries. No zero-test or compilation-only command establishes behavior. Tests were not run with ignored native/live selections. The existing single ignored Agent helper is explicitly counted.

| Raw check | Original exit | SHA-256 |
|---|---|---|
| adc-redirect-probe | 101 | `814261de64f2f7d43ea52e4f1c39914a5e5c81a4af78e9acef1e14358dbfb50e` |
| audit-probes | 101 | `a021802fdccdf153ce2dc39844ceac4846f1b580638be9ccd1d08857c1c215a2` |
| budget-probe-executed | 101 | `1029e9a1bfe4ce9bc1ba652ddc1a72cf2517cd0f9e97b546a6700d77b1762fae` |
| budget-probe | 101 | `82a255f43d6bc33b6820ea8662d208243355a6ef30c47726628340e356d35481` |
| cargo-fmt | 0 | `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855` |
| doc-check-alias | 0 | `058e14f4a41df91dec666fc9b4c8ab4df93998813c6c00245f71e34b50f92d32` |
| doc-check | 1 | `3855b5a29f210a25b66353e33dc275a6c2973a88ca15ccf73ee07ad8df0a9025` |
| more-probes-executed | 101 | `c361cba328186f024fa7a41b52f2faac349a4215ed2313c422c95812bbae4b8f` |
| more-probes | 101 | `25c06914b73497951838edf05520a67c8663b1325f512639438933079fd8b37a` |
| opi-agent-tests | 101 | `d9c1ab68a42e32ccaeb48b0431b7369688e1141a0d2a54c126bb587b477f0bff` |
| opi-ai-all-targets | 101 | `b6fdba842e398e8c69aaa555690e68ce4e758040cce5f4438fe4ba3cfd7eb16c` |
| readme-package | 2 | `9a670874ab11c37962319f6329635f1f9cb1a6bd8684c1279c2f76e067d2e830` |
| remaining-tests | 101 | `1f95e41088bdade855cf1267ef7255ea054d55e2882cd5cf1f0947d53de11da3` |
| workspace-check | 0 | `aecb08bc5fd3fc137c02795e77e039044e0c3e01a66ac81be585897c05273ac0` |

## Bounded Probe Reproduction

The following complete sources were added only to the isolated export, executed against unchanged committed production bytes, then removed from its crate before format checks. Save a source under its shown temporary test target name in a fresh export and run cargo test -p opi-ai --test <name> --offline --locked -- --nocapture. Exact final source bytes and raw logs are retained in the private audit base; the embedded sources make the checks reproducible without depending on the transient directory. Initial audit-harness import/helper compilation failures were repaired only in probe sources and retained separately; they are not product failures.

### audit_probe_budget

SHA-256: `391b52de77a21186d6c99efd1e1a6fd37f5b0fbda71e6e473c1ca8613506e527`

```rust
use opi_ai::provider::Request;
use std::sync::Arc;
use opi_ai::auth::{AuthScheme, StaticAuthResolver};
use opi_ai::http::HttpClient;
use opi_ai::{AuthProvenanceSource, ChatModelRecord, ModelCapabilities, ModelInfo, Models, ThinkingLevel, WireApi};
use secrecy::SecretString;
use wiremock::{Mock, MockServer, ResponseTemplate};
use wiremock::matchers::method;

#[tokio::test]
async fn simple_numeric_claude_adds_thinking_budget_to_requested_answer_budget() {
    let server=MockServer::start().await;
    Mock::given(method("POST")).respond_with(ResponseTemplate::new(200)
        .insert_header("content-type","text/event-stream").set_body_string(concat!(
            "event: message_start\ndata: {\"type\":\"message_start\",\"message\":{\"id\":\"m\",\"model\":\"numeric-fixture\",\"usage\":{\"input_tokens\":1,\"output_tokens\":0}}}\n\n",
            "event: message_delta\ndata: {\"type\":\"message_delta\",\"delta\":{\"stop_reason\":\"end_turn\"},\"usage\":{\"output_tokens\":1}}\n\n",
            "event: message_stop\ndata: {\"type\":\"message_stop\"}\n\n"
        ))).expect(1).mount(&server).await;
    let info=ModelInfo::new("numeric-fixture","Numeric fixture",WireApi::AnthropicMessages,
        ModelCapabilities::new(100_000,16_384).with_thinking(true).with_streaming(true)).with_base_url(server.uri());
    let mut models=Models::new();
    models.register_route(Box::new(opi_ai::anthropic::AnthropicProvider::for_route(
        "anthropic".into(),vec![info.clone()],Some(server.uri()),Default::default(),Arc::new(HttpClient::new()),false
    )),Arc::new(StaticAuthResolver::new(AuthScheme::ApiKey,SecretString::from("synthetic-key"))),AuthProvenanceSource::Static,Default::default()).unwrap();
    let request=Request {
        projection: Default::default(),transport: Default::default(),model:"ignored".into(),
        context:opi_ai::transcript::normalize_context(Default::default()).unwrap(),
        max_tokens:Some(4096),temperature:None,sampling_params:Default::default(),
        thinking:opi_ai::provider::ThinkingConfig { enabled:true,level:ThinkingLevel::Low,budget_tokens:Some(2048) },
        stop_sequences:vec![],metadata:None,cancel:Default::default(),timeout:None,extra_headers:vec![],
        cache_retention:Default::default(),session_id:None,telemetry_context:None,callbacks:Default::default(),
    };
    models.complete_simple(ChatModelRecord::new("anthropic",info),request,Default::default()).await.unwrap();
    let requests=server.received_requests().await.unwrap();
    let body:serde_json::Value=serde_json::from_slice(&requests[0].body).unwrap();
    assert_eq!(body["max_tokens"],serde_json::json!(6144),"SDK165 F=min(C+B,modelcap), not just C");
    assert_eq!(body["thinking"]["budget_tokens"],serde_json::json!(2048));
}
```

### audit_probe_more

SHA-256: `16be39f0cf06ecac854d6e91cf2413784028de4fd44852e454b7ff8994dd8977`

```rust
mod scoped {
use std::sync::Arc;
use opi_ai::auth::{AuthContext, AuthContextError};
use opi_ai::credential::{ApiKeyCredential, Credential, CredentialStore};
use opi_ai::http::HttpClient;
use opi_ai::provider_collection::{BuiltinOverrides, register_builtin};
use opi_ai::{ChatModelRecord, InMemoryCredentialStore, Models};
use secrecy::SecretString;
use opi_ai::provider::Request;
use wiremock::{Mock, MockServer, ResponseTemplate};
use wiremock::matchers::method;

struct InjectedAmbient;
impl AuthContext for InjectedAmbient {
    fn env(&self, name: &str) -> Result<Option<String>, AuthContextError> {
        Ok((name == "GROQ_API_KEY").then(|| "synthetic-other-account".into()))
    }
    fn file_exists(&self, _: &str) -> Result<bool, AuthContextError> { Ok(false) }
}
fn request() -> Request {
    Request {
        projection: Default::default(), transport: Default::default(), model: "ignored".into(),
        context: opi_ai::transcript::normalize_context(Default::default()).unwrap(),
        max_tokens: Some(128), temperature: None, sampling_params: Default::default(), thinking: Default::default(),
        stop_sequences: vec![], metadata: None, cancel: Default::default(), timeout: None,
        extra_headers: vec![], cache_retention: Default::default(), session_id: None,
        telemetry_context: None, callbacks: Default::default(),
    }
}

#[tokio::test]
async fn credential_scoped_env_key_must_win_on_real_adapter_wire() {
    let server = MockServer::start().await;
    Mock::given(method("POST")).respond_with(ResponseTemplate::new(200).set_body_string(
        "data: {\"choices\":[{\"index\":0,\"delta\":{\"content\":\"ok\"},\"finish_reason\":\"stop\"}]}\n\ndata: [DONE]\n\n"
    )).expect(1).mount(&server).await;
    let store = Arc::new(InMemoryCredentialStore::new());
    store.write("groq", &Credential::ApiKey(ApiKeyCredential::new(None, std::collections::BTreeMap::from([
        ("GROQ_API_KEY".into(), SecretString::from("synthetic-selected-account"))
    ])))).await.unwrap();
    let mut models = Models::new();
    models.set_credential_store(store);
    let mut overrides = BuiltinOverrides::default();
    overrides.base_urls.insert("groq".into(), server.uri());
    register_builtin(&mut models, Arc::new(InjectedAmbient), Arc::new(HttpClient::new()), &overrides).unwrap();
    let info = models.get_models(Some("groq")).remove(0);
    models.complete_simple(ChatModelRecord::new("groq", info), request(), Default::default()).await.unwrap();
    let requests = server.received_requests().await.unwrap();
    assert_eq!(requests[0].headers["authorization"], "Bearer synthetic-selected-account", "scoped stored identity must outrank the injected ambient key at the actual consumer");
}

}
mod hint {

use std::sync::Arc;
use opi_ai::http::HttpClientBuilder;
use opi_ai::openai_chat::OpenAiChatProvider;
use opi_ai::{Models,AuthScheme,StaticAuthResolver,AuthProvenanceSource,ChatStream};
use opi_ai::provider::CacheRetention;
use opi_ai::retry::AcquisitionPolicy;
use secrecy::SecretString;
use tokio_util::sync::CancellationToken;
use wiremock::{Mock,MockServer,ResponseTemplate};
use wiremock::matchers::method;
    async fn real_adapter_models(server_uri: &str) -> Models {
        let client = Arc::new(HttpClientBuilder::new().build().expect("client build"));
        let provider = OpenAiChatProvider::with_client(
            Some(server_uri.to_owned()),
            "openai".to_owned(),
            vec![],
            client,
        );
        let mut models = Models::new();
        models
            .register_route(
                Box::new(provider),
                Arc::new(StaticAuthResolver::new(
                    AuthScheme::Bearer,
                    SecretString::from("acquisition-contract-key"),
                )),
                AuthProvenanceSource::Static,
                opi_ai::models::CompatMetadata {
                    openai_compatible: true,
                    profile: None,
                },
            )
            .expect("register real adapter route");
        models
    }

    fn chat_request() -> opi_ai::provider::Request {
        opi_ai::provider::Request {
            projection: Default::default(),
            transport: Default::default(),
            model: "openai:gpt-4o".into(),
            context: opi_ai::transcript::normalize_context(opi_ai::transcript::Context {
                system_prompt: None,
                messages: vec![],
                tools: vec![],
            })
            .expect("valid context declarations"),
            max_tokens: None,
            temperature: None,
            sampling_params: Default::default(),
            thinking: Default::default(),
            stop_sequences: vec![],
            metadata: None,
            cancel: CancellationToken::new(),
            timeout: None,
            extra_headers: vec![],
            cache_retention: CacheRetention::None,
            session_id: None,
            telemetry_context: None,
            callbacks: Default::default(),
        }
    }

#[tokio::test]
async fn oversized_server_retry_hint_is_refused_instead_of_clamped() {
 let server=MockServer::start().await;
 Mock::given(method("POST")).respond_with(ResponseTemplate::new(429).insert_header("retry-after","120")).expect(1).mount(&server).await;
 let models=real_adapter_models(&server.uri()).await;
 let call=models.prepare_call_with_policy("openai:gpt-4o",chat_request(),AcquisitionPolicy::with_max_retries(1)).await.unwrap();
 let result=ChatStream::new(call.start_attempt().unwrap()).result().await;
 assert!(result.is_err());
 assert!(call.retry_admission().is_none(),"server-required120s must exceed60s cap without retry; observed {:?}",call.retry_admission());
}

}
```

### audit_probe_redirect

SHA-256: `9b3d19209896a4ab8c97ccd30dab3df9bfee9d72ded898270b5e19996ceba07d`

```rust
use opi_ai::auth::adc::native::{AdcSourceRequest, AdcSourceTransport, ReqwestAdcTransport};
use std::{io::{Read, Write}, net::{TcpListener, TcpStream}, time::{Duration, Instant}};

fn read_head(stream: &mut TcpStream) -> String {
    stream.set_read_timeout(Some(Duration::from_secs(2))).unwrap();
    let mut head = Vec::new();
    let mut byte = [0u8; 1];
    while head.len() < 16 * 1024 {
        assert_eq!(stream.read(&mut byte).unwrap(), 1, "complete request head");
        head.push(byte[0]);
        if head.ends_with(b"\r\n\r\n") { return String::from_utf8(head).unwrap(); }
    }
    panic!("bounded synthetic request head exceeded");
}
fn accept_bounded(listener: TcpListener) -> TcpStream {
    listener.set_nonblocking(true).unwrap();
    let until = Instant::now() + Duration::from_secs(4);
    loop {
        match listener.accept() {
            Ok((stream, peer)) => { assert!(peer.ip().is_loopback()); return stream; }
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                assert!(Instant::now() < until, "no request reached synthetic loopback peer");
                std::thread::sleep(Duration::from_millis(5));
            }
            Err(error) => panic!("accept: {error}"),
        }
    }
}
#[tokio::test]
async fn adc_source_redirect_does_not_forward_query_as_unsolicited_referer() {
    let source = TcpListener::bind("127.0.0.1:0").unwrap();
    let target = TcpListener::bind("127.0.0.1:0").unwrap();
    let source_addr = source.local_addr().unwrap();
    let target_addr = target.local_addr().unwrap();
    assert_ne!(source_addr.port(), target_addr.port());
    let source_job = std::thread::spawn(move || {
        let mut io = accept_bounded(source);
        let head = read_head(&mut io);
        assert!(head.contains("?subject_token=synthetic-query-canary"));
        let response = format!("HTTP/1.1 302 Found\r\nLocation: http://{target_addr}/subject\r\nContent-Length: 0\r\nConnection: close\r\n\r\n");
        io.write_all(response.as_bytes()).unwrap();
        head
    });
    let target_job = std::thread::spawn(move || {
        let mut io = accept_bounded(target);
        let head = read_head(&mut io);
        io.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 7\r\nConnection: close\r\n\r\nsubject").unwrap();
        head
    });
    let reply = ReqwestAdcTransport::new().send(AdcSourceRequest {
        method: "GET".into(),
        url: format!("http://{source_addr}/redirect?subject_token=synthetic-query-canary"),
        headers: vec![("authorization".into(), "Bearer synthetic-source-auth".into())],
        body: None,
        timeout: Some(Duration::from_secs(2)),
    }).await.unwrap();
    assert_eq!(reply.body, b"subject");
    let source_head = source_job.join().unwrap();
    let target_head = target_job.join().unwrap();
    let target_lower = target_head.to_ascii_lowercase();
    let source_has_auth = source_head.to_ascii_lowercase().contains("authorization: bearer synthetic-source-auth");
    let target_has_auth = target_lower.contains("authorization: bearer synthetic-source-auth");
    let referer = target_head.lines().find(|line| line.to_ascii_lowercase().starts_with("referer:"));
    println!("synthetic raw facts: source_auth={source_has_auth}; target_auth={target_has_auth}; target_referer={referer:?}; same_host=true; different_port=true");
    assert!(source_has_auth);
    assert!(referer.is_none(), "source-qualified Gaxios/node-fetch path must not invent a Referer forwarding the source query; observed {referer:?}");
}
```

### audit_probe_root

SHA-256: `13c73c8b819255afbf7449822906acbb4ed8bad0e7f8dd2ed2b09494e0b70078`

```rust
use opi_ai::ApiKind;
use opi_ai::message::{AssistantContent, AssistantMessage};
use opi_ai::stream::{StopReason, Usage};
use opi_ai::retry::{WholeCallRetryPolicy,WholeCallRetryCallbacks,WholeCallRetryClassification};
use std::sync::{Arc,atomic::{AtomicUsize,Ordering}};
fn message(stop: StopReason, error: Option<&str>) -> AssistantMessage {
    AssistantMessage {
        projection: None,
        content: vec![AssistantContent::Text {
            text: "partial".into(),
            signature: None,
        }],
        api: ApiKind::Anthropic,
        provider: "anthropic".into(),
        model: "claude-sonnet-4-5-20250514".into(),
        response_model: None,
        wire: None,
        response_id: None,
        usage: Usage {
            input_tokens: 3,
            output_tokens: 5,
            ..Usage::default()
        },
        stop_reason: stop,
        error_message: error.map(str::to_string),
        timestamp_ms: 1_000,
        diagnostics: Vec::new(),
        duration_ms: None,
        raw_stop_reason: None,
        provider_thinking_level: None,
        deferred: None,
    }
}


#[test]
fn audit_float_zero_null_normalization() {
 let c=opi_ai::schema::compile(serde_json::json!({"type":"null"})).unwrap();
 for raw in ["0","false","\"\"","0.0","-0.0"] {
  let input:serde_json::Value=serde_json::from_str(raw).unwrap();
  let out=opi_ai::schema::validate_tool_arguments(&c,&input,None).unwrap();
  assert!(matches!(out,opi_ai::schema::ToolArgumentOutcome::Valid{ref normalized,..} if normalized.is_null()),"{raw}: {out:?}");
 }
}
#[tokio::test]
async fn audit_attempt_start_cancel_blocks_next_produce() {
 let cancel=tokio_util::sync::CancellationToken::new();let callback_cancel=cancel.clone();
 let calls=Arc::new(AtomicUsize::new(0));let observed=calls.clone();
 let mut callbacks=WholeCallRetryCallbacks::new();
 callbacks.on_retry_attempt_start=Some(Box::new(move || { callback_cancel.cancel();Ok(()) }));
 let p=WholeCallRetryPolicy{enabled:true,max_retries:1,base_delay_ms:0,max_delay_ms:Some(0)};
 let out=opi_ai::retry_assistant_call(move || {let n=observed.fetch_add(1,Ordering::SeqCst);std::future::ready(if n==0 {message(StopReason::Error,Some("HTTP 503"))}else{message(StopReason::Stop,None)})},Some(&p),cancel,Some(callbacks)).await.unwrap();
 assert_eq!(calls.load(Ordering::SeqCst),1,"cancel after sleep allowed new produce; final={:?}",out.stop_reason);
 assert_eq!(out.stop_reason,StopReason::Aborted);
}
#[test]
fn audit_context_overflow_is_not_transient() {
 let m=message(StopReason::Error,Some("maximum context length is 500000 tokens"));
 assert_eq!(opi_ai::retry::classify_whole_call_retry(&m),WholeCallRetryClassification::Terminal);
}

#[tokio::test]
async fn audit_mismatched_terminal_reason_must_not_succeed() {
 let event=opi_ai::AssistantStreamEvent::Done{reason:StopReason::Stop,message:message(StopReason::Error,Some("failure"))};
 let out=opi_ai::ChatStream::new(Box::pin(futures_util::stream::iter(vec![Ok(event)]))).result().await;
 assert!(out.is_err(),"contradictory failure terminal was accepted as Ok: {out:?}");
}
```

### audit_probe_sdk

SHA-256: `5b5cf58c5e782600aa3eabc6574b70bda2500d4f1f9f4b481d449387534d6d51`

```rust
#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use opi_ai::api::openrouter_images::{OpenRouterImagesApi, default_openrouter_image_record};
    use opi_ai::auth::{AuthScheme, StaticAuthResolver};
    use opi_ai::callbacks::{RequestCallbacks, CallbackError};
    use opi_ai::http::HttpClient;
    use opi_ai::images::{ImageOutputModality, ImagesCall, ImagesContext, ImagesStopReason};
    use opi_ai::{Models, AuthProvenanceSource};
    use wiremock::{Mock, MockServer, ResponseTemplate};
    use wiremock::matchers::method;

    fn registration(base: String, output: Vec<ImageOutputModality>) -> (Models, opi_ai::images::ImageModelRecord) {
        let mut record = default_openrouter_image_record();
        record.base_url = base;
        record.output = output;
        let mut models = Models::new();
        models.register_image_api(
            Arc::new(OpenRouterImagesApi::new(HttpClient::new(), record.clone())),
            Arc::new(StaticAuthResolver::new(AuthScheme::Bearer, secrecy::SecretString::from("synthetic-key"))),
            AuthProvenanceSource::Static,
        ).unwrap();
        (models, record)
    }

    #[tokio::test]
    async fn image_text_output_still_requests_image_then_text() {
        let server = MockServer::start().await;
        Mock::given(method("POST")).respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"choices":[]}))).expect(1).mount(&server).await;
        let (models, record) = registration(server.uri(), vec![ImageOutputModality::Text]);
        let result = models.generate_images(record, ImagesContext::default(), ImagesCall::default()).await;
        assert_eq!(result.stop_reason, ImagesStopReason::Stop);
        let requests = server.received_requests().await.unwrap();
        let payload: serde_json::Value = serde_json::from_slice(&requests[0].body).unwrap();
        assert_eq!(payload["modalities"], serde_json::json!(["image","text"]), "SDK contract L123 fixes image modalities, independent of record order");
    }

    #[tokio::test]
    async fn image_failure_does_not_echo_provider_body_secret() {
        let server = MockServer::start().await;
        Mock::given(method("POST")).respond_with(ResponseTemplate::new(400).set_body_json(serde_json::json!({"error":{"message":"Bearer synthetic-private-canary"}}))).expect(1).mount(&server).await;
        let (models, record) = registration(server.uri(), vec![ImageOutputModality::Image]);
        let result = models.generate_images(record, ImagesContext::default(), ImagesCall::default()).await;
        assert_eq!(result.stop_reason, ImagesStopReason::Error);
        assert!(!format!("{result:?}").contains("synthetic-private-canary"), "default image error/debug must redact upstream body");
    }

    #[tokio::test]
    async fn image_response_observer_failure_retains_received_result_and_usage() {
        let server = MockServer::start().await;
        Mock::given(method("POST")).respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id":"received-image-id",
            "choices":[{"message":{"content":"received text","images":[{"image_url":{"url":"data:image/png;base64,eA=="}}]}}],
            "usage":{"prompt_tokens":10,"completion_tokens":5}
        }))).expect(1).mount(&server).await;
        let (models, record) = registration(server.uri(), vec![ImageOutputModality::Image]);
        let call = ImagesCall {
            callbacks: RequestCallbacks::new().with_on_response(Arc::new(|_,_| Err(CallbackError::new("synthetic observer failure")))),
            ..Default::default()
        };
        let result = models.generate_images(record, ImagesContext::default(), call).await;
        assert_eq!(result.stop_reason, ImagesStopReason::Error);
        assert!(result.usage.is_some(), "already received billed usage must survive observer failure");
        assert_eq!(result.response_id.as_deref(), Some("received-image-id"));
        assert_eq!(result.output.len(), 2);
    }

    struct SyntheticOAuth;
    impl opi_ai::auth::OAuthProvider for SyntheticOAuth {
        fn id(&self) -> &str { "synthetic-oauth" }
        fn login<'a>(&'a self, _: &'a dyn opi_ai::auth::LoginPresenter) -> opi_ai::credential::BoxAuthFuture<'a, Result<opi_ai::auth::OAuthCredential, opi_ai::provider::ProviderError>> {
            Box::pin(async { Ok(opi_ai::auth::OAuthCredential {
                access: secrecy::SecretString::from("synthetic-access"),
                refresh: secrecy::SecretString::from("synthetic-refresh"),
                expires_at: None,
                base_url: None,
                account_id: None,
                client_id: None,
                enterprise_domain: None,
                available_model_ids: None,
                scopes: None,
            }) })
        }
        fn refresh<'a>(&'a self, credential: &'a opi_ai::auth::OAuthCredential) -> opi_ai::credential::BoxAuthFuture<'a, Result<opi_ai::auth::OAuthCredential, opi_ai::provider::ProviderError>> {
            Box::pin(async move { Ok(credential.clone()) })
        }
    }
    struct SyntheticPresenter;
    impl opi_ai::auth::LoginPresenter for SyntheticPresenter {
        fn present_auth_url<'a>(&'a self, _: &'a str) -> opi_ai::credential::BoxAuthFuture<'a, Result<(), opi_ai::provider::ProviderError>> { Box::pin(async { Ok(()) }) }
        fn present_device_code<'a>(&'a self, _: &'a str, _: &'a str) -> opi_ai::credential::BoxAuthFuture<'a, Result<(), opi_ai::provider::ProviderError>> { Box::pin(async { Ok(()) }) }
        fn await_manual_code<'a>(&'a self) -> opi_ai::credential::BoxAuthFuture<'a, Result<String, opi_ai::provider::ProviderError>> { Box::pin(std::future::pending()) }
        fn notify_success(&self) {}
        fn notify_failure(&self, _: &str) {}
    }
    #[tokio::test]
    async fn models_login_uses_default_memory_credential_store() {
        let mut models = Models::new();
        models.register_oauth_flow("synthetic-oauth", Arc::new(SyntheticOAuth));
        let result = models.login("synthetic-oauth", &SyntheticPresenter).await;
        assert!(result.is_ok(), "SDK L19 requires default memory credentials; login must work without injection of a persistent store: {result:?}");
    }

    #[tokio::test]
    async fn cloudflare_classifier_failure_does_not_echo_envelope_secret() {
        use opi_ai::api::cloudflare::{CloudflareWorkersAiSystemOneApi, default_cloudflare_classifier_records};
        let server = MockServer::start().await;
        Mock::given(method("POST")).respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "success": false,
            "errors": [{"message":"Bearer synthetic-classifier-private-canary"}]
        }))).expect(1).mount(&server).await;
        let mut record = default_cloudflare_classifier_records().remove(0);
        record.base_url = server.uri();
        let mut models = Models::new();
        models.register_classifier_api(
            Arc::new(CloudflareWorkersAiSystemOneApi::new(HttpClient::new(), record.clone())),
            Arc::new(StaticAuthResolver::new(AuthScheme::Bearer, secrecy::SecretString::from("synthetic-key"))),
            AuthProvenanceSource::Static,
        ).unwrap();
        let result = models.classify(record, opi_ai::classifier::ClassifierContext::default(), opi_ai::classifier::ClassifierCall::default()).await;
        assert_eq!(result.stop_reason, opi_ai::classifier::ClassifierStopReason::Error);
        assert!(!format!("{result:?}").contains("synthetic-classifier-private-canary"), "default classifier error/debug must redact untrusted envelope errors");
    }
}
```

## Verdict Rationale

FAIL follows mechanically from non-met mandatory criteria, including reproduced production-path defects, current failing verification and required proof gaps. Passing fixtures establish their observed paths and do not compensate for unmet native/package/source/platform or public workflow obligations. This report is a historical assurance artifact only. Test impact: retain existing repository tests; add13 distinct private audit probes; repository production/test edits none. This new suffixed assurance artifact has no bilingual counterpart convention; current bilingual delivery-source drift is separately reported.

<!-- opi-audit-history/1
{
  "audit\u005fhead": "b3f1810801195d4e4746d9c9f0ac63d39da57313",
  "audit\u005frun\u005fid": "phase20-codex-gpt61-b3f1810-20261103t142820z",
  "comparison": {
    "comparisons": [
      {
        "cause": "This claim is absent from the immediately superseded22-finding roster under substantially the same registered sources. This is a bounded review omission classification, not proof of when code was introduced or that a formerly working repair regressed.",
        "current\u005fevidence": "Installed independent core at b3f1810801195d4e4746d9c9f0ac63d39da57313, P20-AUD-001: BuiltinResolver::select keeps Credential::ApiKey.env but iterates candidate keys through self.env, which reads only base AuthContext. Actual Models→Groq Chat request sent Bearer synthetic-other-account when stored scoped GROQ\u005fAPI\u005fKEY selected synthetic-selected-account. Empty scoped values likewise fail to mask ambient. Current owning evidence remains exactly as installed; no new core finding or verdict was compiled after historical exposure.",
        "current\u005ffinding\u005fid": "P20-AUD-001",
        "prior\u005fclaim": "No matching finding appears in the inspected immediately superseded same-pair22-finding list. No absence claim is made about any other report, reviewer, model or older history.",
        "prior\u005ffindings": [],
        "raw\u005fevidence": [],
        "relationship": "new-omission",
        "repair\u005fcommits": [],
        "verification": "Use the current finding exact public-boundary reproduction or requirement-specific applicability check at the intended remediation head. Revalidate source/dependency/material identities; history does not close the finding."
      },
      {
        "cause": "The prior defect concerned the source redirect hop count. Current20-hop controls pass; the independently reproduced unsolicited Referer/query branch violates the same source-qualified redirect/header invariant. No working repaired header state or regression commit is established.",
        "current\u005fevidence": "Installed independent core at b3f1810801195d4e4746d9c9f0ac63d39da57313, P20-AUD-002: Two actual loopback peers on the same host and different ports: ReqwestAdcTransport GET sends configured Authorization to source, removes it at target, and inserts Referer containing the complete source URL including subject\u005ftoken=synthetic-query-canary. The public probe fails its no-unsolicited-Referer assertion. Current owning evidence remains exactly as installed; no new core finding or verdict was compiled after historical exposure.",
        "current\u005ffinding\u005fid": "P20-AUD-002",
        "prior\u005fclaim": "Superseded phase20-codex-gpt61-703d8bd-20261101t181806z, P20-AUD-016, reported: Default HttpClient/OpenAI Chat accepts at most10 redirects rather than the pinned source-qualified20-redirect behavior; a lawful11-hop source request fails under ordinary SDK defaults.",
        "prior\u005ffindings": [
          {
            "audit\u005frun\u005fid": "phase20-codex-gpt61-703d8bd-20261101t181806z",
            "finding\u005fid": "P20-AUD-016",
            "source\u005fid": "prior-findings"
          }
        ],
        "raw\u005fevidence": [],
        "relationship": "missed-branch",
        "repair\u005fcommits": [],
        "verification": "Use the current finding exact public-boundary reproduction or requirement-specific applicability check at the intended remediation head. Revalidate source/dependency/material identities; history does not close the finding."
      },
      {
        "cause": "This claim is absent from the immediately superseded22-finding roster under substantially the same registered sources. This is a bounded review omission classification, not proof of when code was introduced or that a formerly working repair regressed.",
        "current\u005fevidence": "Installed independent core at b3f1810801195d4e4746d9c9f0ac63d39da57313, P20-AUD-003: Both public loopback probes reproduced default-result Debug exposure: image HTTP400 error.message echoes synthetic-private-canary; Cloudflare success:false envelope echoes synthetic-classifier-private-canary. ImagesResult.error\u005fmessage and ClassifierResult.error\u005fmessage copy untrusted provider text separately from their typed causes. Current owning evidence remains exactly as installed; no new core finding or verdict was compiled after historical exposure.",
        "current\u005ffinding\u005fid": "P20-AUD-003",
        "prior\u005fclaim": "No matching finding appears in the inspected immediately superseded same-pair22-finding list. No absence claim is made about any other report, reviewer, model or older history.",
        "prior\u005ffindings": [],
        "raw\u005fevidence": [],
        "relationship": "new-omission",
        "repair\u005fcommits": [],
        "verification": "Use the current finding exact public-boundary reproduction or requirement-specific applicability check at the intended remediation head. Revalidate source/dependency/material identities; history does not close the finding."
      },
      {
        "cause": "This claim is absent from the immediately superseded22-finding roster under substantially the same registered sources. This is a bounded review omission classification, not proof of when code was introduced or that a formerly working repair regressed.",
        "current\u005fevidence": "Installed independent core at b3f1810801195d4e4746d9c9f0ac63d39da57313, P20-AUD-004: Prepared auth failures now carry ProviderError::StreamFailure(CredentialNeeded), but provider\u005fauth\u005ffailure matches only direct credential variants. The common TUI/RPC/CLI remediation route therefore returns None instead of arming its existing login retry or emitting typed remediation. Current owning evidence remains exactly as installed; no new core finding or verdict was compiled after historical exposure.",
        "current\u005ffinding\u005fid": "P20-AUD-004",
        "prior\u005fclaim": "No matching finding appears in the inspected immediately superseded same-pair22-finding list. No absence claim is made about any other report, reviewer, model or older history.",
        "prior\u005ffindings": [],
        "raw\u005fevidence": [],
        "relationship": "new-omission",
        "repair\u005fcommits": [],
        "verification": "Use the current finding exact public-boundary reproduction or requirement-specific applicability check at the intended remediation head. Revalidate source/dependency/material identities; history does not close the finding."
      },
      {
        "cause": "This claim is absent from the immediately superseded22-finding roster under substantially the same registered sources. This is a bounded review omission classification, not proof of when code was introduced or that a formerly working repair regressed.",
        "current\u005fevidence": "Installed independent core at b3f1810801195d4e4746d9c9f0ac63d39da57313, P20-AUD-005: ChatStream accepts Done.reason=Stop with message.stop\u005freason=Error as Ok(AssistantMessage(Error)), violating the single typed failure contract. The same invalid boundary can flow through ChatCall. Current owning evidence remains exactly as installed; no new core finding or verdict was compiled after historical exposure.",
        "current\u005ffinding\u005fid": "P20-AUD-005",
        "prior\u005fclaim": "No matching finding appears in the inspected immediately superseded same-pair22-finding list. No absence claim is made about any other report, reviewer, model or older history.",
        "prior\u005ffindings": [],
        "raw\u005fevidence": [],
        "relationship": "new-omission",
        "repair\u005fcommits": [],
        "verification": "Use the current finding exact public-boundary reproduction or requirement-specific applicability check at the intended remediation head. Revalidate source/dependency/material identities; history does not close the finding."
      },
      {
        "cause": "This claim is absent from the immediately superseded22-finding roster under substantially the same registered sources. This is a bounded review omission classification, not proof of when code was introduced or that a formerly working repair regressed.",
        "current\u005fevidence": "Installed independent core at b3f1810801195d4e4746d9c9f0ac63d39da57313, P20-AUD-006: A real OpenAI Chat 429 with Retry-After:120 is admitted for retry after60000ms. The required helper rule fails immediately when the server-required delay exceeds the default60s cap; clamping sends earlier than the server allows. Current owning evidence remains exactly as installed; no new core finding or verdict was compiled after historical exposure.",
        "current\u005ffinding\u005fid": "P20-AUD-006",
        "prior\u005fclaim": "No matching finding appears in the inspected immediately superseded same-pair22-finding list. No absence claim is made about any other report, reviewer, model or older history.",
        "prior\u005ffindings": [],
        "raw\u005fevidence": [],
        "relationship": "new-omission",
        "repair\u005fcommits": [],
        "verification": "Use the current finding exact public-boundary reproduction or requirement-specific applicability check at the intended remediation head. Revalidate source/dependency/material identities; history does not close the finding."
      },
      {
        "cause": "This claim is absent from the immediately superseded22-finding roster under substantially the same registered sources. This is a bounded review omission classification, not proof of when code was introduced or that a formerly working repair regressed.",
        "current\u005fevidence": "Installed independent core at b3f1810801195d4e4746d9c9f0ac63d39da57313, P20-AUD-007: The public whole-call classifier treats maximum context length is 500000 tokens as Transient because the broad 500 pattern matches the token limit. An enabled helper repeats an input that the required overflow veto declares terminal. Current owning evidence remains exactly as installed; no new core finding or verdict was compiled after historical exposure.",
        "current\u005ffinding\u005fid": "P20-AUD-007",
        "prior\u005fclaim": "No matching finding appears in the inspected immediately superseded same-pair22-finding list. No absence claim is made about any other report, reviewer, model or older history.",
        "prior\u005ffindings": [],
        "raw\u005fevidence": [],
        "relationship": "new-omission",
        "repair\u005fcommits": [],
        "verification": "Use the current finding exact public-boundary reproduction or requirement-specific applicability check at the intended remediation head. Revalidate source/dependency/material identities; history does not close the finding."
      },
      {
        "cause": "This claim is absent from the immediately superseded22-finding roster under substantially the same registered sources. This is a bounded review omission classification, not proof of when code was introduced or that a formerly working repair regressed.",
        "current\u005fevidence": "Installed independent core at b3f1810801195d4e4746d9c9f0ac63d39da57313, P20-AUD-008: Compiler clamps budget against the base answer ceiling without computing F=context-clamp(min(C+B,modelcap)). Current public real-Anthropic probe sends4096 where C4096+B2048 requires6144 under SDK165. Current owning evidence remains exactly as installed; no new core finding or verdict was compiled after historical exposure.",
        "current\u005ffinding\u005fid": "P20-AUD-008",
        "prior\u005fclaim": "No matching finding appears in the inspected immediately superseded same-pair22-finding list. No absence claim is made about any other report, reviewer, model or older history.",
        "prior\u005ffindings": [],
        "raw\u005fevidence": [],
        "relationship": "new-omission",
        "repair\u005fcommits": [],
        "verification": "Use the current finding exact public-boundary reproduction or requirement-specific applicability check at the intended remediation head. Revalidate source/dependency/material identities; history does not close the finding."
      },
      {
        "cause": "The superseded report named Chat/Responses source-control projection loss. Current tests retain those concrete controls; this current finding concerns a different omitted Anthropic compatibility or prompt-cache metadata branch at the source-to-executable-record boundary. This is not a claim that the repaired named Chat/Responses fields were lost again.",
        "current\u005fevidence": "Installed independent core at b3f1810801195d4e4746d9c9f0ac63d39da57313, P20-AUD-009: AnthropicMessagesCompat has only6flags; source metadata decoding omits allowEmptySignature,supportsCacheControlOnTools,supportsLongCacheRetention,supportsMidConvoEffort,allowedFallbackModels and affinity settings. Current authentic bundle contains true empty-signature overrides, false Fireworks cache overrides, nine true effort overrides and one fallback-model list. No public typed field or production caller implements these source branches. Current owning evidence remains exactly as installed; no new core finding or verdict was compiled after historical exposure.",
        "current\u005ffinding\u005fid": "P20-AUD-009",
        "prior\u005fclaim": "Superseded phase20-codex-gpt61-703d8bd-20261101t181806z, P20-AUD-013, reported: The executable ModelInfo/compat projection preserves a subset of required fields and silently loses adopted Chat routing and Responses feature controls; the production Responses builder consequently sends disallowed max\u005foutput\u005ftokens and the obsolete cache wire form for declared explicit-cache models.",
        "prior\u005ffindings": [
          {
            "audit\u005frun\u005fid": "phase20-codex-gpt61-703d8bd-20261101t181806z",
            "finding\u005fid": "P20-AUD-013",
            "source\u005fid": "prior-findings"
          }
        ],
        "raw\u005fevidence": [],
        "relationship": "missed-branch",
        "repair\u005fcommits": [],
        "verification": "Use the current finding exact public-boundary reproduction or requirement-specific applicability check at the intended remediation head. Revalidate source/dependency/material identities; history does not close the finding."
      },
      {
        "cause": "This claim is absent from the immediately superseded22-finding roster under substantially the same registered sources. This is a bounded review omission classification, not proof of when code was introduced or that a formerly working repair regressed.",
        "current\u005fevidence": "Installed independent core at b3f1810801195d4e4746d9c9f0ac63d39da57313, P20-AUD-010: Only explicit caller ProxyConfig is consumed. The sole environment helper reads process env, prefers uppercase, globally chooses HTTPS\u005fPROXY before HTTP\u005fPROXY and cannot select by target protocol or ALL\u005fPROXY. Prepared auth/source environment is not connected to any alternate proxy owner; reqwest ambient discovery is disabled. Current owning evidence remains exactly as installed; no new core finding or verdict was compiled after historical exposure.",
        "current\u005ffinding\u005fid": "P20-AUD-010",
        "prior\u005fclaim": "No matching finding appears in the inspected immediately superseded same-pair22-finding list. No absence claim is made about any other report, reviewer, model or older history.",
        "prior\u005ffindings": [],
        "raw\u005fevidence": [],
        "relationship": "new-omission",
        "repair\u005fcommits": [],
        "verification": "Use the current finding exact public-boundary reproduction or requirement-specific applicability check at the intended remediation head. Revalidate source/dependency/material identities; history does not close the finding."
      },
      {
        "cause": "This claim is absent from the immediately superseded22-finding roster under substantially the same registered sources. This is a bounded review omission classification, not proof of when code was introduced or that a formerly working repair regressed.",
        "current\u005fevidence": "Installed independent core at b3f1810801195d4e4746d9c9f0ac63d39da57313, P20-AUD-011: ClassifierContext.questions and ClassifierQuestion::Choice.criteria are BTreeMaps. Source declaration order cannot be represented: llama iterates those keys to assign A/B/... and sequential question work. First-probability tie thus follows lexical keys rather than the declared first criterion. Current owning evidence remains exactly as installed; no new core finding or verdict was compiled after historical exposure.",
        "current\u005ffinding\u005fid": "P20-AUD-011",
        "prior\u005fclaim": "No matching finding appears in the inspected immediately superseded same-pair22-finding list. No absence claim is made about any other report, reviewer, model or older history.",
        "prior\u005ffindings": [],
        "raw\u005fevidence": [],
        "relationship": "new-omission",
        "repair\u005fcommits": [],
        "verification": "Use the current finding exact public-boundary reproduction or requirement-specific applicability check at the intended remediation head. Revalidate source/dependency/material identities; history does not close the finding."
      },
      {
        "cause": "This claim is absent from the immediately superseded22-finding roster under substantially the same registered sources. This is a bounded review omission classification, not proof of when code was introduced or that a formerly working repair regressed.",
        "current\u005fevidence": "Installed independent core at b3f1810801195d4e4746d9c9f0ac63d39da57313, P20-AUD-012: Models::new and from\u005fregistry set credential\u005fstore=None. Registering a fake OAuth flow and invoking Models.login fails no credential store attached before interaction, contrary to the specified memory default. Current owning evidence remains exactly as installed; no new core finding or verdict was compiled after historical exposure.",
        "current\u005ffinding\u005fid": "P20-AUD-012",
        "prior\u005fclaim": "No matching finding appears in the inspected immediately superseded same-pair22-finding list. No absence claim is made about any other report, reviewer, model or older history.",
        "prior\u005ffindings": [],
        "raw\u005fevidence": [],
        "relationship": "new-omission",
        "repair\u005fcommits": [],
        "verification": "Use the current finding exact public-boundary reproduction or requirement-specific applicability check at the intended remediation head. Revalidate source/dependency/material identities; history does not close the finding."
      },
      {
        "cause": "This claim is absent from the immediately superseded22-finding roster under substantially the same registered sources. This is a bounded review omission classification, not proof of when code was introduced or that a formerly working repair regressed.",
        "current\u005fevidence": "Installed independent core at b3f1810801195d4e4746d9c9f0ac63d39da57313, P20-AUD-013: A complete public custom ImageModelRecord with output=[Text] is accepted and emits modalities=[text]. The governing wire rule requires [image,text] when text output is supported and [image] otherwise, independent of order/omission in metadata. Current owning evidence remains exactly as installed; no new core finding or verdict was compiled after historical exposure.",
        "current\u005ffinding\u005fid": "P20-AUD-013",
        "prior\u005fclaim": "No matching finding appears in the inspected immediately superseded same-pair22-finding list. No absence claim is made about any other report, reviewer, model or older history.",
        "prior\u005ffindings": [],
        "raw\u005fevidence": [],
        "relationship": "new-omission",
        "repair\u005fcommits": [],
        "verification": "Use the current finding exact public-boundary reproduction or requirement-specific applicability check at the intended remediation head. Revalidate source/dependency/material identities; history does not close the finding."
      },
      {
        "cause": "This claim is absent from the immediately superseded22-finding roster under substantially the same registered sources. This is a bounded review omission classification, not proof of when code was introduced or that a formerly working repair regressed.",
        "current\u005fevidence": "Installed independent core at b3f1810801195d4e4746d9c9f0ac63d39da57313, P20-AUD-014: Successful nonstream JSON is fully received and parsed before onResponse. If onResponse fails or is cancelled, failure\u005ffrom\u005ferror constructs an empty result with no usage,output,responseId instead of preserving received content/billing facts. Current owning evidence remains exactly as installed; no new core finding or verdict was compiled after historical exposure.",
        "current\u005ffinding\u005fid": "P20-AUD-014",
        "prior\u005fclaim": "No matching finding appears in the inspected immediately superseded same-pair22-finding list. No absence claim is made about any other report, reviewer, model or older history.",
        "prior\u005ffindings": [],
        "raw\u005fevidence": [],
        "relationship": "new-omission",
        "repair\u005fcommits": [],
        "verification": "Use the current finding exact public-boundary reproduction or requirement-specific applicability check at the intended remediation head. Revalidate source/dependency/material identities; history does not close the finding."
      },
      {
        "cause": "This claim is absent from the immediately superseded22-finding roster under substantially the same registered sources. This is a bounded review omission classification, not proof of when code was introduced or that a formerly working repair regressed.",
        "current\u005fevidence": "Installed independent core at b3f1810801195d4e4746d9c9f0ac63d39da57313, P20-AUD-015: The current public compile/validate\u005ftool\u005farguments probe accepts integer0, false and empty-string controls, then returns Invalid(type null) for0.0 and fails. The branch checks only Number::as\u005fu64/as\u005fi64. Exact locked serde\u005fjson1.0.149 returns None for Float from both accessors. Current owning evidence remains exactly as installed; no new core finding or verdict was compiled after historical exposure.",
        "current\u005ffinding\u005fid": "P20-AUD-015",
        "prior\u005fclaim": "No matching finding appears in the inspected immediately superseded same-pair22-finding list. No absence claim is made about any other report, reviewer, model or older history.",
        "prior\u005ffindings": [],
        "raw\u005fevidence": [],
        "relationship": "new-omission",
        "repair\u005fcommits": [],
        "verification": "Use the current finding exact public-boundary reproduction or requirement-specific applicability check at the intended remediation head. Revalidate source/dependency/material identities; history does not close the finding."
      },
      {
        "cause": "This claim is absent from the immediately superseded22-finding roster under substantially the same registered sources. This is a bounded review omission classification, not proof of when code was introduced or that a formerly working repair regressed.",
        "current\u005fevidence": "Installed independent core at b3f1810801195d4e4746d9c9f0ac63d39da57313, P20-AUD-016: The committed opi-ai default suite has three failing tests: both legacy profile tests expect strict:true without constrained intent, and the enrichment test expects cache Disabled to suppress separately configured affinity headers. The current verification gate cannot pass. Current owning evidence remains exactly as installed; no new core finding or verdict was compiled after historical exposure.",
        "current\u005ffinding\u005fid": "P20-AUD-016",
        "prior\u005fclaim": "No matching finding appears in the inspected immediately superseded same-pair22-finding list. No absence claim is made about any other report, reviewer, model or older history.",
        "prior\u005ffindings": [],
        "raw\u005fevidence": [],
        "relationship": "new-omission",
        "repair\u005fcommits": [],
        "verification": "Use the current finding exact public-boundary reproduction or requirement-specific applicability check at the intended remediation head. Revalidate source/dependency/material identities; history does not close the finding."
      },
      {
        "cause": "Prior018 concerned exact README-source onboarding execution and manual first-use setup. Current source has a real same-source runner and recorded3-flow observations, while current017 concerns the larger full SDK package-consumer matrix. The scopes differ; no same-defect recurrence or regression is inferred.",
        "current\u005fevidence": "Installed independent core at b3f1810801195d4e4746d9c9f0ac63d39da57313, P20-AUD-017: external\u005fsdk is an in-workspace test binary and calls itself consumer-shaped. None of the package runners copies or executes that suite as an outside-workspace consumer. README package consumer covers three Anthropic simple calls, Schema consumer covers validation, HTTP/crypto consumers use advanced preparation, and WS consumer covers Codex stream\u005fsimple/cleanup. The mandated full external four-entrance/direct-custom/typed operation/serde/Send/two-instance/shared-store matrix has no current source execution path. Current owning evidence remains exactly as installed; no new core finding or verdict was compiled after historical exposure.",
        "current\u005ffinding\u005fid": "P20-AUD-017",
        "prior\u005fclaim": "Superseded phase20-codex-gpt61-703d8bd-20261101t181806z, P20-AUD-018, reported: The actual-package onboarding harness stops at cargo check; controlled-peer executions use different workspace test code, and the first-use examples still require manual route/resolver assembly. This does not meet ONB01/02 same-source ordinary package-consumer acceptance.",
        "prior\u005ffindings": [
          {
            "audit\u005frun\u005fid": "phase20-codex-gpt61-703d8bd-20261101t181806z",
            "finding\u005fid": "P20-AUD-018",
            "source\u005fid": "prior-findings"
          }
        ],
        "raw\u005fevidence": [],
        "relationship": "insufficient-information",
        "repair\u005fcommits": [],
        "verification": "Use the current finding exact public-boundary reproduction or requirement-specific applicability check at the intended remediation head. Revalidate source/dependency/material identities; history does not close the finding."
      },
      {
        "cause": "Both runs leave complete package/material applicability unestablished. Prior020 specifically attributed valid3OS/MSRV CI proof at its own source identity; that is not rewritten as never executed. Current code and proof boundaries differ, and the full original/current artifact/native/source identities were not verified in this audit. Prior019 Stream-proof concerns are not asserted repaired solely from new test names or Future proof.",
        "current\u005fevidence": "Installed independent core at b3f1810801195d4e4746d9c9f0ac63d39da57313, P20-AUD-018: The linked mandatory criteria lack complete applicable proof. This includes current full external package consumers, native/feature-union/source-build identities, native Linux/macOS/Windows and MSRV1.97 execution, formal admission/exit receipts, and named finite-case conformance gaps detailed in each requirement record. Substantial current code and passing fixtures establish only the portions they actually observe. Current owning evidence remains exactly as installed; no new core finding or verdict was compiled after historical exposure.",
        "current\u005ffinding\u005fid": "P20-AUD-018",
        "prior\u005fclaim": "Superseded phase20-codex-gpt61-703d8bd-20261101t181806z, P20-AUD-018, reported: The actual-package onboarding harness stops at cargo check; controlled-peer executions use different workspace test code, and the first-use examples still require manual route/resolver assembly. This does not meet ONB01/02 same-source ordinary package-consumer acceptance. Superseded phase20-codex-gpt61-703d8bd-20261101t181806z, P20-AUD-019, reported: Mandatory Stream Pending/cross-thread/unpolled-Drop propagation is not established by the current carrier/operation suite. Production telemetry has only a Future wrapper; the Stream test supplies its own in\u005fscope implementation and can pass without a production Stream carrier path. Superseded phase20-codex-gpt61-703d8bd-20261101t181806z, P20-AUD-020, reported: Applicable3OS workspace runtime and MSRV compilation proof exists, but supplied records do not provide original current full-SDK ordinary/default/C-union package-consumer artifacts/logs and native-build provenance needed to assess the mandatory final package/platform closure. Separately, the current catalog artifact and packer bytes match their manifest, but original upstream pin/helper/acquisition material seals needed to establish source association are unavailable; this does not allege invalid catalog runtime content.",
        "prior\u005ffindings": [
          {
            "audit\u005frun\u005fid": "phase20-codex-gpt61-703d8bd-20261101t181806z",
            "finding\u005fid": "P20-AUD-018",
            "source\u005fid": "prior-findings"
          },
          {
            "audit\u005frun\u005fid": "phase20-codex-gpt61-703d8bd-20261101t181806z",
            "finding\u005fid": "P20-AUD-019",
            "source\u005fid": "prior-findings"
          },
          {
            "audit\u005frun\u005fid": "phase20-codex-gpt61-703d8bd-20261101t181806z",
            "finding\u005fid": "P20-AUD-020",
            "source\u005fid": "prior-findings"
          }
        ],
        "raw\u005fevidence": [],
        "relationship": "evidence-gap",
        "repair\u005fcommits": [],
        "verification": "Use the current finding exact public-boundary reproduction or requirement-specific applicability check at the intended remediation head. Revalidate source/dependency/material identities; history does not close the finding."
      },
      {
        "cause": "The superseded report named Chat/Responses source-control projection loss. Current tests retain those concrete controls; this current finding concerns a different omitted Anthropic compatibility or prompt-cache metadata branch at the source-to-executable-record boundary. This is not a claim that the repaired named Chat/Responses fields were lost again.",
        "current\u005fevidence": "Installed independent core at b3f1810801195d4e4746d9c9f0ac63d39da57313, P20-AUD-019: ModelInfo has only cache-control capability booleans and no promptCache short/long seconds. Catalog raw retains the source object, but conversion/store encoding into an executable complete ModelInfo does not represent those durations; reading a complete owned model or restoring its typed store record cannot preserve required metadata. Current owning evidence remains exactly as installed; no new core finding or verdict was compiled after historical exposure.",
        "current\u005ffinding\u005fid": "P20-AUD-019",
        "prior\u005fclaim": "Superseded phase20-codex-gpt61-703d8bd-20261101t181806z, P20-AUD-013, reported: The executable ModelInfo/compat projection preserves a subset of required fields and silently loses adopted Chat routing and Responses feature controls; the production Responses builder consequently sends disallowed max\u005foutput\u005ftokens and the obsolete cache wire form for declared explicit-cache models.",
        "prior\u005ffindings": [
          {
            "audit\u005frun\u005fid": "phase20-codex-gpt61-703d8bd-20261101t181806z",
            "finding\u005fid": "P20-AUD-013",
            "source\u005fid": "prior-findings"
          }
        ],
        "raw\u005fevidence": [],
        "relationship": "missed-branch",
        "repair\u005fcommits": [],
        "verification": "Use the current finding exact public-boundary reproduction or requirement-specific applicability check at the intended remediation head. Revalidate source/dependency/material identities; history does not close the finding."
      },
      {
        "cause": "This claim is absent from the immediately superseded22-finding roster under substantially the same registered sources. This is a bounded review omission classification, not proof of when code was introduced or that a formerly working repair regressed.",
        "current\u005fevidence": "Installed independent core at b3f1810801195d4e4746d9c9f0ac63d39da57313, P20-AUD-020: The public credential reports Some(9999-12-31) for a permanent key and has no typed non-expiring variant. The target explicitly replaces the maximum-time sentinel with typed permanence. Current owning evidence remains exactly as installed; no new core finding or verdict was compiled after historical exposure.",
        "current\u005ffinding\u005fid": "P20-AUD-020",
        "prior\u005fclaim": "No matching finding appears in the inspected immediately superseded same-pair22-finding list. No absence claim is made about any other report, reviewer, model or older history.",
        "prior\u005ffindings": [],
        "raw\u005fevidence": [],
        "relationship": "new-omission",
        "repair\u005fcommits": [],
        "verification": "Use the current finding exact public-boundary reproduction or requirement-specific applicability check at the intended remediation head. Revalidate source/dependency/material identities; history does not close the finding."
      },
      {
        "cause": "This claim is absent from the immediately superseded22-finding roster under substantially the same registered sources. This is a bounded review omission classification, not proof of when code was introduced or that a formerly working repair regressed.",
        "current\u005fevidence": "Installed independent core at b3f1810801195d4e4746d9c9f0ac63d39da57313, P20-AUD-021: English specifies that retry is blocked when capacity is below required cost5/transient10. Chinese only states that zero capacity blocks retry, omitting positive-but-insufficient capacities1..4/1..9. This is incomplete bilingual boundary wording; it does not prove Chinese explicitly permits those retries or that production is wrong. Current owning evidence remains exactly as installed; no new core finding or verdict was compiled after historical exposure.",
        "current\u005ffinding\u005fid": "P20-AUD-021",
        "prior\u005fclaim": "No matching finding appears in the inspected immediately superseded same-pair22-finding list. No absence claim is made about any other report, reviewer, model or older history.",
        "prior\u005ffindings": [],
        "raw\u005fevidence": [],
        "relationship": "new-omission",
        "repair\u005fcommits": [],
        "verification": "Use the current finding exact public-boundary reproduction or requirement-specific applicability check at the intended remediation head. Revalidate source/dependency/material identities; history does not close the finding."
      },
      {
        "cause": "The current implementation/previously executed owning assertions establish the specific old predicate correction. No exact repair commit or broader causal history is established from the permitted single superseded record; no old closure label is adopted as proof.",
        "current\u005fevidence": "At sealed head b3f1810801195d4e4746d9c9f0ac63d39da57313: crates/opi-ai/src/bedrock/event\u005fstream.rs:27-42 now checks preludeCRC/header relation before waiting for declared total; current invalid\u005fprelude\u005fis\u005frejected\u005fbefore\u005fpayload\u005fwait and valid\u005fprelude\u005fremains\u005fbuffered\u005funtil\u005fframe\u005fcomplete both executed and passed.",
        "current\u005ffinding\u005fid": null,
        "prior\u005fclaim": "Superseded phase20-codex-gpt61-703d8bd-20261101t181806z, P20-AUD-001, reported: The AWS parser accepts a known-invalid prelude as incomplete until its declared total length arrives. Bad prelude CRC or headers exceeding total minus16 can cause retention up to the declared approximately4GiB before rejection or EOF/cancellation, instead of early protocol rejection.",
        "prior\u005ffindings": [
          {
            "audit\u005frun\u005fid": "phase20-codex-gpt61-703d8bd-20261101t181806z",
            "finding\u005fid": "P20-AUD-001",
            "source\u005fid": "prior-findings"
          }
        ],
        "raw\u005fevidence": [],
        "relationship": "preserved-repair",
        "repair\u005fcommits": [],
        "verification": "Keep early-invalid/valid-incomplete controls; no new payload cap is implied."
      },
      {
        "cause": "The current implementation/previously executed owning assertions establish the specific old predicate correction. No exact repair commit or broader causal history is established from the permitted single superseded record; no old closure label is adopted as proof.",
        "current\u005fevidence": "At sealed head b3f1810801195d4e4746d9c9f0ac63d39da57313: Current typed url::Host guard at auth/aws/metadata.rs117-139 accepts literal loopback/link-local or exact allowed localhost, not prefix domains. Current ecs\u005ffull\u005furi\u005fparsed\u005fhost\u005fguard\u005fruns\u005fbefore\u005ftoken\u005ffile\u005for\u005fsend passed through the builtin source path.",
        "current\u005ffinding\u005fid": null,
        "prior\u005fclaim": "Superseded phase20-codex-gpt61-703d8bd-20261101t181806z, P20-AUD-002, reported: A configured http://127.attacker.invalid/creds or http://169.254.attacker.invalid/creds passes the link-local/loopback guard; the public Models builtin resolver issues one source request carrying Authorization to that DNS domain.",
        "prior\u005ffindings": [
          {
            "audit\u005frun\u005fid": "phase20-codex-gpt61-703d8bd-20261101t181806z",
            "finding\u005fid": "P20-AUD-002",
            "source\u005fid": "prior-findings"
          }
        ],
        "raw\u005fevidence": [],
        "relationship": "preserved-repair",
        "repair\u005fcommits": [],
        "verification": "Retain parsed domain/IP alternate-form and before-token-I/O zero-send controls."
      },
      {
        "cause": "The current implementation/previously executed owning assertions establish the specific old predicate correction. No exact repair commit or broader causal history is established from the permitted single superseded record; no old closure label is adopted as proof.",
        "current\u005fevidence": "At sealed head b3f1810801195d4e4746d9c9f0ac63d39da57313: Current Chat, Responses-shared and Gemini multiline\u005fframes\u005fsurvive\u005fevery\u005fline\u005fending\u005fsplit passed, plus actual HTTP multiline\u005fsse\u005fline\u005fendings\u005fsurvive\u005fhttp\u005fchunk\u005fsplits fixtures. The prior split CR/LF multiline predicate is now discriminated.",
        "current\u005ffinding\u005fid": null,
        "prior\u005fclaim": "Superseded phase20-codex-gpt61-703d8bd-20261101t181806z, P20-AUD-003, reported: Incremental drains convert a trailing CR to LF before the following LF arrives, creating a false blank line and parsing an incomplete legal multiline JSON event.",
        "prior\u005ffindings": [
          {
            "audit\u005frun\u005fid": "phase20-codex-gpt61-703d8bd-20261101t181806z",
            "finding\u005fid": "P20-AUD-003",
            "source\u005fid": "prior-findings"
          }
        ],
        "raw\u005fevidence": [],
        "relationship": "preserved-repair",
        "repair\u005fcommits": [],
        "verification": "Keep each affected wire every-boundary multiline fixture, not only single-line UTF8 controls."
      },
      {
        "cause": "The current implementation/previously executed owning assertions establish the specific old predicate correction. No exact repair commit or broader causal history is established from the permitted single superseded record; no old closure label is adopted as proof.",
        "current\u005fevidence": "At sealed head b3f1810801195d4e4746d9c9f0ac63d39da57313: Current deferred\u005fcontract::remote\u005fcancel\u005fobserves\u005fcaller\u005fabort\u005fbefore\u005fand\u005fduring\u005fauth executed and passed. The current cancel operation preserves identity and distinct explicit remote-cancel semantics; this predicate is separate from ordinary whole-call delay cancellation.",
        "current\u005ffinding\u005fid": null,
        "prior\u005fclaim": "Superseded phase20-codex-gpt61-703d8bd-20261101t181806z, P20-AUD-004, reported: Models::cancel\u005fdeferred awaits an uncancellable resolver before reaching the backend, so a pre-aborted call starts source work and never settles if the resolver remains pending.",
        "prior\u005ffindings": [
          {
            "audit\u005frun\u005fid": "phase20-codex-gpt61-703d8bd-20261101t181806z",
            "finding\u005fid": "P20-AUD-004",
            "source\u005fid": "prior-findings"
          }
        ],
        "raw\u005fevidence": [],
        "relationship": "preserved-repair",
        "repair\u005fcommits": [],
        "verification": "Retain pre-aborted and pending-auth cancellation with zero backend effect; keep store-owned rotation independent."
      },
      {
        "cause": "The current implementation/previously executed owning assertions establish the specific old predicate correction. No exact repair commit or broader causal history is established from the permitted single superseded record; no old closure label is adopted as proof.",
        "current\u005fevidence": "At sealed head b3f1810801195d4e4746d9c9f0ac63d39da57313: Current public\u005futilities::context\u005foverflow\u005flength\u005fstop\u005fhandles\u005ffull\u005fu64\u005fwindow\u005frange executed and passed with the public large-window controls. Separate overflow-pattern/knownness tests also pass. This repair is independent of current whole-call retry misclassification P20-AUD-007.",
        "current\u005ffinding\u005fid": null,
        "prior\u005fclaim": "Superseded phase20-codex-gpt61-703d8bd-20261101t181806z, P20-AUD-005, reported: is\u005fcontext\u005foverflow uses unchecked window\u002a99 in its Length/output0 branch. For reported input\u005ftokens1 and context\u005fwindow186330748219288401, the accepted u64 argument causes an overflow panic instead of returning false.",
        "prior\u005ffindings": [
          {
            "audit\u005frun\u005fid": "phase20-codex-gpt61-703d8bd-20261101t181806z",
            "finding\u005fid": "P20-AUD-005",
            "source\u005fid": "prior-findings"
          }
        ],
        "raw\u005fevidence": [],
        "relationship": "preserved-repair",
        "repair\u005fcommits": [],
        "verification": "Retain u64-extreme/knownness/ratio controls without source-free input caps."
      },
      {
        "cause": "The current implementation/previously executed owning assertions establish the specific old predicate correction. No exact repair commit or broader causal history is established from the permitted single superseded record; no old closure label is adopted as proof.",
        "current\u005fevidence": "At sealed head b3f1810801195d4e4746d9c9f0ac63d39da57313: Current schema\u005fengine and schema\u005fpublic lifecycle tests compile/check/drop10,000-deep schemas, exercise root$id, subvalidator clone and compile-error release on explicit2MiB threads. Three direct child executions were separately captured with original exit0 and checker-confirmed1passed/0failed/0ignored each for this historical comparison.",
        "current\u005ffinding\u005fid": null,
        "prior\u005fclaim": "Superseded phase20-codex-gpt61-703d8bd-20261101t181806z, P20-AUD-006, reported: A valid10000-deep allOf schema aborts the process on ordinary2MiB Rust thread stacks: root$id triggers recursive anchor admission; otherwise compile/check succeeds but public normalization and dropping CompiledSchema each stack-overflow. Main evaluator stack safety does not extend across the public owned schema lifecycle.",
        "prior\u005ffindings": [
          {
            "audit\u005frun\u005fid": "phase20-codex-gpt61-703d8bd-20261101t181806z",
            "finding\u005fid": "P20-AUD-006",
            "source\u005fid": "prior-findings"
          }
        ],
        "raw\u005fevidence": [
          {
            "checked": "Exact current sealed-baseline executable; originalexit0; expected named case1passed0failed0ignored; execution checker PASS; explicit2MiB child stack. Binary/log/checker digests retained.",
            "limitations": "Windows current toolchain fixture proof only; not platform/MSRV/native package acceptance. Independent core remained byte-identical.",
            "path": "C:\\Users\\Luiz\\AppData\\Local\\Temp\\opi-phase20-audit-nc3hj\u005fmw\\checks\\history-deep\u005fschemas\u005fdo\u005fnot\u005fexhaust\u005fthe\u005frust\u005fstack.log",
            "sha256": "5e36aaa8a44c1f78a07c6ad294d1037385d80702f9e39f915324f3e9dcf521bb"
          },
          {
            "checked": "Exact current sealed-baseline executable; originalexit0; expected named case1passed0failed0ignored; execution checker PASS; explicit2MiB child stack. Binary/log/checker digests retained.",
            "limitations": "Windows current toolchain fixture proof only; not platform/MSRV/native package acceptance. Independent core remained byte-identical.",
            "path": "C:\\Users\\Luiz\\AppData\\Local\\Temp\\opi-phase20-audit-nc3hj\u005fmw\\checks\\history-deep\u005fcompile\u005ferror\u005freleases\u005fowned\u005fschema.log",
            "sha256": "657ca34138932acc224aa0130660a14f124d0000402f7031fd2437477048d468"
          },
          {
            "checked": "Exact current sealed-baseline executable; originalexit0; expected named case1passed0failed0ignored; execution checker PASS; explicit2MiB child stack. Binary/log/checker digests retained.",
            "limitations": "Windows current toolchain fixture proof only; not platform/MSRV/native package acceptance. Independent core remained byte-identical.",
            "path": "C:\\Users\\Luiz\\AppData\\Local\\Temp\\opi-phase20-audit-nc3hj\u005fmw\\checks\\history-deep\u005fschema\u005fnormalization\u005fand\u005fsubvalidator\u005fclone\u005frelease\u005fnormally.log",
            "sha256": "81b22f5c58cc8f18f946e96e104948bfabb5a32abb52e4f634f65ad7fe0f8c45"
          }
        ],
        "relationship": "preserved-repair",
        "repair\u005fcommits": [],
        "verification": "Retain normal and error destruction, reference registration and normalization on ordinary stacks; no mem::forget success is accepted."
      },
      {
        "cause": "The current implementation/previously executed owning assertions establish the specific old predicate correction. No exact repair commit or broader causal history is established from the permitted single superseded record; no old closure label is adopted as proof.",
        "current\u005fevidence": "At sealed head b3f1810801195d4e4746d9c9f0ac63d39da57313: Current metadata\u005fchecked\u005fexpiry, service\u005faccount\u005fchecked\u005fexpiry, authorized\u005fuser\u005fchecked\u005fexpiry, external\u005fauthorized\u005fuser\u005fchecked\u005fexpiry and external-account STS extreme-expiry fixtures all passed with ordinary and invalid-expiry controls.",
        "current\u005ffinding\u005fid": null,
        "prior\u005fclaim": "Superseded phase20-codex-gpt61-703d8bd-20261101t181806z, P20-AUD-007, reported: A successful selected ADC metadata/token response with positive i64::MAX expires\u005fin passes field parsing and panics during OffsetDateTime addition rather than returning typed failure.",
        "prior\u005ffindings": [
          {
            "audit\u005frun\u005fid": "phase20-codex-gpt61-703d8bd-20261101t181806z",
            "finding\u005fid": "P20-AUD-007",
            "source\u005fid": "prior-findings"
          }
        ],
        "raw\u005fevidence": [],
        "relationship": "preserved-repair",
        "repair\u005fcommits": [],
        "verification": "Retain each ADC family extreme/wrong-type/ordinary expiry boundary and typed no-send failure."
      },
      {
        "cause": "The current implementation/previously executed owning assertions establish the specific old predicate correction. No exact repair commit or broader causal history is established from the permitted single superseded record; no old closure label is adopted as proof.",
        "current\u005fevidence": "At sealed head b3f1810801195d4e4746d9c9f0ac63d39da57313: Current adc\u005fowner\u005fregressions::aws\u005fsubject\u005fregion\u005fsuffix\u005fand\u005fenvironment\u005fprecedence and the ordinary external\u005faws\u005fimds\u005fflow\u005fwith\u005fv2\u005ftoken\u005fand\u005fregion\u005fstrip executed and passed, covering source region decoding and precedence rather than the old unchecked byte slice.",
        "current\u005ffinding\u005fid": null,
        "prior\u005fclaim": "Superseded phase20-codex-gpt61-703d8bd-20261101t181806z, P20-AUD-008, reported: The external-account aws1 supplier removes one byte from arbitrary decoded metadata rather than one validated region suffix; an otherwise successful region response ending in a multibyte character panics.",
        "prior\u005ffindings": [
          {
            "audit\u005frun\u005fid": "phase20-codex-gpt61-703d8bd-20261101t181806z",
            "finding\u005fid": "P20-AUD-008",
            "source\u005fid": "prior-findings"
          }
        ],
        "raw\u005fevidence": [],
        "relationship": "preserved-repair",
        "repair\u005fcommits": [],
        "verification": "Retain malformed Unicode/ASCII source-response controls and selected environment precedence."
      },
      {
        "cause": "The current implementation/previously executed owning assertions establish the specific old predicate correction. No exact repair commit or broader causal history is established from the permitted single superseded record; no old closure label is adopted as proof.",
        "current\u005fevidence": "At sealed head b3f1810801195d4e4746d9c9f0ac63d39da57313: Current Chat and Responses each pass duration\u005fincludes\u005fpending\u005ffirst\u005fbody\u005fpoll, duration\u005fexcludes\u005fauth\u005fsetup\u005fcallback\u005fand\u005funpolled\u005ftime and untimed setup-failure tests. Azure and Google controls also pass, without substituting their implementations for Chat/Responses.",
        "current\u005ffinding\u005fid": null,
        "prior\u005fclaim": "Superseded phase20-codex-gpt61-703d8bd-20261101t181806z, P20-AUD-009, reported: Chat and Responses initialize activation only after a body chunk has arrived, excluding the time already spent polling the response body. AIR-061/RDF-02 require response-stream activation/first-poll timing.",
        "prior\u005ffindings": [
          {
            "audit\u005frun\u005fid": "phase20-codex-gpt61-703d8bd-20261101t181806z",
            "finding\u005fid": "P20-AUD-009",
            "source\u005fid": "prior-findings"
          }
        ],
        "raw\u005fevidence": [],
        "relationship": "preserved-repair",
        "repair\u005fcommits": [],
        "verification": "Retain owning wire activation/auth/body-wait/collector duration assertions."
      },
      {
        "cause": "The current implementation/previously executed owning assertions establish the specific old predicate correction. No exact repair commit or broader causal history is established from the permitted single superseded record; no old closure label is adopted as proof.",
        "current\u005fevidence": "At sealed head b3f1810801195d4e4746d9c9f0ac63d39da57313: Current accepted\u005fprojection\u005fsurvives\u005fauth\u005fcallback\u005fpartial\u005feof\u005fand\u005fcancel\u005ffailures, strict\u005fprojection\u005frejects\u005fbefore\u005fauth\u005fcallbacks\u005fand\u005fnetwork, and projection\u005fpolicy\u005fand\u005ffacts\u005ffollow\u005factual\u005fnative\u005fsimple\u005fdirect\u005fand\u005fprepared\u005fcalls passed.",
        "current\u005ffinding\u005fid": null,
        "prior\u005fclaim": "Superseded phase20-codex-gpt61-703d8bd-20261101t181806z, P20-AUD-010, reported: Actual adapters unconditionally choose NativeOrCollapse and discard replay.facts; chronological collapse returns only Cow\u003cTranscriptContext\u003e, with no mode/affected range/source-prefix facts. Strict helper has no selected-fidelity ingress in actual call preparation.",
        "prior\u005ffindings": [
          {
            "audit\u005frun\u005fid": "phase20-codex-gpt61-703d8bd-20261101t181806z",
            "finding\u005fid": "P20-AUD-010",
            "source\u005fid": "prior-findings"
          }
        ],
        "raw\u005fevidence": [],
        "relationship": "preserved-repair",
        "repair\u005fcommits": [],
        "verification": "Keep selected fidelity pre-auth admission, frozen facts and terminal/partial/failure propagation through actual owning paths."
      },
      {
        "cause": "The current implementation/previously executed owning assertions establish the specific old predicate correction. No exact repair commit or broader causal history is established from the permitted single superseded record; no old closure label is adopted as proof.",
        "current\u005fevidence": "At sealed head b3f1810801195d4e4746d9c9f0ac63d39da57313: Current context\u005fbudget::independently\u005freported\u005ftotal\u005fwins\u005fwith\u005fknown\u005for\u005funknown\u005fparent\u005fbuckets and independent\u005ftotal\u005fstill\u005frequires\u005fa\u005fsuccessful\u005ftimely\u005fresponse executed and passed.",
        "current\u005ffinding\u005fid": null,
        "prior\u005fclaim": "Superseded phase20-codex-gpt61-703d8bd-20261101t181806z, P20-AUD-011, reported: The canonical estimator ignores an independently known positive total when the parent-bucket reported flag is false, substantially underestimating valid continuation context and simple output budgets.",
        "prior\u005ffindings": [
          {
            "audit\u005frun\u005fid": "phase20-codex-gpt61-703d8bd-20261101t181806z",
            "finding\u005fid": "P20-AUD-011",
            "source\u005fid": "prior-findings"
          }
        ],
        "raw\u005fevidence": [],
        "relationship": "preserved-repair",
        "repair\u005fcommits": [],
        "verification": "Retain independent-total anchor and required success/timestamp controls separately from fallback parent sum."
      },
      {
        "cause": "The current implementation/previously executed owning assertions establish the specific old predicate correction. No exact repair commit or broader causal history is established from the permitted single superseded record; no old closure label is adopted as proof.",
        "current\u005fevidence": "At sealed head b3f1810801195d4e4746d9c9f0ac63d39da57313: Current custom\u005fprovider\u005fregistration passes arbitrary exact ApiId controls for chat/image/classifier/deferred, including the formerly refused forms, with no auth. Provider-ID validation remains a different contract.",
        "current\u005ffinding\u005fid": null,
        "prior\u005fclaim": "Superseded phase20-codex-gpt61-703d8bd-20261101t181806z, P20-AUD-012, reported: All4 Models API registration seams reject empty and whitespace-only exact ApiId values even though registered source explicitly disallows adding nonempty/trim grammar.",
        "prior\u005ffindings": [
          {
            "audit\u005frun\u005fid": "phase20-codex-gpt61-703d8bd-20261101t181806z",
            "finding\u005fid": "P20-AUD-012",
            "source\u005fid": "prior-findings"
          }
        ],
        "raw\u005fevidence": [],
        "relationship": "preserved-repair",
        "repair\u005fcommits": [],
        "verification": "Retain all four exact API identity admission paths, options portability and real type mismatch rejection."
      },
      {
        "cause": "The current implementation/previously executed owning assertions establish the specific old predicate correction. No exact repair commit or broader causal history is established from the permitted single superseded record; no old closure label is adopted as proof.",
        "current\u005fevidence": "At sealed head b3f1810801195d4e4746d9c9f0ac63d39da57313: Current catalog projection\u005fcompat\u005ftests preserve Chat optional false/inference/nested values and Responses absence/false. Current model/actual-adapter fixtures exercise the named max-output/sign-in/cache/tool controls. This row is limited to prior013 named Chat/Responses branches; current009/019 document different retained omissions.",
        "current\u005ffinding\u005fid": null,
        "prior\u005fclaim": "Superseded phase20-codex-gpt61-703d8bd-20261101t181806z, P20-AUD-013, reported: The executable ModelInfo/compat projection preserves a subset of required fields and silently loses adopted Chat routing and Responses feature controls; the production Responses builder consequently sends disallowed max\u005foutput\u005ftokens and the obsolete cache wire form for declared explicit-cache models.",
        "prior\u005ffindings": [
          {
            "audit\u005frun\u005fid": "phase20-codex-gpt61-703d8bd-20261101t181806z",
            "finding\u005fid": "P20-AUD-013",
            "source\u005fid": "prior-findings"
          }
        ],
        "raw\u005fevidence": [],
        "relationship": "preserved-repair",
        "repair\u005fcommits": [],
        "verification": "Retain named wire false/omission/explicit-cache controls and real consumers, not only metadata field existence."
      },
      {
        "cause": "The current implementation/previously executed owning assertions establish the specific old predicate correction. No exact repair commit or broader causal history is established from the permitted single superseded record; no old closure label is adopted as proof.",
        "current\u005fevidence": "At sealed head b3f1810801195d4e4746d9c9f0ac63d39da57313: Current selected\u005fowner\u005fmanagement tests pass removal/re-registration/clear with frozen old calls and namespaces, selected force refresh before source work, subset/tombstone preservation, and actual all-operation owner lists.",
        "current\u005ffinding\u005fid": null,
        "prior\u005fclaim": "Superseded phase20-codex-gpt61-703d8bd-20261101t181806z, P20-AUD-014, reported: The SDK cannot remove/clear a provider, enumerate/ask availability of all operation kinds including images, or choose/force one provider refresh through any public Models action.",
        "prior\u005ffindings": [
          {
            "audit\u005frun\u005fid": "phase20-codex-gpt61-703d8bd-20261101t181806z",
            "finding\u005fid": "P20-AUD-014",
            "source\u005fid": "prior-findings"
          }
        ],
        "raw\u005fevidence": [],
        "relationship": "preserved-repair",
        "repair\u005fcommits": [],
        "verification": "Retain public action/selection-before-I/O and old-call/isolation controls; default memory store current012 remains distinct."
      },
      {
        "cause": "The current implementation/previously executed owning assertions establish the specific old predicate correction. No exact repair commit or broader causal history is established from the permitted single superseded record; no old closure label is adopted as proof.",
        "current\u005fevidence": "At sealed head b3f1810801195d4e4746d9c9f0ac63d39da57313: SpawnedCredentialProcess now derives Default with policy:Option=None; only with\u005fpolicy selects10s/64KiB limits. Current default\u005frunner\u005faccepts\u005flarge\u005fvalid\u005foutput and no-policy child lifecycle controls passed. CredentialProcessPolicy::default still defines an explicit policy and is not the runner omission default.",
        "current\u005ffinding\u005fid": null,
        "prior\u005fclaim": "Superseded phase20-codex-gpt61-703d8bd-20261101t181806z, P20-AUD-015, reported: Default builtin AWS source assembly still chooses a runner enforcing10seconds and64KiB stdout without caller opt-in, although AIR-023 withdraws these SDK parity defaults. Valid larger/slower credential processes are rejected as discovery failures and can select a later source.",
        "prior\u005ffindings": [
          {
            "audit\u005frun\u005fid": "phase20-codex-gpt61-703d8bd-20261101t181806z",
            "finding\u005fid": "P20-AUD-015",
            "source\u005fid": "prior-findings"
          }
        ],
        "raw\u005fevidence": [],
        "relationship": "preserved-repair",
        "repair\u005fcommits": [],
        "verification": "Retain actual default source assembly, large valid output and cancellation/reap controls with caller-only budgets."
      },
      {
        "cause": "The current implementation/previously executed owning assertions establish the specific old predicate correction. No exact repair commit or broader causal history is established from the permitted single superseded record; no old closure label is adopted as proof.",
        "current\u005fevidence": "At sealed head b3f1810801195d4e4746d9c9f0ac63d39da57313: Current HttpClient RedirectPolicy::default is Limited20 and source\u005fqualified\u005fdefault\u005fredirect\u005fbudget\u005fhas\u005fexact\u005ftwenty\u005fhop\u005fboundary executed and passed. This preserves the hop-count correction only; current002 separately reports unsolicited header/query forwarding.",
        "current\u005ffinding\u005fid": null,
        "prior\u005fclaim": "Superseded phase20-codex-gpt61-703d8bd-20261101t181806z, P20-AUD-016, reported: Default HttpClient/OpenAI Chat accepts at most10 redirects rather than the pinned source-qualified20-redirect behavior; a lawful11-hop source request fails under ordinary SDK defaults.",
        "prior\u005ffindings": [
          {
            "audit\u005frun\u005fid": "phase20-codex-gpt61-703d8bd-20261101t181806z",
            "finding\u005fid": "P20-AUD-016",
            "source\u005fid": "prior-findings"
          }
        ],
        "raw\u005fevidence": [],
        "relationship": "preserved-repair",
        "repair\u005fcommits": [],
        "verification": "Retain exact20-success/21-failure send counts and source-specific header/method/audience controls."
      },
      {
        "cause": "The current implementation/previously executed owning assertions establish the specific old predicate correction. No exact repair commit or broader causal history is established from the permitted single superseded record; no old closure label is adopted as proof.",
        "current\u005fevidence": "At sealed head b3f1810801195d4e4746d9c9f0ac63d39da57313: Current strict\u005frequired\u005fshape\u005fis\u005fvalidated\u005fwith\u005for\u005fwithout\u005fproperties, malformed\u005frequired\u005fis\u005ftyped\u005fand\u005fpreserves\u005fthe\u005ftrusted\u005fschema and malformed\u005frequired\u005ffails\u005fbefore\u005fnative\u005fauthentication\u005fand\u005fdispatch executed and passed.",
        "current\u005ffinding\u005fid": null,
        "prior\u005fclaim": "Superseded phase20-codex-gpt61-703d8bd-20261101t181806z, P20-AUD-017, reported: make\u005fstrict\u005fjson\u005fschema accepts required:\"a\",required:1 and required:{} for an object properties map and silently replaces them with all property names plus nullable wrappers, although TP14 requires non-string-array required to fail. Require intent therefore can pass this malformed shape into real wire compilation.",
        "prior\u005ffindings": [
          {
            "audit\u005frun\u005fid": "phase20-codex-gpt61-703d8bd-20261101t181806z",
            "finding\u005fid": "P20-AUD-017",
            "source\u005fid": "prior-findings"
          }
        ],
        "raw\u005fevidence": [],
        "relationship": "preserved-repair",
        "repair\u005fcommits": [],
        "verification": "Retain scalar/object/array wrong-shape and prefer/require actual-wire controls; do not confuse local inert-schema semantics with strict conversion."
      },
      {
        "cause": "The current implementation/previously executed owning assertions establish the specific old predicate correction. No exact repair commit or broader causal history is established from the permitted single superseded record; no old closure label is adopted as proof.",
        "current\u005fevidence": "At sealed head b3f1810801195d4e4746d9c9f0ac63d39da57313: Current transcript\u005fcontract::normalization\u005frejects\u005fduplicate\u005fsections\u005fin\u005finitial\u005fand\u005flater\u005fmessages, duplicate\u005fsection\u005fappend\u005fpreserves\u005fentire\u005fprior\u005ftranscript, and tool\u005ferrors\u005fprecede\u005fduplicate\u005fsection\u005frejection executed and passed. JSON-object parser last-value behavior is a different ingress.",
        "current\u005ffinding\u005fid": null,
        "prior\u005fclaim": "Superseded phase20-codex-gpt61-703d8bd-20261101t181806z, P20-AUD-021, reported: Transcript normalization,Serde message validation and append validate only tools\u005fadded. Programmatic SystemMessage.sections can contain duplicate names and is silently folded by last value, though adopted TP-02 requires ambiguous input rejection.",
        "prior\u005ffindings": [
          {
            "audit\u005frun\u005fid": "phase20-codex-gpt61-703d8bd-20261101t181806z",
            "finding\u005fid": "P20-AUD-021",
            "source\u005fid": "prior-findings"
          }
        ],
        "raw\u005fevidence": [],
        "relationship": "preserved-repair",
        "repair\u005fcommits": [],
        "verification": "Retain programmatic Vec duplicate rejection before installation and atomic failed append."
      },
      {
        "cause": "The current implementation/previously executed owning assertions establish the specific old predicate correction. No exact repair commit or broader causal history is established from the permitted single superseded record; no old closure label is adopted as proof.",
        "current\u005fevidence": "At sealed head b3f1810801195d4e4746d9c9f0ac63d39da57313: Current schema\u005fengine cardinality\u005fmin\u005fproperties\u005fboundaries, cardinality\u005fmax\u005fproperties\u005fboundaries, cardinality\u005fmin\u005fitems\u005fboundaries and cardinality\u005fmax\u005fitems\u005fboundaries all executed and passed.",
        "current\u005ffinding\u005fid": null,
        "prior\u005fclaim": "Superseded phase20-codex-gpt61-703d8bd-20261101t181806z, P20-AUD-022, reported: The required complete42-mechanism corpus lacks positive/negative cardinality acceptance for minProperties/maxProperties and rejecting-bound cases for minItems/maxItems. Their implementation exists but current supplied tests do not discriminate removing these four checks.",
        "prior\u005ffindings": [
          {
            "audit\u005frun\u005fid": "phase20-codex-gpt61-703d8bd-20261101t181806z",
            "finding\u005fid": "P20-AUD-022",
            "source\u005fid": "prior-findings"
          }
        ],
        "raw\u005fevidence": [],
        "relationship": "preserved-repair",
        "repair\u005fcommits": [],
        "verification": "Retain B-1/B/B+ rejection/acceptance controls so removal of each leaf check is discriminated."
      }
    ],
    "limitations": "Independent core, metadata, requirement/finding sidecars and verdict were frozen by successful complete before any historical content was read. This advisory suffix changes none of them.\nNo proven regression is asserted. Matching broad titles/invariants does not establish a previously repaired working state followed by a later failing state; repair\u005fcommits remain empty when not established.\nPrior020 attributes valid3OS runtime/MSRV proof to an earlier CI source identity. Its original raw CI logs/artifacts and current complete source/dependency/feature applicability were not re-fetched or revalidated here. Do not reinterpret that attribution as never executed, or use it to close current native/package/material evidence gaps.\nPrior018 same-source onboarding now has a real execution path and recorded3-flow facts; complete original/current package proof remains limited. Current017 full-SDK external matrix is a different predicate, so no same-defect recurrence is claimed.\nPrior019 questioned test-only Stream scope proof. Current independent records cite expanded Pending/cross-thread/unpolled tests, explicit production carrier helpers and actual producer propagation; these observations alone do not establish whether the prior generic production-Stream-wrapper interpretation is required. No preserved-repair or regression claim is made for prior019; a fresh independent contract-specific review can resolve that bounded lead.\nThree named ordinary-stack child cases were rerun solely for historical repair verification with inventories/ignored inventories and execution-checker-confirmed counts; their raw hashes are attached to old006. They did not revise the installed independent core or initiate live/native-platform acceptance trials.\nAll unpaired current findings are bounded omissions from this one prior roster. Every other historical report, audit pair, older appendix and remediation record remains uninspected.",
    "schema": "opi-audit-history/1",
    "scope": "Only the immediately superseded codex/gpt61 run phase20-codex-gpt61-703d8bd-20261101t181806z at audit head703d8bd8f5033e5ecc89fc1950fc6b7615b64068, whose exact four records this complete moved to history. Each source was matched byte-for-byte to commit8010ed2af97cd5deeb1f7240097436fdf32b8d54 that last carried it. Prior report older appendix, all other runs/pairs and unrelated remediation remain excluded. New-omission means absent from this22-finding roster, not code introduced after it.",
    "sources": [
      {
        "audit\u005frun\u005fid": "phase20-codex-gpt61-703d8bd-20261101t181806z",
        "commit": "8010ed2af97cd5deeb1f7240097436fdf32b8d54",
        "id": "prior-meta",
        "path": "docs/snapshots/phase20/assurance/audit.codex.gpt61.meta.json",
        "sha256": "2e3f257a65a5b9a4215bfb54bfadc9ed9014c51742851781ce579a6e6bf4a6cc"
      },
      {
        "audit\u005frun\u005fid": "phase20-codex-gpt61-703d8bd-20261101t181806z",
        "commit": "8010ed2af97cd5deeb1f7240097436fdf32b8d54",
        "id": "prior-requirements",
        "path": "docs/snapshots/phase20/assurance/audit.codex.gpt61.requirements.jsonl",
        "sha256": "688dcd613cb65eec466fd4aa3583369b26a29bfc2a14c772159f503211b2f177"
      },
      {
        "audit\u005frun\u005fid": "phase20-codex-gpt61-703d8bd-20261101t181806z",
        "commit": "8010ed2af97cd5deeb1f7240097436fdf32b8d54",
        "id": "prior-findings",
        "path": "docs/snapshots/phase20/assurance/audit.codex.gpt61.findings.jsonl",
        "sha256": "1c60c2f6c88d4d5318822c7412fb4a3fd5d48287253bc661d36ba91e7c2a62ef"
      },
      {
        "audit\u005frun\u005fid": "phase20-codex-gpt61-703d8bd-20261101t181806z",
        "commit": "8010ed2af97cd5deeb1f7240097436fdf32b8d54",
        "id": "prior-report",
        "path": "docs/snapshots/phase20/assurance/audit.codex.gpt61.md",
        "sha256": "11950b30586154137615d09446c5895bdfb27765ab24d0972ece41dc049ac8ac"
      }
    ],
    "status": "compared"
  },
  "core": {
    "findings\u005fsha256": "4cec97af8ef6bb773c630ee251728f31212e12fdc1bf9631f10b2dc3084bc0f6",
    "meta\u005fsha256": "c32046ece08058e59e274ddd2bb37276edc21ca9e296355f8908d505ba51a5e4",
    "report\u005fbytes": 364772,
    "report\u005fsha256": "9c22247af09d749291e961145ba26ff80ad3c90e7cd129cf3487084762e5b48b",
    "requirements\u005fsha256": "c2600889ab81ce63688c346e9d7d3bf2a1e0fdbfc09daeba2b08f42b50c69898"
  },
  "schema": "opi-audit-history/1"
}
-->

## Post-audit historical reference — advisory

This section was appended after installation of the independent core. It cannot change findings, requirements, independence or verdict. Historical closure claims require current independent verification.

- Owning audit run: phase20&#45;codex&#45;gpt61&#45;b3f1810&#45;20261103t142820z
- Audit head: b3f1810801195d4e4746d9c9f0ac63d39da57313
- Core report bytes: 364772
- Core report SHA-256: 9c22247af09d749291e961145ba26ff80ad3c90e7cd129cf3487084762e5b48b
- Current findings SHA-256: 4cec97af8ef6bb773c630ee251728f31212e12fdc1bf9631f10b2dc3084bc0f6
- Comparison status: compared
- Inspected history: Only the immediately superseded codex/gpt61 run phase20&#45;codex&#45;gpt61&#45;703d8bd&#45;20261101t181806z at audit head703d8bd8f5033e5ecc89fc1950fc6b7615b64068, whose exact four records this complete moved to history&#46; Each source was matched byte&#45;for&#45;byte to commit8010ed2af97cd5deeb1f7240097436fdf32b8d54 that last carried it&#46; Prior report older appendix, all other runs/pairs and unrelated remediation remain excluded&#46; New&#45;omission means absent from this22&#45;finding roster, not code introduced after it&#46;
- Limitations: Independent core, metadata, requirement/finding sidecars and verdict were frozen by successful complete before any historical content was read&#46; This advisory suffix changes none of them&#46;<br>No proven regression is asserted&#46; Matching broad titles/invariants does not establish a previously repaired working state followed by a later failing state; repair&#95;commits remain empty when not established&#46;<br>Prior020 attributes valid3OS runtime/MSRV proof to an earlier CI source identity&#46; Its original raw CI logs/artifacts and current complete source/dependency/feature applicability were not re&#45;fetched or revalidated here&#46; Do not reinterpret that attribution as never executed, or use it to close current native/package/material evidence gaps&#46;<br>Prior018 same&#45;source onboarding now has a real execution path and recorded3&#45;flow facts; complete original/current package proof remains limited&#46; Current017 full&#45;SDK external matrix is a different predicate, so no same&#45;defect recurrence is claimed&#46;<br>Prior019 questioned test&#45;only Stream scope proof&#46; Current independent records cite expanded Pending/cross&#45;thread/unpolled tests, explicit production carrier helpers and actual producer propagation; these observations alone do not establish whether the prior generic production&#45;Stream&#45;wrapper interpretation is required&#46; No preserved&#45;repair or regression claim is made for prior019; a fresh independent contract&#45;specific review can resolve that bounded lead&#46;<br>Three named ordinary&#45;stack child cases were rerun solely for historical repair verification with inventories/ignored inventories and execution&#45;checker&#45;confirmed counts; their raw hashes are attached to old006&#46; They did not revise the installed independent core or initiate live/native&#45;platform acceptance trials&#46;<br>All unpaired current findings are bounded omissions from this one prior roster&#46; Every other historical report, audit pair, older appendix and remediation record remains uninspected&#46;

### Pinned historical sources

- prior&#45;meta: docs/snapshots/phase20/assurance/audit&#46;codex&#46;gpt61&#46;meta&#46;json; commit 8010ed2af97cd5deeb1f7240097436fdf32b8d54; SHA-256 2e3f257a65a5b9a4215bfb54bfadc9ed9014c51742851781ce579a6e6bf4a6cc; audit run phase20&#45;codex&#45;gpt61&#45;703d8bd&#45;20261101t181806z
- prior&#45;requirements: docs/snapshots/phase20/assurance/audit&#46;codex&#46;gpt61&#46;requirements&#46;jsonl; commit 8010ed2af97cd5deeb1f7240097436fdf32b8d54; SHA-256 688dcd613cb65eec466fd4aa3583369b26a29bfc2a14c772159f503211b2f177; audit run phase20&#45;codex&#45;gpt61&#45;703d8bd&#45;20261101t181806z
- prior&#45;findings: docs/snapshots/phase20/assurance/audit&#46;codex&#46;gpt61&#46;findings&#46;jsonl; commit 8010ed2af97cd5deeb1f7240097436fdf32b8d54; SHA-256 1c60c2f6c88d4d5318822c7412fb4a3fd5d48287253bc661d36ba91e7c2a62ef; audit run phase20&#45;codex&#45;gpt61&#45;703d8bd&#45;20261101t181806z
- prior&#45;report: docs/snapshots/phase20/assurance/audit&#46;codex&#46;gpt61&#46;md; commit 8010ed2af97cd5deeb1f7240097436fdf32b8d54; SHA-256 11950b30586154137615d09446c5895bdfb27765ab24d0972ece41dc049ac8ac; audit run phase20&#45;codex&#45;gpt61&#45;703d8bd&#45;20261101t181806z

### Comparison 1: P20&#45;AUD&#45;001

- Relationship: new&#45;omission
- Prior claim (attributed): No matching finding appears in the inspected immediately superseded same&#45;pair22&#45;finding list&#46; No absence claim is made about any other report, reviewer, model or older history&#46;
- Current evidence: Installed independent core at b3f1810801195d4e4746d9c9f0ac63d39da57313, P20&#45;AUD&#45;001: BuiltinResolver::select keeps Credential::ApiKey&#46;env but iterates candidate keys through self&#46;env, which reads only base AuthContext&#46; Actual Models→Groq Chat request sent Bearer synthetic&#45;other&#45;account when stored scoped GROQ&#95;API&#95;KEY selected synthetic&#45;selected&#45;account&#46; Empty scoped values likewise fail to mask ambient&#46; Current owning evidence remains exactly as installed; no new core finding or verdict was compiled after historical exposure&#46;
- Cause / missed boundary: This claim is absent from the immediately superseded22&#45;finding roster under substantially the same registered sources&#46; This is a bounded review omission classification, not proof of when code was introduced or that a formerly working repair regressed&#46;
- Suggested current verification: Use the current finding exact public&#45;boundary reproduction or requirement&#45;specific applicability check at the intended remediation head&#46; Revalidate source/dependency/material identities; history does not close the finding&#46;

### Comparison 2: P20&#45;AUD&#45;002

- Relationship: missed&#45;branch
- Prior claim (attributed): Superseded phase20&#45;codex&#45;gpt61&#45;703d8bd&#45;20261101t181806z, P20&#45;AUD&#45;016, reported: Default HttpClient/OpenAI Chat accepts at most10 redirects rather than the pinned source&#45;qualified20&#45;redirect behavior; a lawful11&#45;hop source request fails under ordinary SDK defaults&#46;
- Current evidence: Installed independent core at b3f1810801195d4e4746d9c9f0ac63d39da57313, P20&#45;AUD&#45;002: Two actual loopback peers on the same host and different ports: ReqwestAdcTransport GET sends configured Authorization to source, removes it at target, and inserts Referer containing the complete source URL including subject&#95;token=synthetic&#45;query&#45;canary&#46; The public probe fails its no&#45;unsolicited&#45;Referer assertion&#46; Current owning evidence remains exactly as installed; no new core finding or verdict was compiled after historical exposure&#46;
- Cause / missed boundary: The prior defect concerned the source redirect hop count&#46; Current20&#45;hop controls pass; the independently reproduced unsolicited Referer/query branch violates the same source&#45;qualified redirect/header invariant&#46; No working repaired header state or regression commit is established&#46;
- Suggested current verification: Use the current finding exact public&#45;boundary reproduction or requirement&#45;specific applicability check at the intended remediation head&#46; Revalidate source/dependency/material identities; history does not close the finding&#46;
- Prior finding: phase20&#45;codex&#45;gpt61&#45;703d8bd&#45;20261101t181806z / P20&#45;AUD&#45;016; source prior&#45;findings

### Comparison 3: P20&#45;AUD&#45;003

- Relationship: new&#45;omission
- Prior claim (attributed): No matching finding appears in the inspected immediately superseded same&#45;pair22&#45;finding list&#46; No absence claim is made about any other report, reviewer, model or older history&#46;
- Current evidence: Installed independent core at b3f1810801195d4e4746d9c9f0ac63d39da57313, P20&#45;AUD&#45;003: Both public loopback probes reproduced default&#45;result Debug exposure: image HTTP400 error&#46;message echoes synthetic&#45;private&#45;canary; Cloudflare success:false envelope echoes synthetic&#45;classifier&#45;private&#45;canary&#46; ImagesResult&#46;error&#95;message and ClassifierResult&#46;error&#95;message copy untrusted provider text separately from their typed causes&#46; Current owning evidence remains exactly as installed; no new core finding or verdict was compiled after historical exposure&#46;
- Cause / missed boundary: This claim is absent from the immediately superseded22&#45;finding roster under substantially the same registered sources&#46; This is a bounded review omission classification, not proof of when code was introduced or that a formerly working repair regressed&#46;
- Suggested current verification: Use the current finding exact public&#45;boundary reproduction or requirement&#45;specific applicability check at the intended remediation head&#46; Revalidate source/dependency/material identities; history does not close the finding&#46;

### Comparison 4: P20&#45;AUD&#45;004

- Relationship: new&#45;omission
- Prior claim (attributed): No matching finding appears in the inspected immediately superseded same&#45;pair22&#45;finding list&#46; No absence claim is made about any other report, reviewer, model or older history&#46;
- Current evidence: Installed independent core at b3f1810801195d4e4746d9c9f0ac63d39da57313, P20&#45;AUD&#45;004: Prepared auth failures now carry ProviderError::StreamFailure&#40;CredentialNeeded&#41;, but provider&#95;auth&#95;failure matches only direct credential variants&#46; The common TUI/RPC/CLI remediation route therefore returns None instead of arming its existing login retry or emitting typed remediation&#46; Current owning evidence remains exactly as installed; no new core finding or verdict was compiled after historical exposure&#46;
- Cause / missed boundary: This claim is absent from the immediately superseded22&#45;finding roster under substantially the same registered sources&#46; This is a bounded review omission classification, not proof of when code was introduced or that a formerly working repair regressed&#46;
- Suggested current verification: Use the current finding exact public&#45;boundary reproduction or requirement&#45;specific applicability check at the intended remediation head&#46; Revalidate source/dependency/material identities; history does not close the finding&#46;

### Comparison 5: P20&#45;AUD&#45;005

- Relationship: new&#45;omission
- Prior claim (attributed): No matching finding appears in the inspected immediately superseded same&#45;pair22&#45;finding list&#46; No absence claim is made about any other report, reviewer, model or older history&#46;
- Current evidence: Installed independent core at b3f1810801195d4e4746d9c9f0ac63d39da57313, P20&#45;AUD&#45;005: ChatStream accepts Done&#46;reason=Stop with message&#46;stop&#95;reason=Error as Ok&#40;AssistantMessage&#40;Error&#41;&#41;, violating the single typed failure contract&#46; The same invalid boundary can flow through ChatCall&#46; Current owning evidence remains exactly as installed; no new core finding or verdict was compiled after historical exposure&#46;
- Cause / missed boundary: This claim is absent from the immediately superseded22&#45;finding roster under substantially the same registered sources&#46; This is a bounded review omission classification, not proof of when code was introduced or that a formerly working repair regressed&#46;
- Suggested current verification: Use the current finding exact public&#45;boundary reproduction or requirement&#45;specific applicability check at the intended remediation head&#46; Revalidate source/dependency/material identities; history does not close the finding&#46;

### Comparison 6: P20&#45;AUD&#45;006

- Relationship: new&#45;omission
- Prior claim (attributed): No matching finding appears in the inspected immediately superseded same&#45;pair22&#45;finding list&#46; No absence claim is made about any other report, reviewer, model or older history&#46;
- Current evidence: Installed independent core at b3f1810801195d4e4746d9c9f0ac63d39da57313, P20&#45;AUD&#45;006: A real OpenAI Chat 429 with Retry&#45;After:120 is admitted for retry after60000ms&#46; The required helper rule fails immediately when the server&#45;required delay exceeds the default60s cap; clamping sends earlier than the server allows&#46; Current owning evidence remains exactly as installed; no new core finding or verdict was compiled after historical exposure&#46;
- Cause / missed boundary: This claim is absent from the immediately superseded22&#45;finding roster under substantially the same registered sources&#46; This is a bounded review omission classification, not proof of when code was introduced or that a formerly working repair regressed&#46;
- Suggested current verification: Use the current finding exact public&#45;boundary reproduction or requirement&#45;specific applicability check at the intended remediation head&#46; Revalidate source/dependency/material identities; history does not close the finding&#46;

### Comparison 7: P20&#45;AUD&#45;007

- Relationship: new&#45;omission
- Prior claim (attributed): No matching finding appears in the inspected immediately superseded same&#45;pair22&#45;finding list&#46; No absence claim is made about any other report, reviewer, model or older history&#46;
- Current evidence: Installed independent core at b3f1810801195d4e4746d9c9f0ac63d39da57313, P20&#45;AUD&#45;007: The public whole&#45;call classifier treats maximum context length is 500000 tokens as Transient because the broad 500 pattern matches the token limit&#46; An enabled helper repeats an input that the required overflow veto declares terminal&#46; Current owning evidence remains exactly as installed; no new core finding or verdict was compiled after historical exposure&#46;
- Cause / missed boundary: This claim is absent from the immediately superseded22&#45;finding roster under substantially the same registered sources&#46; This is a bounded review omission classification, not proof of when code was introduced or that a formerly working repair regressed&#46;
- Suggested current verification: Use the current finding exact public&#45;boundary reproduction or requirement&#45;specific applicability check at the intended remediation head&#46; Revalidate source/dependency/material identities; history does not close the finding&#46;

### Comparison 8: P20&#45;AUD&#45;008

- Relationship: new&#45;omission
- Prior claim (attributed): No matching finding appears in the inspected immediately superseded same&#45;pair22&#45;finding list&#46; No absence claim is made about any other report, reviewer, model or older history&#46;
- Current evidence: Installed independent core at b3f1810801195d4e4746d9c9f0ac63d39da57313, P20&#45;AUD&#45;008: Compiler clamps budget against the base answer ceiling without computing F=context&#45;clamp&#40;min&#40;C&#43;B,modelcap&#41;&#41;&#46; Current public real&#45;Anthropic probe sends4096 where C4096&#43;B2048 requires6144 under SDK165&#46; Current owning evidence remains exactly as installed; no new core finding or verdict was compiled after historical exposure&#46;
- Cause / missed boundary: This claim is absent from the immediately superseded22&#45;finding roster under substantially the same registered sources&#46; This is a bounded review omission classification, not proof of when code was introduced or that a formerly working repair regressed&#46;
- Suggested current verification: Use the current finding exact public&#45;boundary reproduction or requirement&#45;specific applicability check at the intended remediation head&#46; Revalidate source/dependency/material identities; history does not close the finding&#46;

### Comparison 9: P20&#45;AUD&#45;009

- Relationship: missed&#45;branch
- Prior claim (attributed): Superseded phase20&#45;codex&#45;gpt61&#45;703d8bd&#45;20261101t181806z, P20&#45;AUD&#45;013, reported: The executable ModelInfo/compat projection preserves a subset of required fields and silently loses adopted Chat routing and Responses feature controls; the production Responses builder consequently sends disallowed max&#95;output&#95;tokens and the obsolete cache wire form for declared explicit&#45;cache models&#46;
- Current evidence: Installed independent core at b3f1810801195d4e4746d9c9f0ac63d39da57313, P20&#45;AUD&#45;009: AnthropicMessagesCompat has only6flags; source metadata decoding omits allowEmptySignature,supportsCacheControlOnTools,supportsLongCacheRetention,supportsMidConvoEffort,allowedFallbackModels and affinity settings&#46; Current authentic bundle contains true empty&#45;signature overrides, false Fireworks cache overrides, nine true effort overrides and one fallback&#45;model list&#46; No public typed field or production caller implements these source branches&#46; Current owning evidence remains exactly as installed; no new core finding or verdict was compiled after historical exposure&#46;
- Cause / missed boundary: The superseded report named Chat/Responses source&#45;control projection loss&#46; Current tests retain those concrete controls; this current finding concerns a different omitted Anthropic compatibility or prompt&#45;cache metadata branch at the source&#45;to&#45;executable&#45;record boundary&#46; This is not a claim that the repaired named Chat/Responses fields were lost again&#46;
- Suggested current verification: Use the current finding exact public&#45;boundary reproduction or requirement&#45;specific applicability check at the intended remediation head&#46; Revalidate source/dependency/material identities; history does not close the finding&#46;
- Prior finding: phase20&#45;codex&#45;gpt61&#45;703d8bd&#45;20261101t181806z / P20&#45;AUD&#45;013; source prior&#45;findings

### Comparison 10: P20&#45;AUD&#45;010

- Relationship: new&#45;omission
- Prior claim (attributed): No matching finding appears in the inspected immediately superseded same&#45;pair22&#45;finding list&#46; No absence claim is made about any other report, reviewer, model or older history&#46;
- Current evidence: Installed independent core at b3f1810801195d4e4746d9c9f0ac63d39da57313, P20&#45;AUD&#45;010: Only explicit caller ProxyConfig is consumed&#46; The sole environment helper reads process env, prefers uppercase, globally chooses HTTPS&#95;PROXY before HTTP&#95;PROXY and cannot select by target protocol or ALL&#95;PROXY&#46; Prepared auth/source environment is not connected to any alternate proxy owner; reqwest ambient discovery is disabled&#46; Current owning evidence remains exactly as installed; no new core finding or verdict was compiled after historical exposure&#46;
- Cause / missed boundary: This claim is absent from the immediately superseded22&#45;finding roster under substantially the same registered sources&#46; This is a bounded review omission classification, not proof of when code was introduced or that a formerly working repair regressed&#46;
- Suggested current verification: Use the current finding exact public&#45;boundary reproduction or requirement&#45;specific applicability check at the intended remediation head&#46; Revalidate source/dependency/material identities; history does not close the finding&#46;

### Comparison 11: P20&#45;AUD&#45;011

- Relationship: new&#45;omission
- Prior claim (attributed): No matching finding appears in the inspected immediately superseded same&#45;pair22&#45;finding list&#46; No absence claim is made about any other report, reviewer, model or older history&#46;
- Current evidence: Installed independent core at b3f1810801195d4e4746d9c9f0ac63d39da57313, P20&#45;AUD&#45;011: ClassifierContext&#46;questions and ClassifierQuestion::Choice&#46;criteria are BTreeMaps&#46; Source declaration order cannot be represented: llama iterates those keys to assign A/B/&#46;&#46;&#46; and sequential question work&#46; First&#45;probability tie thus follows lexical keys rather than the declared first criterion&#46; Current owning evidence remains exactly as installed; no new core finding or verdict was compiled after historical exposure&#46;
- Cause / missed boundary: This claim is absent from the immediately superseded22&#45;finding roster under substantially the same registered sources&#46; This is a bounded review omission classification, not proof of when code was introduced or that a formerly working repair regressed&#46;
- Suggested current verification: Use the current finding exact public&#45;boundary reproduction or requirement&#45;specific applicability check at the intended remediation head&#46; Revalidate source/dependency/material identities; history does not close the finding&#46;

### Comparison 12: P20&#45;AUD&#45;012

- Relationship: new&#45;omission
- Prior claim (attributed): No matching finding appears in the inspected immediately superseded same&#45;pair22&#45;finding list&#46; No absence claim is made about any other report, reviewer, model or older history&#46;
- Current evidence: Installed independent core at b3f1810801195d4e4746d9c9f0ac63d39da57313, P20&#45;AUD&#45;012: Models::new and from&#95;registry set credential&#95;store=None&#46; Registering a fake OAuth flow and invoking Models&#46;login fails no credential store attached before interaction, contrary to the specified memory default&#46; Current owning evidence remains exactly as installed; no new core finding or verdict was compiled after historical exposure&#46;
- Cause / missed boundary: This claim is absent from the immediately superseded22&#45;finding roster under substantially the same registered sources&#46; This is a bounded review omission classification, not proof of when code was introduced or that a formerly working repair regressed&#46;
- Suggested current verification: Use the current finding exact public&#45;boundary reproduction or requirement&#45;specific applicability check at the intended remediation head&#46; Revalidate source/dependency/material identities; history does not close the finding&#46;

### Comparison 13: P20&#45;AUD&#45;013

- Relationship: new&#45;omission
- Prior claim (attributed): No matching finding appears in the inspected immediately superseded same&#45;pair22&#45;finding list&#46; No absence claim is made about any other report, reviewer, model or older history&#46;
- Current evidence: Installed independent core at b3f1810801195d4e4746d9c9f0ac63d39da57313, P20&#45;AUD&#45;013: A complete public custom ImageModelRecord with output=&#91;Text&#93; is accepted and emits modalities=&#91;text&#93;&#46; The governing wire rule requires &#91;image,text&#93; when text output is supported and &#91;image&#93; otherwise, independent of order/omission in metadata&#46; Current owning evidence remains exactly as installed; no new core finding or verdict was compiled after historical exposure&#46;
- Cause / missed boundary: This claim is absent from the immediately superseded22&#45;finding roster under substantially the same registered sources&#46; This is a bounded review omission classification, not proof of when code was introduced or that a formerly working repair regressed&#46;
- Suggested current verification: Use the current finding exact public&#45;boundary reproduction or requirement&#45;specific applicability check at the intended remediation head&#46; Revalidate source/dependency/material identities; history does not close the finding&#46;

### Comparison 14: P20&#45;AUD&#45;014

- Relationship: new&#45;omission
- Prior claim (attributed): No matching finding appears in the inspected immediately superseded same&#45;pair22&#45;finding list&#46; No absence claim is made about any other report, reviewer, model or older history&#46;
- Current evidence: Installed independent core at b3f1810801195d4e4746d9c9f0ac63d39da57313, P20&#45;AUD&#45;014: Successful nonstream JSON is fully received and parsed before onResponse&#46; If onResponse fails or is cancelled, failure&#95;from&#95;error constructs an empty result with no usage,output,responseId instead of preserving received content/billing facts&#46; Current owning evidence remains exactly as installed; no new core finding or verdict was compiled after historical exposure&#46;
- Cause / missed boundary: This claim is absent from the immediately superseded22&#45;finding roster under substantially the same registered sources&#46; This is a bounded review omission classification, not proof of when code was introduced or that a formerly working repair regressed&#46;
- Suggested current verification: Use the current finding exact public&#45;boundary reproduction or requirement&#45;specific applicability check at the intended remediation head&#46; Revalidate source/dependency/material identities; history does not close the finding&#46;

### Comparison 15: P20&#45;AUD&#45;015

- Relationship: new&#45;omission
- Prior claim (attributed): No matching finding appears in the inspected immediately superseded same&#45;pair22&#45;finding list&#46; No absence claim is made about any other report, reviewer, model or older history&#46;
- Current evidence: Installed independent core at b3f1810801195d4e4746d9c9f0ac63d39da57313, P20&#45;AUD&#45;015: The current public compile/validate&#95;tool&#95;arguments probe accepts integer0, false and empty&#45;string controls, then returns Invalid&#40;type null&#41; for0&#46;0 and fails&#46; The branch checks only Number::as&#95;u64/as&#95;i64&#46; Exact locked serde&#95;json1&#46;0&#46;149 returns None for Float from both accessors&#46; Current owning evidence remains exactly as installed; no new core finding or verdict was compiled after historical exposure&#46;
- Cause / missed boundary: This claim is absent from the immediately superseded22&#45;finding roster under substantially the same registered sources&#46; This is a bounded review omission classification, not proof of when code was introduced or that a formerly working repair regressed&#46;
- Suggested current verification: Use the current finding exact public&#45;boundary reproduction or requirement&#45;specific applicability check at the intended remediation head&#46; Revalidate source/dependency/material identities; history does not close the finding&#46;

### Comparison 16: P20&#45;AUD&#45;016

- Relationship: new&#45;omission
- Prior claim (attributed): No matching finding appears in the inspected immediately superseded same&#45;pair22&#45;finding list&#46; No absence claim is made about any other report, reviewer, model or older history&#46;
- Current evidence: Installed independent core at b3f1810801195d4e4746d9c9f0ac63d39da57313, P20&#45;AUD&#45;016: The committed opi&#45;ai default suite has three failing tests: both legacy profile tests expect strict:true without constrained intent, and the enrichment test expects cache Disabled to suppress separately configured affinity headers&#46; The current verification gate cannot pass&#46; Current owning evidence remains exactly as installed; no new core finding or verdict was compiled after historical exposure&#46;
- Cause / missed boundary: This claim is absent from the immediately superseded22&#45;finding roster under substantially the same registered sources&#46; This is a bounded review omission classification, not proof of when code was introduced or that a formerly working repair regressed&#46;
- Suggested current verification: Use the current finding exact public&#45;boundary reproduction or requirement&#45;specific applicability check at the intended remediation head&#46; Revalidate source/dependency/material identities; history does not close the finding&#46;

### Comparison 17: P20&#45;AUD&#45;017

- Relationship: insufficient&#45;information
- Prior claim (attributed): Superseded phase20&#45;codex&#45;gpt61&#45;703d8bd&#45;20261101t181806z, P20&#45;AUD&#45;018, reported: The actual&#45;package onboarding harness stops at cargo check; controlled&#45;peer executions use different workspace test code, and the first&#45;use examples still require manual route/resolver assembly&#46; This does not meet ONB01/02 same&#45;source ordinary package&#45;consumer acceptance&#46;
- Current evidence: Installed independent core at b3f1810801195d4e4746d9c9f0ac63d39da57313, P20&#45;AUD&#45;017: external&#95;sdk is an in&#45;workspace test binary and calls itself consumer&#45;shaped&#46; None of the package runners copies or executes that suite as an outside&#45;workspace consumer&#46; README package consumer covers three Anthropic simple calls, Schema consumer covers validation, HTTP/crypto consumers use advanced preparation, and WS consumer covers Codex stream&#95;simple/cleanup&#46; The mandated full external four&#45;entrance/direct&#45;custom/typed operation/serde/Send/two&#45;instance/shared&#45;store matrix has no current source execution path&#46; Current owning evidence remains exactly as installed; no new core finding or verdict was compiled after historical exposure&#46;
- Cause / missed boundary: Prior018 concerned exact README&#45;source onboarding execution and manual first&#45;use setup&#46; Current source has a real same&#45;source runner and recorded3&#45;flow observations, while current017 concerns the larger full SDK package&#45;consumer matrix&#46; The scopes differ; no same&#45;defect recurrence or regression is inferred&#46;
- Suggested current verification: Use the current finding exact public&#45;boundary reproduction or requirement&#45;specific applicability check at the intended remediation head&#46; Revalidate source/dependency/material identities; history does not close the finding&#46;
- Prior finding: phase20&#45;codex&#45;gpt61&#45;703d8bd&#45;20261101t181806z / P20&#45;AUD&#45;018; source prior&#45;findings

### Comparison 18: P20&#45;AUD&#45;018

- Relationship: evidence&#45;gap
- Prior claim (attributed): Superseded phase20&#45;codex&#45;gpt61&#45;703d8bd&#45;20261101t181806z, P20&#45;AUD&#45;018, reported: The actual&#45;package onboarding harness stops at cargo check; controlled&#45;peer executions use different workspace test code, and the first&#45;use examples still require manual route/resolver assembly&#46; This does not meet ONB01/02 same&#45;source ordinary package&#45;consumer acceptance&#46; Superseded phase20&#45;codex&#45;gpt61&#45;703d8bd&#45;20261101t181806z, P20&#45;AUD&#45;019, reported: Mandatory Stream Pending/cross&#45;thread/unpolled&#45;Drop propagation is not established by the current carrier/operation suite&#46; Production telemetry has only a Future wrapper; the Stream test supplies its own in&#95;scope implementation and can pass without a production Stream carrier path&#46; Superseded phase20&#45;codex&#45;gpt61&#45;703d8bd&#45;20261101t181806z, P20&#45;AUD&#45;020, reported: Applicable3OS workspace runtime and MSRV compilation proof exists, but supplied records do not provide original current full&#45;SDK ordinary/default/C&#45;union package&#45;consumer artifacts/logs and native&#45;build provenance needed to assess the mandatory final package/platform closure&#46; Separately, the current catalog artifact and packer bytes match their manifest, but original upstream pin/helper/acquisition material seals needed to establish source association are unavailable; this does not allege invalid catalog runtime content&#46;
- Current evidence: Installed independent core at b3f1810801195d4e4746d9c9f0ac63d39da57313, P20&#45;AUD&#45;018: The linked mandatory criteria lack complete applicable proof&#46; This includes current full external package consumers, native/feature&#45;union/source&#45;build identities, native Linux/macOS/Windows and MSRV1&#46;97 execution, formal admission/exit receipts, and named finite&#45;case conformance gaps detailed in each requirement record&#46; Substantial current code and passing fixtures establish only the portions they actually observe&#46; Current owning evidence remains exactly as installed; no new core finding or verdict was compiled after historical exposure&#46;
- Cause / missed boundary: Both runs leave complete package/material applicability unestablished&#46; Prior020 specifically attributed valid3OS/MSRV CI proof at its own source identity; that is not rewritten as never executed&#46; Current code and proof boundaries differ, and the full original/current artifact/native/source identities were not verified in this audit&#46; Prior019 Stream&#45;proof concerns are not asserted repaired solely from new test names or Future proof&#46;
- Suggested current verification: Use the current finding exact public&#45;boundary reproduction or requirement&#45;specific applicability check at the intended remediation head&#46; Revalidate source/dependency/material identities; history does not close the finding&#46;
- Prior finding: phase20&#45;codex&#45;gpt61&#45;703d8bd&#45;20261101t181806z / P20&#45;AUD&#45;018; source prior&#45;findings
- Prior finding: phase20&#45;codex&#45;gpt61&#45;703d8bd&#45;20261101t181806z / P20&#45;AUD&#45;019; source prior&#45;findings
- Prior finding: phase20&#45;codex&#45;gpt61&#45;703d8bd&#45;20261101t181806z / P20&#45;AUD&#45;020; source prior&#45;findings

### Comparison 19: P20&#45;AUD&#45;019

- Relationship: missed&#45;branch
- Prior claim (attributed): Superseded phase20&#45;codex&#45;gpt61&#45;703d8bd&#45;20261101t181806z, P20&#45;AUD&#45;013, reported: The executable ModelInfo/compat projection preserves a subset of required fields and silently loses adopted Chat routing and Responses feature controls; the production Responses builder consequently sends disallowed max&#95;output&#95;tokens and the obsolete cache wire form for declared explicit&#45;cache models&#46;
- Current evidence: Installed independent core at b3f1810801195d4e4746d9c9f0ac63d39da57313, P20&#45;AUD&#45;019: ModelInfo has only cache&#45;control capability booleans and no promptCache short/long seconds&#46; Catalog raw retains the source object, but conversion/store encoding into an executable complete ModelInfo does not represent those durations; reading a complete owned model or restoring its typed store record cannot preserve required metadata&#46; Current owning evidence remains exactly as installed; no new core finding or verdict was compiled after historical exposure&#46;
- Cause / missed boundary: The superseded report named Chat/Responses source&#45;control projection loss&#46; Current tests retain those concrete controls; this current finding concerns a different omitted Anthropic compatibility or prompt&#45;cache metadata branch at the source&#45;to&#45;executable&#45;record boundary&#46; This is not a claim that the repaired named Chat/Responses fields were lost again&#46;
- Suggested current verification: Use the current finding exact public&#45;boundary reproduction or requirement&#45;specific applicability check at the intended remediation head&#46; Revalidate source/dependency/material identities; history does not close the finding&#46;
- Prior finding: phase20&#45;codex&#45;gpt61&#45;703d8bd&#45;20261101t181806z / P20&#45;AUD&#45;013; source prior&#45;findings

### Comparison 20: P20&#45;AUD&#45;020

- Relationship: new&#45;omission
- Prior claim (attributed): No matching finding appears in the inspected immediately superseded same&#45;pair22&#45;finding list&#46; No absence claim is made about any other report, reviewer, model or older history&#46;
- Current evidence: Installed independent core at b3f1810801195d4e4746d9c9f0ac63d39da57313, P20&#45;AUD&#45;020: The public credential reports Some&#40;9999&#45;12&#45;31&#41; for a permanent key and has no typed non&#45;expiring variant&#46; The target explicitly replaces the maximum&#45;time sentinel with typed permanence&#46; Current owning evidence remains exactly as installed; no new core finding or verdict was compiled after historical exposure&#46;
- Cause / missed boundary: This claim is absent from the immediately superseded22&#45;finding roster under substantially the same registered sources&#46; This is a bounded review omission classification, not proof of when code was introduced or that a formerly working repair regressed&#46;
- Suggested current verification: Use the current finding exact public&#45;boundary reproduction or requirement&#45;specific applicability check at the intended remediation head&#46; Revalidate source/dependency/material identities; history does not close the finding&#46;

### Comparison 21: P20&#45;AUD&#45;021

- Relationship: new&#45;omission
- Prior claim (attributed): No matching finding appears in the inspected immediately superseded same&#45;pair22&#45;finding list&#46; No absence claim is made about any other report, reviewer, model or older history&#46;
- Current evidence: Installed independent core at b3f1810801195d4e4746d9c9f0ac63d39da57313, P20&#45;AUD&#45;021: English specifies that retry is blocked when capacity is below required cost5/transient10&#46; Chinese only states that zero capacity blocks retry, omitting positive&#45;but&#45;insufficient capacities1&#46;&#46;4/1&#46;&#46;9&#46; This is incomplete bilingual boundary wording; it does not prove Chinese explicitly permits those retries or that production is wrong&#46; Current owning evidence remains exactly as installed; no new core finding or verdict was compiled after historical exposure&#46;
- Cause / missed boundary: This claim is absent from the immediately superseded22&#45;finding roster under substantially the same registered sources&#46; This is a bounded review omission classification, not proof of when code was introduced or that a formerly working repair regressed&#46;
- Suggested current verification: Use the current finding exact public&#45;boundary reproduction or requirement&#45;specific applicability check at the intended remediation head&#46; Revalidate source/dependency/material identities; history does not close the finding&#46;

### Comparison 22: preserved historical repair

- Relationship: preserved&#45;repair
- Prior claim (attributed): Superseded phase20&#45;codex&#45;gpt61&#45;703d8bd&#45;20261101t181806z, P20&#45;AUD&#45;001, reported: The AWS parser accepts a known&#45;invalid prelude as incomplete until its declared total length arrives&#46; Bad prelude CRC or headers exceeding total minus16 can cause retention up to the declared approximately4GiB before rejection or EOF/cancellation, instead of early protocol rejection&#46;
- Current evidence: At sealed head b3f1810801195d4e4746d9c9f0ac63d39da57313: crates/opi&#45;ai/src/bedrock/event&#95;stream&#46;rs:27&#45;42 now checks preludeCRC/header relation before waiting for declared total; current invalid&#95;prelude&#95;is&#95;rejected&#95;before&#95;payload&#95;wait and valid&#95;prelude&#95;remains&#95;buffered&#95;until&#95;frame&#95;complete both executed and passed&#46;
- Cause / missed boundary: The current implementation/previously executed owning assertions establish the specific old predicate correction&#46; No exact repair commit or broader causal history is established from the permitted single superseded record; no old closure label is adopted as proof&#46;
- Suggested current verification: Keep early&#45;invalid/valid&#45;incomplete controls; no new payload cap is implied&#46;
- Prior finding: phase20&#45;codex&#45;gpt61&#45;703d8bd&#45;20261101t181806z / P20&#45;AUD&#45;001; source prior&#45;findings

### Comparison 23: preserved historical repair

- Relationship: preserved&#45;repair
- Prior claim (attributed): Superseded phase20&#45;codex&#45;gpt61&#45;703d8bd&#45;20261101t181806z, P20&#45;AUD&#45;002, reported: A configured http://127&#46;attacker&#46;invalid/creds or http://169&#46;254&#46;attacker&#46;invalid/creds passes the link&#45;local/loopback guard; the public Models builtin resolver issues one source request carrying Authorization to that DNS domain&#46;
- Current evidence: At sealed head b3f1810801195d4e4746d9c9f0ac63d39da57313: Current typed url::Host guard at auth/aws/metadata&#46;rs117&#45;139 accepts literal loopback/link&#45;local or exact allowed localhost, not prefix domains&#46; Current ecs&#95;full&#95;uri&#95;parsed&#95;host&#95;guard&#95;runs&#95;before&#95;token&#95;file&#95;or&#95;send passed through the builtin source path&#46;
- Cause / missed boundary: The current implementation/previously executed owning assertions establish the specific old predicate correction&#46; No exact repair commit or broader causal history is established from the permitted single superseded record; no old closure label is adopted as proof&#46;
- Suggested current verification: Retain parsed domain/IP alternate&#45;form and before&#45;token&#45;I/O zero&#45;send controls&#46;
- Prior finding: phase20&#45;codex&#45;gpt61&#45;703d8bd&#45;20261101t181806z / P20&#45;AUD&#45;002; source prior&#45;findings

### Comparison 24: preserved historical repair

- Relationship: preserved&#45;repair
- Prior claim (attributed): Superseded phase20&#45;codex&#45;gpt61&#45;703d8bd&#45;20261101t181806z, P20&#45;AUD&#45;003, reported: Incremental drains convert a trailing CR to LF before the following LF arrives, creating a false blank line and parsing an incomplete legal multiline JSON event&#46;
- Current evidence: At sealed head b3f1810801195d4e4746d9c9f0ac63d39da57313: Current Chat, Responses&#45;shared and Gemini multiline&#95;frames&#95;survive&#95;every&#95;line&#95;ending&#95;split passed, plus actual HTTP multiline&#95;sse&#95;line&#95;endings&#95;survive&#95;http&#95;chunk&#95;splits fixtures&#46; The prior split CR/LF multiline predicate is now discriminated&#46;
- Cause / missed boundary: The current implementation/previously executed owning assertions establish the specific old predicate correction&#46; No exact repair commit or broader causal history is established from the permitted single superseded record; no old closure label is adopted as proof&#46;
- Suggested current verification: Keep each affected wire every&#45;boundary multiline fixture, not only single&#45;line UTF8 controls&#46;
- Prior finding: phase20&#45;codex&#45;gpt61&#45;703d8bd&#45;20261101t181806z / P20&#45;AUD&#45;003; source prior&#45;findings

### Comparison 25: preserved historical repair

- Relationship: preserved&#45;repair
- Prior claim (attributed): Superseded phase20&#45;codex&#45;gpt61&#45;703d8bd&#45;20261101t181806z, P20&#45;AUD&#45;004, reported: Models::cancel&#95;deferred awaits an uncancellable resolver before reaching the backend, so a pre&#45;aborted call starts source work and never settles if the resolver remains pending&#46;
- Current evidence: At sealed head b3f1810801195d4e4746d9c9f0ac63d39da57313: Current deferred&#95;contract::remote&#95;cancel&#95;observes&#95;caller&#95;abort&#95;before&#95;and&#95;during&#95;auth executed and passed&#46; The current cancel operation preserves identity and distinct explicit remote&#45;cancel semantics; this predicate is separate from ordinary whole&#45;call delay cancellation&#46;
- Cause / missed boundary: The current implementation/previously executed owning assertions establish the specific old predicate correction&#46; No exact repair commit or broader causal history is established from the permitted single superseded record; no old closure label is adopted as proof&#46;
- Suggested current verification: Retain pre&#45;aborted and pending&#45;auth cancellation with zero backend effect; keep store&#45;owned rotation independent&#46;
- Prior finding: phase20&#45;codex&#45;gpt61&#45;703d8bd&#45;20261101t181806z / P20&#45;AUD&#45;004; source prior&#45;findings

### Comparison 26: preserved historical repair

- Relationship: preserved&#45;repair
- Prior claim (attributed): Superseded phase20&#45;codex&#45;gpt61&#45;703d8bd&#45;20261101t181806z, P20&#45;AUD&#45;005, reported: is&#95;context&#95;overflow uses unchecked window&#42;99 in its Length/output0 branch&#46; For reported input&#95;tokens1 and context&#95;window186330748219288401, the accepted u64 argument causes an overflow panic instead of returning false&#46;
- Current evidence: At sealed head b3f1810801195d4e4746d9c9f0ac63d39da57313: Current public&#95;utilities::context&#95;overflow&#95;length&#95;stop&#95;handles&#95;full&#95;u64&#95;window&#95;range executed and passed with the public large&#45;window controls&#46; Separate overflow&#45;pattern/knownness tests also pass&#46; This repair is independent of current whole&#45;call retry misclassification P20&#45;AUD&#45;007&#46;
- Cause / missed boundary: The current implementation/previously executed owning assertions establish the specific old predicate correction&#46; No exact repair commit or broader causal history is established from the permitted single superseded record; no old closure label is adopted as proof&#46;
- Suggested current verification: Retain u64&#45;extreme/knownness/ratio controls without source&#45;free input caps&#46;
- Prior finding: phase20&#45;codex&#45;gpt61&#45;703d8bd&#45;20261101t181806z / P20&#45;AUD&#45;005; source prior&#45;findings

### Comparison 27: preserved historical repair

- Relationship: preserved&#45;repair
- Prior claim (attributed): Superseded phase20&#45;codex&#45;gpt61&#45;703d8bd&#45;20261101t181806z, P20&#45;AUD&#45;006, reported: A valid10000&#45;deep allOf schema aborts the process on ordinary2MiB Rust thread stacks: root$id triggers recursive anchor admission; otherwise compile/check succeeds but public normalization and dropping CompiledSchema each stack&#45;overflow&#46; Main evaluator stack safety does not extend across the public owned schema lifecycle&#46;
- Current evidence: At sealed head b3f1810801195d4e4746d9c9f0ac63d39da57313: Current schema&#95;engine and schema&#95;public lifecycle tests compile/check/drop10,000&#45;deep schemas, exercise root$id, subvalidator clone and compile&#45;error release on explicit2MiB threads&#46; Three direct child executions were separately captured with original exit0 and checker&#45;confirmed1passed/0failed/0ignored each for this historical comparison&#46;
- Cause / missed boundary: The current implementation/previously executed owning assertions establish the specific old predicate correction&#46; No exact repair commit or broader causal history is established from the permitted single superseded record; no old closure label is adopted as proof&#46;
- Suggested current verification: Retain normal and error destruction, reference registration and normalization on ordinary stacks; no mem::forget success is accepted&#46;
- Prior finding: phase20&#45;codex&#45;gpt61&#45;703d8bd&#45;20261101t181806z / P20&#45;AUD&#45;006; source prior&#45;findings
- Candidate raw proof: C:&#92;Users&#92;Luiz&#92;AppData&#92;Local&#92;Temp&#92;opi&#45;phase20&#45;audit&#45;nc3hj&#95;mw&#92;checks&#92;history&#45;deep&#95;schemas&#95;do&#95;not&#95;exhaust&#95;the&#95;rust&#95;stack&#46;log; SHA-256 5e36aaa8a44c1f78a07c6ad294d1037385d80702f9e39f915324f3e9dcf521bb; checked: Exact current sealed&#45;baseline executable; originalexit0; expected named case1passed0failed0ignored; execution checker PASS; explicit2MiB child stack&#46; Binary/log/checker digests retained&#46;; limits: Windows current toolchain fixture proof only; not platform/MSRV/native package acceptance&#46; Independent core remained byte&#45;identical&#46;
- Candidate raw proof: C:&#92;Users&#92;Luiz&#92;AppData&#92;Local&#92;Temp&#92;opi&#45;phase20&#45;audit&#45;nc3hj&#95;mw&#92;checks&#92;history&#45;deep&#95;compile&#95;error&#95;releases&#95;owned&#95;schema&#46;log; SHA-256 657ca34138932acc224aa0130660a14f124d0000402f7031fd2437477048d468; checked: Exact current sealed&#45;baseline executable; originalexit0; expected named case1passed0failed0ignored; execution checker PASS; explicit2MiB child stack&#46; Binary/log/checker digests retained&#46;; limits: Windows current toolchain fixture proof only; not platform/MSRV/native package acceptance&#46; Independent core remained byte&#45;identical&#46;
- Candidate raw proof: C:&#92;Users&#92;Luiz&#92;AppData&#92;Local&#92;Temp&#92;opi&#45;phase20&#45;audit&#45;nc3hj&#95;mw&#92;checks&#92;history&#45;deep&#95;schema&#95;normalization&#95;and&#95;subvalidator&#95;clone&#95;release&#95;normally&#46;log; SHA-256 81b22f5c58cc8f18f946e96e104948bfabb5a32abb52e4f634f65ad7fe0f8c45; checked: Exact current sealed&#45;baseline executable; originalexit0; expected named case1passed0failed0ignored; execution checker PASS; explicit2MiB child stack&#46; Binary/log/checker digests retained&#46;; limits: Windows current toolchain fixture proof only; not platform/MSRV/native package acceptance&#46; Independent core remained byte&#45;identical&#46;

### Comparison 28: preserved historical repair

- Relationship: preserved&#45;repair
- Prior claim (attributed): Superseded phase20&#45;codex&#45;gpt61&#45;703d8bd&#45;20261101t181806z, P20&#45;AUD&#45;007, reported: A successful selected ADC metadata/token response with positive i64::MAX expires&#95;in passes field parsing and panics during OffsetDateTime addition rather than returning typed failure&#46;
- Current evidence: At sealed head b3f1810801195d4e4746d9c9f0ac63d39da57313: Current metadata&#95;checked&#95;expiry, service&#95;account&#95;checked&#95;expiry, authorized&#95;user&#95;checked&#95;expiry, external&#95;authorized&#95;user&#95;checked&#95;expiry and external&#45;account STS extreme&#45;expiry fixtures all passed with ordinary and invalid&#45;expiry controls&#46;
- Cause / missed boundary: The current implementation/previously executed owning assertions establish the specific old predicate correction&#46; No exact repair commit or broader causal history is established from the permitted single superseded record; no old closure label is adopted as proof&#46;
- Suggested current verification: Retain each ADC family extreme/wrong&#45;type/ordinary expiry boundary and typed no&#45;send failure&#46;
- Prior finding: phase20&#45;codex&#45;gpt61&#45;703d8bd&#45;20261101t181806z / P20&#45;AUD&#45;007; source prior&#45;findings

### Comparison 29: preserved historical repair

- Relationship: preserved&#45;repair
- Prior claim (attributed): Superseded phase20&#45;codex&#45;gpt61&#45;703d8bd&#45;20261101t181806z, P20&#45;AUD&#45;008, reported: The external&#45;account aws1 supplier removes one byte from arbitrary decoded metadata rather than one validated region suffix; an otherwise successful region response ending in a multibyte character panics&#46;
- Current evidence: At sealed head b3f1810801195d4e4746d9c9f0ac63d39da57313: Current adc&#95;owner&#95;regressions::aws&#95;subject&#95;region&#95;suffix&#95;and&#95;environment&#95;precedence and the ordinary external&#95;aws&#95;imds&#95;flow&#95;with&#95;v2&#95;token&#95;and&#95;region&#95;strip executed and passed, covering source region decoding and precedence rather than the old unchecked byte slice&#46;
- Cause / missed boundary: The current implementation/previously executed owning assertions establish the specific old predicate correction&#46; No exact repair commit or broader causal history is established from the permitted single superseded record; no old closure label is adopted as proof&#46;
- Suggested current verification: Retain malformed Unicode/ASCII source&#45;response controls and selected environment precedence&#46;
- Prior finding: phase20&#45;codex&#45;gpt61&#45;703d8bd&#45;20261101t181806z / P20&#45;AUD&#45;008; source prior&#45;findings

### Comparison 30: preserved historical repair

- Relationship: preserved&#45;repair
- Prior claim (attributed): Superseded phase20&#45;codex&#45;gpt61&#45;703d8bd&#45;20261101t181806z, P20&#45;AUD&#45;009, reported: Chat and Responses initialize activation only after a body chunk has arrived, excluding the time already spent polling the response body&#46; AIR&#45;061/RDF&#45;02 require response&#45;stream activation/first&#45;poll timing&#46;
- Current evidence: At sealed head b3f1810801195d4e4746d9c9f0ac63d39da57313: Current Chat and Responses each pass duration&#95;includes&#95;pending&#95;first&#95;body&#95;poll, duration&#95;excludes&#95;auth&#95;setup&#95;callback&#95;and&#95;unpolled&#95;time and untimed setup&#45;failure tests&#46; Azure and Google controls also pass, without substituting their implementations for Chat/Responses&#46;
- Cause / missed boundary: The current implementation/previously executed owning assertions establish the specific old predicate correction&#46; No exact repair commit or broader causal history is established from the permitted single superseded record; no old closure label is adopted as proof&#46;
- Suggested current verification: Retain owning wire activation/auth/body&#45;wait/collector duration assertions&#46;
- Prior finding: phase20&#45;codex&#45;gpt61&#45;703d8bd&#45;20261101t181806z / P20&#45;AUD&#45;009; source prior&#45;findings

### Comparison 31: preserved historical repair

- Relationship: preserved&#45;repair
- Prior claim (attributed): Superseded phase20&#45;codex&#45;gpt61&#45;703d8bd&#45;20261101t181806z, P20&#45;AUD&#45;010, reported: Actual adapters unconditionally choose NativeOrCollapse and discard replay&#46;facts; chronological collapse returns only Cow&#60;TranscriptContext&#62;, with no mode/affected range/source&#45;prefix facts&#46; Strict helper has no selected&#45;fidelity ingress in actual call preparation&#46;
- Current evidence: At sealed head b3f1810801195d4e4746d9c9f0ac63d39da57313: Current accepted&#95;projection&#95;survives&#95;auth&#95;callback&#95;partial&#95;eof&#95;and&#95;cancel&#95;failures, strict&#95;projection&#95;rejects&#95;before&#95;auth&#95;callbacks&#95;and&#95;network, and projection&#95;policy&#95;and&#95;facts&#95;follow&#95;actual&#95;native&#95;simple&#95;direct&#95;and&#95;prepared&#95;calls passed&#46;
- Cause / missed boundary: The current implementation/previously executed owning assertions establish the specific old predicate correction&#46; No exact repair commit or broader causal history is established from the permitted single superseded record; no old closure label is adopted as proof&#46;
- Suggested current verification: Keep selected fidelity pre&#45;auth admission, frozen facts and terminal/partial/failure propagation through actual owning paths&#46;
- Prior finding: phase20&#45;codex&#45;gpt61&#45;703d8bd&#45;20261101t181806z / P20&#45;AUD&#45;010; source prior&#45;findings

### Comparison 32: preserved historical repair

- Relationship: preserved&#45;repair
- Prior claim (attributed): Superseded phase20&#45;codex&#45;gpt61&#45;703d8bd&#45;20261101t181806z, P20&#45;AUD&#45;011, reported: The canonical estimator ignores an independently known positive total when the parent&#45;bucket reported flag is false, substantially underestimating valid continuation context and simple output budgets&#46;
- Current evidence: At sealed head b3f1810801195d4e4746d9c9f0ac63d39da57313: Current context&#95;budget::independently&#95;reported&#95;total&#95;wins&#95;with&#95;known&#95;or&#95;unknown&#95;parent&#95;buckets and independent&#95;total&#95;still&#95;requires&#95;a&#95;successful&#95;timely&#95;response executed and passed&#46;
- Cause / missed boundary: The current implementation/previously executed owning assertions establish the specific old predicate correction&#46; No exact repair commit or broader causal history is established from the permitted single superseded record; no old closure label is adopted as proof&#46;
- Suggested current verification: Retain independent&#45;total anchor and required success/timestamp controls separately from fallback parent sum&#46;
- Prior finding: phase20&#45;codex&#45;gpt61&#45;703d8bd&#45;20261101t181806z / P20&#45;AUD&#45;011; source prior&#45;findings

### Comparison 33: preserved historical repair

- Relationship: preserved&#45;repair
- Prior claim (attributed): Superseded phase20&#45;codex&#45;gpt61&#45;703d8bd&#45;20261101t181806z, P20&#45;AUD&#45;012, reported: All4 Models API registration seams reject empty and whitespace&#45;only exact ApiId values even though registered source explicitly disallows adding nonempty/trim grammar&#46;
- Current evidence: At sealed head b3f1810801195d4e4746d9c9f0ac63d39da57313: Current custom&#95;provider&#95;registration passes arbitrary exact ApiId controls for chat/image/classifier/deferred, including the formerly refused forms, with no auth&#46; Provider&#45;ID validation remains a different contract&#46;
- Cause / missed boundary: The current implementation/previously executed owning assertions establish the specific old predicate correction&#46; No exact repair commit or broader causal history is established from the permitted single superseded record; no old closure label is adopted as proof&#46;
- Suggested current verification: Retain all four exact API identity admission paths, options portability and real type mismatch rejection&#46;
- Prior finding: phase20&#45;codex&#45;gpt61&#45;703d8bd&#45;20261101t181806z / P20&#45;AUD&#45;012; source prior&#45;findings

### Comparison 34: preserved historical repair

- Relationship: preserved&#45;repair
- Prior claim (attributed): Superseded phase20&#45;codex&#45;gpt61&#45;703d8bd&#45;20261101t181806z, P20&#45;AUD&#45;013, reported: The executable ModelInfo/compat projection preserves a subset of required fields and silently loses adopted Chat routing and Responses feature controls; the production Responses builder consequently sends disallowed max&#95;output&#95;tokens and the obsolete cache wire form for declared explicit&#45;cache models&#46;
- Current evidence: At sealed head b3f1810801195d4e4746d9c9f0ac63d39da57313: Current catalog projection&#95;compat&#95;tests preserve Chat optional false/inference/nested values and Responses absence/false&#46; Current model/actual&#45;adapter fixtures exercise the named max&#45;output/sign&#45;in/cache/tool controls&#46; This row is limited to prior013 named Chat/Responses branches; current009/019 document different retained omissions&#46;
- Cause / missed boundary: The current implementation/previously executed owning assertions establish the specific old predicate correction&#46; No exact repair commit or broader causal history is established from the permitted single superseded record; no old closure label is adopted as proof&#46;
- Suggested current verification: Retain named wire false/omission/explicit&#45;cache controls and real consumers, not only metadata field existence&#46;
- Prior finding: phase20&#45;codex&#45;gpt61&#45;703d8bd&#45;20261101t181806z / P20&#45;AUD&#45;013; source prior&#45;findings

### Comparison 35: preserved historical repair

- Relationship: preserved&#45;repair
- Prior claim (attributed): Superseded phase20&#45;codex&#45;gpt61&#45;703d8bd&#45;20261101t181806z, P20&#45;AUD&#45;014, reported: The SDK cannot remove/clear a provider, enumerate/ask availability of all operation kinds including images, or choose/force one provider refresh through any public Models action&#46;
- Current evidence: At sealed head b3f1810801195d4e4746d9c9f0ac63d39da57313: Current selected&#95;owner&#95;management tests pass removal/re&#45;registration/clear with frozen old calls and namespaces, selected force refresh before source work, subset/tombstone preservation, and actual all&#45;operation owner lists&#46;
- Cause / missed boundary: The current implementation/previously executed owning assertions establish the specific old predicate correction&#46; No exact repair commit or broader causal history is established from the permitted single superseded record; no old closure label is adopted as proof&#46;
- Suggested current verification: Retain public action/selection&#45;before&#45;I/O and old&#45;call/isolation controls; default memory store current012 remains distinct&#46;
- Prior finding: phase20&#45;codex&#45;gpt61&#45;703d8bd&#45;20261101t181806z / P20&#45;AUD&#45;014; source prior&#45;findings

### Comparison 36: preserved historical repair

- Relationship: preserved&#45;repair
- Prior claim (attributed): Superseded phase20&#45;codex&#45;gpt61&#45;703d8bd&#45;20261101t181806z, P20&#45;AUD&#45;015, reported: Default builtin AWS source assembly still chooses a runner enforcing10seconds and64KiB stdout without caller opt&#45;in, although AIR&#45;023 withdraws these SDK parity defaults&#46; Valid larger/slower credential processes are rejected as discovery failures and can select a later source&#46;
- Current evidence: At sealed head b3f1810801195d4e4746d9c9f0ac63d39da57313: SpawnedCredentialProcess now derives Default with policy:Option=None; only with&#95;policy selects10s/64KiB limits&#46; Current default&#95;runner&#95;accepts&#95;large&#95;valid&#95;output and no&#45;policy child lifecycle controls passed&#46; CredentialProcessPolicy::default still defines an explicit policy and is not the runner omission default&#46;
- Cause / missed boundary: The current implementation/previously executed owning assertions establish the specific old predicate correction&#46; No exact repair commit or broader causal history is established from the permitted single superseded record; no old closure label is adopted as proof&#46;
- Suggested current verification: Retain actual default source assembly, large valid output and cancellation/reap controls with caller&#45;only budgets&#46;
- Prior finding: phase20&#45;codex&#45;gpt61&#45;703d8bd&#45;20261101t181806z / P20&#45;AUD&#45;015; source prior&#45;findings

### Comparison 37: preserved historical repair

- Relationship: preserved&#45;repair
- Prior claim (attributed): Superseded phase20&#45;codex&#45;gpt61&#45;703d8bd&#45;20261101t181806z, P20&#45;AUD&#45;016, reported: Default HttpClient/OpenAI Chat accepts at most10 redirects rather than the pinned source&#45;qualified20&#45;redirect behavior; a lawful11&#45;hop source request fails under ordinary SDK defaults&#46;
- Current evidence: At sealed head b3f1810801195d4e4746d9c9f0ac63d39da57313: Current HttpClient RedirectPolicy::default is Limited20 and source&#95;qualified&#95;default&#95;redirect&#95;budget&#95;has&#95;exact&#95;twenty&#95;hop&#95;boundary executed and passed&#46; This preserves the hop&#45;count correction only; current002 separately reports unsolicited header/query forwarding&#46;
- Cause / missed boundary: The current implementation/previously executed owning assertions establish the specific old predicate correction&#46; No exact repair commit or broader causal history is established from the permitted single superseded record; no old closure label is adopted as proof&#46;
- Suggested current verification: Retain exact20&#45;success/21&#45;failure send counts and source&#45;specific header/method/audience controls&#46;
- Prior finding: phase20&#45;codex&#45;gpt61&#45;703d8bd&#45;20261101t181806z / P20&#45;AUD&#45;016; source prior&#45;findings

### Comparison 38: preserved historical repair

- Relationship: preserved&#45;repair
- Prior claim (attributed): Superseded phase20&#45;codex&#45;gpt61&#45;703d8bd&#45;20261101t181806z, P20&#45;AUD&#45;017, reported: make&#95;strict&#95;json&#95;schema accepts required:&#34;a&#34;,required:1 and required:&#123;&#125; for an object properties map and silently replaces them with all property names plus nullable wrappers, although TP14 requires non&#45;string&#45;array required to fail&#46; Require intent therefore can pass this malformed shape into real wire compilation&#46;
- Current evidence: At sealed head b3f1810801195d4e4746d9c9f0ac63d39da57313: Current strict&#95;required&#95;shape&#95;is&#95;validated&#95;with&#95;or&#95;without&#95;properties, malformed&#95;required&#95;is&#95;typed&#95;and&#95;preserves&#95;the&#95;trusted&#95;schema and malformed&#95;required&#95;fails&#95;before&#95;native&#95;authentication&#95;and&#95;dispatch executed and passed&#46;
- Cause / missed boundary: The current implementation/previously executed owning assertions establish the specific old predicate correction&#46; No exact repair commit or broader causal history is established from the permitted single superseded record; no old closure label is adopted as proof&#46;
- Suggested current verification: Retain scalar/object/array wrong&#45;shape and prefer/require actual&#45;wire controls; do not confuse local inert&#45;schema semantics with strict conversion&#46;
- Prior finding: phase20&#45;codex&#45;gpt61&#45;703d8bd&#45;20261101t181806z / P20&#45;AUD&#45;017; source prior&#45;findings

### Comparison 39: preserved historical repair

- Relationship: preserved&#45;repair
- Prior claim (attributed): Superseded phase20&#45;codex&#45;gpt61&#45;703d8bd&#45;20261101t181806z, P20&#45;AUD&#45;021, reported: Transcript normalization,Serde message validation and append validate only tools&#95;added&#46; Programmatic SystemMessage&#46;sections can contain duplicate names and is silently folded by last value, though adopted TP&#45;02 requires ambiguous input rejection&#46;
- Current evidence: At sealed head b3f1810801195d4e4746d9c9f0ac63d39da57313: Current transcript&#95;contract::normalization&#95;rejects&#95;duplicate&#95;sections&#95;in&#95;initial&#95;and&#95;later&#95;messages, duplicate&#95;section&#95;append&#95;preserves&#95;entire&#95;prior&#95;transcript, and tool&#95;errors&#95;precede&#95;duplicate&#95;section&#95;rejection executed and passed&#46; JSON&#45;object parser last&#45;value behavior is a different ingress&#46;
- Cause / missed boundary: The current implementation/previously executed owning assertions establish the specific old predicate correction&#46; No exact repair commit or broader causal history is established from the permitted single superseded record; no old closure label is adopted as proof&#46;
- Suggested current verification: Retain programmatic Vec duplicate rejection before installation and atomic failed append&#46;
- Prior finding: phase20&#45;codex&#45;gpt61&#45;703d8bd&#45;20261101t181806z / P20&#45;AUD&#45;021; source prior&#45;findings

### Comparison 40: preserved historical repair

- Relationship: preserved&#45;repair
- Prior claim (attributed): Superseded phase20&#45;codex&#45;gpt61&#45;703d8bd&#45;20261101t181806z, P20&#45;AUD&#45;022, reported: The required complete42&#45;mechanism corpus lacks positive/negative cardinality acceptance for minProperties/maxProperties and rejecting&#45;bound cases for minItems/maxItems&#46; Their implementation exists but current supplied tests do not discriminate removing these four checks&#46;
- Current evidence: At sealed head b3f1810801195d4e4746d9c9f0ac63d39da57313: Current schema&#95;engine cardinality&#95;min&#95;properties&#95;boundaries, cardinality&#95;max&#95;properties&#95;boundaries, cardinality&#95;min&#95;items&#95;boundaries and cardinality&#95;max&#95;items&#95;boundaries all executed and passed&#46;
- Cause / missed boundary: The current implementation/previously executed owning assertions establish the specific old predicate correction&#46; No exact repair commit or broader causal history is established from the permitted single superseded record; no old closure label is adopted as proof&#46;
- Suggested current verification: Retain B&#45;1/B/B&#43; rejection/acceptance controls so removal of each leaf check is discriminated&#46;
- Prior finding: phase20&#45;codex&#45;gpt61&#45;703d8bd&#45;20261101t181806z / P20&#45;AUD&#45;022; source prior&#45;findings
