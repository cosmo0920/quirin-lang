# Cycle-19 custody: the B and W card series (wave-42 archive)

> [!NOTE]
> **Current status (executed wave-42 queue).** Both the `B01`–`B04` expression-position call series and the `W42` priority slices were **executed and integrated** in wave 42. For current corpus figures (1,119+ Safe functions vs. the 2026-09-20 baseline below) see [`docs/STATUS.md`](../../STATUS.md); for active C-gateway coverage queues see [`clang-gap-cards`](../clang-gap-cards/README.md) and [`unsupported-coverage`](../unsupported-coverage/README.md).

## 1. Execution outcome summary

| Card / Series | Scope | Standing and evidence |
|---|---|---|
| **B01–B04** (`RETURN-EXPRESSION`, `BRANCH-CONDITION`, `LOOP-AND-SWITCH`, `NESTED-CALL-ARGUMENT`) | Unconditional externally declared contracted calls in return, branch/loop/switch controlling, and 1-level resource-neutral nested argument positions | **Completed** — [B-series integration](../../reports/b-series-integration/README.md) (gateway constraints 5d–5f; later widened for call-bearing Boolean trees in `w44-branch-condition-calls`, constraint 5h). |
| **W42-INDIRECT-ACCESS** | Bounded scalar indirect-access slice alongside `B02` | **Completed** — [W42 priority integration](../../reports/w42-priority-integration/README.md) (expanded in waves 43–48 via typed scalar/traversal/chain rules and `CG10`–`CG14`). |
| **W42-MONKEY-CALLBACK-COVERAGE** | Total callback inventory accounting for the forced-Libevent profile | **Accounting slice completed** — [callback coverage](../../reports/w42-monkey-callback-coverage/README.md) & [composition](../../reports/archive/waves-40-47/w43-monkey-callback-composition/README.md) (3/3 callbacks accounted with explicit `Unknown` reasons; Cycle-18 production callback composition remains open under `CYCLE18-*`). |
| **W42-UNPROJECTED-DEFINITIONS** | Reconcile 4,921 raw definitions vs. 4,834 projected functions across 24 TUs | **Completed** — [unprojected census](../../reports/w42-unprojected-definitions/README.md) (all 87 absent entries proved to be pinned-build conditional exclusions; 0 enumerator false positives, 0 projection defects). |
| **W42-CHUNK-LIFECYCLE-CERT** | Source/build-bound chunk lifecycle inventory | **Bounded source profile landed; full lifecycle `Safe` open** — [W43 chunk source profile](../../reports/w43-chunk-source-profile/README.md) (0 Safe / 96 Unknown over 96 `flb_input_chunk.c` and `cio_chunk.c` functions; nine composition legs remain open). |

---

## 2. Key methodological finding retained from the wave-42 census

The [blocker census script](blocker_census.py) (`blocker-census.txt`, measured on `w37-corpus-contracts-4` before `B01`–`B04`) established two structural principles that govern subsequent C-gateway card sizing:

1. **Rank cards by *sole-blocked functions* and *co-blocker pairs*, never by leaf counts.** At that checkpoint (4,537 `Unknown` functions), `indirect memory access` had 17,775 leaves across 3,157 functions (285 sole-blocked), whereas `return expression` had only 1,452 leaves yet **471 sole-blocked functions**—making `B01` the highest-yield single-blocker slice, while `B02` (branch conditions, 44 sole) had to pair with indirect access (196 co-blocked) to move verdicts.
2. **`unknown_alias` is a downstream provenance symptom, not a standalone syntax target.** Although 1,366 functions carried `unknown_alias` leaves (4,114 leaves), it blocked only 3 functions alone (re-measured at 8 in `W47-ALIAS-RESIDUALS-CENSUS`), clearing only alongside indirect-access and callee-contract resolution.

*(The raw 2026-09-20 leaf tables in [`blocker-census.txt`](blocker-census.txt) remain frozen as the pre-`B01` checkpoint.)*
