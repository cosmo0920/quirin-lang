#!/usr/bin/env python3
"""STATUS-COMPACT: inventory docs/STATUS.md before (REV) and after (the working tree).

Usage: inventory.py REV

Checks, each printed with its counts:
  1. the checkpoint reproduces REV's page (delegates to checkpoint.py's check);
  2. every UPPER-KEBAB name, report pointer, backticked revision and bolded
     figure on REV's page is on the new page or in the checkpoint;
  3. every invariant statement in MUST_SURVIVE is on the new page verbatim;
  4. every headline figure re-derived by rederive.py appears on the new page;
  5. headings before/after, and inbound fragment links into STATUS.md;
  6. every local link on the new page resolves.
Exit 1 if any of 1-4 or 6 fails.
"""
import json
import os
import re
import subprocess
import sys
from pathlib import Path

rev = sys.argv[1]
before = subprocess.check_output(['git', 'show', f'{rev}:docs/STATUS.md']).decode()
after = Path('docs/STATUS.md').read_text()
parts = sorted(Path('docs/development/increments').glob('wave-48-compaction-status-part*.md'),
               key=lambda p: int(re.search(r'(\d+)\.md$', p.name)[1]))
# The fenced bodies only, concatenated: what check 1 asserts equals the page at REV.
checkpoint = ''.join(p.read_text().split('~~~~~text\n', 1)[1].rsplit('~~~~~\n', 1)[0] for p in parts)
failed = False


def report(label, ok, detail=''):
    global failed
    failed |= not ok
    print(('OK   ' if ok else 'FAIL ') + label + (f': {detail}' if detail else ''))


# 1.
r = subprocess.run([sys.executable, 'docs/reports/status-compact/scripts/checkpoint.py', 'check', rev],
                   capture_output=True, text=True)
report('checkpoint reproduces the page', r.returncode == 0, r.stdout.strip() or r.stderr.strip())

# 2.
NAME = re.compile(r'\b([A-Z][A-Z0-9]*(?:-[A-Z0-9]+)+)\b')
REPORTS = {p.name for p in Path('docs/reports').iterdir() if p.is_dir()}
POINTER = re.compile(r'reports/([a-z0-9][a-z0-9-]*[a-z0-9])')
REVISION = re.compile(r'`([0-9a-f]{8,40})`')
FIGURE = re.compile(r'\*\*([^*\n]*\d[^*\n]*)\*\*')
for label, rx in (('names', NAME), ('report pointers', POINTER), ('revisions', REVISION), ('bold figures', FIGURE)):
    b = set(rx.findall(before))
    if label == 'report pointers':
        b &= REPORTS
    a = set(rx.findall(after))
    dropped = b - a
    lost = sorted(x for x in dropped if x not in checkpoint)
    report(f'{label}: {len(b)} before, {len(b & a)} kept, {len(dropped)} moved to the checkpoint only',
           not lost, f'lost {lost[:10]}' if lost else '')
    print(f'     {label} new on the page: {len(a - b)}')

# 3. Invariant statements the compacted page must still carry verbatim.
MUST_SURVIVE = [
    'Safe < Pending < Unknown < Unsafe',
    'Safe only after every obligation discharges',
    'Unsafe only for a validated\n  reachable violation',
    'never Safe, always stays in the denominator',
    'awaiting-human-review',
    'contract-pin-mismatch',
    'plugin-claim-uncertified',
    '`outer-contract` in the runtime projection',
    'never a validated upstream defect claim',
    'Unknowns stay in the denominator',
    'Projection coverage is not safety\n  coverage',
    'Raw units remain unpromoted',
    'function-level roll-up',
    'not a promotion',
    'chunk **lifecycle** result',
    'a checked field contract is not a lifecycle result',
    '`input_chunk_get` residual remains\n  Unknown',
    'never in\n  its denominator',
    'not a bounds certificate',
    'explicit result\n  assumption, not a source-derived fact',
    'not unconditional\n  C-body proofs',
    'an uncertified plugin claim is Pending, every\n  failure Unknown',
    'abstract rule theorems',
    'trust boundary 28',
    'Mapping is not conformance',
    "**CT50's covers the capability half of its scenario only**",
    'CT50-PROOF-ADDRESS-REUSE-UNPROVED',
    'CT-PROOF-F02-CANDIDATES-EXHAUSTED',
    'Native execution is observed, not proved',
    'without implying sanitizer execution there',
    "reproduce its predecessor's verdict-bearing artifacts and\n  attribute every delta",
    'Full language conformance | **false**',
    '**10/50, Linux/macOS only**',
    'not upstream defect claims',
    'not an upstream defect claim',
    'refused on every per-unit route',
    'no progress verdict (Quirin proves resource safety, not progress)',
    '**aggregate Unknown**',
    '**4 Linux Observed / 3 foreign-host Unknown**',
    '**3/3 accounted, 0/3 Cycle 18 composed, 3 explicit Unknown**',
    'no coverage promotion',
    'Cycles 1–17 are complete only at their bounded milestones',
    'all v1 Safe proposals are refused, including empty scopes',
    '`QP010`',
    'CYCLE18-CALLBACK-TASK-HANDOFF',
    'CYCLE18-NATIVE-PLATFORM-MATRIX',
    'CYCLE18-AGGREGATE-POLICY-OBLIGATION-SYNC',
    '**halted at the user\'s request**',
    'remains a failed result',
    'development/contributions.md',
    'Unpublished landings (user decision 2026-11-16)',
    'completed steps 1–7 of `scripts/land.py`',
    'land.py publish --previous 556460e31',
    'not published baselines',
    'landing-gate count, not a published baseline',
]
for s in MUST_SURVIVE:
    report(f'invariant {s!r}', s in after)

