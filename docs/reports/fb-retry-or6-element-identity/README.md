# FB-RETRY-OR6-ELEMENT-IDENTITY: a formal's link obligation, carried to the caller

Card FB-RETRY-OR6-ELEMENT-IDENTITY (Fluent Bit priority 2, FB-RETRY-LIFETIME
with CG17 on OR-6). Base `76c1f9cfc` (r77). Gateway rule **91** (new, not
switchable); rule 46 amended (its kill policy factored into
`link_member::reach`, behaviour unchanged); contract specification §C04.8
amended; Lean `QCore.CarriedLinkObligation`. Commits: probe `a82b33559`
(the base verdicts pinned), [prediction](PREDICTION.md) `1b7624991`,
semantic `e8016a423` (the measured tip); `d6536e6ea` moves the fixtures,
unchanged, into `link_member_premises.rs` (section 7).

## Headline

* **LERI-HELD-FORMAL-LINK-OBLIGATION: discharged for every body whose formal
  is destroyed on every exit.** A contract stating
  `resource_unpublished(p, MEMBER)` at a bound position gives its body `p`'s
  `MEMBER` links as an entry fact; rule 43's held arm, refused by rule 46 (3)
  for a formal, admits `p` on that fact at a destroyer of the same member;
  `p` counts as released only through that destroyer. W4 publishes such a
  body's summary as an element destroyer of the formal, so rules 34, 43 and
  46 check the obligation at every call: the caller's own initialisation,
  its published object, an element bound through `MEMBER`, or its own formal
  (carried again). Every other actual refuses at the caller.
  `flb_sched_timer_cb_destroy` is that body.
* **Not discharged: a formal destroyed on one exit class only.**
  `flb_output_task_queue_enqueue` destroys `retry` on its allocation failure
  (`return -1`) and stores it on `return 0`. No mode holds at every exit, and
  in the exit-specific route the `return 0` class keeps the destroyer path
  (diverged) while `retry` is stored: stating the clause there would claim an
  unlink where the body stores, and no clause states the precondition
  without the unlink. The row stays Unknown; the residual stays on
  LERI-HELD-FORMAL-LINK-OBLIGATION (amended).
* **OR-6: not stated, unchanged.** Its two functions are a mutually
  recursive pair; the blocker is the recursive component's summary, not the
  element identity or the member (section 6).
