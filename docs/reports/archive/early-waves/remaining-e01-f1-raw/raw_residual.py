#!/usr/bin/env python3
"""E01-F1-RAW: the raw-scan-vs-enumeration residual, before and after.

Card E01-F1-RAW. Read-only: it opens two produced `resource-diagnostics.json`
documents and reports, per unit, how the enumeration's `absent_from_raw_scan`
list moved, together with every other per-unit change (status, denominator,
diagnostics, reason, function rows). It never edits an artifact.

    python3 -B docs/reports/remaining-e01-f1-raw/raw_residual.py \\
        <before>/resource-diagnostics.json <after>/resource-diagnostics.json

`absent_from_raw_scan` is exactly the delta this unit shrinks: definitions the
audited denominator carries that the historical raw token scan did not see.
"""
import json, sys, collections, pathlib

def units(path):
    doc = json.loads(pathlib.Path(path).read_text())
    return {(u.get("library"), u.get("file")): u for u in doc["units"]}, doc

b, bdoc = units(sys.argv[1])
a, adoc = units(sys.argv[2])

print("corpus diagnostic_counts before:", bdoc.get("diagnostic_counts"))
print("corpus diagnostic_counts after :", adoc.get("diagnostic_counts"))

def total(d, key):
    return sum(len((u.get("enumeration") or {}).get(key) or []) for u in d.values())

for key in ("absent_from_raw_scan", "unanalyzable", "refusals"):
    print(f"{key}: before={total(b, key)} after={total(a, key)}")

print("\nunits whose absent_from_raw_scan changed:")
for k in sorted(set(b) & set(a)):
    eb = (b[k].get("enumeration") or {}).get("absent_from_raw_scan") or []
    ea = (a[k].get("enumeration") or {}).get("absent_from_raw_scan") or []
    if eb != ea:
        gone = sorted(set(eb) - set(ea))
        new = sorted(set(ea) - set(eb))
        print(f"  {k[0]}/{k[1]}: {len(eb)} -> {len(ea)}")
        if new:
            print(f"      newly absent-from-raw ({len(new)}): {new[:20]}")
        if len(gone) <= 20:
            print(f"      no longer absent ({len(gone)}): {gone}")
        else:
            print(f"      no longer absent ({len(gone)}): {gone[:8]} ...")

print("\nunits with any other change:")
moved = 0
for k in sorted(set(b) & set(a)):
    ub, ua = b[k], a[k]
    out = []
    if ub.get("status") != ua.get("status"):
        out.append(f"  STATUS {ub['status']} -> {ua['status']}")
    if ub.get("source_functions") != ua.get("source_functions"):
        out.append(f"  source_functions {ub.get('source_functions')} -> {ua.get('source_functions')}")
    if ub.get("diagnostic_counts") != ua.get("diagnostic_counts"):
        out.append(f"  diagnostic_counts {ub.get('diagnostic_counts')} -> {ua.get('diagnostic_counts')}")
    if ub.get("not_projected") != ua.get("not_projected"):
        out.append(f"  not_projected {ub.get('not_projected')} -> {ua.get('not_projected')}")
    if ub.get("reason") != ua.get("reason"):
        out.append(f"  reason:\n    - {ub.get('reason')}\n    + {ua.get('reason')}")
    if ub.get("reason_code") != ua.get("reason_code"):
        out.append(f"  reason_code {ub.get('reason_code')} -> {ua.get('reason_code')}")
    fb = {f["function"]: f for f in (ub.get("functions") or [])}
    fa = {f["function"]: f for f in (ua.get("functions") or [])}
    added = sorted(set(fa) - set(fb))
    removed = sorted(set(fb) - set(fa))
    changed = [n for n in sorted(set(fb) & set(fa)) if fb[n].get("status") != fa[n].get("status")]
    dchanged = [n for n in sorted(set(fb) & set(fa)) if fb[n].get("diagnostics") != fa[n].get("diagnostics")]
    if added:
        out.append(f"  +function rows ({len(added)}): " + ", ".join(f"{n}={fa[n].get('status')}" for n in added[:15]))
    if removed:
        out.append(f"  -function rows ({len(removed)}): " + ", ".join(f"{n}={fb[n].get('status')}" for n in removed[:15]))
    if changed:
        out.append(f"  ~function status ({len(changed)}): " + ", ".join(f"{n} {fb[n]['status']}->{fa[n]['status']}" for n in changed[:15]))
    if dchanged and not changed:
        out.append(f"  ~function diagnostics only ({len(dchanged)}): {dchanged[:10]}")
    if out:
        moved += 1
        print(f"{k[0]}/{k[1]}")
        print("\n".join(out))
print(f"\nunits with a non-enumeration change: {moved}")