# 4. Re-derived headline figures (rederive.py output) on the page.
fig = json.loads(subprocess.check_output(
    [sys.executable, 'docs/reports/status-compact/scripts/rederive.py', sys.argv[2]] if len(sys.argv) > 2 else
    [sys.executable, 'docs/reports/status-compact/scripts/rederive.py',
     'docs/reports/cg10-indirect-access-sole-residuals-r4/integration/gate.json']).decode())


def n(x):
    return f'{x:,}'


expected = []
for name, p in fig['live_profiles'].items():
    s = p['row_statuses']
    expected.append(f"{n(p['rows'])}: **{n(s.get('Safe', 0))} / {s.get('Pending', 0)} / {n(s['Unknown'])} / {s['Unsafe']}**")
    v = p['null_verdicts']
    expected.append(f"{n(p['null_rows'])}: {n(v['Safe'])} / {n(v['Unknown'])} / {v.get('Unsafe', 0)}")
    expected.append(f"`{p['measured_revision'][:9]}`")
g = fig['gate']
expected += [f"**{n(g['workspace_tests'])} passed", f"**{g['lean_audits']} Lean audits**",
             f"**{g['provider_tests']} passed", f"**{g['python_evidence_validator_tests']} tests**",
             f"**{n(fig['matrix']['named_followups'])}**", f"**{fig['ct']['mapped']}/50**",
             f"**{len(fig['ct']['proof'])}/50**: {', '.join(fig['ct']['proof'])}",
             f"**{len(fig['ct']['native_sanitized'])}/50, Linux/macOS only**",
             f"**{fig['ct']['distinct_source_fixture_digests']}** distinct",
             f"**{fig['ct']['shared_fixture_groups']}** reviewed",
             f"**`{fig['lean']['toolchain']}`**", f"**{fig['lean']['audit_modules']}** audits",
             f"**{fig['contracts']['locked_modules']}** modules ({fig['contracts']['locked_declarations']} declarations, {n(fig['contracts']['locked_clauses'])} clauses)"]
for e in expected:
    report(f'figure {e!r}', e in after)

# 5.
def heads(t):
    return [line for line in t.splitlines() if re.match(r'^#{1,6} ', line)
            and not line.startswith('#' * 7)]
hb = [h for h in heads(before)]
ha = heads(after)
print(f'INFO headings before {hb}')
print(f'INFO headings after  {ha}')
inbound = subprocess.run(['git', 'grep', '-n', r'STATUS\.md#', rev, '--'], capture_output=True, text=True).stdout
print(f'INFO inbound fragment links into STATUS.md at {rev}: {len(inbound.splitlines())}')

# 6.
missing = []
for m in re.finditer(r'\]\(([^)#\s]+)(#[^)]+)?\)', after):
    target = m.group(1)
    if target.startswith('http'):
        continue
    if not os.path.exists(os.path.normpath(os.path.join('docs', target))):
        missing.append(target)
report(f'local links on the new page resolve ({len(re.findall(r"]\(", after))} links)', not missing, str(missing))

print(f'INFO bytes before {len(before.encode()):,}, after {len(after.encode()):,}; lines {before.count(chr(10))} -> {after.count(chr(10))}')
sys.exit(1 if failed else 0)
