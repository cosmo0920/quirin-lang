# CURRENT-CONTEXT-NEW-TRIAGE

Card CURRENT-CONTEXT-NEW-TRIAGE (user decision 2026-11-16, "yes, launch it").
Base `0f6e17118` (integrate/r12). Subject: the provisional `Unsafe` of
`lib/monkey/tls/tls_openssl.c` (Fluent Bit `046accf6`, Monkey as linked) in
the integrator's `current --monkey-linked` run at binary `89fa31d21`
(`private/current-linked-556460e31/reports-89fa31d21/`, read only).

Sections 1-4 are the census, committed at `f88e6a16d` before any
implementation; the [prediction](PREDICTION.md) was committed at
`191220d46`, before the implementation commit `1e691eecc` (the measured tip;
later commits change records only). Gateway rule 5l amended in place with
item (6), rule 7's pointer updated; Lean `QCore.RefusedStoreHandOn` (new).

## Headline

* **Verdict (a): a modelling defect.** `context_new`'s two leak claims are
  not validated: the node is linked into the per-thread list that
  `mk_tls_exit` releases (section 1). Nothing is presented or filed upstream.
* **Fixed under the existing rule.** B08's owner refusal reports
  `UnknownAlias`, which W44-LEAK-PIN-AUDIT's refused hand-on rule never read;
  rule 5l (6) gives it rule 5l (5)'s treatment. A withdrawal only:
  `context_new` is `Unknown`, never `Safe`.
* **Accepting the store** (`context_new` `Safe`) needs a semantic choice no
  rule settles; options with a recommendation are in section 7.
* **Measured on unit-scoped reproductions** (evidence tree, base vs tip
  binary): one row moves, `context_new` `Unsafe -> Unknown`; corpus, system
  and slice predicted unchanged (section 6). The integrator reruns
  `current` and measures the live routes.

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

## 5. The repair (rule 5l (6))

`crates/qcore-c-gateway/src/legacy_transfer.rs`, B08's owner-refusal branch:
before the bindings are poisoned, the places the refused store hands on go
through `refused_operands::withdraw_refused_call` (live sites only, cascading
through containment and field links). `refused_store_hand_ons`: an `Alias`
store hands on its value; a contracted call whose result the store would take
hands on what rule 5l (5) lists (`refused_call_hand_ons`, now shared with the
bound-read/member-address refusal, which is unchanged); `Clear` and an
identity-free `AggregateStore` hand on nothing. The node keeps its
`UnknownAlias`, so a function it fully withdraws is `Unknown`.

Fixtures, `crates/qcore-c-gateway/tests/refused_store_hand_on.rs` (7 tests,
all pass at the tip; run against the base transfer source, 3 fail as the
census says):

| Fixture | Base | Tip |
|---|---|---|
| store of a node into a borrowed root's member (`t = bu_get(0); t->contexts = n;`) | Unsafe, 1 leak | Unknown, no leak |
| `context_new`'s shape, both returns (head insert through a borrowed `thctx`) | Unsafe, 2 leaks (the `ssl_new` failure return and the success return) | Unknown, no leak |
| store through an owner this body released | Unsafe, leak | not Safe, no leak, refusal kept |
| a second allocation the refused store does not name | Unsafe, 1 leak | unchanged |
| admitted store into a body owner, owner released (P-B08-SHALLOW) | Unsafe, 1 leak | unchanged |
| call result into a refused member (`t->contexts = r_wrap(p)`, `p` borrowed) | Unsafe, `p`'s leak | unchanged |
| refused store then explicit release of the value | not Safe | not Safe |

The fourth and fifth rows are the brief's "store into an unowned or
unreleased root still leaks" in the form the rule gives it: an admitted
store (a resolved owner) is untouched and keeps its accusation; a refused
store is no longer an accusation of the value it stored. The released-owner
row is the one precision cost: the stored node there is unreachable, but the
refusal is uniform, as rule 5l (5)'s is.

Lean `QCore.RefusedStoreHandOn` (`proof/AuditRefusedStoreHandOn.lean`, 8
theorems, `propext` only): the W44 rule at a wider refusal signal that extends
the call rule; only withdraws and changes no status; an admitted store is
untouched; an unnamed site keeps its leak; the stored value is withdrawn; a
refused store is never `Safe`.

## 6. Measurement

Unit-scoped `current --monkey-linked` on the evidence tree (read only),
reduced copies of its compile database, the rule 40 link artifact, base
binary (`0f6e17118`) against the tip binary; script `private/repro/run.sh`,
`run_pair.sh`, comparison `compare_all.py`:

* `tls_openssl.c` alone: `context_new`'s row equals the evidence row byte for
  byte at base (one unrelated `config_parse` diagnostic differs: the reduced
  database lacks the unit that defines `mk_rconf_section_get`).
* every unit of a function row that is `Unsafe` with an unresolved identity
  in the evidence run (`mk_http.c`, `mk_net.c`, `tls_openssl.c`, `rdhttp.c`,
  `rdkafka_sasl_oauthbearer_oidc.c`, `snappy.c`, `ctr_random.c`, then
  `cmt_decode_statsd.c`; 181 + 93 rows): **one row moves**, `context_new`
  `Unsafe -> Unknown` (leaks at `:533` and `:541` withdrawn); the unit
  `Unsafe -> Unknown`, `leak` 2 -> 0. Nothing else changes, diagnostics
  included.

