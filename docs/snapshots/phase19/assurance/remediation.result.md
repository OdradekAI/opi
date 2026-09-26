# Phase 19 remediation result

**Status**: COMPLETE
**Audit index SHA-256**: `702853fe37c1779c38612f7979a7e995395d1890954c4d3d8d6205592625e6d3`
**Plan SHA-256**: `091ebca29c960a6816a79932502dd17e3850a7229d8b4a98c059c843e1926d1a`
**Changed paths**: ["CHANGELOG.md", "crates/opi-eval/README.md", "crates/opi-eval/README.zh.md", "crates/opi-eval/external-locks/static/linux-x86_64.json", "crates/opi-eval/integrations/harbor/opi_eval_harbor.py", "crates/opi-eval/integrations/pier/opi_eval_pier.py", "crates/opi-eval/scripts/materialize-external-locks.sh", "crates/opi-eval/scripts/test_native_integrations.py", "crates/opi-eval/scripts/test_prepare_acquisition.py", "crates/opi-eval/scripts/test_verify_external_lock_artifact.py", "crates/opi-eval/tests/native_driver.rs", "crates/opi-eval/tests/support/native_witness_material.rs"]
**Remediation head**: `62758ce04ec19e7c1bfeb3ea776c7eed6d7fd0bd`
**Dispositions**: `remediation.result.dispositions.jsonl`

## Outcome and approval boundary

The exact approved apply plan is complete: B1, B2 and B3 are Closed. The strict union contains 11 source findings: three planned closures and eight independently Refuted findings. Three bounded verification-blocking incidental repairs, I1, I2 and I3, are recorded separately. All source disposition fields are copied from the approved plan except observed green-after and final remediation status.

The plan, active index, indexed member identities and committed HEAD remain unchanged. The two plan files were already unstaged at admission and were preserved byte-for-byte. This apply changed the twelve attributed paths above and the two fixed result outputs. It did not commit, update the implementation ledger, invoke another opi workflow, or grant Phase PASS.

## B1 — locked standalone pi runtime

The selected pi package now includes chord 0.85.1, photon-node 0.3.4, jiti 2.7.0, esbuild 0.28.1 and its Linux x64 executable from the locked offline build. The known chord workspace link is resolved into regular release files. Missing packages, identity drift, escaping/nested links and special files reject before cache publication. Bounded isolated startup, ESM imports and an actual native esbuild transform pass. Install hooks and dependency versions remain unchanged. Only the materializer producer digest in the production static lock changed; the new LF digest is `edaae15faa05eb21b237f763af5d552ab43bc7c0e90f434cb23982f4bf1883cd`.

All three benchmark families passed independent-directory public cold/warm preparation with both selected real Agent packages. Warm preparation schedules no acquisition and preserves the frozen selected Agent, benchmark and static identities. English/Chinese README text and Unreleased release notes are synchronized. Exact recipes, package identities and command results are in external `b1-closure.json` and `public-*-identities.json`.

## B2 — pinned native grader contrasts

The three original ignored artifact witnesses executed against the corresponding B1 materials. All six retained raw trials use the pinned native runner and official task grader: oracle reward 1 versus nop reward 0, no native exception, and distinct collected artifact/patch digests. The independent inspector verified every retained inventory digest and the expected collection/patch provenance. These are grader-sensitivity diagnostics, not Agent benchmark scores. See external `b2-closure.json` and `retained-official-witnesses/`.

## B3 — current clients and native lifecycle

The actual Opi HEAD package and locked pi package pass the protocol matrix, 52-turn default/projection/pi budgets, explicit two-turn boundaries, omitted-control pre-dispatch rejection, and exact 4096 output-cap probe. The probe uses a separate experiment with context window 32768. At context 8192, pinned pi intentionally clamps the wire cap to 2679/2658 after prompt estimation and its 4096-token reserve; that failed probe remains retained and is not represented as passing. The cap, client bytes and equality assertion were not weakened.

Private Git/Python tasks use the unchanged pinned Harbor/Pier runners and the admitted immutable image `python@sha256:272efc3f94bec901e2bc7a5c9984b3d113c317839edca2714c6a388b9476f8fc`. They contain a real Git base/patch, actual content-sensitive Python grading and independently hashed task/profile/corpus material. The image lacks Git, so the private task context retains the host Git executable, dynamic loader/libraries and notices; those bytes are covered by task manifests. No production dependency, image pin or schema was added.

