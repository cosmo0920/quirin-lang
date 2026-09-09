use std::fs;
use std::path::{Path, PathBuf};

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
    ("chunkio/chunk_destructor.qcore", "cio_chunk_close", 10),
    ("chunkio/chunk_destructor.qcore", "cio_chunk_delete", 5),
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
    ("cmetrics/counter_reset.qcore", "cmt_counter_allow_reset", 4),
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
    (
        "language/quantified_indices.qcore",
        "preserve_five_chunk_credits",
        2,
    ),
    ("libc/memory.qcore", "malloc", 3),
    ("libc/memory.qcore", "calloc", 4),
    ("libc/memory.qcore", "free", 3),
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
    ("monkey/memory.qcore", "mk_mem_alloc", 3),
    ("monkey/memory.qcore", "mk_mem_alloc_z", 3),
    ("monkey/memory.qcore", "mk_mem_free", 3),
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
];

fn corpus_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("compiler crate has a repository parent")
        .join("contract/qcore")
}

fn contract_sources(directory: &Path) -> Vec<PathBuf> {
    fn visit(directory: &Path, sources: &mut Vec<PathBuf>) {
        for entry in fs::read_dir(directory).expect("contract directory is readable") {
            let path = entry.expect("contract entry is readable").path();
            if path.is_dir() {
                visit(&path, sources);
            } else if path
                .extension()
                .is_some_and(|extension| extension == "qcore")
            {
                sources.push(path);
            }
        }
    }

    let mut sources = Vec::new();
    visit(directory, &mut sources);
    sources.sort();
    sources
}

#[test]
fn every_qcore_contract_source_parses_round_trips_and_compiles() {
    let root = corpus_root();
    let sources = contract_sources(&root);
    // 60 -> 61 by card HASH-CONTRACTS' deliberate addition of
    // `fluent_bit/hash_table_publication.qcore`.
    assert_eq!(sources.len(), 61, "unexpected contract corpus size");

    for path in sources {
        let source = fs::read_to_string(&path).expect("contract source is readable");
        // Card HASH-CLAUSE. The `qcore-premise` comment directive is retired in
        // favour of §C04.5's `resource_literal_argument` clause. The resource
        // loader rejects a module carrying it, but only the six modules the
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
    let root = corpus_root();
    let mut actual = Vec::new();
    let mut all_expressions = Vec::new();
    let mut has_normal = false;
    let mut has_error = false;
    let mut has_any = false;

    for path in contract_sources(&root) {
        let source = fs::read_to_string(&path).expect("contract source is readable");
        let parsed = parse_source_program(&source).expect("contract source parses");
        let relative = path
            .strip_prefix(&root)
            .expect("contract lies below corpus root")
            .to_string_lossy()
            .replace('\\', "/");
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
    let path = corpus_root().join("monkey/composition.qcore");
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
