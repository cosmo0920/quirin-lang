# W45-PARENTHESISED-CALLEE: frozen prediction

Frozen before any gateway code changes. At this commit the only new file
besides this one is the test file `crates/qcore-c-gateway/tests/parenthesised_callee.rs`,
which pins the intended behaviour and fails on this tree.

## Base

Local `main` `1db9a569` (W45-CONTRACT-DEFERRED-REVIEW integrated; the brief
named `2e6ae8f9`, and `main` moved while the card started). A BEFORE run at
`2e6ae8f9` reproduced the baseline
`docs/reports/w45-sub-object-stores/integration/artifacts/` byte for byte in
`diagnostics`, `chunk-contracts` and `source`, `contracts` after the
checkout-root alias, and `source-effects` differs only in checkout-path
digests. The BEFORE run at `1db9a569` must reproduce the same baseline.

* functions Safe / provisional Unsafe / Unknown: 390 / 12 / 4,432;
* `leak` 37, `unknown_alias` 3,568, `unsupported` 42,694, `summary_conflict` 1,
  violations 0.

## Root cause (read from the code before measuring)

Every lexical call reader of the gateway (`find_call`, `contains_call`,
`nested_call`, the condition, return and neutral-position readers, the
withdrawal and hold rewrites) recognises a call only as the token pair
`NAME (`. A call whose callee is anything else is invisible to all of them,
and the statement classifier's last arm lowers a statement with no visible
call, assignment, increment or access to `NoOp`. The same arm is reached by a
`for` header's init and increment clauses, which reach the classifier with
their grouping tokens stripped (`W4-EXPR-POS-FOR-CLAUSES`), so `q_free(p)`
there reads as the two names `q_free p`.

## Forecast

No census was taken before the rule; the forecast is directional, by class.

* **Normalised `(NAME)(…)` sites** are decided exactly as `NAME(…)`. Where the
  parenthesised call was the only difference, a function moves the way the
  plain spelling would decide it.
* **Functions leaving `Safe`:** between 0 and 5, every one a correction
  attributed to a statement the gateway skipped (a parenthesised callee, a
  callee expression that is not a name, or a `for` clause with a call). None
  is presented as a regression or an upstream defect.
* **Functions entering `Safe`:** at most 2, only where a parenthesised call's
  old refusal (for example `unsupported assignment expression` for
  `p = (f)(x)`) was the only blocker and the normalised call is fully checked.
* **Provisional `Unsafe`:** may rise by at most 2, only where a newly modeled
  release exposes a double release or a newly modeled acquisition a leak; each
  is classified by reading.
* **Leak pin:** 37 ± 3; each moved leaf attributed.
* **`unsupported`:** changes by fewer than 200 leaves in either direction:
  new refusals of callee expressions and `for` clauses, fewer refusals where a
  normalised call is now modeled.
* **Units:** no unit moves.
