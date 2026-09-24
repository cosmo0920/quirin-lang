use std::fs;

use qcore_compiler::{
    CompileOptions, SourceContractClauseKind, SourceContractExitSelector, SourceDeclarationKind,
    SourceFile, compile_files, format_diagnostic, parse_source_program,
};

const EXPECTED_CONTRACTS: &[(&str, &str, usize)] = &[
    (
        "cfl/arena_lifetime.qcore",
        "cfl_arena_create_with_options",
        5,
    ),
    ("cfl/arena_lifetime.qcore", "cfl_arena_destroy", 5),
    ("cfl/intrusive_list.qcore", "cfl_list_add", 4),
    ("cfl/intrusive_list.qcore", "cfl_list_del", 5),
    // Card W36-CORPUS-CONTRACTS-3: the CFL dynamic string's release, the
    // highest function-cover callee in the baseline (8 projected functions name
    // it and nothing else). Read from `lib/cfl/src/cfl_sds.c:249-275`;
    // `nullable` because the body's first statement is `if (!s) { return; }`.
    ("cfl/sds_lifetime.qcore", "cfl_sds_destroy", 3),
    // Card W37-CORPUS-CONTRACTS-4: the CFL wall-clock reading, sole blocker
    // of `ctr_span_end`. Zero parameters and a scalar `uint64_t` result, so
    // the contract is `requires` + `return_none` and nothing else — the
    // `flb_time_now` shape, whose C header is unprototyped `()` in exactly
    // the same way, which is what settles the arity gate by precedent.
    // Read from `lib/cfl/src/cfl_time.c:31-60`.
    ("cfl/time.qcore", "cfl_time_now", 2),
    ("chunkio/chunk_destructor.qcore", "cio_chunk_close", 10),
    ("chunkio/chunk_destructor.qcore", "cio_chunk_delete", 5),
    // Card NEIGHBOURS: the corpus moves 64 -> 67 modules with this file,
    // `fluent_bit/input_chunk_helpers.qcore` and `fluent_bit/time.qcore`, and
    // the resource-v1 declaration count 25 -> 31. Deliberate; see
    // `docs/reports/remaining-neighbours/README.md`. All three residency
    // helpers borrow the chunk and return no custody, which is the only v1
    // mode combination with the caller-side effect each pinned body was read
    // to have: neither `cio_chunk_up_force` nor `cio_chunk_down` frees the
    // chunk or its backend, so the caller's obligation to `cio_chunk_close` is
    // unchanged across both.
    ("chunkio/chunk_residency.qcore", "cio_chunk_is_up", 3),
    ("chunkio/chunk_residency.qcore", "cio_chunk_up_force", 3),
    ("chunkio/chunk_residency.qcore", "cio_chunk_down", 3),
    // Card W31-CORPUS-CONTRACTS-2: the corpus moves 74 -> 76 modules with this
    // file and `fluent_bit/input_chunk_total_size.qcore`, and the resource-v1
    // declaration count 41 -> 43. Deliberate; see
    // `docs/reports/w31-corpus-contracts-2/README.md`. The body
    // (`lib/chunkio/src/cio_chunk.c:315-334` at the pinned checkout) is a pure
    // size query — an error-field reset and backend length-field reads — so
    // `resource_borrowed(ch)` and `resource_return_none()` are the only
    // faithful modes, the same reading `fluent_bit/input_chunk_sizes.qcore`
    // recorded for its wrapper. This lands the ChunkIO-side module card
    // W29-CORPUS-CONTRACTS' `W29-CONTENT-SIZE-UNDECLARED` asked for.
    ("chunkio/chunk_sizes.qcore", "cio_chunk_get_content_size", 3),
    (
        "chunkio/constructor_failure_observation.qcore",
        "native_constructor_failure_observation",
        11,
    ),
    ("chunkio/context_destructor.qcore", "cio_destroy", 7),
    ("chunkio/counters.qcore", "cio_chunk_counter_total_add", 10),
    ("chunkio/counters.qcore", "cio_chunk_counter_total_sub", 10),
    (
        "chunkio/counters.qcore",
        "cio_chunk_counter_total_up_add",
        10,
    ),
    (
        "chunkio/counters.qcore",
        "cio_chunk_counter_total_up_sub",
        10,
    ),
    ("chunkio/error_frame.qcore", "cio_error_reset", 5),
    ("chunkio/error_state.qcore", "cio_error_reset", 5),
    ("chunkio/file_close.qcore", "file_close_no_delete", 17),
    ("chunkio/file_close_delete.qcore", "file_close_delete", 13),
    (
        "chunkio/filesystem_attempts.qcore",
        "file_close_result_use",
        6,
    ),
    (
        "chunkio/filesystem_attempts.qcore",
        "file_unmap_result_use",
        3,
    ),
    (
        "chunkio/filesystem_attempts.qcore",
        "native_close_result_use",
        3,
    ),
    (
        "chunkio/filesystem_attempts.qcore",
        "native_unmap_result_use",
        3,
    ),
    (
        "chunkio/filesystem_chunk_close.qcore",
        "filesystem_chunk_close",
        22,
    ),
    (
        "chunkio/filesystem_constructor_failure_observation.qcore",
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
    (
        "chunkio/filesystem_delete.qcore",
        "native_delete_outcomes",
        25,
    ),
    (
        "chunkio/filesystem_delete_attempt.qcore",
        "native_delete_result_use",
        3,
    ),
    (
        "chunkio/filesystem_delete_observation.qcore",
        "native_delete_observation",
        12,
    ),
    (
        "chunkio/filesystem_descriptor_failure_observation.qcore",
        "native_file_descriptor_failure_observation",
        20,
    ),
    (
        "chunkio/filesystem_existing_observation.qcore",
        "native_file_existing_observation",
        17,
    ),
    (
        "chunkio/filesystem_integrity_observation.qcore",
        "native_file_integrity_observation",
        21,
    ),
    (
        "chunkio/filesystem_observation.qcore",
        "native_cleanup_observation",
        14,
    ),
    (
        "chunkio/filesystem_os_constructor_observation.qcore",
        "native_file_os_constructor_observation",
        17,
    ),
    (
        "chunkio/filesystem_os_observation.qcore",
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
        "chunkio/filesystem_outcomes.qcore",
        "native_close_outcomes",
        26,
    ),
    (
        "chunkio/filesystem_outcomes.qcore",
        "native_unmap_outcomes",
        29,
    ),
    (
        "chunkio/filesystem_reopen_failure_observation.qcore",
        "native_file_reopen_failure_observation",
        28,
    ),
    (
        "chunkio/filesystem_sync_failure_observation.qcore",
        "native_file_sync_failure_observation",
        18,
    ),
    (
        "chunkio/filesystem_teardown_observation.qcore",
        "native_file_teardown_observation",
        16,
    ),
    (
        "chunkio/filesystem_transition_observation.qcore",
        "native_file_transition_observation",
        16,
    ),
    (
        "chunkio/filesystem_unmap_failure_observation.qcore",
        "native_file_unmap_failure_observation",
        19,
    ),
    (
        "chunkio/import_composition.qcore",
        "check_imported_chunk_program",
        7,
    ),
    ("chunkio/list_unlink.qcore", "__mk_list_del", 6),
    ("chunkio/list_unlink.qcore", "mk_list_del", 12),
    ("chunkio/lock_state.qcore", "cio_chunk_is_locked", 7),
    ("chunkio/lock_state.qcore", "cio_chunk_unlock", 8),
    ("chunkio/memory_backend.qcore", "cio_memfs_close", 22),
    ("chunkio/memory_close.qcore", "cio_chunk_close", 19),
    ("chunkio/nested_cleanup.qcore", "nested_cleanup", 12),
    (
        "chunkio/nested_memory_teardown.qcore",
        "nested_memory_teardown",
        10,
    ),
    ("chunkio/parent_teardown.qcore", "parent_teardown", 11),
    ("chunkio/resource_open_close.qcore", "cio_chunk_open", 8),
    ("chunkio/resource_open_close.qcore", "cio_chunk_close", 4),
    ("chunkio/stream_destructor.qcore", "cio_stream_destroy", 6),
    ("chunkio/stream_destructor.qcore", "cio_stream_delete", 5),
    (
        "chunkio/teardown_observation.qcore",
        "native_teardown_observation",
        11,
    ),
    ("chunkio/teardown_traversal.qcore", "teardown_traversal", 11),
    ("chunkio/unmap_wrapper.qcore", "unmap_file_outcomes", 17),
    (
        "chunkio/unsupported_close.qcore",
        "unsupported_chunk_close",
        12,
    ),
    (
        "chunkio/unsupported_constructor_observation.qcore",
        "native_unsupported_constructor",
        11,
    ),
    // Card W36-CORPUS-CONTRACTS-3: the CMetrics context's release, read from
    // `lib/cmetrics/src/cmetrics.c:92-146`. `nonnull` — measured, not
    // defaulted: the body has no leading null guard and its first executable
    // statement reads `cmt->counters`.
    ("cmetrics/context_lifetime.qcore", "cmt_destroy", 3),
    ("cmetrics/counter_reset.qcore", "cmt_counter_allow_reset", 4),
    // Card W37-CORPUS-CONTRACTS-4: two CMetrics metric-value accessors, each
    // the sole blocker of one projected function (`check_staled_timestamp` and
    // `cmt_metric_set`). Both are custody-NEUTRAL — the metric is read through
    // or written through, never released — and both are defined in
    // `cmt_metric.c`, an ANALYSED unit, so constraint 8's duty gate applies;
    // the declarations are the neutral element, which a faithful summary of a
    // body that releases nothing cannot contradict. Read from
    // `lib/cmetrics/src/cmt_metric.c:244-251` and `:71-81`.
    (
        "cmetrics/metric_values.qcore",
        "cmt_metric_get_timestamp",
        3,
    ),
    ("cmetrics/metric_values.qcore", "cmt_metric_set_double", 5),
    // Card W37-CORPUS-CONTRACTS-4: the unsigned msgpack tag reader, sole
    // blocker of `unpack_meta_ver`. `output_buffer` is an OUT parameter, but
    // what is written through it is a SCALAR, not a resource identity, so it
    // is `borrowed` and not a custody `Out` — declaring `Out` would assert the
    // caller acquires an obligation it does not acquire. Both pointer formals
    // are NULL-guarded before any dereference, so the body has real failure
    // paths a borrowed declaration cannot delete (constraint 4). Read from
    // `lib/cmetrics/src/cmt_mpack_utils.c:88-113`.
    (
        "cmetrics/mpack_utils.qcore",
        "cmt_mpack_consume_uint_tag",
        4,
    ),
    // Card W36-CORPUS-CONTRACTS-3: `cmt_destroy`'s exact sibling one library
    // over, read from `lib/ctraces/src/ctraces.c:58-71`, `nonnull` on the same
    // ground. This is the corpus's first `ctraces` contract module.
    ("ctraces/context_lifetime.qcore", "ctr_destroy", 3),
    (
        "fluent_bit/busy_state.qcore",
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
        "fluent_bit/hash_table_publication.qcore",
        "flb_hash_table_add",
        8,
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
        "fluent_bit/input_chunk_destroy.qcore",
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
    // CONSTRUCTOR-RESIDUALS-ABI-TYPEDEF gave `msgpack/packer.qcore`.
    (
        "fluent_bit/input_chunk_destroy_direct_routes.qcore",
        "flb_input_chunk_destroy_direct_routes",
        4,
    ),
    // Card NEIGHBOURS, continued. Both helpers are file-`static` definitions
    // inside `src/flb_input_chunk.c`, itself a corpus unit: a declared contract
    // always wins over a W4-inferred summary, and a disagreement is reported as
    // a `SummaryConflict` diagnostic rather than silently resolved.
    (
        "fluent_bit/input_chunk_helpers.qcore",
        "generate_chunk_name",
        5,
    ),
    (
        "fluent_bit/input_chunk_helpers.qcore",
        "input_chunk_write_header",
        6,
    ),
    // Card PROD-CONTRACTS: the corpus moves 61 -> 64 modules with this file,
    // `fluent_bit/logging.qcore` and `fluent_bit/routes_mask.qcore`, and the
    // resource-v1 declaration count 20 -> 25. Deliberate; see
    // `docs/reports/remaining-prod-contracts/README.md`.
    (
        "fluent_bit/input_chunk_name.qcore",
        "flb_input_chunk_get_name",
        3,
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
        "fluent_bit/input_chunk_sizes.qcore",
        "flb_input_chunk_get_size",
        3,
    ),
    (
        "fluent_bit/input_chunk_sizes.qcore",
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
        "fluent_bit/input_chunk_total_size.qcore",
        "flb_input_chunk_total_size",
        3,
    ),
    ("fluent_bit/logging.qcore", "flb_log_check", 3),
    // Card CONSTRUCTOR-RESIDUALS: the corpus moves 67 -> 68 modules with
    // `libc/errno.qcore`, and this module gains one declaration. Deliberate;
    // see `docs/reports/remaining-constructor-residuals/README.md`. Five
    // clauses: the marker, three borrowed scalars/pointers, and the scalar
    // result. No variadic tail — the C declaration is not variadic.
    ("fluent_bit/logging.qcore", "flb_errno_print", 5),
    ("fluent_bit/logging.qcore", "flb_log_print", 7),
    ("fluent_bit/memory.qcore", "flb_malloc", 6),
    ("fluent_bit/memory.qcore", "flb_calloc", 7),
    ("fluent_bit/memory.qcore", "flb_realloc", 9),
    ("fluent_bit/memory.qcore", "flb_realloc_z", 12),
    ("fluent_bit/memory.qcore", "flb_free", 5),
    ("fluent_bit/resource_memory.qcore", "flb_malloc", 3),
    ("fluent_bit/resource_memory.qcore", "flb_calloc", 4),
    ("fluent_bit/resource_memory.qcore", "flb_free", 3),
    ("fluent_bit/resource_memory.qcore", "flb_sds_create_len", 4),
    ("fluent_bit/resource_memory.qcore", "flb_sds_destroy", 3),
    // Card CONTRACT-INHERIT-ADOPTION: user-directed declared assumptions,
    // each inheriting `libc::strdup` / `libc::strndup`; elaborated counts.
    ("fluent_bit/resource_string.qcore", "flb_strdup", 3),
    ("fluent_bit/resource_string.qcore", "flb_strndup", 4),
    // Card PROD-CONTRACTS, continued.
    (
        "fluent_bit/routes_mask.qcore",
        "flb_routes_mask_set_by_tag",
        6,
    ),
    (
        "fluent_bit/routes_mask.qcore",
        "flb_routes_mask_get_size",
        3,
    ),
    // Card NEIGHBOURS, continued. The FIRST production declaration whose C
    // result is floating-point: `double flb_time_now()` is declared `-> f64`
    // rather than projected onto an integer it does not have, which is why the
    // card added `f32`/`f64` to `SCALAR_ABI_TYPES`. Two clauses: the marker and
    // the result mode. It takes no parameter, so it has no parameter clause.
    ("fluent_bit/time.qcore", "flb_time_now", 2),
    (
        "language/quantified_indices.qcore",
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
    // Card W43-DESCRIPTOR-RESOURCE-TRACKING: the new `libc/dirent.qcore`, a
    // directory-stream pair. `opendir`: the marker, one borrowed path, the
    // owned result and its `directory` kind (4); `closedir`: the marker, the
    // released stream, its kind and `none` (4). Deliberate; see
    // `docs/reports/w43-descriptor-resource-tracking/README.md`.
    ("libc/dirent.qcore", "opendir", 4),
    ("libc/dirent.qcore", "closedir", 4),
    ("libc/dlfcn.qcore", "dlerror", 2),
    // Card CONSTRUCTOR-RESIDUALS, continued. Two clauses: the marker and the
    // result mode. It takes no parameter, so it has no parameter clause — the
    // same shape as `fluent_bit/time.qcore`'s `flb_time_now`.
    ("libc/errno.qcore", "__errno_location", 2),
    ("libc/math.qcore", "ceil", 3),
    ("libc/math.qcore", "floor", 3),
    ("libc/math.qcore", "fabs", 3),
    ("libc/math.qcore", "pow", 4),
    ("libc/memory.qcore", "malloc", 3),
    ("libc/memory.qcore", "calloc", 4),
    ("libc/memory.qcore", "free", 3),
    ("libc/process.qcore", "prctl", 5),
    // Card CONSTRUCTOR-RESIDUALS-ABI-TYPEDEF. Three clauses each: the marker,
    // the borrowed mutex pointer and the `int` result. `pthread_mutex_t` is a
    // union the typedef DEFINES, which card LOADER-ABI's reader would not read;
    // this card's widening reads it, and neither function takes custody of the
    // object, so `resource_borrowed` is the whole custody statement.
    ("libc/pthread_mutex.qcore", "pthread_mutex_lock", 3),
    ("libc/pthread_mutex.qcore", "pthread_mutex_unlock", 3),
    ("libc/socket.qcore", "setsockopt", 7),
    ("libc/socket.qcore", "send", 6),
    ("libc/socket.qcore", "recv", 6),
    ("libc/socket.qcore", "listen", 4),
    ("libc/socket.qcore", "sendfile", 6),
    ("libc/socket.qcore", "htonl", 3),
    ("libc/socket.qcore", "ntohl", 3),
    ("libc/socket.qcore", "gai_strerror", 3),
    // Card W43-DESCRIPTOR-RESOURCE-TRACKING: `socket`, appended. The marker,
    // three borrowed scalars, the `sentinel` owned result and its `descriptor`
    // kind (6).
    ("libc/socket.qcore", "socket", 6),
    ("libc/stdio.qcore", "perror", 3),
    ("libc/stdio.qcore", "fflush", 3),
    ("libc/stdio.qcore", "remove", 3),
    // Card W43-DESCRIPTOR-RESOURCE-TRACKING: the stream pair and `fileno`,
    // appended. `fopen`: marker, two borrowed, owned result, `stream` kind (5);
    // `fclose`: marker, released stream, kind, `none` (4); `fileno`: the
    // neutral element over one parameter (3).
    ("libc/stdio.qcore", "fopen", 5),
    ("libc/stdio.qcore", "fclose", 4),
    ("libc/stdio.qcore", "fileno", 3),
    ("libc/stdlib.qcore", "atoi", 3),
    ("libc/stdlib.qcore", "srand", 3),
    ("libc/stdlib.qcore", "rand_r", 3),
    ("libc/stdlib.qcore", "system", 3),
    ("libc/string.qcore", "strcmp", 4),
    ("libc/string.qcore", "strlen", 3),
    ("libc/string.qcore", "strdup", 3),
    ("libc/string.qcore", "strerror_r", 5),
    // Card W43-LIBC-RESULT-ALIAS: thirteen derived-result declarations, each a
    // trusted assumption read from the prototype the pinned units declare. The
    // shape is the neutral one with a derived result in place of `none`: the
    // marker, one borrowed clause per parameter and the result mode. Five exact
    // aliases (`resource_return_alias(dest, nonnull)`) and six interior
    // pointers (`resource_return_interior(s, nullable)`) here, two exact
    // aliases in `libc/time.qcore`. Deliberate; see
    // `docs/reports/w43-libc-result-alias/README.md`.
    ("libc/string.qcore", "strcpy", 4),
    ("libc/string.qcore", "strncpy", 5),
    ("libc/string.qcore", "memcpy", 5),
    ("libc/string.qcore", "memmove", 5),
    ("libc/string.qcore", "memset", 5),
    ("libc/string.qcore", "strchr", 4),
    ("libc/string.qcore", "strrchr", 4),
    ("libc/string.qcore", "strstr", 4),
    ("libc/string.qcore", "strcasestr", 4),
    ("libc/string.qcore", "memchr", 5),
    ("libc/string.qcore", "memrchr", 5),
    // Card CONTRACT-INHERIT-ADOPTION: `strndup`, user-directed; `strdup`'s
    // custody statement plus a borrowed size (2 + 1 per parameter + owned).
    ("libc/string.qcore", "strndup", 4),
    ("libc/sys_stat.qcore", "fstat", 4),
    ("libc/sys_stat.qcore", "mkdir", 4),
    ("libc/sys_stat.qcore", "umask", 3),
    ("libc/threads.qcore", "mtx_lock", 3),
    ("libc/threads.qcore", "mtx_unlock", 3),
    ("libc/threads.qcore", "cnd_wait", 4),
    ("libc/threads.qcore", "pthread_rwlock_rdlock", 3),
    ("libc/threads.qcore", "pthread_rwlock_wrlock", 3),
    ("libc/threads.qcore", "pthread_rwlock_unlock", 3),
    ("libc/threads.qcore", "pthread_self", 2),
    ("libc/threads.qcore", "pthread_kill", 4),
    ("libc/threads.qcore", "pthread_setcancelstate", 4),
    ("libc/threads.qcore", "pthread_setcanceltype", 4),
    ("libc/threads.qcore", "pthread_attr_setdetachstate", 4),
    ("libc/threads.qcore", "sigemptyset", 3),
    ("libc/threads.qcore", "sigfillset", 3),
    ("libc/threads.qcore", "sigaddset", 4),
    ("libc/time.qcore", "time", 3),
    ("libc/time.qcore", "timegm", 3),
    ("libc/time.qcore", "gmtime_r", 4),
    ("libc/time.qcore", "localtime_r", 4),
    ("libc/unistd.qcore", "sleep", 3),
    ("libc/unistd.qcore", "usleep", 3),
    ("libc/unistd.qcore", "sysconf", 3),
    ("libc/unistd.qcore", "getpagesize", 2),
    ("libc/unistd.qcore", "geteuid", 2),
    ("libc/unistd.qcore", "getegid", 2),
    ("libc/unistd.qcore", "setsid", 2),
    ("libc/unistd.qcore", "access", 4),
    ("libc/unistd.qcore", "unlink", 3),
    ("libc/unistd.qcore", "read", 5),
    ("libc/unistd.qcore", "write", 5),
    // Card W43-DESCRIPTOR-RESOURCE-TRACKING: the descriptor pair, appended.
    // `open`: marker, two borrowed, the variadic data tail, the `sentinel`
    // owned result and its `descriptor` kind (6); `close`: marker, the `valid`
    // released descriptor, its kind and `none` (4).
    ("libc/unistd.qcore", "open", 6),
    ("libc/unistd.qcore", "close", 4),
    (
        "library/custody_evidence.qcore",
        "publish_library_resource",
        6,
    ),
    (
        "library/custody_evidence.qcore",
        "handoff_library_resource",
        6,
    ),
    (
        "library/custody_evidence.qcore",
        "reclaim_library_resource",
        6,
    ),
    (
        "library/custody_evidence.qcore",
        "check_imported_library_custody",
        7,
    ),
    (
        "library/custody_evidence.qcore",
        "release_library_resource",
        3,
    ),
    ("library/source_projection.qcore", "source_enum_value", 2),
    ("library/source_projection.qcore", "source_scan_effects", 3),
    ("library/source_projection.qcore", "source_scan_initial", 2),
    ("library/source_projection.qcore", "source_type_match", 1),
    ("library/source_projection.qcore", "source_symbol", 1),
    ("library/source_projection.qcore", "source_loop_index", 1),
    ("library/source_projection.qcore", "source_scan_region", 1),
    ("library/source_projection.qcore", "frame_write", 2),
    ("library/source_projection.qcore", "frame_declaration", 2),
    ("library/source_projection.qcore", "frame_dependency", 2),
    // Card W36-CORPUS-CONTRACTS-3: cJSON is vendored INSIDE librdkafka
    // (`lib/librdkafka-2.15.0/src/cJSON.c`), which is why this module sits in
    // the librdkafka family. Read from `:253-276`; `nullable` because the loop
    // condition `while (item != NULL)` precedes every dereference, and the
    // three call sites do reach the null path.
    ("librdkafka/cjson.qcore", "cJSON_Delete", 3),
    // Card W36-CORPUS-CONTRACTS-3: kept in its own module rather than appended
    // to `librdkafka/memory.qcore`, so that module's pin stays untouched. Read
    // from `lib/librdkafka-2.15.0/src/rdkafka.c:7131-7133`: `rk` is read
    // nowhere, `ptr` is forwarded to the already-declared `rd_free`.
    ("librdkafka/kafka_mem.qcore", "rd_kafka_mem_free", 4),
    // Card W37-CORPUS-CONTRACTS-4: the deprecated in-place LZ4 stream reset,
    // sole blocker of `LZ4_resetStreamState`. ZEROING IS NOT RELEASING — the
    // body is one `MEM_INIT` (memset) through the parameter, and the freeing
    // neighbour `LZ4_freeStream` is a different name already decided through
    // `kafka_mem.qcore` above. Read from
    // `lib/librdkafka-2.15.0/src/lz4.c:1524-1528`.
    ("librdkafka/lz4_stream.qcore", "LZ4_resetStream", 3),
    ("librdkafka/memory.qcore", "rd_malloc", 3),
    ("librdkafka/memory.qcore", "rd_calloc", 4),
    ("librdkafka/memory.qcore", "rd_strdup", 3),
    ("librdkafka/memory.qcore", "rd_strndup", 4),
    ("librdkafka/memory.qcore", "rd_free", 3),
    ("librdkafka/memory.qcore", "rd_realloc", 4),
    (
        "librdkafka/queue_destructor.qcore",
        "rd_kafka_q_destroy_final",
        6,
    ),
    ("librdkafka/refcount.qcore", "rd_refcnt_add0", 4),
    ("librdkafka/refcount.qcore", "rd_refcnt_sub0", 4),
    ("librdkafka/refcount.qcore", "rd_refcnt_destroy_final", 3),
    // Card W37-CORPUS-CONTRACTS-4: the in-place ordering of a topic-partition
    // list — this card's HIGHEST function-cover yield at two projected
    // functions (`..._sort_by_topic` and `..._sort_by_topic_id`). Sorting
    // permutes an array the caller still owns whole, so all three formals are
    // borrowed; `cmp` is `fnptr`, the spelling `msgpack/packer.qcore` already
    // uses for `msgpack_packer_init`'s callback. Read from
    // `lib/librdkafka-2.15.0/src/rdkafka_partition.c:3664-3674`. Its
    // higher-yield unit-mate `rd_kafka_topic_partition_destroy0` was read and
    // DECLINED on two grounds: `static` with no header declaration in the very
    // unit that calls it, and a CONDITIONAL release (`if (do_free)`) whose
    // other five sites in that unit pass 0.
    (
        "librdkafka/topic_partition.qcore",
        "rd_kafka_topic_partition_list_sort",
        5,
    ),
    ("librdkafka/typed_queue.qcore", "rd_kafka_q_enq0", 5),
    ("librdkafka/typed_queue.qcore", "rd_kafka_q_deq0", 5),
    ("monkey/callback_binding.qcore", "binding_scope", 8),
    ("monkey/callback_binding.qcore", "binding_memory", 3),
    ("monkey/callback_binding.qcore", "binding_execution", 3),
    ("monkey/callback_binding.qcore", "binding_select_path", 1),
    ("monkey/callback_binding.qcore", "binding_send", 4),
    ("monkey/callback_binding.qcore", "binding_errno_location", 2),
    ("monkey/callback_binding.qcore", "binding_perror", 1),
    (
        "monkey/callback_binding.qcore",
        "binding_evutil_closesocket",
        1,
    ),
    ("monkey/callback_binding.qcore", "binding_event_del", 1),
    ("monkey/callback_binding.qcore", "binding_event_free", 1),
    ("monkey/callback_binding.qcore", "binding_mk_mem_free", 3),
    ("monkey/callback_binding.qcore", "binding_symbol", 2),
    ("monkey/callback_binding.qcore", "binding_call_arguments", 2),
    ("monkey/callback_binding.qcore", "binding_call_interval", 3),
    ("monkey/callback_binding.qcore", "binding_runtime_effect", 2),
    (
        "monkey/callback_task_handoff.qcore",
        "selected_callbacks",
        4,
    ),
    (
        "monkey/callback_task_handoff.qcore",
        "selected_callback_identity",
        6,
    ),
    (
        "monkey/callback_task_handoff.qcore",
        "effect_errno_location",
        3,
    ),
    ("monkey/callback_task_handoff.qcore", "effect_event_del", 3),
    ("monkey/callback_task_handoff.qcore", "effect_event_free", 3),
    (
        "monkey/callback_task_handoff.qcore",
        "effect_evutil_closesocket",
        3,
    ),
    (
        "monkey/callback_task_handoff.qcore",
        "effect_mk_mem_free",
        3,
    ),
    ("monkey/callback_task_handoff.qcore", "effect_perror", 3),
    ("monkey/callback_task_handoff.qcore", "effect_send", 3),
    (
        "monkey/callback_task_handoff.qcore",
        "bounded_no_task_observation",
        7,
    ),
    (
        "monkey/callback_task_handoff.qcore",
        "taskful_fixture_source",
        16,
    ),
    (
        "monkey/callback_task_handoff.qcore",
        "taskful_fixture_primitives",
        12,
    ),
    ("monkey/composition.qcore", "evutil_socketpair", 3),
    ("monkey/composition.qcore", "evutil_socket_geterror", 2),
    // Deliberate pin update (card O03-U5): the superseded
    // `descriptor_custody_is_preserved` error clause asserted a live, reusable
    // descriptor for an operation that is `close(2)` on the audited build, and
    // is replaced by four clauses stating the unresolved endpoint, the reuse
    // and retry prohibitions and the platform error channel, plus one `any`
    // clause separating durability. 4 -> 8.
    ("monkey/composition.qcore", "evutil_closesocket", 8),
    ("monkey/composition.qcore", "cb_event", 3),
    ("monkey/composition.qcore", "cb_timeout", 4),
    ("monkey/composition.qcore", "cb_wait_2_timeout", 2),
    ("monkey/composition.qcore", "check_monkey_composition", 13),
    // Deliberate Cycle 18 executable policy pins.
    ("monkey/cycle18_composition.qcore", "callback_inventory", 18),
    (
        "monkey/cycle18_composition.qcore",
        "context_identity_join",
        6,
    ),
    (
        "monkey/cycle18_composition.qcore",
        "wait_failure_source_join",
        2,
    ),
    ("monkey/cycle18_composition.qcore", "evidence_join", 41),
    ("monkey/cycle18_composition.qcore", "caller_size_bound", 8),
    (
        "monkey/cycle18_composition.qcore",
        "aggregate_acceptance",
        11,
    ),
    (
        "monkey/descriptor_observation.qcore",
        "observation_socketpair",
        4,
    ),
    (
        "monkey/descriptor_observation.qcore",
        "observation_acquire",
        4,
    ),
    ("monkey/descriptor_observation.qcore", "observation_use", 1),
    (
        "monkey/descriptor_observation.qcore",
        "observation_close",
        2,
    ),
    (
        "monkey/descriptor_observation.qcore",
        "observation_complete",
        8,
    ),
    // Card CONSTRUCTOR-RESIDUALS, continued. Four clauses: the marker, two
    // borrowed pointers and the unit result.
    (
        "monkey/event_payload.qcore",
        "cb_event_source_projection",
        12,
    ),
    (
        "monkey/event_payload.qcore",
        "contextual_payload_boundary",
        1,
    ),
    ("monkey/event_payload.qcore", "contextual_authority", 13),
    ("monkey/event_payload.qcore", "native_allocation", 8),
    (
        "monkey/event_payload.qcore",
        "contextual_payload_access",
        13,
    ),
    ("monkey/intrusive_list.qcore", "mk_list_add", 4),
    // Card W36-CORPUS-CONTRACTS-3: this card's ONE variadic declaration, the
    // `fluent_bit/logging.qcore` shape one library over — five clauses because
    // of `resource_variadic_data_tail()`. Read from
    // `lib/monkey/mk_core/mk_utils.c:200-264`: the tail is consumed by
    // `vprintf` into `stdout` and no pointer to it outlives the call.
    ("monkey/logging.qcore", "mk_print", 5),
    ("monkey/memory.qcore", "mk_mem_alloc", 3),
    ("monkey/memory.qcore", "mk_mem_alloc_z", 3),
    ("monkey/memory.qcore", "mk_mem_free", 3),
    // Deliberate Cycle 18 executable policy pins.
    (
        "monkey/native_platform_matrix.qcore",
        "native_platform_source_identity",
        3,
    ),
    (
        "monkey/native_platform_matrix.qcore",
        "linux_epoll_asan_profile",
        7,
    ),
    (
        "monkey/native_platform_matrix.qcore",
        "linux_poll_asan_profile",
        7,
    ),
    (
        "monkey/native_platform_matrix.qcore",
        "linux_select_asan_profile",
        7,
    ),
    (
        "monkey/native_platform_matrix.qcore",
        "macos_kqueue_asan_profile",
        7,
    ),
    (
        "monkey/native_platform_matrix.qcore",
        "windows_win32_asan_profile",
        7,
    ),
    (
        "monkey/native_platform_matrix.qcore",
        "windows_iocp_asan_profile",
        7,
    ),
    (
        "monkey/native_platform_matrix.qcore",
        "linux_epoll_tsan_profile",
        7,
    ),
    ("monkey/native_platform_matrix.qcore", "observed_row", 14),
    ("monkey/native_platform_matrix.qcore", "unknown_row", 4),
    ("monkey/timeout_teardown.qcore", "source_identity", 4),
    ("monkey/timeout_teardown.qcore", "cb_timeout_source", 7),
    (
        "monkey/timeout_teardown.qcore",
        "cb_timeout_teardown_path",
        1,
    ),
    ("monkey/timeout_teardown.qcore", "libevent_self_teardown", 5),
    ("monkey/timeout_teardown.qcore", "teardown_obligations", 21),
    ("monkey/wait_failure_policy.qcore", "loop_init_failure", 6),
    (
        "monkey/wait_failure_policy.qcore",
        "context_allocation_failure",
        7,
    ),
    (
        "monkey/wait_failure_policy.qcore",
        "base_creation_failure",
        8,
    ),
    (
        "monkey/wait_failure_policy.qcore",
        "fired_allocation_failure",
        10,
    ),
    (
        "monkey/wait_failure_policy.qcore",
        "timer_creation_failure",
        9,
    ),
    (
        "monkey/wait_failure_policy.qcore",
        "timer_registration_failure",
        10,
    ),
    (
        "monkey/wait_failure_policy.qcore",
        "success_observation",
        10,
    ),
    ("monkey/wait_failure_policy.qcore", "loop_size_bounds", 6),
    (
        "monkey/wait_identity_policy.qcore",
        "wait_identity_context",
        23,
    ),
    (
        "monkey/wait_identity_policy.qcore",
        "wait_identity_entry",
        1,
    ),
    (
        "monkey/wait_identity_policy.qcore",
        "wait_observed_profile",
        5,
    ),
    ("monkey/wait_source_policy.qcore", "source_identity", 4),
    (
        "monkey/wait_source_policy.qcore",
        "wait_projection_identity",
        4,
    ),
    ("monkey/wait_source_policy.qcore", "cb_wait_2_timeout", 9),
    (
        "monkey/wait_source_policy.qcore",
        "contract_cb_wait_2_timeout",
        9,
    ),
    (
        "monkey/wait_source_policy.qcore",
        "_mk_event_loop_create",
        9,
    ),
    (
        "monkey/wait_source_policy.qcore",
        "contract__mk_event_loop_create",
        9,
    ),
    (
        "monkey/wait_source_policy.qcore",
        "_mk_event_wait_with_flags",
        9,
    ),
    (
        "monkey/wait_source_policy.qcore",
        "contract__mk_event_wait_with_flags",
        9,
    ),
    ("monkey/wait_source_policy.qcore", "_mk_event_wait_2", 9),
    (
        "monkey/wait_source_policy.qcore",
        "contract__mk_event_wait_2",
        9,
    ),
    (
        "monkey/wait_source_policy.qcore",
        "_mk_event_loop_destroy",
        9,
    ),
    (
        "monkey/wait_source_policy.qcore",
        "contract__mk_event_loop_destroy",
        9,
    ),
    ("monkey/wait_source_policy.qcore", "artifact_set", 4),
    (
        "monkey/wrapper_paths.qcore",
        "mk_event_libevent_socketpair",
        3,
    ),
    ("monkey/wrapper_paths.qcore", "_mk_event_init", 3),
    ("monkey/wrapper_paths.qcore", "_mk_event_loop_create", 5),
    ("monkey/wrapper_paths.qcore", "_mk_event_loop_destroy", 4),
    ("monkey/wrapper_paths.qcore", "_mk_event_update", 5),
    ("monkey/wrapper_paths.qcore", "_mk_event_add", 7),
    ("monkey/wrapper_paths.qcore", "_mk_event_del", 7),
    ("monkey/wrapper_paths.qcore", "_mk_event_timeout_create", 7),
    ("monkey/wrapper_paths.qcore", "_mk_event_timeout_destroy", 4),
    ("monkey/wrapper_paths.qcore", "_mk_event_channel_create", 5),
    ("monkey/wrapper_paths.qcore", "_mk_event_channel_destroy", 4),
    ("monkey/wrapper_paths.qcore", "_mk_event_inject", 6),
    ("monkey/wrapper_paths.qcore", "_mk_event_wait_with_flags", 4),
    ("monkey/wrapper_paths.qcore", "_mk_event_wait_2", 5),
    ("monkey/wrapper_paths.qcore", "_mk_event_backend", 3),
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
    ("mpack/writer.qcore", "mpack_write_cstr", 4),
    ("mpack/writer.qcore", "mpack_write_str", 5),
    ("mpack/writer.qcore", "mpack_write_i64", 4),
    ("mpack/writer.qcore", "mpack_write_double", 4),
    // Card ABI-TYPEDEF-ADDRESS-OF-MEMBER. Five clauses: the marker, three
    // borrowed parameters and the `unit` result. The third parameter is the
    // corpus's first `fnptr`, on which `resource_borrowed` is both the only
    // mode the loader admits (`AGENTS.md` §L21 / spec §C04.1) and the true one,
    // a function designator never being a resource identity. Card
    // CONSTRUCTOR-RESIDUALS-ABI-TYPEDEF wrote this declaration and deliberately
    // did not land it at measured payoff zero; this card repaired the blocker
    // it named and measured the payoff, so it lands.
    ("msgpack/packer.qcore", "msgpack_packer_init", 5),
];

#[path = "support/contract_corpus.rs"]
mod contract_corpus;

#[test]
fn every_qcore_contract_source_parses_round_trips_and_compiles() {
    let sources = contract_corpus::sources();
    // 60 -> 61 by card HASH-CONTRACTS' deliberate addition of
    // `fluent_bit/hash_table_publication.qcore`; 61 -> 64 by card
    // PROD-CONTRACTS' deliberate addition of
    // `fluent_bit/input_chunk_name.qcore`, `fluent_bit/logging.qcore` and
    // `fluent_bit/routes_mask.qcore`; 64 -> 67 by card NEIGHBOURS' deliberate
    // addition of `chunkio/chunk_residency.qcore`,
    // `fluent_bit/input_chunk_helpers.qcore` and `fluent_bit/time.qcore`;
    // 67 -> 69 by card CONSTRUCTOR-RESIDUALS' deliberate addition of
    // `libc/errno.qcore` and `monkey/intrusive_list.qcore`; 69 -> 70 by card
    // CONSTRUCTOR-RESIDUALS-ABI-TYPEDEF's deliberate addition of
    // `libc/pthread_mutex.qcore`. That card read `msgpack/packer.qcore`
    // against its body too and deliberately did NOT land it: measured payoff
    // zero, because the address-of-a-member actual refuses every one of its
    // call sites before any contract is consulted. 70 -> 71 by card
    // ABI-TYPEDEF-ADDRESS-OF-MEMBER, which repaired exactly that blocker and
    // then measured the declaration's payoff on the production constructor, so
    // `msgpack/packer.qcore` lands under the unchanged PROD-CONTRACTS rule.
    // 71 -> 72 by card ADDRESS-OF-MEMBER-DESTROY-UNDECLARED's deliberate
    // addition of `fluent_bit/input_chunk_destroy.qcore`, the declaration that
    // decides the leak leaf the previous card opened, landed on the same rule
    // after its payoff was measured. 72 -> 73 by card AOM-DESTROY-SIBLINGS'
    // deliberate addition of
    // `fluent_bit/input_chunk_destroy_direct_routes.qcore`, which decides one
    // of the two siblings that card left undeclared and declines the other at
    // measured payoff zero. 73 -> 74 by card W29-CORPUS-CONTRACTS' deliberate
    // addition of `fluent_bit/input_chunk_sizes.qcore`, whose two read-only
    // size queries decide the thirteen standing uncontracted-call refusals
    // that named them inside the unit that defines them. 74 -> 76 by card
    // W31-CORPUS-CONTRACTS-2's deliberate addition of
    // `chunkio/chunk_sizes.qcore` and
    // `fluent_bit/input_chunk_total_size.qcore`, the two size-sibling
    // declarations W29's named non-declarations pointed at, each read against
    // its own pinned body. 76 -> 82 by card W36-CORPUS-CONTRACTS-3's
    // deliberate addition of six release/custody-neutral declarations, chosen
    // by FUNCTION-COVER yield over the 193 baseline functions whose only
    // refusals are `call to uncontracted function` leaves:
    // `cfl/sds_lifetime.qcore` (`cfl_sds_destroy`, 8 such functions),
    // `librdkafka/kafka_mem.qcore` (`rd_kafka_mem_free`, 4),
    // `cmetrics/context_lifetime.qcore` (`cmt_destroy`, 3),
    // `librdkafka/cjson.qcore` (`cJSON_Delete`, 3),
    // `monkey/logging.qcore` (`mk_print`, 3, this card's one variadic) and
    // `ctraces/context_lifetime.qcore` (`ctr_destroy`, 2) — the last opening
    // the corpus's first `ctraces` family. Each was read against its own pinned
    // body before it was declared; the card also read `add`, `memcpy` and
    // `memset` and deliberately did NOT land them, with the readings published
    // in `docs/reports/w36-corpus-contracts-3/README.md`.
    // 82 -> 88 by card W37-CORPUS-CONTRACTS-4's deliberate addition of six
    // custody-NEUTRAL modules, chosen by FUNCTION-COVER yield over the 163
    // baseline functions whose only refusals are `call to uncontracted
    // function` leaves: `librdkafka/topic_partition.qcore`
    // (`rd_kafka_topic_partition_list_sort`, 2 such functions),
    // `mpack/writer.qcore` (four writer primitives, 1 function each, and the
    // corpus's first `mpack` family — distinct from the existing `msgpack/`,
    // which is a different library), `cmetrics/metric_values.qcore` (2),
    // `cmetrics/mpack_utils.qcore` (1), `cfl/time.qcore` (1) and
    // `librdkafka/lz4_stream.qcore` (1). Not one is a release: after W36
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
    // `fluent_bit/resource_string.qcore` (`flb_strdup`, `flb_strndup`
    // inheriting libc). Its conversion of `fluent_bit/resource_memory.qcore`
    // to `contract inherit` changes no module count.
    // 108 -> 109 by card W43-DESCRIPTOR-RESOURCE-TRACKING's `libc/dirent.qcore`.
    assert_eq!(sources.len(), 109, "unexpected contract corpus size");

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
                        SourceContractClauseKind::Requires => {}
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
        .remove("monkey/composition.qcore")
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
        "fluent_bit/resource_memory.qcore",
        "flb_malloc",
        "libc::malloc",
    ),
    (
        "fluent_bit/resource_memory.qcore",
        "flb_calloc",
        "libc::calloc",
    ),
    ("fluent_bit/resource_memory.qcore", "flb_free", "libc::free"),
    // `include/fluent-bit/flb_str.h`: user-directed declared assumptions over
    // `static inline` bodies the analyzer could read (the report says so).
    (
        "fluent_bit/resource_string.qcore",
        "flb_strdup",
        "libc::strdup",
    ),
    (
        "fluent_bit/resource_string.qcore",
        "flb_strndup",
        "libc::strndup",
    ),
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
