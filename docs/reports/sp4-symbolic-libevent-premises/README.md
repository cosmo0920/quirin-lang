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
* **Vocabulary.** 39 terms in an SP4 block appended after SP1's entries, in
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
* **The source operator gate** (superseded at the SP2/SP3 merge, §12: their
  one exemption is kept and this card's dropped) (`qcore_frontend::pipe::check_source_operators`,
  `Q038`) admits `&&` and `|` inside a `requires resource_premise(NAME, …)`
  clause (`premise_formula_spans`), as it already admits a quant check's
  connectives and a result set's operators. Gate 1 found this: the corpus
  test `qcore_contract_corpus::every_qcore_contract_source_parses_round_trips_and_compiles`
  compiles every load-path module through the compiler, and refused the
  formulas' `&&` at `Q038`. SP1 never met it because the OpenSSL user crate
  is not in that corpus. Outside a premise both operators still refuse, `||`
  refuses inside one too, and a malformed formula is still refused by the
  premise reader (`compiler/tests/premise_formula_operators.rs`, 3 tests).
  G05-formula records the admission; the grammar itself is unchanged.
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
| `#live_at_use` | set(obj) | storage allocated, not yet released, at every point of the call that reads, writes or releases the object |
| `#inside(o)` | obj → set(obj) | storage within `o`'s allocation, so releasing `o` ends it |
| `#released_only_by_call` | set(obj) | nothing but the call releases the object |
| `#released` | set(obj) | the call releases it, directly or through a function it runs |
| `#ended` | set(obj) | the call ends its lifetime, by releasing it or an object whose storage holds it |
| `#tracked` | set(obj) | reused from the Fluent Bit block (SP3): the objects the resource projection holds a custody state for |
| `#overlaps(S)` | set(obj) → set(obj) | storage overlapping an object of `S` |
| `#stores(S)` | set(obj) → set(obj) | the member cells of `S`'s objects the call writes |
| `#interleaved` | set(fn) | functions running concurrently with or reentrantly into the call; `\|->` reads each run overlapping the call |
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
  `quant all #m in #args: #mk_parent(#m) in #live_at_use && #m in #thread_data(#mk_parent(#m)) && #m in #inside(#mk_parent(#m)) && #mk_parent(#m) in #released_only_by_call`
* `scheduler_tls_params_owned`:
  `#libco_params_key in #getspecific_ok && #libco_params_key in #setspecific_ok && quant all #p in #holds(#libco_params_key): #p in #live_at_use \ #overlaps(#tracked) && #p in #sole_owned(#libco_params_key) && #p in #freed \ #held_at_return(#libco_params_key)`
* `scheduler_callee_live`:
  `quant all #m in #args: quant all #c in #singleton(#mk_callee(#mk_parent(#m))): #c in #freed && #c in #co_deletable && #c in #created_for(#mk_parent(#m)) && #c in #suspended \ #co_active && #interleaved |-> (#no_resumes(#singleton(#c)) && #no_releases(#singleton(#c)))`
* `scheduler_borrowed_chain_live`:
  `quant all #m in #args: quant all #s in #singleton(#mk_session(#m)): quant all #ch in #singleton(#mk_channel(#s)): quant all #ev in #singleton(#mk_event(#ch)): #s in #live_at_use && #ch in #live_at_use \ (#released | #ended) && #ev in #live_at_use \ (#released | #ended) && #mk_type(#ev) in #stores(#singleton(#s) | #singleton(#ch) | #singleton(#ev)) && quant all #w in #stores(#singleton(#s) | #singleton(#ch) | #singleton(#ev)): #w in #singleton(#mk_type(#ev))`
