# CG06-VALUE-CONNECTIVE-RESIDUALS, sibling-effect part: value regions composed with CG05's footprints

Wave 48, round 8 (the composition batch), route G. Authority: the registered
follow-up `CG06-VALUE-CONNECTIVE-RESIDUALS` (recorded by
CG06-VALUE-CONNECTIVES-AND-CONDITIONAL, rule 10z), its classes "a sibling of the
region with an effect" (1,133) and "a deciding call beside a non-inert sibling"
(4) only.

- **Rule:** [gateway 11m](../../agents/gateway.md#constraint-11m) (amends 10z's
  order premise only).
- **Base:** local `main` `d4df096f` at dispatch (round 7 complete); rebased onto
  main `0a22a998` (MSVC C ABI declarations, disjoint files) at the
  integrator's instruction. The final figures (§3) are against `0a22a998`.
  Commit map across the rebase: census build `c2461609` → `6cd0fd7c`, Lean
  `46f2aa61` → `9e609849`, prediction `82e47234` → `ce81fa94`, implementation
  `56138e4a` → `111a4bd3`, then `3cb8a830` (rustfmt) and `8f3a6a18` (clippy).
- **Prediction:** [PREDICTION.md](PREDICTION.md), frozen before the admitting
  commit and before any AFTER run, from a census build that computed the
  composition without admitting it.
- **Lean:** `QCore.ValueConnectives`' new composition section, 17 theorems
  registered in `proof/AuditValueConnectives.lean` (261 audits, none with a
  placeholder or project axiom).

## 1. What changed

### 1.1 The composition (`connective_footprints.rs`, new)

Where rule 10z's `admit` refuses `SiblingEffect` or `UnorderedDecision`, the
region is composed with its siblings (the subexpressions off the root-to-region
path, exactly 10z's list). For **each arm** separately, one `EvalGraph`:

- the deciding operand's events, then `Branch`, then that arm's events, then
  `Merge`, with the sequenced-before edges C guarantees inside the region (the
  deciding operand before the branch, the arm after it);
- every sibling's events, with no edge to the region's or to each other's (C
  guarantees none; none is chosen, no left-to-right serialisation).

Every operand is read by CG05's own reading (each evaluated call at any depth,
with its contract footprint and its actuals sequenced before it; each named
read with its cells). The region is admitted only when
`EvalGraph::check_footprints` (unchanged) admits **both** arms' graphs. The two
arms never share a graph, so they never pair (`exclusive`'s reading; the shared
`check_footprints` does not consult `exclusive`, and its meaning is not
changed). `Branch` and `Merge` carry empty footprints.

Refused by name (`Composed`): a sibling, deciding operand or arm with an
effect that is not a call (store, increment, statement expression, volatile
access: invisible to the footprints, so class (d) stays refused; CG02's order
model is not asked); an uncontracted sibling call (class (c), no footprint);
any operand CG05's reading refuses; a deciding operand meeting a sibling's call
at an address (below); every pair that does not commute (class (b)).

**The extra premise.** CG05 gives a scalar object's storage no cell
(`&n` of a scalar writes nothing in the custody state). A deciding operand's
value steers the branch, so the composition additionally refuses a deciding
operand that reads an object whose address a sibling's call takes, and a
deciding call that takes the address of an object a sibling reads. It refuses
only; it matched no corpus consultation.

### 1.2 Admission (`value_connective.rs`, additive)

`admit_composed` = `admit`, with its two order refusals asked of
`Reading::compose` (an omitted-middle operand with an effect is never composed:
C evaluates it once and the lowering re-reads it). An undischarged refusal is
`Refusal::Composition(premise, why)`, named in the parity row as `…; composed
with CG05's footprints: …`. Then rule 10z lowers as before.

### 1.3 Edges (an amendment found while implementing, recorded in PREDICTION.md)

