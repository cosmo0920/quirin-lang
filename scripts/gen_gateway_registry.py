#!/usr/bin/env python3
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

"""Generate the C gateway's rule tables from `crates/gateway-rules/registry.toml`.

Card GATEWAY-RULE-API (plan docs/plans/pluggable-cards/README.md, seam B). The
registry is the source; this script writes

* `crates/qcore-c-gateway/src/gateway_rule_registry.rs`, the whole file: the
  registry as `qcore_gateway_rule_api::RuleEntry` rows (what the run-time
  switches `QCORE_GATEWAY_RULES_OFF` are checked against), and
* the region between the `generated:registry` markers of
  `crates/qcore-c-gateway/src/legacy_route_table.rs`: the ordered tables of the
  positions that have one (statement and return fallbacks), each row naming
  the hand-written `ask` function of its route.

It validates first and fails closed: an unknown key or position, a placement
with both or neither of `order` and `arm`, a gap or repeat in a table's
orders, an ordered placement at a position with no generated table, a hold
that is switchable, a missing rule document or Lean module, a duplicated id
or reason key, or a crate that is neither `qcore-c-gateway` nor a rule package
`crates/gateway-rules/<crate>/` (card RULE35-PILOT) whose arm routes are
`pub fn`s of its `src/<module>.rs` writes nothing. Every ordered placement must also equal the
frozen route map's row; the Python suite checks that
(`scripts/tests/test_gateway_registry.py`), with committed equals generated.

Regenerate with `python3 scripts/gen_gateway_registry.py`; validate with
`--check`. On a merge conflict in a generated file, take either side and rerun.
"""
from pathlib import Path
import argparse
import re
import tomllib

ROOT = Path(__file__).resolve().parents[1]
REGISTRY = Path("crates/gateway-rules/registry.toml")
REGISTRY_RS = Path("crates/qcore-c-gateway/src/gateway_rule_registry.rs")
TABLE_RS = Path("crates/qcore-c-gateway/src/legacy_route_table.rs")
SCHEMA = "qcore-gateway-rules/1"
GATEWAY = "qcore-c-gateway"
RULE_PACKAGES = Path("crates/gateway-rules")
START = "// generated:registry:start"
END = "// generated:registry:end"

# Registry spelling -> `qcore_gateway_rule_api::Position` variant, in the
# route-map order of `Position::ALL` (the Python suite checks both).
POSITIONS = {
    "token-hold": "TokenHold",
    "statement-pre": "StatementPre",
    "classification": "Classification",
    "call-hold": "CallHold",
    "statement-rewrite": "StatementRewrite",
    "statement-fallback": "StatementFallback",
    "statement-attribution": "StatementAttribution",
    "condition": "Condition",
    "return-route": "ReturnRoute",
    "return-fallback": "ReturnFallback",
    "typed-reader": "TypedReader",
    # Card RULE34-PILOT: the transfer of one admitted contract call (route
    # map §10), where rule 34 orders a destroyer's unlink and release.
    "transfer": "Transfer",
}
HOLDS = {"token-hold", "call-hold"}
OFF = {"refuses": "Refuses", "not-switchable": "NotSwitchable"}

# The positions whose ordered table this script generates: constant, row
# type, `ask` module, and whether rows carry `clears_origins`.
TABLES = {
    "statement-fallback": ("STATEMENT_FALLBACKS", "StatementFallback", "statement", True,
                           "The statement fallbacks of `expression_statement`, in consultation\n"
                           "/// order (the registry's `statement-fallback` placements)."),
    "return-fallback": ("RETURN_FALLBACKS", "ReturnFallback", "returns", False,
                        "The return fallbacks of the `return` arm, in consultation order (the\n"
                        "/// registry's `return-fallback` placements)."),
}

RULE_KEYS = {"id", "crate", "lean", "document", "off", "placements", "reason_keys"}
PLACEMENT_KEYS = {"position", "order", "arm", "route", "clears_origins"}
PLAIN_ID = re.compile(r"^[A-Za-z0-9._-]+$")
IDENTIFIER = re.compile(r"^[a-z_][a-z0-9_]*$")
ARM_ROUTE = re.compile(r"^[a-z_][a-z0-9_]*::[a-z_][a-z0-9_]*$")
KEBAB = re.compile(r"^[a-z0-9]+(-[a-z0-9]+)*$")


class RegistryError(ValueError):
    pass


