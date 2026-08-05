#!/usr/bin/env bash
#
# Standalone acceptance smoke for the opi-sandbox binary (Phase 16 task 16.11.2).
# Launches ONLY the explicit --binary path; never invokes cargo or opi.
#
# Usage: opi-sandbox-smoke.sh --binary PATH --artifact-dir PATH [--archive PATH]
#
# On Linux/macOS this proves the complete extracted-binary direct and backend
# contracts. When --archive is supplied, each independent success marker is
# bound to that archive's SHA-256 for the artifact auditor.
set -euo pipefail

SCRIPT_DIR="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"
BINARY=""
ARTIFACT_DIR=""
ARCHIVE=""
while [ "$#" -gt 0 ]; do
    case "$1" in
        --binary) BINARY="${2:?}"; shift 2 ;;
        --artifact-dir) ARTIFACT_DIR="${2:?}"; shift 2 ;;
        --archive) ARCHIVE="${2:?}"; shift 2 ;;
        *) echo "opi-sandbox-smoke: unknown argument: $1" >&2; exit 2 ;;
    esac
done

if [ -z "$BINARY" ] || [ -z "$ARTIFACT_DIR" ]; then
    echo "usage: opi-sandbox-smoke.sh --binary PATH --artifact-dir PATH [--archive PATH]" >&2
    exit 2
fi
[ -x "$BINARY" ] || { echo "opi-sandbox-smoke: binary not executable: $BINARY" >&2; exit 2; }
mkdir -p "$ARTIFACT_DIR"

hash_stream() {
    if command -v sha256sum >/dev/null 2>&1; then
        sha256sum | awk '{print $1}'
    else
        shasum -a 256 | awk '{print $1}'
    fi
}
ARCHIVE_SHA=""
if [ -n "$ARCHIVE" ]; then
    [ -f "$ARCHIVE" ] || { echo "opi-sandbox-smoke: archive not found: $ARCHIVE" >&2; exit 2; }
    ARCHIVE_SHA="$(hash_stream < "$ARCHIVE")"
fi

# Isolation: scrub opi from PATH; point Opi config/session/package/model env at
# sentinel locations under the artifact dir. The binary must ignore all of them
# (opi-sandbox has no opi dependency and reads no Opi configuration).
SENTINEL="$ARTIFACT_DIR/sentinel"
mkdir -p "$SENTINEL/opi"
CANARY="$SENTINEL/opi/config.toml"
echo "CANARY-opi-config-not-read" > "$CANARY"
export HOME="$SENTINEL"
export XDG_CONFIG_HOME="$SENTINEL"
export OPI_CONFIG_DIR="$SENTINEL"
export OPI_SESSIONS_DIR="$SENTINEL/sessions"
export OPI_PACKAGE_STORE="$SENTINEL/store"
export OPI_MODEL="sentinel-model-not-used"
# Rebuild PATH excluding any directory that contains an opi executable.
NEWPATH=""
OLDIFS="$IFS"
IFS=':'
for d in $PATH; do
    [ -x "$d/opi" ] || NEWPATH="$NEWPATH:$d"
done
IFS="$OLDIFS"
PATH="${NEWPATH#:}"
export PATH

# 1. --help
"$BINARY" --help >"$ARTIFACT_DIR/help.txt" 2>&1
grep -q "run" "$ARTIFACT_DIR/help.txt"
grep -q "doctor" "$ARTIFACT_DIR/help.txt"

# 2. --version
"$BINARY" --version >"$ARTIFACT_DIR/version.txt" 2>&1
grep -q "opi-sandbox" "$ARTIFACT_DIR/version.txt"

