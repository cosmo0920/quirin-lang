# LOOP-REACQUISITION-LEAK-ACCUSATION: prediction (frozen before the semantic change)

Card LOOP-REACQUISITION-LEAK-ACCUSATION (round 26, card-high; USER DECISION
2026-11-07, verbatim: "Register four as implementation cards"). Base
`82ec0e80b` (integrate/r6); measurement side
`baseline:7b726587bdf6a8213f451fa1c78297bb5e476ef7`: corpus 1,054 Safe /
3,769 Unknown / 11 provisional Unsafe, system 101 / 343 / 4, slice 58 Safe /
341 Unknown / 4 Pending / 1 provisional Unsafe (counted from the baseline
reports). Committed after the census (`0fb3c63ee` + the census evidence in
`evidence/before/`) and before the semantic change. New rule id 42 (40
reserved, 41 SCALAR-STATEMENT-FORMS; every numeric and lettered id checked
on the base and every local branch).

## 1. Candidate rows

A row can move only if it has a `leak` whose acquisition line also carries
`loop re-acquisition: an earlier generation that may be live is still held`
in the same function (`scripts/candidates.py`). At the baseline:

| route | row | baseline | leaks at the refused acquisition |
|---|---|---|---|
| corpus | `librdkafka::snappy.c::rd_kafka_snappy_java_uncompress` | Unsafe | 7 (:1648, :1660, :1680, :1694, :1701, :1712, :1721; acquired :1705) |
| corpus | `librdkafka::rdkafka_sasl_scram.c::rd_kafka_sasl_safe_string` | Unsafe | 1 (:249; acquired :243) |
| system | none | | |
| slice | none | | |

The user's `current` run (046accf6) holds the same two rows, and the census
run on this base (two-unit selection, `evidence/before/rows.json`)
reproduces them byte for byte in the leaf list: snappy 7 leaks, scram 1,
both units Unsafe.

## 2. What the census shows, and the prediction it implies

At the settled exits (`evidence/before/table.json`), for snappy:

* :1680 and :1694: the current generation is `RELEASED` (status 2), :1721
  `ESCAPED` (status 4), :1712 absent (status 0: the `unlikely(!outbuf)` null
  arm is refined at this base). Only the previous and older generations
  have a witness path to these exits: **generation joins**, withdrawn.
* :1648, :1660, :1701: the current generation has its own witness path
  (acquisition :1705, the pass loop's back edge, the pass-2 re-check or the
  `pass == 1` arm, the exit) passing **no** re-acquisition: not a join
  (PATH-INSENSITIVE-SCALAR-CORRELATION / LOOP-GENERATION-COUNTER-CORRELATION),
  kept.
* scram :249: the current generation (bound to `d` only, the chained
  assignment's inner destination `safe` unbound) has its own path: not a
  join, kept (CHAINED-ASSIGNMENT-INNER-DESTINATION).

So the census refutes the loop-join attribution for snappy :1648, :1660 and
:1701 and confirms it for :1680, :1694, :1721 and :1712. The rule is scoped
to what it confirms. Predicted:

* **corpus**: no status move. `rd_kafka_snappy_java_uncompress` stays
  provisional Unsafe with 3 leak leaves (:1648, :1660, :1701), losing 4
  (:1680, :1694, :1712, :1721); its refusal at :1705 and every other leaf
  unchanged. `rd_kafka_sasl_safe_string` byte-identical.
* **system**, **slice**: byte-identical (no candidate row).
* **No Safe move** on any route; no Safe → non-Safe.
* **Null property**: identical on all three routes.
* **current run (046accf6)**: the same: snappy 7 → 3 leaves, still
  provisional Unsafe; scram unchanged; no unit status change; totals 10 Safe
  / 0 Pending / 8 provisional Unsafe / 164 Unknown / 4 data-only unchanged.
  snappy.c does **not** leave Unsafe from this rule (acceptance 3 is not
  met, by the census: the three remaining leaves are not generation joins).

```measure-prediction
route corpus
none
route system
none
route slice
none
```

## 3. A census finding outside the rule

The settled exits reproduce **none** of the eight leaks: at every reporting
exit of both functions every generation, current included, is `LIVE` with an
unresolved effect (`settled: false` on every witness). All eight are
reported on intermediate iterates of the work list, before the refusal's
unresolved effect reaches the exit (the scheduling sensitivity C02-P-ALIAS's
`-LEAK-ORDER` residual records). A rule that publishes a leak only where the
settled exit still reports it would withdraw all eight (snappy.c and
rdkafka_sasl_scram.c to Unknown); that is a different rule with corpus-wide
reach (the 271-leaf pin history), not this card's, and becomes a follow-up.

## 4. Lean and ledger

One new module, `QCore.LoopReacquisitionLeak` (its audit
`proof/AuditLoopReacquisitionLeak.lean`). Ledger: LOOP-REACQUISITION-LEAK-ACCUSATION
retired (scoped by the census to the sites it confirms);
LOOP-GENERATION-COUNTER-CORRELATION and BUILTIN-EXPECT-NULL-TEST amended
with the census; one follow-up registered for §3.

## 5. Amendment (after the semantic commit): scope update for :1712

Added after `5de709b79`; §§1-4 and the `measure-prediction` block above are
unchanged. The coordinator moved snappy.c:1712 from BUILTIN-EXPECT-NULL-TEST
to this card (that card found its premise stale: rule 5h already reads
`__builtin_expect(E, K)` as `E`, and `-D__builtin_expect(e,c)=(e)` leaves all
53 snappy.c functions identical). Nothing in the prediction changes: §2
already counts :1712 among the four confirmed generation joins the rule
withdraws, on this card's own census (the failure arm removes the current
generation, status 0). All seven snappy leaves are covered: four withdrawn
(:1680, :1694, :1712, :1721), three kept as current-generation witnesses
(:1648, :1660, :1701). One fixture was added
(`refused-earlier-generation-expect`, the `__builtin_expect(!p, 0)` failure
arm); BUILTIN-EXPECT-NULL-TEST is no longer amended by this card (its own
records cover it), and §4's follow-up for §3 is an amendment of
C02-P-ALIAS-O rather than a new id.
