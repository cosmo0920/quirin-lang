# A checked owned byte buffer: `source.buffer.v1` / `std.buffer`

> **Status: landed** by card `STD-BUFFER` (wave 22). This document is the
> design note the card wrote first; §11 records the eight places where the
> implementation deviated from it, because measurement moved them.

Card brief: *"QCore has NO string type, zero string literals, and its one
buffer model carries a single `i32` payload with no extent. The std.http
direction needs a source-level byte-buffer tier: `std.buffer` — a checked,
owned, extent-carrying byte buffer."*

## 0. The one-paragraph statement

`std.buffer` v1 adds a **source-level** owned byte buffer with a length that
the model carries: `let b = bytes(8);` acquires eight bytes, `b[0] = 65;`
writes one, `let x = b[0];` reads one, `let n = extent(b);` binds the length as
an ordinary integer, and `drop b;` releases. Every index, length and byte value
is a **check-time constant**, so the checker decides every bounds question
before emission and the reference interpreter can predict the program's exact
observation. Custody is not reimplemented: the route lowers to
`qcore_core::ownership`'s **unedited** cycle-3 IR, so the leak, double-release
and use-after-release refusals are that checker's, not this card's. Emission is
self-contained C11 — `calloc`/`free` plus two bounds-checked accessor functions
— with no extern boundary and no library. (`calloc` rather than the `malloc`
this note first wrote: the model's buffer is zero at acquisition, so a read
before a write must have a defined answer.)

**A byte buffer is not a string.** v1 has no encoding, no literal, no
concatenation and no text rendering; §9 names those as follow-ons rather than
leaving them implied.

## 1. Relation to `NATIVE_LIBRARY_LINKAGE_V1.md`

That document names `std.buffer` as the first target of a **native-library
backed** module (its §6, staged at §12(c)), and records the blocker plainly:
*extent is not transported*, because C04 pointers are extentless `u8`
projections, so a `std.buffer` worth recommending needs the buffer-extent
follow-on, its stage (b).

This card is **not** stage (c) and does not become it. It is a different tier of
the same name, and the honest reading of the two is:

| | `NATIVE_LIBRARY_LINKAGE_V1.md` §6 `std.buffer` | this document's `std.buffer` |
|---|---|---|
| Where the mechanism lives | an audited native library, bound by a manifest | the emitted C11 of the program itself |
| What crosses a boundary | a `*mut u8` at the C04 ABI projection | nothing — there is no boundary |
| Extent | not transported (`NoExtentTransport`) | **carried in the model**, as an ownership footprint |
| What QCore checks | the custody protocol at the boundary | the whole program, including every access |
| Trust | `ContractTrust::Declared` — the library's conformance is a premise | no premise about a callee, because there is no callee |

They are complementary and they do not compete: this tier is what a source
program uses when the bytes are *its own*, and the native tier is what it uses
when the bytes belong to a library. **Neither retires the other**, and this card
retires no part of stage (a)/(c). What it does supply is the *capability* stage
(b) was named for — an extent that a checked model transports — but it supplies
it **on the source side only**. Making the *C04 contract vocabulary* carry an
extent is untouched here and remains that document's stage (b), because it
changes the contract loader, which this card does not open. §9 records that as
`STD-BUFFER-ABI-EXTENT`.

## 2. Relation to the task-route `Buffer`

`crates/qcore-core/src/task_execution.rs`'s `Buffer` is the scoped-task
profile's model: one `i32` payload, no extent, reached through the
`qcore_buffer_new` / `qcore_buffer_read` / `qcore_buffer_write` intrinsics under
`import std.concurrency;`. It is **not modified by this card**, not renamed, and
not related to the type below by any conversion. The two models share no code
and no reserved word.

That is a deliberate v1 boundary and not an oversight: relating them means
deciding whether a byte buffer can cross a task boundary, which is the task
ledger's parent/child custody question, and `STD-MODULES` §8 already measured
that the ledger has **no post-creation transfer operation**. §9 records the
relation as `STD-BUFFER-TASKS`.

## 3. The census

Read-only, over `crates/qcore-core/src`, `crates/qcore-frontend/src` and the
source-side `.qcore` corpus, asking: **what byte-addressable, extent-carrying
storage does a QCore source program already have?**

