# LINK-STANDING-IMPORT-LIMITERS

Card of round 31 (user decision 2026-11-16, "yes, launch it"). Base: main
`0f6e17118`. Rules amended in place: [4c](../../agents/rules/4c.md) (replay
budget) and [40](../../agents/rules/40.md) (stray probes).

Rule 40 derived 1,583 homes on the user's linked `current` run, but callers
held only 262 imports and six Unknown -> Safe rows depended on one. This card
measures why and removes the two limiters that fail-closed does not require.

## Census

Evidence: the integrator's linked run at binary `89fa31d21`
(`private/current-linked-556460e31`, read-only). The scripts in
[`census/`](census/) preprocess each unit with its own recorded arguments
(`clang -E`), reproduce rule 4c's scope (`for_caller`: the 260 run homes, the
unit's definitions, its used identifiers, the run's member-named homes) and
compare it with the run's own `cross-unit scope` progress lines. All 186
units with a recorded compile task reproduce exactly (0 mismatches in
reachable and named-only counts). The per-unit table is
[`census/census-table.json`](census/census-table.json): reachable homes and
which, member-named and stray homes, the scope after the card, the held and
withheld homes, and the functions blocked by home calls.

| Measure | Base run | After the card (census) |
|---|---:|---:|
| units with a scope | 186 | 186 |
| units above the budget of 24 (no import held) | 99 | 4 (hold a subset) |
| member-named homes per unit (`named only`) | 18 to 23 | unchanged where declared |
| stray probes (member-named, no token in the unit) | 3,633 (1,554 probed in held units) | 0 |
| reachable homes, before rule 40's caller conditions | 2,016 held in 87 units | 1,545 held in 138 units |
| functions whose every diagnostic is a call to a run home | 7 | all 7 have their homes held; 1 home is refused by type (`rd_kafka_mem_malloc` in `lz4.c`) |
| functions blocked by a home call and another diagnostic | 255 | (status cannot move) |

Distribution of reachable homes per unit without the strays: 48 units 0,
18 units 1, then 2 to 22; four units exceed 24 (26, 27, 27, 31). In the
base run, every withheld unit had 25 to 49 reachable homes, of which 18 to
23 were member-named: the strays alone exhausted the budget in 95 of the 99.

### Stray probes: where each name comes from

Traced end to end in `cmetrics/lib/cmetrics/src/cmt_label.c` (23 refusals,
none of the 23 names spelled in its source or headers):

1. `for_caller` (`legacy_cross_unit.rs`) builds the scope over every run home.
   A home is reachable when the caller does not define it and either uses it
   or `named(name)` holds.
2. `named` is `RunMemberFacts::named_functions()`: the union, over **every
   member of every unit of the run**, of the functions a member's closure can
   deliver. It is run-wide by design (rule 4c, card CURRENT-RUN-WORKER-SPIN):
   it is a sound over-approximation of the candidates a member invocation in
   any unit can resolve to.
3. The 23 names enter it from two store sites:
   - 18 Monkey functions, stored into Monkey's plugin API table in
     `mk_server/mk_plugin.c` (`api->kernel_features_print =
     mk_kernel_features_print;`, `api->sched_get_connection = ...`,
     `api->pointer_print = mk_ptr_print;`, `api->ev_backend =
     mk_event_backend;` and so on);
   - 5 librdkafka destructors passed to `rd_list_init`/`rd_list_new`, whose
     formal the list stores in `rl_free_cb` (`rdkafka_admin.c`,
     `rdkafka_cgrp.c`).
4. Each is a run home (exported Safe), so every caller that does not define
   it holds it: 23 of the 24 budget slots in a cmetrics unit.
5. Rule 40's admission (`LinkStanding::admit`) then probes
   `typedef __typeof__(NAME) ...` in the caller unit. A cmetrics unit never
   declares a Monkey or librdkafka function, so Clang rejects the probe
   ("use of undeclared identifier") and the home is refused at condition (14).

