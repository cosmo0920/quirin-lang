# CG00-CLANG-GAP-BASELINE: executed BEFORE controls for the Clang gap entries, and their corpus attribution

Card CG00 of the [Clang gap card queue](../../plans/clang-gap-cards/README.md),
wave 48 round 1, Medium. Base `d3508e10`; `main` moved to `7e7ba401`
(CG01-T-OPERATOR-CLOSURE landed) during the card and was merged in, so the
tip carries CG01. The input is the archived, **static** gap analysis
([supplied/](../../reviews/clang-gap-2026-09-29/supplied/); it ran no Rust
test). This card turns its claims into executed tests and ranks the
implementation cards by the corpus functions they would free.

**Records only.** No analyzer, contract, provider or Lean change; the only
code added is tests, one test-fixture contract file and scripts. No Lean
module: the card adds no semantics. The prediction was frozen before any test
or script ([PREDICTION.md](PREDICTION.md)).

## Result

- **All 98 carded entries have an executed test** (97) or a named reading
  justification with a pinning test (W06). 106 new tests in six files, all
  passing at the tip:

  | File | Route | Tests |
  |---|---|---:|
  | `crates/qcore-c-gateway/tests/clang_gap_phase1.rs` | G, T (CG01–CG09) | 29 |
  | `crates/qcore-c-gateway/tests/clang_gap_phase2.rs` | G, T (CG10–CG15) | 14 |
  | `crates/qcore-c-gateway/tests/clang_gap_phase3.rs` | G, T (CG16–CG19) | 15 |
  | `crates/qcore-c-gateway/tests/clang_gap_phase4.rs` | G, T, contract loader (CG20–CG30) | 33 |
  | `crates/qcore-core/tests/clang_gap_core.rs` | core models | 11 |
  | `crates/qcore-audit/tests/clang_gap_flags.rs` | compile-database reader | 4 |

  Shared harness: `crates/qcore-c-gateway/tests/support/clang_gap.rs`
  (`g` runs `legacy_c::analyze_unrestricted_c11`, `t` runs
  `typed_source::import_source_functions`); fixture contracts
  `tests/fixtures/contracts/clang_gap.qv` (`cg_sink`, `cg_sink2`, and `cg_log`
  with a data tail), synthetic trusted assumptions.
- **Reproduction.** 84 entries reproduce as stated (83 by test, W06 by
  reading; 3 of them with a clarification of scope: C04, C05, I06); **14 are
  corrections** to the input (§2), of which 10 are partial (the refusal holds
  for some of the cited forms but not others) and 4 are a refusal for a
  different named reason.
- **Corpus attribution.** All 35,737 unsupported leaves of the arbiter
  (4,298 Unknown functions) are classified: 25,913 to an entry, 9,824
  `unattributed` (8,643 of them uncontracted calls: callee closure 6,737,
  external 1,906). Every Unknown function is in exactly one partition class
  (§3). **CG10 frees 491 functions alone** (293 with no alias leaf), ten
  times the next card; the ranking is in §3.3.
- **Corpus identity.** BEFORE (base binary) and AFTER (tip binary, CG01
  merged) are both byte-identical to the baseline `ae17152b…`, 526 / 10 /
  4,298.

## 1. The executed controls

Each test is named after its entry and asserts the route's **current**
result: the refusal description, the Unknown/Unsafe status or the limit.
Entries that are **limits** rather than refusals (the gateway publishes
custody-`Safe` with no extent, alignment, arithmetic or ordering obligation,
because none is in the custody guarantee) are pinned as that `Safe`, so the
card that adds the obligation moves it deliberately. §8 axes covered where
they distinguish behaviour: position (statement, initializer, actual,
condition, loop condition, return), effect (borrowed, released, consumed,
owned, out, derived), place (local, formal, field, embedded field, element,
global, thread-local), type (scalar, pointer, `void *`, union, bit-field,
volatile, atomic, complex, vector, `_BitInt`) and evaluation (short-circuit
unevaluated side, sibling calls, one-line identity).

