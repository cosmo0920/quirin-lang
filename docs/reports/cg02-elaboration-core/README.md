# CG02-ELABORATION-CORE: the common expression elaborator and its order model

Card CG02 of the [Clang gap card queue](../../plans/clang-gap-cards/README.md)
(wave 48, round 2; base `e182d403`, local `main` = origin/main at dispatch;
main `f1ba8da2`, carrying W48-QR-SCALAR-TYPES-R3 and CG10-COMMON-TYPED-PLACE,
merged before finishing and every measurement retaken on it, §5.0). Gap
entries E03 (values), E04, E05, E08 and E12 (the order model); evidence Q11,
Q12, Q14, Q19, Q24, Q38, Q40. Routes **G** (the custody gateway) and **T**
(`typed_source.rs`). Rule: [gateway 10s](../../agents/gateway.md#constraint-10s).
The prediction was frozen before any route hook was committed
([PREDICTION.md](PREDICTION.md), commit `24e78301`).

## 1. The elaborator (`crates/qcore-c-gateway/src/elaboration.rs`)

`elaborate(node, &ctx) -> Result<Elaboration<F>, Refusal>` reads one Clang
full expression, or a one-declarator declaration, and yields:

- **`result: Operand`**: a value (pointer or scalar class) with its
  `ValueSource`. For an increment the source is `Old(place)` or `New(place)`, and
  never both, so `i++` is not `i = i + 1` as an expression. An assignment's
  value is `New(place)`; a call's is `Call(callee)`.
- **`graph: EvalGraph`**: `events` in one permitted serialisation (every edge
  points forward), `before` (explicit sequenced-before edges) and `unsequenced`
  (the operand groups C leaves unordered).
- **Events**: `Read(Place)`, `Write { place, pointer }`, `Call { callee }`, and
  the reserved `Branch`, `Merge` and `LifetimeStart`/`End`. The reserved events
  are the vocabulary for CG06, CG07 and CG14; no CG02 form emits them.
- **`facts`**: the route's facts for each path event (in G, the typed
  access-path reader's `StatementPlan`). **`actuals`**: each call's argument
  events. **`address_reads`**: the reads a path's own address performs.
- **`obligations`**: `Access(place)` for each path, and `Contract(callee)` for
  each call, applied once by the route. **`assumptions`**:
  `NoNumericClaim` wherever arithmetic appears, since values are
  custody-neutral and no numeric result is claimed (CG20).

**Sequencing.**

- The operands of an operator, an assignment's place and value, and a call's
  actuals are unsequenced.
- A side effect (a store) follows only its operands' *value computations*.
- A comma orders every left event before every right one. A literal right
  operand still orders the left one before a consumer.
- The sequence point before a call orders every event of its designator and
  actuals, side effects included.
- A compound assignment reads its place once, then writes it once after the
  right operand's value. The place read and the right operand are unsequenced.

**The order model** (`EvalGraph::check`). A graph is refused unless every pair
the transitive closure leaves unordered commutes:

- two reads commute;
- a read/write or write/write pair commutes only on disjoint places:
  - distinct named objects are disjoint;
  - a path inside an object overlaps only that object;
  - a path through a pointer is disjoint from an automatic local or formal whose
    address no `&` takes;
  - two paths are never proved disjoint (CG10 and CG13 own footprints).

The refusals are:

- `UnsequencedConflict`: one named object on both sides, which is C undefined
  behaviour;
- `NotDisjoint`: places not proved disjoint, refused *without* claiming
  undefined behaviour;
- `UnorderedCall`: any call unordered with another event (CG05).

No order is ever chosen left to right. A route lowers `events` in graph order
only because an admitted graph reaches the same state in every permitted order
(`QCore.ElaborationCore.interleave_run`).

**Named refusals.** Every form refuses through `FORM_TABLE` with its owning
card:

| Form | Card |
|---|---|
| value `&&`, value `\|\|`, `?:` | CG06 |
| nested call | CG05 |
| indirect call | CG22 |
| pointer store | CG11 |
| pointer arithmetic | CG16 |
| type query with an operand | CG08 |
| `_Generic`, statement expression | CG07 |
| compound literal | CG14 |
| volatile access | CG15 |
| atomic access | CG19 |

In addition:

- a cast kind outside `LValueToRValue`, `NoOp` and `IntegralCast` is CG08's;
- a place the reader refuses is CG10's;
- a call whose actuals carry effects is CG03's.

Anything else is refused as `unlisted form`.

### The interface CG03–CG07 extend

- **`Context`** (the route): `scalar`, `pointer`, `place(node, store) ->
  Facts` and `unaliased(decl)`. `place` is the narrow place interface. CG10's
  common typed place replaces the G implementation (`GContext`, over
  `typed_access_path::Reader::path`) without touching the elaborator.
  `path_place` computes the place identity (root declaration, `indirect`) from
  the AST.
- **CG03** reads `Obligation::Contract` and `Elaboration::actuals`. Today a
  call is lowered only as a sequenced piece handed to the existing classifier,
  and `EffectfulActual` marks the calls CG03 must apply.
- **CG04 and CG06** emit `Branch`/`Merge` in place of the `value &&`, `value
  ||` and `conditional` refusals, and plan conditions through the same
  `ConditionPlan` lowering that `elaborated_condition` uses.
- **CG05** replaces the `Commutation::Call` refusal with a footprint-based
  commutation, over the same unordered-pair check.
- **CG07** emits `LifetimeStart`/`End` for statement expressions, in place of
  the refusal.

## 2. Route G (`legacy_elaboration.rs`, thin hooks)

Every hook is a fallback, asked only after every older route of its position
refused.

- The hooks are:
  - `expression_statement`'s last step. This function is the statement arm,
    extracted unchanged.
  - A refused `return`. `return_statement` is the return arm, extracted
    unchanged, and `return_refused` excludes the cast-return refusal.
  - `planned_condition`'s last step (`planned_condition_routes` is the old
    body).
