#!/usr/bin/env python3
"""CONTRACT-NORETURN-AND-BORROWED-TAILS: attribute the moving functions that
lost no card leaf by line.

Usage: attribute_other.py BEFORE AFTER PARITY.jsonl[,PARITY.jsonl...] OUT.json

For each function that changed between BEFORE and AFTER without losing a
`call to uncontracted function `rd_kafka_crash`` or a tail-refusal leaf, lists
every removed and added leaf with its line, and every parity row (any
position, from each given stream: pass the BEFORE and AFTER streams) of that
function whose statement tokens start with a card callee (`rd_kafka_crash`,
`printf`, `fprintf`, `snprintf`, `mk_print`, `flb_log_print`). A leaf whose
line is such a row's line is attributed to that callee: at the base such a
statement could be refused for one of its actuals (an indirect access) before
the uncontracted-call refusal was reached.
"""
import collections
import json
import pathlib
import re
import sys

CARD = re.compile(r"^(call to uncontracted function `rd_kafka_crash`|arguments beyond the declared "
                  r"parameters of `(printf|fprintf|snprintf|mk_print|flb_log_print)` are outside the "
                  r"resource projection)$")


def load(path):
    path = pathlib.Path(path)
    if path.is_dir():
        path = path / "resource-diagnostics.json"
    return json.loads(path.read_text())


def leaves(report):
    out = {}
    for unit in report["units"]:
        for entry in unit["functions"]:
            out.setdefault((unit["file"], entry["function"]), []).extend(
                (d.get("line"), f"{d['category']}: {d.get('description', '')}")
                for d in entry.get("diagnostics") or [])
    return out


def main() -> int:
    before, after = leaves(load(sys.argv[1])), leaves(load(sys.argv[2]))
    moving = {}
    for key in before:
        old, new = collections.Counter(before[key]), collections.Counter(after.get(key, []))
        if old == new:
            continue
        gone, came = old - new, new - old
        if any(CARD.match(text.split(": ", 1)[1]) for _, text in gone):
            continue
        moving[key[1]] = {"file": key[0], "removed": sorted(gone.elements(), key=str),
                          "added": sorted(came.elements(), key=str), "rows": []}
    seen = set()
    names = ("rd_kafka_crash", "printf", "fprintf", "snprintf", "mk_print", "flb_log_print")
    for path in sys.argv[3].split(","):
        for line in open(path, encoding="utf-8"):
            if not any(f'"texts":["{name}"' in line for name in names):
                continue
            row = json.loads(line)
            if row["function"] not in moving or row["texts"][0] not in names:
                continue
            key = (row["function"], row["line"], tuple(row["texts"]), row["position"])
            if key in seen:
                continue
            seen.add(key)
            moving[row["function"]]["rows"].append(
                {"position": row["position"], "callee": row["texts"][0], "line": row["line"],
                 "texts": " ".join(row["texts"])[:200]})
    summary = collections.Counter()
    for function, entry in moving.items():
        lines = {row["line"]: row["callee"] for row in entry["rows"]}
        entry["attributed"] = all(line in lines for line, _ in entry["removed"] + entry["added"])
        entry["callees"] = sorted({lines[line] for line, _ in entry["removed"] + entry["added"]
                                   if line in lines})
        summary[(entry["attributed"], tuple(entry["callees"]))] += 1
    out = {"functions": len(moving),
           "summary": {f"attributed={a} callees={','.join(c)}": n for (a, c), n in summary.items()},
           "moves": moving}
    pathlib.Path(sys.argv[4]).write_text(json.dumps(out, indent=1, ensure_ascii=False) + "\n")
    print(json.dumps({k: out[k] for k in ("functions", "summary")}, indent=1))
    return 0


if __name__ == "__main__":
    sys.exit(main())
