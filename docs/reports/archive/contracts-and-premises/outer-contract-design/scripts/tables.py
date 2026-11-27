#!/usr/bin/env python3
"""tables.py -- OUTER-CONTRACT-DESIGN: the README and PREDICTION tables from
artifacts/outer-rows.json and artifacts/projection.json (printed as
Markdown; the documents carry the printed text)."""
import json
import pathlib

ART = pathlib.Path(__file__).resolve().parent.parent / "artifacts"
rows = json.loads((ART / "outer-rows.json").read_text())
proj = json.loads((ART / "projection.json").read_text())

print("NULL_TABLE")
for route in ("corpus", "system", "slice"):
    null = rows["routes"][route]["null"] or {}
    for cls in ("fb-stable", "fb-scope", "premise:rk_no_plugin_library"):
        v = null.get("by_class_null_verdict", {}).get(cls)
        if not v:
            continue
        c = null["by_class_combined_verdict"][cls]
        n = sum(v.values())
        print(f"| {route} | `{cls}` | {n:,} | {v.get('Safe', 0):,} | {v.get('Unknown', 0):,} | "
              f"{v.get('Unsafe', 0):,} | {c.get('Safe', 0):,} | {null['obligations'].get(cls, 0):,} |")

print("CUSTODY_TABLE")
for route in ("corpus", "system", "slice"):
    cust = rows["routes"][route]["custody"] or {}
    for cls, v in cust.get("by_class_status", {}).items():
        n = sum(v.values())
        print(f"| {route} | `{cls}` | {n:,} | {v.get('Safe', 0):,} | {v.get('Unknown', 0):,} | "
              f"{v.get('Unsafe', 0):,} | {cust['obligations_declarations_rested_on'].get(cls, 0):,} |")

print("PROJECTION_TABLE")
print("| Route | Scenario | Custody Safe → Pending / Unknown | Null Safe → Pending / Unknown | Combined Safe → Pending / Unknown | Runtime custody | Runtime null | Runtime combined |")
print("| --- | --- | ---: | ---: | ---: | --- | --- | --- |")


def fmt(t):
    return " / ".join(f"{t.get(k, 0):,}" for k in ("Safe", "Pending", "Unknown", "Unsafe"))


for route in ("corpus", "system", "slice"):
    scen = proj.get(route)
    if not scen:
        continue
    for s, v in scen.items():
        if s == "static":
            continue
        m = v["safe_moves"]
        def pair(part):
            return f"{m.get(part + ' -> Pending', 0):,} / {m.get(part + ' -> Unknown', 0):,}"
        print(f"| {route} | {s} | {pair('custody')} | {pair('null')} | {pair('combined')} | "
              f"{fmt(v['custody'])} | {fmt(v['null'])} | {fmt(v['combined'])} |")
