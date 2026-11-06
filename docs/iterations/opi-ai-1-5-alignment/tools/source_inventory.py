"""Read-only source inventory; syntax records are not behavioral coverage.

Install requirements into an isolated tool directory, then pass --parser-path.
No Cargo/Node execution, network, generated-source edits, or product imports.
"""
import argparse
import fnmatch
import hashlib
import json
from pathlib import Path
import re
import sys

SOURCE_BYTES = None


def node_bytes(node):
    return SOURCE_BYTES[node.start_byte:node.end_byte] if SOURCE_BYTES is not None else node.text


def sha(data):
    return hashlib.sha256(data).hexdigest()


def walk(node):
    yield node
    for child in node.named_children:
        yield from walk(child)


def text(node):
    return node_bytes(node).decode("utf-8") if node else ""


def at(node, path):
    return {"path": path, "line": node.start_point.row + 1,
            "end_line": node.end_point.row + 1, "kind": node.type}


def name(node):
    return text(node.child_by_field_name("name"))


def signature(node):
    body = node.child_by_field_name("body")
    raw = node_bytes(node)[:body.start_byte - node.start_byte] if body else node_bytes(node)
    return re.sub(r"\s+", " ", raw.decode("utf-8")).strip()


def declarations(node, path):
    if node.type in ("lexical_declaration", "variable_declaration"):
        return [{**at(c, path), "name": name(c), "classification": "runtime"}
                for c in node.named_children if c.type == "variable_declarator"]
    if name(node):
        item = {**at(node, path), "name": name(node),
                "classification": "type-only" if node.type in (
                    "interface_declaration", "type_alias_declaration") else "runtime"}
        if "function" in node.type:
            item["signature"] = signature(node)
        body = node.child_by_field_name("body")
        if body and body.type in ("interface_body", "class_body"):
            item["declaration_header"] = signature(node)
            item["member_scope"] = "own declarations only; inherited members remain on their declaring type"
            item["members"] = []
            for member in body.named_children:
                if member.type == "comment":
                    continue
                if any(text(c) in ("private", "protected") for c in member.named_children):
                    continue
                member_name = name(member)
                if member_name.startswith("#"):
                    continue
                item["members"].append({**at(member, path), "name": member_name,
                                         "signature": signature(member)})
        return [item]
    return []


def ts_module(tree, path):
    local, direct, edges, imports = {}, {}, [], {}
    for statement in tree.root_node.named_children:
        if statement.type == "import_statement":
            source = text(statement.child_by_field_name("source")).strip("\"'")
            for spec in walk(statement):
                if spec.type == "import_specifier":
                    original = text(spec.child_by_field_name("name"))
                    alias = text(spec.child_by_field_name("alias")) or original
                    imports[alias] = {"source": source, "original": original}
        if statement.type != "export_statement":
            for d in declarations(statement, path):
                local.setdefault(d["name"], []).append(d)
            continue
        source = statement.child_by_field_name("source")
        clause = next((c for c in statement.named_children if c.type == "export_clause"), None)
        namespace = next((c for c in statement.named_children if c.type == "namespace_export"), None)
        if source or clause:
            specs = []
            if clause:
                for spec in clause.named_children:
                    n = spec.child_by_field_name("name")
                    alias = spec.child_by_field_name("alias")
                    specs.append({"original": text(n), "exported": text(alias or n),
                                  "type_only": text(spec).startswith("type ")})
            edges.append({**at(statement, path), "source": text(source).strip("\"'"),
                          "specs": specs, "star": not clause and not namespace,
                          "namespace": text(namespace),
                          "type_only": text(statement).startswith("export type ")})
        else:
            for child in statement.named_children:
                for d in declarations(child, path):
                    direct.setdefault(d["name"], []).append(d)
                    local.setdefault(d["name"], []).append(d)
    return {"local": local, "direct": direct, "edges": edges, "imports": imports}


