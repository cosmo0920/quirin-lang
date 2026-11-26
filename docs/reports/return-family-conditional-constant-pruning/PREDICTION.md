# RETURN-FAMILY-CONDITIONAL-CONSTANT-PRUNING: prediction

Committed before the implementation commit. Base `1187c3f6d` (the tip of
RETURN-FAMILY-UNTRACKED-CONDITIONAL-ACCESS, rule 74x, approved as r79; on
rules 74w (r77), 74t (r74), 74r (r72) and 74m (r68)). USER DECISION
2026-11-24: "launch the conditional-access follow-up cards".

## What was known when this was written

No implementation was measured.

* **Reachability.** Rule 74x's `arm_roots` makes every untracked root of
  both arms of an accessed conditional a base and never decides the
  condition, so `long w(const struct s *q) { const struct s *p = 0; return
  (0 ? p : q)->n; }` is `Unknown` (`unknown_alias:p`), though C never
  evaluates `p` (C11 6.5.15p4). Family S is asked before rule 10z's
  `typed_connective_return`, so family S's reading is the verdict's.
* **Census.** Rule 74x's [scripts/census.py](scripts/census.py), copied
  verbatim, reports every `return` on the route's units (Fluent Bit 046accf6's
  `fluent-bit-clang-production` and Monkey's `monkey-build`) in which a
  parenthesised conditional reaches `->`, `[` or a unary `*`. It is a
  superset of what this card can touch, because pruning applies only inside
  such a group. Rerun at this base, it finds **0 `return`s**
  (`private/rfcp/census-route.txt`, not committed: `total 0 with-call 0
  without-call 0`). Headers are not read.

## Planned change

A new function decides a conditional's condition only when it is an integer
constant expression of integer literals (decimal, octal, hex, with
suffixes), parentheses, unary `+ - ! ~` and the binary arithmetic, shift,
bitwise, relational, equality and logical operators. It evaluates the
expression exactly under the readers' LP64 premise (rule 11y), with C11's
literal typing and usual arithmetic conversions. Division or modulo by zero,
signed overflow, an out-of-range shift, a negative left operand of a signed
shift and anything else leave the condition undecided. Names (enumeration
constants and macros included), `sizeof`, casts, calls, floats and
characters are never decided. In `arm_roots`, a decided condition drops the
rule 74x bases of the arm C does not evaluate. That arm must still be
classified, so its refusals stand. Every other test, and every base the old
tests give inside that arm, is kept. An undecided condition keeps rule 74x's
reading.

## Predicted movements (opt-in route, base vs tip)

A move can only be `Unknown` -> `Safe` (or `Unknown` -> `Unsafe`), and only
at a `return` family S admits whose accessed conditional has a decided
condition and an untracked root in its untaken arm. **Expected: none**,
because the census finds no accessed conditional in a `return` on the route.

```measure-prediction
route corpus
none
route system
none
route slice
none
```

**Pinned routes** (`quirin-audit measure --declared`) need inputs this host
does not have. Rules 74r, 74t, 74w and 74x were refused for a missing
`compile_commands.json`. The integrator should expect the opt-in route's
result. A move there would be one this file does not predict.
