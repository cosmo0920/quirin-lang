# C-GATEWAY-INDIRECT-CENSUS-2 — the indirect-access refusal family, measured from the analyzer's own projection

Wave 36 **measurement card**. It changes no analyzer rule, moves no verdict and
owns no re-pin. Base **`60958bf0`**, the wave-36 opening commit. Its deliverable
is a census that scopes a wave-37 arbiter honestly, and its central result is a
**negative** one, reported as a result rather than written around.

## 1. Headline

`indirect memory access is outside the resource projection` is the corpus's
largest refusal family: **18,555 leaves across 3,239 functions in 171 units**.
The census explains **18,554 of them (99.99%)** from the analyzer's own token
view, declaration facts and never-bound base set, at the moment each refusal was
decided.

| | |
|---|---|
| Leaves explained | **18,554 / 18,555** (99.99%); 171/171 units joined exactly |
| Corpus movement | **none** — instrumentation only; both verdict-bearing artifacts byte-identical to `docs/reports/c-gateway-constant-do-while/artifacts/` **with the census feature ON and actively streaming** |
| Sole-blocked functions | **284**, holding **833** leaves (**4.5%** of the family) — the integrator's wave-33 figure reproduced exactly, independently |
| Constraint 5's premise | **fails for 95.7%** of the family and **87.2%** of the sole-blocked leaves: the deciding chain's base is one the function *may bind* to a resource identity |
| Final member class | **72.2% `Unresolved`** — the declaration reader will not commit to the member at all |
| Best single shape, discounted | **11 functions decided** |
| Best pair of shapes, discounted | **17 functions decided** |
| Union of all **403** shapes | **45 functions** — the ceiling if every shape were admitted at once |
| Record-body reader route | measured separately: **2 functions** decided (§7.2) |
| Recommendation | **No shape and no reader widening survives the discount.** Every route into this family clears leaves and decides almost nothing; see §7.3 |

**The one sentence that matters for scoping.** Ranking shapes by leaf mass and
ranking them by *decided functions* produce nearly disjoint orders. The largest
shape by leaves — 1,416 leaves across 710 functions — decides **zero**
functions. The best shape by decided functions holds **34** leaves and decides
**11**. A wave-37 card cut on the leaf count would clear leaves and decide
almost nothing, which is the failure mode this card exists to prevent.

The same asymmetry appears again by an independent route. Because 72.2% of
leaves end at a member the declaration reader will not classify, the card also
instrumented that reader; the whole-record refusal it found accounts for 2,713
leaves and would decide **2** functions (§7). And the reader's own dominant
answer is that **78.5% of unresolved members were never a question at all** —
the *base*'s declared type was not committed — which is a different reader again
and the honest next measurement (`BASE-TYPE-READER-UNMEASURED`, §9).

## 2. What was instrumented, and why not source text

The abandoned first census card (`C-GATEWAY-INDIRECT-CENSUS`, wave 33)
classified the **original source line** of each refusal. Refusals are recorded
at the **preprocessed** line (constraint 12a), and wave 35's
`C-GATEWAY-CONSTANT-DO-WHILE` measured the gap between the two at roughly a
factor of 57 for macro idioms, so every macro-expanded access was misclassified.
That is not a bug to be fixed in a text classifier; it is a reason not to have
one.

This card instruments the analyzer instead, on the `JOINED-PROVENANCE-CYCLES`
pattern — populations read from the analyzer's own structures rather than from a
text scan. At each of the three constructors of this diagnostic in
`crates/qcore-c-gateway/src/legacy_c.rs`, **after the classifier has already
committed to refusing**, the census reads:

* the **token texts the gate itself read**, through the gate's own window — the
  exempted spans (`borrowed_actual`, `aggregate_store`, `variadic_nested`)
  subtracted, from the gate's own start index;
* `FunctionTypes::may_bind` — the analyzer's own over-approximation of the names
  a function may bind to a resource identity, whose negation is **literally
  constraint 5's premise**;
* `UntrackedBases::never_bound` — the same question as the admission rules ask
  it;
* `TypedAccessModel`'s record-member classification — `scalar_field`,
  `record_pointer_field`, `single_pointer_field`, `void_pointer_field`,
  `addressable_field`, `borrowable_field` — so the member's class is reported in
  the analyzer's own vocabulary and not in one this card invented;
* the formal-parameter set and declared-type table, for the base's kind.

There is therefore no second reader that could disagree with the first.

### 2.1 Inertness

Everything is behind the off-by-default cargo feature **`indirect-census`** on
`qcore-c-gateway`, enabled by no workspace member. With the feature off the
module and every call site are compiled out. With the feature **on**, and the
census actively streaming 88,001 statement records, the pinned corpus audit
still produces the archived artifacts byte for byte (§8) — which is the stronger
half of the claim, because it says the instrumentation perturbs no decision even
when running.

## 3. The join, and its three measured caveats

Each caveat was measured, and each makes the answer **more conservative** than a
naive one would have been.

1. **The unit join is exact.** The census stamps a translation-unit ordinal at
   `analyze_unrestricted_c11_with_inventory`. It records **171** distinct
   ordinals against the document's **171** leaf-carrying units, and the greedy
   overlap assignment is a bijection covering every unit: **0 unassigned**. The
   handful of C function names that recur across units therefore never had to be
   guessed at — the `JOINED-PROVENANCE-CYCLES` census recorded 36 such names as
   a residual; here the ordinal removes the question. (Measured separately: only
   **4** `(function, line)` pairs occur under more than one ordinal at all.)

