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

"""Failed down must retain its original OS epoch, up ring and counter."""
import copy
from pathlib import Path
import sys
import unittest
sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from check_file_unmap_failure_trace import check_trace, load_trace

FIXTURE = Path(__file__).resolve().parents[2] / 'docs/reports/cycle19-file-unmap-failures'


def event(rows, scenario, kind, **fields):
    return next(r for r in rows if r['case'] == 600+scenario and r['kind'] == kind
                and all(r.get(k) == v for k, v in fields.items()))


def context_after(rows, scenario, name):
    index = rows.index(event(rows, scenario, 'operation', name=name))
    return rows[index+2]['context']


def after(rows, scenario, name):
    return context_after(rows, scenario, name)['streams'][0]['chunks'][0]


class UnmapFailureTests(unittest.TestCase):
    def test_native_matrix(self):
        observations = check_trace(load_trace(FIXTURE/'trace.jsonl'), load_trace(FIXTURE/'processes.jsonl'))
        values = [row['values'] for row in observations]
        for key, count in [('allocations',40), ('releases',40), ('transient_releases',4),
                           ('teardown_releases',36), ('open_epochs',7), ('mapping_epochs',7),
                           ('suppressed_unmap_calls',4), ('suppressed_sync_calls',2),
                           ('failed_down_retains_mapping_and_descriptor',4),
                           ('failed_down_retains_up_ring_and_counter',4),
                           ('up_while_still_mapped_rejected',4),
                           ('successful_sync_before_failed_unmap',1),
                           ('retry_down_releases_original_epoch',3),
                           ('parent_teardown_recovers_original_epoch',1)]:
            self.assertEqual(sum(v[key] for v in values), count)
        self.assertTrue(all(v['static_certificate'] == v['durability_proved'] == 0 for v in values))

    def test_reopen_can_relocate_mapping(self):
        rows = load_trace(FIXTURE/'trace.jsonl'); processes = load_trace(FIXTURE/'processes.jsonl')
        start = rows.index(event(rows,2,'map',epoch=2)); old = rows[start]['result']; new = old+4096*64
        def relocate(value):
            if isinstance(value,dict): return {k:relocate(v) for k,v in value.items()}
            if isinstance(value,list): return [relocate(v) for v in value]
            return new if value == old else value
        for i in range(start,len(rows)):
            if rows[i]['case'] == 602: rows[i] = relocate(rows[i])
        self.assertEqual(len(check_trace(rows,processes)),4)

    def test_trace_mutations_reject(self):
        original = load_trace(FIXTURE/'trace.jsonl'); processes = load_trace(FIXTURE/'processes.jsonl')
        mutations = {
            'empty':lambda r:r.clear(),
            'missing_case':lambda r:r.__setitem__(slice(None),[x for x in r if x['case'] != 603]),
            'schema':lambda r:r[0].update(schema_version=2),
            'boolean':lambda r:r[0].update(root_length=True),
            'float':lambda r:r[0].update(case=600.0),
            'extra':lambda r:r.append(r[-1]),
            'missing_release':lambda r:r.remove(event(r,0,'free')),
            'missing_arm':lambda r:r.remove(event(r,0,'arm_unmap_failure')),
            'double_arm':lambda r:event(r,0,'arm_unmap_failure').update(unmap_count=2),
            'missing_suppression':lambda r:r.remove(event(r,0,'suppressed_unmap')),
            'wrong_suppression_epoch':lambda r:event(r,0,'suppressed_unmap').update(epoch=2),
            'pending_suppression':lambda r:event(r,0,'suppressed_unmap').update(remaining=1),
            'wrong_unmap_errno':lambda r:event(r,0,'unmap',result=-1).update(errno=12),
            'unmap_failure_as_success':lambda r:event(r,0,'unmap',result=-1).update(result=0,errno=0),
            'wrong_unmap_mapping':lambda r:event(r,0,'unmap',result=-1).update(map=4096),
            'wrong_unmap_size':lambda r:event(r,0,'unmap',result=-1).update(size=0),
            'wrong_unmap_phase':lambda r:event(r,0,'unmap',result=-1).update(phase='teardown'),
            'missing_sync_suppression':lambda r:r.remove(event(r,2,'suppressed_sync')),
            'wrong_sync_errno':lambda r:event(r,2,'sync',result=-1).update(errno=12),
            'wrong_sync_flags':lambda r:event(r,2,'sync',result=-1).update(flags=4),
            'failed_down_erases_mapping':lambda r:after(r,0,'down_failed').update(map=0),
            'failed_down_closes_descriptor':lambda r:after(r,0,'down_failed').update(fd=-1),
            'failed_down_erases_size':lambda r:after(r,1,'down_failed').update(size=0),
            'failed_down_erases_content':lambda r:after(r,1,'down_failed').update(data_size=0,content=''),
            'failed_down_erases_header':lambda r:after(r,1,'down_failed').update(header_length=0),
            'failed_down_decrements_counter':lambda r:context_after(r,0,'down_failed').update(up=0),
            'failed_down_unlinks_state':lambda r:context_after(r,0,'down_failed')['streams'][0]['up_ring'].update(nodes=[]),
            'failed_down_keeps_dirty_after_sync':lambda r:after(r,1,'down_failed').update(synced=0),
            'combined_failure_invents_sync':lambda r:after(r,2,'down_failed').update(synced=1),
            'retry_uses_new_epoch':lambda r:event(r,2,'unmap',result=0,epoch=1).update(epoch=2),
            'retry_retains_mapping':lambda r:after(r,2,'down_retry').update(map=4096),
            'retry_not_synced':lambda r:after(r,2,'down_retry').update(synced=0),
            'missing_retry_sync':lambda r:r.remove(event(r,2,'sync',result=0)),
            'missing_parent_sync':lambda r:r.remove(event(r,3,'sync',result=0)),
            'parent_sync_wrong_epoch':lambda r:event(r,3,'sync',result=0).update(epoch=2),
            'stale_open_epoch':lambda r:event(r,2,'open',epoch=2).update(epoch=1),
            'stale_map_epoch':lambda r:event(r,2,'map',epoch=2).update(epoch=1),
            'stale_map_fd_epoch':lambda r:event(r,2,'map',epoch=2).update(fd_epoch=1),
            'wrong_inode':lambda r:event(r,2,'probe',label='up').update(fd_inode=1),
            'failed_down_probe_mapping_absent':lambda r:event(r,2,'probe',label='down_failed').update(map_result=-1,map_errno=12),
            'failed_down_probe_descriptor_absent':lambda r:event(r,2,'probe',label='down_failed').update(fd_result=-1,fd_errno=9),
            'terminal_descriptor_live':lambda r:event(r,3,'probe',label='parents_released').update(fd_result=0,fd_errno=0),
            'terminal_mapping_live':lambda r:event(r,3,'probe',label='parents_released').update(map_result=0,map_errno=0),
        }
        for row in original:
            if row['kind'] == 'operation':
                n, name, result = row['case']-600, row['name'], row['result']
                mutations[f'operation:{n}:{name}'] = lambda r,n=n,name=name,result=result:event(r,n,'operation',name=name).update(result=result+1)
        for name, mutate in mutations.items():
            with self.subTest(name=name):
                rows = copy.deepcopy(original); mutate(rows)
                with self.assertRaises(ValueError): check_trace(rows, processes)

    def test_process_mutations_reject(self):
        rows = load_trace(FIXTURE/'trace.jsonl'); original = load_trace(FIXTURE/'processes.jsonl')
        mutations = {
            'missing':lambda p:p.pop(),
            'identity':lambda p:p[0].update(case=500),
            'failed':lambda p:p[0].update(returncode=1),
            'sanitizer':lambda p:p[0].update(stderr=p[0]['stderr']+'LeakSanitizer failed\n'),
            'missing_error':lambda p:p[0].update(stderr=''),
            'reversed_errors':lambda p:p[2].update(stderr='\n'.join(reversed(p[2]['stderr'].splitlines()))),
            'different_error':lambda p:p[0].update(stderr=p[0]['stderr'].replace('errno=5','errno=12')),
            'extra_field':lambda p:p[0].update(durability_proved=1),
            'changed_root':lambda p:p[0].update(root_after=[1,2]),
            'changed_inode':lambda p:p[0].update(initial_inode=1),
            'changed_initial':lambda p:p[0].update(initial_hex=''),
            'invented_byte_rollback':lambda p:p[1].update(final_hex=p[1]['initial_hex']),
            'parent_erases_append':lambda p:p[3].update(final_hex=p[3]['initial_hex']),
            'unexpected_file':lambda p:p[0]['entries'].append(p[0]['entries'][0]),
            'no_cleanup':lambda p:p[2].update(fixture_cleanup_complete=0),
        }
        for name, mutate in mutations.items():
            with self.subTest(name=name):
                processes = copy.deepcopy(original); mutate(processes)
                with self.assertRaises(ValueError): check_trace(rows, processes)


if __name__ == '__main__': unittest.main()
