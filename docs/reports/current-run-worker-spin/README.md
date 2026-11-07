# CURRENT-RUN-WORKER-SPIN

Card base `13337d0af` (origin/main). Brief: round 24, the first prerequisite
of the user's decision on BODY-SUMMARY-REUSE-LINK-STANDING ("Option A
(rec.)"). The follow-up: a `quirin-audit current` run of Fluent Bit 046accf6
with the 165-home experiment link map
(`docs/reports/body-summary-reuse/link-map-experiment.json`) ran for 10 h 18 min
with three workers at 100 % CPU, no I/O and 14.4 GiB RSS. It was stopped. The
same run with no map finishes in 1 h 39 min.

## Diagnosis

This section was committed before any code change.

**The route does not loop. Its cost is super-linear: `callers × homes`
whole-unit analyses.** Every loop on the path is finite:

- the refutation two-phase run (`refuted_contracts::run_two_phases_with_bounds`)
  analyses every unit at most twice;
- the home export scans every unit once;
- the import phase (`current_route.rs`, after `links.select`) re-analyses
  every unit once;
- `check_imports` iterates over a finite home list.

No fixed point or re-queue involves the imports, and an imported-home chain
is refused, so no import depends on another.

The hot path, per caller unit, in the import phase:

1. `projection::project_prepared_unit_into_with_cross_unit` calls
   `legacy_cross_unit::for_caller`. That function dropped only the homes the
   caller defines itself, so **every caller held every run home** (all 165
   names, less its own).
2. `analyze_preprocessed_unrestricted_c11` calls `check_imports`, which
   replays each held home. Each replay runs
   `analyze_legacy_c_translation_unit_selected_partial` over the home's
   *whole* translation unit, with every definition selected, so it costs
   what that unit's own analysis costs. A home unit that holds several map
   names is analysed once per name.
3. For a `Safe` caller, `check_legacy_safety_certificate` calls
   `check_imports` again over `certificate.cross_unit_homes`, a second full
   replay.

So the import phase costs Σ over callers of Σ over held homes of
T(home unit). With U callers and H homes, that is U × H home analyses in
place of U unit analyses. It grows linearly in the homes for each caller
and as `units × homes` for the run.

**Measured growth.** The run's own unit times give T. The tip run below
prints a time for every unit of the first phase, and that phase is the
base code (no import is installed before the import phase). Those times,
summed by `scripts/base_cost.py` over every caller and every map name
(an upper bound: all 165 names are counted), give:

| | CPU | on 3 workers |
|---|---:|---:|
| First phase: 186 units, each analysed once | 0.9 h | 0.3 h |
| Base import phase, replays at issue only | 306.6 h | 102.2 h |

One home unit dominates. `librdkafka/.../rdkafka_admin.c` takes 627 s to
analyse and holds 4 map names, so every caller replays it 4 times: 128.9 h
CPU. Next come `rdkafka.c` (146 s × 7 names, 52.6 h), `rdkafka_request.c`
(152 s × 4, 31.3 h) and `rdkafka_partition.c` (50 s × 9, 23.2 h). The
stopped run's 10 h 18 min, 25.6 h CPU, with no output after the pre-pass,
was early in the import phase.

**Smallest reproducer.** A map with one home in `rdkafka_admin.c` makes
every one of the 186 callers replay a 627 s analysis: 186 × 627 s / 3
workers ≈ 10.8 h. That alone matches the observed hang. A map of a single
home in any multi-minute unit reproduces the symptom. The direct base
measurement is a selection of two cmetrics callers plus `rdkafka.c`, with a
one-home map naming one `rdkafka.c` function; it is reported with the
other runs below.

**Memory.** Every `CrossUnitHomeEvidence` carries a full clone of the
contract environment, plus the home's raw, marked and preprocessed source.
`for_caller` cloned every home per caller, per worker, and a `Safe`
certificate clones its homes again. Three workers each analysing
`rdkafka_admin.c` at the same time, plus 165 environment clones per worker,
fit the 14.4 GiB observed. The member-facts pre-pass alone peaks at
6.8 GiB (map-free selection runs below), so the map-free run's 7.5 GiB is
already above the 7 GiB admission.

**Output.** The route prints a unit's console lines only when every phase
has ended, so nothing showed where the run was.

## Fix

