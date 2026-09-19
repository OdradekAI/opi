#!/usr/bin/env python3
"""Offline production acquisition tests with pinned local sources and fake builders."""
import base64
import hashlib
import io
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tarfile
import tempfile
import unittest

SCRIPTS = Path(__file__).resolve().parent
FAMILIES = ("terminal-bench-2.1", "terminal-bench-3.0", "deepswe-v1.1")


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def write(path, value, executable=False):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(value, newline="\n")
    if executable:
        path.chmod(0o755)


def git(root, *args):
    return subprocess.check_output(["/usr/bin/git", "-C", str(root), *args], text=True).strip()


def commit(root):
    git(root, "init", "--quiet")
    git(root, "add", "--", ".")
    git(root, "-c", "user.name=Acquisition Fixture", "-c", "user.email=fixture@example.invalid", "commit", "--quiet", "-m", "test: pinned acquisition fixture")
    return git(root, "rev-parse", "HEAD")


def blob(root, relative):
    return {"path": relative, "git_blob": git(root, "rev-parse", f"HEAD:{relative}"), "sha256": sha(root / relative)}


def archive(path, entries):
    with tarfile.open(path, "w:gz") as output:
        for name, data, mode in entries:
            content = data.encode() if isinstance(data, str) else data
            item = tarfile.TarInfo(name)
            item.size = len(content)
            item.mode = mode
            output.addfile(item, io.BytesIO(content))
    return {"url": path.as_uri(), "sha256": sha(path)}


