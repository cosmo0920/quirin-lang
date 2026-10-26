# Bounded Boolean condition consumer

The preimplementation prediction is in PREDICTION.md. Four existing functions in
three provider modules now replace fifteen sentence entry roles and eleven
ordinary mathematical postconditions with typed `quant condition entry/normal`
clauses. migration.json pins the before/after source identities.

The reader recognizes complete typed formulas and parameter signatures for
modeled-object liveness, exclusive access during the selected call, and integer
membership in `{0,1}`. The modeled set must contain every mapped source object;
its interpretation and the complete call interval are inputs. Liveness and
exclusivity remain caller assumptions. Exact canonical condition-definition
identities join the task's assumption map. Neither classification nor a true
assumption-map value supplies generic graph evidence or establishes actual C
liveness, access exclusion or synchronization.

Normal assertions use the existing entry/exit ghost interpretation and checked
i64 equality/inequality/addition/subtraction/negation evaluation. Every declared
parameter requires a valuation. Only variables actually used in an expression
count toward mandatory output constraints; an unused declared output cannot
make `0 == 0` a complete contract. Unsupported roles, selectors, operators,
sorts and missing valuations refuse. Existing signature, borrow mode, source
path completeness, companion-field and helper-footprint checks remain in place.
Synthetic legacy fixtures remain accepted by the compatibility reader.

Matching `QCore.BooleanConditionProfile` proves Boolean integer encoding,
one/two-domain finite enumeration, and conditional modeled-object caller
predicates. Source extraction, predicate interpretation, actual caller
compliance and Rust/Lean transcription remain separate obligations.

Validation: `cargo check -p qcore-contract -p quirin-source-analysis -p
qcore-verification --all-targets --offline --locked` exited 0 in
`private/quant-contract-hypotheses/boolean-profile-3/`. The real typed-source
inventory exited 0 in `boolean-models-1/`, retaining four declarations,
fifteen entry conditions and eleven normal conditions. The targeted Lean
build/audit command exited 0 with five axiom checks, recorded by
`private/gate-slot/20261026T160958Z-19402-lean.json` and
`private/quant-contract-hypotheses/boolean-profile-lean-1/run.log`.
Its surrounding shell exited 1 while trying to assign zsh's readonly `status`
variable after the successful command; no wrapper-success claim is made.
Rust test execution, the full provider/card gates and native comparison remain
pending under the shared-slot policy. This is a migration checkpoint, not card
closure or permission to publish a new Safe result.

The official `python3 providers/contracts.py sync` exited 0 in
`private/quant-contract-hypotheses/boolean-records-1/`; its generated inventory
and load-path changes are included. The current CMetrics source-policy manifest
identity was refreshed from the resulting manifest bytes. All-target Clippy
`cargo clippy --all-targets --offline --locked -- -D warnings` exited 0 in
`private/quant-contract-hypotheses/boolean-clippy-1/`. The subsequent unit-test
fixture correction supplies its already-required record identity; it changes
no production semantics. The pending Rust gate will compile and exercise it.
