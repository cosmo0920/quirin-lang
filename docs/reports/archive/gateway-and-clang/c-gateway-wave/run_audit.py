#!/usr/bin/env python3
"""Run the canonical pinned 185-unit audit using a captured audit executable.

Preparation (once, from the repository root):
  python3 -B scripts/prepare_cycle19_clang_audit.py \
    --fluent-bit /home/cosmo/Documents/GitHub/fluent-bit \
    --revision 66910c10a4d7eafa810b84229cf2dcf7b0f26f97 \
    --output build/cycle19/c-gateway-wave-pin \
    --clang /usr/bin/clang --clangxx /usr/bin/clang++
Then run:
  python3 docs/reports/c-gateway-wave/run_audit.py STAGE AUDIT_BINARY
Select an unchanged verified ChunkIO 0.13.0 package with --package as needed.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess

ROOT = Path(__file__).resolve().parents[3]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('stage')
    parser.add_argument('binary', type=Path)
    parser.add_argument('--package', type=Path, default=ROOT / 'build/cycle19/provider-packages-wave20/qcore.chunkio')
    args = parser.parse_args()
    pin = ROOT / 'build/cycle19/c-gateway-wave-pin'
    source = pin / 'fluent-bit-clang-source'
    database = pin / 'fluent-bit-clang-production/compile_commands.json'
    target = source / 'build/compile_commands.json'
    target.parent.mkdir(exist_ok=True)
    if target.exists():
        assert target.read_bytes() == database.read_bytes(), 'source database differs from this pin'
    else:
        shutil.copyfile(database, target)
    out = ROOT / 'measure/c-gateway-wave' / args.stage
    out.mkdir(parents=True, exist_ok=True)
    binary = args.binary.resolve(strict=True)
    command = [str(binary), '--pinned-evidence', str(source),
               str(pin / 'monkey/monkey-build/compile_commands.json'),
               str(out / 'monkey-source.json'), str(out / 'resource-diagnostics.json')]
    receipt = {'command': command, 'binary_sha256': hashlib.sha256(binary.read_bytes()).hexdigest(),
               'compile_database_sha256': hashlib.sha256(database.read_bytes()).hexdigest(),
               'compile_database_entries': len(json.loads(database.read_text())),
               'head': subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=ROOT, text=True).strip(),
               'package': str(args.package.resolve()), 'source_input': 'HistoricalStdin'}
    with (out / 'audit.log').open('w') as log:
        result = subprocess.run(command, cwd=ROOT, stdout=log, stderr=subprocess.STDOUT,
                                env=dict(os.environ, QCORE_CLANG='/usr/bin/clang',
                                         QCORE_CHUNKIO_PACKAGE=str(args.package.resolve())))
    receipt['exit'] = result.returncode
    (out / 'run.json').write_text(json.dumps(receipt, indent=2) + '\n')
    raise SystemExit(result.returncode)


if __name__ == '__main__':
    main()
