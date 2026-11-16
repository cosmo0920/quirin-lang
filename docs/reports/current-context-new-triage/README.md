# CURRENT-CONTEXT-NEW-TRIAGE

Card CURRENT-CONTEXT-NEW-TRIAGE (user decision 2026-11-16, "yes, launch it").
Base `0f6e17118` (integrate/r12). Subject: the provisional `Unsafe` of
`lib/monkey/tls/tls_openssl.c` (Fluent Bit `046accf6`, Monkey as linked) in
the integrator's `current --monkey-linked` run at binary `89fa31d21`
(`private/current-linked-556460e31/reports-89fa31d21/`, read only).

This section is the census, committed before any implementation.

## 1. Validation verdict: (a) a modelling defect

`context_new` (the only `Unsafe` row of the unit) is accused of two leaks of
`ctx_head`, acquired at `:521` and reported at the returns `:533` and `:541`.
Source, `lib/monkey/tls/tls_openssl.c` at `046accf6` (the copy under
`plugins/tls/` differs and is not the analysed unit):

```c
static struct tls_thread_context *local_thread_context(void)
{   return pthread_getspecific(local_context); }                  /* :213-216 */

static SSL *context_new(int fd)                                    /* :508 */
{   ...
    thctx = local_thread_context();                                /* :514 */
    if (thctx == NULL) { return NULL; }
    ctx_head = context_get_head(-1);                               /* :519, a free slot */
    if (ctx_head == NULL) {
        ctx_head = mk_mem_alloc_z(sizeof(struct tls_context_head));/* :521 */
        if (ctx_head == NULL) { return NULL; }
        ctx_head->fd = -1; ctx_head->want_event = MK_EVENT_READ;
        ctx_head->_next = thctx->contexts;                         /* :527 */
        thctx->contexts = ctx_head;                                /* :528 */
    }
    ssl = SSL_new(server_context->ctx);
    if (ssl == NULL) { return NULL; }                              /* :533 */
    ...
    return ssl;                                                    /* :541 */
}
```

Every path from `:521` to `:533` or `:541` passes `:528`, which links the
node at the head of the calling thread's list. What owns that list:

* `thctx` is the value of the thread-specific key `local_context`
  (`pthread_key_create(&local_context, NULL)`, `:771`, no destructor). It is
  set once per worker in `mk_tls_thread_init` (`:803-825`): allocated, added
  to the global `server_context->threads` under its mutex (`:821`), then
  `pthread_setspecific` (`:824`). `mk_tls_thread_init` is called by every
  scheduler worker (`mk_server/mk_scheduler.c:394`).
* Nodes are never unlinked while the server runs. `context_unset` (`:544`)
  frees the node's `SSL` and marks it free (`fd = -1`); `context_new` reuses
  a free node (`context_get_head(-1)`) before allocating.
* `mk_tls_exit` (`:828-856`) walks `server_context->threads` and calls
  `contexts_free(thctx->contexts)` (`:561-575`, frees every node and its
  `SSL`) and then frees `thctx`. `mk_exit` calls it after
  `mk_sched_workers_join` (`mk_server/monkey.c:244-252`).
* On the `:533` path the node stays linked with `fd == -1` and `ssl == NULL`
  (zeroed by `mk_mem_alloc_z`): a free slot, reused later and released by
  `mk_tls_exit`. On `:541` it holds the new `SSL`.

So the node's custody moved into a list rooted in a per-thread object that a
global list owns and a known teardown path releases. Neither return leaks it:
**(a), a modelling defect.** The leak claim is not a validated reachable
violation (hard invariant 2). Nothing here is an upstream claim, and nothing
is filed.

## 2. The reading that keeps the accusation

Reproduced on the real unit (preprocessed with its recorded compile command,
`private/repro/tls_openssl.i`; gateway probe with the libc contracts and a
`malloc`-equivalent `mk_mem_alloc_z`) and unit-scoped through `current
--monkey-linked` on the evidence tree (`private/repro/run.sh`, a reduced copy
of the compile database: `context_new`'s row equals the evidence row byte for
byte). The projected CFG of `context_new`:

* `:514` `ContractCall local_thread_context` (derived summary: result
  `None`, from `pthread_getspecific`'s `resource_return_none()`), so `thctx`
  carries no identity;
* `:527` `Unknown` (the read `thctx->contexts` is an indirect access), its
  writes sparing `ctx_head`; the statement hands on nothing tracked;
* `:528` `Alias { destination: "thctx->contexts", source: "ctx_head" }`: O00
  blocker **B08**'s typed field store. `thctx` passes B08's lexical base
  premises (a local `struct T *` bound once by a call), so the statement is
  lowered as an admitted store, and the flow-sensitive owner check
  (`resolved_aggregate_owner`) then fails because `thctx` names no tracked
  site. The transfer reports `UnknownAlias thctx`, poisons the bindings and
  returns **without the store's effect and without touching `ctx_head`'s
  site**.

That refusal reports `UnknownAlias`, not `Unsupported`, so W44-LEAK-PIN-AUDIT's
refused hand-on rule (`refused_operands::apply`, which fires on `Unsupported`
only) does not mark the stored value, and rule 5l (5)'s extension
(`withdraw_refused_call`) covers a contracted call refused before its effects,
not a field store. The site stays `LIVE` with no unresolved effect, and
`report_leaks` accuses it at both returns. The same statement lowered by the
lexical gate (any owner B08's base premises refuse, e.g. a formal) is an
`Unsupported` indirect access whose value operand the hand-on rule withdraws:
the B08 admission made the refusal *weaker* than the gate's.

Minimal fixture (gateway harness, a borrowed-result root `t = bu_get(0)`):
`n = q_maybe_alloc(); if (!n) return; t->contexts = n;` is `Unsafe` (`Leak`,
`UnknownAlias t`) at base.

## 3. Repair class: fail-closed, no semantic choice

The repair is the existing rule's: a refused statement hands on its value,
and an identity handed to a statement of unknown effect carries an unresolved
effect (W44-LEAK-PIN-AUDIT; rule 5l (5) for refusals that report
`UnknownAlias`). It only withdraws an accusation; the node keeps its
`UnknownAlias`, so nothing becomes `Safe`. It does **not** accept the store
as a custody transfer.

Accepting it (`context_new` `Safe`) would need two clauses no rule or
contract states, so it is out of this card's repair and is offered as a
decision (hand-back): `pthread_getspecific(local_context)` returning the
object `mk_tls_thread_init` retained (the open
`W43-RESULT-ALIAS-CONDITIONAL-AND-RETAINED` / `W46-FLUENT-TLS-RETAINED-RESULT`),
and that object's `contexts` list being an owner released on a known path
(`mk_tls_exit` via `contexts_free`), i.e. a checked owner rather than
"stored anywhere = released".

## 4. Census: the unit's 27 rows

Base status from the evidence report; "shape" is whether the row contains a
B08 field store of a tracked identity refused on its owner (the defect
above). Only `context_new` has it.

| Function | Base | Shape | Predicted tip | Why |
|---|---|---|---|---|
| `context_new` | Unsafe (2 leak) | yes, `:528` | Unknown | the two leak leaves withdrawn; `UnknownAlias thctx` and the uncontracted calls remain |
| `config_free`, `local_thread_context`, `mk_tls_enabled`, `mk_tls_transport` | Safe | no | Safe | no refused store |
| `config_parse` | Unknown | no | Unknown | stores through the formal `conf` are lexical refusals (already `Unsupported`, B08 excludes formals); uncontracted `mk_rconf_*`, `mk_string_build` |
| `context_get`, `context_get_or_create`, `tls_event_interest` | Unknown | no | Unknown | uncontracted local callees (`context_get_head`, `context_new`) |
| `context_get_head`, `contexts_free`, `context_unset`, `tls_handle_return` | Unknown | no | Unknown | reads through untracked list nodes; uncontracted `SSL_*` |
| `mk_tls_close` | Unknown | no | Unknown | uncontracted `context_unset` |
| `mk_tls_exit` | Unknown | no | Unknown | `server_context` reads, `__builtin_offsetof`, uncontracted `contexts_free`, `pthread_key_delete` |
| `mk_tls_init` | Unknown | no | Unknown | static-storage publication (`CTWIN-PUBLISH`), uncontracted calls |
| `mk_tls_thread_init` | Unknown | no | Unknown | `mk_list_add` into a container not shown to outlive the call (rule 10d); `pthread_setspecific` `retention-undecided` (rule 12t) |
| `mk_tls_read`, `mk_tls_write`, `mk_tls_writev`, `mk_tls_send_file` | Unknown | no | Unknown | uncontracted `context_get_or_create`, `tls_handle_return`, `SSL_write`, `tls_pread` |
| `tls_configure_dh`, `tls_create_server_context`, `tls_init_library`, `tls_load_builtin_credentials`, `tls_load_credentials`, `tls_pread` | Unknown | no | Unknown | uncontracted OpenSSL/libc calls; unmodelled conditions |

Unit: `Unsafe` → `Unknown` (its only `Unsafe` row moves). Wider census and
the measured confirmation follow in later sections.
