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

"""Native filesystem parent release must retain observed OS residuals."""
import copy
from pathlib import Path
import sys
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from check_file_teardown_trace import check_trace, load_trace

FIXTURE = Path(__file__).resolve().parents[2] / "docs/reports/cycle19-file-tree/trace.jsonl"


class FileTeardownTests(unittest.TestCase):
    def test_native_matrix(self):
        values = [r["values"] for r in check_trace(load_trace(FIXTURE))]
        self.assertEqual(len(values), 6)
        self.assertEqual(sum(v["allocations"] for v in values), 66)
        self.assertEqual(sum(v["transient_releases"] for v in values), 12)
        self.assertEqual(values[3]["mapping_residual"], 1)
        self.assertEqual(values[3]["up_residual"], 1)
        self.assertEqual(values[4]["descriptor_residual"], 1)
        self.assertEqual(values[5]["sync_failed"], 1)
        self.assertTrue(all(v["parent_allocations_released"] and not v["static_certificate"]
                            and not v["durability_proved"] for v in values))

    def test_mutations_reject(self):
        original = load_trace(FIXTURE)
        def event(rows, case, kind):
            return next(r for r in rows if r["case"] == case and r["kind"] == kind)
        def chunk(rows, case=0):
            return event(rows,case,"snapshot")["context"]["streams"][0]["chunks"][0]
        def remove(rows, case, kind):
            rows.remove(event(rows,case,kind))
            for sequence, record in enumerate(r for r in rows if r["case"] == case):
                record["sequence"] = sequence
        mutations = {
            "missing_case": lambda r: r.__setitem__(slice(None), [x for x in r if x["case"] != 2]),
            "empty": lambda r: r.clear(),
            "schema": lambda r: r[0].update(schema_version=2),
            "ABI": lambda r: r[0]["layout"].update(backend_size=128),
            "float": lambda r: r[0].update(case=0.0),
            "boolean": lambda r: r[0].update(write=False),
            "wrong_alloc": lambda r: event(r,0,"allocation").update(allocator="malloc"),
            "wrapped_alloc": lambda r: event(r,0,"allocation").update(address=2**64),
            "wrong_generation": lambda r: event(r,0,"allocation").update(id=2),
            "lost_temporary": lambda r: remove(r,0,"free"),
            "wrong_return": lambda r: event(r,0,"constructor_result").update(chunk=0),
            "wrong_error": lambda r: event(r,0,"constructor_result").update(error=-1),
            "wrong_parent": lambda r: chunk(r).update(context=6),
            "foreign_backend": lambda r: chunk(r).update(backend=6),
            "memory_backend": lambda r: event(r,0,"snapshot")["context"]["streams"][0].update(memory=1),
            "nonreciprocal_ring": lambda r: event(r,0,"snapshot")["context"]["ring"].update(next=1),
            "missing_up_count": lambda r: event(r,0,"snapshot")["context"].update(up=0),
            "wrong_mapping": lambda r: event(r,0,"os_unmap").update(map=4096),
            "wrong_fd": lambda r: event(r,0,"os_close").update(fd=99),
            "wrong_path": lambda r: event(r,2,"os_delete").update(path=9),
            "wrong_sync_flags": lambda r: event(r,1,"os_sync").update(flags=1),
            "hidden_injection": lambda r: event(r,3,"os_unmap").update(injected=0),
            "forged_success": lambda r: event(r,4,"os_close").update(result=0),
            "missing_sync": lambda r: remove(r,1,"os_sync"),
            "missing_parent": lambda r: remove(r,0,"enter"),
            "wrong_release": lambda r: event(r,0,"free").update(id=1),
            "failed_unmap_reset": lambda r: event(r,3,"backend_state").update(map=0),
            "failed_sync_recorded_synced": lambda r: event(r,5,"backend_state").update(synced=1),
            "erased_up_counter": lambda r: event(r,3,"context_state").update(up=0),
            "erased_mapping": lambda r: event(r,3,"os_terminal").update(map_result=-1,map_errno=12),
            "erased_descriptor": lambda r: event(r,4,"os_terminal").update(fd_result=-1,fd_errno=9),
            "wrong_inode": lambda r: event(r,0,"os_terminal").update(inode=1),
            "unrelated_path": lambda r: chunk(r,2).update(path_inode=1),
            "wrong_terminal_map": lambda r: event(r,3,"os_terminal").update(map=4096),
            "wrong_terminal_fd": lambda r: event(r,4,"os_terminal").update(fd=99),
            "delete_not_observed": lambda r: event(r,2,"os_terminal").update(path_result=0),
            "bad_unlink": lambda r: event(r,0,"chunk_state").update(primary_cleared=0),
            "live_allocation": lambda r: event(r,0,"end").update(live_allocations=1),
            "unreclaimed_fixture": lambda r: event(r,3,"end").update(fixture_cleanup_complete=0),
            "trailing_record": lambda r: r.append(r[-1]),
        }
        for name, mutate in mutations.items():
            with self.subTest(name=name):
                records = copy.deepcopy(original)
                mutate(records)
                with self.assertRaises(ValueError): check_trace(records)


if __name__ == "__main__": unittest.main()
