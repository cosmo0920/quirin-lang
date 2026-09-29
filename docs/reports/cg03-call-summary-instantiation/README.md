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
| K03 on the corpus: 15 leaves over 12 functions | **Yes**, all present. **Correction to the input:** 12 of the 15 are pointer arithmetic in an actual, which is CG16's, not a substitution gap. Eleven are offset actuals (`memcpy(key + prefix_len, …)`, `(char *)base + i * n`, `dst + (size - 3)`); one is a pointer difference (`rd_strndup(jwt, dot1 - jwt)`). The other three are a `sizeof` operand the lexical gate read as a protected name (`cmt_create`, `rd_kafka_conf_kv_split`, `rd_kafka_ApiVersions_copy`). |
| K12 argument-shape: "arguments do not match resource contract" is a contract shape gap | **Partly.** Three are a pointer cast of a formal at a `Released` formal: a substitution gap, now closed. Three are `rd_kafka_curr_msgs_get(rk, &tot_cnt, &tot_size)`, whose derived summary makes the out-pointer formals custody-taking: an address at a custody formal, which stays refused by design. |
| CG02: 3 statement consultations refused as `EffectfulActual` | **Yes.** The shape `cg_sink(i++)` is now admitted (fixtures). All three corpus sites stay refused because their callee has no contract in force in the unit: `rd_list_set_int32` is defined in another unit, and `append_attribute_to_data_point`'s own row is Unknown. |
| CG00's upper bound: 17 functions freed alone | **Correction:** CG00 counted any corpus function as "contracted". Many callees have no contract in force in the caller's unit, so the return stays refused as an uncontracted call. They fall into three groups: a local body whose own row is Unknown (`rd_kafka_broker_ApiVersion_supported0`, `LZ4F_compressBound_internal`); a body in another unit (`rd_fnv1a`, `rd_murmur2`, `rd_crc32`, `rd_kafka_cgrp_assignment_is_lost`, `rd_kafka_Uuid_hash`: cross-unit summaries are K14, not carded); and compiler builtins (`__builtin_clz`). The call-and-return shape itself is admitted. |

## 3. Measurements

### 3.1 On the merged base (main `cb20c28e`): the published figures

CG11-POINTER-VALUED-STORES landed while the card ran, so main was merged
(`3308bb44`) and every measurement was retaken on it.

**BEFORE.**

- Binary: the release `quirin-audit` built at `cb20c28e`
  (`private/cg03/quirin-audit-main`, `412f5cb5…`).
- Runs: `private/cg03/before-main{,-system}` and the shared slice attempt's
  `cg03-before-main`, all exit 0.
- Identity: it reproduces CG11's AFTER byte for byte
  (`resource-diagnostics.json` `a6dc500b…`; 548 / 10 / 4,276). The system
  is `c6fab2ce…` (33 / 2 / 199) and the slice `e9c0d776…`
  (26 / 1 / 242 / 1 Pending).

**AFTER.**

- Binary: the tip's release binary (`c1d28456…`, built at `5aabb0f2`,
  code-identical to the final tip).
- Runs: `private/cg03/after-main*`, all exit 0.
- Comparisons: [corpus-comparison.json](artifacts/corpus-comparison.json),
  [corpus-changed.json](artifacts/corpus-changed.json),
  [system-comparison.json](artifacts/system-comparison.json),
  [slice-comparison.json](artifacts/slice-comparison.json).

| Route | BEFORE | AFTER | Moves |
|---|---|---|---|
| corpus (`66910c10`) | 548 / 10 / 4,276 | **558 / 10 / 4,266** (`f5e15b80…`) | **+10 Unknown → Safe**; 66 more Unknown functions change diagnostics; sidecars byte-identical |
| system (`046accf6`) | 33 / 2 / 199 | **33 / 2 / 199** (`610badca…`) | no status move; 11 functions: −12 branch-condition leaves, +10 `unknown_alias` |
| tail-pause slice (`90978ac6`) | 26 / 1 / 242 / 1 Pending | **26 / 1 / 242 / 1 Pending** (`c6039e1d…`) | no status move; 34 functions: −4 branch-condition leaves, +137 `unknown_alias` |

**The ten moves, each attributed by reading.**

- **`rd_hdr_nextNonEquivalentValue`, `rd_hdr_medianEquivalentValue`.** Return
  operand: two local identity calls in one leaf (`lowest(h, v) + size(h, v)`).
