# REACH-GATE — the INDIRECT half of the joint expression-position gate — 2026-09-09

Follow-on to [C03-REACH](../remaining-c03-reach/README.md), whose §5 measured
that the neutral-frame recursive route's corpus reach is gated on
`POSITION + INDIRECT` **jointly**, and to
[W4-EXPR-POS](../remaining-w4-expr-pos/README.md), which landed the POSITION
half in wave 12 and moved the reach 0 → 1.

**Baseline:** main `bf210eb` (wave-12 merged tree), pinned corpus artifacts
[`docs/reports/wave12-corpus/artifacts/`](../wave12-corpus/artifacts).
**Branch:** `claude/reach-gate`. Fluent Bit `66910c10`, explicit Clang
`Ubuntu clang 21.1.8`, Libevent submodule `d82464a`.

---

## Headline

**Nothing is promoted.** The pinned 185-unit corpus stays
**2 Safe / 42 provisional Unsafe / 137 Unknown / 4 data-only**, with **no unit
verdict move in either direction** and **no newly surfaced provisional
`Unsafe` unit**. Leak diagnostics are unchanged at **271**. Four units change a
leaf; **two functions move `Unknown` → `Safe`**, each attributed to its rule in
§6.

**The neutral-frame recursive route's reach does not move.** It stays at **1 of
46** components (`digits10`), and the reach census is **byte-identical**
before and after this card
(SHA-256 `a2db6f26913ef734ba5b60eb1942d9f6cd5193d8e6b634563bb3c51e6ab1975a` for
both runs). §5 measures why, per component, and the answer is the load-bearing
finding of this card:

> **The design note's `POSITION + INDIRECT` payoff of 11 components is not
> reachable by any sound admission.** Each of the ten components at that
> threshold needs at least one construct that must stay refused — a release
> through a place the projection cannot name, a subscripted pointer-array
> element, a call through a function-pointer member, an uncontracted callee, a
> cast, or a pointer local bound from a pointer field. The joint upper bound is
> an upper bound on a family, not on an admission.

`qcore-schemas` stays 9.0.0; no crate version moves; no diagnostic code is
added, changed or downgraded; the contract corpus stays 60 modules; CT coverage
stays 31/50.

---

## 1. What the card was asked to land, and what it found

The brief asked for "the INDIRECT half in expression positions — C02's typed
member-access family (and the chain/bound-read extensions where their premises
hold)". Measurement corrected two of its premises before a line was written:

1. **C02's member-access family was already admitted in expression positions.**
   W4-EXPR-POS's family F3 *is* C02's whitelist, evaluated in an expression
   position. What was still refused there is everything C02's whitelist refuses:
   a **chain** of two or more selections, and a **pointer-typed leaf**. Those
   two are what this card lands, as family **F4**.
