#!/usr/bin/env bash
#
# Host-neutral opi-sandbox package builder (Phase 16 task 16.15.1).
# Builds a distribution archive from an explicit built binary; never invokes opi
# and never claims native restriction success (native run is 16.13/16.14.1).
#
# Usage:
#   package-opi-sandbox.sh --binary PATH --artifact-dir PATH          (pack)
#   package-opi-sandbox.sh --artifact-dir PATH --verify               (verify)
#
# Package layout (under $ARTIFACT_DIR):
#   package/package.toml          rendered manifest (target + sha256 filled)
#   package/bin/opi-sandbox       the executable (chmod +x on Unix)
#   package/schemas/command-execution-jsonl-v1.schema.json
#   package/licenses/LICENSE      project license
#   opi-sandbox-<target>.tar.gz   distribution archive (package contents at root)
#   extracted/                    clean extraction of the archive
#   package-lock.toml             BUILD-TIME audit lock (8 LockMaterial fields)
#   target                        exact target triple for artifact audit
#
# The archive contains package.toml + bin/ + schemas/ + licenses/ at its root
# (NO wrapping directory),
# matching the package_root that 16.5 install passes to 16.4
# validate_executable_contributions. package-lock.toml is an audit artifact;
# 16.5 recomputes LockMaterial via 16.4 against the extracted package and does
# NOT ingest this file.
#
# Exit codes: 0 success; 1 layout/hash mismatch or undecodable lock; 2 usage
# (missing args, missing/empty binary, rustc unavailable, target undetected).
set -euo pipefail

BINARY=""
ARTIFACT_DIR=""
MODE="pack"
while [ "$#" -gt 0 ]; do
    case "$1" in
        --binary) BINARY="${2:?--binary requires a value}"; shift 2 ;;
        --artifact-dir) ARTIFACT_DIR="${2:?--artifact-dir requires a value}"; shift 2 ;;
        --verify) MODE="verify"; shift ;;
        *) echo "package-opi-sandbox: unknown argument: $1" >&2; exit 2 ;;
    esac
done

if [ -z "$ARTIFACT_DIR" ]; then
    echo "usage: package-opi-sandbox.sh --binary PATH --artifact-dir PATH [--verify]" >&2
    exit 2
fi
if [ "$MODE" = "pack" ] && [ -z "$BINARY" ]; then
    echo "package-opi-sandbox: --binary PATH is required in pack mode" >&2
    exit 2
fi

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
TEMPLATE="$SCRIPT_DIR/../packaging/opi-sandbox/package.toml.template"
WORKSPACE_MANIFEST="$SCRIPT_DIR/../Cargo.toml"
SCHEMA_SNAPSHOT="$SCRIPT_DIR/../crates/opi-protocol/tests/snapshots/execution_v1_schema__schema_v1.snap"
LICENSE_FILE="$SCRIPT_DIR/../LICENSE"

# Portable SHA-256: macOS ships `shasum -a 256`; Linux and git-bash ship
# `sha256sum`. Both emit lowercase hex. The stream form reads stdin so the
# caller can pipe LF-normalized text or raw bytes.
hash_stream() {
    if command -v sha256sum >/dev/null 2>&1; then
        sha256sum | awk '{print $1}'
    else
        shasum -a 256 | awk '{print $1}'
    fi
}
# SHA-256 over the LF-normalized bytes of a file (drops every 0x0D), matching
# execution::contribution::lf_normalize + sha256_hex used by 16.4.
sha256_lf() { tr -d '\r' < "$1" | hash_stream; }
# SHA-256 over the raw bytes of a file (no CR stripping: binary may contain 0x0D).
sha256_raw() { hash_stream < "$1"; }

# Read `key = "value"` from the build-time lock (fixed format, single quotes
# never used by the emitter).
lock_value() {
    grep "^$1 = " "$2" | sed "s/^$1 = \"//;s/\"$//"
}

