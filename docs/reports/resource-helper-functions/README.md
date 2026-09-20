# Bounded resource-aware helper functions

## Scope and outcome

This card implements `source.resource-calls.v1`, a deliberately closed path for
direct acyclic source calls over the extent-carrying `std.buffer` value. It
closes Quirin review P1-A/D1 for this bounded profile: an entry can allocate a
byte buffer, transfer it to a helper, read and mutate it through helper loans,
receive owned storage back, observe the returned bytes, and release it. The
review is evidence, not specification authority; the grammar and custody rules
are stated in `docs/QCORE_IMPLEMENTED_GRAMMARS_r0_3.md` and implemented by the
checked IR.

The accepted fixture uses these types and calls:

```qcore
fn inspect(value: borrow<bytes>) -> i32
fn fill(value: lend<bytes>, byte: i32) -> unit effects { mut }
fn process(value: own<bytes> use one) -> own<bytes>

let seen = inspect(borrow value);
fill(lend(value), seen);
let output = process(move input);
```

`bytes` is intentional. Task execution's `Buffer` denotes a different fixed
four-byte resource and is refused here. `move` and `borrow` were already source
words; `lend(NAME)` is contextual at an argument position and follows the
existing standard-buffer `lend(BUFFER, LOW, HIGH)` shape. No lexer or reserved
word table changed. At base `119115de`, a tracked `.qcore`/`.qc` census found no
`own<bytes>`, `borrow<bytes>` or `lend<bytes>` spelling. Existing `lend(` forms
belonged to the standard-buffer slice route and remain selected there.

## Checked architecture

The frontend constructs a typed resource-call graph. The core checks every
function signature and body, rejects uncalled helpers, proves the graph acyclic,
computes longest call depth independently of declaration order, derives
transitive `alloc`/`mut` effects and propagates each parameter's minimum extent.
Every call site checks arity, type, exact argument evaluation order, extent and
alias compatibility. Repeating one underlying owner is allowed only for read
loans; any owned or mutable occurrence makes another occurrence incompatible.

The graph is then expanded under a 32,768-event budget into the existing
`OwnershipOp` and `BufferProgram` operations. The existing ownership checker
decides moves, lexical shared/mutable loans, access while borrowed, stale use,
drop and normal-exit leaks. The existing standard-buffer checker decides byte
ranges and independently rechecks its certificate. Scalar helper returns are
carried through flattening as actual scalar bindings, including when used as a
later stored byte. Owned results are transferred back and the caller's moved
binding remains stale.

Allocation failure has no source error payload in v1 and emitted C exits 86.
Consequently, a local allocation refuses when another owner is live, and a call
whose transitive summary allocates refuses when ownership retained by the
caller could be stranded. This includes allocating helpers called with a read
or write loan. An owner moved into an allocating helper is allowed only when
that helper's checked body establishes its own custody before allocation. This
is not general typed-error cleanup support.

The C emitter retains real helper functions. Owned values cross by value, read
loans use pointers to const byte descriptors, write loans use mutable pointers,
and the caller's moved descriptor is poisoned after the helper returns. The
generated bounds and allocation checks remain runtime witnesses
of decisions already made by the checker.

## Acceptance boundary

The profile accepts at most 256 reachable functions, 256 parameters per
function, call depth 64, 16,384 source IR statements and 32,768 expanded items.
An expanded item is a visited statement, emitted buffer event, or generated
ownership operation, including each literal-argument bind and loan wrapper. It
supports `i32`, grade-one `own<bytes>`, `borrow<bytes>` and `lend<bytes>`
parameters; unit, i32
and owned-byte results; direct calls; and the flat standard-buffer operations.
The existing grade-one spellings `use one`, `use 1` and `use [1,1]` are
semantically equivalent and accepted; non-one grades refuse at Q100.
The effect row is absent/inferred or an explicit subset of `alloc, mut` that
contains every derived direct and indirect effect. It uses explicit comma
separators, no trailing comma, appears once, and consumes the complete header.

It explicitly refuses:

* cycles, indirect calls, helper shadowing and unreachable helper definitions;
* branches, loops, records, extern calls, computed indices/extents and calls
  across other source profiles;
* `Buffer` in place of `bytes`, unsupported explicit usage grades, borrowed
  results, subloans, relending and a borrowed parameter escaping as ownership;
* a move, use or drop of a stale owner, drop/access while borrowed, incompatible
  aliases in one call and a normal-path leak;
