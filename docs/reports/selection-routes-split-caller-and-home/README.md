# SELECTION-ROUTES-SPLIT-CALLER-AND-HOME

Card SELECTION-ROUTES-SPLIT-CALLER-AND-HOME (named follow-up registered by
LERI-CROSS-UNIT-ELEMENTS-RELEASED, amended by
LINK-STANDING-SELECTION-TWO-CONFIGURATION-KEYS). Base `f47823dd1`
(integrate/r10); measured tip `2c3bfc453` (later commits are records). The
[prediction](PREDICTION.md) was committed at `b5763ac87`, before the new
route was run under rule 40 (section 3 says what had run). Tooling and one new
selection: no rule, gateway, contract, provider or schema change, so no Lean
change (no semantic change to mirror).

## Headline

* **A measured route holds both halves.**
  [`system-teardown-selection.json`](../../../contract/profiles/fluent-bit/system-teardown-selection.json)
  is the system selection's eight files plus `src/flb_input.c`,
  `src/flb_input_thread.c` and `src/flb_output_thread.c`, every file that
  calls a released-element drain home from another file and whose census is
  complete. Under rule 40, `flb_input_instance_destroy` (the static, linked
  configuration) now imports `flb_engine_destroy_tasks`, a rule 43 drain:
  with `QCORE_GATEWAY_RULES_OFF=43` that home is not exported and the call is
  uncontracted again, so the import is the `ElementsReleased` summary.
* **The measure plugin has a link-resolution side** (`--link-resolution
  NAME`, fail-closed, section 2), so `quirin-audit measure --declared` now
  measures a route under rule 40.
* **No status moves, as predicted** (declared result `match` on both routes).
  `flb_input_instance_destroy` loses one leaf and stays Unknown on 59;
  `flb_storage_input_destroy`, a Safe row, is not exported, so its two calls
  stay uncontracted (follow-up).

## 1. Design

**A new selection, not a widened live one.** Widening `system-selection.json`
would move the live `fluent-bit-system` profile's figures and baseline, a
user decision, and the slice cannot hold the homes under rule 40 (its
preparation is upstream `90978ac6`, and `tail_file.c`'s census refuses its
import phase). The new selection is a card-local route with no evaluation
profile: it is measured through an inputs manifest
([artifacts/inputs.json](artifacts/inputs.json)) and changes no baseline.
Its files are the system selection plus the cross-unit callers of the six
drain homes LERI-CROSS-UNIT-ELEMENTS-RELEASED named
(`flb_engine_destroy_tasks`, `flb_storage_input_destroy`, `flb_output_exit`,
`flb_sched_timer_cleanup`, `flb_task_retry_clean`, `flb_sched_destroy`):
`flb_input.c` (`flb_input_instance_destroy`), `flb_input_thread.c` and
`flb_output_thread.c` (the scheduler drains). `flb_config.c`
(`flb_config_exit` calls `flb_sched_destroy`) was in the first draft and is
left out: its census is refused (`conditional-arm-depth-disagreement`, the
ground of TAIL-FILE-DEFINITION-CENSUS-INCOMPLETE; 12 definitions absent from
the raw scan), which refused the whole import phase
(`private/runs/teardown-map-1`). Callers of `flb_task_retry_clean`,
`flb_output_exit` and `flb_sched_timer_cleanup` in `flb_engine.c` were already
system units.

## 2. The link-resolution side

