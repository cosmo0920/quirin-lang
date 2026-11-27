# RETURN-FAMILY-STAR-AFTER-UNRESOLVED-CAST: prediction

Committed before the implementation commit. Base `5cfca8170` (`integrate/r62`).
USER DECISION 2026-11-23: "RETURN-FAMILY-STAR-AFTER-UNRESOLVED-CAST -> Let's
get started."

## What was known when this was written

No implementation was measured. Known from reading the base and from a scratch
end-to-end probe (not committed):

* **Reachability.** An unresolved type name reaches family S.
  `long w(...) { const long *p = 0; return (volatile long) *p; }` is decided by
  family S at the base with **no base** (`bases: []`), and the function is
  `Safe`. The same body with `(long) *p` or `*p` makes `p` a base, and the flow
  check refuses it (`Unknown`, `unknown_alias:p`). `volatile` is one of the
  declaration reader's rejecting qualifiers, so `is_cast_group` does not
  resolve `(volatile long)` and `unary_position` reads the `*` as a
  multiplication. That is the admitting direction the follow-up names, and
  here it certifies a null dereference.
* **The may-bind direction.** For `return (volatile long) *q;` with `q`
  may-bind, family S refuses at the base (`stars_before` counts no `*`), and
  W47-ACCESS-PATH-POSITIONS' typed return route then decides it from Clang's
  AST (`UseAfterRelease` after `k_free(q)`). Routing the `)` arm through the
  shared detector alone would make `q` an admitted dereference base of family
  S instead.
* A rough `grep` of Fluent Bit 046accf6 `src/`, `plugins/` and `lib/monkey`
  finds 11 single-line `return` expressions with a `) *` or `) &` (not
  counting `sizeof`). They are calls, address-of returns into pointer results
  (not family S) and a parenthesised arithmetic group, which the shared
  detector reads as binary.

## Planned change

`legacy_return_family` keeps `is_cast_group` where it resolves a type, and asks
`ampersand_runs::may_close_a_cast` about every other `)`. A `)` the detector
cannot rule out is **undecided**, and family S keeps every refusal and every
base that either reading raises:

* the `&` refusal fires when either reading is unary;
* an untracked root after an undecided `*` is a base (the union of both
  readings' bases);
* a may-bind root's `stars_before` must count exactly one `*` under **both**
  readings, and a member or subscript root must count none under the cast
  reading. So `(T) *q` keeps its refusal, and `(a) * *q` with `a` undecided
  is newly refused.

Every change refuses more or adds a base. None admits more.

## Predicted movements (opt-in route, base vs tip)

* An added base can only add a refusal (`unknown_alias`) where the untracked
  root's binding is null or unknown. The corpus spellings above are not
  family S's undecided shapes, so no move is expected. Any move would be
  `Safe` -> `Unknown` at a `return` with an unresolved parenthesised name
  before `*` or `&`.
* A newly refused may-bind root falls through to the typed return route,
  which decides it from Clang's AST. No move is expected.

```measure-prediction
route corpus
none
route system
none
route slice
none
```

**Pinned routes** (`quirin-audit measure --declared`) need `build/cycle19`
inputs this host does not have. The integrator should expect the same as on
the opt-in route. Any move is unpredicted by name and is reported.
