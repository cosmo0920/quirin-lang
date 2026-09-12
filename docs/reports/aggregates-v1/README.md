# AGGREGATES-V1 — the first source-level aggregate type (wave 27)

Card brief: *"This is the wave's language head: the first source-level aggregate
(record/struct) type. Recorded consumers: CT09's gap ('the source language has
no aggregate type — no struct/field access in source'), the future HTTP/socket
data shapes, and the general type-system growth the user has directed."*

Base commit: main `f122cc03` (the wave-26 close).

**What landed.** `source.record.v1` — a named record type with a fixed list of
distinctly named scalar fields, a construction, and a field read:

```qcore
type Point { x: i32, y: i32, weight: i32 }

let p = Point { x: 65, y: 66, weight: base };
let a = p.x;
```

It is admitted on `source.buffer.v1`; every other route keeps the refusal it
already had, measured and pinned. A v1 record is a **plain copyable value**:
every field is an `i32`, so it carries no custody, opens no obligation and
derives no effect. Records lower to real C structs with one distinct member per
field and are byte-compared against a sanitized native run. **Zero reserved
words**, **zero new tokens**, **zero new diagnostic codes**, **no change to a
single line of name resolution**, 56 tests, 28 audited Lean entries in one new
audit file, no new axiom, and **all 80 emitting programs byte-identical** to the
base build.

**Three measurements carry the card.** The census rejected the obvious spelling
on evidence (§1). The field-custody decision was measured against the ownership
IR rather than argued from taste (§3). And the two positions the construct
introduces turned out to be **already drained** by name-resolution rules that
predate it, which is why this card changed no line of `lexical_references` and
reserved no word (§2.3).

---

## 1. The census (standing constraint 14)

Over all **435** `.qcore`/`.qc` files in the tree, counting *code* positions
separately from `//` and `/* */` comment text (`measure/census.py`), and a
second pass over QCore source embedded in Rust string literals across **575**
Rust files and **48,442** literals (`measure/census_rust.py`).

Every number in this section is the **base tree's**, which is the only tree a
census can be about. The archived re-run beside this report
([`census.txt`](census.txt)) is the *card* tree's and therefore reads one higher
in two places — 436 files and 12 `type NAME {` occurrences — the extra one in
each being this card's own fixture. Both are published rather than one silently
standing in for the other.

| Candidate | Code | Comment | Verdict |
|---|---|---|---|
| **`type`** | — | — | **already reserved**, and the head this card reuses |
| **`record`** | **8** | 19 | **rejected by the census** — see §1.1 |
| `struct` | 0 | 26 | unused; rejected on grounds other than collision (§1.2) |
| `field`, `fields` | 5, 0 | 23, 11 | `field` occurs in code as a contract parameter name |
| `data`, `object` | 12, 1 | 20, 16 | contract parameter names |
| `tuple`, `row` | 0 | 0, 5 | unused; not needed |

**Reserved by this card: none.** The declaration head is `type`, already
reserved; the construction and the field read are ordinary `let` statements; and
`{`, `}`, `,`, `:` and `.` are symbols the lexer has always produced.

### 1.1 `record` collides, in an archived probe pinned to a refusal

Seven of `record`'s eight code occurrences are `extern fn` **parameter names**
(`extern fn sample_reset(record: borrow<'call, mut, sample_record> use one)`),
which the referenced-names filter already shields because a word followed by `:`
is skipped. Reserving the word would not have broken those.

The eighth is decisive:

```
docs/reports/remaining-v07/probes/d02_boundary_record.qcore
    record R { }
```

That file is one line, and **two archived reports pin it to a refusal**:

| Report | Row |
|---|---|
| `docs/reports/remaining-v07/README.md:225` | `… + d02_boundary_record` (probe d02) → `d02_boundary_record.qcore:1:1` |
| `docs/reports/remaining-d01/README.md:59` | `… + d02_boundary_record` → `d02_boundary_record.qcore:1:1 Q000` |

Claiming `record` at declaration position would make that file **parse**, which
falsifies both tables. Standing constraint 21 forbids repairing that by editing
a frozen archive, so the card's syntax was renamed rather than the pinned
surface edited — character for character the `buffer` → `bytes`,
`work` → `effort` and `stream`-rejection shape. The Rust-literal pass found
`record` at a declaration head **zero** times outside this card's own suite, so
the archive is the whole of the collision, and it is enough.

`record_is_still_not_a_declaration_head` pins that the probe's line still
refuses at `Q000`.

### 1.2 `struct` is free and was still not taken

`struct` is census-clean at zero code occurrences. It was rejected on a
different ground: the language already has a **declaration head for a named
type**, and a second one would mean two spellings for "introduce a type" with
nothing to tell a reader which to use. `type NAME { … }` also sits directly
beside the existing `resource type NAME { … }`, which is the same shape for the
type that *does* own something — so the two read as one family rather than two.

### 1.3 `type NAME {` was measured before it was claimed

The head is claimed contextually: `type NAME =` is the alias it always was, and
`type NAME {` is a record. The census found **11** `type NAME {` occurrences in
code across the tree and **every one of them is `resource type NAME {`**:

```
contract/qcore/cfl/arena_lifetime.qcore, cfl/intrusive_list.qcore,
chunkio/chunk_destructor.qcore, chunkio/context_destructor.qcore,
chunkio/stream_destructor.qcore, library/custody_evidence.qcore,
librdkafka/queue_destructor.qcore, librdkafka/refcount.qcore,
librdkafka/typed_queue.qcore (×2), monkey/composition.qcore
```

The declaration reader tests `resource type` **before** `type`, so none of those
eleven ever reaches the new branch. **Bare `type NAME {`: zero, in
`.qcore`/`.qc` files and in Rust string literals alike.** The shape this card
claims was a parse failure before it — a bare `type NAME {` failed the alias's
own `=` validation — which is exactly the `extern library NAME;` precedent.

`a_resource_type_is_not_a_record`, `a_type_alias_is_still_a_type_alias` and
`a_braced_type_declaration_is_a_record` pin all three readings.

### 1.4 The collision the census found in the other direction

