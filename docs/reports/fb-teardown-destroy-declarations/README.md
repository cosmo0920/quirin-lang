# FB-TEARDOWN-DESTROY-DECLARATIONS: rule 30 at a direct call, the parameter guard, and Fluent Bit's destroyers

Card FB-TEARDOWN-DESTROY-DECLARATIONS (wave 48 round 30, card-high). Gateway
rule **30** amended in place; contract specification **C04.19.6** amended;
grammar G05d; Lean `QCore.MemberPathDirectCall`; provider module
`quirin-provider-fluent-bit/contracts/fluent_bit/teardown_destroy.qv`.
Base: local `main` `8b360766c`, rebased onto `aa8d8b030` (section 8; the card started on `cc3e79559` and
fast-forwarded before any code; the census was read there). The census and
the frozen [prediction](PREDICTION.md) were committed (`d2b8fe7e4` before
the rebases, `70cc193d8` after them) before any implementation code. No new word, value or form was needed beyond the
decided guard, so the brief's pause point did not apply.

## 1. User decision (2026-10-16, verbatim)

Decision 2, "Scalar guard": **"Yes, allow it"**. A `when` guard may test a
plain integer parameter, e.g. `... when #del != 0`; a known constant actual
decides; an unknown value gives the cautious join of both cases.

## 2. Census