def resolve_exports(modules):
    cache, unresolved = {}, []

    def resolve(path, stack=()):
        if path in cache:
            return cache[path]
        if path in stack:
            unresolved.append({"path": path, "reason": "cyclic re-export requires compiler resolution", "cycle": list(stack) + [path]})
            return modules[path]["direct"]
        if path not in modules:
            return {}
        module = modules[path]
        result = dict(module["direct"])
        for edge in module["edges"]:
            source = edge["source"]
            if source.startswith("."):
                import posixpath
                target = posixpath.normpath(str(Path(path).parent.as_posix()) + "/" + source)
                target = re.sub(r"\.js$", ".ts", target)
                other = resolve(target, stack + (path,))
            elif not source:
                target, other = path, module["local"]
            else:
                target, other = source, {}
            if edge["namespace"]:
                unresolved.append({**edge, "reason": "namespace export requires review"})
            elif edge["star"]:
                if not other:
                    unresolved.append({**edge, "reason": "star target missing", "target": target})
                for key, value in other.items():
                    if key != "default":
                        result.setdefault(key, value)
            else:
                for spec in edge["specs"]:
                    values = other.get(spec["original"])
                    if values is None and not source and spec["original"] in module["imports"]:
                        imported = module["imports"][spec["original"]]
                        import posixpath
                        target = posixpath.normpath(str(Path(path).parent.as_posix()) + "/" + imported["source"])
                        values = resolve(target, stack + (path,)).get(imported["original"])
                    if values is None:
                        unresolved.append({**edge, "reason": "named target missing", "symbol": spec,
                                           "target": target})
                        continue
                    result[spec["exported"]] = [
                        {**v, "classification": "type-only"} if edge["type_only"] or spec["type_only"] else v
                        for v in values]
        cache[path] = result
        return result

    for path in modules:
        resolve(path)
    return cache, unresolved


def ts_cases(tree, path):
    cases = []
    for n in walk(tree.root_node):
        if n.type != "call_expression":
            continue
        func = text(n.child_by_field_name("function"))
        if not re.match(r"^(?:test|it)(?:\b|\.)", func) or func in ("test.each", "it.each"):
            continue
        args = n.child_by_field_name("arguments")
        if not args or not args.named_children:
            continue
        first = args.named_children[0]
        # Excludes table-builder calls. Keeps dynamic/template declarations;
        # actual parameter expansion is a later runtime runner obligation.
        if (len(args.named_children) < 2 and ".todo" not in func) or first.type in ("arrow_function", "function_expression"):
            continue
        title = text(first)
        if first.type not in ("string", "template_string", "identifier", "binary_expression"):
            continue
        suite, suite_forms = [], []
        parent = n.parent
        while parent:
            if parent.type == "call_expression":
                f = text(parent.child_by_field_name("function"))
                a = parent.child_by_field_name("arguments")
                if re.match(r"^describe\b", f) and a and a.named_children:
                    suite.insert(0, text(a.named_children[0]))
                    suite_forms.insert(0, f)
            parent = parent.parent
        cases.append({**at(n, path), "id": f"{path}#byte={n.start_byte}",
                      "title": title, "suite": suite, "suite_forms": suite_forms, "form": func,
                      "parameterized": ".each" in func or "for(" in func,
                      "skip_or_conditional": bool(re.search(r"skip|todo|runIf|skipIf", " ".join([func] + suite_forms))),
                      "body_sha256": sha(node_bytes(n)), "execution": "not-run"})
    return cases


def rust_records(tree, path):
    public, cases = [], []
    for n in walk(tree.root_node):
        if n.type in ("comment", "line_comment", "block_comment"):
            continue
        attributes = []
        prev = n.prev_named_sibling
        while prev and prev.type in ("attribute_item", "line_comment", "block_comment"):
            if prev.type == "attribute_item":
                attributes.insert(0, text(prev))
            prev = prev.prev_named_sibling
        attrs = " ".join(attributes)
        chain, parent = [], n.parent
        while parent:
            if parent.type in ("mod_item", "impl_item", "trait_item", "enum_item", "struct_item"):
                chain.insert(0, name(parent) or text(parent.child_by_field_name("type")))
            parent = parent.parent
        if n.type == "function_item" and re.search(r"#\[(?:[\w:]*::)?test(?:\]|\()", attrs):
            cases.append({**at(n, path), "id": f"{path}#byte={n.start_byte}",
                          "name": name(n), "scope": chain, "attributes": attributes,
                          "body_sha256": sha(node_bytes(n)), "execution": "not-run"})
        vis = next((c for c in n.named_children if c.type == "visibility_modifier"), None)
        implicit_public = n.type in ("enum_variant", "function_signature_item", "associated_type")
        if (vis and text(vis) == "pub") or implicit_public:
            item = {**at(n, path), "id": f"{path}#byte={n.start_byte}", "name": name(n),
                    "scope": chain, "attributes": attributes,
                    "visibility": "pub" if vis else "enclosing-type",
                    "reachability": "declaration-candidate; verify module/re-export/cfg at migration"}
            if n.type == "use_declaration" or "function" in n.type:
                item["signature"] = signature(n)
            if "tests" not in chain and "test" not in chain:
                public.append(item)
    return public, cases


