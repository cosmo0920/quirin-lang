"""Residual-name discovery (`reconciliation_sweep.untracked_residuals`) before and after.

Builds a documents-only root for the base revision in the given scratch
directory (increment records, rule pages, platform evidence, the matrix), and
compares its discovery with the working tree's. No card report is named, so
the sources are the increment records and the mention-only rule documents.
"""
import json
import subprocess
import sys
from pathlib import Path

sys.path.insert(0, 'scripts')
import reconciliation_sweep as sweep  # noqa: E402

BASE, scratch = '8c1f15eb', Path(sys.argv[1])
paths = subprocess.check_output(['git', 'ls-tree', '-r', '--name-only', BASE]).decode().split('\n')
want = [p for p in paths if p.startswith(('docs/development/increments/', 'docs/agents/'))
        or p in ('AGENTS.md', 'CLAUDE.md', 'docs/development/platform-evidence.md',
                 'docs/reviews/r0_3/acceptance_matrix.json')]
for p in want:
    target = scratch / p
    target.parent.mkdir(parents=True, exist_ok=True)
    target.write_bytes(subprocess.check_output(['git', 'show', f'{BASE}:{p}']))
before = {r.name: r for r in sweep.untracked_residuals(scratch, [])}
after = {r.name: r for r in sweep.untracked_residuals(Path('.'), [])}
print(f'before {len(before)} names, after {len(after)} names')
print('lost:', sorted(set(before) - set(after)))
print('new:', sorted(set(after) - set(before)))
moved = [n for n in before if n in after and before[n].mentions != after[n].mentions]
print('mention sets changed:', len(moved))
for n in sorted(moved):
    print(' ', n, before[n].mentions, '->', after[n].mentions)
