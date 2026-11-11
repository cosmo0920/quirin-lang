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

"""Per-rule contribution ledgers (pluggable-cards seam A, card
CONTRIBUTION-LEDGERS).

This script is the single authority for the ledger schema
`quirin-contribution-ledger/1`, the ledgers' place in the repository, the
profile consistency check and the generated contributions page.

**What a ledger is.** One rule's measured contribution to one live evaluation
profile: the function rows its run-time switch moves, the leaves it trades,
the rows it touches that stay Unknown, measured from **one binary** as TIP
against TIP with rules off (`QCORE_GATEWAY_RULES_OFF`, seam B), never as two
revisions, so a sibling's landing does not move it. `quirin-audit measure
--rules-off IDS [--per-rule]` builds the ledger bodies into the
native-comparison record's `contribution` slot
(`crates/quirin-audit-plugin-measure/src/ledger.rs` states the attribution);
`write` below copies them into the repository and stamps their provenance.
Rows two rules move together are **interaction** ledgers, never attributed to
either rule alone (plan §7).

**Layout.** `contract/contributions/<rule-id>/<profile>.json` for a rule
(`<rule-id>` is the id of `docs/agents/rules/<rule-id>.md`, so the gateway
rule registry can reference a rule's directory), and
`contract/contributions/interactions/<id>+<id>[+...]/<profile>.json` for an
interaction (ids in rule order). Nothing else lives under
`contract/contributions/`. (Not under `contract/profiles/`: a directory there
without `profile.json` empties the first-party provider registry; card
EVALUATION-PROFILES.)

**Schema.** A committed ledger is one JSON object with exactly these keys:

- `schema`: `quirin-contribution-ledger/1`.
- `kind`: `rule` or `interaction`; `key`: the rule id, or the interaction's
  ids joined by `+` in rule order; `rules`: `[key]` for a rule, the two or
  more ids for an interaction.
- `profile`, `route`: the live evaluation profile and the measured route.
- `source`: `measured-by-switch`. (No other source is accepted. The pilot
  rules' hand-written report deltas were not transcribed; see the card
  report.)
- `measurement`: `{tool, tip_side, comparison, binary_sha256, revision,
  build_commit, bundles, source_date_epoch}`: the one binary both sides ran
  (`binary_sha256`, 64 hex), its revision or build commit, its contract
  bundle digests, and the comparison (`<off side>-><tip side>`) the rows come
  from.
- `group`: the measurement's switch group, identical in every ledger it
  produced for this profile: `rules` (every switched id), `per_rule`,
  `interactions` (the interaction keys), `joint_comparison`, `base` (the
  status totals with every switched rule off), `on` (the TIP totals),
  `tip_figures` (the TIP side's `evaluation_profiles.py summarize` figures),
  `interactions_measured` (false when one rule was switched alone: its
  interactions with other rules were not measured).
- `totals`: `{off, on}` status totals of the ledger's own comparison.
- `delta`: the non-zero status changes of `rows_moved` (for an interaction,
  of each row's `joint` move); re-derived from the rows by `check`.
- `rows_moved`: `[{row, before, after, premises, leaves_removed,
  leaves_added}]`; an interaction row adds `joint` (`{before, after}` or
  null) and `per_rule` (`{id: {before, after}}`). `premises` is
  `{"declarations": {declaration: [premise names]}}`, or carries
  `unresolved` when the route wrote no contract sidecar.
- `premises`: the distinct premise names of `rows_moved`, or null when a
  row's premises are unresolved.
- `leaves_traded`: `{removed, added, rows}`: the content-changed rows (status
  unchanged) and the reason leaves the switch removes and adds on them.
- `rows_unknown`: `[{row, why}]`: content-changed rows that stay Unknown,
  `why` their remaining TIP-side leaves.
- `record`: `{path, sha256}`, the committed native-comparison record the
  ledger was written from (repository-relative) and its digest.
- `profile_baseline`: `{sha256, measured_revision}`, the profile baseline
  record whose figures equal `group.tip_figures` when the ledger was written.

The ledger without `record` and `profile_baseline` is byte-for-byte (as JSON)
an entry of the record's `contribution.ledgers`.

**Checks.** `check` (fail-closed, every committed ledger): the layout, the
schema, `delta` re-derived from the rows, the record's digest, the ledger
equal to its record entry, every rule id known (an entry of
`crates/gateway-rules/registry.toml` once that registry exists; until then a
rule document `docs/agents/rules/<id>.md` with a router `summary`), the
profile live, and every group complete (each ledger the group lists is
committed) with **base plus its ledgers' deltas equal to on**.

`consistency`: for each live profile, a group is **current** when its
`profile_baseline.sha256` is the profile's baseline digest now. For each
current group the profile's real-run total (the baseline's
`figures.row_statuses`, re-derived from a real run at every landing, never
summed) must equal the group's `on` and the group's base plus its ordered rule
contributions plus its named interactions. Two current groups sharing a rule
or a moved row are an unnamed interaction (measure the rules together with
`--per-rule`). A disagreement is reported and exits 1; nothing is repaired. A
group measured on an older baseline is listed as stale (re-measure it) and is
not counted.

`render` writes `docs/development/contributions.md` (linked from
`docs/STATUS.md`) from the ledgers and the profiles; `render --check` and the
Python suite require the committed page to equal the generated one.

    contribution_ledgers.py write --record R [--root DIR]
    contribution_ledgers.py validate-record R
    contribution_ledgers.py check [--root DIR]
    contribution_ledgers.py consistency [--root DIR]
    contribution_ledgers.py render [--check] [--root DIR]

The amendments field `contribution` (`scripts/apply_amendments.py`) names the
ledgers a card commits and is checked with `check_contribution`.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import re
import sys
from pathlib import Path, PurePosixPath

SCHEMA = "quirin-contribution-ledger/1"
SOURCE = "measured-by-switch"
ROOT = Path(__file__).resolve().parents[1]
CONTRIBUTIONS = PurePosixPath("contract/contributions")
INTERACTIONS = "interactions"
EVALUATIONS = PurePosixPath("contract/evaluations")
RULES = PurePosixPath("docs/agents/rules")
REGISTRY = PurePosixPath("crates/gateway-rules/registry.toml")
PAGE = PurePosixPath("docs/development/contributions.md")
RULE_ID_RE = re.compile(r"[0-9][0-9a-z-]*")
PROFILE_RE = re.compile(r"[a-z0-9][a-z0-9-]*")
SHA_RE = re.compile(r"[0-9a-f]{64}")
BODY_KEYS = ("schema", "kind", "key", "rules", "profile", "route", "source", "measurement",
             "group", "totals", "delta", "rows_moved", "premises", "leaves_traded",
             "rows_unknown")
STAMP_KEYS = ("record", "profile_baseline")
MEASUREMENT_KEYS = ("tool", "tip_side", "comparison", "binary_sha256", "revision",
                    "build_commit", "bundles", "source_date_epoch")
GROUP_KEYS = ("rules", "per_rule", "interactions", "joint_comparison", "base", "on",
              "tip_figures", "interactions_measured")
NUMBERS = ("no", "one", "two", "three", "four", "five", "six", "seven", "eight", "nine", "ten")


class LedgerError(Exception):
    """A refusal; nothing has been written when it is raised."""


def sha256(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def rule_order(rule: str):
    """Numeric prefix, then the rest (`32 < 32a < 33 < 100`); the measure
    plugin's `ledger::rule_order`."""
    digits = re.match(r"[0-9]*", rule).group(0)
    return (int(digits) if digits else sys.maxsize, rule[len(digits):])


