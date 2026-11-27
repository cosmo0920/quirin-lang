# HANDOFF-CANDIDATE-OWN-CALLBACKS: census and prediction

Card HANDOFF-CANDIDATE-OWN-CALLBACKS (wave 48, card-high). Frozen before any
implementation code. Base: local `main` `d708641bc` (corpus 936 / 9 / 3,889,
`71387bd3…`; system 93 / 4 / 351; slice 52 / 1 / 347 / 4 Pending; null
1,113 / 1,435 / 2,355 / 18 Unsafe, 84 / 125 / 245, 71 / 103 / 232;
follow-ups 780; Lean 324). Authority: the ledger entry
`HANDOFF-CANDIDATE-OWN-CALLBACKS` and its DISCHARGE; rules 13u, 12m, 12n, 30
and 15w.

**No pause.** No new surface syntax is needed (the composition reads the
clauses the declarations already state), and the corpus has no cycle among
candidates that carry their own member invocations (section 3), so a
coinductive reading would decide nothing more here; none is proposed, and
the least fixed point of rule 15w is implemented as the brief asks.

## 1. The defect, read in the code

`legacy_callback_route::Parser::classified` reads a candidate through
`ResourceContracts::contract` (the function map). A declaration carrying any
callback clause is a retaining declaration (rule 13u), withheld from that map,
so unless it is a hand-off (rule 12m, `handed_class`) or a member-path
declaration (rule 30) it classifies `Missing` (`callback-uncontracted`);
`handed_class` itself returns `Missing` when the declaration carries any
callback clause. Four declarations in force carry their own member
invocations **and** are candidates of some invocation:

| candidate | is a candidate of | its own clauses |
| --- | --- | --- |
| `rd_kafka_oauthbearer_refresh_op` (`rdkafka_sasl_oauthbearer.c:170`) | `rd_kafka_op_s.rko_op_cb` (27 candidates) | `rk`, `rkq`, `rko` borrowed; `member_invoked(rk, rd_kafka_s.rk_conf.sasl.oauthbearer.token_refresh_cb, rk, _, _)` |
| `rd_kafka_poll_cb` (`rdkafka.c:5813`) | `rd_kafka_op_s.rko_serve` (4 candidates); the designator at the served-queue calls (`rd_kafka_q_serve`, `_pop_serve`, …; `rdkafka.c`, eleven sites) | hand-off `#rko: #owner \|-> #callback when #result != #PASS`; twelve member invocations (below) |
| `rd_kafka_consume_cb` (`rdkafka.c:4932`) | the designator at `rd_kafka_q_serve` (`rdkafka.c:4971`) | `rko` consumed; `member_invoked(opaque, consume_ctx.consume_cb, _, _)` |
| `rd_kafka_background_queue_serve` (`rdkafka_background.c:67`) | the designator at `rd_kafka_q_serve` (`rdkafka_background.c:133`) | `rko` consumed; nine `rd_kafka_s.rk_conf.*` member invocations |

The brief names the first two; the last two are the same defect (rule 13u
reads them uncontracted at their one invocation each) and the composition
treats them alike. Every other retaining declaration with callback clauses
(the serve and request families, `rd_kafka_op_destroy`, libc's `qsort`
family, cmetrics, ctraces) is never a candidate, or carries clauses other
than member invocations, which stay outside the composition (`Missing`, as
today).

## 2. The composition each candidate needs

Rule (to be stated as an amendment of 13u, with 12m and 12n): a candidate
whose declaration in force carries its own **member** invocations (and, beside
its custody contract, at most a hand-off) is decided at the invocation as the
composition of its own clauses:

1. **Its own member invocations**, each decided exactly as rule 12n decides
   it in the candidate's own body: over the **run's** set (the pre-pass),
   each inner candidate classified with the candidate itself as the invoker
   (rule 12m's caller branch check from the candidate's body), then
   `decide_run` (E1: a closed empty set invokes nothing). Only the run's set:
   a unit's own set is never used here, because the fold checks the members
   of the declarations a row lists, and a row does not list the composed
   candidate (`MEMBER-CANDIDATE-DEPENDENCIES-UNLISTED`); without the pre-pass
   the composition refuses, named.
2. **Its formals' modes**: its custody contract, strengthened by every
   requirement its inner candidates put on its own parameters (rule 13u's
   `resource_owned(n)` on a parameter actual becomes `resource_owned(n)` on
   that formal), then classified against the invocation's actuals as today.
3. **Its hand-off**: rule 12m's caller branch check at the outer invoker, as
   today, after the composition.
4. **Its member-path states**, and those of its inner candidates: a state at
   one of its formals that the outer invocation hands at a **parameter**
   position would reach the outer caller's state through two levels; it
   refuses (fail closed). At a `_` or `supplied_owned` position it hands an
   object that was never the caller's (rule 30) and changes nothing.

The candidate counts as contracted only when every own invocation decides;
otherwise it refuses, naming the invocation and the inner refusal (new reason
key `callback-own-invocation-refused`, checked after `callback-uncontracted`).
A declaration whose own callback clauses include anything but member
invocations (a formal invocation, a table, a registry) stays uncontracted as
a candidate.

