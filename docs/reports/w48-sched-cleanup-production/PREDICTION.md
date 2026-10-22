# W48-SCHED-CLEANUP-PRODUCTION: prediction before implementation

Base: main `e282aaccc` (contract cycle4 merged; its six-route native
acceptance is still pending, so no cycle4 figure is borrowed as a baseline).
Isolated delegated branch `card/w48-sched-cleanup-production`; no main merge or
push. Reserved gateway rule 34.

Target, read from the pinned source (all seven cycle4 inventory hashes match
the corpus inputs under `build/cycle19/c-gateway-wave-pin/monkey/monkey-source`
and both Fluent Bit checkouts at `66910c10` and `046accf6`):

- `sched_thread_cleanup` (mk_scheduler.c:614–632): `mk_list_foreach_safe` over
  the list formal, `mth = mk_list_entry(head, …)`, `mk_http_thread_destroy(mth)`.
  Its planner own leaf is the one uncontracted cross-unit call.
- `mk_http_thread_destroy` (mk_http_thread.c:251–272), inline
  `mk_thread_destroy` (mk_thread_libco.h:64–78): frees the current thread's
  libco params read from the pthread key and clears that key, unlinks
  `mth->_head`, writes a scalar through the borrowed session chain, calls the
  optional `cb_destroy`, a Valgrind client request (`MK_HAVE_VALGRIND` is on in
  the corpus build), `co_delete(th->callee)` and frees `mth->parent`, the
  allocation `mth` is embedded in.

Source readings that matter for cleanup (not assumed): the only inserters
into `sched->threads` / `threads_purge` are `mk_http_thread_create` and
`mk_http_thread_purge`; creation calls `mk_thread_new(sizeof(...), NULL)`, so
every scheduler element's `cb_destroy` is NULL at creation and no other monkey
source writes it. The embedded-data NULL branch returns before insertion, so
it never reaches cleanup. A NULL `co_create` result leaves a linked element
whose `callee` is NULL; `thread_params_set` then switches to it before any
cleanup, so no cleanup state rests on it beyond `co_delete`'s own premise.

Planned generic change (rule 34, element destroyer): a declaration may state
`resource_unpublished(p)` beside `resource_released(p, …)`. A caller applies the
existing rule 10d/10f unlink first (custody of an object this path published,
or rule 10f(2)'s admitted foreign element of a safe iteration over a
formal-rooted container), then the declared release. Every other entry keeps
the unlink refusal; rule 10f(3)'s no-summary gate is unchanged. The production
declaration of `mk_http_thread_destroy` goes in the owning Monkey provider
crate (`qcore-provider-libevent`, `monkey/`), pinned at both audited revisions,
with the parts no generic rule proves carried as named `resource_premise`
clauses reusing cycle4's hypothesis names (parent embedding, callback
footprint with the creation-NULL reading, params TLS ownership, coroutine
deletion, borrowed chain, valid membership, later alias use, Valgrind request
neutrality, no concurrent writer). No neutral-callback fallback.

Predicted corpus movement under the unchanged explicit eight-home link map
(fresh BASE is authoritative): `sched_thread_cleanup` Unknown → Safe, and the
same-unit `mk_sched_threads_destroy_conn` Unknown → Safe if its only leaves are
the destroy call and admitted reads, both resting on the new declaration and
its named premises. `mk_sched_threads_purge` / `mk_sched_threads_destroy_all`
are predicted to stay Unknown: rule 10f(3) keeps a body with a foreign unlink
from publishing a summary, so the planner's two cross-unit frees do not follow
(a caller-side drain rule is a follow-up, not this card).
`mk_http_thread_destroy`'s own row is predicted to stay non-Safe with its body
evidence recording a declared assumption, not validation. No other row should
move; any incidental movement is attributed. System and tail/pause selections
contain no Monkey unit; they are run only if the gateway change could reach
them. No upstream-defect claim; no Safe regression.

Acceptance: full standing gate, provider gate (provider workspace and Python,
inventory, all five package builds and verifications, `pin --check`),
`qcore_contract_corpus.rs` pins updated deliberately, rule 34 Rust with
matching audited Lean registered in an `Audit*.lean`, BASE/TIP corpus
attribution, report and integration amendments. Heavy work serialized on
`/tmp/qcore-c-card-validation-20261022.lock`.
