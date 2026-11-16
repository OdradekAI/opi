# #113 Preserve final numeric decoding through tools and sessions

Status: final numeric decoding repair and bounded verification complete.
Previously approved representation differences remain explicit.

Continuation of [#112](112-retain-complete-preview-at-representation-boundaries.md),
requested by the user on 2026-11-14. Worktree HEAD remains
`ee18dd1323a089d536bdc7e293d0b4869777c344`. Frozen Pi 1.5.0 is revision
`f9a26365568127461c47a07a37dcb42c140a3ac5` at
`C:/Temp/opi-ai-frozen-lock-20261111`, retaining its original lock and OpenAI
7.19.0. No live provider, credentials, staging, commit, branch or publication.

## Failure and repair

The raw integer one followed by 64 zeroes decodes to the adjacent lower f64
`9.999999999999999e63` with the previous serde_json configuration. Its bits are
`4d384f03e93ff9f4`; Pi and correctly rounded Rust parsing produce `1e64`, bits
`4d384f03e93ff9f5`. The sealed #112 interface executable reproduces the error.
`ToolCallArgumentsStream::finish` returns raw text/validity and is not the
source of this loss.

New failing consumer tests establish three consequences: the ordinary Agent
passes the lower value to hooks, authorization and the actual tool; a schema
requiring exactly `1e64` rejects it before authorization/execution; and
`SessionCoordinator` writes the lower value during argument redaction. Reading
that serialization and writing it again can introduce another rounding change.

The only production change enables `serde_json/float_roundtrip` in the SDK's
ordinary dependency declaration. The locked serde_json 1.0.149 implements this
without additional dependencies or a lock-version change. Cargo unifies the
feature for callers using that same version, including Agent/product and SDK
package consumers. It is not confined to one function. The root workspace's
independent serde_json defaults are unchanged. A custom JSON parser would add
unnecessary grammar and recovery logic while leaving later generic decoding
exposed to the same error.

Native `i64`/`u64` numbers remain exact; the change does not force all integers
through JavaScript Number. Raw final text, strict overflow/Unicode/depth
rejection, preview ownership and redaction policies remain intact. Opening an
old session does not rewrite its bytes or execute historical tools. Already
misrounded persisted text cannot reliably reveal the original number, so this
change does not migrate old sessions.

## Acceptance and evidence

- SDK tests cover ten finite numeric spellings, independent f64 bit checks,
  serialization/re-decoding, strict errors and exact native integers.
- Product tests use the real Agent loop with a scripted provider and harmless
  tool, checking hook, schema admission, trusted authorization, tool execution
  and public event values. The session test uses real disk writes, closes and
  reads the file, reconstructs history, opens it unchanged, supplies restored
  history to the next request and executes one new call from the restored
  arguments. A second save/read preserves both calls and the prior file prefix.
- Frozen Pi controls use the actual final parser, public `runToolCall` and
  `SessionManager` disk recovery through frozen source imports. These are
  component controls, not a whole Pi Agent loop or authorization-policy match.
- The paired preview comparator removes the old numeric-difference exemption
  and binds the long-scalar final text to accumulated input. Old rounding,
  synchronized forged values and forged accumulated/final text must fail.

Raw attempts, including the initial test compilation correction, red runs and
sealed pre-fix executables, are retained under
`C:/Temp/opi-final-numbers-20261114/`. Actual archive consumption uses
`C:/Temp/opi-final-numbers-package-20261114/`. All earlier JSON receipts and
sealed artifacts remain historical evidence rather than being rewritten.

Test impact: `add` final numeric decoding and real Agent/session regressions,
frozen caller controls and comparator mutations; `update` comparator numeric
acceptance; `retain` preview/raw/strict-final/terminal evidence; `delete=none`.
Performance-budget, other-platform/MSRV, real Deferred (#94) and complete
six-gate acceptance remain separate.

## Completed verification

The [paired receipt](../evidence/phase5-final-numbers-differential.json) binds
[current materials](../evidence/phase5-final-numbers-materials.json), five sealed
Cargo-reported executables and three replays. Each replay passes all ten finite
numeric controls, actual Rust hook/authorization/tool/event checks, and frozen
Pi tool dispatch/disk recovery. The original 65 Chat checkpoints, 18 preview
closures, 23 interface cases / 124 operations and 1,200 array checkpoints remain
verified. The old derived final-number exception is gone; 27 approved
representation/rejection differences remain, so overall strict alignment is
still false. Cost timings remain observations, not a performance budget.

The [check receipt](../evidence/phase5-final-numbers-checks.json) records exact
commands, logs, applicability and worktree inventories:

- The unfiltered product workspace run (`--workspace --exclude opi-eval
  --all-targets --offline --no-fail-fast`) passed **6,507** top-level tests,
  failed one and ignored five; five isolated child invocations passed.
  The sole failure was an older public-module inventory test missing the
  previously approved `streaming_json` module. Its exact inventory now includes
  that module; both `tool_result_wire` tests pass on focused rebuild/retest.
  The current required check union therefore covers **6,508 passing tests**.
  The original full-run failure is retained, not relabeled as a clean run.
- Workspace Clippy with warnings denied, all 12 doctests, rustdoc with warnings
  denied and formatting pass. Debug symbols are disabled for these host checks;
  incremental compilation and the same persistent shared target are retained.
- All **19 comparator tests** pass, including old-rounding, forged common
  values/raw text and caller/serialization mutations. The public CLI passes
  positive and no-overwrite/old-rounding rejection controls. The actual frozen
  numeric probe also rejects a forged bit value before creating a session.
- [README package flows](../evidence/phase5-final-numbers-readme-package.json)
  and the [independent archive consumer](../evidence/phase5-final-numbers-package-consumer.json)
  pass defaults off/on. Each profile executes **18 tests plus its depth child**.
  The consumer requests plain serde_json 1.0.149 without `float_roundtrip`;
  resolved metadata proves that feature comes from the archived SDK.
  [Payload bindings](../evidence/phase5-final-numbers-package-bindings.json)
  match all 318 SDK and 17 telemetry original files against the workspace.
  SDK archive SHA256 is
  `4807f02a5f00f51702067e885dd682712c8ef10d0fb16e1cffa527244fa50648`.

The module-inventory test repair happened after the sealed numeric pairing.
Of the broad 647-file build input inventory, only this independent test file
changed; all 646 other inputs, including linked production code, paired targets,
fixtures and manifest/lock, remain identical. The final receipt records both
hashes and the successful guard rebuild. No numerical replay dependency was
changed by that test-only repair.

All **224 pre-existing JSON evidence files**, previous package archives and
sealed preview/Chat executables retain their bytes. English/Chinese utility
guides, the existing Unreleased Fixed section and the owning decision were
updated together. README usage remains correct; this iteration issue has no
language mirror. Worktree-only delivery: `committed=none-for-this-task`,
`staged=none`, HEAD unchanged. The SDK manifest is the sole production change;
Agent, provider and persistence source are unchanged.

The shared target has no active leases at handoff, but its disk had only
783,609,856 free bytes after verification. No cache deletion occurred. Prepare
and review a separate incremental-cache maintenance plan before another large
Cargo build; this task does not authorize applying a new eviction plan.