| Entry | Test | Outcome |
|---|---|---|
| E01 | `e01_t_legacy_consumer_refuses_six_binary_operators_by_name` | reproduced at base; after CG01 the importer reads them and the legacy consumer refuses each by name; G reads all six |
| E02 | `e02_t_legacy_consumer_refuses_unary_plus_and_complement_by_name` | as E01 |
| E03 | `e03_t_legacy_consumer_admits_only_int_or_assign`, `e03_g_scalar_compound_assignments_are_partial` | T as E01; **G correction** |
| E04 | `e04_assignment_as_a_value_is_refused_on_both_routes` | reproduced |
| E05 | `e05_value_producing_increments_are_refused` | reproduced |
| E06 | `e06_short_circuit_values_are_refused_outside_conditions` | reproduced |
| E07 | `e07_conditional_values_are_refused_outside_conditions` | reproduced (a scalar GNU `?:` initializer and a condition `?:` are admitted, as the entry says) |
| E08 | `e08_comma_is_refused_in_value_positions` | reproduced |
| E09 | `e09_an_owned_acquisition_in_a_condition_is_refused_and_not_hoisted` | reproduced (heap, nullable, loop, descriptor) |
| E10 | `e10_non_identity_effects_of_a_nested_call_are_refused` | reproduced (example B) |
| E11 | `e11_deeper_and_partial_nested_calls_are_refused` | reproduced |
| E12 | `e12_t_refuses_unsequenced_calls_and_reads`, `e12_g_sibling_calls_compose_only_as_identity_effects` | reproduced |
| E13 | `e13_condition_leaf_composition_is_partial` | **correction (partial)** |
| E14 | `e14_t_legacy_consumer_refuses_floating_and_character_literals_by_name`, `e14_g_reads_literals_except_a_character_initializer` | T as E01; **G correction** |
| E15 | `e15_t_cast_kinds_outside_the_six_are_refused` | reproduced |
| E16 | `e16_type_queries_are_read_only_in_fixed_shapes` | **correction (partial)** |
| E17 | `e17_only_selection_invariant_generics_are_read` | reproduced (G spells the refusal `call to uncontracted function \`_Generic\``) |
| E18 | `e18_offsetof_and_container_of_are_refused` | reproduced |
| E19 | `e19_statement_expressions_are_refused` | reproduced |
| E20 | `e20_builtins_are_ordinary_uncontracted_calls` | reproduced |
| M01 | `m01_mixed_paths_need_a_never_bound_root` | reproduced |
| M02 | `m02_general_pointer_valued_stores_are_refused` | **correction (partial)** |
| M03 | `m03_field_stores_through_formals_and_local_owners_are_refused` | reproduced |
| M04 | `m04_nested_array_and_function_pointer_stores_are_refused` | reproduced |
| M05 | `m05_readback_from_a_void_field_is_refused` | reproduced |
| M06 | `m06_union_stores_are_refused_and_reads_are_neutral`, `core_footprint_refuses_every_listed_construct` | **correction (partial)**; core reproduced |
| M07 | `m07_bitfield_stores_are_refused_and_reads_are_neutral`, `m07_core_keeps_one_bitfield_unit_one_memory_location` | **correction (partial)**; core reproduced (example E) |
| M08 | `m08_aggregate_copies_are_refused` | reproduced |
| M09 | `m09_braced_initializers_are_refused` | reproduced |
| M10 | `m10_compound_literals_have_no_identity` | reproduced |
| M11 | `m11_string_initializers_are_refused_or_unresolved` | **correction (partial)** |
| M12 | `m12_variable_length_arrays_are_refused`, core | **correction (reason)** |
| M13 | `m13_flexible_array_members_carry_no_extent`, core | reproduced (limit) |
| M14 | `m14_computed_indices_carry_no_bounds_obligation`, core | reproduced (limit) |
| M15 | `m15_pointer_arithmetic_carries_no_offset`, core | reproduced (limit) |
| M16 | `m16_pointer_difference_and_ordering_carry_no_same_object_obligation` | reproduced (limit) |
| M17 | `m17_one_past_formation_is_not_separated_from_dereference` | reproduced (limit) |
| M18 | `m18_pointer_integer_round_trips_are_not_modelled` | reproduced |
| M19 | `m19_derived_addresses_carry_no_alignment_obligation` | reproduced (limit) |
| M20 | `m20_stale_aliases_are_unresolved_not_generation_checked`, `m20_core_retires_handles_of_a_reallocated_generation` | reproduced |
| M21 | `m21_cross_allocation_accesses_need_no_disjointness_for_custody`, `m21_core_refuses_disjointness_across_allocations` | reproduced |
| M22 | `m22_type_punning_and_restrict`, core | reproduced |
| M23 | `m23_volatile_accesses_are_not_counted_or_ordered` | reproduced |
| M24 | `m24_static_storage_is_refused_or_unresolved` | reproduced |
| M26 | `m26_memcpy_byte_semantics_are_a_contract_boundary`, core | reproduced |
| N01–N03 | `n01_n02_n03_t_keeps_conversions_as_named_casts_only`, `n03_core_conversion_never_truncates` | reproduced |
| N04–N07 | `n04_n05_n06_n07_g_proves_no_arithmetic_property`, `n04_core_arithmetic_refuses_where_c_wraps`, `n05_core_division_faults_are_typed_not_obligations`, `n06_core_has_no_shift_operation` | reproduced (limit) |
| N08 | `n08_floating_conversions_have_no_representation_in_t`, `n08_floating_transfer_is_not_modelled` | reproduced |
| N11 | `n11_type_queries_carry_no_evaluated_value` | reproduced |
| N12 | `n12_complex_values_are_refused` | reproduced |
| N13 | `n13_vector_values_are_refused` | reproduced |
| N14 | `n14_bit_precise_integers_are_refused` | **correction (reason)** |
| C01 | `c01_t_has_no_general_loops` | reproduced |
| C02 | `c02_t_has_no_switch` | reproduced |
| C03 | `c03_t_has_no_jumps` | reproduced |
| C04 | `c04_switch_values_do_not_prune_cases` | reproduced; clarification: a provisional Unsafe |
| C05 | `c05_branch_correlation_is_not_tracked` | reproduced; clarification: a provisional Unsafe |
| C06 | `c06_collection_loops_have_no_invariant` | reproduced |
| C07 | `c07_recursion_passing_a_resource_has_no_summary` | **correction (partial)** |
| C08 | `c08_budgets_are_reported_never_safe` | reproduced (T path limit; G alias widening at 8) |
| C09 | `c09_computed_goto_is_refused` | reproduced (G does not project the function) |
| C10 | `c10_setjmp_has_no_return_twice_edge` | reproduced |
| C11 | `c11_cleanup_attributes_are_refused` | **correction (reason)** |
| C12 | `c12_inline_asm_is_refused_as_a_call` | **correction (reason)** |
| C13 | `c13_noreturn_is_read_only_as_a_terminal_statement` | reproduced |
| K01 | `k01_indirect_callees_are_not_resolved` | reproduced |
| K02 | `k02_callback_registration_has_no_capture_relation` | reproduced |
| K03 | `k03_complex_actual_places_are_not_substituted` | reproduced |
| K04 | `k04_no_read_write_frame_clause_exists` | reproduced (loader refuses) |
| K05 | `k05_no_relational_postcondition_exists` | reproduced (loader refuses) |
| K06 | `k06_derived_results_have_no_offset_or_length` | reproduced (loader refuses; exhaustive schema pins) |
| K07 | `k07_resource_kinds_are_exactly_four` | reproduced |
| K08 | `k08_variadic_tails_are_data_only` | reproduced |
| K09 | `k09_va_list_operations_are_not_modelled` | reproduced |
| K10 | `k10_format_agreement_is_not_checked` | reproduced (limit) |
| K11 | `k11_an_exit_specific_disposition_has_no_summary` | reproduced |
| K12 | `k12_one_contract_is_accepted_in_one_position_and_refused_in_another` | reproduced |
| W01 | `w01_atomic_accesses_do_not_reach_the_atomic_model` | reproduced (G reads an `_Atomic` load as a neutral scalar) |
| W02–W04 | `w02_w03_w04_atomic_rmw_cas_and_fences_are_uncontracted_calls`, `w02_w03_w04_core_has_no_rmw_cas_or_fence_event` | reproduced |
| W05 | `w05_w06_core_has_no_consume_order` | reproduced |
| W06 | same pin, plus **reading**: `crates/qcore-core/src/weak_memory.rs:5-8` ("plain loads/stores and same-thread store release sequences") | scope statement, verified by reading |
| W07 | `w07_core_accepts_one_graph_not_the_program` | reproduced (one relaxed flag program: one graph accepted, another a data race) |
| W08 | `w08_field_updates_carry_no_lock_invariant` | reproduced (limit) |
| W09 | `w09_general_publication_is_not_derived_from_source` | reproduced |
| W10 | `core_footprint_refuses_every_listed_construct` (`AtomicReclamation`) | reproduced |
| I01 | `i01_the_dialect_set_is_c11_and_three_gnu_modes`, `i01_dialects_outside_the_four_are_refused` | reproduced |
| I02 | `i02_target_and_abi_flags_are_refused` | reproduced |
| I03 | `i03_type_profiles_do_not_canonicalise_typedefs` | reproduced |
| I04 | `i04_one_line_identity_refuses_duplicate_candidates` | reproduced |
| I05 | `i05_typeof_is_admitted_by_t_and_refused_by_g` | **correction (partial)** |
| I06 | `i06_extension_pragmas_are_inert_in_the_audited_build`, `i06_extension_flags_are_refused` | reproduced; clarification |

