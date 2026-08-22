# Finding lineage and closure matrix

Use the normalized finding contract for ingestion and the remediation
disposition contract for output. Preserve every source record unchanged.

## Lineage classification

Compare immutable `(source_path, id)`, `closure_key`, `family_key`, and prior
disposition evidence:

| Kind | Rule |
|---|---|
| `new` | No prior occurrence has the same closure or family identity. |
| `recurrent-same-defect` | The same closure identity was previously observed and was not explicitly deferred; prior disposition shows whether a closure was attempted. |
| `recurrent-adjacent-path` | The family is known but the current closure predicate identifies a different behavioral path. |
| `regression` | Evidence ties the reappearance to a known previously passing endpoint or change. |
| `carried-forward-deferred` | The same closure identity was explicitly deferred and remains unresolved. |

Text similarity is only a search hint. Do not classify recurrence from titles,
shared files, or recommendations alone. When history is incomplete, prefer
`new` and record the evidence limitation rather than invent lineage.

When exact-closure history contains conflicting dispositions, preserve all of
them. `carried-forward-deferred` applies only when every exact prior disposition
is an explicit deferral; any non-deferred exact occurrence yields
`recurrent-same-defect`. Use `regression` only when `regression_of` evidence ties
the current failure to a known passing endpoint/change.

## Closure clustering

Two findings may enter the same closure batch only if all are true:

1. they have one closure predicate;
2. one root change is expected to satisfy that predicate for every member;
3. one red-before/green-after evidence pair can discriminate closure;
4. they can be reviewed and reverted as one bounded change.

Shared files, family keys, severity, or suggested refactors do not satisfy the
one closure predicate rule. Keep adjacent paths separate and link them through
`family_key`.

## Coverage and severity

Record independent source-family coverage as full overlap, partial overlap,
single source, or correlated/degraded overlap. Coverage is descriptive, not a
vote. Candidate severity is the highest normalized source tier; final severity
comes from current verification and retains every source tier plus rationale.

Contradictory sources remain separate evidence until verification resolves the
claim. `Cannot confirm` is not `Refuted`.

## Matrix

| Batch | Closure key/predicate | Family | Source findings | Lineage | Coverage | Source severity | Verification | Decision |
|---|---|---|---|---|---|---|---|---|
| B1 | ... | ... | ... | ... | ... | ... | ... | ... |
