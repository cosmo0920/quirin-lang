# W45-RETURN-EXPRESSION-FAMILIES — frozen forecast

Written and committed **before** any rule of this card was implemented, on base
`b160d765`. It is not edited after the AFTER run; misses are recorded in the
report beside it.

## Inputs

* BEFORE run `measure/before-census/` (census-feature release build of the
  base, `run_audit.py … --census`, exit 0): the verdict-bearing diagnostics,
  chunk-contracts and Monkey source artifacts are byte-identical to the
  baseline `docs/reports/w44-collection-allocation/integration/artifacts/`,
  contracts identical after the root alias, source-effects differ only in
  checkout-path digests ([before-identity.json](before-identity.json)).
* Re-census ([census/census-before.txt](census/census-before.txt), W43's
  `return_census.py` unchanged): **496** published return-expression leaves,
  all covered by a census record; **495** on **417** Unknown functions; the
  refusal is the only distinct reason of **147** functions (149 leaves).
* Forecast script [scripts/predict.py](scripts/predict.py) (output
  [census/prediction.json.gz](census/prediction.json.gz)). It reads the
  enclosing function's declared result from the pinned *source* text
  heuristically; the Rust rules read the preprocessed tokens, so the figures
  below are a forecast of the rules, not a replay of them.

## The rules this card intends to land

**S — scalar-result read.** `return E;` in a function every declaration of
which declares a non-pointer scalar result (arithmetic, enumeration, or a
typedef the unit's reader resolves to one), where `E` carries no call (a
`sizeof`/`_Alignof` over a parenthesised type or name is a constant), no brace,
assignment, increment, comma or unary `&`, and reads no tracked descriptor.
Every name `E` evaluates is a name this body never binds or a declared scalar,
except that a name the body may bind may be the root of exactly one
dereference (`B->F`, `B->F.G`, `B[I]`, `*B`) in a position C always
evaluates. The value is a scalar: it carries no custody, so the exit escapes
nothing (`NoOp`). The evaluation is one read node before the exit whose
dereferenced bases keep the typed access route's obligations (the base check
and the may-alias check).

**K — cast of a whole call.** `return ( T ) … CALL ( … );`. The synthetic
`PLACE = CALL(…)` is classified as W4 does. A refusal is lowered to that
refusal and a `NoOp` exit (W43's re-attribution, now under a cast). An
admitted call is taken only when its result carries nothing (`None`) or when
it is `Owned`, every cast in the chain targets a pointer type and the unit
declares the callee to return an object pointer, so the conversion keeps the
object (C11 6.3.2.3p7) and the exit escapes exactly the fresh site. Every
other admitted classification keeps the return refusal.

**Kept refused, sized in the report:** pointer-valued results (a load, an
interior address and an array decay cannot be told apart without member
typing, and an interior address is a derived result), compound expressions
with a call, comma expressions, and the may-bind shapes above.

## Forecast

| Measure | BEFORE | Forecast |
|---|---:|---:|
| return-expression leaves on Unknown functions | 495 | **≈ 312** (160 S removed, 23 K removed) |
| functions sole-blocked by the return refusal | 147 | **≈ 80** |
| sole-blocked functions that lose every return leaf through S or a K admission | — | **67** (range 50–67: the Rust result reader may not commit to some typedef results) |
| leak | 37 | **37** (S escapes nothing and reads no tracked value; K escapes only a fresh site) |
| provisional Unsafe functions | 12 | **12** |

Per family (Unknown leaves → forecast): F access 289 → 143 S; E call-free
conditional 26 → 11 S; D type operator 6 → 6 S; A whole call under a cast 23 →
20 K (lowered to the classifier's own refusal, no status move); B contracted
call under a cast 3 → 3 K (the two `XXH*_createState` `Owned` results escape
their fresh site; the `None` one is admitted); C compound with a call 79 and G
comma/braces 69 stay refused.

**Function moves.** The freed sole-blocked functions are forecast
Unknown → **Safe** (their only reason was the return refusal and neither rule
adds a diagnostic on an untracked read). A freed function can validate a W4
summary, so callers whose only other blocker was an uncontracted call to it
may also move Unknown → Safe; that cascade is not forecast function by
function. **No** function is forecast to move to provisional Unsafe. An S read
through a may-bind base can add `unknown_alias` where its base check fails;
forecast: a handful, never an accusation.

**Unsupported** falls by about the removed S and B leaves (≈ 163) plus any
cascade; the 20 A leaves are exchanges (same coordinate, the classifier's
refusal wording).
