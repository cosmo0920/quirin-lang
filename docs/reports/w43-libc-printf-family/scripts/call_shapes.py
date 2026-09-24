#!/usr/bin/env python3
"""Classify every printf-family call-site leaf of a baseline by its lexical shape.

Usage: call_shapes.py BASELINE_DIAGNOSTICS(.json|.json.gz) UNIT_TEXT_DIR OUT.json [NAME...]

A FORECAST aid, written and run before any declaration was drafted into the
manifest: it reads no analyzer internals and decides nothing. For each
`call to uncontracted function `NAME`` leaf it resolves the leaf's analysed
line in the unit's `-E -P` text (`w43-libc-contracts/scripts/unit_texts.py`),
finds the call there, joins following lines until its parentheses balance, and
classifies:

* the statement position of the call: `discarded` (`NAME(...);`), `assigned`
  (`x = NAME(...)`), `condition` (inside `if`/`while`/`for`/`switch` parentheses),
  `returned` (`return NAME(...)`), `argument` (inside another call's
  parentheses) or `other`;
* each tail actual (arguments beyond the fixed parameters), lexically:
  `literal` (string, character or number literals, possibly concatenated),
  `name` (one identifier), `pointer-formal` (one identifier that is a
  parameter of the enclosing definition declared with a `*`), `chain`
  (identifiers joined only by `->`/`.`), `call` (contains `NAME (`), `address`
  (contains `&`), `deref-or-subscript` (contains `*` or `[`), `expression`
  (anything else);
* each fixed actual likewise.

The gateway's actual rule (`variadic_nested::tail_arguments_are_data_with_nested`)
needs declared types and the C02-P closure, which this script does not compute;
its classes are therefore an upper and lower bound, not a prediction of a leaf.
"""

import collections
import gzip
import json
import pathlib
import re
import sys

FIXED = {"printf": 1, "fprintf": 2, "snprintf": 3, "sprintf": 2, "vsnprintf": 4, "vprintf": 2}
TOKEN = re.compile(r'"(?:\\.|[^"\\])*"|\'(?:\\.|[^\'\\])*\'|->|[A-Za-z_]\w*|\d[\w.]*|\S')
IDENT = re.compile(r"[A-Za-z_]\w*$")
KEYWORDS = {"if", "while", "for", "switch"}


def load(path):
    opener = gzip.open if str(path).endswith(".gz") else open
    with opener(path, "rt") as handle:
        return json.load(handle)


def split_arguments(tokens):
    """tokens start after the call's `(`; returns (args, consumed) at the matching `)`."""
    depth, current, args = 0, [], []
    for index, token in enumerate(tokens):
        if token in "([{":
            depth += 1
        elif token in ")]}":
            if depth == 0:
                args.append(current)
                return args, index
            depth -= 1
        if token == "," and depth == 0:
            args.append(current)
            current = []
            continue
        current.append(token)
    return None, None


def classify(argument, pointer_formals):
    text = [t for t in argument]
    if not text:
        return "empty"
    if all(t.startswith('"') or t.startswith("'") or t[0].isdigit() for t in text):
        return "literal"
    if any(IDENT.match(t) and i + 1 < len(text) and text[i + 1] == "(" and t != "sizeof" for i, t in enumerate(text)):
        return "call"
    if "&" in text:
        return "address"
    if "*" in text or "[" in text:
        return "deref-or-subscript"
    if len(text) == 1 and IDENT.match(text[0]):
        return "pointer-formal" if text[0] in pointer_formals else "name"
    if all(IDENT.match(t) or t in ("->", ".") for t in text) and ("->" in text or "." in text):
        return "chain"
    return "expression"


def enclosing_pointer_formals(lines, line_index, function):
    """Pointer parameters of `function`'s definition, read from its header."""
    pattern = re.compile(r"(?<![\w])" + re.escape(function) + r"\s*\(")
    for start in range(line_index, -1, -1):
        if pattern.search(lines[start]):
            header = " ".join(lines[start:start + 8])
            match = pattern.search(header)
            tokens = TOKEN.findall(header[match.end():])
            args, _ = split_arguments(tokens)
            formals = set()
            for arg in args or []:
                names = [t for t in arg if IDENT.match(t)]
                if "*" in arg and names:
                    formals.add(names[-1])
            return formals
    return set()