None of the candidate causes in the brief is it: the names are not carried
from another unit's home set, not macro-expanded and not counted function
pointer calls. They are the run-wide member-named set, which rule 40 then
probes in units that cannot type them.

## Stray probes: the fix (rule 40, amended)

The run-wide set stays: it is rule 4c's sound over-approximation of what a
member invocation can resolve to, and narrowing it for every run would be a
separate decision (follow-up `CROSS-UNIT-MEMBER-NAMED-SCOPE-PER-UNIT`). On a
derived run, though, rule 40 adds condition (14), and (14) cannot hold for a
name that has no token at all in the caller's preprocessed unit: the unit
never declares it, so Clang has no type for it there. `for_linked_caller`
(`legacy_cross_unit.rs`, taken by `projection.rs` whenever a link admission
is in force) therefore leaves such a home out of the scope before the
budget:

- it is not held, not probed and not counted toward the budget;
- it is not a caller refusal; the unit row counts it as
  `cross_unit_unprobed_homes`, and the progress line says
  `N member-named left unprobed`;
- a name the unit declares (a header prototype) or uses stays in scope and is
  probed as before. If Clang cannot type it, it still refuses (fixture: a
  K&R `void named();` declaration refuses "no prototype").

Fail-closed: the filter only removes homes from what a caller holds, and
every removed home would have been refused at (14). A stated map (rule 4c
alone, the pinned routes) is unchanged: there the member-named home is still
held, since no type condition applies.

## Replay budget: the subset (rule 4c, amended)

Before the card a caller over `REPLAY_BUDGET_PER_UNIT = 24` held nothing.
Now `select` holds at most 24:

1. each function the unit defines gives its home set: the run homes its body
   names, closed over the unit's own functions it names (a function's row
   needs its local callees' summaries, so their homes too);
2. sets are taken smallest first, the function's position in name order
   breaking ties; a set is held whole when the union still fits, otherwise
   skipped (a partial set cannot make that function Safe);
3. the remaining reachable homes (member-named ones, file-scope initializer
   uses, functions the scan missed) fill what room is left, by home name.

The subset depends only on the unit's text and the run's homes, never on the
order the sets are given in (`selection_is_deterministic`). The unit row's
`cross_unit_imports_withheld` names every withheld home; their calls stay
uncontracted. The bound, the certificate check's refusal of a home set above
it, and every per-home check of rules 4c and 40 are unchanged, so the choice
of subset is a coverage priority inside a fixed cost bound, not a semantic
premise. With the stray fix only four units of the user's run exceed the
budget (26 to 31 reachable), and none of the seven sole-blocked functions
loses a home to the subset.

Raising the bound was not needed and is not done: the census shows the
budget was exhausted by strays, not by real reach.

## Replay cost

