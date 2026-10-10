#!/usr/bin/env python3
"""triage_diff.py BEFORE-triage.json AFTER-triage.json [OUT.json] --
NULL-TRIAGE-UNRESOLVED-PATHS: every searched candidate (keyed by library,
file, function, origin, subject) with its BEFORE and AFTER status, class and
reason; the moves between statuses; and the AFTER unresolved reasons by class
(the leading clause of the reason, line numbers dropped)."""
import collections
import json
import re
import sys


def entries(path):
    data = json.load(open(path))
    out = {}
    for e in data["findings"] + data["not_findings"]:
        key = (e["library"], e["file"], e["function"], e["origin"], e["subject"])
        out.setdefault(key, []).append(e)
    return out


def klass(entry):
    reason = entry["reason"]
    if entry["status"] != "unresolved":
        return entry["status"]
    if reason.startswith("an inexact path reaches the dereference NULL"):
        tail = reason.split(", but only along ", 1)[-1]
        for prefix in ("a condition over the result of", "a correlated condition",
                       "a condition over a value the search does not follow",
                       "a switch over", "a correlated switch", "an ordered pointer comparison",
                       "a correlated pointer comparison", "a correlated test", "a relation",
                       "a comparison over a value of unknown range"):
            if tail.startswith(prefix):
                return "inexact path: " + prefix
        return "inexact path: other"
    return re.sub(r" at line \d+.*$", "", reason)


before, after = entries(sys.argv[1]), entries(sys.argv[2])
rows = []
moves = collections.Counter()
for key in sorted(set(before) | set(after)):
    b, a = before.get(key, []), after.get(key, [])
    for i in range(max(len(b), len(a))):
        be = b[i] if i < len(b) else None
        ae = a[i] if i < len(a) else None
        bs = be and (be["status"] + ("/" + be["class"] if be.get("class") else ""))
        as_ = ae and (ae["status"] + ("/" + ae["class"] if ae.get("class") else ""))
        moves[f"{bs} -> {as_}"] += 1
        rows.append({
            "library": key[0], "file": key[1], "function": key[2], "origin": key[3],
            "subject": key[4], "before": bs, "after": as_,
            "before_reason": be and be["reason"], "after_reason": ae and ae["reason"],
            "after_class": ae and klass(ae),
        })
classes = collections.Counter(r["after_class"] for r in rows if r["after"] and r["after"].startswith("unresolved"))
out = {"moves": dict(moves.most_common()), "unresolved_by_class": dict(classes.most_common()),
       "candidates": rows}
if len(sys.argv) > 3:
    open(sys.argv[3], "w").write(json.dumps(out, indent=1, ensure_ascii=False) + "\n")
print(json.dumps({k: out[k] for k in ("moves", "unresolved_by_class")}, indent=1))
for r in rows:
    if r["before"] != r["after"]:
        print(r["before"], "->", r["after"], r["function"], r["origin"], "|", (r["after_reason"] or "")[:220])
