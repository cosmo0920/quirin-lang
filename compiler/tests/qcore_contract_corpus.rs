use std::fs;

use qcore_compiler::{
    CompileOptions, SourceContractClauseKind, SourceContractExitSelector, SourceDeclarationKind,
    SourceFile, compile_files, format_diagnostic, parse_source_program,
};

const EXPECTED_CONTRACTS: &[(&str, &str, usize)] = &[
    ("cfl/arena_lifetime.qv", "cfl_arena_create_with_options", 5),
    ("cfl/arena_lifetime.qv", "cfl_arena_destroy", 5),
    ("cfl/intrusive_list.qv", "cfl_list_add", 4),
    ("cfl/intrusive_list.qv", "cfl_list_del", 5),
    // Card NULL-TRIAGE-UNRESOLVED-PATHS: `cfl_list_size`'s integer result
    // range (bounds contract v1, the crate's new bounds view, read only by
    // the null property): selector and one range. Read from
    // `lib/cfl/include/cfl/cfl_list.h:200-212` at both revisions.
    ("cfl/null_deref.qv", "cfl_list_size", 2),
    // Card W36-CORPUS-CONTRACTS-3: the CFL dynamic string's release, the
    // highest function-cover callee in the baseline (8 projected functions name
    // it and nothing else). Read from `lib/cfl/src/cfl_sds.c:249-275`;
    // `nullable` because the body's first statement is `if (!s) { return; }`.
    ("cfl/sds_lifetime.qv", "cfl_sds_destroy", 3),
    // Card W46-EXTERNAL-CALLEE-CONTRACTS: the length query
    // (`lib/cfl/src/cfl_sds.c:193-200`): marker, borrowed string, `none`.
    ("cfl/sds_lifetime.qv", "cfl_sds_len", 3),
    // Card W47-CALLEE-CLOSURE-R2: the zeroed arena allocation
    // (`lib/cfl/src/cfl_arena.c:354-371`): marker, borrowed arena, count and
    // size, and a nullable result interior to the arena.
    ("cfl/sds_lifetime.qv", "cfl_arena_calloc", 5),
    // Card W45-CALLEE-CASCADE-RESIDUAL: the kvlist count and the four copying
    // inserts (`lib/cfl/src/cfl_kvlist.c:561-705`): marker, borrowed formals,
    // `none`.
    ("cfl/sds_lifetime.qv", "cfl_kvlist_count", 3),
    ("cfl/sds_lifetime.qv", "cfl_kvlist_insert_string", 5),
    ("cfl/sds_lifetime.qv", "cfl_kvlist_insert_bool", 5),
    ("cfl/sds_lifetime.qv", "cfl_kvlist_insert_int64", 5),
    ("cfl/sds_lifetime.qv", "cfl_kvlist_insert_double", 5),
    // Card CALLEE-RETENTION-CLASS1, stage (a): the custody-taking inserts,
    // marker, two borrowed formals, two return-code classes, `none`.
    ("cfl/sds_lifetime.qv", "cfl_kvlist_insert_array", 6),
    ("cfl/sds_lifetime.qv", "cfl_kvlist_insert_kvlist", 6),
    // Card SLOT-MOVE-AND-INTERIOR-BORROW (class 2, rule 12x): the slot move,
    // read from `lib/cfl/src/cfl_sds.c:376-389`.
    ("cfl/sds_lifetime.qv", "cfl_sds_cat_safe", 6),
    // Card W45-CALLEE-CASCADE-RESIDUAL round 3: the list counter: marker, borrowed, `none`.
    ("cfl/sds_lifetime.qv", "cfl_list_size", 3),
    // Card W37-CORPUS-CONTRACTS-4: the CFL wall-clock reading, sole blocker
    // of `ctr_span_end`. Zero parameters and a scalar `uint64_t` result, so
    // the contract is `requires` + `return_none` and nothing else — the
    // `flb_time_now` shape, whose C header is unprototyped `()` in exactly
    // the same way, which is what settles the arity gate by precedent.
    // Read from `lib/cfl/src/cfl_time.c:31-60`.
    ("cfl/time.qv", "cfl_time_now", 2),
    ("chunkio/chunk_destructor.qv", "cio_chunk_close", 10),
    ("chunkio/chunk_destructor.qv", "cio_chunk_delete", 5),
    // Card NEIGHBOURS: the corpus moves 64 -> 67 modules with this file,
    // `fluent_bit/input_chunk_helpers.qv` and `fluent_bit/time.qv`, and
    // the resource-v1 declaration count 25 -> 31. Deliberate; see
    // `docs/reports/remaining-neighbours/README.md`. All three residency
    // helpers borrow the chunk and return no custody, which is the only v1
    // mode combination with the caller-side effect each pinned body was read
    // to have: neither `cio_chunk_up_force` nor `cio_chunk_down` frees the
    // chunk or its backend, so the caller's obligation to `cio_chunk_close` is
    // unchanged across both.
    ("chunkio/chunk_residency.qv", "cio_chunk_is_up", 3),
    ("chunkio/chunk_residency.qv", "cio_chunk_up_force", 3),
    ("chunkio/chunk_residency.qv", "cio_chunk_down", 3),
    // Card W45-CALLEE-CASCADE-RESIDUAL: `cio_chunk_up_force`'s body
    // (`lib/chunkio/src/cio_file.c:816-883`): marker, borrowed chunk and flag,
    // `none`.
    ("chunkio/chunk_residency.qv", "_cio_file_up", 4),
    // Card CG10-INDIRECT-ACCESS-SOLE-RESIDUALS round 2: the neutral
    // `cio_errno_print` (three borrowed formals, no result custody).
    ("chunkio/chunk_residency.qv", "cio_errno_print", 5),
    // Card W31-CORPUS-CONTRACTS-2: the corpus moves 74 -> 76 modules with this
    // file and `fluent_bit/input_chunk_total_size.qv`, and the resource-v1
    // declaration count 41 -> 43. Deliberate; see
    // `docs/reports/w31-corpus-contracts-2/README.md`. The body
    // (`lib/chunkio/src/cio_chunk.c:315-334` at the pinned checkout) is a pure
    // size query — an error-field reset and backend length-field reads — so
    // `resource_borrowed(ch)` and `resource_return_none()` are the only
    // faithful modes, the same reading `fluent_bit/input_chunk_sizes.qv`
    // recorded for its wrapper. This lands the ChunkIO-side module card
    // W29-CORPUS-CONTRACTS' `W29-CONTENT-SIZE-UNDECLARED` asked for.
    ("chunkio/chunk_sizes.qv", "cio_chunk_get_content_size", 3),
    // Card W45-CALLEE-CASCADE-RESIDUAL round 3: two size readers whose bodies are Safe in
    // their home units: marker, borrowed formals, `none`.
    ("chunkio/chunk_sizes.qv", "cio_file_native_get_size", 4),
    ("chunkio/chunk_sizes.qv", "cio_stream_size_chunks_up", 3),
    (
        "chunkio/constructor_failure_observation.qv",
        "native_constructor_failure_observation",
        11,
    ),
    ("chunkio/context_destructor.qv", "cio_destroy", 7),
    ("chunkio/counters.qv", "cio_chunk_counter_total_add", 10),
    ("chunkio/counters.qv", "cio_chunk_counter_total_sub", 10),
    ("chunkio/counters.qv", "cio_chunk_counter_total_up_add", 10),
    ("chunkio/counters.qv", "cio_chunk_counter_total_up_sub", 10),
    ("chunkio/error_frame.qv", "cio_error_reset", 5),
    ("chunkio/error_state.qv", "cio_error_reset", 5),
    ("chunkio/file_close.qv", "file_close_no_delete", 17),
    ("chunkio/file_close_delete.qv", "file_close_delete", 13),
    // Card CG16-BOUNDS-TRANSPORT-AND-CHUNKIO-NEIGHBOURS: the neighbouring
    // `cio_file_st.h` accessors (bounds contract v1, the crate's bounds view):
    // `get_hash` selector, read frame and derived relation; `infer_content_len`
    // and `set_content_len` selector, quant check and frame.
    ("chunkio/file_st_bounds.qv", "cio_file_st_get_hash", 3),
    (
        "chunkio/file_st_bounds.qv",
        "cio_file_st_infer_content_len",
        3,
    ),
    (
        "chunkio/file_st_bounds.qv",
        "cio_file_st_set_content_len",
        3,
    ),
    ("chunkio/filesystem_attempts.qv", "file_close_result_use", 6),
    ("chunkio/filesystem_attempts.qv", "file_unmap_result_use", 3),
    (
        "chunkio/filesystem_attempts.qv",
        "native_close_result_use",
        3,
    ),
    (
        "chunkio/filesystem_attempts.qv",
        "native_unmap_result_use",
        3,
    ),
    (
        "chunkio/filesystem_chunk_close.qv",
        "filesystem_chunk_close",
        22,
    ),
    (
        "chunkio/filesystem_constructor_failure_observation.qv",
        "native_file_constructor_failure_observation",
        16,
    ),
    // Deliberate pin update (card P04-B1): the cleanup-outcome policies now
    // declare the referents their `CleanupProfile` binds, so a transported
    // profile has a machine-checkable contract-side inventory instead of none.
    // 12 -> 25: thirteen `cleanup_` premises — the `cleanup_vocabulary_v1`
    // selector, the pinned function, the record pointer type, the `unlink`
    // callee and its signature, the eligibility and mapping fields, three
    // indexed field entries carrying C type and reset value, one indexed path
    // argument, and the two diagnostic helpers. No `ensures` clause moved.
    ("chunkio/filesystem_delete.qv", "native_delete_outcomes", 25),
    (
        "chunkio/filesystem_delete_attempt.qv",
        "native_delete_result_use",
        3,
    ),
    (
        "chunkio/filesystem_delete_observation.qv",
        "native_delete_observation",
        12,
    ),
    (
        "chunkio/filesystem_descriptor_failure_observation.qv",
        "native_file_descriptor_failure_observation",
        20,
    ),
    (
        "chunkio/filesystem_existing_observation.qv",
        "native_file_existing_observation",
        17,
    ),
    (
        "chunkio/filesystem_integrity_observation.qv",
        "native_file_integrity_observation",
        21,
    ),
    (
        "chunkio/filesystem_observation.qv",
        "native_cleanup_observation",
        14,
    ),
    (
        "chunkio/filesystem_os_constructor_observation.qv",
        "native_file_os_constructor_observation",
        17,
    ),
    (
        "chunkio/filesystem_os_observation.qv",
        "native_os_cleanup_observation",
        13,
    ),
    // Deliberate pin updates (card P04-B1), same declaration family as
    // `filesystem_delete.qcore` above and no `ensures` clause moved. Close
    // 16 -> 26: ten `cleanup_` premises over a one-field inventory with one
    // argument. Unmap 16 -> 29: thirteen, over three fields and two ordered
    // `munmap` arguments. Neither declares a mapping field: that clause exists
    // only for the guarded-delete policy, and declaring one here is refused.
    (
        "chunkio/filesystem_outcomes.qv",
        "native_close_outcomes",
        26,
    ),
    (
        "chunkio/filesystem_outcomes.qv",
        "native_unmap_outcomes",
        29,
    ),
    (
        "chunkio/filesystem_reopen_failure_observation.qv",
        "native_file_reopen_failure_observation",
        28,
    ),
    (
        "chunkio/filesystem_sync_failure_observation.qv",
        "native_file_sync_failure_observation",
        18,
    ),
    (
        "chunkio/filesystem_teardown_observation.qv",
        "native_file_teardown_observation",
        16,
    ),
    (
        "chunkio/filesystem_transition_observation.qv",
        "native_file_transition_observation",
        16,
    ),
    (
        "chunkio/filesystem_unmap_failure_observation.qv",
        "native_file_unmap_failure_observation",
        19,
    ),
    (
        "chunkio/import_composition.qv",
        "check_imported_chunk_program",
        7,
    ),
    ("chunkio/list_unlink.qv", "__mk_list_del", 6),
    ("chunkio/list_unlink.qv", "mk_list_del", 12),
    ("chunkio/lock_state.qv", "cio_chunk_is_locked", 7),
    ("chunkio/lock_state.qv", "cio_chunk_unlock", 8),
    ("chunkio/memory_backend.qv", "cio_memfs_close", 22),
    ("chunkio/memory_close.qv", "cio_chunk_close", 19),
    // Card CHUNKIO-META-LEN-BOUNDS-CONTRACTS: the corpus's first bounds
    // contract v1 module (the crate's bounds view, never a resource view):
    // the header accessors of `lib/chunkio/include/chunkio/cio_file_st.h:76-86`,
    // selector, quant check `22 <= #i && #i < 24` and a closed frame each.
    ("chunkio/meta_len_bounds.qv", "cio_file_st_get_meta_len", 3),
    ("chunkio/meta_len_bounds.qv", "cio_file_st_set_meta_len", 3),
    ("chunkio/nested_cleanup.qv", "nested_cleanup", 12),
    (
        "chunkio/nested_memory_teardown.qv",
        "nested_memory_teardown",
        10,
    ),
    ("chunkio/parent_teardown.qv", "parent_teardown", 11),
    ("chunkio/resource_open_close.qv", "cio_chunk_open", 8),
    ("chunkio/resource_open_close.qv", "cio_chunk_close", 4),
    ("chunkio/stream_destructor.qv", "cio_stream_destroy", 6),
    ("chunkio/stream_destructor.qv", "cio_stream_delete", 5),
    (
        "chunkio/teardown_observation.qv",
        "native_teardown_observation",
        11,
    ),
    ("chunkio/teardown_traversal.qv", "teardown_traversal", 11),
    ("chunkio/unmap_wrapper.qv", "unmap_file_outcomes", 17),
    (
        "chunkio/unsupported_close.qv",
        "unsupported_chunk_close",
        12,
    ),
    (
        "chunkio/unsupported_constructor_observation.qv",
        "native_unsupported_constructor",
        11,
    ),
    // Card W36-CORPUS-CONTRACTS-3: the CMetrics context's release, read from
    // `lib/cmetrics/src/cmetrics.c:92-146`. `nonnull` — measured, not
    // defaulted: the body has no leading null guard and its first executable
    // statement reads `cmt->counters`.
    ("cmetrics/context_lifetime.qv", "cmt_destroy", 3),
    // Card W45-CALLEE-CASCADE-RESIDUAL round 3: no parameter: marker, `none`.
    ("cmetrics/context_lifetime.qv", "cmt_atomic_initialize", 2),
    ("cmetrics/counter_reset.qv", "cmt_counter_allow_reset", 4),
    // Card W37-CORPUS-CONTRACTS-4: two CMetrics metric-value accessors, each
    // the sole blocker of one projected function (`check_staled_timestamp` and
    // `cmt_metric_set`). Both are custody-NEUTRAL — the metric is read through
    // or written through, never released — and both are defined in
    // `cmt_metric.c`, an ANALYSED unit, so constraint 8's duty gate applies;
    // the declarations are the neutral element, which a faithful summary of a
    // body that releases nothing cannot contradict. Read from
    // `lib/cmetrics/src/cmt_metric.c:244-251` and `:71-81`.
    ("cmetrics/metric_values.qv", "cmt_metric_get_timestamp", 3),
    ("cmetrics/metric_values.qv", "cmt_metric_set_double", 5),
    // W46-METRIC-TRACE-CONTRACT-GAPS: scalar-only reads, writes, and
    // bit-pattern conversions; every argument remains borrowed.
    ("cmetrics/metric_values.qv", "cmt_metric_get_value", 3),
    ("cmetrics/metric_values.qv", "cmt_metric_get_value_type", 3),
    ("cmetrics/metric_values.qv", "cmt_metric_get_int64_value", 3),
    (
        "cmetrics/metric_values.qv",
        "cmt_metric_get_uint64_value",
        3,
    ),
    ("cmetrics/metric_values.qv", "cmt_metric_set_int64", 5),
    ("cmetrics/metric_values.qv", "cmt_metric_set_uint64", 5),
    ("cmetrics/metric_values.qv", "cmt_metric_set_timestamp", 4),
    (
        "cmetrics/metric_values.qv",
        "cmt_metric_set_start_timestamp",
        4,
    ),
    (
        "cmetrics/metric_values.qv",
        "cmt_metric_unset_start_timestamp",
        3,
    ),
    (
        "cmetrics/metric_values.qv",
        "cmt_metric_has_start_timestamp",
        3,
    ),
    (
        "cmetrics/metric_values.qv",
        "cmt_metric_get_start_timestamp",
        3,
    ),
    ("cmetrics/metric_values.qv", "cmt_math_d64_to_uint64", 3),
    ("cmetrics/metric_values.qv", "cmt_math_uint64_to_d64", 3),
    // W46-METRIC-AGGREGATE-CONTRACTS: six scalar aggregate reads.
    ("cmetrics/metric_values.qv", "cmt_metric_hist_get_value", 4),
    (
        "cmetrics/metric_values.qv",
        "cmt_metric_hist_get_count_value",
        3,
    ),
    (
        "cmetrics/metric_values.qv",
        "cmt_metric_hist_get_sum_value",
        3,
    ),
    (
        "cmetrics/metric_values.qv",
        "cmt_summary_quantile_get_value",
        4,
    ),
    ("cmetrics/metric_values.qv", "cmt_summary_get_sum_value", 3),
    (
        "cmetrics/metric_values.qv",
        "cmt_summary_get_count_value",
        3,
    ),
    // Card CG10-INDIRECT-ACCESS-SOLE-RESIDUALS: the three `cmt_atomic` value
    // operations (every backend copy read, they agree): marker, borrowed
    // formals, `none`.
    ("cmetrics/metric_values.qv", "cmt_atomic_load", 3),
    ("cmetrics/metric_values.qv", "cmt_atomic_store", 4),
    (
        "cmetrics/metric_values.qv",
        "cmt_atomic_compare_exchange",
        5,
    ),
    // Card W37-CORPUS-CONTRACTS-4: the unsigned msgpack tag reader, sole
    // blocker of `unpack_meta_ver`. `output_buffer` is an OUT parameter, but
    // what is written through it is a SCALAR, not a resource identity, so it
    // is `borrowed` and not a custody `Out` — declaring `Out` would assert the
    // caller acquires an obligation it does not acquire. Both pointer formals
    // are NULL-guarded before any dereference, so the body has real failure
    // paths a borrowed declaration cannot delete (constraint 4). Read from
    // `lib/cmetrics/src/cmt_mpack_utils.c:88-113`.
    ("cmetrics/mpack_utils.qv", "cmt_mpack_consume_uint_tag", 4),
    // CG22-CALLBACK-REGISTRY-CONTRACT: the array and map unpackers.
    ("cmetrics/mpack_utils.qv", "cmt_mpack_unpack_array", 6),
    ("cmetrics/mpack_utils.qv", "cmt_mpack_unpack_map", 6),
    // Card NULL-SAFETY-PROPERTY: `quant null-deref` checks (bounds
    // contract v1, the crate's bounds view): selector and one check each.
    ("cmetrics/null_deref.qv", "cmt_destroy", 2),
    // Card W36-CORPUS-CONTRACTS-3: `cmt_destroy`'s exact sibling one library
    // over, read from `lib/ctraces/src/ctraces.c:58-71`, `nonnull` on the same
    // ground. This is the corpus's first `ctraces` contract module.
    ("ctraces/context_lifetime.qv", "ctr_destroy", 3),
    ("ctraces/context_lifetime.qv", "ctr_id_cmp", 4),
    ("ctraces/context_lifetime.qv", "ctr_id_get_len", 3),
    ("ctraces/context_lifetime.qv", "ctr_span_set_flags", 4),
    (
        "ctraces/context_lifetime.qv",
        "ctr_span_set_dropped_events_count",
        4,
    ),
    (
        "ctraces/context_lifetime.qv",
        "ctr_span_set_dropped_links_count",
        4,
    ),
    (
        "ctraces/context_lifetime.qv",
        "ctr_span_set_dropped_attributes_count",
        4,
    ),
    // Card W45-CALLEE-CASCADE-RESIDUAL: two msgpack tag readers and four
    // copying attribute setters (`ctr_mpack_utils.c`, `ctr_attributes.c:54-76`):
    // marker, borrowed formals, `none`.
    (
        "ctraces/context_lifetime.qv",
        "ctr_mpack_consume_uint_tag",
        4,
    ),
    (
        "ctraces/context_lifetime.qv",
        "ctr_mpack_consume_int_tag",
        4,
    ),
    (
        "ctraces/context_lifetime.qv",
        "ctr_attributes_set_string",
        5,
    ),
    ("ctraces/context_lifetime.qv", "ctr_attributes_set_bool", 5),
    ("ctraces/context_lifetime.qv", "ctr_attributes_set_int64", 5),
    (
        "ctraces/context_lifetime.qv",
        "ctr_attributes_set_double",
        5,
    ),
    // Card CALLEE-RETENTION-CLASS1, stage (a): the forwarding setters,
    // marker, two borrowed formals, two return-code classes, `none`.
    ("ctraces/context_lifetime.qv", "ctr_attributes_set_array", 6),
    (
        "ctraces/context_lifetime.qv",
        "ctr_attributes_set_kvlist",
        6,
    ),
    // Card SLOT-MOVE-AND-INTERIOR-BORROW (class 2, rule 12x): the static
    // forwarder of `cfl_sds_cat_safe` in `ctr_encode_text.c:22-28`.
    ("ctraces/context_lifetime.qv", "sds_cat_safe", 5),
    // Card CG10-INDIRECT-ACCESS-SOLE-RESIDUALS round 2: the neutral
    // `ctr_errno_print`.
    ("ctraces/context_lifetime.qv", "ctr_errno_print", 5),
    // CG22-CALLBACK-REGISTRY-CONTRACT: the array and map unpackers.
    ("ctraces/context_lifetime.qv", "ctr_mpack_unpack_array", 6),
    ("ctraces/context_lifetime.qv", "ctr_mpack_unpack_map", 6),
    (
        "fluent_bit/busy_state.qv",
        "flb_input_chunk_release_lock",
        7,
    ),
    // Card HASH-CONTRACTS: the corpus moves 60 -> 61 modules with this file,
    // and the resource-v1 declaration count 19 -> 20. Deliberate.
    //
    // Card HASH-CLAUSE: 7 -> 8 clauses. The size-0 boundary moves off a
    // machine-parsed comment directive onto the §C04.5 clause
    // `requires resource_literal_argument(val_size, 0);`, which the loader
    // validates total-or-reject. Deliberate; the declaration's modes and the
    // premise's refusal set are unchanged and measured so
    // (`docs/reports/remaining-hash-clause/README.md`).
    (
        "fluent_bit/hash_table_publication.qv",
        "flb_hash_table_add",
        8,
    ),
    // Card W45-CALLEE-CASCADE-RESIDUAL round 3: the entry removal: marker, two borrowed,
    // `none`.
    (
        "fluent_bit/hash_table_publication.qv",
        "flb_hash_table_del",
        4,
    ),
    // Card ADDRESS-OF-MEMBER-DESTROY-UNDECLARED: the corpus moves 71 -> 72
    // modules with this file, and the resource-v1 declaration count 37 -> 38.
    // Deliberate; see `docs/reports/wave22-aom-destroy/README.md`. The body has
    // exactly one exit and `flb_free(ic)` is unconditional on it, so
    // `resource_released` is the only v1 parameter mode with the caller-side
    // effect the pinned body was read to have; `nonnull` because the first
    // statement dereferences `ic`. Like the two NEIGHBOURS helpers below, the
    // callee is defined inside `src/flb_input_chunk.c`, itself a corpus unit,
    // so the declared contract wins over any W4-inferred summary and a
    // disagreement is a `SummaryConflict` diagnostic, never a silent override.
    (
        "fluent_bit/input_chunk_destroy.qv",
        "flb_input_chunk_destroy",
        4,
    ),
    // Card AOM-DESTROY-SIBLINGS: the corpus moves 72 -> 73 modules with this
    // file, and the resource-v1 declaration count 38 -> 39. Deliberate; see
    // `docs/reports/aom-destroy-siblings/README.md`. It decides one half of the
    // previous card's `AOM-DESTROY-SIBLINGS-UNDECLARED`, and it decides it
    // DIFFERENTLY from its neighbour above: this body's third statement is
    // `if (!routes) { return; }`, so the release is `nullable`, not the
    // `nonnull` the sibling carries. The other half,
    // `flb_input_chunk_destroy_corrupted`, was read against its body too and
    // deliberately NOT landed — measured payoff zero, because its single
    // in-corpus call site is refused as an indirect memory access before the
    // call rule is reached — which is the disposition card
    // CONSTRUCTOR-RESIDUALS-ABI-TYPEDEF gave `msgpack/packer.qv`.
    (
        "fluent_bit/input_chunk_destroy_direct_routes.qv",
        "flb_input_chunk_destroy_direct_routes",
        4,
    ),
    // Card NEIGHBOURS, continued. Both helpers are file-`static` definitions
    // inside `src/flb_input_chunk.c`, itself a corpus unit: a declared contract
    // always wins over a W4-inferred summary, and a disagreement is reported as
    // a `SummaryConflict` diagnostic rather than silently resolved.
    (
        "fluent_bit/input_chunk_helpers.qv",
        "generate_chunk_name",
        5,
    ),
    (
        "fluent_bit/input_chunk_helpers.qv",
        "input_chunk_write_header",
        6,
    ),
    // Card CALLEE-RETENTION-CLASS1, stage (b): marker, eight borrowed
    // formals, the retention of `ins` by the `global` holder, `none`.
    (
        "fluent_bit/input_chunk_helpers.qv",
        "append_to_ring_buffer",
        11,
    ),
    (
        "fluent_bit/input_chunk_is_up.qv",
        "flb_input_chunk_is_up",
        4,
    ),
    // W46-FLUENT-ROUTING-QUERY-CONTRACTS: five source-pinned, custody-neutral
    // scalar queries. The existing routes-mask size declaration is retained.
    (
        "fluent_bit/input_chunk_is_up.qv",
        "input_chunk_has_magic_bytes",
        5,
    ),
    (
        "fluent_bit/input_chunk_is_up.qv",
        "flb_input_chunk_is_mem_overlimit",
        4,
    ),
    (
        "fluent_bit/input_chunk_is_up.qv",
        "flb_input_chunk_is_storage_overlimit",
        4,
    ),
    // Card PROD-CONTRACTS: the corpus moves 61 -> 64 modules with this file,
    // `fluent_bit/logging.qv` and `fluent_bit/routes_mask.qv`, and the
    // resource-v1 declaration count 20 -> 25. Deliberate; see
    // `docs/reports/remaining-prod-contracts/README.md`.
    // Card VERDICT-CONTRACT-PREMISE-ANNOTATIONS: `requires resource_premise(untracked_handback, ...)`
    // (contract spec C04.18), one clause: 3 -> 4.
    (
        "fluent_bit/input_chunk_name.qv",
        "flb_input_chunk_get_name",
        4,
    ),
    // Card W29-CORPUS-CONTRACTS: the corpus moves 73 -> 74 modules with this
    // file, and the resource-v1 declaration count 39 -> 41. Deliberate; see
    // `docs/reports/w29-corpus-contracts/README.md`. Both bodies are pure size
    // queries read against the pinned checkout (`src/flb_input_chunk.c:756-759`
    // and `:767-790`): fields read, read-only ChunkIO queries called, a scalar
    // returned, so `resource_borrowed(ic)` and `resource_return_none()` are
    // the only faithful modes — the same reading as `flb_input_chunk_get_name`
    // above. `ssize_t` spells `isize` under the LP64 declared-ABI projection.
    // Both callees are defined inside `src/flb_input_chunk.c`, itself a corpus
    // unit, so the declared contract wins over any W4-inferred summary and a
    // disagreement is a `SummaryConflict` diagnostic, never a silent override.
    (
        "fluent_bit/input_chunk_sizes.qv",
        "flb_input_chunk_get_size",
        3,
    ),
    (
        "fluent_bit/input_chunk_sizes.qv",
        "flb_input_chunk_get_real_size",
        3,
    ),
    // Card W31-CORPUS-CONTRACTS-2: the aggregate size query over the input
    // INSTANCE (`src/flb_input_chunk.c:2947-2955` at the pinned checkout): one
    // cast field read, one call to the read-only ChunkIO walk
    // `cio_stream_size_chunks_up` (`lib/chunkio/src/cio_stream.c:258-276`),
    // one scalar return. `resource_borrowed(in)` / `resource_return_none()`
    // on this module's own body reading — the residency-mutating siblings
    // W29-SIZE-SIBLINGS-UNDECLARED also named stay undeclared. `size_t`
    // spells `usize` under the LP64 declared-ABI projection.
    (
        "fluent_bit/input_chunk_total_size.qv",
        "flb_input_chunk_total_size",
        3,
    ),
    // Card FB-EVENT-LOOP-RETENTION-AND-REGISTRATION: the collector setters,
    // each a registration forwarded through its returned id.
    (
        "fluent_bit/input_collector_registration.qv",
        "flb_input_set_collector_time",
        8,
    ),
    (
        "fluent_bit/input_collector_registration.qv",
        "flb_input_set_collector_event",
        7,
    ),
    (
        "fluent_bit/input_collector_registration.qv",
        "flb_input_set_collector_socket",
        7,
    ),
    // Card W47-LOOP-OWNER-INSTANCE-FLOW: the observations the census needed,
    // each a marker plus its clauses (held clauses wrapped for review).
    ("fluent_bit/input_instance.qv", "flb_create", 2),
    ("fluent_bit/input_instance.qv", "flb_input", 5),
    ("fluent_bit/input_instance.qv", "flb_input_new", 6),
    ("fluent_bit/input_instance.qv", "flb_service_set", 4),
    // Card W47-SERVICE-PROPERTY-KEYS: the property function the census reads
    // a keyed `flb_service_set` call through, withheld for review.
    ("fluent_bit/input_instance.qv", "flb_config_set_property", 5),
    // Card W47-CONDVAR-JOIN-STOP-PATHS: the marker, the contract-level review
    // marker carrying the observed join behaviour, one parameter mode and the
    // scalar result; every declaration is withheld awaiting review.
    // Card W47-FLB-LIB-LIFECYCLE: `flb_start` and `flb_stop` gain their
    // lifecycle obligation (4 -> 5); `flb_destroy` is added with the marker,
    // its lifecycle obligation, the review marker, one parameter mode and
    // the unit result.
    ("fluent_bit/lib_engine_thread.qv", "flb_start", 5),
    ("fluent_bit/lib_engine_thread.qv", "flb_stop", 5),
    ("fluent_bit/lib_engine_thread.qv", "flb_destroy", 5),
    (
        "fluent_bit/lib_engine_thread.qv",
        "flb_input_thread_instance_exit",
        4,
    ),
    (
        "fluent_bit/lib_engine_thread.qv",
        "flb_output_thread_pool_destroy",
        4,
    ),
    (
        "fluent_bit/lib_engine_thread.qv",
        "flb_downstream_worker_runtime_stop",
        4,
    ),
    ("fluent_bit/logging.qv", "flb_log_check", 3),
    // Card CONSTRUCTOR-RESIDUALS: the corpus moves 67 -> 68 modules with
    // `libc/errno.qv`, and this module gains one declaration. Deliberate;
    // see `docs/reports/remaining-constructor-residuals/README.md`. Five
    // clauses: the marker, three borrowed scalars/pointers, and the scalar
    // result. No variadic tail — the C declaration is not variadic.
    ("fluent_bit/logging.qv", "flb_errno_print", 5),
    ("fluent_bit/logging.qv", "flb_log_print", 7),
    // Card OP-APP-RK-CALLBACKS-VIA-POLL-CB: Fluent Bit's registered rk_conf callbacks.
    ("fluent_bit/logging.qv", "cb_kafka_msg", 6),
    ("fluent_bit/logging.qv", "cb_kafka_logger", 7),
    ("fluent_bit/logging.qv", "oauthbearer_token_refresh_cb", 6),
    // Card W47-LOOP-OWNER-CONTRACTS: the marker, the loop-owner obligation, two
    // borrowed arguments, the neutral user clause and the scalar result.
    ("fluent_bit/loop_owner.qv", "flb_input_collector_running", 6),
    ("fluent_bit/loop_owner.qv", "flb_input_collector_pause", 6),
    // Card VERDICT-CONTRACT-PREMISE-ANNOTATIONS: `requires resource_premise(ingestion_active, ...)`
    // (contract spec C04.18), one clause: 6 -> 7.
    ("fluent_bit/loop_owner.qv", "flb_input_collector_resume", 7),
    ("fluent_bit/loop_owner.qv", "flb_input_collector_delete", 6),
    ("fluent_bit/memory.qv", "flb_malloc", 6),
    ("fluent_bit/memory.qv", "flb_calloc", 7),
    ("fluent_bit/memory.qv", "flb_realloc", 9),
    ("fluent_bit/memory.qv", "flb_realloc_z", 12),
    ("fluent_bit/memory.qv", "flb_free", 5),
    // Card NULL-SAFETY-PROPERTY: `quant null-deref` checks (bounds
    // contract v1, the crate's bounds view): selector and one check each.
    ("fluent_bit/null_deref.qv", "flb_output_set_context", 2),
    ("fluent_bit/null_deref.qv", "flb_output_net_default", 2),
    ("fluent_bit/null_deref.qv", "flb_output_name", 2),
    ("fluent_bit/null_deref.qv", "flb_output_set_property", 2),
    ("fluent_bit/null_deref.qv", "flb_input_chunk_get_name", 2),
    // Card FB-PLUGIN-DESCRIPTOR-NONNULL-SCOPE (rules 14x-14z): the stable
    // plugin descriptor on the four instance constructors (selector and one
    // member clause each).
    ("fluent_bit/null_deref.qv", "flb_input_new", 2),
    ("fluent_bit/null_deref.qv", "flb_output_new", 2),
    ("fluent_bit/null_deref.qv", "flb_filter_new", 2),
    (
        "fluent_bit/null_deref.qv",
        "flb_processor_instance_create",
        2,
    ),
    // ... and ten function-scope entries `nonnull_member(flb_input_instance,
    // p, ins) where quant #scope [[scope]]` (selector and one entry each;
    // card NONNULL-MEMBER-UNIFIED-QUALIFIER migrated their spelling, counts
    // unchanged).
    ("fluent_bit/null_deref.qv", "flb_input_plugin_pause", 2),
    ("fluent_bit/null_deref.qv", "flb_input_plugin_resume", 2),
    ("fluent_bit/null_deref.qv", "flb_input_pause", 2),
    ("fluent_bit/null_deref.qv", "flb_input_resume", 2),
    ("fluent_bit/null_deref.qv", "flb_input_instance_pre_run", 2),
    ("fluent_bit/null_deref.qv", "flb_input_instance_exit", 2),
    (
        "fluent_bit/null_deref.qv",
        "flb_input_thread_instance_pre_run",
        2,
    ),
    (
        "fluent_bit/null_deref.qv",
        "flb_input_thread_instance_pause",
        2,
    ),
    (
        "fluent_bit/null_deref.qv",
        "flb_input_thread_instance_resume",
        2,
    ),
    (
        "fluent_bit/null_deref.qv",
        "flb_input_thread_instance_exit",
        2,
    ),
    // Card OUTER-CONTRACT (rule 31, C04.20): the outer contract
    // `flb_external_plugin` (outer contract v1, the crate's outer view, pinned
    // in `outer-contracts.lock`): the defining loader's selector, header,
    // runtime condition and four envelope lines; the citing loader's selector,
    // header and runtime condition.
    ("fluent_bit/outer.qv", "flb_plugin_load", 85),
    ("fluent_bit/outer.qv", "flb_plugin_proxy_create", 3),
    (
        "fluent_bit/output_flush_id.qv",
        "flb_output_flush_id_get",
        4,
    ),
    // Card W45-FB-RETRY-LIFETIME: OR-1 and OR-7 as checked declarations —
    // the marker, one mode per parameter, two exit clauses, the result.
    // Card VERDICT-CONTRACT-PREMISE-ANNOTATIONS: `requires resource_premise(synchronous_output_reachability, ...)`
    // (contract spec C04.18), one clause: 9 -> 10.
    (
        "fluent_bit/output_singleplex.qv",
        "flb_output_task_singleplex_enqueue",
        10,
    ),
    (
        "fluent_bit/output_singleplex.qv",
        "flb_output_task_flush",
        7,
    ),
    ("fluent_bit/resource_memory.qv", "flb_malloc", 3),
    ("fluent_bit/resource_memory.qv", "flb_calloc", 4),
    ("fluent_bit/resource_memory.qv", "flb_free", 3),
    ("fluent_bit/resource_memory.qv", "flb_sds_create_len", 4),
    ("fluent_bit/resource_memory.qv", "flb_sds_destroy", 3),
    // Card CONTRACT-INHERIT-ADOPTION: user-directed declared assumptions,
    // each inheriting `libc::strdup` / `libc::strndup`; elaborated counts.
    ("fluent_bit/resource_string.qv", "flb_strdup", 3),
    ("fluent_bit/resource_string.qv", "flb_strndup", 4),
    // Card PROD-CONTRACTS, continued.
    ("fluent_bit/routes_mask.qv", "flb_routes_mask_set_by_tag", 6),
    ("fluent_bit/routes_mask.qv", "flb_routes_mask_get_size", 3),
    ("fluent_bit/routes_mask.qv", "flb_routes_mask_get_slots", 4),
    ("fluent_bit/routes_mask.qv", "flb_routes_mask_is_empty", 5),
    // Card W45-FB-BACKLOG-CUSTODY: the twelve storage-backlog functions of
    // `plugins/in_storage_backlog/sb.c` as CHECKED SPECIFICATIONS (contract
    // spec §C06.3): each carries `requires resource_body_checked;` beside the
    // marker, so every count below is two requirements, one clause per
    // parameter and the result mode. A specification is in force only in a
    // unit that defines its body and whose body establishes it; the corpus
    // units define none of these, so no corpus call site applies them.
    // Deliberate; see `docs/reports/w45-fb-backlog-custody/README.md`.
    ("fluent_bit/storage_backlog.qv", "sb_allocate_chunk", 6),
    ("fluent_bit/storage_backlog.qv", "sb_destroy_chunk", 4),
    ("fluent_bit/storage_backlog.qv", "sb_destroy_backlog", 5),
    ("fluent_bit/storage_backlog.qv", "sb_allocate_backlogs", 4),
    ("fluent_bit/storage_backlog.qv", "sb_destroy_backlogs", 4),
    (
        "fluent_bit/storage_backlog.qv",
        "sb_remove_chunk_from_segregated_backlog",
        6,
    ),
    (
        "fluent_bit/storage_backlog.qv",
        "sb_remove_chunk_from_segregated_backlogs",
        5,
    ),
    (
        "fluent_bit/storage_backlog.qv",
        "sb_append_chunk_to_segregated_backlog",
        7,
    ),
    (
        "fluent_bit/storage_backlog.qv",
        "sb_append_chunk_to_segregated_backlogs",
        6,
    ),
    ("fluent_bit/storage_backlog.qv", "sb_segregate_chunks", 4),
    (
        "fluent_bit/storage_backlog.qv",
        "sb_release_output_queue_space",
        5,
    ),
    ("fluent_bit/storage_backlog.qv", "cb_queue_chunks", 6),
    // Card NEIGHBOURS, continued. The FIRST production declaration whose C
    // result is floating-point: `double flb_time_now()` is declared `-> f64`
    // rather than projected onto an integer it does not have, which is why the
    // card added `f32`/`f64` to `SCALAR_ABI_TYPES`. Two clauses: the marker and
    // the result mode. It takes no parameter, so it has no parameter clause.
    // Card W45-FB-RETRY-LIFETIME: the counter operations (the drop carries a
    // literal premise) and two neutral callees.
    (
        "fluent_bit/task_route_queries.qv",
        "flb_task_get_route_status",
        5,
    ),
    (
        "fluent_bit/task_route_queries.qv",
        "flb_task_set_route_status",
        6,
    ),
    (
        "fluent_bit/task_route_queries.qv",
        "flb_task_retry_count",
        5,
    ),
    (
        "fluent_bit/task_route_queries.qv",
        "flb_task_running_count",
        4,
    ),
    ("fluent_bit/task_users.qv", "flb_task_users_inc", 4),
    ("fluent_bit/task_users.qv", "flb_task_users_dec", 6),
    ("fluent_bit/task_users.qv", "flb_task_retry_destroy", 4),
    (
        "fluent_bit/task_users.qv",
        "flb_output_thread_pool_flush",
        6,
    ),
    // Card FB-TEARDOWN-DESTROY-DECLARATIONS: the teardown destroyers, ten
    // with rule 30's member-path states (read at a direct call).
    (
        "fluent_bit/teardown_destroy.qv",
        "flb_sched_timer_destroy",
        3,
    ),
    (
        "fluent_bit/teardown_destroy.qv",
        "flb_sched_request_destroy",
        4,
    ),
    ("fluent_bit/teardown_destroy.qv", "flb_task_destroy", 6),
    (
        "fluent_bit/teardown_destroy.qv",
        "flb_task_queue_destroy",
        3,
    ),
    (
        "fluent_bit/teardown_destroy.qv",
        "flb_event_chunk_destroy",
        4,
    ),
    (
        "fluent_bit/teardown_destroy.qv",
        "flb_output_flush_destroy",
        4,
    ),
    ("fluent_bit/teardown_destroy.qv", "flb_coro_destroy", 4),
    (
        "fluent_bit/teardown_destroy.qv",
        "flb_input_chunk_release_route",
        4,
    ),
    (
        "fluent_bit/teardown_destroy.qv",
        "flb_output_instance_destroy",
        34,
    ),
    (
        "fluent_bit/teardown_destroy.qv",
        "flb_input_instance_destroy",
        34,
    ),
    (
        "fluent_bit/teardown_destroy.qv",
        "flb_input_coro_destroy",
        4,
    ),
    (
        "fluent_bit/teardown_destroy.qv",
        "flb_input_collector_destroy",
        3,
    ),
    ("fluent_bit/teardown_destroy.qv", "flb_tail_file_remove", 15),
    (
        "fluent_bit/teardown_destroy.qv",
        "flb_tail_config_destroy",
        18,
    ),
    ("fluent_bit/time.qv", "flb_time_now", 2),
    (
        "language/quantified_indices.qv",
        "preserve_five_chunk_credits",
        2,
    ),
    // Card W43-LIBC-CONTRACTS: eleven libc/POSIX modules, all owned by
    // `providers/quirin-provider-libc`, 55 declarations, each a trusted
    // assumption read from the prototype the pinned units declare. Every
    // declaration is the neutral element (borrowed parameters, a `none`
    // result: two clauses plus one per parameter) except `strdup`
    // (`return_owned(nullable)`, 3) and `prctl` (literal premise on its
    // option plus a variadic tail, 5). Deliberate; see
    // `docs/reports/w43-libc-contracts/README.md`.
    // Card W43-DESCRIPTOR-RESOURCE-TRACKING: the new `libc/dirent.qv`, a
    // directory-stream pair. `opendir`: the marker, one borrowed path, the
    // owned result and its `directory` kind (4); `closedir`: the marker, the
    // released stream, its kind and `none` (4). Deliberate; see
    // `docs/reports/w43-descriptor-resource-tracking/README.md`.
    ("libc/dirent.qv", "opendir", 5),
    ("libc/dirent.qv", "closedir", 5),
    ("libc/dlfcn.qv", "dlerror", 3),
    // Card W43-LIBC-RESTRICT-PROTOTYPES: ten `__restrict` prototypes, each
    // the neutral element (the marker, one borrowed clause per parameter,
    // `none`), in `dlfcn`, `socket`, `sys_stat`, `threads`, `time` and the new
    // `libc/regex.qv`. Deliberate; see
    // `docs/reports/w43-libc-restrict-prototypes/README.md`.
    ("libc/dlfcn.qv", "dlsym", 5),
    // Card CG25-RESOURCE-KINDS: `dlopen` (the marker, the flexible revision,
    // two borrowed clauses, the owned result and its `dynamic_library` kind:
    // 6) and `dlclose` (marker, revision, the release, its kind, `none`: 5),
    // appended to `libc/dlfcn.qv`. Deliberate; see
    // `docs/reports/cg25-resource-kinds/README.md`.
    ("libc/dlfcn.qv", "dlopen", 6),
    ("libc/dlfcn.qv", "dlclose", 5),
    // Card CONSTRUCTOR-RESIDUALS, continued. Two clauses: the marker and the
    // result mode. It takes no parameter, so it has no parameter clause — the
    // same shape as `fluent_bit/time.qv`'s `flb_time_now`.
    // Card VERDICT-CONTRACT-PREMISE-ANNOTATIONS: `requires resource_premise(nested_call_admission, ...)`
    // (contract spec C04.18), one clause: 3 -> 4.
    ("libc/errno.qv", "__errno_location", 4),
    // Card W44-LIBC-RESIDUALS: `strerror`, appended beside the `errno`
    // accessor (the marker, one borrowed scalar, `none`: 3).
    // Card VERDICT-CONTRACT-PREMISE-ANNOTATIONS: `requires resource_premise(nested_call_admission, ...)`
    // (contract spec C04.18), one clause: 4 -> 5.
    ("libc/errno.qv", "strerror", 5),
    ("libc/math.qv", "ceil", 4),
    ("libc/math.qv", "floor", 4),
    ("libc/math.qv", "fabs", 4),
    ("libc/math.qv", "pow", 5),
    ("libc/memory.qv", "malloc", 4),
    ("libc/memory.qv", "calloc", 5),
    ("libc/memory.qv", "free", 4),
    ("libc/process.qv", "prctl", 6),
    // Card W43-ABI-ENUM-AND-CONST-PP: `getrlimit`/`setrlimit` over the
    // enumeration typedef `__rlimit_resource_t` (rule 12u): marker, flexible
    // revision, two borrowed formals, `none`.
    ("libc/process.qv", "getrlimit", 5),
    ("libc/process.qv", "setrlimit", 5),
    // Card CONSTRUCTOR-RESIDUALS-ABI-TYPEDEF. Three clauses each: the marker,
    // the borrowed mutex pointer and the `int` result. `pthread_mutex_t` is a
    // union the typedef DEFINES, which card LOADER-ABI's reader would not read;
    // this card's widening reads it, and neither function takes custody of the
    // object, so `resource_borrowed` is the whole custody statement.
    ("libc/pthread_mutex.qv", "pthread_mutex_lock", 4),
    ("libc/pthread_mutex.qv", "pthread_mutex_unlock", 4),
    // Card W47-CONDVAR-JOIN-PUBLICATION: the marker, the flexible revision,
    // the thread-synchronization obligation, one borrowed clause per
    // parameter and the scalar result; `pthread_join` adds its null-result
    // literal premise.
    ("libc/pthread_sync.qv", "pthread_cond_init", 6),
    ("libc/pthread_sync.qv", "pthread_cond_destroy", 5),
    ("libc/pthread_sync.qv", "pthread_cond_signal", 5),
    ("libc/pthread_sync.qv", "pthread_cond_wait", 6),
    ("libc/pthread_sync.qv", "pthread_cond_timedwait", 7),
    ("libc/pthread_sync.qv", "pthread_join", 7),
    // Card W43-LIBC-RESTRICT-PROTOTYPES, continued: `libc/regex.qv`: neutral (2 + 1 per parameter).
    // Card CG25-RESOURCE-KINDS adds one clause to each, `resource_kind(preg,
    // regex)`: the compiled object is a use of the in-place `regex` kind
    // (8 -> 9, 7 -> 8), and appends `regcomp` (marker, revision, the in-place
    // initialiser, its kind, two borrowed clauses, `none`: 7) and `regfree`
    // (marker, revision, the finaliser, its kind, `none`: 5).
    ("libc/regex.qv", "regexec", 9),
    ("libc/regex.qv", "regerror", 8),
    ("libc/regex.qv", "regcomp", 7),
    ("libc/regex.qv", "regfree", 5),
    ("libc/socket.qv", "setsockopt", 8),
    ("libc/socket.qv", "send", 7),
    ("libc/socket.qv", "recv", 7),
    ("libc/socket.qv", "listen", 5),
    ("libc/socket.qv", "sendfile", 7),
    ("libc/socket.qv", "htonl", 4),
    ("libc/socket.qv", "ntohl", 4),
    ("libc/socket.qv", "gai_strerror", 4),
    // Card W43-LIBC-RESTRICT-PROTOTYPES, continued: neutral (2 + 1 per parameter).
    ("libc/socket.qv", "getsockopt", 8),
    ("libc/socket.qv", "getpeername", 6),
    // Card W43-DESCRIPTOR-RESOURCE-TRACKING: `socket`, appended. The marker,
    // three borrowed scalars, the `sentinel` owned result and its `descriptor`
    // kind (6).
    ("libc/socket.qv", "socket", 7),
    ("libc/stdio.qv", "perror", 4),
    ("libc/stdio.qv", "fflush", 4),
    ("libc/stdio.qv", "remove", 4),
    // Card W43-LIBC-PRINTF-FAMILY: the formatted-output family, each a
    // trusted assumption read from the prototype the pinned units declare. The
    // marker, one borrowed clause per declared parameter, the variadic data
    // tail and the scalar result: 3 + parameters clauses each. Deliberate; see
    // `docs/reports/w43-libc-printf-family/README.md`.
    ("libc/stdio.qv", "printf", 5),
    ("libc/stdio.qv", "fprintf", 6),
    ("libc/stdio.qv", "snprintf", 7),
    ("libc/stdio.qv", "sprintf", 6),
    // Card W43-DESCRIPTOR-RESOURCE-TRACKING: the stream pair and `fileno`,
    // appended. `fopen`: marker, two borrowed, owned result, `stream` kind (5);
    // `fclose`: marker, released stream, kind, `none` (4); `fileno`: the
    // neutral element over one parameter (3).
    ("libc/stdio.qv", "fopen", 6),
    ("libc/stdio.qv", "fclose", 5),
    ("libc/stdio.qv", "fileno", 4),
    // Card W44-LIBC-RESIDUALS: the two called `va_list` members, appended.
    // The marker, one borrowed clause per parameter (the `va_list` included)
    // and `none`: `vsnprintf` 6, `vprintf` 4.
    ("libc/stdio.qv", "vsnprintf", 7),
    ("libc/stdio.qv", "vprintf", 5),
    ("libc/stdlib.qv", "atoi", 4),
    ("libc/stdlib.qv", "srand", 4),
    ("libc/stdlib.qv", "rand_r", 4),
    ("libc/stdlib.qv", "system", 4),
    // Card CG21-CALL-EXITS-AND-BOUNDARIES: the no-return exits (3 + 1 per
    // parameter: the marker, the revision, the result and the exit).
    ("libc/stdlib.qv", "abort", 4),
    ("libc/stdlib.qv", "exit", 5),
    ("libc/stdlib.qv", "__assert_fail", 8),
    // Card SLOT-MOVE-AND-INTERIOR-BORROW: the derived out slots (rule 12z),
    // flexible like every libc declaration.
    ("libc/stdlib.qv", "strtol", 7),
    ("libc/stdlib.qv", "strtoll", 7),
    ("libc/stdlib.qv", "strtoul", 7),
    ("libc/stdlib.qv", "strtoull", 7),
    ("libc/stdlib.qv", "strtod", 6),
    // CG22-CALLBACK-REGISTRY-CONTRACT: synchronous comparators.
    ("libc/stdlib.qv", "bsearch", 9),
    ("libc/stdlib.qv", "qsort", 8),
    ("libc/string.qv", "strcmp", 5),
    ("libc/string.qv", "strlen", 4),
    // W46-LIBC-EVENT-CONTRACT-GAPS: scalar byte/string reads. These are
    // reusable library declarations; the current corpus has no uncontracted
    // leaves for these four names.
    ("libc/string.qv", "memcmp", 6),
    ("libc/string.qv", "strncmp", 6),
    ("libc/string.qv", "strnlen", 5),
    ("libc/string.qv", "strcasecmp", 5),
    ("libc/string.qv", "strdup", 4),
    ("libc/string.qv", "strerror_r", 6),
    // Card W43-LIBC-RESULT-ALIAS: thirteen derived-result declarations, each a
    // trusted assumption read from the prototype the pinned units declare. The
    // shape is the neutral one with a derived result in place of `none`: the
    // marker, one borrowed clause per parameter and the result mode. Five exact
    // aliases (`resource_return_alias(dest, nonnull)`) and six interior
    // pointers (`resource_return_interior(s, nullable)`) here, two exact
    // aliases in `libc/time.qv`. Deliberate; see
    // `docs/reports/w43-libc-result-alias/README.md`.
    ("libc/string.qv", "strcpy", 5),
    ("libc/string.qv", "strncpy", 6),
    ("libc/string.qv", "memcpy", 6),
    ("libc/string.qv", "memmove", 6),
    ("libc/string.qv", "memset", 6),
    ("libc/string.qv", "strchr", 5),
    ("libc/string.qv", "strrchr", 5),
    ("libc/string.qv", "strstr", 5),
    ("libc/string.qv", "strcasestr", 5),
    ("libc/string.qv", "memchr", 6),
    ("libc/string.qv", "memrchr", 6),
    // Card CONTRACT-INHERIT-ADOPTION: `strndup`, user-directed; `strdup`'s
    // custody statement plus a borrowed size (2 + 1 per parameter + owned).
    ("libc/string.qv", "strndup", 5),
    ("libc/sys_stat.qv", "fstat", 5),
    ("libc/sys_stat.qv", "mkdir", 5),
    ("libc/sys_stat.qv", "umask", 4),
    // Card W43-LIBC-RESTRICT-PROTOTYPES, continued: neutral (2 + 1 per parameter).
    ("libc/sys_stat.qv", "stat", 5),
    // Card W48-CALLEE-CLOSURE-R3: `stat`'s neutral statement for `lstat`
    // (2 + 1 per parameter).
    ("libc/sys_stat.qv", "lstat", 5),
    ("libc/threads.qv", "mtx_lock", 4),
    ("libc/threads.qv", "mtx_unlock", 4),
    ("libc/threads.qv", "cnd_wait", 5),
    ("libc/threads.qv", "pthread_rwlock_rdlock", 4),
    ("libc/threads.qv", "pthread_rwlock_wrlock", 4),
    ("libc/threads.qv", "pthread_rwlock_unlock", 4),
    ("libc/threads.qv", "pthread_self", 3),
    ("libc/threads.qv", "pthread_kill", 5),
    ("libc/threads.qv", "pthread_setcancelstate", 5),
    ("libc/threads.qv", "pthread_setcanceltype", 5),
    ("libc/threads.qv", "pthread_attr_setdetachstate", 5),
    ("libc/threads.qv", "sigemptyset", 4),
    ("libc/threads.qv", "sigfillset", 4),
    ("libc/threads.qv", "sigaddset", 5),
    // Card W43-LIBC-RESTRICT-PROTOTYPES, continued: neutral (2 + 1 per parameter).
    ("libc/threads.qv", "cnd_timedwait", 6),
    ("libc/threads.qv", "pthread_sigmask", 6),
    // CG22-CALLBACK-REGISTRY-CONTRACT: a synchronous initialiser.
    ("libc/threads.qv", "pthread_once", 6),
    ("libc/time.qv", "time", 4),
    ("libc/time.qv", "timegm", 4),
    ("libc/time.qv", "gmtime_r", 5),
    ("libc/time.qv", "localtime_r", 5),
    // Card W43-LIBC-RESTRICT-PROTOTYPES, continued: neutral (2 + 1 per parameter).
    ("libc/time.qv", "strftime", 7),
    ("libc/time.qv", "gettimeofday", 5),
    // Card W48-CALLEE-CLOSURE-R3: `clock_gettime`, neutral (2 + 1 per
    // parameter).
    ("libc/time.qv", "clock_gettime", 5),
    ("libc/unistd.qv", "sleep", 4),
    ("libc/unistd.qv", "usleep", 4),
    ("libc/unistd.qv", "sysconf", 4),
    ("libc/unistd.qv", "getpagesize", 3),
    ("libc/unistd.qv", "geteuid", 3),
    ("libc/unistd.qv", "getegid", 3),
    ("libc/unistd.qv", "setsid", 3),
    ("libc/unistd.qv", "access", 5),
    ("libc/unistd.qv", "unlink", 4),
    ("libc/unistd.qv", "read", 6),
    ("libc/unistd.qv", "write", 6),
    // Card W43-DESCRIPTOR-RESOURCE-TRACKING: the descriptor pair, appended.
    // `open`: marker, two borrowed, the variadic data tail, the `sentinel`
    // owned result and its `descriptor` kind (6); `close`: marker, the `valid`
    // released descriptor, its kind and `none` (4).
    ("libc/unistd.qv", "open", 7),
    ("libc/unistd.qv", "close", 5),
    // Card W46-EXTERNAL-CALLEE-CONTRACTS: `fcntl` at `F_SETFD` only: marker,
    // flexible, the premise `cmd == 2`, two borrowed parameters, the data
    // tail and `none` (7).
    ("libc/unistd.qv", "fcntl", 7),
    ("library/custody_evidence.qv", "publish_library_resource", 6),
    ("library/custody_evidence.qv", "handoff_library_resource", 6),
    ("library/custody_evidence.qv", "reclaim_library_resource", 6),
    (
        "library/custody_evidence.qv",
        "check_imported_library_custody",
        7,
    ),
    ("library/custody_evidence.qv", "release_library_resource", 3),
    // Card W43-DESCRIPTOR-NATIVE-CHECK: the executable lifecycle policy the
    // native descriptor replay (`qcore_contract::descriptor_observation`)
    // evaluates on recorded executions, one parameterless rule per property.
    (
        "library/descriptor_native_observation.qv",
        "descriptor_generation_fresh",
        3,
    ),
    (
        "library/descriptor_native_observation.qv",
        "descriptor_counted_reference",
        3,
    ),
    (
        "library/descriptor_native_observation.qv",
        "descriptor_sentinel_validity",
        5,
    ),
    (
        "library/descriptor_native_observation.qv",
        "descriptor_release_at_most_once",
        1,
    ),
    (
        "library/descriptor_native_observation.qv",
        "descriptor_release_matches_kind",
        2,
    ),
    (
        "library/descriptor_native_observation.qv",
        "descriptor_no_use_after_release",
        1,
    ),
    (
        "library/descriptor_native_observation.qv",
        "descriptor_operand_identity_valid",
        1,
    ),
    (
        "library/descriptor_native_observation.qv",
        "descriptor_relation_holds",
        1,
    ),
    (
        "library/descriptor_native_observation.qv",
        "descriptor_mapping_extent",
        1,
    ),
    (
        "library/descriptor_native_observation.qv",
        "descriptor_leak_set_well_defined",
        3,
    ),
    ("library/source_projection.qv", "source_enum_value", 2),
    ("library/source_projection.qv", "source_scan_effects", 3),
    ("library/source_projection.qv", "source_scan_initial", 2),
    ("library/source_projection.qv", "source_type_match", 1),
    ("library/source_projection.qv", "source_symbol", 1),
    ("library/source_projection.qv", "source_loop_index", 1),
    ("library/source_projection.qv", "source_scan_region", 1),
    ("library/source_projection.qv", "frame_write", 2),
    ("library/source_projection.qv", "frame_declaration", 2),
    ("library/source_projection.qv", "frame_dependency", 2),
    // Card W36-CORPUS-CONTRACTS-3: cJSON is vendored INSIDE librdkafka
    // (`lib/librdkafka-2.15.0/src/cJSON.c`), which is why this module sits in
    // the librdkafka family. Read from `:253-276`; `nullable` because the loop
    // condition `while (item != NULL)` precedes every dereference, and the
    // three call sites do reach the null path.
    ("librdkafka/cjson.qv", "cJSON_Delete", 3),
    // Card W45-CALLEE-CASCADE-RESIDUAL: the node constructor (`:241-250`):
    // marker, borrowed hook table, an owned nullable result.
    // Card VERDICT-CONTRACT-PREMISE-ANNOTATIONS: `requires resource_premise(cjson_allocator_hooks, ...)`
    // (contract spec C04.18), one clause: 3 -> 4.
    ("librdkafka/cjson.qv", "cJSON_New_Item", 4),
    // Card CALLEE-RETENTION-CLASS1, stage (a): the custody-taking inserts,
    // marker, borrowed formals, two return-code classes, `none`.
    // Card VERDICT-CONTRACT-PREMISE-ANNOTATIONS: `requires resource_premise(cjson_list_invariant, ...)`
    // (contract spec C04.18), one clause: 5 -> 6.
    ("librdkafka/cjson.qv", "add_item_to_array", 6),
    ("librdkafka/cjson.qv", "replace_item_in_object", 7),
    // Card SLOT-MOVE-AND-INTERIOR-BORROW: a borrowed child (class 8, rule
    // 12y) and a derived out slot beside a `global` retention (rules 12z, 12t).
    ("librdkafka/cjson.qv", "get_object_item", 5),
    ("librdkafka/cjson.qv", "cJSON_ParseWithLengthOpts", 8),
    // Card QR-PRINT-ROUTE-MATCHES-DECLARATION: cJSON's static `print`
    // (`:1234-1299`), re-declared as W45-CALLEE-CASCADE-RESIDUAL round 3
    // recorded it once the module compiles off the print-call route: marker,
    // the allocator-hooks premise, three borrowed formals, an owned nullable
    // result.
    ("librdkafka/cjson.qv", "print", 6),
    // Card W43-ABI-ENUM-AND-CONST-PP: librdkafka functions over enumeration
    // (rule 12u) and pointer-to-pointer-to-const (rule 12v) prototypes:
    // marker, borrowed formals, a variadic data tail for the two variadic
    // ones, two retentions (`resource_retained_by`), the result mode.
    ("librdkafka/client.qv", "rd_kafka_error_new_v", 5),
    ("librdkafka/client.qv", "rd_kafka_error_new", 5),
    // Card LIBRDKAFKA-PLUGIN-PREMISE-UNSTATED: +1 on each of the 34 client.qv declarations
    // whose body may reach rk_conf.log_cb (`rk_no_plugin_library`, user decision U2).
    ("librdkafka/client.qv", "rd_kafka_err_action", 7),
    (
        "librdkafka/client.qv",
        "rd_kafka_oauthbearer_set_token_failure",
        5,
    ),
    ("librdkafka/client.qv", "rd_kafka_toppars_pause_resume", 8),
    ("librdkafka/client.qv", "rd_kafka_AclBinding_new0", 11),
    (
        "librdkafka/client.qv",
        "rd_kafka_ConsumerGroupDescription_new",
        12,
    ),
    ("librdkafka/client.qv", "rd_kafka_admin_add_config0", 5),
    ("librdkafka/client.qv", "rd_kafka_anyconf_dump", 7),
    ("librdkafka/client.qv", "rd_kafka_confval_set_type", 8),
    ("librdkafka/client.qv", "rd_kafka_interceptor_method_add", 8),
    ("librdkafka/client.qv", "rd_flags2str", 6),
    // Card SLOT-MOVE-AND-INTERIOR-BORROW: borrowed results (class 8, rule 12y)
    // and a derived out slot (rule 12z).
    (
        "librdkafka/client.qv",
        "rd_kafka_admin_result_ret_topics",
        4,
    ),
    (
        "librdkafka/client.qv",
        "rd_kafka_admin_result_ret_resources",
        4,
    ),
    (
        "librdkafka/client.qv",
        "rd_kafka_admin_result_ret_groups",
        4,
    ),
    (
        "librdkafka/client.qv",
        "rd_kafka_topic_partition_get_private",
        3,
    ),
    ("librdkafka/client.qv", "rd_slice_reader0", 6),
    // Card REFCOUNT-RESOURCE-KIND (C04.17, rules 13b and 13c): four keeps and
    // five drops (marker, borrowed formals, the reference clause, the alias
    // or `none` result), the queue handle constructor and six destructors
    // (`rd_kafka_topic_partition_destroy0` under its literal premise).
    ("librdkafka/client.qv", "rd_kafka_toppar_keep0", 6),
    ("librdkafka/client.qv", "rd_kafka_q_keep", 4),
    ("librdkafka/client.qv", "rd_kafka_q_keep_nolock", 4),
    ("librdkafka/client.qv", "rd_kafka_topic_keep", 4),
    ("librdkafka/client.qv", "rd_kafka_toppar_destroy0", 7),
    ("librdkafka/client.qv", "rd_kafka_q_destroy0", 6),
    ("librdkafka/client.qv", "rd_kafka_q_destroy_owner", 5),
    ("librdkafka/client.qv", "rd_kafka_topic_destroy0", 5),
    ("librdkafka/client.qv", "rd_kafka_enq_once_trigger", 7),
    // 4 -> 5 by card REFCOUNT-REFERENCE-RETURNING-DECLARATIONS: the handle
    // retains `rk` by its result (C04.14.2, rule 13e).
    ("librdkafka/client.qv", "rd_kafka_queue_new0", 5),
    (
        "librdkafka/client.qv",
        "rd_kafka_topic_partition_list_destroy",
        4,
    ),
    (
        "librdkafka/client.qv",
        "rd_kafka_topic_partition_destroy0",
        6,
    ),
    (
        "librdkafka/client.qv",
        "rd_kafka_AlterConsumerGroupOffsets_destroy",
        4,
    ),
    (
        "librdkafka/client.qv",
        "rd_kafka_DeleteConsumerGroupOffsets_destroy",
        4,
    ),
    (
        "librdkafka/client.qv",
        "rd_kafka_ListConsumerGroupOffsets_destroy",
        4,
    ),
    (
        "librdkafka/client.qv",
        "rd_kafka_topic_partition_result_destroy",
        4,
    ),
    // Card REFCOUNT-REFERENCE-RETURNING-DECLARATIONS (C04.17, C04.14.2, rules
    // 13d and 13e): nine returned references (marker, borrowed formals, the
    // owned result and its reference kind; `rd_kafka_q_new0` also retains
    // `rk` and `func` by its result) and one drop.
    ("librdkafka/client.qv", "rd_kafka_toppar_get0", 8),
    ("librdkafka/client.qv", "rd_kafka_toppar_desired_get", 5),
    // Card VERDICT-CONTRACT-PREMISE-ANNOTATIONS: the four non-null results
    // (`rd_kafka_toppar_new0`, `rd_kafka_toppar_desired_add`, `rd_kafka_q_new0`,
    // `ut_create_mock_queue`) state `requires resource_premise(asserted_allocation,
    // ...)` (contract spec C04.18), one clause each: 7 -> 8, 5 -> 6, 9 -> 10, 3 -> 4.
    ("librdkafka/client.qv", "rd_kafka_toppar_new0", 9),
    ("librdkafka/client.qv", "rd_kafka_toppar_desired_add", 7),
    ("librdkafka/client.qv", "rd_kafka_toppar_get2", 9),
    ("librdkafka/client.qv", "rd_kafka_toppar_get_avail", 8),
    ("librdkafka/client.qv", "rd_kafka_q_new0", 10),
    ("librdkafka/client.qv", "ut_create_mock_toppar", 7),
    ("librdkafka/client.qv", "ut_create_mock_queue", 4),
    ("librdkafka/client.qv", "ut_destroy_mock_queue", 5),
    // Card REFCOUNT-FAMILY-BLOCKING-CALLEES: four plain declarations between a
    // reference's acquisition and its drop (marker, borrowed formals, `none`).
    // Card VERDICT-CONTRACT-DEPENDENCIES: the first two also state the
    // served-queue premise (`requires resource_premise(served_queue, ...)`,
    // contract spec C04.18), one clause each: 4 -> 5.
    ("librdkafka/client.qv", "rd_kafka_q_wait_result", 6),
    ("librdkafka/client.qv", "rd_kafka_q_purge0", 6),
    ("librdkafka/client.qv", "rd_kafka_toppar_needs_query", 4),
    ("librdkafka/client.qv", "rd_kafka_toppar_fetch_backoff", 6),
    // CG22-CALLBACK-REGISTRY-CONTRACT: lists with a free callback and
    // nanopb's output stream.
    ("librdkafka/client.qv", "rd_list_init", 6),
    ("librdkafka/client.qv", "rd_list_new", 5),
    ("librdkafka/client.qv", "rd_list_add", 5),
    ("librdkafka/client.qv", "rd_list_destroy", 7),
    ("librdkafka/client.qv", "rd_list_clear", 5),
    ("librdkafka/client.qv", "pb_write", 6),
    ("librdkafka/client.qv", "pb_encode_varint", 5),
    // Card W45-CALLEE-CASCADE-RESIDUAL round 3: five Admin-API destructors (marker,
    // released, `none`) and `unit_test_telemetry` (marker, ten borrowed, the
    // callback invocation, `none`).
    ("librdkafka/client.qv", "rd_kafka_AclBinding_destroy", 3),
    ("librdkafka/client.qv", "rd_kafka_ConfigResource_destroy", 3),
    ("librdkafka/client.qv", "rd_kafka_NewPartitions_destroy", 3),
    ("librdkafka/client.qv", "rd_kafka_NewTopic_destroy", 3),
    (
        "librdkafka/client.qv",
        "rd_kafka_UserScramCredentialAlteration_destroy",
        3,
    ),
    ("librdkafka/client.qv", "unit_test_telemetry", 14),
    // Card CG22-OP-CALLBACKS-AND-SERVED-QUEUES: the queue serving family
    // (marker, every formal borrowed, the callback invocation, the two op
    // member invocations, the result) and `rd_kafka_op_destroy` (marker, the
    // discriminant, the release, the op callback, 170 arm clauses, `none`).
    ("librdkafka/client.qv", "rd_kafka_q_serve0", 13),
    ("librdkafka/client.qv", "rd_kafka_q_serve", 12),
    ("librdkafka/client.qv", "rd_kafka_q_serve_maybe_consume", 12),
    ("librdkafka/client.qv", "rd_kafka_q_pop_serve0", 13),
    ("librdkafka/client.qv", "rd_kafka_q_pop_serve", 12),
    (
        "librdkafka/client.qv",
        "rd_kafka_q_pop_serve_maybe_consume",
        12,
    ),
    ("librdkafka/client.qv", "rd_kafka_op_destroy", 176),
    // Card OP-CONSTRUCTOR-DECLARATIONS: the op constructors (a result state,
    // C04.19.4; `rd_kafka_op_new0`'s `quant live #result` state clause is one
    // clause) and the request family.
    ("librdkafka/client.qv", "rd_kafka_op_new0", 7),
    ("librdkafka/client.qv", "rd_kafka_op_new_cb", 9),
    ("librdkafka/client.qv", "rd_kafka_op_req0", 9),
    ("librdkafka/client.qv", "rd_kafka_op_req", 8),
    ("librdkafka/client.qv", "rd_kafka_op_req2", 7),
    // Card OP-CALLBACK-CANDIDATE-CONTRACTS: the declared op-callback
    // candidates. An `rko_op_cb` candidate: marker, the `rk_log_callback`
    // premise, three borrowed formals, `none` (6), plus one more premise for
    // the broker monitor and the assignor (7); an `rko_serve` candidate:
    // marker, the premise, the op consumed, four borrowed, `none` (8), plus
    // the `rk_closesocket_callback` premise for the mock cluster (9).
    ("librdkafka/client.qv", "rd_kafka_broker_monitor_op_cb", 7),
    (
        "librdkafka/client.qv",
        "rd_kafka_cgrp_assignor_handle_Metadata_op",
        7,
    ),
    (
        "librdkafka/client.qv",
        "rd_kafka_cgrp_consumer_handle_Metadata_op",
        6,
    ),
    (
        "librdkafka/client.qv",
        "rd_kafka_cgrp_handle_Metadata_op",
        6,
    ),
    ("librdkafka/client.qv", "rd_kafka_offset_reset_op_cb", 6),
    ("librdkafka/client.qv", "rd_kafka_offset_validate_op_cb", 6),
    (
        "librdkafka/client.qv",
        "rd_kafka_share_ack_commit_cb_register_op",
        6,
    ),
    (
        "librdkafka/client.qv",
        "rd_kafka_share_fetch_reply_op_cb",
        6,
    ),
    (
        "librdkafka/client.qv",
        "rd_kafka_txn_op_init_transactions",
        6,
    ),
    (
        "librdkafka/client.qv",
        "rd_kafka_txn_op_ack_init_transactions",
        6,
    ),
    (
        "librdkafka/client.qv",
        "rd_kafka_txn_op_begin_transaction",
        6,
    ),
    ("librdkafka/client.qv", "rd_kafka_txn_op_begin_commit", 6),
    (
        "librdkafka/client.qv",
        "rd_kafka_txn_op_commit_transaction",
        6,
    ),
    (
        "librdkafka/client.qv",
        "rd_kafka_txn_op_commit_transaction_ack",
        6,
    ),
    ("librdkafka/client.qv", "rd_kafka_txn_op_begin_abort", 6),
    (
        "librdkafka/client.qv",
        "rd_kafka_txn_op_abort_transaction",
        6,
    ),
    (
        "librdkafka/client.qv",
        "rd_kafka_txn_op_abort_transaction_ack",
        6,
    ),
    ("librdkafka/client.qv", "rd_kafka_cgrp_op_serve", 8),
    ("librdkafka/client.qv", "rd_kafka_mock_cluster_op_serve", 8),
    // Card OP-APP-RK-CALLBACKS-VIA-POLL-CB: the rk_conf setters, `rd_kafka_conf_new`, the member
    // invokers and the run-closed candidates.
    (
        "librdkafka/client.qv",
        "rd_kafka_conf_set_background_event_cb",
        7,
    ),
    ("librdkafka/client.qv", "rd_kafka_conf_set_dr_cb", 7),
    ("librdkafka/client.qv", "rd_kafka_conf_set_dr_msg_cb", 7),
    ("librdkafka/client.qv", "rd_kafka_conf_set_consume_cb", 7),
    ("librdkafka/client.qv", "rd_kafka_conf_set_rebalance_cb", 7),
    (
        "librdkafka/client.qv",
        "rd_kafka_conf_set_offset_commit_cb",
        7,
    ),
    ("librdkafka/client.qv", "rd_kafka_conf_set_error_cb", 7),
    ("librdkafka/client.qv", "rd_kafka_conf_set_throttle_cb", 7),
    ("librdkafka/client.qv", "rd_kafka_conf_set_log_cb", 7),
    ("librdkafka/client.qv", "rd_kafka_conf_set_stats_cb", 7),
    (
        "librdkafka/client.qv",
        "rd_kafka_conf_set_oauthbearer_token_refresh_cb",
        7,
    ),
    ("librdkafka/client.qv", "rd_kafka_conf_set_socket_cb", 7),
    ("librdkafka/client.qv", "rd_kafka_conf_set_connect_cb", 7),
    (
        "librdkafka/client.qv",
        "rd_kafka_conf_set_closesocket_cb",
        7,
    ),
    ("librdkafka/client.qv", "rd_kafka_conf_set_open_cb", 7),
    ("librdkafka/client.qv", "rd_kafka_conf_set_resolve_cb", 7),
    (
        "librdkafka/client.qv",
        "rd_kafka_conf_set_ssl_cert_verify_cb",
        7,
    ),
    ("librdkafka/client.qv", "rd_kafka_conf_new", 8),
    ("librdkafka/client.qv", "rd_kafka_consume_cb", 9),
    (
        "librdkafka/client.qv",
        "rd_kafka_background_queue_serve",
        17,
    ),
    ("librdkafka/client.qv", "rd_kafka_oauthbearer_refresh_op", 7),
    ("librdkafka/client.qv", "rd_kafka_log_print", 6),
    (
        "librdkafka/client.qv",
        "rd_kafka_oauthbearer_unsecured_token",
        6,
    ),
    (
        "librdkafka/client.qv",
        "rd_kafka_oidc_token_jwt_bearer_refresh_cb",
        6,
    ),
    (
        "librdkafka/client.qv",
        "rd_kafka_oidc_token_client_credentials_refresh_cb",
        6,
    ),
    (
        "librdkafka/client.qv",
        "rd_kafka_oidc_token_metadata_azure_imds_refresh_cb",
        6,
    ),
    (
        "librdkafka/client.qv",
        "rd_kafka_oidc_token_metadata_aws_iam_refresh_cb",
        6,
    ),
    // Card SUPPLIED-OWNED-RETURN-CODE-HANDOFF: the two hand-off declarations
    // (C04.19.5): `rd_kafka_poll_cb` (marker, premise, five modes, twelve
    // member invocations, the result, the not-PASS hand-off) and the KEEP-only
    // `rd_kafka_txn_op_send_offsets_to_transaction`.
    ("librdkafka/client.qv", "rd_kafka_poll_cb", 21),
    (
        "librdkafka/client.qv",
        "rd_kafka_txn_op_send_offsets_to_transaction",
        7,
    ),
    // Card OP-CALLBACK-MEMBER-PATH-RELEASE: the eight op-callback candidates
    // of shape (b) (C04.19.6), each the marker, the `rk_no_plugin_library`
    // premise, three modes, the result and its member-path states:
    // `rd_kafka_admin_worker` and the leaders worker beside a KEEP hand-off
    // (four states each), `rd_kafka_share_fetch_fanout_op` with its
    // `share_ack_batches_nonempty` premise, `rd_kafka_share_commit_sync_fanout_op`
    // with three states.
    ("librdkafka/client.qv", "rd_kafka_admin_worker", 11),
    (
        "librdkafka/client.qv",
        "rd_kafka_topic_partition_list_query_leaders_async_worker_op_cb",
        11,
    ),
    ("librdkafka/client.qv", "rd_kafka_admin_fanout_worker", 7),
    (
        "librdkafka/client.qv",
        "rd_kafka_DeleteRecords_leaders_queried_cb",
        7,
    ),
    (
        "librdkafka/client.qv",
        "rd_kafka_ListOffsets_leaders_queried_cb",
        7,
    ),
    (
        "librdkafka/client.qv",
        "rd_kafka_share_commit_async_fanout_op",
        7,
    ),
    ("librdkafka/client.qv", "rd_kafka_share_fetch_fanout_op", 8),
    (
        "librdkafka/client.qv",
        "rd_kafka_share_commit_sync_fanout_op",
        9,
    ),
    // Card W36-CORPUS-CONTRACTS-3: kept in its own module rather than appended
    // to `librdkafka/memory.qv`, so that module's pin stays untouched. Read
    // from `lib/librdkafka-2.15.0/src/rdkafka.c:7131-7133`: `rk` is read
    // nowhere, `ptr` is forwarded to the already-declared `rd_free`.
    ("librdkafka/kafka_mem.qv", "rd_kafka_mem_free", 4),
    // Card W45-CALLEE-CASCADE-RESIDUAL: five librdkafka-internal helpers,
    // marker and borrowed formals; four `none`, the metadata copy an owned
    // nullable result.
    ("librdkafka/kafka_mem.qv", "load_descriptor_values", 3),
    (
        "librdkafka/kafka_mem.qv",
        "rd_kafka_aborted_txns_next_offset",
        6,
    ),
    ("librdkafka/kafka_mem.qv", "rd_kafka_anyconf_desensitize", 4),
    // Card LIBRDKAFKA-PLUGIN-PREMISE-REMAINDER: +1, the dump names `rk_no_plugin_library`.
    ("librdkafka/kafka_mem.qv", "rd_kafka_dump0", 6),
    (
        "librdkafka/kafka_mem.qv",
        "rd_kafka_metadata_copy_internal",
        5,
    ),
    // Card W45-CALLEE-CASCADE-RESIDUAL round 3: two callees whose bodies are Safe in their
    // home units: the hash (marker, two borrowed, `none`) and the calloc
    // wrapper (marker, three borrowed, an owned nullable result).
    ("librdkafka/kafka_mem.qv", "rd_bytes_hash", 4),
    ("librdkafka/kafka_mem.qv", "rd_kafka_mem_calloc", 5),
    // Card W37-CORPUS-CONTRACTS-4: the deprecated in-place LZ4 stream reset,
    // sole blocker of `LZ4_resetStreamState`. ZEROING IS NOT RELEASING — the
    // body is one `MEM_INIT` (memset) through the parameter, and the freeing
    // neighbour `LZ4_freeStream` is a different name already decided through
    // `kafka_mem.qcore` above. Read from
    // `lib/librdkafka-2.15.0/src/lz4.c:1524-1528`.
    ("librdkafka/lz4_stream.qv", "LZ4_resetStream", 3),
    // Card W45-CALLEE-CASCADE-RESIDUAL: three one-shot buffer functions
    // (`lz4hc.c:958-973`, `lz4.c:1799-1896`, `lz4.c:1440-1443`): marker,
    // borrowed formals, `none`.
    ("librdkafka/lz4_stream.qv", "LZ4_compress_HC", 7),
    (
        "librdkafka/lz4_stream.qv",
        "LZ4_decompress_unsafe_generic",
        8,
    ),
    // Card W43-ABI-ENUM-AND-CONST-PP: the generic checked decoder
    // (`lz4.c:1942-2344`, two enumeration directives, rule 12u): marker,
    // nine borrowed formals, `none`.
    ("librdkafka/lz4_stream.qv", "LZ4_decompress_generic", 11),
    ("librdkafka/lz4_stream.qv", "LZ4_compress_default", 6),
    // Card CALLEE-RETENTION-CLASS1, stage (b): the retaining stream states,
    // marker, borrowed formals, the retention (and for `_extState` the
    // holder reset), `none`.
    ("librdkafka/lz4_stream.qv", "LZ4_compress_HC_extStateHC", 9),
    ("librdkafka/lz4_stream.qv", "LZ4_compress_HC_continue", 8),
    ("librdkafka/lz4_stream.qv", "LZ4_compress_fast_continue", 9),
    ("librdkafka/lz4_stream.qv", "LZ4_compress_fast_extState", 10),
    ("librdkafka/memory.qv", "rd_malloc", 3),
    ("librdkafka/memory.qv", "rd_calloc", 4),
    ("librdkafka/memory.qv", "rd_strdup", 3),
    ("librdkafka/memory.qv", "rd_strndup", 4),
    ("librdkafka/memory.qv", "rd_free", 3),
    ("librdkafka/memory.qv", "rd_realloc", 4),
    // Card W46-EXTERNAL-CALLEE-CONTRACTS: the atomic counter read
    // (`src/rdatomic.h:100-112`): marker, borrowed counter, `none`.
    ("librdkafka/memory.qv", "rd_atomic32_get", 3),
    // Card W47-CALLEE-CLOSURE-R2: the whole-file reader
    // (`src/rd.h:472-521`): marker, three borrowed arguments, and a
    // nullable owned `rd_malloc` buffer.
    ("librdkafka/memory.qv", "rd_file_read", 5),
    // Card CONTRACT-NORETURN-AND-BORROWED-TAILS: `rd_kafka_assert`'s failure call
    // (`src/rdkafka.c:6640-6649`): marker, five borrowed arguments, `none`
    // and the no-return exit.
    // Card LIBRDKAFKA-PLUGIN-PREMISE-REMAINDER: +1, the crash path names `rk_no_plugin_library`.
    ("librdkafka/memory.qv", "rd_kafka_crash", 9),
    // Card CG10-INDIRECT-ACCESS-SOLE-RESIDUALS: custody-neutral scalar
    // helpers (`src/rdatomic.h`, `src/tinycthread_extra.c:92-129`,
    // `src/rdhdrhistogram.c:391-407`): marker, borrowed formals, `none`.
    ("librdkafka/memory.qv", "cnd_timedwait_ms", 5),
    ("librdkafka/memory.qv", "rd_hdr_histogram_record", 4),
    ("librdkafka/memory.qv", "rd_atomic64_add", 4),
    ("librdkafka/memory.qv", "rd_atomic64_get", 3),
    // Card NULL-SAFETY-PROPERTY: `quant null-deref` checks (bounds
    // contract v1, the crate's bounds view): selector and one check each.
    ("librdkafka/null_deref.qv", "rd_kafka_destroy", 2),
    (
        "librdkafka/null_deref.qv",
        "rd_kafka_topic_partition_list_destroy",
        2,
    ),
    (
        "librdkafka/null_deref.qv",
        "rd_kafka_topic_partition_list_add",
        2,
    ),
    ("librdkafka/null_deref.qv", "rd_list_destroy", 2),
    ("librdkafka/null_deref.qv", "rd_list_add", 2),
    ("librdkafka/null_deref.qv", "rd_kafka_queue_destroy", 2),
    ("librdkafka/null_deref.qv", "rd_kafka_message_destroy", 2),
    // Card NULL-CANDIDATE-TRIAGE (rules 14t-14u): non-null members (each the
    // selector and one member clause, two on the topic constructor), a
    // non-null result, and seven more `quant null-deref` checks.
    // Card NONNULL-MEMBER-UNIFIED-QUALIFIER (USER DECISION 2026-10-16, "Yes,
    // all kinds"): +1 on each of the four plain member declarations, which
    // name `rk_no_plugin_library` (2 -> 3, 3 -> 4, 2 -> 3, 2 -> 3).
    ("librdkafka/null_deref.qv", "rd_kafka_toppar_new0", 3),
    ("librdkafka/null_deref.qv", "rd_kafka_topic_new0", 4),
    ("librdkafka/null_deref.qv", "rd_kafka_cgrp_new", 3),
    ("librdkafka/null_deref.qv", "rd_kafka_transport_new", 3),
    ("librdkafka/null_deref.qv", "rd_kafka_topic_keep", 2),
    ("librdkafka/null_deref.qv", "rd_kafka_flush", 2),
    ("librdkafka/null_deref.qv", "rd_kafka_message_errstr", 2),
    ("librdkafka/null_deref.qv", "rd_kafka_queue_poll", 2),
    ("librdkafka/null_deref.qv", "rd_kafka_event_error", 2),
    ("librdkafka/null_deref.qv", "rd_kafka_outq_len", 2),
    ("librdkafka/null_deref.qv", "rd_kafka_op_destroy", 2),
    ("librdkafka/null_deref.qv", "rd_kafka_topic_name", 2),
    // Card NULL-MEMBER-INVARIANT-SCOPE (rule 14x): a stable member, hosted
    // on the broker's constructor (the selector and one stable clause).
    // Card LIBRDKAFKA-PLUGIN-PREMISE-REMAINDER: +1 on each of the 67 premised
    // stable and scope member declarations (`rk_no_plugin_library`; spelled
    // `nonnull_member(...) where quant #scope [[stable]]` / `[[scope]]` since
    // card NONNULL-MEMBER-UNIFIED-QUALIFIER, counts unchanged).
    ("librdkafka/null_deref.qv", "rd_kafka_broker_add", 3),
    // Card NULL-MEMBER-INVARIANT-SCOPE (rule 14z): sixty-six function-scope
    // entries of the registered broker (the selector and one scope clause).
    (
        "librdkafka/null_deref.qv",
        "rd_kafka_ShareAcknowledgeRequest",
        3,
    ),
    (
        "librdkafka/null_deref.qv",
        "rd_kafka_broker_active_toppar_add",
        3,
    ),
    (
        "librdkafka/null_deref.qv",
        "rd_kafka_broker_active_toppar_del",
        3,
    ),
    ("librdkafka/null_deref.qv", "rd_kafka_broker_buf_enq0", 3),
    ("librdkafka/null_deref.qv", "rd_kafka_broker_buf_retry", 3),
    (
        "librdkafka/null_deref.qv",
        "rd_kafka_broker_bufq_timeout_scan",
        3,
    ),
    ("librdkafka/null_deref.qv", "rd_kafka_broker_conn_closed", 3),
    ("librdkafka/null_deref.qv", "rd_kafka_broker_connect", 3),
    (
        "librdkafka/null_deref.qv",
        "rd_kafka_broker_connect_auth",
        3,
    ),
    (
        "librdkafka/null_deref.qv",
        "rd_kafka_broker_connect_done",
        3,
    ),
    ("librdkafka/null_deref.qv", "rd_kafka_broker_connect_up", 3),
    (
        "librdkafka/null_deref.qv",
        "rd_kafka_broker_consumer_serve",
        3,
    ),
    (
        "librdkafka/null_deref.qv",
        "rd_kafka_broker_destroy_final",
        3,
    ),
    (
        "librdkafka/null_deref.qv",
        "rd_kafka_broker_feature_disable",
        3,
    ),
    (
        "librdkafka/null_deref.qv",
        "rd_kafka_broker_feature_enable",
        3,
    ),
    (
        "librdkafka/null_deref.qv",
        "rd_kafka_broker_features_set",
        3,
    ),
    (
        "librdkafka/null_deref.qv",
        "rd_kafka_broker_fetch_backoff",
        3,
    ),
    (
        "librdkafka/null_deref.qv",
        "rd_kafka_broker_fetch_toppars",
        3,
    ),
    (
        "librdkafka/null_deref.qv",
        "rd_kafka_broker_handle_purge_queues",
        3,
    ),
    ("librdkafka/null_deref.qv", "rd_kafka_broker_idle_check", 3),
    (
        "librdkafka/null_deref.qv",
        "rd_kafka_broker_internal_serve",
        3,
    ),
    (
        "librdkafka/null_deref.qv",
        "rd_kafka_broker_needs_connection",
        3,
    ),
    (
        "librdkafka/null_deref.qv",
        "rd_kafka_broker_produce_toppars",
        3,
    ),
    ("librdkafka/null_deref.qv", "rd_kafka_broker_resolve", 3),
    (
        "librdkafka/null_deref.qv",
        "rd_kafka_broker_retry_bufs_move",
        3,
    ),
    ("librdkafka/null_deref.qv", "rd_kafka_broker_serve", 3),
    (
        "librdkafka/null_deref.qv",
        "rd_kafka_broker_session_add_partition_to_toppars_in_session",
        3,
    ),
    (
        "librdkafka/null_deref.qv",
        "rd_kafka_broker_session_remove_partition_from_toppars_in_session",
        3,
    ),
    (
        "librdkafka/null_deref.qv",
        "rd_kafka_broker_session_reset",
        3,
    ),
    (
        "librdkafka/null_deref.qv",
        "rd_kafka_broker_session_update_epoch",
        3,
    ),
    (
        "librdkafka/null_deref.qv",
        "rd_kafka_broker_session_update_partitions",
        3,
    ),
    (
        "librdkafka/null_deref.qv",
        "rd_kafka_broker_set_api_versions",
        3,
    ),
    (
        "librdkafka/null_deref.qv",
        "rd_kafka_broker_set_nodename",
        3,
    ),
    ("librdkafka/null_deref.qv", "rd_kafka_broker_set_state", 3),
    (
        "librdkafka/null_deref.qv",
        "rd_kafka_broker_share_fetch_session_clear",
        3,
    ),
    (
        "librdkafka/null_deref.qv",
        "rd_kafka_broker_share_fetch_session_leave",
        3,
    ),
    ("librdkafka/null_deref.qv", "rd_kafka_broker_share_rpc", 3),
    (
        "librdkafka/null_deref.qv",
        "rd_kafka_broker_share_session_toppar_add",
        3,
    ),
    (
        "librdkafka/null_deref.qv",
        "rd_kafka_broker_timeout_scan",
        3,
    ),
    (
        "librdkafka/null_deref.qv",
        "rd_kafka_broker_toppar_msgq_scan",
        3,
    ),
    ("librdkafka/null_deref.qv", "rd_kafka_broker_wakeup", 3),
    ("librdkafka/null_deref.qv", "rd_kafka_buf_new_request0", 3),
    ("librdkafka/null_deref.qv", "rd_kafka_buf_retry", 3),
    (
        "librdkafka/null_deref.qv",
        "rd_kafka_bufq_connection_reset",
        3,
    ),
    ("librdkafka/null_deref.qv", "rd_kafka_bufq_dump", 3),
    ("librdkafka/null_deref.qv", "rd_kafka_bufq_purge", 3),
    ("librdkafka/null_deref.qv", "rd_kafka_dr_implicit_ack", 3),
    ("librdkafka/null_deref.qv", "rd_kafka_features_check", 3),
    (
        "librdkafka/null_deref.qv",
        "rd_kafka_fetch_reply_handle_partition_error",
        3,
    ),
    (
        "librdkafka/null_deref.qv",
        "rd_kafka_group_MemberMetadata_consumer_read",
        3,
    ),
    (
        "librdkafka/null_deref.qv",
        "rd_kafka_handle_Produce_metadata_update",
        3,
    ),
    (
        "librdkafka/null_deref.qv",
        "rd_kafka_idemp_request_pid_failed",
        3,
    ),
    (
        "librdkafka/null_deref.qv",
        "rd_kafka_msgbatch_handle_Produce_result",
        3,
    ),
    ("librdkafka/null_deref.qv", "rd_kafka_op_throttle_time", 3),
    (
        "librdkafka/null_deref.qv",
        "rd_kafka_parse_Metadata_update_topic",
        3,
    ),
    ("librdkafka/null_deref.qv", "rd_kafka_recv", 3),
    ("librdkafka/null_deref.qv", "rd_kafka_req_response", 3),
    ("librdkafka/null_deref.qv", "rd_kafka_sasl_broker_init", 3),
    ("librdkafka/null_deref.qv", "rd_kafka_sasl_broker_term", 3),
    ("librdkafka/null_deref.qv", "rd_kafka_send", 3),
    ("librdkafka/null_deref.qv", "rd_kafka_ssl_hmac", 3),
    (
        "librdkafka/null_deref.qv",
        "rd_kafka_toppar_fetch_backoff",
        3,
    ),
    ("librdkafka/null_deref.qv", "rd_kafka_transport_connect", 3),
    (
        "librdkafka/null_deref.qv",
        "rd_kafka_transport_ssl_connect",
        3,
    ),
    ("librdkafka/null_deref.qv", "rd_kafka_waitresp_find", 3),
    (
        "librdkafka/null_deref.qv",
        "rd_kafkap_share_leader_changes_apply",
        3,
    ),
    // RK-PLUGIN-LIBRARY-OUTER-ENVELOPE: selector, definition, runtime guard,
    // 34 callback registrations, 12 kept members and 8 premises.
    ("librdkafka/outer.qv", "rd_kafka_plugin_new", 85),
    // Card LIBRDKAFKA-PLUGIN-PREMISE-REMAINDER: 6 -> 7, the model contract names
    // `rk_no_plugin_library` (user decision 2026-10-15, "Add premise support").
    (
        "librdkafka/queue_destructor.qv",
        "rd_kafka_q_destroy_final",
        7,
    ),
    ("librdkafka/refcount.qv", "rd_refcnt_add0", 4),
    ("librdkafka/refcount.qv", "rd_refcnt_sub0", 4),
    ("librdkafka/refcount.qv", "rd_refcnt_destroy_final", 3),
    // Card W37-CORPUS-CONTRACTS-4: the in-place ordering of a topic-partition
    // list — this card's HIGHEST function-cover yield at two projected
    // functions (`..._sort_by_topic` and `..._sort_by_topic_id`). Sorting
    // permutes an array the caller still owns whole, so all three formals are
    // borrowed; `cmp` is `fnptr`, the spelling `msgpack/packer.qv` already
    // uses for `msgpack_packer_init`'s callback. Read from
    // `lib/librdkafka-2.15.0/src/rdkafka_partition.c:3664-3674`. Its
    // higher-yield unit-mate `rd_kafka_topic_partition_destroy0` was read and
    // DECLINED on two grounds: `static` with no header declaration in the very
    // unit that calls it, and a CONDITIONAL release (`if (do_free)`) whose
    // other five sites in that unit pass 0.
    // CG22-CALLBACK-REGISTRY-CONTRACT: re-declared, 5 -> 6 clauses (the
    // comparator's synchronous invocation).
    (
        "librdkafka/topic_partition.qv",
        "rd_kafka_topic_partition_list_sort",
        6,
    ),
    ("librdkafka/typed_queue.qv", "rd_kafka_q_enq0", 5),
    ("librdkafka/typed_queue.qv", "rd_kafka_q_deq0", 5),
    ("monkey/callback_binding.qv", "binding_scope", 8),
    ("monkey/callback_binding.qv", "binding_memory", 3),
    ("monkey/callback_binding.qv", "binding_execution", 3),
    ("monkey/callback_binding.qv", "binding_select_path", 1),
    ("monkey/callback_binding.qv", "binding_send", 4),
    ("monkey/callback_binding.qv", "binding_errno_location", 2),
    ("monkey/callback_binding.qv", "binding_perror", 1),
    (
        "monkey/callback_binding.qv",
        "binding_evutil_closesocket",
        1,
    ),
    ("monkey/callback_binding.qv", "binding_event_del", 1),
    ("monkey/callback_binding.qv", "binding_event_free", 1),
    ("monkey/callback_binding.qv", "binding_mk_mem_free", 3),
    ("monkey/callback_binding.qv", "binding_symbol", 2),
    ("monkey/callback_binding.qv", "binding_call_arguments", 2),
    ("monkey/callback_binding.qv", "binding_call_interval", 3),
    ("monkey/callback_binding.qv", "binding_runtime_effect", 2),
    ("monkey/callback_task_handoff.qv", "selected_callbacks", 4),
    (
        "monkey/callback_task_handoff.qv",
        "selected_callback_identity",
        6,
    ),
    (
        "monkey/callback_task_handoff.qv",
        "effect_errno_location",
        3,
    ),
    ("monkey/callback_task_handoff.qv", "effect_event_del", 3),
    ("monkey/callback_task_handoff.qv", "effect_event_free", 3),
    (
        "monkey/callback_task_handoff.qv",
        "effect_evutil_closesocket",
        3,
    ),
    ("monkey/callback_task_handoff.qv", "effect_mk_mem_free", 3),
    ("monkey/callback_task_handoff.qv", "effect_perror", 3),
    ("monkey/callback_task_handoff.qv", "effect_send", 3),
    (
        "monkey/callback_task_handoff.qv",
        "bounded_no_task_observation",
        7,
    ),
    (
        "monkey/callback_task_handoff.qv",
        "taskful_fixture_source",
        16,
    ),
    (
        "monkey/callback_task_handoff.qv",
        "taskful_fixture_primitives",
        12,
    ),
    ("monkey/composition.qv", "evutil_socketpair", 3),
    ("monkey/composition.qv", "evutil_socket_geterror", 2),
    // Deliberate pin update (card O03-U5): the superseded
    // `descriptor_custody_is_preserved` error clause asserted a live, reusable
    // descriptor for an operation that is `close(2)` on the audited build, and
    // is replaced by four clauses stating the unresolved endpoint, the reuse
    // and retry prohibitions and the platform error channel, plus one `any`
    // clause separating durability. 4 -> 8.
    ("monkey/composition.qv", "evutil_closesocket", 8),
    ("monkey/composition.qv", "cb_event", 3),
    ("monkey/composition.qv", "cb_timeout", 4),
    ("monkey/composition.qv", "cb_wait_2_timeout", 2),
    ("monkey/composition.qv", "check_monkey_composition", 13),
    // Deliberate Cycle 18 executable policy pins.
    ("monkey/cycle18_composition.qv", "callback_inventory", 18),
    ("monkey/cycle18_composition.qv", "context_identity_join", 6),
    (
        "monkey/cycle18_composition.qv",
        "wait_failure_source_join",
        2,
    ),
    ("monkey/cycle18_composition.qv", "evidence_join", 41),
    ("monkey/cycle18_composition.qv", "caller_size_bound", 8),
    ("monkey/cycle18_composition.qv", "aggregate_acceptance", 11),
    (
        "monkey/descriptor_observation.qv",
        "observation_socketpair",
        4,
    ),
    ("monkey/descriptor_observation.qv", "observation_acquire", 4),
    ("monkey/descriptor_observation.qv", "observation_use", 1),
    ("monkey/descriptor_observation.qv", "observation_close", 2),
    (
        "monkey/descriptor_observation.qv",
        "observation_complete",
        8,
    ),
    // Card FB-EVENT-LOOP-RETENTION-AND-REGISTRATION: Monkey's event-loop
    // operations (retentions, removals of one registration, the descriptor
    // premise).
    ("monkey/event_loop.qv", "mk_event_add", 8),
    ("monkey/event_loop.qv", "mk_event_timeout_create", 8),
    ("monkey/event_loop.qv", "mk_event_channel_create", 8),
    ("monkey/event_loop.qv", "mk_event_del", 5),
    ("monkey/event_loop.qv", "mk_event_timeout_destroy", 6),
    ("monkey/event_loop.qv", "mk_event_channel_destroy", 7),
    ("monkey/event_loop.qv", "mk_event_wait", 3),
    // Card RETENTION-UNMATCHED-REMOVAL-POISONS: the loop's create and destroy
    // (a destroy ends every record the loop holds, rule 12t).
    ("monkey/event_loop.qv", "mk_event_loop_create", 3),
    ("monkey/event_loop.qv", "mk_event_loop_destroy", 3),
    // Card CONSTRUCTOR-RESIDUALS, continued. Four clauses: the marker, two
    // borrowed pointers and the unit result.
    ("monkey/event_payload.qv", "cb_event_source_projection", 12),
    ("monkey/event_payload.qv", "contextual_payload_boundary", 1),
    ("monkey/event_payload.qv", "contextual_authority", 13),
    ("monkey/event_payload.qv", "native_allocation", 8),
    ("monkey/event_payload.qv", "contextual_payload_access", 13),
    // Card W45-FB-RETRY-LIFETIME: `resource_published` / `resource_unpublished`.
    ("monkey/intrusive_list.qv", "mk_list_add", 5),
    ("monkey/intrusive_list.qv", "mk_list_del", 4),
    // Card W45-CALLEE-CASCADE-RESIDUAL round 3: the FIFO queue destructor: marker,
    // borrowed `ctx`, released `q`, `none`.
    ("monkey/intrusive_list.qv", "mk_fifo_queue_destroy", 4),
    // Card W36-CORPUS-CONTRACTS-3: this card's ONE variadic declaration, the
    // `fluent_bit/logging.qv` shape one library over — five clauses because
    // of `resource_variadic_data_tail()`. Read from
    // `lib/monkey/mk_core/mk_utils.c:200-264`: the tail is consumed by
    // `vprintf` into `stdout` and no pointer to it outlives the call.
    ("monkey/logging.qv", "mk_print", 5),
    // Card W46-EXTERNAL-CALLEE-CONTRACTS: the errno reporter
    // (`mk_core/mk_utils.h:84-96`): marker, three borrowed, `none`.
    ("monkey/logging.qv", "mk_utils_libc_error", 5),
    ("monkey/memory.qv", "mk_mem_alloc", 3),
    ("monkey/memory.qv", "mk_mem_alloc_z", 3),
    ("monkey/memory.qv", "mk_mem_free", 3),
    // Deliberate Cycle 18 executable policy pins.
    (
        "monkey/native_platform_matrix.qv",
        "native_platform_source_identity",
        3,
    ),
    (
        "monkey/native_platform_matrix.qv",
        "linux_epoll_asan_profile",
        7,
    ),
    (
        "monkey/native_platform_matrix.qv",
        "linux_poll_asan_profile",
        7,
    ),
    (
        "monkey/native_platform_matrix.qv",
        "linux_select_asan_profile",
        7,
    ),
    (
        "monkey/native_platform_matrix.qv",
        "macos_kqueue_asan_profile",
        7,
    ),
    (
        "monkey/native_platform_matrix.qv",
        "windows_win32_asan_profile",
        7,
    ),
    (
        "monkey/native_platform_matrix.qv",
        "windows_iocp_asan_profile",
        7,
    ),
    (
        "monkey/native_platform_matrix.qv",
        "linux_epoll_tsan_profile",
        7,
    ),
    ("monkey/native_platform_matrix.qv", "observed_row", 14),
    ("monkey/native_platform_matrix.qv", "unknown_row", 4),
    // W46-LIBC-EVENT-CONTRACT-GAPS: pinned, scalar-only observations.
    ("monkey/scalar_observation.qv", "_mk_string_search", 6),
    ("monkey/scalar_observation.qv", "mk_string_char_search", 5),
    ("monkey/scalar_observation.qv", "mk_string_char_search_r", 5),
    ("monkey/scalar_observation.qv", "MK_EVENT_NEW", 3),
    ("monkey/scalar_observation.qv", "event_base_get_features", 3),
    (
        "monkey/scalar_observation.qv",
        "event_get_struct_event_size",
        2,
    ),
    (
        "monkey/scalar_observation.qv",
        "event_get_version_number",
        2,
    ),
    // MONKEY-VHOST-PROPERTY-CUSTODY: `mk_string_dup`, pinned.
    ("monkey/string.qv", "mk_string_dup", 3),
    // Card CONTRACT-VIEW-PROJECTION-CHECK removed this module's
    // `source_identity` (4 clauses), a textual duplicate of
    // `monkey/wait_source_policy.qv`'s, which is now its single owner.
    ("monkey/timeout_teardown.qv", "cb_timeout_source", 7),
    ("monkey/timeout_teardown.qv", "cb_timeout_teardown_path", 1),
    ("monkey/timeout_teardown.qv", "libevent_self_teardown", 5),
    ("monkey/timeout_teardown.qv", "teardown_obligations", 21),
    // CONTRACT-PIN-TIP-MODE: SUP04's fixed `mk_vhost_set`, its lock entry a
    // tip identity (contract spec C06.2).
    ("monkey/vhost_set.qv", "mk_vhost_set", 5),
    ("monkey/wait_failure_policy.qv", "loop_init_failure", 6),
    (
        "monkey/wait_failure_policy.qv",
        "context_allocation_failure",
        7,
    ),
    ("monkey/wait_failure_policy.qv", "base_creation_failure", 8),
    (
        "monkey/wait_failure_policy.qv",
        "fired_allocation_failure",
        10,
    ),
    ("monkey/wait_failure_policy.qv", "timer_creation_failure", 9),
    (
        "monkey/wait_failure_policy.qv",
        "timer_registration_failure",
        10,
    ),
    ("monkey/wait_failure_policy.qv", "success_observation", 10),
    ("monkey/wait_failure_policy.qv", "loop_size_bounds", 6),
    (
        "monkey/wait_identity_policy.qv",
        "wait_identity_context",
        23,
    ),
    ("monkey/wait_identity_policy.qv", "wait_identity_entry", 1),
    ("monkey/wait_identity_policy.qv", "wait_observed_profile", 5),
    ("monkey/wait_source_policy.qv", "source_identity", 4),
    (
        "monkey/wait_source_policy.qv",
        "wait_projection_identity",
        4,
    ),
    ("monkey/wait_source_policy.qv", "cb_wait_2_timeout", 9),
    (
        "monkey/wait_source_policy.qv",
        "contract_cb_wait_2_timeout",
        9,
    ),
    ("monkey/wait_source_policy.qv", "_mk_event_loop_create", 9),
    (
        "monkey/wait_source_policy.qv",
        "contract__mk_event_loop_create",
        9,
    ),
    (
        "monkey/wait_source_policy.qv",
        "_mk_event_wait_with_flags",
        9,
    ),
    (
        "monkey/wait_source_policy.qv",
        "contract__mk_event_wait_with_flags",
        9,
    ),
    ("monkey/wait_source_policy.qv", "_mk_event_wait_2", 9),
    (
        "monkey/wait_source_policy.qv",
        "contract__mk_event_wait_2",
        9,
    ),
    ("monkey/wait_source_policy.qv", "_mk_event_loop_destroy", 9),
    (
        "monkey/wait_source_policy.qv",
        "contract__mk_event_loop_destroy",
        9,
    ),
    ("monkey/wait_source_policy.qv", "artifact_set", 4),
    ("monkey/wrapper_paths.qv", "mk_event_libevent_socketpair", 3),
    ("monkey/wrapper_paths.qv", "_mk_event_init", 3),
    ("monkey/wrapper_paths.qv", "_mk_event_loop_create", 5),
    ("monkey/wrapper_paths.qv", "_mk_event_loop_destroy", 4),
    ("monkey/wrapper_paths.qv", "_mk_event_update", 5),
    ("monkey/wrapper_paths.qv", "_mk_event_add", 7),
    ("monkey/wrapper_paths.qv", "_mk_event_del", 7),
    ("monkey/wrapper_paths.qv", "_mk_event_timeout_create", 7),
    ("monkey/wrapper_paths.qv", "_mk_event_timeout_destroy", 4),
    ("monkey/wrapper_paths.qv", "_mk_event_channel_create", 5),
    ("monkey/wrapper_paths.qv", "_mk_event_channel_destroy", 4),
    ("monkey/wrapper_paths.qv", "_mk_event_inject", 6),
    ("monkey/wrapper_paths.qv", "_mk_event_wait_with_flags", 4),
    ("monkey/wrapper_paths.qv", "_mk_event_wait_2", 5),
    ("monkey/wrapper_paths.qv", "_mk_event_backend", 3),
    // Card W37-CORPUS-CONTRACTS-4: the corpus's first `mpack` family, and it is
    // NOT the `msgpack/` directly above — `msgpack_packer_init` belongs to
    // msgpack-c, `mpack_write_*` to mpack, two different libraries vendored
    // side by side in the audited tree that share only a naming resemblance.
    // Each of the four is the sole blocker of exactly one projected function
    // (`pack_cmetrics_type`, `pack_string`, `pack_int64`, `pack_double`).
    //
    // Each name has FOUR definitions in the tree (the amalgamation copy plus
    // the cmetrics, ctraces and cprofiles vendored copies), which is the
    // `ADD-NAME-COLLISION` hazard's shape but NOT its substance: all four
    // copies spell each signature identically with external linkage and a
    // matching header prototype, so they are four copies of one library at one
    // version rather than four functions sharing a name. The distinction that
    // matters is not how many definitions but whether they AGREE — compared,
    // not assumed. None of the four `.c` copies is an analysed unit, so
    // constraint 8's duty gate has no summary to compare against.
    //
    // Read from `lib/cmetrics/lib/mpack/src/mpack/mpack.c` at `:2045` (cstr),
    // `:1968` (str), `:1774` (i64) and `:1816` (double). All custody-neutral:
    // the writer is written through and never released, the byte sources are
    // read and never retained, the scalars pass by value, every result `void`.
    ("mpack/writer.qv", "mpack_write_cstr", 4),
    ("mpack/writer.qv", "mpack_write_str", 5),
    ("mpack/writer.qv", "mpack_write_i64", 4),
    ("mpack/writer.qv", "mpack_write_double", 4),
    // Card W46-EXTERNAL-CALLEE-CONTRACTS: five more writer primitives, each
    // the marker, its borrowed parameters and `none`.
    ("mpack/writer.qv", "mpack_finish_map", 3),
    ("mpack/writer.qv", "mpack_finish_array", 3),
    ("mpack/writer.qv", "mpack_write_true", 3),
    ("mpack/writer.qv", "mpack_write_false", 3),
    ("mpack/writer.qv", "mpack_write_bin", 5),
    // Card ABI-TYPEDEF-ADDRESS-OF-MEMBER. Five clauses: the marker, three
    // borrowed parameters and the `unit` result. The third parameter is the
    // corpus's first `fnptr`, on which `resource_borrowed` is both the only
    // mode the loader admits (`AGENTS.md` §L21 / spec §C04.1) and the true one,
    // a function designator never being a resource identity. Card
    // CONSTRUCTOR-RESIDUALS-ABI-TYPEDEF wrote this declaration and deliberately
    // did not land it at measured payoff zero; this card repaired the blocker
    // it named and measured the payoff, so it lands.
    // CG22-CALLBACK-REGISTRY-CONTRACT: re-declared, 5 -> 7 clauses (the
    // retention of `data` and the registration of `callback` in `pk`).
    ("msgpack/packer.qv", "msgpack_packer_init", 7),
];

