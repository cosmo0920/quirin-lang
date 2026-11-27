# Landing batch A (integrate/r74): return-family rules

USER DECISION 2026-11-26: "Three batches". Batch A lands ten finished cards in
one integrate branch with one full landing gate, in the approved queue order:

| Card | Report | Rule |
|---|---|---|
| CG25-IN-PLACE-REGEX-REGERROR-FAILURE-EDGE | [report](../../language-and-tooling/regex-regerror-failure-edge/README.md) | 74n |
| RETURN-FAMILY-UNTRACKED-MEMBER-THROUGH-CAST | [report](../../gateway-and-clang/return-family-untracked-member-cast/README.md) | 74t |
| TYPE-GROUP-QUALIFIED-POINTER | [report](../../gateway-and-clang/type-group-qualified-pointer/README.md) | (cast reader) |
| RETURN-FAMILY-UNTRACKED-ACCESS-THROUGH-ARITHMETIC | [report](../../gateway-and-clang/return-family-untracked-arith-access/README.md) | 74w |
| RETURN-FAMILY-UNTRACKED-REVERSED-SUBSCRIPT | [report](../../gateway-and-clang/return-family-untracked-reversed-subscript/README.md) | 74y |
| RETURN-FAMILY-UNTRACKED-CONDITIONAL-ACCESS | [report](../../gateway-and-clang/return-family-untracked-conditional-access/README.md) | 74x |
| RETURN-STRICT-MEMBER-NULL-UNTRACKED-ROOT | [report](../../gateway-and-clang/return-strict-member-null/README.md) | (strict member) |
| RETURN-FAMILY-ARITH-WALK-CROSSES-CONDITIONAL | [report](../../gateway-and-clang/return-family-arith-walk-conditional/README.md) | 82 |
| UNTRACKED-BASE-NULL-JOIN-DISCHARGE | [report](../../language-and-tooling/untracked-base-null-join/README.md) | 83 |
| RETURN-FAMILY-CONDITIONAL-CONSTANT-PRUNING | [report](../../gateway-and-clang/return-family-conditional-constant-pruning/README.md) | 74z |

Each card's own report holds its evidence, prediction and measurement; this
page records the integration only. The landing gate record is
[integration/gate.json](integration/gate.json).

## Integration

* Base: main 1ecaf97e9. Main reorganised `docs/QCORE_CONTRACT_SPEC_r0_3.md`
  and `docs/QCORE_IMPLEMENTED_GRAMMARS_r0_3.md` after the regerror card's
  base; its additions were placed in main's structure (C04.6.1 paragraph,
  `kind_predicate` alternative, a Failure use bullet).
* `crates/qcore-c-gateway/src/legacy_return_family.rs` conflicted twice on
  independent additions (reversed subscript against conditional access, and
  arith walk against constant pruning); both sides were kept each time.
* The regerror card's new `in_place_regex_failure_use.rs` declared regex
  sizes as plain `unsigned long`; they are spelled `__SIZE_TYPE__`
  (docs/development/windows-types notice, rule 11q).
* Constant pruning: USER DECISION 2026-11-25, macro option A (the analysed
  preprocessed expansion).
* Amendments applied in queue order with `--profiles-base origin/main`.

## Registered here

RETURN-AST-ROUTE-REVERSED-SUBSCRIPT-DEREF: the reversed-subscript report
(Observed, not registered) found `long w(long **q, long n) { return n[q[0]]; }`
`Safe` at base and tip, admitted by an older typed route before family S,
which refuses it (unit pin `n [ q [ 0 ] ]`).
