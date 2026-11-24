# RETURN-FAMILY-UNTRACKED-DEREF-THROUGH-CAST: prediction

Committed before the implementation commit. Base `7678fe5f1` (the tip of
RETURN-FAMILY-DEREF-THROUGH-RESOLVED-CAST, rule 74m, approved to land as r68
on main `3ca85c7a6`). USER DECISION 2026-11-24: "launch the two return-family
follow-up cards".

## What was known when this was written

No implementation was measured.

* **Reachability**, from the finding card's end-to-end probe (its report §6)
  and its unit pin `the_base_readings_are_kept`:
  `long w(void) { const long *p = 0; return *(long *)p; }` is `Safe`, while
  `return *p;` is `Unknown` (`unknown_alias:p`). Family S's `scalar_read`
  makes an untracked root a starred base only when the token right before it
  is a `*`, so `* ( long * ) p` gives `bases: []` and the flow check never
  sees the dereference of the null `p`.
* A `grep` of Fluent Bit 046accf6 `src/`, `plugins/` and `lib/monkey` for a
  `*` followed, through opening parentheses and parenthesised type groups
  (at least one with a `*`), by a name that is not called
  (`scratch grep.py`, its pattern recorded in the report) finds 108 lines.
  One of them is on a `return` line, and it is
  `return( 8 * ((const mbedtls_rsa_context *) ctx)->len );`, a
  multiplication with a member root, which the change does not touch. A
  `return` expression that spans lines is not covered by the `grep`.

## Planned change

The untracked root's starred base test keeps its current reading (a `*` that
may be unary right before the root) and adds one that reads back from the
root through opening parentheses and through every group
`cast_semantics::type_group_open` reads as a type (rule 12h's transparency,
no second detector), to the first `*` that may be unary under either of rule
74h's readings. The root is a base when either reading makes it one. The set
of bases can only grow and family S refuses exactly where it refused, so no
reading admits more than at the base: a new base adds the typed access
route's obligation (`unknown_alias` on a possibly-null or unknown binding, a
`use_after_release` candidate on a released one), never removes one.

## Predicted movements (opt-in route, base vs tip)

A move can only be `Safe` -> `Unknown` (a possibly-null or unknown binding)
or `Safe` -> `Unsafe` (an untracked root cannot be released by this body, so
this one is not expected) at a `return` that family S admits and whose
expression dereferences an untracked root through a cast group or a
parenthesis. An untracked root with no binding (a formal) or a non-null one
discharges the new obligation, so it does not move. The `grep` finds no such
`return`, so no move is expected.

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
