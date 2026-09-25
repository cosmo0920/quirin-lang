#!/usr/bin/env python3
"""Apply or revert W44-LEAK-PIN-AUDIT's measurement-only rule-off edit.

Usage: rule_off.py apply|revert

`apply` saves the exact bytes of
crates/qcore-c-gateway/src/legacy_refused_operands.rs to
private/rule-off/original.rs and inserts the two lines of
docs/reports/w44-leak-pin-audit/evidence/census-rule-off.patch at the top of
`apply`, so the refused hand-on rule withdraws nothing. `revert` rewrites the
saved bytes and asserts the round trip (the SHA-256 recorded at apply time).
The edit is never committed; a census binary is built between the two.
"""

import hashlib
import pathlib
import sys

ROOT = pathlib.Path(__file__).resolve().parents[4]
SOURCE = ROOT / "crates/qcore-c-gateway/src/legacy_refused_operands.rs"
SAVE = ROOT / "private/rule-off"
ANCHOR = """    diagnostics: &BTreeSet<LegacyDiagnostic>,
) {
    if !read.is_empty() && refused(diagnostics) {"""
EDIT = """    diagnostics: &BTreeSet<LegacyDiagnostic>,
) {
    let _ = (&output, read, diagnostics);
    return;
    if !read.is_empty() && refused(diagnostics) {"""


def main() -> int:
    mode = sys.argv[1]
    SAVE.mkdir(parents=True, exist_ok=True)
    if mode == "apply":
        data = SOURCE.read_bytes()
        text = data.decode()
        if text.count(ANCHOR) != 1:
            raise SystemExit("anchor not found exactly once")
        (SAVE / "original.rs").write_bytes(data)
        (SAVE / "original.sha256").write_text(hashlib.sha256(data).hexdigest() + "\n")
        SOURCE.write_bytes(text.replace(ANCHOR, EDIT).encode())
        print("applied")
    elif mode == "revert":
        data = (SAVE / "original.rs").read_bytes()
        SOURCE.write_bytes(data)
        expected = (SAVE / "original.sha256").read_text().strip()
        actual = hashlib.sha256(SOURCE.read_bytes()).hexdigest()
        if actual != expected:
            raise SystemExit(f"round trip failed: {actual} != {expected}")
        print("reverted", actual)
    else:
        raise SystemExit("usage: rule_off.py apply|revert")
    return 0


if __name__ == "__main__":
    sys.exit(main())
