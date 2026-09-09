# W4-EXPR-POS — neutral expression positions, environment-aware — 2026-09-09

Implements [`docs/design/W4_NEUTRAL_EXPRESSION_POSITIONS.md`](../../design/W4_NEUTRAL_EXPRESSION_POSITIONS.md),
the disposition of card [C03-REACH](../remaining-c03-reach/README.md) step (3).

**Baseline:** main `67961f1` (wave-11 merged tree), pinned corpus artifacts
[`docs/reports/wave11-corpus/artifacts/`](../wave11-corpus/artifacts).
**Branch:** `claude/w4-expr-pos`.

**Headline: nothing is promoted.** The pinned 185-unit corpus stays
**2 Safe / 42 provisional Unsafe / 137 Unknown / 4 data-only** at every one of
the three measured checkpoints, with **no unit verdict move in either
direction** and **no newly surfaced provisional `Unsafe` unit**. Twenty-eight
functions move `Unknown` → `Safe` at function level, each attributed to its
rule in §5. The neutral-frame recursive route's corpus reach moves **0 → 1**:
`digits10`, the design note's §6 minimal validation case.

---

## 1. The architecture change

The design note's §5 states the problem precisely: the premise the widening
needs — the contract environment — is *already computed*, and is *already
handed to* `build_function_cfg`; it simply is not reachable from
`supported_neutral_expression`, a token-level free function called during
parsing. Threading it there is the work.

The predicate is **not** loosened where it lives. It is byte-identical.
`Parser` gains one method:

```rust
fn supported_expression(&self, tokens: &[Token]) -> bool {
    if supported_neutral_expression(tokens) {
        return true;                     // unchanged, and consulted first
    }
    crate::neutral_position::is_neutral_expression(
        &texts, self.contracts, &self.typed_model, &self.typed_types)
}
```

`Parser` already carries `contracts: &'a ResourceContracts`, so the
environment reaches the classification without changing a single call
signature. Because the lexical predicate is consulted **first**, the admission
is *strictly additive*: an expression this project accepted before is accepted
by the same code, for the same reason, and a `true` from the new rule is always
a new admission.

Five of the six positions the note names now use the method
(`if`, `while`, `for`, `do`/`while`, `switch`); the sixth, `return`, landed
first and alone, as §6 asks. A seventh call site, the `case` label, keeps the
strict lexical predicate — see §8, `W4-EXPR-POS-CASE`.

## 2. The three admitted families

`crates/qcore-c-gateway/src/neutral_position.rs` (new). Each family is a
conjunction and each is typed.

**F1 — the neutral call.** A call `f ( … )` is resource-neutral when

1. `f` is a plain declared name, not a reserved word, not preceded by `.` or
   `->`, and not preceded by an identifier or a numeric literal;
2. the contract **in force at this point of the bottom-up walk** exists for `f`
   and declares result `ResourceReturnMode::None`;
3. every declared parameter is `ResourceParameterMode::Borrowed`;
4. the number of written top-level arguments **equals** the declared parameter
   count;
5. every argument is non-empty and admitted by this same rule, at one greater
   depth, to a bound of 4 (a refusal, never a truncation).

**F2 — the typed scalar operand.** Every identifier outside an admitted call
span is an enumeration constant or a variable this function declares with a
non-pointer, non-array arithmetic or enumeration type; every other token is a
numeric literal or one of card C02's admitted operators. `&`, `*`, `[`, `]`,
`{`, `}`, `,` and `;` are absent from that operator set, so an address, an
indirection, a subscript, an aggregate and the comma operator each refuse.

**F3 — the typed direct scalar-field read.** Exactly card C02's whitelist,
evaluated in an expression position instead of a statement position:
`base -> field` / `base . field` on a declared single-star record pointer or
record value, a completely read `struct` with no `union`, bitfield or nested
anonymous aggregate, a scalar member without `volatile` or `_Atomic`, no
continuation, and a base this function **never binds** to a resource identity.

F2 and F3 are **not re-implemented**. Both are C02's single walker,
`typed_field_access::scalar_statement_accesses`, reached through one new
`pub(crate) fn is_scalar_operand_expression`, applied to the tokens outside
every admitted call span.

