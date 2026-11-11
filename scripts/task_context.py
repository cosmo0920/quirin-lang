#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
#
# Licensed under the Apache License, Version 2.0 (the "License");
# you may not use this file except in compliance with the License.
# You may obtain a copy of the License at
#
#     http://www.apache.org/licenses/LICENSE-2.0
#
# Unless required by applicable law or agreed to in writing, software
# distributed under the License is distributed on an "AS IS" BASIS,
# WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
# See the License for the specific language governing permissions and
# limitations under the License.

"""Produce a conservative, read-only task packet from paths and explicit topics."""
import argparse
import fnmatch
import hashlib
import json
from pathlib import Path
import re
import subprocess

ROOT = Path(__file__).resolve().parents[1]
TOPICS = {
    'docs': ['evidence'],
    'contracts': ['gateway', 'source-identity', 'diagnostics'],
    'gateway': ['gateway', 'source-identity', 'diagnostics'],
    'providers': ['providers', 'source-identity'],
    'source': ['source-routes', 'diagnostics'],
    'buffers': ['buffers-records'],
    'effects': ['effects', 'source-routes'],
    'io': ['io', 'source-routes'],
    'concurrency': ['concurrency', 'buffers-records', 'source-routes'],
    'status': [], 'integration': [], 'maintenance': [], 'rationale': [],
}
# Rule history is read on demand: an explicit `rationale` topic, or the
# conservative fallback, reads it; agent-guidance paths list it as a pointer.
HISTORY = ['docs/reports/task-context-contract-bundle/operations-before.txt',
           'docs/development/increments/structure-checkpoints/agent-cycle-guidance.md']
# Match ALL applicable entries. Broad shared implementation paths deliberately
# select all topics; narrower rules never cancel broader obligations.
ROUTES = [
    (['AGENTS.md', 'CLAUDE.md', 'docs/agents/*', 'scripts/task_context.py',
      'scripts/tests/test_task_context.py'], ['docs', 'maintenance']),
    (['README.md', 'DEVELOPMENT.md', 'docs/README.md', 'docs/reports/*',
      'docs/development/*'], ['docs']),
    (['docs/STATUS.md', 'docs/reviews/*', 'docs/reevaluation/*'], ['docs', 'status']),
    (['providers/*'], ['providers', 'contracts']),
    (['contract/*', 'crates/qcore-contract/*'], ['contracts']),
    (['crates/qcore-c-gateway/*'], ['gateway']),
    (['crates/qcore-verifier-host/*'], ['providers']),
    (['crates/qcore-audit/*'], ['gateway', 'providers']),
    (['compiler/*', 'crates/qcore-core/*', 'crates/qcore-frontend/*',
      'crates/qcore-verification/*', 'crates/qcore-schemas/*', 'crates/qcorec/*',
      'proof/*', 'docs/QCORE_*', 'docs/GENERALIZATION_PLAN.md'], list(TOPICS)),
]


def git(root, *args):
    return subprocess.check_output(['git', '-C', str(root), *args])


def read_source(path):
    return path.read_bytes().decode('utf-8')


def changed_paths(root, base):
    # --no-renames includes both old and new names, even for staged renames.
    result = set()
    for args in [('diff', '--no-renames', '--name-only', '-z', base, 'HEAD', '--'),
                 ('diff', '--no-renames', '--name-only', '-z', '--'),
                 ('diff', '--cached', '--no-renames', '--name-only', '-z', '--'),
                 ('ls-files', '--others', '--exclude-standard', '-z', '--')]:
        result.update(p.decode('utf-8', 'surrogateescape') for p in git(root, *args).split(b'\0') if p)
    return result


def select(paths, topics):
    reasons = {topic: ['explicit task topic'] for topic in topics}
    unknown = []
    for path in sorted(paths):
        matched = False
        # Reports may contain executable harnesses and language fixtures, not
        # just prose. Their location must not silently classify code as docs.
        report_code = path.startswith('docs/') and Path(path).suffix in {
            '.py', '.rs', '.lean', '.qcore', '.qc', '.c', '.h', '.sh'
        }
        for patterns, selected in ROUTES:
            if any(fnmatch.fnmatchcase(path, pattern) for pattern in patterns):
                matched = True
                for topic in selected:
                    reasons.setdefault(topic, []).append(path)
        if not matched or report_code:
            unknown.append(path)
    if unknown or not (paths or topics):
        for topic in TOPICS:
            reasons.setdefault(topic, []).append('conservative fallback: unknown path or empty scope')
    return reasons, unknown


def spans(path, text, sections=None):
    """Exact heading blocks, with line references; no paraphrased rule copies."""
    lines = text.splitlines(keepends=True)
    starts = [i for i, line in enumerate(lines) if re.match(r'^## ', line)]
    if sections is not None:
        for section in sections:
            if sum(lines[start].strip()[3:] == section for start in starts) != 1:
                raise ValueError(f'{path}: missing or duplicated required section {section}')
    if sections is None:
        ranges = [(0, len(lines))]
    else:
        ranges = [(start, starts[n+1] if n+1 < len(starts) else len(lines))
                  for n, start in enumerate(starts) if lines[start].strip()[3:] in sections]
    return [{'path': path, 'start': a+1, 'end': b,
             'bytes': len(''.join(lines[a:b]).encode()),
             'sha256': hashlib.sha256(''.join(lines[a:b]).encode()).hexdigest(),
             'rule_ids': re.findall(r'<a id="(constraint-[^"]+)"', ''.join(lines[a:b]))}
            for a, b in ranges]


