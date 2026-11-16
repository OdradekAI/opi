"""Consume the packaged WebSocket/compression owner from an external caller."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys
import time

import package_slice
import run_early_package_proof as shared


HARNESS = Path(__file__).resolve().parent
REPO = HARNESS.parents[1]
FORBIDDEN = {
    "opi-catalog",
    "hmac",
    "aws-smithy-query",
    "aws-smithy-xml",
    "xmlparser",
    "p256",
    "sha1",
}
REQUIRED = {
    "reqwest",
    "rustls",
    "rustls-native-certs",
    "aws-lc-rs",
    "libz-rs-sys",
    "flate2",
}


def check_graph(metadata: dict, package: Path) -> dict:
    packages = metadata["packages"]
    sdk = [entry for entry in packages if entry["name"] == "opi-ai"]
    if len(sdk) != 1 or Path(sdk[0]["manifest_path"]).resolve() != package / "Cargo.toml":
        raise ValueError("consumer must resolve the extracted opi-ai package")
    nodes = {node["id"]: node for node in metadata["resolve"]["nodes"]}
    sdk_features = set(nodes[sdk[0]["id"]]["features"])
    expected_features = {"http-transport", "tls-backend", "websocket-compression"}
    if sdk_features != expected_features:
        raise ValueError(f"unexpected WebSocket feature union: {sorted(sdk_features)}")
    names = {entry["name"] for entry in packages}
    leaked = FORBIDDEN.intersection(names)
    if leaked:
        raise ValueError(f"WebSocket consumer contains forbidden provider owner: {sorted(leaked)}")
    present = REQUIRED.intersection(names)
    if present != REQUIRED:
        raise ValueError(f"WebSocket consumer lost required owner: {sorted(REQUIRED - present)}")
    local = {entry["id"] for entry in packages if entry.get("source") is None}
    allowed = {sdk[0]["id"], metadata["resolve"]["root"]}
    telemetry = [entry for entry in packages if entry["name"] == "opi-telemetry"]
    if len(telemetry) != 1:
        raise ValueError("WebSocket consumer must resolve one telemetry dependency")
    allowed.add(telemetry[0]["id"])
    if local != allowed:
        raise ValueError(f"unexpected local dependency closure: {sorted(local - allowed)}")
    return {
        "features": sorted(sdk_features),
        "forbidden_absent": sorted(FORBIDDEN),
        "compression_present": sorted(present),
        "packages": sorted(entry["name"] + "@" + entry["version"] for entry in packages),
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--scratch", required=True)
    parser.add_argument("--target-dir", required=True)
    parser.add_argument("--evidence-output", required=True)
    args = parser.parse_args()
    scratch = Path(args.scratch).expanduser().resolve()
    target = Path(args.target_dir).expanduser().resolve()
    output = Path(args.evidence_output).expanduser().resolve()
    if output.exists():
        parser.error("evidence already exists; select a new path")
    try:
        shared.validate_paths(REPO, scratch, target)
        configs = package_slice.check_cargo_configs(
            package_slice.controlled_environment(dict(os.environ), target),
            [REPO, scratch / "consumer", scratch / "package"],
        )
    except (OSError, ValueError) as exc:
        parser.error(str(exc))
    scratch.mkdir(parents=True)
    env = package_slice.controlled_environment(dict(os.environ), target)
    env.setdefault("CARGO_BUILD_JOBS", "4")
    env["CARGO_TARGET_DIR"] = str(target)
    evidence = {
        "family": "websocket-compression-package",
        "scope": "external consumer of the websocket-compression feature from the actual opi-ai archive",
        "target": str(target),
        "cargo_configs": configs,
        "runner_sha256": hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
        "steps": [],
        "verdict": "FAIL",
    }
    candidate = None

    def run(name: str, command: list[str], cwd: Path) -> str:
        nonlocal candidate
        if candidate is not None:
            command = candidate.command(command)
        started = time.monotonic()
        result = subprocess.run(
            command,
            cwd=cwd,
            env=env,
            capture_output=True,
            text=True,
            encoding="utf-8",
            errors="replace",
            timeout=3600,
        )
        paths = {}
        for stream in ("stdout", "stderr"):
            path = scratch / f"{name}.{stream}"
            path.write_text(getattr(result, stream), encoding="utf-8", newline="\n")
            paths[stream] = {
                "path": str(path),
                "sha256": hashlib.sha256(path.read_bytes()).hexdigest(),
            }
        evidence["steps"].append(
            {
                "name": name,
                "command": command,
                "exit": result.returncode,
                "elapsed_seconds": round(time.monotonic() - started, 2),
                **paths,
            }
        )
        if result.returncode:
            raise ValueError(f"{name} failed with exit {result.returncode}")
        if candidate is not None and command[0] == "cargo" and "metadata" in command:
            candidate.verify_graph(json.loads(result.stdout))
        return result.stdout

    lease = [sys.executable, str(REPO / "scripts/opi-cargo-cache.py"), "lease"]
    leased = False
    try:
        run("rustc-version", ["rustc", "-vV"], REPO)
        run("cargo-version", ["cargo", "-vV"], REPO)
        run("lease-start", lease + ["start", "--target", str(target), "--pid", str(os.getpid())], REPO)
        leased = True
        package, evidence["package"], candidate = shared.package_sdk(run, scratch, target)
        consumer = scratch / "consumer"
        (consumer / "src").mkdir(parents=True)
        manifest = (
            '[package]\nname = "opi-websocket-compression-consumer"\nversion = "0.0.0"\n'
            'edition = "2024"\n[workspace]\n[dependencies]\n'
            f'opi-ai = {{ path = {json.dumps(package.as_posix())}, default-features = false, '
            'features = ["websocket-compression"] }\n'
            'tokio = { version = "=1.52.3", features = ["macros", "rt-multi-thread", "io-util", "net"] }\n'
            'base64 = "=0.22.1"\nurl = "=2.5.8"\n'
        )
        (consumer / "Cargo.toml").write_text(manifest, encoding="utf-8", newline="\n")
        (consumer / "src/main.rs").write_text(
            """use base64::Engine as _;