| # | Candidate | Extent? | Byte-addressable? | Reachable from a sequential source program? |
|---|---|---|---|---|
| B1 | task-route `Buffer` (`qcore_buffer_new/read/write`) | **no** — "this backend's `Buffer` has no extent" (`source_epoch_tasks.rs:49-53`) | no — one `i32` payload | no: the intrinsics require `import std.concurrency;` and route to the task model |
| B2 | extern `*mut u8` through a declared C04 contract | **no** — `ExternAssumption::NoExtentTransport` is a standing premise of every extern certificate | only inside the callee, which QCore does not analyse | yes, but every byte is the callee's premise, not a checked fact |
| B3 | `OwnershipOp::Acquire`'s `Footprint` | **yes** — `Footprint(BTreeMap<String, u64>)` is an exact componentwise quantity | no — it is a quantity, not storage | the IR is reachable; **no source construct produces a byte-axis acquisition** |
| B4 | `SourceTypeKind::Array` / `Slice` | type-level names only | no | no value of either type is constructible in any source grammar |
| B5 | `IndexTerm::size_of(TypeName)` | a static type-level symbol | no | not spellable in any source grammar |
| B6 | a string or character value | — | — | **none exists**: `STD-MODULES` measured zero string and zero character literals outside comments across the whole source-side corpus, and the atomic type list is textless |

**The census result is B3, and it is the card's central finding.** The extent
capability the tree lacked was never a missing data structure — it was a
missing *source construct*. `Footprint` has carried an exact, checked,
componentwise quantity since cycle 3, `OwnershipState::live()` aggregates it
across the ledger, and `check_ownership` refuses a normal exit that leaves any
of it live. What no source program could do was **acquire on a byte axis**.
`bytes(N)` is exactly that construct, and the extent it transports is
`Footprint::single("bytes", N)` — a quantity the existing checker already
maintains, compares and refuses on.

So the answer to "how does v1 transport an extent" is *it does not add a
transport*; it spends one the language had and had never spelled.

## 4. The model

`crates/qcore-core/src/std_buffer.rs`. A program is a flat, ordered list of
events in one entry function, exactly as `source.std-io.v1` is.

```rust
pub enum BufferEvent {
    Bind    { binding: String, value: i32 },
    Acquire { name: String, extent: u32 },
    Store   { name: String, index: BufferOperand, value: BufferOperand },
    Load    { binding: String, name: String, index: BufferOperand },
    Extent  { binding: String, name: String },
    Release { name: String },
    Line(StdIoEvent),
    Return,
}
```

`BufferOperand` is `Literal(i32)` or `Scalar(String)` — **the same two
constructors `StdIoOperand` has, and for the same reason**: neither can denote a
resource, so an index or a stored byte can never be a buffer. A buffer is named
only at the four positions that take a `name: String`, and each of those
positions is a custody operation.

### 4.1 What the extent is

`Acquire { extent }` becomes `OwnershipOp::Acquire { footprint:
Footprint::single("bytes", extent) }`. Three consequences, none of them written
by this card:

* `OwnershipState::live()` is the live byte total of the program at any point;
* `check_ownership` refuses a normal exit with any of it live — that is the
  **leak** refusal, inherited verbatim;
* the ledger entry retains the extent for the whole lifetime, so
  `extent(b)` reads it out of the custody model rather than out of a
  side table.

The last point is what makes the extent *transported* rather than *remembered*:
there is one extent per resource, it lives in the ledger the custody checker
maintains, and a program that could desynchronize the two would have to
desynchronize `check_ownership` from itself.

### 4.2 What is inherited, and what is not

Following the `PIPE-CHAIN` precedent ("inherit, don't reimplement"):

| Refusal | Owner | This card's contribution |
|---|---|---|
| leak at normal exit | `ownership::check_ownership` → `NormalPathLeak` | the lowering, and the diagnostic attribution |
| double release | `ownership` → `DoubleDrop` | the same |
| use after release | `ownership` → `UseAfterDrop` | the same |
| use of an unbound name | `ownership` → `UnknownBinding` | the same |
| rebinding a live name | `ownership` → `BindingAlreadyExists` | the same |
| footprint arithmetic overflow | `ownership` → `Arithmetic(Overflow)` | the same |
| **bounds** (index outside `[0, extent)`) | **this card** | the whole rule — no existing checker knows about indices |
| **byte range** (a stored value outside `0..=255`) | **this card** | the whole rule |
| **decidability** (a non-constant index) | **this card** | the whole rule |

The three new rules are new because nothing in the tree had ever indexed
anything. Everything about *custody* is the unedited checker's, and the card's
report pins that by showing the refusals arriving as `OwnershipError` values.

