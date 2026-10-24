#!/usr/bin/env python3
"""W48-RK-LOG-CALLER-FRAME: split every changed row (status or leaves) of a
BASE/TIP corpus pair into rule 32's and its siblings'.

usage: split.py BASE_DIR TIP_DIR OUT_JSON

A row is rule 32's when it is a frame member (rd_kafka_log0,
rd_kafka_log_buf) or its TIP contract dependencies consult either
declaration; rule 32 changes nothing else (its declarations and its hold act
only at calls to those two names). Every other changed row is a sibling
effect (rules 34 and 35 are merged into this TIP's base). Leaf trades are
counted per group by closure_r3.family, and CG06 / W47 leaf kinds are named:
`unknown_alias` (W47-ALIAS-CALLEE-BORROWED-CLOSURE's leaf) and value-connective
refusals (CG06-VALUE-CONNECTIVE-RESIDUALS).
"""
import collections
import json
import pathlib
import sys

ROOT = next(p for p in pathlib.Path(__file__).resolve().parents if (p / "docs/reports").is_dir())
sys.path.insert(0, str(ROOT / "docs/reports/w48-bottom-up-callee-bodies/scripts"))
import closure_r3  # noqa: E402

FRAME = ("rd_kafka_log0", "rd_kafka_log_buf")


def rows(path):
    out, seen = {}, collections.Counter()
    for unit in closure_r3.load(pathlib.Path(path) / "resource-diagnostics.json")["units"]:
        for entry in unit["functions"]:
            key = (unit["file"], entry["function"])
            out[key + (seen[key],)] = entry
            seen[key] += 1
    return out


def leaves(entry):
    return collections.Counter((d.get("category"), d.get("description")) for d in entry.get("diagnostics") or [])


def main():
    base, tip = rows(sys.argv[1]), rows(sys.argv[2])
    groups = {g: {"rows": 0, "status": [], "removed": collections.Counter(), "added": collections.Counter(),
                  "unknown_alias": [0, 0], "value_connective": [0, 0]} for g in ("rule32", "sibling")}
    for key in sorted(set(base) & set(tip)):
        b, t = base[key], tip[key]
        lb, lt = leaves(b), leaves(t)
        if b["status"] == t["status"] and lb == lt:
            continue
        consulted = set((t.get("contract_dependencies") or {}).get("declarations") or [])
        group = "rule32" if key[1] in FRAME or consulted & set(FRAME) else "sibling"
        g = groups[group]
        g["rows"] += 1
        if b["status"] != t["status"]:
            g["status"].append([key[0], key[1], b["status"], t["status"]])
        for (cat, desc), n in (lb - lt).items():
            g["removed"][closure_r3.family({"category": cat, "description": desc})] += n
        for (cat, desc), n in (lt - lb).items():
            g["added"][closure_r3.family({"category": cat, "description": desc})] += n
        for side, counter in ((0, lb), (1, lt)):
            for (cat, desc), n in counter.items():
                if cat == "unknown_alias":
                    g["unknown_alias"][side] += n
                if "value-connective" in (desc or "") or "value connective" in (desc or ""):
                    g["value_connective"][side] += n
    report = {g: dict(v, removed=dict(v["removed"].most_common()), added=dict(v["added"].most_common()))
              for g, v in groups.items()}
    pathlib.Path(sys.argv[3]).write_text(json.dumps(report, indent=1, sort_keys=True) + "\n")
    for g, v in report.items():
        print(g, "rows", v["rows"], "status", len(v["status"]), "alias", v["unknown_alias"],
              "value-connective", v["value_connective"])
        print("  removed", v["removed"])
        print("  added", v["added"])


if __name__ == "__main__":
    main()
