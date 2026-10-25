# Reference Product configuration — delivery-source candidate

[简体中文](delivery-candidate.zh.md)

## Status and authority

This candidate materializes the user-approved [design](../../superpowers/specs/2026-10-25-opi-ai-onboarding-and-config-design.md) and [acceptance plan](../../superpowers/plans/2026-10-25-opi-ai-onboarding-and-config-acceptance.md). It is **not registered in opi-implement, not in the current spec_files, not an admitted Phase, and not runtime evidence**. HOST-CFG and HOST-VALUES are stable source-row identifiers, not reserved Phase/task numbers. Future registration must select a Phase, register this bilingual source explicitly, extract a draft and complete P.0/P.2/P.4 before canonical task installation.

The parent [Opi specification](../../opi-spec.md) governs AUTH-003/004, GOAL-001/003, PRIN-001–005, PLACE-001–005, CTRL-001/003, INV-001/002 and PHASE-001–006. This product usability work follows STRAT-001's independently usable runtime and the Reference Product assembly route in §4.2 and §9 Parallel routes. [CONTEXT](../../CONTEXT.md) governs Human Authority, Reference Product, User Policy, Package Trust and Supervision. No parent amendment, new Agent Core module or inward dependency reversal is required.

## Problem, outcome and placement

The product's existing TOML vocabulary and combined settings/provider document prevent direct reuse of Pi settings/model fields. Merely changing a parser would not align model selection, provider definitions, value sources or the actual request. The outcome is default TOML with explicit Pi JSON over one product-owned meaning and one actual configuration-to-request path.

Placement is **reference-product**: `opi-coding-agent` already owns config/CLI/startup/trust/provider assembly. Reuse `src/config.rs`, `cli.rs`, `main.rs`, `provider_factory.rs`, the established credential backend and injectable SDK sources. Neither `opi-ai` nor `opi-agent` gains a product file parser, command-policy owner or dependency on the product. Two syntaxes are not two resolvers. This is a direct external-format cutover, not a permanent compatibility shim or generic config framework.

Priority: consume the admitted SDK factory/native option seams after the relevant Phase 20 owners; keep the standalone SDK independently usable before and without this product work. HOST-CFG is independently demonstrable with literal synthetic credentials; HOST-VALUES builds on it. Neither is added to Phase 20's 57 AIR / 54 V SDK exit or made a prerequisite of 20.57/58/59.

## Fixed comparison inputs and version boundary

Planning base: Opi `de703bb12a7602d0f128b22a7cbb4bb6c81f12ab`. The inward comparison is local `.repo/pi-1.4.0`, limited to configuration fields and their real loaders. SHA-256 values below identify local bytes, not an upstream Git revision.

| Relative Pi path | SHA-256 |
|---|---|
| `packages/coding-agent/schemas/settings.schema.json` | `d4b19b0e492cb887a23c50034f00213c476048956fa92c5cce2f1536ef50b9b0` |
| `packages/coding-agent/schemas/models.schema.json` | `86ade9716724902bdfe91347a728980ad7242a76d0773dbc7217ce4584d696ea` |
| `packages/coding-agent/src/core/settings-manager.ts` | `aef8fea51000f291c1c51d81fd9b30f7a2bdd0e68370ad84b969e5fd15667e98` |
| `packages/coding-agent/src/core/model-registry.ts` | `f4ced2d92df9097647aba7da3ae0bb273fa8089e966be15dba880db51d5dee0b` |
| `packages/coding-agent/src/core/resolve-config-value.ts` | `0f53dad47fe5d5d8837c022b7951ccd3bd5a9b577bd662f0986272110e83bcc7` |

Selecting these config fields does not adopt all Pi 1.4 behavior. SDK protocol/option defaults remain the registered Pi 1.3 contract until their own source-delta admission. Client identity, changed whitespace tool-result behavior and new unrelated product features remain outside this candidate. Recheck loader imports and relevant helper identities before future execution; fixed schema bytes alone do not establish behavior.

## Configuration contract