[PREDICTION.md](PREDICTION.md) (the destroyer table: object, member paths
per guard, callers' actuals, declared or not, with reasons);
`artifacts/destroyers-{system,slice}-before.json` (every destroyer leaf of
the routes, `scripts/destroyers.py`); `artifacts/bodies.txt` (every body read,
hashed at the three trees, `scripts/bodies.py`). Two findings shape the card:

* **No destroyer's custody depends on a scalar.** `flb_task_destroy`'s `del`
  (like the already declared `flb_input_chunk_destroy`'s) selects only the
  on-disk deletion; `task->ic` is destroyed on every path. The census
  draft's `when #del != 0` would have been unfaithful, so no Fluent Bit
  declaration carries a guard; the guard is implemented and tested on
  fixtures.
* **The callers' actuals are mostly not tracked.** The reported routes do not
  bind a caller's formal or a list entry, so a declared release there reads
  `unresolved resource identity` (as the 30 system leaves already did for
  `flb_input_chunk_destroy` and `flb_task_retry_destroy`). A declaration turns
  `call to uncontracted function` into that leaf and decides nothing.

Six teardown functions stay undeclared: `flb_task_retry_clean`,
`flb_sched_request_invalidate`, `flb_output_exit`, `flb_input_exit_all`,
`flb_engine_destroy_tasks` and `flb_tail_file_remove_all` destroy **list
entries** reachable from a borrowed formal, which v1 cannot state
(`resource_borrowed` would hide a release; the class is
`W45-FB-BACKLOG-REACHABLE-RELEASE`, amended). `flb_task_users_release` stays
`W45-FB-USERS-RELEASE-CHECK`.

## 3. Mechanism

* **Direct call** (`legacy_member_path_route.rs`: `direct_operands`,
  `apply_direct`, `refuse_release_of_taken`; hooked in
  `legacy_callback_route::callback_operands` and hook 2 before the arms). A
  direct call of a member-path declaration **without a hand-off** reads its
  states once (beside a hand-off it still refuses `member-path-direct`). For
  each tracked object the caller linked at a member path of the actual's
  object, `qcore_schemas::member_path::compose_direct` decides: `#released` /
  `#moved` **gone** (released once, the link dropped); `#kept` and every
  unnamed path **stays** (P-B08-SHALLOW, as before); `#stale` **taken**
  (released once, the link kept, the site recorded in
  `AbstractState::member_path_taken`, joined by union, so a later release or
  hand-off of it refuses `member-path-double-release` in
  `apply_resource_action`); a set, an undecided guard's join and a link that
  is not one live object refuse `member-path-undecided`; a path below or
  above a stated one, or a declaration its unit's body check withdrew,
  `member-path-unstated`.
* **Binder** (`contract_member_path.rs`). Beside `resource_borrowed`, a formal
  the declaration states `resource_released` with no kind: the destroyer's
  own object. Its states say what the call did to the objects its members
  held.
* **Parameter guard** (`ParameterGuard`, `MemberPathState::parameter_guard`,
  serialized only when present). `when #X OP K`: `X` a plain integer
  parameter other than the binder, `OP` one of the six comparisons, `K` a
  canonical decimal in range that partitions it, an `@` enumerator or a
  module alias. At a direct call the actual is decided by
  `retention_guard::constant_value` (C04.14.1's decimal and unshadowed
  file-scope enumerator reading); an `@` constant resolves in the guarded
  parameter's own enumeration (`handoff_branch::resolve_parameter`, new
  `MemberFacts::parameter_types`, rule 13l). Holds: the state; fails:
  `#kept`; undecided: the cautious join `{V, #kept}` (`state_under`), which
  refuses at a direct call. At a callback invocation a parameter guard is
  never decided (`invocation_states`: the join). The body check reads a
  parameter-guarded state at every exit (more demanding, never less).

## 4. Lean

`QCore.MemberPathDirectCall` (audit `AuditMemberPathDirectCall.lean`):
`the_direct_call_state_is_sound` (gone: taken exactly once and cleared;
stays: never taken; taken: once, still designated),
`a_release_after_taken_is_a_double_release`,
`a_set_never_decides_at_a_direct_call`, `an_unstated_path_never_decides`,
`a_constant_selects_its_arm`, `an_unknown_value_gives_the_join` (every
value's reading is among the join's), `an_unknown_value_never_releases`,
`the_join_is_cautious` (what an unknown actual decides, every value
decides), `direct_vectors_hold` and `guard_vectors_hold` (Rust
`DIRECT_VECTORS`, `GUARD_VECTORS`, checked equal by the schema's tests).

## 5. Declarations

`fluent_bit/teardown_destroy.qv`: fourteen declared assumptions, each body
cited and read (`artifacts/bodies.txt`), pinned with `quirin-audit pin`
(`scripts/pin.py`) at the trees whose body and member set it was read
against:

| Declaration | Clause shape | Pinned at |
| --- | --- | --- |
| `flb_sched_timer_destroy`, `flb_task_queue_destroy`, `flb_input_collector_destroy` | `resource_released(P, nonnull)` (function map) | 66910c10, 046accf6, 90978ac6 |
| `flb_input_chunk_release_route` | both borrowed, no release (function map) | all three |
| `flb_sched_request_destroy` | released nullable; `timer |-> #moved` (to `timers_drop`) | all three |
| `flb_task_destroy` | released; `ic`, `event_chunk` `|-> #released`; `del` borrowed, no guard | all three |
| `flb_event_chunk_destroy` | released nullable; `tag |-> #released` | all three |
| `flb_output_flush_destroy`, `flb_coro_destroy`, `flb_input_coro_destroy` | released; one path `|-> #released` | all three |
| `flb_output_instance_destroy` | released; 29 paths `#released`, `tls` and `singleplex_queue` `{#released, #kept}` | 046accf6, 90978ac6 (in force at 66910c10 through the identical body; the stated `tls_crl_file` member does not exist there) |
| `flb_input_instance_destroy` | released; 30 paths `#released`, `tls` `{#released, #kept}` | 046accf6, 90978ac6 |
| `flb_tail_file_remove` | released; 12 paths `#released` | 90978ac6 |
| `flb_tail_config_destroy` | released; 10 paths `#released`, five SQLite statements `{#released, #kept}` | 90978ac6 |

The module is selected in `cycle19-resource-contracts.json` (the
fluent-bit profile's manifest) and listed in the load path. Every state is
a declared assumption unless rule 30's body check validates it in the unit
that defines the body; for a released binder the body never clears the
member before freeing the object, so the states stay declared assumptions.

## 6. Measurement

BEFORE: a clean `git archive` export of main `8b360766c`
(`scripts/before-clean.sh`, binary `74d5a6fb…`; the export's contracts, so no
card edit can reach it). AFTER: this card's tree, binary `93bca2d6…`
(`scripts/build.sh after1`, built at `1670c3ba9`, the implementation commit
before the rebases, `4e9ab03a2` after them; the manifest, pins and test
files committed after it are runtime data or test code). Each route under the
shared heavy lock with `QCORE_NULL_PROPERTY` (`scripts/measure.sh`);
identities in `artifacts/runs.json`. The BEFORE corpus report is main's
`de6343c7…`.

| Route | Custody before | Custody after | Null (before = after) | Report after |
| --- | --- | --- | --- | --- |
| corpus `66910c10` (`--jobs 4`) | 938 / 9 / 3,887 | 938 / 9 / 3,887 | 1,113 / 1,435 / 2,355 / 18 Unsafe | `70c61369…` |
| system (448 rows) | 93 / 4 / 351 | **95** / 4 / **349** | 84 / 125 / 245 | `28ef1781…` |
| tail-pause slice (404 rows) | 52 / 1 / 347 / 4 Pending | 52 / 1 / 347 / 4 Pending | 71 / 103 / 232 | `7fb09a8e…` |

Every moved row is in `artifacts/compare-{corpus,system,slice}.json`
(`scripts/compare.py`) and was read:

* **The one gain** (system, both configurations of `src/flb_scheduler.c`):
  `flb_sched_timer_cb_destroy` Unknown → **Safe**. Its body is
  `flb_sched_timer_destroy(timer); return 0;`; its only leaf was the
  uncontracted call. Clauses it rests on: `flb_sched_timer_destroy`'s
  `resource_released(timer, nonnull)` and `resource_return_none()` (row
  `contract_dependencies`: that one declaration, a declared assumption read
  at the three trees and pinned there; its own body, in the same unit, stays
  Unknown: `flb_sched_timer_cb_disable` is uncontracted and `flb_free(timer)`
  reads an unresolved identity, so the declaration is not body-validated).
  Premises: none beyond the declaration; the null row is Safe conditional on
  the inferred `quant null-deref #timer` (unchanged), and the combined row
  becomes Safe. The caller `flb_sched_event_handler` now reads its validated
  summary (`call to uncontracted function flb_sched_timer_cb_destroy` →
  `unresolved resource identity timer`).
* **Leaf trades, no status move** (system 50 more rows, slice 31, corpus 2):
  `call to uncontracted function` at a declared destroyer becomes
  `unresolved resource identity` (system 80, slice 78: list entries of
  `mk_list_foreach_safe`, formals whose binding an earlier unmodelled step
  poisoned, objects never tracked), `indirect memory access is outside the
  resource projection` (system 16, corpus 2: member-read actuals such as
  `ic->task`, `ins->singleplex_queue`, `instance->coro`, `task->event_chunk`),
  and in `flb_tail_config_create` also `nullable resource ctx used by a
  non-null contract` (27: the `flb_calloc` result read nullable at the
  destroy calls). `handle_output_event` loses its
  `flb_input_chunk_release_route` leaf and gains none.
* **No correction:** no Safe row left Safe, no Unsafe appeared, the
  pre-existing provisional Unsafe `flb_output_task_flush` only trades its
  `flb_output_flush_destroy` leaf. **Rule 30's direct-call reading fired
  nowhere** on the three routes (no `member-path-*` refusal and no linked
  member released): no caller links a tracked object at a stated path of a
  tracked object it then destroys.
* **Null property:** tallies identical on all three routes; the stream
  differs only in `flb_sched_timer_cb_destroy`'s custody and combined fields
  and in one `custody_null_premises` entry of `handle_output_event` (the
  removed leaf's line) (`artifacts/null-{corpus,system,slice}.json`).
* **The 30 unresolved argument identities** of the census
  (`flb_input_chunk_destroy` 16, `flb_task_retry_destroy` 14) are unchanged:
  no declaration decides them, as the brief expected.

**Prediction against outcome.** As predicted: no corpus or slice status
move, the null property unchanged, the alone rows `flb_engine_destroy_tasks`,
`flb_sched_request_invalidate` and `flb_tail_file_remove_all` kept their
leaves as `unresolved resource identity`. Not as predicted: one system
function became Safe (`flb_sched_timer_cb_destroy`); the prediction had
generalised from `flb_task_queue_destroy`'s and `flb_task_retry_reschedule`'s
formals (which earlier unmodelled steps poison) to every formal, and this
formal stays bound. The prediction's `flb_tail_config_create` direct-call
reading did not happen: its `ctx` reads unresolved at the destroy calls.

**Obligations** (`artifacts/obligations.json`, `scripts/obligations.py`):
OB-10, OR-2, OR-3, OR-4, OR-5, OR-8, OR-9, OR-10 and OR-11 move at leaf level
only (`flb_output_instance_destroy`, `flb_sched_request_create`,
`handle_output_event`, `flb_sched_event_handler`,
`flb_sched_request_invalidate`, `flb_sched_timer_cleanup`,
`flb_task_retry_reschedule`); OR-6 does not move; none becomes decided.

## 7. Ledger

789 → 789: `FB-TEARDOWN-DESTROY-DECLARATIONS` retired (its DISCHARGE holds:
the decision on the parameter guard, rule 30 at a direct call with Lean
parity, the destroyers declared and checked against their bodies where a
unit defines them, three routes measured with every moved row read);
`FB-TEARDOWN-RELEASE-ACTUAL-IDENTITIES` registered (the unbound actuals
above); `W45-FB-BACKLOG-REACHABLE-RELEASE` amended (the six list-entry
destroyers left undeclared). `W45-FB-USERS-RELEASE-CHECK` is unchanged.
`scripts/update_ledger.py`.

## 8. Rebase and gate

**Rebase.** Measured on `8b360766c`; rebased (no merge commits, no
conflict) onto `a5e4d39d8` and then `aa8d8b030` (main's
`handoff_branch::check_path` stored-result conversion refusal and its Lean
fix). The only shared file is `handoff_branch.rs`, where this card adds
`resolve_parameter` beside `resolve`; the landed change touches the
librdkafka hand-off route, which no Fluent Bit row reads, so the arbiters
above were not rerun on the rebased tip (landing-race policy: records and a
disjoint function; the integrator's combined arbiter decides).

**Gate g1** (at `4c035b231`, the tip on `a5e4d39d8` before the second rebase) was stopped after two steps
failed: 04-tests 2 failures (two production-module count pins this card
missed, `neighbours.rs`, `prod_contracts.rs`; fixed in `d92069d2f`) and
07-lean 1 (`QCore.ReturnCodeHandOff` on main `a5e4d39d8` itself, which
`aa8d8b030` fixed; no file of this card). Its logs are kept in
`private/ftd/gate-g1`.

**Gate g2** (`gate.json`; every code step at `ead6d2112` on `aa8d8b030`, one
exclusive flock for the whole gate, logs `private/ftd/gate-g2`, test logs
`private/ftd/tests-g2`): `cargo check --workspace --all-targets` 0;
submodule 0; `cargo fmt --check` 0; `clippy --locked` 0;
`run_test_binaries.py` **505 units, 518 summaries, 8,169 passed, 0 failed,
15 ignored**, exit 0, no rerun; both reevaluation probes 0;
`check_lean_audits.py` **330** Lean audits (329 on main; the audit
`AuditMemberPathDirectCall.lean` added); provider gate: `package.py` build
and verify (libevent, librdkafka) 0 / 0 / 0, `cargo check --locked` in
`providers/` 0, `providers/contracts.py check` 0, `cargo test` in
`providers/` **133 passed** / 0 failed, `cargo fmt` and `clippy` in
`providers/` 0. Two Python count pins failed at that head
(13-provider-python 1: `test_contract_load_path`, 62 != 61; 08-python:
`test_contract_corpus`, 132 != 131, beside the matrix test that always fails
before the regeneration); the fix (`a756f7ed8`) changes those two Python
test files only (`python-rerun-diff.txt`), so the Rust, Lean and
provider-Rust steps stand, and `scripts/gate-python-rerun.sh` reran
13-provider-python (25 OK, exit 0) and the docs part at `a756f7ed8`
(08-python 374 tests, 1 failure: the accounted matrix test; 09-index 0); the
failed logs are kept in `gate-g2/superseded/`. Then `finish-gate.sh`:
`build_matrix.py . --gate gate.json` (09a), the Python suite again (09b) and
the docs index (10b, 10c); exits in `gate/`.

## 9. Combined arbiter against main `aa8d8b030`

Requested by the integrator: main `aa8d8b030` (three code commits since the
measured base: `a5e4d39d8`, `b93c8f8b9`, `aa8d8b030`), built from a clean
export (binary `88b27dd6…`), against this card's tip `8673dbfde` (binary
`87c1abdb…`), each route under the heavy lock with the null property
(`scripts/combined.sh`, `scripts/combined_compare.sh`;
`artifacts/combined-aa8d8b030-*.json`). Corpus 938 / 9 / 3,887 → 938 / 9 /
3,887 (reports `7326ba39…` → `998b4310…`); system 93 / 4 / 351 → **95** / 4 /
**349**, the only status move `flb_sched_timer_cb_destroy` in both
configurations; slice 52 / 1 / 347 / 4 Pending unchanged; null tallies
identical on all three routes (the null stream differs only in the same two
system rows as in section 6). The moved rows (corpus 2, system 52, slice 31)
are exactly section 6's, row for row with the same leaves lost and gained
(compared by leaf text; line numbers are the analysed stream's). No other
difference.
