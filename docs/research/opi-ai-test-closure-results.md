# opi-ai evidence closure and test layout

[中文](opi-ai-test-closure-results.zh.md)

## Scope and source binding

2026-10-21 follow-up to the [first execution](opi-ai-test-acceptance-results.md):
close the mapped evidence gaps reachable through existing interfaces, and make
private versus public-interface tests easier to find. This is ordinary scoped
test/documentation maintenance. No production caller, public seam, dependency,
feature, shared fixture payload or Phase status is introduced or changed. New
inline test inputs distinguish the added behavioral cases.

Worktree-only HEAD is `7cb5f0c7b3f3db277bc4e4672b08b324dc26dfbf`;
committed=none-for-this-task and staged=none. The previous round and unrelated
ledger/README/WS/harness edits were carried in. Baseline files and byte hashes
are retained under `E:/opi/acceptance/opi-ai-test-closure-20261021/baseline/`.
Only task-owned edits were applied on top of those inputs.

Carried-in-and-touched paths are the crate README pair, crypto owner, Gemini
fixture, public WS target, three harness files, and the previous plan/result
pairs. Task-only changes are the HTTP/TLS wiring, Codex/replay/transcript
parents, Vertex fixture and new files described below. Other carried-in edits,
including `.opi-impl-state.json`, are retained. Final index/worktree/new-file
inventories are recorded separately in `final-{staged,unstaged,untracked}.txt`.

## Rust conventions and applied organization

There is no rule requiring private tests to remain inline in `mod.rs`.
`#[cfg(test)] mod tests;` can load a child module with the same private access.
Cargo's root `tests/` targets are separate crates crossing the library's public
interface. A combination of private modules remains internal evidence; one
public function can be an integration-test subject. Official sources and the
current tree are recorded in the bilingual [test organization guide](../../crates/opi-ai/tests/README.md).

Crypto's production owner is now 256 lines instead of 874. Its 15 existing
tests moved to `src/crypto/tests/{admission,signing,worker,jwt}.rs`, with a small
`tests/mod.rs` index. Test attributes and normalized bodies are identical;
the production prefix is byte-identical. The original private fixture and
verification owners remain in place. `crypto-test-layout-proof.json` records
all old/new names and hashes.

The shared TLS constants moved byte-for-byte from root `tests/tls_fixtures.rs`
to `tests/support/tls_fixtures.rs`. HTTP/TLS/WS targets declare that helper as an
internal module. Cargo metadata now discovers 47 integration targets, with no
empty fixture target. New acceptance topics are submodules of existing
contract targets, not additional executables. The package guard and its tests
only synchronize the fixture path; package-local path/include validation was
not weakened.

## Closed evidence cells

| Cell | Actual interface and distinguishing evidence |
| --- | --- |
| Transcript/current tools and complete schema projection | Two [provider_projection](../../crates/opi-ai/tests/transcript_contract/provider_projection.rs) tests call six independent serializers and four actual facades/factories. A shared normalized chronology has system appends, section replacement/removal, and tool removal/redefinition/addition. Assertions cover a literal final prompt, current names/order/descriptions, full nested schema values and canonical immutability. |
| Dedicated Codex replay | Five [Codex replay](../../crates/opi-ai/tests/openai_codex_responses/replay.rs) cases drive the dedicated builder: exact identity versus provider/wire/requested-model/legacy differences, dual IDs, successful versus error results, malformed string arguments, missing-result unknown effects and typed ambiguous-batch rejection. Ordinary Responses is not used as a substitute. |
| Codex typed usage/lifecycle | Two updated HTTP cases and one new table in [openai_codex_responses](../../crates/opi-ai/tests/openai_codex_responses.rs) assert Stop versus Length, exact input/output/cache/reasoning buckets, unknown versus reported zero, unique terminal then EOF, provenance, one HTTP request and unchanged canonical input. |
| Gemini/Bedrock object-wire replay | Two [object_wire](../../crates/opi-ai/tests/replay_contract/object_wire.rs) cases cover malformed JSON/non-object arrays, present/absent parse diagnostics, explicit trusted non-execution facts, exact sanitized context and consumed invalid results. A valid pair in the same batch is retained; error text alone still yields typed Config. Actual roles and call-before-result order are asserted. |
| Bedrock wire IDs | A public builder probe generates an alias that is then supplied as a second canonical ID, producing a real collision without copying a private hash algorithm. Independent assertions require unique legal bounded IDs, synchronized results, preserved valid spelling, role/order, deterministic repeated builds and unchanged canonical input. |
| Gemini and Vertex HTTP usage | Existing [Gemini](../../crates/opi-ai/tests/gemini_fixtures.rs) and [Vertex](../../crates/opi-ai/tests/vertex_fixtures.rs) positive HTTP cases now assert complete typed Usage values and exact request counts, without swallowing errors. |

The projection matrix uses Anthropic, Chat, Responses, Codex, Gemini and
Bedrock, plus Azure, Vertex, Mistral and OpenRouter's actual public constructors.
The last four still reuse their underlying serializers. These are direct
public-builder results, not new HTTP evidence. Cache retention is explicitly
Disabled in that matrix; default strict absence versus Codex `strict:null` is
checked. Native mid-conversation placement and every strict/cache/profile
combination are not inferred from these cases.

