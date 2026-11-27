# RETURN-FAMILY-STAR-OUTSIDE-ROOT-PARENTHESES: prediction

Committed before the implementation commit. Base `7678fe5f1` (the tip of
RETURN-FAMILY-DEREF-THROUGH-RESOLVED-CAST, rule 74m). USER DECISION
2026-11-24: "launch the two return-family follow-up cards".

## What was known when this was written

No implementation was measured. A scratch end-to-end probe at the base (not
committed; log in the worktree's `private/rso/probe-rows-base.log`) showed:

* **Reachability.** With a may-bind formal `long **q`, `return *(*q + 1);`,
  `*((long *)*q + 1)`, `*(1 + *q)`, `*(n + *q)` (`long n`), `*(*q - 1)`,
  `*(long *)(*q + 1)`, `(*q + 1)[0]` and (with a struct pointee)
  `(*q + 1)->f` are all `Safe`. `return **q;` and `return (*q)[0];` keep the
  return refusal (`Unknown`, "unmodeled call, aggregate or indirect access in
  return expression"). After `k_free(q)`, `return *(*q + 1);` is `Unsafe`
  (`UseAfterRelease`, `unknown_alias:q`): the flow check accuses the first
  dereference, which C evaluates.
* Family S's `stars_before` walks back from the root through `(` and `*`. It
  stops at a binary operator (`*(1 + *q)`) or at a `*` behind more opening
  parentheses than close right after the root (`*(*q + 1)`), so the count is
  one and the root is admitted. `ends_one_dereference` looks only at the
  tokens right after the root, so `(*q + 1)[0]` is admitted too.
* A `grep` of Fluent Bit 046accf6 `src/`, `plugins/` and `lib/monkey` finds
  no `return` whose expression has a `*` before a group containing a starred
  name with arithmetic, and no group containing a starred name followed by
  `[` or `->` other than the root's own parentheses (`(*res)->iana`, already
  refused at the base and pointer-valued).

## Planned change

A starred may-bind root keeps rule 74m's counts, and is additionally refused
when any parenthesised group enclosing it is dereferenced: a `*` that may be
unary (under either of rule 74h's readings) right before the group's `(`,
read back through any parenthesised group in between (a cast, an undecided
group, or a call group the expression refuses anyway), or a `[` or `->` right
after its `)`. The reader cannot tell whether such a group's value is the
pointer computed from the root, so every such group refuses. The admission is
the base admission and a further condition, so it can only shrink. `*q`,
`(*q)`, `((*q))`, `*q + 1`, `2 * (*q + 1)`, `(T) *q` and `(long) (*q)` keep
their admission, and `*(T *)*q` keeps rule 74m's refusal.

## Predicted movements (opt-in route, base vs tip)

A newly refused return falls through to the typed return route (rule 10k) and
from there to the return refusal. The corpus has no such spelling, so no move
is expected. Any move would be `Safe` -> `Unknown` at a `return` that
dereferences, subscripts or selects through a parenthesised group containing
a starred may-bind name.

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
result as on the opt-in route.