if [ "$MODE" = "verify" ]; then
    PKG="$ARTIFACT_DIR/package"
    EXTRACTED="$ARTIFACT_DIR/extracted"
    LOCK="$ARTIFACT_DIR/package-lock.toml"
    for f in "$PKG/package.toml" "$PKG/bin/opi-sandbox" \
             "$PKG/schemas/command-execution-jsonl-v1.schema.json" \
             "$PKG/licenses/LICENSE" \
             "$EXTRACTED/package.toml" "$EXTRACTED/bin/opi-sandbox" "$LOCK" \
             "$EXTRACTED/schemas/command-execution-jsonl-v1.schema.json" \
             "$EXTRACTED/licenses/LICENSE" \
             "$ARTIFACT_DIR/target"; do
        [ -f "$f" ] || { echo "package-opi-sandbox: verify: missing $f" >&2; exit 1; }
    done
    declared_mh="$(lock_value manifest_hash "$LOCK")"
    declared_exe="$(lock_value executable_sha256 "$LOCK")"
    [ -n "$declared_mh" ] && [ -n "$declared_exe" ] || {
        echo "package-opi-sandbox: verify: undecodable lock" >&2; exit 1; }
    actual_mh="$(sha256_lf "$PKG/package.toml")"
    [ "$actual_mh" = "$declared_mh" ] || {
        echo "package-opi-sandbox: verify: manifest_hash mismatch" >&2; exit 1; }
    exe_pkg="$(sha256_raw "$PKG/bin/opi-sandbox")"
    exe_ext="$(sha256_raw "$EXTRACTED/bin/opi-sandbox")"
    [ "$exe_pkg" = "$declared_exe" ] || {
        echo "package-opi-sandbox: verify: package executable sha mismatch" >&2; exit 1; }
    [ "$exe_ext" = "$declared_exe" ] || {
        echo "package-opi-sandbox: verify: extracted executable sha mismatch" >&2; exit 1; }
    cmp -s "$PKG/schemas/command-execution-jsonl-v1.schema.json" \
        "$EXTRACTED/schemas/command-execution-jsonl-v1.schema.json" || {
        echo "package-opi-sandbox: verify: extracted schema mismatch" >&2; exit 1; }
    cmp -s "$PKG/licenses/LICENSE" "$EXTRACTED/licenses/LICENSE" || {
        echo "package-opi-sandbox: verify: extracted license mismatch" >&2; exit 1; }
    echo "verified opi-sandbox layout: manifest_hash=$actual_mh, executable_sha256=$declared_exe"
    exit 0
fi

# --- pack mode ---
mkdir -p "$ARTIFACT_DIR"
# Re-packaging wipes prior outputs (clean staging tree, no stale overlay).
shopt -s nullglob
rm -rf "$ARTIFACT_DIR/package" "$ARTIFACT_DIR/extracted" "$ARTIFACT_DIR"/opi-sandbox-*.tar.gz
rm -f "$ARTIFACT_DIR/target"
shopt -u nullglob

# Detect host target triple from rustc. This assumes the supplied --binary was
# built for this same triple (native build); cross-compiled binaries must be
# packaged on a matching host.
if ! command -v rustc >/dev/null 2>&1; then
    echo "package-opi-sandbox: rustc not found on PATH; cannot detect target" >&2
    exit 2
fi
TARGET="$(rustc -vV | sed -n 's/^host: //p' | tr -d '[:space:]')"
if [ -z "$TARGET" ]; then
    echo "package-opi-sandbox: could not parse host triple from rustc -vV" >&2
    exit 2
fi

# Validate + hash the input binary up front (fail fast before writing layout).
if [ ! -f "$BINARY" ]; then
    echo "package-opi-sandbox: binary not found: $BINARY" >&2; exit 2
fi
if [ ! -s "$BINARY" ]; then
    echo "package-opi-sandbox: binary is empty: $BINARY" >&2; exit 2
fi
EXEC_SHA="$(sha256_raw "$BINARY")" || {
    echo "package-opi-sandbox: cannot read binary: $BINARY" >&2; exit 2; }

if [ ! -f "$TEMPLATE" ]; then
    echo "package-opi-sandbox: template not found: $TEMPLATE" >&2; exit 2
fi
if [ ! -f "$WORKSPACE_MANIFEST" ]; then
    echo "package-opi-sandbox: workspace manifest not found: $WORKSPACE_MANIFEST" >&2; exit 2
fi
if [ ! -f "$SCHEMA_SNAPSHOT" ] || [ ! -f "$LICENSE_FILE" ]; then
    echo "package-opi-sandbox: schema snapshot or LICENSE is missing" >&2
    exit 2
fi

