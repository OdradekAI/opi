# Opi onboarding and configuration: design acceptance plan

> **For agentic workers:** After source/graph admission, use `superpowers:subagent-driven-development` or `superpowers:executing-plans` for the admitted work. This document is an acceptance proposal, not implementation admission. The user's current instruction permits planning only; it permits no production, specification or ledger edits and no commit.

[简体中文](2026-10-25-opi-ai-onboarding-and-config-acceptance.zh.md) · [Design](../specs/2026-10-25-opi-ai-onboarding-and-config-design.md)

**Goal:** Make the public SDK independently usable from its manual, give the Reference Product default TOML plus explicitly selected Pi JSON, and preserve adopted provider request semantics.

**Architecture:** SDK factories and adapters own provider behavior. The product owns one presence-aware configuration resolver with two input syntaxes. Documentation is verified against the same examples used by an external consumer of the actual Cargo package.

**Tech stack:** Existing Rust workspace, Serde/TOML/JSON, Tokio, local HTTP peers, fake clock, isolated configuration roots and the existing Cargo cache/package-proof workflow. No dependency addition is approved by this plan.

## Global constraints and evidence status

- Baseline: Opi `de703bb12a7602d0f128b22a7cbb4bb6c81f12ab`; ledger SHA-256 `b9273baa5b5751717f6e4fdea7ab9c7fbd5be11a9e1ff7150c6c04454476592a`.
- Current Phase 20 source remains Pi 1.3.0. Pi 1.4.0 provides proposed configuration vocabulary and comparison evidence; behavioral deltas require explicit adoption.
- Old external TOML fields may break. Use Pi names directly; no legacy alias, dual semantic model or automatic old-format reader.
- Preserve Opi's execution authority, project trust, credential backend and SDK/product dependency direction. Preserve current capability through a Pi equivalent or a documented Opi extension.
- No real accounts, browser authorization, cloud metadata or paid providers. Tests use synthetic keys and isolated HOME/env/config roots; environment mutations are serialized.
- No blanket timeout/retry policy, speculative public trait, full Pi runtime import, performance claim from functional tests, or package publication.
- Only an explicitly invoked `opi-implement` workflow may reconcile `.opi-impl-state.json`. This proposal creates no new task ID or parallel progress ledger.
- The case names below identify acceptance requirements, not task status. All runtime commands are **planned, not executed in this planning turn**.

## File ownership before implementation

Paths are relative to the repository root. Existing files are owners to inspect and change only after admission; this list does not confer path authority.

| Track | Existing owners | Proposed additions / boundary |
|---|---|---|
| SDK onboarding | `crates/opi-ai/README.md`, `README.zh.md`, `src/models.rs`, `src/provider.rs`; root `README.md` / `README.zh.md` for product configuration | Future `crates/opi-ai/examples/quick_start.rs`, `quick_start_stream.rs`, `tool_loop.rs`. They do not exist at this baseline. API changes stay with factories/options tasks. |
| Configuration selection and resolution | `crates/opi-coding-agent/src/config.rs`, `src/cli.rs`, `src/main.rs`, existing startup/provider-factory consumers and tests | Private format-specific parsing may be split from `config.rs` if needed; no new SDK loader. Future product settings/model schemas and bilingual config guide must be named by the admitted task; no product schema directory currently exists. |
| Configuration-value resolution | Product auth/header source integration, existing credential and process-lifecycle seams | A distinct host slice owns command resolution and cancellation. It must not duplicate AWS credential-process or change auth-store ownership. |
| Provider options | `crates/opi-ai/src/models.rs`, actual provider modules, `tests/request_enrichment.rs`, protocol fixture/lifecycle tests; product factory/config tests | Extend the existing parameter inventory and source-qualified fixtures. No generic arbitrary-body passthrough. |
| Package and delivery | `scripts/pi-sdk-conformance/`, existing package evidence, `CHANGELOG.md` | Future `tests/external_sdk.rs`, `catalog_artifact.rs`, `platform_delivery.rs` are task-declared targets, not existing passing evidence. Extend existing harness ownership rather than creating a second package framework. |

## Dependency order and canonical task disposition

