# BASE-TYPE-READER-CENSUS — the reader beneath the member reader, measured

Wave 37 **measurement card**. It changes no analyzer rule, moves no verdict and
owns no re-pin. Base **`477b733d`**, the wave-37 opening commit. It discharges
residual **`BASE-TYPE-READER-UNMEASURED`**, which card
`C-GATEWAY-INDIRECT-CENSUS-2` (wave 36) named as "the honest next census".

## 1. Headline

| | |
|---|---|
| Corpus movement | **none** — instrumentation only; both verdict-bearing artifacts byte-identical to `docs/reports/joined-provenance-call-frames/artifacts/` **with the census feature ON and actively streaming** (§8) |
| Join | **171 / 171** units, 0 unassigned; 18,548 of **18,549** leaves explained (99.99%) |
| Wave-36's 78.5% | **reproduced exactly** — 10,514 of 13,396 `Unresolved` leaves carry no tag |
| …and **corrected** | only **7,115 (67.7%)** of those are base-type questions. **3,399 (32.3%) are chains whose base the reader DID commit**, which lost the tag at a member or a subscript — the wave-36 card's own question. The base-type share of the `Unresolved` row is **53.1%**, not 78.5% |
| Largest single cause | **`typedef struct TAG { … } t;`** — a typedef that *defines* its aggregate. **3,220 leaves (17.4%)** of the whole family, **223 (26.7%)** of the sole-blocked leaves. It is one `match` arm |
| Causes unattributed | **0**. Every `Opaque` answer and every whole-function refusal carries the rule that produced it |
| Best single reader change, first discount | **30 functions** — against wave-36's best access shape at 11 |
| …under the **second** discount (the member must then classify) | **12** |
| …of which constraint 5's premise holds for | **2** |
| Both readers together (base typedef + per-member record body) | **17**, premise holding for **5** |
| Union of **every** cause committed at once | **47** of 285 sole-blocked, 47 of 3,239 touched |
| Recommendation | **No base-type reader change survives the discount as a standalone wave-38 arbiter** (§7). It is, however, the *cheapest* route yet measured per function decided, and that is a different claim |

**The sentence that matters for scoping.** The wave-36 card found that the
family's leaf mass and its function reach are nearly disjoint, and warned wave
37 off cutting an arbiter on the 18,555. **That asymmetry reappears intact one
level down, inside the base-type reader.** The second-largest cause —
`FunctionRefused/LocalDeclaratorToken`, 2,147 leaves across 245 functions —
decides **zero**. `ArrayDeclarator`, 479 leaves across 201 functions, decides
**zero**. `NotDeclared/Unknown` touches **304** functions and decides **zero**.
A wave-38 card cut on the cause with the most leaves would repeat the error
wave 36 was commissioned to prevent, having merely moved one reader deeper.

## 2. What was instrumented, and the one-reader rule

The wave-36 card instrumented the *member* reader: `TypedAccessModel`'s record
classification, plus `read_record_body`'s whole-body refusal. It could not say
why a **base**'s declared type failed to commit, because nothing recorded that.

This card instruments exactly that, on the same principle and in the same place
— **the reader records the cause at the return that produces it**, and the
census carries the answer out rather than re-deriving it. That is not a style
preference. Card `C-GATEWAY-INDIRECT-CENSUS`'s abandoned source-text classifier
was a second reader that disagreed with the first by roughly a factor of 57 on
macro idioms; a second *type* reader would be the same mistake with a longer
fuse, because a disagreement about a declared type produces a plausible number
rather than an obviously wrong one.

Concretely, in `crates/qcore-c-gateway/src/typed_field_access.rs`:

* `classify_specifiers` and `declared_type` each gained a note-taking variant
  that the old signature now calls. Every `DeclaredType::Opaque` return writes
  its own cause. The classifier's answers are unchanged — the one structural
  edit is splitting `["union", _] | ["union"] | ["struct"]` into two arms that
  both still answer `Opaque`.
* `read_parameters`, `read_locals` and `read_declaration` write the rule that
  refused a whole function.
* `record_variable` writes the shadowing collision.
* `readable_declarator` — the predicate — was **replaced by**
  `unreadable_declarator_token`, of which it was the `is_none`. Same refusal
  set, one rule, and the census gets the offending token.