def packet(root, paths, topics):
    reasons, unknown = select(paths, topics)
    ops = ['Working rules', 'Worktree and measurement hygiene', 'Task reading protocol']
    if 'integration' in reasons:
        ops += ['Card briefs and delegation', 'Integration and reconciliation']
    if 'maintenance' in reasons:
        ops.append('Maintenance cadence')
    selected = {'AGENTS.md': None, 'docs/agents/operations.md': ops,
                'docs/agents/validation.md': None, 'docs/agents/evidence.md': None}
    why = {path: ['every change'] for path in selected}
    for topic, causes in reasons.items():
        for page in TOPICS[topic]:
            path = f'docs/agents/{page}.md'
            selected[path] = None
            why.setdefault(path, []).extend(f'{topic}: {cause}' for cause in causes)
    if 'status' in reasons:
        selected['docs/STATUS.md'] = None
        why['docs/STATUS.md'] = reasons['status']
    on_demand = []
    if 'rationale' in reasons:
        for path in HISTORY:
            selected[path] = None
            why[path] = ['rule changes require the preserved rationale; this is historical text']
    elif 'maintenance' in reasons:
        on_demand = [{'path': path, 'when': 'changing a rule or investigating its rationale; '
                      'pass --topic rationale to read it'} for path in HISTORY]
    readings = []
    for path, sections in selected.items():
        for span in spans(path, read_source(root/path), sections):
            span['reasons'] = sorted(set(why[path]))
            readings.append(span)
    # Extract canonical shell commands instead of maintaining another gate list.
    validation = read_source(root/'docs/agents/validation.md')
    commands = re.findall(r'```sh\r?\n(.*?)```', validation, re.S)
    if not commands:
        raise ValueError('standing gate has no canonical shell command block')
    if 'providers' in reasons:
        provider_sections = read_source(root/'providers/README.md').split('## Build, verify and reproduce', 1)
        if len(provider_sections) != 2:
            raise ValueError('provider build/verification section is missing')
        provider_commands = re.findall(r'```sh\r?\n(.*?)```', provider_sections[1], re.S)[:1]
        if not provider_commands:
            raise ValueError('provider verification has no canonical shell command block')
        commands += provider_commands
    followups = json.loads((root/'docs/reviews/r0_3/acceptance_matrix.json').read_text()).get('named_followups', {})
    # Do not guess applicability from prose: return the canonical tracker pointer
    # and explicit exact-match path mentions as candidates, not a complete triage.
    entries = followups if isinstance(followups, list) else list(followups.values())
    candidates = [value for value in entries
                  if any(path in json.dumps(value) for path in paths)]
    return {'paths': sorted(paths), 'topics': sorted(reasons), 'unknown_paths': unknown,
            'readings': readings, 'on_demand': on_demand, 'required_commands': commands,
            'additional_validation': 'Read provider rule 18 and validation.md for package gates when applicable.',
            'followups': {'source': 'docs/reviews/r0_3/acceptance_matrix.json#named_followups',
                          'exact_path_candidates': candidates,
                          'limitation': 'Also inspect affected report follow-ons and semantic topic residuals.'},
            'metrics': {'unique_selected_bytes': sum(r['bytes'] for r in readings),
                        'single_pass_retrieval_bytes': sum(r['bytes'] for r in readings),
                        'actual_session_tokens': None,
                        'note': 'Selection estimate only; repeated reads add retrieval. Bytes are not billed tokens.'}}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--base', required=True)
    parser.add_argument('--topic', action='append', choices=sorted(TOPICS), default=[])
    parser.add_argument('--path', action='append', default=[], help='repository-relative planned path')
    output = parser.add_mutually_exclusive_group()
    output.add_argument('--render', action='store_true', help='print exact selected text, once per range')
    output.add_argument('--json', action='store_true', help='emit the full machine-readable packet')
    args = parser.parse_args()
    try:
        base = git(ROOT, 'rev-parse', '--verify', '--end-of-options', args.base + '^{commit}').decode().strip()
        paths = changed_paths(ROOT, base) | set(args.path)
        result = packet(ROOT, paths, args.topic)
        result['base'] = base
        if args.render:
            for item in result['readings']:
                print(f"\n--- {item['path']}:{item['start']}-{item['end']} ---")
                print(''.join(read_source(ROOT/item['path']).splitlines(keepends=True)[item['start']-1:item['end']]), end='')
        elif args.json:
            print(json.dumps(result, indent=2, ensure_ascii=True))
        else:
            print(f"Base: {base}; {len(paths)} paths; topics: {', '.join(result['topics'])}")
            for item in result['readings']:
                why = item['reasons'][:3]
                if len(item['reasons']) > 3:
                    why.append(f"and {len(item['reasons'])-3} other path reasons")
                rules = ', '.join(item['rule_ids']) or 'operative section / reference'
                print(f"- {item['path']}:{item['start']}-{item['end']} ({item['bytes']} bytes; {rules})")
                print('  Why: ' + '; '.join(why))
            for item in result['on_demand']:
                print(f"- on demand: {item['path']} ({item['when']})")
            if result['unknown_paths']:
                print('Unknown paths select every topic: ' + json.dumps(result['unknown_paths']))
            print('Required commands (additional conditional requirements remain in the selected rules):')
            for commands in result['required_commands']:
                print(commands.rstrip())
            print('Follow-ups: ' + result['followups']['source'])
            print('Exact-path candidates: ' + ', '.join(str(v.get('id', '?')) for v in result['followups']['exact_path_candidates']))
            print(result['followups']['limitation'])
            print(f"Selected source: {result['metrics']['unique_selected_bytes']} unique bytes; token billing unmeasured.")
    except (OSError, ValueError, subprocess.CalledProcessError) as error:
        parser.exit(1, f'task context failed; read the full rule router: {error}\n')


if __name__ == '__main__':
    main()