### 2.1 Per candidate (run sets from the base's corpus sidecar, OP-CALLBACK-MEMBER-PATH-RELEASE `artifacts/callback-members-corpus.json`)

**`rd_kafka_oauthbearer_refresh_op`.** One own invocation:
`token_refresh_cb` over the run's closed set {`oauthbearer_token_refresh_cb`
(Fluent Bit, `fluent_bit/logging.qv`), `rd_kafka_oauthbearer_unsecured_token`
and the four OIDC refreshers}, each declared the identity over three borrowed
formals: **decides** (identity, no requirement, no member path). Composed, it
is its custody contract (three borrowed formals, `resource_return_none()`):
the identity at `rd_kafka_op_destroy`'s `rko_op_cb(_, _, rko)`.

**`rd_kafka_poll_cb`.** Its twelve own invocations, in clause order:

| # | member | run set | decides |
| --- | --- | --- | --- |
| 1 | `rk_conf.rebalance_cb` | closed, empty | yes (E1) |
| 2 | `rk_conf.error_cb` | closed, empty | yes |
| 3 | `rk_conf.dr_msg_cb` | {`cb_kafka_msg`}, identity | yes |
| 4 | `rk_conf.dr_cb` | closed, empty | yes |
| 5 | `rk_conf.throttle_cb` | closed, empty | yes |
| 6 | `rk_conf.stats_cb` | closed, empty | yes |
| 7 | `rk_conf.log_cb` | {`cb_kafka_logger`, `rd_kafka_log_print`}, identities | yes |
| 8 | `rk_conf.consume_cb` | closed, empty | yes |
| 9 | `rk_share_consumer.share_acknowledgement_commit_cb` | closed, empty | yes |
| 10 | `rd_kafka_op_s.rko_u.offset_commit.cb` | **open** (the union key: a cast, a `memmove` of `rd_kafka_group_member_s`, whole-object writes of `rd_kafka_fetch_pos_s` and `rd_kafka_replyq_s`) | **no** (`callback-member-open`) |
| 11 | `rd_kafka_op_s.rko_u.terminated.cb` | **open** (the same) | **no** |
| 12 | `rd_kafka_op_s.rko_op_cb` (`_, _, rko`) | closed over 27 | **no** by reading: its three `== KEEP` hand-off candidates (`rd_kafka_txn_op_send_offsets_to_transaction`, `rd_kafka_admin_worker`, `…query_leaders_async_worker_op_cb`) are checked with `rd_kafka_poll_cb` as the invoker handing its own parameter `rko`, and `rd_kafka_poll_cb` returns `KEEP` to its caller (`rd_kafka_op_call` returns it unchanged), which rule 12m refuses at the invoker ("returns to its caller where the callback took its parameter") |

So `rd_kafka_poll_cb` **refuses**, at its first undecided invocation (10,
`rko_u.offset_commit.cb` open). Its hand-off is not reached. Invocation 12
would refuse even with 10 and 11 closed: rule 12m does not compose a
hand-off through an invoker whose own hand-off (`!= PASS`) covers the
returned `KEEP`; that composition is a residual, named and not built here.

**`rd_kafka_consume_cb`.** One own invocation, `consume_ctx.consume_cb`
(object `opaque`), closed and empty over the run: **decides** (E1). Composed,
`rko` consumed: `TakesSupplied` at `rd_kafka_q_serve`'s
`callback(_, _, supplied_owned, cb_type, opaque)`.

**`rd_kafka_background_queue_serve`.** Nine own invocations (`rk_conf`
`background_event_cb`, `rebalance_cb`, `error_cb`, `dr_msg_cb`, `dr_cb`,
`throttle_cb`, `stats_cb`, `log_cb`, `consume_cb`), every set closed (empty,
or the declared identities above): **decides**. Composed, `rko` consumed:
`TakesSupplied` at its `rd_kafka_q_serve`.

## 3. Cycles

The graph "candidate → the candidates of its own invocations", over the four
composed candidates: `rd_kafka_oauthbearer_refresh_op` → the six refreshers
(none carries a callback clause); `rd_kafka_poll_cb` → the rk_conf sets
(declared identities), the two open union members, and `rko_op_cb`'s 27
(only `rd_kafka_oauthbearer_refresh_op` among them composes, and it reaches
no `rko_serve` or `rko_op_cb`); `rd_kafka_consume_cb` → none;
`rd_kafka_background_queue_serve` → the rk_conf sets. `rd_kafka_poll_cb` is a
candidate of `rko_serve` only, which no composed candidate invokes. **No
cycle in the corpus**, direct or through others. The least fixed point is
built anyway (a candidate met again while its own composition is being
decided refuses, `callback-own-invocation-refused` naming the cycle) and
tested on fixtures (a self-cycle, a two-candidate cycle).

## 4. A member copied before its call

`rd_kafka_op_handle` (`rdkafka_op.c:1096`, identical at both revisions):

```c
if (rko->rko_serve) {
        callback              = rko->rko_serve;
        opaque                = rko->rko_serve_opaque;
        rko->rko_serve        = NULL;
        rko->rko_serve_opaque = NULL;
}
…
if (callback)
        res = callback(rk, rkq, rko, cb_type, opaque);
```

