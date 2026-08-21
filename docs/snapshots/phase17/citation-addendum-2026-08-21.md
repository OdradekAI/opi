# Phase 17 citation addendum — 2026-08-21

Dated companion to the archived ledger
`docs/snapshots/phase17/opi-impl-state.json`. The archived ledger is a
historical record and is NOT rewritten; this addendum maps its stale
criteria_trace citations to their current locations, records corrected
residuals, and records deferred spec divergences. Produced by the
2026-08-21 remediation cycle (`docs/snapshots/phase17/remediation-plan.md`,
sources `audit.codex.md` + `audit.glm5.3.md` at `a680c5d`).

## 1. Stale citation map (archived ledger -> current code)

| Archived citation | Current location | Note |
|---|---|---|
| `agent.rs:62` (collection field, P17-PRV-001) | `crates/opi-agent/src/agent.rs:547` | field moved by post-exit restructuring |
| `agent_loop.rs:733-761` prepare/apply (P17-OUT-002, NXT-003..006) | `crates/opi-agent/src/agent_loop.rs:1065-1193` | candidate build + validation at 1085-1099; atomic apply is `std::mem::replace(&mut state, candidate)` at `:1099` |
| `execute_tool` at `agent_loop.rs:1100-1288` (P17-OUT-003) | `preflight_tool` `agent_loop.rs:1672-1843` + `execute_prepared_tool` `:1847-1960` | split by parallel-batch restructuring |
| `RUN_ID_COUNTER` (P17-EVD-001) | removed; UUIDv7 `IdentityAllocator` | `RunId = uuid::Uuid::now_v7()` (`crates/opi-agent/src/evidence.rs:149`, v7 validation `:96-98`) |
| `require_complete` (P17-EVD-003, P17-OUT-004, 17.7 checkpoint) | removed; `ManifestCandidate::validate` + opaque `FinalizedManifest` | `crates/opi-agent/src/evidence.rs:2529` / `:1847` (private tuple field; external mutation unrepresentable) |
| `phase17_cross_mode.rs:134-529` "each mode dispatches alpha exactly once" (MIG-005, A14) | `phase17_cross_mode.rs:456-876` | HEAD asserts exactly TWO wire dispatches per mode, evidence-kind equality across capture families, durable manifests + denial per durable mode; TUI consecutive-runs test at `:1370-1454` |
| `phase17_cross_mode.rs:65-80` (PLT-002 hermeticity scan) | line drift only | substance unchanged |
| `evidence_runtime.rs:319` (P17-A12/OUT-003 stale-allow evidence) | stale | current substrate mechanics at `agent_loop.rs:1492-1600`; reauthorization recovery leg now also tested (this cycle) |

## 2. Corrected residuals (recorded residual text vs HEAD)

