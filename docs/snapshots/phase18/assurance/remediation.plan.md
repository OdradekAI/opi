# Phase 18 Remediation Plan

**Status**: READY-FOR-APPLY
**Audit index SHA-256**: `60931e9889c2ee28896758522ca2073dcb48b2fc92e15dde510d6cdf71e9e815`
**Remediation head**: `b8877443444056cbb183515cfdad5bfb9b99c0d5`
**Disposition artifact**: `remediation.plan.dispositions.jsonl`
**Dirty-worktree baseline**: staged=`[]`; unstaged=`[]`; untracked=`[]`
**Unresolved decisions**: none

## Current Finding Verification

| Source run / Finding ID | Verification | Source and final severity + rationale | Closure key/family | Batch | Decision |
|---|---|---|---|---|---|
| `phase18-codex-gpt56-dd7eda7-20260831t135641z` / `P18-AUD-001` | Confirmed | Major -> Major. Current `ci-receipt.json` binds `e2e225fa0665c737542f71aefa27c963daf2bf73`, not the remediation head. Native run `33423005233` at that behaviorally identical implementation failed at `trial-opi-deepswe-v1.1` because `answer.txt` was absent. An independent Linux process reproduction showed Opi exited 0 after one provider request with `stop_reason=tool_use`, empty assistant content, zero tool results, and no `answer.txt`. The scripted provider places `finish_reason=tool_calls` on the same SSE chunk as the tool-call delta, while the repository's OpenAI fixtures require the tool delta to precede a separate terminal finish chunk. | `phase18.exact-candidate-native-agent-evidence` / `phase18.exit-evidence` | B1 | `fix:separate-stream-finish-require-final-workspace-and-refresh-evidence` |
| `phase18-pi-glm53-25d0e68-20260831t124752z` / `P18-AUD-001` | Refuted | Minor retained as source severity. At the remediation head there are no blanket `#[allow(dead_code)]` declarations in the cited files, `external_lock.rs` and its module declaration are absent, and `cargo clippy -p opi-eval --all-targets -- -D warnings` passes in the committed archive. | `opi-eval.no-masked-dead-surface` / `opi-eval.seam-hygiene` | none | `no-action:refuted-current-head` |
| `phase18-pi-glm53-25d0e68-20260831t124752z` / `P18-AUD-002` | Refuted | Minor retained as source severity. The current crate documentation describes the public entry surface by module rather than enumerating only two items, and `RUSTDOCFLAGS="-D warnings" cargo doc -p opi-eval --no-deps` passes in the committed archive. | `opi-eval.accurate-entry-surface-docs` / `opi-eval.seam-hygiene` | none | `no-action:refuted-current-head` |

## Unresolved Decisions

none

## Closure Batches

### Batch B1: Make deterministic native tool execution observable and refresh exact-candidate evidence

**Closure predicate**: The scripted provider emits tool-call data before a separate terminal SSE finish chunk; native Opi and pi conformance refuse success unless the real process leaves a non-empty `answer.txt` in the final workspace; the same committed implementation candidate then passes the repository's three-platform CI and the Phase 18 Linux native smoke for both Agents across all three registered benchmark revisions, with the downloaded artifacts verified against that candidate.
**Dependencies**: none for local repair; commit, push, CI dispatch, native dispatch, and receipt refresh occur only after separate user authorization at the materialization boundary.
**Verification union**: scripted-provider unit and subprocess tests; native-driver conformance test; `opi-eval` clippy; native producer/verifier tests; documentation and diff checks; Phase-exit workspace gates; exact-candidate PR CI receipt; exact-candidate Linux native artifact verification.

#### Fix B1.1: Separate streamed tool data from completion and require final-workspace proof

