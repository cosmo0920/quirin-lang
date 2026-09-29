# CG06-VALUE-CONNECTIVES-AND-CONDITIONAL: value `&&`, `||` and `?:` by branching

Card CG06 of the [Clang gap card queue](../../plans/clang-gap-cards/README.md)
(wave 48, round 4; base `27dc7c9a` = local `main` at dispatch, CG03's final
records commit).

- **Gap entries:** E06 and E07, on route **G** (the custody gateway). Route T
  is unchanged: no typed-source consumer decides a value connective (CG01's
  census), so T keeps its refusals and the CG00 T controls stand.
- **Evidence:** Q11, Q13, Q14, Q19, Q37.
- **Rule:** [gateway 10z](../../agents/gateway.md#constraint-10z) (amends 10s
  and 10n, cross-referenced there).
- **Lean:** `QCore.ValueConnectives`, `proof/AuditValueConnectives.lean`
  (28 theorems); `QCore.ElaborationCore`'s form table updated.
- **Prediction:** frozen in [PREDICTION.md](PREDICTION.md) (commit
  `d1a79820`), before any AFTER run. The first implementation commit
  (`4fed4fcf`) and fixture probes preceded it; no corpus figure did.

## 1. What changed

### 1.1 The elaborator's branch regions (`elaboration.rs`, additive)

The three rows CG02 reserved for this card (`value &&`, `value ||`,
`conditional`) are now `Disposition::Branching`, a new variant, and a new
row `omitted-middle conditional` is added (29 rows). `elaborate` emits a
region as: the deciding operand's events, `Event::Branch`, each arm's events
sequenced after the branch, `Event::Merge`.

