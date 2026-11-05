#[path = "support/contract_corpus.rs"]
mod contract_corpus;

use std::collections::BTreeMap;
use std::path::PathBuf;

use qcore_compiler::contract_loader::{
    FUNCTION_POINTER_ABI_TYPE, POINTER_ABI_TYPES, SCALAR_ABI_TYPES, VA_LIST_ABI_TYPE,
    check_source_assumptions, load_resource_contract_files, load_resource_contract_manifest,
    load_resource_contract_sources,
};
use qcore_compiler::{
    AnalysisStatus, C11TranslationUnit, DerivedExtent, ResourceApiKind, ResourceContracts,
    ResourceFunctionContract, ResourceKind, ResourceParameterMode as P, ResourceReturnMode as R,
    UnrestrictedLegacyResult, analyze_legacy_c_translation_unit, analyze_unrestricted_c11,
    check_legacy_safety_certificate,
};

fn manifest() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../cycle19-resource-contracts.json")
}

fn load(source: &str) -> Result<ResourceContracts, String> {
    load_resource_contract_sources(&BTreeMap::from([("test.qcore".into(), source.into())]))
}

const ALLOC: &str = "module contract;
extern fn alloc() -> ptr<mut, u8> contract {
    requires resource_contract_v1;
    ensures normal resource_return_owned(nullable);
};";

