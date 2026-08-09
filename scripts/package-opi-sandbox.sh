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
#   package/bin/opi-sandbox       the executable (canonical mode 0755 on Unix)
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
PACKAGE_HELPER="$SCRIPT_DIR/opi-sandbox-package.py"
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

if [ "$MODE" = "verify" ]; then
    python3 "$PACKAGE_HELPER" verify --artifact-dir "$ARTIFACT_DIR" \
        --archive-suffix .tar.gz --workspace-license "$LICENSE_FILE" \
        --schema-snapshot "$SCHEMA_SNAPSHOT"
    exit $?
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
python3 "$PACKAGE_HELPER" validate-executable --binary "$BINARY" --target "$TARGET" \
    || exit $?
EXEC_SHA="$(sha256_raw "$BINARY")" || {
    echo "package-opi-sandbox: cannot read binary: $BINARY" >&2; exit 2; }

if [ ! -f "$TEMPLATE" ] || [ ! -f "$PACKAGE_HELPER" ]; then
    echo "package-opi-sandbox: template not found: $TEMPLATE" >&2; exit 2
fi
if [ ! -f "$WORKSPACE_MANIFEST" ]; then
    echo "package-opi-sandbox: workspace manifest not found: $WORKSPACE_MANIFEST" >&2; exit 2
fi
if [ ! -f "$SCHEMA_SNAPSHOT" ] || [ ! -f "$LICENSE_FILE" ]; then
    echo "package-opi-sandbox: schema snapshot or LICENSE is missing" >&2
    exit 2
fi

PKG="$ARTIFACT_DIR/package"
mkdir -p "$PKG/bin" "$PKG/schemas" "$PKG/licenses"

# One shared strict SemVer parser and literal renderer is used by both platform
# wrappers. The metadata sidecar contains two already-validated literal lines.
PACKAGE_META="$ARTIFACT_DIR/package-meta.txt"
python3 "$PACKAGE_HELPER" render --workspace-manifest "$WORKSPACE_MANIFEST" \
    --template "$TEMPLATE" --target "$TARGET" --sha256 "$EXEC_SHA" \
    --output "$PKG/package.toml" --metadata-output "$PACKAGE_META" || exit $?
PACKAGE_VERSION="$(sed -n '1p' "$PACKAGE_META")"
OPI_RANGE="$(sed -n '2p' "$PACKAGE_META")"
rm -f "$PACKAGE_META"

# Copy the binary into the layout (basename always opi-sandbox; no extension).
cp "$BINARY" "$PKG/bin/opi-sandbox"
chmod 0755 "$PKG/bin/opi-sandbox"

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
tar -C "$PKG" -czf "$ARCHIVE" \
    package.toml bin/opi-sandbox \
    schemas/command-execution-jsonl-v1.schema.json licenses/LICENSE

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

# Authenticate the archive users receive, through the same independent
# extraction path exposed by --verify, before a workflow can smoke or publish.
python3 "$PACKAGE_HELPER" verify --artifact-dir "$ARTIFACT_DIR" \
    --archive-suffix .tar.gz --workspace-license "$LICENSE_FILE" \
    --schema-snapshot "$SCHEMA_SNAPSHOT"

echo "packaged opi-sandbox for $TARGET: sha256=$EXEC_SHA, layout=$PKG"
exit 0