### 2.1 Why the permissive direction is safe

The load-bearing consequence of the conjunction is one sentence:

> **An admitted neutral expression contains no pointer-valued operand
> anywhere.**

Every identifier is an enumeration constant, a declared *scalar* variable, a
callee name whose contract is neutral, or the base or field of an admitted
direct scalar-field access whose base the body never binds. A pointer-typed
local, a formal holding a resource, a `void *`, an array and every base the
body may bind are refused by name.

That is what makes turning the position's `Unknown` into a `NoOp` skip *no*
modeling. A statement-position call is modeled — it checks the liveness of its
`Borrowed` actuals, and card C02-P-ALIAS attaches a may-alias obligation to
each of its read arguments. Nothing is skipped here because there is nothing to
model: no argument can carry a tracked identity, and `may_alias` relates only
names declared `struct TAG *`, none of which can appear.

The rule is **all-or-nothing** over the expression: one call-shaped occurrence
this rule cannot admit refuses the whole position, exactly as `contains_call`
refuses it today, so no unmodelled call can hide beside an admitted one.

Deliberately not admitted, each with a test: an uncontracted callee; any
non-`Borrowed` parameter or non-`None` result (`Acquire`, `Release`, `Escape`,
`Out`, `Stored`, `Removed`, `Reallocated`); a written arity other than the
declared one (so a variadic callee refuses at every other arity, the
`O04-ARMS-VARIADIC` treatment); a call through a function pointer, a member or
an expression; `sizeof` and every other reserved word in call position; a
pointer, address-of, dereference, subscript or pointer-arithmetic operand.

## 3. A measured fail-open, found and repaired

Building the `for`-condition test surfaced a **pre-existing fail-open** that
this card's threading would otherwise have been silently unable to reach.

`split_for_clauses` consumes `(`, `)`, `[` and `]` while tracking depth, and
they never reach the clause vectors. So `for (; external(n); )` arrived at the
classification as the two tokens `external n`: `contains_call` finds no call
because there is no `(`, and `contains_unmodeled_access` finds no subscript
because there is no `[`. The position was classified `NoOp` and the unmodelled
call disappeared. Measured before the repair, with no resource in sight:

```
for (; external(n); ) { }        =>  Safe          (no diagnostic at all)
while (external(n)) { }          =>  Unknown       unmodeled … in loop condition
if (external(n)) { }             =>  Unknown       unmodeled … in branch condition
for (; c->slot; ) { }            =>  Unknown       unmodeled … in for-loop condition
```

The last row is why the family was never absent from the corpus: an access
whose tokens survive the stripping still refuses.

The repair is `for_condition_tokens`, which returns the condition clause **with
its grouping tokens intact**; the condition is classified from its original
spelling, like every other position. It can only turn an accepted condition
into a refused one. It is confined to the condition: `init` and `increment`
keep the stripped spelling `split_for_clauses` gives them, byte for byte —
recorded, not repaired, as `W4-EXPR-POS-FOR-CLAUSES` (§8) with a pinned test.

Its corpus effect is isolated by its own measurement (§5, "repair alone"):
**+121 refusals, zero function moves, zero unit moves.** Purely fail-closed.

## 4. `digits10` end to end

The design note's §6 minimal validation case, from Fluent Bit `66910c10`'s
`lib/monkey/mk_core/mk_string.c`:

```c
uint32_t digits10(uint64_t v) {
    if (v < 10) return 1;
    /* … nine more literal and scalar-arithmetic returns … */
    return 12 + digits10(v / 1000000000000UL);
}
```

Its single blocking instruction is the last return. Under `N(digits10)` the
callee's contract is `[Borrowed]` / `None`; the argument `v / 1000000000000UL`
is a declared scalar and a numeric literal; `12 +` is F2. The expression is
admitted, the body's resource CFG retains no `Unknown` instruction, and the
whole of NFR's V1–V10 discharges:

* **V1** no declared contract for the component;
* **V3** `N(1)` seeded, parameters readable;
* **V4** the descent's first round is stable, so the greatest post-fixed point
  below all-`Neutral` is all-`Neutral`;