| Order / track | Existing task owner | Required plan or acceptance addition |
|---|---|---|
| Source reconciliation | Existing Phase 20 registered-source and graph admission | Review approved deltas, preserve original evidence and pins, and obtain the existing human graph confirmation. Do not simply replace source hashes. |
| SDK mechanisms | 20.35 and 20.51, with their existing prerequisites | Complete simple/native parameter inventory and header/callback semantics. |
| SDK adapters | 20.36–20.44 and other registered prerequisites | Actual request/response proof for each native protocol; auth-only evidence does not close generation. |
| Typed operations | 20.45–20.49 | Their own image/classifier/Decisions option and wire contracts; do not substitute a Chat-only parameter inventory. |
| SDK assembly | 20.32 / 20.33 → 20.34, preserving the ledger's complete dependencies | Packaged catalog, dynamic catalog and factory/profile integration. Add effective-parameter public request cases to 20.34; do not add host parser ownership. |
| Host configuration, separate lane | New Reference Product slice, unnumbered and unadmitted | Bundle discovery, Pi fields in TOML/JSON, field inventory, merge/trust/diagnostics and configuration-to-request proof. Consumes admitted SDK factories/options. |
| Host value sources | New Reference Product slice, unnumbered and unadmitted | Literal/env/escaping/command semantics through actual auth/headers, with an owned process lifecycle. Consumes the host resolver. |
| SDK manual / retirement | 20.56 | README organization, exact current examples, outdated public-symbol retirement and migration documentation. File ownership alone does not admit host migration. |
| External usability | 20.57 | Same-source quick starts in an ordinary external consumer of the real `.crate`; V-39 and independent SDK operation. |
| Platform/package claims | 20.58 | Native/MSRV 1.97, features, actual dependency graph and supported platform evidence. |
| Documentary closure | 20.59 | Bilingual manual, source-qualified evidence and remaining gaps. Keep P.0/P.2/P.4 drift reconciliation and human confirmation. |
| Phase exit | Independent F.1a | Assess the complete admitted graph. Neither 20.59 nor this proposal declares Phase exit. |

SDK onboarding does not depend on reading product TOML/JSON. The host lane must not become a prerequisite of the standalone SDK. Do not reopen passing auth tasks for field renaming; rerun their affected regressions when a real consumer/source change invalidates evidence.

This table is a scoped dependency view, not a replacement graph. Preserve 20.54 before the real adapters, 20.55 before 20.56, and all existing 20.50/20.52/20.53 subtask dependencies before their registered consumers. Their deferred/retry/transport/telemetry obligations remain active without adding new scope here.

## Acceptance package A: source, field inventory and admission

**Inputs:** The design, registered Phase 20 sources, current Opi configuration types/consumers, and exact local Pi schema/loader sources. **Output:** An approved source delta and canonical task ownership, produced only by the owning workflow after the user requests that work.

1. Extract the current Opi external fields and their actual consumers. Compare Pi settings schema, model schema, `settings-manager.ts`, `model-registry.ts` and `resolve-config-value.ts`; inspect source behavior where schema/prose is insufficient.
   -> verify: every old capability is mapped to a genuine Pi equivalent or typed `opi.*` extension, and every supported external field has presence, null, unit, merge, authority and consumer rules.
2. Review the following minimum migration dispositions. They constrain the inventory; they do not claim a full Pi feature implementation.

| Current capability | Proposed destination / acceptance condition |
|---|---|
| `defaults.model` | Separate `defaultProvider` and `defaultModel`; resolve the pair after layers, with atomic CLI/env selector override. |
| Theme and project-trust default | Pi `theme` / `defaultProjectTrust` names where meanings match; keep Opi's pre-trust authority and project restriction. |
| Thinking enabled/budget | `defaultThinkingLevel`, `modelThinkingLevels`, `thinkingBudgets`; do not mechanically map one old boolean/budget to all native wires. |
| Compaction and retry | Pi nested fields only for matching semantics. An absolute occupancy threshold is not `reserveTokens`; retain a necessary Opi-only behavior under `opi.*` unless its owning contract is explicitly revised. Agent retry is not provider retry. |
| Provider/model definitions and compatible/custom profiles | Pi model document `providers`, canonical provider/API IDs, `baseUrl`, `apiKey`, `authHeader`, `headers`, `models`, `modelOverrides`, `compat`, metadata and cost units. Preserve existing supported wires and proxy behavior through exact equivalents or Opi extensions. |
| Iteration/output/tool/image limits, mutation policy, execution routing/permissions and credential backend | Typed `opi.*` fields where Pi has no genuine equivalent; preserve omission/default and authorization semantics. |
| Resource paths / packages | Pi list/object forms only for supported resource behavior; retain Opi trust and precedence. No implicit Pi package, MCP or skill execution. |
| Existing keybindings | Preserve current actions. Pi owns a separate keybinding file/protocol; do not claim it is part of `settings.json` or silently drop Opi bindings. Use a scoped Opi extension until that separate protocol is admitted. |

