# Controlled lifecycle differential

The TypeScript oracle imports the frozen Pi source through
`docs/iterations/opi-ai-1-5-alignment/tools/observe_upstream.mjs`. The Rust
example uses public `Models`/`ChatCall` with a controlled provider. A named
barrier prevents its terminal from racing ahead of consumer-stop observations.
It records the actual public result, source EOF and source Drop separately.

## Checks

Run on the native Windows toolchain used by this candidate:

```powershell
python -B docs/iterations/opi-ai-1-5-alignment/tools/test_acceptance_materials.py -v
node --experimental-import-meta-resolve --test docs/iterations/opi-ai-1-5-alignment/tools/test_inspect_acceptance_materials.mjs
python -B -m unittest discover -s scripts/pi-sdk-conformance/differential -p "test_*.py" -v
```

The driver test builds once through the shared Cargo-cache wrapper, then runs
the compiler-reported executable with a separate watchdog. Provider credentials
and proxy variables are not inherited by either runtime probe. Cargo builds
retain their own normal environment and shared incremental cache.

## Record a new candidate

Use new output paths; both writers reject overwriting historical evidence.
The freezer is offline and verifies the saved raw catalog against its pin.
It inventories dist/source data, resolves nested dependencies from the actual
AI importer and distinguishes internal validity from pinned identity.

```powershell
python -B docs/iterations/opi-ai-1-5-alignment/tools/freeze_acceptance_materials.py `
  --repo D:/Luiz/Odradek/opi `
  --output C:/Temp/opi-materials-next.json

python -B scripts/pi-sdk-conformance/differential/run_lifecycle_differential.py `
  --repo D:/Luiz/Odradek/opi `
  --materials C:/Temp/opi-materials-next.json `
  --output C:/Temp/opi-lifecycle-next.json
```

The runner revalidates material identities before and after replay. Every TS
case has three recorded repetitions; inconsistent observations fail rather
than being retried until green. Build identity, exact commands, stdout/stderr,
raw traces and comparisons are retained.

## Interpretation

- `FAIL` / exit 1: actual mismatch or failed probe. Known source-tail failures
  remain failures even when other material gates are blocked.
- `BLOCKED_WITH_RECORDED_OBSERVATIONS` / exit 2: observations were recorded but
  required material provenance is unresolved.
- `DIFFERENCES_REQUIRE_DECISION` / exit 0: successful evidence generation with
  observed differences, **not** strict alignment or approval of exceptions.
- Stale/changed materials stop replay or invalidate its result.

The setup-failure comparison covers only a single error outcome: Pi rejected
setup and Rust unregistered-route failure are different injection points.
The two completion scenarios retain the call for a later actual result;
separate Rust example tests cover dropping the consumer entirely. No discarded
result is fabricated. See the
[iteration report](../../../docs/iterations/opi-ai-1-5-alignment/phase5-supplemented-materials-and-gated-differential.md)
for the historical product failures and material blockers. The
[source-tail repair report](../../../docs/iterations/opi-ai-1-5-alignment/phase5-ordinary-source-tail.md)
records the corrected completion cases: current Rust-driver tests require source
EOF, while the raw probe still records the actual state and the comparator still
rejects mismatches. Material gates and unapproved differences remain open. This
is not a live provider, complete feature/platform, or full Pi alignment suite.
