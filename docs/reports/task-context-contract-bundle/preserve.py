#!/usr/bin/env python3
"""Check byte preservation and explicit destinations for all old operation bullets."""
import hashlib
import json
from pathlib import Path
import re
import subprocess
ROOT = Path(__file__).resolve().parents[3]
OUT = Path(__file__).resolve().parent
base = json.loads((OUT/'baseline.json').read_text())['base']
raw = (OUT/'operations-before.txt').read_bytes()
assert raw == subprocess.check_output(['git','-C',str(ROOT),'show',f'{base}:docs/agents/operations.md'])
text = raw.decode(); current = (ROOT/'docs/agents/operations.md').read_text()
starts = [m.start() for m in re.finditer(r'^- ', text, re.M)]
claims = [
 ['Preserve fail-closed gates', 'Update'],
 ['Preserve Safe/Unsafe/Unknown'],
 ['Run the full [standing gate]', 'incomplete attempts'],
 ['Read [generalization]', 'target crate'],
 ['A card closes with Rust', 'Audit*.lean'],
 ['Add exactly one increment paragraph', 'primary evidence'],
 ['Compact AGENTS.md, CLAUDE.md and README.md every 2–3 waves', 'move unique history verbatim'],
 ['Reevaluate stability every two waves', 'discharge condition', 'into its brief'],
 ['Work every card\'s integrator list', 'reconciliation_sweep.py', 'unverifiable', 'never silently soften'],
 ['Confirm a retired name was actually registered', 'before and after removal'],
 ['Prefer temporary WIP commits'],
 ['worktree-private', 'Every run gets', 'disk-exhausted'],
 ['only after its final report'],
 ['Restore mutation inputs by rewriting bytes', 'touch', 'green baseline after'],
 ['Delegated cards', 'Integrate cards one at a time', 'Publish a mutation count only', 'integration-gate.json'],
 ['A delegated card never advances main', 'on that command only', 'Only the integrator installs', 'pack-refs'],
 ['Kill only captured PIDs', 'never `pkill`/`pgrep`'],
 ['never end with an unchanged tree', 'at least one commit ahead'],
]
assert len(starts) == len(claims)
normal = ' '.join(current.split())
rows=[]
for n, (start, required) in enumerate(zip(starts, claims)):
    end = starts[n+1] if n+1 < len(starts) else len(text)
    for phrase in required:
        assert ' '.join(phrase.split()) in normal, phrase
    rows.append({'old_start_line':text[:start].count('\n')+1,'old_heading':text[start:].splitlines()[0],
                 'old_block_sha256':hashlib.sha256(text[start:end].encode()).hexdigest(),
                 'current_file':'docs/agents/operations.md','required_phrases':required})
(OUT/'preservation.json').write_text(json.dumps({'base':base,'checkpoint_sha256':hashlib.sha256(raw).hexdigest(),
    'old_operation_bullets':len(rows),'rows':rows,
    'boundary':'Byte preservation and quoted operative destinations, not an automated proof of natural-language equivalence. Policy scoping deliberately changes; old narrative remains historical.'},indent=2)+'\n')
print(f'Preserved {len(raw)} bytes; {len(rows)} operation bullets have checked current destinations')
