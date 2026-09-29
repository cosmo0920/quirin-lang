# CG03-CALL-SUMMARY-INSTANTIATION: one call-contract application point

Card CG03 of the [Clang gap card queue](../../plans/clang-gap-cards/README.md)
(wave 48, round 3; base `f19204fa` = local and origin `main` at dispatch).

- **Gap entries:** K12 and K03, on route **G** (the custody gateway).
- **Evidence:** Q17, Q19, Q20, Q31, Q42.
- **Rule:** [gateway 10u](../../agents/gateway.md#constraint-10u).
- **Lean:** `QCore.CallInstantiation`, `proof/AuditCallInstantiation.lean`.
- **Prediction:** frozen in [PREDICTION.md](PREDICTION.md) (commit `98dc7708`).
  It was committed before the implementation commit and before any AFTER
  run. Fixture probes of the routes had run before it.

## 1. What changed

### 1.1 The application point (`legacy_call_instantiation.rs`)

`Parser::instantiate_call(callee, statement, destination, use)` is the one
place a consumer asks for a call's contract. **Cards CG04 and CG05 call this
API.**

It runs the steps in this order:

1. It reads the callee's **standing**, before the classifier runs:
   - declared (no body in the unit);
   - validated (a local body outside the caller's recursive component);
   - recursive.
2. It runs the classifier's one lookup and per-formal substitution:
   `classify_simple`, with every withdrawal, hold and premise rewriting.
3. When the lexical gate refused the actuals, it takes the statement
   position's typed argument reads (W44-INDIRECT-CALL-ARGUMENT-ACCESS: the
   `IndirectScalarAccess` / `ArgumentReads` obligations before the call).
4. It returns the `ContractCall` (with its reads) that the flow applies once
   (`apply_contract_call`, unchanged). No consumer builds a second model.

### 1.2 Acceptance (`call_instantiation.rs`)

`call_instantiation::accept(standing, result class, identity, use)` decides
whether a consumer takes the instantiated call. It is a function of the use
C makes of the call's **value**, never of the consumer:

| Use | Accepted when |
|---|---|
| any | the callee is not in the caller's recursive component (constraint 4's provisional summary is not admission evidence) |
| discarded, tested (a condition leaf, a scalar `return` operand) | the result owns nothing (`None` or derived) |
| stored | always (the classifier's own binding rules follow) |
| returned (a whole `return` value) | the result is `None` or fresh `Owned` (card C-GATEWAY-RETURN-CALL-W4) |
| passed (an actual) | the call is the identity with no result (card B04; widening it is CG05's) |

The condition desugaring (rule 5h) now asks `instantiate_call` for every
leaf call. It maps each refusal to its own census vocabulary, unchanged.

### 1.3 New consumers

Each is a fallback asked after every older route of its position (hard
invariant 3). A declined consultation restores the site and element counters.

**A scalar `return` operand** (`instantiated_return`, asked after the CG02
elaborator). It applies in a function whose definition declares a scalar
result, when the refused `return` expression is one condition leaf
(negations allowed, no connective). The leaf is lowered exactly as in an
`if`:

- its calls, each through `instantiate_call` with use *tested*, two only when
  each is the identity;
- then an exit that escapes nothing.

It keeps two premises:

- rule 5j's: no unary `&` outside the calls' actuals;
- the reserved-place guard.

B01's older gate (externally declared callee, `None` result) is contained in
this acceptance, and the older route is still asked first.

**A call whose actuals carry effects** (`effectful_call_piece`, a hook in
CG02's statement-piece lowering). A piece that is exactly one direct call is
lowered in two parts:

- first its elaborated events, which C sequences before the call
  (`ForgetScalar`, typed path accesses);
- then the call, with each effectful actual replaced by data at a `Borrowed`
  formal.

A pointer-valued effectful actual, a custody formal and a nested call all
refuse. This closes CG02's `Refusal::EffectfulActual` for `cg_sink(i++)`.
CG02's shared `lower_events` was split into `lower_event` without a change of
meaning.

**A typed substitution** (`substituted_call`, asked last in
`expression_statement`). It applies to a call statement in two cases:

- the lexical substitution was refused (`call has an unmodeled formal address
  or borrowed expression`);
- a custody formal was left empty, which the flow reports as `arguments do
  not match resource contract`.

The actuals are then read from Clang's AST by
`call_instantiation::substitute` (value → identity → formal):

- a named pointer object through value-preserving pointer conversions
  (parentheses, `LValueToRValue`, `NoOp`, a `BitCast` between object
  pointers) is its identity;
- `&x` at an `Out` formal is the slot;
- an expression whose evaluated reads are scalar is data (a `sizeof`/
  `_Alignof` operand is unevaluated, a VLA operand refuses);
- a member, subscript or indirection is a place read by CG10's
  `typed_place::read_place`, observed only at a `Borrowed` formal.

A place, an address or a scalar at a custody-taking formal refuses
(`W44-CALL-ARGUMENT-CUSTODY-POSITION` by design; a scalar custody actual is
the descriptor route's). So do pointer arithmetic (CG16), a nested call
(CG05), a conditional or value connective (CG06) and any other cast (CG08).

The statement is then reclassified with the actuals respelled as the AST
reads them. It is admitted only when every custody formal is bound to exactly
the identity the AST names.

### 1.4 Shared items added (for the integrator)

- **`elaboration.rs`:** unchanged.
- **`typed_place.rs`:** unchanged. The substitution calls `read_place`.
- **`legacy_elaboration.rs`:**
  - `elaboration_node` is `pub(super)`;
  - `lower_events` now loops over the new `pub(super) lower_event`;
  - one hook in `lower_step` (`effectful_call_piece`).
- **`legacy_condition_desugar.rs`:**
  - `is_identity_call` and `recursive_local_callee` are `pub(super)`;
  - the new `pub(super) single_leaf_plan`;
  - the leaf loop asks `instantiate_call`.
- **`legacy_c.rs`:** the module declaration, and three hooks:
  - `return` after the elaborator;
  - `expression_statement` last;
  - parity beside the W4 and B01 return admissions and every statement
    `ContractCall`.
- **`lib.rs`:** `mod call_instantiation`.

## 2. The cited claims

CG00's controls and the card's own BEFORE runs were checked against the
input's claims:

| Claim | Reproduced? |
|---|---|
| K12: one contract is accepted in one position and refused in another | **Yes.** Two cases. (a) `h(p) != 0` with a local callee: admitted in an `if` (rule 5h), refused in a `return` (B01 took only an externally declared callee). (b) `r_borrow_code(&s->a)`: admitted as a statement (W44), refused in an `if` and a `return`. Both are executed BEFORE controls: the new tests fail with the route disabled. |
| K12 on the corpus: 28 leaves over 28 functions (CG00, `ae17152b`) | **Yes.** All 22 return leaves and 6 argument-shape leaves are still present on this base, at the same lines (`scripts/rederive.py`). |
| K03: complex actual places are not substituted (`q_free(s->in.p)`, `q_free(s->slots[i])`, `q_make(&s->p)`) | **Yes, and they stay refused by design** (`W44-CALL-ARGUMENT-CUSTODY-POSITION`; CG11 for the pointer store). |
| K03 on the corpus: 15 leaves over 12 functions | **Yes**, all present. **Correction to the input:** 12 of the 15 are pointer arithmetic in an actual (`memcpy(key + prefix_len, …)`, `(char *)base + i * n`, `dst + (size - 3)`), which is CG16's, not a substitution gap. Two are a `sizeof` operand the lexical gate read as a protected name (`cmt_create`, `rd_kafka_conf_kv_split`). One is a pointer-arithmetic length (`rd_strndup(jwt, dot1 - jwt)`). |
| K12 argument-shape: "arguments do not match resource contract" is a contract shape gap | **Partly.** Three are a pointer cast of a formal at a `Released` formal: a substitution gap, now closed. Three are `rd_kafka_curr_msgs_get(rk, &tot_cnt, &tot_size)`, whose derived summary makes the out-pointer formals custody-taking: an address at a custody formal, which stays refused by design. |
| CG02: 3 statement consultations refused as `EffectfulActual` | **Yes** (CG02's measurement). The shape is admitted here. |
| CG00's upper bound: 17 functions freed alone | **Correction:** CG00 counted any corpus function as contracted. Callees defined in *another* unit (`rd_fnv1a`, `rd_murmur2`, `rd_crc32`, `rd_kafka_cgrp_assignment_is_lost`, `LZ4F_compressBound_internal`'s caller's builtins) have no contract in force in the caller's unit, so the return stays refused as an uncontracted call. That is cross-unit summary matching (K14, not carded). |
