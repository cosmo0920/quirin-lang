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
