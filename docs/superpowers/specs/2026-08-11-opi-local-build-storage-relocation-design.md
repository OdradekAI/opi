# Opi Local Build Storage Relocation Design

## Context

Opi builds have left project-owned artifacts on both the repository drive and
the Windows user profile drive. The current machine already has a user Cargo
configuration that points the normal Cargo target directory at
`E:/opi/cargo-targets/windows-direct`, and `OPI_CARGO_CACHE_ROOT` is set to
`E:\opi\cargo-targets`, but older build trees and workflow scratch directories
remain on C: and D:.

The observed reclaim candidates are:

- `D:\Luiz\Odradek\opi\target`: about 32.57 GiB.
- `C:\Users\Luiz\AppData\Local\opi\cargo-targets`: about 29.93 GiB.
- `C:\Users\Luiz\AppData\Local\Temp\opi-*`: about 89.28 GiB before
  classification.

The C: temporary candidates include at least one registered Git worktree, so a
blanket recursive deletion is unsafe.

## Scope

This change relocates only Opi project build output, Opi-managed Cargo caches,
and temporary files created by Cargo subprocesses for this checkout.

It does not relocate or delete:

- `C:\Users\Luiz\.cargo`, including credentials, installed binaries, registry
  cache, and Git cache.
- `C:\Users\Luiz\.rustup` or installed Rust toolchains.
- Windows user or machine `TEMP` and `TMP` settings.
- Existing caches under `E:\opi`.
- Source files, dirty Git worktrees, or implementation ledgers.

## Chosen Design

### Project-local Cargo routing

Create an untracked local Cargo configuration at `.cargo/config.toml` for this
checkout. It sets:

```toml
[build]
target-dir = "E:/opi/cargo-targets/windows-direct"

[env]
OPI_CARGO_CACHE_ROOT = { value = "E:/opi/cargo-targets", force = true }
TEMP = { value = "E:/opi/tmp", force = true }
TMP = { value = "E:/opi/tmp", force = true }
```

The file is added to `.git/info/exclude`, not the repository `.gitignore`,
because the E: drive is machine-specific. This prevents a Windows-local path
from becoming part of Opi's cross-platform contract.

The user-level `OPI_CARGO_CACHE_ROOT` remains
`E:\opi\cargo-targets`, ensuring that `scripts/opi-cargo-cache.py` resolves the
same root before Cargo starts. The shared Cargo and Rustup homes remain on C:.

### E: directory layout

The local machine uses:

```text
E:\opi\
  cargo-targets\
    windows-direct\
    opi-<workspace-toolchain-key>\
  tmp\
```

`windows-direct` owns ordinary direct Cargo output. The keyed `opi-*`
directories remain managed by `scripts/opi-cargo-cache.py`. `tmp` is used by
build scripts, test processes, and other subprocesses launched by Cargo for
this checkout.

### Cleanup policy

Every destructive target must be resolved to an absolute path and pass an
allowlist check in the same PowerShell process that deletes it. Cleanup is
blocked while Cargo, rustc, or sccache is active.

The following categories may be removed:

1. The exact repository target directory
   `D:\Luiz\Odradek\opi\target`.
2. Marked, inactive Opi Cargo caches below
   `C:\Users\Luiz\AppData\Local\opi\cargo-targets` after confirming that no
   lease is active.
3. An `opi-*` temporary root that is itself a Cargo target directory, identified
   by `.rustc_info.json` plus a direct `debug` directory and by the absence of
   `.git` and `Cargo.toml`.
4. Build-artifact subdirectories inside
   `C:\Users\Luiz\AppData\Local\Temp\opi-*`, provided the resolved directory
   remains below that root and is not Git metadata, source, or a ledger.
5. Empty disposable `opi-*` temporary directories after artifact cleanup,
   excluding every registered Git worktree root.

Registered worktree roots are never removed as ordinary temporary directories.
If a registered worktree contains a build-artifact directory, only that exact
artifact directory may be removed. Unclassified non-empty directories are
retained and reported.

## Verification

Verification has four gates:

1. `cargo metadata --no-deps --format-version 1` reports a target directory
   below `E:/opi`.
2. `scripts/opi-cargo-cache.py status` reports its root below `E:/opi` and no
   active cache on C:.
3. A focused `cargo check -p opi-protocol` succeeds using E: output.
4. A post-build scan finds no newly created Opi target/cache artifacts on C:
   or D:. Any retained C: temporary directory is listed with the reason it was
   protected.

## Rollback

Rollback removes the local `.cargo/config.toml` entry from `.git/info/exclude`
and restores or deletes the local configuration file. Deleted build outputs and
caches are not restored; Cargo can regenerate them. Source, credentials, and
toolchains are outside the cleanup scope and therefore do not require rollback.
