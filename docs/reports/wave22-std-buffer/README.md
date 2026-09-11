# STD-BUFFER — `std.buffer`, a checked owned byte buffer (wave 22)

Card brief: *"QCore has NO string type, zero string literals, and its one buffer
model carries a single `i32` payload with no extent. The std.http direction needs
a source-level byte-buffer tier: `std.buffer` — a checked, owned,
extent-carrying byte buffer. Design-first, census what exists, then land v1."*

Spine head of the `std.http` direction. Base commit: main `ec5535a9`.

Design note written first, as the brief required:
[`docs/design/STD_BUFFER_V1.md`](../../design/STD_BUFFER_V1.md).

**What landed.** A source-level route `source.buffer.v1` behind
`import std.buffer;`: `let b = bytes(8);` acquires eight zeroed bytes,
`b[0] = 65;` writes one, `let x = b[0];` reads one, `let n = extent(b);` binds
the length as an ordinary integer, `drop b;` releases. Emission is
self-contained C11 — `calloc`, `free` and two bounds-checked accessors defined
in the emitted file, no library and no boundary of any kind — compared against
the reference execution per stream under ASan/UBSan/LSan with a **positive** leak
control. **Two reserved words**, `bytes` and `extent`; a new semantic pass
`StandardBuffer` owning **`Q051`–`Q057`**. 52 tests, 38 audited Lean
entries in one new audit file, no new axiom.

**The headline is the census result (§1), and it is not what the brief
expected.** The extent capability the tree lacked was never a missing data
structure. `Footprint` — the ownership IR's exact componentwise resource
quantity — has carried one since cycle 3, `OwnershipState::live()` aggregates it
across the ledger, and `check_ownership` refuses a normal exit that leaves any of
it live. What no source program could do was **acquire on a byte axis**.
`bytes(N)` is exactly that construct, and the extent it transports is
`Footprint::single("bytes", N)`. So v1 adds no transport mechanism; it spends one
the language had and had never spelled.

**Custody is inherited, not reimplemented** (the `PIPE-CHAIN` precedent): every
leak, double release, access-after-release and rebound-live-name refusal on this
route is `qcore_core::ownership::check_ownership`'s, over an operation list this
route derives. §3 shows the refusal arriving as an `OwnershipError` value, which
is what makes "inherited" a checkable claim rather than a description.

---

## 1. The census

Read-only, over `crates/qcore-core/src`, `crates/qcore-frontend/src` and the
source-side `.qcore` corpus, asking one question: **what byte-addressable,
extent-carrying storage does a QCore source program already have?**

| # | Candidate | Extent? | Byte-addressable? | Reachable from a sequential source program? |
|---|---|---|---|---|
| B1 | task-route `Buffer` (`qcore_buffer_new/read/write`) | **no** — "this backend's `Buffer` has no extent" (`crates/qcore-frontend/src/source_epoch_tasks.rs:49-53`) | no — one `i32` payload | no: the intrinsics require `import std.concurrency;` and route to the task model |
| B2 | an extern `*mut u8` through a declared C04 contract | **no** — `ExternAssumption::NoExtentTransport` is a standing premise of every extern certificate | only inside the callee, which QCore does not analyse | yes, but every byte is the callee's premise, not a checked fact |
| B3 | `OwnershipOp::Acquire`'s `Footprint` | **yes** — `Footprint(BTreeMap<String, u64>)`, an exact componentwise quantity (`crates/qcore-core/src/ownership.rs:9`) | no — it is a quantity, not storage | the IR is reachable; **no source construct produced a byte-axis acquisition** |
| B4 | `SourceTypeKind::Array` / `Slice` | type-level names only | no | no value of either type is constructible in any source grammar |
| B5 | `IndexTerm::size_of(TypeName)` | a static type-level symbol | no | not spellable in any source grammar |
| B6 | a string or character value | — | — | **none exists**: `STD-MODULES` measured zero string and zero character literals outside comments across the whole source-side corpus, and the atomic type list is textless |

**The result is B3**, and it is why the implementation is as small as it is. The
line `Footprint::single("bytes", extent)` is the whole of the extent transport,
and `check_ownership` is the whole of the custody discipline; what this card
wrote is the *source construct* that reaches them, plus the three rules about
indices that had no prior art because nothing in the tree had ever indexed
anything.

### 1.1 Relation to the two prior `std.buffer` statements

`docs/design/NATIVE_LIBRARY_LINKAGE_V1.md` §6 names `std.buffer` as the first
target of a **native-library-backed** module, staged at §12(c), and records the
blocker: extent is not transported, so a `std.buffer` worth recommending needs
the buffer-extent follow-on, its stage (b).

This card is **not** stage (c) and does not become it. The two are different
tiers of one name, and §1 of the design note tabulates the difference in full;
in one line: this tier is for bytes that are the program's own and QCore checks
every access, that tier is for bytes that belong to a library and QCore checks
the custody protocol at the boundary. **Neither retires the other**, no part of
stage (a) or (c) is retired here, and the C04 vocabulary is untouched — making
*contracts* carry an extent remains that document's stage (b), recorded here as
`STD-BUFFER-ABI-EXTENT`.

The task-route `Buffer` (B1) is **not modified, renamed or related** to this
type by any conversion. That is a deliberate boundary, not an oversight:
relating them means deciding whether a byte buffer can cross a task boundary,
which is the task ledger's parent/child custody question, and `STD-MODULES` §8
already measured that the ledger has no post-creation transfer operation. It is
the follow-on `STD-BUFFER-TASKS`.

---

## 2. The v1 surface

