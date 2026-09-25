#!/usr/bin/env python3
"""Measure the agent-cycle reading surface at two revisions.

Usage: python3 docs/reports/w44-agent-guidance-opus55/measure.py BEFORE AFTER [OUT]

For each revision the script exports only the files the task packet reads
(`git archive`), imports that revision's own `scripts/task_context.py`, and
builds the packet for three representative tasks with explicit paths, so the
measuring checkout's working tree cannot leak into either side. It then checks
that every constraint id, operations section and gate command the BEFORE packet
selected is still selected AFTER, and counts emphasis markers in the operative
agent-cycle files. Bytes are source bytes, not billed tokens.
"""
import importlib.util
import io
import json
from pathlib import Path
import re
import subprocess
import sys
import tarfile
import tempfile

ROOT = Path(__file__).resolve().parents[3]
EXPORT = ['AGENTS.md', 'CLAUDE.md', 'docs/agents', 'docs/STATUS.md',
          'docs/reports/task-context-contract-bundle/operations-before.txt',
          'docs/development/increments/structure-checkpoints',
          'providers/README.md', 'docs/reviews/r0_3/acceptance_matrix.json',
          'scripts/task_context.py']
TASKS = {
    'gateway change': ({'crates/qcore-c-gateway/src/typed_source.rs'}, []),
    'provider contract change': (
        {'providers/quirin-provider-libc/contracts/libc/stdio.qcore'}, []),
    'integration record': (
        {'docs/development/increments/wave-44.md', 'docs/STATUS.md',
         'docs/reports/example-card/integration-gate.json',
         'docs/reviews/r0_3/acceptance_matrix.json'}, ['integration']),
    'agent-guidance edit': ({'docs/agents/operations.md'}, []),
}
OPERATIVE = ['AGENTS.md', 'CLAUDE.md', 'docs/agents/operations.md',
             'docs/agents/validation.md', 'docs/agents/README.md']
PATTERNS = {
    'bold_spans': r'\*\*[^*\n]+\*\*',
    'all_caps_emphasis': r'\b(?:NEVER|MUST|ALWAYS|IMPORTANT|NOT|ONLY|ALL)\b',
    'self_check_phrases': r'(?i)\b(?:re-read|double-check|re-verify|verify again|think carefully)\b',
    'wave_or_incident_mentions': r'(?i)\bwave[- ]\d+\b',
}


def export(rev, into):
    present = [p for p in EXPORT if subprocess.run(
        ['git', '-C', str(ROOT), 'cat-file', '-e', f'{rev}:{p}'],
        capture_output=True).returncode == 0]
    data = subprocess.check_output(['git', '-C', str(ROOT), 'archive', rev, '--', *present])
    with tarfile.open(fileobj=io.BytesIO(data)) as archive:
        archive.extractall(into, filter='data')


def load(root):
    spec = importlib.util.spec_from_file_location(f'task_context_{id(root)}', root/'scripts/task_context.py')
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def headings(root, reading):
    lines = (root/reading['path']).read_text(encoding='utf-8').splitlines()
    return [l[3:].strip() for l in lines[reading['start']-1:reading['end']] if l.startswith('## ')]


def measure(rev):
    with tempfile.TemporaryDirectory() as directory:
        root = Path(directory)
        export(rev, root)
        context = load(root)
        out = {'revision': subprocess.check_output(
            ['git', '-C', str(ROOT), 'rev-parse', rev]).decode().strip()}
        injected = ['AGENTS.md', 'CLAUDE.md']
        out['startup_injected_bytes'] = sum((root/p).stat().st_size for p in injected)
        out['operative_file_bytes'] = {p: (root/p).stat().st_size for p in OPERATIVE}
        text = ''.join((root/p).read_text(encoding='utf-8') for p in OPERATIVE)
        out['operative_markers'] = {k: len(re.findall(v, text)) for k, v in PATTERNS.items()}
        out['tasks'] = {}
        for name, (paths, topics) in TASKS.items():
            packet = context.packet(root, set(paths), topics)
            readings = packet['readings']
            out['tasks'][name] = {
                'topics': packet['topics'],
                'reading_bytes': sum(r['bytes'] for r in readings),
                'files': {f"{r['path']}:{r['start']}-{r['end']}": r['bytes'] for r in readings},
                'rule_ids': sorted({i for r in readings for i in r['rule_ids']}),
                'operations_sections': sorted(
                    h for r in readings if r['path'] == 'docs/agents/operations.md'
                    for h in headings(root, r)),
                'required_commands': packet['required_commands'],
                'on_demand': [h['path'] for h in packet.get('on_demand', [])],
            }
        return out


def main():
    before, after = measure(sys.argv[1]), measure(sys.argv[2])
    checks = {}
    for name in TASKS:
        b, a = before['tasks'][name], after['tasks'][name]
        checks[name] = {
            'rule_ids_kept': set(b['rule_ids']) <= set(a['rule_ids']),
            'rule_ids_lost': sorted(set(b['rule_ids']) - set(a['rule_ids'])),
            'operations_sections_kept': set(b['operations_sections']) <= set(a['operations_sections']),
            'operations_sections_added': sorted(set(a['operations_sections']) - set(b['operations_sections'])),
            'commands_identical': b['required_commands'] == a['required_commands'],
            'reading_bytes': [b['reading_bytes'], a['reading_bytes']],
        }
    result = {'before': before, 'after': after, 'checks': checks,
              'note': 'Source bytes of an explicit reading packet; not runtime telemetry or token billing.'}
    text = json.dumps(result, indent=2, sort_keys=True) + '\n'
    if len(sys.argv) > 3:
        Path(sys.argv[3]).write_text(text, encoding='utf-8')
    print(json.dumps(checks, indent=2, sort_keys=True))
    ok = all(c['rule_ids_kept'] and c['operations_sections_kept'] and c['commands_identical']
             for c in checks.values())
    sys.exit(0 if ok else 1)


if __name__ == '__main__':
    main()
