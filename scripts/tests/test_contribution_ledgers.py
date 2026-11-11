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

"""Card CONTRIBUTION-LEDGERS: `scripts/contribution_ledgers.py` validates the
per-rule ledgers fail-closed, writes them from a native-comparison record's
contribution slot with their provenance, checks each live profile's real-run
total against base + contributions + named interactions without repairing it,
and renders the committed contributions page. Fixture trees are built file by
file (no directory walk in this test; constraint 29)."""
import contextlib
import copy
import hashlib
import io
import json
import sys
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts"))
import contribution_ledgers as cl  # noqa: E402

REV = "1" * 40
BINARY = "b" * 64
ON = {"Safe": 3, "Unknown": 1}
BASE = {"Unknown": 4}


def sha(data):
    return hashlib.sha256(data).hexdigest()


def figures(statuses=None):
    statuses = statuses or ON
    return {"rows": sum(statuses.values()), "row_statuses": statuses,
            "row_verdicts_sha256": "c" * 64}


def move(row, before="Unknown", after="Safe", premises=None):
    return {"row": row, "before": before, "after": after,
            "premises": {"declarations": premises or {}},
            "leaves_removed": [{"description": "leaf"}], "leaves_added": []}


def ledger(kind, key, rules, rows, delta, off, group, comparison):
    return {
        "schema": cl.SCHEMA, "kind": kind, "key": key, "rules": rules, "profile": "demo",
        "route": "corpus", "source": cl.SOURCE,
        "measurement": {"tool": {"name": "quirin-audit-measure"}, "tip_side": "tip",
                        "comparison": comparison, "binary_sha256": BINARY, "revision": REV,
                        "build_commit": REV, "bundles": {"contract": "d" * 64},
                        "source_date_epoch": 1767225600},
        "group": group, "totals": {"off": off, "on": ON}, "delta": delta,
        "rows_moved": rows,
        "premises": sorted({name for row in rows
                            for names in row["premises"]["declarations"].values()
                            for name in names}),
        "leaves_traded": {"removed": 0, "added": 0, "rows": []},
        "rows_unknown": [],
    }


def slot(tip_figures=None, rules=("33", "35"), per_rule=True):
    """The ledgers the measure plugin builds for rows f (rule 33), g and h
    (interaction 33+35), as crates/quirin-audit-plugin-measure's test does."""
    group = {"rules": list(rules), "per_rule": per_rule, "interactions": ["33+35"],
             "joint_comparison": "tip-off->tip", "base": BASE, "on": ON,
             "tip_figures": tip_figures if tip_figures is not None else figures(),
             "interactions_measured": per_rule}
    f = move("lib::a.c::unit#0::p::demo::f#0", premises={"log0": ["rk_plugin_callback_closure"]})
    g = dict(move("lib::a.c::unit#0::p::demo::g#0"),
             joint={"before": "Unknown", "after": "Safe"},
             per_rule={"33": {"before": "Unknown", "after": "Safe"},
                       "35": {"before": "Unknown", "after": "Safe"}})
    h = dict(move("lib::a.c::unit#0::p::demo::h#0"),
             joint={"before": "Unknown", "after": "Safe"}, per_rule={})
    ledgers = [
        ledger("rule", "33", ["33"], [f], {"Safe": 1, "Unknown": -1},
               {"Safe": 1, "Unknown": 3}, group, "tip-off-33->tip"),
        ledger("rule", "35", ["35"], [], {}, {"Safe": 2, "Unknown": 2}, group,
               "tip-off-35->tip"),
        ledger("interaction", "33+35", ["33", "35"], [g, h], {"Safe": 2, "Unknown": -2},
               BASE, group, "tip-off->tip"),
    ]
    return {"schema": cl.SCHEMA, "rules": list(rules), "per_rule": per_rule,
            "comparisons": [], "ledgers": ledgers}


RULE_DOC = "---\nrule: {id}\ntopic: gateway\nsummary: A rule.\n---\n\n{id}. **Rule.**\n"


