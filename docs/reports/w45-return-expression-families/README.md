# W45-RETURN-EXPRESSION-FAMILIES — the refused return expressions that block functions alone

Card W45-RETURN-EXPRESSION-FAMILIES, 2026-09-25, on base `b160d765` (main's
combined baseline `docs/reports/w44-collection-allocation/integration/artifacts/`).
The return-position refusal ("unmodeled call, aggregate or indirect access in
return expression") was the only reason of **147** Unknown functions. This card
re-censuses it by family on the current baseline and admits the two families
that free the most of those functions, each with the returned value's custody
stated exactly and Lean parity.

## Headline

* **S — the scalar-result read.** In a function whose definition declares a
  scalar value result, a return expression that is a *read* (no call, write,
  comma, brace, GNU `?:` or unary `&`; no tracked descriptor; untracked or
  declared-scalar names, and a name the body may bind only as the root of one
  dereference C always evaluates) lowers to one typed read node over its
  dereferenced bases and a `NoOp` exit. The value is a number: it carries no
  custody, so nothing escapes.
* **K — a cast of a whole call.** `return (T) … CALL(…);` is classified as W4
  classifies the call. A refusal is lowered to itself (W43's re-attribution
  under a cast); a `None` result is admitted under any cast; an `Owned` result
  only under pointer casts of a declared object pointer, escaping exactly its
  fresh site; everything else keeps the refusal.
* **Arbiter.** BEFORE reproduces the baseline's verdict-bearing artifacts byte
  for byte. AFTER: **76 functions Unknown → Safe**, none to provisional Unsafe;
  the return refusal falls from **495 leaves / 417 functions / 147 sole** to
  **305 / 249 / 69**; leak **37 → 37**; unknown_alias 3,507 → 3,517;
  unsupported 43,590 → 43,215; units unchanged (3 / 10 / 168 / 4). Every one of
  the 405 removed and 40 added leaves and all 76 moves are attributed
  mechanically (`attribution_complete: true`).
* **Forecast.** Every one of the 67 sole-blocked functions forecast to be freed
  was freed; 5 more were (the forecast read a coarser may-bind fact and missed
  one `None`-result cast). 72 sole-blocked functions became `Safe` (69 by S, 3
  by K), and 4 other functions through a cascade.

| Measure | BEFORE | AFTER | Delta |
|---|---:|---:|---:|
| Functions Safe / provisional Unsafe / Unknown | 306 / 12 / 4,516 | **382 / 12 / 4,440** | +76 / 0 / −76 |
| Units Safe / provisional Unsafe / Unknown / data-only | 3 / 10 / 168 / 4 | 3 / 10 / 168 / 4 | 0 |
| leak / unknown_alias / unsupported | 37 / 3,507 / 43,590 | 37 / 3,517 / 43,215 | 0 / +10 / −375 |
| Return refusal: leaves / functions / sole (Unknown functions) | 495 / 417 / 147 | **305 / 249 / 69** | −190 / −168 / −78 |

## 1. Baseline reproduction

`run_audit.py measure/before-census b160d765 --census` with a release build of
the base carrying the `indirect-census` feature ([before-run.json](before-run.json):
binary `7939a7eb…`, exit 0). [before-identity.json](before-identity.json):
diagnostics, chunk-contracts and Monkey source **byte-identical** to the
baseline; contracts identical after the root alias; source-effects differ only
in checkout-path digests, as at every earlier card. The census feature changes
no artifact (W43 measured it; the AFTER census run below reproduces the AFTER
artifacts byte for byte too).

## 2. Re-census by family

W43's `return_census.py`, unchanged, over the BEFORE stream
([census/census-before.txt](census/census-before.txt)): **496** published
leaves, all covered, **495** on **417** Unknown functions, **149** leaves on the
**147** sole-blocked ones. The brief's families, with the enclosing function's
result carrier read from its definition line in the pinned source
([scripts/family_table.py](scripts/family_table.py),
[census/family-table.txt](census/family-table.txt)); leaves / functions /
sole-blocked functions on Unknown functions:

| Family | BEFORE | scalar / pointer result | AFTER |
|---|---:|---:|---:|
| member, subscript or dereference read | 289 / 262 / 79 | 170 / 119 | 144 / 130 / 27 |
| call inside a larger expression | 52 / 48 / 18 | 47 / 5 | 52 / 48 / 18 |
| conditional or short-circuit over a call | 27 / 25 / 17 | 18 / 9 | 27 / 25 / 17 |
| conditional or short-circuit, no call | 26 / 26 / 17 | 20 / 6 | 13 / 13 / 6 |
| whole call under a cast or in parentheses | 26 / 21 / 11 | 13 / 13 | **0** |
| type operator (`sizeof(LZ4_stream_t)`) | 6 / 6 / 4 | 6 / 0 | **0** |
| comma operator (nanopb `PB_RETURN_ERROR`) | 67 / 32 / 1 | 67 / 0 | 67 / 32 / 1 |
| statement expression or compound literal | 2 / 2 / 0 | 1 / 1 | 2 / 2 / 0 |
| **total** | **495 / 417 / 147** | 348 / 153 | **305 / 249 / 69** |

The 147 sole-blocked functions were overwhelmingly **accessors** (`node->id`,
`acl->operation`, `(item->type & 0xFF) == (1 << 5)`,
`error && error->fatal ? 1 : 0`, `sizeof(LZ4_stream_t)`), refused for the
*member typing* of their operand — an unreadable `* const` base, an enumeration
a typedef defines, a typedef'd record, an array member — that no return rule
had a reason to need: when the function returns a scalar, C converts the value
to that scalar (C11 6.8.6.4p3), so its custody is settled by the declaration,
not by the member. That is what family S uses. The pointer-result half cannot
use it (§5).

## 3. The rules and their custody

Both live in `crates/qcore-c-gateway/src/legacy_return_family.rs`, consulted
by `Parser::return_expression_instruction` only after every older route (the
strict and pointer-leaf rules of constraint 5b, W44's cast return, the W4 whole
call, both B01 routes and W43's refused-whole-call lowering) has declined; a
thin hook in `legacy_c.rs` and one `Parser` field carry them. No new
`LegacyInstruction` exists, so `descriptor_custody::admits_descriptor_read`
needs no new arm.

**S — scalar-result read.** Premises, each fail-closed:

1. the enclosing function's **definition** declares a scalar value result:
   `TypedAccessModel::is_scalar_value_type` (the classifier's `Scalar` class,
   plus an enumeration a typedef defines, read from its head), no `*`, a
   readable prefix (attribute groups skipped);
2. the expression is a read: no call of any kind (a name, member, subscript or
   parenthesised operand followed by `(`), no brace, assignment, `++`/`--`,
   comma, GNU `?:` or unary `&` (a cast group before `&` makes it unary);
3. a type operator only over identifiers the reader resolves as a never
   variably modified type (`never_variably_modified`, W44's VLA argument); a
   pointer type name keeps the fixed type-query rule's refusal and its pins;
4. no name that may hold a tracked descriptor is mentioned;
5. every evaluated name is `UntrackedBases::never_bound` (C02-P premise 4) or
   a declared scalar; a name the body may bind is admitted only as the root of
   exactly one dereference (`B->F`, `B->F.G`, `B[I]`, `*B`) in a position C
   always evaluates (not after a `?`, `&&` or `||` of its group).

**Custody:** the exit is `NoOp` — a scalar carries nothing. The evaluation node
is `IndirectScalarAccess` with no destination, frame or chain: every directly
dereferenced root is a base, keeping the typed route's flow obligation
(unbound, a non-null untracked binding, or one known, non-null, live identity,
else `unknown_alias`; a released base is a `use_after_release` candidate), and
every `->` root keeps its C02-P-ALIAS obligation. Premise 5's position clause is
what confines that accusation to dereferences C performs; an untracked name is
never released by the body, so its check can refuse but never accuse.

**K — cast of a whole call.** `( T1 ) … ( Tn ) CALL ( … )`, possibly
parenthesised, every `Ti` identifiers the reader resolves as a type followed by
`*`s — so `(f)(x)`, a call of `f`, is never read as a cast. The synthetic
`PLACE = CALL(…)` goes through `classify_simple` exactly as W4's does:

| Classification | Casts | Answer | Custody |
|---|---|---|---|
| refusal | any | lowered: the refusal, then `NoOp` | none; still poisoned; counter restored |
| `None` result | any | admitted as W4 | nothing escapes |
| `Owned` result | all pointer, callee declared to return an object pointer | admitted as W4 | the fresh site, and only it, escapes (C11 6.3.2.3p7) |
| `Owned` under an integer cast, `Reallocated`, `Derived` | any cast | return refusal kept | — |
| any | none (parentheses only) | W4's own admission | as W4 |

**Evaluation order.** Both families lower to one evaluation node before the
exit node at the `return`'s own line, so every read and call happens before the
function returns.

**Consistency.** Constraint 5b's rules and W44's cast return answer first and
are untouched; K handles only what the cast return leaves to the pre-card parse
(a cast of a call). The derived-result machinery is untouched: S never applies
to a pointer result, and K refuses a derived result under a cast (a caller that frees what a wrapper returning `(char *) d_chr(…)` hands back
is never certified; the same caller of the uncast wrapper is not certified
either — both pinned).

## 4. Arbiter AFTER and attribution

Implementation `ca84f6b9`; frozen forecast `054c554c`
([PREDICTION.md](PREDICTION.md)). AFTER ([artifacts/run.json](artifacts/run.json)):
`run_audit.py … --jobs 16`, exit 0. A census run of the same rules
([census/after-census-run.json](census/after-census-run.json), with the
census-only `QCORE_RETURN_FAMILY_CENSUS` stream the next commit adds)
reproduces all five AFTER artifacts byte for byte. Against the baseline
([after-identity.json](after-identity.json)) only the diagnostics differ;
chunk-contracts and the Monkey source are byte-identical, and contracts and
source-effects are byte-identical to the same-checkout BEFORE.

[scripts/attribute.py](scripts/attribute.py) (exit 0,
[attribution/attribution-summary.txt](attribution/attribution-summary.txt),
per-leaf [attribution/attribution.json.gz](attribution/attribution.json.gz))
keys the direct half on the analyzer's own record of every return the module
decided, and accepts a delta only as:

* **direct** — a removed return leaf the family census decided: **189**
  (163 scalar reads, 22 cast calls lowered, 4 admitted);
* **exchange** — the classifier's own refusal at a lowered cast call's line:
  **22** added (14 lexical indirect access, 8 uncontracted call);
* **base-obligation** — an `unknown_alias` naming a read's base at its line:
  **4** added (`rd_kafka_topic_partition_cmp`'s `a` and `b`, copies of unbound
  formals; `rd_kafka_anyconf_is_modified`'s `confhdr`; `mk_vhost_create`'s
  `h`) — refusals, not accusations;
* **cascade** — a leaf whose statement calls a function the card decided (or a
  helper or macro whose body calls one): **209** removed, **14** added. A
  decided accessor validates a W4 summary, so its callers' calls are
  classified or their conditions desugared (constraint 5h) where they were
  refused: 110 branch-, 16 for-, 7 loop-condition leaves, 53 uncontracted-call,
  18 indirect-access and 5 nested-call leaves go; 10 `unknown_alias`, 3
  nested-call and 1 formal-in-expression refusal appear where a newly modeled
  call exposes the rest of its statement. 206 at call-graph distance 0 (the
  statement names a decided function: `rd_list_cnt` 101 leaves,
  `rd_kafka_msgq_len`, `LZ4_read32`, …), 17 at distance 1 through a helper or
  macro (`rd_list_empty`, `RD_MAP_FOREACH_ELEM`, `cio_file_st_get_content`);
* **cascade-flow** — **7** removed leaves of a function that decided a return
  itself or calls a decided function, whose own statement does not name it: 4
  `unknown_alias` (a copy of a formal a newly validated entry binds; `data`
  after `cio_file_st_get_content`), 2 returns of
  `mk_http_parser_content_length` (its formal `p` is no longer handed to an
  uncontracted `mk_http_parser_is_content_chunked`, so the older typed chain
  route admits `p->header_content_length`), 1 do-while condition whose origin
  is its `do`.

**Moves.** All **76** are Unknown → Safe. **72** lose only direct leaves, and
they are exactly the 72 sole-blocked functions freed: 69 by S, and 3 by K —
`XXH32_createState` and `XXH64_createState` (owned pointer casts) and
`rd_kafka_header_cnt` (a `None`-result cast). **4** move through a cascade:
`get_available_size` and `rd_kafka_ConsumerGroupDescription_member_count`
(their only leaf called a newly summarized accessor),
`mk_http_parser_content_length` (a condition desugared and two returns the
older chain route now admits), and `mykey_hash` (its own return decided, and the
`unknown_alias` of its formal's copy gone once the validated entry binds it). Of the 147 sole-blocked
functions ([attribution/sole-moves.json](attribution/sole-moves.json)): 72 are
Safe, 8 now carry another reason (a lowered cast call's own refusal, or a base
obligation), 67 are still only return-blocked; 2 functions became sole-blocked
when a cascade cleared their other reasons.

**Leaks and violations.** No leak, double release, use after release or
invalid release leaf is added or removed; no function or unit moves to
provisional Unsafe. The only escaping admission is K's `Owned` pointer cast
(`XXH32_createState`, `XXH64_createState`), whose returned value *is* the
tracked object the call created; S escapes nothing and reads no tracked value
as data, so it withdraws no leak (`QCore.ReturnExpressionFamilies.the_read_withdraws_no_leak`).

**Forecast comparison.** Forecast: ≈312 leaves and ≈80 sole functions after,
67 freed (50–67). Measured: 305, 69, 72. The five unforecast frees are
`rd_kafka_error_is_fatal`, `rd_kafka_error_is_retriable`,
`rd_kafka_error_txn_requires_abort` (the forecast read the census's
`FunctionTypes::may_bind` fact, which reads the second `&` of `e && e->f` as an
address; the rule uses `UntrackedBases::never_bound`), `rd_kafka_header_cnt`
(a `None`-result cast call; the forecast counted only `Owned` K admissions as
frees) and `rd_kafka_message_delivery_count` (the forecast counted
`rkm->rkm_u.consumer.delivery_count` as a chain; it is one dereference). The
forecast stated no cascade function by name; four occurred. Leak, provisional
Unsafe and unit figures held. The misses are recorded, not corrected into the
forecast.

## 5. What stays refused, and why

Residual sizing of the 305 AFTER leaves by the first S or K premise they fail
([census/residual-sizing.txt](census/residual-sizing.txt); the result carrier
is read from the source's definition line, so this sizes the residual rather
than replaying the Rust):

| Obstacle | Leaves / functions / sole | Residual |
|---|---:|---|
| pointer-valued result (`node->host`, `rk->rk_name`, `&rk->rk_conf`) | 140 / 129 / 37 | `W45-RETURN-POINTER-RESULT-READS` |
| a call inside a scalar-result expression | 65 / 59 / 30 | `W43-RETURN-COMPOUND-CALL` |
| comma operator with a member store (nanopb) | 68 / 33 / 1 | `W43-RETURN-COMMA-MACRO` |
| a bound base in a skipped position or under a second dereference | 15 / 14 / 1 | `W45-RETURN-BOUND-BASE-POSITIONS` |
| a record-object member of a bound name, an aggregate result, an unresolved result typedef | 14 / 13 / 0 | `W45-RETURN-BOUND-BASE-POSITIONS` |
| address-of, increment, braces | 3 / 3 / 0 | — |

A **pointer-valued** read is deliberately not admitted: `B->F` is a load when
`F` is a pointer member, but an interior address when `F` is an array (the
decay of `rk->rk_name`), and `&B->F` always is. An interior address is a result
*derived* from `B`, which must keep its derivation (`Derived` with `nullable`
and `stepped`); published as a result that carries nothing, it would let a
caller free `B` and keep using the result unaccused. Telling the two apart needs
the member typing this card sidesteps only for scalars. The call-bearing
compound and conditional families need the evaluation model of W44's condition
desugaring applied to a *value*; not taken here.

## 6. Tests

* `crates/qcore-c-gateway/tests/return_expression_families.rs` (11 rows, real
  Clang): the accessor shapes decided `Safe` with a certificate that rechecks;
  a live owned base read and not returned is a true leak; a released base is a
  `use_after_release` candidate; a bound base after `?` or `||` keeps the
  refusal and accuses nothing (counterexample); an interior pointer of a
  released object and of a borrowed one keeps the refusal (counterexample); a
  call inside the expression, an address and a tracked descriptor keep it; an
  accessor validates a borrowing summary and a caller that releases first is
  not certified; K's pointer cast is exactly the uncast return (a caller that
  frees the wrapper's result is decided, one that drops it leaks); an integer
  cast of an owned call keeps the refusal while one of a `None` result is
  decided; a refused call under a cast agrees with the uncast lowering; a
  derived result under a cast keeps the refusal and a caller that frees it is
  never certified (counterexample).
* `crates/qcore-c-gateway/tests/unit/legacy_return_family.rs` (10 rows): the
  shape readers over a fixed name oracle — the effects that refuse, the bound
  name's one-dereference premise, conditional positions, `&` and `=` readings,
  the cast chain, the definition-result walk.
* **Deliberate re-pins**, each documented in place: `return_position_modeled_call.rs`
  (`return (void *) q_alloc();` is now the uncast return),
  `return_refused_call_lowering.rs` (a cast of an uncontracted call is lowered;
  `sizeof (struct w_pair)` is decided; the "old answer" reference spelling moves
  from a cast to `CALL + 0`), and `tests/unit/legacy_c.rs` (`return *p;` after
  a release is the same `use_after_release` candidate `if (*p)` already was).
  Each still asserts non-`Safe` wherever the old row did, except the
  `sizeof` constant, which is now decided.

## 7. Lean

`proof/QCore/ReturnExpressionFamilies.lean`, registered in
`proof/AuditReturnExpressionFamilies.lean`, 23 theorems. S: the rule only
replaces the return refusal and only for a scalar result; an admitted read
mentions no descriptor; every bound name it reads is an unconditionally
evaluated direct root and a base; under premise 4 an accusation is on a
dereference C evaluates; the exit returns nothing and withdraws no leak the
refusal reported. K: a refused call is lowered to its own refusal; parentheses
alone are W4; an owned result under casts is a pointer conversion of a
declared object pointer; an integer cast, a reallocated and a derived result
refuse; an admitted cast call discharges nothing the body owed (through
`QCore.ReturnCallCustody.admitted_modes_discharge_nothing_prior`); the
evaluation precedes the exit; a declined or lowered probe restores the counter;
both families are non-vacuous. Abstract rule theorems; extraction, the
declaration readers and the Rust correspondence are constraint-28 premises.

## 8. Findings

* The census-sink guard (`census_sink_inventory::every_declared_sink_forces_one_job`)
  failed on the base with the feature on: W44-INDIRECT-CALL-ARGUMENT-ACCESS's
  `QCORE_CALL_ARGUMENT_CENSUS` was unlisted, so a parallel audit would have
  interleaved it. It is listed now (two visibility widenings, one line).
* `FunctionTypes::may_bind` and `UntrackedBases` read the second `&` of
  `a && p` as an address of `p` when `p` does not continue into a member. Both
  over-approximate, so it costs precision only; S uses `never_bound`, which
  the `e && e->f` accessors pass.

## 9. Validation

The standing gate through [scripts/run_gate.py](scripts/run_gate.py) (W44's
runner with the `test` step now `scripts/run_test_binaries.py`), each exit
captured directly from the process, `CARGO_INCREMENTAL=0`, the dev profile
reclaimed after the test step and after the core steps. Rust, Lean and tests
were measured at `e3d6fdb0`; later commits change the report, the ledger,
STATUS and the increment record only. Facts: [gate-facts.json](gate-facts.json);
generator input: [gate.json](gate.json); logs `private/gate/` and
`private/test-binaries/20260925T153613Z-3166676/` in this worktree.

| Command | Exit | Result |
|---|---:|---|
| `git submodule update --init --recursive` | 0 | libevent at `d82464a2…` |
| `clang --version` | 0 | Ubuntu clang 21.1.8, recorded here ([clang-version.txt](clang-version.txt)) |
| `python3 scripts/run_test_binaries.py` | 0 | `test binaries: 290 units, 300 summaries; 5665 passed; 0 failed; 11 ignored` |
| `cargo fmt --all -- --check` | 0 | clean |
| `cargo clippy --workspace --all-targets -- -D warnings` | 0 | clean |
| `cargo run -p qcore-compiler --example reevaluation_probe` | 0 | 8/8 rows pass |
| `cargo run -p qcore-compiler --example reevaluation_followup` | 0 | 11/11 rows pass |
| `python3 scripts/check_lean_audits.py --jobs 4` | 0 | **190** Lean audits, no project axioms or placeholders |
| `cargo clippy -p qcore-c-gateway --features indirect-census --all-targets -- -D warnings`; `cargo test -p qcore-c-gateway --features indirect-census --lib` | 0 | clean; 543 passed (the sink guard included) |
| `python3 scripts/build_docs_index.py --check` | 0 | the index matches every report directory |
| `python3 docs/reports/remaining-d02/build_matrix.py . --gate docs/reports/w45-return-expression-families/gate.json` | 0 | 393 follow-ups, 190 audits |
| `python3 -m unittest discover -s scripts/tests` (after the regeneration) | 0 | **351** tests |

Rust 5,665 = main's 5,644 (299 summaries) plus this card's 11 integration rows
(one new binary) and 10 unit rows. No provider file changed, so the provider
gate is not run and its 124 tests are carried. Attempts that are not the gate,
disclosed: the first focused runs of the new integration file failed 3 of 11
rows while the rule used `FunctionTypes::may_bind` (§8) and while two rows
pinned an older route's answer; the first full gateway run failed 6 rows (exit 101):
the five §6 then re-pins, and `neutral_expression_positions.rs`'s
`sizeof(void *)` row, which the rule now honours by refusing a pointer type
name under a type operator (§3); a census-feature guard
run failed on the base's unlisted sink (§8); and a Python run before the matrix
regeneration failed its stale-derived-figure row (`lean_audits` 189 vs 190),
as it must.

## 10. Ledger and status

* Retired: `W43-RETURN-CALL-UNDER-CAST` (the cast family is 0 after the card,
  positively recognised, the refused half lowered, the owned half admitted
  under pointer conversions with a Lean statement and refused under integer
  ones) and `W43-RETURN-TYPE-OPERATOR` (0 type-operator return leaves after;
  the fixed type-query rule elsewhere is unchanged)
  ([retired-followups.json](retired-followups.json)).
* Registered: `W45-RETURN-POINTER-RESULT-READS`,
  `W45-RETURN-BOUND-BASE-POSITIONS` ([followups.json](followups.json)).
* Amended with W45's re-measured sizes: `W43-RETURN-ACCESS-OPERANDS`,
  `W43-RETURN-COMPOUND-CALL`, `W43-RETURN-CONDITIONAL-OPERANDS`,
  `W43-RETURN-COMMA-MACRO` ([amended-followups.json](amended-followups.json)).
* `corpus.cycle19_raw_units` names this report's artifacts; its previous row is
  [previous-corpus-row.json](previous-corpus-row.json).

## 11. For the integrator

Proposed gateway rule (constraint **5j**):

> **A scalar-result return decides a read; a cast of a whole call is the
> call.** In a function whose definition declares a scalar value result, a
> `return` expression that reads (no call, write, comma, brace, `?:` elvis or
> unary `&`; no tracked descriptor; untracked or scalar names, and a name the
> body may bind only as the root of one always-evaluated dereference) lowers to
> a typed read over its dereferenced bases and a `NoOp` exit: the value
> carries nothing. `return (T) … CALL(…);` is classified as W4 classifies the
> call: a refusal is lowered to itself, a `None` result is admitted, an `Owned`
> result only under pointer casts of a declared object pointer (escaping its
> fresh site); everything else keeps the return refusal. A pointer-valued read
> is never this rule's: an interior address is a derived result.
> `legacy_return_family.rs`; `QCore.ReturnExpressionFamilies`.

The contracts artifact embeds this worktree's path in its source-map keys; a
same-checkout re-run at the root is the byte-identity test for that artifact.

## Reproduction

```sh
python3 docs/reports/w43-return-expression/scripts/return_census.py docs/reports/w45-return-expression-families/census/return-census-before.jsonl.gz docs/reports/w44-collection-allocation/integration/artifacts/diagnostics.json.gz
python3 docs/reports/w45-return-expression-families/scripts/predict.py measure/census-before.json measure/before-census/return-census.jsonl
python3 docs/reports/w45-return-expression-families/scripts/attribute.py docs/reports/w44-collection-allocation/integration/artifacts/diagnostics.json.gz docs/reports/w45-return-expression-families/artifacts/diagnostics.json.gz docs/reports/w45-return-expression-families/census/return-family-after.jsonl.gz
python3 docs/reports/w45-return-expression-families/scripts/sole_moves.py docs/reports/w44-collection-allocation/integration/artifacts/diagnostics.json.gz docs/reports/w45-return-expression-families/artifacts/diagnostics.json.gz
python3 docs/plans/cycle19-custody-cards/blocker_census.py docs/reports/w45-return-expression-families/artifacts
```

The attribution replays from the archived inputs byte for byte (its summary
equals [attribution/attribution-summary.txt](attribution/attribution-summary.txt)).
Private raw outputs: `measure/before-census/`, `measure/after/`,
`measure/after-census/` in this worktree.