Constraint 14 is usually about a *new* word colliding with existing source. This
card adds none, so its census had to ask the reverse question: which **existing**
reserved words does a record declaration put in a new position?

The answer is the six words of the two **whole-token-stream** route selectors —
`module_imports::concurrency_span` and `source_tasks::has_source_tasks`, the pair
card `NATIVE-LIBRARY-SOURCE-BINDING` recorded. A record whose type name or any
field name is `spawn`, `join`, `qcore_buffer_new`, `qcore_buffer_read`,
`qcore_buffer_write` or `qcore_error_payload` selects the concurrency route from
its own spelling, and the refusal it earns depends on which half of the
construct the module writes:

```
type Job { spawn: i32 }                 -> Q015: concurrency requires an explicit
                                                 `import std.concurrency;`

let j = Job { spawn: 1 };               -> Q000: expected identifier
                                                 (the task-expression parser,
                                                  one layer earlier still)
```

**Both are fail-closed and neither is a wrong acceptance**, and that is why the
`NATIVE-LIBRARY-SOURCE-BINDING` span exclusion was measured and **deliberately
not copied**. Excluding a record declaration's span from the two selectors
retires the `Q015` half and leaves the `Q000` half exactly where it is — so the
exclusion would buy a type that **declares and cannot be constructed**, which is
worse than a collision that refuses consistently. Repairing it properly means
stopping the task-expression reader from claiming a `WORD:` at a statement
position, and that is a concurrency-route change rather than an aggregate one.

Recorded as `AGGREGATE-SELECTOR-WORDS`, pinned by
`a_field_named_for_a_route_selector_refuses_fail_closed`, and bounded from the
other side by `a_field_named_for_a_reserved_statement_word_is_admitted`: every
*other* reserved word — `drop`, `close`, `print` — is an ordinary field name,
because a field name is never collected as a reference (§2.3) and the statement
dispatches on its `let` head. The collision is six words, not the reserved set.

### 1.5 The two statement shapes

`let NAME = NAME { … };` and `let NAME = NAME.FIELD;` occur **zero** times in
code across all 435 `.qcore`/`.qc` files. The Rust-literal pass reports non-zero
counts for both, and every one was inspected: they are **Rust source and emitted
C** caught by a deliberately over-approximating literal extractor (an unbalanced
quote inside a comment lets a "literal" span real code), not QCore. The pass is
published as-is rather than tightened, because an over-approximation that is
then read is the safe direction for a census.

---

## 2. The form, and why this one

```ebnf
decl    = "type", NAME, "{", field, { ",", field }, [ "," ], "}" ;
field   = NAME, ":", "i32" ;
stmt    = "let", NAME, "=", NAME, "{", assign, { ",", assign }, [ "," ], "}", ";"
        | "let", NAME, "=", NAME, ".", NAME, ";" ;
assign  = NAME, ":", OPERAND ;
OPERAND = [ "-" ], INTEGER | NAME ;             (* `source.buffer.v1`'s own *)
```

### 2.1 Named fields, not positional

`Point(3, 4)` would have been simpler to read and was rejected. CT09's scenario
is *"distinct ordinary struct fields"*, and a positional construction makes
field identity a counting exercise at every site: two fields of one type
swapped at a construction is a defect no rule of this model could see. Naming
them makes the construction's totality check — every declared field supplied
exactly once — the thing that decides it.

It also costs nothing, because of §2.3.

### 2.2 A field value is the admitting route's own operand

A field takes an integer literal or a name the entry bound to a
check-time-constant integer — `source.buffer.v1`'s `BufferOperand`, read by
**that route's own reader**. So a value form one route admits and the other
refuses is not expressible, and a buffer at a field position is that route's
existing `Q055` rather than a rule this card wrote
(`a_buffer_at_a_field_position_is_refused_by_the_existing_rule`).

The same choice is what makes a field read need **no new operand form** at all:
it binds an ordinary scalar of that route's environment, so `print dec a`,
`b[a] = 1` and `for i in 0..a` all work on a field with nothing added — exactly
what card `LOOP-V1` found for a loop index.

### 2.3 Name resolution needed no new rule, and that is the card's quiet result

Both positions the construct introduces were **already drained** by rules
`frontend::lexical_references` has carried since long before it:

| Position | Existing rule | What it already shields |
|---|---|---|
| `Point { x: 1 }` — the field name | a word followed by a `Colon` is skipped | every `extern fn` parameter name |
| `p.x` — the field name | a word preceded by a `Dot` is skipped | `cost.total`, `std.buffer`, `qcore.libevent` |

What *is* collected is the type name at a construction (`Point`), and it
resolves because every declaration's name is a global. So:

* **no line of `lexical_references` changed**, and
* **no line of `is_reserved_source_word` changed**, and
* a construction naming a type the module does not declare refuses at **`Q013`**
  — name resolution, before this card's reader ever runs
  (`a_construction_of_an_undeclared_type_refuses_at_name_resolution`).

`a_field_name_is_not_a_reserved_word` pins the consequence from the other side:
a module may bind locals `x` and `y` **and** declare a record with fields of
those names, and everything resolves.

### 2.4 The `.` adjacency, pinned rather than assumed

The brief flagged the hazard by name: card `SOCKET-INET-ADDRESSES` measured that
`127.0.0.1` lexes `Float Dot Float`, because a `.` between two digit runs is part
of a float spelling. A field access is not that shape and **cannot become it**:
the lexer starts a `Float` only at a **digit**, so

| Spelling | Tokens | Pinned by |
|---|---|---|
| `p.x` | `Word Dot Word` | `a_field_access_lexes_as_word_dot_word` |
| `127.0.0.1` | `Float Dot Float` | `a_dotted_quad_still_lexes_as_float_dot_float` |
| `p.1.2` | `Word Dot Float` | `a_numeric_field_path_is_refused_by_name` |

The third row is the one place the float rule does reach this construct, and the
reader requires a `Word` after the `Dot` and refuses it **by name**, saying so in
the message. **No lexer rule changed.**

