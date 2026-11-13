# LINK-STANDING-SELECTION-TWO-CONFIGURATION-KEYS

Card LINK-STANDING-SELECTION-TWO-CONFIGURATION-KEYS (named follow-up
registered by LERI-CROSS-UNIT-ELEMENTS-RELEASED, not yet landed). Base
`2b5c8907a`, the merge of that card's tip `7e2e255e1` onto origin/main
`591e5523f` (parents `591e5523f`, `7e2e255e1`; the brief's fast-forward was
impossible because this branch already sat on `591e5523f`, so the merge was
taken in the other direction, with the same tree). Rule 40 amended in place
(and one sentence of rule 4c); Lean `QCore.CrossUnitSummary` extended. The
[prediction](PREDICTION.md) was committed at `5dc3a789b`, before any code
change.

MEASUREMENTS

## 1. Mechanism

The Fluent Bit system and tail-pause selections keep every compilation
configuration of a selected file, so each core file is two route units
(static, linked into `fluent-bit`; shared, for `libfluent-bit.so`, not
linked) under one `library/file` key. Condition 13 counted both and refused
every home there.

* **Shown outside the link** (`shown_outside`): the link-resolution
  artifact records the configuration with `linked: false`, and every link
  input's attribution (the configurations whose object has its bytes) is
  readable and names it nowhere. Then the link used nothing that
  configuration compiles. `linked: false` alone is not enough: an input
  whose bytes match two configurations leaves both `linked: false`, and
  either may be the one linked.
* **Condition 13** counts, under the home's key, only the route
  configurations not shown outside the link; the home's own configuration
  must be one of them. A configuration the artifact does not record, one an
  input is attributed to, or any configuration when one input's attribution
  is unreadable still counts (fail-closed).
* **Rule 4c's census** on a derived run: a configuration shown outside the
  link exports and defines nothing (`current_route.rs`, as a not-linked
  standalone unit already did since CURRENT-MONKEY-LINKED-CONFIGURATION),
  so the key's one candidate is the linked configuration and `select` finds
  one export and one definition row. Without this half, both configurations
  export the home and the import is refused as ambiguous (verified: with the
  exclusion disabled, the end-to-end fixture's export count is 2 and it
  fails; `private/logs/e2e-revert-census.log`).
* The outside configuration's rows are still analysed and published; as a
  caller it fails condition 3 as before. The run's
  `cross_unit_link_resolution` assumption lists the configurations left out
  (`outside_the_link`). The artifact format and `prepare --link` are
  unchanged; the artifact's own route-relative reason ("home unit key names
  more than one route configuration") stays advisory and is re-decided by
  the route, as rule 40's condition 10 already says.

Why this is sound: an admitted home is still the one strong definition of
its name among the linked objects (condition 8), attributed to the home's
linked configuration (conditions 10 and 11). The census exclusion removes
only rows of a configuration no linked object comes from; whether the
static and shared compilations of the same file would give the same summary
is irrelevant, because the replayed home is the linked configuration's.

## 2. Fixtures

`crates/qcore-audit/src/cross_unit_links.rs` (unit tests):
`an_unlinked_second_configuration_leaves_the_key_to_the_linked_one` derives
the home of a key whose `-fPIC` configuration is recorded and unattributed,
and refuses three keys whose second configuration is attributed through an
input matching two configurations, linked, or not recorded; with one input's
attribution unreadable, all four are refused and nothing is outside.
`outside_vectors_hold` runs `OUTSIDE_VECTORS` (9 rows, every `linked` ×
attribution case), equal to the Lean `outsideVectors`. The existing
`a_derived_resolution_decides_every_home_condition` is unchanged (its
`amb.c` second configuration is not recorded, so it stays refused).

`crates/qcore-audit/tests/link_standing_two_configurations.rs` (end to end,
`quirin-audit current` with a selection that keeps both configurations of
`src/home.c`, static and `-fPIC -DQ_SHARED`): without the artifact `subject`
is Unknown (uncontracted `drain`); with the second configuration outside the
link, rule 40 derives the home (refused `{}`), one home export, `subject`
Safe, and the sidecar lists the `-fPIC` configuration alone under
`outside_the_link`; with it attributed to a link input, or unrecorded, 0
homes, the condition 13 refusal, `subject` Unknown.

## 3. Lean

`proof/QCore/CrossUnitSummary.lean`, audited by
`proof/AuditCrossUnitSummary.lean` (13 new `#print axioms`):
`shownOutside` and `shownOutside_iff`; `outsideTheLink` over the artifact's
record and the inputs' attribution, with
`outside_configuration_is_no_link_input` (what "defines nothing the link
could choose" rests on), `an_unrecorded_configuration_is_not_outside`,
`an_attributed_configuration_is_not_outside`,
`an_unreadable_attribution_shows_nothing_outside`; `keyUnique` over
`usableUnder`, with `a_single_configuration_key_is_unique` (rule 40 as it
was), `an_outside_second_configuration_leaves_the_key`,
`a_usable_second_configuration_keeps_the_key` and, through `standing`,
`a_usable_second_configuration_refuses`; `derivedCensus`, with
`the_census_leaves_the_linked_home_unique`,
`an_unexcluded_second_definition_is_ambiguous` and
`an_outside_second_configuration_admits_the_linked_home` (the admission
through `derived_resolution_discharges`); `outside_vectors_hold`. No
placeholder, no project axiom. `DerivedFacts`, `standing` and
`standingVectors` are unchanged: condition 13 is still field 12, only how
`derive` decides it changed.
