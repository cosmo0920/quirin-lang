# W44-BRANCH-CONDITION-CALLS: frozen prediction

Frozen before the admission is enabled. Base `c1e69bb5`; census commit
`df5edb61` (recogniser compiled in with `ADMIT = false`, census stream behind
the `indirect-census` feature). Nothing in this file is edited after the AFTER
run; outcomes are recorded in the README beside it.

## Measured before the rule

* **BEFORE** (`c1e69bb5`, plain release binary): diagnostics, chunk-contracts
  and source are byte-identical to the W43-INDEXED-RESOURCE-COLLECTIONS
  integration baseline; contracts identical after the checkout-root alias;
  source-effects differs only in its checkout-path `resource_contracts`
  digest ([before-identity.txt](before-identity.txt)).
* **Census run** (`df5edb61`, `--features qcore-c-gateway/indirect-census`):
  the same answer on all five artifacts, so the recogniser and the stream are
  inert ([census-identity.txt](census-identity.txt)).
* The family at the baseline (Unknown functions): branch condition 8,287 /
  2,320 / 49 sole; loop 294 / 187 / 5; for-loop 839 / 537 / 1; do-while 29 /
  27 / 0; switch 78 / 75 / 1.

## The split ([census/predict.txt](census/predict.txt))

Every final census record is the rule's own answer for one controlling
expression every pre-existing route refused. By first refusal, branch
condition leaves (Unknown functions):

| Outcome | Leaves | Functions | Owner |
|---|---:|---:|---|
| call-free expression (`no-call`) | 3,900 | 1,551 | untyped / chain / address / write classes |
| a call the classifier refuses | 1,780 | 952 | indirect argument (2,413 leaves over all positions) → W44-INDIRECT-CALL-ARGUMENT-ACCESS; uncontracted callee (1,000) → W44-CORPUS-CALLEE-SUMMARIES / libc |
| a call-free leaf beside a call refused | 1,200 | 616 | chain / untyped classes |
| mixed call-refused + no-call on one line | 555 | 95 | as above |
| **admitted** | **269** | **177** | this card |
| type operator | 198 | 95 | refused by design |
| assignment shape (store through a pointer, member) | 204 | 86 | refused by design |
| assignment destination (formal, opaque, descriptor) | 67 | 61 | refused by design |
| recursive local callee | 31 | 18 | constraint 4 premise |
| other shapes (braces, indirect callee, residual, increment, custody result) | 69 | ~60 | refused by design |

By operator shape, the call-bearing leaves are dominated by `plain` (2,049)
and `!` (709): the short-circuit and ternary shapes the brief names are a
minority of the family (`&&` 101, `||` 57, mixed ~300), because most
call-bearing conditions fail on the **call**, not on the connective. By callee
status, 3,926 call-bearing branch leaves carry some uncontracted callee; 142
are all externally contracted, 58 carry a contracted local callee.

## Forecast

1. **Leaves leaving the families: 276** on Unknown functions — branch 269,
   loop 6, do-while 1, for-loop 0, switch 0. Leaves of `Unsafe` functions are
   not forecast (the census join covers Unknown functions); any such removal is
   attributed after the run.
2. **Function moves: 0** Unknown → Safe (upper bound from the census: no
   Unknown function has every leaf admitted). **Unit moves: 0.**
3. **Leak**: 271 → **271 to 276**. Admitting a condition removes an
   unmodeled path flag, so a live resource on a path whose only refusal was the
   condition becomes visible to `report_leaks` (the accusing direction,
   constraint 7). Every added leak is a provisional candidate, attributed leaf
   by leaf; none is an upstream defect claim.
4. **unknown_alias**: 4,358 **± 100**. Removing a refusal removes its binding
   poison (fewer downstream aliases); a newly modeled call checks its actuals
   (possibly more).
5. **unsupported**: 45,848 → about **45,570** (−276 condition leaves, plus
   any Unsafe-function condition leaves; a desugared call adds no
   `unsupported` leaf, since only a `ContractCall` is admitted).
6. **Violation candidates**: leaks only, if any; no double release,
   invalid release or use after release is forecast.
7. **Lowering identity**: `if ((p = malloc(n)) == NULL)` keeps its refusal
   byte for byte (owned result, `CustodyResult`/classifier refusal); every
   condition the rule refuses keeps its site numbering (counters restored).

## Composition (not this card's movement)

The rule consumes the ordinary contract lookup, so W44-CORPUS-CALLEE-SUMMARIES'
summaries and W44-INDIRECT-CALL-ARGUMENT-ACCESS's argument admission reach
condition calls without further change. At this census, 2,413 condition leaves
(all positions) fail first on an indirect call argument and 1,000 on an
uncontracted callee; a first-failure count is an upper bound on what those
cards can unlock here, since a later leaf may refuse on its own.
