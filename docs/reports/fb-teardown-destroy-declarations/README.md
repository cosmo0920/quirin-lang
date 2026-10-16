# FB-TEARDOWN-DESTROY-DECLARATIONS: rule 30 at a direct call, the parameter guard, and Fluent Bit's destroyers

Card FB-TEARDOWN-DESTROY-DECLARATIONS (wave 48 round 30, card-high). Gateway
rule **30** amended in place; contract specification **C04.19.6** amended;
grammar G05d; Lean `QCore.MemberPathDirectCall`; provider module
`quirin-provider-fluent-bit/contracts/fluent_bit/teardown_destroy.qv`.
Base: local `main` `8b360766c` (the card started on `cc3e79559` and
fast-forwarded before any code; the census was read there). The census and
the frozen [prediction](PREDICTION.md) were committed (`d2b8fe7e4`) before
any implementation code. No new word, value or form was needed beyond the
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
