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

"""Card STATUS-REGROWTH-BY-PREPEND: `scripts/unpublished_landings.py`
generates the unpublished-landings sentence of `docs/STATUS.md` from the
landing gate records and the publication records, in a temporary git
repository, and the committed sentence equals the generated one."""
import json
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts"))
import unpublished_landings as ul  # noqa: E402

STATUS = ("# Current status\n\n### Unpublished landings\n\n"
          "<!-- unpublished-landings: hand-written once. -->\nOld sentence.\n"
          "<!-- unpublished-landings:end -->\n\n## Next\n\nKept.\n")


def git(root, *args):
    return subprocess.run(["git", "-C", str(root), "-c", "user.name=Test",
                           "-c", "user.email=test@example.invalid", *args],
                          check=True, capture_output=True, text=True).stdout.strip()


class Repository:
    """A temporary repository with landing gate records and profiles."""

    def __init__(self, directory):
        self.root = Path(directory)
        git(self.root, "init", "-q")
        (self.root / "docs").mkdir()
        (self.root / ul.STATUS).write_text(STATUS)
        self.commit("initial")

    def commit(self, message):
        git(self.root, "add", "-A")
        git(self.root, "commit", "-qm", message, "--allow-empty")
        return git(self.root, "rev-parse", "HEAD")

    def gate(self, directory, note):
        path = self.root / "docs/reports" / directory / "integration/gate.json"
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(json.dumps({"note": note, "standing_gate": "pass"}))

    def landing(self, label, card, directory=None):
        self.gate(directory or card.lower(),
                  f"Landing integrate/{label} (base main 1234567; merges {card} abcdef0, no "
                  "conflicts). Full landing gate on 7654321.")
        return self.commit(f"landing {label}")

    def baseline_history(self, profile, revision, source):
        path = self.root / ul.EVALUATIONS / profile / "evaluation.json"
        path.parent.mkdir(parents=True, exist_ok=True)
        record = json.loads(path.read_text()) if path.is_file() else {
            "name": profile, "baseline_history": []}
        record["baseline_history"].append({
            "sha256": "0" * 64, "measured_revision": revision,
            "reason": f"Landing {revision[:9]} against published 1111111: one row moved.",
            "source": source})
        path.write_text(json.dumps(record))

    def publication(self, revision):
        path = self.root / ul.PUBLICATIONS
        path.parent.mkdir(parents=True, exist_ok=True)
        document = json.loads(path.read_text()) if path.is_file() else {
            "schema": ul.PUBLICATIONS_SCHEMA, "publications": []}
        document["publications"].append({"revision": revision, "previous": "0" * 40,
                                         "published_at": "now", "baseline": "private/x"})
        path.write_text(json.dumps(document))


class CardNames(unittest.TestCase):
    """The `merges ...` clause of the committed notes' shapes."""

    def test_the_merges_clause_names_the_cards(self):
        cases = {
            "Landing integrate/r13 (base main 0f6e17118; merges, in order, RULE50-X c74ea2139, "
            "MEMBER-Y b3a933f98 and GATE-Z 75f66c905). The second merge conflicted with MEMBER-W "
            "1234567.": ["RULE50-X", "MEMBER-Y", "GATE-Z"],
            "Landing integrate/r16 from main 285f51092: merges THREAD-A-B 6a766cb72 (conflicts "
            "only in ROUTER-ROWS abcdef1; both kept) and CURRENT-C D3 7862cca97 (kept), amendments "
            "applied at 39b1964ac. Later CARD-LATER 1234567.": ["THREAD-A-B", "CURRENT-C D3"],
            "Landing integrate/r15. History: built as integrate/r13 from main 0f6e17118 (merges "
            "CURRENT-C phase 1 3d735b88c, LINK-L 00cf0be02, OPENSSL-P a5d216a24, no conflicts), "
            "gated on 0497a6b8b": ["CURRENT-C phase 1", "LINK-L", "OPENSSL-P"],
            "Landing integrate/r41 (base main 9a769c6b0; merges CG10-SOLE round 3 0111ddf38 in "
            "its merge commit, no conflicts)": ["CG10-SOLE round 3"],
            "Landing integrate/r42 (base main 80d0e2944; merges fix/windows-plain-long-fixtures "
            "8231af235, based on main 9a769c6b0)": [],
            "Landing integrate/r17: the merge of origin/main 285f51092 (W48-A and W48-B)": [],
        }
        for note, cards in cases.items():
            with self.subTest(note[:30]):
                self.assertEqual(ul.cards_of(note), cards)


