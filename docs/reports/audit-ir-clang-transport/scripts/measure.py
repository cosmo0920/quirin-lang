#!/usr/bin/env python3
"""Generate a 30,000-definition control and measure both transport modes.

Run from the repository root with a built clang_transport_measure example.
Each run has a fresh directory. Build/setup is excluded. File cache is not
controlled; the shared advisory lock serializes cooperating heavy jobs only.
"""
import argparse
import fcntl
import hashlib
import json
from pathlib import Path
import subprocess
import sys


def digest(path):
    with path.open('rb') as f:
        return hashlib.file_digest(f, 'sha256').hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--binary', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=False)
    spool = output / 'spool'
    spool.mkdir()
    source = output / 'large.c'
    source.write_text('struct S { int x; int y; };\n' + ''.join(
        f'int f{i}(struct S *p, int x) {{ p->x += x; p->y = p->x * 3; if (x > 4) return p->y; return x + p->x; }}\n'
        for i in range(30000)))
    results = []
    with open('/tmp/quirin-audit-ir-next-heavy.lock', 'a') as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        for mode, storage in [('before', 'disk'), ('after', 'disk'), ('before', 'tmpfs'), ('after', 'tmpfs')]:
            name = mode + '-' + storage
            config = output / (name + '.json')
            config.write_text(json.dumps(dict(mode=mode, source=str(source),
                output=str(output / (name + '.qc')),
                temporary_directory=str(spool) if storage == 'disk' else '/tmp'), indent=2) + '\n')
            command = [sys.executable, 'docs/reports/audit-ir-admission/scripts/measure_memory.py',
                '--binary', str(args.binary.resolve()), '--config', str(config),
                '--output', str(output / name), '--jobs', '1', '--timeout', '900']
            result = subprocess.run(command)
            results.append(dict(name=name, argv=command, exit=result.returncode))
    records = {name: digest(output / (name + '.qc')) for name in [r['name'] for r in results]
               if (output / (name + '.qc')).exists()}
    receipt = dict(runs=results, source_sha256=digest(source), full_ast_sha256=records,
                   all_records_equal=len(records) == 4 and len(set(records.values())) == 1)
    (output / 'comparison.json').write_text(json.dumps(receipt, indent=2) + '\n')
    return 0 if all(r['exit'] == 0 for r in results) and receipt['all_records_equal'] else 1


if __name__ == '__main__':
    raise SystemExit(main())
