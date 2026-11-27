# RULE50-CONSUMER-COMPOSITION

Card RULE50-CONSUMER-COMPOSITION discharges two follow-ups that card
[SCHEDULER-DESTROY-MEMBER-STATES](../../language-and-tooling/scheduler-destroy-member-states/README.md)
registered: RULE50-CONSUMER-COMPOSITION and MEMBER-PATH-HELPER-AT-A-CALL. It
adds gateway rule [51](../../../../agents/gateway.md#constraint-51), amends rule 50,
[C04.19.6](../../../../QCORE_CONTRACT_SPEC_r0_3.md#c04196-a-release-or-move-out-at-a-member-path)
and G05d, and adds the Lean module `QCore.DestroyerHelperComposition`. Base
`0f6e17118`; measured tip `8e00d7493` (a first tip, `05c0c6b1d`, was measured
and corrected, §2). Later commits change only this report and its records.

The [prediction](PREDICTION.md) was committed (`c682ab049`) before the first
semantic commit.

## 1. What changed

**Why a clause.** Rule 50 composes `mk_thread_destroy` inside
`mk_http_thread_destroy`'s own unit (`mk_server/mk_http_thread.c`). The
destroyer's consumers (`sched_thread_cleanup` and
`mk_sched_threads_destroy_conn`) are in `mk_server/mk_scheduler.c`, which
cannot read that body. Body evidence never feeds an analysis (rule 13f), and
the run's fold sees only what rows consulted. So no reading inside a unit,
and no fold, could tell a consumer which helper takes the stale path's
object. The composition therefore has to be stated, with the body as its
evidence, as every other clause is (rule 13i).

The card adds one predicate. `resource_member_helper(P, TAG.PATH, H)` says
that the object `P->PATH` held is taken, inside the call, by the unit helper
`H`. Placement (loader, total-or-reject):

- at most one clause per declaration;
- `P` is a formal the declaration releases;
- `PATH` is stated `#stale`, unguarded and not a set;
- `H` is not the declaration itself;
- the declaration has no callback clause and no hand-off.

The last condition means a helper destroyer never names a helper, so only one
frame is ever composed. Every violation rejects the module. Nine misplacements
are tested. The spelling is this card's proposal: no user decision chose it,
and it is offered for review with the card.

**The consumer composition (RULE50-CONSUMER-COMPOSITION).** At a direct call
of a destroyer that names `H`, after the destroyer's own states
(`legacy_callback_route::composed_helpers`, `member_path_route::apply_direct`):

1. `H`'s declaration must be in force as a helper destroyer: not withdrawn in
   the unit, not in the function map. Each member invocation it states must
   read a closed empty set over the run's member facts, which invokes
   nothing. The call refuses otherwise:
   - an open set refuses `callback-member-open`;
   - a closed non-empty set, a route without the run's facts, or a missing or
     non-helper declaration refuses `member-path-helper`.
2. `H`'s states are then read at the one live object the caller linked at the
   stale path, as a direct call of `H` on that object reads them.

Looking `H` up makes the row rest on `H`. Its `declarations` list `H`, its
`run_set_members` list `H`'s members, and the run's fold (rule 12n) demotes
the row unless their run sets are closed. A callback invocation never
composes `H`.

Step 2 is coded and fail-closed, but it decides nothing in any measured row.
The tracker holds no link below a linked object: a store such as
`m->parent = t; t->callee = co` refuses earlier as an indirect access, with
or without this card, which the test `the_helpers_states_decide_what_a_caller_links_and_a_deeper_link_is_refused`
shows. No measured caller links anything at `mth->parent` either.

**The helper at a direct call (MEMBER-PATH-HELPER-AT-A-CALL).** A helper
destroyer no longer refuses `member-path-direct`. Its member invocations are
decided first, as every direct call's are (C04.19.3). Its states are then
applied: `apply` now runs the invocation candidates' member-path states
before the direct states. Over the run's closed empty set, a caller that
links an object at `t->callee` and calls `thr_destroy(t)` is Safe, and
releasing that object afterwards refuses `member-path-double-release`.
Without the run's facts, the unit's own empty set refuses. A callback
candidate of a helper destroyer is still missing, because C04.19.7 composes
no declaration with states of its own (follow-up below). The text of the
`member-path-direct` refusal is unchanged: card MEMBER-PATH-DIRECT-REFUSAL-TEXT
owns it, and this card changed only the condition beside it.

**Rule 50's evidence.** In a unit that defines the declaration, the rule 50
reading admits only the named helper. So `50` means the take went through
`H`, and `50` covers the new clause kind `memberHelper`. Nothing else covers
it (`ClauseKind::MemberHelper`; `r50` in `QCore.BodyClauseStanding` amended).
A declaration naming another helper gets no `50` (tested).

**The premise decision: `scheduler_callback_footprint` is removed.** The
premise stated that `th->cb_destroy` (with `th = mth->parent`) is NULL or a
callback with a bounded footprint. After this card, every row resting on
`mk_http_thread_destroy` carries checks of exactly that:

- the consumer's call composes `mk_thread_destroy`'s `mk_thread.cb_destroy`
  invocation, and refuses unless the run's set is closed and empty;
- the row then rests on `mk_thread_destroy`, so the fold re-checks the
  closure.

The remaining assumption is rule 12n's own premise,
P-CG22-MEMBER-CLOSED-WORLD, under which every other member-invocation row
already stands. That the destroyer calls `mk_thread_destroy` at `parent` is
the new clause: a declared assumption at the consumer and body-checked by
rule 50 in `mk_http_thread.c` (evidence `50` covers it, §2). Lean:
`the_composition_carries_the_footprint` shows that over closed empty sets the
composition equals the reading the premise stood for and the fold keeps the
row, and every other set refuses. The other seven premises and their
refinements are unchanged.

**Pins and locks.** `providers/contracts.py sync` first rewrote the inventory
lock and digest, then refused because `mk_http_thread_destroy`'s pin was
stale (`private/logs/sync-2.log`). The module keeps 87 clauses: one premise
out, one helper clause in. `quirin-audit pin` (tip worktree binary,
[pin.sh](artifacts/pin.sh)) re-pinned `mk_http_thread_destroy` at `046accf6`
(a fresh `prepare clang-build` tree, [prep_older.sh](artifacts/prep_older.sh))
and at `66910c10` (the corpus input tree). Both exited 0 (`private/pin-1/`).
A second `sync` then exited 0 (`private/logs/sync-3.log`). `pin --check`
exits 1 at both trees, as at base: the only entry not in force is the
pre-existing `mk_vhost_set`, whose tip repository this host lacks.
`pin --verify-contracts` exits 0 (`private/pin-check-1/`).

**Tests.** `tests/rule50_consumer_composition.rs` (8 tests over Clang):

- loader attachment and nine misplacements, plus the nested form on a helper
  destroyer;
- a consumer that is Safe over the run's closed empty set and consults the
  helper;
- refusals without the run's facts, without the helper's declaration, and
  for a non-empty set and an open set;
- the helper at a direct call: Safe over the run, refused without it;
- rule 50 evidence `30+50` only through the named helper;
- the linked-object takes and the deeper-link refusal.

`tests/member_value_helper.rs`'s direct-call test now expects the invocation
reading (`callback-member-open` on that route, no pre-pass) in place of
`member-path-direct`.

**Lean.** `QCore.DestroyerHelperComposition` has 13 theorems, no placeholder
and no project axiom (`proof/AuditDestroyerHelperComposition.lean`):

- order: a refused invocation applies no state;
- refusals: open, non-empty, unread and deeper-frame sets refuse;
- composition only over closed empty sets, and the fold agrees;
- the stale take refuses a later release;
- evidence only through the named helper;
- the scheduler's case.

## 2. Measurement

**Arbiter.** `quirin-audit measure --declared PREDICTION.md`, run with the
measure plugin built from each tip and driven by
[arbiter.sh](artifacts/arbiter.sh) and [arbiter2.sh](artifacts/arbiter2.sh)
over [inputs.json](artifacts/inputs.json). This host has no published
live-route inputs, refuses user namespaces (so no `rev:` side runs), and the
earlier cards' preparations were deleted. This card therefore prepared the
**corpus route only**, in its own worktree, with the base binary
([prep.sh](artifacts/prep.sh)): `prepare clang-build --link` of the local
Fluent Bit clone at `66910c10`, its production compile database copied into
`fluent-bit-clang-source/build/`, `prepare monkey`, and
`providers/package.py build`. Both sides are `git archive` exports under
`private/`, each with its `third_party/libevent` cloned at `d82464a2` and the
same driver adaptation (`run_pinned_audit.py` with `QCORE_CLANG` =
`/usr/lib/llvm-21/bin/clang`), with `--no-slots`. Absolute figures differ from
the published baselines; every comparison here is base against tip on the
same inputs.

**Routes not measured: system and slice.** They analyse no Monkey unit, no
row there consults `mk_http_thread_destroy` or `mk_thread_destroy`, no Fluent
Bit declaration is a helper destroyer, and none states the new clause. Their
preparations were not built. Nothing is claimed about them beyond that
reading.

**Attempts.**

- `measure/arbiter-1`: base export `0f6e17118` against tip export
  `05c0c6b1d`. It completed with status `complete`, no move and a declared
  `match`, but it showed a defect. The consumers' rows did not list
  `mk_thread_destroy`: rule 13f keeps only the names a body spells, so neither
  the fold nor `run_set_members` read `mk_thread.cb_destroy` for them. With
  the premise removed, that is not the reading the card claims. It is kept as
  [native-comparison-corpus-attempt1.json](artifacts/native-comparison-corpus-attempt1.json)
  and discarded as evidence for the claim.
- `8e00d7493` notes the composed helper as relied, as a decided callback
  candidate is. The test now asserts the row's published dependencies.
- `measure/arbiter-2`: base side reused (`run:` of arbiter-1's base side)
  against tip export `8e00d7493`. This is the measurement of record:
  [native-comparison-corpus.json](artifacts/native-comparison-corpus.json).

| Route | Base Safe / Unknown / Unsafe | Tip | Moves | Declared check |
|---|---|---|---|---|
| corpus (4,704 rows) | 1,081 / 3,617 / 6 | 1,081 / 3,617 / 6 | 0 | **match** |

Status `complete`, no unmatched row, no failure. Diagnostic counts trade one
leaf: `unknown_alias` 10,707 → 10,708, `unsupported` 23,454 → 23,453.

**Content changes (six rows, no status change), as predicted.**

- `sched_thread_cleanup`, `mk_sched_threads_destroy_conn`,
  `mk_sched_threads_purge`, `mk_sched_threads_destroy_all` (Safe) and
  `mk_sched_drop_connection` (Unknown):
  - `mk_thread_destroy` joins their `declarations`;
  - `mk_thread.cb_destroy` joins their `run_set_members`;
  - their `hypothesis_dependencies` for `mk_http_thread_destroy` go from 27
    atoms to 26, because `scheduler_callback_footprint`'s atom is gone and the
    seven other premises and 19 refinement atoms remain.

  The run's fold kept every Safe row, so the run's set of
  `mk_thread.cb_destroy` is closed. The consumer's call composed, so the set
  is also empty.
- `mk_http_thread_destroy`'s own row (Unknown): `member-path-direct` at
  `mk_server/mk_http_thread.c:269` becomes `unresolved resource identity th`
  at the same line. The direct call of the helper now composes, and its
  argument is a copy of a formal's member, which the row cannot resolve. Its
  hypothesis atoms go from 27 to 26.

**Body evidence (tip, `resource-diagnostics.components.json`).** In
`mk_server/mk_http_thread.c`:

- `resource_member_helper(#0, mk_http_thread.parent, mk_thread_destroy)` is
  covered by `["50"]`;
- `quant live #0: mk_http_thread.parent |-> #stale` is covered by
  `["30","50"]`.

The clause the consumers now read is therefore body-validated through exactly
the named helper. Wall time was about 96 minutes (arbiter-1, both sides
concurrently, with the gate running) and 75 minutes (arbiter-2, tip only).

## 3. Gate

The scoped card gate, `sh private/toolchain/env.sh python3 scripts/gate_scope.py
--base 0f6e17118 --out <dir>` ([gate.sh](artifacts/gate.sh)), ran in full mode,
because the diff reaches `qcore-c-gateway`, `qcore-contract`, `qcore-schemas`
and `providers/`. Toolchain: Clang 21 and Lean 4.35.0. Gate-2 ran on
`8e00d7493` and is the record ([gate.json](gate.json), logs in
`private/gate-2/`). Gate-1, on `05c0c6b1d` (`private/gate-1/`), also passed
all 14 steps.

| Step | Exit | Counts |
|---|---|---|
| submodules, fmt, clippy | 0, 0, 0 | |
| Rust suite (`run_test_binaries.py`) | 0 | 639 units, 682 summaries; 9,517 passed, 0 failed, 17 ignored |
| reevaluation probe, follow-up | 0, 0 | |
| Lean audits | 0 | 435 audits |
| Python suite | 0 | 761 tests |
| provider package build, verify-all, verify `qcore.libevent` | 0, 0, 0 | |
| provider tests, fmt, clippy | 0, 0, 0 | 43 summaries; 133 passed, 0 failed, 1 ignored |

## 4. Follow-ons

Registered in [integration-amendments.json](integration-amendments.json):

- `MEMBER-PATH-HELPER-AS-CANDIDATE`: a helper destroyer is still a missing
  callback candidate (C04.19.7 composes no declaration with states of its
  own). No measured invocation has one as a candidate.
- `HELPER-STATES-BELOW-A-LINKED-OBJECT`: rule 51's second step, the helper's
  states at the object linked at the stale path, decides nothing today,
  because the tracker refuses a link below a linked object. It is tested only
  through the helper's own direct call.
- `MEMBER-HELPER-CLAUSE-SPELLING`: `resource_member_helper` is this card's
  spelling, adopted without a user decision; confirm it or choose another.

RULE50-CONSUMER-COMPOSITION and MEMBER-PATH-HELPER-AT-A-CALL are retired.
SCHED-DESTROYER-PREMISES is amended: seven premises remain Open source truth.
Out of scope and still open: RULE50-NONEMPTY-RUN-SET and
MK-THREAD-DESTROY-UCONTEXT-IDENTITY.
