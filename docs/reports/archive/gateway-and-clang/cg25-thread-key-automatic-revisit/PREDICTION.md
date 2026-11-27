# CG25-THREAD-KEY-AUTOMATIC-REVISIT: prediction

Frozen before the semantic change. The integrator measures with base
`rev:5c824e443`. No route measurement was run for this file. The census
and the unit differential below are read-only reproductions.

## Mechanism

The change follows the soundness analysis ([README](README.md) §1). It
lives in `legacy_in_place_custody.rs`, the libc `threads.qv` declaration of
`pthread_getspecific`, and rule 11v (7).

* **Tracking.** `pthread_key_create(&k, 0)` with `k` the one automatic
  object of the function is tracked as CG25's in-place initialiser. "The
  one automatic object" means one `VarDecl`, and every `DeclRefExpr` naming
  `k` designates it. A creation into any other storage keeps D3's refusal
  `THREAD_KEY_STORAGE_REFUSAL`, with its text unchanged: a member, a formal,
  a pointer-reached object, or a name that also designates a file-scope
  object. A static creation keeps D3's reading.
* **Kinded getter.** `pthread_getspecific`'s key gains
  `resource_kind(key, thread_key)`: a by-value use. A use whose actual names
  no tracked key keeps the plain borrowed reading. `pthread_setspecific` is
  unchanged.
* **Certified use (rule 11v (7)).** A by-value use of a tracked key is
  admitted only where the settled state certifies the key. A released key is
  the `use_after_close` candidate; any other uncertain state refuses.
* **Address versus value.** `&k` is never custody at a by-value position.
* **Nested uses.** A nested by-value use (`f(pthread_getspecific(G))`) is not
  refused by the in-place CFG check, so an untracked key's nested getter
  keeps the reading it had while the position was unkinded.

## Rows it can touch

1. **Tracking.** Only a body that creates a key into an automatic object
   tracks anything. A source census of every evidence tree found **no such
   body** (§ Census below).
2. **Kinded getter.** Every body that calls `pthread_getspecific` now passes
   its key through `in_place_actuals`. For an untracked key (every key in
   the evidence: static, member, formal), that is the identity:
   * the by-value branch finds no tracked name;
   * `untracked_use` holds;
   * the slot keeps the place it had;
   * the nested-use exemption keeps nested getters as they were.

   The unit differential below confirms this on every evidence unit that
   names a key call.

| Function | Route(s) | Today | Predicted |
|---|---|---|---|
| every row | corpus, system, slice, `current` | unchanged | unchanged, no content change |

## Census: no automatic key in any evidence tree

How: `rg -n 'pthread_key_create\s*\('` over every `.c` and `.h` of three
trees, then each actual's declaration was read
(`rg 'pthread_key_t\s+NAME'`, the `FLB_TLS_DEFINE` / `MK_TLS_DEFINE`
definitions). The trees:

* the pinned corpus inputs `build/cycle19/c-gateway-wave-pin/` (Fluent Bit
  `fluent-bit-clang-source`, Monkey `monkey/monkey-source`);
* the user's `current` evidence tree
  `private/current-linked-556460e31/run/`, both sources;
* their bundled `lib/` trees.

The `FLB_TLS_INIT(…)` and `MK_TLS_INIT(…)` arguments were read the same way.

| Actual form | Sites | Storage | Reading |
|---|---|---|---|
| `&G`, `G` file scope or file `static` (Monkey `local_context`, `cgi_request_list`, `cache_*`, `mk_tls_*`, `mk_utils_error_key`, `mk_server_fifo_key`; Fluent Bit `libco_in_param_key`, `oauth2_*`, every `FLB_TLS_DEFINE` key; avro `error_data_key`; jemalloc `tsd_tsd`; WAMR sample `name_key` and test `key`) | all the rest | static | D3, unchanged |
| `&ctx->remove_mpa_key` (`out_loki`), `&nb->thread_key` (neverbleed) | 2 | member | refused, unchanged |
| `ctx->key` (`mk_fifo.c`), `node->thread_key` (`mk_plugin.c`), `key` (tinycthread formal `tss_t *key`) | 3 | through a pointer | refused, unchanged |
| `&k`, `k` an automatic local | **0** | — | — |

Several static sites carry a non-null destructor (`libco_in_param_key`, the
`oauth2_*` keys, jemalloc, avro, neverbleed). They stay outside the libc
declaration's literal premise (§C04.5), unchanged.

## Unit differential (base `5c824e443` against the change)

Every unit of the two `current` compile databases whose source names a key
call or a TLS macro was analysed twice:

* 40 units, Fluent Bit and Monkey, preprocessed with their recorded compile
  commands into `private/probe/units/`;
* `analyze_unrestricted_c11` with every loadable libc module, GNU11, every
  preprocessed body;
* run once by a base tree (`git archive 5c824e443`) and once by the change.

The prediction is that every function's status and diagnostics are
identical in both runs. The result is recorded in [README](README.md) §2.

## Predicted moves

* corpus, system and slice: none, and no content change.
* the user's `current --monkey-linked`: none.

```measure-prediction
route corpus
none
route system
none
route slice
none
```
