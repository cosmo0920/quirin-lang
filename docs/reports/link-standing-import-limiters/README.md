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
