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

"""Reject evidence that equates a failed reopen with descriptor or mapping release."""
import copy
from pathlib import Path
import sys
import unittest
sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from check_file_reopen_failure_trace import check_trace, load_trace

FIXTURE = Path(__file__).resolve().parents[2] / 'docs/reports/cycle19-reopen-failures'
MAP_FAILED = 2**64 - 1


def event(rows, scenario, kind, **fields):
    return next(r for r in rows if r['case'] == 800+scenario and r['kind'] == kind
                and all(r.get(k) == v for k, v in fields.items()))


def context_after(rows, scenario, name):
    return rows[rows.index(event(rows, scenario, 'operation', name=name))+2]['context']


def after(rows, scenario, name):
    return context_after(rows, scenario, name)['streams'][0]['chunks'][0]


class ReopenFailureTests(unittest.TestCase):
    def test_native_matrix(self):
        observations = check_trace(load_trace(FIXTURE/'trace.jsonl'),
                                   load_trace(FIXTURE/'processes.jsonl'))
        values = [row['values'] for row in observations]
        for key, count in [('allocations',50), ('releases',50), ('transient_releases',5),
                           ('teardown_releases',45), ('open_epochs',10), ('mapping_epochs',6),
                           ('library_mapping_releases',6), ('library_descriptor_releases',8),
                           ('parent_descriptor_residuals',2), ('harness_descriptor_releases',2),
                           ('suppressed_size_lookups',5), ('suppressed_map_calls',2),
                           ('stat_fallback_completes_reopen',1),
                           ('failed_size_lookup_retains_descriptor',2),
                           ('failed_map_retains_descriptor',2),
                           ('reopen_failure_retains_new_descriptor',4),
                           ('failed_map_reports_up_ring_without_counter',2),
                           ('down_retry_rejects_without_new_epoch',4),
                           ('up_retry_rejects_without_new_epoch',3),
                           ('invalid_map_unmap_failures',4),
                           ('sync_succeeds_without_mapping',1)]:
            self.assertEqual(sum(v[key] for v in values), count, key)
        self.assertTrue(all(v['static_certificate'] == v['durability_proved'] == 0 for v in values))

    def test_reopen_can_reuse_or_relocate_descriptor_and_map(self):
        for relocate in (False, True):
            rows = load_trace(FIXTURE/'trace.jsonl')
            processes = load_trace(FIXTURE/'processes.jsonl')
            start = rows.index(event(rows, 0, 'open', epoch=2))
            first_fd = event(rows, 0, 'open', epoch=1)['result']
            first_map = event(rows, 0, 'map', epoch=1)['result']
            fd, mapping = (first_fd+5, first_map+4096*64) if relocate else (first_fd, first_map)
            for row in rows[start:]:
                if row['case'] != 800: continue
                if row['kind'] == 'open': row['result'] = fd
                if row['kind'] == 'map': row.update(fd=fd, result=mapping)
                if row['kind'] in ('close','probe'): row['fd'] = fd
                if row['kind'] in ('unmap','probe'): row['map'] = mapping
                if row['kind'] == 'snapshot':
                    chunk = row['context']['streams'][0]['chunks'][0]
                    chunk.update(fd=fd, map=mapping)
            self.assertEqual(len(check_trace(rows, processes)), 5)

    def test_trace_mutations_reject(self):
        original = load_trace(FIXTURE/'trace.jsonl')
        processes = load_trace(FIXTURE/'processes.jsonl')
        mutations = {
            'empty':lambda r:r.clear(),
            'missing_case':lambda r:r.__setitem__(slice(None),[x for x in r if x['case'] != 804]),
            'schema':lambda r:r[0].update(schema_version=2),
            'boolean':lambda r:r[0].update(root_length=True),
            'float':lambda r:r[0].update(case=800.0),
            'extra':lambda r:r.append(r[-1]),
            'missing_release':lambda r:r.remove(event(r,0,'free')),
            'overlap':lambda r:event(r,0,'allocation',id=2).update(
                address=event(r,0,'allocation',id=1)['address']),
            'wrong_generation':lambda r:event(r,0,'free').update(id=1),
            'missing_arm':lambda r:r.remove(event(r,1,'arm_reopen_failure')),
            'extra_size_armed':lambda r:event(r,1,'arm_reopen_failure').update(stat_count=2),
            'extra_map_armed':lambda r:event(r,3,'arm_reopen_failure').update(map_count=2),
            'missing_map_suppression':lambda r:r.remove(event(r,3,'suppressed_map')),
            'wrong_map_suppression_epoch':lambda r:event(r,3,'suppressed_map').update(epoch=1),
            'wrong_map_suppression_remaining':lambda r:event(r,4,'suppressed_map').update(remaining=1),
            'suppressed_map_as_success':lambda r:event(r,3,'map',epoch=1).update(
                result=4096, errno=0),
            'suppressed_map_new_epoch':lambda r:event(r,3,'map',result=MAP_FAILED).update(epoch=2),
            'missing_fstat_suppression':lambda r:r.remove(
                event(r,1,'size_lookup',call='fstat',injected=1)),
            'missing_stat_suppression':lambda r:r.remove(
                event(r,1,'size_lookup',call='stat',injected=1)),
            'stat_fallback_claimed_as_failure':lambda r:event(
                r,0,'size_lookup',call='stat').update(result=-1,errno=5,injected=1,size=-1),
            'size_failure_reports_size':lambda r:event(
                r,1,'size_lookup',call='stat',injected=1).update(size=4096),
            'wrong_size_lookup_epoch':lambda r:event(
                r,1,'size_lookup',call='stat',injected=1).update(epoch=1),
            'unarmed_size_lookup_injected':lambda r:event(
                r,3,'size_lookup',phase='transition',epoch=2).update(injected=1),
            'reopen_stale_epoch':lambda r:event(r,1,'open',epoch=2).update(epoch=1),
            'reopen_failed_open':lambda r:event(r,1,'open',epoch=2).update(result=-1,errno=5),
            'failed_size_lookup_erases_fd':lambda r:after(
                r,1,'up_failed_size_lookup').update(fd=-1),
            'failed_size_lookup_invents_map':lambda r:after(
                r,1,'up_failed_size_lookup').update(map=4096),
            'failed_size_lookup_keeps_size':lambda r:after(
                r,1,'up_failed_size_lookup').update(fs_size=4096),
            'failed_size_lookup_claims_up_ring':lambda r:swap_rings(context_after(
                r,1,'up_failed_size_lookup')['streams'][0]),
            'failed_size_lookup_denies_down_ring':lambda r:context_after(
                r,1,'up_failed_size_lookup')['streams'][0]['down_ring'].update(nodes=[]),
            'failed_size_lookup_claims_counter':lambda r:context_after(
                r,1,'up_failed_size_lookup').update(up=1),
            'failed_map_erases_fd':lambda r:after(r,3,'up_failed_map').update(fd=-1),
            'failed_map_clears_field':lambda r:after(r,3,'up_failed_map').update(map=0),
            'failed_map_clears_alloc_size':lambda r:after(r,3,'up_failed_map').update(size=0),
            'failed_map_invents_counter':lambda r:context_after(r,3,'up_failed_map').update(up=1),
            'failed_map_denies_up_ring':lambda r:context_after(
                r,3,'up_failed_map')['streams'][0]['up_ring'].update(nodes=[]),
            'wrong_ring_edge':lambda r:context_after(
                r,3,'up_failed_map')['streams'][0]['up_ring'].update(next=0),
            'retry_down_erases_fd':lambda r:after(r,1,'down_retry_unmapped').update(fd=-1),
            'retry_up_erases_fd':lambda r:after(r,1,'up_retry_descriptor_present').update(fd=-1),
            'invalid_unmap_as_success':lambda r:event(
                r,3,'unmap',map=MAP_FAILED,phase='transition').update(result=0,errno=0),
            'invalid_unmap_wrong_errno':lambda r:event(
                r,3,'unmap',map=MAP_FAILED,phase='transition').update(errno=5),
            'missing_invalid_unmap':lambda r:r.remove(
                event(r,3,'unmap',map=MAP_FAILED,phase='teardown')),
            'sync_claims_failure':lambda r:event(
                r,4,'operation',name='sync_after_failed_map').update(result=-1),
            'sync_invents_mapping':lambda r:after(r,4,'sync_after_failed_map').update(map=4096),
            'wrong_probe_inode':lambda r:event(
                r,1,'probe',label='up_failed_size_lookup').update(fd_inode=1),
            'invented_release_probe':lambda r:event(
                r,1,'probe',label='up_failed_size_lookup').update(
                    fd_result=-1,fd_errno=9,fd_device=0,fd_inode=0),
            'invented_mapping_probe':lambda r:event(
                r,3,'probe',label='up_failed_map').update(map_result=0,map_errno=0),
            'parent_residual_erased':lambda r:event(r,2,'backend_release').update(fd=-1),
            'parent_map_residual_erased':lambda r:event(r,4,'backend_release').update(map=0),
            'parent_probe_residual_erased':lambda r:event(
                r,2,'probe',label='parents_released').update(
                    fd_result=-1,fd_errno=9,fd_device=0,fd_inode=0),
            'wrong_parent_close_epoch':lambda r:event(r,0,'close',phase='teardown').update(epoch=1),
            'missing_parent_close':lambda r:r.remove(event(r,1,'close',phase='teardown')),
            'parent_close_failure_as_success':lambda r:event(
                r,2,'close',phase='teardown').update(result=0,errno=0,injected=0),
            'missing_harness_close':lambda r:r.remove(event(r,2,'harness_close')),
            'harness_wrong_fd':lambda r:event(r,4,'harness_close').update(fd=99),
            'harness_wrong_epoch':lambda r:event(r,4,'harness_close').update(epoch=1),
            'harness_failed':lambda r:event(r,4,'harness_close').update(result=-1,errno=5),
            'harness_still_live':lambda r:event(
                r,2,'probe',label='harness_reclaimed').update(fd_result=0,fd_errno=0),
            'harness_close_counted_as_library':lambda r:r.insert(
                rows_index(r,2,'close',phase='teardown'), copy.deepcopy(event(r,2,'harness_close'))),
            'missing_completion':lambda r:r.remove(event(r,1,'complete')),
            'pending_suppression':lambda r:event(r,1,'complete').update(pending_stat=1),
            'missing_counter_release':lambda r:r.remove(
                event(r,0,'exit',function='cio_chunk_counter_total_up_sub')),
            'invented_counter_release':lambda r:r.insert(
                rows_index(r,3,'exit',function='cio_file_native_unmap')+1,
                copy.deepcopy(event(r,0,'enter',function='cio_chunk_counter_total_up_sub'))),
        }
        for row in original:
            if row['kind'] == 'operation':
                n, name, result = row['case']-800, row['name'], row['result']
                mutations[f'operation:{n}:{name}'] = (
                    lambda r, n=n, name=name, result=result:
                    event(r, n, 'operation', name=name).update(result=result+1))
        for name, mutate in mutations.items():
            with self.subTest(name=name):
                rows = copy.deepcopy(original); mutate(rows)
                with self.assertRaises(ValueError): check_trace(rows, processes)

    def test_process_mutations_reject(self):
        rows = load_trace(FIXTURE/'trace.jsonl')
        original = load_trace(FIXTURE/'processes.jsonl')
        mutations = {
            'missing':lambda p:p.pop(),
            'identity':lambda p:p[0].update(case=700),
            'failed':lambda p:p[0].update(returncode=1),
            'sanitizer':lambda p:p[0].update(stderr=p[0]['stderr']+'LeakSanitizer failed\n'),
            'missing_error':lambda p:p[3].update(stderr=''),
            'reversed_errors':lambda p:p[4].update(
                stderr='\n'.join(reversed(p[4]['stderr'].splitlines()))+'\n'),
            'different_error':lambda p:p[3].update(stderr=p[3]['stderr'].replace('errno=22','errno=5')),
            'silent_failure':lambda p:p[0].update(stderr=p[3]['stderr']),
            'extra_field':lambda p:p[0].update(static_certificate=1),
            'changed_root':lambda p:p[0].update(root_after=[1,2]),
            'changed_inode':lambda p:p[0].update(initial_inode=1),
            'changed_initial':lambda p:p[0].update(initial_hex=''),
            'changed_final':lambda p:p[3].update(final_hex=''),
            'unexpected_file':lambda p:p[0]['entries'].append(p[0]['entries'][0]),
            'no_cleanup':lambda p:p[2].update(fixture_cleanup_complete=0),
        }
        for name, mutate in mutations.items():
            with self.subTest(name=name):
                processes = copy.deepcopy(original); mutate(processes)
                with self.assertRaises(ValueError): check_trace(rows, processes)


def rows_index(rows, scenario, kind, **fields):
    return rows.index(event(rows, scenario, kind, **fields))


def swap_rings(stream):
    stream['up_ring']['nodes'], stream['down_ring']['nodes'] = (
        stream['down_ring']['nodes'], stream['up_ring']['nodes'])


if __name__ == '__main__': unittest.main()