* any call-site extent below a callee's derived transitive minimum;
* allocation failure that could leave an already-live owner without custody;
* source contracts and general typed-error payloads.

All bounds refuse without truncation. The expanded-item bound is charged before
generated argument events and operations are appended, so a high-arity call DAG
that invokes the next helper twice cannot grow exponentially up to the depth
limit.

## Diagnostic attribution

This card allocates no code or semantic pass. Frontend signature, call and graph
shape conditions use OrdinaryTypes Q020; an empty contract block is unsupported
resource-helper header syntax at Q020, while a nonempty sequential source
contract retains Q021; unsupported grades use Usage Q100. A malformed inherited buffer
statement uses StandardBuffer Q051. Core extent obligations use Q053, inherited
custody refusals Q054, independent certificate/expansion failures Q056 and
transitive effect-row refusals Q057. Tests pin representative Q020, Q053, Q054,
Q057 and Q100 paths so resource syntax, usage, extent, custody and effects do
not collapse into one buffer-shape diagnostic.

## Proof and native evidence

`QCore.ResourceCalls` models the bounded decisions for access permissions,
compatible aliases, affine owned slots, borrowed-return refusal, extent
composition, decreasing call ranks, transitive effects and the zero-live-owner
allocation-failure premise. `AuditResourceCalls.lean` audits 22 theorems. These
are decision-parity lemmas; they do not prove the complete frontend-to-C
translation. Lexical loan custody continues to rely on the existing ownership
model rather than a new equality-shaped theorem.

Rust unit tests cover the ownership path and adversarial controls: repeated
owner aliasing, stale moves, local and transitive allocation failure, uncalled
helpers, call-site minimum extents, an exponentially expanding DAG, a repeated
high-arity DAG that multiplies generated argument events and operations, a
reverse-declared chain beyond depth 64, and an invalid scalar return binding.
End-to-end tests cover routing, distinct task `Buffer`, whitespace-normalized
types, usage grades, shadowed calls, effects and diagnostic attribution. On
Linux and macOS, a Clang AddressSanitizer/UndefinedBehaviorSanitizer harness
calls the emitted `process` helper directly and asserts both returned extent
and byte value before releasing it; a silent unit entry is not the observation.

Focused pre-gate results at commit `780e72de` plus the diagnostic/docs working
tree were: core resource-call tests 11/11, compiler resource-helper tests 10/10,
and the Lean module/audit green with only the project's permitted `propext` and
`Quot.sound`. The final standing-gate measurement and exact private log paths
are recorded in `gate.json` and the section below after the semantic tip is
fixed.

## Residuals

1. **RESOURCE-HELPERS-GENERAL-ABI.** This is a closed executable profile, not a general resource-capable function
   type system or separately compiled library ABI.
2. **RESOURCE-HELPERS-UNCALLED-PROOF.** Every resource helper must be reachable from the selected entry. Universal
   argument proofs for uncalled/public helpers remain unimplemented.
3. **RESOURCE-HELPERS-SUBLOANS.** Loans cover a whole buffer for one call. Range subloans, returned borrows,
   stored loans and relending need a richer lifetime/path model.
4. **RESOURCE-HELPERS-TYPED-ERRORS.** Allocation failure has one process-exit behavior. Typed errors, recovery and
   resource payloads require a checked per-error-edge cleanup model.
5. **RESOURCE-HELPERS-C-REFINEMENT.** Lean covers the bounded decision rules, not parser extraction, graph
   flattening or C refinement end to end.

## Integrator reconciliation

1. Apply the annotation-enforcement sibling dependency first if its
   `source_annotation.rs` export is selected. This card intentionally does not
   duplicate or call that parser; its resource parameter parser remains local.
2. Reconcile `crates/qcore-frontend/src/lib.rs` by retaining both the sibling's
   `source_annotation` export and this card's `source_resource_calls` export.
3. Reconcile `crates/qcore-verification/src/driver.rs` structurally: retain the
   sibling's legacy local-annotation gate and this card's resource-call import,
   `ResourceCallsV1` profile, `CheckedBufferModule.resource_calls` field, route
   selection, checked lowering/emission and diagnostic attribution. Do not take
   either side's whole file.
4. The sibling owns annotation enforcement and any shared annotation API. This
   card owns resource helper lowering, checking and C emission. If integration
   adopts the shared parser later, preserve the exact `bytes`/`Buffer`
   distinction and the explicit `None` versus unsupported-grade refusal tests.
5. After reconciliation, run the full standing gate once on the combined tip;
   the gate here measures only this isolated resource-helper branch.
