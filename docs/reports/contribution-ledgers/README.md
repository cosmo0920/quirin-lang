# CONTRIBUTION-LEDGERS: per-rule contribution ledgers

Pluggable-cards plan seam A, phase P2
([plan](../../plans/pluggable-cards/README.md) §3 A "Per-rule contribution
ledgers" and "Impact declaration", §3 F, §4 steps 3 and 5, §7). Base
`b2af78eaf` (main = origin/main); measured tip `b75be63fe`. The diff is the
measure plugin (`crates/quirin-audit-plugin-measure`), Python tooling and its
tests, two documentation paragraphs, one generated page and this report. It
changes no analyzer, contract, Lean module or verdict, and no live evaluation
baseline (`profiles: []`). No identity run is needed: no analysis input or
output changes.

## What changed

- **Ledger schema and checks.** New `scripts/contribution_ledgers.py` is the
  single authority for `quirin-contribution-ledger/1`. A ledger is one rule's
  measured contribution to one live profile:
  - rows moved, each with the named premises its TIP row rests on;
  - leaves traded on rows whose status does not move;
  - touched rows that stay Unknown, with their remaining leaves as the reason.

  Provenance has three parts:
  - the one binary both sides ran (`binary_sha256`, revision, contract bundle
    digests) and its comparison;
  - the committed native-comparison record and its digest (`record`);
  - the profile baseline whose figures equal the measured TIP figures
    (`profile_baseline`).
- **Layout.** A rule's ledger is `contract/contributions/<rule-id>/<profile>.json`.
  An interaction's ledger is `contract/contributions/interactions/<id>+<id>/<profile>.json`.
  - Rule ids are the `docs/agents/rules/<id>.md` ids, so the gateway rule
    registry can reference a rule's directory later. No crate directory is
    required now.
  - Nothing else may live under `contract/contributions/`.
  - It is not under `contract/profiles/`: the EVALUATION-PROFILES finding still
    holds there.
  - `scripts/gate_scope.py` classifies `contract/contributions/**` as records
    the Python suite checks, beside `contract/evaluations/**`. A ledger-only
    diff selects no Rust crate. `contract/contributions.json` and
    `contract/contributionsX/` still fail closed.
- **Production: the measure plugin fills its `contribution` slot.**
  `quirin-audit measure --side tip=. --rules-off <ids> [--per-rule]` now writes
  ledger bodies into the record (new `ledger.rs`; `compare::annotate` adds
  `premises` and `after_leaves` to the switched comparisons only).
  - **Premises.** Each premise is resolved the way rule 32's `premises.py` did
    it: from the row's `contract_dependencies.hypothesis_dependencies`, through
    the route's contract sidecar. Without a sidecar the premises are marked
    `unresolved`, never guessed.
  - **One rule switched alone** owns every row its switch moves. Its ledger
    says its interactions were not measured.
  - **`--per-rule`** with two or more ids also measures one side per id with
    only that rule off (`<tip>-off-<id>`). A row belongs to a rule only when
    three things hold: that rule's switch alone moves it, no other single
    switch moves it, and the joint switch moves it the same way.
  - **Every other moved row** is a named interaction (`33+35`) and is never
    attributed to either rule. This covers rows both rules are needed for,
    rows either rule suffices for, and rows whose single and joint moves
    differ. Without `--per-rule`, several rules are one interaction.
  - **Identity check.** The group's base (all switched rules off) plus every
    ledger's delta must equal the TIP totals. Otherwise the run fails and the
    slot is refused; nothing is repaired.
  - **Comparison interactions.** Rows of the per-rule comparisons are no longer
    listed as comparison `interactions`, because the slot attributes them.
  - **Other refusals.** Rule ids must be distinct. A side name may not collide
    with a switched side. Two routes on one profile are refused, because a
    ledger is per rule and profile.
- **Writing ledgers into the tree** is a converter:
  `contribution_ledgers.py write --record <committed record>`.
  - It refuses a record outside the repository, an incomplete measurement, a
    refused slot and a non-live profile.
  - It also refuses TIP figures that differ from the profile's baseline
    figures, so a ledger is always bound to the baseline its TIP run produced.
  - It stamps `record` and `profile_baseline` on each ledger.
  - The ledger minus those two stamps is, as JSON, an entry of the record's
    slot. `check` verifies that.
- **`check`** runs over every committed ledger and refuses any of these:
  - the layout or schema is wrong, or `delta` differs from the rows;
  - the record digest or the slot entry does not match;
  - a rule id is unknown;
  - the profile is not live;
  - a group is incomplete or does not add up.

  A rule id is known when it is an entry of
  `crates/gateway-rules/registry.toml`, once that registry exists. Until then
  it must have a rule document with a router `summary`, which excludes
  lead-ins such as `11-series`.
- **`consistency`** runs for each live profile:
  - A group is current when its stamped baseline digest is the profile's
    baseline digest now. Each current group's base, plus its ordered rule
    contributions, plus its named interactions, must equal the profile's
    real-run total: the baseline's `figures.row_statuses`, re-derived from a
    real run at landing and never summed. The measured TIP figures must also
    equal the baseline figures.
  - Two current groups that share a moved row form an unnamed interaction.
  - Any of these failures is reported and exits 1, with nothing repaired.
  - A group measured on an older baseline is listed as stale and not counted.
    Staleness is the expected state after a sibling moves the profile, and it
    does not fail anyone's gate.
- **Reporting.** `contribution_ledgers.py render` generates
  [`docs/development/contributions.md`](../../development/contributions.md)
  and is under a committed-equals-generated test. The page has one section per
  live profile:
  - the real-run total;
  - per measurement, the base and then lines such as "rule 33: one row under
    one premise (`rk_plugin_callback_closure`); Unknown→Safe 1";
  - the interaction lines and the consistency line.

  Contributions are ordered by the registry's order once it exists, and until
  then by rule id (`32 < 32a < 33`). STATUS.md is not hand-edited; this card's
  status paragraph links the page. `scripts/land.py` adds
  `contribution_ledgers.py render` to the landing sequence, and `land.py check`
  refuses a stale page.
- **Amendments.** The `contribution` field of `quirin-amendments/1` is narrowed
  to `null` or `{"ledgers": [path, ...]}` and applied:
  - Every run checks each listed ledger as `check` does.
  - The listing must include every other ledger of the same measurement and
    profile.
  - The tree must pass `consistency`.
  - A stale ledger is noted, not refused.
  - A write regenerates the contributions page when it differs.

  No committed document used the reserved field, so no valid document changes
  meaning and no schema bump is needed. This discharges the `contribution`
  half of `AMENDMENTS-RESERVED-FIELDS-UNAPPLIED`; RECORDS-TOOLING discharged
  its `profiles` half.

## Backfill decision

The pilot rules (32/32a, 33, 34, 35, 36, 37) are **not** seeded from their
reports.
- **Different quantity.** Their hand-written deltas were measured BASE→TIP
  across revisions, and some by compile-time arms. Rule 32's report, for
  example, separates rules 32, 34 and 35 by a TIP35OFF build. That is not the
  single-binary switch measurement a ledger records.
- **No mechanical source.** Transcription would need per-row premises and
  leaves that only some reports kept, so it is not mechanical.
- **Schema.** `source` accepts only `measured-by-switch`. A transcribed ledger
  would need a schema change and a provenance kind that no tool can check.

Each pilot's migration card measures its ledger by switch instead. One
follow-up is registered per pilot (`CONTRIBUTION-LEDGER-RULE-*`), each naming
the report delta the measured ledger should be compared with.

## What this card needs from GATEWAY-RULE-API

One hook covers it: **rule-id identity across the registry, the switch and the
provenance.** Each `crates/gateway-rules/registry.toml` row carries `id` equal
to its `docs/agents/rules/<id>.md` id, and rows appear in consultation order.
`QCORE_GATEWAY_RULES_OFF` takes exactly those ids. Every component of a
switched run records them in `provenance.rule_set.off`. The binary also names
the variable, as MEASURE-TOOL's `advertises_rule_switch` expects.
- The ledgers already key by that id.
- `known_rule` reads any table carrying a string `id`, in file order, for
  existence and order.
- `rules_off_mismatch` requires the off-list after every switched run.

If the registry's row shape differs, only `registry_ids` changes. That is
follow-up `CONTRIBUTION-LEDGERS-REGISTRY-BINDING`.

## Evidence

- Rust (`crates/quirin-audit-plugin-measure`):
  - **Unit tests** cover rule order and interaction keys; one rule owning its
    rows; both-needed and either-suffices rows becoming interactions; a group
    that does not add up being refused; unseparated rules forming one
    interaction; two routes on one profile being refused; and premise
    resolution through a contract sidecar.
  - **Integration test**
    `per_rule_switches_fill_the_contribution_slot_with_named_interactions`
    runs the plugin end to end with a stub analyzer that honours
    `QCORE_GATEWAY_RULES_OFF`. The stub copies one fixture state per off-list,
    with components recording that list. Two measurements run:
    `--rules-off 35,33 --per-rule` (four comparisons; ledgers `33`, `35` and
    `33+35`; premises resolved; a touched row staying Unknown with its reason)
    and `--rules-off 33` alone. Both records pass the Python authority's
    `validate-record`.
  - `per_rule_needs_two_rules_and_side_names_stay_free` covers usage refusals.
- Python (`scripts/tests`):
  - `test_contribution_ledgers.py` covers schema refusals; write, check and
    render; write refusals; rule ids against documents and then the registry;
    record-digest, slot, stray-file, misplacement and incomplete-group
    tampering; a stale group being noted; a current group missing the real
    total, which fails and leaves the files unchanged; two measurements
    sharing a row, which is an unnamed interaction; the amendments field; and,
    on the real repository, check, consistency and the committed page.
  - `test_apply_amendments.py` covers the narrowed field, its refusals, the
    page edit and an inconsistent tree being refused.
  - `test_gate_scope.py` covers the records class.
  - `test_land.py` covers the stale page.
- The gate is listed below.

GATE_PLACEHOLDER

## Boundaries

- No real switched measurement exists yet. RULE-SWITCHES and GATEWAY-RULE-API
  have not landed, so no binary advertises the switch, and every ledger in
  this card's tests comes from synthetic switched runs.
  `MEASURE-TOOL-RULES-OFF-LIVE` stays open for the first real measurement.
- A ledger attributes rows; it does not make a contract or a premise verified.
  Unknown rows stay in the denominator.
- Premises are named only as far as the contract sidecar records them. A row
  moved without hypothesis dependencies rests on no named premise.
- A ledger is bound to one baseline. Once the baseline moves, the ledger is
  stale until re-measured.