Rule 4c is amended in place ([4c](../../agents/rules/4c.md)). There is no
new rule id. The changes are in `qcore-c-gateway` (`legacy_cross_unit.rs`,
`member_run_facts.rs`) and `qcore-audit` (`projection.rs`, `current_route.rs`,
`route_progress.rs`, `report.rs`, `pinned_route.rs`, `current.rs`).

1. **A caller holds only the homes it may reach** (`for_caller`, pure core
   `scope`). A home is reachable when the caller does not define the name
   and either:
   - uses it, meaning it appears inside any brace or after a file-scope
     `=`. A file-scope prototype is not a use, because a library header
     declares every function of its library; or
   - some member's run closure (rule 12n) can deliver it: a function
     designator stored into a member, or passed to a setter whose formal a
     member stores, transitively (`RunMemberFacts::named_functions`). This
     is the only way a member closure can deliver a function the caller
     never spells. An address-taken function only opens a closure, so it is
     not a candidate.

   Every other import is absent from the caller's environment. Its lookup
   finds no summary and the call stays uncontracted.
   `QCore.CrossUnitReplayBudget.restricted_lookup_is_full_or_absent` shows
   that holding fewer homes never supplies a summary, and
   `consulted_lookup_unchanged` shows that every name the analysis can
   consult reads what the full table supplied.
2. **Budget.** A caller holds at most `REPLAY_BUDGET_PER_UNIT` = 24 homes. A
   caller that may reach more holds none:
   - its calls stay `Unknown` (`exhausted_import_call_is_unknown`);
   - its unit row carries `cross_unit_imports_withheld` with the reason
     (`cross-unit replay budget exhausted: N reachable home(s) exceed the
     per-unit budget of 24; no import held`);
   - its console line repeats that reason.

   `check_imports` refuses a set above the budget before any replay, so the
   certificate check is bounded as well. The run states the budget as
   `replay_budget_per_unit` in its `cross_unit_link_resolution` assumption
   (both routes). The import phase therefore costs at most 24 home analyses
   per caller per check (`replay_work_bound`: `callers × budget`), linear in
   the units.
3. **One replay per home unit and process.** `check_imports` still runs
   every per-home check for every caller. The W4 re-run of a home unit, its
   rows reduced to what the check reads per function, now comes from a
   process-wide table, keyed by values this check recomputed from the home's
   bytes:
   - the input identity: raw, preprocessed and scope-evidence digests and
     the request digest;
   - the environment digest;
   - the dialect;
   - the selected names.

   Equal keys are therefore equal inputs. The requested entry fills only the
   summary's `entry`, which no check reads, so homes of one unit share the
   replay. The replay is recorded once, in a fresh recording
   (`contract_dependencies::captured`). Its primary stream is a digest no
   stream has, so a home reading site is a foreign stream, as it was inside
   the caller's recording. At every check, that record is written into the
   caller's recording once per held home (`absorb_record`: logs and sets
   joined, counts added). A caller's rule 13f dependencies are therefore
   what a replay inside its own recording wrote before. The import phase
   costs one analysis per distinct home unit, plus per-caller checks; it no
   longer costs `callers × homes` analyses.
4. **Progress.** `route_progress` prints on standard error when the link
   map is selected, or when `QCORE_AUDIT_PROGRESS` is set to a nonempty
   value. It prints:
   - a begin and end line per phase: pre-pass, units under the loaded
     contracts or the refutation table, home export, import;
   - a `start` and a `done` line, with its wall time, per unit, so a stuck
     unit is the one with no `done`;
   - each caller's scope in the import phase: reachable homes, how many of
     them only the member facts name, and the held homes by name.

   With neither switch, a map-free run is byte-identical. The home-export
   scan now runs on the route's workers and folds in task order.

The pinned route shares the projection code, so it gets the same scope and
budget. It prints no progress.

## Lean

`QCore.CrossUnitReplayBudget` (new; audited by
`proof/AuditCrossUnitReplayBudget.lean`, no placeholder, no project axiom)
models `scope` and the restricted lookup:

| Theorem | Statement |
|---|---|
| `held_within_budget` | a caller holds at most `budget` homes |
| `replay_work_bound` | a run's held homes per check are at most `callers × budget` |
| `held_is_a_run_home` | a held home is a run home the caller does not define |
| `exhausted_holds_none` / `within_budget_holds_reachable` | the two branches of the budget |
| `restricted_lookup_is_full_or_absent` | a restricted lookup is the full lookup or no import |
| `exhausted_import_call_is_unknown` | at exhaustion an import-only call reads `Unknown` |
| `consulted_lookup_unchanged` | within the budget, every consulted name reads what the full table supplies |
| `scope_vectors_hold` | the six `scopeVectors` |

`SCOPE_VECTORS` in `legacy_cross_unit.rs` is the same table, and
`scope_vectors_hold` (gateway unit test) runs it through the Rust `scope`.
The Rust/tool boundary is the hypothesis of `consulted_lookup_unchanged`:
the caller's analysis looks up an import only for a name its unit uses
outside a file-scope prototype, or a name a member closure delivers
(`RunMemberFacts::named_functions` is the union of every closure's reading).
The equality runs below check that hypothesis on two maps.

## Tests

In `crates/qcore-audit/src/cross_unit_links.rs`:

- `a_caller_holds_and_replays_only_the_homes_it_may_reach`: three exported
  homes and a caller that calls one. The caller holds that one home, its
  import and its identity, is `Safe`, and the certificate replays one home
  and checks. A home that only a member store names is held as well; one
  that is only address-taken is not.
- `a_caller_reaching_more_homes_than_the_budget_holds_none`: 25 exported
  homes, all called. The caller is withheld with the budget reason, holds no
  import and reads `Unknown`. The check refuses the full 25-home set with
  the budget reason before any replay.

`scope_vectors_hold` is in `crates/qcore-c-gateway/src/legacy_cross_unit.rs`.

## Runs

Every run used the user's `current` arguments from
`resource-diagnostics.invocation.json` with this worktree's profile. The
scripts are under `scripts/`. Each went through `gate_slot.py audit
--memory-budget 7G` (three workers) with `--wait-record` and a `timeout`.
"Run" is wall time minus the slot wait. "Peak" is the receipt's
`max_rss_kib`, the largest single descendant. The host was shared with
other cards and the integrator throughout, so wall times carry contention.
Private outputs are in `private/exp/<run>/`.

The cmetrics selection is 7 cmetrics files plus the Monkey units:
36 configurations. Its maps hold k of the 18 homes in those files.

| Run | Binary | Map | Exit | Run | Peak | Import phase |
|---|---|---|---:|---:|---:|---:|
| `b3-k0` | base | none | 0 | 594 s | 6.83 GB | n/a |
| `t3-k0` | tip (scope only) | none | 0 | 815 s | 6.83 GB | n/a |
| `b3-k18` | base | 18 homes | 0 | 1,662 s | 6.81 GB | not printed |
| `t10-k18` | tip (shared replay, unrecorded) | 18 homes | 0 | 648 s | 6.80 GB | 34 s |
| `t11-k18` | final tip (`ed86172af`) | 18 homes | 0 | 624 s | 6.86 GB | 29 s |
| `base-r1` | base | 1 home in `rdkafka.c` | 0 | 3,326 s (7,375 s user CPU) | 8.82 GB | not printed |
| `t2-full-165` | tip (scope, budget) | 165 homes | stopped at 87/186 import units | | 9.49 GB | > 1.4 h at 87/186 |
| `t7-full-165` | tip (+ precise member set) | 165 homes | stopped at 87/186 import units | | | > 1.5 h at 87/186 |
| `t9-full-165` | tip (+ shared replay) | 165 homes | **0** | **7,895 s (2 h 12 min)** | **9.38 GB** | **3,302 s** |

`t3-k0` is the tip with no map. Its report is byte-identical to `b3-k0`'s,
and its standard error carries no progress line: without a map the route is
unchanged. `base-r1` is the reproducer: two cmetrics callers, `rdkafka.c`
and the Monkey units (32 configurations), with a one-home map naming
`rd_kafka_consumer_group_state_code`. Every caller replays the 146 s
`rdkafka.c` analysis, and the run takes 55 min against about 10 min of
pre-pass. `t2` and `t7` each show the scope alone was not enough: their
callers still replayed every held home, and heavy librdkafka callers with
several homes in `rdkafka.c`, `rdkafka_request.c` or `rdkafka_admin.c` took
minutes each. Both were stopped by their captured task. The shared replay
(`t9`) closed that gap.

## Acceptance: the 165-home map

`t9-full-165` used binary `fdbc7a485` and
`QCORE_CROSS_UNIT_LINK_MAP=link-map-experiment.json` (165 homes, experiment
only, never adopted). The final tip adds two commits after `fdbc7a485`:

- `ed86172af` writes the shared replay's record into each caller's rule 13f
  recording. It touches dependency evidence only; nothing in the analysis
  reads that record (rule 13f).
- `aa09196b9` boxes a record for Clippy.

The acceptance results:

- **Exit 0 after 2 h 12 min wall** (12,066 s user CPU). Phases: pre-pass
  568 s; units under the loaded contracts 2,700 s; home export 1,315 s
  (143 exports); import 3,302 s. The stopped base run spent 10 h 18 min
  without finishing.
- **Peak RSS 9.38 GB, above the 7 GiB admission.** This is a finding, not
  a regression: the map-free base run peaked at 7.5 GiB, and the
  member-facts pre-pass alone reaches 6.8 GB on a 36-unit selection. The
  base map run reached 14.4 GiB.
- **No caller exhausted the budget.** The largest reachable count was 24,
  reached by 2 of 186 callers. In 165 callers, 8 of the held homes are
  named only by member closures: the `mk_*` functions and
  `rd_kafka_topic_partition_list_destroy_free`. The rest are homes the
  caller uses.
- **Unit statuses are unchanged** against the map-free run (the user's
  report of the same code, equal to the base run row for row):
  10 Safe / 8 provisional Unsafe / 164 Unknown / 4 data-only.
- **291 function rows change** ([`acceptance-rows.json`](acceptance-rows.json),
  `scripts/import_justify.py`):

  | Before → after | Rows | Kind |
  |---|---:|---|
  | Unknown → Safe | 16 | each loses an uncontracted-import leaf |
  | Unknown → Unknown | 259 | a held import's leaf is gone (deeper leaves may appear) |
  | Unknown → Unknown | 14 | consequential: an alias or branch-condition leaf over the import call |
  | Unsafe → Unsafe | 2 | an import leaf is gone |

  Every changed row is in a unit that held an import, and none moves away
  from `Safe`. The two runs share contracts and refutation table (the
  import phase is refused otherwise), so the held imports are the only
  difference between them. In total, 411 `call to uncontracted function F`
  leaves with F imported are resolved, and 174 deeper leaves appear in 105
  rows. Every `Safe` gain is conditional on the map's link premise, as
  rule 4c states. The 16 newly Safe rows include all 7 cross-unit
  sole-blocked functions of BODY-SUMMARY-REUSE's census:
  - `ctxTypeID_to_size`;
  - `cio_chunk_hash`;
  - `cio_chunk_is_up`;
  - `mk_mq_send`;
  - `rd_kafka_share_consumer_closed_err`;
  - `net_plain_writev`;
  - `rd_kafka_transport_ssl_threadid_cb`.

## Equality on a reduced map that finishes at base

`b3-k18` (base) against the tip on the same 18-home cmetrics map
(`scripts/compare.py`):

- **Verdict and leaf differences: 0** for the scope-only tip (`t8-k18`),
  for the shared replay with an unrecorded replay (`t10-k18`), and for the
  final tip (`t11-k18`, below).
- Evidence fields differ:
  - `hypothesis_context.evidence_version` (432 rows): the caller's
    environment now holds only its imports.
  - `contract_body_evidence` (36 units): at base, a caller replayed every
    run home inside its own rule 13g recording, so its unit row listed the
    home units' body validations (for example `cmt_counter.c` listed
    `cmt_summary_get_sum_value` and five more) as its own. A caller now
    replays only the homes it holds.
  - Reading-site fields (1 row).