Spacing is immaterial — `p . x` lexes identically to `p.x` — so the reader
admits both. This is deliberately *unlike* `SOCKET-INET-ADDRESSES`' addresses,
which require lexical adjacency: there, adjacency is what separates an address
position from a name position, and here nothing needs separating because the
token *kinds* already decide it.

### 2.5 Immutable in v1, and what that buys

There is no field write. `p.x = 5;` is not a statement of this revision
(`AGGREGATE-FIELD-WRITE`), and the decision is honest rather than incidental: a
construct-then-read record is what CT09's access pattern needs, it makes
`construct` total and therefore `project` total, and it keeps the whole of the
aggregate's semantics in two functions a Lean model can state exactly.

Mutation is cheap to add on this model — a record owns nothing, so a field write
would move a value in a map and emit one member assignment — and it is still a
separate card, because it adds a rule to *every* composition (what a mutation in
a loop body means for an outer record, what it means for the certificate's
published construction) and this card has none of those to pay for.

Distinctness does **not** become vacuous without mutation: the fixture
constructs one value with three different numbers and reads all three back, so a
model or a lowering that aliased two fields prints one value twice. §6 is that
measurement.

---

## 3. The field-custody decision, measured

The brief asked this to be the card's real design decision, and it is. The answer
is **v1 records have scalar fields only**, and the measurement is in the
ownership IR rather than in taste.

### 3.1 What the tree has

| Machinery | Present on `f122cc03`? |
|---|---|
| an owner named by a plain `String` in `OwnershipOp` | yes — every one of them |
| an owner named by a **path** (`p.buf`) anywhere | **no** |
| a partial-move rule (moving one field out of an aggregate) | **no** |
| a drop-through-aggregate rule | **no** |
| a leak rule at normal exit that sees *into* an aggregate | **no** — `NormalPathLeak` reports live owner **names** |

So an owned field is not a field type away. `check_ownership` cannot be *asked*
about `p.buf`, because the question has no spelling in the IR it consumes. An
owned field needs four rules, none of which exists, and landing them behind a v1
that cannot yet even read a field would have been the wrong order.

### 3.2 The decision, and how it is checked rather than asserted

A v1 record contributes **no ownership operation at all**. That is pinned from
three sides:

* in Rust, `a_record_contributes_no_custody_operation` asserts
  `custody_operations` of a construct-and-read program is **empty**;
* in Lean, `a_record_only_program_presents_no_event` and
  `a_record_only_program_leaves_the_ledger_unchanged` say the same thing against
  `QCore.StdBuffer`'s own ledger;
* and from the accusing side, `a_record_does_not_launder_a_leak` shows a buffer
  leak beside a record still refusing at `Q054` with the inherited checker's own
  message.

The consequences fall out rather than being added: `drop p;` refuses at `Q052`
with *"is not a buffer this entry acquired"* — this route's **existing** rule,
not one this card wrote (`a_record_cannot_be_dropped`) — and a record at a
rendering operand, an index or a stored byte refuses at `Q052` too, with a
message naming the field read that would fix it.

### 3.3 A view field was considered separately and also deferred

A `string` field is a smaller step than an owned one — a `{storage, offset, len}`
view is not a custody obligation — but it is not free: `source.string.v1`'s
borrow **window** (constraint 15h) runs from the borrow to the last statement
reading the view *or anything derived from it*, and a view stored in a record
makes "derived from it" reach through an aggregate the window computation cannot
see into. That is a real rule, not a type-table entry. `AGGREGATE-VIEW-FIELD`.

### 3.4 The effect row says the same thing

A construction and a field read derive **nothing** — no `Alloc`, no `Mut`, no
`Io` — so a record-only entry may declare the empty `effects { }` row and be
admitted (`a_record_derives_no_effect`). An owned field would make the first of
those false, which is one more reason it is a different card. The Lean contrast
`an_acquisition_beside_it_still_derives_alloc` keeps the claim informative
rather than a fact about the function.

---

## 4. Check-time discipline

Every field value is check-time-known, for the reason the whole family shares:
this route's evidence is a **byte comparison between a reference execution and a
native run**, and a value the reference interpreter cannot compute is a value it
cannot predict a line for. That is `source.buffer.v1`'s decidability rule
(`Q052`), inherited unchanged.

Two rules are the card's own and both live in `qcore_core::source_record`:

* **Type formation** (`admit_type`): a record declares 1 to 32 fields and no two
  may share a name. The bound is there for the reason `MAX_EXTENT` is — every
  field of every constructed value is published in the certificate and emitted
  as a distinct C member, so a record the model cannot enumerate is not modelled
  at all — and it is a **refusal, never a truncation**.
* **Construction totality** (`construct`): every declared field is supplied
  exactly once and nothing else is. This is what makes `project` total on the
  declared names, which is what makes the field-access rule a function rather
  than a partial one with an error arm.

The module's type table is built **total-or-reject**: a module carrying one
declaration `admit_type` refuses gets no table at all, so no later rule can read
a type the model did not admit.

### 4.1 The certificate publishes both halves

`BufferCertificate` gains `constructions` and `projections`, beside the existing
`accesses`. A reader sees every record the program built with the value supplied
for each declared field, and every field it read with the value that read
produced — without re-deriving anything. Both are compared field for field by
`recheck_buffer_certificate`, whose two new failure variants are pinned
(`a_tampered_projection_fails_the_recheck`,
`a_tampered_construction_fails_the_recheck`).

Constructions are published in **declared** order rather than written order, so
two certificates of one program compare field for field however the source wrote
them. The fixture's second value is written `{ weight: 9, y: 2, x: 1 }` and
published `x, y, weight`
(`the_certificate_publishes_every_construction_in_declared_order`).

### 4.2 Emission

One C struct per declared type, one member per field in declared order, one
designated initializer per construction, one member read per field read:

```c
struct qcore_record_Point {
    int32_t f_x;
    int32_t f_y;
    int32_t f_weight;
};

struct qcore_record_Point r_p = { .f_x = INT32_C(65), .f_y = INT32_C(66), .f_weight = s_base };
int32_t s_a = r_p.f_x;
```

