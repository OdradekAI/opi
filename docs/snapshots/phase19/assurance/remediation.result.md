# Phase 19 Remediation Result

**Status**: COMPLETE
**Audit index SHA-256**: `23b39dcdc52f71dc1b17b4f87678f39aa3d86289c3753eb0b5f55373ad82873e`
**Plan SHA-256**: `944841c1bbd9e8f1c0907d2b92ace9511232c34878a55a3afafdb0f080fb26af`
**Changed paths**: ["CHANGELOG.md", "Cargo.lock", "Cargo.toml", "crates/opi-eval/Cargo.toml", "crates/opi-eval/README.md", "crates/opi-eval/README.zh.md", "crates/opi-eval/scripts/test_verify_local_acceptance.py", "crates/opi-eval/scripts/verify-local-acceptance.py", "crates/opi-eval/src/experiment.rs", "crates/opi-eval/src/runner/plan.rs", "crates/opi-eval/tests/experiment_contract.rs", "crates/opi-eval/tests/native_driver.rs", "crates/opi-eval/tests/support/native_witness_material.rs", "docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.md", "docs/superpowers/specs/2026-09-05-phase19-local-native-agent-evaluation-design.zh.md"]

Disposition accounting is complete for the exact current eight-key union.
Execution is **STOPPED-WITH-NOT-CLOSED**: B1, B2 and B5 are Closed; B3, B4,
B6, B7 and B8 are Not closed. COMPLETE is the fixed result-artifact status,
not a claim that all repairs, native gates or Phase acceptance passed.

## Approval and carried-in work

The explicit invocation was `opi-remediate mode=apply phase=19
plan_sha256=944841c1bbd9e8f1c0907d2b92ace9511232c34878a55a3afafdb0f080fb26af`. Locked audit-set/apply admission passed.
Source baseline and current HEAD are both
`e40c46f42b9ebc87e66bb596d778ff239cc3183e`; no commit was made. The carried-in
unstaged fixed plan pair and prepared English/Chinese registered source pair
were retained. Staged and untracked baseline inventories were empty.
The approved plan pair and active index/member bytes remain unchanged.
Both prepared source hashes remain exactly f507f29d91ddfc919ead6fa84df8d5f912b04af1d84ed5e68777e20c80c135c5
and 7d97f00f3a06723765e8cb2c8ea7ed13933defc8d7c230bdc0d165ad1284e7dc.
All fifteen attributed source paths changed; fixed result outputs are separately
owned artifacts. No unowned path, implementation ledger or original raw proof
was edited. No incidental repair was accepted.

## Batch outcomes

| Batch | Source key | Outcome | Evidence |
|---|---|---|---|
| B1 | C/P19-AUD-001 | Closed | Routes23/23; experiment35/35; parsed exact loopback admission; direct locked URL edge only. |
| B2 | C/P19-AUD-002 | Closed | Cardinality3/3; isolated huge/small/equal controls; new and retained caps1pass each. |
| B3 | P/P19-AUD-002 | Not closed | Flushed admitted starts/version/grammar pass; required real stage/deadline body unadmitted. |
| B4 | P/P19-AUD-003 | Not closed | Native names each list1; fixtures each execute1pass; actual native report/regrade bodies unadmitted. |
| B5 | P/P19-AUD-004 | Closed | Ignore reason material/7 agrees with consumer; intentionally0executed/1ignored. |
| B6 | C/P19-AUD-003 | Not closed | Admission positive+23 negatives; actual client four exact filters1pass each; seven native Docker gates absent and required all-targets union fails. |
| B7 | C/P19-AUD-004 | Not closed | Projection/semantic tests and full immutable f2 /2 structure pass; original operator/source-build/time qualification incomplete. |
| B8 | P/P19-AUD-001 | Not closed | Revised two-family coverage/consumers/source pair synchronized; complete required native/operator/final union remains incomplete. |

C is phase19-codex-gpt61-2ec1ccb-20261004t023807z, findings
bd454f15884f80e537772452aabc05593d73763e5f4500326edbf13e8d018bc4.
P is phase19-pi-glm53-9d805b4-20261004t094337z, findings
377d730be9a09201c6056b3513739fa2f524ec684019ae516ef04182da56eeaf.
Same textual IDs remain separate records. Original indexed states/severities,
source criteria and aggregate FAIL remain unchanged.

## Verification and retained evidence

Evidence root: `C:/Users/Luiz/AppData/Local/Temp/opi-phase19-remediate-apply-crtgd33k` (WSL `/mnt/c/Users/Luiz/AppData/Local/Temp/opi-phase19-remediate-apply-crtgd33k`).
`result-check-index.json` binds exact argv, exits, counts and raw log digests;
final receipts bind unchanged relevant source bytes and owned process groups.
The protocol and input-generator provisional runs are retained separately;
final inputv2/source runs supply current green evidence.