- `t10-k18` also lost 10 rows' `contract_dependencies`, because its replay
  ran unrecorded. In `cmt_counter_add` the import `cmt_metric_add` and its
  home's declarations dropped out. The final tip records the shared replay
  once and writes that record into every caller's recording, which
  restores them: `t11-k18` has no `contract_dependencies` difference from
  base. Its only differences are the three evidence fields above.

## Prediction and measurement

See [PREDICTION.md](PREDICTION.md), committed with the diagnosis. All three
live routes run with the eight-home map, so the prediction is no verdict or
leaf move on any of them. The user's map-free `current` run is unchanged
byte for byte (`t3-k0`).

The measurement, which the integrator runs:

```sh
target/release/quirin-audit --plugin-path target/release --output <dir> --timeout-ms 36000000 measure \
  --inputs crates/quirin-audit-plugin-measure/inputs/live.json --inputs-base /media/Data/GitHub/quirin-lang \
  --side base=rev:13337d0af --side tip=rev:aa09196b9 \
  --declared docs/reports/current-run-worker-spin/PREDICTION.md
```

## Follow-ups

- `CROSS-UNIT-MEMBER-CLOSURE-SCOPE`: the member-named part of a caller's
  scope is the union of every run closure. In 165 of 186 callers it holds 8
  homes the caller's members cannot deliver.
