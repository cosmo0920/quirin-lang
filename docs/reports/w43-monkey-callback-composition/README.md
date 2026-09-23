# W43 Monkey callback Cycle 18 composition

Date: 2026-09-23. Card: `MONKEY-CALLBACK-CYCLE18-COMPOSITION`. Base:
`7e8d59ef26fc5f2929a674a8e23638f465eeb61a`. Input baseline: the
[W42 callback accounting](../w42-monkey-callback-coverage/README.md), the
[observed](../cycle18-observed-integration/README.md),
[priority-one](../cycle18-priority1-integration/README.md) and
[broader](../cycle18-broader-integration/README.md) Cycle 18 integrations, and
the frozen [bounded aggregate](../cycle18-broader-integration/aggregate.json).

## Result in one paragraph

**0 of 3 callbacks are Cycle-18-composed; all three stay individually
Unknown.** The seven obligations the bounded aggregate leaves unresolved are
now measured per callback and per source path: **51 relevant
callback/path/obligation cells over 3 callbacks and 8 paths, 5 discharged and
46 open before, 17 discharged and 34 open after**. This card discharges, from
the pinned compile task, **initialization reachability for all three callbacks**
(8 cells) and **natural activation for `cb_timeout` and `cb_wait_2_timeout`**
(4 cells), and composes **all three `cb_timeout` source paths as Cycle 18 event
programs on the close-success branch**; that last result narrows but does not
discharge its path-coverage obligation. No callback can compose against this
pinned source: every one keeps caller null handling and registration
liveness open, and every one's registration context retains source-preserving
provisional-Unsafe scenarios (`base_creation_failure`,
`fired_allocation_failure`, and for `cb_wait_2_timeout` also
`timer_creation_failure`). Full composition is **not reachable** in this card
without a production caller inventory and without a change to the Monkey
source or a proof that its unchecked creation/registration results cannot
fail. `MONKEY-CALLBACK-CYCLE18-COMPOSITION` is **narrowed, not retired**; two
residuals are registered.

## Scope

In scope: the three callbacks the forced-Libevent Monkey profile selects
(`cb_event`, `cb_timeout`, `cb_wait_2_timeout`) at Fluent Bit revision
`66910c10a4d7eafa810b84229cf2dcf7b0f26f97`, their eight imported source paths,
their registration wrappers and loop creation. Untouched: the C gateway
projection (card W43-RETURN-EXPRESSION), every provider-owned and shared
contract (the aggregate policy `cycle18_composition.qcore` included), the
185-unit corpus baseline and its frozen artifacts, and every earlier Cycle 18
report. No pinned expectation changed; the only derived count that moves is
the Lean audit file count (165 to 166).

New code:

- `crates/qcore-verification/src/cycle18_callback_obligations.rs` — the
  obligation table, three source-bound dischargers, the `cb_timeout` path
  composition and the per-callback decision rule;
- `crates/qcore-verification/tests/unit/cycle18_callback_obligations.rs` — 17
  unit tests, most of them source-level negative controls;