def interaction_key(rules) -> str:
    return "+".join(sorted(rules, key=rule_order))


def ledger_path(ledger) -> PurePosixPath:
    if ledger["kind"] == "rule":
        return CONTRIBUTIONS / ledger["key"] / f"{ledger['profile']}.json"
    return CONTRIBUTIONS / INTERACTIONS / ledger["key"] / f"{ledger['profile']}.json"


def body(ledger) -> dict:
    """The ledger as its record's slot holds it (without the stamps)."""
    return {key: value for key, value in ledger.items() if key not in STAMP_KEYS}


def dump(value) -> str:
    return json.dumps(value, indent=1, sort_keys=True, ensure_ascii=False) + "\n"


# --------------------------------------------------------------------------
# schema


def _require(condition, message):
    if not condition:
        raise LedgerError(message)


def _counts(value, where):
    _require(isinstance(value, dict) and all(
        isinstance(k, str) and isinstance(v, int) and not isinstance(v, bool)
        for k, v in value.items()), f"{where} must map statuses to integers")
    return value


def _add(totals, delta):
    out = dict(totals)
    for status, count in delta.items():
        out[status] = out.get(status, 0) + count
    return {status: count for status, count in out.items() if count}


def _nonzero(totals):
    return {status: count for status, count in totals.items() if count}


