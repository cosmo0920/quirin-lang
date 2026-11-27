# RETURN-FAMILY-ARITH-WALK-CROSSES-CONDITIONAL: prediction

Committed before the implementation commit. Base `1187c3f6d` (the tip of
RETURN-FAMILY-UNTRACKED-CONDITIONAL-ACCESS, rule 74x, approved as r79; on
rule 74w's r77, rule 74t's r74, rule 74r's r72 and rule 74m's r68). USER
DECISION 2026-11-24: "launch the conditional-access follow-up cards".

## What was known when this was written

No implementation was measured.

* **The refusal**, from rule 74x's report §2 and its unit pins: in
  `( n ? 1 + p - n : a ) - > m` and `( n ? a : n + p ) [ 0 ]`, rule 74w's
  `accessed_through_arithmetic` meets a binary `+` before `p` and an
  operator or `)` after it, takes the conditional's parentheses as the
  additive group (`enclosing_group`), finds `?` among the other operands,
  and returns `Unclassified`, so family S refuses the expression. That is
  the base's reading, fail-closed.
* **Census.** Rule 74x's [scripts/census.py](scripts/census.py), copied
  verbatim, reports every `return` in a route unit in which a parenthesised
  group with a top-level `?` reaches `->`, `[` or a unary `*` through
  parentheses, type groups and additive groups. The refusal this card
  relaxes needs exactly that: rule 74w's walk crosses the conditional group
  and then reaches an access, and the walk past the conditional group is the
  one the census follows. So the census is a superset of this card's reach.
  Rerun at the base over Fluent Bit 046accf6's `fluent-bit-clang-production`
  and Monkey's `monkey-build` databases, it finds **0 `return`s**
  (`private/rawc/census-route.txt`, not committed). Headers are not read.

## Planned change

Rule 74w's walk keeps its path and its old result, but when the group it
widens into holds a top-level `?` and the span lies in an arm (the first
top-level `?` or `:` after the span is not a `?`, and one precedes it), the
group is classified within that arm: the arm's own delimiters stand in for
the group's parentheses. When the old result is `Unclassified` and every
crossed group is classified under that arm reading, the walk returns a new
outcome, `Arm`. Family S then refuses as before unless rule 74x's
`conditional_roots` made that root a base. So the one refusal removed is the
arm delimiter's, and only where rule 74x reads the arm and makes the root a
base. Every other refusal (a root in the condition, a non-additive operand,
another pointer term, a subtracted pointer, an unclassified group before or
after the conditional) stays.

## Predicted movements (opt-in route, base vs tip)

A move can only be `Unknown` -> `Safe` or `Unknown` -> `Unsafe`, at a
`return` family S refused only because rule 74w's walk read a conditional
group as an additive group. **Expected: none**, since the census finds no
`return` in the route's units with an accessed conditional group at all.

```measure-prediction
route corpus
none
route system
none
route slice
none
```

**Pinned routes** (`quirin-audit measure --declared`) need inputs this host
does not have (rules 74r, 74t, 74w and 74x were refused for a missing
`compile_commands.json`). The integrator should expect the opt-in route's
result; a move there would be one this file does not predict.
