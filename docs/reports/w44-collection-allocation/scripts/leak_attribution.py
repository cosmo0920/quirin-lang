#!/usr/bin/env python3
"""Card W44-COLLECTION-ALLOCATION: attribute every leak leaf AFTER adds.

Usage: leak_attribution.py BEFORE_RUN AFTER_RUN UNIT_TEXTS_DIR OUT_JSON OUT_TXT

For every leak leaf present in AFTER and absent from BEFORE (a leaf is
`(unit, function, line, description)`), the script finds the allocation it
names: the leaf's description carries the acquisition's origin coordinate
(`live obligation acquired at FILE:LINE`), and the BEFORE run holds, in the
same function, the `indirect memory access` leaf this card withdrew at that
origin coordinate, whose `line` is the analysed text's line. The allocated
name is the assignment destination on that line. Then it classifies the leaf
by what happens to that name on the paths the analysis did not model:

* `returned-under-cast`: the leak's own line is a `return` whose expression
  names the allocation under a cast or other non-place spelling; the return
  route answers such an expression with no escaped place (constraint 5b), so
  the identity is still owed at the exit;
* `handed-to-a-refused-statement`: the allocated name appears, after the
  allocation, on a line carrying an `unsupported` leaf in AFTER — a store into
  an object, a list-insertion macro, an uncontracted callee. A refused
  statement withdraws a residual claim only for a publication the publication
  reader recognises, so a store it does not recognise leaves the identity owed;
* `other`: neither; the path to the exit is modelled throughout.

It also records whether the allocation line was withdrawn from BEFORE (the
causal link to this card) and the function's status in both runs. A reading
of source text and leaf coordinates: a forecast-grade classifier, never a
verdict; the classes are checked by reading the functions (see the report).
"""

import collections
import json
import pathlib
import re
import sys

ACCESS = "indirect memory access is outside the resource projection"
LEAK = re.compile(r"live obligation acquired at (\S+)")
NAME = r"[A-Za-z_]\w*"


def load(run):
    return json.loads((run / "resource-diagnostics.json").read_text())


def index(document):
    functions = {}
    for unit in document["units"]:
        key = f"{unit['library']}/{unit['file']}"
        text_key = f"{unit['library']}__{unit['file'].replace('/', '__')}.i"
        for function in unit.get("functions", []):
            functions[(key, function["function"])] = (text_key, function)
    return functions


def main() -> int:
    before, after = load(pathlib.Path(sys.argv[1])), load(pathlib.Path(sys.argv[2]))
    texts = pathlib.Path(sys.argv[3])
    out_json, out_txt = pathlib.Path(sys.argv[4]), pathlib.Path(sys.argv[5])
    before_functions, after_functions = index(before), index(after)
    rows = []
    for (key, name), (text_key, function) in sorted(after_functions.items()):
        old = before_functions.get((key, name))
        old_leaves = collections.Counter(
            (leaf["line"], leaf["category"], leaf["description"]) for leaf in (old[1]["diagnostics"] if old else [])
        )
        lines = (texts / text_key).read_text(errors="replace").split("\n") if (texts / text_key).exists() else []
        after_unsupported = {
            leaf["line"] for leaf in function["diagnostics"] if leaf["category"] == "unsupported"
        }
        for leaf in function["diagnostics"]:
            if leaf["category"] != "leak":
                continue
            signature = (leaf["line"], leaf["category"], leaf["description"])
            if old_leaves[signature] > 0:
                old_leaves[signature] -= 1
                continue
            origin = LEAK.search(leaf["description"]).group(1)
            withdrawn = [
                l for l in (old[1]["diagnostics"] if old else [])
                if l["description"] == ACCESS and l.get("origin") == origin
            ]
            allocation_line = withdrawn[0]["line"] if withdrawn else None
            allocated = None
            if allocation_line and 0 < allocation_line <= len(lines):
                # The statement may span lines (`T *x =` then the call).
                statement = " ".join(lines[allocation_line - 1:allocation_line + 2])
                match = re.search(rf"({NAME})\s*=\s*(?:\([^()]*\)\s*)?{NAME}\s*\(", statement)
                allocated = match.group(1) if match else None
            leak_text = lines[leaf["line"] - 1].strip() if 0 < leaf["line"] <= len(lines) else ""
            kind = "other"
            handed = []
            if allocated:
                word = re.compile(rf"\b{re.escape(allocated)}\b")
                if leak_text.startswith("return") and word.search(leak_text) and not re.fullmatch(
                    rf"return\s+{re.escape(allocated)}\s*;", leak_text
                ):
                    kind = "returned-under-cast"
                else:
                    handed = sorted(
                        line for line in after_unsupported
                        if allocation_line < line and 0 < line <= len(lines) and word.search(lines[line - 1])
                    )
                    if handed:
                        kind = "handed-to-a-refused-statement"
            rows.append({
                "unit": key, "function": name,
                "status_before": old[1]["status"] if old else None, "status_after": function["status"],
                "leak_line": leaf["line"], "acquisition": origin,
                "allocation_line": allocation_line, "allocated": allocated,
                "allocation_withdrawn_by_this_card": bool(withdrawn),
                "class": kind, "handed_lines": handed[:5], "leak_text": leak_text[:160],
            })
    summary = {
        "added_leak_leaves": len(rows),
        "functions": len({(r["unit"], r["function"]) for r in rows}),
        "allocation_withdrawn_by_this_card": sum(r["allocation_withdrawn_by_this_card"] for r in rows),
        "by_class": dict(collections.Counter(r["class"] for r in rows).most_common()),
        "functions_by_class": dict(collections.Counter(
            c for c in (
                sorted({r["class"] for r in rows if (r["unit"], r["function"]) == f})[0]
                for f in {(r["unit"], r["function"]) for r in rows}
            )
        ).most_common()),
        "status_moves_of_these_functions": dict(collections.Counter(
            f"{s}->{t}" for (s, t) in {(r["unit"], r["function"]): (r["status_before"], r["status_after"]) for r in rows}.values()
        )),
    }
    out_json.write_text(json.dumps({"summary": summary, "leaves": rows}, indent=1) + "\n")
    lines_out = ["W44-COLLECTION-ALLOCATION: added leak leaves", ""]
    lines_out += [f"{k}: {v}" for k, v in summary.items()]
    lines_out.append("")
    for r in rows:
        lines_out.append(
            f"  [{r['class']}] {r['unit']}::{r['function']} ({r['status_before']}->{r['status_after']}) "
            f"leak L{r['leak_line']} alloc L{r['allocation_line']} `{r['allocated']}` ({r['acquisition']})"
            + (f" handed at L{','.join(map(str, r['handed_lines']))}" if r["handed_lines"] else "")
            + f" | {r['leak_text'][:90]}"
        )
    out_txt.write_text("\n".join(lines_out) + "\n")
    print("\n".join(lines_out[:12]))
    return 0


if __name__ == "__main__":
    sys.exit(main())
