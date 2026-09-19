# Cycle-19 custody: the B and W card series

Proposed card definitions, 2026-09-20. **Not dispatched work and not a coverage claim.** Every
figure below is computed by `blocker_census.py` in this directory and printed in
`blocker-census.txt`; re-run it at each arbiter rather than carrying the numbers forward, because
they are properties of the pinned corpus at the baseline `docs/reports/w37-corpus-contracts-4/artifacts/`.

Execution update, later 2026-09-20: the user explicitly dispatched B01–B04
in parallel. Their bounded implementation, prediction misses, combined arbiter
and validation are recorded in [B-series integration](../../reports/b-series-integration/README.md).
The figures and ordering proposal below remain the pre-dispatch checkpoint;
the W-series cards and registered residuals are not closed by that dispatch.

## What the census changes about card sizing

Of the **4,537** Unknown functions, only **990** have a single distinct blocker and **1,030** have
exactly two; the rest need three or more refusal classes to move at all. A reason's *leaf* count and
its *sole-blocker* count therefore rank differently, and sizing a card by leaves is how three
consecutive arbiters produced zero unit movement:

| Refusal | Leaves | Functions showing it | Functions it blocks **alone** |
|---|---:|---:|---:|
| indirect memory access outside the resource projection | 17,775 | 3,157 | 285 |
| call to uncontracted function | 11,352 | 2,445 | 152 |
| unmodeled call/aggregate/indirect in branch condition | 8,755 | 2,426 | 44 |
| unresolved resource identity (`unknown_alias`) | 4,114 | 1,366 | 3 |
| braced initializer, expression or declaration not modeled | 2,083 | 986 | 6 |
| unsupported assignment expression | 1,683 | 589 | 0 |
| unmodeled … in **return expression** | 1,452 | 1,120 | **471** |
| unmodeled … in for-loop condition | 927 | 602 | 0 |

Two consequences carry into every card below. **`unknown_alias` is not a modelling target**: it
blocks 3 functions alone out of 1,366 that show it, because it is the identity lattice failing to
resolve a name rather than a construct the projection refuses; it clears when provenance resolves
(constraint 10b), not when a model is added for `ptr`. And the **dominant blocker pairs decide the
wave's shape** — indirect+branch (196 functions), uncontracted+indirect (194), indirect+identity
(77), indirect+return (62), branch+return (61) — so a card that lands one half of a pair moves
neither function and must say so in its prediction rather than discover it in the re-pin.

## B series — the unmodeled-call family, one rule at several positions

`legacy_c` refuses a call, aggregate access or indirect access appearing in a position whose
evaluation it does not model. The family is one rule; each position is a card so that each carries
its own measured prediction and its own Lean statement.

| Card | Position | Leaves | Functions | Sole | Why this size |
|---|---|---:|---:|---:|---|
| **B01-RETURN-EXPRESSION** | return expression | 1,452 | 1,120 | **471** | the largest single-blocker class in the corpus, and eighth by leaves — the cheapest verdict movement available |
| **B02-BRANCH-CONDITION** | branch condition | 8,755 | 2,426 | 44 | alone it moves little; paired with indirect access it is the corpus's biggest pair (196) |
| **B03-LOOP-AND-SWITCH** | for-loop, loop, switch, do-while conditions | 1,372 | 924 | 4 | completes the family at low cost; `switch` is 96 leaves over 90 functions |
| **B04-NESTED-CALL-ARGUMENT** | call nested in a modeled call's arguments | 22 | 13 | 4 | small by count and already recorded as *the commonest way a decided leaf is exchanged rather than discharged* (`NESTED-CALL-IN-ARGUMENT-POSITION`) |

Every B card: admit the position in the resource projection **in the refusing direction first** —
a newly modelled evaluation must not turn a refusal into an admission without the card measuring
and naming what it admits (invariant 1, and `POISON-UNMASK-ON-ADMISSION`'s shape, where admitting
an access moves counts through an edge becoming infeasible rather than through the access being
modelled). No new `Q` code: these are gateway-internal `Unsupported` descriptions, outside `Q`
space. Rust plus audited Lean for the evaluation rule; a predicted function movement published
**before** the AFTER run, as `W37-CORPUS-CONTRACTS-4` did; mutations shipped.

## W series — the classes that are not the call family

| Card | Subject | Sole | Notes |
|---|---|---:|---|
| **W42-INDIRECT-ACCESS** | the destination-side lexical gate (`->`, `.`, `[` in an assignment destination) | 285 | 17,775 leaves, a third of all refusals; `aggregate_store.rs` narrowed exactly one shape and the rest of the family was measured out as an arbiter target in wave 37 — this card is worth cutting **only paired with B02**, and its brief must say which pair it is buying |
| **W42-W4-STATIC-SUMMARIES** | uncontracted calls at the `static`-helper frontier | 152 | the declaration lever is spent (`W37-COVER-FRONTIER-IS-STATIC`); `CTRACES-FORMATTER-CHAIN` is its measurable test, and `C03-NFR-REACH` records recursion reach closed at 1 of 46 |
| **W42-CORPUS-CONTRACTS-5** | the arbiter that re-pins after the above | — | BEFORE must reproduce the current baseline byte for byte; every delta attributed leaf by leaf; the leak pin is a finding to attribute, never to absorb (constraint 7) |

## Ordering, and what each card must predict

1. **B01** first: it is the only card whose sole-blocker count justifies it on its own.
2. **B02 + W42-INDIRECT-ACCESS** as a pair, or neither — the census says each alone moves tens of
   functions while together they reach the 196-function pair plus their own 44 and 285.
3. **B03**, **B04** and **W42-W4-STATIC-SUMMARIES** are independent and may run beside either.
4. **W42-CORPUS-CONTRACTS-5** last, as the wave's arbiter.

Each card states, before it measures the AFTER: how many functions it expects to move, how many it
expects to *exchange* (a decided leaf becoming a different refusal), and which entries it would
retire. A card that moves no function is a legitimate result when its prediction said so; a card
that discovers this in its re-pin has measured nothing.

## Entries these cards would discharge

`NESTED-CALL-IN-ARGUMENT-POSITION`, `RETURN-POSITION-SUBSHAPE-B`, `RETURN-CALL-INCOMPLETE-LINE-455`,
`RETURN-POSITION-TYPED-MEMBER-REACH`, `RETURN-POSITION-CONDITIONAL`, `W37-COVER-FRONTIER-IS-STATIC`,
`CTRACES-FORMATTER-CHAIN`, `W36-COVER-HIDDEN-BLOCKER` (whose discharge is this census),
`TYPEDEF-ANONYMOUS-AGGREGATE-UNREACHED` and `LOADER-ABI-INLINE-FNPTR-DECLARATOR` if a card takes the
reader repairs beside the projection work. None is retired by planning; each is retired only by a
card whose condition is met word for word.
