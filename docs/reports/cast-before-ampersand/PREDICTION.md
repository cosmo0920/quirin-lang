# CAST-BEFORE-AMPERSAND-READ-AS-BINARY: prediction

Committed before the implementation commit. Base `d7540d746` (main). USER
DECISION 2026-11-23: "launch the cast-before-ampersand card".

## What was known when this was written

No implementation was measured. Known from reading the base:

* `legacy_refused_operands::handed_on` (W44-LEAK-PIN-AUDIT's refused hand-on)
  reads a single `&` before an access chain as binary when the token before
  it ends an operand (`ends_operand`: a name, a number, `)` or `]`). A cast
  `(T *) &x->f` therefore hands on the value `x->f` alone, not `x` and every
  prefix of the chain. The reader only **adds** unresolved-effect sites to a
  refused node (it withdraws leak accusations, never makes one and never makes
  a row `Safe`), so its miss leaves a leak accusation that should have been
  withdrawn.
* `legacy_call_instantiation::residual_takes_an_address` (rule 5j's scalar
  return premise) already reads `) &` as unary after a cast close
  (`is_cast_close`: a parenthesised run of identifiers and `*` not preceded by
  a callee name). Its detector misses casts whose type name holds nested
  parentheses, brackets or `...` (function-pointer and pointer-to-array casts).
* The other 26 rule 74e call sites read any odd run as an address whatever
  precedes it, so a cast cannot hide an address from them. Rule 71's
  `logical_and_before` decides only a run of exactly two `&` before a group,
  which no cast can precede.
* The Fluent Bit source spells `* ) & name ->` or `* ) & name .` 337 times
  (a rough `grep`, not a census of refused statements).

## Planned change

One shared cast-close detector in `ampersand_runs.rs`, used by both readers,
reading a parenthesised group before `&` as a possible cast unless it is a
call's argument list or definitely an expression (an expression-only token, or
a single name the function declares as an object). An undecided group is a
cast: the `&` is unary and the whole chain is handed on.

## Predicted movements (opt-in route, base vs tip)

* **Refused hand-on**: more places handed on at refused statements that spell
  `(T *) &x->f`. Only withdrawals: `Unsafe` → `Unknown` where a leak
  accusation was a row's only violation and its identity is reached through
  such a cast, and leak leaves gone from rows that stay `Unknown` or `Unsafe`.
  No `Safe` row moves through this reader (it fires only at refused nodes).
* **Rule 5j**: the broadened detector reads a few more return expressions as
  taking an address, refusing the hook there: a `Safe` → `Unknown` move is
  possible but not expected (a return that casts an address and calls a
  function, through a function-pointer or array type name, is rare).

Expected totals equal to the base's or within a few rows; every moved row is
reported and any lost `Safe` row is shown with its cast address in the C
source.

```measure-prediction
route corpus
none
route system
none
route slice
none
```

**Pinned routes** (`quirin-audit measure --declared`) cannot run on this host
(no `build/cycle19` inputs or baselines). Expected there as on the opt-in
route: possibly a few `Unsafe` → `Unknown` leak withdrawals and leaf-only
changes, no `Safe` row lost unless through rule 5j's broadened detector. Any
move is unpredicted by name and is reported.