Test impact: **add 11** functions, **update 4** existing behavioral tests,
**retain 15** private crypto cases through relocation, **delete 0** test cases.
Assembly/helper paths are updated; one zero-test helper target is removed.

## Verification and counterexamples

Raw commands, source digests, exact exits, selected names/counts and package
evidence are retained at `E:/opi/acceptance/opi-ai-test-closure-20261021/`.
`verified-source-sha256.json`, `windows-execution.json`, `wsl-execution.json`
and each package's `evidence.json` bind the actual inputs and results.

Windows uses Rust 1.99.1 (`x86_64-pc-windows-msvc`); WSL uses Rust 1.97.1
(`x86_64-unknown-linux-gnu`). Existing platform targets remain selected with
incremental compilation enabled.

| Executed check | Windows MSVC | WSL GNU |
| --- | --- | --- |
| `cargo test --locked --offline -p opi-ai --all-targets` | 1340 top-level cases + 2 TLS child executions passed | 1341 top-level cases + 2 TLS child executions passed |
| `cargo test --locked --offline -p opi-ai --doc` | 4 passed | 4 passed |
| `cargo clippy --locked --offline -p opi-ai --all-targets -- -D warnings` | PASS | PASS |
| `python -B -m unittest discover -s scripts/pi-sdk-conformance -p 'test_package*.py'` | 70 passed | 70 passed |
| `package_slice.py --family ws --offline` with fresh scratch/stable target | All three graphs: private 93, public Upgrade 16, external consumer 6 per graph; clippy and package rustdoc PASS | Same counts and gates PASS |
| Extracted SDK `--lib crypto::tests::` | 15 passed | 15 passed |
| Extracted SDK's five public targets: transcript/replay/Codex/Gemini/Vertex | 111 passed (12/21/20/30/28) | 111 passed (12/21/20/30/28) |

All selected commands exited 0; executed Rust cases have zero failed/ignored
tests. Exact argv, scratch/manifest paths and log digests are in the two
execution JSON files. `final-checks.json` records formatting, documentation,
whitespace and final source-binding checks. This is the selected one-crate
maintenance check union, not a workspace/Phase/release gate claim.

WSL has one additional existing Unix-only proxy-environment case. Counts keep
the two isolated TLS child executions separate from top-level cases. The
[previous report](opi-ai-test-acceptance-results.md) also corrects an old WSL
regex aggregation that missed an interleaved child result line;
`historical-count-audit.json` binds that correction to the retained raw logs.

Each current archive contains 152 files with identical payload hashes across
platforms. Archive SHA-256 is
`a6c5059fede7432585b7c0b5712798d5e229350bacc0659d984f0f9d5f854879` on
Windows and `bf7055c9a603b649dbed4ada027b549c98ff21e2ee8d7623e81b1d143ffa5c69`
on WSL. The 149 metadata differences are file modes (`0644` versus `0755`);
`artifact-comparison.json` records the member-by-member comparison. These are
platform-bound evidence artifacts, not a claim of reproducible archive bytes.

Commit preparation on 2026-10-22 normalizes the 119 CRLF line endings in
`tests/support/ws_peer.rs` to LF after the archived runs above. Its decoded
lines are identical; `commit-line-ending-normalization.json` records both
byte hashes. The archive hashes above identify the earlier tested payloads,
not a newly packaged artifact after this whitespace-only adjustment.

Four controlled counterexamples exited 101 at the intended assertion: dropping
nested schema values, treating Bedrock as a string-argument wire, bypassing ID
collision allocation, and changing a Codex fixture's token value. All source
bytes were restored. The collision counterexample was rerun after review
replaced the hardcoded hash alias with a public probe and added role/order
assertions. `counterexamples.json` and `replay-object-wire-review.json` retain
the evidence. No mutation is part of the delivered source.

The harness CLI also rejected an existing scratch with exit 2 before a build.
Package checks require a fresh scratch and the stable marked platform cache;
no `cargo clean`, per-session target, live provider or ambient credentials are
used. Previous crypto/HTTP/Schema consumer proofs remain historical evidence;
this round refreshes the affected WS/package-layout proof and runs the newly
closed public batteries against the actual extracted SDK. It does not claim
every unchanged historical family was rerun against the new archive.

## Remaining production and platform boundaries

These mapped test cells are closed only within the cases above. They do not
prove every possible adapter/profile combination, full SDK distribution or a
Phase exit. Exact Rust 1.97.0, other target triples and native macOS are not
established by MSVC and WSL GNU runs.

Two actual production paths remain absent: Models-to-Codex-to-WS-codec
(registered task 20.16) and JWT/ADC-to-signing-worker/Vertex (20.21.1). The ledger
still reports both failing; tests do not fabricate callers or mark those tasks
passing. The repository [working contract](../../AGENTS.md#skills-and-workflow)
reserves implementation admission/ledger execution for explicit `opi-implement`
invocation. That production work requires its own task/dependency gates; it is
not silently included in this test-maintenance closure.
