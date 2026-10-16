"""Apply the bounded research change to an isolated exact Hyper source copy.

This is a prototype of response header-slot storage, not a complete selected
production patch. It does not fix trailer/header-buffer policy, and these stay
explicit implementation obligations. Original parsing/framing is unchanged.
"""
from pathlib import Path
import difflib
import hashlib
import json
import sys

root = Path(sys.argv[1]).resolve()
source = root / "src/proto/h1/role.rs"
before = source.read_text(encoding="utf-8")
if hashlib.sha256(before.encode()).hexdigest() != "c13307092a033ae9696893c83f27f75f3dbe5da50d17a2c58cd8e98ebfbf1917":
    raise SystemExit("Hyper role.rs source pin mismatch; no mutation")
start = before.index("impl Http1Transaction for Client {")
head, section = before[:start], before[start:]
needle = """        loop {
            let mut headers_indices:"""
insert = """        loop {
            // Research: count only already-buffered head lines, stopping at
            // the first empty line; coalesced body bytes do not buy slots.
            let header_bytes: &[u8] = buf.as_ref();
            let dynamic_header_slots = header_bytes
                .split(|byte| *byte == b'\\n')
                .take_while(|line| !line.is_empty() && *line != b\"\\r\")
                .count()
                .max(DEFAULT_MAX_HEADERS);
            let mut headers_indices:"""
assert section.count(needle) == 1
section = section.replace(needle, insert, 1)
needle = "None => smallvec_inline![MaybeUninit::uninit(); DEFAULT_MAX_HEADERS],"
replacement = """None if dynamic_header_slots > DEFAULT_MAX_HEADERS =>
                        smallvec![MaybeUninit::uninit(); dynamic_header_slots],
                    None => smallvec_inline![MaybeUninit::uninit(); DEFAULT_MAX_HEADERS],"""
# Exactly the response parser's two arrays. Server parsing remains untouched.
assert section.count(needle) == 2
section = section.replace(needle, replacement)
after = head + section
source.write_text(after, encoding="utf-8", newline="\n")
patch = "".join(difflib.unified_diff(before.splitlines(True), after.splitlines(True),
                                   fromfile="hyper-1.9.0/src/proto/h1/role.rs",
                                   tofile="hyper-1.9.0-research/src/proto/h1/role.rs"))
(root.parent / "hyper-response-slots.patch").write_text(patch, encoding="utf-8")
print(json.dumps({"before_sha256": hashlib.sha256(before.encode()).hexdigest(),
                  "after_sha256": hashlib.sha256(after.encode()).hexdigest(),
                  "response_array_changes": 2, "scope": "response header-slot storage only"}))