class Tree:
    """A repository root with one live profile `demo`, rules 33 and 35 and a
    committed native-comparison record."""

    def __init__(self, directory, statuses=None):
        self.root = Path(directory)
        for rule in ("33", "35"):
            self.put(f"docs/agents/rules/{rule}.md", RULE_DOC.format(id=rule))
        self.put("docs/agents/rules/11-series.md", "---\nrule: 11-series\ntopic: gateway\n---\n")
        self.set_baseline(statuses)

    def put(self, relative, text):
        path = self.root / relative
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(text, encoding="utf-8")
        return path

    def set_baseline(self, statuses=None, kind="live"):
        baseline = json.dumps({"schema": "quirin-evaluation-baseline/1", "profile": "demo",
                               "measured_revision": REV, "source": "r.md",
                               "figures": figures(statuses), "artifacts": []}, indent=2) + "\n"
        self.put("contract/evaluations/demo/baseline.json", baseline)
        self.put("contract/evaluations/demo/evaluation.json", json.dumps({
            "schema": "quirin-evaluation-profile/1", "name": "demo",
            "policy": {"kind": kind} if kind == "live" else {"kind": kind, "revision": REV},
            "baseline": {"path": "contract/evaluations/demo/baseline.json",
                         "sha256": sha(baseline.encode())}}, indent=2) + "\n")

    def record(self, contribution=None, name="card", status="complete"):
        relative = f"docs/reports/{name}/native-comparison.json"
        self.put(relative, json.dumps({"schema": "quirin-native-comparison/1", "status": status,
                                       "contribution": contribution or slot()}, indent=2))
        return relative

    def write(self, relative):
        return cl.write(self.root, self.root / relative)


def refused(test, message, function, *args):
    with test.assertRaises(cl.LedgerError) as caught:
        function(*args)
    test.assertIn(message, str(caught.exception))


class RecordSlot(unittest.TestCase):
    def test_the_slot_validates_and_its_group_adds_up(self):
        self.assertEqual(len(cl.validate_record({"contribution": slot()})), 3)

    def test_defects_are_refused(self):
        cases = {
            "delta": (lambda s: s["ledgers"][0].update(delta={"Safe": 2, "Unknown": -2}),
                      "is not the rows'"),
            "missing interaction": (lambda s: s["ledgers"].pop(2), "the group lists"),
            "unknown key": (lambda s: s["ledgers"][0].update(extra=1), "takes exactly"),
            "source": (lambda s: s["ledgers"][0].update(source="report-transcribed"),
                       "only 'measured-by-switch'"),
            "interaction key": (lambda s: s["ledgers"][2].update(key="35+33"), "rule order"),
            "binary": (lambda s: s["ledgers"][0]["measurement"].update(binary_sha256=None),
                       "binary_sha256"),
            "base": (lambda s: [ledger["group"].update(base={"Unknown": 5})
                                for ledger in s["ledgers"]], "never repaired"),
            "refused slot": (lambda s: s.update(refused="failed"), "was refused"),
        }
        for name, (change, message) in cases.items():
            with self.subTest(name):
                document = copy.deepcopy(slot())
                change(document)
                refused(self, message, cl.validate_record, {"contribution": document})

    def test_unseparated_rules_have_no_rule_ledger(self):
        document = slot(per_rule=False)
        refused(self, "without --per-rule", cl.validate_record, {"contribution": document})