def _move(delta, before, after):
    if before == after:
        return
    if before != "absent":
        delta[before] = delta.get(before, 0) - 1
    if after != "absent":
        delta[after] = delta.get(after, 0) + 1


def derived_delta(ledger):
    delta = {}
    for row in ledger["rows_moved"]:
        if ledger["kind"] == "rule":
            _move(delta, row["before"], row["after"])
        elif row.get("joint"):
            _move(delta, row["joint"]["before"], row["joint"]["after"])
    return _nonzero(delta)


def validate_body(ledger, where, *, stamped=False):
    """Check one ledger against the schema (module doc); raise on a defect."""
    _require(isinstance(ledger, dict), f"{where}: a ledger is a JSON object")
    expected = set(BODY_KEYS) | (set(STAMP_KEYS) if stamped else set())
    _require(set(ledger) == expected,
             f"{where}: keys {sorted(ledger)}; {SCHEMA} takes exactly {sorted(expected)}")
    _require(ledger["schema"] == SCHEMA, f"{where}: schema {ledger['schema']!r}")
    _require(ledger["kind"] in ("rule", "interaction"), f"{where}: kind {ledger['kind']!r}")
    rules = ledger["rules"]
    _require(isinstance(rules, list) and rules and all(
        isinstance(r, str) and RULE_ID_RE.fullmatch(r) for r in rules),
        f"{where}: rules must be rule ids")
    _require(len(set(rules)) == len(rules), f"{where}: rules repeats an id")
    if ledger["kind"] == "rule":
        _require(rules == [ledger["key"]], f"{where}: a rule ledger's rules are [key]")
    else:
        _require(len(rules) >= 2 and ledger["key"] == interaction_key(rules)
                 and rules == sorted(rules, key=rule_order),
                 f"{where}: an interaction's key is its two or more ids in rule order")
    for field in ("profile", "route"):
        _require(isinstance(ledger[field], str) and ledger[field], f"{where}: {field}")
    _require(PROFILE_RE.fullmatch(ledger["profile"]), f"{where}: profile name")
    _require(ledger["source"] == SOURCE,
             f"{where}: source {ledger['source']!r}; only {SOURCE!r} is accepted")
    measurement = ledger["measurement"]
    _require(isinstance(measurement, dict) and set(measurement) == set(MEASUREMENT_KEYS),
             f"{where}: measurement takes exactly {list(MEASUREMENT_KEYS)}")
    _require(isinstance(measurement["binary_sha256"], str)
             and SHA_RE.fullmatch(measurement["binary_sha256"]),
             f"{where}: measurement.binary_sha256 must be the binary's SHA-256")
    _require(isinstance(measurement["comparison"], str) and "->" in measurement["comparison"],
             f"{where}: measurement.comparison")
    group = ledger["group"]
    _require(isinstance(group, dict) and set(group) == set(GROUP_KEYS),
             f"{where}: group takes exactly {list(GROUP_KEYS)}")
    _require(isinstance(group["rules"], list) and set(rules) <= set(group["rules"]),
             f"{where}: group.rules must hold the ledger's rules")
    _require(isinstance(group["interactions"], list), f"{where}: group.interactions")
    separated = len(group["rules"]) == 1 or group["per_rule"] is True
    if ledger["kind"] == "rule":
        _require(separated, f"{where}: rules switched together without --per-rule have no "
                            "rule ledger")
    else:
        _require(ledger["key"] in group["interactions"],
                 f"{where}: the group does not list interaction {ledger['key']}")
    _counts(group["base"], f"{where}: group.base")
    _counts(group["on"], f"{where}: group.on")
    totals = ledger["totals"]
    _require(isinstance(totals, dict) and set(totals) == {"off", "on"}, f"{where}: totals")
    _counts(totals["off"], f"{where}: totals.off")
    _counts(totals["on"], f"{where}: totals.on")
    _counts(ledger["delta"], f"{where}: delta")
    rows = ledger["rows_moved"]
    _require(isinstance(rows, list) and all(
        isinstance(r, dict) and isinstance(r.get("row"), str) and isinstance(r.get("before"), str)
        and isinstance(r.get("after"), str) for r in rows), f"{where}: rows_moved")
    _require(len({r["row"] for r in rows}) == len(rows), f"{where}: rows_moved repeats a row")
    _require(ledger["delta"] == derived_delta(ledger),
             f"{where}: delta {ledger['delta']} is not the rows' {derived_delta(ledger)}")
    _require(ledger["premises"] is None or (isinstance(ledger["premises"], list) and all(
        isinstance(p, str) for p in ledger["premises"])), f"{where}: premises")
    traded = ledger["leaves_traded"]
    _require(isinstance(traded, dict) and set(traded) == {"removed", "added", "rows"},
             f"{where}: leaves_traded")
    _require(isinstance(ledger["rows_unknown"], list), f"{where}: rows_unknown")
    if stamped:
        record = ledger["record"]
        _require(isinstance(record, dict) and set(record) == {"path", "sha256"}
                 and isinstance(record["path"], str) and SHA_RE.fullmatch(str(record["sha256"])),
                 f"{where}: record must be {{path, sha256}}")
        baseline = ledger["profile_baseline"]
        _require(isinstance(baseline, dict) and set(baseline) == {"sha256", "measured_revision"}
                 and SHA_RE.fullmatch(str(baseline["sha256"])),
                 f"{where}: profile_baseline must be {{sha256, measured_revision}}")


