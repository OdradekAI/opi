# Cross-Reference Matrix

Algorithm and rules for cross-referencing findings from multiple independent
audit reports.

## Severity unification

Different auditors use different severity scales. Normalize all findings to the
project's four-tier scale before cross-referencing.

| Unified tier | Codex equivalents | GLM equivalents | Opus equivalents |
|---|---|---|---|
| Blocker | P0 | Critical | Blocker |
| Major | P1 | High | Major |
| Minor | P2 | Medium | Minor |
| Info | P3 | Low | Info |

When an auditor uses a non-standard label (e.g., "Warning", "Note"), map it
based on the finding's described impact:
- Data loss, security vulnerability, crash on normal path -> Blocker
- Incorrect behavior, unhandled edge case, spec deviation -> Major
- Code quality gap, missing test, doc inconsistency -> Minor
- Improvement suggestion, style, future consideration -> Info

## Clustering algorithm

### Step 1: Extract findings

From each audit report, extract a flat list of findings. Each finding needs:

```
auditor:      <model-id>
finding_id:   <auditor's own ID, e.g. "H1", "M2", "m5">
severity:     <unified tier>
files:        <list of cited file paths>
theme:        <behavioral theme, e.g. "branch_summary provider drop">
description:  <one-sentence summary>
recommendation: <auditor's suggested fix>
```

### Step 2: Cluster by theme

Two findings belong to the same cluster when they describe the same underlying
issue. Use these signals:

1. **File-path overlap**: findings citing the same file(s) AND the same
   function/method are strong candidates.
2. **Behavioral-theme match**: findings describing the same observable behavior
   (e.g., "metadata lost on resume", "walker divergence on corrupt Leaf") even
   if they cite different lines.
3. **Recommendation overlap**: findings recommending the same fix (e.g., "unify
   the two walkers") even if their framing differs.

Do NOT cluster findings that merely touch the same file but describe unrelated
issues. The unit of clustering is the behavioral issue, not the file.

### Step 3: Assign consensus tier

| Tier | Condition | Trust weight |
|---|---|---|
| Full consensus | All auditors report the finding | 1.0 |
| Majority consensus | >50% of auditors report it | 0.8 |
| Unique finding | Single auditor only | 0.5 |

Trust weight is advisory -- it guides verification priority (Phase C) but does
not automatically determine whether a finding enters the plan.

### Step 4: Resolve severity conflicts

When auditors assign different unified severities to the same cluster:

- **Candidate severity** = highest severity assigned by any auditor.
- **Record the range** (e.g., "Major (Codex P1, GLM H2) / Minor (Opus)").
- Phase C verification may adjust the final severity based on actual code
  evidence.

## Single-report mode

When only one audit report is available:

- Skip Steps 2-4 (no clustering or consensus possible).
- Treat every finding as `trust_weight = 0.5` (unverified single-source).
- Phase C verification is especially critical: increase the verification
  depth for each finding.
- The remediation plan should note that findings are single-source and have
  not been cross-validated.

## Cross-reference matrix output

The matrix is an internal working document consumed by Phase D. Format:

```markdown
| Cluster | Theme | Auditors | Consensus | Severity | Verification |
|---------|-------|----------|-----------|----------|-------------|
| C1 | walker divergence on corrupt Leaf | Codex P2, GLM M1, Opus M1 | Full (3/3) | Major | pending |
| C2 | rootless metadata inconsistency | Codex P2, GLM M4, Opus M2 | Full (3/3) | Major | pending |
| C3 | BranchSummary provider drop | Codex P1, GLM H2 | Majority (2/3) | Major | pending |
| C4 | model picker bypasses durable write | Codex P1 | Unique (1/3) | Major | pending |
```

The `Verification` column is updated during Phase C.

## Edge cases

- **Contradictory findings**: When one auditor reports a finding and another
  explicitly refutes it (e.g., GLM's "Refuted / non-finding" section), record
  both positions. Phase C must independently verify.
- **Partially overlapping findings**: When two findings describe overlapping
  but not identical issues, create separate clusters but note the relationship.
- **Info-level findings**: Do not cluster Info findings unless they converge
  into a pattern that suggests a higher-severity systemic issue.
