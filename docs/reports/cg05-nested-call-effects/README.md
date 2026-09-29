# CG05-NESTED-CALL-EFFECTS: nested and sibling call effects

Card CG05 of the [Clang gap card queue](../../plans/clang-gap-cards/README.md)
(wave 48, round 4; base `27dc7c9a` = local and origin `main` at dispatch).

- **Gap entries:** E10, E11 and E12, on route **G** (the custody gateway).
- **Evidence:** Q03, Q14, Q19, Q20 and example B.
- **Rule:** [gateway 10y](../../agents/gateway.md#constraint-10y); rule 10u's
  passed row amended.
- **Lean:** `QCore.NestedCallEffects`, `proof/AuditNestedCallEffects.lean`
  (27 theorems); `QCore.CallInstantiation`'s passed row.
- **Prediction:** frozen in [PREDICTION.md](PREDICTION.md) (commit `b13ec691`),
  before the implementation commit and before any AFTER run; fixture probes of
  the new routes had run.

## 1. What changed

### 1.1 The reading (`nested_call_effects.rs`)

`read(call, facts, use)` reads a call whose actuals hold calls into a call
tree, at any depth. Each actual is one of three forms:

| Form | What it is | How it is applied |
|---|---|---|
| plain | no evaluated call | the classifier substitutes it, as in a statement |
| bound | the whole actual is one pointer-valued call, through parentheses, pointer `NoOp`/`BitCast` and a `_Generic` selection's selected association | `__qcore_nested_value_N = g(…)`, use `Passed`; the outer actual is the reserved place, with the inner result's identity and alias state |
| data | a scalar value holding calls (`f(g(p) + 1)`, a scalar `g(p)`) | only at a `Borrowed` formal or a declared data tail; read by CG03's data reading with each call standing for a scalar; each call `Passed` when its value is the whole actual, `Tested` when an operator consumes it; the outer actual is `0` |

`read_data(value, facts)` reads a scalar value holding calls that is not one
call (`n = strlen(a) + strlen(b)`, `return h(p) * k`).

**The order (E12).** The reading builds CG02's `EvalGraph`. Each call is an
event sequenced after every event of its actuals (C11 6.5.2.2p10); every other
pair is unordered. Every event carries a contract footprint, and
`EvalGraph::check_footprints` (new, additive) admits an unordered pair
involving a call only when the footprints commute:

- An identity at a `Borrowed` formal is read. At any other formal it is
  written (a stored or removed resource also writes its owner).
- `&x` of a non-scalar object is a write of `x`'s storage.
- An owned result is a written **fresh** identity.
- A read of a pointer object reads its storage and its identity. A scalar read
  is custody-neutral.

Overlap (`cells_overlap`) is sound, not precise:

- two identity cells always overlap (names may alias; identity disjointness is
  **not** proved);
- an identity reaches a named object only if its address is taken;
- a fresh identity overlaps only itself.

No order C leaves unspecified is chosen. The route lowers in the graph's event
order only because an admitted graph reaches one state in every permitted
order.

### 1.2 The consumers (`legacy_nested_call.rs`)

Each is asked last, after every older route of its position (hard invariant 3):

- **Statement** (after CG03's typed substitution in `expression_statement`),
  only for a statement the classifier refused with O04-B09B10's `nested call
  … inside a modeled call` or with `resource call with a formal embedded in an
  unmodeled expression`. Shapes:
  - `CALL(…)`, use `Discarded`;
  - `NAME = CALL(…)` or a one-declarator declaration of one, use `Stored`;
  - `NAME = VALUE` / a declaration with a scalar value holding calls: the
    calls, then `ForgetScalar NAME` (the elaborator's scalar local write, with
    its guard).
- **`return`** (after CG03's return operand), for a refused return or one W43
  lowered to the nested refusal:
  - a whole call with nested actuals: C-GATEWAY-RETURN-CALL-W4's desugaring,
    `__qcore_returned_value = CALL(…)`, use `Returned`, the outer call the
    return's escaping evaluation;
  - in a scalar-result function, a scalar value holding calls: the calls, then
    an exit that escapes nothing.

Every call goes through `Parser::instantiate_call` inside-out. Each
application must apply exactly the contract its footprint was read from, else
the statement refuses. The statement's reserved places are cleared after it,
so an owned value the outer call only borrowed stays owned by the body and is
reported as a leak. The Clang node is matched by line and exact tokens. Only
where W44-C11-GENERIC-SELECTION rewrote an invariant `_Generic` to its core is
it matched by line, destination and outer callee instead. In both cases every
call's callee and arity and every plain actual's tokens are checked against
the statement's own tokens (`locate`). Declines: descriptor names, list
frames, opaque callees, braces, and a body that spells the reserved prefix.
Every consultation is a parity row (`position: "nested-call"`).

### 1.3 Shared items added (for the integrator)

- **`elaboration.rs`** (additive only):
  - `Cell` (`Object`, `Identity`, `Fresh`), `Footprint`;
  - `cells_overlap`, `footprints_commute`;
  - `EvalGraph::check_footprints` (`check` is unchanged, and so is its caller
    `elaborate`);
  - `Refusal::CallFootprintConflict`.
- **`call_instantiation.rs`:**
  - the `Passed` row of `accept`: B04's identity row first, unchanged, then
    any result but a reallocation (`Decline::PassedReallocation`, new);
  - `Decline::NotIdentity` is kept in the vocabulary and no decision reaches it
    any more;
  - `ACCEPTANCE_VECTORS` gains three rows and changes two.
  - **CG04 widens `Tested` in the same function; the two edits are in
    different arms.**
- **`typed_place.rs`:** unchanged.
- **`legacy_elaboration.rs`:** `aliasing` is `pub(super)`.
- **`legacy_c.rs`:** the module declaration and two hooks (the last step of
  `expression_statement`; after `instantiated_return` in the `return` arm).
- **`lib.rs`:** `mod nested_call_effects`.
- **Test fixtures:** `cg_pair(void *p, int value)` added to
  `tests/fixtures/contracts/clang_gap.qv` and to `support/clang_gap.rs`'s
  declarations.

## 2. The cited claims

| Claim | Reproduced? |
|---|---|
| E10: non-identity nested effects are refused (`sink(release_and_status(p))`) | **Yes.** CG00's executed controls held on the base (`e10_non_identity_effects_of_a_nested_call_are_refused`). |
| E11: depth > 1 and calls in part of an argument are refused | **Yes** for the shapes CG00 pinned (`f(g(h(p)))`, `f(g(p) + 1)`). |
| E12: G restricts sibling calls to identity effects | **Yes** (`e12_g_sibling_calls_compose_only_as_identity_effects`). |
| CG02: 98 elaborator consultations refused as an unordered or nested call, mostly `P = f(...)` with `P`'s read unsequenced with the call | **Yes, exactly:** 98 de-duplicated `elaboration-*` rows at the base, all `call … unsequenced with another event (CG05)`, over 33 functions, and all a compound assignment or store of a call (`len += f(acl)`, `x \|= g(r)`). **Not closed by this card:** the footprint check is applied to the nested-call consumers, not yet to the elaborator's own call events, and the classifier refuses those statements with another refusal, so rule 10y is not asked. They are still 98 at the tip (`CG05-ELABORATOR-CALL-FOOTPRINTS`). |
| CG00: 405 leaves over 269 Unknown functions; E10 130, E11 275 | **Correction.** E11's `path:call` reason (223 leaves) is a *call-rooted place*, not a call occupying part of an argument: `errno` (`(*__errno_location ())`), the ctype macros (`(*__ctype_b_loc ())[c]`), `f(r)->m`, `a[f(i)]`. The lexical indirect-access gate (180), a condition (39) or a `return` (4) refuses them before any nested-call rule. All 223 are still present at the tip, over 157 functions (`CG05-CALL-ROOTED-PLACES`). The nested-call family proper is 177 leaves over 96 functions at the base. |
| CG00: 23 functions freed alone | **Correction:** 22 of them have a second cause CG00 did not label. 15 are call-rooted places. 6 have an inner callee with no contract in force: the three `cJSON_DeleteItemFrom*`, `rd_kafka_srand`, `rd_kafka_error_to_legacy`, `rd_kafka_topic_partition_cmp_topic_id`. `cio_file_native_compose_path` keeps two `errno` places. Only `snappy.c::hash` was freeable, and it moved. |
| W44-PRINTF-NESTED-INNER-CALLEES: 87 printf-family nested refusals over 60 functions | **Yes, exactly:** 87 over 60 on the base's statement consultations. |

## 3. Measurements

### 3.1 On the merged base (main `c0ccc66a`): the published figures

W48-QR-SCALAR-TYPES-R4 and then CG04-CONDITION-ACQUISITION landed while the
card ran. Main was merged twice (`524d08f9` at `6883623c`, then `32635478` at
`c0ccc66a`), and the corpus was retaken on the final merge.

- **BEFORE:** the release binary built at `c0ccc66a`
  (`private/cg05/quirin-audit-main3`, `fd3b7932…`); run `private/cg05/before3`,
  exit 0. It reproduces CG04's AFTER byte for byte (`24571074…`,
  **565 / 10 / 4,259**).
- **AFTER:** the release binary built at the merge `32635478`
  (`quirin-audit-after3`, `209485a4…`); run `private/cg05/after3`, exit 0:
  **567 / 10 / 4,257** (`8642ec02…`).
  [corpus-comparison-merged.json](artifacts/corpus-comparison-merged.json).
- The moves (`snappy.c::hash`, `lz4hc.c::LZ4HC_hashPtr`), the 15 functions
  whose diagnostics change and the leaf deltas are **identical** to §3.2's.
  Nothing composes differently with CG04's condition lowering.
- System: BEFORE 33 / 2 / 199 (`42259780…`, byte-identical to the base's), AFTER **33 / 2 / 199** (`32f9a5aa…`, byte-identical to the base's AFTER): the same two leaves go.
- Tail-pause slice: **26 / 1 / 242 / 1 Pending**, BEFORE and AFTER byte-identical (`fc9a6e0b…`).

### 3.2 At the card's base `27dc7c9a`

**BEFORE.**

- Binary: the release `quirin-audit` built at `27dc7c9a`
  (`private/cg05/quirin-audit-before`, `d305eb08…`).
- Corpus `private/cg05/before`: exit 0, `resource-diagnostics.json`
  `7393fd21…`, CG03's archived AFTER byte for byte: **564 / 10 / 4,260**.
- System `private/cg05/before-system`: **33 / 2 / 199**. The report embeds the
  worktree path, so its hash (`42259780…`) is this worktree's.
- Tail-pause slice `…/w47-tail/attempt-1/cg05-before`:
  **26 / 1 / 242 / 1 Pending** (`fc9a6e0b…`).

**AFTER.**

- Binary: the tip's release binary built at `6787ea86`
  (`private/cg05/quirin-audit-after1`, `52041842…`). Later commits change
  tests, Lean and records only.
- Runs: `private/cg05/after1`, `after1-system`, `…/cg05-after1`; all exit 0.
- Comparisons: [corpus-comparison.json](artifacts/corpus-comparison.json),
  [corpus-changed.json](artifacts/corpus-changed.json),
  [system-comparison.json](artifacts/system-comparison.json),
  [slice-comparison.json](artifacts/slice-comparison.json).

| Route | BEFORE | AFTER | Moves |
|---|---|---|---|
| corpus (`66910c10`) | 564 / 10 / 4,260 | **566 / 10 / 4,258** (`3b38d897…`) | **+2 Unknown → Safe**; 15 more Unknown functions change diagnostics; no leaf added |
| system (`046accf6`) | 33 / 2 / 199 | **33 / 2 / 199** (`32f9a5aa…`) | no status move; 2 functions lose one leaf each (below) |
| tail-pause slice (`90978ac6`) | 26 / 1 / 242 / 1 Pending | **26 / 1 / 242 / 1 Pending** (`fc9a6e0b…`) | **byte-identical** |

In the system route:

- `flb_scheduler.c::backoff_full_jitter` loses `nested call ipow`: `temp =
  xmin(cap, base * ipow(2, n))` is a scalar store of calls.
- `flb_output.c::flb_output_upstream_set` loses one `indirect memory access`
  leaf, as a cascade. The header function `flb_stream_enable_flags`, whose one
  statement is `flb_stream_set_flags(stream, flb_stream_get_flags(stream) |
  flag)`, is now composed (`flb_stream_get_flags` `Tested`). Its validated
  summary then lets W44's call-argument route admit
  `flb_stream_enable_flags(&u->base, flags)` at `flb_output.c:1994`.

The corpus sidecars (`monkey-source.json`, `*.contracts.json`,
`*.chunk-contracts.json`, `*.source-effects.json`) are byte-identical.

**The two moves, each attributed by reading.**

- **`snappy.c::hash`**. `return hash_bytes(get_unaligned32((u32 *)(p)),
  shift);` is a whole-call return. Its inner `get_unaligned32` has a validated
  same-unit summary, and its value is a whole scalar actual (`Passed`).
  `hash_bytes` is `Returned`, with no result custody.
- **`lz4hc.c::LZ4HC_hashPtr`**. `return ((LZ4_read32(ptr)) * 2654435761U) >>
  ((4 * 8) - 15);` is a scalar value holding one call in a scalar-result
  function (`LZ4_read32` `Tested`). CG03 predicted it and missed it, because
  the value is not a condition leaf.

**Leaf deltas** (corpus; every one a removal, no leaf added):

- −7 `nested call strlen`, −3 `get_unaligned32`, −1 `get_unaligned64`,
  −1 `rd_list_cnt`: 11 statement consultations admitted. Nine are scalar
  stores of `strlen` sums or differences; there are also `strncpy(…,
  strlen(tmp) - 3)` and `put_unaligned32/64(get_unaligned…(…), …)`.
- −6 `resource call with a formal embedded in an unmodeled expression`:
  `format_context_common` ×3 (`tlen = strlen(context->index) + 12`), two
  `is_consumer_protocol_type = !strcmp(…)`, and `strsz = vsnprintf(NULL, 0,
  fmt, ap2) + 1`.
- −2 return refusals: the two moves.
- −1 `call to uncontracted function hash`, −1 `…LZ4HC_hashPtr`: the callers
  `compress_fragment` and `LZ4HC_Insert` now see validated summaries. Both
  stay Unknown on other leaves.
- −1 `unknown_alias` (`create_jws_compact_serialization`).

**No correction.**

- No Safe → non-Safe move and no Unknown → Unsafe move.
- No leak or violation was added: no admitted consultation in the corpus hands
  an owned inner result to a borrowing outer call.
- The leak such a call would report is exercised by the fixtures
  (`q_observe(r_wrap(p))`). It is an accusation over synthetic contracts,
  never an upstream defect.

**Parity** (de-duplicated `position: nested-call` rows,
[followups.json](artifacts/followups.json)):

- **Statement:** 197 consultations over 108 functions, **17 admitted**. The
  refusals:
  - 128 an uncontracted inner callee;
  - 25 footprints that do not commute (24 `fprintf(stderr, …, f(&local))`,
    1 `localtime_r`);
  - 7 an unmatched Clang node;
  - 6 not a call or a scalar value;
  - 6 a store to a non-scalar-local object;
  - 4 a data actual refused;
  - 2 a contract rewritten by the classifier;
  - 1 unapplied;
  - 1 with no nested call.
- **Return:** 113 returns holding calls over 85 functions, **3 admitted**
  (the two moves and `compare_double`). The refusals: 35 a data value the
  reading refuses, 35 an uncontracted callee, 15 a whole call with no
  nesting, 11 outside a scalar-result function, 8 an indirect callee, 5
  unapplied, 1 unmatched node.
- **Family sizes** (`scripts/families.py`):
  - nested-call leaves 177 / 96 → **165 / 89**;
  - formal-embedded 18 / 13 → **12 / 9**;
  - return 253 / 201 → **251 / 199**.

## 4. Prediction against outcome

| Prediction ([PREDICTION.md](PREDICTION.md)) | Outcome |
|---|---|
| 1. BEFORE reproduces the baseline | **Held** (`7393fd21…`). |
| 2. At most 5 Safe moves; of CG00's 23 at most `snappy.c::hash`; at most 4 from the formal-embedded and return families | **Held:** 2 moves, `hash` and `LZ4HC_hashPtr` (return family). |
| CG00's other 22 stay Unknown, with the blockers read | **Held**, each for the blocker the prediction named. |
| 3. No Safe → non-Safe | **Held.** |
| 4. Added leaks possible, each classified | **None added.** |
| 5. Leaf exchanges | **None:** every admitted consultation removed its leaf, and no later refusal surfaced. |
| 6. Sidecars byte-identical; system and slice unmoved | **Held.** Sidecars are byte-identical. The system moves no status (2 leaves removed). The slice is byte-identical. |

## 5. Tests and flips

**New controls.**

- `tests/cg05_nested_call_effects.rs`, 9 tests. Each admission is paired
  with its fail-closed controls. They cover:
  - example B;
  - an owned inner result released by the outer call, and one only borrowed
    (a leak);
  - depth 2 and 3;
  - partial data;
  - scalar stores;
  - whole-call and data returns;
  - siblings with fresh results (admitted) and the refusal controls;
  - a nested owned result in a loop. Its reserved place is cleared each
    iteration, so an owned value the outer call does not take is reported
    and never hidden behind a later generation of the same site
    (`CG04-LOOP-SITE-GENERATIONS`).
- `tests/unit/nested_call_effects.rs`: the overlap vectors both ways, the
  order vectors read by Clang, the refusal reasons, inside-out order and
  sequencing, and the two Lean bindings.

**Refusal controls required by the brief.**

- `f(free_and_status(p), use(p))`: `cg_sink2(r_release_code(p),
  r_borrow_code(p))` and its mirror stay refused in both argument orders
  (`CallFootprintConflict`).
- `h(p, g(p))` with `g` releasing `p`: `cg_pair(p, r_release_code(p))` stays
  refused.

**Flipped deliberately.**

- CG00's controls (`clang_gap_phase1.rs`):
  - `e10_…_are_refused` → `e10_non_identity_effects_of_a_nested_call_are_composed`:
    three shapes now Safe;
  - `e11_…_are_refused` → `e11_deeper_and_partial_nested_calls_are_composed`:
    depth two reports the wrapper's leak, and `f(g(p) + 1)` is Safe;
  - `e12_g_sibling_calls_compose_only_as_identity_effects` →
    `e12_…_only_when_their_footprints_commute`: the operator initializer is
    now Safe, and the two non-commuting pairs stay refused;
  - `k12_one_contract_is_accepted_in_one_position_and_refused_in_another`:
    `q_observe(r_wrap(p))` now reports the leak instead of refusing.
- Re-pinned in O04's tests:
  - `o04_b09b10::a_nested_call_inside_a_modeled_call_is_never_admitted`;
  - `o04_trace::a_nested_call_producing_custody_is_never_admitted`;
  - `o04_trace::two_level_nesting_is_never_admitted`.

  Each still asserts non-Safe, and now accepts the nested refusal **or** the
  leak. The acquisition is never dropped.
- `o04_trace::a_partial_nested_call_expression_is_never_admitted`: its first
  body (`v_log("%d", v_size(b) + 1)`) is now Safe. Its custody-position body
  stays refused.
- `generic_selection`'s invariance tests hold. The glibc `_Generic` spelling
  and the bare call are decided alike: both are admitted.

**§8 axes.**

- **Position:** statement, scalar store, whole-call and scalar return.
  Conditions stay CG04's.
- **Effect:** Owned, Released, Consumed and Out are applied through the one
  application point. A reallocated result, a container effect and an owned
  scalar result are refused.
- **Depth:** any.
- **Evaluation order:** only footprint-commuting unordered pairs; no order is
  chosen. The elaborator's own compound assignments of a call stay refused
  (`CG05-ELABORATOR-CALL-FOOTPRINTS`).
- **Place:** plain actuals only, through the classifier. A place inside a data
  value refuses; a call-rooted place is `CG05-CALL-ROOTED-PLACES`.
- **Boundary:** contracts in force only.
- **Verdict:** no Safe → non-Safe.

## 6. Lean

`QCore.NestedCallEffects`, `proof/AuditNestedCallEffects.lean`, **27
theorems**, no placeholders and no project axioms.
`check_lean_audits.py` passes **234** audits (233 on main + this one).

- **Cells:**
  - `overlap_vectors_hold` (bound to `OVERLAP_VECTORS`);
  - `overlap_symm`;
  - `overlap_sound`: sound for any denotation meeting the stated premises
    (distinct declarations, fresh sites, unaliased automatics).
- **Commutation:**
  - `footprint_commute_sound`: effects that respect their footprints and whose
    footprints commute commute on every state;
  - `no_clash`, `disjoint_of_no_clash`;
  - `overlapping_writes_do_not_commute` with `store_respects`: the refusal is
    necessary.
- **Every interleaving:** `interleaving_runs_as_append`, `step_through`,
  `run_append`, `interleave_mem`, `interleave_append`.
- **Inside-out, depth n** (by induction over the call tree `Tm`):
  - `inner_effects_precede_the_outer_entry`;
  - `post_is_permitted`, `permitted_mem`;
  - `permitted_runs_as_post`;
  - `check_admits`;
  - `every_interleaving_runs_alike`: a tree the footprint check admits reaches
    the lowering order's state in every order C permits.
- **Refusals:**
  - `a_non_commuting_pair_is_refused`;
  - the brief's controls, `a_release_and_a_borrow_of_one_identity_are_refused`
    and `a_read_unordered_with_its_release_is_refused`;
  - `two_releases_are_refused`, `fresh_results_from_borrowed_reads_commute`;
  - `only_listed_forms_are_admitted`;
  - `order_vectors_count` (the vectors bound to `ORDER_VECTORS`).
- **`QCore.CallInstantiation`:** the passed row is widened.
  `a_passed_value_is_the_identity` is replaced by
  `a_passed_value_is_not_a_reallocation` and `every_b04_admission_stands`, and
  `acceptanceVectors` is extended. The audit counts 27 there.

**What is premise and what is model.** These are stated, not proved:

- faithful Clang extraction, the node match and the token check;
- that a call's effect on the abstract state is its contract's (constraint
  4), as for a call standing alone;
- the denotation premises of `overlap_sound`.

The clauses themselves are applied by the unchanged `apply_contract_call`.

## 7. Follow-ups

Ledger 629 → 630 ([scripts/update_ledger.py](scripts/update_ledger.py)).

- **Retired:**
  - `CG05-NESTED-CALL-EFFECTS`: every DISCHARGE clause holds.
  - `NESTED-CALL-IN-ARGUMENT-POSITION` and `CJSON-DELETE-NESTED-DETACH`
    (absorbed): the positional refusal is answered; their remainder is carried
    below with re-measured sizes.
- **Resized:**
  - `W44-PRINTF-NESTED-INNER-CALLEES`: 87 / 60 reproduced; 61 uncontracted
    inner, 24 footprint conservatism, 2 other; N4 is decided.
  - `W43-RETURN-COMPOUND-CALL`: 113 returns holding calls over 85 functions,
    3 admitted, the rest by reason.
- **Registered:**
  - `CG05-UNCONTRACTED-INNER-CALLEES`: 128 statement and 35 return
    consultations; 165 nested leaves over 89 functions.
  - `CG05-FOOTPRINT-ADDRESS-CONSERVATISM`: 25 consultations.
  - `CG05-CALL-ROOTED-PLACES`: 223 leaves over 157 functions (the correction
    above).
  - `CG05-ELABORATOR-CALL-FOOTPRINTS`: CG02's 98 compound-assignment
    consultations over 33 functions. The elaborator's own call events do not
    yet carry footprints.

## 8. Gate

The final gate ran step by step at tip `9d5cde95` (main `c0ccc66a` merged),
using the commands of [scripts/gate.sh](scripts/gate.sh); this session's
sandbox refuses a script launch. The exits are in [gate/](gate/), the record is
[gate.json](gate.json), and the logs are under `private/cg05/gate2/`.

| Step | Command | Exit | Count |
|---|---|---|---|
| submodule | `git submodule update --init third_party/libevent` | 0 | the `--recursive` form was refused by the sandbox; libevent is the only submodule (`d82464a2`) |
| tests | `python3 scripts/run_test_binaries.py` | 0 | 376 units, 389 summaries, **6,703 passed**, 0 failed, 11 ignored (`private/test-binaries/20260929T193830Z-2080891/`) |
| fmt | `cargo fmt --all -- --check` | 0 | |
| clippy | `cargo clippy --all-targets -- -D warnings` | 0 | |
| probe | `cargo run -p qcore-compiler --example reevaluation_probe` | 0 | |
| follow-up probe | `cargo run -p qcore-compiler --example reevaluation_followup` | 0 | |
| Lean | `python3 scripts/check_lean_audits.py` | 0 | **236 audits** (235 on main + `AuditNestedCallEffects.lean`) |
| Python | `python3 -m unittest discover -s scripts/tests` | 0 | 365 tests OK, after the matrix validation was regenerated from `gate.json` (`remaining-d02/build_matrix.py --gate`) |
| docs index | `python3 scripts/build_docs_index.py` | 0 | |

The first card gate ran at `cd72cdca`, before the merges (`private/cg05/gate/`,
its test exits kept as `gate/first-gate-*`). It passed the same steps: 6,667 tests on a
rerun, Lean 234, Python 365. The test runner's first run there failed one
test, `qcore-audit` `checkpoint_cli::interrupted_run_preserves_completed_units_for_resume`.
That test has a 30 s wall-clock deadline and ran under load; alone it passes
in 7.8 s (`gate/first-gate-03b-checkpoint-rerun.exit`). `providers/` is unchanged, so the provider gate is not
due. Clang 21.1.8 is recorded in `gate/clang-version.txt`.

## 9. Scripts

| Script | What it does |
|---|---|
| [measure.sh](scripts/measure.sh) | The corpus, system and slice commands (run here step by step; see §3). |
| [compare.py](scripts/compare.py), [changed.py](scripts/changed.py), [routes.py](scripts/routes.py) | Status moves and leaf deltas (from CG03). |
| [census.py](scripts/census.py) | The nested-call leaf census and the parity outcome counts. |
| [families.py](scripts/families.py) | The three families the consumers are asked after. |
| [rederive.py](scripts/rederive.py) | CG00's CG05 function list on a run. |
| [followups.py](scripts/followups.py) | The absorbed follow-ups' re-measurement from the parity rows. |
| [show_origins.py](scripts/show_origins.py) | The pinned source line of an origin. |
| [update_ledger.py](scripts/update_ledger.py) | The ledger edits. |
| [gate.sh](scripts/gate.sh) | The standing gate. |