## 5. The source surface

Behind `import std.buffer;`, gated exactly as the five existing bundled modules
are (`Q014` unknown module, `Q015` capability without its file-local import).

```ebnf
module  ::= import* (error-decl | entry)*
import  ::= "import" ("std.buffer" | "std.fmt" | "std.io" | "std.logger") ";"
entry   ::= "fn" NAME "(" ")" "->" "unit"
            [ "effects" "{" EFFECT { "," EFFECT } "}" ] "{" stmt* "}"
stmt    ::= "let" NAME "=" [ "-" ] INTEGER ";"
          | "let" NAME "=" "bytes" "(" INTEGER ")" ";"      (* acquire        *)
          | "let" NAME "=" "extent" "(" NAME ")" ";"        (* bind the length *)
          | "let" NAME "=" NAME "[" OPERAND "]" ";"         (* read one byte   *)
          | NAME "[" OPERAND "]" "=" OPERAND ";"            (* write one byte  *)
          | "drop" NAME ";"                                 (* release         *)
          | "print" RENDERING OPERAND ";"                   (* std.io + std.fmt *)
          | "log" LEVEL RENDERING OPERAND ";"               (* std.logger + std.fmt *)
          | "flush" ";"                                     (* std.logger      *)
          | "return" ";"
OPERAND ::= [ "-" ] INTEGER | NAME
EFFECT  ::= "io" | "alloc" | "mut"
```

### 5.1 Reserved words: exactly two, and one measured collision

**`bytes` and `extent`.** Everything else in the grammar reuses a word or a
symbol the language already has:

* `let`, `drop`, `return` are already reserved. `drop b;` is deliberately **not**
  a new `release` keyword: the operation *is* `OwnershipOp::Drop`, so a new word
  would have been a synonym for one the language already spells. The parser
  already classifies it as `StatementKind::Drop { place }`.
* `[` and `]` are already lexed (`Symbol::LeftBracket` / `RightBracket`) and are
  consumed today only in a **usage-interval** position (`[lo, inf]`) inside type
  and quantity annotations — never in a statement body. `b[0] = 65;` therefore
  parses as `StatementKind::Assignment { place: "b" }` and `let x = b[0];` as an
  ordinary `Let`, with no parser change at all.
* `print`, `log`, `flush`, `dec`, `hex`, `trace`, `debug`, `info`, `warn`,
  `error`, `fatal` are
  `STD-MODULES`'s and are reused unchanged.

Both new words are **load-bearing in the measured sense** `grow` and `rank`
established: name resolution collects the tokens after a `let`'s `=` as
referenced names, so without reservation `let b = bytes(8);` refuses at `Q013`
with `bytes` reported as an unknown name. Both directions are pinned.

**The constructor is spelled `bytes` because `buffer` collides, measured.** The
natural spelling `buffer` occurs **555 times** across the source-side corpus,
and — unlike a parameter name, which the referenced-names filter already shields
because it is followed by `:` — it occurs in **binding position** in shipped,
pinned fixtures: `let buffer = qcore_buffer_new(1)?;` and `drop buffer;` in
`examples/threads/shared_readers.qcore`, and `fn worker(buffer: own<Buffer>…)`
in `examples/threads/owned_buffer.qcore`. Reserving it would refuse those files
at `Q013`. Per the `work` → `effort` (T08-EPOCH-SRC)
precedent, **this card's syntax is renamed rather than the pinned
surface edited**, and the refusal is pinned as a test so the decision is
recorded in executable form rather than only in prose.

The **module** is still named `std.buffer`. A module name is a dotted path in an
`import` declaration, not a reserved word, so nothing about the collision
reaches it — and keeping the name is what lets this document and
`NATIVE_LIBRARY_LINKAGE_V1.md` §6 go on talking about the same capability.

Two further candidates were measured and rejected for the same reason, and are
recorded so a later card does not re-derive them: **`alloc`** is worse than
`buffer` — it is a live *effect-set member* (`effects { io, alloc, mut }`, 89
uses) as well as an extern-fn name and a `quant axis` name, so reserving it
would move a pinned effect keyword; and **`size`**, **`len`**, **`free`**,
**`store`**, **`release`**, **`make`**, **`allocate`** and **`span`** each
collide as extern-function or parameter names. `bytes`, `extent`, `byte`,
`capacity`, `at`, `index`, `get`, `put`, `new`, `slice` and `load` are free —
each occurs in `.qcore` files only inside `//` comments, if at all.

