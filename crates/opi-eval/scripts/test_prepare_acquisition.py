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
    def __init__(self, root, runtime_case=None):
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
        runtime_versions = {"@earendil-works/chord": "0.84.1", "@silvia-odwyer/photon-node": "0.3.4", "jiti": "2.7.0", "esbuild": "0.28.1", "@esbuild/linux-x64": "0.28.1"}
        runtime_lock = {"node_modules/" + name: {"version": version} for name, version in runtime_versions.items()}
        runtime_lock["node_modules/@earendil-works/chord"] = {"link": True, "resolved": "packages/chord"}
        runtime_lock["packages/chord"] = {"version": "0.84.1", "dependencies": {"esbuild": "0.28.1"}}
        write(pi / "package-lock.json", json.dumps({"packages": runtime_lock}))
        write(pi / "packages/chord/package.json", json.dumps({"name": "@earendil-works/chord", "version": "0.84.1", "dependencies": {"esbuild": "0.28.1"}}))
        write(pi / "packages/chord/README.md", "chord runtime\n")
        write(pi / "LICENSE", "MIT fixture license\n")
        integrity = "sha512-" + base64.b64encode(hashlib.sha512(data.read_bytes()).digest()).decode()
        write(pi / "packages/coding-agent/npm-shrinkwrap.json", json.dumps({"packages": {"node_modules/@earendil-works/pi-ai": {"version": "0.84.1", "integrity": integrity}}}))
        write(pi / ".gitignore", "node_modules/\npackages/*/dist/\npackages/ai/src/providers/data/\n")
        write(pi / "packages/ai/src/providers/provider.ts", "// pinned source\n")
        for name in ("docs/usage.md", "examples/example.js", "README.md", "CHANGELOG.md"):
            write(pi / "packages/coding-agent" / name, "pinned resource\n")
        pi_commit = commit(pi)
        subjects.append({"id": "pi", "kind": "agent-source", "repository_url": str(pi), "commit": pi_commit, "blobs": [blob(pi, name) for name in ("package-lock.json", "packages/coding-agent/npm-shrinkwrap.json", "packages/coding-agent/package.json")], "model_data_archive": {"package": "@earendil-works/pi-ai", "version": "0.84.1", "url": data.as_uri(), "sha256": sha(data)}})
        node = f'''#!/usr/bin/env python3
import json, os, pathlib, sys
args = sys.argv[1:]
if args == ["--version"]:
    print("v22.19.0")
else:
    assert "NODE_PATH" not in os.environ and "NODE_OPTIONS" not in os.environ
    assert "API_KEY" not in os.environ
    assert pathlib.Path.home() != pathlib.Path({str(self.root)!r})
    assert pathlib.Path.cwd().name != "pi-source"
    with open({str(self.root / 'runtime-checks.jsonl')!r}, "a") as output:
        output.write(json.dumps(args) + "\\n")
    if args[-1] == "--version":
        if {runtime_case!r} == "startup-failure": sys.exit(9)
        print("wrong-version" if {runtime_case!r} == "startup-version" else "0.84.1")
    elif {runtime_case!r} == "import-failure":
        sys.exit(8)
'''
        npm = f'''#!/usr/bin/env python3
import json, pathlib, sys
args = sys.argv[1:]
if args == ["--version"]:
    print("10.9.3")
elif args[0] == "ci":
    pathlib.Path("node_modules").mkdir()
    pathlib.Path("node_modules/pinned").write_text("locked dependencies")
    for name, version in {runtime_versions!r}.items():
        target = pathlib.Path("node_modules") / name
        target.parent.mkdir(parents=True, exist_ok=True)
        if name == "@earendil-works/chord":
            target.symlink_to("../../packages/chord")
        else:
            target.mkdir()
            (target / "package.json").write_text(json.dumps({{"name": name, "version": version}}))
            (target / "LICENSE").write_text("locked license")
            (target / "runtime.js").write_text("locked runtime")
            if name == "@esbuild/linux-x64":
                (target / "bin").mkdir()
                binary = target / "bin/esbuild"
                binary.write_text("native executable")
                binary.chmod(0o755)
elif args == ["run", "build:offline"]:
    assert pathlib.Path("packages/ai/src/providers/data/models.json").read_text() == "{{}}\\n"
    path = pathlib.Path("packages/coding-agent/dist/bundle/cli.js")
    path.parent.mkdir(parents=True)
    path.write_text("// pinned fixture bundle")
    chord = pathlib.Path("packages/chord/dist")
    chord.mkdir()
    (chord / "index.js").write_text("// built chord")
    case = {runtime_case!r}
    if case and case.startswith("missing:"):
        import shutil
        target = pathlib.Path("node_modules") / case.split(":", 1)[1]
        if target.is_symlink(): target.unlink()
        else: shutil.rmtree(target)
    elif case in ("identity", "chord-identity"):
        target = pathlib.Path("node_modules/jiti/package.json" if case == "identity" else "packages/chord/package.json")
        doc = json.loads(target.read_text())
        doc["version"] = "0.0.0"
        target.write_text(json.dumps(doc))
    elif case in ("workspace-escape", "workspace-target"):
        target = pathlib.Path("node_modules/@earendil-works/chord")
        target.unlink()
        target.symlink_to("/tmp" if case == "workspace-escape" else "../../packages/ai")
    elif case == "nested-link":
        pathlib.Path("node_modules/jiti/escape").symlink_to("/etc/passwd")
    elif case == "special-file":
        import os
        os.mkfifo("packages/chord/dist/fifo")
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
                        package = fixture.cache / "agents/pi"
                        for name in ("@earendil-works/chord", "@silvia-odwyer/photon-node", "jiti", "esbuild", "@esbuild/linux-x64"):
                            self.assertTrue((package / "node_modules" / name / "package.json").is_file())
                        self.assertTrue((package / "node_modules/@earendil-works/chord/LICENSE").is_file())
                        self.assertTrue((package / "node_modules/@esbuild/linux-x64/bin/esbuild").stat().st_mode & 0o111)
                        self.assertFalse((package / "node_modules/pinned").exists())
                        self.assertFalse(any(path.is_symlink() for path in package.rglob("*")))
                        checks = [json.loads(line) for line in (fixture.root / "runtime-checks.jsonl").read_text().splitlines()]
                        self.assertEqual(len(checks), 2)
                    before = fixture.events.read_bytes()
                    warm = fixture.prepare(family, products, with_source=False)
                    self.assertEqual(warm.returncode, 0, warm.stderr)
                    self.assertEqual(fixture.events.read_bytes(), before)

    def test_pi_runtime_closure_rejects_before_publication(self):
        cases = ["missing:" + name for name in ("@earendil-works/chord", "@silvia-odwyer/photon-node", "jiti", "esbuild", "@esbuild/linux-x64")]
        cases += ["identity", "chord-identity", "workspace-escape", "workspace-target", "nested-link", "special-file", "startup-failure", "startup-version", "import-failure"]
        for index, case in enumerate(cases):
            with self.subTest(case=case):
                fixture = Fixture(self.base / str(index), runtime_case=case)
                result = fixture.prepare(FAMILIES[0], ["pi"], with_source=False)
                self.assertNotEqual(result.returncode, 0, result.stdout)
                self.assertFalse((fixture.cache / "agents/pi").exists())

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

    def test_static_pi_data_pin_accepts_source_without_shrinkwrap_integrity(self):
        fixture = Fixture(self.base / "absent-sri")
        source = fixture.root / "sources/pi"
        relative = "packages/coding-agent/npm-shrinkwrap.json"
        shrinkwrap = json.loads((source / relative).read_text())
        del shrinkwrap["packages"]["node_modules/@earendil-works/pi-ai"]["integrity"]
        write(source / relative, json.dumps(shrinkwrap))
        revision = commit(source)
        subject = next(row for row in fixture.lock["subjects"] if row["id"] == "pi")
        subject["commit"] = revision
        subject["blobs"] = [blob(source, row["path"]) for row in subject["blobs"]]
        fixture.save_lock()
        result = fixture.prepare(FAMILIES[0], ["pi"], with_source=False)
        self.assertEqual(result.returncode, 0, result.stderr + result.stdout)
        self.assertTrue((fixture.cache / "agents/pi/pi").is_file())
        self.assertEqual(json.loads(fixture.events.read_text())["product"], "pi")

    def test_pi_source_without_published_shrinkwrap_builds_with_null_digest(self):
        fixture = Fixture(self.base / "absent-shrinkwrap")
        source = fixture.root / "sources/pi"
        (source / "packages/coding-agent/npm-shrinkwrap.json").unlink()
        revision = commit(source)
        subject = next(row for row in fixture.lock["subjects"] if row["id"] == "pi")
        subject["commit"] = revision
        subject["blobs"] = [blob(source, row["path"]) for row in subject["blobs"]
                             if row["path"] != "packages/coding-agent/npm-shrinkwrap.json"]
        fixture.save_lock()
        result = fixture.prepare(FAMILIES[0], ["pi"], with_source=False)
        self.assertEqual(result.returncode, 0, result.stderr + result.stdout)
        identity = json.loads((fixture.cache / "agents/pi/build-identity.json").read_text())
        self.assertIsNone(identity["shrinkwrap_sha256"])
        self.assertTrue((fixture.cache / "agents/pi/pi").is_file())

    def test_invalid_pi_data_pins_reject_before_build_or_publication(self):
        for case in ("missing", "malformed", "duplicate-subject", "duplicate-field", "package", "version", "url", "digest", "uppercase-digest", "tampered"):
            with self.subTest(case=case):
                fixture = Fixture(self.base / case)
                subject = next(row for row in fixture.lock["subjects"] if row["id"] == "pi")
                pin = subject["model_data_archive"]
                if case == "missing":
                    del subject["model_data_archive"]
                elif case == "malformed":
                    subject["model_data_archive"] = []
                elif case == "duplicate-subject":
                    fixture.lock["subjects"].append(subject.copy())
                elif case == "package":
                    pin["package"] = "wrong-package"
                elif case == "version":
                    pin["version"] = "0.0.0"
                elif case == "url":
                    pin["url"] = "relative/archive.tgz"
                elif case == "digest":
                    pin["sha256"] = "invalid"
                elif case == "uppercase-digest":
                    pin["sha256"] = "A" * 64
                elif case == "tampered":
                    (fixture.root / "pi-data.tgz").write_bytes(b"untrusted archive bytes")
                fixture.save_lock()
                if case == "duplicate-field":
                    text = fixture.lock_path.read_text()
                    write(fixture.lock_path, text.replace('"model_data_archive":', '"model_data_archive": {}, "model_data_archive":', 1))
                result = fixture.prepare(FAMILIES[0], ["pi"], with_source=False)
                self.assertNotEqual(result.returncode, 0, result.stdout)
                self.assertIn("pi build data:", result.stderr)
                self.assertFalse((fixture.cache / "agents/pi").exists())
                self.assertFalse(fixture.events.exists())

    def test_pi_builder_requires_explicit_static_lock(self):
        fixture = Fixture(self.base / "missing-static-lock")
        result = subprocess.run(
            ["bash", str(SCRIPTS / "build-agent-artifacts.sh"), "--pi-source", str(fixture.root / "sources/pi"), "--out", str(fixture.root / "build")],
            cwd=fixture.cwd, env=fixture.environment(), text=True, capture_output=True, timeout=60,
        )
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("--static-lock is required", result.stderr)
        self.assertFalse(fixture.events.exists())

    def test_selected_image_pulls_are_immutable_verified_and_reusable(self):
        for case in ("cold-warm", "missing", "duplicate", "manifest", "reference", "pull-failed", "digest"):
            with self.subTest(case=case):
                room = self.base / case
                cache = room / "cache"
                bin_dir = room / "bin"
                marker = room / "pulled"
                events = room / "pulls"
                manifest = "sha256:" + "a" * 64
                image = {"id": "selected", "reference": "selected:1", "manifest": manifest}
                lock = {"subjects": [], "tools": [], "images": [image]}
                if case == "missing":
                    lock["images"] = []
                elif case == "duplicate":
                    lock["images"].append(image.copy())
                elif case == "manifest":
                    image["manifest"] = "sha256:bad"
                elif case == "reference":
                    image["reference"] = "--unsafe"
                write(cache / "pins/static/linux-x86_64.json", json.dumps(lock))
                write(room / "plan.json", json.dumps([{"id": "images/selected", "products": ["pi"], "benchmark": FAMILIES[1]}]))
                write(bin_dir / "docker", f'''#!/usr/bin/python3
