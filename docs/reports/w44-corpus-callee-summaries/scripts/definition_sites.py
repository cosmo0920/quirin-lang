#!/usr/bin/env python3
"""Map every analysed unit to the definitions its translation unit carries.

Usage: definition_sites.py CAPTURE_DIR DIAGNOSTICS.json[.gz] OUT.json

CAPTURE_DIR is a `capture_run.sh` output: `pp/*.i` holds every line-marked
(`-p0`) and `-P` (`-p1`) preprocessing the audit asked Clang for, the unit text
arriving on stdin. A line-marked capture is joined to a diagnostics unit by its
`<stdin>` definitions: the capture whose main-source definition set contains
the unit's analysed function rows and is smallest wins; ties and misses are
reported, never guessed.

For each joined unit the script records every file-scope function definition
in the translation unit with the file that supplied it (`<stdin>` for the main
source), whether that file is a system header (line-marker flag 3), and
whether the definition's declaration specifiers carry `static` (internal
linkage) and/or `inline`. It reads text only; it runs nothing.
"""

import collections
import gzip
import json
import pathlib
import re
import sys

MARKER = re.compile(r'^# (\d+) "((?:[^"\\]|\\.)*)"((?: \d)*)\s*$')
IDENT = re.compile(r"[A-Za-z_][A-Za-z0-9_]*")
KEYWORDS = {"if", "while", "for", "switch", "return", "sizeof", "_Generic", "__attribute__",
            "__typeof__", "typeof", "__asm__", "asm", "defined", "__extension__"}


def tokens(text):
    """Yield (file, system, token) for the line-marked text, dropping strings."""
    current, system = None, False
    for line in text.split("\n"):
        marker = MARKER.match(line)
        if marker:
            current = marker.group(2)
            system = " 3" in marker.group(3)
            continue
        if line.startswith("#"):
            continue
        position = 0
        while position < len(line):
            char = line[position]
            if char in "\"'":
                quote = char
                position += 1
                while position < len(line) and line[position] != quote:
                    position += 2 if line[position] == "\\" else 1
                position += 1
                yield current, system, "__lit__"
                continue
            if char.isspace():
                position += 1
                continue
            match = IDENT.match(line, position)
            if match:
                yield current, system, match.group(0)
                position = match.end()
                continue
            yield current, system, char
            position += 1


def definitions(text):
    """File-scope definitions: NAME ( ... ) {  at brace depth 0."""
    stream = list(tokens(text))
    found = {}
    depth = 0
    last_boundary = -1
    index = 0
    while index < len(stream):
        _, _, tok = stream[index]
        if tok == "{":
            depth += 1
        elif tok == "}":
            depth = max(0, depth - 1)
            if depth == 0:
                last_boundary = index
        elif tok == ";" and depth == 0:
            last_boundary = index
        elif depth == 0 and IDENT.fullmatch(tok) and tok not in KEYWORDS \
                and index + 1 < len(stream) and stream[index + 1][2] == "(":
            # find matching paren
            level = 0
            close = None
            for probe in range(index + 1, len(stream)):
                t = stream[probe][2]
                if t == "(":
                    level += 1
                elif t == ")":
                    level -= 1
                    if level == 0:
                        close = probe
                        break
            if close is not None and close + 1 < len(stream) and stream[close + 1][2] == "{":
                specifiers = [t for (_, _, t) in stream[last_boundary + 1:index]]
                file, system, _ = stream[index]
                found.setdefault(tok, {
                    "file": file,
                    "system": system,
                    "static": "static" in specifiers,
                    "inline": any(s in ("inline", "__inline", "__inline__") for s in specifiers),
                    "extern": "extern" in specifiers,
                })
                index = close + 1
                continue
        index += 1
    return found


def main() -> int:
    capture, diagnostics, out = sys.argv[1:4]
    opener = gzip.open if diagnostics.endswith(".gz") else open
    data = json.load(opener(diagnostics))
    captures = []
    for path in sorted(pathlib.Path(capture, "pp").glob("*-p0.i")):
        defs = definitions(path.read_text(errors="replace"))
        main_defs = {n for n, d in defs.items() if d["file"] == "<stdin>"}
        captures.append((path.name, defs, main_defs))
    units = {}
    problems = []
    for unit in data["units"]:
        key = unit["library"] + "/" + unit["file"]
        want = {f["function"] for f in unit["functions"]}
        matches = [c for c in captures if want <= c[2] and want]
        if not matches:
            problems.append({"unit": key, "problem": "no capture"})
            continue
        matches.sort(key=lambda c: len(c[2]))
        best = [c for c in matches if len(c[2]) == len(matches[0][2])]
        bodies = {json.dumps(c[1], sort_keys=True) for c in best}
        if len(bodies) != 1:
            problems.append({"unit": key, "problem": "ambiguous", "captures": [c[0] for c in best]})
            continue
        units[key] = {"capture": best[0][0], "definitions": best[0][1]}
    record = {"captures": len(captures), "units": units, "problems": problems}
    pathlib.Path(out).write_text(json.dumps(record, indent=1, sort_keys=True) + "\n")
    print(json.dumps({"captures": len(captures), "joined": len(units), "problems": problems[:20]}, indent=1))
    return 0


if __name__ == "__main__":
    sys.exit(main())
