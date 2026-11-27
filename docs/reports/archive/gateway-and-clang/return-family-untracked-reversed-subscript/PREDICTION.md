# RETURN-FAMILY-UNTRACKED-REVERSED-SUBSCRIPT: prediction

Committed before the implementation commit. Base `03aadb221` (the tip of
RETURN-FAMILY-UNTRACKED-ACCESS-THROUGH-ARITHMETIC, rule 74w, approved as r77;
on rule 74t's r74, rule 74r's r72 and rule 74m's r68). USER DECISION
2026-11-24: "launch the two arithmetic-access follow-up cards".

## What was known when this was written

No implementation was measured.

* **Reachability**, from the finding card's probe (rule 74w's report §1) and
  the follow-up it registered: `1[p]` through a null untracked `p` is `Safe`
  at rule 74w's base and tip. C11 6.5.2.1p2 defines `E1[E2]` as
  `(*((E1)+(E2)))`, so `1[p]` dereferences `p` as `p[1]` does. Every base
  test of family S's untracked branch reads the root before a `[` (`p [`),
  after a unary `*`, or through `(`/`)` and type groups; `enclosing_group`
  (rule 74w) stops at a `[`, so a root inside a subscript's brackets is never
  a base. RETURN-FAMILY-SUBSCRIPT-ENCLOSING-STARRED-ROOT (registered by r71,
  rule 74s, not in this base) is the may-bind form of the same shape:
  `long w(long **q, long n) { return n[*q]; }` is `Safe`.
* **Census.** [scripts/census.py](scripts/census.py) reads every translation
  unit the route's two compile databases name and reports a name that fills
  a subscript's brackets through parentheses, type groups and additive terms
  (a superset of the planned reading: it does not decide declared types). It
  finds **302 `return`s** (`private/rfrs/census-route.txt`, not committed):
  57 contain a call, which family S refuses before any base is read; none
  has a numeric literal outside the brackets (no `1[p]` spelling); 245 are
  ordinary `X [ i ]` subscripts.

## Planned change

A new function in `legacy_return_family.rs` walks outward from an untracked,
non-scalar root through type groups, parentheses that enclose exactly what it
has read and parenthesised additive groups (rule 74w's widening), and asks
whether the span then fills the brackets of a subscript `X [ … ]`, as the
whole bracket or as a term of a sum in it. If so, the root is a base unless
the reader knows `X` is the pointer: `X` a name the function's declaration
reader declares as a pointer or array (`FunctionTypes::pointer_kind`), not a
member name. With `X` an integer (a literal, a declared scalar) the root is
the pointer; with `X` undecided both are bases (`X` is already one when it is
untracked). An unclassified sum in the brackets (another non-scalar operand)
with an undecided `X` refuses the expression, as rule 74w refuses. A may-bind
root inside such brackets refuses, which closes the r71 follow-up's shape.
The old tests are untouched; bases only grow and the new refusals are the
expression's, so nothing admits more than at the base.

## Predicted movements (opt-in route, base vs tip)

A move can only be `Safe` -> `Unknown` (or `Unsafe` -> `Unknown` through a new
refusal) at a `return` family S admits. **Expected: none.** The 245 call-free
census hits are ordinary subscripts. Their index is a formal (unbound: its
new base obligation discharges), a declared scalar (not read), or a local the
flow check binds to a non-null untracked value. A move would be a local index
of a type the declaration reader does not read as a scalar, under an `X` it
does not read as a pointer, bound to a possibly-null value; this file does
not predict one, and any measured move is reported as unpredicted. No
`Unknown` -> `Safe` move is possible.

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
result.