- **P17-PRV-005**: the recorded residual ("auth_source populated accurately
  only in tests/OAuth-route registrations") overstates the inaccuracy at HEAD.
  `prepare_call` applies route-level `Static` only when the resolver returned
  default provenance (`crates/opi-ai/src/provider_collection.rs:436-459`,
  added by `211aba8`), and all production resolvers report truthful sources
  (`credential_store.rs:1036-1068`, `:1413-1488`;
  `provider_factory.rs:2011-2031`). Only genuinely static registrations label
  `Static`.
- **P17-MIG-005 cross-mode asymmetries**: both recorded residuals ("interactive
  binary wires no capture"; "RPC binary does not forward `--trace`") are closed
  at HEAD — all product modes forward `--trace`
  (`crates/opi-coding-agent/src/main.rs:911`, `:1103`, `:1259-1264`).
- **P17-PRV-003/A03 wording**: "exactly ONE Provider record" should read "one
  distinct Provider call identity". The cited test proves identity uniqueness
  via `HashSet` (`crates/opi-agent/tests/evidence_runtime.rs:1050-1062`) with
  multiple lifecycle records sharing it by design
  (`phase17_product_evidence.rs:1981-1982` asserts two records, one identity).
- **P17-PRV-006 adapter count**: `per_request_auth.rs` drives FOUR adapters via
  wiremock (anthropic ×2, openai_chat ×2, openai_responses ×2, codex ×1) plus
  one collection-level dead-endpoint test — not five. Complementary boundary
  coverage exists in the gemini/azure/vertex/bedrock fixture suites.
- **In-adapter capability preflight inventory** (audit INFO-04 correction):
  four adapters run `validate_request_capabilities` in `stream_prepared`
  (anthropic, openai_chat, openai_codex_responses, openai_responses); gemini,
  azure_openai, vertex AND bedrock lack it; `api_mapped` performs the
  equivalent model-resolved `validate_request_for_model` before delegating.
  Collection-level validation in `prepare` covers every dispatched call, so
  the gap affects direct library use only.

## 3. Deferred divergences (post-exit spec tightenings)

`93d75f4`/`aff8875` (2026-08-20) tightened PRIN-004, INV-005, and INV-007 six
days after phase exit (2026-08-14). Phase 17's registered contract predates
them; these divergences are recorded for the spec-owning flow:

- **INV-007 entry classification (MIN-13)**: unimplemented. Unknown session
  entry types are skipped, counted, never fatal (`session.rs:18-23`,
  `:516-533`) and not preserved on rewrite (`:277-279`). Compliance requires a
  durable envelope-format decision contradicting the documented additive
  "never fatal" policy — needs a scoped specification change, not remediation.
- **PRIN-004/INV-005 registration-order permutation tests (MIN-15)**: no test
  permutes authority-contributor registration order and asserts decision
  invariance. The only registration-order test pins hook CALL order
  (`extensions.rs:712`), not decision invariance. Deferred to a tasked phase.
- **Project-trust conflict rule (MIN-16)**: conflicting decided resolver votes
  resolve first-registered-wins (`project_trust.rs:478-484`), pinned as
  intended by `project_trust_store.rs:786-828`; tightened PRIN-004 demands
  most-restrictive independent of registration order. Latent (the standard
  CLI registers no resolvers, `main.rs:334-335`). Requires a spec-owner
  decision to reconcile or except.
- **Failed-run rollback residual (MIN-24)**: `run_with_token` discards all
  in-run turns on ordinary failure including `CredentialNeeded`
  (`agent.rs:950-963`; documented intentional rollback contract), and
  `retry_last_prompt` re-runs without rewind (`harness.rs:2651-2668`). The
  re-execution hazard is gated by `!output_began` (`interactive.rs:770`),
  which does NOT flip on tool-call-only turns (`ToolCallEnd` emits no
  `MessageUpdate`), so tool-only prefixes remain exposed. Secondary effect:
  `persist_turn` is skipped on error, so in-run usage/cost never reaches
  session totals. A fix needs new mechanism (retry-time idempotency or
  read-only retained turns) — deferred.
- **Repo-wide phase-history comments**: 119 phase-14..17 references exist in
  production `crates/*/src` beyond the 15 Phase-17 sites remediated this
  cycle (e.g. `opi-tui/src/trust_prompt.rs:1`, `opi-sandbox/src/cli.rs:2,4`).
  Out of this cycle's finding scope; noted for a dedicated cleanup.

## 4. Evidence currency (audit M1/M2)

Three-platform CI evidence run 31798070731 covers exit SHA `40f2e6e` only. At
audit HEAD `a680c5d`: 5 unpushed commits (`211aba8..a680c5d`), zero CI runs;
the glm5.3 audit's local runs (621 focused tests across 26 phase17 binaries,
fmt, clippy-lib, doc-check; Windows) were the only validation attaching to
`a680c5d`. Post-remediation CI status: see the remediation commit and the CI
run recorded for the final SHA.

## 5. Remediation cycle record (2026-08-21)

Executed per `remediation-plan.md` (all layers gated green locally on
Windows): glm5.3 M3+M4 (dispatchability validated in `try_configure_model`;
resume `.expect` replaced with the typed diagnostic branch; all four
resume/fork/builder call sites covered by tests), codex AUD-17-005 (bare-model
unique-route enumeration on the write path, ambiguity/missing typed errors,
pinned test revised), MIN-12 (in-band stream Error terminal fails the run,
non-retryable, partial message retained), MIN-23 (CLI startup model
validation, typed exit), AUD-17-007 (value-pattern secret scrubbing at the
public event boundary + NDJSON/RPC terminal-shape canary tests; conversation
echo recorded as intended), MIN-07 (six adapters enforce the prepared
`AuthScheme`), MIN-18 (in-memory sink fails closed before setup; both shared
conformance contracts gained the before-setup leg), MIN-09 (ActiveSnapshot
manifest-rejection restored at candidate level and in the product test),
MIN-01/AUD-17-003 (15 Phase-17 comment sites rewritten), MIN-11 (guidance
lockstep contract made truthful for the CLAUDE.md symlink; doc-check
de-vacuated), MIN-08/INFO-03 (truthful AUT-003 phrasing; spec-line citations
replaced), MIN-02/03/04/06 (completion-helper extraction; recorder
delegation; adoption-tail extraction; speculative seam + dead producer chain
removed), MIN-14/17/19/20/21 + INFO-12 (test-strength fixes incl. the
unsupported-version fixture and the saturation test), MIN-22 (dropped
extra-route diagnostics), INFO-16 (measured artifact values), INFO-27
(`tree_read_error` summary redaction), INFO-17-adjacent (unused
`tracing-subscriber` dependency removed), AUD-17-001/006 (conformant
semantics pinned by tests; one-sentence design-spec clarifications added to
the EVD-009 and AUT-008 rows), AUD-17-002 (two same-crate closed enums
un-`non_exhaustive`d; the two auth enums keep the attribute as documented
fail-closed conversion design), AUD-17-004 (dead `dir()` removed;
`completed_run_dirs` documented as a verification seam).

Deferred items and no-action Infos are recorded in the plan's Scope
exclusions; the deferred spec-divergence set stays in section 3 above.
