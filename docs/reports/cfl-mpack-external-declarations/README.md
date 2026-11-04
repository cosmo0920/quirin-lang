# CFL-MPACK-EXTERNAL-DECLARATIONS

Card brief: `private/integrator-briefs/r14-cfl-mpack.md` (round 14,
2026-11-05), which also applies `r13-common.md`. Base `c8c7edf21`. The card is
new. It is registered and retired through its
[amendments](integration-amendments.json).

> **USER REQUEST 2026-11-05** (verbatim): "Now, we're still needed to fill the
> gap of the missing contracts."

**Question.** The user's `quirin-audit current` run at Fluent Bit `046accf6`
reports 78 `cfl_`/`mpack_` callees as "call to uncontracted function", in 306
blocked (function, callee) pairs. None of them has a body in the run's 186
configurations. Which of them can a new resource-v1 declaration state
truthfully from the callee's body?

**Answer.** 55 callees, in 197 pairs, are now declared. The new text is in
the two already-selected modules: 30 CFL callees in `cfl/sds_lifetime.qv`
(`quirin-provider-cfl`) and 25 mpack callees in `mpack/writer.qv`
(`quirin-provider-mpack`). Each is pinned at 66910c10, 046accf6 and
34155955. The other 23 callees fall into three groups:

- **Undeclared, 7 callees.** The vocabulary cannot state their effect. Each
  is left undeclared with the missing form named as a follow-up.
- **Owned elsewhere, 7 callees.** These are the CFL list and arena
  callees.
- **Not CFL, 9 callees.** These are cmetrics' own `static` helpers that
  share the `cfl_variant_` prefix.

Following card 1's lesson (CONTRACT-ADOPTION-NOT-IN-FORCE §5), every
declaration is new resource-v1 text read from the body. None is adopted from
a bounds or model declaration. No analyser, gateway or Lean code changes, so
no semantics change and no Lean parity is due.

## 1. Inputs and method

- **Run report.** The main checkout's
  `build/cycle19/clang/046accf6aded7906f66a1baf0202da349e521736/fluent-bit-clang-source/reports/resource-diagnostics.json`
  (SHA-256 `e965950e…d017f`), read-only. The integrator's
  `private/research/contract-gap-20261105/` lists the same names.
- **Bodies.** The library sources at each pin, read from the Fluent Bit trees
  at 66910c10 (`build/cycle19/c-gateway-wave-pin`), 046accf6
  (`build/cycle19/clang/046accf6…`) and 34155955 (the tail-pause slice
  preparation). The census is [census/bodies.py](census/bodies.py), with its
  output in [census/bodies.txt](census/bodies.txt). It finds:
  - `lib/cfl/src`, `lib/cfl/include` and the cmetrics, ctraces and
    cprofiles copies of mpack are byte-identical at all three revisions.
    The one difference is the CMake-generated `cfl_info.h`.
  - No mpack callback installer or stream/file initialiser is called
    outside the mpack copies.
  - Of every audited compile database, only `lib/lwrb/dev/main.c` defines
    `_DEBUG`, so mpack's tracking is off wherever mpack is compiled.
- **Census.** [census/census.py](census/census.py) writes
  [census/census.json](census/census.json) and
  [census/census.md](census/census.md) (§3). It refuses an unclassified
  in-scope name, so the table is complete.
- **Rows.** [census/routes.py](census/routes.py) lists the in-scope rows of
  each baseline route. [census/cascade.py](census/cascade.py) is the
  upper bound on freed rows. [census/current.py](census/current.py) gives
  the user-run figures.
- **Pins.** [scripts/pin.sh](scripts/pin.sh) runs `quirin-audit pin` per
  crate and revision, then `pin --check` at each revision.

## 2. What each declaration states

The module comments carry the body citations, line by line. In summary:

| Form | Callees |
|---|---|
| owned result, nullable (heap kind) | `cfl_sds_create`, `_create_len`, `_create_size`, `cfl_kvlist_create`, `cfl_array_create`, `cfl_variant_create_from_bool`/`_int64`/`_double`/`_string`, `cfl_utils_split` |
| released, nullable (cascading destruction is a trusted model, C04.2) | `cfl_kvlist_destroy`, `cfl_array_destroy`, `cfl_variant_destroy`, `cfl_utils_split_free` |
| reallocation (`resource_return_reallocated`, as libc `realloc`) | `cfl_sds_cat` |
| slot move (C04.15) | `cfl_sds_printf` |
| borrowed result (C04.3.4) | `cfl_kvlist_fetch`: the variant the list owns |
| return-code disposition, `eq 0` consumed / `ne 0` owned (C04.13) | `cfl_kvlist_insert`, `cfl_array_append`, `_append_array`, `_append_kvlist` |
| borrowed under the literal premise `referenced = 0` (C04.5) | `cfl_kvlist_insert_bytes`, `cfl_array_append_bytes` |
| borrowed, none | the copying inserts and appends, the sds length setters, every mpack writer, reader and tag primitive |

Four decisions need saying here:

- **The mpack error state is stated, not ignored.** The body that sets it
  (`mpack_writer_flag_error`, `mpack_reader_flag_error`) stores the code
  once and clamps a reader's data. It then makes every later operation a
  no-op and calls the object's `error_fn` if one is installed. Nothing the
  caller holds changes custody. The buffer's custody belongs to the
  undeclared `mpack_writer_init_growable`/`mpack_writer_destroy` pair. So
  each primitive is `resource_borrowed` of its object. The module header
  says so, and also says which callbacks a body can reach and that the
  audited code installs none. That boundary joins
  W46-METRIC-TRACE-CALLBACK-PREMISES.