def groups_of(ledgers):
    """{(profile, joint comparison, binary): [ledger, ...]}"""
    groups = {}
    for ledger in ledgers:
        key = (ledger["profile"], ledger["group"]["joint_comparison"],
               ledger["measurement"]["binary_sha256"])
        groups.setdefault(key, []).append(ledger)
    return groups


def check_group(ledgers, where):
    """Every ledger of one group: identical group, complete, and adding up."""
    group = ledgers[0]["group"]
    for ledger in ledgers[1:]:
        _require(ledger["group"] == group, f"{where}: ledgers of one measurement disagree "
                                           "on their group")
    separated = len(group["rules"]) == 1 or group["per_rule"] is True
    expected = {("interaction", key) for key in group["interactions"]}
    if separated:
        expected |= {("rule", rule) for rule in group["rules"]}
    present = [(ledger["kind"], ledger["key"]) for ledger in ledgers]
    _require(len(set(present)) == len(present), f"{where}: a ledger appears twice")
    _require(set(present) == expected,
             f"{where}: the group lists {sorted(expected)}, the ledgers are {sorted(present)}")
    rows = [row["row"] for ledger in ledgers for row in ledger["rows_moved"]]
    _require(len(set(rows)) == len(rows), f"{where}: a row is in two ledgers of one group")
    derived = dict(group["base"])
    for ledger in ledgers:
        derived = _add(derived, ledger["delta"])
    _require(_nonzero(derived) == _nonzero(group["on"]),
             f"{where}: base {group['base']} plus the ledgers' deltas is {derived}, not the "
             f"TIP totals {group['on']} (reported, never repaired)")


def validate_record(record, where="record"):
    """The `contribution` slot of a native-comparison record: every ledger
    body valid, every group complete and adding up. Returns the ledgers."""
    _require(isinstance(record, dict), f"{where}: not a JSON object")
    slot = record.get("contribution")
    _require(isinstance(slot, dict) and slot.get("schema") == SCHEMA,
             f"{where}: the contribution slot is not {SCHEMA}")
    _require("refused" not in slot, f"{where}: the slot was refused: {slot.get('refused')}")
    ledgers = slot.get("ledgers")
    _require(isinstance(ledgers, list), f"{where}: contribution.ledgers")
    for index, ledger in enumerate(ledgers):
        validate_body(ledger, f"{where}: contribution.ledgers[{index}]")
    for key, members in groups_of(ledgers).items():
        check_group(members, f"{where}: group {key[1]} on {key[0]}")
    return ledgers


# --------------------------------------------------------------------------
# the repository


def live_profile(root, name):
    """(evaluation record, baseline record) of a live profile, or raise."""
    path = Path(root) / EVALUATIONS / name / "evaluation.json"
    _require(path.is_file(), f"{name} is not an evaluation profile ({path} is missing)")
    record = json.loads(path.read_bytes())
    _require(record.get("policy", {}).get("kind") == "live",
             f"{name} is not a live profile; ledgers are per rule and live profile")
    baseline_path = Path(root) / record["baseline"]["path"]
    data = baseline_path.read_bytes()
    _require(sha256(data) == record["baseline"]["sha256"],
             f"{name}: {record['baseline']['path']} does not have its recorded digest")
    return record, json.loads(data)