- A declined answer restores the site and element counters, so the refusal
  stays byte for byte.
- Each consultation writes a `position: "elaboration-*"` row to the
  `QCORE_TYPED_PATH_PARITY` stream. The row records the old refusal and the
  elaborator's answer.

**Statements.**

- The statement must hold one of this card's forms: a compound assignment, an
  increment, a comma, or an assignment *used as a value*. A statement's own
  top-level `=` is a plain store, and its refusal is the older routes' or
  another card's. An earlier run without this gate admitted a `sizeof`
  statement and moved one function (see §5).
- It is matched to a single outermost Clang node by line and exact tokens. The
  whole graph must pass the order model.
- It is then lowered in sequenced pieces:
  - each comma operand, and the inner assignment of `A = (B = E)` with `A` and
    `B` distinct objects, is a statement of its own;
  - the older lowering decides each piece first, so a call keeps its one
    contract application;
  - a piece the older lowering refuses is lowered from its own events.
- **Events.**
  - A scalar local write is `ForgetScalar`. The older route's own scalar check
    applies, and a protected formal is refused.
  - A path read or write is the typed reader's `IndirectScalarAccess`, with
    bases, chains and alias reads, and a frame for `*`/`[]` writes, exactly as
    `typed_path_statement` builds it. Its per-hop obligations are checked by
    `transfer`.
  - An object read, or an address read covered by its path's plan, lowers to
    nothing.
  - A call outside a piece, a pointer-valued store, a non-scalar or non-local
    write, a tracked descriptor, braces, an opaque callee and a list-iteration
    refusal all decline.

**Returns.**

- The leading comma operands are pieces. The last operand goes to the older
  return lowering.
- `return NAME = E` becomes `NAME = E; return NAME;`.
- Otherwise, a scalar value in a scalar-result function becomes its events and
  a `NoOp` exit, and escapes nothing.

