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

"""The archived corpus diff documents' backfilled `contracts` section.

Card `GEN-CTREASONS`, follow-on `DIFF-TOOL-CONTRACTS-BACKFILL`. Card HYGIENE-16
taught `docs/reports/corpus-repin/diff_artifacts.py` to itemize `contracts.json`
— until then it never opened the file — so every consolidation diff document
written before that landed described four artifacts and its wave's
contract-inventory movement lived only in prose.

`docs/reports/wave17-gen-ctreasons/backfill_contracts.py` regenerates only that
section, from each document's **own committed artifact pair**, and splices it in
behind a `backfilled_sections` record. The properties pinned here are the ones
that make it a backfill rather than a rewrite of somebody else's evidence:

  1. **Reproducible from committed bytes.** The tool's `--check` mode rebuilds
     every section and requires the committed document to be exactly what it
     produces, so a reader can regenerate the whole set and get the same bytes.
  2. **Append-only.** Removing the two paths the tool adds gives back the
     original document byte for byte — checked against the digest the record
     itself carries, and, where the repository still has it, against the blob at
     the commit that record names.
  3. **The two recorded blind-spot cases reproduce.** The wave-14 consolidation
     verified a grown contract inventory by hand past the tool (three modules
     added, one changed) and wave 15 repeated that for three more; both are
     asserted here as the values the backfilled sections publish.
  4. **The oracle refuses.** A document whose pre-existing entries do not
     reproduce from the named inputs is not backfilled at all.

Nothing here is a corpus measurement: no audit is re-run and no verdict, count
or pin is read.
"""

import gzip
import importlib.util
import json
import pathlib
import shutil
import subprocess
import unittest

ROOT = pathlib.Path(__file__).resolve().parents[2]
TOOL = ROOT / "docs/reports/wave17-gen-ctreasons/backfill_contracts.py"


def load_tool():
    specification = importlib.util.spec_from_file_location("qcore_backfill_contracts", TOOL)
    module = importlib.util.module_from_spec(specification)
    specification.loader.exec_module(module)
    return module


backfill = load_tool()

#: The two cases the wave-14 and wave-16 consolidation READMEs verified by hand,
#: past the tool, and recorded as the blind spot. These are the findings the
#: backfilled sections have to reproduce.
RECORDED_FINDINGS = {
    "docs/reports/wave14-corpus/diff-wave13-to-wave14.json.gz": {
        "added": [
            "qcore/fluent_bit/input_chunk_name.qcore",
            "qcore/fluent_bit/logging.qcore",
            "qcore/fluent_bit/routes_mask.qcore",
        ],
        "changed": ["qcore/fluent_bit/hash_table_publication.qcore"],
    },
    "docs/reports/wave15-corpus/diff-wave14-to-wave15.json.gz": {
        "added": [
            "qcore/chunkio/chunk_residency.qcore",
            "qcore/fluent_bit/input_chunk_helpers.qcore",
            "qcore/fluent_bit/time.qcore",
        ],
        "changed": [],
    },
}