3. Classify Pi 1.4-only additions separately, including client identity and changed whitespace tool-result behavior. Retain current Pi 1.3 assertions until the owning source is superseded.
   -> verify: no test change or new field silently changes the active version; no retired Opi spelling is retained as a reader alias.
4. Reconcile the approved source text and task graph through `opi-implement plan`, including actual source-drift gates and human confirmation.
   -> verify: task DoD, source pins, owned paths and dependency order describe the same work. This step is **not performed by this planning delivery**.

## Acceptance package B: TOML/JSON host configuration

**Future owner:** The new host configuration slice. **Test homes:** Existing `crates/opi-coding-agent/tests/config_tests.rs`, `provider_factory.rs`, `cross_mode_runtime.rs`, `trust_resource_gating.rs`, `early_command_trust.rs`, `doctor_cli.rs`, `list_models.rs`; add schema fixtures under the admitted product owner.

First add the paired synthetic fixtures below. Treat the address as data; tests substitute their own bound local peer before making a request. No service is presumed at port 18080.

Settings TOML / JSON, respectively:

```toml
defaultProvider = "fixture"
defaultModel = "chat"
defaultThinkingLevel = "medium"
```

```json
{
  "defaultProvider": "fixture",
  "defaultModel": "chat",
  "defaultThinkingLevel": "medium"
}
```

Companion model TOML / JSON, respectively:

```toml
[providers.fixture]
api = "openai-completions"
baseUrl = "http://127.0.0.1:18080/v1"
apiKey = "synthetic-test-key"
authHeader = true

[[providers.fixture.models]]
id = "chat"
name = "Fixture Chat"
input = ["text"]
contextWindow = 8192
maxTokens = 1024
```

```json
{
  "providers": {
    "fixture": {
      "api": "openai-completions",
      "baseUrl": "http://127.0.0.1:18080/v1",
      "apiKey": "synthetic-test-key",
      "authHeader": true,
      "models": [{
        "id": "chat",
        "name": "Fixture Chat",
        "input": ["text"],
        "contextWindow": 8192,
        "maxTokens": 1024
      }]
    }
  }
}
```

