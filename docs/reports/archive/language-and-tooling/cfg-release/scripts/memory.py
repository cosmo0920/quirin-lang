#!/usr/bin/env python3
"""Summarise CFG-RELEASE memory observations (cgmeasure.sh outputs).

Usage: memory.py RUNS_DIR ROUTE [ROUTE...]  -> JSON on stdout

Per side and route: the audit's exit, the driver's exit, /usr/bin/time -v's
maximum resident set size (the largest single waited-for process, the
quirin-audit process) and elapsed wall time, and the systemd scope's
memory.peak, memory.swap.peak, memory.events and memory.pressure (PSI totals)
read just before the scope ended. Read-only.
"""
import json
import re
import sys
from pathlib import Path


def time_fields(path):
    text = path.read_text()
    rss = int(re.search(r'Maximum resident set size \(kbytes\): (\d+)', text).group(1))
    elapsed = re.search(r'Elapsed \(wall clock\) time \(h:mm:ss or m:ss\): (\S+)', text).group(1)
    parts = [float(p) for p in elapsed.split(':')]
    seconds = sum(value * 60 ** index for index, value in enumerate(reversed(parts)))
    status = int(re.search(r'Exit status: (\d+)', text).group(1))
    return {'max_rss_kib': rss, 'max_rss_bytes': rss * 1024, 'elapsed_seconds': seconds,
            'time_exit_status': status}


def cgroup_fields(path):
    sections, current = {}, None
    for line in path.read_text().splitlines():
        if line.startswith('== '):
            current = line[3:]
            sections[current] = []
        elif current is None:
            key, _, value = line.partition(' ')
            sections[key] = value
        else:
            sections[current].append(line)
    events = dict(line.split() for line in sections['memory.events'])
    pressure = {}
    for line in sections['memory.pressure']:
        kind, *fields = line.split()
        pressure[kind] = dict(field.split('=') for field in fields)
    stat = dict(line.split() for line in sections['memory.stat'])
    return {'exit': int(sections['exit']), 'cgroup': sections['cgroup'],
            'memory_peak_bytes': int(sections['memory.peak'][0]),
            'memory_swap_peak_bytes': int(sections['memory.swap.peak'][0]),
            'memory_max': sections['memory.max'][0],
            'memory_events': {key: int(value) for key, value in events.items()},
            'memory_pressure': pressure,
            'memory_stat_at_end': {key: int(stat[key]) for key in ('anon', 'file', 'kernel') if key in stat}}


def main():
    runs = Path(sys.argv[1])
    out = {}
    for route in sys.argv[2:]:
        for side in ('base', 'tip'):
            record = {'driver_exit': int((runs / f'{side}-{route}.driver.exit').read_text())}
            invocation = json.loads((runs / side / route / 'invocation.json').read_text())
            record['audit_exit'] = invocation.get('audit_exit', invocation.get('exit'))
            record['binary_sha256'] = invocation['binary_sha256']
            record['jobs'] = invocation['jobs']
            record.update(time_fields(runs / f'{side}-{route}.mem.time'))
            record.update(cgroup_fields(runs / f'{side}-{route}.mem.cgroup'))
            out[f'{side}/{route}'] = record
        base, tip = out[f'base/{route}'], out[f'tip/{route}']
        out[f'delta/{route}'] = {
            'max_rss_bytes_tip_minus_base': tip['max_rss_bytes'] - base['max_rss_bytes'],
            'max_rss_relative': (tip['max_rss_bytes'] - base['max_rss_bytes']) / base['max_rss_bytes'],
            'memory_peak_bytes_tip_minus_base': tip['memory_peak_bytes'] - base['memory_peak_bytes'],
        }
    print(json.dumps(out, indent=1))


if __name__ == '__main__':
    main()
