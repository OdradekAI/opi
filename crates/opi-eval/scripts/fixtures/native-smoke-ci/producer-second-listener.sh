#!/usr/bin/env bash
# Negative fixture: a producer that starts a second listener endpoint
# beside the single pre-resolved admitted endpoint. The contract admits
# exactly one listener; this must be rejected (family `endpoint`).

set -euo pipefail

start_provider() {
  endpoint=$1
  env -i PATH="/usr/bin:/bin" HOME="$(mktemp -d)" \
    "$(readlink -f "$(command -v python3)")" -I -S provider.py \
    --listen "$endpoint" &
}

# First (admitted) endpoint.
start_provider 127.0.0.1:48127
# Second listener: forbidden.
start_provider 127.0.0.1:48227