def live_profiles(root):
    names = []
    for path in sorted((Path(root) / EVALUATIONS).glob("*/evaluation.json")):
        record = json.loads(path.read_bytes())
        if record.get("policy", {}).get("kind") == "live":
            names.append(path.parent.name)
    return names


def registry_ids(root):
    """The rule ids of `crates/gateway-rules/registry.toml` in file order, or
    None when the registry does not exist. Any table carrying a string `id`
    is a row (GATEWAY-RULE-API owns the format)."""
    path = Path(root) / REGISTRY
    if not path.is_file():
        return None
    import tomllib
    try:
        data = tomllib.loads(path.read_text(encoding="utf-8"))
    except tomllib.TOMLDecodeError as error:
        raise LedgerError(f"{REGISTRY} is not TOML ({error})") from error
    ids = []

    def walk(value):
        if isinstance(value, dict):
            if isinstance(value.get("id"), str):
                ids.append(value["id"])
            for item in value.values():
                walk(item)
        elif isinstance(value, list):
            for item in value:
                walk(item)
    walk(data)
    return ids


def known_rule(root, rule):
    """Refuse a rule id the registry (once it exists) or the rule documents
    do not define."""
    ids = registry_ids(root)
    if ids is not None:
        _require(rule in ids, f"rule {rule} is not in {REGISTRY}")
        return
    path = Path(root) / RULES / f"{rule}.md"
    _require(RULE_ID_RE.fullmatch(rule) and path.is_file(),
             f"rule {rule} has no rule document {RULES}/{rule}.md")
    text = path.read_text(encoding="utf-8")
    front = text.split("---", 2)[1] if text.startswith("---") else ""
    _require(re.search(rf"^rule: {re.escape(rule)}$", front, re.M)
             and re.search(r"^summary: ", front, re.M),
             f"{RULES}/{rule}.md is not a rule with a router row (front matter rule and summary)")


def rule_rank(root):
    ids = registry_ids(root)
    if ids is None:
        return rule_order
    return lambda rule: (ids.index(rule), "") if rule in ids else (len(ids), rule)


def read_ledger_file(root, relative):
    """Load, validate and verify one committed ledger against its record."""
    relative = PurePosixPath(relative)
    path = Path(root) / relative
    _require(path.is_file(), f"{relative}: no such ledger")
    ledger = json.loads(path.read_bytes())
    validate_body(ledger, str(relative), stamped=True)
    _require(ledger_path(ledger) == relative,
             f"{relative}: a {ledger['kind']} ledger for {ledger['key']} on {ledger['profile']} "
             f"belongs at {ledger_path(ledger)}")
    record_path = Path(root) / ledger["record"]["path"]
    _require(record_path.is_file(), f"{relative}: its record {ledger['record']['path']} is missing")
    data = record_path.read_bytes()
    _require(sha256(data) == ledger["record"]["sha256"],
             f"{relative}: {ledger['record']['path']} does not match the ledger's record digest")
    slot = validate_record(json.loads(data), ledger["record"]["path"])
    _require(body(ledger) in slot,
             f"{relative}: the ledger is not an entry of its record's contribution slot")
    for rule in ledger["rules"]:
        known_rule(root, rule)
    live_profile(root, ledger["profile"])
    return ledger


def ledger_files(root):
    base = Path(root) / CONTRIBUTIONS
    if not base.is_dir():
        return []
    files = sorted(path.relative_to(root).as_posix() for path in base.rglob("*") if path.is_file())
    for relative in files:
        parts = PurePosixPath(relative).parts[2:]
        well_placed = (len(parts) == 2 and RULE_ID_RE.fullmatch(parts[0])) or (
            len(parts) == 3 and parts[0] == INTERACTIONS)
        _require(well_placed and parts[-1].endswith(".json"),
                 f"{relative}: only ledgers live under {CONTRIBUTIONS} "
                 "(<rule-id>/<profile>.json, interactions/<key>/<profile>.json)")
    return files


def load_all(root):
    return [read_ledger_file(root, relative) for relative in ledger_files(root)]


def check(root):
    """Every committed ledger (module doc); returns the ledgers."""
    ledgers = load_all(root)
    for key, members in groups_of(ledgers).items():
        check_group(members, f"group {key[1]} on {key[0]}")
    return ledgers