def position(before_tokens):
    """Classify what encloses the call from the tokens preceding it on its statement."""
    depth = 0
    for index in range(len(before_tokens) - 1, -1, -1):
        token = before_tokens[index]
        if token in ")]":
            depth += 1
        elif token in "([":
            if depth == 0:
                prior = before_tokens[index - 1] if index > 0 else ""
                if prior in KEYWORDS:
                    return "condition"
                if IDENT.match(prior):
                    return "argument"
                continue
            depth -= 1
        elif depth == 0 and token in (";", "{", "}"):
            break
    stripped = [t for t in before_tokens[-4:]]
    if stripped[-1:] == ["return"]:
        return "returned"
    if stripped[-1:] == ["="]:
        return "assigned"
    if not stripped or stripped[-1] in (";", "{", "}", ":", ")", "else"):
        return "discarded"
    return "other"


def main() -> int:
    data = load(sys.argv[1])
    texts = pathlib.Path(sys.argv[2])
    out = pathlib.Path(sys.argv[3])
    names = set(sys.argv[4:]) or set(FIXED)
    pairing = json.loads((texts / "pairing.json").read_text())
    rows = []
    for unit in data["units"]:
        entry = pairing.get(f"{unit['library']}/{unit['file']}") or {}
        lines = (texts / entry["text"]).read_text(errors="replace").split("\n") if entry.get("text") else None
        for function in unit["functions"]:
            for leaf in function.get("diagnostics") or []:
                match = re.fullmatch(r"call to uncontracted function `([^`]+)`", leaf["description"])
                if not match or match.group(1) not in names:
                    continue
                name = match.group(1)
                row = {"unit": f"{unit['library']}/{unit['file']}", "function": function["function"],
                       "status": function["status"], "callee": name, "line": leaf["line"]}
                if lines is None:
                    row["shape"] = "unpaired"
                    rows.append(row)
                    continue
                index = leaf["line"] - 1
                window = " ".join(lines[index:index + 12])
                call = re.search(r"(?<![\w])" + re.escape(name) + r"\s*\(", window)
                if not call:
                    row["shape"] = "call-not-on-line"
                    rows.append(row)
                    continue
                before = TOKEN.findall(" ".join(lines[max(0, index - 3):index]) + " " + window[:call.start()])
                args, _ = split_arguments(TOKEN.findall(window[call.end():]))
                if args is None:
                    row["shape"] = "unbalanced"
                    rows.append(row)
                    continue
                formals = enclosing_pointer_formals(lines, index, function["function"])
                fixed = FIXED[name]
                row["position"] = position(before)
                row["fixed"] = [classify(a, formals) for a in args[:fixed]]
                row["tail"] = [classify(a, formals) for a in args[fixed:]]
                row["text"] = " ".join(" ".join(a) for a in args)[:200]
                row["shape"] = "read"
                rows.append(row)
    summary = collections.Counter()
    tail_worst = collections.Counter()
    order = ["pointer-formal", "call", "address", "deref-or-subscript", "expression", "chain", "name", "literal", "empty"]
    for row in rows:
        if row["shape"] != "read":
            summary[(row["callee"], row["shape"])] += 1
            continue
        summary[(row["callee"], row["position"])] += 1
        worst = next((c for c in order if c in row["tail"] + row["fixed"]), "none")
        tail_worst[(row["callee"], worst)] += 1
    record = {
        "leaves": len(rows),
        "by_callee_and_position": {f"{a} {b}": n for (a, b), n in sorted(summary.items())},
        "by_callee_and_worst_actual": {f"{a} {b}": n for (a, b), n in sorted(tail_worst.items())},
        "rows": rows,
    }
    out.write_text(json.dumps(record, indent=1) + "\n")
    print(json.dumps({k: v for k, v in record.items() if k != "rows"}, indent=1))
    return 0


if __name__ == "__main__":
    sys.exit(main())
