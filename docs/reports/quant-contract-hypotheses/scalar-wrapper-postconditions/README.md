# Counter, gauge and untyped wrapper conditions

PREDICTION.md was committed before implementation at e5c2dd08b. Three model-only
scalar wrappers replace twenty sentence clauses with twenty-six typed normal
conditions; their thirty-nine entry roots are unchanged. The histogram, summary
and exponential-histogram default setters in this same file remain for later
migration. migration.json pins this stage's source identities and replacements.

The formulas state the ordinary `0/-1` result domain, direct write-op lookup
(argument index 4 equals one), null lookup failure, successful scalar-call
arguments and five ordered nested stores. Counter conditions additionally bind
the observed value read, reset/decrease refusal and the complementary success
case. The source always reads the metric value after a present lookup, even
when reset is enabled: its floating comparison is the left operand of `&&`.

The decrease predicate binds the actual C floating comparison `old > val`.
It cannot be replaced by comparing truncated integers: a negative fraction can
convert to zero while remaining a decrease from zero. Logical equality of the
floating value tokens binds exact source values; it does not mean C's `==`
operator or canonicalize NaNs. Observed comparison, bit encoding and defined C
conversion interpretations remain independent inputs.

Direct wrapper events are distinct from complete nested store/custody events.
A logging callback's scalar invocation is not the wrapper's own scalar call.
Root/map custody spans the whole call. Metric/nested custody starts strictly
after the lookup invocation returns, using an explicit return-position binding
and the after-lookup footprint. New attachment, index replacement and other
permitted lookup effects are not forbidden by a blanket before/after frame.
A failed setter does not imply rollback of those effects.

`QCore.CmetricsScalarWrapperConditions` models these branch choices with the
observed decrease Boolean as an input, records the unconditional present-metric
read, and composes the audited scalar-store model after lookup. Nine audited
properties cover result/branch/call behavior and preservation of post-lookup
ownership/state on refusal. No actual C floating comparison, source import,
backend or Rust/Lean correspondence is proved by this model. These declarations
install no ordinary interpreted C permission.

Private evidence under `private/quant-contract-hypotheses/`:

- Typed inventories 1 and 2 exited 0. Inventory 2 owns the corrected phase-specific
  frame: the first candidate was type-correct but overconstrained lookup's
  internal ownership changes. It was corrected before commitment or publication.
- `scalar-wrapper-models-2/` retains selected condition/root counts 23/13 for
  counter, 21/13 for gauge and 21/13 for untyped.
- `scalar-wrapper-lean-1/` exited 0 with nine checks and no project axioms.

Rust execution, the complete card/provider gate and native comparison remain
pending. The all-provider migration, main integration and push are open.

Inventory 3 additionally binds each successful store sequence to descendants of
the actual direct scalar invocation. Unrelated lookup/logging stores cannot
satisfy that condition. It exited 0 with unchanged clause/root counts.
`cargo check -p qcore-contract --all-targets --offline --locked` exited 0 in
`scalar-wrapper-compile-1/`. The first official sync and all-target Clippy
both exited 0 (`scalar-wrapper-records-1/`, `scalar-wrapper-clippy-1/`) before
this final event-correlation refinement; the second records/checks own the final
source identity and are recorded separately.

A separate byte-identity check compared twelve CMetrics implementation/header
files with their exact 66910c10a4d7eafa810b84229cf2dcf7b0f26f97 Git blobs. All
matched; [the identity record](../cmetrics/post-source-identities.json) retains
individual hashes. This observation confirms the read source bytes, not build
configuration, semantic correctness or a source-to-model proof.

Final official sync exited 0 (`scalar-wrapper-records-2/`), with deliberate
per-declaration clause counts and refreshed manifest/source-policy identity.
Final all-target Clippy exited 0 (`scalar-wrapper-clippy-2/`):
`cargo clippy --all-targets --offline --locked -- -D warnings`.
