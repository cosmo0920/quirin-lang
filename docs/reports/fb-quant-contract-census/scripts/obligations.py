#!/usr/bin/env python3
"""FB-QUANT-CONTRACT-CENSUS: which Fluent Bit system obligations each pattern touches.

usage: obligations.py SYSTEM_CLASSIFIED SLICE_CLASSIFIED OUT_JSON

System obligations are OR-1..OR-11 and OB-1..OB-10 of
docs/reports/w45-fb-system-selection/lifetime-inventory.json (046accf6 line
ranges in `decidable_from`). Each cited `file:a-b` range is mapped to the
functions whose definitions overlap it in the system export (a lexical
function-span finder: a column-0 definition head, its column-0 `{` and the
next column-0 `}`). The tail-pause slice's contracts C1..C5
(docs/reports/w47-tail-pause-bindings/README.md section 5) are mapped by the
functions that report binds them, listed below.

An obligation depends on a pattern when one of its functions carries a leaf
of that pattern on the route (custody leaf or unresolved null check); the
leaves that fall inside the cited ranges are listed separately.
"""
import json
import os
import re
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
REPO = os.path.abspath(os.path.join(HERE, "..", "..", "..", ".."))
SYSTEM = "/home/cosmo/Documents/GitHub/qcore-c/private/w45-fb/w46-external-callee-contracts/attempt-1/fluent-bit-clang-source"
INVENTORY = os.path.join(REPO, "docs/reports/w45-fb-system-selection/lifetime-inventory.json")
# The tail-pause contracts, by the functions w47-tail-pause-bindings section 5
# binds them to (B-rows), at 90978ac6.
SLICE_OBLIGATIONS = {
    "C1": {"statement": "collector access follows the loop owner",
           "functions": ["collector_create", "flb_input_collector_start", "flb_input_collector_pause",
                         "flb_input_collector_resume", "flb_input_collector_delete", "flb_input_collector_running",
                         "flb_input_set_collector_time", "flb_input_set_collector_event",
                         "flb_input_set_collector_socket", "flb_input_collectors_start", "collector_start"]},
    "C2": {"statement": "two completion meanings (pause/resume enqueued vs completed)",
           "functions": ["flb_input_pause", "flb_input_resume", "flb_input_thread_instance_pause",
                         "flb_input_thread_instance_resume", "input_thread_instance_set_status",
                         "input_thread_instance_get_status", "flb_input_pause_all", "flb_input_resume_all"]},
    "C3": {"statement": "paired backend collectors (in_tail pause/resume)",
           "functions": ["in_tail_pause", "in_tail_resume", "flb_tail_fs_inotify_pause", "flb_tail_fs_inotify_resume",
                         "flb_tail_fs_stat_pause", "flb_tail_fs_stat_resume", "flb_tail_fs_pause", "flb_tail_fs_resume"]},
    "C4": {"statement": "condvar publication",
           "functions": ["input_thread_instance_set_status", "input_thread_instance_get_status", "input_thread"]},
    "C5": {"statement": "join before release",
           "functions": ["flb_stop", "flb_destroy", "flb_input_thread_destroy", "flb_input_thread_instance_exit"]},
}

HEAD = re.compile(r"^(?!static\s+inline\s+\w+\s*;)[A-Za-z_][\w\s\*]*?\b([A-Za-z_]\w*)\s*\(")


def spans(path):
    with open(path, encoding="utf-8", errors="replace") as f:
        lines = f.read().split("\n")
    out = []
    i = 0
    while i < len(lines):
        line = lines[i]
        m = HEAD.match(line)
        if m and not line.rstrip().endswith(";") and not line.startswith(("#", "//", "/*", " ", "\t", "}")):
            name = m.group(1)
            j = i
            ok = False
            while j < min(i + 12, len(lines)):
                if lines[j].startswith("{"):
                    ok = True
                    break
                if lines[j].rstrip().endswith(";"):
                    break
                j += 1
            if ok and name not in ("if", "while", "for", "switch", "return", "sizeof"):
                k = j + 1
                while k < len(lines) and lines[k] != "}":
                    k += 1
                out.append((name, i + 1, k + 1))
                i = k + 1
                continue
        i += 1
    return out


