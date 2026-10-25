# Opi SDK onboarding, configuration and request-parameter design

[简体中文](2026-10-25-opi-ai-onboarding-and-config-design.zh.md) · [Acceptance plan](../plans/2026-10-25-opi-ai-onboarding-and-config-acceptance.md)

## Status and authority

This is a **design proposal for review**, written on 2026-10-25 against Opi
`de703bb12a7602d0f128b22a7cbb4bb6c81f12ab` and ledger SHA-256
`b9273baa5b5751717f6e4fdea7ab9c7fbd5be11a9e1ff7150c6c04454476592a`.
It does not amend a registered specification, admit a task, change a task's
status, authorize implementation, or establish that a proposed API exists.
The current Phase 20 contract remains pinned to Pi 1.3.0. The local Pi 1.4.0
README and product configuration sources supply the comparison and proposed
field vocabulary; new 1.4 behavior needs separate source and graph admission.

Only this proposal and its acceptance plan are delivered now. Production,
current READMEs, registered sources, schemas, Cargo files and the canonical
ledger remain unchanged. No commit, publication or live provider call is part
of this work. The unrelated `pi-1.4.0-upgrade-assessment.md` is excluded.

## Confirmed user decisions

1. The final `opi-ai` README should teach an ordinary developer to use the SDK
   with a first-use experience comparable to Pi AI.
2. TOML remains the default application configuration format. JSON is selected
   explicitly and uses Pi's settings/model fields.
3. **Old Opi TOML field names need no compatibility.** New TOML uses the same
   Pi field names and nesting as JSON, including camelCase. Do not retain old
   aliases, a legacy merge path, or an automatic conversion reader.
4. Provider request behavior should match the adopted Pi contract. Rust types,
   ownership and errors may express it idiomatically; they must not silently
   change defaults, precedence, accepted values or wire behavior.

## Existing sources and observed gaps

The parent [specification](../../opi-spec.md) owns architecture and authority;
the registered [Phase 20 source](../../research/pi-ai-opi-ai-alignment/spec-draft.md)
owns the current SDK contract. In particular, AIR-045/046 and its public SDK
table require ordinary direct calls, default memory stores, lazy sources and
explicit built-in assembly. The [ledger](../../../.opi-impl-state.json) already
assigns factories, native options, external packages and final evidence.

The current crate README puts its minimal example after internal backend
details, exposes manual resolver/route/attempt assembly, and still constructs
removed `Request.system/messages/tools` fields. Current
[`Request`](../../../crates/opi-ai/src/provider.rs) instead owns a canonical
`context`. This is evidence of documentation drift, not permission to rewrite
the current manual before its owning slice is ready.

The product currently loads `config.toml` through `TomlConfig` in
[`config.rs`](../../../crates/opi-coding-agent/src/config.rs). Its CLI has
`-c, --config <FILE>`; there is no config-directory or model-config flag.
Pi separates preferences in `settings.json` from provider/model definitions
in `models.json`. Merely JSON-encoding the current `OpiConfig` is insufficient.

## Chosen approach and alternatives

| Approach | Result |
|---|---|
| Default TOML, explicit Pi JSON, one semantic owner | Chosen. Preserve default discovery locations, replace external vocabulary, and share resolution and validation. |
| JSON only | Simpler parser surface, but does not meet the requested TOML default. |
| Two syntaxes over the old Opi field vocabulary | Does not provide Pi-field compatibility and retains unnecessary migration burden. |

The work has three tracks: SDK onboarding documentation, Reference Product
configuration, and protocol-specific request parameters. They share acceptance
fixtures where useful, not a new SDK facade or configuration framework.

## SDK onboarding and README closure

The ordinary API-key path is: add the crate and documented runtime dependencies,
configure a provider key, select a provider/model, build a conversation Context,
then await completion or consume a stream. It must not require the caller to
construct an auth resolver, mutation queue, product configuration, Agent, or
prepared-attempt lifecycle. These remain advanced extension seams.

`Models::new` may remain an empty collection. A small provider factory or the
explicit built-in assembly supplies catalog, lazy auth and real adapters.
Defaults must not start authentication or catalog network work. A custom store
and presenter are optional for ordinary API-key use. OAuth persistence and UI
remain explicit host responsibilities.

The final bilingual README is ordered as follows:

1. Purpose, installation, supported runtime and platform requirements.
2. One complete quick start, then streaming and the shared final-result path.
3. Provider factories, explicit built-ins and model lookup.
4. Default auth, per-call overrides, stored credentials and optional OAuth.
5. Context, tools, tool-result continuation, model changes and serialization.
6. Native versus simple options, errors, cancellation, usage and cost.
7. Advanced customization and links to protocol/architecture references.
8. Contributor guidance and migration links.

