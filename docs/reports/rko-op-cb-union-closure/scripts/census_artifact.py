#!/usr/bin/env python3
"""census_artifact.py OUT.json -- RKO-OP-CB-RUN-SET-OPEN-THROUGH-UNION: the
census evidence in one file: the closures `member_facts_census` printed over
librdkafka's units at the base (`census-base`) and the card's code
(`census-after4`), the run's layout of `rd_kafka_op_s.rko_u.*` as the units
that probed it give it (all equal), and the arm-site sizing summary."""
import collections
import json
import pathlib
import subprocess
import sys

PRIVATE = pathlib.Path(__file__).resolve().parents[4] / "private/rkou"
HERE = pathlib.Path(__file__).resolve().parent


def closures(log):
    out = {}
    for line in (PRIVATE / log).read_text().splitlines():
        for prefix in ("rd_kafka_op_s.", "rd_list_s.", "rd_kafka_timer_s."):
            if line.startswith(prefix):
                member, rest = line.split(": ", 1)
                out[member] = rest
    return out


facts = json.loads((PRIVATE / "census-after4.json").read_text())
layouts = collections.defaultdict(list)
for unit, data in facts.items():
    layout = data.get("union_layouts", {}).get("rd_kafka_op_s.rko_u.*") if isinstance(data, dict) else None
    if layout:
        layouts[json.dumps(layout, sort_keys=True)].append(unit.split("/src/")[-1])
sizing = subprocess.run([sys.executable, str(HERE / "arm_sites_summary.py"), str(PRIVATE / "arm-sites.json")],
                        capture_output=True, text=True, check=True).stdout.splitlines()
json.dump({
    "base_closures": closures("census-base.log"),
    "card_closures": closures("census-after4.log"),
    "rko_u_layouts": [{"units": units, "layout": json.loads(text)} for text, units in layouts.items()],
    "arm_site_sizing": sizing,
}, open(sys.argv[1], "w"), indent=1)
print(len(layouts), "distinct layout(s) of rko_u over", sum(len(u) for u in layouts.values()), "unit(s)")
