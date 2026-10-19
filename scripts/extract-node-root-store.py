#!/usr/bin/env python3
"""Extract the pinned Node reference default root store for opi-ai TLS.

RDF-23 pins opi-ai's default TLS trust to the packaged Node reference CA
bundle (the runtime's compiled-in default set, `tls.rootCertificates`), not
to webpki-roots, the OS store or any ambient discovery. This script makes
that pin reproducible: it reads the bundle from an explicit Node binary and
prints the canonical PEM bundle to stdout (or `--out`) with provenance JSON
on stderr.

Usage:
    python scripts/extract-node-root-store.py --node <path-to-node> \
        [--out crates/opi-ai/src/tls/node-root-store.pem]

The pinned reference is Node v22.19.0 (the pi 1.3.0 engines floor). Any
Node binary may be passed for comparison; the generated Rust module records
the version and bundle digest it was produced from.
"""

from __future__ import annotations

import argparse
import base64
import hashlib
import json
import re
import subprocess
import sys

EXTRACT = (
    "process.stdout.write(require('tls').rootCertificates.join('\\n') + '\\n')"
)
PEM_BLOCK = re.compile(
    r"^-----BEGIN CERTIFICATE-----\n"
    r"([A-Za-z0-9+/=\n]+?)"
    r"-----END CERTIFICATE-----$",
    re.MULTILINE,
)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--node", required=True, help="explicit node binary path")
    parser.add_argument("--out", help="write the PEM bundle here (default stdout)")
    args = parser.parse_args()

    version = subprocess.run(
        [args.node, "--version"], capture_output=True, text=True, check=True
    ).stdout.strip()
    result = subprocess.run(
        [args.node, "-e", EXTRACT], capture_output=True, text=True, check=True
    )
    bundle = result.stdout
    if not bundle.startswith("-----BEGIN CERTIFICATE-----\n"):
        raise SystemExit("unexpected bundle shape from node")
    certs = []
    for match in PEM_BLOCK.finditer(bundle):
        body = re.sub(r"\s+", "", match.group(1))
        certs.append(base64.b64decode(body, validate=True))
    unique = {hashlib.sha256(c).hexdigest() for c in certs}
    provenance = {
        "node_version": version,
        "certificate_count": len(certs),
        "unique_der_count": len(unique),
        "pem_bundle_sha256": hashlib.sha256(bundle.encode("ascii")).hexdigest(),
        "source": "tls.rootCertificates (compiled-in Node default CA set)",
    }
    if args.out:
        with open(args.out, "w", encoding="ascii", newline="\n") as handle:
            handle.write(bundle)
    else:
        sys.stdout.write(bundle)
    print(json.dumps(provenance, indent=2), file=sys.stderr)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
