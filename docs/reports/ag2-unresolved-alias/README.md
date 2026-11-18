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

## 4. Fixtures and Lean

[`tests/ag2_unresolved_alias.rs`](../../../crates/qcore-c-gateway/tests/ag2_unresolved_alias.rs)
(14 tests; every `Safe` rechecks its certificate). Admitted: a literal copy
on one or both arms (`Safe`, base `Unknown` on `__qcore_literal__`), an
enumerator of an anonymous-enum typedef (`Safe`, base `Unknown` on both
enumerators and the local), a callback context copied under a null test and
read through a member chain, and a copy whose member is returned (`Safe`,
base `Unknown` on the formal), each beside the same body on the formal
itself (`Safe` at base and tip). Near misses, unchanged from base: a release
of a literal copy, a literal joined with a live binding then released
(`unknown_alias:s`), a literal over a live acquisition (the `Leak` stays), a
literal into a formal, a shadowed destination, a file-scope scalar and
pointer object, a released same-type peer (`r/q` stays), a second copy
released, a copy stored, returned, handed to an uncontracted callee or
address-taken, a rebound formal, a loop cursor, and three release wrappers
whose W4 summary still validates (`a_released_copy_keeps_the_w4_signal`).
`may_alias_boundary` (32) and `thread_local_custody_free_read` (9) pass
unchanged. Unit tests (`tests/unit/custody_free_copy.rs`, 5): the decision
table against the vectors, the withholding uses, the type readers (a
pointer-to-pointer formal is not single-level) and `admits`.

[`QCore.CustodyFreeCopy`](../../../proof/QCore/CustodyFreeCopy.lean), audited
by `proof/AuditCustodyFreeCopy.lean` (17 theorems, axioms `propext`,
`Classical.choice`, `Quot.sound` only): `decision_vectors_hold` (bound to the
Rust table by `the_lean_decision_vectors_match_the_shared_table`),
`a_copy_is_admitted_exactly_on_a_read`, `a_custody_use_of_a_copy_withholds`,
`a_write_to_the_formal_withholds`, `no_premise_no_reading`,
`the_copy_changes_no_obligation`, `the_copy_denotes_what_the_source_denotes`,
`every_binding_decision_agrees`, `no_other_binding_changes`,
`the_admitted_copy_reports_nothing`, `the_withheld_answer_is_unchanged`,
`withheld_keeps_the_report`, `a_bound_source_keeps_the_old_answer`,
`a_release_of_the_copy_refuses`, `handing_the_copy_out_escapes_nothing`,
`step_keeps_other_bindings`, `a_never_written_name_stays_unbound`.

## 5. Shared-file hook points

All new logic is in `custody_free_copy.rs` (the reading) and
`legacy_custody_free_copy.rs` (the parser adapter). Shared files carry only
delimited hooks, each between `AG2-UNRESOLVED-ALIAS hook (rule 62): begin` and
`end` comments:

| File | Hook |
|---|---|
| `legacy_c.rs` | the `#[path = "legacy_custody_free_copy.rs"] mod custody_free_copy_route;` declaration after `typed_roots`; the `custody_free` field of `TraversalNames` (serde default, skipped when empty); the call `parser.custody_free_copies(&cfg, &scalars, &pointers)` before `cfg.traversal` is assigned, and the struct literal there |
| `legacy_transfer.rs` | one `LegacyInstruction::Alias` arm in `transfer_base`, after rule 54's and before the generic alias arm |
| `lib.rs` | `pub mod custody_free_copy;` after `thread_local_read` |

No change to `legacy_cfg_builder.rs` or `typed_access_path.rs`
(`TraversalNames::default()` covers every other CFG constructor).

## 6. Unit-scoped reproduction of `current`

Run after the semantic commit, against the frozen prediction. `quirin-audit
current --monkey-linked` with the user's argv, on a compile database filtered
to the 46 units of the alias-only rows, the brief's sample units, one
ctraces unit and `src/flb_input_chunk.c` (the route requires every selected
library; [artifacts/repro-units.txt](artifacts/repro-units.txt)), and the full
Monkey database: 49 configurations, 1,894 rows, through `gate_slot.py audit`
(`--jobs 3`, peak 2.1 GB). Base `a9c6dcf44` (binary sha256 `7745a8a3…`)
reproduces the user's report on these rows exactly in status; eight rows
differ only in run-wide callback-closure diagnostics a unit subset cannot
see ([artifacts/repro-fidelity.json](artifacts/repro-fidelity.json)). One
earlier base attempt died writing its report on a full disk (`No space left
on device`, the host's, not this card's) and is retained as
`private/ag2/repro-base-failed-enospc/`, not used. Tip (binary sha256 `5aa7ba7e…`, built from the
working tree committed as `cda5bfff6`, which then still held a test-only
off-switch read from `QCORE_AG2_PROTOTYPE_OFF`, unset in the run and removed
before the commit) against base
([scripts/compare.py](scripts/compare.py),
[artifacts/repro-compare.json](artifacts/repro-compare.json)):

