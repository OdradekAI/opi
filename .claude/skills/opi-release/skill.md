---
name: opi-release
description: Orchestrates the full release process for the opi Rust workspace — publishes to GitHub Releases and crates.io with phased safety gates
arguments: "<version> [--fix] [--skip-cross]"
---

# opi-release

Release the opi Rust workspace to GitHub Releases and crates.io.

## Arguments

- `<version>` — target semver version (e.g., `0.2.0`)
- `--fix` — auto-fix fmt/clippy issues during pre-flight
- `--skip-cross` — skip cross-compilation (source-only release)

## Architecture: Seven Phased Gates

Each phase reports status and requires user confirmation before proceeding.

```
Phase 1: Pre-flight checks
Phase 2: Version bump + dry-run validation
Phase 3: Changelog generation
Phase 4: Build, cross-compile & artifact self-check
Phase 5: Commit, tag, push & GitHub Draft Release (PARTIALLY reversible)
Phase 6: Publish to crates.io (IRREVERSIBLE)
Phase 7: Finalize & post-release verification
```

### Irreversibility Boundaries
- **Phase 1–4**: Fully reversible. No side effects outside local filesystem.
- **Phase 5**: Commit and tag are PUBLIC immediately. Draft release is private. Rollback = `git revert`.
- **Phase 6**: crates.io publishes are permanent. Yanking hides but does not delete.
- **Phase 7**: Publishing draft release is trivially reversible.

## Phase 1: Pre-flight Checks

Release pre-flight gates are deterministic and must not depend on live provider
or dogfood runs. If a release candidate cites a dogfood report as evidence,
that report must already have passed the opi-implement Artifact Truthfulness
Gate; opi-release does not run live model checks.

Create a TaskCreate for Phase 1. Run ALL checks below and report a summary table.

### 1.1 File Presence
```bash
test -f LICENSE && test -f README.md && test -f Cargo.lock
# Each crate must have README.md or readme field in Cargo.toml
```

### 1.2 Git State
```bash
git status --porcelain  # must be empty
git branch --show-current  # must be "main"
git fetch origin && git diff origin/main..HEAD --stat  # must be empty
git tag -l "v$VERSION"  # must NOT exist
git ls-remote --tags origin "refs/tags/v$VERSION"  # must NOT exist
```

### 1.3 CI Status (HEAD-bound)
```bash
HEAD_SHA=$(git rev-parse HEAD)
gh api repos/OdradekAI/opi/commits/$HEAD_SHA/check-runs \
  --jq '.check_runs[] | {name, conclusion}'
```
ALL required checks must be `success` for the exact HEAD SHA. If any `failure` or `pending`: BLOCKED.

### 1.4 Code Quality
```bash
cargo fmt --check --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --all-targets
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps
```
If `--fix` flag: run `cargo fmt --all` and `cargo clippy --fix --workspace --allow-dirty` first, then re-verify.

### 1.5 Security & Dependencies
```bash
cargo audit  # if installed
# Verify no git/patch deps, no path deps outside workspace
grep -r '\[patch\]' Cargo.toml  # must be empty
# Check internal deps have version field
cargo metadata --format-version 1 --no-deps | jq '.packages[].dependencies[] | select(.path != null) | .req'
# No secrets in tracked files
git ls-files '*.env' '*.key' '*.pem' '*.p12' '*.pfx'  # must be empty
grep -rn 'AKIA\|sk-\|ghp_\|glpat-' --include='*.rs' --include='*.toml'
```

### 1.6 MSRV
If `rust-version` is set in workspace, verify: `cargo +<msrv> check --workspace`. If not set, emit WARN.

### 1.7 Release Metadata & Permissions
```bash
# Each crate has description, license, repository
cargo metadata --format-version 1 --no-deps | jq '.packages[] | {name, description, license, repository}'
# Verify auth
test -f ~/.cargo/credentials.toml || test -n "$CARGO_REGISTRY_TOKEN"
# Verify ownership (will fail for first-time publish — handle gracefully)
for crate in opi-ai opi-tui opi-agent opi-coding-agent; do
  cargo owner --list $crate 2>/dev/null || echo "NEW_CRATE:$crate"
done
# Version not already published
cargo search opi-ai --limit 1 | grep -v "$VERSION"
```

### 1.8 Package Content Check
```bash
for crate in opi-ai opi-tui opi-agent opi-coding-agent; do
  cargo package -p $crate --list 2>/dev/null
done
```
Verify: no files >1MB, no `.env`/IDE configs, total <5MB per crate.