Phase identifiers, AIR/RDF matrices, internal signing/codec maintenance and
historical validation journals belong in their existing design/evidence homes.
Move facts with their conditions intact; do not delete safety or lifecycle
contracts merely to shorten the README. Do not copy Pi's compatibility shim,
browser/Node packaging assumptions or stale prose when Rust/current source
requires a documented difference.

Proposed example sources are `crates/opi-ai/examples/quick_start.rs`,
`quick_start_stream.rs` and `tool_loop.rs`; these paths **do not exist yet**.
They are created under the owning future tasks. The README embeds or extracts
their exact source. The actual packaged crate must be used from a fresh
workspace-external consumer to compile and execute that same example with a
controlled endpoint. A separate complex test consumer is not the first-use
proof. Live success, model output quality and performance are not inferred.

## Product configuration selection and ownership

### Files and explicit JSON selection

Default discovery keeps the current locations: Windows `%APPDATA%/opi`, Unix
`~/.config/opi`, and trusted project `.opi/`. The primary default settings file
remains `config.toml`. Its optional companion `models.toml` contains Pi's
`providers` document. This avoids making a filename rename a hidden condition
of the user's field change.

The existing `--config` option is proposed to select a settings **bundle**:

| Selected settings | Optional companion in that same directory |
|---|---|
| Default user `config.toml` | `models.toml` |
| Trusted project `.opi/config.toml` | `.opi/models.toml` |
| Explicit `--config /chosen/config.toml` | `/chosen/models.toml` |
| Explicit `--config /chosen/settings.json` | `/chosen/models.json` |

These are target semantics, not currently working commands. An explicit JSON
file may have another basename; its role is still settings, not a raw model
document. A model document supplied as the primary settings file is rejected.
Extending the current single-file option to read the named companion is a
visible behavior change and must be included in help and admission review.

Select syntax by the known filename extension, without content sniffing.
JSON is not auto-discovered. Within one layer, never merge both representations
or use JSON failure to fall back to TOML. A missing optional companion is an
empty model layer; a present unreadable, malformed or unsupported companion
fails the selected bundle before configuration is published. A missing explicit
settings file fails. Lower-priority layers may remain TOML when the explicit
layer is JSON. Selecting a bundle does not import its `auth.json`, extensions,
skills, prompts, packages, or instructions.

For default user/trusted-project discovery, a missing settings file is an empty
settings layer, not a reason to skip a present model companion. Model-only
configuration is allowed at those selected locations; project trust still
applies. This differs from an explicitly selected missing settings file.

### One configuration meaning

Both parsers feed one private, presence-aware configuration representation and
one merge/validation owner in `opi-coding-agent`. Preserve field provenance
through selection and diagnostics. `opi-ai` does not acquire a product file
loader or a dependency on `opi-coding-agent`.

Keep the current authority order: built-in defaults, user bundle, trusted
project bundle, explicit bundle, then CLI model override. Retain the existing
`OPI_MODEL` exception: it overrides the lower layers only when `--config` is
absent. This host-specific choice is explicit; it is not called Pi precedence.
Provider/model preferences are separate Pi fields, resolved after merging;
an explicit CLI/env model selector overrides the corresponding pair atomically.

Scalar values use the highest present value. Supported nested settings merge
per field according to the pinned Pi settings mechanism; model-specific maps
use exact provider/model keys. Arrays use their field's policy: resource lists
and model selection are not interchangeable. The compatibility inventory must
name replace/merge/clear behavior for each supported array and model override;
no generic concatenate-every-array algorithm is admitted.

### Field vocabulary and coverage

| New external field/location | Meaning and boundary |
|---|---|
| `defaultProvider`, `defaultModel` | Pi's separate provider and bare model selection; replace the old `model` preference field. |
| `defaultThinkingLevel`, `modelThinkingLevels`, `thinkingBudgets` | Pi names and level vocabulary; neutral policy remains separate from each wire's native parameters. |
| `compaction.*`, `retry.*` | Pi vocabulary; only fields with an admitted host behavior are supported. Provider retry and agent retry remain separate owners. |
| `models.*: providers.<id>` | Pi model document: `baseUrl`, `api`, `apiKey`, `headers`, `models`, `modelOverrides`, protocol-qualified `compat`. |
| `models[].contextWindow`, `maxTokens`, `reasoning`, `input`, `cost`, `thinkingLevelMap` | Pi model metadata, including its actual unit and optionality; not an instruction to enforce every metadata limit at runtime. |
| `opi.*` | One explicit namespace for existing Opi-only execution/authority/limit settings without a Pi equivalent. Preserve their behavior and ownership. No old top-level aliases. |

