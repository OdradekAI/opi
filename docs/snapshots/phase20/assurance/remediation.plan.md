# Phase 20 Remediation Plan

**Status**: READY-FOR-APPLY
**Audit index SHA-256**: `fb22a4ff86318a1f322cfbcf9bf107d748d65bbaa7c1ccb1037f6496d69db3ce`
**Remediation head**: `8010ed2af97cd5deeb1f7240097436fdf32b8d54`
**Disposition artifact**: `remediation.plan.dispositions.jsonl`
**Dirty-worktree baseline**: worktree-only; original staged=[]; unstaged=[]; untracked=[]. Continuation carried in the two task-owned fixed plan files; user explicitly requested their advancement. committed=none-for-this-task in the shared worktree.
**Unresolved decisions**: none

All24 current source keys remain separate:22 planned fixes and2 no-action dispositions. Source severities and original audit states are retained. This plan selects repairs and their future green tests; it does not claim repairs are applied, rewrite requirements or declare the Phase conformant. No partial apply is admitted.

## Admission and exact current set

The active index and both indexed metadata/requirements/findings groups were validated before consuming semantic input. The continuation `audit-set` CLI reports an incomplete four-file remediation group because only the fixed plan pair exists; the locked `plan` command uses the implemented `allow_partial_remediation=True` path and revalidates the full same indexed set, sidecars and report bindings without manufacturing result files. Index bytes remain unchanged.

| Run | Audit head | Findings SHA-256 |
|---|---|---|
| `phase20-codex-gpt61-703d8bd-20261101t181806z` | `703d8bd8f5033e5ecc89fc1950fc6b7615b64068` | `1c60c2f6c88d4d5318822c7412fb4a3fd5d48287253bc661d36ba91e7c2a62ef` |
| `phase20-pi-glm53-6adacc37-20261101t164203z` | `6adacc379df15ff2e5fa66ddb2afd3abeef5bb89` | `fd1c6180b3c1951fa82efc2bdf2244af006ea8db1feca261c90267c59a881e1a` |

Production inspection/probes used unique git-archive exports at `C:\Users\Luiz\AppData\Local\Temp\opi-phase20-remediate-hvrszrbs\source` and `C:\Users\Luiz\AppData\Local\Temp\opi-phase20-ready-w8p6zu1s\source`. The latter is a fresh pristine export without temporary probe additions and owns package proofs. No Git worktree was used. The user separately authorized an isolated verification clone/remote branch, only verifier/workflow commits and bounded macOS CI retries (cumulative runner budget90minutes); the shared worktree HEAD, branch, source, ledgers and audit set remain unchanged. Remote verifier source `e433ac833a28ece771ec8dbb00f947efbd592055` has zero relevant diff against remediation head for crates/Cargo/scripts/CI/registered SDK sources/parent spec; local audit commits were not published.

Both validated indexed appendices are no-history. No prior assurance conclusion/history/eval or unindexed old raw proof supplied closure authority. Current pinned upstream files and exact current CI run explicitly cited by finding020 were independently checked as source/evidence inputs. New observations below are current planning proof, not inherited closure. Current finding identity is full run/digest/ID; duplicated textual IDs and titles never merge sources.

## Current Finding Verification

| Source | Current verification / decision | Source -> final severity | Closure key / family | Batch | Planning obligation |
|---|---|---|---|---|---|
| `phase20-codex-gpt61-703d8bd-20261101t181806z` / P20-AUD-001 | Confirmed; fix:bedrock.early-prelude-validation; Retain source Major: Bedrock buffers declared payload before rejecting an invalid prelude. | Major -> Major | `bedrock.early-prelude-validation` / `stream.framing` | B1 | none; future apply green remains required |
| `phase20-codex-gpt61-703d8bd-20261101t181806z` / P20-AUD-002 | Confirmed; fix:ecs.parsed-http-host; Retain source Major: ECS HTTP host guard accepts externally resolvable prefix domains. | Major -> Major | `ecs.parsed-http-host` / `auth.aws-metadata` | B2 | none; future apply green remains required |
| `phase20-codex-gpt61-703d8bd-20261101t181806z` / P20-AUD-003 | Confirmed; fix:sse.multiline-crlf-fragmentation; Retain source Major: Retain incomplete newline framing across chunks and normalize only complete boundaries; use the existing stream framing ownership rather than a new public policy. | Major -> Major | `sse.multiline-crlf-fragmentation` / `stream.framing` | B3 | none; future apply green remains required |
| `phase20-codex-gpt61-703d8bd-20261101t181806z` / P20-AUD-004 | Confirmed; fix:deferred.cancel-auth-wait; Retain source Major: Deferred cancellation ignores cancellation while awaiting authentication. | Major -> Major | `deferred.cancel-auth-wait` / `models.cancellation` | B4 | none; future apply green remains required |
| `phase20-codex-gpt61-703d8bd-20261101t181806z` / P20-AUD-005 | Confirmed; fix:diagnostics.overflow-arithmetic; Retain source Major: Context overflow helper panics on a large positive public context window. | Major -> Major | `diagnostics.overflow-arithmetic` / `public.utilities` | B5 | none; future apply green remains required |
| `phase20-codex-gpt61-703d8bd-20261101t181806z` / P20-AUD-006 | Confirmed; fix:complete-owned-schema-lifecycle-work-stack; Retain source Major: Stack safety covers the evaluator but not admission, normalization or owned destruction. | Major -> Major | `schema.owned-lifecycle.stack-safety` / `schema.lifecycle` | B6 | none; future apply green remains required |
| `phase20-codex-gpt61-703d8bd-20261101t181806z` / P20-AUD-007 | Confirmed; fix:adc.checked-expiration; Retain source Major: ADC expiry arithmetic can panic on a source response. | Major -> Major | `adc.checked-expiration` / `auth.adc-boundary` | B7 | none; future apply green remains required |
| `phase20-codex-gpt61-703d8bd-20261101t181806z` / P20-AUD-008 | Confirmed; fix:adc.aws-region-utf8; Retain source Major: AWS subject region decoding slices through UTF-8. | Major -> Major | `adc.aws-region-utf8` / `auth.adc-boundary` | B8 | none; future apply green remains required |
| `phase20-codex-gpt61-703d8bd-20261101t181806z` / P20-AUD-009 | Confirmed; fix:duration.first-response-poll; Retain source Major: Move owning response activation timestamp to first poll/activation before the body await, preserving setup failure unknownness. | Major -> Major | `duration.first-response-poll` / `stream.duration` | B9 | none; future apply green remains required |
| `phase20-codex-gpt61-703d8bd-20261101t181806z` / P20-AUD-010 | Confirmed; fix:propagate-frozen-call-projection-policy-and-facts; Retain source Major: Public selected-fidelity ingress and delivery of existing required projection facts are missing. | Major -> Major | `projection.public-fidelity-and-facts` / `transcript.projection` | B10 | none; future apply green remains required |
| `phase20-codex-gpt61-703d8bd-20261101t181806z` / P20-AUD-011 | Confirmed; fix:context.independent-total-anchor; Retain source Major: Evaluate known positive usage.total_tokens before the is_reported parent-bucket fallback gate; update the existing assertion that encodes the narrowed rule. | Major -> Major | `context.independent-total-anchor` / `context.estimation` | B11 | none; future apply green remains required |
| `phase20-codex-gpt61-703d8bd-20261101t181806z` / P20-AUD-012 | Confirmed; fix:api-id.exact-arbitrary-registration; Retain source Major: Remove only invented ApiId trim/nonempty guards and their stale registration documentation/assertions; preserve provider identity validation and existing exact-key/upsert/type mismatch rules. | Major -> Major | `api-id.exact-arbitrary-registration` / `models.registration` | B12 | none; future apply green remains required |
| `phase20-codex-gpt61-703d8bd-20261101t181806z` / P20-AUD-013 | Confirmed; fix:preserve-full-source-qualified-compat-through-wire-compilation; Retain source Major: full source-qualified38-field map is now complete against8 pinned files, and15 new Chat/Responses controls plus the earlier maximum/cache negative are reproduced through real decoded models/builders/HTTP. | Major -> Major | `compat.source-fields-to-real-wire` / `models.compat` | B17 | none; future apply green remains required |
| `phase20-codex-gpt61-703d8bd-20261101t181806z` / P20-AUD-014 | Confirmed; fix:complete-instance-owned-provider-and-operation-management; Retain source Major: Registered spec-draft262-268 and AIR-034 explicitly require these SDK actions; implement only their already-adopted meaning once the typed signatures and removal/publication ownership are concrete. | Major -> Major | `models.provider-management-and-selectors` / `models.management` | B13 | none; future apply green remains required |
| `phase20-codex-gpt61-703d8bd-20261101t181806z` / P20-AUD-015 | Confirmed; fix:make-credential-process-limits-explicit; Retain source Major: Make the ordinary runner follow the selected uncapped source behavior; retain explicit optional caller policy and current cancellation/process ownership. | Major -> Major | `aws.credential-process.opt-in-limits` / `auth.aws-process` | B14 | none; future apply green remains required |
| `phase20-codex-gpt61-703d8bd-20261101t181806z` / P20-AUD-016 | Confirmed; fix:http.source-qualified-redirect-budget; Retain source Major: Bind explicit pinned generation redirect budget to its source route/default client assembly rather than retaining backend10; avoid assigning one arbitrary redirect budget to unrelated AWS/ADC sources. | Major -> Major | `http.source-qualified-redirect-budget` / `transport.redirects` | B15 | none; future apply green remains required |
| `phase20-codex-gpt61-703d8bd-20261101t181806z` / P20-AUD-017 | Confirmed; fix:strict.required-shape-admission; Retain source Major: Distinguish absent required from present wrong-type required before extracting required names; preserve existing entry/unknown-name checks and local-schema inert guards. | Major -> Major | `strict.required-shape-admission` / `schema.strict` | B16 | none; future apply green remains required |
| `phase20-codex-gpt61-703d8bd-20261101t181806z` / P20-AUD-018 | Confirmed; fix:execute-same-source-packaged-onboarding-with-builtins; Retain source Major: Packaged onboarding proof compiles examples but never executes the required same-source flows. | Major -> Major | `onboarding.same-source-package-execution` / `delivery.onboarding` | B18 | none; future apply green remains required |
| `phase20-codex-gpt61-703d8bd-20261101t181806z` / P20-AUD-019 | Confirmed; fix:scope-real-sdk-stream-poll-and-drop-with-frozen-context; Retain source Major: Telemetry Stream scope proof is implemented by a test-only stream. | Major -> Major | `telemetry.stream-poll-drop-context` / `telemetry.carrier` | B19 | none; future apply green remains required |
| `phase20-codex-gpt61-703d8bd-20261101t181806z` / P20-AUD-020 | Refuted; no-action:current-native-package-and-material-proof-established; Retain original Major priority; the exact missing-proof claim is refuted at current source by newly executed native actual-package matrices and sealed pinned catalog material. This does not refute separate runtime findings or declare Phase conformance. | Major -> Major | `delivery.final-package-material-proof` / `delivery.provenance` | none | none; future apply green remains required |
| `phase20-codex-gpt61-703d8bd-20261101t181806z` / P20-AUD-021 | Confirmed; fix:reject-programmatic-duplicate-section-batches; Retain source Minor: Programmatic duplicate prompt sections bypass validation. | Minor -> Minor | `transcript.duplicate-section-batch-rejection` / `transcript.declaration-validation` | B20 | none; future apply green remains required |
| `phase20-codex-gpt61-703d8bd-20261101t181806z` / P20-AUD-022 | Confirmed; fix:add-schema-cardinality-boundary-corpus; Retain source Minor: Four schema cardinality keywords lack discriminating rejection coverage. | Minor -> Minor | `schema.cardinality-discriminating-corpus` / `schema.coverage` | B21 | none; future apply green remains required |
| `phase20-pi-glm53-6adacc37-20261101t164203z` / P20-AUD-001 | Refuted; no-action:current-native-package-proof-established; Retain source Info; actual original CI logs independently prove macOS/aarch64 execution and dispatch at unchanged relevant source. New actual-package provenance is separately verified; no product edit or new Phase verdict follows. | Info -> Info | `delivery.native-dispatch-absence` / `delivery.platform` | none | none; future apply green remains required |
| `phase20-pi-glm53-6adacc37-20261101t164203z` / P20-AUD-002 | Confirmed; fix:append-factual-assertion-deletion-tally-correction; Retain source Minor: V-59 phase tally records delete = 0 while a scoped assertion deletion occurred. | Minor -> Minor | `delivery.assertion-deletion-tally` / `delivery.documentation` | B22 | none; future apply green remains required |

### Exact linked criterion matrix

All405 indexed requirements were parsed; linked rows retain their exact path/SHA/citation, IDs, original states and reciprocal finding IDs. Different criterion identities remain separate. This is a coverage cross-reference, not a revised audit verdict.

| Member | Requirement / original state | Exact criterion path / SHA-256 / citation | Finding IDs |
|---|---|---|---|
| audit.codex.gpt61 | FRAME-01 / not-met | `docs/research/pi-ai-opi-ai-alignment/spec-draft.md` / `8c6cd66f7771e026210f06e49d037da54029c74258a1d45925cdbddd710f5f21` / AIR-006 | P20-AUD-003 |
| audit.codex.gpt61 | FRAME-03 / partially-met | `docs/research/pi-ai-opi-ai-alignment/spec-draft.md` / `8c6cd66f7771e026210f06e49d037da54029c74258a1d45925cdbddd710f5f21` / AIR-006 | P20-AUD-001 |
| audit.codex.gpt61 | CONTEXT-05 / partially-met | `docs/research/pi-ai-opi-ai-alignment/spec-draft.md` / `8c6cd66f7771e026210f06e49d037da54029c74258a1d45925cdbddd710f5f21` / AIR-008/049 | P20-AUD-021 |
| audit.codex.gpt61 | CONTEXT-06 / partially-met | `docs/research/pi-ai-opi-ai-alignment/spec-draft.md` / `8c6cd66f7771e026210f06e49d037da54029c74258a1d45925cdbddd710f5f21` / AIR-011 | P20-AUD-010 |
| audit.codex.gpt61 | CONTEXT-07 / partially-met | `docs/research/pi-ai-opi-ai-alignment/spec-draft.md` / `8c6cd66f7771e026210f06e49d037da54029c74258a1d45925cdbddd710f5f21` / AIR-011 | P20-AUD-010 |
| audit.codex.gpt61 | REPLAY-13 / partially-met | `docs/research/pi-ai-opi-ai-alignment/spec-draft.md` / `8c6cd66f7771e026210f06e49d037da54029c74258a1d45925cdbddd710f5f21` / AIR-038 | P20-AUD-010 |
| audit.codex.gpt61 | DURATION-01 / not-met | `docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-rust-differences.md` / `411d2f6d262b1afc22e84c73213dffd0eb890ed70e210dfa497d2a5d021cd31d` / RDF-02 | P20-AUD-009 |
| audit.codex.gpt61 | P20-AT-HTTP-ROUTING / partially-met | `docs/research/pi-ai-opi-ai-alignment/spec-draft.md` / `8c6cd66f7771e026210f06e49d037da54029c74258a1d45925cdbddd710f5f21` / docs/research/pi-ai-opi-ai-alignment/spec-draft.md:474 | P20-AUD-016 |
| audit.codex.gpt61 | P20-AT-HTTP-PACKAGE / not-assessable | `docs/research/pi-ai-opi-ai-alignment/spec-draft.md` / `8c6cd66f7771e026210f06e49d037da54029c74258a1d45925cdbddd710f5f21` / docs/research/pi-ai-opi-ai-alignment/spec-draft.md:474 | P20-AUD-020 |
| audit.codex.gpt61 | P20-AT-CRYPTO-PROVENANCE / not-assessable | `docs/research/pi-ai-opi-ai-alignment/spec-draft.md` / `8c6cd66f7771e026210f06e49d037da54029c74258a1d45925cdbddd710f5f21` / docs/research/pi-ai-opi-ai-alignment/spec-draft.md:482 | P20-AUD-020 |
| audit.codex.gpt61 | P20-AT-TLS-PACKAGE / not-assessable | `docs/research/pi-ai-opi-ai-alignment/spec-draft.md` / `8c6cd66f7771e026210f06e49d037da54029c74258a1d45925cdbddd710f5f21` / docs/research/pi-ai-opi-ai-alignment/spec-draft.md:484 | P20-AUD-020 |
| audit.codex.gpt61 | P20-AT-WS-GRAPHS / not-assessable | `docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-ws.md` / `61547ccfc0549018d2fd4042ade8edac3547599b412e12ea2479821b40f67a8f` / docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-ws.md:109 | P20-AUD-020 |
| audit.codex.gpt61 | P20-AT-AWS-LOCAL / partially-met | `docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-rust-backends.md` / `acb7f5f798b3384c63d36ad9af844b580755e2ea792ce3e2d32d9d3af2f15e81` / docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-rust-backends.md:130 | P20-AUD-015 |
| audit.codex.gpt61 | P20-AT-AWS-ECS / partially-met | `docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-rust-backends.md` / `acb7f5f798b3384c63d36ad9af844b580755e2ea792ce3e2d32d9d3af2f15e81` / docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-rust-backends.md:130 | P20-AUD-002 |
| audit.codex.gpt61 | P20-AT-ADC-NATIVE / partially-met | `docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-rust-backends.md` / `acb7f5f798b3384c63d36ad9af844b580755e2ea792ce3e2d32d9d3af2f15e81` / docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-rust-backends.md:160 | P20-AUD-007 |
| audit.codex.gpt61 | P20-AT-ADC-METADATA / partially-met | `docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-rust-backends.md` / `acb7f5f798b3384c63d36ad9af844b580755e2ea792ce3e2d32d9d3af2f15e81` / docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-rust-backends.md:160 | P20-AUD-007 |
| audit.codex.gpt61 | P20-AT-ADC-AWS1 / partially-met | `docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-rust-backends.md` / `acb7f5f798b3384c63d36ad9af844b580755e2ea792ce3e2d32d9d3af2f15e81` / docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-rust-backends.md:160 | P20-AUD-008 |
| audit.codex.gpt61 | P20-SU-PROFILE-CONSUMER / not-assessable | `docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-rust-differences.md` / `411d2f6d262b1afc22e84c73213dffd0eb890ed70e210dfa497d2a5d021cd31d` / RDF-21 | P20-AUD-020 |
| audit.codex.gpt61 | P20-SU-KW-minProperties / partially-met | `docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-schema.md` / `878d15231e734dc03772e1f1bd48cb8ffe2252a2818748e4b8630af43a2e8a76` / §3.1–3.4 / minProperties | P20-AUD-022 |
| audit.codex.gpt61 | P20-SU-KW-maxProperties / partially-met | `docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-schema.md` / `878d15231e734dc03772e1f1bd48cb8ffe2252a2818748e4b8630af43a2e8a76` / §3.1–3.4 / maxProperties | P20-AUD-022 |
| audit.codex.gpt61 | P20-SU-KW-maxItems / partially-met | `docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-schema.md` / `878d15231e734dc03772e1f1bd48cb8ffe2252a2818748e4b8630af43a2e8a76` / §3.1–3.4 / maxItems | P20-AUD-022 |
| audit.codex.gpt61 | P20-SU-KW-minItems / partially-met | `docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-schema.md` / `878d15231e734dc03772e1f1bd48cb8ffe2252a2818748e4b8630af43a2e8a76` / §3.1–3.4 / minItems | P20-AUD-022 |
| audit.codex.gpt61 | P20-SU-RECURSION / partially-met | `docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-schema.md` / `878d15231e734dc03772e1f1bd48cb8ffe2252a2818748e4b8630af43a2e8a76` / §3.2 | P20-AUD-006 |
| audit.codex.gpt61 | P20-SU-RESOLVER / partially-met | `docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-schema.md` / `878d15231e734dc03772e1f1bd48cb8ffe2252a2818748e4b8630af43a2e8a76` / §3.3 | P20-AUD-006 |
| audit.codex.gpt61 | P20-SU-STRICT / partially-met | `docs/research/pi-ai-opi-ai-alignment/spec-draft.md` / `8c6cd66f7771e026210f06e49d037da54029c74258a1d45925cdbddd710f5f21` / AIR-040 / L401 / TP-14 | P20-AUD-017 |
| audit.codex.gpt61 | P20-SU-OVERFLOW / partially-met | `docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md` / `9e3d6ef690c3a794bc9c47ddc2dce04840f98d8881649be1ab2e8db0df287fd5` / §7 diagnostics/overflow/error parsing | P20-AUD-005 |
| audit.codex.gpt61 | P20-SU-TEL-DROP / partially-met | `docs/research/pi-ai-opi-ai-alignment/spec-draft.md` / `8c6cd66f7771e026210f06e49d037da54029c74258a1d45925cdbddd710f5f21` / L314 | P20-AUD-019 |
| audit.codex.gpt61 | M20-CUSTOM-API-ID / not-met | `docs/research/pi-ai-opi-ai-alignment/spec-draft.md` / `8c6cd66f7771e026210f06e49d037da54029c74258a1d45925cdbddd710f5f21` / docs/research/pi-ai-opi-ai-alignment/spec-draft.md:514 | P20-AUD-012 |
| audit.codex.gpt61 | M20-SELECTORS / partially-met | `docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md` / `9e3d6ef690c3a794bc9c47ddc2dce04840f98d8881649be1ab2e8db0df287fd5` / docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md:21 | P20-AUD-014 |
| audit.codex.gpt61 | M20-CAT-ARTIFACT / not-assessable | `docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-runtime.md` / `1328a81e561e1e1bfb2c35df788ce6d08fec2f71e14eb8abba9d57e1f03f1c7b` / docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-runtime.md:33 | P20-AUD-020 |
| audit.codex.gpt61 | M20-CAT-REFRESH / partially-met | `docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md` / `9e3d6ef690c3a794bc9c47ddc2dce04840f98d8881649be1ab2e8db0df287fd5` / docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md:201 | P20-AUD-014 |
| audit.codex.gpt61 | M20-BUDGET-NATIVE / partially-met | `docs/research/pi-ai-opi-ai-alignment/spec-draft.md` / `8c6cd66f7771e026210f06e49d037da54029c74258a1d45925cdbddd710f5f21` / docs/research/pi-ai-opi-ai-alignment/spec-draft.md:357 | P20-AUD-013 |
| audit.codex.gpt61 | M20-ANCHOR / not-met | `docs/research/pi-ai-opi-ai-alignment/spec-draft.md` / `8c6cd66f7771e026210f06e49d037da54029c74258a1d45925cdbddd710f5f21` / docs/research/pi-ai-opi-ai-alignment/spec-draft.md:373 | P20-AUD-011 |
| audit.codex.gpt61 | M20-COMPAT-ROUTE / partially-met | `docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md` / `9e3d6ef690c3a794bc9c47ddc2dce04840f98d8881649be1ab2e8db0df287fd5` / docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md:174 | P20-AUD-013 |
| audit.codex.gpt61 | M20-COMPAT-PERWIRE / partially-met | `docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md` / `9e3d6ef690c3a794bc9c47ddc2dce04840f98d8881649be1ab2e8db0df287fd5` / docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md:175 | P20-AUD-013 |
| audit.codex.gpt61 | M20-DEFER-HANDLE / partially-met | `docs/research/pi-ai-opi-ai-alignment/spec-draft.md` / `8c6cd66f7771e026210f06e49d037da54029c74258a1d45925cdbddd710f5f21` / docs/research/pi-ai-opi-ai-alignment/spec-draft.md:201 | P20-AUD-004 |
| audit.codex.gpt61 | M20-DEFER-CONSUMER / partially-met | `docs/research/pi-ai-opi-ai-alignment/spec-draft.md` / `8c6cd66f7771e026210f06e49d037da54029c74258a1d45925cdbddd710f5f21` / docs/research/pi-ai-opi-ai-alignment/spec-draft.md:202 | P20-AUD-004 |
| audit.codex.gpt61 | M20-CAPABILITY-PROOF / partially-met | `docs/research/pi-ai-opi-ai-alignment/spec-draft.md` / `8c6cd66f7771e026210f06e49d037da54029c74258a1d45925cdbddd710f5f21` / docs/research/pi-ai-opi-ai-alignment/spec-draft.md:185 | P20-AUD-014, P20-AUD-013 |
| audit.codex.gpt61 | M20-PACKAGE / not-assessable | `docs/research/pi-ai-opi-ai-alignment/spec-draft.md` / `8c6cd66f7771e026210f06e49d037da54029c74258a1d45925cdbddd710f5f21` / docs/research/pi-ai-opi-ai-alignment/spec-draft.md:215 | P20-AUD-020 |
| audit.codex.gpt61 | M20-ONBOARD-README / partially-met | `docs/research/pi-ai-opi-ai-alignment/spec-draft.md` / `8c6cd66f7771e026210f06e49d037da54029c74258a1d45925cdbddd710f5f21` / docs/research/pi-ai-opi-ai-alignment/spec-draft.md:554 | P20-AUD-018 |
| audit.codex.gpt61 | M20-ONBOARD-CONSUMER / partially-met | `docs/research/pi-ai-opi-ai-alignment/spec-draft.md` / `8c6cd66f7771e026210f06e49d037da54029c74258a1d45925cdbddd710f5f21` / docs/research/pi-ai-opi-ai-alignment/spec-draft.md:555 | P20-AUD-018 |
| audit.codex.gpt61 | M20-ONBOARD-PARAMS / partially-met | `docs/research/pi-ai-opi-ai-alignment/spec-draft.md` / `8c6cd66f7771e026210f06e49d037da54029c74258a1d45925cdbddd710f5f21` / docs/research/pi-ai-opi-ai-alignment/spec-draft.md:556 | P20-AUD-013 |
| audit.codex.gpt61 | M20-ONBOARD-FACTORY / partially-met | `docs/research/pi-ai-opi-ai-alignment/spec-draft.md` / `8c6cd66f7771e026210f06e49d037da54029c74258a1d45925cdbddd710f5f21` / docs/research/pi-ai-opi-ai-alignment/spec-draft.md:558 | P20-AUD-013 |
| audit.codex.gpt61 | M20-ONBOARD-PLATFORM / not-assessable | `docs/research/pi-ai-opi-ai-alignment/spec-draft.md` / `8c6cd66f7771e026210f06e49d037da54029c74258a1d45925cdbddd710f5f21` / docs/research/pi-ai-opi-ai-alignment/spec-draft.md:559 | P20-AUD-020 |
| audit.codex.gpt61 | M20-NATIVE-PROOF / not-assessable | `docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-runtime.md` / `1328a81e561e1e1bfb2c35df788ce6d08fec2f71e14eb8abba9d57e1f03f1c7b` / docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-runtime.md:43 | P20-AUD-020 |
| audit.codex.gpt61 | M20-SUPPLY-CHAIN / not-assessable | `docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-runtime.md` / `1328a81e561e1e1bfb2c35df788ce6d08fec2f71e14eb8abba9d57e1f03f1c7b` / docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-runtime.md:39 | P20-AUD-020 |
| audit.codex.gpt61 | M20-PACKAGE-WS / not-assessable | `docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-rust-backends.md` / `acb7f5f798b3384c63d36ad9af844b580755e2ea792ce3e2d32d9d3af2f15e81` / docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-rust-backends.md:212 | P20-AUD-020 |
| audit.codex.gpt61 | M20-API-openai-completions / partially-met | `docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md` / `9e3d6ef690c3a794bc9c47ddc2dce04840f98d8881649be1ab2e8db0df287fd5` / docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md:32 | P20-AUD-013 |
| audit.codex.gpt61 | M20-API-openai-responses / partially-met | `docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md` / `9e3d6ef690c3a794bc9c47ddc2dce04840f98d8881649be1ab2e8db0df287fd5` / docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md:34 | P20-AUD-013 |
| audit.pi.glm53 | T20.58 / met | `docs/snapshots/phase20/opi-impl-state.json` / `222372715140f4d66ae43ed6c39779daca50f167e0407503d252536bcbd86bb8` / task 20.58 '真实 native 平台／MSRV／feature 与供应链交付验收' (parent row DEL-14; scenarios: V-58, P20-S58, P20-EXIT-06, P20-GAP-SCHEMA, P20-GAP-WS, P20-GAP-HTTP, P20-GAP-CRYPTO, P20-GAP-AUTH, P20-GAP-TELEMETRY, ONB-06-20.58) | P20-AUD-001 |
| audit.pi.glm53 | T20.59 / met | `docs/snapshots/phase20/opi-impl-state.json` / `222372715140f4d66ae43ed6c39779daca50f167e0407503d252536bcbd86bb8` / task 20.59 '上游跟进／测试处置与交付证据基线' (parent row DEL-14; scenarios: V-59, AIR-067, P20-S59, P20-ACTOR-10, P20-EXIT-08, P20-GAP-DELIVERY, DEL-14, ONB-07-20.59) | P20-AUD-002 |

## Unresolved Decisions

none

## Planning Evidence Work

| Former item | Determined closure decision and evidence |
|---|---|
| D1 | Complete the existing explicit work-stack throughout SDK-owned schema admission/normalization/clone/destruction; three independent corrected deep-phase reds already observed. No new public seam, cap, leak or larger stack. |
| D2 | One frozen request projection policy and typed prepared/partial/terminal projection facts; actual successful lossy Chat wire currently serializes no facts. Real behavioral red, no missing-symbol compiler assertion. |
| D3 | Complete38-field source map(27Chat+11Responses),8 matching pinned source files,15 new actual wire negatives plus earlier maximum/cache case. Per-wire Azure/Codex exceptions, false/omission/null/zero and nested routing precedence fixed. |
| D4 | One instance-owned provider/operation inventory. A real image-only registration is absent from current provider_ids; new management/selected-refresh behavior is fully specified, including captured revisions/shared-store ACK ownership. |
| D5 | Default process policyNone; explicit with_policySome and policy()->Option. Coding-agent explicitly keeps its intentional10s/64KiB. Actual current lawful>64KiB default failure red retained. |
| D6 | Use existing builtins/factories, complete packaged examples and loopback peer1/1/2flows. Current real harness reports PASS with0 executions after actual artifact/embedding controls; controlled executor is disclosed, not package runtime proof. |
| D7 | Private pinned Stream wrapper around actual native ResourceApiStream/shared PreparedAttempt owner/deferred returned stream. Current public plain adapter leaks Pending/drop observations to ambientB;1executed1failed and exact business counts. |
| D8 | New current catalog material seal/packer validation; original cited3OS/MSRV CI logs; newly executed actual-package matrices with native graphs/materials. Only applicable original executed artifacts close proof availability. |
| D9 | Append factual >=1 constructor-assertion deletion correction. P.0/P.2/P.4 is the determined later ledger/hash/task/Phase handoff boundary; no ledger mutation or new source meaning is authorized. |

All new probes use fixed exported production/rLib inputs, positive/negative controls and bounded local peers/subprocesses; no live provider/credentials-triggered work. Original constructor/compiler/archive-symlink failures and CI setup failures are not behavioral reds. New runtime red aggregate17executed/17failed/0ignored/8filtered; existing8behavior reds retained. New telemetry1failed and onboarding Python harness1failed have independently logged controls/limitations. The earlier corrected auth/deep/duplicate/prelude evidence remains bound in the reproducible evidence appendix.

## Concrete resolved designs

These implement already registered AIR/ONB behavior. Signature/representation choices are scoped0.x caller migrations, not new product scope, authority or Pi1.4 adoption. Dependencies/unsafe/native code/default features remain unchanged.

### Design for P20-AUD-006
Complete the existing explicit work-stack model across every schema-depth-dependent stage of the SDK-owned compiled-schema lifecycle. This is one bounded schema-owner repair, without a new public seam, schema rejection policy, or dependency.

- refs.rs: replace register_resource_anchors and unregister_resource_anchors recursion by explicit preorder worklists. Retain root=true handling, nested-$id pruning, dynamic-anchor push/pop order, source js_keys enumeration and array order. Push children in reverse onto LIFO to retain current visit order.
- refs.rs: replace find_dynamic_anchor and find_base recursion by explicit worklists. Retain first-match order, equality-before-$id transition in find_base, branch-local base URL propagation, invalid-join branch failure and sibling search. This preserves lexical/resource/dynamic/recursive authority; do not build a global ID table.
- normalize.rs: replace schema-dependent recursive optional-null/coercion helpers by private continuation frames. Own the current Value in each frame; retain source allOf sequential fold, union original-first then clone-per-arm selection, object/additionalProperties and tuple/single-items order, independent SubValidators compiles, and final validation against original schema. Return child results through owned frames with mem::take and safe reinsertion; no raw mutable pointers or public traits.
- Provide a private iterative JSON clone for SubValidators::get(schema) instead of serde_json Value::clone on deeply nested schema. Keep the cache and independently compiled child schema semantics. Share the private value traversal utility with owned destruction only where both consumers need it; no public export.
- ir.rs: implement safe iterative Compiled destruction by replacing/detaching its Box<Value> root with Null and draining owned Array/Object descendants onto a Vec before their containers drop. Preserve the existing Box allocation and node-pointer identity throughout all live evaluation. Initialize this owner before any fallible admission so every compile-error path uses the same drop guard. Arc clone/last-owner semantics remain unchanged.
- Do not claim to control destruction of a caller-owned arbitrary serde_json::Value after it leaves the SDK. The closure scope is schema-depth-dependent SDK-owned compilation/normalization/CompiledSchema release.

### Design for P20-AUD-010
Existing transcript/replay compiler returns one frozen projection record used by real preauth preparation, payload compilation and message mappers. Caller policy replaces hardcoded NativeOrCollapse. One root compiler record and actual-consumer closure predicate.

- Request gains pub projection: ProjectionOptions; ProjectionOptions { pub temporal: TranscriptProjection, pub replay: ReplayProjection }; both existing enums default NativeOrCollapse. Single request-level option captured at creation/clone across native/simple/direct/advanced calls; no duplicate native/model-visible metadata policy.
- ProjectionFacts { pub source_prefix: SourcePrefix, pub temporal: TemporalProjectionFacts, pub replay: ReplayFacts }; SourcePrefix { pub message_count: usize, pub sha256: String }; TemporalProjectionFacts { pub mode: ProjectionMode, pub affected_ranges: Vec<MessageRange> }; MessageRange { pub start: usize, pub end: usize }; ProjectionMode::{Lossless,Native,Collapse}. Serde camelCase fields/lowercase mode strings.
- PreparedRoute gains pub projection: ProjectionFacts. AssistantMessage gains serde(default,skip_serializing_if=Option::is_none) pub projection: Option<ProjectionFacts>; old serialized records stayNone/unknown.
- resolve_transcript returns TranscriptProjectionOutput { context: Cow<TranscriptContext>, facts: TemporalProjectionFacts }; existing project_replay already returns messages+ReplayFacts. Reuse existing ReplayLoss/ReplayFacts and derive Serde; do not invent a parallel loss taxonomy.
- SourcePrefix covers frozen original complete canonical prefix [0,message_count) with SHA256 of deterministic serde_json original message-array serialization, before dropping any initial system message. Digest is association only, never authority/reconstruction proof. No raw prompt/thinking/signature/credential bytes copied into facts or logs.
- No later effective changes=>Lossless; every requested change class faithfully represented=>Native; any unsupported class under NativeOrCollapse=>current-effective-head Collapse. Affected ranges are sorted merged half-open original message coordinates whose effective view changed. Example [system0,user1,systemUpdate2,user3]: sourcePrefix.messageCount4, collapse affected[0,3), since updated head changes earlier prefix throughupdate2. Per body/section/tool class, not a global native-transcript bool.
- Common preauth structure/native validation calls the same pure per-wire compiler using the frozen selected ModelInfo; actual adapter compiler consumes/reuses it. Native API default validation and every real direct adapter receive identical policies; direct prepared-auth calls validate before callback/network. Auth-derived endpoint/header finalization does not reselect model/source meaning.
- TemporalStrict rejects unsupported temporal change classes before resolver/store/network. ReplayStrict preserves existing strict loss rejection independently. Default calls succeed with facts and unchanged canonical context. Source-capability classes are concrete per-wire; The temporal prerequisite ownership below is part of this batch; the later compat-completion batch must not be needed for this batch green.
- Every newly executed builtin Start/partial/Done/failure partial carries the same known facts; setup failures before projection known may retainNone. Prepared retry and admitted strict-prefer variants reuse original prefix and correct variant facts. Existing grammar/strict diagnostics stay separate. Collector/serde/Agent forwarding preserves projection, and old history is never retrospectively markedlossless.
- Acyclic ownership: this projection batch owns the minimal real per-class native capability inputs and branches needed for its full predicate: Chat supportsMidConvoSystemMessages/supportsMidConvoToolAdditions; shared Responses supportsMidConvoSystemMessages/supportsAdditionalTools/supportsToolSearch, including AdditionalTools-over-ToolSearch precedence and supported-class chronological placement. Add their ModelInfo/codec plumbing here, not in the later dependent compat batch. Preserve native PiMessages and every existing wire default. Mixed AddOnly/body-collapse remains class-specific; remove/redefine unsupported cases collapse or Strict-reject before auth. Actual fixtures cover Native/AddOnly/Collapse and zero-auth Strict negatives before this batch can pass.
- Use the final shared ResponsesSourceCompat owner when introducing these temporal controls. Move already implemented strict/grammar/mid-effort values into it mechanically now, preserving every existing wire/factory value; synchronize their config/test callers here. The later compat batch extends this one owner with remaining source controls and does not introduce a parallel facade or reassign this prerequisite.
- Projection facts do not claim dispatch or durable acknowledgement. Strict rejection returns its typed unsupported-temporal cause; a failure before an accepted projection is formed may retain unknown facts, while failures after a formed projection preserve them.

Constructor migration uses the attached current typed-owner inventory. Return-only/definition/comment/struct-update matches are distinguished; old historical omission fixtures remain unknown. No standalone AssistantMessage JSON schema exists; its actual Serde owner stays schema3 and session-v2 generic readers/writers retain their format. Known safe projection facts must survive the existing explicit Agent redaction mapper.

### Design for P20-AUD-013
Complete presence-aware typed per-wire compat projection raw catalog/factory/custom ModelInfo→store codec→frozen effective model→actual body/header/stream. Raw-only retention or onPayload is not a substitute.

- Extend OpenAiCompletionsCompat with supports_finish_reason:Option<bool>, open_router_routing:Option<OpenRouterRouting>, vercel_gateway_routing:Option<VercelGatewayRouting>, supports_grammar_tools:Option<bool>, supports_mid_convo_system_messages:Option<bool>, supports_mid_convo_tool_additions:Option<bool>, cache_control_format:Option<CacheControlFormat>, session_affinity_format:Option<SessionAffinityFormat>, supports_long_cache_retention:Option<bool>. Keep existing normalized store/instruction_role keys and retired-alias rejection. vllm_priority becomes Option<serde_json::Number>, a finite JSON-number representation; explicit0/fractional values and existing exact i64 host values survive without a lossy i64-to-f64 cast. Presence-aware template maps retain explicit{} as override.
- OpenRouterRouting has13 optional fields: allow_fallbacks, require_parameters, data_collection(Deny|Allow), zdr, enforce_distillable_text, order, only, ignore, quantizations, sort(String|{by:Option<String>,partition:Option<Option<String>>}), max_price with5 optional NumberOrString prices(prompt/completion/image/audio/request), preferred_min_throughput(Number|PercentileCutoffs), preferred_max_latency(Number|PercentileCutoffs). Percentiles optionalp50/p75/p90/p99. Do not hydrate schema-hint defaults into supplied object. Preserve false/0/[]/null. VercelGatewayRouting{only:Option<Vec<String>>,order:Option<Vec<String>>}; empty arrays emit. SessionAffinityFormat::{OpenAi,OpenAiNoSession,OpenRouter}; CacheControlFormat::Anthropic.
- One OpenAiResponsesSourceCompat value contains exactly11 optional source controls(10Option<bool> plus Option<SessionAffinityFormat>) and is held by ordinaryR/Azure/Codex owning compat records. Keep per-wire route/path/store/account fields separate. Resolve default values at concrete wire boundary; ordinaryR Strictfalse, Azure/Codex Stricttrue. Migrate current strict/grammar/mideffort fields to this one source representation; no duplicate facade.
- Chat precedence: frozen exact route/provider/final auth-derived endpoint/model→pinned detectCompat baseline→explicit provider profile→explicit model overrides; false is value; null/None infer; explicit{} and[] retained. Plain source compat decoded first; current complete x-opi.compat store snapshot wins per existing codec contract. Native/simple named values then sampling modelbase→effectivelevel→request perkey, onPayload last under unchanged identity/wire/budget validation.
- Chat actual compiler consumes all27 sourcefields: precise inference table, routing provider/providerOptions, EOF FinishReason owner, strict/grammar tools and replay, class-specific native chronology throughD2, Anthropic cache markers, independent affinity headers and long retention. Source flags are executable controls, not merely persisted metadata.
- Responses exact defaults/branches: developertrue, maxtrue/longtrue ordinaryR; mid-system/grammar/additional/search/explicitcache/mideffortfalse. AdditionalTools wins ToolSearch. additional_tools developer item at System.toolsAdded; else ToolSearch paired deterministic call/output(client/completed,sameID,query names,limit count). Explicitcache+Disabled=>prompt_cache_options{mode:explicit}; Long+longsupport=>ttl30m; explicitcachefalse+Long+longsupport=>legacyretention24h; other retention omitted. Prompt cache key independent.
- Source-qualified ChatGPTsignin bool computed once at prepared-auth adapter setup: provider=openai AND final model baseURL exactlyhttps://api.openai.com/v1 AND defined client key not starting sk-; defined empty key remains distinct from absence. Private secret-freebool suppresses ordinaryR max/temperature/cache-retention/options and configuration_update. Reuse acrossvariants/retries; never classify all Bearer/OAuth as signin or rescan sources.
- OrdinaryR max emits only truthymax/maxcaptrue/notSignin and floor16. Azure truthy native max always floor16 regardless maxcap; Codex emits no max. Shared developer/midsystem/additional/search/grammar/Strict converter retains perwire defaults; Codex initial instructions outsideinput, strict:null, own toolchoice/text/parallel/HTTP/WS unchanged. OrdinaryR explicitcache/mideffort/maxcap behavior is not copied to wires that neverconsume them.
- Codec encodes/decodes all38 fields in plain source compat/current x-opi extension. Older snapshots missing optional fields retainNone and resolve actualwire defaults; unknown extension keys are not asserted effective. Preserve omission/false/zero/null/emptycollections and operation/wire identity. No dualwrites/new durableversion.
- The projection prerequisite already owns the five temporal capability inputs/native branches and mechanical existing Responses projection-field migration. This batch completes the remaining source-qualified fields/codec/default/body/header/EOF semantics and rechecks all38 fields without waiting backward on a later batch.
- Keep coding-agent TOML keys and accepted host integer values unchanged: convert its existing Option<i64> to serde_json::Number::from at the SDK assembly boundary. Native/dynamic fractional JSON priorities remain finite Numbers. Re-map strict/mid-effort reads/writes to the one shared owner; do not delete host config controls or add aliases.
- Template args/kwargs use Option<BTreeMap>: None inherits lower-precedence values, Some(empty) explicitly clears them, Some(map) overrides. Synchronize catalog_lifecycle fixture insertions and all owning codec/adapter/default constructors with this representation; preserve independently owned llama JSON fields.
- Retain the existing typed ChatTemplateKwargValue::Literal(Value) branch, including object literals containing $var/omitWhenOff protected by the x-opi literal wrapper. The Option migration changes presence only; source variable parsing and explicit typed literal decoding remain distinct. Borrow or clone the non-Copy JSON Number at its actual Chat consumption sites.

Use a small existing-codec/custom-deserializer for present-null nested `sort.partition`; default Option<Option<String>> deserialization alone must not erase explicitnull. No new dependency is needed.

### Design for P20-AUD-014
One instance-owned registration inventory coordinates actual route/API owner maps and serves provider list/remove/clear, operation selectors/allavailability and refresh selection/publication. No parallelregistry/facade.

- provider_ids(&self)->Vec<&str>: sorted union actual Provider/typedChat/Image/Classifier/Deferred registration owners
- delete_provider(&mut self,provider:&str)->bool; clear_providers(&mut self)
- get_models(&self,provider:Option<&str>)->Vec<ModelInfo>; current get_model remains chat-only
- get_model_of_type(&self,operation:CatalogOperation,provider:&str,id:&str)->Option<CatalogRecord>
- get_models_of_type(&self,operation:CatalogOperation,provider:Option<&str>)->Vec<CatalogRecord>; get_all_models(&self,provider:Option<&str>)->Vec<CatalogRecord>
- async get_available_of_type(&self,operation:CatalogOperation,provider:Option<&str>)->Result<Vec<CatalogRecord>,ModelsError>; async get_all_available(&self,provider:Option<&str>)->Result<Vec<CatalogRecord>,ModelsError>
- ModelsRefreshOptions{allow_network:bool,providers:Option<BTreeSet<String>>,force:Option<bool>}; Default true/None/None
- async refresh(&mut self,options:ModelsRefreshOptions)->Result<RefreshReport,ModelsError>; async refresh_with(&mut self,options:ModelsRefreshOptions,cancel:&CancellationToken)->Result<RefreshReport,ModelsError>
- CatalogRefreshContext{allow_network:bool,force:Option<bool>,providers:BTreeSet<String>,cancel:CancellationToken}
- Replace Provider::refresh_models() with refresh_models(&self,context:CatalogRefreshContext)->BoxAuthFuture<...,Result<Option<Vec<ModelInfo>>,ProviderError>>; defaultstatic None/no-work, synchronize implementors
- DynamicCatalogSource::provider_ids(&self)->&[String]; fetch(&self,context:CatalogRefreshContext)->BoxFuture<static,Result<HashMap<String,ModelsStoreEntry>,ProviderError>>
- HttpCatalogSource::new(url,provider_ids:impl IntoIterator<Item=String>)->Self; explicit owner declaration without constructor I/O
- Existing register/register_route and immutable registry are real advanced list/read/replace equivalents: do not rename/wrap them merely to mirrorTS. New missing mutators/selectors target the current sourcefinding. Image/classifier-only owners must participate in same live inventory.
- Delete/clear remove owner registry provider/overrides/dynamiccache, route auth descriptor/resolver/provenance/probes, typed Chat/Image/Classifier/Deferred entries byproviderID, builtin factory/check/default flags and OAuth route/flow bindings, active resourceowner pointer and sourceownership membership. Unknownid false/no-op. Do not modify credentials/sharedmodelstore/env/remote/history/callerowned clients.
- Captured old calls hold their Arc revisions and remainusable. Keep retired resourceowners in all_resource_owners for existing explicitcleanup untilsettled; delete/clear do not cancel caller tokens or mutate capturedauth/model/source. Increment local owner/publicationgeneration onreplace/remove/clear; late refresh may publish only current capturedgeneration, never another instance namespace.
- Use existing closed CatalogOperation/CatalogRecord rather than new operation enum/genericplugin. Missing=>None/[]; same bareID acrossops retained; syncselectors never auth/network. Dynamic authoritativeempty suppressesfallback, absence usesregistered records; defaultchat stayschat.
- Availability checks existing per-registration check_configuration/builtin checks, retaining provider+ApiId custom overrides/account operation restrictions; do not resolve generationauth or refreshcredentials/network for builtin checks. Custom checks retain declared I/O. get_all_available is typed union, no fakechatmetadata.
- Refresh selection first: providersNone all; Some(empty) none; unknown/static no-source owners nofetch/auth/network. Restore selected cachedproviderstate beforeauth; allow_networkfalse stops and neverforwardsforce. Networkphase forwards optionalforce exactly to selectedowner’ssource-specific freshness mechanism; no hostTTL/scheduler.
- Dynamic source declaredproviderIDs intersect selectedIDs beforefetch; empty intersection=>zero sourcecall. Pass intersection/force/token context and publishonlyselectedowners. Aggregate HTTP source mayfetch fixed documentonce for selectedintersection, never mutate others or invent forceheaders/query/retry rules. Fake source/native adapters assert exactforce forwarding.
- Read current complete sharedstore snapshot so unselectedpersisted entries/publishers survive. Restore/publish only selectedactiveowners to thisinstance; commit complete merged map with existing guard/ACK and sourcepublisher rules. AfterACK publishexactselectedacknowledgedvalues withtoken/generationchecks; failure/cancel/superseded preserve priorselected/unselectedstate. Do not use global restoredrevision shortcut to skip a selectedprovider notrestored during earlier subset refresh.
- Deleted-owner cachedstore data remains but cannot be exposed/restored without current matching registeredoperation/API owner. Re-register deliberately permits existingvalidated source-owned restore. Preserve tombstones/native-source precedence/staleACK/mixedfailure rules; no new durableversion or parallel refresh path.
- Validate aggregate output against the DynamicCatalogSource.provider_ids declared scope: any undeclared provider is a typed invalid-source failure, preserving poisoned-output rejection. An entry inside the declared scope but outside this refresh selection is not published and cannot alter that provider existing state; publish selected entries only under the selected generation/ACK guard.

Delete/clear touch only bindings owned by the selected live model-provider inventory. Preserve independently registered auth-only flows and shared stores. Existing all_resource_owners lifetime/explicit cleanup/Models-drop ownership is retained; no speculative resource GC or cancellation of captured calls.

### Design for P20-AUD-015
Represent absence of a selected process resource policy explicitly through the existing runner, and condition time/output checks on that selection.

- pub struct CredentialProcessPolicy { pub timeout: Duration, pub max_stdout: usize } and its existing Default(10s,64KiB) remain available as an explicitly chosen bounded policy.
- pub struct SpawnedCredentialProcess { policy: Option<CredentialProcessPolicy> }
- impl Default for SpawnedCredentialProcess returns policy=None
- pub fn with_policy(policy: CredentialProcessPolicy) -> Self stores Some(policy); signature unchanged
- pub fn policy(&self) -> Option<CredentialProcessPolicy> replaces the current unconditionally bounded return type; record this deliberately scoped0.x break in Unreleased and migrate known callers.
- Thread Option<CredentialProcessPolicy> through run_spawned_credential_process, run_spawned_credential_process_inner and output collection. None reads ordinary stdout until EOF, source failure or caller cancellation and waits for actual child status; Some(policy) applies the same absolute timeout across read and wait and the selected stdout limit. Do not replace absence with a larger guessed cap or zero/magic-number sentinel.
- Use one private cancellation-safe collection path with optional limit/deadline. Check actual length arithmetic and real allocation failures; avoid policy.max_stdout+1 overflow. Preserve typed secret-free source errors, read/wait failure cleanup and source continue/selected semantics.
- Retain own_process, CancelProcessOnDrop, ProcessTree and terminate_tree_and_reap ownership semantics. These are independent of whether policy is selected and must execute after caller abandonment or explicit policy failure.
- Leave builtin_aws::factory using SpawnedCredentialProcess::default(), which now intentionally selects no SDK resource policy. Its already existing injected process field remains the opt-in customization path; no new config key/flag/trait.
- Preserve intentional product behavior by migrating opi-coding-agent/src/provider_factory.rs resolve_bedrock_auth from SpawnedCredentialProcess::default() to SpawnedCredentialProcess::with_policy(CredentialProcessPolicy::default()). This makes its existing10s/64KiB host policy explicit rather than removing it incidentally.
- Migrate existing timeout/output-limit unit assertions to explicit with_policy; default process success/error/redaction and cancellation tests retain their true contracts. Document optional getter semantics in Rustdoc and add synchronized brief SDK README/README.zh note; no need to alter unrelated coding-agent tool-output64KiB documentation.

### Design for P20-AUD-018
Use existing public Models::new, provider_collection::register_builtin, BuiltinOverrides.base_urls, auth::AmbientAuthContext and HttpClient::new; no new SDK method, route/resolver abstraction, config key, feature or manual auth assembly.

- Replace each manual AnthropicProvider/register_route/StaticAuthResolver/eager env key assembly with Models::new(); register_builtin(&mut models, Arc::new(AmbientAuthContext), Arc::new(HttpClient::new()), &overrides)?;. Models stays empty until this explicit registration; default stores remain memory and no credential source is consulted during construction/registration/catalog query.
- Use the actual bundled Anthropic model id claude-sonnet-4-5 (existing direct-route typo-like20250514 record is absent from bundled catalog). Preserve existing request/result/tool execution semantics and examples exact source embedding in both READMEs.
- Each example accepts one optional positional base URL, read from std::env::args rather than credentials/env; when present, insert into overrides.base_urls under anthropic before register_builtin. Omitted argument retains the source default endpoint. This is example CLI plumbing over the existing explicit endpoint override, not a product config key. Document optional [base-url] only for a controlled endpoint.
- Remove direct secrecy dependency/import from examples and consumer-readme once unused; retain ordinary Tokio/Serde/stream/cancellation dependencies actually used. Do not change opi-ai Cargo dependency graph.
- Keep exact cargo package artifact extraction, source hashes, bilingual embedding and single-opi-dependency checks. Replace final check-only action with cargo build --offline --locked --bins --message-format=json in extracted-artifact consumer; use returned compiler-artifact executable paths, not assumed target paths.
- Start Python stdlib ThreadingHTTPServer on127.0.0.1 ephemeral port, no external HTTP. For each consumer binary, pass its loopback base URL positional argument. Execute with OS-minimal runtime environment, new empty HOME/USERPROFILE, synthetic ANTHROPIC_API_KEY, no inherited provider keys/cloud vars/proxies. Build environment may retain existing cache/toolchain but runtime cannot inherit actual credentials.
- Controlled Anthropic /v1/messages peer emits canned valid SSE: quick_start -> final text in1request; streaming -> event sequence and same final text in1request; tool_loop -> get_weather(Tokyo) tool_use then require its returned tool_result text "22 C, clear" in second request before final text. Assert exact request counts1/1/2, canonical model id, synthetic auth identity, expected output and successful exit; reject extra/unexpected endpoint/body/requests. Persist raw requests, outputs, exits and counts. No live provider path or registry publication.
- A minimal additional external consumer probe uses a counting AuthContext and the same explicit register_builtin assembly to assert Models::new empty, registration/model queries env/file/store/source-send counters0; then selected first call uses only the controlled Anthropic key. Retain existing two-instance/default-memory isolation proofs; do not substitute this probe for the three same-source binary flows.
- Add test_readme_examples.py that exercises the real runner main with controlled subprocess executor and valid package fixture. Require three run records/counts before PASS, reject missing example/final/tool continuation/failed binary and prove no execution follows refused artifact/embedding/graph validation. Retain offline/environment/path/config/lease controls.

### Design for P20-AUD-019
Add private InstrumentedStream<S: Stream> in telemetry.rs, matching current Instrumented<F> ownership: TelemetryContext plus Option<Pin<Box<S>>>; poll_next executes exactly one inner poll under context.in_scope; Drop takes and destroys inner under same scope, even unpolled. No public telemetry API, new trait, unsafe, global subscriber, exporter, business span or extra poll. Disabled carrier uses existing scoped no-op dispatch. Preserve returned Pending/item/EOF exactly.

- pub(crate) fn TelemetryContext::instrument_stream<S: futures_core::Stream>(&self, stream:S) -> InstrumentedStream<S>
- pub(crate) InstrumentedStream<S> implements Stream<Item=S::Item> and Drop; fields remain private; boxed pin gives safe projection. No public exported type.
- private models helper scope_event_stream(context: Option<TelemetryContext>, stream: EventStream) -> EventStream returns unchanged stream for None and boxed private instrumented stream for Some (including explicit noop)
- Capture request.telemetry_context before moving request into native/custom ChatApi dispatch. At materialize_chat_call ChatApi branch wrap the ResourceApiStream before ChatStream consumption; the plain non-self-scoped adapter probe then runs under the selected source during poll/drop.
- Capture self.request.telemetry_context in PreparedProviderCall::start_attempt and wrap the returned AttemptStream at that boundary, outside its poll/release/Drop owner. This is the existing shared prepared/Provider simple/direct/advanced boundary and preserves acquisition counters/lazy dispatch/cancellation/retry. Avoid duplicate nested scoping where the same stream already passes this owner.
- For Models::stream_deferred, capture call.telemetry_context before move and wrap the final returned event stream at its shared local-fetch boundary. Image/classifier operations return futures rather than EventStream and retain existing Future instrumentation; no unnecessary stream abstraction is added to them.
- Keep already-correct producer Future context capture/handoff; wrapper supplements Stream poll/drop rather than replaces producer ownership. Custom adapter can still use its explicitly selected other context; SDK wrapper establishes only the frozen caller context for unscoped synchronous observation.

### Complete source-qualified compat field map

| Wire | Source field | Rust representation | Exact default/precedence | Actual consumer effect | Current state / controls |
|---|---|---|---|---|---|
| openai-completions | `supportsStore` | Option<ChatStoreField>: SendFalse/Omit | None/null=>!isNonStandard detection; explicit false=>Omit; true=>SendFalse | store:false iff enabled; never store:true | implemented via normalized field; omitted vs true vs false; final endpoint profile/model override |
| openai-completions | `supportsDeveloperRole` | Option<ChatInstructionRole>: DeveloperForReasoning/System | None/null=>OpenRouter anthropic/openai model or standard non-OpenRouter; explicit false=>System | developer only on reasoning models when enabled; otherwise system | implemented normalized field; reasoning true/false × omitted/true/false; retired names still reject |
| openai-completions | `supportsReasoningEffort` | Option<bool> | None/null=>false for Grok/Zai/Moonshot/Together/CFgateway/Nvidia/AntLing, true otherwise; false is not omission | gates effort fields only; format-native toggles remain independent | implemented; every format including off and explicit false |
| openai-completions | `supportsUsageInStreaming` | Option<bool> | default true; null=>default; false retained | stream_options.include_usage omitted when false | implemented; omitted/true/false |
| openai-completions | `supportsFinishReason` | Option<bool> | default true; explicit false retained | true requires finish fact; false infers Stop or ToolUse on EOF with accumulated call state; no raw reason invented | missing; actual HTTP EOF with/without toolcalls; false success vs true typed failure |
| openai-completions | `maxTokensField` | Option<String>, preserving the current exact validated wire names | default max_tokens on chutes/DeepSeek/Moonshot/CFgateway/Together/Nvidia/AntLing/Zai; max_completion_tokens otherwise | selects named max field; preserve existing source native truthy input rule independently | implemented; both exact fields × native/simple; retain retired assertions migration |
| openai-completions | `requiresToolResultName` | bool (source default false) | default false; explicit false and true retained | tool result name emitted only when enabled and source name exists | implemented; both bools × result text/image |
| openai-completions | `requiresAssistantAfterToolResult` | bool | default false | insert exactly existing fixed assistant bridge at toolResult→user and projected tool-image user boundaries; empty assistant content form changes | implemented; bool × both transition branches, canonical unchanged |
| openai-completions | `requiresThinkingAsText` | bool | default false | same-identity replay thinking uses source <thinking> delimiters when true | implemented; false/true × thinking/reasoning content |
| openai-completions | `requiresReasoningContentOnAssistantMessages` | Option<bool> | default isDeepSeek; explicit false defeats detection | when reasoning enabled every replayed assistant has required empty reasoning_content | implemented; DeepSeek inferredtrue vs explicitfalse; ordinary explicittrue |
| openai-completions | `thinkingFormat` | Option<ChatThinkingFormat> | detect DeepSeek→Zai→Together→AntLing→OpenRouter→OpenAI; explicit enum wins | 11 formats: openai/openrouter/deepseek/together/baseten/zai/qwen/chat-template/qwen-chat-template/string-thinking/ant-ling | implemented; all11 mapped effort/off/budget shapes; invalid enum pre-auth error |
| openai-completions | `chatTemplateKwargs` | Option<BTreeMap<String,ChatTemplateKwargValue>>, None absent and Some(empty) an explicit override | default {}; explicit {} is an override; only selected chat-template format consumes | literal scalar/null or $var thinking.enabled/effort/budget with omitWhenOff; no unrelated format emission | implemented map, retain explicit-presence semantics; literal0/false/null; off variable omitWhenOff; empty map vs omission |
| openai-completions | `chatTemplateArgs` | Option<BTreeMap<String,ChatTemplateKwargValue>>, None absent and Some(empty) an explicit override | default {}; explicit {} retained; baseten consumes | same typed variable values emitted as chat_template_args | implemented map; same matrix as kwargs; baseten only |
| openai-completions | `openRouterRouting` | Option<OpenRouterRouting> | default absent; explicit {} is forwarded; no hydration of nested schema default hints | params.provider equals complete supplied nested routing object before final sampling overrides | missing; all13 nested fields, mixed numeric/string prices, false/0/[]/sort.partition null, omitted vs {} |
| openai-completions | `vercelGatewayRouting` | Option<VercelGatewayRouting> | absent=>omit; explicit only/order[] are truthy arrays and retained | providerOptions.gateway contains only/order supplied keys; neither key=>omit | missing; both only/order, one omitted, explicit empty arrays, empty object |
| openai-completions | `zaiToolStream` | bool | default false; explicit true only | tool_stream:true only when tools actually emitted | implemented; no tools / tools × false/true |
| openai-completions | `thinkingTokenBudgetField` | Option<ChatThinkingTokenBudgetField> | explicit field beats supportsThinkingTokenBudget alias; absent=>alias or no field | thinking_token_budget / thinking_budget / thinking_budget_tokens actual top-level budget | implemented; all3, alias, explicit precedence, absent budget and off |
| openai-completions | `supportsThinkingTokenBudget` | bool | default false; explicit field has priority | true aliases thinking_token_budget; does not select another thinking format | implemented; false/true × explicit field present/absent |
| openai-completions | `supportsOpenAIGrammarTools` | Option<bool> | default false, null=>false | true emits grammar custom tools/call/result shape; false projects ordinary function tool and records loss; lark wins nonempty regex | missing; true actual custom wire, false fallback plus facts, missing variants typed fail |
| openai-completions | `supportsMidConvoSystemMessages` | Option<bool> | default false | true keeps supported body/section updates in place; false collapses through D2 policy/facts | missing; true/false × body append/section replace/remove; Strict false-cap preauth error |
| openai-completions | `supportsMidConvoToolAdditions` | Option<bool> | default false and effective only when mid-convo system also true | true anchors added tools in chronological system messages; false uses current effective request tools | missing; both capability bits × toolsAdded; tool removals collapse/fail when not actually native |
| openai-completions | `supportsStrictMode` | existing strict_tool_schema: bool, false baseline | default false; explicit false is retained | true runs shared strict converter for require/prefer, false declines require and emits ordinary tools for other intents; source strict:false field for unconstrained tools only when capability true | metadata exists but require always rejected in Chat; captrue valid require success; capfalse require typed failure; prefer conversion/downgrade; input immutable |
| openai-completions | `cacheControlFormat` | Option<CacheControlFormat::Anthropic> | inferred only provider exact openrouter plus model id anthropic/; null/absent=>inference; explicit anthropic wins | cache_control markers on system, last tool and last eligible user/assistant/tool-result text; Disabled no markers; Long TTL1h only with long support | missing; infer/explicit × Disabled/Short/Long × longfalse; exact marker positions |
| openai-completions | `sendSessionAffinityHeaders` | Option<bool> | default isOpenRouter(provider or URL); explicit false wins | if nonempty sessionId and enabled, selected affinity headers independently of prompt_cache_key | partially implemented, format missing; provider/URL defaults, explicittrue/false, empty/missing ID, request header overlay rules |
| openai-completions | `sessionAffinityFormat` | Option<SessionAffinityFormat::{OpenAi,OpenAiNoSession,OpenRouter}> | default OpenRouter if provider or URL; otherwise OpenAi; explicit enum wins | Chat openai: session_id+x-client-request-id+x-session-affinity; nosession: latter2; openrouter:x-session-id only | missing; all3 exact headers, raw full SDK ID preserved for scope; no conflation with cache key |
| openai-completions | `supportsLongCacheRetention` | Option<bool> | default false for Together/CFworkers/CFgateway/Nvidia/AntLing; true otherwise; explicit false wins | Long=>24h + clamped key only when supported; Anthropic format ttl1h; source default/Short/Disabled branches preserved | missing; inferredfalse overridden true; inferredtrue overridden false; Long/Short/Disabled |
| openai-completions | `vllmPriority` | Option<serde_json::Number>, preserving exact i64 and finite fractional JSON values | no inferred/default emitted value; explicit0 and fractional values preserved | top-level priority number regardless thinking format; no cast-to-i64 discard | integer-only, fractional lost; None/0/negative/fractional; malformed nonnumber typed decode error |
| openai-responses / azure-openai-responses / openai-codex-responses | `supportsDeveloperRole` | Option<bool> | true | shared input instruction role developer only if reasoning and flag !=false; otherwise system | missing; Each actual wire: absent/null/false/true with defaults above; preserve ModelInfo→store codec→actual adapter. Fields not consumed by a source wire stay inert rather than inheriting ordinaryR behavior. |
| openai-responses / azure-openai-responses / openai-codex-responses | `supportsMidConvoSystemMessages` | Option<bool> | false | chronological system/sections honored by shared input converter, D2 policy/facts owns collapse or strict rejection | missing; Each actual wire: absent/null/false/true with defaults above; preserve ModelInfo→store codec→actual adapter. Fields not consumed by a source wire stay inert rather than inheriting ordinaryR behavior. |
| openai-responses / azure-openai-responses / openai-codex-responses | `sessionAffinityFormat` | Option<SessionAffinityFormat> | OpenRouter when provider or URL identifies it, otherwise OpenAi | ordinaryR openai=session_id+x-client-request-id; nosession=x-client-request-id; openrouter=x-session-id; independent of prompt_cache_key | missing; Each actual wire: absent/null/false/true with defaults above; preserve ModelInfo→store codec→actual adapter. Fields not consumed by a source wire stay inert rather than inheriting ordinaryR behavior. |
| openai-responses / azure-openai-responses / openai-codex-responses | `supportsLongCacheRetention` | Option<bool> | true in ordinaryR | ordinaryR Long legacy24h unless explicitcache; explicitcache Long+support ttl30m; false never sends retention | missing; Each actual wire: absent/null/false/true with defaults above; preserve ModelInfo→store codec→actual adapter. Fields not consumed by a source wire stay inert rather than inheriting ordinaryR behavior. |
| openai-responses / azure-openai-responses / openai-codex-responses | `supportsStrictMode` | Option<bool> | ordinaryR false; Azure/Codex true | same shared strict converter with perwire default; Codex unconstrained strict:null, no global true/false | ordinaryR/Azure bool loses correct shared absence default; Codex absent; Each actual wire: absent/null/false/true with defaults above; preserve ModelInfo→store codec→actual adapter. Fields not consumed by a source wire stay inert rather than inheriting ordinaryR behavior. |
| openai-responses / azure-openai-responses / openai-codex-responses | `supportsOpenAIGrammarTools` | Option<bool> | false | shared grammar custom type, IDs, raw wrapping and replay; false fallback records existing diagnostic | R/Azure implemented; Codex source flag not preserved; Each actual wire: absent/null/false/true with defaults above; preserve ModelInfo→store codec→actual adapter. Fields not consumed by a source wire stay inert rather than inheriting ordinaryR behavior. |
| openai-responses / azure-openai-responses / openai-codex-responses | `supportsAdditionalTools` | Option<bool> | false | shared input additional_tools {role:developer,tools} at System toolsAdded; wins when ToolSearch also true | missing; Each actual wire: absent/null/false/true with defaults above; preserve ModelInfo→store codec→actual adapter. Fields not consumed by a source wire stay inert rather than inheriting ordinaryR behavior. |
| openai-responses / azure-openai-responses / openai-codex-responses | `supportsToolSearch` | Option<bool> | false | when additionalToolsfalse, deterministic tool_search_call/output pair with same id, execution client/statuscompleted, query names and limit count; tools conversion toolSearchResulttrue | missing; Each actual wire: absent/null/false/true with defaults above; preserve ModelInfo→store codec→actual adapter. Fields not consumed by a source wire stay inert rather than inheriting ordinaryR behavior. |
| openai-responses / azure-openai-responses / openai-codex-responses | `supportsExplicitPromptCacheMode` | Option<bool> | false | ordinaryR only: Disabled=>prompt_cache_options{mode:explicit}; Long+longsupport=>{ttl:30m}; Short=>omit; legacy retention omitted; ChatGPTsignin omits entire mode | missing; Each actual wire: absent/null/false/true with defaults above; preserve ModelInfo→store codec→actual adapter. Fields not consumed by a source wire stay inert rather than inheriting ordinaryR behavior. |
| openai-responses / azure-openai-responses / openai-codex-responses | `supportsMidConvoEffort` | ResponsesSourceCompat.supports_mid_convo_effort: Option<bool>, resolved per wire after absence preservation | false | ordinaryR only truthy native effort, except exact ChatGPTsignin; config updates keep top-level medium. Azure/Codex do not inherit this flag consumer | ordinaryR implemented but signin gate must share exact frozen credential classification; Each actual wire: absent/null/false/true with defaults above; preserve ModelInfo→store codec→actual adapter. Fields not consumed by a source wire stay inert rather than inheriting ordinaryR behavior. |
| openai-responses / azure-openai-responses / openai-codex-responses | `supportsMaxOutputTokens` | Option<bool> | true ordinaryR | ordinaryR truthymax + capabilitytrue + !ChatGPTsignin=>max_output_tokens max(value,16); Azure truthymax always floor16 independentof flag; Codex omits max entirely | missing; Each actual wire: absent/null/false/true with defaults above; preserve ModelInfo→store codec→actual adapter. Fields not consumed by a source wire stay inert rather than inheriting ordinaryR behavior. |

### Exact migration ownership inventory

The110 current migration/docs/proof paths and typed constructor reasons are sealed below. Definition-only sites own the new field; return-only/struct-update/generic Serde/session readers are retained/verified, not mechanically rewritten. All deliberately changed binaries execute in the workspace union. Full imports/snippets/classification proof is `C:\Users\Luiz\AppData\Local\Temp\opi-phase20-remediate-hvrszrbs\ready-runtime-migration-paths.json`, SHA-256 `ca1f6c1d1d82722d35aaf13c05dfad53224b66e4c18477698a3c4e93a62c519c`.

| Path | Current SHA-256 | Exact owned change |
|---|---|---|
| `CHANGELOG.md` | `a271b2d58b8fb177d6a931fb5ea328aa6fd3f57922f043058119e95b1079516a` | Record the deliberately scoped0.x Request/AssistantMessage/refresh-source API changes and synchronized callers; apply must read complete current Unreleased before writing its task-owned entry. |
| `crates/opi-agent/src/agent_loop.rs` | `55054868e009a82c998f6e748273b4c37ca3f5959c27d86a008316d2f18872e1` | Actual opi_ai::provider::Request struct literal(s) need projection: Default::default(); imports/fully-qualified type ownership verified. Preserve caller current default projection and unrelated request behavior. |
| `crates/opi-agent/src/event.rs` | `5f46c5205433d4e7e5c926b79682600491f07bb6ee4ee319b7143e2bca3a8cf5` | Public redaction reconstructs an opi_ai::AssistantMessage explicitly; carry known projection facts through after verifying they contain only prefix identity/ranges/typed loss facts; keep unrelated existing metadata redaction policy. |
| `crates/opi-agent/tests/agent_loop_semantics.rs` | `8877aeba433fae2dc4d83cf54573fb20860cc1a4090e198a5b2b71fc6fb1bf47` | Actual opi_ai::message::AssistantMessage fixture/helper literal(s) need projection:None for unknown preexisting synthesized message; use Some only in newly added explicit projection propagation tests. |
| `crates/opi-agent/tests/agent_wrapper.rs` | `81e1e0692896c43344c33f8189214552d27fc71def0229224de93b49adf809f1` | Actual opi_ai::message::AssistantMessage fixture/helper literal(s) need projection:None for unknown preexisting synthesized message; use Some only in newly added explicit projection propagation tests. |
| `crates/opi-agent/tests/compaction.rs` | `efb601144466430509f4809da17899f6b93df697ed316a295930e8c48a07acb8` | Actual opi_ai::message::AssistantMessage fixture/helper literal(s) need projection:None for unknown preexisting synthesized message; use Some only in newly added explicit projection propagation tests. |
| `crates/opi-agent/tests/evidence_runtime.rs` | `8cccf3a1c50a6299944246bf3e38118f7330c512b5c0c821f46150dc67f59d5d` | Actual opi_ai::message::AssistantMessage fixture/helper literal(s) need projection:None for unknown preexisting synthesized message; use Some only in newly added explicit projection propagation tests. |
| `crates/opi-agent/tests/hooks_queues.rs` | `8e75fb6b8dfa07ff23ceb58271784d2847c2ae8a7fa5283187a71e1774c1c5ea` | Actual opi_ai::message::AssistantMessage fixture/helper literal(s) need projection:None for unknown preexisting synthesized message; use Some only in newly added explicit projection propagation tests. |
| `crates/opi-agent/tests/preparation_transaction.rs` | `e762a9283078583754d816fcd18f5bbfca6bc325f5e1a830943514c348f11de9` | Actual opi_ai::provider::Request struct literal(s) need projection: Default::default(); imports/fully-qualified type ownership verified. Preserve caller current default projection and unrelated request behavior. |
| `crates/opi-agent/tests/provider_public_safety.rs` | `62764f1f234c9f20d9718a89fae350e1073f79b93eba4d889cc4742a8694e258` | Actual opi_ai::provider::Request struct literal(s) need projection: Default::default(); imports/fully-qualified type ownership verified. Preserve caller current default projection and unrelated request behavior. |
| `crates/opi-agent/tests/sdk_embedding.rs` | `ee553dceac2a4c79ea3cc322c9eb741e45771ad9babb833d54e9f4fa4a47109b` | Add explicit Some(projection) AgentEvent→agent_event_to_value propagation and None/legacy omission compatibility checks; sdk.rs mapper is generic Serde and needs no code change. |
| `crates/opi-agent/tests/session_branching.rs` | `0008be99849abdd7329611a27e37a9126cb6fd1113509ba1fb06cdf07f56e11e` | Actual opi_ai::message::AssistantMessage fixture/helper literal(s) need projection:None for unknown preexisting synthesized message; use Some only in newly added explicit projection propagation tests. |
| `crates/opi-agent/tests/session_context.rs` | `a04a159ba20a43268eb04033dec2411a4252ba81c32ea05c9b646c055adfa1fd` | Actual opi_ai::message::AssistantMessage fixture/helper literal(s) need projection:None for unknown preexisting synthesized message; use Some only in newly added explicit projection propagation tests. |
| `crates/opi-agent/tests/session_contract.rs` | `be73b0bea7dcaa93d7f28aa9a391571e3ff4903944cffcf060c767a2ad69fa06` | Actual opi_ai::message::AssistantMessage fixture/helper literal(s) need projection:None for unknown preexisting synthesized message; use Some only in newly added explicit projection propagation tests.; Existing assistant helper migration plus add same session-v2 writer/read roundtrip of Some(projection); retain old v1/v2 omitted-field fixture bytes unchanged. |
| `crates/opi-agent/tests/thinking_session.rs` | `72740d38766e544c3c2e231f3fd8b91b5de6132cee0d1751a63f872309155b0a` | Actual opi_ai::message::AssistantMessage fixture/helper literal(s) need projection:None for unknown preexisting synthesized message; use Some only in newly added explicit projection propagation tests. |
| `crates/opi-agent/tests/tool_validation.rs` | `b8b6a2beac071dabd0757baea6c64c294ba5d62b9d13d2926e842d571787e25e` | Actual opi_ai::message::AssistantMessage fixture/helper literal(s) need projection:None for unknown preexisting synthesized message; use Some only in newly added explicit projection propagation tests. |
| `crates/opi-ai/README.md` | `6c98dc2f7b5e6b24f3a7805b87f5ca3772e23be60580e049c94d010cae49481f` | Update the three copied exact-source quick_start/quick_start_stream/tool_loop Request constructions with their source examples; document projection options and known/legacy-unknown facts in bilingual SDK manual. |
| `crates/opi-ai/README.zh.md` | `083d282deee15c495e2191dc3f353d9d9b3e89ee77e67a9382b8ea32d3b9b310` | Update the three copied exact-source quick_start/quick_start_stream/tool_loop Request constructions with their source examples; document projection options and known/legacy-unknown facts in bilingual SDK manual. |
| `crates/opi-ai/examples/quick_start.rs` | `78b49a65ad083f665cf94654c867c1d94b026992da3592ff70b0b7edda6b4aa8` | Actual opi_ai::provider::Request struct literal(s) need projection: Default::default(); imports/fully-qualified type ownership verified. Preserve caller current default projection and unrelated request behavior. |
| `crates/opi-ai/examples/quick_start_stream.rs` | `c42d503148ea9d0115df8f33ac5bf652596085eccf0f4e3b34a1d1d6444e5419` | Actual opi_ai::provider::Request struct literal(s) need projection: Default::default(); imports/fully-qualified type ownership verified. Preserve caller current default projection and unrelated request behavior. |
| `crates/opi-ai/examples/tool_loop.rs` | `ba4d8ee21506b56f2bb045855590f6979d996d5d22d19b2e024d4546c5838bef` | Actual opi_ai::provider::Request struct literal(s) need projection: Default::default(); imports/fully-qualified type ownership verified. Preserve caller current default projection and unrelated request behavior. |
| `crates/opi-ai/src/anthropic.rs` | `c2f49a7cf9a44a328fbcef3703bb67ad6ad7ac65c38da3fdb41fb28615a2d2b8` | Actual opi_ai::provider::Request struct literal(s) need projection: Default::default(); imports/fully-qualified type ownership verified. Preserve caller current default projection and unrelated request behavior.; Actual owning SDK AssistantMessage initializer(s) need projection:None before setup facts exist; actual adapters seed known frozen facts before public event/terminal emission. Unit fixtures in same file use None. |
| `crates/opi-ai/src/bedrock/mod.rs` | `ffd75bb4f965f7c6de34d3f16a34380e2561bf97631b553b80f85398eb912f06` | Actual owning SDK AssistantMessage initializer(s) need projection:None before setup facts exist; actual adapters seed known frozen facts before public event/terminal emission. Unit fixtures in same file use None. |
| `crates/opi-ai/src/gemini.rs` | `b590df0c9be0b40a5348f4556f080d408018a145f5b449eb13fea82a829f0cf7` | Actual owning SDK AssistantMessage initializer(s) need projection:None before setup facts exist; actual adapters seed known frozen facts before public event/terminal emission. Unit fixtures in same file use None. |
| `crates/opi-ai/src/message.rs` | `fba53183183921bba450a5083e285bd806397d7271602c546455b152dfc56971` | Declare optional serde-default AssistantMessage.projection field; this Rust Serde type is the owning JSON shape, not a standalone schema file. |
| `crates/opi-ai/src/mistral.rs` | `b5e0f624646bf2e8ecc3dae4cec2449d3ccc3fd341c84f5c4e1d87324d411310` | Actual owning SDK AssistantMessage initializer(s) need projection:None before setup facts exist; actual adapters seed known frozen facts before public event/terminal emission. Unit fixtures in same file use None. |
| `crates/opi-ai/src/openai_chat.rs` | `e3646c6ae826e1051a55d8aae6482f8762a23525e8d2c4c65ac20c4030d9084b` | Actual owning SDK AssistantMessage initializer(s) need projection:None before setup facts exist; actual adapters seed known frozen facts before public event/terminal emission. Unit fixtures in same file use None. |
| `crates/opi-ai/src/openai_responses_shared.rs` | `534d8443c555dc329a83a43461c300f02dc7104485afbffce0db0a08d03491f4` | Actual owning SDK AssistantMessage initializer(s) need projection:None before setup facts exist; actual adapters seed known frozen facts before public event/terminal emission. Unit fixtures in same file use None. |
| `crates/opi-ai/src/pi_messages.rs` | `7c7ad00c5918f5d14c3ca736dc4bbea89ae107db7b8238d6c4e120b3c63eff1b` | Actual owning SDK AssistantMessage initializer(s) need projection:None before setup facts exist; actual adapters seed known frozen facts before public event/terminal emission. Unit fixtures in same file use None. |
| `crates/opi-ai/src/provider.rs` | `9e8614f47073664055cb3a8d80cdd5eb27b1f3066f123abd719757729d6ce832` | Declare request-level ProjectionOptions field and its API contract; this is the owning type definition, not a literal migration. |
| `crates/opi-ai/src/replay.rs` | `18cb94def42060e51cac1450b04bd8dbf08aa80726d6241c45414ac29ffe20b7` | Actual opi_ai::message::AssistantMessage fixture/helper literal(s) need projection:None for unknown preexisting synthesized message; use Some only in newly added explicit projection propagation tests. |
| `crates/opi-ai/src/stream.rs` | `3088d399d4b9c8b38dc8205f46d36192c77895ae989f22bb157a7a960995f0a6` | Actual owning SDK AssistantMessage initializer(s) need projection:None before setup facts exist; actual adapters seed known frozen facts before public event/terminal emission. Unit fixtures in same file use None. |
| `crates/opi-ai/src/test_support.rs` | `6ab6f81b67f55f5e7326ec2f266cc18f13350e2a735d8a91d7b3b4a7efea1e5d` | Actual opi_ai::message::AssistantMessage fixture/helper literal(s) need projection:None for unknown preexisting synthesized message; use Some only in newly added explicit projection propagation tests. |
| `crates/opi-ai/tests/anthropic_fixtures.rs` | `460ef9001c3c6314935b3fddbeefdc56d00f870928ea490f2b199da7861cdc56` | Actual opi_ai::provider::Request struct literal(s) need projection: Default::default(); imports/fully-qualified type ownership verified. Preserve caller current default projection and unrelated request behavior.; Actual opi_ai::message::AssistantMessage fixture/helper literal(s) need projection:None for unknown preexisting synthesized message; use Some only in newly added explicit projection propagation tests. |
| `crates/opi-ai/tests/anthropic_thinking_roundtrip.rs` | `7c2b0902ba89b8c11e6d8bb107299e53210ffbd05919af671e6d16adaf0d4fd4` | Actual opi_ai::provider::Request struct literal(s) need projection: Default::default(); imports/fully-qualified type ownership verified. Preserve caller current default projection and unrelated request behavior. |
| `crates/opi-ai/tests/api_mapped_provider.rs` | `4f7891a167c2159ce429bc81054a44b8c488f67a756e4051d91742b2f6b7a453` | Actual opi_ai::provider::Request struct literal(s) need projection: Default::default(); imports/fully-qualified type ownership verified. Preserve caller current default projection and unrelated request behavior.; Actual opi_ai::message::AssistantMessage fixture/helper literal(s) need projection:None for unknown preexisting synthesized message; use Some only in newly added explicit projection propagation tests. |
| `crates/opi-ai/tests/azure_openai_fixtures.rs` | `86d1d6ec69c9eb363a9ac897ace47408c3585b30bebbb4c30d420753526aef10` | Actual opi_ai::provider::Request struct literal(s) need projection: Default::default(); imports/fully-qualified type ownership verified. Preserve caller current default projection and unrelated request behavior.; Actual opi_ai::message::AssistantMessage fixture/helper literal(s) need projection:None for unknown preexisting synthesized message; use Some only in newly added explicit projection propagation tests. |
| `crates/opi-ai/tests/bedrock_fixtures.rs` | `830f90876fa9cd3b55369271bc28d56ed1572bfd65a0d26a22e0ea8e6d866123` | Actual opi_ai::provider::Request struct literal(s) need projection: Default::default(); imports/fully-qualified type ownership verified. Preserve caller current default projection and unrelated request behavior.; Actual opi_ai::message::AssistantMessage fixture/helper literal(s) need projection:None for unknown preexisting synthesized message; use Some only in newly added explicit projection propagation tests. |
| `crates/opi-ai/tests/builtin_auth_ownership.rs` | `91e62e56c9dca8c309b5ba86191b4bd18581201f88820deb793ce1398380919f` | Actual opi_ai::provider::Request struct literal(s) need projection: Default::default(); imports/fully-qualified type ownership verified. Preserve caller current default projection and unrelated request behavior. |
| `crates/opi-ai/tests/builtin_native_scoped.rs` | `c3f5943003d69b1eba8aa56ef3927757ef39852f643e8042685b757f3f293024` | Actual opi_ai::provider::Request struct literal(s) need projection: Default::default(); imports/fully-qualified type ownership verified. Preserve caller current default projection and unrelated request behavior. |
| `crates/opi-ai/tests/builtin_scoped_env.rs` | `76d76ef2a811ef330d8059a800a8eb5d30125a58e02b60b127532952b7fe2919` | Actual opi_ai::provider::Request struct literal(s) need projection: Default::default(); imports/fully-qualified type ownership verified. Preserve caller current default projection and unrelated request behavior. |
| `crates/opi-ai/tests/catalog_lifecycle.rs` | `ab90853853b0a136a07e35b31e22ac25b8f9b1f53eed6213376d736525753c32` | Actual opi_ai::provider::Request struct literal(s) need projection: Default::default(); imports/fully-qualified type ownership verified. Preserve caller current default projection and unrelated request behavior. |
| `crates/opi-ai/tests/catalog_operations.rs` | `b2ce4044d2b746772a09adbbd2f19a67a74b19e5a6d3148b126ec03972396189` | Actual opi_ai::provider::Request struct literal(s) need projection: Default::default(); imports/fully-qualified type ownership verified. Preserve caller current default projection and unrelated request behavior. |
| `crates/opi-ai/tests/catalog_personalization.rs` | `304bafd372c99966cd19e4f4a154622a17001c8eb4be2dd48cd0367a4ace30f9` | Actual opi_ai::provider::Request struct literal(s) need projection: Default::default(); imports/fully-qualified type ownership verified. Preserve caller current default projection and unrelated request behavior. |
| `crates/opi-ai/tests/chat_capture_replay.rs` | `ded51161b3c76513bd3ade21f65a846554116718b820d7ed17a56ce522492954` | Actual opi_ai::provider::Request struct literal(s) need projection: Default::default(); imports/fully-qualified type ownership verified. Preserve caller current default projection and unrelated request behavior. |
| `crates/opi-ai/tests/chat_pi_messages.rs` | `1f5cf81ae55e9e68628f5960204a42451a0a02841a0451c540995546402b0059` | Actual opi_ai::provider::Request struct literal(s) need projection: Default::default(); imports/fully-qualified type ownership verified. Preserve caller current default projection and unrelated request behavior. |
| `crates/opi-ai/tests/codex_native_options.rs` | `932d44ec89fb4fcf2bf857c281bf743f59d4eb5834114e39ba7a315ab000e6d1` | Actual opi_ai::provider::Request struct literal(s) need projection: Default::default(); imports/fully-qualified type ownership verified. Preserve caller current default projection and unrelated request behavior. |
| `crates/opi-ai/tests/codex_telemetry.rs` | `0e226c7c579f769036d55d282a00c13f3207d2d5417299a8a8fdefed7c1fc3d9` | Actual opi_ai::provider::Request struct literal(s) need projection: Default::default(); imports/fully-qualified type ownership verified. Preserve caller current default projection and unrelated request behavior. |
| `crates/opi-ai/tests/context_budget.rs` | `4137555ec894fcb892baa4f272907cb8d349e48c5601cca67d4423538aaaf921` | Actual opi_ai::message::AssistantMessage fixture/helper literal(s) need projection:None for unknown preexisting synthesized message; use Some only in newly added explicit projection propagation tests. |
| `crates/opi-ai/tests/custom_provider_registration.rs` | `31c981e0379a4b22070ed845a8347406abbcc32fe59771c79bc0b10b4614c52b` | Actual opi_ai::provider::Request struct literal(s) need projection: Default::default(); imports/fully-qualified type ownership verified. Preserve caller current default projection and unrelated request behavior. |
| `crates/opi-ai/tests/deferred_contract.rs` | `b2b6cf3672a61f6eb6d128fadb8cbf5f06bab5119d331a845fe13c3025c58801` | Actual opi_ai::provider::Request struct literal(s) need projection: Default::default(); imports/fully-qualified type ownership verified. Preserve caller current default projection and unrelated request behavior. |
| `crates/opi-ai/tests/external_sdk.rs` | `7dfd92de6b92b4809616a6b756570d26e91355cfca673e93000b82173f646517` | Actual opi_ai::provider::Request struct literal(s) need projection: Default::default(); imports/fully-qualified type ownership verified. Preserve caller current default projection and unrelated request behavior. |
| `crates/opi-ai/tests/gemini_fixtures.rs` | `77ca1f830ace3e30a077ab44833aaef81c378eff553e1ec40df414040cf83f90` | Actual opi_ai::provider::Request struct literal(s) need projection: Default::default(); imports/fully-qualified type ownership verified. Preserve caller current default projection and unrelated request behavior.; Actual opi_ai::message::AssistantMessage fixture/helper literal(s) need projection:None for unknown preexisting synthesized message; use Some only in newly added explicit projection propagation tests. |
| `crates/opi-ai/tests/gemini_lifecycle.rs` | `5d1fb19eaec4759d59a348883cb91b1bce564cad355fb291e6525c820a151956` | Actual opi_ai::provider::Request struct literal(s) need projection: Default::default(); imports/fully-qualified type ownership verified. Preserve caller current default projection and unrelated request behavior. |
| `crates/opi-ai/tests/http_acceptance.rs` | `6c776ec83f5b0ddee7c48d54f1fa78a4f82fbf74278b8367e33dcf86ce81e985` | Actual opi_ai::provider::Request struct literal(s) need projection: Default::default(); imports/fully-qualified type ownership verified. Preserve caller current default projection and unrelated request behavior. |
| `crates/opi-ai/tests/image_provider_serialization.rs` | `52213b740c71e8f6ad7c3c3be2469d16400ab25f2c27af7c2b891d0456eae4e7` | Actual opi_ai::provider::Request struct literal(s) need projection: Default::default(); imports/fully-qualified type ownership verified. Preserve caller current default projection and unrelated request behavior. |
| `crates/opi-ai/tests/mistral_fixtures.rs` | `c837e0c494b7c0a41c2da13b2fbb5275931a764af8076157f606d1e7c5b57fba` | Actual opi_ai::provider::Request struct literal(s) need projection: Default::default(); imports/fully-qualified type ownership verified. Preserve caller current default projection and unrelated request behavior.; Actual opi_ai::message::AssistantMessage fixture/helper literal(s) need projection:None for unknown preexisting synthesized message; use Some only in newly added explicit projection propagation tests. |
| `crates/opi-ai/tests/model_wire_metadata.rs` | `2597152439d0704ef1d63c4928bc8f392a55bfbbfbb98ddc9a6e56ac422c1c10` | Actual opi_ai::provider::Request struct literal(s) need projection: Default::default(); imports/fully-qualified type ownership verified. Preserve caller current default projection and unrelated request behavior. |
| `crates/opi-ai/tests/node4_preparation.rs` | `238afd64e06bbcfa8755ebc2b05b4ca4d6234e7bfbd29de8aae76d83d534e6aa` | Actual opi_ai::provider::Request struct literal(s) need projection: Default::default(); imports/fully-qualified type ownership verified. Preserve caller current default projection and unrelated request behavior.; Actual opi_ai::message::AssistantMessage fixture/helper literal(s) need projection:None for unknown preexisting synthesized message; use Some only in newly added explicit projection propagation tests. |
| `crates/opi-ai/tests/oauth_flows.rs` | `e5530906690b5e9c3cb1a57e8f4a51aca5a4fdec683fa1a6258667976d62aca6` | Actual opi_ai::provider::Request struct literal(s) need projection: Default::default(); imports/fully-qualified type ownership verified. Preserve caller current default projection and unrelated request behavior. |
| `crates/opi-ai/tests/oauth_store_integration.rs` | `6588d8f4a9dd3ae8b4e782287cc12986b3904b2868906f150823b315ae6d4654` | Actual opi_ai::provider::Request struct literal(s) need projection: Default::default(); imports/fully-qualified type ownership verified. Preserve caller current default projection and unrelated request behavior. |
| `crates/opi-ai/tests/oauth_wire_shape.rs` | `74d16c44043e935e1b48baa39d61afa395039e2a66b313eebe7f3bcee7f50e7f` | Actual opi_ai::provider::Request struct literal(s) need projection: Default::default(); imports/fully-qualified type ownership verified. Preserve caller current default projection and unrelated request behavior. |
| `crates/opi-ai/tests/openai_chat_fixtures.rs` | `cc834836a6d2c8eaa56598cf12253a6c3a6a80fe0fcb55e21a9b08db78729da5` | Actual opi_ai::provider::Request struct literal(s) need projection: Default::default(); imports/fully-qualified type ownership verified. Preserve caller current default projection and unrelated request behavior.; Actual opi_ai::message::AssistantMessage fixture/helper literal(s) need projection:None for unknown preexisting synthesized message; use Some only in newly added explicit projection propagation tests. |
| `crates/opi-ai/tests/openai_codex_responses.rs` | `335c1fa2c8c41c78cafd83bcecc45aa6306f60ca6066aed146601c7efb678137` | Actual opi_ai::provider::Request struct literal(s) need projection: Default::default(); imports/fully-qualified type ownership verified. Preserve caller current default projection and unrelated request behavior. |
| `crates/opi-ai/tests/openai_codex_responses/replay.rs` | `3353541ee75d6b6e8ffc65d2e90042227de5b1fba936a6856c95095706e41f31` | Actual opi_ai::message::AssistantMessage fixture/helper literal(s) need projection:None for unknown preexisting synthesized message; use Some only in newly added explicit projection propagation tests. |
| `crates/opi-ai/tests/openai_compat_lifecycle.rs` | `258424e72c8401a6c11487da377e3215a3967ca627bb89d1b4e844ffbdeb89c9` | Actual opi_ai::provider::Request struct literal(s) need projection: Default::default(); imports/fully-qualified type ownership verified. Preserve caller current default projection and unrelated request behavior. |
| `crates/opi-ai/tests/openai_responses_fixtures.rs` | `750b2ea6d442dbb0e6f5c28507927391b339f2d0f1998999cd0976d72dede299` | Actual opi_ai::provider::Request struct literal(s) need projection: Default::default(); imports/fully-qualified type ownership verified. Preserve caller current default projection and unrelated request behavior.; Actual opi_ai::message::AssistantMessage fixture/helper literal(s) need projection:None for unknown preexisting synthesized message; use Some only in newly added explicit projection propagation tests. |
| `crates/opi-ai/tests/openai_responses_lifecycle.rs` | `50881b946181185cd5e55a19e451c761a3ce4656613a6efb6a591072ac058ad5` | Actual opi_ai::provider::Request struct literal(s) need projection: Default::default(); imports/fully-qualified type ownership verified. Preserve caller current default projection and unrelated request behavior. |
| `crates/opi-ai/tests/openrouter_fixtures.rs` | `f9af44e0c150ef8bc9926bd73b73d1ddd35d78d250871243d90b4f3bf7b4a59e` | Actual opi_ai::provider::Request struct literal(s) need projection: Default::default(); imports/fully-qualified type ownership verified. Preserve caller current default projection and unrelated request behavior. |
| `crates/opi-ai/tests/per_request_auth.rs` | `ab9f33bb2210f799d59c8ac2c4020667753b89ba874df320127cc38114df20bc` | Actual opi_ai::provider::Request struct literal(s) need projection: Default::default(); imports/fully-qualified type ownership verified. Preserve caller current default projection and unrelated request behavior. |
| `crates/opi-ai/tests/provider_collection.rs` | `ee5df40bb6eaa9558b1174e182c63a382607f38f789c56c1b617493af975941a` | Actual opi_ai::provider::Request struct literal(s) need projection: Default::default(); imports/fully-qualified type ownership verified. Preserve caller current default projection and unrelated request behavior.; Actual opi_ai::message::AssistantMessage fixture/helper literal(s) need projection:None for unknown preexisting synthesized message; use Some only in newly added explicit projection propagation tests. |
| `crates/opi-ai/tests/provider_diagnostics.rs` | `4b5618d6ff81a537b820684de937a4965dc8c80ade49acda38b50a7a356e1338` | Actual opi_ai::provider::Request struct literal(s) need projection: Default::default(); imports/fully-qualified type ownership verified. Preserve caller current default projection and unrelated request behavior. |
| `crates/opi-ai/tests/provider_error_classes.rs` | `a314a1d49bd4f0d3f6bb3b5b40778c466bc851d229e6f4393f63181f9d7a6731` | Actual opi_ai::provider::Request struct literal(s) need projection: Default::default(); imports/fully-qualified type ownership verified. Preserve caller current default projection and unrelated request behavior. |
| `crates/opi-ai/tests/provider_lifecycle.rs` | `657cc0918c6145cff3578a0c9af200de9576492098b6a8d8b47461cff6f6a3c0` | Actual opi_ai::provider::Request struct literal(s) need projection: Default::default(); imports/fully-qualified type ownership verified. Preserve caller current default projection and unrelated request behavior. |
| `crates/opi-ai/tests/provider_trait.rs` | `a925f0f662114174d90af219338e5bfd42b63239177c39d271ebd24401c73764` | Actual opi_ai::provider::Request struct literal(s) need projection: Default::default(); imports/fully-qualified type ownership verified. Preserve caller current default projection and unrelated request behavior.; Actual opi_ai::message::AssistantMessage fixture/helper literal(s) need projection:None for unknown preexisting synthesized message; use Some only in newly added explicit projection propagation tests. |
| `crates/opi-ai/tests/proxy_support.rs` | `e4ff34916243b00df354dd72b0fba8fc2dfb0caea2fd6af2cdef60a5a78f074e` | Actual opi_ai::provider::Request struct literal(s) need projection: Default::default(); imports/fully-qualified type ownership verified. Preserve caller current default projection and unrelated request behavior. |
| `crates/opi-ai/tests/public_utilities.rs` | `b1bff9af1bfa6b55c759fc33f4bd61cab8237929b6956fda9405f60343569410` | Actual opi_ai::provider::Request struct literal(s) need projection: Default::default(); imports/fully-qualified type ownership verified. Preserve caller current default projection and unrelated request behavior.; Actual opi_ai::message::AssistantMessage fixture/helper literal(s) need projection:None for unknown preexisting synthesized message; use Some only in newly added explicit projection propagation tests. |
| `crates/opi-ai/tests/replay_contract.rs` | `337bd60f23ef2e9c9ed2612c99d861fe9f06188742b87d985aa091200e23581f` | Actual opi_ai::provider::Request struct literal(s) need projection: Default::default(); imports/fully-qualified type ownership verified. Preserve caller current default projection and unrelated request behavior.; Actual opi_ai::message::AssistantMessage fixture/helper literal(s) need projection:None for unknown preexisting synthesized message; use Some only in newly added explicit projection propagation tests. |
| `crates/opi-ai/tests/request_enrichment.rs` | `b2e1446d85bebbfe2efbc58fc72e082bda86a03a0c94687d27d9a3546108bb9b` | Actual opi_ai::provider::Request struct literal(s) need projection: Default::default(); imports/fully-qualified type ownership verified. Preserve caller current default projection and unrelated request behavior. |
| `crates/opi-ai/tests/resource_cleanup.rs` | `970d706659339c4f3b9db41819bfee527d061d619913d6c6fed3559c1d2f9182` | Actual opi_ai::provider::Request struct literal(s) need projection: Default::default(); imports/fully-qualified type ownership verified. Preserve caller current default projection and unrelated request behavior. |
| `crates/opi-ai/tests/retry_backoff.rs` | `1557cd69bcb33aa15cf653652073c2fccaf80970dcb2970c418965c2b142a0a8` | Actual opi_ai::provider::Request struct literal(s) need projection: Default::default(); imports/fully-qualified type ownership verified. Preserve caller current default projection and unrelated request behavior. |
| `crates/opi-ai/tests/service_tier_cost.rs` | `df730cdfaf7d0d0728c57493dbdde96cfaf74c8f55b3da291dc00f7601f46751` | Actual opi_ai::provider::Request struct literal(s) need projection: Default::default(); imports/fully-qualified type ownership verified. Preserve caller current default projection and unrelated request behavior. |
| `crates/opi-ai/tests/stream_events.rs` | `a9953a2461d4e97bb7a2f77771af81276dfbd0383d4199bcf84a324f638d4f9c` | Actual opi_ai::message::AssistantMessage fixture/helper literal(s) need projection:None for unknown preexisting synthesized message; use Some only in newly added explicit projection propagation tests.; Existing AssistantMessage helper migration plus add Some/None projection serde roundtrip and actual stream terminal/partial propagation tests. |
| `crates/opi-ai/tests/telemetry_operations.rs` | `986c311ea0a6679e3cee3477feaeba3dbe5e3a1f7db0449a1c1d18b1b648e9ce` | Actual opi_ai::provider::Request struct literal(s) need projection: Default::default(); imports/fully-qualified type ownership verified. Preserve caller current default projection and unrelated request behavior. |
| `crates/opi-ai/tests/test_support_tests.rs` | `187f3829b58b7891f72c76f7857e83aad62f06bc7fb3feaad3ed981dc121f9a5` | Actual opi_ai::provider::Request struct literal(s) need projection: Default::default(); imports/fully-qualified type ownership verified. Preserve caller current default projection and unrelated request behavior. |
| `crates/opi-ai/tests/tls_contract.rs` | `2ea8277e9a9fc105e8fa4d80597e183e3dfd45d38662027cbc19a3a56d1e05c1` | Actual opi_ai::provider::Request struct literal(s) need projection: Default::default(); imports/fully-qualified type ownership verified. Preserve caller current default projection and unrelated request behavior. |
| `crates/opi-ai/tests/tool_result_no_leak.rs` | `2e61e47dded76960417d8541a4ca763082fdd12974deeab3a2741011704aafb3` | Actual opi_ai::provider::Request struct literal(s) need projection: Default::default(); imports/fully-qualified type ownership verified. Preserve caller current default projection and unrelated request behavior.; Actual opi_ai::message::AssistantMessage fixture/helper literal(s) need projection:None for unknown preexisting synthesized message; use Some only in newly added explicit projection propagation tests. |
| `crates/opi-ai/tests/tool_result_wire.rs` | `b300db8f07235c7330dbf2e5a34a3d65038c0be99e550e9fa46e9493b6f6e0d3` | Actual opi_ai::provider::Request struct literal(s) need projection: Default::default(); imports/fully-qualified type ownership verified. Preserve caller current default projection and unrelated request behavior.; Actual opi_ai::message::AssistantMessage fixture/helper literal(s) need projection:None for unknown preexisting synthesized message; use Some only in newly added explicit projection propagation tests. |
| `crates/opi-ai/tests/transcript_contract.rs` | `cc6e353e87d142ecfffb7b1a7dea01a857caf5c46ec1d5e48343114ed6c2f287` | Actual opi_ai::provider::Request struct literal(s) need projection: Default::default(); imports/fully-qualified type ownership verified. Preserve caller current default projection and unrelated request behavior. |
| `crates/opi-ai/tests/transcript_contract/provider_projection.rs` | `c3b468c970ab4e24fb415caa84543f027a91a5bfb594d2b483c1a747e05af320` | Actual opi_ai::provider::Request struct literal(s) need projection: Default::default(); imports/fully-qualified type ownership verified. Preserve caller current default projection and unrelated request behavior. |
| `crates/opi-ai/tests/vertex_fixtures.rs` | `2cb149a947bb975439d60dc996ed22e127312dfa2d239d20de48579cbe5d1be0` | Actual opi_ai::provider::Request struct literal(s) need projection: Default::default(); imports/fully-qualified type ownership verified. Preserve caller current default projection and unrelated request behavior. |
| `crates/opi-ai/tests/whole_call_retry.rs` | `a6e333e317c5e019324e14663ec457ef6a39135a53a3eeb400efd740b4444a50` | Actual opi_ai::provider::Request struct literal(s) need projection: Default::default(); imports/fully-qualified type ownership verified. Preserve caller current default projection and unrelated request behavior.; Actual opi_ai::message::AssistantMessage fixture/helper literal(s) need projection:None for unknown preexisting synthesized message; use Some only in newly added explicit projection propagation tests. |
| `crates/opi-coding-agent/src/compaction.rs` | `3123381306c077fdd3af7dff28c656056b72b32d7cf40ef97733e2d13a01446f` | Actual opi_ai::provider::Request struct literal(s) need projection: Default::default(); imports/fully-qualified type ownership verified. Preserve caller current default projection and unrelated request behavior. |
| `crates/opi-coding-agent/tests/anthropic_cache_markers.rs` | `9be5f817edcfda07f9c2a5d51aeeadf609fd0967caf0c937d4bdca39257fd839` | Actual opi_ai::provider::Request struct literal(s) need projection: Default::default(); imports/fully-qualified type ownership verified. Preserve caller current default projection and unrelated request behavior.; Actual opi_ai::message::AssistantMessage fixture/helper literal(s) need projection:None for unknown preexisting synthesized message; use Some only in newly added explicit projection propagation tests. |
| `crates/opi-coding-agent/tests/bedrock_provider_wiring.rs` | `ec0d8359324f8109840f94dd856237d627db99f60bab909f5a724563df06a3b1` | Actual opi_ai::provider::Request struct literal(s) need projection: Default::default(); imports/fully-qualified type ownership verified. Preserve caller current default projection and unrelated request behavior. |
| `crates/opi-coding-agent/tests/custom_provider_map.rs` | `5225fb14e1f63b2b4b00e19251d0d78f8fbf044ec882b4e08127d4113d271a86` | Actual opi_ai::provider::Request struct literal(s) need projection: Default::default(); imports/fully-qualified type ownership verified. Preserve caller current default projection and unrelated request behavior. |
| `crates/opi-coding-agent/tests/custom_provider_registration.rs` | `55931d767d696cb73e10dc73f4c2d584c530aa05d6f284d174cf190ff7ecd716` | Actual opi_ai::provider::Request struct literal(s) need projection: Default::default(); imports/fully-qualified type ownership verified. Preserve caller current default projection and unrelated request behavior. |
| `crates/opi-coding-agent/tests/extension_mapped_catalog.rs` | `11aff191981f918416a20826e2f83c35de349c763de381b47251749a0d8bc00c` | Actual opi_ai::provider::Request struct literal(s) need projection: Default::default(); imports/fully-qualified type ownership verified. Preserve caller current default projection and unrelated request behavior. |
| `crates/opi-coding-agent/tests/github_copilot_provider.rs` | `c9ed11cca32080b7e2f86eb74ad93d753cbfb8326fca1bc72ceaa9ae909b9409` | Actual opi_ai::provider::Request struct literal(s) need projection: Default::default(); imports/fully-qualified type ownership verified. Preserve caller current default projection and unrelated request behavior.; Actual opi_ai::message::AssistantMessage fixture/helper literal(s) need projection:None for unknown preexisting synthesized message; use Some only in newly added explicit projection propagation tests. |
| `crates/opi-coding-agent/tests/list_models.rs` | `e3bb8ed5ec7a834e5a5b57828337888ae8d93ef8da21692e4958a4a5b2ac4e78` | Actual opi_ai::provider::Request struct literal(s) need projection: Default::default(); imports/fully-qualified type ownership verified. Preserve caller current default projection and unrelated request behavior. |
| `crates/opi-coding-agent/tests/oauth_auth.rs` | `5b795941dc56b890220a00b83b29085f0b6dcd8326fffbadd00d8275bdfb33d9` | Actual opi_ai::provider::Request struct literal(s) need projection: Default::default(); imports/fully-qualified type ownership verified. Preserve caller current default projection and unrelated request behavior. |
| `crates/opi-coding-agent/tests/product_source_barrier.rs` | `acba4084b9a26de61fe60544b1f8b56eb73fda9c1d0ee0346dcafcbf87612773` | Actual opi_ai::provider::Request struct literal(s) need projection: Default::default(); imports/fully-qualified type ownership verified. Preserve caller current default projection and unrelated request behavior. |
| `crates/opi-coding-agent/tests/provider_factory.rs` | `c4953c0a4a5393660ee4cea656c07b427daa701343ee75079f712e5badbad160` | Actual opi_ai::provider::Request struct literal(s) need projection: Default::default(); imports/fully-qualified type ownership verified. Preserve caller current default projection and unrelated request behavior. |
| `crates/opi-coding-agent/tests/provider_runtime.rs` | `363cf562be8310b98b904c59d5f2eb0c7d69122a1b03cf21a1a2fbff2e3a3dd4` | Actual opi_ai::provider::Request struct literal(s) need projection: Default::default(); imports/fully-qualified type ownership verified. Preserve caller current default projection and unrelated request behavior. |
| `crates/opi-coding-agent/tests/rpc_jsonl.rs` | `1f4afe9411f42c291c55884182c911fc778420bc0303f9b2620fd0e47c45675f` | Actual opi_ai::message::AssistantMessage fixture/helper literal(s) need projection:None for unknown preexisting synthesized message; use Some only in newly added explicit projection propagation tests.; Existing assistant literal migration plus add public JSONL MessageStart/Update/End/TurnEnd propagation with Some(projection), no raw prompt/opaque bytes leaked. |
| `crates/opi-coding-agent/tests/runtime_failure_recovery.rs` | `a3d0ddde556578460d75123e2fdbd8bc091daf6be0ec1047dc86363ffb641cb3` | Actual opi_ai::provider::Request struct literal(s) need projection: Default::default(); imports/fully-qualified type ownership verified. Preserve caller current default projection and unrelated request behavior. |
| `crates/opi-coding-agent/tests/session_export.rs` | `7631c2dd7621996ba56858e852e7ec9d4818a2cf5063b090f54b8c04ecc78051` | Actual opi_ai::message::AssistantMessage fixture/helper literal(s) need projection:None for unknown preexisting synthesized message; use Some only in newly added explicit projection propagation tests.; Migrate actual helper literals; verify optional new facts do not disturb existing content rendering, without rewriting old fixture/history bytes. |
| `scripts/pi-sdk-conformance/consumer-crypto/src/main.rs` | `eab2662f96e57eadcad3e3f677709c1a810dfb4d46a91f32796496908cb42fd4` | Current executable workspace-external package consumer imports opi_ai::provider::Request and constructs it; add default projection field. This current source is not an immutable historical proof or old fixture. |
| `scripts/pi-sdk-conformance/consumer-http/src/main.rs` | `00314b9cc411624989c8f38b78937b5d56d8adc2dee18b6ba58c6337af438a4f` | Current executable workspace-external package consumer imports opi_ai::provider::Request and constructs it; add default projection field. This current source is not an immutable historical proof or old fixture. |
| `scripts/pi-sdk-conformance/consumer-ws/src/main.rs` | `53634bdf1f1c86285c8571f1206ae9711d6bb401472159c03fabfa67cea54439` | Current executable workspace-external package consumer imports opi_ai::provider::Request and constructs it; add default projection field. This current source is not an immutable historical proof or old fixture. |

Counts: `{"request":{"candidate_paths":79,"actual_literal_paths":76,"owning_definition_paths":1,"return_only_paths":2},"assistant_message":{"candidate_paths":49,"actual_literal_paths":44,"owning_definition_paths":1,"return_only_paths":3,"struct_update_only_paths":1},"foreign_type_constructor_paths":0,"task_owned_migration_paths":110}`. Generic Serde/session/RPC verification-only owners and historical-fixture policy: [{"path": "crates/opi-agent/src/sdk.rs", "lines": "288-299", "reason": "agent_event_to_value serializes entire AgentEvent via serde_json::to_value; optional projection is automatically forwarded after event.rs redaction preserves it. Add tests in sdk_embedding.rs; no mapper edit needed."}, {"path": "crates/opi-coding-agent/src/rpc.rs", "lines": "507-513", "reason": "RPC subscription forwards agent_event_to_value(event) intact. New field is additive in existing SDK schema3; no new version or manual JSON field map."}, {"path": "crates/opi-coding-agent/src/runner.rs", "lines": "81-90, JSON event subscription", "reason": "NDJSON serde AgentEvent forwarding/delta suppression is structural; default Other paths pass through, terminal snapshots retain complete message. Verify existing JSON runner tests, no data-shape rewrite."}, {"path": "crates/opi-coding-agent/src/runner/json_output.rs", "reason": "Writer transports pre-serialized JSON strings with ACK/backpressure; never enumerates AssistantMessage fields. No edit."}, {"path": "crates/opi-agent/src/session.rs", "lines": "47,548ff", "reason": "Existing FORMAT_VERSION2 writer serializes SessionEntryEnvelope via Serde; optional message field requires no format bump or reader rewrite. Historical omitted-field messages deserialize None."}, {"path": "crates/opi-protocol/src/execution/v1/schema.rs", "reason": "Command-execution protocol schema does not contain opi_ai AssistantMessage; unrelated and must remain untouched."}, {"path": "crates/opi-protocol/tests/snapshots/execution_v1_schema__schema_v1.snap", "reason": "Unrelated generated execution-protocol schema snapshot; no change."}, {"owner": "crates/opi-ai/src/message.rs optional Serde field and new ProjectionFacts Serde types in transcript/replay; no separate checked-in AssistantMessage JSON Schema exists in current tree", "session": "Retain session-v2 writer and legacy-v1 read-only policy; no new session format/version, no rewritten history or older fixtures", "public_sdk": "SDK schema3 generic AgentEvent JSON serializer carries additive optional projection after redaction mapper copies it; retain old omitted-field payloads valid", "fixtures": "Add fresh positive Some(projection) cases in existing owning Rust test binaries; old None/unknown and omitted-field JSON fixtures remain byte-identical. Provider wire fixture bodies need no field because SDK projection facts are not model-visible wire data.", "external_consumers": "Three current consumer-{crypto,http,ws}/src/main.rs literals require default field; current full consumer/src/main.rs has no Request/AssistantMessage construction. No prior offline-probe/eval artifact touched."}]

## Closure Batches

Each source key owns exactly one batch with one root change and predicate. Dependency-ready source Major repairs precede Minor repairs; explicit prerequisites may run first regardless of source severity. Shared files alone never merge closure keys. All temporary consumer predicates are ported/relinked after scoped0.x field migrations; mechanical default-field additions are allowed, assertion/control weakening is not. A stale static binary cannot be green.

### Batch B1: bedrock.early-prelude-validation

**Closure predicate**: A complete invalid prelude fails typed immediately before any declared-payload wait; valid incomplete frames remain buffered; complete/every-split frames preserve payload and message CRC validation.
**Dependencies**: []
**Verification union**: matching relinked current probe and owning focused suites; shared union below.

#### Fix B1.1: bedrock.early-prelude-validation

- **Finding source(s)**: `phase20-codex-gpt61-703d8bd-20261101t181806z` + `1c60c2f6c88d4d5318822c7412fb4a3fd5d48287253bc661d36ba91e7c2a62ef` + P20-AUD-001
- **Decision**: fix:bedrock.early-prelude-validation
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-ai/src/bedrock/event_stream.rs`, `crates/opi-ai/tests/bedrock_fixtures.rs`, `CHANGELOG.md`
- **Change kind**: behavioral
- **Change**: Validate complete 12-byte prelude CRC, total minimum and checked headers<=total-16 before waiting for declared payload. Preserve valid incomplete and complete-message CRC paths; introduce no payload cap.
- **Closure predicate**: A complete invalid prelude fails typed immediately before any declared-payload wait; valid incomplete frames remain buffered; complete/every-split frames preserve payload and message CRC validation.
- **Red-before**: {"command": "C:\\Users\\Luiz\\AppData\\Local\\Temp\\opi-phase20-remediate-hvrszrbs\\event-prelude-probe.exe", "expected": "FAIL required-contract assertion after controls; compiler/setup failure does not count.", "observed": "FAIL: Exact exported production parser path-module: valid incomplete control buffered; invalid CRC/u32MAX and valid-CRC impossible header relation both Ok([]) at16bytes, assertion exit101. Full-frame CRC fixtures do not guard this incomplete branch."}
- **Green-after**: `python "C:\Users\Luiz\AppData\Local\Temp\opi-phase20-remediate-hvrszrbs\replay_probe.py" --source-root "D:\Luiz\Odradek\opi" --target-dir "E:\opi\cargo-targets\opi-72fa37a0fa044958" --suite event`; Rebuild repaired source and relink identical probe; exit0 plus all consumer/boundary controls in closure predicate.
- **Test impact**: "add/update focused regressions; delete none; retain owning security/lifecycle/source controls."

### Batch B2: ecs.parsed-http-host

**Closure predicate**: Forbidden cleartext DNS host returns typed invalid-endpoint with token-file reads0/source sends0; permitted literal/localhost/HTTPS controls preserve precedence and budgets; selected ECS failure never falls through to IMDS.
**Dependencies**: []
**Verification union**: matching relinked current probe and owning focused suites; shared union below.

#### Fix B2.1: ecs.parsed-http-host

- **Finding source(s)**: `phase20-codex-gpt61-703d8bd-20261101t181806z` + `1c60c2f6c88d4d5318822c7412fb4a3fd5d48287253bc661d36ba91e7c2a62ef` + P20-AUD-002
- **Decision**: fix:ecs.parsed-http-host
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-ai/src/auth/aws/metadata.rs`, `crates/opi-ai/tests/provider_collection.rs`, `CHANGELOG.md`
- **Change kind**: behavioral
- **Change**: Replace string-prefix admission with url::Url::host() matching IPv4 loopback/link-local, IPv6 loopback and exact localhost. Reject other HTTP domains before token read/send; preserve HTTPS, relative URI precedence and validated legacy classification.
- **Closure predicate**: Forbidden cleartext DNS host returns typed invalid-endpoint with token-file reads0/source sends0; permitted literal/localhost/HTTPS controls preserve precedence and budgets; selected ECS failure never falls through to IMDS.
- **Red-before**: {"command": "C:\\Users\\Luiz\\AppData\\Local\\Temp\\opi-phase20-remediate-hvrszrbs\\auth-schema-public-probe.exe ecs C:\\Users\\Luiz\\AppData\\Local\\Temp\\opi-phase20-remediate-hvrszrbs", "expected": "FAIL required-contract assertion after controls; compiler/setup failure does not count.", "observed": "FAIL: Injected public builtin resolver: literal127.0.0.1 sends1; ordinary external domain sends0; both127.attacker.invalid and169.254.attacker.invalid send1 Authorization each, then assertion101. No DNS/socket; fake source transport only."}
- **Green-after**: `python "C:\Users\Luiz\AppData\Local\Temp\opi-phase20-remediate-hvrszrbs\replay_probe.py" --source-root "D:\Luiz\Odradek\opi" --target-dir "E:\opi\cargo-targets\opi-72fa37a0fa044958" --suite auth --case ecs`; Rebuild repaired source and relink identical probe; exit0 plus all consumer/boundary controls in closure predicate.
- **Test impact**: "add/update focused regressions; delete none; retain owning security/lifecycle/source controls."

### Batch B3: sse.multiline-crlf-fragmentation

**Closure predicate**: For Chat, Responses and Gemini, all legal CRLF split/multiline variants produce the same event/text/arguments/terminal sequence as coalesced input; no incomplete prefix is emitted or parsed.
**Dependencies**: []
**Verification union**: matching relinked current probe and owning focused suites; shared union below.

#### Fix B3.1: sse.multiline-crlf-fragmentation

- **Finding source(s)**: `phase20-codex-gpt61-703d8bd-20261101t181806z` + `1c60c2f6c88d4d5318822c7412fb4a3fd5d48287253bc661d36ba91e7c2a62ef` + P20-AUD-003
- **Decision**: fix:sse.multiline-crlf-fragmentation
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-ai/src/openai_chat.rs`, `crates/opi-ai/src/openai_responses_shared.rs`, `crates/opi-ai/src/gemini.rs`, `crates/opi-ai/tests/openai_chat_fixtures.rs`, `crates/opi-ai/tests/openai_responses_fixtures.rs`, `crates/opi-ai/tests/gemini_fixtures.rs`, `CHANGELOG.md`
- **Change kind**: behavioral
- **Change**: Retain incomplete newline framing across chunks and normalize only complete boundaries; use the existing stream framing ownership rather than a new public policy.
- **Closure predicate**: For Chat, Responses and Gemini, all legal CRLF split/multiline variants produce the same event/text/arguments/terminal sequence as coalesced input; no incomplete prefix is emitted or parsed.
- **Red-before**: {"command": "C:\\Users\\Luiz\\AppData\\Local\\Temp\\opi-phase20-remediate-hvrszrbs\\runtime_public_probe.exe --exact multiline_crlf_split_preserves_event --nocapture --test-threads=1", "expected": "exit 101, failed required-behavior assertion after controls succeed", "observed": "FAIL: Current public Chat consumer: coalesced identical legal multiline SSE succeeds; split between CR/LF returns typed malformed OpenAI Chat SSE frame; required-equivalence assertion fails (exit101). Three named production drains normalize trailing CR before the next byte."}
- **Green-after**: `python "C:\Users\Luiz\AppData\Local\Temp\opi-phase20-remediate-hvrszrbs\replay_probe.py" --source-root "D:\Luiz\Odradek\opi" --target-dir "E:\opi\cargo-targets\opi-72fa37a0fa044958" --suite runtime --case multiline_crlf_split_preserves_event`; exit 0; 1 executed, 1 passed, 0 ignored; preserving the same controls Rebuild/relink repaired source, never run a stale static baseline binary.
- **Test impact**: "add/update focused regressions; delete none; retain owning security/lifecycle/source controls."

### Batch B4: deferred.cancel-auth-wait

**Closure predicate**: Precancel invokes auth0/backend0 and settles Cancelled; cancelling while auth Pending stops caller wait and prevents later backend dispatch; normal auth dispatches once, invalid identity still fails before auth.
**Dependencies**: []
**Verification union**: matching relinked current probe and owning focused suites; shared union below.

#### Fix B4.1: deferred.cancel-auth-wait

- **Finding source(s)**: `phase20-codex-gpt61-703d8bd-20261101t181806z` + `1c60c2f6c88d4d5318822c7412fb4a3fd5d48287253bc661d36ba91e7c2a62ef` + P20-AUD-004
- **Decision**: fix:deferred.cancel-auth-wait
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-ai/src/models.rs`, `crates/opi-ai/tests/deferred_contract.rs`, `CHANGELOG.md`
- **Change kind**: behavioral
- **Change**: After identity validation check caller cancellation before invoking resolver; race pending auth wait against the caller token and recheck before backend handoff. Store-owned shared rotation ownership and local-fetch versus remote-cancel distinction remain.
- **Closure predicate**: Precancel invokes auth0/backend0 and settles Cancelled; cancelling while auth Pending stops caller wait and prevents later backend dispatch; normal auth dispatches once, invalid identity still fails before auth.
- **Red-before**: {"command": "C:\\Users\\Luiz\\AppData\\Local\\Temp\\opi-phase20-remediate-hvrszrbs\\auth-schema-public-probe.exe deferred C:\\Users\\Luiz\\AppData\\Local\\Temp\\opi-phase20-remediate-hvrszrbs", "expected": "FAIL required-contract assertion after controls; compiler/setup failure does not count.", "observed": "FAIL: Wrong-route control fails before auth; precancelled matching call starts pending auth1/backend0 and fails to settle within100ms, assertion101. Sibling image/classifier cancellation paths do not guard cancel_deferred."}
- **Green-after**: `python "C:\Users\Luiz\AppData\Local\Temp\opi-phase20-remediate-hvrszrbs\replay_probe.py" --source-root "D:\Luiz\Odradek\opi" --target-dir "E:\opi\cargo-targets\opi-72fa37a0fa044958" --suite auth --case deferred`; Rebuild repaired source and relink identical probe; exit0 plus all consumer/boundary controls in closure predicate.
- **Test impact**: "add/update focused regressions; delete none; retain owning security/lifecycle/source controls."

### Batch B5: diagnostics.overflow-arithmetic

**Closure predicate**: All accepted u64 windows return the mathematical predicate without panic/wrap in debug or release; ordinary99-percent, absent/zero window and known/unknown usage controls retain meaning.
**Dependencies**: []
**Verification union**: matching relinked current probe and owning focused suites; shared union below.

#### Fix B5.1: diagnostics.overflow-arithmetic

- **Finding source(s)**: `phase20-codex-gpt61-703d8bd-20261101t181806z` + `1c60c2f6c88d4d5318822c7412fb4a3fd5d48287253bc661d36ba91e7c2a62ef` + P20-AUD-005
- **Decision**: fix:diagnostics.overflow-arithmetic
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-ai/src/message.rs`, `crates/opi-ai/tests/public_utilities.rs`, `CHANGELOG.md`
- **Change kind**: behavioral
- **Change**: Widen both 99-percent comparison operands to u128 before multiplying; preserve all knownness/Stop/Length/output/error conditions.
- **Closure predicate**: All accepted u64 windows return the mathematical predicate without panic/wrap in debug or release; ordinary99-percent, absent/zero window and known/unknown usage controls retain meaning.
- **Red-before**: {"command": "C:\\Users\\Luiz\\AppData\\Local\\Temp\\opi-phase20-remediate-hvrszrbs\\auth-schema-public-probe.exe overflow C:\\Users\\Luiz\\AppData\\Local\\Temp\\opi-phase20-remediate-hvrszrbs", "expected": "FAIL required-contract assertion after controls; compiler/setup failure does not count.", "observed": "FAIL: Ordinary window100000 returns false; window186330748219288401 panics at production message.rs795 window*99, exit101; no public upper bound exists."}
- **Green-after**: `python "C:\Users\Luiz\AppData\Local\Temp\opi-phase20-remediate-hvrszrbs\replay_probe.py" --source-root "D:\Luiz\Odradek\opi" --target-dir "E:\opi\cargo-targets\opi-72fa37a0fa044958" --suite auth --case overflow`; Rebuild repaired source and relink identical probe; exit0 plus all consumer/boundary controls in closure predicate.
- **Test impact**: "add/update focused regressions; delete none; retain owning security/lifecycle/source controls."

### Batch B6: schema.owned-lifecycle.stack-safety

**Closure predicate**: On an ordinary explicitly 2MiB Rust thread stack, a 10000-level lawful allOf schema, including root $id, can compile, check a scalar, normalize that scalar through validate_tool_arguments, and release the last CompiledSchema Arc normally; typed compile errors release their owned input normally; exact resolver/normalizer semantics remain unchanged and no schema depth cap, leaked owner or enlarged thread stack is used.
**Dependencies**: []
**Verification union**: Rerun the exact current-session three-phase public probe with the same source, depth, 2MiB stack, memory and process timeout; each must exit0 with successful expected phase and actual normal drop (remove only intentionally isolated leak used to identify baseline normalization failure).; cargo test --offline --locked -p opi-ai --test schema_engine; cargo test --offline --locked -p opi-ai --test schema_public; cargo test --offline --locked -p opi-ai --lib schema::; shared union below.

#### Fix B6.1: schema.owned-lifecycle.stack-safety

- **Finding source(s)**: `phase20-codex-gpt61-703d8bd-20261101t181806z` + `1c60c2f6c88d4d5318822c7412fb4a3fd5d48287253bc661d36ba91e7c2a62ef` + P20-AUD-006
- **Decision**: fix:complete-owned-schema-lifecycle-work-stack
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-ai/src/schema/refs.rs`, `crates/opi-ai/src/schema/normalize.rs`, `crates/opi-ai/src/schema/ir.rs`, `crates/opi-ai/src/schema/mod.rs`, `crates/opi-ai/tests/schema_engine.rs`, `crates/opi-ai/tests/schema_public.rs`, `CHANGELOG.md`
- **Change kind**: behavioral
- **Change**: Complete the existing explicit work-stack model across every schema-depth-dependent stage of the SDK-owned compiled-schema lifecycle. This is one bounded schema-owner repair, without a new public seam, schema rejection policy, or dependency.
- **Closure predicate**: On an ordinary explicitly 2MiB Rust thread stack, a 10000-level lawful allOf schema, including root $id, can compile, check a scalar, normalize that scalar through validate_tool_arguments, and release the last CompiledSchema Arc normally; typed compile errors release their owned input normally; exact resolver/normalizer semantics remain unchanged and no schema depth cap, leaked owner or enlarged thread stack is used.
- **Red-before**: {"command": "C:\\Users\\Luiz\\AppData\\Local\\Temp\\opi-phase20-remediate-hvrszrbs\\auth-schema-public-probe.exe deep-id C:\\Users\\Luiz\\AppData\\Local\\Temp\\opi-phase20-remediate-hvrszrbs; C:\\Users\\Luiz\\AppData\\Local\\Temp\\opi-phase20-remediate-hvrszrbs\\auth-schema-public-probe.exe deep-normalize C:\\Users\\Luiz\\AppData\\Local\\Temp\\opi-phase20-remediate-hvrszrbs; C:\\Users\\Luiz\\AppData\\Local\\Temp\\opi-phase20-remediate-hvrszrbs\\auth-schema-public-probe.exe deep-drop C:\\Users\\Luiz\\AppData\\Local\\Temp\\opi-phase20-remediate-hvrszrbs", "expected": "Valid deep phases complete; shallow control succeeds.", "observed": "FAIL: all3 corrected10000-depth probes on2MiB stack abort3221225725. deep-id after START; normalization/drop after COMPILE ok and CHECK true. Shallow32 exits0."}
- **Green-after**: `python "C:\Users\Luiz\AppData\Local\Temp\opi-phase20-remediate-hvrszrbs\replay_probe.py" --source-root "D:\Luiz\Odradek\opi" --target-dir "E:\opi\cargo-targets\opi-72fa37a0fa044958" --suite auth --case deep-id ; python "C:\Users\Luiz\AppData\Local\Temp\opi-phase20-remediate-hvrszrbs\replay_probe.py" --source-root "D:\Luiz\Odradek\opi" --target-dir "E:\opi\cargo-targets\opi-72fa37a0fa044958" --suite auth --case deep-normalize ; python "C:\Users\Luiz\AppData\Local\Temp\opi-phase20-remediate-hvrszrbs\replay_probe.py" --source-root "D:\Luiz\Odradek\opi" --target-dir "E:\opi\cargo-targets\opi-72fa37a0fa044958" --suite auth --case deep-drop`; Each isolated2MiB/10000-depth phase exits0; remove only red-isolation forget for final combined lifecycle test, require normal last-owner and compile-error cleanup.
- **Test impact**: {"update": ["schema_engine::deep_schemas_do_not_exhaust_the_rust_stack: remove mem::forget and assert full normal release"], "add": ["isolated subprocess probes for root-$id admission / normalization / last-Arc drop on 2MiB stacks", "compile failure cleanup with an invalid selected leaf at depth, plus deep dynamic-anchor/FindBase traversal preserving existing first-match semantics"], "retain": ["productive-child and unchanged-instance NoProgress tests", "resolver source-order/dynamic/recursive/duplicate-ID corpus", "normalizer order/raw-preservation/union-arm fixtures"]}

### Batch B7: adc.checked-expiration

**Closure predicate**: Representable positive expiry is retained; unrepresentable positive expiry returns typed error without panic or identity fallback across metadata and all selected native exchanges, preserving retry/secret/rotation ownership.
**Dependencies**: []
**Verification union**: matching relinked current probe and owning focused suites; shared union below.

#### Fix B7.1: adc.checked-expiration

- **Finding source(s)**: `phase20-codex-gpt61-703d8bd-20261101t181806z` + `1c60c2f6c88d4d5318822c7412fb4a3fd5d48287253bc661d36ba91e7c2a62ef` + P20-AUD-007
- **Decision**: fix:adc.checked-expiration
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-ai/src/auth/adc/metadata.rs`, `crates/opi-ai/src/auth/adc/native.rs`, `crates/opi-ai/tests/adc_owner_regressions.rs`, `CHANGELOG.md`
- **Change kind**: behavioral
- **Change**: Use checked datetime addition for positive expires_in in direct metadata and all three native exchange paths; map unrepresentable selected expiry to existing InvalidTokenResponse without fallback. Preserve absent/nonpositive expiry semantics; introduce no cap.
- **Closure predicate**: Representable positive expiry is retained; unrepresentable positive expiry returns typed error without panic or identity fallback across metadata and all selected native exchanges, preserving retry/secret/rotation ownership.
- **Red-before**: {"command": "C:\\Users\\Luiz\\AppData\\Local\\Temp\\opi-phase20-remediate-hvrszrbs\\auth-schema-public-probe.exe adc C:\\Users\\Luiz\\AppData\\Local\\Temp\\opi-phase20-remediate-hvrszrbs", "expected": "FAIL required-contract assertion after controls; compiler/setup failure does not count.", "observed": "FAIL: Fixed-clock fake200 metadata with3600 succeeds/send1; i64MAX expires_in panics at metadata.rs122, exit101. Native.rs979/1020/1078 statically share unchecked addition; native greens must cover each. Checked OAuth ChatGPT path is unaffected."}
- **Green-after**: `python "C:\Users\Luiz\AppData\Local\Temp\opi-phase20-remediate-hvrszrbs\replay_probe.py" --source-root "D:\Luiz\Odradek\opi" --target-dir "E:\opi\cargo-targets\opi-72fa37a0fa044958" --suite auth --case adc`; Rebuild repaired source and relink identical probe; exit0 plus all consumer/boundary controls in closure predicate.
- **Test impact**: "add/update focused regressions; delete none; retain owning security/lifecycle/source controls."

### Batch B8: adc.aws-region-utf8

**Closure predicate**: Decoded region data cannot panic at a UTF-8 boundary. ASCII/multibyte suffix removal, empty typed error and AWS_REGION/AWS_DEFAULT_REGION precedence hold through the actual supplier.
**Dependencies**: []
**Verification union**: matching relinked current probe and owning focused suites; shared union below.

#### Fix B8.1: adc.aws-region-utf8

- **Finding source(s)**: `phase20-codex-gpt61-703d8bd-20261101t181806z` + `1c60c2f6c88d4d5318822c7412fb4a3fd5d48287253bc661d36ba91e7c2a62ef` + P20-AUD-008
- **Decision**: fix:adc.aws-region-utf8
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-ai/src/auth/adc/subject.rs`, `crates/opi-ai/tests/adc_owner_regressions.rs`, `CHANGELOG.md`
- **Change kind**: behavioral
- **Change**: Remove the availability-zone final character through a UTF-8 character-boundary operation instead of len-1 byte slicing. Preserve source precedence and empty typed error; add no unrelated ASCII-only region grammar.
- **Closure predicate**: Decoded region data cannot panic at a UTF-8 boundary. ASCII/multibyte suffix removal, empty typed error and AWS_REGION/AWS_DEFAULT_REGION precedence hold through the actual supplier.
- **Red-before**: {"command": "C:\\Users\\Luiz\\AppData\\Local\\Temp\\opi-phase20-remediate-hvrszrbs\\auth-schema-public-probe.exe aws1 C:\\Users\\Luiz\\AppData\\Local\\Temp\\opi-phase20-remediate-hvrszrbs", "expected": "FAIL required-contract assertion after controls; compiler/setup failure does not count.", "observed": "FAIL: Public fake200 region us-east-1a proceeds to later typed malformed-credentials error (control exit0); us-east-1界 panics at subject.rs450 char boundary, exit101. Ambient environment override is a separate valid branch, no guard for metadata."}
- **Green-after**: `python "C:\Users\Luiz\AppData\Local\Temp\opi-phase20-remediate-hvrszrbs\replay_probe.py" --source-root "D:\Luiz\Odradek\opi" --target-dir "E:\opi\cargo-targets\opi-72fa37a0fa044958" --suite auth --case aws1`; Rebuild repaired source and relink identical probe; exit0 plus all consumer/boundary controls in closure predicate. Preserve source final-character semantics through valid UTF-8 boundaries; an unrepresentable UTF-16 suffix result returns typed source error, never a panic or unrelated ASCII-only grammar.
- **Test impact**: "add/update focused regressions; delete none; retain owning security/lifecycle/source controls."

### Batch B9: duration.first-response-poll

**Closure predicate**: Duration begins immediately before first response-body stream poll, includes Pending body/backpressure waits through terminal, excludes Models auth/loading and handle pre-poll time, and preserves supplied/older deferred values.
**Dependencies**: ["B3"]
**Verification union**: matching relinked current probe and owning focused suites; shared union below.

#### Fix B9.1: duration.first-response-poll

- **Finding source(s)**: `phase20-codex-gpt61-703d8bd-20261101t181806z` + `1c60c2f6c88d4d5318822c7412fb4a3fd5d48287253bc661d36ba91e7c2a62ef` + P20-AUD-009
- **Decision**: fix:duration.first-response-poll
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-ai/src/openai_chat.rs`, `crates/opi-ai/src/openai_responses.rs`, `crates/opi-ai/tests/openai_chat_fixtures.rs`, `crates/opi-ai/tests/openai_responses_lifecycle.rs`, `CHANGELOG.md`
- **Change kind**: behavioral
- **Change**: Move owning response activation timestamp to first poll/activation before the body await, preserving setup failure unknownness.
- **Closure predicate**: Duration begins immediately before first response-body stream poll, includes Pending body/backpressure waits through terminal, excludes Models auth/loading and handle pre-poll time, and preserves supplied/older deferred values.
- **Red-before**: {"command": "C:\\Users\\Luiz\\AppData\\Local\\Temp\\opi-phase20-remediate-hvrszrbs\\runtime_public_probe.exe --exact duration_includes_pending_first_body_poll --nocapture --test-threads=1", "expected": "exit 101, failed required-behavior assertion after controls succeed", "observed": "FAIL: Public direct Chat: elapsed356.96ms with350ms body delay after successful headers; terminal duration_ms=Some(0); assertion >=300ms fails exit101. Chat and Responses both initialize Instant after ready body chunk."}
- **Green-after**: `python "C:\Users\Luiz\AppData\Local\Temp\opi-phase20-remediate-hvrszrbs\replay_probe.py" --source-root "D:\Luiz\Odradek\opi" --target-dir "E:\opi\cargo-targets\opi-72fa37a0fa044958" --suite runtime --case duration_includes_pending_first_body_poll`; exit 0; 1 executed, 1 passed, 0 ignored; preserving the same controls Rebuild/relink repaired source, never run a stale static baseline binary.
- **Test impact**: "add/update focused regressions; delete none; retain owning security/lifecycle/source controls."

### Batch B10: projection.public-fidelity-and-facts

**Closure predicate**: Actual native/simple/direct/advanced calls honor frozen temporal/replay fidelity before auth and preserve exact typed projection mode/ranges/source-prefix/replay facts through prepared route, partial/terminal, failure and serialization without canonical mutation.
**Dependencies**: []
**Verification union**: Relink same probe=>1pass. Add actual native/simple/direct/prepare cases for Lossless/Native/Collapse exact prefix/ranges; body/section/tool change classes; Strict positive and unsupported negative before auth/store/network counters; replay losses; partialfailure/cancel/EOF/serde/collectors samefacts/canonical unchanged.; shared union below.

#### Fix B10.1: projection.public-fidelity-and-facts

- **Finding source(s)**: `phase20-codex-gpt61-703d8bd-20261101t181806z` + `1c60c2f6c88d4d5318822c7412fb4a3fd5d48287253bc661d36ba91e7c2a62ef` + P20-AUD-010
- **Decision**: fix:propagate-frozen-call-projection-policy-and-facts
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-ai/src/provider.rs`, `crates/opi-ai/src/transcript.rs`, `crates/opi-ai/src/replay.rs`, `crates/opi-ai/src/models.rs`, `crates/opi-ai/src/message.rs`, `crates/opi-ai/src/stream.rs`, `crates/opi-ai/src/openai_chat.rs`, `crates/opi-ai/src/openai_responses.rs`, `crates/opi-ai/src/openai_responses_shared.rs`, `crates/opi-ai/src/anthropic.rs`, `crates/opi-ai/src/bedrock/mod.rs`, `crates/opi-ai/src/gemini.rs`, `crates/opi-ai/src/vertex.rs`, `crates/opi-ai/src/azure_openai.rs`, `crates/opi-ai/src/openai_codex_responses.rs`, `crates/opi-ai/src/mistral.rs`, `crates/opi-ai/src/pi_messages.rs`, `crates/opi-ai/README.md`, `crates/opi-ai/README.zh.md`, `CHANGELOG.md`, `crates/opi-agent/src/agent_loop.rs`, `crates/opi-agent/src/event.rs`, `crates/opi-agent/tests/agent_loop_semantics.rs`, `crates/opi-agent/tests/agent_wrapper.rs`, `crates/opi-agent/tests/compaction.rs`, `crates/opi-agent/tests/evidence_runtime.rs`, `crates/opi-agent/tests/hooks_queues.rs`, `crates/opi-agent/tests/preparation_transaction.rs`, `crates/opi-agent/tests/provider_public_safety.rs`, `crates/opi-agent/tests/sdk_embedding.rs`, `crates/opi-agent/tests/session_branching.rs`, `crates/opi-agent/tests/session_context.rs`, `crates/opi-agent/tests/session_contract.rs`, `crates/opi-agent/tests/thinking_session.rs`, `crates/opi-agent/tests/tool_validation.rs`, `crates/opi-ai/examples/quick_start.rs`, `crates/opi-ai/examples/quick_start_stream.rs`, `crates/opi-ai/examples/tool_loop.rs`, `crates/opi-ai/src/test_support.rs`, `crates/opi-ai/tests/anthropic_fixtures.rs`, `crates/opi-ai/tests/anthropic_thinking_roundtrip.rs`, `crates/opi-ai/tests/api_mapped_provider.rs`, `crates/opi-ai/tests/azure_openai_fixtures.rs`, `crates/opi-ai/tests/bedrock_fixtures.rs`, `crates/opi-ai/tests/builtin_auth_ownership.rs`, `crates/opi-ai/tests/builtin_native_scoped.rs`, `crates/opi-ai/tests/builtin_scoped_env.rs`, `crates/opi-ai/tests/catalog_lifecycle.rs`, `crates/opi-ai/tests/catalog_operations.rs`, `crates/opi-ai/tests/catalog_personalization.rs`, `crates/opi-ai/tests/chat_capture_replay.rs`, `crates/opi-ai/tests/chat_pi_messages.rs`, `crates/opi-ai/tests/codex_native_options.rs`, `crates/opi-ai/tests/codex_telemetry.rs`, `crates/opi-ai/tests/context_budget.rs`, `crates/opi-ai/tests/custom_provider_registration.rs`, `crates/opi-ai/tests/deferred_contract.rs`, `crates/opi-ai/tests/external_sdk.rs`, `crates/opi-ai/tests/gemini_fixtures.rs`, `crates/opi-ai/tests/gemini_lifecycle.rs`, `crates/opi-ai/tests/http_acceptance.rs`, `crates/opi-ai/tests/image_provider_serialization.rs`, `crates/opi-ai/tests/mistral_fixtures.rs`, `crates/opi-ai/tests/model_wire_metadata.rs`, `crates/opi-ai/tests/node4_preparation.rs`, `crates/opi-ai/tests/oauth_flows.rs`, `crates/opi-ai/tests/oauth_store_integration.rs`, `crates/opi-ai/tests/oauth_wire_shape.rs`, `crates/opi-ai/tests/openai_chat_fixtures.rs`, `crates/opi-ai/tests/openai_codex_responses.rs`, `crates/opi-ai/tests/openai_codex_responses/replay.rs`, `crates/opi-ai/tests/openai_compat_lifecycle.rs`, `crates/opi-ai/tests/openai_responses_fixtures.rs`, `crates/opi-ai/tests/openai_responses_lifecycle.rs`, `crates/opi-ai/tests/openrouter_fixtures.rs`, `crates/opi-ai/tests/per_request_auth.rs`, `crates/opi-ai/tests/provider_collection.rs`, `crates/opi-ai/tests/provider_diagnostics.rs`, `crates/opi-ai/tests/provider_error_classes.rs`, `crates/opi-ai/tests/provider_lifecycle.rs`, `crates/opi-ai/tests/provider_trait.rs`, `crates/opi-ai/tests/proxy_support.rs`, `crates/opi-ai/tests/public_utilities.rs`, `crates/opi-ai/tests/replay_contract.rs`, `crates/opi-ai/tests/request_enrichment.rs`, `crates/opi-ai/tests/resource_cleanup.rs`, `crates/opi-ai/tests/retry_backoff.rs`, `crates/opi-ai/tests/service_tier_cost.rs`, `crates/opi-ai/tests/stream_events.rs`, `crates/opi-ai/tests/telemetry_operations.rs`, `crates/opi-ai/tests/test_support_tests.rs`, `crates/opi-ai/tests/tls_contract.rs`, `crates/opi-ai/tests/tool_result_no_leak.rs`, `crates/opi-ai/tests/tool_result_wire.rs`, `crates/opi-ai/tests/transcript_contract.rs`, `crates/opi-ai/tests/transcript_contract/provider_projection.rs`, `crates/opi-ai/tests/vertex_fixtures.rs`, `crates/opi-ai/tests/whole_call_retry.rs`, `crates/opi-coding-agent/src/compaction.rs`, `crates/opi-coding-agent/tests/anthropic_cache_markers.rs`, `crates/opi-coding-agent/tests/bedrock_provider_wiring.rs`, `crates/opi-coding-agent/tests/custom_provider_map.rs`, `crates/opi-coding-agent/tests/custom_provider_registration.rs`, `crates/opi-coding-agent/tests/extension_mapped_catalog.rs`, `crates/opi-coding-agent/tests/github_copilot_provider.rs`, `crates/opi-coding-agent/tests/list_models.rs`, `crates/opi-coding-agent/tests/oauth_auth.rs`, `crates/opi-coding-agent/tests/product_source_barrier.rs`, `crates/opi-coding-agent/tests/provider_factory.rs`, `crates/opi-coding-agent/tests/provider_runtime.rs`, `crates/opi-coding-agent/tests/rpc_jsonl.rs`, `crates/opi-coding-agent/tests/runtime_failure_recovery.rs`, `crates/opi-coding-agent/tests/session_export.rs`, `scripts/pi-sdk-conformance/consumer-crypto/src/main.rs`, `scripts/pi-sdk-conformance/consumer-http/src/main.rs`, `scripts/pi-sdk-conformance/consumer-ws/src/main.rs`, `crates/opi-agent/tests/evidence_contract.rs`, `crates/opi-ai/src/model_info.rs`, `crates/opi-ai/src/catalog/wire.rs`, `crates/opi-ai/src/provider_collection.rs`, `crates/opi-coding-agent/src/config.rs`
- **Change kind**: behavioral
- **Change**: Existing transcript/replay compiler returns one frozen projection record used by real preauth preparation, payload compilation and message mappers. Caller policy replaces hardcoded NativeOrCollapse. One root compiler record and actual-consumer closure predicate.
- **Closure predicate**: Actual native/simple/direct/advanced calls honor frozen temporal/replay fidelity before auth and preserve exact typed projection mode/ranges/source-prefix/replay facts through prepared route, partial/terminal, failure and serialization without canonical mutation.
- **Red-before**: {"command": "C:\\Users\\Luiz\\AppData\\Local\\Temp\\opi-phase20-remediate-hvrszrbs\\ready_runtime_probe.exe --exact ready_projection_facts_survive_actual_lossy_adapter --nocapture --test-threads=1", "expected": "Required current public-consumer behavior assertions fail after stated controls; no nonexistent-method or fixture/compiler failures.", "observed": "FAIL: Actual successful public direct Chat call really collapses a later system update; original canonical serde unchanged; result projection=null; required structured-facts runtime assertion fails101. Not a nonexistent-method compile error."}
- **Green-after**: `python "C:\Users\Luiz\AppData\Local\Temp\opi-phase20-remediate-hvrszrbs\replay_probe.py" --source-root "D:\Luiz\Odradek\opi" --target-dir "E:\opi\cargo-targets\opi-72fa37a0fa044958" --suite ready-runtime --case ready_projection_facts_survive_actual_lossy_adapter`; Relink same probe=>1pass. Add actual native/simple/direct/prepare cases for Lossless/Native/Collapse exact prefix/ranges; body/section/tool change classes; Strict positive and unsupported negative before auth/store/network counters; replay losses; partialfailure/cancel/EOF/serde/collectors samefacts/canonical unchanged.
- **Test impact**: "add actual consumer conformance; update scoped0.x constructors/signatures/defaults and bilingual docs; retain old omission fixtures and owning security/lifecycle/source controls; delete none."

### Batch B11: context.independent-total-anchor

**Closure predicate**: Last successful timely independently known positive total supplies the anchor even with unknown parent buckets; only absent/zero total fallback requires fully known positive parents.
**Dependencies**: []
**Verification union**: matching relinked current probe and owning focused suites; shared union below.

#### Fix B11.1: context.independent-total-anchor

- **Finding source(s)**: `phase20-codex-gpt61-703d8bd-20261101t181806z` + `1c60c2f6c88d4d5318822c7412fb4a3fd5d48287253bc661d36ba91e7c2a62ef` + P20-AUD-011
- **Decision**: fix:context.independent-total-anchor
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-ai/src/context.rs`, `crates/opi-ai/tests/context_budget.rs`, `CHANGELOG.md`
- **Change kind**: behavioral
- **Change**: Evaluate known positive usage.total_tokens before the is_reported parent-bucket fallback gate; update the existing assertion that encodes the narrowed rule.
- **Closure predicate**: Last successful timely independently known positive total supplies the anchor even with unknown parent buckets; only absent/zero total fallback requires fully known positive parents.
- **Red-before**: {"command": "C:\\Users\\Luiz\\AppData\\Local\\Temp\\opi-phase20-remediate-hvrszrbs\\runtime_public_probe.exe --exact known_total_without_known_parents_is_anchor --nocapture --test-threads=1", "expected": "exit 101, failed required-behavior assertion after controls succeed", "observed": "FAIL: Known999/reportedtrue control anchors999; unknown-total/reportedfalse control anchors0; known999/reportedfalse yields usage_tokens0/tokens2 instead of999. Required assertion fails exit101."}
- **Green-after**: `python "C:\Users\Luiz\AppData\Local\Temp\opi-phase20-remediate-hvrszrbs\replay_probe.py" --source-root "D:\Luiz\Odradek\opi" --target-dir "E:\opi\cargo-targets\opi-72fa37a0fa044958" --suite runtime --case known_total_without_known_parents_is_anchor`; exit 0; 1 executed, 1 passed, 0 ignored; preserving the same controls Rebuild/relink repaired source, never run a stale static baseline binary.
- **Test impact**: "add/update focused regressions; delete none; retain owning security/lifecycle/source controls."

### Batch B12: api-id.exact-arbitrary-registration

**Closure predicate**: All Chat/Image/Classifier/Deferred typed ApiId registrations preserve exact arbitrary byte strings, including empty and whitespace, with no auth/network at registration; provider-id invariants remain separately enforced.
**Dependencies**: []
**Verification union**: matching relinked current probe and owning focused suites; shared union below.

#### Fix B12.1: api-id.exact-arbitrary-registration

- **Finding source(s)**: `phase20-codex-gpt61-703d8bd-20261101t181806z` + `1c60c2f6c88d4d5318822c7412fb4a3fd5d48287253bc661d36ba91e7c2a62ef` + P20-AUD-012
- **Decision**: fix:api-id.exact-arbitrary-registration
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-ai/src/models.rs`, `crates/opi-ai/tests/custom_provider_registration.rs`, `crates/opi-ai/tests/openrouter_fixtures.rs`, `crates/opi-ai/tests/classifier_contract.rs`, `crates/opi-ai/tests/deferred_contract.rs`, `CHANGELOG.md`
- **Change kind**: behavioral
- **Change**: Remove only invented ApiId trim/nonempty guards and their stale registration documentation/assertions; preserve provider identity validation and existing exact-key/upsert/type mismatch rules.
- **Closure predicate**: All Chat/Image/Classifier/Deferred typed ApiId registrations preserve exact arbitrary byte strings, including empty and whitespace, with no auth/network at registration; provider-id invariants remain separately enforced.
- **Red-before**: {"command": "C:\\Users\\Luiz\\AppData\\Local\\Temp\\opi-phase20-remediate-hvrszrbs\\runtime_public_probe.exe --exact exact_empty_and_whitespace_api_id_register --nocapture --test-threads=1", "expected": "exit 101, failed required-behavior assertion after controls succeed", "observed": "FAIL: Normal exact ApiId registers; empty and one-space both return EmptyApiId. Both forbidden-grammar cases observed; acceptance assertion fails exit101. Static source finds identical trim-empty gate at all four registration seams."}
- **Green-after**: `python "C:\Users\Luiz\AppData\Local\Temp\opi-phase20-remediate-hvrszrbs\replay_probe.py" --source-root "D:\Luiz\Odradek\opi" --target-dir "E:\opi\cargo-targets\opi-72fa37a0fa044958" --suite runtime --case exact_empty_and_whitespace_api_id_register`; exit 0; 1 executed, 1 passed, 0 ignored; preserving the same controls Rebuild/relink repaired source, never run a stale static baseline binary.
- **Test impact**: "add/update focused regressions; delete none; retain owning security/lifecycle/source controls."

### Batch B13: models.provider-management-and-selectors

**Closure predicate**: One instance-owned provider inventory exposes all registered operation owners and implements source-named management/selectors/availability/selected refresh without altering captured calls, shared credentials/stores or unselected publishers.
**Dependencies**: []
**Verification union**: Same exactprobe passes. New publicconformance: mixed3ops/samebareID/image+classifier-only ownership; delete/clear absentnoop and zero store/credential/env/network effects; capturedoldcall settles/newcall missing; otherinstance intact; latepublication/newgeneration; perregistrationavailability; exactselectednative/source counters, Someempty/unknown/static0, forceNone/false/true onlynetwork, restore-beforeauth, cancellation/ACK/no-latepublish,mixedfailure/unselectedretention. New-symbol tests are green-after acceptance, not planningred.; shared union below.

#### Fix B13.1: models.provider-management-and-selectors

- **Finding source(s)**: `phase20-codex-gpt61-703d8bd-20261101t181806z` + `1c60c2f6c88d4d5318822c7412fb4a3fd5d48287253bc661d36ba91e7c2a62ef` + P20-AUD-014
- **Decision**: fix:complete-instance-owned-provider-and-operation-management
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-ai/src/models.rs`, `crates/opi-ai/src/registry.rs`, `crates/opi-ai/src/provider.rs`, `crates/opi-ai/src/catalog/http_source.rs`, `crates/opi-ai/src/catalog/store.rs`, `crates/opi-ai/tests/provider_collection.rs`, `crates/opi-ai/tests/catalog_operations.rs`, `crates/opi-ai/tests/custom_provider_registration.rs`, `crates/opi-ai/tests/builtin_auth_ownership.rs`, `crates/opi-ai/README.md`, `crates/opi-ai/README.zh.md`, `CHANGELOG.md`, `crates/opi-ai/src/api_mapped.rs`, `crates/opi-ai/src/catalog/mod.rs`, `crates/opi-ai/src/lib.rs`, `crates/opi-ai/src/provider_collection.rs`, `crates/opi-ai/tests/api_mapped_provider.rs`, `crates/opi-ai/tests/catalog_lifecycle.rs`, `crates/opi-ai/tests/node4_preparation.rs`, `crates/opi-ai/tests/provider_trait.rs`, `crates/opi-ai/tests/support/catalog_store.rs`
- **Change kind**: behavioral
- **Change**: One instance-owned registration inventory coordinates actual route/API owner maps and serves provider list/remove/clear, operation selectors/allavailability and refresh selection/publication. No parallelregistry/facade.
- **Closure predicate**: One instance-owned provider inventory exposes all registered operation owners and implements source-named management/selectors/availability/selected refresh without altering captured calls, shared credentials/stores or unselected publishers.
- **Red-before**: {"command": "C:\\Users\\Luiz\\AppData\\Local\\Temp\\opi-phase20-remediate-hvrszrbs\\ready_runtime_probe.exe --exact ready_image_only_owner_is_visible_to_provider_management --nocapture --test-threads=1", "expected": "Required current public-consumer behavior assertions fail after stated controls; no nonexistent-method or fixture/compiler failures.", "observed": "FAIL: Real OpenRouterImagesApi registration, synthetic resolver,no source work; image_model(image-only,image) succeeds but existing provider_ids still[probe] after validchat control, omitting image-only. Existing actual public management runtime assertion fails101; absent methods are not compiled."}
- **Green-after**: `python "C:\Users\Luiz\AppData\Local\Temp\opi-phase20-remediate-hvrszrbs\replay_probe.py" --source-root "D:\Luiz\Odradek\opi" --target-dir "E:\opi\cargo-targets\opi-72fa37a0fa044958" --suite ready-runtime --case ready_image_only_owner_is_visible_to_provider_management`; Same exactprobe passes. New publicconformance: mixed3ops/samebareID/image+classifier-only ownership; delete/clear absentnoop and zero store/credential/env/network effects; capturedoldcall settles/newcall missing; otherinstance intact; latepublication/newgeneration; perregistrationavailability; exactselectednative/source counters, Someempty/unknown/static0, forceNone/false/true onlynetwork, restore-beforeauth, cancellation/ACK/no-latepublish,mixedfailure/unselectedretention. New-symbol tests are green-after acceptance, not planningred.
- **Test impact**: "add actual consumer conformance; update scoped0.x constructors/signatures/defaults and bilingual docs; retain old omission fixtures and owning security/lifecycle/source controls; delete none."

### Batch B14: aws.credential-process.opt-in-limits

**Closure predicate**: The SDK builtin AWS source factory selects a default runner with policy None; a valid source process taking >10s or yielding >64KiB is accepted without implicit timeout/output rejection; explicitly bounded callers still receive their existing typed Timeout/OutputLimit and cancellation/drop always terminates/reaps the owned child tree.
**Dependencies**: []
**Verification union**: Rerun exact current-session lawful large-output and slow-process probes against the default runner: accepted source credentials; rerun same commands with selected small policy: typed OutputLimit/Timeout.; cargo test --offline --locked -p opi-ai --lib auth::aws::process::tests::; cargo test --offline --locked -p opi-ai --test provider_collection; cargo test --offline --locked -p opi-ai --test auth_process_lifecycle; cargo test --offline --locked -p opi-coding-agent provider_factory; cargo fmt --check --all; cargo clippy --workspace --all-targets -- -D warnings; cargo test --workspace --all-targets; cargo test --workspace --doc; RUSTDOCFLAGS=-D warnings cargo doc --workspace --no-deps; shared union below.

#### Fix B14.1: aws.credential-process.opt-in-limits

- **Finding source(s)**: `phase20-codex-gpt61-703d8bd-20261101t181806z` + `1c60c2f6c88d4d5318822c7412fb4a3fd5d48287253bc661d36ba91e7c2a62ef` + P20-AUD-015
- **Decision**: fix:make-credential-process-limits-explicit
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-ai/src/auth/aws/process.rs`, `crates/opi-ai/src/auth/builtin_aws.rs`, `crates/opi-coding-agent/src/provider_factory.rs`, `crates/opi-ai/tests/provider_collection.rs`, `crates/opi-ai/tests/auth_process_lifecycle.rs`, `crates/opi-ai/README.md`, `crates/opi-ai/README.zh.md`, `CHANGELOG.md`
- **Change kind**: behavioral
- **Change**: Represent absence of a selected process resource policy explicitly through the existing runner, and condition time/output checks on that selection.
- **Closure predicate**: The SDK builtin AWS source factory selects a default runner with policy None; a valid source process taking >10s or yielding >64KiB is accepted without implicit timeout/output rejection; explicitly bounded callers still receive their existing typed Timeout/OutputLimit and cancellation/drop always terminates/reaps the owned child tree.
- **Red-before**: {"command": "C:\\Users\\Luiz\\AppData\\Local\\Temp\\opi-phase20-remediate-hvrszrbs\\runtime_public_probe.exe --exact uncapped_default_accepts_valid_large_credential_process --nocapture --test-threads=1", "expected": "exit 101, failed required-behavior assertion after controls succeed", "observed": "FAIL: Small valid synthetic JSON passes default; valid70000-byte Padding credential JSON passes caller128KiB/20s control; same larger valid input under default returns Some(OutputLimit), assertion fails exit101. Source default also enforces10s."}
- **Green-after**: `python "C:\Users\Luiz\AppData\Local\Temp\opi-phase20-remediate-hvrszrbs\replay_probe.py" --source-root "D:\Luiz\Odradek\opi" --target-dir "E:\opi\cargo-targets\opi-72fa37a0fa044958" --suite runtime --case uncapped_default_accepts_valid_large_credential_process`; Rebuilt/relinked actual runner accepts lawful large output; add >10s success, explicit bounded controls, factory selection and owned-tree cancellation/reap tests.
- **Test impact**: {"add": ["public/default runner accepts lawful valid stdout beyond64KiB and source runtime beyond10s", "builtin factory uses absent policy and returned source credentials, not later-source fallback", "policy getter None/default and Some(explicit) distinguishes selection", "matching explicit timeout/output controls plus cancellation without a policy"], "update": ["spawned_runner_failures_are_typed_and_bounded selects explicit policy for timeout/output cases", "product assembly preserves prior explicit bounds"], "retain": ["frozen cwd/env, redaction, typed malformed output", "child/descendant cleanup on cancellation, inherited stdout and parent exit", "source precedence/failure/rotation fixtures"]}

### Batch B15: http.source-qualified-redirect-budget

**Closure predicate**: Pinned source-qualified generation routes admit20 redirects/21 sends and reject21 redirects without extra sends, while preserving source-owned method/body/header/audience behavior and explicit caller override.
**Dependencies**: []
**Verification union**: matching relinked current probe and owning focused suites; shared union below.

#### Fix B15.1: http.source-qualified-redirect-budget

- **Finding source(s)**: `phase20-codex-gpt61-703d8bd-20261101t181806z` + `1c60c2f6c88d4d5318822c7412fb4a3fd5d48287253bc661d36ba91e7c2a62ef` + P20-AUD-016
- **Decision**: fix:http.source-qualified-redirect-budget
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-ai/src/http.rs`, `crates/opi-ai/tests/http_acceptance.rs`, `CHANGELOG.md`
- **Change kind**: behavioral
- **Change**: Set shared generation RedirectPolicy::default() to Limited(20), update exact rustdoc/default test. Current AWS explicitly uses None, ADC explicitly None/20, OAuth no-redirect client and websocket None: preserve these explicit source policies and caller overrides; no adapter duplicate is needed.
- **Closure predicate**: Pinned source-qualified generation routes admit20 redirects/21 sends and reject21 redirects without extra sends, while preserving source-owned method/body/header/audience behavior and explicit caller override.
- **Red-before**: {"command": "C:\\Users\\Luiz\\AppData\\Local\\Temp\\opi-phase20-remediate-hvrszrbs\\runtime_public_probe.exe --exact eleven_redirects_are_accepted_by_default_generation --nocapture --test-threads=1", "expected": "exit 101, failed required-behavior assertion after controls succeed", "observed": "FAIL: Public default OpenAiChatProvider makes11 sends, returns typed network too-many-redirects for lawful11 redirects requiring12 sends, and required-success assertion fails exit101."}
- **Green-after**: `python "C:\Users\Luiz\AppData\Local\Temp\opi-phase20-remediate-hvrszrbs\replay_probe.py" --source-root "D:\Luiz\Odradek\opi" --target-dir "E:\opi\cargo-targets\opi-72fa37a0fa044958" --suite runtime --case eleven_redirects_are_accepted_by_default_generation`; exit 0; 1 executed, 1 passed, 0 ignored; preserving the same controls Rebuild/relink repaired source, never run a stale static baseline binary.
- **Test impact**: "add/update focused regressions; delete none; retain owning security/lifecycle/source controls."

### Batch B16: strict.required-shape-admission

**Closure predicate**: Every present required must be a string array naming existing properties before strict conversion; absent remains valid; require fails typed before auth/dispatch, prefer follows declared fallback, trusted schema remains unchanged.
**Dependencies**: []
**Verification union**: matching relinked current probe and owning focused suites; shared union below.

#### Fix B16.1: strict.required-shape-admission

- **Finding source(s)**: `phase20-codex-gpt61-703d8bd-20261101t181806z` + `1c60c2f6c88d4d5318822c7412fb4a3fd5d48287253bc661d36ba91e7c2a62ef` + P20-AUD-017
- **Decision**: fix:strict.required-shape-admission
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-ai/src/constrained_sampling.rs`, `crates/opi-ai/tests/anthropic_fixtures.rs`, `crates/opi-ai/tests/openai_responses_fixtures.rs`, `CHANGELOG.md`
- **Change kind**: behavioral
- **Change**: Distinguish absent required from present wrong-type required before extracting required names; preserve existing entry/unknown-name checks and local-schema inert guards.
- **Closure predicate**: Every present required must be a string array naming existing properties before strict conversion; absent remains valid; require fails typed before auth/dispatch, prefer follows declared fallback, trusted schema remains unchanged.
- **Red-before**: {"command": "C:\\Users\\Luiz\\AppData\\Local\\Temp\\opi-phase20-remediate-hvrszrbs\\runtime_public_probe.exe --exact wrong_type_required_is_strict_error --nocapture --test-threads=1", "expected": "exit 101, failed required-behavior assertion after controls succeed", "observed": "FAIL: Valid required[string] succeeds, invalid required[1] errors; wrong-type string/number/object each return Ok rewritten schema. All three observed; rejection assertion fails exit101."}
- **Green-after**: `python "C:\Users\Luiz\AppData\Local\Temp\opi-phase20-remediate-hvrszrbs\replay_probe.py" --source-root "D:\Luiz\Odradek\opi" --target-dir "E:\opi\cargo-targets\opi-72fa37a0fa044958" --suite runtime --case wrong_type_required_is_strict_error`; exit 0; 1 executed, 1 passed, 0 ignored; preserving the same controls Rebuild/relink repaired source, never run a stale static baseline binary.
- **Test impact**: "add/update focused regressions; delete none; retain owning security/lifecycle/source controls."

### Batch B17: compat.source-fields-to-real-wire

**Closure predicate**: All38 pinned source-qualified compat controls survive catalog/store/profile/model projection into actual per-wire body/header/stream behavior, with exact absence/false/zero/null precedence and Azure/Codex exceptions.
**Dependencies**: ["B10", "B16"]
**Verification union**: Rebuild/relink unchanged15 filters plus previous response_compat_maximum_false_reaches_wire. For all38 inventory rows use actual native/simple/builtin/custom/dynamic/store consumer matrix at owning seam; every missing control has positive/false/default/wrongwire/invalid controls, exact executedcounts; retain current cancellation/callback/source-counts/security checks; Azure/Codex actual HTTP/WS exceptions regression.; shared union below.

#### Fix B17.1: compat.source-fields-to-real-wire

- **Finding source(s)**: `phase20-codex-gpt61-703d8bd-20261101t181806z` + `1c60c2f6c88d4d5318822c7412fb4a3fd5d48287253bc661d36ba91e7c2a62ef` + P20-AUD-013
- **Decision**: fix:preserve-full-source-qualified-compat-through-wire-compilation
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-ai/src/model_info.rs`, `crates/opi-ai/src/catalog/wire.rs`, `crates/opi-ai/src/provider_collection.rs`, `crates/opi-ai/src/openai_chat.rs`, `crates/opi-ai/src/openai_responses.rs`, `crates/opi-ai/src/openai_responses_shared.rs`, `crates/opi-ai/src/azure_openai.rs`, `crates/opi-ai/src/openai_codex_responses.rs`, `crates/opi-ai/tests/model_wire_metadata.rs`, `crates/opi-ai/tests/catalog_operations.rs`, `crates/opi-ai/tests/openai_chat_fixtures.rs`, `crates/opi-ai/tests/openai_responses_fixtures.rs`, `crates/opi-ai/tests/azure_openai_fixtures.rs`, `crates/opi-ai/tests/openai_codex_responses.rs`, `crates/opi-ai/README.md`, `crates/opi-ai/README.zh.md`, `CHANGELOG.md`, `crates/opi-coding-agent/src/config.rs`, `crates/opi-ai/tests/chat_capture_replay.rs`, `crates/opi-coding-agent/tests/custom_provider_map.rs`, `crates/opi-ai/tests/catalog_lifecycle.rs`
- **Change kind**: behavioral
- **Change**: Complete presence-aware typed per-wire compat projection raw catalog/factory/custom ModelInfo→store codec→frozen effective model→actual body/header/stream. Raw-only retention or onPayload is not a substitute.
- **Closure predicate**: All38 pinned source-qualified compat controls survive catalog/store/profile/model projection into actual per-wire body/header/stream behavior, with exact absence/false/zero/null precedence and Azure/Codex exceptions.
- **Red-before**: {"command": "C:\\Users\\Luiz\\AppData\\Local\\Temp\\opi-phase20-remediate-hvrszrbs\\ready_runtime_probe.exe ready_chat_ --nocapture --test-threads=1 ; C:\\Users\\Luiz\\AppData\\Local\\Temp\\opi-phase20-remediate-hvrszrbs\\ready_runtime_probe.exe ready_responses_ --nocapture --test-threads=1", "expected": "Required current public-consumer behavior assertions fail after stated controls; no nonexistent-method or fixture/compiler failures.", "observed": "FAIL: New aggregate actually executes10Chat+5Responses behavioral failures from real decoded CatalogModel/public bodybuilder/loopback HTTP. Missing nestedrouting/providerOptions, fractionalprioritynull, cachemarkersfalse,longretentionnull, grammarfunction-notcustom, validstrictrequireunsupported,native chronology3vs4, declared finishlessEOFerror, affinityformatheaderabsent; Responses additions/searchabsent, developerfalseinputstartsuser, explicitmodeabsent,longfalse still24h. Previous public probe confirms maxcapfalse emits32 and explicitcacheLong legacy24h. No copied replacement implementation or nonexistent APIs."}
- **Green-after**: `python "C:\Users\Luiz\AppData\Local\Temp\opi-phase20-remediate-hvrszrbs\replay_probe.py" --source-root "D:\Luiz\Odradek\opi" --target-dir "E:\opi\cargo-targets\opi-72fa37a0fa044958" --suite ready-runtime --filter ready_chat_ ; python "C:\Users\Luiz\AppData\Local\Temp\opi-phase20-remediate-hvrszrbs\replay_probe.py" --source-root "D:\Luiz\Odradek\opi" --target-dir "E:\opi\cargo-targets\opi-72fa37a0fa044958" --suite ready-runtime --filter ready_responses_ ; python "C:\Users\Luiz\AppData\Local\Temp\opi-phase20-remediate-hvrszrbs\replay_probe.py" --source-root "D:\Luiz\Odradek\opi" --target-dir "E:\opi\cargo-targets\opi-72fa37a0fa044958" --suite runtime --case response_compat_maximum_false_reaches_wire`; Rebuild/relink unchanged15 filters plus previous response_compat_maximum_false_reaches_wire. For all38 inventory rows use actual native/simple/builtin/custom/dynamic/store consumer matrix at owning seam; every missing control has positive/false/default/wrongwire/invalid controls, exact executedcounts; retain current cancellation/callback/source-counts/security checks; Azure/Codex actual HTTP/WS exceptions regression.
- **Test impact**: "add actual consumer conformance; update scoped0.x constructors/signatures/defaults and bilingual docs; retain old omission fixtures and owning security/lifecycle/source controls; delete none."

### Batch B18: onboarding.same-source-package-execution

**Closure predicate**: Actual extracted-package consumer executes all three byte-identical README/example sources using ordinary explicit builtins and synthetic loopback auth;1/1/2 real request counts plus final outcome/tool-result continuation, default-memory/lazy-construction facts and exact artifact/consumer/source identities are recorded. PASS impossible when executions absent.
**Dependencies**: ["B10"]
**Verification union**: python scripts/pi-sdk-conformance/test_readme_examples.py; python scripts/pi-sdk-conformance/run_readme_examples.py --offline --scratch <fresh-external-scratch> --target-dir E:\opi\cargo-targets\opi-72fa37a0fa044958; cargo test --locked --offline -p opi-ai --test external_sdk; python scripts/opi-doc-check.py; git diff --check; shared union below.

#### Fix B18.1: onboarding.same-source-package-execution

- **Finding source(s)**: `phase20-codex-gpt61-703d8bd-20261101t181806z` + `1c60c2f6c88d4d5318822c7412fb4a3fd5d48287253bc661d36ba91e7c2a62ef` + P20-AUD-018
- **Decision**: fix:execute-same-source-packaged-onboarding-with-builtins
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-ai/examples/quick_start.rs`, `crates/opi-ai/examples/quick_start_stream.rs`, `crates/opi-ai/examples/tool_loop.rs`, `crates/opi-ai/README.md`, `crates/opi-ai/README.zh.md`, `scripts/pi-sdk-conformance/run_readme_examples.py`, `scripts/pi-sdk-conformance/consumer-readme/Cargo.toml`, `scripts/pi-sdk-conformance/test_readme_examples.py`, `CHANGELOG.md`
- **Change kind**: behavioral
- **Change**: Use existing public Models::new, provider_collection::register_builtin, BuiltinOverrides.base_urls, auth::AmbientAuthContext and HttpClient::new; no new SDK method, route/resolver abstraction, config key, feature or manual auth assembly.
- **Closure predicate**: Actual extracted-package consumer executes all three byte-identical README/example sources using ordinary explicit builtins and synthetic loopback auth;1/1/2 real request counts plus final outcome/tool-result continuation, default-memory/lazy-construction facts and exact artifact/consumer/source identities are recorded. PASS impossible when executions absent.
- **Red-before**: {"command": "D:\\Users\\Luiz\\anaconda3\\python.exe C:\\Users\\Luiz\\AppData\\Local\\Temp\\opi-phase20-remediate-hvrszrbs\\ready_onboarding_harness_probe.py", "expected": "Runner cannot report PASS without three actual example executions after package/embedding controls.", "observed": "FAIL exit1: unittest1 executed1failure; current harness returns PASS after real package/example embedding checks, execution_count0 rather than required3; subprocesses mocked, no Cargo build or live call"}
- **Green-after**: `python scripts/pi-sdk-conformance/run_readme_examples.py --offline --scratch "C:\Users\Luiz\AppData\Local\Temp\opi-phase20-ready-w8p6zu1s\apply-readme-package" --target-dir "E:\opi\cargo-targets\opi-72fa37a0fa044958"`; Actual extracted-package consumer executes all three byte-identical README/example sources using ordinary explicit builtins and synthetic loopback auth;1/1/2 real request counts plus final outcome/tool-result continuation, default-memory/lazy-construction facts and exact artifact/consumer/source identities are recorded. PASS impossible when executions absent.
- **Test impact**: {"add": "runner subprocess acceptance/negative cases and three actual same-source controlled-peer executions plus lazy-default probe", "update": "examples, bilingual exact embeddings, consumer dependency list", "delete": "only newly unused manual resolver/import dependencies in example harness; no retained behavior tests", "retain": "existing SDK all-wire and packaging/env/lease/source controls"}

### Batch B19: telemetry.stream-poll-drop-context

**Closure predicate**: Actual production Stream wrappers scope every observed poll_next and Drop with frozen source parent/Dispatch across Pending and thread hops; exactly one inner poll per outer poll and exactly one inner destruction. Private wrapper unit checks cover unpolled Drop without any business poll, ready/EOF and noop/None controls. Real Models native/simple/direct/advanced/deferred consumers preserve results, retry/cancel/poll/drop counts and do not leak to ambient or add business work.
**Dependencies**: ["B10", "B13"]
**Verification union**: rebuild baseline-current source rlib and relink ready_telemetry_probe.rs; run --nocapture --test-threads=1 expected1executed1passed; cargo test --locked --offline -p opi-ai --test telemetry_carrier; cargo test --locked --offline -p opi-ai --test telemetry_operations; cargo test --locked --offline -p opi-ai --test deferred_contract; shared union below.

#### Fix B19.1: telemetry.stream-poll-drop-context

- **Finding source(s)**: `phase20-codex-gpt61-703d8bd-20261101t181806z` + `1c60c2f6c88d4d5318822c7412fb4a3fd5d48287253bc661d36ba91e7c2a62ef` + P20-AUD-019
- **Decision**: fix:scope-real-sdk-stream-poll-and-drop-with-frozen-context
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-ai/src/telemetry.rs`, `crates/opi-ai/src/models.rs`, `crates/opi-ai/tests/telemetry_carrier.rs`, `crates/opi-ai/tests/telemetry_operations.rs`, `CHANGELOG.md`
- **Change kind**: behavioral
- **Change**: Add private InstrumentedStream<S: Stream> in telemetry.rs, matching current Instrumented<F> ownership: TelemetryContext plus Option<Pin<Box<S>>>; poll_next executes exactly one inner poll under context.in_scope; Drop takes and destroys inner under same scope, even unpolled. No public telemetry API, new trait, unsafe, global subscriber, exporter, business span or extra poll. Disabled carrier uses existing scoped no-op dispatch. Preserve returned Pending/item/EOF exactly.
- **Closure predicate**: Actual production Stream wrappers scope every observed poll_next and Drop with frozen source parent/Dispatch across Pending and thread hops; exactly one inner poll per outer poll and exactly one inner destruction. Private wrapper unit checks cover unpolled Drop without any business poll, ready/EOF and noop/None controls. Real Models native/simple/direct/advanced/deferred consumers preserve results, retry/cancel/poll/drop counts and do not leak to ambient or add business work.
- **Red-before**: {"command": "C:\\Users\\Luiz\\AppData\\Local\\Temp\\opi-phase20-remediate-hvrszrbs\\ready_telemetry_probe.exe --nocapture --test-threads=1", "expected": "Plain non-self-instrumented stream poll/drop belongs to frozen sourceA, exact business counts retained.", "observed": "FAIL exit101: compile0; real public Models::stream with registered plain ChatApi EventStream; positive context.in_scope control parented at sourceA. First outer poll Pending (inner polls1); drop on other thread (drops1). Plain stream does not call context.in_scope itself. SourceA only frozen_parent/control; both poll/drop markers root at ambientB.1executed1failed0ignored, exit101. Earlier invalid root import compiler setup excluded."}
- **Green-after**: `python "C:\Users\Luiz\AppData\Local\Temp\opi-phase20-remediate-hvrszrbs\replay_probe.py" --source-root "D:\Luiz\Odradek\opi" --target-dir "E:\opi\cargo-targets\opi-72fa37a0fa044958" --suite telemetry`; Actual production Stream wrappers scope every observed poll_next and Drop with frozen source parent/Dispatch across Pending and thread hops; exactly one inner poll per outer poll and exactly one inner destruction. Private wrapper unit checks cover unpolled Drop without any business poll, ready/EOF and noop/None controls. Real Models native/simple/direct/advanced/deferred consumers preserve results, retry/cancel/poll/drop counts and do not leak to ambient or add business work.
- **Test impact**: {"add": "private production Stream pending/cross-thread/unpolled-Drop/ready/noop tests and real public plain-adapter consumer discriminator", "update": "existing MarkerStream proof must exercise production wrapper rather than implement scoping itself", "delete": "none", "retain": "existing Future carrier/producer/request propagation/callback/retry/secret tests"}

### Batch B20: transcript.duplicate-section-batch-rejection

**Closure predicate**: Normalization and append reject same-name sections within one SystemMessage before state installation with typed error; failed append preserves prior transcript; unique batches, across-message replacement/delete/readd and JSON-object documented parser behavior remain valid.
**Dependencies**: []
**Verification union**: matching relinked current probe and owning focused suites; shared union below.

#### Fix B20.1: transcript.duplicate-section-batch-rejection

- **Finding source(s)**: `phase20-codex-gpt61-703d8bd-20261101t181806z` + `1c60c2f6c88d4d5318822c7412fb4a3fd5d48287253bc661d36ba91e7c2a62ef` + P20-AUD-021
- **Decision**: fix:reject-programmatic-duplicate-section-batches
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-ai/src/transcript.rs`, `crates/opi-ai/tests/transcript_contract.rs`, `CHANGELOG.md`
- **Change kind**: behavioral
- **Change**: Reject observable duplicate names within one programmatic SystemMessage before normalize/append installation; preserve JSON-object facts/across-message replacement.
- **Closure predicate**: Normalization and append reject same-name sections within one SystemMessage before state installation with typed error; failed append preserves prior transcript; unique batches, across-message replacement/delete/readd and JSON-object documented parser behavior remain valid.
- **Red-before**: {"command": "C:\\Users\\Luiz\\AppData\\Local\\Temp\\opi-phase20-remediate-hvrszrbs\\delivery-probe.exe --nocapture", "expected": "FAIL duplicate batch rejection assertions; unique-name control succeeds", "observed": "FAIL: exit101;2 executed,0passed,2failed,0ignored; duplicate normalize and append return Ok before typed rejection assertions."}
- **Green-after**: `python "C:\Users\Luiz\AppData\Local\Temp\opi-phase20-remediate-hvrszrbs\replay_probe.py" --source-root "D:\Luiz\Odradek\opi" --target-dir "E:\opi\cargo-targets\opi-72fa37a0fa044958" --suite delivery`; Normalization and append reject same-name sections within one SystemMessage before state installation with typed error; failed append preserves prior transcript; unique batches, across-message replacement/delete/readd and JSON-object documented parser behavior remain valid.
- **Test impact**: "add/update focused regressions; delete none; retain owning security/lifecycle/source controls."

### Batch B21: schema.cardinality-discriminating-corpus

**Closure predicate**: Each of four cardinality mechanisms has discriminating boundary/positive/negative controls; existing73 plus4 named checks total77 in both actual-package Schema profiles, with strict missing/duplicate/failed report rejection retained.
**Dependencies**: ["B6"]
**Verification union**: cargo test --locked --offline -p opi-ai --test schema_engine cardinality; cargo test --locked --offline -p opi-ai --test schema_public cardinality; python scripts/pi-sdk-conformance/test_package_proof.py; python scripts/pi-sdk-conformance/run_early_package_proof.py --scratch <fresh-workspace-external-scratch> --target-dir E:\opi\cargo-targets\opi-72fa37a0fa044958; shared union below.

#### Fix B21.1: schema.cardinality-discriminating-corpus

- **Finding source(s)**: `phase20-codex-gpt61-703d8bd-20261101t181806z` + `1c60c2f6c88d4d5318822c7412fb4a3fd5d48287253bc661d36ba91e7c2a62ef` + P20-AUD-022
- **Decision**: fix:add-schema-cardinality-boundary-corpus
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-ai/tests/schema_engine.rs`, `crates/opi-ai/tests/schema_public.rs`, `scripts/pi-sdk-conformance/consumer/src/main.rs`, `scripts/pi-sdk-conformance/run_early_package_proof.py`, `scripts/pi-sdk-conformance/test_package_proof.py`
- **Change kind**: test-only
- **Change**: Append exactly4 named combined positive/negative/boundary checks to actual consumer (one per cardinality keyword); existing73 retained gives77. Update harness check_report strict total/list-length/unique-name77 and error message, test fixture range77/last-name76 and add old73report-rejected control. Never replace exact-count check with permissive minimum or bypass missing/duplicate/failed checks.
- **Closure predicate**: Each of four cardinality mechanisms has discriminating boundary/positive/negative controls; existing73 plus4 named checks total77 in both actual-package Schema profiles, with strict missing/duplicate/failed report rejection retained.
- **Red-before**: "Test-only missing rejection corpus: current production cardinality leaves are present; no wrong runtime result is alleged. Static full scoped inventory establishes gap; new tests must pass current implementation and discriminate individual leaf removal."
- **Green-after**: `cargo test --locked --offline -p opi-ai --test schema_engine cardinality; cargo test --locked --offline -p opi-ai --test schema_public cardinality; python scripts/pi-sdk-conformance/test_package_proof.py; python scripts/pi-sdk-conformance/run_early_package_proof.py --scratch "C:\Users\Luiz\AppData\Local\Temp\opi-phase20-ready-w8p6zu1s\apply-cardinality-package" --target-dir "E:\opi\cargo-targets\opi-72fa37a0fa044958"`; Each of four cardinality mechanisms has discriminating boundary/positive/negative controls; existing73 plus4 named checks total77 in both actual-package Schema profiles, with strict missing/duplicate/failed report rejection retained.
- **Test impact**: "add4 cardinality controls, update exact73->77 report checker/tests, delete none, retain every existing73 case and negative CLI/report/path/lease controls."

### Batch B22: delivery.assertion-deletion-tally

**Closure predicate**: Append a factual superseding correction for the false zero, naming provider_kind_variants as proven lower bound; preserve original identities/history and report exact changed registered-source digest to its existing ledger owner.
**Dependencies**: []
**Verification union**: python scripts/opi-doc-check.py; git diff --check; static cross-check appended correction names provider_kind_variants, supersedes only incorrect delete0 and does not claim exhaustive totals or ledger reconciliation; shared union below.

#### Fix B22.1: delivery.assertion-deletion-tally

- **Finding source(s)**: `phase20-pi-glm53-6adacc37-20261101t164203z` + `fd1c6180b3c1951fa82efc2bdf2244af006ea8db1feca261c90267c59a881e1a` + P20-AUD-002
- **Decision**: fix:append-factual-assertion-deletion-tally-correction
- **Verification status**: Confirmed
- **File(s)**: `docs/research/pi-ai-opi-ai-alignment/acceptance-matrix.md`
- **Change kind**: documentation
- **Change**: Append a dated V-59 factual correction superseding only the2026-10-30 Phase20delivery-group delete=0 statement: at least1 documented deleted assertion, provider_kind_variants, retired with its shell in20.56 per current Unreleased. State this is a proven lower bound, not an invented exhaustive phase total; task20.59-own documentation edit has test impact none. Keep all AIR/V identities, earlier source observations, historical70/probe results and unrelated counts intact. No acceptance-matrix.zh.md counterpart exists; this source is already Chinese and source-only bilingual counterpart editing does not apply.
- **Closure predicate**: Append a factual superseding correction for the false zero, naming provider_kind_variants as proven lower bound; preserve original identities/history and report exact changed registered-source digest to its existing ledger owner.
- **Red-before**: "Documentation-only falsifiable current text comparison: matrix385 establishes per-assertion disposition;579-583 says delete0; current CHANGELOG11 documents the deleted assertion; current crates symbol absent. No new runtime behavior is being repaired."
- **Green-after**: `python scripts/opi-doc-check.py; git diff --check`; Factual correction explicitly records the established >=1 assertion deletion without inventing totals; all normative identities and ledger/status bytes unchanged.
- **Test impact**: {"add": "none", "update": "documented factual tally only", "delete": "none new", "retain": "all prior runtime/test/replacement/registered-source history and gate ownership"}

## Final Verification

Execute each created/modified focused test/filter and every same-predicate relinked probe. Public struct/trait/codec changes touch real cross-crate consumers, so use the deduplicated cross-crate union:

    cargo fmt --check --all
    cargo clippy --workspace --all-targets -- -D warnings
    cargo test --workspace --all-targets
    cargo test --workspace --doc
    $env:RUSTDOCFLAGS = "-D warnings"; cargo doc --workspace --no-deps
    python scripts/opi-doc-check.py
    git diff --check

Workflow/checker changes additionally run exact Python units plus representative subprocess invalid input/refusal. B21/cardinality uses `run_early_package_proof.py --scratch <fresh> --target-dir <existing-cache>` with no invented --offline CLI flag; both internal Cargo profiles are offline/locked and report exactly77. B18/onboarding runs actual exact package sources through controlled peers and refuses PASS with absent executions. Private/public package/native protocol gates and source/material input proofs must be refreshed when relevant changed source invalidates them; phase-wide native acceptance and F.1a remain under their existing owner after materialization.

Read complete current Unreleased before editing; all scoped0.x public behavior changes have owning bullets and synchronized current callers/docs/schemas/fixtures. Keep English/Chinese SDK README embeddings equal. Existing historical schema3/sessionv2 omission fixtures do not gain fabricated known facts or a new format. The fixed assurance artifacts have no Chinese counterpart. No ledger, source hash, task status or Phase status is written by remediation.

## Exclusions

| Source | Disposition / strongest current counter-evidence | Surviving observation / boundary |
|---|---|---|
| `phase20-codex-gpt61-703d8bd-20261101t181806z` / P20-AUD-020 | Refuted; no-action:current-native-package-and-material-proof-established; Retain original Major priority; the exact missing-proof claim is refuted at current source by newly executed native actual-package matrices and sealed pinned catalog material. This does not refute separate runtime findings or declare Phase conformance. | No product edit; linked source requirements are not relabeled met. Known separate runtime repairs and later phase gates remain distinct. |
| `phase20-pi-glm53-6adacc37-20261101t164203z` / P20-AUD-001 | Refuted; no-action:current-native-package-proof-established; Retain source Info; actual original CI logs independently prove macOS/aarch64 execution and dispatch at unchanged relevant source. New actual-package provenance is separately verified; no product edit or new Phase verdict follows. | No product edit; linked source requirements are not relabeled met. Known separate runtime repairs and later phase gates remain distinct. |

## Current native/package and material evidence

Current proof roots are `C:\Users\Luiz\AppData\Local\Temp\opi-phase20-ready-w8p6zu1s` (pristine archive/package/native consumers), `C:\Users\Luiz\AppData\Local\Temp\opi-phase20-remediate-hvrszrbs` (bounded red/design probes), and the explicit native job artifact. These are proof inputs, not alternate source/plan/lineage authority. Exact logs/package/compiler/graph/executable identities are checked before a no-action missing-proof refutation.

### Proof record: ready-ci-current-proof.json

SHA-256 `52fdaa36bcd6caf3b6717b5c9a67f59b6a7a0dd237a1de5ab7569985df532d50`

```json
{
  "checked_source_head": "8010ed2af97cd5deeb1f7240097436fdf32b8d54",
  "ci_source_head": "9e8d9156e3e5722eb7727db91bb6c6136308be04",
  "relevant_source_diff": [],
  "run_id": 41300159581,
  "artifact_list": [],
  "jobs": {
    "macos": {
      "job_id": 124829247918,
      "saved_decoded_log": "C:\\Users\\Luiz\\AppData\\Local\\Temp\\opi-phase20-ready-w8p6zu1s\\ci-macos.log",
      "sha256_saved_bytes": "b0ddc96b606a097629d04fc75e28fe3777cd5a4cfd6492604545f19e474296e1",
      "summaries": 272,
      "passed": 6394,
      "ignored": 7,
      "has_native_package_proof": false
    },
    "windows": {
      "job_id": 124829247817,
      "saved_decoded_log": "C:\\Users\\Luiz\\AppData\\Local\\Temp\\opi-phase20-ready-w8p6zu1s\\ci-windows.log",
      "sha256_saved_bytes": "ce216a53be344ad52f92aa272e0c7718c282e489e4114659401131ed0b05259d",
      "summaries": 272,
      "passed": 6264,
      "ignored": 5,
      "has_native_package_proof": false
    },
    "linux": {
      "job_id": 124829247913,
      "saved_decoded_log": "C:\\Users\\Luiz\\AppData\\Local\\Temp\\opi-phase20-ready-w8p6zu1s\\ci-linux.log",
      "sha256_saved_bytes": "709586179f7ee8ec7d480e0c28cf204ff732a544af7e7dc0dba24ce0b2ab61f8",
      "summaries": 272,
      "passed": 6543,
      "ignored": 20,
      "has_native_package_proof": false
    },
    "msrv": {
      "job_id": 124829247778,
      "saved_decoded_log": "C:\\Users\\Luiz\\AppData\\Local\\Temp\\opi-phase20-ready-w8p6zu1s\\ci-msrv.log",
      "sha256_saved_bytes": "ad74f92ec49f2cb3dab6f74ec09c1086b23d8a328473f6c18360c579f7b3c433",
      "summaries": 0,
      "passed": 0,
      "ignored": 0,
      "has_native_package_proof": false
    }
  }
}
```

### Proof record: full-sdk-proof-v2/evidence.json

SHA-256 `f682be1574f321038c46c972ffdaed3e662d99b4e09faa566b0c9bf84053fab9`

```json
{
  "package": "C:\\Users\\Luiz\\AppData\\Local\\Temp\\opi-phase20-ready-w8p6zu1s\\package-ws-proof\\package\\opi-ai-0.8.3",
  "tests": [
    "external_sdk",
    "provider_collection",
    "oauth_flows",
    "deferred_contract",
    "classifier_contract"
  ],
  "verdict": "PASS",
  "profiles": [
    {
      "name": "ordinary",
      "executed": 385,
      "failed": 0,
      "ignored": 0
    },
    {
      "name": "default",
      "executed": 385,
      "failed": 0,
      "ignored": 0
    },
    {
      "name": "c-zlib",
      "executed": 385,
      "failed": 0,
      "ignored": 0
    }
  ]
}
```

### Proof record: package-ws-proof/evidence.json

SHA-256 `bbb49c001c46814736bb3182e8959fad7bdd6fd31b8b3aae1ca8a01f1c0a3c56`

```json
{
  "family": "ws",
  "scope": "packaged opi-ai protocol supplements and public Upgrade tests plus the external real-chain Upgrade proof (task 20.16)",
  "target": "E:\\opi\\cargo-targets\\opi-72fa37a0fa044958",
  "controlled_native_inputs": {
    "AWS_LC_SYS_USE_SYSTEM": "0",
    "LIBZ_SYS_STATIC": "1",
    "VCPKGRS_DISABLE": "1",
    "CC_ENABLE_DEBUG_OUTPUT": "1"
  },
  "verdict": "PASS",
  "public_codec_status": "CLOSED: task 20.16 real-chain consumer check",
  "public_codec_exercised": true,
  "rustc": "rustc 1.99.1 (58cf8f9c3 2026-10-13)\nbinary: rustc\ncommit-hash: 58cf8f9c35b355199727627d07ae34a4e2846d67\ncommit-date: 2026-10-13\nhost: x86_64-pc-windows-msvc\nrelease: 1.99.1\nLLVM version: 23.1.1\n",
  "cargo": "cargo 1.99.1 (5f94df478 2026-08-27)\nrelease: 1.99.1\ncommit-hash: 5f94df4789f005f9a352888e8355ffc645b7ed0e\ncommit-date: 2026-08-27\nhost: x86_64-pc-windows-msvc\nlibgit2: 1.9.6 (sys:0.21.0 vendored)\nlibcurl: 8.21.0-DEV (sys:0.4.90+curl-8.21.0 vendored ssl:Schannel)\nos: Windows 10.0.22631 (Windows 11 Core) [64-bit]\n",
  "package": {
    "file": "opi-ai-0.8.3.crate",
    "sha256": "0110e99980f002998edf81aa76fba30a4f0a402b864b4ff9a8d88e7344061a22"
  },
  "consumer_lock_sha256": "c5bf33bc5f450f1cc968bf948ed4f4c455b4c0f47254299bf60b2ed3f40068e5",
  "lock_checksums": {
    "aws-lc-rs": "faac5829c2b74c28f830747e7818ccfb684261b5f48a1118b1e2a13d36dfab13",
    "aws-lc-sys": "f1622d8446a2d4b2ce0c7eefc73dd43a99779028d5ee5c2dd8073a658ba8a2bc",
    "flate2": "6e634e2e0ebac1ee034020da1ca582e17ffe4e0f5e985823721e168928136dcb",
    "libz-rs-sys": "434324ee61a8e4a05dd3fdf03d51c073ea7fdaf5a35b2ca33c4ca9f3f37def4a",
    "libz-sys": "f710a23e6dbf193214fd46ca56a9d6864e550abe86202184532ae7275e46de19",
    "rustls": "ef86cd5876211988985292b91c96a8f2d298df24e75989a43a3c73f2d4d8168b",
    "zlib-rs": "b268e58e7c693d7c271f93ffc4ba3b380412554231c85bf61ca7af91042a4112"
  },
  "profiles": [
    {
      "name": "ordinary",
      "scope": "external public Upgrade, raw stream and the real Models -> Codex -> Upgrade -> private-codec chain",
      "report": {
        "verdict": "PASS",
        "scope": "external public HTTP Upgrade, raw stream and the real Models -> Codex -> Upgrade -> private-codec chain",
        "public_codec_status": "CLOSED: real-chain window8/PMD/tail/Close/cleanup checks",
        "failed": 0,
        "total": 10,
        "checks": [
          {
            "name": "strict-upgrade-and-raw-tail-once",
            "passed": true,
            "detail": ""
          },
          {
            "name": "models-codex-upgrade-private-codec-chain",
            "passed": true,
            "detail": ""
          },
          {
            "name": "models-codex-window8-pmd-trailing-close",
            "passed": true,
            "detail": ""
          },
          {
            "name": "models-codex-active-cleanup",
            "passed": true,
            "detail": ""
          },
          {
            "name": "models-codex-idle-cleanup",
            "passed": true,
            "detail": ""
          },
          {
            "name": "direction-specific-pmd-facts",
            "passed": true,
            "detail": ""
          },
          {
            "name": "reject-mismatched-accept",
            "passed": true,
            "detail": ""
          },
          {
            "name": "reject-unoffered-subprotocol",
            "passed": true,
            "detail": ""
          },
          {
            "name": "reject-unsolicited-extension",
            "passed": true,
            "detail": ""
          },
          {
            "name": "redirect-is-terminal",
            "passed": true,
            "detail": ""
          }
        ]
      }
    },
    {
      "name": "union-default",
      "scope": "external public Upgrade, raw stream and the real Models -> Codex -> Upgrade -> private-codec chain",
      "report": {
        "verdict": "PASS",
        "scope": "external public HTTP Upgrade, raw stream and the real Models -> Codex -> Upgrade -> private-codec chain",
        "public_codec_status": "CLOSED: real-chain window8/PMD/tail/Close/cleanup checks",
        "failed": 0,
        "total": 10,
        "checks": [
          {
            "name": "strict-upgrade-and-raw-tail-once",
            "passed": true,
            "detail": ""
          },
          {
            "name": "models-codex-upgrade-private-codec-chain",
            "passed": true,
            "detail": ""
          },
          {
            "name": "models-codex-window8-pmd-trailing-close",
            "passed": true,
            "detail": ""
          },
          {
            "name": "models-codex-active-cleanup",
            "passed": true,
            "detail": ""
          },
          {
            "name": "models-codex-idle-cleanup",
            "passed": true,
            "detail": ""
          },
          {
            "name": "direction-specific-pmd-facts",
            "passed": true,
            "detail": ""
          },
          {
            "name": "reject-mismatched-accept",
            "passed": true,
            "detail": ""
          },
          {
            "name": "reject-unoffered-subprotocol",
            "passed": true,
            "detail": ""
          },
          {
            "name": "reject-unsolicited-extension",
            "passed": true,
            "detail": ""
          },
          {
            "name": "redirect-is-terminal",
            "passed": true,
            "detail": ""
          }
        ]
      }
    },
    {
      "name": "union-c-zlib",
      "scope": "external public Upgrade, raw stream and the real Models -> Codex -> Upgrade -> private-codec chain",
      "report": {
        "verdict": "PASS",
        "scope": "external public HTTP Upgrade, raw stream and the real Models -> Codex -> Upgrade -> private-codec chain",
        "public_codec_status": "CLOSED: real-chain window8/PMD/tail/Close/cleanup checks",
        "failed": 0,
        "total": 10,
        "checks": [
          {
            "name": "strict-upgrade-and-raw-tail-once",
            "passed": true,
            "detail": ""
          },
          {
            "name": "models-codex-upgrade-private-codec-chain",
            "passed": true,
            "detail": ""
          },
          {
            "name": "models-codex-window8-pmd-trailing-close",
            "passed": true,
            "detail": ""
          },
          {
            "name": "models-codex-active-cleanup",
            "passed": true,
            "detail": ""
          },
          {
            "name": "models-codex-idle-cleanup",
            "passed": true,
            "detail": ""
          },
          {
            "name": "direction-specific-pmd-facts",
            "passed": true,
            "detail": ""
          },
          {
            "name": "reject-mismatched-accept",
            "passed": true,
            "detail": ""
          },
          {
            "name": "reject-unoffered-subprotocol",
            "passed": true,
            "detail": ""
          },
          {
            "name": "reject-unsolicited-extension",
            "passed": true,
            "detail": ""
          },
          {
            "name": "redirect-is-terminal",
            "passed": true,
            "detail": ""
          }
        ]
      }
    }
  ]
}
```

### Proof record: package-http-proof/evidence.json

SHA-256 `f5ed07b71377248cb6713eb0b2d4b435c670a030fbb831ed564406a9bd307f65`

```json
{
  "family": "http",
  "scope": "packaged public-consumer family proof; not final SDK acceptance",
  "target": "E:\\opi\\cargo-targets\\opi-72fa37a0fa044958",
  "controlled_native_inputs": {
    "AWS_LC_SYS_USE_SYSTEM": "0",
    "LIBZ_SYS_STATIC": "1",
    "VCPKGRS_DISABLE": "1",
    "CC_ENABLE_DEBUG_OUTPUT": "1"
  },
  "verdict": "PASS",
  "rustc": "rustc 1.99.1 (58cf8f9c3 2026-10-13)\nbinary: rustc\ncommit-hash: 58cf8f9c35b355199727627d07ae34a4e2846d67\ncommit-date: 2026-10-13\nhost: x86_64-pc-windows-msvc\nrelease: 1.99.1\nLLVM version: 23.1.1\n",
  "cargo": "cargo 1.99.1 (5f94df478 2026-08-27)\nrelease: 1.99.1\ncommit-hash: 5f94df4789f005f9a352888e8355ffc645b7ed0e\ncommit-date: 2026-08-27\nhost: x86_64-pc-windows-msvc\nlibgit2: 1.9.6 (sys:0.21.0 vendored)\nlibcurl: 8.21.0-DEV (sys:0.4.90+curl-8.21.0 vendored ssl:Schannel)\nos: Windows 10.0.22631 (Windows 11 Core) [64-bit]\n",
  "package": {
    "file": "opi-ai-0.8.3.crate",
    "sha256": "0110e99980f002998edf81aa76fba30a4f0a402b864b4ff9a8d88e7344061a22"
  },
  "consumer_lock_sha256": "1aba2f2d5f37a1fe77b7d88774c652c55c0f68331c321a86dfbbdbf8605777f0",
  "lock_checksums": {
    "http": "e3ba2a386d7f85a81f119ad7498ebe444d2e22c2af0b86b069416ace48b3311a",
    "hyper": "6299f016b246a94207e63da54dbe807655bf9e00044f73ded42c3ac5305fbcca",
    "hyper-util": "96547c2556ec9d12fb1578c4eaf448b04993e7fb79cbaad930a656880a6bdfa0",
    "reqwest": "eddd3ca559203180a307f12d114c268abf583f59b03cb906fd0b3ff8646c1147"
  },
  "profiles": [
    {
      "name": "ordinary",
      "scope": "external public consumer",
      "report": {
        "verdict": "PASS",
        "profile": "ordinary",
        "total": 6,
        "failed": 0,
        "checks": [
          {
            "name": "redirect-policy-owned-default",
            "passed": true,
            "detail": "HttpClient default is Limited(10) and configurable"
          },
          {
            "name": "same-origin-redirect-preserves-authorization",
            "passed": true,
            "detail": "sends=2 hop2_target=Some(\"/final\")"
          },
          {
            "name": "cross-origin-redirect-strips-authorization",
            "passed": true,
            "detail": "origin_auth=Some(\"Bearer pkg-consumer-key\") target_auth=None target_sends=1"
          },
          {
            "name": "redirect-limit-typed-failure-exact-sends",
            "passed": true,
            "detail": "error=Network(ProviderErrorSummary(\"too many redirects\")) sends=3"
          },
          {
            "name": "proxy-routing-with-scoped-bypass",
            "passed": true,
            "detail": "proxy_hits=1 absolute_target=Some(\"http://127.0.0.1:5426/v1/chat/completions\") target_sends=2"
          },
          {
            "name": "acquisition-failure-and-single-terminal-eof",
            "passed": true,
            "detail": "acquisition=Network(ProviderErrorSummary(\"error sending request for url (http://127.0.0.1:5431/v1/chat/completions)\")) eof=StreamFailure(StreamError(ProviderErrorSummary(\"stream ended without a terminal event\"))) sends=1"
          }
        ]
      }
    },
    {
      "name": "caller-union",
      "scope": "external public consumer",
      "report": {
        "verdict": "PASS",
        "profile": "caller-union",
        "total": 6,
        "failed": 0,
        "checks": [
          {
            "name": "redirect-policy-owned-default",
            "passed": true,
            "detail": "HttpClient default is Limited(10) and configurable"
          },
          {
            "name": "same-origin-redirect-preserves-authorization",
            "passed": true,
            "detail": "sends=2 hop2_target=Some(\"/final\")"
          },
          {
            "name": "cross-origin-redirect-strips-authorization",
            "passed": true,
            "detail": "origin_auth=Some(\"Bearer pkg-consumer-key\") target_auth=None target_sends=1"
          },
          {
            "name": "redirect-limit-typed-failure-exact-sends",
            "passed": true,
            "detail": "error=Network(ProviderErrorSummary(\"too many redirects\")) sends=3"
          },
          {
            "name": "proxy-routing-with-scoped-bypass",
            "passed": true,
            "detail": "proxy_hits=1 absolute_target=Some(\"http://127.0.0.1:11723/v1/chat/completions\") target_sends=2"
          },
          {
            "name": "acquisition-failure-and-single-terminal-eof",
            "passed": true,
            "detail": "acquisition=Network(ProviderErrorSummary(\"error sending request for url (http://127.0.0.1:11728/v1/chat/completions)\")) eof=StreamFailure(StreamError(ProviderErrorSummary(\"stream ended without a terminal event\"))) sends=1"
          }
        ]
      }
    }
  ]
}
```

### Proof record: package-crypto-proof/evidence.json

SHA-256 `5cc9763253e62e3889f85d0386a84fe77fc407d10bedd4059daa717bca16f070`

```json
{
  "family": "crypto",
  "scope": "packaged public-consumer family proof; not final SDK acceptance",
  "target": "E:\\opi\\cargo-targets\\opi-72fa37a0fa044958",
  "controlled_native_inputs": {
    "AWS_LC_SYS_USE_SYSTEM": "0",
    "LIBZ_SYS_STATIC": "1",
    "VCPKGRS_DISABLE": "1",
    "CC_ENABLE_DEBUG_OUTPUT": "1"
  },
  "verdict": "PASS",
  "rustc": "rustc 1.99.1 (58cf8f9c3 2026-10-13)\nbinary: rustc\ncommit-hash: 58cf8f9c35b355199727627d07ae34a4e2846d67\ncommit-date: 2026-10-13\nhost: x86_64-pc-windows-msvc\nrelease: 1.99.1\nLLVM version: 23.1.1\n",
  "cargo": "cargo 1.99.1 (5f94df478 2026-08-27)\nrelease: 1.99.1\ncommit-hash: 5f94df4789f005f9a352888e8355ffc645b7ed0e\ncommit-date: 2026-08-27\nhost: x86_64-pc-windows-msvc\nlibgit2: 1.9.6 (sys:0.21.0 vendored)\nlibcurl: 8.21.0-DEV (sys:0.4.90+curl-8.21.0 vendored ssl:Schannel)\nos: Windows 10.0.22631 (Windows 11 Core) [64-bit]\n",
  "package": {
    "file": "opi-ai-0.8.3.crate",
    "sha256": "0110e99980f002998edf81aa76fba30a4f0a402b864b4ff9a8d88e7344061a22"
  },
  "consumer_lock_sha256": "699d4346b2c06ff5d2368b633d2c4a03dc9060b0ff18023002034259ebe4aa7e",
  "lock_checksums": {
    "aws-lc-rs": "faac5829c2b74c28f830747e7818ccfb684261b5f48a1118b1e2a13d36dfab13",
    "aws-lc-sys": "f1622d8446a2d4b2ce0c7eefc73dd43a99779028d5ee5c2dd8073a658ba8a2bc",
    "rustls": "ef86cd5876211988985292b91c96a8f2d298df24e75989a43a3c73f2d4d8168b"
  },
  "profiles": [
    {
      "name": "ordinary",
      "scope": "external public consumer",
      "report": {
        "verdict": "PASS",
        "profile": "ordinary",
        "total": 6,
        "failed": 0,
        "checks": [
          {
            "name": "ordinary-no-global-provider",
            "passed": true,
            "detail": "fresh ordinary consumer has no process-global provider"
          },
          {
            "name": "chat-mtls-tls13-h2-round-trip",
            "passed": true,
            "detail": "response_id=Ok(\"chatcmpl-pkg\") accepts=1 http=1"
          },
          {
            "name": "chat-tls12-h1-explicit-selection",
            "passed": true,
            "detail": "response_id=Ok(\"chatcmpl-pkg\")"
          },
          {
            "name": "untrusted-roots-zero-business-http",
            "passed": true,
            "detail": "error=Err(Network(ProviderErrorSummary(\"error sending request for url (https://127.0.0.1:10616/v1/chat/completions)\"))) http=0"
          },
          {
            "name": "mismatched-identity-rejected-before-io",
            "passed": true,
            "detail": "build_error=Some(\"build: TLS configuration rejected: client identity could not be assembled\")"
          },
          {
            "name": "global-provider-state-unchanged",
            "passed": true,
            "detail": "global present=false (union=false)"
          }
        ]
      }
    },
    {
      "name": "caller-union",
      "scope": "external public consumer",
      "report": {
        "verdict": "PASS",
        "profile": "caller-union",
        "total": 6,
        "failed": 0,
        "checks": [
          {
            "name": "caller-union-host-ring-installed",
            "passed": true,
            "detail": "host ring provider installed as the process default"
          },
          {
            "name": "chat-mtls-tls13-h2-round-trip",
            "passed": true,
            "detail": "response_id=Ok(\"chatcmpl-pkg\") accepts=1 http=1"
          },
          {
            "name": "chat-tls12-h1-explicit-selection",
            "passed": true,
            "detail": "response_id=Ok(\"chatcmpl-pkg\")"
          },
          {
            "name": "untrusted-roots-zero-business-http",
            "passed": true,
            "detail": "error=Err(Network(ProviderErrorSummary(\"error sending request for url (https://127.0.0.1:11179/v1/chat/completions)\"))) http=0"
          },
          {
            "name": "mismatched-identity-rejected-before-io",
            "passed": true,
            "detail": "build_error=Some(\"build: TLS configuration rejected: client identity could not be assembled\")"
          },
          {
            "name": "global-provider-state-unchanged",
            "passed": true,
            "detail": "global present=true (union=true)"
          }
        ]
      }
    }
  ]
}
```

### Proof record: ready-final-proof.json

SHA-256 `32dbd4650263d833f96f658f90ff1dc75b384f82cbaa5f74e4a7662947d77326`

```json
{
  "schema": "opi-ready-current-proof/1",
  "status": "PASS",
  "verdict": "PASS",
  "identity": {
    "head": "8010ed2af97cd5deeb1f7240097436fdf32b8d54",
    "audit_index_sha256": "fb22a4ff86318a1f322cfbcf9bf107d748d65bbaa7c1ccb1037f6496d69db3ce",
    "native_source_head": "e433ac833a28ece771ec8dbb00f947efbd592055",
    "relevant_files": 948,
    "relevant_tree_sha256": "65f4c93c6101f4919d7e663aacb4ebfa833353282cb4c139294672acac077b53",
    "local_ledger_sha256": {
      ".opi-impl-state.json": "15c7f977d0c2cd7aee098836d5d47c195b402e1e2fb9a82a16fb69c89f52f975",
      "docs/snapshots/phase20/opi-impl-state.json": "222372715140f4d66ae43ed6c39779daca50f167e0407503d252536bcbd86bb8"
    },
    "equivalence_boundary": "Runtime/build/CI and registered normative SDK inputs only; remote e433 has a different root delivery ledger and omits Phase20 ledger. Local ledger bytes remain separately bound to H; no remote ledger equivalence or status is asserted."
  },
  "ci": {
    "runs": [
      {
        "run_id": 41367832337,
        "conclusion": "failure",
        "head": "eec075ab7d20049aa16120e7a335183ad185ac75"
      },
      {
        "run_id": 41368083798,
        "conclusion": "failure",
        "head": "8a256a9fbf6b3f09610c5ce8cfedec25b6cc69ac"
      },
      {
        "run_id": 41368868967,
        "conclusion": "success",
        "head": "98db80dc048badf769f9eedbae4ededcccecfc1e"
      }
    ],
    "cumulative_runner_seconds": 1881,
    "limit_seconds": 5400,
    "successful_ci_head": "98db80dc048badf769f9eedbae4ededcccecfc1e",
    "exact_driver_sha256": {
      "native_ready_proof.py": "86bb2fab258c9c5460841dacec253ea97141f3fd23c1fb26cd4c76ac846fa354",
      "full_sdk_matrix.py": "0de5150e406bad76810fe1ff67e0c244bb4297a0391c6e9d1327128358c71874",
      "workflow_git_blob": "5e80122b0a00b57917971a1fb4ade37b929773433fe8ac905a7654d54c99cd6a"
    },
    "failed_setup_runs_excluded_from_acceptance": true,
    "cited_workspace_run": 41300159581,
    "cited_workspace_source_head": "9e8d9156e3e5722eb7727db91bb6c6136308be04",
    "cited_workspace_boundary": "Native workspace execution/MSRV compilation only; current actual-package proof is independently established below."
  },
  "catalog": {
    "artifact_sha256": "ffa3e98626230f378f80200debb736c4a2238b5f2ba246a92cfc7d8e19604678",
    "original_material_files": 13,
    "boundary": "Pinned local Pi1.3 source/pin/helper association; no upstream tag/npm authentication or new network acquisition claimed."
  },
  "platforms": {
    "windows": {
      "families": {
        "ws": {
          "package": {
            "sha256": "0110e99980f002998edf81aa76fba30a4f0a402b864b4ff9a8d88e7344061a22",
            "members": 250
          },
          "profiles": [
            {
              "profile": "ordinary",
              "checks": 10,
              "recorded_executable_sha256": "5708f7c4617d9644ded90dfb0749cbc09c223e53b87ca0358c887375c5d91bb9",
              "native": [
                {
                  "owner": "aws-lc-sys",
                  "source_checksum": "f1622d8446a2d4b2ce0c7eefc73dd43a99779028d5ee5c2dd8073a658ba8a2bc",
                  "source_files": 2044,
                  "static_archive_sha256": [
                    "f2edfab3a8ed19254af1be9df28445ce7a6298ceceae7cb9e675851a81e55163"
                  ]
                }
              ]
            },
            {
              "profile": "union-default",
              "checks": 10,
              "recorded_executable_sha256": "70ec76e94ac88b7590dcfce37890cd6d80445ce9f3124f4deaa7cafa7ccc8a52",
              "native": [
                {
                  "owner": "aws-lc-sys",
                  "source_checksum": "f1622d8446a2d4b2ce0c7eefc73dd43a99779028d5ee5c2dd8073a658ba8a2bc",
                  "source_files": 2044,
                  "static_archive_sha256": [
                    "f2edfab3a8ed19254af1be9df28445ce7a6298ceceae7cb9e675851a81e55163"
                  ]
                }
              ]
            },
            {
              "profile": "union-c-zlib",
              "checks": 10,
              "recorded_executable_sha256": "476d8c5a874585a981742fb76fa1950b8a14b2633772564f96a0e098018e8020",
              "native": [
                {
                  "owner": "aws-lc-sys",
                  "source_checksum": "f1622d8446a2d4b2ce0c7eefc73dd43a99779028d5ee5c2dd8073a658ba8a2bc",
                  "source_files": 2044,
                  "static_archive_sha256": [
                    "f2edfab3a8ed19254af1be9df28445ce7a6298ceceae7cb9e675851a81e55163"
                  ]
                },
                {
                  "owner": "libz-sys",
                  "source_checksum": "f710a23e6dbf193214fd46ca56a9d6864e550abe86202184532ae7275e46de19",
                  "source_files": 238,
                  "static_archive_sha256": [
                    "db1ddc810d004ec463e8ee41095c44e6f512d7a88f7a165674691e07811ef3fd"
                  ]
                }
              ]
            }
          ],
          "supplementary_executable_retention": "Only final profile original binary retained; earlier profile hashes/reports/Cargo/native logs retained. Post-family ordinary/default-labelled copies contain final binary, not earlier originals."
        },
        "http": {
          "package": {
            "sha256": "0110e99980f002998edf81aa76fba30a4f0a402b864b4ff9a8d88e7344061a22",
            "members": 250
          },
          "profiles": [
            {
              "profile": "ordinary",
              "checks": 6,
              "recorded_executable_sha256": "35596f27b94da130609a81f2ef68f96b16697b3ef29d82b36eb308e2773d53aa",
              "native": [
                {
                  "owner": "aws-lc-sys",
                  "source_checksum": "f1622d8446a2d4b2ce0c7eefc73dd43a99779028d5ee5c2dd8073a658ba8a2bc",
                  "source_files": 2044,
                  "static_archive_sha256": [
                    "f2edfab3a8ed19254af1be9df28445ce7a6298ceceae7cb9e675851a81e55163"
                  ]
                }
              ]
            },
            {
              "profile": "caller-union",
              "checks": 6,
              "recorded_executable_sha256": "a1de81430c567b216d6c245ee3d94e32eb19fc5ee4300e1e430f5d2fc3ff4b6e",
              "native": [
                {
                  "owner": "aws-lc-sys",
                  "source_checksum": "f1622d8446a2d4b2ce0c7eefc73dd43a99779028d5ee5c2dd8073a658ba8a2bc",
                  "source_files": 2044,
                  "static_archive_sha256": [
                    "f2edfab3a8ed19254af1be9df28445ce7a6298ceceae7cb9e675851a81e55163"
                  ]
                }
              ]
            }
          ],
          "supplementary_executable_retention": "Only final profile original binary retained; earlier profile hashes/reports/Cargo/native logs retained. Post-family ordinary/default-labelled copies contain final binary, not earlier originals."
        },
        "crypto": {
          "package": {
            "sha256": "0110e99980f002998edf81aa76fba30a4f0a402b864b4ff9a8d88e7344061a22",
            "members": 250
          },
          "profiles": [
            {
              "profile": "ordinary",
              "checks": 6,
              "recorded_executable_sha256": "14ab7ce19be5d79abd6412262ff9dabb5e026c22b383cd23128b0f9bc5d8b37f",
              "native": [
                {
                  "owner": "aws-lc-sys",
                  "source_checksum": "f1622d8446a2d4b2ce0c7eefc73dd43a99779028d5ee5c2dd8073a658ba8a2bc",
                  "source_files": 2044,
                  "static_archive_sha256": [
                    "f2edfab3a8ed19254af1be9df28445ce7a6298ceceae7cb9e675851a81e55163"
                  ]
                }
              ]
            },
            {
              "profile": "caller-union",
              "checks": 6,
              "recorded_executable_sha256": "851dbe6df6253d73853269899f9178c66474a50b3ebfb99b9c7a08fd109c5a13",
              "native": [
                {
                  "owner": "aws-lc-sys",
                  "source_checksum": "f1622d8446a2d4b2ce0c7eefc73dd43a99779028d5ee5c2dd8073a658ba8a2bc",
                  "source_files": 2044,
                  "static_archive_sha256": [
                    "f2edfab3a8ed19254af1be9df28445ce7a6298ceceae7cb9e675851a81e55163"
                  ]
                }
              ]
            }
          ],
          "supplementary_executable_retention": "Only final profile original binary retained; earlier profile hashes/reports/Cargo/native logs retained. Post-family ordinary/default-labelled copies contain final binary, not earlier originals."
        }
      },
      "full_sdk": [
        {
          "profile": "ordinary",
          "executed": 385,
          "failed": 0,
          "ignored": 0,
          "retained_executables": 6,
          "graph_sha256": "c482c5c668013759e0f1c7c511f277c57f98002fd3cc18a998eaf4ada2087970"
        },
        {
          "profile": "default",
          "executed": 385,
          "failed": 0,
          "ignored": 0,
          "retained_executables": 6,
          "graph_sha256": "e561ab949b02a259b9dd0a721f0743f6c2ddfae586d018c8d308ad9cbf17a186"
        },
        {
          "profile": "c-zlib",
          "executed": 385,
          "failed": 0,
          "ignored": 0,
          "retained_executables": 6,
          "graph_sha256": "ac98a098d6d685358c3594ca23ad0ca67b0ea36ff0e7a1319c258f16bcdf2fb5"
        }
      ],
      "schema": {
        "package": {
          "sha256": "0110e99980f002998edf81aa76fba30a4f0a402b864b4ff9a8d88e7344061a22",
          "members": 250
        },
        "profiles": 2,
        "checks_per_profile": 73
      }
    },
    "macos": {
      "families": {
        "ws": {
          "package": {
            "sha256": "0110e99980f002998edf81aa76fba30a4f0a402b864b4ff9a8d88e7344061a22",
            "members": 250
          },
          "profiles": [
            {
              "profile": "ordinary",
              "checks": 10,
              "recorded_executable_sha256": "6629baabb0750001df0a5d27bc754e33b1b85a56a2c99b7f6340f2c44ca4ac68",
              "native": [
                {
                  "owner": "aws-lc-sys",
                  "source_checksum": "f1622d8446a2d4b2ce0c7eefc73dd43a99779028d5ee5c2dd8073a658ba8a2bc",
                  "source_files": 2044,
                  "static_archive_sha256": [
                    "0ff7c176f9eb551eb0b49889d9a1edd0def5fdcb87f4fde15289fa71a3320abf"
                  ]
                }
              ]
            },
            {
              "profile": "union-default",
              "checks": 10,
              "recorded_executable_sha256": "33e21361cb0589ae1c0a96328b19190cca2d022890cba111bf20ff59d509b713",
              "native": [
                {
                  "owner": "aws-lc-sys",
                  "source_checksum": "f1622d8446a2d4b2ce0c7eefc73dd43a99779028d5ee5c2dd8073a658ba8a2bc",
                  "source_files": 2044,
                  "static_archive_sha256": [
                    "0ff7c176f9eb551eb0b49889d9a1edd0def5fdcb87f4fde15289fa71a3320abf"
                  ]
                }
              ]
            },
            {
              "profile": "union-c-zlib",
              "checks": 10,
              "recorded_executable_sha256": "ba93e8491c9688cca338f316cf04ad8e33f0f4b0ab044cdb7bdca7108a134085",
              "native": [
                {
                  "owner": "aws-lc-sys",
                  "source_checksum": "f1622d8446a2d4b2ce0c7eefc73dd43a99779028d5ee5c2dd8073a658ba8a2bc",
                  "source_files": 2044,
                  "static_archive_sha256": [
                    "0ff7c176f9eb551eb0b49889d9a1edd0def5fdcb87f4fde15289fa71a3320abf"
                  ]
                },
                {
                  "owner": "libz-sys",
                  "source_checksum": "f710a23e6dbf193214fd46ca56a9d6864e550abe86202184532ae7275e46de19",
                  "source_files": 238,
                  "static_archive_sha256": [
                    "65b3b46b2e2dffc59c2432c9d74960857f7ddff102ce50a4f82f845bc291018a"
                  ]
                }
              ]
            }
          ],
          "supplementary_executable_retention": "Only final profile original binary retained; earlier profile hashes/reports/Cargo/native logs retained. Post-family ordinary/default-labelled copies contain final binary, not earlier originals."
        },
        "http": {
          "package": {
            "sha256": "0110e99980f002998edf81aa76fba30a4f0a402b864b4ff9a8d88e7344061a22",
            "members": 250
          },
          "profiles": [
            {
              "profile": "ordinary",
              "checks": 6,
              "recorded_executable_sha256": "b87738bd445e43e5452b8e2a778bbf78ae6bc51d5d17402666d714499252fc9f",
              "native": [
                {
                  "owner": "aws-lc-sys",
                  "source_checksum": "f1622d8446a2d4b2ce0c7eefc73dd43a99779028d5ee5c2dd8073a658ba8a2bc",
                  "source_files": 2044,
                  "static_archive_sha256": [
                    "0ff7c176f9eb551eb0b49889d9a1edd0def5fdcb87f4fde15289fa71a3320abf"
                  ]
                }
              ]
            },
            {
              "profile": "caller-union",
              "checks": 6,
              "recorded_executable_sha256": "cc2e0ac19cc06ba52425081b3a0841448113c08403b4d82fa5ee6928d2edc1c0",
              "native": [
                {
                  "owner": "aws-lc-sys",
                  "source_checksum": "f1622d8446a2d4b2ce0c7eefc73dd43a99779028d5ee5c2dd8073a658ba8a2bc",
                  "source_files": 2044,
                  "static_archive_sha256": [
                    "0ff7c176f9eb551eb0b49889d9a1edd0def5fdcb87f4fde15289fa71a3320abf"
                  ]
                }
              ]
            }
          ],
          "supplementary_executable_retention": "Only final profile original binary retained; earlier profile hashes/reports/Cargo/native logs retained. Post-family ordinary/default-labelled copies contain final binary, not earlier originals."
        },
        "crypto": {
          "package": {
            "sha256": "0110e99980f002998edf81aa76fba30a4f0a402b864b4ff9a8d88e7344061a22",
            "members": 250
          },
          "profiles": [
            {
              "profile": "ordinary",
              "checks": 6,
              "recorded_executable_sha256": "a04b956d61f08112dd5b12eb5dd3b7e6074981773d6da5de5c6ee3e3e7d16371",
              "native": [
                {
                  "owner": "aws-lc-sys",
                  "source_checksum": "f1622d8446a2d4b2ce0c7eefc73dd43a99779028d5ee5c2dd8073a658ba8a2bc",
                  "source_files": 2044,
                  "static_archive_sha256": [
                    "0ff7c176f9eb551eb0b49889d9a1edd0def5fdcb87f4fde15289fa71a3320abf"
                  ]
                }
              ]
            },
            {
              "profile": "caller-union",
              "checks": 6,
              "recorded_executable_sha256": "5ca373df0b3bb68856244ec411c232f1010b0e565010e9dcbc1294d0907b4bb2",
              "native": [
                {
                  "owner": "aws-lc-sys",
                  "source_checksum": "f1622d8446a2d4b2ce0c7eefc73dd43a99779028d5ee5c2dd8073a658ba8a2bc",
                  "source_files": 2044,
                  "static_archive_sha256": [
                    "0ff7c176f9eb551eb0b49889d9a1edd0def5fdcb87f4fde15289fa71a3320abf"
                  ]
                }
              ]
            }
          ],
          "supplementary_executable_retention": "Only final profile original binary retained; earlier profile hashes/reports/Cargo/native logs retained. Post-family ordinary/default-labelled copies contain final binary, not earlier originals."
        }
      },
      "full_sdk": [
        {
          "profile": "ordinary",
          "executed": 385,
          "failed": 0,
          "ignored": 0,
          "retained_executables": 6,
          "graph_sha256": "c482c5c668013759e0f1c7c511f277c57f98002fd3cc18a998eaf4ada2087970"
        },
        {
          "profile": "default",
          "executed": 385,
          "failed": 0,
          "ignored": 0,
          "retained_executables": 6,
          "graph_sha256": "e561ab949b02a259b9dd0a721f0743f6c2ddfae586d018c8d308ad9cbf17a186"
        },
        {
          "profile": "c-zlib",
          "executed": 385,
          "failed": 0,
          "ignored": 0,
          "retained_executables": 6,
          "graph_sha256": "ac98a098d6d685358c3594ca23ad0ca67b0ea36ff0e7a1319c258f16bcdf2fb5"
        }
      ],
      "schema": {
        "package": {
          "sha256": "0110e99980f002998edf81aa76fba30a4f0a402b864b4ff9a8d88e7344061a22",
          "members": 250
        },
        "profiles": 2,
        "checks_per_profile": 73
      }
    },
    "linux-msrv": {
      "families": {
        "ws": {
          "package": {
            "sha256": "0110e99980f002998edf81aa76fba30a4f0a402b864b4ff9a8d88e7344061a22",
            "members": 250
          },
          "profiles": [
            {
              "profile": "ordinary",
              "checks": 10,
              "recorded_executable_sha256": "1e5d2f4aba6a72f3642ee763fd49f05a7b1e891dbef38cae802dc93a3fc1a691",
              "native": [
                {
                  "owner": "aws-lc-sys",
                  "source_checksum": "f1622d8446a2d4b2ce0c7eefc73dd43a99779028d5ee5c2dd8073a658ba8a2bc",
                  "source_files": 2044,
                  "static_archive_sha256": [
                    "7afdc2e308a8b576c346999b2cbcc6fd0038cd4cd755c470fac5eb66b5026655"
                  ]
                }
              ]
            },
            {
              "profile": "union-default",
              "checks": 10,
              "recorded_executable_sha256": "597af121416531cb4bd95288faf5c466d2f9cd8f9d7b6d24bcdc429617459852",
              "native": [
                {
                  "owner": "aws-lc-sys",
                  "source_checksum": "f1622d8446a2d4b2ce0c7eefc73dd43a99779028d5ee5c2dd8073a658ba8a2bc",
                  "source_files": 2044,
                  "static_archive_sha256": [
                    "7afdc2e308a8b576c346999b2cbcc6fd0038cd4cd755c470fac5eb66b5026655"
                  ]
                }
              ]
            },
            {
              "profile": "union-c-zlib",
              "checks": 10,
              "recorded_executable_sha256": "fcd46b59348f2cd14160dcf532e6bf8c73937e1b46a088e20ec1cb12b1d363df",
              "native": [
                {
                  "owner": "aws-lc-sys",
                  "source_checksum": "f1622d8446a2d4b2ce0c7eefc73dd43a99779028d5ee5c2dd8073a658ba8a2bc",
                  "source_files": 2044,
                  "static_archive_sha256": [
                    "7afdc2e308a8b576c346999b2cbcc6fd0038cd4cd755c470fac5eb66b5026655"
                  ]
                },
                {
                  "owner": "libz-sys",
                  "source_checksum": "f710a23e6dbf193214fd46ca56a9d6864e550abe86202184532ae7275e46de19",
                  "source_files": 238,
                  "static_archive_sha256": [
                    "9f5ebcac7714c1db94cb5ade328afd1c54c6505ab764aa87061a45d7106a9352"
                  ]
                }
              ]
            }
          ],
          "supplementary_executable_retention": "Only final profile original binary retained; earlier profile hashes/reports/Cargo/native logs retained. Post-family ordinary/default-labelled copies contain final binary, not earlier originals."
        },
        "http": {
          "package": {
            "sha256": "0110e99980f002998edf81aa76fba30a4f0a402b864b4ff9a8d88e7344061a22",
            "members": 250
          },
          "profiles": [
            {
              "profile": "ordinary",
              "checks": 6,
              "recorded_executable_sha256": "88fd9643df8c540b3ca15a960e05e7ca297e3400391dadf8c6f8f8bc3df79128",
              "native": [
                {
                  "owner": "aws-lc-sys",
                  "source_checksum": "f1622d8446a2d4b2ce0c7eefc73dd43a99779028d5ee5c2dd8073a658ba8a2bc",
                  "source_files": 2044,
                  "static_archive_sha256": [
                    "7afdc2e308a8b576c346999b2cbcc6fd0038cd4cd755c470fac5eb66b5026655"
                  ]
                }
              ]
            },
            {
              "profile": "caller-union",
              "checks": 6,
              "recorded_executable_sha256": "f119a10269faa8e12f8df1669adccf0679b4a4d31dc0f1db50e0a40d84b99926",
              "native": [
                {
                  "owner": "aws-lc-sys",
                  "source_checksum": "f1622d8446a2d4b2ce0c7eefc73dd43a99779028d5ee5c2dd8073a658ba8a2bc",
                  "source_files": 2044,
                  "static_archive_sha256": [
                    "7afdc2e308a8b576c346999b2cbcc6fd0038cd4cd755c470fac5eb66b5026655"
                  ]
                }
              ]
            }
          ],
          "supplementary_executable_retention": "Only final profile original binary retained; earlier profile hashes/reports/Cargo/native logs retained. Post-family ordinary/default-labelled copies contain final binary, not earlier originals."
        },
        "crypto": {
          "package": {
            "sha256": "0110e99980f002998edf81aa76fba30a4f0a402b864b4ff9a8d88e7344061a22",
            "members": 250
          },
          "profiles": [
            {
              "profile": "ordinary",
              "checks": 6,
              "recorded_executable_sha256": "747fd255259218cc082dcc44c6504e383672dad1709850df1c85db3b5f3295b2",
              "native": [
                {
                  "owner": "aws-lc-sys",
                  "source_checksum": "f1622d8446a2d4b2ce0c7eefc73dd43a99779028d5ee5c2dd8073a658ba8a2bc",
                  "source_files": 2044,
                  "static_archive_sha256": [
                    "7afdc2e308a8b576c346999b2cbcc6fd0038cd4cd755c470fac5eb66b5026655"
                  ]
                }
              ]
            },
            {
              "profile": "caller-union",
              "checks": 6,
              "recorded_executable_sha256": "d401f4c6de8481730ef87fa2b4a25b730851897f6b8dab1a8ba71fc58b810354",
              "native": [
                {
                  "owner": "aws-lc-sys",
                  "source_checksum": "f1622d8446a2d4b2ce0c7eefc73dd43a99779028d5ee5c2dd8073a658ba8a2bc",
                  "source_files": 2044,
                  "static_archive_sha256": [
                    "7afdc2e308a8b576c346999b2cbcc6fd0038cd4cd755c470fac5eb66b5026655"
                  ]
                }
              ]
            }
          ],
          "supplementary_executable_retention": "Only final profile original binary retained; earlier profile hashes/reports/Cargo/native logs retained. Post-family ordinary/default-labelled copies contain final binary, not earlier originals."
        }
      },
      "full_sdk": [
        {
          "profile": "ordinary",
          "executed": 385,
          "failed": 0,
          "ignored": 0,
          "retained_executables": 6,
          "graph_sha256": "c482c5c668013759e0f1c7c511f277c57f98002fd3cc18a998eaf4ada2087970"
        },
        {
          "profile": "default",
          "executed": 385,
          "failed": 0,
          "ignored": 0,
          "retained_executables": 6,
          "graph_sha256": "e561ab949b02a259b9dd0a721f0743f6c2ddfae586d018c8d308ad9cbf17a186"
        },
        {
          "profile": "c-zlib",
          "executed": 385,
          "failed": 0,
          "ignored": 0,
          "retained_executables": 6,
          "graph_sha256": "ac98a098d6d685358c3594ca23ad0ca67b0ea36ff0e7a1319c258f16bcdf2fb5"
        }
      ],
      "schema": {
        "package": {
          "sha256": "0110e99980f002998edf81aa76fba30a4f0a402b864b4ff9a8d88e7344061a22",
          "members": 250
        },
        "profiles": 2,
        "checks_per_profile": 73
      }
    }
  },
  "limitations": [
    "Supplementary family original ordinary/default executables were overwritten between profiles. Per-profile run/graph/native logs and executable hashes are original; driver copies made after family completion contain final profile binary even when labelled ordinary/default. Only final family binary is authenticated as retained.",
    "All six original full-SDK test executables per each of three graphs per host are retained and independently hashed. Native source archives and static-link outputs/controlled inputs are checked; external-host rlib/rmeta/fingerprint file bytes are not asserted retained.",
    "This closes current proof availability/applicability for finding020 only. Known runtime/code/test defects still require approved repair and matching green; changed inputs invalidate affected native/package evidence. No Phase verdict or ledger/hash update is implied.",
    "Pinned local Pi1.3 source/pin/helper association; no upstream tag/npm authentication or new network acquisition claimed."
  ],
  "inventory": {
    "path": "C:\\Users\\Luiz\\AppData\\Local\\Temp\\opi-phase20-ready-w8p6zu1s\\ready-final-proof.inventory.json",
    "sha256": "f64daeb00e2d8301b643273e09a4e68f80e4f4675c2f7f3d4e4055cf43c17799",
    "files": 5513
  }
}
```

### Material record: ready-catalog-material.json

SHA-256 `6cc722b7f43601236dee538ed219e06a101d86173bd82eb9f10a50290e951528`

```json
{
  "head": "8010ed2af97cd5deeb1f7240097436fdf32b8d54",
  "inputs": {
    "nix/model-catalog.json": "2cc2eae4302522ad28c3dbca63d9754bfcbb50e779bde331ffb2b8f2b535d4a3",
    "packages/ai/scripts/model-data.ts": "f2605a61ad57d8a3edb93e5c36c0f543d051796b8353452a1fd584eec33f02f4",
    "packages/ai/scripts/hydrate-model-catalog.ts": "153433c81547149bf04b7a913c6c72a45666035b53233cd6b6e985248e5a9492",
    "packages/ai/src/models.generated.ts": "8cd91fa7eae8e9b219b8f50ed7581d7c31fe6df29b5a07a0a672589bb7323c21",
    "packages/ai/src/model-catalog.ts": "567b559600439eac5e6c341658bb45b149c5e8a22e34624a54ad5a06f5a92885",
    "packages/ai/src/providers/compat-schema.ts": "2df3f174686bc945844cdee99e063eafe8ff675d236b032b06fe370754a6c11a",
    "packages/ai/src/api/openai-completions.ts": "bddb09f395df6d5d29fbab4dd8039f89dea0c58f892dc4c15c73be74fe6e0b7d",
    "packages/ai/src/api/openai-responses.ts": "63e0662468c5df8f2165d105fbe72625cfdf36fdc0960e48fc2c7f512aaa0386",
    "packages/ai/src/api/openai-responses-shared.ts": "f940a1539a3ca03fbee79b2c848999e0b51c74f7cceb45484d3d52bc2303de3c",
    "packages/ai/src/api/azure-openai-responses.ts": "91fefcb7ff00d3db7a91180667d913b4f767a64fcc55ce8993493291f769f600",
    "packages/ai/src/api/openai-codex-responses.ts": "41a0095ce395f6ddf50ce3adbe7d03960bf183fb0f2f94412c88923adaa013b0",
    "packages/ai/src/types.ts": "258627828297eb0165de9828cab62666265a77e932dea5dba1542ace9c47c7d5",
    "packages/ai/src/models.ts": "c10385291e4e79dc3ea5301629288a35cac354f8a3635c07f9312608c269f573"
  },
  "checked_catalog": {
    "artifact": {
      "file": "catalog.json",
      "kind": "pi-published-model-catalog",
      "revision": "sha256-ffa3e98626230f378f80200debb736c4a2238b5f2ba246a92cfc7d8e19604678",
      "sha256": "ffa3e98626230f378f80200debb736c4a2238b5f2ba246a92cfc7d8e19604678",
      "url": "https://pi.dev/api/models/revisions/sha256-ffa3e98626230f378f80200debb736c4a2238b5f2ba246a92cfc7d8e19604678?types=chat,image,classifier"
    },
    "bundle_schema_version": 1,
    "counts": {
      "chat": 1608,
      "classifier": 38,
      "image": 59,
      "providers": 42
    },
    "data_schema_version": 6,
    "generator": {
      "packer": "scripts/pi-catalog/pack.py",
      "packer_sha256": "40fd9998d7b9f44ce495a6727b3b69b7b159bbd4c4c145189e8e6c2576062f66",
      "pi_hydrate_model_catalog_ts_sha256": "153433c81547149bf04b7a913c6c72a45666035b53233cd6b6e985248e5a9492",
      "pi_model_data_ts_sha256": "f2605a61ad57d8a3edb93e5c36c0f543d051796b8353452a1fd584eec33f02f4"
    },
    "min_reader": {
      "bundle_schema_version": 1
    },
    "providers": [
      "amazon-bedrock",
      "ant-ling",
      "anthropic",
      "azure",
      "baseten",
      "cerebras",
      "cloudflare-ai-gateway",
      "cloudflare-workers-ai",
      "deepseek",
      "fireworks",
      "github-copilot",
      "google",
      "google-vertex",
      "groq",
      "huggingface",
      "kimi-coding",
      "meta",
      "minimax",
      "minimax-cn",
      "mistral",
      "moonshotai",
      "moonshotai-cn",
      "nvidia",
      "openai",
      "openai-codex",
      "opencode",
      "opencode-go",
      "openrouter",
      "qwen-token-plan",
      "qwen-token-plan-cn",
      "qwen-token-plan-individual",
      "radius",
      "together",
      "typesafe",
      "vercel-ai-gateway",
      "xai",
      "xiaomi",
      "xiaomi-token-plan-ams",
      "xiaomi-token-plan-cn",
      "xiaomi-token-plan-sgp",
      "zai",
      "zai-coding-cn"
    ],
    "source": {
      "local_origin": ".repo/pi-1.3.0",
      "pi_package": "@earendil-works/pi-ai@1.3.0",
      "revision_pin": ".repo/pi-1.3.0/nix/model-catalog.json",
      "tree_evidence": "docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-source-evidence.json"
    },
    "structure_sha256": "0269b76c00ffa005620a06979655e04f46c6fb9ab34b1b6f90a9323947eb0cff"
  },
  "source_limitation": "Original local pinned source material sealed; no claim of upstream tag/npm archive identity or new network acquisition."
}
```

## Reproducible current evidence

Save the named source blocks together. Runner rebuilds the explicit source root with offline locked Cargo, resolves that build artifacts and relinks the same consumer predicates. No stale statically linked baseline executable is green. Deep probes use iterative Map/Array constructors; only baseline normalization isolation deliberately forgets its owner, while closure requires separate normal last-owner drop and compile-error cleanup.

### Baseline current-session source/record: replay_probe.py

SHA-256 `8c8fc70b6ff97d12e88953f2cedd08be5f3b0d981728bb5e87bb5a3d67708314`

```python
import argparse,hashlib,json,os,pathlib,re,subprocess,sys,tempfile

def main():
 p=argparse.ArgumentParser();p.add_argument('--source-root',required=True);p.add_argument('--target-dir',required=True);p.add_argument('--suite',choices=['auth','event','runtime','delivery','ready-runtime','telemetry'],required=True);select=p.add_mutually_exclusive_group();select.add_argument('--case');select.add_argument('--filter');a=p.parse_args()
 root=pathlib.Path(__file__).resolve().parent;source=pathlib.Path(a.source_root).resolve();out=pathlib.Path(tempfile.mkdtemp(prefix=a.suite+'-replay-',dir=root));env=dict(os.environ,CARGO_TARGET_DIR=a.target_dir,CARGO_INCREMENTAL='1')
 build=['cargo','build','--offline','--locked','-p','opi-ai','--message-format=json'];r=subprocess.run(build,cwd=source,env=env,capture_output=True,timeout=600);(out/'build.jsonl').write_bytes(r.stdout);(out/'build.stderr.log').write_bytes(r.stderr)
 if r.returncode: print(r.stderr.decode(errors='replace'));return r.returncode
 rows=[json.loads(s) for s in r.stdout.decode('utf-8').splitlines() if s.strip()];artifacts={x['target']['name']:x for x in rows if x.get('reason')=='compiler-artifact'};rlib=next(x for x in artifacts['opi_ai']['filenames'] if x.endswith('.rlib'))
 names={'auth':'auth-schema-public-probe.rs','event':'event-prelude-probe.rs','runtime':'runtime_public_probe.rs','delivery':'delivery_public_probe.rs','ready-runtime':'ready_runtime_probe.rs','telemetry':'ready_telemetry_probe.rs'};probe=root/names[a.suite]
 migrated=[]
 # Only synchronize known public0.x constructor additions; retain every
 # actual consumer assertion and its positive/negative controls unchanged.
 provider=(source/'crates/opi-ai/src/provider.rs').read_text(encoding='utf-8')
 if re.search(r'pub\s+projection\s*:',provider):
  for name in ['auth-schema-public-probe.rs','runtime_public_probe.rs','ready_runtime_probe.rs','ready_telemetry_probe.rs','delivery_public_probe.rs']:
   text=(root/name).read_text(encoding='utf-8')
   text,n=re.subn(r'\bRequest\s*\{\s*(?=(?:transport|model)\s*:)',lambda m:m[0]+'projection: Default::default(),',text)
   text,m=re.subn(r'\bAssistantMessage\s*\{\s*(?=content\s*:)',lambda x:x[0]+'projection: None,',text)
   (out/name).write_text(text,encoding='utf-8');migrated.append({'source':name,'request_defaults':n,'synthesized_message_unknowns':m})
  if a.suite!='event':probe=out/names[a.suite]
 if a.suite=='event':
  content=probe.read_text(encoding='utf-8');target=source/'crates/opi-ai/src/bedrock/event_stream.rs';content=re.sub(r'^#\[path = r"[^"\n]+"\]',lambda _: '#[path = r"'+str(target)+'"]',content,count=1);probe=out/'event.rs';probe.write_text(content,encoding='utf-8')
 command=['rustc','--edition=2024',str(probe),'-L','dependency='+str(pathlib.Path(a.target_dir)/'debug/deps')]
 testing=a.suite in ['runtime','delivery','ready-runtime','telemetry']
 if testing:command.append('--test')
 deps={'auth':['opi_ai','tokio','serde_json','time'],'event':[],'runtime':['opi_ai','tokio','tokio_util','serde_json','futures_util','secrecy'],'delivery':['opi_ai'],'ready-runtime':['opi_ai','tokio','tokio_util','serde_json','futures_util','secrecy'],'telemetry':['opi_ai','futures_core','futures_util','tracing','tracing_core','secrecy']}[a.suite]
 for name in deps:command+=['--extern',name+'='+next(x for x in artifacts[name]['filenames'] if x.endswith('.rlib'))]
 for x in rows:
  if x.get('reason')=='build-script-executed':
   for path in x.get('linked_paths',[]):command+=['-L',path]
 binary=out/('probe.exe' if os.name=='nt' else 'probe');command+=['-o',str(binary)];c=subprocess.run(command,capture_output=True,timeout=120);(out/'compile.log').write_bytes(c.stdout+c.stderr)
 if c.returncode:print((c.stdout+c.stderr).decode(errors='replace'));return c.returncode
 run=[str(binary)]
 if a.suite=='auth':run += [a.case,str(out)]
 elif testing:
  if a.case:run+=['--exact',a.case]
  if a.filter:run+=[a.filter]
  run+=['--nocapture','--test-threads=1']
 r=subprocess.run(run,capture_output=True,timeout=45);log=r.stdout+r.stderr;(out/'run.log').write_bytes(log)
 text=log.decode('utf-8',errors='replace');executed=None
 if testing:
  counts=re.findall(r'test result: (?:ok|FAILED)\. (\d+) passed; (\d+) failed;',text)
  executed=sum(int(a)+int(b) for a,b in counts)
 record={'source_root':str(source),'build':build,'compile':command,'run':run,'exit':r.returncode,'executed':executed,'constructor_migration':migrated,'probe_sha256':hashlib.sha256(probe.read_bytes()).hexdigest(),'rlib_sha256':hashlib.sha256(pathlib.Path(rlib).read_bytes()).hexdigest(),'log_sha256':hashlib.sha256(log).hexdigest()};(out/'record.json').write_text(json.dumps(record,indent=2),encoding='utf-8');print('EVIDENCE='+str(out));print(text)
 if testing and not executed:print('REFUSED: zero executed tests cannot prove the predicate');return 2
 return r.returncode
if __name__=='__main__':raise SystemExit(main())
```

### Baseline current-session source/record: auth-schema-public-probe.rs

SHA-256 `edc01899d957143785ac8c07b32a6d315c342244b943f5377c312ab6d34092e1`

```rust
use std::sync::{Arc, Mutex, atomic::{AtomicUsize, Ordering}};
use opi_ai::auth::{AuthContext,AuthContextError,AuthResolver,ResolvedAuth,AuthProvenanceSource};
use opi_ai::credential::BoxAuthFuture;
use opi_ai::provider::ProviderError;
use opi_ai::auth::adc::native::*;
use opi_ai::auth::aws::source_http::*;
use serde_json::{json,Value};
struct NoAmbient;
impl AuthContext for NoAmbient {fn env(&self,_:&str)->Result<Option<String>,AuthContextError>{Ok(None)} fn file_exists(&self,_:&str)->Result<bool,AuthContextError>{Ok(false)}}
struct PendingAuth(AtomicUsize);
impl AuthResolver for PendingAuth {fn resolve(&self)->BoxAuthFuture<'_,Result<ResolvedAuth,ProviderError>>{self.0.fetch_add(1,Ordering::SeqCst);Box::pin(std::future::pending())}}
struct AdcTransport { body:Vec<u8>, requests:Mutex<Vec<AdcSourceRequest>> }
impl AdcSourceTransport for AdcTransport {fn send(&self,r:AdcSourceRequest)->BoxAuthFuture<'_,Result<AdcSourceResponse,AdcSourceHttpError>> {self.requests.lock().unwrap().push(r); let body=self.body.clone();Box::pin(async move {Ok(AdcSourceResponse{status:200,headers:vec![("Metadata-Flavor".into(),"Google".into())],body})})}}
struct AwsTransport(Mutex<Vec<SourceHttpRequest>>);
impl AwsSourceTransport for AwsTransport {fn send(&self,r:SourceHttpRequest)->BoxAuthFuture<'_,Result<SourceHttpResponse,SourceHttpError>>{self.0.lock().unwrap().push(r);Box::pin(async {Ok(SourceHttpResponse{status:200,body:br#"{"AccessKeyId":"SYNTHETIC","SecretAccessKey":"SYNTHETIC","Token":"SYNTHETIC"}"#.to_vec()})})}}
fn rt()->tokio::runtime::Runtime{tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap()}
fn overflow(){
 let mut m=opi_ai::message::AssistantMessage{content:vec![],api:opi_ai::ApiKind::Anthropic,provider:"test".into(),model:"test".into(),response_model:None,wire:None,response_id:None,usage:opi_ai::stream::Usage::unknown(),stop_reason:opi_ai::stream::StopReason::Stop,raw_stop_reason:None,error_message:None,timestamp_ms:0,diagnostics:vec![],duration_ms:None,provider_thinking_level:None,deferred:None};m.stop_reason=opi_ai::stream::StopReason::Length;m.usage=opi_ai::stream::Usage{input_tokens:1,reported:true,..opi_ai::stream::Usage::unknown()};
 assert!(!opi_ai::message::is_context_overflow(&m,Some(100_000)));println!("CONTROL ordinary-window=false");
 let result=std::panic::catch_unwind(||opi_ai::message::is_context_overflow(&m,Some(186_330_748_219_288_401)));println!("OBSERVED extreme-window={result:?}");assert_eq!(result.unwrap(),false);
}
fn deferred(){rt().block_on(async{
 let backend=Arc::new(opi_ai::test_support::ScriptedDeferredBackend::new("test","mock-model"));let auth=Arc::new(PendingAuth(AtomicUsize::new(0)));let mut models=opi_ai::Models::new();
 models.register_deferred_api(backend.clone() as Arc<dyn opi_ai::deferred::DeferredApi<()>>,auth.clone(),AuthProvenanceSource::Static).unwrap();
 let mut h=opi_ai::deferred::DeferredHandle{provider:"wrong".into(),model_id:"mock-model".into(),api:"scripted-deferred".into(),id:"x".into(),expires_at_ms:None,poll_after_ms:None,data:None};
 assert!(models.cancel_deferred(&h,Default::default()).await.is_err());assert_eq!(auth.0.load(Ordering::SeqCst),0);println!("CONTROL wrong-route rejects before auth");
 h.provider="test".into();let call=opi_ai::deferred::DeferredCancelCall::default();call.cancel.cancel();
 let r=tokio::time::timeout(std::time::Duration::from_millis(100),models.cancel_deferred(&h,call)).await;println!("OBSERVED preaborted settled={} auth_started={} backend_cancels={}",r.is_ok(),auth.0.load(Ordering::SeqCst),backend.cancelled_handles().len());assert!(r.is_ok(),"preaborted caller must settle while auth pending");assert_eq!(auth.0.load(Ordering::SeqCst),0);
})}
fn adc(){rt().block_on(async{
 let clock=FixedAdcClock::new(time::OffsetDateTime::from_unix_timestamp(1_700_000_000).unwrap());
 let control=AdcTransport{body:br#"{"access_token":"SYNTHETIC","expires_in":3600}"#.to_vec(),requests:Mutex::new(vec![])};
 assert!(opi_ai::auth::adc::metadata::resolve_direct_metadata_token(&NoAmbient,&control,&clock,&[],Some(false)).await.is_ok());println!("CONTROL expiry=3600 succeeds sends={}",control.requests.lock().unwrap().len());
 let bad=AdcTransport{body:br#"{"access_token":"SYNTHETIC","expires_in":9223372036854775807}"#.to_vec(),requests:Mutex::new(vec![])};
 let r=opi_ai::auth::adc::metadata::resolve_direct_metadata_token(&NoAmbient,&bad,&clock,&[],Some(false)).await;println!("OBSERVED i64MAX response={r:?}");assert!(r.is_err(),"unrepresentable expiry must return typed failure");
})}
fn ecs(){rt().block_on(async{
 let mut violations=0; for (uri,expected) in [("http://127.0.0.1/creds",1),("http://credentials.invalid/creds",0),("http://127.attacker.invalid/creds",0),("http://169.254.attacker.invalid/creds",0)]{
 let trans=Arc::new(AwsTransport(Mutex::new(vec![])));let ambient:Arc<dyn AuthContext>=Arc::new(NoAmbient);let mut models=opi_ai::Models::new();models.set_auth_context(ambient.clone());let mut o=opi_ai::provider_collection::BuiltinOverrides::default();o.aws_auth=opi_ai::auth::builtin_aws::BuiltinAwsConfig{region:Some("us-east-1".into()),transport:Some(trans.clone()),scoped:opi_ai::auth::aws::local::ScopedAwsEnv{container_credentials_full_uri:Some(uri.into()),container_authorization_token:Some("SYNTHETIC".into()),..Default::default()},..Default::default()};
 opi_ai::provider_collection::register_builtin(&mut models,ambient,Arc::new(opi_ai::http::HttpClient::new()),&o).unwrap();let result=models.get_auth("amazon-bedrock",Default::default()).await;let req=trans.0.lock().unwrap();let sends=req.len();let auth=req.iter().filter(|r|r.headers.iter().any(|(k,_)|k.eq_ignore_ascii_case("authorization"))).count();println!("OBSERVED {uri} ok={} sends={sends} authorization_sends={auth} expected={expected}",result.is_ok());
 if sends!=expected {violations+=1;}
 }
 assert_eq!(violations,0,"DNS prefix bypassed cleartext host guard");
})}
fn deep(mode:String){std::thread::Builder::new().stack_size(2*1024*1024).spawn(move||{
 let mut s=json!({"type":"integer"});for _ in 0..if mode=="deep-shallow"{32}else{10_000}{let mut obj=serde_json::Map::new();obj.insert("allOf".into(),Value::Array(vec![s]));s=Value::Object(obj);}if mode=="deep-id"{s.as_object_mut().unwrap().insert("$id".into(),Value::String("https://example.invalid/root".into()));}
 eprintln!("START {mode}");let compiled=opi_ai::schema::compile(s).unwrap();eprintln!("COMPILE ok");assert!(compiled.check(&json!(1)).unwrap());eprintln!("CHECK true");
 if mode=="deep-normalize"{let r=opi_ai::schema::validate_tool_arguments(&compiled,&json!(1),None).unwrap();eprintln!("NORMALIZE {r:?}");std::mem::forget(compiled);}else{drop(compiled);eprintln!("DROP ok");}
 }).unwrap().join().unwrap();}
fn aws1(){rt().block_on(async{
 let dir=std::env::args().nth(2).unwrap();let path=std::path::PathBuf::from(dir).join("aws1-input.json");let f=json!({"type":"external_account","audience":"//iam.googleapis.com/projects/1/locations/global/workloadIdentityPools/p/providers/a","subject_token_type":"urn:ietf:params:aws:token-type:aws4_request","token_url":"https://sts.googleapis.com/v1/token","credential_source":{"environment_id":"aws1","region_url":"http://metadata.invalid/region","url":"http://metadata.invalid/credentials","regional_cred_verification_url":"https://sts.{region}.amazonaws.com?Action=GetCallerIdentity&Version=2011-06-15"}});std::fs::write(&path,serde_json::to_vec(&f).unwrap()).unwrap();
 let scoped=ScopedAdcEnv::default();let clock=FixedAdcClock::new(time::OffsetDateTime::from_unix_timestamp(1_700_000_000).unwrap());let region=if std::env::args().nth(1).as_deref()==Some("aws1-ascii"){"us-east-1a"}else{"us-east-1界"};let transport=AdcTransport{body:region.as_bytes().to_vec(),requests:Mutex::new(vec![])};let input=AdcNativeInput::new(AdcAuthSelection::CredentialsFile(path),&scoped,&NoAmbient,&transport,&clock);let r=resolve_adc_native(&input).await;println!("AWS1 returned={r:?}");assert!(r.is_err());
})}
fn main(){let mode=std::env::args().nth(1).unwrap();match mode.as_str(){"overflow"=>overflow(),"deferred"=>deferred(),"adc"=>adc(),"ecs"=>ecs(),"aws1"|"aws1-ascii"=>aws1(),_=>deep(mode)}}
```

### Baseline current-session source/record: event-prelude-probe.rs

SHA-256 `eb9fcdecf0576e98d34f499cc95414fd78f82052a535e737ffe1dee4023a9c0b`

```rust
#[path = r"C:\Users\Luiz\AppData\Local\Temp\opi-phase20-remediate-hvrszrbs\source\crates\opi-ai\src\bedrock\event_stream.rs"]
mod event_stream;
fn crc32(bytes:&[u8])->u32 { let mut c=0xffff_ffffu32; for b in bytes {c^=*b as u32; for _ in 0..8 { c=if c&1==1 {(c>>1)^0xedb8_8320}else{c>>1}; }} !c }
fn main(){
 let frame=event_stream::build_test_frame("messageStart","application/json",b"{}");
 let mut valid=frame[..16].to_vec(); assert!(event_stream::parse_frames(&mut valid).unwrap().is_empty()); println!("CONTROL valid_incomplete retained={}",valid.len());
 let mut bad_crc=Vec::from(u32::MAX.to_be_bytes());bad_crc.extend_from_slice(&0u32.to_be_bytes());bad_crc.extend_from_slice(&0u32.to_be_bytes());bad_crc.extend_from_slice(&[0;4]);
 let crc_result=event_stream::parse_frames(&mut bad_crc); println!("OBSERVED bad_crc_u32max result={:?} retained={}",crc_result,bad_crc.len());
 let mut bad_headers=Vec::from(32u32.to_be_bytes());bad_headers.extend_from_slice(&17u32.to_be_bytes());let crc=crc32(&bad_headers);bad_headers.extend_from_slice(&crc.to_be_bytes());bad_headers.extend_from_slice(&[0;4]);
 let header_result=event_stream::parse_frames(&mut bad_headers);println!("OBSERVED impossible_headers result={:?} retained={}",header_result,bad_headers.len());
 assert!(crc_result.is_err(),"known-invalid prelude CRC must fail before declared payload");
 assert!(header_result.is_err(),"known-impossible headers relation must fail before declared payload");
}
```

### Baseline current-session source/record: runtime_public_probe.rs

SHA-256 `694e4f520f81a1298d4ac5c0d4b5e21027d340367d6d24bb9fcacd56646c4472`

```rust
use std::{sync::Arc, time::Duration};
use opi_ai::{Models, ModelInfo, WireApi};
use opi_ai::models::{ApiId, ChatApi};
use opi_ai::provider::{Request, EventStream, Provider, CacheRetention, ThinkingConfig};
use opi_ai::auth::{AuthProvenanceSource, AuthScheme, StaticAuthResolver, ResolvedAuth};
use opi_ai::message::{Message, UserMessage, InputContent, AssistantMessage, AssistantContent};
use opi_ai::stream::{Usage,StopReason};
use tokio_util::sync::CancellationToken;
use serde_json::json;

fn runtime() -> tokio::runtime::Runtime { tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap() }
fn request() -> Request {
 Request { transport:Default::default(),model:"openai:gpt-4o".into(), context:opi_ai::transcript::normalize_context(opi_ai::transcript::Context {system_prompt:None,messages:vec![Message::User(UserMessage {content:vec![InputContent::Text{text:"Hello".into()}],timestamp_ms:0})],tools:vec![]}).unwrap(), max_tokens:None,temperature:None,sampling_params:Default::default(),thinking:ThinkingConfig::default(),stop_sequences:vec![],metadata:None,cancel:CancellationToken::new(),timeout:Some(Duration::from_secs(5)),extra_headers:vec![],cache_retention:CacheRetention::None,session_id:None,telemetry_context:None,callbacks:Default::default() }
}
fn assistant(total:Option<u64>, reported:bool) -> Message {
 let mut usage=Usage::reported(500,20,100,50,Some(10),Some(15)); usage.total_tokens=total;usage.reported=reported;
 Message::Assistant(AssistantMessage {content:vec![AssistantContent::Text{text:"done".into(),signature:None}],api:opi_ai::ApiKind::OpenAi,provider:"test".into(),model:"model".into(),response_model:None,wire:None,response_id:None,usage,stop_reason:StopReason::Stop,error_message:None,timestamp_ms:2,diagnostics:vec![],duration_ms:None,raw_stop_reason:None,provider_thinking_level:None,deferred:None})
}
#[test] fn known_total_without_known_parents_is_anchor() {
 let control=opi_ai::context::estimate_context_tokens(None,&[assistant(Some(999),true)],&[]);assert_eq!(control.usage_tokens,999);
 let unknown=opi_ai::context::estimate_context_tokens(None,&[assistant(None,false)],&[]);assert_eq!(unknown.usage_tokens,0);
 let estimate=opi_ai::context::estimate_context_tokens(None,&[assistant(Some(999),false)],&[]);
 eprintln!("independent_total: usage_tokens={}, tokens={}",estimate.usage_tokens,estimate.tokens);
 assert_eq!(estimate.usage_tokens,999,"positive independently known total is authoritative without known parent buckets");
}
struct DummyApi(ApiId);
impl ChatApi<()> for DummyApi {
 fn api_id(&self)->&ApiId {&self.0} fn provider_id(&self)->&str {"probe"}
 fn complete_record(&self)->ModelInfo {ModelInfo::new("model","model",WireApi::OpenAiCompletions,opi_ai::registry::ModelCapabilities::new(4096,256))}
 fn simple_options(&self) {} fn stream_prepared(&self,_:Request,_:(),_:ResolvedAuth)->EventStream {Box::pin(futures_util::stream::empty())}
}
#[test] fn exact_empty_and_whitespace_api_id_register() {
 let mut outcomes=Vec::new();
 for id in ["normal","", " "] {
  let mut models=Models::new(); let result=models.register_chat_api(Arc::new(DummyApi(ApiId::new(id))),Arc::new(StaticAuthResolver::new(AuthScheme::Bearer,secrecy::SecretString::from("synthetic"))),AuthProvenanceSource::Static);
  eprintln!("api_id={id:?}, result={:?}",result.as_ref().map(|h| h.api_id().as_str()));
  outcomes.push(result.is_ok());
 }
 assert!(outcomes.into_iter().all(|v|v),"ApiId must preserve arbitrary exact source strings including empty and whitespace");
}
#[test] fn wrong_type_required_is_strict_error() {
 use opi_ai::constrained_sampling::{make_strict_json_schema,NoRejectedKeywords};
 let valid=json!({"type":"object","properties":{"a":{"type":"string"}},"required":["a"]});assert!(make_strict_json_schema(&valid,&NoRejectedKeywords).is_ok());
 let invalid=json!({"type":"object","properties":{"a":{"type":"string"}},"required":[1]});assert!(make_strict_json_schema(&invalid,&NoRejectedKeywords).is_err());
 let mut outcomes=Vec::new();
 for malformed in [json!("a"),json!(1),json!({})] {
  let schema=json!({"type":"object","properties":{"a":{"type":"string"}},"required":malformed});
  let result=make_strict_json_schema(&schema,&NoRejectedKeywords);eprintln!("required={malformed}, result={result:?}");outcomes.push(result.is_err());
 }
 assert!(outcomes.into_iter().all(|v|v),"each present wrong-type required must fail strict shape admission");
}
#[test] fn response_compat_maximum_false_reaches_wire() {
 use opi_ai::catalog::{CatalogModel,CatalogOperation,CatalogInputModality};
 let raw=json!({"id":"gpt-5.6-sol","provider":"openai","name":"probe","api":"openai-responses","contextWindow":4096,"maxTokens":256,"input":["text"],"compat":{"supportsMaxOutputTokens":false,"supportsExplicitPromptCacheMode":true}});
 let model=CatalogModel{provider:"openai".into(),id:"gpt-5.6-sol".into(),name:"probe".into(),operation:CatalogOperation::Chat,api:"openai-responses".into(),base_url:String::new(),reasoning:false,input:vec![CatalogInputModality::Text],cost:None,context_window:Some(4096),max_tokens:Some(256),raw}.to_chat_model_info().unwrap();
 let provider=opi_ai::openai_responses::OpenAiResponsesProvider::for_route(None,"openai".into(),Default::default(),vec![model],Arc::new(opi_ai::http::HttpClient::new())).with_affinity_policy(opi_ai::openai_responses::ResponsesAffinityPolicy::Direct);
 let mut call=request();call.model="openai:gpt-5.6-sol".into();call.max_tokens=Some(32);call.cache_retention=CacheRetention::Long;call.session_id=Some("probe-session".into());
 let body=provider.build_request_body(&call).unwrap();eprintln!("max_output_tokens={}, cache_options={}, prompt_cache_retention={}",body["max_output_tokens"],body["cache_options"],body["prompt_cache_retention"]);
 assert!(body.get("max_output_tokens").is_none(),"source false must omit maximum output tokens");
}
async fn chat_on_peer(chunks:Vec<(Duration,String)>, header_delay:Duration)->Result<AssistantMessage,opi_ai::stream::CallFailure> {
 use tokio::io::{AsyncReadExt,AsyncWriteExt};
 let listener=tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();let address=listener.local_addr().unwrap();
 let task=tokio::spawn(async move {
  let (mut socket,_)=listener.accept().await.unwrap();let mut read=vec![0;65536];let _=socket.read(&mut read).await.unwrap();
  tokio::time::sleep(header_delay).await;socket.write_all(b"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\n").await.unwrap();
  for (delay,body) in chunks {tokio::time::sleep(delay).await;let chunk=format!("{:x}\r\n{}\r\n",body.len(),body);if socket.write_all(chunk.as_bytes()).await.is_err(){return;}socket.flush().await.unwrap();}
  let _=socket.write_all(b"0\r\n\r\n").await;
 });
 let provider=opi_ai::openai_chat::OpenAiChatProvider::new(Some(format!("http://{address}")));
 let result=opi_ai::stream::ChatStream::new(provider.stream_prepared(request(),opi_ai::test_support::resolved_bearer_auth())).result().await;let _=task.await;result
}
fn terminal() -> String {"data: {\"id\":\"probe\",\"model\":\"gpt-4o\",\"choices\":[{\"index\":0,\"delta\":{},\"finish_reason\":\"stop\"}]}\r\n\r\ndata: [DONE]\r\n\r\n".into()}
#[test] fn multiline_crlf_split_preserves_event() {runtime().block_on(async {
 let first="data: {\"id\":\"probe\",\"model\":\"gpt-4o\",\r";
 let second="\ndata: \"choices\":[{\"index\":0,\"delta\":{\"content\":\"hello\"},\"finish_reason\":null}]}\r\n\r\n";
 let control=chat_on_peer(vec![(Duration::ZERO,format!("{first}{second}{}",terminal()))],Duration::ZERO).await;assert!(control.is_ok(),"coalesced legal multiline control {control:?}");
 let split=chat_on_peer(vec![(Duration::ZERO,first.into()),(Duration::from_millis(80),format!("{second}{}",terminal()))],Duration::ZERO).await;eprintln!("split_result={split:?}");assert!(split.is_ok(),"split CRLF must preserve same legal SSE event");
});}
#[test] fn duration_includes_pending_first_body_poll() {runtime().block_on(async {
 let start=std::time::Instant::now();let result=chat_on_peer(vec![(Duration::from_millis(350),terminal())],Duration::ZERO).await.unwrap();let elapsed=start.elapsed();eprintln!("elapsed={elapsed:?}, duration_ms={:?}",result.duration_ms);assert!(result.duration_ms.unwrap()>=300,"first body Pending wait belongs to response stream interval");
});}
#[test] fn eleven_redirects_are_accepted_by_default_generation() {runtime().block_on(async {
 use tokio::io::{AsyncReadExt,AsyncWriteExt};use std::sync::atomic::{AtomicUsize,Ordering};
 let listener=tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();let address=listener.local_addr().unwrap();let sends=Arc::new(AtomicUsize::new(0));let seen=sends.clone();
 let task=tokio::spawn(async move {loop {let (mut socket,_)=listener.accept().await.unwrap();let mut buf=[0;65536];let _=socket.read(&mut buf).await.unwrap();let n=seen.fetch_add(1,Ordering::SeqCst);let body=if n<11 {format!("HTTP/1.1 307 Temporary Redirect\r\nLocation: /hop/{}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",n+1)} else {let s=terminal();format!("HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",s.len(),s)};let _=socket.write_all(body.as_bytes()).await;}});
 let provider=opi_ai::openai_chat::OpenAiChatProvider::new(Some(format!("http://{address}")));let result=opi_ai::stream::ChatStream::new(provider.stream_prepared(request(),opi_ai::test_support::resolved_bearer_auth())).result().await;task.abort();eprintln!("sends={}, result={result:?}",sends.load(Ordering::SeqCst));assert!(result.is_ok(),"source-qualified default accepts lawful 11-hop request");assert_eq!(sends.load(Ordering::SeqCst),12);
});}

#[test] fn uncapped_default_accepts_valid_large_credential_process() {runtime().block_on(async {
 use opi_ai::auth::aws::process::{CredentialProcessRunner,CredentialProcessSpec,SpawnedCredentialProcess,CredentialProcessPolicy};
 let small=r#"[Console]::Out.Write('{"Version":1,"AccessKeyId":"SYNTHETIC","SecretAccessKey":"synthetic"}')"#;
 let large=r#"[Console]::Out.Write('{"Version":1,"AccessKeyId":"SYNTHETIC","SecretAccessKey":"synthetic","Padding":"' + ('x' * 70000) + '"}')"#;
 let small=CredentialProcessSpec::inherit(small);let large=CredentialProcessSpec::inherit(large);
 assert!(SpawnedCredentialProcess::default().run(&small).await.is_ok(),"small valid credential control");
 assert!(SpawnedCredentialProcess::with_policy(CredentialProcessPolicy{timeout:Duration::from_secs(20),max_stdout:128*1024}).run(&large).await.is_ok(),"large valid credential parses with explicit larger policy control");
 let result=SpawnedCredentialProcess::default().run(&large).await;eprintln!("default large credential output accepted={}, error={:?}",result.is_ok(),result.as_ref().err());assert!(result.is_ok(),"omitted caller policy must not impose withdrawn 64KiB parity cap");
});}
```

### Baseline current-session source/record: delivery_public_probe.rs

SHA-256 `4752fb1927c7fd87b550808f5d24bc91a863dc943c423e65e11e2d6b73b13aa6`

```rust
use opi_ai::message::Message;
use opi_ai::transcript::{Context, SystemMessage, SystemSection, normalize_context};

fn system(names: &[&str]) -> SystemMessage {
    SystemMessage {
        sections: names.iter().enumerate().map(|(i, name)| SystemSection {
            name: (*name).to_owned(), value: Some(format!("value-{i}")),
        }).collect(),
        ..Default::default()
    }
}

#[test]
fn duplicate_sections_normalization_rejects_ambiguous_batch() {
    let control = normalize_context(Context { messages: vec![Message::System(system(&["a", "b"]))], ..Default::default() });
    assert!(control.is_ok(), "unique section names remain valid");
    let invalid = normalize_context(Context { messages: vec![Message::System(system(&["a", "a"]))], ..Default::default() });
    assert!(invalid.is_err(), "programmatic duplicate section batch must fail before installation");
}

#[test]
fn duplicate_sections_append_preserves_prior_context() {
    let mut context = normalize_context(Context { messages: vec![Message::System(system(&["a", "b"]))], ..Default::default() }).unwrap();
    let before = context.clone();
    let invalid = context.append_system_change(system(&["a", "a"]));
    assert!(invalid.is_err(), "ambiguous append must return typed failure");
    assert_eq!(context, before, "failed append must preserve prior context");
}
```

### Baseline current-session source/record: runtime-probe-final.metadata.json

SHA-256 `d9c7088f1c6cd5db9c38872d0dbc88e9084e14c5aedf627037133731bba877dd`

```json
{
  "remediation_head": "8010ed2af97cd5deeb1f7240097436fdf32b8d54",
  "command": [
    "C:\\Users\\Luiz\\AppData\\Local\\Temp\\opi-phase20-remediate-hvrszrbs\\runtime_public_probe.exe",
    "--nocapture",
    "--test-threads=1"
  ],
  "exit_code": 101,
  "executed": 8,
  "passed": 0,
  "failed": 8,
  "ignored": 0,
  "compile_command_file": "C:\\Users\\Luiz\\AppData\\Local\\Temp\\opi-phase20-remediate-hvrszrbs\\runtime-probe-compile-command.json",
  "source_identity": {
    "C:\\Users\\Luiz\\AppData\\Local\\Temp\\opi-phase20-remediate-hvrszrbs\\runtime_public_probe.rs": "694e4f520f81a1298d4ac5c0d4b5e21027d340367d6d24bb9fcacd56646c4472",
    "C:\\Users\\Luiz\\AppData\\Local\\Temp\\opi-phase20-remediate-hvrszrbs\\runtime_public_probe.exe": "29189bc740fd1bb086ddfb5f556bfce3d23e67eebfc630f1d156e416cd358388",
    "E:\\opi\\cargo-targets\\opi-72fa37a0fa044958\\debug\\libopi_ai.rlib": "ae11d93563ede488c4b0d42a42b8e6b9cab8f8e58737385b191de7b25c499a12",
    "C:\\Users\\Luiz\\AppData\\Local\\Temp\\opi-phase20-remediate-hvrszrbs\\runtime-probe-final.log": "7b47907ea92c82159da56336ecc03909210fef4fe7231d33774f4ef6793c6e98"
  }
}
```

### Baseline current-session source/record: delivery-probe-record.json

SHA-256 `703c7bf7fbc0ca9f9a03e470e122c2dae59012f15f73f4f7c9cdfa64120d1b9f`

```json
{
  "remediation_head": "8010ed2af97cd5deeb1f7240097436fdf32b8d54",
  "source": "C:\\Users\\Luiz\\AppData\\Local\\Temp\\opi-phase20-remediate-hvrszrbs\\source\\crates\\opi-ai\\tests\\remediation_delivery_probe.rs",
  "source_sha256": "4752fb1927c7fd87b550808f5d24bc91a863dc943c423e65e11e2d6b73b13aa6",
  "rlib": "E:\\opi\\cargo-targets\\opi-72fa37a0fa044958\\debug\\libopi_ai.rlib",
  "rlib_sha256": "ae11d93563ede488c4b0d42a42b8e6b9cab8f8e58737385b191de7b25c499a12",
  "compile_command": [
    "rustc",
    "--edition=2024",
    "--test",
    "C:\\Users\\Luiz\\AppData\\Local\\Temp\\opi-phase20-remediate-hvrszrbs\\source\\crates\\opi-ai\\tests\\remediation_delivery_probe.rs",
    "-L",
    "dependency=E:\\opi\\cargo-targets\\opi-72fa37a0fa044958\\debug\\deps",
    "--extern",
    "opi_ai=E:\\opi\\cargo-targets\\opi-72fa37a0fa044958\\debug\\libopi_ai.rlib",
    "-o",
    "C:\\Users\\Luiz\\AppData\\Local\\Temp\\opi-phase20-remediate-hvrszrbs\\delivery-probe.exe"
  ],
  "compile_exit": 0,
  "run_command": [
    "C:\\Users\\Luiz\\AppData\\Local\\Temp\\opi-phase20-remediate-hvrszrbs\\delivery-probe.exe",
    "--nocapture"
  ],
  "run_exit": 101,
  "run_output": "\nrunning 2 tests\ntest duplicate_sections_append_preserves_prior_context ... FAILED\ntest duplicate_sections_normalization_rejects_ambiguous_batch ... FAILED\n\nfailures:\n\nfailures:\n    duplicate_sections_append_preserves_prior_context\n    duplicate_sections_normalization_rejects_ambiguous_batch\n\ntest result: FAILED. 0 passed; 2 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s\n\n\nthread 'duplicate_sections_append_preserves_prior_context' (58384) panicked at C:\\Users\\Luiz\\AppData\\Local\\Temp\\opi-phase20-remediate-hvrszrbs\\source\\crates\\opi-ai\\tests\\remediation_delivery_probe.rs:26:5:\nambiguous append must return typed failure\nnote: run with `RUST_BACKTRACE=1` environment variable to display a backtrace\n\nthread 'duplicate_sections_normalization_rejects_ambiguous_batch' (62336) panicked at C:\\Users\\Luiz\\AppData\\Local\\Temp\\opi-phase20-remediate-hvrszrbs\\source\\crates\\opi-ai\\tests\\remediation_delivery_probe.rs:18:5:\nprogrammatic duplicate section batch must fail before installation\n"
}
```

### Baseline current-session source/record: auth-schema-compile-command.json

SHA-256 `10e83bb52a14168f990f36d2ecdf6ef0cd42e65c646e2296d9333a5cee53bcb3`

```json
[
  "rustc",
  "--edition=2024",
  "C:\\Users\\Luiz\\AppData\\Local\\Temp\\opi-phase20-remediate-hvrszrbs\\auth-schema-public-probe.rs",
  "-o",
  "C:\\Users\\Luiz\\AppData\\Local\\Temp\\opi-phase20-remediate-hvrszrbs\\auth-schema-public-probe.exe",
  "-L",
  "dependency=E:/opi/cargo-targets/opi-72fa37a0fa044958/debug/deps",
  "--extern",
  "opi_ai=E:\\opi\\cargo-targets\\opi-72fa37a0fa044958\\debug\\libopi_ai.rlib",
  "--extern",
  "tokio=E:\\opi\\cargo-targets\\opi-72fa37a0fa044958\\debug\\deps\\libtokio-c5fb5773e2b13606.rlib",
  "--extern",
  "serde_json=E:\\opi\\cargo-targets\\opi-72fa37a0fa044958\\debug\\deps\\libserde_json-fe255549a686bc35.rlib",
  "--extern",
  "time=E:\\opi\\cargo-targets\\opi-72fa37a0fa044958\\debug\\deps\\libtime-8dd810b7286d335e.rlib",
  "-L",
  "native=E:\\opi\\cargo-targets\\opi-72fa37a0fa044958\\debug\\build\\ring-7574a4c94db18143\\out",
  "-L",
  "native=E:\\opi\\cargo-targets\\opi-72fa37a0fa044958\\debug\\build\\aws-lc-sys-67dcf9723aa65dad\\out"
]
```

### Baseline current-session source/record: event-prelude-probe.log

SHA-256 `9dda650b6549316792e4464e60f50a9c25a303c758e73ad67ec819ff98c8f32a`

```text
CONTROL valid_incomplete retained=16
OBSERVED bad_crc_u32max result=Ok([]) retained=16
OBSERVED impossible_headers result=Ok([]) retained=16

thread 'main' (48864) panicked at C:\Users\Luiz\AppData\Local\Temp\opi-phase20-remediate-hvrszrbs\event-prelude-probe.rs:11:2:
known-invalid prelude CRC must fail before declared payload
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
```

### Baseline current-session source/record: runtime-probe-final.log

SHA-256 `7b47907ea92c82159da56336ecc03909210fef4fe7231d33774f4ef6793c6e98`

```text

running 8 tests
test duration_includes_pending_first_body_poll ... FAILED
test eleven_redirects_are_accepted_by_default_generation ... FAILED
test exact_empty_and_whitespace_api_id_register ... FAILED
test known_total_without_known_parents_is_anchor ... FAILED
test multiline_crlf_split_preserves_event ... FAILED
test response_compat_maximum_false_reaches_wire ... FAILED
test uncapped_default_accepts_valid_large_credential_process ... FAILED
test wrong_type_required_is_strict_error ... FAILED

failures:

failures:
    duration_includes_pending_first_body_poll
    eleven_redirects_are_accepted_by_default_generation
    exact_empty_and_whitespace_api_id_register
    known_total_without_known_parents_is_anchor
    multiline_crlf_split_preserves_event
    response_compat_maximum_false_reaches_wire
    uncapped_default_accepts_valid_large_credential_process
    wrong_type_required_is_strict_error

test result: FAILED. 0 passed; 8 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.97s

elapsed=356.9569ms, duration_ms=Some(0)

thread 'duration_includes_pending_first_body_poll' (38888) panicked at C:\Users\Luiz\AppData\Local\Temp\opi-phase20-remediate-hvrszrbs\runtime_public_probe.rs:81:237:
first body Pending wait belongs to response stream interval
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
sends=11, result=Err(CallFailure { message: AssistantMessage { content: [], api: Anthropic, provider: "", model: "", response_model: None, wire: None, response_id: None, usage: Usage { input_tokens: 0, output_tokens: 0, cache_read_tokens: 0, cache_write_tokens: 0, cache_write_1h_tokens: None, reasoning_tokens: None, total_tokens: None, service_tier: None, effective_service_tier: None, reported: false }, stop_reason: Error, raw_stop_reason: None, error_message: Some("network error: too many redirects"), timestamp_ms: 1793576075787, diagnostics: [], duration_ms: None, provider_thinking_level: None, deferred: None }, cause: Network(ProviderErrorSummary("too many redirects")) })

thread 'eleven_redirects_are_accepted_by_default_generation' (66940) panicked at C:\Users\Luiz\AppData\Local\Temp\opi-phase20-remediate-hvrszrbs\runtime_public_probe.rs:87:318:
source-qualified default accepts lawful 11-hop request
api_id="normal", result=Ok("normal")
api_id="", result=Err(EmptyApiId)
api_id=" ", result=Err(EmptyApiId)

thread 'exact_empty_and_whitespace_api_id_register' (59272) panicked at C:\Users\Luiz\AppData\Local\Temp\opi-phase20-remediate-hvrszrbs\runtime_public_probe.rs:39:2:
ApiId must preserve arbitrary exact source strings including empty and whitespace
independent_total: usage_tokens=0, tokens=2

thread 'known_total_without_known_parents_is_anchor' (60844) panicked at C:\Users\Luiz\AppData\Local\Temp\opi-phase20-remediate-hvrszrbs\runtime_public_probe.rs:24:2:
assertion `left == right` failed: positive independently known total is authoritative without known parent buckets
  left: 0
 right: 999
split_result=Err(CallFailure { message: AssistantMessage { content: [], api: OpenAi, provider: "openai", model: "gpt-4o", response_model: None, wire: Some(OpenAiCompletions), response_id: None, usage: Usage { input_tokens: 0, output_tokens: 0, cache_read_tokens: 0, cache_write_tokens: 0, cache_write_1h_tokens: None, reasoning_tokens: None, total_tokens: None, service_tier: None, effective_service_tier: None, reported: false }, stop_reason: Error, raw_stop_reason: None, error_message: Some("stream error: malformed OpenAI Chat SSE frame"), timestamp_ms: 1793576075793, diagnostics: [], duration_ms: None, provider_thinking_level: None, deferred: None }, cause: StreamError(ProviderErrorSummary("malformed OpenAI Chat SSE frame")) })

thread 'multiline_crlf_split_preserves_event' (51828) panicked at C:\Users\Luiz\AppData\Local\Temp\opi-phase20-remediate-hvrszrbs\runtime_public_probe.rs:78:180:
split CRLF must preserve same legal SSE event
max_output_tokens=32, cache_options=null, prompt_cache_retention="24h"

thread 'response_compat_maximum_false_reaches_wire' (55428) panicked at C:\Users\Luiz\AppData\Local\Temp\opi-phase20-remediate-hvrszrbs\runtime_public_probe.rs:59:2:
source false must omit maximum output tokens
default large credential output accepted=false, error=Some(OutputLimit)

thread 'uncapped_default_accepts_valid_large_credential_process' (47540) panicked at C:\Users\Luiz\AppData\Local\Temp\opi-phase20-remediate-hvrszrbs\runtime_public_probe.rs:97:173:
omitted caller policy must not impose withdrawn 64KiB parity cap
required="a", result=Ok(Object {"type": String("object"), "properties": Object {"a": Object {"anyOf": Array [Object {"type": String("string")}, Object {"type": String("null")}]}}, "required": Array [String("a")], "additionalProperties": Bool(false)})
required=1, result=Ok(Object {"type": String("object"), "properties": Object {"a": Object {"anyOf": Array [Object {"type": String("string")}, Object {"type": String("null")}]}}, "required": Array [String("a")], "additionalProperties": Bool(false)})
required={}, result=Ok(Object {"type": String("object"), "properties": Object {"a": Object {"anyOf": Array [Object {"type": String("string")}, Object {"type": String("null")}]}}, "required": Array [String("a")], "additionalProperties": Bool(false)})

thread 'wrong_type_required_is_strict_error' (67000) panicked at C:\Users\Luiz\AppData\Local\Temp\opi-phase20-remediate-hvrszrbs\runtime_public_probe.rs:50:2:
each present wrong-type required must fail strict shape admission
```

### Baseline current-session source/record: delivery-run.log

SHA-256 `56f0e061c1637001103421e43235e6254743b99bd7331982173424ee25223e9f`

```text

running 2 tests
test duplicate_sections_append_preserves_prior_context ... FAILED
test duplicate_sections_normalization_rejects_ambiguous_batch ... FAILED

failures:

failures:
    duplicate_sections_append_preserves_prior_context
    duplicate_sections_normalization_rejects_ambiguous_batch

test result: FAILED. 0 passed; 2 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


thread 'duplicate_sections_append_preserves_prior_context' (58384) panicked at C:\Users\Luiz\AppData\Local\Temp\opi-phase20-remediate-hvrszrbs\source\crates\opi-ai\tests\remediation_delivery_probe.rs:26:5:
ambiguous append must return typed failure
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

thread 'duplicate_sections_normalization_rejects_ambiguous_batch' (62336) panicked at C:\Users\Luiz\AppData\Local\Temp\opi-phase20-remediate-hvrszrbs\source\crates\opi-ai\tests\remediation_delivery_probe.rs:18:5:
programmatic duplicate section batch must fail before installation
```

## Additional reproducible current proof sources

### Current source/record: ready_runtime_probe.rs

SHA-256 `54d76d3cfd95129ba6ef417a2f1d7afd945319dcafd82f11eff68695aca31a43`

```rust
include!("runtime_public_probe.rs");
fn ready_model(api:&str, compat:serde_json::Value, reasoning:bool)->ModelInfo {
 use opi_ai::catalog::{CatalogModel,CatalogOperation,CatalogInputModality};
 let raw=json!({"id":"probe-model","provider":"probe","name":"probe","api":api,"reasoning":reasoning,"contextWindow":8192,"maxTokens":1024,"input":["text"],"compat":compat});
 CatalogModel{provider:"probe".into(),id:"probe-model".into(),name:"probe".into(),operation:CatalogOperation::Chat,api:api.into(),base_url:String::new(),reasoning,input:vec![CatalogInputModality::Text],cost:None,context_window:Some(8192),max_tokens:Some(1024),raw}.to_chat_model_info().unwrap()
}
fn ready_tool()->opi_ai::message::ToolDef {opi_ai::message::ToolDef{name:"lookup".into(),description:"lookup".into(),input_schema:json!({"type":"object","properties":{"query":{"type":"string"}},"required":["query"]}),constrained_sampling:None}}
fn ready_context(add_tool:bool)->opi_ai::transcript::TranscriptContext {
 use opi_ai::transcript::{SystemMessage,Context,normalize_context};
 normalize_context(Context{system_prompt:None,tools:vec![],messages:vec![Message::System(SystemMessage{content:"initial instructions".into(),timestamp_ms:0,..Default::default()}),Message::User(UserMessage{content:vec![InputContent::Text{text:"before update".into()}],timestamp_ms:1}),Message::System(SystemMessage{content:"later instructions".into(),tools_added:if add_tool{vec![ready_tool()]}else{vec![]},timestamp_ms:2,..Default::default()}),Message::User(UserMessage{content:vec![InputContent::Text{text:"after update".into()}],timestamp_ms:3})]}).unwrap()
}
fn ready_chat(compat:serde_json::Value,reasoning:bool)->opi_ai::openai_chat::OpenAiChatProvider {opi_ai::openai_chat::OpenAiChatProvider::for_route(None,"probe".into(),Default::default(),vec![ready_model("openai-completions",compat,reasoning)],Arc::new(opi_ai::http::HttpClient::new()))}
fn ready_response(compat:serde_json::Value,reasoning:bool)->opi_ai::openai_responses::OpenAiResponsesProvider {opi_ai::openai_responses::OpenAiResponsesProvider::for_route(None,"probe".into(),Default::default(),vec![ready_model("openai-responses",compat,reasoning)],Arc::new(opi_ai::http::HttpClient::new())).with_affinity_policy(opi_ai::openai_responses::ResponsesAffinityPolicy::Direct)}
fn ready_request()->Request {let mut r=request();r.model="probe:probe-model".into();r}
async fn ready_chat_peer(compat:serde_json::Value,call:Request,body:String)->(Result<AssistantMessage,opi_ai::stream::CallFailure>,String) {
 use tokio::io::{AsyncReadExt,AsyncWriteExt};
 let listener=tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();let address=listener.local_addr().unwrap();
 let task=tokio::spawn(async move {let (mut socket,_)=listener.accept().await.unwrap();let mut bytes=Vec::new();loop {let mut buf=[0;4096];let n=socket.read(&mut buf).await.unwrap();if n==0{break;}bytes.extend_from_slice(&buf[..n]);if let Some(i)=bytes.windows(4).position(|x|x==b"\r\n\r\n"){let h=String::from_utf8_lossy(&bytes[..i]);let len=h.lines().find_map(|l|l.to_ascii_lowercase().strip_prefix("content-length:").map(|x|x.trim().parse::<usize>().unwrap())).unwrap_or(0);if bytes.len()>=i+4+len {break;}}}
 let h=format!("HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",body.len());socket.write_all(h.as_bytes()).await.unwrap();let _=socket.write_all(body.as_bytes()).await;String::from_utf8(bytes).unwrap()});
 let provider=opi_ai::openai_chat::OpenAiChatProvider::for_route(Some(format!("http://{address}")),"probe".into(),Default::default(),vec![ready_model("openai-completions",compat,false)],Arc::new(opi_ai::http::HttpClient::new()));
 let result=opi_ai::stream::ChatStream::new(provider.stream_prepared(call,opi_ai::test_support::resolved_bearer_auth())).result().await;(result,task.await.unwrap())
}
#[test] fn ready_projection_facts_survive_actual_lossy_adapter() {runtime().block_on(async {
 let mut call=ready_request();call.context=ready_context(false);let canonical=serde_json::to_value(&call.context).unwrap();
 let (result,request_bytes)=ready_chat_peer(json!({}),call.clone(),terminal()).await;let message=result.unwrap();assert_eq!(canonical,serde_json::to_value(&call.context).unwrap(),"canonical transcript unchanged");
 let captured:serde_json::Value=serde_json::from_str(request_bytes.split_once("\r\n\r\n").unwrap().1).unwrap();assert!(captured["messages"][0]["content"].as_str().unwrap().contains("later instructions"),"actual consumer really collapsed a later system update");
 let result=serde_json::to_value(message).unwrap();eprintln!("actual projection facts={}",result["projection"]);assert!(result["projection"].is_object(),"actual successful lossy projection must preserve structured facts in result serialization");assert_eq!(result["projection"]["temporal"]["mode"],"collapse");assert_eq!(result["projection"]["sourcePrefix"]["messageCount"],4);
 });}
#[test] fn ready_image_only_owner_is_visible_to_provider_management() {
 use opi_ai::api::openrouter_images::OpenRouterImagesApi;use opi_ai::images::{ImageModelRecord,ImageOutputModality};
 let mut models=Models::new();models.register_chat_api(Arc::new(DummyApi(ApiId::new("chat"))),Arc::new(StaticAuthResolver::new(AuthScheme::Bearer,secrecy::SecretString::from("synthetic"))),AuthProvenanceSource::Static).unwrap();assert_eq!(models.provider_ids(),vec!["probe"]);
 let image=ImageModelRecord{cost_tiers:vec![],headers:vec![],input_limits:None,input:vec![opi_ai::catalog::CatalogInputModality::Text],provider_id:"image-only".into(),id:"image".into(),name:"image".into(),api:ApiId::new("openrouter-images"),base_url:"http://127.0.0.1:1".into(),output:vec![ImageOutputModality::Image],cost:None};models.register_image_api(Arc::new(OpenRouterImagesApi::new(opi_ai::http::HttpClient::new(),image)),Arc::new(StaticAuthResolver::new(AuthScheme::Bearer,secrecy::SecretString::from("synthetic"))),AuthProvenanceSource::Static).unwrap();assert!(models.image_model("image-only","image").is_ok());let ids=models.provider_ids();eprintln!("registered chat+image owners management sees={ids:?}");assert!(ids.contains(&"image-only"),"provider management must include all registered operation owners without source I/O");
}
#[test] fn ready_chat_routing_preserves_all_nested_strategies() {
 let routing=json!({"allow_fallbacks":false,"require_parameters":true,"data_collection":"deny","zdr":true,"enforce_distillable_text":false,"order":["a","b"],"only":[],"ignore":["c"],"quantizations":["fp8"],"sort":{"by":"latency","partition":null},"max_price":{"prompt":"0.1","completion":0.2,"image":0,"audio":"1","request":1.5},"preferred_min_throughput":{"p50":0,"p75":10,"p90":20,"p99":30},"preferred_max_latency":0.5});
 let baseline=ready_chat(json!({}),false).build_request_body(&ready_request()).unwrap();assert!(baseline.get("provider").is_none());let body=ready_chat(json!({"openRouterRouting":routing}),false).build_request_body(&ready_request()).unwrap();eprintln!("routing={}",body["provider"]);assert_eq!(body["provider"],routing);
}
#[test] fn ready_chat_vercel_routes_preserve_empty_only_and_order() {let body=ready_chat(json!({"vercelGatewayRouting":{"only":[],"order":["anthropic","openai"]}}),false).build_request_body(&ready_request()).unwrap();eprintln!("providerOptions={}",body["providerOptions"]);assert_eq!(body["providerOptions"],json!({"gateway":{"only":[],"order":["anthropic","openai"]}}));}
#[test] fn ready_chat_fractional_vllm_priority_survives() {let body=ready_chat(json!({"vllmPriority":1.5}),false).build_request_body(&ready_request()).unwrap();eprintln!("priority={}",body["priority"]);assert_eq!(body["priority"],1.5);}
#[test] fn ready_chat_long_cache_retention_flag_reaches_wire() {let mut call=ready_request();call.cache_retention=CacheRetention::Long;call.session_id=Some("synthetic-session".into());let body=ready_chat(json!({"supportsLongCacheRetention":true}),false).build_request_body(&call).unwrap();eprintln!("retention={}",body["prompt_cache_retention"]);assert_eq!(body["prompt_cache_retention"],"24h");}
#[test] fn ready_chat_cache_control_format_reaches_markers() {let mut call=ready_request();call.context=ready_context(true);call.cache_retention=CacheRetention::Long;let body=ready_chat(json!({"cacheControlFormat":"anthropic","supportsLongCacheRetention":true}),false).build_request_body(&call).unwrap();eprintln!("cache markers present={}",body.to_string().contains("cache_control"));assert!(body.to_string().contains("cache_control"),"explicit Anthropic cache format marks system/tool/last eligible content");}
#[test] fn ready_chat_grammar_capability_reaches_custom_wire_tool() {let mut tool=ready_tool();tool.constrained_sampling=Some(opi_ai::constrained_sampling::ConstrainedSampling::Grammar{openai_lark:Some("start: /.+/".into()),openai_regex:None});let mut call=ready_request();call.context=opi_ai::transcript::normalize_context(opi_ai::transcript::Context{tools:vec![tool],..Default::default()}).unwrap();let body=ready_chat(json!({"supportsOpenAIGrammarTools":true}),false).build_request_body(&call).unwrap();eprintln!("grammar tool={}",body["tools"][0]);assert_eq!(body["tools"][0]["type"],"custom");}
#[test] fn ready_chat_strict_capability_accepts_valid_require() {let mut tool=ready_tool();tool.constrained_sampling=Some(opi_ai::constrained_sampling::ConstrainedSampling::JsonSchema{strict:opi_ai::constrained_sampling::StrictPreference::Require});let mut call=ready_request();call.context=opi_ai::transcript::normalize_context(opi_ai::transcript::Context{tools:vec![tool],..Default::default()}).unwrap();let body=ready_chat(json!({"supportsStrictMode":true}),false).build_request_body(&call);eprintln!("require result={body:?}");assert!(body.is_ok(),"explicit capable Chat strict accepts a convertible require schema");}
#[test] fn ready_chat_midconversation_capability_preserves_position() {let mut call=ready_request();call.context=ready_context(false);let body=ready_chat(json!({"supportsMidConvoSystemMessages":true}),false).build_request_body(&call).unwrap();let ms=body["messages"].as_array().unwrap();eprintln!("message count={}",ms.len());assert_eq!(ms.len(),4,"native supported system update remains between its surrounding user turns");assert_eq!(ms[2]["role"],"system");}
#[test] fn ready_chat_finish_reason_false_accepts_legal_eof() {runtime().block_on(async {let s="data: {\"id\":\"probe\",\"model\":\"probe-model\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\"hello\"},\"finish_reason\":null}]}\n\n";let (strict,_)=ready_chat_peer(json!({"supportsFinishReason":true}),ready_request(),s.into()).await;assert!(strict.is_err(),"required finish fact absence control");let (inferred,_)=ready_chat_peer(json!({"supportsFinishReason":false}),ready_request(),s.into()).await;eprintln!("finishless declared branch={inferred:?}");assert!(inferred.is_ok(),"declared source without finish_reason infers stop/toolUse at EOF");});}
#[test] fn ready_chat_session_affinity_format_reaches_headers() {runtime().block_on(async {let mut call=ready_request();call.session_id=Some("synthetic-session".into());let (result,request)=ready_chat_peer(json!({"sendSessionAffinityHeaders":true,"sessionAffinityFormat":"openrouter"}),call,terminal()).await;assert!(result.is_ok());let headers=request.split_once("\r\n\r\n").unwrap().0.to_ascii_lowercase();eprintln!("x-session-id present={}",headers.contains("x-session-id: synthetic-session"));assert!(headers.contains("x-session-id: synthetic-session"),"explicit OpenRouter affinity format emits x-session-id");});}
#[test] fn ready_responses_additional_tools_capability_reaches_input() {let mut call=ready_request();call.context=ready_context(true);let body=ready_response(json!({"supportsMidConvoSystemMessages":true,"supportsAdditionalTools":true}),false).build_request_body(&call).unwrap();eprintln!("additional input={}",body["input"]);assert!(body["input"].as_array().unwrap().iter().any(|v|v["type"]=="additional_tools"));}
#[test] fn ready_responses_tool_search_capability_reaches_paired_items() {let mut call=ready_request();call.context=ready_context(true);let body=ready_response(json!({"supportsMidConvoSystemMessages":true,"supportsAdditionalTools":false,"supportsToolSearch":true}),false).build_request_body(&call).unwrap();let input=body["input"].as_array().unwrap();eprintln!("search input={}",body["input"]);assert!(input.iter().any(|v|v["type"]=="tool_search_call")&&input.iter().any(|v|v["type"]=="tool_search_output"));}
#[test] fn ready_responses_explicit_cache_disabled_mode_reaches_wire() {let mut call=ready_request();call.cache_retention=CacheRetention::Disabled;let body=ready_response(json!({"supportsExplicitPromptCacheMode":true}),false).build_request_body(&call).unwrap();eprintln!("prompt_cache_options={}",body["prompt_cache_options"]);assert_eq!(body["prompt_cache_options"],json!({"mode":"explicit"}));assert!(body.get("prompt_cache_retention").is_none());}
#[test] fn ready_responses_long_cache_false_omits_legacy_retention() {let mut call=ready_request();call.cache_retention=CacheRetention::Long;call.session_id=Some("synthetic-session".into());let body=ready_response(json!({"supportsLongCacheRetention":false,"supportsExplicitPromptCacheMode":false}),false).build_request_body(&call).unwrap();eprintln!("legacy retention withfalse={}",body["prompt_cache_retention"]);assert!(body.get("prompt_cache_retention").is_none());}
#[test] fn ready_responses_developer_false_preserves_system_role() {let mut call=ready_request();call.context=ready_context(false);let body=ready_response(json!({"supportsDeveloperRole":false,"supportsMidConvoSystemMessages":true}),true).build_request_body(&call).unwrap();eprintln!("system-role input={}",body["input"]);assert_eq!(body["input"][0]["role"],"system","source false overrides reasoning developer role and is represented as input system");}
```

### Current source/record: ready_telemetry_probe.rs

SHA-256 `6a89cfcbb0e0c1644fdf97ddc4d4d09240ae672af40adfbfa9ff4ba761b85d81`

```rust
use std::sync::{Arc,Mutex};
use std::sync::atomic::{AtomicUsize,Ordering};
use std::pin::Pin;
use std::task::{Context as TaskContext,Poll};
use futures_core::Stream;
use opi_ai::models::{Models,ChatApi,ApiId,ChatModelRecord};
use opi_ai::provider::{Request,EventStream,ProviderError};
use opi_ai::auth::{ResolvedAuth,AuthScheme,AuthProvenanceSource,StaticAuthResolver};
use opi_ai::{ModelInfo,ModelCapabilities,WireApi};
use opi_ai::telemetry::TelemetryContext;
use opi_ai::stream::AssistantStreamEvent;
use tracing::Level;
use tracing_core::field::{Field,Visit};
use tracing_core::span::{Attributes,Current,Id,Record};
use tracing_core::{Dispatch,Event,Metadata,Subscriber,dispatcher};
#[derive(Default)]
struct Recording {
    next: u64,
    spans: Vec<(&'static str, Option<u64>)>,
    events: Vec<(&'static str, Vec<(String, String)>)>,
    metadata: std::collections::HashMap<u64, &'static Metadata<'static>>,
    entered: std::collections::HashMap<std::thread::ThreadId, Vec<Id>>,
}

struct Recorder(Arc<Mutex<Recording>>);

impl Subscriber for Recorder {
    fn enabled(&self, _metadata: &Metadata<'_>) -> bool {
        true
    }
    fn new_span(&self, attributes: &Attributes<'_>) -> Id {
        let mut state = self.0.lock().unwrap();
        state.next += 1;
        let id = Id::from_u64(state.next);
        let parent = attributes.parent().cloned().or_else(|| {
            attributes
                .is_contextual()
                .then(|| {
                    state
                        .entered
                        .get(&std::thread::current().id())
                        .and_then(|stack| stack.last())
                        .cloned()
                })
                .flatten()
        });
        state
            .spans
            .push((attributes.metadata().name(), parent.map(|id| id.into_u64())));
        state.metadata.insert(id.into_u64(), attributes.metadata());
        id
    }
    fn record(&self, _: &Id, _: &Record<'_>) {}
    fn record_follows_from(&self, _: &Id, _: &Id) {}
    fn event(&self, event: &Event<'_>) {
        let mut fields = Vec::new();
        let mut visitor = FieldCollector(&mut fields);
        event.record(&mut visitor);
        self.0
            .lock()
            .unwrap()
            .events
            .push((event.metadata().name(), fields));
    }
    fn enter(&self, id: &Id) {
        self.0
            .lock()
            .unwrap()
            .entered
            .entry(std::thread::current().id())
            .or_default()
            .push(id.clone());
    }
    fn exit(&self, _: &Id) {
        self.0
            .lock()
            .unwrap()
            .entered
            .get_mut(&std::thread::current().id())
            .unwrap()
            .pop();
    }
    fn current_span(&self) -> Current {
        let state = self.0.lock().unwrap();
        match state
            .entered
            .get(&std::thread::current().id())
            .and_then(|stack| stack.last())
        {
            Some(id) => Current::new(id.clone(), state.metadata[&id.into_u64()]),
            None => Current::none(),
        }
    }
}

struct FieldCollector<'a>(&'a mut Vec<(String, String)>);

impl Visit for FieldCollector<'_> {
    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
        self.0.push((field.name().to_owned(), format!("{value:?}")));
    }
    fn record_str(&mut self, field: &Field, value: &str) {
        self.0.push((field.name().to_owned(), value.to_owned()));
    }
}

fn recorder() -> (Arc<Mutex<Recording>>, Dispatch) {
    let records = Arc::new(Mutex::new(Recording::default()));
    let dispatch = Dispatch::new(Recorder(records.clone()));
    (records, dispatch)
}


#[derive(Default)] struct Counts { polls:AtomicUsize,drops:AtomicUsize }
struct PlainStream { counts:Arc<Counts> }
impl Stream for PlainStream {
 type Item=Result<AssistantStreamEvent,ProviderError>;
 fn poll_next(self:Pin<&mut Self>,_:&mut TaskContext<'_>)->Poll<Option<Self::Item>> {
  let n=self.counts.polls.fetch_add(1,Ordering::SeqCst);
  tracing::span!(Level::INFO,"plain_stream_poll").in_scope(||());
  if n==0 { Poll::Pending } else {
   let message=opi_ai::test_support::base_assistant();
   Poll::Ready(Some(Ok(AssistantStreamEvent::Done{reason:message.stop_reason,message})))
  }
 }
}
impl Drop for PlainStream {fn drop(&mut self){self.counts.drops.fetch_add(1,Ordering::SeqCst);tracing::span!(Level::INFO,"plain_stream_drop").in_scope(||());}}
struct PlainApi(Arc<Counts>);
fn model()->ModelInfo {ModelInfo::new("marker","Marker",WireApi::OpenAiCompletions,ModelCapabilities::new(100000,4096).with_streaming(true))}
impl ChatApi<()> for PlainApi {
 fn api_id(&self)->&ApiId {static ID:std::sync::OnceLock<ApiId>=std::sync::OnceLock::new();ID.get_or_init(||ApiId::new("plain-stream"))}
 fn provider_id(&self)->&str {"plain"}
 fn complete_record(&self)->ModelInfo {model()}
 fn simple_options(&self){}
 fn stream_prepared(&self,_:Request,_:(),_:ResolvedAuth)->EventStream {Box::pin(PlainStream{counts:self.0.clone()})}
}
fn request(context:TelemetryContext)->Request {
 Request {model:"plain:marker".into(),context:opi_ai::transcript::normalize_context(Default::default()).unwrap(),max_tokens:Some(64),temperature:None,sampling_params:Default::default(),thinking:Default::default(),stop_sequences:vec![],metadata:None,cancel:Default::default(),timeout:None,extra_headers:vec![],cache_retention:Default::default(),session_id:None,telemetry_context:Some(context),callbacks:Default::default(),transport:Default::default()}
}
#[test]
fn actual_chat_stream_pending_cross_thread_drop_uses_frozen_source() {
 let (a,ad)=recorder();let (b,bd)=recorder();let counts=Arc::new(Counts::default());
 let parent=dispatcher::with_default(&ad,||tracing::span!(Level::INFO,"frozen_parent"));let id=parent.id().unwrap().into_u64();let context=TelemetryContext::from_span(parent);
 context.in_scope(||tracing::span!(Level::INFO,"positive_control").in_scope(||()));
 assert!(a.lock().unwrap().spans.iter().any(|(n,p)|*n=="positive_control"&&*p==Some(id)));
 let mut models=Models::new();let handle=models.register_chat_api(Arc::new(PlainApi(counts.clone())),Arc::new(StaticAuthResolver::new(AuthScheme::Bearer,secrecy::SecretString::from("SYNTHETIC"))),AuthProvenanceSource::Static).unwrap();
 let mut call=models.stream(ChatModelRecord::new("plain",model()),request(context),handle.options(()));
 assert_eq!(counts.polls.load(Ordering::SeqCst),0);
 let w=futures_util::task::noop_waker();let mut cx=TaskContext::from_waker(&w);
 assert!(dispatcher::with_default(&bd,||futures_util::StreamExt::poll_next_unpin(&mut call,&mut cx)).is_pending());
 assert_eq!(counts.polls.load(Ordering::SeqCst),1,"one outer poll must poll business stream exactly once");
 std::thread::spawn(move||dispatcher::with_default(&bd,||drop(call))).join().unwrap();
 assert_eq!(counts.drops.load(Ordering::SeqCst),1);
 let a=a.lock().unwrap();let b=b.lock().unwrap();println!("OBSERVED source={:?} ambient={:?} business_polls=1 drops=1",a.spans,b.spans);
 assert!(a.spans.iter().any(|(n,p)|*n=="plain_stream_poll"&&*p==Some(id)),"Pending Stream poll observation must use frozen source dispatch/parent");
 assert!(a.spans.iter().any(|(n,p)|*n=="plain_stream_drop"&&*p==Some(id)),"cross-thread Drop observation must use frozen source dispatch/parent");
 assert!(b.spans.is_empty(),"plain stream observation must not leak to ambient");
}
```

### Current source/record: ready_onboarding_harness_probe.py

SHA-256 `461923285db3a36a662625316fed567127958396473b3527d36addca7f442604`

```python
import hashlib,importlib.util,json,os,pathlib,shutil,subprocess,sys,tempfile,unittest
from types import SimpleNamespace
from unittest.mock import patch
sys.dont_write_bytecode=True
SOURCE=pathlib.Path(r"C:\Users\Luiz\AppData\Local\Temp\opi-phase20-remediate-hvrszrbs\source")
OUT=SOURCE.parent
ARCHIVE=pathlib.Path(r"C:\Users\Luiz\AppData\Local\Temp\opi-phase20-ready-w8p6zu1s\package-ws-proof\opi-ai-0.8.3.crate")
sys.path.insert(0,str(SOURCE/'scripts/pi-sdk-conformance'))
spec=importlib.util.spec_from_file_location('readme_proof',SOURCE/'scripts/pi-sdk-conformance/run_readme_examples.py');proof=importlib.util.module_from_spec(spec);spec.loader.exec_module(proof)
class SameSourceExecutionContract(unittest.TestCase):
 def test_success_requires_three_real_packaged_example_executions(self):
  fixture=pathlib.Path(tempfile.mkdtemp(prefix='ready-readme-driver-',dir=OUT));harness=fixture/'harness';harness.mkdir();(harness/'evidence').mkdir();shutil.copytree(SOURCE/'scripts/pi-sdk-conformance/consumer-readme',harness/'consumer-readme')
  # This is an executor fixture, not a Cargo build/cache: no process runs Cargo.
  fake_cache=fixture/'fake-cache';(fake_cache/'package').mkdir(parents=True);(fake_cache/'.opi-cargo-cache.json').write_text(json.dumps({'workspace':str(SOURCE)}),encoding='utf-8');shutil.copyfile(ARCHIVE,fake_cache/'package'/ARCHIVE.name)
  scratch=fixture/'scratch';commands=[]
  def executor(command,**kwargs):
   commands.append(list(map(str,command)))
   if command[:2]==['cargo','generate-lockfile']:
    (pathlib.Path(kwargs['cwd'])/'Cargo.lock').write_text('version = 4\n[[package]]\nname = "consumer-readme"\nversion = "0.0.0"\n[[package]]\nname = "opi-ai"\nversion = "0.8.3"\n',encoding='utf-8')
   return SimpleNamespace(returncode=0,stdout='fixture-success\n',stderr='')
  with patch.object(proof,'HARNESS',harness),patch.object(proof,'controlled_environment',side_effect=lambda env,target:dict(env,CARGO_TARGET_DIR=str(target))),patch.object(proof,'check_cargo_configs',return_value=[]),patch.object(proof.subprocess,'run',side_effect=executor),patch.object(sys,'argv',['run_readme_examples.py','--offline','--scratch',str(scratch),'--target-dir',str(fake_cache)]):
   result=proof.main()
  evidence=json.loads((harness/'evidence/readme-examples-proof.json').read_text(encoding='utf-8'))
  self.assertEqual(result,0,'positive control: valid current artifact/examples/manual embeddings were admitted')
  self.assertTrue(all(evidence['manuals_embed_exact_sources'].values()))
  self.assertEqual(len(evidence['packaged_examples']),3)
  execution=[c for c in commands if c[:2] in [['cargo','run'],['cargo','test']] or any(pathlib.Path(c[0]).name in ['quick_start.exe','quick_start_stream.exe','tool_loop.exe','quick_start','quick_start_stream','tool_loop'] for _ in [0])]
  record={'package':str(ARCHIVE),'package_sha256':hashlib.sha256(ARCHIVE.read_bytes()).hexdigest(),'fixture':str(fixture),'commands':commands,'harness_verdict':evidence['verdict'],'manuals_embed_exact_sources':evidence['manuals_embed_exact_sources'],'execution_count':len(execution),'boundary':'Real unchanged runner main and real fresh .crate source/embedding checks; Cargo/lease subprocesses are a controlled test executor. This proves false PASS on zero execution, not successful package-graph execution.'}
  (OUT/'ready-onboarding-harness-record.json').write_text(json.dumps(record,indent=2)+'\n',encoding='utf-8');print('OBSERVED',json.dumps(record))
  self.assertEqual(len(execution),3,'ONB-02 requires execution of all three identical packaged sources before PASS')
if __name__=='__main__':unittest.main()
```

### Current source/record: ready-runtime-proof-metadata.json

SHA-256 `c7ba203c864fa5f207ffbc7dd72a8a2da3524b021d5a2238cdfbe144b6c9815e`

```json
{
  "remediation_head": "8010ed2af97cd5deeb1f7240097436fdf32b8d54",
  "command": [
    "C:\\Users\\Luiz\\AppData\\Local\\Temp\\opi-phase20-remediate-hvrszrbs\\ready_runtime_probe.exe",
    "ready_",
    "--nocapture",
    "--test-threads=1"
  ],
  "compile_command": [
    "rustc",
    "--edition=2024",
    "--test",
    "C:\\Users\\Luiz\\AppData\\Local\\Temp\\opi-phase20-remediate-hvrszrbs\\ready_runtime_probe.rs",
    "-L",
    "dependency=E:\\opi\\cargo-targets\\opi-72fa37a0fa044958\\debug\\deps",
    "--extern",
    "opi_ai=E:\\opi\\cargo-targets\\opi-72fa37a0fa044958\\debug\\libopi_ai.rlib",
    "--extern",
    "tokio=E:\\opi\\cargo-targets\\opi-72fa37a0fa044958\\debug\\deps\\libtokio-c5fb5773e2b13606.rlib",
    "--extern",
    "tokio_util=E:\\opi\\cargo-targets\\opi-72fa37a0fa044958\\debug\\deps\\libtokio_util-0735778fe3c86a05.rlib",
    "--extern",
    "serde_json=E:\\opi\\cargo-targets\\opi-72fa37a0fa044958\\debug\\deps\\libserde_json-fe255549a686bc35.rlib",
    "--extern",
    "futures_util=E:\\opi\\cargo-targets\\opi-72fa37a0fa044958\\debug\\deps\\libfutures_util-a771d3a0cb0e8911.rlib",
    "--extern",
    "secrecy=E:\\opi\\cargo-targets\\opi-72fa37a0fa044958\\debug\\deps\\libsecrecy-cd855173832e8acd.rlib",
    "-o",
    "C:\\Users\\Luiz\\AppData\\Local\\Temp\\opi-phase20-remediate-hvrszrbs\\ready_runtime_probe.exe"
  ],
  "exit_code": 101,
  "executed": 17,
  "passed": 0,
  "failed": 17,
  "ignored": 0,
  "filtered_out": 8,
  "artifacts": {
    "C:\\Users\\Luiz\\AppData\\Local\\Temp\\opi-phase20-remediate-hvrszrbs\\ready_runtime_probe.rs": "54d76d3cfd95129ba6ef417a2f1d7afd945319dcafd82f11eff68695aca31a43",
    "C:\\Users\\Luiz\\AppData\\Local\\Temp\\opi-phase20-remediate-hvrszrbs\\runtime_public_probe.rs": "694e4f520f81a1298d4ac5c0d4b5e21027d340367d6d24bb9fcacd56646c4472",
    "C:\\Users\\Luiz\\AppData\\Local\\Temp\\opi-phase20-remediate-hvrszrbs\\ready_runtime_probe.exe": "9f2aa80a1c69112cd5ee62d0a55d2773cbe5bca109d29667b4c413401b679d13",
    "C:\\Users\\Luiz\\AppData\\Local\\Temp\\opi-phase20-remediate-hvrszrbs\\ready-runtime-red.log": "7998f820d2bb77c59ace254bc4529db7665d4b01dde13c3c3bd3819612fb8068",
    "E:\\opi\\cargo-targets\\opi-72fa37a0fa044958\\debug\\libopi_ai.rlib": "ae11d93563ede488c4b0d42a42b8e6b9cab8f8e58737385b191de7b25c499a12"
  }
}
```

### Current source/record: ready-telemetry-record.json

SHA-256 `875203cfb95ac79e27c8eb7e108726501d17d662addff4642c5e3b857613d4dc`

```json
{
  "command": [
    "rustc",
    "--edition=2024",
    "--test",
    "C:\\Users\\Luiz\\AppData\\Local\\Temp\\opi-phase20-remediate-hvrszrbs\\ready_telemetry_probe.rs",
    "-L",
    "dependency=E:\\opi\\cargo-targets\\opi-72fa37a0fa044958\\debug\\deps",
    "--extern",
    "opi_ai=E:\\opi\\cargo-targets\\opi-72fa37a0fa044958\\debug\\libopi_ai.rlib",
    "--extern",
    "futures_core=E:\\opi\\cargo-targets\\opi-72fa37a0fa044958\\debug\\deps\\libfutures_core-2c9d0e403b6efbe1.rlib",
    "--extern",
    "futures_util=E:\\opi\\cargo-targets\\opi-72fa37a0fa044958\\debug\\deps\\libfutures_util-a771d3a0cb0e8911.rlib",
    "--extern",
    "tracing=E:\\opi\\cargo-targets\\opi-72fa37a0fa044958\\debug\\deps\\libtracing-40e1734752daa2ef.rlib",
    "--extern",
    "tracing_core=E:\\opi\\cargo-targets\\opi-72fa37a0fa044958\\debug\\deps\\libtracing_core-850b5a32b0b59124.rlib",
    "--extern",
    "secrecy=E:\\opi\\cargo-targets\\opi-72fa37a0fa044958\\debug\\deps\\libsecrecy-cd855173832e8acd.rlib",
    "-o",
    "C:\\Users\\Luiz\\AppData\\Local\\Temp\\opi-phase20-remediate-hvrszrbs\\ready_telemetry_probe.exe"
  ],
  "compile_exit": 0,
  "source_sha256": "6a89cfcbb0e0c1644fdf97ddc4d4d09240ae672af40adfbfa9ff4ba761b85d81",
  "setup_failure_excluded": "Earlier invalid root TelemetryContext import corrected; compilation failure was not behavioral red.",
  "run": [
    "C:\\Users\\Luiz\\AppData\\Local\\Temp\\opi-phase20-remediate-hvrszrbs\\ready_telemetry_probe.exe",
    "--nocapture",
    "--test-threads=1"
  ],
  "run_exit": 101,
  "log_sha256": "f4c0a6a3061b1180ff224805b5f433d3112e84be2c782a6d15d04fa481ef147f"
}
```

### Current source/record: ready-onboarding-harness-record.json

SHA-256 `8662825dc24f6077edd850cef25b5a0992f18d831653437e754aa45b1aa365b7`

```json
{
  "package": "C:\\Users\\Luiz\\AppData\\Local\\Temp\\opi-phase20-ready-w8p6zu1s\\package-ws-proof\\opi-ai-0.8.3.crate",
  "package_sha256": "0110e99980f002998edf81aa76fba30a4f0a402b864b4ff9a8d88e7344061a22",
  "fixture": "C:\\Users\\Luiz\\AppData\\Local\\Temp\\opi-phase20-remediate-hvrszrbs\\ready-readme-driver-r5n7s5o3",
  "commands": [
    [
      "rustc",
      "-vV"
    ],
    [
      "cargo",
      "-vV"
    ],
    [
      "D:\\Users\\Luiz\\anaconda3\\python.exe",
      "C:\\Users\\Luiz\\AppData\\Local\\Temp\\opi-phase20-remediate-hvrszrbs\\source\\scripts\\opi-cargo-cache.py",
      "lease",
      "start",
      "--target",
      "C:\\Users\\Luiz\\AppData\\Local\\Temp\\opi-phase20-remediate-hvrszrbs\\ready-readme-driver-r5n7s5o3\\fake-cache",
      "--pid",
      "64772"
    ],
    [
      "cargo",
      "package",
      "-p",
      "opi-ai",
      "--allow-dirty",
      "--no-verify",
      "--offline",
      "--locked"
    ],
    [
      "cargo",
      "generate-lockfile",
      "--offline"
    ],
    [
      "cargo",
      "check",
      "--offline",
      "--locked",
      "--bins"
    ],
    [
      "D:\\Users\\Luiz\\anaconda3\\python.exe",
      "C:\\Users\\Luiz\\AppData\\Local\\Temp\\opi-phase20-remediate-hvrszrbs\\source\\scripts\\opi-cargo-cache.py",
      "lease",
      "stop",
      "--target",
      "C:\\Users\\Luiz\\AppData\\Local\\Temp\\opi-phase20-remediate-hvrszrbs\\ready-readme-driver-r5n7s5o3\\fake-cache",
      "--pid",
      "64772"
    ]
  ],
  "harness_verdict": "PASS",
  "manuals_embed_exact_sources": {
    "README.md": true,
    "README.zh.md": true
  },
  "execution_count": 0,
  "boundary": "Real unchanged runner main and real fresh .crate source/embedding checks; Cargo/lease subprocesses are a controlled test executor. This proves false PASS on zero execution, not successful package-graph execution."
}
```

## Materialization and approval boundary

Observable completion: exact strict-union dispositions -> verify: dispositions validator; full index/head/batch/obligation binding -> verify: locked plan validator; carried-in preservation -> verify: same shared HEAD/branch and only two task-owned fixed plan files changed; document/whitespace -> verify: doc checker and diff checks.

The factual matrix correction fixes only its false-zero finding. Result handoff must supply exact changed registered-source digest to the existing ledger owner for P.0/P.2/P.4 before any future source-hash installation/task/Phase closure; that determined ownership boundary is retained, not silently waived.

This fixed pair has a new validator-emitted digest. No remediation apply is authorized by planning, native verification commits or CI approval. Apply needs a new explicit invocation naming the exact READY digest; subsequent plan-only commits preserve this source baseline, while other source/index/plan-byte changes require their prescribed reseal. No result is produced during planning.