| Form | Deciding operand | Non-zero arm | Zero arm | Value |
|---|---|---|---|---|
| `A && B` | `A` | `B` | none | `int` 0/1 |
| `A \|\| B` | `A` | none | `B` | `int` 0/1 |
| `C ? A : B` | `C` | `A` | `B` | the conditional's class |
| `X ?: B` | `X`, once | none (`X`'s value) | `B` | the conditional's class |

The two arms are recorded in `EvalGraph::exclusive`; `check` never pairs an
event of one arm with one of the other (they never both occur) and still
checks each against every event outside the region it is unordered with.
`Branch`/`Merge` commute with everything (`commutes`, a new first arm).
`Elaboration::branches` lists each `BranchRegion`. Because CG02's routes
lower only reads, scalar writes and sequenced calls, a graph with a branch
still refuses in CG02's own event lowering (`event outside this card's
lowering`); what changes there is that the whole-statement order check no
longer refuses a statement for holding a connective, so a comma or chain
statement's pieces reach the older lowering, and with it this card's hook.

### 1.2 Value regions (`value_connective.rs`)

For a statement, a one-declarator declaration or a `return` value, the full
expression's Clang node (matched by line and exact tokens) is walked for
**regions**: connectives and conditionals C evaluates as part of it, found
through `TRANSPARENT` positions (parentheses, implicit and C casts, unary and
binary operators, a direct call's actuals, member and subscript bases, the
root's `=`/`op=` operands and a declaration's initializer). A region inside
an arm is a region of that arm; one in a deciding operand is the condition
CFG's; one under `sizeof` is no region. Under a comma, a nested assignment,
a statement expression, an initializer list, `_Generic` or an indirect callee
it refuses by name, as does an unparenthesised comma middle operand
(`C ? A, B : D`, which would re-parse once substituted).

**The order premise** (`admit`): the lowering evaluates the deciding operand
first. C sequences it before the arms and before every consumer on the path
from the root to the region, and leaves it unsequenced only with the
region's **siblings**. So every sibling must be effect-free (no call, store,
increment, statement expression or volatile access), and a deciding operand
with a call needs every sibling **inert** (a literal, a designation, or a read
of an automatic object whose address is never taken). `X ?: B` needs a
reads-only `X`, because the lowering reads `X` again on the non-zero edge.
At most 8 leaves.

### 1.3 The route-G lowering (`legacy_value_connective.rs`)

Two hooks, each the **last** route of its position (hard invariant 3):
`expression_statement`'s last step for a refused statement, and the `return`
branch after CG03's `instantiated_return`. A declined consultation restores
the site and element counters, so the refusal stays byte for byte.

For the first region, the deciding operand (`C`, `X`, or the whole
connective) becomes an `if` condition decided by the `if` statement's own
routes, in `parse_statement`'s order (lexical support, B02 call, typed access,
null/element/distinct tests, then rule 5h's condition CFG, which gives the
short circuit). Each edge carries the expression with the region replaced by
the arm C evaluates there (`1`/`0` for a connective, `X` for `X ?: B`),
lowered as a statement or `return` of its own by the position's older routes
(the whole chain: `return_statement`, then CG02's and CG03's fallbacks).
Two thin additions let those routes read an edge as C evaluates it:

- `value_connective::edge_node` in `argument_node` (rule 10k's call actual):
  on an edge, a region standing at an actual (through parentheses and
  value-preserving implicit conversions) is read as its chosen arm's node;
- `value_connective::deciding_candidate` in `expression_node`: among several
  Clang nodes with an operand's line and tokens, after rule 10n's
  controlling-scope match declined, the one inside the deciding operand being
  lowered.

A refused statement edge is then read from its Clang node with the arm
substituted (`value_connective::substitute`) and lowered from its elaborated
events exactly as CG02 lowers a refused piece (`leaf_events`, a new
`pub(super)` wrapper of `event_instructions`). Any undecided condition, any
refused edge, and any edge whose lowering carries an explicit refusal (an
`Unknown` step or a `Return` whose evaluation is `Unknown`) declines the
whole expression.

**Merge.** The edges join at the `if`'s join, the flow's existing join:
alias targets are united and statuses joined, so two owned identities become
one binding with two targets and a release through it is refused
(`UnknownAlias`), never resolved to one; every obligation of either edge is
carried.

### 1.4 Shared items added (for the integrator)

- `elaboration.rs`: `Disposition::Branching`; the three reserved rows
  switched to it plus the new `omitted-middle conditional` row;
  `EvalGraph::exclusive` and `EvalGraph::excluded`; the `Branch | Merge` arm
  of `commutes`; `Elaboration::branches` and `BranchRegion`; the private
  `region`/`connective`/`conditional`/`omitted_middle` methods. No existing
  variant's meaning changed (`Branch`/`Merge` were reserved and unused).
- `legacy_elaboration.rs`: `Parser::leaf_events` (new `pub(super)`).
- `typed_access_positions.rs`: one-line hooks in `argument_node` and
  `expression_node` (both inert unless this card's lowering is running).
- `legacy_c.rs`: `mod value_connective_route` and the two hooks.
- `lib.rs`: `mod value_connective`.
- `call_instantiation.rs` is **not** touched: `accept` and the typed
  substitution keep refusing a connective or conditional (rule 10u); this
  card reaches actuals by re-lowering the whole call per edge.

## 2. Cited claims

| Claim | Evidence | Outcome |
|---|---|---|
| E06: G decides `&&`/`\|\|` only as a condition CFG; T keeps them in trees but refuses short-circuit values outside `branch_condition` | Q13 Q14 Q19; CG00 `e06_…` | **reproduced** on the BEFORE (the CG00 control ran green at `27dc7c9a`) |
| E07: G's `?:` only in condition contexts; T's expression handler has no `ConditionalOperator` | Q11 Q19 Q37; CG00 `e07_…` | **reproduced**, with CG00's recorded qualification: a scalar GNU `?:` initializer and a call-free scalar `?:`/connective return (rule 10n) were already admitted |
| The 40 functions CG00 predicted freed alone | `predictions/CG06.md` | all 40 still Unknown at the BEFORE, each with its CG00 leaf ([rederive-before.json](artifacts/rederive-before.json)); 2,052 of CG00's 2,077 E06/E07 leaves still present |

No claim failed to reproduce.

## 3. Measurements

Scripts: [measure.sh](scripts/measure.sh) (corpus arbiter, system route,
tail-pause slice; release `quirin-audit`), [compare_all.sh](scripts/compare_all.sh),
[compare.py](scripts/compare.py), [changed.py](scripts/changed.py),
[parity_census.py](scripts/parity_census.py), [absorbed.py](scripts/absorbed.py),
[rederive.py](scripts/rederive.py). Runs under `private/cg06/` (worktree) and
`private/w47-tail/attempt-1/cg06-*`; every exit 0.

### 3.1 BEFORE (base `27dc7c9a`, binary `fb62d605…`)

| Route | Figures | Identity |
|---|---|---|
| corpus (`66910c10`) | 564 / 10 / 4,260 | `resource-diagnostics.json` `7393fd21…`, **byte-identical** to CG03's archived AFTER |
| system (`046accf6`) | 33 / 2 / 199 | status and leaf multiset identical to CG03's AFTER (`compare.py`); bytes differ only in worktree-path fields (review-hold module paths, inventory and contract-source hashes) |
| tail-pause slice (`90978ac6`) | 26 / 1 / 242 / 1 Pending | `compare.py` against CG03's `cg03-after-main2`: 0 moves, 0 changed functions |

### 3.2 AFTER (tip code `7693c599`, binary `34949f1b…`)

| Route | BEFORE | AFTER | Moves |
|---|---|---|---|
| corpus | 564 / 10 / 4,260 | **582 / 10 / 4,242** (`16b723de…`) | **+18 Unknown → Safe**; 158 more Unknown functions change diagnostics |
| system | 33 / 2 / 199 | **34 / 2 / 198** | +1 (`flb_chunk_route_plugin_matches`); 3 functions change leaves |
| tail-pause slice | 26 / 1 / 242 / 1 Pending | **26 / 1 / 242 / 1 Pending** | none; byte-for-byte leaves |

[corpus-comparison.json](artifacts/corpus-comparison.json),
[corpus-changed.json](artifacts/corpus-changed.json),
[system-comparison.json](artifacts/system-comparison.json),
[slice-comparison.json](artifacts/slice-comparison.json).

**No Safe → non-Safe move, no correction, no new provisional Unsafe, and no
leak or violation added** (the leaf deltas hold no `Leak`, `DoubleRelease`,
`UseAfterRelease` or conflict entry).

### 3.3 The eighteen moves, each attributed by reading

- **Connective returns with calls** (the condition CFG applies each call on
  the edge C evaluates it; each edge returns `1`/`0`):
  `rd_kafka_broker_needs_persistent_connection` (`a || rd_atomic32_get(&…)`),
  `rd_kafka_conf_validate_partitioner` (`!strcmp(…) || …`, seven leaves),
  `rd_kafka_toppar_fetch_decide_start_from_next_fetch_start` (`… || cmp(&…,
  &…) || …`), `rd_kafka_share_acknowledgement_mode_is_explicit` /
  `…_is_implicit` (`p->m && !strcmp(p->m, "…")`).
- **Pointer `?:` returns** (W47-ACCESS-PATH-POSITION-RESIDUALS's named
  shape): `get_client_rack`, `get_group_id`, `get_group_instance_id`,
  `get_transactional_id` — `return P ? (const char *)P : NULL` (and
  `P && LEN(P) ? (const char *)P->str : NULL`); each edge's return is rule
  10k's never-bound pointer return or the null constant.
- **Stores and declarations**: `rd_kafka_buf_calc_timeout` (`RD_MIN` into a
  member, the edge lowered from its substituted node as a CG02 piece),
  `rd_kafka_ConfigResource_cmp` (`int r = RD_CMP(a->restype, b->restype)`,
  same), `rd_kafka_sasl_plain_conf_validate` (`both_set = P->u && P->p`),
  `ut_build_record_v2` (two `x = … ? … : …` assignments),
  `flb_chunk_route_plugin_matches` (`stored_length = (route)->len > 0 ?
  (int)… : …`, two regions; also the system slice's move).
- **A call actual**: `rd_kafka_mk_brokername`
  (`rd_snprintf(…, source == L ? "" : (source == I ? "/internal" :
  "/bootstrap"))`, nested regions, string-literal arms).
- **Summary validation on the reported premises (rule 4b)**:
  `rd_kafka_metadata_broker_cmp`, `rd_kafka_metadata_broker_internal_cmp`,
  `rd_kafka_metadata_partition_internal_cmp`. These are three of the eight
  summaries W47-SUMMARY-VALIDATION-PROTECTED-FORMALS withdrew because their
  own rows, analysed under the summary (`Owned`, `Owned`), refused: the return
  family refused `RD_CMP`'s reads of the bound copies `a`/`b` for standing
  in a `?:` arm. Rule 10z branches on the comparison, which C always
  evaluates, and reads each arm unconditionally on its own edge, so the row
  under the summary is Safe and the summary validates on exactly the premises
  that row reads. This is not a reversal of W47's correction: its criterion
  (validation follows the reported row) holds; the row changed. Their
  contract-free rows' four `unknown_alias` leaves each are gone because the
  reported row is now the one under the summary.
- **Cascades without a status move:** −12 branch-condition leaves over 9
  Unknown functions that call, in a condition, a function whose summary now
  validates: `flb_chunk_route_plugin_matches` (4 in
  `input_chunk_collect_output_references`), the fetch/share-acknowledgement
  helpers above, and row-less header helpers whose `RD_CMP`/`RD_MIN` bodies
  now lower (`rd_kafkap_str_cmp`: its "uncontracted" leaf, −1, is gone too).

**Leaf deltas** (corpus): −103 indirect-access, −84 unsupported assignment
expression, −13 return, −12 branch condition, −3 `rd_kafka_op_set_replyq`,
−2 `snprintf` data tail, −2 formal-identity mutation, −1 nested `fabs`, −1
`rd_kafkap_str_cmp`, −1 destination; **+113 `unknown_alias`** over 66
functions: newly admitted reads (an arm's member read, a deciding operand's
typed read) whose base the flow cannot resolve at the node — the obligation
of an admitted access failing, never a violation.

### 3.4 Prediction against outcome

Of CG00's 40 functions, **14 moved** (PREDICTION.md: 19 upper-bound Safe, 8
conditional on the edge actual, 13 Unknown):

- predicted Safe and moved (11): the five connective returns above, the four
  telemetry getters, `rd_kafka_sasl_plain_conf_validate`,
  `rd_kafka_buf_calc_timeout`;
- conditional and moved (2): `rd_kafka_mk_brokername`,
  `rd_kafka_ConfigResource_cmp`;
- predicted Unknown and moved (1): `ut_build_record_v2` (its regions are
  whole assignment values after all);
- predicted Safe, stayed Unknown (8), each a second cause read from the
  AFTER parity rows: `cJSON_HasObjectItem`, `rd_kafka_path_is_dir`,
  `rd_kafka_broker_destroy_error`, `rd_kafka_broker_needs_connection` (no
  `if` route decides the deciding operand: a same-unit callee's condition
  use, a `&out` actual, a call inside `& 0x1`, a nested call-bearing `||`
  under `!`); `rd_kafka_error_name`, `rd_kafka_error_string` (an edge's
  `return err2name(e->code)` is an uncontracted call); `ut_get_consumer_rack`
  (`sizeof` of an array on the edge, CG08); `log2_floor` (`__builtin_clz`);
- conditional, stayed Unknown (6): `rd_kafka_log_print` (the arm
  `rk->rk_name` is an array member's decay, rule 5j), `rd_kafka_queue_forward`
  and `rd_kafka_msgset_writer_compress_lz4` (the edge's call refuses its other
  actuals), `rd_kafka_MemberDescription_copy` (the edge's returned call),
  `ConsumerPair_cmp` (admitted; its `return` refuses otherwise),
  `rd_kafka_txn_schedule_register_partitions` (the edge's address actuals);
- predicted Unknown and stayed (12), as predicted.

Four moves were outside CG00's list: `flb_chunk_route_plugin_matches` and
the three metadata comparators (their CG00 blocker was `unknown_alias`,
attributed to no card). Corpus total +18 against the predicted +19…+27:
within the prediction's shape, below its upper bound, as §3.5 explains.

### 3.5 The consultations (AFTER parity, [parity-census.json](artifacts/parity-census.json))

2,452 de-duplicated consultations (2,338 statements, 114 returns), **232
admitted** in 163 functions. Refusals, each named in its parity row:

| Refusal | Rows | Owner |
|---|---:|---|
| a sibling of the region carries an effect (mostly `rd_kafka_log0(…, f(x), …)` logging macros) | 1,133 | CG05 (nested calls) |
| an edge's expression an older route refuses | 420 | the edge's own refusal (CG11 owned `?:` stores `p->f = s ? strdup(s) : NULL`, CG16 pointer arithmetic, uncontracted calls) |
| no `if` route decides the deciding operand | 292 | CG04 / the condition routes |
| no unique Clang node for the statement | 284 | I04 (two identical runs on one line; CG10's occurrence identity serves statement runs only) |
| under a comma | 67 | CG02 / CG08 (below) |
| under an indirect callee (`__builtin_assume(…)`) | 14 | CG22 |
| a deciding call beside a non-inert sibling | 4 | CG05 |
| more than 8 leaves | 4 | kept |
| under a nested assignment / a multi-declarator declaration | 2 | CG02 |

## 4. Absorbed and related follow-ups ([absorbed.json](artifacts/absorbed.json))

- **RETURN-POSITION-CONDITIONAL** (28 leaves, wave 33, a measurement
  request): answered and **retired**. Every refused `return` whose value
  holds a connective is now consulted: 114 consultations, 17 admitted; of
  the 97 refused, 67 are the nanopb comma below, 25 carry a call (9
  undecided deciding operands, 3 siblings with effects, 1 no unique node, 12
  an edge the return routes refuse, mostly an uncontracted call on the edge)
  and 5 are call-free with a refused edge. 58 Unknown functions keep a return
  leaf on a refused consultation.
- **W43-RETURN-CONDITIONAL-OPERANDS** (13 call-free leaves over 13
  functions): **retired**; the call-free non-comma remainder is 5 refused
  consultations, carried by the new residual below.
- **W47-ACCESS-PATH-POSITION-RESIDUALS**: **amended** — its pointer `?:`
  return rows are discharged (the four telemetry getters, Safe) and its
  `?:` actual rows are absorbed: rule 10n's 88 conditional call-actual
  residual-shape rows join on (function, line) to 5 statements admitted here
  (3 functions), 42 refused (31 functions) and 41 never consulted (24
  functions: another route decided or declined the statement first).
- **CG02-COMMA-OPERAND-RESIDUALS** (nanopb `PB_RETURN_ERROR`, 67 leaves over
  32 functions): **amended**. The first refusal is still the elaborator's
  `IntegralToBoolean` (CG08); this card is consulted on the same 67 returns
  and refuses by position (a region under the comma, which is CG02's to
  split). Past both, the piece `stream->errmsg = stream->errmsg ? … : "…"`
  is a pointer-valued member store of a pointer read, CG11's refusal.
- **CG06-VALUE-CONNECTIVES-AND-CONDITIONAL**: **retired** (every DISCHARGE
  clause holds: BEFORE reproduces, prediction frozen, every move attributed,
  absorbed entries closed or amended with re-measured sizes, Rust refusal
  controls, audited Lean).
- **New: CG06-VALUE-CONNECTIVE-RESIDUALS** — the refused consultations of
  §3.5 with their sizes and owners.

## 5. Tests and controls

- `tests/cg06_value_connectives.rs` (6 tests): admitted positions; the
  brief's controls — `x = c && release(p)` leaks on the zero edge (Unsafe
  `Leak`), `x = p && release(p)` is Safe (the zero edge is `p == NULL`);
  `r = c ? a : b; free(r)` refuses with `UnknownAlias r`, never picking one;
  `x ?: f()` runs `f` only on the zero edge (a leak where `x` is non-zero; a
  double release after it on the zero edge; `n++ ?: …` refused); premises
  that refuse; `mergeVectors` run through route G.
- Unit: `value_connective::tests` (admission table, Lean binding of
  `admissionVectors` and `transparent`, effect/inert readings);
  `elaboration::tests::cg06_region_vectors_match_the_elaborator` and
  `cg06_omitted_middle_evaluates_once`.
- **Flipped controls, each named in the test:**
  - CG00 `e06_…`: the `&&` initializer and the `||` actual → Safe;
  - CG00 `e07_…`: the pointer `?:` initializer → the join's
    `nullable resource p` refusal (identical to `if (flag) q = p; else q = 0;`),
    the `?:` return of two releases → Safe, the `?:` actual → Safe;
  - CG02 `cg02_unlisted_and_out_of_scope_forms_are_refused_by_name`: the
    three connective rows moved to the region vectors;
  - CG03 `k12_…keeps_every_refusal…`: `return r_borrow_code(p) && n` → Safe;
  - `access_path_position_residuals`: `return c ? q->head.first : q->arr` → Safe;
  - `constructor_residuals`: `d_report(d_slot_of(0) ? 1 : 0, "x")` → Safe,
    and `d_consume(d_slot() ? h : h); d_close(h)` → the fixture's own
    `DoubleRelease` (premise N8's refusal hid it);
  - `leak_pin_audit`: `return p->n > 0 ? p : 0` → the zero edge's leak;
  - `return_position_modeled_call`: `return r_release_code(p) ? 1 : 0` →
    Safe; `return flag ? r_release_code(p) : 0` and `flag && …` → Unsafe,
    one leak each (never Safe);
  - `return_position_pointer_leaf`: `return flag ? getter->value : 0` →
    the direct return's status;
  - `summary_validation_premises`, `typed_const_pointer_value`: the
    comparator's row under its summary and the wrapper → Safe (§3.3).
- Route T controls (CG00) unchanged.

## 6. Lean

`QCore.ValueConnectives` (28 theorems, `AuditValueConnectives.lean`, no
`sorry`, no project axiom): `and_skips`, `or_skips`, `and_evaluates`,
`or_evaluates`, `connective_value_boolean` (short circuit, 0/1 value);
`cond_selects`, `cond_yes`, `cond_no` (exactly one arm); `elvis_once`,
`elvis_skips`, `elvis_not_cond`, `elvis_reread` (single evaluation, and the
re-read lowering sound for a reads-only `X`); `decision_first`,
`inert_read_stable`, `control_commutes` (order); `merge_carries`,
`release_merged_refuses`, `release_never_picks`, `release_single` (merge);
`branching_forms_elaborated`, `unlisted_position_refused`,
`sequencing_positions_refused`, `admission_vectors_hold`,
`sibling_effect_refused`, `effectful_decision_needs_inert`,
`omitted_middle_effect_refused` (forms, positions, admission); `truth_boolean`,
`run_reads`. `QCore.ElaborationCore`: `Disposition.branching`, the updated
`formTable`, and `value_connectives_branch` replacing
`value_connectives_refused`. Bound by shared-vector tests: `admissionVectors`
= `ADMISSION_VECTORS`, `transparent` = `TRANSPARENT`, `regionVectors` and
`exclusiveVectors` through Clang and `elaborate`, `controlVectors` against
`commutes`, `mergeVectors` through route G, `formTable` = `FORM_TABLE`.

Premises stated, not proved: faithful Clang extraction and node matching
(trust boundary 28); that an edge's older route decides its statement as it
would the same statement written in source; the flow's join is the merge.

**Loop sites (`CG04-LOOP-SITE-GENERATIONS`).** An arm's acquisition is
lowered by the classifier as the same statement would be, one abstract site
per node, like every other acquisition; the branching duplicates the
enclosing call once per edge, never inside a loop body of its own making.
The merge relies on nothing per-node beyond the flow's existing site model,
so the defect CG04 registered for re-acquisition in loops applies here as it
does to any acquisition, and is not widened.

## 7. Gate

At tip code `7693c599` (script [gate.sh](scripts/gate.sh), logs
`private/cg06/gate`; exits in [gate/](gate/), [gate.json](gate.json)):

| Step | Exit | Counts |
|---|---:|---|
| `git submodule update --init --recursive` | 0 | |
| `python3 scripts/run_test_binaries.py` | 101 | 374 units, 6,663 passed, **1 failed**, 11 ignored: the known wall-clock flake `checkpoint_cli::interrupted_run_preserves_completed_units_for_resume` ("no unit checkpoint before timeout", W47-PREMISES-INSTANCE records it); rerun alone exit 0, 7 passed (`03b-checkpoint-rerun.exit`) |
| `cargo fmt --all -- --check` | 0 | |
| `cargo clippy --all-targets -- -D warnings` | 0 | |
| `reevaluation_probe`, `reevaluation_followup` | 0, 0 | |
| `python3 scripts/check_lean_audits.py` | 0 | 234 audits (233 + `AuditValueConnectives.lean`) |
| `python3 -m unittest discover -s scripts/tests` | 0 | 365 tests |
| `python3 scripts/build_docs_index.py` | 0 | |

`providers/` is untouched, so the provider gate is not due. Clang 21.1.8.