### 1.9 Version Semantics Check
```bash
LAST_TAG=$(git describe --tags --abbrev=0 --match='v*' 2>/dev/null || echo "")
if [ -n "$LAST_TAG" ]; then
  git log "$LAST_TAG"..HEAD --pretty=format:'%s' | grep -E '^(feat!|BREAKING)'
fi
```
Warn if version bump doesn't match commit types (breaking→major, feat→minor, fix→patch).

### 1.10 CLI Version Command
```bash
cargo run -p opi-coding-agent -- --version 2>/dev/null
```
If no `--version` flag exists: **BLOCKED** — prerequisite not met.

### 1.11 Post-Fix Re-verification (only if --fix used)
After auto-fix, commit changes and re-verify clean state:
```bash
git add -A && git commit -m "chore: pre-release auto-fix"
git status --porcelain  # must be empty
```

### Phase 1 Report
Output a summary table:
```
Check               | Status   | Details
--------------------|----------|--------
File Presence       | PASS     |
Git State           | PASS     |
CI Status           | PASS     | All checks passed for <sha>
Code Quality        | PASS/FAIL|
Tests               | PASS     |
Security            | PASS     |
Dependencies        | PASS     |
MSRV                | WARN     |
Package Content     | PASS     |
Release Metadata    | PASS     |
Crate Ownership     | PASS     |
Version Semantics   | WARN     |
CLI --version       | PASS     |
```
If any critical check FAIL: **BLOCKED**. If all critical pass: ask user to proceed to Phase 2.

## Phase 2: Version Bump + Dry-Run Validation

Create a TaskCreate for Phase 2.

### 2.1 Version Update
```bash
VERSION="$1"  # from skill argument
# Update workspace.package.version
sed -i "s/^version = \".*\"/version = \"$VERSION\"/" Cargo.toml
# Update ALL internal [workspace.dependencies] version fields
# e.g., opi-ai = { path = "crates/opi-ai", version = "=0.1.0" } → version = "=$VERSION"
cargo check --workspace  # verify update is valid
```
Show diff of changed lines to user.

### 2.2 Publish Dry-Run Gate
After version bump, validate each crate can be packaged:
```bash
# Compute publish order from cargo metadata
ORDER=$(cargo metadata --format-version 1 --no-deps | \
  jq -r '.packages[] | select(.publish == null or .publish != []) | .name')
for crate in $ORDER; do
  cargo publish --dry-run -p $crate
done
```
If dry-run fails: **BLOCKED** — revert with `git checkout -- Cargo.toml`.

## Phase 3: Changelog Generation

Create a TaskCreate for Phase 3.

### 3.1 Commit Collection
```bash
LAST_TAG=$(git describe --tags --abbrev=0 --match='v*' 2>/dev/null || echo "")
if [ -n "$LAST_TAG" ]; then
  git log "$LAST_TAG"..HEAD --pretty=format:'%H|%s|%an'
else
  git log --pretty=format:'%H|%s|%an'
fi
```

### 3.2 Categorization (Conventional Commits)
Parse and group:
- `feat:` → **Added**
- `fix:` → **Fixed**
- `perf:` → **Performance**
- `docs:` → **Documentation**
- `refactor:` → **Changed**
- `BREAKING CHANGE` / `feat!:` → **Breaking Changes**
- `chore:` → omit from changelog

### 3.3 Format
Generate `CHANGELOG.md` entry (Keep a Changelog format):
```markdown
## [<version>] - YYYY-MM-DD

### Breaking Changes
- Description ([#N](https://github.com/OdradekAI/opi/issues/N))

### Added
- Description

### Fixed
- Description
```
Extract `#<number>` from commits and link to GitHub issues/PRs.
Prepend new entry to `CHANGELOG.md` (create file if missing).
Also generate GitHub Release notes (same content, for `--notes-file`).

## Phase 4: Build & Cross-Compile

Create a TaskCreate for Phase 4.

### Build Strategy Selection

Ask user which build strategy to use:
1. **CI-driven** (recommended) — Only build native platform locally for smoke-testing. After Phase 5 pushes the tag, the `release.yml` workflow builds all 6 targets in CI and uploads them to the GitHub Release automatically.
2. **Local cross-compile** — Build all available targets locally using `cross`. Use when CI is unavailable or for pre-CI verification.
3. **Skip builds** (`--skip-cross`) — Source-only release, no binary artifacts.

### 4.1 Release Build
```bash
cargo build --release --workspace
cargo test --release --workspace
```

### 4.2 Host Capability Detection
```bash
HOST=$(rustc -vV | grep host | awk '{print $2}')
cross --version 2>/dev/null  # check if cross is available
```

Determine buildable targets based on host:

| Target | Buildable when |
|--------|---------------|
| `x86_64-unknown-linux-gnu` | Linux host OR `cross` available |
| `aarch64-unknown-linux-gnu` | `cross` available |
| `x86_64-apple-darwin` | macOS host only |
| `aarch64-apple-darwin` | macOS host only |
| `x86_64-pc-windows-msvc` | Windows host OR `cargo-xwin` |
| `aarch64-pc-windows-msvc` | Windows arm64 host or CI |

Report which targets will be built and which are skipped.
If `--skip-cross`: only build native platform.

### 4.3 Cross-Compilation
For each buildable target:
```bash
# Native target
cargo build --release --target $TARGET -p opi-coding-agent
# Cross target (using cross)
cross build --release --target $TARGET -p opi-coding-agent
```

### 4.4 Asset Packaging
All build artifacts go into `release-artifacts/v$VERSION/`.
```bash
mkdir -p release-artifacts/v$VERSION
# Linux/macOS: tar.gz
tar -czf release-artifacts/v$VERSION/opi-$PLATFORM.tar.gz -C target/$TARGET/release opi README.md LICENSE
# Windows: zip
zip release-artifacts/v$VERSION/opi-$PLATFORM.zip target/$TARGET/release/opi.exe README.md LICENSE
# Checksums (local integrity verification, NOT uploaded to GitHub Release)
cd release-artifacts/v$VERSION && sha256sum opi-*.tar.gz opi-*.zip > SHA256SUMS.txt
```

### 4.5 Artifact Self-Check
For each archive in `release-artifacts/v$VERSION/`:
1. Unpack to temp dir, verify expected files exist
2. Native platform: run `./opi --version`, confirm output = target version
3. Cross-compiled: verify file type with `file` command
4. Verify SHA256SUMS.txt covers all archives: `cd release-artifacts/v$VERSION && sha256sum -c SHA256SUMS.txt`
5. Warn if any archive >50MB

## Phase 5: Commit, Tag, Push & GitHub Draft Release

Create a TaskCreate for Phase 5.

### IRREVERSIBILITY WARNING — Present to user before proceeding:
> "Phase 5 will push a release commit and tag to origin/main. This is publicly visible immediately. The draft release itself is private, but the commit and tag are not. Proceed?"

Wait for explicit user confirmation.

### 5.1 Commit & Tag
```bash
git add Cargo.toml Cargo.lock CHANGELOG.md
git commit -m "chore: release v$VERSION"
git tag -a "v$VERSION" -m "Release v$VERSION"
git push origin main --follow-tags
```

### 5.2 Create Draft Release
```bash
gh release create "v$VERSION" \
  --draft \
  --title "v$VERSION" \
  --notes-file release-notes.md \
  release-artifacts/v$VERSION/opi-*.tar.gz \
  release-artifacts/v$VERSION/opi-*.zip
```
Only upload archive files (tar.gz/zip). Do NOT upload SHA256SUMS.txt to GitHub Release.
Only include archives that were actually built (skip unavailable targets).

**CI-driven strategy:** If using CI-driven builds, create the draft release with only the release notes (no local artifacts). After the tag push triggers `release.yml`, CI builds all 6 targets and uploads them via `gh release upload --clobber`. Wait for the workflow to complete before proceeding to Phase 7:
```bash
gh run list --workflow=release.yml --branch="v$VERSION" --limit=1 --json status
```

### 5.3 User Confirmation Gate
Show draft release URL and ask:
> "Draft GitHub Release v<version> created at <url>. Please review. If correct, I'll proceed with the IRREVERSIBLE crates.io publish. Continue?"

**Do NOT proceed to Phase 6 without explicit user approval.**

## Phase 6: Publish to crates.io (IRREVERSIBLE)

Create a TaskCreate for Phase 6.

### 6.1 Compute Publish Order
```bash
cargo metadata --format-version 1 --no-deps | \
  jq '[.packages[] | select(.manifest_path | startswith("'$(pwd)'")) | {name, deps: [.dependencies[] | select(.path != null) | .name]}]'
```
Build dependency graph → topological sort → publish in batches.
Exclude crates with `publish = false`.

Expected order (computed dynamically, not hardcoded):
- Batch 1: `opi-ai`, `opi-tui` (no internal deps)
- Batch 2: `opi-agent` (depends on Batch 1)
- Batch 3: `opi-coding-agent` (depends on Batch 1 & 2)

### 6.2 Publish Each Batch
```bash
for crate in $BATCH; do
  cargo publish -p $crate
done
# Wait 30s between batches for crates.io index propagation
sleep 30
```

### 6.3 Verification After Each Publish
```bash
cargo search $crate --limit 1 | grep "$VERSION"
```

### 6.4 Error Classification & Retry Policy

**Auto-retryable (up to 3 attempts, exponential backoff):**
- Network timeout / connection reset
- HTTP 5xx from crates.io
- "crate index not updated yet"