**The post-CG01 state.** CG01 landed while this card ran. Its importer reads
every E01/E02/E03(T)/E14 node, admits them only through
`import_monkey_wrappers`, and the legacy consumer behind
`import_source_functions` refuses each by name (`typed-source node \`binary
-\` is not classified by the legacy typed-source consumer`). The four T
controls were written against the base (where they asserted `unmodeled
callback expression BinaryOperator` and its siblings, which reproduced
exactly) and now pin the post-CG01 state instead.

## 2. Corrections to the input

Each is asserted by the named test; the evidence is the test's assertion.

| Entry | The analysis says | What the route does |
|---|---|---|
| E03 (G) | G "already has scalar compound assignments" | On local scalars 8 of 10 are admitted; `x *= a` and `x &= a` are refused as `unrecognized assignment destination`; every one of the ten is refused (IND) once its right operand reads a field, and a field destination (`s->a += 1`) is refused |
| E14 (G) | G's typed scalar reader has the float/char literal branches | Literals in comparisons and returns are read, but `char c = 'a';` binds the literal as the unresolved identity `__qcore_literal__` (Unknown) |
| E13 | nested connectives, type operators, increments and expression callees are rejections in condition plans | Increments, `sizeof`, expression callees and a field read in a call-bearing leaf are refused; nested connectives, a scalar cast of a call, an assigned scalar call (`(c = f(p)) && c > 1`) and a selection-invariant `_Generic` are **admitted** |
| E16 (G) | G supports `sizeof(*p)` forms | Admitted as a call actual; refused (IND) in an initializer and (RET) in a return |
| M02 | `*slot = p` is not supported | Refused for a formal value (`store through \`*slot\` without an out-parameter contract`), but a fresh acquisition stored through a pointer-to-pointer formal is admitted as an Out hand-off |
| M06 (G) | typed access rejects union owners | Union **stores** are refused; a scalar union member **read** through a never-bound root is admitted custody-neutral |
| M07 (G) | G's typed access rejects bit-fields | Bit-field stores refused; reads admitted custody-neutral |
| M11 | string literals exist; extents are the gap | G refuses a string-initialised local array (`unrecognized assignment destination`), and a pointer to a string literal binds `__qcore_literal__` unresolved |
| M12 (G) | a G Unknown fixture for VLA `sizeof` side effects | A VLA declaration is refused as `local declaration may shadow a formal resource identity` |
| N14 (G) | the builtin scalar table is not Clang's type system | `_BitInt` is refused, through the IND family |
| C07 | not general semantics for multiple ranks | Two recursive calls and mutual recursion over a local resource are admitted; recursion passing a resource has no summary (`call to uncontracted function \`subject\``) |
| C11 (G) | local attributes are rejected | A `cleanup` local is refused through the IND family, not as an attribute |
| C12 (G) | the typed-root reader rejects GCCAsmStmt | In G the asm statement is read as a call to an uncontracted function named `volatile`, `__asm__` or `goto` |
| I05 | no general C23 support | T admits GNU `__typeof__` over a profile type (G refuses it, IND); `nullptr` and `true` are refused by Clang itself in every admitted dialect |

