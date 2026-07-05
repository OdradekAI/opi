# Realignment Report Template

Use the user's language for the final report unless they request otherwise.
Keep the chat summary high-signal; place exhaustive tables in a local report
when the audit is large.

## Executive Summary

```text
Conclusion: <low/medium/high drift>.
Core semantic drift: <level>.
Product parity gap: <level>.
Ecosystem parity gap: <level>.
Main risk: <one sentence>.
Recommended next move: <one sentence>.
```

## Required Tables

### Package / Module Mapping

| Target package/module | Current package/module | Target responsibility | Current implementation | Verdict | Adjustment |
|---|---|---|---|---|---|

### Feature / Function Mapping

| Area | Target behavior | Current behavior | Evidence | Verdict | Priority |
|---|---|---|---|---|---|

### Roadmap / Phase Alignment

| Current phase/plan | Target evidence | Alignment | Risk | Recommendation |
|---|---|---|---|---|

### Language-Native Architecture

| Architecture choice | Target shape | Current-language best practice | Current choice | Verdict |
|---|---|---|---|---|

### Adjustment Priorities

| Priority | Change | Owner/path | Why | Verification |
|---|---|---|---|---|

## Recommendation Rules

- `P0`: direction or layering risk that could make later work expensive.
- `P1`: important seam or product gap needed by near-term roadmap.
- `P2`: useful parity work after foundations are stable.
- `P3`: ecosystem breadth, polish, or optional compatibility.

## Spec-Adjustment Addendum

When the user asks to update specs or roadmap docs, include:

- exact files changed;
- boundaries added or clarified;
- non-goals added;
- acceptance criteria made more concrete;
- localized docs touched or reason not touched;
- validation commands run.
