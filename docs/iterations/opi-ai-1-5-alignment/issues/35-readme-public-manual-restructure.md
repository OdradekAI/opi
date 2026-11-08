# Restructure the public opi-ai manuals around developer tasks

Status: done
Owner: Codex for user
Parent: [opi-ai 对齐 Pi 1.5.0：重构方案与实施交接](../spec.md)
Depends on: [Remove historical alignment labels from public opi-ai manuals](34-remove-internal-labels-from-public-manuals.md)

## Scope

The public README was still carrying implementation-order chapters for
transport, schema, provider adapters, test support and OAuth. This slice keeps
the public behavior contract while making the README follow the Pi-style task
order from installation through development.

## Source-to-destination map

| Former public content | New owner |
| --- | --- |
| onboarding, streaming, context and tool-loop examples | `README.md`, `README.zh.md` |
| proxy, TLS, WebSocket, deferred transport and cleanup | `docs/transport*.md` |
| schema, replay, transcript, identity, diagnostics, retry and callbacks | `docs/utilities*.md` |
| provider-specific wire, image, classifier and native options | `docs/providers*.md` |
| Faux, controlled peers, test layers and package proof | `docs/testing*.md` |
| provider/auth/catalog/refresh and cost/pricing | existing `docs/provider-auth*.md` and `docs/cost-pricing*.md` |
| implementation history and acceptance evidence | `docs/iterations/opi-ai-1-5-alignment/` |

## Implementation

The bilingual README files now expose the same task order: installation,
feature profiles, Quick Start, streaming/result, context/tools, providers,
images/classification, reasoning, errors, telemetry, custom providers, Faux,
deferred, migration and development. The three packaged examples remain the
canonical complete programs and are embedded byte-for-byte in both manuals.

Long current contracts moved to four bilingual package-local topics. Private
implementation headings, module-layout details, raw acceptance narratives and
historical alignment labels no longer appear in the public README or topics.
The runtime, Rust API, Cargo features and provider behavior were not changed.

## Acceptance

`test_readmes_follow_pi_style_public_task_order` fixes the bilingual public
heading sequence. `test_public_readmes_do_not_expose_private_implementation_sections`
requires all four topic links and rejects private sections. The topic contract
and alignment-label tests cover all six bilingual topic pairs plus the three
examples. The full conformance suite, `opi-doc-check.py`, `cargo fmt --all --
--check` and `git diff --check` must pass; the final gate is the offline
external packaged-consumer proof in
`scripts/pi-sdk-conformance/evidence/readme-examples-proof.json`.

The final run passed 105 conformance tests, the documentation and formatting
checks, the packaged file-set check and the external consumer proof in both
feature profiles. Both localized manuals reported exact example embedding.

## Limits

This slice changes documentation structure and documentation contract tests
only. It does not claim complete Pi or pi-coding-agent capability alignment,
real-provider availability, new public convenience APIs or a new crate split.