| Case | Setup / action | Required observation |
|---|---|---|
| CFG-01 | Load each paired document into the common semantic resolver. | Same values, presence and selected model/auth source; diagnostics retain each path/format. No env, command, auth-store or network work merely from parsing. |
| CFG-02 | Put valid JSON beside default user/project TOML; start without `--config`. Then explicitly select JSON. | Default uses TOML only. Explicit JSON uses its optional `models.json` companion; no auto-discovery of the sibling `auth.json` or resources. |
| CFG-03 | Select malformed JSON while valid TOML exists, use a missing explicit settings path, put TOML text in `.json`, and supply a raw model document as primary. | Selected source fails with path/format/field diagnostic; no sniffing, fallback, source rediscovery or partial publication. |
| CFG-04 | Omit optional companion; then supply unreadable, malformed and unsupported companions. Also discover a default bundle with only its model companion present. | Missing companion means empty model layer. Existing invalid companion fails the entire selected bundle; no old catalog/config is reported as newly loaded. Missing default settings still permits the selected model layer, subject to project trust. |
| CFG-05 | User/project/explicit layers select distinct values; add CLI selector and `OPI_MODEL` with/without `--config`. | Exact documented Opi order, atomic provider/model CLI/env override, and existing `OPI_MODEL` exception. An explicit JSON layer may overlay lower TOML layers. |
| CFG-06 | Exercise nested maps, arrays, empty collections and model overrides using the inventory's source-qualified examples. | False/zero/present-empty survive. Each field uses its own replace/merge/clear policy; resource arrays do not inherit a universal concatenation rule. |
| CFG-07 | Use old `[defaults]`, `model`, `api_key_env`, old thinking paths and mixed old/new keys. | Known retired keys fail with exact location and replacement guidance. No alias, silent ignore or automatic conversion. |
| CFG-08 | Supply a recognized but unsupported Pi feature, an unknown extension key, and `$schema` URL. | Unsupported feature fails explicitly. Unknown data is inert, preserved and visibly warned as having no effect; no request passthrough or remote schema fetch. |
| CFG-09 | Use schema-approved JSON `thinkingLevelMap` null, then compare omission. | Null means unsupported, omission retains the source's distinct default. The manual states that TOML cannot express native null; no sentinel or omission coercion. |
| CFG-10 | Trusted/untrusted project bundles try command keys, resource activation and `opi` execution permission grants. | Existing Opi trust boundary holds; pre-trust parsing starts nothing; project cannot self-authorize persistent execution, even when otherwise trusted. |
| CFG-11 | Exercise each existing Opi-only capability after renaming, including omission/default/override cases. | Behavior and authority unchanged; compaction threshold/reserve meanings and credential-backend selection remain distinguishable. |
| CFG-12 | Start the real product with each format, select fixture provider/model and send one request to the local peer. | Actual method/path/model/auth/header/body match in both runs; discovery/list/doctor agree on the same effective model without triggering auth exchange. |
| CFG-13 | If the admitted implementation writes configuration, edit one setting with inert unknown data and a companion present. | Atomic write of only the selected document, same format, unknown data retained, companion/other format untouched; failure does not report persistence success. |

Implementation order after admission:

1. Add CFG-01–09 fixtures and negative assertions to existing config tests.
   -> verify: newly required cases fail for the expected missing behavior, not from a missing dependency or a misconfigured fixture.
2. Replace the external reader vocabulary and share selection/merge/validation; synchronize owned consumers and schemas. Do not add old-field compatibility.
   -> verify: CFG-01–09 pass and unchanged authority cases remain valid.
3. Wire trusted staging, real factories/startup, diagnostics and document-scoped persistence where applicable.
   -> verify: CFG-10–13 plus affected execution, credential and trust regression tests pass.
4. Update bilingual guides/help/examples, actual schema references and the complete Unreleased section's appropriate subsection.
   -> verify: docs describe real behavior and the CLI does not advertise a nonexistent format/directory option.

## Acceptance package C: configuration values and provider parameters

The host value-source slice is independently rejectable from dual parsing. Parsing support must reject unsupported command expressions until this slice is delivered.