#[test]
fn production_contracts_preserve_the_previous_resource_modes() {
    let contracts = load_resource_contract_manifest(&manifest()).unwrap();
    // 6 -> 9 manifest-selected modules by card PROD-CONTRACTS; 9 -> 12 by card
    // NEIGHBOURS (`fluent_bit/input_chunk_helpers.qv`,
    // `fluent_bit/time.qv`, `chunkio/chunk_residency.qv`); 12 -> 14 by
    // card CONSTRUCTOR-RESIDUALS (`libc/errno.qv`,
    // `monkey/intrusive_list.qv`); 14 -> 15 by card
    // CONSTRUCTOR-RESIDUALS-ABI-TYPEDEF (`libc/pthread_mutex.qv`; that card
    // deliberately did not land `msgpack/packer.qv`, measured payoff zero);
    // 15 -> 16 by card ABI-TYPEDEF-ADDRESS-OF-MEMBER, which repaired the
    // blocker that measurement rested on and landed it; 16 -> 17 by card
    // ADDRESS-OF-MEMBER-DESTROY-UNDECLARED
    // (`fluent_bit/input_chunk_destroy.qv`), which decides the leak leaf
    // that card's corpus movement opened; 17 -> 18 by card AOM-DESTROY-SIBLINGS
    // (`fluent_bit/input_chunk_destroy_direct_routes.qv`), which decides one
    // of the two siblings that card deliberately left undeclared; 18 -> 19 by
    // card W29-CORPUS-CONTRACTS (`fluent_bit/input_chunk_sizes.qv`), whose
    // two borrowed/none size queries decide thirteen uncontracted-call leaves;
    // 19 -> 21 by card W31-CORPUS-CONTRACTS-2 (`chunkio/chunk_sizes.qv` and
    // `fluent_bit/input_chunk_total_size.qv`), the two size-sibling
    // declarations that card's predecessor named and declined, each landed on
    // its own pinned-body reading.

    // 21 -> 27 by card W36-CORPUS-CONTRACTS-3, six declarations chosen by
    // function-cover yield over the 193 baseline functions blocked only by
    // uncontracted-call leaves, each read against its own pinned body:
    // `cfl/sds_lifetime.qv`, `cmetrics/context_lifetime.qv`,
    // `ctraces/context_lifetime.qv`, `librdkafka/cjson.qv`,
    // `librdkafka/kafka_mem.qv` and `monkey/logging.qv`.

    // 27 -> 33 by card W37-CORPUS-CONTRACTS-4, six declarative modules chosen
    // by function-cover yield over the 163 baseline functions blocked only by
    // uncontracted-call leaves, each read against its own pinned body:
    // `cfl/time.qv`, `cmetrics/metric_values.qv`,
    // `cmetrics/mpack_utils.qv`, `mpack/writer.qv` (the corpus's first
    // `mpack` family, distinct from `msgpack/`), `librdkafka/lz4_stream.qv`
    // and `librdkafka/topic_partition.qv`. Every one of the ten
    // declarations is the NEUTRAL ELEMENT — every formal borrowed, result
    // none — so none can contradict a faithful derived summary.
    // 33 -> 44 by card W43-LIBC-CONTRACTS: eleven `libc/` modules owned by
    // `providers/quirin-provider-libc` (55 trusted libc/POSIX declarations,
    // each read from the prototype the pinned units declare).
    // 44 -> 45 by card CONTRACT-INHERIT-ADOPTION: `fluent_bit/resource_string.qv`
    // selected by `crate:module` reference (schema 2); `resource_memory.qcore`
    // moved from a path entry to a reference without changing the count.
    // 45 -> 46 by card W43-LIBC-RESTRICT-PROTOTYPES (`libc/regex.qv`).
    // 46 -> 47 by card W43-DESCRIPTOR-RESOURCE-TRACKING: `libc/dirent.qv`.
    // 47 -> 49 by card W45-FB-RETRY-LIFETIME: `fluent_bit/task_users.qv` and
    // `fluent_bit/output_singleplex.qv`.
    // 49 -> 50 by card W45-FB-BACKLOG-CUSTODY: `fluent_bit/storage_backlog.qv`
    // (twelve checked specifications, contract spec C06.3).
    // Combined W46 cards plus six metric getters and five routing queries:
    // 54 selected modules, 230 resource declarations, 118 corpus modules.
    // 54 -> 55 by card W47-LOOP-OWNER-CONTRACTS: `fluent_bit/loop_owner.qv`.
    // 55 -> 56 by card W47-CONDVAR-JOIN-PUBLICATION: `libc/pthread_sync.qv`.
    // 56 -> 57 by card W47-CONDVAR-JOIN-STOP-PATHS: `fluent_bit/lib_engine_thread.qv` (five declarations awaiting review).
    // 57 -> 58 by card W47-LOOP-OWNER-INSTANCE-FLOW: `fluent_bit/input_instance.qv`.
    // 58 -> 59 by card CONTRACT-PIN-TIP-MODE: `monkey/vhost_set.qv`.
    // 59 -> 60 by card MONKEY-VHOST-PROPERTY-CUSTODY: `monkey/string.qv`.
    // 60 -> 61 by card W43-ABI-ENUM-AND-CONST-PP: `librdkafka/client.qv`.
    // 61 -> 63 by card FB-EVENT-LOOP-RETENTION-AND-REGISTRATION:
    // `monkey/event_loop.qv` and `fluent_bit/input_collector_registration.qv`.
    // 63 -> 64 by card FB-TEARDOWN-DESTROY-DECLARATIONS: `fluent_bit/teardown_destroy.qv`.
    // Kafka callback placement adds librdkafka/logging.qv to the selected sources.
    // 65 -> 66 by card W48-RK-LOG-CALLER-FRAME: `librdkafka/log_caller_frame.qv`.
    assert_eq!(contracts.source_assumptions().len(), 66);
    check_source_assumptions(&contracts).unwrap();
    let mut old = ResourceContracts::default();
    for name in [
        "malloc",
        "calloc",
        "mk_mem_alloc",
        "mk_mem_alloc_z",
        "rd_malloc",
        "rd_calloc",
        "rd_strdup",
        "rd_strndup",
        "flb_malloc",
        "flb_calloc",
        "flb_sds_create_len",
        "cio_chunk_open",
    ] {
        old.insert_contract(
            name,
            ResourceFunctionContract::new([], R::Owned { nullable: true }),
        );
    }
    for name in [
        "free",
        "mk_mem_free",
        "rd_free",
        "flb_free",
        "flb_sds_destroy",
    ] {
        old.insert_contract(
            name,
            ResourceFunctionContract::new([P::Released { nullable: true }], R::None),
        );
    }
    old.insert("cio_chunk_close", ResourceApiKind::Release);
    // Card HASH-CONTRACTS adds one declaration that was NOT in the old table.
    // It is listed explicitly rather than exempted, so the parity check below
    // still covers every previous name and this addition is visible as an
    // addition: a publication that borrows every parameter and returns no
    // custody, restricted to its declared literal-argument premise.
    old.insert_contract(
        "flb_hash_table_add",
        ResourceFunctionContract::new(
            [
                P::Borrowed,
                P::Borrowed,
                P::Borrowed,
                P::Borrowed,
                P::Borrowed,
            ],
            R::None,
        ),
    );
    // Card PROD-CONTRACTS adds five declarations that were NOT in the old
    // table, listed explicitly here for the same reason: the parity check below
    // still covers every previous name, and each addition is visible as an
    // addition rather than exempted from the comparison. All five borrow every
    // parameter and return no custody, which is the only v1 mode with the
    // caller-side effect each pinned body was read to have.
    for (name, parameters) in [
        ("flb_log_check", 1),
        ("flb_log_print", 4),
        ("flb_input_chunk_get_name", 1),
        ("flb_routes_mask_set_by_tag", 4),
        ("flb_routes_mask_get_size", 1),
        // Card NEIGHBOURS adds six more, listed explicitly for the same
        // reason and in the same shape: every parameter borrowed, no result
        // custody. `flb_time_now` takes no parameter at all, so its row is
        // the empty vector, and it is the first production declaration whose
        // C result is floating-point (`-> f64`).
        ("generate_chunk_name", 3),
        ("input_chunk_write_header", 4),
        ("flb_time_now", 0),
        ("cio_chunk_is_up", 1),
        ("cio_chunk_up_force", 1),
        ("cio_chunk_down", 1),
        // Card CONSTRUCTOR-RESIDUALS adds three more, listed explicitly for
        // the same reason and in the same shape. `__errno_location` takes no
        // parameter at all; `mk_list_add` is the first production declaration
        // whose callee **publishes** through an argument, and it is declared
        // `Borrowed` on exactly card HASH-CONTRACTS' measured ground — the
        // callee frees nothing and takes no custody, so it deletes no caller
        // obligation, and the publication itself stays unmodeled.
        ("__errno_location", 0),
        ("flb_errno_print", 3),
        // Card CONSTRUCTOR-RESIDUALS-ABI-TYPEDEF adds two, listed explicitly
        // for the same reason and in the same shape: every parameter borrowed,
        // no result custody. Both were UNWRITABLE before that card, because
        // card LOADER-ABI's declared-ABI reader would not commit to a union
        // typedef (`pthread_mutex_t *`) and withdrew them at every call site.
        ("pthread_mutex_lock", 1),
        ("pthread_mutex_unlock", 1),
        // Card W47-CONDVAR-JOIN-PUBLICATION: every parameter borrowed, no
        // result custody; the thread-synchronization obligation is a
        // call-site precondition, not a custody mode.
        ("pthread_cond_init", 2),
        ("pthread_cond_destroy", 1),
        ("pthread_cond_signal", 1),
        ("pthread_cond_wait", 2),
        ("pthread_cond_timedwait", 3),
        ("pthread_join", 2),
        // Card ABI-TYPEDEF-ADDRESS-OF-MEMBER adds the third declaration that
        // card wrote and then measured at payoff zero, now that the blocker it
        // named — the address-of-a-member actual refusing the statement before
        // any contract is consulted — is repaired. It is the corpus's first
        // declaration carrying an `fnptr` parameter (its `callback`); a
        // function designator is never a resource identity, so `Borrowed` is
        // both the only mode the loader admits there and the true one.
        ("msgpack_packer_init", 3),
        // Card W29-CORPUS-CONTRACTS adds two more, listed explicitly for the
        // same reason and in the same shape: every parameter borrowed, no
        // result custody. Both are pure size queries over the input chunk's
        // own state (`src/flb_input_chunk.c:756-759` and `:767-790`), and
        // their `ssize_t` result spells `isize` under the LP64 declared-ABI
        // projection.
        ("flb_input_chunk_get_size", 1),
        ("flb_input_chunk_get_real_size", 1),
        // Card W31-CORPUS-CONTRACTS-2 adds two more, listed explicitly for
        // the same reason and in the same shape: every parameter borrowed, no
        // result custody. Both are the size-sibling read-only queries card
        // W29-CORPUS-CONTRACTS named and declined
        // (`lib/chunkio/src/cio_chunk.c:315-334`,
        // `src/flb_input_chunk.c:2947-2955`); the first spells its `ssize_t`
        // result `isize` and the second its `size_t` result `usize` under the
        // LP64 declared-ABI projection.
        ("cio_chunk_get_content_size", 1),
        ("flb_input_chunk_total_size", 1),
        // Card W36-CORPUS-CONTRACTS-3 adds ONE to this all-borrowed loop:
        // Monkey's variadic diagnostic sink `mk_print`
        // (`lib/monkey/mk_core/mk_utils.c:200-264`), whose `int` level and
        // `const char *` format are both read-only and whose variadic tail is
        // consumed by `vprintf` into `stdout` — no tail argument freed, no
        // pointer to one retained past the call. The DECLARED arity is two;
        // the `...` is carried by `resource_variadic_data_tail()` and not by a
        // third parameter, exactly as `flb_log_print` above carries it at
        // arity four. The card's other five declarations release a parameter
        // and are therefore listed below the loop rather than inside it.
        ("mk_print", 2),
        // Card W37-CORPUS-CONTRACTS-4 adds TEN to this all-borrowed loop and
        // NONE below it — the exact inverse of W36 above (one in the loop,
        // five releases below), and the inversion is this card's finding
        // rather than a coincidence: W36 harvested the release-shaped
        // callees, and what the sole-blocker cover frontier still offered to
        // a NAME-KEYED declaration afterwards was custody-neutral. The
        // release-shaped candidates that remained
        // (`rd_kafka_topic_partition_destroy0`, `rd_kafka_destroy_app`,
        // `otlp_kvpair_list_destroy`, `destroy_context_list`) are all `static`
        // inside the very units that call them, where W4 and not declaration
        // is the lever; the readings are published in the card's report.
        //
        //   * `cfl_time_now` (`lib/cfl/src/cfl_time.c:31-60`) — ZERO
        //     parameters, `uint64_t` result. Its C header spells `()`,
        //     unprototyped, exactly as `flb_time_now`'s does, which is what
        //     settles the arity gate by precedent rather than by hope.
        //   * `cmt_metric_get_timestamp` / `cmt_metric_set_double`
        //     (`lib/cmetrics/src/cmt_metric.c:244-251`, `:71-81`) — the metric
        //     is read through / written through, never released. Writing
        //     through a borrowed pointer is `msgpack_packer_init`'s position.
        //   * `cmt_mpack_consume_uint_tag`
        //     (`lib/cmetrics/src/cmt_mpack_utils.c:88-113`) — its OUT
        //     parameter carries a SCALAR, not a resource identity, so it is
        //     borrowed and not a custody `Out`.
        //   * the four mpack writers (`lib/cmetrics/lib/mpack/src/mpack/
        //     mpack.c:2045`, `:1968`, `:1774`, `:1816`) — four vendored copies
        //     each, compared and found to AGREE character for character, which
        //     is what separates them from the `ADD-NAME-COLLISION` case.
        //   * `LZ4_resetStream` (`lib/librdkafka-2.15.0/src/lz4.c:1524-1528`)
        //     — zeroing is not releasing; the freeing neighbour is a different
        //     name, already decided through `rd_kafka_mem_free`.
        //   * `rd_kafka_topic_partition_list_sort`
        //     (`lib/librdkafka-2.15.0/src/rdkafka_partition.c:3664-3674`) —
        //     this card's highest yield at two functions; permuting an array
        //     the caller owns whole changes no custody, and `cmp` is an
        //     `fnptr`, `msgpack_packer_init`'s spelling.
        ("cfl_time_now", 0),
        ("cmt_metric_get_timestamp", 1),
        ("cmt_metric_set_double", 3),
        ("cmt_mpack_consume_uint_tag", 2),
        ("mpack_write_cstr", 2),
        ("mpack_write_str", 3),
        ("mpack_write_i64", 2),
        ("mpack_write_double", 2),
        ("LZ4_resetStream", 1),
        ("rd_kafka_topic_partition_list_sort", 3),
        // Card W43-LIBC-CONTRACTS adds 54 libc/POSIX declarations to this
        // all-borrowed loop, every one owned by `providers/quirin-provider-libc`
        // and each a trusted assumption read from the prototype the pinned
        // units declare: every parameter is read, written through or a scalar,
        // nothing is allocated, freed or retained, and every result is a
        // scalar or (`dlerror`, `gai_strerror`) a pointer to library-owned
        // storage the caller must not free. `prctl` additionally carries the
        // literal premise `option == 15` (`PR_SET_NAME`), which is not a mode
        // and so is not compared here. The card's one acquisition, `strdup`,
        // is listed below the loop.
        ("dlerror", 0),
        ("ceil", 1),
        ("floor", 1),
        ("fabs", 1),
        ("pow", 2),
        ("prctl", 1),
        ("setsockopt", 5),
        ("send", 4),
        ("recv", 4),
        ("listen", 2),
        ("sendfile", 4),
        ("htonl", 1),
        ("ntohl", 1),
        ("gai_strerror", 1),
        ("perror", 1),
        ("fflush", 1),
        ("remove", 1),
        ("atoi", 1),
        ("srand", 1),
        ("rand_r", 1),
        ("system", 1),
        ("strcmp", 2),
        ("strlen", 1),
        ("strerror_r", 3),
        ("fstat", 2),
        ("mkdir", 2),
        ("umask", 1),
        ("mtx_lock", 1),
        ("mtx_unlock", 1),
        ("cnd_wait", 2),
        ("pthread_rwlock_rdlock", 1),
        ("pthread_rwlock_wrlock", 1),
        ("pthread_rwlock_unlock", 1),
        ("pthread_self", 0),
        ("pthread_kill", 2),
        ("pthread_setcancelstate", 2),
        ("pthread_setcanceltype", 2),
        ("pthread_attr_setdetachstate", 2),
        ("sigemptyset", 1),
        ("sigfillset", 1),
        ("sigaddset", 2),
        ("time", 1),
        ("timegm", 1),
        ("sleep", 1),
        ("usleep", 1),
        ("sysconf", 1),
        ("getpagesize", 0),
        ("geteuid", 0),
        ("getegid", 0),
        ("setsid", 0),
        ("access", 2),
        ("unlink", 1),
        ("read", 3),
        ("write", 3),
        // Card W43-LIBC-RESTRICT-PROTOTYPES adds ten `__restrict` prototypes,
        // each the neutral element and each a trusted assumption: every
        // parameter is read or written through into caller storage, and every
        // result is a scalar or (`dlsym`) a symbol address the caller acquires
        // no obligation for.
        // (`regexec` and `regerror` are listed below since card
        // CG25-RESOURCE-KINDS kinds their `__preg`.)
        ("dlsym", 2),
        ("getsockopt", 5),
        ("getpeername", 3),
        ("stat", 2),
        ("cnd_timedwait", 3),
        ("pthread_sigmask", 3),
        ("strftime", 4),
        ("gettimeofday", 2),
        // Card W48-CALLEE-CLOSURE-R3: `stat`'s statement for `lstat`, and
        // `gettimeofday`'s for `clock_gettime` (a scalar clock id and a
        // written-through `struct timespec`), both the neutral element.
        ("lstat", 2),
        ("clock_gettime", 2),
        // Card CG21-CALL-EXITS-AND-BOUNDARIES: the three no-return exits in
        // `libc/stdlib.qv`. Their modes are the neutral element (a scalar
        // status, three read strings and a scalar line); the exit itself is
        // the separate `resource_no_return()` clause.
        ("abort", 0),
        ("exit", 1),
        ("__assert_fail", 4),
        // Card CONTRACT-NORETURN-AND-BORROWED-TAILS: librdkafka's
        // `rd_kafka_crash` (`librdkafka/memory.qv`), the same shape: four read
        // or scalar arguments and a borrowed, only-read `rk`.
        ("rd_kafka_crash", 5),
        // Card W43-LIBC-PRINTF-FAMILY: the formatted-output family. Every
        // declared parameter is borrowed (a written-through buffer or stream
        // the caller keeps, a size, a read format), the result is an `int`
        // count, and the `...` is `resource_variadic_data_tail()`, carried by
        // no parameter, exactly as `mk_print` above.
        ("printf", 1),
        ("fprintf", 2),
        ("snprintf", 3),
        ("sprintf", 2),
        // Card W44-LIBC-RESIDUALS: the two called `va_list` members (the
        // `va_list` is borrowed data) and `strerror` (a scalar in, a message
        // the caller must not free).
        ("vsnprintf", 4),
        ("vprintf", 2),
        ("strerror", 1),
        // Card W46-EXTERNAL-CALLEE-CONTRACTS: nine external callees, each the
        // neutral element read off its own body at both audited revisions.
        // `fcntl` carries the literal premise `cmd == 2` (`F_SETFD`) and a
        // variadic data tail, neither of which is a mode; the other eight are
        // pinned in their own library crates.
        ("fcntl", 2),
        ("mk_utils_libc_error", 3),
        ("rd_atomic32_get", 1),
        ("cfl_sds_len", 1),
        ("mpack_finish_map", 1),
        ("mpack_finish_array", 1),
        ("mpack_write_true", 1),
        ("mpack_write_false", 1),
        ("mpack_write_bin", 3),
        // W46-METRIC-TRACE-CONTRACT-GAPS: all selected metric and trace
        // helpers have neutral custody, including scalar field writes.
        ("cmt_metric_get_value", 1),
        ("cmt_metric_get_value_type", 1),
        ("cmt_metric_get_int64_value", 1),
        ("cmt_metric_get_uint64_value", 1),
        ("cmt_metric_set_int64", 3),
        ("cmt_metric_set_uint64", 3),
        ("cmt_metric_set_timestamp", 2),
        ("cmt_metric_set_start_timestamp", 2),
        ("cmt_metric_unset_start_timestamp", 1),
        ("cmt_metric_has_start_timestamp", 1),
        ("cmt_metric_get_start_timestamp", 1),
        ("cmt_math_d64_to_uint64", 1),
        ("cmt_math_uint64_to_d64", 1),
        ("ctr_id_cmp", 2),
        ("ctr_id_get_len", 1),
        ("ctr_span_set_flags", 2),
        ("ctr_span_set_dropped_events_count", 2),
        ("ctr_span_set_dropped_links_count", 2),
        // Card CG10-INDIRECT-ACCESS-SOLE-RESIDUALS: seven neutral-element
        // declarations read off their bodies at both audited revisions
        // (`cmetrics/metric_values.qv`, `librdkafka/memory.qv`).
        ("cmt_atomic_load", 1),
        ("cmt_atomic_store", 2),
        ("cmt_atomic_compare_exchange", 3),
        ("cnd_timedwait_ms", 3),
        ("rd_hdr_histogram_record", 2),
        ("rd_atomic64_add", 2),
        ("rd_atomic64_get", 1),
        // Card W45-CALLEE-CASCADE-RESIDUAL: nineteen neutral-element
        // declarations read off their bodies at both audited revisions
        // (`librdkafka/lz4_stream.qv`, `librdkafka/kafka_mem.qv`,
        // `ctraces/context_lifetime.qv`, `cfl/sds_lifetime.qv`,
        // `chunkio/chunk_residency.qv`); its two owned-result declarations
        // are listed after this loop.
        ("LZ4_compress_HC", 5),
        ("LZ4_decompress_unsafe_generic", 6),
        ("LZ4_compress_default", 4),
        ("load_descriptor_values", 1),
        ("rd_kafka_aborted_txns_next_offset", 4),
        ("rd_kafka_anyconf_desensitize", 2),
        ("rd_kafka_dump0", 3),
        ("ctr_mpack_consume_uint_tag", 2),
        ("ctr_mpack_consume_int_tag", 2),
        ("ctr_attributes_set_string", 3),
        ("ctr_attributes_set_bool", 3),
        ("ctr_attributes_set_int64", 3),
        ("ctr_attributes_set_double", 3),
        ("cfl_kvlist_count", 1),
        ("cfl_kvlist_insert_string", 3),
        ("cfl_kvlist_insert_bool", 3),
        ("cfl_kvlist_insert_int64", 3),
        ("cfl_kvlist_insert_double", 3),
        ("_cio_file_up", 2),
        ("ctr_span_set_dropped_attributes_count", 2),
        // W46-METRIC-AGGREGATE-CONTRACTS: scalar reads of histogram and
        // summary values. The indexed getters borrow their index too.
        ("cmt_metric_hist_get_value", 2),
        ("cmt_metric_hist_get_count_value", 1),
        ("cmt_metric_hist_get_sum_value", 1),
        ("cmt_summary_quantile_get_value", 2),
        ("cmt_summary_get_sum_value", 1),
        ("cmt_summary_get_count_value", 1),
        // W46-LIBC-EVENT-CONTRACT-GAPS: eleven neutral catalog and pinned
        // observation contracts, including the zero-argument Libevent queries.
        ("memcmp", 3),
        ("strncmp", 3),
        ("strnlen", 2),
        ("strcasecmp", 2),
        ("_mk_string_search", 4),
        ("mk_string_char_search", 3),
        ("mk_string_char_search_r", 3),
        ("MK_EVENT_NEW", 1),
        ("event_base_get_features", 1),
        ("event_get_struct_event_size", 0),
        ("event_get_version_number", 0),
    ] {
        old.insert_contract(
            name,
            ResourceFunctionContract::new(vec![P::Borrowed; parameters], R::None),
        );
    }
    // Card ADDRESS-OF-MEMBER-DESTROY-UNDECLARED adds the first production
    // declaration of this family that is NOT all-borrowed, so it is listed
    // here rather than in the loop above: `flb_input_chunk_destroy` RELEASES
    // its first parameter. The pinned body (`src/flb_input_chunk.c:2704-2807`)
    // has exactly one exit and reaches `flb_free(ic)` unconditionally on it,
    // and its first statement dereferences `ic`, so `nonnull` — not the
    // `nullable` that `flb_free`'s own declaration carries, since `free(NULL)`
    // is defined and this is not that. `del` is an `int` by value, the only
    // mode the loader admits on a scalar and also the true one.
    old.insert_contract(
        "flb_input_chunk_destroy",
        ResourceFunctionContract::new([P::Released { nullable: false }, P::Borrowed], R::None),
    );
    // Card AOM-DESTROY-SIBLINGS adds the second non-all-borrowed production
    // declaration, and it is listed beside its neighbour deliberately, because
    // the two DISAGREE on the conjunct that decides every call site. This body
    // (`src/flb_input_chunk.c:2483-2504`) guards its own null argument —
    // `if (!routes) { return; }` at `:2490` — so the release is `nullable`,
    // the position `flb_free` holds and for the same reason. Copying the
    // neighbour's `nonnull` would assert a precondition this body does not
    // have. `route_count` is an `int` by value, and the result is `void`.
    old.insert_contract(
        "flb_input_chunk_destroy_direct_routes",
        ResourceFunctionContract::new([P::Released { nullable: true }, P::Borrowed], R::None),
    );
    old.insert_contract(
        "rd_realloc",
        ResourceFunctionContract::new(
            [P::Owned { nullable: true }, P::Borrowed],
            R::Reallocated {
                parameter: 0,
                nullable: true,
            },
        ),
    );
    // Card W36-CORPUS-CONTRACTS-3 adds five releasing declarations, listed
    // here rather than in the all-borrowed loop and listed one by one so each
    // nullability marker is visible as its own decision. The markers are NOT
    // uniform, and that is the point: each was read off its own pinned body.
    //
    //   * `cfl_sds_destroy` (`lib/cfl/src/cfl_sds.c:249-275`) — `nullable`,
    //     because the body's FIRST statement is `if (!s) { return; }`. It is
    //     the highest function-cover callee in the baseline.
    //   * `cJSON_Delete` (`lib/librdkafka-2.15.0/src/cJSON.c:253-276`) —
    //     `nullable`, because `while (item != NULL)` is evaluated before any
    //     dereference. The null path is REACHED at all three call sites, which
    //     pass a `cJSON_DetachItem…` result that is NULL when not found.
    //   * `cmt_destroy` (`lib/cmetrics/src/cmetrics.c:92-146`) and
    //     `ctr_destroy` (`lib/ctraces/src/ctraces.c:58-71`) — `nonnull`, the
    //     opposite reading on the opposite evidence: neither body has a
    //     leading null guard, and each dereferences its parameter in its first
    //     executable statement. This is `flb_input_chunk_destroy`'s position
    //     above, for `flb_input_chunk_destroy`'s reason.
    //   * `rd_kafka_mem_free` (`lib/librdkafka-2.15.0/src/rdkafka.c:7131-7133`)
    //     is the only two-parameter one: `rk` is read NOWHERE in the body, so
    //     it is `Borrowed` and is deliberately not asserted non-null (LZ4's
    //     `FREEMEM` passes a null handle); `ptr` is forwarded to `rd_free`,
    //     whose own declaration already carries `nullable`, so the marker is
    //     inherited from the delegate rather than guessed.
    old.insert_contract(
        "cfl_sds_destroy",
        ResourceFunctionContract::new([P::Released { nullable: true }], R::None),
    );
    old.insert_contract(
        "cJSON_Delete",
        ResourceFunctionContract::new([P::Released { nullable: true }], R::None),
    );
    old.insert_contract(
        "cmt_destroy",
        ResourceFunctionContract::new([P::Released { nullable: false }], R::None),
    );
    old.insert_contract(
        "ctr_destroy",
        ResourceFunctionContract::new([P::Released { nullable: false }], R::None),
    );
    old.insert_contract(
        "rd_kafka_mem_free",
        ResourceFunctionContract::new([P::Borrowed, P::Released { nullable: true }], R::None),
    );
    // Card W43-LIBC-CONTRACTS: `strdup` reads its argument and returns a
    // fresh `malloc` allocation or NULL -- `malloc`'s own result mode, from the
    // allocator `free` releases.
    old.insert_contract(
        "strdup",
        ResourceFunctionContract::new([P::Borrowed], R::Owned { nullable: true }),
    );
    // Card W43-LIBC-RESULT-ALIAS: thirteen libc declarations whose result is
    // DERIVED from one borrowed argument and owns nothing. The old table had no
    // such mode, so each is listed with the mode the card reads off the
    // prototype: `const_dropped` exactly where a `const` base yields a mutable
    // result.
    for (name, parameters, base, extent, nullable, const_dropped) in [
        ("strcpy", 2, 0, DerivedExtent::Exact, false, false),
        ("strncpy", 3, 0, DerivedExtent::Exact, false, false),
        ("memcpy", 3, 0, DerivedExtent::Exact, false, false),
        ("memmove", 3, 0, DerivedExtent::Exact, false, false),
        ("memset", 3, 0, DerivedExtent::Exact, false, false),
        ("gmtime_r", 2, 1, DerivedExtent::Exact, true, false),
        ("localtime_r", 2, 1, DerivedExtent::Exact, true, false),
        ("strchr", 2, 0, DerivedExtent::Interior, true, true),
        ("strrchr", 2, 0, DerivedExtent::Interior, true, true),
        ("strstr", 2, 0, DerivedExtent::Interior, true, true),
        ("strcasestr", 2, 0, DerivedExtent::Interior, true, true),
        ("memchr", 3, 0, DerivedExtent::Interior, true, true),
        ("memrchr", 3, 0, DerivedExtent::Interior, true, true),
    ] {
        old.insert_contract(
            name,
            ResourceFunctionContract::new(
                vec![P::Borrowed; parameters],
                R::Derived {
                    parameter: base,
                    extent,
                    nullable,
                    const_dropped,
                },
            ),
        );
    }
    // Card W47-CALLEE-CLOSURE-R2: two external callees read off their bodies
    // at both audited revisions. `cfl_arena_calloc`'s result is interior to
    // its borrowed arena (a zeroed allocation inside a chunk the arena owns and
    // `cfl_arena_destroy` frees), never `const`-dropped; `rd_file_read`'s is a
    // fresh `rd_malloc` buffer or NULL, which `rd_free` releases.
    old.insert_contract(
        "cfl_arena_calloc",
        ResourceFunctionContract::new(
            vec![P::Borrowed; 3],
            R::Derived {
                parameter: 0,
                extent: DerivedExtent::Interior,
                nullable: true,
                const_dropped: false,
            },
        ),
    );
    old.insert_contract(
        "rd_file_read",
        ResourceFunctionContract::new(vec![P::Borrowed; 3], R::Owned { nullable: true }),
    );
    // Card CONTRACT-INHERIT-ADOPTION: `strndup` is `strdup` with a borrowed
    // size (the trailing borrow is canonicalised away below).
    old.insert_contract(
        "strndup",
        ResourceFunctionContract::new([P::Borrowed], R::Owned { nullable: true }),
    );
    // And `flb_strdup` / `flb_strndup`, which inherit exactly those two.
    for name in ["flb_strdup", "flb_strndup"] {
        old.insert_contract(
            name,
            ResourceFunctionContract::new([P::Borrowed], R::Owned { nullable: true }),
        );
    }
    // Card W43-DESCRIPTOR-RESOURCE-TRACKING: eight kinded libc declarations. The
    // old table had no kind, so each is listed with the kind the card reads off
    // the prototype; the two `descriptor` results are `sentinel` (may be `-1`),
    // the `descriptor` release is `valid`, and `fileno` is the neutral element.
    for (name, parameters, kind) in [
        ("open", 2, ResourceKind::Descriptor),
        ("socket", 3, ResourceKind::Descriptor),
        ("fopen", 2, ResourceKind::Stream),
        ("opendir", 1, ResourceKind::Directory),
    ] {
        old.insert_contract(
            name,
            ResourceFunctionContract::new(
                vec![P::Borrowed; parameters],
                R::Owned { nullable: true },
            )
            .with_result_kind(kind),
        );
    }
    for (name, kind) in [
        ("close", ResourceKind::Descriptor),
        ("fclose", ResourceKind::Stream),
        ("closedir", ResourceKind::Directory),
    ] {
        old.insert_contract(
            name,
            ResourceFunctionContract::new([P::Released { nullable: false }], R::None)
                .with_parameter_kind(0, kind),
        );
    }
    old.insert_contract(
        "fileno",
        ResourceFunctionContract::new([P::Borrowed], R::None),
    );
    // Card CG25-RESOURCE-KINDS: `dlopen`/`dlclose` with the `dynamic_library`
    // kind, `regcomp`/`regfree` with the in-place `regex` kind (an initialiser
    // whose status is its result, and a finaliser), and `regexec`/`regerror`,
    // still neutral, with their `__preg` kinded as a use of the object.
    old.insert_contract(
        "dlopen",
        ResourceFunctionContract::new(vec![P::Borrowed; 2], R::Owned { nullable: true })
            .with_result_kind(ResourceKind::DynamicLibrary),
    );
    old.insert_contract(
        "dlclose",
        ResourceFunctionContract::new([P::Released { nullable: false }], R::None)
            .with_parameter_kind(0, ResourceKind::DynamicLibrary),
    );
    old.insert_contract(
        "regcomp",
        ResourceFunctionContract::new(
            [P::Out { nullable: true }, P::Borrowed, P::Borrowed],
            R::None,
        )
        .with_parameter_kind(0, ResourceKind::Regex),
    );
    old.insert_contract(
        "regfree",
        ResourceFunctionContract::new([P::Released { nullable: false }], R::None)
            .with_parameter_kind(0, ResourceKind::Regex),
    );
    for (name, arity) in [("regexec", 5), ("regerror", 4)] {
        let position = usize::from(name == "regerror");
        old.insert_contract(
            name,
            ResourceFunctionContract::new(vec![P::Borrowed; arity], R::None)
                .with_parameter_kind(position, ResourceKind::Regex),
        );
    }
    // Card W45-FB-BACKLOG-CUSTODY: the twelve checked specifications of
    // `fluent_bit/storage_backlog.qv`; ten borrow every argument and return
    // no custody (the canonical form below drops their trailing `Borrowed`s).
    old.insert_contract(
        "sb_allocate_chunk",
        ResourceFunctionContract::new([], R::Owned { nullable: true }),
    );
    old.insert_contract(
        "sb_destroy_chunk",
        ResourceFunctionContract::new([P::Released { nullable: false }], R::None),
    );
    for name in [
        "sb_destroy_backlog",
        "sb_allocate_backlogs",
        "sb_destroy_backlogs",
        "sb_remove_chunk_from_segregated_backlog",
        "sb_remove_chunk_from_segregated_backlogs",
        "sb_append_chunk_to_segregated_backlog",
        "sb_append_chunk_to_segregated_backlogs",
        "sb_segregate_chunks",
        "sb_release_output_queue_space",
        "cb_queue_chunks",
    ] {
        old.insert_contract(name, ResourceFunctionContract::new([], R::None));
    }
    // Card W45-FB-RETRY-LIFETIME: `mk_list_add` keeps its borrowed modes and
    // gains the publication its body performs (contract specification
    // C04.8); `mk_list_del` is its new converse. The Fluent Bit counter
    // operations, the two neutral callees and the two checked obligation
    // declarations are additions, listed explicitly so the parity check
    // below still covers every previous name.
    {
        use qcore_schemas::contract::{ContainerEffect, UserEffect, UserExitClause, UserOperation};
        let mut list_add = ResourceFunctionContract::new([P::Borrowed, P::Borrowed], R::None);
        list_add.container_effects.insert(
            0,
            ContainerEffect::Published {
                container_parameter: 1,
            },
        );
        old.insert_contract("mk_list_add", list_add);
        let mut list_del = ResourceFunctionContract::new([P::Borrowed], R::None);
        list_del
            .container_effects
            .insert(0, ContainerEffect::Unpublished);
        old.insert_contract("mk_list_del", list_del);
        // Card W48-SCHED-CLEANUP-PRODUCTION (gateway rule 34): the element
        // destroyer `mk_http_thread_destroy` unlinks its object and releases
        // it, an addition listed explicitly.
        let mut thread_destroy =
            ResourceFunctionContract::new([P::Released { nullable: false }], R::None);
        thread_destroy
            .container_effects
            .insert(0, ContainerEffect::Unpublished);
        old.insert_contract("mk_http_thread_destroy", thread_destroy);
        let operation = |effect| UserOperation {
            effect,
            counter: "users".to_owned(),
        };
        let mut inc = ResourceFunctionContract::new([P::Borrowed], R::None);
        inc.counted_users
            .effects
            .insert(0, operation(UserEffect::Added));
        old.insert_contract("flb_task_users_inc", inc);
        let mut dec = ResourceFunctionContract::new([P::Borrowed, P::Borrowed], R::None);
        dec.counted_users
            .effects
            .insert(0, operation(UserEffect::Dropped));
        old.insert_contract("flb_task_users_dec", dec);
        let mut retry = ResourceFunctionContract::new([P::Released { nullable: false }], R::None);
        retry.counted_users.neutral = true;
        old.insert_contract("flb_task_retry_destroy", retry);
        let mut pool =
            ResourceFunctionContract::new([P::Borrowed, P::Borrowed, P::Borrowed], R::None);
        pool.counted_users.neutral = true;
        old.insert_contract("flb_output_thread_pool_flush", pool);
        let exits = |task: usize| {
            vec![
                UserExitClause {
                    parameter: task,
                    returned: -1,
                    delta: 0,
                },
                UserExitClause {
                    parameter: task,
                    returned: 0,
                    delta: 1,
                },
            ]
        };
        let mut enqueue = ResourceFunctionContract::new(
            [
                P::Borrowed,
                P::Consumed { nullable: true },
                P::Borrowed,
                P::Borrowed,
                P::Borrowed,
            ],
            R::None,
        );
        enqueue.counted_users.exits = exits(2);
        old.insert_contract("flb_output_task_singleplex_enqueue", enqueue);
        let mut flush =
            ResourceFunctionContract::new([P::Borrowed, P::Borrowed, P::Borrowed], R::None);
        flush.counted_users.exits = exits(0);
        old.insert_contract("flb_output_task_flush", flush);
    }
    // W46-FLUENT-EVENT-CONTRACT-GAPS and W46-FLUENT-ROUTING-QUERY-CONTRACTS:
    // scalar/state helpers borrow every argument and preserve task-user
    // counts. This explicit previous-mode table grows with the manifest.
    for (name, arity) in [
        ("flb_input_chunk_is_up", 1),
        ("input_chunk_has_magic_bytes", 2),
        ("flb_input_chunk_is_mem_overlimit", 1),
        ("flb_input_chunk_is_storage_overlimit", 1),
        ("flb_routes_mask_get_slots", 1),
        ("flb_routes_mask_is_empty", 2),
        ("flb_output_flush_id_get", 1),
        ("flb_task_get_route_status", 2),
        ("flb_task_set_route_status", 3),
        ("flb_task_retry_count", 2),
        ("flb_task_running_count", 1),
        // W47-LOOP-OWNER-CONTRACTS: the four collector operations borrow both
        // arguments; their loop-owner obligation is not a custody mode.
        ("flb_input_collector_running", 2),
        ("flb_input_collector_pause", 2),
        ("flb_input_collector_resume", 2),
        ("flb_input_collector_delete", 2),
    ] {
        let mut contract = ResourceFunctionContract::new(vec![P::Borrowed; arity], R::None);
        contract.counted_users.neutral = true;
        old.insert_contract(name, contract);
    }
    // CONTRACT-PIN-TIP-MODE: `mk_vhost_set` borrows both named arguments
    // (its borrowed variadic tail is no parameter mode).
    old.insert_contract(
        "mk_vhost_set",
        ResourceFunctionContract::new([P::Borrowed, P::Borrowed], R::None),
    );
    // MONKEY-VHOST-PROPERTY-CUSTODY: `mk_string_dup` borrows its string and
    // returns an owned, nullable copy (`strdup`'s custody statement).
    old.insert_contract(
        "mk_string_dup",
        ResourceFunctionContract::new([P::Borrowed], R::Owned { nullable: true }),
    );
    // W45-CALLEE-CASCADE-RESIDUAL: `cJSON_New_Item` borrows its hook table and
    // `rd_kafka_metadata_copy_internal` its source and scalars; each returns
    // a fresh owned, nullable allocation.
    old.insert_contract(
        "cJSON_New_Item",
        ResourceFunctionContract::new([P::Borrowed], R::Owned { nullable: true }),
    );
    old.insert_contract(
        "rd_kafka_metadata_copy_internal",
        ResourceFunctionContract::new(
            [P::Borrowed, P::Borrowed, P::Borrowed],
            R::Owned { nullable: true },
        ),
    );
    // W43-ABI-ENUM-AND-CONST-PP: declarations over enumeration and
    // pointer-to-pointer-to-const prototypes (rules 12u, 12v). Every parameter
    // is borrowed; five return a fresh owned, nullable allocation and
    // `rd_flags2str` returns its `dst` (an exact, never-null alias). The two
    // retaining ones are withheld into their own table above.
    for name in [
        "LZ4_decompress_generic",
        "rd_kafka_err_action",
        "rd_kafka_oauthbearer_set_token_failure",
        "rd_kafka_toppars_pause_resume",
        "rd_kafka_admin_add_config0",
        "getrlimit",
        "setrlimit",
    ] {
        old.insert_contract(name, ResourceFunctionContract::new([], R::None));
    }
    for name in [
        "rd_kafka_error_new_v",
        "rd_kafka_error_new",
        "rd_kafka_AclBinding_new0",
        "rd_kafka_ConsumerGroupDescription_new",
        "rd_kafka_anyconf_dump",
    ] {
        old.insert_contract(
            name,
            ResourceFunctionContract::new([], R::Owned { nullable: true }),
        );
    }
    old.insert_contract(
        "rd_flags2str",
        ResourceFunctionContract::new(
            vec![P::Borrowed; 4],
            R::Derived {
                parameter: 0,
                extent: DerivedExtent::Exact,
                nullable: false,
                const_dropped: false,
            },
        ),
    );
    // Card SLOT-MOVE-AND-INTERIOR-BORROW: the borrowed results (class 8)
    // had no previous reading; each is the borrowed base and the result.
    for (name, parameter, nullable, const_dropped) in [
        ("rd_kafka_admin_result_ret_topics", 0, true, true),
        ("rd_kafka_admin_result_ret_resources", 0, true, true),
        ("rd_kafka_admin_result_ret_groups", 0, true, true),
        ("rd_kafka_topic_partition_get_private", 0, false, false),
        ("get_object_item", 0, true, true),
    ] {
        old.insert_contract(
            name,
            ResourceFunctionContract::new(
                [],
                R::Derived {
                    parameter,
                    extent: DerivedExtent::Borrowed,
                    nullable,
                    const_dropped,
                },
            ),
        );
    }
    // Card REFCOUNT-RESOURCE-KIND: the queue handle constructor and six
    // destructors had no previous reading; each is its plain v1 mode (the
    // keeps and drops are withheld, below).
    old.insert_contract(
        "rd_kafka_queue_new0",
        ResourceFunctionContract::new([], R::Owned { nullable: false }),
    );
    for name in [
        "rd_kafka_topic_partition_list_destroy",
        "rd_kafka_topic_partition_destroy0",
        "rd_kafka_AlterConsumerGroupOffsets_destroy",
        "rd_kafka_DeleteConsumerGroupOffsets_destroy",
        "rd_kafka_ListConsumerGroupOffsets_destroy",
        "rd_kafka_topic_partition_result_destroy",
    ] {
        old.insert_contract(
            name,
            ResourceFunctionContract::new([P::Released { nullable: false }], R::None),
        );
    }
    // Card REFCOUNT-REFERENCE-RETURNING-DECLARATIONS: the returned references
    // with no other clause had no previous reading; each is an owned result
    // of the reference kind (`rd_kafka_q_new0` retains, and is withheld).
    for (name, nullable) in [
        ("rd_kafka_toppar_get0", true),
        ("rd_kafka_toppar_desired_get", true),
        ("rd_kafka_toppar_new0", false),
        ("rd_kafka_toppar_desired_add", false),
        ("rd_kafka_toppar_get2", true),
        ("rd_kafka_toppar_get_avail", true),
        ("ut_create_mock_toppar", true),
        ("ut_create_mock_queue", false),
    ] {
        old.insert_contract(
            name,
            ResourceFunctionContract::new([], R::Owned { nullable })
                .with_result_kind(ResourceKind::Reference),
        );
    }
    // Card REFCOUNT-FAMILY-BLOCKING-CALLEES: four plain declarations between a
    // reference's acquisition and its drop had no previous reading; each
    // borrows every parameter and returns no custody.
    for name in [
        "rd_kafka_q_wait_result",
        "rd_kafka_q_purge0",
        "rd_kafka_toppar_needs_query",
        "rd_kafka_toppar_fetch_backoff",
    ] {
        old.insert_contract(name, ResourceFunctionContract::new([], R::None));
    }
    // Card CG10-INDIRECT-ACCESS-SOLE-RESIDUALS round 2: two plain neutral
    // declarations, `chunkio/chunk_residency.qv` and `ctraces/context_lifetime.qv`;
    // each borrows every parameter and returns no custody.
    for name in ["cio_errno_print", "ctr_errno_print"] {
        old.insert_contract(name, ResourceFunctionContract::new([], R::None));
    }
    // Card W45-CALLEE-CASCADE-RESIDUAL round 3: thirteen declarations in the
    // function map had no previous reading (`unit_test_telemetry`, a callback
    // declaration, is withheld with the retaining ones): five Admin-API
    // destructors and `mk_fifo_queue_destroy` release their object, six are
    // neutral, and `rd_kafka_mem_calloc` returns `rd_calloc`'s owned result.
    for (name, nullable) in [
        ("rd_kafka_AclBinding_destroy", false),
        ("rd_kafka_ConfigResource_destroy", false),
        ("rd_kafka_NewPartitions_destroy", false),
        ("rd_kafka_NewTopic_destroy", false),
        ("rd_kafka_UserScramCredentialAlteration_destroy", true),
    ] {
        old.insert_contract(
            name,
            ResourceFunctionContract::new([P::Released { nullable }], R::None),
        );
    }
    old.insert_contract(
        "mk_fifo_queue_destroy",
        ResourceFunctionContract::new([P::Borrowed, P::Released { nullable: false }], R::None),
    );
    for name in [
        "rd_bytes_hash",
        "cfl_list_size",
        "flb_hash_table_del",
        "cio_file_native_get_size",
        "cio_stream_size_chunks_up",
        "cmt_atomic_initialize",
    ] {
        old.insert_contract(name, ResourceFunctionContract::new([], R::None));
    }
    old.insert_contract(
        "rd_kafka_mem_calloc",
        ResourceFunctionContract::new([], R::Owned { nullable: true }),
    );
    // Card OP-CALLBACK-CANDIDATE-CONTRACTS: nineteen op-callback candidates
    // had no previous reading. The seventeen `rko_op_cb` candidates borrow
    // every formal and return no custody; the two `rko_serve` candidates take
    // the op they are handed (`rko`, the third formal) on every return.
    for name in [
        "rd_kafka_broker_monitor_op_cb",
        "rd_kafka_cgrp_assignor_handle_Metadata_op",
        "rd_kafka_cgrp_consumer_handle_Metadata_op",
        "rd_kafka_cgrp_handle_Metadata_op",
        "rd_kafka_offset_reset_op_cb",
        "rd_kafka_offset_validate_op_cb",
        "rd_kafka_share_ack_commit_cb_register_op",
        "rd_kafka_share_fetch_reply_op_cb",
        "rd_kafka_txn_op_init_transactions",
        "rd_kafka_txn_op_ack_init_transactions",
        "rd_kafka_txn_op_begin_transaction",
        "rd_kafka_txn_op_begin_commit",
        "rd_kafka_txn_op_commit_transaction",
        "rd_kafka_txn_op_commit_transaction_ack",
        "rd_kafka_txn_op_begin_abort",
        "rd_kafka_txn_op_abort_transaction",
        "rd_kafka_txn_op_abort_transaction_ack",
    ] {
        old.insert_contract(name, ResourceFunctionContract::new([], R::None));
    }
    for name in ["rd_kafka_cgrp_op_serve", "rd_kafka_mock_cluster_op_serve"] {
        old.insert_contract(
            name,
            ResourceFunctionContract::new(
                [P::Borrowed, P::Borrowed, P::Consumed { nullable: false }],
                R::None,
            ),
        );
    }
    // Card OP-APP-RK-CALLBACKS-VIA-POLL-CB: the run-closed candidates of the
    // application's rk_conf callbacks, every parameter borrowed.
    for name in [
        "rd_kafka_log_print",
        "rd_kafka_oauthbearer_unsecured_token",
        "rd_kafka_oidc_token_jwt_bearer_refresh_cb",
        "rd_kafka_oidc_token_client_credentials_refresh_cb",
        "rd_kafka_oidc_token_metadata_azure_imds_refresh_cb",
        "rd_kafka_oidc_token_metadata_aws_iam_refresh_cb",
        "cb_kafka_msg",
        "cb_kafka_logger",
        "oauthbearer_token_refresh_cb",
    ] {
        old.insert_contract(name, ResourceFunctionContract::new([], R::None));
    }
    // Card FB-EVENT-LOOP-RETENTION-AND-REGISTRATION: Monkey's channel
    // destroy (no removal: its condition is unstated) and wait, every
    // parameter borrowed.
    for name in ["mk_event_channel_destroy", "mk_event_wait"] {
        old.insert_contract(name, ResourceFunctionContract::new([], R::None));
    }
    // Card QR-PRINT-ROUTE-MATCHES-DECLARATION: cJSON's static `print` returns
    // a fresh `hooks->allocate` buffer or NULL.
    old.insert_contract(
        "print",
        ResourceFunctionContract::new([], R::Owned { nullable: true }),
    );
    // Card FB-TEARDOWN-DESTROY-DECLARATIONS: the four teardown declarations
    // with no member-path state (the ten with states are retaining).
    for name in [
        "flb_sched_timer_destroy",
        "flb_task_queue_destroy",
        "flb_input_collector_destroy",
    ] {
        old.insert_contract(
            name,
            ResourceFunctionContract::new([P::Released { nullable: false }], R::None),
        );
    }
    old.insert_contract(
        "flb_input_chunk_release_route",
        ResourceFunctionContract::new([], R::None),
    );
    // Card RETENTION-UNMATCHED-REMOVAL-POISONS: Monkey's loop create (a fresh
    // loop or NULL) and destroy (consumes a non-null loop), plain
    // declarations.
    old.insert_contract(
        "mk_event_loop_create",
        ResourceFunctionContract::new([], R::Owned { nullable: true }),
    );
    old.insert_contract(
        "mk_event_loop_destroy",
        ResourceFunctionContract::new([P::Released { nullable: false }], R::None),
    );
    // Card W48-RK-LOG-CALLER-FRAME: librdkafka's logger pair, every parameter
    // borrowed, plain declarations.
    for name in ["rd_kafka_log_buf", "rd_kafka_log0"] {
        old.insert_contract(name, ResourceFunctionContract::new([], R::None));
    }
    // W48-BU-EXTERNAL-CALLEE-CONTRACTS deliberately extends this expected
    // table: borrowed writev arguments and the scalar current-thread identifier.
    old.insert_contract(
        "writev",
        ResourceFunctionContract::new([P::Borrowed, P::Borrowed, P::Borrowed], R::None),
    );
    old.insert_contract("thrd_current", ResourceFunctionContract::new([], R::None));
    // Checked same-item detach deliberately extends the production mode table:
    // the result restores item (formal 1) from parent (formal 0), never fresh.
    old.insert_contract(
        "cJSON_DetachItemViaPointer",
        ResourceFunctionContract::new(
            [
                P::Owned { nullable: false },
                P::Removed {
                    owner_parameter: 0,
                    nullable: true,
                },
            ],
            R::Removed {
                parameter: 1,
                nullable: true,
            },
        ),
    );
    // Card W48-RK-LOG-CALLER-FRAME (rule 32): the logger's caller-frame
    // declarations, every formal borrowed, no result.
    for name in ["rd_kafka_log0", "rd_kafka_log_buf"] {
        old.insert_contract(name, ResourceFunctionContract::new([], R::None));
    }
    // Card CFL-MPACK-EXTERNAL-DECLARATIONS: fifty plain declarations read off
    // the CFL and mpack bodies at 66910c10, 046accf6 and 34155955 (the four
    // return-code dispositions and the `cfl_sds_printf` slot move are withheld
    // into their own tables and skipped below). Owned results are the
    // nullable heap constructors; releases are the nullable destructors;
    // `cfl_sds_cat` is `realloc`'s reallocation of its owned formal 0;
    // `cfl_kvlist_fetch` is borrowed from its list (formal 0).
    for (name, parameters) in [
        ("cfl_sds_create_size", 1),
        ("cfl_sds_create_len", 2),
        ("cfl_sds_create", 1),
        ("cfl_kvlist_create", 0),
        ("cfl_array_create", 1),
        ("cfl_variant_create_from_bool", 1),
        ("cfl_variant_create_from_int64", 1),
        ("cfl_variant_create_from_double", 1),
        ("cfl_variant_create_from_string", 1),
        ("cfl_utils_split", 3),
    ] {
        old.insert_contract(
            name,
            ResourceFunctionContract::new(
                vec![P::Borrowed; parameters],
                R::Owned { nullable: true },
            ),
        );
    }
    for name in [
        "cfl_kvlist_destroy",
        "cfl_array_destroy",
        "cfl_variant_destroy",
        "cfl_utils_split_free",
    ] {
        old.insert_contract(
            name,
            ResourceFunctionContract::new([P::Released { nullable: true }], R::None),
        );
    }
    for name in [
        "cfl_sds_set_len",
        "cfl_sds_len_set",
        "cfl_kvlist_insert_uint64",
        "cfl_kvlist_insert_bytes",
        "cfl_array_append_string",
        "cfl_array_append_bytes",
        "cfl_array_append_bool",
        "cfl_array_append_int64",
        "cfl_array_append_double",
        "mpack_start_map",
        "mpack_start_array",
        "mpack_write_nil",
        "mpack_write_u64",
        "mpack_write_u32",
        "mpack_write_i32",
        "mpack_write_int",
        "mpack_write_uint",
        "mpack_writer_flag_error",
        "mpack_tag_type",
        "mpack_tag_int_value",
        "mpack_tag_uint_value",
        "mpack_tag_double_value",
        "mpack_tag_array_count",
        "mpack_tag_map_count",
        "mpack_tag_str_length",
        "mpack_tag_bin_length",
        "mpack_done_array",
        "mpack_done_map",
        "mpack_done_str",
        "mpack_done_bin",
        "mpack_reader_destroy",
        "mpack_expect_nil",
        "mpack_read_bytes",
        "mpack_read_cstr",
    ] {
        old.insert_contract(name, ResourceFunctionContract::new([], R::None));
    }
    old.insert_contract(
        "cfl_sds_cat",
        ResourceFunctionContract::new(
            [P::Owned { nullable: true }, P::Borrowed],
            R::Reallocated {
                parameter: 0,
                nullable: true,
            },
        ),
    );
    old.insert_contract(
        "cfl_kvlist_fetch",
        ResourceFunctionContract::new(
            vec![P::Borrowed; 2],
            R::Derived {
                parameter: 0,
                extent: DerivedExtent::Borrowed,
                nullable: true,
                const_dropped: false,
            },
        ),
    );
    // Rule 37 deliberately replaces only these nullable inherited modes.
    // Their stronger result is a body-checked specification under the named
    // allocation premise; it must never become an unchecked non-null fact.
    let checked =
        qcore_compiler::contract_loader::declared_checked_specifications(&contracts).unwrap();
    let premises = qcore_compiler::contract_loader::declared_premises(&contracts).unwrap();
    for name in ["rd_calloc", "rd_strdup"] {
        assert!(checked.contains(name), "{name} must remain body-checked");
        assert!(
            premises[name]
                .iter()
                .any(|p| p.name == "asserted_allocation")
        );
        old.insert_contract(
            name,
            ResourceFunctionContract::new([], R::Owned { nullable: false }),
        );
    }
    // Explicitly borrowed trailing arguments were implicit in the old table.
    fn canonical(mut contract: ResourceFunctionContract) -> ResourceFunctionContract {
        while contract.parameters.last() == Some(&P::Borrowed) {
            contract.parameters.pop();
        }
        contract
    }
    for source in contracts.source_assumptions().values() {
        for declaration in qcore_compiler::parse_source_program(source)
            .unwrap()
            .declarations
        {
            let name = declaration.value.name;
            // W47-CONDVAR-JOIN-STOP-PATHS: a declaration awaiting review is
            // withheld (C06.1): it has no mode to preserve.
            // Card W47-LOOP-OWNER-INSTANCE-FLOW: a declaration held for human
            // review is withheld from the function map (contract spec C06.1).
            if contracts.pending_review().contains_key(&name) {
                assert!(contracts.contract(&name).is_none(), "{name}");
                continue;
            }
            // Card CALLEE-RETENTION-CLASS1: a return-code disposition and a
            // retaining declaration are withheld into their own tables (C04.13,
            // C04.14); neither has a previous plain mode to preserve.
            if contracts.relational_contract(&name).is_some()
                || contracts.retaining_contract(&name).is_some()
            {
                assert!(contracts.contract(&name).is_none(), "{name}");
                continue;
            }
            assert_eq!(
                canonical(contracts.contract(&name).unwrap().clone()),
                canonical(old.contract(&name).unwrap().clone()),
                "{name}"
            );
        }
    }
}