* `TypedAccessModel` gained a census-only **file-scope name table**, built by
  the *same* `is_declaration_candidate` / `statement_end` / `read_declaration`
  the function bodies use, so "the unit declares this name at file scope" is
  separable from "nothing declares it anywhere".

With the `indirect-census` feature off, `OpaqueNote` and `DeclCensus` are empty
newtypes and each `note_*!` macro drops its cause tokens before type checking:
no cause value is constructed, no map is allocated, no branch is taken. The
newtypes are deliberate rather than `()` — they keep `::default()` an ordinary
construction at every call site under both configurations, so neither
configuration needs a lint exemption to say the same thing.

**Zero causes came back `Unrecorded`.** The vocabulary carries an `Unrecorded`
variant for both the refusal and the `Opaque` answer precisely so that a gap in
this plumbing would be *published as a gap* rather than silently attributed to a
neighbouring cause; the stream contains none, across 87,976 statement records.

## 3. Reproducing the wave-36 figure before extending it, and correcting it

The join is card `C-GATEWAY-INDIRECT-CENSUS-2`'s, used verbatim — the
unit-ordinal bijection, the hardest-statement attribution, the distinct-record
reduction — so the two cards measure the same population and any difference is a
difference in what was instrumented.

**Reproduced exactly.** 13,396 leaves whose deciding chain ends at an
`Unresolved` member; **10,514 (78.5%)** of them carrying no record tag at all.
Both figures match that card's §7.1 to the unit.

**And then decomposed, which changes the reading.** `tag` is the tag the
**last** selection resolved against. A chain whose base the reader *did* commit
to can still lose the tag partway — at a member that is not a `struct TAG *`, or
at a subscript — and arrive at the same `tag: null`. The census now records
`tag_lost_step`, so the two are separable:

| of the 10,514 tagless `Unresolved` leaves | leaves | share |
|---|---:|---:|
| **the BASE was never committed** — this card's question | **7,115** | **67.7%** |
| base **committed**, tag lost at step 0 — a *member* question | 3,332 | 31.7% |
| base committed, tag lost at step 1 | 64 | 0.6% |
| base committed, tag lost at step 2 | 3 | 0.0% |

So the wave-36 card's sentence — "78.5% of unresolved members were never a
member question at all" — **overstates by about half again**. Just under a third
of that row is a member question, of exactly the kind that card owns. The
base-type reader's true share of the `Unresolved` population is **7,115 of
13,396, or 53.1%**.

This is recorded as a correction rather than an error found: the wave-36 card
read the only field it had, said in terms that it had not instrumented this
reader, and named the residual that asked for the measurement. The correction is
what the residual was for.

### 3.1 Why this card's family total is 18,549 and wave 36's was 18,555

Not a disagreement, and attributed exactly rather than plausibly. Counting the
family in each archived `diagnostics.json.gz` of the chain:

| archive | leaves | sole-blocked functions | sole-blocked leaves |
|---|---:|---:|---:|
| `c-gateway-constant-do-while` (wave 35) | 18,555 | 284 | 833 |
| `joined-provenance-cycles` (wave 35) | 18,555 | 284 | 833 |
| `w36-corpus-contracts-3` BEFORE | 18,555 | 284 | 833 |
| **`w36-corpus-contracts-3` AFTER** | **18,549** | **285** | **835** |
| `joined-provenance-call-frames` BEFORE / AFTER | 18,549 | 285 | 835 |

Wave 36's **first, declarative** arbiter `W36-CORPUS-CONTRACTS-3` decided six
leaves of this family and, in doing so, moved one function *into* the
sole-blocked set with two leaves. Wave 36's second arbiter moved this family by
**zero** on both sides. The census card measured the corpus as it stood before
its own wave's contract card landed, which is neither card's mistake and is
exactly the live instance of the registered residual
**`JPC-CENSUS-CONTRACT-SET-DEPENDENT`** — "a contract card invalidates the
census's published figures". That residual is hereby **confirmed rather than
hypothetical**, with the six leaves named as its first measured instance.

## 4. The cause distribution

