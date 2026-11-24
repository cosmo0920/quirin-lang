# RETURN-FAMILY-DEREF-THROUGH-RESOLVED-CAST: prediction

Committed before the implementation commit. Base `3ca85c7a6` (`integrate/r65`).
USER DECISION 2026-11-24: "launch them all".

## What was known when this was written

No implementation was measured. A scratch end-to-end probe at the base (not
committed; log in the worktree's `private/rfd/probe-base.log`) showed:

* **Reachability.** `long w(void **q) { return *(long *)*q; }` is `Safe` at the
  base, and `long w(long **q) { return *(long *)*q; }` is too.
  `return **q;`, the same two dereferences with no cast, keeps the return
  refusal (`Unknown`, "unmodeled call, aggregate or indirect access in return
  expression"). So does the statement `long v = *(long *)*q;` ("indirect memory
  access is outside the resource projection"). Family S's `stars_before`, with
  a finite `closing`, stops at the resolved cast's `)` and counts one `*`, so it
  admits `q` as the root of one dereference. The outer `*`, which dereferences
  the loaded pointer `*q`, carries no obligation. That is the admitting
  direction the follow-up names.
* After `k_free(q)`, `return *(long *)*q;` is `Unsafe` (`UseAfterRelease`), as
  `return *q;` is. The accusation stands on `*q`, the first dereference, which
  C evaluates, so it is not a false accusation.
* A `grep` of Fluent Bit 046accf6 `src/`, `plugins/` and `lib/monkey` for a
  `*` before a pointer cast group followed by `*` (optionally through `(`) finds
  no line at all, `return` or otherwise.

## Planned change

A starred may-bind root keeps its current count, which stops at the first
cast group (`inner`), and also gets a count that a cast group does not stop
(`through`), the rule 12h transparency the member and subscript roots already
have. The root is admitted only when both counts are exactly one under both
of rule 74h's readings. `through >= inner`, so the admission set can only
shrink: `*(T *)*q`, `*(T *)(U *)*q` and `*(T *)(*q)` refuse, while
`(T) *q`, `(int)(long) *q` and `*q` keep their admission. `*(T *)q`
keeps its refusal, because `inner` is 0.

## Predicted movements (opt-in route, base vs tip)

A newly refused return falls through to the typed return route (rule 10k),
and from there to the return refusal. The corpus has no such spelling, so no
move is expected. Any move would be `Safe` -> `Unknown` at a `return` whose
expression dereferences a cast of a dereference.

```measure-prediction
route corpus
none
route system
none
route slice
none
```

**Pinned routes** (`quirin-audit measure --declared`) need `build/cycle19`
inputs, which this host does not have. The integrator should expect the same
result as on the opt-in route. Any move would be one this file does not
predict, and it would be reported.
