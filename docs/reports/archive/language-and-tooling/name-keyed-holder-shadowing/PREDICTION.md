# NAME-KEYED-HOLDER-SHADOWING: prediction

Committed before the implementation commit. Base `43410b230` (main).
USER DECISION 2026-11-25, "lanich four of queued cards". Discharges
`NAME-KEYED-HOLDER-SHADOWING`, registered by FRAME-HOLDER-SHADOWED-NAME.

## What was known when this was written

No implementation was measured. Known from reading the base and from the
pins `crates/qcore-c-gateway/tests/name_keyed_holder_shadowing.rs`
(committed `a4bfa1e91`), run at base (`private/card-logs/pins-at-base.log`:
5 passed):

* **The readings.** Rule 12t's formal-keyed readings decide from
  `CallerFormals::contains`, a token census of the formal's spelling: the
  retained actual `NAME` (a caller's object), the interior `&NAME->f`, the
  path holder `NAME->f` (`path_holder_at`), and a removal keyed on either.
* **Not reachable at base.** Every probe of a shadowing local (inner block,
  for-init, pointer, array, typedef'd array, `__auto_type`) refuses at the
  classifier's guard (`local declaration may shadow a formal resource
  identity`, `legacy_classify.rs`). A declaration the guard does not
  recognize (an opening `__attribute__`, `_Alignas`, `__typeof__`, a
  parenthesized declarator, a block-scope enumeration constant) refuses by
  another route. Those refusals are incidental. A path holder is also
  protected by its own census: the shadow's declarator is a bare mention of
  the root, so the path is not stable. No probe is `Safe`.

## Planned change

The formal-keyed readings themselves also require Clang's declaration
reading: the name has one declaration in the function, its `ParmVarDecl`,
and every `DeclRefExpr` of the name designates it. A formal that fails this
check is read as a plain name: a tracked identity or undecided
(`retention-undecided`). A path holder there is no path holder, and a
removal keyed on it is unmatched, so it poisons and drops nothing. Only
`Safe -> Unknown` is possible.

## Predicted movements

None. At base a shadowing declaration already refuses its function. A
Fluent Bit or Monkey function whose formal-keyed retention, path holder or
removal names a formal that a second declaration in the same body shares
is not expected outside those refusals. If one exists, it moves
`Safe -> Unknown`, never the other way. No `Unknown`, `Pending` or `Unsafe`
row becomes `Safe`, and no `Unsafe` is added.

Opt-in `current` (this card's arbiter): no row moves and the totals are
identical.

```measure-prediction
route corpus
none
route system
none
route slice
none
```

**Pinned routes** (`quirin-audit measure --declared`) cannot run on this host
(no `build/cycle19` inputs or baselines). Expected there as above: no move.