#[test]
fn loaded_contract_changes_drive_custody_verdicts() {
    let contracts = load_resource_contract_manifest(&manifest()).unwrap();
    for (source, expected) in [
        (
            "void test(void) { void *p = malloc(8); if (p) free(p); }",
            AnalysisStatus::Safe,
        ),
        (
            "void test(void) { void *p = malloc(8); }",
            AnalysisStatus::Unsafe,
        ),
        (
            "void test(void) { void *p = malloc(8); if (p) { free(p); free(p); } }",
            AnalysisStatus::Unsafe,
        ),
        ("void test(void) { absent(); }", AnalysisStatus::Unknown),
        (
            "void test(void) { void *p = rd_malloc(8); void *q = rd_realloc(p, 16); if (q) { rd_free(q); } else { rd_free(p); } }",
            AnalysisStatus::Safe,
        ),
        (
            "void test(void) { void *p = rd_malloc(8); p = rd_realloc(p, 16); rd_free(p); }",
            AnalysisStatus::Unsafe,
        ),
    ] {
        assert_eq!(
            analyze_legacy_c_translation_unit(source, "test", &contracts)
                .unwrap()
                .status,
            expected,
            "{source}"
        );
    }
    let source = "void test(void) { void *p = alloc(); }";
    assert_eq!(
        analyze_legacy_c_translation_unit(source, "test", &load(ALLOC).unwrap())
            .unwrap()
            .status,
        AnalysisStatus::Unsafe
    );
    let neutral = ALLOC.replace("resource_return_owned(nullable)", "resource_return_none()");
    assert_eq!(
        analyze_legacy_c_translation_unit(source, "test", &load(&neutral).unwrap())
            .unwrap()
            .status,
        AnalysisStatus::Safe
    );
}