def inventory(repo, parser_path):
    global SOURCE_BYTES
    if parser_path:
        sys.path.insert(0, str(parser_path))
    from tree_sitter import Language, Parser
    import tree_sitter_typescript
    import tree_sitter_rust
    from importlib.metadata import version
    parser_versions = {package: version(package) for package in ("tree-sitter", "tree-sitter-typescript", "tree-sitter-rust")}
    if parser_versions != {"tree-sitter": "0.25.2", "tree-sitter-typescript": "0.23.2", "tree-sitter-rust": "0.24.0"}:
        raise RuntimeError(f"Parser versions differ from the reviewed requirements: {parser_versions}")
    ts = Parser(Language(tree_sitter_typescript.language_typescript()))
    rs = Parser(Language(tree_sitter_rust.language()))
    upstream = repo / ".pi/pi-1.5.0"
    files, modules, cases, rust_public, errors, adjustments = [], {}, [], [], [], []
    groups = [(upstream / "packages/ai/src", "ts-source", ts, upstream, "*.ts"),
              (upstream / "packages/ai/test", "ts-test", ts, upstream, "*.ts"),
              (repo / "crates/opi-ai/src", "rust-source", rs, repo, "*.rs"),
              (repo / "crates/opi-ai/tests", "rust-test", rs, repo, "*.rs")]
    for root, role, parser, base, glob in groups:
        for file in sorted(root.rglob(glob)):
            raw = file.read_bytes()
            path = file.relative_to(base).as_posix()
            parse_bytes = raw
            # Pinned grammars reject legal tagged-template incomplete escapes
            # and misread a Rust variable named raw after & as raw-reference
            # syntax. Adjust only the in-memory parse buffer, at equal byte
            # length. Every recorded text/hash still comes from original bytes.
            if role.startswith("ts"):
                def mask_template(match):
                    return match[0].replace(b"\\u", b"\\v")
                parse_bytes = re.sub(rb"String\.raw`(?:\\.|[^`])*`", mask_template, raw)
            else:
                parse_bytes = re.sub(rb"&raw\b(?!\s+(?:const|mut)\b)", b"&r_w", raw)
            if parse_bytes != raw:
                adjustments.append({"path": path, "reason": "tagged-raw escape grammar" if role.startswith("ts") else "raw identifier grammar",
                                    "offsets_preserved": len(parse_bytes) == len(raw)})
            SOURCE_BYTES = raw
            tree = parser.parse(parse_bytes)
            files.append({"path": path, "role": role, "sha256": sha(raw), "bytes": len(raw)})
            for n in walk(tree.root_node):
                if n.is_error or n.is_missing:
                    errors.append({**at(n, path), "missing": n.is_missing})
            if role == "ts-source":
                modules[path] = ts_module(tree, path)
            if role == "ts-test":
                cases.extend(ts_cases(tree, path))
            if role.startswith("rust"):
                declarations_, tests = rust_records(tree, path)
                cases.extend(tests)
                if role == "rust-source":
                    rust_public.extend(declarations_)
    exports, unresolved = resolve_exports(modules)
    package = json.loads((upstream / "packages/ai/package.json").read_text(encoding="utf-8"))
    entrypoints, symbols = [], []
    for key, conditions in package["exports"].items():
        pattern = conditions["import"].replace("./dist/", "packages/ai/src/").removesuffix(".js") + ".ts"
        for file in sorted(modules):
            if file.endswith(".d.ts") or not fnmatch.fnmatchcase(file, pattern):
                continue
            entry = key
            if "*" in key:
                before, after = pattern.split("*")
                entry = key.replace("*", file[len(before):len(file) - len(after)])
            entrypoints.append({"entrypoint": entry, "path": file, "basis": "source projected by package exports; dist absent"})
            for symbol, definitions in sorted(exports[file].items()):
                symbols.append({"id": f"{entry}::{symbol}", "entrypoint": entry, "name": symbol,
                                "definitions": definitions, "execution": "not-run"})
    return {"schema_version": 1, "method": "Tree-sitter syntax; no typecheck, macro expansion, test execution or artifact export proof",
            "parser_versions": parser_versions,
            "files": files, "entrypoints": entrypoints, "upstream_symbols": symbols,
            "rust_declarations": rust_public, "test_declarations": cases,
            "unresolved_reexports": unresolved, "syntax_errors": errors,
            "parser_adjustments": adjustments}


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("--repo", type=Path, required=True)
    p.add_argument("--parser-path", type=Path)
    p.add_argument("--output", type=Path, required=True)
    args = p.parse_args()
    data = inventory(args.repo.resolve(), args.parser_path)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(data, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    print(json.dumps({k: len(data[k]) for k in ("files", "entrypoints", "upstream_symbols", "rust_declarations",
                                              "test_declarations", "unresolved_reexports", "syntax_errors")}))
    if data["unresolved_reexports"] or data["syntax_errors"]:
        raise SystemExit(1)


if __name__ == "__main__":
    main()
