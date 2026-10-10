# NULL-CANDIDATE-TRIAGE: prediction

Frozen before any implementation code. Base: local `main` at `cc7a20c3e`.
BEFORE (binary `private/nct/bin/before`, `01c2dc82…`, run `b0`): corpus custody
**915 / 9 / 3,910** (`ded14632…`), null property 1,219 Safe unconditional /
1,442 Safe conditional / 2,260 Unknown / 0 Unsafe, 12,402 candidates in 2,173
functions (allocation 37, lookup 1, null-literal 95, uninitialized 5, global 1,
uncontracted-result 738, formal 1,816, memory-field 9,607, other 102): the
brief's figures reproduce. System and slice BEFORE are recorded in the report.

## Design fixed here

1. **Findings.** For every candidate check carrying a strong source
   (allocation, lookup, nullable result, null literal, uninitialized, global:
   139 candidates on the base), a path search from the function's entry
   (one path at a time, every branch a decision) looks for a path that reaches
   the dereference with the pointer NULL under a named condition: an
   allocation that fails, a lookup that misses, a null constant, or the null
   arm of the function's own NULL test. A branch decision is admitted only
   when the walk decides it (a NULL test, a constant), or when its condition
   reads values no earlier decision on the path constrained (formals, call
   results, memory read since the last call); a condition re-reading a
   constrained value is *correlated* and ends that path unresolved. A found
   path is a **finding**: the row is `Unsafe` (provisional, never an upstream
   defect), labelled `null-on-allocation-failure` (the only NULL condition is
   an allocation failure) or `null-on-normal-path`. A search that explores
   every path without one is **infeasible** with its reason; one that meets a
   correlated condition or its budget is **unresolved** (the check stays a
   candidate). An uninitialized pointer on the path is recorded as
   `indeterminate`, not as a NULL finding.
2. **Declared non-NULL members**, `requires null_member_nonnull(TAG, FIELD);`
   in bounds contract v1 (the null environment; custody's environment and
   digest unchanged). Every read of `E->FIELD` / `E.FIELD` of a `struct TAG`
   is non-null under the listed assumption; every store to it in every
   analysed unit is a check, and one not proved non-null **withdraws** the
   declaration for the run: every row that relied on it (its dependency set,
   through in-unit summaries) is published `Unknown`. Zero-initialised
   construction is a listed premise, so a member declaration is at best
   `partly-validated`, never `validated`.
3. **Declared non-NULL results**, `ensures normal null_result_nonnull();`,
   checked against every analysed body that defines the function (validated
   when every return is non-null, refuted and withdrawn when one is null on a
   path), standing per rules 13g/13j's fold.
4. **Cross-unit travel** after every unit is analysed (no second analysis): an
   external-linkage definition analysed in exactly one unit exports its
   inferred clauses and its non-null result; a caller in another unit gains a
   `call-precondition` check of each matching actual, and a candidate whose
   every source is an untested result of such callees becomes proved. Neither
   makes a row Safe that relied on anything else.

## Predicted outcome

| Quantity | Prediction |
| --- | --- |
| Custody report, all three routes | byte-identical to BEFORE (the null selection never changes custody) |
| Custody Safe rows with an own null premise, every premise proved | corpus 1 -> 1 to 4 |
| Strong-source candidates (139): findings | 35 to 80 |
| of which `null-on-allocation-failure` | 25 to 40 |
| of which `null-on-normal-path` | 10 to 45 |
| infeasible with a reason | 5 to 35 |
| unresolved (correlated or budget) | the rest, 20 to 80 |
| Null Unsafe rows (corpus) | 0 -> 25 to 60 |
| `rd_kafka_buf_s.rkbuf_rkb` (3,474 candidates) | withdrawn (an explicit `= NULL` reset in `rdkafka_broker.c`) |
| Member declarations kept (not withdrawn) | 2 to 6 of those declared |
| Memory-field candidates discharged by kept member declarations | 200 to 1,500 |
| Cross-unit non-null results: candidates proved | 300 to 470 (462 sole-source on the base) |
| Cross-unit clause travel: new caller checks | 1,000 to 6,000, of which candidates 200 to 2,000 |
| Total candidates after (all kinds) | 10,000 to 14,000 (travel adds, declarations and results remove) |
| Null Safe rows (corpus) | 2,661 -> 2,650 to 2,900 (withdrawn dependants and new obligations can lower it) |
| Any Safe -> non-Safe null move | only through travel's new obligations or a withdrawn declaration, each named |
| Wall time, corpus with the null selection | +0 % to +15 % over BEFORE's 478.9 s |
