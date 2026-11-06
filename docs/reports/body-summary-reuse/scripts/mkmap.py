"""EXPERIMENT-ONLY link map for the BODY-SUMMARY-REUSE census.

usage: mkmap.py REPORT CENSUS_JSON OUT_MAP

Names, for every callee of the census population (classify.py's output
`sites`), its one defining configuration in REPORT as the home. This is not
a premise adoption: the map exists to ask rule 4c's existing route what it
would do with each home, so the census can tell "no summary exportable" from
"summary exportable but no link premise on this route". Nothing here is
evidence that a linker chose these definitions.
"""
import json, sys, collections

report, census, out = sys.argv[1:4]
d = json.load(open(report))
homes = collections.defaultdict(set)
for u in d['units']:
    for f in u.get('functions', []):
        homes[f['function']].add(f"{u['library']}/{u['file']}")
population = sorted({s['callee'] for s in json.load(open(census))['sites']})
symbols = [{"function": n, "home_unit": next(iter(homes[n]))}
           for n in population if len(homes.get(n, ())) == 1]
json.dump({
    "schema_version": 1,
    "premise": "EXPERIMENT ONLY (card BODY-SUMMARY-REUSE census, not adopted): for the selected linked program and each selected compilation configuration, every external call to each listed symbol has a compatible function type and calling convention and resolves to its listed external definition.",
    "symbols": symbols,
}, open(out, 'w'), indent=1)
print(len(symbols), 'symbols;', len(population) - len(symbols), 'without a unique home')
