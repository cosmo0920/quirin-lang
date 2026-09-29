# CG04-CONDITION-ACQUISITION: acquisitions in conditions and complex condition leaves

Card CG04 of the [Clang gap card queue](../../plans/clang-gap-cards/README.md)
(wave 48, round 4; base `27dc7c9a` = local and origin `main` at dispatch).

- **Gap entries:** E09 and E13, on route **G** (the custody gateway).
- **Evidence:** Q19, Q29, Q47 and example A of the analysis.
- **Rule:** [gateway 10x](../../agents/gateway.md#constraint-10x), amending 5h
  and the tested row of 10u.
- **Lean:** `QCore.ConditionAcquisition`, `proof/AuditConditionAcquisition.lean`
  (30 theorems); `QCore.CallInstantiation` gains `Use.testedSplit`.
- **Prediction:** [PREDICTION.md](PREDICTION.md), committed at `4a79e47d`,
  after the first implementation commit (`3325d36f`) and before any corpus run
  of an implementation build was read. The brief asked for it before
  implementing; this is the deviation.

## 1. What changed

### 1.1 The NULL split (E09, example A)

```c
void *p;
if ((p = malloc(n)) != NULL) { free(p); }
```

The condition desugaring (rule 5h) asked the one application point
(`Parser::instantiate_call`, rule 10u) for each leaf call with the use
`Tested`, which refused an owned, reallocated or descriptor result as
`custody-result`. This card adds the use **`ResultUse::TestedSplit`** (the
tested row widened by the split, never a consumer-specific gate): a
`(NAME = CALL(…))` group whose leaf's residual is the existing null test of
`NAME`, or for a descriptor its existing validity test. `accept` takes every
result class for it; a recursive-component callee still refuses.

The leaf then lowers to exactly the instructions of the statement pair
`NAME = CALL(…); if (NAME …)`, inside the condition's plan:

1. the fresh identity from the contract: the classifier's `ContractCall`, its
   produced site;
2. its binding to the destination: the call's destination `NAME`;
3. the split: `BranchOnNull` on `NAME`, or `BranchOnValidity` against the
   descriptor sentinel;
4. success and failure states on each edge: the flow's refinement keeps the
   obligation on the success edge; a reallocation's failure edge keeps the old
   identity live (the contract's failure arm);
5. the merge: the plan's edges join the successors as the pair's do.

So the condition is decided exactly as the statement pair is
(`example_a_decides_as_the_statement_pair` compares nine pairs diagnostic by
diagnostic). The call is never hoisted: it is one node of the condition's
plan, at the condition's line. An acquisition in the right operand of `&&`
is applied only on the edges C evaluates it on (rule 5h's edges).

Premises, each a refusal when it fails:

- **Destination.** `NAME` is one automatic, non-formal local the lexical or
  the Clang reader types (rule 10l's declared local). This admits `void *` and
  typedef'd pointer locals, which the lexical reader alone did not. A
  descriptor name is a destination only for its own integer-carried
  acquisition.
- **Carrier** (`carrier`). A custody result is split only on its own carrier's
  failure value: a pointer's NULL, a descriptor's sentinel.
- **Placement** (`placement`). No custody split on a node a CFG cycle may pass
  through: a loop position, a loop body, or a range a backward `goto` closes
  (`CycleRanges`: every node of a cycle lies between the target and source of
  one of its backward edges; an unbounded extent, an unknown label or `setjmp`
  make the whole body a cycle). See §3.
- An owned result that is discarded (`if (malloc(n))`), bound but not split
  (`(p = malloc(n)) == q`) or split on another carrier keeps its refusal.

### 1.2 Complex leaves (E13)

The desugaring refused the **whole** condition when a type operator or an
increment appeared anywhere, before it split the leaves. `leaf_route` decides
each leaf instead:

| Leaf | Decision |
|---|---|
| a call, no type-operand call, no increment, no nested connective | the calls route (unchanged) |
| a type operator whose parenthesised operand holds no call | residual text; the existing residual routes decide it |
| a call inside a type operand | refused (`call-in-type-operand`): C evaluates none of it |
| a type operator without a parenthesised operand (`sizeof *p`) | refused as before (its extent is not read) |
| an increment beside a call in one leaf | refused (`increment-beside-call`, CG05) |
| a connective below a leaf holding a call | refused (`connective-beside-call`, CG06) |
| call-free with an increment | the elaborator (rule 10s): its events, then a branch that refines nothing |
| call-free, every older route refused | the elaborator, asked last |

A call-free condition still keeps a plan only when a typed or elaborated
operand was needed (rule 10k's premise). Value `&&`, `||` and `?:` outside
condition position stay CG06's.

### 1.3 Code, and the shared items added

- New: `crates/qcore-c-gateway/src/condition_acquisition.rs` (the decision
  tables), `src/legacy_condition_acquisition.rs` (route-G adapter:
  `leaf_features`, `holds_real_call`, `CycleRanges`, `acquisition_split`, the
  parity row), `tests/condition_acquisition.rs`,
  `tests/unit/condition_acquisition.rs`,
  `tests/unit/legacy_condition_acquisition.rs`,
  `tests/fixtures/contracts/cg04_condition.qv`, `proof/QCore/ConditionAcquisition.lean`,
  `proof/AuditConditionAcquisition.lean`.
- **Shared foundations, additive only:**
  - `call_instantiation.rs`: the variant `ResultUse::TestedSplit` (code
    `tested-split`), one new `accept` arm, five `ACCEPTANCE_VECTORS` rows
    appended; Lean `Use.testedSplit`, its `accept` arm and five
    `acceptanceVectors` rows. No existing variant changes meaning.
  - `legacy_elaboration.rs`: one new `pub(super) fn elaborated_leaf`.
  - `elaboration.rs` and `typed_place.rs`: untouched.
- Thin hooks: `legacy_c.rs` (module declaration, two `Parser` fields:
  `condition_widening`, `condition_cycles`), `lib.rs`.
- `legacy_condition_desugar.rs` (rule 5h's module): a `Refusal::LeafShape`
  variant; the two whole-condition refusals replaced by the per-leaf decision;
  the residual built before the instantiation so the split is known; the
  carrier and placement checks after it; `top_level_calls` skips a type
  operand; the declared-destination check reads `position_declared_local`.
  Every consultation writes a `condition-desugar` parity row.

## 2. Claims of the input, reproduced

| Claim | Outcome |
|---|---|
| E09: condition desugaring refuses Owned/Reallocated/descriptor results (Q19) | **reproduced** by CG00's control `e09_…` on this base (heap, nullable, loop, descriptor), flipped here for the branch forms |
| E09: example A is not judged Safe, and the call is not hoisted (Q47) | **reproduced**: `an_owned_result_assigned_in_a_condition_keeps_todays_exact_lowering` on the base; its no-hoist half stays a regression |
| E13: nested connectives, type operators, increments, expression callees are rejections in condition plans | **partly reproduced**, as CG00 found: increments, `sizeof` and expression callees refused; nested connectives already admitted |
| Brief: an out-parameter acquisition in a condition (`if (f(&p) == 0)`) stays refused | **correction:** rule 5h already admits it on the base when the call's result owns nothing, with the contract's unconditional `Out` on both edges (so the failure edge reports the leak that contract implies). This card adds no split for it (CG23); pinned in `unlisted_acquisition_shapes_keep_the_refusal` |
| CG00's E13 function list is freed by condition-leaf composition | **correction:** by reading the 28 remaining functions, most E13 leaves are calls whose actual is a place on a bound root (`strcmp(p->shortname, x)`: rule 10k's never-bound premise, CG10) or an address of a place (`cmt_opts_compare(&c->opts, o)`: CG11); the census puts that class at 1,768 leaves (§4) |

## 3. A soundness defect found: loop site generations

The flow keeps **one abstract site per acquisition node**. `create_resource`
re-inserts the site `LIVE` when a loop re-executes the acquisition, without
telling a live earlier generation apart, and a refuted nullable acquisition on
the failure edge removes the site. On this base, in the statement form an
older route already admits:

```c
void *p;
for (;;) { p = malloc(n); if (p == NULL) break; use(p); }   /* reported Safe */
```

Every earlier generation leaks, and no leak is reported. With another exit
(`while (flag) { … }`) the live generation reaches it and the leak is
reported. The condition form (`while ((p = malloc(n)) != NULL) use(p);`) is
the same pattern. So rule 10x refuses a custody split on any node a cycle may
pass through. The defect is pinned as observed
(`the_statement_form_loop_site_defect_is_pinned`) and registered as
**`CG04-LOOP-SITE-GENERATIONS`**. It is not fixed here: the fix changes the
flow's transfer for every statement form and needs its own corpus re-pin.
The guard refused 2 corpus consultations (`rd_gz_decompress`,
`rd_kafka_lz4_decompress`). An exploratory run without the guard (not a
record) admitted 7 splits and 2 more functions' conditions.

## 4. Measurements

Scripts: [measure.sh](scripts/measure.sh) (CG03's, renamed),
[compare.py](scripts/compare.py), [census.py](scripts/census.py),
[followups.py](scripts/followups.py), [unconsulted.py](scripts/unconsulted.py),
[rederive.py](scripts/rederive.py). Runs: `private/cg04/{before,after1}*`
and `private/w47-tail/attempt-1/cg04-{before,after1}`, every exit 0.
Binaries: BEFORE built at `27dc7c9a` (`cab96c8f…`); AFTER built at `6728d9dd`
(`f9eb4bf0…`); later commits change tests, records and comments only.

| Route | BEFORE | AFTER | Moves |
|---|---|---|---|
| corpus (`66910c10`) | 564 / 10 / 4,260 (`7393fd21…`, byte-identical to CG03's archived AFTER) | **565 / 10 / 4,259** (`24571074…`) | **+1 Unknown → Safe**; 13 more Unknown functions change diagnostics; no Safe → non-Safe; sidecars byte-identical |
| system (`046accf6`) | 33 / 2 / 199 (`4597b90c…`) | 33 / 2 / 199, byte-identical | none |
| tail-pause slice (`90978ac6`) | 26 / 1 / 242 / 1 Pending (`a6c01e91…`) | the same, byte-identical | none |

The system and slice reports embed the measuring worktree's profile paths, so
their bytes differ across worktrees: the slice BEFORE equals CG03's archived
`cg03-after-main2` run field for field except those paths and the contract
digests derived from them; the system totals equal CG03's.

**The move.** `rdkafka_mock.c::rd_kafka_mock_cluster_op_io`:
`while (rd_socket_read(fd, buf, sizeof(buf)) > 0) ;`. The `sizeof` sits in a
call's actual; the whole-condition type-operator refusal is lifted, and the
leaf is the call (`read`, borrowed actuals) and a neutral residual. It acquires
nothing, so `Safe` is the function's own reading.

**The 13 same-status functions** (all Unknown before and after;
[corpus-comparison.json](artifacts/corpus-comparison.json)):

- A split acquisition admitted, the function held by other leaves:
  `mk_file_to_buffer` (`fopen`), `mk_rconf_read` (`fopen`),
  `mk_utils_register_pid` (`open`, a descriptor), each losing its condition
  refusal and its `unknown_alias` leaves on the acquired name (−3, −10, −3).
- A split acquisition admitted, with a new fail-closed leaf (classified by
  reading; neither is a leak or violation):
  - `mk_vhost_init`: `if (!(dir = opendir(sites))) { …; exit(EXIT_FAILURE); }`,
    then `readdir(dir)`. The flow does not treat `exit` as no-return, so the
    null path reaches `readdir`: `nullable resource dir used by a non-null
    contract` (Unsupported). The source cannot take that path; the statement
    pair decides identically. An artifact of the flow's `exit` reading, kept
    fail-closed.
  - `rd_buf_alloc_segment0`: `else if ((seg = rd_malloc(…))) { … } else
    rd_assert(…); return seg;`. Under `NDEBUG` the null path reaches
    `return seg`: an Unsupported leaf and an `unknown_alias` leaf. Real under
    `NDEBUG`, again the statement pair's reading.
- A type operator or an elaborated increment in a leaf, the function held by
  other leaves: `cio_file_format_check`, `LZ4_count`, `read_variable_length`,
  `LZ4HC_InsertAndGetWiderMatch`, `mk_socket_reset`,
  `rd_kafka_mock_broker_new_listener`, `rd_kafka_transport_new` (2),
  `rd_kafka_transport_poll`.

Leaf deltas: condition refusals −15 (12 branch, 3 loop), `unknown_alias` −15,
Unsupported nullable-use +2. No leak, double release or other violation is
added.

**Census** ([condition-census.txt](artifacts/condition-census.txt),
consultations de-duplicated per unit, function, line and tokens): 2,334
admitted as before, **123 admitted with an elaborated leaf, 10 with a type
operator, 4 with a split acquisition, 1 with both**; refused: 2,702 by a call
whose actual the typed routes refuse (`indirect memory access`), 975 by an
uncontracted call, 222 `AssignmentShape`, 68 `CallFreeLeafRefused`, 32
`ResidualRefused`, 31 recursive-component callees, 30 `IndirectCallee`, 22
`Braces`, 12 `AssignmentDestination`, 5 `increment-beside-call`, 3
`connective-beside-call`, 2 `acquisition-on-cycle`, 26 other classifier
refusals.

## 5. Prediction against outcome

| Frozen | Outcome |
|---|---|
| ≤ 13 of CG00's 28 remaining functions Unknown → Safe (point estimate 8), conditional on the leaf taking typed actuals | **0**. The 13 "actual forms" functions are blocked by typed actuals on a bound root (rule 10k's pointer-actual premise: `strcmp(p->shortname, …)` with `p` a loop variable) or addresses of places (CG11), not by the condition's composition; the prediction misread the typed call actual route |
| 15 stay Unknown for named reasons (6 alias, 8 address of a place, 1 nested call) | **held** |
| E09: ≤ 5 Unknown → Safe, 0 new provisional Unsafe | **0** and **0**: 5 split acquisitions admitted, in functions other leaves hold; 2 refused by the placement guard |
| Partners: up to 36 of CG00's one-partner functions | **1** (`rd_kafka_mock_cluster_op_io`, a type operator in an actual) |
| No Safe → non-Safe | **held** |

## 6. Axes (analysis §8)

- **Position:** condition (`if`, `while`, `for`, `do`, `switch`) covered;
  the scalar `return` operand reads the same leaf and refuses a split (its
  exit must escape nothing); statement, actual, nested and `for` clause are
  other cards'.
- **Effect:** Owned, Reallocated, descriptor (split), None and Derived
  (unchanged); Out gets no split (CG23); Released and Consumed in a leaf are
  unchanged (rule 5h).
- **Evaluation:** once, in position; zero times under a skipped `&&`/`||`
  operand; two calls of one leaf still need identity effects; no order is
  introduced (a write beside a call in one leaf refuses).
- **Place:** local destinations; formals, members and pointers refuse
  (CG11's stores).
- **Type:** pointer NULL, descriptor sentinel, `void *`, typedef'd pointers;
  a carrier mismatch refuses.
- **Boundary:** declared and validated summaries; recursive-component callees
  refuse.
- **Verdict:** `Safe` only as the statement pair's; a failure edge that leaks
  in the source is reported (`a_failed_realloc_in_a_condition_keeps_the_old_identity`).

## 7. Tests and controls

- `tests/condition_acquisition.rs` (9 tests): example A and eight variants
  against their statement pairs; no hoisting; refusal controls for the `&&`
  right operand, a failed realloc in a condition, an unguarded use after the
  condition, an acquisition on a cycle (loop, loop body, backward `goto`),
  unlisted shapes; complex leaves admitted and refused; the pinned defect.
- `tests/unit/condition_acquisition.rs`: the four tables, all 16 feature
  combinations, and the four Lean bindings (shared vectors).
- `tests/unit/legacy_condition_acquisition.rs`: the lexical readers,
  `CycleRanges`, and **one fixture per refusal arm** of the desugaring
  (`IndirectCallee`, `Braces`, `AssignmentDestination`, `ResidualRefused`,
  `CallFreeLeafRefused`, `SwitchShape`), each asserting the arm that answered.
- **Controls flipped deliberately:**
  - `clang_gap_phase1::e09_an_owned_acquisition_in_a_condition_is_refused_and_not_hoisted`:
    the three branch forms now `Safe` with the call applied once; the loop
    form stays refused (placement).
  - `clang_gap_phase1::e13_condition_leaf_composition_is_partial`:
    `r_borrow_code(p) == sizeof(int)` and `i++ < 4 && r_borrow_code(p)` now
    `Safe`; `== i++`, `== s->a`, the uncontracted call and the expression
    callee stay refused.
  - `condition_call_desugar::an_owned_result_assigned_in_a_condition_keeps_todays_exact_lowering`
    (Q47): the acquisition is now `Safe`, applied once in the condition's
    plan with its `BranchOnNull`; discarded owned results stay refused.
  - `condition_call_desugar::shapes_outside_the_desugaring_keep_the_refusal`:
    `flag++ && r_borrow_code(p)` moves to the new
    `a_write_in_its_own_leaf_is_sequenced_by_the_connective`; the fixture is
    now `r_borrow_code(p) == flag++`.
  - `condition_call_desugar::a_descriptor_keeps_the_sentinel_validity_rules`:
    the owned descriptor assigned in the condition is now `Safe`; a
    comparison that refines nothing (`== 3`) is decided as the pair (leak).
  - `cg02_elaboration::refusals_stay_byte_for_byte`: `while (n-- > 0 && m)`
    is now `Safe` (the increment is its own call-free leaf, the elaborator's,
    sequenced by `&&`; CG02's comment had given it to CG06, the brief gives it
    here); `while ((n-- && m) > 0)` is the new refused control.
  - `typed_const_pointer_value::a_const_pointer_value_from_a_bound_root_is_not_safe`:
    `if (!(b = c_make(n))) return 0; return b->item;` loses its branch
    refusal and equals the assignment-statement pair (`Unknown`, the return's
    refusal). The pair `b = c_make(n); if (!b)` and the declaration
    `const struct c_box *b = c_make(n); if (!b)` already differed on the base
    (Unknown against a provisional Unsafe leak); the condition form reads as
    the assignment pair.
  - `call_instantiation` unit table: five `tested-split` rows.

## 8. Lean

`QCore.ConditionAcquisition` (audited in `AuditConditionAcquisition.lean`,
30 theorems, no placeholders, no project axioms):

- the NULL split preserves custody: `exactly_one_edge_carries_the_obligation`,
  `every_outcome_reaches_one_edge`, `the_obligation_follows_the_outcome`;
- acquisition evaluated once, in position:
  `the_acquisition_is_applied_as_c_evaluates_it`,
  `an_acquisition_leaf_applies_its_call_once`;
- under short-circuit an unevaluated operand acquires nothing:
  `an_unevaluated_operand_acquires_nothing`,
  `the_right_operand_acquires_only_when_the_left_holds`;
- the realloc failure arm keeps the old identity:
  `on_the_failed_edge_the_old_identity_is_live`, `exactly_one_identity_is_live`;
- unlisted shapes are refused: `only_listed_leaves_are_admitted`,
  `an_unsplit_custody_result_keeps_its_refusal`,
  `a_call_in_a_type_operand_is_refused`, `a_type_operand_applies_no_call`,
  `an_admitted_leaf_applies_every_call`;
- the guard: `off_every_cycle_a_site_has_one_generation`,
  `a_merged_site_would_hide_a_live_generation`;
- four `*_vectors_hold`, bound to `USE_VECTORS`, `CARRIER_VECTORS`,
  `PLACEMENT_VECTORS` and `LEAF_VECTORS` by shared-vector tests.

Premises stated, not proved: the flow's `BranchOnNull`/`BranchOnValidity` and
reallocation arms are the custody meaning of the split, and faithful Clang
extraction (trust boundary 28).

## 9. Gate

See [gate/](gate/) for every exit and [gate.json](gate.json). Commands and
counts are in the increment paragraph and the card's reply.

## 10. Ledger

- **Retired:** `CG04-CONDITION-ACQUISITION` (every DISCHARGE clause holds);
  `W44-CONDITION-CALL-TYPE-OPERATOR` (its discharge holds: a call-free type
  operand is residual text, with `a_type_operand_applies_no_call`; 0 leaves
  remain in its class); `CONDITION-DESUGAR-REFUSAL-ARMS-UNPINNED` (one test per
  arm; the six W44 leaves are not identifiable from W44's archive, and the
  current analog is registered).
- **Resized, with the AFTER census** ([followups.txt](artifacts/followups.txt)):
  `W44-CONDITION-CALL-ASSIGNMENT` 231 leaves / 95 functions;
  `W44-CONDITION-CALL-FREE-OPERAND` 63 / 44;
  `W44-CONDITION-CALL-REFUSED-SHAPES` 72 / 62;
  `W43-INDEX-TRAVERSAL-CONDITION-WRITE` 172 / 103 → 71 / 62 under this card's
  definition. `W44-CONDITION-DISCARDED-DERIVED-RESULT` noted (the reading is
  kept; the integrator's decision remains its discharge).
  `W44-CONDITION-CALL-INDIRECT-ARGUMENT` noted with its size (1,768 / 708).
- **Registered:** `CG04-LOOP-SITE-GENERATIONS` (soundness, §3) and
  `CG04-UNCONSULTED-CONDITION-LEAVES` (129 leaves / 86 functions with no
  consultation row; [unconsulted.txt](artifacts/unconsulted.txt)).