* **Measured** (fluent-bit-system route, base `76c1f9cfc` against tip
  `e8016a423`, each from its own `git archive` tree and release binary, over
  this host's 046accf6 preparation): custody 114 / 0 / 329 / 5 →
  **116** / 0 / **327** / 5, exactly the prediction
  (`flb_sched_timer_cb_destroy`, both configurations, Unknown → Safe); one
  leaf trade; null property unchanged. The opt-in current route reads none of
  these units and is unchanged (section 5).

## 1. Census

**The rows.** `flb_sched_timer_cb_destroy(timer)` is
`flb_sched_timer_destroy(timer); return 0;` (`flb_scheduler.c:730`).
`flb_output_task_queue_enqueue(queue, retry, task, out_ins, config)`
(`flb_output.c:283`) allocates `queued_task`, and on failure calls
`flb_errno()` and `flb_task_retry_destroy(retry)` when `retry` is non-null,
returning -1; otherwise it stores `retry` in `queued_task` and links
`queued_task` into `queue->pending`, returning 0. Both destroyers declare
`resource_unpublished(P, _head)` (`teardown_destroy.qv`, `task_users.qv`).

**Every formal handed to a member-naming destroyer**
([scripts/census.py](scripts/census.py),
[artifacts/census.json](artifacts/census.json), the prepared tree): 17
calls in 11 functions. Only `flb_sched_timer_cb_destroy` destroys its formal
on every exit; the other ten (`flb_output_task_queue_enqueue`, four
`flb_input_*_property_check`, `flb_output_net_property_check`,
`flb_output_enable_multi_threading`, `flb_task_retry_reschedule`,
`flb_engine_dispatch_retry`, and `in_tail`'s `check_purge_deleted_file` and
`reconcile_file_state`) destroy it on a failure path only.

**Base behaviour pinned** (probe commit `a82b33559`,
`crates/qcore-c-gateway/tests/held_formal_link_obligation.rs` at base, since
folded into `link_member_premises.rs`): the
wrapper shape is Unknown (`unlink by item_destroy of t, which this function
has not published`, with the formal's unresolved identity), the enqueue
shape is Unknown, and every caller of the wrapper reads `call to
uncontracted function tdestroy`.

## 2. Mechanism

`crates/qcore-c-gateway/src/legacy_carried_link.rs`:

1. **The entry fact.** `EntryCustody::build` seeds `Ready(MEMBER)` at each
   bound formal whose contract position states `resource_unpublished(p,
   MEMBER)` (`carried_members`, `seed`; new state
   `AbstractState::carried_links`, joined by equality).
2. **The kill policy.** A `Ready` record is forgotten wherever rule 46
   forgets a `link_ready` record (`link_member::reach`, the factored policy:
   a member-place store, a contract call's actuals and written place, any
   other memory-writing statement), at a nested call, and at a rule 36b
   self-initialisation of an object its base may denote (it overwrites the
   caller's links). A `Spent` record survives.
3. **The admission.** In rule 43's held arm, after rule 46's own check
   fails, `admits` takes the object when its record is `Ready(MEMBER)` for
   the destroyer's declared member; the object is marked unlinked and the
   record `Spent`.
4. **The exit.** `observe_exit` reads a carried formal `Released` only with
   a `Spent` record; otherwise `Mixed`, which no mode matches.
5. **The summary.** `infer_cfg_summary` offers the clause in its probe at
   each bound formal handed directly to a member-naming destroyer
   (`candidates`: one member per formal; `offer`) and attaches it where the
   settled mode is `Released` (`attach`), then checks the candidate against a
   fresh run as before.

No consumer changes: a derived contract with `Unpublished` and a link member
at a `Released` position is lowered at its callers exactly as a declared
destroyer is (`container_publication::operands`, rule 34's
`destroyer-order`, rules 43 and 46). **RETURN-POSITION-SOUNDNESS-PREMISE-ELIDED
/ -CONTRACT-ROUTE**: no projection-tracked place, mode or binding is
created; re-read, no amendment.

**Hook points in shared files** (each a call into the new module):
`legacy_container_publication.rs` (the module declaration and one `else if`
in rule 43's held arm), `legacy_transfer.rs` (the state field, the join, the
nested-call clear, the per-statement retain, the entry seed and the exit
observation), `legacy_summary.rs` (three lines in `infer_cfg_summary`), and
`legacy_link_member.rs` (rule 46's `retain` factored into `reach`, no
behaviour change).

## 3. Controls

`crates/qcore-c-gateway/tests/link_member_premises.rs`, module
`carried_link_obligation` (base pins at `a82b33559` and tip expectations at
`e8016a423` in `held_formal_link_obligation.rs`, folded in at `d6536e6ea`) and the module's unit tests
(the Lean vectors):

| Fixture | Base | Tip | Refusal at tip |
|---|---|---|---|
| wrapper `tdestroy(t) { item_destroy(t); return 0; }` | Unknown | **Safe** | |
| caller: `q_alloc`, `mk_list_init(&e->_head)`, `tdestroy(e)` | Unknown | **Safe** | |
| caller: element of `q->items` bound through `_head` | Unknown | **Safe** | |
| caller: its own formal (carried again) | Unknown | **Safe** | |
| caller: `q_alloc`, `tdestroy(e)` | Unknown | Unknown | rule 46 held refusal at `tdestroy` |
| caller: element bound through `_rotate` | Unknown | Unknown | rule 46 member refusal at `tdestroy` |
| caller of a carrier, object not initialised | Unknown | Unknown | |
| wrapper writing its formal first (`q_use(t)`) | Unknown | Unknown | unlink not published |
| wrapper destroying on one path, `q_free` on the other | Unknown | Unknown | unlink not published |
| wrapper initialising its formal's `_head` first | Unknown | Unknown | |
| caller of that withheld wrapper | Unknown | Unknown | uncontracted `tdestroy` |
| declared destroyer whose body frees without the unlink | (not pinned) | Unknown | |
| enqueue shape | Unknown | Unknown | |

No fixture is accused. Amended pinned expectation (attributed):
`link_member_premises.rs`'s `formal-handed-to-destroyer`
(`subject(e) { item_destroy(e); }`) moves Unknown → Safe: its summary now
carries the obligation. `element_release_summary.rs`'s
`formal-handed-to-destroyer-caller` (the caller passes an uninitialised
`q_alloc` object) stays Unknown, now at the caller's rule 46 refusal.

The declared-destroyer row is a tightening: a body checked against
`resource_unpublished(p, _head)` + `resource_released(p)` that frees `p`
without the unlink no longer validates. The measured routes' declared
destroyers already refuse their own `mk_list_del(&p->_head)` (a plain unlink
of an unpublished object), so no measured row is expected to move on it.

## 4. Lean

`proof/QCore/CarriedLinkObligation.lean`, audited by
`proof/AuditCarriedLinkObligation.lean` (15 theorems, axioms `propext` and
`Quot.sound` only). The entry's `ready` records hold when every caller
discharges the clause (`the_seeded_record_holds`); they survive exactly the
statements that cannot write the object, and the join
(`retaining_keeps_every_ready_record`, `the_join_keeps_every_ready_record`);
the held arm admits only a `ready` record of the declared member
(`the_carried_admission_is_sound`, `a_spent_record_admits_nothing`,
`a_memberless_destroyer_takes_no_carried_object`); a carried formal is
observed released only through the destroyer
(`a_carried_release_is_the_destroyers`, `an_uncarried_disposition_is_unchanged`,
`the_observation_only_refutes`); each caller discharge (its own
initialisation, an element through the member under P-CRR-LINKED, its own
formal carried again) establishes the precondition
(`the_discharge_establishes_the_precondition`) and nothing else admits
(`no_discharge_no_admission`). `vectors_hold` equals `ADMIT_VECTORS` and
`OBSERVE_VECTORS`. The candidates' lexical reading, the kill policy's
over-approximation, every consumer's application or refusal of a container
effect, and the Rust correspondence are trust boundary 28's.

## 5. Measurement

**Routes.** The rows are system-route rows. The fluent-bit-system route is
the evaluation profile's command (`current --selection
contract/profiles/fluent-bit/system-selection.json`, the eight-home link map,
null and outer projections on), run by [scripts/system.sh](scripts/system.sh)
as `docs/reports/audit-canonical-paths/scripts/measure.py` runs it, over this
host's preparation `private/fb-046accf6` (the profile's revision, 046accf6;
not the profile's own prepared directory, which this host lacks, so absolute
figures are this preparation's). The opt-in `current --monkey-linked` route
([scripts/run.sh](scripts/run.sh), [scripts/opt-in-current.sh](scripts/opt-in-current.sh))
was run too. Pinned routes (`quirin-audit measure --declared`) cannot run on
this host (no published live-route inputs); nothing is fabricated for them.
Base binary `9ff6d676…`, tip `1e106fb1…` (`private/m/run-{base,tip}/binary.sha256`).

**System** ([system-totals-base.json](artifacts/system-totals-base.json),
[system-totals-tip.json](artifacts/system-totals-tip.json),
[system-moved.jsonl](artifacts/system-moved.jsonl),
[system-changed.txt](artifacts/system-changed.txt); runs
`private/m/sys-{base,tip}`, both exit 0):

| | Safe | Pending | Unknown | Unsafe | combined custody+null Safe |
|---|---|---|---|---|---|
| base | 114 | 0 | 329 | 5 | 74 |
| tip | **116** | 0 | **327** | 5 | **76** |

Every moved and changed row (the scripts key rows by file and function, so
the two configurations print once; the totals and a count of the tip report
show both moved):

* **`flb_sched_timer_cb_destroy`** (`flb_scheduler.c`, both
  configurations) **Unknown → Safe.** Base leaves: `unresolved resource
  identity timer` and `unlink by flb_sched_timer_destroy of timer, which
  this function has not published` (rule 46 (3)'s refusal, read through the
  W4 fallback). Justification: the body is `flb_sched_timer_destroy(timer);
  return 0;`; the derived summary states `resource_unpublished(timer, _head)`
  beside `resource_released(timer)`, the obligation P-LERI-HELD-LINKS
  refused is now the caller's, and every in-unit caller checks it (below).
  The row rests on the declaration `flb_sched_timer_destroy` (its tip
  `contract_dependencies`). Its null reading was Safe on both sides; the
  combined reading moves with the custody row.
* **`flb_sched_event_handler`** (Unknown, both): trades `call to uncontracted
  function flb_sched_timer_cb_destroy` at line 18031 for `unresolved resource
  identity timer` and `unlink by flb_sched_timer_cb_destroy of timer, which
  this function has not published`: its `timer` is `(struct flb_sched_timer
  *) event`, neither initialised, published nor an element, so the caller
  does not discharge the obligation and refuses there. This is the clause
  checked at a call, as designed.

No other row's status or leaves changed. No leak, double release or Unsafe
added or removed. The null property's verdicts are identical; the null
records differ only in their evidence-version and build-identity strings.

**Opt-in current** (`private/m/run-{base,tip}`, both exit 0;
[current-totals-base.json](artifacts/current-totals-base.json),
[current-totals-tip.json](artifacts/current-totals-tip.json)): 1,205 / 2
Pending / 3,590 / 7 on both sides, no row moved and no row's leaves changed.
Its 181 units are `lib/` and `src/flb_input_chunk.c`; none hands a formal to
a member-naming destroyer. **The prediction's prose was wrong here**: it
expected the `flb_sched_timer_cb_destroy` move on this route too, without
checking that the route reads `flb_scheduler.c` (it does not). The
`measure-prediction` block names the system route only, and that is where
the row moved.

**Prediction against measured.** The [prediction](PREDICTION.md)'s system
block (`flb_sched_timer_cb_destroy` Unknown → Safe, Safe +2, Unknown −2) is
exactly what was measured, with the predicted leaf trade in
`flb_sched_event_handler`. Slice and corpus were not run (no preparation on
this host; the prediction says none).

## 6. OR-6 and CG17

OR-6 (`queued_task` is not used after the flush fails) needs a caller of
`flb_output_task_singleplex_flush_next` to learn that it releases elements
of the queue's lists. `flb_output_task_queue_flush_one` binds
`queued_task = mk_list_entry_first(&queue->pending, struct
flb_task_enqueued, _head)`: rule 10f's foreign element, whose binding member
rule 46 already records (`_head`), unlinked by `mk_list_del(&queued_task->_head)`
and re-published into `queue->in_progress`. CG17's member provenance (rule
11u) is not what OR-6 lacks: the element's identity after the unlink is
already tracked (W45-CONTAINER-ROOT-REACH), and its member is checked (rule
46 (2)). What OR-6 lacks is a summary: `flb_output_task_singleplex_flush_next`
would publish `ElementsReleased` at `queue` (rule 43), but it calls
`flush_one`, which calls it back; neither row is Safe alone, so neither
publishes first, and the recursive-component routes attach no clause. Rule
91 does not reach the pair (`flush_one`'s destroyer actual is
`queued_task->retry`, not a formal). OR-6 stays not stated;
W45-FB-OR6-ELEMENT-IDENTITY and CG17-CONTAINER-OF-LIST-LOOKUPS are amended
with this reading, and no CG17 shape was discharged.

## 7. Gate

`python3 scripts/gate_scope.py --base 76c1f9cfc --out <worktree>/private/gate`
through `private/toolchain/env.sh`, on `e8016a423`: [gate.json](gate.json),
logs `private/gate/`; Clang recorded in
[gate/clang-version.txt](gate/clang-version.txt). Mode full (the diff reaches
`qcore-c-gateway`); all Lean (a proof root changed); no provider gate
(`providers/` unchanged). **Exit 1**, one failure, this card's:

| Step | Exit | Counts |
|---|---|---|
| submodules, fmt, clippy | 0 / 0 / 0 | |
| Rust suite (`run_test_binaries.py`) | 0 | 726 units, 771 summaries; 10,367 passed, 0 failed, 24 ignored |
| both reevaluation probes | 0 / 0 | |
| Lean (`check_lean_audits.py`) | 0 | 532 audits (this card adds `AuditCarriedLinkObligation.lean`) |
| Python suite | **1** | 807 tests, 1 failure |

The failure: `test_gate_scope_census.test_the_census_covers_the_suite`
(703 observed units not greater than 0.9 × 782). The card's new test binary
`held_formal_link_obligation` was the 782nd unit; at base the ratio holds by
0.1 of a unit. Fix `d6536e6ea`: the fixtures move, unchanged, into a module
of `link_member_premises.rs` (no new unit). Re-run after it, through the same
environment: `cargo fmt --check`, `cargo clippy -p qcore-c-gateway
--all-targets -D warnings`, `link_member_premises` (3 passed),
`check_lean_audits.py` (exit 0, 532 audits; `private/post-lean.log`) and the
Python suite with `QUIRIN_CARD_GATE_BASE=76c1f9cfc` as the gate sets it
(exit 0, 807 tests; `private/post-python.log`). The Rust suite was not re-run
whole: the only code change is the test file's location. A discarded
attempt of the Python suite without that variable failed
`test_build_matrix_sections` on the Lean-audit count the integrator
regenerates at landing (expected outside a card gate).

LLP64 fixture scan over the changed test files: exit 0 (0 counted, 0 found;
the fixtures use no size type, `long` or enumeration).

## 8. Follow-ups

[integration-amendments.json](integration-amendments.json) amends
LERI-HELD-FORMAL-LINK-OBLIGATION (discharged for a formal destroyed on every
exit; the residual is a formal destroyed on one exit class only, which needs
a precondition clause that states no unlink), W45-FB-OR6-ELEMENT-IDENTITY
and CG17-CONTAINER-OF-LIST-LOOKUPS (section 6), and registers nothing new.
The increment paragraph is in the amendments file. No evaluation profile's
baseline changes here (`profiles: []`).
