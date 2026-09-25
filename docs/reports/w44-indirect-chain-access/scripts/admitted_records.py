#!/usr/bin/env python3
"""Card W44-INDIRECT-CHAIN-ACCESS: list every leaf the rule's recogniser admits.

Usage: admitted_records.py CENSUS_RUN CHAIN_CENSUS_JSON OUT_TXT

Reads `reach_leaves` from `chain_census.py`'s output and prints, per leaf, the
function status, the forecast arm (`clear`: every chain root never bound;
`flow`: some root the body may bind, which needs one live identity) and each
census record's text and chains, so the forecast can be checked by hand.
"""

import importlib.util
import json
import pathlib
import sys

HERE = pathlib.Path(__file__).resolve().parent
spec = importlib.util.spec_from_file_location("chain_census", HERE / "chain_census.py")
cc = importlib.util.module_from_spec(spec)
spec.loader.exec_module(cc)
census = cc.census


def main():
    run = pathlib.Path(sys.argv[1])
    reach = json.loads(pathlib.Path(sys.argv[2]).read_text())["reach_leaves"]
    _, functions, unit_keys = census.load_document(run / "resource-diagnostics.json")
    chains = cc.load_chain_stream(run / "chain-census.jsonl")
    indirect = census.load_stream(run / "indirect-census.jsonl", lambda record: census.INDIRECT)
    statements = census.load_stream(
        run / "statement-census.jsonl", lambda record: census.outcome_family(record["outcome"])
    )
    streams = [
        indirect,
        statements,
        {seq: {(f, l, None): [] for (f, l) in found} for seq, found in chains.items()},
    ]
    assignment = census.match_units(streams, unit_keys)
    lines = []
    for fkey, data in sorted(functions.items(), key=str):
        seq = assignment.get(fkey[:2])
        for line, description in data["leaves"]:
            name = "/".join(str(p) for p in fkey) + f":{line}:{description}"
            if name not in reach:
                continue
            lines.append(f"{data['status']:8} {reach[name]:5} {name}")
            for record in chains.get(seq, {}).get((fkey[2], line), []):
                if record["admitted"] is None:
                    continue
                roots = ", ".join(
                    f"{c['root']}{'' if c['dereferenced_root'] else '.'}"
                    f"[{'never-bound' if c['root_never_bound'] else 'may-bind'}]"
                    f"->{'->'.join(c['links'])}->{c['member']}"
                    for c in record["admitted"]
                )
                lines.append(f"    {' '.join(record['texts'])}")
                lines.append(f"      chains: {roots}")
    pathlib.Path(sys.argv[3]).write_text("\n".join(lines) + "\n")
    print(f"{sum(1 for l in lines if not l.startswith(' '))} leaves")
    return 0


if __name__ == "__main__":
    sys.exit(main())