- Default user locations remain `%APPDATA%/opi` on Windows and `~/.config/opi` on Unix. Trusted project location remains `.opi/`. Settings default to `config.toml` with optional `models.toml` in the same directory.
- Existing `--config <FILE>` explicitly selects a settings bundle: `.toml` uses companion `models.toml`; `.json` uses `models.json`. A different settings basename is allowed. Raw models as primary, an unsupported extension or content of the wrong syntax fail. JSON is never auto-discovered or used as a fallback.
- Default missing settings is an empty settings layer and does not suppress a present companion. Missing explicit settings fails. Missing companion means empty model layer; a present unreadable/malformed/unsupported companion fails the complete selected bundle before publication. No import of adjacent `auth.json`, packages, extensions, skills, prompts or instructions follows from bundle selection.
- Both syntaxes feed one private presence-aware typed resolver with provenance. Preserve built-in defaults → user → trusted project → explicit → CLI order and the existing `OPI_MODEL` override only when `--config` is absent. CLI/env model selectors override provider/model atomically; normal Pi settings retain separate `defaultProvider`/`defaultModel` fields.
- Use Pi names and nesting directly, including camelCase. Known old Opi fields (`defaults`, old `model`, `api_key_env`, old thinking paths and their nested forms) fail with path/replacement diagnostics. No aliases, legacy reader or automatic converter.
- Produce the complete field/consumer inventory before cutover: Pi equivalents for model/theme/thinking/trust and matching compaction/retry fields; model-document provider/API/baseUrl/apiKey/headers/models/modelOverrides/compat/authHeader plus metadata/cost; typed `opi.*` for existing limits, authority/execution/permissions/backend/keybindings and other behavior without a true Pi equivalent. Preserve existing capabilities and omission semantics. An absolute compaction threshold is not Pi reserveTokens; Pi's separate keybindings protocol is not silently imported into settings.
- A recognized unsupported Pi field fails explicitly. Truly unknown additional data stays inert, preserved and visibly warned as having no effect. It cannot reach a request body. `$schema` is metadata, never network discovery. Per-field nested/map/array/clear behavior follows the adopted loader; do not concatenate every array or replace explicit false/zero/empty with defaults.
- The common non-null subset resolves identically. TOML has no native null; schema-approved null-only configurations use explicit JSON. In `thinkingLevelMap`, null marks unsupported while omission has the source default; do not add a null sentinel or coerce omission to null.
- Preserve Opi project/resource/instruction trust and pre-trust staging. A project cannot grant persistent execution permission, including after it is trusted. Preserve current credential-backend/keychain ownership. Only an actual selected source resolves credentials; parse/construct/list/configuration inspection cannot execute a value command or auth refresh.
- If a selected document is saved by existing UI paths, write only that document atomically in the same format; preserve inert unknown values and do not rewrite companions/other formats. Failure does not report persistence success.

## Value-source contract

HOST-CFG parses expression forms without evaluating them; until HOST-VALUES supplies a form, use an explicit unsupported diagnostic, never a literal credential or implicit fallback. Existing SDK lazy default sources remain available independently.

HOST-VALUES implements the selected Pi model-value path: literal values, `$NAME`/`${NAME}` interpolation, `$$`/`$!` escaping and leading `!command`. Freeze the selected resolved value/identity for its logical request; resolve again for the next request rather than copying Pi's separate process-global cached helper. Honor the source's empty/error semantics with typed/redacted failures and no account rediscovery.

Commands belong to the product's source-resolution lifecycle: use the configured shell with correct argv, freeze cwd/env at selection, apply the source's 10000ms command timeout, propagate cancellation and supervise the process tree through cleanup, suppressing late results. This timeout does not replace auth-network/ACK or generation timing. Parsing/configuration checks are inert; an untrusted project cannot execute a command. Reuse owned process and credential mechanisms after examining their true contracts; AWS credential-process JSON output is not this command's string-output protocol. No automatic auth.json import/export or credential-store implementation replacement is included.

## Source rows and proposed task registration

These two independently demonstrable rows become separate tasks only after a future Phase/source/graph is confirmed. They do not receive `20.60/20.61` IDs and do not modify the current graph.

| Field | HOST-CFG | HOST-VALUES |
|---|---|---|
| Deliverable | Default TOML + explicit Pi JSON, direct Pi fields, one resolver, real product request | Pi model-value sources through real auth/headers with owned command lifecycle |
| Placement / tier / evaluator | reference-product / workspace / true | reference-product / workspace / true |
| Real blockers | Admitted SDK factories, native options and callbacks from the current SDK owners; resolve exact cross-Phase references at registration | HOST-CFG and reusable admitted auth/process mechanisms; resolve exact references at registration |
| Existing production seam | `config::stage_config`/`StagedConfig::finalize_with_project`/`resolve_config` → startup → `provider_factory` → real SDK adapter | Product provider/auth/header assembly → selected source → actual SDK adapter request |
| Candidate owned paths | Product `src/config.rs`, `src/cli.rs`, `src/main.rs`, `src/provider_factory.rs` and necessary existing startup/diagnostic/trust consumers; exact config/command consumers from the field inventory; root/crate bilingual product READMEs and CHANGELOG | Product `src/config.rs`, `src/provider_factory.rs`, necessary credential/header source consumer paths; bilingual config guide and CHANGELOG |
| Future files | `crates/opi-coding-agent/schemas/settings.schema.json`, `schemas/models.schema.json`, `tests/fixtures/pi_config/**`, bilingual product config guide; private parser split only if justified | `crates/opi-coding-agent/src/config_values.rs`, `tests/config_value_sources.rs` only after reuse search confirms the missing product seam |
| Existing test homes | `config_tests`, `provider_factory`, `cross_mode_runtime`, `trust_resource_gating`, `early_command_trust`, `doctor_cli`, `list_models`, affected execution/credential tests | Existing credential/trust/factory tests plus the distinct public command-source binary above |
| Replacement / ceiling | Replace old external vocabulary once; keep one resolver and no old aliases. Revisit when a source-valid Pi config cannot be represented/diagnosed or an existing capability loses its consumer. | One selected source-resolution owner, no general shell/plugin framework or global token cache. Revisit on a concrete new supplier requiring different lifecycle/authority. |

