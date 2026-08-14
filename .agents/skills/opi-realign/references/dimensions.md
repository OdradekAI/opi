# Comparison dimensions

This is the full inward comparison surface for opi and pi. A scoped audit uses
only the named slice but still measures both projects from current, pinned
source. Adapt a dimension only when the same concept is named differently; do
not replace it with an outward feature proposal.

For every dimension, record both projects with `file:line` anchors (or
`absent: searched <paths>`), then write objective differences.

1. **Provider abstraction and dispatch** - provider interface, registry,
   factory, model lookup, and dispatch path to the wire.
2. **Authentication and credentials** - sources, persistence, OAuth, refresh,
   inspectability without a request, and logout.
3. **Provider catalog** - built-in breadth, first-class vs compatibility
   profiles, multiple APIs per provider, and OAuth-bearing providers.
4. **Stream and transport lifecycle** - event shape, retry/backoff,
   cancellation, proxying, usage/cost, response IDs, cache control, and
   capability preflight.
5. **Agent loop and harness** - loop, hooks, stateful agent wrapper, generic
   harness, save points/pending writes, and the product loop's actual path.
6. **Session model** - format/version, entries, migration, branches/forks,
   leaves, compaction, reconstruction, and storage abstraction.
7. **Extension and plugin surface** - tools, commands, providers, lifecycle
   hooks, state, UI/message renderers, events, subagents, registration timing,
   and out-of-process adapters.
8. **Built-in tools** - set, mode policy, mutation opt-in, remote operations,
   mutation queueing, and per-tool hardening.
9. **Terminal UI** - renderer, components, pickers, image protocols, themes,
   keybindings, and extension injection points.
10. **Skills and prompt templates** - parsing, discovery layers, argument
    substitution, system-prompt emission, model invocation, and placement in
    runtime vs product/package layers.
11. **CLI modes** - interactive, non-interactive, JSON/NDJSON, RPC, models,
    completions, doctor, session commands, and export.
12. **Packaging and update** - add/remove/update/list/doctor, self-update,
    package/gallery distribution, and trust/source integrity.
13. **Export and share** - output formats, redaction, branch/tree scopes, and
    web/share/publish surfaces.
14. **Permissions and sandbox** - tool policy, containerization patterns,
    project trust, and fail-open/fail-closed boundaries.
15. **MCP** - core runtime/client/server behavior vs adapters/examples only.
16. **Image generation** - generation surfaces, distinct from image input.
17. **Diagnostics and observability** - health checks, traces, redaction,
    errors, and operator-visible diagnostics.
18. **Wire protocols and embedding** - versioned NDJSON/SDK/RPC/trace formats,
    embedding APIs, clients, and browser/web targets.

## Execution

For a full audit, process dimensions in bounded batches sized to available
worker capacity and reserve one slot for coordination. A measurer reads both
projects and writes state plus deltas. A different verifier then red-teams that
output, hunting the source behind every `lacks` claim.

Measurement and verification are separate passes, but they do not need to run
concurrently and must not exceed the host's worker limit. Resume around rate
limits. Re-measure a dropped dimension rather than leaving it uncovered. When
independent workers are unavailable, label the fresh second-pass verification
honestly instead of claiming independence.