2. **One published leaf can cover several refused statements.** A leaf's
   identity is `(unit, function, preprocessed line)`, and **1,964 leaves
   (10.6%)** cover more than one refused statement — one preprocessed line
   holding several statements, which is what macro expansion does. The widest is
   **15 statements** at
   `librdkafka/rdkafka_partition.c/rd_kafka_msgq_insert_msgq_before/20648`. Such
   a leaf clears only if **every** one of its statements is admitted, so the
   census attributes each leaf its *hardest* statement.

3. **W4 validates a summary by re-analysis** (constraint 4), so one statement is
   classified several times. Byte-identical repeats are one statement; only
   distinct records are distinct statements. The 88,001 streamed records reduce
   to the statement set the leaves are explained by.

**The one unexplained leaf (0.01%)** is recorded as a residual rather than
rounded away: `CENSUS-UNEXPLAINED-LEAF`, §9.

## 4. The distribution

Counted per **leaf** — the published unit of refusal — each leaf attributed its
hardest statement, and each statement its hardest access chain (a statement
carrying several chains must have every one admitted, so the hardest decides).

**Where the refusal fires.** The lexical access gate accounts for essentially
all of it: **18,539 (99.9%)** `LexicalGate`, 15 `UnclassifiedStoreValue`, and
**zero** `NonPointerResultStore`. The three-way split of the gate's own
disjunction:

| position | leaves | share |
|---|---:|---:|
| `CallArgument` — no `=`, statement carries a call | 7,910 | 42.6% |
| `AssignmentValue` — a **read** on the value side | 5,520 | 29.8% |
| `AssignmentDestination` — a **write** through an indirect place | 4,939 | 26.6% |
| `ExpressionStatement` | 170 | 0.9% |
| `Declaration` | 15 | 0.1% |

**The base.** 43.1% `Formal`, 34.1% `AutomaticLocal`, 14.2%
`UnreadableDeclarations` (the declaration reader refused the whole function, so
no name in it has a committed type), 6.5% `CallResult`, 1.1% file-scope or
otherwise outside the function's declarations.

**Constraint 5's premise, which bounds every admission.**

| | leaves | share |
|---|---:|---:|
| the function **may bind** that base to a resource identity | 17,763 | **95.7%** |
| the function **never binds** it — the premise holds | 603 | **3.2%** |
| no chain at all | 188 | 1.0% |

The whitelist premise `remaining-c02`'s fail-closed pin already caught once —
"the base is a name this function never binds to a resource identity" — is
therefore **unavailable for 95.7% of this family by construction**. That single
row bounds any wave-37 arbiter that does not propose to weaken it, and
constraint 5 says in terms that it "was not weakened; keep it that way".

**The two readings of that premise do not agree**, and which one an arbiter
reads changes the admission space by a factor of four:

| | leaves | share |
|---|---:|---:|
| `may_bind=true`, `never_bound=false` | 14,870 | 81.0% |
| `may_bind=true`, `never_bound=**true**` | 2,893 | **15.8%** |
| `may_bind=false`, `never_bound=true` | 563 | 3.1% |
| `may_bind=false`, `never_bound=false` | 40 | 0.2% |

`FunctionTypes::may_bind` (`bound_names`) and `UntrackedBases::never_bound`
(`untracked_bases`) are two different over-approximations of one English
sentence, and they disagree on **2,933 leaves (16.0%)**. Constraint 5's own
wording is `may_bind`'s, and `may_bind` is the stricter refuser here. This is
recorded as `CENSUS-TWO-PREMISE-READINGS`, §9: a wave-37 card that reads the
looser one is quietly working in a space four times larger than the constraint
describes.

**Chain shape.** Depth 1 accounts for 67.7%, depth 2 for 20.6%, depth 3 for
9.2%. By spelling: `->` 55.3%, `->.` 10.5%, `->->` 7.0%, `->..` 6.5%, `[]`
4.6%, `*` 4.3%, `.` 3.5%. There are **403** distinct shapes in all.

**The member the chain ends at** — and this is the row the recommendation turns
on:

| member class | leaves | share |
|---|---:|---:|
| **`Unresolved`** — the reader will not commit to the member | **13,396** | **72.2%** |
| `NoMember` (a subscript or bare dereference) | 2,084 | 11.2% |
| `Scalar` — the class constraint 5's whitelist is about | 1,051 | 5.7% |
| `AggregateValued` | 544 | 2.9% |
| `SinglePointer` | 494 | 2.7% |
| `RecordPointer` | 455 | 2.5% |
| `VoidPointer` | 214 | 1.2% |
| `OtherPointer` | 128 | 0.7% |

Restricting to the **10,260 depth-1 `->` leaves**, where "which reader refused"
is separable, the answer splits cleanly:

| | leaves | share |
|---|---:|---:|
| the **base's** type was not committed to `struct TAG *` | 4,943 | 48.2% |
| base committed, the **member** was not classified | 2,783 | 27.1% |
| base committed, member classified (all classes) | 2,534 | 24.7% |