Why no other row of any route can move, and the per-route census of the
`Unsafe` rows with an unresolved identity at `baseline:497bdeb90`: see the
[prediction](PREDICTION.md). The live routes do not read `tls_openssl.c`.

## 7. Decision offered: accepting the store

Fail-closed settles the withdrawal; it does not settle acceptance. To make
`context_new` `Safe` the analysis would have to know both:

1. that `pthread_getspecific(local_context)` returns the object
   `mk_tls_thread_init` retained with `pthread_setspecific` (today
   `resource_return_none()`; open as `W43-RESULT-ALIAS-CONDITIONAL-AND-RETAINED`
   and `W46-FLUENT-TLS-RETAINED-RESULT`), and
2. that `thctx->contexts` is a checked owner: a list whose nodes a known path
   releases (`mk_tls_exit` → `contexts_free`, after the workers join).

Options:

* **A (recommended): keep `Unknown`; record the observation on the open
  follow-ups.** No vocabulary change; the row is honest and in the
  denominator.
* **B: a declared premise for Monkey's TLS context list.** A
  `resource_premise` (deferred-review, so `Pending`) on a contract of
  `local_thread_context` stating that its result is the calling worker's
  `tls_thread_context`, whose `contexts` list `mk_tls_exit` releases; the
  head insert would then be a publication into that list (rule 10d's
  container reading extended to a singly linked head insert). Moves the row
  to `Pending`, needs a new store-publication reading with Lean parity.
* **C: a retained-result relation for `pthread_getspecific`** (the open
  follow-ups' discharge), plus an owner relation from the TLS slot to a list
  released on a teardown path: general, and the largest.

A is recommended: B and C each add a semantic clause (an owner relation for
a singly linked head insert through retained thread storage) that only the
user can choose, and neither changes any live route.

**Phase 2** (user decision 2026-11-16, "Go further to gather what should
make it safe"): the facts a Safe verdict needs, the options with their rule,
contract and Lean changes, and the reach census are in
[PROPOSAL.md](PROPOSAL.md). It is a proposal only; the code above is
unchanged.

## 8. Follow-ups

* `UNKNOWN-ALIAS-REFUSAL-HAND-ON-CENSUS` (registered): other transfer
  branches that report `UnknownAlias` and return before the instruction's
  effect may share the gap rule 5l (5) and (6) closed for calls and B08
  stores; no census lists them.
* `W43-RESULT-ALIAS-CONDITIONAL-AND-RETAINED` (amended): Monkey's
  `context_new` measured as a consumer of the retained result.

## 9. Gate

`python3 scripts/gate_scope.py --base 0f6e17118 --out private/gate-1` at
`310fd412b` (the code is that of `1e691eecc`; later commits are records);
[gate.json](gate.json), Clang recorded in [gate/](gate/clang-version.txt).
Mode full (the diff reaches `qcore-c-gateway`).

| Step | Exit | Counts |
|---|---|---|
| submodules | 0 | 1 submodule |
| `cargo fmt --all -- --check` | 0 | |
| `cargo clippy --all-targets -- -D warnings` | 0 | |
| Rust suite (`run_test_binaries.py`) | 0 | 639 units, 682 summaries; 9,516 passed, 0 failed, 17 ignored, 148 filtered (5,288 s) |
| `reevaluation_probe`, `reevaluation_followup` | 0, 0 | |
| `check_lean_audits.py` | 0 | 435 audits |
| Python suite | 1 | 761 tests, 1 failure: `test_gate_slot_starvation.Drain.test_a_drain_gives_up_reopens_admission_releases_and_names_the_holder` ("0.18 not greater than 0.2 : the holder spins") |
| that module rerun alone (`python3 -m unittest test_gate_slot_starvation` in `scripts/tests`) | 0 | 26 tests OK |

The one Python failure is the timing-sensitive starvation test the brief
names; rerun alone it passes (both recorded). The host was shared with
another card's audit job (gate_slot status: 9.6 GB RSS over its 7 GB
admission) while the suite ran.

## Integrator measurement (2026-11-16)

Declared measurement, base `rev:0f6e17118` (the card's base; r8–r12 landed after the newest published baseline `497bdeb90` without a publish, so the card base isolates this card), tip `rev:1e691eecc`, run from the main checkout (`private/measure/context-new-20261116T040720Z`): exit 0, all three routes complete, no unmatched rows, declared result **match**.

- corpus: 0 rows moved; 0 rows changed content only.
- system: 0 rows moved; 0 rows changed content only.
- slice: 0 rows moved; 0 rows changed content only.

[Comparison](integration/native-comparison.json.gz) (gzip of
`native-comparison.json`, sha256
`8eb61cba4b522a2b6019c0824c19f028fb8638fef95ee3337c9e2065faf1e0d4`).

The change runs only on a refused-owner branch; no pinned route reads Monkey `tls_openssl.c`, so nothing moves. Its effect is on the user's `current --monkey-linked` run (`context_new` Unsafe to Unknown), which the integrator reruns after landing. Landed through integrate/r15 with CURRENT-CONTEXT-NEW-TRIAGE (phase 1), LINK-STANDING-IMPORT-LIMITERS and OPENSSL-PROVIDER-PLUGIN; all three merged without conflict.

The integrate/r15 landing gate record is [OPENSSL-PROVIDER-PLUGIN's](../openssl-provider-plugin/integration/gate.json) (full gate on the merged tree 96bf2cc25 plus a targeted rerun; see its note).
