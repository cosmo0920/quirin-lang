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

"""Replay and adversarial checks of the pinned native allocation-failure matrix."""
import copy
from pathlib import Path
import sys
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from check_teardown_trace import load_trace
from check_constructor_failure_trace import check_trace

FIXTURE = Path(__file__).resolve().parents[2] / "docs/reports/cycle19-constructor-failures/trace.jsonl"


class ConstructorFailureTraceTests(unittest.TestCase):
    def setUp(self):
        self.records = load_trace(FIXTURE)

    def test_real_failure_matrix(self):
        values = [row["values"] for row in check_trace(self.records)]
        self.assertEqual(len(values), 10)
        self.assertEqual(sum(row["injected_failures"] for row in values), 9)
        self.assertEqual(sum(row["allocations"] for row in values), 55)
        self.assertEqual(sum(row["rollback_releases"] for row in values), 6)
        self.assertEqual(values[2]["nullable_chmod"], 1)
        self.assertEqual(values[6]["nullable_chunk_name"], 1)
        self.assertTrue(all(values[case]["chunk_null"] and values[case]["error_zero"] for case in (7, 8)))

    def test_mutations_reject(self):
        def event(rows, case, kind):
            return next(row for row in rows if row["case"] == case and row["kind"] == kind)
        def remove(rows, case, kind):
            rows.remove(event(rows, case, kind))
            for index, row in enumerate(r for r in rows if r["case"] == case):
                row["sequence"] = index
        mutations = {
            "empty": lambda r: r.clear(),
            "missing_case": lambda r: r.__setitem__(slice(None), [row for row in r if row["case"] != 3]),
            "wrong_fail_target": lambda r: event(r, 6, "begin_failure").update(fail_at=7),
            "missing_attempt": lambda r: remove(r, 2, "attempt"),
            "unexpected_failure": lambda r: event(r, 0, "attempt").update(failed=1),
            "failure_not_injected": lambda r: event(r, 1, "attempt").update(failed=0),
            "wrong_size": lambda r: event(r, 1, "attempt").update(size=0),
            "wrong_allocator": lambda r: event(r, 1, "attempt").update(allocator="malloc"),
            "stale_generation": lambda r: event(r, 2, "allocation").update(id=100),
            "wrong_rollback": lambda r: event(r, 8, "free").update(id=1),
            "missing_rollback": lambda r: remove(r, 8, "free"),
            "changed_error": lambda r: event(r, 7, "constructor_result").update(error=-1),
            "invented_success": lambda r: event(r, 7, "constructor_result").update(chunk=5),
            "wrong_meta_result": lambda r: event(r, 9, "constructor_result").update(metadata_result=0),
            "name_alias": lambda r: event(r, 6, "snapshot")["context"]["streams"][0]["chunks"][0].update(name=5),
            "wrong_optional_field": lambda r: event(r, 2, "snapshot")["context"]["fields"].__setitem__(2, 1),
            "unrolled_counter": lambda r: event(r, 8, "snapshot")["context"].update(total=1),
            "orphan_after_rollback": lambda r: event(r, 8, "end").update(live_allocations=1),
            "null_context_leak": lambda r: event(r, 1, "end").update(live_allocations=1),
            "missing_teardown": lambda r: remove(r, 4, "enter"),
        }
        for name, mutate in mutations.items():
            with self.subTest(name=name):
                rows = copy.deepcopy(self.records)
                mutate(rows)
                with self.assertRaises(ValueError):
                    check_trace(rows)


if __name__ == "__main__":
    unittest.main()
