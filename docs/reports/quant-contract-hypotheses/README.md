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
The first source increment implements `quant proposition` (G01p) and graph
version 3, replacing the eight opaque libc reading atoms with typed first-order
formulas. The theorem/effect source interfaces and the remaining provider
migration are still open. Parsing/type checking supplies no source truth.

## Initial implementation

Value and relation parameters, named quantifier domains, equality, negation,
conjunction, disjunction and implication have a bounded AST and type checker.
Free symbols, arity/type mismatches, duplicate parameters, active shadowing,
unsupported syntax and excessive depth/nodes refuse. Graph-v1/v2 encodings stay
unchanged; formal atoms require graph v3. Resource/bounds readers validate the
whole graph and retain every ordinary semantic clause. Inheritance and view
rendering preserve the new introducer. Unsupported theorem/effect forms reject.

`libc/errno.qv` now expresses the guarded reading-route alternative and the
remaining seven predicates mathematically, with explicit environment/site and
relation arguments. Primitive relation interpretations remain open obligations.
`QCore.QuantProposition` gives first-order semantics and six audited logical
properties; existing `QCore.LibcReadingPremises` gives the concrete conditional
libc decomposition. No Rust/Lean or imported-C refinement theorem is claimed.

The official pin command refreshes only the two flexible libc declaration
digests (`__errno_location`, `strerror`); it changes no reviewed C identity.
`providers/contracts.py sync` also repaired preexisting stale inventories for
CMetrics map source and CFL's initializer/hash model. The original main gate
found five failures: those stale inventories, the stale CMetrics source-policy
manifest digest, and the corpus test's missing CFL hash module/count. The live
source-policy digest and deliberate non-v1 count are updated to the already
tracked source. Historical reports and model source are unchanged.

The first inventory sync refused stale libc pins; its parent wrapper's final
exit did not capture that command separately. The refusal log is retained as
`private/quant-contract-hypotheses/implementation-1/contracts-sync.log` and is
not a successful gate. The retry after official pinning captured exit 0 for
both commands. Compile check before the provider migration exited 0; the final
standing/scoped/provider gate owns the complete implementation's evidence.

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