Per **leaf**, each attributed its hardest statement and that statement's
hardest chain — a statement clears only if every chain in it clears, so the
census reports the one an arbiter would have to answer first.

| base-type cause | leaves | share |
|---|---:|---:|
| `Committed` — the base is fine; the refusal is elsewhere | 8,484 | 45.7% |
| **`DeclaredOpaque/TypedefDefinesAggregate`** | **3,220** | **17.4%** |
| `FunctionRefused/LocalDeclaratorToken` | 2,147 | 11.6% |
| `NoBaseName` — a call result or a parenthesized sub-expression | 1,986 | 10.7% |
| `DeclaredOpaque/TypedefDisagreeing` | 647 | 3.5% |
| `FunctionRefused/ParameterListToken` | 484 | 2.6% |
| `DeclaredOpaque/ArrayDeclarator` | 479 | 2.6% |
| `DeclaredOpaque/UnreadableSpecifiers` | 262 | 1.4% |
| `DeclaredOpaque/PointerToScalar` (`char *`, `int *`) | 255 | 1.4% |
| (no chain) | 188 | 1.0% |
| `NotDeclared/FileScopeDeclaration` | 161 | 0.9% |
| `DeclaredOpaque/PointerDepthAboveOne` | 113 | 0.6% |
| `NotDeclared/Unknown` | 69 | 0.4% |
| `DeclaredScalar` | 28 | 0.2% |
| `DeclaredOpaque/UnionSpecifier` | 24 | 0.1% |
| `DeclaredOpaque/ShadowedByDifferentType` | 1 | 0.0% |
| (unexplained leaf) | 1 | 0.0% |

The single unexplained leaf is `CENSUS-UNEXPLAINED-LEAF`, carried unchanged from
wave 36; this card did not chase it either, and says so.

**The dominant cause in one sentence.** `typedef struct rd_kafka_msg_s { … }
rd_kafka_msg_t;` records the target as an *aggregate the typedef defines*, and
`classify_specifiers` answers `Opaque` for it — a deliberate decision of card
`ABI-TYPEDEF-ADDRESS-OF-MEMBER`, which kept that answer byte-identical so its
own widening moved nothing. The **forward** spelling `typedef struct
rd_kafka_msg_s rd_kafka_msg_t;` records specifiers and *does* resolve, through
the recursive arm, to `Record("rd_kafka_msg_s")`. So the reader commits to a
typedef that forward-declares and refuses the one that defines — and librdkafka,
cJSON and cmetrics are written almost entirely in the second style. The top
payloads are `struct rd_kafka_topic_partition_s` (202 leaves),
`struct rd_kafka_topic_partition_list_s` (162), an **anonymous** `struct` (158),
`struct rd_kafka_share_ack_batches_s` (117), `struct rd_kafka_msg_s` (112).

## 5. The sole-blocked subset, by cause

285 functions whose only blocker family is this one, holding 835 leaves (4.5%).

| cause | leaves | share |
|---|---:|---:|
| `Committed` | 503 | 60.2% |
| **`DeclaredOpaque/TypedefDefinesAggregate`** | **223** | **26.7%** |
| `NoBaseName` | 31 | 3.7% |
| `DeclaredOpaque/TypedefDisagreeing` | 22 | 2.6% |
| `DeclaredOpaque/PointerDepthAboveOne` | 19 | 2.3% |
| `FunctionRefused/ParameterListToken` | 17 | 2.0% |
| `DeclaredOpaque/UnionSpecifier` | 5 | 0.6% |
| `DeclaredScalar` | 4 | 0.5% |
| `DeclaredOpaque/PointerToScalar` | 3 | 0.4% |
| everything else | 6 | 0.7% |

Two rows are worth reading against the whole family's. `Committed` is **higher**
here (60.2% against 45.7%): in the population that could actually be *decided*,
the base type is more often already fine, and the refusal is the member's or the
access rule's. And `FunctionRefused/LocalDeclaratorToken`, the family's second
largest cause at 2,147 leaves, collapses to **2** leaves here — the functions
whose declarations the reader refuses whole are, almost without exception,
functions carrying other blockers too.