The `f_` and `qcore_record_` prefixes are load-bearing: a QCore field or type
name is a QCore word and nothing stops it being a C keyword, so `type int { … }`
emits a compilable struct rather than a syntax error. `r_` is a third binding
prefix beside `b_` and `s_`, so a record, a buffer and a scalar of one source
name are three distinct C objects (`the_three_binding_prefixes_are_distinct`).

The construction and the field read are emitted by `emit_events` rather than by
the published `buffer_event_statement`, and that function **refuses** them with
`UnemittableRecord` — exactly the `UnemittableLoop` shape card `LOOP-V1`
established, for its reason: emitting either needs the record environment (which
C object a binding names, which type it has) that `buffer_event_statement` does
not carry, and returning `None` would have been the one fail-open shape
available, silently emitting nothing for a construction and leaving every later
field read addressing an object that was never declared.

### 4.3 One emission change outside the aggregate, and why it is byte-neutral

A program that acquires **no** buffer now carries **no** buffer prelude — not
even the `qcore_bytes_new`/`qcore_bytes_drop` pair, which was previously
unconditional. This was forced: the helpers are `static`, the emitted C is built
with `-Wall -Wextra -Werror`, and an unused `static` function is an error, so a
record-only module would not have compiled.

It is byte-neutral for every previously admitted program, and that is a
measurement rather than a hope: among *admitted* programs, "reads or writes a
byte" **implies** "acquires a buffer", because a store, a load or an extent read
naming nothing this entry acquired is already refused at `Q052`. So no program
that reached the emitter before could have failed the new test. §7 measures it
over the whole tree, and `a_buffer_program_still_carries_its_prelude` and
`a_module_without_a_record_carries_no_record_fragment` pin both directions
in-suite.

---

## 5. Diagnostics: zero new codes

Under the `Q-NAMESPACE-DECISION` ice box no card allocates new `Q` codes, and
this one allocates none. Every refusal lands on a condition of the admitting
route's existing `Q051`–`Q057` band (the `StandardBuffer` pass), **widened and
stated** in §L21 and here rather than taken silently — the
`SOCKET-EVENT-COMPOSE` / `SOCKET-INET-ADDRESSES` precedent. The mapping lives in
`RecordRefusal::code` and nowhere else.

| Code | Existing condition | Widened to also cover |
|---|---|---|
| `Q051` | the flat entry **shape** this profile models is violated | a record declaration outside the form; a field declared without its `:` or with a type this revision does not model; a construction outside the form; a construction supplying a field twice, an undeclared field, or not every declared field; a field value that is not exactly one operand; a numeric field path |
| `Q052` | an operand that is not a check-time constant this entry bound, **or a custody position naming no buffer** | a field read naming something this entry constructed no record for; a field the record's type does not declare; a record at a position that takes an integer |
| `Q053` | an index, byte or extent **range** violation | a field count outside `1..=32` |

Three refusals deliberately land **outside** this band, each at an existing code
that says the truer thing:

* a construction of an **undeclared type** is `Q013` (name resolution), because
  a type name is an ordinary reference (§2.3);
* a **buffer** at a field position is `Q055`, this route's existing crossing
  rule;
* a **leak** beside a record is `Q054`, the inherited ownership checker's.

**Why not a new pass.** Card `LOOP-V1` took a new pass (`Loop`) for a construct
that decides *how many times a body runs*, which no bundled module's band could
state. This construct decides *what a name denotes and what a position takes* —
which is what `Q051`, `Q052` and `Q053` already are — and it is admitted on one
route, whose band therefore already describes what it decides. Adding a pass
would also have needed a range, and there is none: the `Q0xx` space below `Q100`
is exhausted, which is the ice box's whole premise.

`Q056` (a failed certificate rederivation) gains **two more ways to fire** — the
recheck now compares the published constructions and projections as well — and
its **condition is unchanged**, because "the published certificate failed its
independent rederivation" already describes exactly that. It is listed here
rather than in the table above for that reason: a code whose condition still
reads true of a new case is not a widening, it is the condition working.

---

## 6. The payoff: CT09's scenario, executed

`compiler/tests/fixtures/source_record/point.qcore` declares three fields,
constructs one value with three *different* numbers, reads all three back and
prints them; then constructs a second value of the same type and reads two of
its fields in a different order.

```
stdout: 65, 66, 40        stderr: [INFO] 9, [INFO] 1
```

That is CT09's *"distinct ordinary struct fields"* made executable. A model that
aliased two fields, or a lowering that gave two fields one C member, would print
one value twice — so the byte comparison is a **check** of distinctness rather
than a restatement of it.

`the_emitted_program_matches_the_reference_per_stream` compiles the emitted C
under `-std=c11 -Wall -Wextra -Werror -pedantic -g -O0
-fsanitize=address,undefined -fno-omit-frame-pointer` with
`ASAN_OPTIONS=detect_leaks=1:halt_on_error=1` and
`UBSAN_OPTIONS=halt_on_error=1:print_stacktrace=1`, and compares stdout against
the reference's stdout projection and stderr against its stderr projection.

The sanitized run is a **negative allocation control** rather than a positive
leak control, and saying which matters: the program allocates nothing at all, so
LSan reporting a leak would mean the aggregate had acquired something the model
says it cannot.

### 6.1 What a CT lane can now map, and what it cannot

**This card edits no CT manifest** — that is a mapping lane's work, and growing
a denominator without adding evidence is what constraint 23 forbids. What it
leaves for such a lane:

* CT09 (`"Distinct ordinary struct fields"`, expected `"Accept only after
  layout/access proof"`) now has the three things it was missing: a **route**
  (`source.buffer.v1`, with `source.record.v1` admitted on it), a **source
  fixture shape** that expresses the scenario, and a checked model whose
  artifact a named workspace test actually runs — parse, elaboration, checking,
  C emission and a sanitized native execution, all on one fixture. Which of the
  manifest's stages a lane may then read as `executed` is that lane's judgment
  against constraint 23's condition — *a named workspace test runs the stage on
  the case's own artifact* — and this card deliberately does not pre-empt it.
