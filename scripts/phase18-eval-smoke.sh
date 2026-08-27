#!/bin/sh
# Phase 18 assembled-run smoke wrapper (task 18.12), POSIX shells.
#
# Runs the production `opi-eval run` path twice (a completed run and a
# native-verifier failure) and preserves the exact command, stdout, stderr,
# exit code, receipt bytes, and bundle identities under an output root.
# Exits 0 only when both observed outcomes match the pinned expectations.
#
# Usage: scripts/phase18-eval-smoke.sh [--fixtures DIR] [--out DIR]

set -u

REPO_ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
FIXTURES="${REPO_ROOT}/crates/opi-eval/tests/fixtures"
OUT=$(mktemp -d)

while [ $# -gt 0 ]; do
  case "$1" in
    --fixtures) FIXTURES=$2; shift 2 ;;
    --out) OUT=$2; shift 2 ;;
    *) echo "unknown argument: $1" >&2; exit 2 ;;
  esac
done

CONFIG="${FIXTURES}/experiment/phase18-local.toml"

run_case() {
  behavior=$1
  expected=$2
  case_dir="${OUT}/${behavior}"
  mkdir -p "${case_dir}"
  command -v cargo >/dev/null 2>&1 || { echo "cargo not found" >&2; exit 2; }
  {
    printf 'cargo run -q -p opi-eval -- run --config %s --root %s/root --fixtures %s --behavior %s\n' \
      "$CONFIG" "$case_dir" "$FIXTURES" "$behavior"
  } > "${case_dir}/command.txt"
  cargo run -q -p opi-eval -- run \
    --config "$CONFIG" \
    --root "${case_dir}/root" \
    --fixtures "$FIXTURES" \
    --behavior "$behavior" \
    > "${case_dir}/stdout.json" 2> "${case_dir}/stderr.txt"
  code=$?
  printf '%s\n' "$code" > "${case_dir}/exit_code"
  if [ "$code" -ne "$expected" ]; then
    echo "behavior ${behavior}: expected exit ${expected}, observed ${code}" >&2
    cat "${case_dir}/stderr.txt" >&2
    exit 1
  fi
}

run_case happy 0
run_case verifier-failure 1

# Artifact audit: preserved receipts, sealed bundles, and content-addressed
# bundle identities for the completed run.
{
  printf '{\n'
  printf '  "schema": "phase18-eval-smoke-audit/1",\n'
  printf '  "happy": {\n'
  printf '    "bundle_identities": [\n'
  first=1
  for receipt in "${OUT}"/happy/root/trials/*/receipt.json; do
    trial_dir=$(dirname "$receipt")
    bundle_dir="${trial_dir}/bundle"
    identity=$(sed -n 's/.*"bundle_identity":"\([0-9a-f]*\)".*/\1/p' "$receipt")
    receipt_sha=$(sha256sum "$receipt" | cut -d' ' -f1)
    if [ "$first" -eq 1 ]; then first=0; else printf ',\n'; fi
    printf '      {"trial": "%s", "bundle_identity": "%s", "receipt_sha256": "%s", "sealed": %s}' \
      "$(basename "$trial_dir")" "$identity" "$receipt_sha" \
      "$([ -f "${bundle_dir}/manifest.json" ] && printf true || printf false)"
  done
  printf '\n    ]\n'
  printf '  },\n'
  printf '  "verifier_failure": {\n'
  printf '    "receipt_written": %s\n' \
    "$([ -f "${OUT}/verifier-failure/root/trials/trial-opi-1/receipt.json" ] && printf true || printf false)"
  printf '  }\n'
  printf '}\n'
} > "${OUT}/audit.json"

printf '%s\n' "$OUT"
exit 0
