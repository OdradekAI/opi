# Sealed pi baseline reuse

Read when an Opi-only benchmark run compares against a previously sealed pi
result instead of rerunning pi. This is a benchmark-branch operator policy,
not a Companion feature: the experiment declares a single `opi` subject with
no artificial edge, and the cross-run pairing below is disclosed in the human
conclusion report. `acceptance` keeps its same-run six-trial pairing; `verify`
and `fidelity` are unaffected. Nothing here reruns, edits or regrades sealed
baseline evidence.

## Baseline eligibility

The baseline is a root registered in `docs/eval/evidence-roots.jsonl` whose
sealed bundles still pass Companion `verify` (exit 0) at reuse time. Record
the root path, its manifest digest where applicable, the owning baseline
report, and the exact pi trial bundle digests used for pairing.
Configured candidate operations discover the registered snapshot and freeze
its path/digest before live dispatch. A complete version baseline accounts for
every expected task/repetition with a valid native settlement, including zero.
Retain incomplete snapshots with coverage gaps; partial matches may support
labelled per-task comparisons but never a full-corpus claim. Select the newest
eligible matching snapshot deterministically, never the highest-scoring one.

## Identity tuple

Reuse is comparable only when every dimension below matches the new run's
frozen records; any mismatch or unknown marks the affected pair
`incomparable` and blocks reuse of that pair:

- pi subject identity: static-lock pin (commit and blobs), resolved package
  version, staged executable digest;
- benchmark family with corpus revision, per-task package digests, image
  digests, native grader/runner identity and task lifecycle/resource limits;
- connection (provider/model/api/base_url) and all five effective
  `model_controls` values;
- task selection: pairing compares the same task ID, not merely the same
  family, with matched repetition/seed and effective stage budgets;
- environment identity as recorded in both runs (Linux arch, engine
  observations);
- Companion build identity: reuse requires the same verified Companion
  executable digest as the baseline run. A changed Companion defaults to
  `incomparable`; only a fresh paired run or an explicit user-accepted
  grading-path equivalence statement, recorded with both digests and the
  diff scope, permits continued reuse.

Other Opi workspace crates changing between the baseline and the new run do
not affect the pi baseline; they change only the Opi candidate identity,
which is recorded as usual under native-run provenance.

Preserve each run's complete material digest for its own verification. It is
not a cross-run comparison key: material includes different selected Agent
packages, absolute paths and request-log locations. Compare the semantic
benchmark/control identities above, retaining both original values and digests.
Likewise, free disk samples and observation timestamps are diagnostics, not
task execution limits; unavailable required environment identity remains a gap.

## Workflow

1. Step 1: collect `baseline_root` or discover a matching registered snapshot;
   verify the baseline seals offline before
   requesting any live scope.
2. Step 2: check every identity dimension against the new experiment's frozen
   material and plan; disclose the reuse and any `incomparable` dimension in
   the approval preview before live authorization.
3. Step 3: execute only the Opi trials.
4. Step 5: the conclusion report pairs each Opi trial with the sealed pi
   trial of the same task, labelled cross-run with the baseline root and
   bundle digests. Repetition control pairs stay same-run only. Scores stay
   benchmark-native per run; a six-trial cross-run pairing is not a
   statistical superiority claim.

Freeze one comparison manifest binding the selected baseline, the candidate
plan and the per-task identity checks. Finalize its outcome as a separate
derivative referencing both immutable run bundles. Missing baseline results
stay `missing`, mismatches/unknown required dimensions stay `incomparable`,
and partial coverage is disclosed beside any paired metric. Baseline selection
does not authorize another pi run or verifier regrade.

**Completion criterion**: the report names the baseline root, its registry
record, every checked identity dimension with both runs' values, and the
exact paired bundle digests. A failed check yields `incomparable`, never a
substituted fresh pi number or an edited baseline bundle.
