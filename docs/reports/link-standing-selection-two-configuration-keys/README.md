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

## Headline

* **Condition 13 now applies over the configurations the link can use.**
  Rule 40 derives **105** system homes (was 2) and **124** slice homes (was
  62); the 103 and 62 condition 13 refusals are gone and every other refusal
  count is unchanged, exactly as [predicted](PREDICTION.md).
* **No verdict moves.** Slice rows are unchanged (`tail_file.c`'s incomplete
  census still refuses the import phase). On the system route 13 homes are
  imported and four Unknown rows of the linked configurations lose leaves,
  but each keeps others. The declared measurement (stated map) moved no
  row: **match** on both routes.
* Fail-closed: a second configuration the link may have used (attributed to
  a link input, linked, unrecorded, or with an unreadable attribution) keeps
  the home refused, as before.

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

## 4. Measurement

### Inputs

This host has no published live-route inputs. The cross-unit card's prepared
trees bind absolute paths into that card's worktree (database, artifact,
link inputs), so they could not be reused by copying without changing the
digests rule 40 checks. This card prepared its own: `quirin-audit prepare
clang-build --link` with the base binary (`29b4d4a4…`), Ubuntu clang
21.1.8, Fluent Bit `046accf6` (system) and upstream `90978ac6` (slice), both
linked, every linked object reproduced ([inputs.json](artifacts/inputs.json);
logs `private/logs/prep-{system,slice}.log`; artifacts system `4945551b…`,
slice `77261274…`). Their structure is the cross-unit card's: system 1,412
configurations, 999 link inputs (14 attributed to no configuration);
the same 26-home `flb_input_chunk.c` pattern in the artifact's own
route-relative census.

### The link-standing route (quick runs, not the arbiter)

`private/scripts/route.sh` (the measure plugin's route environment, copied
from the cross-unit card) with `QCORE_CROSS_UNIT_LINK_RESOLUTION` set to the
preparation's artifact; base binary `29b4d4a4…` (`2b5c8907a`), tip
`8a9dc126…` (`b5783177b`). [Lines](artifacts/route-lines.txt); row
comparisons with the cross-unit card's
[compare.py](../leri-cross-unit-elements-released/scripts/compare.py):
[system](artifacts/quick-system-resolution-base-tip.json),
[slice](artifacts/quick-slice-resolution-base-tip.json).

| Route | derived homes | condition 13 refusals | other refusals | exports / imported | rows (Safe / Unknown / Unsafe / Pending) |
|---|---|---|---|---|---|
| system, base | 2 | 103 | 4,955 / 579 / 678 / 17 | 0 / 0 | 107 / 337 / 4 / 0 |
| system, tip | **105** | **0** | unchanged | 15 / 13 | 107 / 337 / 4 / 0, no move |
| slice, base | 62 | 62 | 4,948 / 579 / 678 / 17 | 3 / census refused | 59 / 340 / 1 / 4 |
| slice, tip | **124** | **0** | unchanged | 8 / census refused | 59 / 340 / 1 / 4, no row differs |

(Other refusals: not a route configuration, not default visibility, not in
the executable, artifact refuses the facts.) The tip system run lists the
seven shared configurations, and only them, under `outside_the_link`.

System, the 13 imported homes (rule 4c's `select` holds 13 of the 15
exports; the two others were not examined further, they move no row): `flb_chunk_route_plugin_matches`,
`flb_input_chunk_down`, `flb_input_chunk_release_lock`,
`flb_input_chunk_set_up_down` (`flb_input_chunk.c`);
`flb_engine_destroy_tasks`, `flb_engine_exit`, `flb_engine_exit_status`
(`flb_engine.c`); `flb_output_get_cmt_instance`, `flb_output_log_check`,
`flb_output_prepare` (`flb_output.c`); `flb_sched_ctx_init`,
`flb_sched_timer_cb_destroy` (`flb_scheduler.c`); `flb_task_from_fs_storage`
(`flb_task.c`). The shared configurations' units list them as refused for
the caller (condition 3), and `flb_engine.c`'s static unit refuses
`flb_sched_ctx_init` (no prototype in the caller, condition 14).

**Every changed row, read** (system; all in the static, linked
configuration; status unchanged, no leaf gained):