# --------------------------------------------------------------------------
# consistency and the page


def _statuses(counts):
    return ", ".join(f"{status} {counts[status]}" for status in sorted(counts)) or "none"


def _number(n):
    return NUMBERS[n] if 0 <= n < len(NUMBERS) else str(n)


def _moves(ledger):
    tally = {}
    for row in ledger["rows_moved"]:
        move = row.get("joint") if ledger["kind"] == "interaction" else row
        if move:
            label = f"{move['before']}→{move['after']}"
            tally[label] = tally.get(label, 0) + 1
    return ", ".join(f"{label} {count}" for label, count in sorted(tally.items()))


def summary_line(ledger):
    rows = len(ledger["rows_moved"])
    noun = "row" if rows == 1 else "rows"
    name = (f"rule {ledger['key']}" if ledger["kind"] == "rule"
            else f"interaction {ledger['key']}")
    if ledger["premises"] is None:
        premises = " (premises unresolved)"
    elif ledger["premises"]:
        count = len(ledger["premises"])
        premises = (f" under {_number(count)} premise{'s' if count != 1 else ''} "
                    f"({', '.join(f'`{p}`' for p in ledger['premises'])})")
    else:
        premises = " under no named premise" if rows else ""
    moves = _moves(ledger)
    extra = []
    if moves:
        extra.append(moves)
    if ledger["rows_unknown"]:
        extra.append(f"{len(ledger['rows_unknown'])} touched rows stay Unknown")
    traded = ledger["leaves_traded"]
    if traded["removed"] or traded["added"]:
        extra.append(f"leaves traded −{traded['removed']} +{traded['added']}")
    tail = f"; {'; '.join(extra)}" if extra else ""
    return f"{name}: {_number(rows) if rows <= 10 else rows} {noun}{premises}{tail}"


def profile_report(root, name, ledgers):
    """(lines, errors) for one live profile."""
    record, baseline = live_profile(root, name)
    total = baseline["figures"].get("row_statuses", {})
    lines = [f"Real-run total (baseline `{record['baseline']['sha256'][:12]}`, measured at "
             f"`{baseline['measured_revision'][:12]}`): {_statuses(total)}."]
    errors = []
    mine = [ledger for ledger in ledgers if ledger["profile"] == name]
    if not mine:
        lines.append("")
        lines.append("No contribution ledger is recorded for this profile.")
        return lines, errors
    rank = rule_rank(root)
    current_rules, current_rows = {}, {}
    for key, members in sorted(groups_of(mine).items()):
        group = members[0]["group"]
        stamp = members[0]["profile_baseline"]
        revision = members[0]["measurement"]["revision"] or members[0]["measurement"]["build_commit"]
        current = stamp["sha256"] == record["baseline"]["sha256"]
        lines.append("")
        lines.append(f"Measurement `{members[0]['record']['sha256'][:12]}` "
                     f"({members[0]['record']['path']}), rules "
                     f"{', '.join(group['rules'])} switched off in one binary "
                     f"(`{str(revision)[:12]}`), {'current' if current else 'stale'}:")
        lines.append("")
        lines.append(f"- base (rules {', '.join(group['rules'])} off): {_statuses(group['base'])}")
        rules = sorted((m for m in members if m["kind"] == "rule"),
                       key=lambda m: rank(m["key"]))
        interactions = sorted((m for m in members if m["kind"] == "interaction"),
                              key=lambda m: [rank(r) for r in m["rules"]])
        for ledger in rules + interactions:
            lines.append(f"- {summary_line(ledger)}")
        if not group["interactions_measured"] and len(group["rules"]) == 1:
            lines.append(f"- interactions of rule {group['rules'][0]} with other rules: not "
                         "measured (switched alone)")
        if not current:
            lines.append(f"- stale: measured on baseline `{stamp['sha256'][:12]}`; the profile's "
                         "baseline has moved since. Not counted; re-measure with "
                         "`quirin-audit measure --rules-off`.")
            continue
        derived = dict(group["base"])
        for ledger in rules + interactions:
            derived = _add(derived, ledger["delta"])
        problems = []
        if _nonzero(group["on"]) != _nonzero(total):
            problems.append(f"the measured TIP totals {_statuses(group['on'])} are not the "
                            f"real-run total")
        if group["tip_figures"] != baseline["figures"]:
            problems.append("the measured TIP figures are not the baseline's figures")
        if _nonzero(derived) != _nonzero(total):
            problems.append(f"base plus contributions plus interactions is {_statuses(derived)}")
        for rule in group["rules"]:
            if rule in current_rules:
                problems.append(f"rule {rule} is also in current measurement "
                                f"`{current_rules[rule][:12]}`: measure them together")
            current_rules[rule] = members[0]["record"]["sha256"]
        for ledger in members:
            for row in ledger["rows_moved"]:
                if row["row"] in current_rows:
                    problems.append(f"row {row['row']} is also moved in current measurement "
                                    f"`{current_rows[row['row']][:12]}`: an unnamed "
                                    "interaction; measure the rules together with --per-rule")
                current_rows[row["row"]] = members[0]["record"]["sha256"]
        if problems:
            lines.append("- consistency: FAILS: " + "; ".join(problems))
            errors.extend(f"{name}: {problem}" for problem in problems)
        else:
            lines.append(f"- consistency: base + contributions + interactions = "
                         f"{_statuses(derived)} = the real-run total")
    return lines, errors


