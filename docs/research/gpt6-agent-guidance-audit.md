# GPT-6 agent guidance audit

Date: 2026-09-27. Baseline: `5803c8758006593efd4efeba214ae8b73c903580`.
[中文](gpt6-agent-guidance-audit.zh.md)

This is non-normative instruction-maintenance evidence, not a Phase audit,
implementation ledger or measured model-performance report.

## Scope and sources

Reviewed root guidance, all ten project `SKILL.md` entrypoints, their Codex
sidecars, active workflow references and the bilingual skill manual. Inspected
CI/documentation contracts to preserve their integration; no workflow script,
Rust/Cargo, CI YAML, normative specification, runtime model configuration or
canonical ledger was changed. `CLAUDE.md` and `.claude/skills` remain symlinks.
There is no Chinese AGENTS/individual-skill counterpart; the paired manual and
this report were synchronized.

Official guidance retrieved on the audit date:

- [Rethinking skills and prompts for GPT-6 Astra](https://developers.openai.com/blog/rethinking-skills-and-prompts-for-gpt-6-astra)
  motivates precise discovery, conditional context and removal of unnecessary
  procedural scaffolding.
- [GPT-6 model guidance](https://developers.openai.com/api/docs/guides/latest-model)
  addresses follow-through, boundaries, delegation and proportional verification.
- [GPT-6 Sol](https://developers.openai.com/api/docs/models/gpt-6-sol) confirms its
  coding/agentic role; task routing recommendations in the manual are local
  heuristics, not an official mandatory model split.

## Findings and disposition

| Surface | Finding | Applied change / retained constraint |
|---|---|---|
| AGENTS.md | Repeated workflow inventories and generic process rules increased always-loaded context | Context by decision; scoped autonomous completion; compact safety and verification rules |
| Workflow/manual | Router said eval could not fix, contradicting the confirmed-repair branch | Link to the owning repair contract; keep trial and source-repair authorization separate |
| Shaping routes | Missing optional Matt/tracker tooling could block product deliberation | Check actual availability; permit human shaping in the current conversation; retain source registration |
| Implement | A/B/F “always execute” conflated status/plan/docs with runtime execution | Active-mode routing; no Cargo bootstrap for docs/status/plan; A–F in a linked execution reference |
| Implement parallel work | Full verification after each sub-unit repeated assembled checks | Integrate in ledger order, verify assembled scope, rerun invalidated evidence |
| Audit | Reader/install details duplicated their shared contracts | Load finding/rendering rules at use; retain fresh context, complete committed scope, hashes and independent installation |
| Eval | Branch-independent reading and duplicate fidelity scoring | Load selected mode; evaluator prompt owns scoring; retain preserved evidence, budgets and privacy stops |
| Remediate | Apply text obscured continuation within an already approved batch scope | Continue approved batches; changed plan/index binding still blocks before edits |
| Document | Unrelated release history was mandatory for every targeted edit | Read version/release sources when claims require them; default full README audit remains exhaustive |
| Realign | Root duplicated layer/evidence and worker rules from references | Route to the owning references; retain pinned identity, Layer A/H coverage and independent verification |
| Research | Placement candidates omitted Independent Companion | Align with spec Chapters 4/8; reuse resolved scope; evidence remains non-normative |
| Release | Cached crate count/order and skip-cross completion assumptions could drift | Derive publishable graph; verify artifacts according to chosen strategy; preserve public and irreversible gates |
| Slim-tests | Repeated shared test-stewardship rules | Keep shared proof owner; retain behavior/platform coverage and no automatic commit |

Existing shared assurance/check-selection contracts, validators and output
schemas remain authoritative. Detailed gates were retained where they protect
artifact integrity, authority or reproducibility. No product changelog entry
was needed: this change concerns contributor instructions, not shipped runtime
behavior.

## Size evidence

Counts use UTF-8 bytes with CRLF normalized to LF, comparing Git baseline with
the working files. They are not tokenizer counts or latency measurements.

| Entry | Before | After | Reduction |
|---|---:|---:|---:|
| AGENTS.md | 13,193 | 8,643 | 34.5% |
| Implement entrypoint | 29,290 | 13,785 | 52.9% |
| English manual | 13,785 | 9,911 | 28.1% |
| Chinese manual | 12,603 | 9,006 | 28.5% |

Across AGENTS.md, all ten skill entrypoints and both manuals, the aggregate
decreased from 133,331 to 98,010 bytes (26.5%). This is a file inventory,
not an assertion that all thirteen files load on each request.

Implementation detail moved to `opi-implement/references/execution.md` remains
available for execution; moving it reduces unrelated mode loading rather than
deleting its obligations. Research gained a small amount of text to repair
placement semantics. A smaller file was not the acceptance criterion.

## Verification and limits

- `python scripts/opi-doc-check.py`: PASS after integration.
- `git diff --check`: PASS.
- `python .agents/skills/opi-implement/scripts/skill-contract.tests.py`:
  3 tests passed.
- Independent read-only tabletop validation covered six requests: a root typo,
  implementation status, shaping without optional tools, confirmed eval repair,
  default full README audit, and remediation after index drift. It found a
  stale implementation-to-Matt return path, which was synchronized with the
  router. This validates instruction interpretation, not execution success.
- The generic installed skill-creator validator is incompatible with existing
  project frontmatter (`disable-model-invocation`, release `arguments`);
  one native-Windows invocation also hit default text-encoding failure.
  Repository validators passed; those external failures were not reported as
  successes or “fixed” by weakening project invocation policy.

Test impact: `none`; existing checks were retained and executed. No paid
provider trial, live release, formal Phase audit or Astra/Sol A/B performance
run was performed. No Rust compilation was needed for these Markdown/YAML
instruction changes.

Existing release limitation: `.github/workflows/release.yml` builds/uploads
binaries for every `v*` tag, so a source-only `--skip-cross` release still needs
a separate CI/strategy decision. The skill now detects that mismatch before
the public push gate; this audit does not claim to implement that release mode.

Worktree-only scope: `committed=none-for-this-task`, `staged=none`; modified
tracked instructions are unstaged, the execution reference and bilingual audit
record are untracked. The initial worktree was clean and HEAD was unchanged.
No commit or publication was created.

## Performance follow-up

Use the same representative tasks, source state, host/tools, model and effective
reasoning setting for baseline/candidate comparisons. Check task success and
authority preservation first, then unnecessary approval pauses, tool calls,
wall time, token/cost coverage and variability. Compare Astra and Sol separately;
report unknown telemetry as unknown. Until such paired runs exist, the supported
claim is reduced instruction redundancy and clearer routing, not a speed,
quality or cost gain.