[`link.rs`](../../../crates/quirin-audit-plugin-measure/src/link.rs), wired
in `lib.rs`, `sides.rs`, `inputs.rs` and `baseline.rs`;
[usage](../../development/audit-subcommand-plugins.md#the-measure-plugin).

* A configured route of an inputs manifest may declare
  `"link_resolution": {"path", "sha256"}` (relative to the inputs base; the
  digest is required; a pinned route refuses one). A route without one keeps
  its canonical JSON, so every existing route digest and baseline is
  unchanged.
* `--link-resolution NAME` (repeatable) names a side that runs every route
  with `QCORE_CROSS_UNIT_LINK_RESOLUTION` set to the route's artifact and
  without `QCORE_CROSS_UNIT_LINK_MAP`. Every other side runs the stated map,
  so two sides of one binary compare the stated map with rule 40.
* Fail-closed, never a fallback to the map, at three points: before anything
  is built or run, every route must declare an artifact that exists, has the
  declared digest and the `quirin-link-resolution/1` schema, and binds the
  compile database the route reads as spelled (rule 40's condition 1; a
  re-spelled mirror refuses), and the side must run (`baseline:` and `run:`
  sides are refused); after the build, the side's binary must name the
  variable (an analyzer without rule 40 would import nothing silently); after
  each route, the analyzer must have printed `cross-unit link resolution
  derived (rule 40) from … (sha256 <artifact>)` and its contract sidecar must
  record no whole-run refusal, otherwise the route fails with exit 102
  (`link_resolution_error`). The artifact is part of the route's
  before/after input digests.
* Records: `run.json` and the comparison record's `sides` slot carry
  `link_resolution`; each route's `invocation.json` the artifact and what the
  run derived (`link_resolution_run`: the derived line, import-phase
  refusals, the imported count). `measure publish` refuses a linked side: a
  profile's baseline is its route under the stated map.
* Tests: `inputs.rs` (`a_link_resolution_artifact_is_pinned_and_configured_only`),
  `link.rs` (`a_run_is_accepted_only_when_it_derived_from_the_artifact`) and
  [`tests/link_resolution.rs`](../../../crates/quirin-audit-plugin-measure/tests/link_resolution.rs)
  through the host harness with a stub analyzer: the linked side runs under
  the artifact and never the map, the map side never under the artifact, a
  declared comparison matches, `publish` refuses; a route without an
  artifact, a wrong digest, an artifact bound to another database, a missing
  artifact, a binary that does not read the variable, a `run:` side and an
  unknown side name each refuse before any route runs; a run that printed the
  whole-run refusal fails its route with exit 102 and the measurement fails.

## 3. Prediction and inputs

This host has no published live-route inputs and refuses unprivileged user
namespaces, so no `rev:` side runs here. `quirin-audit prepare clang-build
--link` with the base binary (`95eb1575…`, built from `f47823dd1`), Ubuntu
clang 21.1.8, prepared and linked Fluent Bit `046accf6`
(`private/prep/system`, log `private/logs/prep-system.log`; database
`40bc0b35…`, artifact `5756d7d8…`, its rule 40 artifact). Before the
prediction was committed, the new selection had run under the stated map
(with and without `flb_config.c`) and the system selection under rule 40
(105 derived homes, 14 exports, 12 imported); the plugin side was committed
(`d487691cb`) but had measured nothing. [PREDICTION.md](PREDICTION.md):
no status move on either route, `flb_input_instance_destroy` and
`flb_input_collectors_start` losing their uncontracted-call leaves to the
imported homes, the system route's four known leaf trades.

## 4. Measurement

### Arbiter

`quirin-audit measure --declared PREDICTION.md --routes
system,system-teardown --link-resolution linked`
(`private/scripts/arbiter.sh`, output `measure/arbiter-2`, log
`private/logs/arbiter-2.log`). Both sides are the same binary,
`private/tip-src/target/release/quirin-audit` (`6543f75b…`), built in a `git
archive` export of the tip `2c3bfc453` with `third_party/libevent` cloned at
its pin `d82464a2` and `QCORE_AUDIT_BUILD_COMMIT` set; the plugin is the
export's own (`feee3376…`). Side `map` runs the stated eight-home map, side
`linked` the preparation's artifact. A binary side outside a repository
records no revision. `--no-slots --jobs 6`, disclosed. The analyzer is
unchanged by this card (no `crates/qcore-*` file changed), so no
base-against-tip comparison of it is needed. A first attempt (`arbiter-1`,
the worktree's own binary as both sides) was stopped: a test edit made
during it would have changed a side's inputs mid-route; it is discarded.
Exit 0, status complete, no failure, declared **match**.
[native-comparison-map-linked.json](artifacts/native-comparison-map-linked.json):

| Route | rows | map | linked | moved | content changed | derived / exported / imported |
|---|---|---|---|---|---|---|
| system | 448 | 103 / 4 / 341 | 103 / 4 / 341 | 0 | 8 | 105 / 14 / 12 |
| system-teardown | 662 | 135 / 4 / 523 | 135 / 4 / 523 | 0 | 11 | 162 / 19 / 16 |

(Safe / Unsafe / Unknown; no Pending.) Each linked route printed the derived
line for artifact `5756d7d8…` with no import-phase refusal; 7 and 10 shared
configurations are listed outside the link.

**Every content-changed row, read** (no row gains a leaf; status unchanged):

| Row (static configuration) | change | attribution |
|---|---|---|
| `flb_input.c::flb_input_instance_destroy` | loses `call to uncontracted function flb_engine_destroy_tasks` (`flb_input.c:1239`); summaries gain `flb_engine_destroy_tasks`, declarations `flb_task_destroy` (the home's) | the import of the drain (rule 43's `ElementsReleased`, confirmed by the rule 43 switch below); 59 leaves remain: unresolved `ins` ×9, indirect accesses ×7, uncontracted `flb_config_map_destroy` ×5, `flb_kv_release` ×4, `flb_hash_table_destroy` ×4, `flb_storage_input_destroy` ×2, member releases through the formal ×14, ... |
| `flb_input.c::flb_input_collectors_start` | loses uncontracted `flb_input_thread_collectors_signal_start` | imported from `flb_input_thread.c`; 3 leaves remain (nested `flb_input_name` ×2, uncontracted `flb_input_collectors_signal_start`) |
| `flb_engine_dispatch.c::flb_engine_dispatch` | loses uncontracted `flb_input_chunk_down`, `flb_input_chunk_release_lock` ×5 | imports, as LINK-STANDING-SELECTION-TWO-CONFIGURATION-KEYS measured |
| `flb_task.c::flb_task_retry_create` | loses uncontracted `flb_input_chunk_down`, `flb_input_chunk_set_up_down` | same |
| `flb_task.c::task_collect_output_references` | loses 4 branch-condition leaves and unresolved `route` | same (`flb_chunk_route_plugin_matches`) |
| `flb_engine.c::handle_output_event` | loses 1 branch-condition leaf; summaries gain `flb_task_from_fs_storage` | same |
| `sb.c::sb_allocate_chunk`, `sb_destroy_chunk` (Safe), `sb_append_chunk_to_segregated_backlog`, `sb_remove_chunk_from_segregated_backlog` (Unknown) | dependency record only: their own unit's declaration `sb_allocate_chunk` / `sb_destroy_chunk` moves from `declarations` to `withdrawn` | the unit's contract-body evidence gains `C06.3-no-body` withdrawals of those names, recorded from a replayed home's stream (rule 4c writes the replay's record into the caller's recording); no leaf moves (follow-up CROSS-UNIT-REPLAY-RELABELS-CALLER-DEPENDENCIES) |
| `flb_output.c::flb_output_new` (teardown only) | dependency record only: summaries gain `instance_id` (a static helper of the same unit) | the import-phase re-analysis of `flb_output.c`, which now holds `flb_input.c` homes; not explained further (same follow-up) |

The system route's eight are the four sb.c rows and the four import rows
(`flb_engine_dispatch`, `flb_task_retry_create`, `task_collect_output_references`, `handle_output_event`). Against the prediction:
status moves exact (none); the predicted leaf trades all occurred, except that
`flb_input_instance_destroy` keeps its two `flb_storage_input_destroy` leaves
(predicted lost: that home is not exported); the sb.c and `flb_output_new`
record changes were not predicted.

### Rule 40 quick runs (not the arbiter)

`private/scripts/route.sh` (the plugin's route environment) with the base
binary `95eb1575…`; [lines](artifacts/route-lines.txt), row comparisons with
LERI-CROSS-UNIT-ELEMENTS-RELEASED's
[compare.py](../leri-cross-unit-elements-released/scripts/compare.py):

* teardown, map against rule 40
  ([quick-teardown-map-resolution.json](artifacts/quick-teardown-map-resolution.json)):
  0 moves, the same six leaf-trading rows as the arbiter;
* teardown under rule 40, rule 43 on against off
  ([quick-teardown-resolution-off43.json](artifacts/quick-teardown-resolution-off43.json)):
  with `QCORE_GATEWAY_RULES_OFF=43`, 18 exports (19), 15 imported (16):
  `flb_engine_destroy_tasks` alone is no longer exported, and the only row that
  changes is `flb_input_instance_destroy`, which regains the uncontracted
  `flb_engine_destroy_tasks` leaf. The import that crosses is therefore rule
  43's released-element summary.

### Which drains reach their callers

| Drain home | row | exported | callers in the route |
|---|---|---|---|
| `flb_engine_destroy_tasks` | Safe | yes | `flb_input_instance_destroy` (static) imports it; the shared configuration fails condition 3 |
| `flb_storage_input_destroy` | Safe | **no** | `flb_input_instance_destroy` keeps two uncontracted calls (SELECTION-TEARDOWN-STORAGE-DRAIN-NOT-EXPORTED) |
| `flb_task_retry_clean` | Unknown (uncontracted `flb_task_clear_route_retry_context`) | no | `flb_engine.c` |
| `flb_sched_timer_cleanup` | Unknown (uncontracted `flb_sched_timer_coro_cleanup`) | no | `flb_engine.c`, the two thread loops |
| `flb_sched_destroy` | Unknown (unresolved `sched` ×3, an undecided descriptor cell) | no | the two thread loops; `flb_config.c` left out |
| `flb_output_exit` | Unknown (9 leaves) | no | `flb_engine.c` |

Only a Safe home with a validated summary is exported (rule 4c), so the four
Unknown drains reach no caller whatever the route; that is their own rows'
work, out of this card's scope.

## 5. Corpus and live routes

Not measured. The pinned corpus route refuses a link-resolution artifact, no
analyzer, contract or provider code changed, and the live profiles are
untouched (no `contract/evaluations` change; the system selection is
unchanged; every existing route digest is unchanged because a route without
`link_resolution` serialises as before).

## 6. Follow-ups

[integration-amendments.json](integration-amendments.json) (`--check` exit
0): retires SELECTION-ROUTES-SPLIT-CALLER-AND-HOME; amends
LINK-PREPARE-PINNED-ROUTE-INPUTS (the plugin side exists; the frozen inputs
are still unlinked and the live profiles still run the stated map) and
TAIL-FILE-DEFINITION-CENSUS-INCOMPLETE (`flb_config.c` has the same census
refusal); registers SELECTION-TEARDOWN-STORAGE-DRAIN-NOT-EXPORTED,
CROSS-UNIT-REPLAY-RELABELS-CALLER-DEPENDENCIES and
SYSTEM-TEARDOWN-LIVE-PROFILE (making this route, or the system profile under
rule 40, a live profile is a user decision). `profiles: []`. The increment
paragraph is the file's `increment_paragraph`.

## 7. Gate

`python3 scripts/gate_scope.py --base f47823dd1` (through
`private/toolchain/env.sh`) on `cf846229f`: the measured tip `2c3bfc453` plus
the integrator's fixture fix `562d4594d`, cherry-picked at the integrator's
request (both cross-unit fixtures' `item_destroy` now name the link member
`_head`, which rule 46 requires; test-only plus an increment paragraph; the
five failures it fixes, `cross_unit_elements_released` ×4 and
`link_standing_elements_released` ×1, are the base's, not this card's). The
analyzer and plugin code are those measured. [gate.json](gate.json), logs
`private/gate-scope/20261113T165757Z-2962791/`, console
`private/logs/gate-2.console.log`; Clang recording
[gate/clang-version.txt](gate/clang-version.txt), taken in this worktree.
**Exit 1**, mode scoped, 10 steps:

| Step | Exit | Counts |
|---|---|---|
| submodules, fmt, clippy (4 invocations, `-D warnings`) | 0 / 0 / 0 | 1 submodule |
| Rust suite (`qcore-audit`, `qcore-compiler`, `qcorec`, `quirin-audit-plugin-measure`, `quirin-source-analysis` whole; 33 test targets) | 0 | 251 units, 255 summaries; 3,264 passed, 0 failed, 5 ignored |
| both reevaluation probes | 0 / 0 | |
| Lean | not selected | no `proof/` change |
| Python suite | **1** | 761 tests, 1 failure |

The one Python failure is the base's, not this card's:
`CommittedMatrixIsSelfConsistent.test_each_derived_dimension_matches_the_section_it_counts`
reads `dimensions.named_followups` 949 against 956 entries in the committed
`docs/reviews/r0_3/acceptance_matrix.json`, which this branch does not change
(`git diff f47823dd1 HEAD -- docs/reviews scripts` is empty): the integrate/r10
amendments were applied without regenerating the matrix, the integrator's
record. A first gate attempt (`private/logs/gate-1.console.log`, on
`2c3bfc453`) was stopped before its Rust suite to take the cherry-pick; it is
discarded. No provider change, so no provider gate.