# 3. doctor --json (stable object). The supported posture is OS-dependent: on
#    supported Linux (16.13) doctor reports supported=true + landlock/seccomp;
#    on supported macOS (16.14.1) supported=true + seatbelt; off-native
#    (Windows, other Unix) it stays unsupported and `run` refuses pre-start.
TARGET_OS="$(uname -s | tr 'A-Z' 'a-z')"
# std::env::consts::OS (the doctor `target` field) names macOS "macos", not
# "darwin" (uname -s). Map the uname value to the Rust OS family so the
# target-field grep matches on macOS.
RUST_OS="$TARGET_OS"
[ "$RUST_OS" = "darwin" ] && RUST_OS="macos"
"$BINARY" doctor --json >"$ARTIFACT_DIR/doctor.json" 2>&1
grep -q '"schema_version":1' "$ARTIFACT_DIR/doctor.json"
grep -q "\"target\":\"$RUST_OS\"" "$ARTIFACT_DIR/doctor.json"
grep -q '"workspace-write"' "$ARTIFACT_DIR/doctor.json"
if [ "$TARGET_OS" = "linux" ]; then
    grep -q '"supported":true' "$ARTIFACT_DIR/doctor.json"
    grep -q '"landlock"' "$ARTIFACT_DIR/doctor.json"
    grep -q '"seccomp"' "$ARTIFACT_DIR/doctor.json"
    EXPECTED_RUN_CODE=0
elif [ "$TARGET_OS" = "darwin" ]; then
    grep -q '"supported":true' "$ARTIFACT_DIR/doctor.json"
    grep -q '"seatbelt"' "$ARTIFACT_DIR/doctor.json"
    EXPECTED_RUN_CODE=0
else
    grep -q '"supported":false' "$ARTIFACT_DIR/doctor.json"
    grep -q '"mechanisms":\[\]' "$ARTIFACT_DIR/doctor.json"
    EXPECTED_RUN_CODE=125
fi

# 4. Direct CLI: on Linux/macOS an explicit workspace target proves exact argv,
#    inherited stdin, binary stdout/stderr, normal/nonzero/signal exits. The
#    backend smoke below proves the bounded timeout outcome.
WORKSPACE="$ARTIFACT_DIR/ws"
mkdir -p "$WORKSPACE"
DIRECT_TARGET="$WORKSPACE/direct-target.sh"
cat >"$DIRECT_TARGET" <<'EOF'
#!/bin/sh
mode=$1; shift
[ "$#" -eq 2 ] && [ "$1" = 'arg one' ] && [ "$2" = '--literal' ] || exit 96
case "$mode" in
  output)
    IFS= read -r input
    [ "$input" = 'direct stdin' ] || exit 95
    printf '\001\377'
    printf '\002\376' >&2
    ;;
  nonzero) exit 37 ;;
  signal) kill -TERM $$; sleep 5 ;;
  *) exit 97 ;;
esac
EOF
chmod +x "$DIRECT_TARGET"

