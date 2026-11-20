# SP4-SYMBOLIC-LIBEVENT-PREMISES

Base origin/main `1cd7da6b8` (SP1-SYMBOLIC-OPENSSL-PREMISES landed). User
decision 2026-11-20 (verbatim): "we don't want to write down the sentences
on premise clauses. We really dislike this kind of results so we must write
down as condition symbols like |-> or other types of logical symbols to
represent the prerequisites which compose for premise conditions." SP1
converted OpenSSL; the user launched SP2-SP4 on 2026-11-21. This card
converts `qcore-provider-libevent`.

[Prediction](PREDICTION.md) (committed first, `95077c00d`): no status move on
any route; content changes only where a premise is rendered; one lock entry
re-pinned.

## 1. What changed

* **The seven premises** of `mk_http_thread_destroy`
  (`providers/qcore-provider-libevent/contracts/monkey/intrusive_list.qv`)
  state C04.18.9 formulas instead of English sentences. Names, the typed
  refinements (`quant proposition #sched_*`), the
  `resource_premise_decomposition` attachments and the
  `resource_premise_flow_v1` chains are unchanged.
* **Vocabulary.** 40 terms in an SP4 block appended after SP1's entries, in
  the Rust table (`qcore_frontend::premise_formula::VOCABULARY`), the
  specification table (C04.18.9) and the Lean table
  (`QCore.PremiseFormula.vocab`). No token, operator, sort, constant or
  grammar production is added, so G05-formula is unchanged. 11 decision
  vectors are appended in an SP4 block too.
* **Prose ban.** The libevent crate has no checker plugin. Its contract-tree
  test (`providers/qcore-provider-libevent/src/main.rs`,
  `every_premise_is_a_formula`) is the nearest equivalent of OpenSSL's
  `prose_premise`: it refuses any `requires resource_premise(NAME, "…")` in
  any owned module and counts the 7 formula premises. It runs in the provider
  test step of every gate.
* **Semantics unchanged.** A formula premise is metadata exactly as a string
  one: no consumer that decides a verdict reads it.
* **Rule 13h** amended; no new rule id.

## 2. Census

| | Before (base) | After (tip) |
|---|---|---|
| prose `resource_premise` clauses in `qcore-provider-libevent` | 7, all in `monkey/intrusive_list.qv`, all on `mk_http_thread_destroy` | 0 |
| formula `resource_premise` clauses | 0 | 7 (same file, same declaration) |
| by name | `scheduler_exact_parent_embedding`, `scheduler_tls_params_owned`, `scheduler_callee_live`, `scheduler_borrowed_chain_live`, `scheduler_valid_membership`, `scheduler_no_later_alias_use`, `scheduler_valgrind_request_neutral`, one each | the same seven, one each |
| clauses of the module / of the declaration | 87 / 67 | 87 / 67 |

Counted per clause, comments excluded; the tip count is the provider test's
assertion.

## 3. Vocabulary added (SP4 block)

