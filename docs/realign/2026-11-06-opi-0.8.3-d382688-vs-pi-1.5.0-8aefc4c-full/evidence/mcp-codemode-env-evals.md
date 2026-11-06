# Batch MCP/CDM/ENV/EVL — MCP client, codemode, remote env, evals (Dims 15, 8, 18, 20)

Measured 2026-11-06 against inputs of `00-inputs.md` (pi 1.5.0 export `8aefc4c29d420c27`, Opi tree `d382688`).
Batches measured: pi `packages/mcp`, `packages/codemode`, `packages/env`, `packages/evals` vs Opi `crates/*` (bounded absence searches + `opi-sandbox`, `opi-protocol`, `opi-eval` inspection).
Tallies (25 records): equivalent 0, partial 1, missing 17, different 5, opi-only 2, unresolved 0.
Biggest gaps: no MCP client of any kind (no JSON-RPC/stdio/Streamable-HTTP/OAuth), no QuickJS codemode sandbox, no remote/SSH execution and no fs/watch/pty surface, and no live-model behavioral/docs-lift evals.
Uncertainties: pi export provenance unverified (per `00-inputs.md`); `rg` unavailable in this environment, GNU `grep` equivalents used (exact commands recorded per record).

## MCP — pi `packages/mcp` vs Opi