| Case | Setup / action | Required observation / owner |
|---|---|---|
| VAL-01 | Literal, `$NAME`, `${NAME}`, `$$`, `$!`, missing env, empty and malformed values. | Adopted Pi source semantics and typed selection failures; redacted diagnostics. Resolution occurs at use, not registration. New host value-source slice. |
| VAL-02 | Controlled fake shell command returns a key/header, fails, stalls or spawns a child; cancel during execution. Change env/output between calls. | Correct shell/argv, frozen cwd/env, exact source timeout, tree cleanup and uncached model-value behavior. No AWS JSON-process protocol or fallback to another account. |
| VAL-03 | Untrusted project config contains a command; another selected source fails or is cancelled. | No command before authority is granted; selected-source failure does not discover another identity. Actual request uses the selected value or fails. |
| PAR-01 | Capture a public call with omitted defaults, explicit false/zero, model/provider defaults and per-call overrides. | Source-qualified precedence and units preserved from product field to native/simple option to actual wire. 20.35 / adapter / 20.34. |
| PAR-02 | OpenAI Chat/Responses/Codex variants vary output limit, reasoning, store false/omission, session/cache/transport and tool choice. | Correct variant fields/omissions, callback count and retry identity; no universal serializer. 20.36 / 20.37 / dependent owners. |
| PAR-03 | Anthropic native thinking omitted, disabled, enabled with omitted budget and explicit budget; simple reasoning separately. | Native source default (Pi 1.3: 1024 where enabled without budget) is not replaced by the present generic 10000 path or the simple policy. 20.39 / 20.35. |
| PAR-04 | Google and Vertex native budget `-1`, `0`, positive value, supported levels and tool/signature cases. | Dynamic/disabled semantics survive typed conversion; Vertex auth target/quota provenance also reaches the real request. 20.40 / 20.41. |
| PAR-05 | Azure Chat and Responses, Bedrock, Mistral and Radius-specific requests. | Both Azure wires retain their source obligation; exact protocol fields, auth endpoint and retries. No auth-only closure for 20.41/42 generation; Radius uses its admitted owner. |
| PAR-06 | Header omission/set/removal and functional callbacks across retries/variants. | Null removal follows the SDK source, protected auth identity survives, exact per-variant timing/count. Runtime callbacks/signal/fetch are not JSON fields. 20.51 plus actual consumers. |
| PAR-07 | Malformed/unsupported option, selected-source error, cancellation, endpoint-sensitive compatibility and typed provider errors. | Fail before wrong-wire use; no account fallback, silent clamping or universal auth/network timeout. Auth-derived endpoint precedes compatibility inference. Existing auth regressions remain intact. |
| PAR-08 | Typed image/classifier/Decisions option fixtures, including unsupported input and Decisions credential restrictions. | The owning typed public operation consumes its own exact fields and returns its typed result/error; no Chat serialization or OAuth acceptance where API-key-only is required. 20.45–20.49. |

Implement the smallest owner-specific failing request-capture case, then the owned conversion/adapter, then its successful and invalid-input cases. Verify actual URL/headers/body and each attempted request. Record exchange requests separately from generation retries; reuse valid auth evidence rather than duplicating all source-chain tests.

## Acceptance package D: README and independent SDK use

**Owners:** 20.56 for manual/retirement; 20.57 for independent package proof; 20.58 for supported claims; 20.59 for handoff.

| Case | Proof required |
|---|---|
| SDK-01 | The first complete example is ordinary factory/model → Context → complete/stream use, with environment or synthetic explicit API key. No product config, Agent, manual resolver, route registration or prepare/attempt lifecycle is needed. |
| SDK-02 | Factory/default construction, registration and static model queries start no env/store/network operation. Explicit built-in assembly supplies the promised catalog/adapters; empty `Models::new` is not mislabeled as automatically populated. Default memory stores work without a host persistence implementation. |
| SDK-03 | Quick-start, streaming and tool-result continuation examples compile and run against the actual `.crate` from a fresh workspace-external consumer. Example source equals README source. A local peer proves a request and final result through the real adapter. |
| SDK-04 | Streaming and nonstreaming share the admitted final-result path; cancellation, typed error, usage and model/context continuation examples match their public contracts. Simple/native options and auth persistence responsibilities are distinct and usable. |
| SDK-05 | English/Chinese manuals have the same conditions, platform limits, examples and advanced links. Removed `Request.system/messages/tools` and retired global/frame APIs have no active example. Internal protocol/history detail retains a discoverable owning home. |
| SDK-06 | The external consumer's only Opi dependency is `opi-ai`, plus documented ordinary runtime/Serde/stream dependencies. No product factory, unpublished path outside the extracted package, `[patch]`, workspace-private feature or live credential is needed. Platform, MSRV, feature and performance claims cite actual appropriate evidence. |

1. Finish the admitted factories/native option behavior before presenting it as a working manual.
   -> verify: public SDK tests for the exact path pass; record still-pending providers honestly.
2. Create the proposed example sources, use them in the manual and implement the package consumer using the same sources.
   -> verify: SDK-01–04 and SDK-06 run against the artifact, not only workspace source; retain artifact/consumer lockfile/dependency graph hashes.
3. Reorganize the bilingual README and reconcile links, migration notes, root product examples and Unreleased entries.
   -> verify: SDK-05, doc checker, rustdoc and affected example targets pass. No stale manual section becomes a correctness source.
