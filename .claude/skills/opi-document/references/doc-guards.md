# opi doc-guards reference

The authoritative list of guard-test suites that read opi docs from disk and
assert specific prose. Read this before editing any doc (Phase C of
`opi-document`); run the verify command after (Phase G).

The guards check token **presence** (required keywords must remain) and forbid
**overclaim** phrases (which pass only when negated). They do **not** enforce
absence of internal jargon like "Phase N" — a leftover phase label won't trip a
guard, so pair every edit with a `grep "Phase [0-9]"` pass.

## Verify command

```sh
cargo test -p opi-coding-agent \
  --test productized_packages_docs \
  --test phase11_tooling_quality_docs \
  --test phase12_provider_correctness_docs \
  --test phase13_session_context_docs \
  --test observability_docs \
  --test runtime_contract_docs
cargo test -p opi-agent --test transport
```

Expect `0 failed` across all suites (~67 tests total). Any failure names the
file + the missing/forbidden phrase.

## The eight suites and what each pins

### `crates/opi-coding-agent/tests/productized_packages_docs.rs` (Phases 5/6/9/10)
Reads: root `README`(.zh), `docs/opi-spec.md`(.zh), `docs/pi-alignment-matrix.md`(.zh),
`AGENTS.md`, `CLAUDE.md`, `CHANGELOG.md`, the four crate `README`s (EN+ZH),
`.claude/skills/opi-release/skill.md`.
- Version strings, **exact**: root `The workspace package version in \`Cargo.toml\` is \`0.6.5\``
  (EN) / `\`Cargo.toml\` 中的 workspace 包版本是 \`0.6.5\`` (ZH); crate
  `Current crate version: \`0.6.5\`` (EN) / `当前 crate 版本是 \`0.6.5\`` (ZH).
  **Every version-bearing line must move in lockstep on a bump.**
- `Packages are trusted code` (EN) / `Package 是受信任代码` (ZH);
  `not enforced sandbox policy` / `不是强制 sandbox 策略`.
- Package CLI token (`opi package add/remove/list/doctor`); the words
  `process` and `adapter` both present.
- pi-0.80.2 baseline (`.repo/pi-0.80.2`).
- **Forbidden overclaim** (positive mentions trip the guard unless negated):
  `npm`, `marketplace`, `hot-reload`, bundled Node/JS runtime, TypeScript
  extension API, `pi session v3`, broad OAuth provider parity, image generation,
  web UI, `opi-types`.

### `crates/opi-coding-agent/tests/phase11_tooling_quality_docs.rs`
Reads: `crates/opi-coding-agent/README.md`(.zh), `docs/opi-spec.md`(.zh), `CHANGELOG.md`.
Pins exact strings in the coding README (EN + ZH): `cmd /C` / `sh -c`,
`30 seconds` / `30 秒`, `timeout_secs`, `cancelled=true` / `timed_out=true`,
`2000` + `lines omitted`, `64 KiB`, `details.env`, `details.full_output`,
`workspace root` / `工作区根目录`, `not restricted to the workspace` /
`不限制在工作区内`, `--tools` / `--no-tools` / `--no-builtin-tools` /
`--allow-mutating`, the read-only/mutating classes (`只读` / `修改性`), and the
"tool-selection check, not a permission or sandbox subsystem" sentence
(`工具选择校验`). Also requires the **nine Phase-11 non-goal tokens** in both
languages: `permission popup` / `权限弹窗`, `background bash` / `后台 bash`,
`remote execution` / `远程执行`, `IDE project index` / `IDE 项目索引`,
`language-server` / `语言服务器`, `automatic formatting` / `自动格式化`,
`package ecosystem` / `package 生态`, `workflow tools` / `工作流工具`,
`sandbox`.

### `crates/opi-coding-agent/tests/phase12_provider_correctness_docs.rs`
Reads: `crates/opi-ai/README.md`(.zh), `crates/opi-coding-agent/README.md`(.zh),
root `README`(.zh), `docs/opi-spec.md`(.zh), `docs/pi-alignment-matrix.md`(.zh).
- The nine provider module ids (`anthropic`, `openai`, `openai-responses`,
  `openrouter`, `mistral`, `gemini`, `bedrock`, `azure`, `vertex`) in the root
  README; the nine `pub mod` names backticked in the opi-ai README.
- `SigV4`; `config-driven` + `preferred`.
- The nine CompatConfig flags (both opi-ai and coding READMEs):
  `system_role_override`, `max_tokens_field`, `tool_result_name_field`,
  `usage_in_stream`, `strict_tool_schema`, `reasoning_effort`, `cache_key`,
  `require_assistant_after_tool_result`, `chat_completions_path`.
  (`extra_headers` is pinned separately, not as a flag.) `ModelCompatOverride`.
- `include_usage` across **eight** surfaces (EN + ZH); `any streaming chunk` /
  `任意流式 chunk`.
- `ResponsesConfig`, `store`, `strict_tools`, `previous_response_id`.
- The OpenAI-Chat response-ID sentence, four surfaces EN + ZH:
  `OpenAI Chat captures the ID from any` + `chunk carrying \`id\`` (opi-ai);
  `OpenAI Chat captures response IDs` + `chunk carrying \`id\`` (coding/spec);
  root `OpenAI Chat chunk carrying \`id\`` + `response IDs captured`;
  ZH `OpenAI Chat 会从任何携带 \`id\` 的 chunk` + `捕获 response ID`.
