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

"""`contracts.json` is itemized by the corpus diff tool (card HYGIENE-16).

Finding `DIFF-TOOL-CONTRACTS`: `docs/reports/corpus-repin/diff_artifacts.py`
carried no section for the fifth archived artifact at all, so a deliberately
grown contract inventory was invisible in the diff document. The wave-14
consolidation verified corpus 61 -> 64 by direct comparison past the tool and
recorded the blind spot; wave 15 repeated that by hand for 64 -> 67.

Three properties are pinned here, all on committed bytes:

  1. a synthetic before/after pair whose `contracts.json` differs in every way
     the artifact can differ produces the itemized rows;
  2. the two recorded blind-spot cases — the real archived wave-13 -> wave-14 and
     wave-14 -> wave-15 artifact pairs — now show their contract-inventory growth
     as rows; and
  3. the four pre-existing artifact entries are byte-identical to the archived
     diff documents those consolidations committed, so nothing else moved.

Property 3 is the strongest form available: the archived diff documents are the
pre-change tool's own output, and both reproduce exactly from the two committed
artifact archives, so the comparison is against a real recorded run rather than
against a re-implementation.
"""

import gzip
import importlib.util
import json
import pathlib
import subprocess
import sys
import tempfile
import unittest

ROOT = pathlib.Path(__file__).resolve().parents[2]
TOOL = ROOT / "docs/reports/corpus-repin/diff_artifacts.py"

#: (archived artifacts, measured artifacts, the diff document that run committed)
RECORDED_PAIRS = [
    (
        ROOT / "docs/reports/wave13-corpus/artifacts",
        ROOT / "docs/reports/wave14-corpus/artifacts",
        ROOT / "docs/reports/wave14-corpus/diff-wave13-to-wave14.json.gz",
    ),
    (
        ROOT / "docs/reports/wave14-corpus/artifacts",
        ROOT / "docs/reports/wave15-corpus/artifacts",
        ROOT / "docs/reports/wave15-corpus/diff-wave14-to-wave15.json.gz",
    ),
]

#: The four artifacts the tool diffed before this card. Their entries must not
#: move: this card adds a fifth section and changes no other diff semantics.
PRE_EXISTING = ["chunk-contracts", "diagnostics", "source", "source-effects"]


def load_tool():
    specification = importlib.util.spec_from_file_location("qcore_diff_artifacts", TOOL)
    module = importlib.util.module_from_spec(specification)
    specification.loader.exec_module(module)
    return module


diff_artifacts = load_tool()


def unpack(archive, target):
    """Decompress an archived artifact set under the file names a run produces."""
    manifest = json.loads((archive / "manifest.json").read_text(encoding="utf-8"))
    for stem, entry in manifest.items():
        blob = gzip.decompress((archive / f"{stem}.json.gz").read_bytes())
        (target / entry["produced_file"]).write_bytes(blob)


def run_tool(archived, measured, output):
    completed = subprocess.run(
        [
            sys.executable,
            "-B",
            str(TOOL),
            "--archived",
            str(archived),
            "--measured",
            str(measured),
            "--output",
            str(output),
        ],
        capture_output=True,
        text=True,
    )
    assert completed.returncode == 0, completed.stderr
    return json.loads(output.read_text(encoding="utf-8"))


def read_diff(path):
    return json.loads(gzip.decompress(path.read_bytes()))


