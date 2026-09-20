# Cycle 18 observed-composition context admission

This card adds one bounded prerequisite for
`MONKEY-CALLBACK-CYCLE18-COMPOSITION`: a caller-held expected context must
exactly match the context presented with an observed run, and the observation
and binding payload digests must be recomputed from the presented bytes. The
card does not publish a composition verdict and does not close that follow-up.

## Boundary

`EventCompositionContext::derive` constructs a complete context for the fixed
property `cycle18-observed-callback-composition` and evidence version
`qcore.cycle18.observed-composition.v1`. It binds:

- artifact, semantic profile, callback scope, bounds and assumptions;
- source compile-task identity, Fluent Bit revision, and Monkey source,
  profile and build identities;
- libevent revision, source, profile and build identities;
- contract, source-rule and observation-rule identities;
- observer binary, harness and linked Monkey/libevent library identities;
- the explicit producer-supplied observation-trust premise; and
- SHA-256 identities derived from the exact observation and binding bytes.

Construction refuses empty payloads, empty or non-canonical claim fields,
empty scope/bounds/assumptions, zero bounds, malformed lowercase SHA-256 or
40-character revision identities, and missing Monkey or libevent linked-library
identities. `check_event_composition_context_for` validates both contexts,
rehashes the presented bytes, and compares every field with an independently
held expected context before returning a private-field,
non-serializable `ValidatedEventCompositionContext`.

The token exposes immutable context accessors and a deterministic context
digest for downstream comparison and reporting. Digest equality identifies the
recorded bytes and fields; it does not attest the compiler, source checkout,
observer, harness, linked libraries, runtime or host.

The public implementation is in
`crates/qcore-verification/src/event_composition_context.rs`. Focused tests in
`crates/qcore-verification/tests/event_composition_context.rs` cover the
positive control plus substitutions of claim, scope, bounds, assumptions,
source/profile/build, contracts/rules, observer/library identities, stale raw
payload bytes, coherent alternate payload/context pairs, and incomplete or
malformed contexts.

## Lean correspondence

`proof/QCore/EventCompositionContext.lean` models completeness, exact expected
context equality and payload rederivation. It proves that acceptance implies
both contexts are complete, the presented context equals the caller-held
expected context, and both payload identities match. General unequal-context,
scope-substitution and observation/binding-substitution theorems prove refusal.
`context_validation_remains_unknown` makes the verdict boundary explicit.

The Lean model abstracts validated strings and digests as positive natural
numbers. Rust performs concrete canonical-form and SHA-256 checks; the Lean
result is an equality/refusal theorem after those representations have been
derived. `proof/AuditEventCompositionContext.lean` audits all eight theorems.

## Evidence and gate

The semantic implementation and Lean module at `f39f253a` passed the full Rust
workspace: **4,939 passed, 0 failed, 10 ignored** across 258 strict
`test result:` summaries. The final commit `bd4f7be9` adds immutable getters
only. Its full-workspace confirmation exercised every target but exited 101
because `qcore-verifier-host --test enforced_process_limits` reported one
host-sensitive fork-control failure (`Broken pipe`): 11 passed and 1 failed in
that binary. An immediate isolated rerun passed **12/12**. This report preserves
both results and does not relabel the exact-tip repeat green; the integration
branch's combined gate supplies final exact-tip workspace coverage.

At `bd4f7be9`, formatting, clippy, the 8-case reevaluation probe, the 11-case
reevaluation follow-up and all **154/154** Lean audits passed. The Python suite
passed **317/317** after a scratch acceptance-matrix regeneration accounted for
the new audit file; the scratch matrix was then restored, because the
integration owner reconciles the generator-owned audit count. The recorded
Clang banner is `Ubuntu clang version 21.1.8 (6ubuntu1)`.

Private logs are under
`private/cycle18-composition-admission-gate-20260920/` in the card worktree.
`gate.json`, `clang-version.txt` and `clang-version.exit` preserve the public
gate record.

## Residuals and reconciliation

- The downstream join must independently derive its expected identities from
  configured files and immutable prepared inputs. Presented metadata cannot be
  its own expectation.
- A validated context must be joined with the matching sealed source
  projection and checked observation/event-program evidence. Context admission
  alone remains `Unknown` and is never `Safe` or `Composed`.
- Declared producer observation trust is a recorded premise. Runtime provenance
  and attestation remain undisclosed by this card.
- The first observed-run artifact retains all callback-accounting rows as
  `Unknown` unless the later join discharges the existing callback claims.
  General task and deferral semantics remain unsupported.
- The integration owner must regenerate
  `docs/reviews/r0_3/acceptance_matrix.json` so its derived Lean-audit count and
  file list include `AuditEventCompositionContext.lean`; this card deliberately
  does not own that global matrix.
- `docs/STATUS.md`, the callback coverage report and corpus counts are unchanged.