* The **`proof` stage is not discharged** by this card and a lane must not read
  it as one. `QCore.SourceRecord` proves the *access* half — construction
  totality, projection soundness, distinctness — and proves the *custody* half
  is empty. It proves **nothing about C layout**: that a C compiler's designated
  initializer assigns each member the value the model says is cross-checked by
  the sanitized byte comparison, not proved. CT09's "layout proof" is therefore
  **open**, and the honest reading is that this card supplies the *access* proof
  and an executed layout observation, not a layout proof (`AGGREGATE-CT09-LAYOUT`).

---

## 7. Route admission and byte identity, both measured

### 7.1 Route admission

v1 admits the construct on `source.buffer.v1` only, and every other route's
outcome is measured rather than widened:

| Route | Outcome | Pinned by |
|---|---|---|
| `source.buffer.v1` | **admits** — with or without a buffer, and with no `import std.buffer;` | `a_record_only_module_reaches_the_buffer_route` |
| composed string/buffer | `Q068` — that route's own "binds an integer literal, a quoted literal, `len(NAME)` or one slice; nothing else" | `the_composed_string_route_refuses_a_record_statement` |
| scoped tasks | `Q500`/`Q600` — the concurrent route is selected above this card's guard | `a_task_module_declaring_a_record_keeps_its_own_refusal` |
| `source.extern-call.v1` | that route's own refusal, selected above | `an_extern_module_declaring_a_record_keeps_its_own_refusal` |
| sequential, std-io, event, socket, HTTP | unreached with a declaration (the guard routes to the buffer route); unchanged without one | `a_module_with_no_record_declaration_still_reaches_the_sequential_route` |

**The route-selection change is one condition: a record type declaration.** A
module that constructs a value of a type it does not declare fails the guard and
keeps the route it had (`a_construction_without_a_declaration_keeps_the_route_it_had`),
and **no program written before this card can satisfy the guard at all**,
because `type NAME { … }` did not parse. That is the `STRING-CONCAT` property —
one new door, opened only by one new spelling.

A record construct **outside the entry** is refused rather than left unmodelled
(`a_record_outside_the_entry_refuses_rather_than_going_unchecked`), the same
rule `LOOP-V1` applies to a `for`, and for invariant 1's reason.

### 7.2 Byte identity, measured against main rather than asserted

The base commit's compiler was rebuilt in a scratch tree
(`measure/byte_identity.sh`, `git archive f122cc03`) and used to emit the C of
every `.qcore`/`.qc` file under `compiler/tests/fixtures`, `examples` and
`crates` that emits at all. The card build was then run over the same inputs
**from its own tree with the same relative spelled path** — standing constraint
11, and card `LOOP-V1` §10.3's measurement hazard, avoided by construction —
and the outputs compared byte for byte:

```
byte-identical: 80   differ: 0   did-not-emit-either-side: 111   absent-in-card-tree: 0
```

