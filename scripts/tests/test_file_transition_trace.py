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

"""Native rollback and lock flags must not gain stronger semantics during replay."""
import copy
from pathlib import Path
import sys
import unittest
sys.path.insert(0,str(Path(__file__).resolve().parents[1]))
from check_file_transition_trace import check_trace,load_trace

FIXTURE=Path(__file__).resolve().parents[2]/'docs/reports/cycle19-file-transitions'


def event(rows,n,kind,**fields):
    return next(r for r in rows if r['case']==400+n and r['kind']==kind
                and all(r.get(k)==v for k,v in fields.items()))


def after(rows,n,operation):
    index=rows.index(event(rows,n,'operation',name=operation))
    return rows[index+2]['context']['streams'][0]['chunks'][0]


class TransitionTests(unittest.TestCase):
    def test_native_matrix(self):
        rows=check_trace(load_trace(FIXTURE/'trace.jsonl'),load_trace(FIXTURE/'processes.jsonl'))
        values=[r['values'] for r in rows]
        for key,count in [('allocations',50),('releases',50),('transient_releases',5),
                          ('teardown_releases',45),('open_epochs',7),('mapping_epochs',7),
                          ('logical_rollback',2),('reloaded_appended_bytes',1),('locked_write_succeeded',1)]:
            self.assertEqual(sum(v[key] for v in values),count)
        self.assertEqual(values[3]['locked_begin_retry'],1)
        self.assertTrue(all(v['static_certificate']==v['durability_proved']==0 for v in values))

    def test_reopened_mapping_may_change_address(self):
        rows=load_trace(FIXTURE/'trace.jsonl');processes=load_trace(FIXTURE/'processes.jsonl')
        start=rows.index(event(rows,0,'map',epoch=2));old=rows[start]['result'];new=old+4096*64
        def relocate(value):
            if isinstance(value,dict):return {k:relocate(v) for k,v in value.items()}
            if isinstance(value,list):return [relocate(v) for v in value]
            return new if value==old else value
        for i in range(start,len(rows)):
            if rows[i]['case']==400:rows[i]=relocate(rows[i])
        self.assertEqual(len(check_trace(rows,processes)),5)

    def test_trace_mutations_reject(self):
        original=load_trace(FIXTURE/'trace.jsonl');processes=load_trace(FIXTURE/'processes.jsonl')
        mutations={
            'empty':lambda r:r.clear(),
            'missing_case':lambda r:r.__setitem__(slice(None),[x for x in r if x['case']!=404]),
            'schema':lambda r:r[0].update(schema_version=2),
            'boolean':lambda r:r[0].update(root_length=True),
            'float':lambda r:r[0].update(case=400.0),
            'allocation_overflow':lambda r:event(r,0,'allocation').update(address=2**64),
            'missing_allocation_release':lambda r:r.remove(event(r,0,'free')),
            'extra':lambda r:r.append(r[-1]),
            'stale_open_epoch':lambda r:event(r,0,'open',epoch=2).update(epoch=1),
            'stale_map_epoch':lambda r:event(r,0,'map',epoch=2).update(epoch=1),
            'stale_map_fd_epoch':lambda r:event(r,0,'map',epoch=2).update(fd_epoch=1),
            'foreign_map_fd':lambda r:event(r,0,'map',epoch=2).update(fd=99),
            'stale_unmap_epoch':lambda r:event(r,0,'unmap',epoch=2).update(epoch=1),
            'stale_close_epoch':lambda r:event(r,0,'close',epoch=2).update(epoch=1),
            'invalid_new_mapping':lambda r:event(r,0,'map',epoch=2).update(result=1),
            'wrong_map_mode':lambda r:event(r,0,'map',epoch=2).update(prot=1),
            'down_keeps_fd':lambda r:after(r,0,'down').update(fd=3),
            'down_keeps_length':lambda r:after(r,0,'down').update(data_size=4),
            'rollback_erases_header':lambda r:after(r,1,'rollback').update(header_length=4),
            'rollback_keeps_length':lambda r:after(r,1,'rollback').update(data_size=7),
            'rollback_retains_transaction':lambda r:after(r,1,'rollback').update(transaction=1),
            'rollback_erases_checkpoint':lambda r:after(r,1,'rollback').update(tx_length=0),
            'rollback_erases_unsynced':lambda r:after(r,1,'rollback').update(synced=1),
            'reload_invents_logical_rollback':lambda r:after(r,4,'up').update(data_size=4,content=b'seed'.hex()),
            'locked_write_rejected':lambda r:event(r,3,'operation',name='write_locked').update(result=-2),
            'locked_begin_accepted':lambda r:event(r,3,'operation',name='begin_locked').update(result=0),
            'locked_begin_transaction':lambda r:after(r,3,'begin_locked').update(transaction=1),
            'locked_write_erased':lambda r:after(r,3,'write_locked').update(data_size=4,header_length=4),
            'lock_cleared_by_write':lambda r:after(r,3,'write_locked').update(locked=0),
            'commit_not_synced':lambda r:after(r,2,'commit').update(synced=0),
            'commit_transaction_live':lambda r:after(r,2,'commit').update(transaction=1),
            'missing_sync':lambda r:r.remove(event(r,2,'sync')),
            'wrong_sync_flags':lambda r:event(r,2,'sync').update(flags=4),
            'wrong_sync_epoch':lambda r:event(r,4,'sync').update(epoch=2),
            'wrong_sync_phase':lambda r:event(r,1,'sync').update(phase='transition'),
            'down_probe_live':lambda r:event(r,0,'probe',label='down').update(map_result=0,map_errno=0),
            'reopen_wrong_inode':lambda r:event(r,0,'probe',label='up').update(fd_inode=1),
            'terminal_live_descriptor':lambda r:event(r,4,'probe',label='parents_released').update(fd_result=0,fd_errno=0),
            'terminal_live_mapping':lambda r:event(r,4,'probe',label='parents_released').update(map_result=0,map_errno=0),
            'wrong_parent_counter':lambda r:event(r,4,'context_state').update(up=1),
            'missing_teardown':lambda r:r.remove(event(r,4,'enter')),
            'ring':lambda r:event(r,0,'snapshot')['context']['streams'][0]['up_ring'].update(next=1)}
        # All 26 public operation results are independently checked, including
        # repeated begin/down/up/lock/unlock/rollback calls.
        operations=[(r['case']-400,r['name'],r['result']) for r in original if r['kind']=='operation']
        for n,name,result in operations:
            mutations[f'operation:{n}:{name}']=lambda r,n=n,name=name,result=result:event(r,n,'operation',name=name).update(result=result+1)
        for name,mutate in mutations.items():
            with self.subTest(name=name):
                rows=copy.deepcopy(original);mutate(rows)
                with self.assertRaises(ValueError):check_trace(rows,processes)

    def test_process_mutations_reject(self):
        rows=load_trace(FIXTURE/'trace.jsonl');original=load_trace(FIXTURE/'processes.jsonl')
        mutations={
            'missing':lambda p:p.pop(),'identity':lambda p:p[0].update(case=300),
            'failed':lambda p:p[0].update(returncode=1),
            'sanitizer':lambda p:p[0].update(stderr='LeakSanitizer failed\n'),
            'changed_root':lambda p:p[0].update(root_after=[1,2]),
            'changed_inode':lambda p:p[0].update(initial_inode=1),
            'changed_initial':lambda p:p[0].update(initial_hex=''),
            'invented_byte_rollback':lambda p:p[1].update(final_hex=p[1]['initial_hex']),
            'invented_lock_guard':lambda p:p[3].update(final_hex=p[3]['initial_hex']),
            'unexpected_file':lambda p:p[0]['entries'].append(p[0]['entries'][0]),
            'no_cleanup':lambda p:p[2].update(fixture_cleanup_complete=0)}
        for name,mutate in mutations.items():
            with self.subTest(name=name):
                processes=copy.deepcopy(original);mutate(processes)
                with self.assertRaises(ValueError):check_trace(rows,processes)


if __name__=='__main__':unittest.main()