- `crates/qcore-audit/src/cycle18_composition.rs` — the existing
  `cycle18-composition` route's leaf preparation is shared (behaviour
  preserved, see [parity](#aggregate-recheck-and-parity)) and a new configured
  route `quirin-audit cycle18-obligations CONFIG.json` is added;
- `proof/QCore/Cycle18CallbackObligations.lean` with 22 theorems, audited by
  `proof/AuditCycle18CallbackObligations.lean` and imported by `QCore.lean`.

## Step 1 — measurement before any discharge

The route first rechecks the aggregate from its retained evidence (every leaf
audit rerun, no serialized leaf report consumed) and derives, for each
callback, which obligations govern which of its imported paths. Relevance is a
source fact of the callback's own summary, fail-closed:

| Obligation | Relevant to | Why (source fact) |
|---|---|---|
| `initialization_reachability` | every path of all three | every callback is registered on a context returned by `_mk_event_loop_create` |
| `caller_null_handling` | every path of all three | each registration wrapper dereferences the loop context |
| `caller_size_bounds` | `cb_event` paths 0–3 | only paths that read the `fired` array (or call an unadmitted function) |
| `allocation_authority_provenance` | `cb_event` 0–3, `cb_timeout` 0–2 | paths that dereference, write through or release the callback argument; `cb_wait_2_timeout`'s sole `return` uses none |
| `registration_liveness` | every path of all three | every activation depends on the event registration |
| `natural_activation_reachability` | every path of all three | every path is entered by a Libevent activation |
| `all_path_execution_coverage` | every path of all three | every imported path must be composed |

A provisional-Unsafe scenario of the aggregate blocks a callback when its
source function precedes that callback's registration: the loop-creation
scenarios block all three, `timer_creation_failure` blocks
`cb_wait_2_timeout`; an unrecognized scenario would block every callback.

**Before** (the rechecked aggregate, no discharger applied):

| Callback | Paths | Relevant cells | Discharged | Open obligations | Blocking scenarios |
|---|---:|---:|---:|---|---|
| `cb_event` | 4 | 28 | 4 | initialization, caller null, caller size, allocation authority, registration, natural activation | base creation, fired allocation |
| `cb_timeout` | 3 | 18 | 0 | initialization, caller null, allocation authority, registration, natural activation, all-path coverage | base creation, fired allocation |
| `cb_wait_2_timeout` | 1 | 5 | 1 | initialization, caller null, registration, natural activation | base creation, fired allocation, timer creation |
| **total** | **8** | **51** | **5** | | |

The five cells discharged before this card are execution coverage by joined
bounded native families (observation, not proof): four controlled-activation
executions over `cb_event`'s four paths and one observed execution of
`cb_wait_2_timeout`'s sole path. What each open obligation needed, at this
point, was the aggregate's own text (`aggregate_unresolved_obligations` in
[the report](obligation-report.json)).

## Step 2 — what was discharged and how

### Initialization reachability (all three callbacks, 8 cells, source fact)

`discharge_initialization_reachability` takes the independently imported
`_mk_event_init` and `_mk_event_loop_create` summaries of the pinned
translation unit (`mk_core/mk_event.c` under its pinned compile task and the
selected Unix profile). Under that profile `_mk_event_init` has **one path, a
single `return 0`**: the `#ifdef _WIN32` body is not in the preprocessed unit.
Every loop-create path begins with the one argument-free call and tests its
result exactly once with `!= 0`. The discharger evaluates each guard against
the constant: **path 0** — the only path whose sole call is the
initialization call, i.e. the initialization-failure branch — requires
`0 != 0` taken and is infeasible; paths 1–3 remain feasible. Production
digests: `_mk_event_init` `ced6a224…`, `_mk_event_loop_create` `3a76dc34…`.

It refuses (and the obligation stays open) when the callee has a failing
return, returns a non-literal, has any operation other than one constant
return, when the guard is one the constant satisfies, when either summary is
not a Wrapper import of the pinned revision under the aggregate's profile, or
when the arguments are swapped. Soundness is modelled by
`refuted_failure_branch_is_never_taken`.

### Natural activation (`cb_timeout`, `cb_wait_2_timeout`; 4 cells, source fact)

`activation_flag_dependence` scans every operation of every imported path for
the activation-flags parameter. **No operation of `cb_timeout` or
`cb_wait_2_timeout` reads it**, so every activation, natural or controlled,
selects its path from argument state alone (`evutil_socket_geterror(fd)` is
`errno` on Unix; the socket parameter is not read either). This discharges the
obligation in the direction safety needs — no natural activation can select a
path or entry state that the composed evidence does not cover — and claims
nothing about whether or how often a natural activation occurs. For `cb_event` the
flags are read **only** by single-bit guards on `EV_READ` (2) and `EV_WRITE`
(4) whose four combinations are exactly its four paths; any other use of the
flags — flowing into memory or a multi-bit guard — refuses. That settles
`cb_event`'s path selection but not its entry state (see Step 4), so for
`cb_event` the result is recorded as a partial discharge only.

### `cb_timeout` path composition (3 paths, model proof on one branch)

