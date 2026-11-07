# BUILTIN-EXPECT-NULL-TEST: prediction (frozen before the semantic change)

Base `82ec0e80b` (integrate/r6: origin/main `532e03c97` plus the records
commit registering this card). Measurement side
`baseline:7b726587bdf6a8213f451fa1c78297bb5e476ef7`: corpus 4,834 rows =
1,054 Safe / 3,769 Unknown / 11 provisional Unsafe; system 448 = 101 / 343 /
4; slice 404 = 58 Safe / 4 Pending / 341 Unknown / 1 provisional Unsafe.

## What the census found (committed before this file)

[README.md §1](README.md#1-census) and [census.json.gz](census.json.gz): 2,905
`__builtin_expect` sites in the analysed function bodies of the current run
(fluent-bit `046accf6`, librdkafka 2.15.0 and Monkey), the same 2,905 in the
pinned corpus (`66910c10`, same library sources), none in the system and
slice selections. Every hint is `0`, `1`, `(0)` or `(1)`. Rule 5h's Boolean
reading (`legacy_condition_desugar.rs`, card W44-BRANCH-CONDITION-CALLS)
already reads each of them as its operand, and the wrapped-versus-bare census
harness ([README.md §2](README.md#2-wrapped-against-bare)) measures what the
wrapper changes against the bare test on every unit with a site.

In particular snappy.c:1706 `if (unlikely(!outbuf))` is read today (no
diagnostic at the line); the failure arm's leaf at :1712 is reproduced with
the bare `if (!outbuf)` in the same two-pass loop, and it also appears at the
pre-allocation return (:1701 in the unit, the fixture's `uof <= 0` return).
It rests on the loop generation join (`loop re-acquisition` at :1705), card
LOOP-REACQUISITION-LEAK-ACCUSATION's mechanism, not on the wrapper.

## The semantic change

Rule 5h amended: the hint may be any one integer literal (C11 6.4.4.1: hex,
octal, `u`/`l`/`ll` suffixes), not only a digit sequence; every other hint
keeps the refusal. No census site has such a hint, so the change admits no
site of any route. `QCore.BuiltinExpectCondition` is new (the wrapper's
reading had no Lean model).

## Predicted measurement

* **Corpus, system, slice: no status move, no diagnostic change** (no hint
  outside `0`/`1`/`(0)`/`(1)` on any route; system and slice have no site).
* **No Safe gain, no Safe loss.** No new Safe row, so no per-function
  justification is owed.
* **Null property:** unchanged on all three routes.
* **Current run (046accf6, user's `quirin-audit current`):** unchanged by this
  card: no callee leaves the uncontracted list, no function leaves the "only
  uncontracted" set, no unit status changes. `snappy.c` stays provisional
  Unsafe from this card alone: `rd_kafka_snappy_java_uncompress`'s seven
  leaves (:1648, :1660, :1680, :1694, :1701, :1712, :1721) do not move here.
  With LOOP-REACQUISITION-LEAK-ACCUSATION landed, :1712 belongs to that card's
  measurement with the other six (its mechanism is the generation join), and
  the unit may then stay Unsafe or become Unknown on its other sites; never
  Safe from either card (the function also carries refusals at :1652, :1671,
  :1707 and :1721).
* **Lean:** one new audited module, 11 theorems.
* **Ledger:** BUILTIN-EXPECT-NULL-TEST retired with the :1712 leaf re-homed
  to LOOP-REACQUISITION-LEAK-ACCUSATION; `BUILTIN-EXPECT-OPERAND-CONVERSION`
  registered for the conversion-to-`long` premise.

```measure-prediction
route corpus
none
route system
none
route slice
none
```
