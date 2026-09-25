# W45-CALLEE-SUMMARY-CASCADE: which callee bodies unblock whom, and the destructor body modeled

Card W45-CALLEE-SUMMARY-CASCADE, 2026-09-26, on base `b160d765` ("Open the
wave-45 increment record"). `call to uncontracted function` is the largest sole
blocker at the baseline: **412** Unknown functions are blocked by nothing else
(9,200 leaves on 2,614 functions). A callee gets a validated W4 summary only
when its own body validates, so this card first measures which callee bodies,
and which blocker families inside them, would unblock the most functions
through summary chains, then models the highest-leverage family no sibling
card owns.

## Headline

**Integration review repair.** The integrator returned the first version of
this card: rule 1 read a released member as untracked and certified
`free(n->buf); free(n->buf); free(n);` `Safe`, and a caller releasing a member
again after a callee that released it, flipping `Unknown` to `Safe` on a
genuine violation (hard invariant 1). The review also asked for the
pre-existing hole this card had registered, `AGGREGATE-NULL-TEST-AFTER-RELEASE`,
to be closed. Both are repaired here, and this report describes the repaired
card; the first version's runs are kept as superseded evidence (§3).

| Measure | BEFORE (baseline) | AFTER | Delta |
|---|---:|---:|---:|
| Functions Safe / provisional Unsafe / Unknown | 306 / 12 / 4,516 | **313 / 12 / 4,509** | +7 / 0 / −7 |
| Units Safe / provisional Unsafe / Unknown / data-only | 3 / 10 / 168 / 4 | **4 / 10 / 167 / 4** | +1 / 0 / −1 / 0 |
| `unsupported` / `unknown_alias` | 43,590 / 3,507 | **43,299 / 3,558** | −291 / +51 |
| `leak` / `summary_conflict` / violations | 37 / 1 / 0 | 37 / 1 / 0 | 0 |
| uncontracted-call leaves / Unknown functions carrying them | 9,200 / 2,614 | 9,194 / 2,613 | −6 / −1 |
| **blocked only by uncontracted calls** / by exactly one callee | **412** / 300 | **411** / 299 | **−1** / −1 |

* **Census** ([§1](#1-the-cascade-census), [census/](census/)). Per-callee
  leverage is small: validating any single callee's body unblocks at most 12
  functions. A cross-unit route over the existing `Safe` home bodies unblocks
  2, so it was not built. The lever is a *family*: removing indirect access
  from the callee bodies unblocks 186, return expressions 37, branch
  conditions 12. Most of the indirect-access family is sub-object stores and
  return expressions, which W45-SUB-OBJECT-STORES and
  W45-RETURN-EXPRESSION-FAMILIES own; the next shape is the destructor body.
* **The change** ([§2](#2-the-rules), gateway constraint 5k,
  `legacy_member_release.rs`, `QCore.MemberRelease`). A member `B->F` handed
  to a `Released { nullable }` position is admitted **only as the last use of
  its place**: `B` is a formal never copied, the place is released once and
  only null-tested before, no loop or `goto` can repeat the release, and `B`
  is released afterwards on every path, so a summary releases its formal. A
  pointer member null test is the typed read of its base; every null test
  whose place dereferences a pointer, the O04-B08 aggregate test included,
  obliges those pointers; and calls to undeclared unit callees that may
  release through a formal refuse an actual owning sub-custody.
* **Arbiter** ([§3](#3-arbiter)). BEFORE reproduces the baseline byte for
  byte. AFTER changes **350 leaves, every one attributed**; **7 functions**
  and the unit `chunkio/cio_error.c` move Unknown → Safe; **no function
  leaves `Safe`** (the aggregate repair adds 17 refusals, all in functions
  already Unknown); no leak or violation leaf is added or removed.
* **Prediction missed on size** ([§3.4](#34-prediction-vs-outcome)): 13–44
  movers predicted, 9 in the first version, 7 after the repair. The rest are
  held by the last-use premises (110 releases) and by declarations the typed
  reader cannot type (254 actuals), both registered.

## 1. The cascade census

**Method** ([scripts/cascade.py](scripts/cascade.py)). Every reported row is a
node. An uncontracted-call leaf in unit U naming f resolves, first rule wins,
to U's own row for f, a header definition U carries (internal or external
linkage, from W44-CORPUS-CALLEE-SUMMARIES' definition sites), another pinned
unit's unique row, several homes, or no pinned body. "f validates" makes f's
node clean; a function becomes clean when every leaf it has is an
uncontracted call to a clean node that a summary route can reach (the same
unit or an internal-linkage header body), to a fixed point. For each callee
the census records its body's own blockers, the functions that become clean
(`unblocked`), and those that only lose their last uncontracted leaf. An
other-unit callee's figure is reported separately as an upper bound for a
cross-unit route. Family leverage removes one blocker family everywhere, or
only from callee bodies, and reruns the fixed point.

| At the baseline | Figure |
|---|---:|
| Unknown functions / with uncontracted-call leaves / only such leaves / one callee | 4,516 / 2,614 / 412 / 300 |
| Leaves by callee body: same unit / other unit / header `static` / no pinned body / header external | 3,737 / 3,139 / 1,281 / 957 / 86 |
| Callees whose validation would unblock ≥ 1 function | 170 |
| Largest single-callee leverage | 12 (`LZ4_decompress_generic`, 121 body leaves) |
| Cross-unit route over existing `Safe` home bodies | 2 unblocked (`cio_file_real_size`, `cmt_initialize`), 17 lose their last call leaf |
| Only-call roots whose cone needs a callee with no same-unit body | 253 of 412 |

| Family removed from callee bodies | Leaves there | Unblocked | Removed everywhere |
|---|---:|---:|---:|
| indirect memory access | 9,707 | **186** | 423 |
| return expression (W45-RETURN-EXPRESSION-FAMILIES) | 187 | 37 | 151 |
| branch condition | 4,419 | 12 | 50 |
| assignment expression | 691 | 5 | 12 |
| braced initializer | 1,216 | 4 | 13 |
| loop condition | 148 | 4 | 5 |

The full ranking is [census/cascade-base.json.gz](census/cascade-base.json.gz)
(before) and [cascade-after.json.gz](census/cascade-after.json.gz) (after);
the head is in [cascade-base.txt](census/cascade-base.txt). Among the cheap
cones ([scripts/root_cost.py](scripts/root_cost.py),
[root-cost.json.gz](census/root-cost.json.gz)) the recurring body is the
destructor:

```c
if (p->m) release(p->m);   /* branch-condition refusal, then indirect access */
release(p);                /* unresolved resource identity: p was poisoned   */
```

**Where released members stand** ([scripts/release_positions.py](scripts/release_positions.py),
[census/release-positions.json](census/release-positions.json),
[scripts/residual_releases.py](scripts/residual_releases.py),
[census/residual-releases.json](census/residual-releases.json)). The W44
call-argument census sink, deduplicated, holds 490 distinct refused actuals at
a `Released` position: 138 typed pointer reads, and 352 the recogniser
refuses (untyped base 112, chain 89, unreadable declarations 75, untyped
member 67, other 9). In the reported rows, 137 statements the lexical gate
refused carry a recognised pointer actual at a `Released` position; after the
card **27 are admitted and 110 stay refused**, in 58 functions
(`rd_kafka_group_list_destroy` 11, a loop; `mk_config_free_all` 10;
`flb_input_chunk_map` 8; clear-style bodies such as
`rd_kafka_metadata_partition_clear` and `rd_kafka_snappy_free_env` that keep
their base). The first version of this report said "135 of 138 admitted": its
script mapped census records to units by `unit_seq`, which does not index the
unit list (180 sequence numbers for 185 units), and it counted lines no
reported row refused. The script now maps each sequence number to the unit
whose reported functions it names, and counts only reported refusals; the 27
equal the 27 `member-release` leaves of §3.1.

**Contracts at a library boundary.** None was added. The largest no-body
callee, `cfl_sds_cat_safe` (170 leaves), reallocates the string its
`cfl_sds_t *` parameter points at in place, which the v1 vocabulary cannot
state (a replacement is result-only); `__builtin_memcpy`/`__builtin_memmove`
have no C prototype for the ABI gate, as W44-VA-START-END-BUILTINS records for
the `va` builtins.

## 2. The rules

[`legacy_member_release.rs`](../../../crates/qcore-c-gateway/src/legacy_member_release.rs),
with thin commented hooks in `legacy_call_argument_access.rs`
(`candidate_arguments`, `probe_call_arguments`, `call_argument_access`) and
`legacy_c.rs` (the module line, `condition_access`, the end of
`build_function_cfg_with`).

1. **A member release, admitted only as the last use of its place.** The
   exact shape `B -> F` at a `Released { nullable: true }` position is
   admitted, lowered with that position `Borrowed` after the unchanged W44
   block (the typed read of `B`, then `ArgumentReads`, which refuses a base
   owning sub-custody and a bound place), **only** if the body satisfies four
   lexical premises, read once per body into `AdmissibleReleases`:
   * **P1** `B` is a formal, and every use of the name `B` is a member access
     `B->G`, a null test of `B` in an `if` condition, or the whole single
     actual of a call whose contract releases its only parameter. No copy,
     declaration, address, cast or other actual of `B` exists.
   * **P2** exactly one occurrence of `B->F` is a release; every other one is
     a null test in an `if` condition before it in token order. The place is
     never copied into a local or another member, passed, written or read
     after the release.
   * **P3** no loop encloses the release (`do … while (0)` is not a loop,
     constraint 5c) and the body has no `goto`, `setjmp` or `longjmp`, so the
     release runs at most once and everything after it is later in token
     order.
   * **P4** a statement `R ( B ) ;` of the body's own block (not under an
     `if`, loop or label) follows, `R` releases its only parameter, and no
     `return` lies between. Every path through the member release therefore
     releases `B`: every later access through `B` meets a released identity,
     and a validated summary says `Released` for `B`. A body that releases
     through a formal without releasing the formal gets no rule 1 and so no
     summary a caller could rely on.

   Not admitted: a non-null release (nothing establishes an untracked member
   non-null), a member address, `B[I]`, `*B`, chains, a consuming position.
   Stated premises: distinct members own distinct objects (the destructor's
   ownership premise; P2 refuses every copy the body makes); W44 premise 4 (a
   stored pointer is tracked only through a bound place or sub-custody); a
   declaration's `Borrowed` covers what is reachable through its argument
   (P-B08-SHALLOW).

   **Why the first version was wrong.** It admitted any recognised pointer
   read at a nullable release and called the released object's liveness
   "outside the property", on the precedent of W43's typed reads. A read is
   custody-neutral; a release is custody, and a double release of it is
   exactly the resource property. The review's two counterexamples and their
   neighbours (a release in a branch arm and after it, in any loop, a null
   test, read or pass after it, a copy of the base, a re-read into a local,
   a callee that keeps its base, a caller after a full destructor) are pinned
   as tests.
2. **A pointer member null test** (`B->F`, `!B->F`, `B->F ==/!= N`,
   `N ==/!= B->F`, `B` a declared record pointer, `F` a pointer member) lowers
   to `IndirectScalarAccess` over `B`: both edges carry the state unchanged,
   and `B` keeps its dereference obligation. It composes with the W44
   condition desugaring (constraint 5h): an operand of `&&`, `||` or
   `__builtin_expect` is lowered the same way.

   **The aggregate null test after release** (`AGGREGATE-NULL-TEST-AFTER-RELEASE`,
   pre-existing). Card O04-B08's aggregate condition lowered `b->item == 0`
   to a bare `BranchOnNull` with no obligation on `b`, so `n = q_alloc();
   q_free(n); if (n->buf) return;` was `Safe` (the scalar `if (n->n)` was
   correctly accused). `oblige_member_null_tests`, a post-pass at the end of
   the CFG build, appends before every `BranchOnNull` whose place dereferences
   a pointer the typed read of every pointer the place reads through (`b` for
   `b->item`; `a` and `a->b` for `a->b->c`), with every edge into the test,
   loop back edges included, redirected to it. The branch and its refinement
   are unchanged; a released, nullable, unresolved or ambiguous base now
   records `UnknownAlias`, and a base settled on a released identity is a
   `use_after_release` candidate from the settled states, exactly as for the
   scalar read. The collection element test (`BranchOnElementNull`) was
   checked and already requires a live owner, falling back to its refusal.
3. **The guard.** A summary validated through rule 1 releases objects reached
   through its formals, which no mode states. A caller that linked a tracked
   child into its actual could release that child again after the call.
   Before every call to a function the unit defines, **the environment does
   not declare, and whose body may release through a formal** (lexically: it
   hands a pointer read to a call, or calls a body that does, to a fixed
   point; `member_releasing_functions`), an appended `ArgumentReads` node
   refuses a plain-name actual whose target owns sub-custody. Every edge into
   the call is redirected to it, so no node id moves. Declared callees are
   excluded because a declaration governs every call site (constraint 8) and
   keeps its P-B08-SHALLOW premise. Two coarser selections were measured and
   rejected:
   * **every unit-defined callee** (first AFTER run, `a0bc229a`,
     [after-first-headline.json](after-first-headline.json)):
     `flb_input_chunk_create` left `Safe` over the declared `static inline`
     `flb_free`, and a leak appeared in
     `rd_kafka_mock_handle_ShareGroupHeartbeat` with no leaf change in that
     function. That leak is an artifact of the over-broad guard; its mechanism
     was not isolated, and it is absent from both later runs;
   * **every undeclared unit-defined callee** (second AFTER run, `f2a15b14`,
     [after-second-headline.json](after-second-headline.json)): corpus
     figures as AFTER, but the gate failed 9 tests (5 gateway `legacy_summary`,
     4 compiler `nullable_removed`): the callers of validated
     `Removed` wrappers own sub-custody by construction and were refused.

No new `LegacyInstruction` variant is added (rule 3 reuses `ArgumentReads`
and the aggregate repair `IndirectScalarAccess`, both already decided by
`admits_descriptor_read`). `Derivation::Derived`, the `B[X++]` forget rule,
collections and constraints 5h/5i are untouched: rule 1 never names a
destination.

**Tests** (`crates/qcore-c-gateway/tests/unit/legacy_member_release.rs`, 19
tests, 33 pinned cases, and two deliberate pin updates):
* the review's counterexamples and their neighbours, none `Safe`: a second
  release on one path, in a branch arm and after it, in both arms and after
  them, in a `for`, a `while` and a `do … while (k)` loop; a null test, a read
  or a pass of the member after its release; the base passed to another
  call; a copy of the base; a re-read into a local; the member copied into
  another member; a `goto`; a `return` between the member and the base
  release; the base released only conditionally; the base tested in a
  condition that is not a null test;
* the summary route, none `Safe`: a callee that releases the member and
  keeps its base, then the caller releases it; a full destructor, then the
  caller releases, null-tests or reads the member or passes the base again;
  the caller copying the member first; an acquired base destroyed, then its
  member released;
* the destructor shapes, `Safe` with a certifying caller: two members and the
  base; the `RD_IF_FREE` `do … while (0)` shape; a `__builtin_expect` null
  test; a null test of the base first (`cmt_histogram_buckets_destroy`); a
  scalar member written first;
* the aggregate repair: `n->buf`, `n->buf == 0`, `!n->buf` and
  `n->next != NULL` after `q_free(n)` are not `Safe`; the same test before
  the release is; the dereferenced prefixes of a place;
* unchanged from the first version: non-null release, member address,
  consuming position and linked owner refuse; the guard refuses a caller
  whose actual owns a linked child, precedes only undeclared unit callees
  that may release through a formal, and that set is closed under calls;
  member null tests admit only single pointer members.

Mutation check (measured once, harness not shipped): with `AdmissibleReleases::admits`
forced to the first version's answer, the counterexample, summary-route and
keeps-its-formal tests fail; with `oblige_member_null_tests` disabled, the
aggregate test fails. Both mutations were applied through an environment
switch in a working copy and the file was restored byte for byte (SHA-256
checked). Pin updates: `tests/o04_b08.rs`'s `o->box == 0` row is now rule 2's
typed read of an unbound formal and stays Unknown on its uncontracted call;
`tests/neighbours.rs`'s declarations-withdrawn before-state of
`flb_input_chunk_create` moves 20 → 21 diagnostics, the new obligation on `ic`
at `if (ic->routes_mask == NULL)` (20 with the repair off in the same tree).

**Lean** (`proof/QCore/MemberRelease.lean`, audited by
`proof/AuditMemberRelease.lean`, 30 theorems, no `sorry`, no project axiom):
* the review repair: `an_admitted_release_is_the_last_use_of_its_place` and
  `no_second_release_follows` (under P2's `Admissible` and P3's
  `ForwardAfter`, no occurrence of the place executes after the release),
  `two_releases_are_not_admissible`, `a_use_after_the_release_is_not_admissible`,
  `a_member_releasing_summary_releases_its_formal` (under P4's
  `ReleasedAfter`, the mode W4 publishes when every exit agrees is
  `released`), `the_witness_destructor_is_admissible`;
* the aggregate repair: `a_dereferencing_null_test_obliges_its_bases` and
  `an_aggregate_null_test_over_a_released_base_is_not_safe` (the read before
  the branch refuses a released base, from
  `QCore.IndirectScalarAccess.a_released_base_refuses`);
* unchanged: admission only at a nullable release, never for a non-null,
  consuming, scalar or baseless read; `the_rewrite_changes_no_tracked_effect`
  and `an_admitted_member_release_releases_no_tracked_site` (under
  `TrackedOnlyThrough`); a linked owner or bound place refuses; the null test
  changes no binding; `a_passing_guard_hides_no_caller_release` (under
  `ReachableOnlyThroughSubCustody`), `the_guard_refuses_an_owner_of_sub_custody`,
  `every_summary_chain_to_a_member_release_is_guarded`; the insertion keeps
  every node id and routes every edge into the call through the guard.

What the Lean does not cover: that the Rust reader establishes P1–P4
(`admissible_places`, `is_null_test`, `loop_spans`) and that C's control flow
without loops and `goto` runs later statements later in token order are review
premises (trust boundary 28), as are the construction premises and the Rust
correspondence.

## 3. Arbiter

`docs/reports/w43-return-expression/scripts/run_audit.py` with the pinned
inputs, default job count.

| Run | Tree | Binary sha256 | Exit | Record |
|---|---|---|---:|---|
| BEFORE | `b160d765` | `0b614369…` (baseline's) | 0 | [before-run.json](before-run.json), [before-identity.json](before-identity.json) |
| CAPTURE | `b160d765`, W44 Clang tee wrapper | same | 0 | private; diagnostics byte-identical to BEFORE |
| CENSUS | `b160d765` + `indirect-census` | private build | 0 | [scripts/run_census.sh](scripts/run_census.sh); diagnostics byte-identical to BEFORE |
| AFTER (first) | `a0bc229a` | `51ba7de3…` | 0 | [after-first-run.json](after-first-run.json), [headline](after-first-headline.json); superseded, §2 rule 3 |
| AFTER (second) | `f2a15b14` | `45dedb35…` | 0 | [after-second-run.json](after-second-run.json), [headline](after-second-headline.json); superseded, §2 rule 3 (gate) |
| AFTER (third) | `5aed4e54` | `1cdc4b03…` | 0 | [after-third-run.json](after-third-run.json); the version the review returned (diagnostics byte-identical to the second run; attribution in [attribution-before-review.json.gz](attribution-before-review.json.gz)) |
| AFTER (fourth) | `5275d58f` | see record | 0 | [after-fourth-run.json](after-fourth-run.json), [headline](after-fourth-headline.json); the repair before P1 admitted a null test of the base: 6 movers |
| AFTER | `1dedce71` | see record | 0 | [after-run.json](after-run.json), [artifacts/](artifacts/) |

**BEFORE vs baseline**
(`docs/reports/w44-collection-allocation/integration/artifacts/`, root-aliased
onto the integrator worktree): diagnostics, chunk-contracts and source
byte-identical; contracts identical after the alias; source-effects differs
only in the checkout-path digests, as in every isolated card.

**AFTER vs BEFORE:** chunk-contracts, contracts, source-effects and source are
byte-identical; the diagnostics differ in 350 leaves. AFTER is archived as the
new baseline in [artifacts/](artifacts/) ([scripts/archive.py](scripts/archive.py)).

### 3.1 Attribution

[`scripts/attribute.py`](scripts/attribute.py) reads each changed leaf against
its analysed line and exits 1 on any unattributed leaf; it exits 0.
Per-leaf list: [attribution.json.gz](attribution.json.gz).

| Class | Removed | Added | What it is |
|---|---:|---:|---|
| `member-release` | 27 | 0 | rule 1, under P1–P4 (128 in the returned version) |
| `member-null-test` | 247 | 0 | rule 2 |
| `member-null-test-desugared` | 10 | 0 | rule 2 through an operand of the W44 condition desugaring |
| `decided-statement` | 1 | 0 | `rd_kafka_q_consumer_propagate(rkq->rkq_fwdq)`: the recursive callee's neutral summary now validates, so W44's `Borrowed` read admits the actual |
| `decided-callee` | 6 | 0 | an uncontracted call to a newly `Safe` same-unit callee |
| `poison-lifted` | 4 | 0 | an unresolved identity the lifted refusal had poisoned |
| `exchange` | 0 | 34 | `unresolved resource identity B` on a line that lost a rule-1/2 refusal: the base the typed read now obliges does not resolve there |
| `decided-same-line` | 0 | 4 | the now-modeled `rd_kafka_consumer_group_metadata_destroy(x)` checks its `Released` actual, which does not resolve (as W44's `same-line` class) |
| `aggregate-base-obligation` | 0 | 17 | the aggregate repair: `unresolved resource identity B` at a condition that null-tests `B->F`, in 13 functions all Unknown before and after (`cmt_counter_create`, `cmt_gauge_create`, `cmt_histogram_create`, `cmt_summary_create`, `cmt_untyped_create`, `cmt_exp_histogram_create`, `ctr_link_create`, `ctr_resource_span_create`, `ctr_span_create`, `flb_input_chunk_map`, `mk_fifo_worker_create`, `mk_mimetype_add`, `mk_server_create`) |

### 3.2 Movers

All seven move Unknown → Safe, each **relative to the declared release
premises** it calls (`free`, `rd_free`, `mk_mem_free`, …):

| Function | Unit | Why |
|---|---|---|
| `cio_error_set` | chunkio/cio_error.c | its only leaf was `if (ch->ctx != NULL)`; the unit moves to `Safe` and re-certifies |
| `cmt_histogram_buckets_destroy` | cmetrics/cmt_histogram.c | `if (!b) return; if (b->upper_bounds) free(b->upper_bounds); free(b);` |
| `rd_kafka_ConsumerGroupListing_destroy` | librdkafka/rdkafka_admin.c | `RD_IF_FREE(grplist->group_id, rd_free); rd_free(grplist);` |
| `rd_kafka_ConsumerGroupListing_free` | librdkafka/rdkafka_admin.c | its only leaf was the call to the destructor above, now summarised `Released` |
| `rd_kafka_consumer_group_metadata_destroy` | librdkafka/rdkafka_cgrp.c | three member `rd_free`s, one member null test, then the base |
| `rd_kafka_q_consumer_propagate` | librdkafka/rdkafka_queue.c | `if (!rkq->rkq_fwdq)`, then the recursive call (neutral route) |
| `mk_config_sanity_check` | monkey-libevent/mk_server/mk_config.c | `if (!server->path_conf_root)` |

**No longer moving, as the review requires:** `rd_kafka_metadata_partition_clear`
and `rd_kafka_snappy_free_env` release members through a formal they keep, so
P4 refuses them and they stay Unknown.

**Corrections from the aggregate repair: none on this corpus.** No function
that was `Safe` at BEFORE leaves `Safe`: the 17 new obligations land in 13
functions already Unknown. The combined tree the integrator measures may
differ; re-run `scripts/attribute.py` there and list any `Safe` → Unknown move
as a correction, never as an upstream defect.

**Transitive movers:** `rd_kafka_ConsumerGroupListing_free` (through the
destructor's `Released` summary). Four more callers lost the uncontracted
leaf of a newly summarised callee but keep other blockers and stay Unknown:
`rd_kafka_q_fwd_set0` and three callers of
`rd_kafka_consumer_group_metadata_destroy` (`rd_kafka_cgrp_handle_SyncGroup_memberstate`,
`rd_kafka_cgrp_offsets_commit`, `unittest_consumer_group_metadata_iteration`),
where the now-modeled call checks an actual that does not resolve.

**Leaks and violations:** none added or removed, so none to classify. The one
leak of the superseded first run is described in §2 rule 3.

### 3.3 Composition with the sibling cards

* **W45-SUB-OBJECT-STORES.** Rule 1 rests on W44's premise 4: a pointer
  stored in memory is tracked only through a bound place or sub-custody. A
  store route that puts a tracked identity into a member without recording
  either would let rule 1 release it untracked; such a route must refuse this
  read, or record the link. Re-run `scripts/attribute.py` on the combined
  tree.
* **W45-RETURN-EXPRESSION-FAMILIES.** The destructors' `_copy` siblings
  (`return X_new(src->a, …)`) and the `rd_kafka_admin_result_ret_*` accessors
  are return-expression shapes; their movement belongs to that card. A
  derived result never reaches rule 1, which admits only memory reads.
* **The guard** adds `ArgumentReads` nodes before calls to undeclared unit
  callees everywhere, so a card that adds field links or collections will see
  its callers refuse at such calls, the refusing direction.

### 3.4 Prediction vs outcome

| Predicted ([PREDICTION.md](PREDICTION.md)) | Measured |
|---|---|
| 13–44 functions Unknown → Safe | **7** (9 in the returned version). The estimate matched release statements textually; the rest are held by the last-use premises (110 releases, `W45-MEMBER-RELEASE-PREMISES`) and by typedef'd bases or members the typed reader does not type (`ctr_id_destroy`, `rd_kafka_Node_destroy`, `destroy_label`, `destroy_chunk_raw`; `W45-MEMBER-RELEASE-UNTYPED-DECLARATIONS`) |
| sole blocker −5 to −20 | **−1** (412 → 411) |
| `unsupported` down by several hundred; `unknown_alias` up | −291; +51 |
| leak 37 ± a few, each classified | 37, unchanged |
| guard: 0–5 functions leave `Safe` | 0 (1 in the superseded first run, fixed by excluding declared callees) |
| units: none | `cio_error.c` Unknown → Safe (its one function moved) |

The appended revision in [PREDICTION.md](PREDICTION.md) covers the repair; it
was written after the repaired runs and says so.

The prediction was frozen after the rules were unit-tested and before any
corpus run of them; the brief asked for it before implementation. The
estimate scripts frozen with it (`predict_estimate.py`, `root_cost.py`,
`cone_lines.py`) are kept exactly as run: they read the worktree-private
BEFORE run and capture at `measure/before/` and `measure/capture/` and are run
from the repository root.

## 4. Validation

Gate on the last code commit `1dedce71`; later commits change documents, the
ledger and report scripts only. Logs are worktree-private under
`measure/gate/` and `measure/provider-gate/`; [gate.json](gate.json) is the
generator input, with [clang-version.txt](clang-version.txt) recorded in this
worktree.

| Command | Exit | Result | Log |
|---|---:|---|---|
| `git submodule update --init --recursive` | 0 | libevent at `d82464a2…` | session output (fresh worktree) |
| `clang --version` | 0 | Ubuntu clang 21.1.8 | [clang-version.txt](clang-version.txt) |
| `python3 scripts/run_test_binaries.py` (`CARGO_INCREMENTAL=0`) | 0 | **5,663 passed / 0 failed / 11 ignored**, 299 summaries in 289 units | `measure/gate/test-binaries.log`, `private/test-binaries/20260925T160037Z-3576850/` |
| `cargo fmt --all -- --check` | 0 | clean | `measure/gate/fmt.log` |
| `cargo clippy --all-targets -- -D warnings` | 0 | clean | `measure/gate/clippy.log` |
| `cargo run -p qcore-compiler --example reevaluation_probe` | 0 | | `measure/gate/probe.log` |
| `cargo run -p qcore-compiler --example reevaluation_followup` | 0 | | `measure/gate/followup.log` |
| `cargo clippy -p qcore-c-gateway --all-targets --features indirect-census -- -D warnings` | 0 | clean | `measure/gate/census-clippy.log` |
| `cargo test -p qcore-c-gateway --features indirect-census --no-fail-fast` | 0 | 1,484 passed / 0 failed / 7 ignored, 66 summaries | `measure/gate/census-test.log` |
| `python3 scripts/check_lean_audits.py` | 0 | **190** audits (`AuditMemberRelease`, 30 theorems) | `measure/gate/lean.log` |
| `python3 docs/reports/remaining-d02/build_matrix.py . --gate docs/reports/w45-callee-summary-cascade/gate.json` | 0 | 397 follow-ups, 111 contract modules, 190 Lean audits | `measure/build-matrix2.log` |
| `python3 scripts/build_docs_index.py`, then `--check` | 0 / 0 | regenerated and matching | `measure/docs-index-check3.log` |
| `python3 -m unittest discover -s scripts/tests` (after the final regeneration) | 0 | **351** tests | `measure/python-final3.log` |
| `cargo test --manifest-path providers/Cargo.toml --workspace --locked --offline --no-fail-fast` | **101** | 122 passed / **2 failed** / 0 ignored, 39 summaries: the two process-provider rows of `configured_source_contract` timed out (`QP006`) under load | `measure/provider-gate/ptest.log` |
| `… -p qcore-provider-conformance --test configured_source_contract -- --test-threads=1` (constraint 16 isolation) | 0 | 6 passed / 0 failed | `measure/provider-gate/pisolated.log` |
| provider `fmt --check` / `clippy -D warnings` / `contracts.py check` | 0 / 0 / 0 | clean / clean / unchanged | `measure/provider-gate/` |
| `providers/package.py build`, then `verify` ×5 | 0 / 0 | 5 packages | `measure/provider-gate/pbuild.log`, `pverify.log` |
| `python3 -m unittest discover -s providers/tests` | 0 | 15 tests | `measure/provider-gate/ppython.log` |

No provider file changed. The provider workspace ran right after the full core
gate, and its two timed-out rows are process providers under a declared
timeout; they pass in isolation and were not weakened.

**Counts.** This card adds 19 Rust tests (the `legacy_c::member_release` unit
module), two pin updates (`tests/o04_b08.rs`, `tests/neighbours.rs`) and one
Lean audit. The base's own figures were not re-measured, so the delta is
stated by test.

**Runs that are not the gate, disclosed.** The returned version's gates
(`measure/gate-first/`, `-second/`, `-third/`, `measure/provider-gate-first/`)
are superseded evidence. On `f2a15b14` the runner failed 11 tests: 9 on the
over-broad guard (§2 rule 3), the `o04_b08` row before its pin update, and
the load-sensitive `execution_quota` row (constraint 16), which passed in every
later full run. On `5aed4e54` everything passed except the census-feature
sink inventory, a failure the base already has (§6), repaired by `1d1f226b`.
The returned version's gate on `1d1f226b` passed in full (5,658 / 0 / 11;
provider 124 / 0 / 0). During the repair, the gateway suite first failed the
`neighbours` before-state pin (20 → 21, the new aggregate obligation, updated
deliberately). A Python run before a matrix regeneration fails the two
`build_matrix` consistency checks by design, as it did once here.

## 5. Ledger and status

* **Retired:** `AGGREGATE-NULL-TEST-AFTER-RELEASE`, discharged in full by the
  repair (§2 rule 2): every `BranchOnNull` whose place dereferences a pointer
  now obliges every pointer it reads through, and the element test already
  required a live owner. It was registered only on this unmerged branch, so
  its retirement removes this card's own entry; it never reached main.
  `W44-CALLEE-DESTRUCTOR-SUMMARY` stays open: list walks, typedef'd shapes
  and clear-style bodies remain. `W44-CALLEE-CROSS-UNIT-SUMMARY` is
  re-measured (2 functions reachable) and left open. `W44-CALLEE-NO-CORPUS-BODY`
  gains no contract (§1).
* **Registered** ([followups.json](followups.json), 393 → 397):
  `W45-MEMBER-RELEASE-PREMISES` (110 recognised releases in 58 functions
  held by P1–P4), `W45-MEMBER-RELEASE-UNTYPED-DECLARATIONS` (254 release
  actuals), `W45-MEMBER-RELEASE-CHAINS` (89), and `W45-CALLEE-CASCADE-RESIDUAL`
  (411 sole-blocked, ranked).
* `corpus.cycle19_raw_units` points at [artifacts/](artifacts/); its contents
  before this card are in [previous-corpus-row.json](previous-corpus-row.json)
  ([scripts/update_ledger.py](scripts/update_ledger.py), now idempotent: it
  replaces this card's own entries on a rerun).

## 6. For the integrator

* **Constraint 7:** record this card as leaving the leak pin at 37: 7
  functions moved, no leak leaf added or withdrawn.
* **Numbering:** the rule is gateway constraint **5k**, after the 5j of
  W45-RETURN-EXPRESSION-FAMILIES, which is integrated first. That card also
  lists `QCORE_CALL_ARGUMENT_CENSUS` among the census sinks, the same fix as
  this card's `1d1f226b`: expect a trivial conflict in `lib.rs`; keep one
  entry.
* **A pre-existing gate failure, repaired.** The census-feature sink
  inventory (`census_sink_inventory::every_declared_sink_forces_one_job`)
  fails on the base: W44-INDIRECT-CALL-ARGUMENT-ACCESS's process-global
  `QCORE_CALL_ARGUMENT_CENSUS` stream is not in `census_sink_variables()`, so
  a parallel audit would interleave it. Commit `1d1f226b` lists it (and makes
  its module crate-visible to name the constant). The census run above set
  other listed sinks, so it ran one unit at a time regardless.
* **Files likely to conflict:** `crates/qcore-c-gateway/src/legacy_c.rs` (the
  module line after `call_argument_access`, whose declaration is now
  `pub(crate)`; one `Parser` field `member_releases` and its initialisation
  with the `AdmissibleReleases::read` line before `let mut parser`;
  `condition_access`'s chain-condition `else`; the guard and aggregate-test
  post-pass calls before `Ok(cfg)` in `build_function_cfg_with`); `legacy_call_argument_access.rs`
  (`AdmittedArgument` fields, `candidate_arguments`, `admitted_call`,
  `call_argument_access`, `pub(crate) mod census`);
  `crates/qcore-c-gateway/src/lib.rs` (`census_sink_variables`);
  `crates/qcore-c-gateway/tests/o04_b08.rs` and `tests/neighbours.rs` (one
  pin update each);
  `proof/QCore.lean` (one import after
  `CallArgumentAccess`); `docs/agents/gateway.md` and `docs/agents/README.md`
  (5k, after the 5j of W45-RETURN-EXPRESSION-FAMILIES); `docs/QCORE_CONTRACT_SPEC_r0_3.md` (one paragraph after the W44
  derivation-scope paragraph); `docs/STATUS.md`,
  `docs/reviews/r0_3/acceptance_matrix.json`, `docs/reports/INDEX.md`,
  `docs/development/increments/wave-45.md`.

## Reproduction

```sh
python3 docs/reports/w43-return-expression/scripts/run_audit.py <before> b160d765
docs/reports/w44-corpus-callee-summaries/scripts/capture_run.sh <base binary> <capture>
bash docs/reports/w45-callee-summary-cascade/scripts/run_census.sh <indirect-census binary> <census>
python3 docs/reports/w45-callee-summary-cascade/scripts/cascade.py <before>/resource-diagnostics.json \
    docs/reports/w44-corpus-callee-summaries/census/definition-sites.json.gz <out.json> \
    docs/reports/w44-corpus-callee-summaries/census/scope-census.jsonl.gz
python3 docs/reports/w45-callee-summary-cascade/scripts/release_positions.py <census>/call-argument.jsonl <out.json>
python3 docs/reports/w43-return-expression/scripts/run_audit.py <after> 1dedce71
python3 docs/reports/w45-callee-summary-cascade/scripts/compare.py <before>/resource-diagnostics.json <after>/resource-diagnostics.json <out.json>
python3 docs/reports/w45-callee-summary-cascade/scripts/attribute.py <before>/resource-diagnostics.json \
    <after>/resource-diagnostics.json <capture> <attribution.json>
python3 docs/reports/w45-callee-summary-cascade/scripts/residual_releases.py <census>/call-argument.jsonl \
    <census>/resource-diagnostics.json <after>/resource-diagnostics.json <out.json>
```
