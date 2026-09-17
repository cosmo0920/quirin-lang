# Wave 36 increment records

[Development table of contents](../../../DEVELOPMENT.md) · [History index](README.md)

> Preserved checkpoint narrative. Counts, platform observations and old
> instructions describe their recorded dates. Current agent rules and headline
> status live in [AGENTS.md](../../../AGENTS.md); each card's report owns its evidence.

`STABILITY-REEVAL` #2 (2026-09-18), the cadence's **second firing**, audited
waves 34–35 by `CLAUDE.md`'s four steps and closed with **31 registered
residuals and zero retirements** (`named_followups` 121 → **152**), the
zero deliberate: a retirement is what this firing found doing the damage. Over
firing #1's 27 residuals the tally is **15 discharged · 3 partial · 10 open**,
and the pattern is that every residual needing a **test, a control or a
mechanism** is open while almost every one needing a **sentence** is
discharged. Two findings are about the mechanisms the project relies on rather
than any card. `AGENTS.md`'s Cycle 19 raw corpus row carries "**44** corpus
functions" and "**21** sites" where card `JOINED-PROVENANCE-CYCLES` §11 said
"apply in these words" and the words were **40** and **14**; the 21 is the
**superseded `a300a39d` census** that card's own §2 exists to stop anyone
quoting, and the string 44 occurs nowhere in its report or logs
(`JPC-LEDGER-44-21-STALE`, measured independently by two sub-audits). And
wave 35's `reconciliation.json` records that very item **`applied`**, naming
the row as the place and asserting the 40 and the 14 "is present" — which they
are not. The close rule already demands an `applied` verdict say *where*, and
this one does; nothing checks what it **asserts** against what is **there**, and
a scan of all 103 wave-34/35 verdicts shows the defect invisible to any check
that does not read the named location (`RECONCILIATION-APPLIED-ASSERTS-UNVERIFIED`).
Three Lean statements in one module are described by more than they prove:
`an_off_cycle_node_does_not_return`'s only hypothesis asserts **whole-graph
acyclicity**, so it is vacuous exactly where its card operates, while the
shipped `acyclic_sites` tests per-node off-cycle-ness inside a cyclic graph and
the property claimed ("runs at most once per call") is about execution, which
`Steps` cannot express; `fixed_point_is_sound` is described by the acyclicity
hypothesis it **lacks** rather than the `TransferSound` premise it **carries**,
a name that appears in no report, record or ledger; and `acyclic_solution_is_unique`
holds only among solutions agreeing at the entry. Constraint **5c** is false
about the shipped tree for `for (;;)`: `condition.is_empty()` has dropped that
header's **exit** edge since Cycle 19 (`490f5151`), with neither the Lean
statement nor the re-pin 5c says such a step needs — exact, not unsound, but the
neighbour is unowned (`FOREVER-FOR-EXIT-EDGE-UNOWNED`). The sentence 5c records
the integrator as having caught, because landing it "would have put a statement
contradicting 10b into this file", is landed verbatim in the committed script
5c sends readers to. Wave 35's own record shows a **gate exit swallowed by a
pipe** — a merge reached origin with the Python suite red — "the second time
this wave". Measured **clean** and reported as such: both `C-GATEWAY-RETURN-CALL-W4`
arms go red under mutation and both mutations produce a false `Safe`, so the
premises are load-bearing; all three unread `AGENTS.md` merge resolutions
dropped nothing; **zero** weakened tests across waves 34–35, with three pinned
expectations moved and all three disclosed in both report and commit message;
and an `AbstractPointer` with empty targets, `unknown` and `may_be_null` false
provably cannot reach `state.pointers`, by exhaustive enumeration of fourteen
write sites — though that invariant is asserted nowhere and the struct's derived
`Default` **is** the forbidden triple. Forward: the 64-round fixed-point bound
is driven by **no test**, counted by **no census**, and its decline returns the
**same value** as "nothing to derive", so the rate is unmeasurable without a code
change; `release_site` has **seven** refusal disjuncts and three are
instrumented; and both wave-36 arbiters are chartered against numbers that exist
only outside the tree. `docs/reports/stability-reeval-2/README.md`.