#[test]
fn unsupported_or_incomplete_contracts_fail_atomically() {
    for bad in [
        ALLOC.replace("resource_contract_v1", "resource_contract_v2"),
        ALLOC.replace(
            "requires resource_contract_v1;",
            "requires resource_contract_v1; requires size > 0;",
        ),
        ALLOC.replace(
            "resource_return_owned(nullable)",
            "result != error_sentinel",
        ),
        ALLOC.replace("ensures normal", "ensures error Failed"),
        ALLOC.replace("ensures normal", "ensures any"),
        ALLOC.replace("ensures normal", "quant normal"),
        ALLOC.replace(
            "resource_return_owned(nullable)",
            "resource_return_owned(nullable,)",
        ),
        ALLOC.replace(
            "resource_return_owned(nullable)",
            "resource_return_owned(nullable) and hidden()",
        ),
        ALLOC.replace(
            "resource_return_owned(nullable)",
            "resource_return_owned(maybe)",
        ),
        ALLOC.replace("requires resource_contract_v1;", ""),
        ALLOC.replace("ensures normal resource_return_owned(nullable);", ""),
        ALLOC.replace("ptr<mut, u8> contract", "usize contract"),
        ALLOC.replace("contract {", "effects { hidden } contract {"),
        ALLOC.replace("extern fn", "fn"),
        ALLOC.replace("module contract;", "module library;"),
        ALLOC.replace("alloc()", "alloc(size: usize)"),
        format!("{ALLOC}\nextern fn other() -> unit;"),
        format!("{ALLOC}\nconst other: usize = 1;"),
        "module contract;".into(),
        String::new(),
    ] {
        assert!(load(&bad).is_err(), "accepted {bad}");
    }
    assert!(load_resource_contract_sources(&BTreeMap::new()).is_err());
    assert!(
        load_resource_contract_files(&[manifest().with_file_name("missing-contract.qcore")])
            .is_err()
    );
}

