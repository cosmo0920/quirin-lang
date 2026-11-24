# R34-K-REPAIR: the combined K tip's six pin failures and three unpredicted moves

Round 34 repair (2026-11-25), branch `r34-k-repair`, base `c5cd10688` (the
combined tip of SP10 + K1/K3/K5/K7, `integrate/r34-k-batch`, on origin/main
`4be28c3cd`). Inputs, read only: the landing gate
`private/gate-full-c5cd10688/` of the integrator's landing worktree (Rust 695
units, 10,097 passed, 7 failed; the libcurl `curlver.h` failure is the known
host failure) and the landing measurement
`private/measure/r34-landing/native-comparison.json` of the main checkout.

**Outcome.** Every Safe reading is sound and is covered exactly by the stated
admission of the rule that produces it. The six pins are K5's (gateway rule
77) and are re-pinned, each with a near-miss that stays refused. The three
unpredicted corpus moves are two K1 rows (rule 75) and one K5 row (rule 77),
with no interaction between the cards. No gateway code, contract or Lean module
changes. The integrator expected the three K3 twelve-home rows; they are
confirmed as K3's declared corpus rows.

Notation. The conditions are formulas over the rules' readings
(`borrowed_path_local.rs`, `parenthesised_member_operand.rs`):

- `root(r)`: rule 77 (1)'s admitted formal;
- `auto(l)`: rule 62's automatic object-pointer local with one declaration;
- `writes(l)` and `uses(l)`: the kinds rule 77 (2) reads (`Written`, `Use`);
- `l ∈ bp`: membership of the greatest fixed point `TraversalNames::borrowed_path`;
- `untracked(l) ⇔ targets(l) = ∅ ∧ derivation(l) = identity ∧ ¬widened`.

Rule 77 (3) is `guard(l) = Refined ⇔ l ∈ bp ∧ untracked(l) ∧ l ∉
witnesses`. Rule 77 (4) is `escape(return l) = ∅ ⇔ l ∈ bp ∧ untracked(l)`.
Rule 75 is `seed(& ( p ) -> f) = ∅`.

## 1. Attribution

**Method.** Each card tip (K1 `a5d07e184`, K3 `334513e64`, K5 `ad6b12a75`,
K7 `7434d6f77`, all on `3ca85c7a6`), SP10's `dda6fe512`, the base
`3ca85c7a6` and the combined base `4be28c3cd` were checked out as temporary
detached worktrees under `private/tips/`. The six failing test binaries ran at
each tip ([scripts/tip_tests.sh](scripts/tip_tests.sh)) through
`gate_slot.py build`.

A first pass shared one `CARGO_TARGET_DIR` across tips. Cargo's metadata hash
does not depend on the path, and its freshness check uses mtimes, so the next
tips reused the previous tip's library ("Finished … 0.01s"). Those runs were
discarded. Every run reported here used a target directory of its own and
compiled the tip's `qcore-c-gateway` from that tip's path
([evidence/tip-tests.txt](evidence/tip-tests.txt)).

The corpus moves were reproduced with a unit-scoped run of the user's
`current` route ([scripts/run_repro.sh](scripts/run_repro.sh), after K5's
`run_repro.sh`). The run used the user's prepared tree (Fluent Bit `046accf6`;
the three bodies are the same at the corpus pin `66910c10`). The compile
database was filtered to the seven units in
[evidence/repro-units.txt](evidence/repro-units.txt): the two librdkafka
units, `lib/monkey/mk_core/mk_string.c` and one unit of each library the route
requires. A second run linked a Monkey database filtered to `mk_string.c` and
the required `mk_event.c`. The run was made at five points:

- the base `3ca85c7a6`, K1 and K5, release binaries built at each tip;
- `4be28c3cd` and `c5cd10688`, the measurement's own base and tip binaries.

The binaries and their sha256 are in
[evidence/repro-binaries.txt](evidence/repro-binaries.txt). Fidelity: at
`4be28c3cd` each row carries exactly the leaves the measurement's base side
removed, and the combined binary moves exactly the measured rows of these
units. There is no other move ([evidence/repro-compare.txt](evidence/repro-compare.txt)).

