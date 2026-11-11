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

"""Constructor rollback must not erase suppressed-close descriptor custody."""
import copy
from pathlib import Path
import sys
import unittest
sys.path.insert(0,str(Path(__file__).resolve().parents[1]))
from check_file_os_constructor_trace import check_trace,load_trace

FIXTURE=Path(__file__).resolve().parents[2]/"docs/reports/cycle19-file-os-constructors"


class OsConstructorTests(unittest.TestCase):
    def test_native_matrix(self):
        rows=check_trace(load_trace(FIXTURE/"trace.jsonl"),load_trace(FIXTURE/"processes.jsonl"))
        values=[row["values"] for row in rows]
        self.assertEqual(len(rows),8)
        for key,count in (("allocations",88),("releases",88),("transient_releases",16),
                          ("rollback_releases",24),("teardown_releases",48),("injected_failures",10)):
            self.assertEqual(sum(v[key] for v in values),count)
        self.assertEqual(values[2]["fallback_stat_succeeded"],1)
        self.assertEqual(values[4]["posix_error_distinct_from_errno"],1)
        self.assertEqual(values[6]["descriptor_residual_after_return"],1)
        self.assertEqual(values[6]["descriptor_residual_after_parents"],1)
        self.assertEqual(values[6]["harness_descriptor_reclaimed"],1)
        self.assertTrue(all(v["static_certificate"]==v["durability_proved"]==0 for v in values))

    def test_trace_mutations_reject(self):
        original=load_trace(FIXTURE/"trace.jsonl");processes=load_trace(FIXTURE/"processes.jsonl")
        def event(rows,scenario,kind):
            return next(r for r in rows if r["case"]==200+scenario and r["kind"]==kind)
        def remove(rows,scenario,kind):
            rows.remove(event(rows,scenario,kind))
            for index,row in enumerate(r for r in rows if r["case"]==200+scenario):row["sequence"]=index
        def probe(rows,phase="parents_released"):
            return next(r for r in rows if r["case"]==206 and r["kind"]=="constructor_probe" and r["phase"]==phase)
        def chunk(rows):return event(rows,0,"snapshot")["context"]["streams"][0]["chunks"][0]
        mutations={
            "empty":lambda r:r.clear(),
            "missing_case":lambda r:r.__setitem__(slice(None),[x for x in r if x["case"]!=206]),
            "schema":lambda r:r[0].update(schema_version=2),
            "boolean":lambda r:r[0].update(scenario=False),
            "float":lambda r:r[0].update(case=200.0),
            "generation":lambda r:event(r,0,"allocation").update(id=2),
            "address_wrap":lambda r:event(r,0,"allocation").update(address=2**64),
            "allocation_size":lambda r:event(r,0,"allocation").update(size=128),
            "missing_temporary":lambda r:remove(r,0,"free"),
            "open_failed_but_fd":lambda r:event(r,1,"constructor_open").update(result=3),
            "wrong_open_flags":lambda r:event(r,0,"constructor_open").update(flags=0),
            "foreign_path":lambda r:event(r,0,"constructor_open").update(path=9),
            "wrong_open_inode":lambda r:event(r,0,"constructor_open").update(inode=1),
            "hidden_fstat_failure":lambda r:event(r,2,"constructor_fstat").update(injected=0),
            "fallback_omitted":lambda r:remove(r,2,"constructor_stat"),
            "wrong_fallback_inode":lambda r:event(r,2,"constructor_stat").update(inode=1),
            "wrong_second_lookup":lambda r:next(x for x in r if x["case"]==207 and x["kind"]=="constructor_fstat" and x["ordinal"]==2).update(ordinal=1),
            "wrong_chmod":lambda r:event(r,0,"constructor_chmod").update(mode=384),
            "resize_minus_one":lambda r:event(r,4,"constructor_resize").update(result=-1),
            "resize_wrong_errno":lambda r:event(r,4,"constructor_resize").update(errno=28),
            "resize_wrong_prior_errno":lambda r:event(r,4,"constructor_resize").update(errno_before=0),
            "resize_foreign_fd":lambda r:event(r,0,"constructor_resize").update(fd=99),
            "map_null_sentinel":lambda r:event(r,5,"constructor_map").update(result=0),
            "map_wrong_length":lambda r:event(r,0,"constructor_map").update(length=8192),
            "map_readonly":lambda r:event(r,0,"constructor_map").update(prot=1),
            "missing_close":lambda r:remove(r,5,"constructor_close"),
            "forged_close_success":lambda r:event(r,6,"constructor_close").update(result=0),
            "erased_backend_fd":lambda r:event(r,6,"rollback_backend").update(fd=-1),
            "erased_failed_map":lambda r:event(r,5,"rollback_backend").update(map=0),
            "erased_file_growth":lambda r:event(r,5,"rollback_backend").update(fs_size=0),
            "rollback_counter":lambda r:event(r,5,"rollback_backend").update(up=1),
            "false_chunk_return":lambda r:event(r,6,"constructor_result").update(chunk=8),
            "wrong_error":lambda r:event(r,4,"constructor_result").update(error=28),
            "erased_return_residual":lambda r:probe(r,"returned").update(fd_result=-1,fd_errno=9),
            "erased_parent_residual":lambda r:probe(r).update(fd_result=-1,fd_errno=9),
            "foreign_residual_inode":lambda r:probe(r).update(fd_inode=1),
            "foreign_residual_fd":lambda r:probe(r).update(fd=99),
            "erased_path_growth":lambda r:probe(r).update(path_size=0),
            "wrong_ring":lambda r:event(r,0,"snapshot")["context"]["ring"].update(next=1),
            "foreign_backend":lambda r:chunk(r).update(backend=6),
            "false_unlink":lambda r:event(r,0,"chunk_state").update(state_cleared=0),
            "missing_parent":lambda r:remove(r,6,"enter"),
            "mapping_residual":lambda r:event(r,0,"terminal_mapping").update(result=0,errno=0),
            "missing_reclamation":lambda r:remove(r,6,"harness_descriptor_cleanup"),
            "wrong_reclamation_fd":lambda r:event(r,6,"harness_descriptor_cleanup").update(fd=99),
            "failed_reclamation":lambda r:event(r,6,"harness_descriptor_cleanup").update(result=-1,errno=5),
            "unreleased_allocation":lambda r:event(r,0,"end").update(live_allocations=1),
            "extra_record":lambda r:r.append(r[-1]),
        }
        for name,mutate in mutations.items():
            with self.subTest(name=name):
                rows=copy.deepcopy(original);mutate(rows)
                with self.assertRaises(ValueError):check_trace(rows,processes)

    def test_process_mutations_reject(self):
        records=load_trace(FIXTURE/"trace.jsonl");original=load_trace(FIXTURE/"processes.jsonl")
        mutations={
            "missing":lambda p:p.pop(),
            "case":lambda p:p[6].update(case=205),
            "exit_failure":lambda p:p[6].update(returncode=1),
            "sanitizer":lambda p:p[0].update(stderr="AddressSanitizer error\n"),
            "wrong_posix_errno":lambda p:p[4].update(stderr=p[4]["stderr"].replace("errno=95","errno=28")),
            "wrong_source":lambda p:p[6].update(stderr=p[5]["stderr"]),
            "root_identity":lambda p:p[0].update(root_after=[1,2]),
            "aliased_file":lambda p:p[0]["entries"][1].update(inode=p[0]["entries"][0]["inode"]),
            "foreign_file":lambda p:p[6]["entries"][1].update(inode=1),
            "erased_file":lambda p:p[6]["entries"].pop(),
            "unexpected_created_file":lambda p:p[1].update(entries=p[0]["entries"]),
            "fixture_cleanup":lambda p:p[6].update(fixture_cleanup_complete=0),
        }
        for name,mutate in mutations.items():
            with self.subTest(name=name):
                processes=copy.deepcopy(original);mutate(processes)
                with self.assertRaises(ValueError):check_trace(records,processes)


if __name__=="__main__":unittest.main()