import json, pathlib, sys
args = sys.argv[1:]
marker = pathlib.Path({str(marker)!r})
if args[:2] == ["image", "inspect"]:
    if not marker.exists(): sys.exit(1)
    print(json.dumps(["selected:1@" + ("sha256:"+"0"*64 if {case!r} == "digest" else {manifest!r})]))
elif args[0] == "pull":
    assert args == ["pull", "--quiet", "selected:1@"+{manifest!r}], args
    with open({str(events)!r}, "a") as output: output.write("pull\\n")
    if {case!r} == "pull-failed": sys.exit(9)
    marker.write_text("pulled")
else: raise SystemExit("unexpected Docker command")
''', True)
                argv = ["bash", str(SCRIPTS / "materialize-external-locks.sh"), "prepare", "--cache", str(cache), "--plan", str(room / "plan.json")]
                environment = {"HOME": str(room), "PATH": str(bin_dir) + ":/usr/bin:/bin"}
                result = subprocess.run(argv, env=environment, capture_output=True, text=True, timeout=20)
                if case == "cold-warm":
                    self.assertEqual(result.returncode, 0, result.stderr)
                    warm = subprocess.run(argv, env=environment, capture_output=True, text=True, timeout=20)
                    self.assertEqual(warm.returncode, 0, warm.stderr)
                    self.assertEqual(events.read_text(), "pull\n")
                else:
                    self.assertNotEqual(result.returncode, 0, result.stdout)
                    if case not in ("pull-failed", "digest"):
                        self.assertFalse(events.exists())


if __name__ == "__main__":
    if len(sys.argv) == 3 and sys.argv[1] == "--seed":
        fixture = Fixture(sys.argv[2])
        print(json.dumps({"cache": str(fixture.cache), "opi_source": str(fixture.opi), "opi_commit": fixture.opi_commit, "path": str(fixture.bin) + os.pathsep + os.environ["PATH"]}))
    else:
        unittest.main()
