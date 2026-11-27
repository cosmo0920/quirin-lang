#!/usr/bin/env python3
"""summarize.py LAYOUT_DIR CENSUS_DIR OUT.json -- FB-PROCESSOR-INSTANCE-ALLOCATION-SIZE:
join census.py's member accesses with layout.py's member extents per
configuration and write the report artifact:

* per configuration: sizes, units read, accesses by member with the member's
  end offset and whether it lies within sizeof(struct flb_filter_instance)
  (an access past it is marked `outside`);
* the access table of the allocation's own path: flb_processor_instance_create
  (src/flb_processor.c:2144-2212 at 046accf6 and 66910c10), its three
  callers' sites and the unit lifecycle (flb_processor.c), by file and line;
* every conversion into or out of `struct flb_processor_instance *`, grouped
  by (from, to, cast kind) with its sites, any non-void, non-self type marked.
"""
import collections
import json
import os
import sys

layout_dir, census_dir, out = sys.argv[1:]
layouts = json.load(open(os.path.join(layout_dir, "layouts.json")))
result = {"configurations": []}
for config in ("corpus-66910c10", "system-046accf6", "pins-046accf6", "slice-34155955"):
    rows = [r for r in layouts if r["config"] == config]
    census = json.load(open(os.path.join(census_dir, f"{config}.census.json")))
    extent = {m["name"]: m for m in rows[0]["processor_members"]}
    filter_size = min(r["filter_sizeof"] for r in rows)
    by_member = collections.Counter()
    outside, table, conversions = [], [], collections.defaultdict(list)
    for unit in census["units"]:
        for a in unit["accesses"]:
            m = extent[a["member"]]
            by_member[a["member"]] += 1
            if m["end"] > filter_size:
                outside.append({**a, "unit": unit["unit"]})
            if unit["unit"] in ("src/flb_processor.c", "src/flb_help.c", "src/fluent-bit.c") and \
                    a["file"] == unit["unit"]:
                table.append({"file": a["file"], "line": a["line"], "member": a["member"],
                              "use": a["use"], "offset": m["layout_offset"], "end": m["end"]})
        for c in unit["conversions"]:
            key = (c["from"], c["to"], c["cast"])
            conversions[key].append(f"{c['file']}:{c['line']}" + (f" ({c['site']})" if c.get("site") and not str(c["file"]).endswith(".c") else ""))
    table = sorted({json.dumps(t, sort_keys=True) for t in table})
    table = sorted((json.loads(t) for t in table), key=lambda t: (t["file"], t["line"], t["member"]))
    conv = []
    for (src, dst, kind), sites in sorted(conversions.items()):
        other = dst if src == "struct flb_processor_instance *" else src
        conv.append({"from": src, "to": dst, "cast": kind, "count": len(sites),
                     "other_type_is_void_or_self": other in ("void *", "void", "struct flb_processor_instance *"),
                     "sites": sorted(set(sites))})
    result["configurations"].append({
        "config": config, "revision": rows[0]["revision"],
        "variants": [{"variant": r["variant"], "processor_sizeof": r["processor_sizeof"],
                      "filter_sizeof": r["filter_sizeof"]} for r in rows],
        "units_read": len(census["units"]),
        "units_failed": [u["unit"] for u in census["units"] if u["exit"] != 0],
        "units_without_record": [u["unit"] for u in census["units"] if not u["record_found"]],
        "accesses": sum(by_member.values()),
        "accesses_by_member": {k: {"count": by_member[k], "offset": extent[k]["layout_offset"],
                                   "end": extent[k]["end"]} for k in extent},
        "max_member_end": max(m["end"] for m in extent.values()),
        "outside_filter_sizeof": outside,
        "allocation_path_table": table,
        "conversions": conv,
    })
with open(out, "w") as handle:
    json.dump(result, handle, indent=1)
    handle.write("\n")
for c in result["configurations"]:
    print(c["config"], c["revision"][:8], "units", c["units_read"], "failed", len(c["units_failed"]),
          "no-record", len(c["units_without_record"]), "accesses", c["accesses"],
          "outside", len(c["outside_filter_sizeof"]), "max end", c["max_member_end"],
          "conversion kinds", len(c["conversions"]),
          "non-void/self", sum(1 for x in c["conversions"] if not x["other_type_is_void_or_self"]))
