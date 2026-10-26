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
formulas. Typed theorem/effect source interfaces are implemented as model representations;
their production proof/effect consumers and the remaining provider migration
are still open. Parsing/type checking supplies no source truth.

## Initial implementation

Value and relation parameters, named quantifier domains, equality, negation,
conjunction, disjunction and implication have a bounded AST and type checker.
Free symbols, arity/type mismatches, duplicate parameters, active shadowing,
unsupported syntax and excessive depth/nodes refuse. Graph-v1/v2 encodings stay
unchanged; formal atoms require graph v3. Resource/bounds readers validate the
whole graph and retain every ordinary semantic clause. Inheritance and view
rendering preserve the new introducer. The theorem/effect interfaces retain explicit model requests; strict semantic
consumers reject them.

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

## CMetrics, model prerequisites and instantiation interfaces

The second source increment adds mathematical integer terms, typed function
parameters and graph-v3 standalone `requirements`. Four recent CMetrics files
replace 105 sentence-shaped prerequisites with 161 formal clauses across 23
extern declarations. The getter bundle becomes nine separate obligations;
conversion bounds retain truncation and finite-value conditions. The exact
per-declaration replacement map is [cmetrics/migration.json](cmetrics/migration.json).
`QCore.CmetricsHypotheses` audits sixteen conditional decomposition theorems;
source relations, domains, ABI and Rust/Lean correspondence remain open.

Another 39 model-only prerequisites in seven CFL/ChunkIO files now have explicit
formulas, retaining alternatives and guarded nondefault frames. The map is
[remaining-models/migration.json](remaining-models/migration.json). These declared
models supply no ordinary C summary. Active source/custody, Monkey structural
and observation policies require coordinated consumer migration and remain open;
administrative cleanup/counter selectors retain their configuration meaning.
The private census routes 295 occurrences across those families, rather than
claiming that all-provider migration is finished.

`quant theorem` has a typed application/schema interface, checked symbol
substitution and capture refusal. Instantiation retains all prerequisites and
conclusion in a sealed record with model-proof/source bindings open. Regression
fixtures cover input/output/filter/processor/custom role requests with explicit
instance, descriptor, configuration, callback, ABI, lifecycle, custody and effect
parameters. They are requests, not proven concrete plugin instances.
`QCore.QuantTheorem` audits five conditional application properties.

`quant effect` retains entry/exit states, selector, guard, read/write terms,
frame and transition. The CFL atomic model covers selected GCC/Clang backends
with five own-event cases (initialize/store/load/CAS success/CAS failure).
Strong CAS does not update a separate caller expected slot on failure. Generic
mutex and MSVC behavior require separate models; the current profile does not
admit them. `QCore.QuantEffect` audits five frame/guard properties and
`QCore.CflAtomicEffects` audits seven own-step properties. No atomicity,
synchronization or imported-C execution is established by a declaration.

### Validation so far (not a completed card gate)

* CMetrics typed inventory: exit 0, 23 declarations / 161 formal clauses.
  `private/quant-contract-hypotheses/cmetrics-adoption-2/`; the preceding
  attempt exited 1 for formula grouping and is retained.
* Additional model inventory: exit 0, 39 replacements / seven files.
  `private/quant-contract-hypotheses/remaining-models-1/`.
* Atomic typed inventory: exit 0, four declarations / seven prerequisites /
  five effects. `private/quant-contract-hypotheses/atomic-model-1/`.
* Focused theorem/effect test compilation exited 0. Their targeted Lean audits
  exited 0 (five checks each), under `private/quant-theorem-agent/` and
  `private/quant-effect-check-1/` respectively.
* CMetrics targeted Lean build/audit exited 0, sixteen checks:
  `private/quant-contract-hypotheses/lean-cmetrics-1/`.
* Atomic targeted Lean build/audit exited 0, seven checks:
  `private/quant-contract-hypotheses/lean-atomic-3/`. Attempts 1/2 exited 1
  for a reserved binder and Lean-4.35 simplification/deprecation errors; both
  failed attempts are retained.

The earlier prototype scoped gate failed Clippy, and its queued Rust step was
explicitly superseded before executing tests. The inherited original main gate
had five failures as recorded above. Neither attempt is green. The expanded
Rust suite, final full/provider gates and new native measurements remain due.

The complete workspace/all-target compile attempt in `compile-3/` exited 101
because the corpus test's exhaustive clause-kind match lacked the new forms.
The corpus reader now validates each formal clause with its typed reader.
The subsequent `cargo clippy --all-targets --offline --locked -- -D warnings`
exited 0 (`private/quant-contract-hypotheses/clippy-3/`). Provider inventory sync
exited 0 in `provider-records-3/`; the reviewed changes include the new four-extern
CFL atomic module and the model source digests. The corpus expectation deliberately
becomes 144 modules / 365 non-v1 declarations, with resource/bounds counts unchanged.

## Explicit condition selectors

The additional user decision introduces `quant condition entry|normal|error(E)|any`
(G01c). [The condition record](condition/README.md) owns the initial real
postcondition migrations, matching audited selector semantics and validation.
[The plugin record](plugin/README.md) owns the 1,162 typed plugin atom replacements,
166 refreshed open manifests and exact reviewed C identity preservation.
The all-provider pre/postcondition migration and final landing remain open.
