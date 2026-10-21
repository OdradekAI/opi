# opi-ai testing and acceptance results

[中文](opi-ai-test-acceptance-results.zh.md)

Subsequent evidence closure and directory changes are recorded in the
[follow-up results](opi-ai-test-closure-results.md). Counts below distinguish
top-level cases from two isolated TLS subprocess executions. The old WSL parser
missed one interleaved subprocess result line: its 1331 aggregate is corrected
to 1332 executions (1330 top-level cases plus two subprocess executions).

## Scope and source binding

Execution of the [test and acceptance plan](opi-ai-test-acceptance-plan.md),
2026-10-21. This is verification evidence and scoped test maintenance, not
Phase completion, implementation admission or final SDK distribution acceptance.
No production behavior, public interface, dependency, retained fixture data
file or harness implementation changed in this task. The crypto changes are
inside `cfg(test)`.

Worktree-only baseline: HEAD
`7cb5f0c7b3f3db277bc4e4672b08b324dc26dfbf`; committed=none-for-this-task;
staged=none. Existing ledger, README, WS migration and harness edits were
carried in. `tests/ws_upgrade.rs` was already untracked and was deliberately
extended; the other carried-in implementation files were not edited. The
implementation ledger remains owned by `opi-implement`.

Task-only unstaged changes are the nine tracked test-bearing files listed
below. Carried-in-and-touched inputs are the untracked `ws_upgrade.rs` and plan
pair, with only results links added to the latter. The results pair is task-only
untracked output. `baseline-status.txt` and `final-status.txt` retain the full
inventories separately.

Applied the local matt library's `codebase-design` and `tdd` guidance to the
interfaces already selected in the approved plan: exercise public callers,
retain private mechanism ownership, and use observable counterexamples instead
of adding public seams or speculative abstractions.

## Test impact

| Impact | Change and observable contract |
| --- | --- |
| add | [connection_pooling](../../crates/opi-ai/tests/connection_pooling.rs): two fully consumed H1 calls through one public client require two requests and one accepted socket. The same peer with `max_idle_per_host(0)` requires two sockets. Existing settings/Arc tests remain. |
| add | [ws_upgrade](../../crates/opi-ai/tests/ws_upgrade.rs): one table with missing/wrong/duplicate Upgrade and duplicate selected subprotocol; precise typed rejection, one observed request, EOF and owned peer cleanup. |
| update, add | [crypto private tests](../../crates/opi-ai/src/crypto/mod.rs): strengthen two cancellation tests with a controlled rustls signer around a real admitted key; add cancellation after signer entry with a live receiver, and unpolled-future drop. Entry/release synchronization, call counts, weak ownership and worker join distinguish skipping, retention and suppressed publication. The control releases on unwind and drops its mutex guard before assertions/signing. |
| update | Six existing HTTP text tests in [Anthropic](../../crates/opi-ai/tests/anthropic_fixtures.rs), [Chat](../../crates/opi-ai/tests/openai_chat_fixtures.rs), [Responses](../../crates/opi-ai/tests/openai_responses_fixtures.rs), [Gemini](../../crates/opi-ai/tests/gemini_fixtures.rs), [Mistral](../../crates/opi-ai/tests/mistral_fixtures.rs) and [OpenRouter](../../crates/opi-ai/tests/openrouter_fixtures.rs): require one matching request, no stream errors, successful `Done(Stop)` and consumption to EOF. Anthropic now expects the existing cached system-block wire shape. |
| update | Seven positive [per_request_auth](../../crates/opi-ai/tests/per_request_auth.rs) tests require a matching request. Anthropic also forbids `x-api-key` with Bearer; Codex uses its actual WireApi and a synthetic account ID, with the account header checked. Empty SSE remains intentional auth-only evidence. |
| retain | All other test/fixture ownership, real signature verification, private WS codec batteries, concrete credential-store tests and production callers. |
| delete | none |

Four new test functions and fifteen updated functions. A table row is a
scenario, not a separate Rust test. `.expect(1)` constrains matching requests;
it is not a universal count of all requests received by a server.