- **`rd_hdr_highestEquivalentValue`.** Return operand
  (`next(h, v) - 1`). It is a cascade: its callee is newly Safe with a
  validated summary.
- **`rd_kafka_cgrp_assignment_is_lost`.** Return operand
  `rd_atomic32_get(&rkcg->rkcg_assignment_lost) != 0`. The typed sub-object
  address is now part of the one application (W44 reads).
- **`rd_kafka_topic_result_free`, `rd_strtup_free`,
  `rd_kafka_metadata_destroy`.** Typed substitution: `destroy((T *)ptr)` /
  `rd_free((void *)metadata)`, a pointer cast of a formal at a `Released`
  formal, bound to the formal's identity.
- **`ctr_id_cmp`, `rd_kafka_consumer_group_state_code`,
  `rd_kafka_consumer_group_type_code`.** A condition leaf whose call reads a
  member or array element (`memcmp(cid1->buf, …)`,
  `rd_strcasecmp(names[i], name)`). The statement position admitted these
  through W44; the condition consumer now asks the same application point.

**Leaf deltas** (corpus):

- Removed:
  - −67 branch-condition leaves, −6 for-loop condition, −1 loop condition;
  - −7 return;
  - −2 K03;
  - −3 argument-shape;
  - −1 `call to uncontracted function rd_hdr_highestEquivalentValue` (the
    cascade).
- Added:
  - +18 `unknown_alias`: the newly admitted reads' own obligations failing
    at the flow, on a base the flow cannot resolve;
  - +1 `nullable resource out used by a non-null contract`
    (`rd_kafka_conf_kv_split`). The typed substitution read
    `rd_malloc((sizeof(*out) * n) + lens)`, and the next statement uses the
    nullable result without a test. It is an Unknown-class statement true
    under the declared contract.
- **Added leaks or violations: none.** No Safe → non-Safe move, no Unknown →
  Unsafe move, **no correction**.

In the system and the slice, the `unknown_alias` rise is the same exchange.
Fluent Bit's `flb_plg_*` log macros expand to two conditions on one line.
The first, `flb_log_check_level(ctx->ins->log_level, …)`, is now admitted
with its read obligation (the unresolved `ctx` of the plugin's
`in_context`). The second stays refused, and its leaf is identical, so the
line keeps one refusal leaf. No status moves.

**Parity** ([parity-census.json](artifacts/parity-census.json),
[position rows before](artifacts/position-rows-before.txt) /
[after](artifacts/position-rows-after.txt)), de-duplicated:

- **Return operand:** 81 consultations, 10 admitted. The refused rows are
  `CG03-RETURN-OPERAND-RESIDUALS`.
- **Typed substitution:** 22 consultations, 5 admitted. The refused rows are
  `CG03-TYPED-SUBSTITUTION-RESIDUALS`.
- **Effectful actual:** the 11 elaborator rows are unchanged. The 3 CG03 rows
  keep an uncontracted callee.
- **Older admissions the acceptance declines:** 126 rows, all
  `recursive-local` except 3 `returned-alias`
  (`CG03-RECURSIVE-COMPONENT-ADMISSIONS`). The condition consumer shows no
  disagreement, by construction.

### 3.2 At the card's base `f19204fa`

Measured before the merge, for identity:

- **BEFORE** (`private/cg03/before`) reproduces CG02's archived AFTER byte
  for byte (`963d7c9f…`, 543 / 10 / 4,281). System: 32 / 2 / 200. Slice:
  26 / 1 / 242 / 1 Pending.
- **AFTER** at `c4621b56` (`after2`, `ace310c4…`) is **553 / 10 / 4,271**,
  with the same ten moves and the same leaf deltas as §3.1. System and slice
  do not move.
- An earlier AFTER at `a20f85ef` (`after1`, 549 / 10 / 4,275) did not yet
  carry the W44 argument reads in the application point. Its six moves are a
  subset of the ten.
- The last change (effectful pieces with a stored result and scalar path
  actuals) moved no function. The merged AFTER shows the same ten moves.

## 4. Prediction against outcome

