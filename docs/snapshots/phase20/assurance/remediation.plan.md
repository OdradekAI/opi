# Phase 20 Remediation Plan

**Status**: READY-FOR-APPLY
**Audit index SHA-256**: `e1b34739db46331707a93ab0f6f9336ed6386a4564b0215d8488ce288319f0ae`
**Remediation head**: `2eee4afebc911482d969ead8452115a4a7f9592d`
**Disposition artifact**: `remediation.plan.dispositions.jsonl`
**Dirty-worktree baseline**: committed source=`2eee4afebc911482d969ead8452115a4a7f9592d`; staged=[]; unstaged=[]; untracked=[] at replanning admission. Only the fixed plan pair is generated afterward; B01 is already committed.
**Unresolved decisions**: none

## Admission and observable completion

Validate the active set -> verify: `validate_assurance_artifact.py audit-set` PASS before semantic consumption; raw index SHA-256 above was independently computed. Verify every source key -> verify: exactly28 dispositions (21 Codex +7 Pi), with full member run/digest identities. Seal fixed pair -> verify: dispositions, plan validator, documentation check and whitespace check. This run delivers the sealed READY-FOR-APPLY plan after final validation; it makes no product repair, apply result, requirement-met assertion or Phase verdict.

Only the current indexed set supplies findings. Cnnn abbreviates the full Codex run below and Pnnn the full Pi run; `nnn` is the suffix of `P20-AUD-nnn`. Abbreviations do not merge duplicate textual IDs. Original severities, linked requirement states and evidence remain untouched. The active approved design is resealed after an observed execution-order blocker; only its clean tracked fixed plan outputs are replaced. Existing result files remain unread and unchanged, and are not results of this plan.

| Member | Full run | Findings SHA-256 | Audit head |
|---|---|---|---|
| audit.codex.gpt61 | `phase20-codex-gpt61-b3f1810-20261103t142820z` | `4cec97af8ef6bb773c630ee251728f31212e12fdc1bf9631f10b2dc3084bc0f6` | `b3f1810801195d4e4746d9c9f0ac63d39da57313` |
| audit.pi.glm53 | `phase20-pi-glm53-44374ec-20261103t160703z` | `f8c06b50b3c51c95ac5b1309e72a8b31d06c34785ad588ed4e42bff19d914990` | `44374ec3c70b82125a183757dde2017b46299c45` |

Original runtime reproduction evidence was inspected at the previous committed head in a unique `git archive` export, never a Git worktree: `C:\Users\Luiz\AppData\Local\Temp\opi-phase20-remediate-plan-jqdki4yk\source`. Its `.claude/skills` symlink required directory-type rematerialization after Windows tar extraction; link target and committed file contents stayed unchanged. The original export tar SHA-256 is `705e8104d5d051225355351d28d78e72b3cc98d942d0b95a701cb7ed95fb3b7f`; after all planning checks, all1672 regular files matched its exact bytes. A separate archive for the private product classifier appends only `#[cfg(test)]` instrumentation; its original production prefix and893 crate/build inputs were compared. External probe crates use path dependencies into the archive solely for planning reproduction, not as claimed distributable-package proof.

The user subsequently authorized Git commits. Workflow/source-binding prerequisite commit `68e276822185346db0bf57e31175dcac599ef7c7` was the original planning source baseline. A fresh unique `git archive` export at `C:\Users\Luiz\AppData\Local\Temp\opi-p20-commit-rebind-m2s4_bwe\source` was checked against the original archive: all1672 regular files and path/mode sets are verified; only the10 committed workflow/ledger/changelog paths differ. At that original comparison, every production crate, manifest/lockfile, fixture, package runner, CI file and registered normative source retained identical bytes. Original execution identities stay immutable; the later B01 delta is separately qualified and replayed below. Fresh archive SHA256 `7135479bfd3e8a0b29cc7093d0ae474e049dd90e552254b0151eefb6c8ad494a`; exact applicability receipt `D:\Luiz\Odradek\opi-remediation-evidence\phase20\commit-rebind-68e2768\source-applicability.json` SHA256 `f30212e988d96295d8c7a9d65a30ff79bec5b2aafbb4ec35d98451941d9b4ed2`; durable archive `D:\Luiz\Odradek\opi-remediation-evidence\phase20\commit-rebind-68e2768\source.tar`. That export records the original planning inspection. The present inspection uses the later B01 commit and unique archive below; earlier probes retain their original execution identities.

Current source baseline remains valid through future commits touching only this Phase fixed plan pair, under the shared source-baseline rule. Any other intervening committed change, including required ledger/source reconciliation, requires a fresh plan. The active index and all member digests must remain exact. No history report, earlier remediation result or eval artifact supplied a disposition. The sole raw-proof candidate used was the exact delivery-evidence document identified below; its historical conclusion was not adopted.

## Current source baseline and executable verification schedule

Current baseline is `2eee4afebc911482d969ead8452115a4a7f9592d`. The user authorized only B01's three-path commit after fresh red3executed/1pass/2fail/exit101 and final9+5passing auth tests, affected all-target clippy, format/doc/diff checks. Exact tested bytes match the sole-parent commit; no complete remediation result is claimed. Commit association receipt `D:/Luiz/Odradek/opi-remediation-evidence/phase20/apply-20261104T144841-722b4977/b01-commit-association.json` SHA256 `278039b9a57e8e768b8a58cd89d38d32ddf9fad855762ef63d61047e2b14ac42` binds the exact tested input receipt SHA256 `0c7fa52df9d3d2e27ff16d03b2b256ba391eaa2e608015ddf02e185125dfea3e`, authorized three-path scope and sole parent. Current unique archive: `C:\Users\Luiz\AppData\Local\Temp\opi-p20-replan-a8v0alf0\source`; durable tar `D:\Luiz\Odradek\opi-remediation-evidence\phase20\replan-2eee4af-d0303afe\source.tar` SHA256 `2c404dd65986793e318e12ff1f46f086545796363dfad7174f73e55fa6eca62a`;1672regularfiles and both symlinks verified. Source-export receipt `D:\Luiz\Odradek\opi-remediation-evidence\phase20\replan-2eee4af-d0303afe\source-export.json` SHA256 `86bf87f0bb56eb8593a94d6eb45ff7151da25734b130b3d6bfcc97f3c72e7dc6`; applicability receipt `D:\Luiz\Odradek\opi-remediation-evidence\phase20\replan-2eee4af-d0303afe\source-applicability.json` SHA256 `1083cb6f1b12915d911be788d234b4866dec6d9f541a2b717da5ab11193ff3d5`.

Relative to68e2768 only B01 production/test/changelog and fixed plan bytes changed. Manifests/lock/registered sources/ledgers/frozen audit are unchanged. Original red receipts retain actual source, commands, exits and controls; no earlier execution is relabeled. Exact probe source/lock was copied with only its opi-ai manifest path changed to the new archive. Fresh13probe run:6pass/7fail/0ignored/exit101; C001's scoped cases now pass while ADC Referer, nonchat privacy, terminal consistency, retry-cap/overflow and float-null still fail at the same assertions. Receipt `D:\Luiz\Odradek\opi-remediation-evidence\phase20\replan-2eee4af-d0303afe\current-source-probes.json` SHA256 `7dd5e110b52379536c61e5adca713b39af8e12ba86344ef2f037b14721728674`. Fresh8SDKprobe run:3pass/5fail/0ignored/exit101, retaining exact budget/default-store/modality/received-fact failures and controls. Receipt `D:\Luiz\Odradek\opi-remediation-evidence\phase20\replan-2eee4af-d0303afe\current-sdk-probes.json` SHA256 `eba710d479b0878de9300df70ae01ca1ba86b534c26ab27953451d8eea58da57`. These are source probes, not package/native acceptance.

The extracted Windows `.claude/skills` directory symlink required rematerialization before isolated documentation checking. Original setup check exit1 and corrected check exit0 are retained; all1672regularfiles and logical symlink targets still match the unchanged tar. Receipt `D:/Luiz/Odradek/opi-remediation-evidence/phase20/replan-2eee4af-d0303afe/export-alias-rematerialization.json` SHA256 `2a6547b40db23a9cc478c740b17bd670098db8194352323990e331bdda256411`. This was archive alias setup, not behavioral red or a product change.

The strict union remains28dispositions:25fixes/3no-actions,98formal batches, with C018's71batch conjunction and156criterion records unchanged. B13 precedes B02; B14 precedes B03; explicit edges cover their other full-suite consumers. Current openrouter feasibility execution42pass/1fail/0ignored/exit101 establishes the B14 prerequisite. It is no incidental defect or permitted failed gate.

Each `Verification union` and local `Green-after` is an immediate gate: all its checks must pass before advancing. Introduce each batch's own planned assertions and retain existing controls. Mandatory final-source cross-validation executes at its actual downstream owner, avoiding forward cycles without dropping checks:

| Local root | Mandatory downstream proof owner / source closure |
|---|---|
| B22-B27 | C009 requires all six local predicates; final workspace and B91/B17/B95/B92 integration/package/native proof remain mandatory later. |
| B90 | Admission/filesystem/negative checks gate B90; actual B91/B17 integration is already explicitly a later obligation. |
| B09 | Local SDK/product/snapshot/default-owner checks first; B19/final workspace and B17/B95/B92 final package/native coverage remain mandatory. |
| B98 | Local floor/state/ACK/product/caller/workspace checks first; B17/B95 consumers and B92 native evidence are later mandatory C018 conjuncts. |
| B96 | Local identity/public-consumer/workspace checks first; B17/B95 final artifacts and B92 native execution remain mandatory C018 conjuncts. |
| B99 | Local CONNECT/HTTP/proxy/TLS/WS/caller/workspace checks first; B91 ordinary/union families, B17/B95 consumers and B92 native1.97.1 remain mandatory C018 conjuncts. |

A downstream failure blocks the complete result/source conjunction. No local pass substitutes for final candidate bytes, package/graph identity, named executed counts or native provenance. B92 still needs Linux/macOS/Windows; the user will provide macOS access at final commit/CI stage. That is a materialization boundary, not native green or publication/dispatch authority. New fixed plan bytes require a new exact apply invocation; the old digest is not approval for this plan.

## Current Finding Verification

| Source run / Finding ID | Verification and counter-evidence | Source and final severity + rationale | Closure key/family | Batch | Decision / remaining obligations |
|---|---|---|---|---|---|
| C001 / P20-AUD-001 | Refuted at current remediation head only: authorized B01 commit, exact tested-source association and current-archive nonempty/empty wire controls. Original observation remains valid at audit head. | Major -> Major; original impact retained; no new root defect at this source baseline. | auth.stored-scoped-env-candidate / auth.selected-source | none | no-action:already-repaired-at-remediation-head; linked broad auth/package/native gates remain mandatory. |
| C002 / P20-AUD-002 | Confirmed: http.rs:382-399 leaves reqwest referer default enabled; native.rs:352-379 builds default ADC GET client through it;393-408 dispatches. readiness-transport-auth31-37 explicitly identifies unwanted reqwest Referer vs source behavior and frozen-source header ownership. Counter-evidence:  | Major -> Major; Mandatory existing behavior fails at a public/consumer boundary; current independent evidence supports original Major severity. | http.adc-no-unsolicited-referer / http.source-redirect | B02 | fix:disable-automatic-referer-generation; No additional planning obligation; green-after still required. |
| C003 / P20-AUD-003 | Confirmed: openrouter_images.rs:335-341,469-486,509-530 copies raw JSON message to error_message and sanitized typed summary; images.rs:221-238 retains raw message. cloudflare.rs:203-216 joins raw envelope strings; system_one.rs parse/failure paths and classifier.rs:301-315 retain them. ProviderErrorSummary::from_untrusted649-655 intentionally discards upstream bytes; sanitized666-674 uses http::safe_excerpt551 pattern-only defense. SDK contracts189, RDF-06 and V-13 require redacted typed diagnostic defaults. Counter-evidence:  | Major -> Major; Mandatory existing behavior fails at a public/consumer boundary; current independent evidence supports original Major severity. | diagnostics.nonchat-untrusted-body / diagnostics.provider-failure | B03 | fix:close-nonchat-untrusted-error-summary-boundary; No additional planning obligation; green-after still required. |
| C004 / P20-AUD-004 | Confirmed: models.rs:1843-1851 wraps preparation failure using with_projection; provider.rs:899-908 preserves it in StreamFailure. harness.rs:100-126 only direct CredentialNeeded/Revoked/AccountIdMissing; interactive766, rpc1100, runner840/882/919 all call it. Counterevidence interactive_auth269 recurses only for presentation; stream collector unwrap is a different boundary. continuation_contract121-159 stops at direct-variant assertion before steering check. Current test-only archive executes actual private product classifier after public Models.prepare_call; three independent missing classifications reproduced. Input manifest compares893 crate/build files; only appended cfg(test) module differs. Counter-evidence:  | Major -> Major; Mandatory existing behavior fails at a public/consumer boundary; current independent evidence supports original Major severity. | product.auth-preparation-classification / auth.consumer-migration | B04 | fix:classify-auth-causes-through-streamfailure-envelope; No additional planning obligation; green-after still required. |
| C005 / P20-AUD-005 | Confirmed: stream.rs:850-864 matches Done.reason only and returns unchanged message; public ChatStream::new exposes custom producer seam. RDF-03 line74 and V-03/40 forbid Ok(Error/Aborted), require one error and permanent EOF with retained terminal. Counter-evidence:  | Major -> Major; Mandatory existing behavior fails at a public/consumer boundary; current independent evidence supports original Major severity. | stream.terminal-reason-consistency / stream.owned-outcome | B05 | fix:reject-contradictory-terminal-reasons; No additional planning obligation; green-after still required. |
| C006 / P20-AUD-006 | Confirmed: retry.rs:594-615 always admits retryable hint then723 clamps it; Smithy branch exits earlier at572 and owns different rule. SDK contracts186: above60s immediate fail, cap0 unlimited; acceptance V-05 preserves distinct Smithy behavior. Counter-evidence:  | Major -> Major; Mandatory existing behavior fails at a public/consumer boundary; current independent evidence supports original Major severity. | retry.helper-server-hint-cap / retry.acquisition | B06 | fix:refuse-helper-hints-above-cap; No additional planning obligation; green-after still required. |
| C007 / P20-AUD-007 | Confirmed: retry.rs:1010-1032 lacks overflow veto and broad pattern500 matches token count. message.rs:777 exposes existing is_context_overflow classifier; current matrix V-57 at220 explicitly excludes overflow/auth/config before retry. SDK contract189 separates recognition from policy. Counter-evidence:  | Major -> Major; Mandatory existing behavior fails at a public/consumer boundary; current independent evidence supports original Major severity. | retry.whole-call-overflow-veto / retry.whole-call-classification | B07 | fix:veto-context-overflow-before-transient-patterns; No additional planning obligation; green-after still required. |
| C008 / P20-AUD-008 | Confirmed: models.rs3930-3985 never adds B to C; at3984 clamps B to max_tokens-1024 anthropic.rs1389-1397 sends request.max_tokens unchanged; native options remain separate SDK contracts165 fixes numeric Claude formula F=context-clamp(min(C+B,modelcap)), Bprime=min(B,max(0,F-1024)) Counter-evidence: provider_collection.rs1939-1981 tests saturated/tiny output ceilings and cannot distinguish the missing addition Anthropic native budget defaults and adaptive effort are different branches | Major -> Major; Retain source Major: models.rs3930-3985 never adds B to C; at3984 clamps B to max_tokens-1024 anthropic.rs1389-1397 sends request.max_tokens unchanged; native options remain separate SDK contracts165 fixes numeric Claude formula F=context-clamp(min(C+B,modelcap)), Bprime=min(B,max(0,F-1024)) Counter-evidence/limits: provider_collection.rs1939-1981 tests saturated/tiny output ceilings and cannot distinguish the missing addition Anthropic native budget defaults and adaptive effort are different branches | simple.numeric-claude.answer-plus-thinking-budget / sdk.simple-budget | B08 | fix:add-numeric-claude-thinking-budget-before-final-context-clamp; No additional planning obligation; green-after still required. |
| C009 / P20-AUD-009 | Confirmed: Retain Major. Exact source pins resolve all public/default/precedence/response-cost choices;15current consumer probes execute4controls pass/11behavioral failures. One source identity now explicitly maps to metadata/default preparation plus5independent roots; source closure is their conjunction, not a shared-struct shortcut. No additional product meaning or fallback-network authority is introduced. | Major -> Major; retained | sdk.anthropic.complete-compat.source-branches / sdk.anthropic-compat | B22, B23, B24, B25, B26, B27 | fix:preserve-and-execute-current-source-anthropic-compat-fields-and-defaults; six independently verified roots; source closes only on full conjunction. |
| C010 / P20-AUD-010 | Partially confirmed: Retain Major for incomplete required public proxy capability and default ADC source routing. Registered L9/L193/L39 permit Rust shape/caller assembly and do not mandate a default-client constructor or rewrap of supplied clients. Existing ALL fallback and actual default ADC have independent real red; required route matrices are specified as future green. | Major -> Major; retained | transport.source-qualified-pi-utility-and-adc / sdk.transport-source | B28, B97 | fix:source-qualified-pi-utility-and-default-adc-conjunction; all decisions sealed; green not-run |
| C011 / P20-AUD-011 | Confirmed: Retain source Major. Existing registered source semantics determine this bounded representation; current independent probes below establish its defect. FAIL:5executed,2passed/3failed,0ignored,exit101; z/a question sends become a/z, equal zeta/alpha probabilities choose alpha, integer-like key enumeration is lexicographic. Source/lock receipt 94bea0954a45a58265cbe8971b56090a50a0238c176b97bf26706991206ab5e1 Linked SDK-CLASSIFIER-032 usage is independently checked:3public positive tests pass with12local requests across all4classifier adapters and image, original audit requirement state is unchanged; usage receipt 16adefbb614d904d3adfb64282b8df598a6c2ed2dd1df87cd6a0ca4aafcb6b5f. | Major -> Major; retained | classifier.source-property-enumeration / sdk.classifier-contract | B19 | fix:preserve-source-classifier-declaration-enumeration; No distinct planning obligation; green-after remains mandatory. |
| C012 / P20-AUD-012 | Confirmed: models.rs630 and681 initialize credential_store=None;2452-2457 login rejects before flow mutation.rs603-617 creates InMemoryCredentialStore and queue with local memory only models.rs2405 register_oauth_route also requires attached store;2262 documents absent by default, contrary to SDK19 Counter-evidence: Injected InMemoryCredentialStore and FakeStore login paths work, but callers must explicitly attach them dynamic_catalogs supplies in-memory catalog state; this finding does not justify adding default persistent or external catalog behavior | Major -> Major; Retain source Major: models.rs630 and681 initialize credential_store=None;2452-2457 login rejects before flow mutation.rs603-617 creates InMemoryCredentialStore and queue with local memory only models.rs2405 register_oauth_route also requires attached store;2262 documents absent by default, contrary to SDK19 Counter-evidence/limits: Injected InMemoryCredentialStore and FakeStore login paths work, but callers must explicitly attach them dynamic_catalogs supplies in-memory catalog state; this finding does not justify adding default persistent or external catalog behavior | models.default-in-memory-credential-store / sdk.models-lifecycle | B09 | fix:initialize-independent-memory-credential-store-in-constructors; No additional planning obligation; green-after still required. |
| C013 / P20-AUD-013 | Confirmed: openrouter_images.rs174-181 copies record.output directly; images preflight validates headers and not output membership SDK123 requires image,text when Text present and image otherwise Counter-evidence: openrouter_fixtures679 and1335 use already-correct metadata ordering, hiding the issue The source contract allows empty successful result; do not conflate request modality fix with stricter output rejection | Major -> Major; Retain source Major: openrouter_images.rs174-181 copies record.output directly; images preflight validates headers and not output membership SDK123 requires image,text when Text present and image otherwise Counter-evidence/limits: openrouter_fixtures679 and1335 use already-correct metadata ordering, hiding the issue The source contract allows empty successful result; do not conflate request modality fix with stricter output rejection | images.source-modalities-normalization / sdk.images-wire | B10 | fix:derive-image-modalities-from-text-capability; No additional planning obligation; green-after still required. |
| C014 / P20-AUD-014 | Confirmed: openrouter_images.rs375 yields acquired parsed JSON;393-399 response callback then failure_from_error before image result conversion failure_from_error490-504 calls ImagesResult::failure, images.rs221-238 zeroes output/response_id/usage/cost SDK123,V51 preserve acquired result on failure/cancel and forbid retry after response handoff Counter-evidence: Existing1721 callback timing test uses empty choices and successful callback; invalid JSON callback count0 is preserved Normal-path usage parsing exists and no retry on callback failure is already correct | Major -> Major; Retain source Major: openrouter_images.rs375 yields acquired parsed JSON;393-399 response callback then failure_from_error before image result conversion failure_from_error490-504 calls ImagesResult::failure, images.rs221-238 zeroes output/response_id/usage/cost SDK123,V51 preserve acquired result on failure/cancel and forbid retry after response handoff Counter-evidence/limits: Existing1721 callback timing test uses empty choices and successful callback; invalid JSON callback count0 is preserved Normal-path usage parsing exists and no retry on callback failure is already correct | images.post-json-callback-retains-received-facts / sdk.images-lifecycle | B11 | fix:preserve-received-image-result-on-response-callback-failure-or-cancel; No additional planning obligation; green-after still required. |
| C015 / P20-AUD-015 | Confirmed: schema/normalize.rs:608-612 accepts numeric zero only through as_u64/as_i64; neighboring boolean branch explicitly handles as_f64. TP-13 transcript-target-proposal.md:61 requires 0/-0 independent of JSON Number storage. Public normalize retains raw and performs final original-schema validation, so missing conversion yields visible Invalid. Counter-evidence: Integer zero/false/empty-string are valid controls; nonzero and literal strings null/0/false remain invalid; existing final root validation is correct. | Major -> Major; Retain source Major: schema/normalize.rs:608-612 accepts numeric zero only through as_u64/as_i64; neighboring boolean branch explicitly handles as_f64. TP-13 transcript-target-proposal.md:61 requires 0/-0 independent of JSON Number storage. Public normalize retains raw and performs final original-schema validation, so missing conversion yields visible Invalid. Counter-evidence/limits: Integer zero/false/empty-string are valid controls; nonzero and literal strings null/0/false remain invalid; existing final root validation is correct. | schema.null-coercion-float-zero / schema.primitive-coercion | B12 | fix:accept-floating-zero-for-null-coercion; No additional planning obligation; green-after still required. |
| C016 / P20-AUD-016 | Confirmed: Retain Confirmed/Major and all original source observations. Current exact3failures are exhaustively mapped to narrower live P002/P003; administrative exclusion is only no additional change. This neither grants requirement-met status nor waives either independently mandatory repair. Final3-check predicate remains required. | Major -> Major; retained | audit.c016-observations-exactly-covered / tests.stale-contract-expectations | — | no-action:exact-observations-covered-by-p002-and-p003; No distinct planning obligation; green-after remains mandatory. |
| C017 / P20-AUD-017 | Confirmed: AIR-050 spec-draft.md:215 and acceptance-matrix.md:473 require actual outside-workspace package consumption, four ordinary entrances, direct/custom, tool loop, serde, Send/lifetime, two instances/shared store/cleanup. external_sdk.rs is a workspace integration target. Source search of scripts/pi-sdk-conformance Python/Rust/manifests finds no runner that copies/includes that suite. run_readme_examples.py:549-689 builds actual .crate consumers but check_report demands exactly three onboarding flows; package_slice.py family templates cover crypto/http/ws separately. Copying external_sdk.rs alone is incomplete: source uses complete_simple and one negative native complete, without successful stream/stream_simple or a custom ChatApi implementation/Send assertion. Counter-evidence: Existing true package consumers are useful genuine proof for their own bounded families; this finding does not deny their existence. Existing assembled suite covers meaningful wire/typed image/classifier/manual-loop/two-instance behavior inside workspace. | Major -> Major; Retain source Major: AIR-050 spec-draft.md:215 and acceptance-matrix.md:473 require actual outside-workspace package consumption, four ordinary entrances, direct/custom, tool loop, serde, Send/lifetime, two instances/shared store/cleanup. external_sdk.rs is a workspace integration target. Source search of scripts/pi-sdk-conformance Python/Rust/manifests finds no runner that copies/includes that suite. run_readme_examples.py:549-689 builds actual .crate consumers but check_report demands exactly three onboarding flows; package_slice.py family templates cover crypto/http/ws separately. Copying external_sdk.rs alone is incomplete: source uses complete_simple and one negative native complete, without successful stream/stream_simple or a custom ChatApi implementation/Send assertion. Counter-evidence/limits: Existing true package consumers are useful genuine proof for their own bounded families; this finding does not deny their existence. Existing assembled suite covers meaningful wire/typed image/classifier/manual-loop/two-instance behavior inside workspace. | sdk.external-package-assembled-matrix / sdk.delivery-proof | B17 | fix:execute-assembled-sdk-matrix-from-extracted-package; No additional planning obligation; green-after still required. |
| C018 / P20-AUD-018 | Partially confirmed: Retain Major. All156 exact source criteria and all runtime/test/package/native roots are specified. Real WS/sampling/duration/Radius/CONNECT failures retain genuine baseline evidence; only registered SDK-AUTH-006 has the exact user-confirmed static missing-input baseline. Source owner reconciliation is completed. No planning decision or remaining obligation is outstanding; all repaired-byte green is future execution. | Major -> Major; retained | delivery.exact-156-criterion-conjunction / sdk.delivery-proof | B29, B30, B32, B33, B34, B35, B36, B37, B38, B39, B40, B41, B42, B43, B44, B45, B46, B47, B50, B51, B52, B53, B54, B55, B57, B58, B59, B60, B61, B63, B64, B65, B66, B67, B68, B69, B70, B71, B72, B73, B74, B75, B76, B77, B78, B79, B80, B81, B82, B83, B84, B85, B86, B87, B88, B89, B90, B94, B48, B49, B56, B98, B31, B62, B96, B99, B91, B17, B95, B92, B93 | fix:complete-exact-criterion-runtime-and-proof-conjunction; no remaining planning obligation; green not-run |
| C019 / P20-AUD-019 | Confirmed: Retain source Minor. Existing registered source semantics determine this bounded representation; current independent probes below establish its defect. FAIL: public store round trip omits promptCache; missing promptCache control passes. Final combined compatibility suite15executed,4controls pass/11behavioral failures; receipt 358c56cbfe385ef9e29c5d6c84bbc3e32d3c8fdf7e5d6174ab0381a6b3a8f5be | Minor -> Minor; retained | model.prompt-cache-duration-metadata / model.complete-record | B20 | fix:retain-typed-prompt-cache-duration-metadata; No distinct planning obligation; green-after remains mandatory. |
| C020 / P20-AUD-020 | Confirmed: Retain source Minor. Existing registered source semantics determine this bounded representation; current independent probes below establish its defect. FAIL:3executed,2passed/1failed,0ignored,exit101; returned and persisted public metadata contain9999-12-31. Actual fake token exchange/store ACK occurred; no immediate key-loss claim. Receipt 158695e0633e29acc04421178ed1de463c536cee647e980ddf26a19bb8211a9d | Minor -> Minor; retained | oauth.openrouter-typed-permanence / auth.credential-lifetime | B21 | fix:represent-oauth-permanence-with-typed-expiry; No distinct planning obligation; green-after remains mandatory. |
| C021 / P20-AUD-021 | Confirmed: At line334 English says capacity below required retry cost blocks retry; Chinese says capacity zero blocks retry. Both preserve initial500, cost5/transient10 and per-call ownership. Counter-evidence: Chinese does not explicitly permit positive insufficient quota, so this is incomplete precision, not evidence of runtime defect or contradictory intentional policy. | Minor -> Minor; Retain source Minor: At line334 English says capacity below required retry cost blocks retry; Chinese says capacity zero blocks retry. Both preserve initial500, cost5/transient10 and per-call ownership. Counter-evidence/limits: Chinese does not explicitly permit positive insufficient quota, so this is incomplete precision, not evidence of runtime defect or contradictory intentional policy. | docs.bedrock-quota-bilingual-boundary / docs.source-contract-equivalence | B18 | fix:translate-below-retry-cost-boundary; No additional planning obligation; green-after still required. |
| P001 / P20-AUD-001 | Confirmed: Retain Major. Current owning Linux/MSRV paired test independently reproduces default pooled connection failure; advertising the already-existing scripted close removes it without disabling pooling, changing production redirects or weakening20/21-hop assertions. Earlier Windows pass remains valid host-specific counter-evidence. | Major -> Major; retained | tests.redirect-peer-close-fidelity / http.source-redirect | B13 | fix:advertise-scripted-redirect-connection-close; No distinct planning obligation; green-after remains mandatory. |
| P002 / P20-AUD-002 | Confirmed: Both current assertions require true for an unannotated tool. openai_chat.rs:2845 default_strict=Some(false); README capability table and pinned openai-completions.ts:1507-1515 distinguish capability from requested strictness. Counter-evidence: Native Mistral Conversations has its own independent defect P005; a shared Chat test change must not alter that adapter. | Major -> Major; Retain source Major: Both current assertions require true for an unannotated tool. openai_chat.rs:2845 default_strict=Some(false); README capability table and pinned openai-completions.ts:1507-1515 distinguish capability from requested strictness. Counter-evidence/limits: Native Mistral Conversations has its own independent defect P005; a shared Chat test change must not alter that adapter. | tests.compat-strict-capability-versus-intent / tests.stale-contract-expectations | B14 | fix:correct-shared-chat-strict-default-assertions; No additional planning obligation; green-after still required. |
| P003 / P20-AUD-003 | Confirmed: openai_chat.rs:3034-3065 emits configured affinity based on session_id, independently from prompt-cache retention. request_enrichment.rs:435 expects absent headers; exact test fails. Pinned source openai-completions.ts:778-788 agrees with implementation. Counter-evidence: prompt_cache_key is still retention-gated. Retaining headers does not authorize cache insertion. | Major -> Major; Retain source Major: openai_chat.rs:3034-3065 emits configured affinity based on session_id, independently from prompt-cache retention. request_enrichment.rs:435 expects absent headers; exact test fails. Pinned source openai-completions.ts:778-788 agrees with implementation. Counter-evidence/limits: prompt_cache_key is still retention-gated. Retaining headers does not authorize cache insertion. | tests.affinity-independent-of-cache-retention / tests.stale-contract-expectations | B15 | fix:retain-affinity-when-cache-retention-disabled; No additional planning obligation; green-after still required. |
| P004 / P20-AUD-004 | Partially confirmed: models.rs:1843-1851 wraps preparation failure using with_projection; provider.rs:899-908 preserves it in StreamFailure. harness.rs:100-126 only direct CredentialNeeded/Revoked/AccountIdMissing; interactive766, rpc1100, runner840/882/919 all call it. Counterevidence interactive_auth269 recurses only for presentation; stream collector unwrap is a different boundary. continuation_contract121-159 stops at direct-variant assertion before steering check. Current test-only archive executes actual private product classifier after public Models.prepare_call; three independent missing classifications reproduced. Input manifest compares893 crate/build files; only appended cfg(test) module differs. Private actual product probe5 executed:2controls pass,3nested auth classifications fail; input-identity.json binds test-only append and unchanged production prefix. Counter-evidence: Projection-bearing StreamFailure is required at the SDK boundary; its existence is not itself the defect. The actual product classifier fails three nested auth causes. The current continuation test fails before steering assertions; no steering loss was observed. Preserve the envelope and migrate the consumer/old assertion. | Major -> Major; Mandatory existing behavior fails at a public/consumer boundary; current independent evidence supports original Major severity. | product.auth-preparation-classification / auth.consumer-migration | B04 | fix:classify-auth-causes-through-streamfailure-envelope; No additional planning obligation; green-after still required. |
| P005 / P20-AUD-005 | Confirmed: mistral.rs:446-475 defaults strict to true and converts unannotated schema. Pinned mistral-conversations.ts:777-789 sends strict ?? false and original parameters when unconstrained. No registered difference admits the true default. Counter-evidence: Existing strict-default-true test and both READMEs lock/document implementation behavior; they do not override registered source wire semantics. Explicit Require positive control passes. | Major -> Major; Retain source Major: mistral.rs:446-475 defaults strict to true and converts unannotated schema. Pinned mistral-conversations.ts:777-789 sends strict ?? false and original parameters when unconstrained. No registered difference admits the true default. Counter-evidence/limits: Existing strict-default-true test and both READMEs lock/document implementation behavior; they do not override registered source wire semantics. Explicit Require positive control passes. | mistral.unconstrained-tool-source-default / sdk.mistral-wire | B16 | fix:use-false-default-with-original-unconstrained-schema; No additional planning obligation; green-after still required. |
| P006 / P20-AUD-006 | Confirmed: Retain original Minor and Confirmed verification. Explicitly authorized opi-implement P0/P2/P4 then confirm-all/write-only installed the live source binding:16/16current hashes match. Historical Phase20 snapshot/source identity and68summary entries remain immutable, as required; their old source hash is not a live drift repair target. | Minor -> Minor; retained | workflow.registered-source-hash-reconciliation / delivery.authority | — | no-action:reconciled-by-explicit-opi-implement-owner; all decisions sealed; green not-run |
| P007 / P20-AUD-007 | Partially confirmed: Retain source Major: Exact indexed-appendix candidate phase20-delivery-evidence.md SHA-2565217b9f0228d1dad64aa19e23ad4fc591923ba7059b8e3a839046085f8447585 was verified. Its earlier tables cite20.57/20.58 inputs. Its later lines157-193 record CI run41300159581 at9e8d915 on2026-11-01, including macOS native workspace tests and MSRV. Thus the broad claim of never-dispatched CI or no recorded macOS execution is contradicted by the candidate itself. git diff --name-only 9e8d915 d053965b8ef3bca8d79990793459bb0ba87fe69c -- crates Cargo.toml Cargo.lock .github lists changed SDK/Agent production and test inputs. Current exact tests reproduce4failures. No final-byte applicability or original remote CI raw proof was established. Counter-evidence/limits: Recorded earlier CI success is acknowledged, not promoted to current closure or an independently checked remote run. Passing Windows redirect case narrows the current failure set. | Major -> Major; retained | delivery.final-source-artifact-native-conjunction / sdk.delivery-proof | B90, B91, B17, B95, B92, B93 | fix:restore-final-source-package-and-native-proof; all closure gates specified; green not-run |

## Unresolved Decisions

None. D2 is resolved by the source-qualified public utility/assembly design and real common-domain red; D5 by exact existing Radius input/identity red, ordinary owned CONNECT design/negative handoff proof, and the explicitly user-confirmed one-input static baseline; D8 by the separately invoked owner P0/P2/P4 and write-only install. Future repaired-byte green is not-run, with concrete tests and ownership below. There is no partial-apply branch.

## Planning Evidence Work

All runtime probes used synthetic local peers/fake credentials and no paid or live provider. Toolchain was rustc1.99.1, host x86_64-pc-windows-msvc. The persistent target was `E:\opi\cargo-targets\opi-72fa37a0fa044958`; quick status and coordinated cache observation/leases were used. No clean/cache deletion/per-session target was used. External lock registry tuples/checksums match the committed lock; unused workspace packages are omitted, not upgraded. Initial probe compilation or fixture-setup failures are retained in temporary logs but are not behavioral red evidence.

| Question | Fixed source/build and actual command | Positive/negative controls | Distinguishing current observation | Dependent decisions |
|---|---|---|---|---|
| Do scoped identity, redirect privacy, nonchat diagnostics, terminal consistency, retry admission/veto and float-null coercion obey current contracts? | python D:/Luiz/Odradek/opi/scripts/opi-cargo-cache.py run --target E:/opi/cargo-targets/opi-72fa37a0fa044958 -- test --manifest-path C:/Users/Luiz/AppData/Local/Temp/opi-p20-plan-auth-probes-08ct7l_l/Cargo.toml --offline --locked --test probes -- --nocapture; archive d053965b8ef3bca8d79990793459bb0ba87fe69c | 4 standalone controls; per-probe count/status/header assertions. The final corrected image fixture made an actual request. | 13 executed:4passed/9failed/0ignored; exit101. Includes C015; -0.0 follows failed0.0 and was not reached. | C001–007 and C015 (C004 separately below) |
| Do simple budget/default store/image modalities/received result retention match their source contracts? | python C:\Users\Luiz\AppData\Local\Temp\opi-phase20-remediate-plan-jqdki4yk\source\scripts\opi-cargo-cache.py run --target E:\opi\cargo-targets\opi-72fa37a0fa044958 -- test --manifest-path C:\Users\Luiz\AppData\Local\Temp\opi-phase20-remediate-plan-jqdki4yk\sdk-current-probes\Cargo.toml --offline --locked --lib -- --nocapture; same archive | 3 positive controls: no-thinking, injected memory store, ordinary successful image result. | 8 executed:3passed/5failed/0ignored; exit101. Both callback error and cancel discard received facts after1callback/1request. | C008/C012/C013/C014 |
| Does actual private product classifier inspect projected prepare errors? | python scripts/opi-cargo-cache.py run --target E:/opi/cargo-targets/opi-72fa37a0fa044958 -- test -p opi-coding-agent --lib --offline --locked remediation_auth_wrapper_probe -- --nocapture; cwd C:\Users\Luiz\AppData\Local\Temp\opi-p20-product-auth-red-0qz8ymva\source | 2controls: direct auth variants and wrapped non-auth. Actual Models.prepare_call creates each wrapped error; projection and0dispatch checked first. | 5 executed:2passed/3failed/0ignored; exit101. Needed/Revoked/AccountIdMissing all classified None. | C004/P004; preserve envelope |
| Does Mistral unannotated-tool wire preserve source default/schema? | E:\opi\cargo-targets\opi-72fa37a0fa044958\debug\deps\mistral-ca7cac90210ef694.exe mistral_43::plan_ --nocapture; executable SHA 42f2ce2e3063783c437c4926380b75e4fcb3b1d82c7e4f56c8850fc68fe863e3 | Explicit Require case passes; two unannotated cases independently observe strict and parameter conversion. | 3 executed:1passed/2failed/0ignored; exit101. Cargo-built source-bound binary replay records original exit separately. | P005 |
| Does current http_acceptance exact case reproduce? | python scripts/opi-cargo-cache.py run --target E:\opi\cargo-targets\opi-72fa37a0fa044958 -- test -p opi-ai --test http_acceptance --offline --locked source_qualified_default_redirect_budget_has_exact_twenty_hop_boundary -- --exact --nocapture | Existing source-case controls unchanged; no assertion edited. | test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 20 filtered out; finished in 0.07s; exit0 | P001: host-limited result |
| Does current mistral_fixtures exact case reproduce? | python scripts/opi-cargo-cache.py run --target E:\opi\cargo-targets\opi-72fa37a0fa044958 -- test -p opi-ai --test mistral_fixtures --offline --locked mistral_profile_inherits_shared_compat_flags -- --exact --nocapture | Existing source-case controls unchanged; no assertion edited. | thread 'mistral_profile_inherits_shared_compat_flags' (33840) panicked at crates\opi-ai\tests\mistral_fixtures.rs:537:5:; test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 51 filtered out; finished in 0.00s; exit101 | P002/C016 |
| Does current openrouter_fixtures exact case reproduce? | python scripts/opi-cargo-cache.py run --target E:\opi\cargo-targets\opi-72fa37a0fa044958 -- test -p opi-ai --test openrouter_fixtures --offline --locked openrouter_profile_inherits_shared_compat_flags -- --exact --nocapture | Existing source-case controls unchanged; no assertion edited. | thread 'openrouter_profile_inherits_shared_compat_flags' (50344) panicked at crates\opi-ai\tests\openrouter_fixtures.rs:490:5:; test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 42 filtered out; finished in 0.00s; exit101 | P002/C016 |
| Does current request_enrichment exact case reproduce? | python scripts/opi-cargo-cache.py run --target E:\opi\cargo-targets\opi-72fa37a0fa044958 -- test -p opi-ai --test request_enrichment --offline --locked session_affinity_wire_mappings -- --exact --nocapture | Existing source-case controls unchanged; no assertion edited. | thread 'session_affinity_wire_mappings' (61488) panicked at crates\opi-ai\tests\request_enrichment.rs:435:9:; test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 31 filtered out; finished in 0.01s; exit101 | P003/C016 |
| Does current continuation_contract exact case reproduce? | python scripts/opi-cargo-cache.py run --target E:\opi\cargo-targets\opi-72fa37a0fa044958 -- test -p opi-agent --test continuation_contract --offline --locked public_retry_retains_initial_steering_after_auth_preparation_failure -- --exact --nocapture | Existing source-case controls unchanged; no assertion edited. | thread 'public_retry_retains_initial_steering_after_auth_preparation_failure' (50672) panicked at crates\opi-agent\tests\continuation_contract.rs:140:5:; test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 14 filtered out; finished in 0.01s; exit101 | C004/P004; shape assertion only |

Initial ledger-field projection -> verify: both stored acceptance-matrix hashes were0c05c6a7… versus committed c8315863…; only required source identity fields were exposed. The separately authorized owner has since reconciled the live binding to c8315863…, preserving the frozen historical identity and68summaries. Candidate raw proof -> verify: `docs/research/pi-ai-opi-ai-alignment/phase20-delivery-evidence.md` exactly hashes5217b9f0228d1dad64aa19e23ad4fc591923ba7059b8e3a839046085f8447585. Its later CI section contradicts the unqualified missing-macOS/dispatch claim, but `git diff --name-only 9e8d915 d053965b8ef3bca8d79990793459bb0ba87fe69c -- crates Cargo.toml Cargo.lock .github` establishes changed relevant inputs. No remote CI raw logs were fetched or adopted. D5 preserves final-byte applicability obligations.

### Evidence locations and byte identities

These are this planning run’s retained local raw artifacts, not additional authority or prior-plan inputs. Before any later use verify exact bytes, source/dependency applicability and executed counts. A durable outside-repository copy is sealed at `D:\Luiz\Odradek\opi-remediation-evidence\phase20\remediation-20261104-d053965b` with `D:\Luiz\Odradek\opi-remediation-evidence\phase20\remediation-20261104-d053965b\manifest.json` SHA256 `f94adbcfc29771a35dda4462ffccc8334a88ff87a153faf56007b74c8ae29494` (230files). The manifest maps cited TEMP roots to identical copied bytes; original source.tar archives plus cfg(test) patches reconstruct the exported sources. A missing original may use its verified identical durable copy, never an assumed pass. Build caches and Windows aliases are not copied.

| Artifact | SHA-256 |
|---|---|
| `C:\Users\Luiz\AppData\Local\Temp\opi-p20-plan-auth-probes-08ct7l_l\tests\probes.rs` | `c3395dd8db0aefdcb9ba21f23eda089d231f4fcb216341876dd9fb3ff7a76846` |
| `C:\Users\Luiz\AppData\Local\Temp\opi-p20-plan-auth-probes-08ct7l_l\Cargo.toml` | `75fe505b03c14e197426e4d10ae6b3205c20b4c915f8e38fae25f81ffb569ce3` |
| `C:\Users\Luiz\AppData\Local\Temp\opi-p20-plan-auth-probes-08ct7l_l\Cargo.lock` | `d66e18206a4bb2a536b0ca791ce9fbda82ccc03c82594c84ac53299d70b588f6` |
| `C:\Users\Luiz\AppData\Local\Temp\opi-p20-plan-auth-probes-08ct7l_l\probe-final.log` | `40a9ea4da50524564f14160f6e2fe8834309967bf2d824b75d65fd136e7b4aeb` |
| `C:\Users\Luiz\AppData\Local\Temp\opi-p20-plan-auth-probes-08ct7l_l\probe-identities.json` | `729c7f69cb270f7ebea9584533c16f2efde6e19772b0ce2e681417fb07f5a81e` |
| `C:\Users\Luiz\AppData\Local\Temp\opi-phase20-remediate-plan-jqdki4yk\sdk-current-probes\src\lib.rs` | `bf7550031e3f79cd651e2def57f34afd92c64147fc11c6518fbdfdaf7c0436d8` |
| `C:\Users\Luiz\AppData\Local\Temp\opi-phase20-remediate-plan-jqdki4yk\sdk-current-probes\Cargo.toml` | `ba1e1d7899888bec9c0d9cd2c8045da237359b8f4fece4b9718e89cdf2777198` |
| `C:\Users\Luiz\AppData\Local\Temp\opi-phase20-remediate-plan-jqdki4yk\sdk-current-probes\Cargo.lock` | `49339f26befa43a7bcd3884700b10197d9d66a5674ec6ba2ccf16794a012d57e` |
| `C:\Users\Luiz\AppData\Local\Temp\opi-phase20-remediate-plan-jqdki4yk\sdk-current-probes.log` | `f40b5755553c607493bc72cc3d77734e4e3234da194f7b9141ff516eccb20304` |
| `C:\Users\Luiz\AppData\Local\Temp\opi-phase20-remediate-plan-jqdki4yk\sdk-current-probe-summary.json` | `f1206fa1b7d4cddcd731eca71e1d99ef9e8556f7c2ff5baa9b18d6fe82565f1f` |
| `C:\Users\Luiz\AppData\Local\Temp\opi-p20-product-auth-red-0qz8ymva\test-only.patch` | `5fb8e59f454a188852cd68a07f3e6fa55759cb4ad7bb6f41d8a5c8d0926a702e` |
| `C:\Users\Luiz\AppData\Local\Temp\opi-p20-product-auth-red-0qz8ymva\input-identity.json` | `1903eb46a184bf6ef3d7baaf3919457eb354a23065a33849eb4a234a2dac4489` |
| `C:\Users\Luiz\AppData\Local\Temp\opi-p20-product-auth-red-0qz8ymva\tested-source-manifest.json` | `6cc87ebd14f9ebf59c2dd2e89de3ce3ff1a60ac7c7a3c283b54c5a2e2591a703` |
| `C:\Users\Luiz\AppData\Local\Temp\opi-p20-product-auth-red-0qz8ymva\product-auth-red.log` | `7ff69b892cb168bb78fdbdc3d3d7fbb360ffbb6dbc1a61d7906acb549834ad9f` |
| `C:\Users\Luiz\AppData\Local\Temp\opi-phase20-remediate-plan-jqdki4yk\mistral-probe\tests\mistral.rs` | `6a77824f42235473f23d1d06c0e12fb3e9bfc0cebbdb6e6b3822284344fef42c` |
| `C:\Users\Luiz\AppData\Local\Temp\opi-phase20-remediate-plan-jqdki4yk\mistral-probe\probe-final.log` | `c7b9a721002f0febe7674f4a211be2ec375c2d0b4690151783a7c66ac6b12030` |
| `C:\Users\Luiz\AppData\Local\Temp\opi-phase20-remediate-plan-jqdki4yk\mistral-probe\executed-final.log` | `e15c87bfcd7f164082fa871a8debd09739b545ff995694007fe45b5cb36f2abf` |
| `C:\Users\Luiz\AppData\Local\Temp\opi-phase20-remediate-plan-jqdki4yk\mistral-probe\execution-summary.json` | `de84516a1f07584d991de4d7c8bfa28ea6e9a495b1b415a6b43bb59bb010006c` |
| `C:\Users\Luiz\AppData\Local\Temp\opi-phase20-remediate-plan-jqdki4yk\root-checks\http_acceptance.log` | `343e09bfc806261a2bed184e865dfed83d6ca14e6c6634f57e9d3eede0367038` |
| `C:\Users\Luiz\AppData\Local\Temp\opi-phase20-remediate-plan-jqdki4yk\root-checks\mistral_fixtures.log` | `4dd1b128a0e71e4673481e45c7d5f608fcd03059b740e45e97d0813d8c632f63` |
| `C:\Users\Luiz\AppData\Local\Temp\opi-phase20-remediate-plan-jqdki4yk\root-checks\openrouter_fixtures.log` | `3f6d69474db6f9e120ef29e36b116b8ea8ca5f0a414f6c9e2d457e3f47b1334c` |
| `C:\Users\Luiz\AppData\Local\Temp\opi-phase20-remediate-plan-jqdki4yk\root-checks\request_enrichment.log` | `d44501444dac7840fe84cac56a5f3d56ba434197bbbff5403a5b7400aa8df46a` |
| `C:\Users\Luiz\AppData\Local\Temp\opi-phase20-remediate-plan-jqdki4yk\root-checks\continuation_contract.log` | `364949b23d6998ea4bae21c71e17ea306f0d6eea392b84dc810f1beac92380ba` |

## Closure Batches

All batches below are the sealed plan; execution requires a new explicit apply invocation containing its validator-emitted digest. Independent dependency-ready Major batches precede Minor documentation work. Same-file edits remain sequential; shared file/family membership does not merge predicates. B04 alone shares an identical root/predicate across two source identities. B17 packages the final approved source state after all source/test-affecting batches, including its Minor metadata/lifetime prerequisites. C016 requests no additional batch; C009 has six actual roots, and C018 has its complete finite156-criterion mapping and source-level conjunction below.

For each behavioral batch, migrate the sealed distinguishing temporary assertions and controls into the listed owning repository tests before production edits, run them red, then rerun them green. The original observed external red remains bound above. If replaying an external probe, change only its opi-ai path to the actually tested candidate export, retain probe source bytes/registry identities, rebuild and record actual source/executable hashes; green against the old archive cannot close a fix. All newly created/modified test binaries or filters must actually execute. Preserve raw input/known usage/privacy and forbid unapproved fallback/retry. Read the full current Unreleased section before additive entries; released history is immutable.

### Batch B13: advertise scripted redirect connection close

**Closure predicate**: The scripted redirect response declares its actual close lifetime; both explicit and genuine default pooled clients pass20 hops and reject21 with exactly21 sends and typed too-many-redirects, with no pooling-disable workaround.
**Dependencies**: []
**Verification union**: cargo test -p opi-ai --test http_acceptance --offline --locked on current native host; exact source_qualified_default_redirect_budget_has_exact_twenty_hop_boundary also on owning native Linux rustc1.97.1, genuine default pooling and unchanged20/21-hop assertions; retain raw commands/counts/host/cache identities; cargo clippy -p opi-ai --all-targets --offline --locked -- -D warnings

#### Fix B13.1: advertise scripted redirect connection close

- **Finding source(s)**: phase20-pi-glm53-44374ec-20261103t160703z / f8c06b50b3c51c95ac5b1309e72a8b31d06c34785ad588ed4e42bff19d914990 / P20-AUD-001
- **Decision**: fix:advertise-scripted-redirect-connection-close
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-ai/tests/http_acceptance.rs`
- **Change kind**: test-only
- **Change**: Add Connection: close to the scripted redirect response; retain default pooling and all20/21-hop send/body/auth assertions. Do not change HttpClient behavior.
- **Closure predicate**: The scripted redirect response declares its actual close lifetime; both explicit and genuine default pooled clients pass20 hops and reject21 with exactly21 sends and typed too-many-redirects, with no pooling-disable workaround.
- **Red-before**: python3 scripts/opi-cargo-cache.py run --target /mnt/e/opi/cargo-targets/opi-8abcf38f9871653e -- test -p opi-ai --test http_acceptance --offline --locked source_qualified_default_redirect_budget_has_exact_twenty_hop_boundary -- --exact --nocapture; cwd /tmp/opi-p20-linux-redirect-vww67bxm/source -> FAIL baseline0passed/1failed on native WSL Linux rustc1.97.1, explicit=false Network; after only scripted redirect Connection: close header1passed/0failed. Product source unchanged. Before log 9f9a3da3bfdb458654010152041090703ff2ce44ed63089f69def12659bd47b7; control log 33ecc72247fdbd512106182f8799834262b76837fc0a347b04d5edbe1872d527
- **Green-after**: cargo test -p opi-ai --test http_acceptance --offline --locked on current native host; cargo +1.97.1 test -p opi-ai --test http_acceptance --offline --locked source_qualified_default_redirect_budget_has_exact_twenty_hop_boundary -- --exact --nocapture on owning native Linux; cargo clippy -p opi-ai --all-targets --offline --locked -- -D warnings -> The scripted redirect response declares its actual close lifetime; both explicit and genuine default pooled clients pass20 hops and reject21 with exactly21 sends and typed too-many-redirects, with no pooling-disable workaround.
- **Test impact**: add/update; retain existing safety and valid-source controls; delete none.

### Batch B02: disable automatic referer generation

**Closure predicate**: Actual ADC default GET source redirect sends exactly source+target, follows source hop budget, removes cross-origin protected auth per current contract, and adds no unsolicited Referer/source query secret; explicitly supplied lawful headers retain their contract.
**Dependencies**: ["B13"]
**Verification union**: cargo test -p opi-ai --test adc_owner_regressions; cargo test -p opi-ai --test http_acceptance; cargo clippy -p opi-ai --all-targets -- -D warnings

#### Fix B02.1: disable automatic referer generation

- **Finding source(s)**: phase20-codex-gpt61-b3f1810-20261103t142820z / 4cec97af8ef6bb773c630ee251728f31212e12fdc1bf9631f10b2dc3084bc0f6 / P20-AUD-002
- **Decision**: fix:disable-automatic-referer-generation
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-ai/src/http.rs`, `crates/opi-ai/tests/adc_owner_regressions.rs`, `crates/opi-ai/tests/http_acceptance.rs`, `CHANGELOG.md`
- **Change kind**: behavioral
- **Change**: Disable reqwest automatic Referer on controlled shared builder using existing builder setting; add actual default ADC cross-port canary regression and explicit-header/no-redirect controls. Do not rewrite redirect authority or change protected-header policy.
- **Closure predicate**: Actual ADC default GET source redirect sends exactly source+target, follows source hop budget, removes cross-origin protected auth per current contract, and adds no unsolicited Referer/source query secret; explicitly supplied lawful headers retain their contract.
- **Red-before**: python D:/Luiz/Odradek/opi/scripts/opi-cargo-cache.py run --target E:/opi/cargo-targets/opi-72fa37a0fa044958 -- test --manifest-path C:/Users/Luiz/AppData/Local/Temp/opi-p20-plan-auth-probes-08ct7l_l/Cargo.toml --offline --locked --test probes -- --nocapture -> FAIL: target Referer = http://127.0.0.1:<source-port>/redirect?subject_token=synthetic-query-canary.
- **Green-after**: cargo test -p opi-ai --test adc_owner_regressions; cargo test -p opi-ai --test http_acceptance -> Actual ADC default GET source redirect sends exactly source+target, follows source hop budget, removes cross-origin protected auth per current contract, and adds no unsolicited Referer/source query secret; explicitly supplied lawful headers retain their contract.
- **Test impact**: add/update; retain existing safety and valid-source controls; delete none.

### Batch B14: correct shared chat strict default assertions

**Closure predicate**: Both profiles assert false for unconstrained tools on capable wires, explicit Require/Prefer retain strict conversion, and unsupported capability preserves the source omission/refusal contract.
**Dependencies**: []
**Verification union**: cargo test -p opi-ai --test mistral_fixtures; cargo test -p opi-ai --test openrouter_fixtures; cargo clippy -p opi-ai --all-targets -- -D warnings

#### Fix B14.1: correct shared chat strict default assertions

- **Finding source(s)**: phase20-pi-glm53-44374ec-20261103t160703z / f8c06b50b3c51c95ac5b1309e72a8b31d06c34785ad588ed4e42bff19d914990 / P20-AUD-002
- **Decision**: fix:correct-shared-chat-strict-default-assertions
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-ai/tests/mistral_fixtures.rs`, `crates/opi-ai/tests/openrouter_fixtures.rs`
- **Change kind**: test-only
- **Change**: Correct the two stale default assertions and add explicit constrained/unconstrained/capability controls while preserving developer-role/max-token assertions.
- **Closure predicate**: Both profiles assert false for unconstrained tools on capable wires, explicit Require/Prefer retain strict conversion, and unsupported capability preserves the source omission/refusal contract.
- **Red-before**: python scripts/opi-cargo-cache.py run --target E:\opi\cargo-targets\opi-72fa37a0fa044958 -- test -p opi-ai --test mistral_fixtures --offline --locked mistral_profile_inherits_shared_compat_flags -- --exact --nocapture; python scripts/opi-cargo-cache.py run --target E:\opi\cargo-targets\opi-72fa37a0fa044958 -- test -p opi-ai --test openrouter_fixtures --offline --locked openrouter_profile_inherits_shared_compat_flags -- --exact --nocapture -> FAIL: both exact cases executed separately,0passed/1failed each; strict false disagrees with old true assertion.
- **Green-after**: cargo test -p opi-ai --test mistral_fixtures; cargo test -p opi-ai --test openrouter_fixtures -> Both profiles assert false for unconstrained tools on capable wires, explicit Require/Prefer retain strict conversion, and unsupported capability preserves the source omission/refusal contract.
- **Test impact**: add/update; retain existing safety and valid-source controls; delete none.

### Batch B03: close nonchat untrusted error summary boundary

**Closure predicate**: For image HTTP400 and classifier HTTP200 success:false with arbitrary synthetic provider text, default result Debug/error_message/typed cause expose no canary while preserving operation identity, Error, HTTP status or typed class, known usage, and zero extra sends.
**Dependencies**: ["B14"]
**Verification union**: cargo test -p opi-ai --test openrouter_fixtures; cargo test -p opi-ai --test classifier_contract; cargo clippy -p opi-ai --all-targets -- -D warnings

#### Fix B03.1: close nonchat untrusted error summary boundary

- **Finding source(s)**: phase20-codex-gpt61-b3f1810-20261103t142820z / 4cec97af8ef6bb773c630ee251728f31212e12fdc1bf9631f10b2dc3084bc0f6 / P20-AUD-003
- **Decision**: fix:close-nonchat-untrusted-error-summary-boundary
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-ai/src/api/openrouter_images.rs`, `crates/opi-ai/src/api/system_one.rs`, `crates/opi-ai/src/api/cloudflare.rs`, `crates/opi-ai/tests/openrouter_fixtures.rs`, `crates/opi-ai/tests/classifier_contract.rs`, `CHANGELOG.md`
- **Change kind**: behavioral
- **Change**: Use existing from_untrusted/closed HTTP status summaries for untrusted provider body/envelope errors at non-chat result producer boundary; populate display and typed diagnostic from same safe representation. Preserve typed status/known usage and locally generated safe messages. Update fixtures that currently demand echoed raw provider text; retain source-control classification parsing privately. Do not rely on safe_excerpt regex alone.
- **Closure predicate**: For image HTTP400 and classifier HTTP200 success:false with arbitrary synthetic provider text, default result Debug/error_message/typed cause expose no canary while preserving operation identity, Error, HTTP status or typed class, known usage, and zero extra sends.
- **Red-before**: python D:/Luiz/Odradek/opi/scripts/opi-cargo-cache.py run --target E:/opi/cargo-targets/opi-72fa37a0fa044958 -- test --manifest-path C:/Users/Luiz/AppData/Local/Temp/opi-p20-plan-auth-probes-08ct7l_l/Cargo.toml --offline --locked --test probes -- --nocapture -> Both image_failure_must_not_echo_body and classifier_failure_must_not_echo_body FAIL after exactly one send each. Both error_message AND typed cause contain the arbitrary canary.
- **Green-after**: cargo test -p opi-ai --test openrouter_fixtures; cargo test -p opi-ai --test classifier_contract -> For image HTTP400 and classifier HTTP200 success:false with arbitrary synthetic provider text, default result Debug/error_message/typed cause expose no canary while preserving operation identity, Error, HTTP status or typed class, known usage, and zero extra sends.
- **Test impact**: add/update; retain existing safety and valid-source controls; delete none.

### Batch B04: classify auth causes through streamfailure envelope

**Closure predicate**: For a projected pre-output credential preparation error, product shared helper classifies nested CredentialNeeded/Revoked/AccountIdMissing exactly as direct variants, enabling same TUI/RPC/CLI remediation; nonauth stays none, partial/projection remain intact, initial steering survives retry.
**Dependencies**: []
**Verification union**: cargo test -p opi-coding-agent --lib remediation_auth_wrapper_probe -- --nocapture; cargo test -p opi-agent --test continuation_contract

#### Fix B04.1: classify auth causes through streamfailure envelope

- **Finding source(s)**: phase20-codex-gpt61-b3f1810-20261103t142820z / 4cec97af8ef6bb773c630ee251728f31212e12fdc1bf9631f10b2dc3084bc0f6 / P20-AUD-004; phase20-pi-glm53-44374ec-20261103t160703z / f8c06b50b3c51c95ac5b1309e72a8b31d06c34785ad588ed4e42bff19d914990 / P20-AUD-004
- **Decision**: fix:classify-auth-causes-through-streamfailure-envelope
- **Verification status**: C004 Confirmed; P004 Partially confirmed (same product defect, narrower SDK-envelope claim)
- **File(s)**: `crates/opi-coding-agent/src/harness.rs`, `crates/opi-agent/tests/continuation_contract.rs`, `CHANGELOG.md`
- **Change kind**: behavioral
- **Change**: Preserve SDK projection envelope. Recurse through StreamFailure in product helper, with direct/nested/auth/nonauth fixture cases in its owning test module; update stale continuation test to inspect leaf typed cause while retaining zero-dispatch and steering continuation assertions.
- **Closure predicate**: For a projected pre-output credential preparation error, product shared helper classifies nested CredentialNeeded/Revoked/AccountIdMissing exactly as direct variants, enabling same TUI/RPC/CLI remediation; nonauth stays none, partial/projection remain intact, initial steering survives retry.
- **Red-before**: python scripts/opi-cargo-cache.py run --target E:/opi/cargo-targets/opi-72fa37a0fa044958 -- test -p opi-coding-agent --lib --offline --locked remediation_auth_wrapper_probe -- --nocapture -> Actual crate private helper: 5 executed, 2 controls pass, 3 wrapped credentials fail None vs Some(expected). All wrapped cases first verify zero provider dispatch and preserved projection. No setup/compile failure.
- **Green-after**: cargo test -p opi-coding-agent --lib remediation_auth_wrapper_probe -- --nocapture; cargo test -p opi-agent --test continuation_contract -> For a projected pre-output credential preparation error, product shared helper classifies nested CredentialNeeded/Revoked/AccountIdMissing exactly as direct variants, enabling same TUI/RPC/CLI remediation; nonauth stays none, partial/projection remain intact, initial steering survives retry.
- **Test impact**: add/update; retain existing safety and valid-source controls; delete none.

### Batch B05: reject contradictory terminal reasons

**Closure predicate**: A mismatch between Done.reason and message.stop_reason cannot produce Ok(Error/Aborted): yield one typed protocol failure with terminal content/usage, release inner, permanent EOF, and same result after partial/full/no iteration; consistent Stop/Length/ToolUse/Deferred remain Ok and Error/Aborted Err.
**Dependencies**: []
**Verification union**: cargo test -p opi-ai --test stream_events; cargo clippy -p opi-ai --all-targets -- -D warnings

#### Fix B05.1: reject contradictory terminal reasons

- **Finding source(s)**: phase20-codex-gpt61-b3f1810-20261103t142820z / 4cec97af8ef6bb773c630ee251728f31212e12fdc1bf9631f10b2dc3084bc0f6 / P20-AUD-005
- **Decision**: fix:reject-contradictory-terminal-reasons
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-ai/src/stream.rs`, `crates/opi-ai/tests/stream_events.rs`, `CHANGELOG.md`
- **Change kind**: behavioral
- **Change**: Validate terminal reason agreement before deciding success; mismatch becomes existing typed protocol failure retaining message. Add actual stream iteration/result modes and post-terminal tail/drop counter checks to owning suite, without changing valid reasons.
- **Closure predicate**: A mismatch between Done.reason and message.stop_reason cannot produce Ok(Error/Aborted): yield one typed protocol failure with terminal content/usage, release inner, permanent EOF, and same result after partial/full/no iteration; consistent Stop/Length/ToolUse/Deferred remain Ok and Error/Aborted Err.
- **Red-before**: python D:/Luiz/Odradek/opi/scripts/opi-cargo-cache.py run --target E:/opi/cargo-targets/opi-72fa37a0fa044958 -- test --manifest-path C:/Users/Luiz/AppData/Local/Temp/opi-p20-plan-auth-probes-08ct7l_l/Cargo.toml --offline --locked --test probes -- --nocapture -> FAIL: Done(Stop,message.stop_reason=Error) returns Ok with Error assistant and usage3/5 retained.
- **Green-after**: cargo test -p opi-ai --test stream_events -> A mismatch between Done.reason and message.stop_reason cannot produce Ok(Error/Aborted): yield one typed protocol failure with terminal content/usage, release inner, permanent EOF, and same result after partial/full/no iteration; consistent Stop/Length/ToolUse/Deferred remain Ok and Error/Aborted Err.
- **Test impact**: add/update; retain existing safety and valid-source controls; delete none.

### Batch B06: refuse helper hints above cap

**Closure predicate**: After one real helper-wire429 with Retry-After120s and cap60s, no retry admission/timer/send and call sealed; exact60s remains admitted; cap0 admits120s unchanged; Smithy retains source bounded-hint rules.
**Dependencies**: []
**Verification union**: cargo test -p opi-ai --test provider_collection; cargo test -p opi-ai --test retry_backoff; cargo clippy -p opi-ai --all-targets -- -D warnings

#### Fix B06.1: refuse helper hints above cap

- **Finding source(s)**: phase20-codex-gpt61-b3f1810-20261103t142820z / 4cec97af8ef6bb773c630ee251728f31212e12fdc1bf9631f10b2dc3084bc0f6 / P20-AUD-006
- **Decision**: fix:refuse-helper-hints-above-cap
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-ai/src/retry.rs`, `crates/opi-ai/tests/provider_collection.rs`, `CHANGELOG.md`
- **Change kind**: behavioral
- **Change**: Before incrementing helper retry count or scheduling deadline, refuse nonzero-cap server hint that exceeds cap, preserving original typed failure. Avoid changing Smithy branch or generic legacy RetryConfig contracts. Add below/equal/above/zero cap actual-adapter tests.
- **Closure predicate**: After one real helper-wire429 with Retry-After120s and cap60s, no retry admission/timer/send and call sealed; exact60s remains admitted; cap0 admits120s unchanged; Smithy retains source bounded-hint rules.
- **Red-before**: python D:/Luiz/Odradek/opi/scripts/opi-cargo-cache.py run --target E:/opi/cargo-targets/opi-72fa37a0fa044958 -- test --manifest-path C:/Users/Luiz/AppData/Local/Temp/opi-p20-plan-auth-probes-08ct7l_l/Cargo.toml --offline --locked --test probes -- --nocapture -> FAIL: actual one429 produced Some(RetryAdmission{retry_index:1,delay_ms:60000}), expected None.
- **Green-after**: cargo test -p opi-ai --test provider_collection; cargo test -p opi-ai --test retry_backoff -> After one real helper-wire429 with Retry-After120s and cap60s, no retry admission/timer/send and call sealed; exact60s remains admitted; cap0 admits120s unchanged; Smithy retains source bounded-hint rules.
- **Test impact**: add/update; retain existing safety and valid-source controls; delete none.

### Batch B07: veto context overflow before transient patterns

**Closure predicate**: An Error message matching context-overflow pattern is Terminal despite transient-looking token counts; enabled helper performs one produce and no scheduled/attempt/finished retry callbacks; genuine service500 remains transient and quota/abort precedence remains.
**Dependencies**: []
**Verification union**: cargo test -p opi-ai --test whole_call_retry; cargo clippy -p opi-ai --all-targets -- -D warnings

#### Fix B07.1: veto context overflow before transient patterns

- **Finding source(s)**: phase20-codex-gpt61-b3f1810-20261103t142820z / 4cec97af8ef6bb773c630ee251728f31212e12fdc1bf9631f10b2dc3084bc0f6 / P20-AUD-007
- **Decision**: fix:veto-context-overflow-before-transient-patterns
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-ai/src/retry.rs`, `crates/opi-ai/tests/whole_call_retry.rs`, `CHANGELOG.md`
- **Change kind**: behavioral
- **Change**: Consult existing is_context_overflow(message,None) in failure classification before transient regex; retain independent overflow utility semantics. Add helper invocation/count/callback check and overflow corpus without product recovery changes.
- **Closure predicate**: An Error message matching context-overflow pattern is Terminal despite transient-looking token counts; enabled helper performs one produce and no scheduled/attempt/finished retry callbacks; genuine service500 remains transient and quota/abort precedence remains.
- **Red-before**: python D:/Luiz/Odradek/opi/scripts/opi-cargo-cache.py run --target E:/opi/cargo-targets/opi-72fa37a0fa044958 -- test --manifest-path C:/Users/Luiz/AppData/Local/Temp/opi-p20-plan-auth-probes-08ct7l_l/Cargo.toml --offline --locked --test probes -- --nocapture -> FAIL: maximum context length is 500000 tokens classified Transient, expected Terminal.
- **Green-after**: cargo test -p opi-ai --test whole_call_retry -> An Error message matching context-overflow pattern is Terminal despite transient-looking token counts; enabled helper performs one produce and no scheduled/attempt/finished retry callbacks; genuine service500 remains transient and quota/abort precedence remains.
- **Test impact**: add/update; retain existing safety and valid-source controls; delete none.

### Batch B08: add numeric claude thinking budget before final context clamp

**Closure predicate**: For simple numeric Claude C=4096,B=2048,modelcap=64000,context=200000 the one real adapter request sends max_tokens=6144,budget_tokens=2048; non-thinking stays4096, adaptive/native paths retain their existing contract; model/context saturation computes F before Bprime.
**Dependencies**: []
**Verification union**: cargo test -p opi-ai --test anthropic_fixtures; cargo test -p opi-ai --test provider_collection; cargo test -p opi-ai --test context_budget; cargo clippy -p opi-ai --all-targets -- -D warnings

#### Fix B08.1: add numeric claude thinking budget before final context clamp

- **Finding source(s)**: phase20-codex-gpt61-b3f1810-20261103t142820z / 4cec97af8ef6bb773c630ee251728f31212e12fdc1bf9631f10b2dc3084bc0f6 / P20-AUD-008
- **Decision**: fix:add-numeric-claude-thinking-budget-before-final-context-clamp
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-ai/src/models.rs`, `crates/opi-ai/tests/anthropic_fixtures.rs`, `crates/opi-ai/tests/provider_collection.rs`, `crates/opi-ai/tests/context_budget.rs`, `CHANGELOG.md`
- **Change kind**: behavioral
- **Change**: In the simple numeric Claude branch compute the answer ceiling C and thinking budget B before clamping: F=context-clamp(min(C+B,model cap)), then Bprime=min(B,max(0,F-1024)). Preserve the source zero-budget wire mapping and existing adaptive/native branches. Add the unsaturated4096+2048 case plus model/context saturation controls; record the user-visible fix in Unreleased.
- **Closure predicate**: For simple numeric Claude C=4096,B=2048,modelcap=64000,context=200000 the one real adapter request sends max_tokens=6144,budget_tokens=2048; non-thinking stays4096, adaptive/native paths retain their existing contract; model/context saturation computes F before Bprime.
- **Red-before**: python C:\Users\Luiz\AppData\Local\Temp\opi-phase20-remediate-plan-jqdki4yk\source\scripts\opi-cargo-cache.py run --target E:\opi\cargo-targets\opi-72fa37a0fa044958 -- test --manifest-path C:\Users\Luiz\AppData\Local\Temp\opi-phase20-remediate-plan-jqdki4yk\sdk-current-probes\Cargo.toml --offline --locked --lib -- --nocapture -> FAIL: one real Anthropic request max_tokens=4096 vs required6144, thinking.budget_tokens=2048; matching no-thinking control PASS4096.
- **Green-after**: cargo test -p opi-ai --test anthropic_fixtures; cargo test -p opi-ai --test provider_collection; cargo test -p opi-ai --test context_budget -> For simple numeric Claude C=4096,B=2048,modelcap=64000,context=200000 the one real adapter request sends max_tokens=6144,budget_tokens=2048; non-thinking stays4096, adaptive/native paths retain their existing contract; model/context saturation computes F before Bprime.
- **Test impact**: add/update; retain existing safety and valid-source controls; delete none.

### Batch B09: initialize independent memory credential store in constructors

**Closure predicate**: Models::new and from_registry each own an independent local credential store; fake flow login succeeds and acknowledges it without injection, one instance cannot observe another login, explicit store replacement remains owner and construction reads no environment/store/network or runtime.
**Dependencies**: []
**Verification union**: cargo test -p opi-ai --test oauth_flows --test provider_collection --test external_sdk --test builtin_auth_ownership --test auth_contracts --offline --locked; cargo test -p opi-coding-agent --test list_models --offline --locked; cargo clippy -p opi-ai -p opi-coding-agent --all-targets --offline --locked -- -D warnings

#### Fix B09.1: initialize independent memory credential store in constructors

- **Finding source(s)**: phase20-codex-gpt61-b3f1810-20261103t142820z / 4cec97af8ef6bb773c630ee251728f31212e12fdc1bf9631f10b2dc3084bc0f6 / P20-AUD-012
- **Decision**: fix:initialize-independent-memory-credential-store-in-constructors
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-ai/src/models.rs`, `crates/opi-ai/tests/oauth_flows.rs`, `crates/opi-ai/tests/provider_collection.rs`, `crates/opi-ai/tests/external_sdk.rs`, `crates/opi-ai/README.md`, `crates/opi-ai/README.zh.md`, `CHANGELOG.md`, `crates/opi-ai/tests/builtin_auth_ownership.rs`, `crates/opi-ai/tests/auth_contracts.rs`, `crates/opi-coding-agent/tests/list_models.rs`
- **Change kind**: behavioral
- **Change**: Initialize each Models constructor with its own existing InMemoryCredentialStore and preserve explicit replacement, lazy construction and instance isolation. Preserve the existing set_probe configuration contract through a private implicit-memory versus explicitly-attached store ownership distinction: actual credentials win; only Ok(None) from constructor-owned memory may consult an injected label-only probe. Explicit-store absence/error remains authoritative; resolvers and dispatch never consume probes. Synchronize affected store/default rustdoc and both READMEs, plus default barrier/flow-removal assumptions. Add fake login/ACK, ownership-priority and actual product listing controls; preserve zero interaction, zero secret reads and fail-closed source errors; record Unreleased.
- **Closure predicate**: Models::new and from_registry each own an independent local credential store; fake flow login succeeds and acknowledges it without injection, one instance cannot observe another login, explicit store replacement remains owner and construction reads no environment/store/network or runtime.
- **Red-before**: python C:\Users\Luiz\AppData\Local\Temp\opi-phase20-remediate-plan-jqdki4yk\source\scripts\opi-cargo-cache.py run --target E:\opi\cargo-targets\opi-72fa37a0fa044958 -- test --manifest-path C:\Users\Luiz\AppData\Local\Temp\opi-phase20-remediate-plan-jqdki4yk\sdk-current-probes\Cargo.toml --offline --locked --lib -- --nocapture -> FAIL: CredentialStore Backend(no credential store attached), fake flow_count0; injected-memory-store control PASS, flow_count1.
- **Green-after**: cargo test -p opi-ai --test oauth_flows --test provider_collection --test external_sdk --test builtin_auth_ownership --test auth_contracts --offline --locked; cargo test -p opi-coding-agent --test list_models --offline --locked; cargo clippy -p opi-ai -p opi-coding-agent --all-targets --offline --locked -- -D warnings -> Models::new and from_registry each own an independent local credential store; fake flow login succeeds and acknowledges it without injection, one instance cannot observe another login, explicit store replacement remains owner and construction reads no environment/store/network or runtime. Existing snapshot/configuration and actual listing behavior remain; explicit empty/failing stores do not fall back; probes never authenticate.
- **Test impact**: add/update; retain existing safety and valid-source controls; delete none.

**Existing-consumer synchronization**: In `builtin_auth_ownership::remove_and_clear_keep_auth_only_flows_and_remove_model_owned_flows`, use a dedicated noninteractive fake flow returning typed `LoginCancelled` without calling any presenter method. Retained flows produce that nested `ModelsError::AuthResolution` cause; removed model-owned flows return `UnknownOAuthFlow` before the fake flow runs. Preserve presenter no-interaction assertions. In `auth_contracts`, replace only the default `mutation_barrier().is_none()` expectation with an empty constructor-owned barrier/isolation assertion; retain shared explicit-store cutoff/error and snapshot controls. Extend `list_models` stored-only/presence fixtures through actual `available_model_entries`, retaining redaction, zero secret reads and corrupt/operational-probe refusal. Product `provider_factory.rs` stays unchanged: its existing descriptor/probe consumer remains supported.

**Bilingual default-store contract**: English: Models constructors own independent memory credential stores without external discovery. Descriptor configuration checks prefer actual default-store credentials; only empty implicit memory may consult redacted probe metadata. An explicitly attached store remains authoritative, including absence/errors. Probes never supply authentication or authorize dispatch. Persistent catalog/product stores remain explicit. Chinese: Models 构造器各自拥有独立的内存凭据存储，不主动发现外部来源。descriptor 配置检查优先读取默认存储中的实际凭据；只有隐式默认存储缺少条目时才可使用脱敏 probe 元数据。显式接入的存储始终具有权威性，包括缺失和错误。probe 不提供认证，也不授权生成请求。持久化目录／产品存储仍需显式接入。 Qualify README catalog-store and optional product-store wording; do not describe those stores as absence of SDK local memory.

**Downstream final-source proof**: B19 and final workspace gates, plus B17/B95/B92 package/native consumers, remain mandatory. All local B09 source/consumer regression checks must pass before continuing; downstream ownership waives no final gate.

### Batch B10: derive image modalities from text capability

**Closure predicate**: One image request always includes image first exactly once; Text present yields [image,text], otherwise [image], regardless of record list ordering/duplicates/absence; ordinary successful result parsing is preserved.
**Dependencies**: ["B14"]
**Verification union**: cargo test -p opi-ai --test openrouter_fixtures; cargo clippy -p opi-ai --all-targets -- -D warnings

#### Fix B10.1: derive image modalities from text capability

- **Finding source(s)**: phase20-codex-gpt61-b3f1810-20261103t142820z / 4cec97af8ef6bb773c630ee251728f31212e12fdc1bf9631f10b2dc3084bc0f6 / P20-AUD-013
- **Decision**: fix:derive-image-modalities-from-text-capability
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-ai/src/api/openrouter_images.rs`, `crates/opi-ai/tests/openrouter_fixtures.rs`, `CHANGELOG.md`
- **Change kind**: behavioral
- **Change**: Build request modalities from Text membership rather than copying output metadata: emit [image,text] when Text is present and [image] otherwise. Preserve successful empty/text-only result acceptance. Add absent/duplicate/reordered metadata captures and record Unreleased.
- **Closure predicate**: One image request always includes image first exactly once; Text present yields [image,text], otherwise [image], regardless of record list ordering/duplicates/absence; ordinary successful result parsing is preserved.
- **Red-before**: python C:\Users\Luiz\AppData\Local\Temp\opi-phase20-remediate-plan-jqdki4yk\source\scripts\opi-cargo-cache.py run --target E:\opi\cargo-targets\opi-72fa37a0fa044958 -- test --manifest-path C:\Users\Luiz\AppData\Local\Temp\opi-phase20-remediate-plan-jqdki4yk\sdk-current-probes\Cargo.toml --offline --locked --lib -- --nocapture -> FAIL: captured modalities=[text] vs[image,text], one request, valid response Stop; image-only/result retention control PASS.
- **Green-after**: cargo test -p opi-ai --test openrouter_fixtures -> One image request always includes image first exactly once; Text present yields [image,text], otherwise [image], regardless of record list ordering/duplicates/absence; ordinary successful result parsing is preserved.
- **Test impact**: add/update; retain existing safety and valid-source controls; delete none.

### Batch B11: preserve received image result on response callback failure or cancel

**Closure predicate**: After successful full JSON acquisition, callback fires once; callback error/cancel yields Error/Aborted with correct typed cause while preserving received output,id,usage,cost and original unknowns; exactly one request, no fallback/retry, invalid JSON does not notify callback.
**Dependencies**: ["B14"]
**Verification union**: cargo test -p opi-ai --test openrouter_fixtures; cargo clippy -p opi-ai --all-targets -- -D warnings

#### Fix B11.1: preserve received image result on response callback failure or cancel

- **Finding source(s)**: phase20-codex-gpt61-b3f1810-20261103t142820z / 4cec97af8ef6bb773c630ee251728f31212e12fdc1bf9631f10b2dc3084bc0f6 / P20-AUD-014
- **Decision**: fix:preserve-received-image-result-on-response-callback-failure-or-cancel
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-ai/src/api/openrouter_images.rs`, `crates/opi-ai/src/images.rs`, `crates/opi-ai/tests/openrouter_fixtures.rs`, `CHANGELOG.md`
- **Change kind**: behavioral
- **Change**: In the successful-JSON handoff branch preserve a parsed ImagesResult when onResponse fails or cancellation wins: keep output,id,usage,cost and unknowns, replace only terminal reason/cause/safe error display. Keep callback at its source-defined post-JSON boundary and exactly once, invalid-JSON callback count0 and no retry. Do not replace partial facts with ImagesResult::failure defaults; add failure/cancel/control cases and record Unreleased.
- **Closure predicate**: After successful full JSON acquisition, callback fires once; callback error/cancel yields Error/Aborted with correct typed cause while preserving received output,id,usage,cost and original unknowns; exactly one request, no fallback/retry, invalid JSON does not notify callback.
- **Red-before**: python C:\Users\Luiz\AppData\Local\Temp\opi-phase20-remediate-plan-jqdki4yk\source\scripts\opi-cargo-cache.py run --target E:\opi\cargo-targets\opi-72fa37a0fa044958 -- test --manifest-path C:\Users\Luiz\AppData\Local\Temp\opi-phase20-remediate-plan-jqdki4yk\sdk-current-probes\Cargo.toml --offline --locked --lib -- --nocapture -> FAIL2: after acquired successful JSON, onResponse failure and cancellation each callback1/request1, correct Error/Aborted cause, but response_id=None,usage=None,output.len=0; ordinary success control PASS retained id,usage,output2.
- **Green-after**: cargo test -p opi-ai --test openrouter_fixtures -> After successful full JSON acquisition, callback fires once; callback error/cancel yields Error/Aborted with correct typed cause while preserving received output,id,usage,cost and original unknowns; exactly one request, no fallback/retry, invalid JSON does not notify callback.
- **Test impact**: add/update; retain existing safety and valid-source controls; delete none.

### Batch B12: accept floating zero for null coercion

**Closure predicate**: Public compile({type:null}) plus validate_tool_arguments accepts 0, -0, 0.0 and -0.0 as normalized Null without mutating raw; nonzero and literal strings are rejected.
**Dependencies**: []
**Verification union**: cargo test -p opi-ai --test schema_public; cargo test -p opi-ai --test schema_engine; cargo clippy -p opi-ai --all-targets -- -D warnings

#### Fix B12.1: accept floating zero for null coercion

- **Finding source(s)**: phase20-codex-gpt61-b3f1810-20261103t142820z / 4cec97af8ef6bb773c630ee251728f31212e12fdc1bf9631f10b2dc3084bc0f6 / P20-AUD-015
- **Decision**: fix:accept-floating-zero-for-null-coercion
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-ai/src/schema/normalize.rs`, `crates/opi-ai/tests/schema_public.rs`, `CHANGELOG.md`
- **Change kind**: behavioral
- **Change**: Add the numeric Float zero check to the null coercion branch, preserving raw input and the final original-schema validator. Add public positive 0/-0/0.0/-0.0 controls and negative nonzero/literal-string controls.
- **Closure predicate**: Public compile({type:null}) plus validate_tool_arguments accepts 0, -0, 0.0 and -0.0 as normalized Null without mutating raw; nonzero and literal strings are rejected.
- **Red-before**: python D:/Luiz/Odradek/opi/scripts/opi-cargo-cache.py run --target E:/opi/cargo-targets/opi-72fa37a0fa044958 -- test --manifest-path C:/Users/Luiz/AppData/Local/Temp/opi-p20-plan-auth-probes-08ct7l_l/Cargo.toml --offline --locked --test probes -- --nocapture -> Executed failure: after integer0,false,empty-string acceptance and 1,string0,stringnull,true rejection controls pass, floating0.0 produces Invalid(type null) rather than normalized Null. Assertion stops before -0.0; not claimed executed. Total combined external probe run 13 tests,4passed,9failed,0ignored.
- **Green-after**: cargo test -p opi-ai --test schema_public; cargo test -p opi-ai --test schema_engine -> Public compile({type:null}) plus validate_tool_arguments accepts 0, -0, 0.0 and -0.0 as normalized Null without mutating raw; nonzero and literal strings are rejected.
- **Test impact**: add/update; retain existing safety and valid-source controls; delete none.

### Batch B15: retain affinity when cache retention disabled

**Closure predicate**: Disabled cache retention suppresses prompt_cache_key but preserves the three explicitly enabled session-affinity headers; direct/default/format controls retain their expected headers.
**Dependencies**: []
**Verification union**: cargo test -p opi-ai --test request_enrichment; cargo clippy -p opi-ai --all-targets -- -D warnings

#### Fix B15.1: retain affinity when cache retention disabled

- **Finding source(s)**: phase20-pi-glm53-44374ec-20261103t160703z / f8c06b50b3c51c95ac5b1309e72a8b31d06c34785ad588ed4e42bff19d914990 / P20-AUD-003
- **Decision**: fix:retain-affinity-when-cache-retention-disabled
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-ai/tests/request_enrichment.rs`
- **Change kind**: test-only
- **Change**: Update only the disabled-retention expected header values and assertion explanation; retain prompt_cache_key absence and all other mappings.
- **Closure predicate**: Disabled cache retention suppresses prompt_cache_key but preserves the three explicitly enabled session-affinity headers; direct/default/format controls retain their expected headers.
- **Red-before**: python scripts/opi-cargo-cache.py run --target E:\opi\cargo-targets\opi-72fa37a0fa044958 -- test -p opi-ai --test request_enrichment --offline --locked session_affinity_wire_mappings -- --exact --nocapture -> FAIL: thread 'session_affinity_wire_mappings' (61488) panicked at crates\opi-ai\tests\request_enrichment.rs:435:9:; test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 31 filtered out; finished in 0.01s; log SHA-256 d44501444dac7840fe84cac56a5f3d56ba434197bbbff5403a5b7400aa8df46a
- **Green-after**: cargo test -p opi-ai --test request_enrichment -> Disabled cache retention suppresses prompt_cache_key but preserves the three explicitly enabled session-affinity headers; direct/default/format controls retain their expected headers.
- **Test impact**: add/update; retain existing safety and valid-source controls; delete none.

### Batch B16: use false default with original unconstrained schema

**Closure predicate**: For an unannotated tool, real Conversations request sends strict:false and original parameters without inserted additionalProperties/required; explicit Require remains strict:true with converted schema.
**Dependencies**: ["B14"]
**Verification union**: cargo test -p opi-ai --test mistral_fixtures; cargo clippy -p opi-ai --all-targets -- -D warnings

#### Fix B16.1: use false default with original unconstrained schema

- **Finding source(s)**: phase20-pi-glm53-44374ec-20261103t160703z / f8c06b50b3c51c95ac5b1309e72a8b31d06c34785ad588ed4e42bff19d914990 / P20-AUD-005
- **Decision**: fix:use-false-default-with-original-unconstrained-schema
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-ai/src/mistral.rs`, `crates/opi-ai/tests/mistral_fixtures.rs`, `crates/opi-ai/README.md`, `crates/opi-ai/README.zh.md`, `CHANGELOG.md`
- **Change kind**: behavioral
- **Change**: Use false for absent constrained intent, correct source-attribution rustdoc, migrate default test to false plus original-schema comparison, retain explicit Require control, and synchronize both READMEs/Unreleased.
- **Closure predicate**: For an unannotated tool, real Conversations request sends strict:false and original parameters without inserted additionalProperties/required; explicit Require remains strict:true with converted schema.
- **Red-before**: E:\opi\cargo-targets\opi-72fa37a0fa044958\debug\deps\mistral-ca7cac90210ef694.exe mistral_43::plan_ --nocapture -> FAIL:3executed,1passed,2failed,0ignored; Bool(true) versus Bool(false), and additionalProperties inserted; exit101. Sealed probe source 6a77824f42235473f23d1d06c0e12fb3e9bfc0cebbdb6e6b3822284344fef42c
- **Green-after**: cargo test -p opi-ai --test mistral_fixtures -> For an unannotated tool, real Conversations request sends strict:false and original parameters without inserted additionalProperties/required; explicit Require remains strict:true with converted schema.
- **Test impact**: add/update; retain existing safety and valid-source controls; delete none.

### Batch B19: preserve source classifier declaration enumeration

**Closure predicate**: Classifier question and choice declarations retain pinned own-property enumeration: canonical uint32 keys0..4294967294 numeric first, other strings first-insertion order; replacement keeps slot and remove/reinsert appends ordinary keys. All four wires inherit that order, Llama sequential sends/labels/ties use it, outputs retain their existing maps.
**Dependencies**: ["B13", "B14", "B04", "B09", "B15"]
**Verification union**: cargo test -p opi-ai --test classifier_contract --offline --locked; cargo test --workspace --all-targets --offline --locked; cargo clippy --workspace --all-targets --offline --locked -- -D warnings

#### Fix B19.1: preserve source classifier declaration enumeration

- **Finding source(s)**: phase20-codex-gpt61-b3f1810-20261103t142820z / 4cec97af8ef6bb773c630ee251728f31212e12fdc1bf9631f10b2dc3084bc0f6 / P20-AUD-011
- **Decision**: fix:preserve-source-classifier-declaration-enumeration
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-ai/src/classifier.rs`, `crates/opi-ai/src/api/decisions.rs`, `crates/opi-ai/tests/classifier_contract.rs`, `crates/opi-ai/README.md`, `crates/opi-ai/README.zh.md`, `CHANGELOG.md`
- **Change kind**: behavioral
- **Change**: Replace only input declaration fields with public ClassifierDeclarations<T>, private Vec<(String,T)> and minimal actually used collection interface. Maintain numeric-prefix/insertion-tail on insert; duplicate last value keeps original slot. Bound-free Default, conditional Clone/Debug/PartialEq, insert/remove/get/len/is_empty/iter/keys/values, borrowed/owned IntoIterator, FromIterator and array From; Serialize emits ordered JSON object. No dependency or output-map changes. Correct Decisions alphabetical comment and existing explicit BTreeMap input locals; keep inferred array/collect callers unchanged where they compile.
- **Closure predicate**: Classifier question and choice declarations retain pinned own-property enumeration: canonical uint32 keys0..4294967294 numeric first, other strings first-insertion order; replacement keeps slot and remove/reinsert appends ordinary keys. All four wires inherit that order, Llama sequential sends/labels/ties use it, outputs retain their existing maps.
- **Red-before**: python D:/Luiz/Odradek/opi/scripts/opi-cargo-cache.py run --target E:/opi/cargo-targets/opi-72fa37a0fa044958 -- test --manifest-path C:\Users\Luiz\AppData\Local\Temp\opi-p20-classifier-order-zfhygquw\Cargo.toml --offline --locked --test order -- --nocapture -> FAIL:5executed,2passed/3failed,0ignored,exit101; z/a question sends become a/z, equal zeta/alpha probabilities choose alpha, integer-like key enumeration is lexicographic. Source/lock receipt 94bea0954a45a58265cbe8971b56090a50a0238c176b97bf26706991206ab5e1
- **Green-after**: cargo test -p opi-ai --test classifier_contract --offline --locked; cargo test --workspace --all-targets --offline --locked; cargo clippy --workspace --all-targets --offline --locked -- -D warnings -> PASS; all original distinguishing assertions and controls must execute.
- **Test impact**: add/update; retain wire/safety/legacy-reader controls; delete only made-obsolete sentinel assertions after replacement proof.

#### C011/D3 and C016/D4 current planning resolution

Source baseline: d053965b8ef3bca8d79990793459bb0ba87fe69c. Active index: e1b34739db46331707a93ab0f6f9336ed6386a4564b0215d8488ce288319f0ae. No repository or exported production source changed.

##### D3 — confirmed ordering defect; concrete public representation

The public collection owner is the root change. Replace only declaration maps with `ClassifierDeclarations<T>` in `classifier.rs`, backed by a private `Vec<(String,T)>`. `ClassifierContext.questions` becomes `ClassifierDeclarations<ClassifierQuestion>` and `ClassifierQuestion::Choice.criteria` becomes `ClassifierDeclarations<String>`. Keep result answers/probability maps and score level Vec unchanged. Two actual typed declaration consumers justify the single shared type; no crate, dependency, feature, trait boundary, root manifest or lock change.

Selected semantics reproduce the exact source object enumeration used by the pinned classifier implementations:

- Own string keys only. Canonical decimal array-index keys in 0..=4294967294 enumerate numerically before ordinary string keys. All other strings retain first-insertion order. `01`, `-0`, `4294967295` are ordinary strings. Empty strings remain allowed; no new key/capacity rejection.
- `insert(key,value) -> Option<T>` updates an existing value in its original slot. `FromIterator<(String,T)>` applies sequential inserts: last duplicate value wins without moving its first slot. `remove(&str) -> Option<T>` removes that slot and preserves other relative order; reinserting an ordinary key appends it, matching delete/re-add. No duplicate rejection is added.
- Maintain numeric-prefix/insertion-tail invariant at insertion, not by sorting every consumer. Detect a canonical array index by parsing u32, rejecting u32::MAX, and requiring `n.to_string()==key`. No unsafe or allocation beyond the owned keys/Vec is necessary. Lookup/insert/remove are linear; no speculative index/cache is needed for small classifier declaration lists.
- Minimal actual-consumer interface: `new`, bound-free Default, Clone/Debug/PartialEq where T supports them, `insert`, `remove`, `get(&str)`, `len`, `is_empty`, `iter`, `keys`, `values`, borrowed/owned IntoIterator, FromIterator, and From<[(String,T);N]>. Do not add unused map APIs. Implement `serde::Serialize` for `T: Serialize` as a JSON object in collection enumeration order, required by existing System One choice rendering. No new Context serde contract or durable format is introduced by this fix.
- Existing `.into_iter().collect()` and array `.into()` call sites continue compiling through this concrete type. Three explicit BTreeMap locals in classifier_contract.rs become ClassifierDeclarations. Existing source declaration order changes their expected wire order, so migrate expectations; do not reorder input fixtures merely to make old alphabetical expectations pass.

Pinned-source evidence (all six relevant paths matched the current source-evidence manifest; see source-pin-check.json):

- types.ts:664–694 uses Record<string,string>/Record<string,ClassifierQuestion>.
- llama-cpp-classify.ts:104–128 assigns labels from Object.keys(criteria); :150–178 builds overview from Object.values(questions); :209–216 chooses first equal maximum; :443–450 validates and executes questions via Object.keys/Object.entries sequentially.
- system-one-shared.ts:86–99 maps Object.entries(questions), retains choice object; cloudflare-workers-ai-system-one.ts wraps the same shared request.
- openai-decisions.ts:57 maps choice Object.entries to arrays; :180 maps question Object.entries to array.
- The local Node v24.13.0 probe demonstrates mixed integer keys and duplicate replacement; it is a bounded language-control probe, not a claim to have executed the full Pi SDK. The source package version is 1.3.0 and exact selected source digests are recorded. The snapshot is not a separate Git repository; no root Git HEAD is misrepresented as a Pi commit.

Actual current red evidence (test source is pure external public consumer; production input unchanged):

`python D:/Luiz/Odradek/opi/scripts/opi-cargo-cache.py run --target E:/opi/cargo-targets/opi-72fa37a0fa044958 -- test --manifest-path Cargo.toml --offline --locked --test order -- --nocapture`

Cwd is this notes directory. Exit101; 5 executed,2passed,3failed,0ignored. All268 external registry dependency versions/checksums equal root lock. See probe-identities.json.

- `order_red_question_wire_sequence`: actual Models->Llama->loopback with two completion and two template requests; observed `[a-second,z-first]`, required `[z-first,a-second]`.
- `order_red_choice_tie_preserves_declared_first`: actual Models->Llama->loopback; both option probabilities0.5, exactly one template/completion request; observed `alpha`, required declared first `zeta`.
- `order_red_js_integer_property_prefix`: public render_question observes `[01,10,2,alpha,zeta]`, required `[2,10,01,zeta,alpha]`.
- Controls pass: alphabetically declared equal-probability first choice remains alpha; duplicate key replacement updates description and retains unique key count.

One closure predicate: for both declaration roles, source-qualified key enumeration/replacement is retained at public construction and every downstream adapter consumer; Llama sequential question sends and equal-probability choice use that same order. This is one root owner, not independent per-adapter policy.

Exact minimal proposed changed paths:

- crates/opi-ai/src/classifier.rs — collection owner/type fields/rustdoc.
- crates/opi-ai/src/api/decisions.rs — remove the current inaccurate alphabetical-order comment; its iteration already consumes the collection.
- crates/opi-ai/tests/classifier_contract.rs — migrate explicit collection locals/current alphabetical expectations; add public order/control probes and all four wire assertions.
- crates/opi-ai/README.md and README.zh.md — describe typed declaration collection, overwrite and integer-key ordering.
- CHANGELOG.md — deliberate 0.x public field type break; read full Unreleased before addition.

The other three adapters already traverse keys/values/iter and need no behavior rewrite: system_one.rs:191–222 collects into preserve_order serde_json::Map; cloudflare.rs:143–165 wraps it; llama.rs:560,652,732,775,831,852 consumes the declaration methods. Decisions:234/575 arrays inherit the new invariant. Consumers inspected: api.rs/models.rs signatures, builtin_auth_ownership, catalog_operations, external_sdk, provider_collection, telemetry_operations, classifier_contract. No product crate constructs a classifier declaration. Unchanged inferred/array call sites remain unchanged if they compile; preserve unrelated BTreeMaps.

Verification union: migrate these exact probes into classifier_contract before production edits and reproduce red; after owner change run complete classifier_contract binary and every modified test target. Add object-wire order assertions for Typesafe and Cloudflare (inspect actual decoded key order, not object equality), array order assertions for Decisions, exact sends/label/tie assertions for Llama. Add canonical key boundary0/4294967294 vs4294967295/01/-0, duplicate replace and remove/reinsert controls. Run existing external_sdk, catalog_operations, builtin_auth_ownership, provider_collection, telemetry_operations targets as affected caller conformance, and affected package clippy. Public-break scope joins parent final workspace fmt/clippy/all-targets/doc/rustdoc union. No provider credentials/live calls.

### C011 linked usage criterion032

The sidecar's P20-SDK-CLASSIFIER-032 is a distinct usage/no-authority criterion at SDK131, not evidence of question order. Do not mark its original state met by fixing order. Current static counter-evidence: classifier-shared.ts114–127 explicitly gives classifier input/output parents with zero cache buckets; Rust classifier.rs244–269 preserves those with unknown/known handling. Image openrouter_images.rs684–717 computes cache_write=W, cache_read=cached-W and input=prompt-cache_read-cache_write, then totals the disjoint counts. Current public probes now independently discharge this incidental applicability obligation: `python D:/Luiz/Odradek/opi/scripts/opi-cargo-cache.py run --target E:/opi/cargo-targets/opi-72fa37a0fa044958 -- test --manifest-path Cargo.toml --offline --locked --test usage -- --nocapture` executed3tests,3passed,0failed,0ignored,exit0. TypeSafe/Cloudflare/Decisions each execute known, malformed-partial and absent usage cases (9 total actual single requests); known parent100/output7 remains100/7/cache0/0/total107 even with cache child metadata. Image known prompt100/cached30/write10/output7 yields disjoint70/20/10/7,total107; missing-parent control remains unreported/unknown (2single requests). Llama successful completion keeps usage/cost absent (1completion). Counts and public result assertions bind the real adapters, not copied math. No tool/decision executor is present in these SDK contracts; returning typed answers creates no host authority. See usage-final.log and usage-identities.json. Initial direct-options compile setup failure was retained in usage-setup-failure.log and is not behavioral evidence. Do not rewrite original requirement state or claim wider C018 completion; no unrelated usage production edit is needed.


#### C011 owning-contract draft

These are proposed apply-time clauses, not claims about current behavior. They belong to the existing classifier public contract and both README counterparts. No repository file has been edited.

##### Rustdoc in crates/opi-ai/src/classifier.rs

For `ClassifierDeclarations<T>`:

```rust
/// Named classifier declarations in source-defined enumeration order.
///
/// Canonical decimal array-index keys from `0` through `4294967294` appear
/// first in numeric order. Other string keys retain first-insertion order;
/// for example, `01`, `-0` and `4294967295` are ordinary string keys.
/// Replacing a key changes its value without moving it. Removing and then
/// reinserting an ordinary string key places it after the other ordinary
/// keys. Collection from an iterator applies these same replacement rules.
///
/// This order determines question wire order, Llama question execution,
/// choice labels and the first choice selected when Llama probabilities tie.
/// Serialization emits a JSON object in the same enumeration order.
```

For `ClassifierContext.questions`, replace its current field comment with:

```rust
/// Typed questions keyed by caller id, in declaration enumeration order.
```

For `ClassifierQuestion::Choice.criteria`, replace its current field comment with:

```rust
/// Option keys and descriptions in declaration enumeration order.
```

##### English README clause

Insert after the classifier operation paragraph introducing `ClassifierContext` and the driver methods in `crates/opi-ai/README.md`:

`ClassifierContext::questions` and `ClassifierQuestion::Choice::criteria`
use `ClassifierDeclarations<T>`. Canonical decimal array-index keys from
`0` through `4294967294` enumerate first in numeric order; all other string
keys retain their first insertion order. `01`, `-0` and `4294967295` are
ordinary string keys. Inserting a duplicate key replaces its value without
moving it; removing and reinserting an ordinary key appends it to the
ordinary keys. Array conversion and iterator collection use the same rules.
All four adapters preserve this declaration order on the wire. Llama also
uses it to execute questions sequentially, assign choice labels and select
the first choice when probabilities tie. Output answer and probability maps
retain their existing representation.

##### Chinese README counterpart

Insert after the corresponding classifier operation paragraph in `crates/opi-ai/README.zh.md`:

`ClassifierContext::questions` 和 `ClassifierQuestion::Choice::criteria`
使用 `ClassifierDeclarations<T>`。`0` 到 `4294967294` 范围内的规范十进制数组
索引键先按数值升序枚举，其他字符串键保持首次插入顺序。`01`、`-0` 和
`4294967295` 都是普通字符串键。插入重复键会替换其值而不移动位置；删除后
重新插入普通键会将其追加到其他普通键之后。数组转换和迭代器收集使用相同
规则。四个适配器均在 wire 上保留此声明顺序；Llama 还据此依次执行问题、
分配选项标签，并在概率相同时选择顺序最前的选项。输出答案和概率 map 的
现有表示保持不变。

##### Unreleased change content

After reading the complete apply-time Unreleased section and reusing its relevant subsection, record the deliberately scoped 0.x field-type break:

> Classifier question and choice declarations now use `ClassifierDeclarations<T>` instead of `BTreeMap`, preserving source-defined key enumeration, replacement and Llama tie order. Construct declarations with arrays, iterator collection or `insert`; callers that explicitly name the former field type must migrate.

This content is to integrate under the current changelog style during the owning apply batch. Reread the complete current Unreleased section before insertion and preserve every released section.

### Batch B22: Preserve typed complete Anthropic metadata/defaults through source/owned-store codecs and synchronize existing callers

**Closure predicate**: All selected source booleans, optional affinity/fallback fields, exact true defaults, explicitfalse, empty-versus-absent and fallback local rates survive executable record import/store/restore; invalid typed values and morethan3targets fail before commit; extension precedence remains exact.
**Dependencies**: []
**Verification union**: model_wire_metadata/catalog_operations source and owned roundtrip,invalidtypes,max3,allliteral caller compile; preserve explicit caller policy; applicable checks for the implemented branch and changed targets.

**Downstream final-source proof**: C009 requires all B22-B27 predicates after all six local branches pass. Final workspace gates and B91/B17/B95/B92 integration/package/native proof remain mandatory at those declared owners. No descendant gates this local branch.

#### Fix B22.1: anthropic.complete-source-metadata

- **Finding source(s)**: phase20-codex-gpt61-b3f1810-20261103t142820z / 4cec97af8ef6bb773c630ee251728f31212e12fdc1bf9631f10b2dc3084bc0f6 / P20-AUD-009
- **Decision**: fix:preserve-and-execute-current-source-anthropic-compat-fields-and-defaults
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-ai/src/model_info.rs`, `crates/opi-ai/src/catalog/wire.rs`, `crates/opi-ai/src/anthropic.rs`, `crates/opi-ai/tests/model_wire_metadata.rs`, `crates/opi-ai/tests/catalog_operations.rs`, `crates/opi-ai/tests/anthropic_fixtures.rs`, `crates/opi-ai/tests/anthropic_thinking_roundtrip.rs`, `crates/opi-ai/tests/node4_preparation.rs`, `crates/opi-ai/tests/telemetry_operations.rs`, `crates/opi-coding-agent/src/config.rs`, `crates/opi-coding-agent/src/github_copilot.rs`, `crates/opi-ai/README.md`, `crates/opi-ai/README.zh.md`, `CHANGELOG.md`
- **Change kind**: behavioral
- **Change**: Preserve typed complete Anthropic metadata/defaults through source/owned-store codecs and synchronize existing callers; {"public_fields": ["supports_long_cache_retention: bool defaulttrue", "supports_cache_control_on_tools: bool defaulttrue", "allow_empty_signature: bool defaultfalse", "supports_mid_convo_effort: bool defaultfalse", "send_session_affinity_headers: Option<bool> defaultNone (preserves contextual inference versus explicitfalse)", "session_affinity_format: Option<AnthropicSessionAffinityFormat> defaultNone", "allowed_fallback_models: Option<Vec<AnthropicAllowedFallbackModel>> defaultNone (empty list distinct metadata but same omit behavior)"], "types": ["AnthropicSessionAffinityFormat enum with only OpenRouter, serde rename openrouter; avoids accepting OpenAI dialect values unavailable on this source wire", "AnthropicAllowedFallbackModel { provider: String, model: String, cost: ModelPricing }; reuse existing base+tiers pricing representation and validation"], "defaults": "Replace derived Default on AnthropicMessagesCompat with explicit source defaults: eager,long-cache,tool-cache,temperature=true; force-adaptive,empty-signature,strict,mid-effort,mid-system,mid-toolchanges=false; optional affinity/fallback fieldsNone.", "decode_encode": "apply_source_metadata reads all source fields; compat_to_extension writes all resolved booleans and present optionals; parse_compat reads extended owned store shape with defaults for newly missing fields, retains existing required old fields; present source/extension wrong types reject, fallback list length<=3, provider/model nonempty, cost present and valid source metadata. New fields preserve through public ModelsStore.", "precedence": "Existing explicit x-opi.compat projection continues to win over source compat as documented at wire.rs865-867. For plain Anthropic source records, derive cacheControl=true and longCacheRetention=true unless explicit existing x-opi capability overrides; runtime also gates TTL on compat supports_long_cache_retention. Existing direct ModelInfo callers' explicit generic cache opt-out remains respected. Source compatfalse can suppress a branch even when generic capabilitytrue. Session enable/format retain None until concrete provider/baseURL resolution.", "current_red": ["c009_absent_source_true_defaults_remain_true: authentic fable-5 missing eager/temperature becomes false/false", "c009_fallback_local_prices_survive_public_store_roundtrip: source two fallback targets with rates disappear from persisted executable record"], "caller_sync": ["crates/opi-ai/src/anthropic.rs literals957,1279: fill new defaults intentionally and preserve explicit conservative unknown-model fallback where current contract requires it", "crates/opi-ai/src/catalog/wire.rs literal505/source844/encoder157", "crates/opi-ai/tests/anthropic_fixtures.rs literals1711,2448,2849,3513", "crates/opi-ai/tests/node4_preparation.rs679", "crates/opi-ai/tests/telemetry_operations.rs903", "crates/opi-coding-agent/src/config.rs1927", "crates/opi-coding-agent/src/github_copilot.rs427"]}
- **Closure predicate**: All selected source booleans, optional affinity/fallback fields, exact true defaults, explicitfalse, empty-versus-absent and fallback local rates survive executable record import/store/restore; invalid typed values and morethan3targets fail before commit; extension precedence remains exact.
- **Red-before**: python scripts/opi-cargo-cache.py run --target E:\opi\cargo-targets\opi-72fa37a0fa044958 -- test --manifest-path C:\Users\Luiz\AppData\Local\Temp\opi-phase20-remediate-plan-jqdki4yk\sdk-compat-probes\Cargo.toml --offline --locked --lib -- --nocapture --test-threads=1 -> FAIL in c009_absent_source_true_defaults_remain_true: authentic fable-5 missing eager/temperature becomes false/false, c009_fallback_local_prices_survive_public_store_roundtrip: source two fallback targets with rates disappear from persisted executable record; actual final15-case assertion transcript and source/lock/artifact identities are bound in sdk-compat-proof-summary.json.
- **Green-after**: cargo test -p opi-ai --test model_wire_metadata --test catalog_operations --test anthropic_fixtures --test anthropic_thinking_roundtrip --test node4_preparation --test telemetry_operations --offline --locked -> every named distinguishing assertion and controls PASS; future source tests compile against this exact branch contract.
- **Test impact**: add/update branch-specific tests; retain current signature/identity/cache/strict/opaque/failure/unknownness controls.

### Batch B23: Use the resolved Anthropic cache compatibility when assigning tool markers and long TTL

**Closure predicate**: With cacheenabled, supportsCacheControlOnToolsfalse suppresses only tool cache_control; supportsLongCacheRetentionfalse retains ephemeral markers but omits1h TTL; true/default and Disabled controls obey their branches
**Dependencies**: ["B22"]
**Verification union**: sourceplain default markers,explicitfalse tools,explicitfalse TTL,independent combinations,true defaults,requestDisabled,owned x-opi generic opt-out,inline tool changes last initial-tool marker; applicable checks for the implemented branch and changed targets.

**Downstream final-source proof**: C009 requires all B22-B27 predicates after all six local branches pass. Final workspace gates and B91/B17/B95/B92 integration/package/native proof remain mandatory at those declared owners. No descendant gates this local branch.

#### Fix B23.1: anthropic.cache-marker-compat

- **Finding source(s)**: phase20-codex-gpt61-b3f1810-20261103t142820z / 4cec97af8ef6bb773c630ee251728f31212e12fdc1bf9631f10b2dc3084bc0f6 / P20-AUD-009
- **Decision**: fix:preserve-and-execute-current-source-anthropic-compat-fields-and-defaults
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-ai/src/anthropic.rs`, `crates/opi-ai/tests/anthropic_fixtures.rs`
- **Change kind**: behavioral
- **Change**: Use the resolved Anthropic cache compatibility when assigning tool markers and long TTL
- **Closure predicate**: With cacheenabled, supportsCacheControlOnToolsfalse suppresses only tool cache_control; supportsLongCacheRetentionfalse retains ephemeral markers but omits1h TTL; true/default and Disabled controls obey their branches
- **Red-before**: python scripts/opi-cargo-cache.py run --target E:\opi\cargo-targets\opi-72fa37a0fa044958 -- test --manifest-path C:\Users\Luiz\AppData\Local\Temp\opi-phase20-remediate-plan-jqdki4yk\sdk-compat-probes\Cargo.toml --offline --locked --lib -- --nocapture --test-threads=1 -> FAIL in c009_tool_cache_false_suppresses_only_tool_marker, c009_long_cache_false_omits_ttl; actual final15-case assertion transcript and source/lock/artifact identities are bound in sdk-compat-proof-summary.json.
- **Green-after**: cargo test -p opi-ai --test model_wire_metadata --test catalog_operations --test anthropic_fixtures --test anthropic_thinking_roundtrip --test node4_preparation --test telemetry_operations --offline --locked -> every named distinguishing assertion and controls PASS; future source tests compile against this exact branch contract.
- **Test impact**: add/update branch-specific tests; retain current signature/identity/cache/strict/opaque/failure/unknownness controls.

### Batch B24: Resolve and merge source affinity headers at Anthropic prepared request construction

**Closure predicate**: Exact full nonempty sessionId uses x-session-affinity when enabled/defaultformat, x-session-id for explicit/inferredOpenRouter, explicitfalse/no id emits neither, model/caller headers and null tombstones preserve source precedence
**Dependencies**: ["B22"]
**Verification union**: Fireworksexplicittrue,OpenRouterprovider,OpenRouterURLalias,explicitfalseoverride,explicitformatOpenRouter,empty/nosession,cacheDisabled,headeroverride/tombstone,OAuthbranch; applicable checks for the implemented branch and changed targets.

**Downstream final-source proof**: C009 requires all B22-B27 predicates after all six local branches pass. Final workspace gates and B91/B17/B95/B92 integration/package/native proof remain mandatory at those declared owners. No descendant gates this local branch.

#### Fix B24.1: anthropic.affinity-headers

- **Finding source(s)**: phase20-codex-gpt61-b3f1810-20261103t142820z / 4cec97af8ef6bb773c630ee251728f31212e12fdc1bf9631f10b2dc3084bc0f6 / P20-AUD-009
- **Decision**: fix:preserve-and-execute-current-source-anthropic-compat-fields-and-defaults
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-ai/src/anthropic.rs`, `crates/opi-ai/tests/anthropic_fixtures.rs`
- **Change kind**: behavioral
- **Change**: Inference provider==openrouter OR baseURLcontainsopenrouter.ai. Enable explicit option wins over inference; format explicit wins over inferredOpenRouter otherwise defaultaffinity. Apply only the source API-key/header-owned/federation construction branch; source OAuth client returns earlier. Pinned consumer1075 gates sessionId+enabled, not cache retention; therefore retain affinity even with cacheDisabled. Defaults then affinity then model headers then caller headers; never overwrite a caller tombstone after merge.
- **Closure predicate**: Exact full nonempty sessionId uses x-session-affinity when enabled/defaultformat, x-session-id for explicit/inferredOpenRouter, explicitfalse/no id emits neither, model/caller headers and null tombstones preserve source precedence
- **Red-before**: python scripts/opi-cargo-cache.py run --target E:\opi\cargo-targets\opi-72fa37a0fa044958 -- test --manifest-path C:\Users\Luiz\AppData\Local\Temp\opi-phase20-remediate-plan-jqdki4yk\sdk-compat-probes\Cargo.toml --offline --locked --lib -- --nocapture --test-threads=1 -> FAIL in c009_explicit_affinity_sends_full_session_id; actual final15-case assertion transcript and source/lock/artifact identities are bound in sdk-compat-proof-summary.json.
- **Green-after**: cargo test -p opi-ai --test model_wire_metadata --test catalog_operations --test anthropic_fixtures --test anthropic_thinking_roundtrip --test node4_preparation --test telemetry_operations --offline --locked -> every named distinguishing assertion and controls PASS; future source tests compile against this exact branch contract.
- **Test impact**: add/update branch-specific tests; retain current signature/identity/cache/strict/opaque/failure/unknownness controls.

### Batch B25: Thread allowEmptySignature into Anthropic content conversion after existing canonical identity projection

**Closure predicate**: Sameidentity nonblank thinking with missing/empty/whitespace signature becomes thinking with signatureempty iff flagtrue, otherwise text; real nonblank signature/redacted payload stay unchanged; crossidentity discard and blankunsigned behavior remain source-consistent
**Dependencies**: ["B22"]
**Verification union**: true/false x None/empty/whitespace/nonblank signature x blank/nonblankthinking,redacted,crossidentity; applicable checks for the implemented branch and changed targets.

**Downstream final-source proof**: C009 requires all B22-B27 predicates after all six local branches pass. Final workspace gates and B91/B17/B95/B92 integration/package/native proof remain mandatory at those declared owners. No descendant gates this local branch.

#### Fix B25.1: anthropic.empty-signature-replay

- **Finding source(s)**: phase20-codex-gpt61-b3f1810-20261103t142820z / 4cec97af8ef6bb773c630ee251728f31212e12fdc1bf9631f10b2dc3084bc0f6 / P20-AUD-009
- **Decision**: fix:preserve-and-execute-current-source-anthropic-compat-fields-and-defaults
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-ai/src/anthropic.rs`, `crates/opi-ai/tests/anthropic_thinking_roundtrip.rs`
- **Change kind**: behavioral
- **Change**: Do not make canonical replay globally permissive. Existing replay retains sameidentity nonblank unsigned material; adjust wire-specific conversion map to filter/map source branches. Source blank thinking and missing/whitespace signature drops at conversion; signed nonblank retains exact signature and sanitized text. Replay facts/strict mode still apply to identity loss.
- **Closure predicate**: Sameidentity nonblank thinking with missing/empty/whitespace signature becomes thinking with signatureempty iff flagtrue, otherwise text; real nonblank signature/redacted payload stay unchanged; crossidentity discard and blankunsigned behavior remain source-consistent
- **Red-before**: python scripts/opi-cargo-cache.py run --target E:\opi\cargo-targets\opi-72fa37a0fa044958 -- test --manifest-path C:\Users\Luiz\AppData\Local\Temp\opi-phase20-remediate-plan-jqdki4yk\sdk-compat-probes\Cargo.toml --offline --locked --lib -- --nocapture --test-threads=1 -> FAIL in c009_allow_empty_signature_materializes_empty_wire_signature, c009_disallow_empty_signature_projects_thinking_as_text; actual final15-case assertion transcript and source/lock/artifact identities are bound in sdk-compat-proof-summary.json.
- **Green-after**: cargo test -p opi-ai --test model_wire_metadata --test catalog_operations --test anthropic_fixtures --test anthropic_thinking_roundtrip --test node4_preparation --test telemetry_operations --offline --locked -> every named distinguishing assertion and controls PASS; future source tests compile against this exact branch contract.
- **Test impact**: add/update branch-specific tests; retain current signature/identity/cache/strict/opaque/failure/unknownness controls.

### Batch B26: Compile source managed-effort transcript and seed response effort from that compiled request

**Closure predicate**: For flagged model with thinkingnotdisabled, sameprovider+exactAnthropicMessageswire historical supported effort markers precede corresponding assistant turns and current effort marker ends transcript; wire adaptive block_binding drop_block/output_confighigh/bothbetas and terminal provider_thinking_level match source; disabled/unflagged controls omit managed sequence
**Dependencies**: ["B22"]
**Verification union**: activecurrenteffort,omittedhigh,historicalsameprovider,differentprovider/unsupportedhistorical,nohistory,explicitoffallowed/offnull,unflagged,tempomission,replyStart/Done/failurestamp; applicable checks for the implemented branch and changed targets.

**Downstream final-source proof**: C009 requires all B22-B27 predicates after all six local branches pass. Final workspace gates and B91/B17/B95/B92 integration/package/native proof remain mandatory at those declared owners. No descendant gates this local branch.

#### Fix B26.1: anthropic.mid-conversation-effort

- **Finding source(s)**: phase20-codex-gpt61-b3f1810-20261103t142820z / 4cec97af8ef6bb773c630ee251728f31212e12fdc1bf9631f10b2dc3084bc0f6 / P20-AUD-009
- **Decision**: fix:preserve-and-execute-current-source-anthropic-compat-fields-and-defaults
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-ai/src/anthropic.rs`, `crates/opi-ai/tests/anthropic_fixtures.rs`, `crates/opi-ai/tests/anthropic_thinking_roundtrip.rs`
- **Change kind**: behavioral
- **Change**: Reuse existing AnthropicMessagesOptions effort/thinkingEnabled/display and existing AssistantMessage.provider_thinking_level; no new public field. activeeffort=nativeeffectiveeffort or sourcehigh. Emit historical effort only supported low/medium/high/xhigh/max, sameprovider, wireSome(AnthropicMessages) (source1493-1498 checks exactapi, not coarse ApiKind and does not require exact model match); preserve pinned conversion ordering. Suppress temperature whenever supportsMidConvoEfforttrue, including disabled branch. Disabled only when modelreasoning and thinkingEnabledfalse and offmappingnotnull; emits disabled with no binding or marker. Flagbetas emitted regardless of active managed branch exactly source1153. Do not add global effort normalizer.
- **Closure predicate**: For flagged model with thinkingnotdisabled, sameprovider+exactAnthropicMessageswire historical supported effort markers precede corresponding assistant turns and current effort marker ends transcript; wire adaptive block_binding drop_block/output_confighigh/bothbetas and terminal provider_thinking_level match source; disabled/unflagged controls omit managed sequence
- **Red-before**: python scripts/opi-cargo-cache.py run --target E:\opi\cargo-targets\opi-72fa37a0fa044958 -- test --manifest-path C:\Users\Luiz\AppData\Local\Temp\opi-phase20-remediate-plan-jqdki4yk\sdk-compat-probes\Cargo.toml --offline --locked --lib -- --nocapture --test-threads=1 -> FAIL in c009_mid_effort_emits_binding_and_stamps_response; actual final15-case assertion transcript and source/lock/artifact identities are bound in sdk-compat-proof-summary.json.
- **Green-after**: cargo test -p opi-ai --test model_wire_metadata --test catalog_operations --test anthropic_fixtures --test anthropic_thinking_roundtrip --test node4_preparation --test telemetry_operations --offline --locked -> every named distinguishing assertion and controls PASS; future source tests compile against this exact branch contract.
- **Test impact**: add/update branch-specific tests; retain current signature/identity/cache/strict/opaque/failure/unknownness controls.

### Batch B27: Carry allowed fallback declaration through request and the source server-fallback response state machine

**Closure predicate**: Nonempty allowed list emits ordered modelonly fallbacks and exactbeta; absent/empty omitsboth. Leading fallback marker is accepted without fakecontent, later output retains requested model and reportedresponseModel, midoutputfallback returns typed StreamError with canonicalpartial. Local cost resolution selects only matching sameprovider+responseModel allowed rates and otherwise original model rates.
**Dependencies**: ["B22"]
**Verification union**: absent/empty/1/3targets,>3reject,missingprovider/model/costreject,orderedmodelonlypayload,beta,leadingmarkerthenoutput,midoutputtypedfailure+partial,reportedallowedmodelcost,sameidcost,unlisted/crossproviderfallbackcostbase,tiers/cachewrite1h,unknownprice/usageNone; applicable checks for the implemented branch and changed targets.

**Downstream final-source proof**: C009 requires all B22-B27 predicates after all six local branches pass. Final workspace gates and B91/B17/B95/B92 integration/package/native proof remain mandatory at those declared owners. No descendant gates this local branch.

#### Fix B27.1: anthropic.server-fallback

- **Finding source(s)**: phase20-codex-gpt61-b3f1810-20261103t142820z / 4cec97af8ef6bb773c630ee251728f31212e12fdc1bf9631f10b2dc3084bc0f6 / P20-AUD-009
- **Decision**: fix:preserve-and-execute-current-source-anthropic-compat-fields-and-defaults
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-ai/src/anthropic.rs`, `crates/opi-ai/src/model_info.rs`, `crates/opi-ai/tests/anthropic_fixtures.rs`, `crates/opi-ai/tests/model_wire_metadata.rs`
- **Change kind**: behavioral
- **Change**: Add raw/internal fallback event variant, ignore it only before any outputcontent, fail after contentwith source unsupportedmidoutput cause; ensure ignored source block index never shifts later text/tool block addressing. Preserve response_model reported value and requested model separately. Request beta server-side-fallback-2026-07-01; serialize models only, costs remain local. Reuse ModelPricing/calculate_cost_tiered via inherent public ModelInfo::calculate_response_cost(&AssistantMessage)->Option<CostBreakdown>: Anthropic selects allowed entry only if response.response_model differs from self.id and fallback.provider==response.provider and fallback.model==response.response_model; otherwise self.pricing. Then calculate with response usage and selected rates, preserving knownness/tier/cacheWrite1h behavior. Existing lowlevel calculate_cost_tiered stays available and unchanged for explicit pricing callers. No new AssistantMessage cost field, no provider fact rewrite, no fallbacknetworkcall or retry policy.
- **Closure predicate**: Nonempty allowed list emits ordered modelonly fallbacks and exactbeta; absent/empty omitsboth. Leading fallback marker is accepted without fakecontent, later output retains requested model and reportedresponseModel, midoutputfallback returns typed StreamError with canonicalpartial. Local cost resolution selects only matching sameprovider+responseModel allowed rates and otherwise original model rates.
- **Red-before**: python scripts/opi-cargo-cache.py run --target E:\opi\cargo-targets\opi-72fa37a0fa044958 -- test --manifest-path C:\Users\Luiz\AppData\Local\Temp\opi-phase20-remediate-plan-jqdki4yk\sdk-compat-probes\Cargo.toml --offline --locked --lib -- --nocapture --test-threads=1 -> FAIL in c009_allowed_fallbacks_reach_request, c009_fallback_local_prices_survive_public_store_roundtrip, c009_leading_fallback_block_is_not_a_parse_failure; actual final15-case assertion transcript and source/lock/artifact identities are bound in sdk-compat-proof-summary.json.
- **Green-after**: cargo test -p opi-ai --test model_wire_metadata --test catalog_operations --test anthropic_fixtures --test anthropic_thinking_roundtrip --test node4_preparation --test telemetry_operations --offline --locked -> every named distinguishing assertion and controls PASS; future source tests compile against this exact branch contract.
- **Test impact**: add/update branch-specific tests; retain current signature/identity/cache/strict/opaque/failure/unknownness controls.

Control limit: Current midoutputcontrol passes via malformed SSE classification, preservingpartial; future green must prove explicit server-fallback semantic cause, not treat this control as existingcorrectparser.

The response-aware cost lookup is fixed as `ModelInfo::calculate_response_cost(&AssistantMessage) -> Option<CostBreakdown>` using the existing calculator and only a matching same-provider declared fallback rate. Preserve requested model and actual response_model separately; no new AssistantMessage cost field, implicit retry, new network call or account selection.

### Batch B29: bind active logical cancellation to private WS decoder

**Closure predicate**: Every active Codex decompression backend call and publication observes the same logical request token, and a pooled connection never retains an earlier completed request token.
**Dependencies**: []
**Verification union**: python scripts/opi-cargo-cache.py run --target <inspected-persistent-target> -- test -p opi-ai --lib ws::tests::logical_cancel_probe --offline --locked -- --nocapture; python scripts/opi-cargo-cache.py run --target <inspected-persistent-target> -- test -p opi-ai --test openai_codex_responses --offline --locked -- --nocapture; applicable final workspace gates and exact changed targets; documentation and whitespace checks.

#### Fix B29.1: bind active logical cancellation to private WS decoder

- **Finding source(s)**: phase20-codex-gpt61-b3f1810-20261103t142820z / 4cec97af8ef6bb773c630ee251728f31212e12fdc1bf9631f10b2dc3084bc0f6 / P20-AUD-018
- **Decision**: fix:bind-active-logical-cancellation-to-private-WS-decoder
- **Verification status**: Partially confirmed
- **File(s)**: `crates/opi-ai/src/ws/pmd.rs`, `crates/opi-ai/src/ws/io.rs`, `crates/opi-ai/src/openai_codex_responses/ws_transport.rs`, `crates/opi-ai/src/ws/tests/logical_cancel_probe.rs`, `crates/opi-ai/src/ws/tests/mod.rs`
- **Change kind**: behavioral
- **Change**: Bind the current logical cancellation token privately for each active run_stream/lease, propagate to Driver/Decoder and observe before each backend call and publication; clear/rebind before pool reuse. Do not freeze the first request token in a long-lived pooled decoder. Preserve existing abort/nonreusable behavior and idle cancellation wake ownership; no public API/config/trait.
- **Closure predicate**: Every active Codex decompression backend call and publication observes the same logical request token, and a pooled connection never retains an earlier completed request token.
- **Red-before**: FAIL: actual Models→Codex→Upgrade→PMD, same Request token cancelled after first actual inflate; control3calls/1callback, cancelled3calls including2after cancellation/0callback;1executed/1failed/0ignored, exit101. Receipt C:\Users\Luiz\AppData\Local\Temp\opi-ws023-instrumented-z20k2lui\probe-identities.json SHA256 fc24e122c55944b38402406d8c8e03179fd723cca1a6f20905464a9fe990cf67. One cfg(test) observer; original production body is recoverable exactly.
- **Green-after**: python scripts/opi-cargo-cache.py run --target <inspected-persistent-target> -- test -p opi-ai --lib ws::tests::logical_cancel_probe --offline --locked -- --nocapture; python scripts/opi-cargo-cache.py run --target <inspected-persistent-target> -- test -p opi-ai --test openai_codex_responses --offline --locked -- --nocapture -> Every active Codex decompression backend call and publication observes the same logical request token, and a pooled connection never retains an earlier completed request token. Observed: not-run.
- **Test impact**: add the exact discriminating cases; update affected public callers and fixtures; retain valid-source, typed-failure, cancellation and secret-canary controls; delete none.

Exact future cases and controls:

```json
[
  {
    "name": "ws023_actual_codex_observes_logical_cancel_before_each_backend_call",
    "input": "Actual public Models→Codex→Upgrade→Driver; observer cancels Request token after first actual inflate.",
    "expected": "Zero later backend calls and zero callbacks after cancellation.",
    "controls": [
      "uncancelled identical frame completes with one callback and multiple backend calls",
      "test observer never sets private Decoder cancellation flag"
    ]
  },
  {
    "name": "pooled_codex_decoder_binds_each_active_request_token",
    "input": "Successful call A returns pool entry; cancel completed A token; call B on same entry uses fresh uncancelled token; then cancel B while inflating.",
    "expected": "B succeeds despite cancelled A; B cancellation suppresses its subsequent backend work; no reused failed decoder.",
    "controls": [
      "assert connection/lease reuse actually occurred before claiming reset coverage"
    ]
  }
]
```

### Batch B30: preserve optional sampling map presence

**Closure predicate**: Absent and explicit empty optional sampling maps remain distinguishable through public model/request values, owned store/reopen, and the base < effective-level < request shallow merge, without changing supported-wire application policy.
**Dependencies**: ["B15"]
**Verification union**: cargo test -p opi-ai --test model_wire_metadata --test catalog_lifecycle --test openai_chat_fixtures --test request_enrichment --offline --locked; applicable final workspace gates and exact changed targets; documentation and whitespace checks.

#### Fix B30.1: preserve optional sampling map presence

- **Finding source(s)**: phase20-codex-gpt61-b3f1810-20261103t142820z / 4cec97af8ef6bb773c630ee251728f31212e12fdc1bf9631f10b2dc3084bc0f6 / P20-AUD-018
- **Decision**: fix:preserve-optional-sampling-map-presence
- **Verification status**: Partially confirmed
- **File(s)**: `crates/opi-ai/src/model_info.rs`, `crates/opi-ai/src/catalog/wire.rs`, `crates/opi-ai/src/provider.rs`, `crates/opi-ai/src/openai_chat.rs`, `crates/opi-ai/src/openai_responses.rs`, `crates/opi-ai/tests/model_wire_metadata.rs`, `crates/opi-ai/tests/catalog_lifecycle.rs`, `crates/opi-ai/tests/openai_chat_fixtures.rs`, `crates/opi-ai/tests/request_enrichment.rs`, `crates/opi-ai/README.md`, `crates/opi-ai/README.zh.md`, `CHANGELOG.md`
- **Change kind**: behavioral
- **Change**: Represent the optional model/request sampling maps as Option<SamplingParams>; preserve absent versus explicit empty and the existing shallow base < effective-level < request merge.
- **Closure predicate**: Absent and explicit empty optional sampling maps remain distinguishable through public model/request values, owned store/reopen, and the base < effective-level < request shallow merge, without changing supported-wire application policy.
- **Red-before**: FAIL: python scripts/opi-cargo-cache.py run --target E:\opi\cargo-targets\opi-72fa37a0fa044958 -- test --manifest-path C:\Users\Luiz\AppData\Local\Temp\opi-phase20-remediate-plan-jqdki4yk\c018-sampling-probes\Cargo.toml --offline --locked --lib -- --nocapture --test-threads=1;3executed/2passed/1failed/0ignored,exit101. Actual public catalog→ModelInfo→ModelsStore emits absent samplingParams as an empty object. Explicit-empty and nonempty controls pass; receipt `C:\Users\Luiz\AppData\Local\Temp\opi-phase20-remediate-plan-jqdki4yk\c018-sdk-probe-receipts.json` SHA256 `a105e293c8e3072eec7244c15f31814ed4f2fa4bafd711682c913576f0465b83`.
- **Green-after**: cargo test -p opi-ai --test model_wire_metadata --test catalog_lifecycle --test openai_chat_fixtures --test request_enrichment --offline --locked -> Absent and explicit empty optional sampling maps remain distinguishable through public model/request values, owned store/reopen, and the base < effective-level < request shallow merge, without changing supported-wire application policy. Observed: not-run.
- **Test impact**: add the exact discriminating cases; update affected public callers and fixtures; retain valid-source, typed-failure, cancellation and secret-canary controls; delete none.

- ModelInfo.sampling_params and Request.sampling_params become Option<SamplingParams>; keep SamplingParams map alias. Default is None, builder with_sampling_params(map) sets Some(map), including empty.
- Keep per-level BTreeMap<ThinkingLevel, SamplingParams>: absence of a level key versus a present empty map is already represented. Do not introduce nullable level values without a source requirement.
- resolve_sampling_params accepts optional request map (Option<&SamplingParams>); returns None only when model map, effective-level entry and request map are all absent; otherwise Some(shallow merged map), including empty. Preserve source merge order and nested-value replacement.
- catalog/wire only emits samplingParams when present and decodes absent to None, {} to Some(empty); x-opi precedence over source value remains unchanged; invalid nonobject stays typed error.
- Only Chat/Responses/Azure consumers apply arbitrary keys as before, after named fields and before payload callback. Other wires may retain metadata without effective-application claims.
- Exact caller synchronization: model_wire_metadata accesses/indexing and helper arguments; openai_chat_fixtures3025 literal map; request_enrichment717 assignment. Other inspected Request literal sites use Default::default() and continue as None without edits. No blanket 80-file refactor.

Exact cases:

```json
[
  {
    "name": "sampling_presence_survives_catalog_store_reopen",
    "inputs": "None,{},nonempty; source vs x-opi; three values",
    "expected": "None remains omitted; explicit empty retained; nonempty value retained; reopening has same typed presence"
  },
  {
    "name": "sampling_presence_shallow_merge_matrix",
    "inputs": "model/request absent or empty; effective-level absent or empty; base a1/b1, level b2/nested{x2}, request c3/nested{y3}",
    "expected": "all absent None; any explicit empty Someempty; request nested replaces entire lower object; precedence a1,b2,c3"
  },
  {
    "name": "sampling_presence_reaches_supported_wires_once",
    "inputs": "Chat/Responses/Azure local captures, named temperature plus maps; unsupported Anthropic control",
    "expected": "same final keys and one payload callback; unsupported wire does not silently apply arbitrary map"
  }
]
```

### Batch B32: keep custom Radius gateway catalog initially empty

**Closure predicate**: PiMessagesProvider::new admits bundled Radius baseline only for no gateway or the exact built-in BASE_URL; an explicit nondefault gateway starts with an empty model catalog and still supports caller upsert through the existing seam.
**Dependencies**: []
**Verification union**: cargo test -p opi-ai --test chat_pi_messages --test catalog_lifecycle --offline --locked; run every created or modified target/filter and affected-target clippy; final workspace/doc/whitespace union applies.

#### Fix B32.1: keep custom Radius gateway catalog initially empty

- **Finding source(s)**: phase20-codex-gpt61-b3f1810-20261103t142820z / 4cec97af8ef6bb773c630ee251728f31212e12fdc1bf9631f10b2dc3084bc0f6 / P20-AUD-018
- **Decision**: fix:keep-custom-Radius-gateway-catalog-initially-empty
- **Verification status**: Partially confirmed
- **File(s)**: `crates/opi-ai/src/pi_messages.rs`, `crates/opi-ai/tests/chat_pi_messages.rs`, `crates/opi-ai/README.md`, `crates/opi-ai/README.zh.md`, `CHANGELOG.md`
- **Change kind**: behavioral
- **Change**: Apply only the bounded production decision and synchronized tests below.
- **Closure predicate**: PiMessagesProvider::new admits bundled Radius baseline only for no gateway or the exact built-in BASE_URL; an explicit nondefault gateway starts with an empty model catalog and still supports caller upsert through the existing seam.
- **Red-before**: FAIL: exact command/lock/source in `C:\Users\Luiz\AppData\Local\Temp\opi-phase20-remediate-plan-jqdki4yk\c018-radius-receipt.json` SHA256 `89779656b4a261f5f089fdf19c826e40b56590b50779aea95912fbefb9113fbb`;2tests/1pass/1fail/0ignored,exit101. Default no-gateway/exactBASE_URL control passes; customHTTPS constructor returns one baseline model, expected zero.
- **Green-after**: cargo test -p opi-ai --test chat_pi_messages --test catalog_lifecycle --offline --locked -> PiMessagesProvider::new admits bundled Radius baseline only for no gateway or the exact built-in BASE_URL; an explicit nondefault gateway starts with an empty model catalog and still supports caller upsert through the existing seam. Observed:not-run.
- **Test impact**: add named assertions/cases; update helpers only as required; retain valid-source and negative controls; delete none.

Change only the existing new(custom_gateway) baseline selection. Retain default None/exactBASE_URL positive controls, nondefault local gateway empty control, caller-owned complete upsert/actual request and catalog empty/reopen no-resurrection controls. Custom provider id/name input is a distinct static missing-input question; this gateway repair does not invent or seal that public declaration.

### Batch B33: schema boundary conformance

**Closure predicate**: Actual public argument finalization rejects non-JSON UTF16/nonfinite syntax with raw provenance and typed arguments_error before schema coercion; valid surrogate pair remains valid.
**Dependencies**: []
**Verification union**: python scripts/opi-cargo-cache.py run --target <inspected-persistent-target> -- test -p opi-ai --test schema_public public_argument_finalizer_rejects_non_json_utf16_without_replacement --offline --locked -- --nocapture; run every created or modified target/filter and affected-target clippy; final workspace/doc/whitespace union applies.

#### Fix B33.1: schema boundary conformance

- **Finding source(s)**: phase20-codex-gpt61-b3f1810-20261103t142820z / 4cec97af8ef6bb773c630ee251728f31212e12fdc1bf9631f10b2dc3084bc0f6 / P20-AUD-018
- **Decision**: fix:schema-boundary-conformance
- **Verification status**: Partially confirmed
- **File(s)**: `crates/opi-ai/tests/schema_public.rs`
- **Change kind**: test-only
- **Change**: Add the exact finite assertion matrix below at its current owning seam; production behavior and dependencies are unchanged by this test-only batch.
- **Closure predicate**: Actual public argument finalization rejects non-JSON UTF16/nonfinite syntax with raw provenance and typed arguments_error before schema coercion; valid surrogate pair remains valid.
- **Red-before**: Not applicable to production behavior: this batch adds the finite source-defined conformance test only. Preserve any actual baseline failure and stop for its separately bound behavioral repair; no conditional production edit is admitted.
- **Green-after**: python scripts/opi-cargo-cache.py run --target <inspected-persistent-target> -- test -p opi-ai --test schema_public public_argument_finalizer_rejects_non_json_utf16_without_replacement --offline --locked -- --nocapture -> Actual public argument finalization rejects non-JSON UTF16/nonfinite syntax with raw provenance and typed arguments_error before schema coercion; valid surrogate pair remains valid. Observed:not-run.
- **Test impact**: add named assertions/cases; update helpers only as required; retain valid-source and negative controls; delete none.

Exact source inputs, expected results and connected controls:

```json
[
  {
    "name": "public_argument_finalizer_rejects_non_json_utf16_without_replacement",
    "input": [
      "{\"x\":\"\\ud800\"}",
      "{\"x\":\"\\udfff\"}",
      "{\"x\":\"\\ud800x\"}",
      "{\"x\":NaN}",
      "{\"x\":Infinity}",
      "{\"x\":undefined}",
      "{\"x\":Symbol(\"x\")}"
    ],
    "expected": "finalize_tool_arguments retains each exact raw string and yields arguments_error; validate_tool_arguments returns ArgumentsError and cannot coerce it; no replacement U+FFFD introduced",
    "controls": [
      "{\"x\":\"\\ud83d\\ude00\"} decodes the actual scalar and passes string schema",
      "already parsed ordinary object passes and trusted schema is unchanged"
    ]
  }
]
```

For new test-only cases, first execute on the committed baseline. A failure does not authorize silently changing production; preserve it and give the owning source defect a distinct behavioral batch.

### Batch B34: schema applicators conformance

**Closure predicate**: Every visited applicator propagates the same typed URI leaf failure; unvisited short-circuited leaves remain unevaluated.
**Dependencies**: []
**Verification union**: python scripts/opi-cargo-cache.py run --target <inspected-persistent-target> -- test -p opi-ai --test schema_engine visited_applicators_propagate_uri_errors_and_unvisited_branches_do_not --offline --locked -- --nocapture; run every created or modified target/filter and affected-target clippy; final workspace/doc/whitespace union applies.

#### Fix B34.1: schema applicators conformance

- **Finding source(s)**: phase20-codex-gpt61-b3f1810-20261103t142820z / 4cec97af8ef6bb773c630ee251728f31212e12fdc1bf9631f10b2dc3084bc0f6 / P20-AUD-018
- **Decision**: fix:schema-applicators-conformance
- **Verification status**: Partially confirmed
- **File(s)**: `crates/opi-ai/tests/schema_engine.rs`
- **Change kind**: test-only
- **Change**: Add the exact finite assertion matrix below at its current owning seam; production behavior and dependencies are unchanged by this test-only batch.
- **Closure predicate**: Every visited applicator propagates the same typed URI leaf failure; unvisited short-circuited leaves remain unevaluated.
- **Red-before**: Not applicable to production behavior: this batch adds the finite source-defined conformance test only. Preserve any actual baseline failure and stop for its separately bound behavioral repair; no conditional production edit is admitted.
- **Green-after**: python scripts/opi-cargo-cache.py run --target <inspected-persistent-target> -- test -p opi-ai --test schema_engine visited_applicators_propagate_uri_errors_and_unvisited_branches_do_not --offline --locked -- --nocapture -> Every visited applicator propagates the same typed URI leaf failure; unvisited short-circuited leaves remain unevaluated. Observed:not-run.
- **Test impact**: add named assertions/cases; update helpers only as required; retain valid-source and negative controls; delete none.

Exact source inputs, expected results and connected controls:

```json
[
  {
    "name": "visited_applicators_propagate_uri_errors_and_unvisited_branches_do_not",
    "input": "Runtime-invalid ref {\"$ref\":\"http://[\"} under not, if, matching patternProperties, allOf, first anyOf and selected then/else.",
    "expected": "Every visited leaf returns CheckError::Uri rather than false/true; check exact discriminant.",
    "controls": [
      "anyOf [true,badref] returns true",
      "false if skips then badref",
      "true if skips else badref",
      "nonmatching property skips patternProperties badref"
    ]
  }
]
```

For new test-only cases, first execute on the committed baseline. A failure does not authorize silently changing production; preserve it and give the owning source defect a distinct behavioral batch.

### Batch B35: schema resolver conformance

**Closure predicate**: Encoded JSON-pointer segments and percent fragments resolve the selected target, preserving lexical bases and matched/invalid behavior.
**Dependencies**: []
**Verification union**: python scripts/opi-cargo-cache.py run --target <inspected-persistent-target> -- test -p opi-ai --test schema_engine encoded_pointer_targets_preserve_selected_resource_identity --offline --locked -- --nocapture; python scripts/opi-cargo-cache.py run --target <inspected-persistent-target> -- test -p opi-ai --test schema_engine admission_ --offline --locked -- --nocapture; python scripts/opi-cargo-cache.py run --target <inspected-persistent-target> -- test -p opi-ai --test schema_engine dynamic_ --offline --locked -- --nocapture; python scripts/opi-cargo-cache.py run --target <inspected-persistent-target> -- test -p opi-ai --test schema_engine recursive_ --offline --locked -- --nocapture; run every created or modified target/filter and affected-target clippy; final workspace/doc/whitespace union applies.

#### Fix B35.1: schema resolver conformance

- **Finding source(s)**: phase20-codex-gpt61-b3f1810-20261103t142820z / 4cec97af8ef6bb773c630ee251728f31212e12fdc1bf9631f10b2dc3084bc0f6 / P20-AUD-018
- **Decision**: fix:schema-resolver-conformance
- **Verification status**: Partially confirmed
- **File(s)**: `crates/opi-ai/tests/schema_engine.rs`
- **Change kind**: test-only
- **Change**: Add the exact finite assertion matrix below at its current owning seam; production behavior and dependencies are unchanged by this test-only batch.
- **Closure predicate**: Encoded JSON-pointer segments and percent fragments resolve the selected target, preserving lexical bases and matched/invalid behavior.
- **Red-before**: Not applicable to production behavior: this batch adds the finite source-defined conformance test only. Preserve any actual baseline failure and stop for its separately bound behavioral repair; no conditional production edit is admitted.
- **Green-after**: python scripts/opi-cargo-cache.py run --target <inspected-persistent-target> -- test -p opi-ai --test schema_engine encoded_pointer_targets_preserve_selected_resource_identity --offline --locked -- --nocapture; python scripts/opi-cargo-cache.py run --target <inspected-persistent-target> -- test -p opi-ai --test schema_engine admission_ --offline --locked -- --nocapture; python scripts/opi-cargo-cache.py run --target <inspected-persistent-target> -- test -p opi-ai --test schema_engine dynamic_ --offline --locked -- --nocapture; python scripts/opi-cargo-cache.py run --target <inspected-persistent-target> -- test -p opi-ai --test schema_engine recursive_ --offline --locked -- --nocapture -> Encoded JSON-pointer segments and percent fragments resolve the selected target, preserving lexical bases and matched/invalid behavior. Observed:not-run.
- **Test impact**: add named assertions/cases; update helpers only as required; retain valid-source and negative controls; delete none.

Exact source inputs, expected results and connected controls:

```json
[
  {
    "name": "encoded_pointer_targets_preserve_selected_resource_identity",
    "input": "Definitions with literal slash and tilde keys, refs #/$defs/a~1b and #/$defs/a~0b; percent-encoded Unicode key; same fragment under two different nested resource IDs.",
    "expected": "Each ref checks the intended distinct schema, including opposite positive/negative instances; trusted input unchanged.",
    "controls": [
      "unencoded distinct key resolves its own target",
      "unknown fragment is the existing typed unresolved outcome; no fallback to another resource"
    ]
  }
]
```

For new test-only cases, first execute on the committed baseline. A failure does not authorize silently changing production; preserve it and give the owning source defect a distinct behavioral batch.

### Batch B36: schema formats conformance

**Closure predicate**: All 21 fixed formats have public compile/check positive and negative controls; unknown formats and nonstring instances remain inert.
**Dependencies**: []
**Verification union**: python scripts/opi-cargo-cache.py run --target <inspected-persistent-target> -- test -p opi-ai --test schema_public public_default_formats_cover_all_twenty_one_source_algorithms --offline --locked -- --nocapture; run every created or modified target/filter and affected-target clippy; final workspace/doc/whitespace union applies.

#### Fix B36.1: schema formats conformance

- **Finding source(s)**: phase20-codex-gpt61-b3f1810-20261103t142820z / 4cec97af8ef6bb773c630ee251728f31212e12fdc1bf9631f10b2dc3084bc0f6 / P20-AUD-018
- **Decision**: fix:schema-formats-conformance
- **Verification status**: Partially confirmed
- **File(s)**: `crates/opi-ai/tests/schema_public.rs`
- **Change kind**: test-only
- **Change**: Add the exact finite assertion matrix below at its current owning seam; production behavior and dependencies are unchanged by this test-only batch.
- **Closure predicate**: All 21 fixed formats have public compile/check positive and negative controls; unknown formats and nonstring instances remain inert.
- **Red-before**: Not applicable to production behavior: this batch adds the finite source-defined conformance test only. Preserve any actual baseline failure and stop for its separately bound behavioral repair; no conditional production edit is admitted.
- **Green-after**: python scripts/opi-cargo-cache.py run --target <inspected-persistent-target> -- test -p opi-ai --test schema_public public_default_formats_cover_all_twenty_one_source_algorithms --offline --locked -- --nocapture -> All 21 fixed formats have public compile/check positive and negative controls; unknown formats and nonstring instances remain inert. Observed:not-run.
- **Test impact**: add named assertions/cases; update helpers only as required; retain valid-source and negative controls; delete none.

Exact source inputs, expected results and connected controls:

```json
[
  {
    "name": "public_default_formats_cover_all_twenty_one_source_algorithms",
    "input": "Public table copies positive/negative case identities from schema/formats.rs:567-733 for the21 named formats. Additional precise negatives: uri-reference \"https://example.com/%zz\" ->false because P_URI_REFERENCE admits percent only with2hex; uri-template \"https://example.com/{id\" ->false because P_URI_TEMPLATE requires closing brace. Both source patterns are current format_patterns.rs:96/101.",
    "expected": "Each public CompiledSchema result equals its predetermined table boolean; exactly21 format names, at least one positive and one negative each. No oracle selected after observing a test result.",
    "controls": [
      "unknown source format admits arbitrary string",
      "every format ignores nonstring instance absent a type restriction"
    ]
  }
]
```

For new test-only cases, first execute on the committed baseline. A failure does not authorize silently changing production; preserve it and give the owning source defect a distinct behavioral batch.

### Batch B37: schema generation conformance

**Closure predicate**: Owned compiled and prepared-call generations survive caller replacement without mutation; subsequent preparations observe replacement.
**Dependencies**: []
**Verification union**: python scripts/opi-cargo-cache.py run --target <inspected-persistent-target> -- test -p opi-ai --test schema_public compiled_schema_generations_keep_original_declarations_after_replacement --offline --locked -- --nocapture; python scripts/opi-cargo-cache.py run --target <inspected-persistent-target> -- test -p opi-ai --test provider_collection prepared_calls_keep_schema_generation_while_new_calls_take_replacement --offline --locked -- --nocapture; run every created or modified target/filter and affected-target clippy; final workspace/doc/whitespace union applies.

#### Fix B37.1: schema generation conformance

- **Finding source(s)**: phase20-codex-gpt61-b3f1810-20261103t142820z / 4cec97af8ef6bb773c630ee251728f31212e12fdc1bf9631f10b2dc3084bc0f6 / P20-AUD-018
- **Decision**: fix:schema-generation-conformance
- **Verification status**: Partially confirmed
- **File(s)**: `crates/opi-ai/tests/schema_public.rs`, `crates/opi-ai/tests/provider_collection.rs`
- **Change kind**: test-only
- **Change**: Add the exact finite assertion matrix below at its current owning seam; production behavior and dependencies are unchanged by this test-only batch.
- **Closure predicate**: Owned compiled and prepared-call generations survive caller replacement without mutation; subsequent preparations observe replacement.
- **Red-before**: Not applicable to production behavior: this batch adds the finite source-defined conformance test only. Preserve any actual baseline failure and stop for its separately bound behavioral repair; no conditional production edit is admitted.
- **Green-after**: python scripts/opi-cargo-cache.py run --target <inspected-persistent-target> -- test -p opi-ai --test schema_public compiled_schema_generations_keep_original_declarations_after_replacement --offline --locked -- --nocapture; python scripts/opi-cargo-cache.py run --target <inspected-persistent-target> -- test -p opi-ai --test provider_collection prepared_calls_keep_schema_generation_while_new_calls_take_replacement --offline --locked -- --nocapture -> Owned compiled and prepared-call generations survive caller replacement without mutation; subsequent preparations observe replacement. Observed:not-run.
- **Test impact**: add named assertions/cases; update helpers only as required; retain valid-source and negative controls; delete none.

Exact source inputs, expected results and connected controls:

```json
[
  {
    "name": "compiled_schema_generations_keep_original_declarations_after_replacement",
    "input": "Compile old string schema; hold an Arc clone; replace caller-owned declaration with integer schema and compile new generation.",
    "expected": "old accepts string/rejects integer; new does inverse; all original schema bytes unchanged.",
    "controls": [
      "drop new owner and repeat old check"
    ]
  },
  {
    "name": "prepared_calls_keep_schema_generation_while_new_calls_take_replacement",
    "input": "Create first Models call before replacing tool declaration in next request; dispatch first and second to counted local fixture.",
    "expected": "First actual wire retains original declaration, second has replacement, original canonical requests unchanged.",
    "controls": [
      "two fresh calls without replacement have identical schema bytes"
    ]
  }
]
```

For new test-only cases, first execute on the committed baseline. A failure does not authorize silently changing production; preserve it and give the owning source defect a distinct behavioral batch.

### Batch B38: schema cache conformance

**Closure predicate**: Cold and reused normalization have identical outcomes after many distinct subvalidators; finite work size is not an admission quota.
**Dependencies**: []
**Verification union**: python scripts/opi-cargo-cache.py run --target <inspected-persistent-target> -- test -p opi-ai --test schema_public normalization_cache_misses_never_become_schema_admission_quota --offline --locked -- --nocapture; run every created or modified target/filter and affected-target clippy; final workspace/doc/whitespace union applies.

#### Fix B38.1: schema cache conformance

- **Finding source(s)**: phase20-codex-gpt61-b3f1810-20261103t142820z / 4cec97af8ef6bb773c630ee251728f31212e12fdc1bf9631f10b2dc3084bc0f6 / P20-AUD-018
- **Decision**: fix:schema-cache-conformance
- **Verification status**: Partially confirmed
- **File(s)**: `crates/opi-ai/tests/schema_public.rs`
- **Change kind**: test-only
- **Change**: Add the exact finite assertion matrix below at its current owning seam; production behavior and dependencies are unchanged by this test-only batch.
- **Closure predicate**: Cold and reused normalization have identical outcomes after many distinct subvalidators; finite work size is not an admission quota.
- **Red-before**: Not applicable to production behavior: this batch adds the finite source-defined conformance test only. Preserve any actual baseline failure and stop for its separately bound behavioral repair; no conditional production edit is admitted.
- **Green-after**: python scripts/opi-cargo-cache.py run --target <inspected-persistent-target> -- test -p opi-ai --test schema_public normalization_cache_misses_never_become_schema_admission_quota --offline --locked -- --nocapture -> Cold and reused normalization have identical outcomes after many distinct subvalidators; finite work size is not an admission quota. Observed:not-run.
- **Test impact**: add named assertions/cases; update helpers only as required; retain valid-source and negative controls; delete none.

Exact source inputs, expected results and connected controls:

```json
[
  {
    "name": "normalization_cache_misses_never_become_schema_admission_quota",
    "input": "8193 unique nested property schemas, each valid; check cold and repeated normalization; replace final property value with invalid type.",
    "expected": "Both valid runs equal full normalized object; invalid final property rejects in both runs; original schema unchanged.",
    "controls": [
      "same outcome at 1,4096,4097,8193 entries; no public cache-tuning API introduced"
    ]
  }
]
```

For new test-only cases, first execute on the committed baseline. A failure does not authorize silently changing production; preserve it and give the owning source defect a distinct behavioral batch.

### Batch B39: schema cancel conformance

**Closure predicate**: Cancellation after the actual blocking schema worker enters suppresses authorization, execution and late publication when that worker returns.
**Dependencies**: []
**Verification union**: python scripts/opi-cargo-cache.py run --target <inspected-persistent-target> -- test -p opi-agent --lib cancellation_after_schema_worker_entry_suppresses_late_stages --offline --locked -- --nocapture; run every created or modified target/filter and affected-target clippy; final workspace/doc/whitespace union applies.

#### Fix B39.1: schema cancel conformance

- **Finding source(s)**: phase20-codex-gpt61-b3f1810-20261103t142820z / 4cec97af8ef6bb773c630ee251728f31212e12fdc1bf9631f10b2dc3084bc0f6 / P20-AUD-018
- **Decision**: fix:schema-cancel-conformance
- **Verification status**: Partially confirmed
- **File(s)**: `crates/opi-agent/src/agent_loop.rs`
- **Change kind**: test-only
- **Change**: Add the exact finite assertion matrix below at its current owning seam; production behavior and dependencies are unchanged by this test-only batch.
- **Closure predicate**: Cancellation after the actual blocking schema worker enters suppresses authorization, execution and late publication when that worker returns.
- **Red-before**: Not applicable to production behavior: this batch adds the finite source-defined conformance test only. Preserve any actual baseline failure and stop for its separately bound behavioral repair; no conditional production edit is admitted.
- **Green-after**: python scripts/opi-cargo-cache.py run --target <inspected-persistent-target> -- test -p opi-agent --lib cancellation_after_schema_worker_entry_suppresses_late_stages --offline --locked -- --nocapture -> Cancellation after the actual blocking schema worker enters suppresses authorization, execution and late publication when that worker returns. Observed:not-run.
- **Test impact**: add named assertions/cases; update helpers only as required; retain valid-source and negative controls; delete none.

Exact source inputs, expected results and connected controls:

```json
[
  {
    "name": "cancellation_after_schema_worker_entry_suppresses_late_stages",
    "input": "Private cfg(test) enter/release gate inside the actual existing spawn_blocking schema worker, tested from agent_loop unit module through a real Agent request and counted authorizer/tool. Cancel after worker entered, then release worker. Keep gate off in production and avoid a public test feature.",
    "expected": "Worker may finish; zero authorizer calls, zero tool executions and no later success/ToolEnd publication; terminal cancellation is stable.",
    "controls": [
      "same gate without cancellation authorizes and executes exactly once",
      "do not claim Future drop kills synchronous regex; observer cannot substitute a preparation/candidate gate"
    ]
  }
]
```

For new test-only cases, first execute on the committed baseline. A failure does not authorize silently changing production; preserve it and give the owning source defect a distinct behavioral batch.

### Batch B40: http proxy negatives conformance

**Closure predicate**: Every configured proxy failure is terminal before target HTTP and never causes direct fallback; success preserves outer-proxy TLS then target TLS order.
**Dependencies**: []
**Verification union**: python scripts/opi-cargo-cache.py run --target <inspected-persistent-target> -- test -p opi-ai --test http_acceptance https_proxy_failure_never_contacts_direct_target --offline --locked -- --nocapture; python scripts/opi-cargo-cache.py run --target <inspected-persistent-target> -- test -p opi-ai --test http_acceptance https_proxy_outer_tls_connect_tunnel_inner_tls --offline --locked -- --nocapture; run every created or modified target/filter and affected-target clippy; final workspace/doc/whitespace union applies.

#### Fix B40.1: http proxy negatives conformance

- **Finding source(s)**: phase20-codex-gpt61-b3f1810-20261103t142820z / 4cec97af8ef6bb773c630ee251728f31212e12fdc1bf9631f10b2dc3084bc0f6 / P20-AUD-018
- **Decision**: fix:http-proxy-negatives-conformance
- **Verification status**: Partially confirmed
- **File(s)**: `crates/opi-ai/tests/http_acceptance.rs`
- **Change kind**: test-only
- **Change**: Add the exact finite assertion matrix below at its current owning seam; production behavior and dependencies are unchanged by this test-only batch.
- **Closure predicate**: Every configured proxy failure is terminal before target HTTP and never causes direct fallback; success preserves outer-proxy TLS then target TLS order.
- **Red-before**: Not applicable to production behavior: this batch adds the finite source-defined conformance test only. Preserve any actual baseline failure and stop for its separately bound behavioral repair; no conditional production edit is admitted.
- **Green-after**: python scripts/opi-cargo-cache.py run --target <inspected-persistent-target> -- test -p opi-ai --test http_acceptance https_proxy_failure_never_contacts_direct_target --offline --locked -- --nocapture; python scripts/opi-cargo-cache.py run --target <inspected-persistent-target> -- test -p opi-ai --test http_acceptance https_proxy_outer_tls_connect_tunnel_inner_tls --offline --locked -- --nocapture -> Every configured proxy failure is terminal before target HTTP and never causes direct fallback; success preserves outer-proxy TLS then target TLS order. Observed:not-run.
- **Test impact**: add named assertions/cases; update helpers only as required; retain valid-source and negative controls; delete none.

Exact source inputs, expected results and connected controls:

```json
[
  {
    "name": "https_proxy_failure_never_contacts_direct_target",
    "input": "Local HTTPS proxy cases: untrusted outer certificate, CONNECT407, truncated CONNECT head, EOF; separate counted target listener.",
    "expected": "Typed Network/TLS acquisition cause, zero target connections and zero business requests; one proxy attempt only.",
    "controls": [
      "same target/proxy with trusted outer certificate+CONNECT200 reaches one target request",
      "count proxy source exchanges separately from business sends"
    ]
  }
]
```

For new test-only cases, first execute on the committed baseline. A failure does not authorize silently changing production; preserve it and give the owning source defect a distinct behavioral batch.

### Batch B41: http terminal conformance

**Closure predicate**: Accepted HTTP/SSE body failure preserves known partial content/usage, stable terminal error and sealed retry admission.
**Dependencies**: ["B05"]
**Verification union**: python scripts/opi-cargo-cache.py run --target <inspected-persistent-target> -- test -p opi-ai --test http_acceptance accepted_http_failure_preserves_partial_usage_and_seals_retry --offline --locked -- --nocapture; run every created or modified target/filter and affected-target clippy; final workspace/doc/whitespace union applies.

#### Fix B41.1: http terminal conformance

- **Finding source(s)**: phase20-codex-gpt61-b3f1810-20261103t142820z / 4cec97af8ef6bb773c630ee251728f31212e12fdc1bf9631f10b2dc3084bc0f6 / P20-AUD-018
- **Decision**: fix:http-terminal-conformance
- **Verification status**: Partially confirmed
- **File(s)**: `crates/opi-ai/tests/http_acceptance.rs`
- **Change kind**: test-only
- **Change**: Add the exact finite assertion matrix below at its current owning seam; production behavior and dependencies are unchanged by this test-only batch.
- **Closure predicate**: Accepted HTTP/SSE body failure preserves known partial content/usage, stable terminal error and sealed retry admission.
- **Red-before**: Not applicable to production behavior: this batch adds the finite source-defined conformance test only. Preserve any actual baseline failure and stop for its separately bound behavioral repair; no conditional production edit is admitted.
- **Green-after**: python scripts/opi-cargo-cache.py run --target <inspected-persistent-target> -- test -p opi-ai --test http_acceptance accepted_http_failure_preserves_partial_usage_and_seals_retry --offline --locked -- --nocapture -> Accepted HTTP/SSE body failure preserves known partial content/usage, stable terminal error and sealed retry admission. Observed:not-run.
- **Test impact**: add named assertions/cases; update helpers only as required; retain valid-source and negative controls; delete none.

Exact source inputs, expected results and connected controls:

```json
[
  {
    "name": "accepted_http_failure_preserves_partial_usage_and_seals_retry",
    "input": "Actual Chat adapter local chunked response with Start, text delta, usage input3/output2, then malformed/truncated chunk; enumerate after failure twice.",
    "expected": "Typed body-phase failure; exact partial text and usage3/2 retained; exactly one terminal error; subsequent polls EOF; retry admission sealed and exactly one generation send.",
    "controls": [
      "same prefix+lawful terminal tail completes successfully",
      "pre-head EOF has acquisition phase with no fabricated usage"
    ]
  }
]
```

For new test-only cases, first execute on the committed baseline. A failure does not authorize silently changing production; preserve it and give the owning source defect a distinct behavioral batch.

### Batch B42: http framing conformance

**Closure predicate**: Bytewise/coalesced HTTP framing follows the selected ordinary backend profile without corrupting callbacks, EOF or pool reuse.
**Dependencies**: []
**Verification union**: python scripts/opi-cargo-cache.py run --target <inspected-persistent-target> -- test -p opi-ai --test http_acceptance interim_and_obs_fold_responses_preserve_http_lifecycle --offline --locked -- --nocapture; python scripts/opi-cargo-cache.py run --target <inspected-persistent-target> -- test -p opi-ai --test connection_pooling http_client_reuses_h1_socket_after_fully_consumed_calls --offline --locked -- --nocapture; run every created or modified target/filter and affected-target clippy; final workspace/doc/whitespace union applies.

#### Fix B42.1: http framing conformance

- **Finding source(s)**: phase20-codex-gpt61-b3f1810-20261103t142820z / 4cec97af8ef6bb773c630ee251728f31212e12fdc1bf9631f10b2dc3084bc0f6 / P20-AUD-018
- **Decision**: fix:http-framing-conformance
- **Verification status**: Partially confirmed
- **File(s)**: `crates/opi-ai/tests/http_acceptance.rs`
- **Change kind**: test-only
- **Change**: Add the exact finite assertion matrix below at its current owning seam; production behavior and dependencies are unchanged by this test-only batch.
- **Closure predicate**: Bytewise/coalesced HTTP framing follows the selected ordinary backend profile without corrupting callbacks, EOF or pool reuse.
- **Red-before**: Not applicable to production behavior: this batch adds the finite source-defined conformance test only. Preserve any actual baseline failure and stop for its separately bound behavioral repair; no conditional production edit is admitted.
- **Green-after**: python scripts/opi-cargo-cache.py run --target <inspected-persistent-target> -- test -p opi-ai --test http_acceptance interim_and_obs_fold_responses_preserve_http_lifecycle --offline --locked -- --nocapture; python scripts/opi-cargo-cache.py run --target <inspected-persistent-target> -- test -p opi-ai --test connection_pooling http_client_reuses_h1_socket_after_fully_consumed_calls --offline --locked -- --nocapture -> Bytewise/coalesced HTTP framing follows the selected ordinary backend profile without corrupting callbacks, EOF or pool reuse. Observed:not-run.
- **Test impact**: add named assertions/cases; update helpers only as required; retain valid-source and negative controls; delete none.

Exact source inputs, expected results and connected controls:

```json
[
  {
    "name": "interim_and_obs_fold_responses_preserve_http_lifecycle",
    "input": "100 Continue→200 and103 Early Hints→200 with valid SSE; obsolete folded response-header case. Run bytewise/coalesced.",
    "expected": "100/103 interim responses do not trigger final-header callback; exactly one200 callback and one terminal. The selected ordinary backend rejects obs-fold: a response field X-Test: one followed by CRLF+SP+two fails typed acquisition with zero accepted-response callback and zero body/partial publication, for both bytewise and coalesced schedules.",
    "controls": [
      "ordinary200 samebody succeeds",
      "no101 treated as final200",
      "second lawful request after failure has clean pool/body state"
    ]
  }
]
```

For new test-only cases, first execute on the committed baseline. A failure does not authorize silently changing production; preserve it and give the owning source defect a distinct behavioral batch.

### Batch B43: replay openrouter conformance

**Closure predicate**: OpenRouter route reaches an actual second HTTP request after producer capture and persist/reload, retaining valid same-route opaque content and stripping foreign content.
**Dependencies**: ["B25"]
**Verification union**: python scripts/opi-cargo-cache.py run --target <inspected-persistent-target> -- test -p opi-ai --test chat_capture_replay openrouter_route_capture_reload_replays_only_bound_opaque_blocks --offline --locked -- --nocapture; run every created or modified target/filter and affected-target clippy; final workspace/doc/whitespace union applies.

#### Fix B43.1: replay openrouter conformance

- **Finding source(s)**: phase20-codex-gpt61-b3f1810-20261103t142820z / 4cec97af8ef6bb773c630ee251728f31212e12fdc1bf9631f10b2dc3084bc0f6 / P20-AUD-018
- **Decision**: fix:replay-openrouter-conformance
- **Verification status**: Partially confirmed
- **File(s)**: `crates/opi-ai/tests/chat_capture_replay.rs`
- **Change kind**: test-only
- **Change**: Add the exact finite assertion matrix below at its current owning seam; production behavior and dependencies are unchanged by this test-only batch.
- **Closure predicate**: OpenRouter route reaches an actual second HTTP request after producer capture and persist/reload, retaining valid same-route opaque content and stripping foreign content.
- **Red-before**: Not applicable to production behavior: this batch adds the finite source-defined conformance test only. Preserve any actual baseline failure and stop for its separately bound behavioral repair; no conditional production edit is admitted.
- **Green-after**: python scripts/opi-cargo-cache.py run --target <inspected-persistent-target> -- test -p opi-ai --test chat_capture_replay openrouter_route_capture_reload_replays_only_bound_opaque_blocks --offline --locked -- --nocapture -> OpenRouter route reaches an actual second HTTP request after producer capture and persist/reload, retaining valid same-route opaque content and stripping foreign content. Observed:not-run.
- **Test impact**: add named assertions/cases; update helpers only as required; retain valid-source and negative controls; delete none.

Exact source inputs, expected results and connected controls:

```json
[
  {
    "name": "openrouter_route_capture_reload_replays_only_bound_opaque_blocks",
    "input": "OpenRouter actual route using the shared Chat adapter; ordered reasoning_details blocks, visible answer, signed/opaque empty block where supported; persist/reload captured AssistantMessage.",
    "expected": "Second same-identity request preserves exact supported opaque bytes/order; changed model/API/provider strips ineligible opaque/thinking and retains answer; original captured message byte equality.",
    "controls": [
      "assert first producer actually emitted metadata; use ordinary public Models route, not serializer-only construction",
      "existing unknown-metadata control remains typed/projection policy"
    ]
  }
]
```

For new test-only cases, first execute on the committed baseline. A failure does not authorize silently changing production; preserve it and give the owning source defect a distinct behavioral batch.

### Batch B44: transcript effort conformance

**Closure predicate**: Native effort markers and returned stamp reflect the selected prepared policy under the finite source cases; payload edits cannot retroactively rewrite the stamp.
**Dependencies**: ["B26"]
**Verification union**: python scripts/opi-cargo-cache.py run --target <inspected-persistent-target> -- test -p opi-ai --test openai_responses_fixtures responses_effort_history_marker_boundaries_use_prepared_stamp --offline --locked -- --nocapture; python scripts/opi-cargo-cache.py run --target <inspected-persistent-target> -- test -p opi-ai --test anthropic_fixtures anthropic_off_marker_output_config_binding_and_stamp --offline --locked -- --nocapture; run every created or modified target/filter and affected-target clippy; final workspace/doc/whitespace union applies.

#### Fix B44.1: transcript effort conformance

- **Finding source(s)**: phase20-codex-gpt61-b3f1810-20261103t142820z / 4cec97af8ef6bb773c630ee251728f31212e12fdc1bf9631f10b2dc3084bc0f6 / P20-AUD-018
- **Decision**: fix:transcript-effort-conformance
- **Verification status**: Partially confirmed
- **File(s)**: `crates/opi-ai/tests/openai_responses_fixtures.rs`, `crates/opi-ai/tests/anthropic_fixtures.rs`
- **Change kind**: test-only
- **Change**: Add the exact finite assertion matrix below at its current owning seam; production behavior and dependencies are unchanged by this test-only batch.
- **Closure predicate**: Native effort markers and returned stamp reflect the selected prepared policy under the finite source cases; payload edits cannot retroactively rewrite the stamp.
- **Red-before**: Not applicable to production behavior: this batch adds the finite source-defined conformance test only. Preserve any actual baseline failure and stop for its separately bound behavioral repair; no conditional production edit is admitted.
- **Green-after**: python scripts/opi-cargo-cache.py run --target <inspected-persistent-target> -- test -p opi-ai --test openai_responses_fixtures responses_effort_history_marker_boundaries_use_prepared_stamp --offline --locked -- --nocapture; python scripts/opi-cargo-cache.py run --target <inspected-persistent-target> -- test -p opi-ai --test anthropic_fixtures anthropic_off_marker_output_config_binding_and_stamp --offline --locked -- --nocapture -> Native effort markers and returned stamp reflect the selected prepared policy under the finite source cases; payload edits cannot retroactively rewrite the stamp. Observed:not-run.
- **Test impact**: add named assertions/cases; update helpers only as required; retain valid-source and negative controls; delete none.

Exact source inputs, expected results and connected controls:

```json
[
  {
    "name": "responses_effort_history_marker_boundaries_use_prepared_stamp",
    "input": "Same-identity history with two equal low-effort nonempty replies, empty reply, final tool output; current high native option; onPayload overrides visible effort.",
    "expected": "Repeated required markers retained; empty reply no marker; tool-output ordering preserved; final high marker; top-level medium source rule; returned provider_thinking_level remains prepared high.",
    "controls": [
      "foreign history produces no inherited marker",
      "sampling override does not rewrite stored generation stamp"
    ]
  },
  {
    "name": "anthropic_off_marker_output_config_binding_and_stamp",
    "input": "Actual native Anthropic off option with prior signed same-identity thinking and adaptive/off compatible model.",
    "expected": "For supported thinking-off: thinking.type=disabled; no effort markers, no output_config, no block_binding and returned provider_thinking_level=None. Canonical history remains unchanged. Registered acceptance-matrix V16 and spec-draft:251 select these exact absences.",
    "controls": [
      "Enabled adaptive high profile emits thinking.type=adaptive and output_config.effort=high, proving the fixture reached the selected native option path; canonical source unchanged."
    ]
  }
]
```

For new test-only cases, first execute on the committed baseline. A failure does not authorize silently changing production; preserve it and give the owning source defect a distinct behavioral batch.

### Batch B45: strict variant markers conformance

**Closure predicate**: Exactly the three source-qualified400 invalid_request_error markers permit one pre-Start variant; status/type/cancellation/callback/Start controls never branch.
**Dependencies**: ["B27"]
**Verification union**: python scripts/opi-cargo-cache.py run --target <inspected-persistent-target> -- test -p opi-ai --test anthropic_fixtures strict_variant_all_source_markers_and_exclusions_reach_actual_wire --offline --locked -- --nocapture; python scripts/opi-cargo-cache.py run --target <inspected-persistent-target> -- test -p opi-ai --test anthropic_fixtures ordinary_retry_budget_is_per_variant_with_total_bound --offline --locked -- --nocapture; run every created or modified target/filter and affected-target clippy; final workspace/doc/whitespace union applies.

#### Fix B45.1: strict variant markers conformance

- **Finding source(s)**: phase20-codex-gpt61-b3f1810-20261103t142820z / 4cec97af8ef6bb773c630ee251728f31212e12fdc1bf9631f10b2dc3084bc0f6 / P20-AUD-018
- **Decision**: fix:strict-variant-markers-conformance
- **Verification status**: Partially confirmed
- **File(s)**: `crates/opi-ai/tests/anthropic_fixtures.rs`
- **Change kind**: test-only
- **Change**: Add the exact finite assertion matrix below at its current owning seam; production behavior and dependencies are unchanged by this test-only batch.
- **Closure predicate**: Exactly the three source-qualified400 invalid_request_error markers permit one pre-Start variant; status/type/cancellation/callback/Start controls never branch.
- **Red-before**: Not applicable to production behavior: this batch adds the finite source-defined conformance test only. Preserve any actual baseline failure and stop for its separately bound behavioral repair; no conditional production edit is admitted.
- **Green-after**: python scripts/opi-cargo-cache.py run --target <inspected-persistent-target> -- test -p opi-ai --test anthropic_fixtures strict_variant_all_source_markers_and_exclusions_reach_actual_wire --offline --locked -- --nocapture; python scripts/opi-cargo-cache.py run --target <inspected-persistent-target> -- test -p opi-ai --test anthropic_fixtures ordinary_retry_budget_is_per_variant_with_total_bound --offline --locked -- --nocapture -> Exactly the three source-qualified400 invalid_request_error markers permit one pre-Start variant; status/type/cancellation/callback/Start controls never branch. Observed:not-run.
- **Test impact**: add named assertions/cases; update helpers only as required; retain valid-source and negative controls; delete none.

Exact source inputs, expected results and connected controls:

```json
[
  {
    "name": "strict_variant_all_source_markers_and_exclusions_reach_actual_wire",
    "input": [
      "The compiled grammar is too large",
      "Too many strict tools (65)",
      "Schemas contains too many parameters with union types (fixture)"
    ],
    "expected": "Each400 invalid_request_error dispatches exactly original+one downgraded variant at N0, removes only Prefer, keeps Require, freezes auth/model, rebuilds onPayload and validates payload, emits one downgrade diagnostic.",
    "controls": [
      "same marker at401/429 does not qualify",
      "different error type at400 does not qualify",
      "cancel before second send, callback error, or post-Start failure yields no second variant",
      "existing N1 ordinary retry case asserts bound2*(1+N) and exact per-variant budget"
    ]
  }
]
```

For new test-only cases, first execute on the committed baseline. A failure does not authorize silently changing production; preserve it and give the owning source defect a distinct behavioral batch.

### Batch B46: verify p20-v-01 finite source cases

**Closure predicate**: The finite selection_frozen_four_entries source cases produce exactly the registered results and side-effect counts below; typed invalid-input controls fail before the stated later boundary.
**Dependencies**: ["B08", "B22", "B23", "B24", "B25", "B26", "B27"]
**Verification union**: python scripts/opi-cargo-cache.py run --target <persistent-target> -- test -p opi-ai --test model_wire_metadata --offline --locked -- --test-threads=1; python scripts/opi-cargo-cache.py run --target <persistent-target> -- test -p opi-ai --test provider_collection --offline --locked -- --test-threads=1; run every created or modified target/filter and affected-target clippy; final workspace/doc/whitespace union applies.

#### Fix B46.1: verify p20-v-01 finite source cases

- **Finding source(s)**: phase20-codex-gpt61-b3f1810-20261103t142820z / 4cec97af8ef6bb773c630ee251728f31212e12fdc1bf9631f10b2dc3084bc0f6 / P20-AUD-018
- **Decision**: fix:verify-p20-v-01-finite-source-cases
- **Verification status**: Partially confirmed
- **File(s)**: `crates/opi-ai/tests/provider_collection.rs`, `crates/opi-ai/tests/model_wire_metadata.rs`
- **Change kind**: test-only
- **Change**: Add the exact finite assertion matrix below at its current owning seam; production behavior and dependencies are unchanged by this test-only batch.
- **Closure predicate**: The finite selection_frozen_four_entries source cases produce exactly the registered results and side-effect counts below; typed invalid-input controls fail before the stated later boundary.
- **Red-before**: Not applicable to production behavior: this batch adds the finite source-defined conformance test only. Preserve any actual baseline failure and stop for its separately bound behavioral repair; no conditional production edit is admitted.
- **Green-after**: python scripts/opi-cargo-cache.py run --target <persistent-target> -- test -p opi-ai --test model_wire_metadata --offline --locked -- --test-threads=1; python scripts/opi-cargo-cache.py run --target <persistent-target> -- test -p opi-ai --test provider_collection --offline --locked -- --test-threads=1 -> The finite selection_frozen_four_entries source cases produce exactly the registered results and side-effect counts below; typed invalid-input controls fail before the stated later boundary. Observed:not-run.
- **Test impact**: add named assertions/cases; update helpers only as required; retain valid-source and negative controls; delete none.

Assigned current criterion: `P20-V-01`; exact source `docs/research/pi-ai-opi-ai-alignment/acceptance-matrix.md` SHA256 `c831586339910f9496a64e6b3c9390a0b7a2a366d37230222874a5f399099a5d`, citation `V-01, line 56`.

Current assertion evidence and limits: provider_collection4046–4096 old prepared binding survives catalog removal;4957–5045 complete owned id/baseURL/headers;5976–6097 one source read/payload and frozen Azure endpoint/deployment across retries.

Finite inputs and oracle: selection_frozen_four_entries: old call keeps old full record after replacement; new call uses new; endpoint_after_auth_defaults: absent compat uses final endpoint once, explicitfalse wins; invalid preauth/authdependent input fails beforeauth/HTTP; effort_requested_effective: higherthenlower simple clamp, nativeunsupported reject.

Authority: No product-meaning decision; use existing public seam and registered predicate.

This batch owns only the named additional test portion. Referenced C017/package/native proof is a separate conjunction/barrier; it does not create a backward dependency on the final B17 runner. Existing known behavioral defects keep their named production batches. Unexpected baseline red is preserved and replanned, never silently repaired here.

### Batch B47: verify p20-v-26 finite source cases

**Closure predicate**: The finite typed_nonchat_effects source cases produce exactly the registered results and side-effect counts below; typed invalid-input controls fail before the stated later boundary.
**Dependencies**: ["B10", "B11", "B19"]
**Verification union**: python scripts/opi-cargo-cache.py run --target <persistent-target> -- test -p opi-ai --test classifier_contract --offline --locked -- --test-threads=1; python scripts/opi-cargo-cache.py run --target <persistent-target> -- test -p opi-ai --test external_sdk --offline --locked -- --test-threads=1; run every created or modified target/filter and affected-target clippy; final workspace/doc/whitespace union applies.

#### Fix B47.1: verify p20-v-26 finite source cases

- **Finding source(s)**: phase20-codex-gpt61-b3f1810-20261103t142820z / 4cec97af8ef6bb773c630ee251728f31212e12fdc1bf9631f10b2dc3084bc0f6 / P20-AUD-018
- **Decision**: fix:verify-p20-v-26-finite-source-cases
- **Verification status**: Partially confirmed
- **File(s)**: `crates/opi-ai/tests/classifier_contract.rs`, `crates/opi-ai/tests/external_sdk.rs`
- **Change kind**: test-only
- **Change**: Add the exact finite assertion matrix below at its current owning seam; production behavior and dependencies are unchanged by this test-only batch.
- **Closure predicate**: The finite typed_nonchat_effects source cases produce exactly the registered results and side-effect counts below; typed invalid-input controls fail before the stated later boundary.
- **Red-before**: Not applicable to production behavior: this batch adds the finite source-defined conformance test only. Preserve any actual baseline failure and stop for its separately bound behavioral repair; no conditional production edit is admitted.
- **Green-after**: python scripts/opi-cargo-cache.py run --target <persistent-target> -- test -p opi-ai --test classifier_contract --offline --locked -- --test-threads=1; python scripts/opi-cargo-cache.py run --target <persistent-target> -- test -p opi-ai --test external_sdk --offline --locked -- --test-threads=1 -> The finite typed_nonchat_effects source cases produce exactly the registered results and side-effect counts below; typed invalid-input controls fail before the stated later boundary. Observed:not-run.
- **Test impact**: add named assertions/cases; update helpers only as required; retain valid-source and negative controls; delete none.

Assigned current criterion: `P20-V-26`; exact source `docs/research/pi-ai-opi-ai-alignment/acceptance-matrix.md` SHA256 `c831586339910f9496a64e6b3c9390a0b7a2a366d37230222874a5f399099a5d`, citation `V-26, line 81`.

Current assertion evidence and limits: classifier_contract3390–3573 actual4protocol results and distinct noul/predicate payload+headers; external_sdk630–899 fourclassifier/image dispatch but Decisions case is504failure.

Finite inputs and oracle: typed_nonchat_effects: all4success+typedfailure and image success+receivedfailure via localpeers, exactcallbacks/payload/accounting; no files/tool execution; package metadata shows onlyopi-ai Opi runtime.

Authority: No product-meaning decision; use existing public seam and registered predicate.

This batch owns only the named additional test portion. Referenced C017/package/native proof is a separate conjunction/barrier; it does not create a backward dependency on the final B17 runner. Existing known behavioral defects keep their named production batches. Unexpected baseline red is preserved and replanned, never silently repaired here.

Exact assembled-test ownership: changes to external_sdk.rs are additional assertions/local helpers inside the already declared classifier_and_image_apis_answer_typed_through_one_instance. Add no new #[test]/#[tokio::test] item to that source and do not rename or expand B17's exact nineteen-case allowlist. Separately named cases may be added only to the other explicitly listed owning binary; run those binaries separately.

### Batch B50: verify p20-sdk-core-028 finite source cases

**Closure predicate**: The finite store_unknown_model_type source cases produce exactly the registered results and side-effect counts below; typed invalid-input controls fail before the stated later boundary.
**Dependencies**: []
**Verification union**: python scripts/opi-cargo-cache.py run --target <persistent-target> -- test -p opi-ai --test catalog_operations --offline --locked -- --test-threads=1; run every created or modified target/filter and affected-target clippy; final workspace/doc/whitespace union applies.

#### Fix B50.1: verify p20-sdk-core-028 finite source cases

- **Finding source(s)**: phase20-codex-gpt61-b3f1810-20261103t142820z / 4cec97af8ef6bb773c630ee251728f31212e12fdc1bf9631f10b2dc3084bc0f6 / P20-AUD-018
- **Decision**: fix:verify-p20-sdk-core-028-finite-source-cases
- **Verification status**: Partially confirmed
- **File(s)**: `crates/opi-ai/tests/catalog_operations.rs`
- **Change kind**: test-only
- **Change**: Add the exact finite assertion matrix below at its current owning seam; production behavior and dependencies are unchanged by this test-only batch.
- **Closure predicate**: The finite store_unknown_model_type source cases produce exactly the registered results and side-effect counts below; typed invalid-input controls fail before the stated later boundary.
- **Red-before**: Not applicable to production behavior: this batch adds the finite source-defined conformance test only. Preserve any actual baseline failure and stop for its separately bound behavioral repair; no conditional production edit is admitted.
- **Green-after**: python scripts/opi-cargo-cache.py run --target <persistent-target> -- test -p opi-ai --test catalog_operations --offline --locked -- --test-threads=1 -> The finite store_unknown_model_type source cases produce exactly the registered results and side-effect counts below; typed invalid-input controls fail before the stated later boundary. Observed:not-run.
- **Test impact**: add named assertions/cases; update helpers only as required; retain valid-source and negative controls; delete none.

Assigned current criterion: `P20-SDK-CORE-028`; exact source `docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md` SHA256 `9e3d6ef690c3a794bc9c47ddc2dce04840f98d8881649be1ab2e8db0df287fd5`, citation `L159`.

Current assertion evidence and limits: catalog_operations190–196 unknownoperation publicdecode Err; store Corrupt typed on malformedentries.

Finite inputs and oracle: store_unknown_model_type: mutatevalidmixedsavedmodeltype; reopen ModelsStoreError::Corrupt (unique selected outcome), zerochat reinterpretation/auth/send;3knownoperation control.

Authority: No product-meaning decision; use existing public seam and registered predicate.

This batch owns only the named additional test portion. Referenced C017/package/native proof is a separate conjunction/barrier; it does not create a backward dependency on the final B17 runner. Existing known behavioral defects keep their named production batches. Unexpected baseline red is preserved and replanned, never silently repaired here.

Unique invalid-type oracle is current decode_store_document at catalog/wire.rs1473–1488: decode_provider_entry errors map directly to ModelsStoreError::Corrupt. No filtering/report branch is selected or introduced.

### Batch B51: verify p20-sdk-core-045 finite source cases

**Closure predicate**: The finite source_validation_caps source cases produce exactly the registered results and side-effect counts below; typed invalid-input controls fail before the stated later boundary.
**Dependencies**: []
**Verification union**: python scripts/opi-cargo-cache.py run --target <persistent-target> -- test -p opi-ai --test classifier_contract --offline --locked -- --test-threads=1; python scripts/opi-cargo-cache.py run --target <persistent-target> -- test -p opi-ai --test external_sdk --offline --locked -- --test-threads=1; python scripts/opi-cargo-cache.py run --target <persistent-target> -- test -p opi-ai --test model_wire_metadata --offline --locked -- --test-threads=1; run every created or modified target/filter and affected-target clippy; final workspace/doc/whitespace union applies.

#### Fix B51.1: verify p20-sdk-core-045 finite source cases

- **Finding source(s)**: phase20-codex-gpt61-b3f1810-20261103t142820z / 4cec97af8ef6bb773c630ee251728f31212e12fdc1bf9631f10b2dc3084bc0f6 / P20-AUD-018
- **Decision**: fix:verify-p20-sdk-core-045-finite-source-cases
- **Verification status**: Partially confirmed
- **File(s)**: `crates/opi-ai/tests/classifier_contract.rs`, `crates/opi-ai/tests/external_sdk.rs`, `crates/opi-ai/tests/model_wire_metadata.rs`
- **Change kind**: test-only
- **Change**: Add the exact finite assertion matrix below at its current owning seam; production behavior and dependencies are unchanged by this test-only batch.
- **Closure predicate**: The finite source_validation_caps source cases produce exactly the registered results and side-effect counts below; typed invalid-input controls fail before the stated later boundary.
- **Red-before**: Not applicable to production behavior: this batch adds the finite source-defined conformance test only. Preserve any actual baseline failure and stop for its separately bound behavioral repair; no conditional production edit is admitted.
- **Green-after**: python scripts/opi-cargo-cache.py run --target <persistent-target> -- test -p opi-ai --test classifier_contract --offline --locked -- --test-threads=1; python scripts/opi-cargo-cache.py run --target <persistent-target> -- test -p opi-ai --test external_sdk --offline --locked -- --test-threads=1; python scripts/opi-cargo-cache.py run --target <persistent-target> -- test -p opi-ai --test model_wire_metadata --offline --locked -- --test-threads=1 -> The finite source_validation_caps source cases produce exactly the registered results and side-effect counts below; typed invalid-input controls fail before the stated later boundary. Observed:not-run.
- **Test impact**: add named assertions/cases; update helpers only as required; retain valid-source and negative controls; delete none.

Assigned current criterion: `P20-SDK-CORE-045`; exact source `docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-rust-differences.md` SHA256 `411d2f6d262b1afc22e84c73213dffd0eb890ed70e210dfa497d2a5d021cd31d`, citation `L99`.

Current assertion evidence and limits: LinkedSDKL99 is stale (providerrow); actualvalidation in124–131/163/165/190. classifier562/3074 nonfiniteanswers; model_info1153–1164 finitepricing; external_sdk1477 lawful>64messages.

Finite inputs and oracle: source_validation_caps:65messages/65questions/>64KiB lawfulmetadata/deepJSON beyondoldcap accepted; missingrequired/wrongtype/nonfinite failtyped; sourceDecisions129images stillreject; u64checkedarithmetic nooverflow.

Authority: No product-meaning decision; use existing public seam and registered predicate.

This batch owns only the named additional test portion. Referenced C017/package/native proof is a separate conjunction/barrier; it does not create a backward dependency on the final B17 runner. Existing known behavioral defects keep their named production batches. Unexpected baseline red is preserved and replanned, never silently repaired here.

Exact assembled-test ownership: changes to external_sdk.rs are additional assertions/local helpers inside the already declared lawful_collection_over_sixty_four_messages_is_accepted (for its message boundary); use existing classifier_contract/model_wire_metadata targets for the other finite boundaries. Add no new #[test]/#[tokio::test] item to that source and do not rename or expand B17's exact nineteen-case allowlist. Separately named cases may be added only to the other explicitly listed owning binary; run those binaries separately.

### Batch B52: verify p20-sdk-profile-030 finite source cases

**Closure predicate**: The finite cloudflare_binding_explicit source cases produce exactly the registered results and side-effect counts below; typed invalid-input controls fail before the stated later boundary.
**Dependencies**: []
**Verification union**: python scripts/opi-cargo-cache.py run --target <persistent-target> -- test -p opi-ai --test classifier_contract --offline --locked -- --test-threads=1; run every created or modified target/filter and affected-target clippy; final workspace/doc/whitespace union applies.

#### Fix B52.1: verify p20-sdk-profile-030 finite source cases

- **Finding source(s)**: phase20-codex-gpt61-b3f1810-20261103t142820z / 4cec97af8ef6bb773c630ee251728f31212e12fdc1bf9631f10b2dc3084bc0f6 / P20-AUD-018
- **Decision**: fix:verify-p20-sdk-profile-030-finite-source-cases
- **Verification status**: Partially confirmed
- **File(s)**: `crates/opi-ai/tests/classifier_contract.rs`
- **Change kind**: test-only
- **Change**: Add the exact finite assertion matrix below at its current owning seam; production behavior and dependencies are unchanged by this test-only batch.
- **Closure predicate**: The finite cloudflare_binding_explicit source cases produce exactly the registered results and side-effect counts below; typed invalid-input controls fail before the stated later boundary.
- **Red-before**: Not applicable to production behavior: this batch adds the finite source-defined conformance test only. Preserve any actual baseline failure and stop for its separately bound behavioral repair; no conditional production edit is admitted.
- **Green-after**: python scripts/opi-cargo-cache.py run --target <persistent-target> -- test -p opi-ai --test classifier_contract --offline --locked -- --test-threads=1 -> The finite cloudflare_binding_explicit source cases produce exactly the registered results and side-effect counts below; typed invalid-input controls fail before the stated later boundary. Observed:not-run.
- **Test impact**: add named assertions/cases; update helpers only as required; retain valid-source and negative controls; delete none.

Assigned current criterion: `P20-SDK-PROFILE-030`; exact source `docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md` SHA256 `9e3d6ef690c3a794bc9c47ddc2dce04840f98d8881649be1ab2e8db0df287fd5`, citation `L113`.

Current assertion evidence and limits: api/cloudflare73 explicit HttpClient+complete record; ClassifierApi is current hostadapter seam; SDKcontracts113 permits platformadapter without JSobjectABI.

Finite inputs and oracle: cloudflare_binding_explicit: caller-owned ClassifierApi with injectedhandle; onecall exactenvelope/result/failure; zeroWorkerenv/URLconversion; RESTconstructor localpeer control. No new Worker transporttrait.

Authority: No product-meaning decision; use existing public seam and registered predicate.

This batch owns only the named additional test portion. Referenced C017/package/native proof is a separate conjunction/barrier; it does not create a backward dependency on the final B17 runner. Existing known behavioral defects keep their named production batches. Unexpected baseline red is preserved and replanned, never silently repaired here.

### Batch B53: verify p20-sdk-profile-033 finite source cases

**Closure predicate**: The finite placeholder_lifecycle source cases produce exactly the registered results and side-effect counts below; typed invalid-input controls fail before the stated later boundary.
**Dependencies**: []
**Verification union**: python scripts/opi-cargo-cache.py run --target <persistent-target> -- test -p opi-ai --test bedrock_fixtures --offline --locked -- --test-threads=1; run every created or modified target/filter and affected-target clippy; final workspace/doc/whitespace union applies.

#### Fix B53.1: verify p20-sdk-profile-033 finite source cases

- **Finding source(s)**: phase20-codex-gpt61-b3f1810-20261103t142820z / 4cec97af8ef6bb773c630ee251728f31212e12fdc1bf9631f10b2dc3084bc0f6 / P20-AUD-018
- **Decision**: fix:verify-p20-sdk-profile-033-finite-source-cases
- **Verification status**: Partially confirmed
- **File(s)**: `crates/opi-ai/tests/bedrock_fixtures.rs`
- **Change kind**: test-only
- **Change**: Add the exact finite assertion matrix below at its current owning seam; production behavior and dependencies are unchanged by this test-only batch.
- **Closure predicate**: The finite placeholder_lifecycle source cases produce exactly the registered results and side-effect counts below; typed invalid-input controls fail before the stated later boundary.
- **Red-before**: Not applicable to production behavior: this batch adds the finite source-defined conformance test only. Preserve any actual baseline failure and stop for its separately bound behavioral repair; no conditional production edit is admitted.
- **Green-after**: python scripts/opi-cargo-cache.py run --target <persistent-target> -- test -p opi-ai --test bedrock_fixtures --offline --locked -- --test-threads=1 -> The finite placeholder_lifecycle source cases produce exactly the registered results and side-effect counts below; typed invalid-input controls fail before the stated later boundary. Observed:not-run.
- **Test impact**: add named assertions/cases; update helpers only as required; retain valid-source and negative controls; delete none.

Assigned current criterion: `P20-SDK-PROFILE-033`; exact source `docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md` SHA256 `9e3d6ef690c3a794bc9c47ddc2dce04840f98d8881649be1ab2e8db0df287fd5`, citation `L117`.

Current assertion evidence and limits: bedrock2574–2711 realH1/dummySigV4/region/canaryabsent/zeroenv/Done onModels path; no direct retry/cancel/drop coverage.

Finite inputs and oracle: placeholder_lifecycle:Models+direct x success/503retry/cancel-beforeheaders/drop-midbody; readcompletebody,H1,placeholderAuth,callbacks,sends,terminal/release; realcredentialcanarieszeroIO; standardAWSendpointzero-send negative.

Authority: No product-meaning decision; use existing public seam and registered predicate.

This batch owns only the named additional test portion. Referenced C017/package/native proof is a separate conjunction/barrier; it does not create a backward dependency on the final B17 runner. Existing known behavioral defects keep their named production batches. Unexpected baseline red is preserved and replanned, never silently repaired here.

### Batch B54: verify p20-sdk-auth-018 finite source cases

**Closure predicate**: The finite persistent_two_process_lock source cases produce exactly the registered results and side-effect counts below; typed invalid-input controls fail before the stated later boundary.
**Dependencies**: []
**Verification union**: python scripts/opi-cargo-cache.py run --target <persistent-target> -- test -p opi-coding-agent --test credential_store --offline --locked -- --test-threads=1; run every created or modified target/filter and affected-target clippy; final workspace/doc/whitespace union applies.

#### Fix B54.1: verify p20-sdk-auth-018 finite source cases

- **Finding source(s)**: phase20-codex-gpt61-b3f1810-20261103t142820z / 4cec97af8ef6bb773c630ee251728f31212e12fdc1bf9631f10b2dc3084bc0f6 / P20-AUD-018
- **Decision**: fix:verify-p20-sdk-auth-018-finite-source-cases
- **Verification status**: Partially confirmed
- **File(s)**: `crates/opi-coding-agent/tests/credential_store.rs`
- **Change kind**: test-only
- **Change**: Add the exact finite assertion matrix below at its current owning seam; production behavior and dependencies are unchanged by this test-only batch.
- **Closure predicate**: The finite persistent_two_process_lock source cases produce exactly the registered results and side-effect counts below; typed invalid-input controls fail before the stated later boundary.
- **Red-before**: Not applicable to production behavior: this batch adds the finite source-defined conformance test only. Preserve any actual baseline failure and stop for its separately bound behavioral repair; no conditional production edit is admitted.
- **Green-after**: python scripts/opi-cargo-cache.py run --target <persistent-target> -- test -p opi-coding-agent --test credential_store --offline --locked -- --test-threads=1 -> The finite persistent_two_process_lock source cases produce exactly the registered results and side-effect counts below; typed invalid-input controls fail before the stated later boundary. Observed:not-run.
- **Test impact**: add named assertions/cases; update helpers only as required; retain valid-source and negative controls; delete none.

Assigned current criterion: `P20-SDK-AUTH-018`; exact source `docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md` SHA256 `9e3d6ef690c3a794bc9c47ddc2dce04840f98d8881649be1ab2e8db0df287fd5`, citation `L153`.

Current assertion evidence and limits: credential291–309 persistentlock owned by injectedproductstore. productcredential_store1317–1360 uses2tasks/2stores+sharedfakebackend, not2processes.

Finite inputs and oracle: persistent_two_process_lock: A entersbackendsaveholdsIPCgate;Bsamepath cannotenteruntilArelease; distinctpathcontrol concurrent;timeouttyped/no mutation;reopen value. Testbackend injected,no realkeychain.

Authority: No product-meaning decision; use existing public seam and registered predicate.

This batch owns only the named additional test portion. Referenced C017/package/native proof is a separate conjunction/barrier; it does not create a backward dependency on the final B17 runner. Existing known behavioral defects keep their named production batches. Unexpected baseline red is preserved and replanned, never silently repaired here.

### Batch B55: verify p20-sdk-auth-026 finite source cases

**Closure predicate**: The finite receipt_retention source cases produce exactly the registered results and side-effect counts below; typed invalid-input controls fail before the stated later boundary.
**Dependencies**: []
**Verification union**: python scripts/opi-cargo-cache.py run --target <persistent-target> -- test -p opi-ai --test auth_contracts --offline --locked -- --test-threads=1; run every created or modified target/filter and affected-target clippy; final workspace/doc/whitespace union applies.

#### Fix B55.1: verify p20-sdk-auth-026 finite source cases

- **Finding source(s)**: phase20-codex-gpt61-b3f1810-20261103t142820z / 4cec97af8ef6bb773c630ee251728f31212e12fdc1bf9631f10b2dc3084bc0f6 / P20-AUD-018
- **Decision**: fix:verify-p20-sdk-auth-026-finite-source-cases
- **Verification status**: Partially confirmed
- **File(s)**: `crates/opi-ai/tests/auth_contracts.rs`
- **Change kind**: test-only
- **Change**: Add the exact finite assertion matrix below at its current owning seam; production behavior and dependencies are unchanged by this test-only batch.
- **Closure predicate**: The finite receipt_retention source cases produce exactly the registered results and side-effect counts below; typed invalid-input controls fail before the stated later boundary.
- **Red-before**: Not applicable to production behavior: this batch adds the finite source-defined conformance test only. Preserve any actual baseline failure and stop for its separately bound behavioral repair; no conditional production edit is admitted.
- **Green-after**: python scripts/opi-cargo-cache.py run --target <persistent-target> -- test -p opi-ai --test auth_contracts --offline --locked -- --test-threads=1 -> The finite receipt_retention source cases produce exactly the registered results and side-effect counts below; typed invalid-input controls fail before the stated later boundary. Observed:not-run.
- **Test impact**: add named assertions/cases; update helpers only as required; retain valid-source and negative controls; delete none.

Assigned current criterion: `P20-SDK-AUTH-026`; exact source `docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-rust-differences.md` SHA256 `411d2f6d262b1afc22e84c73213dffd0eb890ed70e210dfa497d2a5d021cd31d`, citation `L144`.

Current assertion evidence and limits: mutation94–97 unknownnotclearedotherkey;506–535 retainsallreceipts,no reclamation. auth_contracts1593–1621 unknownoldkey survivesnewkeysuccess;1626retains300.

Finite inputs and oracle: receipt_retention: heldbarrier/receipt known+unknown;otherkeysuccessnevereraseunknown; conservative samekeyunknownretention is permitted unless actualreconciliationprovided; no inventedcap/reclamationfeature.

Authority: No product-meaning decision; use existing public seam and registered predicate.

This batch owns only the named additional test portion. Referenced C017/package/native proof is a separate conjunction/barrier; it does not create a backward dependency on the final B17 runner. Existing known behavioral defects keep their named production batches. Unexpected baseline red is preserved and replanned, never silently repaired here.

### Batch B57: verify p20-sdk-classifier-023 finite source cases

**Closure predicate**: The finite llama_label_failure_recovery source cases produce exactly the registered results and side-effect counts below; typed invalid-input controls fail before the stated later boundary.
**Dependencies**: ["B19"]
**Verification union**: python scripts/opi-cargo-cache.py run --target <persistent-target> -- test -p opi-ai --test classifier_contract --offline --locked -- --test-threads=1; run every created or modified target/filter and affected-target clippy; final workspace/doc/whitespace union applies.

#### Fix B57.1: verify p20-sdk-classifier-023 finite source cases

- **Finding source(s)**: phase20-codex-gpt61-b3f1810-20261103t142820z / 4cec97af8ef6bb773c630ee251728f31212e12fdc1bf9631f10b2dc3084bc0f6 / P20-AUD-018
- **Decision**: fix:verify-p20-sdk-classifier-023-finite-source-cases
- **Verification status**: Partially confirmed
- **File(s)**: `crates/opi-ai/tests/classifier_contract.rs`
- **Change kind**: test-only
- **Change**: Add the exact finite assertion matrix below at its current owning seam; production behavior and dependencies are unchanged by this test-only batch.
- **Closure predicate**: The finite llama_label_failure_recovery source cases produce exactly the registered results and side-effect counts below; typed invalid-input controls fail before the stated later boundary.
- **Red-before**: Not applicable to production behavior: this batch adds the finite source-defined conformance test only. Preserve any actual baseline failure and stop for its separately bound behavioral repair; no conditional production edit is admitted.
- **Green-after**: python scripts/opi-cargo-cache.py run --target <persistent-target> -- test -p opi-ai --test classifier_contract --offline --locked -- --test-threads=1 -> The finite llama_label_failure_recovery source cases produce exactly the registered results and side-effect counts below; typed invalid-input controls fail before the stated later boundary. Observed:not-run.
- **Test impact**: add named assertions/cases; update helpers only as required; retain valid-source and negative controls; delete none.

Assigned current criterion: `P20-SDK-CLASSIFIER-023`; exact source `docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md` SHA256 `9e3d6ef690c3a794bc9c47ddc2dce04840f98d8881649be1ab2e8db0df287fd5`, citation `L129`.

Current assertion evidence and limits: llama913–955 newlineprefixthenalone;onlysuccesscached;collisionreject. classifier2485–2511 provesonly repeatedsameinstancecachehit.

Finite inputs and oracle: llama_label_failure_recovery:multi-tokenfails0completion then correctedretokenizes;collisionlabels→sametokenreject;uniquecontrol;prefix+one/exactfallback shapes.

Authority: No product-meaning decision; use existing public seam and registered predicate.

This batch owns only the named additional test portion. Referenced C017/package/native proof is a separate conjunction/barrier; it does not create a backward dependency on the final B17 runner. Existing known behavioral defects keep their named production batches. Unexpected baseline red is preserved and replanned, never silently repaired here.

### Batch B58: verify p20-sdk-classifier-024 finite source cases

**Closure predicate**: The finite llama_cache_owner_matrix source cases produce exactly the registered results and side-effect counts below; typed invalid-input controls fail before the stated later boundary.
**Dependencies**: ["B19"]
**Verification union**: python scripts/opi-cargo-cache.py run --target <persistent-target> -- test -p opi-ai --test classifier_contract --offline --locked -- --test-threads=1; run every created or modified target/filter and affected-target clippy; final workspace/doc/whitespace union applies.

#### Fix B58.1: verify p20-sdk-classifier-024 finite source cases

- **Finding source(s)**: phase20-codex-gpt61-b3f1810-20261103t142820z / 4cec97af8ef6bb773c630ee251728f31212e12fdc1bf9631f10b2dc3084bc0f6 / P20-AUD-018
- **Decision**: fix:verify-p20-sdk-classifier-024-finite-source-cases
- **Verification status**: Partially confirmed
- **File(s)**: `crates/opi-ai/tests/classifier_contract.rs`
- **Change kind**: test-only
- **Change**: Add the exact finite assertion matrix below at its current owning seam; production behavior and dependencies are unchanged by this test-only batch.
- **Closure predicate**: The finite llama_cache_owner_matrix source cases produce exactly the registered results and side-effect counts below; typed invalid-input controls fail before the stated later boundary.
- **Red-before**: Not applicable to production behavior: this batch adds the finite source-defined conformance test only. Preserve any actual baseline failure and stop for its separately bound behavioral repair; no conditional production edit is admitted.
- **Green-after**: python scripts/opi-cargo-cache.py run --target <persistent-target> -- test -p opi-ai --test classifier_contract --offline --locked -- --test-threads=1 -> The finite llama_cache_owner_matrix source cases produce exactly the registered results and side-effect counts below; typed invalid-input controls fail before the stated later boundary. Observed:not-run.
- **Test impact**: add named assertions/cases; update helpers only as required; retain valid-source and negative controls; delete none.

Assigned current criterion: `P20-SDK-CLASSIFIER-024`; exact source `docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md` SHA256 `9e3d6ef690c3a794bc9c47ddc2dce04840f98d8881649be1ab2e8db0df287fd5`, citation `L129`.

Current assertion evidence and limits: llama101 adapter-owned cache;935–951keyserverroot/model/label; existing2485 onlyhit.

Finite inputs and oracle: llama_cache_owner_matrix: eachkeydimensionchange misses;2sameid Models diffcredential/clienttokensisolated;explicitsharedsameadapter control sameownerhit;dropone leavesother.

Authority: No product-meaning decision; use existing public seam and registered predicate.

This batch owns only the named additional test portion. Referenced C017/package/native proof is a separate conjunction/barrier; it does not create a backward dependency on the final B17 runner. Existing known behavioral defects keep their named production batches. Unexpected baseline red is preserved and replanned, never silently repaired here.

### Batch B59: verify p20-sdk-model-003 finite source cases

**Closure predicate**: The finite thinking_clamp_table source cases produce exactly the registered results and side-effect counts below; typed invalid-input controls fail before the stated later boundary.
**Dependencies**: []
**Verification union**: python scripts/opi-cargo-cache.py run --target <persistent-target> -- test -p opi-ai --test model_wire_metadata --offline --locked -- --test-threads=1; python scripts/opi-cargo-cache.py run --target <persistent-target> -- test -p opi-ai --test provider_collection --offline --locked -- --test-threads=1; run every created or modified target/filter and affected-target clippy; final workspace/doc/whitespace union applies.

#### Fix B59.1: verify p20-sdk-model-003 finite source cases

- **Finding source(s)**: phase20-codex-gpt61-b3f1810-20261103t142820z / 4cec97af8ef6bb773c630ee251728f31212e12fdc1bf9631f10b2dc3084bc0f6 / P20-AUD-018
- **Decision**: fix:verify-p20-sdk-model-003-finite-source-cases
- **Verification status**: Partially confirmed
- **File(s)**: `crates/opi-ai/tests/model_wire_metadata.rs`, `crates/opi-ai/tests/provider_collection.rs`
- **Change kind**: test-only
- **Change**: Add the exact finite assertion matrix below at its current owning seam; production behavior and dependencies are unchanged by this test-only batch.
- **Closure predicate**: The finite thinking_clamp_table source cases produce exactly the registered results and side-effect counts below; typed invalid-input controls fail before the stated later boundary.
- **Red-before**: Not applicable to production behavior: this batch adds the finite source-defined conformance test only. Preserve any actual baseline failure and stop for its separately bound behavioral repair; no conditional production edit is admitted.
- **Green-after**: python scripts/opi-cargo-cache.py run --target <persistent-target> -- test -p opi-ai --test model_wire_metadata --offline --locked -- --test-threads=1; python scripts/opi-cargo-cache.py run --target <persistent-target> -- test -p opi-ai --test provider_collection --offline --locked -- --test-threads=1 -> The finite thinking_clamp_table source cases produce exactly the registered results and side-effect counts below; typed invalid-input controls fail before the stated later boundary. Observed:not-run.
- **Test impact**: add named assertions/cases; update helpers only as required; retain valid-source and negative controls; delete none.

Assigned current criterion: `P20-SDK-MODEL-003`; exact source `docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md` SHA256 `9e3d6ef690c3a794bc9c47ddc2dce04840f98d8881649be1ab2e8db0df287fd5`, citation `L160`.

Current assertion evidence and limits: model_info315–331 exact/upward/reversedownward logic.

Finite inputs and oracle: thinking_clamp_table: exact,higherfartherthanlowerstillwins,nohigher→nearestlower,allunsupportedNone,xhigh/maxexplicitnonnull; wire requested/effective vsnative reject.

Authority: No product-meaning decision; use existing public seam and registered predicate.

This batch owns only the named additional test portion. Referenced C017/package/native proof is a separate conjunction/barrier; it does not create a backward dependency on the final B17 runner. Existing known behavioral defects keep their named production batches. Unexpected baseline red is preserved and replanned, never silently repaired here.

### Batch B60: verify p20-sdk-model-022 finite source cases

**Closure predicate**: The finite default_budget_table source cases produce exactly the registered results and side-effect counts below; typed invalid-input controls fail before the stated later boundary.
**Dependencies**: ["B08"]
**Verification union**: python scripts/opi-cargo-cache.py run --target <persistent-target> -- test -p opi-ai --test anthropic_fixtures --offline --locked -- --test-threads=1; python scripts/opi-cargo-cache.py run --target <persistent-target> -- test -p opi-ai --test bedrock_fixtures --offline --locked -- --test-threads=1; python scripts/opi-cargo-cache.py run --target <persistent-target> -- test -p opi-ai --test provider_collection --offline --locked -- --test-threads=1; run every created or modified target/filter and affected-target clippy; final workspace/doc/whitespace union applies.

#### Fix B60.1: verify p20-sdk-model-022 finite source cases

- **Finding source(s)**: phase20-codex-gpt61-b3f1810-20261103t142820z / 4cec97af8ef6bb773c630ee251728f31212e12fdc1bf9631f10b2dc3084bc0f6 / P20-AUD-018
- **Decision**: fix:verify-p20-sdk-model-022-finite-source-cases
- **Verification status**: Partially confirmed
- **File(s)**: `crates/opi-ai/tests/provider_collection.rs`, `crates/opi-ai/tests/anthropic_fixtures.rs`, `crates/opi-ai/tests/bedrock_fixtures.rs`
- **Change kind**: test-only
- **Change**: Add the exact finite assertion matrix below at its current owning seam; production behavior and dependencies are unchanged by this test-only batch.
- **Closure predicate**: The finite default_budget_table source cases produce exactly the registered results and side-effect counts below; typed invalid-input controls fail before the stated later boundary.
- **Red-before**: Not applicable to production behavior: this batch adds the finite source-defined conformance test only. Preserve any actual baseline failure and stop for its separately bound behavioral repair; no conditional production edit is admitted.
- **Green-after**: python scripts/opi-cargo-cache.py run --target <persistent-target> -- test -p opi-ai --test anthropic_fixtures --offline --locked -- --test-threads=1; python scripts/opi-cargo-cache.py run --target <persistent-target> -- test -p opi-ai --test bedrock_fixtures --offline --locked -- --test-threads=1; python scripts/opi-cargo-cache.py run --target <persistent-target> -- test -p opi-ai --test provider_collection --offline --locked -- --test-threads=1 -> The finite default_budget_table source cases produce exactly the registered results and side-effect counts below; typed invalid-input controls fail before the stated later boundary. Observed:not-run.
- **Test impact**: add named assertions/cases; update helpers only as required; retain valid-source and negative controls; delete none.

Assigned current criterion: `P20-SDK-MODEL-022`; exact source `docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md` SHA256 `9e3d6ef690c3a794bc9c47ddc2dce04840f98d8881649be1ab2e8db0df287fd5`, citation `L165`.

Current assertion evidence and limits: models3971–3981numeric budgettable1024/2048/8192/16384;XHighMax→High.

Finite inputs and oracle: default_budget_table: each6level+None,unsaturated/saturatedC008ceilings;nativeexplicitunchanged;ChatbudgetnotClaudeaddition.

Authority: No product-meaning decision; use existing public seam and registered predicate.

This batch owns only the named additional test portion. Referenced C017/package/native proof is a separate conjunction/barrier; it does not create a backward dependency on the final B17 runner. Existing known behavioral defects keep their named production batches. Unexpected baseline red is preserved and replanned, never silently repaired here.

### Batch B61: verify p20-sdk-model-026 finite source cases

**Closure predicate**: The finite zero_remaining_wire_table source cases produce exactly the registered results and side-effect counts below; typed invalid-input controls fail before the stated later boundary.
**Dependencies**: ["B08"]
**Verification union**: python scripts/opi-cargo-cache.py run --target <persistent-target> -- test -p opi-ai --test anthropic_fixtures --offline --locked -- --test-threads=1; python scripts/opi-cargo-cache.py run --target <persistent-target> -- test -p opi-ai --test bedrock_fixtures --offline --locked -- --test-threads=1; python scripts/opi-cargo-cache.py run --target <persistent-target> -- test -p opi-ai --test openai_chat_fixtures --offline --locked -- --test-threads=1; python scripts/opi-cargo-cache.py run --target <persistent-target> -- test -p opi-ai --test provider_collection --offline --locked -- --test-threads=1; run every created or modified target/filter and affected-target clippy; final workspace/doc/whitespace union applies.

#### Fix B61.1: verify p20-sdk-model-026 finite source cases

- **Finding source(s)**: phase20-codex-gpt61-b3f1810-20261103t142820z / 4cec97af8ef6bb773c630ee251728f31212e12fdc1bf9631f10b2dc3084bc0f6 / P20-AUD-018
- **Decision**: fix:verify-p20-sdk-model-026-finite-source-cases
- **Verification status**: Partially confirmed
- **File(s)**: `crates/opi-ai/tests/provider_collection.rs`, `crates/opi-ai/tests/anthropic_fixtures.rs`, `crates/opi-ai/tests/bedrock_fixtures.rs`, `crates/opi-ai/tests/openai_chat_fixtures.rs`
- **Change kind**: test-only
- **Change**: Add the exact finite assertion matrix below at its current owning seam; production behavior and dependencies are unchanged by this test-only batch.
- **Closure predicate**: The finite zero_remaining_wire_table source cases produce exactly the registered results and side-effect counts below; typed invalid-input controls fail before the stated later boundary.
- **Red-before**: Not applicable to production behavior: this batch adds the finite source-defined conformance test only. Preserve any actual baseline failure and stop for its separately bound behavioral repair; no conditional production edit is admitted.
- **Green-after**: python scripts/opi-cargo-cache.py run --target <persistent-target> -- test -p opi-ai --test anthropic_fixtures --offline --locked -- --test-threads=1; python scripts/opi-cargo-cache.py run --target <persistent-target> -- test -p opi-ai --test bedrock_fixtures --offline --locked -- --test-threads=1; python scripts/opi-cargo-cache.py run --target <persistent-target> -- test -p opi-ai --test openai_chat_fixtures --offline --locked -- --test-threads=1; python scripts/opi-cargo-cache.py run --target <persistent-target> -- test -p opi-ai --test provider_collection --offline --locked -- --test-threads=1 -> The finite zero_remaining_wire_table source cases produce exactly the registered results and side-effect counts below; typed invalid-input controls fail before the stated later boundary. Observed:not-run.
- **Test impact**: add named assertions/cases; update helpers only as required; retain valid-source and negative controls; delete none.

Assigned current criterion: `P20-SDK-MODEL-026`; exact source `docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md` SHA256 `9e3d6ef690c3a794bc9c47ddc2dce04840f98d8881649be1ab2e8db0df287fd5`, citation `L165`.

Current assertion evidence and limits: models3984–3990zeroallowed;Anthropicfixtures3603native0→1024sourceor semantics.

Finite inputs and oracle: zero_remaining_wire_table:Anthropicceiling<1024→budget1024;Bedrock→0;Chatomit0;no new preflightsolelyzero;positive/absentcontrols.

Authority: No product-meaning decision; use existing public seam and registered predicate.

This batch owns only the named additional test portion. Referenced C017/package/native proof is a separate conjunction/barrier; it does not create a backward dependency on the final B17 runner. Existing known behavioral defects keep their named production batches. Unexpected baseline red is preserved and replanned, never silently repaired here.

### Batch B63: verify p20-sdk-model-030 finite source cases

**Closure predicate**: The finite compat_presence_matrix source cases produce exactly the registered results and side-effect counts below; typed invalid-input controls fail before the stated later boundary.
**Dependencies**: ["B22", "B23", "B24", "B25", "B26", "B27"]
**Verification union**: python scripts/opi-cargo-cache.py run --target <persistent-target> -- test -p opi-ai --test catalog_lifecycle --offline --locked -- --test-threads=1; python scripts/opi-cargo-cache.py run --target <persistent-target> -- test -p opi-ai --test model_wire_metadata --offline --locked -- --test-threads=1; run every created or modified target/filter and affected-target clippy; final workspace/doc/whitespace union applies.

#### Fix B63.1: verify p20-sdk-model-030 finite source cases

- **Finding source(s)**: phase20-codex-gpt61-b3f1810-20261103t142820z / 4cec97af8ef6bb773c630ee251728f31212e12fdc1bf9631f10b2dc3084bc0f6 / P20-AUD-018
- **Decision**: fix:verify-p20-sdk-model-030-finite-source-cases
- **Verification status**: Partially confirmed
- **File(s)**: `crates/opi-ai/tests/model_wire_metadata.rs`, `crates/opi-ai/tests/catalog_lifecycle.rs`
- **Change kind**: test-only
- **Change**: Add the exact finite assertion matrix below at its current owning seam; production behavior and dependencies are unchanged by this test-only batch.
- **Closure predicate**: The finite compat_presence_matrix source cases produce exactly the registered results and side-effect counts below; typed invalid-input controls fail before the stated later boundary.
- **Red-before**: Not applicable to production behavior: this batch adds the finite source-defined conformance test only. Preserve any actual baseline failure and stop for its separately bound behavioral repair; no conditional production edit is admitted.
- **Green-after**: python scripts/opi-cargo-cache.py run --target <persistent-target> -- test -p opi-ai --test catalog_lifecycle --offline --locked -- --test-threads=1; python scripts/opi-cargo-cache.py run --target <persistent-target> -- test -p opi-ai --test model_wire_metadata --offline --locked -- --test-threads=1 -> The finite compat_presence_matrix source cases produce exactly the registered results and side-effect counts below; typed invalid-input controls fail before the stated later boundary. Observed:not-run.
- **Test impact**: add named assertions/cases; update helpers only as required; retain valid-source and negative controls; delete none.

Assigned current criterion: `P20-SDK-MODEL-030`; exact source `docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md` SHA256 `9e3d6ef690c3a794bc9c47ddc2dce04840f98d8881649be1ab2e8db0df287fd5`, citation `L168`.

Current assertion evidence and limits: model_wire_metadata74typedwiremismatch;91–131routingempty/nestednullandinvalidtypes;C009red missingAnthropicbranches.

Finite inputs and oracle: compat_presence_matrix: wirefieldabsent/false/true/nullasallowed,wrongwirezeroauth,unknownextensionpersistsbutnoeffectiveflag,exactpayloadnewfields.

Authority: No product-meaning decision; follow registered predicate with existing seams.

This batch owns only the named additional test portion. Referenced C017/package/native proof is a separate conjunction/barrier; it does not create a backward dependency on the final B17 runner. Existing known behavioral defects keep their named production batches. Unexpected baseline red is preserved and replanned, never silently repaired here.

### Batch B64: verify p20-sdk-model-038 finite source cases

**Closure predicate**: The finite responses_default_matrix source cases produce exactly the registered results and side-effect counts below; typed invalid-input controls fail before the stated later boundary.
**Dependencies**: []
**Verification union**: python scripts/opi-cargo-cache.py run --target <persistent-target> -- test -p opi-ai --test azure_openai_fixtures --offline --locked -- --test-threads=1; python scripts/opi-cargo-cache.py run --target <persistent-target> -- test -p opi-ai --test codex_native_options --offline --locked -- --test-threads=1; python scripts/opi-cargo-cache.py run --target <persistent-target> -- test -p opi-ai --test openai_responses_fixtures --offline --locked -- --test-threads=1; run every created or modified target/filter and affected-target clippy; final workspace/doc/whitespace union applies.

#### Fix B64.1: verify p20-sdk-model-038 finite source cases

- **Finding source(s)**: phase20-codex-gpt61-b3f1810-20261103t142820z / 4cec97af8ef6bb773c630ee251728f31212e12fdc1bf9631f10b2dc3084bc0f6 / P20-AUD-018
- **Decision**: fix:verify-p20-sdk-model-038-finite-source-cases
- **Verification status**: Partially confirmed
- **File(s)**: `crates/opi-ai/tests/openai_responses_fixtures.rs`, `crates/opi-ai/tests/azure_openai_fixtures.rs`, `crates/opi-ai/tests/codex_native_options.rs`
- **Change kind**: test-only
- **Change**: Add the exact finite assertion matrix below at its current owning seam; production behavior and dependencies are unchanged by this test-only batch.
- **Closure predicate**: The finite responses_default_matrix source cases produce exactly the registered results and side-effect counts below; typed invalid-input controls fail before the stated later boundary.
- **Red-before**: Not applicable to production behavior: this batch adds the finite source-defined conformance test only. Preserve any actual baseline failure and stop for its separately bound behavioral repair; no conditional production edit is admitted.
- **Green-after**: python scripts/opi-cargo-cache.py run --target <persistent-target> -- test -p opi-ai --test azure_openai_fixtures --offline --locked -- --test-threads=1; python scripts/opi-cargo-cache.py run --target <persistent-target> -- test -p opi-ai --test codex_native_options --offline --locked -- --test-threads=1; python scripts/opi-cargo-cache.py run --target <persistent-target> -- test -p opi-ai --test openai_responses_fixtures --offline --locked -- --test-threads=1 -> The finite responses_default_matrix source cases produce exactly the registered results and side-effect counts below; typed invalid-input controls fail before the stated later boundary. Observed:not-run.
- **Test impact**: add named assertions/cases; update helpers only as required; retain valid-source and negative controls; delete none.

Assigned current criterion: `P20-SDK-MODEL-038`; exact source `docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md` SHA256 `9e3d6ef690c3a794bc9c47ddc2dce04840f98d8881649be1ab2e8db0df287fd5`, citation `L175`.

Current assertion evidence and limits: RegisteredSDK175wiredefaults; model_info distinctResponses/Azure/Codex configurations; testnames alone insufficient.

Finite inputs and oracle: responses_default_matrix:3familyconsumers x absent/explicitflags,developer/maxouttruewhereapplies,mid-system/grammar/additionaltools/toolsearch/explicitcachefalse; wire-specificstrict;inspectpayload/headersandunsupportedcontrols.

Authority: No product-meaning decision; follow registered predicate with existing seams.

This batch owns only the named additional test portion. Referenced C017/package/native proof is a separate conjunction/barrier; it does not create a backward dependency on the final B17 runner. Existing known behavioral defects keep their named production batches. Unexpected baseline red is preserved and replanned, never silently repaired here.

### Batch B65: verify p20-sdk-model-042 finite source cases

**Closure predicate**: The finite bedrock_mistral_defaults_local source cases produce exactly the registered results and side-effect counts below; typed invalid-input controls fail before the stated later boundary.
**Dependencies**: ["B14", "B16"]
**Verification union**: python scripts/opi-cargo-cache.py run --target <persistent-target> -- test -p opi-ai --test bedrock_fixtures --offline --locked -- --test-threads=1; python scripts/opi-cargo-cache.py run --target <persistent-target> -- test -p opi-ai --test mistral_fixtures --offline --locked -- --test-threads=1; run every created or modified target/filter and affected-target clippy; final workspace/doc/whitespace union applies.

#### Fix B65.1: verify p20-sdk-model-042 finite source cases

- **Finding source(s)**: phase20-codex-gpt61-b3f1810-20261103t142820z / 4cec97af8ef6bb773c630ee251728f31212e12fdc1bf9631f10b2dc3084bc0f6 / P20-AUD-018
- **Decision**: fix:verify-p20-sdk-model-042-finite-source-cases
- **Verification status**: Partially confirmed
- **File(s)**: `crates/opi-ai/tests/bedrock_fixtures.rs`, `crates/opi-ai/tests/mistral_fixtures.rs`
- **Change kind**: test-only
- **Change**: Add the exact finite assertion matrix below at its current owning seam; production behavior and dependencies are unchanged by this test-only batch.
- **Closure predicate**: The finite bedrock_mistral_defaults_local source cases produce exactly the registered results and side-effect counts below; typed invalid-input controls fail before the stated later boundary.
- **Red-before**: Not applicable to production behavior: this batch adds the finite source-defined conformance test only. Preserve any actual baseline failure and stop for its separately bound behavioral repair; no conditional production edit is admitted.
- **Green-after**: python scripts/opi-cargo-cache.py run --target <persistent-target> -- test -p opi-ai --test bedrock_fixtures --offline --locked -- --test-threads=1; python scripts/opi-cargo-cache.py run --target <persistent-target> -- test -p opi-ai --test mistral_fixtures --offline --locked -- --test-threads=1 -> The finite bedrock_mistral_defaults_local source cases produce exactly the registered results and side-effect counts below; typed invalid-input controls fail before the stated later boundary. Observed:not-run.
- **Test impact**: add named assertions/cases; update helpers only as required; retain valid-source and negative controls; delete none.

Assigned current criterion: `P20-SDK-MODEL-042`; exact source `docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md` SHA256 `9e3d6ef690c3a794bc9c47ddc2dce04840f98d8881649be1ab2e8db0df287fd5`, citation `L177`.

Current assertion evidence and limits: model_info341–346Bedrockstrictdefaultfalse;SDK177Mistralmidsystemfalse;rootcurrentMistralsharedflagredexists.

Finite inputs and oracle: bedrock_mistral_defaults_local:absent/explicittrue/false onbothwires under sameproviderid;strictschema/midsystemactualbody andnocrosscontamination.

Authority: No product-meaning decision; follow registered predicate with existing seams.

This batch owns only the named additional test portion. Referenced C017/package/native proof is a separate conjunction/barrier; it does not create a backward dependency on the final B17 runner. Existing known behavioral defects keep their named production batches. Unexpected baseline red is preserved and replanned, never silently repaired here.

### Batch B66: verify p20-sdk-utility-028 finite source cases

**Closure predicate**: The finite isolated_surrogate_final source cases produce exactly the registered results and side-effect counts below; typed invalid-input controls fail before the stated later boundary.
**Dependencies**: []
**Verification union**: python scripts/opi-cargo-cache.py run --target <persistent-target> -- test -p opi-ai --test public_utilities --offline --locked -- --test-threads=1; python scripts/opi-cargo-cache.py run --target <persistent-target> -- test -p opi-ai --test replay_contract --offline --locked -- --test-threads=1; run every created or modified target/filter and affected-target clippy; final workspace/doc/whitespace union applies.

#### Fix B66.1: verify p20-sdk-utility-028 finite source cases

- **Finding source(s)**: phase20-codex-gpt61-b3f1810-20261103t142820z / 4cec97af8ef6bb773c630ee251728f31212e12fdc1bf9631f10b2dc3084bc0f6 / P20-AUD-018
- **Decision**: fix:verify-p20-sdk-utility-028-finite-source-cases
- **Verification status**: Partially confirmed
- **File(s)**: `crates/opi-ai/tests/public_utilities.rs`, `crates/opi-ai/tests/replay_contract.rs`
- **Change kind**: test-only
- **Change**: Add the exact finite assertion matrix below at its current owning seam; production behavior and dependencies are unchanged by this test-only batch.
- **Closure predicate**: The finite isolated_surrogate_final source cases produce exactly the registered results and side-effect counts below; typed invalid-input controls fail before the stated later boundary.
- **Red-before**: Not applicable to production behavior: this batch adds the finite source-defined conformance test only. Preserve any actual baseline failure and stop for its separately bound behavioral repair; no conditional production edit is admitted.
- **Green-after**: python scripts/opi-cargo-cache.py run --target <persistent-target> -- test -p opi-ai --test public_utilities --offline --locked -- --test-threads=1; python scripts/opi-cargo-cache.py run --target <persistent-target> -- test -p opi-ai --test replay_contract --offline --locked -- --test-threads=1 -> The finite isolated_surrogate_final source cases produce exactly the registered results and side-effect counts below; typed invalid-input controls fail before the stated later boundary. Observed:not-run.
- **Test impact**: add named assertions/cases; update helpers only as required; retain valid-source and negative controls; delete none.

Assigned current criterion: `P20-SDK-UTILITY-028`; exact source `docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md` SHA256 `9e3d6ef690c3a794bc9c47ddc2dce04840f98d8881649be1ab2e8db0df287fd5`, citation `L190`.

Current assertion evidence and limits: public_utilities392–422 preservesmalformedfinalraw+error,butnotsurrogates;serde_jsonString rejectsisolatedsurrogate.

Finite inputs and oracle: isolated_surrogate_final:escapedhigh/lowalone message/toolargs typedreject,rawescapedargumentretainedwithparseerror;validpairscalarcontrol;partialrepairnotfinalauthority.

Authority: No product-meaning decision; follow registered predicate with existing seams.

This batch owns only the named additional test portion. Referenced C017/package/native proof is a separate conjunction/barrier; it does not create a backward dependency on the final B17 runner. Existing known behavioral defects keep their named production batches. Unexpected baseline red is preserved and replanned, never silently repaired here.

### Batch B67: verify p20-sdk-utility-031 finite source cases

**Closure predicate**: The finite nonvision_placeholder_runs source cases produce exactly the registered results and side-effect counts below; typed invalid-input controls fail before the stated later boundary.
**Dependencies**: []
**Verification union**: python scripts/opi-cargo-cache.py run --target <persistent-target> -- test -p opi-ai --test replay_contract --offline --locked -- --test-threads=1; run every created or modified target/filter and affected-target clippy; final workspace/doc/whitespace union applies.

#### Fix B67.1: verify p20-sdk-utility-031 finite source cases

- **Finding source(s)**: phase20-codex-gpt61-b3f1810-20261103t142820z / 4cec97af8ef6bb773c630ee251728f31212e12fdc1bf9631f10b2dc3084bc0f6 / P20-AUD-018
- **Decision**: fix:verify-p20-sdk-utility-031-finite-source-cases
- **Verification status**: Partially confirmed
- **File(s)**: `crates/opi-ai/tests/replay_contract.rs`
- **Change kind**: test-only
- **Change**: Add the exact finite assertion matrix below at its current owning seam; production behavior and dependencies are unchanged by this test-only batch.
- **Closure predicate**: The finite nonvision_placeholder_runs source cases produce exactly the registered results and side-effect counts below; typed invalid-input controls fail before the stated later boundary.
- **Red-before**: Not applicable to production behavior: this batch adds the finite source-defined conformance test only. Preserve any actual baseline failure and stop for its separately bound behavioral repair; no conditional production edit is admitted.
- **Green-after**: python scripts/opi-cargo-cache.py run --target <persistent-target> -- test -p opi-ai --test replay_contract --offline --locked -- --test-threads=1 -> The finite nonvision_placeholder_runs source cases produce exactly the registered results and side-effect counts below; typed invalid-input controls fail before the stated later boundary. Observed:not-run.
- **Test impact**: add named assertions/cases; update helpers only as required; retain valid-source and negative controls; delete none.

Assigned current criterion: `P20-SDK-UTILITY-031`; exact source `docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md` SHA256 `9e3d6ef690c3a794bc9c47ddc2dce04840f98d8881649be1ab2e8db0df287fd5`, citation `L191`.

Current assertion evidence and limits: replay_contract964–998 collisiongeneratedIDtwo distinctids,matchingresults,stablewireandcanonicalunchanged;bedrock1436factualtoolimageplaceholder.

Finite inputs and oracle: nonvision_placeholder_runs:consecutiveuser/toolimages eachownplaceholderdedup;textbreakpreservesseparation;IDsbijectiveindependentofname;missingresultunknownexecutionnotguessed.

Authority: No product-meaning decision; follow registered predicate with existing seams.

This batch owns only the named additional test portion. Referenced C017/package/native proof is a separate conjunction/barrier; it does not create a backward dependency on the final B17 runner. Existing known behavioral defects keep their named production batches. Unexpected baseline red is preserved and replanned, never silently repaired here.

### Batch B68: verify p20-sdk-utility-032 finite source cases

**Closure predicate**: The finite grammar_require_prefer source cases produce exactly the registered results and side-effect counts below; typed invalid-input controls fail before the stated later boundary.
**Dependencies**: []
**Verification union**: python scripts/opi-cargo-cache.py run --target <persistent-target> -- test -p opi-ai --test anthropic_fixtures --offline --locked -- --test-threads=1; python scripts/opi-cargo-cache.py run --target <persistent-target> -- test -p opi-ai --test bedrock_fixtures --offline --locked -- --test-threads=1; python scripts/opi-cargo-cache.py run --target <persistent-target> -- test -p opi-ai --test public_utilities --offline --locked -- --test-threads=1; run every created or modified target/filter and affected-target clippy; final workspace/doc/whitespace union applies.

#### Fix B68.1: verify p20-sdk-utility-032 finite source cases

- **Finding source(s)**: phase20-codex-gpt61-b3f1810-20261103t142820z / 4cec97af8ef6bb773c630ee251728f31212e12fdc1bf9631f10b2dc3084bc0f6 / P20-AUD-018
- **Decision**: fix:verify-p20-sdk-utility-032-finite-source-cases
- **Verification status**: Partially confirmed
- **File(s)**: `crates/opi-ai/tests/public_utilities.rs`, `crates/opi-ai/tests/anthropic_fixtures.rs`, `crates/opi-ai/tests/bedrock_fixtures.rs`
- **Change kind**: test-only
- **Change**: Add the exact finite assertion matrix below at its current owning seam; production behavior and dependencies are unchanged by this test-only batch.
- **Closure predicate**: The finite grammar_require_prefer source cases produce exactly the registered results and side-effect counts below; typed invalid-input controls fail before the stated later boundary.
- **Red-before**: Not applicable to production behavior: this batch adds the finite source-defined conformance test only. Preserve any actual baseline failure and stop for its separately bound behavioral repair; no conditional production edit is admitted.
- **Green-after**: python scripts/opi-cargo-cache.py run --target <persistent-target> -- test -p opi-ai --test anthropic_fixtures --offline --locked -- --test-threads=1; python scripts/opi-cargo-cache.py run --target <persistent-target> -- test -p opi-ai --test bedrock_fixtures --offline --locked -- --test-threads=1; python scripts/opi-cargo-cache.py run --target <persistent-target> -- test -p opi-ai --test public_utilities --offline --locked -- --test-threads=1 -> The finite grammar_require_prefer source cases produce exactly the registered results and side-effect counts below; typed invalid-input controls fail before the stated later boundary. Observed:not-run.
- **Test impact**: add named assertions/cases; update helpers only as required; retain valid-source and negative controls; delete none.

Assigned current criterion: `P20-SDK-UTILITY-032`; exact source `docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md` SHA256 `9e3d6ef690c3a794bc9c47ddc2dce04840f98d8881649be1ab2e8db0df287fd5`, citation `L191`.

Current assertion evidence and limits: constrained_sampling470/556 owners;Anthropic2316–2344 grammarpreference/fallback/missingvariant;public_utilitiesliteralpropertycontrols.

Finite inputs and oracle: grammar_require_prefer:capable/incapable x prefer/require;unsupportedrequiredkeywordtypedzeroHTTP;preferfallbacktruthfulnonstrictwithoutconstraintdeletion; literalproperty/union/unknownkeywordcontrols.

Authority: No product-meaning decision; follow registered predicate with existing seams.

This batch owns only the named additional test portion. Referenced C017/package/native proof is a separate conjunction/barrier; it does not create a backward dependency on the final B17 runner. Existing known behavioral defects keep their named production batches. Unexpected baseline red is preserved and replanned, never silently repaired here.

### Batch B69: verify p20-sdk-utility-034 finite source cases

**Closure predicate**: The finite copilot_headers source cases produce exactly the registered results and side-effect counts below; typed invalid-input controls fail before the stated later boundary.
**Dependencies**: []
**Verification union**: python scripts/opi-cargo-cache.py run --target <persistent-target> -- test -p opi-ai --test oauth_flows --offline --locked -- --test-threads=1; python scripts/opi-cargo-cache.py run --target <persistent-target> -- test -p opi-ai --test request_enrichment --offline --locked -- --test-threads=1; run every created or modified target/filter and affected-target clippy; final workspace/doc/whitespace union applies.

#### Fix B69.1: verify p20-sdk-utility-034 finite source cases

- **Finding source(s)**: phase20-codex-gpt61-b3f1810-20261103t142820z / 4cec97af8ef6bb773c630ee251728f31212e12fdc1bf9631f10b2dc3084bc0f6 / P20-AUD-018
- **Decision**: fix:verify-p20-sdk-utility-034-finite-source-cases
- **Verification status**: Partially confirmed
- **File(s)**: `crates/opi-ai/tests/request_enrichment.rs`, `crates/opi-ai/tests/oauth_flows.rs`
- **Change kind**: test-only
- **Change**: Add the exact finite assertion matrix below at its current owning seam; production behavior and dependencies are unchanged by this test-only batch.
- **Closure predicate**: The finite copilot_headers source cases produce exactly the registered results and side-effect counts below; typed invalid-input controls fail before the stated later boundary.
- **Red-before**: Not applicable to production behavior: this batch adds the finite source-defined conformance test only. Preserve any actual baseline failure and stop for its separately bound behavioral repair; no conditional production edit is admitted.
- **Green-after**: python scripts/opi-cargo-cache.py run --target <persistent-target> -- test -p opi-ai --test oauth_flows --offline --locked -- --test-threads=1; python scripts/opi-cargo-cache.py run --target <persistent-target> -- test -p opi-ai --test request_enrichment --offline --locked -- --test-threads=1 -> The finite copilot_headers source cases produce exactly the registered results and side-effect counts below; typed invalid-input controls fail before the stated later boundary. Observed:not-run.
- **Test impact**: add named assertions/cases; update helpers only as required; retain valid-source and negative controls; delete none.

Assigned current criterion: `P20-SDK-UTILITY-034`; exact source `docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md` SHA256 `9e3d6ef690c3a794bc9c47ddc2dce04840f98d8881649be1ab2e8db0df287fd5`, citation `L192`.

Current assertion evidence and limits: provider354Openai-Intentconversation-edits,contains_image_contentvision;362–365lastmessageUser/Noneuser,otheragent.

Finite inputs and oracle: copilot_headers:emptyhistory/userlast/assistantlast/toolresultlast xChat/Responses;user/toolimagevisiontrue;thinkingonlyfalse; exactintent;protectedcallerheaderreject;accountbaseURLretained.

Authority: Use pinned lastmessage semantics; do not invent skipping emptycontentmessages.

This batch owns only the named additional test portion. Referenced C017/package/native proof is a separate conjunction/barrier; it does not create a backward dependency on the final B17 runner. Existing known behavioral defects keep their named production batches. Unexpected baseline red is preserved and replanned, never silently repaired here.

### Batch B70: verify p20-sdk-catalog-016 finite source cases

**Closure predicate**: The finite caller_owned_remote_policy source cases produce exactly the registered results and side-effect counts below; typed invalid-input controls fail before the stated later boundary.
**Dependencies**: []
**Verification union**: python scripts/opi-cargo-cache.py run --target <persistent-target> -- test -p opi-ai --test catalog_lifecycle --offline --locked -- --test-threads=1; python scripts/opi-cargo-cache.py run --target <persistent-target> -- test -p opi-ai --test catalog_operations --offline --locked -- --test-threads=1; run every created or modified target/filter and affected-target clippy; final workspace/doc/whitespace union applies.

#### Fix B70.1: verify p20-sdk-catalog-016 finite source cases

- **Finding source(s)**: phase20-codex-gpt61-b3f1810-20261103t142820z / 4cec97af8ef6bb773c630ee251728f31212e12fdc1bf9631f10b2dc3084bc0f6 / P20-AUD-018
- **Decision**: fix:verify-p20-sdk-catalog-016-finite-source-cases
- **Verification status**: Partially confirmed
- **File(s)**: `crates/opi-ai/tests/catalog_lifecycle.rs`, `crates/opi-ai/tests/catalog_operations.rs`
- **Change kind**: test-only
- **Change**: Add the exact finite assertion matrix below at its current owning seam; production behavior and dependencies are unchanged by this test-only batch.
- **Closure predicate**: The finite caller_owned_remote_policy source cases produce exactly the registered results and side-effect counts below; typed invalid-input controls fail before the stated later boundary.
- **Red-before**: Not applicable to production behavior: this batch adds the finite source-defined conformance test only. Preserve any actual baseline failure and stop for its separately bound behavioral repair; no conditional production edit is admitted.
- **Green-after**: python scripts/opi-cargo-cache.py run --target <persistent-target> -- test -p opi-ai --test catalog_lifecycle --offline --locked -- --test-threads=1; python scripts/opi-cargo-cache.py run --target <persistent-target> -- test -p opi-ai --test catalog_operations --offline --locked -- --test-threads=1 -> The finite caller_owned_remote_policy source cases produce exactly the registered results and side-effect counts below; typed invalid-input controls fail before the stated later boundary. Observed:not-run.
- **Test impact**: add named assertions/cases; update helpers only as required; retain valid-source and negative controls; delete none.

Assigned current criterion: `P20-SDK-CATALOG-016`; exact source `docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md` SHA256 `9e3d6ef690c3a794bc9c47ddc2dce04840f98d8881649be1ab2e8db0df287fd5`, citation `L206`.

Current assertion evidence and limits: catalog/http_source21source-ownedfreshness;130–170statelessGETnon2xxerror. SDK206genericproductwrapper4h304404501;V56forbidsdefaultproductpolicyinSDK.

Finite inputs and oracle: caller_owned_remote_policy:injectedDynamicCatalogSource clock3h59m59snoGET,4hGET;304requirescachedbody;404501explicitpersistedtombstone;personalization404retain/Radiusemptycontrols;forceNone/false/trueforwarded.

Authority: Test explicit caller/source policy plusSDKmechanism; do not claim defaultHttpCatalogSource implementsproductwrapper. Productionwrapperdelivery interpretation requires owningproductscope.

This batch owns only the named additional test portion. Referenced C017/package/native proof is a separate conjunction/barrier; it does not create a backward dependency on the final B17 runner. Existing known behavioral defects keep their named production batches. Unexpected baseline red is preserved and replanned, never silently repaired here.

### Batch B71: verify p20-mech-auth-001 finite source cases

**Closure predicate**: The finite acquisition_outcomes source cases produce exactly the registered results and side-effect counts below; typed invalid-input controls fail before the stated later boundary.
**Dependencies**: []
**Verification union**: python scripts/opi-cargo-cache.py run --target <persistent-target> -- test -p opi-ai --test auth_contracts --offline --locked -- --test-threads=1; python scripts/opi-cargo-cache.py run --target <persistent-target> -- test -p opi-ai --test bedrock_fixtures --offline --locked -- --test-threads=1; run every created or modified target/filter and affected-target clippy; final workspace/doc/whitespace union applies.

#### Fix B71.1: verify p20-mech-auth-001 finite source cases

- **Finding source(s)**: phase20-codex-gpt61-b3f1810-20261103t142820z / 4cec97af8ef6bb773c630ee251728f31212e12fdc1bf9631f10b2dc3084bc0f6 / P20-AUD-018
- **Decision**: fix:verify-p20-mech-auth-001-finite-source-cases
- **Verification status**: Partially confirmed
- **File(s)**: `crates/opi-ai/tests/bedrock_fixtures.rs`, `crates/opi-ai/tests/auth_contracts.rs`
- **Change kind**: test-only
- **Change**: Add the exact finite assertion matrix below at its current owning seam; production behavior and dependencies are unchanged by this test-only batch.
- **Closure predicate**: The finite acquisition_outcomes source cases produce exactly the registered results and side-effect counts below; typed invalid-input controls fail before the stated later boundary.
- **Red-before**: Not applicable to production behavior: this batch adds the finite source-defined conformance test only. Preserve any actual baseline failure and stop for its separately bound behavioral repair; no conditional production edit is admitted.
- **Green-after**: python scripts/opi-cargo-cache.py run --target <persistent-target> -- test -p opi-ai --test auth_contracts --offline --locked -- --test-threads=1; python scripts/opi-cargo-cache.py run --target <persistent-target> -- test -p opi-ai --test bedrock_fixtures --offline --locked -- --test-threads=1 -> The finite acquisition_outcomes source cases produce exactly the registered results and side-effect counts below; typed invalid-input controls fail before the stated later boundary. Observed:not-run.
- **Test impact**: add named assertions/cases; update helpers only as required; retain valid-source and negative controls; delete none.

Assigned current criterion: `P20-MECH-AUTH-001`; exact source `docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-transport-auth.md` SHA256 `42e1bc90c02e958ada4d2a01db999cd906ae33ee971496c50c07ed38067cdcfb`, citation `L13,L101`.

Current assertion evidence and limits: builtin_aws57–82/161–188 andaws/local ownContinueSelectedTerminal;bedrock111–169zeroIOexplicitidentity;4453–4510selecteddenialterminal.

Finite inputs and oracle: acquisition_outcomes:sourceabsenceContinue,selectedsuccessidentityexpiry,selectedservicefailureTerminal;separateread/exchange/generationcounts andsecretredaction

Authority: No product-meaning decision; follow registered predicate with existing seams.

This batch owns only the named additional test portion. Referenced C017/package/native proof is a separate conjunction/barrier; it does not create a backward dependency on the final B17 runner. Existing known behavioral defects keep their named production batches. Unexpected baseline red is preserved and replanned, never silently repaired here.

### Batch B72: verify p20-mech-auth-002 finite source cases

**Closure predicate**: The finite source_generation_budgets source cases produce exactly the registered results and side-effect counts below; typed invalid-input controls fail before the stated later boundary.
**Dependencies**: []
**Verification union**: python scripts/opi-cargo-cache.py run --target <persistent-target> -- test -p opi-ai --test bedrock_fixtures --offline --locked -- --test-threads=1; run every created or modified target/filter and affected-target clippy; final workspace/doc/whitespace union applies.

#### Fix B72.1: verify p20-mech-auth-002 finite source cases

- **Finding source(s)**: phase20-codex-gpt61-b3f1810-20261103t142820z / 4cec97af8ef6bb773c630ee251728f31212e12fdc1bf9631f10b2dc3084bc0f6 / P20-AUD-018
- **Decision**: fix:verify-p20-mech-auth-002-finite-source-cases
- **Verification status**: Partially confirmed
- **File(s)**: `crates/opi-ai/tests/bedrock_fixtures.rs`
- **Change kind**: test-only
- **Change**: Add the exact finite assertion matrix below at its current owning seam; production behavior and dependencies are unchanged by this test-only batch.
- **Closure predicate**: The finite source_generation_budgets source cases produce exactly the registered results and side-effect counts below; typed invalid-input controls fail before the stated later boundary.
- **Red-before**: Not applicable to production behavior: this batch adds the finite source-defined conformance test only. Preserve any actual baseline failure and stop for its separately bound behavioral repair; no conditional production edit is admitted.
- **Green-after**: python scripts/opi-cargo-cache.py run --target <persistent-target> -- test -p opi-ai --test bedrock_fixtures --offline --locked -- --test-threads=1 -> The finite source_generation_budgets source cases produce exactly the registered results and side-effect counts below; typed invalid-input controls fail before the stated later boundary. Observed:not-run.
- **Test impact**: add named assertions/cases; update helpers only as required; retain valid-source and negative controls; delete none.

Assigned current criterion: `P20-MECH-AUTH-002`; exact source `docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-transport-auth.md` SHA256 `42e1bc90c02e958ada4d2a01db999cd906ae33ee971496c50c07ed38067cdcfb`, citation `L101`.

Current assertion evidence and limits: bedrock4453–4510STS403typedcode/requestid,redactedbody,no processfallback;7805–7933generation3sends/oneauth/frozenpayload.8384countertestisdisconnectednotreal2layerproof.

Finite inputs and oracle: source_generation_budgets:STS503→2002exchanges thenBedrock503→2002generations independentlimits;source403→generation0/nofallback;redirectlegcounts+reqwestextraretryoff

Authority: No product-meaning decision; follow registered predicate with existing seams.

This batch owns only the named additional test portion. Referenced C017/package/native proof is a separate conjunction/barrier; it does not create a backward dependency on the final B17 runner. Existing known behavioral defects keep their named production batches. Unexpected baseline red is preserved and replanned, never silently repaired here.

### Batch B73: verify p20-mech-auth-003 finite source cases

**Closure predicate**: The finite frozen_nine_flow_dependencies source cases produce exactly the registered results and side-effect counts below; typed invalid-input controls fail before the stated later boundary.
**Dependencies**: []
**Verification union**: python scripts/opi-cargo-cache.py run --target <persistent-target> -- test -p opi-ai --test oauth_flows --offline --locked -- --test-threads=1; python scripts/opi-cargo-cache.py run --target <persistent-target> -- test -p opi-ai --test oauth_store_integration --offline --locked -- --test-threads=1; run every created or modified target/filter and affected-target clippy; final workspace/doc/whitespace union applies.

#### Fix B73.1: verify p20-mech-auth-003 finite source cases

- **Finding source(s)**: phase20-codex-gpt61-b3f1810-20261103t142820z / 4cec97af8ef6bb773c630ee251728f31212e12fdc1bf9631f10b2dc3084bc0f6 / P20-AUD-018
- **Decision**: fix:verify-p20-mech-auth-003-finite-source-cases
- **Verification status**: Partially confirmed
- **File(s)**: `crates/opi-ai/tests/oauth_store_integration.rs`, `crates/opi-ai/tests/oauth_flows.rs`
- **Change kind**: test-only
- **Change**: Add the exact finite assertion matrix below at its current owning seam; production behavior and dependencies are unchanged by this test-only batch.
- **Closure predicate**: The finite frozen_nine_flow_dependencies source cases produce exactly the registered results and side-effect counts below; typed invalid-input controls fail before the stated later boundary.
- **Red-before**: Not applicable to production behavior: this batch adds the finite source-defined conformance test only. Preserve any actual baseline failure and stop for its separately bound behavioral repair; no conditional production edit is admitted.
- **Green-after**: python scripts/opi-cargo-cache.py run --target <persistent-target> -- test -p opi-ai --test oauth_flows --offline --locked -- --test-threads=1; python scripts/opi-cargo-cache.py run --target <persistent-target> -- test -p opi-ai --test oauth_store_integration --offline --locked -- --test-threads=1 -> The finite frozen_nine_flow_dependencies source cases produce exactly the registered results and side-effect counts below; typed invalid-input controls fail before the stated later boundary. Observed:not-run.
- **Test impact**: add named assertions/cases; update helpers only as required; retain valid-source and negative controls; delete none.

Assigned current criterion: `P20-MECH-AUTH-003`; exact source `docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-transport-auth.md` SHA256 `42e1bc90c02e958ada4d2a01db999cd906ae33ee971496c50c07ed38067cdcfb`, citation `L105`.

Current assertion evidence and limits: oauth_store_integration152–194single rotation/sharedstorewaiterdrop;builtin_auth_ownershiprebindingpreservescustomowners.

Finite inputs and oracle: frozen_nine_flow_dependencies:providerrevision/presenter/clock/store/scopedenvfrozenbeforecall;replaceaftercreationchangesnewcallonly;refreshsameidentity;actualexpiryC020

Authority: No product-meaning decision; follow registered predicate with existing seams.

This batch owns only the named additional test portion. Referenced C017/package/native proof is a separate conjunction/barrier; it does not create a backward dependency on the final B17 runner. Existing known behavioral defects keep their named production batches. Unexpected baseline red is preserved and replanned, never silently repaired here.

### Batch B74: verify p20-mech-auth-007 finite source cases

**Closure predicate**: The finite aws_errorclass_table source cases produce exactly the registered results and side-effect counts below; typed invalid-input controls fail before the stated later boundary.
**Dependencies**: []
**Verification union**: python scripts/opi-cargo-cache.py run --target <persistent-target> -- test -p opi-ai --test bedrock_fixtures --offline --locked -- --test-threads=1; python scripts/opi-cargo-cache.py run --target <persistent-target> -- test -p opi-ai --test builtin_native_scoped --offline --locked -- --test-threads=1; run every created or modified target/filter and affected-target clippy; final workspace/doc/whitespace union applies.

#### Fix B74.1: verify p20-mech-auth-007 finite source cases

- **Finding source(s)**: phase20-codex-gpt61-b3f1810-20261103t142820z / 4cec97af8ef6bb773c630ee251728f31212e12fdc1bf9631f10b2dc3084bc0f6 / P20-AUD-018
- **Decision**: fix:verify-p20-mech-auth-007-finite-source-cases
- **Verification status**: Partially confirmed
- **File(s)**: `crates/opi-ai/tests/bedrock_fixtures.rs`, `crates/opi-ai/tests/builtin_native_scoped.rs`
- **Change kind**: test-only
- **Change**: Add the exact finite assertion matrix below at its current owning seam; production behavior and dependencies are unchanged by this test-only batch.
- **Closure predicate**: The finite aws_errorclass_table source cases produce exactly the registered results and side-effect counts below; typed invalid-input controls fail before the stated later boundary.
- **Red-before**: Not applicable to production behavior: this batch adds the finite source-defined conformance test only. Preserve any actual baseline failure and stop for its separately bound behavioral repair; no conditional production edit is admitted.
- **Green-after**: python scripts/opi-cargo-cache.py run --target <persistent-target> -- test -p opi-ai --test bedrock_fixtures --offline --locked -- --test-threads=1; python scripts/opi-cargo-cache.py run --target <persistent-target> -- test -p opi-ai --test builtin_native_scoped --offline --locked -- --test-threads=1 -> The finite aws_errorclass_table source cases produce exactly the registered results and side-effect counts below; typed invalid-input controls fail before the stated later boundary. Observed:not-run.
- **Test impact**: add named assertions/cases; update helpers only as required; retain valid-source and negative controls; delete none.

Assigned current criterion: `P20-MECH-AUTH-007`; exact source `docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-transport-auth.md` SHA256 `42e1bc90c02e958ada4d2a01db999cd906ae33ee971496c50c07ed38067cdcfb`, citation `L106`.

Current assertion evidence and limits: auth_contracts440constructionzeroIO;bedrock5656–5696exactprocessenv/cwd;selectedADC2035malformedfileterminal.

Finite inputs and oracle: aws_errorclass_table:completepartialenv/absentunreadablemalformedINI/cycle/processabsenceparse/service denialsourceexactContinueTerminal;regiononlyendpointnotaccount;HOMEcanaryzero constructionread

Authority: No product-meaning decision; follow registered predicate with existing seams.

This batch owns only the named additional test portion. Referenced C017/package/native proof is a separate conjunction/barrier; it does not create a backward dependency on the final B17 runner. Existing known behavioral defects keep their named production batches. Unexpected baseline red is preserved and replanned, never silently repaired here.

### Batch B75: verify p20-mech-auth-009 finite source cases

**Closure predicate**: The finite sso_login_ACK_capability source cases produce exactly the registered results and side-effect counts below; typed invalid-input controls fail before the stated later boundary.
**Dependencies**: []
**Verification union**: python scripts/opi-cargo-cache.py run --target <persistent-target> -- test -p opi-ai --test bedrock_fixtures --offline --locked -- --test-threads=1; run every created or modified target/filter and affected-target clippy; final workspace/doc/whitespace union applies.

#### Fix B75.1: verify p20-mech-auth-009 finite source cases

- **Finding source(s)**: phase20-codex-gpt61-b3f1810-20261103t142820z / 4cec97af8ef6bb773c630ee251728f31212e12fdc1bf9631f10b2dc3084bc0f6 / P20-AUD-018
- **Decision**: fix:verify-p20-mech-auth-009-finite-source-cases
- **Verification status**: Partially confirmed
- **File(s)**: `crates/opi-ai/tests/bedrock_fixtures.rs`
- **Change kind**: test-only
- **Change**: Add the exact finite assertion matrix below at its current owning seam; production behavior and dependencies are unchanged by this test-only batch.
- **Closure predicate**: The finite sso_login_ACK_capability source cases produce exactly the registered results and side-effect counts below; typed invalid-input controls fail before the stated later boundary.
- **Red-before**: Not applicable to production behavior: this batch adds the finite source-defined conformance test only. Preserve any actual baseline failure and stop for its separately bound behavioral repair; no conditional production edit is admitted.
- **Green-after**: python scripts/opi-cargo-cache.py run --target <persistent-target> -- test -p opi-ai --test bedrock_fixtures --offline --locked -- --test-threads=1 -> The finite sso_login_ACK_capability source cases produce exactly the registered results and side-effect counts below; typed invalid-input controls fail before the stated later boundary. Observed:not-run.
- **Test impact**: add named assertions/cases; update helpers only as required; retain valid-source and negative controls; delete none.

Assigned current criterion: `P20-MECH-AUTH-009`; exact source `docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-transport-auth.md` SHA256 `42e1bc90c02e958ada4d2a01db999cd906ae33ee971496c50c07ed38067cdcfb`, citation `L107`.

Current assertion evidence and limits: bedrock6521–6608SSOfailedfilesave retainsR2throughexpiry yetWrittenreceiptisownercacheACK;6612–6684loginanalog. NotdurablefileACKproof.

Finite inputs and oracle: sso_login_ACK_capability:memorycommit vsselectedpersistentfile success/failure/unknown;retainonlyrefresh,nextsameidentityusesR2;clockRNGinjected,no secrettrace

Authority: Resolved as routine owner-qualified ACK evidence: current registered pi-1.3.0-rust-backends.md178 explicitly permits memory-only ACK without file/cross-process durability claim. aws/cache.rs574–594 acknowledges in-memory value only; sso477/552 andlogin600/747best-effort filewrites match. Add explicit truthfulcapability assertion; no newpersistentfilemandate.

This batch owns only the named additional test portion. Referenced C017/package/native proof is a separate conjunction/barrier; it does not create a backward dependency on the final B17 runner. Existing known behavioral defects keep their named production batches. Unexpected baseline red is preserved and replanned, never silently repaired here.

### Batch B76: verify p20-mech-auth-010 finite source cases

**Closure predicate**: The finite aws_process_lifecycle source cases produce exactly the registered results and side-effect counts below; typed invalid-input controls fail before the stated later boundary.
**Dependencies**: []
**Verification union**: python scripts/opi-cargo-cache.py run --target <persistent-target> -- test -p opi-ai --test auth_process_lifecycle --offline --locked -- --test-threads=1; python scripts/opi-cargo-cache.py run --target <persistent-target> -- test -p opi-ai --test bedrock_fixtures --offline --locked -- --test-threads=1; run every created or modified target/filter and affected-target clippy; final workspace/doc/whitespace union applies.

#### Fix B76.1: verify p20-mech-auth-010 finite source cases

- **Finding source(s)**: phase20-codex-gpt61-b3f1810-20261103t142820z / 4cec97af8ef6bb773c630ee251728f31212e12fdc1bf9631f10b2dc3084bc0f6 / P20-AUD-018
- **Decision**: fix:verify-p20-mech-auth-010-finite-source-cases
- **Verification status**: Partially confirmed
- **File(s)**: `crates/opi-ai/tests/auth_process_lifecycle.rs`, `crates/opi-ai/tests/bedrock_fixtures.rs`
- **Change kind**: test-only
- **Change**: Add the exact finite assertion matrix below at its current owning seam; production behavior and dependencies are unchanged by this test-only batch.
- **Closure predicate**: The finite aws_process_lifecycle source cases produce exactly the registered results and side-effect counts below; typed invalid-input controls fail before the stated later boundary.
- **Red-before**: Not applicable to production behavior: this batch adds the finite source-defined conformance test only. Preserve any actual baseline failure and stop for its separately bound behavioral repair; no conditional production edit is admitted.
- **Green-after**: python scripts/opi-cargo-cache.py run --target <persistent-target> -- test -p opi-ai --test auth_process_lifecycle --offline --locked -- --test-threads=1; python scripts/opi-cargo-cache.py run --target <persistent-target> -- test -p opi-ai --test bedrock_fixtures --offline --locked -- --test-threads=1 -> The finite aws_process_lifecycle source cases produce exactly the registered results and side-effect counts below; typed invalid-input controls fail before the stated later boundary. Observed:not-run.
- **Test impact**: add named assertions/cases; update helpers only as required; retain valid-source and negative controls; delete none.

Assigned current criterion: `P20-MECH-AUTH-010`; exact source `docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-transport-auth.md` SHA256 `42e1bc90c02e958ada4d2a01db999cd906ae33ee971496c50c07ed38067cdcfb`, citation `L108`.

Current assertion evidence and limits: bedrock5656–5696snapshotonceexactenv/cwd;auth_process_lifecycle436/542/670cancelreaptree;processowner noinsertedshell.

Finite inputs and oracle: aws_process_lifecycle:quotedargv/exactenv/cwd,selectedinvalidtyped,cancelbefore/during,rootexitdescendantthenreap;readexecuteexchangecountersdistinct

Authority: No product-meaning decision; follow registered predicate with existing seams.

This batch owns only the named additional test portion. Referenced C017/package/native proof is a separate conjunction/barrier; it does not create a backward dependency on the final B17 runner. Existing known behavioral defects keep their named production batches. Unexpected baseline red is preserved and replanned, never silently repaired here.

### Batch B77: verify p20-mech-auth-011 finite source cases

**Closure predicate**: The finite web_identity_service_denial source cases produce exactly the registered results and side-effect counts below; typed invalid-input controls fail before the stated later boundary.
**Dependencies**: []
**Verification union**: python scripts/opi-cargo-cache.py run --target <persistent-target> -- test -p opi-ai --test bedrock_fixtures --offline --locked -- --test-threads=1; run every created or modified target/filter and affected-target clippy; final workspace/doc/whitespace union applies.

#### Fix B77.1: verify p20-mech-auth-011 finite source cases

- **Finding source(s)**: phase20-codex-gpt61-b3f1810-20261103t142820z / 4cec97af8ef6bb773c630ee251728f31212e12fdc1bf9631f10b2dc3084bc0f6 / P20-AUD-018
- **Decision**: fix:verify-p20-mech-auth-011-finite-source-cases
- **Verification status**: Partially confirmed
- **File(s)**: `crates/opi-ai/tests/bedrock_fixtures.rs`
- **Change kind**: test-only
- **Change**: Add the exact finite assertion matrix below at its current owning seam; production behavior and dependencies are unchanged by this test-only batch.
- **Closure predicate**: The finite web_identity_service_denial source cases produce exactly the registered results and side-effect counts below; typed invalid-input controls fail before the stated later boundary.
- **Red-before**: Not applicable to production behavior: this batch adds the finite source-defined conformance test only. Preserve any actual baseline failure and stop for its separately bound behavioral repair; no conditional production edit is admitted.
- **Green-after**: python scripts/opi-cargo-cache.py run --target <persistent-target> -- test -p opi-ai --test bedrock_fixtures --offline --locked -- --test-threads=1 -> The finite web_identity_service_denial source cases produce exactly the registered results and side-effect counts below; typed invalid-input controls fail before the stated later boundary. Observed:not-run.
- **Test impact**: add named assertions/cases; update helpers only as required; retain valid-source and negative controls; delete none.

Assigned current criterion: `P20-MECH-AUTH-011`; exact source `docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-transport-auth.md` SHA256 `42e1bc90c02e958ada4d2a01db999cd906ae33ee971496c50c07ed38067cdcfb`, citation `L108`.

Current assertion evidence and limits: bedrock4515–4586oneWebIdentityform/noSigV4/exactprovenance/accountid/noprocess;4591–4637badfileTerminal0sends.

Finite inputs and oracle: web_identity_service_denial:validfileSTS403→TerminalnotUnsupported,nootherprofile/process/metadata,generation0;missingbadtokenfilecontrols

Authority: No product-meaning decision; follow registered predicate with existing seams.

This batch owns only the named additional test portion. Referenced C017/package/native proof is a separate conjunction/barrier; it does not create a backward dependency on the final B17 runner. Existing known behavioral defects keep their named production batches. Unexpected baseline red is preserved and replanned, never silently repaired here.

### Batch B78: verify p20-mech-auth-014 finite source cases

**Closure predicate**: The finite smithy_state_trace source cases produce exactly the registered results and side-effect counts below; typed invalid-input controls fail before the stated later boundary.
**Dependencies**: []
**Verification union**: python scripts/opi-cargo-cache.py run --target <persistent-target> -- test -p opi-ai --test bedrock_fixtures --offline --locked -- --test-threads=1; run every created or modified target/filter and affected-target clippy; final workspace/doc/whitespace union applies.

#### Fix B78.1: verify p20-mech-auth-014 finite source cases

- **Finding source(s)**: phase20-codex-gpt61-b3f1810-20261103t142820z / 4cec97af8ef6bb773c630ee251728f31212e12fdc1bf9631f10b2dc3084bc0f6 / P20-AUD-018
- **Decision**: fix:verify-p20-mech-auth-014-finite-source-cases
- **Verification status**: Partially confirmed
- **File(s)**: `crates/opi-ai/tests/bedrock_fixtures.rs`
- **Change kind**: test-only
- **Change**: Add the exact finite assertion matrix below at its current owning seam; production behavior and dependencies are unchanged by this test-only batch.
- **Closure predicate**: The finite smithy_state_trace source cases produce exactly the registered results and side-effect counts below; typed invalid-input controls fail before the stated later boundary.
- **Red-before**: Not applicable to production behavior: this batch adds the finite source-defined conformance test only. Preserve any actual baseline failure and stop for its separately bound behavioral repair; no conditional production edit is admitted.
- **Green-after**: python scripts/opi-cargo-cache.py run --target <persistent-target> -- test -p opi-ai --test bedrock_fixtures --offline --locked -- --test-threads=1 -> The finite smithy_state_trace source cases produce exactly the registered results and side-effect counts below; typed invalid-input controls fail before the stated later boundary. Observed:not-run.
- **Test impact**: add named assertions/cases; update helpers only as required; retain valid-source and negative controls; delete none.

Assigned current criterion: `P20-MECH-AUTH-014`; exact source `docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-transport-auth.md` SHA256 `42e1bc90c02e958ada4d2a01db999cd906ae33ee971496c50c07ed38067cdcfb`, citation `L110`.

Current assertion evidence and limits: bedrock7805–7933realretrydelays;8176–8228adaptivewaitformula;8319–8346legacy100/202650ms+cost14;retry683–733CUBICimplementation.

Finite inputs and oracle: smithy_state_trace:seededclockRNGquotaacquire/refund/exhaust/hintbounds;legacy2026bothconstants;adaptivefirstsendwait/successincrease/throttledecrease/recovery fixedsourcevectors+actualsendcounts

Authority: No product-meaning decision; follow registered predicate with existing seams.

This batch owns only the named additional test portion. Referenced C017/package/native proof is a separate conjunction/barrier; it does not create a backward dependency on the final B17 runner. Existing known behavioral defects keep their named production batches. Unexpected baseline red is preserved and replanned, never silently repaired here.

### Batch B79: verify p20-mech-auth-015 finite source cases

**Closure predicate**: The finite smithy_owner_scope source cases produce exactly the registered results and side-effect counts below; typed invalid-input controls fail before the stated later boundary.
**Dependencies**: []
**Verification union**: python scripts/opi-cargo-cache.py run --target <persistent-target> -- test -p opi-ai --test bedrock_fixtures --offline --locked -- --test-threads=1; run every created or modified target/filter and affected-target clippy; final workspace/doc/whitespace union applies.

#### Fix B79.1: verify p20-mech-auth-015 finite source cases

- **Finding source(s)**: phase20-codex-gpt61-b3f1810-20261103t142820z / 4cec97af8ef6bb773c630ee251728f31212e12fdc1bf9631f10b2dc3084bc0f6 / P20-AUD-018
- **Decision**: fix:verify-p20-mech-auth-015-finite-source-cases
- **Verification status**: Partially confirmed
- **File(s)**: `crates/opi-ai/tests/bedrock_fixtures.rs`
- **Change kind**: test-only
- **Change**: Add the exact finite assertion matrix below at its current owning seam; production behavior and dependencies are unchanged by this test-only batch.
- **Closure predicate**: The finite smithy_owner_scope source cases produce exactly the registered results and side-effect counts below; typed invalid-input controls fail before the stated later boundary.
- **Red-before**: Not applicable to production behavior: this batch adds the finite source-defined conformance test only. Preserve any actual baseline failure and stop for its separately bound behavioral repair; no conditional production edit is admitted.
- **Green-after**: python scripts/opi-cargo-cache.py run --target <persistent-target> -- test -p opi-ai --test bedrock_fixtures --offline --locked -- --test-threads=1 -> The finite smithy_owner_scope source cases produce exactly the registered results and side-effect counts below; typed invalid-input controls fail before the stated later boundary. Observed:not-run.
- **Test impact**: add named assertions/cases; update helpers only as required; retain valid-source and negative controls; delete none.

Assigned current criterion: `P20-MECH-AUTH-015`; exact source `docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-transport-auth.md` SHA256 `42e1bc90c02e958ada4d2a01db999cd906ae33ee971496c50c07ed38067cdcfb`, citation `L110`.

Current assertion evidence and limits: bedrock8233–8277scoped2attemptsoverprofile9 with2actualrequests;8384quota/sourcecountertestnotactualacquisition.

Finite inputs and oracle: smithy_owner_scope:onecallquotaexhaustiondoesnotaffectsecondcall/instance;scopedmode/max/profileprecedence;source2exchangesindependentgeneration2attemptallowance

Authority: No product-meaning decision; follow registered predicate with existing seams.

This batch owns only the named additional test portion. Referenced C017/package/native proof is a separate conjunction/barrier; it does not create a backward dependency on the final B17 runner. Existing known behavioral defects keep their named production batches. Unexpected baseline red is preserved and replanned, never silently repaired here.

### Batch B80: verify p20-mech-auth-016 finite source cases

**Closure predicate**: The finite adc_gaxios_actualwire source cases produce exactly the registered results and side-effect counts below; typed invalid-input controls fail before the stated later boundary.
**Dependencies**: []
**Verification union**: python scripts/opi-cargo-cache.py run --target <persistent-target> -- test -p opi-ai --test adc_owner_regressions --offline --locked -- --test-threads=1; python scripts/opi-cargo-cache.py run --target <persistent-target> -- test -p opi-ai --test auth_contracts --offline --locked -- --test-threads=1; run every created or modified target/filter and affected-target clippy; final workspace/doc/whitespace union applies.

#### Fix B80.1: verify p20-mech-auth-016 finite source cases

- **Finding source(s)**: phase20-codex-gpt61-b3f1810-20261103t142820z / 4cec97af8ef6bb773c630ee251728f31212e12fdc1bf9631f10b2dc3084bc0f6 / P20-AUD-018
- **Decision**: fix:verify-p20-mech-auth-016-finite-source-cases
- **Verification status**: Partially confirmed
- **File(s)**: `crates/opi-ai/tests/auth_contracts.rs`, `crates/opi-ai/tests/adc_owner_regressions.rs`
- **Change kind**: test-only
- **Change**: Add the exact finite assertion matrix below at its current owning seam; production behavior and dependencies are unchanged by this test-only batch.
- **Closure predicate**: The finite adc_gaxios_actualwire source cases produce exactly the registered results and side-effect counts below; typed invalid-input controls fail before the stated later boundary.
- **Red-before**: Not applicable to production behavior: this batch adds the finite source-defined conformance test only. Preserve any actual baseline failure and stop for its separately bound behavioral repair; no conditional production edit is admitted.
- **Green-after**: python scripts/opi-cargo-cache.py run --target <persistent-target> -- test -p opi-ai --test adc_owner_regressions --offline --locked -- --test-threads=1; python scripts/opi-cargo-cache.py run --target <persistent-target> -- test -p opi-ai --test auth_contracts --offline --locked -- --test-threads=1 -> The finite adc_gaxios_actualwire source cases produce exactly the registered results and side-effect counts below; typed invalid-input controls fail before the stated later boundary. Observed:not-run.
- **Test impact**: add named assertions/cases; update helpers only as required; retain valid-source and negative controls; delete none.

Assigned current criterion: `P20-MECH-AUTH-016`; exact source `docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-transport-auth.md` SHA256 `42e1bc90c02e958ada4d2a01db999cd906ae33ee971496c50c07ed38067cdcfb`, citation `L111`.

Current assertion evidence and limits: auth_contracts2225–2278canonicalJWTPOSTalgonly/claims/expiry;2482–2504scripted3requests;2562–2584noresponse3attempts.

Finite inputs and oracle: adc_gaxios_actualwire:serviceaccount/authorizeduser statuses408429500599retry vs400401600no;network2extra/exchange3extra/exactdelays/terminalredirect/nohiddenreqwestextra;quotaprojectpreserved

Authority: No product-meaning decision; follow registered predicate with existing seams.

This batch owns only the named additional test portion. Referenced C017/package/native proof is a separate conjunction/barrier; it does not create a backward dependency on the final B17 runner. Existing known behavioral defects keep their named production batches. Unexpected baseline red is preserved and replanned, never silently repaired here.

### Batch B81: verify p20-mech-auth-017 finite source cases

**Closure predicate**: The finite adc_recursive_sources source cases produce exactly the registered results and side-effect counts below; typed invalid-input controls fail before the stated later boundary.
**Dependencies**: []
**Verification union**: python scripts/opi-cargo-cache.py run --target <persistent-target> -- test -p opi-ai --test adc_owner_regressions --offline --locked -- --test-threads=1; python scripts/opi-cargo-cache.py run --target <persistent-target> -- test -p opi-ai --test auth_contracts --offline --locked -- --test-threads=1; run every created or modified target/filter and affected-target clippy; final workspace/doc/whitespace union applies.

#### Fix B81.1: verify p20-mech-auth-017 finite source cases

- **Finding source(s)**: phase20-codex-gpt61-b3f1810-20261103t142820z / 4cec97af8ef6bb773c630ee251728f31212e12fdc1bf9631f10b2dc3084bc0f6 / P20-AUD-018
- **Decision**: fix:verify-p20-mech-auth-017-finite-source-cases
- **Verification status**: Partially confirmed
- **File(s)**: `crates/opi-ai/tests/auth_contracts.rs`, `crates/opi-ai/tests/adc_owner_regressions.rs`
- **Change kind**: test-only
- **Change**: Add the exact finite assertion matrix below at its current owning seam; production behavior and dependencies are unchanged by this test-only batch.
- **Closure predicate**: The finite adc_recursive_sources source cases produce exactly the registered results and side-effect counts below; typed invalid-input controls fail before the stated later boundary.
- **Red-before**: Not applicable to production behavior: this batch adds the finite source-defined conformance test only. Preserve any actual baseline failure and stop for its separately bound behavioral repair; no conditional production edit is admitted.
- **Green-after**: python scripts/opi-cargo-cache.py run --target <persistent-target> -- test -p opi-ai --test adc_owner_regressions --offline --locked -- --test-threads=1; python scripts/opi-cargo-cache.py run --target <persistent-target> -- test -p opi-ai --test auth_contracts --offline --locked -- --test-threads=1 -> The finite adc_recursive_sources source cases produce exactly the registered results and side-effect counts below; typed invalid-input controls fail before the stated later boundary. Observed:not-run.
- **Test impact**: add named assertions/cases; update helpers only as required; retain valid-source and negative controls; delete none.

Assigned current criterion: `P20-MECH-AUTH-017`; exact source `docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-transport-auth.md` SHA256 `42e1bc90c02e958ada4d2a01db999cd906ae33ee971496c50c07ed38067cdcfb`, citation `L112`.

Current assertion evidence and limits: auth_contracts4685–4755IAMurl/sourceBearer/delegates/scope/lifetime/provenance/expiry;4810–4835recursive3legs;2308Basicexternaluser.

Finite inputs and oracle: adc_recursive_sources:authorized/service/externaluser depthbeyondoldcap;targetdelegateslifetimeexact;BasicreplacementR2nextrefresh;sourceidentitynotretaggedtarget

Authority: No product-meaning decision; follow registered predicate with existing seams.

This batch owns only the named additional test portion. Referenced C017/package/native proof is a separate conjunction/barrier; it does not create a backward dependency on the final B17 runner. Existing known behavioral defects keep their named production batches. Unexpected baseline red is preserved and replanned, never silently repaired here.

### Batch B82: verify p20-mech-auth-018 finite source cases

**Closure predicate**: The finite adc_rotation_fences source cases produce exactly the registered results and side-effect counts below; typed invalid-input controls fail before the stated later boundary.
**Dependencies**: []
**Verification union**: python scripts/opi-cargo-cache.py run --target <persistent-target> -- test -p opi-ai --test adc_owner_regressions --offline --locked -- --test-threads=1; run every created or modified target/filter and affected-target clippy; final workspace/doc/whitespace union applies.

#### Fix B82.1: verify p20-mech-auth-018 finite source cases

- **Finding source(s)**: phase20-codex-gpt61-b3f1810-20261103t142820z / 4cec97af8ef6bb773c630ee251728f31212e12fdc1bf9631f10b2dc3084bc0f6 / P20-AUD-018
- **Decision**: fix:verify-p20-mech-auth-018-finite-source-cases
- **Verification status**: Partially confirmed
- **File(s)**: `crates/opi-ai/tests/adc_owner_regressions.rs`
- **Change kind**: test-only
- **Change**: Add the exact finite assertion matrix below at its current owning seam; production behavior and dependencies are unchanged by this test-only batch.
- **Closure predicate**: The finite adc_rotation_fences source cases produce exactly the registered results and side-effect counts below; typed invalid-input controls fail before the stated later boundary.
- **Red-before**: Not applicable to production behavior: this batch adds the finite source-defined conformance test only. Preserve any actual baseline failure and stop for its separately bound behavioral repair; no conditional production edit is admitted.
- **Green-after**: python scripts/opi-cargo-cache.py run --target <persistent-target> -- test -p opi-ai --test adc_owner_regressions --offline --locked -- --test-threads=1 -> The finite adc_rotation_fences source cases produce exactly the registered results and side-effect counts below; typed invalid-input controls fail before the stated later boundary. Observed:not-run.
- **Test impact**: add named assertions/cases; update helpers only as required; retain valid-source and negative controls; delete none.

Assigned current criterion: `P20-MECH-AUTH-018`; exact source `docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-transport-auth.md` SHA256 `42e1bc90c02e958ada4d2a01db999cd906ae33ee971496c50c07ed38067cdcfb`, citation `L112`.

Current assertion evidence and limits: adc_owner_regressions130/177replacementfences,226sharedbarrier;auth4685source/targetdistinct.

Finite inputs and oracle: adc_rotation_fences:waiterdrop/replace/sameidentityinvalidate preserveonlyR2,distinctsourcenorebind;defaultADCfilebytesunchanged;failedunknownACKnotpersisted;explicitstorecontrol

Authority: No product-meaning decision; follow registered predicate with existing seams.

This batch owns only the named additional test portion. Referenced C017/package/native proof is a separate conjunction/barrier; it does not create a backward dependency on the final B17 runner. Existing known behavioral defects keep their named production batches. Unexpected baseline red is preserved and replanned, never silently repaired here.

### Batch B83: verify p20-mech-auth-020 finite source cases

**Closure predicate**: The finite adc_aws1_isolation source cases produce exactly the registered results and side-effect counts below; typed invalid-input controls fail before the stated later boundary.
**Dependencies**: []
**Verification union**: python scripts/opi-cargo-cache.py run --target <persistent-target> -- test -p opi-ai --test adc_owner_regressions --offline --locked -- --test-threads=1; python scripts/opi-cargo-cache.py run --target <persistent-target> -- test -p opi-ai --test auth_contracts --offline --locked -- --test-threads=1; run every created or modified target/filter and affected-target clippy; final workspace/doc/whitespace union applies.

#### Fix B83.1: verify p20-mech-auth-020 finite source cases

- **Finding source(s)**: phase20-codex-gpt61-b3f1810-20261103t142820z / 4cec97af8ef6bb773c630ee251728f31212e12fdc1bf9631f10b2dc3084bc0f6 / P20-AUD-018
- **Decision**: fix:verify-p20-mech-auth-020-finite-source-cases
- **Verification status**: Partially confirmed
- **File(s)**: `crates/opi-ai/tests/auth_contracts.rs`, `crates/opi-ai/tests/adc_owner_regressions.rs`
- **Change kind**: test-only
- **Change**: Add the exact finite assertion matrix below at its current owning seam; production behavior and dependencies are unchanged by this test-only batch.
- **Closure predicate**: The finite adc_aws1_isolation source cases produce exactly the registered results and side-effect counts below; typed invalid-input controls fail before the stated later boundary.
- **Red-before**: Not applicable to production behavior: this batch adds the finite source-defined conformance test only. Preserve any actual baseline failure and stop for its separately bound behavioral repair; no conditional production edit is admitted.
- **Green-after**: python scripts/opi-cargo-cache.py run --target <persistent-target> -- test -p opi-ai --test adc_owner_regressions --offline --locked -- --test-threads=1; python scripts/opi-cargo-cache.py run --target <persistent-target> -- test -p opi-ai --test auth_contracts --offline --locked -- --test-threads=1 -> The finite adc_aws1_isolation source cases produce exactly the registered results and side-effect counts below; typed invalid-input controls fail before the stated later boundary. Observed:not-run.
- **Test impact**: add named assertions/cases; update helpers only as required; retain valid-source and negative controls; delete none.

Assigned current criterion: `P20-MECH-AUTH-020`; exact source `docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-transport-auth.md` SHA256 `42e1bc90c02e958ada4d2a01db999cd906ae33ee971496c50c07ed38067cdcfb`, citation `L113`.

Current assertion evidence and limits: auth_contracts3412–3471signedsubjecturl/region/credential/targetresourceunsigned;needzerosend/discoveryassertionsbeyondshape.

Finite inputs and oracle: adc_aws1_isolation:envpairregion→GoogleSTSonly;poisonBedrockprofileprocessSSO unread;IMDSexactlegs→encodedsubject,no actualAWSSTS;partialpair/missingregiontyped

Authority: No product-meaning decision; follow registered predicate with existing seams.

This batch owns only the named additional test portion. Referenced C017/package/native proof is a separate conjunction/barrier; it does not create a backward dependency on the final B17 runner. Existing known behavioral defects keep their named production batches. Unexpected baseline red is preserved and replanned, never silently repaired here.

### Batch B84: verify p20-mech-auth-021 finite source cases

**Closure predicate**: The finite adc_mtls_isolation source cases produce exactly the registered results and side-effect counts below; typed invalid-input controls fail before the stated later boundary.
**Dependencies**: []
**Verification union**: python scripts/opi-cargo-cache.py run --target <persistent-target> -- test -p opi-ai --test auth_contracts --offline --locked -- --test-threads=1; python scripts/opi-cargo-cache.py run --target <persistent-target> -- test -p opi-ai --test tls_contract --offline --locked -- --test-threads=1; run every created or modified target/filter and affected-target clippy; final workspace/doc/whitespace union applies.

#### Fix B84.1: verify p20-mech-auth-021 finite source cases

- **Finding source(s)**: phase20-codex-gpt61-b3f1810-20261103t142820z / 4cec97af8ef6bb773c630ee251728f31212e12fdc1bf9631f10b2dc3084bc0f6 / P20-AUD-018
- **Decision**: fix:verify-p20-mech-auth-021-finite-source-cases
- **Verification status**: Partially confirmed
- **File(s)**: `crates/opi-ai/tests/auth_contracts.rs`, `crates/opi-ai/tests/tls_contract.rs`
- **Change kind**: test-only
- **Change**: Add the exact finite assertion matrix below at its current owning seam; production behavior and dependencies are unchanged by this test-only batch.
- **Closure predicate**: The finite adc_mtls_isolation source cases produce exactly the registered results and side-effect counts below; typed invalid-input controls fail before the stated later boundary.
- **Red-before**: Not applicable to production behavior: this batch adds the finite source-defined conformance test only. Preserve any actual baseline failure and stop for its separately bound behavioral repair; no conditional production edit is admitted.
- **Green-after**: python scripts/opi-cargo-cache.py run --target <persistent-target> -- test -p opi-ai --test auth_contracts --offline --locked -- --test-threads=1; python scripts/opi-cargo-cache.py run --target <persistent-target> -- test -p opi-ai --test tls_contract --offline --locked -- --test-threads=1 -> The finite adc_mtls_isolation source cases produce exactly the registered results and side-effect counts below; typed invalid-input controls fail before the stated later boundary. Observed:not-run.
- **Test impact**: add named assertions/cases; update helpers only as required; retain valid-source and negative controls; delete none.

Assigned current criterion: `P20-MECH-AUTH-021`; exact source `docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-transport-auth.md` SHA256 `42e1bc90c02e958ada4d2a01db999cd906ae33ee971496c50c07ed38067cdcfb`, citation `L114`.

Current assertion evidence and limits: auth_contracts3720–3777realmtlsSTS/plaintransport0/exactleaf;3781–3808both/neitherconfigtyped.

Finite inputs and oracle: adc_mtls_isolation:matchedleafkeychainpositive,wrongkey/leaf/chainnegative;trustchainnotserverroot;subsequentgenerationserverseesno clientcert;plainADCcontrolunaffected

Authority: No product-meaning decision; follow registered predicate with existing seams.

This batch owns only the named additional test portion. Referenced C017/package/native proof is a separate conjunction/barrier; it does not create a backward dependency on the final B17 runner. Existing known behavioral defects keep their named production batches. Unexpected baseline red is preserved and replanned, never silently repaired here.

### Batch B85: verify p20-mech-auth-022 finite source cases

**Closure predicate**: The finite executable_cache_table source cases produce exactly the registered results and side-effect counts below; typed invalid-input controls fail before the stated later boundary.
**Dependencies**: []
**Verification union**: python scripts/opi-cargo-cache.py run --target <persistent-target> -- test -p opi-ai --test auth_contracts --offline --locked -- --test-threads=1; python scripts/opi-cargo-cache.py run --target <persistent-target> -- test -p opi-ai --test auth_process_lifecycle --offline --locked -- --test-threads=1; run every created or modified target/filter and affected-target clippy; final workspace/doc/whitespace union applies.

#### Fix B85.1: verify p20-mech-auth-022 finite source cases

- **Finding source(s)**: phase20-codex-gpt61-b3f1810-20261103t142820z / 4cec97af8ef6bb773c630ee251728f31212e12fdc1bf9631f10b2dc3084bc0f6 / P20-AUD-018
- **Decision**: fix:verify-p20-mech-auth-022-finite-source-cases
- **Verification status**: Partially confirmed
- **File(s)**: `crates/opi-ai/tests/auth_contracts.rs`, `crates/opi-ai/tests/auth_process_lifecycle.rs`
- **Change kind**: test-only
- **Change**: Add the exact finite assertion matrix below at its current owning seam; production behavior and dependencies are unchanged by this test-only batch.
- **Closure predicate**: The finite executable_cache_table source cases produce exactly the registered results and side-effect counts below; typed invalid-input controls fail before the stated later boundary.
- **Red-before**: Not applicable to production behavior: this batch adds the finite source-defined conformance test only. Preserve any actual baseline failure and stop for its separately bound behavioral repair; no conditional production edit is admitted.
- **Green-after**: python scripts/opi-cargo-cache.py run --target <persistent-target> -- test -p opi-ai --test auth_contracts --offline --locked -- --test-threads=1; python scripts/opi-cargo-cache.py run --target <persistent-target> -- test -p opi-ai --test auth_process_lifecycle --offline --locked -- --test-threads=1 -> The finite executable_cache_table source cases produce exactly the registered results and side-effect counts below; typed invalid-input controls fail before the stated later boundary. Observed:not-run.
- **Test impact**: add named assertions/cases; update helpers only as required; retain valid-source and negative controls; delete none.

Assigned current criterion: `P20-MECH-AUTH-022`; exact source `docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-transport-auth.md` SHA256 `42e1bc90c02e958ada4d2a01db999cd906ae33ee971496c50c07ed38067cdcfb`, citation `L115`.

Current assertion evidence and limits: auth3998quotedargv;4017gatebeforecache/spawn;4049freshcachezero;4147malformedterminal;4171version/type/expiry.

Finite inputs and oracle: executable_cache_table:gateoffvalidcache0spawn;fresh0,stale1,malformed0;version/token_type/expirytyped;argvexact

Authority: No product-meaning decision; follow registered predicate with existing seams.

This batch owns only the named additional test portion. Referenced C017/package/native proof is a separate conjunction/barrier; it does not create a backward dependency on the final B17 runner. Existing known behavioral defects keep their named production batches. Unexpected baseline red is preserved and replanned, never silently repaired here.

### Batch B86: verify p20-mech-auth-023 finite source cases

**Closure predicate**: The finite executable_output_deadline source cases produce exactly the registered results and side-effect counts below; typed invalid-input controls fail before the stated later boundary.
**Dependencies**: []
**Verification union**: python scripts/opi-cargo-cache.py run --target <persistent-target> -- test -p opi-ai --test auth_contracts --offline --locked -- --test-threads=1; python scripts/opi-cargo-cache.py run --target <persistent-target> -- test -p opi-ai --test auth_process_lifecycle --offline --locked -- --test-threads=1; run every created or modified target/filter and affected-target clippy; final workspace/doc/whitespace union applies.

#### Fix B86.1: verify p20-mech-auth-023 finite source cases

- **Finding source(s)**: phase20-codex-gpt61-b3f1810-20261103t142820z / 4cec97af8ef6bb773c630ee251728f31212e12fdc1bf9631f10b2dc3084bc0f6 / P20-AUD-018
- **Decision**: fix:verify-p20-mech-auth-023-finite-source-cases
- **Verification status**: Partially confirmed
- **File(s)**: `crates/opi-ai/tests/auth_process_lifecycle.rs`, `crates/opi-ai/tests/auth_contracts.rs`
- **Change kind**: test-only
- **Change**: Add the exact finite assertion matrix below at its current owning seam; production behavior and dependencies are unchanged by this test-only batch.
- **Closure predicate**: The finite executable_output_deadline source cases produce exactly the registered results and side-effect counts below; typed invalid-input controls fail before the stated later boundary.
- **Red-before**: Not applicable to production behavior: this batch adds the finite source-defined conformance test only. Preserve any actual baseline failure and stop for its separately bound behavioral repair; no conditional production edit is admitted.
- **Green-after**: python scripts/opi-cargo-cache.py run --target <persistent-target> -- test -p opi-ai --test auth_contracts --offline --locked -- --test-threads=1; python scripts/opi-cargo-cache.py run --target <persistent-target> -- test -p opi-ai --test auth_process_lifecycle --offline --locked -- --test-threads=1 -> The finite executable_output_deadline source cases produce exactly the registered results and side-effect counts below; typed invalid-input controls fail before the stated later boundary. Observed:not-run.
- **Test impact**: add named assertions/cases; update helpers only as required; retain valid-source and negative controls; delete none.

Assigned current criterion: `P20-MECH-AUTH-023`; exact source `docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-transport-auth.md` SHA256 `42e1bc90c02e958ada4d2a01db999cd906ae33ee971496c50c07ed38067cdcfb`, citation `L115`.

Current assertion evidence and limits: executable242–257combinedpipe/exactenv;270–307unboundedread;309–314killreapjoin;process_lifecycle311largstderr/436542descendants.

Finite inputs and oracle: executable_output_deadline:combinedstdoutstderrvalidJSON,>64KiB,default30s/allowed5and120/outsideinvalid;output_fileunchanged;cancel/timeouttree nativecases

Authority: No product-meaning decision; follow registered predicate with existing seams.

This batch owns only the named additional test portion. Referenced C017/package/native proof is a separate conjunction/barrier; it does not create a backward dependency on the final B17 runner. Existing known behavioral defects keep their named production batches. Unexpected baseline red is preserved and replanned, never silently repaired here.

### Batch B87: verify p20-mech-auth-024 finite source cases

**Closure predicate**: The finite factory_no_metadata_discovery source cases produce exactly the registered results and side-effect counts below; typed invalid-input controls fail before the stated later boundary.
**Dependencies**: []
**Verification union**: python scripts/opi-cargo-cache.py run --target <persistent-target> -- test -p opi-ai --test adc_owner_regressions --offline --locked -- --test-threads=1; python scripts/opi-cargo-cache.py run --target <persistent-target> -- test -p opi-ai --test builtin_native_scoped --offline --locked -- --test-threads=1; run every created or modified target/filter and affected-target clippy; final workspace/doc/whitespace union applies.

#### Fix B87.1: verify p20-mech-auth-024 finite source cases

- **Finding source(s)**: phase20-codex-gpt61-b3f1810-20261103t142820z / 4cec97af8ef6bb773c630ee251728f31212e12fdc1bf9631f10b2dc3084bc0f6 / P20-AUD-018
- **Decision**: fix:verify-p20-mech-auth-024-finite-source-cases
- **Verification status**: Partially confirmed
- **File(s)**: `crates/opi-ai/tests/adc_owner_regressions.rs`, `crates/opi-ai/tests/builtin_native_scoped.rs`
- **Change kind**: test-only
- **Change**: Add the exact finite assertion matrix below at its current owning seam; production behavior and dependencies are unchanged by this test-only batch.
- **Closure predicate**: The finite factory_no_metadata_discovery source cases produce exactly the registered results and side-effect counts below; typed invalid-input controls fail before the stated later boundary.
- **Red-before**: Not applicable to production behavior: this batch adds the finite source-defined conformance test only. Preserve any actual baseline failure and stop for its separately bound behavioral repair; no conditional production edit is admitted.
- **Green-after**: python scripts/opi-cargo-cache.py run --target <persistent-target> -- test -p opi-ai --test adc_owner_regressions --offline --locked -- --test-threads=1; python scripts/opi-cargo-cache.py run --target <persistent-target> -- test -p opi-ai --test builtin_native_scoped --offline --locked -- --test-threads=1 -> The finite factory_no_metadata_discovery source cases produce exactly the registered results and side-effect counts below; typed invalid-input controls fail before the stated later boundary. Observed:not-run.
- **Test impact**: add named assertions/cases; update helpers only as required; retain valid-source and negative controls; delete none.

Assigned current criterion: `P20-MECH-AUTH-024`; exact source `docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-transport-auth.md` SHA256 `42e1bc90c02e958ada4d2a01db999cd906ae33ee971496c50c07ed38067cdcfb`, citation `L116,L126`.

Current assertion evidence and limits: auth2016nofilenoconfig/nosends;2035selectedbadfileterminal;adc_owner402/428/459presenceandprobefailures.

Finite inputs and oracle: factory_no_metadata_discovery:Modelsnew/register/available/checkmissingfiles zero metadataHTTP/BIOS/health/gcloud/process evenwithcanaries;explicitdirectoneleg;projectlocationmissingpreHTTP

Authority: No product-meaning decision; follow registered predicate with existing seams.

This batch owns only the named additional test portion. Referenced C017/package/native proof is a separate conjunction/barrier; it does not create a backward dependency on the final B17 runner. Existing known behavioral defects keep their named production batches. Unexpected baseline red is preserved and replanned, never silently repaired here.

### Batch B88: verify p20-mech-auth-025 finite source cases

**Closure predicate**: The finite residency_scope source cases produce exactly the registered results and side-effect counts below; typed invalid-input controls fail before the stated later boundary.
**Dependencies**: []
**Verification union**: python scripts/opi-cargo-cache.py run --target <persistent-target> -- test -p opi-ai --test adc_owner_regressions --offline --locked -- --test-threads=1; python scripts/opi-cargo-cache.py run --target <persistent-target> -- test -p opi-ai --test auth_contracts --offline --locked -- --test-threads=1; run every created or modified target/filter and affected-target clippy; final workspace/doc/whitespace union applies.

#### Fix B88.1: verify p20-mech-auth-025 finite source cases

- **Finding source(s)**: phase20-codex-gpt61-b3f1810-20261103t142820z / 4cec97af8ef6bb773c630ee251728f31212e12fdc1bf9631f10b2dc3084bc0f6 / P20-AUD-018
- **Decision**: fix:verify-p20-mech-auth-025-finite-source-cases
- **Verification status**: Partially confirmed
- **File(s)**: `crates/opi-ai/tests/auth_contracts.rs`, `crates/opi-ai/tests/adc_owner_regressions.rs`
- **Change kind**: test-only
- **Change**: Add the exact finite assertion matrix below at its current owning seam; production behavior and dependencies are unchanged by this test-only batch.
- **Closure predicate**: The finite residency_scope source cases produce exactly the registered results and side-effect counts below; typed invalid-input controls fail before the stated later boundary.
- **Red-before**: Not applicable to production behavior: this batch adds the finite source-defined conformance test only. Preserve any actual baseline failure and stop for its separately bound behavioral repair; no conditional production edit is admitted.
- **Green-after**: python scripts/opi-cargo-cache.py run --target <persistent-target> -- test -p opi-ai --test adc_owner_regressions --offline --locked -- --test-threads=1; python scripts/opi-cargo-cache.py run --target <persistent-target> -- test -p opi-ai --test auth_contracts --offline --locked -- --test-threads=1 -> The finite residency_scope source cases produce exactly the registered results and side-effect counts below; typed invalid-input controls fail before the stated later boundary. Observed:not-run.
- **Test impact**: add named assertions/cases; update helpers only as required; retain valid-source and negative controls; delete none.

Assigned current criterion: `P20-MECH-AUTH-025`; exact source `docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-transport-auth.md` SHA256 `42e1bc90c02e958ada4d2a01db999cd906ae33ee971496c50c07ed38067cdcfb`, citation `L122,L126`.

Current assertion evidence and limits: metadata50–92explicitresidencytrueNone/other3000;auth2612–2651assert3000/2656–2673None.

Finite inputs and oracle: residency_scope:Nonefalse true→3000/3000/None;hostobservationsonlyselectedpathfrozenonce;trueallows4sresponse;otherADCtimeoutnotchanged

Authority: No product-meaning decision; follow registered predicate with existing seams.

This batch owns only the named additional test portion. Referenced C017/package/native proof is a separate conjunction/barrier; it does not create a backward dependency on the final B17 runner. Existing known behavioral defects keep their named production batches. Unexpected baseline red is preserved and replanned, never silently repaired here.

### Batch B89: verify p20-mech-auth-026 finite source cases

**Closure predicate**: The finite metadata_send_matrix source cases produce exactly the registered results and side-effect counts below; typed invalid-input controls fail before the stated later boundary.
**Dependencies**: []
**Verification union**: python scripts/opi-cargo-cache.py run --target <persistent-target> -- test -p opi-ai --test adc_owner_regressions --offline --locked -- --test-threads=1; python scripts/opi-cargo-cache.py run --target <persistent-target> -- test -p opi-ai --test auth_contracts --offline --locked -- --test-threads=1; run every created or modified target/filter and affected-target clippy; final workspace/doc/whitespace union applies.

#### Fix B89.1: verify p20-mech-auth-026 finite source cases

- **Finding source(s)**: phase20-codex-gpt61-b3f1810-20261103t142820z / 4cec97af8ef6bb773c630ee251728f31212e12fdc1bf9631f10b2dc3084bc0f6 / P20-AUD-018
- **Decision**: fix:verify-p20-mech-auth-026-finite-source-cases
- **Verification status**: Partially confirmed
- **File(s)**: `crates/opi-ai/tests/auth_contracts.rs`, `crates/opi-ai/tests/adc_owner_regressions.rs`
- **Change kind**: test-only
- **Change**: Add the exact finite assertion matrix below at its current owning seam; production behavior and dependencies are unchanged by this test-only batch.
- **Closure predicate**: The finite metadata_send_matrix source cases produce exactly the registered results and side-effect counts below; typed invalid-input controls fail before the stated later boundary.
- **Red-before**: Not applicable to production behavior: this batch adds the finite source-defined conformance test only. Preserve any actual baseline failure and stop for its separately bound behavioral repair; no conditional production edit is admitted.
- **Green-after**: python scripts/opi-cargo-cache.py run --target <persistent-target> -- test -p opi-ai --test adc_owner_regressions --offline --locked -- --test-threads=1; python scripts/opi-cargo-cache.py run --target <persistent-target> -- test -p opi-ai --test auth_contracts --offline --locked -- --test-threads=1 -> The finite metadata_send_matrix source cases produce exactly the registered results and side-effect counts below; typed invalid-input controls fail before the stated later boundary. Observed:not-run.
- **Test impact**: add named assertions/cases; update helpers only as required; retain valid-source and negative controls; delete none.

Assigned current criterion: `P20-MECH-AUTH-026`; exact source `docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-transport-auth.md` SHA256 `42e1bc90c02e958ada4d2a01db999cd906ae33ee971496c50c07ed38067cdcfb`, citation `L124,L126`.

Current assertion evidence and limits: auth2612–2710Flavor/scopes/expiry/wrongFlavor/timeoutattempt1+sends1;metadata40–45distinct3/3budget.

Finite inputs and oracle: metadata_send_matrix:absentwrongflavor/invalidJSON/checkedexpiry/status/noresponse/cancelactualcounts;nonresident4sone-sendtimeout;Modelsavailablezero-discovery distinctfromexplicitdirecttoken;no globaldeadline

Authority: No product-meaning decision; follow registered predicate with existing seams.

This batch owns only the named additional test portion. Referenced C017/package/native proof is a separate conjunction/barrier; it does not create a backward dependency on the final B17 runner. Existing known behavioral defects keep their named production batches. Unexpected baseline red is preserved and replanned, never silently repaired here.

### Batch B90: bind frozen source exports to the existing cache owner

**Closure predicate**: The three existing package runners accept an explicit original cache-owner root solely for marker validation while packaging only the bound frozen source export, and reject wrong owner, source substitution and overlapping/reused scratch before Cargo.
**Dependencies**: []
**Verification union**: python -m unittest discover -s scripts/pi-sdk-conformance -p test_*.py -v; real CLI missing/wrong owner,overlap,reused scratch negatives with zero Cargo; direct real-filesystem positive admission against the sealed export and existing marked target; downstream actual B91/B17 package runs are integration coverage and are not a prerequisite for B90 closure

#### Fix B90.1: bind frozen source exports to the existing cache owner

- **Finding source(s)**: phase20-codex-gpt61-b3f1810-20261103t142820z / 4cec97af8ef6bb773c630ee251728f31212e12fdc1bf9631f10b2dc3084bc0f6 / P20-AUD-018; phase20-pi-glm53-44374ec-20261103t160703z / f8c06b50b3c51c95ac5b1309e72a8b31d06c34785ad588ed4e42bff19d914990 / P20-AUD-007
- **Decision**: fix:bind-frozen-source-exports-to-the-existing-cache-owner
- **Verification status**: Partially confirmed
- **File(s)**: `scripts/pi-sdk-conformance/run_early_package_proof.py`, `scripts/pi-sdk-conformance/package_slice.py`, `scripts/pi-sdk-conformance/run_readme_examples.py`, `scripts/pi-sdk-conformance/test_package_proof.py`, `scripts/pi-sdk-conformance/test_package_slice.py`, `scripts/pi-sdk-conformance/test_readme_examples.py`
- **Change kind**: test-only
- **Change**: Add the shared optional --cache-workspace input and synchronize the three existing runner entry points. Existing omitted-option behavior remains exact. Validate scratch outside source export,owner and target; bind both identities; never rewrite cache markers or substitute the live checkout as packaging cwd.
- **Closure predicate**: The three existing package runners accept an explicit original cache-owner root solely for marker validation while packaging only the bound frozen source export, and reject wrong owner, source substitution and overlapping/reused scratch before Cargo.
- **Red-before**: Not applicable to production behavior: one bounded harness/proof-collection root. Current source inventory establishes the missing exact route/admission; synthetic unit/subprocess negatives prove guard behavior only and are never labeled native/product acceptance.
- **Green-after**: python -m unittest discover -s scripts/pi-sdk-conformance -p test_*.py -v; real CLI missing/wrong owner,overlap,reused scratch negatives with zero Cargo; direct real-filesystem positive admission against the sealed export and existing marked target; downstream actual B91/B17 package runs are integration coverage and are not a prerequisite for B90 closure -> The three existing package runners accept an explicit original cache-owner root solely for marker validation while packaging only the bound frozen source export, and reject wrong owner, source substitution and overlapping/reused scratch before Cargo. Observed:not-run.
- **Test impact**: add/update exact harness guards and finite execution cases; retain controls; delete none.

B90 closes solely after shared guard and all three existing entry-point admission tests pass, including wrong owner/source/scratch zero-Cargo negatives and valid real-filesystem binding. B90 does not edit not-yet-created B17 files, require B91/B17 runtime evidence, or claim product/package acceptance. B17 owns its new --cache-workspace caller wiring and tests after B90 supplies the shared interface.

### Batch B94: verify catalog protocol version admission

**Closure predicate**: Distribution schema v1 and hydrated catalog data schema v6 remain distinct; required index/minimumPiVersion and explicit compatibility version are validated before publication, and malformed or unknown required versions preserve the previous revision without auth or sends.
**Dependencies**: []
**Verification union**: cargo test -p opi-ai --test catalog_artifact --test catalog_operations --offline --locked; affected-target clippy; documentation and whitespace checks.

#### Fix B94.1: verify catalog protocol version admission

- **Finding source(s)**: phase20-codex-gpt61-b3f1810-20261103t142820z / 4cec97af8ef6bb773c630ee251728f31212e12fdc1bf9631f10b2dc3084bc0f6 / P20-AUD-018
- **Decision**: fix:add-catalog-protocol-version-admission-controls
- **Verification status**: Partially confirmed
- **File(s)**: `crates/opi-ai/tests/catalog_artifact.rs`, `crates/opi-ai/tests/catalog_operations.rs`
- **Change kind**: test-only
- **Change**: Add the protocol-version portion of current P20-V-56 independently of authoritative empty-catalog lifecycle tests.
- **Closure predicate**: Distribution schema v1 and hydrated catalog data schema v6 remain distinct; required index/minimumPiVersion and explicit compatibility version are validated before publication, and malformed or unknown required versions preserve the previous revision without auth or sends.
- **Red-before**: Not applicable to production behavior: finite test-only admission assertions. Unexpected genuine baseline failures stop for separately bound behavioral repair.
- **Green-after**: cargo test -p opi-ai --test catalog_artifact --test catalog_operations --offline --locked -> version/schema domains remain distinct, valid controls publish and malformed/unknown-required inputs preserve prior revision with zero auth/send. Observed:not-run.
- **Test impact**: add focused valid/rejection/publication controls; retain authentic bundle and lifecycle tests; delete none.

Exact source tuple is the current P20-V-56 acceptance-matrix citation already retained in the156-row mapping. Inputs include valid distribution1 plus hydration6, valid index/minimumPiVersion/compat version, wrong domain substitution, missing required version, malformed version type and unknown required version. Record source compatibility version separately from the Rust User-Agent. No default network fetch/hydration or new compatibility policy is introduced. B48 owns only catalog empty lifecycle; this batch owns only protocol-version admission.

### Batch B18: translate below retry cost boundary

**Closure predicate**: Both line334 contract rows prohibit retry whenever capacity is below the applicable 5/10 cost, including positive insufficient values; all remaining row semantics unchanged.
**Dependencies**: []
**Verification union**: python scripts/opi-doc-check.py; git diff --check

#### Fix B18.1: translate below retry cost boundary

- **Finding source(s)**: phase20-codex-gpt61-b3f1810-20261103t142820z / 4cec97af8ef6bb773c630ee251728f31212e12fdc1bf9631f10b2dc3084bc0f6 / P20-AUD-021
- **Decision**: fix:translate-below-retry-cost-boundary
- **Verification status**: Confirmed
- **File(s)**: `docs/research/pi-ai-opi-ai-alignment/spec-draft.zh.md`
- **Change kind**: documentation
- **Change**: At spec-draft.zh.md:334 replace capacity 为零禁止 retry with capacity 低于所需 retry cost 时禁止 retry. English already states the target precisely and needs no change.
- **Closure predicate**: Both line334 contract rows prohibit retry whenever capacity is below the applicable 5/10 cost, including positive insufficient values; all remaining row semantics unchanged.
- **Red-before**: Documentation-only translation completeness correction demonstrated by paired current source rows; no runtime behavior changes.
- **Green-after**: python scripts/opi-doc-check.py; git diff --check -> Both line334 contract rows prohibit retry whenever capacity is below the applicable 5/10 cost, including positive insufficient values; all remaining row semantics unchanged.
- **Test impact**: none

English line334 already states the exact below-cost boundary, so only the Chinese sentence needs correction. This is translation equivalence, not a new quota policy; no runtime test is added. D8 retains the registered-source hash reconciliation consequence.

### Batch B20: retain typed prompt cache duration metadata

**Closure predicate**: Source promptCache optional short/long positive finite Number seconds survive import, complete owned ModelInfo and public ModelsStore write/reopen without integer/fraction or absent-tier loss; metadata creates no warm/resize/dispatch policy.
**Dependencies**: []
**Verification union**: cargo test -p opi-ai --test model_wire_metadata --offline --locked; cargo test -p opi-ai --test catalog_operations --offline --locked; cargo clippy -p opi-ai --all-targets --offline --locked -- -D warnings

#### Fix B20.1: retain typed prompt cache duration metadata

- **Finding source(s)**: phase20-codex-gpt61-b3f1810-20261103t142820z / 4cec97af8ef6bb773c630ee251728f31212e12fdc1bf9631f10b2dc3084bc0f6 / P20-AUD-019
- **Decision**: fix:retain-typed-prompt-cache-duration-metadata
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-ai/src/model_info.rs`, `crates/opi-ai/src/catalog/wire.rs`, `crates/opi-ai/src/lib.rs`, `crates/opi-ai/tests/model_wire_metadata.rs`, `crates/opi-ai/tests/catalog_operations.rs`, `crates/opi-ai/README.md`, `crates/opi-ai/README.zh.md`, `CHANGELOG.md`
- **Change kind**: behavioral
- **Change**: Define ModelPromptCache { pub short: Option<serde_json::Number>, pub long: Option<serde_json::Number> } and ModelInfo.prompt_cache: Option<ModelPromptCache>. Constructor defaults None; decode/encode promptCache in catalog wire, validate finite positive seconds without rounding or implicit policy. Preserve absent object versus present object with absent tiers, integer/fraction Number representation. Export beside complete model metadata; synchronize both READMEs and additive Unreleased.
- **Closure predicate**: Source promptCache optional short/long positive finite Number seconds survive import, complete owned ModelInfo and public ModelsStore write/reopen without integer/fraction or absent-tier loss; metadata creates no warm/resize/dispatch policy.
- **Red-before**: python scripts/opi-cargo-cache.py run --target E:\opi\cargo-targets\opi-72fa37a0fa044958 -- test --manifest-path C:\Users\Luiz\AppData\Local\Temp\opi-phase20-remediate-plan-jqdki4yk\sdk-compat-probes\Cargo.toml --offline --locked --lib -- --nocapture --test-threads=1 -> FAIL: public store round trip omits promptCache; missing promptCache control passes. Final combined compatibility suite15executed,4controls pass/11behavioral failures; receipt 358c56cbfe385ef9e29c5d6c84bbc3e32d3c8fdf7e5d6174ab0381a6b3a8f5be
- **Green-after**: cargo test -p opi-ai --test model_wire_metadata --offline --locked; cargo test -p opi-ai --test catalog_operations --offline --locked; cargo clippy -p opi-ai --all-targets --offline --locked -- -D warnings -> PASS; all original distinguishing assertions and controls must execute.
- **Test impact**: add/update; retain wire/safety/legacy-reader controls; delete only made-obsolete sentinel assertions after replacement proof.

Existing SDK annex159/163 already mandates this metadata meaning; concrete Rust shape follows the existing numeric metadata convention. Add source/store complete-round-trip controls for integer/fraction, absent object/tier, and invalid zero/negative/non-number fields. English/Chinese contract says optional seconds and unknown missing tier, with no automatic cache policy.

### Batch B48: verify Radius empty refresh persistence

**Closure predicate**: The finite radius_empty_reopen source cases produce exactly the registered results and side-effect counts below; typed invalid-input controls fail before the stated later boundary.
**Dependencies**: ["B22", "B23", "B24", "B25", "B26", "B27", "B32", "B20"]
**Verification union**: python scripts/opi-cargo-cache.py run --target <persistent-target> -- test -p opi-ai --test catalog_artifact --offline --locked -- --test-threads=1; python scripts/opi-cargo-cache.py run --target <persistent-target> -- test -p opi-ai --test catalog_lifecycle --offline --locked -- --test-threads=1; run every created or modified target/filter and affected-target clippy; final workspace/doc/whitespace union applies.

#### Fix B48.1: verify Radius empty refresh persistence

- **Finding source(s)**: phase20-codex-gpt61-b3f1810-20261103t142820z / 4cec97af8ef6bb773c630ee251728f31212e12fdc1bf9631f10b2dc3084bc0f6 / P20-AUD-018
- **Decision**: fix:verify-p20-v-56-finite-source-cases
- **Verification status**: Partially confirmed
- **File(s)**: `crates/opi-ai/tests/catalog_lifecycle.rs`
- **Change kind**: test-only
- **Change**: Add the exact finite assertion matrix below at its current owning seam; production behavior and dependencies are unchanged by this test-only batch.
- **Closure predicate**: The finite radius_empty_reopen source cases produce exactly the registered results and side-effect counts below; typed invalid-input controls fail before the stated later boundary.
- **Red-before**: Not applicable to production behavior: this batch adds the finite source-defined conformance test only. Preserve any actual baseline failure and stop for its separately bound behavioral repair; no conditional production edit is admitted.
- **Green-after**: python scripts/opi-cargo-cache.py run --target <persistent-target> -- test -p opi-ai --test catalog_artifact --offline --locked -- --test-threads=1; python scripts/opi-cargo-cache.py run --target <persistent-target> -- test -p opi-ai --test catalog_lifecycle --offline --locked -- --test-threads=1 -> The finite radius_empty_reopen source cases produce exactly the registered results and side-effect counts below; typed invalid-input controls fail before the stated later boundary. Observed:not-run.
- **Test impact**: add named assertions/cases; update helpers only as required; retain valid-source and negative controls; delete none.

Assigned current criterion: `P20-V-56`; exact source `docs/research/pi-ai-opi-ai-alignment/acceptance-matrix.md` SHA256 `c831586339910f9496a64e6b3c9390a0b7a2a366d37230222874a5f399099a5d`, citation `V-56, line 219`.

Current assertion evidence and limits: catalog_lifecycle106–146 persisted tombstone preventsstalecommit/reopen;151–233 metadata+corruptfield; catalog_personalization67–200 chat-only/accountseparation/no-key/401403404 preserve; catalog_operations224–399 mixed3operation store/reopen→wire.

Finite inputs and oracle: radius_empty_reopen: populated→empty authoritative→failure→reopen no resurrection.

Authority: No product-meaning decision; use existing public seam and registered predicate.

This batch owns only the named additional test portion. Referenced C017/package/native proof is a separate conjunction/barrier; it does not create a backward dependency on the final B17 runner. Existing known behavioral defects keep their named production batches. Unexpected baseline red is preserved and replanned, never silently repaired here.

Only the authoritative populated→empty→failed refresh→reopen no-resurrection predicate belongs here. Protocol-version admission is the independent B94 root.

### Batch B49: verify p20-sdk-core-027 finite source cases

**Closure predicate**: The finite owned_model_full_roundtrip source cases produce exactly the registered results and side-effect counts below; typed invalid-input controls fail before the stated later boundary.
**Dependencies**: ["B22", "B23", "B24", "B25", "B26", "B27", "B30", "B20"]
**Verification union**: python scripts/opi-cargo-cache.py run --target <persistent-target> -- test -p opi-ai --test catalog_operations --offline --locked -- --test-threads=1; python scripts/opi-cargo-cache.py run --target <persistent-target> -- test -p opi-ai --test model_wire_metadata --offline --locked -- --test-threads=1; run every created or modified target/filter and affected-target clippy; final workspace/doc/whitespace union applies.

#### Fix B49.1: verify p20-sdk-core-027 finite source cases

- **Finding source(s)**: phase20-codex-gpt61-b3f1810-20261103t142820z / 4cec97af8ef6bb773c630ee251728f31212e12fdc1bf9631f10b2dc3084bc0f6 / P20-AUD-018
- **Decision**: fix:verify-p20-sdk-core-027-finite-source-cases
- **Verification status**: Partially confirmed
- **File(s)**: `crates/opi-ai/tests/catalog_operations.rs`, `crates/opi-ai/tests/model_wire_metadata.rs`
- **Change kind**: test-only
- **Change**: Add the exact finite assertion matrix below at its current owning seam; production behavior and dependencies are unchanged by this test-only batch.
- **Closure predicate**: The finite owned_model_full_roundtrip source cases produce exactly the registered results and side-effect counts below; typed invalid-input controls fail before the stated later boundary.
- **Red-before**: Not applicable to production behavior: this batch adds the finite source-defined conformance test only. Preserve any actual baseline failure and stop for its separately bound behavioral repair; no conditional production edit is admitted.
- **Green-after**: python scripts/opi-cargo-cache.py run --target <persistent-target> -- test -p opi-ai --test catalog_operations --offline --locked -- --test-threads=1; python scripts/opi-cargo-cache.py run --target <persistent-target> -- test -p opi-ai --test model_wire_metadata --offline --locked -- --test-threads=1 -> The finite owned_model_full_roundtrip source cases produce exactly the registered results and side-effect counts below; typed invalid-input controls fail before the stated later boundary. Observed:not-run.
- **Test impact**: add named assertions/cases; update helpers only as required; retain valid-source and negative controls; delete none.

Assigned current criterion: `P20-SDK-CORE-027`; exact source `docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md` SHA256 `9e3d6ef690c3a794bc9c47ddc2dce04840f98d8881649be1ab2e8db0df287fd5`, citation `L159`.

Current assertion evidence and limits: catalog_operations224–399 preserves3operation input/inputLimits/output/pricing/headers/sampling/routing throughstore/reopen/actualwire;652–672 unknownimagecost andtierorder. C019/C009 currentred show missingpromptCache/fallbackprices.

Finite inputs and oracle: owned_model_full_roundtrip: onecompletechat,image,classifier with everydeclaredmetadata field; absent/empty/null controls and actualwire metadata use.

Authority: No product-meaning decision; use existing public seam and registered predicate.

This batch owns only the named additional test portion. Referenced C017/package/native proof is a separate conjunction/barrier; it does not create a backward dependency on the final B17 runner. Existing known behavioral defects keep their named production batches. Unexpected baseline red is preserved and replanned, never silently repaired here.

### Batch B21: represent oauth permanence with typed expiry

**Closure predicate**: OpenRouter public Models login/store ACK/keychain round trip and exact legacy-v1 sentinel decode expose NonExpiring with no invented timestamp or extra refresh network. Unknown remains refresh-needed; At retains actual service expiry and one existing skew; other providers/timestamps never become permanent.
**Dependencies**: []
**Verification union**: cargo test --workspace --all-targets --offline --locked; cargo test --workspace --doc --offline --locked; cargo clippy --workspace --all-targets --offline --locked -- -D warnings; cargo doc --workspace --no-deps --offline --locked (RUSTDOCFLAGS=-D warnings); python scripts/opi-doc-check.py; git diff --check

#### Fix B21.1: represent oauth permanence with typed expiry

- **Finding source(s)**: phase20-codex-gpt61-b3f1810-20261103t142820z / 4cec97af8ef6bb773c630ee251728f31212e12fdc1bf9631f10b2dc3084bc0f6 / P20-AUD-020
- **Decision**: fix:represent-oauth-permanence-with-typed-expiry
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-ai/src/credential.rs`, `crates/opi-ai/src/auth.rs`, `crates/opi-ai/src/auth/store_oauth.rs`, `crates/opi-ai/src/auth/oauth/mod.rs`, `crates/opi-ai/src/auth/oauth/openrouter.rs`, `crates/opi-ai/src/auth/oauth/chatgpt.rs`, `crates/opi-ai/src/auth/oauth/copilot.rs`, `crates/opi-ai/src/auth/oauth/kimi_coding.rs`, `crates/opi-ai/src/auth/oauth/meta.rs`, `crates/opi-ai/src/auth/oauth/radius.rs`, `crates/opi-ai/src/auth/oauth/xai.rs`, `crates/opi-ai/src/lib.rs`, `crates/opi-coding-agent/src/credential_store.rs`, `crates/opi-ai/tests/auth_contracts.rs`, `crates/opi-ai/tests/builtin_auth_ownership.rs`, `crates/opi-ai/tests/oauth_flows.rs`, `crates/opi-ai/tests/oauth_store_integration.rs`, `crates/opi-ai/tests/provider_collection.rs`, `crates/opi-coding-agent/tests/credential_store.rs`, `crates/opi-coding-agent/tests/doctor_cli.rs`, `crates/opi-coding-agent/tests/github_copilot_provider.rs`, `crates/opi-coding-agent/tests/list_models.rs`, `crates/opi-coding-agent/tests/oauth_auth.rs`, `crates/opi-ai/README.md`, `crates/opi-ai/README.zh.md`, `CHANGELOG.md`
- **Change kind**: behavioral
- **Change**: Introduce CredentialExpiry::{Unknown, NonExpiring, At(OffsetDateTime)} in credential.rs and re-export; replace OAuthCredential and Credential::OAuthToken expires_at fields with expiry. Unknown refreshes, NonExpiring does not, At preserves one skew and existing post-ACK expiry checks. OAuth writes strict v2 tagged expiry, reads v1; only provider=openrouter + empty refresh + exact253402300799 legacy sentinel becomes NonExpiring. API-key v1/v2 unchanged. Preserve all other metadata and secret redaction, remove unused public sentinel helper and keep private legacy migration constant only.
- **Closure predicate**: OpenRouter public Models login/store ACK/keychain round trip and exact legacy-v1 sentinel decode expose NonExpiring with no invented timestamp or extra refresh network. Unknown remains refresh-needed; At retains actual service expiry and one existing skew; other providers/timestamps never become permanent.
- **Red-before**: python D:/Luiz/Odradek/opi/scripts/opi-cargo-cache.py run --target E:/opi/cargo-targets/opi-72fa37a0fa044958 -- test --manifest-path C:\Users\Luiz\AppData\Local\Temp\opi-p20-expiry-red-wyr9zqbx\Cargo.toml --offline --test lifetime -- --nocapture -> FAIL:3executed,2passed/1failed,0ignored,exit101; returned and persisted public metadata contain9999-12-31. Actual fake token exchange/store ACK occurred; no immediate key-loss claim. Receipt 158695e0633e29acc04421178ed1de463c536cee647e980ddf26a19bb8211a9d
- **Green-after**: cargo test --workspace --all-targets --offline --locked; cargo test --workspace --doc --offline --locked; cargo clippy --workspace --all-targets --offline --locked -- -D warnings; cargo doc --workspace --no-deps --offline --locked (RUSTDOCFLAGS=-D warnings); python scripts/opi-doc-check.py; git diff --check -> PASS; all original distinguishing assertions and controls must execute.
- **Test impact**: add/update; retain wire/safety/legacy-reader controls; delete only made-obsolete sentinel assertions after replacement proof.

#### C020 / D7 concrete closure design

Source key: `phase20-codex-gpt61-b3f1810-20261103t142820z / P20-AUD-020`, findings SHA-256 `4cec97af8ef6bb773c630ee251728f31212e12fdc1bf9631f10b2dc3084bc0f6`.
Verified baseline: `d053965b8ef3bca8d79990793459bb0ba87fe69c`.
Verification: Confirmed; severity Minor retained. The existing year9999 timestamp misstates public credential metadata; no immediate key loss or unnecessary refresh was established.

##### Authority decision resolved

Registered `pi-1.3.0-sdk-contracts.md:151` explicitly requires typed non-expiring credentials instead of a maximum timestamp. Row143 defines the OpenRouter permanent-key/empty-refresh/no-network-refresh behavior. This design realizes the already selected source meaning. The enum name, Rust field spelling and strict versioned wire encoding are routine implementation choices, with the scoped0.x API migration and legacy reader preservation governed by AGENTS and registered replacement rules. No specification revision or new authority is needed. Do not claim all of V-55 met from this bounded closure.

##### Public declarations fixed by this plan

Define in `opi_ai::credential`, re-export beside Credential at the crate root:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CredentialExpiry {
    Unknown,
    NonExpiring,
    At(time::OffsetDateTime),
}
```

Replace only OAuth lifetime fields:

```rust
pub struct OAuthCredential {
    // Existing secret and metadata fields retained.
    pub expiry: CredentialExpiry, // replaces expires_at: Option<OffsetDateTime>
}
// Credential::OAuthToken likewise has expiry: CredentialExpiry.
```

This enum has three existing distinguishable source states. Do not add traits, flags, new provider selection or compatibility aliases. No serde ABI is attached to the public type; the product keychain codec owns its versioned representation.

`OAuthCredential::needs_refresh()` matches Unknown => true, NonExpiring => false, At(expiry) => the existing one-time5min comparison. The post-store-ACK expired check in StoreOAuthResolver checks At only; Unknown retains current permissive-after-refresh behavior, rather than silently becoming NonExpiring. Every known service expiry maps to At without subtracting another skew; absent expiry maps to Unknown. Preserve existing source-specific refresh/error/store transaction timing. AWS/ADC expiry structs and provider HTTP payload keys are separate contracts and remain unchanged.

OpenRouter login returns NonExpiring, access=exchanged key, refresh empty; identity refresh clones it and sends no request. Remove the public year9999 constant/helper (made unused by this replacement); keep any required legacy sentinel only as a private codec migration constant. Debug names the new expiry variant and still redacts access/refresh.

##### Durable encoding and migration fixed by this plan

The codec already dispatches by version+kind: API-key v2 exists while OAuth is currently v1. Write OAuth v2 with a mandatory `expiry` tagged object, retaining all other OAuth fields and zeroizing secret buffers:

```json
{"version":2,"kind":"oauth","access":"<secret>","refresh":"<secret>","expiry":{"kind":"unknown"}}
{"version":2,"kind":"oauth","access":"<secret>","refresh":"","expiry":{"kind":"non_expiring"}}
{"version":2,"kind":"oauth","access":"<secret>","refresh":"<secret>","expiry":{"kind":"at","unix_seconds":1700000000}}
```

Retain strict legacy OAuth v1 decoding. Map `expires_at` absent/null to Unknown. Map a valid timestamp to At except the exact source-qualified legacy OpenRouter shape: provider_id is `openrouter`, refresh is empty, and Unix seconds are exactly253402300799; decode this single shape as NonExpiring. Do not recognize the sentinel globally, infer permanence from an empty refresh token alone, or reinterpret arbitrary timestamps. Keep legacy API-key v1 and scoped API-key v2 behavior unchanged. Decode alone does not rewrite the keychain; subsequent authorized writes use OAuth v2. Preserve access/account/client/base URL/scopes/enterprise/model metadata through migration.

Strict OAuth v2 decoding rejects missing/unknown expiry kind, missing/out-of-range At timestamp, extra timestamp on Unknown/NonExpiring, conflicting old `expires_at`, and unknown envelope fields; failure remains typed and does not trigger env fallback. Keep currently supported Unix-second precision rather than adding an unrelated time-format change. Existing OAuth v1 invalid-time/unknown-version/secret-redaction tests remain valid.

##### One closure batch

Suggested key `credential.oauth-expiry-state`; family `auth.credential-lifetime`; decision `fix:represent-permanent-oauth-expiry-explicitly`; no prerequisite beyond ordinary highest-severity scheduling. One root change is replacement of the ambiguous optional timestamp across the OAuth value/store boundary.

Predicate: public Models login of the permanent OpenRouter key, its store-acknowledged Credential, its product-keychain round trip, and exact legacy OpenRouter v1 sentinel decode all expose NonExpiring and no fabricated timestamp; resolver/direct refresh cause no additional network call or rotation. Unknown remains distinct and requests refresh, real expiring values retain one skew and post-ACK expiration behavior; other providers/timestamps are never reclassified as permanent.

The path-by-path inventory and baseline hashes are `affected-inputs.json`. Production changes are only the typed value, constructors/conversions, resolver match and product codec. Test impact: add lifetime/legacy-migration/conflict controls; update old OAuth literal/assertion syntax and the test asserting a9999 sentinel; retain actual wire JSON expiry fixtures, secret redaction, identity, store ACK, failure/cancel and historical v1 readers; delete only obsolete sentinel-specific assertions after stronger typed behavior proof exists. Update English/Chinese crate README together and the full reviewed Unreleased section for the explicit0.x field/helper break and reader-preserving codec upgrade.

##### Verification union

- External `lifetime` probe against baseline: public Models login->fake token endpoint->InMemoryStore; returned/persisted public metadata must not report9999. Independent controls cover no-network identity refresh and unknown/real expiry behavior. Preserve red evidence before production edits.
- At apply, retain the unchanged public returned/stored Debug no-sentinel red assertion as green, and migrate only the old-type control constructors to the declared enum. Add compile-time variant assertions `credential.expiry == CredentialExpiry::NonExpiring` and `Credential::OAuthToken { expiry: NonExpiring, .. }` at the actual public Models/store seam; do not accept merely hiding the old timestamp from Debug.
- Product fake-keychain tests: round trip all3 variants; decode exact legacy OpenRouter sentinel; other-provider same timestamp, nonempty refresh, real OpenRouter timestamp and absent expiry controls; malformed/conflicting v2 inputs fail closed; existing API-key v1/v2 and unknown-version controls retained.
- Run every changed integration target: opi-ai `auth_contracts`, `builtin_auth_ownership`, `oauth_flows`, `oauth_store_integration`, `provider_collection`; opi-coding-agent `credential_store`, `doctor_cli`, `github_copilot_provider`, `list_models`, `oauth_auth`; owning lib tests. Also retain/run `interactive_auth` as a legacy-v1 reader seam.
- Cross-crate/API/durable-format union: cargo fmt --check --all; cargo clippy --workspace --all-targets -- -D warnings; cargo test --workspace --all-targets; cargo test --workspace --doc; RUSTDOCFLAGS=-D warnings cargo doc --workspace --no-deps; python scripts/opi-doc-check.py; git diff --check. The workspace all-targets run satisfies the listed modified targets when logs confirm execution; do not duplicate them without a new failure or edit.

The original lifetime behavioral analysis edited no product code. Its separately authorized workflow/ledger prerequisite remains preserved; the current baseline additionally contains only the later authorized B01 correction. All probes point to the immutable committed export; use a fresh apply source binding for green and preserve probe test/input identities.

### Batch B56: verify p20-sdk-auth-027 finite source cases

**Closure predicate**: The finite shared_store_conformance source cases produce exactly the registered results and side-effect counts below; typed invalid-input controls fail before the stated later boundary.
**Dependencies**: ["B21"]
**Verification union**: python scripts/opi-cargo-cache.py run --target <persistent-target> -- test -p opi-ai --test auth_contracts --offline --locked -- --test-threads=1; python scripts/opi-cargo-cache.py run --target <persistent-target> -- test -p opi-coding-agent --test credential_store --offline --locked -- --test-threads=1; run every created or modified target/filter and affected-target clippy; final workspace/doc/whitespace union applies.

#### Fix B56.1: verify p20-sdk-auth-027 finite source cases

- **Finding source(s)**: phase20-codex-gpt61-b3f1810-20261103t142820z / 4cec97af8ef6bb773c630ee251728f31212e12fdc1bf9631f10b2dc3084bc0f6 / P20-AUD-018
- **Decision**: fix:verify-p20-sdk-auth-027-finite-source-cases
- **Verification status**: Partially confirmed
- **File(s)**: `crates/opi-coding-agent/tests/credential_store.rs`, `crates/opi-ai/tests/auth_contracts.rs`
- **Change kind**: test-only
- **Change**: Add the exact finite assertion matrix below at its current owning seam; production behavior and dependencies are unchanged by this test-only batch.
- **Closure predicate**: The finite shared_store_conformance source cases produce exactly the registered results and side-effect counts below; typed invalid-input controls fail before the stated later boundary.
- **Red-before**: Not applicable to production behavior: this batch adds the finite source-defined conformance test only. Preserve any actual baseline failure and stop for its separately bound behavioral repair; no conditional production edit is admitted.
- **Green-after**: python scripts/opi-cargo-cache.py run --target <persistent-target> -- test -p opi-ai --test auth_contracts --offline --locked -- --test-threads=1; python scripts/opi-cargo-cache.py run --target <persistent-target> -- test -p opi-coding-agent --test credential_store --offline --locked -- --test-threads=1 -> The finite shared_store_conformance source cases produce exactly the registered results and side-effect counts below; typed invalid-input controls fail before the stated later boundary. Observed:not-run.
- **Test impact**: add named assertions/cases; update helpers only as required; retain valid-source and negative controls; delete none.

Assigned current criterion: `P20-SDK-AUTH-027`; exact source `docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-rust-differences.md` SHA256 `411d2f6d262b1afc22e84c73213dffd0eb890ed70e210dfa497d2a5d021cd31d`, citation `L145`.

Current assertion evidence and limits: CredentialStore353inline/inertbarrier differsqueue-backedsharedstore; auth_contracts1374–1621ACK/unknown/barrier; product2176/2242/2308 rotationcancel/barrier.

Finite inputs and oracle: shared_store_conformance: samewrite/read/delete/update/keep/failure/cancel-afterstart/unknown/cutoff cases forMemory andinjectedpersistentadapter; truthfulmemoryvsbackendACK, inlineweakadapternotdurable.

Authority: No product-meaning decision; use existing public seam and registered predicate.

This batch owns only the named additional test portion. Referenced C017/package/native proof is a separate conjunction/barrier; it does not create a backward dependency on the final B17 runner. Existing known behavioral defects keep their named production batches. Unexpected baseline red is preserved and replanned, never silently repaired here.

### Batch B98: expose the registered optional OAuth minimum validity

**Closure predicate**: The selected stored-OAuth owner receives the exact optional minimum, applies max300s before refresh and (only when explicitly present, including zero) after ACK, treats equality as insufficient, preserves rotated tokens and source identity, and retains default None short-TTL behavior without affecting other auth sources.
**Dependencies**: ["B21"]
**Verification union**: oauth_store_integration/auth_contracts plus product oauth_auth/credential_store; private exact floor/ACK/expiry unit table; every modified test binary; applicable workspace fmt/clippy/all-targets/doc-test/rustdoc/doc/diff.

**Downstream final-source proof**: Actual B17/B95 package consumers and B92 native execution are mandatory later C018 conjuncts. They do not gate B98 because those package owners depend on this runtime/input correction.

#### Fix B98.1: expose the registered optional OAuth minimum validity

- **Finding source(s)**: phase20-codex-gpt61-b3f1810-20261103t142820z / 4cec97af8ef6bb773c630ee251728f31212e12fdc1bf9631f10b2dc3084bc0f6 / P20-AUD-018
- **Decision**: fix:forward-optional-minimum-to-selected-stored-oauth-owner
- **Verification status**: Partially confirmed
- **File(s)**: `CHANGELOG.md`, `crates/opi-agent/tests/continuation_contract.rs`, `crates/opi-agent/tests/preparation_transaction.rs`, `crates/opi-ai/README.md`, `crates/opi-ai/README.zh.md`, `crates/opi-ai/src/auth.rs`, `crates/opi-ai/src/auth/adc/native.rs`, `crates/opi-ai/src/auth/builtin.rs`, `crates/opi-ai/src/auth/builtin_aws.rs`, `crates/opi-ai/src/auth/builtin_vertex.rs`, `crates/opi-ai/src/auth/store_oauth.rs`, `crates/opi-ai/src/models.rs`, `crates/opi-ai/src/provider_collection.rs`, `crates/opi-ai/src/test_support.rs`, `crates/opi-ai/tests/anthropic_fixtures.rs`, `crates/opi-ai/tests/api_mapped_provider.rs`, `crates/opi-ai/tests/auth_contracts.rs`, `crates/opi-ai/tests/azure_openai_fixtures.rs`, `crates/opi-ai/tests/bedrock_fixtures.rs`, `crates/opi-ai/tests/builtin_auth_ownership.rs`, `crates/opi-ai/tests/catalog_operations.rs`, `crates/opi-ai/tests/chat_capture_replay.rs`, `crates/opi-ai/tests/classifier_contract.rs`, `crates/opi-ai/tests/codex_native_options.rs`, `crates/opi-ai/tests/codex_telemetry.rs`, `crates/opi-ai/tests/custom_provider_registration.rs`, `crates/opi-ai/tests/deferred_contract.rs`, `crates/opi-ai/tests/external_sdk.rs`, `crates/opi-ai/tests/gemini_lifecycle.rs`, `crates/opi-ai/tests/node4_preparation.rs`, `crates/opi-ai/tests/oauth_store_integration.rs`, `crates/opi-ai/tests/openai_chat_fixtures.rs`, `crates/opi-ai/tests/openai_codex_responses.rs`, `crates/opi-ai/tests/openai_responses_fixtures.rs`, `crates/opi-ai/tests/openai_responses_lifecycle.rs`, `crates/opi-ai/tests/openrouter_fixtures.rs`, `crates/opi-ai/tests/per_request_auth.rs`, `crates/opi-ai/tests/provider_collection.rs`, `crates/opi-ai/tests/request_enrichment.rs`, `crates/opi-ai/tests/resource_cleanup.rs`, `crates/opi-ai/tests/service_tier_cost.rs`, `crates/opi-ai/tests/support/pending_auth.rs`, `crates/opi-ai/tests/telemetry_operations.rs`, `crates/opi-ai/tests/vertex_fixtures.rs`, `crates/opi-coding-agent/src/credential_store.rs`, `crates/opi-coding-agent/src/provider_factory.rs`, `crates/opi-coding-agent/tests/anthropic_cache_markers.rs`, `crates/opi-coding-agent/tests/credential_store.rs`, `crates/opi-coding-agent/tests/cross_mode_runtime.rs`, `crates/opi-coding-agent/tests/oauth_auth.rs`, `crates/opi-coding-agent/tests/provider_runtime.rs`, `crates/opi-coding-agent/tests/runtime_failure_recovery.rs`, `crates/opi-coding-agent/tests/vertex_provider_wiring.rs`, `scripts/pi-sdk-conformance/consumer-ws/src/main.rs`
- **Change kind**: behavioral
- **Baseline class**: user-confirmed source-input extension, limited to P20-SDK-AUTH-006
- **Change**: Add Option<Duration> to AuthResolutionOverrides and the existing AuthResolver method, synchronize its exact caller inventory, and apply the source-prescribed floor under the existing mutation/ACK owner.
- **Closure predicate**: The selected stored-OAuth owner receives the exact optional minimum, applies max300s before refresh and (only when explicitly present, including zero) after ACK, treats equality as insufficient, preserves rotated tokens and source identity, and retains default None short-TTL behavior without affecting other auth sources.
- **Red-before**: Explicit user-confirmed static missing-input baseline, not a runtime FAIL: current public option/trait/store paths cannot express the registered input. Exact source/absence receipt `C:\Users\Luiz\AppData\Local\Temp\opi-phase20-remediate-plan-jqdki4yk\c018-d5-minimum-validity-static-receipt.json` SHA256 `e5cb4b1cf4ac2e43f3e0d2b18e436617a5868adab10ef1f5b2008fe406630bc5` and54path caller inventory are retained. User accepted only this input's static baseline with all future behavior checks mandatory. No compiler failure or default120s valid result is relabeled red. Other C018 behavioral roots retain their actual observed red.
- **Green-after**: Exact None/Some0/Some120/Some600/MAX and -1/0/299/300/301/599/600/601 expiry table; selected get_auth/direct resolver/product bridge through shared-store mutation/refresh/ACK; all modified public owning tests and applicable workspace checks -> The selected stored-OAuth owner receives the exact optional minimum, applies max300s before refresh and (only when explicitly present, including zero) after ACK, treats equality as insufficient, preserves rotated tokens and source identity, and retains default None short-TTL behavior without affecting other auth sources. Observed:not-run.
- **Test impact**: add named floor/state/ACK/source-scope assertions; update exact trait/caller fixture signatures; retain default120s/secret/cancel/account/mutation controls; delete none.

The user explicitly instructed “接受仅此输入的静态基线方案” after reviewing the complete design, absence proof and unchanged runtime acceptance obligation. This instruction supplies the narrowly bounded plan-level baseline classification for an unrepresentable new input. It does not apply to another source or ordinary runtime defect and does not change the general validator or active audit records. At apply, new-input behavior tests precede production implementation; compile-time setup failures remain setup, not fabricated behavioral evidence. Source closure requires every assigned runtime predicate, including this one, to execute successfully.

```json
{
  "id": "C018-minimum-validity-exact-design",
  "criterion": "P20-SDK-AUTH-006",
  "status": "Selected source-input extension; explicit user-confirmed static baseline for this exact registered input",
  "proof_class_decision": "User answer: 接受仅此输入的静态基线方案. This is limited to P20-SDK-AUTH-006 in this plan. No compiler failure is runtime red; all new-input runtime green obligations remain.",
  "source_contract": {
    "registered": "sdk-contracts.md:151",
    "pinned_resolve": {
      "path": "packages/ai/src/auth/resolve.ts",
      "sha256": "b64d4b13ccdb3e8d056ec4849190aa68f002988ee0417317151d59d8668502ec",
      "input_lines": "19-24: AuthResolutionOverrides.minOAuthValidityMs?: number",
      "selection_lines": "70-80: only stored OAuth consumes override",
      "floor_lines": "169-170: effective=max(300000, min??0); expiresSoon = now+effective >= expires",
      "post_refresh_lines": "173-183: explicit presence, including zero, rechecks effective floor after refresh; normal default refresh window is not a provider post-refresh floor"
    },
    "pinned_models": {
      "path": "packages/ai/src/models.ts",
      "sha256": "c10385291e4e79dc3ea5301629288a35cac354f8a3635c07f9312608c269f573",
      "lines": "747-757: getAuth accepts AuthResolutionOverrides and forwards to resolveProviderAuth"
    }
  },
  "current_absence_evidence": [
    {
      "path": "crates/opi-ai/src/models.rs",
      "lines": "226-235,2640-2701",
      "fact": "Only api_key/bearer/aws_sigv4 overrides; selected live resolver invoked without options; type has no serde flatten/metadata escape hatch."
    },
    {
      "path": "crates/opi-ai/src/auth.rs",
      "lines": "491-505,554-579,598-606",
      "fact": "AuthResolver.resolve accepts only self; credential exposes actual expiry and fixed5min needs_refresh, no min-validity input."
    },
    {
      "path": "crates/opi-ai/src/auth/store_oauth.rs",
      "lines": "14-39,84-107,130-142",
      "fact": "Only refresh network timeout configurable; fixed default window before refresh and actual expiry after ACK. Timeout is a different boundary."
    },
    {
      "path": "crates/opi-ai/src/credential.rs",
      "lines": "49-68,121-148",
      "fact": "Scoped env belongs to API-key credentials; OAuth envelope has explicit metadata fields, no min-validity key. Reusing scopes, expires_at or gateway would change meaning/identity or falsify real expiry."
    },
    {
      "path": "crates/opi-ai/src/auth/builtin.rs",
      "lines": "313-331",
      "fact": "Builtin selected OAuth constructs StoreOAuthResolver then calls resolve() with no per-call values."
    }
  ],
  "no_substitute_red": "No missing-field compiler error, invented env key, shortened fake expiry, or caller-owned custom AuthResolver algorithm is a red against the current SDK min-validity input; that input is absent. Existing refreshed120s default success is a valid None control.",
  "selected_public_declarations": {
    "AuthResolutionOverrides": "pub minimum_oauth_validity: Option<std::time::Duration> ; Default=None; ephemeral per-resolution input, no credential/model/store schema field.",
    "AuthResolver": "fn resolve<'a>(&'a self, minimum_oauth_validity: Option<std::time::Duration>) -> BoxAuthFuture<'a, Result<ResolvedAuth, ProviderError>>; retain one object-safe method, no new trait/compat adapter.",
    "StoreOAuthResolver": "Existing AuthResolver implementation consumes the same argument; no global state or constructor-only minimum override.",
    "product_bridge": "CredentialResolver::resolve_oauth takes the same optionalDuration as its last argument and forwards to StoreOAuthResolver; AuthSource::Store/Layered forward the captured value."
  },
  "exact_public_entrances": [
    "Models::get_auth(provider_id, AuthResolutionOverrides { minimum_oauth_validity: Some(duration), ..Default::default() })",
    "Direct AuthResolver::resolve(Some(duration)), including direct StoreOAuthResolver and product CredentialResolver::resolve_oauth(...,Some(duration))",
    "Existing Models generation/classifier/image/deferred/preparation paths pass None; no new Request/StreamOptions minimum field is introduced without source mandate. Their default5min behavior stays selected-owner-local."
  ],
  "algorithm": [
    "Capture the Option<Duration> before source selection/await. Explicit supplied API-key/bearer/SigV4 continues to bypass configured sources; min validity concerns stored OAuth, not those explicit tokens.",
    "For store-owned OAuth, one private pure helper accepts actual expiry, now and optional duration. Effective floor is max(300s, explicit.unwrap_or(0)); remaining <= floor triggers refresh, including equality. Compare remaining duration without overflowing now+Duration::MAX.",
    "Recheck under the same store/source/account mutation lock and reuse only a same-identity token meeting that exact effective floor; no new identity or fallback source. Network timeout remains15s and excludes lock/ACK.",
    "Persist unique rotated credentials and await ACK before successful return. After ACK, always reject actual expiry <= now. If explicit input is Some, reject remaining <= effective floor even for Some(0) or Some(120s). None accepts a freshly rotated120s token once ACKed and still actually unexpired.",
    "A too-short refreshed token is still saved before returning typed AuthFailed; do not discard the rotated refresh token or falsely report rollback. Subsequent source selection remains unchanged.",
    "C020 typed expiry dependency: NonExpiring satisfies any finite minimum; Unknown triggers refresh and cannot satisfy an explicit proof of remaining lifetime if still Unknown after refresh; default Unknown behavior stays C020-owned.",
    "Builtin API-key/env/AWS/ADC source branches consume no stored-OAuth floor; they explicitly ignore this argument rather than treating all bearer credentials as stored OAuth. Builtin OAuth and AuthSource delegated store branches forward it. Custom AuthResolver implementors now receive the required optional input and document their stored-OAuth behavior.",
    "Descriptor-only stored OAuth lacks a refresh owner: an explicit floor must be checked against its actual known expiry and typed AuthFailed if not met; it cannot claim refresh success. No floor/None retains existing lookup-only contract.",
    "Use one private threshold helper taking explicit now. Production resolver reads OffsetDateTime::now_utc at the existing pre-refresh/post-ACK points; a private resolve_with_clock generic/helper may supply a deterministic clock only to internal unit state-machine tests. No new public clock trait/config/global mutation. Public integration tests use safe TTL margins; exact equality and ACK-clock advancement are verified through the same private production state machine with fixed clock."
  ],
  "boundary_table": [
    {
      "input": "None",
      "effective_seconds": 300,
      "existing_remaining_seconds": 240,
      "refresh_remaining_seconds": 120,
      "result": "refresh once; ACK then success if actually unexpired"
    },
    {
      "input": "Some(0)",
      "effective_seconds": 300,
      "existing_remaining_seconds": 240,
      "refresh_remaining_seconds": 120,
      "result": "refresh once; ACK then AuthFailed"
    },
    {
      "input": "Some(120s)",
      "effective_seconds": 300,
      "existing_remaining_seconds": 240,
      "refresh_remaining_seconds": 301,
      "result": "refresh once; ACK then success"
    },
    {
      "input": "Some(600s)",
      "effective_seconds": 600,
      "existing_remaining_seconds": 540,
      "refresh_remaining_seconds": 601,
      "result": "refresh once; ACK then success"
    },
    {
      "input": "Some(600s)",
      "effective_seconds": 600,
      "existing_remaining_seconds": 600,
      "refresh_remaining_seconds": 600,
      "result": "boundary equality triggers refresh and post-ACK rejection"
    },
    {
      "input": "Some(600s)",
      "effective_seconds": 600,
      "existing_remaining_seconds": 601,
      "result": "no refresh; use current same-identity credential"
    },
    {
      "input": "Some(Duration::MAX)",
      "result": "no arithmetic panic; finite expiring credential cannot satisfy floor"
    }
  ],
  "caller_inventory": "C:\\Users\\Luiz\\AppData\\Local\\Temp\\opi-phase20-remediate-plan-jqdki4yk\\c018-d5-auth-caller-inventory.json",
  "caller_sync": [
    "All exact AuthResolver implementation sites in inventory gain optionalDuration parameter. SDK/product OAuth delegates forward it; native-source-only and static fixtures accept an explicitly unused parameter.",
    "Every existing auth .resolve() call supplies None unless forwarding an already selected floor; models.get_auth supplies its override. Ignore non-auth handle.resolve/job.resolve search matches at models.rs2238 and openai_codex_responses.rs492; they are not this trait.",
    "All7 explicit AuthResolutionOverrides literals in tests/auth_contracts.rs591,661,713,753,758,763,769 add None; Default-based callers require no source change.",
    "Product resolve_oauth calls in credential_store.rs and oauth_auth.rs tests add None, except new explicit-floor cases; consumer-ws LocalAuth and external_sdk StaticBearer implementations update signature, then actual package consumer reruns."
  ],
  "exact_changed_paths": [
    "CHANGELOG.md",
    "crates/opi-agent/tests/continuation_contract.rs",
    "crates/opi-agent/tests/preparation_transaction.rs",
    "crates/opi-ai/README.md",
    "crates/opi-ai/README.zh.md",
    "crates/opi-ai/src/auth.rs",
    "crates/opi-ai/src/auth/adc/native.rs",
    "crates/opi-ai/src/auth/builtin.rs",
    "crates/opi-ai/src/auth/builtin_aws.rs",
    "crates/opi-ai/src/auth/builtin_vertex.rs",
    "crates/opi-ai/src/auth/store_oauth.rs",
    "crates/opi-ai/src/models.rs",
    "crates/opi-ai/src/provider_collection.rs",
    "crates/opi-ai/src/test_support.rs",
    "crates/opi-ai/tests/anthropic_fixtures.rs",
    "crates/opi-ai/tests/api_mapped_provider.rs",
    "crates/opi-ai/tests/auth_contracts.rs",
    "crates/opi-ai/tests/azure_openai_fixtures.rs",
    "crates/opi-ai/tests/bedrock_fixtures.rs",
    "crates/opi-ai/tests/builtin_auth_ownership.rs",
    "crates/opi-ai/tests/catalog_operations.rs",
    "crates/opi-ai/tests/chat_capture_replay.rs",
    "crates/opi-ai/tests/classifier_contract.rs",
    "crates/opi-ai/tests/codex_native_options.rs",
    "crates/opi-ai/tests/codex_telemetry.rs",
    "crates/opi-ai/tests/custom_provider_registration.rs",
    "crates/opi-ai/tests/deferred_contract.rs",
    "crates/opi-ai/tests/external_sdk.rs",
    "crates/opi-ai/tests/gemini_lifecycle.rs",
    "crates/opi-ai/tests/node4_preparation.rs",
    "crates/opi-ai/tests/oauth_store_integration.rs",
    "crates/opi-ai/tests/openai_chat_fixtures.rs",
    "crates/opi-ai/tests/openai_codex_responses.rs",
    "crates/opi-ai/tests/openai_responses_fixtures.rs",
    "crates/opi-ai/tests/openai_responses_lifecycle.rs",
    "crates/opi-ai/tests/openrouter_fixtures.rs",
    "crates/opi-ai/tests/per_request_auth.rs",
    "crates/opi-ai/tests/provider_collection.rs",
    "crates/opi-ai/tests/request_enrichment.rs",
    "crates/opi-ai/tests/resource_cleanup.rs",
    "crates/opi-ai/tests/service_tier_cost.rs",
    "crates/opi-ai/tests/support/pending_auth.rs",
    "crates/opi-ai/tests/telemetry_operations.rs",
    "crates/opi-ai/tests/vertex_fixtures.rs",
    "crates/opi-coding-agent/src/credential_store.rs",
    "crates/opi-coding-agent/src/provider_factory.rs",
    "crates/opi-coding-agent/tests/anthropic_cache_markers.rs",
    "crates/opi-coding-agent/tests/credential_store.rs",
    "crates/opi-coding-agent/tests/cross_mode_runtime.rs",
    "crates/opi-coding-agent/tests/oauth_auth.rs",
    "crates/opi-coding-agent/tests/provider_runtime.rs",
    "crates/opi-coding-agent/tests/runtime_failure_recovery.rs",
    "crates/opi-coding-agent/tests/vertex_provider_wiring.rs",
    "scripts/pi-sdk-conformance/consumer-ws/src/main.rs"
  ],
  "finite_cases": [
    {
      "path": "crates/opi-ai/src/auth.rs",
      "name": "minimum_oauth_validity_boundary_table",
      "input": "fixed now and expiry -1/0/299/300/301/599/600/601 seconds crossed with None/Some0/Some120/Some600/MAX; C020 expiry variants",
      "expected": "exact floor/presence/equality/overflow outcomes above"
    },
    {
      "path": "crates/opi-ai/tests/oauth_store_integration.rs",
      "name": "explicit_minimum_is_checked_under_lock_and_after_ack",
      "input": "two Models share one store; controlled clock/flow and delayed ACK; old9min, explicit10min, refreshed11min; second source defaultNone; repeat refreshed10min and2min; deterministic exact clock/ACK crossings through the same private production state-machine helper, public entry checks use safe TTL margins",
      "expected": "one same-key refresh, ACK first, explicit floor checked at return, rotated token persisted on short-floor error, None shortTTL control passes, no account/envfallback"
    },
    {
      "path": "crates/opi-ai/tests/auth_contracts.rs",
      "name": "get_auth_minimum_reaches_selected_oauth_owner_only",
      "input": "get_auth None/Some0/Some600 with registered OAuth resolver; explicit3credentialoverride controls; descriptor-only OAuth; nativeAWS/ADC/APIkey sources",
      "expected": "floor forwarded exactly to selected OAuth owner, no sourcefallback, no effect on explicit or non-stored-OAuth branches"
    },
    {
      "path": "crates/opi-coding-agent/tests/oauth_auth.rs",
      "name": "product_auth_source_forwards_explicit_oauth_minimum",
      "input": "AuthSource Store/Layered through persistent resolver, refreshedshorttoken and exactsamekeyACK",
      "expected": "same floor and persistedrotation behavior as SDK, existing default call sites unchanged semantically"
    }
  ],
  "test_impact": {
    "add": [
      "named floor/state-machine tests"
    ],
    "update": "trait-implementor fixture signatures and exact caller list",
    "delete": [],
    "retain": [
      "existing default120s shortTTL, mutationACK/cancel/account isolation controls"
    ]
  },
  "verification": [
    "Run every modified test binary, plus opi-ai unit helper filter; SDK/product public-boundary integration and external actual crate consumer (C017).",
    "Cross-crate runtime/public trait change requires workspace fmt, clippy all-targets -Dwarnings, all-targets tests, doc tests and rustdoc -Dwarnings under shared lifecycle.",
    "No current behavioral-red claim until authorized narrow proof-class decision; final repaired-byte green is future apply evidence."
  ],
  "remaining_authority": "No unresolved planning decision. Only new explicit apply invocation of the final READY digest admits product changes."
}
```

### Batch B31: measure response activation duration in missing HTTP and Codex WS producers

**Closure predicate**: Anthropic, Mistral, Codex HTTP, Radius/PiMessages, Bedrock and Codex WS stamp absent terminal duration once from first actual response-stream poll through success, partial failure or cancellation; known zero and existing/deferred/tool-owned duration survive, while setup/auth and unpolled time are excluded.
**Dependencies**: ["B98"]
**Verification union**: cargo test -p opi-ai --test response_duration --test deferred_contract --offline --locked; every touched producer fixture target; workspace fmt/clippy/all-targets/doc-test/rustdoc gates; doc and whitespace checks.

#### Fix B31.1: measure response activation duration in missing HTTP and Codex WS producers

- **Finding source(s)**: phase20-codex-gpt61-b3f1810-20261103t142820z / 4cec97af8ef6bb773c630ee251728f31212e12fdc1bf9631f10b2dc3084bc0f6 / P20-AUD-018
- **Decision**: fix:stamp-response-activation-duration-at-five-http-producers
- **Verification status**: Partially confirmed
- **File(s)**: `crates/opi-ai/src/response_timing.rs`, `crates/opi-ai/src/lib.rs`, `crates/opi-ai/src/anthropic.rs`, `crates/opi-ai/src/mistral.rs`, `crates/opi-ai/src/bedrock/mod.rs`, `crates/opi-ai/src/pi_messages.rs`, `crates/opi-ai/src/openai_codex_responses.rs`, `crates/opi-ai/src/openai_codex_responses/ws_transport.rs`, `crates/opi-ai/tests/response_duration.rs`, `crates/opi-ai/tests/deferred_contract.rs`, `crates/opi-ai/README.md`, `crates/opi-ai/README.zh.md`, `CHANGELOG.md`, `crates/opi-ai/tests/anthropic_fixtures.rs`
- **Change kind**: behavioral
- **Change**: Add one private ResponseTiming activation/stamp mechanism to the five observed missing HTTP and Codex WS producers. Preserve existing timed adapters as independent controls and retain their timer ownership. Codex WS remains a separately proved path before it can join this scope.
- **Closure predicate**: Anthropic, Mistral, Codex HTTP, Radius/PiMessages, Bedrock and Codex WS stamp absent terminal duration once from first actual response-stream poll through success, partial failure or cancellation; known zero and existing/deferred/tool-owned duration survive, while setup/auth and unpolled time are excluded.
- **Red-before**: FAIL: actual Codex WS plain/compressed chains also pass original peer/id/Stop/text/Close controls then report durationNone (8checks pass,2fail,total10; original harness process exit0 with JSON verdictFAIL). Exact receipt `C:\Users\Luiz\AppData\Local\Temp\opi-phase20-remediate-plan-jqdki4yk\c018-ws-duration-receipt.json` SHA256 `dc172c47fe50ac678f151fcdb66c290610ee7d12d744e9e572a81412b640cea6`; process exit0 is not relabeled. FAIL: python scripts/opi-cargo-cache.py run --target E:\opi\cargo-targets\opi-72fa37a0fa044958 -- test --manifest-path C:\Users\Luiz\AppData\Local\Temp\opi-phase20-remediate-plan-jqdki4yk\c018-duration-matrix\Cargo.toml --offline --test external_sdk assembled_matrix_drives_all_ten_published_chat_wires_through_one_instance -- --exact --nocapture --test-threads=1;1test executed/1failed/0ignored,exit101. All ten original completion/text assertions and all ten peer verifications pass; Anthropic/Mistral/CodexHTTP/Radius/Bedrock are None, five other owners Some(0). Receipt `C:\Users\Luiz\AppData\Local\Temp\opi-phase20-remediate-plan-jqdki4yk\c018-duration-matrix-receipt.json` SHA256 `4153ea420940c78121a57f03ca9976c96a6f0aeffaa1167bb5cd3a4297392e07`. Earlier Anthropic standalone red/Responses control remain separately bound.
- **Green-after**: cargo test -p opi-ai --test response_duration --test deferred_contract --offline --locked plus affected producer fixtures -> Anthropic, Mistral, Codex HTTP, Radius/PiMessages, Bedrock and Codex WS stamp absent terminal duration once from first actual response-stream poll through success, partial failure or cancellation; known zero and existing/deferred/tool-owned duration survive, while setup/auth and unpolled time are excluded. Observed:not-run.
- **Test impact**: add exact ten-wire/entry/terminal/knownness controls; update affected producer tests; retain existing timing owners and deferred/tool metadata; delete none.

- Add private ResponseTiming state with optional activation Instant and stamp-if-absent method; use existing standard monotonic clock production path and test-only injected elapsed source. No public clock trait, global timer or background task.
- Activate inside each missing adapter producer immediately before its first response-body/frame poll, even if Pending. Creating or polling the outer Models future during auth does not activate it. Setup/preflight/HTTP-acquisition failure before response activation remains None.
- Pass terminal AssistantMessage through the same stamp operation on Done, partial typed error and cancellation after activation. Do not overwrite Some, including zero; no second measurement in ChatStream.result/complete/replay or Models.
- Five missing HTTP owners are now behavioral-red through unchanged current10wire complete_simple matrix: Anthropic,MistralConversations,Bedrock,PiMessages,CodexHTTP. Existing Chat/Responses/Azure/Google/Vertex timing controls allSome0. Do not automaticallyrefactor them. CodexWS is separate unproved branch and is not in this repaired-pathunion.
- No timing added to fetched previously ready deferred result. ToolResult duration remains caller metadata and is never synthesized by LLM response timing.

```json
[
  {
    "name": "response_duration_success_across_entries_and_wires",
    "inputs": "10 published wire identities x direct/native stream/native complete/simple stream/simple complete =50 local cases",
    "expected": "successful activated terminal Some(duration), direct and forwarded terminal retain same value; setup/auth waits excluded"
  },
  {
    "name": "response_duration_partial_failure_and_abort",
    "inputs": "10wires x emitted-prefix then typed failure or cancellation =20 local cases",
    "expected": "received prefix/usage/id retained with Some duration and one terminal; no timing for preactivation failures"
  },
  {
    "name": "response_duration_clock_and_knownness",
    "inputs": "private fake elapsed0,17; unpolled; setupfailure; preexistingSome7; delayedfetch readySome7; toolownerNone/0/9",
    "expected": "0 representable,monotonic nonnegative,activation once,no unpolledtask,existing/deferred/toolmetadata unchanged"
  }
]
```

Codex WS producer starts timing immediately before its first actual response-frame poll (including Pending), excluding setup/Upgrade acquisition. Rebind per active call on pooled transport. Reuse existing private elapsed/stamp owner; retain cancellation/cleanup barriers and known terminal fields. Repeat plain+PMD source-consumer controls and all four entry forms. A proof runner must reject any JSON failed check even when its process exits0.

### Batch B62: verify p20-sdk-model-029 finite source cases

**Closure predicate**: The finite tool_duration_owner source cases produce exactly the registered results and side-effect counts below; typed invalid-input controls fail before the stated later boundary.
**Dependencies**: ["B31"]
**Verification union**: python scripts/opi-cargo-cache.py run --target <persistent-target> -- test -p opi-ai --test external_sdk --offline --locked -- --test-threads=1; python scripts/opi-cargo-cache.py run --target <persistent-target> -- test -p opi-ai --test replay_contract --offline --locked -- --test-threads=1; run every created or modified target/filter and affected-target clippy; final workspace/doc/whitespace union applies.

#### Fix B62.1: verify p20-sdk-model-029 finite source cases

- **Finding source(s)**: phase20-codex-gpt61-b3f1810-20261103t142820z / 4cec97af8ef6bb773c630ee251728f31212e12fdc1bf9631f10b2dc3084bc0f6 / P20-AUD-018
- **Decision**: fix:verify-p20-sdk-model-029-finite-source-cases
- **Verification status**: Partially confirmed
- **File(s)**: `crates/opi-ai/tests/external_sdk.rs`, `crates/opi-ai/tests/replay_contract.rs`
- **Change kind**: test-only
- **Change**: Add the exact finite assertion matrix below at its current owning seam; production behavior and dependencies are unchanged by this test-only batch.
- **Closure predicate**: The finite tool_duration_owner source cases produce exactly the registered results and side-effect counts below; typed invalid-input controls fail before the stated later boundary.
- **Red-before**: Not applicable to production behavior: this batch adds the finite source-defined conformance test only. Preserve any actual baseline failure and stop for its separately bound behavioral repair; no conditional production edit is admitted.
- **Green-after**: python scripts/opi-cargo-cache.py run --target <persistent-target> -- test -p opi-ai --test external_sdk --offline --locked -- --test-threads=1; python scripts/opi-cargo-cache.py run --target <persistent-target> -- test -p opi-ai --test replay_contract --offline --locked -- --test-threads=1 -> The finite tool_duration_owner source cases produce exactly the registered results and side-effect counts below; typed invalid-input controls fail before the stated later boundary. Observed:not-run.
- **Test impact**: add named assertions/cases; update helpers only as required; retain valid-source and negative controls; delete none.

Assigned current criterion: `P20-SDK-MODEL-029`; exact source `docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md` SHA256 `9e3d6ef690c3a794bc9c47ddc2dce04840f98d8881649be1ab2e8db0df287fd5`, citation `L166`.

Current assertion evidence and limits: SDKcontracts166toolowneronly; externalmanualtoolloop hostappendsresult,noSDKtool execution.

Finite inputs and oracle: tool_duration_owner: callerNone/Some0/Some17Serde/replaypreserved;LLMtimerdoesnotrewrite; no implicittoolcallbackexecution.

Authority: No product-meaning decision; use existing public seam and registered predicate.

This batch owns only the named additional test portion. Referenced C017/package/native proof is a separate conjunction/barrier; it does not create a backward dependency on the final B17 runner. Existing known behavioral defects keep their named production batches. Unexpected baseline red is preserved and replanned, never silently repaired here.

Exact assembled-test ownership: changes to external_sdk.rs are additional assertions/local helpers inside the already declared manual_tool_loop_continues_after_serde_roundtrip and duration_is_measured_once_drained_and_survives_serialization. Add no new #[test]/#[tokio::test] item to that source and do not rename or expand B17's exact nineteen-case allowlist. Separately named cases may be added only to the other explicitly listed owning binary; run those binaries separately.

### Batch B96: propagate canonical route identity through neutral output

**Closure predicate**: Every canonical-route public event/terminal/partial result retains the selected outer provider identity; inner wire endpoint/model/auth/callback semantics and frozen old/new registration remain unchanged.
**Dependencies**: ["B98"]
**Verification union**: cargo test -p opi-ai --test provider_collection --offline --locked; cargo test -p opi-ai --test chat_pi_messages --offline --locked; rerun exact sealed external radius_seams binary after authorized repair; required runtime/source-public-API verification union; all changed targets and applicable workspace fmt/clippy/all-targets/doc-test/rustdoc; doc and diff checks.

**Downstream final-source proof**: B17/B95 execute the final-artifact public consumers; B92 executes their native matrix. They remain mandatory later C018 conjuncts, after local route-identity/public/workspace checks.

#### Fix B96.1: propagate canonical route identity through neutral output

- **Finding source(s)**: phase20-codex-gpt61-b3f1810-20261103t142820z / 4cec97af8ef6bb773c630ee251728f31212e12fdc1bf9631f10b2dc3084bc0f6 / P20-AUD-018
- **Decision**: fix:propagate-canonical-route-identity-through-neutral-output
- **Verification status**: Partially confirmed
- **File(s)**: `crates/opi-ai/src/provider_collection.rs`, `crates/opi-ai/src/provider.rs`, `crates/opi-ai/tests/provider_collection.rs`, `crates/opi-ai/tests/chat_pi_messages.rs`, `crates/opi-ai/README.md`, `crates/opi-ai/README.zh.md`, `CHANGELOG.md`
- **Change kind**: behavioral
- **Change**: Apply only the selected source-preserving root below; retain explicit transport/identity authority and all existing valid-input controls.
- **Closure predicate**: Every canonical-route public event/terminal/partial result retains the selected outer provider identity; inner wire endpoint/model/auth/callback semantics and frozen old/new registration remain unchanged.
- **Red-before**: FAIL: python scripts/opi-cargo-cache.py run --target E:\opi\cargo-targets\opi-72fa37a0fa044958 -- test --manifest-path C:\Users\Luiz\AppData\Local\Temp\opi-phase20-remediate-plan-jqdki4yk\c018-d5-seams-probe\Cargo.toml --offline --locked --test radius_seams -- --nocapture;build succeeded,3tests2passed/1failed/0ignored,wrapper exit1. Existing canonical_route accepts custom id/empty records, register_route retains profile name, both old/new requests reach their exact frozen gateways and Stop/id/model checks pass, but returned provider is radius instead of radius-owned. Receipt `C:\Users\Luiz\AppData\Local\Temp\opi-phase20-remediate-plan-jqdki4yk\c018-d5-radius-seams-receipt.json` SHA256 `c1c455f9c7e4c9853fb4c35d5d03d7cb3b6299dfc9ec514c40c73d69c0929703`.
- **Green-after**: cargo test -p opi-ai --test provider_collection --offline --locked; cargo test -p opi-ai --test chat_pi_messages --offline --locked; rerun exact sealed external radius_seams binary after authorized repair; required runtime/source-public-API verification union -> Every canonical-route public event/terminal/partial result retains the selected outer provider identity; inner wire endpoint/model/auth/callback semantics and frozen old/new registration remain unchanged. Observed:not-run.
- **Test impact**: add the named discriminating cases; update exact mechanical API callers; retain valid-source/typed-failure/secret/cancel controls; delete only implementation made unused by this exact replacement.

```json
{
  "id": "C018-radius-canonical-identity",
  "criterion": "P20-SDK-PROFILE-010",
  "decision": "Existing SDK public seams express id/name/gateway. Remove missing-public-input obligation for this branch; use actual identity propagation red. No new public constructor or user-wrapper implementation is required.",
  "source_authority": [
    {
      "path": "docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md",
      "lines": [
        93,
        147
      ],
      "predicate": "explicit id/name/gateway fixed at construction"
    },
    {
      "path": "packages/ai/src/providers/radius.ts",
      "sha256": "40acce75f4f5c99cbeb03247cbf0fbdde9e911b097af3c25132cdf1f75346c32",
      "lines": [
        15,
        29,
        35,
        42
      ],
      "predicate": "factory owns id/name/normalized gateway and models provider=id"
    }
  ],
  "existing_input": {
    "id": "provider_collection::canonical_route(custom_id, owned_records, Box<PiMessagesProvider>)",
    "name": "Models::register_route(..., CompatMetadata { profile: Some(name), .. }) ; same field used by register_builtin at provider_collection.rs:947-951",
    "gateway": "PiMessagesProvider::new(Some(gateway))",
    "oauth": "RadiusOAuthProvider::new_with_gateway(gateway).with_identity(id,name), registered through Models::register_oauth_route",
    "empty_catalog": "with_models(vec![]) plus canonical_route; no ApiMappedProvider nonempty constraint or caller-defined Provider implementation"
  },
  "red_receipt": "C:\\Users\\Luiz\\AppData\\Local\\Temp\\opi-phase20-remediate-plan-jqdki4yk\\c018-d5-radius-seams-receipt.json",
  "one_root": "CanonicalRoute owns a public provider identity but delegates returned neutral message identity unchanged to inner fixed-id producer.",
  "predicate": "Every canonical-route public event/terminal/partial result retains the selected outer provider identity; inner wire endpoint/model/auth/callback semantics and frozen old/new registration remain unchanged.",
  "minimal_changed_paths": [
    "crates/opi-ai/src/provider_collection.rs",
    "crates/opi-ai/src/provider.rs",
    "crates/opi-ai/tests/provider_collection.rs",
    "crates/opi-ai/tests/chat_pi_messages.rs",
    "crates/opi-ai/README.md",
    "crates/opi-ai/README.zh.md",
    "CHANGELOG.md"
  ],
  "design": [
    "One private CanonicalRoute stream mapping helper wraps each of stream_prepared, stream_attempt, stream_resource_attempt, stream_model_resource_attempt. Rewrite neutral message.provider from the wrapped inner id to the captured canonical id in all Start/progress/Done/legacy Error snapshots, never mutate raw callback payloads or unrelated provider identities.",
    "Add a crate-private ProviderError method that rewrites only the existing StreamFailure.message provider in place; preserve typed cause, response_received and call_cancelled bits. Do not destructure through into_stream_parts/rebuild, which loses transport/cancellation facts. Bare setup errors retain outer collector identity.",
    "Keep id/model-prefix rewriting and exact old/new gateway snapshot ownership; selected response id/model/wire/usage/duration and canonical caller history stay untouched.",
    "Retain same-identity versus foreign-identity replay controls through the existing projection path. This closure does not authorize inventing identity affinity by overwriting canonical input transcripts; any current cross-identity replay defect must map to its own proved root rather than being silently assumed fixed.",
    "Use the existing OAuth custom id/name/gateway path and current selected actual-login fixture; no public for_route addition is needed to satisfy these inputs."
  ],
  "cases": [
    {
      "name": "canonical_route_preserves_custom_identity_and_gateway_snapshot",
      "input": "Same sealed external probe, old and new route captured around registration replacement",
      "expected": "2 requests to respective gateways, both provider radius-owned, Stop/response id/model controls unchanged"
    },
    {
      "name": "canonical_radius_identity_covers_events_partial_failure_and_cancel",
      "input": "Local valid start/text SSE then normal Done, explicit remote error, truncated body, cancellation after text; exercise stream_prepared and Models simple collection",
      "expected": "Every emitted/partial provider is outer id; text/usage/response id preserved, one typed terminal, no new retry or callback"
    },
    {
      "name": "canonical_radius_identity_replay_control",
      "input": "Own previous returned assistant versus same model under a different custom id; signed thinking and ordinary text",
      "expected": "Canonical input unchanged, selected outer identity governs existing replay affinity; unrelated identities never renamed into same identity"
    },
    {
      "name": "canonical_default_radius_identity_control",
      "input": "Sealed default radius control",
      "expected": "Existing radius output identity unchanged"
    },
    {
      "name": "empty_custom_identity_registration_is_owned_and_zero_io",
      "input": "Sealed empty custom route and matching OAuth factory input",
      "expected": "Empty catalog accepted, name retained, no network, id/name frozen"
    }
  ],
  "verification": [
    "cargo test -p opi-ai --test provider_collection --offline --locked",
    "cargo test -p opi-ai --test chat_pi_messages --offline --locked",
    "rerun exact sealed external radius_seams binary after authorized repair",
    "required runtime/source-public-API verification union and package consumer"
  ],
  "separate_roots": [
    "Nondefault gateway baseline still has its earlier independent actual red c018-radius-receipt.json.",
    "Dynamic received empty catalog authority remains separate finite catalog test batch."
  ],
  "remaining": "D8 owner source reconciliation is complete under explicit P0/P2/P4 confirmation and write-only install; final repaired-byte green is not-run. No missing-input proof exception remains for Radius identity/name."
}
```

### Batch B99: preserve CONNECT overread in one owned HTTP transport

**Closure predicate**: The controlled HttpClient path hands every byte after a successful CONNECT head to the next protocol owner exactly once; proxy and target TLS remain correctly scoped, with the same frozen source configuration, pool ownership, typed failure, redirects/retry/cancellation and ordinary upstream HTTP acceptance profile.
**Dependencies**: ["B02", "B98"]
**Verification union**: original public CONNECT negative schedules plus HTTP/proxy/TLS/WS integration targets; all synchronized HTTP/OAuth/ADC/AWS/public SDK caller targets; all changed targets and applicable workspace fmt/clippy/all-targets/doc-test/rustdoc; doc and diff checks.

**Downstream final-source proof**: B91 ordinary/feature-union families, B17/B95 final public consumers and B92 native rustc1.97.1 execution remain mandatory C018 conjuncts. Actual receipts are required before source/result closure. They do not gate B99 because B91 and its descendants depend on this transport correction.

#### Fix B99.1: preserve CONNECT overread in one owned HTTP transport

- **Finding source(s)**: phase20-codex-gpt61-b3f1810-20261103t142820z / 4cec97af8ef6bb773c630ee251728f31212e12fdc1bf9631f10b2dc3084bc0f6 / P20-AUD-018
- **Decision**: fix:preserve-CONNECT-overread-in-one-owned-HTTP-transport
- **Verification status**: Partially confirmed
- **File(s)**: `CHANGELOG.md`, `Cargo.lock`, `Cargo.toml`, `crates/opi-ai/Cargo.toml`, `crates/opi-ai/README.md`, `crates/opi-ai/README.zh.md`, `crates/opi-ai/src/anthropic.rs`, `crates/opi-ai/src/api/llama.rs`, `crates/opi-ai/src/api/openrouter_images.rs`, `crates/opi-ai/src/api/system_one.rs`, `crates/opi-ai/src/auth/adc/native.rs`, `crates/opi-ai/src/auth/anthropic_federation.rs`, `crates/opi-ai/src/auth/aws/source_http.rs`, `crates/opi-ai/src/auth/oauth/anthropic.rs`, `crates/opi-ai/src/auth/oauth/chatgpt.rs`, `crates/opi-ai/src/auth/oauth/codex.rs`, `crates/opi-ai/src/auth/oauth/copilot.rs`, `crates/opi-ai/src/auth/oauth/kimi_coding.rs`, `crates/opi-ai/src/auth/oauth/meta.rs`, `crates/opi-ai/src/auth/oauth/mod.rs`, `crates/opi-ai/src/auth/oauth/openrouter.rs`, `crates/opi-ai/src/auth/oauth/radius.rs`, `crates/opi-ai/src/auth/oauth/xai.rs`, `crates/opi-ai/src/azure_openai.rs`, `crates/opi-ai/src/bedrock/mod.rs`, `crates/opi-ai/src/gemini.rs`, `crates/opi-ai/src/http.rs`, `crates/opi-ai/src/http/proxy_tunnel.rs`, `crates/opi-ai/src/http/request.rs`, `crates/opi-ai/src/http/transport.rs`, `crates/opi-ai/src/mistral.rs`, `crates/opi-ai/src/openai_chat.rs`, `crates/opi-ai/src/openai_codex_responses.rs`, `crates/opi-ai/src/openai_responses.rs`, `crates/opi-ai/src/pi_messages.rs`, `crates/opi-ai/src/provider.rs`, `crates/opi-ai/src/vertex.rs`, `crates/opi-ai/src/ws.rs`, `crates/opi-ai/tests/http_acceptance.rs`, `crates/opi-ai/tests/oauth_flows.rs`, `crates/opi-ai/tests/platform_delivery.rs`, `crates/opi-ai/tests/proxy_support.rs`, `crates/opi-ai/tests/tls_contract.rs`, `crates/opi-ai/tests/ws_upgrade.rs`, `crates/opi-coding-agent/src/harness.rs`, `crates/opi-coding-agent/src/interactive.rs`, `crates/opi-coding-agent/src/interactive_auth.rs`, `crates/opi-coding-agent/src/oauth.rs`, `crates/opi-coding-agent/tests/interactive_auth.rs`, `crates/opi-coding-agent/tests/interactive_tui_auth.rs`, `crates/opi-coding-agent/tests/oauth_auth.rs`, `crates/opi-coding-agent/tests/test_hermeticity.rs`
- **Change kind**: behavioral
- **Change**: Apply only the selected source-preserving root below; retain explicit transport/identity authority and all existing valid-input controls.
- **Closure predicate**: The controlled HttpClient path hands every byte after a successful CONNECT head to the next protocol owner exactly once; proxy and target TLS remain correctly scoped, with the same frozen source configuration, pool ownership, typed failure, redirects/retry/cancellation and ordinary upstream HTTP acceptance profile.
- **Red-before**: FAIL: original actual HttpClient coalesced CONNECT200+TLS-alert red remains1executed/1failed/0ignored,exit101. It proves suffix handoff/error-owner discrimination, not normal TLS negotiation; control read only ClientHello first-byte synchronization. Exact qualification `C:\Users\Luiz\AppData\Local\Temp\opi-http005-protocol-qualification-0uw_lin0\qualification.json` SHA256 `f02f852dcb8f6436b6e3cc7579574186364510bb36b92a92a40cea75d09661bf`; original receipt retained. Public API composition compile-only check passes with258exact registry identities;0network/0behavioral tests, not claimed green.
- **Green-after**: original public CONNECT negative schedules plus HTTP/proxy/TLS/WS integration targets; all synchronized HTTP/OAuth/ADC/AWS/public SDK caller targets -> The controlled HttpClient path hands every byte after a successful CONNECT head to the next protocol owner exactly once; proxy and target TLS remain correctly scoped, with the same frozen source configuration, pool ownership, typed failure, redirects/retry/cancellation and ordinary upstream HTTP acceptance profile. Observed:not-run.
- **Test impact**: add the named discriminating cases; update exact mechanical API callers; retain valid-source/typed-failure/secret/cancel controls; delete only implementation made unused by this exact replacement.

Selected root and complete bounded API/caller/ownership/provenance design:

```json
{
  "id": "C018-owned-connect-dispatch",
  "criterion": "P20-MECH-HTTP-005",
  "predicate": "The controlled HttpClient path hands every byte after a successful CONNECT head to the next protocol owner exactly once; proxy and target TLS remain correctly scoped, with the same frozen source configuration, pool ownership, typed failure, redirects/retry/cancellation and ordinary upstream HTTP acceptance profile.",
  "selected_mechanism": "One SDK-owned transport dispatch beneath the existing HttpClient seam, retaining ordinary reqwest0.12.28/Hyper1.9.0/hyper-util0.1.20/rustls0.23.40; use ordinary HTTP CONNECT+public Response.upgrade instead of reqwest automatic TLS tunnel for controlled HTTPS proxy paths. No fork, loopback intermediary, custom parser, signer or public transport trait.",
  "proofs": {
    "red": "C:\\Users\\Luiz\\AppData\\Local\\Temp\\opi-http005-overread-k6nnyhhs\\probe-identities.json",
    "red_qualification": "C:\\Users\\Luiz\\AppData\\Local\\Temp\\opi-http005-protocol-qualification-0uw_lin0\\qualification.json",
    "public_typecheck": "C:\\Users\\Luiz\\AppData\\Local\\Temp\\opi-p20-ready-final-dp5e95fw\\transport-typecheck-receipt.json"
  },
  "interfaces": [
    "HttpClient retains construction/configuration/pool metadata. Its client()/private websocket_client() return a concrete owned HttpTransport handle rather than bare reqwest::Client. The handle captures controlled frozen configuration or explicitly Injected(reqwest::Client); injected handles forward exactly and are never reconfigured or double-resolved.",
    "HttpRequestBuilder delegates URL/method/header/body/json/form/query/timeout/version/build/try_clone encoding to the ordinary reqwest builder, but send routes through the captured transport owner. Implement only the actual SDK/product/documented method inventory; do not add a generic framework. All existing public raw-client injection functions accept the concrete handle or a standard Into conversion, preserving explicit raw-client ownership.",
    "Keep reqwest::Response at response/callback boundaries. Public TryFrom<reqwest::Request> produces the Hyper request. Public Body::wrap(Incoming) preserving body frames/trailers and ResponseBuilderExt::url plus From<http::Response> preserve URL/status/version/headers/extensions/streaming body. Typed HttpTransportError preserves underlying reqwest/Hyper/TLS/IO causes and timeout/connect stages; Preserve the existing public ProviderError categories and redacted ProviderErrorSummary mapping, including Network/Timeout. Keep typed HttpTransportError causes at the actual transport/WS/ADC/AWS owners; do not invent a ProviderError::Http variant or forge a reqwest::Error.",
    "No new universal proxy timeout or public timeout field is introduced. The pinned Codex WS15s remains at its existing WS connection scope. Pi helper/Bedrock have no such shared15s deadline; ADC and other owners retain their exact existing per-request/source timing."
  ],
  "algorithm": [
    "For direct or plain HTTP targets use the existing controlled reqwest path. For controlled HTTPS via a configured HTTP/HTTPS proxy, create an ordinary HTTP-target Method::CONNECT request whose URI authority is the target host:port. This avoids the internal automatic HTTPS Tunnel path. Proxy authorization is proxy-only; target credentials are never copied into CONNECT. Outer proxy CONNECT is explicitly H1; the target selected ALPN remains independent and is not globally downgraded.",
    "Verify outer HTTPS-proxy TLS against proxy identity first using the frozen source trust/profile. Require successful CONNECT and obtain Response.upgrade(); Hyper1.9 splits the response head and preserves remaining read_buf in Rewind. Feed that Upgraded IO to target tokio-rustls using target SNI/trust and the selected target client identity. No proxy-failure direct fallback or hidden ambient lookup.",
    "Use a private Hyper legacy client/pool connector over that upgraded/TLS IO for target requests. Its pool belongs to the immutable transport identity and target/protocol/profile; a new source/TLS/proxy snapshot gets a different owner. Preserve actual selected H1/H2 ALPN,10idle/90s defaults or explicit existing settings, and separate classic Upgrade H1 pool. No global provider installation.",
    "The controlled handle remains the sole transport policy owner. Preserve configured RedirectPolicy/method/body/header behavior with exact existing reqwest-profile/source-specific rules; None admits one leg. No backend retry is introduced. Only existing source-owned connection timeouts cover their selected acquisition scope; request/body timeout and cancellation keep their existing scope.",
    "Owned connection/response-body futures retain backpressure and drop/cancel semantics. Partially received content/usage/id and error-owner stage survive; no extra callbacks, reauth, cross-identity send or late publication. Idle reusable connections remain owned by the shared transport; one Models cleanup never cancels another instance.",
    "Migrate all actual controlled client/send consumers together, including10chat,nonchat,catalog,ADC/AWS exchanges,OAuth flows and WS Upgrade. Explicit raw injected clients remain the separate forwarding variant. There is one SDK HttpClient seam, not a retained parallel SDK API."
  ],
  "dependency_impact": [
    "Promote already locked tokio-rustls/hyper/hyper-util/http-body-util edges from SDK dev to runtime, reusing root workspace declarations and exact lock versions/checksums. No new crate family, native/proc-macro implementation, version bump, HTTP fork or namespace.",
    "Move server-only Hyper/Hyper-util features from shared root defaults to SDK dev edges so ordinary library runtime does not gain a server role; retain actual client/http1/http2/tokio/pool feature needs. Explain/review metadata and unexpected Cargo-generated lock changes. Existing native AWS-LC/rustls instance ownership remains; all affected package ordinary/feature unions/native receipts are regenerated."
  ],
  "matched_red_green": "Preserve original CONNECT200+same seven-byte TLS alert schedules and error-owner assertions; disclose only mechanical public-type/struct-literal API migration, never relabel the trace a normal successful TLS handshake. The separated control observed only a ClientHello first-byte synchronization, not full negotiation.",
  "cases": [
    "Original coalesced versus separated negative byte-handoff test yields the same target TLS failure instead of CONNECT EOF; assert no lost/duplicated suffix and exact proxy sends.",
    "Successful real local target TLS through HTTP and HTTPS proxy, plus target/proxy trust/SNI negative controls, mTLS source-audience isolation, selected H1/H2 ALPN and classic WSS Upgrade/overread.",
    "Fragmented/coalesced CONNECT head, non200/auth rejection/EOF/cancel/timeout, no direct fallback and safe typed error/no credentials in debug.",
    "Pool reuse and different frozen profile isolation; unread body/drop/acquisition cancel/partial error; per-instance cleanup with another caller active.",
    "Redirect method/body/header/source-leg counts and no hidden backend retries; raw/payload/response callback counts; all public wire/source/caller consumers compile and execute.",
    "Explicit injected reqwest client retains its own proxy/redirect/TLS semantics with zero automatic wrapping/resolution."
  ],
  "verification": [
    "Exact old red and new owned-transport public tests",
    "All changed SDK/Agent/product integration targets and workspace fmt/clippy/all-targets/doc-test/rustdoc",
    "B91 ordinary and caller-feature HTTP/crypto/WS, B17/B95 real external public SDK consumers, B92 native1.97.1 matrix",
    "Doc checker and diff check"
  ],
  "changed_paths": [
    "CHANGELOG.md",
    "Cargo.lock",
    "Cargo.toml",
    "crates/opi-ai/Cargo.toml",
    "crates/opi-ai/README.md",
    "crates/opi-ai/README.zh.md",
    "crates/opi-ai/src/anthropic.rs",
    "crates/opi-ai/src/api/llama.rs",
    "crates/opi-ai/src/api/openrouter_images.rs",
    "crates/opi-ai/src/api/system_one.rs",
    "crates/opi-ai/src/auth/adc/native.rs",
    "crates/opi-ai/src/auth/anthropic_federation.rs",
    "crates/opi-ai/src/auth/aws/source_http.rs",
    "crates/opi-ai/src/auth/oauth/anthropic.rs",
    "crates/opi-ai/src/auth/oauth/chatgpt.rs",
    "crates/opi-ai/src/auth/oauth/codex.rs",
    "crates/opi-ai/src/auth/oauth/copilot.rs",
    "crates/opi-ai/src/auth/oauth/kimi_coding.rs",
    "crates/opi-ai/src/auth/oauth/meta.rs",
    "crates/opi-ai/src/auth/oauth/mod.rs",
    "crates/opi-ai/src/auth/oauth/openrouter.rs",
    "crates/opi-ai/src/auth/oauth/radius.rs",
    "crates/opi-ai/src/auth/oauth/xai.rs",
    "crates/opi-ai/src/azure_openai.rs",
    "crates/opi-ai/src/bedrock/mod.rs",
    "crates/opi-ai/src/gemini.rs",
    "crates/opi-ai/src/http.rs",
    "crates/opi-ai/src/http/proxy_tunnel.rs",
    "crates/opi-ai/src/http/request.rs",
    "crates/opi-ai/src/http/transport.rs",
    "crates/opi-ai/src/mistral.rs",
    "crates/opi-ai/src/openai_chat.rs",
    "crates/opi-ai/src/openai_codex_responses.rs",
    "crates/opi-ai/src/openai_responses.rs",
    "crates/opi-ai/src/pi_messages.rs",
    "crates/opi-ai/src/provider.rs",
    "crates/opi-ai/src/vertex.rs",
    "crates/opi-ai/src/ws.rs",
    "crates/opi-ai/tests/http_acceptance.rs",
    "crates/opi-ai/tests/oauth_flows.rs",
    "crates/opi-ai/tests/platform_delivery.rs",
    "crates/opi-ai/tests/proxy_support.rs",
    "crates/opi-ai/tests/tls_contract.rs",
    "crates/opi-ai/tests/ws_upgrade.rs",
    "crates/opi-coding-agent/src/harness.rs",
    "crates/opi-coding-agent/src/interactive.rs",
    "crates/opi-coding-agent/src/interactive_auth.rs",
    "crates/opi-coding-agent/src/oauth.rs",
    "crates/opi-coding-agent/tests/interactive_auth.rs",
    "crates/opi-coding-agent/tests/interactive_tui_auth.rs",
    "crates/opi-coding-agent/tests/oauth_auth.rs",
    "crates/opi-coding-agent/tests/test_hermeticity.rs"
  ],
  "test_impact": {
    "add": "owned proxy transport handoff/lifecycle/public-client tests",
    "update": "mechanical controlled handle/type/error callers and package graph assertions",
    "retain": "existing wire,source,raw-injected,secret,HTTP-profile,ALPN and cancellation controls",
    "delete": "none except implementation made unused by this exact source-preserving cutover"
  },
  "observed_green": "not-run",
  "review_corrections": [
    "No universal15s Pi/proxy/Bedrock deadline; only actual CodexWS source has15s.",
    "Synchronize native ADC/AWS map_reqwest_error typed signatures.",
    "Preserve complete body frames/trailers via Body::wrap(Incoming), not a data-only stream."
  ]
}
```

Independent source-contract review: review.json SHA256 `49da80ccbf387bb47dcec1db1b0d4cd0dd1b0300200f53e4f80824a56b97a7b9`. No HTTP fork/new transport trait/runtime role; existing ordinary reqwest/Hyper selected profile and single source policy owner retained. Only actual Codex WS has15s connect budget; no universal proxy/Bedrock deadline. Body::wrap(Incoming) preserves frames/trailers; final no-network typecheck of that path passes. Actual wire/pool/cancel/ALPN/error/native behavior remains future green.

Exact Upgrade caller is src/ws.rs (not a new ws/upgrade.rs module). Synchronize WsUpgradeError::Transport to the selected typed HttpTransportError while preserving original status/handshake/cancel distinctions. reqwest::Response::upgrade still returns reqwest::Error, so retain an explicit conversion for that path; do not expect transitive From chaining. Synchronize ADC/AWS map_reqwest_error and all actual reqwest::Error callers listed above.

For explicitly configured legacy ProxyConfig.no_proxy, reuse the same public hyper-util0.1.20 client::proxy::matcher::Matcher::builder().all(proxy).no(raw_no_proxy).build().intercept(uri) used by locked reqwest0.12.28. Validate selected proxy URL before matcher creation and never call from_env/from_system. This preserves the existing backend grammar without duplicating it or silently treating bypass as a plaintext CONNECT. Pi/gaxios source resolvers already decide their own dialect and pass no_proxy=None; their grammars are not merged into this legacy backend matcher. Preserve proxy/connection metadata through public connection extras captured from the outer response where required, without fabricating target metadata. Final compile-only proof also covers the public matcher API and Body::wrap(Incoming); no network or behavior execution is claimed.

### Batch B28: freeze scoped proxy selection for default ADC

**Closure predicate**: Default builtin ADC source exchanges use one frozen selected-context gaxios proxy/bypass snapshot; malformed selected proxy fails before direct send; explicitly injected ADC transport remains unchanged.
**Dependencies**: ["B99"]
**Verification union**: cargo test -p opi-ai --test adc_proxy_contract --offline --locked; affected ADC/source/mTLS/builtin targets; applicable final workspace gates and exact changed targets; documentation and whitespace checks.

#### Fix B28.1: freeze scoped proxy selection for default ADC

- **Finding source(s)**: phase20-codex-gpt61-b3f1810-20261103t142820z / 4cec97af8ef6bb773c630ee251728f31212e12fdc1bf9631f10b2dc3084bc0f6 / P20-AUD-010
- **Decision**: fix:freeze-scoped-proxy-selection-for-default-ADC
- **Verification status**: Partially confirmed
- **File(s)**: `crates/opi-ai/src/auth.rs`, `crates/opi-ai/src/auth/builtin.rs`, `crates/opi-ai/src/auth/builtin_vertex.rs`, `crates/opi-ai/src/auth/adc/native.rs`, `crates/opi-ai/src/auth/adc/proxy.rs`, `crates/opi-ai/src/auth/adc/external.rs`, `crates/opi-ai/src/auth/adc/impersonation.rs`, `crates/opi-ai/tests/adc_proxy_contract.rs`, `crates/opi-ai/README.md`, `crates/opi-ai/README.zh.md`, `CHANGELOG.md`
- **Change kind**: behavioral
- **Change**: Distinguish private default-built ADC ownership from supplied transports, retain raw presence for gaxios nullish bypass selection, and freeze selected source proxy configuration at acquisition rather than construction.
- **Closure predicate**: Default builtin ADC source exchanges use one frozen selected-context gaxios proxy/bypass snapshot; malformed selected proxy fails before direct send; explicitly injected ADC transport remains unchanged.
- **Red-before**: FAIL: python C:\Users\Luiz\AppData\Local\Temp\opi-phase20-remediate-plan-jqdki4yk\source\scripts\opi-cargo-cache.py run --target E:\opi\cargo-targets\opi-72fa37a0fa044958 -- test --manifest-path C:\Users\Luiz\AppData\Local\Temp\opi-p20-adc-proxy-red-g6m8y3c_\Cargo.toml --offline --lib -- --nocapture; 8executed/4passed/4failed/0ignored, exit101; default route sends direct instead of HTTPS proxy, has wrong uppercase/target precedence, treats star as bypass, and sends direct for an invalid selected proxy. Four independent controls pass. Receipt `C:\Users\Luiz\AppData\Local\Temp\opi-p20-adc-proxy-red-g6m8y3c_\receipt-final.json` SHA256 `a07eeab60983366f89615fb356131ebc9e06da86021650fb879ea9574bcc41eb`.
- **Green-after**: cargo test -p opi-ai --test adc_proxy_contract --offline --locked; affected ADC/source/mTLS/builtin targets -> Default builtin ADC source exchanges use one frozen selected-context gaxios proxy/bypass snapshot; malformed selected proxy fails before direct send; explicitly injected ADC transport remains unchanged. Observed: not-run.
- **Test impact**: add the exact discriminating cases; update affected public callers and fixtures; retain valid-source, typed-failure, cancellation and secret-canary controls; delete none.

One behavioral root: default builtin ADC transport has no selected-context proxy snapshot. Closure predicate: default source exchanges select the frozen gaxios proxy/bypass policy from the injected source context; malformed selected proxies fail before any direct send; injected ADC transport remains unchanged. Keep GET Limited20, token POST None, existing retries/timeout/cancel and source credential identity rules.

Concrete private design: distinguish default-built ADC transport ownership from AdcNativeOwner::new's supplied transport at factory construction. Capture a redacted immutable GaxiosProxySnapshot during actual selected-source snapshot preparation, never construction/check_configured, and carry it to every owned exchange. ReqwestAdcTransport receives explicit resolved ProxyConfig per target/leg; backend ambient discovery remains disabled. Use HTTPS_PROXY -> https_proxy -> HTTP_PROXY -> http_proxy independently of target scheme; no ALL_PROXY. Bypass selection follows gaxios's NO_PROXY ?? no_proxy and exact origin/host/href or leading-dot suffix, not reqwest's/Pi's generalized grammar. Decide bypass before proxy URL admission. A selected invalid proxy never becomes None/direct.

Exact implementation paths for this root: auth/builtin_vertex.rs (mark default versus injected owner and bind selected context); auth/adc/native.rs (freeze snapshot and route owned source sends); new private auth/adc/proxy.rs (gaxios policy/grammar); auth/adc/external.rs (certificate-source owned STS transport must preserve the same frozen proxy while limiting client identity to STS). Use a concrete presence-preserving AuthContext::env_raw(name) method in auth.rs, with a default implementation delegating to self.env(name) so injected contexts never escape to real ambient sources. AmbientAuthContext overrides it to retain present empty/whitespace values and typed non-Unicode failures; ScopedContext in auth/builtin.rs returns stored raw values first and otherwise calls base.env_raw. Existing env/identity behavior is unchanged. This is the minimum representational addition needed for gaxios NO_PROXY ?? no_proxy; do not change existing identity masking. Add a private default-owner constructor/mode for AdcNativeOwner; its public new(selection, supplied_transport, clock) still marks the transport injected. AdcSnapshot carries the frozen default proxy context; private exchange_family/embedded/external/impersonation call plumbing forwards it to the certificate-only STS builder, without adding fields to public AdcNativeInput or wrapping supplied transports. Include auth/adc/impersonation.rs only for that private nested-source forwarding. This raw-presence requirement is concrete implementation work, not a new proxy policy.

Preserve existing credential/token cache keys, single-flight ownership, unique refresh material, real expiry and ACK behavior. The actual acquisition owner freezes its transport once; its retries/subordinate source legs reuse it. Cached/coalesced token results are not additional HTTP sends. Pools built for a later different proxy snapshot must not reuse the prior transport. Do not invent a credential-cache policy merely to implement source proxy routing. Keep injected source owner unchanged. Candidate tests: add these8actual public cases to builtin_scoped_env.rs or a dedicated adc_proxy_contract.rs; update ADC source/mTLS/owner cases only where modified. Document changed default source behavior in both crate READMEs and Unreleased. The eight-case baseline is the matched green-after minimum; add empty/nullish precedence and snapshot-retry/mTLS controls for new branches. No green-after has run.

### Batch B97: resolve scoped target proxy inputs through the public utility

**Closure predicate**: The source-qualified Pi helper selects the exact target/scoped/ambient/NO_PROXY/ALL rules from immutable maps and its caller-owned HTTP/WS/AWS routes use that one frozen result; supplied transports and Reference Product host defaults retain their own ownership.
**Dependencies**: ["B99"]
**Verification union**: cargo test -p opi-ai --test proxy_support --test proxy_source_contract --offline --locked; cargo test -p opi-coding-agent --lib config::tests --offline --locked; same external3case probe with disclosed signature shim; all changed targets and applicable workspace fmt/clippy/all-targets/doc-test/rustdoc; doc and diff checks.

#### Fix B97.1: resolve scoped target proxy inputs through the public utility

- **Finding source(s)**: phase20-codex-gpt61-b3f1810-20261103t142820z / 4cec97af8ef6bb773c630ee251728f31212e12fdc1bf9631f10b2dc3084bc0f6 / P20-AUD-010
- **Decision**: fix:resolve-scoped-target-proxy-inputs-through-the-public-utility
- **Verification status**: Partially confirmed
- **File(s)**: `crates/opi-ai/src/http.rs`, `crates/opi-ai/tests/proxy_support.rs`, `crates/opi-ai/tests/proxy_source_contract.rs`, `crates/opi-coding-agent/src/config.rs`, `crates/opi-ai/README.md`, `crates/opi-ai/README.zh.md`, `CHANGELOG.md`
- **Change kind**: behavioral
- **Change**: Apply only the selected source-preserving root below; retain explicit transport/identity authority and all existing valid-input controls.
- **Closure predicate**: The source-qualified Pi helper selects the exact target/scoped/ambient/NO_PROXY/ALL rules from immutable maps and its caller-owned HTTP/WS/AWS routes use that one frozen result; supplied transports and Reference Product host defaults retain their own ownership.
- **Red-before**: FAIL: D:\Users\Luiz\anaconda3\python.exe C:\Users\Luiz\AppData\Local\Temp\opi-phase20-remediate-plan-jqdki4yk\source\scripts\opi-cargo-cache.py run --target E:\opi\cargo-targets\opi-72fa37a0fa044958 -- test --manifest-path C:\Users\Luiz\AppData\Local\Temp\opi-p20-proxy-utility-red-xb4aauuz\Cargo.toml --offline --test public_proxy -- --nocapture;3tests2passed/1failed/0ignored,Cargo101,build succeeded. ALL_PROXY-only returns no proxy; HTTP_PROXY-only and empty-env controls pass.258registry identities match. Receipt `C:\Users\Luiz\AppData\Local\Temp\opi-p20-proxy-utility-red-xb4aauuz\receipt.json` SHA256 `d695afff2823bcd35e6403bf8b3a28a1fcf5db6f0806ecb3ec3b322527cc0126`. Matched green preserves public_proxy.rs and all3input/output cases; only the helper invocation shape migrates as explicitly described below.
- **Green-after**: cargo test -p opi-ai --test proxy_support --test proxy_source_contract --offline --locked; cargo test -p opi-coding-agent --lib config::tests --offline --locked; same external3case probe with disclosed signature shim -> The source-qualified Pi helper selects the exact target/scoped/ambient/NO_PROXY/ALL rules from immutable maps and its caller-owned HTTP/WS/AWS routes use that one frozen result; supplied transports and Reference Product host defaults retain their own ownership. Observed:not-run.
- **Test impact**: add the named discriminating cases; update exact mechanical API callers; retain valid-source/typed-failure/secret/cancel controls; delete only implementation made unused by this exact replacement.

# C010 narrowed closure design and actual red

This supersedes the unresolved constructor assumption in ownership-correction.md. It records a bounded plan recommendation, not product implementation or observed green.

## Decision and authority

Keep C010 Partially confirmed, original/final Major. The confirmed defect is the incomplete public source-qualified proxy resolver capability and separately the real default ADC source transport. Narrow the claim that any supplied plain client must automatically consume ResolvedAuth.env: no such override is established by SDK-contracts L193 or readiness L39, and both preserve custom-client ownership. SDK-contracts L9 permits Rust naming/type substitutions and separates SDK mechanism from product default assembly. The required utility and caller-owned HTTP/WS/AWS integration can therefore close without a new builtin default-client constructor. This is implementation shape of the already selected behavior, not a new product-authority request.

C010 closure is the conjunction of two roots: (U) source-qualified Pi resolver with explicit frozen inputs and caller-owned actual route conformance; (A) source-qualified default ADC gaxios transport, with public injected source transports unchanged. The existing ADC proof and design remain separate. Do not merge gaxios precedence/NO_PROXY/ALL rules into the Pi resolver.

## U: concrete candidate API and migration

Evolve the existing public helper (a deliberate 0.x API migration), selected exact public signature:

```rust
pub fn proxy_from_env(
    target: &str,
    scoped: Option<&std::collections::HashMap<String, String>>,
    ambient: &std::collections::HashMap<String, String>,
) -> Result<ProxyConfig, ProxyResolutionError>;
```

These are raw immutable snapshots, not an implicit process lookup. The injected ambient map is the entire fallback authority; no hidden std::env lookup is permitted. Scoped lower, scoped upper, ambient lower, ambient upper is a single nonempty-string chain. Empty string falls through; do not trim before deciding source truthiness. Target protocol selection then ALL_PROXY and target-specific NO_PROXY are resolved before parsing the chosen proxy. Preserve actual Pi invalid-target/no-proxy result, scheme normalization, unsupported-proxy and invalid-proxy errors, using redacted typed errors. A resolved ProxyConfig is target-qualified and must not be reused for a different target without resolving against the same frozen snapshots. Do not forward Pi NO_PROXY syntax to reqwest and assume the grammars coincide.

Keep `resolve_proxy` as its explicitly documented legacy value-selection function if still useful; do not silently claim it is target-aware. Migrate coding-agent/config.rs to a small private ambient-reading wrapper around that existing legacy rule. Preserve its existing uppercase-first, HTTPS-first, HTTP(S)/NO_PROXY-only host policy and explicit-config precedence; changing the SDK utility must not implicitly add ALL_PROXY to Reference Product defaults. Update the public utility's callers, rustdoc, tests, bilingual crate docs and Unreleased entry together.

No additional production provider constructor or transport trait is required for this narrowed capability. The caller explicitly resolves source input, builds one HttpClient from the resulting declared ProxyConfig/TLS, and passes that owned client to the existing HTTP, Codex and Bedrock constructors. These are caller-owned source assemblies, not default builtin discovery. Already supplied clients stay untouched.

## Actual red and matched green domain

Command and exact inputs are in receipt.json. Build succeeded. Cargo exit101; 3 tests executed, 2 passed, 1 failed, 0 ignored. 258 registry identities exactly match the committed workspace lock; no unexpected registry dependency. The probe has no network and uses env_clear child processes without parent environment mutation.

- Empty env: actual `{url:null,no_proxy:null}`; PASS.
- Only HTTP_PROXY=`http://127.0.0.1:17891`: actual configured URL; PASS.
- Only ALL_PROXY with the same URL: actual `{url:null,no_proxy:null}`; FAIL against the registered fallback predicate.

The matching green preserves `tests/public_proxy.rs` byte-for-byte, all three child envs, expected JSON and test names. Only `src/main.rs` adapts the deliberately migrated call signature: collect the child's explicit ambient snapshot, empty scoped map, fixed valid `http://target.example` target, call the migrated helper and print the same result JSON. All three cases lie in the common target-independent input/output domain; no scoped value or target conflict is introduced to make a different test pass. Record this invocation-shape migration and both source identities explicitly. It is not evidence that a scoped input or automatic builtin consumer was exercised before the change.

A failure on the currently public ALL fallback is a real behavioral red for the resolver root. Static current absence of target/scoped inputs supplements it; no compiler failure or relabelled injected client is counted. New target/scoped/route cases below are required green-after coverage of that same resolver capability, not falsely claimed baseline observations.

## Future checks: all not-run

| Case group | Actual assembly / input | Required predicate |
|---|---|---|
| Matched public domain | Same three env_clear child cases, migrated main call only | All three pass; legacy ALL failure becomes selected URL |
| Snapshot precedence | Scoped lower and upper both present; scoped upper vs ambient lower; empty scoped values; lower/upper empty fallback; HTTP vs HTTPS vs ALL; maps with both cases on Windows | Exact single source chain and target-specific choice; no process env consulted |
| Target and NO_PROXY | Domain boundary, suffix, explicit/mismatched port, bracketed IPv6, universal star, comma/space separators | Match pinned helper, including no substring-boundary bypass |
| Validation order | Invalid selected URL and SOCKS/PAC; bypass matching target with an invalid proxy URL; schemeless proxy; invalid target | Typed rejection with zero sends where source rejects; bypass-before-parse and invalid-target result match source |
| HTTP caller-owned route | Public resolver -> HttpClientBuilder -> actual HttpClient request to local origin via local explicit proxy | Proxy1/origin0; bypass origin1/proxy0; invalid proxy zero/zero; no ambient rediscovery |
| Codex caller-owned WS route | Map ws/wss to http/https before resolver, build frozen client, existing OpenAiCodexResponsesProvider -> actual local WS peer | Proxy route is reached once; no direct fallback on failed proxy; H1 Upgrade; prepared client's route unchanged after caller maps change; separate client/connection ownership when route identity changes |
| Bedrock caller-owned generation route | Public resolver -> explicitly built H1 proxy client -> existing BedrockProvider with synthetic prepared credentials | Actual ConverseStream/eventstream exchange uses proxy, zero direct bypass, correct frozen signature/endpoint; retain ordinary no-proxy H2 acceptance separately |
| Custom-client controls | Already built supplied HTTP/Codex/Bedrock clients, including explicit no-proxy and explicit proxy | Exact supplied client ownership; zero resolver reads/wrapping caused by the adapter; no hidden override from ResolvedAuth.env |
| Host migration control | coding-agent private legacy wrapper and explicit config | Existing host uppercase/HTTPS preference, no ALL addition and explicit-config precedence unchanged |

The same fixed snapshot survives retries. Actual source route tests must count sends and exclude credential/environment access outside the selected owner. Tests should use local fixtures, synthetic credentials and serialized/child-only environment setup. These tests do not claim an automatic Models source-proxy path, a Google-generation policy or AWS credential-chain proxy policy.

## Exact candidate changed paths for U

- crates/opi-ai/src/http.rs (public helper, private pure resolver/error, docs)
- crates/opi-ai/tests/proxy_support.rs (public signature migration, retained declared client/legacy-rule controls)
- crates/opi-ai/tests/proxy_source_contract.rs (new scoped/target and actual caller-owned HTTP/WS/AWS matrix)
- crates/opi-coding-agent/src/config.rs (private legacy ambient wrapper and bounded preservation tests)
- crates/opi-ai/README.md
- crates/opi-ai/README.zh.md
- CHANGELOG.md

No Cargo manifests/lockfile, provider constructor, provider_env identity semantics or default builtin registration need change for U. Tests: update proxy_support; add proxy_source_contract and the focused config preservation checks; retain existing provider/H2/injected transport tests.

Required focused commands after implementation: cache-wrapped `test -p opi-ai --test proxy_support`; `test -p opi-ai --test proxy_source_contract`; `test -p opi-coding-agent --lib config::tests` (use the actual added filter, not a zero-match success); the matched external three-case check with disclosed API shim migration. Run bilingual doc checker and git diff --check. The cross-crate API migration additionally takes the applicable workspace fmt/clippy/all-targets/doc-test/rustdoc verification union already owned by the final plan.

## A: separate existing default ADC root

Reuse ../opi-p20-adc-proxy-red-g6m8y3c_/receipt-final.json and the corrected ADC source design: default builtin Vertex ADC, injected AuthContext, actual default ReqwestAdcTransport; 8 executed / 4 pass / 4 fail. The source policy is gaxios7.1.4 (HTTPS upper/lower before HTTP upper/lower, no ALL_PROXY, distinct NO_PROXY semantics). Injected AdcSourceTransport stays untouched. Reproduce these eight cases and full raw-empty/nullish/frozen retry/mTLS inheritance controls after implementation. U and A share C010 identity, not a root or policy.

Remaining obligations for this selected narrowed design are green-after checks, all explicitly not-run at plan stage. They are implementation acceptance work, not missing plan-stage evidence. If a later plan adds automatic builtin proxy discovery, that is outside this bounded U decision and needs its own concrete source/predicate/evidence assessment rather than being smuggled into this red.

Exact invalid target oracle: unparseable/missing-host target returns no selected proxy, matching parseProxyTargetUrl/getProxyForUrl; it does not send or parse an otherwise invalid proxy. A valid target selecting an invalid/unsupported proxy returns redacted typed error, unless NO_PROXY bypass was selected first.


The target-qualified helper result sets no_proxy=None after applying the pinned bypass grammar itself, preventing a second reqwest-grammar decision. Caller-owned source assembly disables automatic redirects for its helper-owned per-leg route; any permitted next URL is resolved from the same frozen maps, with no fresh ambient lookup. Existing explicitly injected clients retain their own redirect/proxy responsibility. Invalid/missing-host target selects no proxy; a valid target with a selected invalid proxy yields a redacted typed error unless bypass applied first.

### Batch B91: require complete named extracted package conformance

**Closure predicate**: Actual Cargo-produced extracted opi-ai resources and fresh ordinary/feature-union consumers execute every declared named Schema/HTTP/crypto/WS/README case with correct semantic results; private and public test corpora are separately counted, exact-case complete and zero-ignored, with source/graph/native identities.
**Dependencies**: ["B13", "B02", "B14", "B03", "B04", "B05", "B06", "B07", "B08", "B09", "B10", "B11", "B12", "B15", "B16", "B19", "B22", "B23", "B24", "B25", "B26", "B27", "B29", "B30", "B32", "B33", "B34", "B35", "B36", "B37", "B38", "B39", "B40", "B41", "B42", "B43", "B44", "B45", "B46", "B47", "B50", "B51", "B52", "B53", "B54", "B55", "B57", "B58", "B59", "B60", "B61", "B63", "B64", "B65", "B66", "B67", "B68", "B69", "B70", "B71", "B72", "B73", "B74", "B75", "B76", "B77", "B78", "B79", "B80", "B81", "B82", "B83", "B84", "B85", "B86", "B87", "B88", "B89", "B90", "B94", "B18", "B20", "B48", "B49", "B21", "B56", "B98", "B31", "B62", "B96", "B99", "B28", "B97"]
**Verification union**: python -m unittest discover -s scripts/pi-sdk-conformance -p test_*.py -v; real negative CLI/subprocess wrong-package,missing/extra/duplicate/ignored/filtered/failedJSON exit0/source-drift cases; run_early_package_proof --scratch <fresh> --target-dir <selected> --cache-workspace <owner>; package_slice --family http|crypto|ws --offline --scratch <fresh-each> --target-dir <selected> --cache-workspace <owner>; run_readme_examples --offline --scratch <fresh> --target-dir <selected> --cache-workspace <owner>; actual expected schema73+finitecases/profile,http6/profile,crypto6/profile,ws10/profile,README3flows; owning changed targets/clippy/doc/diff

#### Fix B91.1: require complete named extracted package conformance

- **Finding source(s)**: phase20-codex-gpt61-b3f1810-20261103t142820z / 4cec97af8ef6bb773c630ee251728f31212e12fdc1bf9631f10b2dc3084bc0f6 / P20-AUD-018; phase20-pi-glm53-44374ec-20261103t160703z / f8c06b50b3c51c95ac5b1309e72a8b31d06c34785ad588ed4e42bff19d914990 / P20-AUD-007
- **Decision**: fix:require-complete-named-extracted-package-conformance
- **Verification status**: Partially confirmed
- **File(s)**: `scripts/pi-sdk-conformance/run_early_package_proof.py`, `scripts/pi-sdk-conformance/package_slice.py`, `scripts/pi-sdk-conformance/run_readme_examples.py`, `scripts/pi-sdk-conformance/test_package_proof.py`, `scripts/pi-sdk-conformance/test_package_slice.py`, `scripts/pi-sdk-conformance/test_readme_examples.py`, `scripts/pi-sdk-conformance/consumer/src/main.rs`, `scripts/pi-sdk-conformance/consumer-http/src/main.rs`, `scripts/pi-sdk-conformance/consumer-crypto/src/main.rs`, `scripts/pi-sdk-conformance/consumer-ws/src/main.rs`, `scripts/pi-sdk-conformance/evidence/upstream-package-proof.json`, `scripts/pi-sdk-conformance/evidence/http-package-proof.json`, `scripts/pi-sdk-conformance/evidence/crypto-package-proof.json`, `scripts/pi-sdk-conformance/evidence/ws-package-proof.json`, `scripts/pi-sdk-conformance/evidence/readme-examples-proof.json`
- **Change kind**: test-only
- **Change**: Replace minimum-count-only WS admission with required named-set discovery bound to sealed package test/fixture source and target cfg, then require exact executed-set equality and zero ignored/filtered. Add the registered finite public Schema cases to the actual consumer, retaining current73. Public test/fixture copies are source-hash-bound and use public SDK only. Reject any failed JSON check even with process exit0. Record real ordinary and caller-union resolution, resources, registry checksums, native build/source/tool/output/link inputs. Preserve safe extraction/config/native guards.
- **Closure predicate**: Actual Cargo-produced extracted opi-ai resources and fresh ordinary/feature-union consumers execute every declared named Schema/HTTP/crypto/WS/README case with correct semantic results; private and public test corpora are separately counted, exact-case complete and zero-ignored, with source/graph/native identities.
- **Red-before**: Not applicable to production behavior: one bounded harness/proof-collection root. Current source inventory establishes the missing exact route/admission; synthetic unit/subprocess negatives prove guard behavior only and are never labeled native/product acceptance.
- **Green-after**: python -m unittest discover -s scripts/pi-sdk-conformance -p test_*.py -v; real negative CLI/subprocess wrong-package,missing/extra/duplicate/ignored/filtered/failedJSON exit0/source-drift cases; run_early_package_proof --scratch <fresh> --target-dir <selected> --cache-workspace <owner>; package_slice --family http|crypto|ws --offline --scratch <fresh-each> --target-dir <selected> --cache-workspace <owner>; run_readme_examples --offline --scratch <fresh> --target-dir <selected> --cache-workspace <owner>; actual expected schema73+finitecases/profile,http6/profile,crypto6/profile,ws10/profile,README3flows; owning changed targets/clippy/doc/diff -> Actual Cargo-produced extracted opi-ai resources and fresh ordinary/feature-union consumers execute every declared named Schema/HTTP/crypto/WS/README case with correct semantic results; private and public test corpora are separately counted, exact-case complete and zero-ignored, with source/graph/native identities. Observed:not-run.
- **Test impact**: add/update exact harness guards and finite execution cases; retain controls; delete none.

### Batch B17: execute assembled sdk matrix from extracted package

**Closure predicate**: The actual locally packaged opi-ai artifact is the only Opi runtime dependency of a fresh outside-workspace ordinary default-feature consumer. Its exact copied assembled test source discovers all19 named cases and executes every one with 1passed,0failed,0ignored; every public-boundary predicate below holds. Metadata, resolved graph/checksums/features, packaged source/resource hashes, executable identity, raw logs and original exit codes bind evidence to the candidate source. Missing, filtered, ignored, extra, compile-only, substituted-package or product-dependent runs are rejected.
**Dependencies**: ["B13", "B02", "B14", "B03", "B04", "B05", "B06", "B07", "B08", "B09", "B10", "B11", "B12", "B15", "B16", "B19", "B22", "B23", "B24", "B25", "B26", "B27", "B29", "B30", "B32", "B33", "B34", "B35", "B36", "B37", "B38", "B39", "B40", "B41", "B42", "B43", "B44", "B45", "B46", "B47", "B50", "B51", "B52", "B53", "B54", "B55", "B57", "B58", "B59", "B60", "B61", "B63", "B64", "B65", "B66", "B67", "B68", "B69", "B70", "B71", "B72", "B73", "B74", "B75", "B76", "B77", "B78", "B79", "B80", "B81", "B82", "B83", "B84", "B85", "B86", "B87", "B88", "B89", "B90", "B18", "B20", "B48", "B49", "B21", "B56", "B31", "B62", "B28", "B91"]
**Verification union**: python -m unittest discover -s scripts/pi-sdk-conformance -p test_external_sdk.py -v: accept exact case list and all19 individual successes; reject missing/duplicate/extra names, exit101, zero executed, ignored1, filtered-all, no execution log, multiple/wrong executable artifacts, wrong SDK manifest, extra opi-* runtime crate, path/git/patch/replace dependency, source test byte drift, private include, reused/in-repo scratch.; CLI negative: python scripts/pi-sdk-conformance/run_external_sdk.py --scratch <new-external-negative-path> --target-dir <selected-cache> (omit --offline); assert nonzero argparse exit and no Cargo/no scratch creation.; CLI positive: python scripts/pi-sdk-conformance/run_external_sdk.py --offline --scratch <fresh-external-source-bound-run> --target-dir <selected-cache>; require19/19 executed pass and external-sdk-proof.json identity check.; Representative wrong-package subprocess negative: call the same runner check_consumer_graph through a fresh interpreter with otherwise valid synthetic metadata but opi-ai manifest_path pointed to workspace/crates/opi-ai; require rejection. Do not mutate real source or run synthetic metadata as product proof.; cargo test -p opi-ai --test external_sdk --offline --locked; cargo clippy -p opi-ai --test external_sdk --offline --locked -- -D warnings; cargo fmt --check --all; python scripts/opi-doc-check.py; git diff --check.

#### Fix B17.1: execute assembled sdk matrix from extracted package

- **Finding source(s)**: phase20-codex-gpt61-b3f1810-20261103t142820z / 4cec97af8ef6bb773c630ee251728f31212e12fdc1bf9631f10b2dc3084bc0f6 / P20-AUD-017; phase20-codex-gpt61-b3f1810-20261103t142820z / 4cec97af8ef6bb773c630ee251728f31212e12fdc1bf9631f10b2dc3084bc0f6 / P20-AUD-018; phase20-pi-glm53-44374ec-20261103t160703z / f8c06b50b3c51c95ac5b1309e72a8b31d06c34785ad588ed4e42bff19d914990 / P20-AUD-007
- **Decision**: fix:execute-assembled-sdk-matrix-from-extracted-package
- **Verification status**: Confirmed
- **File(s)**: `crates/opi-ai/tests/external_sdk.rs`, `scripts/pi-sdk-conformance/consumer-external-sdk/Cargo.toml`, `scripts/pi-sdk-conformance/run_external_sdk.py`, `scripts/pi-sdk-conformance/test_external_sdk.py`, `scripts/pi-sdk-conformance/evidence/external-sdk-proof.json`
- **Change kind**: test-only
- **Change**: Make the assembled matrix an actual extracted-package consumer, with missing assembly assertions inside its existing single Rust test source. Runner supplies execution/provenance, not production behavior. No second public seam, no product configuration/Agent/persistence prerequisite.
- **Closure predicate**: The actual locally packaged opi-ai artifact is the only Opi runtime dependency of a fresh outside-workspace ordinary default-feature consumer. Its exact copied assembled test source discovers all19 named cases and executes every one with 1passed,0failed,0ignored; every public-boundary predicate below holds. Metadata, resolved graph/checksums/features, packaged source/resource hashes, executable identity, raw logs and original exit codes bind evidence to the candidate source. Missing, filtered, ignored, extra, compile-only, substituted-package or product-dependent runs are rejected.
- **Red-before**: Test-only absent outside-workspace execution path is established by current source inventory; no product behavior is changed. New runner guard tests should demonstrate their own negative admissions, but no fictitious product red-before is required.
- **Green-after**: python scripts/pi-sdk-conformance/run_external_sdk.py --offline --scratch <new-external-path> --target-dir <selected-persistent-cache>; cargo test -p opi-ai --test external_sdk -> The actual locally packaged opi-ai artifact is the only Opi runtime dependency of a fresh outside-workspace ordinary default-feature consumer. Its exact copied assembled test source discovers all19 named cases and executes every one with 1passed,0failed,0ignored; every public-boundary predicate below holds. Metadata, resolved graph/checksums/features, packaged source/resource hashes, executable identity, raw logs and original exit codes bind evidence to the candidate source. Missing, filtered, ignored, extra, compile-only, substituted-package or product-dependent runs are rejected.
- **Test impact**: add/update; retain existing safety and valid-source controls; delete none.

This test-only batch does not claim native3-platform/MSRV or every C018 criterion. Missing product obligations stay visible; runner failures cannot authorize unbounded production repair.

- `crates/opi-ai/tests/external_sdk.rs`: Keep self-contained public-only test source; retain10 existing names, strengthen actual recorder/store/loop assertions, add9 named cases. Keep helpers local or inline modules; no workspace-private include/path module. Correct header to distinguish shared test source from actual outside-workspace execution supplied by runner.
- `scripts/pi-sdk-conformance/consumer-external-sdk/Cargo.toml`: Test-only standalone package publish=false, edition2024, empty [workspace]; [dependencies] only opi-ai={path="@PACKAGE@"}, with default features enabled. [dev-dependencies] exactly tokio=1.52.3 (rt-multi-thread,macros,sync,time,net,io-util),tokio-util=0.7.18,futures-util=0.3.32,serde_json=1.0.149,secrecy=0.10.3,tempfile=3.27.0,wiremock=0.6.5, all normal registry dependencies pinned from current Cargo.lock. No root manifests/lock/dependencies change.
- `scripts/pi-sdk-conformance/run_external_sdk.py`: Python stdlib runner --offline --scratch <fresh-external-dir> --target-dir <persistent-cache>. Reuse existing config/env/source guard and safe unpack/lease functions; package with cargo package -p opi-ai --allow-dirty --no-verify --offline --locked, preserve artifact, extract safely. Compare package/tests/external_sdk.rs byte-for-byte to current source and copy to scratch/consumer/tests/assembled.rs; no repository consumer Rust copy. Build/list/execute as described below. Output exact structured proof in scratch and evidence/external-sdk-proof.json, verdict FAIL by default/finally on any failure.
- `scripts/pi-sdk-conformance/test_external_sdk.py`: Unit and subprocess tests for admission and exact-case/execution parsing; no mocked Cargo success accepted as SDK evidence. CLI negatives fail before Cargo or scratch mutation. Reuse existing guard tests only where unchanged; new external graph/case checks get direct negative tests.
- `scripts/pi-sdk-conformance/evidence/external-sdk-proof.json`: Generated actual run report with input hashes, package/archive/extracted manifests, graph, toolchain, native host identity, executable hash, exact19 names/counts, commands/exits/raw-log hashes and limitations; no ledger or Phase-conformant claim.

**External consumer execution protocol**

1. Admission: --offline required; fresh scratch outside repo/cache; persistent cache quick status and existing marker ownership; source-replacement/build override Cargo config/env rejection via controlled_environment/check_cargo_configs; validate_paths from run_early_package_proof. Cache ownership is the actual invoking repository. If execution uses a sealed source export, explicitly pass the original cache-owner root to the shared path validation while packaging only the sealed export; bind both paths and archive source hashes. Do not rewrite markers, silently switch target, or claim an export is a Git workspace.
2. Acquire cache lease before Cargo, always release own lease in finally. Capture rustc -vV/cargo -vV and source inputs (Cargo.toml/Cargo.lock, opi-ai sources/resources, exact test/runner/template bytes). This is a local package smoke; no registry publish/release.
3. Cargo creates .crate offline/locked, copy its exact bytes into scratch and SHA256 it; safe extract using shared.unpack_package. Verify distributable Cargo.toml, package identity/version, all packaged production/resource bytes and the exact assembled source equal the source slice before use. Preserve Cargo.toml.orig and .cargo_vcs_info identities where present; do not treat vcs_info alone as source equivalence.
4. Copy consumer template, substitute only @PACKAGE@ with extracted package absolute path; copy exact packaged tests/external_sdk.rs to tests/assembled.rs. Generate fresh consumer lock offline (no network/install); validate no extra Opi runtime/crates, no path/git dependencies except the SDK, no patch/replace/source injection. Hash complete resolved lock and compare required versions/checksums to source lock; record ordinary caller graph/features.
5. cargo metadata --offline --locked --format-version1 from consumer must identify exactly one opi-ai and manifest_path equal extracted package/Cargo.toml; consumer workspace_root is external scratch and contains no original workspace members. Metadata cannot point to source/crates/opi-ai. Confirm default feature profile.
6. cargo test --offline --locked --test assembled --no-run --message-format=json and cargo clippy --offline --locked --test assembled -- -D warnings; require one compiler-artifact target assembled with executable inside selected persistent target. Save complete Cargo messages and hash executable.
7. Run the exact just-built executable --list --format terse; parse named tests and require set equality to fixed19 names, uniqueness, no benchmark/test omissions. Then execute every named case separately with --exact <name> --nocapture --test-threads=1; require process exit0, that exact case ok, and exactly1passed/0failed/0ignored (remaining18 filtered). Save raw output/exit for each. Compilation or a positive total without per-case execution never passes.
8. Isolate only child process environment using existing execution_environment proxy isolation (host untouched); use attached fake AuthContext, store, synthetic auth and loopback for every test. No environment key can activate live providers. Record native host as tested only; this does not fulfill separate three-platform/MSRV finding.
9. Write proof only after all checks, current input hashes rechecked, and successful lease release. Preserve failed logs and FAIL report on any missing case/build/test/identity/lease issue; do not fabricate zero gaps. Output evidence is part of planned changed paths.

| Exact case name | Test impact | Public-boundary predicate |
|---|---|---|
| assembled_matrix_drives_all_ten_published_chat_wires_through_one_instance | retain | One Models calls all ten existing real adapter fixtures via complete_simple; verify every exact local route and final output. Keep catalog package resources and injected auth/source wiring. |
| classifier_and_image_apis_answer_typed_through_one_instance | retain | Execute all four classifier adapters plus image on one Models; preserve Decisions504 typed negative, answer/stop/output facts and real loopback counts. Strengthen each peer.verify if necessary; no live path. |
| simple_options_port_across_wires_and_mismatched_native_fails_typed_preflight | retain | Neutral tool option reaches real wire; incompatible typed native options fail before auth/network. |
| request_callbacks_observe_once_and_protected_headers_fail_typed | retain | Actual request callback counts/order and protected-header failure through packaged code. |
| duration_is_measured_once_drained_and_survives_serialization | retain | Known duration survives two AssistantMessage serde roundtrips without a second request. |
| manual_tool_loop_continues_after_serde_roundtrip | update | Real Anthropic two-request loop; assert ToolUse, no tool execution before caller validation, validate returned arguments via public validation, caller appends matching tool result, serde transcript preserves associations/raw args/knownness, second response completes; add Length/Deferred prohibition assertions only to the existing relevant deferred/loop case, not a new runtime hook. |
| two_instances_share_a_store_but_keep_routes_isolated_and_clean_up | update | Use one explicit InMemoryCredentialStore and fake OAuth flow with public store-owned write/ACK; both Models use public register_oauth_route so actual resolver reads the same credential. Verify read_count increases and both loopback requests carry synthetic shared account. Same provider/model id can select different endpoint/model data; no route overwrite across instances. No static-resolver bypass of store. Keep cleanup calls; non-noop cleanup is separately covered below. |
| empty_models_registers_no_provider_and_static_reads_do_no_io | update | Attach the recording AuthContext/store to the actual tested Models; synchronous no-runtime subcase constructs/registers/reads static metadata and proves source/store/network counts0. Do not count an unattached recorder or availability management as construction proof. |
| lawful_collection_over_sixty_four_messages_is_accepted | retain | Actual long legal conversation reaches real peer and terminal completion without artificial collection limit. |
| deferred_pending_then_explicit_fetch_serves_the_recorded_final | retain | Typed custom deferred backend returns pending then explicit fetch final; no automatic tool execution or hidden polling. |
| ordinary_native_stream_reaches_wire_and_collects_terminal | add | Use one real loopback OpenAI Chat adapter with typed native options and explicit max_tokens256; call Models.stream, consume one event then result(), assert exactly one send, Start then terminal, final text/usage/provenance and EOF. Do not require stream Sync. |
| ordinary_native_complete_reaches_wire_and_collects_terminal | add | Call Models.complete with equivalent typed native options; assert one send and same semantic final fields as the fixed fixture (exclude duration/timestamp from equality). |
| ordinary_simple_stream_reaches_wire_and_collects_terminal | add | Call Models.stream_simple with effective options equal to native fixture; consume events then result, exactly one send and same semantic terminal fields. |
| ordinary_simple_complete_reaches_wire_and_collects_terminal | add | Call Models.complete_simple with equivalent effective options; one send and same semantic terminal fields. This explicit case makes the required four-entrance inventory machine-checkable. |
| direct_adapter_uses_supplied_auth_without_models | add | Construct real adapter + complete record + explicit ResolvedAuth and use direct stream_prepared; no Models/resolver/store. Observe synthetic auth/header on loopback, drain via public collector, one send with expected terminal facts. |
| custom_chat_options_survive_compatible_instances_upsert_and_old_call | add | Test-local EchoOptions and ChatApi implementation built solely with public interfaces. Register same ApiId/options type in two Models, transfer handle-created native options, capture old owned call, compatible upsert changes future calls only; consume native/simple entries and assert recorded options/model/auth. Incompatible options type and unknown API each fail pre-auth. No private includes or new production API. |
| owned_stream_and_completion_are_send_static_after_models_drop | add | Compile require_owned<T: Send + static> over native/simple stream and completion call objects; spawn each on host Tokio after Models dropped, verify real loopback completion. Separately replace route/context after capture and assert old call uses frozen source while new call uses replacement. No borrowed Models, no private executor. |
| unpolled_calls_drop_without_source_or_transport_work | add | Create each of four ordinary calls against attached recording source/store/resolver and drop unpolled; source/store/auth/request counts remain0. Positive control polls one call and observes exactly one auth/send, proving counters are connected. |
| shared_transport_cleanup_awaits_exit_and_preserves_other_instance | add | Reuse public SharedCodex wrapper pattern with one shared Arc provider/HttpClient, two Models, same session id, distinct parent tokens. Actual GET503->delayed SSE local transport for both creates active source-owned generations. Wait for4 peer requests, cleanup Some(session) on left, require left Cancelled and cancelled_generations1 while right completes and parents remain live; Some(empty) typed/no effects and final None cleanup pass. Await peer/local completion before declaring barrier done; source-specific WS framing remains existing consumer-ws responsibility. |

**Admission and negative checks**

- python -m unittest discover -s scripts/pi-sdk-conformance -p test_external_sdk.py -v: accept exact case list and all19 individual successes; reject missing/duplicate/extra names, exit101, zero executed, ignored1, filtered-all, no execution log, multiple/wrong executable artifacts, wrong SDK manifest, extra opi-* runtime crate, path/git/patch/replace dependency, source test byte drift, private include, reused/in-repo scratch.
- CLI negative: python scripts/pi-sdk-conformance/run_external_sdk.py --scratch <new-external-negative-path> --target-dir <selected-cache> (omit --offline); assert nonzero argparse exit and no Cargo/no scratch creation.
- CLI positive: python scripts/pi-sdk-conformance/run_external_sdk.py --offline --scratch <fresh-external-source-bound-run> --target-dir <selected-cache>; require19/19 executed pass and external-sdk-proof.json identity check.
- Representative wrong-package subprocess negative: call the same runner check_consumer_graph through a fresh interpreter with otherwise valid synthetic metadata but opi-ai manifest_path pointed to workspace/crates/opi-ai; require rejection. Do not mutate real source or run synthetic metadata as product proof.
- cargo test -p opi-ai --test external_sdk --offline --locked; cargo clippy -p opi-ai --test external_sdk --offline --locked -- -D warnings; cargo fmt --check --all; python scripts/opi-doc-check.py; git diff --check.

C018 assigns only its assembled public-consumer proof portion to B17; the remaining conjunctive predicates do not become covered merely by a nineteen-case package pass.

B17 owns creation of run_external_sdk.py/test_external_sdk.py and wiring the optional --cache-workspace input into the B90 shared guard. Its own CLI negatives/positive actual19case run verify that integration; they are not B90 closure prerequisites.

### Batch B95: execute the public SDK fixture corpus from the actual package

**Closure predicate**: A fresh ordinary workspace-external consumer of the actual opi-ai .crate compiles and executes every declared public SDK fixture target with exact sealed source/helper/resource/test names and zero ignored/filtered/failed cases; opi-ai is its only Opi runtime dependency, and published profile/OAuth/nonchat/callback/failure/cancellation behavior is observed through public callers rather than enumeration alone.
**Dependencies**: ["B91", "B17"]
**Verification union**: python -m unittest discover -s scripts/pi-sdk-conformance -p test_public_sdk.py -v; actual run_public_sdk --offline --scratch <fresh> --target-dir <selected> --cache-workspace <owner>; exact-target/list/graph/source/failedJSON CLI negatives; all copied test binaries execute; affected-target clippy/doc/diff.

#### Fix B95.1: execute the public SDK fixture corpus from the actual package

- **Finding source(s)**: phase20-codex-gpt61-b3f1810-20261103t142820z / 4cec97af8ef6bb773c630ee251728f31212e12fdc1bf9631f10b2dc3084bc0f6 / P20-AUD-018; phase20-pi-glm53-44374ec-20261103t160703z / f8c06b50b3c51c95ac5b1309e72a8b31d06c34785ad588ed4e42bff19d914990 / P20-AUD-007
- **Decision**: fix:execute-public-sdk-fixtures-through-extracted-artifact
- **Verification status**: Partially confirmed
- **File(s)**: `scripts/pi-sdk-conformance/run_public_sdk.py`, `scripts/pi-sdk-conformance/test_public_sdk.py`, `scripts/pi-sdk-conformance/consumer-public-sdk/Cargo.toml`, `scripts/pi-sdk-conformance/evidence/public-sdk-proof.json`
- **Change kind**: test-only
- **Change**: Add one public-fixture execution harness, separate from B17's fixed nineteen ordinary assembled cases. Copy only byte-sealed public integration-test sources, test-only helper modules and resources from the actual extracted package; compile them as external consumer tests of that package, with normalized ordinary registry test dependencies. Never include production source or substitute a workspace library.
- **Closure predicate**: Every declared public fixture target and named case executes successfully through the actual packaged SDK; source/helper/resource/graph/executable/log identities bind all results, with only opi-ai as Opi runtime dependency and zero ignored/filtered/missing cases.
- **Red-before**: Not applicable to production behavior: the missing complete outside-workspace fixture execution route is a test-only gap. Unit/mocked guard success is not SDK acceptance. A discovered runtime failure is preserved and stops for separately bound remediation.
- **Green-after**: run_public_sdk --offline --scratch <fresh> --target-dir <selected> --cache-workspace <owner> -> all declared copied public targets/cases pass0ignored/filtered with actual package identities and original raw exits; published42factory/nineOAuth/tenchat/oneimage/fourclassifier source/wire/failure/callback cases retain finite assertions from their owning source tests. Observed:not-run.
- **Test impact**: add harness admission/subprocess negative tests and actual outside-package execution; update existing public fixture assertions only through their individually assigned batches; retain all original cases; delete none.

The current immutable source has73integration targets. B17 alone owns external_sdk's exact nineteen assembled cases; platform_delivery remains the source graph/native-owner metadata suite and is not copied into a consumer that would change CARGO_MANIFEST_DIR. The other71 current targets below are required, plus the explicitly planned new public adc_proxy_contract,response_duration and proxy_source_contract targets. Copy nested test modules/support fixtures/resources needed by these exact targets, preserving their relative test/fixture layout and bytes; reject any source dependency escaping the extracted package or including private production source. Copy no root Cargo manifest/lock/config. Generate a fresh standalone consumer manifest with an empty workspace, only extracted opi-ai as Opi runtime dependency, and the normalized crates.io test dependencies used by the copied fixtures. Record actual independent consumer resolution; do not treat package Cargo.lock as consumer authority.

Required existing public targets:

```json
[
  "adc_owner_regressions",
  "anthropic_fixtures",
  "anthropic_thinking_roundtrip",
  "api_mapped_provider",
  "auth_contracts",
  "auth_process_lifecycle",
  "azure_openai_fixtures",
  "bedrock_fixtures",
  "builtin_auth_ownership",
  "builtin_native_scoped",
  "builtin_scoped_env",
  "catalog_artifact",
  "catalog_lifecycle",
  "catalog_operations",
  "catalog_personalization",
  "chat_capture_replay",
  "chat_pi_messages",
  "classifier_contract",
  "codex_native_options",
  "codex_telemetry",
  "connection_pooling",
  "context_budget",
  "custom_provider_registration",
  "deferred_contract",
  "gemini_fixtures",
  "gemini_lifecycle",
  "http_acceptance",
  "image_input",
  "image_provider_serialization",
  "mistral_fixtures",
  "model_capabilities_migration",
  "model_wire_metadata",
  "node4_preparation",
  "oauth_flows",
  "oauth_store_integration",
  "oauth_wire_shape",
  "openai_chat_fixtures",
  "openai_codex_responses",
  "openai_compat_lifecycle",
  "openai_responses_fixtures",
  "openai_responses_lifecycle",
  "openrouter_fixtures",
  "output_content_image",
  "per_request_auth",
  "provider_collection",
  "provider_diagnostics",
  "provider_error_classes",
  "provider_lifecycle",
  "provider_trait",
  "proxy_support",
  "public_utilities",
  "registry",
  "replay_contract",
  "request_enrichment",
  "resource_cleanup",
  "retry_backoff",
  "schema_engine",
  "schema_public",
  "service_tier_cost",
  "stream_events",
  "telemetry_carrier",
  "telemetry_operations",
  "test_support_tests",
  "tls_contract",
  "tool_result_no_leak",
  "tool_result_wire",
  "transcript_contract",
  "usage_cost",
  "vertex_fixtures",
  "whole_call_retry",
  "ws_upgrade"
]
```

Before execution, require exact declared target presence and the complete discovered named-case set for the bound source/target cfg; then require every target's actual execution to equal that set, original exit0 and no failed JSON/ignored/filtered cases. Reject a missing target, renamed/source-drifted helper, absent fixture, private source include, wrong SDK manifest, extra Opi runtime dependency, duplicate/missing executable, omitted test case or failed-verdict/exit0 report. Exact user-facing examples stay in their three-flow runner. The protocol family/private codec/feature-union/native reports remain B91/B92; this ordinary public corpus supplements them and does not rename substrate/private proof as public acceptance. All provider/source tests use the existing isolated fake contexts/loopback peers; credentials merely present never enable live calls.

The explicit final public target set is71current targets plus3planned additions (adc_proxy_contract,response_duration,proxy_source_contract), totaling74. B17 and platform_delivery stay separately owned. No new test file may be silently omitted from exact target discovery/execution.

### Batch B92: execute exact MSRV package proof on three native hosts

**Closure predicate**: Native Linux/macOS/Windows each execute the complete selected package/profile/behavior matrix at rustc1.97.1 using actual candidate source and artifact identities; reports contain all raw commands/exits/named counts/graph/native provenance and cannot pass from config,cross-check or mocked Cargo.
**Dependencies**: ["B91", "B17", "B95"]
**Verification union**: python -m unittest discover -s scripts/pi-sdk-conformance -p test_native_package_matrix.py -v with wronghost/wrongtoolchain/missingprofile/missingnative/failedJSON/sourceartifactmismatch negatives; actual run_native_package_matrix --offline --scratch <fresh> --target-dir <selected> --cache-workspace <owner> on each native host; exact1.97.1 workspace behavior gates and all B91/B17 package profiles; owning native CI run/artifact/log identities required before result closure

#### Fix B92.1: execute exact MSRV package proof on three native hosts

- **Finding source(s)**: phase20-codex-gpt61-b3f1810-20261103t142820z / 4cec97af8ef6bb773c630ee251728f31212e12fdc1bf9631f10b2dc3084bc0f6 / P20-AUD-018; phase20-pi-glm53-44374ec-20261103t160703z / f8c06b50b3c51c95ac5b1309e72a8b31d06c34785ad588ed4e42bff19d914990 / P20-AUD-007
- **Decision**: fix:execute-exact-MSRV-package-proof-on-three-native-hosts
- **Verification status**: Partially confirmed
- **File(s)**: `.github/workflows/ci.yml`, `scripts/pi-sdk-conformance/run_native_package_matrix.py`, `scripts/pi-sdk-conformance/test_native_package_matrix.py`
- **Change kind**: test-only
- **Change**: Add one dedicated three-OS CI job with existing pinned checkout/rust-toolchain/rust-cache actions and exact1.97.1. Reuse upload-artifact@ea165f8d65b6e75b540449e92b4886f43607fa02 already present in release workflow for source/proof/log artifacts, including failure reports. New Python orchestrator uses marked persistent lifecycle target, registry prerequisite fetch before offline proof, fresh scratch, B91 family runners and B17 actual19case consumer. Remove inherited RUSTFLAGS/CARGO_ENCODED_RUSTFLAGS keys from proof subprocess env and preserve strict native guard; warnings via clippy -D warnings. Record actual CC/CMake-if-used/build/link identities. No provider/live calls, commit/push/PR or dispatch is authorized by this plan execution; source publication/native-host execution remains an explicit materialization boundary.
- **Closure predicate**: Native Linux/macOS/Windows each execute the complete selected package/profile/behavior matrix at rustc1.97.1 using actual candidate source and artifact identities; reports contain all raw commands/exits/named counts/graph/native provenance and cannot pass from config,cross-check or mocked Cargo.
- **Red-before**: Not applicable to production behavior: one bounded harness/proof-collection root. Current source inventory establishes the missing exact route/admission; synthetic unit/subprocess negatives prove guard behavior only and are never labeled native/product acceptance.
- **Green-after**: python -m unittest discover -s scripts/pi-sdk-conformance -p test_native_package_matrix.py -v with wronghost/wrongtoolchain/missingprofile/missingnative/failedJSON/sourceartifactmismatch negatives; actual run_native_package_matrix --offline --scratch <fresh> --target-dir <selected> --cache-workspace <owner> on each native host; exact1.97.1 workspace behavior gates and all B91/B17 package profiles; owning native CI run/artifact/log identities required before result closure -> Native Linux/macOS/Windows each execute the complete selected package/profile/behavior matrix at rustc1.97.1 using actual candidate source and artifact identities; reports contain all raw commands/exits/named counts/graph/native provenance and cannot pass from config,cross-check or mocked Cargo. Observed:not-run.
- **Test impact**: add/update exact harness guards and finite execution cases; retain controls; delete none.

The native orchestration executes B95 ordinary public fixture corpus as well as B17 nineteen cases and B91 protocol/feature-union families. Complete package action/source/auth behavior is not reduced to the nineteen assembled smoke/first-use cases.

### Batch B93: reconcile final source bound proof and bilingual claims

**Closure predicate**: Existing tracking/evidence and both README counterparts report exactly the final current source/package/native results and per-test impact, with reproducible identities and honest gaps; they neither rewrite audit/history nor self-certify ledger admission or Phase exit.
**Dependencies**: ["B92"]
**Verification union**: python scripts/opi-doc-check.py; git diff --check; exact current source/example/bundle hashes and all runtime/package/native prerequisite receipt checks; all declared claim links resolve to actual evidence

#### Fix B93.1: reconcile final source bound proof and bilingual claims

- **Finding source(s)**: phase20-codex-gpt61-b3f1810-20261103t142820z / 4cec97af8ef6bb773c630ee251728f31212e12fdc1bf9631f10b2dc3084bc0f6 / P20-AUD-018; phase20-pi-glm53-44374ec-20261103t160703z / f8c06b50b3c51c95ac5b1309e72a8b31d06c34785ad588ed4e42bff19d914990 / P20-AUD-007
- **Decision**: fix:reconcile-final-source-bound-proof-and-bilingual-claims
- **Verification status**: Partially confirmed
- **File(s)**: `docs/research/pi-ai-opi-ai-alignment/upstream-tracking.md`, `docs/research/pi-ai-opi-ai-alignment/phase20-delivery-evidence.md`, `crates/opi-ai/README.md`, `crates/opi-ai/README.zh.md`, `scripts/pi-sdk-conformance/README.md`
- **Change kind**: test-only
- **Change**: Collect only actual prerequisite receipts and immutable source/graph/toolchain/artifact/executable/log hashes. Record add/update/delete/retain/none plus exact command/filter/executed/ignored/skip/missing counts. Reconcile all57active SDK/ninehost/oneretired identities and registered RDF/platform limits, source-equal three examples and authentic catalog. Keep F1a/ledger/frozen-snapshot authority with its existing owner. If any prerequisite lacks native execution, do not claim success or close this predicate.
- **Closure predicate**: Existing tracking/evidence and both README counterparts report exactly the final current source/package/native results and per-test impact, with reproducible identities and honest gaps; they neither rewrite audit/history nor self-certify ledger admission or Phase exit.
- **Red-before**: Not applicable to production behavior: one bounded harness/proof-collection root. Current source inventory establishes the missing exact route/admission; synthetic unit/subprocess negatives prove guard behavior only and are never labeled native/product acceptance.
- **Green-after**: python scripts/opi-doc-check.py; git diff --check; exact current source/example/bundle hashes and all runtime/package/native prerequisite receipt checks; all declared claim links resolve to actual evidence -> Existing tracking/evidence and both README counterparts report exactly the final current source/package/native results and per-test impact, with reproducible identities and honest gaps; they neither rewrite audit/history nor self-certify ledger admission or Phase exit. Observed:not-run.
- **Test impact**: add/update exact harness guards and finite execution cases; retain controls; delete none.

## Final Verification

This replanning changes only the fixed plan pair after separately authorized B01 commit `2eee4afebc911482d969ead8452115a4a7f9592d`. Check current-set/dispositions/plan bindings, source/criterion identities and executable dependency/proof-owner graph, `python scripts/opi-doc-check.py`, `git diff --check`, and separate staged/unstaged/untracked inventory. B01 has exact tested-source/commit association; no other production repair, result or ledger write is made by replanning. The machine-bound assurance pair has no localized sibling; future product README changes remain paired.

The future verification union (all remaining planned repair green remains not-run; current baseline probes are separately observed) is selected from all admitted change surfaces: focused changed test binaries and actual public/wire regression cases above; owning script unit plus CLI/subprocess negatives and actual package consumer for B17; affected-target clippy; bilingual doc check. Retain C001 no-action checks through `builtin_auth_ownership`, `builtin_scoped_env`, `auth_contracts` and `oauth_store_integration` without promoting broader linked criteria. Because B04 crosses SDK/Agent/product consumers, the applicable final workspace gates are:

```text
cargo fmt --check --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --all-targets
cargo test --workspace --doc
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps
python scripts/opi-doc-check.py
git diff --check
```

Use native shell environment syntax for RUSTDOCFLAGS and the selected persistent cache observation wrapper for Cargo. No paid/live provider, native cross-platform success, CI dispatch or release is implied. Reuse unaffected proof only with exact current identity; broaden/repeat checks only for changed inputs, failures or unresolved gaps. Test impact: workflow regressions add16/update fixtures/retain others/delete none; repository Rust tests none; temporary probes add; proposed apply add/update/retain as recorded, delete none.

## Exclusions and Retained Verification

C001 has current-head no-action status after independently verified authorized B01 repair; original Major impact, audit observations and linked requirement states remain unchanged. Retain its wire/suite checks and all broader source/package/native obligations.

| Source run / Finding ID | Disposition | Current conclusion and retained observation | Remaining obligation / final administrative check |
|---|---|---|---|

| C016 / P20-AUD-016 | no-action:exact-observations-covered-by-p002-and-p003 | Confirmed / Major; Retain Confirmed/Major and all original source observations. Current exact3failures are exhaustively mapped to narrower live P002/P003; administrative exclusion is only no additional change. This neither grants requirement-met status nor waives either independently mandatory repair. Final3-check predicate remains required. | After B14/B15 each exact filter executes1passed/0failed/0ignored; collect all3results. Any failure prevents completing this administrative verification/full result. |

| P006 / P20-AUD-006 | no-action:reconciled-by-explicit-opi-implement-owner | Confirmed / Minor; Retain original Minor and Confirmed verification. Explicitly authorized opi-implement P0/P2/P4 then confirm-all/write-only installed the live source binding:16/16current hashes match. Historical Phase20 snapshot/source identity and68summary entries remain immutable, as required; their old source hash is not a live drift repair target. | Final administrative owner-receipt preservation check only; no remaining planning obligation. |


No source key is dropped, duplicated or treated as historically closed. C001 refutes only present implementation-defect applicability after fresh verification, not its historical observation or linked requirements. P006 is an explicit administrative exclusion with completed owner reconciliation and receipt preservation. C016 is an exact duplicate-observation administrative exclusion, with its final three checks still required. All planning decisions are resolved. Only a new explicit apply invocation of this READY plan and its emitted digest can admit production work; the whole source closure conjunction remains mandatory.

## Exact Criterion Cross-reference Matrix

This is bookkeeping over the current indexed strict union, not an additional audit or a consensus verdict. The source requirement records are unchanged. A row aligns only the exact tuple `(criterion_source.path, criterion_source.sha256, criterion_source.citation)`. Textual similarity, equal requirement IDs, common line numbers in differently written citations, and nearby requirements do not align records. Repeated exact tuples within one member retain every original requirement ID and state in that member column.

**Matrix index binding**: `e1b34739db46331707a93ab0f6f9336ed6386a4564b0215d8488ce288319f0ae`

| Member | Run | Requirements sidecar SHA-256 | Findings sidecar SHA-256 |
|---|---|---|---|
| `audit.codex.gpt61` | `phase20-codex-gpt61-b3f1810-20261103t142820z` | `c2600889ab81ce63688c346e9d7d3bf2a1e0fdbfc09daeba2b08f42b50c69898` | `4cec97af8ef6bb773c630ee251728f31212e12fdc1bf9631f10b2dc3084bc0f6` |
| `audit.pi.glm53` | `phase20-pi-glm53-44374ec-20261103t160703z` | `4ffa18d273639122681a2285fe29d51cc2022b5330d54ddd9ac9bbab79bb7bf8` | `f8c06b50b3c51c95ac5b1309e72a8b31d06c34785ad588ed4e42bff19d914990` |

Only requirement records referenced by at least one finding in their owning current member are selected; each reciprocal finding link and owning run was checked. Other requirement records stay outside this finding-closure matrix.

| Member | All sidecar requirements | Selected linked requirements | Current findings | Exact selected criterion tuples |
|---|---:|---:|---:|---:|
| `audit.codex.gpt61` | 942 | 232 | 21 | 192 |
| `audit.pi.glm53` | 257 | 21 | 7 | 18 |
| Total | 1199 | 253 | 28 | 210 distinct union tuples |

Exact cross-member aligned identities: **0**. Cross-member aligned identities with differing state sets: **0**. All 210 criterion rows are unaligned across members and show `—` in the other member column. This does not assert substantive agreement or disagreement between differently identified criteria.
Within-member tuples containing more than one original state: **8** (K087, K094, K095, K125, K127, K130, K142, K169).

### Criterion source identities

Each source alias binds both path and exact source digest; the criterion table supplies the literal citation to complete the identity.

| Source alias | Path | SHA-256 |
|---|---|---|
| S01 | `docs/opi-spec.md` | `b698133fbc2c0347726447159259c18effa8a1091796aae559c4bb04cb5612f3` |
| S02 | `docs/research/pi-ai-opi-ai-alignment/acceptance-matrix.md` | `c831586339910f9496a64e6b3c9390a0b7a2a366d37230222874a5f399099a5d` |
| S03 | `docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-design-readiness.md` | `b74c0b09afd31bc06cf8d5d1df834a6af9d53f2d08c3653aa492ed36396c4eba` |
| S04 | `docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-runtime.md` | `1328a81e561e1e1bfb2c35df788ce6d08fec2f71e14eb8abba9d57e1f03f1c7b` |
| S05 | `docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-schema.md` | `878d15231e734dc03772e1f1bd48cb8ffe2252a2818748e4b8630af43a2e8a76` |
| S06 | `docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-transport-auth.md` | `42e1bc90c02e958ada4d2a01db999cd906ae33ee971496c50c07ed38067cdcfb` |
| S07 | `docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-ws.md` | `61547ccfc0549018d2fd4042ade8edac3547599b412e12ea2479821b40f67a8f` |
| S08 | `docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-rust-backends.md` | `acb7f5f798b3384c63d36ad9af844b580755e2ea792ce3e2d32d9d3af2f15e81` |
| S09 | `docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-rust-differences.md` | `411d2f6d262b1afc22e84c73213dffd0eb890ed70e210dfa497d2a5d021cd31d` |
| S10 | `docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md` | `9e3d6ef690c3a794bc9c47ddc2dce04840f98d8881649be1ab2e8db0df287fd5` |
| S11 | `docs/research/pi-ai-opi-ai-alignment/replay-contracts.md` | `223b21e1aa590d0e4646d99cd7f6792a694c7d8bf0f34b31dd6380979d324fa4` |
| S12 | `docs/research/pi-ai-opi-ai-alignment/spec-draft.md` | `8c6cd66f7771e026210f06e49d037da54029c74258a1d45925cdbddd710f5f21` |
| S13 | `docs/research/pi-ai-opi-ai-alignment/transcript-target-proposal.md` | `25a4234507ab1c4c12d483c56bb4083172bf4d5c35dfec1dc92bc98c77a5d94e` |

### Original criterion records

Each cell retains `requirement ID (original state; mandatory/optional)`; these are audit states, not remediation verification statuses. Empty member cells are `—`.

| Criterion | Source | Exact citation | audit.codex.gpt61 original requirements | audit.pi.glm53 original requirements |
|---|---|---|---|---|
| K001 | S01 | AUTH-003, line 36 | `P20-AUTH-003` (not-assessable; mandatory) | — |
| K002 | S01 | CTRL-003, line 280 | `P20-CTRL-003` (partially-met; mandatory) | — |
| K003 | S01 | GATE-001, line 551 | `P20-GATE-001` (not-assessable; mandatory) | — |
| K004 | S01 | GOAL-001, line 72 | `P20-GOAL-001` (partially-met; mandatory) | — |
| K005 | S01 | INV-001, line 405 | `P20-INV-001` (partially-met; mandatory) | — |
| K006 | S01 | INV-002, line 406 | `P20-INV-002` (not-assessable; mandatory) | — |
| K007 | S01 | INV-006, line 443 | `P20-INV-006` (partially-met; mandatory) | — |
| K008 | S01 | PRIN-002, line 121 | `P20-PRIN-002` (not-assessable; mandatory) | — |
| K009 | S01 | PRIN-004, line 123 | `P20-PRIN-004` (partially-met; mandatory) | — |
| K010 | S01 | PRIN-005, line 124 | `P20-PRIN-005` (not-assessable; mandatory) | — |
| K011 | S02 | L255 | `P20-SDK-FACTORY-015` (not-met; mandatory)<br>`P20-SDK-FACTORY-050` (not-met; mandatory)<br>`P20-SDK-FACTORY-130` (not-met; mandatory)<br>`P20-SDK-FACTORY-135` (not-met; mandatory)<br>`P20-SDK-FACTORY-140` (not-met; mandatory)<br>`P20-SDK-FACTORY-175` (not-met; mandatory) | — |
| K012 | S02 | ONB-01 line 531; spec-draft.md line 554 | `P20-DEL-ONB-01-B` (not-assessable; mandatory) | — |
| K013 | S02 | ONB-03 line 533; spec-draft.md line 556 | `P20-DEL-ONB-03-B` (not-met; mandatory) | — |
| K014 | S02 | ONB-05 line 535; spec-draft.md line 558 | `P20-DEL-ONB-05-A` (not-met; mandatory) | — |
| K015 | S02 | V-01, line 56 | `P20-V-01` (not-assessable; mandatory) | — |
| K016 | S02 | V-03, line 58 | `P20-V-03` (partially-met; mandatory) | — |
| K017 | S02 | V-05, line 60 | `P20-V-05` (partially-met; mandatory) | — |
| K018 | S02 | V-09, line 64 | `P20-V-09` (not-assessable; mandatory) | — |
| K019 | S02 | V-11, line 66 | `P20-V-11` (partially-met; mandatory) | — |
| K020 | S02 | V-12, line 67 | `P20-V-12` (partially-met; mandatory) | — |
| K021 | S02 | V-13, line 68 | `P20-V-13` (partially-met; mandatory) | — |
| K022 | S02 | V-15, line 70 | `P20-V-15` (partially-met; mandatory) | — |
| K023 | S02 | V-16, line 71 | `P20-V-16` (not-assessable; mandatory) | — |
| K024 | S02 | V-20, line 75 | `P20-V-20` (partially-met; mandatory) | — |
| K025 | S02 | V-22, line 77 | `P20-V-22` (partially-met; mandatory) | — |
| K026 | S02 | V-23, line 78 | `P20-V-23` (partially-met; mandatory) | — |
| K027 | S02 | V-24, line 79 | `P20-V-24` (partially-met; mandatory) | — |
| K028 | S02 | V-25, line 80 | `P20-V-25` (partially-met; mandatory) | — |
| K029 | S02 | V-26, line 81 | `P20-V-26` (not-assessable; mandatory) | — |
| K030 | S02 | V-31, line 93 | `P20-V-31` (not-assessable; mandatory) | — |
| K031 | S02 | V-32, line 94 | `P20-V-32` (not-assessable; mandatory) | — |
| K032 | S02 | V-34, line 96 | `P20-V-34` (not-assessable; mandatory) | — |
| K033 | S02 | V-35, line 97 | `P20-V-35` (partially-met; mandatory) | — |
| K034 | S02 | V-36, line 98 | `P20-V-36` (partially-met; mandatory) | — |
| K035 | S02 | V-38, line 100 | `P20-V-38` (not-assessable; mandatory) | — |
| K036 | S02 | V-39, line 182 | `P20-V-39` (partially-met; mandatory) | — |
| K037 | S02 | V-40, line 183 | `P20-V-40` (partially-met; mandatory) | — |
| K038 | S02 | V-41, line 184 | `P20-V-41` (partially-met; mandatory) | — |
| K039 | S02 | V-42, line 185 | `P20-V-42` (not-assessable; mandatory) | — |
| K040 | S02 | V-45, line 193 | `P20-V-45` (partially-met; mandatory) | — |
| K041 | S02 | V-49, line 212 | `P20-V-49` (partially-met; mandatory) | — |
| K042 | S02 | V-50, line 213 | `P20-V-50` (partially-met; mandatory) | — |
| K043 | S02 | V-51, line 214 | `P20-V-51` (partially-met; mandatory) | — |
| K044 | S02 | V-53, line 216 | `P20-V-53` (not-assessable; mandatory) | — |
| K045 | S02 | V-55, line 218 | `P20-V-55` (partially-met; mandatory) | — |
| K046 | S02 | V-56, line 219 | `P20-V-56` (not-assessable; mandatory) | — |
| K047 | S02 | V-57, line 220 | `P20-V-57` (partially-met; mandatory) | — |
| K048 | S02 | V-58, line 221 | `P20-V-58` (not-assessable; mandatory) | — |
| K049 | S02 | V-59, line 222 | `P20-V-59` (not-assessable; mandatory) | — |
| K050 | S02 | docs/research/pi-ai-opi-ai-alignment/acceptance-matrix.md#Pi 1.3.0 完整公开能力补齐（V-49–59） / V-58 | — | `V-58` (partially-met; mandatory) |
| K051 | S02 | docs/research/pi-ai-opi-ai-alignment/acceptance-matrix.md#Pi 1.3.0 完整公开能力补齐（V-49–59） / V-59 | — | `V-59` (partially-met; mandatory) |
| K052 | S02 | docs/research/pi-ai-opi-ai-alignment/acceptance-matrix.md#已有要求的修订验收（V-01–26） / V-05 | — | `V-05` (partially-met; mandatory) |
| K053 | S02 | docs/research/pi-ai-opi-ai-alignment/acceptance-matrix.md#已有要求的修订验收（V-01–26） / V-11 | — | `V-11` (partially-met; mandatory) |
| K054 | S02 | docs/research/pi-ai-opi-ai-alignment/acceptance-matrix.md#已确认 SDK 协议、身份与预算（V-43–45） / V-44 | — | `V-44` (partially-met; mandatory) |
| K055 | S02 | lines 431-448 | `P20-DEL-REPLACE-05` (not-assessable; mandatory) | — |
| K056 | S02 | lines 470-474,477,493 | `P20-DEL-EXIT-07` (not-met; mandatory) | — |
| K057 | S03 | L19,L35,L43,L47 | `P20-MECH-DESIGN-001` (not-assessable; mandatory) | — |
| K058 | S03 | L39,L41 | `P20-MECH-DESIGN-002` (not-assessable; mandatory) | — |
| K059 | S03 | L43,L45 | `P20-MECH-DESIGN-003` (not-assessable; mandatory) | — |
| K060 | S03 | L47 | `P20-MECH-DESIGN-004` (not-assessable; mandatory) | — |
| K061 | S04 | L29 | `P20-MECH-DELIVERY-001` (not-assessable; mandatory) | — |
| K062 | S04 | L29,L33 | `P20-MECH-DELIVERY-002` (not-assessable; mandatory) | — |
| K063 | S04 | L31,L33 | `P20-MECH-DELIVERY-003` (not-assessable; mandatory) | — |
| K064 | S04 | L33 | `P20-MECH-DELIVERY-004` (not-assessable; mandatory) | — |
| K065 | S04 | L39 | `P20-MECH-DELIVERY-005` (not-assessable; mandatory)<br>`P20-MECH-DELIVERY-006` (not-assessable; mandatory)<br>`P20-MECH-DELIVERY-007` (not-assessable; mandatory) | — |
| K066 | S04 | L43 | `P20-MECH-DELIVERY-008` (not-assessable; mandatory) | — |
| K067 | S04 | L43,L45 | `P20-MECH-DELIVERY-009` (not-assessable; mandatory) | — |
| K068 | S05 | L36 | `P20-MECH-SCHEMA-005` (not-assessable; mandatory) | — |
| K069 | S05 | L54 | `P20-MECH-SCHEMA-011` (not-assessable; mandatory) | — |
| K070 | S05 | L60 | `P20-MECH-SCHEMA-016` (not-assessable; mandatory) | — |
| K071 | S05 | L60,L62 | `P20-MECH-SCHEMA-017` (not-assessable; mandatory) | — |
| K072 | S05 | L68 | `P20-MECH-SCHEMA-021` (not-assessable; mandatory) | — |
| K073 | S05 | L70,L72 | `P20-MECH-SCHEMA-025` (not-assessable; mandatory) | — |
| K074 | S05 | L74 | `P20-MECH-SCHEMA-028` (not-met; mandatory) | — |
| K075 | S05 | L78 | `P20-MECH-SCHEMA-031` (not-assessable; mandatory)<br>`P20-MECH-SCHEMA-032` (not-assessable; mandatory)<br>`P20-MECH-SCHEMA-033` (not-assessable; mandatory)<br>`P20-MECH-SCHEMA-034` (not-assessable; mandatory) | — |
| K076 | S05 | L80,L86,L88 | `P20-MECH-SCHEMA-035` (not-assessable; mandatory) | — |
| K077 | S05 | L86,L88 | `P20-MECH-SCHEMA-036` (not-assessable; mandatory) | — |
| K078 | S06 | L101 | `P20-MECH-AUTH-002` (not-assessable; mandatory) | — |
| K079 | S06 | L105 | `P20-MECH-AUTH-003` (not-assessable; mandatory) | — |
| K080 | S06 | L106 | `P20-MECH-AUTH-007` (not-assessable; mandatory) | — |
| K081 | S06 | L107 | `P20-MECH-AUTH-009` (not-assessable; mandatory) | — |
| K082 | S06 | L108 | `P20-MECH-AUTH-010` (not-assessable; mandatory)<br>`P20-MECH-AUTH-011` (not-assessable; mandatory) | — |
| K083 | S06 | L11,L37 | `P20-MECH-HTTP-001` (not-met; mandatory) | — |
| K084 | S06 | L110 | `P20-MECH-AUTH-014` (not-assessable; mandatory)<br>`P20-MECH-AUTH-015` (not-assessable; mandatory) | — |
| K085 | S06 | L111 | `P20-MECH-AUTH-016` (not-assessable; mandatory) | — |
| K086 | S06 | L112 | `P20-MECH-AUTH-017` (not-assessable; mandatory)<br>`P20-MECH-AUTH-018` (not-assessable; mandatory) | — |
| K087 | S06 | L113 | `P20-MECH-AUTH-019` (not-met; mandatory)<br>`P20-MECH-AUTH-020` (not-assessable; mandatory) | — |
| K088 | S06 | L114 | `P20-MECH-AUTH-021` (not-assessable; mandatory) | — |
| K089 | S06 | L115 | `P20-MECH-AUTH-022` (not-assessable; mandatory)<br>`P20-MECH-AUTH-023` (not-assessable; mandatory) | — |
| K090 | S06 | L116,L126 | `P20-MECH-AUTH-024` (not-assessable; mandatory) | — |
| K091 | S06 | L122,L126 | `P20-MECH-AUTH-025` (not-assessable; mandatory) | — |
| K092 | S06 | L124,L126 | `P20-MECH-AUTH-026` (not-assessable; mandatory) | — |
| K093 | S06 | L13,L101 | `P20-MECH-AUTH-001` (not-assessable; mandatory) | — |
| K094 | S06 | L37 | `P20-MECH-HTTP-002` (not-met; mandatory)<br>`P20-MECH-HTTP-003` (not-assessable; mandatory) | — |
| K095 | S06 | L39 | `P20-MECH-HTTP-004` (not-met; mandatory)<br>`P20-MECH-HTTP-005` (not-assessable; mandatory) | — |
| K096 | S06 | L47 | `P20-MECH-HTTP-006` (not-assessable; mandatory)<br>`P20-MECH-HTTP-007` (not-assessable; mandatory)<br>`P20-MECH-HTTP-008` (not-assessable; mandatory)<br>`P20-MECH-HTTP-009` (not-assessable; mandatory) | — |
| K097 | S06 | L51 | `P20-MECH-HTTP-010` (not-assessable; mandatory) | — |
| K098 | S06 | L65 | `P20-MECH-TLS-006` (not-assessable; mandatory) | — |
| K099 | S06 | L93 | `P20-MECH-TLS-013` (not-assessable; mandatory)<br>`P20-MECH-TLS-014` (not-assessable; mandatory) | — |
| K100 | S07 | L100,L130,L134 | `P20-MECH-WS-006` (not-assessable; mandatory) | — |
| K101 | S07 | L110 | `P20-MECH-WS-023` (not-assessable; mandatory) | — |
| K102 | S07 | L116,L132 | `P20-MECH-WS-029` (not-assessable; mandatory) | — |
| K103 | S07 | L120 | `P20-MECH-WS-033` (not-assessable; mandatory) | — |
| K104 | S07 | L122,L132 | `P20-MECH-WS-036` (not-assessable; mandatory) | — |
| K105 | S07 | L130 | `P20-MECH-WS-039` (not-assessable; mandatory)<br>`P20-MECH-WS-040` (not-assessable; mandatory) | — |
| K106 | S07 | L134 | `P20-MECH-WS-041` (not-assessable; mandatory) | — |
| K107 | S08 | lines 104-116,126,146,152-154 | `P20-DEL-PROV-05` (not-assessable; mandatory) | — |
| K108 | S08 | lines 186-188,212-216; spec-draft.md lines 450,476,478,486 | `P20-DEL-PROV-01` (not-assessable; mandatory) | — |
| K109 | S08 | lines 188,214-216; spec-draft.md lines 496 | `P20-DEL-PROV-03` (not-assessable; mandatory) | — |
| K110 | S08 | lines 92,212,214; spec-draft.md lines 492-500 | `P20-DEL-PKG-06` (not-assessable; mandatory) | — |
| K111 | S09 | L144 | `P20-SDK-AUTH-026` (not-assessable; mandatory) | — |
| K112 | S09 | L145 | `P20-SDK-AUTH-027` (not-assessable; mandatory) | — |
| K113 | S09 | L211 | `P20-SDK-PACKAGE-010` (not-assessable; mandatory) | — |
| K114 | S09 | L274 | `P20-SDK-PACKAGE-011` (not-assessable; mandatory) | — |
| K115 | S09 | L74 | `P20-SDK-CORE-038` (not-met; mandatory) | — |
| K116 | S09 | L99 | `P20-SDK-CORE-045` (not-assessable; mandatory) | — |
| K117 | S09 | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-rust-differences.md#rdf-21-upstream-data-profile | — | `P20-RDF-21` (partially-met; mandatory) |
| K118 | S10 | L108 | `P20-SDK-PROFILE-013` (not-met; mandatory) | — |
| K119 | S10 | L113 | `P20-SDK-PROFILE-030` (not-assessable; mandatory) | — |
| K120 | S10 | L117 | `P20-SDK-PROFILE-033` (not-assessable; mandatory) | — |
| K121 | S10 | L123 | `P20-SDK-IMAGE-002` (not-met; mandatory)<br>`P20-SDK-IMAGE-007` (not-met; mandatory) | — |
| K122 | S10 | L129 | `P20-SDK-CLASSIFIER-023` (not-assessable; mandatory)<br>`P20-SDK-CLASSIFIER-024` (not-assessable; mandatory) | — |
| K123 | S10 | L131 | `P20-SDK-CLASSIFIER-032` (not-met; mandatory) | — |
| K124 | S10 | L143 | `P20-SDK-OAUTH-OPENROUTER-004` (not-met; mandatory) | — |
| K125 | S10 | L151 | `P20-SDK-AUTH-006` (not-assessable; mandatory)<br>`P20-SDK-AUTH-007` (not-met; mandatory) | — |
| K126 | S10 | L153 | `P20-SDK-AUTH-018` (not-assessable; mandatory) | — |
| K127 | S10 | L159 | `P20-SDK-CORE-027` (not-assessable; mandatory)<br>`P20-SDK-CORE-028` (not-assessable; mandatory)<br>`P20-SDK-MODEL-001` (not-met; mandatory) | — |
| K128 | S10 | L160 | `P20-SDK-MODEL-003` (not-assessable; mandatory)<br>`P20-SDK-MODEL-004` (not-assessable; mandatory) | — |
| K129 | S10 | L163 | `P20-SDK-MODEL-016` (not-met; mandatory) | — |
| K130 | S10 | L165 | `P20-SDK-MODEL-021` (not-assessable; mandatory)<br>`P20-SDK-MODEL-022` (not-assessable; mandatory)<br>`P20-SDK-MODEL-023` (not-met; mandatory)<br>`P20-SDK-MODEL-026` (not-assessable; mandatory) | — |
| K131 | S10 | L166 | `P20-SDK-MODEL-029` (not-assessable; mandatory) | — |
| K132 | S10 | L168 | `P20-SDK-MODEL-030` (not-assessable; mandatory) | — |
| K133 | S10 | L175 | `P20-SDK-MODEL-038` (not-assessable; mandatory) | — |
| K134 | S10 | L176 | `P20-SDK-MODEL-040` (not-met; mandatory)<br>`P20-SDK-MODEL-041` (not-met; mandatory) | — |
| K135 | S10 | L177 | `P20-SDK-MODEL-042` (not-assessable; mandatory) | — |
| K136 | S10 | L186 | `P20-SDK-UTILITY-011` (not-met; mandatory) | — |
| K137 | S10 | L189 | `P20-SDK-UTILITY-022` (not-met; mandatory) | — |
| K138 | S10 | L19 | `P20-SDK-CORE-004` (not-met; mandatory) | — |
| K139 | S10 | L190 | `P20-SDK-UTILITY-028` (not-assessable; mandatory) | — |
| K140 | S10 | L191 | `P20-SDK-UTILITY-031` (not-assessable; mandatory)<br>`P20-SDK-UTILITY-032` (not-assessable; mandatory) | — |
| K141 | S10 | L192 | `P20-SDK-UTILITY-034` (not-assessable; mandatory) | — |
| K142 | S10 | L193 | `P20-SDK-UTILITY-035` (not-met; mandatory)<br>`P20-SDK-UTILITY-036` (not-assessable; mandatory) | — |
| K143 | S10 | L206 | `P20-SDK-CATALOG-016` (not-assessable; mandatory) | — |
| K144 | S10 | L211 | `P20-SDK-PACKAGE-001` (not-assessable; mandatory)<br>`P20-SDK-PACKAGE-002` (not-assessable; mandatory)<br>`P20-SDK-PACKAGE-003` (not-assessable; mandatory) | — |
| K145 | S10 | L212 | `P20-SDK-PACKAGE-004` (not-assessable; mandatory) | — |
| K146 | S10 | L216 | `P20-SDK-PACKAGE-005` (not-assessable; mandatory) | — |
| K147 | S10 | L218 | `P20-SDK-PACKAGE-006` (not-assessable; mandatory)<br>`P20-SDK-PACKAGE-007` (not-assessable; mandatory)<br>`P20-SDK-PACKAGE-009` (not-assessable; mandatory) | — |
| K148 | S10 | L220 | `P20-SDK-PACKAGE-008` (not-assessable; mandatory) | — |
| K149 | S10 | L93 | `P20-SDK-PROFILE-010` (not-assessable; mandatory) | — |
| K150 | S11 | RP-11,L114,L148 | `P20-MECH-REPLAY-016` (not-assessable; mandatory) | — |
| K151 | S12 | AIR-050 line 215; lines 274,478,488; matrix lines 473,532 | `P20-DEL-PKG-01` (not-assessable; mandatory) | — |
| K152 | S12 | DEL-01, line 40; lines 27,57,67,73,79,583 | `P20-DEL-01` (not-assessable; mandatory) | — |
| K153 | S12 | DEL-02, line 41; lines 210,274,535-536 | `P20-DEL-02` (not-met; mandatory) | — |
| K154 | S12 | DEL-03, line 42; lines 129,214,585 | `P20-DEL-03` (not-assessable; mandatory) | — |
| K155 | S12 | DEL-04, line 43; lines 177,399,504-510 | `P20-DEL-04` (not-met; mandatory) | — |
| K156 | S12 | DEL-05, line 44; lines 468-478 | `P20-DEL-05` (not-met; mandatory) | — |
| K157 | S12 | DEL-06, line 45; lines 482-488 | `P20-DEL-06` (not-assessable; mandatory) | — |
| K158 | S12 | DEL-07, line 46; lines 492-502 | `P20-DEL-07` (not-assessable; mandatory) | — |
| K159 | S12 | DEL-08, line 47; lines 176,338,411-415 | `P20-DEL-08` (not-met; mandatory) | — |
| K160 | S12 | DEL-09, line 48; lines 216,242,345-349,462-464 | `P20-DEL-09` (not-met; mandatory) | — |
| K161 | S12 | DEL-10, line 49; lines 243,373,450 | `P20-DEL-10` (not-met; mandatory) | — |
| K162 | S12 | DEL-11, line 50; lines 89,185,236-245,587 | `P20-DEL-11` (not-met; mandatory) | — |
| K163 | S12 | DEL-13, line 52; lines 518-539,589 | `P20-DEL-13` (not-met; mandatory) | — |
| K164 | S12 | DEL-14, line 53; lines 29,588-596 | `P20-DEL-14` (not-met; mandatory) | — |
| K165 | S12 | ONB-02 line 555; lines 211,296 | `P20-DEL-ONB-02-D` (not-met; mandatory) | — |
| K166 | S12 | ONB-02 line 555; matrix line 532 | `P20-DEL-ONB-02-A` (not-assessable; mandatory)<br>`P20-DEL-ONB-02-B` (not-assessable; mandatory) | — |
| K167 | S12 | ONB-03 line 556 | `P20-DEL-ONB-03-C` (not-assessable; mandatory) | — |
| K168 | S12 | ONB-03 line 556; matrix line 533 | `P20-DEL-ONB-03-D` (not-met; mandatory) | — |
| K169 | S12 | ONB-04 line 557; matrix line 534 | `P20-DEL-ONB-04-B` (not-met; mandatory)<br>`P20-DEL-ONB-04-C` (not-assessable; mandatory) | — |
| K170 | S12 | ONB-05 line 558; matrix line 535 | `P20-DEL-ONB-05-B` (not-met; mandatory) | — |
| K171 | S12 | ONB-06 line 559; matrix line 536 | `P20-DEL-ONB-06-A` (not-assessable; mandatory) | — |
| K172 | S12 | ONB-07 line 560; lines 562-570; matrix line 537 | `P20-DEL-ONB-07-C` (not-assessable; mandatory) | — |
| K173 | S12 | ONB-07 line 560; matrix line 537 | `P20-DEL-ONB-07-A` (not-assessable; mandatory) | — |
| K174 | S12 | ONB-07 line 560; matrix lines 537,558-603 | `P20-DEL-ONB-07-B` (not-assessable; mandatory) | — |
| K175 | S12 | docs/research/pi-ai-opi-ai-alignment/spec-draft.md#Acceptance dependencies and completion / 6 | — | `P20-EXIT-06` (partially-met; mandatory) |
| K176 | S12 | docs/research/pi-ai-opi-ai-alignment/spec-draft.md#Delivery decomposition / DEL-11 | — | `P20-S43` (partially-met; mandatory)<br>`P20-S51` (partially-met; mandatory) |
| K177 | S12 | docs/research/pi-ai-opi-ai-alignment/spec-draft.md#Delivery decomposition / DEL-14 | — | `P20-S58` (partially-met; mandatory)<br>`P20-S59` (partially-met; mandatory)<br>`DEL-14` (partially-met; mandatory) |
| K178 | S12 | docs/research/pi-ai-opi-ai-alignment/spec-draft.md#Requirements: binding, streams and authentication / AIR-007 | — | `AIR-007` (partially-met; mandatory) |
| K179 | S12 | docs/research/pi-ai-opi-ai-alignment/spec-draft.md#Requirements: full Pi 1.3.0 public coverage / AIR-067 | — | `AIR-067` (partially-met; mandatory) |
| K180 | S12 | docs/research/pi-ai-opi-ai-alignment/spec-draft.md#Requirements: standalone SDK calls and public outcomes / AIR-052 | — | `AIR-052` (partially-met; mandatory) |
| K181 | S12 | docs/research/pi-ai-opi-ai-alignment/spec-draft.md#SDK onboarding and parameter acceptance / ONB-02 | — | `ONB-02-20.57` (partially-met; mandatory) |
| K182 | S12 | docs/research/pi-ai-opi-ai-alignment/spec-draft.md#SDK onboarding and parameter acceptance / ONB-03 | — | `ONB-03-20.43` (partially-met; mandatory) |
| K183 | S12 | docs/research/pi-ai-opi-ai-alignment/spec-draft.md#SDK onboarding and parameter acceptance / ONB-04 | — | `ONB-04-20.51` (partially-met; mandatory) |
| K184 | S12 | docs/research/pi-ai-opi-ai-alignment/spec-draft.md#SDK onboarding and parameter acceptance / ONB-06 | — | `ONB-06-20.58` (partially-met; mandatory) |
| K185 | S12 | docs/research/pi-ai-opi-ai-alignment/spec-draft.md#SDK onboarding and parameter acceptance / ONB-07 | — | `ONB-07-20.59` (partially-met; mandatory) |
| K186 | S12 | docs/research/pi-ai-opi-ai-alignment/spec-draft.md#Upstream HTTP dependency profile; docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-rust-differences.md#rdf-22-upstream-http-profile | — | `P20-S12` (partially-met; mandatory) |
| K187 | S12 | lines 215,274,588; matrix line 473 | `P20-DEL-PKG-02` (not-met; mandatory) | — |
| K188 | S12 | lines 246,539,594-596; AIR-067 | `P20-DEL-EXIT-05` (not-assessable; mandatory) | — |
| K189 | S12 | lines 27,49,450; backend lines 186,206; matrix lines 507,550 | `P20-DEL-PKG-07` (not-assessable; mandatory) | — |
| K190 | S12 | lines 28,502; backend lines 186,206 | `P20-DEL-PLAT-02` (not-assessable; mandatory) | — |
| K191 | S12 | lines 28-29,502; backend lines 186,188,206 | `P20-DEL-PLAT-01` (not-assessable; mandatory) | — |
| K192 | S12 | lines 29,17,452,613 | `P20-DEL-EXIT-01` (not-met; mandatory) | — |
| K193 | S12 | lines 29,53,560,590 | `P20-DEL-EXIT-03` (not-assessable; mandatory) | — |
| K194 | S12 | lines 29,583,590,592 | `P20-DEL-EXIT-02` (not-met; mandatory) | — |
| K195 | S12 | lines 30-31,353 | `P20-DEL-SCOPE-06` (not-assessable; mandatory) | — |
| K196 | S12 | lines 32,36,57,73 | `P20-DEL-SCOPE-05` (not-assessable; mandatory) | — |
| K197 | S12 | lines 474-478,510 | `P20-DEL-PKG-03` (not-assessable; mandatory) | — |
| K198 | S12 | lines 482,486,488; backend lines 188,214-216 | `P20-DEL-PROV-02` (not-assessable; mandatory) | — |
| K199 | S12 | lines 482-488 | `P20-DEL-PKG-04` (not-assessable; mandatory) | — |
| K200 | S12 | lines 486; backend lines 188,216 | `P20-DEL-PLAT-03` (not-assessable; mandatory) | — |
| K201 | S12 | lines 496,498; backend lines 66,98,216 | `P20-DEL-PROV-04` (not-assessable; mandatory) | — |
| K202 | S12 | lines 496,500-502; backend lines 90-100,212-216 | `P20-DEL-PKG-05` (not-assessable; mandatory) | — |
| K203 | S12 | lines 510; backend lines 64,66,216 | `P20-DEL-PROV-06` (not-assessable; mandatory) | — |
| K204 | S12 | lines 520,528-539,589 | `P20-DEL-EXIT-04` (not-met; mandatory) | — |
| K205 | S12 | lines 528-529,535 | `P20-DEL-REPLACE-02` (not-assessable; mandatory) | — |
| K206 | S12 | lines 531-534 | `P20-DEL-REPLACE-03` (not-met; mandatory) | — |
| K207 | S12 | lines 536-538 | `P20-DEL-REPLACE-04` (not-met; mandatory) | — |
| K208 | S12 | lines 594-596 | `P20-DEL-EXIT-06` (not-assessable; mandatory) | — |
| K209 | S12 | lines 94,97,215,236,245 | `P20-DEL-PLAT-04` (not-assessable; mandatory) | — |
| K210 | S13 | TP-13,L61 | `P20-MECH-TRANSCRIPT-010` (not-met; mandatory) | — |

### Matrix verification

The generation command checks active-index and both sidecar raw-byte digests, complete referenced-ID selection, owning run identity, reciprocal current finding membership, and unchanged source bytes after rendering. It performs no product check and does not revise any original requirement state.

## READY continuation evidence and remaining scope

Continuation requested2026-11-04. The fixed pair was carried-in-and-touched; the initial baseline HEAD and active index were exact, and staged/untracked inventories stayed empty. After the later explicit commit request, the current source baseline is the prerequisite commit recorded in the header; the active index remains exact. Planning does not apply production fixes. D1/D3/D4/D6/D7/D9 are resolved by the concrete source-preserving designs/proofs above. All planning gates are resolved by the bounded designs, explicit one-input baseline acceptance and source-owner receipt below. Required repaired-byte green checks may be not-run at plan stage; F.1a is the later independent owner gate, not a new prerequisite run before remediation. All current applicability/product-meaning decisions are sealed; future failures stop for a newly bound plan rather than unbounded repair.

### New raw evidence

| Evidence | SHA-256 |
|---|---|
| `C:\Users\Luiz\AppData\Local\Temp\opi-p20-proxy-red-5rhuvd4l\src\lib.rs` | `eab63e5a88a699c2133025c748adb91b05a07891e7898c5acc80da06b5af322d` |
| `C:\Users\Luiz\AppData\Local\Temp\opi-p20-proxy-red-5rhuvd4l\Cargo.toml` | `78f3dcc48f74f7d7b0ed02b421877a48261a3c0c12fbc42cbfdfa620d74902ae` |
| `C:\Users\Luiz\AppData\Local\Temp\opi-p20-proxy-red-5rhuvd4l\Cargo.lock` | `83620a07d5bff950759b9ba56063ff90e1f57665601f2d70a63ba6d81d335efd` |
| `C:\Users\Luiz\AppData\Local\Temp\opi-p20-proxy-red-5rhuvd4l\probe-final.log` | `11254962807a7bc9a24155d7632546fdda426662bd2f3972b830f870e77a6971` |
| `C:\Users\Luiz\AppData\Local\Temp\opi-p20-proxy-red-5rhuvd4l\receipt-final.json` | `600b8263a529f9440da56458eaed4bb195616800f9bee1d99a99b76e8bcd5204` |
| `C:\Users\Luiz\AppData\Local\Temp\opi-p20-classifier-order-zfhygquw\probe-identities.json` | `94bea0954a45a58265cbe8971b56090a50a0238c176b97bf26706991206ab5e1` |
| `C:\Users\Luiz\AppData\Local\Temp\opi-p20-classifier-order-zfhygquw\probe.log` | `ca54b49bb329c878512e879225a0e253b7cd0b7777443e0f1fabe0a74ad32ec8` |
| `C:\Users\Luiz\AppData\Local\Temp\opi-p20-classifier-order-zfhygquw\planning-resolution.md` | `5db0ab19bc6e243a181f53c5b989cf493971fdc2e0e6266e36ca3ddac636ca80` |
| `C:\Users\Luiz\AppData\Local\Temp\opi-p20-classifier-order-zfhygquw\authority-draft.md` | `d5c5222309553e4437f24ee320b079f4abb585f6c84e893e8f81a1a342f61ae4` |
| `C:\Users\Luiz\AppData\Local\Temp\opi-p20-classifier-order-zfhygquw\tests\usage.rs` | `c0ef8e774689d4338ec7fe8b4edbd888f130d0c8fb6180fde22a3a5e249a7f24` |
| `C:\Users\Luiz\AppData\Local\Temp\opi-p20-classifier-order-zfhygquw\usage-final.log` | `c0e1f9391b0676a1b330cf271ef09b27c7b2d6a4bc1e750c17216465dcc44a5c` |
| `C:\Users\Luiz\AppData\Local\Temp\opi-p20-classifier-order-zfhygquw\usage-identities.json` | `16adefbb614d904d3adfb64282b8df598a6c2ed2dd1df87cd6a0ca4aafcb6b5f` |
| `C:\Users\Luiz\AppData\Local\Temp\opi-p20-expiry-red-wyr9zqbx\receipt.json` | `158695e0633e29acc04421178ed1de463c536cee647e980ddf26a19bb8211a9d` |
| `C:\Users\Luiz\AppData\Local\Temp\opi-p20-expiry-red-wyr9zqbx\closure-design.md` | `7735f9f70b31890195bbe23e2501c3c41830d2b7e7fdb987ae976b5a5ab55922` |
| `C:\Users\Luiz\AppData\Local\Temp\opi-p20-expiry-red-wyr9zqbx\affected-inputs.json` | `aa6a536198834a72b7d78729e27e98bf5208ece9f4771dbb537dfa6a2cc8bcfa` |
| `C:\Users\Luiz\AppData\Local\Temp\opi-p20-expiry-red-wyr9zqbx\expiry-red.log` | `f8e69202db2b8bde6c0024bbe1733b5776112afe22bbf14b575a7af08f7975a0` |
| `C:\Users\Luiz\AppData\Local\Temp\opi-phase20-remediate-plan-jqdki4yk\sdk-compat-proof-summary.json` | `358c56cbfe385ef9e29c5d6c84bbc3e32d3c8fdf7e5d6174ab0381a6b3a8f5be` |
| `C:\Users\Luiz\AppData\Local\Temp\opi-phase20-remediate-plan-jqdki4yk\sdk-compat-probes-final.log` | `ce71c6297705e869fb13ae05dee5087446739bb04959109ae369874fcbed52ab` |
| `C:\Users\Luiz\AppData\Local\Temp\opi-phase20-remediate-plan-jqdki4yk\linux-redirect-evidence\receipt.json` | `95f8f86a1904730664ce59b99d4d90e1b12fc90a63c2579d20110cd5ed7ec1aa` |
| `C:\Users\Luiz\AppData\Local\Temp\opi-phase20-remediate-plan-jqdki4yk\linux-redirect-evidence\before.log` | `9f9a3da3bfdb458654010152041090703ff2ce44ed63089f69def12659bd47b7` |
| `C:\Users\Luiz\AppData\Local\Temp\opi-phase20-remediate-plan-jqdki4yk\linux-redirect-evidence\close-header-control.log` | `33ecc72247fdbd512106182f8799834262b76837fc0a347b04d5edbe1872d527` |

The final15-case compatibility fixture uses one extra test-only transitive fastrand2.5.0 versus root2.4.1; runtime opi-ai rlib and dependency closure are byte/identity-equal to the archived baseline. The receipt records this finite test graph difference; it is not a product dependency change. Compiler/fixture setup failures are separately retained and not counted as behavioral red.

### Authorized workflow maintenance and materialization boundary

On2026-11-04 the user explicitly authorized the required ordered unique `closure_batches` design and synchronized validator/tests/docs, while declining `opi-implement plan`. The workflow files now implement the array contract; all fixed plan records use arrays. Audit member/index bytes and source requirement states remain unchanged. At that original workflow-maintenance stage, runtime proof bound the immutable d053965 export and no product remediation had been applied. The present baseline additionally contains only authorized B01; current source/probe applicability is separately established above.

The user subsequently authorized the commit. Workflow maintenance and the confirmed live ledger binding are committed in `68e276822185346db0bf57e31175dcac599ef7c7`. The fresh export/applicability receipt above fulfills the required remediation source rebinding. The original runtime export does not contain this maintenance, and workflow maintenance is not a source-drift/P.4 receipt. Failing original behavioral probes remain red evidence, not a claimed implementation result.

### Completed source-drift owner handoff

The user explicitly authorized limited opi-implement plan, then confirmed P.4 draft fe8085f2a430e95169ffb78876851b821158c9f4b05dce56fd486ee3ebbf5311 and selected write-only. P.0/P.2 were READY; current contract bytes before the appended bilingual tally correction are unchanged.16registered live hashes now match;active tasks0,68historical summary rows and phase_exit/frozen snapshot preserved. Installed live ledger SHA256 `8ed820e0a10830b70f453eddc01e1700ffdb68458534febefa22e220bc53780b` is now committed in `68e276822185346db0bf57e31175dcac599ef7c7` after the later explicit user request. Owner receipt `C:\Users\Luiz\AppData\Local\Temp\opi-p20-ready-final-dp5e95fw\implement-source-reconciliation-receipt.json` SHA256 `8ba4e834fd798722f61eaccd3a0fb45dbfb58d9b9d90d9ddc175d9f883b5f365`. No current runtime/audit success is inferred from the old archive. Canonical ledger changes belong to opi-implement, never a remediation apply batch.

That earlier owner-reconciliation stage used its own prerequisite-commit export. The present remediation source baseline is2eee4af and its fresh archive described above. The explicit live owner receipt remains the historical write-only installation fact; the later user-authorized commit materializes those exact bytes without changing its graph or scope. Preserve the exact committed ledger bytes during apply. Any further non-plan commit requires another committed remediation export/binding.

### D5 continuation scope

The complete156-row mapping and concrete candidate proof designs now appear below. Final repaired-byte green and later independent F1a remain future execution, with exact ownership preserved. The selected source/backend/input choices below are sealed; a test-only coverage candidate grants no conditional authority to change product semantics if a new failure appears.

### Fixed Anthropic technical decision artifact

The complete source-pinned branch declarations, defaults, precedence, caller inventory and response-cost choice are retained below. They are current proposed apply contracts, not historical closure or an applied result.

```json
{
  "finding": "P20-AUD-009",
  "status": "verified; branch implementation choices resolved; compound batch representation must be adjudicated explicitly",
  "original_severity": "Major",
  "final_severity": "Major",
  "closure_key": "sdk.anthropic.complete-compat.source-branches",
  "family_key": "sdk.anthropic-compat",
  "decision": "fix:preserve-and-execute-current-source-anthropic-compat-fields-and-defaults",
  "authority": [
    "registered pi-1.3.0-sdk-contracts.md176 explicitly names all flags, source defaults, affinity format, fallback cardinality/provider/model/cost and effectivefalse",
    "registered contracts37 requires server fallback,responseModel,cacheWrite1h,and signature replay; contracts159 complete metadata/owned roundtrip",
    "pinned compat-schema.ts412-505 and anthropic-messages.ts219-232,701-731,1072-1087,1152-1155,1203-1224,1254-1265,1278-1290,1326-1331,1442-1475,1564-1579 verified by evidence pins"
  ],
  "shared_metadata_prerequisite": {
    "public_fields": [
      "supports_long_cache_retention: bool defaulttrue",
      "supports_cache_control_on_tools: bool defaulttrue",
      "allow_empty_signature: bool defaultfalse",
      "supports_mid_convo_effort: bool defaultfalse",
      "send_session_affinity_headers: Option<bool> defaultNone (preserves contextual inference versus explicitfalse)",
      "session_affinity_format: Option<AnthropicSessionAffinityFormat> defaultNone",
      "allowed_fallback_models: Option<Vec<AnthropicAllowedFallbackModel>> defaultNone (empty list distinct metadata but same omit behavior)"
    ],
    "types": [
      "AnthropicSessionAffinityFormat enum with only OpenRouter, serde rename openrouter; avoids accepting OpenAI dialect values unavailable on this source wire",
      "AnthropicAllowedFallbackModel { provider: String, model: String, cost: ModelPricing }; reuse existing base+tiers pricing representation and validation"
    ],
    "defaults": "Replace derived Default on AnthropicMessagesCompat with explicit source defaults: eager,long-cache,tool-cache,temperature=true; force-adaptive,empty-signature,strict,mid-effort,mid-system,mid-toolchanges=false; optional affinity/fallback fieldsNone.",
    "decode_encode": "apply_source_metadata reads all source fields; compat_to_extension writes all resolved booleans and present optionals; parse_compat reads extended owned store shape with defaults for newly missing fields, retains existing required old fields; present source/extension wrong types reject, fallback list length<=3, provider/model nonempty, cost present and valid source metadata. New fields preserve through public ModelsStore.",
    "precedence": "Existing explicit x-opi.compat projection continues to win over source compat as documented at wire.rs865-867. For plain Anthropic source records, derive cacheControl=true and longCacheRetention=true unless explicit existing x-opi capability overrides; runtime also gates TTL on compat supports_long_cache_retention. Existing direct ModelInfo callers' explicit generic cache opt-out remains respected. Source compatfalse can suppress a branch even when generic capabilitytrue. Session enable/format retain None until concrete provider/baseURL resolution.",
    "current_red": [
      "c009_absent_source_true_defaults_remain_true: authentic fable-5 missing eager/temperature becomes false/false",
      "c009_fallback_local_prices_survive_public_store_roundtrip: source two fallback targets with rates disappear from persisted executable record"
    ],
    "caller_sync": [
      "crates/opi-ai/src/anthropic.rs literals957,1279: fill new defaults intentionally and preserve explicit conservative unknown-model fallback where current contract requires it",
      "crates/opi-ai/src/catalog/wire.rs literal505/source844/encoder157",
      "crates/opi-ai/tests/anthropic_fixtures.rs literals1711,2448,2849,3513",
      "crates/opi-ai/tests/node4_preparation.rs679",
      "crates/opi-ai/tests/telemetry_operations.rs903",
      "crates/opi-coding-agent/src/config.rs1927",
      "crates/opi-coding-agent/src/github_copilot.rs427"
    ]
  },
  "branch_units": [
    {
      "key": "anthropic.cache-marker-compat",
      "root_change": "Use the resolved Anthropic cache compatibility when assigning tool markers and long TTL",
      "predicate": "With cacheenabled, supportsCacheControlOnToolsfalse suppresses only tool cache_control; supportsLongCacheRetentionfalse retains ephemeral markers but omits1h TTL; true/default and Disabled controls obey their branches",
      "paths": [
        "crates/opi-ai/src/anthropic.rs",
        "crates/opi-ai/tests/anthropic_fixtures.rs"
      ],
      "red": [
        "c009_tool_cache_false_suppresses_only_tool_marker",
        "c009_long_cache_false_omits_ttl"
      ],
      "green_inventory": "sourceplain default markers,explicitfalse tools,explicitfalse TTL,independent combinations,true defaults,requestDisabled,owned x-opi generic opt-out,inline tool changes last initial-tool marker"
    },
    {
      "key": "anthropic.affinity-headers",
      "root_change": "Resolve and merge source affinity headers at Anthropic prepared request construction",
      "predicate": "Exact full nonempty sessionId uses x-session-affinity when enabled/defaultformat, x-session-id for explicit/inferredOpenRouter, explicitfalse/no id emits neither, model/caller headers and null tombstones preserve source precedence",
      "paths": [
        "crates/opi-ai/src/anthropic.rs",
        "crates/opi-ai/tests/anthropic_fixtures.rs"
      ],
      "decision": "Inference provider==openrouter OR baseURLcontainsopenrouter.ai. Enable explicit option wins over inference; format explicit wins over inferredOpenRouter otherwise defaultaffinity. Apply only the source API-key/header-owned/federation construction branch; source OAuth client returns earlier. Pinned consumer1075 gates sessionId+enabled, not cache retention; therefore retain affinity even with cacheDisabled. Defaults then affinity then model headers then caller headers; never overwrite a caller tombstone after merge.",
      "red": [
        "c009_explicit_affinity_sends_full_session_id"
      ],
      "control": "c009_affinity_false_control",
      "green_inventory": "Fireworksexplicittrue,OpenRouterprovider,OpenRouterURLalias,explicitfalseoverride,explicitformatOpenRouter,empty/nosession,cacheDisabled,headeroverride/tombstone,OAuthbranch"
    },
    {
      "key": "anthropic.empty-signature-replay",
      "root_change": "Thread allowEmptySignature into Anthropic content conversion after existing canonical identity projection",
      "predicate": "Sameidentity nonblank thinking with missing/empty/whitespace signature becomes thinking with signatureempty iff flagtrue, otherwise text; real nonblank signature/redacted payload stay unchanged; crossidentity discard and blankunsigned behavior remain source-consistent",
      "paths": [
        "crates/opi-ai/src/anthropic.rs",
        "crates/opi-ai/tests/anthropic_thinking_roundtrip.rs"
      ],
      "decision": "Do not make canonical replay globally permissive. Existing replay retains sameidentity nonblank unsigned material; adjust wire-specific conversion map to filter/map source branches. Source blank thinking and missing/whitespace signature drops at conversion; signed nonblank retains exact signature and sanitized text. Replay facts/strict mode still apply to identity loss.",
      "red": [
        "c009_allow_empty_signature_materializes_empty_wire_signature",
        "c009_disallow_empty_signature_projects_thinking_as_text"
      ],
      "green_inventory": "true/false x None/empty/whitespace/nonblank signature x blank/nonblankthinking,redacted,crossidentity"
    },
    {
      "key": "anthropic.mid-conversation-effort",
      "root_change": "Compile source managed-effort transcript and seed response effort from that compiled request",
      "predicate": "For flagged model with thinkingnotdisabled, sameprovider+exactAnthropicMessageswire historical supported effort markers precede corresponding assistant turns and current effort marker ends transcript; wire adaptive block_binding drop_block/output_confighigh/bothbetas and terminal provider_thinking_level match source; disabled/unflagged controls omit managed sequence",
      "paths": [
        "crates/opi-ai/src/anthropic.rs",
        "crates/opi-ai/tests/anthropic_fixtures.rs",
        "crates/opi-ai/tests/anthropic_thinking_roundtrip.rs"
      ],
      "decision": "Reuse existing AnthropicMessagesOptions effort/thinkingEnabled/display and existing AssistantMessage.provider_thinking_level; no new public field. activeeffort=nativeeffectiveeffort or sourcehigh. Emit historical effort only supported low/medium/high/xhigh/max, sameprovider, wireSome(AnthropicMessages) (source1493-1498 checks exactapi, not coarse ApiKind and does not require exact model match); preserve pinned conversion ordering. Suppress temperature whenever supportsMidConvoEfforttrue, including disabled branch. Disabled only when modelreasoning and thinkingEnabledfalse and offmappingnotnull; emits disabled with no binding or marker. Flagbetas emitted regardless of active managed branch exactly source1153. Do not add global effort normalizer.",
      "red": [
        "c009_mid_effort_emits_binding_and_stamps_response"
      ],
      "green_inventory": "activecurrenteffort,omittedhigh,historicalsameprovider,differentprovider/unsupportedhistorical,nohistory,explicitoffallowed/offnull,unflagged,tempomission,replyStart/Done/failurestamp"
    },
    {
      "key": "anthropic.server-fallback",
      "root_change": "Carry allowed fallback declaration through request and the source server-fallback response state machine",
      "predicate": "Nonempty allowed list emits ordered modelonly fallbacks and exactbeta; absent/empty omitsboth. Leading fallback marker is accepted without fakecontent, later output retains requested model and reportedresponseModel, midoutputfallback returns typed StreamError with canonicalpartial. Local cost resolution selects only matching sameprovider+responseModel allowed rates and otherwise original model rates.",
      "paths": [
        "crates/opi-ai/src/anthropic.rs",
        "crates/opi-ai/src/model_info.rs",
        "crates/opi-ai/tests/anthropic_fixtures.rs",
        "crates/opi-ai/tests/model_wire_metadata.rs"
      ],
      "decision": "Add raw/internal fallback event variant, ignore it only before any outputcontent, fail after contentwith source unsupportedmidoutput cause; ensure ignored source block index never shifts later text/tool block addressing. Preserve response_model reported value and requested model separately. Request beta server-side-fallback-2026-07-01; serialize models only, costs remain local. Reuse ModelPricing/calculate_cost_tiered via inherent public ModelInfo::calculate_response_cost(&AssistantMessage)->Option<CostBreakdown>: Anthropic selects allowed entry only if response.response_model differs from self.id and fallback.provider==response.provider and fallback.model==response.response_model; otherwise self.pricing. Then calculate with response usage and selected rates, preserving knownness/tier/cacheWrite1h behavior. Existing lowlevel calculate_cost_tiered stays available and unchanged for explicit pricing callers. No new AssistantMessage cost field, no provider fact rewrite, no fallbacknetworkcall or retry policy.",
      "red": [
        "c009_allowed_fallbacks_reach_request",
        "c009_fallback_local_prices_survive_public_store_roundtrip",
        "c009_leading_fallback_block_is_not_a_parse_failure"
      ],
      "controls": [
        "c009_empty_fallbacks_control",
        "c009_mid_output_fallback_preserves_partial_failure_control"
      ],
      "control_limit": "Current midoutputcontrol passes via malformed SSE classification, preservingpartial; future green must prove explicit server-fallback semantic cause, not treat this control as existingcorrectparser.",
      "green_inventory": "absent/empty/1/3targets,>3reject,missingprovider/model/costreject,orderedmodelonlypayload,beta,leadingmarkerthenoutput,midoutputtypedfailure+partial,reportedallowedmodelcost,sameidcost,unlisted/crossproviderfallbackcostbase,tiers/cachewrite1h,unknownprice/usageNone"
    }
  ],
  "exact_changed_paths_union": [
    "crates/opi-ai/src/model_info.rs",
    "crates/opi-ai/src/catalog/wire.rs",
    "crates/opi-ai/src/anthropic.rs",
    "crates/opi-ai/tests/model_wire_metadata.rs",
    "crates/opi-ai/tests/catalog_operations.rs",
    "crates/opi-ai/tests/anthropic_fixtures.rs",
    "crates/opi-ai/tests/anthropic_thinking_roundtrip.rs",
    "crates/opi-ai/tests/node4_preparation.rs",
    "crates/opi-ai/tests/telemetry_operations.rs",
    "crates/opi-coding-agent/src/config.rs",
    "crates/opi-coding-agent/src/github_copilot.rs",
    "crates/opi-ai/README.md",
    "crates/opi-ai/README.zh.md",
    "CHANGELOG.md"
  ],
  "verification": "Port each named external red to existing corresponding integration binary; execute affected model_wire_metadata/catalog_operations/anthropic_fixtures/anthropic_thinking_roundtrip/node4_preparation/telemetry_operations; verify config/Copilot callers and crosscrateworkspacegates. No Cargo dependency/feature/lockfile edits. Paired docs+Unreleased+doccheck required. Every created/modified binary/filter runs.",
  "test_impact": "add branch-specific public catalog/store/wire/replay cases; update existing sixfield struct literal fixtures to explicitly retain oldtestpolicy; retain unrelated provider branches",
  "remaining_product_authority": "No new product meaning is required: declarations/defaults/precedence/state transitions follow registered contracts and exact pinned consumers. The inherent response-cost method is a concrete Rust translation of already mandated local fallback pricing, sharing existing state/data rather than a new abstraction.",
  "workflow_shape_issue": "The user-authorized ordered closure_batches contract now represents this compound source as six independent roots. No technical product-authority issue remains; every root and the source conjunction must still be green at result stage."
}
```

### Current source-qualified proxy result

C010 now owns B97 and B28 as separate roots. The new source review corrects the earlier default-constructor assumption: no registered clause mandates that Rust constructor shape or automatic reinterpretation of supplied clients. The public utility has an actual ALL_PROXY common-domain red; explicit target/scoped snapshots and caller-owned HTTP/WS/AWS route tests are its bounded Rust API migration. The separate default builtin ADC path has its own gaxios red and policy. No Google-generation or AWS credential-source automatic proxy policy is invented.

### Workflow maintenance verification

TDD unsupported-array CLI red preceded validator production edits. Full owning assurance scripts: `python -m unittest discover -s .agents/skills/_shared/scripts -p test_*.py -v` ->168tests passed,0failures,exit0 (180.28s); raw log `C:\Users\Luiz\AppData\Local\Temp\opi-phase20-remediate-plan-jqdki4yk\workflow-tests.log`, SHA-256`535756d1c3dc5cb1723aca37bf0bc2ba74d32e20e9844add8bbdb65399127840`. This includes real subprocess/CLI malformed-array, mixed legacy-field, heading/dependency/source-key/result and materialization negatives. Added11regressions and updated fixtures; no Rust product tests were changed. Canonical workflow references have no Chinese counterpart files; aliases were untouched. Released changelog tail was independently hash-preserved.

### Final batch-source validation check

Independent review found and reproduced erroneous acceptance of a compound batch without its assigned source. The fix requires one canonical semicolon-separated run/digest/id tuple field per Fix and exact batch-local source union. Five new regression methods cover missing, altered, unassigned and duplicate attribution. After this change, `test_remediation_closure` passed30tests and `test_opi_assurance_skills test_remediation_source_proof` passed92tests (122affected tests total). The tightened valid duplicate-field regression also passed its focused rerun; no later production-script edit occurred. Direct current CLI validation of the intentionally missing-B2-source synthetic fixture returned exit1 with both malformed/missing assigned-source diagnostics. The earlier168-test full script run remains stage-specific evidence, not a claim that it ran after the subsequent attribution edit. Overall maintenance test impact: add16closure regressions, update shared fixtures, retain remaining controls, delete none.

## Complete current C018 criterion mapping and final proof routes

This is an exact156-record projection of the current indexed C018 requirements, not a new audit or a change to their original states. The disjoint partitions are38GroupA+63GroupB+55root. Every source path/digest/literal citation was checked against the current requirements sidecar. Current source/assertion inspection is labeled as such; future green is not claimed executed. The58 newly declared test-only roots each own a finite case matrix, and cannot authorize product edits on unexpected failure.

Formal batch count:99; all28finding records remain unique. Complete mapping raw artifact `C:\Users\Luiz\AppData\Local\Temp\opi-p20-ready-final-dp5e95fw\sealed-156-map.json` SHA256 `6b1293735ac3f41b81e111a8486b98f11632b37fcd153e1c9a549fffd586f142`.

| Requirement / original state | Exact current source identity | Current predicate evidence and limits | Assigned proof route / remaining scope |
|---|---|---|---|
| P20-V-09 / not-assessable | docs/research/pi-ai-opi-ai-alignment/acceptance-matrix.md @ c831586339910f9496a64e6b3c9390a0b7a2a366d37230222874a5f399099a5d; V-09, line 64 | AWS fixture corpus already exercises typed source chain, selected terminal denial, STS/ECS/IMDS/SSO/login and invalidated refresh; link Group-B finite proof and parent source proxy fix. crates/opi-ai/tests/bedrock_fixtures.rs:7123 Late rotation cannot overwrite replacement after invalidate; crates/opi-ai/tests/bedrock_fixtures.rs:6843 Selected denial no generation or alternate identity; crates/opi-ai/tests/bedrock_fixtures.rs:2574 Actual local placeholder-signing H1 fixed identity | auth; Execute the listed current tests and planned finite cases against the final package/source bytes; retain command/counts/exits. Existing test presence is not fresh pass evidence. |
| P20-V-16 / not-assessable | docs/research/pi-ai-opi-ai-alignment/acceptance-matrix.md @ c831586339910f9496a64e6b3c9390a0b7a2a366d37230222874a5f399099a5d; V-16, line 71 | SDK native/collapse/tool controls are substantial; native Responses effort/sign-in and Anthropic off/stamp finite conjunction needs explicit final proof. crates/opi-ai/tests/openai_responses_fixtures.rs:2958 Actual HTTP low history/high current markers, top-levelmedium and returned high stamp; crates/opi-ai/tests/openai_responses_fixtures.rs:4036 Explicit prepared sign-in qualification controls; crates/opi-ai/tests/transcript_contract/provider_projection.rs:549 Native/collapse/strict tool behavior | B44; Execute existing actual sign-in matrix for omitted/empty apiKey and header-only branches plus finite marker cases; parent owning auth compatibility fixes are dependencies. Host-impact active/visible revisions not declared completed. |
| P20-V-31 / not-assessable | docs/research/pi-ai-opi-ai-alignment/acceptance-matrix.md @ c831586339910f9496a64e6b3c9390a0b7a2a366d37230222874a5f399099a5d; V-31, line 93 | SDK tool addition/removal/redefinition/native-collapse and strict preauth behavior are exercised; product loadout/resume remains host-impact scope, not this SDK acceptance. crates/opi-ai/tests/transcript_contract/provider_projection.rs:492 Both compat flags and preserved inline placement; crates/opi-ai/tests/transcript_contract/provider_projection.rs:549 Removal/redefinition collapse or strict rejection; crates/opi-ai/tests/anthropic_fixtures.rs:3839 By-value Anthropic addition/redefinition | ; Run cargo test -p opi-ai --test transcript_contract and relevant actual Anthropic inline fixture; record canonical unchanged assertions. Keep host-impact implementation outside SDK completion. |
| P20-V-32 / not-assessable | docs/research/pi-ai-opi-ai-alignment/acceptance-matrix.md @ c831586339910f9496a64e6b3c9390a0b7a2a366d37230222874a5f399099a5d; V-32, line 94 | Existing actual roundtrips cover opaque order, same/foreign identities, signed-empty, terminal backfill and malformed metadata across Chat/Responses/Codex/Azure/Google/Vertex/Bedrock/Anthropic/Mistral. crates/opi-ai/tests/chat_capture_replay.rs:124 Chat/Responses/Azure/Codex/Anthropic actual producer→persist/reload→request assertions; crates/opi-ai/tests/vertex_fixtures.rs:777 Google/Vertex actual same/foreign signed-empty second requests; crates/opi-ai/tests/gemini_fixtures.rs:2715 Actual Google tool signature persistence/replay; crates/opi-ai/tests/bedrock_fixtures.rs:9751 Bedrock signed-empty/redacted bytes/order through persisted capture; crates/opi-ai/tests/mistral_fixtures.rs:2127 Actual Mistral content/effort and raw malformed tool provenance | B43; B44; Execute the listed current tests and planned finite cases against the final package/source bytes; retain command/counts/exits. Existing test presence is not fresh pass evidence. |
| P20-V-34 / not-assessable | docs/research/pi-ai-opi-ai-alignment/acceptance-matrix.md @ c831586339910f9496a64e6b3c9390a0b7a2a366d37230222874a5f399099a5d; V-34, line 96 | Actual strict variant ordinary/direct paths, per-variant retry bound, nonmarker and callback rejection already exist; complete all3marker and cancellation/Start exclusion table. crates/opi-ai/tests/anthropic_fixtures.rs:3137 Two sends atN0, Prefer removed/Require retained, callbacks and one diagnostic; crates/opi-ai/tests/anthropic_fixtures.rs:3347 Per-variant ordinary retry budget and total bound; crates/opi-ai/tests/anthropic_fixtures.rs:3420 Callback-phase failure no variant; crates/opi-ai/src/anthropic.rs:1983 Exactly3 literal source marker prefixes | B45; Execute the listed current tests and planned finite cases against the final package/source bytes; retain command/counts/exits. Existing test presence is not fresh pass evidence. |
| P20-MECH-SCHEMA-005 / not-assessable | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-schema.md @ 878d15231e734dc03772e1f1bd48cb8ffe2252a2818748e4b8630af43a2e8a76; L36 | Public finalizer retains raw provenance and arguments_error; schema normalizer rejects marker before coercion. Lone surrogate/non-JSON boundary needs explicit actual finalizer fixture. crates/opi-ai/src/message.rs:317 Actual public finalization boundary; crates/opi-ai/src/schema/normalize.rs:66 Arguments-error guard precedes coercion; crates/opi-ai/tests/schema_public.rs:474 Existing manually marked error control | B33; Execute the listed current tests and planned finite cases against the final package/source bytes; retain command/counts/exits. Existing test presence is not fresh pass evidence. |
| P20-MECH-SCHEMA-011 / not-assessable | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-schema.md @ 878d15231e734dc03772e1f1bd48cb8ffe2252a2818748e4b8630af43a2e8a76; L54 | Runtime URI leaf errors are fallible; existing anyOf short-circuit control does not alone cover every visited applicator. crates/opi-ai/tests/schema_engine.rs:939 bad runtime URI returns Err; anyOf true skips bad ref | B34; Execute the listed current tests and planned finite cases against the final package/source bytes; retain command/counts/exits. Existing test presence is not fresh pass evidence. |
| P20-MECH-SCHEMA-016 / not-assessable | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-schema.md @ 878d15231e734dc03772e1f1bd48cb8ffe2252a2818748e4b8630af43a2e8a76; L60 | Existing public engine fixtures exercise distinct lexical/resource/reference bases and dynamic/recursive return frames. crates/opi-ai/tests/schema_engine.rs:727 Repeated nested metadata target visits retain return frames; crates/opi-ai/tests/schema_engine.rs:875 Shared target visited from distinct lexical bases; crates/opi-ai/tests/schema_engine.rs:1131 Dynamic and recursive target/base interactions; crates/opi-ai/tests/schema_engine.rs:699 Duplicate ID selected in source enumeration order; crates/opi-ai/tests/schema_engine.rs:902 Legacy and resolved-resource frames remain distinct | schema; Execute the listed current tests and planned finite cases against the final package/source bytes; retain command/counts/exits. Existing test presence is not fresh pass evidence. |
| P20-MECH-SCHEMA-017 / not-assessable | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-schema.md @ 878d15231e734dc03772e1f1bd48cb8ffe2252a2818748e4b8630af43a2e8a76; L60,L62 | Existing fixtures cover relative IDs, metadata and duplicate/legacy frames; explicit encoded-segment target controls make remaining resolver proof finite. crates/opi-ai/tests/schema_engine.rs:727 Repeated nested metadata target visits retain return frames; crates/opi-ai/tests/schema_engine.rs:875 Shared target visited from distinct lexical bases; crates/opi-ai/tests/schema_engine.rs:1131 Dynamic and recursive target/base interactions; crates/opi-ai/tests/schema_engine.rs:699 Duplicate ID selected in source enumeration order; crates/opi-ai/tests/schema_engine.rs:902 Legacy and resolved-resource frames remain distinct | B35; Execute the listed current tests and planned finite cases against the final package/source bytes; retain command/counts/exits. Existing test presence is not fresh pass evidence. |
| P20-MECH-SCHEMA-021 / not-assessable | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-schema.md @ 878d15231e734dc03772e1f1bd48cb8ffe2252a2818748e4b8630af43a2e8a76; L68 | Private immutable dispatcher contains exactly21 source algorithms and unknown/nonstring inert branch; add public compile/check table. crates/opi-ai/src/schema/formats.rs:517 Fixed21 dispatch, no public mutable registry | B36; Execute the listed current tests and planned finite cases against the final package/source bytes; retain command/counts/exits. Existing test presence is not fresh pass evidence. |
| P20-MECH-SCHEMA-025 / not-assessable | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-schema.md @ 878d15231e734dc03772e1f1bd48cb8ffe2252a2818748e4b8630af43a2e8a76; L70,L72 | Early packaged public consumer exists but is explicitly a smoke and not final acceptance. scripts/pi-sdk-conformance/run_early_package_proof.py:9 Actual package-runner CLI | schema; Execute the listed current tests and planned finite cases against the final package/source bytes; retain command/counts/exits. Existing test presence is not fresh pass evidence. |
| P20-MECH-SCHEMA-031 / not-assessable | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-schema.md @ 878d15231e734dc03772e1f1bd48cb8ffe2252a2818748e4b8630af43a2e8a76; L78 | Compiled Arc ownership and installed Agent schema authority are present; add discriminating old/new generation actual preparation control. crates/opi-ai/src/schema/mod.rs:121 Owned generation; crates/opi-agent/tests/tool_authority.rs:290 Changed installed schema rejects before tool/authorizer | B37; Execute the listed current tests and planned finite cases against the final package/source bytes; retain command/counts/exits. Existing test presence is not fresh pass evidence. |
| P20-MECH-SCHEMA-032 / not-assessable | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-schema.md @ 878d15231e734dc03772e1f1bd48cb8ffe2252a2818748e4b8630af43a2e8a76; L78 | SubValidators cache is invocation-local reusable HashMap, not a quota; no public tuning setting exists. crates/opi-ai/src/schema/normalize.rs:132 Cache misses compile, no admission-count branch | B38; Execute the listed current tests and planned finite cases against the final package/source bytes; retain command/counts/exits. Existing test presence is not fresh pass evidence. |
| P20-MECH-SCHEMA-033 / not-assessable | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-schema.md @ 878d15231e734dc03772e1f1bd48cb8ffe2252a2818748e4b8630af43a2e8a76; L78 | Agent uses spawn_blocking and cancellation selects; existing candidate validation gate alone cannot prove actual schema worker boundary. crates/opi-agent/src/agent_loop.rs:494 Blocking work and cancellation selects; inspect both normalization and schema validation callers | B39; Execute the listed current tests and planned finite cases against the final package/source bytes; retain command/counts/exits. Existing test presence is not fresh pass evidence. |
| P20-MECH-SCHEMA-034 / not-assessable | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-schema.md @ 878d15231e734dc03772e1f1bd48cb8ffe2252a2818748e4b8630af43a2e8a76; L78 | No added schema/fuel/work-count cap in current engine/normalizer. Finite large cases discriminate a quota but do not prove arbitrary completion. crates/opi-ai/src/schema/normalize.rs:132 Cache contains no cap; crates/opi-ai/tests/schema_public.rs:803 Actual deep public normalization/clone/drop assertions | B38; schema; Execute the listed current tests and planned finite cases against the final package/source bytes; retain command/counts/exits. Existing test presence is not fresh pass evidence. |
| P20-MECH-SCHEMA-035 / not-assessable | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-schema.md @ 878d15231e734dc03772e1f1bd48cb8ffe2252a2818748e4b8630af43a2e8a76; L80,L86,L88 | Current64-case frozen corpus and deep/resolver fixtures exist; early external43-leaf smoke does not establish full mechanism differential proof. crates/opi-ai/tests/schema_engine.rs:31 Actual64-case frozen corpus checks exact expected compile/check outcomes; scripts/pi-sdk-conformance/run_early_package_proof.py:9 Early package smoke runner | B33; B34; B35; B36; B37; B38; schema; platform; Execute the listed current tests and planned finite cases against the final package/source bytes; retain command/counts/exits. Existing test presence is not fresh pass evidence. |
| P20-MECH-SCHEMA-036 / not-assessable | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-schema.md @ 878d15231e734dc03772e1f1bd48cb8ffe2252a2818748e4b8630af43a2e8a76; L86,L88 | Cross-platform and MSRV execution is an explicit future evidence matrix, not established by local source.  | B39; schema; platform; Execute the listed current tests and planned finite cases against the final package/source bytes; retain command/counts/exits. Existing test presence is not fresh pass evidence. |
| P20-MECH-WS-006 / not-assessable | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-ws.md @ 61547ccfc0549018d2fd4042ade8edac3547599b412e12ea2479821b40f67a8f; L100,L130,L134 | Actual public package consumer drives real Codex chain; separate instrumented current probe also reached actual production chain. scripts/pi-sdk-conformance/consumer-ws/src/main.rs:274 Real Models/Codex/Upgrade/window8/Close path | ws; Execute the listed current tests and planned finite cases against the final package/source bytes; retain command/counts/exits. Existing test presence is not fresh pass evidence. |
| P20-MECH-WS-023 / not-assessable | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-ws.md @ 61547ccfc0549018d2fd4042ade8edac3547599b412e12ea2479821b40f67a8f; L110 | Confirmed behavioral defect: same logical request cancellation does not reach Decoder before each backend call within one work quantum. crates/opi-ai/src/ws/pmd.rs:260 Private decoder polls its own atomic; crates/opi-ai/src/openai_codex_responses/ws_transport.rs:941 Logical CancellationToken checked at outer select; actual probe exposes inside-quantum gap | B29; Execute the listed current tests and planned finite cases against the final package/source bytes; retain command/counts/exits. Existing test presence is not fresh pass evidence. |
| P20-MECH-WS-029 / not-assessable | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-ws.md @ 61547ccfc0549018d2fd4042ade8edac3547599b412e12ea2479821b40f67a8f; L116,L132 | Actual ordinary/default/C-union encoder graphs and direct Rust decoders must all execute. scripts/pi-sdk-conformance/package_slice.py:127 Profile definitions, extraction/checks and native provenance runner | ws; Execute the listed current tests and planned finite cases against the final package/source bytes; retain command/counts/exits. Existing test presence is not fresh pass evidence. |
| P20-MECH-WS-033 / not-assessable | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-ws.md @ 61547ccfc0549018d2fd4042ade8edac3547599b412e12ea2479821b40f67a8f; L120 | Full compression supply-chain identity plus finite reset/tail/truncation/ownership/lifecycle controls must be recorded per graph. scripts/pi-sdk-conformance/package_slice.py:127 Profile definitions, extraction/checks and native provenance runner | B29; ws; Execute the listed current tests and planned finite cases against the final package/source bytes; retain command/counts/exits. Existing test presence is not fresh pass evidence. |
| P20-MECH-WS-036 / not-assessable | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-ws.md @ 61547ccfc0549018d2fd4042ade8edac3547599b412e12ea2479821b40f67a8f; L122,L132 | Both AWS-LC and libz-sys native construction/static linkage identities required for C-union. scripts/pi-sdk-conformance/package_slice.py:127 Profile definitions, extraction/checks and native provenance runner | ws; Execute the listed current tests and planned finite cases against the final package/source bytes; retain command/counts/exits. Existing test presence is not fresh pass evidence. |
| P20-MECH-WS-039 / not-assessable | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-ws.md @ 61547ccfc0549018d2fd4042ade8edac3547599b412e12ea2479821b40f67a8f; L130 | Extracted package must include private tests/fixtures/licenses and execute its private checks separately. scripts/pi-sdk-conformance/package_slice.py:127 Profile definitions, extraction/checks and native provenance runner | ws; Execute the listed current tests and planned finite cases against the final package/source bytes; retain command/counts/exits. Existing test presence is not fresh pass evidence. |
| P20-MECH-WS-040 / not-assessable | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-ws.md @ 61547ccfc0549018d2fd4042ade8edac3547599b412e12ea2479821b40f67a8f; L130 | External consumer must depend only on real registry SDK package and execute all3 real Codex profiles. scripts/pi-sdk-conformance/package_slice.py:127 Profile definitions, extraction/checks and native provenance runner | B29; ws; Execute the listed current tests and planned finite cases against the final package/source bytes; retain command/counts/exits. Existing test presence is not fresh pass evidence. |
| P20-MECH-WS-041 / not-assessable | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-ws.md @ 61547ccfc0549018d2fd4042ade8edac3547599b412e12ea2479821b40f67a8f; L134 | All graph consumers and typed failures must execute on each named native platform and MSRV. scripts/pi-sdk-conformance/package_slice.py:127 Profile definitions, extraction/checks and native provenance runner | B29; ws; platform; Execute the listed current tests and planned finite cases against the final package/source bytes; retain command/counts/exits. Existing test presence is not fresh pass evidence. |
| P20-MECH-HTTP-003 / not-assessable | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-transport-auth.md @ 42e1bc90c02e958ada4d2a01db999cd906ae33ee971496c50c07ed38067cdcfb; L37 | Existing redirect fixtures prove hop counts and header stripping; prepared credential audience/body destination must use the parent verified source-qualified redirect fix. crates/opi-ai/tests/http_acceptance.rs:533 Exact20-hop policy control; crates/opi-ai/src/http.rs:386 Backend follows selected redirect policy; does not by itself prove account/audience preservation | auth; Link parent redirect/audience behavioral batch and execute its discriminating same/cross-origin body/header/auth-resolution controls; retain current20-hop and None controls. |
| P20-MECH-HTTP-005 / not-assessable | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-transport-auth.md @ 42e1bc90c02e958ada4d2a01db999cd906ae33ee971496c50c07ed38067cdcfb; L39 | Actual current controlled HttpClient loses CONNECT coalesced suffix (one negative handoff test FAIL), not a normal successful TLS negotiation claim. Locked public type composition and Hyper Rewind/reqwest matcher code support the selected ordinary-backend owner mechanism; actual repaired wire green not-run. | B99 single controlled HttpClient dispatch; preserved original negative byte-handoff schedules plus real positive TLS/proxy/H1-H2/Upgrade/pool/cancel/typed-cause and public/package/native matrix. |
| P20-MECH-HTTP-006 / not-assessable | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-transport-auth.md @ 42e1bc90c02e958ada4d2a01db999cd906ae33ee971496c50c07ed38067cdcfb; L47 | Current ordinary reqwest/Hyper and adaptive parser behavior match selected RDF22 mechanism; no fork or namespace is proposed. crates/opi-ai/src/http.rs:382 Ordinary backend with explicit no hidden retry; Cargo.lock:3081 Locked registry identity | http; Execute the listed current tests and planned finite cases against the final package/source bytes; retain command/counts/exits. Existing test presence is not fresh pass evidence. |
| P20-MECH-HTTP-007 / not-assessable | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-transport-auth.md @ 42e1bc90c02e958ada4d2a01db999cd906ae33ee971496c50c07ed38067cdcfb; L47 | Current integration binary already carries finite header-count/read-buffer/trailer/chunk-extension boundaries with bytewise/coalesced schedules and typed phases; package consumer needs those identities too. crates/opi-ai/tests/http_acceptance.rs:932 Actual99/100/101 header cases, plus read-buffer912/980, trailer1116 and chunk-extension1192/1219; both delivery schedules, exact cause and phase | http; Execute the listed current tests and planned finite cases against the final package/source bytes; retain command/counts/exits. Existing test presence is not fresh pass evidence. |
| P20-MECH-HTTP-008 / not-assessable | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-transport-auth.md @ 42e1bc90c02e958ada4d2a01db999cd906ae33ee971496c50c07ed38067cdcfb; L47 | Current mid-body abort and accepted-head tests cover typed partial failure; add known usage+retry-terminal conjunction without is_err-only claims. crates/opi-ai/tests/http_acceptance.rs:830 Actual partial body failure fixture | B41; http; Execute the listed current tests and planned finite cases against the final package/source bytes; retain command/counts/exits. Existing test presence is not fresh pass evidence. |
| P20-MECH-HTTP-009 / not-assessable | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-transport-auth.md @ 42e1bc90c02e958ada4d2a01db999cd906ae33ee971496c50c07ed38067cdcfb; L47 | Conflicting CL/TE/chunk/trailer controls and actual pool reuse exist; finite interim/obs-fold and lifecycle additions complete omitted categories. crates/opi-ai/tests/connection_pooling.rs:125 Two full H1 calls share one socket; disabled-pool control uses two | B42; B41; http; Execute the listed current tests and planned finite cases against the final package/source bytes; retain command/counts/exits. Existing test presence is not fresh pass evidence. |
| P20-MECH-HTTP-010 / not-assessable | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-transport-auth.md @ 42e1bc90c02e958ada4d2a01db999cd906ae33ee971496c50c07ed38067cdcfb; L51 | Actual Bedrock H2/H1 signed capture/reload test and WSS forced H1 control exist; parent source proxy/default profile checks remain dependencies. crates/opi-ai/tests/bedrock_fixtures.rs:386 Real TLS ALPN h2/http1.1 and actual signed requests; crates/opi-ai/tests/ws_upgrade.rs:86 Classic WS H1 while ordinary profile remains H2 preferred | http; crypto; auth; Execute the listed current tests and planned finite cases against the final package/source bytes; retain command/counts/exits. Existing test presence is not fresh pass evidence. |
| P20-MECH-TLS-006 / not-assessable | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-transport-auth.md @ 42e1bc90c02e958ada4d2a01db999cd906ae33ee971496c50c07ed38067cdcfb; L65 | Current explicit pinned Node bundle/default, extra roots and host-root opt-in controls exist; actual package must retain resource/hash. crates/opi-ai/src/tls/node_trust.rs:6 Fixed Node bundle digest; crates/opi-ai/tests/tls_contract.rs:548 Actual TLS1.3/H2 explicit-root request; companion extra570,default1629,wronghost/expired800 and untrusted780 controls | crypto; Execute the listed current tests and planned finite cases against the final package/source bytes; retain command/counts/exits. Existing test presence is not fresh pass evidence. |
| P20-MECH-TLS-013 / not-assessable | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-transport-auth.md @ 42e1bc90c02e958ada4d2a01db999cd906ae33ee971496c50c07ed38067cdcfb; L93 | Package runner owns ordinary/caller-union native source/toolchain/static-link provenance; unexecuted graph combinations cannot inherit this proof. scripts/pi-sdk-conformance/package_slice.py:127 crypto ordinary/caller-union and native validation | crypto; Execute the listed current tests and planned finite cases against the final package/source bytes; retain command/counts/exits. Existing test presence is not fresh pass evidence. |
| P20-MECH-TLS-014 / not-assessable | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-transport-auth.md @ 42e1bc90c02e958ada4d2a01db999cd906ae33ee971496c50c07ed38067cdcfb; L93 | Native TLS/signing per-platform and MSRV proof is an executable matrix owned by parent.  | crypto; platform; Execute the listed current tests and planned finite cases against the final package/source bytes; retain command/counts/exits. Existing test presence is not fresh pass evidence. |
| P20-MECH-REPLAY-016 / not-assessable | docs/research/pi-ai-opi-ai-alignment/replay-contracts.md @ 223b21e1aa590d0e4646d99cd7f6792a694c7d8bf0f34b31dd6380979d324fa4; RP-11,L114,L148 | Actual second-request evidence exists across all listed wire families; explicit OpenRouter facade route case completes adopted-adapter inventory without duplicating mechanisms. crates/opi-ai/tests/chat_capture_replay.rs:124 Chat/Responses/Azure/Codex/Anthropic actual producer→persist/reload→request assertions; crates/opi-ai/tests/vertex_fixtures.rs:777 Google/Vertex actual same/foreign signed-empty second requests; crates/opi-ai/tests/gemini_fixtures.rs:2715 Actual Google tool signature persistence/replay; crates/opi-ai/tests/bedrock_fixtures.rs:9751 Bedrock signed-empty/redacted bytes/order through persisted capture; crates/opi-ai/tests/mistral_fixtures.rs:2127 Actual Mistral content/effort and raw malformed tool provenance | B43; Execute the listed current tests and planned finite cases against the final package/source bytes; retain command/counts/exits. Existing test presence is not fresh pass evidence. |
| P20-DEL-06 / not-assessable | docs/research/pi-ai-opi-ai-alignment/spec-draft.md @ 8c6cd66f7771e026210f06e49d037da54029c74258a1d45925cdbddd710f5f21; DEL-06, line 45; lines 482-488 | TLS/trust/signing/KX behavior is covered by real TLS fixtures; final package native source/build/provenance and profile conjunction remains executable future proof. crates/opi-ai/tests/tls_contract.rs:619 Actual mTLS accepted profiles; P521662,identity-isolation686,untrusted780,KXnegative896,instance provider1605/1722 | crypto; platform; Execute the listed current tests and planned finite cases against the final package/source bytes; retain command/counts/exits. Existing test presence is not fresh pass evidence. |
| P20-DEL-07 / not-assessable | docs/research/pi-ai-opi-ai-alignment/spec-draft.md @ 8c6cd66f7771e026210f06e49d037da54029c74258a1d45925cdbddd710f5f21; DEL-07, line 46; lines 492-502 | Real Codex package chain, private protocol and cleanup tests exist; per-backend logical cancellation has genuine red and must close before delivery conjunction. scripts/pi-sdk-conformance/consumer-ws/src/main.rs:274 Public real Codex production chain; crates/opi-ai/src/openai_codex_responses/ws_transport.rs:941 Actual stream/lease owner | B29; ws; platform; Execute the listed current tests and planned finite cases against the final package/source bytes; retain command/counts/exits. Existing test presence is not fresh pass evidence. |
| P20-V-01 / not-assessable | docs/research/pi-ai-opi-ai-alignment/acceptance-matrix.md @ c831586339910f9496a64e6b3c9390a0b7a2a366d37230222874a5f399099a5d; V-01, line 56 | provider_collection4046–4096 old prepared binding survives catalog removal;4957–5045 complete owned id/baseURL/headers;5976–6097 one source read/payload and frozen Azure endpoint/deployment across retries. | B46; selection_frozen_four_entries: old call keeps old full record after replacement; new call uses new; endpoint_after_auth_defaults: absent compat uses final endpoint once, explicitfalse wins; invalid preauth/authdependent input fails beforeauth/HTTP; effort_requested_effective: higherthenlower simple clamp, nativeunsupported reject. |
| P20-V-26 / not-assessable | docs/research/pi-ai-opi-ai-alignment/acceptance-matrix.md @ c831586339910f9496a64e6b3c9390a0b7a2a366d37230222874a5f399099a5d; V-26, line 81 | classifier_contract3390–3573 actual4protocol results and distinct noul/predicate payload+headers; external_sdk630–899 fourclassifier/image dispatch but Decisions case is504failure. | B47; typed_nonchat_effects: all4success+typedfailure and image success+receivedfailure via localpeers, exactcallbacks/payload/accounting; no files/tool execution; package metadata shows onlyopi-ai Opi runtime. |
| P20-V-38 / not-assessable | docs/research/pi-ai-opi-ai-alignment/acceptance-matrix.md @ c831586339910f9496a64e6b3c9390a0b7a2a366d37230222874a5f399099a5d; V-38, line 100 | deferred_contract828–910 fetchcounts1/2/3,8000/15000 due times,no earlyfetch,onecommit,no resubmit,joined scheduler;914–970 caller5sfallback,no SDK inventedhints,actual fresh-token cancel1 afterlocalabort,missingmodelnofallback. | retain; Retain and rerun exact reopened_consumer_fetches_without_resubmit_and_never_polls plus consumer_fallback_missing_model_and_cancel_failures_are_observable. |
| P20-V-42 / not-assessable | docs/research/pi-ai-opi-ai-alignment/acceptance-matrix.md @ c831586339910f9496a64e6b3c9390a0b7a2a366d37230222874a5f399099a5d; V-42, line 185 | external_sdk1330–1414 uses differently named routes, insufficient same-id real-instance isolation/cleanup proof.1425–1468 assertsstatic zeroenvreads. | B17; Reuse existing C017 exact19case consumer design: fourentrances/directcustom/Serde/toolloop/Send+static/unpolledzeroIO/real2instances/explicitsharedstore/cleanup; run fromactual extracted.crate. |
| P20-V-53 / not-assessable | docs/research/pi-ai-opi-ai-alignment/acceptance-matrix.md @ c831586339910f9496a64e6b3c9390a0b7a2a366d37230222874a5f399099a5d; V-53, line 216 | external_sdk1129 OpenAIChat duration+2Serde rounds; Responsesfixture3052 nativecomplete Some. New duration probe: ResponsesSome0 PASS, Anthropic valid activated successfulreplyNone FAIL. New copied-and-instrumented current external_sdk294 matrix executed all10originalHTTPsuccess+peer assertions; Anthropic,Mistral,CodexHTTP,Radius,Bedrock durationNone; OpenAIResponses,OpenRouterChat,Azure,Google,VertexSome0. See c018-duration-matrix-receipt.json; WS was then separatelyprobed:currentconsumer plain+compressedrealchainoriginalcontrols pass, bothdurationNone;8othercontrolsPASS. See c018-ws-duration-receipt.json. | B31; duration_success_matrix:10wires x direct/native stream/native complete/simple stream/simple complete; terminalmatrix10xpartialerror/cancel; privateclockcontrols0/17/preexisting7/deferredready7/unpolled/setupfailure/toolowner9. See dedicated duration batch. |
| P20-V-56 / not-assessable | docs/research/pi-ai-opi-ai-alignment/acceptance-matrix.md @ c831586339910f9496a64e6b3c9390a0b7a2a366d37230222874a5f399099a5d; V-56, line 219 | catalog_lifecycle106–146 persisted tombstone preventsstalecommit/reopen;151–233 metadata+corruptfield; catalog_personalization67–200 chat-only/accountseparation/no-key/401403404 preserve; catalog_operations224–399 mixed3operation store/reopen→wire. | B48+B94; radius_empty_reopen: populated→empty authoritative→failure→reopen no resurrection; protocol_versions: distributionv1 vs hydrationv6,index/minimumPiVersion explicitcompatversion distinct RustUA,malformed/unknown-required reject beforepublish. |
| P20-SDK-CORE-027 / not-assessable | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md @ 9e3d6ef690c3a794bc9c47ddc2dce04840f98d8881649be1ab2e8db0df287fd5; L159 | catalog_operations224–399 preserves3operation input/inputLimits/output/pricing/headers/sampling/routing throughstore/reopen/actualwire;652–672 unknownimagecost andtierorder. C019/C009 currentred show missingpromptCache/fallbackprices. | B49; owned_model_full_roundtrip: onecompletechat,image,classifier with everydeclaredmetadata field; absent/empty/null controls and actualwire metadata use. |
| P20-SDK-CORE-028 / not-assessable | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md @ 9e3d6ef690c3a794bc9c47ddc2dce04840f98d8881649be1ab2e8db0df287fd5; L159 | catalog_operations190–196 unknownoperation publicdecode Err; store Corrupt typed on malformedentries. | B50; store_unknown_model_type: mutatevalidmixedsavedmodeltype; reopen ModelsStoreError::Corrupt (unique selected outcome), zerochat reinterpretation/auth/send;3knownoperation control. |
| P20-SDK-CORE-045 / not-assessable | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-rust-differences.md @ 411d2f6d262b1afc22e84c73213dffd0eb890ed70e210dfa497d2a5d021cd31d; L99 | LinkedSDKL99 is stale (providerrow); actualvalidation in124–131/163/165/190. classifier562/3074 nonfiniteanswers; model_info1153–1164 finitepricing; external_sdk1477 lawful>64messages. | B51; source_validation_caps:65messages/65questions/>64KiB lawfulmetadata/deepJSON beyondoldcap accepted; missingrequired/wrongtype/nonfinite failtyped; sourceDecisions129images stillreject; u64checkedarithmetic nooverflow. |
| P20-SDK-PROFILE-010 / not-assessable | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md @ 9e3d6ef690c3a794bc9c47ddc2dce04840f98d8881649be1ab2e8db0df287fd5; L93 | Existing SDK canonical_route accepts custom id/empty catalog; register_route retains profile name; Radius OAuth has with_identity. Actual3test public probe has2controls pass and identity propagation FAIL despite correct old/new gateways. | B32 nondefault baseline + B96 canonical neutral event/terminal/partial identity + B48 empty-catalog lifecycle; existing public input retained, no missing id/name gate. |
| P20-SDK-PROFILE-030 / not-assessable | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md @ 9e3d6ef690c3a794bc9c47ddc2dce04840f98d8881649be1ab2e8db0df287fd5; L113 | api/cloudflare73 explicit HttpClient+complete record; ClassifierApi is current hostadapter seam; SDKcontracts113 permits platformadapter without JSobjectABI. | B52; cloudflare_binding_explicit: caller-owned ClassifierApi with injectedhandle; onecall exactenvelope/result/failure; zeroWorkerenv/URLconversion; RESTconstructor localpeer control. No new Worker transporttrait. |
| P20-SDK-PROFILE-033 / not-assessable | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md @ 9e3d6ef690c3a794bc9c47ddc2dce04840f98d8881649be1ab2e8db0df287fd5; L117 | bedrock2574–2711 realH1/dummySigV4/region/canaryabsent/zeroenv/Done onModels path; no direct retry/cancel/drop coverage. | B53; placeholder_lifecycle:Models+direct x success/503retry/cancel-beforeheaders/drop-midbody; readcompletebody,H1,placeholderAuth,callbacks,sends,terminal/release; realcredentialcanarieszeroIO; standardAWSendpointzero-send negative. |
| P20-SDK-AUTH-006 / not-assessable | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md @ 9e3d6ef690c3a794bc9c47ddc2dce04840f98d8881649be1ab2e8db0df287fd5; L151 | models226–235 onlykey/bearer/SigV4 overrides; AuthResolver.resolve nooptions; auth598–606fixed5min; store_oauth132–141postACKactualexpiryonly. oauth_store_integration198 accepts120s defaultcontrol. | B98: explicit user-confirmed static missing-input baseline only for this exact input; Option<Duration> min floor/presence/equality/ACK/knownness and all actual runtime tests mandatory; no compiler failure is runtime red. |
| P20-SDK-AUTH-018 / not-assessable | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md @ 9e3d6ef690c3a794bc9c47ddc2dce04840f98d8881649be1ab2e8db0df287fd5; L153 | credential291–309 persistentlock owned by injectedproductstore. productcredential_store1317–1360 uses2tasks/2stores+sharedfakebackend, not2processes. | B54; persistent_two_process_lock: A entersbackendsaveholdsIPCgate;Bsamepath cannotenteruntilArelease; distinctpathcontrol concurrent;timeouttyped/no mutation;reopen value. Testbackend injected,no realkeychain. |
| P20-SDK-AUTH-026 / not-assessable | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-rust-differences.md @ 411d2f6d262b1afc22e84c73213dffd0eb890ed70e210dfa497d2a5d021cd31d; L144 | mutation94–97 unknownnotclearedotherkey;506–535 retainsallreceipts,no reclamation. auth_contracts1593–1621 unknownoldkey survivesnewkeysuccess;1626retains300. | B55; receipt_retention: heldbarrier/receipt known+unknown;otherkeysuccessnevereraseunknown; conservative samekeyunknownretention is permitted unless actualreconciliationprovided; no inventedcap/reclamationfeature. |
| P20-SDK-AUTH-027 / not-assessable | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-rust-differences.md @ 411d2f6d262b1afc22e84c73213dffd0eb890ed70e210dfa497d2a5d021cd31d; L145 | CredentialStore353inline/inertbarrier differsqueue-backedsharedstore; auth_contracts1374–1621ACK/unknown/barrier; product2176/2242/2308 rotationcancel/barrier. | B56; shared_store_conformance: samewrite/read/delete/update/keep/failure/cancel-afterstart/unknown/cutoff cases forMemory andinjectedpersistentadapter; truthfulmemoryvsbackendACK, inlineweakadapternotdurable. |
| P20-SDK-CLASSIFIER-023 / not-assessable | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md @ 9e3d6ef690c3a794bc9c47ddc2dce04840f98d8881649be1ab2e8db0df287fd5; L129 | llama913–955 newlineprefixthenalone;onlysuccesscached;collisionreject. classifier2485–2511 provesonly repeatedsameinstancecachehit. | B57; llama_label_failure_recovery:multi-tokenfails0completion then correctedretokenizes;collisionlabels→sametokenreject;uniquecontrol;prefix+one/exactfallback shapes. |
| P20-SDK-CLASSIFIER-024 / not-assessable | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md @ 9e3d6ef690c3a794bc9c47ddc2dce04840f98d8881649be1ab2e8db0df287fd5; L129 | llama101 adapter-owned cache;935–951keyserverroot/model/label; existing2485 onlyhit. | B58; llama_cache_owner_matrix: eachkeydimensionchange misses;2sameid Models diffcredential/clienttokensisolated;explicitsharedsameadapter control sameownerhit;dropone leavesother. |
| P20-SDK-MODEL-003 / not-assessable | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md @ 9e3d6ef690c3a794bc9c47ddc2dce04840f98d8881649be1ab2e8db0df287fd5; L160 | model_info315–331 exact/upward/reversedownward logic. | B59; thinking_clamp_table: exact,higherfartherthanlowerstillwins,nohigher→nearestlower,allunsupportedNone,xhigh/maxexplicitnonnull; wire requested/effective vsnative reject. |
| P20-SDK-MODEL-004 / not-assessable | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md @ 9e3d6ef690c3a794bc9c47ddc2dce04840f98d8881649be1ab2e8db0df287fd5; L160 | model_wire_metadata461–493 precedencepasses/defaultemptyNone;model_info1074 conflatesabsence/empty. New samplingprobe:absent→x-opi{}FAIL;empty/nonemptycontrolsPASS. | B30; sampling_presence_and_merge: model/requestNone vsSomeempty vsnonempty;levelmissingvsexplicitempty; shallow nestedreplace;requestwins;store/reopenpresence; unsupportedwiredoesnotclaimeffective. See dedicated batch. |
| P20-SDK-MODEL-021 / not-assessable | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md @ 9e3d6ef690c3a794bc9c47ddc2dce04840f98d8881649be1ab2e8db0df287fd5; L165 | models3946–3959 zero-context leavesM0;model_info1271publicregisteredcontext0invalid. Withvalidpositivecontext sourceformulaM0returns0,notred for<=0. | B08; pure_simple_base_formula: privatehelper(context0,M0)->1,(0,7)->7,(positive,0)->0; usedcompiler verifiedpublicpositivecase; no illegalModelInfofabrication. |
| P20-SDK-MODEL-022 / not-assessable | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md @ 9e3d6ef690c3a794bc9c47ddc2dce04840f98d8881649be1ab2e8db0df287fd5; L165 | models3971–3981numeric budgettable1024/2048/8192/16384;XHighMax→High. | B60; default_budget_table: each6level+None,unsaturated/saturatedC008ceilings;nativeexplicitunchanged;ChatbudgetnotClaudeaddition. |
| P20-SDK-MODEL-026 / not-assessable | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md @ 9e3d6ef690c3a794bc9c47ddc2dce04840f98d8881649be1ab2e8db0df287fd5; L165 | models3984–3990zeroallowed;Anthropicfixtures3603native0→1024sourceor semantics. | B61; zero_remaining_wire_table:Anthropicceiling<1024→budget1024;Bedrock→0;Chatomit0;no new preflightsolelyzero;positive/absentcontrols. |
| P20-SDK-MODEL-029 / not-assessable | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md @ 9e3d6ef690c3a794bc9c47ddc2dce04840f98d8881649be1ab2e8db0df287fd5; L166 | SDKcontracts166toolowneronly; externalmanualtoolloop hostappendsresult,noSDKtool execution. | B62; tool_duration_owner: callerNone/Some0/Some17Serde/replaypreserved;LLMtimerdoesnotrewrite; no implicittoolcallbackexecution. |
| P20-SDK-MODEL-030 / not-assessable | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md @ 9e3d6ef690c3a794bc9c47ddc2dce04840f98d8881649be1ab2e8db0df287fd5; L168 | model_wire_metadata74typedwiremismatch;91–131routingempty/nestednullandinvalidtypes;C009red missingAnthropicbranches. | B63; compat_presence_matrix: wirefieldabsent/false/true/nullasallowed,wrongwirezeroauth,unknownextensionpersistsbutnoeffectiveflag,exactpayloadnewfields. |
| P20-SDK-MODEL-038 / not-assessable | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md @ 9e3d6ef690c3a794bc9c47ddc2dce04840f98d8881649be1ab2e8db0df287fd5; L175 | RegisteredSDK175wiredefaults; model_info distinctResponses/Azure/Codex configurations; testnames alone insufficient. | B64; responses_default_matrix:3familyconsumers x absent/explicitflags,developer/maxouttruewhereapplies,mid-system/grammar/additionaltools/toolsearch/explicitcachefalse; wire-specificstrict;inspectpayload/headersandunsupportedcontrols. |
| P20-SDK-MODEL-042 / not-assessable | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md @ 9e3d6ef690c3a794bc9c47ddc2dce04840f98d8881649be1ab2e8db0df287fd5; L177 | model_info341–346Bedrockstrictdefaultfalse;SDK177Mistralmidsystemfalse;rootcurrentMistralsharedflagredexists. | B65; bedrock_mistral_defaults_local:absent/explicittrue/false onbothwires under sameproviderid;strictschema/midsystemactualbody andnocrosscontamination. |
| P20-SDK-UTILITY-028 / not-assessable | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md @ 9e3d6ef690c3a794bc9c47ddc2dce04840f98d8881649be1ab2e8db0df287fd5; L190 | public_utilities392–422 preservesmalformedfinalraw+error,butnotsurrogates;serde_jsonString rejectsisolatedsurrogate. | B66; isolated_surrogate_final:escapedhigh/lowalone message/toolargs typedreject,rawescapedargumentretainedwithparseerror;validpairscalarcontrol;partialrepairnotfinalauthority. |
| P20-SDK-UTILITY-031 / not-assessable | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md @ 9e3d6ef690c3a794bc9c47ddc2dce04840f98d8881649be1ab2e8db0df287fd5; L191 | replay_contract964–998 collisiongeneratedIDtwo distinctids,matchingresults,stablewireandcanonicalunchanged;bedrock1436factualtoolimageplaceholder. | B67; nonvision_placeholder_runs:consecutiveuser/toolimages eachownplaceholderdedup;textbreakpreservesseparation;IDsbijectiveindependentofname;missingresultunknownexecutionnotguessed. |
| P20-SDK-UTILITY-032 / not-assessable | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md @ 9e3d6ef690c3a794bc9c47ddc2dce04840f98d8881649be1ab2e8db0df287fd5; L191 | constrained_sampling470/556 owners;Anthropic2316–2344 grammarpreference/fallback/missingvariant;public_utilitiesliteralpropertycontrols. | B68; grammar_require_prefer:capable/incapable x prefer/require;unsupportedrequiredkeywordtypedzeroHTTP;preferfallbacktruthfulnonstrictwithoutconstraintdeletion; literalproperty/union/unknownkeywordcontrols. |
| P20-SDK-UTILITY-034 / not-assessable | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md @ 9e3d6ef690c3a794bc9c47ddc2dce04840f98d8881649be1ab2e8db0df287fd5; L192 | provider354Openai-Intentconversation-edits,contains_image_contentvision;362–365lastmessageUser/Noneuser,otheragent. | B69; copilot_headers:emptyhistory/userlast/assistantlast/toolresultlast xChat/Responses;user/toolimagevisiontrue;thinkingonlyfalse; exactintent;protectedcallerheaderreject;accountbaseURLretained. |
| P20-SDK-UTILITY-036 / not-assessable | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md @ 9e3d6ef690c3a794bc9c47ddc2dce04840f98d8881649be1ab2e8db0df287fd5; L193 | proxy_support198 assertsuppercase precedence;parentC010sourcequalifiedPihelper/Codex/Bedrock/ADC,notglobalOpenAIrule. | B97 exact scoped/target Pi utility and caller-owned HTTP/WS/AWS source routes; B28 remains distinct default-ADC gaxios policy; B99 transport prerequisite. |
| P20-SDK-CATALOG-016 / not-assessable | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md @ 9e3d6ef690c3a794bc9c47ddc2dce04840f98d8881649be1ab2e8db0df287fd5; L206 | catalog/http_source21source-ownedfreshness;130–170statelessGETnon2xxerror. SDK206genericproductwrapper4h304404501;V56forbidsdefaultproductpolicyinSDK. | B70; caller_owned_remote_policy:injectedDynamicCatalogSource clock3h59m59snoGET,4hGET;304requirescachedbody;404501explicitpersistedtombstone;personalization404retain/Radiusemptycontrols;forceNone/false/trueforwarded. |
| P20-SDK-PACKAGE-001 / not-assessable | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md @ 9e3d6ef690c3a794bc9c47ddc2dce04840f98d8881649be1ab2e8db0df287fd5; L211 | Cargo manifests showopi-ai nointernalOpi deps andworkspaceMSRV1.97;currentrunner code isnotexecutednativeproof. SDK211/212/216/218/220 define scopedpackage claims;external_sdk294real10wire fixture and630nonchat containsDecisionsfailureonly. | B95 public fixture corpus; B17 ordinary19case; B91 protocol families; B92 native1.97.1; B93 final evidence; external_list_login_actions:scriptedtypedlogin/action/localserver,isolatedcwdunchanged,memorystoreACK,noTTY/keychain/productdep |
| P20-SDK-PACKAGE-002 / not-assessable | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md @ 9e3d6ef690c3a794bc9c47ddc2dce04840f98d8881649be1ab2e8db0df287fd5; L211 | Cargo manifests showopi-ai nointernalOpi deps andworkspaceMSRV1.97;currentrunner code isnotexecutednativeproof. SDK211/212/216/218/220 define scopedpackage claims;external_sdk294real10wire fixture and630nonchat containsDecisionsfailureonly. | B95 public fixture corpus; B17 ordinary19case; B91 protocol families; B92 native1.97.1; B93 final evidence; static_features_ua:actualmetadatafeatures/sources/checksums andlocalwireopi/realplatformUA,no procenvfallback |
| P20-SDK-PACKAGE-003 / not-assessable | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md @ 9e3d6ef690c3a794bc9c47ddc2dce04840f98d8881649be1ab2e8db0df287fd5; L211 | Cargo manifests showopi-ai nointernalOpi deps andworkspaceMSRV1.97;currentrunner code isnotexecutednativeproof. SDK211/212/216/218/220 define scopedpackage claims;external_sdk294real10wire fixture and630nonchat containsDecisionsfailureonly. | B95 public fixture corpus; B17 ordinary19case; B91 protocol families; B92 native1.97.1; B93 final evidence; bedrock_oauth_exports:outsideconsumer reusesexistingBedrockadapter+typedOAuthinteraction,noJSABI |
| P20-SDK-PACKAGE-004 / not-assessable | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md @ 9e3d6ef690c3a794bc9c47ddc2dce04840f98d8881649be1ab2e8db0df287fd5; L212 | Cargo manifests showopi-ai nointernalOpi deps andworkspaceMSRV1.97;currentrunner code isnotexecutednativeproof. SDK211/212/216/218/220 define scopedpackage claims;external_sdk294real10wire fixture and630nonchat containsDecisionsfailureonly. | B95 public fixture corpus; B17 ordinary19case; B91 protocol families; B92 native1.97.1; B93 final evidence; platform_claims:docsclaimsboundedbyactualnativeCI/package/Tokioevidence,noWASM/browser/arbitraryexecutorinference |
| P20-SDK-PACKAGE-005 / not-assessable | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md @ 9e3d6ef690c3a794bc9c47ddc2dce04840f98d8881649be1ab2e8db0df287fd5; L216 | Cargo manifests showopi-ai nointernalOpi deps andworkspaceMSRV1.97;currentrunner code isnotexecutednativeproof. SDK211/212/216/218/220 define scopedpackage claims;external_sdk294real10wire fixture and630nonchat containsDecisionsfailureonly. | B95 public fixture corpus; B17 ordinary19case; B91 protocol families; B92 native1.97.1; B93 final evidence; inventory_behavior:10chat/1image/4classifier/42factory/9OAuth eachsource+wire+normal/failure/callback/terminalcase;enumerationnotproof |
| P20-SDK-PACKAGE-006 / not-assessable | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md @ 9e3d6ef690c3a794bc9c47ddc2dce04840f98d8881649be1ab2e8db0df287fd5; L218 | Cargo manifests showopi-ai nointernalOpi deps andworkspaceMSRV1.97;currentrunner code isnotexecutednativeproof. SDK211/212/216/218/220 define scopedpackage claims;external_sdk294real10wire fixture and630nonchat containsDecisionsfailureonly. | B95 public fixture corpus; B17 ordinary19case; B91 protocol families; B92 native1.97.1; B93 final evidence; reuseC01719exactconsumer cases:soleOpi runtime,offlinebundledresources,2realinstances,allcallfamilies |
| P20-SDK-PACKAGE-007 / not-assessable | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md @ 9e3d6ef690c3a794bc9c47ddc2dce04840f98d8881649be1ab2e8db0df287fd5; L218 | Cargo manifests showopi-ai nointernalOpi deps andworkspaceMSRV1.97;currentrunner code isnotexecutednativeproof. SDK211/212/216/218/220 define scopedpackage claims;external_sdk294real10wire fixture and630nonchat containsDecisionsfailureonly. | B95 public fixture corpus; B17 ordinary19case; B91 protocol families; B92 native1.97.1; B93 final evidence; receipt_fields:actualadd/update/delete/retain,countskipmissing,sourceRustrevision,runnerartifacthashes;negativewronghash/wrongbinary/missingcase rejected |
| P20-SDK-PACKAGE-008 / not-assessable | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md @ 9e3d6ef690c3a794bc9c47ddc2dce04840f98d8881649be1ab2e8db0df287fd5; L220 | Cargo manifests showopi-ai nointernalOpi deps andworkspaceMSRV1.97;currentrunner code isnotexecutednativeproof. SDK211/212/216/218/220 define scopedpackage claims;external_sdk294real10wire fixture and630nonchat containsDecisionsfailureonly. | B95 public fixture corpus; B17 ordinary19case; B91 protocol families; B92 native1.97.1; B93 final evidence; single_upstream_tracking_append:version/API/Context/telemetry/caller/testdisposition;closedunmergednotcommitment;no secondmatrix/ledger |
| P20-SDK-PACKAGE-009 / not-assessable | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-sdk-contracts.md @ 9e3d6ef690c3a794bc9c47ddc2dce04840f98d8881649be1ab2e8db0df287fd5; L218 | Cargo manifests showopi-ai nointernalOpi deps andworkspaceMSRV1.97;currentrunner code isnotexecutednativeproof. SDK211/212/216/218/220 define scopedpackage claims;external_sdk294real10wire fixture and630nonchat containsDecisionsfailureonly. | B95 public fixture corpus; B17 ordinary19case; B91 protocol families; B92 native1.97.1; B93 final evidence; actual_crate_identity:packageactual.crate,extractoutsideworkspace,offlineexactmanifestdep;negative rootpatch/TEMPsubstitution/Agentproductdep rejected |
| P20-SDK-PACKAGE-010 / not-assessable | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-rust-differences.md @ 411d2f6d262b1afc22e84c73213dffd0eb890ed70e210dfa497d2a5d021cd31d; L211 | Cargo manifests showopi-ai nointernalOpi deps andworkspaceMSRV1.97;currentrunner code isnotexecutednativeproof. SDK211/212/216/218/220 define scopedpackage claims;external_sdk294real10wire fixture and630nonchat containsDecisionsfailureonly. | B95 public fixture corpus; B17 ordinary19case; B91 protocol families; B92 native1.97.1; B93 final evidence; native_assembly_identity:actualdeps/features/sources/checksums/platformobjects and consumed.crate,notmonorepocompile |
| P20-SDK-PACKAGE-011 / not-assessable | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-rust-differences.md @ 411d2f6d262b1afc22e84c73213dffd0eb890ed70e210dfa497d2a5d021cd31d; L274 | Cargo manifests showopi-ai nointernalOpi deps andworkspaceMSRV1.97;currentrunner code isnotexecutednativeproof. SDK211/212/216/218/220 define scopedpackage claims;external_sdk294real10wire fixture and630nonchat containsDecisionsfailureonly. | B95 public fixture corpus; B17 ordinary19case; B91 protocol families; B92 native1.97.1; B93 final evidence; native_matrix:Linux/macOS/Windows+MSRV1.97 execute packageconsumer boundsource/toolchain/artifacts;futuregreennotplanstageprecondition |
| P20-MECH-AUTH-001 / not-assessable | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-transport-auth.md @ 42e1bc90c02e958ada4d2a01db999cd906ae33ee971496c50c07ed38067cdcfb; L13,L101 | builtin_aws57–82/161–188 andaws/local ownContinueSelectedTerminal;bedrock111–169zeroIOexplicitidentity;4453–4510selecteddenialterminal. | B71; acquisition_outcomes:sourceabsenceContinue,selectedsuccessidentityexpiry,selectedservicefailureTerminal;separateread/exchange/generationcounts andsecretredaction |
| P20-MECH-AUTH-002 / not-assessable | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-transport-auth.md @ 42e1bc90c02e958ada4d2a01db999cd906ae33ee971496c50c07ed38067cdcfb; L101 | bedrock4453–4510STS403typedcode/requestid,redactedbody,no processfallback;7805–7933generation3sends/oneauth/frozenpayload.8384countertestisdisconnectednotreal2layerproof. | B72; source_generation_budgets:STS503→2002exchanges thenBedrock503→2002generations independentlimits;source403→generation0/nofallback;redirectlegcounts+reqwestextraretryoff |
| P20-MECH-AUTH-003 / not-assessable | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-transport-auth.md @ 42e1bc90c02e958ada4d2a01db999cd906ae33ee971496c50c07ed38067cdcfb; L105 | oauth_store_integration152–194single rotation/sharedstorewaiterdrop;builtin_auth_ownershiprebindingpreservescustomowners. | B73; frozen_nine_flow_dependencies:providerrevision/presenter/clock/store/scopedenvfrozenbeforecall;replaceaftercreationchangesnewcallonly;refreshsameidentity;actualexpiryC020 |
| P20-MECH-AUTH-007 / not-assessable | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-transport-auth.md @ 42e1bc90c02e958ada4d2a01db999cd906ae33ee971496c50c07ed38067cdcfb; L106 | auth_contracts440constructionzeroIO;bedrock5656–5696exactprocessenv/cwd;selectedADC2035malformedfileterminal. | B74; aws_errorclass_table:completepartialenv/absentunreadablemalformedINI/cycle/processabsenceparse/service denialsourceexactContinueTerminal;regiononlyendpointnotaccount;HOMEcanaryzero constructionread |
| P20-MECH-AUTH-009 / not-assessable | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-transport-auth.md @ 42e1bc90c02e958ada4d2a01db999cd906ae33ee971496c50c07ed38067cdcfb; L107 | bedrock6521–6608SSOfailedfilesave retainsR2throughexpiry yetWrittenreceiptisownercacheACK;6612–6684loginanalog. NotdurablefileACKproof. | B75; sso_login_ACK_capability:memorycommit vsselectedpersistentfile success/failure/unknown;retainonlyrefresh,nextsameidentityusesR2;clockRNGinjected,no secrettrace |
| P20-MECH-AUTH-010 / not-assessable | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-transport-auth.md @ 42e1bc90c02e958ada4d2a01db999cd906ae33ee971496c50c07ed38067cdcfb; L108 | bedrock5656–5696snapshotonceexactenv/cwd;auth_process_lifecycle436/542/670cancelreaptree;processowner noinsertedshell. | B76; aws_process_lifecycle:quotedargv/exactenv/cwd,selectedinvalidtyped,cancelbefore/during,rootexitdescendantthenreap;readexecuteexchangecountersdistinct |
| P20-MECH-AUTH-011 / not-assessable | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-transport-auth.md @ 42e1bc90c02e958ada4d2a01db999cd906ae33ee971496c50c07ed38067cdcfb; L108 | bedrock4515–4586oneWebIdentityform/noSigV4/exactprovenance/accountid/noprocess;4591–4637badfileTerminal0sends. | B77; web_identity_service_denial:validfileSTS403→TerminalnotUnsupported,nootherprofile/process/metadata,generation0;missingbadtokenfilecontrols |
| P20-MECH-AUTH-014 / not-assessable | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-transport-auth.md @ 42e1bc90c02e958ada4d2a01db999cd906ae33ee971496c50c07ed38067cdcfb; L110 | bedrock7805–7933realretrydelays;8176–8228adaptivewaitformula;8319–8346legacy100/202650ms+cost14;retry683–733CUBICimplementation. | B78; smithy_state_trace:seededclockRNGquotaacquire/refund/exhaust/hintbounds;legacy2026bothconstants;adaptivefirstsendwait/successincrease/throttledecrease/recovery fixedsourcevectors+actualsendcounts |
| P20-MECH-AUTH-015 / not-assessable | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-transport-auth.md @ 42e1bc90c02e958ada4d2a01db999cd906ae33ee971496c50c07ed38067cdcfb; L110 | bedrock8233–8277scoped2attemptsoverprofile9 with2actualrequests;8384quota/sourcecountertestnotactualacquisition. | B79; smithy_owner_scope:onecallquotaexhaustiondoesnotaffectsecondcall/instance;scopedmode/max/profileprecedence;source2exchangesindependentgeneration2attemptallowance |
| P20-MECH-AUTH-016 / not-assessable | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-transport-auth.md @ 42e1bc90c02e958ada4d2a01db999cd906ae33ee971496c50c07ed38067cdcfb; L111 | auth_contracts2225–2278canonicalJWTPOSTalgonly/claims/expiry;2482–2504scripted3requests;2562–2584noresponse3attempts. | B80; adc_gaxios_actualwire:serviceaccount/authorizeduser statuses408429500599retry vs400401600no;network2extra/exchange3extra/exactdelays/terminalredirect/nohiddenreqwestextra;quotaprojectpreserved |
| P20-MECH-AUTH-017 / not-assessable | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-transport-auth.md @ 42e1bc90c02e958ada4d2a01db999cd906ae33ee971496c50c07ed38067cdcfb; L112 | auth_contracts4685–4755IAMurl/sourceBearer/delegates/scope/lifetime/provenance/expiry;4810–4835recursive3legs;2308Basicexternaluser. | B81; adc_recursive_sources:authorized/service/externaluser depthbeyondoldcap;targetdelegateslifetimeexact;BasicreplacementR2nextrefresh;sourceidentitynotretaggedtarget |
| P20-MECH-AUTH-018 / not-assessable | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-transport-auth.md @ 42e1bc90c02e958ada4d2a01db999cd906ae33ee971496c50c07ed38067cdcfb; L112 | adc_owner_regressions130/177replacementfences,226sharedbarrier;auth4685source/targetdistinct. | B82; adc_rotation_fences:waiterdrop/replace/sameidentityinvalidate preserveonlyR2,distinctsourcenorebind;defaultADCfilebytesunchanged;failedunknownACKnotpersisted;explicitstorecontrol |
| P20-MECH-AUTH-020 / not-assessable | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-transport-auth.md @ 42e1bc90c02e958ada4d2a01db999cd906ae33ee971496c50c07ed38067cdcfb; L113 | auth_contracts3412–3471signedsubjecturl/region/credential/targetresourceunsigned;needzerosend/discoveryassertionsbeyondshape. | B83; adc_aws1_isolation:envpairregion→GoogleSTSonly;poisonBedrockprofileprocessSSO unread;IMDSexactlegs→encodedsubject,no actualAWSSTS;partialpair/missingregiontyped |
| P20-MECH-AUTH-021 / not-assessable | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-transport-auth.md @ 42e1bc90c02e958ada4d2a01db999cd906ae33ee971496c50c07ed38067cdcfb; L114 | auth_contracts3720–3777realmtlsSTS/plaintransport0/exactleaf;3781–3808both/neitherconfigtyped. | B84; adc_mtls_isolation:matchedleafkeychainpositive,wrongkey/leaf/chainnegative;trustchainnotserverroot;subsequentgenerationserverseesno clientcert;plainADCcontrolunaffected |
| P20-MECH-AUTH-022 / not-assessable | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-transport-auth.md @ 42e1bc90c02e958ada4d2a01db999cd906ae33ee971496c50c07ed38067cdcfb; L115 | auth3998quotedargv;4017gatebeforecache/spawn;4049freshcachezero;4147malformedterminal;4171version/type/expiry. | B85; executable_cache_table:gateoffvalidcache0spawn;fresh0,stale1,malformed0;version/token_type/expirytyped;argvexact |
| P20-MECH-AUTH-023 / not-assessable | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-transport-auth.md @ 42e1bc90c02e958ada4d2a01db999cd906ae33ee971496c50c07ed38067cdcfb; L115 | executable242–257combinedpipe/exactenv;270–307unboundedread;309–314killreapjoin;process_lifecycle311largstderr/436542descendants. | B86; executable_output_deadline:combinedstdoutstderrvalidJSON,>64KiB,default30s/allowed5and120/outsideinvalid;output_fileunchanged;cancel/timeouttree nativecases |
| P20-MECH-AUTH-024 / not-assessable | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-transport-auth.md @ 42e1bc90c02e958ada4d2a01db999cd906ae33ee971496c50c07ed38067cdcfb; L116,L126 | auth2016nofilenoconfig/nosends;2035selectedbadfileterminal;adc_owner402/428/459presenceandprobefailures. | B87; factory_no_metadata_discovery:Modelsnew/register/available/checkmissingfiles zero metadataHTTP/BIOS/health/gcloud/process evenwithcanaries;explicitdirectoneleg;projectlocationmissingpreHTTP |
| P20-MECH-AUTH-025 / not-assessable | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-transport-auth.md @ 42e1bc90c02e958ada4d2a01db999cd906ae33ee971496c50c07ed38067cdcfb; L122,L126 | metadata50–92explicitresidencytrueNone/other3000;auth2612–2651assert3000/2656–2673None. | B88; residency_scope:Nonefalse true→3000/3000/None;hostobservationsonlyselectedpathfrozenonce;trueallows4sresponse;otherADCtimeoutnotchanged |
| P20-MECH-AUTH-026 / not-assessable | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-transport-auth.md @ 42e1bc90c02e958ada4d2a01db999cd906ae33ee971496c50c07ed38067cdcfb; L124,L126 | auth2612–2710Flavor/scopes/expiry/wrongFlavor/timeoutattempt1+sends1;metadata40–45distinct3/3budget. | B89; metadata_send_matrix:absentwrongflavor/invalidJSON/checkedexpiry/status/noresponse/cancelactualcounts;nonresident4sone-sendtimeout;Modelsavailablezero-discovery distinctfromexplicitdirecttoken;no globaldeadline |
| P20-V-58 / not-assessable | docs/research/pi-ai-opi-ai-alignment/acceptance-matrix.md @ c831586339910f9496a64e6b3c9390a0b7a2a366d37230222874a5f399099a5d; V-58, line 221 | external_sdk.rs current ten-case source; B17 contains exact nineteen public-only named cases, including four entry points/custom options/frozen calls/cancellation/cleanup. | specified-final-byte-green; B17 actual extracted .crate ordinary consumer, exact19/19 executed0ignored; platform/family branches run separately. |
| P20-V-59 / not-assessable | docs/research/pi-ai-opi-ai-alignment/acceptance-matrix.md @ c831586339910f9496a64e6b3c9390a0b7a2a366d37230222874a5f399099a5d; V-59, line 222 | Fixed current Pi/Opi/index identities, original audit states and test impact retained; a present test file is not executed proof. | specified-documentary-collection; Final reconciliation writes existing upstream tracking/evidence locations with per-test source input, target/filter, actual exit/count/skip/missing and immutable artifact/log hashes. Do not rewrite historical70/probe baseline. |
| P20-AUTH-003 / not-assessable | docs/opi-spec.md @ b698133fbc2c0347726447159259c18effa8a1091796aae559c4bb04cb5612f3; AUTH-003, line 36 | Explicit opi-implement P0/P2 READY, exact P4 confirm-all then write-only install;16live registered hashes match. Active tasks0;68historical summaries and phase_exit/frozen bytes unchanged. Live ledgerSHA 8ed820e0a10830b70f453eddc01e1700ffdb68458534febefa22e220bc53780b. No runtime/audit closure inferred from archive. | Completed owner source binding; preserve exact committed live ledger and original owner receipt during apply. A later non-plan commit requires new remediation export/binding. Later independent F1a remains its owner. |
| P20-PRIN-002 / not-assessable | docs/opi-spec.md @ b698133fbc2c0347726447159259c18effa8a1091796aae559c4bb04cb5612f3; PRIN-002, line 121 | Cargo.toml seven-member graph; opi-ai has no Opi internal dependency, providers are behind existing Models interfaces; no Agent Core capability is moved by plan. | current-placement-plus-final-consumer; Doc graph checker plus B17 public custom chat/image/classifier/deferred and multiple real wire adapters. New seams failing the governing clause stop, never get waived by a fixture. |
| P20-PRIN-005 / not-assessable | docs/opi-spec.md @ b698133fbc2c0347726447159259c18effa8a1091796aae559c4bb04cb5612f3; PRIN-005, line 124 | Every baseline probe retains source/lock/log/exit identity; historical CI is explicitly not current final package proof. | specified-proof-identity; Final-byte generated proof retains source/archive/executable/graph/toolchain/native/log identities; test/harness unit PASS is not package acceptance. |
| P20-INV-002 / not-assessable | docs/opi-spec.md @ b698133fbc2c0347726447159259c18effa8a1091796aae559c4bb04cb5612f3; INV-002, line 406 | external_sdk.rs current ten-case source; B17 contains exact nineteen public-only named cases, including four entry points/custom options/frozen calls/cancellation/cleanup. | specified-final-byte-green; B17 actual extracted .crate ordinary consumer, exact19/19 executed0ignored; platform/family branches run separately. |
| P20-GATE-001 / not-assessable | docs/opi-spec.md @ b698133fbc2c0347726447159259c18effa8a1091796aae559c4bb04cb5612f3; GATE-001, line 551 | Cargo.toml seven-member graph; opi-ai has no Opi internal dependency, providers are behind existing Models interfaces; no Agent Core capability is moved by plan. | current-placement-plus-final-consumer; Doc graph checker plus B17 public custom chat/image/classifier/deferred and multiple real wire adapters. New seams failing the governing clause stop, never get waived by a fixture. |
| P20-MECH-DELIVERY-001 / not-assessable | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-runtime.md @ 1328a81e561e1e1bfb2c35df788ce6d08fec2f71e14eb8abba9d57e1f03f1c7b; L29 | provider_collection.rs5525 checks zero source reads at construction/unpolled calls; external_sdk.rs1425 checks empty Models/static reads; catalog_artifact.rs362 preserves custom complete records independently of failed catalog install. | current-runtime-controls-plus-final-green; B17 connected recording controls cover all four unpolled calls; source availability failures remain typed, never download/hydrate or discover ambient identity/trust at construction. |
| P20-MECH-DELIVERY-002 / not-assessable | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-runtime.md @ 1328a81e561e1e1bfb2c35df788ce6d08fec2f71e14eb8abba9d57e1f03f1c7b; L29,L33 | catalog_artifact.rs121-309 mutates actual bundle and asserts old Arc/revision retained; catalog_operations.rs472-650/767 covers failure/cancel/drop/malformed/persist failure and atomic visible records; catalog_operations.rs700/1083 preserves captured old calls. | current-assertion-level-proof; Retain/run catalog_artifact,catalog_operations,catalog_lifecycle; package/B17 repeats public capture and resource identity. A failed stage cannot replace the last known revision. |
| P20-MECH-DELIVERY-003 / not-assessable | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-runtime.md @ 1328a81e561e1e1bfb2c35df788ce6d08fec2f71e14eb8abba9d57e1f03f1c7b; L31,L33 | catalog-check.json PASS: archived pack.py read-only check_emitted against actual catalog/manifest and four explicitly hashed pinned Pi source files; 42providers/1608chat/38classifier/59image, exact artifact SHA ffa3e98626230f378f80200debb736c4a2238b5f2ba246a92cfc7d8e19604678. Source generator hashes exactly compare, not just hex shape. | current-read-only-exact-artifact-proof-plus-package-green; Repeat exact bundle validation, include manifest/catalog/license in actual .crate, compare extracted byte hashes and exercise Catalog::builtin/from_bytes plus negative staging. No generator, install or live fetch. |
| P20-MECH-DELIVERY-004 / not-assessable | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-runtime.md @ 1328a81e561e1e1bfb2c35df788ce6d08fec2f71e14eb8abba9d57e1f03f1c7b; L33 | catalog_artifact.rs121-309 mutates actual bundle and asserts old Arc/revision retained; catalog_operations.rs472-650/767 covers failure/cancel/drop/malformed/persist failure and atomic visible records; catalog_operations.rs700/1083 preserves captured old calls. | current-assertion-level-proof; Retain/run catalog_artifact,catalog_operations,catalog_lifecycle; package/B17 repeats public capture and resource identity. A failed stage cannot replace the last known revision. |
| P20-MECH-DELIVERY-005 / not-assessable | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-runtime.md @ 1328a81e561e1e1bfb2c35df788ce6d08fec2f71e14eb8abba9d57e1f03f1c7b; L39 | Root-owned workspace.dependencies/manifests and Cargo.lock; platform_delivery.rs234/327 check seven members, internal dependency directions, Cargo-generated lock and unique member identities. | current-static-contract-plus-final-check; Retain/run platform_delivery plus documentation graph checker; Cargo alone regenerates lock if an admitted dependency change requires it. |
| P20-MECH-DELIVERY-006 / not-assessable | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-runtime.md @ 1328a81e561e1e1bfb2c35df788ce6d08fec2f71e14eb8abba9d57e1f03f1c7b; L39 | platform_delivery.rs286/368/386/413/437 inventories root versions/checksums/build.rs/unsafe/proc-macro; real family runners generate consumer metadata/lock/checksum/feature/native reports. Static root graph alone does not freeze consumers. | specified-final-artifact-graph-proof; For each actual package profile record resolved metadata,target,cfg,features,registry checksum,license/build.rs/native/proc-macro/MSRV and reverse unions. Compare supported lock closure exactly; source/tool/input changes invalidate affected receipt. |
| P20-MECH-DELIVERY-007 / not-assessable | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-runtime.md @ 1328a81e561e1e1bfb2c35df788ce6d08fec2f71e14eb8abba9d57e1f03f1c7b; L39 | Existing HttpClient retry-owner separation and schema remote-disabled fixtures; root feature defaults alone are not used as proof. | specified-negative-runtime-proof; Retain selected HTTP no-internal-retry and Schema remote-ref zero-I/O fixtures; package ordinary/union consumers repeat them using controlled loopback/counters and finite no-remote source cases. |
| P20-MECH-DELIVERY-008 / not-assessable | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-runtime.md @ 1328a81e561e1e1bfb2c35df788ce6d08fec2f71e14eb8abba9d57e1f03f1c7b; L43 | Current CI has three native OS workspace jobs and Linux MSRV check, but no full three-host exact-MSRV package matrix. Earlier native evidence is acknowledged with changed-input limitation. | specified-native-final-byte-green; Add exact1.97.1 three-native-host package job; run selected actual package schema/http/crypto/ws/default/C-union/readme/external consumers and all relevant behavior tests. Native logs/artifacts are mandatory; cross-check or config alone cannot pass. |
| P20-MECH-DELIVERY-009 / not-assessable | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-runtime.md @ 1328a81e561e1e1bfb2c35df788ce6d08fec2f71e14eb8abba9d57e1f03f1c7b; L43,L45 | Existing native_proof_guard rejects system/override provenance and binds current Cargo unit outdir,fingerprints,source bytes,compiler,link flags,static archive. Baseline non-FIPS, native roots/TLS/FIPS/other unions not silently supported. | specified-native-provenance-green; Run crypto ordinary/union and ws ordinary/default/C-zlib profiles with AWS_LC_SYS_USE_SYSTEM=0,LIBZ_SYS_STATIC=1,CC_ENABLE_DEBUG_OUTPUT=1 and controlled env; retain exact CC/CMake-if-used/tool/source/build/output/link identities. Unsupported unions remain excluded by their existing contract. |
| P20-MECH-DESIGN-001 / not-assessable | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-design-readiness.md @ b74c0b09afd31bc06cf8d5d1df834a6af9d53f2d08c3653aa492ed36396c4eba; L19,L35,L43,L47 | Every baseline probe retains source/lock/log/exit identity; historical CI is explicitly not current final package proof. | specified-proof-identity; Final-byte generated proof retains source/archive/executable/graph/toolchain/native/log identities; test/harness unit PASS is not package acceptance. |
| P20-MECH-DESIGN-002 / not-assessable | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-design-readiness.md @ b74c0b09afd31bc06cf8d5d1df834a6af9d53f2d08c3653aa492ed36396c4eba; L39,L41 | Plan does not add server roles/global allocator/caps/fallback or change valid input acceptance to hide gaps; indexed product defects retain their own batches and original typed boundaries. | retained-stop-boundary; Any new source narrowing, parser gap, secret leak, provenance change or authority conflict stops that affected batch for a newly bound plan; retain existing credential conflict-before-ACK/rollback tests and actual wire negative controls. This rule cannot turn a failing unknown case into automatic repair authority. |
| P20-MECH-DESIGN-003 / not-assessable | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-design-readiness.md @ b74c0b09afd31bc06cf8d5d1df834a6af9d53f2d08c3653aa492ed36396c4eba; L43,L45 | Seven manifests; private crates/opi-ai/src/ws contains codec and packaged tests. consumer-ws/src/main.rs174-438 drives actual Models->Codex->Upgrade->private codec; no separate WS crate or public codec seam is inferred from stale runner wording. | current-static-ownership-plus-final-package-green; Extract actual .crate, reject source path/include/export escapes, execute named private/public/real-chain corpora separately and preserve feature graphs. |
| P20-MECH-DESIGN-004 / not-assessable | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-design-readiness.md @ b74c0b09afd31bc06cf8d5d1df834a6af9d53f2d08c3653aa492ed36396c4eba; L47 | provider_collection.rs5525 checks zero source reads at construction/unpolled calls; external_sdk.rs1425 checks empty Models/static reads; catalog_artifact.rs362 preserves custom complete records independently of failed catalog install. | current-runtime-controls-plus-final-green; B17 connected recording controls cover all four unpolled calls; source availability failures remain typed, never download/hydrate or discover ambient identity/trust at construction. |
| P20-DEL-01 / not-assessable | docs/research/pi-ai-opi-ai-alignment/spec-draft.md @ 8c6cd66f7771e026210f06e49d037da54029c74258a1d45925cdbddd710f5f21; DEL-01, line 40; lines 27,57,67,73,79,583 | Current source/capability IDs retained; delivery requires57active SDK AIR,nine host-only,one retired identity from fixed source. No current admission is inferred from prior delivery text. | Owner source review/graph confirmed; B93 final source/capability/test identity reconciliation after actual dependencies. No runtime delivery inferred from source admission. |
| P20-DEL-03 / not-assessable | docs/research/pi-ai-opi-ai-alignment/spec-draft.md @ 8c6cd66f7771e026210f06e49d037da54029c74258a1d45925cdbddd710f5f21; DEL-03, line 42; lines 129,214,585 | transcript_contract.rs63/181/229/465/496/673 asserts validation atomicity, declaration/order, canonical immutable projection and serde chronology; external_sdk.rs1189 real two-request caller-owned tool loop preserves association after serde; anthropic_thinking_roundtrip.rs80/171/242/303 checks signature/opaque separation. | current-substantial-proof-plus-public-package-green; Retain full transcript/tool/error/projection fixtures; B17 actual package manual loop and wire matrix, plus protocol-specific replay controls supplied by C018 Group A mapping. |
| P20-DEL-14 / not-met | docs/research/pi-ai-opi-ai-alignment/spec-draft.md @ 8c6cd66f7771e026210f06e49d037da54029c74258a1d45925cdbddd710f5f21; DEL-14, line 53; lines 29,588-596 | Actual package runners exist and have discriminating graph/admission/native checks; assembled SDK path is separately added by B17. No documentary proof substitutes for the conjunctive runtime matrix. | B17/B91/B95/B92/B93 full future actual package/native/runtime conjunction; source owner reconciliation completed; independent F1a/Phase exit stays later owner. |
| P20-DEL-SCOPE-05 / not-assessable | docs/research/pi-ai-opi-ai-alignment/spec-draft.md @ 8c6cd66f7771e026210f06e49d037da54029c74258a1d45925cdbddd710f5f21; lines 32,36,57,73 | Explicit opi-implement P0/P2 READY, exact P4 confirm-all then write-only install;16live registered hashes match. Active tasks0;68historical summaries and phase_exit/frozen bytes unchanged. Live ledgerSHA 8ed820e0a10830b70f453eddc01e1700ffdb68458534febefa22e220bc53780b. No runtime/audit closure inferred from archive. | Completed owner source binding; preserve exact committed live ledger and original owner receipt during apply. A later non-plan commit requires new remediation export/binding. Later independent F1a remains its owner. |
| P20-DEL-SCOPE-06 / not-assessable | docs/research/pi-ai-opi-ai-alignment/spec-draft.md @ 8c6cd66f7771e026210f06e49d037da54029c74258a1d45925cdbddd710f5f21; lines 30-31,353 | Plan does not add server roles/global allocator/caps/fallback or change valid input acceptance to hide gaps; indexed product defects retain their own batches and original typed boundaries. | retained-stop-boundary; Any new source narrowing, parser gap, secret leak, provenance change or authority conflict stops that affected batch for a newly bound plan; retain existing credential conflict-before-ACK/rollback tests and actual wire negative controls. This rule cannot turn a failing unknown case into automatic repair authority. |
| P20-DEL-EXIT-03 / not-assessable | docs/research/pi-ai-opi-ai-alignment/spec-draft.md @ 8c6cd66f7771e026210f06e49d037da54029c74258a1d45925cdbddd710f5f21; lines 29,53,560,590 | This plan makes no Phase exit,ledger or frozen-snapshot completion claim. F1a is later independent evaluation after final runtime evidence, not an extra pre-remediation audit. | later-independent-owner-retained; Result handoff preserves exact current inputs/final checks for the independent owner; opi-remediate never writes .opi-impl-state.json. |
| P20-DEL-EXIT-05 / not-assessable | docs/research/pi-ai-opi-ai-alignment/spec-draft.md @ 8c6cd66f7771e026210f06e49d037da54029c74258a1d45925cdbddd710f5f21; lines 246,539,594-596; AIR-067 | Fixed current Pi/Opi/index identities, original audit states and test impact retained; a present test file is not executed proof. | specified-documentary-collection; Final reconciliation writes existing upstream tracking/evidence locations with per-test source input, target/filter, actual exit/count/skip/missing and immutable artifact/log hashes. Do not rewrite historical70/probe baseline. |
| P20-DEL-EXIT-06 / not-assessable | docs/research/pi-ai-opi-ai-alignment/spec-draft.md @ 8c6cd66f7771e026210f06e49d037da54029c74258a1d45925cdbddd710f5f21; lines 594-596 | Current canonical transcripts and protocol wire fixtures retain actual order/raw bytes/typed failures. Broad passing counts cannot prove all protocol differences. | specified-semantic-fixture-comparison; For each named source fixture record semantic input->Rust target/filter->actual result; compare order,event-time partials,terminal/cancel,error,usage knownness/callback counts/tool IDs/signatures/projection. Normalize only clock/request IDs or explicitly bijective identities; record accepted RDF differences. |
| P20-DEL-REPLACE-02 / not-assessable | docs/research/pi-ai-opi-ai-alignment/spec-draft.md @ 8c6cd66f7771e026210f06e49d037da54029c74258a1d45925cdbddd710f5f21; lines 528-529,535 | Current opi-ai/src exposes no Config,ProviderConfig,Model,ProviderKind or ProviderCollection facade definitions/reexports; provider_collection.rs is a legacy test filename and not a public facade. ApiKind/record/store remain. | current-export-inventory-plus-preauth-control; Doc/export checker plus product retired-key invalid-input controls must assert rejection before auth/send. Do not delete a complete fixture file merely for its old name. |
| P20-DEL-REPLACE-05 / not-assessable | docs/research/pi-ai-opi-ai-alignment/acceptance-matrix.md @ c831586339910f9496a64e6b3c9390a0b7a2a366d37230222874a5f399099a5d; lines 431-448 | Existing runtime fixtures cover leak canaries,usage known/unknown/zero,wire/error/cancel,join/framing,CRC/SigV4 and process cleanup. No original binary/filter is deleted by this plan. | retained-regression-union; Workspace all-targets plus every modified named target; explicit add/update/delete/retain inventory in final handoff. Delete only superseded assertion with stronger named replacement, never corpus by filename. |
| P20-DEL-PKG-01 / not-assessable | docs/research/pi-ai-opi-ai-alignment/spec-draft.md @ 8c6cd66f7771e026210f06e49d037da54029c74258a1d45925cdbddd710f5f21; AIR-050 line 215; lines 274,478,488; matrix lines 473,532 | external_sdk.rs current ten-case source; B17 contains exact nineteen public-only named cases, including four entry points/custom options/frozen calls/cancellation/cleanup. | specified-final-byte-green; B17 actual extracted .crate ordinary consumer, exact19/19 executed0ignored; platform/family branches run separately. |
| P20-DEL-PKG-03 / not-assessable | docs/research/pi-ai-opi-ai-alignment/spec-draft.md @ 8c6cd66f7771e026210f06e49d037da54029c74258a1d45925cdbddd710f5f21; lines 474-478,510 | run_early_package_proof.py and package_slice.py use actual extracted opi-ai ordinary/caller-union graphs; registry/source guards and ICU4X_DATA_DIR rejection already implemented. Current scope is exact effective graph, not eternal Unicode data equivalence. | specified-external-schema-http-proof; Execute schema defaults-off/caller-default-regress73checks each and HTTP ordinary/union6checks each; retain actual dependency/checksum/effective-feature/data/license/build provenance and regression outcomes. |
| P20-DEL-PKG-04 / not-assessable | docs/research/pi-ai-opi-ai-alignment/spec-draft.md @ 8c6cd66f7771e026210f06e49d037da54029c74258a1d45925cdbddd710f5f21; lines 482-488 | package_slice --family crypto real external consumer follows Models->Chat->HttpClient->local TLS; ordinary/caller-union profiles and native_proof_guard exist. | specified-public-native-crypto-proof; Both actual profiles execute6named checks each covering trust/mTLS/ALPN/negative/isolation/cancel and record registry/native source-build identity. Native host/toolchain matrix is separate. |
| P20-DEL-PKG-05 / not-assessable | docs/research/pi-ai-opi-ai-alignment/spec-draft.md @ 8c6cd66f7771e026210f06e49d037da54029c74258a1d45925cdbddd710f5f21; lines 496,500-502; backend lines 90-100,212-216 | consumer-ws actual real Codex chain exists; package_slice currently accepts minimum private95/public15 counts, which alone cannot prove complete named corpus. | specified-public-private-ws-proof; Tighten runner to exact discovered required named tests, fail ignored/filtered/missing/duplicate/nonzero. Execute private corpus,public Upgrade and ten consumer checks separately for ordinary/default/C-zlib, after WS cancellation repair. |
| P20-DEL-PKG-06 / not-assessable | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-rust-backends.md @ acb7f5f798b3384c63d36ad9af844b580755e2ea792ce3e2d32d9d3af2f15e81; lines 92,212,214; spec-draft.md lines 492-500 | Seven manifests; private crates/opi-ai/src/ws contains codec and packaged tests. consumer-ws/src/main.rs174-438 drives actual Models->Codex->Upgrade->private codec; no separate WS crate or public codec seam is inferred from stale runner wording. | current-static-ownership-plus-final-package-green; Extract actual .crate, reject source path/include/export escapes, execute named private/public/real-chain corpora separately and preserve feature graphs. |
| P20-DEL-PKG-07 / not-assessable | docs/research/pi-ai-opi-ai-alignment/spec-draft.md @ 8c6cd66f7771e026210f06e49d037da54029c74258a1d45925cdbddd710f5f21; lines 27,49,450; backend lines 186,206; matrix lines 507,550 | catalog-check.json PASS: archived pack.py read-only check_emitted against actual catalog/manifest and four explicitly hashed pinned Pi source files; 42providers/1608chat/38classifier/59image, exact artifact SHA ffa3e98626230f378f80200debb736c4a2238b5f2ba246a92cfc7d8e19604678. Source generator hashes exactly compare, not just hex shape. | current-read-only-exact-artifact-proof-plus-package-green; Repeat exact bundle validation, include manifest/catalog/license in actual .crate, compare extracted byte hashes and exercise Catalog::builtin/from_bytes plus negative staging. No generator, install or live fetch. |
| P20-DEL-PROV-01 / not-assessable | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-rust-backends.md @ acb7f5f798b3384c63d36ad9af844b580755e2ea792ce3e2d32d9d3af2f15e81; lines 186-188,212-216; spec-draft.md lines 450,476,478,486 | platform_delivery.rs286/368/386/413/437 inventories root versions/checksums/build.rs/unsafe/proc-macro; real family runners generate consumer metadata/lock/checksum/feature/native reports. Static root graph alone does not freeze consumers. | specified-final-artifact-graph-proof; For each actual package profile record resolved metadata,target,cfg,features,registry checksum,license/build.rs/native/proc-macro/MSRV and reverse unions. Compare supported lock closure exactly; source/tool/input changes invalidate affected receipt. |
| P20-DEL-PROV-02 / not-assessable | docs/research/pi-ai-opi-ai-alignment/spec-draft.md @ 8c6cd66f7771e026210f06e49d037da54029c74258a1d45925cdbddd710f5f21; lines 482,486,488; backend lines 188,214-216 | Existing native_proof_guard rejects system/override provenance and binds current Cargo unit outdir,fingerprints,source bytes,compiler,link flags,static archive. Baseline non-FIPS, native roots/TLS/FIPS/other unions not silently supported. | specified-native-provenance-green; Run crypto ordinary/union and ws ordinary/default/C-zlib profiles with AWS_LC_SYS_USE_SYSTEM=0,LIBZ_SYS_STATIC=1,CC_ENABLE_DEBUG_OUTPUT=1 and controlled env; retain exact CC/CMake-if-used/tool/source/build/output/link identities. Unsupported unions remain excluded by their existing contract. |
| P20-DEL-PROV-03 / not-assessable | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-rust-backends.md @ acb7f5f798b3384c63d36ad9af844b580755e2ea792ce3e2d32d9d3af2f15e81; lines 188,214-216; spec-draft.md lines 496 | Existing native_proof_guard rejects system/override provenance and binds current Cargo unit outdir,fingerprints,source bytes,compiler,link flags,static archive. Baseline non-FIPS, native roots/TLS/FIPS/other unions not silently supported. | specified-native-provenance-green; Run crypto ordinary/union and ws ordinary/default/C-zlib profiles with AWS_LC_SYS_USE_SYSTEM=0,LIBZ_SYS_STATIC=1,CC_ENABLE_DEBUG_OUTPUT=1 and controlled env; retain exact CC/CMake-if-used/tool/source/build/output/link identities. Unsupported unions remain excluded by their existing contract. |
| P20-DEL-PROV-04 / not-assessable | docs/research/pi-ai-opi-ai-alignment/spec-draft.md @ 8c6cd66f7771e026210f06e49d037da54029c74258a1d45925cdbddd710f5f21; lines 496,498; backend lines 66,98,216 | platform_delivery unsafe inventory and private ws dependency/source code remain explicit; WS023 token connection repair touches private PMD execution. | specified-affected-unsafe-review-and-replay; Review actual init/reset/query/inflate/end/Send pointer/allocator/end and locked SyncPoint/ResetKeep/internal API contracts for each changed WS source/feature, then run exact private/public/consumer corpus for each supported union. No guessed extern/system zlib fallback. |
| P20-DEL-PROV-05 / not-assessable | docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-rust-backends.md @ acb7f5f798b3384c63d36ad9af844b580755e2ea792ce3e2d32d9d3af2f15e81; lines 104-116,126,146,152-154 | AWS/ADC native official operation/XML/Query/source contracts stay below selected auth interfaces. ADC proxy repair preserves injected transport and scoped credentials; source-specific negative controls are Group B work. | retained-selected-source-protocol-plus-finite-controls; Run affected AWS/ADC owner/XML/Query/mTLS suites and explicit selected-source auth cases; no hidden codec I/O/tracing/globals or excluded cloud service/auth runtime dependency admitted. |
| P20-DEL-PROV-06 / not-assessable | docs/research/pi-ai-opi-ai-alignment/spec-draft.md @ 8c6cd66f7771e026210f06e49d037da54029c74258a1d45925cdbddd710f5f21; lines 510; backend lines 64,66,216 | run_early_package_proof.py and package_slice.py use actual extracted opi-ai ordinary/caller-union graphs; registry/source guards and ICU4X_DATA_DIR rejection already implemented. Current scope is exact effective graph, not eternal Unicode data equivalence. | specified-external-schema-http-proof; Execute schema defaults-off/caller-default-regress73checks each and HTTP ordinary/union6checks each; retain actual dependency/checksum/effective-feature/data/license/build provenance and regression outcomes. |
| P20-DEL-PLAT-01 / not-assessable | docs/research/pi-ai-opi-ai-alignment/spec-draft.md @ 8c6cd66f7771e026210f06e49d037da54029c74258a1d45925cdbddd710f5f21; lines 28-29,502; backend lines 186,188,206 | Current CI has three native OS workspace jobs and Linux MSRV check, but no full three-host exact-MSRV package matrix. Earlier native evidence is acknowledged with changed-input limitation. | specified-native-final-byte-green; Add exact1.97.1 three-native-host package job; run selected actual package schema/http/crypto/ws/default/C-union/readme/external consumers and all relevant behavior tests. Native logs/artifacts are mandatory; cross-check or config alone cannot pass. |
| P20-DEL-PLAT-02 / not-assessable | docs/research/pi-ai-opi-ai-alignment/spec-draft.md @ 8c6cd66f7771e026210f06e49d037da54029c74258a1d45925cdbddd710f5f21; lines 28,502; backend lines 186,206 | Current CI has three native OS workspace jobs and Linux MSRV check, but no full three-host exact-MSRV package matrix. Earlier native evidence is acknowledged with changed-input limitation. | specified-native-final-byte-green; Add exact1.97.1 three-native-host package job; run selected actual package schema/http/crypto/ws/default/C-union/readme/external consumers and all relevant behavior tests. Native logs/artifacts are mandatory; cross-check or config alone cannot pass. |
| P20-DEL-PLAT-03 / not-assessable | docs/research/pi-ai-opi-ai-alignment/spec-draft.md @ 8c6cd66f7771e026210f06e49d037da54029c74258a1d45925cdbddd710f5f21; lines 486; backend lines 188,216 | Existing native_proof_guard rejects system/override provenance and binds current Cargo unit outdir,fingerprints,source bytes,compiler,link flags,static archive. Baseline non-FIPS, native roots/TLS/FIPS/other unions not silently supported. | specified-native-provenance-green; Run crypto ordinary/union and ws ordinary/default/C-zlib profiles with AWS_LC_SYS_USE_SYSTEM=0,LIBZ_SYS_STATIC=1,CC_ENABLE_DEBUG_OUTPUT=1 and controlled env; retain exact CC/CMake-if-used/tool/source/build/output/link identities. Unsupported unions remain excluded by their existing contract. |
| P20-DEL-PLAT-04 / not-assessable | docs/research/pi-ai-opi-ai-alignment/spec-draft.md @ 8c6cd66f7771e026210f06e49d037da54029c74258a1d45925cdbddd710f5f21; lines 94,97,215,236,245 | Current source SDK action/RDF tables define public Rust equivalents and native/Tokio/Stream/Serde substitutions; copied TS symbols do not define Rust runtime support. | specified-source-export-coverage; Final exact57SDK/nine-host/one-retired action table links actual public/package case or explicit registered platform difference and limitation; unresolved missing semantic input stays OPEN, never Unsupported solely to clear row. |
| P20-DEL-ONB-01-B / not-assessable | docs/research/pi-ai-opi-ai-alignment/acceptance-matrix.md @ c831586339910f9496a64e6b3c9390a0b7a2a366d37230222874a5f399099a5d; ONB-01 line 531; spec-draft.md line 554 | Current quick_start,quick_start_stream,tool_loop example bytes equal both README embeds; run_readme_examples runner checks package bytes,actual request and continuation separately from complex consumer. | specified-same-source-first-use-proof; Run cargo check examples/doc checker; actual three fresh package example flows execute with0setup/unpolled work and1/1/2observed sends, including tool-result continuation; archive exact source/artifact/graph/log identities. |
| P20-DEL-ONB-02-A / not-assessable | docs/research/pi-ai-opi-ai-alignment/spec-draft.md @ 8c6cd66f7771e026210f06e49d037da54029c74258a1d45925cdbddd710f5f21; ONB-02 line 555; matrix line 532 | Current quick_start,quick_start_stream,tool_loop example bytes equal both README embeds; run_readme_examples runner checks package bytes,actual request and continuation separately from complex consumer. | specified-same-source-first-use-proof; Run cargo check examples/doc checker; actual three fresh package example flows execute with0setup/unpolled work and1/1/2observed sends, including tool-result continuation; archive exact source/artifact/graph/log identities. |
| P20-DEL-ONB-02-B / not-assessable | docs/research/pi-ai-opi-ai-alignment/spec-draft.md @ 8c6cd66f7771e026210f06e49d037da54029c74258a1d45925cdbddd710f5f21; ONB-02 line 555; matrix line 532 | Current quick_start,quick_start_stream,tool_loop example bytes equal both README embeds; run_readme_examples runner checks package bytes,actual request and continuation separately from complex consumer. | specified-same-source-first-use-proof; Run cargo check examples/doc checker; actual three fresh package example flows execute with0setup/unpolled work and1/1/2observed sends, including tool-result continuation; archive exact source/artifact/graph/log identities. |
| P20-DEL-ONB-03-C / not-assessable | docs/research/pi-ai-opi-ai-alignment/spec-draft.md @ 8c6cd66f7771e026210f06e49d037da54029c74258a1d45925cdbddd710f5f21; ONB-03 line 556 | codex_native_options.rs226/365/392 covers native omission/override/null; vertex_fixtures.rs3882 onward asserts native Google-1/0 untouched; Anthropic native default1024 is protocol-specific and distinct from C008 simple numeric answer-plus-budget defect. | current-native-options-controls-plus-final-green; Run codex_native_options,anthropic_fixtures,gemini_fixtures,vertex_fixtures and B08; add exact native enabled-no-budget1024 if not already asserted, without imposing universal clamp. |
| P20-DEL-ONB-04-C / not-assessable | docs/research/pi-ai-opi-ai-alignment/spec-draft.md @ 8c6cd66f7771e026210f06e49d037da54029c74258a1d45925cdbddd710f5f21; ONB-04 line 557; matrix line 534 | external_sdk.rs999 and protocol fixture callbacks already distinguish source payload/raw/response availability. Image received-result callback defect is B11, diagnostic leak B03. | specified-cross-operation-callback-proof; B17 plus all supported protocol/operation callback variants; explicit unsupported callback must fail pre-send, actual raw events at source seam and exact callback/send counts. B11 preserves received facts on error/cancel. |
| P20-DEL-ONB-06-A / not-assessable | docs/research/pi-ai-opi-ai-alignment/spec-draft.md @ 8c6cd66f7771e026210f06e49d037da54029c74258a1d45925cdbddd710f5f21; ONB-06 line 559; matrix line 536 | Current README embeds source-equal examples; old macOS pending prose is not proof no later native run occurred. Current final-source native receipt remains future. | specified-final-bilingual-claim-reconciliation; After actual package/native/MSRV runs, update both README claim/evidence links together; doc checker and diff check. Missing native execution is shown as missing and cannot pass a claim of support. |
| P20-DEL-ONB-07-A / not-assessable | docs/research/pi-ai-opi-ai-alignment/spec-draft.md @ 8c6cd66f7771e026210f06e49d037da54029c74258a1d45925cdbddd710f5f21; ONB-07 line 560; matrix line 537 | Current README embeds source-equal examples; old macOS pending prose is not proof no later native run occurred. Current final-source native receipt remains future. | specified-final-bilingual-claim-reconciliation; After actual package/native/MSRV runs, update both README claim/evidence links together; doc checker and diff check. Missing native execution is shown as missing and cannot pass a claim of support. |
| P20-DEL-ONB-07-B / not-assessable | docs/research/pi-ai-opi-ai-alignment/spec-draft.md @ 8c6cd66f7771e026210f06e49d037da54029c74258a1d45925cdbddd710f5f21; ONB-07 line 560; matrix lines 537,558-603 | Fixed current Pi/Opi/index identities, original audit states and test impact retained; a present test file is not executed proof. | specified-documentary-collection; Final reconciliation writes existing upstream tracking/evidence locations with per-test source input, target/filter, actual exit/count/skip/missing and immutable artifact/log hashes. Do not rewrite historical70/probe baseline. |
| P20-DEL-ONB-07-C / not-assessable | docs/research/pi-ai-opi-ai-alignment/spec-draft.md @ 8c6cd66f7771e026210f06e49d037da54029c74258a1d45925cdbddd710f5f21; ONB-07 line 560; lines 562-570; matrix line 537 | Explicit opi-implement P0/P2 READY, exact P4 confirm-all then write-only install;16live registered hashes match. Active tasks0;68historical summaries and phase_exit/frozen bytes unchanged. Live ledgerSHA 8ed820e0a10830b70f453eddc01e1700ffdb68458534febefa22e220bc53780b. No runtime/audit closure inferred from archive. | Completed owner source binding; preserve exact committed live ledger and original owner receipt during apply. A later non-plan commit requires new remediation export/binding. Later independent F1a remains its owner. |

### Sealed package and native execution design

The following finite harness/collection roots are separate from every runtime correction and every test-only assertion root. They are now individually assigned B90/B91/B92/B93 predicates, with B17 as the separate assembled consumer prerequisite. They do not form an umbrella closed batch or authorize source publication/CI dispatch. P007 has a concrete test-only closure conjunction; its green remains not-run. Repaired-byte green is future execution. The supported backend, public-input shape and exact one-input baseline classification are now resolved in B98/B99 and the accepted user decision.

1. Source-export cache binding -> verify: extend shared `run_early_package_proof.validate_paths` at its three existing runner entry points; B17 owns its later new-caller wiring with `--cache-workspace <actual-repository>` used only to validate the existing marker owner. Packaging cwd/source stays the sealed export. Check scratch outside both roots/cache; bind export and owner paths/hash; reject owner mismatch/marker rewrite/reused scratch/config source override. Existing no-selector behavior stays exact. Files: run_early_package_proof.py,package_slice.py,run_readme_examples.py,run_external_sdk.py and their owning Python tests. No new cache target or marker rewrite.
2. Complete extracted-package family proof -> verify: use actual Cargo-produced .crate and safe extraction; exact source/resource/license/test/helper hashes; fresh external consumer solely Opi runtime dependency opi-ai. Schema profiles defaults-off/caller-default-regress each run current73named checks plus the required finite public schema cases; HTTP/crypto ordinary and caller-union each run6named consumer checks; WS ordinary/default/C-zlib each run10named consumer checks, plus separately counted exact discovered packaged-private and public-Upgrade named corpora. Replace minimum95/15-only acceptance with exact required test-name set, reject missing/duplicate/extra/ignored/filtered/zero/nonzero exit. Public conformance test copies include only sealed test/fixture bytes and public SDK calls; no production include/private codec export/test-only feature. README runner executes same-source3flows, B17 exact19assembled cases. Record real resolved graphs and checksums; no copied workspace lock as consumer authority.
3. Three-native-host exact-MSRV route -> verify: add a dedicated CI package job for ubuntu-latest/windows-latest/macos-latest, fail-fast:false, exact rustc1.97.1 and existing pinned checkout/rust-toolchain/rust-cache actions from current ci.yml. Keep current jobs unchanged. The new orchestration Python script resolves/quick-inspects the workspace/toolchain persistent cache via opi-cargo-cache.py resolve/status, obtains registry prerequisites with cargo fetch --locked before offline proof, and uses fresh external scratch per family. Actual proof subprocess environment removes RUSTFLAGS/CARGO_ENCODED_RUSTFLAGS and wrapper/source overrides rather than setting them to empty (the current guard rejects key presence); apply warnings through clippy -D warnings separately. Preserve AWS_LC_SYS_USE_SYSTEM=0,LIBZ_SYS_STATIC=1,CC_ENABLE_DEBUG_OUTPUT=1 and actual CC/CMake-if-used/tool/source/build/outdir/static-link proof. No guard weakening, paid provider, credential discovery or source publication. A CI run requires a separately authorized publication/PR or an owning native host; this planning turn performs none. A scheduled job/config/cross-check is not native execution evidence. Original exit/log/archive/executable/feature/target/toolchain/native receipts must exist before a result can pass.
4. Final documentary reconciliation -> verify: after all actual runtime/package/native prerequisites pass, update the existing upstream-tracking/evidence locations and both crate README files together with exact current Pi/Opi/index/package/graph/toolchain/run identities, per-test add/update/delete/retain/none, commands/filters/executed/ignored/skip/missing counts and explicit RDF/platform limits. Catalog checker validates the authentic current resource without generate/install/hydrate/live fetch. Link exact proof receipts; do not rewrite historical70/probe baselines, audit inputs or source authority. Doc checker and diff check pass. Later independent F1a/Phase exit and ledger writes remain with their existing owner.

### New raw evidence and workflow verification

| Artifact | SHA256 | Actual scope/outcome |
|---|---|---|
| `C:\Users\Luiz\AppData\Local\Temp\opi-c018-group-a-ibbr1o2n\criterion-proof-map.json` | `12e0761cce08373aae0c09f0117c11088836ba7cae4991885c66420a851d7b66` | 38exactsource rows; WS023 red and finitecases |
| `C:\Users\Luiz\AppData\Local\Temp\opi-phase20-remediate-plan-jqdki4yk\c018-group-b-proof-map.json` | `d353b31b17321e379a891d9e859996378326c4f4099b87a693acd0c4a59284b6` | 63exactsource rows; current assertions and finitecases |
| `C:\Users\Luiz\AppData\Local\Temp\opi-p20-ready-continuation-vx_jjsra\root-mapping.json` | `5628b13c6ebbdeacfe553cf3ad9a1f9faa602057abd7927ee7d4d5c7f1d901fb` | 55exactsource rows; current and future owner/proof routes |
| `C:\Users\Luiz\AppData\Local\Temp\opi-p20-ready-continuation-vx_jjsra\catalog-check.json` | `f48449a919efba131c209d32de593522c6b666e75c7c8a7d21abea294083255a` | PASS read-only authentic artifact/manifest/generator comparison;42providers/1608chat/38classifier/59image |
| `C:\Users\Luiz\AppData\Local\Temp\opi-phase20-remediate-plan-jqdki4yk\c018-sdk-probe-receipts.json` | `a105e293c8e3072eec7244c15f31814ed4f2fa4bafd711682c913576f0465b83` | sampling3cases2pass1fail;duration2cases1pass1fail;0ignored |
| `C:\Users\Luiz\AppData\Local\Temp\opi-phase20-remediate-plan-jqdki4yk\c018-duration-matrix-receipt.json` | `4153ea420940c78121a57f03ca9976c96a6f0aeffaa1167bb5cd3a4297392e07` | 10HTTPcompletions pass; aggregate duration test1fail,5None/5Some0 |
| `C:\Users\Luiz\AppData\Local\Temp\opi-phase20-remediate-plan-jqdki4yk\c018-radius-receipt.json` | `89779656b4a261f5f089fdf19c826e40b56590b50779aea95912fbefb9113fbb` | 2constructor tests1pass1fail;0ignored |
| `C:\Users\Luiz\AppData\Local\Temp\opi-http005-overread-k6nnyhhs\probe-identities.json` | `79de1b06f8b9f3e59efda2b25b1356920c15a6c6ba65c7f63e87cf0c7d2cf688` | CONNECT identical TLS alert separatedcontrol vs coalescedloss;1test1fail0ignored |
| `C:\Users\Luiz\AppData\Local\Temp\opi-ws023-instrumented-z20k2lui\probe-identities.json` | `fc24e122c55944b38402406d8c8e03179fd723cca1a6f20905464a9fe990cf67` | actual logical cancellation;1test1fail0ignored; cfgtestobserver only |
| `C:\Users\Luiz\AppData\Local\Temp\opi-p20-ready-continuation-vx_jjsra\workflow-tests-final.log` | `5738bbc754287829c68f34d53fcf70c484e1ae7c54a2da09f93fad0f3a3124d3` | 173tests PASS,0failures,exit0,186.267s; current final workflow-script bytes |

Original continuation test impact: maintenance add16regressions/update sharedfixtures/retain others/delete none; that original continuation added temporary probes and planned finite tests without Rust product edits. The present baseline separately includes the authorized B01 correction. Canonical workflow docs have no Chinese sibling; product apply README/semantic changes require both languages. The active index and member bytes remain unchanged. At that original continuation stage only the separately authorized live ledger binding and workflow/ledger prerequisite commit had occurred. Later authorized B01 is accounted for in the current source baseline above. No complete apply result, branch, push, PR, release or CI-dispatch action is claimed.

### Final Codex WS timing source qualification

Actual plain and compressed Models→Codex→Upgrade→private codec chains passed original peer/id/Stop/text/Close controls before finding durationNone. Full copied public consumer:8pass/2fail/total10,JSONverdictFAIL,original process exit0. Receipt `C:\Users\Luiz\AppData\Local\Temp\opi-phase20-remediate-plan-jqdki4yk\c018-ws-duration-receipt.json` SHA256 `dc172c47fe50ac678f151fcdb66c290610ee7d12d744e9e572a81412b640cea6`. This joins the same B31 private per-active-call timing predicate; it is local behavior proof, not final .crate/native acceptance. Existing consumer parser must reject failed JSON regardless of process exit.

### Independent review corrections

B29 now lists the private test-root module declaration; B31 source scope matches actual five HTTP and ordinary/compressed WS red. B47/B51/B62 additions to external_sdk.rs are explicitly assertions/helpers inside named existing nineteen cases. B50 selects ModelsStoreError::Corrupt uniquely from current decode_store_document mapping. Independent catalog lifecycle/version roots are B48/B94. All original source identities/states remain unchanged.

### Current planning completion boundary

D2 is resolved by exact source utility/assembly semantics and real ALL fallback red; Radius id/name is resolved by existing public seams and B96 neutral identity red; CONNECT has the selected ordinary public-API owner design and qualified negative red; D8 is completed under separately authorized opi-implement source review/graph/write-only authority. The user-confirmed B98 static input baseline resolves the last planning decision. All remaining planned repair green remains not-run. No partial apply or fake compiler red is admitted.


## Accepted one-input baseline classification

The user explicitly accepted only P20-SDK-AUTH-006's static missing-input baseline after seeing the source/API/caller evidence and exact runtime matrix. B98 now contains that whole selected design and literal approval. No current compiler error is represented as red, no general validator was changed, and no actual runtime acceptance obligation was removed. All other behavioral fixes retain their genuine runtime baseline. New apply approval of the final digest remains required.

### New evidence receipts

| Artifact | SHA256 | Actual scope |
|---|---|---|
| `C:\Users\Luiz\AppData\Local\Temp\opi-p20-proxy-utility-red-xb4aauuz\receipt.json` | `d695afff2823bcd35e6403bf8b3a28a1fcf5db6f0806ecb3ec3b322527cc0126` | utility3cases2pass1fail |
| `C:\Users\Luiz\AppData\Local\Temp\opi-p20-proxy-utility-red-xb4aauuz\closure-design.md` | `8894b8059dae1f578e95cf94088263c9aba7759f45e52e14207bb7a953e22678` | source-qualified utility/actual caller assembly design |
| `C:\Users\Luiz\AppData\Local\Temp\opi-phase20-remediate-plan-jqdki4yk\c018-d5-radius-seams-receipt.json` | `c1c455f9c7e4c9853fb4c35d5d03d7cb3b6299dfc9ec514c40c73d69c0929703` | Radius3tests2pass1fail; existing input controls pass |
| `C:\Users\Luiz\AppData\Local\Temp\opi-p20-ready-final-dp5e95fw\transport-typecheck-receipt.json` | `283e494c9468cebb511ec15a8d5518fcb3da0053033deffcb3258ff78b02705f` | public type composition only;258exact registry,0runtime/0network |
| `C:\Users\Luiz\AppData\Local\Temp\opi-p20-ready-final-dp5e95fw\transport-public-typecheck-frames.log` | `5f6e19777fb06b625d4f0e14732d9f63b8a902ae9fc14df7d39b63e1ab31ba3e` | Body::wrap(Incoming) frame-preserving API typecheck PASS |
| `C:\Users\Luiz\AppData\Local\Temp\opi-http005-protocol-qualification-0uw_lin0\qualification.json` | `f02f852dcb8f6436b6e3cc7579574186364510bb36b92a92a40cea75d09661bf` | negative byte-handoff qualification,not normal TLS handshake |
| `C:\Users\Luiz\AppData\Local\Temp\opi-p20-ready-final-dp5e95fw\review.json` | `49da80ccbf387bb47dcec1db1b0d4cd0dd1b0300200f53e4f80824a56b97a7b9` | bounded ordinary-backend/single-owner source review |
| `C:\Users\Luiz\AppData\Local\Temp\opi-p20-ready-final-dp5e95fw\implement-source-reconciliation-receipt.json` | `8ba4e834fd798722f61eaccd3a0fb45dbfb58d9b9d90d9ddc175d9f883b5f365` | explicit source-owner P0/P2/P4 confirmed write-only install |

## Final sealing and materialization boundary

The complete plan has98formal batches over the exact28finding strict union and156linked C018 criterion records. There are no pending decisions or remaining obligations. All original requirement states and findings/index identities are preserved. The only static input baseline is the explicit user acceptance for B98/SDK-AUTH-006; every other runtime repair retains actual red evidence and all new-input runtime green remains mandatory.

The user explicitly authorized only verified B01 commit `2eee4afebc911482d969ead8452115a4a7f9592d`, now the current remediation source baseline. Preserve those exact bytes. Earlier workflow/ledger prerequisite68e2768 and original executions keep their own provenance, with current applicability separately qualified above. Any further non-plan commit requires fresh source export/rebinding; a subsequent commit of only this plan pair follows the plan-only exception. No remaining repair, push, PR, CI dispatch, Phase verdict or ledger update is authorized by this planning output.

Current outgoing edits are only the fixed plan pair, following authorized B01 and the preserved earlier workflow/live-source ledger maintenance. Test impact: workflow add16/update fixtures/retain others/delete none (173tests passed on unchanged workflow script bytes); owner metadata none; temporary probes add; repository Rust tests none in plan mode. Future apply tests and0.x/public caller changes are enumerated per batch. Final plan checks are audit-set,dispositions,plan,live ledger guard,source/criterion identity checks,doc checker and git diff --check.

Final no-network public API composition receipt: `C:\Users\Luiz\AppData\Local\Temp\opi-p20-ready-final-dp5e95fw\transport-typecheck-final-receipt.json` SHA256 `711f22da0ca642dae4713a5b83a008eea9606f074101b034168dbddacfe7d4b5`. It records zero behavior tests and is not a repaired green. The optional local CONNECT prototype sub-agent was stopped by automatic content checking; the selected route instead has qualified existing red, locked-source proof and no-network type evidence, with actual wire behavior still required at apply.

### Mechanical probe and caller migration limits

Source-authorized0.x interface changes may require explicit mechanical green-probe shims (new AuthResolver argument None, declared expiry/sampling representation or controlled HTTP handle/error type). Record original/migrated source hashes and exact shim diff; keep semantic inputs,expected wire effects/case counts and assertions unchanged. Compiler/setup errors are never behavioral red. B97 specifically keeps its public_proxy.rs bytes and3env/JSON cases unchanged, with only the disclosed main invocation migration. Any semantic change to a probe requires new discriminating evidence. New consumer fixtures use the final B98 AuthResolver interface; existing and earlier-added implementations within the already declared C018 owned paths receive only its mechanical optional-argument synchronization.
