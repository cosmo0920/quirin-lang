# Cycle 13: Translation Validation and Analysis Hardening

**Status: Done**

Cycle 13 adds an independent source/CFG/C11 translation validator and hardens
the conservative legacy-C analyzer without widening its safety claims.

## Rust implementation

`compiler/src/translation_validation.rs` reconstructs cleanup lowering at the
pass boundary, independently interprets source and CFG, parses the emitted C
observation, and returns a certificate only when all three observations agree.
An exhaustive small Boolean corpus covers acquisition and check outcomes.

The legacy-C importer now:

- offers a Clang C11 syntax gateway before conservative projection;
- recognizes direct struct fields, pointer fields, and constant array places;
- computes translation-unit status from retained per-function summaries;
- treats only separately checked internal functions as resource-neutral calls;
- widens alias sets after eight targets; and
- turns every widening or unmodeled construct into `Unknown`, never `Safe`.

The interprocedural slice does not infer resource transfer through arbitrary C
parameters, function pointers, dynamic array indices, unions, or casts. Those
remain `Unknown`. When Clang is available, the exhaustive acquisition/check
corpus is compiled with AddressSanitizer and UndefinedBehaviorSanitizer, run,
and fed back through the independent translation validator. Platform-specific
sanitizer availability is not a proof gate.

## Lean verification

`proof/QCore/TranslationValidation.lean` proves that a successful validation
certificate preserves the pass boundary and makes source, CFG, and target
observations equal. It also proves that a widened legacy-C state cannot receive
a safe verdict. `proof/AuditCycle13.lean` audits those dependencies.

## Verification

```sh
cargo test --workspace
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
if rg -n '\b(sorry|admit)\b|^[[:space:]]*axiom\b' proof --glob '*.lean'; then exit 1; fi
cd proof
lake build
lake env lean AuditCycle13.lean
```

Cycle 14 is the next implementation milestone.