### 5.2 The decidable-observation rule

Every `OPERAND` is a **check-time constant**: an integer literal, or a name this
entry bound to one — which on this route includes a name bound by `extent(b)` or
by `b[i]`, because the checker computes both. There is no run-time-varying
index in v1, and the reason is the one every prior card respected: this route's
evidence is a **byte comparison between a reference execution and a native run**,
and a value the reference interpreter cannot compute is a value it cannot
predict a line for. A varying index would leave "the" observation ill-defined.

The refusal is by name (`Q052`), not by silence. What it excludes is stated in
§9 as `STD-BUFFER-VARYING-INDEX`, whose missing capability is a relational
domain over indices — the same wall `remaining-borrow-liveness` records for
may-facts, and strictly larger than a rendering change.

The card considered the `succeeds INTEGER` verified-outcome pattern for the
*length* and did not need it: a length written at the acquisition site is
already a literal, so there is no declared outcome to compare against reality.
The pattern's actual subject on this route is **allocation failure**, and §6.2
records how that is handled instead.

### 5.3 Effects

The derived row is the first in the tree that is not `{Io}` or empty:

| Event | Contributes |
|---|---|
| `bytes(N)` | `Alloc` |
| `b[i] = v` | `Mut` |
| `b[i]`, `extent(b)` | — (a read derives nothing; it observes) |
| `drop b` | `Alloc` (the release is the allocator's other half) |
| `print` / `log` / `flush` | `Io` |

A program that allocates and never prints derives `{Alloc, Mut}` and needs no
`std.io`. The `effects { … }` row still has to admit every derived effect
(`Q029`'s rule, re-stated on this route's own code), so the opt-in stays
visible.

## 6. C emission

Self-contained C11. No library, no extern declaration, no manifest.

### 6.1 The accessor and the trap discipline

Following the `QCORE_IO_*` precedent (one prelude, macros rather than a variadic
wrapper, so each site keeps its `-Wformat` checking), the buffer prelude defines
an allocation helper and a bounds-checked accessor. The accessor **aborts** on
an out-of-range index.

**The trap is a witness, not the check.** Every index in an admitted program is
a check-time constant that the checker already proved in range, so the emitted
comparison is provably never taken. It is emitted anyway for the reason the
repository states elsewhere about defensive checks: it costs nothing, it makes
the generated C readable as a standalone artifact, and — the point that matters
— **if it ever fired, that would be a defect in this checker**, so a sanitized
run that completes is evidence about the checker and not only about the program.

What is explicitly **not** claimed: the trap is not a substitute for the static
check, it does not make a varying index admissible, and it is not a recovery
mechanism — `abort()` is not catchable and produces no QCore error value.

### 6.2 Allocation failure

`malloc` can return `NULL`. v1 emits a check that **aborts** on failure, and the
model says nothing about the failing run. That makes every claim on this route a
**partial-correctness** statement, and it is stated rather than hidden: *if the
emitted program produces an observation at all, it is the reference
observation.* An allocation failure produces no observation, so it cannot
contradict one.

The alternative — modelling both arms, so a program could recover — was rejected
for v1 on a measured ground rather than a preference: the route is flat and
branchless, so a failure arm would have nowhere to go. Admitting it needs
`source.branch.v1`'s join, which is a different pass's construct. §9 records it
as `STD-BUFFER-ALLOC-FAILURE`.

### 6.3 Byte-identical for non-users

A module that imports nothing of this card's compiles through the unchanged
routes. The route is selected only when a buffer construct is present, so the
emitted C of every existing fixture is byte-identical by construction rather
than by comparison.

## 7. What does **not** cross a boundary

**A buffer does not cross the extern boundary in v1, and the answer is NO by
refusal, not by omission.** A buffer name in an extern call's argument list is
refused by name. The reasoning is invariant 1 applied to what a C04 contract can
say: a contract's parameter is an extentless `u8` projection
(`NoExtentTransport`), so passing an extent-carrying buffer through one would
**discard the very fact this card adds** and replace it with a premise. A model
that transports an extent and then hands it to a boundary that cannot carry it
is worse than one that refuses, because the refusal is visible.

The future path is the manifest of `NATIVE_LIBRARY_LINKAGE_V1.md` §4 plus its
stage (b): when the C04 vocabulary carries an extent, a contract can *state* the
length it touches, and the two tiers meet. Until then, `STD-BUFFER-EXTERN`.

Likewise refused, each by name: a buffer as a rendering operand (it is not a
scalar, and `borrow` renders an identity that only a *custody-model-publishing*
route supplies — this route publishes none in v1); a buffer inside a `thread
scope`, a mutex or an epoch region (route selection places this route after
every concurrent route, so those modules never reach it and keep `Q600`); a
buffer crossing a pipe stage.

## 8. Diagnostics

A new semantic pass `StandardBuffer` owning **`Q051`–`Q057`**, allocated through
the §L21 process in the same change as the implementation. The census confirmed
`Q051`–`Q099` entirely unallocated; `Q050` is `ComposedTasks`'s single code and
is the last taken.

| Code | Condition |
|---|---|
| `Q051` | the flat entry shape this profile models is violated (the pass's presentation code) |
| `Q052` | an operand that is not a check-time constant this entry bound — the decidable-observation refusal |
| `Q053` | a bounds or range violation: an index outside `[0, extent)`, a stored byte outside `0..=255`, an extent outside the admitted range |
| `Q054` | a custody refusal reported by the inherited ownership checker (leak, double release, use after release, unbound name) |
| `Q055` | a buffer at a position this route refuses to let it cross (an extern argument, a rendering operand) |
| `Q056` | the published certificate failed its independent rederivation |
| `Q057` | the entry performs a derived effect its declared `effects { … }` row does not admit. Deliberately **this pass's own code and not a reuse of `StandardIo`'s `Q029`**: §L21 forbids a second semantic pass reusing a code, and this row can carry an undeclared `Alloc` or `Mut`, which `Q029` never reports |

No existing code changes meaning. `Q020`/`Q021` stay separate (constraint 2).
A collision discovered at integration relocates this **unshipped** band, per the
wave-3 `T07`/`T09` precedent; the code-to-condition mapping is stable under such
a shift.

## 9. Named follow-ons

Each names its missing capability, because that is what makes it a boundary
rather than a backlog item.

| Name | What it is, and what it needs |
|---|---|
| `STD-BUFFER-STRING-LITERAL` | a string literal as a value. Needs a textual type in the atomic type list, a literal form the lexer admits outside `verifier scope` metadata, and an encoding decision — three separate surfaces, none of which exists |
| `STD-BUFFER-RENDER` | rendering a buffer's **contents** as text. Needs `STD-BUFFER-STRING-LITERAL`'s encoding decision first: without one there is no fact about what a byte sequence *says* |
| `STD-BUFFER-VARYING-INDEX` | an index the checker cannot constant-fold. Needs a relational domain over indices, and a comparison discipline for an observation the reference interpreter can no longer predict exactly |
| `STD-BUFFER-ALLOC-FAILURE` | a recoverable allocation failure. Needs `source.branch.v1`'s join, so the failure arm has somewhere to go |
| `STD-BUFFER-EXTERN` | passing a buffer to a declared extern callee. Needs the C04 vocabulary to carry an extent — `NATIVE_LIBRARY_LINKAGE_V1.md`'s stage (b) — or the pointer arrives extentless and the fact is discarded |
| `STD-BUFFER-ABI-EXTENT` | the C04 half of the above, stated separately because it is a **contract-loader** change and this card opens no loader |
| `STD-BUFFER-TASKS` | a buffer crossing a task boundary. Blocked on the same measured wall `STD-MODULES` §8 recorded: the task ledger has no post-creation transfer operation |
| `STD-BUFFER-GROW` | reallocation. Needs a custody shape the ownership IR does not have — an acquisition that replaces a live one — which is `Reallocated` on the extern side and has no source spelling |
| `STD-BUFFER-SLICE` | a borrowed sub-range. Needs the ownership IR's borrow nodes to carry a footprint *fraction*, which they do not |
| `STD-BUFFER-COPY` | a bulk copy between two buffers. Needs no new custody shape, but needs the loop or bulk operation this flat route has no statement for |
| `STD-BUFFER-BORROW-RENDER` | a borrowed render on this route. Needs the route to publish a custody identity and a liveness ledger a join can read |
| `STD-BUFFER-EXAMPLE` | a **published** `examples/buffer/` family with a README, held to `EXAMPLES-SUITE`'s transcript-extraction standard. This card ships test fixtures with that evidence; a published example with its README is a different deliverable |
| `STD-FMT-EXTENT` | *(split, not retired — see §10)* |

## 10. What this card does to `STD-FMT-EXTENT`

`STD-FMT-EXTENT` was recorded by `STD-FMT-BORROW` §9 as: *"rendering a
resource's size. Needs the model to retain a declared extent — which it
deliberately does not."*

This card supplies the missing capability **for its own resources and no
others**, and the distinction is the whole of the honest answer:

* On `source.buffer.v1`, a size is renderable **today, with no rendering
  change at all** — `let n = extent(b); print dec n;` — because `extent(b)`
  binds an ordinary integer scalar, and rendering an integer scalar is
  `source.std-io.v1`'s existing, unedited operand form. No `borrow` marker is
  involved and no new operand constructor exists.
* On the **extern** route, `STD-FMT-EXTENT` is **untouched**.
  `NoExtentTransport` is still a standing premise of every extern certificate,
  a declared C04 contract still cannot say how many bytes a callee touches, and
  nothing here changes that.

So `STD-FMT-EXTENT` is not retired: it is **split**, and what it now needs is
exactly `STD-BUFFER-ABI-EXTENT` — the C04 vocabulary change — and nothing on the
rendering side. That is a strictly better-posed follow-on than the one this card
inherited, and saying which half moved is more useful than claiming the name.

## 11. Deviations found by measurement

Nine, each recorded in full with its evidence in
[`docs/reports/wave22-std-buffer/README.md`](../reports/wave22-std-buffer/README.md)
§7. In brief:

1. **`buffer` collides and the constructor was renamed** to `bytes` — 555
   source-corpus occurrences, in *binding* position in shipped thread fixtures.
   §5.1 above was written after this measurement and already records it.
2. **`alloc` is a worse collision than `buffer`** and was never a candidate: it
   is a live effect-set member as well as an extern-fn and `quant axis` name.
3. **An index naming nothing refuses in two different places.** A byte *write*
   is an `Assignment`, whose tokens before the `=` name resolution drains, so
   its index never reaches `Q013` and this route's `Q052` catches it; a byte
   *read* sits after a `let`'s `=` and refuses at `Q013`. Both halves pinned.
4. **Declaring an extern function is not being an extern module** — route
   selection asks whether a file *calls* one, so a declaration-only file is
   still a buffer module. Correct, and pinned in both directions.
5. **A refused extent must still bind its name**, or every later statement about
   it cascades into a second, misleading refusal.
6. **The emitted prelude had to become selective**: emitting an unused accessor
   fails `-Werror=unused-function`, which is a real constraint of the flags the
   emitted C is built under.
7. **The "first route to derive `Alloc`" claim was false**, and is replaced by
   the true one: the extern route derives those effects too, but off a
   *declared contract*'s projections, where this route derives them from an
   acquisition and a store it discharges itself.
8. **The undeclared-effect refusal was emitting `StandardIo`'s `Q029`**, which
   §L21 forbids a second semantic pass from doing. It now has this pass's own
   `Q057`, and the band is `Q051`–`Q057`.
9. **The Lean `Imports` literals grew a field**, and
   `QCore.StdIo.no_module_substitutes_for_another` is now strictly stronger —
   the same deliberate widening `STD-MODULES` recorded. The spec's §L04 registry
   sentence was also found stale at "five names" (it predates this card:
   `std.extern.unsafe` was missing) and corrected to seven.

## 12. Lean obligations

Per the closing standard (Rust plus audited Lean; Python auxiliary):

* the registry grows by one module, gated by the **same** generic theorem
  `admitted_capability_requires_its_module` every other module is gated by, so
  none can drift;
* the extent is a function of the acquisition and is invariant under every
  non-custody event;
* every admitted access is in range, and the bounds rule is decidable;
* a program's live byte total is zero at a normal exit, and a release strictly
  decreases it;
* the operand forms are exactly two, stated as one list so a widening changes a
  theorem rather than slipping past it;
* no new axiom, no placeholder, no `native_decide`; a new `Audit*.lean`
  registered with `scripts/check_lean_audits.py`.

## 13. Acceptance gate

The standing per-change gate, plus: corpus-neutral by construction (no gateway,
audit, contract-corpus or provider file touched, `qcore-schemas` untouched); spec
§L21 and `QCORE_IMPLEMENTED_GRAMMARS_r0_3.md` updated in the same change as the
implementation; one sanitized ASan/UBSan/LSan native run comparing the emitted
program against the reference observation per stream, with a **positive** leak
control — the fixture allocates, so LSan has something to find and finding
nothing is evidence.
