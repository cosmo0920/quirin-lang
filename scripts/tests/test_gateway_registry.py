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

"""Card GATEWAY-RULE-API: the gateway rule registry
(`crates/gateway-rules/registry.toml`) generates the gateway's rule tables,
the committed files equal the generated ones, and every ordered placement
equals the frozen route map's row (same rule, same route, same order)."""
import copy
import re
import sys
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "scripts"))
import gen_gateway_registry as gen  # noqa: E402


class GatewayRegistryTests(unittest.TestCase):
    def setUp(self):
        self.registry = gen.load(ROOT)
        self.rules = gen.validate(self.registry, ROOT)

    def test_committed_tables_equal_generated(self):
        self.assertEqual(gen.stale(ROOT), [], "run python3 scripts/gen_gateway_registry.py")

    def test_every_ordered_placement_equals_the_frozen_route_map(self):
        route_map = (ROOT / self.registry["route_map"]).read_text(encoding="utf-8")
        frozen = gen.route_map_tables(route_map)
        self.assertEqual(sorted(frozen), sorted(gen.TABLES))
        for position, rows in frozen.items():
            placed = [(rule["id"], placement["route"])
                      for rule, placement in gen.ordered(self.rules, position)]
            self.assertEqual(placed, rows, position)
        self.assertEqual(len(frozen["statement-fallback"]), 15)
        self.assertEqual(len(frozen["return-fallback"]), 8)

    def test_positions_follow_the_route_map_order(self):
        """Card RULE34-PILOT: the generator's positions are the rule API's
        `Position::ALL`, in its route-map order, with the same spellings; and
        the route map holds a section for each position's section number,
        the transfer's §10 included."""
        source = (ROOT / "crates/qcore-gateway-rule-api/src/position.rs").read_text(encoding="utf-8")
        listed = source.split("pub const ALL: [Self; ")[1].split("];")[0]
        variants = re.findall(r"Self::([A-Za-z]+),", listed)
        self.assertEqual(variants, list(gen.POSITIONS.values()))
        for spelling, variant in gen.POSITIONS.items():
            self.assertIn(f'Self::{variant} => "{spelling}",', source)
        self.assertEqual(list(gen.POSITIONS)[-1], "transfer")
        route_map = (ROOT / self.registry["route_map"]).read_text(encoding="utf-8")
        sections = re.findall(r'Self::[A-Za-z |:]+=> "§(\d+)', source)
        for number in sorted(set(sections), key=int):
            self.assertRegex(route_map, rf"(?m)^## {number}\. ")

    def test_every_ordered_route_has_its_ask_function(self):
        table = (ROOT / gen.TABLE_RS).read_text(encoding="utf-8")
        for position, (_, _, module, _, _) in gen.TABLES.items():
            for _, placement in gen.ordered(self.rules, position):
                self.assertIn(f"pub(super) fn {placement['route']}(", table, placement["route"])
                self.assertIn(f"ask: {module}::{placement['route']} }}", table)

    def test_replaced_arm_constants_are_gone(self):
        # Rule 35's arms moved into their package (card RULE35-PILOT).
        paths = [ROOT / "crates/qcore-c-gateway/src" / path
                 for path in ("rule_hosts.rs", "typed_const_pointer_value.rs",
                              "scalar_element_address.rs")]
        paths += [ROOT / "crates/gateway-rules/rule-35-literal-decay/src" / path
                  for path in ("lib.rs", "literal_read.rs", "array_member_decay.rs")]
        for name in ("LITERAL_ARM", "DECAY_ARM", "CONST_VALUE_ARM", "VOID_BIT_CAST_ARM",
                     "ELEMENT_ARM"):
            for path in paths:
                self.assertNotIn(f"const {name}", path.read_text(encoding="utf-8"))

    def test_a_migrated_rule_names_its_package_and_its_routes(self):
        """Card RULE35-PILOT: rule 35 is its own package, the gateway keeps
        none of its decision modules, and a registry row naming a missing
        package or a route that is not a `pub fn` of it fails closed."""
        rule35 = next(r for r in self.rules if r["id"] == "35")
        self.assertEqual(rule35["crate"], "rule-35-literal-decay")
        self.assertIsNotNone(gen.rule_package(ROOT, "rule-35-literal-decay"))
        for gone in ("literal_dereference.rs", "array_member_decay.rs"):
            self.assertFalse((ROOT / "crates/qcore-c-gateway/src" / gone).exists(), gone)

        def rule(registry, rule_id):
            return next(r for r in registry["rule"] if r["id"] == rule_id)

        self.broken(lambda r: rule(r, "35").update(crate="rule-99-missing"))
        self.broken(lambda r: rule(r, "35")["placements"][0].update(route="literal_read::exempt_x"))
        self.broken(lambda r: rule(r, "35")["placements"][1].update(route="no_module::decide"))
        self.broken(lambda r: rule(r, "10s").update(crate="rule-35-literal-decay-x"))

    def broken(self, mutate):
        registry = copy.deepcopy(self.registry)
        mutate(registry)
        with self.assertRaises(gen.RegistryError):
            gen.validate(registry, ROOT)

    def test_broken_registries_fail_closed(self):
        def rule(registry, rule_id):
            return next(r for r in registry["rule"] if r["id"] == rule_id)

        self.broken(lambda r: r["rule"].append(copy.deepcopy(r["rule"][0])))
        self.broken(lambda r: rule(r, "10s")["placements"][0].update(order=99))
        self.broken(lambda r: rule(r, "10s")["placements"][0].update(arm="x"))
        self.broken(lambda r: rule(r, "10s")["placements"][0].pop("clears_origins"))
        self.broken(lambda r: rule(r, "10s")["placements"][1].update(clears_origins=True))
        self.broken(lambda r: rule(r, "10s")["placements"][0].update(position="nowhere"))
        self.broken(lambda r: rule(r, "10s").update(off="sometimes"))
        self.broken(lambda r: rule(r, "10s").update(lean=["QCore.NoSuchModule"]))
        self.broken(lambda r: rule(r, "10s").update(document="docs/agents/rules/none.md"))
        self.broken(lambda r: rule(r, "10s").update(extra=1))
        self.broken(lambda r: rule(r, "35")["placements"][0].update(position="token-hold"))
        self.broken(lambda r: rule(r, "35")["placements"][0].update(route="decide"))
        self.broken(lambda r: rule(r, "35")["placements"].append(
            {"position": "condition", "order": 1, "route": "planned_condition"}))
        self.broken(lambda r: r.update(schema="qcore-gateway-rules/0"))

    def test_a_hold_must_be_registered_not_switchable(self):
        # Card RULE32A-PILOT: rule 32a's caller-frame hold is registered, not
        # switchable, and reserves its reason key; a refusing off is refused.
        def rule(registry, rule_id):
            return next(r for r in registry["rule"] if r["id"] == rule_id)

        hold = rule(self.registry, "32a")
        self.assertEqual(hold["off"], "not-switchable")
        self.assertEqual(hold["crate"], "rule-32a-caller-frame-hold")
        self.assertEqual([p["position"] for p in hold["placements"]], ["token-hold"])
        self.assertEqual(hold["reason_keys"], [{"key": "caller-frame-self-reliance",
                                                "category": "incomplete-obligation-coverage"}])
        self.broken(lambda r: rule(r, "32a").update(off="refuses"))
        registry = copy.deepcopy(self.registry)
        other = {"id": "hold-x", "crate": "qcore-c-gateway", "lean": [],
                 "document": "docs/agents/rules/32a.md", "off": "refuses",
                 "placements": [{"position": "call-hold", "arm": "other-hold",
                                 "route": "legacy_other_hold::hold"}]}
        registry["rule"].append(other)
        with self.assertRaises(gen.RegistryError):
            gen.validate(registry, ROOT)
        other["off"] = "not-switchable"
        self.assertEqual(len(gen.validate(registry, ROOT)), len(self.rules) + 1)
        other["reason_keys"] = [{"key": "caller-frame-self-reliance",
                                 "category": "incomplete-obligation-coverage"}]
        with self.assertRaises(gen.RegistryError):
            gen.validate(registry, ROOT)

    def test_missing_markers_fail_closed(self):
        with self.assertRaises(gen.RegistryError):
            gen.splice("no markers here", "region")
        with self.assertRaises(gen.RegistryError):
            gen.splice(f"{gen.START}\n{gen.END}\n{gen.START}\n{gen.END}\n", "region")

    def test_a_moved_route_map_row_is_detected(self):
        route_map = (ROOT / self.registry["route_map"]).read_text(encoding="utf-8")
        moved = route_map.replace("`connective_return`", "`moved_return`")
        self.assertNotEqual(moved, route_map)
        self.assertNotEqual(gen.route_map_tables(moved)["return-fallback"],
                            gen.route_map_tables(route_map)["return-fallback"])
        with tempfile.TemporaryDirectory() as directory:
            Path(directory, "map.md").write_text("no tables", encoding="utf-8")
            with self.assertRaises(gen.RegistryError):
                gen.route_map_tables(Path(directory, "map.md").read_text(encoding="utf-8"))


if __name__ == "__main__":
    unittest.main()
