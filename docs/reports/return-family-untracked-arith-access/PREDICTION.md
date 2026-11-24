# RETURN-FAMILY-UNTRACKED-ACCESS-THROUGH-ARITHMETIC: prediction

Committed before the implementation commit. Base `497c859fc` (the tip of
RETURN-FAMILY-UNTRACKED-MEMBER-THROUGH-CAST, rule 74t, approved as r74; on
rule 74r's r72 and rule 74m's r68, on main `3ca85c7a6`). USER DECISION
2026-11-24: "launch both cards".

## What was known when this was written

No implementation was measured.

* **Reachability**, from the finding card's tip probes (its report §1) and
  the follow-up it registered: `(p + 1)->n`, `((const struct s *)p + 1)->n`
  and `(p + 1)[0]` through a null untracked `p` are `Safe`. Rule 74t's
  `accessed_through_groups` reads back from the root only through `(` and
  type groups, so the `+` stops it, and the direct tests need `->` or `[`
  right after the name. Rule 74r's `dereferenced_through_groups` reads
  `*(p + 1)` (the `+` is after the root), but not `*(n + p)`, where the walk
  back meets the `+` first.
* **Census.** [scripts/census.py](scripts/census.py) reads every translation
  unit the route's two compile databases name (Fluent Bit 046accf6's
  `fluent-bit-clang-production` and Monkey's `monkey-build`). It reports a
  name that reaches `->`, `[` or a unary `*` through parentheses, type
  groups and at least one parenthesised additive group, without classifying
  the other operands (a superset of the planned reading). It finds **5
  `return`s** (`private/rfa/census-route.txt`, not committed), and every
  one is inside a call (`match_class (…)` twice, `luaL_error (…)`,
  `hd_ringbuf_get (…) ->` twice, the last two a census over-match through an
  argument list's comma). Family S refuses a call of any kind, so none can
  move. Headers are not read.

## Planned change

A new function in `legacy_return_family.rs` walks outward from an untracked,
non-scalar root through type groups, parentheses that enclose exactly what it
has read, and parenthesised additive groups (`+` or `-` on either side of
what it has read, inside a group whose `(` and matching `)` it reads). When
it crosses at least one additive group and what it reached is followed by
`->` or `[`, or follows a `*` that may be unary, the root is a base (and a
`->` root for `->`). If one of the additive groups holds anything other than
numeric literals, declared scalars, `sizeof` of a resolved type,
parentheses and `+ - * / %`, or the root's neighbours in it are not `+`,
`-` or the group's own parentheses, an access refuses the whole expression.
The existing tests are untouched; the reading is their union with this one.
Bases and `->` roots only grow, and the only new refusal is that
expression's, so no result is admitted that the base did not admit.

## Predicted movements (opt-in route, base vs tip)

A move can only be `Safe` -> `Unknown` (or `Unsafe` -> `Unknown` through a
new refusal), at a `return` family S admits whose expression accesses an
untracked root through pointer arithmetic. **Expected: none**, since every
census hit contains a call. No `Unknown` -> `Safe` move is possible.

```measure-prediction
route corpus
none
route system
none
route slice
none
```

**Pinned routes** (`quirin-audit measure --declared`) need inputs this host
does not have (rules 74r and 74t were refused for a missing
`compile_commands.json`). The integrator should expect the opt-in route's
result; a move there would be one this file does not predict.
