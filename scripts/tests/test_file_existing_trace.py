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

"""Existing-file errors, inode identity and residual custody must remain observable."""
import copy
from pathlib import Path
import sys
import unittest
sys.path.insert(0,str(Path(__file__).resolve().parents[1]))
from check_file_existing_trace import check_trace,load_trace

FIXTURE=Path(__file__).resolve().parents[2]/'docs/reports/cycle19-file-existing'


class ExistingFileTests(unittest.TestCase):
    def test_native_matrix(self):
        values=[r['values'] for r in check_trace(load_trace(FIXTURE/'trace.jsonl'),load_trace(FIXTURE/'processes.jsonl'))]
        for key,count in [('allocations',70),('releases',70),('transient_releases',7),
                          ('rollback_releases',20),('teardown_releases',43),('injected_failures',3),
                          ('corruption_result',3),('descriptor_residual_after_parents',1)]:
            self.assertEqual(sum(v[key] for v in values),count)
        self.assertEqual(values[2]['harness_descriptor_reclaimed'],1)
        self.assertEqual([v['chunk_null'] for v in values],[0,1,1,1,1,1,0])
        self.assertTrue(all(v['static_certificate']==v['durability_proved']==0 for v in values))

    def test_trace_mutations_reject(self):
        original=load_trace(FIXTURE/'trace.jsonl'); processes=load_trace(FIXTURE/'processes.jsonl')
        # Each edit changes an independently checked semantic boundary.
        edits=[(0,'begin_existing','schema_version',2),(0,'begin_existing','case',300.0),
            (0,'begin_existing','root_length',False),(0,'allocation','id',2),
            (0,'allocation','address',2**64),(0,'allocation','size',128),
            (0,'open','path',8),(0,'open','flags',0),(6,'open','flags',66),
            (0,'open','epoch',2),(1,'chmod','result',0),(1,'chmod','injected',0),
            (2,'close','result',0),(2,'close','fd',99),(2,'close','epoch',2),
            (0,'map','size',8192),(6,'map','prot',3),(0,'map','fd_epoch',2),
            (0,'map','result',1),(4,'unmap','size',4096),(3,'unmap','phase','teardown'),
            (2,'backend_release','fd',-1),(3,'backend_release','fs_size',0),
            (3,'rollback_chunk','error',-13),(4,'rollback_chunk','error',-11),
            (5,'rollback_chunk','error',0),(5,'constructor_result','error',-1),
            (2,'constructor_result','chunk',7),(0,'constructor_result','chunk',0),
            (2,'probe','fd_result',-1),(2,'probe','fd_inode',1),
            (4,'probe','size',4096),(0,'probe','mode',384),
            (0,'chunk_state','state_cleared',0),(0,'counter','up',1),
            (2,'harness_close','fd',99),(2,'harness_close','result',-1),
            (0,'end','live_allocations',1)]
        def event(rows,n,kind): return next(x for x in rows if x['case']==300+n and x['kind']==kind)
        mutations={f'{n}:{kind}:{field}':lambda r,n=n,k=kind,f=field,v=value:event(r,n,k).update({f:v})
                   for n,kind,field,value in edits}
        mutations.update({
            'empty':lambda r:r.clear(),
            'missing_case':lambda r:r.__setitem__(slice(None),[x for x in r if x['case']!=302]),
            'extra':lambda r:r.append(r[-1]),
            'ring':lambda r:event(r,0,'snapshot')['context']['ring'].update(prev=1),
            'payload':lambda r:event(r,0,'snapshot')['context']['streams'][0]['chunks'][0].update(content=''),
            'backend':lambda r:event(r,6,'snapshot')['context']['streams'][0]['chunks'][0].update(backend=9),
            'parent_residual':lambda r:next(x for x in r if x['case']==302 and x['kind']=='probe'
                and x['label']=='parents_released').update(fd_result=-1,fd_errno=9),
            'missing_reclamation':lambda r:r.remove(event(r,2,'harness_close')),
            'missing_release':lambda r:r.remove(event(r,0,'free'))})
        for name,mutate in mutations.items():
            with self.subTest(name=name):
                rows=copy.deepcopy(original);mutate(rows)
                with self.assertRaises(ValueError):check_trace(rows,processes)

    def test_process_mutations_reject(self):
        records=load_trace(FIXTURE/'trace.jsonl'); original=load_trace(FIXTURE/'processes.jsonl')
        mutations={
            'missing':lambda p:p.pop(),'failed':lambda p:p[0].update(returncode=1),
            'sanitizer':lambda p:p[0].update(stderr='LeakSanitizer failed\n'),
            'hidden_acl_error':lambda p:p[1].update(stderr=''),
            'changed_root':lambda p:p[0].update(root_after=[1,2]),
            'changed_inode':lambda p:p[0].update(initial_inode=1),
            'changed_initial':lambda p:p[0].update(initial_hex=''),
            'changed_final':lambda p:p[0].update(final_hex=''),
            'replaced_file':lambda p:p[0]['entries'][1].update(inode=1),
            'aliased_file':lambda p:p[0]['entries'][1].update(inode=p[0]['entries'][0]['inode']),
            'extra_file':lambda p:p[0]['entries'].append(p[0]['entries'][0]),
            'no_cleanup':lambda p:p[2].update(fixture_cleanup_complete=0)}
        for name,mutate in mutations.items():
            with self.subTest(name=name):
                processes=copy.deepcopy(original);mutate(processes)
                with self.assertRaises(ValueError):check_trace(records,processes)


if __name__=='__main__':unittest.main()
