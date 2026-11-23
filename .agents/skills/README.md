# Opi skills user manual

Canonical skills live in `.agents/skills/`; `.claude/skills` is an alias.
Every `opi-*` skill requires explicit invocation. Reading or maintaining a
skill does not start its workflow. Open the selected `SKILL.md` and only its
applicable references. This manual routes work; it is not a second contract.

## Choose an entry

- New feature or ordinary repair: agree the outcome and acceptance, then follow
  [task-scoped context](../../docs/agents/context.md). Small settled work proceeds
  directly; multi-session/ambiguous work gets a focused brief. No global spec,
  Phase registration or `opi-implement plan` is required.
- Need evidence: `opi-realign` for a supplied pi snapshot;
  `opi-research` for external research and current Opi comparison.
- Need documentation, evaluation, test cleanup or publication: select its named
  skill below; these are not mandatory stages of every iteration.
- Existing Phase task/audit set: read the [legacy boundary](_shared/references/legacy-phase-workflows.md)
  before selecting `opi-implement`, `opi-audit` or `opi-remediate`.
- Unsure: explicitly invoke `opi-workflow` for one recommendation.

The normal loop is: user goal → inspect/clarify → implement a small slice →
verify → synchronize affected context. Evidence and a tracker status do not
approve a product decision. Commit, publication, credentials and paid trials
retain separate authorization. Reconfirm material changes, not routine steps.

## Skill reference

| Skill | Outcome / owned artifact | Entry or stop boundary |
|---|---|---|
| `opi-workflow` | One next action | Read-only; does not invoke other skills |
| `opi-realign` | Snapshot comparison, architecture advice and acceptance in `docs/research/` | Use the supplied source identity. Adoption requires a decision. |
| `opi-research` | Primary-source research, current Opi comparison in `docs/research/` | Evidence, not implementation authority |
| `opi-implement` | Legacy Phase graph, task delivery and ledger | Explicit legacy scope; registered inputs and existing review/commit gates |
| `opi-audit` | Legacy Phase independent report group and historical appendix | Selected committed Phase contract; no production fixes |
| `opi-eval` | External evaluation evidence, reports and confirmed repairs | Live scope/budgets, maintenance and source repairs are separately authorized |
| `opi-remediate` | Legacy indexed finding plan and approved repairs | Exact audit-set and approved plan digests |
| `opi-document` | Audit/synchronize documentation | Default audits every maintained current-product README; targeted/version-bump scope is explicit |
| `opi-release` | Version, packages, public Git and crates.io release | Separate public Git and last-moment irreversible publication gates |
| `opi-slim-tests` | Verified redundant test-binary reduction | Behavioral replacement proof; leaves changes uncommitted |

## Evidence selection

`opi-realign target=<snapshot-path>` accepts a local pi directory without
Git. The supplied contents determine the target; another document's version
never overrides it. `opi-research <source-or-question>` traces primary sources
and current Opi counterparts. Read its
[transfer standard](opi-research/references/comparison-guide.md#transfer-standard)
for learn/adapt, retain, watch and reject outcomes. Keep live latest and local
snapshot identities distinct. Do not use old plans, ledgers or reports to excuse
observed gaps, and do not auto-adopt a newer upstream design.

## Evaluation and verification

Read `opi-eval/SKILL.md` for action/mode inputs and standing authorized defaults;
its [operator contract](opi-eval/references/operator.md) owns offline preparation,
execution, comparison, external evidence storage and retention. Missing baselines
do not start live runs. A confirmed repair uses the skill's
[source-repair branch](opi-eval/SKILL.md#user-confirmed-source-repairs), not an
implicit Phase-plan invocation. Evidence alone never completes an old task.

[Check selection](_shared/references/change-scope-and-check-selection.md) and
[CI/platform policy](../../docs/engineering/ci.md) select applicable checks. Reuse still-valid
proof rather than rerunning it by skill name. Development-only `opi-eval` is
not automatic product CI/release admission. Before builds or cache maintenance,
read the [Cargo cache lifecycle](_shared/references/cargo-cache-lifecycle.md).
Release and document share a bounded version-document procedure, not implicit
invocation of another user-only skill.

## Legacy Phase assurance

Only intentional legacy work loads this branch. The
[boundary](_shared/references/legacy-phase-workflows.md) precedes the individual
procedures, including source applicability and missing-input handling.

- Only `opi-implement` writes `.opi-impl-state.json`. Its
  [task completion proof](opi-implement/references/task-completion.md) and
  [ledger schema](opi-implement/references/ledger-schema.md) own checkpoints.
- Audit uses the [audit-set contract](_shared/references/audit-set-contract.md)
  for independent installation, digest binding and bounded post-install history.
  Prior conclusions never replace independent proof.
- Remediation uses the [disposition contract](_shared/references/remediation-disposition-contract.md)
  for the strict finding union, plan approval, scope and closure. Its two modes
  and exact digest gates remain intact. It does not write the ledger.
- Legacy reports, frozen snapshots and source hashes remain evidence. New task
  briefs neither duplicate their progress nor imply that those Phases passed.

## Matt integration and maintenance

For requested flows, use [tracker configuration](../../docs/agents/issue-tracker.md)
and [domain configuration](../../docs/agents/domain.md). They adapt root glossary
and `.scratch/` defaults to this project's `docs/` layout. `ask-matt` routes;
`wayfinder` resolves a large decision space; neither is a mandatory per-feature
step. Check installed skill instructions before promising compatibility. Missing
optional skills do not block clarification in the current conversation.

Keep workflow helpers/tests in the owning skill's `scripts/`, shared assurance
helpers in `_shared/scripts/`, Companion runtime in its crate, repository-wide
checks/cache/release helpers in root `scripts/`. Put each rule in one owning
contract and use conditional pointers, not copies. Preserve the user's selected
model; evaluate instruction changes through representative task outcomes, not
text length alone. [Documentation map](../../docs/README.md) owns context routing.