```ebnf
stmt    ::= "let" NAME "=" [ "-" ] INTEGER ";"
          | "let" NAME "=" "bytes" "(" INTEGER ")" ";"     (* acquire        *)
          | "let" NAME "=" "extent" "(" NAME ")" ";"       (* bind the length *)
          | "let" NAME "=" NAME "[" OPERAND "]" ";"        (* read one byte   *)
          | NAME "[" OPERAND "]" "=" OPERAND ";"           (* write one byte  *)
          | "drop" NAME ";"                                (* release         *)
          | "print" RENDERING OPERAND ";"                  (* std.io + std.fmt *)
          | "log" LEVEL RENDERING OPERAND ";"              (* std.logger + std.fmt *)
          | "flush" ";"                                    (* std.logger      *)
          | "return" ";"
OPERAND ::= [ "-" ] INTEGER | NAME
```

Also `docs/QCORE_IMPLEMENTED_GRAMMARS_r0_3.md` §G02k, added in the same change.
The shipped fixture is `compiler/tests/fixtures/std_buffer/bytes.qcore`; the
silent counterpart is `silent.qcore`.

### 2.1 The decidable-observation rule, and why it is not conservatism

Every `OPERAND` is a **check-time constant**: an integer literal, or a name this
entry bound to one — which on this route includes a name bound by `extent(b)` or
by `b[i]`, because the checker computes both. A non-constant operand refuses at
`Q052`.

The reason is the one every prior card in this family respected. This route's
evidence is a **byte comparison between a reference execution and a native run**;
a value the reference interpreter cannot compute is a value it cannot predict a
line for, so a run-time-varying index would leave *the* observation ill-defined
— the same wall `PIPE-SCALAR-RESULT` §4.3 hit for an address. The missing
capability is a relational domain over indices, and it is named
`STD-BUFFER-VARYING-INDEX`.

The brief offered the `succeeds INTEGER` verified-outcome pattern for lengths.
It was **not needed and is not used**: a length written at the acquisition site
is already a literal, so there is no declared outcome to compare against
reality. The pattern's real subject on this route is allocation *failure*, which
§4.2 handles differently and states as a boundary.

### 2.2 Effects: derived, not premised

| Event | Derives |
|---|---|
| `bytes(N)`, `drop b` | `Alloc` |
| `b[i] = v` | `Mut` |
| `b[i]`, `extent(b)` | — (a read observes) |
| `print` / `log` / `flush` | `Io` |

`source.std-io.v1`'s derived row is exactly `{Io}` because its operand grammar
structurally admits no resource. This route's does admit one, so its row carries
`Alloc` and `Mut` — pinned by `the_derived_effect_row_carries_alloc_and_mut`, and
its negative counterpart `a_silent_buffer_module_derives_no_io_and_needs_no_output_module`
pins that a program which never renders needs no output module at all.

**A correction made during the card, because the obvious claim is false.** An
earlier draft called this the first source route whose derived row is neither
`{Io}` nor empty. It is not: `source.extern-call.v1` derives `Alloc` and `Mut`
too (`crates/qcore-core/src/extern_call.rs:1146-1166`) — `Alloc` when
`callee.acquires()`, `Mut` when a parameter is a mutable pointer or a custody
position. What is actually different is **where the facts come from**. There they
are read off a *declared contract*'s projections, so they are consequences of a
premise in the `ContractTrust::Declared` sense; here they follow from an
acquisition and a store this route checks and discharges itself. That is the
true statement, and it is the one now carried in the spec, the grammars, the
model's doc comment and the test.

---

## 3. Custody: inherited, and shown to be

`custody_operations` maps the event sequence onto the cycle-3 IR —
`Acquire`/`Write`/`Read`/`Drop` — and `check_ownership` decides it. The
attribution is `Q054` and the message carries the checker's own error text.

| Refusal | Decided by | Test |
|---|---|---|
| leak at normal exit | `ownership` → `NormalPathLeak` | `a_leak_refuses_and_the_refusal_comes_from_the_unedited_ownership_checker` |
| double release | `ownership` → `DoubleDrop` | `a_double_release_refuses` |
| write after release | `ownership` → `UseAfterDrop` | `an_access_after_release_refuses` |
| read after release | `ownership` → `UseAfterDrop` | `a_read_after_release_refuses` |
| extent read after release | `ownership` → `UseAfterDrop` | `an_extent_read_after_release_refuses` |
| one of two released | `ownership` → `NormalPathLeak` | `releasing_only_one_of_two_still_refuses` |
| **bounds** | **this card** | five tests, §4.1 |
| **byte range** | **this card** | `a_stored_byte_outside_the_range_refuses_rather_than_truncating` |
| **decidability** | **this card** | three tests, §4.1 |

The first test is the one that makes "inherited" checkable rather than
descriptive: beside compiling a leaking module and asserting `Q054`, it builds
the same `BufferProgram` directly, calls `custody_operations`, and asserts

```rust
ownership::check_ownership(&operations)
    == Err(OwnershipError::NormalPathLeak(vec!["b".to_owned()]))
```

so the refusal is seen arriving from the unedited checker as that checker's own
value, not as a re-worded copy.

**One deliberate narrowing, and it weakens nothing.** Only names some `Acquire`
event binds contribute an operation. The ownership checker is asked about
*resources*; a name that is not one is not a resource whose custody it could
decide, and asking anyway made it answer `UnknownBinding` — a fact about the
operation list rather than about the program. Such a name now refuses at `Q052`
(`NotABuffer`), which says the true thing. Every refusal that *is* about custody
concerns a name the checker was told about and still arrives from it.

**The published quantities.** The certificate carries `acquisitions` as
`(name, extent)` pairs — the extent transport, published, so a reader sees each
declared length without re-deriving anything — and `peak_live_bytes`, taken from
`OwnershipState::live()`'s own byte axis after each prefix rather than counted
here. `two_buffers_are_both_accounted_for` pins that two live buffers sum
(2 + 3 = 5), because the footprint is componentwise over the whole ledger.

---

## 4. The rules this card does own

### 4.1 Bounds, ranges and decidability

All three are decided **before emission**, so an out-of-range access is a
refusal (`Q053`) and not a run-time trap. Both sides of every boundary are
pinned, so each rule is the rule it names rather than something that happens to
refuse the cases tried:

