# Opi Local Build Storage Relocation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Remove validated Opi build artifacts from C: and D: and ensure future builds for this checkout place project output, managed caches, and Cargo-subprocess temporary files under `E:/opi`.

**Architecture:** Use an untracked checkout-local Cargo configuration for direct Cargo output and Cargo child-process temporary files, while retaining the existing Opi-specific user environment variable for pre-Cargo cache scripts. Perform destructive cleanup through exact resolved-path allowlists and protect active processes, leases, Git worktrees, source, and ledgers.

**Tech Stack:** PowerShell, Cargo configuration, Git worktree metadata, `scripts/opi-cargo-cache.py`.

---

### Task 1: Freeze the cleanup preconditions

**Files:**
- Inspect: `D:/Luiz/Odradek/opi/.git`
- Inspect: `D:/Luiz/Odradek/opi/.opi-impl-state.json`
- Inspect: `C:/Users/Luiz/AppData/Local/opi/cargo-targets`
- Inspect: `C:/Users/Luiz/AppData/Local/Temp`

- [x] **Step 1: Confirm no build process is active**

Run:

```powershell
$active = Get-Process cargo,rustc,sccache -ErrorAction SilentlyContinue
if ($active) { $active | Select-Object Id, ProcessName, Path; throw 'Build process is active' }
```

Expected: no process rows and exit code 0.

- [x] **Step 2: Record registered worktree roots**

Run:

```powershell
git -C 'D:\Luiz\Odradek\opi' worktree list --porcelain
```

Expected: the main worktree and any registered temporary worktree are listed.
Every listed path is added to the protected-root set before cleanup.

- [x] **Step 3: Confirm the Opi cache lease state**

Run:

```powershell
python 'D:\Luiz\Odradek\opi\scripts\opi-cargo-cache.py' status --root 'C:\Users\Luiz\AppData\Local\opi\cargo-targets'
```

Expected: every C: cache selected for deletion reports no active lease. Stop if
any cache is active.

### Task 2: Install checkout-local E: routing

**Files:**
- Create: `D:/Luiz/Odradek/opi/.cargo/config.toml` (local-only)
- Modify: `D:/Luiz/Odradek/opi/.git/info/exclude` (local Git metadata)

- [x] **Step 1: Create the destination directories**

Run:

```powershell
New-Item -ItemType Directory -Force -Path 'E:\opi\cargo-targets\windows-direct' | Out-Null
New-Item -ItemType Directory -Force -Path 'E:\opi\tmp' | Out-Null
```

Expected: both paths exist and resolve below `E:\opi`.

- [x] **Step 2: Create the local Cargo configuration**

Create `D:/Luiz/Odradek/opi/.cargo/config.toml` with exactly:

```toml
[build]
target-dir = "E:/opi/cargo-targets/windows-direct"

[env]
OPI_CARGO_CACHE_ROOT = { value = "E:/opi/cargo-targets", force = true }
TEMP = { value = "E:/opi/tmp", force = true }
TMP = { value = "E:/opi/tmp", force = true }
```

- [x] **Step 3: Keep the machine-local file out of Git**

Add exactly this line to `D:/Luiz/Odradek/opi/.git/info/exclude` if absent:

```gitignore
/.cargo/config.toml
```

Run:

```powershell
git -C 'D:\Luiz\Odradek\opi' check-ignore -v '.cargo/config.toml'
```

Expected: `.git/info/exclude` is the matching rule.

- [x] **Step 4: Pin the Opi-specific cache root for future shells**

Run:

```powershell
[Environment]::SetEnvironmentVariable('OPI_CARGO_CACHE_ROOT', 'E:\opi\cargo-targets', 'User')
$env:OPI_CARGO_CACHE_ROOT = 'E:\opi\cargo-targets'
```

Expected: process and user values both equal `E:\opi\cargo-targets`.

### Task 3: Remove exact C: and D: build artifacts

**Files:**
- Delete: `D:/Luiz/Odradek/opi/target`
- Delete: inactive marked caches below `C:/Users/Luiz/AppData/Local/opi/cargo-targets`
- Delete: validated artifact directories below `C:/Users/Luiz/AppData/Local/Temp/opi-*`

- [x] **Step 1: Remove the exact repository target directory**

Run one PowerShell script that resolves
`D:\Luiz\Odradek\opi\target`, verifies that its parent is exactly
`D:\Luiz\Odradek\opi`, and then calls `Remove-Item -LiteralPath ... -Recurse
-Force`.

Expected: the target directory no longer exists; source paths remain.

