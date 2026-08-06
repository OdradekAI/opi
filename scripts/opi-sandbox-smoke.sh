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
mkdir -p "$ARTIFACT_DIR"
ARTIFACT_DIR="$(CDPATH= cd -- "$ARTIFACT_DIR" && pwd -P)"
BINARY_DIR="$(CDPATH= cd -- "$(dirname -- "$BINARY")" 2>/dev/null && pwd -P)" || {
    echo "opi-sandbox-smoke: binary not found: $BINARY" >&2; exit 2; }
BINARY="$BINARY_DIR/$(basename -- "$BINARY")"
[ -x "$BINARY" ] || { echo "opi-sandbox-smoke: binary not executable: $BINARY" >&2; exit 2; }
if [ -n "$ARCHIVE" ]; then
    ARCHIVE_DIR="$(CDPATH= cd -- "$(dirname -- "$ARCHIVE")" 2>/dev/null && pwd -P)" || {
        echo "opi-sandbox-smoke: archive not found: $ARCHIVE" >&2; exit 2; }
    ARCHIVE="$ARCHIVE_DIR/$(basename -- "$ARCHIVE")"
fi

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

write_result() {
    result_file=$1
    result_label=$2
    result_marker="opi-sandbox-$result_label-smoke: OK"
    [ -z "$ARCHIVE_SHA" ] || result_marker="$result_marker archive_sha256=$ARCHIVE_SHA"
    printf '%s\n' "$result_marker" >"$ARTIFACT_DIR/$result_file"
}

# Every binary launch below begins from a genuinely empty directory, with the
# explicit extracted binary canonicalized before this chdir. No workspace
# target/ build artifact can be discovered from the process working directory.
EMPTY_CWD="$ARTIFACT_DIR/empty-cwd"
rm -rf "$EMPTY_CWD"
mkdir -p "$EMPTY_CWD"
[ -z "$(find "$EMPTY_CWD" -mindepth 1 -print -quit)" ] || {
    echo "opi-sandbox-smoke: empty working directory is not empty" >&2; exit 1; }
cd "$EMPTY_CWD"

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
    # A direct setup failure must map to 125 before a real, marker-capable
    # target can start. Point TMPDIR at a regular file so invocation temp-root
    # creation fails before the release-gate process is spawned.
    SETUP_TMPDIR_FILE="$ARTIFACT_DIR/setup-temp-root-blocker"
    SETUP_NO_START="$WORKSPACE/setup-target-started.txt"
    rm -rf "$SETUP_TMPDIR_FILE"
    printf 'not a directory\n' >"$SETUP_TMPDIR_FILE"
    rm -f "$SETUP_NO_START"
    set +e
    TMPDIR="$SETUP_TMPDIR_FILE" \
        "$BINARY" run --workspace "$WORKSPACE" --profile workspace-write \
        --network deny -- /usr/bin/touch "$SETUP_NO_START" \
        >"$ARTIFACT_DIR/setup-stdout.txt" \
        2>"$ARTIFACT_DIR/setup-stderr.txt"
    SETUP_CODE=$?
    set -e
    [ "$SETUP_CODE" -eq 125 ] || {
        echo "opi-sandbox-smoke: expected setup failure 125, got $SETUP_CODE" >&2; exit 1; }
    [ ! -e "$SETUP_NO_START" ] || {
        echo "opi-sandbox-smoke: setup-failed target crossed the start barrier" >&2; exit 1; }
    write_result "setup-failure-smoke-result.txt" "setup-failure"

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

    # Native workspace-write filesystem contract: an in-workspace write is
    # permitted, while a sibling outside workspace + invocation temp is denied.
    INSIDE_WRITE="$WORKSPACE/filesystem-allowed.txt"
    "$BINARY" run --workspace "$WORKSPACE" --profile workspace-write \
        --network deny -- /bin/sh -c 'printf allowed > "$1"' sh "$INSIDE_WRITE"
    [ "$(cat "$INSIDE_WRITE")" = "allowed" ] || {
        echo "opi-sandbox-smoke: workspace write sentinel failed" >&2; exit 1; }
    write_result "filesystem-allow-smoke-result.txt" "filesystem-allow"

    OUTSIDE_WRITE="$ARTIFACT_DIR/filesystem-denied-must-not-exist.txt"
    rm -f "$OUTSIDE_WRITE"
    set +e
    "$BINARY" run --workspace "$WORKSPACE" --profile workspace-write \
        --network deny -- /bin/sh -c 'printf denied > "$1"' sh "$OUTSIDE_WRITE" \
        >"$ARTIFACT_DIR/filesystem-deny-stdout.txt" \
        2>"$ARTIFACT_DIR/filesystem-deny-stderr.txt"
    FILESYSTEM_DENY_CODE=$?
    set -e
    [ "$FILESYSTEM_DENY_CODE" -ne 0 ] && [ ! -e "$OUTSIDE_WRITE" ] || {
        echo "opi-sandbox-smoke: outside-workspace write was not denied" >&2; exit 1; }
    write_result "filesystem-deny-smoke-result.txt" "filesystem-deny"

    # Deterministic local networking sentinels. INET bind needs no external
    # internet: Linux deny blocks socket creation first; macOS deny blocks bind.
    PYTHON_TARGET="$(command -v python3)"
    [ -x "$PYTHON_TARGET" ] || {
        echo "opi-sandbox-smoke: python3 is required for network sentinels" >&2; exit 2; }
    BIND_SCRIPT='import socket, sys