## Execution evidence

The raw local evidence directory is
`E:/opi/acceptance/opi-ai-test-acceptance-20261021/`. It retains baseline files,
source hashes, exact commands, exits, selected counts, durations, full logs,
mutations and fresh package scratches. `windows-execution.json` and
`wsl-execution.json` identify each command; each package scratch has its own
`evidence.json`, artifact, consumer lock and features. Transport proofs also
bind native provenance; the early Schema proof binds the regex/URL/data graph.
These local paths are not portable repository artifacts.

| Check | Windows: Rust 1.99.1, x86_64 MSVC | WSL: Rust 1.97.1, x86_64 GNU |
| --- | --- | --- |
| `cargo test --locked --offline -p opi-ai --all-targets` | 1329 top-level + 2 TLS subprocess executions passed, 0 failed/ignored | 1330 top-level + 2 TLS subprocess executions passed, 0 failed/ignored |
| `cargo test --locked --offline -p opi-ai --doc` | 4 passed | 4 passed |
| `cargo clippy --locked --offline -p opi-ai --all-targets -- -D warnings` | exit 0 | exit 0 |
| Final private `--lib crypto::` recheck | 15 passed | 15 passed |
| Ordinary WS graph, included in the crate gate | 93 private + 16 public passed | 93 private + 16 public passed |
| WS `flate2/default` graph | 93 private + 16 public passed | 93 private + 16 public passed |
| WS `flate2/zlib` graph | 93 private + 16 public passed | 93 private + 16 public passed |
| Actual WS package, three graphs | Each: 93 private + 16 public + 6 external checks and clippy; package doc passed | Each: 93 private + 16 public + 6 external checks and clippy; package doc passed |
| Actual crypto package, two graphs | Each: 6 external TLS checks | Each: 6 external TLS checks |
| Actual HTTP package, two graphs | Each: 6 external checks | Each: 6 external checks |
| Actual early Schema package, two graphs | Each: 73 external checks | Each: 73 external checks |
| Harness `unittest discover -s scripts/pi-sdk-conformance -p 'test_package*.py'` | 70 passed | 70 passed |

All 22 platform-runner commands exited zero. `tls_fixtures.rs` is an empty
helper target, not an acceptance battery; all reported acceptance counts are
nonzero. Repository `cargo fmt --check --all`, `python scripts/opi-doc-check.py`
and `git diff --check` passed. No test was ignored.

Every family on a platform used the same `opi-ai-0.8.3.crate` identity:

- Windows SHA-256: `765a41680ea968446945c0a0230a634069450db2f19fd7448ee00830129eed7f`.
- WSL SHA-256: `8e516a5aa058475dc833a111aa539a3a0a37d7f6ffd91ca783bd81a94201b8b0`.

The 142 file payloads are identical across platforms. Archive metadata differs
only in 139 file modes, 0644 on Windows versus 0755 under this WSL mount;
`cross-platform-package-comparison.json` records the comparison. Separate
artifact and consumer-lock identities are preserved in `summary.json` and each
family's evidence; matching file payloads do not collapse the platform runs.

All model-facing scenarios use local fixtures or loopback peers. No live
provider calls or credential activation are part of this run. WSL evidence is
Linux GNU execution under WSL, not a separate physical Linux host. Neither
compiler run establishes exact Rust 1.97.0 support or other target triples.

Windows baseline `cargo test --locked --offline -p opi-ai --all-targets`
passed 1327 tests. Stronger assertions then exposed two pre-existing test
false positives: Anthropic's old string-system matcher received HTTP 404, and
the Codex auth case sent no request because account admission failed. The
corrected fixtures now exercise the intended current contracts.

Three reversible counterexamples produced exit 101 at the intended assertion:
allowing late worker publication; skipping the entry cancellation check; and
returning a valid Upgrade response in the missing-header rejection row.
`mutation-evidence.json` records each source digest and byte-for-byte restoration.
The pool test carries its own positive/negative public configuration control.
No mutation remains in production or test sources. After review corrected the
test helper's mutex-unwind path, the private crypto battery was rerun on both
platforms; the other interfaces and assertions were unchanged.