`compose_timeout_source_paths` maps each call of each `cb_timeout` path to a
lifecycle step, checking arguments as well as names: `send` of
`ev_map->pipe[1]`; `__errno_location` and `perror` of a string literal as
effect-free; `evutil_closesocket(ev_map->pipe[1])` as the write-endpoint
close; `event_del(ev_map->event)` and `event_free(ev_map->event)` as
current-callback delete and free; `mk_mem_free(ev_map)` as the map drop. Any
other call, any write through memory or a partial/reordered teardown refuses.
Paths 0 and 1 are teardown paths (equal to the sealed teardown projection's
indices), path 2 is the non-teardown `send` success path.

The event program's registration comes from `_mk_event_timeout_create`'s
imported path 3, not from a declaration: `event_new(ctx->base, -1,
EV_TIMEOUT | EV_PERSIST, cb_timeout, ev_map)` with flags evaluated to 17, the
map a fresh `mk_mem_alloc_z` result on that path, and the created event stored
in `ev_map->event`. The kernel's `check_event_program` then checks nine
programs:

| Program | Close outcome | Result |
|---|---|---|
| teardown path 0; teardown path 1 | success | accepted (15 operations each) |
| path 2 then teardown path 0; path 2 then teardown path 1 | success | accepted (18 operations each) |
| the same four | failure | refused: `L03DescriptorCustody` on the write endpoint |
| path 2 then loop exit (liveness witness) | success | refused: `L03`, `L07`, `L11` |

So **every `cb_timeout` path composes on the close-success branch**, including
a non-teardown activation followed by a teardown activation of the persistent
timer. The **close-failure branch does not**: the source ignores
`evutil_closesocket`'s result, and a failed close leaves the descriptor
indeterminate in the Cycle 18 close vocabulary (card O03). The
`all_path_execution_coverage` cell stays open, narrowed to that branch. The
liveness witness shows that release after path 2 needs a later teardown
activation before the base is freed; that is recorded under registration
liveness.

The same pass performs a registration-local escape check: the map is handed
only to `event_new`, the created event only to `event_add` and the map's
`event` field, and neither is returned or has its local's address taken. This
is recorded as a partial discharge of `cb_timeout`'s argument-ownership
provenance, not a discharge, because it does not establish whole-program
non-use of Libevent's callback-argument accessors.

### Registration sites (source facts, no discharge)

`registration_sites` records, for every wrapper path that creates a callback's
event, whether the creation and registration results are guarded and whether
a failure return leaves a freed event in the map. Production facts:

| Callback | Wrapper paths | `event_new` result | `event_add` result | Other |
|---|---|---|---|---|
| `cb_event` | `_mk_event_add` paths 4–19 | unchecked | checked | — |
| `cb_event` | `_mk_event_update` 24 paths | checked | checked where reached | **16 failure returns leave the freed old event in `ev_map->event`** |
| `cb_timeout` | `_mk_event_timeout_create` path 3 | unchecked | unchecked | `_mk_event_add` result unchecked |
| `cb_wait_2_timeout` | `_mk_event_wait_2` path 2 | unchecked | unchecked | — |

The `_mk_event_update` fact is new: when `event_new` or `event_add` fails
after the old event was deleted and freed, the wrapper returns -1 without
storing the new event, so a later `_mk_event_del` would pass the freed event
to `event_del`/`event_free`. It is a source fact about a failure path, not a
validated upstream defect; it keeps registration liveness open.

## Step 3 — per-callback verdicts after

| Callback | Relevant cells | Discharged | Verdict | Open obligations | Blocking scenarios |
|---|---:|---:|---|---|---|
| `cb_event` | 28 | 8 | **Unknown** | caller null, caller size, allocation authority, registration, natural activation | base creation, fired allocation |
| `cb_timeout` | 18 | 6 | **Unknown** | caller null, allocation authority, registration, all-path coverage | base creation, fired allocation |
| `cb_wait_2_timeout` | 5 | 3 | **Unknown** | caller null, registration | base creation, fired allocation, timer creation |
| **total** | **51** | **17** | **0/3 composed** | | |

