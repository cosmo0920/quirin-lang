# CURRENT-CONTEXT-NEW-TRIAGE, C: prediction

Frozen before any C code. Base for the C measurement: the D3 tip.
Measurement side: `baseline:497bdeb90` (attribute only the moves listed
here; r8-r12, phase 1 and D3 also move rows against that baseline).

User decision, 2026-11-16, relayed by the integrator and quoted exactly:
`context_new` → "C: derived → Safe". The C layers are PROPOSAL.md §3 C:

* **(C1) a keyed-slot retention**, read only for an internal-linkage key
  whose every use is in the unit;
* **(C2) rule 10f widened to `&G->F`** under P-CRR-STATIC-HEAP;
* **(C3) a whole-container destroy reading**;
* **(C4) a singly linked `_next` chain family**;
* **(C5) an interior (borrowed member of a held element) result mode**;
* **(C6) the free-slot invariant**, by body census plus the caller premise
  P-FD-NONNEG;
* **(C7) the rule premise P-TLS-TEARDOWN.**

Every layer is gated on the owner chain C1-C3 establish, so a layer fires
only where that chain is established.

## Reach outside `tls_openssl.c`

* **C1:** every Monkey `pthread_key_t` other than `local_context` has
  external linkage (`MK_TLS_DEFINE`, `mk_utils_error_key`,
  `mk_server_fifo_key`, the `mk_tls_sched_*` keys) and is refused, as are
  every key reached through a pointer and every `__thread` object. Fluent
  Bit uses `__thread` (`FLB_HAVE_C_TLS`), which is D1's reader, not C1's.
* **C2:** the census of "publication … into a container this function does
  not show outlives it (`& X -> F`)" finds exactly one row on all four
  inputs: `mk_tls_thread_init`'s `&server_context->threads` (current only).
  Corpus, system and slice have none (`private/repro/arrow_containers.py`).
* **C3-C6:** gated on C1/C2 owners. No corpus, system or slice row has
  one.

Therefore corpus, system and slice: **no move**. The C measurement covers
only the user's `current --monkey-linked`.

## The user's `current --monkey-linked`, per function (`lib/monkey/tls/tls_openssl.c`)

D2 is the concurrent OPENSSL-PROVIDER-PLUGIN card. Its contracts load only
through a generated user load path. Until a certificate checker exists, a
row whose `Safe` rests on an OpenSSL plugin binding publishes as
**Pending**, which caps every row that calls OpenSSL.

| Function | Today (phase 1) | C without D2 | C with D2 | Why |
|---|---|---|---|---|
| `context_get_head` | Unknown | **Safe** | Safe | C1 (thread context) + C4 traversal; returns a borrowed element (C5) |
| `tls_event_interest` | Unknown | **Safe** | Safe | only `context_get_head`; reads a scalar of the borrowed element |
| `context_get` | Unknown | **Safe** | Safe | `context_get_head` + returns `head->ssl`, a member of a held element (C5) |
| `mk_tls_thread_init` | Unknown | **Safe** | Safe | C2 (`&server_context->threads`, static-heap root) + C1 (the keyed slot retains `thctx`) |
| `context_new` | Unknown | Unknown | **Pending** (capped) | R1-R3 (C1, C4, C5), R7/R8/H1/H2 (C5, C6) are discharged; R4-R6 are `SSL_new`/`SSL_set_fd`/`SSL_set_accept_state` |
| `contexts_free` | Unknown | Unknown | Pending (capped) | C4 whole-chain destroyer; `SSL_free` |
| `context_unset` | Unknown | Unknown | Pending (capped) | C4 member release in a held element; `SSL_shutdown`, `SSL_free` |
| `tls_handle_return` | Unknown | Unknown | Pending (capped) | `context_get_head` (Safe) + `SSL_get_error` |
| `context_get_or_create` | Unknown | Unknown | Pending (capped) | follows `context_new` |
| `mk_tls_read`, `mk_tls_write`, `mk_tls_writev` | Unknown | Unknown | Pending (capped) | `SSL_read`/`SSL_write`, `tls_handle_return`, `context_get_or_create` |
| `mk_tls_close` | Unknown | Unknown | Pending (capped) | follows `context_unset` |
| `mk_tls_send_file` | Unknown | Unknown | Unknown | also `tls_pread` → libc `pread`, uncontracted |
| `mk_tls_exit` | Unknown | Unknown | Unknown | C3 + C7 read the destroy loop, but the `__builtin_offsetof` node (`mk_list_entry`), D3's refused static `pthread_key_delete`, `config_free`, `SSL_CTX_free` and `pthread_mutex_destroy` remain |
| `mk_tls_init` | Unknown | Unknown | Unknown | static publication of `server_context` is read by C2/C7, but `config_parse`, `tls_init_library`, `tls_load_credentials` and the `SSL_CTX_*` calls remain |
| `config_parse`, `tls_pread`, `tls_configure_dh`, `tls_create_server_context`, `tls_init_library`, `tls_load_builtin_credentials`, `tls_load_credentials` | Unknown | Unknown | Unknown | independent blockers (rconf API, OpenSSL configuration calls, `pread`); D2 may move some to Pending, which is D2's attribution, not C's |
| `local_thread_context`, `config_free`, `mk_tls_enabled`, `mk_tls_transport` | Safe | Safe | Safe | |

Totals for C on the user's current:
* **Without D2:** 4 rows `Unknown -> Safe`, nothing else.
* **With D2 under the user load path:** the same 4 `Safe`, plus 9 rows
  `Unknown -> Pending` (`context_new`, `contexts_free`, `context_unset`,
  `tls_handle_return`, `context_get_or_create`, `mk_tls_read`,
  `mk_tls_write`, `mk_tls_writev`, `mk_tls_close`). They become `Safe` once
  D2's bindings are certified.
* `context_new` itself reaches `Safe` only with D2 certified. C removes every
  non-OpenSSL reason on its row.

No `Safe` row moves down anywhere. No leak, double release or `Unsafe` is
added on any route.

```measure-prediction
route corpus
none
route system
none
route slice
none
```
