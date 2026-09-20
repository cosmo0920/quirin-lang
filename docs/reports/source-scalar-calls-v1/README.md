# SOURCE-SCALAR-CALLS-V1

2026-09-20. Base: `418af31dfa4567085f68882d7fab7d2e69a042c7`.
Implementation and validation record for ordinary Quirin scalar functions and explicit standard I/O.

## Implemented boundary

`source.scalar-calls.v1` admits explicit i32/unit signatures, i32 parameters,
immutable i32 locals with optional annotations, direct nested source calls,
parentheses, unary minus and `+ - * / %`, with a final explicit return.
Multiplication/division/remainder bind more tightly than addition/subtraction;
binary operators associate left. Calls and operands evaluate left to right.
A unit function may return a unit call; unit values cannot be stored in locals
or passed as i32 arguments. Call statements may discard either result type.

The core type-checks every definition, including uncalled ones, and rejects
cycles across the entire call graph. Signatures and bodies remain in a private
checked value; C emission consumes that value, retains real functions and calls,
and never substitutes a precomputed entry result. Library modules have no entry
observation or C artifact. An executable has one closed, zero-argument entry.
Its interpreter result is published separately from the all-definition type check.

Arithmetic is checked signed i32. Division truncates toward zero; remainder has
the dividend's sign. Zero divisors and MIN/-1 division **and remainder** refuse.
The closed entry is evaluated during checking, so its overflow refuses with Q020.
Other input values are not claimed overflow-free. Emitted C uses widened int64
arithmetic and explicit narrowing/division guards, terminating silently with exit
86 on a fault. No wrapping or undefined signed overflow is used. There is no
automatic result banner or stderr diagnostic. Functions cannot allocate, transfer
ownership or throw.

`import std.io; print(f);` evaluates the i32 expression `f` and writes its decimal
value plus a newline to stdout through the existing `io_emit::write_line` backend.
No std.fmt import is needed for this form. I/O is inferred when the effect row is
absent; `effects {io}` permits it, and `effects {}` refuses direct or transitive
I/O, including calls inside expressions. Every definition is checked. Other
effects and result/argument types remain refused.

The C backend contributes file-scope fragments and an entry body. The existing
`c_program` assembler supplies exactly one `main`; fragment merging deduplicates
shared I/O helpers. This is emission composition, not general cross-profile call
typing. Ordinary scalar programs produce no stdout or stderr.

Bounds: 256 functions, 16,384 expression nodes across the IR, 16,384 source tokens
per function, expression and call-graph depth 64, and 100,000 evaluated expression
steps per closed invocation. Parser recursion and left-associated chains are
bounded independently. Bounds refuse rather than truncate. No runtime branches,
loops, mutation, generics, indirect calls or cross-profile calls are admitted.
Usage annotations remain on the old route unless a new expression/call feature
selects this profile; the new profile refuses them rather than skipping them.

## Integration and compatibility

The implementation lives in qcore-core (typed IR, checker, interpreter and C
backend), qcore-frontend (reader over retained post-desugaring tokens), and
qcore-verification (driver). The CLI names the new profile. No provider, C gateway,
contract corpus, ownership rule or existing route implementation is changed.
Selection happens after specialized source profiles; function-style print selects
this route before the legacy std.io rendering route. A nonliteral return/local,
typed local, or argument-taking call to a declared function selects the new
profile. Sequential qcore_acquire/qcore_check keep their custody route; the
qcore_ prefix is not reserved. Zero-argument/literal legacy programs keep their
route. Existing while and usage-diagnostic pins are explicitly tested.

No words or tokens are reserved. New conditions extend OrdinaryTypes Q020;
nonempty source contracts retain Q021. The diagnostic namespace is not expanded.
Initial compatibility testing caught while being misclassified as a call; the
selector now requires a declared function at the statement call position.

## Proof and validation

QCore.ScalarCalls defines checked arithmetic, argument binding, immutable frame
extension, decreasing call ranks, and relational expression/body/call semantics.
AuditScalarCalls audits range and arithmetic exactness, invalid division,
argument binding, cycle refusal, slot preservation, unit result separation and
actual quiet increment(41)=42 and printing derivations. Output traces compose
in left-to-right order, and audited effect-row rules cover direct/transitive I/O.
Rust reads the eight Lean arithmetic agreement vectors and checks them against its kernel. This finite differential
and reviewed operational correspondence are **not** a compiler extraction or
universal Rust-to-Lean refinement proof. Implementation budget counters and
source parsing are outside the relational theorem claims.