The rule, in Rust (`decide`) and Lean (`verdict`): a callback is `Composed`
only when its evidence family joined, no blocking scenario remains, and every
obligation governing a non-empty path set is discharged. The Composed arm is
reachable (`fully_discharged_row_composes`), so the all-Unknown result is a
measurement, not a vacuous rule.

## What stays Unknown, and why

- **Caller null handling (all three).** The production caller inventory of
  `mk_event_loop_create` is a whole-program property: a textual search of the
  pinned tree finds 43 call sites, 11 of them outside test directories (in
  `src/`, `plugins/` and `lib/monkey/mk_server/`), with function-pointer and
  build-selection closure unestablished. That search is an observation, not an
  inventory; no sound complete inventory was built and none is claimed.
- **Caller size bounds (`cb_event`).** Same inventory; the literal caller-size
  checker exists but a per-caller bound is not a universal one.
- **Allocation authority (`cb_event`).** `cb_event` writes
  `ctx->fired[ctx->fired_count]` with **no capacity guard**; exclusive write
  authority needs a bound on activations per dispatch against `queue_size`,
  which the source does not provide. `cb_event`'s natural-activation entry
  state reduces to this same bound.
- **Allocation authority (`cb_timeout`).** Registration-local exclusivity is
  derived; the teardown model's owned-argument mode is still declared, and
  whole-program non-use of Libevent's argument accessors is not established.
- **Registration liveness (all three).** The source facts above: unchecked
  `event_new`/`event_add` results in three wrappers, the stale-event failure
  returns in `_mk_event_update`, and `cb_timeout`'s release depending on a
  later teardown activation before the base is freed.
- **All-path coverage (`cb_timeout`).** The close-failure branch of its
  ignored `evutil_closesocket`.
- **Blocking scenarios.** `base_creation_failure` and
  `fired_allocation_failure` (loop creation) and `timer_creation_failure`
  (`_mk_event_wait_2`) are source-preserving fault-model provisional Unsafe
  results of the priority-one card; this card does not relabel them. They are
  not validated upstream defects.

Eight observed paths are not universal composition, and no observation was
relabelled as a proof.

## Negative controls

**Source-level (unit tests, `cycle18_callback_obligations::tests`, 17
tests).** Each control mutates the pinned fixture source text, reimports it
through the same profile, and checks the discharger refuses:

| Control | Expected | Result |
|---|---|---|
| `_mk_event_init` gains a `return -1` path | refuse (not constant) | refuses |
| `_mk_event_init` returns a call result | refuse (non-literal) | refuses |
| loop creation tests `!= 1` | refuse (failure branch not refuted) | refuses |
| wrong profile digest; swapped summaries; a callback as init | refuse | refuse |
| `cb_event` stores `mask \| flags`; tests `flags & (EV_WRITE\|EV_READ)` | refuse | refuse |
| timeout registration without `EV_PERSIST` | refuse | refuses |
| timeout registration also publishes the map | exclusivity false | false |
| `cb_timeout` with close and delete reordered | refuse (projection mismatch) | refuses |
| model: drop before event free; free without delete; delete without close | refuse | refuse |
| aggregate callback digest or profile substituted | refuse | refuse |
| aggregate family refused | every callback Unknown with the refusal named | holds |
| decision rule: one open cell, one blocker, one refused family | Unknown | Unknown |

The fixture is `compiler/tests/fixtures/monkey/{callbacks,wrappers}.c`; the
production run reproduces the fixture's discharges exactly (same infeasible
path, same flags, same programs).

**Configured (`quirin-audit cycle18-obligations`).** The shipped
[runner](check_obligation_controls.py) writes changed copies of the
configuration and keeps each complete output; [its summary](controls.json):
**8/8 as expected** — baselines before and after exit 0 with byte-identical
obligation payloads; an unknown `discharged` field, schema version 2, the
payload card's different compile database (identity join refuses), and a
report aliasing the aggregate report, the configuration itself or a leaf report
each exit 1 without writing a report. Logs:
`measure/w43-monkey-controls-3/`. The first attempt
(`measure/w43-monkey-controls-1/`, 6/8) used a relative output directory, so
both baselines and one alias control refused for a path-resolution reason;
the harness was corrected to absolute paths and rerun, and that failed
attempt is not counted.