All 27 deadline pairs (54 trials) passed: positive, time-cap, agent-cap, verifier-cap, both-agent, both-verifier, minimum-cap, strict-task and unbounded-task across all three families. Positive controls ran first. Agent timeouts retain valid incorrect artifacts; verifier timeouts produce typed rejection without reward. Pier retains its existing 30-second verifier limit and retry; outer 510/270-second bounds, command settlement assertions and neighbor/shared-image checks are unchanged.

Seven verifier-only regrade pairs passed: positive and negative in all families, plus Terminal-Bench 2.1 timeout. Fresh outputs contain no Agent logs or synthetic credential canaries; original run bytes, frozen material and provider request logs remain unchanged. Missing/tampered inputs and bundle, task/base/network drift, missing cache, existing output and uncertain cleanup reject as required. The independent Docker query confirms owned resources are removed.

Each family also passed public `doctor`, `prepare`, `validate`, `plan`, `run --plan`, `verify`, `report` and `regrade` from its own directory with seeded private diagnostic material and no `--fixtures`. These lifecycle clients and tasks are explicitly diagnostic; this result does not upgrade saved live-trial claims or establish real-provider acceptance. See `b3-closure.json`, public workflow identity records and retained native diagnostic inventories.

## Verification

Rust and source-script checks use the faithful LF ext4 archive export of HEAD with only the attributed source overlay, preserved Git executable bits, incremental compilation, and the repository external Cargo target. The current candidate file digests and unchanged executable builder are recorded in `final-candidate-bindings.json`. Dynamic command records bind their exact Companion executable digest. The fixed binary used by final public CLI workflows is retained with SHA-256 `bbebe5519a70d9356059070dabc386dcab06e10b417aa38e9337b4200543f830`; a subsequent Cargo build produced `7088be0373b10aca17bf88820914ea2225a91e21869e349bb324997686817963`, separately retained. Both derive from identical production Rust sources; they are not represented as byte-identical binaries.

| Exact check | Result |
| --- | --- |
| `python3 crates/opi-eval/scripts/test_prepare_acquisition.py` | PASS, 16.90 s; eight tests, including 14 malformed-runtime subcases |
| `python3 -m unittest discover -s crates/opi-eval/scripts -p 'test_*.py'` | PASS, 40.50 s; 228 tests, one intentional skip |
| `python3 crates/opi-eval/scripts/test_native_integrations.py` | PASS, 14.07 s; 32 tests, one intentional skip |
| `python3 crates/opi-eval/scripts/test_verify_external_lock_artifact.py` | PASS, 2.11 s; 53 tests |
| `cargo test -p opi-eval --lib --test native_driver --test assembled_run --test agent_integration_conformance -- --test-threads=4` | PASS, 187.49 s; 248 passed, nine opt-in witnesses executed separately |
| `cargo fmt --check --all` | PASS, 1.53 s |
| `cargo clippy --workspace --all-targets -- -D warnings` | PASS, 18.09 s |
| `cargo test --workspace --all-targets` | PASS, 427.77 s |
| `cargo test --workspace --doc` | PASS, 150.00 s |
| `RUSTDOCFLAGS='-D warnings' cargo doc --workspace --no-deps` | PASS, 78.12 s |

Each native witness uses `cargo test -p opi-eval --test native_driver <filter> -- --ignored --exact --nocapture --test-threads=1` with the recorded matching `OPI_EVAL_NATIVE_WITNESS_MATERIAL` or `OPI_EVAL_NATIVE_CLIENT_MATERIAL`. The nine exact filters are `native_tb21_artifact_witness`, `native_tb3_artifact_witness`, `native_deepswe_patch_witness`, `native_protocol_client_matrix`, `native_client_turn_budgets`, `native_output_control_client_matrix`, `native_owned_resource_cancellation`, `remediation_native_stage_deadlines` and `native_verifier_only_regrade_witness`. Every filter passed; raw test output remains alongside its command record.

The initial source documentation check passed. The fixed result and final main-worktree documentation/diff checks are recorded in the completion-validation evidence after result generation. The active audit set and exact plan digest were revalidated before writing this result.