class ContractItemizationTest(unittest.TestCase):
    """Property 1: a synthetic pair, every way this artifact can differ."""

    BEFORE = {
        "schema_version": 1,
        "trust": "declared resource-projection assumptions, not verified C behavior",
        "source_profile": {"source_set": "demo", "profile_digest": "aaaa"},
        "audit_selection": json.dumps(
            {"schema_version": 1, "libraries": [{"library": "kept", "label": None}]}
        ),
        "manifest": json.dumps({"schema_version": 1, "modules": ["qcore/a.qcore", "qcore/b.qcore"]}),
        "sources": {
            "/one/compiler/../contract/qcore/a.qcore": "module contract; // A",
            "/one/compiler/../contract/qcore/b.qcore": "module contract; // B",
        },
    }
    AFTER = {
        "schema_version": 1,
        "trust": "declared resource-projection assumptions, not verified C behavior",
        "source_profile": {"source_set": "demo", "profile_digest": "bbbb"},
        "audit_selection": json.dumps(
            {
                "schema_version": 1,
                "libraries": [
                    {"library": "kept", "label": "relabelled"},
                    {"library": "added", "label": None},
                ],
            }
        ),
        # A different checkout root and a different unnormalised walk: the same
        # two modules must still compare as the same two modules.
        "manifest": json.dumps(
            {"schema_version": 1, "modules": ["qcore/a.qcore", "qcore/c.qcore"]}
        ),
        "sources": {
            "/two/compiler/../contract/profiles/demo/../../qcore/a.qcore": "module contract; // A edited",
            "/two/compiler/../contract/qcore/c.qcore": "module contract; // C",
        },
    }

    def section(self, full=False):
        return diff_artifacts.diff_contracts(self.BEFORE, self.AFTER, full)

    def test_canonical_locations_compare_without_rewriting_raw_identity(self):
        old = "/old/contract/qcore/monkey/memory.qv"
        provider = "/new/providers/qcore-provider-libevent/contracts/monkey/memory.qv"
        second = "/another/providers/qcore-provider-libevent/contracts/monkey/memory.qv"
        shared = "/new/providers/quirin-provider-shared/contracts/monkey/memory.qv"
        # The pre-QUIRIN-PROVIDER-SHARED spelling of the shared root must still
        # compare, so a diff across that move names the same logical module.
        former_shared = "/old/providers/shared/contracts/monkey/memory.qv"
        library = "/new/providers/quirin-provider-libc/contracts/monkey/memory.qv"
        for path in [provider, second, shared, former_shared, library]:
            self.assertEqual(diff_artifacts.contract_module(old), diff_artifacts.contract_module(path))
        result = diff_artifacts.diff_contracts({"sources": {old: "policy"}}, {"sources": {provider: "changed"}}, False)
        row = result["sources"]["changed"][0]
        self.assertEqual(row["raw_key"], {"before": old, "after": provider})
        collision = diff_artifacts.diff_contracts(
            {"sources": {old: "policy"}}, {"sources": {provider: "policy", shared: "policy"}}, False)
        self.assertTrue(collision["sources"]["key_collisions"])

    def test_a_module_add_a_removal_and_a_content_change_are_rows(self):
        sources = self.section()["sources"]
        self.assertEqual([row["key"] for row in sources["added"]], ["qcore/c.qcore"])
        self.assertEqual([row["key"] for row in sources["removed"]], ["qcore/b.qcore"])
        self.assertEqual([row["key"] for row in sources["changed"]], ["qcore/a.qcore"])
        self.assertEqual(
            sources["counts"],
            {"added": 1, "removed": 1, "changed": 1, "before": 2, "after": 2},
        )

    def test_a_changed_module_publishes_both_digests_and_both_raw_paths(self):
        changed = self.section()["sources"]["changed"][0]
        self.assertNotEqual(changed["before"]["sha256"], changed["after"]["sha256"])
        self.assertEqual(changed["before"]["bytes"], len(b"module contract; // A"))
        self.assertEqual(changed["after"]["bytes"], len(b"module contract; // A edited"))
        # The raw keys differ by checkout root and by the walk the loader spells;
        # publishing both is what keeps the normalisation auditable.
        self.assertEqual(
            changed["raw_key"],
            {
                "before": "/one/compiler/../contract/qcore/a.qcore",
                "after": "/two/compiler/../contract/profiles/demo/../../qcore/a.qcore",
            },
        )

    def test_the_module_text_travels_only_under_full(self):
        self.assertNotIn("after_value", self.section()["sources"]["changed"][0])
        self.assertEqual(
            self.section(full=True)["sources"]["changed"][0]["after_value"]["text"],
            "module contract; // A edited",
        )

    def test_the_embedded_manifest_is_itemized_by_module(self):
        modules = self.section()["manifest"]["modules"]
        self.assertTrue(self.section()["manifest"]["parsed"])
        self.assertEqual([row["key"] for row in modules["added"]], ["qcore/c.qcore"])
        self.assertEqual([row["key"] for row in modules["removed"]], ["qcore/b.qcore"])

    def test_the_embedded_audit_selection_is_itemized_by_library(self):
        libraries = self.section()["audit_selection"]["libraries"]
        self.assertEqual([row["key"] for row in libraries["added"]], ["added"])
        self.assertEqual([row["key"] for row in libraries["changed"]], ["kept"])

    def test_every_remaining_top_level_key_is_compared_field_by_field(self):
        keys = self.section()["keys"]
        # `source_profile` moved; `schema_version` and `trust` did not.
        self.assertEqual(sorted(keys), ["source_profile"])
        self.assertIn("bbbb", keys["source_profile"]["after"])

    def test_an_unrecognised_top_level_key_is_visible_rather_than_absent(self):
        after = dict(self.AFTER, invented_key={"something": 1})
        self.assertIn("invented_key", diff_artifacts.diff_contracts(self.BEFORE, after, False)["keys"])

    def test_an_unparsable_embedded_document_is_reported_not_dropped(self):
        after = dict(self.AFTER, manifest="{ not json")
        section = diff_artifacts.diff_contracts(self.BEFORE, after, False)["manifest"]
        self.assertFalse(section["parsed"])
        self.assertTrue(section["changed"])

    def test_a_normalisation_collision_falls_back_to_raw_keys_and_says_so(self):
        """Two distinct keys must never be silently merged into one row."""
        before = dict(
            self.BEFORE,
            sources={
                "/one/contract/qcore/a.qcore": "first",
                "/one/build/contract/qcore/a.qcore": "second",
            },
        )
        section = diff_artifacts.diff_contracts(before, self.AFTER, False)
        self.assertEqual(section["sources"]["keying"], "raw")
        self.assertEqual(len(section["sources"]["key_collisions"]), 1)
        self.assertEqual(section["sources"]["counts"]["before"], 2)


