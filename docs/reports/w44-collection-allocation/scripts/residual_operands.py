#!/usr/bin/env python3
"""Card W44-COLLECTION-ALLOCATION: the dereference-operand leaves AFTER keeps.

Usage: residual_operands.py SIZEOF_CENSUS_JSON AFTER_RUN UNIT_TEXTS_DIR OUT_TXT

Lists every leaf of AFTER on a line (or one or two lines before a line) the
BEFORE census lists as carrying a dereference `sizeof` operand, grouped by the
leaf description, with the Unknown functions and a line sample. These are the
statements whose operand the rule did not admit or whose statement a later
refusal still holds, sized for the residual this card registers.
"""

import collections
import json
import pathlib
import sys

DEREFERENCE = {"(*NAME)", "*NAME", "(**NAME)", "**NAME", "(NAME[LIT])", "NAME[LIT]"}


def main() -> int:
    census = json.loads(pathlib.Path(sys.argv[1]).read_text())
    after = json.loads((pathlib.Path(sys.argv[2]) / "resource-diagnostics.json").read_text())
    texts = pathlib.Path(sys.argv[3])
    out_txt = pathlib.Path(sys.argv[4])
    operand_lines = set()
    for function in census["functions"]:
        for occurrence in function["occurrences"]:
            if occurrence["shape"] in DEREFERENCE:
                operand_lines.add((function["unit"], function["function"], occurrence["line"]))
    groups = collections.defaultdict(list)
    for unit in after["units"]:
        key = f"{unit['library']}__{unit['file'].replace('/', '__')}.i"
        lines = (texts / key).read_text(errors="replace").split("\n") if (texts / key).exists() else []
        for function in unit.get("functions", []):
            for leaf in function.get("diagnostics", []):
                if leaf["category"] != "unsupported":
                    continue
                if not any((key, function["function"], leaf["line"] + k) in operand_lines for k in range(3)):
                    continue
                source = lines[leaf["line"] - 1].strip() if 0 < leaf["line"] <= len(lines) else ""
                groups[leaf["description"]].append((function["status"], key, function["function"], leaf["line"], source))
    text = ["W44-COLLECTION-ALLOCATION: unsupported leaves AFTER keeps on dereference-operand lines", ""]
    total = sum(len(rows) for rows in groups.values())
    unknown = {(r[1], r[2]) for rows in groups.values() for r in rows if r[0] == "Unknown"}
    text.append(f"leaves: {total}; on Unknown functions: {sum(1 for rows in groups.values() for r in rows if r[0] == 'Unknown')}; Unknown functions: {len(unknown)}")
    text.append("")
    for description, rows in sorted(groups.items(), key=lambda item: -len(item[1])):
        text.append(f"{len(rows)}  {description}")
        for status, key, function, line, source in rows[:6]:
            text.append(f"      {status} {key}::{function} L{line}: {source[:140]}")
    out_txt.write_text("\n".join(text) + "\n")
    print("\n".join(text))
    return 0


if __name__ == "__main__":
    sys.exit(main())