2. **The bound-read extension's premise does not hold in an expression
   position.** Card O04-READ-BOUND admits a read through a base *this body
   binds*, but only as a whole `Borrowed` actual of a modeled call, whose
   liveness `transfer` revalidates flow-sensitively before the call takes
   effect. A branch condition carries no modeled call, so there is nothing to
   attach that revalidation to. The family therefore **stays refused**, exactly
   as the brief instructs ("if the refinement cannot be proved for a family,
   that family stays refused in positions"), and it is recorded as
   **REACH-GATE-BOUND-BASE** in §9.

## 2. The refined soundness sentence

W4-EXPR-POS's sentence was

> an admitted neutral expression contains no pointer-valued operand anywhere.

F4 reads *through* record pointers, so a member access on a record pointer is a
pointer-valued base by construction. The sentence is **refined, not violated**:

> **An admitted neutral expression binds, releases, escapes and addresses
> nothing the projection tracks.** Every call it contains is declared
> all-`Borrowed` / `None` at exactly its written arity over arguments that name
> no pointer at all; every pointer it names appears only as the base or an
> intermediate link of an F4 chain whose base this body **never binds**, so no
> place along that chain is a place the projection binds; and the value such a
> chain produces is either a **scalar**, or — only in a position that discards
> it into a control-flow decision, in an expression carrying no assignment and
> outside every call argument list — **one pointer that is stored nowhere,
> passed to nothing and returned to no caller**.

Each clause is load-bearing and each is checked:

* **the base is never bound.** `FunctionTypes::may_bind` is the same
  over-approximation C02's premise 8 uses: a name written, address-taken or
  named anywhere in a call's argument list is bound, and a protected formal is
  bound at entry. The projection's own tracked `->` places come from card
  O04-B08's aggregate store, whose base must be a *body-acquired* name — which
  is bound by construction — so a never-bound base carries **no tracked field
  place, at any chain depth**. That is what makes reading through the chain
  custody-neutral rather than merely convenient, and it is exactly why
  O04-READ-BOUND's bound bases stay refused.
* **the value is discarded.** A pointer leaf is admitted only in the `Decides`
  role — a branch, `while`, `for`, `do`/`while` or `switch` condition. A
  `return` expression keeps the strict no-pointer rule, because `legacy_c`
  models a return's escape only for a *canonical place*
  (`EscapeIfOwned`): a compound return expression that produced a pointer would
  escape it unmodeled. W4-EXPR-POS's sentence therefore survives **verbatim**
  in that role.
* **no assignment, no call argument, no address.** `&`, `*`, `[`, `]`, `{`,
  `}`, `,` and `;` are refused by the operand walker; an assignment (simple or
  compound) refuses the whole run whenever a pointer leaf is in play; and an
  admitted call's arguments are walked with the pointer leaf switched **off**,
  in either role, so W4-EXPR-POS's argument reasoning is preserved word for
  word.

### 2.1 The may-alias obligation is neither created nor erased

Card C02-P-ALIAS's `may_alias` module header records
`C02-P-ALIAS-USES`: a branch condition is not a modeled read, and that is a
*retained pre-existing answer*, not something this card changes. Two tests pin
it (§4): the obligation's diagnostic vector is byte-identical with and without
an F4-admitted condition beside it, and — the structural reason it can only
read that way — a name that appears in **any** call argument list of the body
is bound, so **no base this rule admits is ever a modeled call's actual**. One
`z_hold(view->head)` anywhere in the body withdraws every admission through
`view`.

## 3. The implementation

`crates/qcore-c-gateway/src/neutral_position.rs` gains family **F4** and the
role split; the wave-12 predicate order is unchanged and consulted first, so
the admission is additive at two levels — the lexical predicate answers first,
then W4-EXPR-POS's F2/F3, then F4.

| file | change |
|---|---|
| `crates/qcore-c-gateway/src/neutral_position.rs` | `ExpressionRole`; `ACCESS_CHAIN_LIMIT`; `admitted_read_run`, `admitted_chain`, `contains_assignment`; `is_neutral_expression` takes the role; header rewritten for the refined sentence; 15 new unit tests |
| `crates/qcore-c-gateway/src/typed_field_access.rs` | three additive read-outs — `FunctionTypes::may_bind`, `is_admitted_operand_punctuation`, `is_admitted_numeric_literal` — so F4 uses C02's *own* bound-name set, operator set and literal test rather than a copy. No existing line changed. |
| `crates/qcore-c-gateway/src/legacy_c.rs` | `Parser::supported_expression` takes an `ExpressionRole`; the five condition positions pass `Decides`, `return` passes `Returned`; one `use` line. No predicate is loosened. |
| `crates/qcore-c-gateway/tests/reach_gate_positions.rs` | new, 10 end-to-end tests |
| `crates/qcore-c-gateway/tests/fixtures/contracts/z_reach.qcore` | new, one `Borrowed` reader for the may-alias test |
| `crates/qcore-c-gateway/tests/o04_b08.rs` | one deliberate pin update, §8 |
| `proof/QCore/ReachGatePosition.lean`, `proof/AuditReachGatePosition.lean`, `proof/QCore.lean` | new namespace, 40 audited theorems |

The `case` label keeps the strict lexical predicate, as `W4-EXPR-POS-CASE`
records: a `case` label is an integer constant expression, so F4 would be inert
there.

## 4. The held-out evidence

`crates/qcore-c-gateway/tests/reach_gate_positions.rs`, ten tests over a `z_*`
declaration family deliberately unrelated to the `q_*` (W4-EXPR-POS), `k_*`
(O04-B08), `g_*` (O04) and `h_*` (O04-ARMS) families, all driven through
`analyze_unrestricted_c11`:

* a three-selection chain to a scalar leaf **discharges a branch condition**,
  the function reports `Safe`, and its certificate **rechecks independently**;
* the same chain is admitted in **all six positions**;
* a **pointer leaf** is admitted in a condition and refused in a `return`, in
  both the bare and the compound (`?:`) return shapes, each keeping the exact
  `… in return expression` diagnostic;
* a **released-base member read in a condition keeps its refusal** — the C02 §2
  hazard, in an expression position — and reports no discharge;
* the adversarial family, eight shapes, each keeping its existing `Unknown` and
  its existing diagnostic: an **address-of** in a condition; an explicit
  **dereference**; a **subscript**; a chain **one selection past the bound**; a
  link the reader will not type (`void *`); a base named in **any call argument
  list**; an **assignment** that could store the pointer; and a **cast**;
* a chain **exactly at the bound** is admitted, so the bound is a refusal and
  not a blanket rejection;
* the **may-alias** obligation fires identically with and without an admitted
  condition, and an admitted base is never a modeled call's actual;
* an admitted condition **never hides a leak** — the same body with a missing
  release still reports `Unsafe` with its one leak;
* the earlier families are unchanged, including an uncontracted call in a
  condition.

Fifteen further unit tests in `neutral_position`'s own module cover the chain
walk, the role split, the depth bound, the record-value `.` base, a `union`
pointee, an untyped link, and the assignment ban. Card W4-EXPR-POS's own
26 integration tests and 24 unit tests are **all green, unchanged**.

## 5. The reach census, re-run — and why the joint bound is unreachable

`compiler/examples/c03_reach_census.rs`, the read-only route C03-REACH built,
run on the pinned prepared clone before and after this card:

| | C03-REACH | W4-EXPR-POS | this card |
|---|---|---|---|
| units examined | 185 | 185 | 185 |
| recursive components | 46 | 46 | 46 |
| **admitted by the neutral route** | 0 | **1** | **1** |
| refused | 46 | 45 | 45 |

Every blocker-family count is unchanged, and the two census JSON documents are
**byte-identical**. This card's reach movement is **0**.

That is not a defect in the admission; it is a fact about the bodies, and the
census measures it exactly. The ten components at the `POSITION + INDIRECT`
threshold (11 in the design note's table, of which `digits10` is already
admitted) and the construct each of them needs:

| component | members | needs, and must keep refusing |
|---|---|---|
| `cmetrics/cmt_encode_opentelemetry.c` `otlp_*_destroy` | 4 | `free(value->string_value)`, `otlp_array_destroy(value->array_value)` — **release through a place the projection cannot name**; `array->values[index]` — subscript |
| `ctraces/ctr_encode_opentelemetry.c` `otlp_*_destroy` | 4 | the same, plus a **pointer-field store** `value->string_value = NULL` |
| `librdkafka/cJSON.c` `cJSON_Delete` | 1 | `global_hooks.deallocate(item->string)` — a release **through a function-pointer member**; `next = item->next` — a pointer local bound from a pointer field |
| `librdkafka/cJSON.c` `cJSON_Compare` | 1 | `strcmp`, `get_object_item` — **uncontracted callees**; pointer locals bound from pointer fields |
| `librdkafka/nanopb/pb_encode.c` `pb_check_proto3_default_value` | 1 | `(const char*)field->pData`, `*(const pb_size_t*)field->pSize` — **casts and dereferences**; `p[i]` — subscript; `pb_field_iter_begin`, `safe_read_bool` — uncontracted |
| `librdkafka/rdavl.c` `rd_avl_find_node` | 1 | `ravl->ravl_cmp(elm, begin->ran_elm)` — a **call through a function-pointer member**; `begin->ran_p[RD_AVL_LEFT]` — subscript |
| `librdkafka/rdavl.c` `rd_avl_insert_node` | 1 | **subscripted stores** into `parent->ran_p[dir]` |
| `librdkafka/rdavl.c` `rd_avl_move` | 1 | subscripted store; recursive call in a subscripted destination |
| `librdkafka/rdavl.c` `rd_avl_remove_elm0` | 1 | the same |
| `librdkafka/rdkafka_conf.c` `rd_kafka_anyconf_get` | 1 | `prop++` over a **global array** — pointer arithmetic; `strcmp` — uncontracted |

The two components that clear on `POSITION alone` in the census table —
`rd_avl_find_node` and `rd_kafka_anyconf_get` — are in that list for the same
reason: their *positions* carry a function-pointer call and an uncontracted
callee respectively, so no position rule admits them either.

C03-REACH §9 already stated the caveat this measures: "the payoff tables are
upper bounds … a real admission covering a fragment of POSITION + INDIRECT will
admit fewer than 11 components, never more." The measured number is **zero
more**, and the reason is not fragment-versus-family: it is that the residual
constructs are the ones the design note's own §4 and §7 require to stay refused
(`free(kvlist->values)`; heap-link chain teardown). Full census output:
[`diffs/joint-gate-census.txt`](diffs/joint-gate-census.txt),
[`artifacts/census.json.gz`](artifacts/census.json.gz).

## 6. Corpus re-measurement — every movement named

Canonical pinned route, freshly prepared pinned clone, provider packages
rebuilt at this tree:

```sh
python3 -B providers/package.py build --output <scratch>/provider-packages
QCORE_CLANG=/usr/bin/clang \
QCORE_CHUNKIO_PACKAGE=<scratch>/provider-packages/qcore.chunkio \
cargo run --quiet -p qcore-compiler --example cycle19_fluent_bit_libraries_audit -- \
  --pinned-evidence <clone> <monkey>/compile_commands.json \
  <out>/monkey-source.json <out>/resource-diagnostics.json
```

Three full runs, all exit 0 with zero gateway failures and 185 units: **BEFORE**
at pristine `bf210eb`, **AFTER** at the card's checkpoint, and a third at the
final formatted tree, which reproduced AFTER **5/5 byte-identical**.

### 6.1 BEFORE reproduces the pinned baseline

| artifact | BEFORE vs wave-12 archive |
|---|---|
| `resource-diagnostics.json` | **byte-identical** |
| `resource-diagnostics.chunk-contracts.json` | **byte-identical** |
| `monkey-source.json` | **byte-identical** |
| `resource-diagnostics.contracts.json` | differs — residue class 1 |
| `resource-diagnostics.source-effects.json` | differs — residue class 2 |

The two residue classes are the ones waves 9–12 already record — absolute
contract paths in `contracts.json`'s `sources` map, and the digest/identity
leaves in `source-effects.json` that bind them and the rebuilt provider
package — and both are **byte-identical between BEFORE and AFTER**, which is
what establishes they are environmental rather than this card's.

### 6.2 BEFORE → AFTER, complete

| | |
|---|---|
| unit verdict moves | **0** |
| newly surfaced provisional `Unsafe` units | **0** |
| units with any changed leaf | **4** |
| function status moves | **2**, both `Unknown` → `Safe` |
| leak diagnostics | **271 → 271** |
| diagnostic delta | −2 `… in branch condition`, −2 `… in return expression`; nothing else |

Full leaf-level output: [`diffs/before-to-after.txt`](diffs/before-to-after.txt),
[`diffs/changed-units.txt`](diffs/changed-units.txt).

### 6.3 The four changed units, each attributed to its rule

| unit | function | leaf | shape, byte-verbatim at `66910c10` |
|---|---|---|---|
| `fluent-bit-chunk/flb_input_chunk.c` | `logs_tag_records_metrics_enabled` | **`Unknown` → `Safe`**, one return-expression refusal removed | `return in->config->telemetry_metrics_logs_tag_records;` — F4 **scalar leaf**, two selections, `Returned` role |
| `monkey-libevent/mk_server/mk_plugin.c` | `mk_plugin_time_now_unix` | **`Unknown` → `Safe`**, one return-expression refusal removed | `return server->clock_context->log_current_utime;` — the same shape |
| `cmetrics/cmt_encode_opentelemetry.c` | `compute_flat_scope_index` | stays `Unknown`, one branch-condition refusal removed | `if (resource_index >= context->metrics_data->n_resource_metrics)` — F4 **scalar leaf**, two selections, `Decides` role |
| `ctraces/ctr_encode_opentelemetry.c` | `destroy_id` | stays `Unknown`, one branch-condition refusal removed | `if (id.data)` — F4 **pointer leaf** on a record **value** base through `.`, `Decides` role |

Both halves of F4 fire on the real corpus: the scalar-leaf chain three times
and the pointer leaf once. The two functions that move to `Safe` are exactly
the shape the card exists for — a two-selection read of a scalar configuration
field through a formal the body never binds — and neither promotes its unit,
which stays provisional `Unsafe` on its other functions.

## 7. Lean

**40 audited theorems** in a new `QCore.ReachGatePosition`
(`proof/QCore/ReachGatePosition.lean`, `proof/AuditReachGatePosition.lean`),
**no new axiom** — every theorem depends on nothing, or on `propext` and
`Quot.sound` alone. The audit count moves **57 → 58**.
`QCore.NeutralExpressionPosition` is **untouched**: its 30 theorems remain true
of its own model, and none is retired.

* **F4, the chain** — a scalar leaf is admitted in either role; a pointer leaf,
  record-typed or not, is admitted *exactly* where the position discards the
  value; a base this body may bind refuses at any depth; an undeclared base, an
  unmodelled leaf and an empty selection list refuse; a link that is not a fully
  typed record pointer refuses; the walk reaches the leaf exactly at the bound
  and any longer chain yields no leaf at all, proved by induction on the fuel,
  so the bound is a refusal and never a truncation.
* **F1, the call** — carried over and restated for this card's step, which
  additionally decides every argument **without** the pointer leaf.
* **The refined sentence of §2**, in four parts —
  `an_admitted_expression_reads_through_no_bound_base`,
  `an_admitted_expression_names_no_bare_pointer`,
  `a_returned_expression_names_no_pointer_value` (W4-EXPR-POS's sentence, kept
  verbatim in the `returned` role) and
  `an_argument_of_an_admitted_call_names_no_pointer_value` (kept verbatim
  inside every call's arguments, in either role), plus
  `an_assignment_refuses_every_pointer_leaf_position`.
* **Additivity and the refusing direction** — a position the earlier families
  accepted is decided by them; a refused expression keeps its `Unknown` and an
  admitted one becomes a `NoOp`; and `refusing_the_pointer_leaf_only_refuses`
  proves switching the leaf off is strictly the more refusing direction.
* **Non-vacuity, in both directions** — the `z_*` fixture family projected: the
  scalar chain is admitted in either role; the pointer chain splits on the role;
  the same chain refuses on a bound base; four selections are admitted and five
  refuse; an unmodelled leaf refuses; a scalar chain is admitted inside a call
  argument and a pointer chain is not; an assignment refuses the pointer chain;
  and a bare pointer operand refuses beside an admitted call.

As in `QCore.LegacyCAnalyzer`, `QCore.BorrowedActual`, `QCore.MayAlias` and
`QCore.NeutralExpressionPosition`, that the Rust reader extracts declared types,
record members, the bound-name over-approximation and the contract environment
faithfully from C source is a **premise, not a theorem**; nothing here is a
claim about arbitrary C memory safety.

## 8. The one deliberate pin update

`crates/qcore-c-gateway/tests/o04_b08.rs`
`a_branch_on_an_unrelated_field_keeps_its_existing_refusal`, row 2 — "a member
of a base this rule never admitted",
`if (o->box == 0)` on a formal `struct k_outer *o`. O04-B08 pinned it refused,
meaning refused *by O04-B08's aggregate null-condition rule*, whose base must be
in `store_bases`. F4 admits that read by a different, typed route: `o` is a
formal this body never binds, and the value is discarded into the branch.

Both load-bearing halves are kept, and one is strengthened:

* the row itself is **retained** with its base made bound (`p = k_pick(o)`), so
  an aggregate branch on a base whose custody this body may change still keeps
  its refusal — and rows 1 and 3 (a scalar member of a *bound* base; a chained
  member of a *bound* base) are untouched and still refuse;
* the positive half is **recorded explicitly** as a new named test,
  `a_branch_on_a_never_bound_formals_member_is_admitted_by_reach_gate`, rather
  than left to a silently weakened assertion.

`neutral_position`'s own `a_pointer_field_read_refuses` is likewise renamed to
`a_pointer_field_read_is_admitted_only_where_the_value_is_discarded` and keeps
both negative halves (refused in a `return`; refused on a bound base, in either
role).

No other pin moved anywhere in the workspace.

## 9. Open boundaries, named

* **The reach is 1 and stays 1.** This card admits no recursive component. The
  design note's `POSITION + INDIRECT` payoff of 11 is measured **unreachable by
  any sound admission** (§5), and the per-component reasons are recorded there.
  An implementing card that wants those ten must first decide
  release-through-an-unnamed-place, subscripted pointer-array elements and
  calls through function-pointer members — the first of which
  `docs/design/W4_NEUTRAL_EXPRESSION_POSITIONS.md` §4 requires to stay refused.
* **`REACH-GATE-BOUND-BASE`.** A read through a base this body binds stays
  refused in every expression position. O04-READ-BOUND's premise is a
  flow-sensitive liveness revalidation attached to a modeled call; admitting it
  in a condition needs a new modeled instruction that carries the obligation,
  which is a larger change than this card's scope and is not attempted here.
  Pinned in both directions by tests.
* **`REACH-GATE-DEREF`.** `*p`, `a[i]`, casts and address-of stay refused in
  every position and both roles. The census records 57 dereferences and
  36 subscripts in the INDIRECT family; none is admitted.
* **The pointer leaf is confined to conditions.** A `return` expression keeps
  the strict rule because `EscapeIfOwned` models only a canonical place.
  Admitting a pointer-valued *return* would need the return position to model
  the escape of a non-place expression; that is a separate change and is not
  attempted (**REACH-GATE-RETURN-POINTER**).
* **The statement-position INDIRECT gate is untouched.** This card is about the
  six *expression* positions. `indirect memory access is outside the resource
  projection` refuses exactly what it refused, which is what keeps
  `free(kvlist->values)` refused.
* **`W4-EXPR-POS-FOR-CLAUSES` and `W4-EXPR-POS-CASE`** are unchanged and
  untouched; so is **`C03-REACH-FNPTR-PARAM`**.
* **No unit is promoted, and nothing here is an upstream defect claim.** The
  185-unit ledger is unchanged; the two function moves are projection coverage,
  not safety coverage.

## 10. Gate

Run from this worktree at the card checkpoint.

| check | result |
|---|---|
| `cargo test --workspace` | **2342 passed**, 0 failed, 5 ignored |
| `cargo fmt --all -- --check` | clean |
| `cargo clippy --all-targets -- -D warnings` | clean |
| `cargo run -p qcore-compiler --example reevaluation_probe` | exit 0 |
| `cargo run -p qcore-compiler --example reevaluation_followup` | exit 0 |
| `python3 scripts/check_lean_audits.py` | **58 audits pass** |
| `cd providers && cargo test` | 70 passed, 0 failed |
| `python3 -m unittest discover -s scripts/tests` | 124 passed, **1 failed by design** (§10.2) |
| `python3 -B providers/package.py build` | five packages built, for §6's runs |
| 185-unit corpus, three runs | §6 |
| reach census, two runs | §5, byte-identical |

The workspace count is 2342 against wave 12's 2332 on this tree before the
card's own test file: exactly the 25 new tests (15 in `neutral_position`,
10 in `reach_gate_positions`), one renamed `o04_b08` test and one added, and no
other binary's count moves.

### 10.1 Known load-flaky, confirmed passing in isolation

`qcore-verifier-host::confined_process_execution::a_confined_fixture_cannot_write_outside_the_admitted_paths`
and
`qcore-verifier-host::enforced_process_limits::a_declared_process_limit_stops_the_fixture_from_forking`
each failed once under full-workspace load with `Io`/broken pipe where
`Terminated` was expected, and each passed on a rerun of its target alone
(12 passed, 12 passed). This is the flakiness already recorded for P02/P03; the
assertions are deliberately not weakened.

### 10.2 One deliberate failure, for the integrator

`scripts/tests/test_build_matrix_sections.py::test_the_derived_facts_are_recomputed_rather_than_carried`
fails with `['9.0.0', 57] != ['9.0.0', 58]`: the committed
`docs/reviews/r0_3/acceptance_matrix.json` records 57 Lean audits and this tree
has 58. The card is forbidden from editing that file, so the reconciliation is
the integrator's, exactly as briefed. Nothing else in that suite moves.

## 11. Cross-card overlap, recorded not resolved

Cards **O04-TRACE**, **BORROW-LIVENESS** and **HASH-CONTRACTS** run in parallel
in `crates/qcore-c-gateway`. This card's production diff is three files:

| file | change | plausible conflict |
|---|---|---|
| `src/neutral_position.rs` | F4 and the role split | none expected: no other wave-13 card is scoped to expression positions |
| `src/typed_field_access.rs` | three additive read-outs, no existing line changed | a card touching C02's walker |
| `src/legacy_c.rs` | `Parser::supported_expression`'s signature and its six call sites in `parse_statement`; one `use` | a card touching `parse_statement` or `classify_simple` |

Semantically this card changes only the six `*_supported` booleans. It adds no
diagnostic code, changes no contract, touches neither the may-alias relation nor
the aggregate-store admission nor the statement-position gate, and leaves
`AGENTS.md`, `CLAUDE.md`, `README.md`, `DEVELOPMENT.md`, `ct_manifest.json`,
`acceptance.json` and `acceptance_matrix.json` untouched. Cross-card
reconciliation is deliberately left to the integrator.