Absence search (all of Opi `crates/*/src`, plus repo-wide sweep):
`grep -rliE 'mcp|model context protocol' crates/*/src` → 5 files, all incidental:
`opi-ai/src/auth/oauth/anthropic.rs:31` (OAuth scope string `user:mcp_servers`), `opi-ai/src/crypto/fixtures.rs` and `opi-ai/src/tls/node_trust.rs` (base64 substring "MCP"), `opi-eval/src/benchmark/task_lifecycle.rs:402,591` (pinned third-party Harbor task TOML fixture field `mcp_servers = []`), `opi-eval/src/native_capture.rs` (pinned pi-1.0.4 session-stream `sections.mcp_servers`/`toolsAdded` parsing, i.e. consuming pi's own recorded output, no client). Repo-wide `grep -rliE 'quickjs|codemode|model.?context.?protocol'` on `crates/` → no hits. Explicit non-goal statements: `crates/opi-agent/README.md:489` ("No in-core plan mode, sub-agent, todo, permission popup, or MCP runtime") and `crates/opi-coding-agent/README.md:696` (MCP workflows are "examples/package patterns, not built-in core workflows").

### MCP-01 — McpClient transport-neutral client core
- Dimension(s): 15
- pi entry: `packages/mcp/src/client.ts:155` (public; npm `@earendil-works/pi-mcp`)
- Opi entry: bounded absence search above; no Opi module correlates request IDs, initialization, timeouts, cancellation, or server-request dispatch for MCP
- Comparison: missing
- Observable difference: pi clients `connect(transport)` (initialize + `notifications/initialized`, `client.ts:204-243`), then `ping`/`listTools` (paginated, dup-cursor and 1,000-page guard `client.ts:38-39,360-375`)/`listResources(Page)`/`listResourceTemplates(Page)`/`readResource`/`callTool`, with 30 s default request timeout (`client.ts:38`), AbortSignal→`notifications/cancelled`, progress notifications that renew the timeout (`client.ts:539-555`), server→client `ping` and `roots/list` handlers (`client.ts:176-180`), typed errors (`McpError/McpTimeoutError/McpAbortError/McpConnectionClosedError`). An Opi caller has no API to talk to an MCP server at all.
- Evidence basis: source inspection
- Review outcome: self-reviewed
- Notes: pi protocol surface pinned to `LATEST_PROTOCOL_VERSION = "2025-11-25"` negotiating `2025-06-18`/`2025-03-26`/`2024-11-05` (`packages/mcp/src/protocol/types.ts:4,9`).

### MCP-02 — StdioTransport
- Dimension(s): 15
- pi entry: `packages/mcp/src/transports/stdio.ts:69` (public)
- Opi entry: absence search above; `crates/opi-ai/src/tls/node_trust.rs` hits are base64 noise. No subprocess JSON-RPC framing in Opi
- Comparison: missing
- Observable difference: pi spawns a server via `cross-spawn` (own process group, `stdio.ts:96-101`), newline-delimited JSON with `maxMessageBytes` guard and 64 KiB stderr ring (`stdio.ts:145-175,206-216`), spec shutdown close-stdin→SIGTERM(500 ms grace)→SIGKILL(2 s) applied to the whole process group including Windows `taskkill /T /F` and an exit hook that SIGTERMs live groups (`stdio.ts:8,17-46,158-179`). Opi spawns no MCP server process.
- Evidence basis: source inspection
- Review outcome: self-reviewed

### MCP-03 — StreamableHttpTransport (sessions, GET stream, resumption)
- Dimension(s): 15
- pi entry: `packages/mcp/src/transports/streamable-http.ts:194` (public; exports `McpHttpError`, `McpAuthRequiredError` 401/403, `McpSessionExpiredError` at `:132-160`)
- Opi entry: absence search above; Opi's `opi-ai` HTTP layer is provider LLM APIs only, no MCP URL scheme
- Comparison: missing
- Observable difference: pi POSTs JSON-RPC accepting `application/json` or `text/event-stream` (`:252-274`), captures `mcp-session-id`, opens the server→client GET SSE stream after `notifications/initialized` (`:266-268,429-456`), reconnects with backoff (1 s initial/30 s max/5 retries, honoring SSE `retry`) and resumes dropped response streams via `Last-Event-ID` (`:380-407,458-462,490-497`), DELETEs the session on close with a 1 s timeout and the last token (`:269-281`), single shared 401/403 recovery through `authProvider.onUnauthorized` (`:299-319`). Opi cannot reach an HTTP MCP server.
- Evidence basis: source inspection
- Review outcome: self-reviewed

### MCP-04 — MCP OAuth client subset
- Dimension(s): 15
- pi entry: `packages/mcp/src/oauth/` — `flow.ts:171,249,280,296,317,433,455` (authorize, dynamic client registration, code exchange, refresh, `stepUpScope`, `authorizeMcp`, `adaptOAuthProvider`), `discovery.ts:39-106` (WWW-Authenticate parse, protected-resource + authorization-server metadata discovery), `provider.ts:53` (`McpOAuthProvider` + `MemoryOAuthStateStore`/`McpOAuthStateStore` seam), `callback.ts:34` (`OAuthCallbackServer.listen()` loopback catcher), `errors.ts` (5 typed errors) (public)
- Opi entry: absence search; Opi's OAuth (`crates/opi-ai/src/auth/oauth/anthropic.rs`, `crates/opi-coding-agent/src/oauth.rs`) is provider-login OAuth, has no MCP protected-resource discovery, no dynamic client registration, no PKCE-code flow against arbitrary MCP authorization servers
- Comparison: missing
- Observable difference: pi can authenticate to OAuth-protected MCP servers end-to-end (discovery → PKCE + DCR → refresh shared by concurrent 401s → step-up `insufficient_scope`), with durable credential storage injectable via `McpOAuthStateStore`. Opi cannot authenticate to any MCP server.
- Evidence basis: source inspection
- Review outcome: self-reviewed
- Notes: adapted from MCP TS SDK v1.29.0 (MIT), license kept (`README.md:127-129`, `LICENSES/`).

### MCP-05 — protocol types, `toLlmContent`, in-memory testing transport
- Dimension(s): 15
- pi entry: `packages/mcp/src/protocol/content.ts:87-118` (`toLlmContent`: text/images pass through, embedded resources unwrapped, audio/resource-link/binary → placeholders, empty content + `structuredContent` → JSON), `protocol/types.ts`, `jsonrpc.ts` (`parseJsonRpcMessage` + guards), `testing/index.ts` → `transports/in-memory.ts:29-51` (`createInMemoryTransportPair`) (public)
- Opi entry: absence search; no Opi type models MCP content blocks or converts them to model content
- Comparison: missing
- Observable difference: pi users can wrap MCP tools as `pi-agent-core` `AgentTool`s (README recipe `README.md:47-66`: `mcp_` name prefix, 64-char `[A-Za-z0-9_-]` sanitization, object-schema enforcement) and test adapters without I/O. Opi exposes no equivalent conversion or test transport.
- Evidence basis: source inspection
- Review outcome: self-reviewed

### MCP-06 — Opi MCP-adapter example (extension-API demonstration)
- Dimension(s): 15, 20
- pi entry: none of this form; nearest pi material is the real `packages/mcp` package plus `packages/agent/examples/mcp-codemode`
- Opi entry: `crates/opi-coding-agent/tests/mcp_adapter_example.rs:1-20` (test-only, fixture data; header states it "does not introduce any MCP protocol or transport into the core runtime", 966 lines)
- Comparison: opi-only
- Observable difference: Opi ships a test demonstrating `mcp/list_tools`, `mcp/call_tool`, `mcp/list_resources`, `mcp/get_resource` dispatch through `Extension::on_command` with fixture tools/resources (`mcp_adapter_example.rs:197-200,305-455`); it performs no MCP wire protocol, no transport, no live server. It is documentation-as-test of the extension seam, not an MCP client.
- Evidence basis: test inspection
- Review outcome: self-reviewed
- Notes: under the reproduction lens this Opi artifact does not reproduce any `packages/mcp` behavior; it is recorded so pass 2 does not mistake it for MCP support.

## CDM — pi `packages/codemode` vs Opi

Absence search: `grep -rliE 'quickjs|codemode' crates/` → no hits (exit 1). Repo-wide sweep (excluding `.pi`, `.git`, `node_modules`, `target`) hits only docs/plan/audit files outside this batch's authority. Opi has no script-sandbox capability of any kind: no JS interpreter/WASM VM integration (workspace Cargo manifests carry no `quickjs`/`rquickjs`/`wasmtime` dependency), no nested-tool-call executor.

### CDM-01 — CodemodeSandbox (worker-thread QuickJS/WASM sandbox)
- Dimension(s): 15, 20
- pi entry: `packages/codemode/src/runtime/execution.ts:460` (`CodemodeSandboxBase`), `CodemodeSandbox` via `src/index.ts`; worker `src/runtime/worker.ts`, WASM loader `src/wasm.ts` (`loadQuickJSWasm`), VM glue `src/runtime/vm.ts` (public)
- Opi entry: absence search above; `crates/opi-sandbox` restricts OS process trees, not model-written JS; nothing interprets scripts
- Comparison: missing
- Observable difference: pi runs model-written JS in a fresh QuickJS/wasm VM per `execute()` (~20 ms, `README.md` "How it works"), with only injected tool calls as capability (no timers/fetch/process/require/modules/WebAssembly), 60 s-style `timeoutMs` deadlines, `memoryLimitBytes` heap caps, abort/timeout/close → interrupt flag + `worker.terminate()` (`execution.ts:392,501,569`), `onWorkerStarted`/`acquireWorker` admission. Opi callers cannot run model-written scripts.
- Evidence basis: source inspection
- Review outcome: self-reviewed
- Notes: pi's coding agent ships this as its built-in `codemode` tool (README:5-7).

### CDM-02 — RemoteCodemodeSandbox (Durable Object) and InlineCodemodeSandbox
- Dimension(s): 15
- pi entry: `packages/codemode/src/runtime/remote.ts:273` (`RemoteCodemodeSandbox`), `src/runtime/inline.ts:122` (`InlineCodemodeSandbox`, `DEFAULT_INTERRUPT_BUDGET = 100_000` at `:39`), `/portable` entry `src/portable.ts`, deployable DO `src/cloudflare.ts` + `examples/cloudflare/` (public)
- Opi entry: absence search above
- Comparison: missing
- Observable difference: pi offers the same script API with the VM in a separately deployed Cloudflare Durable Object (interrupt budget capped server-side, `remote.ts:109`), or inline on the calling thread with a reduced stack guard (`INLINE_MAX_STACK_SIZE`). Opi has no remote or inline sandboxed scripting.
- Evidence basis: source inspection
- Review outcome: self-reviewed

### CDM-03 — parseCodemodeSource, `// @options:`, source grammar
- Dimension(s): 15
- pi entry: `packages/codemode/src/source.ts:11` (`CODEMODE_OPTIONS_PREFIX`), `:22` (`CODEMODE_SOURCE_GRAMMAR` Lark grammar), `:101` (`parseCodemodeSource`), `:45` (`CodemodeSourceError`); `timeout_ms` bounded to `MAX_TIMEOUT_MS = 2_147_483_647` (`:16,86-89`) (public)
- Opi entry: absence search above
- Comparison: missing
- Observable difference: pi scripts may carry `{"max_output_tokens", "timeout_ms"}` on line 1; the options line is replaced by an empty line to preserve stack-trace line numbers; empty input, invalid JSON, unknown fields, misplaced options lines, and code-less options lines all throw `CodemodeSourceError`; a Lark grammar is exported for grammar-constrained tool input. Opi has no counterpart parser or grammar.
- Evidence basis: source inspection
- Review outcome: self-reviewed

### CDM-04 — Script environment (tools/ALL_TOOLS/text/image/exit/store/load) and output/store limits
- Dimension(s): 15, 20
- pi entry: `packages/codemode/src/runtime/prelude-source.ts:34-43` (`MAX_STORE_VALUE_CHARS = 256 Ki`, `MAX_STORE_TOTAL_CHARS = 1 Mi`, `MAX_OUTPUT_CHARS = 16 Mi`, `MAX_OUTPUT_ITEMS = 100_000`), tool-name identifier mapping `src/identifier.ts:5` (`toCodemodeIdentifier`), environment documented `README.md:25-42` (public behavior)
- Opi entry: absence search above
- Comparison: missing
- Observable difference: pi scripts get `tools.<name>(args)` (JSON round trip, same-message Error rejection), `ALL_TOOLS`, `text()`/`image()` output builders (remote image URLs rejected), `exit()`, synchronous `store`/`load` with per-value and total caps failing as in-script `RangeError`, host `globals` with `spread`/`signature` options; `result.calls` records every nested call with `ok|error|cancelled` and un-awaited calls are aborted. Opi offers no nested-call environment, so no observable equivalent.
- Evidence basis: source inspection
- Review outcome: self-reviewed

### CDM-05 — declarations rendering for models
- Dimension(s): 15
- pi entry: `packages/codemode/src/declarations.ts:45` (`renderDeclarations`), `:76,91,100` (`renderToolSignature`/`renderToolSample`/`renderToolOutputType`), `:139` (`schemaToType`, local `$ref` expansion capped at 32, recursive/remote refs → `unknown`, tag truncation at 80 chars, `DEFAULT_INPUT_SCHEMA_MAX_CHARS = 16_000`) (public)
- Opi entry: absence search above; Opi tool declarations for prompts are hand-written per tool, no JSON-Schema→TypeScript renderer exists
- Comparison: missing
- Observable difference: pi turns tool `inputSchema`/`outputSchema` into model-facing TypeScript declarations with property-description comments and constraint tags (`// @integer @minimum 1`). Opi has no schema-to-declaration renderer.
- Evidence basis: source inspection
- Review outcome: self-reviewed

### CDM-06 — deterministic `start()` runs
- Dimension(s): 15
- pi entry: `packages/codemode/src/runtime/execution.ts:530` (`start(code, options)` → `CodemodeRun`), options `src/types.ts:206-243`, determinism rules `README.md:321-348` (public)
- Opi entry: absence search above
- Comparison: missing
- Observable difference: pi pins `Date.now()`/`performance.now()` to `startedAt`, seeds all randomness, forces UTC, drops `WeakRef`/`FinalizationRegistry`, and applies host-answered results strictly in result-number order, enabling restart-and-resume replay with a fingerprint (`onStarted`). Opi has no deterministic script replay.
- Evidence basis: source inspection
- Review outcome: self-reviewed

## ENV — pi `packages/env` vs `opi-sandbox` + `opi-protocol`

### ENV-01 — daemon wire protocol vs `command-execution-jsonl-v1`
- Dimension(s): 18
- pi entry: `packages/env/docs/protocol.md:5-40` (framed binary protocol v1: `u32 len/u8 type/u32 id/u32 jsonLen/json/bytes`, 16 MiB frame cap, 6 frame types, token sync line `PI-ENV <token>`), client `src/connection.ts:204` (`Connection`, `MAX_FRAME = 16 MiB` at `:13`, `hello` at `:340,382-398`); daemon `daemon/src/main.rs:32-49` (`PROTOCOL = 1`, `FEATURES = ["listFiles"]`, 5 s ping / 30 s silence kill, 16 fs worker threads, 4096-handle `EMFILE` cap, 64 KiB control-payload priority) (public + internal daemon tooling)
- Opi entry: `crates/opi-protocol/src/execution/v1/` (`frames.rs`, `codec.rs`, `session.rs`, `bounds.rs`; wire identity `command-execution-jsonl-v1`), backend process model `crates/opi-sandbox/README.md:103-110` (`opi-sandbox backend --stdio`), consumer `crates/opi-coding-agent/src/execution/protocol_host.rs` (opi-coding-agent depends on `opi-protocol`, `Cargo.toml:29`)
- Comparison: different
- Observable difference: both speak a versioned stdio protocol to a child process, but pi-env is a persistent multi-request daemon (request ids, cancel frames, event streams for exec/pty/watch output, liveness pings, handle table, op-named file operations) while opi-protocol is one JSONL execution per backend process (`initialize/ready/execute/accepted/started/stdout|stderr|diagnostic/completed|failed`, one-execution limit stated in `opi-sandbox/README.md:107-108`), closed frame enums with duplicate-frame and cumulative-output checks, no liveness ping, no file operations, no event frames mid-request other than output/diagnostic. Neither is a reproduction of the other; purposes differ (remote dev-machine env vs restricted local command execution).
- Evidence basis: source inspection
- Review outcome: self-reviewed

### ENV-02 — SSH remote execution, host-key trust, daemon deployment
- Dimension(s): 18
- pi entry: `packages/env/src/ssh.ts:182` (`scanHostKey`), `:258` (`acceptHostKey`), `:282` (`forgetHostKey`), `:329` (`detectPlatform`), `:405` (`deployDaemon`, SHA-256-named binary verified before every start), `:519` (`sshConnection` lazy), `:532` (`connectSsh`); errors `HostKeyUnknownError`/`HostKeyChangedError`/`SshError` `:40-76`; `ssh` hardened argv `BatchMode`, app-owned known_hosts under fixed alias, no forwarding (`README.md:28-33`) (public)
- Opi entry: bounded absence search `grep -rniE '\bssh\b|known_hosts' crates/ --include='*.rs' --include='*.md' -l` → only unrelated mentions (`opi-ai/src/auth/oauth/anthropic.rs:33` loopback note, `opi-coding-agent/src/credential_store.rs:296` headless-SSH comment, `package_store.rs:109` git-source syntax, `opi-eval/src/runner/prepare.rs:813` fixture string). `opi-sandbox` executes locally only (`README.md:10-12` "supervising and restricting a single command process tree"; `:140-142` explicitly "not a container, VM, remote executor")
- Comparison: missing
- Observable difference: pi can run an agent's tools on another machine over SSH after an explicit out-of-band host-key acceptance flow, deploying a platform-matched daemon (Linux/macOS/Android-Termux/Windows, x86-64/arm64, `README.md:58`). Opi has no remote execution, host-key management, or daemon deployment of any kind.
- Evidence basis: source inspection
- Review outcome: self-reviewed

### ENV-03 — remote filesystem operation surface + listFiles
- Dimension(s): 18
- pi entry: `packages/env/docs/protocol.md:80-105` (ops `lstat realpath write writeChunk truncate fsync rename mkdir rm mkdtemp open pread fstat scanLines opendir readdir close`; `listFiles` with `hidden/gitignore/exclude/maxEntries/maxBytes/timeoutMs`, defaults 50 000 entries / 4 MiB / 10 s, kind-byte payload records, depth-first byte-order), semantics `docs/semantics.md:20-45`; implementation `daemon/src/fs.rs`, `daemon/src/list.rs`, `daemon/src/scan.rs`; client surface `src/remote-env.ts:354` (`RemoteExecutionEnv implements ExecutionEnv`), `src/list-files.ts`
- Opi entry: bounded absence: `grep -rniE 'lstat|mkdtemp|opendir|readdir|scanLines|listFiles' crates/opi-sandbox crates/opi-protocol` → only incidental hits (`crates/opi-sandbox/src/platform/linux.rs:194,201` Landlock comment removing `AccessFs::ReadDir` for confined children; `crates/opi-sandbox/tests/fixtures/protocol_client.py:161` Python fixture's `tempfile.mkdtemp`); opi-protocol frames carry only command execution (`crates/opi-protocol/src/execution/v1/frames.rs`), and the sandbox SDK exposes no file-operation API
- Comparison: missing
- Observable difference: pi agents can read/write/list remote file trees with Node-compatible semantics (`EEXIST`/`ERR_FS_EISDIR` rules, non-UTF-8 name handling, nonblocking readers, gitignore-aware bounded listing). Opi executes one command per backend process with no file operations over its protocol; its `SandboxRequest` has no file API.
- Evidence basis: source inspection
- Review outcome: self-reviewed

### ENV-04 — command execution and supervision semantics
- Dimension(s): 18
- pi entry: `packages/env/docs/protocol.md:107-110` (exec: shell string or argv; per-command new session/process group; `timeout`/`aborted` codes carrying `spillPath`; output windowing `{maxBytes,maxLines,minIntervalMs,bytesPerSecond}` with `skipped` coalescing), `docs/semantics.md:47-63` (shell fallback bash→sh, `shell_unavailable`, `spawn_error`, env layering, SIGKILL group on timeout/abort, exit 128+signal, drain 100 ms after exit, spill files `pi-output-<uuid>.log`), `daemon/src/exec.rs`, `daemon/src/window.rs`
- Opi entry: `crates/opi-sandbox/src/runner.rs:185` (`SandboxRequest`: program/args/workspace/cwd/timeout/env/stdin), `:293` (`SandboxOutcome`), `:349` (`SandboxEvent::Started/Output/Diagnostic/Completed`), `:482` (`SandboxRunner::run`), process-tree termination `src/process_tree.rs` (`TreeGuard`), CLI exit mapping `README.md:74-81` (target exit unchanged, `128 + signal`, timeout `124`, cancel `130`), 1 MiB per-stream result preview with truncation flag (`README.md:131-135`)
- Comparison: partial
- Observable difference: shared core exists — one supervised child, timeout, cooperative cancellation, whole-tree termination, `128 + signal` mapping, streamed output, host env inheritance options. Opi differs observably: argv only, no implicit shell string (`README.md:67-68`); no output windowing/pacing or `skipped` accounting; no spill-to-disk for oversized output (bounded 1 MiB preview instead); no per-command controlling session/PTY; and Opi adds a restriction contract pi-env lacks (see ENV-07). pi reports errors as code-carrying protocol errors; Opi as typed outcomes/exit codes.
- Evidence basis: source inspection
- Review outcome: self-reviewed

### ENV-05 — file watching
- Dimension(s): 18
- pi entry: `packages/env/docs/protocol.md:112-115` (`watch` op: targets/exclude, `ready`/`change {paths}`/`overflow`/`error` events, request lasts until cancelled), `docs/semantics.md:65-70`, `daemon/src/watch.rs:1-33` (port of Durable's `NodeFileWatcher`: inotify/FSEvents trigger debounced rescans, snapshot diffing, polling fallback on Windows/network/FUSE)
- Opi entry: absence: `grep -rniE 'inotify|fsevents|file.?watch' crates/opi-sandbox crates/opi-protocol` → no hits; no watch frames in `opi-protocol/src/execution/v1/frames.rs`
- Comparison: missing
- Observable difference: pi agents get change events from the remote machine with native/polling modes and overflow reporting. Opi has no file-watching capability.
- Evidence basis: source inspection
- Review outcome: self-reviewed

### ENV-06 — PTY terminals
- Dimension(s): 18
- pi entry: `packages/env/docs/protocol.md:117-125` (`pty` op: program in new pseudo-terminal as session leader, login shell default, `started {pid}`/raw output events, `ptyWrite`/`ptyResize`/`ptyForeground` including foreground-process-group reporting; `ENOTSUP` on Windows), `daemon/src/pty.rs` (581 lines), client `src/terminal.ts` (`openTerminal`), `README.md:52-56`
- Opi entry: absence: `grep -rniE '\bpty\b|pseudo.?terminal' crates/opi-sandbox crates/opi-protocol` → no hits (opi-tui renders terminals but spawns none)
- Comparison: missing
- Observable difference: pi can run interactive programs (editors, shells) on the remote machine with raw byte I/O, resize, and foreground-job detection. Opi cannot run any program under a PTY.
- Evidence basis: source inspection
- Review outcome: self-reviewed

### ENV-07 — OS-level restriction mechanisms (opi surface)
- Dimension(s): 18
- pi entry: none — pi-env deliberately applies no confinement; remote commands run with the SSH user's identity (`packages/env/README.md:1-3` frames it as "remote execution environments", not sandboxing)
- Opi entry: `crates/opi-sandbox/src/policy.rs:32-110` (`Profile::WorkspaceWrite`, `NetworkPolicy`, `Mechanism::{Landlock,Seatbelt,None}`, `ContractStatus::{Restricted,Unrestricted}`, `NoRestriction` at `:262`), Linux Landlock+seccomp `src/platform/linux.rs`, macOS Seatbelt `src/platform/macos.rs`, fail-closed posture and `doctor` CLI (`README.md:86-101,143-147`), backend selection after host probe (`README.md:103-112`)
- Comparison: opi-only
- Observable difference: Opi restricts filesystem mutation to workspace+temp roots and can deny network (AF_UNIX stays available) under the target's own OS identity; pi-env exposes nothing comparable. This is an Opi capability with no pi-env counterpart, not a reproduction.
- Evidence basis: source inspection
- Review outcome: self-reviewed

### ENV-08 — output pacing, spill, and liveness scheduling
- Dimension(s): 18
- pi entry: `packages/env/docs/protocol.md:36-46` (cancel registration before run; `ptyWrite`/`ptyResize` answered in arrival order; results/errors/pings ahead of queued output above 64 KiB; 4 MiB unsent-output backpressure), `:107-110` spill thresholds, `daemon/src/window.rs`, `daemon/src/output.rs`
- Opi entry: `crates/opi-sandbox/src/runner.rs:67` (bounded output channel), result preview caps `README.md:131-135`; no pacing, no spill, no ping
- Comparison: missing
- Observable difference: on slow links pi keeps control frames responsive while throttling bulk data and can spill full raw output to disk; opi-protocol has no mid-execution pacing classes and no spill artifact. Folded here rather than into ENV-04 because it is transport-behavior, not exec semantics.
- Evidence basis: source inspection
- Review outcome: self-reviewed

## EVL — pi `packages/evals` vs `crates/opi-eval`

### EVL-01 — overall design comparison
- Dimension(s): 20
- pi entry: `packages/evals/README.md:1-16` (behavioral evals for Pi's coding agent built on `vitest-evals`; runner in `src/{cli,docker,plan,report,harness}.ts`; suites in `evals/*.eval.ts`; images in `docker/`) (public tooling)
- Opi entry: `crates/opi-eval/src/lib.rs:1-15` ("Unpublished Independent Companion for cross-agent evaluation experiments", agent-neutral, depends on no Opi crate), CLI `src/main.rs:20-186` (validate/run/verify/regrade/report/plan/doctor/prepare/conformance + hidden native-capture modes), modules agent/ benchmark/ bundle/ comparison/ experiment/ integrity/ process/ regrade/ report/ runner/ trajectory/ native_capture* (43,264 LOC)
- Comparison: different
- Observable difference: pi evals measures one product (pi coding agent) against live models with vitest-evals judges and Docker image variants; opi-eval runs frozen cross-agent experiments (Opi and pi as subject adapters, `src/agent/{opi,pi}.rs`) against pinned benchmark datasets (Terminal-Bench 2.1/3.0, DeepSWE, `src/benchmark/`), with sealed evidence bundles, integrity admission, offline regrade/report, and deterministic conformance (scripted provider, never a paid provider — `src/cli/conformance.rs:16-18`). Both exist; neither reproduces the other.
- Evidence basis: source inspection
- Review outcome: self-reviewed

### EVL-02 — Docker differential arms
- Dimension(s): 20
- pi entry: `packages/evals/src/docker.ts:38` (`buildImages`), `:126` (`createDockerContext`), `Dockerfile` + `entrypoint.ts` + `install-runtime.mjs` (two images `without_docs`/`with_docs` from one staged runtime; root-owned unreadable eval fixtures after permanent UID drop; per-run fresh home/agent-dir/workspace/session-dir; `README.md:47-70`)
- Opi entry: absence: `grep -rn 'without_docs\|with_docs\|image variant' crates/opi-eval/src` → no hits; opi-eval's Docker use is benchmark task environments via the `docker` client (`DOCKER_HOST` probing, `src/runner/prepare.rs:126,436`; task `docker_image` fields `src/benchmark/process.rs:165,358`)
- Comparison: missing
- Observable difference: pi can measure the same cohort in two product-build variants differing only by shipped documentation. Opi has no image-variant/differential-build mechanism of any kind; its docker usage is task execution environments for benchmark adapters.
- Evidence basis: source inspection
- Review outcome: self-reviewed

### EVL-03 — docs-lift evals and lift computation
- Dimension(s): 20
- pi entry: `packages/evals/evals/{custom-provider,extensions,models,openai-provider,prompt-templates,tui}.docs.eval.ts` (6 docs eval suites; `tui.docs.eval.ts:277` lines incl. Levenshtein-similarity UI checks), `src/report.ts:360-412` (`summarizeEvalObservations`: exact-arm pairing, blocked pairs withhold headline, pass-rate lift = treatment−control, flags `no-lift|negative-delta|control-saturated|treatment-saturated|flaky`, paired token/tool-call/ms/cost metrics), report schema v3 `:75-90`
- Opi entry: absence: `grep -n 'lift\|Lift' crates/opi-eval/src -ri` → no hits; Opi report computes normalized graded outcomes and pair coverage from sealed evidence (`src/report.rs:1-20`), not a two-arm documentation lift
- Comparison: missing
- Observable difference: pi produces a signed lift number per eval set with blocked-pair withholding and stability flags. Opi produces headline outcomes only from admitted grader-sourced native report artifacts with per-headline provenance (`src/report.rs:11-16`) — different statistic, different evidence model.
- Evidence basis: source inspection
- Review outcome: self-reviewed

### EVL-04 — live-model behavioral harness (vitest-evals)
- Dimension(s): 20
- pi entry: `packages/evals/src/harness.ts:471-483` (`createPiCodingAgentHarness`: in-process `AgentSession` via `createAgentSessionFromServices`, isolated env `applyIsolatedEnvironment:82`, root→unprivileged UID tool sandbox `enterToolSandbox:170-186`), `:485-537` (`DOCUMENTATION_EVAL_TOOLS = read/write/edit/grep/find/ls`, `createPiDocumentationEvalHarness` requiring `PI_EVAL_CONTAINER=1`), suites `evals/smoke.eval.ts` (needs `PI_PROVIDER`/`PI_MODEL`), `evals/documentation-audit.eval.ts` (structured-output verdict tool over every docs page), fixtures `evals/acme-server.ts` (local HTTP provider stand-in), `evals/configured-runtime.ts`
- Opi entry: nearest surfaces are `crates/opi-eval/src/cli/conformance.rs` (deterministic pinned conformance matrices with `scripts/scripted-provider.py`; explicitly "never calls a paid provider", `:16-18`) and `opi_ai::test_support::MockProvider` used by workspace tests; no live-model behavioral eval suite exists
- Comparison: missing
- Observable difference: pi can measure model behavior on tasks like "capital of France" or docs-page audits with real providers and judge grading. Opi's evaluation is hermetic/deterministic by design; there is no live-LLM behavioral eval entry point.
- Evidence basis: source inspection
- Review outcome: self-reviewed

### EVL-05 — opi-eval-only experiment machinery
- Dimension(s): 20
- pi entry: none — pi evals has no frozen-contract, sealing, or regrade machinery (`src/` is 1,447 lines total: cli/docker/plan/report/harness)
- Opi entry: `crates/opi-eval/src/experiment.rs:487` (`ResolvedExperiment` frozen contract; controls/edges/projections), `src/integrity.rs:1-10` (digest-addressed benchmark revision admission, `IntegrityRecord::review` reviewer-evidence boundary), `src/bundle/mod.rs:1-6` (sealed trial bundles, intent-before-effect persistence, mutation rejection), `src/regrade.rs:1-19` (`verify` byte/seal walk + native offline regrader), `src/report.rs` (offline normalized report from sealed outputs), `src/trajectory/mod.rs:1-24` (causal-span trajectory projection with `Fact::Known/Unknown` provenance), `src/native_capture.rs` (bounded privacy filter: private thinking never reaches capture sinks), `src/comparison.rs:1-18` (baseline/candidate pairing only when control fingerprints agree; `NonComparability` stays in the denominator)
- Comparison: opi-only
- Observable difference: Opi provides reproducible, tamper-evident cross-agent measurement with offline recomputation and privacy filtering — capabilities pi's evals package does not attempt.
- Evidence basis: source inspection
- Review outcome: self-reviewed

### EVL-06 — paired comparison/pairing logic (both exist)
- Dimension(s): 20
- pi entry: `packages/evals/src/report.ts:250-268` (`resolvePair`: exact `(case, variant, model, run)` arm identity; missing/duplicate/skipped/pending/unscored/errored arms block the pair), `src/plan.ts:39` (`createTaskPlan`, repetition order alternated by run number)
- Opi entry: `crates/opi-eval/src/comparison.rs` `TrialFact` (`id/subject/task/group/manifest_digest/control_fingerprint`, `:18-40`) and one baseline/candidate pair per edge-task-group with typed `NonComparability` states kept visible
- Comparison: different
- Observable difference: pi pairs product-build variants of the same case and withholds the headline when any pair is blocked; Opi pairs subject arms under matching control fingerprints inside a frozen manifest and keeps every declared pair visible with its exact state. Same family of concern (pairing discipline), different axes (docs variant vs agent subject), different failure presentation.
- Evidence basis: source inspection
- Review outcome: self-reviewed

### EVL-07 — Docker usage (both exist)
- Dimension(s): 20
- pi entry: `packages/evals/src/docker.ts` + `docker/Dockerfile` (builds two product images from the repo, ephemeral mount, workspace tarball packing via consumer-install machinery)
- Opi entry: `crates/opi-eval/src/runner/prepare.rs:126,436` (docker client/`DOCKER_HOST` reachability probe, `docker info` resource observation), benchmark task containers (`src/benchmark/process.rs:165,358` `docker_image`; Terminal-Bench/DeepSWE adapters materialize official task packages via pinned external locks)
- Comparison: different
- Observable difference: pi uses Docker to build product variants under measurement; Opi uses Docker only as the task-environment engine for pinned benchmark datasets and as an observed resource in `doctor`. No shared observable behavior.
- Evidence basis: source inspection
- Review outcome: self-reviewed

## Layer H — active design material encountered in scope

- pi-mcp non-goals: "Batch JSON-RPC messages, legacy HTTP+SSE, servers, sampling, and tasks are outside the initial core" (`packages/mcp/README.md:132-133`) — README authority (shipped doc, implementation fact), implemented-exclusion; observed maturity: core client implemented and tested (`test/{client,stdio,streamable-http,oauth,content}.test.ts`).
- pi-env protocol/semantics docs are normative contracts within the package (`docs/protocol.md:1`, `docs/semantics.md:1`), fully implemented by the daemon and validated by conformance + differential tests (`README.md:60-66`) — maturity: implemented.
- pi-codemode platform caveats carry design content: Bun `terminate()` delays (`README.md:57-59`), Cloudflare interrupt budget vs `timeoutMs` (`README.md:243-247`), "Terminals are not supported on Windows yet" (pi-env `README.md:56`) — authority wording "not supported yet"/measured-on-Cloudflare notes; maturity: implemented-with-known-limits.
- Opi MCP stance: `crates/opi-agent/README.md:489` "No in-core ... MCP runtime" and `crates/opi-coding-agent/README.md:696-698` (MCP workflows are "examples/package patterns, not built-in core workflows") — normative product boundary, maturity: intentional absence, demonstrated only by fixture example test (`tests/mcp_adapter_example.rs`).
- opi-eval self-description: "unpublished Independent Companion ... may change without a compatibility shim" (`crates/opi-eval/src/lib.rs:3-9`) — crate-authority wording, maturity: implemented (43k LOC, full CLI).

## Coverage honesty

- Not inspected: pi `packages/durable` (RemoteExecutionEnv's `ExecutionEnv` counterpart) — out of this batch's four assigned packages; ENV-02/03 conclusions rest on pi-env's own docs/source plus Opi absence searches. `packages/agent/examples/mcp-codemode` was only listed, not read (agent-package batch owns it). pi-mcp OAuth `flow.ts` internals were read for exports and key line anchors only, not line-by-line; no Opi-side OAuth equivalence beyond confirming provider-login scope was claimed.
- Not executed: nothing — read-only source inspection per `00-inputs.md`; no builds, tests, or probes were run in either tree. `rg` was unavailable (exit 127); all searches used GNU grep 3.11 equivalents recorded per section.
- opi-eval depth: `main.rs` CLI surface, module docs, `comparison.rs`, `report.rs`/`regrade.rs`/`integrity.rs`/`bundle` headers, benchmark docker usage, and conformance docs were read; the 4.7k-line `runner/experiment.rs` and benchmark adapters were skimmed for docker/material handling only, sufficient for the EVL verdicts which rest on architecture-level comparison, not line-level parity.
- Verdicts `missing` for MCP/CDM follow from exhaustive term searches over `crates/` (both `src` and whole-tree sweeps shown above); a differently-named implementation that never mentions `mcp`/`model context protocol`/`json-rpc`/`stdio`/`sse`/`quickjs`/`codemode` in ~54k lines of Rust source would be required to overturn them, which the module layout and README statements make implausible.