class RecordedBlindSpotTest(unittest.TestCase):
    """Properties 2 and 3, on the two real consolidations that hit the finding."""

    def test_the_recorded_contract_growth_is_now_itemized(self):
        expected = {
            "wave13-corpus": {
                "added": [
                    "qcore/fluent_bit/input_chunk_name.qcore",
                    "qcore/fluent_bit/logging.qcore",
                    "qcore/fluent_bit/routes_mask.qcore",
                ],
                # HASH-CLAUSE retired the comment carrier in this module; the
                # wave-14 report could only describe it as a byte-size change.
                "changed": ["qcore/fluent_bit/hash_table_publication.qcore"],
            },
            "wave14-corpus": {
                "added": [
                    "qcore/chunkio/chunk_residency.qcore",
                    "qcore/fluent_bit/input_chunk_helpers.qcore",
                    "qcore/fluent_bit/time.qcore",
                ],
                "changed": [],
            },
        }
        for archived, measured, _ in RECORDED_PAIRS:
            with self.subTest(archived.parent.name):
                with tempfile.TemporaryDirectory(prefix="qcore-diff-") as scratch:
                    directory = pathlib.Path(scratch)
                    unpack(measured, directory)
                    produced = run_tool(archived, directory, directory / "diff.json")
                sources = produced["artifacts"]["contracts"]["sources"]
                self.assertEqual(sources["keying"], "module path below contract/")
                self.assertEqual(
                    [row["key"] for row in sources["added"]],
                    expected[archived.parent.name]["added"],
                )
                self.assertEqual([row["key"] for row in sources["removed"]], [])
                self.assertEqual(
                    [row["key"] for row in sources["changed"]],
                    expected[archived.parent.name]["changed"],
                )
                # The embedded manifest grows with the inventory, and the whole
                # section is reachable from the document a consolidation reads.
                self.assertEqual(
                    [row["key"] for row in produced["artifacts"]["contracts"]["manifest"]["modules"]["added"]],
                    expected[archived.parent.name]["added"],
                )

    def test_the_four_pre_existing_artifact_entries_are_byte_stable(self):
        for archived, measured, recorded in RECORDED_PAIRS:
            with self.subTest(recorded.name):
                committed = read_diff(recorded)
                self.assertEqual(committed["schema_version"], 1)
                # Card GEN-CTREASONS backfilled the fifth section into these
                # archived documents (follow-on `DIFF-TOOL-CONTRACTS-BACKFILL`),
                # so the pin moves deliberately: the document is still a
                # schema-1 document, it still carries the four pre-existing
                # entries, and the `contracts` entry it now also carries is
                # marked as backfilled rather than passing as original. The
                # load-bearing half below — the four entries reproducing
                # byte-identically — is unchanged and is what this test is for.
                self.assertEqual(
                    sorted(committed["artifacts"]), sorted(PRE_EXISTING + ["contracts"])
                )
                self.assertIn(
                    "artifacts.contracts",
                    committed.get("backfilled_sections", {}),
                    "a contracts section in a schema-1 document must say it was backfilled",
                )
                with tempfile.TemporaryDirectory(prefix="qcore-diff-") as scratch:
                    directory = pathlib.Path(scratch)
                    unpack(measured, directory)
                    produced = run_tool(archived, directory, directory / "diff.json")
                self.assertEqual(produced["schema_version"], 2)
                for stem in PRE_EXISTING:
                    self.assertEqual(
                        json.dumps(produced["artifacts"][stem], sort_keys=True),
                        json.dumps(committed["artifacts"][stem], sort_keys=True),
                        f"{stem} moved against the archived diff document",
                    )
                self.assertEqual(
                    sorted(produced["artifacts"]), sorted(PRE_EXISTING + ["contracts"])
                )


if __name__ == "__main__":
    unittest.main()
