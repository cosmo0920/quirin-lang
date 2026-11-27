#!/usr/bin/env python3
"""W47-PREMISES-INSTANCE: attribute the census AFTER against BEFORE.

Usage: compare.py BEFORE_DIR AFTER_DIR

For each reading of BEFORE (the six of W47-LOOP-OWNER-PREDEFINED-MACRO-ARMS's
census.py) the same-named AFTER reading must be byte-identical (the census
without `--derive-premises` is unchanged, typed facts' new kinds included).
Each premised AFTER reading (`-premised`, `-configured-premised`) is compared
with its unpremised counterpart: every site whose verdict or first reason
moves, every changed instance-flag reason, and the edge, function and seed
counts; then the premise report is summarised (P-INSTANCE per site by form,
the record views). Prints the result; read-only.
"""

import collections
import hashlib
import json
import pathlib
import sys


def load(path):
    return json.loads(path.read_bytes())


def site_key(site):
    return f"{site['file']}:{site['line']} {site['callee']}"


def verdict(site):
    result = site["result"]
    first = (result.get("reasons") or [result.get("context", "")])[0]
    return result["verdict"], first


def form(reason, status):
    if status in ("derived", "refuted"):
        return status
    if "P-INSTANCE-CONTEXT" in reason:
        return "stated: ctx->ins (P-INSTANCE-CONTEXT)"
    if "coroutine entry" in reason:
        return "stated: through a coroutine entry (P-CORO-HANDLE)"
    if "also reached" in reason:
        return "stated: first parameter of a function also called directly"
    return "stated: another argument form"


def main():
    before, after = pathlib.Path(sys.argv[1]), pathlib.Path(sys.argv[2])
    print("== unpremised readings (must be byte-identical)")
    for path in sorted(before.glob("*.json")):
        other = after / path.name
        same = other.exists() and path.read_bytes() == other.read_bytes()
        print(f"{path.name:36} {hashlib.sha256(path.read_bytes()).hexdigest()[:8]} "
              f"{'identical' if same else 'DIFFERS'}")
    for name in ("fixed", "previous"):
        for base, premised in (("", "-premised"), ("-configured", "-configured-premised")):
            a, b = load(after / f"{name}{base}.json"), load(after / f"{name}{premised}.json")
            print(f"\n== {name}{premised} against {name}{base}")
            tally = lambda d: collections.Counter(s["result"]["verdict"] for s in d["sites"])  # noqa: E731
            ta, tb = tally(a), tally(b)
            print("sites d/n/u:", f"{ta['discharged']}/{ta['not-discharged']}/{ta['unresolved']}",
                  "->", f"{tb['discharged']}/{tb['not-discharged']}/{tb['unresolved']}")
            for key in ("edges", "functions", "unresolved_seeds"):
                print(f"{key}: {a[key]} -> {b[key]}")
            sa = {site_key(s): verdict(s) for s in a["sites"]}
            sb = {site_key(s): verdict(s) for s in b["sites"]}
            for key in sorted(set(sa) | set(sb)):
                if sa.get(key) != sb.get(key):
                    print(f"  site {key}: {sa.get(key)} -> {sb.get(key)}")
            fa = a.get("typed", {}).get("instance_flags", {})
            fb = b.get("typed", {}).get("instance_flags", {})
            for key in sorted(set(fa) | set(fb)):
                if fa.get(key) != fb.get(key):
                    print(f"  flag {key}: {fa.get(key, {}).get('derived')} -> "
                          f"{fb.get(key, {}).get('derived')}")
                    print(f"    after: {fb.get(key, {}).get('reason', '')}")
            report = b.get("premises")
            if not report:
                continue
            verdicts = {site_key(s): s["result"]["verdict"] for s in b["sites"]}
            forms = collections.Counter()
            for key, value in report["instance_sites"].items():
                forms[(form(value["reason"], value["status"]), verdicts.get(key, "?"))] += 1
            print("  P-INSTANCE by form and verdict:")
            for (kind, v), count in sorted(forms.items()):
                print(f"    {count:4} {v:15} {kind}")
            fields = report["fields_by_member"]
            print(f"  record views: read={fields['read']} violations={len(fields['violations'])} "
                  f"escapes={len(fields['escapes'])} other_types={len(fields['other_types'])} "
                  f"harmless={fields['harmless']}")
            for line in fields["violations"] + fields["other_types"]:
                print(f"    {line}")
            generated = {k: v for k, v in report["generated"].items() if v}
            print(f"  generated files resolved by an include: {generated}")


if __name__ == "__main__":
    main()
