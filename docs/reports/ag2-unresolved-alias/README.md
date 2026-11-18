# AG2-UNRESOLVED-ALIAS: copy sources that carry no resource identity

Card AG2-UNRESOLVED-ALIAS (round 33), 2026-11-18, base `a9c6dcf44`
(`integrate/cg25-thread-key`). Gap: `unknown_alias` "unresolved resource
identity `X`" in the user's `current --monkey-linked` run of 2026-11-18
(Fluent Bit `046accf6`; 1,100 Safe / 3,825 Unknown / 9 Unsafe function rows).

## 1. Sub-shape census

**Method.** [scripts/extract.py](scripts/extract.py) reads the user's report
(read-only) into its 4,934 function rows; [scripts/census.py](scripts/census.py)
classifies every `unknown_alias` leaf of an Unknown row against the original
source of the prepared tree; [scripts/shapes.py](scripts/shapes.py) groups the
leaves into sub-shapes and writes [artifacts/census.json](artifacts/census.json)
(per sub-shape: leaves, primary, secondary, rows mentioning, rows alone, the
alone rows, and up to six sampled sites). The census is **lexical**: a name's
class (literal, enumerator-like constant, pointer or scalar formal, local
pointer/scalar/array, file-scope or undeclared) comes from the function's
parameter list and declaration lines by regular expression, and the statement
shape from the leaf's origin line. It is a triage instrument, not evidence of
any verdict; the readings this card admits are decided on Clang's AST (§3).

