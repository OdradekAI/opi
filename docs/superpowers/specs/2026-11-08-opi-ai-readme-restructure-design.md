# opi-ai Public README Restructure Design

Status: approved for planning
Date: 2026-11-08
Owner: Codex for user

## Goal

Restructure the `opi-ai` package documentation so that `README.md` and
`README.zh.md` function as Pi-style user/developer manuals. The README must
lead an external Rust developer from installation to a working call, then to
the next capability they need. Detailed current contracts remain available in
package-local topic documents; implementation history, acceptance snapshots and
maintenance records remain in the iteration documentation.

The change is documentation-only. It must not change Rust APIs, runtime
behavior, Cargo features, provider semantics, test ownership or the external
consumer proof contract.

## Evidence and constraints

The public contract is defined by `docs/iterations/opi-ai-1-5-alignment/spec.md`
and `decisions/09-readme-contract.md`:

- the README follows a Pi-style task order;
- common capabilities keep usable examples and important behavioral limits;
- long protocol and lifecycle details belong in topic documents;
- exact public signatures belong in rustdoc;
- private implementation and historical acceptance belong in engineering and
  iteration documents;
- English and Chinese documentation share the same tested examples;
- external consumers must be able to use the packaged crate without workspace
  paths or implicit initialization.

The upstream Pi 1.5.0 README provides the ordering reference: installation,
quick start, providers/models, auth, tools, image/classification, reasoning,
telemetry/errors, custom providers, Faux, serialization, OAuth, migration and
development.

The current `opi-ai` README is roughly 2,900 lines and mixes onboarding,
public capability contracts, private transport/schema details, test support,
provider implementation records and historical acceptance prose. Existing
package-local `cost-pricing` and `provider-auth` references already establish
the topic-document pattern.

## Information architecture

### Main README

The two localized README files will use the same section order and the same
public examples:

1. **Guide and support boundary** — one short table of goals and links,
   package/version/support notes, and a direct statement of what `opi-ai` does
   and does not own.
2. **Installation and feature profiles** — complete external `Cargo.toml`,
   default and `default-features = false` behavior, and the command used to
   run the packaged examples.
3. **Quick Start** — one complete single-provider call using the existing
   `examples/quick_start.rs` source.
4. **Streaming and final results** — the existing stream source, terminal
   event/result rules, independent start versus consumption, cancellation and
   cleanup constraints that a caller must understand.
5. **Context, tools and continuation** — the existing tool-loop source,
   canonical transcript rules, tool-result continuation, malformed argument
   behavior and the boundary between caller-owned tool execution and SDK
   transport.
6. **Providers, models, auth and catalog** — concise public entry points and
   selection rules, linked to `docs/provider-auth*.md` for factory, credential,
   refresh and catalog detail.
7. **Images, classification and reasoning** — public capability matrix,
   minimal usage constraints and links to provider-specific topic material.
8. **Options, stop reasons, errors, cancellation and usage** — stable caller
   semantics, typed failures, cancellation ownership, terminal vocabulary and
   best-effort cost summary linked to `docs/cost-pricing*.md`.
9. **Telemetry** — explicit context, optional tracing feature, observer
   lifecycle and host-owned exporter/flush policy.
10. **Custom providers, Faux and deferred** — the public extension/test
    boundary, controlled Faux usage, and deferred submit/fetch/cancel semantics.
11. **Serialization, migration and development** — canonical context/replay
    summary, public migration notes, rustdoc/source navigation and a short
    testing/package-proof entry point.
12. **License**.

The README will not repeat the full cost/pricing or Provider/Auth references.
It will retain stable headings or redirects only where existing consumers may
have linked to them, and those redirects will point to the package-local topic
files.

### Package-local topic documents

The following bilingual topic documents own current detail that is useful to
an integrator but too deep for the onboarding path:

- `docs/cost-pricing.md` and `.zh.md`: estimator, sampling, pricing tiers,
  cache pricing, service tier and provider-reported cost.
- `docs/provider-auth.md` and `.zh.md`: provider factory, model/catalog,
  credential, OAuth, request enrichment, refresh and OpenAI-compatible rules.
- `docs/transport.md` and `.zh.md`: proxy, source-scoped HTTP/TLS, WebSocket
  upgrade, resource generation/cleanup and deferred wire constraints.
- `docs/utilities.md` and `.zh.md`: public schema/tool validation, transcript
  and replay projection, headers, identity, diagnostics, rendering, retry and
  callbacks.
- `docs/providers.md` and `.zh.md`: provider-specific wire behavior for image,
  classification, native options and direct adapters; this file links back to
  the provider/auth and transport owners instead of duplicating them.
- `docs/testing.md` and `.zh.md`: Faux/MockProvider usage, external package
  proof, test-layer ownership and maintainer-facing verification entry points.

A topic document may describe a current public contract or a maintainer
workflow, but it must not become a chronological task ledger. Historical
acceptance IDs and phase snapshots remain under
`docs/iterations/opi-ai-1-5-alignment/`.

### Rustdoc and internal records

Rustdoc remains the source for exact signatures, trait bounds and exhaustive
symbol-level API detail. The README and topic files link to it rather than
copying every field or implementation invariant.

Private protocol ownership, compression internals, private schema/crypto
engines, module file layout, MockProvider implementation details and raw
acceptance evidence are not public onboarding content. Keep them in rustdoc
only when they describe a public symbol; otherwise keep them in tests,
engineering docs or iteration evidence.

## Migration rules

Each current README section is assigned one of four actions:

- **retain**: keep a short, task-oriented contract and one usable example;
- **condense**: preserve the public behavior and limits, then link to a topic;
- **move**: copy current detail into the named bilingual topic and replace the
  original section with a short link;
- **remove duplicate**: delete repeated examples or tables after confirming
  that the canonical topic and link cover the same contract.

No section is deleted solely because it is long. Before moving content, record
its destination and preserve any behavior statement that affects a caller's
result, timing, cancellation, error, resource or serialization semantics.

The existing three onboarding sources remain canonical. Any code block that is
advertised as a complete onboarding program must embed the exact source from
`crates/opi-ai/examples/` in both localized README files.

## Verification contract

Add or update deterministic checks in
`scripts/pi-sdk-conformance/test_run_readme_examples.py` to require:

- the exact section order and the Guide/Installation/Quick Start/stream/tools
  entry points in both languages;
- every moved topic has a reachable bilingual link from the README;
- private implementation headings and historical alignment labels do not appear
  in the public README or package-local public topics;
- the three canonical examples are byte-for-byte embedded in both README files;
- the existing external package proof still runs all three examples in both
  feature profiles.

Run the affected checks after each migration batch:

```text
python -B -m unittest discover -s scripts/pi-sdk-conformance -p 'test_*.py' -q
python -B scripts/opi-doc-check.py
cargo fmt --all -- --check
git diff --check
cargo package -p opi-ai --allow-dirty --list
python -B scripts/pi-sdk-conformance/run_readme_examples.py --offline \
  --scratch <new-directory> --target-dir <marked-cargo-cache>
```

The final acceptance result is documentation-only when all links, tests,
format checks and the packaged consumer proof pass. A runtime/API regression
or a missing public capability is outside this design and must be handled as a
separate implementation ticket.

## Non-goals

- Do not change `opi-ai` Rust APIs, feature names, dependencies or runtime
  behavior.
- Do not remove public capability coverage merely to shorten the README.
- Do not rewrite internal issue/report/evidence history into public prose.
- Do not claim unimplemented Pi or pi-coding-agent behavior.
- Do not add a new crate or split the repository as part of this documentation
  migration.
