# Eval Storage Lifecycle Implementation Plan

> Superseded for storage/query and retention policy by
> [file records and latest-three retention](2026-10-06-opi-eval-file-retention.md).
> The user explicitly rejected SQLite and selected latest-three history;
> database and age/quota descriptions below record the previous implementation.

> **For agentic workers:** Use superpowers:subagent-driven-development for bounded implementation and independent review. The user approved the external per-evaluation storage, query index, freshness, archive and cleanup design in this conversation. No paid runs, existing-evidence cleanup, commits or ledger writes are included.

**Goal:** Keep every evaluation and its reports outside the project, query retained evidence without scanning an ever-growing registry, and manage freshness and retention without changing historical outcomes.

**Architecture:** The existing Python operator moves into the unpublished Companion's operator source tree. An operator-owned SQLite catalogue is rebuildable from per-evaluation records and lifecycle receipts; original native evidence remains authoritative. Family operations share one evaluation directory and one live deadline; maintenance consumes exact reviewed plans and preserves evidence dependencies.

**Tech Stack:** Python standard library, SQLite, existing native Companion CLI and offline subprocess fixtures; no Rust dependency or runtime changes.

## Global constraints

- Follow PRIN-003 and CTRL-004 through CTRL-009: no Agent Core dependency, native grading authority, immutable originals, separate execution/change authorities; catalogue writes belong only to the evaluation operator.
- Preserve carried-in research migration, `.gitignore` changes and deleted `docs/eval/operator-decoupling.*` files. Do not recreate those deleted plans.
- Keep original `/1` profiles/snapshots readable. New retention configuration is versioned; no silent private-profile rewrite or required diagnostic expansion.
- No provider/network/Docker invocation in tests; no archive/delete operation against existing user evidence. Synthetic disposable fixtures are isolated.
- Shared Cargo caches and evidence/material dependencies are protected. No global pruning, cache eviction or path rewriting inside seals.
- Full independent runtime distribution remains provisional; this change moves source ownership, not public packaging or pi pin admission.

## Task 1: External records and catalogue

**Files:** Create `crates/opi-eval/operator/operator_catalog.py`, `scripts/test_opi_eval_catalog.py`; move existing five operator modules and sole template to `crates/opi-eval/operator/` under Task 3.

**Interface:** `Store(workspace)` owns `catalog.sqlite`, `evaluations/YYYY/MM/<id>/record.json`, `imports/` and lifecycle receipts. Methods: `create(action, benches, profile)` returns the evaluation root; `refresh(root)` returns a derived catalogue row; `reindex()` rebuilds from records/receipts; `list(**filters)` and `show(id)` expose metadata; `snapshots(bench=None, pi_version=None, max_age_days=0)` returns original pinned eligible snapshot dictionaries; `import_index(path)` imports legacy references read-only; `event(id, action, **fields)` preserves lifecycle decisions; `lease(id)` excludes concurrent execution/maintenance. Active, archived, retired, missing and unverifiable states remain distinct.

- [x] Add failing offline cases for independent dated roots, SQLite rebuild/corruption recovery, filters, old registry import, missing/drifted snapshot, age/version selection, retirement and active leases -> verify: `python scripts/test_opi_eval_catalog.py` observes missing behavior.
- [x] Implement bounded records, parameterized SQL, root/path validation, durable receipts, recovery and immutable import references -> verify: the full catalogue binary passes, including negative inputs and concurrent lease refusal.

## Task 2: Archive, restore and retention plans

**Files:** Create `crates/opi-eval/operator/operator_lifecycle.py`, `scripts/test_opi_eval_lifecycle.py`.

**Interface:** `maintenance_plan(store, policy, action, ids=None)` writes no evidence and returns a versioned inventory/digest-bound plan with protected reasons and measured sizes. `apply_plan(store, plan_path)` rechecks bindings, ownership, leases, current references and policy before acting. `restore(store, id)` restores archived bytes to their original canonical path without modifying seals. Storage events remain outside native sealed trees.

- [x] Add failing cases for protected pi baselines/pins/references/active operations, retention by age/count, deterministic preview, changed inventory refusal, archive byte identity/restore, symlink/archive path attacks and tombstones -> verify: `python scripts/test_opi_eval_lifecycle.py` observes missing behavior.
- [x] Implement isolated archive/restore and explicit plan application, never automatic deletion or cache cleanup -> verify: full test binary and a synthetic CLI preview/apply refusal. Missing/unportable dependencies produce typed gaps rather than altered evidence.

## Task 3: Operator/config integration