use opi_ai::http::HttpClientBuilder;
use opi_ai::ws::{self, PmdOffer, WsUpgradeRequest};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use url::Url;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let listener = TcpListener::bind(("127.0.0.1", 0)).await?;
    let address = listener.local_addr()?;
    let peer = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await?;
        let mut head = Vec::new();
        while !head.ends_with(b"\\r\\n\\r\\n") {
            head.push(stream.read_u8().await?);
        }
        let text = String::from_utf8(head)?;
        let key = text
            .lines()
            .find_map(|line| {
                let (name, value) = line.split_once(':')?;
                name.eq_ignore_ascii_case("sec-websocket-key").then(|| value.trim())
            })
            .ok_or("missing WebSocket key")?;
        let raw = base64::engine::general_purpose::STANDARD.decode(key)?;
        let nonce: [u8; 16] = raw.try_into().map_err(|_| "invalid nonce")?;
        let response = format!(
            "HTTP/1.1 101 Switching Protocols\\r\\nConnection: Upgrade\\r\\nUpgrade: websocket\\r\\nSec-WebSocket-Accept: {}\\r\\nSec-WebSocket-Extensions: permessage-deflate; server_no_context_takeover; client_no_context_takeover; server_max_window_bits=8; client_max_window_bits=8\\r\\n\\r\\n",
            ws::accept_token(&nonce),
        );
        stream.write_all(response.as_bytes()).await?;
        Ok::<_, Box<dyn std::error::Error + Send + Sync>>(())
    });

    let client = HttpClientBuilder::new().build()?;
    let request = WsUpgradeRequest::new(Url::parse(&format!("ws://{address}/socket"))?)
        .permessage_deflate(PmdOffer {
            client_no_context_takeover: true,
            client_max_window_bits: Some(Some(10)),
        });
    let upgraded = ws::upgrade(&client, request).await?;
    let pmd = upgraded.negotiation.pmd.ok_or("PMD was not negotiated")?;
    assert!(pmd.server_no_context_takeover);
    assert!(pmd.client_no_context_takeover);
    assert_eq!(pmd.server_max_window_bits, 8);
    assert_eq!(pmd.client_max_window_bits, Some(8));
    drop(upgraded);
    peer.await??;
    println!("websocket-compression=ok");
    Ok(())
}
""",
            encoding="utf-8",
            newline="\n",
        )
        run("consumer-lock", ["cargo", "generate-lockfile", "--offline"], consumer)
        metadata = json.loads(run(
            "websocket-compression-metadata",
            ["cargo", "metadata", "--format-version", "1", "--offline", "--locked"],
            consumer,
        ))
        evidence["graph"] = check_graph(metadata, package)
        run("websocket-compression-build", ["cargo", "build", "--offline", "--locked"], consumer)
        output_text = run("websocket-compression-run", ["cargo", "run", "--offline", "--locked"], consumer)
        if "websocket-compression=ok" not in output_text:
            raise ValueError("external consumer did not observe the WebSocket/PMD contract")
        evidence["consumer_sources"] = {
            "Cargo.toml": hashlib.sha256((consumer / "Cargo.toml").read_bytes()).hexdigest(),
            "src/main.rs": hashlib.sha256((consumer / "src/main.rs").read_bytes()).hexdigest(),
            "Cargo.lock": hashlib.sha256((consumer / "Cargo.lock").read_bytes()).hexdigest(),
        }
        evidence["verdict"] = "PASS"
    except (OSError, ValueError, subprocess.SubprocessError) as exc:
        evidence["error"] = str(exc)
    finally:
        if leased:
            try:
                run("lease-end", lease + ["end", "--target", str(target), "--pid", str(os.getpid())], REPO)
            except (OSError, ValueError, subprocess.SubprocessError) as exc:
                evidence["verdict"] = "FAIL"
                evidence["lease_error"] = str(exc)
        encoded = json.dumps(evidence, indent=2) + "\n"
        (scratch / "evidence.json").write_text(encoded, encoding="utf-8", newline="\n")
        output.parent.mkdir(parents=True, exist_ok=True)
        with output.open("x", encoding="utf-8", newline="\n") as handle:
            handle.write(encoded)
    return 0 if evidence["verdict"] == "PASS" else 1


if __name__ == "__main__":
    raise SystemExit(main())