* `scheduler_valid_membership`:
  `quant all #m in #args: quant all #n in #singleton(#mk_link(#m)): quant all #l in #singleton(#caller_list(#n)): #l in #lists(#n) && (quant all #k in #lists(#n): #k in #singleton(#l)) && #l in #circular_lists && (quant all #k in #nodes(#l): #k in #reciprocal) && #interleaved |-> #no_writes(#nodes(#l) | #singleton(#mk_callee(#mk_parent(#m))) | #singleton(#mk_session(#m)) | #singleton(#mk_channel(#mk_session(#m))) | #singleton(#mk_event(#mk_channel(#mk_session(#m)))) | #singleton(#libco_params_key))`
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
| "live" | `#mk_parent(#m) in #live_at_use` |
| "which nothing but this call releases" | `#mk_parent(#m) in #released_only_by_call` |
| "so that freeing it ends the storage of mth" | `#m in #inside(#mk_parent(#m))`, stated as its own conjunct rather than left as a consequence |
| "(scheduler decomposition exact_parent_embedding and parent_live" | the hypothesis names; no assumption of its own. The conjuncts carry the attached atoms' content: `sched_parent_data_identity` (`#thread_data`), `sched_parent_storage` (`#live_at_use`, `#inside`), `sched_parent_sole_releaser` (`#released_only_by_call`) |
| "; established at creation by mk_http_thread_create, not by this body)" | provenance: says where the assumption comes from, adds none |
| "that the call takes the object mth->parent names and leaves the member designating it is the state parent \|-> #stale" | not a premise: the declaration's clause `quant live #mth: mk_http_thread.parent \|-> #stale` states it, unchanged |

**`scheduler_tls_params_owned`**

| Prose | Formula |
|---|---|
| "the mk_http_thread_libco_params key of the current thread" | `#libco_params_key` |
| "holds NULL or one live allocation" | `quant all #p in #holds(#libco_params_key): #p in #live_at_use …`: NULL is the empty `#holds`, where the body holds trivially; a pointer designates at most one object |
| "it alone owns" | `#p in #sole_owned(#libco_params_key)` |
| "disjoint from every caller-tracked object" | `#p in #live_at_use \ #overlaps(#tracked)` |
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
| "with no concurrent resume" | `#interleaved \|-> (#no_resumes(#singleton(#c)) && …)` |
| "or deletion" | `… && #no_releases(#singleton(#c)))`: deleting a coroutine releases it |
| "(scheduler decomposition callee_live and suspended_coroutines)" | the hypothesis names |

**`scheduler_borrowed_chain_live`**

| Prose | Formula |
|---|---|
| "mth->session" | `quant all #s in #singleton(#mk_session(#m))` |
| "is live where it is read" | `#s in #live_at_use` (`#live_at_use` is liveness at every point the call reads, writes or releases the object) |
| "and its channel" | `quant all #ch in #singleton(#mk_channel(#s))` |
| "and that channel's event" | `quant all #ev in #singleton(#mk_event(#ch))` |
| "are live" | `#ch in #live_at_use …`, `#ev in #live_at_use …` |
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
| "and no concurrent or reentrant writer" | `#interleaved \|-> #no_writes(…)` (`#interleaved` holds the reentrant runs too) |
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
* **R7** ("live where it is read"). `#live_at_use` is liveness at every point of the
  call that reads, writes or releases the object, so the session is live
  where it is read, and `th` up to and including the call's release of it.

**`|->` and concurrent runs.** C04.18.9 reads `F |-> E` over the runs of `F`
during the call. `#interleaved`'s meaning makes its runs those that overlap the
call, whoever runs them, so `#interleaved |-> E` is not satisfied merely
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

## 10. Gate

**Gate 1** (`python3 scripts/gate_scope.py --base 1cd7da6b8 --out
private/gate-1` on `63a6e1b42`, scoped; [record](gate/gate-1.json)) exited 1:

| Step | Exit | Counts |
|---|---|---|
| fmt, clippy | 0, 0 | |
| rust-suite | 101 | 624 units, 8335 passed, **2 failed**, 20 ignored |
| reevaluation probe, follow-up | 0, 0 | |
| lean (`check_lean_audits.py`) | 0 | 469 audits |
| python | 0 | 779 tests |
| provider package build, verify (all, libevent) | 0, 0, 0 | |
| provider test | 0 | 165 passed, 1 ignored |
| provider fmt, clippy | 0, 0 | |

