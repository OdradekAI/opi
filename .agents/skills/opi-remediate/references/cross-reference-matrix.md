# Cross-Reference Matrix

Algorithm and rules for cross-referencing findings from independent audit and
runtime eval reports.

## Severity unification

Canonical four-tier definitions, foreign-label normalization, and non-standard-
label mapping live in `../../_shared/references/finding-contract.md`. Normalize
every finding to that scale before clustering. Preserve the source label and
normalization rationale; never infer a source model or silently rewrite its
severity.

## Clustering algorithm

### Step 1: Extract findings

From each selected audit or eval report, parse normalized finding blocks from
`../../_shared/references/finding-contract.md`. Preserve these source fields:

```
source_kind:  <audit | eval>
source_path:  <artifact path>
source_model: <reported identity>
finding_id:   <source-stable ID>
axis:         <normalized axis>
severity:     <source unified tier>
evidence:     <locations and observed details>
claim:        <falsifiable problem>
independence: <reported relationship>
```

### Step 2: Cluster by theme

Two findings belong to the same cluster when they describe the same underlying
issue. Use these signals:

1. **File-path overlap**: findings citing the same file(s) AND the same
   function/method are strong candidates.
2. **Behavioral-theme match**: findings describing the same observable behavior
   (e.g., "metadata lost on resume", "walker divergence on corrupt Leaf") even
   if they cite different lines.
3. **Causal overlap**: evidence points to the same violated invariant or
   production seam even when the reports propose different fixes.

Do NOT cluster findings that merely touch the same file but describe unrelated
issues. The unit of clustering is the behavioral issue, not the file.

Recommendations alone are not a clustering key. Two reviewers can recommend
the same refactor for unrelated defects.

### Step 3: Record source coverage

| Coverage | Condition |
|---|---|
| Full independent overlap | Every eligible independent source reports the behavior |
| Partial independent overlap | More than one, but not every, eligible independent source reports it |
| Single independent source | Exactly one eligible independent source reports it |
| Correlated/degraded overlap | Repeated only by same-family or unknown-independence sources |

Count independent source families, not report files. Same-family fresh contexts
remain useful evidence but do not manufacture additional independent votes.
Coverage is descriptive, not a confidence score or a decision about whether the
finding enters remediation. Severity, evidence quality, reproducibility, and
Phase C verification determine action.

### Step 4: Resolve severity conflicts

When sources assign different unified severities to the same cluster:

- **Candidate severity** = highest severity assigned by any source.
- **Record the range** with each source path/model and original label.
- Phase C verification may assign a final severity based on code/trace evidence,
  but the matrix retains every original source severity and the adjustment
  rationale.

## Single-report mode

When only one finding source is available:

- Skip Steps 2-4 (no clustering or source-coverage comparison is possible).
- Mark every finding as single-source and unverified; do not fabricate a
  numeric trust weight.
- Phase C verification is especially critical: increase the verification
  depth for each finding.
- The remediation plan should note that findings are single-source and have
  not been cross-validated.

## Cross-reference matrix output

The matrix is an internal working document consumed by Phase D. Format:

```markdown
| Cluster | Theme | Source findings | Independence | Coverage | Severity range | Verification |
|---------|-------|-----------------|--------------|----------|----------------|-------------|
| C1 | session metadata lost on resume | audit-a:A2; eval-b:E4 | independent-family | Partial independent overlap | Major / Major | pending |
| C2 | picker bypasses durable write | audit-c:S1 | unknown | Single independent source | Major | pending |
```

The `Verification` column is updated during Phase C.

## Edge cases

- **Contradictory findings**: When one source reports a finding and another
  explicitly refutes it, record
  both positions. Phase C must independently verify.
- **Partially overlapping findings**: When two findings describe overlapping
  but not identical issues, create separate clusters but note the relationship.
- **Info-level findings**: Do not cluster Info findings unless they converge
  into a pattern that suggests a higher-severity systemic issue.
- **Cross-kind overlap**: An audit and eval finding may cluster only when they
  describe the same behavior. Runtime evidence strengthens verification but
  does not automatically validate the static audit's causal claim.