The bound is unchanged, but many more callers now use it, and each held
home's unit is replayed once per process (rule 4c's shared replay) before
the first check that holds it. Measured on the heaviest caller the subset
newly lets import: `rdkafka_request.c`, 22 reachable homes in 12 home units
(`rdkafka_admin.c`, `rdkafka_metadata.c` and `rdkafka.c` among them). The
selection is that caller plus its 12 home units
(`private/measure/selection-request.json`, not committed), run with the
user's `current` arguments, rule 40's artifact, `--monkey-linked` and the
tip binary, through `gate_slot.py audit --memory-budget 7G` (3 workers; the
host was shared with another card's jobs):

| Measure | Selection, tip | Base full run, same unit |
|---|---:|---:|
| exit | 0 | 0 |
| wall / user CPU | 1 h 14 min 48 s / 6,297 s | 2 h 14 min (whole run) |
| peak RSS (`time -v`, receipt) | 9.87 GB | 10.4 GB (whole run) |
| phases: pre-pass, units, export, import | 439 s, 1,021 s, 941 s, 2,077 s | 245 s, 3,692 s, 1,990 s, 2,045 s (8 workers) |
| `rdkafka_request.c` import | 1,164.6 s, 16 homes held after rule 40 | 173.2 s, withheld (0 held) |
| `rdkafka_admin.c` import | 558.2 s, 11 held | 551.2 s, withheld |
| `rdkafka.c` import | 280.7 s, 8 held | 184.4 s, withheld |
| `rdkafka_metadata.c` import | 217.5 s, 8 held | 236.7 s, withheld |

The difference for `rdkafka_request.c` (about 990 s) is the first replays of
its home units. That caller happened to pay for them, among them
`rdkafka_admin.c`, whose export analysis takes 514 s. Later callers of the
same homes reuse them (`rdkafka_admin.c` and `rdkafka_metadata.c` import in
their base time). The selection's peak (9.87 GB) is below the base full
run's 10.4 GB. The member-facts pre-pass over all 1,419 units dominates it,
as the CURRENT-RUN-WORKER-SPIN runs found.

Estimate for the full run: the base run replayed 20 home units. The new
scope adds up to 64 more, whose export analyses total about 1,800 s of CPU
(first-phase times total 3,510 s). Each is paid once per process, so the
import phase should grow by about that CPU spread over the workers. On the
critical path, one caller may wait about 1,000 s, as `rdkafka_request.c` did
here. Peak RSS also grows by the retained replay tables. The integrator's
full rerun measures both; this card does not raise the bound or change the
replay sharing. Follow-up `CROSS-UNIT-MEMBER-NAMED-SCOPE-PER-UNIT` would cut
held homes no member invocation of the caller can reach. The Monkey units in
this run each still hold 13 to 17 such homes.

**Verdicts in the selection.** Against the base run's rows of the same 43
units (1,501 rows), one row moves: `mk_lib.c` `mk_mq_send` Unknown -> Safe
(predicted; its home `mk_fifo_send` was withheld by the budget at base). No
other row changes, and the base's import-dependent Safe rows in these units
stay Safe. The withheld homes of `mk_server.c` and `monkey.c` are exactly
the predicted ones. No refusal reads "undeclared identifier" any more. The
caller refusals left are type refusals: 111 "unnamed or unreadable tag" and
32 "no prototype" (below).

## Unnamed tags and functions without a prototype

Both stay refused. Neither is implemented, because each is sound only with
more machinery than the card can justify. In the base run's evidence no
function is blocked by either alone. Once the budget no longer withholds
whole units, though, the unnamed-tag refusal becomes the main remaining
caller refusal (selection run above). Besides `mk_ptr_t` it now refuses:

- librdkafka's `rwlock_rdlock`, `rwlock_rdunlock`, `rwlock_wrlock` and
  `rwlock_wrunlock`: glibc's `pthread_rwlock_t` is a typedef of an untagged
  union;
- `cnd_timedwait_abs`: `cnd_t` is `pthread_cond_t`, another untagged union;
- several librdkafka enumeration typedefs (`rd_kafka_resp_err_t`,
  `rd_kafka_msg_status_t`, `rd_kafka_consumer_group_state_t` and others).
  These are untagged enums, for which C11 6.2.7p1 also requires equal
  enumeration constants, which rule 40 does not compare today.

The follow-up below should be weighed with that in mind.

- **`mk_ptr_t` (42 refusals, unnamed tag).** `typedef struct { char *data;
  unsigned long len; } mk_ptr_t;` in `mk_memory.h`. C11 6.2.7p1 makes two
  untagged structures in different units compatible when their members match
  one to one in name, compatible type and order (and bit-field width).
  Clang's canonical complete layout (already read for tagged records) lists
  exactly those, with embedded records expanded, so equal layouts would
  establish it soundly. The layout dump names an unnamed record only by its
  location `(unnamed at <stdin>:L:C)`, which differs between units'
  preprocessed texts. The signature would have to render the record by its
  structural layout, found through the AST declaration's location. A member
  pointer to another unnamed record would have to refuse, since it has no
  tag to compare. An untagged enum would need its enumeration constants
  compared as well. That is not small. In the base run all 42 refusals are
  `mk_ptr_print` and `mk_ptr_to_buf`, member-named homes (Monkey's plugin API
  table) held in Monkey units that never call them. With the card, called
  homes (`rwlock_*`, `cnd_timedwait_abs`, enum-typed librdkafka functions)
  reach this refusal too. Follow-up `LINK-STANDING-UNNAMED-RECORD-LAYOUT`,
  which covers unions and enums as well.
- **No prototype (19 refusals at base; 32 in the selection, adding
  `mk_utils_get_system_core_count`, `mk_core_init`, `mk_event_init` and other
  Monkey initialisers).** `mk_event_backend` (17), `cio_getpagesize`
  and `mk_net_init`. Each is declared `T name();` in its header and defined
  `T name() { ... }`, so both sides have Clang's `FunctionNoProtoType`. C11
  6.7.6.3p14 says the empty list of a definition means no parameters, and
  6.5.2.2p6 makes a call with another argument count undefined. Sound
  acceptance needs three things: both types without a prototype with equal
  return types; a home definition with no parameters (a K&R definition with
  an identifier list would need the promoted-type rule of 6.5.2.2p6); and
  every use of the name in the caller a call with no arguments (an address
  taken could be called with arguments elsewhere). That is a type-reading
  change, a home-definition check, a caller call-site check and a Lean
  change to `typeAgrees`: not small. No function is blocked by these homes
  alone. `mk_net_init` is a co-blocker of two functions and `cio_getpagesize`
  of one, each with other diagnostics. Follow-up
  `LINK-STANDING-NO-PROTOTYPE-ZERO-PARAMETERS`.

## Lean

`QCore.CrossUnitReplayBudget` (amended; `proof/AuditCrossUnitReplayBudget.lean`):

- `reachable` takes rule 40's `probe` flag and the unit's `present` names;
  `unprobed` is the member-named homes it leaves out;
- `union`, `addGroups`, `fill`, `select`: the subset over ordered inputs
  (the Rust computes the order: set size then function position; home names);
- work bound: `addGroups_length`, `fill_length`, `select_within_budget`,
  `held_within_budget`, `replay_work_bound` (`callers × budget`, unchanged);
- fail-closed: `select_sub_reachable`, `held_is_a_run_home`,
  `within_budget_holds_reachable`, `held_lookup_is_full` (a held home reads
  what the full table supplies), `withheld_import_call_is_unknown` (a
  withheld home's call reads `Unknown`), `restricted_lookup_is_full_or_absent`,
  `consulted_lookup_unchanged`;
- rule 40: `unprobed_is_never_held`, `probing_shrinks_reachable`;
- `scope_vectors_hold`: the ten `SCOPE_VECTORS` of the Rust.

The base theorems `exhausted_holds_none` and
`exhausted_import_call_is_unknown` described the old all-or-nothing rule and
are replaced by `select_within_budget` and `withheld_import_call_is_unknown`.
`QCore.CrossUnitSummary` (rule 40's conditions) is unchanged: the type check
and every condition apply to the held homes exactly as before.

## Fixtures

| Test | What it shows |
|---|---|
| `legacy_c::cross_unit::tests::scope_vectors_hold` | the ten scope vectors (Lean-bound): spelled, member-named, local precedence, at budget, subset by function set then name, unprobed homes, unprobed homes outside the budget, tie-break, an over-budget set never held whole |
| `legacy_c::cross_unit::tests::selection_is_deterministic` | the subset is independent of the order of the function sets |
| `legacy_c::cross_unit::tests::function_uses_close_over_local_callees` | per-function uses close over local callees; a prototype is no definition |
| `cross_unit_links::tests::a_caller_over_the_budget_holds_whole_function_sets_first` | 25 homes, budget 24: `small`'s `f0` held, then by name, `f9` withheld; `small` Safe, `big` Unknown on `f9`; a unit whose only withheld home it never calls is Safe and its certificate replays the 24 held homes; the check still refuses 25 |
| `cross_unit_links::tests::a_caller_over_the_budget_imports_its_subset_of_derived_homes` | the same under a derived resolution, through rule 40's admission |
| `cross_unit_links::tests::a_member_named_home_the_caller_never_declares_is_not_probed` | a member-named home without a token: held under a stated map, unprobed (not held, not refused) under a derived one; a K&R declaration keeps it in scope and it refuses "no prototype" |
| `cross_unit_links::tests::a_caller_holds_and_replays_only_the_homes_it_may_reach` | unchanged: within budget the scope is as before |

## Gate

`python3 scripts/gate_scope.py --base 0f6e17118 --out private/gate-1`
(full mode: the diff reaches `qcore-c-gateway`) at `4722c05d6`; console
`/media/Data/GitHub/quirin-lang/private/link-limiters-logs-ae6c/gate-1.console.log`,
record [`gate.json`](gate.json). Exit 0, 8 steps passed:

| Step | Exit | Counts |
|---|---:|---|
| submodules | 0 | 1 submodule |
| `cargo fmt --all -- --check` | 0 | |
| `cargo clippy --all-targets -- -D warnings` | 0 | |
| Rust suite (`run_test_binaries.py`, rust-suite slot) | 0 | 638 units, 9,512 passed, 0 failed, 17 ignored (3,793 s) |
| `reevaluation_probe`, `reevaluation_followup` | 0, 0 | |
| Lean (`QCore.CrossUnitReplayBudget`, `AuditCrossUnitReplayBudget.lean`) | 0 | 1 audit, 18 theorems, axioms only `propext`/`Quot.sound` |
| Python (`unittest discover -s scripts/tests`) | 0 | 761 tests |

The provider gate does not apply (no `providers/` change).

## Measurement

The integrator runs it, then reruns the user's `current` with the artifact
and `--monkey-linked`:

```text
target/release/quirin-audit --plugin-path target/release --output <dir> --timeout-ms 36000000 measure --inputs crates/quirin-audit-plugin-measure/inputs/live.json --inputs-base /media/Data/GitHub/quirin-lang --side base=baseline:497bdeb906e1ad29a94f54079991f0b62bd701df --side tip=rev:0ae403bc0 --declared docs/reports/link-standing-import-limiters/PREDICTION.md
```

## Integrator measurement (2026-11-16)

Declared measurement, base `rev:0f6e17118` (the card's base; r8–r12 landed after the newest published baseline `497bdeb90` without a publish, so the card base isolates this card), tip `rev:0ae403bc0`, run from the main checkout (`private/measure/link-limiters-20261116T055100Z`): exit 0, all three routes complete, no unmatched rows, declared result **match**.

- corpus: 0 rows moved; 0 rows changed content only.
- system: 0 rows moved; 0 rows changed content only.
- slice: 0 rows moved; 0 rows changed content only.

[Comparison](integration/native-comparison.json.gz) (gzip of
`native-comparison.json`, sha256
`fdf79726490a31623f9f1ec7f8923e3e8568140fe99c96d8ca4ed0a7c49b09d7`).

The pinned routes run the stated eight-home map, which never exceeds the replay budget, and the stray filter applies only to derived (rule 40) runs, so nothing moves. Its effect is on the user's `current` run with the link artifact, which the integrator reruns after landing. Landed through integrate/r15 with CURRENT-CONTEXT-NEW-TRIAGE (phase 1), LINK-STANDING-IMPORT-LIMITERS and OPENSSL-PROVIDER-PLUGIN; all three merged without conflict.

The integrate/r15 landing gate record is [OPENSSL-PROVIDER-PLUGIN's](../openssl-provider-plugin/integration/gate.json) (full gate on the merged tree 96bf2cc25 plus a targeted rerun; see its note).