- `CURRENT-RUN-RSS-ABOVE-ADMISSION`: `current` peaks at 6.8 to 9.4 GB
  against its 7 GiB admission, mostly in the member-facts pre-pass. Each
  home evidence also carries a full clone of the contract environment.

## Records

- `scripts/`:
  - `base_cost.py`: the base cost estimate;
  - `runs.py`: run summaries;
  - `compare.py`: row equality;
  - `import_justify.py`: per-row justification of the acceptance run,
    whose output is [`acceptance-rows.json`](acceptance-rows.json);
  - `import_moves.py`: the earlier, leaf-only classifier.
- [`gate.json`](gate.json): the card gate.
- [`integration-amendments.json`](integration-amendments.json). It retires
  `CURRENT-RUN-WORKER-SPIN` and appends to `BODY-SUMMARY-REUSE-LINK-STANDING`,
  both registered by integrate/r4's BODY-SUMMARY-REUSE records. It checks
  clean against a tree with those records applied. On this card's base the
  retire is refused because the id is not registered there yet.
- Private run outputs: `private/exp/` in the card worktree.

## Gate

`python3 scripts/gate_scope.py --base 13337d0af --out private/gate-4` at
`aa09196b9`: **exit 0**, mode full (the diff reaches `qcore-c-gateway`).
The record is [`gate.json`](gate.json).

| Step | Exit | Counts |
|---|---:|---|
| submodules | 0 | 1 submodule |
| fmt | 0 | |
| clippy | 0 | |
| rust-suite | 0 | 612 units, 9,246 passed, 0 failed, 16 ignored (10,976 s, including the slot wait) |
| reevaluation probe / follow-up | 0 / 0 | |
| lean (all) | 0 | 411 audits |
| python | 0 | 625 tests |

An earlier attempt (`private/gate-3`, at `ed86172af`) failed only at Clippy
(`large_enum_variant` on `Replayed`). It passed the rust suite (9,246/0),
Lean (411) and Python (625). `aa09196b9` boxes the record. Two attempts
before that were stopped before their suites ran, so that audit slots could
go to the acceptance run.

## Integrator measurement (2026-11-07)

Declared measurement, base `rev:13337d0af`, tip `rev:aa09196b9`, run from the
main checkout (`private/measure/worker-spin-20261107T050627Z`): exit 0, all
three routes complete, declared result **match** on corpus, system and slice.
No row or unit moved. Corpus: 186 rows changed content, and in every one only
`contract_dependencies` differs (no leaf added or removed, status unchanged),
which is the prediction's evidence-only change (callers no longer record the
dependencies of homes they never read); the other 4,648 corpus rows and every
system and slice row differ in environment fields only.
[Comparison](integration/native-comparison.json.gz) (gzip of
`native-comparison.json`, sha256
`e48c64f0fa2e8c1a4161a619315fa220ebab89a0d6d2fdd246b62de2a0fdb096`).

Landing merge: `crates/qcore-audit/src/pinned_route.rs` conflicted with
CORPUS-GROUP-CHECKPOINTS, which moved the cross-unit premise into
`cross_unit_premise_of`; `replay_budget_per_unit` is written there.
`crates/qcore-c-gateway/src/member_run_facts.rs` conflicted with
CALLBACK-MEMBER-NON-C-UNITS (independent additions; both kept).
`named_functions` reads read units' stores only, which stays complete under
that card's reading: a scoped C++ unit stores nothing into a member it closes,
and a member an unread unit opens keeps its call Unknown.