Crossed with constraint 5's premise, over the sole-blocked leaves: of the 223
`TypedefDefinesAggregate` leaves, **173 (77.6%) sit on a base this function may
bind** to a resource identity and only 50 are never-bound. The premise the
whitelist needs is unavailable for three quarters of the best cause's own
sole-blocked mass.

## 6. The discounted three-way prediction, per cause

**Method**, stated so it can be checked. For a candidate cause C — a reader
change that committed the declared type of every base blocked by C — take every
function carrying a leaf one of whose statements has a chain blocked by C. It is
predicted **leaf-free** only if: its only blocker family is this one; *and*
every chain of every statement of every leaf it holds is either already
`Committed` or blocked by C; *and* no such statement carries an **uncontracted**
call. Otherwise `unmoved` or `exchanged` respectively. A call the analyzer
already holds a contract for is deliberately not an exchange, exactly as in wave
36.

This inherits wave 36's exchange proxy, unvalidated for the same reason
(`INDIRECT-EXCHANGE-PROXY-UNVALIDATED`), **and adds a second generosity of its
own, stated rather than buried**: committing a base's type yields a *tag*, not
an admission. The member must then classify and the access rule must then admit
it. So this CLEAR column is an upper bound on an upper bound. §6.2 removes the
second layer for the one cause where the census can.

### 6.1 Ranked by decided functions

| cause | leaves | funcs | **CLEAR** | exch | unmoved |
|---|---:|---:|---:|---:|---:|
| `DeclaredOpaque/TypedefDefinesAggregate` | 3,220 | 945 | **30** | 38 | 877 |
| `DeclaredOpaque/PointerDepthAboveOne` | 113 | 81 | **5** | 10 | 66 |
| `DeclaredOpaque/UnionSpecifier` | 24 | 18 | **3** | 1 | 14 |
| `NoBaseName` | 1,986 | 792 | **2** | 15 | 775 |
| `DeclaredOpaque/TypedefDisagreeing` | 647 | 155 | **2** | 3 | 150 |
| `FunctionRefused/ParameterListToken` | 484 | 95 | **2** | 4 | 89 |
| `DeclaredOpaque/PointerToScalar` | 255 | 158 | **1** | 1 | 156 |
| `NotDeclared/FileScopeDeclaration` | 161 | 101 | **1** | 1 | 99 |
| `DeclaredScalar` | 28 | 32 | **1** | 1 | 30 |
| `FunctionRefused/LocalDeclaratorToken` | 2,147 | 245 | **0** | 1 | 244 |
| `DeclaredOpaque/ArrayDeclarator` | 479 | 201 | **0** | 0 | 201 |
| `DeclaredOpaque/UnreadableSpecifiers` | 262 | 66 | **0** | 1 | 65 |
| `NotDeclared/Unknown` | 69 | 304 | **0** | 0 | 304 |
| `DeclaredOpaque/ShadowedByDifferentType` | 1 | 1 | **0** | 0 | 1 |

9 of 14 causes decide at least one function. The union of **every** cause's
cleared set is **47** functions — the ceiling if one card committed every base
type in the corpus at once, still assuming every member then classifies. Best
pair: `TypedefDefinesAggregate` + `NoBaseName` at **40**; every one of the top
ten pairs contains `TypedefDefinesAggregate`, so the pair ranking is one cause
plus a choice of partner.

**The asymmetry, one level down.** 2,147 leaves and 245 functions decide zero;
3,220 leaves and 945 functions decide thirty. Leaf mass predicts almost nothing
about reach *within the base-type reader*, just as it predicted almost nothing
about reach within the access gate.

### 6.2 The second discount, for the one cause that admits it

The `TypedefDefinesAggregate` cause carries the definition's **head** as its
payload, so for this cause — and only this one — the census can ask the next
question rather than assume it: if the base type committed, would the member
then classify, or would the refusal merely move one step?

Over the 3,220 leaves:

| | leaves | share |
|---|---:|---:|
| tag known and its record body **was read** — both readers agree | 1,624 | 50.4% |
| tag known, but its record body is **refused whole** (on `{`) | 1,338 | 41.6% |
| head is a bare `struct` — **anonymous**, no tag to resolve to | 158 | 4.9% |
| tag known, record body refused whole (on `(`) | 63 | 2.0% |
| head is `union` — must stay `Opaque` by construction | 33 | 1.0% |
| head is `enum` | 4 | 0.1% |