def ranges(text):
    """`src/a.c:1-2, 3; include/x.h:4` -> [(path, a, b)]"""
    out = []
    for part in text.split(";"):
        part = part.strip()
        m = re.match(r"([\w/\.\-]+\.[ch]):(.*)", part)
        if not m:
            continue
        path, rest = m.group(1), m.group(2)
        for r in re.findall(r"(\d+)(?:-(\d+))?", rest):
            a = int(r[0]); b = int(r[1]) if r[1] else a
            out.append((path, a, b))
    return out


def main():
    sys_cls = json.load(open(sys.argv[1]))
    slice_cls = json.load(open(sys.argv[2]))
    inv = json.load(open(INVENTORY))
    obligations = {}

    def walk(o):
        if isinstance(o, dict):
            if isinstance(o.get("id"), str) and o["id"][:3] in ("OR-", "OB-"):
                obligations[o["id"]] = o
            for v in o.values():
                walk(v)
        elif isinstance(o, list):
            for v in o:
                walk(v)
    walk(inv)
    span_cache = {}
    result = {"inventory": "docs/reports/w45-fb-system-selection/lifetime-inventory.json",
              "slice_contracts": "docs/reports/w47-tail-pause-bindings/README.md section 5",
              "system": {}, "slice": {}}
    for oid in sorted(obligations, key=lambda s: (s[:2], int(s[3:]))):
        o = obligations[oid]
        cited = ranges(o.get("decidable_from", ""))
        funcs = set()
        for path, a, b in cited:
            if not path.endswith(".c"):
                continue
            if path not in span_cache:
                span_cache[path] = spans(os.path.join(SYSTEM, path))
            for name, s, e in span_cache[path]:
                if s <= b and a <= e:
                    funcs.add((path, name))
        entry = {"statement": o["statement"], "decidable_from": o.get("decidable_from"),
                 "needs": o.get("needs"), "functions": sorted(f"{p}::{n}" for p, n in funcs), "patterns": {}}
        for pat, p in sys_cls["patterns"].items():
            hit_rows = sorted({r["row"] for r in p["rows"] if any(r["row"].endswith("::" + n) and r["row"].startswith(fp) for fp, n in funcs)})
            hit_null = sorted({r["row"] for r in p["null_rows"] if any(r["row"].endswith("::" + n) and r["row"].startswith(fp) for fp, n in funcs)})
            inside = []
            for r in p["rows"]:
                for site in r["sites"]:
                    sp, _, sl = site.rpartition(":")
                    if any(sp == cp and a <= int(sl) <= b for cp, a, b in cited):
                        inside.append(site)
            if hit_rows or hit_null:
                entry["patterns"][pat] = {"custody_rows": hit_rows, "null_rows": hit_null,
                                          "sites_in_cited_ranges": sorted(set(inside))}
        result["system"][oid] = entry
    for cid, c in SLICE_OBLIGATIONS.items():
        entry = {"statement": c["statement"], "functions": c["functions"], "patterns": {}}
        for pat, p in slice_cls["patterns"].items():
            hit_rows = sorted({r["row"] for r in p["rows"] if r["row"].split("::", 1)[1] in c["functions"]})
            hit_null = sorted({r["row"] for r in p["null_rows"] if r["row"].split("::", 1)[1] in c["functions"]})
            if hit_rows or hit_null:
                entry["patterns"][pat] = {"custody_rows": hit_rows, "null_rows": hit_null}
        result["slice"][cid] = entry
    with open(sys.argv[3], "w", encoding="utf-8") as f:
        json.dump(result, f, indent=1, ensure_ascii=False)
        f.write("\n")
    for route in ("system", "slice"):
        for oid, e in result[route].items():
            print(route, oid, len(e["functions"]), "functions;", ", ".join(sorted(e["patterns"])) or "-")
    return 0


if __name__ == "__main__":
    sys.exit(main())