def rule_package(root, name):
    """The directory of rule package `name` (card RULE35-PILOT: a migrated
    rule's crate is `crates/gateway-rules/<name>/`, its manifest's package
    name `name`), or None."""
    directory = root / RULE_PACKAGES / name
    manifest = directory / "Cargo.toml"
    if not manifest.is_file():
        return None
    with open(manifest, "rb") as handle:
        package = tomllib.load(handle).get("package", {})
    return directory if package.get("name") == name else None


def load(root=ROOT):
    with open(root / REGISTRY, "rb") as handle:
        return tomllib.load(handle)


def validate(registry, root=ROOT):
    """The registry's rules, checked; raises RegistryError on the first fault."""
    def fail(text):
        raise RegistryError(f"{REGISTRY}: {text}")

    if set(registry) - {"schema", "route_map", "rule"}:
        fail(f"unknown top-level keys {sorted(set(registry) - {'schema', 'route_map', 'rule'})}")
    if registry.get("schema") != SCHEMA:
        fail(f"schema must be {SCHEMA!r}")
    if not (root / registry.get("route_map", "")).is_file():
        fail(f"route_map {registry.get('route_map')!r} is not a file")
    rules = registry.get("rule")
    if not isinstance(rules, list) or not rules:
        fail("no [[rule]] rows")
    ids, keys = set(), set()
    orders = {}
    arms = set()
    for rule in rules:
        rule_id = rule.get("id")
        where = f"rule {rule_id!r}"
        if set(rule) - RULE_KEYS:
            fail(f"{where}: unknown keys {sorted(set(rule) - RULE_KEYS)}")
        for key in ("id", "crate", "lean", "document", "off", "placements"):
            if key not in rule:
                fail(f"{where}: missing {key}")
        if not isinstance(rule_id, str) or not PLAIN_ID.match(rule_id):
            fail(f"{where}: not a plain id")
        if rule_id in ids:
            fail(f"{where}: registered twice")
        ids.add(rule_id)
        if not isinstance(rule["crate"], str) or not rule["crate"]:
            fail(f"{where}: crate must be a crate name")
        package = rule_package(root, rule["crate"]) if rule["crate"] != GATEWAY else None
        if rule["crate"] != GATEWAY and package is None:
            fail(f"{where}: crate {rule['crate']} is neither {GATEWAY} nor a rule package "
                 f"{RULE_PACKAGES}/{rule['crate']}/ whose manifest names it")
        if rule["off"] not in OFF:
            fail(f"{where}: off must be one of {sorted(OFF)}")
        if not (root / rule["document"]).is_file():
            fail(f"{where}: rule document {rule['document']} is not a file")
        lean = rule["lean"]
        if not isinstance(lean, list):
            fail(f"{where}: lean must be a list")
        for module in lean:
            parts = module.split(".") if isinstance(module, str) else []
            if len(parts) < 2 or parts[0] != "QCore" or not all(p.isidentifier() for p in parts):
                fail(f"{where}: Lean module {module!r} is not a QCore module")
            if not (root / "proof" / Path(*parts).with_suffix(".lean")).is_file():
                fail(f"{where}: Lean module {module} has no proof/{'/'.join(parts)}.lean")
        for reserved in rule.get("reason_keys", []):
            if set(reserved) != {"key", "category"} or not KEBAB.match(reserved["key"]) \
                    or not KEBAB.match(reserved["category"]):
                fail(f"{where}: a reason key is {{key, category}}, both kebab-case")
            if reserved["key"] in keys:
                fail(f"{where}: reason key {reserved['key']} is reserved twice")
            keys.add(reserved["key"])
        placements = rule["placements"]
        if not isinstance(placements, list) or not placements:
            fail(f"{where}: no placement")
        for placement in placements:
            if set(placement) - PLACEMENT_KEYS:
                fail(f"{where}: unknown placement keys {sorted(set(placement) - PLACEMENT_KEYS)}")
            position = placement.get("position")
            if position not in POSITIONS:
                fail(f"{where}: unknown position {position!r}")
            if position in HOLDS and rule["off"] != "not-switchable":
                fail(f"{where}: a package at the hold position {position} must be "
                     "off = \"not-switchable\" (its refusal would widen)")
            has_order, has_arm = "order" in placement, "arm" in placement
            if has_order == has_arm:
                fail(f"{where} at {position}: exactly one of order and arm")
            route = placement.get("route")
            if has_order:
                if position not in TABLES:
                    fail(f"{where}: {position} has no generated table yet; the card that "
                         "migrates its first route adds one")
                order = placement["order"]
                if not isinstance(order, int) or isinstance(order, bool) or order < 1:
                    fail(f"{where} at {position}: order must be a positive integer")
                if not isinstance(route, str) or not IDENTIFIER.match(route):
                    fail(f"{where} at {position}: route must be a parser method name")
                orders.setdefault(position, []).append(order)
                carries = TABLES[position][3]
                if carries != ("clears_origins" in placement):
                    fail(f"{where} at {position}: clears_origins is "
                         + ("required" if carries else "not a field of this table"))
                if carries and not isinstance(placement["clears_origins"], bool):
                    fail(f"{where} at {position}: clears_origins must be a boolean")
            else:
                if "clears_origins" in placement:
                    fail(f"{where} at {position}: an arm has no clears_origins")
                if not isinstance(placement["arm"], str) or not KEBAB.match(placement["arm"]):
                    fail(f"{where} at {position}: arm must be a kebab-case name")
                if (position, placement["arm"]) in arms:
                    fail(f"{where}: arm {placement['arm']} at {position} is registered twice")
                arms.add((position, placement["arm"]))
                if not isinstance(route, str) or not ARM_ROUTE.match(route):
                    fail(f"{where} at {position}: an arm's route is `module::function`")
                if package is not None:
                    module, function = route.split("::")
                    source = package / "src" / f"{module}.rs"
                    if not source.is_file() or not re.search(
                            rf"^pub fn {function}\(", source.read_text(encoding="utf-8"), re.M):
                        fail(f"{where} at {position}: route {route} is not a `pub fn` of "
                             f"{source.relative_to(root)}")
    for position, listed in orders.items():
        if sorted(listed) != list(range(1, len(listed) + 1)):
            fail(f"{position}: consultation orders {sorted(listed)} are not 1..{len(listed)}")
    return rules


