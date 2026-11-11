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

"""Card GENERATED-ROOTS: the Lean roots, the rule topic pages' constraint
sections and the rule router are generated, and the committed files equal the
generated ones. Inputs are tracked files only (constraint 29)."""
import re
import shutil
import sys
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(Path(__file__).resolve().parent))
sys.path.insert(0, str(ROOT / "scripts"))
from tracked_files import tracked_files  # noqa: E402
import gen_lean_roots  # noqa: E402
import gen_rule_docs  # noqa: E402

AGENTS = ROOT / "docs/agents"
PROOF = ROOT / "proof"


class LeanRootsTests(unittest.TestCase):
    def setUp(self):
        self.modules = tracked_files(PROOF / "QCore")

    def test_committed_roots_equal_generated(self):
        self.assertEqual(gen_lean_roots.stale(self.modules, PROOF), [],
                         "run python3 scripts/gen_lean_roots.py")

    def test_every_module_is_imported_once(self):
        text = (PROOF / "QCore.lean").read_text(encoding="utf-8")
        imports = re.findall(r"^import (\S+)$", text, re.M)
        expected = sorted(".".join(path.relative_to(PROOF).with_suffix("").parts)
                          for path in self.modules if path.suffix == ".lean")
        self.assertEqual(imports, expected)

    def test_whole_project_audit_imports_the_root(self):
        text = (PROOF / "AuditAll.lean").read_text(encoding="utf-8")
        self.assertIn("\nimport QCore\n", text)
        self.assertIn("QCore axiom audit", text)

    def test_unimportable_module_name_fails_closed(self):
        with self.assertRaises(ValueError):
            gen_lean_roots.modules([PROOF / "QCore" / "Not-A-Name.lean"], PROOF)
        with self.assertRaises(ValueError):
            gen_lean_roots.modules([], PROOF)


class RuleDocsTests(unittest.TestCase):
    def setUp(self):
        self.files = tracked_files(AGENTS)

    def test_committed_pages_equal_generated(self):
        self.assertEqual(gen_rule_docs.stale(self.files, AGENTS), [],
                         "run python3 scripts/gen_rule_docs.py")

    def test_every_router_row_has_a_rule_file(self):
        rules = {rule["id"]: rule for rule in gen_rule_docs.load_rules(self.files, AGENTS)}
        rows = gen_rule_docs.router_rows((AGENTS / "README.md").read_text(encoding="utf-8"))
        self.assertGreater(len(rows), 0)
        for rule_id, page, anchor in rows:
            self.assertEqual(rule_id, anchor)
            self.assertIn(rule_id, rules, f"router row {rule_id} has no rule file")
            self.assertEqual(f"{rules[rule_id]['topic']}.md", page)
        self.assertEqual(sorted(row[0] for row in rows),
                         sorted(rule_id for rule_id, rule in rules.items()
                                if rule["summary"] is not None))

    def test_every_rule_anchor_is_on_its_page_once(self):
        for rule in gen_rule_docs.load_rules(self.files, AGENTS):
            page = (AGENTS / f"{rule['topic']}.md").read_text(encoding="utf-8")
            self.assertEqual(page.count(f'<a id="constraint-{rule["id"]}"></a>'), 1, rule["id"])

    def test_natural_order(self):
        ids = ["10z", "4", "11a", "11-series", "4a", "15aa", "15z", "11", "5"]
        self.assertEqual(sorted(ids, key=gen_rule_docs.natural_key),
                         ["4", "4a", "5", "10z", "11", "11-series", "11a", "15z", "15aa"])
        with self.assertRaises(gen_rule_docs.RuleError):
            gen_rule_docs.natural_key("A1")


class RuleDocsGeneratorTests(unittest.TestCase):
    """The generator on a temporary copy: a card adds a rule file only."""

    def setUp(self):
        self.tmp = Path(tempfile.mkdtemp())
        self.agents = self.tmp / "agents"
        (self.agents / "rules").mkdir(parents=True)
        (self.agents / "README.md").write_text(
            "# Router\n\n" + gen_rule_docs.ROUTER_START + "\n\n" + gen_rule_docs.ROUTER_END + "\n",
            encoding="utf-8")
        (self.agents / "gateway.md").write_text(
            "# Gateway\n\n" + gen_rule_docs.PAGE_START + "\n\n" + gen_rule_docs.PAGE_END
            + "\n\n## Trailer\n\nkept\n", encoding="utf-8")
        self.files = [self.agents / "README.md", self.agents / "gateway.md"]
        self.add("4", "gateway", "Four.", "4. Rule four, see [5](#constraint-5).")

    def tearDown(self):
        shutil.rmtree(self.tmp)

    def add(self, rule_id, topic, summary, body):
        path = self.agents / "rules" / f"{rule_id}.md"
        path.write_text(gen_rule_docs.render_rule_file(
            {"id": rule_id, "topic": topic, "summary": summary, "body": body}), encoding="utf-8")
        self.files.append(path)
        return path

    def write(self):
        for path, text in gen_rule_docs.render(self.files, self.agents).items():
            path.write_text(text, encoding="utf-8")

    def test_new_rule_lands_in_order_with_a_router_row(self):
        self.add("38", "gateway", "Thirty-eight.", "38. Rule thirty-eight.")
        self.add("4a", "gateway", None, "Lead-in without a router row.")
        self.write()
        page = (self.agents / "gateway.md").read_text(encoding="utf-8")
        self.assertLess(page.index("constraint-4\""), page.index("constraint-4a\""))
        self.assertLess(page.index("constraint-4a\""), page.index("constraint-38\""))
        self.assertTrue(page.endswith(gen_rule_docs.PAGE_END + "\n\n## Trailer\n\nkept\n"))
        router = (self.agents / "README.md").read_text(encoding="utf-8")
        self.assertEqual([row[0] for row in gen_rule_docs.router_rows(router)], ["4", "38"])
        self.assertEqual(gen_rule_docs.stale(self.files, self.agents), [])

    def test_hand_edit_of_a_generated_section_is_stale(self):
        self.write()
        page = self.agents / "gateway.md"
        page.write_text(page.read_text(encoding="utf-8").replace("Rule four", "Rule 4"),
                        encoding="utf-8")
        self.assertEqual(gen_rule_docs.stale(self.files, self.agents), [page])

    def test_malformed_rule_files_fail_closed(self):
        cases = {
            "5": "---\nrule: 6\ntopic: gateway\n---\n\ntext\n",          # name mismatch
            "6": "---\nrule: 6\ntopic: nowhere\n---\n\ntext\n",          # no such page
            "7": "---\nrule: 7\ntopic: gateway\n---\n\n<a id=\"constraint-7\"></a>\n",
            "8": "rule: 8\n",                                             # no front matter
            "9": "---\nrule: 9\ntopic: gateway\ncolour: red\n---\n\ntext\n",
            "10": "---\nrule: 10\ntopic: gateway\n---\n\n\n",            # empty text
        }
        for rule_id, text in cases.items():
            path = self.agents / "rules" / f"{rule_id}.md"
            path.write_text(text, encoding="utf-8")
            with self.subTest(rule_id), self.assertRaises(gen_rule_docs.RuleError):
                gen_rule_docs.render(self.files + [path], self.agents)
            path.unlink()


if __name__ == "__main__":
    unittest.main()