A member of a record is an object here, a cell (C04.19.6's member paths), as
is a slot holding a pointer.

| Term | Sort | Meaning |
|---|---|---|
| `#singleton(o)` | obj → set(obj) | defined: `{o}`; `quant all #x in #singleton(E): F` binds `#x` to `E` |
| `#live` | set(obj) | storage allocated, not yet released, at every point of the call that reads, writes or releases the object |
| `#inside(o)` | obj → set(obj) | storage within `o`'s allocation, so releasing `o` ends it |
| `#released_only_by_call` | set(obj) | nothing but the call releases the object |
| `#released` | set(obj) | the call releases it, directly or through a function it runs |
| `#ended` | set(obj) | the call ends its lifetime, by releasing it or an object whose storage holds it |
| `#tracked` | set(obj) | the caller's projection tracks it |
| `#overlaps(S)` | set(obj) → set(obj) | storage overlapping an object of `S` |
| `#stores(S)` | set(obj) → set(obj) | the member cells of `S`'s objects the call writes |
| `#concurrent` | set(fn) | functions running concurrently with or reentrantly into the call; `\|->` reads each run overlapping the call |
| `#no_writes(S)`, `#no_releases(S)`, `#no_resumes(S)` | set(obj) → eff | the run writes / releases (frees, deletes) / resumes no object of `S` |
| `#unread_after`, `#unresumed_after` | set(obj) | cells nothing reads / through which nothing resumes a thread or coroutine, after the call returns |
| `#mk_parent(o)`, `#mk_callee(t)`, `#mk_session(o)`, `#mk_channel(s)`, `#mk_event(c)` | obj → obj | the object Monkey's member `parent`, `callee`, `session`, `channel`, `event` designates |
| `#mk_type(e)`, `#mk_link(o)` | obj → obj | the member cell `type` of an mk_event; the link member `_head` of an mk_http_thread |
| `#thread_data(t)` | obj → set(obj) | the object at `MK_THREAD_DATA(t)` |
| `#co_deletable`, `#suspended`, `#co_active` | set(obj) | coroutines `co_delete` accepts; suspended while the call runs; the one `co_active()` returns |
| `#created_for(t)` | obj → set(obj) | coroutines created for the mk_thread `t` |
| `#lists(n)`, `#nodes(l)` | obj → set(obj) | list heads `n` is linked in; nodes of the list headed `l`, head included |
| `#caller_list(n)` | obj → obj | the head of the list the caller reaches `n` through |
| `#circular_lists`, `#reciprocal` | set(obj) | heads of valid circular mk_lists; nodes with `n->next->prev = n` and `n->prev->next = n` |
| `#libco_params_key` | obj | the current thread's `mk_http_thread_libco_params` slot |
| `#holds(c)`, `#held_at_return(c)` | obj → set(obj) | what the pointer cell designates when read / at return; none for NULL |
| `#sole_owned(c)` | obj → set(obj) | allocations the cell alone owns |
| `#getspecific_ok`, `#setspecific_ok` | set(obj) | key slots whose every `pthread_getspecific` / `pthread_setspecific` during the call succeeds |
| `#aliases(t)` | obj → set(obj) | cells left designating `t` or an object in its storage at return: `request->thread`, `channel.thread`, the mk_thread key among them |
| `#stack_deregister(t)` | obj → set(fn) | `VALGRIND_STACK_DEREGISTER(t->valgrind_stack_id)` as `mk_thread_destroy(t)` runs it under `MK_HAVE_VALGRIND`; none otherwise |

The full meanings are the specification's (C04.18.9, SP4 block) and the Rust
table's, word for word. Monkey's accessors carry an `mk_` prefix so they
cannot collide with another provider's member names.

## 4. The seven formulas

Laid out in the module one step per line; the canonical printings:

* `scheduler_exact_parent_embedding`:
  `quant all #m in #args: #mk_parent(#m) in #live && #m in #thread_data(#mk_parent(#m)) && #m in #inside(#mk_parent(#m)) && #mk_parent(#m) in #released_only_by_call`
* `scheduler_tls_params_owned`:
  `#libco_params_key in #getspecific_ok && #libco_params_key in #setspecific_ok && quant all #p in #holds(#libco_params_key): #p in #live \ #overlaps(#tracked) && #p in #sole_owned(#libco_params_key) && #p in #freed \ #held_at_return(#libco_params_key)`
* `scheduler_callee_live`:
  `quant all #m in #args: quant all #c in #singleton(#mk_callee(#mk_parent(#m))): #c in #freed && #c in #co_deletable && #c in #created_for(#mk_parent(#m)) && #c in #suspended \ #co_active && #concurrent |-> (#no_resumes(#singleton(#c)) && #no_releases(#singleton(#c)))`
* `scheduler_borrowed_chain_live`:
  `quant all #m in #args: quant all #s in #singleton(#mk_session(#m)): quant all #ch in #singleton(#mk_channel(#s)): quant all #ev in #singleton(#mk_event(#ch)): #s in #live && #ch in #live \ (#released | #ended) && #ev in #live \ (#released | #ended) && #mk_type(#ev) in #stores(#singleton(#s) | #singleton(#ch) | #singleton(#ev)) && quant all #w in #stores(#singleton(#s) | #singleton(#ch) | #singleton(#ev)): #w in #singleton(#mk_type(#ev))`
* `scheduler_valid_membership`:
  `quant all #m in #args: quant all #n in #singleton(#mk_link(#m)): quant all #l in #singleton(#caller_list(#n)): #l in #lists(#n) && (quant all #k in #lists(#n): #k in #singleton(#l)) && #l in #circular_lists && (quant all #k in #nodes(#l): #k in #reciprocal) && #concurrent |-> #no_writes(#nodes(#l) | #singleton(#mk_callee(#mk_parent(#m))) | #singleton(#mk_session(#m)) | #singleton(#mk_channel(#mk_session(#m))) | #singleton(#mk_event(#mk_channel(#mk_session(#m)))) | #singleton(#libco_params_key))`
* `scheduler_no_later_alias_use`:
  `quant all #m in #args: quant all #a in #aliases(#mk_parent(#m)): #a in #unread_after && #a in #unresumed_after`
* `scheduler_valgrind_request_neutral`:
  `quant all #m in #args: #stack_deregister(#mk_parent(#m)) |-> (#no_writes(#tracked) && #no_releases(#tracked))`

`#m` ranges over `#args`: `mk_http_thread_destroy` has one pointer argument,
`mth`. The largest formula has 180 tokens (the bound is 1024).

## 5. Faithfulness: prose to formula

Every phrase of each base statement, in order.

**`scheduler_exact_parent_embedding`**

| Prose | Formula |
|---|---|
| "mth" | `quant all #m in #args` |
| "is the mk_http_thread at MK_THREAD_DATA of" | `#m in #thread_data(…)` |
| "the one … mk_thread allocation mth->parent names" | `#mk_parent(#m)`: a function, so one object, the mk_thread the member names |
| "live" | `#mk_parent(#m) in #live` |
| "which nothing but this call releases" | `#mk_parent(#m) in #released_only_by_call` |
| "so that freeing it ends the storage of mth" | `#m in #inside(#mk_parent(#m))`, stated as its own conjunct rather than left as a consequence |
| "(scheduler decomposition exact_parent_embedding and parent_live" | the hypothesis names; no assumption of its own. The conjuncts carry the attached atoms' content: `sched_parent_data_identity` (`#thread_data`), `sched_parent_storage` (`#live`, `#inside`), `sched_parent_sole_releaser` (`#released_only_by_call`) |
| "; established at creation by mk_http_thread_create, not by this body)" | provenance: says where the assumption comes from, adds none |
| "that the call takes the object mth->parent names and leaves the member designating it is the state parent \|-> #stale" | not a premise: the declaration's clause `quant live #mth: mk_http_thread.parent \|-> #stale` states it, unchanged |

**`scheduler_tls_params_owned`**

| Prose | Formula |
|---|---|
| "the mk_http_thread_libco_params key of the current thread" | `#libco_params_key` |
| "holds NULL or one live allocation" | `quant all #p in #holds(#libco_params_key): #p in #live …`: NULL is the empty `#holds`, where the body holds trivially; a pointer designates at most one object |
| "it alone owns" | `#p in #sole_owned(#libco_params_key)` |
| "disjoint from every caller-tracked object" | `#p in #live \ #overlaps(#tracked)` |
| "pthread_getspecific … of that key succeed" | `#libco_params_key in #getspecific_ok` |
| "and pthread_setspecific of that key succeed" | `#libco_params_key in #setspecific_ok` |
| "so the freed params" | `#p in #freed …` |
| "are no longer reachable from the key" | `… \ #held_at_return(#libco_params_key)` |
| "(scheduler decomposition tls_params_owned and tls_api_success)" | the hypothesis names |

**`scheduler_callee_live`**

| Prose | Formula |
|---|---|
| "th->callee (th = mth->parent)" | `quant all #c in #singleton(#mk_callee(#mk_parent(#m)))` |
| "which mk_thread_destroy deletes" | `#c in #freed` (R3) |
| "is a coroutine co_delete accepts" | `#c in #co_deletable` |
| "created for this thread" | `#c in #created_for(#mk_parent(#m))` |
| "suspended" | `#c in #suspended \ …` |
| "and not co_active" | `… \ #co_active` |
| "with no concurrent resume" | `#concurrent \|-> (#no_resumes(#singleton(#c)) && …)` |
| "or deletion" | `… && #no_releases(#singleton(#c)))`: deleting a coroutine releases it |
| "(scheduler decomposition callee_live and suspended_coroutines)" | the hypothesis names |

**`scheduler_borrowed_chain_live`**

| Prose | Formula |
|---|---|
| "mth->session" | `quant all #s in #singleton(#mk_session(#m))` |
| "is live where it is read" | `#s in #live` (`#live` is liveness at every point the call reads, writes or releases the object) |
| "and its channel" | `quant all #ch in #singleton(#mk_channel(#s))` |
| "and that channel's event" | `quant all #ev in #singleton(#mk_event(#ch))` |
| "are live" | `#ch in #live …`, `#ev in #live …` |
| "borrowed objects this call neither releases" | `\ (#released \| …)` on each (R5) |
| "nor ends" | `\ (… \| #ended)` on each |
| "the call writes one scalar through them, event->type" | `#mk_type(#ev) in #stores(…)` (it writes `event->type`) `&& quant all #w in #stores(…): #w in #singleton(#mk_type(#ev))` (and no other cell of the session, channel or event); `#mk_type` is one scalar cell (R3) |
| "(scheduler decomposition borrowed_chain_live)" | the hypothesis name |
| "that the session itself is not taken is the state session \|-> #kept" | not a premise: the clause `quant live #mth: mk_http_thread.session \|-> #kept` states it, unchanged |

**`scheduler_valid_membership`**

| Prose | Formula |
|---|---|
| "mth->_head" | `quant all #n in #singleton(#mk_link(#m))` |
| "is linked in … mk_list, the one its caller reaches it through" | `quant all #l in #singleton(#caller_list(#n)): #l in #lists(#n)` |
| "exactly one" | `(quant all #k in #lists(#n): #k in #singleton(#l))`: no other list |
| "valid circular" | `#l in #circular_lists` |
| "with reciprocal links" | `(quant all #k in #nodes(#l): #k in #reciprocal)`, every node of the list, `mth->_head` included |
| "and no concurrent or reentrant writer" | `#concurrent \|-> #no_writes(…)` (`#concurrent` holds the reentrant runs too) |
| "of that list" | `#nodes(#l)` |
| "the coroutine" | `#singleton(#mk_callee(#mk_parent(#m)))` |
| "the borrowed chain" | `#singleton(#mk_session(#m)) \| #singleton(#mk_channel(…)) \| #singleton(#mk_event(…))` (R6) |
| "or the key" | `#singleton(#libco_params_key)` (R1) |
| "(scheduler decomposition valid_membership and no_concurrent_writer)" | the hypothesis names |

**`scheduler_no_later_alias_use`**

| Prose | Formula |
|---|---|
| "no alias the source leaves behind" | `quant all #a in #aliases(#mk_parent(#m))` |
| "request->thread" | a member of `#aliases` (its meaning names it) (R2) |
| "channel.thread" | a member of `#aliases` (R2) |
| "or the current thread's mk_thread key" | a member of `#aliases` (R2) |
| "is read" | `#a in #unread_after` |
| "or resumed" | `#a in #unresumed_after` |
| "after the call" | both terms' "after the call returns" |
| "(scheduler decomposition no_later_alias_use)" | the hypothesis name |

**`scheduler_valgrind_request_neutral`**

| Prose | Formula |
|---|---|
| "the Valgrind client request VALGRIND_STACK_DEREGISTER" | `#stack_deregister(#mk_parent(#m))` |
| "compiled in under MK_HAVE_VALGRIND" | the term holds nothing where `MK_HAVE_VALGRIND` does not compile the request in, so the formula is trivially true there, as the prose is |
| "reads th->valgrind_stack_id" | the term is the request on `t->valgrind_stack_id` for `t = #mk_parent(#m)`, which reads it |
| "and changes no object the projection tracks" | `\|-> (#no_writes(#tracked) && #no_releases(#tracked))` (R4) |

**Readings recorded** where a phrase admitted more than one parse. None
needed a user decision: each is the reading the context fixes, or the wider
of two readings (assuming more, never less).

* **R1** ("or the key", membership). The `mk_http_thread_libco_params` slot:
  the one key the premise list introduces before, the one the body reads and
  writes (`MK_TLS_GET`, `MK_TLS_SET`), and the decomposition atom's footprint
  ("TLS mutation"). The alias premise names the other key explicitly ("the
  current thread's mk_thread key"), and the declaration's NOT CLAIMED note
  claims nothing of it.
* **R2** (the alias list). The appositive list is read as its general class,
  "an alias the source leaves behind", with the three listed cells as members
  (`#aliases`, "among them"). Reading it as exhaustive would assume less.
* **R3** (relative clauses about the body). "which mk_thread_destroy deletes"
  and "the call writes one scalar …, event->type" describe what the body does;
  each is stated as a conjunct (`#c in #freed`, `#mk_type(#ev) in #stores(…)`),
  so the formula assumes it too. Leaving it out would assume less.
* **R4** ("changes", Valgrind). Writes or releases: both
  `#no_writes(#tracked)` and `#no_releases(#tracked)`. "Writes" alone would
  assume less.
* **R5** ("borrowed", chain). Defined by the relative clause that follows it:
  objects this call neither releases nor ends.
* **R6** ("the borrowed chain", membership). The session, its channel and that
  channel's event: the chain of the borrowed-chain premise and of its
  decomposition atom ("Session, channel and event"). Leaving the session out
  would assume less.
* **R7** ("live where it is read"). `#live` is liveness at every point of the
  call that reads, writes or releases the object, so the session is live
  where it is read, and `th` up to and including the call's release of it.

**`|->` and concurrent runs.** C04.18.9 reads `F |-> E` over the runs of `F`
during the call. `#concurrent`'s meaning makes its runs those that overlap the
call, whoever runs them, so `#concurrent |-> E` is not satisfied merely
because the call itself runs none of them.

## 6. Fail-closed arms

* The 11 SP4 decision vectors: four accepted, seven refused (`|->` conjoined
  to an effect without parentheses, an accessor applied to a set, an effect
  applied to an object, objects on the left of `|->`, membership of an object
  in an object, a bare effect, a singleton compared as a resource set). The
  Rust reader and the Lean judgment decide each the same way
  (`the_decision_vectors_are_the_lean_table`, `decision_vectors_agree`).
* Every other arm is SP1's and unchanged: an unknown term, an ill-sorted
  application or an unbound variable rejects the whole module.
* The provider test refuses a prose premise in any owned module.

## 7. Lean

`QCore.PremiseFormula` (base, shared with SP1):

* the SP4 rows are appended to `vocab` (the Rust test reads the table,
  skipping the block's comment heading);
* `#singleton` is interpreted as the singleton set; every other SP4 term by
  name through `ext0`, `ext1El` and `ext1St`, so the model gains six
  name-indexed fields with defaults (`xEl0`, `xSt0`, `xEl1`, `xSt1`, `xStSt`,
  `xEfSt`), not one per term. Every model written before (SP1's
  counterexample) is still a model;
* `prim0_kind`, `prim1_kind` and `prim2_kind` take one case per vocabulary
  row by `repeat' rcases`, whatever the number of rows, so another card's
  appended rows need no proof edit;
* `decision_vectors_agree` covers the 11 SP4 vectors.

`QCore.PremiseFormula.Libevent` (new; `proof/AuditPremiseFormulaLibevent.lean`):

* `libevent_formulas_wf`: the seven formulas are well-formed. The Rust test
  `the_libevent_formulas_are_the_lean_definitions` binds each definition to
  its clause as read from the module.
* `eval_parent_embedding`, `eval_tls_params`, `eval_callee_live`,
  `eval_borrowed_chain`, `eval_valid_membership`, `eval_no_later_alias`,
  `eval_valgrind_neutral`: what each formula states over the model, as a
  plain proposition (the singleton quantifiers read as bindings).
* `concurrent_runs_leave_the_callee`: the premises compose. Under the callee
  and membership premises, and reading `#no_writes` as antitone in its set,
  every concurrent run leaves `th->callee` unwritten, unresumed and
  unreleased.
* `libevent_formulas_consistent`: the seven hold together in a model with
  the argument present.
* `libevent_formulas_assume_something`: one model with the argument present
  falsifies all seven, so none is vacuous.

The prose states no entailment between two of the premises; the composition
theorem is the one place two of them speak of the same object.
Placeholder-free; standard axioms only (`propext`, `Classical.choice`,
`Quot.sound`).

## 8. Locks, reviewed

Order: `providers/contracts.py sync` (`private/logs/sync-1.log`, exit 1: it
rewrote the inventory lock and `contracts.json`, then refused the stale pin
of `mk_http_thread_destroy`, the expected refusal); `quirin-audit pin
--source-root <Fluent Bit at REV> --crate qcore-provider-libevent --function
mk_http_thread_destroy` at `046accf6` and then `66910c10` (a `--shared
--no-checkout` clone of the local Fluent Bit repository with `HEAD` set to
each revision; `private/pin-1/pin-*.log`, both exit 0); `contracts.py sync`
again (`sync-2.log`, exit 0). `pin --verify-contracts` and `contracts.py
check` exit 0. `pin --check` at both revisions exits 1 exactly as at base:
28 recorded, and the one entry not in force is the pre-existing `mk_vhost_set`
tip entry (`private/pin-1/check-*.log`).

The diff is the predicted one:

* `contracts.lock`: the `mk_http_thread_destroy` entry's `contract_sha256`
  (`1b8b2bfd…` → `a1e545a2…`) and `pinned_at`; its reviewed identity keeps
  the same sites and the same two revisions. No other entry changes; nothing
  is added or removed.
* `contract-inventory.lock` and `contracts.json`: the digest of
  `monkey/intrusive_list.qv` (`93653fc2…` → `fb43e922…`). 5 declarations and
  87 clauses, unchanged.
* `providers/contract-load-path.json`: unchanged.

## 9. Measurement

No pinned route reads a premise to decide a verdict, so no move is
predicted (`PREDICTION.md`'s `measure-prediction` block, for the
integrator's `quirin-audit measure --declared`). The corpus and twelve-home
routes load this crate and have rows resting on `mk_http_thread_destroy`
(the four Safe consumers, `mk_sched_drop_connection` and the destroyer's own
row): their dependency records print the seven canonical formulas in place of
the English statements. The card did not run the routes; the measurement is
the integrator's.
