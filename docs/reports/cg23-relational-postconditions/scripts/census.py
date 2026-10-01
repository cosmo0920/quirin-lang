#!/usr/bin/env python3
"""census.py RUN_DIR [--json OUT] -- CG23-RELATIONAL-POSTCONDITIONS's
prediction census, read from one BEFORE corpus run and the units' sources.

The card's corpus reach is the exit-specific route (rule 11r): a callee the
unit defines as `int NAME(...)` whose every `return` returns an integer
literal, at least two distinct values, and whose ordinary summary failed (so
its call sites read `call to uncontracted function `NAME``). A call site
moves only when its value is tested in an `if` condition where it stands, or
stored by the statement right before such a test.

The source is read raw (not preprocessed), so a `return MACRO;` whose macro
is spelled in capitals is counted as a possible literal: an upper bound. A
function is "freed alone" (upper bound) when every non-Safe leaf it has is
such a tested call site. No declared module uses the new clause, so the
declared vocabulary moves nothing.
"""

import argparse
import collections
import json
import pathlib
import re

LITERAL = re.compile(r"^\(?\s*-?\s*(0|[1-9][0-9]*)\s*\)?$")
MACRO = re.compile(r"^\(?\s*-?\s*[A-Z][A-Z0-9_]*\s*\)?$")
UNCONTRACTED = re.compile(r"call to uncontracted function `([A-Za-z_][A-Za-z0-9_]*)`")


def strip_comments(text):
    text = re.sub(r"/\*.*?\*/", lambda m: "\n" * m.group(0).count("\n"), text, flags=re.S)
    return re.sub(r"//[^\n]*", "", text)


def definition(text, name):
    """(specifiers, body) of `name`'s definition, or None."""
    for match in re.finditer(r"\b" + re.escape(name) + r"\s*\(", text):
        start = match.start()
        depth, index = 0, match.end() - 1
        while index < len(text):
            if text[index] == "(":
                depth += 1
            elif text[index] == ")":
                depth -= 1
                if depth == 0:
                    break
            index += 1
        rest = text[index + 1:].lstrip()
        if not rest.startswith("{"):
            continue
        open_at = text.index("{", index + 1)
        depth, close = 0, open_at
        while close < len(text):
            if text[close] == "{":
                depth += 1
            elif text[close] == "}":
                depth -= 1
                if depth == 0:
                    break
            close += 1
        head_start = max(text.rfind(";", 0, start), text.rfind("}", 0, start), text.rfind("#", 0, start))
        head = text[head_start + 1:start]
        head = head[head.rfind("\n", 0, len(head)) + 1:] if "\n" in head.strip() else head
        return head.split(), text[open_at:close + 1]
    return None


def literal_helper(text, name):
    found = definition(text, name)
    if not found:
        return None
    specifiers, body = found
    if specifiers.count("int") != 1 or any(
        word not in ("int", "static", "inline", "extern", "__inline", "__inline__") for word in specifiers
    ):
        return None
    values = set()
    macro = False
    for match in re.finditer(r"\breturn\b([^;]*);", body):
        expression = match.group(1).strip()
        if LITERAL.match(expression):
            values.add(expression.replace(" ", "").strip("()"))
        elif MACRO.match(expression):
            values.add(expression.replace(" ", ""))
            macro = True
        else:
            return None
    if len(values) < 2:
        return None
    return {"values": sorted(values), "macro": macro}


def tested(lines, line, name):
    """Whether the call on 1-based `line` is tested where it stands, or stored
    and tested by the next statement."""
    text = lines[line - 1] if 0 < line <= len(lines) else ""
    if re.search(r"\bif\s*\(.*\b" + re.escape(name) + r"\s*\(", text):
        return True
    stored = re.search(r"\b([A-Za-z_]\w*)\s*=\s*" + re.escape(name) + r"\s*\(", text)
    if stored:
        following = " ".join(lines[line:line + 3])
        return re.search(r"^\s*if\s*\(.*\b" + re.escape(stored.group(1)) + r"\b", following) is not None
    return False


SOURCE = pathlib.Path(
    "/home/cosmo/Documents/GitHub/qcore-c/build/cycle19/c-gateway-wave-pin/fluent-bit-clang-source"
)


def source_index():
    index = collections.defaultdict(list)
    for path in SOURCE.rglob("*.c"):
        index[path.name].append(path)
    return index


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("run", type=pathlib.Path)
    parser.add_argument("--json", type=pathlib.Path)
    args = parser.parse_args()
    data = json.loads((args.run / "resource-diagnostics.json").read_text())
    sources = {}
    helpers = {}
    paths = source_index()
    leaves = collections.Counter()
    touched = {}
    for unit in data["units"]:
        candidates = paths.get(unit["file"], [])
        if len(candidates) != 1:
            continue
        path = candidates[0]
        for function in unit.get("functions") or []:
            if function.get("status") == "Safe":
                continue
            diagnostics = [d for d in function.get("diagnostics", []) if d.get("category") != "note"]
            hits = []
            for diagnostic in diagnostics:
                match = UNCONTRACTED.search(diagnostic.get("description") or "")
                if not match:
                    continue
                callee = match.group(1)
                if path not in sources:
                    try:
                        sources[path] = strip_comments(path.read_text(errors="replace"))
                    except OSError:
                        sources[path] = None
                text = sources[path]
                if text is None:
                    continue
                key = (str(path), callee)
                if key not in helpers:
                    helpers[key] = literal_helper(text, callee)
                if helpers[key] is None:
                    continue
                origin = (diagnostic.get("origin") or ":0").rsplit(":", 1)
                site_tested = origin[0] == path.name and tested(
                    text.split("\n"), int(origin[1] or 0), callee
                )
                hits.append((diagnostic, callee, site_tested))
            if not hits:
                continue
            for _, callee, site_tested in hits:
                leaves["tested" if site_tested else "untested"] += 1
            freed = all(
                any(d is hit[0] and hit[2] for hit in hits) for d in diagnostics
            )
            touched[f"{path.name}::{function['function']}"] = {
                "status": function.get("status"),
                "helpers": sorted({callee for _, callee, _ in hits}),
                "tested_sites": sum(1 for hit in hits if hit[2]),
                "untested_sites": sum(1 for hit in hits if not hit[2]),
                "other_leaves": len(diagnostics) - len(hits),
                "freed_alone_upper_bound": freed,
            }
    record = {
        "candidate_helpers": sorted(
            f"{pathlib.Path(path).name}::{name} {info['values']}{' (macro)' if info['macro'] else ''}"
            for (path, name), info in helpers.items()
            if info
        ),
        "leaves": dict(leaves),
        "touched_functions": len(touched),
        "freed_alone_upper_bound": sorted(k for k, v in touched.items() if v["freed_alone_upper_bound"]),
        "touched": touched,
    }
    text = json.dumps(record, indent=1, sort_keys=True)
    if args.json:
        args.json.write_text(text + "\n")
    print(json.dumps({k: record[k] for k in ("candidate_helpers", "leaves", "touched_functions", "freed_alone_upper_bound")}, indent=1))


if __name__ == "__main__":
    main()