So three quarters of even the simplest shape are blocked by the **declaration
reader**, not by the access gate, and the two halves are roughly 2:1.

## 5. The sole-blocked population

| | |
|---|---:|
| functions carrying a leaf of this family | 3,239 |
| functions whose **only** blocker family is this one | **284** |
| leaves those functions hold | **833** (4.5% of 18,555) |

This reproduces the integrator's wave-33 per-function profiling exactly — 284
functions, 833 leaves, 4.5% — by a completely different method, which is the
strongest evidence available that the census reads what it claims to.

The asymmetry the brief names is confirmed: **95.5% of the family's leaf mass
sits in functions carrying other blockers too**, so clearing those leaves
decides nothing. The top co-blockers, by how many of the 3,239 functions carry
them: branch condition **2,108**, uncontracted call **1,965**, unresolved
identity **1,177**, braced initializer **851**, for-loop condition **577**.

**The sole-blocked population's own axes differ sharply from the family's**,
which is why they are reported separately rather than inferred:

| axis | sole-blocked (833 leaves) | whole family |
|---|---|---|
| `AssignmentDestination` (a write) | **40.1%** | 26.6% |
| `CallArgument` | 40.0% | 42.6% |
| `AssignmentValue` | 19.1% | 29.8% |
| carries an **uncontracted** call | **49.8%** | — |
| carries a **contracted** call | 10.4% | — |
| no call at all | 39.7% | — |
| constraint 5's premise **holds** | **12.7%** | 3.2% |
| member `Unresolved` | 61.2% | 72.2% |
| base `Formal` | 63.0% | 43.1% |

The premise holds four times more often here than in the family at large — and
still only for **106 of 833** leaves.

**Half the sole-blocked functions need more than one shape.** Only **136 of 284
(47.9%)** have every leaf-statement in a single shape; 85 need two, 29 need
three, and 34 need four or more. A single-shape arbiter cannot decide the other
148 however sound it is.

## 6. The discounted three-way prediction

The brief asks, for each candidate single-shape admission, the predicted
three-way split — functions left leaf-free, exchanged for another refusal, or
unmoved — so wave 37 is cut on a number that has already been discounted.

**Method, stated so it can be checked rather than believed.** For a candidate
shape S, take every function carrying a leaf whose statements include S. It is
predicted **leaf-free** only if all three hold:

* its only blocker family is this one — otherwise **unmoved**, because another
  family still refuses it;
* every statement of every leaf it holds in this family has shape S — otherwise
  **unmoved**, because the leaves S does not cover remain;
* and no such statement carries a call to an **uncontracted** function, nor a
  second access chain of a different shape — otherwise **exchanged**, because
  admitting the access leaves the statement facing the uncontracted-call gate,
  or a second access S does not cover.

A call the analyzer already holds a contract for is deliberately **not** counted
an exchange: the statement's remaining decision is one the projection can
already make. Without that distinction the census would report every `f(p->x)`
as an exchange and understate the candidate set — the first draft of this
analysis did exactly that and was corrected before publication.

The exchange test is a **proxy**. It errs toward calling a function exchanged
rather than cleared, so the CLEAR column is an **upper bound on what a wave-37
arbiter could hope for**, not a prediction of what it would achieve.

### 6.1 Ranked by decided functions

| shape (base · premise · steps · member · position) | leaves | funcs | **clear** | exch | unmoved |
|---|---:|---:|---:|---:|---:|
| `AutomaticLocal · never-bound · . · Unresolved · AssignmentDestination` | 34 | 25 | **11** | 0 | 14 |
| `Formal · never-bound · ->. · Unresolved · AssignmentDestination` | 22 | 16 | **6** | 0 | 10 |
| `Formal · never-bound · -> · Unresolved · AssignmentDestination` | 70 | 36 | **4** | 2 | 30 |
| `AutomaticLocal · may-bind · * · NoMember · ExpressionStatement` | 49 | 47 | **3** | 0 | 44 |
| `Formal · never-bound · -> · SinglePointer · AssignmentDestination` | 13 | 8 | **3** | 0 | 5 |
| `Formal · may-bind · [] · NoMember · CallArgument` | 53 | 47 | **2** | 10 | 35 |
| `Formal · never-bound · [] · NoMember · AssignmentValue` | 25 | 19 | **2** | 0 | 17 |
| `Formal · never-bound · ->.. · Unresolved · AssignmentDestination` | 15 | 7 | **2** | 1 | 4 |

### 6.2 Ranked by leaf mass — the same shapes, the other order

| shape | leaves | funcs | **clear** | exch | unmoved |
|---|---:|---:|---:|---:|---:|
| `Formal · may-bind · -> · Unresolved · CallArgument` | **1,416** | 710 | **0** | 13 | 697 |
| `AutomaticLocal · may-bind · -> · Unresolved · AssignmentValue` | 1,122 | 352 | **0** | 0 | 352 |
| `Formal · may-bind · ->. · Unresolved · CallArgument` | 904 | 239 | **0** | 12 | 227 |
| `AutomaticLocal · may-bind · -> · Unresolved · CallArgument` | 795 | 364 | **0** | 0 | 364 |
| `Formal · may-bind · -> · Unresolved · AssignmentValue` | 773 | 506 | **0** | 5 | 501 |
| `CallResult · may-bind · -> · Unresolved · CallArgument` | 757 | 342 | **0** | 0 | 342 |
| `Formal · may-bind · -> · Unresolved · AssignmentDestination` | 583 | 269 | **1** | 0 | 268 |
| `Formal · may-bind · -> · Scalar · AssignmentDestination` | 349 | 91 | **1** | 0 | 90 |

