---
name: opi-realign
description: Compare a current implementation with a target/reference project and produce an architecture, feature, design-philosophy, package-boundary, and roadmap realignment review. Use when the user asks to realign, audit drift, compare a port/reimplementation, check whether planned phases match an upstream project, or evaluate cross-language architecture against a target project path.
---

# Opi Realign

## Inputs

Require a target/reference project path. If the user omits it and no obvious
path is present, ask for it before auditing.

Supported input shape:

```text
current=<path>        # optional; defaults to cwd
target=<path>         # required; local reference/upstream/project path
current_label=<name>  # optional display label
target_label=<name>   # optional display label/version
scope=<text>          # optional packages, phases, docs, or roadmap slice
```

Treat `@path`, quoted paths, Windows paths, and POSIX paths as valid path
arguments. Do not hard-code `pi`; the target project can change.

## Workflow

1. **Establish scope**
   - Identify current path, target path, labels, and requested scope.
   - State assumptions that affect the audit.
   - If the user asks a question, answer it before editing or running a long audit.

2. **Build evidence inventories**
   - Read repository guidance files first: `AGENTS.md`, `CLAUDE.md`, README,
     changelogs, specs, architecture docs, package manifests, and phase plans.
   - Inventory packages/crates/modules, public binaries, extension points,
     config/session formats, tests, and roadmap docs in both projects.
   - Prefer `rg`/`rg --files`; read full relevant docs before broad claims.

3. **Compare semantics, not file shapes**
   - Separate core semantic alignment, product parity, and ecosystem parity.
   - Compare architecture boundaries, runtime flow, data formats, provider or
     integration surfaces, UI surfaces, extension/plugin models, tests, docs,
     operational assumptions, and explicit non-goals.
   - When source and target languages differ, use the current implementation
     language's best practices. Do not require one-to-one package/file mapping.

4. **Classify drift**
   - Use levels: `Aligned`, `Intentional divergence`, `Partial`, `Missing`,
     `Overreach`, `Risk`.
   - Distinguish "not implemented yet" from "implemented in the wrong layer".
   - Cite local files/lines for important claims.

5. **Recommend adjustment**
   - Prioritize changes as `P0` through `P3`.
   - Prefer strengthening existing seams before adding breadth.
   - Identify work that should remain future ecosystem scope.
   - If asked to update specs, edit the registered/current spec files only and
     keep localized counterparts in sync when safely editable.

6. **Report**
   - Lead with the conclusion: degree of drift and what to change next.
   - Include tables for package mapping, feature/function mapping, roadmap or
     phase alignment, language-native architecture judgment, and priorities.
   - For large audits, create a local HTML or markdown report in a generated
     report directory and summarize the highest-signal findings in chat.

## References

- Read `references/audit-framework.md` for the full comparison dimensions and
  drift taxonomy.
- Read `references/language-porting.md` when the current and target projects
  use different languages, runtimes, or packaging models.
- Read `references/report-template.md` when producing a detailed written
  report, tables, or spec-adjustment recommendations.

## Guardrails

- Do not claim API, config, package, or file-format compatibility unless the
  evidence proves it.
- Do not treat target-project breadth as automatically desirable in the current
  project.
- Do not recommend copying target-language architecture when it conflicts with
  current-language ownership, dependency, concurrency, packaging, or testing
  norms.
- Do not modify source code, specs, or roadmaps unless the user explicitly asks
  for edits.
- Do not commit unless the user explicitly asks.
