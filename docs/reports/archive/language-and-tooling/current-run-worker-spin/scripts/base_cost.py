"""Estimate the base route's cross-unit import work from a tip run's progress.

usage: base_cost.py STDERR_LOG MAP_JSON CONTRACTS_JSON

At base every caller held every run home it did not define and replayed each
one (a whole-unit W4 analysis of the home) at certificate issue, and again at
the certificate check of a Safe caller. The time of one replay is taken as
the home unit's own analysis time in the run's first phase (`units under the
loaded contracts: done ... in Xs`). Prints the per-caller replay seconds,
the total and the share of the heaviest homes.
"""
import collections
import json
import re
import sys

log, map_path, contracts_path = sys.argv[1:4]
seconds = {}
callers = []
for line in open(log):
    m = re.match(r'progress: \[\d+s\] units under the loaded contracts: done \d+/\d+ (\S+) in ([0-9.]+)s', line)
    if m:
        seconds[m.group(1)] = max(seconds.get(m.group(1), 0.0), float(m.group(2)))
        callers.append(m.group(1))
home_of = {s['function']: s['home_unit'] for s in json.load(open(map_path))['symbols']}
imported = {} if contracts_path == '-' else json.load(open(contracts_path))
link = imported.get('assumptions', imported).get('cross_unit_link_resolution') or {}
exported = [row['function'] for row in link.get('imported', [])] or list(home_of)  # '-': every map name, an upper bound
homes = [(name, home_of[name]) for name in exported]
per_home = collections.Counter()
total = 0.0
for caller in callers:
    for name, unit in homes:
        if unit == caller:
            continue  # the caller defines it: local precedence
        cost = seconds.get(unit, 0.0)
        per_home[unit] += cost
        total += cost
phase_one = sum(seconds[c] for c in callers if c in seconds)
print(f'callers {len(callers)}; exported homes {len(homes)} in {len({u for _, u in homes})} units')
print(f'first phase (sum of unit times) {phase_one:.0f}s')
print(f'base import replays at issue (sum) {total:.0f}s = {total / 3600:.1f} h CPU; '
      f'{total / 3 / 3600:.1f} h on 3 workers (before certificate rechecks)')
for unit, cost in per_home.most_common(8):
    print(f'  {cost / 3600:6.1f} h  {unit} ({seconds.get(unit, 0):.0f}s per replay, '
          f'{sum(1 for _, u in homes if u == unit)} home(s))')
