# Comparison dimensions

The comparison surface for a terminal coding-agent project. Every full audit
measures each dimension below on BOTH projects from current source; a scoped
audit covers only the named slice. Adapt the list when the target's domain
differs.

For each dimension, record each project's current state with a `file:line`
anchor (or `absent: searched <paths>`), then write the objective differences.

1. **Provider abstraction & dispatch** — provider trait/interface, the
   collection/registry that routes requests, the factory, model lookup, and the
   dispatch path to the wire.
2. **Auth & credentials** — credential sources, persistence/store, OAuth,
   refresh, inspectability without a request, logout.
3. **Provider catalog** — built-in count, first-class vs compatibility-profile,
   multi-API providers, which carry OAuth.
4. **Stream/transport lifecycle** — stream event shape, retry/backoff location,
   cancellation, proxy, usage/cost placement, response-id capture,
   cache-control, capability preflight.
5. **Agent loop & harness** — the loop, hooks, the stateful agent wrapper, the
   generic harness (phases/snapshots/save-points/pending-writes), and whether
   the product turn loop actually transits the generic harness.
6. **Session model** — format/version, entry types, migration, tree/branch/fork/
   leaf, compaction algorithm, context reconstruction, storage abstraction.
7. **Extension/plugin surface** — custom tools/commands/providers, hooks
   (including provider request/response), state, UI components, message
   renderers, event bus, sub-agents, dynamic vs startup-locked registration,
   out-of-process adapter protocols.
8. **Built-in tools** — the set, mode-aware policy/mutating opt-in, pluggable
   remote operations, mutation queueing, per-tool hardening.
9. **Terminal UI** — renderer, components, pickers, terminal image protocols,
   themes, keybindings, extension UI injection points.
10. **Skills & prompt templates** — loading/parsing, discovery layers, argument
    substitution, system-prompt emission, whether model-invocable, layering
    (core runtime vs product).
11. **CLI modes** — interactive, print/non-interactive, json/ndjson, rpc,
    list-models, completions, doctor, session list/resume/fork/delete, export.
12. **Packaging & update** — install/remove/update/list/doctor, self-update,
    npm/gallery, trust/source-integrity model.
13. **Export & share** — output formats, redaction, branch/tree scopes,
    web/share/publish.
14. **Permissions & sandbox** — in-process tool policy, containerization
    patterns, project-trust model, fail-open vs fail-closed hooks.
15. **MCP** — core runtime/client/server vs adapter/example only.
16. **Image generation** — distinct from image *input*: a generation surface.
17. **Diagnostics & observability** — doctor/health checks, trace envelopes,
    redaction core, error taxonomy.
18. **Wire protocols & embedding** — schema-versioned NDJSON/SDK/RPC/trace, the
    SDK embedding API, RPC client, browser/web target.

## Execution

For a full audit, fan out across dimensions: one measurer per dimension reads
both projects and writes state + deltas; one verifier per dimension then
red-teams the measurer's output (refute every "lacks" claim by hunting the
source). Measurer and verifier are separate passes — a single pass on "absent"
is not trustworthy. Resume around rate-limits; re-measure any dropped dimension
from source rather than leaving it uncovered.
