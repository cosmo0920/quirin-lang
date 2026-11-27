"""Summarise experiment runs: run seconds (wall minus slot wait), peak RSS,
exit, and the progress phases when present.

usage: runs.py RUN_DIR...
"""
import json
import os
import re
import sys

for run in sys.argv[1:]:
    def read(name):
        try:
            return open(os.path.join(run, name)).read().strip()
        except OSError:
            return None
    wall = read('wall-seconds.txt')
    wait = json.loads(read('slot-wait.json') or '{}').get('waited_seconds', 0)
    receipt = json.loads(read('slot-receipt.json') or '{}')
    peak = receipt.get("max_rss_kib")
    if peak is None:
        for key, value in receipt.items():
            if 'rss' in key and isinstance(value, (int, float)):
                peak = value
    seconds = None if wall is None else int(wall) - round(wait)
    phases = []
    for line in (read('stderr.log') or '').splitlines():
        m = re.match(r'progress: \[\d+s\] (.*): end, .* in ([0-9.]+)s', line)
        if m:
            phases.append(f'{m.group(1)} {float(m.group(2)):.0f}s')
    print(f'{os.path.basename(run)}: exit {read("exit.txt")} run {seconds}s '
          f'(wait {wait:.0f}s) peak {peak} | ' + '; '.join(phases))