#[path = "support/contract_corpus.rs"]
mod contract_corpus;

#[test]
fn every_qcore_contract_source_parses_round_trips_and_compiles() {
    let sources = contract_corpus::sources();
    // 60 -> 61 by card HASH-CONTRACTS' deliberate addition of
    // `fluent_bit/hash_table_publication.qv`; 61 -> 64 by card
    // PROD-CONTRACTS' deliberate addition of
    // `fluent_bit/input_chunk_name.qv`, `fluent_bit/logging.qv` and
    // `fluent_bit/routes_mask.qv`; 64 -> 67 by card NEIGHBOURS' deliberate
    // addition of `chunkio/chunk_residency.qv`,
    // `fluent_bit/input_chunk_helpers.qv` and `fluent_bit/time.qv`;
    // 67 -> 69 by card CONSTRUCTOR-RESIDUALS' deliberate addition of
    // `libc/errno.qv` and `monkey/intrusive_list.qv`; 69 -> 70 by card
    // CONSTRUCTOR-RESIDUALS-ABI-TYPEDEF's deliberate addition of
    // `libc/pthread_mutex.qv`. That card read `msgpack/packer.qv`
    // against its body too and deliberately did NOT land it: measured payoff
    // zero, because the address-of-a-member actual refuses every one of its
    // call sites before any contract is consulted. 70 -> 71 by card
    // ABI-TYPEDEF-ADDRESS-OF-MEMBER, which repaired exactly that blocker and
    // then measured the declaration's payoff on the production constructor, so
    // `msgpack/packer.qv` lands under the unchanged PROD-CONTRACTS rule.
    // 71 -> 72 by card ADDRESS-OF-MEMBER-DESTROY-UNDECLARED's deliberate
    // addition of `fluent_bit/input_chunk_destroy.qv`, the declaration that
    // decides the leak leaf the previous card opened, landed on the same rule
    // after its payoff was measured. 72 -> 73 by card AOM-DESTROY-SIBLINGS'
    // deliberate addition of
    // `fluent_bit/input_chunk_destroy_direct_routes.qv`, which decides one
    // of the two siblings that card left undeclared and declines the other at
    // measured payoff zero. 73 -> 74 by card W29-CORPUS-CONTRACTS' deliberate
    // addition of `fluent_bit/input_chunk_sizes.qv`, whose two read-only
    // size queries decide the thirteen standing uncontracted-call refusals
    // that named them inside the unit that defines them. 74 -> 76 by card
    // W31-CORPUS-CONTRACTS-2's deliberate addition of
    // `chunkio/chunk_sizes.qv` and
    // `fluent_bit/input_chunk_total_size.qv`, the two size-sibling
    // declarations W29's named non-declarations pointed at, each read against
    // its own pinned body. 76 -> 82 by card W36-CORPUS-CONTRACTS-3's
    // deliberate addition of six release/custody-neutral declarations, chosen
    // by FUNCTION-COVER yield over the 193 baseline functions whose only
    // refusals are `call to uncontracted function` leaves:
    // `cfl/sds_lifetime.qv` (`cfl_sds_destroy`, 8 such functions),
    // `librdkafka/kafka_mem.qv` (`rd_kafka_mem_free`, 4),
    // `cmetrics/context_lifetime.qv` (`cmt_destroy`, 3),
    // `librdkafka/cjson.qv` (`cJSON_Delete`, 3),
    // `monkey/logging.qv` (`mk_print`, 3, this card's one variadic) and
    // `ctraces/context_lifetime.qv` (`ctr_destroy`, 2) — the last opening
    // the corpus's first `ctraces` family. Each was read against its own pinned
    // body before it was declared; the card also read `add`, `memcpy` and
    // `memset` and deliberately did NOT land them, with the readings published
    // in `docs/reports/w36-corpus-contracts-3/README.md`.
    // 82 -> 88 by card W37-CORPUS-CONTRACTS-4's deliberate addition of six
    // custody-NEUTRAL modules, chosen by FUNCTION-COVER yield over the 163
    // baseline functions whose only refusals are `call to uncontracted
    // function` leaves: `librdkafka/topic_partition.qv`
    // (`rd_kafka_topic_partition_list_sort`, 2 such functions),
    // `mpack/writer.qv` (four writer primitives, 1 function each, and the
    // corpus's first `mpack` family — distinct from the existing `msgpack/`,
    // which is a different library), `cmetrics/metric_values.qv` (2),
    // `cmetrics/mpack_utils.qv` (1), `cfl/time.qv` (1) and
    // `librdkafka/lz4_stream.qv` (1). Not one is a release: after W36
    // harvested the release-shaped callees the remaining cover frontier is
    // dominated by unit-internal `static` helpers a name-keyed declaration may
    // not address at all, so the card also READ and deliberately did NOT land
    // `rd_kafka_topic_partition_destroy0`, `rd_kafka_destroy_app`,
    // `sds_cat_safe`, `snprintf`, `sprintf`, `XXH32`, `cmt_atomic_initialize`
    // and the `set_result` / `otlp_kvpair_list_destroy` /
    // `destroy_context_list` families, with every reading published in
    // `docs/reports/w37-corpus-contracts-4/README.md`.
    // 93 -> 96: Cycle 18 aggregate, platform and task-handoff executable policies.
    // 96 -> 107 by card W43-LIBC-CONTRACTS' deliberate addition of eleven
    // libc/POSIX modules in `providers/quirin-provider-libc` (`libc/dlfcn`,
    // `math`, `process`, `socket`, `stdio`, `stdlib`, `string`, `sys_stat`,
    // `threads`, `time`, `unistd`): 55 trusted declarations read from the
    // prototypes the pinned units declare. The card also read and deliberately
    // did NOT land the alias-returning (`memcpy`, `strchr`, ...), no-return,
    // lifecycle and `__restrict`-withdrawn prototypes; every reading is in
    // `docs/reports/w43-libc-contracts/README.md`.
    // 107 -> 108 by card CONTRACT-INHERIT-ADOPTION's
    // `fluent_bit/resource_string.qv` (`flb_strdup`, `flb_strndup`
    // inheriting libc). Its conversion of `fluent_bit/resource_memory.qv`
    // to `contract inherit` changes no module count.
    // 108 -> 109 by card W43-LIBC-RESTRICT-PROTOTYPES' `libc/regex.qv`
    // (`regexec`, `regerror`); its eight other declarations are appended to
    // existing libc modules.
    // 109 -> 110 by card W43-DESCRIPTOR-RESOURCE-TRACKING's `libc/dirent.qv`.
    // 110 -> 111 by card W43-DESCRIPTOR-NATIVE-CHECK's
    // `library/descriptor_native_observation.qv` in
    // `providers/quirin-provider-shared`: an observation policy (ten
    // parameterless rules), not a libc API contract.
    // Card W44-LIBC-RESIDUALS adds no module: its `strerror`, `vsnprintf` and
    // `vprintf` are appended to existing libc modules.
    // 111 -> 113 by card W45-FB-RETRY-LIFETIME's `fluent_bit/task_users.qv`
    // and `fluent_bit/output_singleplex.qv`.
    // 113 -> 114 by card W45-FB-BACKLOG-CUSTODY's
    // `fluent_bit/storage_backlog.qv`: twelve checked specifications of the
    // storage-backlog bodies (contract spec §C06.3).
    // Combined W46 cards plus six metric getters and five routing queries:
    // 54 selected modules, 230 resource declarations, 118 corpus modules.
    // 118 -> 119 by card W47-LOOP-OWNER-CONTRACTS' `fluent_bit/loop_owner.qv`.
    // 119 -> 120 by card W47-CONDVAR-JOIN-PUBLICATION's `libc/pthread_sync.qv`.
    // 120 -> 121 by card W47-CONDVAR-JOIN-STOP-PATHS'
    // `fluent_bit/lib_engine_thread.qv`.
    // 121 -> 122 by card W47-LOOP-OWNER-INSTANCE-FLOW's `fluent_bit/input_instance.qv`.
    // 122 -> 123 by card CONTRACT-PIN-TIP-MODE's `monkey/vhost_set.qv`
    // (SUP04's `mk_vhost_set`, its lock entry tracking the fix branch's tip).
    // 123 -> 124 by card MONKEY-VHOST-PROPERTY-CUSTODY's `monkey/string.qv`
    // (`mk_string_dup`).
    // 124 -> 125 by card CHUNKIO-META-LEN-BOUNDS-CONTRACTS'
    // `chunkio/meta_len_bounds.qv`, the first bounds contract v1 module
    // (pinned in `bounds-contracts.lock`, outside every resource view).
    // 125 -> 126 by card W43-ABI-ENUM-AND-CONST-PP's `librdkafka/client.qv`.
    // 126 -> 127 by card CG16-BOUNDS-TRANSPORT-AND-CHUNKIO-NEIGHBOURS'
    // `chunkio/file_st_bounds.qv` (pinned in `bounds-contracts.lock`, outside
    // every resource view).
    // 127 -> 130 by card NULL-SAFETY-PROPERTY's three `null_deref.qv` modules
    // (cmetrics, fluent_bit, librdkafka; bounds contract v1, pinned in each
    // crate's `bounds-contracts.lock`, outside every resource view).
    // 130 -> 131 by card NULL-TRIAGE-UNRESOLVED-PATHS' `cfl/null_deref.qv`
    // (bounds contract v1, pinned in the cfl crate's new `bounds-contracts.lock`).
    // 131 -> 133 by card FB-EVENT-LOOP-RETENTION-AND-REGISTRATION's
    // `monkey/event_loop.qv` and `fluent_bit/input_collector_registration.qv`.
    // 133 -> 134 by card FB-TEARDOWN-DESTROY-DECLARATIONS'
    // `fluent_bit/teardown_destroy.qv`.
    // 134 -> 135 by card OUTER-CONTRACT's `fluent_bit/outer.qv`, the first
    // outer contract v1 module (pinned in `outer-contracts.lock`, outside
    // every resource and bounds view).
    // 135 -> 136 by RK-PLUGIN-LIBRARY-OUTER-ENVELOPE's librdkafka/outer.qv.
    assert_eq!(sources.len(), 136, "unexpected contract corpus size");

    for (logical, path) in &sources {
        let written = fs::read_to_string(path).expect("contract source is readable");
        assert!(
            !written.contains(qcore_contract::contract_loader::RETIRED_PREMISE_DIRECTIVE),
            "{}: the retired premise directive must not reappear",
            path.display()
        );
        // Card CONTRACT-INHERIT-ADOPTION: a module that inherits is checked as
        // its elaboration, the text every loader parses and retains.
        let source = contract_corpus::text(logical, path);
        // Card HASH-CLAUSE. The `qcore-premise` comment directive is retired in
        // favour of §C04.5's `resource_literal_argument` clause. The resource
        // loader rejects a module carrying it, but only the modules the
        // manifest selects ever reach that loader; this sweeps the whole corpus,
        // so a directive cannot rot unread in an unselected module either.
        assert!(
            !source.contains(qcore_contract::contract_loader::RETIRED_PREMISE_DIRECTIVE),
            "{}: the retired premise directive must not reappear; declare \
             `requires resource_literal_argument(PARAMETER, LITERAL);`",
            path.display()
        );
        let parsed = parse_source_program(&source)
            .unwrap_or_else(|error| panic!("{}: {error}", path.display()));
        let reparsed = parse_source_program(&parsed.to_string())
            .unwrap_or_else(|error| panic!("{} failed to round trip: {error}", path.display()));
        assert!(parsed.syntax_equivalent(&reparsed), "{}", path.display());

        let module = compile_files(
            &[SourceFile::new(path.display().to_string(), source)],
            &CompileOptions::default(),
        )
        .unwrap_or_else(|diagnostics| {
            panic!(
                "{} failed compiler regression:\n{}",
                path.display(),
                diagnostics
                    .iter()
                    .map(format_diagnostic)
                    .collect::<Vec<_>>()
                    .join("\n")
            )
        });
        assert_eq!(
            module.mode,
            qcore_compiler::frontend::SourceModuleMode::Contract
        );
        assert!(module.entry.is_none(), "{}", path.display());
        assert!(module.c_source.is_none(), "{}", path.display());
    }
}

