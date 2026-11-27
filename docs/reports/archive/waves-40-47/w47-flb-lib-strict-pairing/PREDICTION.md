# W47-FLB-LIB-STRICT-PAIRING: frozen prediction

Frozen before any AFTER measurement, on base `fe603d49` (local `main`). The
BEFORE census under the base binary and module reproduces the archived
lifecycle census (`docs/reports/w47-flb-lib-lifecycle/artifacts/census/`) at
all four trees, identical except the module's path string
(`private/sp/census-before-identity.json`). The estimates below are read from
that archive (`lifecycle_stops`, `lifecycle.calls`).

## The strict pair (user decision, 2026-09-29)

`flb_stop` only after a successful `flb_start`, exactly once; after a failed
start only `flb_destroy`; a second stop is a caller violation; `flb_start`
twice without a stop stays forbidden. The clause table replaces the "no
effect" clauses L3-L5 by caller preconditions:

| Clause | Kind | Predicted status at the 90978ac6 export, 046accf6, 66910c10 |
|---|---|---|
| L1 start only from created (a restart is not derived: forbidden) | precondition | in force (restricts) |
| L2 the first stop after a successful start joins that start's thread once | body (`flb_stop`) | **in force**, 2 paths (:1058 / :1077), as before |
| L3 no stop after a stop | precondition | in force (restricts) |
| L4 no stop after a failed start | precondition | in force (restricts) |
| L5 no stop of a never-started context | precondition | in force (restricts) |
| L6 destroy only from created, start-failed or stopped | precondition | in force (restricts) |
| L7 a start that returns nonzero leaves no thread it began unjoined (the body fact behind L6's start-failed admission) | body (`flb_start`) | **held**: the reader cannot tell `mk_utils_worker_spawn` failed, so one nonzero return path leaves `flb_lib_worker` unjoined; the two post-spawn failure returns join it (:984 / :999) |

Restart: no body reading derives that `flb_start` can run after `flb_stop`
(the engine thread's run is not read; `flb_stop` destroys
`ctx->config->cf_main`, :1068-1070), so L1 forbids it (fail-closed).

What `flb_stop`'s body does from the forbidden states (outside the contract,
reported as observations, no longer clauses): every followed path joins, from
never started (identifier no start began), after a stop (second join) and
after a failed start (second join, or an identifier no start began), as the
three refutations did before.

## Callers (the census's test roots; 280 readings at each tree)

* `flb_stop` readings from `failed start` (45) or `created, failed start`
  (5): **50 readings violate** (L4, 5 of them also L5) at each of the four
  trees, over **46 roots** (every root that reaches a stop). 0 violations
  before. `flb_create`'s own stop (:215) stays undetermined (46 readings, no
  creation on its local) and is a reading-only L5 candidate in `flb_create`.
* `flb_start` (50 readings) from created: conforming; `flb_destroy` from
  created (38) or stopped: conforming; a destroy reached after a violating
  stop becomes undetermined on that path (an earlier call was refused).
* **Fixed test** (`90978ac6`): `flb_start` :2559 from created; `flb_stop`
  :2583 from started-ok (conforming, L2) **and from start-failed: a caller
  violation (L4)**, a candidate against the test (fix: guard the stop by the
  start's success); `flb_destroy` :2588 from stopped.
* **Old test** (`90978ac6~1`): `flb_stop` :292 (in `test_tail_ctx_destroy`)
  from start-failed: a caller violation (L4) too.
* No double stop in the test roots; no start after a stop.

## C5 per release

* Every release a refused path reaches was already not discharged: 99 at the
  fixed export, 95 at the previous test and at 046accf6 / 66910c10. **Release
  tallies unchanged**: fixed 97 discharged / 134 not discharged, previous
  97 / 131 (and the system and corpus trees as their archive). Reasons of
  those releases change to the lifecycle refusal; every C4 tally unchanged.
* **Fixed test, failed-start path**: refused (L4 violation): never
  discharges. **Success path** (the paths that keep the pair, decided apart):
  **not discharged** at the 5 releases after the stop, because the status
  branch's unchecked first join at :1058 is not counted (P-POSIX-JOIN's busy
  and cycle exclusions not derived) and the retaining descendants stay
  unresolved; with only L2's one engine join on those paths, no second join.
* **Old test**: `unlink` :286 before the stop: not discharged on every path
  (no join), no refusal on its paths (the stop comes later); `flb_destroy`
  :293: failed-start paths refused; kept paths not discharged (a failed join
  at :1077 is not guarded by the test).
* Negative control: the old test's C1 path is untouched (the loop-owner census
  is not changed) and its C5 stays not discharged.

## Corpus, system and slice

0 status moves on all three routes. The contract text changes only in
comments and the three review reasons (`flb_start`, `flb_stop`,
`flb_destroy`), none of which a leaf quotes: **0 leaf moves** predicted; only
the contract environment's identity (`resource_contract_sources`, per-unit
`inventory_sha256`) changes. No leak or violation added or removed; no
provisional `Unsafe` added.

## Candidates

`W47-ENGINE-THREAD-JOIN-CANDIDATES` (i), (iii), (v) are reclassified as
reproductions of caller-contract violations (library behaviour outside the
contract), (ii) and (iv) unchanged. Registered: the test-side candidates
(stop after a failed start in 46 in_tail roots over 14 stop sites; the
fixed test's :2583), and `flb_create`'s own stop of a never-started context
(:215, reading-only).

## Lean

`QCore.CondvarJoin` section `Lifecycle` rewritten to the strict pair: a stop
admitted only from started-ok; a stop after a failed start and a second stop
refused and never discharging; L2 in force ⇒ a kept path has exactly one
engine join; a destroy from start-failed admitted; the old test shape stays
not discharged. The no-effect clause and withdrawal theorems are retired with
their Rust counterparts.
