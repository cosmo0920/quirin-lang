# CT41 publication mapping

CT41 keeps its original scenario, **`volatile` used as a completion flag**, and
expected result, **no synchronization proof**. Its bounded C fixture now runs
through the source publication API and the CT harness. Validation is in progress.

## Attribution decision

The wave-9/wave-35 requirement that removing the named construct move the
verdict cannot demonstrate this negative synchronization claim: removing
`volatile` must not establish ordering. This mapping explicitly supersedes
that requirement **for CT41 only**, as permitted by the registered
`CT41-TYPED-SOURCE-UNDISCRIMINATING` discharge condition. It changes the evidence
criterion, not the scenario or C semantics. Earlier reports retain their
historical decisions and measured results.

The replacement criterion requires all three: a shape-validated volatile
completion refusal; a plain-flag control with the same lack of synchronization;
and an otherwise matching release/acquire atomic protocol with checked payload
ordering. Separate relaxed-store and relaxed-load controls must refuse at their
respective model obligations. The atomic type and operations change together
because C requires that pairing. No plain-versus-volatile safety difference is
invented, and a blanket qualifier rejection cannot satisfy the accepting arm.

## Implementation and scope

`extract_atomic_publication_source` now validates the bounded producer and
consumer bodies and complete declaration/reference inventory before attributing
a plain/volatile refusal. Malformed bodies, extra accesses and entry references
keep their structural refusals. Atomic certificate construction and the legacy
resource projection are unchanged. Non-atomic candidates still return no
certificate; no previous Unknown becomes Safe.

The [case and controls](../../../compiler/tests/fixtures/ct/CT41/expected.json)
execute through `compiler/tests/ct_corpus.rs::atomic_publication::run`, using
`qcore-verification` to compose source extraction and the existing core checker.
The new `atomic_publication` route credits only Clang-gated parsing and the
publication check on `case.c`. Controls do not lend stages to the case.
The [stage record](ct_stage_evidence.json) retires the wave-35 bucket by explicit
declaration; no frozen row is edited.

Exactly one publisher and one successful consumer invocation are declared
bounds. The consumer samples once and reads the payload only after the ready
value; this is no spin-loop, termination or eventual-observation proof.
Faithful Clang extraction and Rust–Lean correspondence remain review premises.
Two new audited general theorems in `QCore.AtomicPublication` establish that a
payload happens-before path requires synchronization and that plain/volatile
flags lack such a path. They are model parity, not case-bound proof evidence.
No native execution or sanitizer evidence is added by CT41.

The intended ledger is **50/50 mapped, 0 unmapped; 5/50 case-bound proof;
10/50 gated sanitized-native on Linux/macOS**. Full language conformance remains
false. Finishing the CT mapping list does not finish legacy-C analysis or certify
Fluent Bit: production source coverage, dynamic invocation bounds, termination,
arbitrary synchronization patterns and extraction/refinement remain separate.

## Reconciliation

`CT41-TYPED-SOURCE-UNDISCRIMINATING` retires through the executed case, protocol
controls and explicit attribution decision above. The old generic typed-source
qualifier refusal remains unchanged and is outside this mapping route. Its
source-model correspondence and bounded execution premises remain disclosed
under the existing trust rules; retirement claims no wider discharge.
The prior prerequisite card's instruction to leave CT41 unmapped was its own
scope restriction; this later mapping supersedes it without rewriting that card.