class Generation(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.repo = Repository(self.directory.name)
        self.root = self.repo.root
        self.git = ul.Git(self.root, untracked=False)

    def tearDown(self):
        self.directory.cleanup()

    def sentence(self):
        _, new, problems = ul.generated_status(self.root, self.git)
        start, end = ul.block_bounds(new)
        return new[start:end].split("\n")[1], problems

    def test_no_record_is_none(self):
        self.assertEqual(self.sentence()[0], "Unpublished landings: none; no landing gate record "
                                             "(`docs/reports/*/integration/gate.json`) exists.")

    def test_landings_after_the_newest_publication_are_unpublished(self):
        first = self.repo.landing("r1", "CARD-ONE")
        self.repo.baseline_history("demo", first, "docs/reports/landing-one/README.md")
        self.repo.baseline_history("other", first, "docs/reports/landing-one/README.md")
        self.repo.commit("record r1's baselines")
        self.repo.landing("r2", "CARD-TWO")
        note = ("Landing integrate/r3 (base main 1234567; merges CARD-THREE-A 1111111, "
                "CARD-THREE-B 2222222).")
        self.repo.gate("card-three-a", note)
        self.repo.gate("card-three-b", note)
        self.repo.commit("landing r3")
        self.repo.landing("r5", "CARD-FIVE")
        self.repo.landing("side-lane", "CARD-SIDE")
        sentence, problems = self.sentence()
        self.assertTrue(sentence.startswith(
            "Unpublished landings (user decision 2026-11-16): the newest committed landing "
            f"baseline record is `{first[:9]}` (demo and other, "
            "[record](reports/landing-one/README.md)); no later landing has one, so 4 landings "
            "completed steps 1–7 of `scripts/land.py` (records, full landing gate, merge) but not "
            "step 8: integrate/r2 (CARD-TWO) to r5 (CARD-FIVE) and integrate/side-lane "
            "(CARD-SIDE); r4 has no unpublished landing gate record. "), sentence)
        self.assertTrue(sentence.endswith(f"(`land.py publish --previous {first[:9]}`)."))
        self.assertEqual(problems, ["docs/reports/card-side/integration/gate.json: "
                                    "integrate/side-lane has no rN number"])
        lines = ul.listing(self.root, self.git)
        self.assertIn("integrate/r1: published: CARD-ONE "
                      "[docs/reports/card-one/integration/gate.json]", lines)
        self.assertIn("integrate/r3: unpublished: CARD-THREE-A and CARD-THREE-B "
                      "[docs/reports/card-three-a/integration/gate.json, "
                      "docs/reports/card-three-b/integration/gate.json]", lines)
        self.assertIn("4 of 5 landings unpublished", lines)

    def test_a_publication_that_moved_no_baseline_is_read_from_the_ledger(self):
        self.repo.landing("r1", "CARD-ONE")
        second = self.repo.landing("r2", "CARD-TWO")
        self.repo.publication(second)
        self.repo.commit("record r2's publication")
        sentence, _ = self.sentence()
        self.assertIn(f"`{second[:9]}` (no baseline moved, "
                      "[record](development/landing-publications.json)); every landing gate "
                      "record", sentence)
        self.assertIn("no landing is unpublished", sentence)
        third = self.repo.landing("r3", "CARD-THREE")
        sentence, _ = self.sentence()
        self.assertIn("so 1 landing completed steps 1–7", sentence)
        self.assertIn("not step 8: integrate/r3 (CARD-THREE).", sentence)
        self.assertNotEqual(third, second)

    def test_a_record_without_a_merges_clause_is_named_by_its_directory(self):
        self.repo.gate("windows-ci-fix", "Landing integrate/r7 (merges fix/windows 8231af235).")
        self.repo.commit("landing r7")
        sentence, problems = self.sentence()
        self.assertIn("integrate/r7 (WINDOWS-CI-FIX)", sentence)
        self.assertIn("named by its directory WINDOWS-CI-FIX", problems[0])

    def test_archived_landing_gate_records_preserve_publication_and_directory_fallback(self):
        first = self.repo.landing("r1", "CARD-ONE")
        self.repo.publication(first)
        self.repo.commit("publish r1 before archiving")
        archived = self.root / "docs/reports/archive/wave-48/card-one"
        archived.parent.mkdir(parents=True, exist_ok=True)
        git(self.root, "mv", "docs/reports/card-one", "docs/reports/archive/wave-48/card-one")
        self.repo.gate("archive/wave-48/legacy-fix",
                       "Landing integrate/r2 (merges fix/legacy 8231af235).")
        self.repo.commit("archive card-one and add archived r2")
        sentence, problems = self.sentence()
        self.assertIn("1 landing completed steps 1–7", sentence)
        self.assertIn("integrate/r2 (LEGACY-FIX)", sentence)
        self.assertIn("named by its directory LEGACY-FIX", problems[0])

    def test_untracked_records_count_only_in_production_mode(self):
        self.repo.gate("card-new", "Landing integrate/r9 (merges CARD-NEW 1234567).")
        self.assertIn("none", self.sentence()[0])
        _, new, _ = ul.generated_status(self.root, ul.Git(self.root))
        self.assertIn("integrate/r9 (CARD-NEW)", new)

    def test_a_published_revision_outside_the_object_store_is_an_error(self):
        self.repo.landing("r1", "CARD-ONE")
        self.repo.publication("f" * 40)
        with self.assertRaisesRegex(ul.LandingError, "object store"):
            ul.generated_status(self.root, self.git)

    def test_malformed_records_are_errors(self):
        self.repo.landing("r1", "CARD-ONE")
        (self.root / ul.PUBLICATIONS).parent.mkdir(parents=True, exist_ok=True)
        (self.root / ul.PUBLICATIONS).write_text(json.dumps({"publications": []}))
        with self.assertRaisesRegex(ul.LandingError, "quirin-landing-publications/1"):
            ul.generated_status(self.root, self.git)

    def test_the_markers_must_be_present_once(self):
        for text in ("# Status\n", STATUS + STATUS):
            (self.root / ul.STATUS).write_text(text)
            with self.assertRaisesRegex(ul.LandingError, "exactly one"):
                ul.generated_status(self.root, self.git)

    def test_regeneration_rewrites_only_the_block(self):
        old, new, _ = ul.generated_status(self.root, self.git)
        self.assertEqual(old, STATUS)
        self.assertTrue(new.startswith("# Current status\n\n### Unpublished landings\n\n"
                                       + ul.START_LINE + "\n"))
        self.assertTrue(new.endswith(ul.END + "\n\n## Next\n\nKept.\n"))
        self.assertEqual(ul.regenerate(new, self.sentence()[0]), new)


class RealRepository(unittest.TestCase):
    def test_the_committed_sentence_equals_the_generated_one(self):
        old, new, _ = ul.generated_status(ROOT, ul.Git(ROOT, untracked=False))
        self.assertEqual(old, new, f"run {ul.WRITE_COMMAND}")


if __name__ == "__main__":
    unittest.main()
