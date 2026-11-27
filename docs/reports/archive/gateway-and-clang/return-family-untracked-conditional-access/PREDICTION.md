# RETURN-FAMILY-UNTRACKED-CONDITIONAL-ACCESS: prediction

Committed before the implementation commit. Base `03aadb221` (the tip of
RETURN-FAMILY-UNTRACKED-ACCESS-THROUGH-ARITHMETIC, rule 74w, approved as r77;
on rule 74t's r74, rule 74r's r72 and rule 74m's r68). USER DECISION
2026-11-24: "launch the two arithmetic-access follow-up cards".

## What was known when this was written

No implementation was measured.

* **Reachability**, from the finding card's probes (its report §1) and the
  follow-up it registered: `(0 ? p : q)->n` through null untracked roots is
  `Safe`. Family S's `scalar_read` refuses only GNU `?:`, so a conditional
  reaches its untracked branch. There no test reads a root that is an arm of
  `?:` inside the accessed group: the direct tests need `->`, `[` or `*`
  next to the name, and rules 74r's, 74t's and 74w's walks stop at the `?`
  or `:` beside it. Family S is asked inside `return_expression_instruction`,
  before rule 10n's/10z's `typed_connective_return`, which is consulted only
  when every route there declined, so rule 10z's arm lowering never sees an
  expression family S admits.
* **Census.** [scripts/census.py](scripts/census.py) reads every translation
  unit the route's two compile databases name (Fluent Bit 046accf6's
  `fluent-bit-clang-production` and Monkey's `monkey-build`). It reports a
  `return` in which a parenthesised group with a top-level `?` reaches `->`,
  `[` or a unary `*` through parentheses, type groups and parenthesised
  additive groups, without classifying the arms (a superset of the planned
  reading, and of family S's reach: a call is not excluded). It finds **0
  `return`s** (`private/rfc/census-route.txt`, not committed; synthetic
  spellings `(0 ? p : q)->n`, `*(k ? p : q)`, `((T *)(k ? p : q))[0]` and
  `((k ? p : q) + 1)->n` are hits, `k ? p->n : q->n` and `(k ? p : q) != 0`
  are not). Headers are not read.

## Planned change

A new function in `legacy_return_family.rs` finds every parenthesised group
whose top level is a conditional and widens it outward as rule 74w's walk
does (type groups before it, parentheses around it, classified additive
groups). When `->` or `[` follows what it reached, or a `*` that may be
unary precedes it, both arms are classified: parentheses, casts, a nested
conditional (whose condition is a read), a sum whose other operands are
literals, declared scalars or `sizeof` of a resolved type, a numeric literal,
a loaded value (`a->f`, `a[i]`, `*a`, whose roots the old tests read) and a
name. Every untracked, non-scalar name an arm yields is a base, and a `->`
root for `->`. Either condition is never decided, even when constant. An arm
the reader cannot classify, or an access through an unclassified additive
group, refuses the whole expression. The existing tests are untouched; the
reading is their union with this one. Bases and `->` roots only grow, and
the only new refusal is that expression's, so no result is admitted that the
base did not admit.

## Predicted movements (opt-in route, base vs tip)

A move can only be `Safe` -> `Unknown` (or `Unsafe` -> `Unknown` through a
new refusal), at a `return` family S admits whose expression accesses a
conditional group. **Expected: none**, since the census finds no such
`return` in the route's units. No `Unknown` -> `Safe` move is possible.

```measure-prediction
route corpus
none
route system
none
route slice
none
```

**Pinned routes** (`quirin-audit measure --declared`) need inputs this host
does not have (rules 74r, 74t and 74w were refused for a missing
`compile_commands.json`). The integrator should expect the opt-in route's
result; a move there would be one this file does not predict.