Clarifications (the claim holds, the published result differs from the
framing): C04 and C05's precision limits publish a **provisional Unsafe**
(leak, double release on an infeasible path), not an Unknown; I06's OpenMP
pragma is inert in the audited build because `-fopenmp` is outside the
frontend-flag allowlist, so the body is read as a plain block. None of these
provisional Unsafe results is an upstream defect.

## 3. Corpus attribution

### 3.1 Method

- **Inputs.** The BEFORE arbiter's `resource-diagnostics.json` (byte-identical
  to `ae17152b…`) and one census run of the `indirect-census` build
  (`w47-alias-residuals-census/scripts/run_census.py corpus`, access-path unit
  stream and parity stream on). The census run is inert: its diagnostics,
  chunk contracts and Monkey source are byte-identical to the baseline
  ([identity](artifacts/census-run.identity.json)).
- **No new scanner.** [`scripts/attribute.py`](scripts/attribute.py) imports
  W46-ACCESS-PATH-CENSUS's `access_census.py`: the same Clang argument vector
  over the analysed text, construct walk, access-path decomposition, refusal
  causes and unit join (180 of 180 function-bearing units, 0 Clang errors).
  It adds a `Walker` subclass that also records node kinds, operators, cast
  kinds and type markers, a `cast-from-void` path cause, and a local
  declaration table (VLA, static/thread storage, string and list initialisers,
  `cleanup`).