The failures: the recorded host failure
`qcore-audit::test::libcurl_user_crate_versions::the_libcurl_ranges_follow_the_audited_curlver_header`
(system `curlver.h`, not fixed here), and this card's
`qcore-compiler::test::qcore_contract_corpus::every_qcore_contract_source_parses_round_trips_and_compiles`
(`Q038` on the formulas' `&&`), fixed in `06edced3e` (§1, the operator gate).

**Gate 2, the gate of record**, after merging main `70e10930f`
(OPENSSL-OBJECT-FREES-SILENT-METHOD-CALLBACKS and
OPENSSL-INNER-FREES-SILENT-METHOD-CALLBACKS; no conflict, no vocabulary
change on main): `python3 scripts/gate_scope.py --base 70e10930f --out
private/gate-2` on `4643b8b5d`, scoped ([record](gate/gate-2.json)). It
exited 1, from the known host failure only:

| Step | Exit | Counts |
|---|---|---|
| fmt, clippy | 0, 0 | |
| rust-suite | 101 | 625 units, 8339 passed, 1 failed (`libcurl_user_crate_versions`, the recorded host failure), 20 ignored |
| reevaluation probe, follow-up | 0, 0 | |
| lean | 0 | 471 audits |
| python | 0 | 779 tests |
| provider package build, verify (all, libevent) | 0, 0, 0 | |
| provider test | 0 | 169 passed, 1 ignored |
| provider fmt, clippy | 0, 0 | |

After the merge `contracts.py check` exits 0 (`private/logs/check-2.log`).
The Python suite passed whole, including the matrix's Lean-audit count, so
no stale count is carried.

## 11. Follow-ups and merge notes

* **SP5-GLOBAL-PROSE-PREMISE-BAN**, registered here (it was not on main):
  the loader refuses string statements once SP2-SP4 land. If SP2 or SP3
  registers the same id first, drop this card's `register` entry at merge.
* **For SP2 and SP3** (both convert crates whose modules the corpus compile
  test reaches): the operator-gate admission of §1 is what lets a formula's
  `&&` and `|` compile; they need it too, and should reuse it rather than add
  a second exemption. The librdkafka and fluent-bit crates' include tests,
  like libevent's, read any contract line starting with `#` as a
  preprocessor directive, so a formula line must not start with a `#` term
  (this card's layout keeps every line starting with a word or `&&`).
* **Vocabulary merge.** SP2 and SP3 may add generic terms of their own
  (`#live_at_use`, `#tracked`, `#interleaved`, `#singleton` are the likely
  candidates). If one lands first with the same meaning, this card's row is
  dropped in favour of it; with a different meaning, one is renamed. The
  Lean kind lemmas no longer enumerate rows, and every SP4 term but
  `#singleton` is read through the name-indexed fallback, so another card's
  appended rows need only their `vocab` lines if their signatures are among
  the fallback's (nullary obj/set; obj → obj/set(obj)/set(fn); set(obj) →
  set(obj)/eff).

**Files likely to conflict**: `crates/qcore-frontend/src/premise_formula.rs`
(the VOCABULARY and DECISION_VECTORS tails), `proof/QCore/PremiseFormula.lean`
(the `vocab` and `decisionVectors` tails, the Model's last fields, the final
`else` arms of `prim0`/`prim1`, the kind lemmas), `crates/qcore-frontend/tests/premise_formula.rs`,
`crates/qcore-frontend/src/pipe.rs`, C04.18.9 of the contract specification,
G05-formula, rule 13h; generated: `proof/QCore.lean`, `docs/agents/gateway.md`,
`docs/reports/INDEX.md` (rerun their generators).

## 12. Merge with the SP2 and SP3 landing (`8952bdb8d`)

On the coordinator's instruction the branch merged `8952bdb8d`
(integrate/sp3-premises: SP2's librdkafka landing `17b0e77b2` with SP3's
Fluent Bit landing on top). Conflicts and their resolution:

* **`crates/qcore-frontend/src/pipe.rs`**: SP2/SP3's `premise_formula_spans`
  kept as the one exemption (it admits `&&` and `|` only in a clause the
  premise reader accepts as a formula); this card's admission (`06edced3e`)
  dropped.
* **`compiler/tests/premise_formula_operators.rs`** (both added the file):
  theirs kept; this card's three tests folded in as two tests over a
  `libevent_module` shape (`a_libevent_premise_formula_keeps_its_operators`,
  `a_libevent_shaped_contract_still_refuses_elsewhere`). Every expectation
  passes against the stricter exemption unchanged; only the term names were
  updated for the renames below.
* **Vocabulary** (Rust `VOCABULARY`, C04.18.9, Lean `vocab`): blocks ordered
  SP1, SP2, SP3, SP4; no existing row moved. Of the names checked, three
  collided:
  * `#tracked`: SP3's row has the same sort and meaning (the objects the
    resource projection holds a custody state for); this card's row is
    deleted and SP3's is reused. The Lean reading moved from `xSt0 "tracked"`
    to `M.fb.tracked`.
  * `#live`: SP2's means "live at the call's entry"; this card's means live at
    every point of the call that reads, writes or releases the object
    (R7), so it is renamed `#live_at_use`.
  * `#concurrent`: SP2's is another thread's steps only; this card's includes
    reentrant runs and signal handlers (the prose's "concurrent or
    reentrant"), so it is renamed `#interleaved`.
  * `#singleton`, `#released`, `#ended`, `#holds`, `#aliases`, `#nodes`,
    `#lists`: not in SP2 or SP3; kept.
  The SP4 block now has 39 terms. The seven formulas, the decision vectors,
  the Lean definitions and the report were updated for the two renames.
* **`proof/QCore/PremiseFormula.lean`**: 8952bdb8d's kind proofs and
  `maxRecDepth` taken as the base. This card's additions were reapplied on
  top: the six `x*` fields after `fb`, `ext0`/`ext1El`/`ext1St`, `#singleton`'s
  interpretation, the final `else` arms, the vocab rows and decision vectors.
  The kind proofs gained `ext0`, `ext1El` and `ext1St` in their simp sets and
  one `try obtain` for the last row. `openssl_formulas_wf` and
  `libevent_formulas_wf` needed `set_option maxRecDepth 8192 in`, as SP3's
  `decision_vectors_agree` did.
* **Docs**: G05-formula takes SP2's exemption paragraph, plus one sentence
  that SP4 adds none; rule 13h takes SP2's and SP3's amendments followed by
  this card's (39 terms, `#tracked` reused). `gateway.md`, `proof/QCore.lean`
  and `docs/reports/INDEX.md` were regenerated. The C04.18.9 SP4 table was
  moved after the Fluent Bit block.
* **Amendments**: SP3 already registers SP5-GLOBAL-PROSE-PREMISE-BAN, so this
  card's `register` entry is dropped; base is `8952bdb8d`.
* **Locks**: `contracts.py sync` reran after the formula renames. Only this
  card's libevent digests differ from `8952bdb8d`, and `mk_http_thread_destroy`
  was re-pinned at both revisions.

## Integrator measurement (2026-11-21)

Landing `1deb8fd9b` on base `8952bdb8d`, measured with `--declared PREDICTION.md`:
corpus, twelve-home, system and slice all match the declared prediction with
0 moves. Comparison: `integration/native-comparison.json.gz` (uncompressed
sha256 `1a4f73ea36a2b236795466c8c7946edfe77d424297607ed48a05e6f32cc60eaa`). Landing gate: `integration/gate.json` (Rust 9821
passed, 1 failed: the known host libcurl failure; Lean 473 audits; Python
779).