| Item | Base `3ca85c7a6` | K1 | K3 | K5 | K7 | `4be28c3cd` | SP10 | Combined | First Safe at |
|---|---|---|---|---|---|---|---|---|---|
| 1 `access_path_positions::an_assignment_operand_is_a_borrowed_read_then_the_null_branch` | pass | pass | pass | **Safe** | pass | pass | pass | Safe | K5 |
| 2 `typed_root_declarations::an_assignment_operand_reads_the_ast_declared_local` | pass | pass | pass | **Safe** | pass | pass | pass | Safe | K5 |
| 3 `lexical_closure_nested_assignment::a_never_bound_root_still_certifies` | pass | pass | pass | **Safe** | pass | pass | pass | Safe | K5 |
| 4 `typed_const_pointer_value::a_void_pointer_bit_cast_is_a_borrowed_read` | pass | pass | pass | **Safe** | pass | pass | pass | Safe | K5 |
| 5 `scalar_read_identity::pointer_destination_retains_opaque_identity_refusal` | pass | pass | pass | **Safe** | pass | pass | pass | Safe | K5 |
| 6 `detach_lookup_membership::production_lookup_baseline_refuses_the_unresolved_returned_cursor` | pass | pass | pass | **Safe** | pass | pass | pass | Safe | K5 |
| 7 `rdkafka_broker.c::rd_kafka_broker_destroy_error` | Unknown | **Safe** | | Unknown | | Unknown | | Safe | K1 |
| 8 `rdkafka_partition.c::rd_kafka_toppar_is_on_cgrp` | Unknown | **Safe** | | Unknown | | Unknown | | Safe | K1 |
| 9 `mk_core/mk_string.c::mk_string_html_escape` | Unknown | Unknown | | **Safe** | | Unknown | | Safe | K5 |

The K3 and K7 rows 7–9 were not reproduced: those cards change neither rule
75's seed nor rule 77's reading. The combined moves equal the union of K1's
moves and K5's moves in both reproductions, so no item needs two cards.

K3's twelve-home rows `buf_read` (`pb_decode.c:71`),
`writer_set_expected_length` (`snappy.c:490`) and `mk_iov_consume`
(`mk_iov.c:191`) are K3's three declared corpus moves. Twelve-home analyses
the same pinned units under another link map. The measurement removes the same
single `indirect memory access` leaf from each row on both routes. K3's own
unit reproduction holds them (its README §7). The integrator was right to
expect them on twelve-home.

## 2. Decisions

All nine items are **sound and intended**. Each pin predates K5 and pinned
rule 10b's identity refusal, which is exactly the arm that rule 77 (3) amends.
None of them pinned a refusal for another reason:

- the cast in item 4 was already admitted at the base: the declared form
  `struct c_leaf *l = b->conf.opaque; c_peek(l);` was `Safe` there;
- the `OpaqueValue` binding in item 5 is, by its own definition, the binding
  of an `Alias` from an unbound place;
