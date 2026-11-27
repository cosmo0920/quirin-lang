"""Measure what dropping `-O3` would have done to the libevent-core audit.

The archived G03 attempt dropped every option the gateway could not transport,
including `-O3`. This runs the same audit twice over the same sources: once
with CMake's unmodified command, once with `-O3` removed, and compares the
published verdicts and the recorded preprocessing digests.
"""
import json
import subprocess
from pathlib import Path

root = Path('/home/cosmo/Documents/GitHub/qcore-c/.claude/worktrees/agent-a819c36ccc835c1d4')
build = root / 'build/cycle19/libevent-clang-build'
scratch = root / 'build/cycle19/o3-effect'
scratch.mkdir(parents=True, exist_ok=True)

database = json.loads((build / 'compile_commands.json').read_text())
for entry in database:
    entry['command'] = entry['command'].replace(' -O3 ', ' ')
(scratch / 'compile_commands.json').write_text(json.dumps(database, indent=2))

config = json.loads((root / 'contract/profiles/libevent-core/audit.json').read_text())
config['profile_directory'] = str(root / 'contract/profiles/libevent-core')
config['checkout'] = str(root / 'third_party/libevent')
config['compile_database'] = str(scratch / 'compile_commands.json')
config['report'] = str(scratch / 'report.json')
(scratch / 'audit.json').write_text(json.dumps(config, indent=2))

result = subprocess.run(
    ['cargo', 'run', '--quiet', '-p', 'qcore-audit', '--', str(scratch / 'audit.json')],
    cwd=root, capture_output=True, text=True)
print('exit', result.returncode)
print(result.stdout.strip().splitlines()[-1] if result.stdout.strip() else result.stderr[-400:])

with_o3 = json.loads((build / 'libevent-core-audit.json').read_bytes())
without = json.loads((scratch / 'report.json').read_bytes())
a = {u['file']: u['status'] for u in with_o3['units']}
b = {u['file']: u['status'] for u in without['units']}
print('verdict differences:', {k: (a[k], b.get(k)) for k in a if a[k] != b.get(k)})
print('with -O3   diagnostics:', with_o3['diagnostic_counts'])
print('without -O3 diagnostics:', without['diagnostic_counts'])

pa = json.loads((build / 'libevent-core-audit.assumptions.json').read_bytes())
pb = json.loads((scratch / 'report.assumptions.json').read_bytes())
da = {t['file']: t['identity']['preprocessed_source_sha256'] for t in pa['compile_tasks']}
db_ = {t['file']: t['identity']['preprocessed_source_sha256'] for t in pb['compile_tasks']}
changed = [f for f in da if da[f] != db_.get(f)]
print(f'units whose preprocessed text changed: {len(changed)} of {len(da)}')
