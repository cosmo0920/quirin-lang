# CG25-RESOURCE-KINDS: a library-handle kind and an in-place kind

Card CG25-RESOURCE-KINDS (wave 48, round 10; High; gateway rule
[11v](../../agents/gateway.md#constraint-11v)), 2026-10-01/02, on base
`b6a3dc42` (local main = origin/main). Clang gap entry K07, route G, absorbing
`W44-DLOPEN-DECLARATION-REMEASURE` and `W43-LIBC-IN-PLACE-LIFECYCLE`. The
property published is **custody safety** (no leak, double release, release of
another kind or use after release of a tracked identity) for the two new
kinds, on route G, over the libc declarations of
`providers/quirin-provider-libc` and, for the in-place kind, over automatic
objects only. No other property is added.

## Headline

* **Kinds** (contract spec [§C04.6, §C04.6.1](../../QCORE_CONTRACT_SPEC_r0_3.md)):
  `dynamic_library` (pointer-carried, failure value NULL; `dlopen`/`dlclose`)
  and the **in-place** `regex` (`regcomp`/`regfree`; the obligation lives in
  the caller's `regex_t`, the failure value is a nonzero status). Each kind has
  one spelling and one carrier (`ResourceCarrier::{Pointer, Integer,
  Storage}`); no kind is equated with another.
* **Gateway** (`legacy_in_place_custody.rs`, thin hooks in `legacy_c.rs`): an
  in-place obligation is bound to one automatic object passed as `&NAME`; the
  initialiser's status, held by a plain name, witnesses it, and a null test of
  the witness refines it; tracked storage is custody, never data.
* **Declarations** (`libc/dlfcn.qv`, `libc/regex.qv`): `dlopen`, `dlclose`,
  `regcomp`, `regfree`; `regexec` and `regerror` gain the `regex` kind on
  `__preg`. 122 corpus modules (unchanged), 255 → 259 v1 declarations, six pin
  rows changed or added, and the libc `contracts.lock` refreshed for the six
  declarations (`quirin-audit pin --function …`, all flexible).
* **Lean**: [`QCore.ResourceKinds`](../../../proof/QCore/ResourceKinds.lean),
  28 theorems audited by [`AuditResourceKinds.lean`](../../../proof/AuditResourceKinds.lean),
  standard axioms only; `QCore.KindedCustody.Kind` gains the two constructors
  and its 30 audited theorems hold unchanged. Shared vectors `kindVectors` and
  `protocolVectors`, bound by `crates/qcore-c-gateway/tests/resource_kinds_lean.rs`.
* **Arbiter** (corpus `66910c10`): 629 / 10 / 4,195 → **630 / 10 / 4,194**
  (`3d53a5d0…` → `5ade5aab…`): `rd_regex_match` Unknown → Safe; 5 functions
  change, every leaf attributed; no correction, no Safe → non-Safe move, no
  leak or violation candidate added. The prediction's second move
  (`mk_plugin_load_dynamic`) did not happen (§4). System and tail-pause slice:
  44 / 2 / 188 and 29 / 1 / 239 / 1 Pending, **no function changes** on
  either (no corpus-only callee of the four is called there).

## 1. BEFORE and the cited claims

| Claim | Source | Reproduced |
|---|---|---|
| K07: `ResourceKind` has exactly four variants (Heap/Descriptor/Stream/Directory) | gap analysis §5, Q29 | **yes**: `ResourceKind::ALL` at `b6a3dc42` is the four; CG00's BEFORE control `k07_resource_kinds_are_exactly_four` passed on main (now `k07_resource_kinds_are_exactly_six`, with an undeclared `handle` still rejected) |
| K07: no Windows HANDLE, registry or GPU kind | gap analysis §5 | **yes**, and unchanged: no audited unit uses one, and a kind is added only with a declared use |
| W44-DLOPEN-DECLARATION-REMEASURE: 5 leaves in 4 functions (`mk_plugin_load_dynamic`, `rd_dl_open0`, `mk_plugin_load`, `rd_dl_close`), each callee sole-blocking one function | the entry's `why` | **yes**: `dlopen` 2 / 2 / 1, `dlclose` 3 / 2 / 1 |
| W43-LIBC-IN-PLACE-LIFECYCLE: `mtx_destroy`, `pthread_rwlock_init`, `pthread_rwlock_destroy`, `pthread_key_create` each the sole blocker of one function | the entry's `why` | **yes** (1 / 1 / 1 each sole-blocking one; `pthread_key_create` 4 leaves / 2 functions / 1 sole) |

BEFORE runs, on the base's own release binary (`90895b8b…`):

| Route | Totals | Digest | Note |
|---|---|---|---|
| corpus `66910c10` | 629 / 10 / 4,195 | `3d53a5d0…` | = the CG05 `after3` baseline |
| system | 44 / 2 / 188 | `e334d193…` | path-dependent: equal to the baseline totals; see below |
| tail-pause slice | 29 / 1 / 239 / 1 Pending | `34c67339…` | equal to CG05's `after3` slice (`93cc660c…`) once checkout roots and path-derived digests are normalised (`scripts/alias_diff.py`: 0 differing lines) |

The system and slice routes read the contract profile and the provider modules
of the checkout they run from, and name them by absolute path, so their raw
digests differ between checkouts. The card's first BEFORE of these two routes
(`measure.sh before`) ran while `libc/dlfcn.qv` was being edited in this
worktree and was rejected (`unsupported resource kind dynamic_library`, exit 1);
it is discarded. The published BEFORE (`before3`, `scripts/measure_routes.sh`)
ran the base binary over a `git archive` of `b6a3dc42`'s `contract/`,
`providers/` and `cycle19-resource-contracts.json`.

## 2. Census and the kinds chosen

`scripts/kind_census.py` over the BEFORE corpus ([artifacts/census-before.json](artifacts/census-before.json); AFTER: [artifacts/census-after.json](artifacts/census-after.json)), for 25
families of acquire/release pairs that are none of the four kinds:

| Family | Leaves | Functions | Sole | Decision |
|---|---:|---:|---:|---|
| `getaddrinfo`/`freeaddrinfo` | 10 | 4 | 0 | not here: a status-conditioned out list is CG23's return-code postcondition (it absorbs `W43-DESCRIPTOR-REFUSED-SHAPES`) |
| `dlopen`/`dlclose` | 5 | 4 | 2 | **`dynamic_library`** |
| `pthread_key_create` | 4 | 2 | 1 | not declared: an integer key written through an out pointer, plus a destructor callback |
| `mtx_init`/`mtx_destroy` | 3 | 3 | 1 | not declared: a global object, and `thrd_success` is implementation-defined (1 in tinycthread), so no zero-status sentinel states it |
| `regcomp`/`regfree` | 2 | 1 | 0 | **`regex`, in place** |
| `pthread_rwlock_init`/`destroy` | 2 | 2 | 2 | not declared: storage reached through a formal pointer |
| `pthread_attr_init` | 2 | 2 | 0 | not declared: never destroyed, and `pthread_create` refuses both functions regardless |
| `glob`/`globfree` | 2 | 1 | 0 | not declared: error callback |
| `fts_open`/`fts_close` | 2 | 1 | 0 | not declared: unreadable `fts_open` (W43-DESCRIPTOR-REFUSED-SHAPES) |

Every other family has no leaf in the corpus. The system route and the slice
have none ([artifacts/census-system-before.json](artifacts/census-system-before.json)). The undeclared in-place pairs are
re-registered as `CG25-IN-PLACE-RESIDUAL-PAIRS` with these sizes.

## 3. The design

### 3.1 Vocabulary

`ResourceKind` gains `DynamicLibrary` and `Regex`, serialized as before (kinds
are omitted when heap, so no earlier contract changes form), and
`ResourceKind::carrier` names what carries an identity. The loader's placement
rules for an in-place kind are in `contract_in_place.rs`
(`crates/qcore-contract/tests/resource_kinds_cg25.rs`, 15 rejection rows):
initialiser `resource_out(p, nullable)` on the object pointer, returning its
`i32` status with `resource_return_none()` and moving no other custody;
finaliser `resource_released(p, nonnull)`; use `resource_borrowed(p)` — the one
kinded borrowed position. `dynamic_library` takes every pointer-kind position.
The native descriptor binding maps both kinds to no observed kind, so binding
one is an error, never a silent pass.

### 3.2 Gateway rule 11v

| Rule | Where | Test (`crates/qcore-c-gateway/tests/resource_kinds.rs`) |
|---|---|---|
| A library handle is its own kind: `free(h)`, `dlclose(p)` accused; double close; lookup after close; leak | existing kind machinery, unchanged | `a_library_handle_is_not_a_heap_pointer`, `the_library_lifecycle_violations_are_candidates` |
| NULL sentinel: the guarded handle certifies; unguarded at `nonnull` refuses; a returned guarded handle escapes | existing | `a_guarded_library_handle_closed_once_is_safe`, `the_null_sentinel_refines_and_its_absence_does_not` |
| A possibly-NULL handle returned under a contract-free exit refuses (inferred summaries stay heap, §C04.6 rule 7) | existing | `a_possibly_null_handle_returned_without_a_contract_refuses` |
| An in-place actual is `&NAME` of one automatic object; its place is `NAME` | `Parser::in_place_actuals`, `in_place_address` | `a_tested_compile_freed_once_is_safe`, `only_an_automatic_object_carries_the_obligation` |
| The status witness refines the object; a written, discarded, condition-consumed or address-taken status decides nothing or refuses | `record_witness`, `forget_witnesses`, `witnessed_storage`, `join_witnesses`, the `refine_edge` hook | `a_tested_compile_freed_once_is_safe` (incl. the verbatim `rd_regex_match` body), `an_untested_status_decides_nothing` |
| Double `regfree`, `regexec` after `regfree`, a compiled object never freed | existing kind machinery over the bound storage | `the_regex_lifecycle_violations_are_candidates` |
| Tracked storage is custody, never data (`memset(&re)` at an unkinded position, a struct copy, `free(&re)`) | `Parser::in_place_guard`; the formal-address gate | `the_storage_is_custody_never_data` |
| No route drops an in-place effect | `verify_cfg` | (structural: every initialiser/finaliser node carries its place) |
| A heap-only environment is unchanged | every rule keyed on an in-place name or kind | `a_heap_only_body_is_unchanged`; corpus: only the 5 functions of §4 change |

### 3.3 `dlsym`, revisited

`libc/dlfcn.qv` asked a card declaring `dlclose` to revisit `dlsym`'s
`resource_return_none()`. Kept, with the reading recorded in the module: a
symbol does not point into an object the handle points to (the handle is
opaque), so `resource_return_interior(handle, …)` would misdescribe it, and
the `fileno` precedent keeps such a value untracked. The handle itself is
checked (`dlsym(h, …)` after `dlclose(h)` is a `use_after_release` candidate).
The symbol's lifetime is registered as `CG25-DLSYM-SYMBOL-LIFETIME`.

## 4. Prediction against outcome

[PREDICTION.md](PREDICTION.md) was committed (`41c726cc`) before any AFTER
measurement, but **after** the vocabulary, gateway module and unit tests were
drafted against synthetic fixtures: a deviation from the brief's
freeze-before-implementing, recorded in the file itself.

| Predicted | Measured |
|---|---|
| 629 / 10 / 4,195 → 631 / 10 / 4,193 | **630 / 10 / 4,194** |
| `rd_regex_match` Unknown → Safe | **yes** |
| `mk_plugin_load_dynamic` Unknown → Safe | **no**: it returns `handle` after `if (!handle) mk_warn(…)`, i.e. possibly NULL, and a function's own nullable non-heap result needs a contract (inferred summaries stay heap, §C04.6 rule 7); the prediction read the guard as returning. Now Unknown on `nullable resource handle used by a non-null contract` + `unknown_alias handle` alone, registered as `CG25-KINDED-WRAPPER-RESULTS` |
| no Safe → non-Safe, no candidate | **0 / 0** |
| 7 callee leaves leave; 2–4 added at `dlclose` of untracked handles | 7 leave; **8 added**: `rd_dl_close` 1 (`dlclose((void *)handle)`: the cast actual does not match), `mk_plugin_load` 2 (`unknown_alias handler`), and 2 + 2 at the nullable returns of `mk_plugin_load_dynamic` and `rd_dl_open0`, which the prediction missed with the move above |

Attribution, every changed function ([artifacts/corpus-changed.json](artifacts/corpus-changed.json)):

| Function | Status | Leaves removed | Leaves added | Reading |
|---|---|---|---|---|
| `rdregex.c:rd_regex_match` | Unknown → **Safe** | `regcomp`, `regfree` | — | `r = regcomp(&re, …)` witnesses `re`; `if (r)` returns on failure; `regexec(&re, …)` uses it; `regfree(&re)` discharges it |
| `mk_plugin.c:mk_plugin_load_dynamic` | Unknown | `dlopen` | `unknown_alias handle`, nullable at non-null (line 6058) | the possibly-NULL return, §C04.6 rule 7 |
| `rddl.c:rd_dl_open0` | Unknown | `dlopen` | `unknown_alias handle`, nullable at non-null (line 2970) | the same, behind the cast return; it keeps `rd_dl_error` and `snprintf` refusals |
| `rddl.c:rd_dl_close` | Unknown | `dlclose` | `arguments do not match resource contract for dlclose` | `(void *)handle` of a formal at a release: no place |
| `mk_plugin.c:mk_plugin_load` | Unknown | `dlclose` ×2 | `unknown_alias handler` ×2 | the handle comes from the uncontracted `mk_plugin_load_dynamic` |

`regexec`/`regerror` kinding moved nothing else: their only corpus caller with a
tracked object is `rd_regex_match`; every other call keeps the plain borrowed
reading.

## 5. System route and tail-pause slice

| Route | BEFORE (`before3`) | AFTER (`after1`) | Changed functions |
|---|---|---|---|
| system | 44 / 2 / 188, `e334d193…` | 44 / 2 / 188, `827821ab…` | **0** |
| tail-pause slice | 29 / 1 / 239 / 1 Pending, `34c67339…` | the same, `6982af38…` | **0** |

The raw digests differ only in checkout roots and in digests derived from paths
and contract texts (`libc/dlfcn.qv` and `libc/regex.qv` changed):
`scripts/alias_diff.py` leaves **0** differing lines on each route
([system-changed.json](artifacts/system-changed.json),
[slice-changed.json](artifacts/slice-changed.json): empty). Neither route calls
`dlopen`, `dlclose`, `regcomp` or `regfree` ([artifacts/census-system-before.json](artifacts/census-system-before.json)).

## 6. Lean

`QCore.ResourceKinds` (28 theorems, `AuditResourceKinds.lean`): the kind table
(`kind_vectors_are_the_table`, `every_kind_is_listed`, `spellings_are_distinct`,
`protocol_vectors_count`); the library handle as a pointer kind
(`freeing_a_library_handle_is_a_candidate`,
`closing_a_heap_allocation_as_a_library_is_a_candidate`,
`a_library_null_edge_removes_the_handle`,
`a_symbol_lookup_after_close_is_a_candidate`); the status as sentinel
(`the_status_table_is_sound`, `the_zero_status_edge_certifies`,
`the_nonzero_status_edge_owes_nothing`, `a_written_status_decides_nothing`,
`an_untested_finalise_is_refused`, `an_unheld_status_is_refused`);
distinctness and candidates (`a_finaliser_of_another_kind_is_a_candidate`,
`a_certified_object_is_finalised`, `a_double_finalise_is_a_candidate`,
`a_use_after_finalise_is_a_candidate`,
`a_data_route_over_tracked_storage_is_refused`); `no_candidate_is_safe`; and
seven witness programs including both paths of `rd_regex_match`. The
Rust–Lean correspondence is by review and by the two shared vectors, not proof
(trust boundary 28).

## 7. Follow-ups

* **Retired**: `CG25-RESOURCE-KINDS`, `W44-DLOPEN-DECLARATION-REMEASURE` (the
  kind and both declarations with loader, gateway and Lean parity, the arbiter
  attributing every moved leaf) and `W43-LIBC-IN-PLACE-LIFECYCLE` (an in-place
  mode the gateway checks, with Lean parity).
* **Registered**: `CG25-IN-PLACE-RESIDUAL-PAIRS` (13 leaves / 10 functions /
  5 sole), `CG25-DLSYM-SYMBOL-LIFETIME` (2 `dlsym` callers, neither closes),
  `CG25-KINDED-WRAPPER-RESULTS` (2 functions: a wrapper returning a
  possibly-NULL kinded handle needs a declared, body-checked contract).
* Not registered, size 0 on every route: an in-place initialiser whose status
  is consumed by the condition that tests it (`if (regcomp(&re, …) != 0)`)
  keeps the condition route's refusal.

GATE_PLACEHOLDER