| Row | lost | still Unknown on |
|---|---|---|
| `flb_engine.c::handle_output_event` | 1 branch-condition leaf (a call to an imported home) | 74 leaves: 24 unmodelled initialisers, uncontracted `flb_metrics_sum`, `flb_task_retry_clean`, … |
| `flb_engine_dispatch.c::flb_engine_dispatch` | uncontracted `flb_input_chunk_down` ×1, `flb_input_chunk_release_lock` ×5 | 11 leaves: unresolved `task`/`p`/`buf_size`, uncontracted `flb_input_chunk_flush`, `flb_task_create`, … |
| `flb_task.c::flb_task_retry_create` | uncontracted `flb_input_chunk_down`, `flb_input_chunk_set_up_down` | 16 leaves: unresolved `retry` ×7, indirect access ×4, `__builtin_offsetof`, … |
| `flb_task.c::task_collect_output_references` | 4 branch-condition leaves, unresolved `route` (`flb_chunk_route_plugin_matches` now contracted) | 10 leaves: increments, an out-parameter store, indirect access, unresolved `label` |

Attribution: all four are this card's (the imported homes exist only
because condition 13 now admits them). Against the prediction: home counts
exact; slice rows exactly as predicted; system 0 status moves, inside the
predicted 0 to 6 `Unknown -> Safe`, no `Safe` or `Unsafe` change. The
released-element homes `flb_engine_destroy_tasks` (imported) and
`flb_storage_input_destroy`, `flb_output_exit`, `flb_sched_timer_cleanup`,
`flb_task_retry_clean`, `flb_sched_destroy` (derived, but not among the 13
imported: rule 4c exports only a Safe home with a validated summary) are no
longer refused by the key; their caller `flb_input_instance_destroy` is still a slice unit only
(SELECTION-ROUTES-SPLIT-CALLER-AND-HOME).

### Arbiter

`quirin-audit measure --declared PREDICTION.md --routes system,slice`
(`private/scripts/arbiter.sh`, output `measure/arbiter-1`, measure plugin
`ba1fd99d…` built from this worktree), the live routes as the plugin runs
them (stated eight-home map: the plugin has no link-resolution side). Sides
are `git archive` exports of `2b5c8907a` (base) and `b5783177b` (tip) with
`third_party/libevent` at its pin `d82464a2`, each built by the tool;
`--no-slots`, disclosed. Exit 0, status complete, no failure.
[native-comparison-system-slice.json](artifacts/native-comparison-system-slice.json):

| Route | base | tip | moved | content changed | declared |
|---|---|---|---|---|---|
| system (448 rows) | 107 / 4 / 337 | 107 / 4 / 337 | 0 | 4 | **match** |
| slice (404 rows) | 59 / 1 / 340 / 4 Pending | 59 / 1 / 340 / 4 | 0 | 8 | **match** |

Every content-changed row was read: all twelve change only `review_holds`,
whose module path spells the side's export directory. With
`private/{base,tip}-src` normalised, status, diagnostics and holds are equal
on all 852 rows (`private/scripts/holds_all.py`), the cross-unit card's
finding again.

## 5. Corpus

Not measured. `quirin-audit pinned` refuses a link-resolution artifact
(rule 40) and the change is reached only under one; no gateway, schema,
contract or provider code changed.

## 6. Follow-ups

[integration-amendments.json](integration-amendments.json) retires
LINK-STANDING-SELECTION-TWO-CONFIGURATION-KEYS and amends
SELECTION-ROUTES-SPLIT-CALLER-AND-HOME (the key no longer refuses its homes).
Both ids are registered by LERI-CROSS-UNIT-ELEMENTS-RELEASED's amendments,
which land first: `--check` exits 0 on a private clone of this branch after
applying those first (`private/scripts/amend_check.sh`), and exits 1 on this
tree alone (the id is not yet registered). No new follow-up: the remaining
blockers on these routes are the two already registered
(TAIL-FILE-DEFINITION-CENSUS-INCOMPLETE, SELECTION-ROUTES-SPLIT-CALLER-AND-HOME).
`profiles: []`. The increment paragraph is the file's `increment_paragraph`.

## 7. Gate

GATE