- `reasoningContent` (EN + ZH); `extra_headers`; `best-effort` / `best effort`;
  `unknown usage` + `cost summaries should therefore be omitted` /
  `usage 未知` + `费用汇总就应省略`; coding `session cost summaries` +
  `omitted when any turn` / `会话费用汇总会被省略`; `HTTPS_PROXY` + `NO_PROXY`.
- The **ten Phase-12 non-goal tokens** in the opi-ai README (EN + ZH):
  `OAuth login` / `OAuth 登录`, `Anthropic subscription auth` /
  `Anthropic 订阅鉴权`, `OpenAI Codex subscription auth` /
  `OpenAI Codex 订阅鉴权`, `GitHub Copilot auth` / `GitHub Copilot 鉴权`,
  `broad new first-class provider list` / `first-class provider 列表`,
  `Image generation` / `图像生成`, `Browser usage` / `浏览器使用`,
  `streaming-adapter protocol` / `流式 adapter 协议`,
  `Paid live provider calls` / `付费实时 provider 调用`,
  `provider-specific config file format` / `provider 专用配置文件格式`.

### `crates/opi-coding-agent/tests/phase13_session_context_docs.rs`
Reads: `docs/opi-spec.md`(.zh), `crates/opi-agent/README.md`(.zh),
`crates/opi-coding-agent/README.md`(.zh), root `README`(.zh).
- Root README `sensitive` (EN) / `敏感` (ZH).
- Coding README `export-session` + `redact` (EN) / `脱敏` (ZH).
- opi-agent README: the five entry names `session_info`, `model_change`,
  `thinking_level_change`, `label`, `branch_summary` (EN + ZH), plus
  `custom_message` and `branch_summary`.

### `crates/opi-coding-agent/tests/observability_docs.rs` (Phase 7)
Reads: `docs/opi-spec.md`(.zh), root `README`(.zh), `docs/pi-alignment-matrix.md`(.zh),
`crates/opi-coding-agent/README.md`(.zh).
- `local` + `explicit` (EN) / `本地` + `显式` (ZH); `0.x` or `unstable` (EN) /
  `不稳定` or `0.x` (ZH).
- `opi doctor` + `trace`; backticked `` `trace` `` in root + coding READMEs;
  `startup_diagnostics` in the coding README (EN + ZH).

### `crates/opi-coding-agent/tests/runtime_contract_docs.rs` (Phase 8) — structural
Reads: `crates/opi-agent/README.md`(.zh), root `README`(.zh), `docs/opi-spec.md`(.zh),
`docs/pi-alignment-matrix.md`(.zh).
Parses the opi-agent README between headings **`## API Surface Classification`**
and **`## Non-Goals`** (ZH: `## API 表面分类` / `## 非目标（Non-Goals）`) and
asserts every crate-root `pub use` in `crates/opi-agent/src/lib.rs` appears as a
classified table row. Required tier labels: `supported 0.x` / `unstable
internal` / `candidate removal` (ZH `支持的 0.x` / `不稳定内部` / `候选移除`).
Also requires `no stable 1.0` (ZH `不会给出稳定 1.0`) + `#[non_exhaustive]`,
and the three schema-version strings `SDK_SCHEMA_VERSION = 3`,
`NDJSON_SCHEMA_VERSION = 2`, `TRACE_SCHEMA_VERSION = 1`.
- The Notes cell (third column) is **free text** — edit it freely; the guard
  parses only the surface-name (col 0) and tier (col 1).
- **Forbidden overclaim** across all listed files: `stable 1.0 public API`,
  `TypeScript extension API`, `package ecosystem expansion`, `package
  marketplace`, `new adapter kind`, `web UI`, `web dashboard`,
  `provider OAuth login`, `in-core plan mode`, `in-core sub-agent`,
  `MCP runtime`, `shared opi-types crate`, `whole agent loop rewrite` (and the
  ZH equivalents) — pass only negated.

### `crates/opi-agent/tests/transport.rs`
Root README must **not** contain `transport abstraction` (EN) or
`transport 抽象` (ZH); `docs/opi-spec.md`(.zh) must not contain the stale
transport-stub phrases.

## Gotchas

- **No README is `include_str!`'d into rustdoc.** README code blocks are not
  doctests; `cargo doc` / `cargo test --doc` are unaffected by README edits.
  Re-check this if a crate gains `doc = include_str!("../README.md")`.
- **`docs/opi-spec.md` carries a separate SHA256 pin**
  (`crates/opi-coding-agent/tests/phase4_ledger.rs`), not a keyword guard. Any
  byte change to `opi-spec.md` requires re-syncing the phase4 snapshot
  (`docs/snapshots/phase4/opi-impl-state.json`) and the live repo-root
  `.opi-impl-state.json` spec-hash.
- **Version-bearing lines move in lockstep** on any version bump: root README +
  `AGENTS.md` + `CLAUDE.md` + the four crate READMEs, EN + ZH. The exact
  phrasings above are match-targets.
- **Negation model**: the overclaim guards are per-line substring checks with a
  negation exemption (`not`, `does not`, `no`, `without`, `不声明`, `不会`,
  `不得`, …). A forbidden phrase survives only inside a clear negation; keep
  the negation on the same line.
- **Pinning density**: `crates/opi-tui/README.md` is the least-pinned README
  (only its version line); it has the most trimming headroom. The coding README
  is the most-pinned.