- **One entry per leaf.** A leaf binds to its function's constructs at its
  line (the census's own kind first). The description family comes first;
  the typed shape decides the entry within it. Access families (IND, COND,
  RET): a VLA declaration, then the first path cause in a fixed order
  (statement expression, union, bit-field, flexible array, pointer-valued
  store split M02/M03/M04, pointer-field step, aggregate value, cast from
  `void *`, other cast, incomplete record, indirect call, call, pointer
  arithmetic, pointer step, static local, address-of base), then construct
  features (`_Complex`, vectors, `_BitInt`, statement expression, `_Generic`,
  `offsetof`, `va_arg`, atomics, compound literal, initializer list, `?:`,
  comma, a value `&&`/`||`, increments, type traits, pointer/integer and
  floating casts), then calls (an uncontracted callee is `unattributed`; an
  assigned call in a condition E09, another call in a condition E13, a
  contracted call in a return K12), then an expressible path: **I04** when its
  line hosts several constructs (the one-line identity), **M01** otherwise
  (the root premise). Uncontracted calls are classified by callee: keywords
  C12, `_Generic` E17, `va_*` K09, atomics W01–W04, `offsetof` E18,
  `setjmp`/`longjmp` C10, other builtins E20, a callee the unit does not
  declare as a function K01; a corpus-defined callee is
  `unattributed:callee-closure` and an external one
  `unattributed:uncontracted-external` (analysis §2: a contract or the P4
  closure supplies these, not a Clang-gap card). The other descriptions map
  by family (listed in `attribute.py`, `classify`).
- **Functions.** [`scripts/functions.py`](scripts/functions.py): a card
  frees a function **alone** when the function's blockers are exactly that
  card; `strict` also requires no `unknown_alias` leaf; no leak or summary
  conflict in either. The unsupported-coverage §5 caveat applies: these are
  **upper bounds**, a modelled access can still fail its obligations.

### 3.2 Reconciliation

35,737 leaves = 25,913 attributed + 9,824 unattributed. By family:

| Family | Attributed | Unattributed |
|---|---:|---:|
| IND | 14,683 | 195 |
| COND | 5,456 | 595 |
| RET | 266 | 4 |
| OTHER | 5,304 | 387 |
| UNC | 204 | 8,643 |

4,298 Unknown functions = 805 blocked by one card + 418 by exactly two +
286 by three or more + 2,765 by at least one unattributed leaf + 24 with no
unsupported leaf (alias, leak or conflict only). Full rows:
[attribution-leaves.jsonl.gz](artifacts/attribution-leaves.jsonl.gz); tables:
[attribution-functions.json](artifacts/attribution-functions.json).

Largest entries by leaves: M01 8,983; E07 2,035; K01 1,747; M06 1,629; E19
1,513; M02 1,441; E16 1,337; I04 1,203; E13 1,054; E05 708; M04 700; M03
531; M09 432; E03 376. Largest unattributed: callee closure 6,737, external
uncontracted 1,906, uncontracted callee in a condition 587, no typed shape
186, nullable obligation 185, other assignment shapes 153.

### 3.3 Ranking (phase 1–3, by functions freed)

| Rank | Card | Leaves | Unknown functions touched | Freed alone (alias allowed) | Freed alone (strict) | Freed with one other card | Top partner |
|---:|---|---:|---:|---:|---:|---:|---|
| 1 | CG10 | 10,186 | 2,606 | **491** | 293 | 298 | CG11 80 |
| 2 | CG13 | 1,629 | 467 | 55 | 48 | 48 | CG10 25 |
| 3 | CG06 | 2,077 | 522 | 40 | 39 | 27 | CG10 7 |
| 4 | CG11 | 2,672 | 1,000 | 37 | 34 | 107 | CG10 80 |
| 5 | CG08 | 1,673 | 688 | 34 | 22 | 103 | CG10 65 |
| 6 | CG04 | 1,245 | 717 | 31 | 25 | 59 | CG10 32 |
| 7 | CG05 | 405 | 269 | 23 | 19 | 25 | CG10 14 |
| 8 | CG07 | 1,532 | 615 | 18 | 15 | 49 | CG10 21 |
| 9 | CG03 | 43 | 39 | 17 | 13 | 3 | CG06 2 |
| 10 | CG14 | 837 | 496 | 17 | 11 | 28 | CG10 8 |
| 11 | CG02 | 1,207 | 578 | 12 | 10 | 33 | CG10 12 |
| 12 | CG15 | 133 | 68 | 7 | 7 | 6 | CG10 2 |
| 13 | CG19 | 2 | 2 | 2 | 2 | 0 | — |
| 14 | CG16 | 24 | 13 | 2 | 1 | 1 | CG02 1 |
| 15 | CG12 | 76 | 33 | 1 | 1 | 5 | CG06 2 |
| 16 | CG17 | 3 | 3 | 0 | 0 | 0 | — |
| — | CG01, CG09 (T), CG18 | 0 | 0 | 0 | 0 | 0 | — |

Phase 4 cards with attributed leaves: CG22 1,747 leaves / 9 alone, CG26
352 / 3, CG21 57 / 3, CG30 12 / 2, CG28 1 / 1.

**Reading.** CG10 dominates because M01 ("the access would be read if its
root premise held") and I04 (one-line identity) are the residue of every
typed position route; its pair with CG11 (80) is the largest two-card set.
The T cards (CG01, CG09) free nothing here because the corpus arbiter runs
only the custody gateway. CG18's entries (M20–M22, M26, W08) are limits the
gateway publishes as `Safe` or `unknown_alias`, never as unsupported leaves,
so they have no leaves to free. 2,765 Unknown functions stay blocked by an
unattributed leaf whatever phase 1–3 card lands, mostly uncontracted calls
(the P4 closure and provider contracts).

## 4. Per-card predictions

Frozen, one per phase 1–2 card except CG01 (landed before this card
finished): [predictions/](predictions/) `CG02.md` … `CG15.md`, generated by
[`scripts/predict.py`](scripts/predict.py) ([summary](predictions/summary.json)).
Each lists the functions the card could free alone with their blocking leaves,
the obligations those accesses would carry, and the predicted verdict: Safe
(upper bound) when no alias leaf remains, Unknown when one does, and no
provisional Unsafe. The absorbed follow-ups are listed with their registered
sizes, carried unchanged: CG00 attributes leaves by entry precedence and did
not re-run any follow-up under its own definition, so no registered size is
amended. One is measured identically: `W44-SIZEOF-CAST-SCALAR-QUERY`'s family
(`C operator \`sizeof\``) is 465 leaves, as registered.

## Prediction and outcome

| Measure | Predicted | Outcome |
|---|---|---|
| BEFORE reproduces `ae17152b…` | 97% | **byte-identical** |
| AFTER byte-identical | 97% | **byte-identical** (tip with CG01 merged) |
| every entry a test or reading | 90% | **yes** (97 tests, W06 reading plus a pin) |
| entries reproducing exactly | 80–90 | **84** (3 with a clarification) |
| corrections | 5–15 | **14** (10 partial, 4 reason) |
| reading-only entries | 5–15 | **1** — below the band: the contract loader and exhaustive schema matches made K04–K07, W02–W05 executable |
| named likely corrections (E04, E08, E16, E07, C13, K08) | — | E16 only; E04, E08, E07, C13 and K08 reproduced |
| sums reconciled | 85% | **yes** |
| UNC stays the largest unattributed bucket | 80% | **yes** (8,643) |
| CG10 largest by leaves | 60% | **yes** (10,186) |
| CG10 first, CG11 second by freed alone | 50% | CG10 first; **CG11 fourth** (after CG13, CG06) |
| T cards free 0 in the corpus | 90% | **yes** |
| best card ≤ 150 alone; all phase 1–3 ≤ 400 | 70% | **no**: CG10 491 alone; phase 1–3 together 787 alone |
| pair adds ≥ 30% over the best single card | 50% | CG10's pairs add 298 to 491 (61%) |

The missed bound: the root premise (M01) is a larger single blocker than the
W46/W47 censuses suggested, because it collects every expressible path whose
root the never-bound closure refuses, in all positions at once.

## Corpus arbiter

`scripts/arbiter.sh` runs `QCORE_TYPED_PATH_PARITY=OUT/parity.jsonl python3
scripts/run_pinned_audit.py OUT 66910c10 --inputs-root …/build/cycle19
--jobs 8` and `scripts/identity.py` against
`w47-flb-lib-lifecycle/integration/artifacts/corpus/`.

| Run | Binary | Exit | `resource-diagnostics.json` |
|---|---|---:|---|
| BEFORE | `4f2af923…` (base `d3508e10`) | 0 | `ae17152b…` byte-identical ([identity](artifacts/arbiter-before.identity.json)) |
| AFTER | `9e0098a5…` (tip, CG01 merged) | 0 | `ae17152b…` byte-identical ([identity](artifacts/arbiter-after.identity.json)) |

Chunk contracts and the Monkey source are byte-identical in both; the
contracts and source-effects sidecars differ only by the checkout they record
(not verdict-bearing, as earlier cards disclose). No function moves. The
system slice is not due: no analyzer path changed.

## Lean

None. The card adds tests and records only, no semantics; the Lean audit
count is the tip's (227, CG01's `AuditTypedSourceOperators.lean` included).

## Gate

At the tip (base `d3508e10` with `7e7ba401` merged); exits in [gate/](gate/),
record in [gate.json](gate.json), logs under `private/cg00/gate2/`.

| Step | Command | Exit | Count |
|---|---|---:|---|
| submodule | `git submodule update --init --recursive` | 0 | |
| tests | `python3 scripts/run_test_binaries.py` | 0 | 367 units, 380 summaries; **6,561 passed**, 0 failed, 11 ignored (+106 over CG01's 6,455; `private/test-binaries/20260929T113425Z-1044802/summary.json`) |
| fmt | `cargo fmt --all -- --check` | 0 | |
| clippy | `cargo clippy --all-targets -- -D warnings` | 0 | |
| probe | `cargo run -p qcore-compiler --example reevaluation_probe` | 0 | |
| follow-up probe | `cargo run -p qcore-compiler --example reevaluation_followup` | 0 | |
| Lean | `python3 scripts/check_lean_audits.py` | 0 | 227 audits (no change) |
| Python | `python3 -m unittest discover -s scripts/tests` | 0 | 365 tests OK (after the matrix edit) |
| docs index | `python3 scripts/build_docs_index.py` | 0 | |
| clean | `cargo clean --profile dev` | 0 | |

A pre-merge run at the base (`private/cg00/gate/`) was green too: 6,551
passed, 226 audits. `providers/` is untouched, so the provider gate is not
due. Clang 21.1.8 recorded in this worktree.

## Follow-ups

- `CG00-CLANG-GAP-BASELINE` retired: every acceptance clause of its brief
  holds (tests or reading for all 98 entries, corrections listed with
  evidence, AFTER byte-identical, sums reconciled). Its ledger text's generic
  "matching audited Lean" clause does not apply: the card adds no semantics.
- CG02–CG15 amended with their measured sizes (leaves / Unknown functions
  touched / freed alone / strict / with one other card).
- Registered `CG00-G-CORRECTION-RESIDUALS`: the G corrections no live card
  owns (E14's character-literal initializer, C11's cleanup local and N14's
  `_BitInt` refused through IND, C12's asm read as a call).
- No absorbed follow-up is closed or resized (§4).