Deadline/client observations bind to `b3-v4-candidate-bindings.json` and the independently retained `retained-v4-source-overlay/`. Subsequent material changes affect only the `deadlines == false` regrade branch: an explicit offline baseline and native cached-image naming. The command recorder additionally asserts the executable digest is unchanged across each call. `reused-test-bindings-final.json` proves the complete test driver is unchanged and isolates these helper edits. I3 changes only the verifier-only manifest consumer sort key; `i3-integration-bindings.json` proves every other integration source byte is unchanged. Deadline material generation and client/cancellation behavior therefore remain unchanged. Final regrade, public workflows, Python union, focused Rust checks, formatting, Clippy and workspace tests use the final candidate.

Test impact: **add** malformed runtime-package coverage, private controlled-material construction and the cross-adapter manifest-order regression; **update** the owning native witness/retention helpers and the I1/I2 fixtures; **retain** existing bounds, exact native assertions and regression counterexamples; **delete** none. Production Rust/Cargo behavior changes: **none**. I3 changes two Python verifier-consumer sort keys to match the unchanged canonical manifest format.

## Retained failed attempts and corrections

All unsuccessful attempts remain in the command ledger. The initial ESM smoke harness used CommonJS resolution for an ESM-only chord export; the corrected standalone ESM probe passes. I1 fixes the missing already-pinned builder fixture. I2 fixes only the unavailable owned-container fixture image. The inadequate-context output probe is explained above.

Private material construction initially exposed missing version anchors/tags, an invalid three-segment native task name, and an empty artifact after Agent timeout. The final constructor supplies truthful private identities, valid `org/name` task names, family-correct work directories and a pre-existing incorrect artifact. All 27 generated configurations were checked by the exact pinned runner schemas; the full deadline matrix passes without assertion, timeout or production-code changes. The first regrade attempt correctly rejected an omitted environment network baseline; the private regrade branch now explicitly sets Harbor `network_mode = "no-network"` and Pier `allow_internet = false`, in addition to phase policy. These failed diagnostic materials and logs are retained separately from final evidence.

Subsequent regrade attempts exposed native build-image naming and Pier filtered-Agent-egress constraints. The private constructor now retains the native cached verifier image through task-authored Compose naming, with Pier Compose limited to the verifier context. No runner code or cache requirement changed. I3 then exposed a real consumer ordering defect: all 22 task files matched, but Python component ordering differed from the Rust canonical relative-string order. Its test failed on both adapters before the two-line repair; final materials use new integration identities.

Three concurrent public preparations temporarily reported unknown engine storage when the existing 12-second observation deadline expired. A serial read-only doctor check observed the same engine and available storage; final workflows run sequentially with unchanged deadlines. The earlier workspace build was intentionally interrupted after 979.63 seconds of compilation to release the mutable CLI for native checks; that exit -2 is not a passing gate. The later full workspace command is reported separately above. All failed evidence remains retained, including `failed-diagnostic-inventories.json` and the I3 supplemental failure inventory.

## Evidence and materialization handoff

The first completed workspace attempt stopped at the unmodified `artifact_audit_accepts_real_declared_commit_objects` regression: a pure Git archive has no commit object database. The exact focused filter reproduced this failure. `archive-git-context.json` records a detached, local metadata directory in the archive export with an object alternate to the original repository; no source files, commits, branches, user Git metadata or worktree registrations were changed. Real `git cat-file` now resolves the same existing commit, the focused test passes, and the full workspace gate was rerun. This verification-environment correction is separate from the three source incidental repairs.

Host command records, stdout/stderr digests, approval baseline, candidate bindings, exact replay scripts, failure diagnoses, closure records and complete command ledger are at:

`D:/Luiz/Odradek/opi-remediation-evidence/phase19-apply-20260926-bd37/`

WSL native caches, packages, retained raw official witnesses, controlled native materials, private captures and source export are at:

`/home/luiz/.local/state/opi-remediation-evidence/phase19-apply-20260926-bd37/`

`command-ledger.md` lists every captured exact command, cwd, exit and duration; `command-ledger.json` retains allowlisted environments and output hashes. Closure JSON and retained inventory JSON identify raw paths and SHA-256 values. Original temporary roots are preserved and copied to durable retained roots; material paths remain absolute and are explicitly external evidence dependencies. Capacity observations are measured by `doctor`; unknown GPU capacity remains unknown.

No commit was requested or made. Before a fresh audit or reviewer rerun, the fixes plus the current live assurance set must be committed, the assurance directory must be clean, and these external evidence dependencies must be resolved. The approved plan is not rewritten retroactively. Phase PASS and release readiness remain outside this apply result.