- [x] **Step 2: Remove inactive marked C: Opi caches**

Run the repository's guarded cache-pruning command:

```powershell
python 'D:\Luiz\Odradek\opi\scripts\opi-cargo-cache.py' prune --root 'C:\Users\Luiz\AppData\Local\opi\cargo-targets' --max-gib 0 --older-than-days 0 --execute
```

The script requires that each resolved candidate is an immediate child of the
selected root, contains `.opi-cargo-cache.json`, is not a symlink, and has no
live `.opi-cargo-lease-*.json` PID before deleting it. Expected: the old marked
cache is absent and the parent is retained.

- [x] **Step 3: Remove temporary roots that are Cargo targets**

Enumerate immediate `opi-*` children of
`C:\Users\Luiz\AppData\Local\Temp`. A whole root is eligible only if all of
these exact predicates hold:

```text
resolved parent == C:\Users\Luiz\AppData\Local\Temp
name starts with opi-
.rustc_info.json exists directly below the root
debug exists directly below the root
Cargo.toml does not exist directly below the root
.git does not exist directly below the root
root is not in the registered-worktree protected set
```

Delete only eligible roots in the same PowerShell process. Expected eligible
roots include the `opi-phase15-final-review`, `opi-phase15-ledger-sync`, and
`opi-phase15-marker-*` Cargo target directories.

- [x] **Step 4: Remove artifact subtrees from Opi temporary roots**

Build the protected-root set from `git worktree list --porcelain`. Enumerate
only resolved directories below
`C:\Users\Luiz\AppData\Local\Temp` whose first path component matches
`opi-*`. Delete an artifact subtree only when its leaf is one of:

```text
target
cargo-targets
incremental
```

Require that the resolved target remains below an `opi-*` temporary root and
is not `.git`, `.opi-impl-state.json`, or a protected worktree root. Delete
deepest paths first. Expected targets include the `target` directory below the
unregistered `opi-phase16-audit-*` source tree and the `tree/target` directory
below `opi-phase16-audit-codex-*`. Build artifacts are removed without deleting
registered worktrees or source.

- [x] **Step 5: Classify empty disposable Opi temporary roots**

Classify an `opi-*` temporary root as disposable only when it is empty after
artifact cleanup and is not in the protected-root set. Direct empty-directory
deletion is blocked by the execution policy, so retain the zero-byte roots
rather than bypassing that policy. Unclassified non-empty roots remain and are
printed for verification.

### Task 4: Verify routing and absence of regression

**Files:**
- Verify: `D:/Luiz/Odradek/opi/.cargo/config.toml`
- Verify: `D:/Luiz/Odradek/opi/scripts/opi-cargo-cache.py`

- [x] **Step 1: Verify Cargo's resolved target directory**

Run:

```powershell
$metadata = cargo metadata --no-deps --format-version 1 | ConvertFrom-Json
$resolved = [IO.Path]::GetFullPath($metadata.target_directory)
if (-not $resolved.StartsWith('E:\opi\', [StringComparison]::OrdinalIgnoreCase)) {
    throw "Unexpected Cargo target directory: $resolved"
}
$resolved
```

Expected: `E:\opi\cargo-targets\windows-direct`.

- [x] **Step 2: Verify the managed cache root**

Run:

```powershell
python 'D:\Luiz\Odradek\opi\scripts\opi-cargo-cache.py' status
```

Expected: the reported cache root and all managed entries are below
`E:\opi\cargo-targets`.

- [x] **Step 3: Run a focused build check**

Record the current time, then run:

```powershell
cargo check -p opi-protocol
```

Expected: exit code 0. The command may update E: caches but must not recreate
`D:\Luiz\Odradek\opi\target` or the old C: managed cache.

- [x] **Step 4: Scan C: and D: for new Opi artifacts**

Confirm all of the following:

```text
D:\Luiz\Odradek\opi\target does not exist
C:\Users\Luiz\AppData\Local\opi\cargo-targets has no marked Opi cache
no Cargo-target root identified by .rustc_info.json plus direct debug remains below C:\...\Temp
no target/cargo-targets/incremental directory below a C:\...\Temp\opi-* root
```

Print every retained non-empty `opi-*` temporary root and its protection
reason. Expected: no build-artifact violation; retained paths are source,
registered worktrees, ledgers, or unclassified data.

- [x] **Step 5: Record test impact**

Test impact: `none`. This task changes machine-local routing and deletes
regenerable build artifacts; it does not change tracked runtime behavior or
tests. Do not create a Git commit unless the user separately requests one.
