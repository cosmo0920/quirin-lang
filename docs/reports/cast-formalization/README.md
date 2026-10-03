# CAST-FORMALIZATION: one formal cast semantics for route G

Card CAST-FORMALIZATION (wave 48, round 16; the user's priority, 2026-10-03:
"We'll always use type casting on our code base so we really wanted to
formalize this"). Base: local `main` at `cb3085365`; rebased onto main
`9c5f2079c` when CG26-TAIL-RESIDUAL-ACTUALS (rule 12g) landed, and every
figure of record below is measured on the rebased tip (the first pair, on
`cb3085365`, is kept in `artifacts/` and §4). Gateway
[rule 12h](../../agents/gateway.md#constraint-12h), amending
[rule 11b](../../agents/gateway.md#constraint-11b) (and, through the view,
rule 11j; through the trailing-actual reader, rule 12g). Lean
`QCore.CastSemantics`, audited by `proof/AuditCastSemantics.lean` (21
theorems; 283 → 284 audits on the rebased base). [PREDICTION.md](PREDICTION.md)
was frozen at `bb2aabe4a` (rebased as `8e90d0a9e`), after the census
instrument (`2598f6e4e`, rebased as `20a591246`; verdict-neutral, checked
byte-identical on both bases) and before any hook that can change a verdict
(`fc2014fb3`, rebased as `f7c5ee4c2`).

## 1. The model: one conversion table

Rule 11b read a cast by its Clang kind only. A kind does not decide what a
conversion does: Clang spells a byte view `(char *) obj`, a typed view of a
`void *`, a type pun `(struct B *) a`, a qualifier conversion and a
function-pointer conversion all `BitCast` or `NoOp`. `cast_semantics.rs`
refines each kind by the operand's and the result's type classes (Clang's
desugared spellings: scalar, `void`, pointer to `void` / a character type /
a function / another object, `_Atomic` and `volatile` noted) into one
**conversion class** (`classify`), and `CONVERSION_TABLE` states once, per
class, its identity effect, the access an lvalue through the result may
make, and the obligations an admission names (C11 §6.3, §6.5.4):

| Class | Kinds | Identity | Access through the result | Obligations |
|---|---|---|---|---|
| `scalar` | the six scalar kinds, scalar `NoOp` | data | — | CG20 (rule 11q) |
| `null-test` | `PointerToBoolean` | lost | — | |
| `pointer-to-integer` | `PointerToIntegral` | lost | — | the pointer carries no tracked identity (11b) |
| `integer-to-pointer` | `IntegralToPointer` | absent | untracked | |
| `null-pointer` | `NullToPointer` | absent | null | |
| `byte-view` | `BitCast`/`NoOp` to `void *` or a character pointer | preserved | character (6.5p7, no premise) | |
| `typed-view` | `BitCast`/`NoOp` from `void *` or a character pointer to another object pointer | preserved | effective type | `P-EFFECTIVE-TYPE`, `P-ALIGNED-POINTER-CONVERSION` |
| `punned-view` | `BitCast` between two other object pointers | preserved | **punned: refused** | `P-ALIGNED-POINTER-CONVERSION` |
| `qualifier` | `NoOp`/`BitCast`, same pointee modulo qualifiers | preserved | same | an added `volatile` keeps rule 11h |
| `function-pointer` | `BitCast`/`NoOp` between function pointers | preserved | callee | CG22 (rule 11p) |
| `function-object` | `BitCast` between a function and an object pointer | refused | refused | (not C) |
| `discard` | `ToVoid` | no value | — | |
| `atomic`, `address-space`, `array-decay`, `function-decay` | their kinds | refused / CG22 | refused / callee | CG19, CG16, CG22 |
| `unlisted` | anything else | refused | refused | |

The table refines rule 11b's `CAST_TABLE`, never forks it:
`agrees_with_cast_table` (Rust) and `QCore.CastSemantics.agrees` (Lean) check
that every class's kinds are `CAST_TABLE` rows (or the function decay) and
that its identity effect is the kind row's provenance flag (kept ↔
preserved, lost ↔ lost, not applicable ↔ absent / no value / data; a scalar
`NoOp` is data).

## 2. Every position consults it (`legacy_cast_semantics.rs`, thin hooks)

| Hook | Position | Decision | Order |
|---|---|---|---|
| **H-SR5** | `return` operand, rule 5j (`legacy_return_family.rs::stars_before`) | a cast group is transparent to the dereference count (`cast_transparent`): `*(T *)q->f` is two dereferences, refused for a may-bind `q` | soundness, asked where the count is read |
| **H-V** | member root `((T *)p)->f`, dereference `*(T *)p`, subscript base `((T *)p)[i]` (the typed access-path reader) | `void_view::explicit_view` asks `cast_semantics::access_view`: `typed-view` (now also from a character pointer) and `byte-view` are views, read under rule 11j's premises unchanged (untracked root or never-bound load, the C02-P-ALIAS check); a `qualifier` conversion is read through with no view (`transparent_cast`); `punned-view`, function/object and every other class refuse | the reader's own step (rule 11j's place) |
| **H-U0** | call actual, variadic tail actual (lexical classifier) | an identity-preserving pointer conversion of a place is that place: a call the classifier admitted with such an actual is classified as its uncast statement, whose answer replaces the original | **soundness repair, asked first** |
| **H-U** | call actual, variadic tail, pointer-step operand, initializer, assignment value | a refused statement's identity-preserving pointer conversions of a plain name (and, in a variadic tail only, a `scalar` conversion of a non-descriptor scalar) are removed and the uncast statement is lowered by the older routes, kept only where they admit it; rule 11w's tail widening is applied with the original statement's Clang node | last, only for a refusal |
| **H-12g** | variadic tail actual, rule 12g's Clang reader (`variadic_tail_actuals.rs::TailExpr::from_node`) | a `scalar` or identity-preserving pointer conversion is read through to its operand's own reading (`cast_semantics::tail_transparent`); every other cast keeps 12g's `cast` refusal; the uncast tail statement of H-U carries 12g's readings for its other positions | 12g's own place (asked where T1-T8 refuse) |
| **H-T** | the elaborator's object-pointer `BitCast`/`NoOp` arm; rule 11b(3)'s uncast operand | ask the class; a non-preserving class refuses by name (`cast \`CLASS\` (CG08)`) | unchanged order; no verdict change |

Returns and call-free conditions keep the elaborator's reading
(rule 11b(1), CG02), which already decides each kind by its `CAST_TABLE` row
and now asks the class for an object-pointer `BitCast`/`NoOp`. A cast at an
access position is never removed by H-U (its type is the access's).

### The soundness repair H-U0 (found writing the controls)

On the base, the lexical classifier read a call actual that is not a place as
**no place**. A `Borrowed` actual `(void *) t` was therefore never checked
against `t`'s state: `struct T *t = q_alloc(); q_free((void *) t);
q_observe((void *) t);` was **Safe** (the release itself was seen; the use
after it was not), while `q_observe(t)` was refused. H-U0 classifies such a
call as its uncast statement, so the use is `UnknownAlias` of `t` (pinned in
`tests/cast_formalization.rs`). On the corpus it was consulted 25 times, every
time admitted; it moved one row (`rd_dl_close`, Unknown → Unknown, §4) and
**no Safe function**: the hole had no instance on a Safe path of the
arbiter's inputs.

### The W45-SR5 dereference count

`W45-SR5-SCALAR-READ-STAR-THROUGH-CAST`: `stars_before` stopped at the `)` of a
cast group, so `return *(T *)q->f;` read one dereference of `q`. Before a
member or subscript root (where every `*` counts and only refuses), a group
spelling a type name is now read through (`cast_semantics::type_group_open`).
A starred root `*(T *)q` keeps its refusal (no widening). Bound to
`QCore.CastSemantics.sr5Vectors` by
`a_star_through_a_cast_counts_as_a_dereference` (rule 5j admits each vector
exactly when its count is one). No corpus row moved.

## 3. The census (task 1)

`record_cast_census` (parity runs only) records every explicit cast
(`CStyleCastExpr`) of a body with its Clang kind, class, position and its
statement's line; `scripts/census.py` joins them to the route's leaves by
function and line (pair of record: [before](artifacts/rebased/census-before2-corpus.txt),
[after](artifacts/rebased/census-after2-corpus.txt); first pair:
[before](artifacts/census-before-corpus.txt),
[after](artifacts/census-after-corpus.txt)). Explicit casts only: an implicit
conversion is not a cast in the source, and Clang inserts thousands. The
script gained the refusals column (category `unsupported`) after the
prediction froze; the BEFORE census was recomputed from the same BEFORE run,
its other figures unchanged.

| | BEFORE | AFTER |
|---|---|---|
| explicit casts / bodies | 16,743 / 3,225 | 16,743 / 3,225 |
| leaves on a cast's line / of them refusals (`unsupported`) | 10,383 / 7,768 | 10,371 / 7,732 |
| functions with such a leaf | 2,273 | 2,268 |
| Unknown functions every leaf of which is on such a line | 285 | 280 |

The join is by line: a macro-expanded line holding many statements (the
`rd_kafka_buf_*` macros) associates each of its leaves with each of its casts,
so these are upper bounds on "a refusal whose statement holds a cast", not on
"a refusal a cast causes"; most of them carry another blocker (an indirect
access, an uncontracted call) on the same statement. `NULL` is itself a
`null-pointer` cast: 7,169 casts, 4,051 joined refusals.

By class × position (casts / joined refusals / Unknown functions, BEFORE →
AFTER):

| Cells | Casts | Refusals | Unknown fns |
|---|---|---|---|
| H-V: `typed-view`, `byte-view`, `qualifier` at an access position | 131 | 69 → 65 | 43 → 43 |
| refused by name: `punned-view` at an access position | 38 | 29 → 29 | 12 → 12 |
| H-U: identity-preserving pointer classes at value positions | 1,735 | 1,072 → 1,046 | 771 → 766 |
| H-U, H-12g: `scalar`, `byte-view` in a variadic tail | 691 | 1,499 → 1,496 | 204 → 201 |
| `function-object`, `integer-to-pointer`, `pointer-to-integer`, `unlisted` | 256 | 142 → 141 | 94 → 94 |

The full class × position × family table is in the census JSON. What the
remaining refusals are, read from it: a cast over arithmetic or a call
(`(char *) p + off` with `p` a member, `(T *) f()`), a cast whose operand is a
member path at an initializer or step (H-U removes only casts of a plain
name), statements whose other parts the older routes refuse (an indirect
access, an uncontracted call, a macro statement), and the classes the table
refuses.

## 4. Measurements

`scripts/remeasure.sh` (the instrument commit built and measured as
`before2`, the card's tip as `after2`, each with `scripts/build.sh` and
`scripts/measure.sh`: corpus arbiter `66910c10 --jobs 4`, system route,
tail-pause slice, each under the shared lock, each with
`QCORE_TYPED_PATH_PARITY`), `scripts/merged_totals.py`, `scripts/compare.py`,
`scripts/census.py`. Runs in this worktree's `private/castf/`; the slice under
`private/w47-tail/attempt-1/castf-*` of the main checkout.
[Totals](artifacts/rebased/totals.txt).

| Run | BEFORE (main `9c5f2079c` + instrument, binary `1893c152…`) | AFTER (`4fbcab80d`, binary `a6c1a706…`) |
|---|---|---|
| Corpus | `b864e7ba…`, **649 / 9 / 4,176** (main's digest, byte for byte) | `d13ed4c1…`, **654 / 9 / 4,171** |
| System | `390e8cf1…`, 44 / 2 / 188 | `9a2b18bd…`, **44 / 2 / 188** |
| Tail-pause slice | `6b9ac710…`, 29 / 1 / 238 / 2 Pending | `6b9ac710…`, byte-identical |

Corpus leaves: leak 40 → 40, summary conflict 2 → 2, unknown_alias 11,482 →
11,524 (+42), unsupported 28,712 → 28,658 (−54).

### 4.1 Every corpus mover ([compare](artifacts/rebased/compare2-corpus.json))

39 rows move; **+5 Unknown → Safe, no Safe → non-Safe, no correction, no leak
or violation added.**

| Hook | Functions |
|---|---|
| H-U, initializer `T *const p = (T *) formal;` — **Safe** (5) | `lz4frame.c::LZ4F_readLE32`, `LZ4F_readLE64`, `LZ4F_writeLE32`, `LZ4F_writeLE64` (`const BYTE *const srcPtr = (const BYTE *) src;`: rule 11b(3) declined it because the destination type `T *const` fails its pointer test); librdkafka `snappy.c::unaligned_copy64` (the `(u64 *)(src)` / `(u64 *)(dst)` actuals of its load/store helpers) |
| H-U, refusal → unresolved identity, or the tail refusal gone (19) | `lz4.c::LZ4_getIndexOnHash`, `LZ4_getPositionOnHash`, `LZ4_wildCopy8`, `LZ4_wildCopy32`; `lz4frame.c::LZ4F_compressBegin_usingCDict`, `LZ4F_compressEnd`, `LZ4F_compressFrame_usingCDict`, `LZ4F_compressUpdateImpl`, `LZ4F_decompress`, `LZ4F_flush`, `LZ4F_makeBlock`; `lz4hc.c::LZ4_initStreamHC`, `LZ4_slideInputBufferHC`; `rdkafka_assignor.c::rd_kafka_member_subscriptions_map`, `ut_destroy_metadata`, `verifyNumPartitionsWithRackMismatch0`; `rdkafka_range_assignor.c::rd_kafka_range_assignor_assign_cb`, `rdkafka_sticky_assignor.c::rd_kafka_sticky_assignor_assign_cb` (a `punned-view` initializer: the identity is kept, accesses through the destination stay its declared type's, as rule 11b(3)); `rdkafka_conf.c::rd_kafka_anyconf_get0` (`(void *) val` in `snprintf`'s tail) |
| H-12g, the tail refusal gone, no leaf added (4) | CG26-TAIL-RESIDUAL-ACTUALS' cast residual, every leaf of it: `rdkafka_cert.c::rd_kafka_conf_set_ssl_cert` (`(int) cert_type`), `rdhdrhistogram.c::ut_quantile` (`(int64_t) i`), `rdmap.c::unittest_untyped_map` (`(float) ts_get / 1000.0`, `(float) ts / 1000.0`: two leaves), `snappy.c::rd_kafka_snappy_java_uncompress` (`(ssize_t) inlen - of`; provisional Unsafe on its other leaves, unchanged) |
| H-V, refusal → unresolved identity (4) | `mk_server/mk_utils.c::mk_utils_gen_hash`, `rdmurmur2.c::rd_murmur2`, `snappy.c::read_uncompressed_length`, `snappy.c::refill_tag` (a `typed-view` of a character pointer read at its first element) |
| H-U0, refusal → unresolved identity (1) | `rddl.c::rd_dl_close`: `dlclose((void *) handle)` was admitted with no place for `handle` and then refused by the contract check; it is now `handle`'s release, whose binding is unresolved |
| callers of a newly Safe body (W4 summaries; 6) | `lz4frame.c::LZ4F_decodeHeader`, `LZ4F_headerSize` (their `LZ4F_readLE32` conditions); `snappy.c::emit_literal`, `incremental_copy_fast_path`, `writer_append_from_self`, `writer_try_fast_append` (their `unaligned_copy64` calls now carry a summary; refusal → unresolved identity or a contract-match refusal of the summary) |

`compare.py` labels the H-12g rows and the callers `other` (no
`cast-semantics-value` row: 12g's reader admits them before H-U is asked);
each was read by hand. Against the prediction: Unknown → Safe 5 (predicted 2
to 12, most likely 5); no Safe → non-Safe (predicted none); unsupported −54
(predicted −100 to −800: fewer cast statements were a statement's only
refusal than the line join suggested); unknown_alias +42 (predicted up to
half the fall: met); no new leak (predicted at most 3). Consultations
(de-duplicated): H-U0 25 admitted; H-U 44 admitted, 2 tail admissions, the
rest still refused.

### 4.2 System route and slice

System ([compare](artifacts/rebased/compare2-system.json)): no status move;
two functions change leaves, each in both of its compile configurations:
`flb_storage.c::flb_storage_quarantine_chunk` loses its `snprintf` tail
refusal (the `(void *) src` actual of CG26-TAIL's cast residual, its two
system leaves; predicted), and `flb_task.c::flb_task_create` trades two
indirect accesses for one unresolved identity of `task` (H-V).
`flb_coro_destroy`'s `((char *) coro) + sizeof(struct flb_coro)` is now 12g's
pointer-arithmetic refusal (the cast is read through, the arithmetic is not);
it carries no reported leaf. The slice is byte-identical.

### The first pair (base `cb3085365`)

Before the rebase the card measured on `cb3085365` (binaries `49439047…` /
`5cce8256…`): corpus `450a9e54…` 649 / 9 / 4,176 → `63bbb5a3…` 654 / 9 /
4,171, the same five Safe moves, 36 rows, unsupported −50, unknown_alias +42;
system `9108b7c1…` → `198c9d24…` 44 / 2 / 188 with the same two functions;
slice byte-identical ([compare](artifacts/compare-corpus.json),
[totals](artifacts/totals.txt)).

### 4.3 The shared probe: `mk_vhost_set` at the tip

`scripts/tip-probe.sh` (the integrator's `tip-contracts.sh` on a private copy
of the probe tree; [rows](artifacts/rebased/tip-probe-rows-after2.txt)),
contracts-only route, AFTER binary (the first pair's rows,
[rows](artifacts/tip-probe/rows-after.txt), are the same):

| Copy | Function | Before (main, the rows CONTRACT-NORETURN-AND-BORROWED-TAILS recorded) | After |
|---|---|---|---|
| tip (`21a2c190`) | `mk_vhost_set` | Unknown: uncontracted `mk_vhost_set_property` (`:533`) | the same |
| tip | `mk_vhost_set_property` | Unknown: two indirect accesses (`:502`, `:503`, the `DocumentRoot` stores) | the same |
| master | `mk_vhost_set` | Unknown: six `va_end` protocol refusals and `mk_vhost_set_property` | the same |
| master | `mk_vhost_set_property` | Unknown: the same two indirect accesses | the same |

As predicted: no cast is among the probe's blockers (`mk_vhost_set_property`
has none; its `DocumentRoot` stores are the real behaviour the custody card
noted), so the card moves nothing there. The base's own binary was not kept
for a BEFORE probe run; the BEFORE rows are those the previous card recorded
on the same probe tree at the code main now carries.

## 5. Lean

`QCore.CastSemantics` (`proof/QCore/CastSemantics.lean`, 21 theorems, audited
by `proof/AuditCastSemantics.lean`, no placeholder, no project axiom; the
axiom audits print only `propext`, `Classical.choice`, `Quot.sound`):

- classification: `function_object_refused`, `view_rows`,
  `classify_vectors_hold` (over `classifyVectors`);
- the table: `classes_unique`, `agrees` (with `CastRepresentation.castTable`),
  `refused_rows`;
- identity: `view_preserves_identity` (byte, typed, punned and qualifier
  rows), `round_trip_same_pointer` (through `void *` or a character pointer
  and back, the same pointer), `pointer_to_integer_loses_provenance`,
  `tracked_pointer_to_integer_refused`, `integer_to_pointer_no_identity`,
  `null_pointer_no_identity`, `refused_classes_refuse`;
- access: `admitted_access_without_premise` (only same type or character),
  `view_access_needs_the_premise` (only a typed view, under
  `P-EFFECTIVE-TYPE`), `punned_access_refused`;
- effect: `cast_keeps_events`, `discard_is_effect_only`;
- W45-SR5: `cast_transparent`, `star_through_cast_is_two`, `sr5_vectors_hold`.

Bound to Rust by shared vectors: `lean_conversion_table_matches_the_rust_table`
(`conversionTable` = `CONVERSION_TABLE`, row for row),
`lean_classify_vectors_are_the_rust_vectors` (the Rust reader's type classes of
`CLASSIFY_VECTORS`, rendered, are `classifyVectors` row for row; Lean proves
its `classify` agrees on each), and
`a_star_through_a_cast_counts_as_a_dereference` (`sr5Vectors` through rule
5j's reader).

## 6. Tests and deliberate flips

New: `tests/cast_formalization.rs` (6 tests: the admitted views at each
access position, the refused views (pun, tracked root, `volatile`), the value
positions (byte view at a release and a step, the leak and the use after
release still found: the H-U0 repair), the refusals without an identity
(integer to pointer, tracked pointer to integer, function to object), `(void)
e`, and casts in a declared borrowed tail including the descriptor refusal);
unit tests in `tests/unit/cast_semantics.rs` (vectors, the table's rows, the
11b agreement, type classes, callee arity, the Lean bindings) and one in
`tests/unit/legacy_return_family.rs`.

| Test | Was | Now |
|---|---|---|
| `cg12_void_view_readback::unlisted_views_are_refused` | `*(int *)p` of an `int *p` refused (not a `void *` operand) | removed from the refused list: a `qualifier` conversion, read through (Safe; pinned in `cast_formalization.rs`) |
| `typed_access_path::tests::unsupported_pointer_store_cast_and_call_refuse` | the plan of `int x = *(int *)p;` over an `int *p` refused | its cast row is now the pun `*(int *)p` over a `long *p` (still refused); the `int *p` form is a qualifier conversion, read through |
| `cg26_tail_residual_actuals::casts_pointer_arithmetic_and_identity_arms_stay_refused` (rule 12g) | `(long) i`, `(float) i / 1000.0`, `(void *) p` in `fprintf`'s tail refused (left to this card) | moved to `casts_the_table_reads_as_their_operand_are_read` (Safe; a use after release through `(void *) p` is not); a pointer made an integer, `(unsigned long) p`, takes their place in the refused list |
| `variadic_tail_actuals::tests::cg26r_the_node_shapes` (rule 12g) | an `IntegralCast` cast node is `TailExpr::Cast` | it reads through to the operand's `Name`; a `PointerToIntegral` cast node is `TailExpr::Cast` |

## 7. Follow-ups

`scripts/update_ledger.py` applies these once, on the rebased ledger (698 →
697); sizes from the pair of record ([before](artifacts/rebased/families-before2.json),
[after](artifacts/rebased/families-after2.json)):

- **Retired:** `W45-SR5-SCALAR-READ-STAR-THROUGH-CAST` (its DISCHARGE holds: a
  `*` before a cast group that precedes a may-bind member or subscript root
  counts, with the unit pin bound to `sr5Vectors`), and
  `CG26-TAIL-RESIDUAL-ACTUALS` (its cast residual, as CG26-TAIL amended it:
  every listed leaf is read, corpus 6 leaves / 5 functions and system 2 →
  0, through the table, a scalar cast of data as data and a pointer view of
  an identified root as that root's `Borrowed` place, never a widening of
  the data tail).
- **Amended** (an earlier amendment of this card replaced):
  `CG08-CAST-AND-QUERY-RESIDUALS` (b) 307 leaves / 103 Unknown functions →
  282 / 84 by the census's Clang join, (c) 515 consultations / 297
  functions, unchanged ([before](artifacts/rebased/cast-refusals-before2.json),
  [after](artifacts/rebased/cast-refusals-after2.json));
  `W45-SUB-OBJECT-STORE-PREFIX` (the cast root's view is the table's in the
  typed reader; the lexical store route's cast-root family 33 / 18,
  unchanged); `W43-INDEX-TRAVERSAL-STEP-COMPUTED` (the cast half read; the
  pointer-step cast family 189 / 74, unchanged: member-path casts and
  offsets with accesses).
- **Registered:** `CAST-FORMALIZATION-RESIDUAL-CENSUS`, the residual census
  by class and position and by owner (punned and tracked-root views, casts
  of member paths or over arithmetic and calls at value positions, scalar
  conversions outside a tail, the refused classes, `NULL`, the elaborator's
  CG08 refusals).

Out of scope, not changed: route T (casts stay its own readers', CG01) and
the Monkey binding's `SizeOf` table (CG08-CAST-AND-QUERY-RESIDUALS (f)).

## 8. Gate

`scripts/gate.sh g2` at `6f3f23869` (each step under the shared lock, each
exit to a file, nothing piped; exits copied to [gate/](gate/), summary in
[gate.json](gate.json)); logs in this worktree's `private/castf/gate-g2/`.

| Step | Command | Exit | Counts |
|---|---|---|---|
| Check | `cargo check --workspace --all-targets` | 0 | |
| Submodule | `git submodule update --init --recursive` | 0 | |
| fmt | `cargo fmt --all -- --check` | 0 | |
| clippy | `cargo clippy --locked --all-targets -- -D warnings` | 0 | |
| Rust | `python3 scripts/run_test_binaries.py` | 0 | 437 units, 450 summaries; 7,525 passed, 0 failed, 11 ignored (`private/test-binaries/20261003T024249Z-2646281`) |
| Probes | `cargo run -p qcore-compiler --example reevaluation_probe` / `…_followup` | 0 / 0 | |
| Lean | `python3 scripts/check_lean_audits.py` | 0 | 283 audits (`AuditCastSemantics.lean` new) |
| Python | `python3 -m unittest discover -s scripts/tests` | 1, then 0 | 365; the failure was the matrix not yet regenerated (`lean_audits` 282 carried); after `build_matrix.py . --gate gate.json` (09a, 0), 365 OK (09b) |
| Index | `python3 scripts/build_docs_index.py --check` | 0 | |
| Provider gate | `scripts/provider-gate.sh` | 0 | `providers/` unchanged; run because `build_matrix.py` needs a measured `provider_tests`: check, fmt, clippy 0; 129 passed; `providers/tests` 24 OK; `contracts.py check`, package build and the five verifies 0 |

A first gate at `388af567c` (`gate-g1`) failed clippy on two test-only items
(now `#[cfg(test)]`) and one unit test pinning `*(int *)p` of an `int *p`
refused (the second deliberate flip of §6); its probes and Lean audits
passed. Clang 21.1.8, recorded in this worktree. The AFTER binary was built at
`fc2014fb3`; the later commits change no analyzer behaviour (an unused
variant and method removed, two test-only items gated, tests and records).