if [ "$EXPECTED_RUN_CODE" -eq 0 ]; then
    printf 'direct stdin\n' | "$BINARY" run --workspace "$WORKSPACE" \
        --profile workspace-write --network deny -- /bin/sh "$DIRECT_TARGET" \
        output "arg one" --literal >"$ARTIFACT_DIR/run-stdout.bin" \
        2>"$ARTIFACT_DIR/run-stderr.bin"
    printf '\001\377' >"$ARTIFACT_DIR/expected-stdout.bin"
    printf '\002\376' >"$ARTIFACT_DIR/expected-stderr.bin"
    cmp "$ARTIFACT_DIR/expected-stdout.bin" "$ARTIFACT_DIR/run-stdout.bin"
    cmp "$ARTIFACT_DIR/expected-stderr.bin" "$ARTIFACT_DIR/run-stderr.bin"

    set +e
    "$BINARY" run --workspace "$WORKSPACE" --profile workspace-write \
        --network deny -- /bin/sh "$DIRECT_TARGET" nonzero "arg one" --literal
    NONZERO_CODE=$?
    "$BINARY" run --workspace "$WORKSPACE" --profile workspace-write \
        --network deny -- /bin/sh "$DIRECT_TARGET" signal "arg one" --literal
    SIGNAL_CODE=$?
    set -e
    [ "$NONZERO_CODE" -eq 37 ] || {
        echo "opi-sandbox-smoke: expected nonzero exit 37, got $NONZERO_CODE" >&2; exit 1; }
    [ "$SIGNAL_CODE" -eq 143 ] || {
        echo "opi-sandbox-smoke: expected signal exit 143, got $SIGNAL_CODE" >&2; exit 1; }
    echo "0" >"$ARTIFACT_DIR/run-exit.txt"

    DIRECT_MARKER="opi-sandbox-direct-smoke: OK"
    [ -z "$ARCHIVE_SHA" ] || DIRECT_MARKER="$DIRECT_MARKER archive_sha256=$ARCHIVE_SHA"
    echo "$DIRECT_MARKER" >"$ARTIFACT_DIR/direct-smoke-result.txt"

    PROTOCOL_CLIENT="$SCRIPT_DIR/../crates/opi-sandbox/tests/fixtures/protocol_client.py"
    [ -f "$PROTOCOL_CLIENT" ] || {
        echo "opi-sandbox-smoke: protocol client not found: $PROTOCOL_CLIENT" >&2; exit 2; }
    if [ -n "$ARCHIVE_SHA" ]; then
        PACKAGE_MANIFEST="$(dirname "$(dirname "$BINARY")")/package.toml"
        EXPECTED_TARGET="$(sed -n 's/^[[:space:]]*target[[:space:]]*=[[:space:]]*"\([^"]*\)".*/\1/p' "$PACKAGE_MANIFEST")"
        [ -n "$EXPECTED_TARGET" ] || {
            echo "opi-sandbox-smoke: package target missing: $PACKAGE_MANIFEST" >&2; exit 2; }
        python3 "$PROTOCOL_CLIENT" "$BINARY" "$ARCHIVE_SHA" "$EXPECTED_TARGET" \
            >"$ARTIFACT_DIR/backend-smoke-result.txt"
    else
        python3 "$PROTOCOL_CLIENT" "$BINARY" \
            >"$ARTIFACT_DIR/backend-smoke-result.txt"
    fi
    grep -q '^opi-sandbox-backend-smoke: OK' "$ARTIFACT_DIR/backend-smoke-result.txt"
else
    set +e
    "$BINARY" run --workspace "$WORKSPACE" --profile workspace-write --network deny \
        -- /bin/sh -c "exit 0" >"$ARTIFACT_DIR/run-stdout.bin" \
        2>"$ARTIFACT_DIR/run-stderr.bin"
    RUN_CODE=$?
    set -e
    echo "$RUN_CODE" >"$ARTIFACT_DIR/run-exit.txt"
    [ "$RUN_CODE" -eq 125 ] || {
        echo "opi-sandbox-smoke: expected unsupported exit 125, got $RUN_CODE" >&2; exit 1; }
fi

# 5. no durable state / no Opi access: the sentinel canary was never read and no
#    file was created under the sentinel tree beyond the canary we planted.
if grep -q "CANARY-opi-config-not-read" "$ARTIFACT_DIR/doctor.json" 2>/dev/null; then
    echo "opi-sandbox-smoke: binary leaked sentinel config into doctor output" >&2
    exit 1
fi
SENTINEL_FILES="$(find "$SENTINEL" -type f | sort)"
if [ "$SENTINEL_FILES" != "$CANARY" ]; then
    echo "opi-sandbox-smoke: binary created files under sentinel: $SENTINEL_FILES" >&2
    exit 1
fi

SMOKE_MARKER="opi-sandbox-smoke: OK"
[ -z "$ARCHIVE_SHA" ] || SMOKE_MARKER="$SMOKE_MARKER archive_sha256=$ARCHIVE_SHA"
echo "$SMOKE_MARKER" >"$ARTIFACT_DIR/smoke-result.txt"
exit 0