| Boundary | Refused | Admitted |
|---|---|---|
| index vs. extent | `b[4]` on `bytes(4)`, `b[99]`, `b[-1]` | `b[3]` on `bytes(4)` |
| stored byte | `256`, `-1` | `255` |
| declared extent | `bytes(0)`, `bytes(4097)` | `bytes(1)`, `bytes(4096)` |
| index decidability | `b[a]` where `a` is a buffer (`Q055`) | `b[i]` where `i` is a bound integer |

`an_index_bound_out_of_range_still_refuses` pins that indirection through a `let`
is not a way around the check: a name is a check-time constant, so the bounds
rule applies to it exactly as to a literal.

### 4.2 The trap discipline, stated

The emitted prelude aborts on an out-of-range index and on a failed `calloc`.
**Both are witnesses, not checks.**

* Every index in an admitted program is a constant the checker already proved in
  range, so the emitted comparison is provably never taken. It is emitted anyway
  because it costs nothing, because it makes the generated C readable as a
  standalone artifact, and — the point that matters — because **if it ever
  fired, that would be a defect in this checker**. A sanitized run that
  completes is therefore evidence about the checker and not only about the
  program.
* `calloc` can fail, and v1 aborts rather than modelling a failure arm. Every
  claim on this route is therefore **partial-correctness**: *if the emitted
  program produces an observation at all, it is the reference observation.* An
  aborting run produces none and so contradicts none. Modelling the arm needs
  `source.branch.v1`'s join, since the route is flat and a failure arm would
  have nowhere to go — `STD-BUFFER-ALLOC-FAILURE`.

`abort()` is not catchable and yields no QCore error value. Neither trap is a
recovery mechanism and neither makes a varying index admissible.

**`calloc` and not `malloc`**, and this is not an optimisation detail: the
model's buffer is zero at acquisition, so a read before a write has a defined
answer. `malloc` would leave the bytes indeterminate — undefined behaviour to
read natively, unpredictable in the reference — and the per-line comparison this
route rests on would be unwritable. The fixture reads an untouched byte and
prints `0` precisely to exercise that agreement, and the emission test asserts
the string `malloc(` does not occur.

---

## 5. What does not cross

**No buffer crosses the extern boundary in v1, and the answer is a refusal
rather than an omission.** A C04 contract's pointer parameter is an extentless
`u8` projection (`NoExtentTransport`), so passing an extent-carrying buffer
through one would **discard the fact this card adds** and replace it with a
premise. A model that transports an extent and then hands it to a boundary that
cannot carry it is worse than one that refuses, because the refusal is visible.

The mechanism is route selection, and that is the honest shape: this route is
selected **after** the extern route, so a module that buffers *and* calls a
declared extern function stays with the extern route and keeps **that route's
existing refusal**. No code of this card's is involved
(`a_module_that_buffers_and_extern_calls_stays_with_the_extern_route` asserts no
`Q05x` appears). The future path is the manifest of
`NATIVE_LIBRARY_LINKAGE_V1.md` §4 plus its stage (b) — `STD-BUFFER-EXTERN`.

| Position | Result | Test |
|---|---|---|
| `print dec b` (a buffer as a rendering operand) | `Q055` | `rendering_a_buffer_refuses_as_a_boundary_crossing` |
| `print dec borrow b` | `Q055` | `a_borrowed_render_refuses_because_this_route_publishes_no_identity` |
| `b[a] = 1` (a buffer as an index) | `Q055` | `a_buffer_as_an_index_refuses_as_a_boundary_crossing` |
| `b[0] = a` (a buffer as a stored byte) | `Q055` | `a_buffer_as_a_stored_byte_refuses_as_a_boundary_crossing` |
| a buffer to a declared extern callee | the **extern route's** refusal | `a_module_that_buffers_and_extern_calls_stays_with_the_extern_route` |
| a buffer inside a `thread scope` | the **concurrent route's** refusal (`Q600`) | `a_module_that_buffers_inside_a_thread_scope_keeps_the_concurrency_refusal` |

The borrowed-render row is `STD-FMT-BORROW`'s own structural refusal applying
here for exactly its own reason: a borrowed render reads an identity a
*custody-model-publishing* route supplies, and this route publishes none in v1.
Admitting one would need a published ledger and a liveness rule, which is a
strictly larger question — recorded, not attempted.

### 5.1 Route placement

Selected after every concurrent route and after the extern route, and **before**
`source.std-io.v1`. Both neighbours are load-bearing:

* after the extern route → the crossing question above;
* before the standard-IO route → a module that both buffers and prints reaches
  this route, which models both. Its line-producing events are
  `source.std-io.v1`'s **own**: read by that route's own `std_io_statement`
  reader and rendered by that route's own `event_statement` emitter, so "this
  route's lines are the standard-IO route's lines" is a statement about code
  rather than about two implementations that agree. A module that only prints
  fails the conjunction and falls through byte for byte
  (`a_printing_module_without_a_buffer_still_selects_the_standard_io_route`).

That is also the answer to the brief's "if a `print` rendering of a buffer
byte/length is cheap, it may land": **it landed, and it was free.** Because
`extent(b)` and `b[i]` bind ordinary integer scalars, rendering them needs no
new operand form, no new rendering and no change to `std_io.rs` at all — the
existing `Scalar` operand already denotes them.

---

## 6. No string in v1

A byte buffer is not a string, and the boundary is recorded rather than implied.
v1 has no textual type, no string or character literal, no encoding, no
concatenation and no text rendering. `STD-MODULES` measured that the language has
**zero** string and character literals outside comments across the source-side
corpus and that its atomic type list is textless; nothing here changes that.

Named follow-ons: **`STD-BUFFER-STRING-LITERAL`** (needs a textual type in the
atomic list, a literal form the lexer admits outside `verifier scope` metadata,
and an encoding decision — three separate surfaces, none of which exists) and
**`STD-BUFFER-RENDER`** (rendering a buffer's *contents* as text, which needs
that encoding decision first: without one there is no fact about what a byte
sequence *says*).

---