**All 80 programs that emit are byte-identical**, including every program that
holds a buffer (§4.3's prelude change moves nothing for them).

### 7.3 Composition with the bounded counted loop

A record is body-local exactly as a scalar and a buffer binding are — one
scoping rule over every kind of binding this route holds, rather than a second
rule:

* a record constructed in a loop body does not outlive it
  (`a_record_constructed_in_a_loop_body_does_not_outlive_it`);
* a body binding shadowing a record the entry made refuses at the loop's own
  `Q097` (`a_loop_body_shadowing_a_record_is_refused`);
* the body runs once per iteration and the record is rebuilt each time, with the
  index as a field value
  (`a_record_in_a_loop_body_is_constructed_once_per_iteration`).

A **loop bound** read out of a field is refused, deliberately: the value is one
the checker computes and the lowering does not, which is character for character
`LOOP-BOUND-FROM-BYTE`'s shape. `AGGREGATE-BOUND-FROM-FIELD`.

---

## 8. Deviations and findings

1. **The obvious spelling was rejected by the census, not by taste** (§1.1). An
   archived probe fixture *is* `record R { }` and two archived reports pin it to
   `Q000`.
2. **The card reserves no word and changes no line of name resolution** (§2.3).
   That was discovered by measurement, not planned: both positions the construct
   introduces were already drained.
3. **The buffer prelude had to become conditional** (§4.3), and that is an
   emission change outside the aggregate. It is byte-neutral for every
   previously admitted program by an argument about which programs the emitter
   could reach, and §7.2 measures it.
4. **`buffer_event_statement` refuses the two record events rather than
   returning `None`**, which is why the composed string/buffer route cannot emit
   a record. It also cannot *check* one — it refuses at `Q068` before emission —
   so the refusal is unreachable in practice and is written for the fail-closed
   direction alone (`AGGREGATE-STRING-COMPOSE`).
5. **A cascade is possible and is not suppressed.** A refused construction
   leaves its binding unbound, so a later `p.x` earns a second `Q052`. Every
   refusal test in the suite is written so that nothing reads the bad binding,
   which is what makes each test measure the rule it names; the cascade itself
   is left alone, because suppressing it would need a notion of "poisoned
   binding" this route does not have (`AGGREGATE-REFUSAL-CASCADE`).
6. **A record type name or field name that is a reserved source word still
   works** — `type Frame { drop: i32, close: i32, print: i32 }` declares and
   constructs — because a field name is never collected as a reference and the
   statement dispatches on its `let` head. **Six words are the exception**, and
   they are the two whole-token-stream route selectors' (§1.4): a record naming
   `spawn`, `join` or one of the four buffer/payload intrinsics refuses at
   `Q015` when it is declared and at `Q000` when it is constructed. The
   `NATIVE-LIBRARY-SOURCE-BINDING` exclusion was measured and **deliberately
   not copied**, because it would retire the `Q015` half and leave the `Q000`
   half — a type that declares and cannot be constructed
   (`AGGREGATE-SELECTOR-WORDS`). All three refusals are pinned.
6a. **A `module library;` file declaring a record refuses at `Q058`**, the
   buffer route's own entry-free refusal, because the route guard sends it
   there. Measured, fail-closed, and recorded rather than repaired: a record
   type that outlives its file is a *module system* question and this revision's
   routes all model one entry (`AGGREGATE-LIBRARY-MODULE`).
7. **A trailing comma is admitted** in both the declaration and the
   construction, because the two spellings differ in no token that means
   anything. Both are pinned.
7a. **A measurement-hygiene finding worth publishing, because it cost a full
   workspace run.** The byte-identity measurement (§7.2) extracts the base
   commit into `measure/base/`, which is worktree-private and gitignored exactly
   as `CLAUDE.md` requires. It is nonetheless **visible to a test that walks the
   filesystem**: card `SOCKET-INET-ADDRESSES`'
   `this_card_reserves_no_word_and_its_address_rule_collides_with_nothing`
   enumerates every `.qcore` file under the worktree root and asserts the
   address-host reader claims a span in exactly two of them — and the extracted
   copy of `examples/sockets/` made that four. Nothing about this card is
   implicated; the run was invalidated by its own measurement apparatus, in the
   `LOOP-V1` §11.1.2 sense. **A baseline tree must be removed before the gate
   runs**, or kept outside the worktree entirely. Gitignoring it is not enough,
   because a test that walks a directory does not consult `.gitignore`.
8. **No file under `crates/qcore-c-gateway/`, `crates/qcore-audit/`,
   `crates/qcore-contract/`, `contract/qcore/` or `providers/` was touched**, and
   `qcore-schemas` is untouched. The one file under `crates/qcore-verification/`
   that changed is the route-selection guard, which is three lines plus its
   comment.

---

## 9. Evidence

### 9.1 Tests

`compiler/tests/source_record.rs` — **56 tests**, including one sanitized native
run. One new fixture, `compiler/tests/fixtures/source_record/point.qcore`.

Every refusal case is written inline against the same declarations, so each
measures the rule it names. No pinned expectation was edited and no existing
fixture moved. `compiler/tests/std_buffer.rs` (52 tests) and
`compiler/tests/source_loop.rs` (39 tests) are unchanged but for the one
`records: Vec::new()` field each of their three hand-built `BufferProgram`
literals now carries.

### 9.2 Lean

`proof/QCore/SourceRecord.lean` (new) and `proof/AuditSourceRecord.lean` (new) —
**28 audited entries**. `QCore.lean` imports the module, so `AuditAll.lean`
includes it in the whole-project axiom closure. **No new axiom, no placeholder,
no `native_decide`** — every audited theorem depends on nothing, or only on the
allowed `propext` and `Quot.sound`. `check_lean_audits.py` moves **110 → 111**.

Load-bearing statements, by family:

* **Type formation decides distinctness** — `admitType_iff` states the rule as
  one equivalence, so a widening must change a theorem rather than slip past
  one, with `a_repeated_field_name_is_refused` and
  `a_record_of_distinct_fields_is_admitted` as the two controls.
* **Construction totality, and the projection totality it buys** —
  `projection_is_total_on_declared_fields` is the consequence a reader needs: a
  constructed value has a value for **every** field it can be asked for, so the
  field-access rule is a function and not a partial one with an error arm.
* **Field access soundness** — `projection_returns_what_construction_supplied`
  is the headline, and it is a theorem rather than a restatement because the
  constructed value is rebuilt in *declared* order out of the supplied
  assignment rather than being that assignment.
  `two_distinct_fields_read_their_own_values` is the same fact stated the way
  CT09 asks for it, with `a_two_field_record_constructs_and_projects` as a
  worked control so neither is vacuous.
* **The refusals as one list** — `refused_constructions_are_exactly_these` and
  `complete_iff`, the `QCore.StdIo.the_operand_forms_are_exactly_these`
  convention.
* **The field-custody decision, proved** — `a_record_item_presents_no_event`,
  `a_record_only_program_presents_no_event` and
  `a_record_only_program_leaves_the_ledger_unchanged`, stated against
  `QCore.StdBuffer`'s own ledger so it is the same ledger the route inherits.
* **Neutrality** — `a_record_free_sequence_presents_itself` against a
  `recordFree` predicate written independently of the erasure, with
  `a_record_item_is_not_record_free` making the claim non-vacuous.
* **Effects** — `a_construction_derives_nothing`, `a_field_read_derives_nothing`
  and the contrast `an_acquisition_beside_it_still_derives_alloc`, which is what
  keeps "derives nothing" a fact about the aggregate rather than about the
  function.

**Not proved**, carried over from the family's convention: that the Rust
lowering extracts a declaration and a construction faithfully from source tokens
(a tested Rust boundary); that a C compiler's designated initializer assigns each
member the value the model says (that is what the sanitized native byte
comparison cross-checks — and it is why CT09's *layout* proof is open, §6.1); and
anything about a field this model gives no type, which is absent by construction.

### 9.3 Corpus neutrality

**Corpus-neutral by construction.** No file under `crates/qcore-c-gateway/`,
`crates/qcore-audit/`, `crates/qcore-contract/`, `contract/qcore/` or
`providers/` is touched, and `qcore-schemas` is untouched. The whole diff is:

```
crates/qcore-core/src/source_record.rs           (new: the aggregate's own model)
crates/qcore-core/src/std_buffer.rs              (two events, the type table, the gates)
crates/qcore-core/src/std_buffer_c_emit.rs       (the struct fragment and the two statements)
crates/qcore-core/src/std_string.rs              (two exhaustive matches, fail-closed)
crates/qcore-core/src/std_string_c_emit.rs       (one exhaustive match, fail-closed)
crates/qcore-core/src/lib.rs                     (one module declaration)
crates/qcore-frontend/src/source_record.rs       (new: the grammar and the census)
crates/qcore-frontend/src/source_std_buffer.rs   (the lowering into the route)
crates/qcore-frontend/src/frontend.rs            (DeclarationKind::Record, the `type NAME {` branch)
crates/qcore-frontend/src/lib.rs                 (one module declaration)
crates/qcore-verification/src/driver.rs          (the route-selection guard)
compiler/tests/source_record.rs                  (new)
compiler/tests/fixtures/source_record/           (new)
compiler/tests/{std_buffer,source_loop}.rs       (one struct field each, three literals)
docs/QCORE_SPEC_r0_3.md                          (§L21, one amendment to the Q051–Q057 row)
docs/QCORE_IMPLEMENTED_GRAMMARS_r0_3.md          (§G07.13, new)
proof/QCore/SourceRecord.lean, proof/AuditSourceRecord.lean, proof/QCore.lean
docs/reports/aggregates-v1/{README.md,census.txt,gate.txt}
```

The contract-corpus pin (73 modules), the 185-unit artifacts, the leak count of
**266** and the CT manifest are main's, and none of them moves for anything here.
Nothing is promoted; no `Unknown` became `Safe`.

---

## 10. Gate

| Gate step | Result |
|---|---|
| `git submodule update --init --recursive` | run in this worktree before the closing measurement (checked out `d82464a277d0f42703702c4dfd9af6af38595a83`) |
| `cargo test --workspace --no-fail-fast` | **4,265 passed, 0 failed, 8 ignored** across 210 suites, **exit 0** (§10.1) |
| `cargo test -p qcore-compiler --test source_record` | **56 pass**, including the sanitized native run |
| `cargo test -p qcore-compiler --test std_buffer` | **52 pass**, unchanged |
| `cargo test -p qcore-compiler --test source_loop` | **39 pass**, unchanged |
| `cargo fmt --all -- --check` | clean |
| `cargo clippy --all-targets -- -D warnings` | clean |
| `cargo run -p qcore-compiler --example reevaluation_probe` | exit **0** |
| `cargo run -p qcore-compiler --example reevaluation_followup` | exit **0** |
| `python3 scripts/check_lean_audits.py` | **111/111**, one new audit file, no new axiom |
| `python3 -m unittest discover -s scripts/tests` | **193 tests, 1 failure** — the expected derived-value tripwire (§10.2) |

Command evidence is in [`gate.txt`](gate.txt) beside this report, and each step's
exit code is written to its own file so no wrapper can lose it (the `STD-BUFFER`
§10.5.1 hazard) — `measure/gate.sh`, `measure/workspace.sh` and the `*.exitcode`
files beside them in the worktree-private `measure/` directory, never under
`target/` and never in the shared scratchpad.

### 10.1 The closing run

**4,265 passed, 0 failed, 8 ignored, across 210 suites reporting
`test result: ok`, exit 0.** The log contains **zero** lines beginning `error`
and **zero** `test result: FAILED` blocks. The exit status is **read rather than
inferred**: `measure/workspace.sh` writes it with `echo $? >` into
`measure/workspace.exitcode`, so no shell wrapper stands between the cargo
process and the recorded number.

**The arithmetic reconciles exactly.** `AGENTS.md`'s wave-26 close records
**4,209** workspace tests with 8 ignored. 4,209 + 56 = **4,265**, and 56 is
precisely this card's new suite — so the card adds its own tests and moves no
other count, which is what corpus neutrality looks like from the test side.

Two environment findings, recorded rather than hidden:

1. **One run was invalidated by this card's own measurement apparatus and
   stopped deliberately** (§8.7a): the byte-identity baseline tree under
   `measure/base/` made card `SOCKET-INET-ADDRESSES`' whole-tree census test see
   four `.qcore` files where it pins two. No number above comes from that run;
   the baseline was removed and the whole suite re-run from the start.
2. **Machine contention.** Four sibling agents were running workspace tests
   against neighbouring worktrees for the whole of the closing measurement, so
   wall-clock timings are not comparable with a quiet run. Nothing about the
   results depends on that.

**None of standing constraint 16's five load-sensitive rows tripped.** No
assertion was weakened and no test was re-run in isolation to obtain a pass.

### 10.2 The one expected failure

`scripts/tests`' `test_the_derived_facts_are_recomputed_rather_than_carried`
trips on `validation.lean_audits`, which moves 110 → 111. This card's brief
forbids editing `docs/reviews/r0_3/acceptance_matrix.json`, so the row is
deliberately left stale for the wave integrator to regenerate — the handling
every wave's derived sections receive. The delta is entirely this card's one
audit file.

---

## 11. For the integrator

* **Zero `Q` codes allocated.** The `Q051`–`Q057` band's `Q051`, `Q052` and
  `Q053` conditions are widened, stated in §L21 and in §5 above. The
  `Q-NAMESPACE-DECISION` ice box is respected and unchanged.
* **Zero reserved words.** Standing constraint 14's inventory gains one more
  "added none" row, with the rejected spelling (`record`) and its archive
  collision recorded.
* `AGENTS.md`'s ledger's Lean-audit count moves **110 → 111**. **Not edited** —
  `AGENTS.md` is integrator surface and this card's brief forbids it.
* `docs/reviews/r0_3/acceptance_matrix.json` carries derived
  `validation.lean_audits` and `validation.lean_audit_files` rows checked by
  `scripts/tests`, so the derived-value tripwire
  `test_the_derived_facts_are_recomputed_rather_than_carried` trips on its
  `lean_audits` count. This card's brief forbids editing the matrix, so it is
  deliberately left stale for the wave integrator to regenerate — the handling
  every wave's derived sections receive. The delta is entirely this card's one
  audit file.
* `qcore_frontend::frontend::DeclarationKind` gained one variant. Every existing
  `match` over it was extended, not defaulted.
* `qcore_core::std_buffer::{BufferEvent, BufferError, BufferRecheckFailure}`
  gained variants and `BufferProgram`/`BufferCertificate`/`BufferStep`/
  `BufferEvaluation` gained fields; `BufferEmissionRefusal` gained one variant.
  Same — every `match` extended, and every one of the new arms in the *string*
  route is written in the fail-closed direction.
* **No standing constraint is touched.** No gateway, audit, contract-loader or
  provider file changed; no pinned expectation was edited; no fail-closed gate
  was weakened.
* **`examples/` is untouched**, by the brief: a docs card owns examples this
  wave. A published `examples/records/` family held to `EXAMPLES-SUITE`'s
  standard is the named follow-on `EXAMPLES-AGGREGATES`.

---

## 12. Named follow-ons

Each names its missing capability, because that is what makes it a boundary
rather than a backlog item.

| Name | What it is, and what it needs |
|---|---|
| `AGGREGATE-OWNED-FIELD` | a field that owns a resource (a `bytes(N)` buffer). Needs four rules none of which exists: a **field-path owner** vocabulary in the cycle-3 ownership IR (whose owners are plain names), a partial-move rule, a drop-through-aggregate rule, and a leak rule at normal exit that sees into the aggregate |
| `AGGREGATE-VIEW-FIELD` | a `string` field. Smaller than the above — a view is not a custody obligation — but needs `source.string.v1`'s borrow **window** (constraint 15h) to reach through an aggregate, which its exact interval computation cannot do today |
| `AGGREGATE-FIELD-WRITE` | `p.x = 5;`. Cheap on this model, but adds a rule to every composition: what a mutation in a loop body means for an outer record, and what the certificate's published construction means once a value can change |
| `AGGREGATE-FIELD-TYPES` | a field of a type other than `i32`. Needs the admitting route to hold a value of that type; every scalar it holds today is an `i32` |
| `AGGREGATE-NESTED` | a record field whose type is another record. Needs a recursive construction and projection rule, and a termination argument for the declaration graph |
| `AGGREGATE-ACROSS-CALL` | a record-valued parameter or result. Needs a source-level call with a non-scalar argument, which no route on this family has |
| `AGGREGATE-ROUTE-WIDENING` | admitting the construct on the string, event, socket, HTTP, extern, task, mutex or epoch routes. Each is a separate checked model; each would inherit `source_record`'s formation and totality rules rather than restate them |
| `AGGREGATE-STRING-COMPOSE` | the composed string/buffer route specifically, which refuses a record at `Q068` today (§8.4) |
| `AGGREGATE-BOUND-FROM-FIELD` | a `for` bound read out of a field — known to the checker, not to the lowering. Exactly `LOOP-BOUND-FROM-BYTE`'s shape and needing its answer |
| `AGGREGATE-REFUSAL-CASCADE` | a refused construction leaves its binding unbound, so a later field read earns a second diagnostic. Needs a poisoned-binding notion this route does not have (§8.5) |
| `AGGREGATE-LIBRARY-MODULE` | a `module library;` file declaring a record type, refused at `Q058` today (§8.6a). Needs a record type that outlives its file, which is a module-system question this revision's one-entry routes do not pose |
| `AGGREGATE-SELECTOR-WORDS` | a record whose type name or field name is one of the six whole-token-stream route-selector words refuses — `Q015` declared, `Q000` constructed (§1.4). Needs the task-expression reader to stop claiming a `WORD:` at a statement position, which is a concurrency-route change; excluding only the declaration span would buy a type that declares and cannot be constructed |
| `AGGREGATE-CT09-LAYOUT` | CT09's *layout* proof. This card supplies the **access** proof and an executed layout **observation**; that a C compiler's designated initializer and member read agree with the model is cross-checked by the byte comparison, not proved |
| `EXAMPLES-AGGREGATES` | a published `examples/records/` family with a README, held to `EXAMPLES-SUITE`'s standard. This card ships a test fixture with that evidence but publishes no example, and the two are not the same deliverable — the `LOOP-EXAMPLE` precedent |

---

## 13. Draft increment paragraph

> The first source-level aggregate type (AGGREGATES-V1): `source.record.v1`
> lands `type NAME { FIELD: i32, … }`, `let p = NAME { FIELD: OPERAND, … };` and
> `let a = p.FIELD;` on `source.buffer.v1`, with every other route's refusal
> measured and pinned rather than widened. **The card's design decision is field
> custody, and it was measured rather than argued**: a v1 record is a plain
> copyable value whose fields are all `i32`, so it contributes **no** ownership
> operation, opens no obligation and derives no effect — because the cycle-3
> ownership IR addresses an owner by a plain **name** and has no notion of a
> path, so `p.buf` is not a question `check_ownership` can be asked, and an owned
> field needs a field-path owner vocabulary, a partial-move rule, a
> drop-through-aggregate rule and a leak rule that sees into the aggregate, none
> of which exists (`AGGREGATE-OWNED-FIELD`, `AGGREGATE-VIEW-FIELD`). The
> consequences are the *existing* route's rules rather than new ones: `drop p;`
> refuses at `Q052` with "is not a buffer this entry acquired", and a leak beside
> a record still refuses at `Q054` with the inherited checker's own message.
> **Zero reserved words**, and the constraint-14 census is why: the obvious
> spelling `record` was **rejected on evidence**, because
> `docs/reports/remaining-v07/probes/d02_boundary_record.qcore` is the single
> line `record R { }` and two archived reports pin it to `Q000`, so claiming the
> word would have falsified a frozen archive (constraint 21). The head is the
> already-reserved `type`, claimed at the one shape a `{` after the name gives
> it, and the census found **zero** bare `type NAME {` in the tree — all eleven
> are `resource type`, which an earlier branch claims. **Zero new diagnostic
> codes**: every refusal is a widened condition of the admitting band's `Q051`,
> `Q052` and `Q053`, stated in §L21 under the `Q-NAMESPACE-DECISION` ice box.
> **No line of name resolution changed either**, and that too was a measurement:
> a field name sits before a `:` at a construction and after a `.` at a read, and
> both positions have been drained since long before this card, so a construction
> of an undeclared type refuses at `Q013` as an ordinary unknown reference. The
> `.` adjacency is pinned rather than assumed — `p.x` is `Word Dot Word` and
> cannot collapse into a float, so `SOCKET-INET-ADDRESSES`' `Float Dot Float`
> reading of `127.0.0.1` is untouched, and a *numeric* field path is one float
> token and refuses by name. Records emit real C structs, one distinctly named
> member per field, and the payoff is **CT09's own scenario executed**: three
> declared fields, three different values, three reads, byte-compared against a
> sanitized ASan/UBSan/LSan run — so a lowering that gave two fields one member
> would print one value twice. 56 tests, 28 audited Lean entries in one new audit
> file, no new axiom, 110 → 111 audits. Corpus-neutral by construction, and byte
> identity is **measured rather than asserted**: the base commit's compiler was
> rebuilt in a scratch tree and all **80** emitting programs compare
> byte-identical. Evidence: `docs/reports/aggregates-v1/README.md`.