**Files:** Move `evaluate.py`, `operator_config.py`, `operator_process.py`, `operator_evidence.py`, `operator_flow.py`, `config.example.toml` from `.agents/skills/opi-eval/` to `crates/opi-eval/operator/`; update `scripts/test_opi_eval_operator.py` and fixture; create `scripts/test_opi_eval_storage_flow.py`.

**Interface:** Retain existing prepare/execute/compare controls; add explicit catalogue queries/import/rebuild, lifecycle preview/apply and restore. `/2` config retains model/thinking/credential/retry controls and adds baseline age and retention (candidate/failed days, newest count, quota signal, deletion disabled). `/1` maps to conservative defaults without mutation. New operations are under dated evaluation/family roots; reports and all index writes stay external. `--bench all` groups selected families and uses one first-dispatch deadline.

- [x] Run unchanged operator tests; write failing storage-flow cases for external registration/reporting, read-only repository, grouped scope/deadline, stale/missing baseline and config validation -> verify: `python scripts/test_opi_eval_operator.py` baseline and targeted new failures.
- [x] Relocate existing runtime modules/template, wire catalogue/lifecycle commands and profile controls, preserve original retry/verification/capture semantics -> verify: all four existing/new operator fixture binaries, real CLI negative cases and Windows/WSL platform checks.

## Task 4: Documentation, review and handoff

**Files:** Update skill/manual counterparts, operator/native/baseline/report/evidence/fidelity/cleanup references, `docs/eval/README.md`, crate README counterparts, complete Unreleased section, doc checker contracts/tests where current expectations change.

- [x] Make external reports/index/history the default; `docs/eval` becomes an optional explicit conclusion export. Document actual commands, retention protections, historical validity versus freshness and offline restore limitations.
- [x] Review independent behavior against approved design and owning contracts; fix consequential findings -> verify: affected script binaries/negative subprocess cases, `python scripts/opi-doc-check.py`, `python scripts/test_opi_doc_check.py`, `git diff --check` and outgoing worktree inventory.
- [x] Record add/update/retain/delete test impact, exact outcomes and remaining capability limits. No live benchmark score is inferred from fixture success.

## Completion evidence

The implementation is complete for the approved storage scope. Tests use disposable
offline process fixtures; no paid provider, Docker benchmark, existing-user archive,
cleanup, commit or ledger write was performed.

| Command | Observed outcome |
|---|---|
| `python scripts/test_opi_eval_operator.py` | 44 pass on Windows and WSL; original retry, native validation and comparison behavior retained |
| `python scripts/test_opi_eval_storage_flow.py` | 14 pass on Windows and WSL; external records/reports, grouped budget/refusal, comparison references/leases and sanitized CLI refusal |
| `python scripts/test_opi_eval_catalog.py` | 17 pass on Windows and WSL; rebuild/corruption recovery, pagination/filtering, freshness, missing roots, immutable identities/imports and references |
| `python scripts/test_opi_eval_lifecycle.py` | Windows: 31 run, 30 pass and one POSIX permission skip; WSL: all 31 pass |
| `python scripts/opi-doc-check.py` | PASS |
| `python scripts/test_opi_doc_check.py` | 26 pass |
| `git diff --check` | Exit 0 |

Windows commands use the workspace Python; WSL commands use `wsl.exe python3`
with `/mnt/d/Luiz/Odradek/opi/scripts/<test>.py`. Rust runtime code, manifests,
dependencies and native protocols are unchanged, so Cargo gates are outside this
script/documentation verification union. Exact script binaries and invalid-input
subprocess cases ran on both supported local surfaces.

Fresh independent review found and confirmed fixes for standalone comparison
reference loss, truncated/untrusted lease ownership discovery, and comparison
outputs mutating unrelated retained trees. Follow-up reproductions verified the
input references survive rebuild, forged discovery metadata cannot bypass a
canonical lease, and unsafe output is refused before mutation. Parent integration
also preserves finished siblings and a gap report after a family refusal. Real
operator-produced inert evidence passed archive/restore byte identity; unreadable
subtrees and unrecognized material dependencies refuse maintenance.

Test impact: **add** three storage/catalogue/lifecycle binaries (14/17/31 cases);
**update** existing operator import paths, dated-root assertions and multi-family
fixture; **retain** 44 operator and 26 doc-check cases plus existing native/Rust
checks; **delete** none. The old runtime paths/template are relocated rather than
maintained as duplicate compatibility entrypoints.

Capability limits: independent runtime distribution/public packaging remain
provisional; native preparation/run still require Linux x86_64, Docker and the
admitted pi pin. No new pi-version admission or richer diagnostic capture was
added. Links, unknown portability/cleanup, and unknown legacy dependency contracts
remain protective gaps; restoration retains original paths and cannot promise
offline regrading. Independent comparison output remains caller-owned and is
referenced from its dated record rather than adopted for destructive maintenance.
