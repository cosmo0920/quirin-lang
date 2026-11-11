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

"""Card W45-CONTRACT-DEFERRED-REVIEW: the corpus identity tool's
`--additive-field` disclosure.

The card adds one top-level key, `pending_human_review`, to the arbiter's
contract sidecar. `baseline_identity.py` keeps its raw byte comparison and its
exit rule (the verdict-bearing diagnostics only); the new option reports,
beside that, whether a sidecar differs from its baseline in exactly the named
key. These tests pin that it ignores exactly that key and nothing else.
"""

import gzip
import hashlib
import json
import pathlib
import runpy
import subprocess
import sys
import tempfile
import unittest

TOOL = (
    pathlib.Path(__file__).resolve().parents[2]
    / "docs/reports/w43-return-expression/scripts/baseline_identity.py"
)
MODULE = runpy.run_path(str(TOOL), run_name="baseline_identity_test")
NAMES = MODULE["NAMES"]


def pretty(value):
    return json.dumps(value, indent=2, ensure_ascii=False).encode()


BASELINE_CONTRACTS = {"manifest": "m", "schema_version": 1, "sources": {"k.qcore": "text"}}
WITH_SECTION = {
    "manifest": "m",
    "pending_human_review": {"declarations": 0, "entries": []},
    "schema_version": 1,
    "sources": {"k.qcore": "text"},
}


class WithoutAdditiveFieldsTests(unittest.TestCase):
    def check(self, run, keys=("pending_human_review",)):
        return MODULE["without_additive_fields"](pretty(run), pretty(BASELINE_CONTRACTS), list(keys))

    def test_exactly_the_named_key_is_ignored(self):
        self.assertEqual(
            self.check(WITH_SECTION),
            {"keys_present": True, "json_equal": True, "bytes_equal": True},
        )

    def test_any_other_difference_still_differs(self):
        changed = dict(WITH_SECTION, sources={"k.qcore": "other"})
        result = self.check(changed)
        self.assertFalse(result["json_equal"])
        self.assertFalse(result["bytes_equal"])

    def test_a_different_key_is_not_ignored(self):
        other = dict(BASELINE_CONTRACTS, something_else=1)
        result = self.check(other)
        self.assertFalse(result["keys_present"])
        self.assertFalse(result["json_equal"])

    def test_a_key_the_baseline_already_has_is_not_additive(self):
        result = MODULE["without_additive_fields"](
            pretty(WITH_SECTION), pretty(WITH_SECTION), ["pending_human_review"]
        )
        self.assertFalse(result["keys_present"])
        self.assertFalse(result["json_equal"])


class CommandLineTests(unittest.TestCase):
    def archive(self, directory, artifacts):
        manifest = {}
        for label, name in NAMES.items():
            data = artifacts[label]
            (directory / f"{label}.json.gz").write_bytes(gzip.compress(data))
            manifest[label] = {
                "file": f"{label}.json.gz",
                "raw_sha256": hashlib.sha256(data).hexdigest(),
            }
        (directory / "manifest.json").write_text(json.dumps(manifest))

    def run_tool(self, *extra):
        with tempfile.TemporaryDirectory() as temporary:
            root = pathlib.Path(temporary)
            baseline, run = root / "baseline", root / "run"
            baseline.mkdir()
            run.mkdir()
            artifacts = {label: pretty({"label": label}) for label in NAMES}
            artifacts["contracts"] = pretty(BASELINE_CONTRACTS)
            self.archive(baseline, artifacts)
            for label, name in NAMES.items():
                (run / name).write_bytes(artifacts[label])
            (run / NAMES["contracts"]).write_bytes(pretty(WITH_SECTION))
            completed = subprocess.run(
                [sys.executable, str(TOOL), str(run), str(baseline), *extra],
                capture_output=True,
                text=True,
            )
            return completed

    def test_the_disclosure_is_reported_and_the_exit_rule_is_unchanged(self):
        completed = self.run_tool("--additive-field", "contracts:pending_human_review")
        self.assertEqual(completed.returncode, 0, completed.stderr)
        result = json.loads(completed.stdout)
        entry = result["artifacts"]["contracts"]
        self.assertFalse(entry["byte_identical"])
        self.assertEqual(entry["additive_fields"], ["pending_human_review"])
        self.assertTrue(entry["identical_ignoring_additive_fields"]["bytes_equal"])
        self.assertTrue(result["verdict_bearing_identical"])

    def test_without_the_option_nothing_is_ignored(self):
        completed = self.run_tool()
        entry = json.loads(completed.stdout)["artifacts"]["contracts"]
        self.assertFalse(entry["byte_identical"])
        self.assertNotIn("identical_ignoring_additive_fields", entry)

    def test_the_verdict_bearing_artifact_cannot_be_named(self):
        completed = self.run_tool("--additive-field", "diagnostics:units")
        self.assertNotEqual(completed.returncode, 0)
        self.assertIn("names a sidecar", completed.stderr)


if __name__ == "__main__":
    unittest.main()