An admitted region's edge statement (`rd_kafka_log0(…, arm, …, f(x))`) holds
nested calls, so the classifier refuses it with O04-B09B10's nested refusal and
rule 10y's consumer is asked. That consumer matches its Clang node by line and
tokens; an edge's statement is spelled nowhere, so every such edge failed with
"no unique Clang node". `value_connective::edge_root` now carries the edge's own
node (the source node with each region replaced by its arm, the node 10z's
`leaf_events` already reads), and rule 10y's statement and `return` consumers
fall back to it only where no source node matches. The edge is still lowered by
the position's older routes only; any refused edge declines the whole
expression byte for byte. On the corpus the fallback moves none of the 411
older edge refusals.

### 1.4 Shared items added (for the integrator)

- `nested_call_effects.rs`: `Operands` (new, `pub(crate)`: `new`, `operand`,
  `control`, `sequence`, `check`, test-only `graph`). It drives the private
  `Reader` unchanged; `check` is `EvalGraph::check_footprints` unchanged.
- `value_connective.rs`: `admit_composed`, `Reading::compose` (a new field:
  **every `Reading` construction site must set it**), `Refusal::Composition`,
  `Composer`, `edge_root`/`with_edge_root`, `COMPOSED_VECTORS`.
- `legacy_value_connective.rs`: the composer over `nested_facts`; the edge
  root set around each leaf (`leaf` split into `leaf` + `edge`, same order of
  routes).
- `legacy_nested_call.rs`: `GFacts` and `nested_facts` are `pub(super)`; the
  edge-root fallback in `lower_nested_statement` and `lower_nested_return`.
- `elaboration.rs`, `legacy_elaboration.rs`, `typed_place.rs`,
  `call_instantiation.rs`: **not touched** (CG05-ELABORATOR-CALL-FOOTPRINTS
  owns `elaborate` and its lowering). `check_footprints`, `footprints_commute`
  and `check_with` are used as they are.
- `lib.rs`: `mod connective_footprints`.

## 2. Cited claims

| Claim (ledger / brief) | Outcome |
|---|---|
| 1,133 refusals "a sibling of the region with an effect", mostly `rd_kafka_log0(…, f(x), …)` | **Reproduced** exactly on `d4df096f` and on `0a22a998` (de-duplicated parity rows); 799 of the 808 statements that later reach an edge are `rd_kafka_log0` |
| 4 "a deciding call beside a non-inert sibling" | **Reproduced** (4) |
| the other classes (420 edge, 291 undecided, 284 I04, 67 comma, 14 indirect callee, 4 leaves, 2 nested assignment) | **Moved since the ledger**, re-sized on the BEFORE: 411 edge, 311 undecided, 284, 67, 14, 4, 2 (one nested assignment, one multi-declarator); 255 admitted of 2,485 (the ledger's 233 / 2,452 was CG06's own tip; later cards moved them) |
| the brief's admitted control `log(ctx, p ? strlen(p) : 0, release_and_status(q))` "with disjoint identities" | **Correction.** With two names `p` and `q`, CG05's cells cannot admit it: two identity cells always overlap (`cells_overlap`, names may alias). It is pinned **refused**; the admitted control releases an identity the footprints prove distinct, a fresh owned result (`r_release_code(r_wrap(p))`) |

## 3. Measurements

Scripts: [measure.sh](scripts/measure.sh), [compare.py](scripts/compare.py),
[census.py](scripts/census.py), [predict.py](scripts/predict.py),
[row_moves.py](scripts/row_moves.py), [edge_blockers.py](scripts/edge_blockers.py).
Runs under `private/cg06r/` and `…/private/w47-tail/attempt-1/cg06r-*`; every
exit 0. `--jobs 4`.

| Route | BEFORE (main `0a22a998`, binary `1317cd77…`) | AFTER (tip `8f3a6a18`, binary `11f5523c…`) | Moves |
|---|---|---|---|
| corpus (`66910c10`) | **628 / 10 / 4,196** (`3b09309a…`) | **628 / 10 / 4,196** (`f9b3420e…`) | **none**; 2 Unknown functions lose one leaf each |
| system | 44 / 2 / 188 (`95cfc3c9…`) | byte-identical | none |
| tail-pause slice | 29 / 1 / 239 / 1 Pending (`8fbcb0a2…`) | byte-identical | none |

