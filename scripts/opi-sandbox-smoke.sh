#!/usr/bin/env bash
#
# Standalone acceptance smoke for the opi-sandbox binary (Phase 16 task 16.11.2).
# Launches ONLY the explicit --binary path; never invokes cargo or opi.
#
# Usage: opi-sandbox-smoke.sh --binary PATH --artifact-dir PATH
#
# Covers spec `### Standalone CLI acceptance` items 1-5, 8 (binary identity,
# no-opi-on-PATH, Opi-sentinel env ignored, help/version/doctor, run pre-start
# refusal, no durable state). Item 6 (installed-binary run success) and item 7
# (backend --stdio) are deferred to 16.13/16.14.1 and 16.12 respectively.
set -euo pipefail

BINARY=""
ARTIFACT_DIR=""
while [ "$#" -gt 0 ]; do
    case "$1" in
        --binary) BINARY="${2:?}"; shift 2 ;;
        --artifact-dir) ARTIFACT_DIR="${2:?}"; shift 2 ;;
        *) echo "opi-sandbox-smoke: unknown argument: $1" >&2; exit 2 ;;
    esac
done

if [ -z "$BINARY" ] || [ -z "$ARTIFACT_DIR" ]; then
    echo "usage: opi-sandbox-smoke.sh --binary PATH --artifact-dir PATH" >&2
    exit 2
fi
[ -x "$BINARY" ] || { echo "opi-sandbox-smoke: binary not executable: $BINARY" >&2; exit 2; }
mkdir -p "$ARTIFACT_DIR"

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

# 3. doctor --json (stable object; supported=false everywhere in 16.11.2)
"$BINARY" doctor --json >"$ARTIFACT_DIR/doctor.json" 2>&1
grep -q '"schema_version":1' "$ARTIFACT_DIR/doctor.json"
grep -q '"supported":false' "$ARTIFACT_DIR/doctor.json"
TARGET_OS="$(uname -s | tr 'A-Z' 'a-z')"
grep -q "\"target\":\"$TARGET_OS\"" "$ARTIFACT_DIR/doctor.json"
grep -q '"mechanisms":\[\]' "$ARTIFACT_DIR/doctor.json"
grep -q '"workspace-write"' "$ARTIFACT_DIR/doctor.json"

# 4. run with a VALID argv -> pre-start platform refusal (125) in 16.11.2.
#    16.13 (Linux) / 16.14.1 (macOS) flip this to a successful native run; that
#    flip is a visible change here, not a silent pass.
WORKSPACE="$ARTIFACT_DIR/ws"
mkdir -p "$WORKSPACE"
set +e
"$BINARY" run --workspace "$WORKSPACE" --profile workspace-write --network deny \
    -- /bin/sh -c "exit 0" >"$ARTIFACT_DIR/run-stdout.txt" 2>"$ARTIFACT_DIR/run-stderr.txt"
RUN_CODE=$?
set -e
echo "$RUN_CODE" >"$ARTIFACT_DIR/run-exit.txt"
[ "$RUN_CODE" -eq 125 ] || {
    echo "opi-sandbox-smoke: expected run exit 125 (pre-start refusal), got $RUN_CODE" >&2
    exit 1
}

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

echo "opi-sandbox-smoke: OK" >"$ARTIFACT_DIR/smoke-result.txt"
exit 0