Applied to the **cleared set** — a function survives only if *every* one of its
blocked bases resolves to a `struct TAG` whose body the reader actually read:

> **30 → 12.** Thirteen fall to a head naming no struct tag; five fall to a
> record body refused whole. Of the surviving **12**, constraint 5's premise
> holds on every blocked base for **2**.

Pairing the change with a per-member record-body reader —
`RECORD-BODY-TOTAL-OR-REJECT`, the genuine defect wave 36 filed — recovers the
five: **17 functions**, premise holding for **5** and unavailable for **12**.

For comparison, on the same corpus and the same discount: wave 36's best access
shape decided **11**, its best access pair **17**, its record-body route alone
**2**, and all 403 access shapes admitted at once **45**.

## 7. Ranked recommendation for a wave-38 card

**No base-type reader change survives the discount as a standalone wave-38
arbiter.** The best decides **12 of 285** sole-blocked and 12 of 3,239 touched
functions; with the record reader beside it, **17**; every cause at once, 47.
That is the same order as the access-shape route the wave-36 card already
declined, reached independently — which is the second time this family has
answered the same way to a different question.

One thing *has* changed, and it is a cost argument rather than a reach argument,
so it is stated separately rather than allowed to colour the numbers above. Wave
36's 17-function pair required legislating two access shapes over a 403-shape
space, each needing its own soundness argument. This card's 17-function pair is
**two localized reader repairs**, one of them a single `match` arm. Per function
decided it is the cheapest route yet measured. Whether that justifies taking it
is the integrator's call, not this card's; what this card refuses to do is
present 17 of 3,239 as a reach result.

**Rank 1 — resolve the aggregate-defining typedef.**
`TypedefTarget::Aggregate("struct TAG")` → `DeclaredType::Record(TAG)` in
`classify_specifiers`, where it currently answers `Opaque`. 3,220 leaves, 945
functions touched, **30 → 12 decided** under both discounts.

*Soundness questions an arbiter must answer, and in which direction:*

* **Constraint 5b applies directly, and this is the case it was written for.**
  It warns that "a card that widens member typing far enough to put a
  projection-bound place on an admitted chain falsifies exactly this premise" —
  `TrackedBasesAreBound`, which
  `QCore.ReturnPositionEscape.an_admitted_chain_names_no_tracked_place` takes as
  an explicit hypothesis and does not prove. Committing `rd_kafka_msg_t *` to
  `RecordPointer("rd_kafka_msg_s")` is exactly that widening: chains that today
  end `Unresolved` would become typed places, and the premise says the
  projection tracks no place whose base the body does not bind. **The direction
  is admitting**, so invariant 1 requires the premise be re-established rather
  than re-assumed. This is the single largest soundness cost of the change and
  it is not shared by any other cause in §6.1.
* **Constraint 5's own premise is unavailable for 10 of the 12** (and 12 of the
  17 with the record reader): the base is a name the function may bind to a
  resource identity. Constraint 5 says the fail-closed pin that caught this once
  "was not weakened; keep it that way". An arbiter must therefore read
  `FunctionTypes::may_bind`, not `UntrackedBases::never_bound`
  (`CENSUS-TWO-PREMISE-READINGS`, still unregistered — see §10).
* **Constraint 5a is untouched in one direction and live in the other.** The
  change commits a *record* type, not a scalar value, so no query token becomes
  a computed value. But `POISON-UNMASK-ON-ADMISSION` applies as it does to every
  candidate: admitting a statement stops it poisoning bindings, and constraint
  10a records that sparing a binding can unmask a stale scalar fact a `NoOp`
  mutation left behind.
* **Constraint 5c is not engaged.** No CFG edge moves.
* **Two heads must stay `Opaque` and the arbiter must say so in its own words.**
  A `union` head (33 leaves) — a union member may overlap a pointer, which is
  why `classify_specifiers` refuses unions today and must keep refusing them.
  And an **anonymous** head, `typedef struct { … } t;` (158 leaves, 4.9% of the
  cause): there is no tag, so there is nothing for this change to commit to.
  Reaching those needs a synthetic tag keyed on the typedef name — a different
  and larger change, and a *new* identity the certificate recheck would have to
  reproduce. Residual `TYPEDEF-ANONYMOUS-AGGREGATE-UNREACHED`.