def consistency(root):
    """({profile: lines}, errors) over every live profile."""
    ledgers = check(root)
    reports, errors = {}, []
    for ledger in ledgers:
        _require(ledger["profile"] in live_profiles(root),
                 f"{ledger_path(ledger)}: {ledger['profile']} is not a live profile")
    for name in live_profiles(root):
        lines, problems = profile_report(root, name, ledgers)
        reports[name] = lines
        errors.extend(problems)
    return reports, errors


def render(root):
    reports, _ = consistency(root)
    out = [
        "# Per-rule contributions to the live evaluation profiles",
        "",
        "<!-- Generated by scripts/contribution_ledgers.py render; do not edit. -->",
        "",
        "Each live evaluation profile's total comes from a real run (its baseline record "
        "under `contract/evaluations/`). Beside it, each rule's measured contribution, from "
        "the ledgers under `contract/contributions/`: rows moved by one binary with the rule "
        "switched off and on, with the named premises they rest on. Rows two rules move "
        "together are named interactions and belong to neither rule. The consistency line "
        "checks base + ordered contributions + interactions against the real-run total; it "
        "never adjusts the total. Schema, attribution and checks: "
        "[`scripts/contribution_ledgers.py`](../../scripts/contribution_ledgers.py).",
    ]
    for name, lines in reports.items():
        out += ["", f"## {name}", ""] + lines
    return "\n".join(out) + "\n"


# --------------------------------------------------------------------------
# writing ledgers from a record


def write(root, record_path):
    """Copy every ledger of a committed record's slot into the repository,
    stamped with the record digest and the profile baseline (module doc)."""
    root = Path(root).resolve()
    record_path = Path(record_path).resolve()
    try:
        relative = record_path.relative_to(root).as_posix()
    except ValueError as error:
        raise LedgerError(f"{record_path} is outside {root}: commit the record (for example "
                          "under docs/reports/<card>/) and write from that copy") from error
    data = record_path.read_bytes()
    record = json.loads(data)
    _require(record.get("status") == "complete",
             f"{relative}: the measurement is {record.get('status')!r}, not complete")
    ledgers = validate_record(record, relative)
    _require(ledgers, f"{relative}: the record's contribution slot holds no ledger "
                      "(measure with --rules-off)")
    written = []
    for ledger in ledgers:
        evaluation, baseline = live_profile(root, ledger["profile"])
        _require(ledger["group"]["tip_figures"] == baseline["figures"],
                 f"{ledger['profile']}: the measured TIP figures are not the profile's baseline "
                 "figures; record the TIP run as the baseline first (evaluation_profiles.py "
                 "record), or measure with --profile")
        for rule in ledger["rules"]:
            known_rule(root, rule)
        stamped = dict(ledger)
        stamped["record"] = {"path": relative, "sha256": sha256(data)}
        stamped["profile_baseline"] = {"sha256": evaluation["baseline"]["sha256"],
                                       "measured_revision": baseline["measured_revision"]}
        validate_body(stamped, str(ledger_path(ledger)), stamped=True)
        written.append((ledger_path(ledger), stamped))
    for path, stamped in written:
        target = root / path
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_text(dump(stamped), encoding="utf-8")
    return [str(path) for path, _ in written]


