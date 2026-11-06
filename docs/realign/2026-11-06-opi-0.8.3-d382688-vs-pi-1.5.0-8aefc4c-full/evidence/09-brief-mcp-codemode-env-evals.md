# Batch MCP/CDM/ENV/EVL — MCP client, codemode, remote env, evals (Dims 15,8,18,20)

(A) pi `packages/mcp` (McpClient, StdioTransport, StreamableHttpTransport,
in-memory testing transport, roots; README + src) vs opi: bounded absence
search (`rg -li 'mcp|model context protocol' crates/*/src`), inspect hits;
verdict likely missing — record searched terms/routes.

(B) pi `packages/codemode` (QuickJS/WASM sandbox: CodemodeSandbox
worker-thread, RemoteCodemodeSandbox Durable Object, InlineCodemodeSandbox,
parseCodemodeSource, `// @options`, tools/ALL_TOOLS/text/image/exit/store/
load, MAX_OUTPUT_*, declarations rendering) vs opi: absence search
(`rg -li 'quickjs|codemode' crates/`), record missing with behavior summary.

(C) pi `packages/env` (pi-env Rust daemon + RemoteExecutionEnv over SSH:
scanHostKey/acceptHostKey/connectSsh, framed protocol in docs/protocol.md,
semantics in docs/semantics.md; daemon source under packages/env/daemon if
present) vs opi `crates/opi-sandbox/src/*` (+ its CLI, platform/, runner/)
and `crates/opi-protocol/src/*`. Decide equivalent/different/opi-only: both
are remote/restricted execution but likely different purposes (pi env =
execute on a remote dev machine; opi sandbox = local restriction SDK/CLI).
Read the pi env protocol docs and daemon source.

(D) pi `packages/evals` (vitest-evals behavioral evals, docker differential
arms, docs-lift evals, report.ts lift computation) vs opi
`crates/opi-eval/src/*` (agent/, benchmark/, bundle/, cli/, comparison.rs,
experiment.rs, process/, regrade.rs, report.rs, runner/, trajectory/,
integrity.rs, native_capture*.rs). Likely substantially different designs:
describe each side's capability set, mark opi-only surface (what opi-eval
does that pi evals does not) and pi-only surface (docker differential,
docs-lift), verdict `different` where both exist.

Output `evidence/mcp-codemode-env-evals.md`, prefixes `MCP`, `CDM`, `ENV`,
`EVL`. Tallies + coverage honesty; exact absence-search commands.