**Primary and secondary.** A leaf is *secondary* when another refusal of the
same row explains it: a refused statement on the same line (576 leaves), an
earlier refused statement whose text mentions the name (the refusal poisoned
its binding, rule AOM-BINDING-POISON; 8,272), a formal handed to an
uncontracted callee of the row (W47's `formal-handed-uncontracted`; 79), or a
local assigned from a refused call (5). Every other leaf is *primary*: the
identity is unresolved on its own.

| Measure | Value |
|---|---:|
| Unknown rows with an `unknown_alias` leaf | 2,280 |
| `unknown_alias` leaves on them | 11,233 |
| primary / secondary leaves | 2,301 / 8,932 |
| rows whose alias leaves are all primary / all secondary | 408 / 1,275 |
| rows blocked by `unknown_alias` alone | 69 |

**Sub-shapes** (rows are unit configurations, as the report counts them;
"alone" = the row's only blockers are leaves of this sub-shape):

| Sub-shape | Leaves | Primary | Secondary | Rows | Alone |
|---|---:|---:|---:|---:|---:|
| S1 pointer formal copied into a local (`T *d = (T *) ctx;`) | 379 | 347 | 32 | 328 | **27** |
| S5 local pointer read, handed or returned (member read origin, loop cursor) | 3,710 | 485 | 3,225 | 1,154 | 11 |
| S4 local pointer in a condition (`if (!(p = q->f))`, `while (p)`) | 1,512 | 468 | 1,044 | 705 | 6 |
| S9 other (local arrays, scalar formals and locals) | 368 | 59 | 309 | 163 | 5 |
| S6 pointer formal used directly (release, store, actual) | 4,238 | 428 | 3,810 | 1,087 | 4 |
| S2 enumeration constant copied or tested (`err = RD_KAFKA_RESP_ERR_NO_ERROR;`) | 252 | 249 | 3 | 111 | 1 |
| S3 string literal copied into a local (`s = "Info";`) | 133 | 133 | 0 | 47 | 1 |
| S7 file-scope object or undeclared name (`r = rd_kafka_global_cnt;`) | 410 | 123 | 287 | 183 | 1 |
| S8 lowering placeholders (`__qcore_nested_value_N` 8, `__ctx` 3, `__qcore_returned_value` 55, `__tmpstr` 115, `_vp` 50) | 231 | 9 | 222 | 55 | 0 |

The finer split by statement shape is in `census.json` (`table.detail`). The
69 alias-only rows: S1 27 (15 cmetrics `unpack_*` decode callbacks, seven
librdkafka comparators, four callbacks, `get_context_arena`); S5 11; S4 6; S9
5; S6 4; S2, S3 and S7 one each; the remaining 13 combine sub-shapes (for
example `format_acl_error_message` S3 + S4, `LZ4F_optimalBSID` S2 + S7,
`mk_esc_filename` S1 + S3 + S5).

**The brief's sample sites.** `cmt_create` (`cmt`, 11 leaves) and the latter
`rd_kafka_error_copy` leaves (`error`) are secondary: an earlier refused
statement mentions the name. `rd_murmur2` (`key`) and `mk_utils_gen_hash`
(`key`) are S1 (`const uint32_t *data = (const uint32_t *) key;`), their
`data` leaves S5. `rdkafka_lz4`/`rdkafka_zstd` carry S2
(`RD_KAFKA_RESP_ERR_NO_ERROR`, `RD_KAFKA_RESP_ERR__BAD_COMPRESSION`) and S6
(`slice` handed to `rd_slice_remains`). `mk_print` carries S3 (12 literal
copies). `__qcore_nested_value_0` in `mk_cache_worker_exit`,
`mk_sched_worker_free` and `mk_server_worker_loop` is the result of a nested
`MK_TLS_GET(...)` (a `return_none` value) released by `mk_mem_free`: a
release of a value this body holds no custody for, a correct refusal (S8).

**What earlier cards decided** (nothing below is re-admitted against it):

- `W43-INDEX-TRAVERSAL` left a copy of an unbound *formal* refused on purpose:
  W4 summary derivation binds exactly the formals an analysis names
  (`legacy_summary::bind_unresolved_formals`), and removing that signal was
  measured to stop it (`may_alias_boundary::a_guard_still_discharges_two_locals_that_are_not_a_definite_alias`).
  Its follow-up `W43-INDEX-TRAVERSAL-FORMAL-COPY` and W47's
  `W47-ALIAS-FORMAL-COPY` ask for exactly the S1 reading with the signal kept
  and the formal's never-bound answer still required.
- Rule 54 reads only *thread-local* objects and says **do not** read a static
  (non-thread-local) object under it; S7 (file-scope objects) is therefore not
  admitted here.
- `W47-ALIAS-TYPED-SCALAR-NAMES` registered S2/S3 (enumeration constants and
  literal actuals as identities) with the discharge "bind nothing for a
  non-pointer object, an enumeration constant or a literal"; this card is that
  discharge for copies.
- `LEXICAL-SCALAR-NOOP-ALIAS-READ` found that a read through a name whose
  same-type peer was released must refuse; S1's reading leaves that check in
  place (fixture `adv-peer-free`, §4).
- `W47-ALIAS-CALLEE-BORROWED-CLOSURE` owns formals handed to uncontracted
  callees (S6, and the secondary class here): it needs callee contracts, not
  an alias reading.

## 2. Chosen shapes and why each is sound

Gateway rule 62 ([rule text](../../agents/rules/62.md),
`crates/qcore-c-gateway/src/custody_free_copy.rs`) reads three of the
census's primary sub-shapes, all of one form: route G lowers `D = S;` to one
`Alias` node, and when `S` has no binding the old transfer reported `S` and
gave `D` an unknown, possibly null binding. Where the reading admits the copy
and `S` is unbound on the incoming state, `D` is left **unbound** with no
report, exactly the transfer W43-INDEX-TRAVERSAL already applies to a copy
between two declared non-formal pointers and rule 54 to a copy of a
thread-local pointer. Every reading writes only an automatic local with one
declaration that every reference designates.

* **S3, literals.** `__qcore_literal__` is the tokenizer's placeholder for a
  string or character literal: the address of a static array (C11 6.4.5p6)
  or an `int` (6.4.4.4p10). No instruction ever writes the placeholder, so
  it is unbound in every reachable state (`a_never_written_name_stays_unbound`)
  and its copy denotes no object this body acquired. Withheld where a
  declaration or reference of the function spells the placeholder.
* **S2, enumeration constants.** A spelling every reference of which is a
  `DeclRefExpr` to an `EnumConstantDecl`, with no declaration of the function
  spelling it: an integer constant (6.4.4.3p2) that designates no object.
  The traversal exemption missed it because an enumerator is not a declared
  variable and the declaration reader does not type a typedef of an
  anonymous `enum` (`rd_kafka_resp_err_t`, `LZ4F_blockSizeID_t`) as a scalar.
* **S1, read-only copies of an unbound pointer formal.** The formal is
  single-level (`T *`, not `T **`), declared once, never written or
  address-taken, and rule 10l's pointer-root premise answers for it; its
  copies are the automatic object-pointer locals receiving its whole value
  (through parentheses and `NoOp`/`BitCast` casts), closed under copying.
  Every reference to a copy is a rebinding, a copy into another copy, a null
  test, a `->`/`[]`/`*` base or a call actual, and the never-bound closure
  answers for every copy. The copy then denotes exactly what the unbound
  formal denotes (`the_copy_denotes_what_the_source_denotes`), so every
  later decision on its binding is the one the formal gets
  (`every_binding_decision_agrees`): a read through it is admitted only where
  the existing rules admit a read through an unbound never-bound name, a
  release or transfer of it refuses, and a release of a same-type peer still
  blocks a read through it (`LEXICAL-SCALAR-NOOP-ALIAS-READ`'s check runs on
  the copy's name). The fixtures pair each admitted body with the same body
  written on the formal directly: both are `Safe` at base and tip.

**The W4 signal is kept.** W43-INDEX-TRAVERSAL kept a formal copy's report
because W4 binds exactly the formals an analysis names. Any copy use that
could need the binding (a `return`, a store of the copy's value, `&`, a write,
or a custody-taking call actual, which the never-bound closure marks) withholds
the reading for the formal and every copy, so the old transfer and its report
stand there (`withheld_keeps_the_report`): the release wrappers of
`may_alias_boundary` and the fixtures `a_released_copy_keeps_the_w4_signal`
still derive their `Released` summaries. Where the reading is admitted the
body never hands the formal on, so W4 no longer binds it and the validated
summary reads it `Borrowed`, the mode a direct dereference of the formal
already gets (the formal-direct controls), instead of the stricter `Owned`.

## 3. Refused shapes and their named reasons

| Sub-shape | Why it stays refused | Owner |
|---|---|---|
| S1 copies with a custody use (returned, stored, address-taken, stepped, handed to an uncontracted callee) | the copy is the formal handed on; the report is W4's binding signal; a stepped walk (`p = s; while (*p) p++;`) is a write | `W43-INDEX-TRAVERSAL-FORMAL-COPY` (amended) |
| S4/S5 locals bound from a member read or a call, loop cursors | the value comes from a place this body does not track; an identity read off a member is rule 5b/10l's opaque field read, refused at a condition, a release or a return | `W47-ALIAS-FLOW-RESIDUALS` (amended) |
| S6 formals used directly | a release or store of an uncontracted formal needs a contract or a W4 summary; `mk_config_print_error_msg` releases `path` and then calls `exit`, so W4 finds no reachable exit (its condition 2) | `W47-ALIAS-CALLEE-BORROWED-CLOSURE` |
| S7 file-scope objects (`r = rd_kafka_global_cnt;`) | rule 54 reads thread-local objects only and says not to read a static object; a static pointer object may be released through its copy elsewhere | `AG2-STATIC-OBJECT-COPY` (new) |
| S8 `__qcore_nested_value_N` (`mk_mem_free(MK_TLS_GET(x))`) | a release of a `return_none` value this body holds no custody for: a correct refusal until the thread-local slot's teardown relation is modelled | `W43-RESULT-ALIAS-CONDITIONAL-AND-RETAINED` (amended) |
| S2/S3 sources outside a copy into a local (scalar names in conditions and returns, literal call actuals, arrays) | not copies; the destination rule does not apply | `W47-ALIAS-TYPED-SCALAR-NAMES` (amended) |
| secondary leaves (8,932) | a consequence of another refusal on the row | AG1, AG3 and the callee-closure cards |