## Lean parity

`QCore.Cycle18CallbackObligations` models the decision rule and the three
dischargers; 22 theorems, all printed by
`AuditCycle18CallbackObligations.lean`, placeholder-free, no project axiom:

- rule: `composed_discharges_every_relevant_obligation`,
  `composed_has_no_blocking_scenario`, `composed_has_joined_family`,
  `open_relevant_obligation_is_unknown`, `blocking_scenario_is_unknown`,
  `fully_discharged_row_composes`,
  `irrelevant_pending_obligation_does_not_block`,
  `measured_wait_callback_is_unknown`;
- initialization: `refuted_failure_branch_is_never_taken` (for every result
  the callee can return, a refuted failure path disagrees with its guard),
  `varying_callee_does_not_refute`, `opaque_callee_path_does_not_refute`,
  `pinned_initialization_branch_is_refuted`,
  `satisfied_guard_does_not_refute`;
- activation: `independent_selection_ignores_flags`,
  `cb_event_classes_partition_every_flags_value` (every flags value selects
  exactly one class);
- `cb_timeout`: `teardown_paths_release_everything`,
  `success_path_preserves_registration`,
  `success_then_teardown_releases_everything`,
  `no_activation_after_teardown`, `close_failure_branch_is_not_released`,
  `success_path_alone_is_not_released`, `reordered_teardown_is_refused`.

The model is an abstract rule model; its correspondence to the Rust is
maintained by review (rule 28), not extraction.

## The verdict contract

Every published result of the route carries it
([`obligation-report.json`](obligation-report.json)): artifact
`fluent-bit/monkey/libevent selected callbacks`; property
`monkey-cycle18-callback-obligations`; semantic profile
`fluent-bit/monkey/libevent` (profile digest `37309028…`); scope the three
callbacks; bounds 3 callbacks, 8 source paths, 7 obligations, 51 relevant and
17 discharged cells; assumptions (faithful import, Rust/Lean correspondence by
review, the enclosing peer close of the `cb_timeout` model, producer-supplied
runtime provenance); evidence version `qcore.cycle18.callback-obligations.v1`;
`universal_safety_certificate: false`.

## Trust boundary (rule 28), kept separate

- **Model proof:** the four accepted `cb_timeout` event programs, checked by
  the Cycle 18 kernel over source-derived operations; the Lean theorems above
  are abstract rule theorems.
- **Artifact binding:** every summary comes from the aggregate's own compile
  task — the route refuses unless revision, compile task, source, profile and
  build digests all equal the wait-failure and timeout leaves' identities, and
  the callback summaries equal the aggregate's rechecked digests. Retained
  native artifacts are bound by the leaf audits, as before.
- **Source coverage:** 3/3 callbacks, 8/8 paths, 51/51 relevant cells
  measured; 17 discharged.
- **Native observations:** unchanged. The five pre-existing family cells are
  observation; no native run was performed or relabelled by this card.
- **Host conditions:** Linux x86_64, Ubuntu clang 21.1.8; the retained native
  evidence keeps its original host conditions.

## Aggregate recheck and parity

[`aggregate-recheck.json`](aggregate-recheck.json) (produced by the shipped
[`aggregate_recheck.py`](aggregate_recheck.py)) compares the frozen
broader-integration aggregate with two reruns over the same retained
evidence: the **unmodified base binary** and **this card's refactored
`cycle18-composition` route**.

- Every verdict-bearing field (verdict, families, scenarios, the seven
  obligations, bounds, scope, policy, revision) equals the frozen report.
- 15 identity fields differ between the frozen report and the base rerun:
  the profile digest (`5507e9f6…` to `37309028…`) and everything that embeds
  it (callback summary digests, compile-task, context and inventory
  digests). The profile's bytes changed in `65157c4c` ("Move contract
  consumers to canonical provider and shared assets"); the source bytes and
  the aggregate verdict did not.
