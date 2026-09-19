# CT41 publication mapping

CT41 keeps its original scenario, **`volatile` used as a completion flag**, and
expected result, **no synchronization proof**. Its bounded C fixture now runs
through the source publication API and the CT harness. The full standing gate passed; see validation below.

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

Review also reproduced `CT41-MEMORY-ORDER-NAME-ONLY`: an enum declaring
`memory_order_release = 0` and `memory_order_acquire = 0` was accepted as
release/acquire even though Clang's builtin operations use relaxed order 0.
The extractor now checks each supported enum member's evaluated int value
against Clang's builtin encoding (relaxed 0, acquire 2, release 3, seq_cst 5),
then checks reference identity as before. The reproduced false certificate now
refuses `UnsupportedMemoryOrder`; normal declarations remain admitted.
Gateway and verification regression tests exercise the repair, with Lean
parity that zero cannot decode as a releasing store or acquiring load.
This finding was discovered and discharged within this card, not counted as a
retirement of an already registered entry. Before/after observations are in
`private/ct41-mapping/enum-value-probe-{before,after}.log`.


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
The audited general theorems in `QCore.AtomicPublication` establish that a
payload happens-before path requires synchronization and that plain/volatile
flags lack such a path. They are model parity, not case-bound proof evidence.
No native execution or sanitizer evidence is added by CT41.

The ledger is **50/50 mapped, 0 unmapped; 5/50 case-bound proof;
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

## Validation

[Gate](gate.json), [captured facts](gate-facts.json) and [reconciliation](reconciliation.json)
record source checkpoint `a0ca619666a06b1a3a3fb143b2a3935165148f98`.
**4,812 Rust passed, 0 failed, 10 ignored** across 246 summaries
(235 Running targets and 9 doc-test sections); **145 Lean audits;
305 Python tests**, including post-regeneration verification.

Every standing command returned 0: `git submodule update --init --recursive`,
`cargo test --workspace --no-fail-fast`, `cargo fmt --all -- --check`,
`cargo clippy --all-targets -- -D warnings`, both `qcore-compiler` examples
(`reevaluation_probe`, `reevaluation_followup`),
`python3 scripts/check_lean_audits.py`, and
`python3 -m unittest discover -s scripts/tests`. Clang was invoked and recorded
in this worktree. Cargo used two jobs, dev/test debug level 0 and incremental
compilation off. Native sanitizer checks retained normal process access.
Provider 122 is carried Unix evidence, not remeasured.

Private logs and individual exits: `/home/cosmo/Documents/GitHub/qcore-c/private/ct-remaining/integration/private/ct41-mapping/gate-20260919T110010Z/`.
The initial Python check failed because scratch tests depended on CT41 remaining
unmapped; they now construct that state explicitly, preserving the tested
refusal arms. An intermediate test edit lost shared constants and was corrected;
the initial Lean module check used an unavailable tactic and was corrected.
The first full gate was deliberately interrupted after the enum-value bug was
reproduced; its logs and interruption record remain under
`private/ct41-mapping/gate-20260919T105029Z/`. A fresh full run on the
corrected source supplies the final measurement. Development attempts are recorded in
`private/ct41-mapping/development-checks.json`. Failed or interrupted attempts
are not treated as passing gates. The complete final gate is measured above.

Registered follow-ups move **263 → 262**. The before/after residual sweep moves
54 → 55 lexical candidates, adding only the deliberately retired CT41 name;
these are not counts of open defects. Records are in
`private/ct41-mapping/residuals-{before,after}-retirement.txt`.
The final sweep has 57 candidates: it additionally names this completed card
and its discharged enum-value finding, both accounted for above
(`private/ct41-mapping/residuals-final.txt`).
