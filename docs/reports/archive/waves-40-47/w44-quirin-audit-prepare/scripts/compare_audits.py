#!/usr/bin/env python3
"""Compare the `quirin-audit current` runs over two preparations.

Card W44-QUIRIN-AUDIT-PREPARE. LEFT and RIGHT are preparation output roots;
each has `reports/` from the same `current` arguments. Every report file is
compared by bytes, then after replacing LEFT's path with RIGHT's. The console
output of each run is compared the same way, apart from the `preparation:`
line, which sums per-unit wall time (docs/agents/validation.md). The RIGHT
console is the `prepare --run` output, whose leading lines are preparation's
printed paths.

Usage: compare_audits.py LEFT RIGHT LEFT_CONSOLE RIGHT_CONSOLE OUT.json
"""
import hashlib
import json
from pathlib import Path
import sys


def main():
    left, right, left_console, right_console, out = (Path(a) for a in sys.argv[1:6])
    lb, rb = str(left).encode(), str(right).encode()
    result = {'normalisation': f'replace {left} with {right}', 'reports': {}}
    names = sorted({p.name for p in (left / 'reports').iterdir()} | {p.name for p in (right / 'reports').iterdir()})
    for name in names:
        a, b = left / 'reports' / name, right / 'reports' / name
        if not (a.is_file() and b.is_file()):
            result['reports'][name] = 'missing on one side'
            continue
        x, y = a.read_bytes(), b.read_bytes()
        state = 'identical' if x == y else (
            'identical_after_normalisation' if x.replace(lb, rb) == y else 'different')
        result['reports'][name] = {'state': state, 'right_sha256': hashlib.sha256(y).hexdigest(),
                                   'bytes': len(y)}
    keep = lambda text: [line for line in text.splitlines() if not line.startswith('preparation:')]
    x = keep(left_console.read_bytes().replace(lb, rb).decode())
    y = keep(right_console.read_text())
    # Preparation prints its three output paths before the audit starts.
    printed = ('/cmetrics-audit.json.config', '/compile_commands.json', '/prepare.json')
    prefix = [line for line in y[:3] if line.startswith('/') and line.endswith(printed)]
    result['right_console_preparation_lines'] = len(prefix)
    result['console'] = 'identical_after_normalisation' if x == y[len(prefix):] else 'different'
    summary = [line for line in y if line.startswith('current revision:')]
    result['summary'] = summary
    Path(out).write_text(json.dumps(result, indent=2) + '\n')
    states = [v['state'] if isinstance(v, dict) else v for v in result['reports'].values()]
    print(json.dumps({'reports': len(states), 'states': sorted(set(states)), 'console': result['console'],
                      'summary': summary}))


if __name__ == '__main__':
    main()
