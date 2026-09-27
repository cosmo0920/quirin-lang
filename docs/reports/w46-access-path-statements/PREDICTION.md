# W46-ACCESS-PATH-STATEMENTS: frozen prediction

Frozen before this card changes gateway or Lean semantics, on base
`207cebe4cdb745f6216ff2b511c4b7fd134581b9`. The earlier P1 prediction
at `9bce3986` remains authoritative for its population and function names:
[P1 prediction](../w46-access-path-census/PREDICTION.md) and
[`prediction.json`](../w46-access-path-census/artifacts/prediction.json).

P1 predicts 45 statement-position functions freed: 38 Safe, seven Unknown
because a tracked root has no dominating non-null fact, and zero provisional
Unsafe. The three cross-unit candidates require a separate summary route and
are excluded. Eighteen of the 38 Safe candidates carry no typed access; the
lexical gate refused non-access syntax. The leaf upper bound is 4,423 `IND`
leaves. This card retains the old routes beside the new typed reader until
parity is measured. Any changed Safe, added provisional Unsafe, or unforecast
Unknown-to-Safe requires individual attribution. The current base includes six
new contract-driven Safe functions relative to P1, so overlap with this list
must be measured, not presumed.

The intended after state is per-hop non-null/live obligations on tracked
pointer roots, explicit scope boundaries on untracked roots and loaded
pointers, custody-neutral non-pointer value stores and borrowed pointer reads.
Pointer-valued stores, unions, casts, call-bearing statements, conditions,
returns and call arguments remain refused by this card.
