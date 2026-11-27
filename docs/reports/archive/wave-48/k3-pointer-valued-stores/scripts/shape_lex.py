#!/usr/bin/env python3
"""Card K3: a lexical first cut of every 11-24 `pointer-valued-store` leaf by the
shape of the statement at its line (the analysed `-E -P` text).

usage: shape_lex.py RECENSUS_JSON COMPILE_COMMANDS UNIT_DIR OUT_JSON

Lexical only: it reads the text of the leaf's line (and the next lines up to the
first `;` when the statement continues), never the AST, so a class is a census
heuristic, not a decision. Shapes:

* `self-cond-literal`  `P = P ? P : "lit"` (nanopb's PB_SET_ERROR / PB_RETURN_ERROR);
* `macro-list`         a `do { ... } while (0)` list or queue macro expansion;
* `call`               the value is a call (`P = f(...)`, under casts);
* `same-place-arith`   `P = P + E` / `P = (T *) P + E` (the destination's own value);
* `arith`              other pointer arithmetic;
* `name`               a bare name (or a cast name);
* `member-read`        a member or element read;
* `address`            `&...`;
* `literal`            a string literal or null;
* `other`.
"""
import collections
import json
import pathlib
import re
import sys

HERE = pathlib.Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
from leaf_lines import preprocess  # noqa: E402

CAST = r"(?:\(\s*(?:const\s+)?[A-Za-z_][\w\s]*\*+\s*\)\s*)*"


def statement(lines, index):
    text = lines[index]
    j = index
    while ";" not in text and j + 1 < len(lines) and j - index < 6:
        j += 1
        text += " " + lines[j].strip()
    return text.strip()


def split_assignment(text):
    depth = 0
    for i, ch in enumerate(text):
        if ch in "([":
            depth += 1
        elif ch in ")]":
            depth -= 1
        elif ch == "=" and depth == 0 and text[i - 1] not in "=!<>+-*/%&|^" and text[i + 1] != "=":
            return text[:i].strip(), text[i + 1:].split(";")[0].strip()
    return None, None


def classify(text):
    if text.startswith("do {") or "while ( 0)" in text or "while (0)" in text:
        return "macro-list"
    body = text
    if body.startswith("return "):
        body = body[len("return "):]
    body = body.lstrip("(")
    lhs, rhs = split_assignment(body)
    if lhs is None:
        if text.startswith(("if", "while", "for", "switch")):
            return "condition-other"
        return "other"
    lhs = lhs.lstrip("(").strip()
    bare = re.sub(r"^" + CAST, "", rhs).strip()
    if re.match(re.escape(lhs) + r"\s*\)?\s*\?\s*", bare) and '"' in bare:
        return "self-cond-literal"
    if bare.startswith('"') or re.match(r"^\(\(void\s*\*\)\s*0\)$|^0$|^NULL$", bare):
        return "literal"
    if re.match(r"^[A-Za-z_]\w*\s*\(", bare):
        return "call"
    if re.match(r"^\(?" + CAST + re.escape(lhs) + r"\)?\s*[-+]", bare):
        return "same-place-arith"
    if re.search(r"[^-][+-][^>+-]", bare) and not bare.startswith("&"):
        return "arith"
    if bare.startswith("&"):
        return "address"
    if re.match(r"^[A-Za-z_]\w*$", bare):
        return "name"
    if re.match(r"^[A-Za-z_][\w\s\.\->\[\]]*$", bare):
        return "member-read"
    return "other"


def main():
    recensus = json.load(open(sys.argv[1]))
    database = json.load(open(sys.argv[2]))
    unit_dir = pathlib.Path(sys.argv[3])
    unit_dir.mkdir(parents=True, exist_ok=True)
    freed = set(recensus["freed_alone"])
    texts = {}
    shapes = collections.Counter()
    shapes_unknown_rows = collections.defaultdict(set)
    shapes_freed = collections.defaultdict(set)
    samples = collections.defaultdict(list)
    rows = []
    for leaf in recensus["tagged_leaves"]:
        path = unit_dir / (leaf["file"].replace("/", "__") + ".i")
        if leaf["file"] not in texts:
            if not path.exists():
                preprocess(database, leaf["file"], path)
            texts[leaf["file"]] = path.read_text().splitlines() if path.exists() else []
        lines = texts[leaf["file"]]
        text = statement(lines, leaf["line"] - 1) if 0 < leaf["line"] <= len(lines) else ""
        shape = classify(text)
        shapes[shape] += 1
        if leaf["status"] == "Unknown":
            shapes_unknown_rows[shape].add(leaf["key"])
        if leaf["key"] in freed:
            shapes_freed[shape].add(leaf["key"])
        if len(samples[shape]) < 8:
            samples[shape].append(f"{leaf['file']}:{leaf['line']} {leaf['function']}: {text[:140]}")
        rows.append({**leaf, "shape": shape, "text": text[:240]})
    out = {
        "leaves_by_shape": dict(shapes.most_common()),
        "unknown_functions_by_shape": {k: len(v) for k, v in shapes_unknown_rows.items()},
        "freed_alone_functions_by_shape": {k: sorted(v) for k, v in shapes_freed.items()},
        "samples": samples,
        "leaves": rows,
    }
    pathlib.Path(sys.argv[4]).write_text(json.dumps(out, indent=1, sort_keys=True) + "\n")
    print(json.dumps({"leaves": out["leaves_by_shape"], "functions": out["unknown_functions_by_shape"],
                      "freed": {k: len(v) for k, v in shapes_freed.items()}}, indent=1))


if __name__ == "__main__":
    main()