| Measure | Base | Tip |
|---|---:|---:|
| rows Safe / Unknown / Unsafe | 335 / 1,557 / 2 | **368 / 1,524 / 2** |
| `Unknown -> Safe` | | 33 |
| other moves | | 0 |
| `unknown_alias` leaves removed | | 278 |
| rows with changed content, status unchanged | | 118 (all Unknown) |

Moves against the prediction: 28 of the 29 declared moves happened. Missing:
`rd_kafka_broker_start_reauth_cb`, whose alias leaf is gone but which now
shows the refusal it hid, `rd_kafka_op_new0`'s constructed-state check
(`constructed-state-open`, a run-wide check of the stores into
`rd_kafka_op_s.rko_op_cb` that a unit subset may not decide as the full run
does). Unpredicted, all `Unknown -> Safe`:

* `LZ4F_optimalBSID` (S2 plus the returned scalar it unbinds), noted as
  possible in the prediction;
* `rd_kafka_op_err_destroy`: with the enumerator no longer reported, W4's
  retry with `rko` bound validates a `Released` summary (the body destroys
  `rko`), and the row is decided under it, as every W4 wrapper is;
* `rd_kafka_topic_partition_cmp_opaque`: a same-unit caller of
  `rd_kafka_topic_partition_cmp`, which now publishes a validated summary;
* `input_chunk_has_magic_bytes` (two configurations): its base row carried
  a `summary_conflict` because W4 bound `buf` through the copy
  `p = (unsigned char *) buf` and inferred `Owned`, against the declared
  `Borrowed`; with the copy read through, W4 infers `Borrowed`, the conflict
  disappears and the row is `Safe` under the declaration
  (`W43-INDEX-TRAVERSAL-FORMAL-COPY`'s "stricter than the `Borrowed` the walk
  needs", observed).

Diagnostics newly visible on rows that stay Unknown (each was hidden behind
a removed alias leaf): two `callback-uncontracted` (`unpack_metric_exp_histogram`,
`unpack_metric_summary`: callbacks that still have no summary), four
`derived result … from an unmodeled base expression` (`append_sample_to_metric`)
and the `constructed-state-open` above. No row left `Safe`; no Unsafe row
changed. Wall time is comparable (about ten minutes each, under host
contention).

## 7. Pinned expectations this card flips

The scoped gate's Rust suite (attempt 3, on `3fe4611bd`) failed six units.
Five are pinned expectations of exactly the old answers rule 62 replaces,
updated deliberately in the next commit with a `FLIPPED by
AG2-UNRESOLVED-ALIAS` note:

| Test | Old pin | New |
|---|---|---|
| `clang_gap_phase1::e14_g_reads_literals_except_a_character_initializer` | `char c = 'a'; return c;` Unknown on `__qcore_literal__` | Safe |
| `cg14_aggregate_values::a_string_initializer_is_a_declaration_with_no_bound_claimed` | `const char *s = "abc"; return s[3];` Unknown on `__qcore_literal__` | Safe |
| `clang_gap_phase2::m11_string_initializers_are_refused_or_unresolved` | the same body | Safe |
| `summary_validation_premises::a_copy_whose_reads_certify_under_protection_still_validates` | `get`'s W4 summary `Owned` (two bodies) | `Borrowed`; rows Safe as before |
| `summary_validation_premises::read_only_released_and_forwarded_formals_still_validate` | `k`'s summary `Owned` | `Borrowed`; the `Released` wrappers unchanged |

The sixth, `qcore-audit::libcurl_user_crate_versions`, is not this card's: its
"unreadable header" case removes the fixture's `curlver.h` and expects every
libcurl call held Unknown, but this host carries libcurl 8.18.0 development
headers (`/usr/include/x86_64-linux-gnu/curl/curlver.h`), which the probe
finds through the default include path; `init_once` is Safe with
`audited_version` 8.18.0 in force. Deterministic in isolation
(`private/ag2/t-libcurl.log`); registered as
`LIBCURL-VERSION-TEST-SYSTEM-HEADER`.

## 8. Records

- [PREDICTION.md](PREDICTION.md), committed before the semantic commit, with
  its `measure-prediction` block (29 declared moves on corpus and
  twelve-home).
- [integration-amendments.json](integration-amendments.json): retires
  `W47-ALIAS-FORMAL-COPY`; amends `W43-INDEX-TRAVERSAL-FORMAL-COPY`,
  `W47-ALIAS-TYPED-SCALAR-NAMES`, `W47-ALIAS-FLOW-RESIDUALS` and
  `W43-RESULT-ALIAS-CONDITIONAL-AND-RETAINED`; registers
  `AG2-STATIC-OBJECT-COPY`, `AG2-LITERAL-OR-CONSTANT-INTO-FORMAL` and
  `LIBCURL-VERSION-TEST-SYSTEM-HEADER`.
- Measurement command (the integrator's): `quirin-audit measure --side
  base=rev:a9c6dcf44 --side tip=rev:<tip> --declared
  docs/reports/ag2-unresolved-alias/PREDICTION.md` on corpus, twelve-home,
  system and slice, then the user's `current --monkey-linked` rerun.