try:
    s = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    s.bind(("127.0.0.1", 0))
except OSError:
    sys.stdout.write("BIND_DENIED\n")
    sys.exit(23)
sys.stdout.write("BIND_OK\n")'
    set +e
    "$BINARY" run --workspace "$WORKSPACE" --profile workspace-write \
        --network deny -- "$PYTHON_TARGET" -c "$BIND_SCRIPT" \
        >"$ARTIFACT_DIR/network-deny-stdout.txt" \
        2>"$ARTIFACT_DIR/network-deny-stderr.txt"
    NETWORK_DENY_CODE=$?
    set -e
    [ "$NETWORK_DENY_CODE" -eq 23 ] || {
        echo "opi-sandbox-smoke: network deny expected exit 23, got $NETWORK_DENY_CODE" >&2; exit 1; }
    grep -q '^BIND_DENIED$' "$ARTIFACT_DIR/network-deny-stdout.txt"
    ! grep -q 'BIND_OK' "$ARTIFACT_DIR/network-deny-stdout.txt" || {
        echo "opi-sandbox-smoke: network deny emitted success sentinel" >&2; exit 1; }
    ! grep -q 'Traceback' "$ARTIFACT_DIR/network-deny-stderr.txt" || {
        echo "opi-sandbox-smoke: network deny leaked a Python traceback" >&2; exit 1; }
    write_result "network-deny-smoke-result.txt" "network-deny"

    "$BINARY" run --workspace "$WORKSPACE" --profile workspace-write \
        --network allow -- "$PYTHON_TARGET" -c "$BIND_SCRIPT" \
        >"$ARTIFACT_DIR/network-allow-stdout.txt" \
        2>"$ARTIFACT_DIR/network-allow-stderr.txt"
    grep -q '^BIND_OK$' "$ARTIFACT_DIR/network-allow-stdout.txt"
    write_result "network-allow-smoke-result.txt" "network-allow"

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
if [ -n "$(find "$EMPTY_CWD" -mindepth 1 -print -quit)" ]; then
    echo "opi-sandbox-smoke: extracted binary wrote into the empty working directory" >&2
    exit 1
fi
write_result "empty-cwd-smoke-result.txt" "empty-cwd"

SMOKE_MARKER="opi-sandbox-smoke: OK"
[ -z "$ARCHIVE_SHA" ] || SMOKE_MARKER="$SMOKE_MARKER archive_sha256=$ARCHIVE_SHA"
echo "$SMOKE_MARKER" >"$ARTIFACT_DIR/smoke-result.txt"
exit 0
