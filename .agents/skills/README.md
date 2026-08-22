# Opi skills user manual

Opi has ten project skills. Their tracked source lives in `.agents/skills/`;
`.claude/skills` is a compatibility symlink to the same directory. Edit only
the canonical `.agents/skills/` files. Every skill requires explicit user
invocation: for example `$opi-workflow` in Codex or `/opi-workflow` in Claude
Code.

Use `opi-workflow` when the correct entry is unclear. It recommends one next
command and stops; it does not create another workflow ledger.

## Choose an entry in 30 seconds

1. Are you trying to decide **what should be built**?
   - Compare opi with the pinned pi revision: `opi-realign`.
   - Investigate an external capability or ecosystem option: `opi-research`.
   - The evidence exists but the product decision is unsettled: use the
     recommended human-led shaping command; do not start implementation.
2. Is there a reviewed and registered Phase delivery source?
   - No: return to evidence or shaping.
   - Yes: run `opi-implement plan`; run `opi-implement` only after admission.
3. Are you checking or correcting shipped behavior?
   - Static requirement conformance: `opi-audit`.
   - Credentialed real-provider behavior: `opi-eval`.
   - Verify and optionally fix normalized findings: `opi-remediate`.
   - Documentation, release, or test-link cleanup: use the named skill.

## Lifecycle and return loops

Solid arrows are workflow progression. Dashed arrows cross a human decision or
materialization boundary. A `READY` verdict is not implementation or commit
authorization.

```mermaid
flowchart TD
    RI[opi-realign<br/>inward evidence]
    RO[opi-research<br/>outward evidence]
    SH[Human-led shaping<br/>review and materialize decisions]
    SRC[Registered Phase<br/>delivery source]
    PLAN[opi-implement plan<br/>admission and graph review]
    EXEC[opi-implement<br/>delivery and Phase exit]
    ASSURE[opi-audit / opi-eval<br/>independent assurance]
    FIX[opi-remediate<br/>verify and optionally fix]
    DOC[opi-document]
    REL[opi-release]
    SLIM[opi-slim-tests<br/>independent test-link optimization]

    RI --> SH
    RO --> SH
    SH -. human approval and registration .-> SRC
    SRC --> PLAN
    PLAN -->|READY + graph gate| EXEC
    PLAN -. RESEARCH_REQUIRED .-> RI
    PLAN -. DESIGN_DECISION_REQUIRED .-> SH
    PLAN -->|GRAPH_REVISION_REQUIRED| PLAN
    EXEC --> ASSURE
    ASSURE -->|confirmed findings| FIX
    FIX --> ASSURE
    ASSURE -->|requirements satisfied| DOC
    DOC -. public and irreversible gates .-> REL
    EXEC -. current test graph .-> SLIM
```

## Boundary rules

- Evidence is not a requirement. `opi-realign` and `opi-research` cannot
  authorize product work.
- Shaping is human-led. Tracker maps and candidate specs are non-normative
  until reviewed and materialized into `docs/opi-spec.md` or a registered Phase
  delivery source.
- `opi-implement plan` tests readiness; it does not repair missing product
  meaning. Graph confirmation and Git commit authorization are separate gates.
- `opi-audit` and `opi-eval` diagnose; they do not edit production code.
  Audit runs seal committed evidence before comparing history.
- `opi-remediate mode=plan` verifies and plans; `mode=apply` requires explicit
  approval of the exact immutable plan. Neither mode writes the live
  `.opi-impl-state.json`.
- `opi-document` proves documentation truth; it does not authorize release.
- `opi-release` is the only public publication workflow. Crates.io publication
  has a separate last-moment irreversible gate.

## Skill reference

Effect labels: `RO` reads only, `W` writes repository files, `C` may create a
local commit after an explicit gate, `$` may use credentials or paid providers,
`P` changes public state, and `I` contains an irreversible step.

