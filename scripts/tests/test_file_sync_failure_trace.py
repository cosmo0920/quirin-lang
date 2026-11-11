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

"""Reject erased sync failures, invented aborts and unearned durability evidence."""
import copy
from pathlib import Path
import sys
import unittest
sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from check_file_sync_failure_trace import check_trace, load_trace

FIXTURE = Path(__file__).resolve().parents[2] / 'docs/reports/cycle19-file-sync-failures'


def event(rows, scenario, kind, **fields):
    return next(r for r in rows if r['case'] == 500+scenario and r['kind'] == kind
                and all(r.get(k) == v for k, v in fields.items()))


def after(rows, scenario, name):
    index = rows.index(event(rows, scenario, 'operation', name=name))
    return rows[index+2]['context']['streams'][0]['chunks'][0]


class SyncFailureTests(unittest.TestCase):
    def test_native_matrix(self):
        observations = check_trace(load_trace(FIXTURE/'trace.jsonl'), load_trace(FIXTURE/'processes.jsonl'))
        values = [row['values'] for row in observations]
        for key, count in [('allocations',30), ('releases',30), ('transient_releases',3),
                           ('teardown_releases',27), ('open_epochs',4), ('mapping_epochs',4),
                           ('suppressed_sync_calls',3), ('failed_commit_preserves_transaction',2),
                           ('retry_clears_transaction',1), ('rollback_retains_header',1),
                           ('down_despite_sync_failure',1), ('unmapped_commit_without_sync',1)]:
            self.assertEqual(sum(v[key] for v in values), count)
        self.assertTrue(all(v['static_certificate'] == v['durability_proved'] == 0 for v in values))

    def test_trace_mutations_reject(self):
        original = load_trace(FIXTURE/'trace.jsonl'); processes = load_trace(FIXTURE/'processes.jsonl')
        mutations = {
            'empty':lambda r:r.clear(),
            'missing_case':lambda r:r.__setitem__(slice(None),[x for x in r if x['case'] != 502]),
            'schema':lambda r:r[0].update(schema_version=2),
            'boolean':lambda r:r[0].update(root_length=True),
            'float':lambda r:r[0].update(case=500.0),
            'extra':lambda r:r.append(r[-1]),
            'missing_release':lambda r:r.remove(event(r,0,'free')),
            'missing_arm':lambda r:r.remove(event(r,0,'arm_sync_failure')),
            'double_arm':lambda r:event(r,0,'arm_sync_failure').update(count=2),
            'missing_suppression':lambda r:r.remove(event(r,0,'suppressed_sync')),
            'wrong_suppression_epoch':lambda r:event(r,0,'suppressed_sync').update(epoch=2),
            'pending_suppression':lambda r:event(r,0,'suppressed_sync').update(remaining=1),
            'wrong_errno':lambda r:event(r,0,'sync',result=-1).update(errno=12),
            'failure_as_success':lambda r:event(r,0,'sync',result=-1).update(result=0,errno=0),
            'wrong_mapping':lambda r:event(r,0,'sync',result=-1).update(map=4096),
            'wrong_sync_flags':lambda r:event(r,0,'sync',result=-1).update(flags=4),
            'wrong_sync_size':lambda r:event(r,0,'sync',result=-1).update(size=0),
            'wrong_sync_phase':lambda r:event(r,0,'sync',result=-1).update(phase='teardown'),
            'missing_retry_sync':lambda r:r.remove(event(r,0,'sync',result=0)),
            'failed_commit_aborts':lambda r:after(r,0,'commit_failed').update(transaction=0),
            'failed_commit_synced':lambda r:after(r,0,'commit_failed').update(synced=1),
            'failed_commit_rolls_back':lambda r:after(r,0,'commit_failed').update(data_size=4),
            'failed_commit_unmaps':lambda r:after(r,0,'commit_failed').update(map=0),
            'failed_commit_closes':lambda r:after(r,0,'commit_failed').update(fd=-1),
            'retry_retains_transaction':lambda r:after(r,0,'commit_retry').update(transaction=1),
            'retry_not_synced':lambda r:after(r,0,'commit_retry').update(synced=0),
            'rollback_erases_header':lambda r:after(r,1,'rollback').update(header_length=4),
            'rollback_keeps_appended_length':lambda r:after(r,1,'rollback').update(data_size=7),
            'inactive_commit_erases_header':lambda r:after(r,1,'commit_after_rollback').update(header_length=4),
            'down_claims_synced':lambda r:after(r,2,'down_sync_failed').update(synced=1),
            'down_aborts':lambda r:after(r,2,'down_sync_failed').update(transaction=0),
            'down_keeps_mapping':lambda r:after(r,2,'down_sync_failed').update(map=4096),
            'unmapped_commit_claims_synced':lambda r:after(r,2,'commit_down').update(synced=1),
            'unmapped_commit_keeps_transaction':lambda r:after(r,2,'commit_down').update(transaction=1),
            'reload_rolls_back':lambda r:after(r,2,'up').update(data_size=4,content=b'seed'.hex()),
            'stale_open_epoch':lambda r:event(r,2,'open',epoch=2).update(epoch=1),
            'stale_map_epoch':lambda r:event(r,2,'map',epoch=2).update(epoch=1),
            'stale_map_fd_epoch':lambda r:event(r,2,'map',epoch=2).update(fd_epoch=1),
            'wrong_inode':lambda r:event(r,2,'probe',label='up').update(fd_inode=1),
            'down_probe_mapping_live':lambda r:event(r,2,'probe',label='down_sync_failed').update(map_result=0,map_errno=0),
            'terminal_descriptor_live':lambda r:event(r,2,'probe',label='parents_released').update(fd_result=0,fd_errno=0),
            'terminal_mapping_live':lambda r:event(r,2,'probe',label='parents_released').update(map_result=0,map_errno=0),
            'bad_ring':lambda r:event(r,0,'snapshot')['context']['streams'][0]['up_ring'].update(next=1),
            'bad_counter':lambda r:event(r,2,'context_state').update(up=1),
        }
        for row in original:
            if row['kind'] == 'operation':
                n, name, result = row['case']-500, row['name'], row['result']
                mutations[f'operation:{n}:{name}'] = lambda r,n=n,name=name,result=result:event(r,n,'operation',name=name).update(result=result+1)
        for name, mutate in mutations.items():
            with self.subTest(name=name):
                rows = copy.deepcopy(original); mutate(rows)
                with self.assertRaises(ValueError): check_trace(rows, processes)

    def test_process_mutations_reject(self):
        rows = load_trace(FIXTURE/'trace.jsonl'); original = load_trace(FIXTURE/'processes.jsonl')
        mutations = {
            'missing':lambda p:p.pop(),
            'identity':lambda p:p[0].update(case=400),
            'failed':lambda p:p[0].update(returncode=1),
            'sanitizer':lambda p:p[0].update(stderr=p[0]['stderr']+'LeakSanitizer failed\n'),
            'missing_error':lambda p:p[0].update(stderr=''),
            'different_error':lambda p:p[0].update(stderr=p[0]['stderr'].replace('errno=5','errno=12')),
            'extra_field':lambda p:p[0].update(durability_proved=1),
            'changed_root':lambda p:p[0].update(root_after=[1,2]),
            'changed_inode':lambda p:p[0].update(initial_inode=1),
            'changed_initial':lambda p:p[0].update(initial_hex=''),
            'invented_byte_rollback':lambda p:p[1].update(final_hex=p[1]['initial_hex']),
            'unmapped_commit_erases_append':lambda p:p[2].update(final_hex=p[2]['initial_hex']),
            'unexpected_file':lambda p:p[0]['entries'].append(p[0]['entries'][0]),
            'no_cleanup':lambda p:p[2].update(fixture_cleanup_complete=0),
        }
        for name, mutate in mutations.items():
            with self.subTest(name=name):
                processes = copy.deepcopy(original); mutate(processes)
                with self.assertRaises(ValueError): check_trace(rows, processes)


if __name__ == '__main__': unittest.main()