The corpus BEFORE on `0a22a998` is byte-identical to the BEFORE on `d4df096f`
and to the census build's corpus (`3b09309a…`), and differs from CG12's
integration `394b190b…` only by CG18's known delta (2 functions, 4
indirect-access leaves become `unknown_alias`). The AFTER at the pre-rebase
tip `56138e4a` (`private/cg06r/after1`) is byte-identical to the AFTER here.
The system run at that tip was interrupted by a host restart and not used.

**The two leaf changes, by reading.** `cmt_encode_text.c::get_data_point_metadata_context`
(line 5731) and `cmt_encode_opentelemetry.c::get_data_point_otlp_metadata_context`
(6870): `snprintf(key, sizeof(key) - 1, fmt, metric != NULL ? metric->hash : 0,
metric != NULL ? cmt_metric_get_timestamp(metric) : 0)`. The first region's
sibling is the second region, whose arm holds a call; the composition admits it
(the borrowed call commutes with every read), each of the four edges is
lowered, and the statement's `indirect memory access` leaf is gone. Both
functions keep other leaves (uncontracted `fetch_metadata_kvlist_key`). No leaf
was added: **no leak, violation or `unknown_alias` added, no Safe → non-Safe
move, no correction.**

### 3.1 The consultations ([parity-census-after.json](artifacts/parity-census-after.json), [row-moves.json](artifacts/row-moves.json))

Of the 1,137 order refusals, **987 compose** (985 sibling, 2 deciding) and 150
stay refused:

| Class | Rows (functions) | Owner |
|---|---:|---|
| (c) an uncontracted sibling call: `rd_kafka_err2str` 46, `rd_kafka_err2name` 23, `rd_kafka_fetch_pos2str` 15, `rd_kafka_broker_name` 13, `rd_kafka_offset2str` 7, `rd_kafka_broker_id` 6, … | 143 (99) | the callees' contracts (`CG05-UNCONTRACTED-INNER-CALLEES` names the same callees) |
| (b) footprints overlap: a read of `mconn` against `rd_list_cnt(&mconn->…)` | 3 (2) | `CG05-FOOTPRINT-ADDRESS-CONSERVATISM`'s identity distinctness |
| the deciding call has no contract (`rd_kafka_fatal_error_code`, `rd_kafka_op_version_outdated`) | 2 (2) | contracts |
| (d) a sibling increment (`grps[i++] = rd_strndup(…)`) | 1 (1) | kept refused (CG02's order model is not asked here) |
| a sibling CG05 does not read (an indirect call, `rd_map_copy`) | 1 (1) | CG22 |
| the deciding operand meets a sibling's call at an address (the extra premise) | 0 | — |

Each of the 987 is then lowered by rule 10z:

| Outcome | Rows |
|---|---:|
| **admitted** (both cmetrics `snprintf` statements) | **2** |
| an edge an older route refuses | 976 (808 statements; 799 `rd_kafka_log0`) |
| more than 8 leaves | 5 |
| a deciding operand no condition route decides | 4 |

The 808 edge-refused statements' first `unsupported` leaf on their line
([edge-blockers.json](artifacts/edge-blockers.json)): the lexical
indirect-access gate 757 (the logging macro's own `&rk->rk_conf`,
`rkbuf->rkbuf_rkb` actuals), `sizeof` 48 (CG08), an uncontracted call 3.
These are the edge's own refusals, owned by the place cards (CG10 /
`CG05-CALL-ROOTED-PLACES`), CG08 and the contracts, never by the composition.

Consultations: 2,485 (unchanged), admitted 255 → **257**; refusals now: 1,387
edge, 315 undecided, 284 I04, 148 sibling, 67 comma, 14 indirect callee, 9 over
8 leaves, 2 deciding call, 2 nested assignment / declaration.

### 3.2 Prediction against outcome