#[test]
fn contract_surface_is_pinned_as_structured_clauses() {
    let mut actual = Vec::new();
    let mut all_expressions = Vec::new();
    let mut has_normal = false;
    let mut has_error = false;
    let mut has_any = false;

    for (relative, path) in contract_corpus::sources() {
        // Card CONTRACT-INHERIT-ADOPTION: the pins count **elaborated** clauses
        // (what the loader loads and the environment retains), so converting a
        // hand-written declaration to `contract inherit` leaves its pin
        // unchanged; the written form is pinned separately by
        // `EXPECTED_INHERITANCE`.
        let source = contract_corpus::text(&relative, &path);
        let parsed = parse_source_program(&source).expect("contract source parses");
        for declaration in &parsed.declarations {
            if declaration.value.kind == SourceDeclarationKind::ExternFunction {
                let contract = declaration
                    .value
                    .function_contract
                    .as_ref()
                    .unwrap_or_else(|| {
                        panic!("{relative}: {} has no contract", declaration.value.name)
                    });
                actual.push((
                    relative.clone(),
                    declaration.value.name.clone(),
                    contract.clauses.len(),
                ));
                for clause in &contract.clauses {
                    assert!(!clause.value.expression_source.is_empty());
                    all_expressions.push(clause.value.expression_source.clone());
                    match &clause.value.kind {
                        SourceContractClauseKind::Requires
                        | SourceContractClauseKind::QuantCheck
                        | SourceContractClauseKind::QuantNullDeref
                        | SourceContractClauseKind::QuantDeref
                        | SourceContractClauseKind::QuantLive
                        | SourceContractClauseKind::QuantOuter => {}
                        SourceContractClauseKind::Ensures(selector)
                        | SourceContractClauseKind::Quant(selector) => match selector {
                            SourceContractExitSelector::Normal => has_normal = true,
                            SourceContractExitSelector::Error(_) => has_error = true,
                            SourceContractExitSelector::Any => has_any = true,
                        },
                    }
                }
            }
        }
    }

    let expected = EXPECTED_CONTRACTS
        .iter()
        .map(|(path, name, clauses)| ((*path).to_owned(), (*name).to_owned(), *clauses))
        .collect::<Vec<_>>();
    assert_eq!(actual, expected);
    assert!(has_normal && has_error && has_any);

    for required in [
        // Card O03-U5: the corrected `evutil_closesocket` error arm. The
        // superseded `descriptor_custody_is_preserved` clause on that
        // declaration is refused below.
        "descriptor_custody_is_unresolved",
        "descriptor_number_must_not_be_reused",
        "no_close_retry_is_permitted",
        "close_error_is_read_on_the_platform_channel",
        "data_durability_is_not_discharged",
        "handoff_has_synchronization_evidence",
        "every_cross_context_transition_has_typed_evidence",
        "delete_only_is_not_object_destruction",
        "nested_teardown_order_is_context_stream_chunk_backend",
        "callback_summaries_are_exact",
        "observed_trace_refines_checked_trace",
        "no_event_certificate_is_issued",
    ] {
        assert!(
            all_expressions
                .iter()
                .any(|expression| expression == required),
            "missing pinned contract predicate `{required}`"
        );
    }
}

