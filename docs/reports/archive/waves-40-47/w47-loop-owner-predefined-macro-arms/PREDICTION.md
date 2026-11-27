# W47-LOOP-OWNER-PREDEFINED-MACRO-ARMS: frozen prediction

Frozen after BEFORE was measured and before any AFTER census, facts
derivation, arbiter or Lean change. Base: local `main` at `f269c231`.
Disclosure: a first draft of the Rust module and the two example hooks
existed, uncompiled and never run, when this was written; no result of it
informs anything below.

## BEFORE (measured)

- Census (base binaries `loop_owner_census` `3ebc9090…`,
  `typed_dispatch_facts` `6653cf8b…`, facts regenerated): all four readings
  byte-identical to the W47-SERVICE-PROPERTY-KEYS archive (`983c74b9…`,
  `4955a12d…`, `d05ebc0e…`, `a5edf1ed…`). `library.json` and
  `fixed-test.json` are byte-identical; `previous-test.json` differs only in
  the generated copy's path.
- Arbiters: running at the base (recorded in the report).

## Locator (never a result)

The base census binary with `--owned-prefix=_` added to both configured
readings (every `_`-prefixed macro no command defines and no read file or
generated header `#define`s read as undefined: a superset of the undefined
decisions this card can make, and none of its defined ones). Fixed reading:
the 3 sites move unresolved → discharged on `(flb_lib_worker, NonThreaded)`
and `(input_thread, Threaded)`; nothing else moves; edges 33,573 → 33,405,
seeds 2,854 → 2,806; unstructured files 4 → 2 (`bufferevent_sock.c`,
lib/monkey's `acutest.h` become structured). Previous reading: no site moves;
edges 33,546 → 33,379.

## Predictions

| Measure | Prediction | Confidence |
|---|---|---|
| derived facts | every one of the 1,421 recorded commands derives with `/usr/lib/llvm-21/bin/clang{,++}` whose `--version` equals the build record's; `_WIN32` probes undefined and `__linux__` defined for every C command; no preprocessed file `#define`s `_WIN32` | 80% |
| fixed test's 3 sites (`:2483`, `:2492`, `:2494`), configured | **discharged** on `(flb_lib_worker, NonThreaded)` and `(input_thread, Threaded)`, resting on the named premises; `mk_utils.c:41` dropped by `#if defined (__linux__)` decided true, `flb_cf_yaml.c:550` by `#ifndef _WIN32` decided true | 75% |
| old test (`90978ac6~1`), configured | **not discharged** at every site it had; its readings may change in edges only | 95% not discharged; 60% byte-identical site verdicts and first reasons |
| other sites, both configured readings | no verdict or first-reason change (the Windows-only plugins' files have no command and are read by no unit, so they stay undecided) | 70% |
| raw readings (fixed, previous) | byte-identical (the raw reading has no configuration) | 95% |
| edges / seeds | fall in both configured readings (arms dropped), by an amount near the locator's | 70% |
| unstructured files | the configured count falls from 4 (at least `bufferevent_sock.c`) | 50% |
| corpus / system / slice | unchanged: the census is not on any audit route | 95% |
| condvar/lifecycle census | untouched (raw reading only) | 95% |
| follow-up `W47-LOOP-OWNER-PREDEFINED-MACRO-ARMS` | retired if the 3 sites discharge under derived facts | 70% |