**Conditions.** A refused call-free controlling expression that holds such a
form, with no connective and a scalar value, becomes a `ConditionPlan`: the
events in order, then one branch that refines nothing. A pointer-valued
condition keeps its refusal, because its null refinement belongs to the older
routes.

## 3. Route T

T gains no admission. CG01's census found no typed-source consumer that
decides a value form, so the importer names each one in its refusal:
`` typed-source value form `comma` is decided by no typed-source consumer (CG02) ``.
`CG01-T-CONSUMER-ADMISSIONS` is amended to cover them. Monkey wrappers are
unaffected: `monkey-source.json` is byte-identical.

## 4. Cited claims

| Claim | Reproduced? |
|---|---|
| E03 (values): T's compound assignment only as a statement; G "has scalar compound assignments" | T: **yes**. G: CG00's correction stands (`*=`/`&=` and field operands were refused). Both are flipped here (§6). |
| E04: assignment as a value refused on both routes (Q11 Q12 Q19 Q38) | **Yes** (CG00 control, BEFORE). |
| E05: value-producing `++`/`--` refused (Q12 Q24) | **Yes.** |
| E08: comma refused in value positions (Q11 Q38 Q40) | **Yes.** |
| E12 (order): T rejects calls beside reads through `reject_unsequenced` (Q14) | **Yes** (`e12_t_refuses_unsequenced_calls_and_reads`, unchanged). |
| W43-RETURN-COMMA-MACRO: 67 leaves over 32 functions, nanopb's `PB_RETURN_ERROR`; W45 re-measured 69/34 | **67/32 reproduce** as the elaborator's comma-return consultations. The first refusal is the comma value's `IntegralToBoolean` (a `bool` function; CG08), before the `?:` (CG06) and pointer store (CG11) the ledger named. |

## 5. Prediction against outcome

### 5.0 On main `f1ba8da2` (after CG10), the published figures

BEFORE is retaken on the merged base: the release binary built at CG10's
integration tip `1ae1c38f` (code-identical to main `f1ba8da2`), run into
`private/cg02/before-cg10`, exit 0, reproduces CG10's AFTER byte for byte
(`resource-diagnostics.json` `b63c1b57…`; 527 / 10 / 4,297). AFTER is this
card's merged tip (`private/cg02/after-main`, exit 0):
[corpus-comparison-main.json](artifacts/corpus-comparison-main.json).

| Route | BEFORE | AFTER | Moves |
|---|---|---|---|
| corpus (`66910c10`) | 527 / 10 / 4,297 | **543 / 10 / 4,281** | the same **16** Unknown → Safe as at the card's base (§5.1); unsupported 33,416 → 32,728 (−688), unknown_alias +372, leaks 36 → 36; 359 Unknown functions change diagnostics; sidecars byte-identical |
| system (`046accf6`) | 31 / 2 / 201 | **32 / 2 / 200** | `ipow`, as at the base |
| tail-pause slice (`90978ac6`) | 24 / 1 / 244 / 1 Pending | **26 / 1 / 242 / 1 Pending** | `flb_input_rate_gate_collect_backpressure` as at the base, and **`in_tail_pre_run`**, a cascade that needs both cards: its callee `tail_signal_manager` (`tail_signal.h`) is admitted only when CG10 reads `ctx->ch_manager[1]` (a mixed path on the tracked formal) and CG02 reads `ctx->ch_writes++`; its validated summary removes the caller's `call to uncontracted function` leaf and the alias leaf with it. CG10 alone moves no slice status (`cg10-after1`). |

System and slice BEFORE are the archived baselines
(`w47-flb-lib-strict-pairing/artifacts/after/{system,slice}`), which CG10's
report shows unmoved by CG10. [system-comparison.json](artifacts/system-comparison.json),
[slice-comparison.json](artifacts/slice-comparison.json). No correction, no
Safe → non-Safe move and no added leak or violation on any route.