# The package identity and compatibility window come from the same checkout as
# this packager. This prevents release/template literals from drifting away
# from the host version that validates the contribution.
PACKAGE_VERSION="$(awk '
    /^\[workspace\.package\][[:space:]]*$/ { in_workspace_package=1; next }
    /^\[/ { in_workspace_package=0 }
    in_workspace_package && /^[[:space:]]*version[[:space:]]*=/ {
        line=$0
        sub(/^[^=]*=[[:space:]]*"/, "", line)
        sub(/"[[:space:]]*$/, "", line)
        print line
        exit
    }
' "$WORKSPACE_MANIFEST")"
VERSION_CORE="${PACKAGE_VERSION%%-*}"
IFS=. read -r VERSION_MAJOR VERSION_MINOR VERSION_PATCH VERSION_EXTRA <<EOF
$VERSION_CORE
EOF
if [ -z "$PACKAGE_VERSION" ] || [ -z "${VERSION_MAJOR:-}" ] || \
   [ -z "${VERSION_MINOR:-}" ] || [ -z "${VERSION_PATCH:-}" ] || \
   [ -n "${VERSION_EXTRA:-}" ]; then
    echo "package-opi-sandbox: invalid workspace package version: $PACKAGE_VERSION" >&2
    exit 2
fi
case "$VERSION_MAJOR$VERSION_MINOR$VERSION_PATCH" in
    *[!0-9]*)
        echo "package-opi-sandbox: invalid workspace package version: $PACKAGE_VERSION" >&2
        exit 2
        ;;
esac
OPI_RANGE=">=$VERSION_MAJOR.$VERSION_MINOR,<$VERSION_MAJOR.$((VERSION_MINOR + 1))"

PKG="$ARTIFACT_DIR/package"
mkdir -p "$PKG/bin" "$PKG/schemas" "$PKG/licenses"

# Render the manifest (substitute tokens) and write LF-only bytes. EXEC_SHA is
# lowercase hex and TARGET is a triple; neither contains sed metacharacters.
sed -e "s/__PACKAGE_VERSION__/$PACKAGE_VERSION/g" \
    -e "s/__OPI_RANGE__/$OPI_RANGE/g" \
    -e "s/__TARGET__/$TARGET/g" -e "s/__SHA256__/$EXEC_SHA/g" "$TEMPLATE" \
    | tr -d '\r' > "$PKG/package.toml"

# Copy the binary into the layout (basename always opi-sandbox; no extension).
cp "$BINARY" "$PKG/bin/opi-sandbox"
chmod +x "$PKG/bin/opi-sandbox"

# The reviewed schema snapshot is the byte-pinned output of opi-protocol's
# generator. Strip only insta's metadata header and package the JSON document.
tr -d '\r' < "$SCHEMA_SNAPSHOT" \
    | awk 'BEGIN { markers=0 } /^---$/ { markers++; next } markers >= 2 { print }' \
    > "$PKG/schemas/command-execution-jsonl-v1.schema.json"
grep -q '"$id": "https://odradek.ai/schemas/command-execution-jsonl-v1.json"' \
    "$PKG/schemas/command-execution-jsonl-v1.schema.json" || {
    echo "package-opi-sandbox: invalid protocol schema snapshot" >&2; exit 2; }
cp "$LICENSE_FILE" "$PKG/licenses/LICENSE"

# manifest_hash over the LF-normalized written manifest.
MANIFEST_HASH="$(sha256_lf "$PKG/package.toml")"

# Archive: package contents at root (no wrapping directory).
ARCHIVE="$ARTIFACT_DIR/opi-sandbox-$TARGET.tar.gz"
tar -C "$PKG" -czf "$ARCHIVE" .

# Clean extracted staging tree.
EXTRACTED="$ARTIFACT_DIR/extracted"
mkdir -p "$EXTRACTED"
tar -C "$EXTRACTED" -xzf "$ARCHIVE"

# Self-verify (defense-in-depth against a copy/archive bug): the extracted
# executable hashes to the same value as the input binary.
EXTRACTED_SHA="$(sha256_raw "$EXTRACTED/bin/opi-sandbox")"
[ "$EXTRACTED_SHA" = "$EXEC_SHA" ] || {
    echo "package-opi-sandbox: extracted binary hash mismatch" >&2; exit 1; }

# Build-time audit lock (flat LockMaterial table).
cat > "$ARTIFACT_DIR/package-lock.toml" <<EOF
# Build-time audit lock for the opi-sandbox package. NOT consumed by 16.5;
# 16.5 recomputes LockMaterial via 16.4 against the extracted package.
manifest_hash = "$MANIFEST_HASH"
executable_rel_path = "bin/opi-sandbox"
executable_sha256 = "$EXEC_SHA"
package_version = "$PACKAGE_VERSION"
target = "$TARGET"
opi_range = "$OPI_RANGE"
protocol = "command-execution-jsonl-v1"
adapter_id = "opi-sandbox"
EOF
printf '%s\n' "$TARGET" > "$ARTIFACT_DIR/target"

echo "packaged opi-sandbox for $TARGET: sha256=$EXEC_SHA, layout=$PKG"
exit 0