## 7. Deviations found by measurement

Nine, each a place the implementation disagreed with the design note, with a
claim an earlier draft of this report made, or with a standing rule it had to be
checked against.

1. **`buffer` collides, and the constructor was renamed.** The natural spelling
   occurs **555 times** across the source-side corpus. Parameter-name
   occurrences are already shielded — the referenced-names filter skips a word
   followed by `:` — but it also occurs in **binding position** in shipped,
   pinned fixtures: `let buffer = qcore_buffer_new(1)?;` and `drop buffer;` in
   `examples/threads/shared_readers.qcore`, and `fn worker(buffer: own<Buffer>…)`
   in `examples/threads/owned_buffer.qcore`. Reserving it would refuse those
   files at `Q013`. Per the `work` → `effort` (T08-EPOCH-SRC) and `error` →
   `fatal` (STD-MODULES) precedent, **this card's syntax was renamed rather than
   the pinned surface edited**: the constructor is `bytes`. Both directions are
   pinned as tests —
   `the_buffer_spelling_refuses_because_buffer_is_not_this_cards_word` (the
   unreserved word refuses at `Q013`) and `buffer_is_still_an_ordinary_binding_name`
   (the fixtures' spelling still compiles, here as a buffer binding). The
   **module** keeps the name `std.buffer`: a module name is a dotted import path
   and never a reserved word.

2. **`alloc` is a worse collision than `buffer` and was never a candidate.** It
   is a live **effect-set member** (`effects { io, alloc, mut }`, 89 uses) as
   well as an extern-fn name and a `quant axis` name, so reserving it would move
   a pinned effect keyword. Recorded so a later card does not re-derive it, along
   with `size`, `len`, `free`, `store`, `release`, `make`, `allocate` and `span`,
   which each collide as extern-function or parameter names. Free and unused:
   `byte`, `capacity`, `at`, `index`, `get`, `put`, `new`, `slice`, `load`.

3. **An index naming nothing refuses at two different places, depending on the
   statement.** Name resolution collects a statement's referenced words, but for
   an `Assignment` it first *drains every token before the `=`*. So the index of
   a byte **write** (`b[nope] = 1;`) is never collected and never reaches
   `Q013`; this route's own `Q052` catches it. The index of a byte **read**
   (`let x = b[nope];`) sits after a `let`'s `=`, is collected, and refuses at
   `Q013` before any lowering — the measurement `PIPE-SCALAR-RESULT` §2.3,
   `STD-IO-COMPOSE` §6.2 and `STD-FMT-BORROW` §7.1 each recorded for their own
   routes. Both halves are pinned by
   `an_index_naming_nothing_refuses_at_the_route_in_a_write_and_earlier_in_a_read`,
   so a change in either direction says so. The useful consequence is that
   `Q052` is reachable for a reason the design did not anticipate.

4. **Declaring an extern function is not being an extern module.** Route
   selection asks whether a file *calls* a declared extern function, so the first
   version of the crossing test — which declared one and never called it —
   reached the buffer route rather than the extern route. That is correct
   behaviour (declaring a symbol crosses nothing), the test was corrected to call
   the callee, and the surprising direction is pinned separately by
   `declaring_an_extern_function_without_calling_one_is_not_an_extern_module`.

5. **A refused extent must still bind its name.** `bytes(0)` first produced
   `["Q053", "Q052"]`: the acquisition was refused and its name left unbound, so
   the later `drop b;` cascaded into a second, misleading refusal. Errors on this
   route are collected rather than fatal, so the acquisition now registers its
   name anyway. The program is refused either way; what this buys is that the
   report names the one thing wrong with it.

6. **The emitted prelude had to become selective.** Emitting all four helpers
   unconditionally failed `-Werror=unused-function` on the silent fixture, which
   never reads a byte. The prelude is now the header plus the lifetime pair plus
   each accessor the program actually uses. This is a real constraint of the
   gate the emitted C is built under (`-Wall -Wextra -Werror`, `/W4 /WX` on
   MSVC), recorded in the prelude's own comment so a later card does not
   reintroduce the unconditional form.

7. **The "first route to derive `Alloc`" claim was false and was corrected.**
   An earlier draft of this report, of the spec row, of the grammar section, of
   the model's doc comment and of a test comment all called this the first
   source route whose derived effect row is neither `{Io}` nor empty.
   `source.extern-call.v1` derives both
   (`crates/qcore-core/src/extern_call.rs:1146-1166`), so the claim was simply
   wrong. It is replaced everywhere by the true and more useful one: those
   effects are read off a **declared contract**'s projections there, and follow
   from an acquisition and a store **this route discharges itself** here. §2.2.

8. **The undeclared-effect refusal was emitting another pass's code.** An
   earlier version reported `StandardIo`'s **`Q029`**, on the same
   reuse-rather-than-duplicate reasoning that correctly governs `Q015`. It is a
   §L21 violation: that rule permits reuse *within* an owning pass — the import
   gate runs as `NameResolution` and emits from its range — but forbids a
   **second semantic pass** taking a code, and `StandardBuffer` is one. The
   conditions differ besides: `Q029` reports an undeclared `Io` on a route that
   derives nothing else, while this one can report an undeclared `Alloc` or
   `Mut`. The refusal now has this pass's own `Q057` and the band is
   `Q051`–`Q057`.

9. **The Lean registry's structure literals grew a field, and one theorem's
   meaning is now stronger.** `Imports` went from six fields to seven; sixteen
   anonymous literals grew with it. `QCore.StdIo.no_module_substitutes_for_another`
   says "each row holds every module except the one it needs", so its new
   `std.buffer` column is `true` — its name and meaning are preserved and the
   claim is strictly stronger than before. This is the same deliberate widening
   `STD-MODULES` recorded when it took the registry from two modules to five.
   Also corrected in passing: the spec's §L04 registry sentence read "exactly
   five names" and had been stale since `UNSAFE-LINKAGE` added
   `std.extern.unsafe`; it now names all **seven**.

---

## 8. Reserved words: the constraint-14 census

**Two words added: `bytes` and `extent`.** Everything else in the grammar reuses
a word or a symbol the language already has.

| Construct | Spelling | Words added |
|---|---|---|
| acquire | `bytes(N)` | **1** (`buffer` collides — §7.1) |
| extent | `extent(b)` | **1** |
| release | `drop b;` | **0** — already reserved, and the operation *is* `OwnershipOp::Drop`, so a new `release` word would have been a synonym for one the language already spells |
| write / read | `b[i] = v;` / `let x = b[i];` | **0** — `[` and `]` are already lexed and are consumed today only in a *usage-interval* position (`[lo, inf]`) inside type and quantity annotations, never in a statement body. `b[0] = 65;` parses as an ordinary `StatementKind::Assignment` and `let x = b[0];` as an ordinary `Let`, with **no parser change at all** |
| lines | `print` / `log` / `flush` / `dec` / `hex` / `info` / `warn` / `fatal` | **0** — `STD-MODULES`'s, reused unchanged |

Both new words were censused against every `.qcore` file and against QCore source
embedded in Rust string literals: each occurs **only inside `//` comments**, if
at all. Both are load-bearing in the measured sense `grow` and `rank`
established — name resolution collects the tokens after a `let`'s `=`, so an
unreserved word there refuses at `Q013`, which is exactly what
`the_buffer_spelling_refuses_because_buffer_is_not_this_cards_word` shows
happening to the unreserved `buffer`, and
`the_two_new_words_are_reserved_and_reach_no_name_resolution` shows *not*
happening to these two.

---

## 9. Diagnostics

A new semantic pass `StandardBuffer` owning **`Q051`–`Q057`**, allocated through
the §L21 process in the same change as the implementation. The census confirmed
`Q051`–`Q099` entirely unallocated; `Q050` (`ComposedTasks`) was the last taken.

| Code | Condition |
|---|---|
| `Q051` | the flat entry shape is violated — a statement outside the grammar, a nested block, a construct outside the entry, a parameterised entry, a malformed construct, a missing return, a malformed `effects` row |
| `Q052` | an operand that is not a check-time constant this entry bound, or a custody position naming no buffer |
| `Q053` | a bounds or range violation — index outside `[0, extent)`, byte outside `0..=255`, extent outside `1..=4096` |
| `Q054` | a refusal reported by the **inherited** ownership checker |
| `Q055` | a buffer at a position it may not cross |
| `Q056` | the published certificate failed its independent rederivation |
| `Q057` | the entry performs a derived effect its declared `effects { … }` row does not admit |
| `Q015` | **reused**, not duplicated: "this file used a capability it did not import" is exactly what it already means, and the message names the missing module |

**No existing code changed meaning.** `Q020`/`Q021` stay separate (standing
constraint 2). `Q015` is reused on the `STD-MODULES` principle — a new code
meaning the same thing would be a second allocation for one condition — and that
reuse is *within* its owning pass, because the import gate runs as
`NameResolution` and emits from `NameResolution`'s range.

**`Q057` is the one place that principle does not apply, and the card initially
got it wrong** (§7.9). An earlier version emitted `StandardIo`'s **`Q029`** for
the undeclared-effect refusal, on the same reuse-rather-than-duplicate reasoning.
That is a §L21 violation: the rule is that a code may not be reused by a *second
semantic pass*, and `StandardBuffer` is one. The conditions differ besides —
`Q029` reports an undeclared `Io` on a route that can derive nothing else, while
this row can carry an undeclared `Alloc` or `Mut`. The refusal now has this
pass's own code.

A separate pass from `StandardIo` on purpose: that pass admits a closed set of
statements with no resource and no custody at all, while this one admits a
resource acquisition, an indexed access and a release. Separate from `Ownership`
on purpose too: its `Q103` is deliberately **not** emitted even though every
custody refusal here is the ownership checker's, because that checker is
*inherited* here rather than run as the sequential resource-flow bundle runs it
— a reader following `Q054` back should find this route's lowering, not that
bundle. The refusal is the same one; the attribution says which route asked.

A collision discovered at integration relocates this **unshipped** band, per the
wave-3 `T07`/`T09` precedent; the code-to-condition mapping is stable under such
a shift.

---

## 10. Evidence

### 10.1 Tests

`compiler/tests/std_buffer.rs` — **52 tests, all passing**, including two
sanitized native runs. Fixtures:
`compiler/tests/fixtures/std_buffer/bytes.qcore` (allocate, three writes, an
extent read, four byte reads, five rendered lines, a barrier, release) and
`silent.qcore` (allocate, one write, release, no output).

Every refusal case is written inline against the same declarations, so each
measures the rule it names. No pinned expectation was edited and no existing
fixture moved.

### 10.2 Native evidence — a positive leak control

`the_emitted_program_matches_the_reference_per_stream` compiles the emitted C
with `-std=c11 -Wall -Wextra -Werror -pedantic -g -O0
-fsanitize=address,undefined -fno-omit-frame-pointer`, under
`ASAN_OPTIONS=detect_leaks=1:halt_on_error=1` and
`UBSAN_OPTIONS=halt_on_error=1:print_stacktrace=1`, and compares **stdout
against the reference's stdout projection and stderr against its stderr
projection**. No cross-stream order is asserted.

**This is a positive leak control, which is the point.** Both fixtures allocate,
so LSan has something to find; had the release been lost, or a store run past
the end, LSan or ASan would have said so rather than these tests passing. That
is a stronger form of evidence than `STD-MODULES`'s, whose fixtures allocated
nothing and whose leak check was therefore a negative control. Apple ASan
provides no LSan, so leak detection stays a Linux-only observation.

The comparison is **byte-level about the buffer**, not only about the lines: the
fixture prints `b[0]`, `b[1]`, `b[7]` and an untouched `b[5]`, so the native
run's printed numbers must equal the ones the model computed out of its own byte
vector. The untouched byte printing `0` is what makes `calloc` a checked
decision rather than a stated one.

### 10.3 Lean

`proof/QCore/StdBuffer.lean` (new) and `proof/AuditStdBuffer.lean` (new) —
**38 audited entries** — 28 theorems of the new module plus the ten new rows
`proof/QCore/ModuleImports.lean` gained.
`QCore.lean` imports `QCore.StdBuffer`, so `AuditAll.lean` includes it in the
whole-project axiom closure. **No new axiom, no placeholder, no `native_decide`**
— every audited theorem depends on nothing, or only on the allowed `propext` and
`Quot.sound`.

Load-bearing statements:

* **the extent is transported** — `the_extent_read_back_is_the_extent_declared`
  (what `extent(b)` reads back is what `bytes(N)` declared, out of the ledger),
  with `a_store_moves_no_extent`, `a_load_moves_no_extent`,
  `an_extent_read_moves_no_extent`, `a_line_moves_no_extent` and their general
  form `non_custody_events_leave_the_ledger_unchanged`;
* **the bounds discipline** — `an_admitted_index_is_inside_the_extent`,
  `an_index_into_nothing_is_refused`, `an_index_with_no_value_is_refused` (the
  decidability rule, stated as what `STD-BUFFER-VARYING-INDEX` would have to
  change), `the_last_index_is_admitted_and_the_extent_itself_is_not`,
  `a_negative_index_is_refused`, `the_byte_range_is_exactly_a_byte`,
  `the_extent_range_is_bounded_at_both_ends`;
* **the custody discipline** — `a_release_removes_exactly_its_own`,
  `a_release_keeps_every_other_entry`, `an_empty_ledger_holds_no_bytes`,
  `a_held_buffer_is_live_bytes`, `an_acquisition_adds_exactly_its_extent`,
  `two_live_buffers_hold_the_sum`;
* **the effect discipline** — `an_acquisition_derives_alloc`,
  `a_store_derives_mut`, `a_read_derives_nothing`,
  `a_silent_buffer_program_derives_no_io`;
* **the operand wall** — `the_operand_forms_are_exactly_two`, written as one
  list so a widening changes a theorem rather than slipping past it (the
  `QCore.StdIo.the_operand_forms_are_exactly_these` convention), and
  `an_operand_is_never_a_custody_act`;
* **the module gate** — `admitted_buffer_requires_its_module`, an instantiation
  of the **same** generic `admitted_capability_requires_its_module` every other
  module is gated by, so the seventh cannot drift from the six; plus
  `the_io_import_alone_grants_no_buffer`,
  `the_buffer_import_alone_grants_no_stream`,
  `concurrency_grants_no_buffer_capability` and
  `neither_extern_module_grants_the_buffer`.

**Not proved**, carried over from the family's convention: that the Rust
lowering extracts these events faithfully from source tokens (a tested Rust
boundary), that `calloc` gives the zeroed block the model assumes (a C-standard
premise), and anything about the failing-allocation arm (§4.2's
partial-correctness boundary is stated, not proved away). The ownership checker
itself is not re-proved here either — `QCore.Cycle3` and its neighbours own the
custody rules, and this module states what the buffer route's operation list
gives them.

### 10.4 Corpus neutrality

**Corpus-neutral by construction.** No file under `crates/qcore-c-gateway/`,
`crates/qcore-audit/`, `crates/qcore-contract/`, `contract/qcore/` or
`providers/` is touched, and `qcore-schemas` is untouched. The whole diff is:

```
crates/qcore-core/src/std_buffer.rs             (new: the model)
crates/qcore-core/src/std_buffer_c_emit.rs      (new: reference execution + C11)
crates/qcore-core/src/lib.rs                    (two module declarations)
crates/qcore-core/src/module_imports.rs         (the registry name and its accessor)
crates/qcore-frontend/src/source_std_buffer.rs  (new: grammar and lowering)
crates/qcore-frontend/src/lib.rs                (one module declaration)
crates/qcore-frontend/src/module_imports.rs     (the import gate)
crates/qcore-frontend/src/frontend.rs           (two reserved words, the pass, its code)
crates/qcore-verification/src/driver.rs         (the checked variant, profile and route)
compiler/tests/std_buffer.rs                    (new)
compiler/tests/fixtures/std_buffer/{bytes,silent}.qcore (new)
docs/design/STD_BUFFER_V1.md                    (new)
docs/QCORE_SPEC_r0_3.md, docs/QCORE_IMPLEMENTED_GRAMMARS_r0_3.md
proof/QCore/{StdBuffer,ModuleImports,StdIo}.lean, proof/AuditStdBuffer.lean, proof/QCore.lean
docs/reports/wave22-std-buffer/README.md
```

The contract-corpus pin (69 modules), the 185-unit artifacts, the leak count of
**272** and the CT manifest are main's, and none of them moves for anything here.
The expectation to verify at consolidation is that the canonical CMetrics
artifacts and the 185-unit baseline are **byte-identical**, with the headline
unchanged at 2 Safe / 42 provisional Unsafe / 137 Unknown / 4 data-only.

`a_module_using_none_of_this_card_emits_byte_identically` pins the emission side
of that: the route is selected only when a buffer construct is present, so an
existing module travels the identical code path.

### 10.5 Gate

| Gate step | Result |
|---|---|
| `git submodule update --init --recursive` | run in this fresh worktree before the closing measurement |
| `cargo test --workspace --no-fail-fast` | **3,395 passed, 0 failed, 8 ignored** across 190 suites, doctests included, from one complete run. The **exit status was not observed** and is not claimed — see §10.5.1 |
| `cargo test -p qcore-compiler --test std_buffer` | **52 pass**, including both sanitized native runs |
| `cargo fmt --all -- --check` | clean |
| `cargo clippy --all-targets -- -D warnings` | clean; one lint of this card's own (`needless_lifetimes`) was found and **fixed, not suppressed** |
| `cargo run -p qcore-compiler --example reevaluation_probe` | exit **0** |
| `cargo run -p qcore-compiler --example reevaluation_followup` | exit **0** |
| `python3 scripts/check_lean_audits.py` | **91/91**, one new audit file, no new axiom |
| `python3 -m unittest discover -s scripts/tests` | **193 tests, 1 failure** — the expected derived-value tripwire, see §10.6 |

Full command evidence is in `gate.txt` beside this report. Measurement outputs
are worktree-private under `measure/` (gitignored), never under `target/` and
never in the shared scratchpad.

#### 10.5.1 What the workspace figure does and does not assert

The run reached and finished its final phase — the merged doctests — and the log
ends with that summary. It contains zero lines beginning `error`, zero
`failures:` blocks, and no `error: test failed`. All **190** suites reported
`test result: ok`. The figures were recounted from the finished log and are
identical to the first reading, so they are the whole run's rather than a
mid-run snapshot.

**The exit status was not observed, and is deliberately not claimed.** The shell
wrapper that would have recorded it did not survive to write its marker, and the
`cargo` process was gone by the time that was noticed, so no exit code was ever
read. An earlier version of this report and of `gate.txt` asserted "exit 0";
that assertion was unsupported and is **withdrawn rather than left standing**.
What is claimed is what the log shows: all 190 suites ran and all passed.

A related measurement hazard is recorded here because it cost real time and
would cost the next card the same: the waiting loops used to block on this run
were written as `while pgrep -f "cargo test --workspace"`, which **matches the
waiter's own command line** and therefore never terminates. Wait on a marker in
the log, not on a `pgrep` of a pattern the waiter itself contains.

**Three workspace runs were started and killed by this card before completing**,
and are recorded rather than hidden: each was invalidated by a source change
landing under it — an unused-helper trim, the derived-effect claim correction,
and the `Q029` → `Q057` reallocation, the last a real behaviour change. **No
number above comes from any of them.** The closing run was taken from the
settled tree at `b05687ad` after `fmt` and `clippy` were re-verified clean on
it, and it is one complete run rather than a composition of halves.

### 10.6 For the integrator

This card adds **one** `Audit*.lean` file, so the derived-value tripwire
`test_the_derived_facts_are_recomputed_rather_than_carried` trips on its
`lean_audits` count, exactly as the two preceding standard-IO cards did. This
card's brief forbids editing `docs/reviews/r0_3/acceptance_matrix.json`, so the
matrix is deliberately left stale for the wave integrator to regenerate — the
same handling every wave's derived sections receive. The delta is entirely this
card's.

Reconciliation list:

* **`Q051`–`Q057` allocated** to a new `StandardBuffer` pass. `Q058`+ are free
  as far as this card is concerned. A collision with a concurrent wave-22 card
  relocates this unshipped band per the `T07`/`T09` precedent;
* `AGENTS.md`'s diagnostics-namespace summary table needs one row for
  `Q051`–`Q057` → `StandardBuffer`, and its ledger needs the Lean-audit count
  moved. **Neither was edited** — `AGENTS.md` is integrator surface and this
  card's brief forbids it;
* `docs/QCORE_SPEC_r0_3.md` §L21 gained one row; §L04's registry sentence was
  corrected from a stale "exactly five names" to **seven** (the stale count
  predates this card — `std.extern.unsafe` was missing);
* `docs/QCORE_IMPLEMENTED_GRAMMARS_r0_3.md` gained §G02k;
* `qcore_core::module_imports::ModuleImports` gained one field and one accessor;
  `CheckedSourceModule` gained one variant, `SourceExecutionProfile` one, and
  `SemanticPass` one. Every existing `match` over them was extended, not
  defaulted;
* the Lean `Imports` structure grew from six fields to seven, so sixteen
  anonymous literals across `ModuleImports.lean`, `StdIo.lean` and
  `CapacityBoundary.lean` grew with it. Existing theorem names and meanings are
  preserved; `no_module_substitutes_for_another` is strictly stronger (§7.7);
* **no standing constraint is touched.** No gateway, audit, contract-loader or
  provider file changed; no pinned expectation was edited; no `Unknown` became
  `Safe`.

**None of the four `P02-FLAKE` rows (standing constraint 16) tripped** on the
closing run; no assertion was weakened and nothing was re-run in isolation to
obtain a pass.

---

## 11. What `STD-FMT-EXTENT` now needs

`STD-FMT-BORROW` §9 recorded it as: *"rendering a resource's size. Needs the
model to retain a declared extent — which it deliberately does not."*

This card supplies the missing capability **for its own resources and no
others**, and the distinction is the whole of the honest answer:

* On `source.buffer.v1` a size is renderable **today, with no rendering change
  at all** — `let n = extent(b); print dec n;` — because `extent(b)` binds an
  ordinary integer scalar and rendering one is `source.std-io.v1`'s existing,
  unedited operand form. No `borrow` marker is involved and no new operand
  constructor exists. The shipped fixture does exactly this and its native run
  prints `8`.
* On the **extern** route `STD-FMT-EXTENT` is **untouched**.
  `ExternAssumption::NoExtentTransport` is still a standing premise of every
  extern certificate, a declared C04 contract still cannot say how many bytes a
  callee touches, and nothing here changes that.

So `STD-FMT-EXTENT` is **not retired — it is split**, and what it now needs is
exactly `STD-BUFFER-ABI-EXTENT`: the C04 vocabulary change, which is a
contract-loader change and therefore
`NATIVE_LIBRARY_LINKAGE_V1.md`'s stage (b) rather than a rendering change.
Saying which half moved is more useful than claiming the name.

---

## 12. Named follow-ons

Each names its missing capability, because that is what makes it a boundary
rather than a backlog item.

| Name | What it is, and what it needs |
|---|---|
| `STD-BUFFER-STRING-LITERAL` | a string literal as a value. Needs a textual type in the atomic type list, a literal form the lexer admits outside `verifier scope` metadata, and an encoding decision — three separate surfaces, none of which exists |
| `STD-BUFFER-RENDER` | rendering a buffer's **contents** as text. Needs the encoding decision above first: without one there is no fact about what a byte sequence *says* |
| `STD-BUFFER-VARYING-INDEX` | an index the checker cannot constant-fold. Needs a relational domain over indices, and a comparison discipline for an observation the reference interpreter can no longer predict exactly |
| `STD-BUFFER-ALLOC-FAILURE` | a recoverable allocation failure. Needs `source.branch.v1`'s join, so the failure arm has somewhere to go |
| `STD-BUFFER-EXTERN` | passing a buffer to a declared extern callee. Needs the C04 vocabulary to carry an extent, or the pointer arrives extentless and the fact is discarded |
| `STD-BUFFER-ABI-EXTENT` | the C04 half of the above, stated separately because it is a **contract-loader** change and this card opens no loader. This is what `STD-FMT-EXTENT` now needs (§11) |
| `STD-BUFFER-TASKS` | a buffer crossing a task boundary, and any relation to the task-route `Buffer`. Blocked on the wall `STD-MODULES` §8 measured: the task ledger has no post-creation transfer operation |
| `STD-BUFFER-GROW` | reallocation. Needs a custody shape the ownership IR does not have — an acquisition that replaces a live one — which is `Reallocated` on the extern side and has no source spelling |
| `STD-BUFFER-SLICE` | a borrowed sub-range. Needs the ownership IR's borrow nodes to carry a footprint *fraction*, which they do not |
| `STD-BUFFER-COPY` | a bulk copy between two buffers. Needs no new custody shape, but needs the loop or the bulk operation the flat route has no statement for; `source.branch.v1`'s neighbours are where that lives |
| `STD-BUFFER-BORROW-RENDER` | a borrowed render on this route. Needs this route to publish a custody identity and a liveness ledger the join can read, which `STD-FMT-BORROW` built for the extern model and which this route has no v1 need for |
| `STD-BUFFER-EXAMPLE` | a **published** `examples/buffer/` family with a README, held to `EXAMPLES-SUITE`'s standard: every transcript extracted from the file and compared with a real sanitized run. This card ships test fixtures with that evidence but publishes no example, and the two are not the same deliverable — `compiler/tests/examples_suite.rs` covers five families and a sixth belongs with its README, not bolted onto a checker card |
| `STD-FMT-EXTENT` | **split, not retired** — see §11 |

---

## 13. Draft increment paragraph

> A checked owned byte buffer (STD-BUFFER), the spine head of the `std.http`
> direction: `source.buffer.v1` behind `import std.buffer;` — `let b = bytes(8);`
> acquires eight zeroed bytes, `b[0] = 65;` writes one, `let x = b[0];` reads one,
> `let n = extent(b);` binds the length as an ordinary integer, `drop b;`
> releases. **The census is half the result**: the extent capability the tree
> lacked was never a missing data structure. `Footprint` has carried an exact
> componentwise quantity since cycle 3, `OwnershipState::live()` aggregates it and
> `check_ownership` refuses a normal exit that leaves any live; what no source
> program could do was *acquire on a byte axis*. `bytes(N)` is that construct and
> the extent it transports is `Footprint::single("bytes", N)`, so v1 adds no
> transport — it spends one the language had and had never spelled. Custody is
> **inherited, not reimplemented** (the PIPE-CHAIN precedent): every leak, double
> release and access-after-release refusal is the unedited cycle-3 ownership
> checker's, shown arriving as its own `OwnershipError` value rather than a
> re-worded copy; what this card owns is the three rules about *indices*, because
> nothing in the tree had ever indexed anything. Every index, length and stored
> byte is a **check-time constant**, so the checker decides every bounds question
> before emission and a run-time-varying index is refused by name — the route's
> evidence is a byte comparison against a native run, and a value the reference
> interpreter cannot compute is one it cannot predict a line for. Emission is
> **self-contained** C11 (`calloc`, `free` and two accessors defined in the
> emitted file; `calloc` and not `malloc`, so a read before a write has a defined
> answer), and the emitted bounds and allocation checks are **witnesses**: every
> index is a constant already proved in range, so if one ever fired that would be
> a defect in the checker — which makes every claim here partial-correctness, *if
> the program produces an observation it is the reference observation*. The
> derived effect row carries `Alloc` and `Mut` — which the extern route derives
> too, but from a *declared contract*'s projections, where here they follow from
> an acquisition and a store the route discharges itself. **No buffer crosses the
> extern boundary**, and the
> refusal is the extern route's own rather than one this card wrote: a C04 pointer
> is an extentless `u8` projection, so the crossing would discard the extent and
> replace it with a premise. **No string in v1** — a byte buffer is not a string,
> and encoding, literals and text rendering are named follow-ons. `buffer` was
> measured to collide at 555 occurrences *in binding position* in shipped thread
> fixtures, so the constructor is spelled `bytes` on the `work` → `effort` and
> `error` → `fatal` precedent while the module keeps the name `std.buffer`; the
> release is the existing `drop` and the accessors the existing `[`/`]`, so the
> card adds **two** reserved words and no parser change. `Q051`–`Q057` under a new
> `StandardBuffer` pass through the §L21 process, `Q057` among them rather than a
> reuse of `StandardIo`'s `Q029`, because §L21 forbids a second pass reusing a
> code. 52 tests, one sanitized
> ASan/UBSan/LSan run as a **positive** leak control, 38 audited Lean entries in
> one new audit file, 91/91 audits, no new axiom. Gate: 3,395 workspace tests
> pass (0 failed, 8 ignored) across 190 suites, fmt and clippy clean, both probes
> exit 0; the workspace run's own exit status was not observed and is not
> claimed. `STD-FMT-EXTENT` is **split
> rather than retired**: a size is renderable today on this route with no
> rendering change at all, and what the extern half now needs is the C04
> vocabulary change `STD-BUFFER-ABI-EXTENT`. Corpus-neutral by construction.
> Evidence: `docs/reports/wave22-std-buffer/README.md`.
