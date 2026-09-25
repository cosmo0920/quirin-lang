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
    // NEIGHBOURS (`fluent_bit/input_chunk_helpers.qcore`,
    // `fluent_bit/time.qcore`, `chunkio/chunk_residency.qcore`); 12 -> 14 by
    // card CONSTRUCTOR-RESIDUALS (`libc/errno.qcore`,
    // `monkey/intrusive_list.qcore`); 14 -> 15 by card
    // CONSTRUCTOR-RESIDUALS-ABI-TYPEDEF (`libc/pthread_mutex.qcore`; that card
    // deliberately did not land `msgpack/packer.qcore`, measured payoff zero);
    // 15 -> 16 by card ABI-TYPEDEF-ADDRESS-OF-MEMBER, which repaired the
    // blocker that measurement rested on and landed it; 16 -> 17 by card
    // ADDRESS-OF-MEMBER-DESTROY-UNDECLARED
    // (`fluent_bit/input_chunk_destroy.qcore`), which decides the leak leaf
    // that card's corpus movement opened; 17 -> 18 by card AOM-DESTROY-SIBLINGS
    // (`fluent_bit/input_chunk_destroy_direct_routes.qcore`), which decides one
    // of the two siblings that card deliberately left undeclared; 18 -> 19 by
    // card W29-CORPUS-CONTRACTS (`fluent_bit/input_chunk_sizes.qcore`), whose
    // two borrowed/none size queries decide thirteen uncontracted-call leaves;
    // 19 -> 21 by card W31-CORPUS-CONTRACTS-2 (`chunkio/chunk_sizes.qcore` and
    // `fluent_bit/input_chunk_total_size.qcore`), the two size-sibling
    // declarations that card's predecessor named and declined, each landed on
    // its own pinned-body reading.

    // 21 -> 27 by card W36-CORPUS-CONTRACTS-3, six declarations chosen by
    // function-cover yield over the 193 baseline functions blocked only by
    // uncontracted-call leaves, each read against its own pinned body:
    // `cfl/sds_lifetime.qcore`, `cmetrics/context_lifetime.qcore`,
    // `ctraces/context_lifetime.qcore`, `librdkafka/cjson.qcore`,
    // `librdkafka/kafka_mem.qcore` and `monkey/logging.qcore`.

    // 27 -> 33 by card W37-CORPUS-CONTRACTS-4, six declarative modules chosen
    // by function-cover yield over the 163 baseline functions blocked only by
    // uncontracted-call leaves, each read against its own pinned body:
    // `cfl/time.qcore`, `cmetrics/metric_values.qcore`,
    // `cmetrics/mpack_utils.qcore`, `mpack/writer.qcore` (the corpus's first
    // `mpack` family, distinct from `msgpack/`), `librdkafka/lz4_stream.qcore`
    // and `librdkafka/topic_partition.qcore`. Every one of the ten
    // declarations is the NEUTRAL ELEMENT — every formal borrowed, result
    // none — so none can contradict a faithful derived summary.
    // 33 -> 44 by card W43-LIBC-CONTRACTS: eleven `libc/` modules owned by
    // `providers/quirin-provider-libc` (55 trusted libc/POSIX declarations,
    // each read from the prototype the pinned units declare).
    // 44 -> 45 by card CONTRACT-INHERIT-ADOPTION: `fluent_bit/resource_string.qcore`
    // selected by `crate:module` reference (schema 2); `resource_memory.qcore`
    // moved from a path entry to a reference without changing the count.
    // 45 -> 46 by card W43-LIBC-RESTRICT-PROTOTYPES (`libc/regex.qcore`).
    // 46 -> 47 by card W43-DESCRIPTOR-RESOURCE-TRACKING: `libc/dirent.qcore`.
    assert_eq!(contracts.source_assumptions().len(), 47);
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
        ("mk_list_add", 2),
        // Card CONSTRUCTOR-RESIDUALS-ABI-TYPEDEF adds two, listed explicitly
        // for the same reason and in the same shape: every parameter borrowed,
        // no result custody. Both were UNWRITABLE before that card, because
        // card LOADER-ABI's declared-ABI reader would not commit to a union
        // typedef (`pthread_mutex_t *`) and withdrew them at every call site.
        ("pthread_mutex_lock", 1),
        ("pthread_mutex_unlock", 1),
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
        ("dlsym", 2),
        ("regexec", 5),
        ("regerror", 4),
        ("getsockopt", 5),
        ("getpeername", 3),
        ("stat", 2),
        ("cnd_timedwait", 3),
        ("pthread_sigmask", 3),
        ("strftime", 4),
        ("gettimeofday", 2),
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