- item 6's refusal was the single `UnknownAlias current_child` at `return
  current_child;`.

Rule 77's text forbids four admissions: a value from a call, a whole formal,
an address or array decay, and a static object. No item reaches any of them.
Rule 77 also says not to widen (3) to rule 10b in general, because "a lookup's
result is no load". No item does that either: none of the six tested values
comes from a call.

1. **Item 1.** Body: `struct c_item *it; if (!(it = b->item)) return 0;
   return 1;`.
   - Admission: `root(b) ∧ auto(it) ∧ writes(it) = {load(b->item)} ∧
     uses(it) = {nullTest}`, so `it ∈ bp ∧ untracked(it)` and the guard is
     `Refined` (clauses (1)–(3)).
   - Why sound: `it` holds the value `b->item` has, and the body takes no
     custody step on it.
   - Re-pinned to `Safe`. Near-miss: `struct c_item **at = &it;` gives
     `Address ∈ uses(it)`, then `decide(Local, Address) = Withhold`, then
     `it ∉ bp`. The test stays `Unknown [unknown_alias:it, unsupported
     assignment expression]`; the second leaf is the `&it` initialiser's.
2. **Item 2.** The same body, with the AST reader's typed local (the
   `int (*cb[1])(int)` trigger).
   - Admission: the same as item 1. Re-pinned to `Safe`.
   - Near-misses, already in the test: a `static` local gives `storage(it) =
     static`, so `it ∉ bp`; a shadowed local gives `|decls(it)| = 2`, so `it
     ∉ bp`. Both still assert a `branch condition` refusal and both pass.
3. **Item 3.** Body: `if ((it = b->item)) c_use(it->count); return
   b->item->count;`.
   - Admission: `writes(it) = {load(b->item)} ∧ uses(it) = {nullTest, deref}`,
     so `it ∈ bp`. `c_use` is borrowed, so no custody use exists.
   - Re-pinned to `Safe`. Near-miss: `&it`, so `it ∉ bp`. The test stays
     `Unknown [unknown_alias:it, return expression, unsupported assignment
     expression]`.
4. **Item 4.** Body: `const struct c_leaf *l; if (!(l = b->opaque)) return
   -1; return l->weight;`.
   - Admission: `writes(l) = {BitCast(load(b->opaque))}`. Clause (2) reads
     through `NoOp`/`BitCast`. `objectPointer(void *)` holds: `void` is an
     object type (C11 6.2.5p19), and `custody_free_copy::object_pointer`
     accepts it. Then `uses(l) = {nullTest, deref}`, so `l ∈ bp`.
   - Why sound: converting `void *` to an object pointer keeps the pointer
     value (C11 6.3.2.3p1), so the tested value is the loaded one. Whether
     the pointee's type matches is a property outside resource custody, as
     the base already treated it.
   - Re-pinned to `Safe`. Near-miss: `&l`, so `l ∉ bp`. The test stays
     `Unknown [unknown_alias:l, return expression, unsupported assignment
     expression]`.
5. **Item 5.** Body: `void *p = ch->st->pointer; if (p == 0) return 1;
   return 0;`.
   - Admission: `root(ch) ∧ writes(p) = {load(ch->st->pointer)}`, where the
     base `ch->st` is itself a load on the root. `uses(p) = {nullTest}`, so
     `p ∈ bp`. The `OpaqueValue` binding (`targets = ∅`, `unknown`,
     `may_be_null`, `Identity`) satisfies `untracked(p)`.
   - Re-pinned to `Safe`, and the lowering assertion (`OpaqueValue { p }`) is
     kept. The test is renamed `pointer_destination_load_is_decided_at_its_null_test`
     because the old name stated the old verdict.
   - Near-miss: `void **at = &p;`, so `p ∉ bp`. The test stays not `Safe`,
     carries `UnknownAlias p` and keeps the `OpaqueValue` lowering.
6. **Item 6.** The retained cJSON `get_array_item`, on the single-function
   route `analyze_legacy_c`.
   - Admission: `root(array)`, whose uses are `{nullTest, deref}`. Then
     `writes(current_child) = {null, load(array->child),
     load(current_child->next)}` and `uses(current_child) ⊆ {nullTest, deref,
     rebind, return}`, so `current_child ∈ bp`. At the return
     `untracked(current_child)` holds, and clause (4) gives `escape = ∅`,
     rule 10k (2)'s answer for `return R->F;`. The `L = L->next` cursor is
     named in rule 77's module documentation.
   - Why sound: the returned pointer is a value loaded through the
     caller's list, and the function takes no custody step.
   - No other summary changes. The checked translation-unit route still
     validates its Borrowed, parent-0 result
     (`checked_lookup_unit_reports_borrowed_result_without_fresh_identity`
     passes), and the corpus already had `get_array_item` `Safe` through that
     route, so no caller moves.
   - Re-pinned to `Safe` with no diagnostic. The test is renamed
     `production_lookup_baseline_reads_the_loaded_cursor`.
   - Near-miss: the advance `current_child = current_child->next + 0;` gives
     `Other ∈ writes(current_child)`, so `current_child ∉ bp`. The test stays
     `Unknown` with a `current_child` leaf.
7. **Item 7** (K1, rule 75). `return rd_kafka_terminating(rk) ? A : B;`
   expands to `rd_atomic32_get(&(rk)->rk_terminate) & …`.
   - Admission: `seed(& ( rk ) -> rk_terminate) = ∅`, so `rk ∉ bound`. The
     actual is the member address `&rk->rk_terminate` at a borrowed position
     (`rd_atomic32_get`: `resource_borrowed(ra)`, `return_none`).
   - Why sound: C11 6.5.2 and 6.5.3 make postfix `->` bind tighter than unary
     `&`, and 6.5.1p5 makes `(rk)` the expression `rk`.
   - This is K1's declared `current` move. K1's forecast "no move on corpus"
     rested on the false premise "no librdkafka unit". Recorded as a corpus
     and twelve-home move of K1.
8. **Item 8** (K1, rule 75). `rd_kafka_toppar_lock(rktp)` is
   `mtx_lock(&(rktp)->rktp_lock)`, at lines 2766 and 2772.
   - Admission: `seed(& ( rktp ) -> rktp_lock) = ∅`. With no other seed,
     `rktp ∉ bound`, so the scalar read `rktp->rktp_flags` at line 2768 needs
     no identity. Its `unknown_alias rktp` leaf was the closure's consequence
     of the seed, not an independent blocker.
   - `mtx_lock` and `mtx_unlock` are `resource_borrowed(mutex)`, so the lock
     is no custody resource in `resource-custody.v1`, and the paths that
     depend on `do_lock` carry no obligation.
   - Recorded as a corpus and twelve-home move of K1.
9. **Item 9** (K5, rule 77). `const char *escape;` is written only `NULL` and
   string literals, and is used in `if (escape)`, `strcpy(p, escape)` and
   `strlen(escape)`.
   - Admission: `writes(escape) ⊆ {null, literal} ∧ uses(escape) ⊆
     {nullTest, callArgument}`, so `escape ∈ bp ∧ untracked(escape)`, and the
     guard at line 510 is decided. A literal's address is a static array (C11
     6.4.5p6), which no custody step touches.
   - The second leaf (line 462, ``possibly-null resource
     `__qcore_returned_value` returned under a non-null result mode``) is rule
     74d's name for a nullable return in the contract-free run. That run is
     used only when no in-unit summary validates (`legacy_nullable_return.rs`).
     Once the guard is decided, the body carries no refusal, and the summary
     validates as `Owned { nullable: true }`. Then `return mk_string_dup("")`
     (an owned, nullable result) and `return NULL` are permitted, and callers
     must test the result. So the leaf disappears as a consequence of item 9,
     and no obligation is dropped.
   - Recorded as a corpus and twelve-home move of K5.

No item is unsound or outside its rule's admission, so no gateway change is
made, no predicted move is lost and no Lean theorem changes. The readings of
rules 75 and 77 are unchanged; `QCore.ParenthesisedMemberOperand` and
`QCore.BorrowedPathLocal` stand as landed.

## 3. Fixture changes

Only test files change: six assertions are re-pinned, and five near-misses are
added beside them; item 2 keeps its two existing near-misses. Each re-pin
carries a comment that names rule 77, the clauses that admit it and the BEFORE
verdict.

| File | Test | Change |
|---|---|---|
| `tests/access_path_positions.rs` | `an_assignment_operand_is_a_borrowed_read_then_the_null_branch` | `Safe`; near-miss `&it` |
| `tests/typed_root_declarations.rs` | `an_assignment_operand_reads_the_ast_declared_local` | `Safe`; the `static` and shadowed near-misses kept |
| `tests/lexical_closure_nested_assignment.rs` | `a_never_bound_root_still_certifies` | `Safe`; near-miss `&it` |
| `tests/typed_const_pointer_value.rs` | `a_void_pointer_bit_cast_is_a_borrowed_read` | `Safe`; near-miss `&l` |
| `tests/scalar_read_identity.rs` | `pointer_destination_load_is_decided_at_its_null_test` (renamed) | `Safe` with `OpaqueValue` kept; near-miss `&p` |
| `tests/detach_lookup_membership.rs` | `production_lookup_baseline_reads_the_loaded_cursor` (renamed) | `Safe`, no diagnostics; near-miss arithmetic advance |

Records amended: the K5 report gains §10 (the re-pins with their admissions).
A post-measurement amendment section is added to K1's and K5's PREDICTION.md;
each frozen block is unchanged.

## 4. Corrected combined prediction

These rows are added to `private/measure/r34-combined-PREDICTION.md`, by
card:

| Card | Route | Row |
|---|---|---|
| K1 | corpus, twelve-home | `rdkafka_broker.c::rd_kafka_broker_destroy_error Unknown -> Safe` |
| K1 | corpus, twelve-home | `rdkafka_partition.c::rd_kafka_toppar_is_on_cgrp Unknown -> Safe` |
| K5 | corpus, twelve-home | `mk_core/mk_string.c::mk_string_html_escape Unknown -> Safe` |
| K3 | twelve-home | `buf_read`, `writer_set_expected_length`, `mk_iov_consume Unknown -> Safe` (K3's corpus rows on the twelve-home route) |

The corrected block, which matches the measurement exactly (corpus Safe +16,
twelve-home Safe +16, system and slice none):

```measure-prediction
route corpus
move buf_read Unknown -> Safe
move writer_set_expected_length Unknown -> Safe
move mk_iov_consume Unknown -> Safe
move cJSON.c::cJSON_GetArraySize Unknown -> Safe
move cio_file.c::format_acl_error_message Unknown -> Safe
move cmt_encode_opentelemetry_batch.c::request_data_point_count Unknown -> Safe
move rdkafka_partition.c::rd_kafka_topic_partition_get_current_leader_epoch Unknown -> Safe
move rdkafka_partition.c::rd_kafka_topic_partition_get_leader_epoch Unknown -> Safe
move rdkafka_partition.c::rd_kafka_toppar_get0 Unknown -> Safe
move rdkafka_sasl_cyrus.c::rd_kafka_sasl_cyrus_ready Unknown -> Safe
move snappy.c::get_hash_table Unknown -> Safe
move cmt_get_statsd_type Unknown -> Safe
move buffer_skip_whitespace Unknown -> Safe
move rdkafka_broker.c::rd_kafka_broker_destroy_error Unknown -> Safe
move rdkafka_partition.c::rd_kafka_toppar_is_on_cgrp Unknown -> Safe
move mk_string.c::mk_string_html_escape Unknown -> Safe
delta Safe +16
route twelve-home
move buf_read Unknown -> Safe
move writer_set_expected_length Unknown -> Safe
move mk_iov_consume Unknown -> Safe
move cJSON.c::cJSON_GetArraySize Unknown -> Safe
move cio_file.c::format_acl_error_message Unknown -> Safe
move cmt_encode_opentelemetry_batch.c::request_data_point_count Unknown -> Safe
move rdkafka_partition.c::rd_kafka_topic_partition_get_current_leader_epoch Unknown -> Safe
move rdkafka_partition.c::rd_kafka_topic_partition_get_leader_epoch Unknown -> Safe
move rdkafka_partition.c::rd_kafka_toppar_get0 Unknown -> Safe
move rdkafka_sasl_cyrus.c::rd_kafka_sasl_cyrus_ready Unknown -> Safe
move snappy.c::get_hash_table Unknown -> Safe
move cmt_get_statsd_type Unknown -> Safe
move buffer_skip_whitespace Unknown -> Safe
move rdkafka_broker.c::rd_kafka_broker_destroy_error Unknown -> Safe
move rdkafka_partition.c::rd_kafka_toppar_is_on_cgrp Unknown -> Safe
move mk_string.c::mk_string_html_escape Unknown -> Safe
route system
none
route slice
none
```

The `current` forecast also gains `rd_kafka_toppar_is_on_cgrp` (K1) and
`mk_string_html_escape` (K5): both rows carry, on the user's 11-24 report,
the same leaves the reproduction removes.

## 5. Checks

No full gate: the integrator reruns the one landing gate. Each heavy job ran
once, in the background, through `scripts/gate_slot.py`. Logs are under
`private/repair/` in this worktree.

| Check | Command | Exit | Result |
|---|---|---:|---|
| attribution, six binaries at each tip | `gate_slot.py build -- bash scripts/tip_tests.sh <tip> <label>` (own target directory each) | K5 101, others 0 | at K5 exactly the six fail; base, K1, K3, K7, `4be28c3cd` and SP10 pass all 45 tests of the six files |
| re-pinned files and the K files | `cargo test -p qcore-c-gateway --test access_path_positions --test typed_root_declarations --test lexical_closure_nested_assignment --test typed_const_pointer_value --test scalar_read_identity --test detach_lookup_membership --test k1_call_actual_access --test k3_pointer_valued_stores --test k5_local_origin_aliases --test k7_residual_causes` | 0 | 67 passed, 0 failed (12, 8, 5, 5, 9, 3, 4, 6, 7, 8) |
| Clippy | `cargo clippy -p qcore-c-gateway --all-targets -- -D warnings` | 0 | |
| fmt | `cargo fmt --all -- --check` | 0 | after `cargo fmt --all` reflowed one new assertion |
| reproduction | `gate_slot.py audit --memory-budget 7G -- bash scripts/repro_pair.sh …` (without and with Monkey) at base, K1, K5, `4be28c3cd`, `c5cd10688` | 0 | [evidence/repro-compare.txt](evidence/repro-compare.txt) |
| Lean | none touched | | no `gate_lean.py` run: no module changes |
| amendments | `python3 scripts/apply_amendments.py docs/reports/r34-k-repair/integration-amendments.json --check` | 0 | |

No module source changed, so no unit test was added.

## 6. Records and follow-ups

[integration-amendments.json](integration-amendments.json) carries the
increment paragraph only. No sub-shape is left, so no follow-up is amended or
registered. Two process notes for the integrator:

- K1's and K5's forecasts missed rows whose second leaf was a consequence of
  the card's own refusal (items 8 and 9). A forecast that counts only rows
  "freed alone" undercounts these. This is a forecasting note, not a rule gap.
- Per-tip builds need their own `CARGO_TARGET_DIR` (§1).
