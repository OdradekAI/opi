# Audit inputs and shared rules — opi-realign 2026-11-06

This file is the single source of input identity for every measurement batch.
Read it fully before measuring. Do not modify it.

## Inputs

| Field | Value |
|---|---|
| Measurement date | 2026-11-06 |
| pi root (target snapshot) | `D:\Luiz\Odradek\opi\.pi\pi-1.5.0` (POSIX in WSL: `/mnt/d/Luiz/Odradek/opi/.pi/pi-1.5.0`) |
| pi identity | Export WITHOUT its own `.git`; it is nested in the Opi repository, so `git -C .pi/pi-1.5.0` resolves to the **Opi** repo — that is NOT pi provenance. Upstream SHA/remote: unverified (export). |
| pi declared versions | All 12 packages `1.5.0` (`@earendil-works/pi-agent-core`, `pi-ai`, `chord`, `pi-codemode`, `pi-coding-agent`, `pi-durable`, `pi-env`, `pi-evals`, `pi-mcp`, `pi-schema`, `pi-telemetry`, `pi-tui`); monorepo root `pi-monorepo` 0.0.3 |
| pi content identity | 2949 files (excluding `.git`, `node_modules`), sorted relative-path sha256 manifest, aggregate `sha256 = 8aefc4c29d420c27` (first 16 hex) |
| pi artifact availability | Source-only export: no `node_modules`, no built `dist`, no `.artifacts`. Generated model-catalog SOURCE is present (`packages/ai/src/providers/*.models.ts`, `packages/ai/catalog/` generators; ~240K). Runtime-generated data absent. |
| Opi root (current) | `D:\Luiz\Odradek\opi` (WSL `/mnt/d/Luiz/Odradek/opi`) |
| Opi identity | git HEAD `068f1cf`, tree `d382688b11cee87fc1970dc22666345dd06145fb`, **clean** working tree at audit start |
| Opi declared versions | workspace `0.8.3`; crates: opi-ai, opi-agent, opi-coding-agent, opi-eval, opi-protocol, opi-sandbox, opi-tui (edition 2024, MSRV 1.97) |

## Independence constraints (mandatory)

- The assessment lens is **full reproduction of the pi 1.5.0 snapshot within
  assigned scope**. Do NOT consult `docs/opi-spec.md`, `docs/CONTEXT.md`,
  `.opi-impl-state.json`, `docs/snapshots/`, ADRs, roadmaps, or any file under
  `docs/realign/` other than this evidence directory.
- `CHANGELOG.md` and shipped docs/READMEs/tests in either tree are allowed as
  **current implementation facts** only.
- No network access, no installs, no running the snapshot's build/tests, no
  modification of either source tree. Read-only inspection (`read`, `grep`,
  `rg`, `find`, `ls`) plus writing YOUR ONE evidence file.
- Do not infer scope from Opi's plan; absence of an Opi roadmap entry never
  explains a gap. The snapshot defines the target.

## Comparison verdicts

`equivalent | partial | missing | different | opi-only | unresolved` — strict
reproduction lens; deliberate/safer behavior is still `different`. Module
layout differences alone are NOT behavioral differences.

## Record format (per contract)

```md
### <ID> — <short name>
- Dimension(s): <numbers from the 20-dimension list>
- pi entry: <path:line> (public/internal/tooling/experimental + exposure)
- Opi entry: <path:line> or bounded absence search (commands + terms + scope)
- Comparison: <verdict>
- Observable difference: <how a caller can distinguish; parameters/defaults,
  ordering, state, errors, cancellation, cleanup>
- Evidence basis: source inspection | test inspection | executed probe (rare)
- Review outcome: self-reviewed | unreviewed (leave for pass 2)
- Notes: <conflicts, open questions, H-layer links>
```

IDs: prefix with your batch prefix (e.g. `AI-MOD-01`). Stable, descriptive.

## Layer H duties (all batches)

While reading the snapshot, record active design material you encounter in
scope (README claims marked experimental, docs/*.md in your packages, TODO
markers with design content): claim + anchor + the artifact's own authority
wording (normative/draft/experimental/non-goal) + observed maturity
(implemented/partial/scaffold/designed-only). Implemented experimental
behavior ALSO belongs to Layer A records with its exposure stated.

## Output contract

Write ONLY `evidence/<your-batch>.md` under this directory (path passed in your
task). Start with a 5-line batch summary (contracts measured, verdict tallies,
biggest gaps, uncertainties). End with "Coverage honesty": what you did NOT
inspect and why. Use file:line anchors everywhere. Prefer precision over
volume; never pad with restated code.