| Check | Outcome |
|---|---|
| `cargo test --locked -p opi-eval --test experiment_contract --test native_driver` | PASS35+122 executed;13 intentionally ignored. Every added/modified test binary ran. |
| `python3 -m unittest discover -s crates/opi-eval/scripts -p test_verify_local_acceptance.py` | PASS74/74. |
| Identical route/cardinality/max-projection/scope/f2-control probes | PASS23/23,3/3,2/2,2/2,4/4 plus actual observations4/4 and lock2/2. |
| Four required actual-client exact ignored filters | Each1pass on final source; loopback scripted provider, private HOME/environment and dummy keys. No Docker/model spend. |
| `cargo fmt --check --all` | PASS. |
| `cargo clippy --workspace --all-targets -- -D warnings` | PASS. |
| `cargo test --workspace --all-targets` | FAIL101 at opi-coding-agent json_streaming_cli:0pass/3fail. Later binaries were not reached. |
| `cargo test -p opi-eval --all-targets` | FAIL101 library:189pass/3fail. Required modified integration binaries ran separately and passed. |
| `cargo test --workspace --doc` | PASS12 executed doctests. |
| `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps` | PASS. |
| `python scripts/opi-doc-check.py`, `git diff --check` | PASS after fixed result materialization. |
| Active audit-set and fixed result validator | PASS; exact eight-source coverage and fifteen-path attribution. |
| Full f2 `/2` public verifier with exact current Companion | PASS exit0/accepted=true, four actual original trials. |
| Public CLI adversarial manifest/control check | PASS intended rejection:exit2/accepted=false, manifest controls differ from sealed experiment; no missing-file/schema shortcut. |

Cargo verification used offline locked inputs and the persistent external cache;
no cargo clean, new per-session target or cache deletion occurred. Cargo.lock was
generated by Cargo and changes only the existing url2.5.8 direct edge. Its
already locked pure-Rust URL/IDNA/ICU/proc-macro closure is reused; no new version,
native library or Opi internal runtime dependency was added.

## Concrete remaining gates

D2 initial native setup cannot currently prove the fixed no-implicit-fetch
boundary. Pinned Harbor initial Compose/buildx paths permit builds/pulls/network
without an offline guard. Existing candidate images and UV_OFFLINE do not govern
those paths; oracle/nop initial witnesses use upstream DockerEnvironment.
`d2-native-admission.json` records REJECTED before effects; `native-preflight.json`
records exact current task/package/base/image/source observations. Packages are
recoverable from immutable current-source-equivalent f2 copies, and candidate
Agent/verifier images exist. This is not a global absence or impossibility claim.
A newly qualified bounded Docker guard/build strategy needs its own admitted
source/tool/argv/context/image proof before the seven named native bodies run.
No default build, pull or native container effect was attempted by this apply.

D1 copied384 exact original material/lock/freeze/seal/capture/raw-command files
into a new evidence presentation. Origin checks and four fresh current Companion
verify/report commands pass. Native outcomes remain TB21 Opi/Pi1/1 and TB3
Opi/Pi0/1. Original outer argv/build-time and fully reproduced source acquisition
remain unavailable within inspected proof. `manifest.operator_provenance` keeps
those fields null; current offline commands do not fill them by inference.
Structural `/2` acceptance alone therefore does not close operator/Phase proof.
Later max-thinking and DeepSWE attempts remain intact and outside primary counts.

Six all-targets failures also reproduce in a unique clean git archive of e40,
not a Git worktree: three JSON streaming CLI cases exit with invalid provider
configuration before the local request; three generic profile assertions retain
old TB3/Deep task IDs and the old18000-second TB3 deadline against current
foodstuff/opa pins and10800 seconds. Exact baseline commands/counts/log hashes
and unchanged source-file digests are in `verification-blockers.json` and
baseline-*.json. They are observations outside this strict finding union,
not new finding IDs or accepted incidental repairs. Required-for-green alone
does not establish the approved causal/owned surface. Their source is preserved.

## Materialization and test impact

Changes remain uncommitted in the shared worktree. Fresh audit/re-run requires
fixes plus the current live set committed, a clean assurance directory, resolved
external evidence and completion of any owning-workflow return. This apply does
not change the implementation ledger, grant Phase PASS or invoke another skill.
The next plan/owning-authority decision must admit missing native/offline proof
and any separately owned baseline test repairs before complete closure.

Test impact: **add** isolated selection regression, private admission and
selected-family/start tests, standalone retained-family selector and fourteen
Python cases; **update** route/native names/annotation/current terminal/control/
scope fixtures; **retain** explicit caps, opaque pi0.85.1 fixtures, importer
fallback, privacy/provenance guards and standalone Deep capability; **delete** none.
The fixed result contract has no localized report sibling; README and registered
source counterparts are synchronized in English/Chinese.