**The top eight shapes by leaf mass — 6,699 leaves, 36% of the whole family —
decide two functions between them.** That is the card's result in one line.

Totals over all **403** distinct shapes: **20** decide at least one function;
the best decides **11**; the union of every shape's cleared set is **45
functions**, which is the ceiling if a single card admitted *all 403 shapes at
once* — 45 of 284 sole-blocked, 45 of 3,239 touched.

### 6.3 Pairs

148 of the 284 sole-blocked functions need two or more shapes, so a single-shape
ranking understates a two-shape arbiter. Under the same discount, the best pair
decides **17** functions:

* A: `AutomaticLocal · never-bound · . · Unresolved · AssignmentDestination`
* B: `Formal · never-bound · ->. · Unresolved · AssignmentDestination`

The next three pairs decide 15, 14 and 14, and **every one of the top eight
pairs contains shape A** — the pair ranking is one shape plus a choice of
partner, not two independent wins.

### 6.4 What "leaf-free" is not

A function predicted leaf-free holds no leaf of any family. **That is not
`Safe`.** `Safe` additionally requires every obligation discharged and full
analysed coverage at the certificate gate (constraint 12), and invariant 2's
verdict contract on top. Card `C-GATEWAY-RETURN-POSITION` made the same point
about its own three-way split and measured the after-status histogram rather
than assuming it; this card cannot measure that histogram because it makes no
change, so **the CLEAR column is a count of functions that would stop being
blocked by this family, and nothing more**.

## 7. Why the member is unresolved — and why that is not the lever either

§4 showed 72.2% of leaves ending at a member the declaration reader will not
classify. Because that row dominates everything else, this card instrumented the
reader too — `TypedAccessModel::read_record_body`, behind the same feature — and
asked it, per record body, what it did and which token stopped it.

**The reader is total-or-reject over the whole body.** Its first statement is

```rust
if body.iter().any(|t| matches!(*t, "{" | "}" | "(" | ")" | ":" | "=")) {
    return None;
}
```

so **one** member declaration it will not parse — a nested anonymous aggregate's
`{`, a function-pointer member's `(`, a bitfield's `:` — makes **every** member
of that record unclassifiable, including the plain scalars beside the offending
one. Measured over the pinned corpus:

| | |
|---|---:|
| distinct record tags whose body the reader met | **909** |
| tags read differently in different units | **0** |
| tags the reader **refused whole** | **173 (19.0%)** |
| refused on `{` (nested anonymous aggregate) | 98 (56.6%) |
| refused on `(` (function-pointer member) | 66 (38.2%) |
| refused on `:` (bitfield) | 9 (5.2%) |

The worked example is `pb_ostream_s` in `librdkafka/nanopb`: it carries a
function-pointer `callback` member, so `stream.max_size` and
`stream.bytes_written` — plain scalars — are unclassifiable too, and
`pb_ostream_from_buffer`'s five stores are all refused. That function is one of
the 11 the best shape would decide.

### 7.1 But the member is not the main blocker — the base is

Crossing the reader's answer with the refused accesses that actually land on
each tag gives the result that settles this card:

| of the **13,396** leaves ending at an `Unresolved` member | leaves | share |
|---|---:|---:|
| **no record tag at all — the BASE's type was never committed** | **10,514** | **78.5%** |
| a tag was committed; the record body was refused whole on `{` | 2,367 | 17.7% |
| a tag was committed; the record body was refused whole on `(` | 346 | 2.6% |
| a tag was committed, the body read, the member simply in no class | 166 | 1.2% |
| the record is never declared in the units that use it | 3 | 0.0% |

(The four tagged rows sum to **2,882**, and 2,882 + 10,514 = 13,396; the two
whole-body refusals sum to the **2,713** quoted below.)

So the whole-body-refusal defect — real, and worth fixing on its own merits —
accounts for **one fifth** of the unresolved members. Four fifths are blocked
one level earlier, at the *base*'s declared type: `classify_specifiers`,
typedef resolution and `read_locals`, none of which this card instrumented.

### 7.2 The ceiling, measured

A per-member total-or-reject reader is the obvious wave-37 candidate, so its
**upper bound** was computed rather than guessed. Counting only sole-blocked
functions whose every leaf-statement is non-exchanging **and** barred solely by
an `Unresolved` member on a whole-body-refused record — generously assuming
every such member would become classified *and* that the resulting access would
then be admitted:

> **2 functions.** Of those, **1** also satisfies constraint 5's premise on
> every base: `librdkafka/nanopb/pb_encode.c/pb_ostream_from_buffer`.

It would retire on the order of 2,713 leaves and decide **two functions**. The
top tags by leaf count show why: `rd_kafka_s` alone carries 593 unresolved-member
leaves, of which **10** are in sole-blocked functions; `rd_kafka_broker_s` 467 of
which 3; `rd_kafka_toppar_s` 226 of which 6. The leaf mass concentrates in a
handful of very large librdkafka structs that appear all over functions with many
other blockers.