The proposed host task produces a field-by-field compatibility table from the
pinned Pi schemas and actual loader sources. Every current Opi configuration
capability must map either to its genuine Pi equivalent or to a typed `opi.*`
field; it cannot disappear under this format change. Each recognized Pi field
is classified as implemented, needs a named host capability, or unsupported
with an explicit diagnostic. Do not advertise all of Pi's tools, MCP, codemode,
UI, package manager or settings merely because their JSON parses.

Pi's generated schemas currently allow additional properties. Preserve truly
unknown extension data as inert data with a visible no-effect warning; it must
not flow into request bodies or acquire runtime behavior. Recognized but
unimplemented Pi options fail explicitly. Known retired Opi fields, including
old `model`, `[defaults]`, `api_key_env` and old thinking fields, fail with their
location and replacement guidance. Do not silently ignore them and use defaults.
`$schema` is local metadata, never a request to fetch a remote schema.

TOML has no native `null`. The shared non-null subset must resolve identically
in both formats. Pi's schema-approved null-only configurations, such as an
unsupported `thinkingLevelMap` entry, use explicit JSON. Do not invent a null
sentinel or treat an omitted TOML key as JSON null. This limitation is disclosed
as a representation difference, not called complete TOML/JSON equivalence.

If a host UI later saves configuration, it writes only the selected settings
or model document, in its selected format, preserves inert extension data, and
does not rewrite its companion or the other format without a requested change.
Atomic persistence and permission rules remain with the product owner.

### Credentials and project trust

No real user configuration, HOME, keychain, account or `.pi` extension is read
or executed during this design or its offline tests. Product trust remains
the current Opi rule, including project instructions/resources; Pi's different
context-file trust behavior is not imported. A project still cannot grant
itself persistent execution permission. Pre-trust staging reads only authorized
layers and does not execute key commands or load extensions.

Pi's `apiKey`/header expressions are a distinct, proposed host-source slice:
literal values; `$NAME`/`${NAME}` interpolation; `$$`/`$!` escaping; and leading
`!command`. They are parsed without execution. Actual resolution belongs to the
selected auth/request source, preserving selected identity and typed failures.
The model-value path must use the source's uncached-per-request behavior rather
than copying a different Pi helper's process-global cache. Command execution
requires an admitted host owner for shell selection, frozen cwd/env, cancellation,
tree cleanup, the source's timeout and redacted diagnostics. Do not substitute
the AWS credential-process JSON protocol or an unrelated global auth timeout.
Until that slice passes, command expressions are rejected as unsupported, not
sent as literal secrets or silently ignored. `auth.json` import/export and
replacement of the native credential backend are outside this proposal.

## Provider request-parameter alignment

Keep three boundaries explicit: product settings/model documents, SDK typed
per-call options, and actual protocol requests. `signal`, `fetch` and functional
callbacks are runtime values, not JSON configuration fields. Rust may use
snake_case members, enums and `Duration`; external Pi documents retain their
field names and millisecond units where specified.

The current SDK task owners continue to implement their Pi 1.3 contract. Their
parameter inventory records source/version, native or simple path, unit,
omitted default, zero/false/null behavior, effective precedence, wire field and
expected request/failure. Use source and wire tests when README prose conflicts:
Pi 1.4's Azure README says Responses-only, while its provider still registers
Responses and Chat. Do not remove the admitted Azure dual-wire obligation.

| Family | Acceptance that must remain source-qualified |
|---|---|
| Common/simple | Temperature, output limit, neutral reasoning/tool choice, cancellation, timeout scope, cache and session options; native and simple defaults remain distinct. |
| OpenAI Chat/Responses/Codex | Exact output fields, store omission versus false, reasoning/tool-choice support, service tier, session/cache/transport and per-variant retry/callback behavior. |
| Anthropic | Native thinking disabled/enabled/omitted, source budget defaults, adaptive effort, tool choice and strict variants; no inherited universal budget. |
| Google/Vertex | Native dynamic budget `-1`, disabled `0`, level choices and signatures; Vertex Express/ADC targets and auth remain separately checked. |
| Azure | Both actual wires and correct v1/resource/deployment/API-version mapping. |
| Bedrock | Native inference/additional request fields, SigV4/Bearer and exact retry modes; existing auth acceptance is not full generation acceptance. |
| Mistral/Radius and typed operations | Their own option whitelist, timeout, raw events and typed results; do not force them through a Chat serializer. |
| Headers and callbacks | Set/remove/omit, protected auth identity, source timing and per-variant counts; arbitrary callback data does not replace the selected account. |

Effective options must follow the registered source's merge rules, retaining
explicit false/zero/removal. Auth-derived endpoints are finalized before
endpoint-sensitive compatibility inference. Capture actual method, URL,
headers and body from public calls, including retries; object equality before
the adapter is not enough. Performance claims require separate measurements.