/// Card LIBRDKAFKA-PLUGIN-PREMISE-REMAINDER (USER DECISION 2026-10-15, "Add
/// premise support"): every corpus module's named premises read through the
/// contract format, whatever its vocabulary, and the Quirin model contract
/// `rd_kafka_q_destroy_final` (outside every resource view, in no lock) names
/// P-RK-PLUGIN-CALLBACK-CLOSURE.
#[test]
fn every_module_reads_its_premises_and_the_model_contract_names_the_plugin_premise() {
    let mut premised = std::collections::BTreeMap::new();
    for (logical, path) in contract_corpus::sources() {
        let source = contract_corpus::text(&logical, &path);
        let premises = qcore_contract::contract_loader::module_premises(&source)
            .unwrap_or_else(|error| panic!("{logical}: {error}"));
        for (function, list) in premises {
            premised.insert((logical.clone(), function), list);
        }
    }
    let model = &premised[&(
        "librdkafka/queue_destructor.qv".to_owned(),
        "rd_kafka_q_destroy_final".to_owned(),
    )];
    assert_eq!(
        model.iter().map(|p| p.name.as_str()).collect::<Vec<_>>(),
        ["rk_plugin_callback_closure"]
    );
    for function in ["rd_kafka_crash", "rd_kafka_dump0"] {
        let module = if function == "rd_kafka_crash" {
            "librdkafka/memory.qv"
        } else {
            "librdkafka/kafka_mem.qv"
        };
        assert!(
            premised[&(module.to_owned(), function.to_owned())]
                .iter()
                .any(|p| p.name == "rk_plugin_callback_closure"),
            "{function}"
        );
    }
    // The 67 premised stable/scope member declarations of the null view,
    // and since card NONNULL-MEMBER-UNIFIED-QUALIFIER ("Yes, all kinds") its
    // four premised plain member declarations: 71.
    assert_eq!(
        premised
            .keys()
            .filter(|(module, _)| module == "librdkafka/null_deref.qv")
            .count(),
        71
    );
    for function in [
        "rd_kafka_toppar_new0",
        "rd_kafka_topic_new0",
        "rd_kafka_cgrp_new",
        "rd_kafka_transport_new",
    ] {
        assert!(
            premised[&("librdkafka/null_deref.qv".to_owned(), function.to_owned())]
                .iter()
                .any(|p| p.name == "rk_plugin_callback_closure"),
            "{function}"
        );
    }
    // Fluent Bit's member declarations name none (out-of-tree plugins are
    // OUTER-CONTRACT-DESIGN's).
    assert!(
        !premised
            .keys()
            .any(|(module, _)| module == "fluent_bit/null_deref.qv")
    );
}

