#!/usr/bin/env python3
"""Rank the census sites (census.py output) by root shape and first blocker.

usage: census_rank.py CORPUS_SITES.json SLICE_SITES.json OUT.json

Shapes (read against the source; `OVERRIDES` records each site whose lexical
classification the reading corrected):

unlink entry reach
  U0 an object this body published (10d's restore)      -> admitted (10d)
  U1 element of a container rooted at an unrebound formal (or a local
     initialised from one): the current element of a safe iteration, or a
     first/last lookup                                   -> admitted
  U2 element of a static-storage container               -> admitted (P-CRR-STATIC)
  U3 element of a container a callee returned            -> refused
  U4 element of a local container object                 -> refused
  U5 element of a container rooted at another element    -> refused
  U6 a formal's own object, its container unnamed        -> refused
  U7 an embedded array element (&P->a[i])                -> refused
publication container root
  C1 unrebound formal chain / local initialised from one -> admitted (10d)
  C2 static-storage object                               -> admitted (P-CRR-STATIC)
  C3 a body-acquired owner that is published or returned -> refused
  C4 a container a callee returned                       -> refused
  C5 a local object or a body-acquired owner kept local  -> refused
  C6 an element                                          -> refused

first blocker: the leaf the audit reports at the site's line, in the order
  10d-container, 10d-entry (not one live object / owns linked objects),
  10d-unlink, identity (unresolved identity at the site), call-refused (the
  whole call refused before its effects), none.
"""
import collections
import json
import sys

OVERRIDES = {
    # an unlink of the object this body published a few lines earlier (10d's own case)
    ("cio_chunk.c", 89): "U0",
    # `task` is allocated by flb_task_create, published and returned
    ("src/flb_task.c", 718): "C3",
    ("src/flb_task.c", 822): "C3",
    ("src/flb_task.c", 870): "C3",
    # formal entries the lexical rebinding reader flagged as rebound
    ("src/flb_task.c", 282): "U6",
    ("src/flb_task.c", 924): "U6",
    # `&task->routes`: `task` is flb_task_destroy's formal; the call is refused first
    ("src/flb_task.c", 918): "U1",
    # the second loop's head is the local list `failed_requests`
    ("src/flb_scheduler.c", 226): "U4",
    # `list` is the TLS list; mk_vhost_fdt_worker_init allocates and publishes it
    ("mk_server/mk_vhost.c", 111): "C5",
    # the section is the last element of `conf->sections`
    ("mk_core/mk_rconf.c", 95): "C6",
    # `plugin` is a loaded plugin table the body does not own
    ("mk_server/mk_plugin.c", 222): "C4",
    ("mk_server/mk_plugin.c", 229): "C4",
    ("mk_server/mk_plugin.c", 236): "C4",
    ("mk_server/mk_plugin.c", 243): "C4",
    ("mk_server/mk_plugin.c", 250): "C4",
    # the request is embedded in the session (`&cs->sr_fixed`) or a lookup
    ("mk_server/mk_http.c", 268): "C1",
    ("mk_server/mk_http.c", 1613): "C4",
    # the head of a whole list being reinitialised (`&cs->request_list`)
    ("mk_server/mk_http.c", 1444): "U6",
    # chunkio's qsort helper unlinks and relinks elements of a formal's list
    ("chunkio.c", 246): "U1",
    # mk_plugin_exit_all is a plain iteration: rule 5l refuses its unlink
    ("mk_server/mk_plugin.c", 552): "U1-plain",
}

UNLINK = {
    "iteration-safe": "U1", "lookup-first": "U1", "formal-entry": "U6", "rebound-formal": "U6",
    "embedded": "U7",
}
HEAD = {"formal": "U1", "derived-formal": "U1", "global": "U2", "callee-result": "U3",
        "local-object": "U4", "element": "U5"}
CONTAINER = {"formal": "C1", "derived-formal": "C1", "global": "C2", "owner-published": "C3",
             "owner-returned": "C3", "callee-result": "C4", "local-object": "C5",
             "owner-local": "C5", "element": "C6"}
ADMITTED = {"U0", "U1", "U2", "C1", "C2"}


def shape(site):
    key = (site["unit"], site["line"])
    if key in OVERRIDES:
        return OVERRIDES[key]
    if site["call"] == "mk_list_del":
        reach = site.get("entry_reach", "")
        if "iteration-plain" in reach:
            return "U1-plain"
        if reach in ("iteration-safe", "lookup-first") or reach.startswith("iteration-safe"):
            heads = site.get("head_shape") or ["?"]
            shapes = {HEAD.get(head, "U?") for head in heads}
            return shapes.pop() if len(shapes) == 1 else "U?:" + ",".join(sorted(shapes))
        return UNLINK.get(reach, "U?")
    return CONTAINER.get(site.get("shape"), "C?")


def blocker(site):
    leaves = " | ".join(site["leaves"])
    if "does not show outlives" in leaves:
        return "10d-container"
    if "does not hold as one live object" in leaves or "owns linked objects" in leaves:
        return "10d-entry"
    if "unlink by" in leaves:
        return "10d-unlink"
    if "unresolved resource identity" in leaves:
        return "identity"
    if leaves:
        return "call-refused"
    return "none"


def main():
    out = {"sites": [], "by_shape": {}}
    for scope, path in (("corpus", sys.argv[1]), ("slice", sys.argv[2])):
        seen = set()
        for site in json.load(open(path)):
            key = (site["unit"], site["line"])
            if key in seen:
                continue
            seen.add(key)
            row = {
                "scope": scope,
                "site": f"{site['unit']}:{site['line']}",
                "function": site["function"],
                "call": site["call"],
                "arguments": site["arguments"],
                "shape": shape(site),
                "first_blocker": blocker(site),
                "status": site["status"],
                "leaves": site["leaves"],
            }
            row["admitted_after"] = row["shape"] in ADMITTED
            out["sites"].append(row)
    table = collections.defaultdict(lambda: {"sites": 0, "functions": set(), "blockers": collections.Counter()})
    for row in out["sites"]:
        entry = table[(row["scope"], row["shape"])]
        entry["sites"] += 1
        entry["functions"].add(row["function"])
        entry["blockers"][row["first_blocker"]] += 1
    for (scope, name), entry in sorted(table.items()):
        out["by_shape"].setdefault(scope, {})[name] = {
            "sites": entry["sites"],
            "functions": len(entry["functions"]),
            "function_names": sorted(entry["functions"]),
            "first_blockers": dict(entry["blockers"]),
        }
    json.dump(out, open(sys.argv[3], "w"), indent=1, sort_keys=True)
    for scope, shapes in out["by_shape"].items():
        print(f"== {scope}")
        ranked = sorted(shapes.items(), key=lambda item: -item[1]["functions"])
        for name, entry in ranked:
            print(f"  {name:10} sites {entry['sites']:3} functions {entry['functions']:3} {entry['first_blockers']}")


main()
