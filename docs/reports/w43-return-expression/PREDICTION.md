# W43-RETURN-EXPRESSION — frozen forecast

Written at implementation commit `29f4a907` from the BEFORE census
(`census.json`), **before** the AFTER corpus run. It is a forecast, not a
result; the report compares against it and keeps any miss.

Rule measured: a return expression that is exactly one call `NAME ( … )`,
whose synthetic assignment `PLACE = NAME ( … );` the statement classifier
refuses, is lowered to that refusal and a `NoOp` exit
(`return_call::lowers_refused_whole_call`).

1. **Removed:** exactly the 940 census leaves in families A (whole uncontracted
   call, no cast: 635 defined in this unit, 290 declared elsewhere, 4 compiler
   builtins) and B (whole contracted call refused by the classifier, no cast:
   11) — all on Unknown functions, 724 functions.
2. **Added:** one classifier refusal at the same internal line for each removed
   leaf, fewer only where an identical `(function, line, description)` leaf
   already exists (set semantics) or two removed leaves share a line.
   Expected descriptions: mostly `call to uncontracted function \`…\``, with
   the lexical `indirect memory access is outside the resource projection` and
   nested-call/argument refusals for calls whose arguments the classifier
   refuses first.
3. **Unchanged:** the other 504 return-expression leaves (500 on Unknown
   functions), every leak (272), `unknown_alias`, `summary_conflict`, every
   function status (223 Safe / 84 provisional Unsafe / 4,527 Unknown), every
   unit status (2 / 42 / 137 / 4). No function becomes `Safe`.
4. **Blocker census:** of the 465 functions whose only distinct blocker was the
   return refusal, 315 lose every return leaf through this rule and become
   blocked by the classifier's refusal instead; they stay Unknown.
5. **Other artifacts:** ChunkIO contracts, selected contracts, source effects
   and Monkey source identical to the same-checkout BEFORE.
