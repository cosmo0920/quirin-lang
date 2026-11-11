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

"""The scoper's run-time attribution against an observed read census.

Card GATE-SCOPE-PRECISE-SELECTION (R1, GATE-SCOPE-RUNTIME-READS-HEURISTIC).
``scripts/gate_read_census.py`` ran every root-workspace test executable under
strace and recorded the repository paths each one read
(``docs/reports/gate-scope-precise-selection/read-census.json``).  These
tests hold the scoper's model of this tree to that observation:

* every tracked file a unit was seen reading selects that unit (its crate as a
  whole, or that test target) -- no observed reader is dropped;
* every walker entry is walked: some unit compiling its source was seen
  reading a file it declares (a declaration is not decoration);
* every directory literal is declared, and no declaration is stale.

The census is the evidence of one commit.  A unit or file that no longer
exists is skipped, and a test that starts reading a new path through data is
not seen until the census is recorded again (the landing gate stays the
arbiter).
"""
import json
from pathlib import Path
import shutil
import sys
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import gate_scope as scope_mod  # noqa: E402

REPO_ROOT = Path(__file__).resolve().parents[2]
CENSUS = REPO_ROOT / "docs/reports/gate-scope-precise-selection/read-census.json"


def unit_ids(model):
    """Unit id -> crate, for the root workspace's test-run targets."""
    ids = {}
    for name, targets in model.targets.items():
        if model.crates[name].workspace != "root":
            continue
        for target in targets:
            ids[scope_mod.unit_uid(name, target)] = name
    return ids


@unittest.skipIf(shutil.which("cargo") is None or shutil.which("git") is None,
                 "cargo or git is not installed")
class CensusAgreesWithTheModel(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.model = scope_mod.real_model()
        cls.census = json.loads(CENSUS.read_text(encoding="utf-8"))
        cls.tracked = set(cls.model.files)
        cls.units = unit_ids(cls.model)
        cls.cache = {}

    def scope(self, path):
        if path not in self.cache:
            self.cache[path] = scope_mod.plan([path], self.model, lean=False)
        return self.cache[path]

    def test_the_census_covers_the_suite(self):
        self.assertEqual(self.census["schema"], "quirin-gate-read-census/1")
        present = set(self.census["units"]) & set(self.units)
        # Most units of the tree were observed (new ones since are not).
        self.assertGreater(len(present), 0.9 * len(self.units))

    def test_every_observed_read_selects_its_unit(self):
        missed = []
        for uid, unit in sorted(self.census["units"].items()):
            if uid not in self.units:
                continue  # removed or renamed since the census
            for path in unit["read"]:
                if path not in self.tracked:
                    continue  # generated or untracked: no diff names it
                if not scope_mod.selects(self.scope(path), uid):
                    missed.append(f"{uid} reads {path}")
        self.assertEqual(missed, [], "observed readers the scoper drops:\n" + "\n".join(missed))

    def test_every_walker_entry_is_walked(self):
        table = scope_mod.load_table()
        unwalked = []
        for entry in table["walkers"]:
            if not entry["reads"]:
                continue
            crates = {name for (name, path) in self.model.file_units if path == entry["source"]}
            crates |= {name for (name, path) in self.model.file_nontest if path == entry["source"]}
            units = {uid for uid, crate in self.units.items() if crate in crates}
            observed = [self.census["units"][uid] for uid in units if uid in self.census["units"]]
            if not observed:
                # Provider-workspace code: the census traces the root suite only.
                self.assertTrue(all(self.model.crates[c].workspace == "providers"
                                    for c in crates), entry)
                continue
            reads = {path for unit in observed for path in unit["read"]}
            regexes = [scope_mod.glob_regex(pattern) for pattern in entry["reads"]]
            literal = entry["literal"]
            # A listing walks too: a directory opened at or under the literal,
            # or a `git ls-files` whose pathspec covers it.
            listed = any(d == literal or d.startswith(literal + "/")
                         for unit in observed for d in unit["dirs"]) or any(
                arg in (".", literal) or literal.startswith(arg.rstrip("/") + "/")
                for unit in observed for spec in unit["ls_files"]
                for arg in json.loads(spec) if not arg.startswith("-"))
            if not listed and not any(regex.match(path) for path in reads for regex in regexes):
                unwalked.append(f"{entry['source']} {entry['literal']}")
        self.assertEqual(unwalked, [], "walker entries no observed unit walks")

    def test_every_directory_literal_is_declared(self):
        self.assertEqual(
            [f"{site['source']}: {site['literal']}" for site in self.model.undeclared], [],
            "declare these sites in scripts/gate_scope_walkers.json")

    def test_no_walker_entry_is_stale(self):
        self.assertEqual(self.model.stale_walkers(), [])


if __name__ == "__main__":
    unittest.main()
