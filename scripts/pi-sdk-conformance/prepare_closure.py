"""Materialize the opi-vendor Unicode16/ECMA/URL dependency closure.

Task 20.8 substrate preparation (DEL-04). Reproduces, from the exact
archives pinned in ``third_party/unicode16-ecma-url/closure.json``:

1. the namespace-transformed package family (10 URL/IDNA/ICU4X crates) —
   only ``Cargo.toml`` changes (package identity, internal dependency
   redirects); every other archive member stays byte-identical;
2. the Unicode 16 table generation for the regress fork, using regress's
   own upstream generator at the pinned commit over official UCD 16.0.0 +
   emoji 16 data — run twice and compared for determinism;
3. packaged ``.crate`` archives for the local sparse registry, in
   dependency order, with per-package integrity evidence. The registry
   manifest grows after each packaged crate so later packages can resolve
   their family dependencies from the running local registry.

Preparation may use the official network for ordinary crates.io
dependencies of the packaging/generator graphs (mirroring the frozen
research preparation contract); behavioral proof commands run against the
local registry only.

This is substrate preparation, not publication: publishing the family to a
real registry requires separate authorization, and the packaged artifacts
embed this run's local registry URL and must be repackaged at publication
time (see ``third_party/unicode16-ecma-url/README.md``).
"""

import argparse
import hashlib
import json
import os
import re
import shutil
import subprocess
import sys
import tarfile
import time
import zipfile
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
CLOSURE_DIR = REPO / "third_party" / "unicode16-ecma-url"
ARCHIVES = CLOSURE_DIR / "archives"
RESEARCH = REPO / "docs/research/pi-ai-opi-ai-alignment/offline-probes/readiness-schema"
HARNESS = Path(__file__).resolve().parent

REGISTRY_NAME = "opi-vendor"
VENDOR_PREFIX = "opi-vendor-"
REGRESS_VERSION = "0.12.0"
REGRESS_FORK_VERSION = "0.12.0-ucd16.1"