class BackfilledContractSections(unittest.TestCase):
    def test_every_archived_consolidation_diff_now_itemizes_contracts(self):
        for relative in backfill.DOCUMENTS:
            with self.subTest(relative):
                document = backfill.read_document(ROOT / relative)
                self.assertIn("contracts", document["artifacts"])
                self.assertEqual(
                    sorted(document["artifacts"]),
                    sorted(backfill.PRE_EXISTING + ["contracts"]),
                )

    def test_the_committed_documents_are_what_the_tool_reproduces(self):
        """Property 1, run over the real committed archives; writes nothing."""
        self.assertEqual(backfill.main(["backfill_contracts.py", "--check"]), 0)

    def test_the_backfill_is_append_only_against_its_own_recorded_digest(self):
        """Property 2, self-contained: strip the added paths, compare digests."""
        checked = 0
        for relative in backfill.DOCUMENTS:
            document = backfill.read_document(ROOT / relative)
            record = document.get(backfill.PROVENANCE_KEY, {}).get(backfill.SECTION_PATH)
            if record is None:
                continue
            checked += 1
            with self.subTest(relative):
                stripped = backfill.without_backfill(document)
                self.assertEqual(
                    backfill.digest(backfill.serialize(stripped)), record["original_sha256"]
                )
                # And the only new paths are the two the record names.
                self.assertEqual(
                    set(document) - set(stripped), {backfill.PROVENANCE_KEY}
                )
                self.assertEqual(
                    set(document["artifacts"]) - set(stripped["artifacts"]), {"contracts"}
                )
                self.assertEqual(list(record), sorted(record))
                self.assertEqual(record["card"], backfill.CARD)
                self.assertEqual(record["tool_section_schema"], 2)
                self.assertEqual(
                    record["document_schema_version_unchanged"], document["schema_version"]
                )
        self.assertTrue(checked, "no document carries a backfill record")

    @unittest.skipUnless(shutil.which("git"), "git is required to read the original blob")
    def test_the_backfill_is_append_only_against_the_commit_it_names(self):
        """Property 2, independently: the pre-backfill bytes, from git."""
        checked = 0
        for relative in backfill.DOCUMENTS:
            document = backfill.read_document(ROOT / relative)
            record = document.get(backfill.PROVENANCE_KEY, {}).get(backfill.SECTION_PATH)
            if record is None:
                continue
            finished = subprocess.run(
                ["git", "-C", str(ROOT), "show", "%s:%s" % (record["original_commit"], relative)],
                capture_output=True,
            )
            if finished.returncode != 0:
                self.skipTest("the commit %s is not in this repository" % record["original_commit"])
            checked += 1
            with self.subTest(relative):
                original = gzip.decompress(finished.stdout).decode("utf-8")
                self.assertEqual(
                    backfill.serialize(backfill.without_backfill(document)),
                    original,
                    "the backfill changed a section the original document already carried",
                )
        self.assertTrue(checked)

    def test_the_recorded_blind_spot_findings_reproduce(self):
        """Property 3: what the two consolidations had to verify by hand."""
        for relative, expected in RECORDED_FINDINGS.items():
            with self.subTest(relative):
                section = backfill.read_document(ROOT / relative)["artifacts"]["contracts"]
                self.assertEqual(
                    [row["key"] for row in section["sources"]["added"]], expected["added"]
                )
                self.assertEqual(
                    [row["key"] for row in section["sources"]["changed"]], expected["changed"]
                )
                self.assertEqual(section["sources"]["counts"]["removed"], 0)
                # The embedded manifest grows with the inventory, and the
                # normalisation the keys rest on is published rather than assumed.
                self.assertEqual(
                    section["sources"]["keying"], "module path below contract/"
                )
                self.assertEqual(
                    section["manifest"]["modules"]["counts"]["added"], len(expected["added"])
                )

    def test_a_document_written_by_the_current_tool_is_not_marked_backfilled(self):
        """Non-vacuity for the other branch: wave 16 needed nothing."""
        native = [
            relative
            for relative in backfill.DOCUMENTS
            if backfill.PROVENANCE_KEY not in backfill.read_document(ROOT / relative)
        ]
        self.assertTrue(native, "every document is backfilled; the skip branch is untested")
        for relative in native:
            document = backfill.read_document(ROOT / relative)
            self.assertEqual(document["schema_version"], 2)
            self.assertIn("contracts", document["artifacts"])

    def test_a_document_whose_recorded_entries_do_not_reproduce_is_refused(self):
        """Property 4: the oracle, exercised rather than described.

        A pre-existing entry that does not come back out of the named artifact
        pair means the pair is not that run's input, so nothing is spliced in.
        """
        relative = "docs/reports/wave14-corpus/diff-wave13-to-wave14.json.gz"
        path = ROOT / relative
        original = backfill.without_backfill(backfill.read_document(path))
        tampered = json.loads(json.dumps(original))
        tampered["artifacts"]["diagnostics"]["counts"] = {"invented": 1}
        with self.assertRaises(SystemExit) as raised:
            backfill.backfill(path, tampered)
        self.assertIn("does not reproduce", str(raised.exception))
        # And the committed document is untouched by the attempt.
        self.assertEqual(backfill.main(["backfill_contracts.py", "--check"]), 0)


if __name__ == "__main__":
    unittest.main()
