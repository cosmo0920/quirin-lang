#!/usr/bin/env python3
"""Classify the source shapes behind two refusal classes of an audit report.

Usage: shape_census.py REPORT SOURCE_ROOT OUT_JSON [--monkey-root DIR]

REPORT is a resource-diagnostics report (plain or gzipped) of the pinned
corpus or of a `quirin-audit current` slice. For every leaf of

  * `braced initializer, expression, or declaration is not modeled`, and
  * `unmodeled call, aggregate or indirect access in for-loop condition`,

the original line its `origin` names is read from SOURCE_ROOT and classified
by the macro or construct spelled there. Leaves whose origin is missing or
unreadable are counted as `unreadable`. The classification is lexical source
reading of the original (unpreprocessed) line; it is a measurement aid for
choosing which shape to admit, not a rule and not a verdict.
"""

import collections
import gzip
import json
import pathlib
import re
import sys

BRACED = "braced initializer, expression, or declaration is not modeled"
FOR_CONDITION = "unmodeled call, aggregate or indirect access in for-loop condition"

ENTRY = re.compile(r"\b(mk_list_entry|cfl_list_entry|container_of|mk_list_entry_first|mk_list_entry_last|cfl_list_entry_first|cfl_list_entry_last|flb_list_entry)\b")
FOREACH = re.compile(r"\b(mk_list_foreach_safe_r|mk_list_foreach_safe|mk_list_foreach_r|mk_list_foreach|cfl_list_foreach_safe|cfl_list_foreach|TAILQ_FOREACH_SAFE|TAILQ_FOREACH|LIST_FOREACH|SLIST_FOREACH|[A-Za-z_]*_foreach[A-Za-z_]*|[A-Za-z_]*FOREACH[A-Za-z_]*)\b")


def load(path):
    opener = gzip.open if str(path).endswith(".gz") else open
    with opener(path, "rt") as handle:
        return json.load(handle)


def classify_braced(line):
    match = ENTRY.search(line)
    if match:
        return "entry:" + match.group(1)
    stripped = line.strip()
    if re.search(r"=\s*\{", stripped):
        return "brace-initializer"
    if re.search(r"\(\s*(struct|union)?\s*[A-Za-z_][A-Za-z_0-9]*\s*\*?\s*\)\s*\{", stripped):
        return "compound-literal"
    if re.search(r"\b[A-Z][A-Z0-9_]{2,}\s*\(", stripped):
        return "other-macro:" + re.search(r"\b([A-Z][A-Z0-9_]{2,})\s*\(", stripped).group(1)
    if re.search(r"\b[a-z_][a-z0-9_]*\s*\(", stripped):
        return "other-call-line"
    return "other"


def classify_for(line):
    match = FOREACH.search(line)
    if match:
        return "loop:" + match.group(1)
    if re.search(r"\bfor\s*\(", line):
        return "plain-for"
    return "other"


def main():
    report = load(sys.argv[1])
    roots = [pathlib.Path(sys.argv[2])]
    if "--monkey-root" in sys.argv:
        roots.append(pathlib.Path(sys.argv[sys.argv.index("--monkey-root") + 1]))
    out = pathlib.Path(sys.argv[3])
    cache = {}
    by_name = collections.defaultdict(list)
    for root in roots:
        for path in root.rglob("*"):
            if path.suffix in (".c", ".h") and path.is_file():
                by_name[path.name].append(path)

    def read(path, number):
        if path not in cache:
            cache[path] = path.read_text(errors="replace").splitlines()
        lines = cache[path]
        index = int(number) - 1
        return lines[index] if 0 <= index < len(lines) else None

    def source_line(origin):
        # A pinned-corpus origin spells the file by basename; a slice origin
        # by its path under the checkout. A basename with several candidates
        # prefers the first whose line spells a recognised shape.
        if not origin or ":" not in origin:
            return None
        file, number = origin.rsplit(":", 1)
        if not number.isdigit():
            return None
        for root in roots:
            path = root / file
            if path.exists():
                return read(path, number)
        candidates = [read(path, number) for path in by_name.get(pathlib.Path(file).name, [])]
        candidates = [line for line in candidates if line is not None]
        for line in candidates:
            if ENTRY.search(line) or FOREACH.search(line):
                return line
        return candidates[0] if candidates else None

    counts = {BRACED: collections.Counter(), FOR_CONDITION: collections.Counter()}
    functions = {BRACED: collections.defaultdict(set), FOR_CONDITION: collections.defaultdict(set)}
    sole = {BRACED: collections.Counter(), FOR_CONDITION: collections.Counter()}
    for unit in report["units"]:
        for function in unit["functions"]:
            classes = set()
            for diagnostic in function["diagnostics"]:
                description = diagnostic["description"]
                if description not in counts:
                    continue
                line = source_line(diagnostic.get("origin"))
                if line is None:
                    shape = "unreadable"
                elif description == BRACED:
                    shape = classify_braced(line)
                else:
                    shape = classify_for(line)
                counts[description][shape] += 1
                functions[description][shape].add((unit["file"], function["function"]))
                classes.add((description, shape))
            for description, shape in classes:
                pass
    result = {
        "report": sys.argv[1],
        "source_roots": [str(root) for root in roots],
        "leaves": {key: dict(value.most_common()) for key, value in counts.items()},
        "functions": {
            key: {shape: len(names) for shape, names in sorted(value.items(), key=lambda item: -len(item[1]))}
            for key, value in functions.items()
        },
    }
    out.write_text(json.dumps(result, indent=1) + "\n")
    print(json.dumps(result, indent=1)[:4000])


if __name__ == "__main__":
    main()