def ordered(rules, position):
    """(rule, placement) of `position`'s table, by order."""
    rows = [(rule, placement) for rule in rules for placement in rule["placements"]
            if placement["position"] == position and "order" in placement]
    return sorted(rows, key=lambda row: row[1]["order"])


def rust_string(text):
    return '"' + text.replace("\\", "\\\\").replace('"', '\\"') + '"'


def camel(kebab):
    return "".join(part.capitalize() for part in kebab.split("-"))


def render_registry(rules):
    uses = ["Off", "Placement", "Position", "RuleEntry"]
    if any(rule.get("reason_keys") for rule in rules):
        uses.insert(3, "ReasonKey")
    lines = [
        "// SPDX-License-Identifier: Apache-2.0",
        "//",
        "// Licensed under the Apache License, Version 2.0 (the \"License\");",
        "// you may not use this file except in compliance with the License.",
        "// You may obtain a copy of the License at",
        "//",
        "//     http://www.apache.org/licenses/LICENSE-2.0",
        "//",
        "// Unless required by applicable law or agreed to in writing, software",
        "// distributed under the License is distributed on an \"AS IS\" BASIS,",
        "// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.",
        "// See the License for the specific language governing permissions and",
        "// limitations under the License.",
        "",
        "// Generated by `python3 scripts/gen_gateway_registry.py` from",
        "// `crates/gateway-rules/registry.toml`; do not edit by hand.",
        "",
        "//! The gateway rule registry (card GATEWAY-RULE-API), one row per rule",
        "//! package in the registry file's order.",
        "",
        "#[rustfmt::skip]",
        f"use qcore_gateway_rule_api::{{{', '.join(uses)}}};",
    ]
    if "ReasonKey" in uses:
        lines += ["#[rustfmt::skip]", "use qcore_schemas::reason::ReasonCategory;"]
    lines += ["", "/// Every registered rule.", "#[rustfmt::skip]",
              "pub const REGISTRY: &[RuleEntry] = &["]
    for rule in rules:
        lines += ["    RuleEntry {",
                  f"        id: {rust_string(rule['id'])},",
                  f"        crate_name: {rust_string(rule['crate'])},",
                  "        placements: &["]
        for placement in rule["placements"]:
            order = f"Some({placement['order']})" if "order" in placement else "None"
            arm = f"Some({rust_string(placement['arm'])})" if "arm" in placement else "None"
            lines.append(
                f"            Placement {{ position: Position::{POSITIONS[placement['position']]}, "
                f"order: {order}, arm: {arm}, route: {rust_string(placement['route'])} }},")
        lean = ", ".join(rust_string(module) for module in rule["lean"])
        keys = ", ".join(
            f"ReasonKey {{ key: {rust_string(reserved['key'])}, "
            f"category: ReasonCategory::{camel(reserved['category'])} }}"
            for reserved in rule.get("reason_keys", []))
        lines += ["        ],",
                  f"        lean: &[{lean}],",
                  f"        document: {rust_string(rule['document'])},",
                  f"        reason_keys: &[{keys}],",
                  f"        off: Off::{OFF[rule['off']]},",
                  "    },"]
    lines += ["];", ""]
    return "\n".join(lines)