# --------------------------------------------------------------------------
# the amendments field


def check_contribution(root, contribution):
    """Check the amendments field `contribution`: null, or `{"ledgers":
    [path, ...]}` naming the committed ledgers the card adds or re-measures.
    Returns note lines (stale ledgers are noted, not refused)."""
    if contribution is None:
        return []
    _require(isinstance(contribution, dict) and set(contribution) == {"ledgers"},
             "contribution must be null or {\"ledgers\": [path, ...]}")
    paths = contribution["ledgers"]
    _require(isinstance(paths, list) and paths and all(isinstance(p, str) for p in paths)
             and len(set(paths)) == len(paths),
             "contribution.ledgers must be a non-empty list of distinct ledger paths")
    notes = []
    ledgers = []
    for path in paths:
        _require(PurePosixPath(path).parts[:2] == CONTRIBUTIONS.parts,
                 f"contribution.ledgers: {path} is not under {CONTRIBUTIONS}")
        ledger = read_ledger_file(root, path)
        ledgers.append(ledger)
        evaluation, _ = live_profile(root, ledger["profile"])
        if ledger["profile_baseline"]["sha256"] != evaluation["baseline"]["sha256"]:
            notes.append(f"{path}: stale against {ledger['profile']}'s baseline (measured on "
                         f"{ledger['profile_baseline']['sha256'][:12]}); consistency lists it "
                         "until it is re-measured")
        else:
            notes.append(f"{path}: {summary_line(ledger)}")
    listed = {PurePosixPath(p) for p in paths}
    for ledger in ledgers:
        for other in load_all(root):
            if (other["record"] == ledger["record"] and other["profile"] == ledger["profile"]
                    and ledger_path(other) not in listed):
                raise LedgerError(f"contribution.ledgers omits {ledger_path(other)}, another "
                                  f"ledger of the measurement {ledger['record']['path']}")
    for key, members in groups_of(ledgers).items():
        check_group(members, f"contribution: group {key[1]} on {key[0]}")
    return notes


# --------------------------------------------------------------------------
# command line


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("--root", type=Path, default=ROOT)
    commands = parser.add_subparsers(dest="command", required=True)
    write_parser = commands.add_parser("write", help="write a record's ledgers into the tree")
    write_parser.add_argument("--record", required=True, type=Path)
    validate_parser = commands.add_parser("validate-record",
                                          help="check a native-comparison record's slot")
    validate_parser.add_argument("record", type=Path)
    commands.add_parser("check", help="check every committed ledger")
    commands.add_parser("consistency", help="check every live profile's ledgers")
    render_parser = commands.add_parser("render", help="write the contributions page")
    render_parser.add_argument("--check", action="store_true",
                               help="require the committed page to equal the generated one")
    args = parser.parse_args(argv)
    root = args.root.resolve()
    try:
        if args.command == "write":
            for path in write(root, args.record):
                print(f"wrote {path}")
        elif args.command == "validate-record":
            ledgers = validate_record(json.loads(args.record.read_bytes()), str(args.record))
            for ledger in ledgers:
                print(f"{ledger['profile']}: {summary_line(ledger)}")
            print(f"contribution_ledgers: {args.record}: {len(ledgers)} ledgers valid")
        elif args.command == "check":
            ledgers = check(root)
            print(f"contribution_ledgers: {len(ledgers)} ledgers valid")
        elif args.command == "consistency":
            reports, errors = consistency(root)
            for name, lines in reports.items():
                print(f"## {name}")
                for line in lines:
                    print(line)
            if errors:
                for error in errors:
                    print(f"contribution_ledgers: inconsistent: {error}", file=sys.stderr)
                return 1
            print("contribution_ledgers: consistent")
        elif args.command == "render":
            text = render(root)
            page = root / PAGE
            if args.check:
                if not page.is_file() or page.read_text(encoding="utf-8") != text:
                    print(f"contribution_ledgers: {PAGE} differs from the generated page; run "
                          "python3 scripts/contribution_ledgers.py render", file=sys.stderr)
                    return 1
                print(f"contribution_ledgers: {PAGE} is current")
            else:
                page.parent.mkdir(parents=True, exist_ok=True)
                page.write_text(text, encoding="utf-8")
                print(f"wrote {PAGE}")
    except (LedgerError, OSError, json.JSONDecodeError, KeyError, TypeError) as error:
        print(f"contribution_ledgers: refused: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
