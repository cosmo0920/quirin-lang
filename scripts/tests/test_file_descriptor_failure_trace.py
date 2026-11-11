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

"""Reject evidence that equates down membership or parent free with fd release."""
import copy
from pathlib import Path
import sys
import unittest
sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from check_file_descriptor_failure_trace import check_trace, load_trace

FIXTURE = Path(__file__).resolve().parents[2] / 'docs/reports/cycle19-file-descriptor-failures'


def event(rows, scenario, kind, **fields):
    return next(r for r in rows if r['case'] == 700+scenario and r['kind'] == kind
                and all(r.get(k) == v for k, v in fields.items()))


def context_after(rows, scenario, name):
    return rows[rows.index(event(rows, scenario, 'operation', name=name))+2]['context']


def after(rows, scenario, name):
    return context_after(rows, scenario, name)['streams'][0]['chunks'][0]


class DescriptorFailureTests(unittest.TestCase):
    def test_native_matrix(self):
        observations = check_trace(load_trace(FIXTURE/'trace.jsonl'), load_trace(FIXTURE/'processes.jsonl'))
        values = [row['values'] for row in observations]
        for key, count in [('allocations',40), ('releases',40), ('transient_releases',4),
                           ('teardown_releases',36), ('open_epochs',5), ('mapping_epochs',5),
                           ('suppressed_close_calls',3), ('suppressed_open_calls',2),
                           ('failed_close_retains_descriptor_in_down_ring',2),
                           ('retry_down_and_up_reject_without_closing',2),
                           ('failed_open_preserves_down_state_without_new_epoch',2),
                           ('reopen_retry_acquires_new_epoch',1), ('library_descriptor_releases',4),
                           ('parent_descriptor_residuals',1), ('harness_descriptor_releases',1)]:
            self.assertEqual(sum(v[key] for v in values), count)
        self.assertTrue(all(v['static_certificate'] == v['durability_proved'] == 0 for v in values))

    def test_reopen_can_reuse_or_relocate_descriptor_and_map(self):
        for relocate in (False, True):
            rows = load_trace(FIXTURE/'trace.jsonl'); processes = load_trace(FIXTURE/'processes.jsonl')
            start = rows.index(event(rows,2,'open',epoch=2))
            first_fd = event(rows,2,'open',epoch=1)['result']
            first_map = event(rows,2,'map',epoch=1)['result']
            fd, mapping = (first_fd+5, first_map+4096*64) if relocate else (first_fd, first_map)
            for row in rows[start:]:
                if row['case'] != 702: continue
                if row['kind'] == 'open': row['result'] = fd
                if row['kind'] == 'map': row.update(fd=fd,result=mapping)
                if row['kind'] in ('close','probe'): row['fd'] = fd
                if row['kind'] in ('unmap','probe'): row['map'] = mapping
                if row['kind'] == 'snapshot':
                    chunk = row['context']['streams'][0]['chunks'][0]
                    chunk.update(fd=fd,map=mapping)
            self.assertEqual(len(check_trace(rows, processes)), 4)

    def test_trace_mutations_reject(self):
        original = load_trace(FIXTURE/'trace.jsonl'); processes = load_trace(FIXTURE/'processes.jsonl')
        mutations = {
            'empty':lambda r:r.clear(),
            'missing_case':lambda r:r.__setitem__(slice(None),[x for x in r if x['case'] != 703]),
            'schema':lambda r:r[0].update(schema_version=2),
            'boolean':lambda r:r[0].update(root_length=True),
            'float':lambda r:r[0].update(case=700.0),
            'extra':lambda r:r.append(r[-1]),
            'missing_release':lambda r:r.remove(event(r,0,'free')),
            'overlap':lambda r:event(r,0,'allocation',id=2).update(address=event(r,0,'allocation',id=1)['address']),
            'wrong_generation':lambda r:event(r,0,'free').update(id=1),
            'missing_arm':lambda r:r.remove(event(r,0,'arm_descriptor_failure')),
            'extra_close_armed':lambda r:event(r,0,'arm_descriptor_failure').update(close_count=2),
            'missing_close_suppression':lambda r:r.remove(event(r,0,'suppressed_close')),
            'wrong_close_epoch':lambda r:event(r,0,'suppressed_close').update(epoch=2),
            'wrong_close_remaining':lambda r:event(r,1,'suppressed_close',phase='transition').update(remaining=0),
            'wrong_close_errno':lambda r:event(r,0,'close',result=-1).update(errno=12),
            'close_failure_as_success':lambda r:event(r,0,'close',result=-1).update(result=0,errno=0),
            'wrong_close_fd':lambda r:event(r,0,'close',result=-1).update(fd=99),
            'wrong_unmap_epoch':lambda r:event(r,0,'unmap').update(epoch=2),
            'unmap_failure':lambda r:event(r,0,'unmap').update(result=-1,errno=5),
            'missing_open_suppression':lambda r:r.remove(event(r,2,'suppressed_open')),
            'wrong_open_epoch':lambda r:event(r,2,'open',result=-1).update(epoch=2),
            'wrong_open_errno':lambda r:event(r,2,'open',result=-1).update(errno=12),
            'new_epoch_on_failed_open':lambda r:after(r,2,'up_failed_open').update(fd_epoch=2),
            'invented_fd_on_failed_open':lambda r:after(r,2,'up_failed_open').update(fd=3),
            'invented_map_on_failed_open':lambda r:after(r,2,'up_failed_open').update(map=4096),
            'failed_close_erases_fd':lambda r:after(r,0,'down_failed_close').update(fd=-1),
            'failed_close_retains_map':lambda r:after(r,0,'down_failed_close').update(map=4096),
            'failed_close_retains_size':lambda r:after(r,0,'down_failed_close').update(size=4096),
            'failed_close_retains_content':lambda r:after(r,0,'down_failed_close').update(data_size=4,content='73656564'),
            'failed_close_retains_up_counter':lambda r:context_after(r,0,'down_failed_close').update(up=1),
            'missing_down_ring':lambda r:context_after(r,0,'down_failed_close')['streams'][0]['down_ring'].update(nodes=[]),
            'wrong_ring_edge':lambda r:context_after(r,0,'down_failed_close')['streams'][0]['down_ring'].update(next=0),
            'retry_down_erases_fd':lambda r:after(r,0,'down_retry_unmapped').update(fd=-1),
            'retry_up_erases_fd':lambda r:after(r,0,'up_descriptor_present').update(fd=-1),
            'wrong_probe_inode':lambda r:event(r,0,'probe',label='down_failed_close').update(fd_inode=1),
            'invented_release_probe':lambda r:event(r,0,'probe',label='down_failed_close').update(fd_result=-1,fd_errno=9,fd_device=0,fd_inode=0),
            'invented_mapping_probe':lambda r:event(r,0,'probe',label='down_failed_close').update(map_result=0,map_errno=0),
            'parent_residual_erased':lambda r:event(r,1,'backend_release').update(fd=-1),
            'parent_probe_residual_erased':lambda r:event(r,1,'probe',label='parents_released').update(fd_result=-1,fd_errno=9,fd_device=0,fd_inode=0),
            'wrong_parent_close_epoch':lambda r:event(r,0,'close',phase='teardown').update(epoch=2),
            'missing_parent_close':lambda r:r.remove(event(r,0,'close',phase='teardown')),
            'missing_harness_close':lambda r:r.remove(event(r,1,'harness_close')),
            'harness_wrong_fd':lambda r:event(r,1,'harness_close').update(fd=99),
            'harness_wrong_epoch':lambda r:event(r,1,'harness_close').update(epoch=2),
            'harness_failed':lambda r:event(r,1,'harness_close').update(result=-1,errno=5),
            'harness_still_live':lambda r:event(r,1,'probe',label='harness_reclaimed').update(fd_result=0,fd_errno=0),
            'missing_completion':lambda r:r.remove(event(r,1,'complete')),
            'pending_suppression':lambda r:event(r,1,'complete').update(pending_close=1),
            'reopen_stale_epoch':lambda r:event(r,2,'open',epoch=2).update(epoch=1),
            'map_stale_epoch':lambda r:event(r,2,'map',epoch=2).update(epoch=1),
        }
        for row in original:
            if row['kind'] == 'operation':
                n, name, result = row['case']-700, row['name'], row['result']
                mutations[f'operation:{n}:{name}'] = lambda r,n=n,name=name,result=result:event(r,n,'operation',name=name).update(result=result+1)
        for name, mutate in mutations.items():
            with self.subTest(name=name):
                rows = copy.deepcopy(original); mutate(rows)
                with self.assertRaises(ValueError): check_trace(rows, processes)

    def test_process_mutations_reject(self):
        rows = load_trace(FIXTURE/'trace.jsonl'); original = load_trace(FIXTURE/'processes.jsonl')
        mutations = {
            'missing':lambda p:p.pop(),
            'identity':lambda p:p[0].update(case=600),
            'failed':lambda p:p[0].update(returncode=1),
            'sanitizer':lambda p:p[0].update(stderr=p[0]['stderr']+'LeakSanitizer failed\n'),
            'missing_error':lambda p:p[0].update(stderr=''),
            'reversed_errors':lambda p:p[0].update(stderr='\n'.join(reversed(p[0]['stderr'].splitlines()))),
            'different_error':lambda p:p[0].update(stderr=p[0]['stderr'].replace('errno=5','errno=12')),
            'extra_field':lambda p:p[0].update(static_certificate=1),
            'changed_root':lambda p:p[0].update(root_after=[1,2]),
            'changed_inode':lambda p:p[0].update(initial_inode=1),
            'changed_initial':lambda p:p[0].update(initial_hex=''),
            'changed_final':lambda p:p[1].update(final_hex=''),
            'unexpected_file':lambda p:p[0]['entries'].append(p[0]['entries'][0]),
            'no_cleanup':lambda p:p[2].update(fixture_cleanup_complete=0),
        }
        for name, mutate in mutations.items():
            with self.subTest(name=name):
                processes = copy.deepcopy(original); mutate(processes)
                with self.assertRaises(ValueError): check_trace(rows, processes)


if __name__ == '__main__': unittest.main()