/// Card O03-U5. `evutil_closesocket` is `close(sock)` on every non-Windows
/// build and `closesocket(sock)` on Windows, with no error translation
/// (`third_party/libevent/evutil.c:613-621` at the pinned revision). Its
/// declared error arm therefore may not assert preserved custody: on the
/// audited build a failed close has already released the descriptor except on
/// `EBADF`, so "custody is preserved" prescribes the retry the platform
/// documents as a hazard. This pins the corrected arm and refuses the old one
/// by name, on that declaration only — `evutil_socket_geterror` keeps its own
/// `descriptor_custody_is_preserved` clause, which is a borrow and is correct.
#[test]
fn the_closesocket_error_arm_states_an_unresolved_endpoint() {
    let path = contract_corpus::sources()
        .remove("monkey/composition.qv")
        .unwrap();
    let source = fs::read_to_string(&path).expect("contract source is readable");
    let parsed = parse_source_program(&source).expect("contract source parses");

    let mut checked = 0;
    for declaration in &parsed.declarations {
        if declaration.value.name != "evutil_closesocket" {
            continue;
        }
        checked += 1;
        let contract = declaration
            .value
            .function_contract
            .as_ref()
            .expect("evutil_closesocket has a contract");
        let error_arm = contract
            .clauses
            .iter()
            .filter(|clause| {
                matches!(
                    &clause.value.kind,
                    SourceContractClauseKind::Ensures(SourceContractExitSelector::Error(_))
                        | SourceContractClauseKind::Quant(SourceContractExitSelector::Error(_))
                )
            })
            .map(|clause| clause.value.expression_source.clone())
            .collect::<Vec<_>>();
        assert_eq!(
            error_arm,
            vec![
                "descriptor_custody_is_unresolved".to_owned(),
                "descriptor_number_must_not_be_reused".to_owned(),
                "no_close_retry_is_permitted".to_owned(),
                "close_error_is_read_on_the_platform_channel".to_owned(),
            ],
        );
        assert!(
            !contract
                .clauses
                .iter()
                .any(|clause| clause.value.expression_source == "descriptor_custody_is_preserved"),
            "the withdrawn returns-on-failure clause is back on evutil_closesocket"
        );
        assert!(
            contract.clauses.iter().any(|clause| {
                clause.value.expression_source == "data_durability_is_not_discharged"
                    && matches!(
                        &clause.value.kind,
                        SourceContractClauseKind::Ensures(SourceContractExitSelector::Any)
                            | SourceContractClauseKind::Quant(SourceContractExitSelector::Any)
                    )
            }),
            "durability must stay separated from descriptor custody on every exit"
        );
    }
    assert_eq!(
        checked, 1,
        "expected exactly one evutil_closesocket declaration"
    );
}