- **`cfl_sds_printf` has no tail.** The loader refuses a variadic tail
  beside a slot move: a slot clause stands only beside plain parameter and
  result predicates. The first probe measured this. A declaration without a
  tail is not an ABI disagreement, and the arity gate refuses every call
  that passes an argument after the format. Of the 33 calls in the audited
  trees, the declaration covers the 10 `ctr_encode_text.c` statements that
  pass none. The result, `*sds` or NULL, has no v1 mode, so it is
  `resource_return_none()`. Every covered call discards it. Every call that
  binds it (Fluent Bit's HTTP client and server, `in_blob`, 16 calls) passes
  arguments and stays outside. Both gaps are CFL-SDS-PRINTF-SLOT-MOVE-FORMS.
- **The bytes inserts state the copying arm only.** With a non-zero
  `referenced`, the variant keeps the caller's pointer. A guard cannot
  test `!= 0` (C04.14.1). The literal premise therefore restricts the
  declaration to calls passing `0`, which every audited call does
  (`CFL_FALSE` is `0` after preprocessing). The retaining arm stays with
  W48-CALLEE-DECISION-TABLE class 1, where the existing kvlist note left
  it.
- **Kinds.** CFL objects are heap objects. `free(cfl_kvlist_create())`
  would discharge, as it does for every heap pair in the corpus
  (`cJSON_New_Item`/`cJSON_Delete`). The module states this.

## 3. Census of every in-scope callee

Columns: pairs and sites in the user's run; class; the declared form, the
reason it stays undeclared (with its follow-up), or the owning ledger entry.
No in-scope callee is in card 1's DECLARED-NAMES-RESOURCE-V1-PROJECTIONS set.
That set holds librdkafka, Fluent Bit, ChunkIO and Monkey names only.

| # | Callee | Pairs | Sites | Class | Form, reason or owner |
|---|---|---|---|---|---|
| 1 | `cfl_list_add` | 43 | 45 | owned elsewhere | W45-SUB-OBJECT-STORE-FORMAL-VALUE (brief: out of scope) |
| 2 | `cfl_list_del` | 24 | 31 | owned elsewhere | W45-SUB-OBJECT-STORE-FORMAL-VALUE (brief: out of scope) |
| 3 | `cfl_sds_create_size` | 18 | 19 | declared | owned(nullable) result |
| 4 | `mpack_start_map` | 17 | 27 | declared | borrowed, none |
| 5 | `cfl_kvlist_destroy` | 14 | 26 | declared | released(nullable) |
| 6 | `mpack_read_tag` | 14 | 14 | undeclared | an `mpack_tag_t` structure result by value; v1 results are scalars, pointers and unit (C04.1) -> `MPACK-STRUCT-RESULT-BY-VALUE` |
| 7 | `mpack_start_array` | 14 | 23 | declared | borrowed, none |
| 8 | `cfl_kvlist_fetch` | 13 | 16 | declared | borrowed_from(list, nullable) result |
| 9 | `mpack_write_nil` | 10 | 22 | declared | borrowed, none |
| 10 | `cfl_sds_create` | 8 | 13 | declared | owned(nullable) result |
| 11 | `cfl_kvlist_create` | 5 | 7 | declared | owned(nullable) result |
| 12 | `cfl_sds_create_len` | 5 | 7 | declared | owned(nullable) result |
| 13 | `cfl_sds_len_set` | 5 | 11 | declared | borrowed, none |
| 14 | `cfl_sds_printf` | 5 | 14 | declared | slot_moved(sds, nullable); none; no tail (refused beside a slot move) |
| 15 | `mpack_write_u32` | 5 | 10 | declared | borrowed, none |
| 16 | `mpack_write_u64` | 5 | 6 | declared | borrowed, none |
| 17 | `cfl_kvlist_insert_uint64` | 4 | 8 | declared | borrowed, none |
| 18 | `cfl_sds_set_len` | 4 | 4 | declared | borrowed, none |
| 19 | `mpack_peek_tag` | 4 | 4 | undeclared | an `mpack_tag_t` structure result by value; v1 results are scalars, pointers and unit (C04.1) -> `MPACK-STRUCT-RESULT-BY-VALUE` |
| 20 | `mpack_tag_array_count` | 4 | 4 | declared | borrowed, none |
| 21 | `mpack_tag_uint_value` | 4 | 4 | declared | borrowed, none |
| 22 | `cfl_array_destroy` | 3 | 13 | declared | released(nullable) |
| 23 | `cfl_variant_destroy` | 3 | 4 | declared | released(nullable) |
| 24 | `mpack_tag_int_value` | 3 | 3 | declared | borrowed, none |
| 25 | `mpack_write_int` | 3 | 8 | declared | borrowed, none |
| 26 | `mpack_writer_init_growable` | 3 | 3 | undeclared | stores the caller's `char **` slot in the writer and writes an owned buffer through it at `mpack_writer_destroy` on success only: a depth-two retention and a deferred, status-conditioned out slot -> `MPACK-GROWABLE-WRITER-DEFERRED-OUT-SLOT` |
| 27 | `cfl_array_create` | 2 | 2 | declared | owned(nullable) result |
| 28 | `cfl_list_init` | 2 | 3 | owned elsewhere | W45-SUB-OBJECT-STORE-FORMAL-VALUE (brief: out of scope) |
| 29 | `cfl_utils_split` | 2 | 2 | declared | owned(nullable) result |
| 30 | `cfl_utils_split_free` | 2 | 5 | declared | released(nullable) |
| 31 | `cfl_variant_kvpair_to_otlp_kvpair` | 2 | 2 | not CFL | static helper of cmt_encode_opentelemetry.c with its body in the run |
| 32 | `mpack_done_array` | 2 | 2 | declared | borrowed, none |
| 33 | `mpack_done_map` | 2 | 2 | declared | borrowed, none |
| 34 | `mpack_done_str` | 2 | 2 | declared | borrowed, none |
| 35 | `mpack_reader_destroy` | 2 | 2 | declared | borrowed, none |
| 36 | `mpack_reader_remaining` | 2 | 2 | undeclared | writes the same referenced-buffer pointer through `data` when non-NULL; both calls pass `NULL`, a cast no literal premise selects (C04.5.1) -> `MPACK-READER-REFERENCED-BUFFER-RESULT` |
| 37 | `mpack_tag_double_value` | 2 | 2 | declared | borrowed, none |
| 38 | `mpack_tag_map_count` | 2 | 2 | declared | borrowed, none |
| 39 | `mpack_tag_str_length` | 2 | 2 | declared | borrowed, none |
| 40 | `mpack_write_uint` | 2 | 14 | declared | borrowed, none |
| 41 | `cfl_arena_create_with_options` | 1 | 1 | owned elsewhere | W47-ARENA-POOL-RESULT-MODE (the arena pool kind; card 1 class B) |
| 42 | `cfl_arena_destroy` | 1 | 1 | owned elsewhere | W47-ARENA-POOL-RESULT-MODE (the arena pool kind; card 1 class B) |
| 43 | `cfl_arena_options_init` | 1 | 1 | owned elsewhere | W47-ARENA-POOL-RESULT-MODE / W48-CALLEE-DECISION-TABLE class 4 (the arena constructor pair) |
| 44 | `cfl_arena_strndup` | 1 | 1 | owned elsewhere | W47-ARENA-POOL-RESULT-MODE (an arena result; the interior reading is the user's interim decision for cfl_arena_calloc only) |
| 45 | `cfl_array_append` | 1 | 1 | declared | eq 0 consumed / ne 0 owned |
| 46 | `cfl_array_append_array` | 1 | 1 | declared | eq 0 consumed / ne 0 owned |
| 47 | `cfl_array_append_bool` | 1 | 1 | declared | borrowed, none |
| 48 | `cfl_array_append_bytes` | 1 | 1 | declared | borrowed, none; premise referenced = 0 |
| 49 | `cfl_array_append_double` | 1 | 1 | declared | borrowed, none |
| 50 | `cfl_array_append_int64` | 1 | 1 | declared | borrowed, none |
| 51 | `cfl_array_append_kvlist` | 1 | 1 | declared | eq 0 consumed / ne 0 owned |
| 52 | `cfl_array_append_string` | 1 | 1 | declared | borrowed, none |
| 53 | `cfl_kvlist_insert` | 1 | 1 | declared | eq 0 consumed / ne 0 owned |
| 54 | `cfl_kvlist_insert_bytes` | 1 | 2 | declared | borrowed, none; premise referenced = 0 |
| 55 | `cfl_sds_cat` | 1 | 2 | declared | owned s; reallocated(s, nullable) result |
| 56 | `cfl_variant_array_to_otlp_any_value` | 1 | 1 | not CFL | static helper of cmt_encode_opentelemetry.c with its body in the run |
| 57 | `cfl_variant_binary_to_otlp_any_value` | 1 | 1 | not CFL | static helper of cmt_encode_opentelemetry.c with its body in the run |
| 58 | `cfl_variant_boolean_to_otlp_any_value` | 1 | 1 | not CFL | static helper of cmt_encode_opentelemetry.c with its body in the run |
| 59 | `cfl_variant_create_from_array` | 1 | 1 | undeclared | custody of `value` moves into the result exactly when it is non-NULL; v1 has no result-nullness disposition (C04.13) -> `CFL-RESULT-NULLNESS-HANDOFF` |
| 60 | `cfl_variant_create_from_bool` | 1 | 1 | declared | owned(nullable) result |
| 61 | `cfl_variant_create_from_double` | 1 | 1 | declared | owned(nullable) result |
| 62 | `cfl_variant_create_from_int64` | 1 | 1 | declared | owned(nullable) result |
| 63 | `cfl_variant_create_from_kvlist` | 1 | 1 | undeclared | custody of `value` moves into the result exactly when it is non-NULL; v1 has no result-nullness disposition (C04.13) -> `CFL-RESULT-NULLNESS-HANDOFF` |
| 64 | `cfl_variant_create_from_string` | 1 | 2 | declared | owned(nullable) result |
| 65 | `cfl_variant_double_to_otlp_any_value` | 1 | 1 | not CFL | static helper of cmt_encode_opentelemetry.c with its body in the run |
| 66 | `cfl_variant_int64_to_otlp_any_value` | 1 | 1 | not CFL | static helper of cmt_encode_opentelemetry.c with its body in the run |
| 67 | `cfl_variant_kvlist_to_otlp_any_value` | 1 | 1 | not CFL | static helper of cmt_encode_opentelemetry.c with its body in the run |
| 68 | `cfl_variant_string_to_otlp_any_value` | 1 | 2 | not CFL | static helper of cmt_encode_opentelemetry.c with its body in the run |
| 69 | `cfl_variant_to_otlp_any_value` | 1 | 1 | not CFL | static helper of cmt_encode_opentelemetry.c with its body in the run |
| 70 | `mpack_done_bin` | 1 | 1 | declared | borrowed, none |
| 71 | `mpack_expect_nil` | 1 | 1 | declared | borrowed, none |
| 72 | `mpack_read_bytes` | 1 | 1 | declared | borrowed, none |
| 73 | `mpack_read_bytes_inplace` | 1 | 1 | undeclared | returns a pointer into the input buffer the reader references (not owns); no derived mode ties a pointer to storage a holder references -> `MPACK-READER-REFERENCED-BUFFER-RESULT` |
| 74 | `mpack_read_cstr` | 1 | 1 | declared | borrowed, none |
| 75 | `mpack_tag_bin_length` | 1 | 1 | declared | borrowed, none |
| 76 | `mpack_tag_type` | 1 | 1 | declared | borrowed, none |
| 77 | `mpack_write_i32` | 1 | 1 | declared | borrowed, none |
| 78 | `mpack_writer_flag_error` | 1 | 2 | declared | borrowed, none |

| Class | Callees | Pairs |
|---|---|---|
| declared | 55 | 197 |
| not CFL | 9 | 10 |
| owned elsewhere | 7 | 73 |
| undeclared | 7 | 26 |

## 4. Prediction

[PREDICTION.md](PREDICTION.md) was committed (`19c39ce56`) before any
contract changed. Its `measure-prediction` block predicts:

- **corpus:** +7 Safe. These are `pack_summary_metric`,
  `ctr_attributes_create` and the five `convert_*_value` helpers of
  `ctr_decode_opentelemetry.c`.
- **system and slice:** no move.

On the user's `current` run it predicts:

- 55 callees and 197 pairs leave the uncontracted list.
- Eight functions leave the 508 "only uncontracted" set: the seven above
  plus `pack_uint64`.
- No unit changes status.

One correction is known after the prediction. `cfl_sds_printf` lost its
tail (§2), so its 6 argument-carrying call sites
(`ctr_encode_text.c:409`, `:411`, `:413`, `:429`, `:449`, `:486`) are now
arity refusals. They may still be counted under its name. The five
functions that call it have other blockers, so no predicted move depends on
it.

## 5. Records

- **Registered:**
  - CFL-SDS-PRINTF-SLOT-MOVE-FORMS
  - CFL-RESULT-NULLNESS-HANDOFF (`cfl_variant_create_from_array`/`_kvlist`)
  - MPACK-STRUCT-RESULT-BY-VALUE (`mpack_read_tag`, `mpack_peek_tag`)
  - MPACK-GROWABLE-WRITER-DEFERRED-OUT-SLOT (`mpack_writer_init_growable`)
  - MPACK-READER-REFERENCED-BUFFER-RESULT (`mpack_read_bytes_inplace`,
    `mpack_reader_remaining`)
- **Amended:**
  - BORROW-CREATED-INSERTED-CHILD: `cfl_kvlist_fetch` is declared, the
    first half of its discharge.
  - W44-CALLEE-EXTERNAL-INLINE-DEFINITION: the second discharge arm, for
    the `MPACK_INLINE` names declared here.
  - W44-CALLEE-NO-CORPUS-BODY: the CFL and mpack names declared.
  - W46-METRIC-TRACE-CALLBACK-PREMISES: the mpack declarations' callback
    boundary.
  - W48-CALLEE-DECISION-TABLE: the bytes inserts' copying arm is declared;
    the retaining arm stays.
- **Unchanged:** W45-SUB-OBJECT-STORE-FORMAL-VALUE (the list callees) and
  W47-ARENA-POOL-RESULT-MODE (the arena callees).
- **This card's id:** registered and retired.

## 6. Gate

GATE_SECTION
