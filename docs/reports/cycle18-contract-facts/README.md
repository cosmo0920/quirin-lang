# Cycle 18 typed contract facts

This card extends the existing bounded observation-predicate consumer with a
strict policy entry point that accepts integer and text facts. It does not
change the legacy numeric APIs or their declaration acceptance. The new API is:

```rust
pub enum ContractFact {
    Integer(i128),
    Text(String),
}

pub fn check_typed_contract_facts(
    source: &str,
    function: &str,
    facts: &[(&str, ContractFact)],
) -> Result<(), String>;
```

`contract_fact_literal(source, function, fact)` is the companion accessor for
one unique, exact `ensures normal FACT == LITERAL;` clause. It lets consumers
derive fixed callback/revision/profile declarations from the same policy text
instead of copying those literals into Rust. Missing, repeated, conditional or
nonliteral shapes refuse.

## Accepted and refused surface

The selected declaration is exactly one safe, argument-free
`extern fn NAME() -> unit` with a contract, no body and no effects block. Only
`requires` and `ensures normal` are evaluated, and at least one normal
postcondition is required. The strictness belongs to the new typed API; the
legacy `check_contract_facts` and `check_contract_observation` paths retain
their existing behavior.

Integer predicates retain G06 precedence, left association, eager evaluation,
checked `i128` arithmetic, 512-token and depth-64 limits. Text literals are
ordinary frontend double-quoted string tokens decoded by the existing
`source.string.v1` decoder. Supported escapes are `\\`, `\"`, `\n`, `\t`,
`\r`, `\0` and `\xHH`; unknown/truncated escapes and decoded invalid UTF-8
refuse. Facts contain the decoded Rust `String` value. `==` and `!=` accept
integer/integer and text/text pairs. Mixed equality, text ordering, text
arithmetic and a text value in Boolean position refuse; there is no coercion.
`and` and `or` accept the integer results of text comparisons and evaluate both
arms before combining them, so neither `false and missing` nor
`true or missing` can hide an undefined fact.

The policy source is limited to 64 KiB, one invocation to 256 facts, names to
256 bytes and text facts/decoded literals to 4096 bytes. Duplicate fact names
refuse in the typed API. These are consumer bounds, not additions to the QCore
language grammar.

## Evidence

Focused Rust tests cover text/source mutations, escape decoding including a
JSON string, type mismatches, missing names in both Boolean arms, duplicate
facts, ordered/arithmetic text refusals, declaration-shape mutations, bounds,
overflow and equality of typed versus legacy numeric results. The crate run
reported 19 passed, 0 failed and 0 ignored across four test binaries, plus zero
doctests.

`QCore.TypedContractFacts` independently models the typed fact domain, eager
evaluation, same-type equality, integer-only ordering/arithmetic, checked i128
addition, duplicate environments and the strict declaration guards. Its audit
prints 11 theorem axiom closures. The permitted `propext` appears on the seven
evaluation reductions; the other four theorems report no axioms. This is a
model proof. The parser, Rust-to-Lean transcription, source bounds and the
existing string decoder remain review premises.

| Command | Exit | Result | Private log |
|---|---:|---|---|
| `cargo test -p qcore-contract --no-fail-fast` | 0 | 19 passed, 0 failed, 0 ignored | `private/cycle18-contract-facts/cargo-test.log` |
| `cargo fmt --all -- --check` | 0 | clean | `private/cycle18-contract-facts/cargo-fmt.log` |
| `cargo clippy -p qcore-contract --all-targets -- -D warnings` | 0 | clean | `private/cycle18-contract-facts/cargo-clippy.log` |
| `cd proof && lake env lean AuditTypedContractFacts.lean` | 0 | 11 theorem closures printed | `private/cycle18-contract-facts/lean-audit.log` |

The combined standing gate, probes and Python suite are integrator-owned for
the merged source-policy and identity-policy result.

## Integrator reconciliation list

1. Document the typed extension beside G06 without changing G06's legacy
   numeric grammar: add double-quoted text primaries only for the strict typed
   API; state decoded `source.string.v1` escapes, same-type `==`/`!=`, no
   coercion, and integer-only Boolean/arithmetic/ordering positions.
2. Record the public helper
   `contract_fact_literal(source, function, fact) -> Result<ContractFact,
   String>` as accepting exactly one standalone
   `ensures normal FACT == LITERAL` clause and refusing missing/ambiguous or
   conditional shapes.
3. Keep the consumer bounds explicit: source 64 KiB, 256 facts, 256-byte names,
   4096-byte text, 512 predicate tokens and depth 64.
4. Re-run the full standing gate after the two policy consumers land. This card
   intentionally does not update the global matrix, status, increment record,
   `QCore.lean` or corpus pins.
