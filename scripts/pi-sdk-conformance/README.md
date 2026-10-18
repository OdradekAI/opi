# pi-sdk-conformance — packaged public SDK smoke proof

This offline harness checks the public `opi-ai` Schema seam from an actual
`cargo package` artifact in a workspace-external ordinary Cargo consumer. It
uses crates.io `regress 0.12.0` and `url 2.5.8`, with the supported upstream data
profile recorded by each run. It does not pin a Unicode release. The selected
contract and accepted data differences are recorded in the
[Schema readiness source](../../docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-readiness-schema.md)
and [Rust difference register](../../docs/research/pi-ai-opi-ai-alignment/pi-1.3.0-rust-differences.md).

The harness retains bounded dependency, conformance and package evidence for
the current tasks. Full strict/grammar projection, conformance and final SDK
distribution acceptance remain with tasks 20.11, 20.34 and 20.57 respectively.
There is no registry publication or live model call.

## Layout

- `run_early_package_proof.py` packages `opi-ai` with Cargo, safely extracts the
  artifact into a new external scratch, resolves an ordinary consumer offline,
  validates its dependency identities/features and runs both feature profiles.
- `consumer/` is the consumer template. Only its SDK path is substituted, with
  the extracted package directory. Backend dependencies use crates.io. There
  is no patch, registry server, source replacement or build-time generator.
- `current-profile.json` explicitly overrides two frozen URL results for the
  accepted raw/percent-encoded Beria Erfe domain difference. It does not change
  the historical Node observations or treat other differences as accepted.
- `frozen/` retains the original 43 leaf cases, historical Node/Unicode16
  expectations and seven script probes. The current consumer selects the
  recorded stock-regress outcomes for script probes.
- `evidence/upstream-package-proof.json` records the actual executed results of
  the current upstream proof: both feature profiles, the resolved consumer
  graph, package and lockfile hashes, per-step exits and the full 73-check
  reports. The remaining files retain historical vendor preparation and
  local-registry results. Those describe the retired implementation, are not
  current passing proof and cannot be replayed with this harness alone: the
  vendor input archive, generation and local-registry tools have been retired.
- `test_package_proof.py` covers scratch/cache safety, package extraction,
  metadata/feature validation, failure evidence, lease release and the CLI
  negative path.

The former vendor source-integrity, table regeneration, namespace packaging,
dual-backend/dual-ICU coexistence and sparse-registry gates are retired. Ordinary
consumer and public Schema validation remain active.

## Run

Resolve or inspect the shared workspace target with the existing
[Cargo cache workflow](../opi-cargo-cache.py). Supply its existing marked path;
the harness refuses an unmarked cache or one belonging to another workspace.
Use a **new** scratch directory outside both the repository and target cache:

`ICU4X_DATA_DIR` must be absent from the environment. Any set value, including
the empty string, is rejected with exit 2 before creating scratch data or
starting a subprocess, so the proof verifies upstream bundled ICU data.

```powershell
python -B scripts/pi-sdk-conformance/run_early_package_proof.py `
  --scratch C:/Temp/opi-sdk-proof-new `
  --target-dir D:/Luiz/cargo-targets/opi-72fa37a0fa044958
python -B -m unittest discover -s scripts/pi-sdk-conformance -p test_package_proof.py -v
```

The target path shown is an example; use the current workspace cache path.
`CARGO_TARGET_DIR` may supply it instead. Cargo uses the existing `CARGO_HOME`
and incremental compilation, with a lease released on command failure as well
as success. It never deletes scratch data, targets or Cargo caches. Missing
offline packages fail with evidence; no network fallback occurs.

The package step uses `--allow-dirty --no-verify --locked --offline`; the actual
extracted package is then compiled and exercised by the consumer, without
relying on the package's own lockfile to constrain downstream resolution.
The consumer creates its own lockfile offline, then builds with `--locked`.
Both caller profiles share the same target and lockfile: defaults disabled,
and caller-enabled `regress/default` feature union. Opi's required `std`,
`utf16` and `prohibit-unsafe` features remain present in both. The latter graph
contains the optional `backend-pikevm` feature; the ordinary `Regex` API still
uses upstream 0.12.0's default executor.

Exit 0 and `"verdict": "PASS"` require both profiles to pass all 73 checks.
The scratch retains the package archive/checksum, consumer sources/lockfile,
metadata, stdout/stderr per command and `evidence.json`. Evidence records input
hashes and the actual URL/regex/IDNA/ICU/Unicode dependency sources and features.
The consumer lockfile records resolved registry checksums; a repository lockfile
does not constrain all future library consumers.

## Battery

All 43 original leaf cases now exercise public Schema operations: 14 ECMA
Number-to-string boundary conversions and 29 URL accept/reject decisions. The
only current leaf overrides are the two explicitly registered domain cases.
Seven existing/new-script samples exercise both public schema patterns and the
caller's ordinary `regress` dependency.

The remaining 23 checks cover new/old/non-letter controls, named backreferences,
lookbehind, invalid-regex admission, inert annotations, typed no-progress
failure, optional/required null, untouched raw arguments, parse-failure
precedence, final original-schema validation, bidirectional raw/ACE and percent
resource identities, canonical IDNA mapping, malformed URL/IDNA controls and
IDN hostname/email formats. These checks also run with the caller's default
regex features enabled; an empty or incomplete PASS report is rejected.
