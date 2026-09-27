#!/usr/bin/env python3
"""Run the same bounded Fluent Bit selection with a chosen audit binary."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import time

p = argparse.ArgumentParser()
p.add_argument('--binary', type=Path, required=True)
p.add_argument('--label', required=True)
p.add_argument('--jobs', type=int, default=1)
p.add_argument('--source', action='append')
p.add_argument('--case', default='flb-task')
p.add_argument('--prepared', type=Path, required=True)
a = p.parse_args()
root = Path.cwd()
base = root / 'private/audit-memory-performance'
run = base / a.label
run.mkdir(parents=True, exist_ok=False)
selection = base / ('selection.json' if a.case == 'flb-task' else 'selection-'+a.case+'.json')
selection.write_text(json.dumps({'schema_version': 1, 'name': 'audit-performance-'+a.case, 'groups': [{'library': 'fluent_bit', 'label': 'fluent-bit-task', 'files': a.source or ['src/flb_task.c']}]}))
cmd = [str(a.binary.resolve()), 'current', '--checkout', str(a.prepared / 'fluent-bit-clang-source'), '--profile-directory', str(root / 'contract/profiles/fluent-bit'), '--compile-database', str(a.prepared / 'fluent-bit-clang-production/compile_commands.json'), '--selection', str(selection), '--contract-manifest', str(root / 'cycle19-resource-contracts.json'), '--report', str(run / 'resource-diagnostics.json'), '--jobs', str(a.jobs)]
env = dict(os.environ, QCORE_CLANG='/usr/lib/llvm-21/bin/clang')
for key in ('QCORE_TYPED_PATH_PARITY', 'QCORE_INDIRECT_CENSUS', 'QCORE_C11_CENSUS', 'QCORE_ACCESS_PATH_CENSUS'):
    env.pop(key, None)
record = {'command': cmd, 'cwd': str(root), 'binary_sha256': hashlib.sha256(a.binary.read_bytes()).hexdigest(), 'source_tip': subprocess.check_output(['git', 'rev-parse', 'HEAD'], text=True).strip(), 'jobs': a.jobs, 'started_unix': time.time()}
(run / 'run.json').write_text(json.dumps(record, indent=2)+'\n')
with (run / 'stdout.log').open('wb') as out, (run / 'stderr.log').open('wb') as err:
    result = subprocess.run(['/usr/bin/time', '-f', 'wall_s=%e\nuser_s=%U\nsystem_s=%S\nmax_rss_kib=%M\nexit=%x', '-o', str(run / 'time.txt'), *cmd], env=env, stdout=out, stderr=err)
record.update(exit=result.returncode, finished_unix=time.time())
(run / 'run.json').write_text(json.dumps(record, indent=2)+'\n')
print(json.dumps(record, indent=2))
print((run / 'time.txt').read_text())
raise SystemExit(result.returncode)
