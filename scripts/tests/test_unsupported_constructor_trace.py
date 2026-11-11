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

"""Native rejected-chunk outcomes and adversarial trace regressions."""
import copy
from pathlib import Path
import sys
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from check_teardown_trace import load_trace, check_tree_and_teardown
from check_unsupported_constructor_trace import check_trace

FIXTURE = Path(__file__).resolve().parents[2] / "docs/reports/cycle19-unsupported-constructors/trace.jsonl"


class UnsupportedConstructorTests(unittest.TestCase):
    def test_native_matrix(self):
        values = [row["values"] for row in check_trace(load_trace(FIXTURE))]
        self.assertEqual(len(values), 4)
        self.assertEqual(sum(v["allocations"] for v in values), 24)
        self.assertEqual(sum(v["rollback_releases"] for v in values), 8)
        self.assertTrue(all(v["chunk_null"] and v["error_sentinel_unchanged"] for v in values))

    def test_mutations_reject(self):
        original = load_trace(FIXTURE)
        def event(rows, kind):
            return next(r for r in rows if r["case"] == 0 and r["kind"] == kind)
        def remove(rows, kind):
            rows.remove(event(rows, kind))
            for i, row in enumerate(r for r in rows if r["case"] == 0): row["sequence"] = i
        mutations = {
            "empty": lambda r: r.clear(),
            "missing_case": lambda r: r.__setitem__(slice(None), [x for x in r if x["case"] != 2]),
            "known_kind": lambda r: event(r,"begin_unsupported").update(backend_kind=1),
            "wrong_build_kind": lambda r: event(r,"begin_unsupported").update(filesystem_kind=9),
            "changed_returned_kind": lambda r: event(r,"constructor_result").update(stream_kind=0),
            "chunk_success": lambda r: event(r,"constructor_result").update(chunk=5),
            "changed_error": lambda r: event(r,"constructor_result").update(error=0),
            "wrong_parent": lambda r: event(r,"constructor_result").update(context=3),
            "wrong_rollback": lambda r: event(r,"free").update(id=5),
            "missing_rollback": lambda r: remove(r,"free"),
            "stale_allocation": lambda r: event(r,"allocation").update(id=99),
            "wrapped_address": lambda r: event(r,"allocation").update(address=2**64),
            "float_generation": lambda r: event(r,"allocation").update(id=1.0),
            "boolean_counter": lambda r: event(r,"snapshot")["context"].update(total=False),
            "wrong_allocator": lambda r: event(r,"allocation").update(allocator="malloc"),
            "wrong_size": lambda r: event(r,"allocation").update(size=1),
            "wrong_memory_flag": lambda r: event(r,"snapshot")["context"]["streams"][0].update(memory=1),
            "residual_counter": lambda r: event(r,"snapshot")["context"].update(total=1),
            "bad_ring": lambda r: event(r,"snapshot")["context"]["streams"][0]["ring"].update(next=0),
            "live_terminal": lambda r: event(r,"end").update(live_allocations=1),
            "missing_teardown": lambda r: remove(r,"enter"),
        }
        for name, mutate in mutations.items():
            with self.subTest(name=name):
                rows = copy.deepcopy(original);mutate(rows)
                with self.assertRaises(ValueError): check_trace(rows)

    def test_nonmemory_tree_cannot_interpret_a_chunk(self):
        with self.assertRaisesRegex(ValueError, "no chunk/backend interpretation"):
            check_tree_and_teardown([],0,{},0,(1,1,0),{},stream_memory=0)


if __name__ == "__main__": unittest.main()