**This is the same asymmetry as §6, reached by an independent route.** Every way
into this family clears leaves and decides almost nothing.

### 7.3 Ranked recommendation

The brief asks for at most two shapes, ranked, with the discount stated, and
says that "if no shape survives the discount, say so; that is a valid result."

**No shape survives the discount as a standalone wave-37 arbiter.** The best
decides **11 of 3,239** touched functions and **11 of 284** sole-blocked ones;
the best pair, **17**; every one of the 403 shapes admitted at once, **45**; and
the record-reader route, **2**. For completeness the two worth naming are:

**Rank 1 — `AutomaticLocal · never-bound · . · Unresolved · AssignmentDestination`.**
34 leaves, 25 functions touched, **11 decided, 0 exchanged, 14 unmoved.** The
form is `local.field = VALUE;` where `local` is an automatic aggregate *value*
this function declares. Its attraction is that it is the one shape in the top of
either ranking where **constraint 5's premise actually holds** — and holds under
`may_bind`, which is the constraint's own reading, not the looser `never_bound`
(see `CENSUS-TWO-PREMISE-READINGS`).

*Soundness questions an arbiter must answer, and in which direction:*

* **Constraint 5** — satisfied by construction for this shape; this is the rare
  corner where it is. But the arbiter must read `may_bind`, not `never_bound`,
  or it silently enlarges its own population.
* **The member's class is unknown**, so `aggregate_store::admitted_field`'s
  requirement — exactly one of scalar / single-pointer / void-pointer — fails.
  Admitting the store *without* the class is **unsound in the admitting
  direction**: an unclassified member may be a pointer, and overwriting it
  destroys a tracked identity and with it a release obligation. The arbiter
  therefore cannot be an access rule; it must first be a **type-reader** change,
  which is §7's route with its measured ceiling of 2.
* **Constraint 5b applies directly and must be re-read before starting.** It
  warns that "a card that widens member typing far enough to put a
  projection-bound place on an admitted chain falsifies exactly this premise" —
  `TrackedBasesAreBound`, which
  `QCore.ReturnPositionEscape.an_admitted_chain_names_no_tracked_place` takes as
  an explicit hypothesis rather than proving. Widening member typing is exactly
  what this shape needs.
