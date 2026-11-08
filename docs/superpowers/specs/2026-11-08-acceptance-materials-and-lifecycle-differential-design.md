# Acceptance Materials and Lifecycle Differential Design

## Goal

Freeze the Pi 1.5.0 acceptance inputs that are available in the repository and
produce the first reproducible TS/Rust differential evidence for the highest
risk request-lifecycle behaviors, without changing product behavior or using a
live provider.

## Scope

This slice has two deliverables:

1. A material-freeze evidence file records the exact Git/source identity,
   package and lockfile inputs, toolchain/platform identity, relevant source
   hashes, and the availability or absence of the pinned model-catalog bytes.
   Missing or unreachable materials are recorded as explicit blockers.
2. A lifecycle differential driver runs the existing upstream TypeScript probe
   and a Rust driver against equivalent controlled scenarios, emits normalized
   JSON traces, and compares only facts that both runtimes expose under the
   same scenario. Known semantic differences remain named outcomes rather than
   being silently treated as passes.

The first scenarios are:

- execution continues when the consumer never polls the returned handle;
- execution continues after the consumer stops consuming;
- setup failure reaches a terminal error outcome with its observable error
  channel.

The slice does not run real providers, claim full provider coverage, freeze the
full feature/platform matrix, or decide catalog equivalence when the pinned
catalog response is unavailable.

## Design

### Material freeze

`freeze_acceptance_materials.py` reads only repository-local inputs and performs
one explicit request for the immutable catalog revision declared by
`.pi/pi-1.5.0/nix/model-catalog.json`. It records SHA-256 hashes, file sizes,
presence states, and the command/runtime identity in a versioned JSON evidence
file. The raw catalog response is copied only when its bytes hash exactly to the
declared revision; otherwise the evidence records the HTTP, transport, or hash
failure and leaves the dependent catalog gates blocked.

The freeze covers the upstream source manifest, `packages/ai/package.json`,
`package-lock.json`, the catalog pin, the two lifecycle source files used by the
TypeScript probe, the Rust workspace manifests and lockfile, and the driver
files. It also records whether `node_modules` and `packages/ai/dist` exist,
because source presence does not prove that the published runtime is available.

### Differential trace

The existing `observe_upstream.mjs` remains the TypeScript oracle and is
extended only as needed to emit a stable trace schema. A Rust lifecycle driver
uses the public `opi-ai` Models/ChatCall/StartedChatStream path with a
controlled provider and deterministic barriers. It emits the same scenario IDs,
step names, and normalized counters/results. Runtime-specific fields are kept
under a namespaced `runtime` object so the comparator cannot mistake an
implementation detail for a shared contract.

The comparator reports one row per scenario with:

- `pass` when all shared observations match;
- `intentional-difference` when the spec documents a Rust/JS timing or error
  channel difference and the trace proves it explicitly;
- `fail` for an unexpected mismatch; and
- `blocked` when a required input or runtime is unavailable.

The evidence includes driver hashes, input hashes, exact commands, normalized
traces, comparison rows, and the final classification. It never rewrites the
existing phase-one observations; the new file is a separate proof for the
current inputs.

## Failure and resource policy

The material freezer must fail closed on a catalog hash mismatch and must not
replace the target catalog with current Rust data or a newly discovered live
catalog. The differential driver uses controlled in-process sources, bounded
watchdogs, and explicit cleanup barriers. A process timeout is a failed probe,
not evidence of `pending`. No test reads user credentials or makes a paid/live
model call.

## Verification

The implementation will run the focused Python unit tests for the freezer and
comparator, both TypeScript oracle scenarios, the Rust driver, JSON schema and
hash self-checks, `git diff --check`, and the repository documentation checker.
The final report will state which material gates are frozen, which are blocked,
which differential scenarios pass or differ intentionally, and what remains
unexecuted.