**NOT auto-retryable (require user decision):**
- HTTP 4xx (auth failure, validation error, version conflict)
- `cargo publish` explicit error (missing field, dep not found)
- Success reported but verification fails (partial state)

### 6.5 Failure Decision Gate
If publish fails mid-batch, present options to user:

> "cargo publish -p <crate> failed: <error>. Already published: <list>.
> Options:
> 1. Retry — fix the issue and retry this crate
> 2. Wait & retry — wait 60s for index propagation then retry
> 3. Yank & abort — yank all published crates, abort release
> 4. Continue later — save progress, exit skill
>
> Choose:"

Do NOT auto-retry on explicit errors. Use AskUserQuestion for this gate.

## Phase 7: Finalize & Post-Release Verification

Create a TaskCreate for Phase 7.

### 7.1 Publish Draft Release
```bash
gh release edit "v$VERSION" --draft=false
```

### 7.2 Post-Release Verification

**crates.io install test:**
```bash
cargo install opi-coding-agent --version $VERSION
opi --version  # must output $VERSION
```

**GitHub Release asset check:**
```bash
gh release download "v$VERSION" -D /tmp/opi-verify
# Verify against local checksums
cd /tmp/opi-verify && sha256sum -c ../../../release-artifacts/v$VERSION/SHA256SUMS.txt
# Run binary if native platform
# (unpack native archive, run ./opi --version)
```

**docs.rs build status (all crates):**
```bash
for crate in opi-ai opi-tui opi-agent opi-coding-agent; do
  STATUS=$(curl -s -o /dev/null -w "%{http_code}" "https://docs.rs/$crate/$VERSION")
  echo "$crate: $STATUS"
done
```
- 200 = built, 404 = not yet built (non-blocking, may take minutes)

### 7.3 Final Report
```
Release v<version> complete!

Published crates:
  - opi-ai v<version>           https://crates.io/crates/opi-ai
  - opi-tui v<version>          https://crates.io/crates/opi-tui
  - opi-agent v<version>        https://crates.io/crates/opi-agent
  - opi-coding-agent v<version> https://crates.io/crates/opi-coding-agent

GitHub Release: https://github.com/OdradekAI/opi/releases/tag/v<version>
Install: cargo install opi-coding-agent
```

## Failure Recovery

### Phase 1-3 Failures
No side effects. Fix issue and re-run skill.

### Phase 4 Failures
Clean build artifacts: `cargo clean`. Fix and retry.

### Phase 5 Failures
If commit/tag pushed but draft release creation failed:
- Retry `gh release create --draft` (idempotent with same tag)

If user rejects the draft release:
```bash
gh release delete "v$VERSION" --yes
git push origin :refs/tags/v$VERSION
git tag -d "v$VERSION"
git revert HEAD --no-edit && git push origin main
```
**NEVER** use `git reset --hard` + `git push --force` automatically.

### Phase 6 Failures
Already-published crates cannot be unpublished (only yanked).
Use the Failure Decision Gate (section 6.5) to let user choose action.

If user chooses "Continue later":
- Save progress to `.opi-release-state.json`:
  ```json
  {"version": "<version>", "published": ["opi-ai", "opi-tui"], "pending": ["opi-agent"]}
  ```
- On next invocation with same version, detect state file and resume

If user chooses "Yank & abort":
```bash
for crate in $PUBLISHED; do
  cargo yank -p $crate --version $VERSION
done
# Then Phase 5 rollback
```

## Resume Support

On skill invocation, check for `.opi-release-state.json`:
```bash
test -f .opi-release-state.json && cat .opi-release-state.json
```
If found and version matches argument, ask user:
> "Found incomplete release state for v<version>. <N> crates already published. Resume from where it left off?"

If yes, skip to Phase 6 and only publish remaining crates.

## Tools Required

- `cargo` (Rust toolchain)
- `cross` (optional, for local cross-compilation: `cargo install cross`)
- `gh` (GitHub CLI, authenticated)
- `git`
- `cargo-audit` (optional, for security checks)

## CI Workflows

- `.github/workflows/ci.yml` — Runs on push/PR to main: fmt, clippy, test, doc. Makes Phase 1.3 meaningful.
- `.github/workflows/release.yml` — Triggered by `v*` tags or manual dispatch. Builds all 6 platform targets (linux-x64, linux-arm64, darwin-x64, darwin-arm64, windows-x64, windows-arm64) and uploads to the GitHub Release.

## Post-Release Cleanup

After Phase 7 completes (or on abort), remove transient release artifacts:
```bash
rm -f release-notes.md
```
The `release-artifacts/v$VERSION/` directory is retained for local reference (checksums, archives). It is in `.gitignore` and does not pollute the repo.
