#!/usr/bin/env python3
"""Census of glibc header-inline function bodies the analysed units call.

Usage: header_inline_census.py CAPTURED_PP_DIR OUT.json

CAPTURED_PP_DIR holds the line-marked `clang -E` outputs a transparent Clang
wrapper saved during a private run of the pinned audit
(`../../w43-libc-result-alias/scripts/clang-tee.sh`). Identical texts are read
once.

For every unit the scan tracks the current file through the line markers:

* a **header-inline definition** is a file-scope function *definition* (its
  parameter list closes before `{`) spelled in a system header (marker flag
  `3`) under `/usr/include/`, whose specifier prefix carries `static` or
  `extern` together with an inline specifier (`__inline`, `__inline__`,
  `inline`). Its body is recorded token by token, whitespace-normalised, with
  its header path and its full signature text;
* a **main-source call** is `NAME (` at brace depth greater than zero while
  the current file is the unit's primary file (the first marker's file), for
  every recorded name. Macro expansions at a main-source line are main-source
  text, which is exactly where the analyzer reads a call.

The output lists, per name: the headers and distinct (signature, body) pairs
seen across units, the number of units defining it, and the number of units
and main-source call sites calling it. A name whose (signature, body) pair is
not unique across units is flagged, because a single trusted declaration then
stands for several bodies.
"""

import collections
import hashlib
import json
import pathlib
import re
import sys

MARKER = re.compile(r'^# (\d+) "((?:[^"\\]|\\.)*)"((?: \d)*)\s*$')
TOKEN = re.compile(r'[A-Za-z_][A-Za-z0-9_]*|0[xX][0-9a-fA-F]+[uUlL]*|\d+[uUlL]*|"(?:[^"\\]|\\.)*"|\'(?:[^\'\\]|\\.)*\'|->|\+\+|--|<<|>>|&&|\|\||[<>=!]=|\S')
INLINE = {"__inline", "__inline__", "inline"}
LINKAGE = {"static", "extern"}


def tokens_with_files(text):
    """Yield (token, file, system) over the whole unit."""
    current = None
    system = False
    for line in text.split("\n"):
        marker = MARKER.match(line)
        if marker:
            current = marker.group(2)
            system = " 3" in marker.group(3)
            continue
        if line.startswith("#"):
            continue
        for token in TOKEN.findall(line):
            yield token, current, system


def scan(text):
    stream = list(tokens_with_files(text))
    first = next((MARKER.match(line) for line in text.split("\n", 50)[:50] if MARKER.match(line)), None)
    primary = first.group(2) if first else None
    definitions = []
    calls = collections.Counter()
    depth = 0
    index = 0
    length = len(stream)
    while index < length:
        token, current, system = stream[index]
        if token == "{":
            depth += 1
        elif token == "}":
            depth = max(0, depth - 1)
        elif depth == 0 and system and current and current.startswith("/usr/include/") and index + 1 < length and stream[index + 1][0] == "(" and re.match(r"[A-Za-z_]", token):
            # Parameter list, then `{` directly (after optional attributes is not glibc's shape).
            close = matching(stream, index + 1)
            if close is not None and close + 1 < length and stream[close + 1][0] == "{":
                start = index
                while start > 0 and re.match(r"[A-Za-z_]", stream[start - 1][0]) and stream[start - 1][0] not in {"return"}:
                    start -= 1
                prefix = [t for t, _, _ in stream[start:index]]
                if INLINE & set(prefix) and LINKAGE & set(prefix):
                    body_end = matching(stream, close + 1)
                    if body_end is not None:
                        signature = " ".join(t for t, _, _ in stream[start : close + 1])
                        body = " ".join(t for t, _, _ in stream[close + 1 : body_end + 1])
                        definitions.append({"name": token, "header": current, "signature": signature, "body": body})
                        index = body_end + 1
                        continue
        elif depth > 0 and current == primary and index + 1 < length and stream[index + 1][0] == "(":
            calls[token] += 1
        index += 1
    return primary, definitions, calls


def matching(stream, open_index):
    opener = stream[open_index][0]
    closer = {"(": ")", "{": "}", "[": "]"}[opener]
    depth = 0
    for index in range(open_index, len(stream)):
        token = stream[index][0]
        if token == opener:
            depth += 1
        elif token == closer:
            depth -= 1
            if depth == 0:
                return index
    return None


def main() -> int:
    source = pathlib.Path(sys.argv[1])
    out = pathlib.Path(sys.argv[2])
    seen = set()
    by_name = collections.defaultdict(lambda: {"headers": set(), "shapes": collections.Counter(), "defining_units": 0, "calling_units": 0, "call_sites": 0})
    units = 0
    for path in sorted(source.glob("*.i")):
        text = path.read_text(errors="replace")
        digest = hashlib.sha256(text.encode()).hexdigest()
        if digest in seen:
            continue
        seen.add(digest)
        units += 1
        primary, definitions, calls = scan(text)
        names = set()
        for definition in definitions:
            row = by_name[definition["name"]]
            row["headers"].add(definition["header"])
            row["shapes"][(definition["signature"], definition["body"])] += 1
            names.add(definition["name"])
        for name in names:
            by_name[name]["defining_units"] += 1
            if calls[name]:
                by_name[name]["calling_units"] += 1
                by_name[name]["call_sites"] += calls[name]
    result = {"distinct_units": units, "names": {}}
    for name, row in sorted(by_name.items()):
        result["names"][name] = {
            "headers": sorted(row["headers"]),
            "defining_units": row["defining_units"],
            "calling_units": row["calling_units"],
            "main_source_call_sites": row["call_sites"],
            "shapes": [{"signature": s, "body": b, "units": n} for (s, b), n in row["shapes"].most_common()],
            "shape_unique": len(row["shapes"]) == 1,
        }
    out.write_text(json.dumps(result, indent=1, sort_keys=True) + "\n")
    for name, row in result["names"].items():
        if row["calling_units"]:
            print(name, row["calling_units"], "units", row["main_source_call_sites"], "sites", "unique" if row["shape_unique"] else "SEVERAL", row["headers"])
    return 0


if __name__ == "__main__":
    sys.exit(main())
