# Quantified contract hypotheses across providers

## Scope and current state

The user requests migration across all providers, including sentence-shaped
`requires`, opaque hypothesis/premise strings and outer-contract assumptions.
The baseline source census covers 144 provider `.qv` files and 838 extern
declarations: 4,233 requires clauses with 423 unique expressions; after
excluding all 603 interpreter/version markers, 435 bare condition clauses;
1,179 hypothesis strings, 223 premise strings and two outer prose definitions.
These lexical counts are a migration inventory, not contract truth or coverage.

The scope includes the Quirin v0.3 normative language/contract specifications,
formal source syntax, a typed representation in the proper schema/core/frontend/
contract layers, matching Lean definitions and audits, and provider migration.
The preimplementation prediction is in [PREDICTION.md](PREDICTION.md).
The new specification requirements remain unimplemented until the corresponding
consumer and migration gates have passed. No provider migration is claimed yet.

## Semantic requirements

A proposition denotes a state predicate. A theorem application names an audited
model theorem, instantiates its quantified arguments and exposes every remaining
premise. An effect denotes a relation between explicit pre/post states with its
read/write footprint and frame condition. A theorem of the model establishes
no imported-C fact unless the source-to-model binding is independently checked.

A condition that lacks a sound decomposition remains an explicitly named open
obligation during migration; renaming it or encoding its English as a string
is not a completed migration. Preserve disjunctions, implications, exceptional
exits and dependency invalidation. The implementation must neither promote an
Unknown result nor silently drop an existing condition.

## Landing

This work extends the authorized LANDING-PUBLISH landing. After integration,
run the full standing/provider gates, compare against the committed prediction,
publish an immutable baseline for the final clean commit, merge onto main and
push to the configured remote. Earlier gates and measurements retain their
original revision identities.
