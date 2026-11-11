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

"""Transport checks for `W43-CHUNK-SOURCE-PROFILE`.

Nothing here decides a lifecycle verdict; Rust does, in
`compiler/tests/chunk_lifecycle_coverage.rs`. What is pinned here:

  1. the committed row delta is exactly the rendering of the W42 and W43
     published results, and every unchanged row keeps its W42 reason verbatim;
  2. the committed refusal-shape measurement accounts for the same 96 selected
     functions as the inventory and its summary agrees with its own rows;
  3. every vendored copy of a pinned unit matches its manifest digest, and the
     manifest's `cio_chunk.c` is the digest the fluent-bit profile pins.
"""

import hashlib
import json
import pathlib
import subprocess
import sys
import unittest
from collections import Counter

ROOT = pathlib.Path(__file__).resolve().parents[2]
REPORT = ROOT / "docs/reports/w43-chunk-source-profile"
W42 = ROOT / "docs/reports/w42-chunk-lifecycle-cert"
VENDORED = ROOT / "compiler/tests/fixtures/pinned/fluent-bit"
PROFILE_REASON = "outside single borrowed chunk / int-or-void-result source profile"


def key(row):
    return (row["library"], row["file"], row["function"])


class W43ChunkSourceProfile(unittest.TestCase):
    def test_committed_row_delta_is_the_rendering_of_both_results(self):
        result = subprocess.run(
            [sys.executable, str(REPORT / "scripts/row_delta.py"), "--check",
             str(REPORT / "row-delta.json")],
            cwd=ROOT, capture_output=True, text=True, check=False)
        self.assertEqual(result.returncode, 0, result.stderr)

    def test_unchanged_rows_keep_their_w42_reason_verbatim(self):
        before = {key(r): r for r in json.loads((W42 / "lifecycle-coverage.json").read_text())["functions"]}
        after = {key(r): r for r in json.loads((REPORT / "lifecycle-coverage.json").read_text())["functions"]}
        delta = json.loads((REPORT / "row-delta.json").read_text())
        changed = {(r["library"], r["file"], r["function"]) for r in delta["changed_rows"]}
        self.assertEqual(set(before), set(after))
        self.assertEqual(len(before), 96)
        for identity in set(before) - changed:
            self.assertEqual(before[identity]["lifecycle"], after[identity]["lifecycle"], identity)
            self.assertEqual(before[identity]["role"], after[identity]["role"], identity)
        # No row is promoted, and each changed row names its own pinned unit.
        for identity in changed:
            row = after[identity]
            self.assertEqual(row["lifecycle"]["status"], "unknown")
            self.assertEqual(row["derivation"]["source"]["function"], identity[2])
            self.assertTrue(row["derivation"]["uncovered_paths"])

    def test_refusal_shapes_account_for_the_selection(self):
        shapes = json.loads((REPORT / "refusal-shapes.json").read_text())
        inventory = json.loads((W42 / "inventory.json").read_text())
        self.assertEqual(shapes["inventory_artifact_sha256"], inventory["artifact_sha256"])
        self.assertEqual({(r["file"], r["function"]) for r in shapes["functions"]},
                         {(r["file"], r["function"]) for r in inventory["functions"]})
        refused = [r for r in shapes["functions"] if r["pinned_projection"] == PROFILE_REASON]
        summary = shapes["summary"]
        self.assertEqual(summary["profile_refusals"], len(refused))
        self.assertEqual(len(refused), 74)
        self.assertEqual(sum(summary["by_result_class"].values()), 74)
        self.assertEqual(
            dict(Counter(r["result_class"] for r in refused)), summary["by_result_class"])
        # No refused function has an ABI the Boolean field models admit.
        self.assertEqual(summary["admitted_boolean_abi"], 0)
        get = next(r for r in shapes["functions"] if r["function"] == "input_chunk_get")
        self.assertEqual(get["result_class"], "pointer")
        self.assertIn("flb_input_chunk_destroy", get["direct_callees"])

    def test_vendored_units_match_their_manifest_and_the_profile_pin(self):
        manifest = json.loads((VENDORED / "manifest.json").read_text())
        listed = set(manifest["files"])
        present = {p.relative_to(VENDORED).as_posix() for p in VENDORED.rglob("*") if p.is_file()}
        self.assertEqual(present - {"manifest.json"}, listed)
        for path, record in manifest["files"].items():
            checkout_bytes = (VENDORED / path).read_bytes()
            digest = hashlib.sha256(checkout_bytes.replace(b"\r\n", b"\n")).hexdigest()
            self.assertEqual(digest, record["sha256"], path)
        profile = json.loads((ROOT / "contract/profiles/fluent-bit/profile.json").read_text())
        pinned = {f["path"]: f["sha256"]
                  for f in profile["libraries"]["chunkio"]["audited_files"]}
        self.assertEqual(manifest["files"]["source/lib/chunkio/src/cio_chunk.c"]["sha256"],
                         pinned["lib/chunkio/src/cio_chunk.c"])
        self.assertEqual(manifest["source_revision"], profile["pins"]["fluent-bit"]["git_commit"])


if __name__ == "__main__":
    unittest.main()
