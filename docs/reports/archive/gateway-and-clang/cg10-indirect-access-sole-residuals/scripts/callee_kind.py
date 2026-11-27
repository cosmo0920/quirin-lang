#!/usr/bin/env python3
"""callee_kind.py RANKING.json RUN SOURCE_ROOT OUT.json [--top N] -- card
CG10-INDIRECT-ACCESS-SOLE-RESIDUALS, task 2: where each ranked callee's body
is, and why that body decides nothing.

For each callee of callee_census.py's ranking (top N):

* `same-unit` / `cross-unit`: a reported row of that name exists in a unit of
  the run (RUN/resource-diagnostics.json) that calls it / only in another
  unit; the row's status and its leaves' families (templated descriptions)
  are listed, so an Unknown body's blockers are named;
* `header-inline`: no reported row, and a definition `NAME (` followed by a
  body is found in a header under SOURCE_ROOT (a `static` header body, inside
  W4's derivation scope per gateway rule 4a but not a reported row);
* `external`: no reported row and no header definition (a library outside the
  pinned units, libc, or a compiler builtin).

The search is lexical (a reading aid): every placement used for a decision
is read in the source and stated in the report.
"""
import argparse
import collections
import json
import pathlib
import re
import subprocess

TEMPLATE = re.compile(r"`[^`]*`")


def header_definitions(root, name):
    pattern = r"^[^;#/]*\b%s\s*\([^;]*$" % re.escape(name)
    try:
        output = subprocess.run(
            ["grep", "-rnE", "--include=*.h", "-e", pattern, str(root)],
            capture_output=True, text=True, check=False).stdout
    except OSError:
        return []
    found = []
    for line in output.splitlines():
        path, number, text = line.split(":", 2)
        if "static" in text or "inline" in text.lower():
            found.append("%s:%s" % (path[len(str(root)) + 1:], number))
    return found


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("ranking")
    parser.add_argument("run")
    parser.add_argument("source_root")
    parser.add_argument("out")
    parser.add_argument("--top", type=int, default=60)
    args = parser.parse_args()
    ranking = json.loads(pathlib.Path(args.ranking).read_text())["ranking"][: args.top]
    report = json.loads((pathlib.Path(args.run) / "resource-diagnostics.json").read_text())
    rows = collections.defaultdict(list)
    for unit in report["units"]:
        for function in unit["functions"]:
            families = collections.Counter(
                TEMPLATE.sub("`_`", d.get("description", d.get("category", "")))
                for d in function.get("diagnostics") or [])
            rows[function["function"]].append({
                "unit": unit["file"], "status": function["status"],
                "families": dict(families.most_common(4))})
    out = []
    for row in ranking:
        name = row["callee"]
        homes = rows.get(name, [])
        if homes:
            units = {home["unit"] for home in homes}
            kind = "same-unit" if units & set(row["units"]) else "cross-unit"
        else:
            headers = header_definitions(pathlib.Path(args.source_root), name)
            kind = "header-inline" if headers else "external"
            homes = [{"header": header} for header in headers[:3]]
        out.append({**row, "kind": kind, "homes": homes[:4]})
        status = ",".join(sorted({h.get("status", "-") for h in homes})) if homes else "-"
        print("%-44s %3d %3d %-13s %s" % (name, row["alone"], row["touched"], kind, status))
    pathlib.Path(args.out).write_text(json.dumps(out, indent=1) + "\n")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