4. Hand source-qualified cases/results to 20.59 and then the independent exit assessment.
   -> verify: missing evidence remains missing; example success does not imply all profiles/platforms passed.

## Test lineage and planned verification commands

Retain current source-valid schema, transcript/replay and auth/rotation tests, including the third-node R-02 public-resolver timing cases. Update old-field config fixtures only with paired new-field positive/negative cases; remove an old assertion only after its contract is superseded. Add tests that expose distinct behavior, not prose or struct mirrors.

Inspect Pi test imports and helpers before executing anything. Some apparently ordinary Pi tests import OAuth helpers that read/refresh user credentials. Prefer pinned synthetic cases transcribed into Rust fixtures with source/version and an expected wire result. Do not run the whole Pi suite or local `.pi` extensions. Pi 1.4 whitespace/identity changes and live model-name changes are not automatic 1.3 regression requirements.

These commands are future checks selected by actual change scope. Run every changed test binary/filter; log command, code identity, exit and meaningful assertion count. A zero-match filter is not proof.

```powershell
python scripts/opi-cargo-cache.py status
cargo test -p opi-coding-agent --test config_tests --test provider_factory --test cross_mode_runtime
cargo test -p opi-coding-agent --test trust_resource_gating --test early_command_trust --test doctor_cli --test list_models
cargo test -p opi-ai --test request_enrichment --test model_wire_metadata --test per_request_auth
cargo test -p opi-ai --test openai_chat_fixtures --test openai_responses_fixtures --test openai_codex_responses
cargo test -p opi-ai --test anthropic_fixtures --test gemini_fixtures --test vertex_fixtures
cargo test -p opi-ai --test azure_openai_fixtures --test bedrock_fixtures --test mistral_fixtures
```

Use the already marked shared external Cargo target and leases as prescribed by the cache tool; keep incremental compilation. Do not run `cargo clean` or allocate a per-session target. Add affected execution/credential/source tests and new package/example targets when their admitted files exist. The existing `run_early_package_proof.py` covers the Schema seam; its previous success does not establish these new SDK quick-start cases.

Cross-crate runtime and final exit require the applicable workspace gates:

```powershell
cargo fmt --check --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --all-targets
cargo test --workspace --doc
$env:RUSTDOCFLAGS = "-D warnings"
cargo doc --workspace --no-deps
```

Restore any prior `RUSTDOCFLAGS` after the scoped check. Complete the real package/MSRV/platform evidence through its owner; never describe a nonexistent `external_sdk` or platform test target as run. Do not broaden/repeat passing checks without a new change, failure or evidence gap.

For this documentation-only planning delivery, the required checks are:

```powershell
python scripts/opi-doc-check.py
git diff --check
```

Also validate the paired TOML/JSON fixture equality, local document links and whitespace in new untracked Markdown files. No runtime behavior or tests are changed; test impact is `none`.

## Reviewable handoff and stopping point

The implementer's future evidence records each case as executed now, reproducible prior evidence, or missing, with relevant source/lockfile/config/artifact identities. Capture committed, staged, unstaged and untracked separately, preserving unrelated work.

This delivery stops with the bilingual design and acceptance plan. It changes no production file, registered source, schema, ledger, test or current README; it stages nothing and creates no commit. The unrelated research file is preserved and excluded. Source/task admission is the next separately authorized action, followed by dependency-ordered implementation and the owning final acceptance.

## This planning delivery's verification

On 2026-10-25, `python scripts/opi-doc-check.py` returned exit 0 with `opi documentation contracts: PASS`; `git diff --check` returned exit 0. A Python standard-library check parsed both pairs of TOML/JSON examples, verified pairwise and bilingual equality, and checked the four new documents' local Markdown links and trailing whitespace. All passed. Cargo/runtime/package tests were not run because only planning Markdown changed.

Worktree-only state: `HEAD` remains the baseline above; `committed=none-for-this-task`, `staged=none`, `unstaged=none`; task-owned untracked files are this plan and design in both languages. The carried-in unrelated research file remains untracked and excluded, SHA-256 `aa18f7ec8106c34ae194d8d23e6657af6559be78e79a84e266a710bca5cb2824`. The ledger retains its baseline hash. Test impact: `none`.