At registration, inventory both tasks' exact paths, runtime callers and test dispositions; then fill the existing four minimum-change notes and any real shared-decision notes. HOST-CFG owns the presence/provenance representation consumed by HOST-VALUES. Syntax helpers are private, not invented production consumers. Preserve the SDK's selected-identity rule without copying its resolver. Task IDs, complete dependency closure and all required fields are derived at the future graph gate, not guessed here.

## Success, exit and executable scenarios

| Criterion / owner | Public scenario and expected effect | Future verification |
|---|---|---|
| HCFG-01 / HOST-CFG | Paired Pi-field settings and provider/model fixtures resolve equally and drive the same local real request. Literal synthetic credential is sufficient. | `cargo test -p opi-coding-agent --test config_tests --test provider_factory --test cross_mode_runtime` |
| HCFG-02 / HOST-CFG | Default TOML only, explicit JSON bundle, model-only default, mixed lower TOML/explicit JSON, missing/invalid/wrong-role files and companions follow exact selection/atomic-failure rules without fallback. | `cargo test -p opi-coding-agent --test config_tests` |
| HCFG-03 / HOST-CFG | Per-field merge, false/zero/empty/null, legacy rejection, unknown/unsupported diagnostics and all retained Opi capabilities reach their actual consumers; no parse-time auth/command effects. | Config/factory tests and affected execution/credential binaries selected by the field inventory |
| HCFG-04 / HOST-CFG | Trust, discovery/list/doctor and pre-trust startup remain safe; project cannot self-authorize. Document-scoped persistence is atomic where the product writes. | `cargo test -p opi-coding-agent --test trust_resource_gating --test early_command_trust --test doctor_cli --test list_models` plus affected persistence tests |
| HVAL-01 / HOST-VALUES | Literal/env/escaping/command values reach actual request auth/headers; absent/empty/malformed/failed selected source returns its typed result without another identity. | `cargo test -p opi-coding-agent --test config_value_sources --test provider_factory` |
| HVAL-02 / HOST-VALUES | Fake shell success/failure/stall/child, cancellation, changed next-call output/env, untrusted project and late-result races prove 10000ms source scope, frozen invocation, cleanup and uncached-per-request behavior. | `cargo test -p opi-coding-agent --test config_value_sources --test early_command_trust` |

Exit requires both rows' actual production-path proof, complete field inventory and bilingual help/schema/manual consistency, preserved authority/identity and truthful missing-platform evidence. Docs-only, parser-only, token-only or substrate tests cannot close runtime success. Run changed binaries and applicable workspace fmt/clippy/all-targets/doc/rustdoc gates; keep shared external Cargo cache and never cargo clean. Use fake clock, synthetic credentials, isolated HOME/env and controlled local peers. Inspect Pi test imports before running approved fixtures; never activate live calls because credentials exist.

## Non-goals, platform, risk and rollback

No whole Pi product port, automatic JSON discovery, auth.json/keychain migration, new CLI config-directory flag, legacy TOML compatibility, session format change, tool/permission redesign, MCP/codemode/package execution or new SDK option contract is selected. Pi `.pi` development resources are not SDK config or executable acceptance inputs. Host-only work does not change current SDK exit.

Native Linux/macOS/Windows coverage follows workspace manifests and supported claims; shell/process behavior requires owning native proof before support is advertised. Missing platform execution stays missing. Stop cutover for identity/secret leakage, unsupported source provenance, duplicate semantic owners, lost Opi capability, trust regression or unresolved parent conflict. No extra numeric resource cap follows.

Changes remain reversible source slices with synchronized consumers/schemas/fixtures/bilingual docs and Unreleased. Keep original config files intact unless a user action writes the selected document; a failed parse/resolve/save does not activate partial configuration. A shipped rollback uses an explicitly authorized revert/release; it does not silently convert user JSON/TOML, resurrect a removed reader or claim remote effects were rolled back. Current planning changes no production/test file and claims no runtime result.
