#!/usr/bin/env python3
"""Phase 18 deterministic local provider fixture (task 18.10.1).

A scripted model endpoint for fixture-level conformance: it reads one JSON
request line per turn from stdin and writes one fixed JSON response line to
stdout, then exits at EOF or after the bounded request cap. It makes no
network connection, reads no credentials, and loads no user or project
resources (P18-AGT-006). The wire shape is a conformance-fixture schema
(``phase18-scripted-provider/1``), not any real provider protocol: task
18.15 re-pins the provider surface against the exact built Opi program's
provider configuration.

Request lines must be objects with exactly a ``prompt`` string field; any
other shape terminates with exit code 2 and a one-line diagnostic on stderr.
Responses are deterministic single objects with exactly a ``schema`` and a
``content`` field.
"""

from __future__ import annotations

import json
import sys

SCHEMA = "phase18-scripted-provider/1"
RESPONSE_CONTENT = "scripted-provider: acknowledged"
MAX_REQUESTS = 8


def main() -> int:
    served = 0
    for line in sys.stdin:
        line = line.strip()
        if not line:
            continue
        try:
            request = json.loads(line)
        except json.JSONDecodeError:
            print("scripted-provider: request line is not JSON", file=sys.stderr)
            return 2
        if (
            not isinstance(request, dict)
            or set(request) != {"prompt"}
            or not isinstance(request["prompt"], str)
        ):
            print(
                "scripted-provider: request must be {\"prompt\": <string>}",
                file=sys.stderr,
            )
            return 2
        served += 1
        if served > MAX_REQUESTS:
            print(
                f"scripted-provider: request cap {MAX_REQUESTS} exceeded",
                file=sys.stderr,
            )
            return 3
        sys.stdout.write(
            json.dumps(
                {"schema": SCHEMA, "content": RESPONSE_CONTENT},
                separators=(",", ":"),
                sort_keys=True,
            )
            + "\n"
        )
        sys.stdout.flush()
    return 0


if __name__ == "__main__":
    sys.exit(main())
