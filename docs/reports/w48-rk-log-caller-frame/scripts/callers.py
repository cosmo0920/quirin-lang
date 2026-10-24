#!/usr/bin/env python3
"""W48-RK-LOG-CALLER-FRAME: every function whose BASE row names rd_kafka_log0
as an uncontracted callee, with its TIP status and remaining leaf families.

usage: callers.py BASE_DIR TIP_DIR OUT_JSON
"""
import collections
import json
import pathlib
import sys

ROOT = next(p for p in pathlib.Path(__file__).resolve().parents if (p / "docs/reports").is_dir())
sys.path.insert(0, str(ROOT / "docs/reports/w48-bottom-up-callee-bodies/scripts"))
import closure_r3  # noqa: E402


def load(path):
    out = {}
    for unit in closure_r3.load(pathlib.Path(path) / "resource-diagnostics.json")["units"]:
        for entry in unit["functions"]:
            out.setdefault((unit["file"], entry["function"]), entry)
    return out


def main():
    base, tip, out = load(sys.argv[1]), load(sys.argv[2]), sys.argv[3]
    rows, remaining = [], collections.Counter()
    for key, entry in sorted(base.items()):
        names = [closure_r3.callee_of(d)[0] for d in entry.get("diagnostics") or []]
        if "rd_kafka_log0" not in names:
            continue
        after = tip[key].get("diagnostics") or []
        families = collections.Counter(closure_r3.family(d) for d in after)
        remaining.update(families)
        rows.append({"file": key[0], "function": key[1], "base": entry["status"],
                     "tip": tip[key]["status"], "base_leaves": len(entry["diagnostics"]),
                     "tip_leaves": len(after), "tip_families": dict(families)})
    report = {"callers": len(rows), "moved": [r["function"] for r in rows if r["base"] != r["tip"]],
              "remaining_families": dict(remaining.most_common()), "rows": rows}
    pathlib.Path(out).write_text(json.dumps(report, indent=1, sort_keys=True) + "\n")
    print(json.dumps({k: report[k] for k in ("callers", "moved")}))
    for family, count in remaining.most_common(10):
        print(count, family)


if __name__ == "__main__":
    main()