| Prediction ([PREDICTION.md](PREDICTION.md)) | Outcome |
|---|---|
| BEFORE reproduces the baseline | **Held** at the base and on the merged base. |
| At most 11 Safe of CG00's 13 strict | **Partly held: 7 of the 11.** Held: `rd_hdr_*` ×3, `rd_kafka_cgrp_assignment_is_lost`, and the three cast substitutions. Missed: `LZ4HC_hashPtr`, whose call is admitted but whose macro residual `((0) * 2654435761U) >> ((4 * 8) - 15)` is not one the condition leaf decides (`ResidualRefused`). Also missed: `rd_kafka_broker_ApiVersion_at_least` and `…_no_lock` (callee `…_supported0` has no validated summary) and `cmt_metric_has_start_timestamp`, where `cmt_atomic_load`'s actual is a member address the W44 reads refuse (the classifier's indirect-access refusal). |
| `rd_kafka_msg_partitioner_fnv1a`/`murmur2` stay Unknown (amended) | **Held:** their callees are uncontracted in the unit. |
| The four alias-held functions stay Unknown | **Held.** |
| K03 pointer-arithmetic functions stay Unknown | **Held** (14 consultations, CG16). |
| `cmt_create`, `rd_kafka_conf_kv_split`, `rd_kafka_ApiVersions_copy` at most move | **Partly.** The first two lose their K03 leaf and keep other leaves (Unknown). `rd_kafka_ApiVersions_copy` keeps its leaf: `*dstp = rd_memdup(…)` is not the statement shape the substitution reads. |
| `rd_kafka_curr_msgs_get` callers | **Stay Unknown:** an address at a custody formal, by design. |
| Beyond CG00's list: further moves possible | **3 more:** `ctr_id_cmp` and the two `consumer_group_*_code`, from aligning the condition consumer with the W44 argument reads. |
| 1. No Safe → non-Safe | **Held.** |
| 2. Added leaks/violations classified | **None added.** The one new Unknown-class `nullable resource` row is read above. |
| 3. Sidecars byte-identical | **Held.** |
| 4. W44 custody refusals unchanged | **Held.** |
| 5. No leaf moves in the condition desugaring | **Missed, deliberately.** The frozen prediction assumed the condition consumer's admissions would be byte-identical. The card then made the W44 argument reads part of the one application (K12: `r_borrow_code(&s->a)` was admitted as a statement and refused in an `if`). So 74 condition leaves are now decided through that route, 3 of them moving functions to Safe, each attributed. Every older condition admission is reproduced exactly: the refusal mapping is unchanged, and the new admissions are only calls the gate refused before. |

## 5. Tests and flips

The new controls are in `crates/qcore-c-gateway/tests/cg03_call_instantiation.rs`
(7 tests). Each admission is paired with its fail-closed controls. The
BEFORE claims are executed: with each route disabled, the admission assertion
fails (checked on the fixture, `private/cg03/tmp/t-disabled.log`,
`re4.log`).

The unit tests are in `tests/unit/call_instantiation.rs`:

- the acceptance table;
- the condition-gate equality over all rows;
- every substitution vector read by Clang;
- arity and one-refused-formal;
- the two Lean vector bindings.

**Flipped or re-pinned deliberately:**

- `clang_gap_phase1::e05_value_producing_increments_are_refused`:
  `cg_sink(i++)` flips from `INCREMENT` to Safe (CG00/CG02's K12 control).
- `return_position_modeled_call::a_call_that_is_not_the_whole_return_expression_is_refused`:
  `return r_release_code(p) & 1;` is one condition leaf. It is now decided
  exactly as the same leaf in an `if`, asserted by an equal-outcome
  comparison. B01 had refused it only lexically.

**Kept by adding a premise, not re-pinned:**

- `return_position_modeled_call::the_scalar_call_path_keeps_the_synthetic_place_collision_guard`:
  the reserved-place guard.
- `neutral_expression_positions::an_address_of_actual_stays_unknown`: rule
  5j's no-unary-`&` return premise.

CG00's `k12_one_contract_is_accepted_in_one_position_and_refused_in_another`
and `k03_complex_actual_places_are_not_substituted` do not flip. Their
refused shapes are an owned result tested (CG04), a non-identity nested call
(CG05) and custody-position places (by design).

**§8 axes.**

- **Position.** Statement, condition leaf, return operand and elaborated
  piece now share one application point. Nested stays B04's.
- **Effect.** A sequenced effectful actual is admitted; an unsequenced one
  refuses.
- **Evaluation order.** No order C leaves unspecified is chosen: two calls of
  one leaf only as identities, and the reads of an identity commute.
- **Place.** A CG10 place is only observed at `Borrowed`.
- **Type.** Pointer bit casts between object pointers only.
- **Boundary.** Declared contracts, and validated local summaries outside the
  caller's recursive component.
- **Verdict.** No Safe → non-Safe.

## 6. Lean

`QCore.CallInstantiation`, `proof/AuditCallInstantiation.lean`, **26
theorems**, no placeholders and no project axioms. `check_lean_audits.py`
passes 232 audits (231 on main + this one).

- **Acceptance.**
  - `acceptance_vectors_hold` (bound to the Rust table);
  - `acceptance_is_position_independent`;
  - `the_return_operand_takes_what_the_condition_leaf_takes`: the tested
    acceptance *is* rule 5h's gate;
  - `every_b01_admission_stands`;
  - `the_widening_is_a_validated_callee_or_a_derived_result`;
  - `a_recursive_callee_is_refused_in_every_use`;
  - `a_custody_result_is_refused_where_it_is_tested`;
  - `a_passed_value_is_the_identity`.
- **Instantiation.**
  - `instantiation_is_one_summary_and_one_substitution`: the contract in
    force, and per formal that formal's rule on its own actual;
  - `instantiation_is_deterministic`.
- **Total or reject.**
  - `an_admitted_substitution_refuses_no_formal`;
  - `one_refused_formal_rejects_the_call`;
  - `a_custody_formal_takes_only_an_identity`;
  - `an_unlisted_actual_is_refused`.
- **Substitution soundness.**
  - `an_identity_substitution_is_the_objects_value`, through value-preserving
    casts (the stated premise);
  - `an_address_substitution_is_the_slot`;
  - `data_evaluates_no_pointer_object`;
  - `a_place_cg10_refuses_stays_refused`, over `TypedPlace.readPlace`;
  - `a_place_is_only_observed`.
- **Lowering.**
  - `each_returned_call_is_applied_once`;
  - `the_return_operand_escapes_nothing`;
  - `the_actual_events_precede_the_call`;
  - `an_effectful_actual_is_scalar_data`.
- **Vectors.** `substitution_vectors_hold` (bound to the Rust table).

What is premise and what is model: faithful Clang extraction, the
token-to-AST match, and C11 6.3.2.3p7 for a bit cast are stated, not proved.
The contract's clauses are applied by the unchanged `apply_contract_call`,
whose models are the older ones.

## 7. Follow-ups

- **Retired:** `CG03-CALL-SUMMARY-INSTANTIATION`. Every DISCHARGE clause
  holds (BEFORE reproduced, prediction frozen, AFTER attributed, absorbed
  follow-up re-sized, Rust controls and audited Lean).
- **Absorbed, resized:** `W46-ACCESS-PATH-PARITY`.
  - Its plan-vocabulary part for call positions is answered by one
    application point and one acceptance/substitution vocabulary.
  - Re-measured: condition-access 1,303 / 196, return-family 266 / 141,
    call-argument 230 / 491 (211 / 455 before). The identical-plan figures
    are carried.
  - The access-plan vocabularies stay distinct, and no route is retired.
- **Registered:**
  - `CG03-RETURN-OPERAND-RESIDUALS`: 71 consultations over 54 functions;
  - `CG03-RECURSIVE-COMPONENT-ADMISSIONS`: statement 61 functions,
    whole-call return 16, derived returns 3;
  - `CG03-TYPED-SUBSTITUTION-RESIDUALS`: 17 over 12.
- **Amended:** CG04 and CG05, with the API they call.
- Ledger 627 → 629. Script: [scripts/update_ledger.py](scripts/update_ledger.py).

## 8. Scripts

| Script | What it does |
|---|---|
| [measure.sh](scripts/measure.sh) | Runs the corpus, system and slice (`SKIP_BUILD=1` measures the binary in `target/release`). |
| [compare.py](scripts/compare.py) | Status moves and leaf deltas (copied from CG02). |
| [changed.py](scripts/changed.py) | Every function whose diagnostics changed. |
| [routes.py](scripts/routes.py) | The system and slice comparisons. |
| [rederive.py](scripts/rederive.py) | CG00's function list on a run. |
| [parity_census.py](scripts/parity_census.py) | The `call-instantiation` rows. |
| [position_rows.py](scripts/position_rows.py) | W47's position rows, tolerant of rows without its keys. |
| [update_ledger.py](scripts/update_ledger.py) | The ledger edits. |
| [gate.sh](scripts/gate.sh) | The standing gate. |
