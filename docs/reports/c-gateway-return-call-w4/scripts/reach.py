#!/usr/bin/env python3
"""Card C-GATEWAY-RETURN-CALL-W4: re-derive the sub-shape B reach.

Wave 33's own reach log was produced by a classifier that read the **original**
source line while refusals are recorded at the **preprocessed** line, so
macro-expanded accesses were misclassified. That defect is not inherited here:
card GATEWAY-C-LOCATION (wave 27) made every diagnostic carry `origin`, the
derived original `file:line`, so this census reads the line the diagnostic
itself names rather than guessing one.

The classifier is checked rather than trusted: every leaf the AFTER run actually
withdrew must be classified `call`, and the script fails if any is not.

Usage: python3 measure/c-gateway-return-call-w4/reach.py
"""
import collections
import json
import pathlib
import re
import sys

M = pathlib.Path(__file__).resolve().parent
PIN = pathlib.Path(
    "/home/cosmo/Documents/GitHub/qcore-c/build/cycle19/c-gateway-wave-pin/fluent-bit-clang-source"
)
RET = "unmodeled call, aggregate or indirect access in return expression"

_sources = {}
_index = None


def build_index():
    global _index
    _index = collections.defaultdict(list)
    for path in PIN.rglob("*.c"):
        _index[path.name].append(path)
    for path in PIN.rglob("*.h"):
        _index[path.name].append(path)


def source_line(origin):
    """The text of the line `origin` names, or None."""
    if not origin or ":" not in origin:
        return None
    name, _, number = origin.rpartition(":")
    if not number.isdigit():
        return None
    base = pathlib.Path(name).name
    if _index is None:
        build_index()
    candidates = _index.get(base) or []
    # Prefer a candidate whose tail matches the spelled path.
    wanted = name.lstrip("./")
    best = None
    for path in candidates:
        if str(path).endswith(wanted):
            best = path
            break
    if best is None and len(candidates) == 1:
        best = candidates[0]
    if best is None:
        return None
    if best not in _sources:
        _sources[best] = best.read_text(errors="replace").splitlines()
    lines = _sources[best]
    i = int(number) - 1
    return lines[i] if 0 <= i < len(lines) else None


IDENT = r"[A-Za-z_][A-Za-z_0-9]*"


def classify(text):
    """The shape of the returned expression on this source line."""
    if text is None:
        return "line-not-resolved"
    stripped = text.strip()
    # The `return` need not begin the line: a one-line function body puts it
    # after the brace. Take the first `return` and the first `;` after it.
    m = re.search(r"\breturn\b\s*([^;]*);", stripped)
    if not m:
        return "not-a-complete-return-on-this-line"
    expr = m.group(1).strip()
    if not expr:
        return "bare-return"
    # `NAME ( ... )` spanning the whole expression.
    m2 = re.match(rf"^({IDENT})\s*\((.*)\)$", expr, re.S)
    if m2:
        depth = 0
        body = m2.group(2)
        balanced = True
        for ch in body:
            if ch == "(":
                depth += 1
            elif ch == ")":
                depth -= 1
                if depth < 0:
                    balanced = False
                    break
        if balanced and depth == 0:
            return "call"
    if expr.startswith("("):
        return "cast-or-parenthesized"
    if "?" in expr:
        return "conditional"
    if "->" in expr or "." in expr:
        return "field-chain-or-mixed"
    if "[" in expr:
        return "subscript"
    if expr.startswith("&"):
        return "address-of"
    if expr.startswith("*"):
        return "dereference"
    if re.fullmatch(IDENT, expr) or re.fullmatch(r"-?\d+", expr):
        return "plain-name-or-literal"
    return "other"


def callee(text):
    m = re.search(rf"\breturn\b\s*({IDENT})\s*\(", (text or "").strip())
    return m.group(1) if m else None


def leaves(stage):
    doc = json.loads((M / "runs" / stage / "resource-diagnostics.json").read_text())
    out = {}
    for u in doc["units"]:
        for f in u.get("functions") or []:
            for d in f.get("diagnostics") or []:
                out[(u["file"], f["function"], d["line"], d["category"], d["description"])] = d.get(
                    "origin"
                )
    return out


def main():
    before, after = leaves("before"), leaves("after")
    ret = {k: v for k, v in before.items() if k[4] == RET}
    withdrawn = {k for k in set(before) - set(after) if k[4] == RET}
    print(f"return-expression leaves in BEFORE: {len(ret)}")
    print(f"withdrawn by this card:             {len(withdrawn)}\n")

    shapes = collections.Counter()
    call_leaves = []
    for key, origin in ret.items():
        text = source_line(origin)
        shape = classify(text)
        shapes[shape] += 1
        if shape == "call":
            call_leaves.append((key, callee(text)))

    print("== shape of the returned expression, over every remaining return leaf ==")
    total = sum(shapes.values())
    for shape, n in shapes.most_common():
        print(f"  {n:>6}  {100 * n / total:>5.1f}%  {shape}")

    print(f"\n== the sub-shape B population re-derived ==\n  `return CALL(...)` leaves: {len(call_leaves)}")
    names = collections.Counter(c for _, c in call_leaves if c)
    print(f"  distinct callees:          {len(names)}")

    # The classifier is checked against ground truth, not trusted.
    misread = [k for k in withdrawn if classify(source_line(ret[k])) != "call"]
    print(f"\n== classifier check ==\n  withdrawn leaves classified `call`: {len(withdrawn) - len(misread)}/{len(withdrawn)}")
    for k in misread:
        print(f"    MISREAD {k[0]} :: {k[1]} :: {ret[k]} -> {classify(source_line(ret[k]))!r}")
        print(f"            line: {source_line(ret[k])!r}")

    print(f"\n== yield over the re-derived population ==")
    if call_leaves:
        print(
            f"  admitted: {len(withdrawn)} of {len(call_leaves)} "
            f"`return CALL(...)` leaves ({100 * len(withdrawn) / len(call_leaves):.1f}%)"
        )
    return 1 if misread else 0


if __name__ == "__main__":
    sys.exit(main())
