# Comparison dimensions

Use these stable numbered dimensions as discovery prompts, not a closed feature
list or a predefined package graph. The supplied snapshot defines what exists.
Inventory its public surfaces and actual consumer paths before filling coverage;
add a snapshot-grounded surface if no existing subcriterion fits. A scoped audit
uses the requested slice and its necessary dependencies, without claiming full
project coverage. Scope comes from the invocation, not Opi planning documents.

Record both projects for each applicable subcriterion, with contract IDs and
`file:line` anchors or bounded absence searches. Several dimensions may refer to
one contract: keep one evidence record, not duplicate counts. Distinguish source,
public package, experimental, example, tooling, and actual product exposure.

## Dimensions

1. **Model runtime and public operations** — provider factories and registration,
   lookup versus real provider/API/operation dispatch, direct/runtime/compat
   entry points, typed/custom operations, capability checks, frozen request
   state, and selected versus actually dispatched model identities.
2. **Authentication and credentials** — source/override precedence, API keys,
   ambient credentials, OAuth login/refresh/logout, side-effect-free status
   checks, rotation concurrency, cancellation versus required persistence,
   account isolation, callback security, and host-specific flows.
3. **Provider and model catalogues** — provider/API breadth, chat/image/classifier
   records, static versus dynamic data, schema/generator inputs and hashes,
   prices/capabilities, overlays, filtering, cache/ETag/offline behavior,
   refresh races, store publication, and stale-generation rejection.
4. **Streams, transport, and request lifecycle** — event/partial/result contracts,
   fragmented input, stop/terminal distinctions, deferred fetch/cancel, callbacks,
   retries and strict fallback, proxy/header rules, distinct timeout phases,
   cancellation/drop, cache/connection ownership, usage and billed versus
   estimated costs. Compare valid provider/API/transport/auth combinations.
5. **Agent loop and harness execution** — stateful wrapper, request preparation,
   completed-turn decisions, steering/follow-up queues, tool continuation,
   termination, transactional next-turn replacement, context/model updates,
   cancellation and budget ownership. Trace generic loop, product harness and
   any alternative experimental harness separately; do not assume old exports
   or hook names remain present.
6. **Sessions and durable execution** — formats/versions, branches/forks,
   reconstruction, compaction, search and storage adapters; task/checkpoint
   scheduling when present, admission deduplication, commit/store/publication
   order, crash/restart/replay, migration, read-only recovery, and external-effect
   guarantees. Transcript persistence alone does not prove durable scheduling
   or exactly-once external effects.
7. **Extensions and plugin runtime** — tools/commands/providers, lifecycle hooks,
   registration and activation, replace/reload/unload, renderers/events/state,
   distributed references and revocation where present, subagent integration,
   in-process versus out-of-process contracts and actual consumers. Separate
   released and experimental interfaces from unexported source implementations.
8. **Tools, orchestration, and execution environments** — available/default tools,
   selection and mutation policy, schema-to-execution path, parallel/sequential
   scheduling, codemode and nested calls, tool search/deferred exposure, worker
   admission versus actual exit, output/store budgets, remote file/process/PTY
   behavior, connection loss, cleanup, and platform limitations.
9. **Terminal UI** — renderer/components, regular/fullscreen modes, pickers,
   keybindings, IME/mouse, themes, images, extension injection, terminal-state
   restoration, escape/control safety, fallback behavior, and streaming/render
   complexity. Rendering library availability is not CLI adoption.
10. **Context and resource assembly** — skills/prompts/context files, discovery,
    arguments, invocation policy, configuration precedence and trust, system
    sections, tool additions/removals, reload, context edits, token estimates,
    model-switch replay, and consistency across live requests and summaries.
11. **CLI and interaction modes** — interactive, print, JSON/NDJSON, RPC, model
    and session commands, completions, diagnostics, output/exit contracts,
    headless trust decisions, cancellation, and shared versus divergent entry
    paths. SDK-only capability is not automatically available from the CLI.
12. **Packaging, installation, and update** — published exports and assets,
    add/remove/update/list/doctor, installation locks, package discovery,
    lifecycle scripts, source integrity, offline builds, native dependencies,
    platform distribution, and workspace-external package consumption.
13. **Export and share** — formats, system/tool history, branch/tree scopes,
    redaction, explicit destination/authentication, failure behavior, and
    web/share/publish consumers. A format encoder is not a complete share flow.
14. **Trust, permissions, and isolation** — project/package trust, per-invocation
    authority, credentials/files/process/network access, container/VM/native
    restrictions, capability transitions and failure policy. Keep permission
    metadata, script isolation, process supervision and OS confinement distinct;
    stronger Opi policy is still a difference when it changes accepted behavior.
15. **MCP** — actual client/server surfaces, versions and negotiation, stdio/HTTP
    transports, pagination/resources, OAuth, session/reconnect/resume/cancel,
    tool exposure and product registration. Check declared unsupported features;
    never inherit an older snapshot's examples-only or built-in assessment.
16. **Typed non-chat model operations** — image generation, classifiers and any
    other snapshot-provided operation; typed models, inputs/results, structured
    answers, probability/usage rules, errors, cancellation, and caller exposure.
    Image input does not prove generation; text generation does not prove a
    classifier implementation.
17. **Diagnostics and telemetry** — errors/health, explicit spans and propagation,
    adapter conformance, metrics/cost attribution, restart correlation, retention,
    queue/drop behavior, exporter failure, cardinality and sensitive content.
    Typed metadata alone proves neither runtime validation nor redaction.
18. **Protocols, SDKs, and embedding** — versioned wire/data contracts, client/server
    and process lifecycles, negotiation/backpressure, external SDK callers,
    browser/worker/native hosts, cross-mode consistency, and public versus
    internal APIs. Inspect an installed artifact when claiming package usability.
19. **Schema and cross-language data contracts** — schema dialect/formats,
    coercion versus strict wire validation, unresolved references, incremental
    parsing versus final validation, missing/null/defaults, open identifiers,
    number/Unicode/order semantics, ownership/aliasing, serialization, and errors.
20. **Evaluation and acceptance infrastructure** — deterministic fixtures and
    differential drivers, actual Agent/grader consumers, cohort/provenance,
    isolation, repetitions, missing/error result semantics, cost/resource
    accounting, artifact privacy and offline recomputation. Keep development
    tooling, public Companion capability and live-product evidence separate.

## Execution

Measure and review bounded batches of related contracts, sized to available
worker capacity. Each handoff carries input identities, owned scope, contract
records, searches, limitations, and expected review checks. Workers do not infer
scope from previous reports or fan out without authorization.

For full audits, reserve coordination capacity; measurement and review need not
run concurrently. Resume using the same identified inputs, remeasuring changed
or dropped surfaces. Apply the framework's independent/self-review and unresolved
rules equally to full and scoped audits. Maintain coverage in report working
material, not a new product-progress ledger.
