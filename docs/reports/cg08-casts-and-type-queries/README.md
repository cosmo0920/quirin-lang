# CG08-CASTS-AND-TYPE-QUERIES: casts as representations and type-query operands

Card CG08 of the [Clang gap card queue](../../plans/clang-gap-cards/README.md)
(wave 48, round 5; base `ca634dc1` = local `main` at dispatch, which contains
the round-5 base `3e69322c` and W48-QR-C-STRUCT-IMPORT); main `bcdc13b9`
(W48-QR-USABLE-RECORDS, corpus identity unchanged) merged at `998b0353`. The
loop-site soundness repair (CG04-LOOP-SITE-GENERATIONS) had not landed on
`main` when these figures were taken; §4 is measured on `ca634dc1` (BEFORE)
and `998b0353` (AFTER), and is to be re-taken on the repaired base.

- **Gap entries:** E15, E16, N11, M18 and N08 (representation only), on
  route **G** (the custody gateway) and, for N11, route **T**.
- **Evidence:** Q11, Q16, Q22, Q23, Q29, Q37, Q40 and example D.
- **Rule:** [gateway 11b](../../agents/gateway.md#constraint-11b) (opens the
  11 series; amends 5a, 10s and 10u, cross-referenced there).
- **Lean:** `QCore.CastRepresentation`, `proof/AuditCastRepresentation.lean`
  (19 theorems); `QCore.ElaborationCore`'s form table gains nine rows.
- **Prediction:** frozen in [PREDICTION.md](PREDICTION.md) (commit
  `24328291`) before any implementation commit.

## 1. What changed

### 1.1 The cast table and the type-query classes (`cast_representation.rs`)

`CAST_TABLE` lists the 17 Clang cast kinds the route reads, each with a
preservation flag, a provenance flag and an admission. No row has a custody
effect; what a cast can do is hide an identity (a pointer to an integer) or
invent one (an integer to a pointer), and the admissions forbid both.