* **The shape is new to `admitted_store`**, which matches only
  `[base, "-", ">", field]`; there is no `.` form at all today. Narrower in one
  way (an automatic object of this function, no pointee, no liveness question,
  and constraint 10's outlives-the-call rule does not apply), new in another.
* **`POISON-UNMASK-ON-ADMISSION`** (§9) applies, as it does to every candidate.

**Rank 2 — `Formal · never-bound · ->. · Unresolved · AssignmentDestination`.**
22 leaves, 16 functions touched, **6 decided, 0 exchanged, 10 unmoved.** Form
`formal->a.b = V;`. Same premise position and same member-class obstacle, plus
one more: the chain crosses an `AggregateValued` member, and `admitted_field`
has no continuation rule for one — only `record_pointer_field` continues a
chain, and only through `->`. Paired with Rank 1 this is the best two-shape
arbiter available, at **17 functions**.

**What this card recommends instead.** Do not cut a wave-37 arbiter on this
family for its size. If the wave wants gateway reach, the measurement that is
missing — and that this card deliberately did **not** take, because it is a
different reader — is the one §7.1 exposes: **78.5% of unresolved members are
blocked at the base's declared type, not at the member's.** Nobody has measured
what `classify_specifiers`, typedef resolution and `read_locals` refuse, or what
the sole-blocked reach of repairing them would be. That is the honest next
census, and it is named `BASE-TYPE-READER-UNMEASURED` in §9.

## 8. Inertness and byte identity

The card's only change to shipped code is instrumentation, and both halves of
"inert" are evidenced rather than asserted.

**Compiled out.** The `indirect_census` module, the `Parser::census_function`
field, the three call sites in `legacy_c.rs` and the one in
`typed_field_access.rs` are all behind `#[cfg(feature = "indirect-census")]`.
The feature is declared on `qcore-c-gateway` and enabled by **no** workspace
member, so every gate step in §10 measures a tree in which this code does not
exist. A second clippy pass **with** the feature on is added to the gate so the
instrumentation is linted rather than merely unlinted.

**Does not perturb.** The stronger half: a pinned Cycle 19 corpus audit run by
the **instrumented binary with the census actively streaming** reproduces the
wave-35 baseline `docs/reports/c-gateway-constant-do-while/artifacts/` **byte
for byte** on both verdict-bearing artifacts.

| artifact | result | sha256 |
|---|---|---|
| **`diagnostics`** | **IDENTICAL** | `9af9ee2e4e0837b6593300478898e64ec7697cd28f4356211c8988dc40003da9` |
| **`chunk-contracts`** | **IDENTICAL** | `c4f438e4ed3dab3c3b0e776af5c7c5181385353a72c67b6874ab4df5ba917980` |
| `source` | IDENTICAL | `ae38f6c93dfbe341b187077e9dd0074501000ea76d621feb481db2303930e529` |
| `source-effects` | differs | carries the checkout root |
| `contracts` | differs | carries the checkout root |

The last two have differed at **every** re-pin in this chain for that reason,
which `C-GATEWAY-RETURN-POSITION`'s and `C-GATEWAY-CONSTANT-DO-WHILE`'s own
byte-identity logs also record; they are stated rather than hidden. The
assertion is published as `logs/byte-identity.txt`, and the audit's exit code
(0) is captured to a file, never through a pipe.

**This card owns no re-pin and adds no archive to constraint 21's frozen list.**
The corpus baseline remains `docs/reports/c-gateway-constant-do-while/artifacts/`.

## 9. Named follow-ons

* **`CENSUS-UNEXPLAINED-LEAF`** — one published leaf of 18,555 (0.01%) is
  explained by no census statement. The likeliest cause is the `for`-clause
  path: `classify_simple` is also called for a `for` init and increment clause,
  where the statement the parser attributes is the `for`'s own line rather than
  the clause's first token, so the census's `(function, line)` key can miss.
  Not chased, because it is one leaf and the direction is understood.
  **Discharge:** thread the statement's attributed line into `classify_simple`
  rather than reading it off the first token, and re-run; or identify the leaf
  and record why it is unexplainable.

* **`CENSUS-TWO-PREMISE-READINGS`** — `FunctionTypes::may_bind` (`bound_names`)
  and `UntrackedBases::never_bound` (`untracked_bases`) are two different
  over-approximations of constraint 5's one English sentence, and they disagree
  on **2,933 leaves (16.0%)** of this family: 2,893 where `may_bind` says the
  base is bindable and `never_bound` says it is not, and 40 the other way.
  Constraint 5's wording is `may_bind`'s, and `may_bind` is the stricter refuser
  here, so a card reading `never_bound` is working in a space roughly four times
  larger than the constraint describes **without saying so**. This is not a
  defect in either function — they are consumed by different rules — but it is a
  trap for any wave-37 arbiter. **Discharge:** state in constraint 5 which
  reading the premise names, or prove the two agree wherever an admission
  consults them.

* **`INDIRECT-EXCHANGE-PROXY-UNVALIDATED`** — §6's three-way split rests on a
  proxy for "exchanged": a statement carrying an uncontracted call, or a second
  chain of a different shape, is predicted to stay refused. The proxy has
  **never been validated against an actual admission**, because this card makes
  none. It errs toward exchange, so the CLEAR column is an upper bound; but the
  *size* of the error is unmeasured. **Discharge:** the first wave-37 arbiter to
  admit any shape publishes its measured three-way split beside this card's
  prediction for the same shape, and the difference is recorded.

* **`INDIRECT-CENSUS-FEATURE-UNBUILT`** — the `indirect-census` feature is built
  by no CI job and by no gate step other than the one this card adds to its own
  gate script. It will rot: a later refactor of `legacy_c.rs` or
  `typed_field_access.rs` will break it silently, and the next card that wants
  the census will pay to repair it. **Discharge:** either a CI job that builds
  `-p qcore-c-gateway --features indirect-census`, or a decision — recorded —
  that the instrumentation is disposable and a future census re-derives it.

* **`CENSUS-DECLARATION-WINDOW-APPROXIMATE`** — for the 15 `Declaration`-arm
  leaves the gate's predicate is not `contains_unmodeled_access` but a separate
  bracket/dot test over the declaration prefix, so the census reads a window the
  gate did not read. 15 leaves of 18,555; recorded rather than repaired.
  **Discharge:** give the declaration arm its own census window, or state the
  arm is out of every arbiter's scope and drop the axis for it.

* **`BASE-TYPE-READER-UNMEASURED`** — §7.1's exposed gap and the honest next
  census. **78.5% of the 13,396 unresolved-member leaves (10,514) carry no
  record tag at all**: the reader never committed to the *base*'s declared type,
  so the member was never a question. That is `classify_specifiers`, typedef
  resolution and `read_locals` — a different reader from the record-body one
  this card instrumented, and one nobody has measured. Every reach number in
  this report is therefore a statement about the routes it measured, not about
  the family's ceiling. **Discharge:** instrument the base-type reader the same
  way — what it refuses, on which spelling — and publish the sole-blocked reach
  of repairing each, before any card is cut on it.

* **`RECORD-BODY-TOTAL-OR-REJECT`** — `TypedAccessModel::read_record_body`
  refuses a whole record body on any `(`, `)`, `{`, `}`, `:` or `=`, so one
  member declaration the reader cannot parse makes **every** member of that
  record unclassifiable. **173 of 909 record tags (19.0%)** in the pinned corpus
  are refused this way — 98 on `{`, 66 on `(`, 9 on `:` — accounting for 2,713
  unresolved-member leaves. A genuine defect in the reader's granularity, filed
  on its own merits. It is **not** a promising arbiter: §7.2's measured ceiling
  is **2 sole-blocked functions**, 1 of them with constraint 5's premise intact.
  **Discharge:** a per-member total-or-reject reader — classify the member
  declarations that parse, mark the ones that do not as explicitly unreadable so
  they stay refused — landed with its own measured re-pin, or a recorded
  decision that the 2-function reach does not justify the risk. **Not a
  discharge:** citing the 2,713 leaves.

* **`LOAD-SENSITIVE-ROW-UNNAMED`** — §10.1's observation.
  `a_forced_override_never_weakens_an_execution`
  (`crates/qcore-verifier-host/tests/execution_quota.rs:777`) failed with
  `Io("Broken pipe (os error 32)")` in a workspace run deliberately concurrent
  with two pinned corpus audits, and **passes in this card's isolated gate**.
  That is constraint 16's exact signature and crate, but the test is **not** one
  of its six named rows nor the deliberately-excluded seventh. Filed rather than
  promoted: constraint 16's rows each carry a measured load figure, and one
  observation under a heavy self-inflicted load is not one. **Discharge:** a
  card that can measure it — bisect the load at which it flips, as
  `vp04_vp07_vp08_process_lifecycle_and_payload_budgets` was measured failing at
  50 and passing at 37 — and either add the row with its figure or record that
  it is the same mechanism as an existing row. **Not a discharge:** re-running
  until green.

* **`POISON-UNMASK-ON-ADMISSION`** — a cross-cutting soundness hazard for
  **every** candidate in §6, and the one most easily missed. Admitting an access
  makes its statement no longer refused, so the statement's `RefusedWrites` no
  longer poisons **any** binding. Constraint 10a records that sparing a binding
  can *unmask* a stale scalar fact a `NoOp` mutation left behind
  (`AOM-POISON-NOOP-INCREMENT`, whose measured instance is `pass++` lowering to
  `NoOp` so a `Clear`-bound scalar stays definitely-null for ever, making a
  `pass != 0` edge `Infeasible` and suppressing the refusals inside it). A
  wave-37 arbiter's re-pin can therefore move counts **in the admitting
  direction for reasons unrelated to the access it admits**, and must attribute
  those moves separately rather than to its own rule. **Discharge:** the arbiter
  measures, for its own admitted set, how many refusals disappear through an
  edge becoming infeasible rather than through the access being modelled, and
  publishes the two numbers apart.

## 10. Standing gate

Run in order from `scripts/gate.sh`, **every exit captured to its own file and
never through a pipe**. Machine: Linux, 621 GB volume with 129 GB free at the
start of the run (well above the 60 GB floor, so the full gate ran rather than
crate-scoped checks).

| step | exit |
|---|---|
| `git submodule update --init --recursive` | **0** |
| `cargo test --workspace --no-fail-fast` | **0** |
| `cargo fmt --all -- --check` | **0** |
| `cargo clippy --all-targets -- -D warnings` | **0** |
| `cargo clippy -p qcore-c-gateway --features indirect-census --all-targets -- -D warnings` | **0** |
| `cargo run -p qcore-compiler --example reevaluation_probe` | **0** |
| `cargo run -p qcore-compiler --example reevaluation_followup` | **0** |
| `python3 scripts/check_lean_audits.py` | **0** — 132 audits passed |

**Workspace tests: 4,580 passed / 0 failed / 8 ignored**, over **230** result
summaries from **228** test executables. That reproduces the wave-35 close
figure exactly, which is the expected outcome for a card that adds no test and
changes no shipped behaviour.

The fifth row is **an extra step this card adds to its own gate**, not part of
the standing list. Every other step runs with `indirect-census` **off** — the
feature is enabled by no workspace member, so the standing gate measures a tree
in which the census does not exist — and that extra pass lints the
instrumentation rather than leaving it unlinted. Per-step exits are archived at
`logs/gate-exits.txt` and `gate.json`.

### 10.1 Constraint 16: a load-sensitive row, both facts recorded

A **separate, earlier** `cargo test --workspace --no-fail-fast`, run deliberately
concurrent with two pinned corpus audits, returned **4,579 passed / 1 failed / 8
ignored** over the same 230 summaries. The one failure:

```
---- a_forced_override_never_weakens_an_execution stdout ----
thread 'a_forced_override_never_weakens_an_execution' panicked at
crates/qcore-verifier-host/tests/execution_quota.rs:777:29:
called `Result::unwrap()` on an `Err` value: Io("Broken pipe (os error 32)")
```

**The same test passes in the isolated gate run above** (`workspace_tests.log`
line 5811, `... ok`). Constraint 16's standing instruction is to verify such a
row in isolation and record **both** facts rather than weaken anything, and that
is what this section does. Nothing was retried until green, no deadline was
raised and no test was touched.

`Io(Broken pipe)` is constraint 16's own named signature for two of its rows —
the confined-fixture write test, and
`enforced_process_limits::memory_growth_beyond_the_declared_limit_is_bounded`
"at load ~40". This failure is the **same shape in the same crate**, and
`execution_quota.rs` is the same file family. But
`a_forced_override_never_weakens_an_execution` is **not** one of constraint 16's
six named rows, nor the deliberately-excluded seventh
(`VERIFIER-HOST-NPROC-ROW-UNVERIFIABLE-UNDER-WAVE`). It is filed as
`LOAD-SENSITIVE-ROW-UNNAMED` in §9. This card does **not** propose adding it to
constraint 16: one observation under an unusually heavy, self-inflicted load is
an observation, and constraint 16's rows each carry a measured load figure that
this card did not take. Naming the observation is the honest step; promoting it
would be the unearned one.

## 11. Reproduction

Scripts are archived under `scripts/` beside this report and were run from
`measure/c-gateway-indirect-census-2/` (worktree-private, per `CLAUDE.md`'s
hygiene rule; the raw streams are ~51 MB and ~150 MB and are **not** committed).

```sh
# 1. the instrumented binary — the feature is enabled here and nowhere else
cargo build --release -p qcore-compiler \
    --example cycle19_fluent_bit_libraries_audit \
    --features qcore-c-gateway/indirect-census

# 2. one pinned Cycle 19 corpus audit, census streaming beside it
sh run_audit.sh final target/release/examples/cycle19_fluent_bit_libraries_audit

# 3. the inertness assertion: both verdict-bearing artifacts byte-identical
python3 byte_identity.py runs/final

# 4. the census
python3 analyse.py runs/final out      # distribution, sole-blocked, predictions
python3 records.py runs/final out      # the record-body reader's own answer
python3 spotcheck.py runs/final "<shape substring>" 20
python3 probe_dupes.py runs/final/indirect-leaves.jsonl
```

The pin, compile database, ChunkIO package and the two environment variables are
exactly those waves 33–35 used; the transport stays `HistoricalStdin`
(constraint 11) and is the audit example's own pinned-evidence route, not
selected here. `QCORE_INDIRECT_CENSUS` and `QCORE_INDIRECT_CENSUS_RECORDS` name
the two census streams; with neither set, and with the feature off, nothing is
written and nothing runs.

## 12. For the integrator

### 12.1 Applied by this card

* **`docs/development/increments/wave-36.md`.** One paragraph, prepended. The
  file already existed (wave 36 was opened at `60958bf0`); it was not created.
* **`docs/reviews/r0_3/acceptance_matrix.json`.** Regenerated with
  `python3 docs/reports/remaining-d02/build_matrix.py . --gate docs/reports/wave35-close/gate.json`
  (the repo root is a positional argument; wave 36 has no close directory yet, so
  wave 35's gate is the one that preserves provenance, exactly as
  `C-GATEWAY-CONSTANT-DO-WHILE` used wave 33's). The regeneration is a
  **byte-identical fixed point — zero insertions, zero deletions.** That is the
  correct outcome: this card adds no Lean audit, no contract module and no
  provider package, and **retires no `named_followup`**, so the file is committed
  unchanged and is recorded here as *verified* rather than *edited*.
  `named_followups` is carried at **121**.
* **Nothing else.** This card owns no re-pin, moves no headline number in
  `AGENTS.md`'s ledger, adds no archive to constraint 21's frozen list and
  changes no corpus baseline. The baseline remains
  `docs/reports/c-gateway-constant-do-while/artifacts/`.

### 12.2 Not applied — needs your decision

* **The nine named follow-ons in §9 are not registered in
  `named_followups`.** A measurement card's whole output is findings, and
  registering nine at once against a 121-entry list is a judgement about the
  tracker's shape rather than about this card, so I have left it to you. The two
  I would register first, because they are the ones a wave-37 card will trip
  over, are **`BASE-TYPE-READER-UNMEASURED`** (the 78.5% nobody has measured)
  and **`CENSUS-TWO-PREMISE-READINGS`** (a card reading `never_bound` instead of
  `may_bind` silently works in a space four times larger than constraint 5
  describes). **`RECORD-BODY-TOTAL-OR-REJECT`** is the one genuine code defect
  found, and deserves an entry whether or not anyone acts on it.

* **`INDIRECT-CENSUS-FEATURE-UNBUILT` is a decision, not a task.** The
  `indirect-census` feature is built by no CI job. Either add one, or record the
  decision that the instrumentation is disposable and the next census
  re-derives it. I have not chosen for you, because the answer depends on
  whether wave 37 takes a gateway card at all — and this card's recommendation
  is that it should not take *this* one.

### 12.3 No standing constraint is proposed

This card lands no "when touching X, do not Y" rule, so the standing-constraint
inventory is untouched. Two existing constraints are *read* in ways worth
noting, and neither needs amending on this card's account:

* **Constraint 5** is the binding one throughout, and this card is evidence
  **for** it rather than against: the premise it protects is unavailable for
  95.7% of the family, which is exactly what makes the family hard, and nothing
  here suggests weakening it. If you want one sentence added anywhere, it is the
  clarification `CENSUS-TWO-PREMISE-READINGS` asks for — *which* of the two
  implementations the premise names — and that belongs to whoever takes that
  residual, in the words they measure, not to this card.
* **Constraint 5b** should be re-read by any wave-37 gateway card before it
  starts, for the reason 5b itself gives: it warns that widening member typing
  far enough to put a projection-bound place on an admitted chain falsifies
  `TrackedBasesAreBound`, and §7.3's Rank 1 needs exactly that widening. That
  warning is already written; this card only confirms it is live.

### 12.4 Recommendation to the wave

**Do not cut a wave-37 arbiter on this family.** It is the largest family in the
corpus and that is precisely the trap: its leaf mass and its function reach are
nearly disjoint, and every route measured here — 403 access shapes, their best
pair, and the record-body reader — decides between 2 and 17 functions against a
3,239-function population. If a gateway card is wanted, the honest prerequisite
is the base-type reader census `BASE-TYPE-READER-UNMEASURED` names, which covers
78.5% of the unresolved members and which nobody has measured. Cutting on the
18,555 would repeat the error this card was commissioned to prevent.
