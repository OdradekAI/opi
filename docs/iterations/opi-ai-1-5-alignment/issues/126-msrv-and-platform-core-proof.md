# #126 Repair the declared MSRV path and record platform-target proof

Status: core and builtins MSRV checks pass; both archive consumer profiles pass; native Linux/Windows and the declared V/all-features matrix pass; native macOS and capability-granular V remain open.

The first exact MSRV attempt exposed a Cargo manifest compatibility defect. With
Cargo 1.97, `opi-ai` could not inherit `opi-telemetry` and then override
`default-features`; Cargo rejected the manifest before dependency resolution.
The workspace default for `opi-telemetry` is empty, so the smallest compatible
fix moves `default-features = false` to the workspace dependency and removes
the package-local override. This preserves the feature contract while allowing
Cargo 1.97 to parse the workspace.

The pre-fix manifest replay is retained at
`C:/Temp/opi-msrv-platform-20261115/check-msrv-1.97-pre-fix.log`; the temporary
manifest bytes were restored exactly before the repaired checks ran.

## Result

- Rust/Cargo 1.97.0 now checks `opi-ai` with `--no-default-features` on the
  Windows MSVC target.
- The same toolchain checks the `builtins` profile, including the default
  `telemetry-tracing` feature, and the OpenAI Chat response-observer regression
  runs successfully (one test).
- Current stable 1.99.1 checks the core profile for the installed Windows,
  Linux and macOS targets. These are cross-target compilation checks from the
  Windows host; they are not native runner execution.
- The previous Cargo 1.97 parse failure and the pre-fix 1.82 edition failure
  remain in the raw logs and are not overwritten.
- A fresh archive consumer reached the builtins/no-tracing contract test (one
  passed test). Its defaults profile then hit Windows linker `LNK1318` because
  the shared target exhausted E:. Both that environment failure and the
  successful builtins receipt remain preserved. A capacity-backed replay on a
  marked C: target subsequently passed both builtins/no-tracing and defaults
  through metadata, clippy and the response-observer contract test.
- Native Linux (WSL2 Ubuntu 24.04.4, rustc 1.97.1) and native Windows
  (Windows 10.0.22631.6199, rustc 1.99.1) now pass the platform delivery,
  auth-process and OpenAI Chat observer checks. The first Linux run exposed a
  stale platform-delivery string assertion after the manifest fix; the test
  now accepts the workspace-owned `default-features = false` edge and the
  corrected rerun is retained.
- The four currently declared feature unions (`core`, `core-tracing`,
  `builtins-no-tracing`, `defaults`) and `all-features` pass the profile
  contract tests on native Linux and Windows. This freezes the declared
  manifest V. It does not freeze capability-granular V: `builtins` remains a
  coarse group, and independent protocol/auth/catalog/TLS/WS/OAuth/Store and
  higher-order runtime combinations remain open.

## Disposition

The MSRV manifest defect is fixed. This slice proves the declared 1.97 MSRV for
the core and builtins library profiles on Windows, records native Linux and
Windows runtime evidence, and executes the declared feature V/all-features on
both hosts. It does not claim native macOS runtime proof, capability-granular V,
quantitative budget acceptance or real Deferred integration (#94). A native
macOS host remains the next platform-specific evidence when available.

The reviewed incremental cache plan for the new 1.97 target removed 504
payload files (995,901,648 reclaimable bytes) after the MSRV test completed;
the plan digest and protection manifest are retained outside the repository.
The later stable-target package replay used three additional reviewed
incremental-only plans (2,798 files; 4,958,245,976 reclaimable bytes in total)
and explicitly ended the two stale failed-run leases. No target-wide deletion
was performed; E: still had only 801,370,112 free bytes. The defaults consumer
was instead completed against the separately marked C: target, whose receipt
is retained with the two earlier E: failure receipts.

Test impact: `add` MSRV/platform evidence, native Linux/Windows runner receipts,
declared feature V/all-features evidence and focused MSRV observer run;
`update` workspace dependency feature placement and the platform-delivery
manifest assertion; `delete` none; `retain` the initial native failure plus all
Pi differential, adapter, observer, package and historical cache evidence.