def render_region(rules):
    lines = [START + " (from crates/gateway-rules/registry.toml by",
             "// `python3 scripts/gen_gateway_registry.py`; do not edit by hand)"]
    for position, (constant, row, module, carries, doc) in TABLES.items():
        lines += ["", f"/// {doc}", "#[rustfmt::skip]",
                  f"pub(super) const {constant}: &[{row}] = &["]
        for rule, placement in ordered(rules, position):
            clears = (f" clears_origins: {'true' if placement['clears_origins'] else 'false'},"
                      if carries else "")
            lines.append(f"    {row} {{ rule: {rust_string(rule['id'])}, "
                         f"route: {rust_string(placement['route'])},{clears} "
                         f"ask: {module}::{placement['route']} }},")
        lines.append("];")
    lines += ["", END]
    return "\n".join(lines)


def splice(text, region):
    """`text` with its marked region replaced by `region`; fails closed when
    the markers are missing or repeated."""
    if text.count(START) != 1 or text.count(END) != 1:
        raise RegistryError(f"{TABLE_RS}: expected exactly one {START!r} and one {END!r}")
    start = text.index(START)
    end = text.index(END) + len(END)
    if end < start:
        raise RegistryError(f"{TABLE_RS}: {END!r} precedes {START!r}")
    return text[:start] + region + text[end:]


def render(root=ROOT):
    """Map each generated path to its text."""
    rules = validate(load(root), root)
    table = (root / TABLE_RS).read_text(encoding="utf-8")
    return {
        REGISTRY_RS: render_registry(rules),
        TABLE_RS: splice(table, render_region(rules)),
    }


def stale(root=ROOT):
    """Generated paths whose committed text differs from the generated text."""
    return [str(path) for path, text in render(root).items()
            if not (root / path).is_file() or (root / path).read_text(encoding="utf-8") != text]


# --- The frozen route map's tables (ROUTE-MAP.md), for the order test. ---

CELL = re.compile(r"(?<!\\)\|")


def route_map_tables(text):
    """The statement fallback rows (§2.1 rows 7-21) and the return fallbacks
    (§4 step 3) of the route map, as `{position: [(rule, route), ...]}`."""
    if "Rows 7-21 are the statement fallback table" not in text:
        raise RegistryError("ROUTE-MAP.md no longer names rows 7-21 as the statement fallbacks")
    statement = {}
    for line in text.splitlines():
        cells = CELL.split(line)
        if len(cells) < 4 or not cells[1].strip().isdigit():
            continue
        number = int(cells[1].strip())
        if 7 <= number <= 21:
            routes = re.findall(r"`([a-z_][a-z0-9_]*)`", cells[2])
            rule = re.split(r"[;,( ]", cells[3].strip())[0]
            if not routes or not rule:
                raise RegistryError(f"ROUTE-MAP.md row {number} names no route or rule")
            if number in statement:
                raise RegistryError(f"ROUTE-MAP.md row {number} appears twice")
            statement[number] = (rule, routes[-1])
    if sorted(statement) != list(range(7, 22)):
        raise RegistryError(f"ROUTE-MAP.md statement rows {sorted(statement)} are not 7..21")
    flat = re.sub(r"\s+", " ", text)
    start = flat.find("3. Back in `parse_statement`, the return fallbacks")
    end = flat.find("These eight are the return fallback table", max(start, 0))
    if start < 0 or end < 0:
        raise RegistryError("ROUTE-MAP.md no longer holds §4 step 3's return fallback list")
    returns = re.findall(r"(\d+[a-z]) `([a-z_][a-z0-9_]*)`", flat[start:end])
    if len(returns) != 8:
        raise RegistryError(f"ROUTE-MAP.md §4 step 3 names {len(returns)} return fallbacks, not 8")
    return {
        "statement-fallback": [statement[number] for number in range(7, 22)],
        "return-fallback": returns,
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__,
                                     formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--check", action="store_true",
                        help="fail if a generated file differs from the committed one")
    args = parser.parse_args()
    if args.check:
        bad = stale()
        if bad:
            raise SystemExit(f"stale gateway rule tables {', '.join(bad)}; "
                             "run python3 scripts/gen_gateway_registry.py")
        print(f"gateway rule tables match {len(validate(load()))} registry rules")
        return
    for path, text in render().items():
        (ROOT / path).write_text(text, encoding="utf-8")
        print(f"Wrote {path}")


if __name__ == "__main__":
    main()
