# SCHEDULER-DESTROY-MEMBER-STATES

Card SCHEDULER-DESTROY-MEMBER-STATES breaks part of the prose behind
`mk_http_thread_destroy`'s eight `resource_premise` clauses into symbolic
clauses (`qcore-provider-libevent`, `contracts/monkey/intrusive_list.qv`,
card W48-SCHED-CLEANUP-PRODUCTION, rule 34). It follows the user's decisions of
2026-11-14, verbatim: **"Go with 1, 2A and typed refinements for 3"**. It adds
gateway rule [50](../../agents/gateway.md#constraint-50), amends
[C04.19.6](../../QCORE_CONTRACT_SPEC_r0_3.md#c04196-a-release-or-move-out-at-a-member-path),
G05d and a pointer in rule 30, and adds the Lean module `QCore.MemberValueHelper`.
Base `89fa31d21`; measured tip `7bbea41df`. Later commits change only this
report and its records.

The [prediction](PREDICTION.md) was committed (`9326ff018`) before the first
semantic commit.

## 1. What changed

**Decision 1: member-path states on `mk_http_thread_destroy`.** The released
formal `mth` now states three paths:

- `parent |-> #stale`. The body copies `mth->parent` into `th` and passes it
  to the inline `mk_thread_destroy`, which frees it. The member is never
  cleared.
- `session |-> #kept` and `request |-> #kept`. Both are read and never stored.

The declaration still states its unlink `resource_unpublished(mth, _head)`, its
release and no result; rule 43 admits the unlink beside member-path states.
The declaration moves from the function map to the retaining table, as the Fluent Bit
teardown destroyers did.

*Composition with the release.* `parent` names the allocation `mth` lives in
(`MK_THREAD_DATA(th) = (char *) th + sizeof(struct mk_thread)`). At a direct
call, the stale path takes the object the caller linked at `mth->parent` once
(rule 30's `taken`), and the release ends `mth` once. If the caller tracked
both as one identity, the second end refuses (`member-path-double-release`).
If it tracked them as two, each identity ends once, which is true of the C:
one `free` ends both. No reading releases one identity twice, and none gains a
`Safe`. A later release or use of either object refuses. Lean:
`distinct_identities_each_end_once` and `one_identity_refuses_the_second_end`.
No measured caller links a tracked object at any of the three paths: both
drains bind only the element, through `container_of(..., _head)`.

**Decision 2A: the helper's own declaration and rule 50.**
`mk_thread_destroy` is declared beside it with these clauses:

- `resource_callback_member_invoked(th, mk_thread.cb_destroy, _)`;
- `resource_released(th, nonnull)` and `resource_return_none()`;
- `callee |-> #stale`, `caller |-> #kept` and `data |-> #kept`.

Before this card the loader refused member-path states beside any callback
clause. It now admits a *helper destroyer*: member invocations may stand beside
the states when every state and every invocation bind one formal that the
declaration releases, with no hand-off. Every other combination still rejects.
Consumers keep refusing such a declaration (`member-path-direct` at a direct
call; a missing candidate). Rule 50 extends rule 30's body reading
(`member_path_reading.rs`, `member_path_check::validates_composing`,
`legacy_callback_route::member_path_helper`):

- *The followed helper.* The reading composes one kind of call: a direct call,
  in the declaration's own frame, of a unit `static` function with a helper
  declaration in force. The call must receive, at the formal the helper
  releases, the value of a path stated `#stale`. That value is either
  `P->M` while `M` is untouched, or its single copy: a local whose only write
  copies it, never updated, address never taken, and written on every path.
  The composition takes the object `M` held and records the helper's states at
  it (`parent->callee` reads `#stale`, `parent->cb_destroy` invokes nothing).
- *When the helper qualifies.* Its own rule 30 check finds no contradiction,
  its pointee is its record, and each of its member invocations reads a closed
  empty set over the run's member facts. A unit's own empty set, an open set,
  a non-empty set, or a route without the pre-pass never qualifies.
- *When the take is established.* Every exit must take the object exactly once
  this way, and the reading must validate the states. The take stays a
  declared assumption after any of these:
  - a take on only some paths, or a second take;
  - a call that is not composed, in any frame, receiving the value;
  - a store or return that may carry the value;
  - a helper reached through a followed helper.
- *Evidence.* An established take is body evidence `50`, recorded beside `30`
  (`contract_clause_standing::EVIDENCE`, and `r50` in
  `QCore.BodyClauseStanding`). No consumer reads it.
- *The link member.* Inside a followed helper, a store at or below the link
  member named by the declaration's own unlink no longer ends the reading.
  Without this, `mk_list_del(&mth->_head)` clearing `_head.prev` and
  `_head.next` would leave decision 1's states a declared assumption.

C04.19.6 said "a member's value passed to a call reaches nothing of the
formal". That sentence still holds: rule 50 does not read the value as the
formal. It composes a declaration at that value.

The `cb_destroy` set is read from the run's member facts, not assumed. The
only store is `th->cb_destroy = cb_destroy` in the inline `mk_thread_new`,
from a formal; for an internal function the facts take that formal's actuals
in its own unit, and its one caller, `mk_http_thread_create`, passes `NULL`.
The pin tool records both header definitions of `mk_thread_destroy`. The
`ucontext` variant was read too, and the declaration's comment says why its
states decide nothing there: `callee` is an embedded `ucontext_t` and there is
no `data` member.

**Decision 3: typed refinements.** Seven premises gain proposed refinements in
the style of PLUGIN-PREMISE-DECOMPOSITION. Each atom is one `quant proposition`
with its `resource_lifecycle_v1(introduce_<id>, |^+ hypothesis(<id>))`. Atoms
are attached by `resource_premise_decomposition` and chained by one
`resource_premise_flow_v1` entry per premise:

| Premise | Atoms |
|---|---|
| `scheduler_exact_parent_embedding` | `sched_parent_data_identity`, `sched_parent_storage`, `sched_parent_sole_releaser` |
| `scheduler_tls_params_owned` | `sched_tls_slot_null_or_owned`, `sched_tls_api_success`, `sched_tls_slot_cleared` |
| `scheduler_callee_live` | `sched_callee_created_for_thread`, `sched_callee_suspended`, `sched_callee_no_concurrent_use` |
| `scheduler_borrowed_chain_live` | `sched_chain_live`, `sched_chain_not_ended`, `sched_chain_single_scalar_write` |
| `scheduler_valid_membership` | `sched_membership_unique`, `sched_membership_reciprocal`, `sched_no_concurrent_writer` |
| `scheduler_no_later_alias_use` | `sched_alias_set`, `sched_alias_not_used_after` |
| `scheduler_valgrind_request_neutral` | `sched_valgrind_configuration`, `sched_valgrind_tracked_unchanged` |

The refinements are evidence-only metadata: they discharge nothing, and every
premise keeps its name and is still published with every row resting on the
declaration. Four premise texts are narrowed to what no symbolic clause
states:

- the parent embedding: the take is now the stale state;
- the callback footprint;
- the coroutine: the deletion is named;
- the borrowed chain: the session's own custody is the kept state.

**The callback footprint premise is kept.** Rule 50 composes the run's closed
empty set of `mk_thread.cb_destroy`, but only as evidence. No consumer of
`mk_http_thread_destroy` folds that set (rule 12n's fold applies to
declarations that state a member invocation). Removing the premise would lose
the run-closure check the consumers' rows rest on, so it would not be fail-closed.

**Pins and locks.** `providers/contracts.py sync` first refused with a stale
inventory lock, then rewrote it. The module now has 5 declarations and 87
clauses: `mk_http_thread_destroy` has 67 and `mk_thread_destroy` has 7.
`quirin-audit pin` re-pinned `mk_http_thread_destroy` and pinned
`mk_thread_destroy` at `046accf6` and `66910c10` (`private/pin-1/`, both exits 0).
The source trees were the LERI card's preparations, read-only.
`pin --check` exits 1 at both trees, as before this card: the only
not-in-force entry is the pre-existing `mk_vhost_set`, whose tip repository
this host lacks. `pin --verify-contracts` exits 0 (`private/pin-check-1/`). A
comment-only edit followed, and a second `sync` exits 0.

**Tests.**
- `member_path_check`: six new unit tests. Positive cases cover the copy and
  the direct value. Refusals cover a take on some paths, a second take, an
  escape by call, store or return, a local written twice, and a non-static
  callee. They also cover a helper that is missing, not releasing, open,
  another record, at another position, or reached in a deeper frame, plus the
  link-member exemption and its limits.
- `tests/member_value_helper.rs`: six tests over Clang on Monkey-shaped
  units. They cover loader admission and three rejections; evidence `30+50`;
  `30` alone when the helper's invocation is undecided (no run facts), for a
  second take, a branch-only take or an escaped copy, and for a helper without
  a declaration; and the `member-path-direct` refusal of a direct call.

**Lean.** `QCore.MemberValueHelper` has 12 theorems, no placeholder and no
project axiom (`proof/AuditMemberValueHelper.lean`). It shows that the
reading's slot exactly abstracts a path's take count
(`the_slot_abstracts_the_path`, `the_reading_is_the_abstract_run`,
`established_iff_taken_once`). It shows that every refusal above never
establishes the take, that joins are established only when both paths are,
and the custody pair. `QCore.BodyClauseStanding` gains `r50`.

## 2. Measurement

**Arbiter.** `quirin-audit measure --declared PREDICTION.md` (the measure
plugin built from this card's code), driven by
[artifacts/arbiter.sh](artifacts/arbiter.sh) over
[artifacts/inputs.json](artifacts/inputs.json). This host has no published
live-route inputs and refuses user namespaces, so no `rev:` side runs. The
inputs are card LERI-HELD-AND-LINK-MEMBER-PREMISES's worktree-private
preparations, read-only and digest-checked: system `046accf6`, slice
`90978ac6`, corpus `66910c10`. Both sides are `git archive` exports under
`private/`: base `89fa31d21` and tip `7bbea41df`. Each side is built in place
by the tool. One driver adaptation is made in both exports, as LERI did:
`scripts/run_pinned_audit.py` points `QCORE_CLANG` at
`/usr/lib/llvm-21/bin/clang`. `--no-slots` was used.

**Attempts.**
- `measure/arbiter-1` was stopped by hand before any route ran, to re-export
  the tip after formatting; it is discarded.
- `measure/arbiter-2` (base binary `beb5e9e8…`, tip `85533375…`) completed
  system and slice. Its corpus failed on both sides, because the exports'
  `third_party/libevent` was a plain copy: the pinned libevent revision check
  read the enclosing worktree's `HEAD` (`SourceRevisionMismatch`). It is
  discarded as corpus evidence.
- `measure/arbiter-3-corpus` (base `05062097…`, tip `39e8eb1d…`) re-ran the
  corpus with each export's submodule a clone at `d82464a2`.

The two runs' binaries differ by rebuild only; the exports' sources are
unchanged.

| Route | Base custody Safe / Unknown / Unsafe | Tip | Moves | Declared check |
|---|---|---|---|---|
| system (448 rows) | 103 / 341 / 4 | 103 / 341 / 4 | 0 | **match** |
| slice (404 rows) | 59 / 340 / 1, 4 Pending | unchanged | 0 | **match** |
| corpus (4,704 rows) | 1,081 / 3,617 / 6 | 1,081 / 3,617 / 6 | 0 | **match** |

Every route's status is `complete`, with no unmatched row. Records:
[native-comparison-system-slice.json](artifacts/native-comparison-system-slice.json)
and [native-comparison-corpus.json](artifacts/native-comparison-corpus.json).
Corpus analysis took about 87 minutes per side.

**Content changes, none a status.**
- *Corpus, six rows.*
  - `sched_thread_cleanup`, `mk_sched_threads_destroy_conn`,
    `mk_sched_threads_purge` and `mk_sched_threads_destroy_all` (all Safe)
    and `mk_sched_drop_connection` (Unknown) gain the 19 refinement atoms in
    their `hypothesis_dependencies`. The eight legacy premise atoms remain.
  - `mk_http_thread_destroy`'s own row (Unknown) now consults
    `mk_thread_destroy` (`run_set_members` `mk_thread.cb_destroy`). It trades
    leaves: `call to uncontracted function mk_thread_destroy` and two
    `unresolved resource identity mth` leaves become the call's
    `member-path-direct` refusal. Its refusal text still says "beside a
    hand-off" for a member invocation (follow-up below).
- *System, four rows; slice, eight rows.* Only `review_holds` changed, and
  only because the hold's module path spells the side's checkout
  (`private/base-src` against `private/tip-src`). This is environment only.

**Body evidence (corpus, `resource-diagnostics.components.json`).** At base
`mk_http_thread_destroy` has no body evidence: all its clauses are declared
assumptions. At tip its three member-path states are covered by `30` and `50`.
Rule 30 validated them in `mk_server/mk_http_thread.c`, the `_head` stores
inside `mk_list_del` being the unlink's. Rule 50 established the take of
`parent`'s object through the composed `mk_thread_destroy`, its
`mk_thread.cb_destroy` set read closed and empty over the run's member
facts. The declaration moves from declared assumption to partly validated.
Its premises, unlink and release stay uncovered, and `mk_thread_destroy`
stays unvalidated (its released binder's states are assumptions). The
prediction holds on every route.

## 3. Gate

GATE-PENDING

## 4. Follow-ons

Registered in [integration-amendments.json](integration-amendments.json):

- `RULE50-CONSUMER-COMPOSITION`: no consumer reads rule 50's composition, so
  the callback footprint stays a premise.
- `MEMBER-PATH-HELPER-AT-A-CALL`: a direct call or callback candidate of a
  helper destroyer refuses.
- `RULE50-NONEMPTY-RUN-SET`: a helper whose invocation set is closed and
  non-empty is never composed.
- `MK-THREAD-DESTROY-UCONTEXT-IDENTITY`: the pin identity covers the
  `ucontext` body, where the states are vacuous.

`SCHED-DESTROYER-PREMISES` is amended, not retired: all eight premises are
still Open source truth.

- `MEMBER-PATH-DIRECT-REFUSAL-TEXT`: the `member-path-direct` refusal says
  "beside a hand-off" for a helper destroyer's member invocation.
