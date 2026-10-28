# RULE37-PILOT: rule 37 as a rule package

Card RULE37-PILOT, the second pilot of `GATEWAY-RULE-API-PILOT-MIGRATIONS`
([plan](../../plans/pluggable-cards/README.md) §3 B and §4;
[interface report](../gateway-rule-api/README.md) §7). Base `80c8aea98`.
Implementation `6b6a3a438`; [prediction](PREDICTION.md) committed before it
(`1ac59451c`).

## 1. What moved

Rule 37 ([rule](../../agents/rules/37.md),
[card](../w48-bu-rk-result-constructors/README.md)) has three parts in the
gateway. Two are admissions and moved; one is a guard and stays.

| Part | Before | After |
|---|---|---|
| Typed `sizeof` exemption: a mention inside a checked, non-VLA `sizeof` operand is not an escape of the store base | inline in `aggregate_store::store_bases` and `insulated_bases` | package arm `sizeof-operand-exemption` (typed reader); the host computes the checked spans with its existing reader and asks the package |
| `asserted_allocation` premise: a checked specification naming it is a wrapper whose non-null result rests on a terminating assertion | contract text only, adopted by the generic checked-specification settlement (W45) | package arm `asserted-allocation-premise` at `legacy_checked_spec::settle`: such a specification stands only while rule 37 admits it, and the admission carries `Obligation::Premise("asserted_allocation")`, which the declaration must state (the first use of `Obligation::Premise`) |
| Null readers' checked-result guard: a checked custody specification is never an unchecked non-null fact | `null_deref.rs`, `walk.rs`, `witness.rs` | unchanged, in the host, never switched off: it is a restriction, so switching it off would widen |

Files:

- `crates/gateway-rules/rule-37-result-constructors/` (new): the package
  (`ResultConstructors`, `ID = "37"`, `ASSERTED_ALLOCATION`), its span
  well-formedness check (fail-closed: spans that do not start at `sizeof`, run
  past the body, cover no operand, overlap or are out of order refuse the whole
  answer) and 6 tests, including Rust vectors of the Lean switch theorems and
  the dependency-set test (the rule interface only).
- `crates/qcore-gateway-rule-api/src/checked_readings.rs` (new, additive): two
  host capabilities whose queries need none of the host's vocabulary,
  `CheckedOperandArm` (§7) and `SpecificationPremiseArm` (§2.2/§6), and one
  test. Nothing existing in the API changed.
- `crates/qcore-c-gateway/src/result_constructor_rule.rs` (new): the host side,
  building the queries and composing answers with `consult` under
  `gateway_rules::enabled("37")`; `aggregate_store.rs` asks it for the exempt
  spans; `legacy_checked_spec.rs` asks it for premise withdrawals (new
  `Withdrawal::PremiseNotAdmitted`; unreadable premises withdraw every checked
  specification, as an unreadable checked set already does).
- Registry row `37` (crate `rule-37-result-constructors`, Lean
  `QCore.ResultConstructors` and `QCore.ResultConstructorSwitch`, `off =
  "refuses"`), regenerated `gateway_rule_registry.rs`; `legacy_route_table.rs`
  is unchanged (arms have no table). `known_rule` now accepts `37`.
- Workspace member, gateway dependency, `workspace_layering.rs` pin (the rule
  crate may depend on the API only; the gateway on the rule crate), the
  gateway's registry test (a body lives in the gateway or in its own
  `rule-<id>-*` crate), `tests/rule37_switch.rs`, rule document 37, the
  architecture row, `proof/QCore.lean` (generated).

**Placement note.** No route-map position names the checked-specification
settlement: it runs before the parser and decides which declarations the
classifier's contracted arm sees. The premise arm is registered at
`classification` because that is where its effect is read (§6, the contracted
or uncontracted arm of a call). A hold position would have forced
`not-switchable`, which is wrong here: refusing withdraws.

## 2. What off means

`QCORE_GATEWAY_RULES_OFF=37` asks neither arm.

- The store scan gets no exempt span. That is the scan's code before rule 37
  (`1f1615cfa^`) exactly, so a `sizeof(*e)` mention is a dereference of the
  base again (premise 4) and the store is refused: the fixtures of
  `result_constructors.rs` and `checked_allocator_constructors.rs` keep the
  refusal `UnknownAlias e` + `indirect memory access is outside the resource
  projection` (`rule37_switch.rs`).
- Every checked specification naming `asserted_allocation` (today `rd_calloc`,
  `rd_strdup`) is withdrawn with "the gateway rule owning its named premise
  does not admit it (switched off)", so the defining rows are `Unknown`.
- **Boundary, stated rather than closed.** A withdrawn checked specification
  leaves its callers whatever W4 validates from an in-unit body (the existing
  W45 withdrawal). For an asserting wrapper body that is the non-null result
  itself, so a constructor with no `sizeof` (`rd_calloc(1, 16)`) stays `Safe`
  with rule 37 off, on W4's summary, whose row names no
  `asserted_allocation` premise. This is no Unknown-to-Safe by the move (the
  row is `Safe` with 37 on as well) and pre-dates rule 37 for any undeclared
  asserting wrapper; the contract environment before rule 37 (nullable
  `inherit libc::calloc`) cannot be restored by a switch, and contract edits
  are out of scope. Registered as `RULE37-W4-ASSERT-PREMISE-VISIBILITY`.

Lean (`QCore.ResultConstructorSwitch`, 13 theorems, `AuditResultConstructorSwitch.lean`):
the switch on is rule 37's model (`the_switch_on_reads_rule_37`,
`the_switch_on_is_the_constructor_step`), off is the old scan
(`the_switch_off_reads_the_old_scan`), off never widens the scan or the
specifications (`the_switch_off_never_widens_the_scan`,
`the_switch_off_never_widens_the_specifications`), off withdraws exactly the
premised specifications and an admitted one carries the premise. Premises kept
separate: Rust/model correspondence, fidelity of the host's checked spans and
of the declared premises.

## 3. Identity run and contribution

MEASUREMENT-PENDING

## 4. Gate

GATE-PENDING

## 5. Parity cost (plan decision 6)

PARITY-PENDING

## 6. Likely conflicts and follow-ups

Files RULE35-PILOT will also touch: `crates/gateway-rules/registry.toml` and
the generated `gateway_rule_registry.rs` (rerun
`scripts/gen_gateway_registry.py`), `Cargo.toml` members and `Cargo.lock`
(take both crates, `cargo update -w`), `compiler/tests/workspace_layering.rs`
(both rule crates and both gateway edges), `crates/qcore-c-gateway/Cargo.toml`,
`crates/qcore-c-gateway/src/lib.rs`, the registry test in `gateway_rules.rs`
(both relax the all-in-gateway assertion), `crates/qcore-gateway-rule-api/src/lib.rs`
(module and re-export lines; this card's capabilities are in their own file),
`proof/QCore.lean` (rerun `scripts/gen_lean_roots.py`),
`docs/development/architecture.md` (the `crates/gateway-rules/` row) and
`docs/agents/gateway.md` (rerun `scripts/gen_rule_docs.py`).

FOLLOWUPS-PENDING
