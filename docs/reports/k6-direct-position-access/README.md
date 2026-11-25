# K6-DIRECT-POSITION-ACCESS: direct-position access, re-censused

Card K6 of round 37 (brief `private/integrator-briefs/r37-k246-common.md`), base
`5087f9295` (origin/main, round 34 landed). Bundle: an expressible typed access
path directly in a statement, return or condition (no call actual) that the typed
layer refuses for its root premise or value form (census tags
`expr:statement:direct`, `expr:cond:direct`, `expr:return:direct`); owners
W47-ACCESS-PATH-POSITION-RESIDUALS (primary), CG10-BOUND-ROOT-PATH-REFUSED,
CG10-BOUND-ROOT-NULLABLE-OR-TRACKED; rules 10k, 10n; census draft
[predictions/K6.md](../current-access-alias-census/predictions/K6.md).

## 1. Re-census (11-07 vs 11-24)

Source: the user's `current` report written 2026-11-24 09:58 (Fluent Bit
`046accf6`, 1,173 Safe / 3,750 Unknown / 11 Unsafe rows; read-only), joined to
CURRENT-ACCESS-ALIAS-CENSUS's 11-07 leaf tags by (file, function, `-E -P` line,
family, sub) with K1's join ([scripts/recensus.py](scripts/recensus.py), the K1
script with the bundle set to the three `direct` tags;
[census/recensus.json](census/recensus.json)). 11,541 of 12,259 IND/COND/RET leaf
groups and 9,420 of 10,108 alias leaves join; an unjoined group carries `new` and
is never counted removable. [scripts/sites.py](scripts/sites.py) lists every K6
leaf group with its origin text ([census/sites.jsonl.gz](census/sites.jsonl.gz)).