## Actual adapter consumers

D = public request-body builder; H = loopback HTTP; F = fixture event mapper;
R = actual facade/factory reusing another serializer. These distinguish evidence
types and supported capabilities, rather than imply complete conformance.

| Consumer | Transcript | Replay | Tool schema projection | Auth | Usage |
| --- | --- | --- | --- | --- | --- |
| Anthropic | D chronology collapse/current tools; H initial prompt | D associations, malformed objects, unknown effects, IDs, images | D tool names; full schema values open | H Bearer/API-key selection and absence | F reported/cache/missing; H invalid subsets and partial state |
| OpenAI Chat | D/H initial system; chronology updates open | D reused batches, malformed string arguments, error marker | D strict flags; full parameters open | H lifecycle/prepared/policy | F cache/reasoning/missing/zero; H invalid/partial |
| OpenAI Responses | D/H initial instructions; chronology open | D same/foreign dual IDs, errors and no-leak | D strict flags; full parameters open | H lifecycle/prepared/policy | F usage/cache/reasoning; H invalid subsets |
| Codex | H initial instructions; chronology open | Independent Codex replay assertions open | H `strict:null`; full parameters open | H account/header policy and corrected prepared request | SSE inputs exist; field-value assertions open |
| Gemini | D/H initial system instruction; chronology open | D error Struct/no-leak; malformed-object counterexample open | Full declaration parameters open | H text success and scheme rejection | F usage/cache; H token-value assertions open |
| Bedrock | H initial system; chronology open | D foreign thinking, status and image placeholder; ID/malformed counterexamples open | Full `inputSchema.json` values open | H SigV4, frozen retries and protected headers | H input/output; F cache; reasoning exclusion is tested |
| Azure | H initial system; R Chat | D actual facade error marker; remaining cells R | D facade strict; full parameters open | H deployment/api-version/API key | F usage; remaining cells R Chat |
| Vertex | H initial system; R Gemini | D actual facade error Struct; remaining cells R | Full parameters open; R Gemini | H project/location/Bearer | F usage; remaining cells R Gemini |
| Mistral | H initial system; R Chat | D actual factory error marker; remaining cells R | D profile strict; full parameters open | H text/prepared policy | F usage; remaining cells R Chat |
| OpenRouter | H initial system; R Chat | D actual factory error marker; remaining cells R | D profile strict; full parameters open | H text and identifying headers; policy | F usage; remaining cells R Chat |

The precise anchors are
`transcript_contract::adapters_consume_only_the_normalized_transcript`
(Anthropic only),
`replay_contract::association_matrix_through_real_adapters` (Anthropic/Chat),
`replay_contract::responses_dual_ids_round_trip_and_cross_drops_item_id`
(ordinary Responses), and
`tool_result_wire::tool_result_error_semantics_across_providers` (nine actual
adapters/factories, excluding Codex). Shared Schema compilation, argument
normalization and final-original-schema validation belong to `schema_engine`
and `schema_public`; they do not need ten copies of the same engine tests.

## Remaining acceptance boundaries

- WS has no production codec caller. Models-to-Codex-to-Upgrade-to-codec,
  continuation/pool and complete production cleanup remain open.
- The signing worker has no production JWT/ADC caller. TLS signing and private
  worker results do not close that public combination.
- The open adapter cells above remain evidence gaps. Whole-crate success does
  not establish chronology, schema-value projection or independent Codex
  replay/usage cells not proved by the mapped cases.
- Package proofs cover the four existing harness families only. They do not
  establish final distribution, every module consumer, every platform or a
  registered Phase exit. Native MSVC and WSL GNU are reported separately.
- Concrete persistent-store/product behavior was not changed; its existing
  tests are retained. No new cross-crate runtime change was introduced, so
  full workspace/product/release gates were not selected for this maintenance.

Use the [plan's commands](opi-ai-test-acceptance-plan.md#acceptance-commands)
to rerun the affected union. Future production callers or missing adapter cells
need their own contract-bounded cases; these results do not admit those features.