Pi 1.4-only behavior, including client identity and changed whitespace tool
result handling, is a source-delta proposal. Old tests are retired only after
their owning contract is superseded and an equal-or-stronger replacement passes.
Live model-name updates, retired frame/global facade assertions and JS raw-error
leaks are not copied mechanically. Reuse the existing acceptance matrix and
test inventory; this document is not a second delivery ledger.

## Task ownership and dependency plan

| Disposition | Owner | Addition or preserved responsibility |
|---|---|---|
| Retain | 20.32 / 20.33 | Authentic packaged catalog / atomic dynamic catalog owner. |
| Supplement acceptance | 20.34 | Factories, canonical IDs and profile-specific effective options through actual requests. Not the host file-format migration. |
| Realize existing contract | 20.35, 20.36–20.44, 20.51 | Parameter inventory, simple/native mapping, each real adapter and headers/callbacks. Keep their existing prerequisites. |
| Retain protocol ownership | 20.45–20.49 | Typed image/classifier/Decisions parameters and requests stay with their existing operation owners. |
| Retain prerequisites | 20.50, 20.52–20.55 and existing dependent tasks | Deferred/retry/transport/telemetry contracts are not waived by this onboarding plan; 20.54 feeds adapters and 20.55 precedes 20.56. |
| Supplement acceptance | 20.56 | Bilingual user-facing README organization, removal of stale examples and links to owning deep documentation. Not new host behavior. |
| Supplement acceptance | 20.57 | Same-source quick-start examples from the actual external package; ordinary SDK calls without product configuration. |
| Retain | 20.58 | Native/MSRV/features/package evidence supporting published platform claims. |
| Supplement handoff | 20.59 | Final bilingual manual and truthful evidence; retain its source-drift reconciliation and independent Phase-exit boundary. |
| New, not numbered or admitted | Reference Product configuration slice | Default TOML, explicit JSON bundles, Pi field vocabulary, old-field rejection, shared merge, trust and real product request integration. |
| New, not numbered or admitted | Reference Product configuration-value sources | Pi key/header expressions and owned command lifecycle; consumes the configuration slice. No auth-file migration. |
| Source review before adoption | Pi 1.4 delta | Exact additional fields/behavior and associated tests; do not relabel old passing evidence. |

The SDK track follows existing dependencies through parameter mechanisms,
adapters, 20.34, 20.56, 20.57, 20.58, 20.59 and independent Phase exit. The host
configuration track consumes admitted factories/options and has its own
configuration-to-request acceptance. It does not become a prerequisite for
standalone SDK use. A path already listed in a task does not enlarge its DoD.

Before implementation, materialize approved meaning in the owning registered
sources, review the source delta and proposed graph, and obtain the existing
human graph confirmation through explicitly invoked `opi-implement plan`.
Preserve current task IDs, original evidence and source pins until that approved
reconciliation. Do not patch hashes alone or preassign a new Phase 20 number.
Task 20.59 remains a documentary handoff; F.1a independently judges full exit.

## Delivery criteria for this proposal

The paired acceptance plan defines concrete positive/negative cases and future
verification commands. This planning delivery requires synchronized English and
Chinese documents, verified local references, no unresolved placeholders or
hidden claims of implementation, a passing documentation checker and clean
diff whitespace. Runtime, package and platform commands in the plan are future
acceptance, not executed evidence. Test impact for this planning change is `none`.

## Local comparison-source identity

The following SHA-256 values pin the relevant local Pi 1.4.0 comparison inputs,
not an upstream Git revision or a new registered Phase 20 baseline. Paths are
relative to `.repo/pi-1.4.0/`.

| Source | SHA-256 |
|---|---|
| `packages/ai/README.md` | `a3c2b8de70b396187b6e12ed679624ba1c3c1c9c028e90a9aad69cd46dd84e8f` |
| `packages/ai/src/types.ts` | `9f7346dfd52bda4f25e527d768c67a210c88fb9dc9c4a16610b693087e626319` |
| `packages/coding-agent/schemas/settings.schema.json` | `d4b19b0e492cb887a23c50034f00213c476048956fa92c5cce2f1536ef50b9b0` |
| `packages/coding-agent/schemas/models.schema.json` | `86ade9716724902bdfe91347a728980ad7242a76d0773dbc7217ce4584d696ea` |
| `packages/coding-agent/src/core/settings-manager.ts` | `aef8fea51000f291c1c51d81fd9b30f7a2bdd0e68370ad84b969e5fd15667e98` |
| `packages/coding-agent/src/core/model-registry.ts` | `f4ced2d92df9097647aba7da3ae0bb273fa8089e966be15dba880db51d5dee0b` |
| `packages/coding-agent/src/core/resolve-config-value.ts` | `0f53dad47fe5d5d8837c022b7951ccd3bd5a9b577bd662f0986272110e83bcc7` |