| Measure | 11-07 (base `7b726587b`) | 11-24 (user's run) |
|---|---:|---:|
| function rows / Unknown | 4,912 / 3,832 | 4,934 / 3,750 |
| K6 leaves | 1,571 | 1,395 (statement 875, condition 465, return 55) |
| Unknown rows touched | 769 | 671 |
| rows freed alone / distinct functions | 21 / 21 | **25 / 25** |
| freed alone by `statement` / `cond` / `return` alone | 6 / 1 / 14 | 8 / 2 / 15 |

The 21 functions of the 11-07 frozen list now: **0 Safe**, **20 still freed
alone**, 1 (`mk_plugin_time_now_human`) whose return leaf moved line in the
`--monkey-linked` text (`new`, not a semantic change). Five functions joined
the freed-alone set since 11-07 (their other blockers went):
`LZ4HC_init_internal`, `rd_kafka_ListConsumerGroups_result_errors`,
`rd_kafka_topic_partition_topic`, `rd_kafka_sasl_cyrus_cb_canon`,
`sort_by_map_elem_val_toppar_list_cnt`.

The 11-24 run predates round 34's landing (rules 75, 77, 78, 10v amended), so
the 25 rows are re-read at the base `5087f9295` before anything is counted
(section 2).

### 1.1 What earlier cards refused on purpose

Nothing below is re-admitted by this card.

- **An interior address returned** (array decay, `&B->F`, `&B->A[I]`, a pointer
  offset): W45-RETURN-EXPRESSION-FAMILIES and rule 10k (2) ("an array member
  returned by decay is an interior address and refuses (rule 5j)"), rule 10n
  (1) ("an address is never returned"), AG3 (`AG3-RETURN-MEMBER-ADDRESS-DERIVED`),
  rule 77 ("do not admit ... an address or an array decay as a value"). An
  interior address is a result derived from `B`; published as a result that
  carries nothing it would let a caller release `B` and keep using the result
  unaccused. Its discharge is a `Derived { parameter, extent }` result through
  the derived-summary route, which is that owner's card, not this one.
  **10 of the 25 rows**, and `mk_plugin_time_now_human` (`&B->F`, its leaf
  `new` on 11-24): `rd_kafka_name` (`rk->rk_name`, `char[128]`),
  `rd_kafka_ConfigEntry_name` (`entry->kv->name`, `char name[1]`),
  `rd_kafka_share_partition_offsets_offsets` (a flexible array member),
  `rd_kafka_MemberDescription_assignment`, `rd_kafka_TopicDescription_topic_id`,
  `rd_kafka_conf` (`&B->F`),
  `rd_kafka_UserScramCredentialsDescription_scramcredentialinfo`,
  `rd_kafka_mock_partition_find` (`&B->A[I]`), `cJSON_GetErrorPtr`
  (`global_error.json + global_error.position`) and `extra_alloc`
  (`p = rbuf->rbuf_extra + of; ... return p;`).
- **A pointer difference or an offset outside an indirection**: rule 78 reads
  `*(P + I)` only and says not to read "a difference of two pointers"
  (`LZ4HC_init_internal`'s `(size_t)(hc4->end - hc4->prefixStart)`).
- **A sibling call beside a deciding operand** (rule 10z (3)):
  `rd_kafka_broker_weight_usable`'s `(rd_clock() - (tx_last > 0 ? tx_last : rkb->rkb_ts_state))`.
- **A record object outside automatic storage** (rule 12j: "do not read ... a
  `static`, `extern`, `register`, thread-local or file-scope root"):
  `clear_unit_test_data`, `unit_test_telemetry_decoded_int64` store scalar
  members of the file-scope `static struct ... unit_test_data`.
- **A whole read of a formal copied into a local** (rule 77, rule 62's copy):
  `rd_kafka_topic_partition_topic`'s `rktp = (const rd_kafka_toppar_t *)rktpar`
  and `sort_by_map_elem_val_toppar_list_cnt`'s `const rd_map_elem_t *a = _a,
  *b = _b;` (then `al = a->value`).

## 2. The 25 rows as C source shapes (method step 2)

Each row was read at its `-E -P` line ([scripts/context.py](scripts/context.py)
for the origin, K1's `preprocess.py` for the analysed text) and probed on the
base with the unit's own typedef spellings over the synthetic contracts
([probes/](probes/), harness [probes/k6_probe_scratch.rs.txt](probes/k6_probe_scratch.rs.txt),
[scripts/probe-run.sh](scripts/probe-run.sh), parity rows read with
[scripts/show_parity.py](scripts/show_parity.py)). The generic `struct T *`
spellings of most shapes are already `Safe` at the base (`probes1.txt`,
`probes2.txt`); the typedef spellings the production rows use are not
(`probes5.txt`), and each refusal has one cause:

| Shape (sample) | Rows | Refusal at the base | Disposition |
|---|---:|---|---|
| interior address returned (`return rk->rk_name;` decay, `return &rk->rk_conf;`, `return &d->a[i];`, `return g.json + g.position;`) | 10 | rule 10k (2) / 10n (1) / 5j, W45 | refused on purpose (§1.1) |
| `return ((&mpart->leader_responses)->tqh_first);` (`TAILQ_FIRST`) | 1 | rule 5b premise: lexical `may_bind` names `mpart` (`lexical-may-bind-only` disagreement; AST answers not bound) | **rule 81 (1)** |
| `size = (int32_t)((((&m->bytes)->len) == -1 ? 0 : ((&m->bytes)->len)) / c);` (`RD_KAFKAP_BYTES_LEN`) | 1 | rule 10z edge: `place: path through UnaryOperator` in the elaborator's place reader | **rule 81 (2)** |
| `size_t HostSize = (((&p->eps[i].Host)->len) == -1 ? 0 : …) + 1;` (`RD_KAFKAP_STR_LEN`) | 1 | the same | **rule 81 (2)** |
| `return (void *)rl->rl_elems[idx];` | 1 | the typed return reader has no value form for an explicit cast (`cast`, `const-pointer` features; the uncast return is `Safe`) | K6-RETURN-IDENTITY-CAST |
| `*cntp = rd_list_cnt(&config->config); … return (const T **)config->config.rl_elems;` | 2 | rule 5b premise: `config` handed to `rd_list_cnt` (the cast alone is `Safe`; the uncast replica is refused too) | W48-POINTER-RETURN-THROUGH-CALL-ACTUAL-ROOT (amended) |
| `if (!cJSON_IsString(item)) return NULL; return item->valuestring;` | 1 | rule 5b premise: `item` handed to a run-defined read-only callee | W48-POINTER-RETURN-THROUGH-CALL-ACTUAL-ROOT (amended) |
| `switch (rkev ? rkev->rko_evtype : 0x0)` on a typedef base | 1 | `no value form of this card` (the `struct` spelling and a temporary are `Safe`) | K6-SWITCH-CONDITIONAL-TYPEDEF-BASE |
| `if (_Generic(…, strstr(rktrans->…mechanisms, "GSSAPI")))` | 1 | rule 10z refuses `_Generic` by name | CG06-VALUE-CONNECTIVE-RESIDUALS (amended) |
| `int idle = (int)((rd_clock() - (tx_last > 0 ? tx_last : rkb->rkb_ts_state)) / 1000000);` | 1 | rule 10z (3): a sibling call | refused on purpose; CG06-VALUE-CONNECTIVE-RESIDUALS (amended) |
| `size_t const bufferSize = (size_t)(hc4->end - hc4->prefixStart);` | 1 | rule 78 reads no pointer difference | refused on purpose; K6-POINTER-DIFFERENCE-SCALAR |
| `unit_test_data.type = …;` (file-scope `static struct`) | 2 | rule 12j: no file-scope root (a read and a condition are `Safe`) | refused on purpose; K6-FILE-SCOPE-RECORD-SCALAR-STORE |
| `rktp = (const T *)rktpar; return rktp->…->str;`, `const T *a = _a, *b = _b; … al = a->value` | 2 | a formal copied into a local (rule 77 excludes it) | refused on purpose |

The `(&X)->f` spelling is far commoner than its three freed rows: 192 of the
K6 leaf groups (153 statement, 38 condition, 1 return) on 125 functions spell
it on their line, mostly `mk_list_foreach`/`cfl_list_foreach`/`TAILQ_FOREACH`
expansions in functions with other blockers
([scripts/arrow_census.py](scripts/arrow_census.py),
[census/arrow-census.json](census/arrow-census.json)).

## 3. Chosen shape and why it is sound: rule 81

[Rule 81](../../agents/gateway.md#constraint-81)
(`crates/qcore-c-gateway/src/address_arrow_place.rs`). Rule 10n (1) reads
`(&X)->f` as `X.f` in the typed access-path reader; two readers did not:

1. **The lexical `may_bind` reader** (`typed_field_access::bound_names`,
   rule 5b's premise for a pointer handed out of the body, rule 10k (2)) read
   the `& p` of `( & p -> q ) -> f` as an address of `p`. For the occurrence
   `#a` of a single `&` before a name and its plain spelling `#plain(#a)`:
   `#exempt(#a) == TRUE -> #binds(#a) == #binds(#plain(#a))`, with
   `#exempt(#a) == TRUE` exactly when
   `#opens_group(#a) == TRUE && #name_arrow(#a) == TRUE && #selection_chain(#a) == TRUE && #group_arrow(#a) == TRUE && #outer_address(#a) == FALSE`;
   and `#outer_address(#a) == TRUE -> #binds(#a) == TRUE`,
   `#name_arrow(#a) == FALSE -> #binds(#a) == TRUE`.
2. **The elaborator's place reader** (`elaboration::path_place`) refused a
   path through the `&`. For an arrow member `#e` over the address of a member
   or subscript place:
   `#arrow_over_member_address(#e) == TRUE -> (#place(#e) == #place(#plain(#e)) && #root(#e) == #root(#plain(#e)))`,
   through rule 10n's own `arrow_through_address` (one shared item).

**Soundness.** `E1->E2` is `(*(E1)).E2` (C11 6.5.2.3p4), `*&E` is `E`
(6.5.3.2p3) and a parenthesised expression is the expression it encloses
(6.5.1p5), so `(&X)->f` designates `X.f`. A postfix operator binds tighter
than a unary one (6.5.2, 6.5.3), so the `&` of `& p -> q` takes the member
place `p->q`, never `p`, and the following `->` dereferences that address at
once: it is no value the program keeps. Each reader now reads exactly what it
reads for the plain spelling `p->q.f`, which the lexical reader never binds
(no `&` precedes `p`) and the place reader reads (root `p`, indirect). The
AST reader (`typed_root_declarations`) already answered `p` not bound. An
outer `&` (`&((&p->q)->f)` is `&p->q.f`) keeps the binding, as the plain arm
binds `&p->q.f`; so does a whole-object address (`(&p)->f`), a group with an
operator (`(&p->q + 1)->f`), `& p . q` and a group not followed by `->`. The
read keeps every per-hop obligation of the plain spelling, so nothing is
discharged: a read after a release is the same `UseAfterRelease`, a rebound,
consumed or addressed root keeps its refusal.

**Lean.** `QCore.AddressArrowPlace` (`proof/AuditAddressArrowPlace.lean`, 12
theorems; axioms `propext`, `Quot.sound` only, no project axiom): the place
reader is C's designation (`new_place_is_c_place`), keeps every old answer
(`the_rule_only_adds`), reads `(&X)->f` as `X.f` with the same root
(`arrow_through_address_is_dot`, `same_root_as_plain`), lifts only an arrow
over an address (`a_lifted_refusal_is_an_arrow_over_an_address`); the
`may_bind` arm only removes (`the_rule_only_removes`), never binds less than
the plain spelling (`at_least_the_plain_reader`), agrees with it on every
dereferenced group (`agrees_with_the_plain_spelling`), exempts only a member
address (`an_exempted_address_is_a_member_address`), keeps a whole address
and an outer address (`a_whole_address_is_bound`,
`an_outer_address_keeps_the_binding`); `lexical_vectors_hold` replays the
Rust `LEXICAL_VECTORS`.

**Formulas.** The statements above are read by the premise-formula reader
(`crates/qcore-frontend/tests/rule81_formulas.rs`: `parse_tokens` accepts each
and its canonical printing reparses). The sort step cannot accept them: the
closed vocabulary registers contract-premise terms only, no gateway-reading
term and no `TRUE`/`FALSE` constant; the test pins that every rejection is an
unknown term or constant (follow-up K6-RULE-FORMULA-VOCABULARY).

**Fixtures** (`crates/qcore-c-gateway/tests/k6_direct_position_access.rs`, 4
tests, every row's status and full key set, every near-miss beside its plain
spelling): admitted — the `TAILQ_FIRST` return and a null test of it, the
`RD_KAFKAP_BYTES_LEN` assignment, the `RD_KAFKAP_STR_LEN` declaration over an
element, an owned local read before its release; refused or accused alike in
both spellings — a root released before the return (`unknown_alias:m`),
handed to an uncontracted callee, an outer address returned, another statement
taking the member's address (and the outer-address form of it), a release
before the conditional read, a loop rebinding the root, and an owned local
read after its release (`Unsafe`, `UseAfterRelease`, in both spellings); the
one pair that differs is a rebound formal (`m = k_alloc(8)`): the plain return
is accused of the leak by an older route, the arrow spelling keeps the return
refusal (never `Safe`). With `ARM = false` (checked once by hand) the three
admitting tests fail with the base's refusals and the loop row regains its
gate leaf. Unit tests: `address_arrow_place::tests` (the decision vectors,
`&&`, the place reader against the plain spelling, a whole-object address and
a `.` over an address refused).

## 4. Refused shapes and their named reasons

Rows freed alone on 11-24, by the formula of their refusal (`#r` the row):

| Shape | Rows | Refusal | Owner |
|---|---:|---|---|
| interior address returned | 10 | `#returns_interior(#r) == TRUE -> #result_derived(#r) == TRUE` (a `None` result would let a caller release the base) | W45-RETURN-POINTER-RESULT-READS, AG3-RETURN-MEMBER-ADDRESS-DERIVED |
| root handed to a callee before a pointer return | 3 | `#handed(#root) == TRUE -> #may_bind(#root) == TRUE` | W48-POINTER-RETURN-THROUGH-CALL-ACTUAL-ROOT |
| explicit cast of a returned load | 1 | `#explicit_cast(#value) == TRUE -> #value_form(#value) == FALSE` | K6-RETURN-IDENTITY-CAST |
| switch over a `?:`, typedef base | 1 | `#switch_conditional(#c) == TRUE -> #value_form(#c) == FALSE` | K6-SWITCH-CONDITIONAL-TYPEDEF-BASE |
| `_Generic` in a condition; a sibling call beside a deciding operand | 2 | `#generic(#region) == TRUE -> #lowered(#region) == FALSE`; `#sibling_call(#region) == TRUE -> #lowered(#region) == FALSE` | CG06-VALUE-CONNECTIVE-RESIDUALS |
| pointer difference | 1 | `#pointer_difference(#e) == TRUE -> #place_read(#e) == FALSE` | K6-POINTER-DIFFERENCE-SCALAR |
| file-scope record scalar store | 2 | `#storage(#root) == STATIC -> #admitted_12j(#root) == FALSE` | K6-FILE-SCOPE-RECORD-SCALAR-STORE |
| formal copied into a local | 2 | `#writes(#l) == {#whole_read(#formal)} -> #admitted_77(#l) == FALSE` | (rule 77, by design) |

## 5. Shared-file hook points

| File | Hook |
|---|---|
| `crates/qcore-c-gateway/src/typed_field_access.rs` | `is_written_at`'s address arm: one added conjunct `&& !crate::address_arrow_place::dereferenced_address_group(body, index - 1)`, between `K6-DIRECT-POSITION-ACCESS hook (rule 81)` comments |
| `crates/qcore-c-gateway/src/elaboration.rs` | `path_place`'s `MemberExpr` arm: the base read through `typed_position_shapes::arrow_through_address` when `address_arrow_place::ARM`, between the same comments |
| `crates/qcore-c-gateway/src/lib.rs` | `mod address_arrow_place;` before `mod aggregate_store;` |
| `proof/QCore.lean` | `import QCore.AddressArrowPlace` (alphabetical, after `AbiEnumConstPointer`) |
| `docs/agents/rules/10k.md`, `10n.md` | one "Amended by rule 81" sentence each; `gateway.md`/`README.md` regenerated |

Of the six files the brief names as shared, only `typed_field_access.rs` is
touched (the one hook above); `legacy_c.rs`, `legacy_cfg_builder.rs`,
`legacy_transfer.rs`, `legacy_classify.rs` and `typed_access_path.rs` are
not, nor is `typed_position_shapes.rs` (its `arrow_through_address` is called,
unchanged). New files:
`address_arrow_place.rs`, `tests/k6_direct_position_access.rs`,
`crates/qcore-frontend/tests/rule81_formulas.rs`,
`proof/QCore/AddressArrowPlace.lean`, `proof/AuditAddressArrowPlace.lean`,
`docs/agents/rules/81.md`.


## 6. Measurement

No measurement of an implementation build is part of this card: the
integrator measures the merged round-37 tip (corpus eight-home, twelve-home,
system, slice, then `current`) against [PREDICTION.md](PREDICTION.md). A
unit-scoped reproduction of the 17 units of the 25 rows
([census/repro-units.txt](census/repro-units.txt), the user's `current
--monkey-linked` argv over the filtered compile database and the full Monkey
database, [scripts/run_repro.sh](scripts/run_repro.sh), parity stream on) was
queued for the base binary (`private/k6/bin/quirin-audit-base`, `5087f9295`)
and the tip binary (`private/k6/bin/quirin-audit-tip1`, the rule-81 analyzer
code of `c674d3108` before formatting, its diff in
`private/k6/bin/quirin-audit-tip1.diff`); section 6.1 records what it gave.

## 7. Targeted checks (no card gate)

No card gate (round-37 brief: one gate on the merged tip). Each check ran in
a gate slot on the committed code of `c674d3108` (CARGO_TARGET_DIR
`private/k6/target-test`), logs and exits under `private/k6/`:

| Check | Command | Exit | Result |
|---|---|---:|---|
| fixtures and every test file of the amended rules or carrying the spelling | `scripts/targeted-tests.sh t3` (`cargo test -p qcore-c-gateway` over 17 `--test` files: k6_direct_position_access, access_path_positions, access_path_position_residuals, cg06_value_connectives, cg06_connective_footprints, cg02_elaboration, cg05_elaborator_call_footprints, cg11_pointer_stores, reach_gate_positions, return_position_pointer_leaf, return_position_modeled_call, neutral_expression_positions, intrusive_list_iteration, k1_call_actual_access, k5_local_origin_aliases, k7_residual_causes, parenthesised_callee) | 0 | 191 passed, 0 failed (`t3-fixtures.log`) |
| unit tests | the same script, `cargo test -p qcore-c-gateway --lib -- address_arrow_place elaboration typed_field_access typed_position_shapes typed_root_declarations readonly_actual_frame value_connective` | 0 | 73 passed (`t3-lib.log`) |
| src unit tests spelling `(&p->q)->f` | `cargo test -p qcore-c-gateway --lib -- legacy_list_iteration pointer_store typed_dispatch_census` | 0 | 12 passed (`t2-lib.log`) |
| qcore-audit test spelling it | `cargo test -p qcore-audit --test loop_measure_selection` | 0 | 3 passed (`t2-audit.log`) |
| formula statements | `cargo test -p qcore-frontend --test rule81_formulas` | 0 | 2 passed (`formulas-1.log`) |
| Clippy | `cargo clippy -p qcore-c-gateway --all-targets -- -D warnings` | 0 | (`clippy-1.log`) |
| fmt | `cargo fmt --all -- --check` | 0 | (`fmt-5.log`) |
| Lean | `scripts/gate_lean.py --module QCore.AddressArrowPlace --module QCore --audit AuditAddressArrowPlace.lean` | 0 | 2 targets built, 1 audit passed; axioms `propext`, `Quot.sound` only (`gate-lean-1.log`) |
| rule docs | `python3 scripts/gen_rule_docs.py` | 0 | `gateway.md`, `README.md` regenerated |
| amendments | `python3 scripts/apply_amendments.py docs/reports/k6-direct-position-access/integration-amendments.json --check` | 0 | valid (7 amends, 5 registers) |

Rule off (`address_arrow_place::ARM = false`, once by hand, not committed):
the three admitting fixture tests fail with the base refusals
(`private/k6/fixture-off.txt`).

## 8. Follow-ups

In [integration-amendments.json](integration-amendments.json): the three
owners (W47-ACCESS-PATH-POSITION-RESIDUALS, CG10-BOUND-ROOT-PATH-REFUSED,
CG10-BOUND-ROOT-NULLABLE-OR-TRACKED) amended with the 11-24 figures and rule
81; W45-RETURN-POINTER-RESULT-READS, AG3-RETURN-MEMBER-ADDRESS-DERIVED,
W48-POINTER-RETURN-THROUGH-CALL-ACTUAL-ROOT and CG06-VALUE-CONNECTIVE-RESIDUALS
amended with their rows; new: K6-RETURN-IDENTITY-CAST,
K6-SWITCH-CONDITIONAL-TYPEDEF-BASE, K6-FILE-SCOPE-RECORD-SCALAR-STORE (a user
decision on rule 12j's storage boundary), K6-POINTER-DIFFERENCE-SCALAR,
K6-RULE-FORMULA-VOCABULARY (the closed premise vocabulary has no
gateway-reading terms, so rule conditions are syntax-checked only).
