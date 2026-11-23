"""Lexical Rust paths used by the offline architecture and test inventory checks."""
from dataclasses import dataclass, replace
import json
import re


# Preserve line numbers. A literal becomes one opaque token so "pub" and "use"
# on either side cannot join. Raw identifiers are distinct from raw strings.
TOKEN = re.compile(
    r'(?P<space>\s+)|//[^\n]*|(?P<block>/\*)|(?P<raw>(?:br|cr|r)(?P<hashes>#{0,255})")'
    r'|(?P<literal>(?:b|c)?"(?:\\[\s\S]|[^"\\])*"|b?\'(?:\\u\{[0-9a-fA-F_]+\}|\\x[\da-fA-F]{2}|\\.|[^\'\\\n])\')'
    r'|(?P<identifier>(?:r#)?[^\W\d]\w*)|::|.', re.DOTALL,
)
BLOCK = re.compile(r"/\*|\*/")


def tokens(source):
    words, lines = [], []
    pos, line = 0, 1
    while pos < len(source):
        match = TOKEN.match(source, pos)
        value, end = match.group(), match.end()
        if match.group("block"):
            depth = 1
            for boundary in BLOCK.finditer(source, end):
                depth += 1 if boundary.group() == "/*" else -1
                if not depth:
                    end = boundary.end()
                    break
            else:
                end = len(source)
        elif match.group("raw"):
            close = '"' + match.group("hashes")
            finish = source.find(close, end)
            end = len(source) if finish < 0 else finish + len(close)
            value = "<literal>"
        if not (match.group("space") or match.group("block") or value.startswith("//")):
            value = "<literal>" if match.group("literal") else value.removeprefix("r#")
            # Keep literal file bindings. Other strings cannot introduce a module or import.
            if value == "<literal>" and words[-3:] in (["[", "path", "="], ["include", "!", "("]):
                raw = source[pos:end]
                value = raw[raw.index('"') + 1:raw.rindex('"')] if match.group("raw") else json.loads(raw)
                value = "<path>" + value
            words.append(value)
            lines.append(line)
        line += source.count("\n", pos, end)
        pos = end
    return words, lines


def groups(words):
    """Closing delimiter for each opener in valid Rust input."""
    closing, stack = {}, []
    for i, word in enumerate(words):
        if word in ("[", "(", "{"):
            stack.append(i)
        elif word in ("]", ")", "}") and stack:
            closing[stack.pop()] = i
    return closing


def module_children(file, words, *, crate_root=False):
    for path, _ in module_bindings(file, words, crate_root=crate_root):
        yield path


def module_bindings(file, words, *, crate_root=False):
    """Physical module paths and their names relative to the current file."""
    root = file.parent if crate_root or file.name == "mod.rs" else file.with_suffix("")
    dirs = [(root, file.parent, ())]
    closing = groups(words)
    override = None
    i = 0
    while i < len(words):
        token = words[i]
        base, attr_base, scope = dirs[-1]
        if words[i:i + 2] == ["#", "["]:
            if words[i + 2:i + 4] == ["path", "="] and words[i + 4].startswith("<path>"):
                override = attr_base / words[i + 4].removeprefix("<path>")
            i = closing[i + 1] + 1
            continue
        if token == "pub":
            i = closing[i + 1] + 1 if words[i + 1:i + 2] == ["("] else i + 1
            continue
        if token == "mod" and i + 2 < len(words):
            child = base / words[i + 1]
            module = (*scope, words[i + 1])
            if words[i + 2] == ";":
                if override:
                    yield override.resolve(), module
                else:
                    flat, nested = child.with_suffix(".rs"), child / "mod.rs"
                    if flat.is_file() or not nested.is_file():
                        yield flat, module
                    if nested.is_file():
                        yield nested, module
            elif words[i + 2] == "{":
                nested = override or child
                dirs.append((nested, nested, module))
                i += 3
                override = None
                continue
        if token == "{":
            dirs.append(dirs[-1])
        elif token == "}":
            dirs.pop()
        override = None
        i += 1


def cfg_without_test(words, start, end, closing):
    """True/False if provable without cfg(test), otherwise unknown."""
    if words[start:end] == ["test"]:
        return False
    if start + 2 >= end or words[start + 1] != "(" or closing[start + 1] != end - 1:
        return None
    children, first = [], start + 2
    i = first
    while i < end - 1:
        if words[i] == ",":
            children.append(cfg_without_test(words, first, i, closing))
            first = i + 1
        i = closing.get(i, i) + 1
    if first < end - 1:
        children.append(cfg_without_test(words, first, end - 1, closing))
    if words[start] == "all":
        return False if False in children else None if None in children else True
    if words[start] == "any":
        return True if True in children else None if None in children else False
    if words[start] == "not" and len(children) == 1 and children[0] is not None:
        return not children[0]
    return None