### 5.1 At the card's base `e182d403`

Corpus arbiter: BEFORE is `private/cg02/before`, AFTER is `private/cg02/after3`,
both exit 0. Identity and comparison: [corpus-comparison.json](artifacts/corpus-comparison.json).

| Prediction | Outcome |
|---|---|
| BEFORE reproduces the baseline | **Held.** `resource-diagnostics.json` `ae17152b05516be7…`, `…chunk-contracts.json` `cbbcfd9c…`, `monkey-source.json` `57acfc58…`; 526 / 10 / 4,298. |
| 1. No Safe → non-Safe | **Held.** No correction. |
| 2. At most 7 of CG00's twelve → Safe | **Held: all 7.** `cio_enable_file_trimming`, `cio_disable_file_trimming`, `LZ4HC_literalsPrice`, `rd_hdr_histogram_record`, `rd_kafka_coord_req_keep`, `rd_kafka_transport_poll_set`, `rd_kafka_transport_poll_clear`. The five predicted Unknown stay Unknown for the stated reasons. `mk_esc_filename`'s `esclen-- > 0` is admitted, and its alias leaves remain. |
| 2'. No further moves (partner leaves) | **Missed: +9 more** Unknown → Safe, attributed below. |
| 3. No Unknown → Unsafe | **Held.** Leaks 36 → 36. |
| 4. Unknown functions lose leaves | **Held.** Unsupported leaves 35,737 → 35,112 (−625); 320 Unknown functions change diagnostics. |
| 5. Sidecars byte-identical | **Held.** `monkey-source.json`, `…chunk-contracts.json`, `…contracts.json` and `…source-effects.json` are identical to BEFORE. |
| 6. `W43-RETURN-COMMA-MACRO` stays refused by name | **Held.** Re-registered (§8). |

**Corpus: 526 / 10 / 4,298 → 542 / 10 / 4,282, +16 Safe.** The nine moves
beyond the prediction, each attributed by reading:

- **`cJSON_SetNumberHelper`.** `return object->valuedouble = number;` is a
  path write, then a `NoOp` exit (E04, return position).
- **`parse_hex4`.** `h += (unsigned int) input[i] - '0'` and its two siblings:
  a compound whose right operand reads `input[i]` (E03).
- **`rd_kafkap_leader_discovery_tmpabuf_add_alloc_topics`.** The macro
  `rd_tmpabuf_add_alloc` expands to `(tbuf)->size += RD_ROUNDUP(sizeof(T) *
  n, 8) * 1`: a field compound whose value holds a type query of a named type,
  read as a custody-neutral scalar as the W46 reader reads it (E03).
- **`XXH32_round`, `XXH64_round`, `XXH32_avalanche`, `XXH64_avalanche`.**
  `seed *= PRIME32_1`, `h32 ^= h32 >> 15` and the like. These are compounds on
  scalar formals that the older classifier misread as the pointer declaration
  `h32 * = …`, hence its `local declaration may shadow a formal` refusal. The AST
  reads the compound (E03). The formal-shadow check stays for a real
  declaration.
- **`LZ4HC_sequencePrice`** and **`XXH64_mergeRound`.** Validated-summary
  cascades: `LZ4HC_literalsPrice` and `XXH64_round` are newly Safe. The first
  also has a direct compound of its own.

An earlier AFTER run (`after1`, without the statement gate) also moved
`rd_kafka_AuthorizedOperations_copy` through a plain `copy_bytes = sizeof(T) *
n` store. The gate withdrew that move before the final run, since the type query
is CG08's, and it is not in the result.

**Leaf deltas.**

- **+304 `unknown_alias`.** Each is an admitted typed access whose base the
  flow cannot resolve. It is the access obligation still refusing, in an
  Unknown function.
- **+40 nested-call refusals in `rdxxhash.c`.** Callers of the newly Safe
  `XXH*_round` now reach their `XXH_readLE*` nested calls.