* **V5** empty entry custody, no retained `Unknown`, `Safe` with zero
  diagnostics;
* **V6**/**V8** every reachable exit leaves every formal untracked and returns
  no tracked resource;
* **V7** the contract read back off the settled observations is exactly `N(1)`.

Measured end to end through the real gateway
(`crates/qcore-c-gateway/tests/neutral_expression_positions.rs`): status
`Safe`, published `ResourceSummaryRecord` with `ContractTrust::InferredValidated`
and contract `[Borrowed] / None`, and `check_legacy_safety_certificate`
**rederives it independently** from the certificate's own content. On the real
corpus the same thing happens: `digits10` moves `Unknown` → `Safe` and its
caller `mk_string_itop` loses its `call to uncontracted function` refusal.

## 5. Corpus re-measurement — every movement named

Canonical pinned route, freshly prepared pinned clone (Fluent Bit `66910c10`,
explicit Clang `Ubuntu clang 21.1.8`), provider packages rebuilt at this tree:

```sh
python3 -B providers/package.py build --output <scratch>/provider-packages
QCORE_CLANG=/usr/bin/clang \
QCORE_CHUNKIO_PACKAGE=<scratch>/provider-packages/qcore.chunkio \
cargo run --quiet -p qcore-compiler --example cycle19_fluent_bit_libraries_audit -- \
  --pinned-evidence <clone> <monkey>/compile_commands.json \
  <out>/monkey-source.json <out>/resource-diagnostics.json
```

Four full runs, all exit 0 with zero gateway failures and 185 units:

| run | tree |
|---|---|
| **BEFORE** | pristine `67961f1` |
| **stage 1** | the `return` position alone |
| **repair only** | stage 1 + §3's `for`-condition token repair, five positions still lexical |
| **AFTER** | the card's checkpoint |

### 5.1 BEFORE reproduces the pinned baseline

| artifact | BEFORE vs wave-11 archive |
|---|---|
| `resource-diagnostics.json` | **byte-identical** |
| `resource-diagnostics.chunk-contracts.json` | **byte-identical** |
| `monkey-source.json` | **byte-identical** |
| `resource-diagnostics.contracts.json` | differs — residue class 1 |
| `resource-diagnostics.source-effects.json` | differs — residue class 2 |

The two differing artifacts are the two residue classes waves 9–11 already
record, and they differ **identically in all four runs**, which is what
establishes they are environmental rather than this card's:

* `contracts.json`'s `sources` map is keyed by **absolute contract paths**, so
  it carries this worktree's directory: ten leaves, five paths, each present
  under both spellings with the same contract text;
* `source-effects.json` moves 56 leaves, every one of them a digest or an
  identity that binds one: `build_digest`, `configuration_digest`,
  `obligation_id`, `task_digest`, `package_digest`, the digest-suffixed
  `bounds.resource_contracts:…` and `task.contract_sources.…` map keys, and
  the `evidence.coverage` obligation identifiers. They move because they bind
  the contract source paths above and the provider package was rebuilt into a
  scratch directory. **No `status`, verdict, reason or count field differs.**

Headline verified independently from the run log: 31 + 17 + 94 + 13 + 1 + 29 =
185 units; 2 Safe / 42 Unsafe / 137 Unknown / 4 data-only.

### 5.2 Every movement, by increment

Full leaf-level attribution in [`diffs/`](diffs); the AFTER artifacts and the
run log are archived under [`artifacts/`](artifacts) with per-file SHA-256 in
`manifest.json`.

| increment | units changed | unit verdict moves | function moves | diagnostic delta |
|---|---|---|---|---|
| BEFORE → stage 1 (`return`) | 10 | **0** | **22** `Unknown`→`Safe` | −23 return-expression, −4 uncontracted-call |
| stage 1 → repair only (§3) | 33 | **0** | **0** | **+121** for-loop-condition, +11 unresolved-identity |
| repair only → AFTER (five positions) | 20 | **0** | **6** `Unknown`→`Safe` | −45 branch-condition, −11 for-loop-condition, −1 switch-expression, −2 uncontracted-call |
| **BEFORE → AFTER** | **55** | **0** | **28** `Unknown`→`Safe` | +110 for-loop-condition, −45 branch-condition, −23 return-expression, −1 switch-expression, −4 uncontracted-call, +11 unresolved-identity |

Net across the card: **69 refusals removed** by the admission and **121 added**
by the repair, plus **11** may-alias `unresolved resource identity` diagnostics
that the newly refused `for` conditions surface (a C02-P-ALIAS obligation on a
path the old fail-open had made unreachable). The net direction is
fail-**closed**; the promotions are the 28 function-level ones below.

### 5.3 The 28 function-level `Unknown` → `Safe` moves, by rule

Every one is a *function-level* projection move inside a unit whose verdict did
not change. No unit is promoted.

**By the `return` position (21), all losing exactly one
`unmodeled call, aggregate or indirect access in return expression`:**

| unit | functions |
|---|---|
| `chunkio/cio_chunk.c` | `cio_chunk_counter_total_add`, `cio_chunk_counter_total_sub`, `cio_chunk_counter_total_up_add`, `cio_chunk_counter_total_up_sub`, `cio_chunk_is_locked` |
| `chunkio/cio_error.c` | `cio_error_get` |
| `monkey-libevent/mk_core/mk_string.c` | **`digits10`** |
| `librdkafka/rdkafka.c` | `rd_kafka_messages_count` |
| `librdkafka/rdkafka_admin.c` | `rd_kafka_ConsumerGroupDescription_is_simple_consumer_group`, `rd_kafka_ConsumerGroupListing_is_simple_consumer_group`, `rd_kafka_ListOffsetsResultInfo_timestamp`, `rd_kafka_ScramCredentialInfo_iterations`, `rd_kafka_TopicDescription_is_internal`, `rd_kafka_TopicPartitionInfo_partition`, `rd_kafka_UserScramCredentialsDescription_scramcredentialinfo_count` |
| `librdkafka/rdkafka_cgrp.c` | `rd_kafka_consumer_group_metadata_generation_id` |
| `librdkafka/rdkafka_mock.c` | `rd_kafka_mock_request_api_key`, `rd_kafka_mock_request_id`, `rd_kafka_mock_request_timestamp` |
| `librdkafka/rdkafka_share_acknowledgement.c` | `rd_kafka_share_partition_offsets_list_count`, `rd_kafka_share_partition_offsets_offsets_cnt` |

**By a branch condition (5):** `chunkio/cio_file.c` `open_and_up`;
`fluent-bit-chunk/flb_input_chunk.c` `flb_input_chunk_is_mem_overlimit` (−2),
`flb_input_chunk_is_task_safe_delete`, `flb_input_chunk_release_lock`;
`librdkafka/lz4frame.c` `LZ4F_getErrorCode`.

**By a switch expression (1):** `ctraces/ctr_span.c` `ctr_span_kind_string`.

**Downstream of a newly published summary (1):** `chunkio/cio_error.c`
`cio_error_get_str`, whose `call to uncontracted function `cio_error_get``
disappears.

Two return-expression refusals disappeared **without** promoting their
function — `fluent-bit-chunk/flb_input_chunk.c` `logs_tag_records_metrics_enabled` and
`monkey-libevent/mk_server/mk_http_parser.c` `method_lookup` — both of which stay
`Unknown` on other blockers. No `while` or `do`/`while` condition refusal
disappeared anywhere in the corpus.

## 6. The reach census, re-run

`compiler/examples/c03_reach_census.rs`, the read-only route C03-REACH built,
re-run at this card's checkpoint on the same pinned inputs
([`artifacts/census.json.gz`](artifacts/census.json.gz), SHA-256
`dd813df0a95dc4f75c62739699aebaabbdb701195a0e4296473dd9bdb1657558`):

| | C03-REACH | this card |
|---|---|---|
| units examined | 185 | 185 |
| recursive components | 46 | 46 |
| members | 96 | 96 |
| **admitted by the neutral route** | **0** | **1** (`digits10`, `mk_core/mk_string.c`) |
| refused | 46 | 45 |

Blocker families at the first refusing obligation, where they moved:

| family | C03-REACH | this card |
|---|---|---|
| unmodeled access in a **branch condition** | 450 | 449 |
| unmodeled access in a **return expression** | 113 | 112 |
| unmodeled access in a **for-loop condition** | 26 | 25 |
| every other family | unchanged | unchanged |

**This is a smaller move than the design note's payoff table projects, and the
note says why.** C03-REACH §9 records that its tables "assume a family is
admitted *completely and soundly*, which no real conjunctive whitelist
achieves"; the `POSITION alone` row's 3 components is an upper bound. The
measured figure is 1. The 45 remaining components refuse in bodies whose
positions carry pointer fields, chained selections and uncontracted calls —
none of which any of F1–F3 admits, by construction. The note's own conclusion
stands unchanged and is now measured rather than projected: **the reach is
gated on `POSITION + INDIRECT` jointly**, and this card is the POSITION half.

## 7. Lean

30 audited theorems in a new `QCore.NeutralExpressionPosition`
(`proof/QCore/NeutralExpressionPosition.lean`,
`proof/AuditNeutralExpressionPosition.lean`), **no new axiom** — every theorem
depends on nothing, or on `propext` and `Quot.sound` alone. The audit count
moves 53 → 54.

* **F2 and F3** — a literal, an enumeration constant and a declared scalar are
  admitted at every depth; a pointer operand never is; a direct scalar-member
  read is admitted exactly when the record model carries the member and the
  body never binds the base, and refuses on either failure alone.
* **F1** — an uncontracted callee, a non-`None` result, any non-`Borrowed`
  parameter and a mismatched arity each refuse; a neutral contract over
  admitted arguments is admitted; every argument of an admitted call is itself
  admitted; and a call with arguments exhausts the depth budget rather than
  being truncated.
* **The soundness sentence of §2.1**, in both of its halves —
  `an_admitted_expression_names_no_pointer` and
  `an_admitted_expression_reads_through_no_bound_base`, by induction on the
  depth budget.
* **Additivity** — a position the lexical predicate already accepted is decided
  by it; a refused expression keeps its `Unknown`; an admitted one becomes a
  `NoOp`.
* **Non-vacuity, in both directions** — the `digits10` shape is admitted, and
  refuses without its contract, with a pointer actual, with a releasing callee
  and with an owned result; four nested neutral calls fit the budget and five
  do not; the field-read family decides both ways; one unadmitted call refuses
  the whole expression.

The model carries the nesting bound as a fuel argument, which is why every
definition is structurally recursive on a `Nat` and every proof is finite. As
in `QCore.LegacyCAnalyzer`, `QCore.BorrowedActual` and `QCore.MayAlias`, that
the Rust reader extracts declared types, record members, the bound-name
over-approximation and the contract environment faithfully from C source is a
**premise, not a theorem**; nothing here is a claim about arbitrary C memory
safety.

## 8. Open boundaries, named

* **`W4-EXPR-POS-FOR-CLAUSES`.** §3's repair is confined to the `for`
  condition. The `init` and `increment` clauses still reach
  `classify_simple` with `(`, `)`, `[` and `]` stripped. Measured, the residual
  there is fail-closed but for the wrong reason: `for (n = external(0); …)`
  refuses as `unsupported assignment expression`, because `n = external` with
  the parentheses gone is not recognised as an assignment — *not* because a
  call was seen. Pinned by
  `a_for_init_and_increment_keep_the_stripped_spelling`.
* **`W4-EXPR-POS-CASE`.** The `case` label keeps the strict lexical predicate.
  A `case` label is an integer constant expression, so neither a call nor a
  member access can appear in one and the typed admission would be inert.
  Pinned by `a_case_label_keeps_the_lexical_predicate`.
* **The INDIRECT half is untouched.** C03-REACH measured that the reach needs
  `POSITION + INDIRECT` jointly; this card is POSITION. The 642-occurrence
  indirect-access family is unchanged, and the design note's §4 boundary —
  `free(kvlist->values)`, a release through an identity the projection cannot
  name — **must stay refused**, and does.
* **Heap-link chain teardown stays refused**, as the note's §7 requires.
  `cJSON_Delete`, `rd_avl_remove_elm0` and the `otlp_*_destroy` family still
  refuse at V5.
* **`C03-REACH-FNPTR-PARAM`** is untouched: `rd_kafka_q_apply` still has no
  `N(m)` candidate because `function_parameters` cannot read its
  `int (*callback)(…)` declarator.
* **The `null_condition` and `distinct_condition` refinements keep the strict
  predicate.** A condition admitted only by the new rule reaches
  `condition_supported` and becomes a `NoOp`, never a `BranchOnNull` or a
  `BranchOnDistinct`. That is the more-refusing direction: it drops a
  refinement, it never adds one.
* **A `do`/`while` whose body always leaves the loop** makes the header
  unreachable, and the containment gate then refuses the whole function with an
  internal error rather than an `Unknown` verdict. Encountered while writing a
  fixture and recorded here as a pre-existing observation; not this card's, not
  repaired.
* **No unit is promoted, and nothing here is an upstream defect claim.** The
  185-unit ledger is unchanged; the 28 function moves are projection coverage,
  not safety coverage.

## 9. Gate

Run from this worktree at the card checkpoint. Clang `Ubuntu clang 21.1.8`,
Libevent submodule at `d82464a`.

| check | result |
|---|---|
| `cargo test --workspace` | **2169 passed**, 0 failed, 4 ignored |
| `cargo fmt --all -- --check` | clean |
| `cargo clippy --all-targets -- -D warnings` | clean |
| `cargo run -p qcore-compiler --example reevaluation_probe` | exit 0 |
| `cargo run -p qcore-compiler --example reevaluation_followup` | exit 0 |
| `python3 scripts/check_lean_audits.py` | **54 audits pass** |
| `cd providers && cargo test` | 76 passed, 0 failed |
| `python3 -m unittest discover -s scripts/tests` | 124 passed, **1 failed by design** (§9.2) |
| `python3 -B providers/package.py build` | five packages built, for §5's runs |
| 185-unit corpus, four runs | §5 |

The workspace count is 2169 against wave 11's 2119: exactly the 50 new tests
(24 in `neutral_position`, 26 in `neutral_expression_positions`), and no other
binary's count moves.

### 9.1 Known load-flaky, confirmed passing in isolation

`qcore-verifier-host::confined_process_execution::a_confined_fixture_cannot_write_outside_the_admitted_paths`
failed once under full-workspace load (`Io` where `Terminated` was expected,
broken pipe) and passed on a rerun of that target alone (12 passed). This is
the flakiness already recorded for P02/P03; the assertion is deliberately not
weakened.

### 9.2 One deliberate failure, for the integrator

`scripts/tests/test_build_matrix_sections.py::test_the_derived_facts_are_recomputed_rather_than_carried`
fails with `['8.0.0', 53] != ['8.0.0', 54]`: the committed
`docs/reviews/r0_3/acceptance_matrix.json` records 53 Lean audits and this tree
has 54. The card is forbidden from editing that file, so the reconciliation is
the integrator's, exactly as briefed. Nothing else in that suite moves.

## 10. Cross-card overlap, recorded not resolved

Cards **O04-B09B10** and **ALIAS-LOCALS** ran in parallel in the same crate.
This card's `crates/` diff is:

| file | change |
|---|---|
| `crates/qcore-c-gateway/src/neutral_position.rs` | new |
| `crates/qcore-c-gateway/src/lib.rs` | one `mod` line |
| `crates/qcore-c-gateway/src/typed_field_access.rs` | one new `pub(crate) fn is_scalar_operand_expression`, no existing line changed |
| `crates/qcore-c-gateway/src/legacy_c.rs` | `is_identifier` becomes `pub(crate)`; new `Parser::supported_expression`; new `for_condition_tokens`; six call sites in `parse_statement` |
| `crates/qcore-c-gateway/tests/neutral_expression_positions.rs` | new |

`legacy_c.rs`'s `parse_statement` and `typed_field_access.rs` are the plausible
textual conflicts with a card touching `aggregate_store`, `borrowed_actual` or
`may_alias`. Semantically, this card changes only the six `*_supported`
booleans and the `for` condition's token stream; it adds no diagnostic code,
changes no contract, and touches neither the may-alias relation nor the
aggregate-store admission. Cross-card reconciliation is deliberately left to
the integrator.