/// Card CONTRACT-SUPERSESSION (contract spec C06.6): a function declared by
/// two modules is no longer refused; with no pin stamps (this loader has none)
/// it is withheld as a contract conflict, identical texts included, and no
/// declaration of it is in force. A function declared twice in one module and
/// a module selected twice are still refused.
#[test]
fn duplicate_functions_are_withheld_and_duplicate_inputs_rejected() {
    let sources = BTreeMap::from([("a".into(), ALLOC.into()), ("b".into(), ALLOC.into())]);
    let loaded = load_resource_contract_sources(&sources).unwrap();
    assert_eq!(loaded.contract_conflict().len(), 1);
    assert!(loaded.functions().is_empty());
    assert!(
        load(&format!(
            "{ALLOC}{}",
            ALLOC.trim_start_matches("module contract;")
        ))
        .unwrap_err()
        .contains("declared twice in one module")
    );
    let file = manifest()
        .parent()
        .unwrap()
        .join("providers/quirin-provider-libc/contracts/libc/memory.qv");
    assert!(
        load_resource_contract_files(&[file.clone(), file])
            .unwrap_err()
            .contains("duplicate contract input")
    );
    let conflicting = BTreeMap::from([
        ("a".into(), ALLOC.into()),
        ("b".into(), ALLOC.replace("nullable", "nonnull")),
    ]);
    let loaded = load_resource_contract_sources(&conflicting).unwrap();
    let conflict = loaded.contract_conflict().values().next().unwrap();
    assert_eq!(conflict.declarations.len(), 2, "both texts are retained");
    assert!(loaded.functions().is_empty());
}

#[test]
fn all_parameter_modes_are_explicit_and_indexed_by_declared_names() {
    for (predicate, expected) in [
        ("resource_borrowed(p)", P::Borrowed),
        ("resource_owned(p, nonnull)", P::Owned { nullable: false }),
        (
            "resource_consumed(p, nullable)",
            P::Consumed { nullable: true },
        ),
        (
            "resource_released(p, nullable)",
            P::Released { nullable: true },
        ),
        ("resource_out(p, nonnull)", P::Out { nullable: false }),
        (
            "resource_stored(p, owner, nullable)",
            P::Stored {
                owner_parameter: 0,
                nullable: true,
            },
        ),
        (
            "resource_removed(p, owner, nonnull)",
            P::Removed {
                owner_parameter: 0,
                nullable: false,
            },
        ),
    ] {
        let source = format!(
            "module contract;
        extern fn transfer(owner: ptr<mut, u8>, p: ptr<mut, ptr<mut, u8>>) -> unit contract {{
            requires resource_contract_v1;
            ensures normal resource_owned(owner, nonnull);
            ensures normal {predicate};
            ensures normal resource_return_none();
        }};"
        );
        let contracts = load(&source).unwrap();
        assert_eq!(
            contracts.contract("transfer").unwrap().parameters,
            [P::Owned { nullable: false }, expected]
        );
        assert!(
            load(&source.replace(
                "resource_return_none()",
                "resource_return_none(); ensures normal resource_return_none()"
            ))
            .is_err()
        );
    }
}

#[test]
fn invalid_parameter_bindings_and_types_are_rejected() {
    let source = "module contract;
        extern fn transfer(owner: ptr<mut, u8>, p: ptr<mut, u8>) -> unit contract {
            requires resource_contract_v1;
            ensures normal resource_owned(owner, nonnull);
            ensures normal resource_stored(p, owner, nullable);
            ensures normal resource_return_none();
        };";
    for bad in [
        source.replace("resource_stored(p, owner", "resource_stored(p, p"),
        source.replace("resource_stored(p, owner", "resource_stored(p, missing"),
        source.replace("resource_owned(owner, nonnull)", "resource_borrowed(owner)"),
        source.replace(
            "resource_stored(p, owner, nullable)",
            "resource_out(p, nullable)",
        ),
        source.replace("p: ptr<mut, u8>", "p: usize"),
        source.replace(
            "resource_stored(p, owner, nullable)",
            "resource_borrowed(owner)",
        ),
        source.replace("p: ptr<mut, u8>", "owner: ptr<mut, u8>"),
        source
            .replace("unit contract", "ptr<mut, u8> contract")
            .replace(
                "resource_return_none()",
                "resource_return_reallocated(p, nullable)",
            ),
    ] {
        assert!(load(&bad).is_err(), "accepted {bad}");
    }
}

#[test]
fn certificates_replay_exact_assumptions_and_reject_table_mutation() {
    let contracts = load_resource_contract_manifest(&manifest()).unwrap();
    let source = concat!(
        "void *malloc(__SIZE_TYPE__ size);\n",
        "void free(void *pointer);\n",
        "void test(void) { void *p = malloc(8); free(p); }",
    );
    let unit = C11TranslationUnit {
        source,
        ..C11TranslationUnit::default()
    };
    let Ok(UnrestrictedLegacyResult::Safe(mut certificate)) =
        analyze_unrestricted_c11(&unit, "test", &contracts)
    else {
        panic!("fixture did not produce a Safe certificate");
    };
    check_legacy_safety_certificate(&certificate).unwrap();
    // 6 -> 9 manifest-selected modules by card PROD-CONTRACTS; 9 -> 12 by card
    // NEIGHBOURS; 12 -> 14 by card CONSTRUCTOR-RESIDUALS; 14 -> 15 by card
    // CONSTRUCTOR-RESIDUALS-ABI-TYPEDEF; 15 -> 16 by card
    // ABI-TYPEDEF-ADDRESS-OF-MEMBER; 16 -> 17 by card
    // ADDRESS-OF-MEMBER-DESTROY-UNDECLARED; 17 -> 18 by card
    // AOM-DESTROY-SIBLINGS; 18 -> 19 by card W29-CORPUS-CONTRACTS; 19 -> 21 by
    // card W31-CORPUS-CONTRACTS-2.

    // 21 -> 27 by card W36-CORPUS-CONTRACTS-3, six declarations chosen by
    // function-cover yield over the 193 baseline functions blocked only by
    // uncontracted-call leaves, each read against its own pinned body:
    // `cfl/sds_lifetime.qv`, `cmetrics/context_lifetime.qv`,
    // `ctraces/context_lifetime.qv`, `librdkafka/cjson.qv`,
    // `librdkafka/kafka_mem.qv` and `monkey/logging.qv`.

    // 27 -> 33 by card W37-CORPUS-CONTRACTS-4, six declarative modules chosen
    // by function-cover yield over the 163 baseline functions blocked only by
    // uncontracted-call leaves, each read against its own pinned body:
    // `cfl/time.qv`, `cmetrics/metric_values.qv`,
    // `cmetrics/mpack_utils.qv`, `mpack/writer.qv` (the corpus's first
    // `mpack` family, distinct from `msgpack/`), `librdkafka/lz4_stream.qv`
    // and `librdkafka/topic_partition.qv`. Every one of the ten
    // declarations is the NEUTRAL ELEMENT — every formal borrowed, result
    // none — so none can contradict a faithful derived summary.
    // 33 -> 44 by card W43-LIBC-CONTRACTS: eleven `libc/` modules owned by
    // `providers/quirin-provider-libc` (55 trusted libc/POSIX declarations,
    // each read from the prototype the pinned units declare).
    // 44 -> 45 by card CONTRACT-INHERIT-ADOPTION: `fluent_bit/resource_string.qv`
    // selected by `crate:module` reference (schema 2); `resource_memory.qcore`
    // moved from a path entry to a reference without changing the count.
    // 45 -> 46 by card W43-LIBC-RESTRICT-PROTOTYPES (`libc/regex.qv`).
    // 46 -> 47 by card W43-DESCRIPTOR-RESOURCE-TRACKING: `libc/dirent.qv`.
    // 47 -> 49 by card W45-FB-RETRY-LIFETIME: `fluent_bit/task_users.qv` and
    // `fluent_bit/output_singleplex.qv`.
    // 49 -> 50 by card W45-FB-BACKLOG-CUSTODY: `fluent_bit/storage_backlog.qv`
    // (twelve checked specifications, contract spec C06.3).
    // Combined W46 cards plus six metric getters and five routing queries:
    // 54 selected modules, 230 resource declarations, 118 corpus modules.
    // 54 -> 55 by card W47-LOOP-OWNER-CONTRACTS: `fluent_bit/loop_owner.qv`.
    // 55 -> 56 by card W47-CONDVAR-JOIN-PUBLICATION: `libc/pthread_sync.qv`.
    // 56 -> 57 by card W47-CONDVAR-JOIN-STOP-PATHS: `fluent_bit/lib_engine_thread.qv` (five declarations awaiting review).
    // 57 -> 58 by card W47-LOOP-OWNER-INSTANCE-FLOW: `fluent_bit/input_instance.qv`.
    // 58 -> 59 by card CONTRACT-PIN-TIP-MODE: `monkey/vhost_set.qv`.
    // 59 -> 60 by card MONKEY-VHOST-PROPERTY-CUSTODY: `monkey/string.qv`.
    // 60 -> 61 by card W43-ABI-ENUM-AND-CONST-PP: `librdkafka/client.qv`.
    // 61 -> 63 by card FB-EVENT-LOOP-RETENTION-AND-REGISTRATION:
    // `monkey/event_loop.qv` and `fluent_bit/input_collector_registration.qv`.
    // 63 -> 64 by card FB-TEARDOWN-DESTROY-DECLARATIONS: `fluent_bit/teardown_destroy.qv`.
    // 65 -> 66 by card W48-RK-LOG-CALLER-FRAME: `librdkafka/log_caller_frame.qv`.
    assert_eq!(certificate.contracts.source_assumptions().len(), 66);
    certificate
        .contracts
        .insert("malloc", ResourceApiKind::Neutral);
    assert!(
        check_legacy_safety_certificate(&certificate)
            .unwrap_err()
            .contains("source assumptions")
    );
}

/// Every scalar in the ABI table is an opaque borrowed-only projection: it can
/// be a borrowed argument or a plain result, and every custody mode on it is
/// rejected rather than approximated.
#[test]
fn every_abi_scalar_is_a_borrowed_only_projection() {
    // Card NEIGHBOURS added `f32`/`f64` under the condition this table's own
    // note stated for them — a resource API in scope that passes one: Fluent
    // Bit's `double flb_time_now()`. The loop below still runs over the WHOLE
    // table, so the two new entries are checked to reject every custody mode
    // exactly as the integer entries do; nothing about the refusals moved.
    assert_eq!(
        SCALAR_ABI_TYPES,
        [
            "bool", "i8", "i16", "i32", "i64", "isize", "u8", "u16", "u32", "u64", "usize", "f32",
            "f64"
        ]
    );
    // Card W43-ABI-ENUM-AND-CONST-PP (gateway rule 12v) added the pointer to
    // a pointer to const, deliberately.
    assert_eq!(
        POINTER_ABI_TYPES,
        [
            "ptr<mut, u8>",
            "ptr<const, u8>",
            "ptr<mut, ptr<mut, u8>>",
            "ptr<mut, ptr<const, u8>>"
        ]
    );
    // Card CONSTRUCTOR-RESIDUALS-ABI-TYPEDEF added the projection of a pointer
    // to function, under the same condition NEIGHBOURS stated for the floats: a
    // resource API in scope passes one — msgpack's `msgpack_packer_init`, which
    // production `flb_input_chunk_create` calls. It is deliberately NOT an entry
    // of `POINTER_ABI_TYPES` and deliberately not spelled `ptr<…>`, because both
    // custody gates key on the leading word `ptr`. The loop below therefore runs
    // over it as well, and checks it rejects every custody mode exactly as an
    // integer entry does: a function designator is never a resource identity, so
    // no declaration may claim custody of one.
    assert_eq!(FUNCTION_POINTER_ABI_TYPE, "fnptr");
    assert!(!POINTER_ABI_TYPES.contains(&FUNCTION_POINTER_ABI_TYPE));
    assert!(!SCALAR_ABI_TYPES.contains(&FUNCTION_POINTER_ABI_TYPE));
    assert!(!FUNCTION_POINTER_ABI_TYPE.starts_with("ptr"));
    for scalar in SCALAR_ABI_TYPES
        .iter()
        .chain(std::iter::once(&FUNCTION_POINTER_ABI_TYPE))
    {
        let argument = format!(
            "module contract;
        extern fn probe(n: {scalar}) -> ptr<mut, u8> contract {{
            requires resource_contract_v1;
            ensures normal resource_borrowed(n);
            ensures normal resource_return_owned(nullable);
        }};"
        );
        let contracts = load(&argument).unwrap_or_else(|e| panic!("{scalar} argument: {e}"));
        assert_eq!(
            contracts.contract("probe").unwrap(),
            &ResourceFunctionContract::new([P::Borrowed], R::Owned { nullable: true }),
            "{scalar}"
        );
        let result = format!(
            "module contract;
        extern fn probe(p: ptr<mut, u8>) -> {scalar} contract {{
            requires resource_contract_v1;
            ensures normal resource_released(p, nonnull);
            ensures normal resource_return_none();
        }};"
        );
        let contracts = load(&result).unwrap_or_else(|e| panic!("{scalar} result: {e}"));
        assert_eq!(
            contracts.contract("probe").unwrap(),
            &ResourceFunctionContract::new([P::Released { nullable: false }], R::None),
            "{scalar}"
        );
        // A scalar never carries custody, and a non-`None` result needs a
        // pointer return type.
        for bad in [
            argument.replace("resource_borrowed(n)", "resource_owned(n, nonnull)"),
            argument.replace("resource_borrowed(n)", "resource_released(n, nonnull)"),
            argument.replace("resource_borrowed(n)", "resource_out(n, nonnull)"),
            result.replace("resource_return_none()", "resource_return_owned(nullable)"),
        ] {
            assert!(load(&bad).is_err(), "accepted {bad}");
        }
    }
    // Card W44-LIBC-RESIDUALS added the projection of the compiler's
    // `__builtin_va_list`, under the same condition: glibc's `vsnprintf`, which
    // 25 functions of the pinned corpus call. Like `fnptr` it is not a `ptr`
    // spelling, so every custody mode on it is rejected; unlike `fnptr` it is a
    // parameter type only, so no result of it loads at all.
    assert_eq!(VA_LIST_ABI_TYPE, "va_list");
    assert!(!POINTER_ABI_TYPES.contains(&VA_LIST_ABI_TYPE));
    assert!(!SCALAR_ABI_TYPES.contains(&VA_LIST_ABI_TYPE));
    assert!(!VA_LIST_ABI_TYPE.starts_with("ptr"));
    let argument = format!(
        "module contract;
        extern fn probe(p: ptr<mut, u8>, ap: {VA_LIST_ABI_TYPE}) -> i32 contract {{
            requires resource_contract_v1;
            ensures normal resource_borrowed(p);
            ensures normal resource_borrowed(ap);
            ensures normal resource_return_none();
        }};"
    );
    let contracts = load(&argument).unwrap_or_else(|e| panic!("va_list argument: {e}"));
    assert_eq!(
        contracts.contract("probe").unwrap(),
        &ResourceFunctionContract::new([P::Borrowed, P::Borrowed], R::None),
    );
    for bad in [
        argument.replace("resource_borrowed(ap)", "resource_owned(ap, nonnull)"),
        argument.replace("resource_borrowed(ap)", "resource_released(ap, nonnull)"),
        argument.replace("resource_borrowed(ap)", "resource_out(ap, nonnull)"),
        argument.replace("-> i32", &format!("-> {VA_LIST_ABI_TYPE}")),
    ] {
        assert!(load(&bad).is_err(), "accepted {bad}");
    }
    // Types outside the table stay rejected: `unit` is a result type only, and
    // a pointer spelling the pointer table does not carry is not a projection
    // this loader can read. Card NEIGHBOURS moved `f32`/`f64` OUT of this list
    // and into `SCALAR_ABI_TYPES`; the two entries that stay here are the
    // load-bearing half of this check, and the loop above now covers the floats
    // in the borrowed-only direction instead.
    for parameter in ["unit", "ptr<const, ptr<const, u8>>"] {
        let source = format!(
            "module contract;
        extern fn probe(n: {parameter}) -> unit contract {{
            requires resource_contract_v1;
            ensures normal resource_borrowed(n);
            ensures normal resource_return_none();
        }};"
        );
        assert!(load(&source).is_err(), "accepted parameter {parameter}");
    }
}

