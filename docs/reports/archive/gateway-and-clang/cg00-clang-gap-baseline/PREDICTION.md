# CG00-CLANG-GAP-BASELINE: frozen prediction

Frozen on base `d3508e10` before any test, script or attribution was written
(the only file preceding it is `scripts/arbiter.sh`, which runs the arbiter
unchanged). The outcome is compared in [README.md](README.md#prediction-and-outcome).

## Corpus

| Measure | Prediction | Confidence |
|---|---|---|
| BEFORE (release `quirin-audit` at `d3508e10`) reproduces `ae17152b…` | byte-identical | 97% |
| AFTER (tip) byte-identical to `ae17152b…` | yes: the card adds tests, scripts and records only; the release binary is unchanged | 97% |
| Corpus figures | 526 Safe / 10 provisional Unsafe / 4,298 Unknown; 35,737 unsupported leaves | 97% |

## Executed BEFORE tests

| Measure | Prediction | Confidence |
|---|---|---|
| Every one of the 98 carded entries gets a test or a named reading-only justification | yes | 90% |
| Entries whose cited refusal or limit reproduces exactly as the analysis states it | 80–90 of 98 | 60% |
| Corrections to the input (the route accepts the form, or refuses for a different named reason) | 5–15 entries | 60% |
| Entries left as reading-only scope statements (no source probe possible without new semantics) | 5–15 | 60% |
| Every test passes against the base and asserts a non-Safe (refusal, Unknown or limit) outcome for its cited form | yes, except where a correction records an acceptance | 85% |

Entries I expect most likely to need a correction: E04 and E08 (the lexical
nested-assignment closure and several G readers accept some assignment-valued
and comma shapes), E16 (T's sizeof refusal is the fixed-type table, not
"childless"), E07 (G's condition desugaring evaluates `?:` in a condition, so
the refusal lies in value positions only), C13 (T's terminal noreturn form is
admitted), K08 (T refuses variadic definitions, G admits declared data tails).

## Corpus attribution

| Measure | Prediction | Confidence |
|---|---|---|
| Every unsupported leaf classified to at most one entry ID or `unattributed`, sums reconciled to 35,737 leaves and 4,298 Unknown functions | yes | 85% |
| Largest attributed family | the call-to-uncontracted-function family (8,847 leaves) stays `unattributed` (a contract or callee-closure question, analysis §2), the largest single bucket | 80% |
| Largest attributed card by leaves | CG10 (mixed places, never-bound and tracked-root premises, line identity) | 60% |
| Largest attributed card by functions freed alone (Phase 1–3) | CG10 first, CG11 second | 50% |
| T-route cards (CG01, CG09) free functions in the corpus arbiter | 0: the corpus arbiter runs the custody gateway (G) only | 90% |
| Functions freed alone by any single Phase 1–3 card | at most 150 for the best card; at most 400 for all Phase 1–3 cards together (upper bound, unsupported-coverage §5) | 70% |
| Functions freed by a pair of cards exceeds the best single card by at least 30% | yes | 50% |

## Scope

No analyzer, contract, provider or Lean change. No Lean module: the card adds
no semantics.
