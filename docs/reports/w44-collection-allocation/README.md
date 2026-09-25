# W44-COLLECTION-ALLOCATION: unevaluated `sizeof` dereference operands

Card W44-COLLECTION-ALLOCATION, 2026-09-25, on base `c1e69bb5` ("Open the
wave-44 increment record"). It discharges
`W43-COLLECTION-SIZEOF-DEREFERENCE-ALLOCATION`: the corpus allocates as
`v = malloc(n * sizeof(*v))`, and the lexical access gate read the operand's
`*` as a dereference, so the destination never bound. The census and forecast
were frozen before any rule existed ([PREDICTION.md](PREDICTION.md), commit
`ddf2222c`).

## Headline

* **The rule** ([§2](#2-the-rule)). C11 6.5.3.4p2 evaluates a `sizeof` operand
  only when its type is a variable length array type. An operand `*NAME`,
  `**NAME` or `NAME[LITERAL]` is admitted when **every** declaration of `NAME`
  in the function is a pointer declarator with enough `*`, no array dimension,
  and a specifier list that resolves to a type that cannot be variably
  modified. In a statement whose first call is an ordinary callee, the lexical
  access gate then reads the operand as a scalar marker. A name inside it no
  longer counts as a protected name the argument hands to its callee.
  Everything else is the existing classifier: the call's contract, its
  borrowed actuals, the nested-call rule and the flow analysis. No transfer
  rule and no `LegacyInstruction` variant is added.
* **Lean.** `QCore.SizeofOperand` has 23 theorems, audited by
  `AuditSizeofOperand.lean`, with no placeholders and no project axioms
  ([§3](#3-lean-parity)).
* **Arbiter** ([§5](#5-arbiter)). BEFORE reproduces the
  W43-INDEXED-RESOURCE-COLLECTIONS integration diagnostics byte for byte.
  AFTER removes **234** leaves (204 of them the withdrawn operand refusals) and
  adds **370**:

| Measure | BEFORE | AFTER | Delta |
|---|---:|---:|---:|
| Functions Safe / provisional Unsafe / Unknown | 294 / 85 / 4,455 | 298 / 129 / 4,407 | +4 / +44 / −48 |
| Units Safe / provisional Unsafe / Unknown / data-only | 3 / 43 / 135 / 4 | 3 / 65 / 113 / 4 | 0 / +22 / −22 / 0 |
| `leak` | 271 | **396** | **+125** |
| `unknown_alias` | 4,358 | 4,453 | +95 |
| `unsupported` | 45,848 | 45,764 | −84 |
| `double_release` / `invalid_release` / `use_after_release` | 0 / 0 / 0 | 0 / 0 / 0 | 0 |
| Functions Safe through a collection element node | — | **0** | 0 |

* **The main finding is in the accusing direction.** 44 functions and 22
  units moved Unknown → provisional Unsafe. Every one of the 125 added leak
  leaves names an allocation this card admitted
  ([census/leak-attribution.txt](census/leak-attribution.txt)).
  * 109 are **unwitnessed**. The identity is handed to a statement the
    classifier refuses, such as a `TAILQ_INSERT_TAIL` expansion or
    `rd_list_add(&conf->plugins, rkplug)` (107), or it is returned under a cast
    (2).
    * A refused statement withdraws the residual claim only for a publication
      the publication reader recognises (constraint 10).
    * It spares every binding it cannot write (constraint 10a).
    * A cast return escapes nothing (constraint 5b).
  * The functions read by hand publish or return the object.
  * **43 of the 44 function moves and 21 of the 22 unit moves rest on these
    claims only.** This is `W44-REFUSED-HANDOFF-LEAK-CLAIMS`
    ([§5.3](#53-leak-attribution)).
  * The other 16 leaves are witnessed: unit-test assertion-failure returns
    that do not free a fake `rk`.
  * None of these is an upstream defect claim.
* **Collections** ([§5.4](#54-collections)). The refusal of 9 of the 14
  `T **` array allocations is withdrawn, and 7 of them now bind their
  destination. Still, **0** of the 27 recognised element operations
  decides under the collection rules, **0** arrays become collections, and
  **0** functions are Safe through a collection element node. So the
  unreached-element question (`W43-COLLECTION-UNREACHED-ELEMENT-NODE`) is not
  engaged by any corpus verdict.
* **The 4 Safe moves** are all non-collection:
  * `XXH32_copyState` and `XXH64_copyState` (`memcpy(dst, src, sizeof(*dst))`);
  * `clear_env` (`memset(env, 0, sizeof(*env))`);
  * `rd_kafka_anyconf_clear_all_is_modified`, whose callee's validated
    summary now exists.
* **Prediction.** Most bands missed ([§5.5](#55-prediction-against-outcome)).
  The frozen forecast put the leak pin at 271–290 and Unknown → Unsafe at
  0–6. It underestimated how many newly bound allocations reach a refused
  handoff.
* **Ledger: 345 → 348.**
  * Retired: `W43-COLLECTION-SIZEOF-DEREFERENCE-ALLOCATION`.
  * Registered: `W44-REFUSED-HANDOFF-LEAK-CLAIMS`,
    `W44-SIZEOF-OPERAND-RESIDUALS`, `W44-SIZEOF-CAST-SCALAR-QUERY` and
    `W44-COLLECTION-ARRAYS-STILL-UNRESOLVED` ([§7](#7-ledger)).

## 1. Census

[`scripts/sizeof_census.py`](scripts/sizeof_census.py) reads the analysed
`-E -P` unit texts of a private capture run of the base binary. The capture
used W43-INDEXED-RESOURCE-COLLECTIONS' tee wrapper, and its texts were paired
with `w43-libc-contracts/scripts/unit_texts.py` (180 units). It classifies
every `sizeof` operand in every projected function
([census/sizeof-census.txt](census/sizeof-census.txt),
[census/sizeof-census.json.gz](census/sizeof-census.json.gz)).

| Question | Measured |
|---|---|
| Operand shapes | `(NAME)` 4,513; other 2,262; `(typedef)` 313; **`(*NAME)` 288**; `(struct T)` 186; `(scalar type)` 123; `(member access)` 122; `(*member access)` 100; `(scalar type *)` 62; `(typedef *)` 43; string literal 31; **`(**NAME)` 4**; `(struct T *)` 3; bare `NAME` 3; `*NAME` unparenthesised 0; `NAME[LIT]` 0 |
| Owned-result allocations carrying a `sizeof` | `(*NAME)` of the destination itself 190 (175 into `T *`, 12 into `T **`, 1 `(**NAME)`); `(struct T)` 120; `(typedef)` 107; `(*member access)` 21 |
| `indirect memory access` leaves whose line's only access is a dereference operand | **226** (224 on **211** Unknown functions, 2 on Unsafe); **4** Unknown functions blocked by nothing else. This reproduces the W43 card's 224 / 211 / 4 |
| Callees on those lines | contracted: `rd_calloc` 122, `rd_malloc` 41, `memset` 29, `memcpy` 4; uncontracted: `rd_tmpabuf_alloc0` 7, `LZ4_initStreamHC` 6, `__builtin_alloca` 4, `LZ4_initStream` 2, `rd_tmpabuf_new` 2, `qsort` 2, `LZ4F_malloc` 1, `rd_tmpabuf_write0` 1; no call 4; `_Generic` 1 |
| `C operator \`sizeof\` requires type/operand analysis` | 465 leaves (447 on 117 Unknown functions, 1 sole-blocked). This is not a dereference family. At AFTER, 398 of the 465 are librdkafka's `size_t __len2 = (size_t)(sizeof(_v));`, a scalar query under a cast (`W44-SIZEOF-CAST-SCALAR-QUERY`) |

The census reads source tokens, so it is a forecast instrument. The analyzer's
own reading is the authority.

## 2. The rule

The syntax and premises live in
[`crates/qcore-c-gateway/src/sizeof_operand.rs`](../../../crates/qcore-c-gateway/src/sizeof_operand.rs),
with its 7 unit tests.

* **Operand shapes.** The rule recognises `* NAME`, `* * NAME` (any number of
  stars) and `NAME [ DECIMAL ]`, parenthesised or not. It refuses an operand
  followed by a postfix: `[`, `(`, `.`, `->`, `++` or `--`. It also refuses
  `(*v)[0]`, `*v->f` and `(v[i])`.
* **Declaration premise.** `OperandTypes::read` takes
  `typed_field_access::declarator_facts`, the same total-or-reject reader the
  collection facts use. Every declaration of `NAME` in the parameter list and
  body must have:
  * `dimensions == 0` and `stars >= k`;
  * specifiers for which `TypedAccessModel::never_variably_modified` holds.
    That is an appended method: arithmetic, `void`, a tagged
    `struct`/`union`/`enum`, or a typedef chain ending in one, including a
    typedef that defines an aggregate.
  * It rejects `volatile`, `_Atomic` and `typedef` storage.

  The following refuse:
  * an unreadable function (a nested declarator such as `int (*p)[n]`
    refuses them all);
  * an undeclared or file-scope name;
  * an array declaration anywhere;
  * an unresolved, pointer, array or disagreeing typedef.

  Block-scope VLA typedefs are read by `read_typedefs` as unresolved, so
  `typedef int row[m]; row *p;` refuses.
* **Why the premise suffices.** The operand's type is the specifier type under
  the remaining stars. Only an ordinary identifier with block or prototype
  scope may have a variably modified type (C11 6.7.6.2p2), and a member may not
  (6.7.2.1p9). So the operand's type is never a VLA type, and it is never
  evaluated.
* **Hooks in `legacy_c.rs`.** There are two thin, commented hooks; nothing is
  reordered. Both apply only when `find_call` returns an ordinary callee.
  * **The lexical gate.** Both gate positions read through a view in which
    admitted operand spans become the marker `1`. With no admitted span the
    view is `outside_spans`, unchanged.
  * **The protected-name mention of an argument.** The check "call has an
    unmodeled formal address or borrowed expression" reads the argument's
    tokens outside admitted spans (`sizeof_operand::evaluated`). With no span,
    that is every token.
* **The marker is never a value** (constraint 5a). It reaches no CFG, branch,
  literal premise or scalar fact. The classifier after the gate reads the
  original tokens, where the operand is part of a data argument and names no
  place.
* **What is untouched.**
  * Statements with no ordinary call keep their refusal byte for byte. This
    covers `n = sizeof(*p);` and a statement whose first call is `sizeof`.
  * Conditions, returns and the fixed type-query route are untouched.
  * No new instruction is added, so `admits_descriptor_read` needs no new arm.
  * Collections stay heap-kind only.
  * The `B[X++]` forget rule and `Derivation::Derived` are untouched.

## 3. Lean parity

[`QCore.SizeofOperand`](../../../proof/QCore/SizeofOperand.lean) is audited by
[`AuditSizeofOperand.lean`](../../../proof/AuditSizeofOperand.lean). It has 23
theorems, no placeholders and no project axioms. It models C types with a
variably-modified flag, the evaluation fact (`evaluates t = t.isVLA`), operand
types under a declaration, the reader's facts, and the admission.

* **Soundness.**
  * `an_admitted_operand_is_not_evaluated`: under `Faithful`, which states that
    a specifier the reader resolves as never variably modified is not
    variably modified, every declaration the name may refer to gives the
    operand a non-VM type.
  * `an_admitted_operand_has_no_effect`: the operand reads and dereferences
    nothing.
* **Refusals and non-vacuity.**
  * `a_variably_modified_operand_is_evaluated` is the `int (*p)[n]` witness.
  * The refusals: `an_unreadable_declaration_is_refused`,
    `an_undeclared_name_is_refused`, `an_array_declarator_is_refused`,
    `a_possibly_variably_modified_specifier_is_refused`,
    `a_deeper_dereference_is_refused` and `no_stars_is_refused`.
* **The gate.**
  * `the_marker_is_not_an_access_or_a_literal`.
  * `no_operand_leaves_the_gate_unchanged`.
  * `the_view_refuses_no_more`: the rule only withdraws a refusal.
  * `an_admitted_statement_keeps_its_lowering`.
  * `a_refusal_is_withdrawn_only_by_the_view`.
* **The mention filter.** `the_mention_filter_only_forgets` and
  `no_operand_forgets_no_name`.
* **Non-vacuity.** `the_collection_allocation_is_admitted` and
  `the_allocation_gate_passes_the_view`.

Trust boundary 28 covers:
* faithful token and type extraction, including the typedef resolution behind
  `Faithful`;
* the Rust–Lean correspondence.

Both are stated as hypotheses, not proved. Only the size's custody is
modelled.

## 4. Tests

[`crates/qcore-c-gateway/tests/sizeof_operand.rs`](../../../crates/qcore-c-gateway/tests/sizeof_operand.rs)
has 16 tests. Each asserts the complete diagnostic vector with the status, and
rechecks every `Safe` certificate.

* **Safe:**
  * parenthesised, unparenthesised and literal-subscript operands;
  * a typedef resolving to a scalar;
  * a data argument naming a formal (`x_zero(e, sizeof(*e))`);
  * a released pointer named in the operand (not a use);
  * the brief's collection: a `T **` allocation, fill loop, release loop and
    array release. Safe through collection element nodes, fixture only.
* **Unsafe:** a newly modelled allocation that is never released (a leak: the
  accusing direction).
* **Refused (Unknown):**
  * a pointer to a VLA;
  * a block-scope VLA typedef;
  * an array declarator;
  * a file-scope name;
  * a member operand;
  * `n * sizeof(*p)` (the declarator reader quirk);
  * a statement with no call;
  * a statement whose first call is `sizeof`.

## 5. Arbiter

### 5.1 Runs

Every run uses
[`../w43-indirect-access-general/scripts/run_audit.py`](../w43-indirect-access-general/scripts/run_audit.py)
with `--binary`, the pinned checkout, `/usr/bin/clang` and the wave-20 ChunkIO
package.

| Run | Binary | Tree | Exit | Record |
|---|---|---|---:|---|
| BEFORE | `6379af44…` | `c1e69bb5` | 0 | [before-run.json](before-run.json), [before-identity.json](before-identity.json) |
| capture | `6379af44…` | the same, through the tee wrapper | 0 | private; §1 |
| AFTER | `5e174728…` | `791fc5fd` | 0 | [after-run.json](after-run.json), [after-diff.json](after-diff.json), [artifacts/](artifacts/) |
| CENSUS | `735a43cb…` | `791fc5fd` + `indirect-census` | 0 | [census-run.json](census-run.json); its five artifacts are byte-identical to AFTER |
| AFTER (confirmation) | `583faba8…` | `c4338703` (the clippy `matches!` rewrite) | 0 | [after-confirmation-run.json](after-confirmation-run.json); all five artifacts byte-identical to AFTER |

The only Rust change after `791fc5fd` is `c4338703`, which rewrites one
`match` as `matches!` for clippy. The confirmation run on it is byte-identical
in all five artifacts. Every other later commit changes only Lean, documents,
the ledger and evidence.

* **BEFORE against the baseline**
  (`w43-indexed-resource-collections/integration/artifacts/`,
  [`scripts/identity.sh`](scripts/identity.sh)):
  * diagnostics, chunk-contracts and source are byte-identical;
  * contracts are identical after the root alias;
  * source-effects carry only checkout-path digests.
* **AFTER against BEFORE.**
  * chunk-contracts, contracts, source-effects and source are byte-identical.
  * The diagnostics differ in leaves and `diagnostic_counts` only.
* **AFTER against the baseline**
  ([after-vs-baseline-identity.json](after-vs-baseline-identity.json)):
  * contracts are identical after the alias;
  * source-effects differ in 29 leaves, all in the fields the brief lists
    ([after-vs-baseline-source-effects.txt](after-vs-baseline-source-effects.txt)):
    `resource_contracts:<digest>` bounds keys and `coverage[0]`,
    `build_digest`, `configuration_digest`, `obligation_id`, `task_digest` and
    a `contract_sources` digest.

### 5.2 Leaf-by-leaf attribution

[`scripts/delta_attribution.py`](scripts/delta_attribution.py)
([census/delta-attribution.txt](census/delta-attribution.txt)) puts every
leaf in one class.

| Class | Removed | Added |
|---|---:|---:|
| Withdrawn operand refusal (`indirect memory access` within two lines of a census dereference operand) | 203 | — |
| Same-line substitution (the statement now reaches a later refusal) | — | 17 unsupported |
| Downstream in one of the 188 functions reached | 28 (26 unknown_alias, 2 unsupported) | 353 (125 leak, 121 unknown_alias, 107 unsupported) |
| Outside a reached function | 3 | 0 |
| Unattributed | 0 | 0 |

The 3 "outside" rows are:
* `pb_field_iter_begin` L437, `memset(iter, 0, sizeof(*iter))`, whose
  definition the census's function reader missed. It is a withdrawn operand
  refusal, which makes **204** withdrawals.
* Two `call to uncontracted function \`clear_env\`` leaves in
  `rd_kafka_snappy_init_env`/`_free_env`. `clear_env` is now Safe and its W4
  summary validates.

The 17 substitutions are:
* uncontracted callees: `LZ4_initStream(HC)`, `__builtin_alloca`,
  `rd_tmpabuf_alloc0`, `rd_memdup` and `_Generic`;
* one `discarded owned result`;
* one `call has an unmodeled formal address or borrowed expression`, from the
  `* formal` quirk of §7.

Downstream rows appear because the destination is now bound where it used to
be poisoned. The added ones are:
* `unknown_alias` and `nullable resource … used by a non-null contract`, where
  a nullable result reaches a non-null use unchecked;
* the leaks of §5.3.

The removed ones are `unresolved resource identity` leaves for names that now
bind.

All 48 function moves are in reached functions:

| Move | Count |
|---|---:|
| Unknown → Safe | 4 |
| Unknown → provisional Unsafe | 44 |

### 5.3 Leak attribution

[`scripts/leak_attribution.py`](scripts/leak_attribution.py)
([census/leak-attribution.txt](census/leak-attribution.txt)) takes each added
leak leaf. It finds the allocation the leaf names: the leaf's origin
coordinate is matched against the refusal this card withdrew. It then reads
what the allocated name meets afterwards. All **125** allocations were
admitted by this card.

| Class | Leaves | Witness status |
|---|---:|---|
| `handed-to-a-refused-statement`: the name reaches, after the allocation, a statement carrying an `unsupported` leaf | 107 | **Unwitnessed.** The functions read by hand insert the object into a list or table the statement refuses. The claim exists because a refused statement withdraws a residual only for a recognised publication (constraint 10) and spares the bindings it cannot write (10a) |
| `returned-under-cast`: `return (rd_kafka_message_t *)rkm;`, `return rd_kafka_toppar_keep0(…, rktp);` | 2 | **Unwitnessed.** The return route escapes no place for a non-place spelling (5b) |
| `other`: the path to the exit is modelled throughout | 16 | **Witnessed by reading the source.** Unit-test `RD_UT_ASSERT` failure returns that do not free a fake `rk` (`unittest_http_get_params_append`, `ut_sasl_oauthbearer_oidc_assertion_create`) |

The functions read by hand were:
* `rd_kafka_coord_cache_add`, which inserts `cce` with `TAILQ_INSERT_HEAD`;
* `rd_kafka_mock_handle_EndTxn`, where `mabort` goes onto
  `mpart->aborted_txns`;
* `rd_kafka_plugin_new`, with `rd_list_add(&conf->plugins, rkplug)`;
* `rd_kafka_message_new`, which returns under a cast.

Each publishes or returns the object, so its leak claim is false. The
classifier is source-token based. Its classes are a disclosure, not a
verdict.

By function:

| Functions | Status move | Added leaks |
|---:|---|---|
| 43 | Unknown → provisional Unsafe | unwitnessed only |
| 1 | Unknown → provisional Unsafe | includes a witnessed leak (`unittest_http_get_params_append`) |
| 2 | already Unsafe | added leaks only |

Units: 21 of the 22 moves rest on unwitnessed claims only.

Every added leak is a provisional `Unsafe` under the existing policy.
**None is presented as an upstream defect**, and the 109 unwitnessed ones are
most likely false. They are registered as `W44-REFUSED-HANDOFF-LEAK-CLAIMS`
for a decision on that accusing arm. W44-LEAK-PIN-AUDIT, running
concurrently, is the natural owner.

**Constraint 7:** the pin moves **271 → 396**. The whole move is these 125
leaves, and no baseline leak leaf is withdrawn.

### 5.4 Collections

[`scripts/collection_reach.py`](scripts/collection_reach.py)
([census/collection-reach.txt](census/collection-reach.txt)) looks at the 14
self-referential dereference allocations into a `T **` destination.

* **9 withdrawn, 7 bind, 5 kept.** The allocation refusal of 9 is withdrawn.
  Two of those statements then reach another refusal:
  * `rd_kafka_conf_kv_split` reaches the `* formal` quirk;
  * `rd_kafka_handle_ApiVersion` stores through `*apis` and reaches
    `discarded owned result`.

  So 7 bind their destination. The other 5 keep the access refusal, for
  another access on the statement or a reader quirk.
* **Element operations.** The census run lowers the same **27** element
  operations in 19 functions as W43
  ([census/element-reach.txt](census/element-reach.txt)). **All 27 keep their
  fallback leaf in AFTER.**
* **The only overlap is `rd_kafka_ListGroups_resp_cb`.** Its
  `grps = rd_malloc(sizeof(*grps) * grpcnt)` now binds. `rd_malloc`'s result
  is nullable and never tested, so `grps[i++] = …` finds no non-null identity
  and stays its fallback.
* **Totals.**
  * **0** arrays become collections.
  * **0** element operations move under the collection rules.
  * **0** functions move through them.
  * **0** functions are Safe through a collection element node, so there was
    nothing to stop for under the unreached-element decision.

### 5.5 Prediction against outcome

| Forecast | Measured | |
|---|---|---|
| ≈190 withdrawn, ≈25 substituted, ≈5 kept of the 226 | 204 withdrawn, 17 substituted, 5 kept | close |
| `unsupported` −150 to −224 | −84 (−204 withdrawn, +124 newly reached downstream and substitutions) | **miss** |
| `unknown_alias` −0 to −300 | **+95** | **miss (sign)** |
| leak 271–290 | **396** | **miss** |
| Unknown → Safe 2–4, none through collections | 4, none through collections | hit |
| Unknown → Unsafe 0–6 | **44** | **miss** |
| Safe → other 0 | 0 | hit |
| units 0 (0–1) | **22** | **miss** |
| arrays resolved as collections 0–3 | 0 | hit |
| element operations moved 0–6 | 0 | hit |
| collection-derived Safe 0 | 0 | hit |
| violation candidates 0–6, all leaks | 125 leaks | **miss** |

The forecast assumed that a newly bound allocation would usually meet a later
refusal that withdraws its claim. It does not. Constraint 10a spares the
bindings a refused statement cannot write, and only a recognised publication
withdraws a residual. The frozen file is not edited.

## 6. Validation

Measured in this worktree by [`scripts/run_gate.py`](scripts/run_gate.py),
W43-INDEXED-RESOURCE-COLLECTIONS' runner, with `CARGO_INCREMENTAL=0`. Every
exit is captured to a file. Logs are in `private/gate/` and
`private/provider-gate/`, and the generator input is [gate.json](gate.json).

The core gate was measured on `c4338703`, the final Rust source, in
`private/gate2`. The Lean step was measured on `bcdc9c06` in `private/gate`;
no Lean source changed after it. At least 70G stayed free on `/home`.

An earlier core gate on `bcdc9c06` (`private/gate`) had clippy exit **101**:
`match_like_matches_macro` in `sizeof_operand.rs`, with the census step failing
for the same reason. It was fixed in `c4338703`, and the test step and every
Rust step were re-run. Its test step had also passed, with 5,413 tests.

| Command | Exit | Result |
|---|---:|---|
| `git submodule update --init --recursive` | 0 | libevent `d82464a2…` |
| `clang --version` | 0 | Ubuntu clang 21.1.8 ([recording](clang-version.txt)) |
| `cargo test --workspace --no-fail-fast` | 0 | **5,413 passed / 0 failed / 11 ignored**, 288 summaries |
| `cargo clean --profile dev` | 0 | reclaimed after the test step |
| `cargo fmt --all -- --check` | 0 | clean |
| `cargo clippy --workspace --all-targets -- -D warnings` | 0 | clean (101 on `bcdc9c06`, see above) |
| `cargo run -p qcore-compiler --example reevaluation_probe` | 0 | |
| `cargo run -p qcore-compiler --example reevaluation_followup` | 0 | |
| `python3 scripts/check_lean_audits.py --jobs 4` | 0 | **178** audits |
| `cargo clippy -p qcore-c-gateway --features indirect-census --all-targets -- -D warnings` and `cargo test -p qcore-c-gateway --features indirect-census --lib` | 0 | clean; 457 passed |
| `python3 docs/reports/remaining-d02/build_matrix.py . --gate docs/reports/w44-collection-allocation/gate.json` | 0 | 348 follow-ups, 111 contract modules, 178 Lean audits |
| `python3 -m unittest discover -s scripts/tests` (after the final regeneration) | 0 | **330** tests |
| `python3 scripts/build_docs_index.py`, then `--check` | 0 / 0 | regenerated and matching |
| `cargo test --manifest-path providers/Cargo.toml --workspace --locked --offline --no-fail-fast` | 0 | **124 passed / 0 failed / 0 ignored**, 39 summaries |
| `… -p qcore-provider-conformance --test packages real_packages_preserve_unknown_and_reject_mutations … -- --exact --test-threads=1` | 0 | 1 passed (isolated) |
| `cargo fmt --manifest-path providers/Cargo.toml --all -- --check` | 0 | clean |
| `cargo clippy --manifest-path providers/Cargo.toml --workspace --all-targets --locked --offline -- -D warnings` | 0 | clean |
| `python3 providers/contracts.py check` | 0 | inventories unchanged |
| `python3 providers/package.py build --output private/provider-gate/packages` | 0 | 5 packages |
| `python3 providers/package.py verify …/<id>` (×5) | 0 ×5 | chunkio, cmetrics, ctraces, libevent, librdkafka |
| `python3 -m unittest discover -s providers/tests` | 0 | 13 tests |

**Counts.** This card adds:
* the 16-test `sizeof_operand.rs` binary;
* 7 unit tests of `sizeof_operand.rs`;
* one Lean audit.

No existing pin moved. The base's figures come from the matrix's validation
section, the W43-DESCRIPTOR-NATIVE-CHECK integration gate: 5,390 tests and 177
audits. So the card accounts for exactly 5,390 + 23 = 5,413 tests and 177 + 1
audits. They were not remeasured on this tree.

## 7. Ledger

**Retired: `W43-COLLECTION-SIZEOF-DEREFERENCE-ALLOCATION`.** Its discharge was
"a type-query card admitting `sizeof(*NAME)`/`sizeof *NAME` for a declared
object pointer whose pointee is not a variably modified type, with its VLA
argument, Lean parity and a measured re-pin". This card delivers:
* that admission at the position the allocation uses, an ordinary call
  statement;
* the VLA argument (§2, `a_variably_modified_operand_is_evaluated` and the
  refusals);
* Lean parity;
* the re-pin, with 204 of the 226 withdrawn.

What stays refused is registered.

**Registered** ([followups.json](followups.json)):

| Id | What stays open |
|---|---|
| `W44-REFUSED-HANDOFF-LEAK-CLAIMS` | 109 unwitnessed added leak leaves, behind 43 function and 21 unit moves; a decision on the accusing arm |
| `W44-SIZEOF-OPERAND-RESIDUALS` | the operand outside an ordinary call statement (type-query, condition and return positions); member operands (100 + 122 occurrences, 21 allocations); the declarator reader reading `( n * sizeof …` as a declaration; protected-formal discovery reading `sizeof(*p) * n` as `*n`; 134 access leaves still within two lines of an operand |
| `W44-SIZEOF-CAST-SCALAR-QUERY` | the `C operator sizeof` family: 465 leaves, 398 `(T)(sizeof(NAME))` casts around a scalar query |
| `W44-COLLECTION-ARRAYS-STILL-UNRESOLVED` | 9 of 14 `T **` allocation refusals withdrawn (7 bind), 0 element operations resolve |

**Not discharged, noted:**
* `W43-COLLECTION-NAME-STORE` (`a[i] = q`). A sound rule needs a
  refusal-bearing binding state for the stored name, so that every later use
  refuses. That is not cheap. It would also reach nothing here: no admitted
  `T **` allocation's function has a lowered name store.
* `W43-COLLECTION-ACCUSING-UNEXERCISED-IN-CORPUS` stays: still no element
  operation's array resolves.
* `W43-COLLECTION-UNREACHED-ELEMENT-NODE` stays the user's decision, and no
  corpus verdict depends on it.
* `W43-RETURN-TYPE-OPERATOR` is untouched.

Follow-ups go **345 → 348**. `corpus.cycle19_raw_units` points at
[artifacts/](artifacts/), and its previous contents are in
[previous-corpus-row.json](previous-corpus-row.json)
([`scripts/update_ledger.py`](scripts/update_ledger.py)).

## 8. For the integrator

* **Decide the leak move before merging.** The card is correct on its own
  premise, but it publishes 44 new provisional-Unsafe functions and 22
  provisional-Unsafe units. 43 and 21 of them rest on claims the attribution
  marks as most likely false. Merging moves STATUS's unit headline from
  3 / 43 / 135 / 4 to 3 / 65 / 113 / 4 and the leak pin to 396. You could:
  * merge and let `W44-REFUSED-HANDOFF-LEAK-CLAIMS` (with W44-LEAK-PIN-AUDIT)
    decide the arm;
  * or hold this card until that arm is decided.

  Either way the user should see it.
* **Proposed gateway amendment**, a new constraint 5j, in these words:

  > W44-COLLECTION-ALLOCATION: a `sizeof` operand `*NAME`, `**NAME` or
  > `NAME[LITERAL]` is unevaluated (C11 6.5.3.4p2) when every declaration of
  > `NAME` in the function is a pointer declarator with at least that many
  > `*`, no array dimension, and a specifier list `never_variably_modified`
  > resolves (arithmetic, `void`, a tagged aggregate, or a typedef chain
  > ending in one; a block-scope VLA typedef is unresolved). Only in a
  > statement whose first call is an ordinary callee, the lexical access gate
  > reads it as the scalar marker and the protected-name mention of an
  > argument skips it; the marker is never a value, and everything after the
  > gate reads the original tokens. Every other position, a variably modified
  > or unresolved operand and an array declarator keep their refusal byte for
  > byte. Admitting an allocation exposes its later handoffs to the accusing
  > arm of constraints 10 and 10a (`W44-REFUSED-HANDOFF-LEAK-CLAIMS`).

* **Constraint 7:** record W44-COLLECTION-ALLOCATION as moving the pin
  271 → 396, with 125 added leaves (109 unwitnessed, 16 witnessed) and no
  withdrawal.
* **Files likely to conflict:**
  * `crates/qcore-c-gateway/src/legacy_c.rs`: the `Parser` field and its
    initialisation; `sizeof_operand_types` beside `collection_types`; the gate
    hook before `unsupported_access` and its two gate calls; and the argument
    mention check in the contracted-call arm. The last also concerns
    W44-INDIRECT-CALL-ARGUMENT-ACCESS.
  * `crates/qcore-c-gateway/src/typed_field_access.rs`: one appended `impl`
    block before the tests.
  * `crates/qcore-c-gateway/src/lib.rs`: one module.
  * `proof/QCore.lean`: one import.
  * `docs/QCORE_SPEC_r0_3.md` §L18: one paragraph.
  * The shared records: `docs/reviews/r0_3/acceptance_matrix.json` (the
    corpus row and 4 entries), `docs/STATUS.md`,
    `docs/development/increments/wave-44.md` and `docs/reports/INDEX.md`.
* **Artifacts.** The `contracts` and `source-effects` artifacts embed this
  worktree's path, as earlier isolated cards' do.

## Reproduction

```sh
python3 docs/reports/w43-indirect-access-general/scripts/run_audit.py <before> c1e69bb5 --binary <base binary>
docs/reports/w44-collection-allocation/scripts/identity.sh <before> before-identity.json
docs/reports/w43-indexed-resource-collections/scripts/capture_run.sh <base binary> <capture>
python3 docs/reports/w43-libc-contracts/scripts/unit_texts.py <before>/resource-diagnostics.json <capture>/ppP <unit-texts>
python3 docs/reports/w44-collection-allocation/scripts/sizeof_census.py <before> <unit-texts> <before>/resource-diagnostics.contracts.json census.json census.txt
python3 docs/reports/w43-indirect-access-general/scripts/run_audit.py <after> <commit> --binary <after binary>
python3 docs/reports/w43-indirect-access-general/scripts/run_audit.py <census> <commit>+census --census --binary <census binary>
python3 docs/reports/w43-indexed-resource-collections/scripts/diff_runs.py <before> <after> after-diff.json
python3 docs/reports/w44-collection-allocation/scripts/delta_attribution.py <before> <after> census.json delta.json delta.txt
python3 docs/reports/w44-collection-allocation/scripts/leak_attribution.py <before> <after> <unit-texts> leaks.json leaks.txt
python3 docs/reports/w43-indexed-resource-collections/scripts/element_reach.py <census> <after> <before> element-reach.json element-reach.txt
python3 docs/reports/w44-collection-allocation/scripts/collection_reach.py census.json delta.json <before> <after> <census> reach.json reach.txt
python3 docs/reports/w44-collection-allocation/scripts/residual_operands.py census.json <after> <unit-texts> residual-operands.txt
python3 docs/reports/w43-libc-contracts/scripts/archive.py <after> docs/reports/w44-collection-allocation/artifacts
python3 docs/reports/w44-collection-allocation/scripts/update_ledger.py
python3 docs/reports/w44-collection-allocation/scripts/run_gate.py <gate-dir> submodules clang test fmt clippy probe followup lean census
```
