# CG25-RESOURCE-KINDS: frozen prediction

Frozen before any AFTER measurement, on base `b6a3dc42`. **Deviation from the
brief, recorded:** the prediction was written after the vocabulary, the gateway
module and the unit tests were drafted (they were exercised only on synthetic
`k_*` fixtures), not before implementation began. No corpus, system or slice
run of the changed tree had been taken when it was committed; the BEFORE runs
on `b6a3dc42`'s own binary were still in flight.

## Census used (pinned corpus diagnostics of the integrator's CG18 tree, a
## base with the same corpus totals; reproduced on this card's BEFORE)

`scripts/kind_census.py` over the acquire/release pairs that are none of the
four kinds:

| Family | Leaves | Functions | Sole-blocked |
|---|---:|---:|---:|
| addrinfo (`getaddrinfo`/`freeaddrinfo`) | 10 | 4 | 0 |
| library handle (`dlopen`/`dlclose`) | 5 | 4 | 2 |
| pthread key (`pthread_key_create`) | 4 | 2 | 1 |
| C11 threads (`mtx_init`/`mtx_destroy`) | 3 | 3 | 1 |
| regex, in place (`regcomp`/`regfree`) | 2 | 1 | 0 |
| pthread rwlock, in place | 2 | 2 | 2 |
| pthread attr, in place (`pthread_attr_init`) | 2 | 2 | 0 |
| glob, in place | 2 | 1 | 0 |
| fts | 2 | 1 | 0 |

## Kinds chosen

* `dynamic_library` (pointer-carried, NULL sentinel): `dlopen`, `dlclose`.
  Demand 5 leaves / 4 functions; `mk_plugin_load_dynamic` is sole-blocked by
  `dlopen`.
* `regex` (in place, nonzero-status sentinel): `regcomp`, `regfree`; `regexec`
  and `regerror` gain the kind on `__preg`. Demand 2 leaves / 1 function
  (`rd_regex_match`), whose other statements are already admitted.

Not chosen, with grounds: `getaddrinfo`/`freeaddrinfo` (a status-conditioned
out list: CG23's return-code postconditions); `pthread_rwlock_*` (initialised
through a formal pointer, not an automatic object); `mtx_init`/`mtx_destroy`
(global storage, and `thrd_success` is implementation-defined, so a zero-status
sentinel would misstate tinycthread); `pthread_key_create` (an integer key
written through an out pointer, plus a destructor callback); `pthread_attr_*`
(both corpus users are blocked by `pthread_create` regardless, and never call
`pthread_attr_destroy`); `glob`, `fts_open` (callback; unreadable prototype).

## Predicted corpus movement (arbiter `66910c10`)

* Totals 629 / 10 / 4,195 → **631 / 10 / 4,193**: two Unknown → Safe moves,
  `mk_server/mk_plugin.c:mk_plugin_load_dynamic` (its only leaf is `dlopen`;
  the handle is tested and returned) and `rdregex.c:rd_regex_match` (its only
  leaves are `regcomp`/`regfree`; the status is held by `r` and tested).
* No Safe → non-Safe move. No new leak, `invalid_release`, `use_after_release`
  or `double_release` candidate.
* The 7 `call to uncontracted function` leaves of the four callees leave.
  Added: an `unknown_alias` (or a refusal) at each `dlclose` of an untracked
  handle: `rd_dl_close` (a formal behind a cast) and `mk_plugin_load` (two
  `dlclose(handler)` of a handle from an uncontracted helper), 2–4 leaves.
  `rd_dl_open0`, `rd_dl_close` and `mk_plugin_load` stay Unknown.
* Every other moved function, if any, is a misprediction to be attributed.

## Predicted system and tail-pause slice movement

No status movement unless the system BEFORE census shows a function whose only
leaves are these four callees; any such function is named in the report with
its reading. Leaf movement limited to the four callees' own leaves and the
`dlclose`-of-untracked refusals.
