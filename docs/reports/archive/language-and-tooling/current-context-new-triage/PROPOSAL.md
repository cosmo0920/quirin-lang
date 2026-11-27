# CURRENT-CONTEXT-NEW-TRIAGE, phase 2: what would make `context_new` Safe

User decision, 2026-11-16, quoted exactly: "Go further to gather what should
make it safe." This is a gathering and proposal document. It adds no
semantics. Phase 1's code (`1e691eecc`, rule 5l (6)) is unchanged, and so is
`context_new`'s landed verdict (`Unknown`). The document ends with the choice
the user has to make.

Sources: `lib/monkey/tls/tls_openssl.c` at Fluent Bit `046accf6`, read from
the evidence tree (read only). The census uses `baseline:497bdeb90` (corpus,
system, slice) and the integrator's `current --monkey-linked` report at
`89fa31d21`. Census scripts: `private/repro/tls_census.py`, `callers_of.py`,
`show_rows.py`; output in `private/repro/tls_census.json`.

## 1. Everything the row still carries

At tip `1e691eecc`, `context_new` (`:508-542`) is `Unknown` with these
reasons:

| # | Line | Diagnostic | Statement |
|---|---|---|---|
| R1 | 519 | call to uncontracted function `context_get_head` | `ctx_head = context_get_head(-1);` (a free slot of the thread's list) |
| R2 | 527 | indirect memory access | `ctx_head->_next = thctx->contexts;` (reads the list head through `thctx`) |
| R3 | 528 | `UnknownAlias thctx` (B08 owner refusal; rule 5l (6) now withdraws `ctx_head`) | `thctx->contexts = ctx_head;` (head insert) |
| R4 | 531 | uncontracted `SSL_new` | `ssl = SSL_new(server_context->ctx);` |
| R5 | 536 | uncontracted `SSL_set_fd` | |
| R6 | 537 | uncontracted `SSL_set_accept_state` | |
| R7 | 538 | indirect memory access | `ctx_head->ssl = ssl;` (an owned `SSL` stored into a list node) |
| R8 | 539 | `UnknownAlias ctx_head` | `ctx_head->fd = fd;` (a scalar store through a base joined from R1's result and the fresh node) |

Two obligations are hidden today behind R1-R8. A Safe verdict has to decide
them too:

* **H1, the `SSL` returned at `:541`.** Once `SSL_new` is contracted
  (owned, nullable), the function both stores `ssl` into the node and
  returns it. The node owns it: `context_unset` and `contexts_free` free it,
  and no caller does (`context_get_or_create` → `mk_tls_read`/`write`/
  `writev`/`send_file` only use it). So the return is a **borrow of a member
  of a held list node**, not a transfer. Read as `Owned`, every caller would
  leak it; read as `None`, the node's custody of it must be stated.
* **H2, the overwrite on the reuse path.** When R1 finds a free slot, `:538`
  overwrites that slot's `ssl` member. This is sound only under the
  free-slot invariant: a node with `fd == -1` has `ssl == NULL`.
  `context_unset` nulls `ssl` before it sets `fd = -1` (`:553-558`);
  `mk_mem_alloc_z` zeroes a fresh node; `context_new` stores `ssl` and then
  a non-negative `fd`. The invariant also needs callers never to pass
  `fd == -1`. With `fd == -1`, a node holding an `SSL` looks free and the
  next `context_new` overwrites and leaks it. That premise (P-FD-NONNEG) is
  about Monkey's callers (connection descriptors), not this body.

## 2. The facts a Safe verdict needs, as clauses

None of these is "stored anywhere = released". Each states which owner
receives the custody, and which known path releases that owner.

### F1. The retained-result relation (`pthread_setspecific` / `pthread_getspecific`)

POSIX: `pthread_key_create` gives every thread the value `NULL` for the new
key. `pthread_setspecific(k, v)` sets the calling thread's value. A later
`pthread_getspecific(k)` on the same thread returns the last value set, or
`NULL`. A key created with a `NULL` destructor never releases its values. A
value read after `pthread_key_delete` is undefined.

In this unit:
* `local_context` is a **`static`** `pthread_key_t` (internal linkage,
  `:80`). It is created once (`:770-773`, guarded by
  `local_context_created`, `NULL` destructor) and deleted in `mk_tls_exit`
  (`:853-856`).
* Its only setter is `mk_tls_thread_init` (`:824`, the `thctx` it
  allocated). Its only reader is `local_thread_context` (`:213-216`).
* Every reader of the value tests `NULL` first: `context_get_head` at
  `:481`, `context_new` at `:515`. A thread that never ran
  `mk_tls_thread_init`, or whose `pthread_setspecific` failed (the return
  code is ignored), reads `NULL` and allocates nothing.

Clause needed: for a key of internal linkage whose every
`pthread_key_create`, `pthread_setspecific`, `pthread_getspecific` and
`pthread_key_delete` site is in this unit, `pthread_getspecific(k)` denotes
"`NULL`, or the object one of the unit's `pthread_setspecific(k, ·)` sites
stored on this thread".

* **Derivable from bodies:** the set of setter sites. The key is static,
  so the census of uses is unit-local (a unit's definition census, as
  rule 4c and rule 49 read one, is complete for it).
* **Not derivable:** that the reader runs on a thread where the setter ran
  before it (a thread-identity fact). This can only be a premise, or it is
  covered by the `NULL` guard, which yields "`NULL` or the stored object".
* **Vocabulary today:** `pthread_getspecific` is `resource_return_none()`,
  and `pthread_setspecific` is `resource_retained_by(pointer, global)`
  (rule 12t: `global` with a non-static pointer is `retention-undecided`).
  Neither can state F1 (open as `W43-RESULT-ALIAS-CONDITIONAL-AND-RETAINED`
  and `W46-FLUENT-TLS-RETAINED-RESULT`).
* **Provider:** libc's `threads.qv` (`quirin-provider-libc`, the only home
  for libc contracts).

### F2. The slot object's owner

* `thctx` is linked into `server_context->threads` (`:821`, `mk_list_add`
  under the mutex). `server_context` is a **static pointer** to a heap object
  allocated in `mk_tls_init` (`:760-763`).
* `mk_tls_exit` (`:828-856`) walks that list with `mk_list_foreach_safe` and
  `mk_list_entry` (`container_of`). For each `thctx` it calls
  `contexts_free(thctx->contexts)` and then frees `thctx`. It then frees
  `server_context`.
* `mk_exit` calls `mk_tls_exit` after `mk_sched_workers_join`
  (`mk_server/monkey.c:244-252`), so no worker reads its list afterwards.

What the analysis can and cannot derive:

| Fact | Derivable today? | Nearest rule | Gap |
|---|---|---|---|
| `thctx` published into `&server_context->threads` | No: refused at `:821` ("container this function does not show outlives it") | 10d / 10f (1) P-CRR-STATIC admits `&G` and `&G.F` for static `G`; **`G->F` stays refused** | A static root *through* a heap object the static pointer owns (`&G->F`) |
| `server_context` owned by the static | No: `CTWIN-PUBLISH` refuses the publication at `:761` | CTWIN-PUBLISH (both directions withdrawn) | A static owner with a known teardown |
| `mk_tls_exit` releases every element of `threads` | Partly: rule 5l reads the `mk_list_foreach_safe` family; rule 43/46 element destroyers need a named member and an unlink | 5l, 43, 46 | The loop frees without unlinking (the list dies with `server_context`); a "destroy the container whole" reading |
| `contexts_free` releases every node of a `_next` chain, and each node's `ssl` | No: `cur`/`ctx` are unresolved | 5l covers only Monkey/CFL `mk_list` headers; 48's finiteness premise is for 5l's family | A singly linked `_next` list family (traversal, release of the whole chain, member release) |
| Teardown runs after the workers stop, on the process's exit path | No (temporal and whole-program) | none | Always a premise (P-TLS-TEARDOWN) |

### F3. The head insert as a publication

For `ctx_head->_next = thctx->contexts; thctx->contexts = ctx_head;` to move
custody into the list:
* `thctx` must resolve to an owner by F1 and F2, not to an untracked root;
* the pair must be read as one head-insert into the `_next` family of
  `thctx->contexts`: the old head becomes the new node's successor, nothing
  is lost, and the store is additive. That is constraint 10's lost-slot
  argument, which here needs the first statement;
* the list must be released on a known path by F2 (`contexts_free`).

Fail-closed refusals kept: a head store without the `_next` link (the old
head would be lost), a store into a list whose owner is not established, a
traversal or insert by any other thread, and a store into a list whose
releaser is unknown.

### F4. The node members

* **R7 / H1:** `ctx_head->ssl = ssl` hands the `SSL` to the node (a member
  owned by the list, released by `contexts_free` and `context_unset`). The
  `return ssl` is then a borrow of that member, a retained/interior result
  (C04.14 has `Retained::TrackedInterior` for retention. No *result* mode
  says "interior of a held object", and that is the same gap as F1).
* **R8:** `ctx_head->fd = fd` is a scalar store. It is admitted once the
  base resolves to one non-null node (rule 12y admits a store through a
  non-null borrow of a parent), on both arms of the join (R1's element and
  the fresh, now published, node).
* **H2:** the free-slot invariant needs a declared premise on the list
  (`fd == -1 ⇒ ssl == NULL`, held by `context_unset`, `context_new` and the
  zeroing allocator) together with P-FD-NONNEG. A body census can check the
  first: every store to `fd` or `ssl` in the unit keeps it. The second is
  about callers outside the unit.

### F5. R1: `context_get_head` as a summary

`context_get_head` (`:475-493`) walks `thctx->contexts` by `_next` and
returns a node or `NULL`. Its row is `Unknown` (`cur` unresolved at 486,
488, 490), so callers see it as uncontracted. With F1 and the `_next`
family, its summary is "returns `NULL` or a borrowed element of the calling
thread's context list", which is again a retained/interior result mode.

### F6. R4-R6: OpenSSL

`SSL_new` (owned, nullable; borrows `ctx`), `SSL_set_fd` and
`SSL_set_accept_state` (borrow the `SSL`). The unit's other rows also need
`SSL_free`, `SSL_shutdown`, `SSL_read`, `SSL_write`, `SSL_get_error` and
`SSL_get_fd`. By the standing decision (external libraries through
`quirin-audit` plugin crates with their own checkers), these belong in a new
OpenSSL plugin crate, not in libc or Monkey's provider. Every option except
A needs them for `context_new` to leave `Unknown`.

## 3. Options

Ordered from no change to the most derived. Each option lists what it
changes, what it derives, what it declares, what it keeps refusing, and its
soundness risks.

### A. Keep `Unknown` (status quo after phase 1)

No changes. The row is honest and counted in the denominator. Risk: none.
Size: zero.

### B. A declared, deferred-review owner for Monkey's TLS context (row becomes `Pending`)

* **Contracts.** A Monkey file in `qcore-provider-libevent`
  (`monkey/tls_context.qv`, pinned at `046accf6`) declares
  `local_thread_context` with a new result clause, for example
  `resource_return_held(premise)`, plus a deferred-review
  `resource_premise(tls_context_owner, …)` stating F1 and F2 for this unit,
  and premises for the free-slot invariant and P-FD-NONNEG. Lock sync.
* **Schema and loader.** `qcore-schemas` gains a held-result mode (a major
  version bump), and the loader gains the clause.
* **Gateway (new rule, 51).**
  * The head-insert pair into `R->contexts` with `R` a held result is a
    publication, discharged under the premise. The row is `Pending` and
    names the premise.
  * A member store into an element of that list (R7) hands the value to the
    list.
  * The element's own result returns as a borrow (H1).
  * `context_get_head`'s traversal of the `_next` family yields a
    held-element summary (F5).
* **OpenSSL plugin crate** (F6).
* **Derived:** nothing about ownership.
* **Declared:** F1, F2, the invariant, P-FD-NONNEG, the teardown.
* **Refusals kept:** any other root, any other list shape, an insert
  without the `_next` link, any release of a held element by the caller.
* **Risk:** the premise is trusted until reviewed. That is acceptable for
  `Pending`, which counts as non-Safe in the denominator.
* **Moves:** `context_new`, `context_get_head` and `tls_event_interest`
  become `Pending`, plus whichever I/O rows the OpenSSL contracts clear
  (section 4).
* **Size:** about 12-16 files (schema, loader, two gateway modules, the
  provider file and lock, the plugin crate of about 5 files, tests) and 2
  Lean modules (`HeldResultPublication`, `SinglyLinkedChain`).

### C. Derived relations plus one rule premise (row becomes `Safe`)

* **libc vocabulary (F1).** A keyed retention: `pthread_setspecific(key, v)`
  as `resource_retained_by(v, slot(key))` and `pthread_getspecific(key)` as
  `resource_return_slot(key)` (C04.14 extended). The slot's possible values
  are derived per unit for a key of internal linkage whose every use is in
  the unit. Any key with external linkage refuses.
* **Static owners (F2).**
  * Rule 10f (1) is extended from `&G`/`&G.F` to `&G->F`, where `G` is a
    static pointer whose only writer stores an allocation the unit releases
    in one function (P-CRR-STATIC-HEAP).
  * A "destroy the container whole" reading for `mk_list_foreach_safe` +
    `mk_list_entry` + release without unlink, when the head's owner is
    released right after.
* **Singly linked chain family (F3, F5, the `contexts_free` loop).**
  Traversal, head insert with the `_next` link, a whole-chain destroyer, and
  member release (`SSL_free(cur->ssl)`). This is a new rule beside 5l, with
  a finiteness premise like rule 48's.
* **Interior result mode (H1, F5).** Borrowed results of a held element or
  member.
* **Rule premise P-TLS-TEARDOWN (declared at rule level, as P-CRR-STATIC
  is):** the unit's teardown function runs after every thread that read the
  slot has stopped, and a process exit before it is not a leak.
* **Free-slot invariant:** derived by a body census of every `fd`/`ssl`
  store in the unit. P-FD-NONNEG stays a declared premise on the callers.
* **OpenSSL plugin crate** (F6).
* **Derived:** F1's value set, F2's chain from `server_context` to the
  nodes, F3, F5, and the invariant.
* **Declared:** P-TLS-TEARDOWN, P-CRR-STATIC-HEAP and P-FD-NONNEG.
* **Refusals kept:** external-linkage keys, a key with a non-`NULL`
  destructor, a slot with two setters storing differently owned objects,
  any traversal outside the family, and a teardown function the unit does
  not show.
* **Risks:** P-CRR-STATIC-HEAP generalises a premise already trusted for
  static containers. The keyed-slot reading is new trust and has to stay
  unit-local. A `Safe` that rests on P-TLS-TEARDOWN is a rule-level
  assumption, as with P-CRR-STATIC.
* **Size:** about 25-35 files (libc contracts and lock, schema, loader, 4-5
  gateway modules for three new rules, the OpenSSL plugin crate, tests) and
  4-5 Lean modules (`KeyedSlotRetention`, `StaticHeapRoot`,
  `ContainerWholeDestroy`, `SinglyLinkedChain`, `InteriorResult`).

### D. Better first steps found by the census (independent of A/B/C)

These do not make `context_new` Safe on their own. They are the parts of F1
and F6 that move rows soundly now, and the larger options would reuse them.

* **D1. A read of a thread-local pointer object is a custody-free value**
  (the `__thread` arm of F1, as `pthread_getspecific` already is). Fluent
  Bit builds with `FLB_HAVE_C_TLS` (841 compile commands), so `FLB_TLS_GET(x)`
  is a plain read of a `__thread` pointer. Its getters are `Unknown` only for
  that read.
  * Reading: in a body that writes no static or thread-local object, a load
    of a static or thread-local pointer object into a local is a
    `return_none` value. Every custody use of it refuses, exactly as
    `pthread_getspecific`'s result does today.
  * Refused: a body that also writes the object (a take-from-global would
    otherwise hide a transfer behind `None`).
  * Size: 1 gateway module, 1 rule amendment (CG10/11d's untracked-root
    family), 1 Lean module, fixtures.
* **D2. The OpenSSL plugin crate alone** (F6), with no TLS relation. Its rows
  move only where OpenSSL calls are the last blocker. In `tls_openssl.c` none
  are: every I/O row also calls the `Unknown` local `context_get_or_create`
  or `context_get_head`.
* **D3. In-place `pthread_key_create`/`pthread_key_delete` for a static
  key** (CG25's in-place mode, rule 11v, extended from `&L` automatic objects
  to a static object, with a literal `NULL` destructor). This is open as
  `CG25-IN-PLACE-RESIDUAL-PAIRS` / `W43-LIBC-IN-PLACE-LIFECYCLE`. A neutral
  declaration would erase the obligation, so it must be the in-place mode.

## 4. Reach census

Rows that touch a thread-local slot are those that declare `pthread_*specific`
or `pthread_key_*`, or whose diagnostic names a TLS variable (the 15
`FLB_TLS_DEFINE` and 2 `MK_TLS_DEFINE` names plus Monkey's pthread keys).

| Route | Rows touching a slot | Status |
|---|---|---|
| corpus (`baseline:497bdeb90`) | 52 | 49 Unknown, 2 Unsafe, 1 Safe |
| system | 8 (4 functions × 2 configurations) | all Unknown |
| slice | 4 (2 × 2) | all Unknown |
| current `--monkey-linked` | 63 | 58 Unknown, 3 Unsafe, 2 Safe |

The corpus's Monkey rows are the same functions as in current, minus
`tls_openssl.c`.

What each would move to. A row stays `Unknown` under every option when it
carries an independent blocker the options do not touch: indirect accesses,
`mk_list_del` of an unpublished element, uncontracted `mk_list_init`/
`mk_thread_yield`, `mk_event_add` retention, a nullable non-null use, loop
re-acquisition, and so on.

| Function (route) | Today | D1 | D3 | B | C | Remaining blocker |
|---|---|---|---|---|---|---|
| `flb_engine_evl_get`, `flb_sched_ctx_get` (system, 2 configurations each) | Unknown | **Safe** | | | Safe | none (only the `__thread` read) |
| `flb_context_get`, `flb_cf_context_get` (slice, 2 configurations each) | Unknown | **Safe** | | | Safe | none |
| `flb_output_exit`, `flb_engine_shutdown` (system) | Unknown | Unknown | | | Unknown | 9 and 16 independent refusals |
| `mk_http_thread_initialize_tls_once` (corpus, current) | Unknown | | **Safe** | | Safe | none (two `pthread_key_create`) |
| `mk_plugin_preworker_calls` (corpus, current) | Unknown | | **Safe** | | Safe | none |
| `mk_set_up_tls_keys` (corpus, current) | Unknown | | Unknown | | Unknown | uncontracted `mk_http_thread_initialize_tls` (the `pthread_once` wrapper) |
| `mk_fifo_create`, `mk_tls_init`, `mk_tls_exit` | Unknown | | Unknown | Unknown | Unknown (`mk_tls_init`/`mk_tls_exit` possibly Safe with C's container-whole destroy and static-heap root) | indirect accesses, `__builtin_offsetof` node, static publication |
| `context_new` (current) | Unknown (after phase 1) | | | **Pending** (needs D2) | **Safe** (needs D2) | none once F1-F6 hold |
| `context_get_head` (current) | Unknown | | | Pending | Safe | none (traversal) |
| `tls_event_interest` (current) | Unknown | | | Pending | Safe | only `context_get_head` |
| `context_get` (current) | Unknown | | | Pending | Safe | the member read `head->ssl` returned (interior result) |
| `tls_handle_return` (current) | Unknown | | | Pending with D2 | Safe with D2 | `SSL_get_error`, scalar stores into the node |
| `context_get_or_create`, `mk_tls_read`/`write`/`writev`/`send_file`, `mk_tls_close` (current) | Unknown | | | Pending with D2 | Safe with D2 | follow their callees (`SSL_read`/`SSL_write`; `tls_pread` also needs libc's `pread`, uncontracted today) |
| `context_unset` (current) | Unknown | | | Unknown | Safe only with C's member release in a held element | `SSL_free(ctx_head->ssl)` of a held element's member |
| `mk_tls_thread_init` (current) | Unknown | | | Pending | Safe | `mk_list_add` into `&server_context->threads` (needs C's `&G->F`) plus the keyed retention |
| `mk_sched_set_request_list` (corpus, current) | Unknown | | | Unknown | Unknown | `mk_tls_sched_*` keys have external linkage: outside C's unit-local reading |
| `mk_sched_thread_lists_init`, `mk_cache_worker_init`, `mk_vhost_fdt_worker_init`, `mk_server_listen_init`, `mk_sched_launch_worker_loop`, `mk_server_worker_loop`, `thread_get_libco_params`, `mk_http_thread_destroy` | Unknown | | | Unknown | Unknown | extern keys, plus independent refusals |
| the other 30 Monkey rows (getter callers: `mk_header_prepare`, `mk_http_*`, `mk_net_*`, `mk_utils_*`, `mk_vhost_fdt_*`, …) | Unknown / Unsafe | | | unchanged | unchanged | independent blockers; no verdict rests on the slot |
| `mk_plugin_sched_get_thread_conf`, `local_thread_context` | Safe | | | | | already Safe (return `None`) |

Totals per option. These are predictions, not measurements.

* **D1:** system +4 Safe rows (2 functions × 2 configurations), slice +4
  Safe rows, corpus 0, current 0.
* **D3:** corpus +2 Safe, current +2 Safe.
* **B** (with D2): current only, about 9-10 rows `Unknown -> Pending`
  (`context_new`, `context_get_head`, `tls_event_interest`, `context_get`,
  `context_get_or_create`, `tls_handle_return`, `mk_tls_read`, `mk_tls_write`,
  `mk_tls_writev`, `mk_tls_send_file`, `mk_tls_close`, `mk_tls_thread_init`
  as the premises cover them). Corpus, system and slice: 0, because no live
  route reads `tls_openssl.c`.
* **C** (with D2): the same current rows become `Safe`, plus `context_unset`
  and possibly `mk_tls_init`/`mk_tls_exit` with the container-whole reading.
  It also covers D1 and D3's rows if they are built as its first layers.

No live-route row other than D1's and D3's rests on the slot relation alone.
`context_new`'s movement is confined to the user's `current
--monkey-linked` run.

## 5. Recommendation

1. **Land D1 and D3 first, as two small cards.** They are sound on their
   own, move 8 system/slice rows and 2 corpus rows to `Safe`, need no new
   trust beyond what `pthread_getspecific` and CG25 already state, and are
   prerequisites of C.
2. **For `context_new` itself, choose B if the goal is a reviewed,
   bounded result now; choose C if the goal is a derivation reusable
   beyond this unit.** I recommend **B, then C only if a second unit needs
   it.** The census finds no other live-route function whose verdict rests
   on a unit-local keyed slot or a `_next` chain. Every other Monkey key has
   external linkage, which C's unit-local derivation refuses anyway. So C's
   extra 3 rules and 3 Lean modules would buy rows only inside
   `tls_openssl.c`. B states the facts this unit's bodies support as a
   reviewable premise, and its new mechanism (held-result publication plus
   the `_next` chain) is the part C would also need.
3. **Either way, D2 (the OpenSSL plugin crate) is required**, and it is
   the largest single piece by files.

Sizes, in summary:

| Option | Files (approx.) | Lean modules | Rule changes | Contract/schema |
|---|---|---|---|---|
| A | 0 | 0 | 0 | 0 |
| D1 | 4-5 | 1 | amend CG10/11d | none |
| D3 | 4-6 | 1 (or extend `QCore.InPlace…`) | amend 11v | libc `threads.qv` + lock |
| D2 | 6-8 (new plugin crate) | 0-1 | none | OpenSSL contracts in the plugin |
| B | 12-16 (+ D2) | 2 | new 51 | schema major bump, Monkey `tls_context.qv` + lock |
| C | 25-35 (+ D2, includes D1/D3 layers) | 4-5 | new 51, 52 (+1), amend 10f, 12t | schema bump, libc keyed retention, lock |

Stopping here for the user's choice: A, B (recommended for `context_new`),
or C, and whether to queue D1 and D3.
