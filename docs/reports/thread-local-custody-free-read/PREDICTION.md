# THREAD-LOCAL-CUSTODY-FREE-READ: prediction

Committed after the census (`d31a2bb5b`) and before the semantic commit. The
rows are those of `baseline:497bdeb90`; the measurement compares
`rev:0f6e17118` with the card's tip, so this assumes r8-r12 left the rows
named here as they were at the baseline (none of those cards touches a
thread-local read or these four getters).

## What the card changes

Gateway rule 54: where every reference to a thread-local pointer object in a
function is a whole read copied into an automatic local, initializing one,
returned or null-tested, and every reference to such a local is a rebinding
or a whole read in the same positions, route G's copy from the unbound object
(and between such locals) drops the destination's binding, the transfer of a
`resource_return_none()` call such as `pthread_getspecific`. Anything else
withholds the reading for the whole function, and every alias keeps its
unresolved-identity answer.

## Per route and function

* **system** (`fluent-bit-system`): `flb_engine_evl_get` and
  `flb_sched_ctx_get`, two configurations each, `Unknown -> Safe` (their two
  diagnostics are the copy's and the return's unresolved identity, both
  removed). Four rows, Safe +4. `flb_engine_evl_set`, `flb_sched_ctx_set`,
  `flb_engine_shutdown`, `flb_output_exit` are withheld (writes, calls,
  dereferences): no move, diagnostics unchanged. Callers of the two getters
  in the selection call them as defined functions; their rows may change
  content where a callee summary of the getter is read (the getter's result
  now derives as custody-free, as `pthread_getspecific`'s), but a caller that
  releases or transfers the result still refuses, so no caller is predicted
  to move.
* **slice** (`fluent-bit-tail-pause`): `flb_context_get` and
  `flb_cf_context_get`, two configurations each, `Unknown -> Safe`. Four rows,
  Safe +4. The two setters stay as they are.
* **corpus** (`eight-home`): no move. Its only thread-local readers are
  librdkafka's (non-pointer objects, written objects, a dereferenced copy, a
  call actual): all withheld.
* **current** (`current --monkey-linked`, the integrator reruns it): no move.
  Its thread-local readers are the same librdkafka rows; Monkey builds
  without `MK_HAVE_C_TLS` (its TLS getters call `pthread_getspecific`), and
  the run carries no Fluent Bit core unit reading a `__thread` object. No
  callee leaves the uncontracted list, no unit status changes.

No Safe -> non-Safe, no Unsafe added or removed, no Pending change; the null
property is unchanged (the getters dereference nothing).

```measure-prediction
route corpus
none
route system
move src/flb_engine.c::flb_engine_evl_get Unknown -> Safe
move src/flb_scheduler.c::flb_sched_ctx_get Unknown -> Safe
delta Safe +4
route slice
move src/flb_lib.c::flb_context_get Unknown -> Safe
move src/flb_lib.c::flb_cf_context_get Unknown -> Safe
delta Safe +4
```
