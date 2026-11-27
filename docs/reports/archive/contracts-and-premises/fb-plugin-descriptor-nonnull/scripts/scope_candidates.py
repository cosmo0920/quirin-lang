#!/usr/bin/env python3
"""scope_candidates.py OUT.json NULL.jsonl... -- FB-PLUGIN-DESCRIPTOR-NONNULL-SCOPE:
every function of the given BEFORE null streams with a candidate on a plugin
descriptor `V->p` (source `memory-field:V->p`, the subject `V->p` or a copy
of it), classified for a `nonnull_member_scope(TAG, p, V)` entry.

A function is chosen when, in some configuration, every candidate of its row
is sourced `memory-field:V->p` for one formal `V` of type `struct
flb_{input,output,filter,processor}_instance *` (so the entry would leave no
candidate in the body), its definition is byte-identical at 046accf6 and the
tail-pause export (so one pin is in force on both routes), the trees take
its address nowhere (an indirect caller is never checked, rule 14z), and it
has external linkage. Run it on the stable-only reading's streams: a
candidate the stable member already removes needs no entry. Every
other function with such a candidate is listed with the reason it is not
chosen: an entry there leaves other candidates in the body while moving the
question to every caller."""
import collections
import json
import os
import re
import sys

NEW = "/home/cosmo/Documents/GitHub/qcore-c/private/w45-fb/w46-external-callee-contracts/attempt-1/fluent-bit-clang-source"
SLICE = "/home/cosmo/Documents/GitHub/qcore-c/private/w47-tail/attempt-1/prep/fluent-bit-clang-source"
TAGS = ("flb_input_instance", "flb_output_instance", "flb_filter_instance", "flb_processor_instance")


def definition(root, path, name):
    try:
        text = open(os.path.join(root, path), errors="replace").read()
    except OSError:
        return None
    for match in re.finditer(r"^[^\s#/][^;{}]*\b%s\s*\(([^;{}]*)\)\s*\{" % re.escape(name), text, re.M):
        depth = 0
        for index in range(match.end() - 1, len(text)):
            if text[index] == "{":
                depth += 1
            elif text[index] == "}":
                depth -= 1
                if depth == 0:
                    return match.group(1), text[match.start():index + 1]
    return None


def formals(params):
    out = {}
    for part in params.split(","):
        m = re.match(r"\s*(?:const\s+)?struct\s+(\w+)\s*\*\s*(\w+)\s*$", part)
        if m:
            out[m.group(2)] = m.group(1)
    return out


MENTIONS = None


def address_taken(name):
    global MENTIONS
    if MENTIONS is None:
        MENTIONS = collections.Counter()
        for d in ("src", "include", "plugins"):
            for base, _, files in os.walk(os.path.join(NEW, d)):
                for f in files:
                    if f.endswith((".c", ".h")) and os.path.isfile(os.path.join(base, f)):
                        for m in re.finditer(r"\b(\w+)\b(?!\s*\()", open(os.path.join(base, f), errors="replace").read()):
                            MENTIONS[m.group(1)] += 1
    return MENTIONS[name] > 0


rows = collections.defaultdict(list)
for path in sys.argv[2:]:
    route = "slice" if "slice" in os.path.basename(path) else "system"
    for line in open(path):
        row = json.loads(line)
        null = row.get("null") or {}
        cands = [c for c in null.get("checks") or [] if c["outcome"] == "candidate"]
        rows[(row["file"], row["function"])].append((route, null.get("verdict"), cands))

result = []
for (path, name), entries in sorted(rows.items()):
    hit = set()
    for route, verdict, cands in entries:
        for c in cands:
            for s in c.get("sources") or []:
                m = re.fullmatch(r"memory-field:(\w+)->p", s)
                if m:
                    hit.add(m.group(1))
    if not hit:
        continue
    record = {"file": path, "function": name, "routes": sorted({e[0] for e in entries}), "bases": sorted(hit)}
    found = definition(NEW, path, name)
    if found is None:
        record["chosen"] = False
        record["reason"] = "no definition read"
        result.append(record)
        continue
    params = formals(found[0])
    record["formals"] = params
    other = definition(SLICE, path, name)
    record["identical_at_046accf6_and_export"] = other is not None and other[1] == found[1]
    sole = None
    for route, verdict, cands in entries:
        sources = {s for c in cands for s in (c.get("sources") or [])}
        bases = {re.fullmatch(r"memory-field:(\w+)->p", s).group(1) if re.fullmatch(r"memory-field:(\w+)->p", s) else None
                 for s in sources}
        if cands and len(bases) == 1 and None not in bases:
            sole = next(iter(bases))
    record["every_candidate_on_one_base"] = sole
    reasons = []
    if sole is None:
        reasons.append("other candidates remain in every configuration")
    elif sole not in params or params[sole] not in TAGS:
        reasons.append(f"`{sole}` is not an instance formal (a local or a member path)")
    if not record["identical_at_046accf6_and_export"]:
        reasons.append("definition differs between 046accf6 and the export")
    if address_taken(name):
        reasons.append("address taken")
    header = found[1][:found[1].index(name + "(") if name + "(" in found[1] else found[1].index(name)]
    if re.search(r"\bstatic\b", header.splitlines()[-1] if header.splitlines() else header):
        reasons.append("internal linkage (a declaration names a function run-wide; rule 14v exports nothing from a static definition)")
    record["chosen"] = not reasons
    if reasons:
        record["reason"] = "; ".join(reasons)
    else:
        record["tag"] = params[sole]
    result.append(record)

open(sys.argv[1], "w").write(json.dumps(result, indent=1) + "\n")
for r in result:
    print(("CHOSEN " if r["chosen"] else "       ") + f"{r['function']} ({r['file']}, {','.join(r['routes'])}) bases={r['bases']} " + (r.get("tag", "") or r.get("reason", "")))