- The refactored route's aggregate equals the base binary's exactly; only the
  configuration digest differs, because the configurations live at different
  paths.

Rerunning the retained configurations needed two disclosed relocations, made
only in run-private copies: the leaf configurations name the pre-move
`contract/qcore/monkey/*.qcore` paths (pointed at the byte-identical
provider-owned files), and the retained timeout run manifest's
`timeout_contract` **role path** names the removed file. The copy
(`relocate_timeout_manifest.py`, kept in each `measure/` run directory)
changes that one path, asserts every digest and every other field unchanged,
and asserts the provider file's digest equals the manifest's recorded
`516fa1fe…`. The retained evidence itself is untouched. The first attempt to
rerun the aggregate without that relocation exited 1 (`No such file or
directory`) and is kept at `measure/w43-monkey-before-1/aggregate-run.log`.

## Validation

All logs are worktree-private, run-scoped files. The standing gate ran from
the driver `measure/w43-gate-1/run.sh` on tip `686f44dc` with a clean tree
(`status-before.txt` empty); each step's exit is written to `<step>.exit` and
nothing is piped. `CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0` were
set for every cargo step; no sanitizer option, deadline or assertion was
changed. The host was shared with other work (1-minute load 7.6–10.4 during the run).
Clang was measured **in this worktree for this run** and is recorded beside
[`gate.json`](gate.json) ([`clang-version.txt`](clang-version.txt),
[`clang-version.exit`](clang-version.exit)).

| Command | Exit | Result | Private log (`measure/w43-gate-1/` unless noted) |
|---|---:|---|---|
| `git submodule update --init --recursive` | 0 | `third_party/libevent` at `d82464a277d0f42703702c4dfd9af6af38595a83` | `submodule.log`, `submodule-status.log` |
| `clang --version` | 0 | `Ubuntu clang version 21.1.8 (6ubuntu1)` | `clang-version.txt` |
| `cargo test --workspace --no-fail-fast` | 0 | **5,122 passed / 0 failed / 12 ignored**; 271 result summaries, 260 running targets, 9 doc-test sections | `cargo-test.log` |
| `cargo fmt --all -- --check` | 0 | clean | `fmt.log` |
| `cargo clippy --all-targets -- -D warnings` | 0 | clean | `clippy.log` |
| `cargo run -p qcore-compiler --example reevaluation_probe` | 0 | **8/8** expectations, 0 failed | `probe.log` |
| `cargo run -p qcore-compiler --example reevaluation_followup` | 0 | **11/11** expectations, 0 failed | `followup.log` |
| `python3 scripts/check_lean_audits.py` | 0 | **166 audits pass** (165 on the base + `AuditCycle18CallbackObligations.lean`); the 22 new theorems depend on no axiom (9), `propext` (9), `propext, Quot.sound` (3) or `propext, Classical.choice, Quot.sound` (1) | `lean-audits.log` |
| `python3 docs/reports/remaining-d02/build_matrix.py . --gate docs/reports/w43-monkey-callback-composition/gate.json` | 0 | matrix regenerated: 166 audits, 278 follow-ups | `measure/w43-gate-2/matrix-1.log` |
| `python3 -m unittest discover -s scripts/tests` (after regeneration) | 0 | **317 tests** | `measure/w43-gate-2/python-1.log` |
| `python3 scripts/build_docs_index.py --check` | 0 | index current | `measure/w43-gate-2/docs-index.log` |

No file under `providers/` changed, so the provider package build/verify
(constraint 18) was not required and was not run; the provider count 123 in
`gate.json` is carried, not remeasured. Rule 16's host-sensitive rows —
`vp04_vp07_vp08_process_lifecycle_and_payload_budgets`,
`a_confined_fixture_cannot_write_outside_the_admitted_paths`,
`memory_growth_beyond_the_declared_limit_is_bounded`,
`a_declared_process_limit_stops_the_fixture_from_forking` and
`closed_input_does_not_mask_descendant_timeout_or_accept_a_partial_request` —
all passed inside the full concurrent run; no isolated rerun was needed and
none is claimed.