* **The recheck.** `ABI-TYPEDEF-ADDRESS-OF-MEMBER` kept this answer `Opaque`
  precisely so every premise reading `classify_specifiers` decided
  byte-identically to the tree before it. Changing the answer moves the scalar
  no-op rule, the borrowable / single-pointer / `void *` member sets, the chain
  walker and the declared types of locals **all at once**, and needs its own
  measured re-pin with each moved leaf attributed — not a global count.

**Rank 2 — land it with the per-member record-body reader**
(`RECORD-BODY-TOTAL-OR-REJECT`). Alone that reader's measured ceiling was 2
functions and wave 36 declined it on that ground. Beside Rank 1 it is worth
**five more** (12 → 17), because 41.6% of Rank 1's leaves resolve to a tag whose
body is refused whole on a `{`. Its soundness question is the narrower one: the
reader must mark the member declarations it cannot parse as *explicitly*
unreadable so they stay refused, rather than omitting them — omission is the
admitting direction.

**Not recommended, and why, so the wave need not re-derive it.**
`FunctionRefused/LocalDeclaratorToken` (2,147 leaves, 0 decided) and
`ArrayDeclarator` (479, 0) are pure leaf mass. `NoBaseName` (1,986 leaves, 2
decided) needs a whole function-return-type reader for a two-function return.
`NotDeclared/Unknown` touches 304 functions and decides zero.

## 8. Inertness and byte identity

Both halves are evidenced, not asserted.

**Compiled out.** Every addition is behind `#[cfg(feature = "indirect-census")]`
or behind a macro whose feature-off expansion drops its cause tokens. The
feature is enabled by **no** workspace member, so every standing gate step in §9
measures a tree in which this code does not exist. A second clippy pass **with**
the feature on is in this card's own gate so the instrumentation is linted.

**Does not perturb.** A pinned Cycle 19 corpus audit run by the instrumented
binary, with both census streams actively writing (71 MB and 178 MB), reproduces
the current baseline byte for byte:

| artifact | result | sha256 |
|---|---|---|
| **`diagnostics`** | **IDENTICAL** | `a7aa3b5f9ab193a0f08277ca069d1f28acda91a40dbcc12c29ac94a46bf5ac7b` |
| **`chunk-contracts`** | **IDENTICAL** | `c4f438e4ed3dab3c3b0e776af5c7c5181385353a72c67b6874ab4df5ba917980` |
| `source` | IDENTICAL | `ae38f6c93dfbe341b187077e9dd0074501000ea76d621feb481db2303930e529` |
| `source-effects` | differs | carries the checkout root |
| `contracts` | differs | carries the checkout root |

The last two have differed at every re-pin in this chain for that reason, which
this card states rather than hides. The audit's exit code (0) is captured to its
own file, never through a pipe. **This card owns no re-pin and adds no archive
to constraint 21's frozen list**; the corpus baseline remains
`docs/reports/joined-provenance-call-frames/artifacts/`.

## 9. Standing gate

Run from `scripts/gate.sh` beside this report, every exit captured to its own
file and never through a pipe. `df -h .` reported 162 GB free at the start —
above the 60 GB floor, so the full gate ran rather than crate-scoped checks.

| step | exit |
|---|---|
| `git submodule update --init --recursive` | **0** |
| `cargo test --workspace --no-fail-fast` | **0** |
| `cargo fmt --all -- --check` | **0** |
| `cargo clippy --all-targets -- -D warnings` | **0** |
| `cargo clippy -p qcore-c-gateway --features indirect-census --all-targets -- -D warnings` | **0** |
| `cargo run -p qcore-compiler --example reevaluation_probe` | **0** |
| `cargo run -p qcore-compiler --example reevaluation_followup` | **0** |
| `python3 scripts/check_lean_audits.py` | **0** |

Workspace tests and Lean audit counts are in `gate.json` beside this file. The
fifth row is an extra step this card adds to its own gate, not part of the
standing list.

## 10. Named follow-ons