/// Card CONTRACT-INHERIT-ADOPTION: every declaration in the corpus whose
/// contract is written `contract inherit NS::NAME;`, as
/// `(logical module, child, parent as written)`, in logical-path then source
/// order. The clause pins above count each child's **elaborated** clauses; this
/// table pins the written form, so a conversion to or from inheritance is a
/// deliberate edit here even though it moves no clause pin.
const EXPECTED_INHERITANCE: &[(&str, &str, &str)] = &[
    // `include/fluent-bit/flb_mem.h`: `flb_malloc` and `flb_calloc` return
    // NULL for a zero size and otherwise forward to `malloc` / `calloc`;
    // `flb_free` is `free(ptr)`. The elaborations equal the hand-written
    // contracts they replace, clause for clause
    // (`compiler/tests/contract_inherit_adoption.rs`).
    (
        "fluent_bit/resource_memory.qv",
        "flb_malloc",
        "libc::malloc",
    ),
    (
        "fluent_bit/resource_memory.qv",
        "flb_calloc",
        "libc::calloc",
    ),
    ("fluent_bit/resource_memory.qv", "flb_free", "libc::free"),
    // `include/fluent-bit/flb_str.h`: user-directed declared assumptions over
    // `static inline` bodies the analyzer could read (the report says so).
    (
        "fluent_bit/resource_string.qv",
        "flb_strdup",
        "libc::strdup",
    ),
    (
        "fluent_bit/resource_string.qv",
        "flb_strndup",
        "libc::strndup",
    ),
    // Card CONTRACT-INHERIT-QCORE-PROVIDER-NAMING: `src/rd.h` forwards to
    // `malloc`, `calloc`, `strdup`, `strndup` and `free` (with `rd_assert`),
    // and `mk_memory.h` to `malloc`, `calloc(1, n)` and `free`. Each
    // elaboration equals the hand-written contract it replaces, clause for
    // clause (`compiler/tests/contract_inherit_qcore_provider.rs`), so the
    // elaborated clause pins in `EXPECTED_CONTRACTS` are unchanged.
    ("librdkafka/memory.qv", "rd_malloc", "libc::malloc"),
    ("librdkafka/memory.qv", "rd_calloc", "libc::calloc"),
    ("librdkafka/memory.qv", "rd_strdup", "libc::strdup"),
    ("librdkafka/memory.qv", "rd_strndup", "libc::strndup"),
    ("librdkafka/memory.qv", "rd_free", "libc::free"),
    ("monkey/memory.qv", "mk_mem_alloc", "libc::malloc"),
    // `calloc(1, size)` in the body; `malloc` by signature (one parameter).
    ("monkey/memory.qv", "mk_mem_alloc_z", "libc::malloc"),
    ("monkey/memory.qv", "mk_mem_free", "libc::free"),
];

