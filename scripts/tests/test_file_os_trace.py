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

"""Real OS endpoints and adversarial cleanup observation regressions."""
import copy
from pathlib import Path
import sys
import tempfile
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from check_file_os_trace import check_trace, load_trace

FIXTURE = Path(__file__).resolve().parents[2] / "docs/reports/cycle19-file-os/trace.jsonl"


class FileOsTests(unittest.TestCase):
    def test_native_matrix(self):
        values = [r["values"] for r in check_trace(load_trace(FIXTURE))]
        self.assertEqual(len(values), 12)
        for name in ("observed_close_success", "observed_unmap_success", "observed_delete_success", "observed_unmap_failure"):
            self.assertEqual(sum(v[name] for v in values), 1)
        self.assertTrue(all(v["static_certificate"] == v["durability_proved"] == 0 for v in values))

    def test_mutations_reject(self):
        original = load_trace(FIXTURE)
        mutations = {
            "empty": lambda r: r.clear(),
            "missing": lambda r: r.pop(),
            "duplicate": lambda r: r.append(r[0]),
            "reordered": lambda r: r.reverse(),
            "schema": lambda r: r[0].update(schema_version=2),
            "float": lambda r: r[0].update(case=0.0),
            "boolean": lambda r: r[0].update(was_null=True),
            "extra": lambda r: r[0].update(unchecked=True),
            "wrapped_pointer": lambda r: r[0]["fixture"].update(map=2**64),
            "wrong_fd": lambda r: r[2]["call"].update(fd=99),
            "wrong_map": lambda r: r[6]["call"].update(map=4096),
            "wrong_length": lambda r: r[6]["call"].update(length=0),
            "wrong_path": lambda r: r[11]["call"].update(path=1),
            "missing_call": lambda r: r[2]["call"].update(count=0),
            "injected_call": lambda r: r[8]["call"].update(count=1),
            "ignored_failure": lambda r: r[7].update(returned=0),
            "wrong_errno": lambda r: r[3]["call"].update(errno=5),
            "missing_diagnostic": lambda r: r[10].update(diagnostics=0),
            "wrong_diagnostic": lambda r: r[7].update(reported_errno=5),
            "failed_unmap_reset": lambda r: r[7]["after"].update(map=0),
            "close_not_reset": lambda r: r[2]["after"].update(fd=r[2]["fixture"]["fd"]),
            "stale_closed_probe": lambda r: r[2]["os_after"].update(descriptor=0, descriptor_errno=0),
            "still_mapped": lambda r: r[6]["os_after"].update(mapping=0, mapping_errno=0),
            "failed_unmap_lost": lambda r: r[7]["os_after"].update(mapping=-1, mapping_errno=12),
            "replaced_witness": lambda r: r[11]["os_after"].update(witness_inode=1),
            "unrelated_target": lambda r: r[11]["os_before"].update(inode=1),
            "unlink_did_not_remove_link": lambda r: r[11]["os_after"].update(witness_links=2),
            "wrong_absence_errno": lambda r: r[11]["os_after"].update(path_errno=13),
            "frame": lambda r: r[0].update(other_fields_preserved=0),
            "path_bytes": lambda r: r[11].update(path_bytes_preserved=0),
            "cleanup": lambda r: r[7].update(fixture_cleanup_complete=0),
        }
        for name, mutate in mutations.items():
            with self.subTest(name=name):
                records = copy.deepcopy(original)
                mutate(records)
                with self.assertRaises(ValueError):
                    check_trace(records)

    def test_duplicate_json_keys_reject(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "trace.jsonl"
            path.write_text('{"case":0,"case":1}\n')
            with self.assertRaisesRegex(ValueError, "duplicate JSON key"):
                load_trace(path)


if __name__ == "__main__":
    unittest.main()