Registered in `docs/reviews/r0_3/acceptance_matrix.json`'s `named_followups`,
unlike wave 36's nine — see §11.

* **`BASE-TYPE-READER-UNMEASURED` — DISCHARGED by this card.** Registered here
  *with* its discharge, because it was never registered at all (§11).

* **`TYPEDEF-ANONYMOUS-AGGREGATE-UNREACHED`** — 158 leaves (4.9% of the dominant
  cause) carry the head `struct` with no tag: `typedef struct { … } t;`. Rank 1
  cannot reach them, because there is no tag to commit to. Reaching them needs a
  synthetic tag keyed on the typedef name, which is a new record identity the
  certificate recheck must reproduce. **Discharge:** a card that either lands the
  synthetic tag with its own re-pin, or records the measured decision that 158
  leaves do not justify a new identity.

* **`BASE-CAUSE-SECOND-DISCOUNT-PARTIAL`** — §6.2's second discount (does the
  member then classify?) could be applied to **one** of fourteen causes, because
  only `TypedefDefinesAggregate` carries a payload naming the record. Every other
  cause's CLEAR column in §6.1 is therefore still an upper bound on an upper
  bound, and the **47-function union is not comparable** with the 12 and the 17.
  **Discharge:** stream the tag a committed base *would* have yielded for the
  other causes, or state per cause that the second question cannot be asked.

* **`BASE-TYPE-CENSUS-FILE-SCOPE-APPROXIMATE`** — the file-scope name table is
  built by the same declaration reader, so a file-scope declaration that reader
  will not read is absent from it and the census reports the name
  `NotDeclared/Unknown` rather than `FileScopeDeclaration`. The split (161 vs 69
  leaves) is therefore approximate **in the direction of under-reporting
  file-scope**. Not repaired: a second reader here would be the very mistake §2
  exists to avoid. **Discharge:** measure the gap against a Clang-derived
  file-scope set, or state the axis as bounded.

* **`W36-CENSUS-FIGURES-PRE-CONTRACT-CARD`** — card
  `C-GATEWAY-INDIRECT-CENSUS-2`'s published 18,555 / 284 / 833 describe the
  corpus **before its own wave's contract arbiter** `W36-CORPUS-CONTRACTS-3`
  landed; the current figures are 18,549 / 285 / 835 (§3.1). The card is not
  wrong and its archive must not be edited (constraint 21). This is the first
  *measured* instance of the registered residual
  `JPC-CENSUS-CONTRACT-SET-DEPENDENT`. **Discharge:** publish the baseline a
  census was taken against beside every census figure, as this card does in §3.1.

* **`BASE-TYPE-READER-CENSUS-FEATURE-UNBUILT`** — this card extends the
  `indirect-census` feature, which is still built by no CI job. It is now
  carrying two cards' instrumentation and will rot twice as expensively.
  Wave 36 raised this as `INDIRECT-CENSUS-FEATURE-UNBUILT` and left it to the
  integrator; a second card has now paid into it. **Discharge:** a CI job
  building `-p qcore-c-gateway --features indirect-census`, or a recorded
  decision that the instrumentation is disposable.

Carried unchanged from wave 36 and **not** re-derived here:
`CENSUS-UNEXPLAINED-LEAF` (still exactly one leaf),
`INDIRECT-EXCHANGE-PROXY-UNVALIDATED` (§6 inherits the proxy),
`CENSUS-TWO-PREMISE-READINGS`, `POISON-UNMASK-ON-ADMISSION`,
`RECORD-BODY-TOTAL-OR-REJECT` (§6.2 gives it a second, larger measured value:
five functions *beside* Rank 1, against two alone),
`CENSUS-DECLARATION-WINDOW-APPROXIMATE`.

## 11. For the integrator

### 11.1 Applied by this card

* `docs/development/increments/wave-37.md` — one paragraph, prepended. The file
  existed; it was not created.