#[test]
fn duplicate_functions_and_inputs_are_rejected_even_when_identical() {
    let sources = BTreeMap::from([("a".into(), ALLOC.into()), ("b".into(), ALLOC.into())]);
    assert!(
        load_resource_contract_sources(&sources)
            .unwrap_err()
            .contains("duplicate function")
    );
    assert!(
        load(&format!(
            "{ALLOC}{}",
            ALLOC.trim_start_matches("module contract;")
        ))
        .is_err()
    );
    let file = manifest()
        .parent()
        .unwrap()
        .join("providers/quirin-provider-libc/contracts/libc/memory.qcore");
    assert!(
        load_resource_contract_files(&[file.clone(), file])
            .unwrap_err()
            .contains("duplicate contract input")
    );
    let conflicting = BTreeMap::from([
        ("a".into(), ALLOC.into()),
        ("b".into(), ALLOC.replace("nullable", "nonnull")),
    ]);
    assert!(load_resource_contract_sources(&conflicting).is_err());
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
        "void *malloc(unsigned long size);\n",
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
    // `cfl/sds_lifetime.qcore`, `cmetrics/context_lifetime.qcore`,
    // `ctraces/context_lifetime.qcore`, `librdkafka/cjson.qcore`,
    // `librdkafka/kafka_mem.qcore` and `monkey/logging.qcore`.

    // 27 -> 33 by card W37-CORPUS-CONTRACTS-4, six declarative modules chosen
    // by function-cover yield over the 163 baseline functions blocked only by
    // uncontracted-call leaves, each read against its own pinned body:
    // `cfl/time.qcore`, `cmetrics/metric_values.qcore`,
    // `cmetrics/mpack_utils.qcore`, `mpack/writer.qcore` (the corpus's first
    // `mpack` family, distinct from `msgpack/`), `librdkafka/lz4_stream.qcore`
    // and `librdkafka/topic_partition.qcore`. Every one of the ten
    // declarations is the NEUTRAL ELEMENT — every formal borrowed, result
    // none — so none can contradict a faithful derived summary.
    // 33 -> 44 by card W43-LIBC-CONTRACTS: eleven `libc/` modules owned by
    // `providers/quirin-provider-libc` (55 trusted libc/POSIX declarations,
    // each read from the prototype the pinned units declare).
    // 44 -> 45 by card CONTRACT-INHERIT-ADOPTION: `fluent_bit/resource_string.qcore`
    // selected by `crate:module` reference (schema 2); `resource_memory.qcore`
    // moved from a path entry to a reference without changing the count.
    // 45 -> 46 by card W43-LIBC-RESTRICT-PROTOTYPES (`libc/regex.qcore`).
    // 46 -> 47 by card W43-DESCRIPTOR-RESOURCE-TRACKING: `libc/dirent.qcore`.
    assert_eq!(certificate.contracts.source_assumptions().len(), 47);
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
    assert_eq!(
        POINTER_ABI_TYPES,
        ["ptr<mut, u8>", "ptr<const, u8>", "ptr<mut, ptr<mut, u8>>"]
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
    // `fluent_bit/hash_table_publication.qcore`, which carries the v1 marker.
    // 61 -> 64: card PROD-CONTRACTS adds `fluent_bit/input_chunk_name.qcore`,
    // `fluent_bit/logging.qcore` and `fluent_bit/routes_mask.qcore`, all three
    // entirely v1 and all three manifest-selected.
    // 64 -> 67: card NEIGHBOURS adds `fluent_bit/input_chunk_helpers.qcore`,
    // `fluent_bit/time.qcore` and `chunkio/chunk_residency.qcore`, all three
    // entirely v1 and all three manifest-selected.
    // 67 -> 69: card CONSTRUCTOR-RESIDUALS adds `libc/errno.qcore` and
    // `monkey/intrusive_list.qcore`, both entirely v1 and both
    // manifest-selected.
    // 69 -> 70: card CONSTRUCTOR-RESIDUALS-ABI-TYPEDEF adds
    // `libc/pthread_mutex.qcore`, entirely v1 and manifest-selected.
    // 70 -> 71: card ABI-TYPEDEF-ADDRESS-OF-MEMBER adds `msgpack/packer.qcore`,
    // entirely v1 and manifest-selected — the declaration the previous card
    // wrote and measured at payoff zero, landed once the blocker that
    // measurement rested on was repaired.
    // 71 -> 72: card ADDRESS-OF-MEMBER-DESTROY-UNDECLARED adds
    // `fluent_bit/input_chunk_destroy.qcore`, entirely v1 and
    // manifest-selected — the declaration that decides the leak leaf the
    // previous card's corpus movement opened.
    // 72 -> 73: card AOM-DESTROY-SIBLINGS adds
    // `fluent_bit/input_chunk_destroy_direct_routes.qcore`, entirely v1 and
    // manifest-selected — one of the two siblings that card deliberately left
    // undeclared, read on its own terms and landed with a `nullable` release
    // where its neighbour carries `nonnull`. The other sibling,
    // `flb_input_chunk_destroy_corrupted`, was read too and deliberately NOT
    // landed at measured payoff zero.
    // 73 -> 74: card W29-CORPUS-CONTRACTS adds
    // `fluent_bit/input_chunk_sizes.qcore`, entirely v1 and manifest-selected
    // — two read-only size queries declared borrowed/none against their
    // pinned bodies.
    // 74 -> 76: card W31-CORPUS-CONTRACTS-2 adds `chunkio/chunk_sizes.qcore`
    // and `fluent_bit/input_chunk_total_size.qcore`, both entirely v1 and
    // both manifest-selected — the two size-sibling read-only queries the
    // previous arbiter named and declined, each declared borrowed/none
    // against its own pinned body.

    // 21 -> 27 by card W36-CORPUS-CONTRACTS-3, six declarations chosen by
    // function-cover yield over the 193 baseline functions blocked only by
    // uncontracted-call leaves, each read against its own pinned body:
    // `cfl/sds_lifetime.qcore`, `cmetrics/context_lifetime.qcore`,
    // `ctraces/context_lifetime.qcore`, `librdkafka/cjson.qcore`,
    // `librdkafka/kafka_mem.qcore` and `monkey/logging.qcore`.

    // 27 -> 33 by card W37-CORPUS-CONTRACTS-4, six declarative modules chosen
    // by function-cover yield over the 163 baseline functions blocked only by
    // uncontracted-call leaves, each read against its own pinned body:
    // `cfl/time.qcore`, `cmetrics/metric_values.qcore`,
    // `cmetrics/mpack_utils.qcore`, `mpack/writer.qcore` (the corpus's first
    // `mpack` family, distinct from `msgpack/`), `librdkafka/lz4_stream.qcore`
    // and `librdkafka/topic_partition.qcore`. Every one of the ten
    // declarations is the NEUTRAL ELEMENT — every formal borrowed, result
    // none — so none can contradict a faithful derived summary.
    // 93 -> 96: Cycle 18 aggregate/platform/task policies belong to the typed interpreter.
    // 96 -> 107: card W43-LIBC-CONTRACTS adds eleven `libc/` modules in
    // `providers/quirin-provider-libc`, all entirely v1 and all
    // manifest-selected.
    // 107 -> 108: card CONTRACT-INHERIT-ADOPTION adds
    // `fluent_bit/resource_string.qcore`, entirely v1 (two inherited
    // declarations) and manifest-selected.
    // 108 -> 109: card W43-LIBC-RESTRICT-PROTOTYPES adds `libc/regex.qcore`,
    // entirely v1 and manifest-selected.
    // 109 -> 110: card W43-DESCRIPTOR-RESOURCE-TRACKING adds `libc/dirent.qcore`,
    // entirely v1 (a kinded pair) and manifest-selected.
    // 110 -> 111: card W43-DESCRIPTOR-NATIVE-CHECK adds
    // `library/descriptor_native_observation.qcore`, a native observation
    // policy (ten parameterless rules), not resource-v1 and not selected.
    assert_eq!(modules.len(), 111, "corpus module count");

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
        // sibling through a module that happens to load.
        for declaration in &parsed.declarations {
            let text = &source[declaration.span.start..declaration.span.end];
            let single =
                BTreeMap::from([(relative.clone(), format!("module contract;\n{text}\n"))]);
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
    // (`libc/errno.qcore`'s `__errno_location`,
    // `monkey/intrusive_list.qcore`'s `mk_list_add`, and `flb_errno_print`
    // added to the existing `fluent_bit/logging.qcore`); 14 -> 15 and 34 -> 36
    // by card CONSTRUCTOR-RESIDUALS-ABI-TYPEDEF (`libc/pthread_mutex.qcore`'s
    // two mutex entry points); 15 -> 16 and 36 -> 37 by card
    // ABI-TYPEDEF-ADDRESS-OF-MEMBER (`msgpack/packer.qcore`'s
    // `msgpack_packer_init`, the declaration the previous card wrote and
    // measured at payoff zero, landed once the blocker it named was repaired);
    // 16 -> 17 and 37 -> 38 by card ADDRESS-OF-MEMBER-DESTROY-UNDECLARED
    // (`fluent_bit/input_chunk_destroy.qcore`'s `flb_input_chunk_destroy`, the
    // release the previous card's newly-bound summary exposed as uncontracted).
    // 17 -> 18 and 38 -> 39 by card AOM-DESTROY-SIBLINGS
    // (`fluent_bit/input_chunk_destroy_direct_routes.qcore`'s
    // `flb_input_chunk_destroy_direct_routes`, the sibling release the previous
    // card deliberately left undeclared, read on its own terms).
    // 18 -> 19 and 39 -> 41 by card W29-CORPUS-CONTRACTS
    // (`fluent_bit/input_chunk_sizes.qcore`'s `flb_input_chunk_get_size` and
    // `flb_input_chunk_get_real_size`, two read-only size queries declared
    // borrowed/none against their pinned bodies).
    // 19 -> 21 and 41 -> 43 by card W31-CORPUS-CONTRACTS-2
    // (`chunkio/chunk_sizes.qcore`'s `cio_chunk_get_content_size` and
    // `fluent_bit/input_chunk_total_size.qcore`'s
    // `flb_input_chunk_total_size`, the two size-sibling read-only queries
    // W29's named non-declarations pointed at, declared borrowed/none against
    // their own pinned bodies).
    // 21 -> 27 and 43 -> 49 by card W36-CORPUS-CONTRACTS-3, six declarations
    // chosen by function-cover yield and each read against its own pinned body:
    // `cfl/sds_lifetime.qcore`'s `cfl_sds_destroy`,
    // `cmetrics/context_lifetime.qcore`'s `cmt_destroy`,
    // `ctraces/context_lifetime.qcore`'s `ctr_destroy`,
    // `librdkafka/cjson.qcore`'s `cJSON_Delete`,
    // `librdkafka/kafka_mem.qcore`'s `rd_kafka_mem_free` and
    // `monkey/logging.qcore`'s `mk_print` — five releases and one variadic
    // custody-neutral diagnostic sink.
    // 27 -> 33 and 49 -> 59 by card W37-CORPUS-CONTRACTS-4, six modules and
    // ten declarations chosen by function-cover yield and each read against its
    // own pinned body: `cfl/time.qcore`'s `cfl_time_now`,
    // `cmetrics/metric_values.qcore`'s `cmt_metric_get_timestamp` and
    // `cmt_metric_set_double`, `cmetrics/mpack_utils.qcore`'s
    // `cmt_mpack_consume_uint_tag`, `mpack/writer.qcore`'s `mpack_write_cstr`,
    // `mpack_write_str`, `mpack_write_i64` and `mpack_write_double`,
    // `librdkafka/lz4_stream.qcore`'s `LZ4_resetStream` and
    // `librdkafka/topic_partition.qcore`'s
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
    // declarations added to the already-selected `libc/string.qcore` (eleven)
    // and `libc/time.qcore` (two), so the selection stays at 44 modules and the
    // rejected count is unchanged.
    // 127 -> 128 by card CONTRACT-INHERIT-ADOPTION: `strndup` appended to
    // `libc/string.qcore`. Its conversion of `flb_malloc`, `flb_calloc` and
    // `flb_free` to `contract inherit` moves nothing here: these counts are
    // of elaborated declarations. 44 -> 45 and 128 -> 130 by the same card's
    // `fluent_bit/resource_string.qcore` (`flb_strdup`, `flb_strndup`).
    // 207 -> 206 rejected by card CONTRACT-VIEW-PROJECTION-CHECK: the
    // duplicate `source_identity` typed policy left
    // `monkey/timeout_teardown.qcore` (its owner is
    // `monkey/wait_source_policy.qcore`); the v1 selection is unchanged.
    // 45 -> 46 and 130 -> 140 by card W43-LIBC-RESTRICT-PROTOTYPES: ten
    // neutral declarations, two in the new `libc/regex.qcore` and eight
    // appended to already-selected libc modules.
    // 140 -> 144 by card W43-LIBC-PRINTF-FAMILY: `printf`, `fprintf`,
    // `snprintf` and `sprintf` appended to the already-selected
    // `libc/stdio.qcore`, so the selection and the rejected count are unchanged.
    // 46 -> 47 and 144 -> 152 by card W43-DESCRIPTOR-RESOURCE-TRACKING: eight
    // declarations, six appended to already-selected modules (`open`, `close`,
    // `socket`, `fopen`, `fclose`, `fileno`) and two in the new
    // `libc/dirent.qcore`. Rejected unchanged.
    // 206 -> 216 rejected by card W43-DESCRIPTOR-NATIVE-CHECK: the ten rules
    // of `library/descriptor_native_observation.qcore` belong to the native
    // descriptor replay and are deliberately rejected by resource-v1.
    // 152 -> 155 by card W44-LIBC-RESIDUALS: `vsnprintf` and `vprintf`
    // appended to `libc/stdio.qcore` and `strerror` to `libc/errno.qcore`, all
    // already selected. Selection and rejected unchanged.
    assert_eq!(
        (selected, v1_declarations, rejected_declarations),
        (47, 155, 216)
    );
    assert_eq!(
        load_resource_contract_manifest(&manifest())
            .unwrap()
            .functions()
            .len(),
        v1_declarations
    );

    // The marker is a selector, not a bypass: attaching it to a lifecycle
    // declaration still rejects, because the rest of that declaration is
    // owned by the language front end.
    let lifecycle = std::fs::read_to_string(&modules["chunkio/chunk_destructor.qcore"]).unwrap();
    let marked = lifecycle.replace(
        "requires chunk_is_live;",
        "requires resource_contract_v1; requires chunk_is_live;",
    );
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
