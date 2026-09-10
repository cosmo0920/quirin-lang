# STD-FMT-BORROW — a borrowed render of a live resource (wave 20)

Card brief: *"The std-io v1 effect discipline is structural: a rendering
operand's type has two constructors and neither can denote a resource, so a
rendering cannot touch custody. STD-FMT-BORROW is the recorded follow-on:
admitting a borrowed render of a live resource — printing something ABOUT a
resource (its descriptor number, its size, a diagnostic identity) without
consuming or laundering it. That needs its own custody argument and the
sequential-pass evidence a widened route would owe."*

Named follow-on of `STD-MODULES`
([`../wave17-std-modules/README.md`](../wave17-std-modules/README.md) §3, §7),
carried unchanged through `STD-IO-COMPOSE`
([`../wave19-std-io-compose/README.md`](../wave19-std-io-compose/README.md) §8),
which recorded that it had become *more* attractive: "a composed module has live
identities beside its renderings, which is exactly the setting that argument
would be about."

Base commit: main `e01cac2`.

**What landed.** One operand form, `borrow NAME`, on the composed route
`source.std-io-extern.v1`. It renders **the resource's model identity** — the
number the emitted C's own call trace spells `rN` — and it is admitted only
where the extern model's *own published ledger* holds that identity live at that
statement. **No reserved word was added**: `borrow` was already one. **No
diagnostic code was added, moved or changed**: `Q028` and `Q043` each gained
conditions inside their own pass's band. The composition still owns no custody
rule — the liveness it reads is the extern checker's own, published rather than
recomputed.

**What did not, and why it is the honest answer.** Rendering a resource's
*state* — its contents, its extent, its descriptor number, its address — is
**refused**, on four separate measured grounds (§2). Three of the four candidate
projections the brief named do not exist in any model a rendering can reach, and
the one live-resource read that does exist (`qcore_buffer_read`) lives on a route
where every rendering is already refused. So the admission below is deliberately
the narrowest sound one, and everything larger is a named follow-on with its
missing capability stated.

---

## 1. The census

The deliverable the brief put first. Read-only, over `crates/qcore-core/src`,
`crates/qcore-frontend/src`, `compiler/src` and the fixture and contract corpora,
answering one question: **what scalar data derived from a live source-language
resource is observable, by an existing admitted read form, without a custody
effect?**

| # | Candidate | Yields a scalar from a live resource? | Reachable from a rendering? | Evidence |
|---|---|---|---|---|
| C1 | `qcore_buffer_read(B)` — the model **computes** the number out of the resource's own state, binds an `i32`, touches no ledger and derives no effect. Admitted on `own<Buffer>` **and** `borrow<Buffer>` | **yes** | **no** — it is the task/epoch-task routes' intrinsic, and route selection sends every module that prints and spawns to the concurrent model, which refuses (`Q600`) | `crates/qcore-frontend/src/source_tasks.rs:1109-1145`; `crates/qcore-core/src/task_execution.rs:112-115`, `:746-752`; `crates/qcore-core/src/task_execution/reference.rs:208-225`; `examples/threads/shared_readers.qcore:6` |
| C2 | a declared extern callee taking the resource at a `resource_borrowed` parameter and returning a custody-free scalar | **yes**, but the number is **declared**, not derived | **already reachable, and already renderable** — this is `STD-IO-COMPOSE`'s payoff, `let total = qcore_extern_sum(b, 8) succeeds 56; print dec total;` | `crates/qcore-core/src/extern_call.rs:404-411`, `:1019-1020`, `:1194-1197`; `compiler/tests/fixtures/compose/chain.qcore:34-36` |
| C3 | a resource's **extent** (a size, a length, a capacity) | **no such projection exists** | — | `crates/qcore-frontend/src/source_epoch_tasks.rs:49-53` states it: "this backend's `Buffer` has **no extent**"; and `ExternAssumption::NoExtentTransport` is a standing premise of every extern certificate — the model deliberately transports none |
| C4 | a **descriptor number** | modeled, but in `os_endpoint` only | **no** — no source route consumes `os_endpoint`, by its own record | `crates/qcore-core/src/os_endpoint.rs:76-81` (`Epoch { kind, number, generation }`), `:11-15` ("**No production audit route consumes it**"); no `os_endpoint` reference exists anywhere under `crates/qcore-frontend/src/` |
| C5 | a **refcount**, a container count, an index | **no projection anywhere** | — | `crates/qcore-core/src/library_custody.rs:35-99` — ten custody operations, none of which binds or returns a value; a refcount appears only as the failure mode `RefcountOverflow` |
| C6 | a chunk length, a CRC, a file length, a mapping length | from **recorded bytes and traces**, never from a live language resource | **no** | `crates/qcore-core/src/chunk_integrity.rs:1-40`, `:469`; `crates/qcore-core/src/capacity_boundary.rs:1-52` |
| C7 | `IndexTerm::size_of(TypeName)` | a static type-level symbol; takes no resource, no binding, no identity | not spellable in any source grammar | `crates/qcore-core/src/index.rs:77-83` |
| C8 | the resource's **address** | it exists at run time | **no**, and this one is refused on a *soundness* ground rather than an absence: an address is not model-determined, so the reference interpreter cannot predict it and the per-stream byte comparison this route rests on would be unwritable — the same wall `PIPE-SCALAR-RESULT` §4.3 hit for a bound call's result | `../wave19-pipe-scalar-result/README.md` §4.3 |
| C9 | the resource's **model identity** | **yes** — the number the ledger, the certificate and the emitted C's trace all already name it by | **yes**, on the composed route, where a ledger exists beside the renderings | `crates/qcore-core/src/extern_c_emit.rs:76-78` (`resource_name(r) = "r{r}"`), `:189` (`acquire {callee} {rN}`) |