#[test]
fn inherited_contracts_are_pinned_in_their_written_form() {
    let mut actual = Vec::new();
    for (logical, path) in contract_corpus::sources() {
        let written = fs::read_to_string(&path).expect("contract source is readable");
        let syntax = qcore_frontend::contract_inclusion::scan_contract_inclusion(&written)
            .unwrap_or_else(|error| panic!("{logical}: {error}"));
        if syntax.is_empty() {
            continue;
        }
        // The module must elaborate (and so load) at all.
        let elaborated = contract_corpus::text(&logical, &path);
        parse_source_program(&elaborated).unwrap();
        for site in &syntax.inherits {
            // The child is the `extern fn` whose signature the `contract`
            // token closes.
            let head = &written[..site.contract_span.start];
            let child = head
                .rfind("extern fn ")
                .map(|start| {
                    head[start + "extern fn ".len()..]
                        .split(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
                        .next()
                        .unwrap()
                        .to_owned()
                })
                .unwrap_or_else(|| panic!("{logical}: an inheriting site names no declaration"));
            let parent = match &site.namespace {
                Some(namespace) => format!("{namespace}::{}", site.parent),
                None => site.parent.clone(),
            };
            actual.push((logical.clone(), child, parent));
        }
    }
    let expected = EXPECTED_INHERITANCE
        .iter()
        .map(|(module, child, parent)| {
            (
                (*module).to_owned(),
                (*child).to_owned(),
                (*parent).to_owned(),
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(actual, expected);
}