- **Added leaks or violations: none.**

**System route** (`046accf6`, `system-selection.json`) against the archived
baseline `w47-flb-lib-strict-pairing/artifacts/after/system`:

- 31 / 2 / 201 → **32 / 2 / 200**.
- `flb_scheduler.c::ipow` moves Unknown → Safe: `result *= base` and `base *=
  base` were refused as the same lexical shadow misreading (E03).
- [system-comparison.json](artifacts/system-comparison.json).

**Tail-pause slice** (`90978ac6`) against the archived baseline:

- 24 / 1 / 244 / 1 Pending → **25 / 1 / 243 / 1 Pending**.
- `flb_input.c::flb_input_rate_gate_collect_backpressure` moves Unknown → Safe.
  `ins->rate_gate_busy_chunks++`, a write through the formal's object and not a
  write of the formal, was refused as `unmodeled expression may mutate a formal
  resource identity`. The elaborator admits it; the parity row is
  `elaboration-statement`, `old_refusal`. The row's three `ins` alias leaves go
  with it.
- CG10's own base run (`cg10-before`) reproduces the archived slice with no
  move.
- [slice-comparison.json](artifacts/slice-comparison.json).

System and slice runs at the base: `private/cg02/after3-system/` and
`…/private/w47-tail/attempt-1/cg02-after3/`, both exit 0.

