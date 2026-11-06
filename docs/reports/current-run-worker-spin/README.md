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