Card-specific measurements (all exits recorded beside their logs):

| Command | Exit | Result | Log |
|---|---:|---|---|
| base binary: `quirin-audit cycle18-composition` over the relocated retained configurations | 0 | aggregate Unknown, 3 families, 7 obligations; 15 identity fields differ from the frozen report, all from the profile move | `measure/w43-monkey-before-1/aggregate-run-2.log` |
| card binary `a56817aa…`: `quirin-audit cycle18-obligations` | 0 | the published [`obligation-report.json`](obligation-report.json) | `measure/w43-monkey-after-2/obligations-run.log` |
| card binary: `quirin-audit cycle18-composition` | 0 | aggregate identical to the base binary's | `measure/w43-monkey-after-2/aggregate-run.log` |
| `check_obligation_controls.py` | 0 | 8/8 | `measure/w43-monkey-controls-3/` |
| `cargo test -p qcore-verification --lib cycle18_callback_obligations` | 0 | 17 passed | `measure/w43-monkey-probe/unit-4.log` |

**Failed and incomplete attempts, kept rather than deleted:** the first base
aggregate rerun exited 1 on the removed timeout-contract role path
(`measure/w43-monkey-before-1/aggregate-run.log`, `leaf-timeout.log`); the
first control run scored 6/8 because of the harness's relative output path
(`measure/w43-monkey-controls-1/`); during development the unit suite failed
three tests once (`measure/w43-monkey-probe/unit-1.log`): a model assertion
that the reactor refuses re-activating a non-persistent timer (it does not;
persistence is the separate source fact checked from the registration path,
and the assertion was removed rather than the model changed), a control whose
variant imported as source-Unknown (replaced by an importable non-literal
variant), and a success-branch check that wrongly counted the liveness-witness
program (the witness now has its own purpose). Two Lean builds failed on proof
script errors before `lean-3.log` passed. None of these contributes to a
counted result.

## Remaining work

1. **Registration liveness** (all three callbacks): establish the behaviour of
   each unchecked `event_new`/`event_add` result against the selected backend,
   or record each as a provisional Unsafe fault-model case as the
   priority-one card did for `_mk_event_wait_2`; classify the
   `_mk_event_update` stale-event failure returns.
2. **Production caller inventory** for `mk_event_loop_create` (null handling
   and size bounds): a complete, build-selected inventory with function-pointer
   closure, then per-caller composition.
3. **`cb_event` fired-array authority:** a bound on activations per dispatch
   against `queue_size`; no capacity guard exists in the source.
4. **`cb_timeout`:** the close-failure branch (a declared close-failure
   premise for the selected host, or keep it Unknown), and whole-program
   argument custody.
5. **Aggregate policy synchronisation** and **retained-evidence path drift**:
   the two new residuals below.

## Integrator reconciliation list

1. **Acceptance matrix:** narrow `MONKEY-CALLBACK-CYCLE18-COMPOSITION` (not
   retired: 0/3 composed); register `CYCLE18-AGGREGATE-POLICY-OBLIGATION-SYNC`
   and `CYCLE18-RETAINED-EVIDENCE-CONTRACT-PATHS`. `named_followups` 276 to
   278, equal to the entry count.
2. **Status:** the Monkey callback accounting row gains the per-cell
   measurement (51 cells, 17 discharged) and stays 0/3 composed, 3 explicit
   Unknown. No corpus, CT or conformance figure changes.
3. **Increment:** one paragraph in `wave-41-continuation.md`.
4. **Lean:** audit files 165 to 166 (`AuditCycle18CallbackObligations.lean`).
5. **Aggregate policy:** `cycle18_composition.qcore`'s `aggregate_acceptance`
   still pins `initialization_reachability == "unknown"` and seven unresolved
   obligations. That is correct for the aggregate, which receives no
   initialization summary, but it now under-reports what the obligations
   route discharges. Updating a provider-owned contract was outside this
   card's scope; see the registered residual.