/// The `requires resource_contract_v1` marker is the interpreter selector.
/// Asking this loader for a `ResourceContracts` environment over an
/// observation/lifecycle module must fail closed — never silently drop the
/// clauses it cannot check and return a weaker table.
#[test]
fn every_corpus_module_outside_the_v1_vocabulary_is_rejected_not_skipped() {
    let modules = contract_corpus::sources();
    // Includes the runtime-only reopen-failure and integrity observation
    // modules; neither carries the v1 marker, so both are rejected here.
    // 60 -> 61: card HASH-CONTRACTS adds
    // `fluent_bit/hash_table_publication.qv`, which carries the v1 marker.
    // 61 -> 64: card PROD-CONTRACTS adds `fluent_bit/input_chunk_name.qv`,
    // `fluent_bit/logging.qv` and `fluent_bit/routes_mask.qv`, all three
    // entirely v1 and all three manifest-selected.
    // 64 -> 67: card NEIGHBOURS adds `fluent_bit/input_chunk_helpers.qv`,
    // `fluent_bit/time.qv` and `chunkio/chunk_residency.qv`, all three
    // entirely v1 and all three manifest-selected.
    // 67 -> 69: card CONSTRUCTOR-RESIDUALS adds `libc/errno.qv` and
    // `monkey/intrusive_list.qv`, both entirely v1 and both
    // manifest-selected.
    // 69 -> 70: card CONSTRUCTOR-RESIDUALS-ABI-TYPEDEF adds
    // `libc/pthread_mutex.qv`, entirely v1 and manifest-selected.
    // 70 -> 71: card ABI-TYPEDEF-ADDRESS-OF-MEMBER adds `msgpack/packer.qv`,
    // entirely v1 and manifest-selected — the declaration the previous card
    // wrote and measured at payoff zero, landed once the blocker that
    // measurement rested on was repaired.
    // 71 -> 72: card ADDRESS-OF-MEMBER-DESTROY-UNDECLARED adds
    // `fluent_bit/input_chunk_destroy.qv`, entirely v1 and
    // manifest-selected — the declaration that decides the leak leaf the
    // previous card's corpus movement opened.
    // 72 -> 73: card AOM-DESTROY-SIBLINGS adds
    // `fluent_bit/input_chunk_destroy_direct_routes.qv`, entirely v1 and
    // manifest-selected — one of the two siblings that card deliberately left
    // undeclared, read on its own terms and landed with a `nullable` release
    // where its neighbour carries `nonnull`. The other sibling,
    // `flb_input_chunk_destroy_corrupted`, was read too and deliberately NOT
    // landed at measured payoff zero.
    // 73 -> 74: card W29-CORPUS-CONTRACTS adds
    // `fluent_bit/input_chunk_sizes.qv`, entirely v1 and manifest-selected
    // — two read-only size queries declared borrowed/none against their
    // pinned bodies.
    // 74 -> 76: card W31-CORPUS-CONTRACTS-2 adds `chunkio/chunk_sizes.qv`
    // and `fluent_bit/input_chunk_total_size.qv`, both entirely v1 and
    // both manifest-selected — the two size-sibling read-only queries the
    // previous arbiter named and declined, each declared borrowed/none
    // against its own pinned body.

    // 21 -> 27 by card W36-CORPUS-CONTRACTS-3, six declarations chosen by
    // function-cover yield over the 193 baseline functions blocked only by
    // uncontracted-call leaves, each read against its own pinned body:
    // `cfl/sds_lifetime.qv`, `cmetrics/context_lifetime.qv`,
    // `ctraces/context_lifetime.qv`, `librdkafka/cjson.qv`,
    // `librdkafka/kafka_mem.qv` and `monkey/logging.qv`.

    // 27 -> 33 by card W37-CORPUS-CONTRACTS-4, six declarative modules chosen
    // by function-cover yield over the 163 baseline functions blocked only by
    // uncontracted-call leaves, each read against its own pinned body:
    // `cfl/time.qv`, `cmetrics/metric_values.qv`,
    // `cmetrics/mpack_utils.qv`, `mpack/writer.qv` (the corpus's first
    // `mpack` family, distinct from `msgpack/`), `librdkafka/lz4_stream.qv`
    // and `librdkafka/topic_partition.qv`. Every one of the ten
    // declarations is the NEUTRAL ELEMENT — every formal borrowed, result
    // none — so none can contradict a faithful derived summary.
    // 93 -> 96: Cycle 18 aggregate/platform/task policies belong to the typed interpreter.
    // 96 -> 107: card W43-LIBC-CONTRACTS adds eleven `libc/` modules in
    // `providers/quirin-provider-libc`, all entirely v1 and all
    // manifest-selected.
    // 107 -> 108: card CONTRACT-INHERIT-ADOPTION adds
    // `fluent_bit/resource_string.qv`, entirely v1 (two inherited
    // declarations) and manifest-selected.
    // 108 -> 109: card W43-LIBC-RESTRICT-PROTOTYPES adds `libc/regex.qv`,
    // entirely v1 and manifest-selected.
    // 109 -> 110: card W43-DESCRIPTOR-RESOURCE-TRACKING adds `libc/dirent.qv`,
    // entirely v1 (a kinded pair) and manifest-selected.
    // 110 -> 111: card W43-DESCRIPTOR-NATIVE-CHECK adds
    // `library/descriptor_native_observation.qv`, a native observation
    // policy (ten parameterless rules), not resource-v1 and not selected.
    // 111 -> 113: card W45-FB-RETRY-LIFETIME adds `fluent_bit/task_users.qv`
    // and `fluent_bit/output_singleplex.qv`, entirely v1 and
    // manifest-selected.
    // 113 -> 114: card W45-FB-BACKLOG-CUSTODY adds
    // `fluent_bit/storage_backlog.qv`, entirely v1 (checked specifications)
    // and manifest-selected.
    // Combined W46 cards plus six metric getters and five routing queries:
    // 54 selected modules, 230 resource declarations, 118 corpus modules.
    // 118 -> 119: card W47-LOOP-OWNER-CONTRACTS adds
    // `fluent_bit/loop_owner.qv`, entirely v1 and manifest-selected.
    // 119 -> 120: card W47-CONDVAR-JOIN-PUBLICATION adds
    // `libc/pthread_sync.qv`, entirely v1 and manifest-selected.
    // 120 -> 121: card W47-CONDVAR-JOIN-STOP-PATHS adds
    // `fluent_bit/lib_engine_thread.qv`, entirely v1 (every declaration
    // awaiting review) and manifest-selected.
    // 121 -> 122: card W47-LOOP-OWNER-INSTANCE-FLOW adds
    // `fluent_bit/input_instance.qv`, entirely v1 (review markers included)
    // and manifest-selected.
    // 122 -> 123: card CONTRACT-PIN-TIP-MODE adds `monkey/vhost_set.qv`
    // (SUP04's `mk_vhost_set`, its lock entry a tip identity), entirely v1
    // and manifest-selected.
    // 123 -> 124: card MONKEY-VHOST-PROPERTY-CUSTODY adds `monkey/string.qv`
    // (`mk_string_dup`, pinned at 66910c10, 046accf6 and Monkey 21a2c190),
    // entirely v1 and manifest-selected.
    // 124 -> 125: card CHUNKIO-META-LEN-BOUNDS-CONTRACTS adds
    // `chunkio/meta_len_bounds.qv`, a bounds contract v1 module (the crate's
    // bounds view, never selected): resource-v1 rejects both declarations.
    // 125 -> 126: card W43-ABI-ENUM-AND-CONST-PP adds `librdkafka/client.qv`
    // (twelve librdkafka declarations over enumeration and pointer-to-const
    // prototypes, two of them retaining), entirely v1 and manifest-selected.
    // 126 -> 127: card CG16-BOUNDS-TRANSPORT-AND-CHUNKIO-NEIGHBOURS adds
    // `chunkio/file_st_bounds.qv`, a bounds contract v1 module (the crate's
    // bounds view, never selected): resource-v1 rejects its three
    // declarations.
    // 127 -> 130: card NULL-SAFETY-PROPERTY adds the three `null_deref.qv`
    // modules (cmetrics, fluent_bit, librdkafka), bounds contract v1 modules
    // (each crate's bounds view, never selected): resource-v1 rejects their
    // thirteen declarations.
    // 130 -> 131: card NULL-TRIAGE-UNRESOLVED-PATHS adds `cfl/null_deref.qv`
    // (`cfl_list_size`'s result range; the crate's new bounds view, never
    // selected): resource-v1 rejects its one declaration.
    // 131 -> 133: card FB-EVENT-LOOP-RETENTION-AND-REGISTRATION adds
    // `monkey/event_loop.qv` (seven Monkey event-loop operations) and
    // `fluent_bit/input_collector_registration.qv` (three collector
    // setters), entirely v1 and manifest-selected.
    // 133 -> 134: card FB-TEARDOWN-DESTROY-DECLARATIONS adds
    // `fluent_bit/teardown_destroy.qv` (fourteen destroyer declarations, ten
    // with member-path states), entirely v1 and manifest-selected.
    // 134 -> 135: card OUTER-CONTRACT adds `fluent_bit/outer.qv`, an outer
    // contract v1 module (the crate's outer view, never selected):
    // resource-v1 rejects both loader declarations.
    // 135 -> 136: RK-PLUGIN-LIBRARY-OUTER-ENVELOPE adds
    // `librdkafka/outer.qv` (one outer-only loader declaration).
    // 137 -> 138 by card W48-RK-LOG-CALLER-FRAME: `librdkafka/log_caller_frame.qv`.
    // Two additional declared CMetrics map/opts lifecycle model modules.
    // The CFL hash model is already present on main; typed quant conditions
    // preserve it, and cfl/atomic.qv adds one further model module.
    // 144 -> 146: card CELL-GUARD-RESULT-RANGE-CHECKED adds bounds contract v1
    // value sets, `libc/result_sets.qv` (three C library result sets) and
    // `monkey/result_range.qv` (one derived range); resource-v1 rejects all
    // four declarations.
    // 146 -> 147: card CELL-RESULT-SET-WRITTEN-SET-DERIVATION adds
    // `fluent_bit/result_range.qv` (`flb_pipe_create`'s derived written sets,
    // a bounds contract v1 value set); resource-v1 rejects its declaration.
    assert_eq!(modules.len(), 147, "corpus module count");

    let mut selected = 0;
    let mut v1_declarations = 0;
    let mut rejected_declarations = 0;
    for (relative, path) in &modules {
        // Card CONTRACT-INHERIT-ADOPTION: a module that inherits is read as its
        // elaboration, the text every loader parses; so every count here is of
        // elaborated declarations, and a conversion to `contract inherit`
        // moves none of them.
        let source = contract_corpus::text(relative, path);
        let parsed = qcore_compiler::parse_source_program(&source).unwrap();
        let marked = |declaration: &qcore_compiler::SourceDeclaration| {
            declaration.function_contract.as_ref().is_some_and(|c| {
                c.clauses
                    .iter()
                    .any(|clause| clause.value.expression_source.trim() == "resource_contract_v1")
            })
        };
        let all_marked = parsed.declarations.iter().all(|d| marked(&d.value));
        let loaded =
            load_resource_contract_sources(&BTreeMap::from([(relative.clone(), source.clone())]));
        assert_eq!(
            loaded.is_ok(),
            all_marked,
            "{relative}: marker and loadability disagree ({loaded:?})"
        );
        if all_marked {
            selected += 1;
        }
        // Per declaration, so a marked declaration cannot smuggle an unmarked
        // sibling through a module that happens to load. Card
        // SUPPLIED-OWNED-RETURN-CODE-HANDOFF: a declaration is read with its
        // module's aliases (module-local context, never a declaration).
        let aliases = parsed
            .aliases()
            .iter()
            .map(|alias| format!("{}\n", &source[alias.span.start..alias.span.end]))
            .collect::<String>();
        for declaration in &parsed.declarations {
            let text = &source[declaration.span.start..declaration.span.end];
            let single = BTreeMap::from([(
                relative.clone(),
                format!("module contract;\n{aliases}{text}\n"),
            )]);
            let outcome = load_resource_contract_sources(&single);
            assert_eq!(
                outcome.is_ok(),
                marked(&declaration.value),
                "{relative}: {} marker and loadability disagree ({outcome:?})",
                declaration.value.name
            );
            if marked(&declaration.value) {
                v1_declarations += 1;
            } else {
                rejected_declarations += 1;
            }
        }
    }
    // The thirteen manifest-selected resource projections and their 33
    // assumptions; every other declaration belongs to another interpreter and
    // is rejected. 5 -> 6 and 19 -> 20 by card HASH-CONTRACTS; 6 -> 9 and
    // 20 -> 25 by card PROD-CONTRACTS; 9 -> 12 and 25 -> 31 by card NEIGHBOURS
    // (two input-chunk helpers, one clock reading, three ChunkIO residency
    // helpers); 12 -> 14 and 31 -> 34 by card CONSTRUCTOR-RESIDUALS
    // (`libc/errno.qv`'s `__errno_location`,
    // `monkey/intrusive_list.qv`'s `mk_list_add`, and `flb_errno_print`
    // added to the existing `fluent_bit/logging.qv`); 14 -> 15 and 34 -> 36
    // by card CONSTRUCTOR-RESIDUALS-ABI-TYPEDEF (`libc/pthread_mutex.qv`'s
    // two mutex entry points); 15 -> 16 and 36 -> 37 by card
    // ABI-TYPEDEF-ADDRESS-OF-MEMBER (`msgpack/packer.qv`'s
    // `msgpack_packer_init`, the declaration the previous card wrote and
    // measured at payoff zero, landed once the blocker it named was repaired);
    // 16 -> 17 and 37 -> 38 by card ADDRESS-OF-MEMBER-DESTROY-UNDECLARED
    // (`fluent_bit/input_chunk_destroy.qv`'s `flb_input_chunk_destroy`, the
    // release the previous card's newly-bound summary exposed as uncontracted).
    // 17 -> 18 and 38 -> 39 by card AOM-DESTROY-SIBLINGS
    // (`fluent_bit/input_chunk_destroy_direct_routes.qv`'s
    // `flb_input_chunk_destroy_direct_routes`, the sibling release the previous
    // card deliberately left undeclared, read on its own terms).
    // 18 -> 19 and 39 -> 41 by card W29-CORPUS-CONTRACTS
    // (`fluent_bit/input_chunk_sizes.qv`'s `flb_input_chunk_get_size` and
    // `flb_input_chunk_get_real_size`, two read-only size queries declared
    // borrowed/none against their pinned bodies).
    // 19 -> 21 and 41 -> 43 by card W31-CORPUS-CONTRACTS-2
    // (`chunkio/chunk_sizes.qv`'s `cio_chunk_get_content_size` and
    // `fluent_bit/input_chunk_total_size.qv`'s
    // `flb_input_chunk_total_size`, the two size-sibling read-only queries
    // W29's named non-declarations pointed at, declared borrowed/none against
    // their own pinned bodies).
    // 21 -> 27 and 43 -> 49 by card W36-CORPUS-CONTRACTS-3, six declarations
    // chosen by function-cover yield and each read against its own pinned body:
    // `cfl/sds_lifetime.qv`'s `cfl_sds_destroy`,
    // `cmetrics/context_lifetime.qv`'s `cmt_destroy`,
    // `ctraces/context_lifetime.qv`'s `ctr_destroy`,
    // `librdkafka/cjson.qv`'s `cJSON_Delete`,
    // `librdkafka/kafka_mem.qv`'s `rd_kafka_mem_free` and
    // `monkey/logging.qv`'s `mk_print` — five releases and one variadic
    // custody-neutral diagnostic sink.
    // 27 -> 33 and 49 -> 59 by card W37-CORPUS-CONTRACTS-4, six modules and
    // ten declarations chosen by function-cover yield and each read against its
    // own pinned body: `cfl/time.qv`'s `cfl_time_now`,
    // `cmetrics/metric_values.qv`'s `cmt_metric_get_timestamp` and
    // `cmt_metric_set_double`, `cmetrics/mpack_utils.qv`'s
    // `cmt_mpack_consume_uint_tag`, `mpack/writer.qv`'s `mpack_write_cstr`,
    // `mpack_write_str`, `mpack_write_i64` and `mpack_write_double`,
    // `librdkafka/lz4_stream.qv`'s `LZ4_resetStream` and
    // `librdkafka/topic_partition.qv`'s
    // `rd_kafka_topic_partition_list_sort` — ten custody-NEUTRAL declarations,
    // no release among them, which is the opposite balance to W36's five
    // releases and is a fact about what the post-W36 cover frontier still held.
    // The rejected count is unchanged in all twelve cases, because every added
    // module is entirely v1.
    // All 34 Cycle 18 typed-policy declarations are deliberately rejected by resource-v1.
    // 33 -> 44 and 59 -> 114 by card W43-LIBC-CONTRACTS: eleven `libc/`
    // modules and 55 declarations, every one a trusted assumption about the C
    // library read from the prototype the pinned units declare; 53 are the
    // neutral element, `strdup` returns fresh `malloc` custody
    // (`return_owned(nullable)`), and `prctl` applies only under its literal
    // `PR_SET_NAME` premise. The rejected count is unchanged: every added
    // module is entirely v1.
    // 114 -> 127 by card W43-LIBC-RESULT-ALIAS: thirteen derived-result
    // declarations added to the already-selected `libc/string.qv` (eleven)
    // and `libc/time.qv` (two), so the selection stays at 44 modules and the
    // rejected count is unchanged.
    // 127 -> 128 by card CONTRACT-INHERIT-ADOPTION: `strndup` appended to
    // `libc/string.qv`. Its conversion of `flb_malloc`, `flb_calloc` and
    // `flb_free` to `contract inherit` moves nothing here: these counts are
    // of elaborated declarations. 44 -> 45 and 128 -> 130 by the same card's
    // `fluent_bit/resource_string.qv` (`flb_strdup`, `flb_strndup`).
    // 207 -> 206 rejected by card CONTRACT-VIEW-PROJECTION-CHECK: the
    // duplicate `source_identity` typed policy left
    // `monkey/timeout_teardown.qv` (its owner is
    // `monkey/wait_source_policy.qv`); the v1 selection is unchanged.
    // 45 -> 46 and 130 -> 140 by card W43-LIBC-RESTRICT-PROTOTYPES: ten
    // neutral declarations, two in the new `libc/regex.qv` and eight
    // appended to already-selected libc modules.
    // 140 -> 144 by card W43-LIBC-PRINTF-FAMILY: `printf`, `fprintf`,
    // `snprintf` and `sprintf` appended to the already-selected
    // `libc/stdio.qv`, so the selection and the rejected count are unchanged.
    // 46 -> 47 and 144 -> 152 by card W43-DESCRIPTOR-RESOURCE-TRACKING: eight
    // declarations, six appended to already-selected modules (`open`, `close`,
    // `socket`, `fopen`, `fclose`, `fileno`) and two in the new
    // `libc/dirent.qv`. Rejected unchanged.
    // 206 -> 216 rejected by card W43-DESCRIPTOR-NATIVE-CHECK: the ten rules
    // of `library/descriptor_native_observation.qv` belong to the native
    // descriptor replay and are deliberately rejected by resource-v1.
    // 152 -> 155 by card W44-LIBC-RESIDUALS: `vsnprintf` and `vprintf`
    // appended to `libc/stdio.qv` and `strerror` to `libc/errno.qv`, all
    // already selected. Selection and rejected unchanged.
    // 47 -> 49 and 155 -> 162 by card W45-FB-RETRY-LIFETIME: two new
    // `fluent_bit/` modules (six declarations) and `mk_list_del` appended to
    // the already-selected `monkey/intrusive_list.qv`. Rejected unchanged.
    // 49 -> 50 and 162 -> 174 by card W45-FB-BACKLOG-CUSTODY: the twelve
    // checked specifications of `fluent_bit/storage_backlog.qv`.
    // 174 -> 183 by card W46-EXTERNAL-CALLEE-CONTRACTS: nine neutral
    // declarations appended to already-selected modules (`libc/unistd.qv`,
    // `monkey/logging.qv`, `librdkafka/memory.qv`,
    // `cfl/sds_lifetime.qv`, `mpack/writer.qv`). Selection and rejected
    // unchanged.
    // The two new W46 cards add six metric getters and five routing queries:
    // 54 selected modules, 230 resource declarations, 118 corpus modules.
    // 230 -> 232 by card W47-CALLEE-CLOSURE-R2: `cfl_arena_calloc` appended
    // to `cfl/sds_lifetime.qv` and `rd_file_read` to `librdkafka/memory.qv`,
    // both already selected. Selection and rejected unchanged.
    // 54 -> 55 and 232 -> 236 by card W47-LOOP-OWNER-CONTRACTS: the four
    // collector operations of `fluent_bit/loop_owner.qv`, each with a declared
    // loop-owner obligation. Rejected unchanged.
    // 55 -> 56 and 236 -> 242 by card W47-CONDVAR-JOIN-PUBLICATION: the six
    // declarations of `libc/pthread_sync.qv`, each with a declared
    // thread-synchronization obligation. Rejected unchanged.
    // 56 -> 57 and 242 -> 247 by card W47-CONDVAR-JOIN-STOP-PATHS: the five
    // declarations of `fluent_bit/lib_engine_thread.qv`, each awaiting
    // review (C06.1), so loaded and withheld: the function table keeps 242
    // and the five are its pending-review entries. Rejected unchanged.
    // 57 -> 58 and 247 -> 251 by card W47-LOOP-OWNER-INSTANCE-FLOW: the four
    // declarations of `fluent_bit/input_instance.qv`, each loaded and then
    // withheld for human review. Rejected unchanged.
    // 251 -> 252 by card W47-FLB-LIB-LIFECYCLE: `flb_destroy` joins
    // `fluent_bit/lib_engine_thread.qv` with its lifecycle obligation, loaded
    // and withheld for human review. Selection and rejected unchanged.
    // 252 -> 253 by card W47-SERVICE-PROPERTY-KEYS: `flb_config_set_property`
    // in `fluent_bit/input_instance.qv`, loaded and withheld for human
    // review. Selected and rejected unchanged.
    // 253 -> 255 by card W48-CALLEE-CLOSURE-R3: `lstat` appended to
    // `libc/sys_stat.qv` and `clock_gettime` to `libc/time.qv`, both already
    // selected, both neutral and flexible. Selection and rejected unchanged.
    // 255 -> 259 by card CG25-RESOURCE-KINDS: `dlopen`/`dlclose` appended to
    // `libc/dlfcn.qv` and `regcomp`/`regfree` to `libc/regex.qv`, both already
    // selected. Selection and rejected unchanged.
    // 259 -> 262 by card CG21-CALL-EXITS-AND-BOUNDARIES: `abort`, `exit`
    // and `__assert_fail` appended to `libc/stdlib.qv`, already selected,
    // each with `resource_no_return()`. Selection and rejected unchanged.
    // 58 -> 59 and 262 -> 263 by card CONTRACT-PIN-TIP-MODE:
    // `monkey/vhost_set.qv`'s `mk_vhost_set` (borrowed formals, a borrowed
    // variadic tail, no result), manifest-selected.
    // 59 -> 60 and 263 -> 264 by card MONKEY-VHOST-PROPERTY-CUSTODY:
    // `monkey/string.qv`'s `mk_string_dup` (a borrowed string, an owned
    // nullable result), manifest-selected.
    // 264 -> 265 by card CONTRACT-NORETURN-AND-BORROWED-TAILS: `rd_kafka_crash`
    // appended to `librdkafka/memory.qv`, already selected, with
    // `resource_no_return()`. Selection and rejected unchanged.
    // 265 -> 272 by card CG10-INDIRECT-ACCESS-SOLE-RESIDUALS: three
    // `cmt_atomic` operations appended to `cmetrics/metric_values.qv` and four
    // scalar helpers to `librdkafka/memory.qv`, both already selected, each
    // the neutral element and pinned. Selection and rejected unchanged.
    // 272 -> 293 by card W45-CALLEE-CASCADE-RESIDUAL: twenty-one pinned
    // declarations appended to five already-selected modules
    // (`librdkafka/cjson.qv` 1, `librdkafka/lz4_stream.qv` 3,
    // `librdkafka/kafka_mem.qv` 5, `ctraces/context_lifetime.qv` 6,
    // `cfl/sds_lifetime.qv` 5, `chunkio/chunk_residency.qv` 1); nineteen are
    // the neutral element, two return an owned allocation. Selection and
    // rejected unchanged.
    // 216 -> 218 rejected by card CHUNKIO-META-LEN-BOUNDS-CONTRACTS: the two
    // bounds declarations of `chunkio/meta_len_bounds.qv`
    // (`cio_file_st_get_meta_len`, `cio_file_st_set_meta_len`) belong to the
    // bounds consumer and are deliberately rejected by resource-v1.
    // 293 -> 304 by card CALLEE-RETENTION-CLASS1: six return-code
    // dispositions (`librdkafka/cjson.qv`'s `add_item_to_array` and
    // `replace_item_in_object`, `cfl/sds_lifetime.qv`'s
    // `cfl_kvlist_insert_array`/`_kvlist`, `ctraces/context_lifetime.qv`'s
    // `ctr_attributes_set_array`/`_kvlist`) and five retaining declarations
    // (`librdkafka/lz4_stream.qv`'s four LZ4 stream functions,
    // `fluent_bit/input_chunk_helpers.qv`'s `append_to_ring_buffer`), all
    // appended to already-selected modules and pinned. Selection and rejected
    // unchanged.
    // 60 -> 61 selected and 304 -> 319 by card W43-ABI-ENUM-AND-CONST-PP: the
    // new `librdkafka/client.qv` (twelve declarations, two of them retaining),
    // `LZ4_decompress_generic` appended to `librdkafka/lz4_stream.qv`, and the
    // flexible libc `getrlimit`/`setrlimit` appended to `libc/process.qv`, all
    // pinned (the libc pair flexible). Rejected unchanged.
    // 319 -> 333 by card SLOT-MOVE-AND-INTERIOR-BORROW: fourteen declarations
    // appended to already-selected modules: the slot moves
    // `cfl/sds_lifetime.qv`'s `cfl_sds_cat_safe` and
    // `ctraces/context_lifetime.qv`'s `sds_cat_safe`, the borrowed results
    // `librdkafka/cjson.qv`'s `get_object_item` and `librdkafka/client.qv`'s
    // three `rd_kafka_admin_result_ret_*` and
    // `rd_kafka_topic_partition_get_private`, the derived out slots
    // `librdkafka/client.qv`'s `rd_slice_reader0`, `librdkafka/cjson.qv`'s
    // `cJSON_ParseWithLengthOpts` (beside a `global` retention) and the
    // flexible libc `strtol`/`strtoll`/`strtoul`/`strtoull`/`strtod` in
    // `libc/stdlib.qv`; the rest pinned. Selection and rejected unchanged.
    // 333 -> 349 by card REFCOUNT-RESOURCE-KIND: sixteen declarations
    // appended to `librdkafka/client.qv`, all pinned: four keeps and five
    // drops (reference clauses, withheld) and seven plain declarations (the
    // queue handle constructor `rd_kafka_queue_new0` and six destructors).
    // Selection and rejected unchanged.
    // 349 -> 359 by card REFCOUNT-REFERENCE-RETURNING-DECLARATIONS: ten
    // declarations appended to `librdkafka/client.qv`, all pinned: nine
    // returned references (`rd_kafka_q_new0` also retaining `rk` and `func` by
    // its result) and the drop `ut_destroy_mock_queue`. Selection and rejected
    // unchanged.
    // 359 -> 363 by card REFCOUNT-FAMILY-BLOCKING-CALLEES: four plain
    // declarations appended to `librdkafka/client.qv`, all pinned and in the
    // function map (`rd_kafka_q_wait_result`, `rd_kafka_q_purge0`,
    // `rd_kafka_toppar_needs_query`, `rd_kafka_toppar_fetch_backoff`).
    // Selection and rejected unchanged.
    // 218 -> 221 rejected by card CG16-BOUNDS-TRANSPORT-AND-CHUNKIO-NEIGHBOURS:
    // the three bounds declarations of `chunkio/file_st_bounds.qv`
    // (`cio_file_st_get_hash`, `cio_file_st_infer_content_len`,
    // `cio_file_st_set_content_len`) belong to the bounds consumer and are
    // deliberately rejected by resource-v1.
    // 363 -> 365 by card CG10-INDIRECT-ACCESS-SOLE-RESIDUALS round 2: two
    // plain declarations, `cio_errno_print` (`chunkio/chunk_residency.qv`) and
    // `ctr_errno_print` (`ctraces/context_lifetime.qv`), pinned and in the
    // function map. Selection and rejected unchanged by it.
    // 365 -> 379 by card CG22-CALLBACK-REGISTRY-CONTRACT: fourteen callback
    // declarations (`bsearch`, `qsort`, `pthread_once`; the cmetrics and
    // ctraces array and map unpackers; `rd_list_init`, `_new`, `_add`,
    // `_destroy`, `_clear`; `pb_write`, `pb_encode_varint`), all withheld
    // with the retaining declarations, plus two re-declared in place
    // (`rd_kafka_topic_partition_list_sort`, `msgpack_packer_init`).
    // Selection unchanged.
    // 221 -> 234 rejected by card NULL-SAFETY-PROPERTY: the thirteen
    // `quant null-deref` declarations of the three `null_deref.qv` modules
    // belong to the bounds consumer and are deliberately rejected by
    // resource-v1. Selection and v1 declarations unchanged.
    // 234 -> 246 rejected by card NULL-CANDIDATE-TRIAGE: twelve more
    // declarations in `librdkafka/null_deref.qv` (four member hosts, one
    // non-null result, seven `quant null-deref` checks), likewise rejected.
    // 246 -> 313 rejected by card NULL-MEMBER-INVARIANT-SCOPE: the stable
    // member's host `rd_kafka_broker_add` and sixty-six function-scope
    // entries in `librdkafka/null_deref.qv`, likewise rejected.
    // 313 -> 314 rejected by card NULL-TRIAGE-UNRESOLVED-PATHS: the
    // `cfl/null_deref.qv` result range (`cfl_list_size`), likewise rejected.
    // 379 -> 393 by card W45-CALLEE-CASCADE-RESIDUAL round 3: fourteen
    // pinned declarations appended to selected modules (librdkafka
    // `client.qv` six, `kafka_mem.qv` two; libevent `monkey/memory.qv`
    // `mk_fifo_queue_destroy`; cfl `sds_lifetime.qv` `cfl_list_size`;
    // fluent-bit `hash_table_publication.qv` `flb_hash_table_del`; chunkio
    // `chunk_sizes.qv` two; cmetrics `context_lifetime.qv`
    // `cmt_atomic_initialize`). Selection and rejected unchanged.
    // 393 -> 394 by card QR-PRINT-ROUTE-MATCHES-DECLARATION: cJSON's static
    // `print` in `librdkafka/cjson.qv`, the declaration round 3 withdrew while
    // the module could not compile off the print-call route.
    // 394 -> 401 by card CG22-OP-CALLBACKS-AND-SERVED-QUEUES: the six queue
    // serving declarations and `rd_kafka_op_destroy` in
    // `librdkafka/client.qv`, all withheld with the retaining declarations.
    // Selection and rejected unchanged.
    // 401 -> 406 by card OP-CONSTRUCTOR-DECLARATIONS: the op constructors
    // `rd_kafka_op_new0` and `rd_kafka_op_new_cb` and the request family
    // `rd_kafka_op_req0`, `_req`, `_req2` in `librdkafka/client.qv`, all
    // withheld with the retaining declarations. Selection and rejected
    // unchanged.
    // 406 -> 425 by card OP-CALLBACK-CANDIDATE-CONTRACTS: nineteen op-callback
    // candidates in `librdkafka/client.qv` (seventeen `rko_op_cb`, two
    // `rko_serve`), plain declarations in the function map. Selection and
    // rejected unchanged.
    // 425 -> 455 by card OP-APP-RK-CALLBACKS-VIA-POLL-CB: in
    // `librdkafka/client.qv` the seventeen rk_conf callback setters,
    // `rd_kafka_conf_new`, `rd_kafka_consume_cb`,
    // `rd_kafka_background_queue_serve` and `rd_kafka_oauthbearer_refresh_op`
    // (withheld with the retaining declarations, 57 -> 78) and the six
    // librdkafka candidates (`rd_kafka_log_print`, five OAUTHBEARER
    // refreshers); in `fluent_bit/logging.qv` Fluent Bit's three registered
    // callbacks (plain declarations). Selection and rejected unchanged.
    // 455 -> 457 by card SUPPLIED-OWNED-RETURN-CODE-HANDOFF: the two hand-off
    // declarations `rd_kafka_poll_cb` and
    // `rd_kafka_txn_op_send_offsets_to_transaction` in `librdkafka/client.qv`
    // (withheld with the retaining declarations, 78 -> 80). Selection and
    // rejected unchanged.
    // 457 -> 465 by card OP-CALLBACK-MEMBER-PATH-RELEASE: the eight
    // op-callback candidates of shape (b) in `librdkafka/client.qv`, each
    // stating its member-path states (two beside a KEEP hand-off), withheld
    // with the retaining declarations. Selection and rejected unchanged.
    // 465 -> 475 by card FB-EVENT-LOOP-RETENTION-AND-REGISTRATION: seven
    // Monkey event-loop operations (`monkey/event_loop.qv`, five withheld
    // with the retaining declarations) and three Fluent Bit collector setters
    // (`fluent_bit/input_collector_registration.qv`, withheld); 61 -> 63
    // selected modules. Rejected unchanged.
    // 63 -> 64 selected, 475 -> 489 by card FB-TEARDOWN-DESTROY-DECLARATIONS:
    // `fluent_bit/teardown_destroy.qv`, fourteen declarations (ten with
    // member-path states, withheld with the retaining declarations).
    // 314 -> 328 rejected by card FB-PLUGIN-DESCRIPTOR-NONNULL-SCOPE: the
    // four stable plugin descriptors and ten function-scope entries in
    // `fluent_bit/null_deref.qv`, likewise rejected.
    // 489 -> 491 by card RETENTION-UNMATCHED-REMOVAL-POISONS:
    // `mk_event_loop_create` and `mk_event_loop_destroy` in
    // `monkey/event_loop.qv` (plain declarations). Selection and rejected
    // unchanged.
    // 328 -> 330 rejected by card OUTER-CONTRACT: the two loader declarations
    // of `fluent_bit/outer.qv` (outer contract v1), likewise rejected.
    // 330 -> 331 rejected by RK-PLUGIN-LIBRARY-OUTER-ENVELOPE:
    // `librdkafka/outer.qv`'s one outer-only loader declaration.
    // Checked same-item detach adds one v1 declaration to the existing cJSON
    // module: 491 -> 492; selected modules and rejected declarations unchanged.
    // Kafka callback placement adds one selected module; declaration counts stay fixed.
    // 492 -> 493 by card W48-SCHED-CLEANUP-PRODUCTION: the element destroyer
    // `mk_http_thread_destroy` in the existing `monkey/intrusive_list.qv`.
    // Selected modules and rejected declarations unchanged.
    // 65 -> 66 selected and 493 -> 495 v1 declarations by card
    // W48-RK-LOG-CALLER-FRAME: `librdkafka/log_caller_frame.qv`, plain
    // caller-frame declarations `rd_kafka_log_buf` and `rd_kafka_log0`.
    // Rejected declarations unchanged.
    // W48-CMT-TRACKED-INTERIOR-SELF-INIT adds cfl_list_init as a declared
    // model only: 331 -> 332 rejected; interpreted modules/declarations stay fixed.
    // Eight API functions plus two resource type declarations: 332 -> 342 non-v1.
    // Setter effect models add 15 non-v1 declarations: 342 -> 357.
    // Main adds interpreted writev and thrd_current: 495 -> 497.
    // EVENT-CALLER-REMOVAL adds the exact mk_event_timeout_disable removal
    // declaration to the existing Monkey event-loop module: 497 -> 498.
    // Selected modules and rejected declarations are unchanged.
    // CELL-PROTOCOL-FB-DECLARATIONS adds the cell-protocol host
    // `flb_config_set_program_name` to the existing
    // `fluent_bit/teardown_destroy.qv`: 498 -> 499; selected modules and
    // rejected declarations unchanged.
    // CELL-GUARD-RESULT-RANGE-CHECKED's two bounds value-set modules add four
    // rejected declarations: 365 -> 369; selected modules and v1 unchanged.
    // CELL-RESULT-SET-WRITTEN-SET-DERIVATION's `fluent_bit/result_range.qv`
    // (`flb_pipe_create`'s written sets) adds one: 369 -> 370.
    // CFL-MPACK-EXTERNAL-DECLARATIONS appends 30 CFL declarations to
    // `cfl/sds_lifetime.qv` and 25 mpack declarations to `mpack/writer.qv`,
    // both already selected: 499 -> 554; selected modules and rejected
    // declarations unchanged.
    assert_eq!(
        (selected, v1_declarations, rejected_declarations),
        (66, 554, 370)
    );
    // A declaration held for human review is loaded and withheld from the
    // function map (contract spec C06.1): the map plus the held set is every
    // v1 declaration (5 from W47-CONDVAR-JOIN-STOP-PATHS, 4 from
    // W47-LOOP-OWNER-INSTANCE-FLOW, 1 from W47-FLB-LIB-LIFECYCLE, 1 from W47-SERVICE-PROPERTY-KEYS).
    let loaded = load_resource_contract_manifest(&manifest()).unwrap();
    assert_eq!(loaded.pending_review().len(), 11);
    // Card CALLEE-RETENTION-CLASS1: a return-code disposition and a retaining
    // declaration are loaded and withheld from the function map into their own
    // tables (contract spec C04.13, C04.14), six and five of them; card
    // W43-ABI-ENUM-AND-CONST-PP adds two retaining declarations
    // (`rd_kafka_confval_set_type`, `rd_kafka_interceptor_method_add`): 5 -> 7.
    // Card SLOT-MOVE-AND-INTERIOR-BORROW: nine declarations with slot clauses
    // are withheld into the same table (C04.15, C04.16): the two slot moves,
    // `rd_slice_reader0`, `cJSON_ParseWithLengthOpts` and the five libc
    // `strto*`: 7 -> 16. The five borrowed results are in the function map.
    // Card REFCOUNT-RESOURCE-KIND: the nine keeps and drops (C04.17) are
    // withheld into the same table: 16 -> 25.
    // Card REFCOUNT-REFERENCE-RETURNING-DECLARATIONS: `rd_kafka_q_new0` and
    // the re-declared `rd_kafka_queue_new0` (retained by the result, C04.14.2)
    // and the drop `ut_destroy_mock_queue`: 25 -> 28. The eight returned
    // references with no other clause stay in the function map.
    // Card CG22-CALLBACK-REGISTRY-CONTRACT: the fourteen callback declarations
    // and the two re-declared with callback clauses
    // (`rd_kafka_topic_partition_list_sort`, `msgpack_packer_init`, which
    // leave the function map) are withheld into the same table (C04.19):
    // 28 -> 44.
    // Card W45-CALLEE-CASCADE-RESIDUAL round 3: `unit_test_telemetry`, a
    // synchronous callback invocation, is withheld into the same table: 44 -> 45.
    // Card CG22-OP-CALLBACKS-AND-SERVED-QUEUES: the six queue serving
    // declarations and `rd_kafka_op_destroy` (C04.19.1-C04.19.3): 45 -> 52.
    // Card CFL-MPACK-EXTERNAL-DECLARATIONS: four return-code dispositions
    // (`cfl_kvlist_insert`, `cfl_array_append`, `_append_array`,
    // `_append_kvlist`): relational 6 -> 10.
    assert_eq!(loaded.relational().len(), 10);
    // Card OP-CONSTRUCTOR-DECLARATIONS: the two op constructors (a result
    // state, C04.19.4) and the three requests (member invocations): 52 -> 57.
    // Card OP-APP-RK-CALLBACKS-VIA-POLL-CB: the seventeen setters (member
    // registrations), `rd_kafka_conf_new` (stated defaults) and three member
    // invokers (`rd_kafka_consume_cb`, `rd_kafka_background_queue_serve`,
    // `rd_kafka_oauthbearer_refresh_op`): 57 -> 78.
    // Card SUPPLIED-OWNED-RETURN-CODE-HANDOFF: the two hand-off declarations
    // (C04.19.5): 78 -> 80.
    // Card OP-CALLBACK-MEMBER-PATH-RELEASE: the eight member-path
    // declarations (C04.19.6): 80 -> 88.
    // Card FB-EVENT-LOOP-RETENTION-AND-REGISTRATION: three Monkey
    // registrations (C04.14), two removals of one registration and three
    // collector setters forwarded through an id (C04.19.3): 88 -> 96.
    // Card FB-TEARDOWN-DESTROY-DECLARATIONS: the ten Fluent Bit destroyers
    // with member-path states (`fluent_bit/teardown_destroy.qv`; the other
    // four are in the function map): 96 -> 106.
    // EVENT-CALLER-REMOVAL adds one withheld removal declaration: 106 -> 107.
    // It must not become an ordinary borrow-only function-map entry.
    // Card MK-EVENT-DESCRIPTOR-QUANT: `mk_event_channel_destroy` now carries
    // descriptor-cell clauses (a guarded removal and guarded releases,
    // C04.6.2) and leaves the function map: 107 -> 108.
    // Card CELL-PROTOCOL-FB-DECLARATIONS: the `quant cell` host
    // `flb_config_set_program_name` is a withheld lifetime declaration
    // (C04.6.3): 108 -> 109.
    // Card CFL-MPACK-EXTERNAL-DECLARATIONS: the slot move `cfl_sds_printf`
    // (C04.15): 109 -> 110.
    assert_eq!(loaded.retaining().len(), 110);
    assert_eq!(
        loaded.functions().len()
            + loaded.pending_review().len()
            + loaded.relational().len()
            + loaded.retaining().len(),
        v1_declarations
    );

    // The marker is a selector, not a bypass: attaching it to a lifecycle
    // declaration still rejects, because the rest of that declaration is
    // owned by the language front end.
    let lifecycle = std::fs::read_to_string(&modules["chunkio/chunk_destructor.qv"]).unwrap();
    let marked = lifecycle.replacen("contract {", "contract { requires resource_contract_v1;", 1);
    assert_ne!(marked, lifecycle);
    assert!(load(&marked).is_err());
}

