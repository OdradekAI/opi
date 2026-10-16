"""Prepare exact cached TW/TTW archives and owned harness in a NEW temp folder.

No network, installer, Cargo cache deletion or product-file mutation. Other
registry packages must already be present at their Cargo.lock checksums.
"""
import argparse
import hashlib
import json
from pathlib import Path
import shutil
import tarfile

parser = argparse.ArgumentParser()
parser.add_argument('--archive-dir', type=Path, required=True)
parser.add_argument('--output', type=Path, required=True)
args = parser.parse_args()
here = Path(__file__).resolve().parent
target = args.output.resolve()
target.mkdir(parents=True, exist_ok=False)
assets = ['ws-client.mjs', 'run-ws-probe.mjs', 'runtime-info.mjs', 'provider-import.mjs', 'rust/Cargo.toml', 'rust/Cargo.lock', 'rust/src/main.rs']
for rel in assets:
    out = target / rel
    out.parent.mkdir(parents=True, exist_ok=True)
    shutil.copyfile(here / rel, out)
pins = {
    'tungstenite-0.28.0': '8628dcc84e5a09eb3d8423d6cb682965dea9133204e8fb3efee74c2a0c259442',
    'tokio-tungstenite-0.28.0': 'd25a406cddcc431a75d3d9afc6a7c0f7428d4891dd973e4d54c56b46127bf857',
}
source = target / 'source'
source.mkdir()
for name, expected in pins.items():
    archive = args.archive_dir / f'{name}.crate'
    actual = hashlib.sha256(archive.read_bytes()).hexdigest()
    if actual != expected:
        raise SystemExit(f'archive SHA256 mismatch: {archive}')
    with tarfile.open(archive, 'r:gz') as tf:
        for m in tf.getmembers():
            if not (source / m.name).resolve().is_relative_to(source) or m.issym() or m.islnk():
                raise SystemExit(f'unsafe archive member: {m.name}')
        tf.extractall(source, filter='data')
        for m in tf.getmembers():
            if m.isfile() and (source / m.name).read_bytes() != tf.extractfile(m).read():
                raise SystemExit(f'extracted member mismatch: {m.name}')
print(json.dumps({'prepared': str(target), 'network': False, 'source_pins': pins}))