* `docs/reviews/r0_3/acceptance_matrix.json` — this card's six residuals
  registered in `named_followups`, plus `BASE-TYPE-READER-UNMEASURED` registered
  *with its discharge* — `named_followups` **171 → 177**. Regenerated afterwards
  with
  `python3 docs/reports/remaining-d02/build_matrix.py . --gate docs/reports/wave35-close/gate.json`
  (the repo root is a positional argument; wave 36 has no close directory yet, so
  wave 35's gate is the one that preserves provenance, exactly as
  `C-GATEWAY-INDIRECT-CENSUS-2` used it). The regeneration's **only** deletion is
  the `dimensions.named_followups` count itself: 37 insertions, 1 deletion,
  nothing else moved.
* Nothing else. No re-pin, no headline ledger number moved, no archive added to
  constraint 21's frozen list, no corpus baseline changed.

### 11.2 Not applied — needs your decision

* **Wave 36's nine residuals are still unregistered.** `BASE-TYPE-READER-
  UNMEASURED`, `RECORD-BODY-TOTAL-OR-REJECT`, `CENSUS-TWO-PREMISE-READINGS`,
  `INDIRECT-CENSUS-FEATURE-UNBUILT`, `POISON-UNMASK-ON-ADMISSION`,
  `INDIRECT-EXCHANGE-PROXY-UNVALIDATED`, `CENSUS-UNEXPLAINED-LEAF`,
  `LOAD-SENSITIVE-ROW-UNNAMED` and `CENSUS-DECLARATION-WINDOW-APPROXIMATE` are
  named in that card's report and in **none** of the 171 `named_followups`
  entries — checked by id, not by eye. I registered only the one this card
  discharges, because registering another card's eight findings is a judgement
  about the tracker rather than about this card. The two a wave-38 gateway card
  will trip over are `CENSUS-TWO-PREMISE-READINGS` and
  `RECORD-BODY-TOTAL-OR-REJECT`, and §6.2 raises the latter's measured value.

* **`RECORD-BODY-TOTAL-OR-REJECT`'s disposition should be revisited.** Wave 36
  declined it on a measured ceiling of two functions. That number is correct and
  this card does not dispute it — but it is the *standalone* number. Beside Rank
  1 the same repair is worth five more functions, because 41.6% of Rank 1's
  leaves land on a record body refused whole. A repair worth 2 alone and 5 in
  company is a different proposition from one worth 2.

### 11.3 No standing constraint is proposed

This card lands no "when touching X, do not Y" rule. Two existing constraints
are read in ways worth noting and neither needs amending on this card's account:

* **Constraint 5b** is confirmed live and given a size. It warns that widening
  member typing falsifies `TrackedBasesAreBound`; §7's Rank 1 is precisely that
  widening, and this card measures what it would buy (12 functions, or 17 with
  Rank 2) so the warning can be weighed rather than merely heeded.
* **Constraint 5** is again evidence *for* itself: the premise is unavailable
  for 10 of the 12 functions the best reader change would decide, and for 173 of
  the 223 sole-blocked leaves of the dominant cause. Nothing here suggests
  weakening it.

## 12. Reproduction

Scripts are archived under `scripts/` beside this report and were run from
`measure/base-type-reader-census/` (worktree-private per `CLAUDE.md`; the raw
streams are 71 MB and 178 MB and are **not** committed).

```sh
# 1. the instrumented binary -- the feature is enabled here and nowhere else
cargo build --release -p qcore-compiler \
    --example cycle19_fluent_bit_libraries_audit \
    --features qcore-c-gateway/indirect-census

# 2. one pinned Cycle 19 corpus audit, both census streams beside it
sh run_audit.sh final target/release/examples/cycle19_fluent_bit_libraries_audit

# 3. the inertness assertion
python3 byte_identity.py runs/final

# 4. the census: reproduction, decomposition, causes, predictions
python3 base_causes.py runs/final out

# 5. the second discount, and the two readers together
python3 discount.py runs/final out
python3 combined.py runs/final

# 6. the baseline attribution of the 18,555 -> 18,549 delta
python3 family_sizes.py
```

The pin, compile database, ChunkIO package and environment are exactly those
waves 33–36 used; the transport stays `HistoricalStdin` (constraint 11) and is
the audit example's own pinned-evidence route, not selected here.
`QCORE_INDIRECT_CENSUS` and `QCORE_INDIRECT_CENSUS_RECORDS` name the two census
streams; with neither set, and with the feature off, nothing is written and
nothing runs.