def sha256_file(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def fail(message: str) -> None:
    raise SystemExit(f"prepare_closure: {message}")


def load_closure() -> dict:
    return json.loads((CLOSURE_DIR / "closure.json").read_text(encoding="utf-8"))


def verify_archives(closure: dict) -> None:
    for pin in closure["inputs"]:
        archive = ARCHIVES / pin["file"]
        if not archive.is_file():
            fail(f"missing pinned archive {pin['file']}")
        got = sha256_file(archive)
        if got != pin["sha256"]:
            fail(f"archive pin mismatch for {pin['file']}: {got}")


def safe_extract_tar(archive: Path, dest: Path) -> None:
    dest_resolved = dest.resolve()
    with tarfile.open(archive) as tar:
        for member in tar.getmembers():
            target = (dest / member.name).resolve()
            if not target.is_relative_to(dest_resolved):
                fail(f"archive traversal in {archive.name}: {member.name}")
        tar.extractall(dest, filter="data")


def safe_extract_zip(archive: Path, dest: Path) -> None:
    dest_resolved = dest.resolve()
    with zipfile.ZipFile(archive) as zipped:
        for name in zipped.namelist():
            target = (dest / name).resolve()
            if not target.is_relative_to(dest_resolved):
                fail(f"zip traversal: {name}")
        zipped.extractall(dest)


# ---------------------------------------------------------------------------
# Manifest transformation
# ---------------------------------------------------------------------------

DEP_TABLE = re.compile(
    r"^\[(?:(?P<target>target\.'[^']+'\.)|(?P<target_plain>target\.[^.]+\.))?"
    r"(?P<kind>dependencies|dev-dependencies|build-dependencies)\.(?P<key>[A-Za-z0-9_-]+)\]$"
)
TOML_STRING = re.compile(r'^"(.*)"$')


def _tolerant_array(assignment: str) -> str:
    """TOML array literal -> JSON (tolerates trailing commas)."""
    value = assignment.partition("=")[2].strip()
    return re.sub(r",\s*\]", "]", value)


def _string(value: str) -> str:
    match = TOML_STRING.match(value)
    return match.group(1) if match else value


def walk_dep_tables(text: str):
    """Yield one record per dependency-table instance (scope-aware).

    Records: {"kind", "key", "target" (cfg string or None), "fields"}.
    Separate normal/dev/build instances of the same dep key stay separate,
    and target-scoped tables keep their cfg qualifier.
    """
    records = []
    current = None
    buffered_features = None
    for line in text.splitlines():
        stripped = line.strip()
        if buffered_features is not None:
            buffered_features += stripped
            if buffered_features.count("[") <= buffered_features.count("]"):
                if current is not None:
                    current["fields"]["features"] = json.loads(
                        _tolerant_array(buffered_features))
                buffered_features = None
            continue
        if stripped.startswith("["):
            match = DEP_TABLE.match(stripped)
            if match:
                target = match.group("target") or match.group("target_plain") or ""
                cfg = None
                if target:
                    inner = target[len("target."):-1]
                    cfg = inner[1:-1] if inner.startswith("'") and inner.endswith("'") else inner
                current = {"kind": match.group("kind"), "key": match.group("key"),
                           "target": cfg, "fields": {}}
                records.append(current)
            else:
                current = None
            continue
        if current is not None and "=" in stripped:
            key, _, value = stripped.partition("=")
            key = key.strip()
            if key == "features":
                buffered_features = stripped
                if buffered_features.count("[") <= buffered_features.count("]"):
                    current["fields"]["features"] = json.loads(
                        _tolerant_array(buffered_features))
                    buffered_features = None
            elif key in ("version", "optional", "default-features", "package", "registry"):
                current["fields"][key] = value.strip()
    if buffered_features is not None:
        raise SystemExit("prepare_closure: unterminated dep features array")
    return records


def parse_manifest_deps(text: str, family: dict) -> list[dict]:
    """Dependency rows (family + ordinary) for the registry manifest."""
    deps = []
    for record in walk_dep_tables(text):
        fields = record["fields"]
        if "version" not in fields:
            continue
        toml_key = record["key"]
        ours = toml_key in family
        deps.append({
            "key": toml_key,
            "name": family[toml_key] if ours else toml_key,
            "req": _string(fields["version"]),
            "ours": ours,
            "kind": {"dependencies": "normal", "dev-dependencies": "dev",
                     "build-dependencies": "build"}[record["kind"]],
            "target": record["target"],
            "optional": fields.get("optional") == "true",
            "default_features": fields.get("default-features", "true") != "false",
            "features": fields.get("features", []),
        })
    return deps


def parse_package_features(text: str) -> dict:
    """The [features] table (feature name -> enabled refs), tolerant of
    multi-line arrays and trailing commas."""
    features = {}
    in_features = False
    name = None
    buffered = None
    for line in text.splitlines():
        stripped = line.strip()
        if buffered is not None:
            buffered += stripped
            if buffered.count("[") <= buffered.count("]"):
                features[name] = json.loads(_tolerant_array(buffered))
                buffered = None
            continue
        if stripped.startswith("["):
            in_features = stripped == "[features]"
            continue
        if in_features and "=" in stripped:
            name = stripped.partition("=")[0].strip()
            buffered = stripped
            if buffered.count("[") <= buffered.count("]"):
                features[name] = json.loads(_tolerant_array(buffered))
                buffered = None
    if buffered is not None:
        raise SystemExit("prepare_closure: unterminated feature array")
    return features


def transform_family_manifest(text: str, original_name: str, family: dict) -> str:
    """Namespace-transform one family Cargo.toml (fail-closed, textual).

    ``family`` maps original package name -> vendor package name. Changes:
    the [package] name, and family dependency tables gain ``package`` and
    ``registry`` redirects. Everything else stays intact.
    """
    out = []
    in_package = False
    pin_writeable = False
    renamed = False
    for line in text.splitlines(keepends=True):
        stripped = line.rstrip("\r\n")
        if stripped == "[package]":
            in_package = True
            out.append(line)
            continue
        if stripped.startswith("["):
            in_package = False
            out.append(line)
            match = DEP_TABLE.match(stripped)
            if match and match.group("key") in family:
                out.append(f'package = "{family[match.group("key")]}"\n')
                out.append(f'registry = "{REGISTRY_NAME}"\n')
                pin_writeable = False
            elif match:
                pin_writeable = match.group("key") == "writeable"
            continue
        if pin_writeable and stripped.startswith("version = "):
            # Controlled-closure pin: writeable 0.6.4 changed macros that
            # ICU4X 2.0.0 code expects; keep the exact version the frozen
            # research graph validated (0.6.3), and request the alloc
            # feature on the dep itself (cross-registry graphs do not
            # feature-unify with the consumer's top-level enablement).
            out.append('version = "=0.6.3"\n')
            out.append('features = ["alloc"]\n')
            pin_writeable = False
            continue
        if in_package and not renamed and stripped.startswith("name = "):
            if stripped != f'name = "{original_name}"':
                fail(f"unexpected package name in {original_name}: {stripped}")
            out.append(f'name = "{family[original_name]}"\n')
            renamed = True
            continue
        out.append(line)
    if not renamed:
        fail(f"package name not found for {original_name}")
    return "".join(out)


def archive_member_bytes(archive: Path) -> dict:
    members = {}
    with tarfile.open(archive) as tar:
        for member in tar.getmembers():
            if member.isfile():
                members[member.name] = tar.extractfile(member).read()
    return members


def check_integrity(archive: Path, tree: Path, allowed_changed: set, label: str) -> dict:
    """Every archive member must be byte-identical except allowed changes."""
    original = archive_member_bytes(archive)
    unchanged, changed = 0, []
    for name, blob in original.items():
        actual = tree.parent / name
        if not actual.is_file():
            if name.endswith(".cargo_vcs_info.json") or name.endswith("/Cargo.toml.orig"):
                continue  # regenerated by cargo package from the transformed manifest
            fail(f"{label}: missing archive member {name}")
        if actual.read_bytes() == blob:
            unchanged += 1
        else:
            changed.append(name)
    unexpected = [c for c in changed if c not in allowed_changed]
    if unexpected:
        fail(f"{label}: unexpected tree drift in {unexpected}")
    return {"label": label, "unchanged_members": unchanged,
            "changed_members": sorted(changed)}


# ---------------------------------------------------------------------------
# Unicode 16 generation (upstream regress generator, pinned commit)
# ---------------------------------------------------------------------------

def generate_unicode16(closure: dict, scratch: Path, cargo_env: dict) -> dict:
    gen_pins = closure["unicode16_generator"]
    vendor = scratch / "generator" / "vendor"
    vendor.mkdir(parents=True, exist_ok=True)
    for key in ("regress_archive", "codegen_archive", "ucd_parse_archive", "regex_lite_archive"):
        safe_extract_tar(ARCHIVES / gen_pins[key]["file"], vendor)

    ucd = scratch / "generator" / "ucd-16.0.0"
    ucd.mkdir(parents=True, exist_ok=True)
    if not (ucd / "emoji-sequences.txt").exists():
        safe_extract_zip(ARCHIVES / gen_pins["ucd_zip"]["file"], ucd)
        shutil.copyfile(ARCHIVES / gen_pins["emoji_sequences"]["file"], ucd / "emoji-sequences.txt")
        shutil.copyfile(ARCHIVES / gen_pins["emoji_zwj_sequences"]["file"],
                        ucd / "emoji-zwj-sequences.txt")

    gen = vendor / f"regress-{REGRESS_VERSION}/gen-unicode"
    (gen / "src").mkdir(parents=True, exist_ok=True)
    upstream = RESEARCH / "upstream-generator"
    for source in sorted(upstream.glob("*.rs")):
        shutil.copyfile(source, gen / "src" / source.name)
    main = gen / "src" / "main.rs"
    original = main.read_text(encoding="utf-8")
    if original.count('"/tmp/ucd-17.0.0"') != 1:
        fail("upstream generator main.rs UCD path anchor changed")
    main.write_text(original.replace('"/tmp/ucd-17.0.0"', json.dumps(ucd.as_posix())),
                    encoding="utf-8")
    shutil.copyfile(upstream / "Cargo.toml.template", gen / "Cargo.toml")
    shutil.copyfile(upstream / "Cargo.lock", gen / "Cargo.lock")

    regress_src = vendor / f"regress-{REGRESS_VERSION}/src/unicodetables.rs"
    regress_tests = vendor / f"regress-{REGRESS_VERSION}/tests/unicode_property_escapes.rs"
    runs = []
    for attempt in (1, 2):
        result = subprocess.run(
            ["cargo", "run", "--release", "--locked", "--manifest-path", str(gen / "Cargo.toml")],
            cwd=gen, env=cargo_env, capture_output=True, timeout=900)
        row = {"attempt": attempt, "exit": result.returncode}
        if result.returncode == 0:
            row["table_sha256"] = sha256_file(regress_src)
            row["tests_sha256"] = sha256_file(regress_tests)
        runs.append(row)
        if result.returncode != 0:
            fail(f"unicode16 generator attempt {attempt} failed: "
                 f"{result.stderr.decode('utf-8', errors='replace')[-2000:]}")
    if runs[0]["table_sha256"] != runs[1]["table_sha256"] or \
            runs[0]["tests_sha256"] != runs[1]["tests_sha256"]:
        fail("unicode16 generation is not deterministic across runs")
    if runs[0]["table_sha256"] != gen_pins["expected_table_sha256"]:
        fail(f"generated table sha {runs[0]['table_sha256']} != pinned "
             f"{gen_pins['expected_table_sha256']}")

    shutil.copyfile(regress_src, scratch / "unicodetables-16.rs")
    shutil.copyfile(regress_tests, scratch / "unicode_property_escapes-16.rs")
    return {"runs": runs, "table_sha256": runs[0]["table_sha256"],
            "tests_sha256": runs[0]["tests_sha256"],
            "upstream_generator_commit": gen_pins["upstream_generator_commit"]}


TEST_FN = re.compile(r"fn unicode_escape_property_script_([a-z0-9_]+)_tc")
TEST_RANGE = re.compile(r"\[(\d+)\.\.=(?:\d+)?(?:, \d+\.\.=\d+)*\]")


def _script_tests(tests_text: str) -> dict:
    """Map script slug -> (first code point, full-name pattern) from a
    generated unicode_property_escapes.rs file."""
    out = {}
    for match in re.finditer(
            r"fn unicode_escape_property_script_([a-z0-9_]+)_tc\(tc: TestConfig\) \{(.*?)\n\}",
            tests_text, re.DOTALL):
        slug, body = match.group(1), match.group(2)
        range_match = TEST_RANGE.search(body)
        # The Rust source escapes the backslash: file bytes are \p{Script=X}
        marker = chr(92) * 2 + "p{Script="
        idx = body.find(marker)
        if range_match and idx >= 0:
            name = body[idx + len(marker):].split("}")[0]
            if name.isidentifier() and \
                    name.lower().replace("_", "") == slug.lower().replace("_", ""):
                out[slug] = (int(range_match.group(1)),
                             "^" + chr(92) + "p{Script=" + name + "}+$")
    return out


def derive_unicode_probes(stock_tests_text: str, ucd16_tests_text: str) -> dict:
    """Match-level probes distinguishing UCD 17 (stock) from UCD 16 (fork).

    Stock regress 0.12.0 ships UCD 17 tables; the fork ships UCD 16. Script
    classes added in UCD 17 must match their code points under stock and not
    under the fork, while scripts present in both must match under both.
    Compile-acceptance alone is not discriminating (regress accepts unknown
    script value names as empty classes).
    """
    stock = _script_tests(stock_tests_text)
    ucd16 = _script_tests(ucd16_tests_text)
    added_in_17 = sorted(set(stock) - set(ucd16))
    shared = sorted(set(stock) & set(ucd16))
    if len(added_in_17) < 3 or not shared:
        fail("script-probe derivation is vacuous")
    probes = []
    for slug in added_in_17[:6]:
        cp, pattern = stock[slug]
        probes.append({"pattern": pattern, "code_point": cp,
                       "stock_matches": True, "fork_matches": False})
    for slug in shared[:3]:
        cp, pattern = ucd16[slug]
        probes.append({"pattern": pattern, "code_point": cp,
                       "stock_matches": True, "fork_matches": True})
    return {"script_match_probes": probes,
            "derived_from": {"ucd17_scripts": len(added_in_17),
                             "shared_scripts": len(shared)}}


# ---------------------------------------------------------------------------
# Family materialization
# ---------------------------------------------------------------------------

def materialize(closure: dict, scratch: Path) -> dict:
    family = {p["name"]: VENDOR_PREFIX + p["name"].replace("_", "-") for p in closure["family"]}
    family_root = scratch / "family"
    family_root.mkdir(parents=True, exist_ok=True)
    integrity, trees = [], {}

    for spec in closure["family"]:
        name, version = spec["name"], spec["version"]
        archive = ARCHIVES / f"{name}-{version}.crate"
        if sha256_file(archive) != spec["archive_sha256"]:
            fail(f"family archive pin mismatch: {name} {version}")
        safe_extract_tar(archive, family_root)
        tree = family_root / f"{name}-{version}"
        manifest = tree / "Cargo.toml"
        manifest.write_text(
            transform_family_manifest(manifest.read_text(encoding="utf-8"), name, family),
            encoding="utf-8")
        # cargo package regenerates both of these; keeping the upstream
        # copies would collide with the regenerated content.
        (tree / ".cargo_vcs_info.json").unlink(missing_ok=True)
        (tree / "Cargo.toml.orig").unlink(missing_ok=True)
        integrity.append(check_integrity(
            archive, tree, allowed_changed={f"{name}-{version}/Cargo.toml"},
            label=family[name]))
        trees[name] = tree

    regress_archive = ARCHIVES / f"regress-{REGRESS_VERSION}.crate"
    if sha256_file(regress_archive) != closure["regress"]["archive_sha256"]:
        fail("regress archive pin mismatch")
    stock_tests = archive_member_bytes(regress_archive)[
        f"regress-{REGRESS_VERSION}/tests/unicode_property_escapes.rs"].decode("utf-8")
    safe_extract_tar(regress_archive, family_root)
    fork = family_root / f"regress-{REGRESS_VERSION}"
    shutil.copyfile(scratch / "unicodetables-16.rs", fork / "src/unicodetables.rs")
    shutil.copyfile(scratch / "unicode_property_escapes-16.rs",
                    fork / "tests/unicode_property_escapes.rs")
    manifest = fork / "Cargo.toml"
    text = manifest.read_text(encoding="utf-8")
    if text.count('name = "regress"') < 1 or text.count('version = "0.12.0"') < 1:
        fail("regress manifest anchors changed")
    text = text.replace('name = "regress"', f'name = "{VENDOR_PREFIX}regress"', 1)
    text = text.replace('version = "0.12.0"', f'version = "{REGRESS_FORK_VERSION}"', 1)
    manifest.write_text(text, encoding="utf-8")
    (fork / ".cargo_vcs_info.json").unlink(missing_ok=True)
    (fork / "Cargo.toml.orig").unlink(missing_ok=True)
    integrity.append(check_integrity(
        regress_archive, fork,
        allowed_changed={f"regress-{REGRESS_VERSION}/Cargo.toml",
                         f"regress-{REGRESS_VERSION}/src/unicodetables.rs",
                         f"regress-{REGRESS_VERSION}/tests/unicode_property_escapes.rs"},
        label=f"{VENDOR_PREFIX}regress"))
    trees["regress"] = fork

    ucd16_tests = (scratch / "unicode_property_escapes-16.rs").read_text(encoding="utf-8")
    probes = derive_unicode_probes(stock_tests, ucd16_tests)

    dep_rows, package_features = {}, {}
    for name, tree in trees.items():
        text = (tree / "Cargo.toml").read_text(encoding="utf-8")
        vendor_name = VENDOR_PREFIX + name.replace("_", "-")
        dep_rows[vendor_name] = parse_manifest_deps(text, family)
        package_features[vendor_name] = normalize_feature_refs(
            parse_package_features(text), family)
    return {"trees": trees, "integrity": integrity, "probes": probes,
            "dep_rows": dep_rows, "package_features": package_features,
            "family_names": family}


def normalize_feature_refs(features: dict, family: dict) -> dict:
    """Rewrite feature refs for the registry index under dependency renames.

    Empirical cargo 1.97 index validation (probed via minimal renamed-dep
    packages): ``dep:X`` and ``X/feat`` refs must use the dep's PACKAGE
    name (the dep row keeps ``explicit_name_in_toml``); a bare ``X`` ref
    must name a feature in this table (use the toml key when a same-named
    feature exists) or the dep's package name otherwise.
    """
    def rename_ref(ref: str) -> str:
        prefix = ""
        if ref.startswith("dep:"):
            prefix, ref = "dep:", ref[len("dep:"):]
        if "/" in ref:
            key, _, tail = ref.partition("/")
            weak = key.endswith("?")
            if weak:
                key = key[:-1]
            renamed = family.get(key, key)
            return prefix + renamed + ("?" if weak else "") + "/" + tail
        renamed = family.get(ref)
        if renamed is None:
            return prefix + ref
        if not prefix and ref in features:  # same-named feature wins for bare refs
            return prefix + ref
        return prefix + renamed

    return {name: [rename_ref(str(ref)) for ref in refs]
            for name, refs in features.items()}


def topo_order(dep_rows: dict) -> list:
    """Vendor names ordered so every family dependency is packaged first."""
    names = sorted(dep_rows)
    edges = {n: {d["name"] for d in deps if d["ours"]} for n, deps in dep_rows.items()}
    ordered, visiting = [], set()

    def visit(node):
        if node in ordered:
            return
        if node in visiting:
            fail(f"dependency cycle in family: {node}")
        visiting.add(node)
        for dep in sorted(edges[node]):
            visit(dep)
        visiting.discard(node)
        ordered.append(node)

    for name in names:
        visit(name)
    return ordered


def vendor_version(closure: dict, vendor_name: str) -> str:
    if vendor_name == f"{VENDOR_PREFIX}regress":
        return REGRESS_FORK_VERSION
    for spec in closure["family"]:
        if VENDOR_PREFIX + spec["name"].replace("_", "-") == vendor_name:
            return spec["version"]
    fail(f"unknown vendor package {vendor_name}")


def package_family(closure: dict, order: list, material: dict, registry: Path,
                   index_url: str, cargo_env: dict, cargo_target_dir: Path,
                   append_manifest_row) -> dict:
    (registry / "crates").mkdir(parents=True, exist_ok=True)
    packaged = {}
    for name in order:
        tree = material["trees"][name.removeprefix(VENDOR_PREFIX).replace("-", "_")]
        config = tree / ".cargo"
        config.mkdir(exist_ok=True)
        (config / "config.toml").write_text(f"[registries.{REGISTRY_NAME}]\nindex = \"{index_url}\"\n")
        version = vendor_version(closure, name)
        result = subprocess.run(["cargo", "package", "--no-verify"],
                                cwd=tree, env=cargo_env, capture_output=True, timeout=900)
        if result.returncode != 0:
            fail(f"cargo package failed for {name}: "
                 f"{result.stderr.decode('utf-8', errors='replace')[-3000:]}")
        crate = cargo_target_dir / "package" / f"{name}-{version}.crate"
        if not crate.is_file():
            fail(f"packaged crate not found: {crate}")
        dest = registry / "crates" / crate.name
        shutil.copyfile(crate, dest)
        packaged[name] = {"crate": crate.name, "sha256": sha256_file(dest)}
        # Publish the index row now so later family packages can resolve it.
        append_manifest_row({"name": name, "vers": version,
                             "deps": material["dep_rows"][name],
                             "features": material["package_features"][name]})
    return packaged


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--scratch", required=True)
    parser.add_argument("--registry", required=True)
    parser.add_argument("--index-url", required=True)
    parser.add_argument("--port", type=int, required=True)
    args = parser.parse_args()

    scratch = Path(args.scratch).resolve()
    registry = Path(args.registry).resolve()
    scratch.mkdir(parents=True, exist_ok=True)
    registry.mkdir(parents=True, exist_ok=True)
    (registry / "crates").mkdir(exist_ok=True)

    closure = load_closure()
    verify_archives(closure)

    cargo_home = scratch / "cargo-home"
    cargo_home.mkdir(exist_ok=True)
    target_dir = scratch / "target"
    env = dict(os.environ)
    env.update({
        "CARGO_HOME": str(cargo_home),
        "CARGO_TARGET_DIR": str(target_dir),
        "no_proxy": "127.0.0.1,localhost",
        "NO_PROXY": "127.0.0.1,localhost",
    })

    dl_url = f"http://127.0.0.1:{args.port}/crates/{{crate}}-{{version}}.crate"
    index_url = f"sparse+http://127.0.0.1:{args.port}/index/"
    manifest_path = registry / "manifest.json"
    manifest_rows = []

    def append_manifest_row(row):
        manifest_rows.append(row)
        manifest_path.write_text(json.dumps(manifest_rows, indent=1) + "\n", encoding="utf-8")

    started = time.time()
    generation = generate_unicode16(closure, scratch, env)
    material = materialize(closure, scratch)
    order = topo_order(material["dep_rows"])
    packaged = package_family(closure, order, material, registry, index_url, env,
                              target_dir, append_manifest_row)

    evidence = {
        "task": "20.8 substrate preparation",
        "closure_sha256": sha256_file(CLOSURE_DIR / "closure.json"),
        "generation": generation,
        "integrity": material["integrity"],
        "unicode_probes": material["probes"],
        "registry_order": order,
        "packaged": packaged,
        "elapsed_seconds": round(time.time() - started, 1),
    }
    (scratch / "prepare-evidence.json").write_text(
        json.dumps(evidence, indent=2) + "\n", encoding="utf-8")
    print(json.dumps({"status": "PREPARED", "order": order,
                      "table_sha256": generation["table_sha256"],
                      "elapsed_seconds": evidence["elapsed_seconds"]}))


if __name__ == "__main__":
    main()