def production_tokens(words, lines):
    closing = groups(words)
    out, out_lines = [], []
    i = 0
    while i < len(words):
        inner = words[i:i + 3] == ["#", "!", "["]
        bracket = i + (2 if inner else 1)
        is_test_fn = words[bracket:bracket + 3] == ["[", "test", "]"] or (
            words[bracket:bracket + 4] == ["[", "tokio", "::", "test"]
            and words[bracket + 4:bracket + 5] in (["]"], ["("])
        )
        disabled_cfg = (words[bracket:bracket + 3] == ["[", "cfg", "("]
                        and cfg_without_test(words, bracket + 3, closing[bracket + 2], closing) is False)
        if words[i] == "#" and (is_test_fn or disabled_cfg):
            if inner:
                # An inner attribute governs this entire file or inline module.
                i = min((end for start, end in closing.items() if words[start] == "{" and start < i < end),
                        default=len(words))
            else:
                i = closing[bracket] + 1
                while i < len(words) and words[i] not in ("{", ";", "}"):
                    i = closing.get(i, i) + 1
                if i < len(words) and words[i] != "}":
                    i = closing.get(i, i) + 1
            continue
        out.append(words[i])
        out_lines.append(lines[i])
        i += 1
    return out, out_lines


def use_leaves(words, start, end, prefix=()):
    """Expand Rust use trees to (path, as-name, source token index)."""
    i, path = start, prefix
    while i < end:
        word = words[i]
        if word == "{":
            child_start, depth = i + 1, 0
            for j in range(child_start, end):
                if words[j] == "{":
                    depth += 1
                elif words[j] == "}":
                    if not depth:
                        yield from use_leaves(words, child_start, j, path)
                        return
                    depth -= 1
                elif words[j] == "," and not depth:
                    yield from use_leaves(words, child_start, j, path)
                    child_start = j + 1
            return
        if word == "as":
            yield path, words[i + 1], i - 1
            return
        if word != "::" and not (word == "self" and path):
            path += (word,)
        i += 1
    if start < end:
        yield path, None, end - 1


@dataclass(frozen=True)
class RustReference:
    path: tuple[str, ...]
    line: int
    kind: str
    alias: str | None
    top_level: bool
    following: str = ""
    previous: str = ""

    @property
    def crate_namespace(self) -> bool:
        return self.kind != "qualified" and self.path in (("crate",), ("crate", "*"))


def rust_module(relative_path):
    parts = relative_path.with_suffix("").parts
    if parts[-1] in {"mod", "lib"}:
        parts = parts[:-1]
    return ("crate", *parts)


def source_references(source, module, *, production=True):
    """Read lexical paths, preserving imports, public exports and qualified uses."""
    words, lines = tokens(source)
    if production:
        words, lines = production_tokens(words, lines)
    scopes = [module]
    references = []
    i = 0
    while i < len(words):
        word = words[i]
        if words[i:i + 4] == ["extern", "crate", "self", "as"]:
            references.append((RustReference(("crate",), lines[i], "extern crate", words[i + 4], len(scopes) == 1),
                               scopes[-1]))
        if word == "use" and words[i + 1:i + 2] not in (["("], ["<"]):
            end = words.index(";", i)
            kind = "pub use" if i and words[i - 1] == "pub" else "use"
            for path, alias, pos in use_leaves(words, i + 1, end):
                references.append((RustReference(path, lines[pos], kind, alias, len(scopes) == 1), scopes[-1]))
            i = end + 1
            continue
        if word == "{":
            nested = (words[i - 1],) if i >= 2 and words[i - 2] == "mod" else ()
            scopes.append(scopes[-1] + nested)
        elif word == "}":
            scopes.pop()
        elif word.isidentifier() and words[i + 1:i + 2] == ["::"]:
            end = i + 1
            while words[end:end + 1] == ["::"] and end + 1 < len(words) and words[end + 1].isidentifier():
                end += 2
            path = tuple(words[i:end:2])
            following = words[end] if end < len(words) else ""
            previous = words[i - 1] if i else ""
            references.append((RustReference(path, lines[i], "qualified", None, len(scopes) == 1, following, previous),
                               scopes[-1]))
            i = end
            continue
        i += 1
    return [replace(reference, path=absolute_path(reference.path, scope)) for reference, scope in references]


def absolute_path(path, module):
    if path and path[0] in ("self", "super"):
        while path and path[0] in ("self", "super"):
            if path[0] == "super":
                module = module[:-1]
            path = path[1:]
        return module + path
    if path[:1] == ("opi_ai",):
        return ("crate",) + path[1:]
    return path


def production_files(files, *, entrypoints=()):
    """Exclude files reachable only by test module declarations."""
    declared, active = set(), {}
    for file, source in files.items():
        words, lines = tokens(source)
        declared.update(module_children(file, words))
        live, _ = production_tokens(words, lines)
        active[file] = set(module_children(file, live)) & files.keys()

    # Audit orphans too. A live binding takes precedence over a test-only binding.
    pending = list(files.keys() - declared) + [p for p in entrypoints if p in files]
    reachable = set()
    while pending:
        file = pending.pop()
        if file not in reachable:
            reachable.add(file)
            pending.extend(active[file])
    return reachable