- **Finding source(s)**: `phase18-codex-gpt56-dd7eda7-20260831t135641z` + `dfb38314eca4a05e99e10a42bbbd594a91c413caafd90226a0ab2375ca9788ef` + `P18-AUD-001`
- **Decision**: Change the deterministic provider's streaming sequence so the role chunk and tool-call/content chunk carry `finish_reason=null`, followed by one empty terminal chunk carrying `tool_calls` or `stop`. Extend the provider test to assert that order. In native Agent conformance, require a non-empty final-workspace `answer.txt` and report an explicit verification note; update the native-driver test to prove the gate. Do not weaken the runner's fail-closed handling of a successfully completed Agent that omits its promised output. After local green, stop for commit/push authorization, then run and verify exact-candidate CI/native evidence and refresh `ci-receipt.json`.
- **Verification status**: Confirmed
- **File(s)**: `scripts/phase18-scripted-provider.py`, `scripts/test_phase18_scripted_provider.py`, `crates/opi-eval/src/cli/conformance.rs`, `crates/opi-eval/tests/native_driver.rs`, `docs/snapshots/phase18/ci-receipt.json`
- **Change kind**: behavioral
- **Change**: Make the fixture emit OpenAI-compatible streaming boundaries, add a regression assertion for the boundary, make native conformance validate the promised final-workspace output, retain that proof in its report, and bind refreshed CI/native evidence to the materialized implementation candidate.
- **Closure predicate**: A provider tool-call stream is consumed as one real `bash` call followed by a final assistant turn; native conformance fails if the answer is absent; the exact implementation candidate's verified native artifact covers Opi/pi on Terminal-Bench 2.1, Terminal-Bench 3.0, and DeepSWE v1.1, and its three-platform CI receipt is successful.
- **Red-before**: In a unique `git archive` of `b8877443444056cbb183515cfdad5bfb9b99c0d5`, adding the protocol-order assertion and running `python -m unittest scripts.test_phase18_scripted_provider.ScriptedProviderListenerTest.test_streaming_tool_call_turn_is_deterministic` failed with `AssertionError: 'tool_calls' is not None`. A direct Linux Opi subprocess against the current listener exited 0 with `answer=absent`, one request, empty assistant content, and no tool result. GitHub run `33423005233` allowed native conformance to pass before the six-trial stage failed with `expected agent output is unreadable`.
- **Green-after**: `python -m unittest scripts.test_phase18_scripted_provider.ScriptedProviderListenerTest.test_streaming_tool_call_turn_is_deterministic` and `cargo test -p opi-eval --test native_driver native_conformance_reruns_the_admitted_cases_through_the_material -- --exact` pass; after authorized materialization, the exact-candidate PR CI receipt and `python scripts/verify-phase18-native-artifact.py --criterion all-native --expected-commit <candidate> --receipt <downloaded-upload-receipt> --artifact <downloaded-native-artifact> --repo .` pass.

## Final Verification

    python scripts/test_phase18_scripted_provider.py
    python scripts/test_verify_phase18_native_ci.py
    python scripts/test_verify_phase18_native_artifact.py
    python scripts/test_verify_phase18_ci.py
    cargo test -p opi-eval --test native_driver native_conformance_reruns_the_admitted_cases_through_the_material -- --exact
    cargo test -p opi-eval --test agent_integration_conformance
    cargo clippy -p opi-eval --all-targets -- -D warnings
    python scripts/opi-doc-check.py
    git diff --check
    cargo fmt --check --all
    cargo clippy --workspace --all-targets -- -D warnings
    cargo test --workspace --all-targets
    cargo test --workspace --doc
    RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps

After the local union is green, stop and obtain explicit authorization before committing or pushing. Bind both remote runs to that implementation commit, not to an uncommitted worktree:

    $candidate = git rev-parse HEAD
    gh workflow run .github/workflows/phase18-native-smoke.yml --ref codex/phase18-remediation-e779477 -f "candidate_sha=$candidate"
    gh run watch <native-run-id> --exit-status
    gh run download <native-run-id> -n phase18-native-smoke -D <native-artifact-dir>
    gh run download <native-run-id> -n phase18-native-smoke-upload-receipt -D <native-receipt-dir>
    python scripts/verify-phase18-native-artifact.py --criterion all-native --expected-commit $candidate --receipt <downloaded-upload-receipt> --artifact <downloaded-native-artifact> --repo .

Use the successful PR CI run for the same candidate to regenerate and verify `docs/snapshots/phase18/ci-receipt.json` with `scripts/verify-phase18-ci.py --terminal`. A later receipt/remediation-result commit must contain no Phase 18 runtime, provider, adapter, verifier, workflow, or test changes relative to `$candidate`; record that diff explicitly in the result.

## Exclusions

| Finding ID | Disposition | Current evidence/authority |
|---|---|---|
| `phase18-pi-glm53-25d0e68-20260831t124752z` / `P18-AUD-001` | Refuted | Current run/digest controls admission; title similarity cannot substitute for identity; no older source was consulted. Current committed-archive scan found no blanket dead-code allow, no `external_lock` module/file, and focused clippy passed. |
| `phase18-pi-glm53-25d0e68-20260831t124752z` / `P18-AUD-002` | Refuted | Current run/digest controls admission; title similarity cannot substitute for identity; no older source was consulted. Current `lib.rs` module-level entry-surface wording and focused rustdoc check contradict the claim at the remediation head. |
