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
    ("chunkio/context_destructor.qcore", "cio_destroy", 7),
    (
        "chunkio/import_composition.qcore",
        "check_imported_chunk_program",
        7,
    ),
    ("chunkio/stream_destructor.qcore", "cio_stream_destroy", 6),
    ("chunkio/stream_destructor.qcore", "cio_stream_delete", 5),
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
    ("monkey/composition.qcore", "evutil_socketpair", 3),
    ("monkey/composition.qcore", "evutil_socket_geterror", 2),
    ("monkey/composition.qcore", "evutil_closesocket", 4),
    ("monkey/composition.qcore", "cb_event", 3),
    ("monkey/composition.qcore", "cb_timeout", 4),
    ("monkey/composition.qcore", "cb_wait_2_timeout", 2),
    ("monkey/composition.qcore", "check_monkey_composition", 13),
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
    assert_eq!(sources.len(), 11, "unexpected contract corpus size");

    for path in sources {
        let source = fs::read_to_string(&path).expect("contract source is readable");
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
        assert_eq!(module.entry, "main", "{}", path.display());
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