class WriteCheckRender(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.tree = Tree(self.directory.name)
        self.root = self.tree.root

    def tearDown(self):
        self.directory.cleanup()

    def test_written_ledgers_check_and_render(self):
        written = self.tree.write(self.tree.record())
        self.assertEqual(written, ["contract/contributions/33/demo.json",
                                   "contract/contributions/35/demo.json",
                                   "contract/contributions/interactions/33+35/demo.json"])
        stored = json.loads((self.root / written[0]).read_text())
        self.assertEqual(stored["record"]["path"], "docs/reports/card/native-comparison.json")
        self.assertEqual(stored["record"]["sha256"],
                         sha((self.root / stored["record"]["path"]).read_bytes()))
        self.assertEqual(len(cl.check(self.root)), 3)
        reports, errors = cl.consistency(self.root)
        self.assertEqual(errors, [])
        page = cl.render(self.root)
        self.assertIn("rule 33: one row under one premise (`rk_plugin_callback_closure`)", page)
        self.assertIn("rule 35: no rows", page)
        self.assertIn("interaction 33+35: two rows under no named premise; Unknown→Safe 2", page)
        self.assertIn("- base (rules 33, 35 off): Unknown 4", page)
        self.assertIn("= Safe 3, Unknown 1 = the real-run total", page)
        self.assertLess(page.index("rule 33:"), page.index("rule 35:"))
        self.assertLess(page.index("rule 35:"), page.index("interaction 33+35:"))

    def test_write_refusals(self):
        record = self.tree.record(slot(tip_figures=figures({"Safe": 2, "Unknown": 2})))
        refused(self, "not the profile's baseline figures", self.tree.write, record)
        refused(self, "not complete", self.tree.write,
                self.tree.record(name="failed", status="failed"))
        outside = Path(tempfile.mkdtemp()) / "native-comparison.json"
        outside.write_text(json.dumps({"status": "complete", "contribution": slot()}))
        refused(self, "is outside", cl.write, self.root, outside)
        self.tree.set_baseline(kind="frozen")
        refused(self, "not a live profile", self.tree.write, self.tree.record())
        self.assertFalse((self.root / "contract/contributions").exists(), "nothing written")

    def test_rule_ids_are_checked_against_the_documents_then_the_registry(self):
        document = slot()
        for item in document["ledgers"]:
            item["group"]["rules"] = ["11-series", "35"] if item["key"] == "33" else ["33", "35"]
        cl.known_rule(self.root, "33")
        refused(self, "has no rule document", cl.known_rule, self.root, "99")
        refused(self, "router row", cl.known_rule, self.root, "11-series")
        self.tree.put("crates/gateway-rules/registry.toml",
                      '[[rule]]\nid = "35"\ncrate = "rule-35"\n')
        cl.known_rule(self.root, "35")
        refused(self, "is not in crates/gateway-rules/registry.toml", cl.known_rule,
                self.root, "33")
        self.tree.put("crates/gateway-rules/registry.toml", "[[rule]\n")
        refused(self, "is not TOML", cl.known_rule, self.root, "35")

    def test_check_refuses_tampering_and_strays(self):
        record = self.tree.record()
        self.tree.write(record)
        path = self.root / "contract/contributions/33/demo.json"
        original = path.read_text()
        cases = {
            "record digest": (lambda: self.tree.put(record, "{}"), "record digest"),
            "slot entry": (lambda: path.write_text(original.replace('"route": "corpus"',
                                                                    '"route": "other"')),
                           "not an entry"),
            "stray file": (lambda: self.tree.put("contract/contributions/notes.md", "x"),
                           "only ledgers live under"),
            "wrong place": (lambda: (self.root / "contract/contributions/35/x.json")
                            .write_text(original), "belongs at"),
            "missing group member": (lambda: (self.root / "contract/contributions/35/demo.json")
                                     .unlink(), "the group lists"),
        }
        for name, (change, message) in cases.items():
            with self.subTest(name), tempfile.TemporaryDirectory() as directory:
                self.tree = Tree(directory)
                self.root = self.tree.root
                record = self.tree.record()
                self.tree.write(record)
                path = self.root / "contract/contributions/33/demo.json"
                original = path.read_text()
                change()
                refused(self, message, cl.check, self.root)

    def test_a_moved_baseline_makes_the_group_stale_not_wrong(self):
        self.tree.write(self.tree.record())
        self.tree.set_baseline({"Safe": 4})
        _, errors = cl.consistency(self.root)
        self.assertEqual(errors, [])
        self.assertIn("stale: measured on baseline", cl.render(self.root))

    def test_a_current_group_that_misses_the_real_total_fails_and_is_not_repaired(self):
        self.tree.write(self.tree.record())
        ledgers = [self.root / "contract/contributions" / p for p in
                   ("33/demo.json", "35/demo.json", "interactions/33+35/demo.json")]
        # Replace the baseline's figures without re-stamping the ledgers: the
        # stamp still matches only if the digest is kept, so rewrite both the
        # baseline and the ledgers' stamp to the new digest.
        self.tree.set_baseline({"Safe": 2, "Unknown": 2})
        digest = json.loads((self.root / "contract/evaluations/demo/evaluation.json")
                            .read_text())["baseline"]["sha256"]
        for path in ledgers:
            stored = json.loads(path.read_text())
            stored["profile_baseline"]["sha256"] = digest
            path.write_text(cl.dump(stored))
        before = {path: path.read_bytes() for path in ledgers}
        _, errors = cl.consistency(self.root)
        self.assertTrue(any("not the real-run total" in error for error in errors), errors)
        self.assertTrue(any("base plus contributions plus interactions" in error
                            for error in errors), errors)
        self.assertEqual({path: path.read_bytes() for path in ledgers}, before)
        self.assertIn("consistency: FAILS", cl.render(self.root))
        out, err = io.StringIO(), io.StringIO()
        with contextlib.redirect_stdout(out), contextlib.redirect_stderr(err):
            code = cl.main(["--root", str(self.root), "consistency"])
        self.assertEqual(code, 1)
        self.assertIn("inconsistent", err.getvalue())

    def test_two_current_measurements_moving_one_row_are_an_unnamed_interaction(self):
        def alone(rule, rows, base, binary):
            group = {"rules": [rule], "per_rule": False, "interactions": [],
                     "joint_comparison": "tip-off->tip", "base": base, "on": ON,
                     "tip_figures": figures(), "interactions_measured": False}
            delta = {"Safe": len(rows), "Unknown": -len(rows)}
            item = ledger("rule", rule, [rule], [move(row) for row in rows], delta, base, group,
                          "tip-off->tip")
            item["measurement"]["binary_sha256"] = binary
            return {"schema": cl.SCHEMA, "rules": [rule], "per_rule": False,
                    "comparisons": [], "ledgers": [item]}

        self.tree.write(self.tree.record(alone("33", ["f", "g"], {"Safe": 1, "Unknown": 3},
                                               BINARY), name="a"))
        self.tree.write(self.tree.record(alone("35", ["g"], {"Safe": 2, "Unknown": 2},
                                               "e" * 64), name="b"))
        self.assertEqual(len(cl.check(self.root)), 2)
        _, errors = cl.consistency(self.root)
        self.assertTrue(any("row g is also moved" in error and "unnamed interaction" in error
                            for error in errors), errors)
        page = cl.render(self.root)
        self.assertIn("interactions of rule 33 with other rules: not measured", page)

    def test_rewriting_one_rule_of_a_group_leaves_the_group_incomplete(self):
        self.tree.write(self.tree.record())
        single = slot()
        single["ledgers"] = [single["ledgers"][0]]
        with self.assertRaises(cl.LedgerError):
            cl.validate_record({"contribution": single})


class Contribution(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.tree = Tree(self.directory.name)
        self.root = self.tree.root
        self.tree.write(self.tree.record())
        self.paths = ["contract/contributions/33/demo.json",
                      "contract/contributions/35/demo.json",
                      "contract/contributions/interactions/33+35/demo.json"]

    def tearDown(self):
        self.directory.cleanup()

    def test_a_complete_listing_passes_with_notes(self):
        notes = cl.check_contribution(self.root, {"ledgers": self.paths})
        self.assertEqual(len(notes), 3)
        self.assertIn("rule 33: one row", notes[0])
        self.assertEqual(cl.check_contribution(self.root, None), [])

    def test_refusals(self):
        cases = {
            "shape": ({"rows": []}, "must be null or"),
            "empty": ({"ledgers": []}, "non-empty"),
            "omitted member": ({"ledgers": self.paths[:2]}, "omits"),
            "absent": ({"ledgers": ["contract/contributions/36/demo.json"]}, "no such ledger"),
            "elsewhere": ({"ledgers": ["docs/x.json"]}, "is not under"),
        }
        for name, (contribution, message) in cases.items():
            with self.subTest(name):
                refused(self, message, cl.check_contribution, self.root, contribution)

    def test_a_stale_ledger_is_noted_not_refused(self):
        self.tree.set_baseline({"Safe": 4})
        notes = cl.check_contribution(self.root, {"ledgers": self.paths})
        self.assertTrue(all("stale against demo" in note for note in notes), notes)


class RealRepository(unittest.TestCase):
    def test_committed_ledgers_check_and_are_consistent(self):
        cl.check(ROOT)
        _, errors = cl.consistency(ROOT)
        self.assertEqual(errors, [])

    def test_the_committed_page_equals_the_generated_page(self):
        self.assertEqual((ROOT / cl.PAGE).read_text(encoding="utf-8"), cl.render(ROOT),
                         "run python3 scripts/contribution_ledgers.py render")

    def test_status_links_the_page_or_its_amendment_does(self):
        """STATUS.md links the page once CONTRIBUTION-LEDGERS' status paragraph
        is applied; until then the card's amendments carry the link."""
        link = "development/contributions.md"
        status = (ROOT / "docs/STATUS.md").read_text(encoding="utf-8")
        amendments = ROOT / "docs/reports/contribution-ledgers/integration-amendments.json"
        pending = amendments.is_file() and link in (
            json.loads(amendments.read_text())["status_paragraph"] or "")
        self.assertTrue(link in status or pending)


if __name__ == "__main__":
    unittest.main()