Focused tests additionally compare 500 arithmetic boundary combinations against
widened arithmetic, exercise nested calls, type errors, uncalled bad definitions,
mutual recursion, bounds, library mode, multi-file input, and old route selection.
Sanitized native checks exercise quiet emission, explicit output order, single-main
composition, the real call graph and five faulting helper
inputs (addition overflow, signed division/remainder overflow, zero division,
unary negation overflow). A first sandbox run preserved its LeakSanitizer ptrace
failure; subsequent native runs use the ordinary host with sanitizers enabled.
A fixture omitted its error declaration initially; that fixture was corrected.
The added print tests initially expected two diagnostic strings from later passes;
the observed earlier fail-closed diagnostics were retained and the new expectations
corrected. The final focused suite passes 21/21 without changing older pins.

The composition test joins scalar fragments with a separate contribution of the
shared IO fragments and another IO entry action. Sanitized execution observes
`20\n22\n42\n99\n`, one prelude and one C main. A separate CLI check emits two
Quirin source files together and observes one main and `42\n`; its negative
control confirms a caller's std.io import does not authorize printing in a
callee file that lacks the import (`private/source-scalar-calls-v1/multifile/`).
[CLI recordings](cli.json) bind the two published examples to the source commit,
source/C digests and exact native output. `initial-cli.json` and
`initial-source-hashes.json` explicitly preserve the superseded pre-steering
checkpoint, including its removed automatic banner.

The GPT-5.6 Sol medium review covered selector/lowering, effects, evaluation
order, arithmetic guards and main assembly; it found no remaining concrete
blocker. Its proposed zero-argument-call counterexample was executed and refused
as required, so the concern was retracted rather than causing an unnecessary
selector change. See [review](review.json).

The first full workspace run also caught two example-publication failures: the
new family lacked its README and index link at that checkpoint. Both links were
added. That run was then stopped when the user requested silent emission and
explicit std.io printing; its logs and interruption reason remain preserved. A
fresh gate measures the final implementation, including those requirements.

## Final standing gate

[Gate](gate.json), [run identity](run-identity.json), and [source hashes](source-hashes.json)
record **4,884 Rust passed, 0 failed, 10 ignored**, 251 result summaries across
249 targets (240 running targets and 9 doc-test targets), **150 Lean audits** and
**313 Python tests**. Five required workspace host-sensitive rows also pass
individually. Clang 21.1.8 was invoked in this measuring checkout; **122 provider
tests are carried Unix evidence, not remeasured**. No pinned C corpus count is
remeasured or promoted by this card.

The Rust/fmt/Clippy/probe source checkpoint is `455108be2bc64830b386a7ebd6c8d8f217eb0e4c`.
The first full Lean build exited 1: the new import was incorrectly placed after
the module doc comment in QCore.lean. Commit `62129bd2` moves that import into
the import section; all 150 audits then pass. This correction changes no Rust,
example or scalar-model definition, so only the affected Lean gate was repeated.
The original failed log remains in `gate-final/lean.log`.

| Command | Final exit / result | Private log |
|---|---|---|
| `git submodule update --init --recursive` | 0 | `gate-final/submodule.log` |
| `cargo test --workspace --no-fail-fast` | 0; 4,884 passed / 0 failed / 10 ignored | `gate-final/rust.log` |
| `cargo fmt --all -- --check` | 0 | `gate-final/fmt.log` |
| `cargo clippy --all-targets -- -D warnings` | 0 | `gate-final/clippy.log` |
| `cargo run -p qcore-compiler --example reevaluation_probe` | 0 | `gate-final/probe.log` |
| `cargo run -p qcore-compiler --example reevaluation_followup` | 0 | `gate-final/followup.log` |
| `python3 scripts/check_lean_audits.py --jobs 2` | 0; 150 audits | `lean-final-rerun.log` |
| `python3 -m unittest discover -s scripts/tests` | 0; 313 tests after regeneration | `python-final.log` |

Private paths above are relative to
`/home/cosmo/Documents/GitHub/qcore-c/private/source-scalar-calls-v1/`.
`isolated-final/facts.json` records the five isolated commands, exits and logs;
`source-scalar-calls-tests-final-2.log` records the focused 21/21 run.
The matrix retains 271 named follow-ups and updates R04 with this profile's
source, model, observation and audited Lean evidence independently. R04 stays
partially repaired. Reconciliation verifies each applied quotation; the residual
scan finds 55 untracked name-shaped candidates: the previous 54 plus this
completed card name, introduced by its increment entry. The added candidate is
classified as discharged by this report, not new open work; no tracker entry is
retired. See the recorded residual assessment in run-identity.json.

## Integrator reconciliation list

- Apply the grammar/specification and source-route diagnostic amendments together;
  this is a bounded scalar/stdio profile extension, not general type-system conformance.
- Keep runtime conditions, profile composition and record/resource call values
  open. Existing IF-RUNTIME-CONDITION and AGGREGATE-ACROSS-CALL are not discharged.
- Publish measured gate counts after validation and regenerate the matrix. C
  projection counts are carried evidence: this card does not remeasure that corpus.