| Kind | Preservation | Provenance | Admission in the elaborator |
|---|---|---|---|
| `IntegralCast`, `IntegralToBoolean`, `FloatingToBoolean`, `IntegralToFloating`, `FloatingToIntegral`, `FloatingCast` | possibly lossy | n/a | scalar value, `NoNumericClaim` (numbers are CG20's) |
| `PointerToBoolean` | possibly lossy | lost | the null test: the pointer's read, a truth value |
| `PointerToIntegral` | possibly lossy | lost | data only of a never-bound automatic pointer; otherwise `PointerEscape` refuses |
| `IntegralToPointer` | possibly lossy | n/a | a pointer value with **no** identity |
| `BitCast`, `NoOp` (object pointers) | preserving | kept | the same pointer value |
| `NullToPointer` | preserving | n/a | the null pointer |
| `ToVoid` | preserving | n/a | no value |
| `AtomicToNonAtomic`, `NonAtomicToAtomic` | preserving | kept | refused (CG19) |
| `AddressSpaceConversion` | preserving | kept | refused (CG08: no address-space model) |
| `ArrayToPointerDecay` | preserving | kept | refused (an address; CG16) |

Any other kind is refused by name. `query_operand` reads a
`UnaryExprOrTypeTraitExpr`'s operand from Clang's node: not variably modified
→ **unevaluated**, no event (`sizeof(*p)`, `sizeof p->f`, `sizeof f()`); a
`sizeof` of a variably modified type → **evaluated**, the size expressions
Clang lists, each once, reads only; `_Alignof` of a variably modified type,
an evaluated expression operand other than a plain name, and other traits →
refused. `clang_constant` is Clang's own evaluation of an integer constant
expression (the value its evaluator gives a probe enumerator).

### 1.2 Route G (`legacy_cast_representation.rs`, thin hooks)

| Hook | Where | What |
|---|---|---|
| H1 | `return`, **before** the older routes | a value Clang converts with `PointerToBoolean` is the elaborator's (events, then an exit that escapes nothing); else, only a never-bound root keeps the older answer; else refused |
| H3 | statement, **before** the older routes | the same for a declaration or assignment: a truth value binds nothing |
| H5 | CG02's hooks (statement, `return`, call-free condition) | also consulted for an explicit cast, a `GATE_KINDS` conversion or a type query; a parenthesised type-query operand is no call to the call-free premise |
| H4 | statement, after the elaborator | `T *NAME = (U *) V;` / `NAME = (U *) V;` with object-pointer `BitCast`/`NoOp` casts is lowered as the uncast statement by the older routes |
| H2 | the W44 cast return's pre-card arm | `return (INTEGER) NAME;` with Clang's `PointerToIntegral` of a possibly bound `NAME` is refused (the W44 refusal) |
| H6 | the lexical gate of an ordinary call statement | a parenthesised `sizeof ( … )` whose every Clang node on its line is unevaluated joins W44's operand spans (the marker) |

H1 and H3 are asked before the older routes because the older routes'
premise (the value is the pointer) is false for them; every other hook is a
fallback (hard invariant 3). CG03's data reading (`call_instantiation.rs`)
takes `FloatingToBoolean` and the null test of a named pointer object.

### 1.3 Route T (N11)

`typed_source.rs::enum_constants` read an enumerator's value only from an
explicit initializer's evaluated `ConstantExpr` and refused an implicit one.
An implicit enumerator's value is now `clang_constant` (Clang's evaluation,
never the host's count from its predecessor); one Clang cannot evaluate at
file scope refuses. Every other T consumer is unchanged; the Monkey binding's
`SizeOf` host table is a residual (§6).

### 1.4 Shared items added (for the integrator)

All additive; no existing variant's meaning changes.

- `elaboration.rs`: `Refusal::PointerEscape`; `Context::untracked_pointer`
  (default `false`); `Elaborator::representation`, `Elaborator::type_query`;
  `pointer_operand`; nine `FORM_TABLE` rows (`representation cast`, `pointer
  null test`, `pointer to integer`, `integer to pointer`, `pointer
  representation`, `null pointer constant`, `discarded value`, `unevaluated
  type query`, `evaluated type query`; 29 → 38). The `CStyleCastExpr`,
  `UnaryExprOrTypeTraitExpr` and implicit-cast arms consult the new methods
  only where they refused before.
- `legacy_elaboration.rs`: `holds_value_form` also names a CG08 form;
  `GContext::untracked_pointer`; the call-free premise of conditions reads
  type-query operands as no call.
- `call_instantiation.rs`: two data-reading cast rows.
- `typed_source.rs`: the implicit enumerator arm.
- Nothing in `typed_place.rs`.

## 2. The soundness repair (found reproducing E15)

On base `ca634dc1` (fixture contracts, `tests/cg08_casts_and_type_queries.rs`):

| Fixture | BEFORE | AFTER |
|---|---|---|
| `bool f(void) { void *p = q_alloc(); return p; }` | **Safe** (the return escaped `p`) | Unsafe, leak |
| `bool f(void) { void *p = q_alloc(); bool b = p; return b; }` | **Safe** (`b` bound to `p`'s identity) | Unsafe, leak |
| `bool b; b = p; return b;` | **Safe** | Unsafe, leak |
| `bool b = p; q_free(p); return b;` | Unsafe (false custody conflict) | Safe |
| `return (bool)p;`, `return p != 0;`, `return !p;` | Unsafe, leak | unchanged |

C converts the pointer to `_Bool` (C11 6.3.1.2): the caller or the
destination receives a truth value, never the pointer. **No corpus function
moved** because of H1/H3 (no Safe → non-Safe in any run); the defect had no
corpus instance on the arbiter's inputs.

## 3. The cited claims

| Claim | Reproduced? |
|---|---|
| Q11: T reads six cast kinds plus `LValueToRValue` | yes (CG00's E15/M18/N08 T controls stand unchanged) |
| Q16/Q37: the typed readers refuse casts inside paths | yes; unchanged (a path cast stays CG10's) |
| Q22/Q23, example D: `1` is a class marker, not a size | yes; no route reads a number from it, pinned by `the_marker_never_reaches_a_bound_or_a_branch` |
| Q29: the contract schema carries no pointer/integer provenance | yes; the representation lives in the elaborator only |
| Q40: G's cast return | yes; the uncommitted-operand gap reproduced (`void *p … return (unsigned long)p;` was provisional Unsafe, a leak) |
| CG02: 165 consultations refused by a cast kind, 67 the nanopb `IntegralToBoolean` | **partly**: on this base the elaborator's CG08 refusals are 204 de-duplicated consultations over 106 functions (68 pointer `NoOp`, 67 `IntegralToBoolean`, 62 `BitCast`, 2 type-query operands, …); the 67 nanopb returns reproduce exactly (67 returns, 32 functions). CG02 measured on its own base, before CG03–CG06 widened the consultations; the difference was not attributed further |
| CG00's 34 candidates | 2 already Safe on this base (freed by earlier cards) |

Corrections to the input: none beyond CG00's recorded E16 partial correction.
M18's "G restricts cast-return forms" was true in the accusing direction only:
an uncommitted operand's integer return was a leak, not a refusal (now
refused).

## 4. Measurements

Commands: `scripts/measure.sh` (corpus arbiter `run_pinned_audit.py … 66910c10
--jobs 8` with `QCORE_TYPED_PATH_PARITY`, the system route and the
tail-pause slice); artifacts from `scripts/artifacts.sh`
([identity](artifacts/identity.txt)). Runs are in this worktree's
`private/cg08/` (`before/`, `base-before*`, `after-998b*`).

### 4.1 Identity and totals

| Run | BEFORE (`ca634dc1`, binary `393ce0f1`) | AFTER (`998b0353`, binary `d699f988`) |
|---|---|---|
| Corpus arbiter | `c012e785…`, **586 / 10 / 4,238** (the baseline, byte for byte) | `be132742…`, **602 / 10 / 4,222** |
| System route | `b0e18a3b…`, 34 / 2 / 198 | `f49826a6…`, **37 / 2 / 195** |
| Tail-pause slice | `22825272…`, 26 / 1 / 242 / 1 Pending | `644d1392…`, unmoved |

The AFTER on `998b0353` is byte-identical to the AFTER on the card's own tip
before the merge (`be132742…`, `private/cg08/base-after`), and the system and
slice reports too: W48-QR-USABLE-RECORDS moves nothing here.

**Corpus: +16 Unknown → Safe, no Safe → non-Safe, no correction.** Corpus
diagnostics ([totals](artifacts/diagnostic-totals.txt)): leak 36 → 37,
summary conflict 1 → 2, unknown_alias 11,343 → 11,695 (+352), unsupported
31,865 → 31,153 (−712); 386 same-status functions change leaves.

### 4.2 Every mover, attributed ([attribution](artifacts/corpus-attribution.json))

| Hook | Functions |
|---|---|
| H4 (uncast pointer statement) — 11 | `lz4.c::LZ4_putIndexOnHash`, `flb_input_chunk.c::flb_input_chunk_get_name`, `rdfnv1a.c::rd_fnv1a`, `rdkafka_assignor.c::rd_kafka_assignor_cmp_idx`, `rdkafka_share_acknowledgement.c::rd_kafka_share_ack_entries_offset_find_cmp_ptr`, `…::rd_kafka_share_ack_entries_sort_cmp_ptr`, `rdkafka_partition.c::rd_kafka_topic_partition_partition`, `nanopb/pb_encode.c::safe_read_bool`, `cmt_decode_msgpack.c::unpack_meta_aggregation_type`, `…::unpack_metric_start_ts`, `…::unpack_metric_ts` |
| H5, elaborated statement (`copy_bytes = sizeof(T) * n;`) — 1 | `rdkafka_admin.c::rd_kafka_AuthorizedOperations_copy` |
| H5 via CG04's leaf route (a `for` condition over `RD_ARRAYSIZE(p->f)`) — 2 | `rdkafka_conf.c::rd_kafka_anyconf_is_any_modified`, `rdkafka_conf.c::rd_kafka_conf_s2i_find` |
| H5 via CG06's edges (a `?:` initializer with type queries) — 1 | `rdkafka_sticky_assignor.c::ut_get_consumer_rack` |
| H6 (`memset(mdi, 0, sizeof(*mdi))`, read by hand; no parity row) — 1 | `rdkafka_request.c::rd_kafkap_leader_discovery_metadata_init` |

System route +3 ([attribution](artifacts/system-attribution.json)):
`flb_engine.c::calculate_chunk_capacity_percent` (H5, an elaborated scalar
`return` with a floating conversion), `flb_input_chunk.c::flb_input_chunk_get_name`
and `flb_task.c::flb_task_from_fs_storage` (H4).

### 4.3 Prediction against outcome

Of CG00's 32 remaining candidates the prediction put **at most 14** at Safe;
**10 moved** (12 of the 34 are now Safe, with the 2 already Safe):

- H4, predicted Safe (12): **8 moved**. `get_context_arena` now ends on an
  unresolved alias of `context` (the uncast `encoder = context;`), the two
  LZ4F writers' uncast `BYTE* const dstPtr = dst;` is refused by the older
  route too (a `* const` declarator), and `input_chunk_has_magic_bytes`
  carries unresolved aliases of `buf` and `p` (§4.4).
- H4, predicted Unknown for an alias leaf (2): `unpack_meta_aggregation_type`
  moved (its alias leaf was the refused cast's own); `unpack_metric_value_uint64`
  did not.
- H5: `rd_kafka_anyconf_is_any_modified` moved (predicted Unknown: its alias
  leaves were the refused condition's); `LZ4HC_rotatePattern` did not
  (`bitsToRotate`'s destination is not a lexical scalar local to rule 10s's
  event lowering).
- H6: `LZ4HC_clearTables` did not move (the array-member actual
  `hc4->hashTable` keeps its refusal).

Six movers are outside CG00's list (`flb_input_chunk_get_name`, `rd_fnv1a`,
`rd_kafka_AuthorizedOperations_copy`, `rd_kafka_conf_s2i_find`,
`ut_get_consumer_rack`, `rd_kafkap_leader_discovery_metadata_init`). The
soundness repair moved nothing: no corpus function returned or stored a
pointer as a truth value on a Safe path.

### 4.4 Added leaves, classified by reading

- **`mk_server/mk_scheduler.c::mk_sched_add_connection`** (provisional Unsafe
  at BEFORE, unchanged): one more leak, the allocation at line 201 (`data =
  mk_mem_alloc_z(size); conn = (struct mk_sched_conn *) data;`, now H4's
  alias) reaching the `if (!conn) … return NULL;` exit at line 210 exactly as
  the pre-existing leak from line 205's `else` allocation does, on the
  failure edge of the same null test. The same class as the existing leaf
  (the function's provisional Unsafe, not presented as an upstream defect):
  the `then` branch's allocation, once visible, is accused where the `else`
  branch's already was.
- **`flb_input_chunk.c::input_chunk_has_magic_bytes`** (Unknown, unchanged):
  a summary conflict (inferred `[Owned, Borrowed]` against the declared
  neutral `[Borrowed, Borrowed]`; the declared contract governs every call
  site) and unresolved aliases for `buf` and `p`. The uncast statement
  `p = buf;` produces the same inferred summary on base as the explicit cast
  now does (reproduced with fixture contracts): the older route's derivation,
  not a new premise.
- **`lz4hc.c::LZ4_slideInputBufferHC`** (Unknown, unchanged): one H2 refusal,
  `return (char*)(uptrval)bufferStart;` — a pointer laundered through an
  integer into the returned value, of a local the closure does not prove
  never bound. The correct refusal (it was the pre-card parse).
- **unknown_alias +352, unsupported −712**: statements this card now lowers
  expose later readings of the same names (the refusals they replaced had
  invalidated the bindings, rule 10a); no added leaf moves a status.
- Tail-pause slice: 12 functions change leaves only; one refusal
  (`publication by mk_list_add of dormant, which this function does not hold
  as one live object`) is newly reached past an admitted statement.

### 4.5 The absorbed families ([before](artifacts/families-before.json), [after](artifacts/families-after.json))

| Family | BEFORE leaves / Unknown functions | AFTER |
|---|---|---|
| `C operator sizeof requires type/operand analysis` (W44-SIZEOF-CAST-SCALAR-QUERY) | 458 / 121 | 91 / 31 |
| other unsupported leaves on a line spelling `sizeof` (W44-SIZEOF-OPERAND-RESIDUALS' neighbourhood, CG00's E16 feature) | 373 / 240 | 297 / 199 |
| `unsupported assignment expression` on a line spelling a pointer cast (CG00's E15) | 193 / 152 | 32 / 24 |
| the cast-return refusal | 0 | 1 / 1 |

The elaborator's CG08 refusals ([before](artifacts/cast-refusals-before.json),
[after](artifacts/cast-refusals-after.json)), de-duplicated by (function, line,
position): 204 over 106 functions → 1,320 over 509. The count grows because
the elaborator is now asked for every refused statement holding a cast or a
type query; 1,257 of the AFTER refusals are `ArrayToPointerDecay` of an array
actual (`snprintf(buf, sizeof(buf), …)`), an address the elaborator does not
read (CG16/CG10), 19 `NullToPointer` and 25 `BitCast` into a function pointer
or through an address, and 19 `PointerEscape` (a pointer read from a call or
a path converted to an integer).

`CG02-COMMA-OPERAND-RESIDUALS` (nanopb): the same 67 returns over 32
functions; the elaborator now reads the comma value's `IntegralToBoolean`,
and its first refusal is the place reader's (`place: the typed access-path
reader refused it (CG10)`) on the pointer-valued member store `stream->errmsg
= …` (CG11's store); rule 10z still refuses each by position.

### 4.6 `W45-CALLEE-CAST-LEXICAL-PREMISE` ([census](artifacts/callee-census.json))

Parity runs record, per function, every `CallExpr` whose callee is a
parenthesised name and every `CStyleCastExpr` to a one-identifier type over a
parenthesised operand, with what the normalised tokens spell on that line.
On the AFTER corpus: 1,437 sites, all casts to Clang; 1,436 are kept as the
group `( T ) (` by the lexical reader (agreement); the one line-join miss,
`mk_http.c`'s `(int)` / `(…)` split over two lines, was read by hand: a keyword
cast the W45 rewrite never touches. Clang shows **no** call through a
parenthesised name in the corpus, so W45's rewrite never fired there and the
unsound direction (a function read as a type) has no instance.

## 5. Lean

`QCore.CastRepresentation` (`proof/QCore/CastRepresentation.lean`, audited by
`proof/AuditCastRepresentation.lean`, no placeholder, no project axiom):

- `kinds_unique`, `provenance_rows` (only a pointer representation keeps
  provenance; every lost row is a null test or a pointer-to-integer);
- `cast_has_no_custody_effect`; `tracked_pointer_to_integer_refused`,
  `untracked_pointer_to_integer_is_data`; `integer_to_pointer_has_no_identity`;
  `pointer_to_boolean_is_the_null_test`; `pointer_value_keeps_identity`;
  `unlisted_cast_refused`, `refused_row_refused`;
- `constant_type_unevaluated`, `alignof_never_evaluated`,
  `unevaluated_operand_has_no_events`, `evaluated_operand_once`,
  `evaluated_effect_refused`;
- `consumed_value_is_clangs`, `marker_never_consumed`,
  `constant_vectors_consumed`;
- `non_typedef_non_tag_never_folds` (`SIZEOF-TYPE-FOLD-NO-LEAN`).

Bound to Rust by shared vectors: `castTable` = `CAST_TABLE`
(`cg08_lean_cast_table_matches_the_rust_table`); `queryVectors` through Clang
and `query_operand`; `constantVectors` against `clang_constant` (x86-64
Linux); `foldVectors` against `fold_type_sizes`
(`cg08_lean_fold_vectors_match_fold_type_sizes`). `ElaborationCore.formTable`
= `FORM_TABLE` (38 rows) by CG02's test.

## 6. Tests, flips and residuals

New: `tests/cg08_casts_and_type_queries.rs` (15 tests: the repair, the
integer round trip refused never Safe, `sizeof(int[f()])` refused, the marker
never in a bound or a branch, the uncast statements, the unevaluated
operands, T's implicit enumerator, the nanopb shape); unit tests in
`tests/unit/cast_representation.rs` and `tests/unit/elaboration.rs`
(`cg08_casts_and_type_queries_are_elaborated_by_their_rows`).

Deliberate flips of pinned refusals (each test names the card):

| Test | Was | Now |
|---|---|---|
| `clang_gap_phase1::e16_type_queries_are_read_only_in_fixed_shapes` | `sizeof(*s)` initializer IND, return refused, `sizeof(int[n])` IND | Safe (unevaluated; VLA extent read once); `sizeof(int[n++])` added, Unknown |
| `clang_gap_phase3::m12_variable_length_arrays_are_refused` | `sizeof(int[n])` IND | Safe (the VLA declaration refusal, CG17's, stands) |
| `branch_condition_calls::a_call_inside_sizeof_stays_refused…` → `…_is_never_applied…` | branch refusal | Safe, no `q_probe` node |
| `neutral_expression_positions::type_queries_do_not_hide_evaluated_calls_or_pointer_results` | 5 Unknown | 2 Unknown kept; `sizeof(q_alloc())`, `sizeof(*p)`, `sizeof(void *)` Safe |
| `neutral_expression_positions::expression_queries_do_not_erase_scalar_mutations` | branch refusal | a refusal **or** the leak (the mutation is forgotten, never the stale zero) |
| `sizeof_operand` (6 tests of `W44-SIZEOF-OPERAND-RESIDUALS`) | refused | Safe (array declarator, file-scope name, member operand, leading multiplication, no-call statement, type-operator-first statement); the two variably modified operands keep their refusal |
| `typed_field_access::ambiguous_accesses…` | `d->revision = (long) h;` refused | Safe (never-bound formal); `(long) p` of a tracked `p` added, Unknown |
| `legacy_condition_acquisition::every_refusal_arm_is_pinned` | fixture `sizeof(buf)` | fixture `sizeof(char[n++])` (the arm still pinned) |

CG00's E15, M18 and N08 controls pass unchanged: T is unchanged and G's
pointer↔integer assignments of a tracked pointer still refuse; N11's T control
(`sizeof(int)` stays a `SizeOf` node) is unchanged.

### Ledger

- **Retired** (every DISCHARGE clause holds): `CG08-CASTS-AND-TYPE-QUERIES`;
  the absorbed `W44-SIZEOF-CAST-SCALAR-QUERY` (the cast around a type query
  is read, 458 → 91 leaves, the rest of another cause), `SIZEOF-TYPE-FOLD-NO-LEAN`
  (`non_typedef_non_tag_never_folds`, bound to `fold_type_sizes` by
  `foldVectors`), `CAST-RETURN-UNCOMMITTED-OPERAND` (H2, Clang's
  `PointerToIntegral`; 1 corpus leaf), `W45-CALLEE-CAST-LEXICAL-PREMISE` (§4.6:
  cross-checked, the one miss reported and read) and
  `W44-SIZEOF-OPERAND-RESIDUALS` (its (a), (b) and the operand side of (c)
  and (d) admitted; what remains is re-registered with its size).
- **Amended**: `CG02-COMMA-OPERAND-RESIDUALS` (§4.5).
- **Registered**: `CG08-CAST-AND-QUERY-RESIDUALS`, by owner: the `sizeof`
  families' remainders (91 / 31 and 297 / 199), the cast-assignment remainder
  (32 / 24), the elaborator's remaining CG08 refusals (1,320 / 509, almost all
  array decay), a tracked pointer's integer used only in a comparison
  (refused), the explicit `(bool)p` condition (admitted by an older route
  with no null refinement), the Monkey binding's `SizeOf` host table on route
  T, and W44's declarator-reader quirk.

`scripts/update_ledger.py` applies these edits once.

## 7. Gate

`scripts/gate.sh private/cg08/gate-d82563f7` at tip `d82563f7` (each exit to a
file, no step piped; exits copied to [gate/](gate/), summary in
[gate.json](gate.json)):

| Step | Command | Exit | Counts |
|---|---|---|---|
| Submodule | `git submodule update --init --recursive` | 0 | |
| Rust | `python3 scripts/run_test_binaries.py` | 0 | 388 units, 401 summaries; 6,832 passed, 0 failed, 11 ignored |
| fmt | `cargo fmt --all -- --check` | 0 | |
| clippy | `cargo clippy --all-targets -- -D warnings` | 0 | |
| Probes | `cargo run -p qcore-compiler --example reevaluation_probe` / `…_followup` | 0 / 0 | |
| Lean | `python3 scripts/check_lean_audits.py` | 0 | 246 audits (AuditCastRepresentation.lean new) |
| Python | `python3 -m unittest discover -s scripts/tests` | 1, then 0 | 365; the one failure was the matrix not yet regenerated (`lean_audits` 245 carried); after `remaining-d02/build_matrix.py --gate gate.json`, 365 OK (`09b-python.exit`) |
| Index | `python3 scripts/build_docs_index.py` | 0 | |

Clang 21.1.8, recorded in this worktree. `providers/` is unchanged, so the
provider gate is not due and was not run. Logs:
`private/cg08/gate-d82563f7/` of this worktree.