| Skill | Role and required input | Owned output | Stop/gate and usual next step | Effect |
|---|---|---|---|---|
| `opi-workflow` | Route an uncertain request | One recommended invocation | Stops at every explicit-skill boundary | RO |
| `opi-realign` | Compare an exact pinned pi revision with opi | `docs/realign/*.md` | Evidence only; next is shaping or `opi-implement plan` after registration | W |
| `opi-research` | Investigate outward capabilities from primary sources | `docs/research/*.md` | Evidence only; next is shaping | W |
| `opi-implement` | Admit a registered Phase source, execute its graph, and archive Phase evidence | `.opi-impl-state.json`, Phase snapshots, task changes | Graph, task-commit, ledger-commit, and failure gates are distinct | W, C |
| `opi-audit` | Seal and verify one Phase against the complete relevant implementation at committed HEAD | Immutable `audit.<model>.<head7>.<run-id>.md` and `.findings.jsonl` siblings | No fixes; confirmed findings go to `opi-remediate` | W |
| `opi-eval` | Run explicit isolated real-provider fidelity cases | `docs/eval/` reports and history | Requires credentials and mutating-tool opt-in where applicable | W, $ |
| `opi-remediate` | With `mode=plan`, verify immutable findings and derive closure batches; with `mode=apply`, execute one approved plan | Immutable plan, dispositions, result, and user-approved fixes | `READY-FOR-APPLY` plus exact-plan approval gates execution; intent changes return to shaping | W |
| `opi-document` | Synchronize truthful English/Chinese docs and source-derived checks | Documentation and doc-check changes | Does not publish | W |
| `opi-release` | Run seven gated release phases for six crates and GitHub assets | Git tag/release and crates.io versions | Public Git gate, then separate irreversible crates gate | W, C, P, I |
| `opi-slim-tests` | Remove duplicate or superseded Rust test binaries without losing behavior | Verified uncommitted test-graph reduction | Never commits automatically | W |

## Assurance model

`opi-audit` and `opi-eval` emit normalized findings using
`_shared/references/finding-contract.md`. `opi-remediate` preserves the original
source, severity, independence, and evidence while recording verification,
lineage, decisions, and closure proofs separately in an immutable disposition
artifact. A remediation result can close a batch; only a fresh audit can prove
Phase conformance. Generic provider-fidelity canaries are runtime signals; only
a registered runtime-fidelity case can close a product criterion.

Use independent models or reviewers when practical and disclose degraded
independence. No preferred provider or model is part of the project contract.

## Durable artifact ownership

| Artifact | Owner and lifetime |
|---|---|
| `docs/realign/*.md` | `opi-realign`; non-normative inward evidence |
| `docs/research/*.md` | `opi-research`; non-normative outward evidence |
| Tracker maps/tickets and candidate specs | Human-led shaping; non-normative until materialized and registered |
| `docs/opi-spec.md` and registered Phase delivery sources | Human-led shaping; normative sources |
| `.opi-impl-state.json` | `opi-implement`; canonical tracked implementation ledger |
| `.opi-impl-state.draft.json` | `opi-implement plan`; ignored scratch retained only while review/resume needs it |
| `docs/snapshots/phase<N>/` | Frozen implementation, audit, and remediation evidence |
| `docs/eval/` | `opi-eval` reports and history |
| `.opi-release-state.json` | `opi-release`; ignored resume state retained only during an incomplete release |
| `_shared/references/finding-contract.md` | Shared finding schema |
| `_shared/references/remediation-disposition-contract.md` | Shared verification, lineage, decision, and closure schema |

Only `opi-implement` writes the canonical implementation ledger. Do not create
a second task ledger or record implementation progress in `docs/opi-spec.md`.

## Maintainer composition note

Project-local skills own Opi artifacts and lifecycle boundaries. Matt skills
may supply evidence, domain modeling, design challenge, test-seam design, TDD,
or review lenses when their invocation policy permits it. Superpowers may
supply narrow operational primitives such as evidence-before-completion.
Neither family may replace `.opi-impl-state.json`, silently invoke a
user-only shaping command, or introduce a second plan/execution state machine
inside `opi-implement`.

After selecting a skill, read its `SKILL.md` and only the references it routes
to. Destructive, costly, credentialed, commit-producing, or publication actions
always remain explicit user gates.
