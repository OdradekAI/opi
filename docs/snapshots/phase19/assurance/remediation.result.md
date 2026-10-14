# Phase 19 Remediation Result

**Status**: COMPLETE
**Audit index SHA-256**: `23b39dcdc52f71dc1b17b4f87678f39aa3d86289c3753eb0b5f55373ad82873e`
**Plan SHA-256**: `571c76317637ab7acca5661f3c381dd8e7482648a9b8a8008d40c42e2a90e237`
**Changed paths**: ["crates/opi-eval/tests/native_driver.rs", "crates/opi-eval/tests/support/native_witness_material.rs", "docs/audits/phase19-local-native-acceptance.md", "docs/audits/phase19-local-native-acceptance.zh.md"]
**Disposition artifact**: `remediation.result.dispositions.jsonl`
**Remediation head**: `9ef3768968a559e0bc0366c1f3c66752d499595a`
**Apply HEAD**: `9ef3768968a559e0bc0366c1f3c66752d499595a`
**B1 outcome**: Not closed; new plan required

COMPLETE records exact result coverage and the execution stop. It does not
claim the approved repair predicate passed. Seven exact claims remain Refuted;
the component-conformance source remains Not closed. Native B1 and Phase
conformance are not granted.

## Admission and ownership

The explicit invocation approved the fixed plan digest above. Locked audit-set
and apply admission passed, including every indexed member and the strict union
of eight `(audit_run_id, id)` source keys. Current HEAD equals remediation_head.
Carried-in staged paths were [], untracked paths were [], and unstaged paths
were exactly the approved fixed plan pair. Their bytes are preserved. There
were no overlapping unowned changes or intervening source commits.

C = `phase19-codex-gpt61-2ec1ccb-20261004t023807z`, findings SHA-256
`bd454f15884f80e537772452aabc05593d73763e5f4500326edbf13e8d018bc4`.
P = `phase19-pi-glm53-9d805b4-20261004t094337z`, findings SHA-256
`377d730be9a09201c6056b3513739fa2f524ec684019ae516ef04182da56eeaf`.
Repeated textual IDs are separate records; source severity/rationale and every
approved stable field remain unchanged.

| Exact source alias / finding | Approved verification | Result disposition |
|---|---|---|
| C / P19-AUD-001 | Refuted | Refuted |
| C / P19-AUD-002 | Refuted | Refuted |
| C / P19-AUD-003 | Partially confirmed | Not closed |
| C / P19-AUD-004 | Refuted | Refuted |
| P / P19-AUD-001 | Refuted | Refuted |
| P / P19-AUD-002 | Refuted | Refuted |
| P / P19-AUD-003 | Refuted | Refuted |
| P / P19-AUD-004 | Refuted | Refuted |

## Applied work and native stop

The two private test files add the exact ignored actual-client regrade case and
bind TB3's sole canonical declared source to `/app/results.txt`. The new fixture
has exact Opi/pi source-to-fresh coverage assertions, a discriminating TB3
verifier baseline, bounded exact socket-refusal checks and retained Docker
query receipts. Both handoff documents now record current four-trial evidence,
partial native execution and the preserved attributed historical record.

Fresh inventories contain 138 total tests, 14 ignored native tests and twelve
selected exact native filters. Current selected runs execute two tests:
TB2.1 passes (official oracle/nop rewards 1/0; checker exit 0), and TB3 fails
(Cargo exit 101; checker exit 2 observes one failed test, not zero execution).
TB3 oracle grades 1 and collects `/app/results.txt`. Nop grades 0 upstream,
but submits no declared file; the production reader refuses it with
`native-binding-declared-artifact-missing`. It supplies neither a valid
incorrect-artifact observation nor a different collected digest.

Current native conformance admits only oracle/nop. Closing the predicate needs
an actual submitted incorrect artifact, beyond the approved four-path private
fixture/handoff scope. Making missing input valid would violate current
registered AC-002. No production, public schema/API, dependency, task script,
client package, source, manifest, lockfile or ledger change was made. No bounded
incidental source repair was admitted. The remaining ten native filters were
not executed; the added actual-client case has no native green claim.

Baseline and earlier attempts remain retained: a pre-task runner-cache refusal,
a reproduced obsolete-path assertion after oracle success, and the first
TB2.1 success whose merged nocapture stream was rejected by the line-based
counter. Final invocations preserve full independent stdout/stderr plus an
untrimmed complete transcript and original exit. No terminal line was invented
or output discarded. Eight retained witness roots and sixteen exact independent
engine queries show zero residual observed owned containers. Prepared immutable
image IDs remain available; original user evidence and shared caches survive.

