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

"""Adversarial replay tests against the archived native execution fixture."""
import copy
import json
from pathlib import Path
import sys
import tempfile
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from check_teardown_trace import check_trace, load_trace

FIXTURE = Path(__file__).resolve().parents[2] / "docs/reports/cycle19-native-teardown/trace.jsonl"


class TeardownTraceTests(unittest.TestCase):
    def setUp(self):
        self.records = load_trace(FIXTURE)

    def test_real_native_trace(self):
        result = check_trace(self.records)
        self.assertEqual([r["values"]["allocations"] for r in result], [2, 6, 8, 30, 36])
        self.assertEqual(sum(r["values"]["chunks"] for r in result), 13)
        self.assertTrue(all(r["values"]["static_certificate"] == 0 for r in result))

    def test_mutated_traces_reject(self):
        def event(rows, kind):
            return next(row for row in rows if row["case"] == 4 and row["kind"] == kind)
        def snapshot(rows):
            return event(rows, "snapshot")["context"]
        def stream(rows):
            return snapshot(rows)["streams"][0]
        def chunk(rows):
            return stream(rows)["chunks"][0]
        def replace_free(rows):
            event(rows, "free")["id"] = snapshot(rows)["id"]
        def remove(rows, kind):
            rows.remove(event(rows, kind))
            # Reindex so deletion is rejected on semantics, not sequence gaps.
            case_rows = [row for row in rows if row["case"] == 4]
            for index, row in enumerate(case_rows):
                row["sequence"] = index
        mutations = {
            "missing_case": lambda r: r.clear(),
            "sequence_gap": lambda r: event(r, "free").update(sequence=999),
            "wrong_case": lambda r: event(r, "free").update(case=6),
            "fresh_generation": lambda r: event(r, "allocation").update(id=99),
            "overlapping_storage": lambda r: [row.update(address=1) for row in r if row["case"] == 4 and row["kind"] == "allocation"],
            "zero_size": lambda r: event(r, "allocation").update(size=0),
            "missing_snapshot": lambda r: remove(r, "snapshot"),
            "wrong_context": lambda r: stream(r).update(context=999),
            "wrong_stream": lambda r: chunk(r).update(stream=999),
            "wrong_backend": lambda r: stream(r).update(memory=0),
            "locked": lambda r: chunk(r).update(locked=1),
            "metadata_alias": lambda r: chunk(r)["fields"].__setitem__(2, chunk(r)["fields"][1]),
            "orphan": lambda r: chunk(r)["fields"].__setitem__(2, 0),
            "missing_generation": lambda r: chunk(r).update(backend=999),
            "wrong_node_offset": lambda r: event(r, "begin")["layout"].update(chunk_node=0),
            "wrong_sentinel": lambda r: stream(r)["ring"].update(sentinel=1),
            "wrong_ring_owner": lambda r: stream(r)["ring"]["nodes"][0].update(owner=999),
            "nonreciprocal_ring": lambda r: stream(r)["up_ring"]["nodes"][0].update(prev=1),
            "wrong_constructor_counter": lambda r: snapshot(r).update(total=999),
            "wrong_release_generation": replace_free,
            "missing_release": lambda r: remove(r, "free"),
            "missing_call": lambda r: remove(r, "enter"),
            "wrong_nesting": lambda r: event(r, "enter").update(function="cio_chunk_close"),
            "wrong_counter": lambda r: event(r, "counter").update(total=999),
            "uncleared_node": lambda r: event(r, "chunk_state").update(primary_cleared=0),
            "nonempty_stream": lambda r: event(r, "stream_state").update(up_empty=0),
            "outstanding_allocation": lambda r: event(r, "end").update(live_allocations=1),
            "missing_final_event": lambda r: r.pop(),
        }
        for name, mutate in mutations.items():
            with self.subTest(name=name):
                rows = copy.deepcopy(self.records)
                mutate(rows)
                with self.assertRaises(ValueError):
                    check_trace(rows)

    def test_duplicate_json_keys_reject(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "trace.jsonl"
            path.write_text('{"case":0,"case":1}\n')
            with self.assertRaisesRegex(ValueError, "duplicate JSON key"):
                load_trace(path)

    def test_record_reordering_rejects(self):
        rows = copy.deepcopy(self.records)
        frees = [i for i, row in enumerate(rows) if row["case"] == 4 and row["kind"] == "free"]
        a, b = frees[:2]
        rows[a]["id"], rows[b]["id"] = rows[b]["id"], rows[a]["id"]
        with self.assertRaises(ValueError):
            check_trace(rows)

    def test_duplicate_release_rejects(self):
        rows = copy.deepcopy(self.records)
        frees = [row for row in rows if row["case"] == 4 and row["kind"] == "free"]
        frees[1]["id"] = frees[0]["id"]
        with self.assertRaises(ValueError):
            check_trace(rows)


if __name__ == "__main__":
    unittest.main()
