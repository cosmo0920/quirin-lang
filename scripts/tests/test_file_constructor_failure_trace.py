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

"""Filesystem ENOMEM outcomes must preserve interrupted, rollback and OS records."""
import copy
from pathlib import Path
import sys
import unittest
sys.path.insert(0,str(Path(__file__).resolve().parents[1]))
from check_file_constructor_failure_trace import check_trace, load_trace

FIXTURE = Path(__file__).resolve().parents[2]/"docs/reports/cycle19-file-constructor-failures"


class FileConstructorFailureTests(unittest.TestCase):
    def test_native_matrix(self):
        results = check_trace(load_trace(FIXTURE/"trace.jsonl"),load_trace(FIXTURE/"processes.jsonl"))
        values = [r["values"] for r in results]
        self.assertEqual(len(values),12)
        self.assertEqual(sum(v["teardown_completed"] for v in values),10)
        self.assertEqual(sum(v["sanitizer_interrupted"] for v in values),2)
        self.assertEqual(results[3]["prefix_outstanding_generations"],[1,2])
        self.assertEqual(results[9]["prefix_outstanding_generations"],[1,2,3,6,7,8])
        self.assertEqual(values[2]["nullable_chmod"],1)
        self.assertEqual(values[5]["stream_without_directory"],1)
        for v in values:
            self.assertEqual(v["allocations"],v["releases"]+v["prefix_outstanding"])
            self.assertEqual(v["native_leak_check_completed"],v["teardown_completed"])
            self.assertEqual(v["static_certificate"],0)

    def test_trace_mutations_reject(self):
        original = load_trace(FIXTURE/"trace.jsonl")
        processes = load_trace(FIXTURE/"processes.jsonl")
        def event(rows,fail,kind):
            return next(r for r in rows if r["case"]==100+fail and r["kind"]==kind)
        def remove(rows,fail,kind):
            rows.remove(event(rows,fail,kind))
            for i,row in enumerate(r for r in rows if r["case"]==100+fail): row["sequence"]=i
        def chunk(rows): return event(rows,0,"snapshot")["context"]["streams"][0]["chunks"][0]
        mutations = {
            "empty":lambda r:r.clear(),
            "missing_interrupted":lambda r:r.__setitem__(slice(None),[x for x in r if x["case"]!=103]),
            "schema":lambda r:r[0].update(schema_version=2),
            "float":lambda r:r[0].update(case=100.0),
            "boolean":lambda r:r[0].update(fail_at=False),
            "root_length":lambda r:r[0].update(root_length=28),
            "attempt":lambda r:event(r,3,"attempt").update(ordinal=3),
            "hidden_failure":lambda r:next(x for x in r if x["case"]==103 and x.get("failed")==1).update(failed=0),
            "bad_size":lambda r:event(r,0,"allocation").update(size=128),
            "wrapped_address":lambda r:event(r,0,"allocation").update(address=2**64),
            "wrong_generation":lambda r:event(r,9,"allocation").update(id=2),
            "missing_context_return":lambda r:remove(r,3,"context_result"),
            "erased_null_root":lambda r:event(r,3,"context_result").update(root=2),
            "erased_null_chmod":lambda r:event(r,2,"context_result").update(chmod=2),
            "wrong_stream_return":lambda r:event(r,5,"stream_result").update(stream=0),
            "wrong_return":lambda r:event(r,5,"constructor_result").update(chunk=7),
            "wrong_error":lambda r:event(r,10,"constructor_result").update(error=0),
            "missing_temporary":lambda r:remove(r,5,"free"),
            "wrong_rollback":lambda r:next(x for x in r if x["case"]==111 and x["kind"]=="free" and x["id"]==10).update(id=8),
            "foreign_backend":lambda r:chunk(r).update(backend=6),
            "wrong_parent":lambda r:chunk(r).update(context=6),
            "wrong_inode":lambda r:chunk(r).update(inode=1),
            "wrong_ring":lambda r:event(r,0,"snapshot")["context"]["ring"].update(next=1),
            "wrong_up":lambda r:event(r,0,"snapshot")["context"].update(up=0),
            "wrong_syscall":lambda r:event(r,0,"os_unmap").update(map=4096),
            "failed_close":lambda r:event(r,0,"os_close").update(result=-1),
            "missing_parent":lambda r:remove(r,2,"enter"),
            "not_unlinked":lambda r:event(r,0,"chunk_state").update(primary_cleared=0),
            "descriptor_residual":lambda r:event(r,0,"os_terminal").update(fd_result=0,fd_errno=0),
            "mapping_residual":lambda r:event(r,0,"os_terminal").update(map_result=0,map_errno=0),
            "orphan":lambda r:event(r,0,"end").update(live_allocations=1),
            "trailing":lambda r:r.append(r[-1]),
        }
        for name,mutate in mutations.items():
            with self.subTest(name=name):
                records=copy.deepcopy(original);mutate(records)
                with self.assertRaises(ValueError): check_trace(records,processes)

    def test_process_mutations_reject(self):
        records=load_trace(FIXTURE/"trace.jsonl")
        original=load_trace(FIXTURE/"processes.jsonl")
        mutations={
            "missing":lambda p:p.pop(),
            "wrong_case":lambda p:p[3].update(case=109),
            "generic_failure":lambda p:p[3].update(stderr="segmentation fault\n"),
            "signal":lambda p:p[3].update(returncode=-11),
            "false_completion":lambda p:p[3].update(returncode=0),
            "bad_return":lambda p:p[0].update(returncode=1),
            "ptrace":lambda p:p[0].update(stderr="LeakSanitizer does not work under ptrace\n"),
            "source_token_collision":lambda p:p[3].update(stderr=p[3]["stderr"].replace("cio_stream.c:65:18","cio_streamxc:65:18")),
            "different_summary_root":lambda p:p[3].update(stderr=p[3]["stderr"].replace("SUMMARY: UndefinedBehaviorSanitizer: undefined-behavior /","SUMMARY: UndefinedBehaviorSanitizer: undefined-behavior /different/")),
            "other_source":lambda p:p[3].update(stderr=p[9]["stderr"]),
            "missing_summary":lambda p:p[3].update(stderr=p[3]["stderr"].split("SUMMARY:")[0]),
            "truncated_stack":lambda p:p[3].update(stderr=p[3]["stderr"].splitlines()[0]+"\n"+p[3]["stderr"].splitlines()[-1]+"\n"),
            "wrong_caller":lambda p:p[9].update(stderr=p[9]["stderr"].replace("cio_chunk_open","different_caller")),
            "extra_before_summary":lambda p:p[3].update(stderr=p[3]["stderr"].replace("SUMMARY:","unexpected\nSUMMARY:")),
            "missing_frame":lambda p:p[3].update(stderr=p[3]["stderr"].replace("check_stream_path","unknown")),
            "extra_diagnostic":lambda p:p[3].update(stderr=p[3]["stderr"]+"error\n"),
            "root_replaced":lambda p:p[0].update(root_after=[1,2]),
            "erased_file":lambda p:p[0]["entries"].pop(),
            "unexpected_directory":lambda p:p[5].update(entries=p[6]["entries"]),
            "wrong_file_identity":lambda p:p[0]["entries"][1].update(inode=1),
            "aliased_inode":lambda p:p[0]["entries"][1].update(inode=p[0]["entries"][0]["inode"]),
            "cleanup_missing":lambda p:p[9].update(fixture_cleanup_complete=0),
            "extra_field":lambda p:p[0].update(static_certificate=1),
        }
        for name,mutate in mutations.items():
            with self.subTest(name=name):
                processes=copy.deepcopy(original);mutate(processes)
                with self.assertRaises(ValueError): check_trace(records,processes)


if __name__ == "__main__": unittest.main()