#[test]
fn manifest_selection_rejects_unknown_schema_missing_inputs_and_traversal() {
    let directory =
        std::env::temp_dir().join(format!("qcore-contract-manifest-{}", std::process::id()));
    std::fs::create_dir_all(&directory).unwrap();
    let path = directory.join("selection.json");
    std::fs::write(directory.join("api.qcore"), ALLOC).unwrap();
    for text in [
        r#"{"schema_version":2,"modules":["api.qcore"]}"#,
        r#"{"schema_version":1,"modules":[]}"#,
        r#"{"schema_version":1,"modules":["api.qcore"],"fallback":true}"#,
        r#"{"schema_version":1,"modules":["api.qcore","absent.qcore"]}"#,
        r#"{"schema_version":1,"modules":["../api.qcore"]}"#,
        r#"{"schema_version":1,"modules":["/api.qcore"]}"#,
        r#"{"schema_version":1,"modules":["api.qcore","api.qcore"]}"#,
        "invalid JSON",
    ] {
        std::fs::write(&path, text).unwrap();
        assert!(
            load_resource_contract_manifest(&path).is_err(),
            "accepted {text}"
        );
    }
    std::fs::write(&path, r#"{"schema_version":1,"modules":["api.qcore"]}"#).unwrap();
    let loaded = load_resource_contract_manifest(&path).unwrap();
    assert_eq!(loaded.source_assumptions().values().next().unwrap(), ALLOC);
    // Immutable evidence survives deletion of the original selected file.
    std::fs::remove_dir_all(&directory).unwrap();
    check_source_assumptions(&loaded).unwrap();
}
