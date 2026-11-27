#!/usr/bin/env python3
"""origins.py REPORT OUT.json -- card REFCOUNT-REFERENCE-RETURNING-DECLARATIONS,
task 1: where the reference a `reference-drop-unheld` drop releases came from.

For every leaf of REPORT reading ``reference-drop-unheld`` whose dropped
operand is a plain name (the source line at the leaf's `origin`, in the
pinned 66910c10 tree, read as REFCOUNT-RESOURCE-KIND's refusals.py reads it),
the name's origin is read lexically from the calling function's own source
text, from its definition to the leaf's line:

* ``formal`` -- the name is a parameter of the function (the caller's
  reference: a transfer, or a drop the function does not own);
* ``call:F`` -- the last assignment ``name = F(`` (or ``T *name = F(``)
  before the leaf names callee F (a reference-returning call when F hands its
  caller a reference);
* ``other:TEXT`` -- any other last assignment (a member read, a list
  iteration, a cast), with its right-hand side;
* ``unassigned`` -- none found.

Per origin: leaves, functions carrying one, and the functions whose every
leaf is such a drop of that origin (cleared alone if the origin were
declared). Per function: every leaf family it carries (`classes`), so a
reader can see which functions a declaration would fully clear and which it
would only re-attribute. Lexical and approximate by design: AFTER is the
arbiter.
"""
import collections
import json
import pathlib
import re
import sys

ROOT = pathlib.Path("/home/cosmo/Documents/GitHub/qcore-c/build/cycle19/c-gateway-wave-pin/fluent-bit-clang-source")
SOURCES = {}
DROP = re.compile(
    r"\b(rd_kafka_toppar_destroy|rd_kafka_toppar_destroy0|rd_kafka_q_destroy|rd_kafka_q_destroy0|"
    r"rd_kafka_q_destroy_owner|rd_kafka_topic_destroy0|rd_kafka_enq_once_trigger)\s*\(")
UNHELD = "(reference-drop-unheld)"


def lines_of(name):
    if name not in SOURCES:
        found = sorted(ROOT.glob("lib/**/" + name))
        SOURCES[name] = found[0].read_text(errors="replace").splitlines() if found else []
    return SOURCES[name]


def split_arguments(text, start):
    depth, current, parts = 0, "", []
    for char in text[start:]:
        if char in "([":
            depth += 1
        elif char in ")]":
            if depth == 0:
                parts.append(current)
                return [part.strip() for part in parts]
            depth -= 1
        if char == "," and depth == 0:
            parts.append(current)
            current = ""
        else:
            current += char
    return None


def dropped_name(text):
    match = DROP.search(text or "")
    if not match:
        return None
    parts = split_arguments(text, match.end())
    if not parts:
        return None
    callee = match.group(1)
    actual = parts[0] if callee in ("rd_kafka_q_destroy0", "rd_kafka_enq_once_trigger") else parts[-1]
    return actual if re.fullmatch(r"[A-Za-z_]\w*", actual) else None


def definition_start(lines, function, before):
    pattern = re.compile(r"\b" + re.escape(function) + r"\s*\(")
    for index in range(before, -1, -1):
        line = lines[index]
        if pattern.search(line) and not line.rstrip().endswith(";"):
            # a definition: the next `{` at column 0 or line end follows
            for ahead in range(index, min(index + 12, len(lines))):
                if ";" in lines[ahead] and "{" not in lines[ahead]:
                    break
                if "{" in lines[ahead]:
                    return index
    return None


def origin_of(lines, function, line_index, name):
    start = definition_start(lines, function, line_index)
    if start is None:
        return "unassigned", None
    header = []
    for index in range(start, line_index + 1):
        header.append(lines[index])
        if "{" in lines[index]:
            break
    signature = " ".join(header)
    signature = signature[signature.find(function):]
    params = signature[signature.find("(") + 1:signature.rfind(")")]
    if re.search(r"[\s\*]" + re.escape(name) + r"\s*(,|$|\[)", params + ","):
        formal = True
    else:
        formal = False
    text = "\n".join(lines[start:line_index + 1])
    assign = re.compile(r"(?<![\w>.])" + re.escape(name) + r"\s*=(?!=)\s*([^;]*);", re.S)
    found = list(assign.finditer(text))
    if found:
        rhs = " ".join(found[-1].group(1).split())
        rhs = re.sub(r"^\(\s*[\w\s\*]+\*\s*\)\s*", "", rhs)
        call = re.match(r"([A-Za-z_]\w*)\s*\(", rhs)
        if call:
            return "call:" + call.group(1), rhs
        return "other:" + rhs[:60], rhs
    if formal:
        return "formal", None
    # `for (name = ...)` or RD_LIST_FOREACH(name, ...)
    if re.search(r"FOREACH\w*\s*\(\s*" + re.escape(name) + r"\b", text):
        return "other:FOREACH", None
    return "unassigned", None


def main():
    report = json.load(open(sys.argv[1]))
    leaves = collections.Counter()
    carrying = collections.defaultdict(set)
    rows = []
    per_function = {}
    for unit in report["units"]:
        for function in unit["functions"]:
            if function["status"] == "Safe":
                continue
            key = (unit["file"], function["function"])
            diagnostics = function.get("diagnostics") or []
            classes = []
            for leaf in diagnostics:
                description = leaf.get("description", "")
                if UNHELD not in description:
                    classes.append(description.split("`")[0][:50] if "`" in description else description[:50])
                    continue
                origin = leaf.get("origin") or ""
                if ":" not in origin:
                    classes.append("unheld:unread")
                    continue
                file, number = origin.rsplit(":", 1)
                lines = lines_of(file)
                index = int(number) - 1
                text = " ".join(lines[index:index + 3]) if 0 <= index < len(lines) else ""
                name = dropped_name(text)
                if name is None:
                    classes.append("unheld:not-a-name")
                    continue
                kind, rhs = origin_of(lines, function["function"], index, name)
                leaves[kind] += 1
                carrying[kind].add(key)
                classes.append("unheld:" + kind)
                rows.append({"file": unit["file"], "function": function["function"], "origin": origin,
                             "name": name, "kind": kind, "rhs": rhs})
            per_function[key] = classes
    alone = collections.Counter()
    for key, classes in per_function.items():
        present = set(classes)
        if len(present) == 1 and next(iter(present)).startswith("unheld:"):
            alone[next(iter(present))[len("unheld:"):]] += 1
    summary = {kind: {"leaves": leaves[kind], "functions": len(carrying[kind]), "alone": alone[kind]}
               for kind in sorted(leaves, key=lambda k: -leaves[k])}
    functions = {"%s:%s" % key: classes for key, classes in sorted(per_function.items())
                 if any(c.startswith("unheld:") for c in classes)}
    with open(sys.argv[2], "w") as out:
        json.dump({"summary": summary, "leaves": rows, "functions": functions}, out, indent=1)
        out.write("\n")
    total = sum(leaves.values())
    print("plain-name unheld drops: %d leaves, %d functions" % (
        total, len({(r["file"], r["function"]) for r in rows})))
    for kind, row in summary.items():
        print("  %-48s leaves %4d functions %4d alone %3d" % (kind, row["leaves"], row["functions"], row["alone"]))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