| Predicted | Outcome |
|---|---|
| at most 987 composed | **987** composed |
| 0 moves from the composition (upper bound 2, both read as staying) | **0**: `rd_kafka_AclBinding_request_size` refuses on 16 leaves, `rd_kafka_buf_read_CurrentLeader` on its edges |
| 0–4 moves in total with the edge fallback (upper bound 24) | **0** |
| no Safe → non-Safe | none |
| system and slice unmoved | byte-identical |

## 4. Tests and controls

- `tests/cg06_connective_footprints.rs` (route G, the brief's controls):
  `cg_log("%d %d", p ? r_borrow_code(p) : 0, r_release_code(r_wrap(p)))` →
  **Safe** (the released identity is fresh); a borrowing sibling → Safe; the
  same beside an owned `p` → its `Leak` (the composition never hides one);
  the released identity `p` itself, another name `q`, an uncontracted sibling
  (`opaque()`), a releasing arm beside a borrowing sibling → each **refused**
  (neither Safe nor Unsafe); a sibling increment and a releasing deciding call
  beside a sibling read → refused.
- Unit `connective_footprints::tests` (5): `FOOTPRINT_VECTORS` through Clang,
  each refusal's class, an effect-free sibling never composed, the two arms in
  two graphs (a release on one arm and a consumption on the other would not
  commute in one), the Lean binding.
- Unit `value_connective::tests::cg06_composed_admission_table_and_lean_vectors`.
- **Flipped controls: none.** No existing control moved (the CG00
  `clang_gap_phase*` E12 controls and CG05's/CG06's own pass unchanged): no
  fixture before this card had a value region beside a contracted sibling
  call.

## 5. Lean

`QCore.ValueConnectives` imports `QCore.NestedCallEffects` and adds:
`admitComposed` with `composed_vectors_hold`, `composition_extends_admit`
(10z's admissions stand), `uncomposed_is_admit`, `composition_never_rereads`;
`armTree` (deciding operand, `Branch`, arm, `Merge`, unsequenced with the
siblings' tree), `ctl_respects`, `composed_arm_runs_alike` and
**`composed_region_runs_alike_on_each_arm`** (an admitted arm runs alike in
every order C permits, the lowering's included; from
`every_interleaving_runs_alike`); **`decision_value_stable`** (whatever
sibling events C runs before the deciding operand, it reads one value, so C
takes the lowering's edge; from `disjoint_of_no_clash`, i.e.
`footprint_commute_sound`'s frame argument); **`an_overlapping_sibling_is_refused`**,
`the_released_identity_is_refused`, `another_name_is_refused`,
`a_fresh_release_commutes`; `Sibling`, `siblingTree`, `compose`,
`unread_sibling_tree`, **`an_uncontracted_sibling_is_refused`**,
`a_non_call_sibling_is_refused`, `compose_admits`; `footprintVectors` with
`footprint_vectors_count`. Bound by `composedVectors` = `COMPOSED_VECTORS` and
`footprintVectors` = `FOOTPRINT_VECTORS`.

Premises stated, not proved: faithful Clang extraction (trust boundary 28);
CG05's (a call's effect on the abstract state is its contract's); rule 10z's
(an edge's older route decides its statement as written in source). The
extra address premise refuses only and has no Lean counterpart beyond
`compose`'s refusals.

## 6. Gate

See [gate.json](gate.json) and [gate/](gate/) (script [gate.sh](scripts/gate.sh),
logs `private/cg06r/gate-8f3a6a18`). `providers/` is untouched, so the provider
gate is not due. `cargo check --workspace --all-targets` on the rebased tree:
exit 0, no warning.

## 7. Ledger

- `CG06-VALUE-CONNECTIVE-RESIDUALS`: **amended**, not retired. Its two
  sibling-effect classes are re-sized (1,137 → 150 order refusals; the 987
  composed now refuse on their own edges, leaf cap or condition), and every
  other class re-sized on the BEFORE.
- No new entry: every residual has an owner already (`CG05-UNCONTRACTED-INNER-CALLEES`,
  `CG05-FOOTPRINT-ADDRESS-CONSERVATISM`, `CG05-CALL-ROOTED-PLACES`, CG08, CG22).