## Verification and evidence

All raw commands, exits, full logs, material/client/build/source identities and
hashes are retained under `/home/luiz/opi-phase19-apply-c368e42d5144468ea33b93d233c25c6d`. Admission is mirrored in the Windows root
`C:/Users/Luiz/AppData/Local/Temp/opi-phase19-apply-c368e42d5144468ea33b93d233c25c6d`.

- Locked/offline current-source musl Companion build: PASS. SHA-256
  `1406e98d638354899fab559d24a75dc313f1bf47448f08b48dd9806253f258e2`.
- Public doctor/prepare and selected local-origin material/7: PASS. Relocated
  Harbor was synced from copied locked UV cache before fresh freeze; earlier
  frozen inputs/refusals are retained. Final composite SHA-256
  `c29d45781e10a271df1399edc964b5baf4bb6b212c1634b6a9fc07cb3a66274a`.
- Current four-trial public acceptance, two verify, four deterministic report
  calls and exhaustive two-Opi/24-file artifact view: PASS, all exits 0.
  117 preserved view hashes and all 28 original bundle/control-settlement
  hashes match; 24 nonprimary and six scoped-out DeepSWE trials stay separate.
- Fmt, Clippy with warnings denied, documentation contracts and diff check: PASS.
- Full workspace all-targets invocation on the verified real-workspace host
  target: FAIL, 4346 passed/2 failed/17 ignored before stop.
  The two unchanged opi-sandbox preparation/cleanup timing cases fail in the
  parallel library run. Each focused serial recheck passes; full sandbox serial
  recheck passes 206 tests with 2 ignored. Remaining TUI checks pass
  181 tests. The deduplicated final ordinary target union passes 4679
  tests with 19 ignored, while the original parallel failure remains disclosed.
  No sandbox source repair or claim of a passing original invocation is made.
- Workspace doc tests: PASS, 12 executed. Rustdoc with
  `RUSTDOCFLAGS="-D warnings"`: PASS. Exact loopback and cardinality checks:
  PASS, one test each.
- Initial export/default-cache Git-object gate failures remain retained. The
  actual commit exists; explicit host rebuild binds test depfiles to the real
  shared workspace. Default-cache binding is recompiled through Cargo without
  cache deletion; its focused original commit-object case is separately checked.

Key receipt groups: `native-union.receipts.json`, `host-source-bindings.json`,
`operator-verification/receipts.json`, `operator-verification/preservation.json`,
`final-verification/receipts.json`, `final-verification-host/receipts.json`,
`final-verification-remainder/receipts.json`, `sandbox-timing-rechecks/receipts.json`,
`final-material-check/receipts.json` and `owned-resource-check/receipts.json`.
The paired handoff contains exact native transcript hashes. Original provider
backend/outer argv/build-timestamp unknowns remain unknown; this run did not
perform new real-provider acceptance, native regrade, DeepSWE dispatch, upload,
push or release.

## Review and materialization

Four private-fixture proof defects were accepted and corrected: exact source
coverage, different TB3 baseline bytes, fail-closed socket absence and retained
engine-query receipts. Unconditional nonempty cleanup arrays were rejected by
current producer contracts: successful normal settlement may have already
removed containers, and capture restoration is used only for timed-out/cancelled
execution (`runner/experiment.rs`, `runner/native_cleanup.rs`). Independent
observed directory queries were retained instead. Fresh request/observation/
native-result/argv/settlement binding remains explicit; the new body was not run.

Test impact: **add** one ignored actual-client regrade case; **update** one TB3
witness assertion; **retain** existing ordinary/native/compatibility tests;
**delete** none. There is no user-visible runtime or 0.x contract change.

The approved plan, live index/member bytes and carried-in work are preserved.
The four attributed source/document paths and fixed result pair are uncommitted;
this invocation grants no commit or Phase PASS. A new plan must resolve the TB3
negative-witness scope and assess the disclosed workspace timing observation.
Fresh audit/reviewer execution remains beyond the materialization boundary:
fixes plus the live set must be committed, assurance clean, external evidence
resolvable and the owning-workflow return complete first. No other explicit-only
workflow was invoked.