Two conclusions, and they are the card's result.

1. **The three projections the brief named do not exist to be rendered.** A
   size is refused by `NoExtentTransport` and by the buffer model's own stated
   absence of an extent; a descriptor number is modeled only where no source
   route reads it; an address cannot be compared against a reference run. Each
   is a *named missing capability*, not an unwritten feature (§9).
2. **The one thing that does exist is the identity**, and admitting it is what
   makes the custody argument the card asks for real rather than hypothetical:
   liveness becomes a checked precondition of writing a line.

One further census result worth stating plainly, because it changes what the
follow-on is: the **descriptor-number case is already discharged and needs no
borrow at all.** `CallParameter::Data`'s own doc says a scalar "may carry a
descriptor number, never descriptor custody (§C04.1)"
(`crates/qcore-core/src/extern_call.rs:274-278`). A program that *has* a
descriptor number has it as a scalar, in the value namespace, and
`STD-IO-COMPOSE` already renders those. There was never a borrow to add there.

**Constraint-14 keyword census.** `borrow` occurs in 47 `.qcore` files, in every
case as the type constructor `borrow<'lifetime, mut, T>` of an extern
declaration or contract, or inside a `//` comment. It is **already a reserved
source word** (`crates/qcore-frontend/src/frontend.rs`, the checked mutex-guard
profile's block), so it is never a binding and never a referenced name. The
rendering-operand position is new with this card, so no existing file can
collide with it. **Words added: 0.**

## 2. What a borrowed render is in v1

```ebnf
OPERAND = [ "-" ], INTEGER | NAME | "borrow", NAME ;
```

`print RENDERING borrow NAME;` and `log LEVEL RENDERING borrow NAME;` render the
**model identity** of the resource `NAME` denotes, through the two existing
`std.fmt` renderings:

```qcore
let b = qcore_extern_alloc(8) succeeds true else raise Failure;
print dec borrow b;          // stdout: 1
qcore_extern_fill(b, 7);
let total = qcore_extern_sum(b, 8) succeeds 56;
log info dec total;          // stderr: [INFO] 56   (the value namespace, unchanged)
log warn hex borrow b;       // stderr: [WARN] 0x00000001
qcore_extern_release(move b);
```

The shipped fixture is
`compiler/tests/fixtures/compose/borrowed_render.qcore`, and those three lines
are what its sanitized native run writes.

**Three boundaries, stated rather than left implicit.**

* **The emitted program reads nothing out of the object.** The identity is a
  number the checker determined, so a borrowed render emits `INT32_C(1)` — a
  literal, exactly as this route's `let`-bound integers already are. What the
  borrow buys is the *checked precondition* that the resource was live at that
  statement, not a run-time read. A run-time liveness probe was considered and
  **rejected as a false reassurance**: the only thing the emitted C could test is
  `rN == NULL`, which a released pointer does not satisfy, so the check would
  pass on exactly the case it appeared to rule out — and reading an indeterminate
  pointer value to do it is its own problem.
* **The identity is a certificate-level number, not an OS-level one.** It
  identifies *which* modelled resource a line is about, and it is the same
  number the extern route's synthesized trace prints on the same stream, so the
  two correlate. It is not a handle, an address, or anything a reader could act
  on outside the model.
* **A rendering of a width the route cannot render refuses**, identically to the
  value case: an identity outside `int32_t` is refused rather than truncated.

## 3. The custody argument

The three clauses the brief specified, each backed by a theorem and a test.

### (a) The resource must be live at the render

A borrowed render is admitted only where the identity is in the extern model's
ledger at that composed step. **The composition derives no liveness of its own.**
`ExternCallCertificate` gains one field, `live_after: Vec<Vec<ResourceId>>` — the
identities live on the normal path after each event, in trace order, taken by
`check_extern_program` from its own `live` list — and the join reads it. That
keeps `STD-IO-COMPOSE`'s central property intact: nothing in
`composed_std_io.rs` re-implements a custody rule, because the answer it uses is
the extern checker's.

**The must-fact bar of `BORROW-LIVENESS` is met by construction, not by a
join.** That card's rule is that "live here" must be a *must*-fact; the standing
record (`../remaining-c02-p-alias/README.md`, `../remaining-borrow-liveness/README.md`)
is that a may-fact is not enough to license a read. On this route the extern
trace is straight-line — no branch construct reaches it — so the ledger at a
step is exact rather than a meet of paths, and the ledger the rendering consults
is the one the emitted program is actually in.

Everything that is *not* live refuses, and the cases are not enumerated
separately because they are one condition: an identity never established
(including one whose acquisition took its declared **null** arm), and one that
departed by consumption, release, storage under a declared owner, or
reallocation, are each simply "not a member of the ledger here".

### (b) Every obligation is unchanged

A rendering appends a line. It opens no obligation, discharges none, moves none,
and the identity is as live after it as before it. Three independent statements
of that, none of them a comment:

* `a_borrowed_render_leaves_the_ledger_unchanged` and
  `a_program_of_renderings_leaves_the_ledger_unchanged` in Lean — the identities
  reachable after any number of renderings are exactly those reachable before;
* the existing `the_composed_ledger_is_the_extern_projections_ledger` still
  holds, unedited, over the widened operand type;
* measured in Rust by `a_borrowed_render_leaves_the_custody_obligations_unchanged`,
  which compiles the shipped fixture **with** and **without** its two renderings
  and compares the extern certificate field for field — acquisitions,
  consumptions, releases, stored, reallocations, error dispositions, `live_after`
  and the premise list, plus both value-namespace fields.

**Why this does not falsify `Borrowed`, given CTWIN.** Standing constraint 10
refuses a *publication* — a store of a tracked identity into an object that
outlives the call — precisely because `Borrowed` asserts the caller's
obligations are unchanged and a publication falsifies exactly that: it creates a
second reachable path to the resource, and someone else can now release or read
it. A rendering creates no such path, on three grounds that hold together:

1. **The destination is not an object of the custody graph.** A byte stream is
   not a location any model tracks, indexes, owns or reads.
2. **What crosses is a copy of the identity number, not the address.** The
   address is the thing that could be dereferenced, and §2 records that it is not
   emitted and could not be — it is not model-determined.
3. **No operation of any model in this repository reads a stream back.** The
   census confirms it: the four trace backends and this route write; the only
   read-back is the compiler's own translation-validation harness, outside the
   language. There is therefore no path from a written line to a custody
   identity, so nothing the caller owes changed.

Where CTWIN's launder crossed the **summary** boundary — a body publishing a
borrowed formal made W4 publish `Borrowed` for the helper, and the helper's
caller then certified — a rendering has no such boundary to cross: it produces
no summary, no effect beyond `Io`, and no value.

### (c) The route boundary

| Route | Borrowed render | Why |
|---|---|---|
| `source.std-io.v1` | **refused**, `Q028` | no custody model, so the identity environment is empty. Structural: the refusal is the absence of a model, not a rule the route states |
| `source.std-io-extern.v1` | **admitted**, under the liveness rule above | the extern model supplies both the identity and the ledger |
| every concurrent route (`thread scope`, mutex, epoch, composed epoch) | **refused**, `Q600` | route selection places the composition *after* every concurrent route, so a module that prints and spawns never reaches it. Unchanged from `STD-MODULES`, and now pinned for the borrowed form too |
| the pipe (`\|>`) | **refused**, `Q036` / `Q028` | see §4 |

A borrow across a task boundary means something this card does not model — the
task ledger's parent/child custody edges, share arithmetic and join discipline —
so refusing it is not an omission but the same route-selected-last discipline
`STD-MODULES` established, applied to a new form. `COMPOSE-CONCURRENCY` is where
that question lives, and it is strictly larger.

## 4. The ValueId wall, and the pipe

**The wall stands.** A borrowed render creates no binding: there is no `let`
form for it, no `ValueId` is introduced, the identity never enters
`operand_values` or `value_abi`, and it can therefore never reach a declared
scalar parameter or any other value position. `ResourceId` never becomes a
`ValueId` on any path. The rendered identity is **data at the render site only**
— that is the half of the brief's alternative this card takes, and it is pinned
in three places:

* two separate maps in the model (`StdIoProgram::scalars` beside
  `StdIoProgram::identities`, `ComposedProgram::operand_values` beside
  `operand_identities`), read by two lookups;
* four Lean theorems stating the independence in both directions —
  `a_borrowed_render_reads_no_scalar`, `a_scalar_render_reads_no_identity` (in
  both modules), `a_borrowed_render_reads_no_value`,
  `a_borrowed_render_binds_no_value`;
* two Rust tests, one per direction: `a_borrowed_render_of_a_scalar_refuses` and
  `a_bare_render_of_a_resource_still_refuses_at_the_standard_io_route`.

**`Q028`'s resource-free rule for a bare operand is not weakened, and the pipe
composes deliberately.** The brief asked for this to be pinned, and it is, in
both spellings a pipeline could try:

* `b |> print dec` desugars to `print dec b;` — **no marker** — which is the
  bare-operand case and still refuses at `Q028`, byte for byte the refusal
  `STD-IO-COMPOSE` pinned;
* `b |> print dec borrow` is a sink stage naming one selector word too many and
  refuses at the pipe's own `Q036`.

So no pipeline can spell a borrowed render in v1. That is a *deliberate*
composition rather than an accident — the marker is what makes it impossible —
and it is a named follow-on (`PIPE-BORROW-SINK`), pinned in both halves by
`a_pipeline_cannot_spell_a_borrowed_render`.

## 5. Diagnostics

**No code was added, moved or changed.** Two existing codes gained conditions,
each inside its own pass's band per §L21, and both were recorded in the spec and
in the implemented grammars in the same change.

| Code | Pass | New condition |
|---|---|---|
| `Q028` | `StandardIo` | a `borrow NAME` operand for which no checked custody model published an identity — on `source.std-io.v1` that is every borrowed render, because the route has no custody model |
| `Q043` | `ComposedStdIo` | a borrowed rendering naming something this module binds no resource for; one whose identity the extern model's published ledger does not hold live at that composed step; one whose identity is outside the width this route renders; and a cross-check that the identity the standard-IO projection rendered is the one the extern model bound |

Taking conditions rather than a code was the deliberate choice: this card states
no rule that a *new* semantic pass would own, other wave-20 cards are allocating
in the `Q044`+ range, and a new code here would have created a reconciliation
conflict for nothing.

`Q020` and `Q021` are untouched and stay separate (standing constraint 2). The
`Q023` default is unmoved. No fixture surface moves.

## 6. Admitted and refused, in full

| Shape | Result |
|---|---|
| `print dec borrow b;` / `log warn hex borrow b;` with `b` live | **admitted** |
| the same after `qcore_extern_release(move b);` | `Q043` — not live |
| the same before the acquisition | `Q013` — measured; name resolution runs first (§7.1) |
| the same where the acquisition declared `succeeds false` | `Q043` — the identity is never established on the modelled run |
| `print dec borrow n;` with `n` a bound scalar | `Q043` — the custody namespace resolves no value |
| `print dec b;` with `b` a resource (no marker) | `Q028`, **unchanged** |
| `print dec borrow total;` on `source.std-io.v1` | `Q028` — no custody model |
| `print dec borrow r;` with `r = qcore_acquire(true)` on `source.std-io.v1` | `Q030`, `Q028` — both halves, as for a bare operand |
| `print dec borrow nope;` | `Q013` |
| `print dec borrow;` | `Q030` — shape |
| a borrowed render inside a `thread scope` | `Q600` — the concurrency exclusion |
| `b \|> print dec borrow;` | `Q036` |
| `b \|> print dec;` | `Q028` |
| a certificate whose borrowed-render list was edited, cleared, or whose standard-IO half's was | recheck failure, three directions |
| a module writing no borrowed render | byte-identical, and its certificate names none |

## 7. Deviations found by measurement

1. **A borrowed render placed before its acquisition refuses at `Q013`, not at
   the join.** Name resolution runs over the parsed body before any lowering, so
   a use before its binding is caught there. This is the third time the same
   measurement has been recorded (`PIPE-SCALAR-RESULT` §2.3, `STD-IO-COMPOSE`
   §6.2), and it is handled the same way: the join's own liveness rule would
   refuse it too — an identity not yet acquired is not in the ledger — and is
   kept as the fail-closed backstop, with the measured `Q013` pinned so a change
   in reachability says so rather than passing silently.
2. **The extern lowering's `resource_bindings` was the wrong dictionary, and was
   corrected rather than worked around.** It published `lowering.bindings`, which
   a departing argument *removes* from, so after `qcore_extern_release(move b)`
   the name `b` was simply absent. A composing route needs both questions
   answered separately, so the lowering now carries `ever_bound` beside
   `bindings`: `bindings` is what the body holds **now**, which argument
   resolution must read, and `ever_bound` is the name-to-identity dictionary a
   borrowed render resolves against. This widens no admission, because **liveness
   is never decided from a name's presence in either map** — it is read from the
   extern certificate's ledger.
3. **A runtime liveness check was designed and then rejected**, on the
   `succeeds INTEGER` precedent that a declared outcome should be compared
   against reality by the emitted program. It does not carry over: the only
   property the emitted C could test is `rN == NULL`, which a *released* pointer
   does not satisfy, so the check would have passed on exactly the case it
   appeared to rule out — a false reassurance, and reached by reading an
   indeterminate pointer value. Recorded rather than shipped (§2).
4. **`check_extern_program`'s per-event body is now a labelled block.** Adding
   the ledger snapshot needed every exit from that body — the early ones
   included — to reach the snapshot, and a missed one would silently shift a
   later index's liveness. The four `continue`s became `break 'event`, semantics
   byte-unchanged, with a `debug_assert` that every event contributed exactly one
   snapshot.
5. **The census contradicted the brief's own candidate list in a useful
   direction.** The brief named "its descriptor number, its size, a diagnostic
   identity"; the first two do not exist to be rendered, and the descriptor-number
   case turns out to be *already discharged* through the value namespace (§1).
   The identity is the one that survived, and it is why the admission is as
   narrow as it is.

## 8. Evidence

### 8.1 Tests

| Target | Count |
|---|---|
| `compiler/tests/std_io_compose.rs` | **36 → 49** (+13), including one sanitized native run |
| `compiler/tests/std_modules.rs` | **31 → 34** (+3) |
| `compiler/tests/extern_calls.rs` | 80, unchanged and unmoved |
| `compiler/tests/pipe_operator.rs` | 41, unchanged and unmoved |

Both directions are pinned throughout — the full table is §6. One shipped
fixture, `compiler/tests/fixtures/compose/borrowed_render.qcore`; every refusal
case is written inline against the same declarations, so each measures the rule
it names. **No pinned expectation was edited**: all 36 existing composed tests,
all 31 existing standard-IO tests and both neighbouring suites pass unchanged.

### 8.2 Native evidence

`the_borrowed_render_program_matches_the_reference_per_stream` compiles the
emitted C against the real `compiler/tests/fixtures/extern_calls/buffer_api.c`
with `-std=c11 -Wall -Wextra -Werror -pedantic -g -O0
-fsanitize=address,undefined -fno-omit-frame-pointer`, `detect_leaks=1`, and
compares **stdout against the reference's stdout projection and stderr against
its stderr projection**. No cross-stream order is asserted.

It is a **positive** leak control for this card, which is the point: the fixture
allocates, renders the identity **twice while the obligation is open**, and
releases. Had a borrowed render moved, duplicated or discharged custody, LSan or
the double-free detector would have said so rather than this test passing. Apple
ASan provides no LSan, so leak detection stays a Linux-only observation.

### 8.3 The sequential-pass evidence the widened route owes

The brief asked for it by name. A borrowed render is a standard-IO statement, so
the extern lowering skips it and it contributes **no** operation to the
`FunctionSemantics` bundle the established sequential passes validate — no
usage-grade op, no ownership op, no resource-flow op, no error op. Those passes
therefore see the composed module's extern half exactly as they see the same
module without its renderings, which is §3(b)'s measurement stated from the other
side, and is what makes "the established sequential passes still re-check the
extern half" (`STD-IO-COMPOSE` §5.1) true of a module that renders identities
too.

### 8.4 Lean

**No new audit file**, so `check_lean_audits.py`'s count is unchanged and the
acceptance matrix's derived `lean_audits` value does not move for this card.

`proof/QCore/StdIo.lean` — audit **45 → 53** theorems (+8):
`a_route_without_a_custody_model_renders_no_borrow` (the structural refusal on
the uncomposed route), `a_borrowed_render_reads_no_scalar`,
`a_scalar_render_reads_no_identity`,
`a_literal_render_reads_neither_environment` (the wall, in every direction),
`the_operand_forms_are_exactly_these` (the surface as one list, so a future
widening changes a theorem rather than slipping past it),
`a_borrowed_render_still_carries_io`,
`a_borrowed_render_allocates_and_mutates_nothing`,
`a_borrowed_render_writes_one_line_on_its_own_stream`.

`proof/QCore/ComposedStdIo.lean` — audit **31 → 40** theorems (+9, with one
renamed): `a_borrowed_render_reads_only_a_live_identity`,
`a_borrowed_render_of_a_dead_identity_has_no_admitted_step`,
`an_unresolved_borrow_has_no_admitted_step`,
`a_borrowed_render_publishes_exactly_one_line` (it publishes the identity and
nothing else — not an address, not an extent),
`a_borrowed_render_leaves_the_ledger_unchanged`,
`a_program_of_renderings_leaves_the_ledger_unchanged`,
`a_borrowed_render_reads_no_value`, `a_scalar_render_reads_no_identity`,
`a_borrowed_render_binds_no_value`. `an_operand_is_a_literal_or_a_value_name`
became `an_operand_is_a_literal_a_value_name_or_a_borrow`, deliberately: its own
doc comment promised a widening would change it rather than slip past it, and
this is that happening.

One `private` helper lemma, `boundAtFrom_ignores_the_custody_namespace`,
following the convention this module already established
(`STD-IO-COMPOSE` §7.5) — covered by `AuditAll.lean`'s whole-project axiom
closure and transitively by the public theorem that depends on it.

**No new axiom, no placeholder, no `native_decide`.** The existing
`the_composed_ledger_is_the_extern_projections_ledger` and every other custody
theorem hold unedited over the widened operand type, which is the strongest form
of "composing added no custody path" this card could inherit.

**Not proved**, carried over: that the Rust lowering agrees with the model (a
tested Rust boundary), and that a callee's C implementation satisfies its
declared contract (`ContractTrust::Declared`).

### 8.5 Corpus neutrality

**Corpus-neutral by construction.** No file under `crates/qcore-c-gateway/`,
`crates/qcore-audit/`, `contract/qcore/` or `providers/` is touched. The whole
diff is:

```
crates/qcore-core/src/extern_call.rs            (the published ledger snapshots)
crates/qcore-core/src/std_io.rs                 (the operand form, the identity map, the certificate field)
crates/qcore-core/src/std_io_c_emit.rs          (the emitted expression)
crates/qcore-core/src/composed_std_io.rs        (the liveness join)
crates/qcore-core/src/composed_std_io_c_emit.rs (one argument)
crates/qcore-frontend/src/source_std_io.rs      (the operand grammar, the borrowed-operand census)
crates/qcore-frontend/src/source_extern.rs      (`ever_bound` beside `bindings`)
crates/qcore-frontend/src/source_composed.rs    (the custody-namespace resolution)
compiler/tests/std_io_compose.rs, std_modules.rs
compiler/tests/fixtures/compose/borrowed_render.qcore
docs/QCORE_SPEC_r0_3.md, docs/QCORE_IMPLEMENTED_GRAMMARS_r0_3.md
proof/QCore/{StdIo,ComposedStdIo}.lean, proof/Audit{StdIo,ComposedStdIo}.lean
docs/reports/wave20-std-fmt-borrow/README.md
```

The contract-corpus pin (69 modules), the 185-unit artifacts, the leak count of
**272** and the CT manifest are main's, and none of them moves for anything here.

## 9. Named follow-ons

Each names its missing capability, because that is what makes it a boundary
rather than a backlog item.

| Name | What it is, and what it needs |
|---|---|
| `STD-FMT-EXTENT` | rendering a resource's **size**. Needs the model to *retain* a declared extent — which it deliberately does not: `ExternAssumption::NoExtentTransport` is a standing premise of every extern certificate, and the task backend's `Buffer` has no extent by its own record. Adding one is a change to what the contract model transports, not to a rendering. |
| `STD-FMT-DESCRIPTOR` | rendering a **descriptor number**. Needs `os_endpoint` to be reachable from a source route; its own module header records that no production route consumes it and that wiring it needs relational branch analysis (follow-on U3). |
| `STD-FMT-ADDRESS` | rendering an **address**. Blocked on determinism, not on plumbing: the reference interpreter cannot compute one, so the per-stream byte comparison this route rests on would be unwritable. It would need the F02 comparison-up-to-an-admitted-relation treatment, or it is not v1-honest. |
| `STD-FMT-BUFFER-READ` | rendering the number `qcore_buffer_read` computes out of a live buffer — the census's one model-**computed** live-resource read (C1). It is unreachable from a rendering only because it lives on the task/epoch routes, so this is a *route* problem: it is strictly inside `COMPOSE-CONCURRENCY`, and lands with it or not at all. |
| `PIPE-BORROW-SINK` | letting a pipeline spell a borrowed render. Today both spellings refuse deliberately (§4); admitting one needs the sink stage's selector-word rule to carry an operand *mode*, which is a pipe-grammar change and not a rendering change. |
| `COMPOSE-CONCURRENCY` | *(unchanged from `STD-IO-COMPOSE`)* a module that prints and spawns. Now carries `STD-FMT-BUFFER-READ` with it. |
| `EXTERN-SCALAR-OUTCOME` *(trace half)*, `COMPOSE-RENDER-WIDTH`, `EXTERN-SCALAR-WIDTH`, `STD-FMT-DIAGNOSTIC`, `STD-LOGGER-ASYNC`, `SEQ-SCALAR-RESULT`, `PIPE-TAP`, `PIPE-ASSIGN`, `PIPE-BINDER-SPAN`, `PIPE-STMT-STAGES`, `PIPE-STAGE-ARITY` | *(unchanged)* |

`STD-FMT-BORROW` itself is **retired** by this card for the identity, and
explicitly **not** retired for state — §1 and the four rows above are the honest
statement of which half landed.

## 10. Gate

| Gate step | Result |
|---|---|
| `git submodule update --init --recursive` | run in this fresh worktree before the closing measurement |
| `cargo test --workspace --no-fail-fast` | **3,150 passed, 0 failed, 8 ignored**, doctests included — run in two halves, see §10.2 |
| `cargo test -p qcore-compiler --test std_io_compose` | **49 pass**, including the sanitized native run |
| `cargo test -p qcore-compiler --test std_modules` | **34 pass** |
| `cargo test -p qcore-compiler --test extern_calls` | 80 pass, unchanged |
| `cargo test -p qcore-compiler --test pipe_operator` | 41 pass, unchanged |
| `cargo fmt --all -- --check` | clean |
| `cargo clippy --all-targets -- -D warnings` | clean |
| `cargo run -p qcore-compiler --example reevaluation_probe` | exit **0** |
| `cargo run -p qcore-compiler --example reevaluation_followup` | exit **0** |
| `python3 scripts/check_lean_audits.py` | **85/85**, no new axiom, **no new audit file** |
| `python3 -m unittest discover -s scripts/tests` | **193 tests, 0 failures** — and **no expected failure**, see §10.1 |

Measurement outputs are worktree-private under `measure/` (gitignored), never
under `target/` and never in the shared scratchpad.

### 10.1 For the integrator

This card adds **no** `Audit*.lean` file, so the derived-value tripwire
`test_the_derived_facts_are_recomputed_rather_than_carried` does **not** trip on
its `lean_audits` count for this card — unlike the two preceding standard-IO
cards, which each left it failing for the wave integrator. `scripts/tests`
reports 193 tests and **no** expected failure here, and nothing in
`docs/reviews/r0_3/acceptance_matrix.json` needs to move for the Lean count on
this card's account.

**None of the four `P02-FLAKE` rows (standing constraint 16) tripped** on these
runs; no assertion was weakened and nothing was re-run in isolation to obtain a
pass.

Reconciliation list:

* **no diagnostic code allocated** — `Q044`–`Q046` and anything above are free
  for the concurrent wave-20 cards as far as this card is concerned;
* `docs/QCORE_SPEC_r0_3.md` §L21 rows for `Q028` and `Q043` gained conditions;
  `docs/QCORE_IMPLEMENTED_GRAMMARS_r0_3.md` §G02g and §G02i were updated in the
  same change;
* `ExternCallCertificate` gained one derived field, `live_after`. It is covered
  by the existing whole-struct `recheck_extern_certificate` comparison and by
  the composed recheck; no certificate is constructed literally anywhere but in
  `check_extern_program`;
* `LoweredExternModule::resource_bindings` changed type from
  `BTreeSet<String>` to `BTreeMap<String, ResourceId>` and now reports every
  name the body *ever* bound (§7.2);
* `std_io_c_emit::event_statement` and
  `source_std_io::lower_source_std_io_skipping` each gained one parameter; the
  single-model entry points pass the empty map, so the uncomposed route travels
  the identical code path and stays byte-identical.

### 10.2 The workspace run was taken in two halves, and why

`cargo test --workspace --no-fail-fast` was killed twice mid-flight by the
environment rather than by a test — exit **144**, once after 45 suites and once
after 7, with **no failing assertion in either partial log**. The machine was
running five sibling worktrees with about 1 GB of free memory and its swap fully
committed, so the cause is recorded as environment pressure and **no result
below is inferred from a truncated run**. The suite was then taken in two
complete runs, each of which exited 0:

| Half | Result |
|---|---|
| `cargo test --workspace --exclude qcore-compiler --no-fail-fast` | **2,438 passed, 0 failed, 8 ignored** across 134 targets (8 doctest targets included) |
| `cargo test -p qcore-compiler --no-fail-fast` | **712 passed, 0 failed, 0 ignored** across 49 targets (1 doctest target included) |

The composed figure is therefore **3,150 passed, 0 failed, 8 ignored**, each
half from one complete run. Wall-clock timings are not comparable with a quiet
machine, and nothing about the results depends on them.

## 11. Draft increment paragraph

> A borrowed render of a live resource (STD-FMT-BORROW): the census is half the
> result. Of the projections the card named, a resource's **extent** does not
> exist to be rendered (`NoExtentTransport` is a standing premise and the task
> backend's `Buffer` has no extent by its own record), its **descriptor number**
> is modeled only in `os_endpoint`, which no source route consumes — and is
> anyway *already* renderable wherever a program holds one, because a descriptor
> number is data in the value namespace — and its **address** is refused on
> determinism, since a reference interpreter cannot predict one and the
> per-stream byte comparison this route rests on would be unwritable. The one
> model-**computed** live-resource read that exists, `qcore_buffer_read`, is a
> task-route intrinsic and every rendering on that route is already refused, so
> it is inside `COMPOSE-CONCURRENCY` rather than inside this card. What is left,
> and what landed, is the resource's **model identity** — the number the ledger,
> the certificate and the emitted C's own call trace already name it by —
> rendered by `print RENDERING borrow NAME;` on the composed route
> `source.std-io-extern.v1`, admitted only where the extern model's **own
> published ledger** holds that identity live at that statement. The composition
> still derives no liveness: `ExternCallCertificate` gains one derived field,
> `live_after`, and the join reads it, so composing owns no custody rule. The
> custody argument is proved rather than asserted — a rendering leaves the ledger
> unchanged, a whole program of renderings leaves it unchanged, and the composed
> ledger is still exactly the extern projection's — and a rendering is **not** a
> publication in the CTWIN sense because its destination is a byte stream rather
> than an object of the custody graph, the bytes carry a copy of the identity
> number rather than the address, and no model operation reads a stream back. The
> ValueId wall stands: no `ValueId` is introduced, so the identity can never
> reach a declared scalar parameter, and four theorems state the two namespaces'
> mutual independence in both directions. **No reserved word was added** —
> `borrow` was already one — and **no diagnostic code was added, moved or
> changed**: `Q028` gained the uncomposed route's structural refusal (that route
> has no custody model, so its identity environment is empty) and `Q043` the
> join's liveness rule. `Q028`'s resource-free rule for a **bare** operand is
> unweakened and the pipe composes deliberately in the refusing direction: `b |>
> print dec` desugars to the bare operand and keeps `Q028`, `b |> print dec
> borrow` is one selector word too many and refuses at `Q036`, so no pipeline can
> spell a borrowed render. A run-time liveness probe was designed and rejected as
> a false reassurance — the only testable property is `rN == NULL`, which a
> released pointer does not satisfy — so the emitted program reads nothing out of
> the object and what the borrow buys is the checked precondition. 16 new tests,
> one sanitized ASan/UBSan/LSan run as a **positive** leak control, 17 new
> audited Lean theorems in two existing modules with **no new audit file**, 85/85
> audits and no new axiom. Corpus-neutral by construction. Evidence:
> `docs/reports/wave20-std-fmt-borrow/README.md`.
