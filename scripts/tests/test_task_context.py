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

import importlib.util
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[2]
spec = importlib.util.spec_from_file_location('task_context', ROOT/'scripts/task_context.py')
context = importlib.util.module_from_spec(spec)
spec.loader.exec_module(context)


class TaskContextTests(unittest.TestCase):
    def test_union_never_drops_explicit_or_shared_topics(self):
        topics, unknown = context.select({'providers/example/contracts/a.qcore'}, ['concurrency'])
        self.assertFalse(unknown)
        self.assertTrue({'providers', 'contracts', 'concurrency'} <= topics.keys())

    def test_unknown_and_empty_scope_select_every_topic(self):
        for paths in [set(), {'unclassified/new.rs'}, {'../outside'},
                      {'docs/reports/new-card/fixture.c'}]:
            topics, _ = context.select(paths, [])
            self.assertEqual(set(topics), set(context.TOPICS))

    def test_documentation_packet_omits_status_until_requested(self):
        packet = context.packet(ROOT, {'docs/README.md'}, [])
        self.assertNotIn('docs/STATUS.md', [x['path'] for x in packet['readings']])
        packet = context.packet(ROOT, {'docs/README.md'}, ['status'])
        self.assertIn('docs/STATUS.md', [x['path'] for x in packet['readings']])
        self.assertIn('cargo test --workspace --no-fail-fast', ''.join(packet['required_commands']))

    def test_selected_ranges_bind_actual_text(self):
        packet = context.packet(ROOT, {'crates/qcore-c-gateway/src/typed_source.rs'}, [])
        self.assertFalse(packet['unknown_paths'])
        self.assertIn('constraint-8', [r for entry in packet['readings'] for r in entry['rule_ids']])
        self.assertEqual(packet['metrics']['unique_selected_bytes'], sum(r['bytes'] for r in packet['readings']))
        self.assertIsNone(packet['metrics']['actual_session_tokens'])
        self.assertTrue(all(r['start'] <= r['end'] and r['reasons'] for r in packet['readings']))

    def test_real_git_union_includes_old_new_committed_staged_unstaged_deleted_untracked(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            def git(*args):
                return subprocess.check_output(['git', '-C', directory, *args], stderr=subprocess.DEVNULL)
            git('init'); git('config', 'user.email', 'test@example.invalid'); git('config', 'user.name', 'test')
            for name in ['old name', 'dirty', 'deleted']:
                (root/name).write_text(name)
            git('add', '.'); git('commit', '-m', 'base'); base = git('rev-parse', 'HEAD').decode().strip()
            (root/'committed').write_text('new'); git('add', '.'); git('commit', '-m', 'new')
            git('mv', 'old name', 'new name')
            untracked = 'untracked name' if sys.platform == 'win32' else 'untracked\nname'
            (root/'dirty').write_text('changed'); (root/'deleted').unlink(); (root/untracked).write_text('new')
            self.assertEqual(context.changed_paths(root, base), {'old name', 'new name', 'dirty', 'deleted', 'committed', untracked})

    def test_invalid_base_does_not_silently_make_empty_diff(self):
        with self.assertRaises(subprocess.CalledProcessError):
            context.changed_paths(ROOT, 'not-a-real-task-context-base')

    def test_missing_or_duplicate_required_sections_refuse(self):
        for text in ['No headings\n', '## Wrong heading\n', '## Working rules\n## Working rules\n']:
            with self.assertRaises(ValueError):
                context.spans('operations.md', text, ['Working rules'])

    def test_missing_canonical_gate_commands_refuse(self):
        original = context.read_source
        def missing(path):
            return '# Missing gate\n' if path.name == 'validation.md' else original(path)
        with patch.object(context, 'read_source', side_effect=missing):
            with self.assertRaises(ValueError):
                context.packet(ROOT, {'docs/README.md'}, [])

    def test_agent_guidance_paths_list_history_on_demand(self):
        packet = context.packet(ROOT, {'docs/agents/operations.md'}, [])
        read = [x['path'] for x in packet['readings']]
        self.assertFalse(set(context.HISTORY) & set(read))
        self.assertEqual([x['path'] for x in packet['on_demand']], context.HISTORY)
        self.assertIn('maintenance', packet['topics'])

    def test_rationale_topic_and_unknown_scope_read_history(self):
        for paths, topics in [({'docs/agents/operations.md'}, ['rationale']),
                              ({'unclassified/new.rs'}, []), (set(), [])]:
            packet = context.packet(ROOT, paths, topics)
            read = [x['path'] for x in packet['readings']]
            self.assertTrue(set(context.HISTORY) <= set(read))
            self.assertEqual(packet['on_demand'], [])

    def test_integration_selects_card_brief_standard(self):
        packet = context.packet(ROOT, {'docs/STATUS.md'}, ['integration'])
        lines = (ROOT/'docs/agents/operations.md').read_text(encoding='utf-8').splitlines()
        headings = [lines[r['start']-1] for r in packet['readings']
                    if r['path'] == 'docs/agents/operations.md']
        self.assertIn('## Card briefs and delegation', headings)
        self.assertIn('## Integration and reconciliation', headings)