`callback` is the function's own formal, assigned once in the body, from
`rko->rko_serve` exactly (`rko` a formal the body never assigns), never
assigned otherwise, incremented or address-taken, then called directly. The
brief's "exact copy" is "an unmodified local, assigned once, from the
member". **Reading taken (flagged for the integrator):** a formal is an
automatic object of its function, so a formal so copied is exact; the site
then reads as an invocation of the member (the value copied) as well as of
the formal (the value it arrived with, on the path that skips the copy). For
rule 12m that is sound in both directions: the member's value is invoked only
there, and the site's handling of the result is the same whichever value is
called. Anything else refuses as today: two assignments, an assignment from
another expression (a cast, a call, another member, another variable), a
compound assignment or increment, an address taken, an object that is
reassigned, or a call through anything but the variable itself.

The caller branch check, with the copy followed (rule 12m's census tool,
`examples/handoff_census.rs`, to be re-run): `rd_kafka_poll_cb` at the
`rko_serve` invocation of `rd_kafka_q_serve0`, `_serve` and
`_serve_maybe_consume` — **admitted** (the same site and handling as the
formal path admitted today); at the three pop serves and `rd_kafka_op_req0`,
`_req`, `_req2` — **refused** (`rd_kafka_q_pop_serve0` returns the op for
every result outside HANDLED, KEEP and YIELD).

## 5. Predicted movement

**Corpus** (936 / 9 / 3,889): **no status move; no Safe gain; no Safe →
non-Safe change.** Leaves only:

* **`rd_kafka_op_destroy`'s 131 leaves (83 functions)** naming
  `rd_kafka_oauthbearer_refresh_op` **decide**: with it composed, the
  invocation's 27 candidates are the 17 declared identities, the six
  member-path candidates (custody contracts), and the three `== KEEP`
  hand-offs, which rule 12m admits at `rd_kafka_op_destroy`
  (`rd_assert(res != KEEP)`; measured admitted for
  `rd_kafka_txn_op_send_offsets_to_transaction` by the hand-off card's
  census). The call then reads as its custody contract with the member-path
  composition (rule 30) and the arms (rule 12o). Every one of the 83 rows
  carries other leaves (the base's leaves artifact lists none decided by
  these alone), so each stays Unknown. Where the refusal had set the path
  unmodelled, later statements are now read: a row may **gain** leaves after
  the call (arm or member-path refusals, later calls). Expected: the 131
  leaves gone, a smaller number gained in some of the 83 rows.
* **The 4 `rd_kafka_poll_cb` leaves** (`rd_kafka_offsets_for_times`,
  `rd_kafka_op_req`, `rd_kafka_op_req2`, `rd_kafka_query_watermark_offsets`):
  * the two at `rko_serve` (through `rd_kafka_op_req0` / `rd_kafka_op_req`)
    trade to `rd_kafka_toppar_op_serve` `callback-uncontracted` (the next
    candidate in the set's order with no contract;
    `OP-CALLBACK-CANDIDATES-APP-CALLBACKS`);
  * the two at a `rd_kafka_q_serve` designator trade to
    `callback-own-invocation-refused` naming `rd_kafka_poll_cb`'s own
    `rd_kafka_op_s.rko_u.offset_commit.cb` (`callback-member-open`,
    the union key, `RKO-OP-CB-RUN-SET-OPEN-THROUGH-UNION`'s territory).
* The leaves naming `rd_kafka_consume_cb` and
  `rd_kafka_background_queue_serve` at their `rd_kafka_q_serve` decide and
  the call refuses at its next clause, `rko_serve`, naming
  `rd_kafka_toppar_op_serve`.
* Every other `rd_kafka_q_serve` / `_pop_serve` designator leaf naming
  `rd_kafka_poll_cb` trades as the two above.

**System** (93 / 4 / 351) and **slice** (52 / 1 / 347 / 4 Pending): no row
moves (no pre-pass on those routes: a composed candidate refuses there,
named, where it was uncontracted; a leaf text may change). **Null property**:
every row identical on all three routes.

**Next blockers** (to be confirmed per row on the AFTER report): for the 83
destroy rows, their remaining leaves (not this card's); for the 4 poll_cb
rows, `rd_kafka_toppar_op_serve` (uncontracted) and `rd_kafka_poll_cb`'s open
union members (then invocation 12's nested hand-off).

## 6. Tests and Lean (planned)

Tests on route G (`crates/qcore-c-gateway/tests/handoff_candidate_own_callbacks.rs`):
a candidate with a decided own invocation is contracted; one with an
undecided own invocation refuses; a self-cycle refuses; a two-candidate
cycle refuses; an exact local copy is admitted by the caller branch check;
an inexact copy refuses; a formal copied once is admitted.

Lean `QCore.CandidateComposition`: a composed candidate is contracted iff
each of its own invocations decides; the least fixed point never supports a
cycle (any set of candidates each needing one of the set is never
contracted); an exact copy is equivalent to the member (the value called is
one the member held, or the formal's own), an inexact one is not.