**Elaborator consultations** at the final tip on main `f1ba8da2`
([parity-census.json](artifacts/parity-census.json), deduplicated per
function, line and text; with CG10 more statements reach the typed reader, so
fewer are refused as CG10's):

| Position | Consultations | Admitted | Functions with an admission |
|---|---:|---:|---:|
| Statements | 33,331 | 662 | 335 |
| Conditions | 1,934 | 120 | 52 |
| Returns | 407 | 8 | 3 |

- **Most refusals are not this card's form**: 31,739 have no value form.
- **The rest are refused by name**:
  - CG10: 420 (the place, or two places not proved disjoint);
  - CG08: 165;
  - CG16: 151;
  - CG05: 98;
  - CG06: 72 (41 of them value `&&` in a condition);
  - CG22: 6;
  - CG03: 3.
- **Unsequenced pairs.** Eight refusals first recorded as undefined behaviour
  were all pairs on two paths (`p->header_min = p->header_max = …`,
  `rkbuf->rkbuf_corrid = ++rkb->rkb_corrid`). They are now `NotDisjoint`,
  refused without that claim (commit `e655b5fc`). No same-object unsequenced
  pair occurs in the corpus consultations.

## 6. §8 axes and the flipped controls

**Flipped controls** (`tests/clang_gap_phase1.rs`, each marked "Flipped by
CG02"):

- **`e03_g_scalar_compound_assignments_are_partial`.**
  - `x *= a` and `x &= a` flip from `DESTINATION` to Safe.
  - All ten `x op= s->a` flip from `IND` to Safe.
  - `s->a += 1` flips from `DESTINATION` to Safe.
  - A released base keeps the access refused.
- **`e04_assignment_as_a_value_is_refused_on_both_routes`.**
  - T: named.
  - G: `p = (q = q_alloc())`, `return (p = q_alloc())` and the scalar chain
    beside a live resource are Safe.
  - A leaking chain is not Safe.
  - `s->a = (s->a = 1)` stays refused.
- **`e05_value_producing_increments_are_refused`.**
  - T: named.
  - G: `int j = i++;` and `int j = s->a++;` are Safe.
  - `cg_sink(i++)` keeps `INCREMENT` (CG03).
  - A pointer step inside a larger value is not Safe.
- **`e08_comma_is_refused_in_value_positions`.**
  - T: named.
  - G: all three forms are Safe, with the calls in C's order.
  - A use after release in the right operand, and a returned object released
    by the left operand, are not Safe.
- **New: `e12_unsequenced_same_place_pairs_stay_refused`.** `i = i++`,
  `a[i] = i++` and `f(x++, x)` keep G's refusal, and the disjoint pair is
  admitted.
- **CG01's `tests/unit/source_operators.rs`.** Two rows now name the value
  form. They were `… CompoundAssignOperator` and `… UnaryOperator`.

**Axes covered** (admitted), from `tests/cg02_elaboration.rs`:

| Axis | Covered |
|---|---|
| Position | statement, declaration, return (value, assignment, comma), condition (`while`, `for`, `if`, `do`) |
| Place | local, formal, field, embedded field, array element, a path inside another path's address |
| Evaluation | unsequenced disjoint pairs; comma and call sequence points |
| Effect | none/Borrowed; Owned through pieces (`p = q = q_alloc()`), with a double release caught |

**Stays refused.**

- Evaluation: a same-object pair, two paths, a call beside another event, and
  value `&&`/`?:`.
- Effect: pointer-valued stores, and calls whose actuals carry effects.
- Type and qualifiers: volatile, atomic, and pointer arithmetic.
- For-loop clauses, which are not connected (`CG02-FOR-CLAUSE-SEQUENCES`).

## 7. Evidence

**Rust.**

- `tests/unit/elaboration.rs`, 10 tests:
  - the model's refusals of six unsequenced same-object forms;
  - the two-path `NotDisjoint`;
  - CG05 calls;
  - the disjoint admissions, including an address-taken local against a path;
  - pre/post value separation;
  - the compound's single place evaluation;
  - comma ordering;
  - the named refusals;
  - T's naming;
  - the Lean binding.
- `tests/cg02_elaboration.rs`: 4 tests of G positions, obligations, refusals
  and pieces.
- `tests/clang_gap_phase1.rs`: the five flipped or added controls in §6.

**Lean.** `QCore.ElaborationCore` is registered in the new
`AuditElaborationCore.lean`, with **29 theorems**, no `sorry` and no project
axiom.

- *Forms*:
  - `forms_unique`;
  - `unlisted_refused`, `listed_refusal_named`;
  - `value_connectives_refused`, `out_of_scope_refused`,
    `value_forms_elaborated`.
- *Commutation*:
  - `commutes_symm`, `commute_vectors_hold`;
  - `write_read_conflict`, `write_write_conflict`, `call_never_commutes`.
- *Graph semantics and order independence*:
  - `run_append`;
  - `commutes_sound`: commuting events commute on every store;
  - `step_through`;
  - **`interleave_run`**: every interleaving of two operands whose events
    pairwise commute (every serialisation C permits) reaches the same store;
  - **`read_stable`**: a read unordered only with events that do not write its
    place observes the same value;
  - `write_order_matters`: the refusal is necessary;
  - **`bin_admitted_order_independent`**: an admitted unsequenced pair is
    order-independent for any concretisation.
- *Order vectors*: `order_vectors_hold` and **`unsequenced_refusals`** (`i =
  i++`, `a[i] = i++`, `f(x++, x)`).
- *Value forms*:
  - **`comma_sequencing`**, `comma_events`;
  - `post_value_old`, `pre_value_new`, **`pre_post_separated`**,
    `post_increment_not_assignment`;
  - **`compound_single_evaluation`**, `compound_desugars`;
  - `assignment_value`.

**Binding.** `cg02_lean_vectors_match_the_rust_tables` reads the module's
`formTable`, `commuteVectors` and `orderVectors`. It compares the first two with
`FORM_TABLE` and `commutes`, and runs each of the ten `orderVectors` C sources
through Clang and `elaborate`, comparing the admission.

**Premises**, stated and not proved:

- The Clang-JSON-to-graph reading (trust boundary 28).
- `IndirectScalarAccess` and `ForgetScalar` as the custody meaning of a scalar
  access and write. These are the W43 and W46 rules.
- The model's places are named objects. Rust's path rules are *stricter* than
  the model's: two paths never commute.

## 8. Ledger (622 → 623 on main `f1ba8da2`; 621 → 622 at the card's base)

- **Retired**:
  - `CG02-ELABORATION-CORE`. Every DISCHARGE clause holds: BEFORE reproduces,
    the prediction is frozen, AFTER is attributed, absorbed follow-ups are
    re-registered, refusal controls exist, and the Lean is audited.
  - `W43-RETURN-COMMA-MACRO`, absorbed and re-measured at 67/32.
- **Registered**:
  - `CG02-COMMA-OPERAND-RESIDUALS`: the 67 comma returns (CG08, then CG06 and
    CG11).
  - `CG02-FOR-CLAUSE-SEQUENCES`: 109 leaves over 71 functions, measured
    lexically (`scripts/for_clause_census.py`).
  - `CG02-OLDER-SCALAR-ROUTE-UNSEQUENCED`: the older scalar route admits
    `x = (x = 1)`. It is custody-neutral, with 0 in the corpus by
    `scripts/same_object_census.py`.
- **Amended** with the elaborator's measured named refusals:
  - `CG01-T-CONSUMER-ADMISSIONS`: the value forms;
  - `CG03`: 3 effectful actuals;
  - `CG04`: condition connectives with increments;
  - `CG05`: 97;
  - `CG06`: 79;
  - `CG08`: 166.

Script: [scripts/update_ledger.py](scripts/update_ledger.py).

## 9. Gate

At tip `a6b3e43e` (main `f1ba8da2` merged), [scripts/gate.sh](scripts/gate.sh);
exits in [gate/](gate/), record [gate.json](gate.json), logs under
`private/cg02/gate2/`.

| Step | Command | Exit | Count |
|---|---|---|---|
| submodule | `git submodule update --init --recursive` | 0 | |
| tests | `python3 scripts/run_test_binaries.py` | 0 | 370 units, 383 summaries; **6,605 passed**, 0 failed, 11 ignored (`private/test-binaries/20260929T135327Z-2683079/`) |
| fmt | `cargo fmt --all -- --check` | 0 | |
| clippy | `cargo clippy --all-targets -- -D warnings` | 0 | the first run (101) flagged two single-element loops in re-pinned tests, fixed in `a6b3e43e`; tests and fmt re-run after it |
| probe | `cargo run -p qcore-compiler --example reevaluation_probe` | 0 | |
| follow-up probe | `cargo run -p qcore-compiler --example reevaluation_followup` | 0 | |
| Lean | `python3 scripts/check_lean_audits.py` | 0 | **230 audits** (229 + `AuditElaborationCore.lean`) |
| Python | `python3 -m unittest discover -s scripts/tests` | 0 | 365 tests OK, after the matrix validation was regenerated from `gate.json` (before it: 1, the derived `lean_audits`) |
| docs index | `python3 scripts/build_docs_index.py` | 0 | |

A pre-merge gate at the card's base tip (`private/cg02/gate/`) ran 6,571
passed and 5 failed: four older pins the elaborator moves, re-pinned
deliberately in `b962cc1b` (`indirect_scalar_access`'s `(*n)++`, two
`scalar_member_store` rows, `legacy_c`'s increment positions,
`return_position_modeled_call`'s comma return), each with its reason in the
test. `providers/` is unchanged, so the provider gate is not due.

## 10. Scripts

- [compare.py](scripts/compare.py): per-function moves and leaf deltas.
- [parity_census.py](scripts/parity_census.py): the elaborator's
  consultations.
- [measure.sh](scripts/measure.sh): the corpus, system and slice runs.
- [gate.sh](scripts/gate.sh): the standing gate.
- [for_clause_census.py](scripts/for_clause_census.py) and
  [same_object_census.py](scripts/same_object_census.py): residual sizes.
- [update_ledger.py](scripts/update_ledger.py): the ledger edits.