class Fixture:
    def __init__(self, root):
        self.root = Path(root).resolve()
        self.root.mkdir(parents=True)
        self.cache = self.root / "cache"
        self.bin = self.root / "bin"
        self.events = self.root / "build-events.jsonl"
        self.opi = self.root / "sources/opi"
        write(self.opi / "Cargo.lock", "version = 4\n")
        write(self.opi / "Cargo.toml", "[workspace]\n")
        self.opi_commit = commit(self.opi)
        write(self.opi / "uncommitted-sentinel", "must not enter the build")
        subjects = []
        for family in FAMILIES:
            source = self.root / "sources" / family
            package = SCRIPTS.parent / "tests/fixtures/benchmarks" / family / "task-package"
            for task in ("openssl-selfsigned-cert", "unselected-task"):
                shutil.copytree(package, source / "tasks" / task)
                task_file = source / "tasks" / task / "task.toml"
                # Every task remains a valid bounded no-resource fixture.
                write(task_file, 'artifacts = ["/root/answer.txt"]\n[task]\nid = "' + task + '"\n[environment]\nstorage_mb=0\ngpus=0\n[verifier]\ntimeout_sec=1\n')
                for dockerfile in (source / "tasks" / task).rglob("Dockerfile"):
                    write(dockerfile, "FROM scratch\n")
            revision = commit(source)
            subjects.append({"id": family, "kind": "benchmark-source", "repository_url": str(source), "commit": revision, "tasks_tree": git(source, "rev-parse", "HEAD:tasks")})
        data = self.root / "pi-data.tgz"
        archive(data, [("package/dist/providers/data/models.json", "{}\n", 0o644)])
        pi = self.root / "sources/pi"
        for name in ("package.json", "package-lock.json", "packages/ai/package.json", "packages/coding-agent/package.json"):
            write(pi / name, json.dumps({"name": "fixture", "version": "0.84.1"}))
        integrity = "sha512-" + base64.b64encode(hashlib.sha512(data.read_bytes()).digest()).decode()
        write(pi / "packages/coding-agent/npm-shrinkwrap.json", json.dumps({"packages": {"node_modules/@earendil-works/pi-ai": {"version": "0.84.1", "integrity": integrity}}}))
        write(pi / ".gitignore", "node_modules/\npackages/*/dist/\npackages/ai/src/providers/data/\n")
        write(pi / "packages/ai/src/providers/provider.ts", "// pinned source\n")
        for name in ("docs/usage.md", "examples/example.js", "README.md", "CHANGELOG.md"):
            write(pi / "packages/coding-agent" / name, "pinned resource\n")
        pi_commit = commit(pi)
        subjects.append({"id": "pi", "kind": "agent-source", "repository_url": str(pi), "commit": pi_commit, "blobs": [blob(pi, name) for name in ("package-lock.json", "packages/coding-agent/npm-shrinkwrap.json", "packages/coding-agent/package.json")]})
        node = "#!/bin/sh\necho v22.19.0\n"
        npm = f'''#!/usr/bin/env python3
import json, pathlib, shutil, sys
args = sys.argv[1:]
if args == ["--version"]:
    print("10.9.3")
elif args[0] == "ci":
    pathlib.Path("node_modules").mkdir()
    pathlib.Path("node_modules/pinned").write_text("locked dependencies")
elif args[0] == "pack":
    target = pathlib.Path(args[args.index("--pack-destination") + 1]) / "earendil-works-pi-ai-0.84.1.tgz"
    shutil.copyfile({str(data)!r}, target)
elif args == ["run", "build:offline"]:
    path = pathlib.Path("packages/coding-agent/dist/bundle/cli.js")
    path.parent.mkdir(parents=True)
    path.write_text("// pinned fixture bundle")
    with open({str(self.events)!r}, "a") as output:
        output.write(json.dumps({{"product": "pi", "cwd": str(pathlib.Path.cwd())}}) + "\\n")
else:
    raise SystemExit("unexpected npm invocation")
'''
        node_pin = archive(self.root / "node.tar.gz", [("node/bin/node", node, 0o755), ("node/bin/npm", npm, 0o755)])
        uv = '''#!/usr/bin/env python3
import os, pathlib, sys
if sys.argv[1:] == ["--version"]:
    print("uv 0.9.7")
elif sys.argv[1:] == ["sync", "--locked", "--no-dev"]:
    root = pathlib.Path.cwd()
    assert root == pathlib.Path(os.environ["UV_PROJECT"])
    target = root / ".venv/bin"
    target.mkdir(parents=True)
    for name in ("python", root.name):
        path = target / name
        path.write_text("#!/bin/sh\\nexit 0\\n")
        path.chmod(0o755)
else:
    raise SystemExit("unexpected uv invocation")
'''
        uv_pin = archive(self.root / "uv.tar.gz", [("uv/uv", uv, 0o755)])
        tools = [{"id": "node", "version": "22.19.0", "archive": node_pin}, {"id": "uv", "version": "0.9.7", "archive": uv_pin}]
        routes = {}
        for runner, url in (("harbor", "https://github.com/harbor-framework/harbor"), ("pier", "https://github.com/datacurve-ai/pier")):
            source = self.root / "sources" / runner
            write(source / "pyproject.toml", f'[project]\nname="{runner}"\nversion="0.0.0"\n')
            write(source / "uv.lock", "version = 1\n")
            write(source / ".gitignore", ".venv/\n")
            revision = commit(source)
            tools.append({"id": runner, "commit": revision, "uv_lock": blob(source, "uv.lock")})
            routes[url] = str(source)
        write(self.bin / "git", f'''#!/usr/bin/env python3
import os, sys
routes = {routes!r}
os.execv("/usr/bin/git", ["git", *[routes.get(arg, arg) for arg in sys.argv[1:]]])
''', True)
        write(self.bin / "cargo", f'''#!/usr/bin/env python3
import json, pathlib, sys
if sys.argv[1:] == ["-V"]:
    print("cargo fixture")
else:
    root = pathlib.Path.cwd()
    assert not (root / "uncommitted-sentinel").exists()
    assert (root / "Cargo.lock").read_text() == "version = 4\\n"
    assert "--locked" in sys.argv
    executable = root / "built-opi"
    executable.write_text("#!/bin/sh\\nexit 0\\n")
    executable.chmod(0o755)
    with open({str(self.events)!r}, "a") as output:
        output.write(json.dumps({{"product": "opi", "cwd": str(root)}}) + "\\n")
    print(json.dumps({{"reason": "compiler-artifact", "target": {{"name": "opi"}}, "executable": str(executable)}}))
''', True)
        write(self.bin / "rustc", "#!/bin/sh\necho 'host: x86_64-unknown-linux-gnu'\n", True)
        producers = []
        for name in ("materialize-external-locks.sh", "build-agent-artifacts.sh", "scripted-provider.py"):
            target = self.cache / "pins/scripts" / name
            write(target, (SCRIPTS / name).read_text(), True)
            producers.append({"path": f"crates/opi-eval/scripts/{name}", "role": "materializer" if name.startswith("materialize") else "builder", "sha256": sha(target)})
        self.lock = {"schema": "opi-eval-external-lock/static/1", "authority": {"producers": producers}, "subjects": subjects, "tools": tools, "images": [], "workspace_input": {"cargo_lock_sha256": sha(self.opi / "Cargo.lock")}}
        self.lock_path = self.cache / "pins/static/linux-x86_64.json"
        self.save_lock()
        for family in FAMILIES:
            target = self.cache / "pins/profiles/benchmarks" / f"{family}.toml"
            write(target, (SCRIPTS.parent / "profiles/benchmarks" / f"{family}.toml").read_text())
        shutil.copytree(SCRIPTS.parent / "integrations", self.cache / "pins/integrations")
        self.cwd = self.root / "unrelated"
        self.cwd.mkdir()

    def save_lock(self):
        write(self.lock_path, json.dumps(self.lock))

    def environment(self):
        return {**os.environ, "PATH": str(self.bin) + os.pathsep + os.environ["PATH"]}

    def prepare(self, family, products, with_source=True):
        runner = "pier" if family == "deepswe-v1.1" else "harbor"
        ids = [f"agents/{product}" for product in products]
        ids += [f"corpora/{family}/tasks", "tools/uv", f"runners/{runner}", f"runners/{runner}/.venv/bin/python"]
        ids = [identity for identity in ids if not (self.cache / identity).exists()]
        plan = [{"id": identity, "products": products, "benchmark": family, "opi_source": str(self.opi) if with_source else None, "opi_commit": self.opi_commit if with_source else None} for identity in ids]
        plan_path = self.root / "plan.json"
        write(plan_path, json.dumps(plan))
        return subprocess.run(["bash", str(SCRIPTS / "materialize-external-locks.sh"), "prepare", "--cache", str(self.cache), "--plan", str(plan_path)], cwd=self.cwd, env=self.environment(), text=True, capture_output=True, timeout=60)


class PrepareAcquisitionTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.base = Path(self.temporary.name)

    def tearDown(self):
        self.temporary.cleanup()

    def test_repository_workspace_input_matches_static_pin(self):
        repository = SCRIPTS.parents[2]
        lock = json.loads((SCRIPTS.parent / "external-locks/static/linux-x86_64.json").read_text())
        cargo_lock = (repository / "Cargo.lock").read_bytes().replace(b"\r\n", b"\n")
        self.assertEqual(
            hashlib.sha256(cargo_lock).hexdigest(),
            lock["workspace_input"]["cargo_lock_sha256"],
        )

    def test_selected_products_and_full_corpus_for_each_runner(self):
        for family in FAMILIES:
            for products in (["opi"], ["pi"], ["opi", "pi"]):
                with self.subTest(family=family, products=products):
                    fixture = Fixture(self.base / (family + "-" + "-".join(products)))
                    result = fixture.prepare(family, products, with_source="opi" in products)
                    self.assertEqual(result.returncode, 0, result.stderr + result.stdout)
                    events = [json.loads(line) for line in fixture.events.read_text().splitlines()]
                    self.assertEqual(sorted(event["product"] for event in events), products)
                    for product in products:
                        self.assertTrue((fixture.cache / "agents" / product / product).is_file())
                    self.assertEqual(sorted(path.name for path in (fixture.cache / "corpora" / family / "tasks").iterdir()), ["openssl-selfsigned-cert", "unselected-task"])
                    if "pi" in products:
                        self.assertTrue((fixture.cache / "agents/pi/docs/usage.md").is_file())
                    before = fixture.events.read_bytes()
                    warm = fixture.prepare(family, products, with_source=False)
                    self.assertEqual(warm.returncode, 0, warm.stderr)
                    self.assertEqual(fixture.events.read_bytes(), before)

    def test_source_lock_and_producer_drift_reject_before_build(self):
        for case in ("missing-source", "missing-commit", "lock", "builder"):
            with self.subTest(case=case):
                fixture = Fixture(self.base / case)
                if case == "missing-commit":
                    fixture.opi_commit = "0" * 40
                elif case == "lock":
                    fixture.lock["workspace_input"]["cargo_lock_sha256"] = "0" * 64
                    fixture.save_lock()
                elif case == "builder":
                    write(fixture.cache / "pins/scripts/build-agent-artifacts.sh", "#!/bin/sh\nexit 0\n", True)
                result = fixture.prepare(FAMILIES[0], ["opi"], with_source=case != "missing-source")
                self.assertNotEqual(result.returncode, 0)
                self.assertFalse((fixture.cache / "agents/opi").exists())
                self.assertFalse(fixture.events.exists())


if __name__ == "__main__":
    if len(sys.argv) == 3 and sys.argv[1] == "--seed":
        fixture = Fixture(sys.argv[2])
        print(json.dumps({"cache": str(fixture.cache), "opi_source": str(fixture.opi), "opi_commit": fixture.opi_commit, "path": str(fixture.bin) + os.pathsep + os.environ["PATH"]}))
    else:
        unittest.main()
