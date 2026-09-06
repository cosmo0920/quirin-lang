use super::*;

const FIXTURE: &str = include_str!("../fixtures/monkey/callbacks.c");

fn import(source: &str) -> Vec<MonkeySourceResult> {
    import_monkey_callbacks(
        &C11TranslationUnit {
            source,
            ..Default::default()
        },
        PINNED_FLUENT_BIT_REVISION,
    )
    .unwrap()
}

fn summary(results: &[MonkeySourceResult], index: usize) -> &MonkeySourceSummary {
    let MonkeySourceResult::Summary(summary) = &results[index] else {
        panic!("{results:?}");
    };
    summary
}

fn calls(path: &MonkeySourcePath) -> Vec<&str> {
    path.operations
        .iter()
        .filter_map(|operation| match operation {
            SourceOperation::Call { function, .. } => Some(function.as_str()),
            _ => None,
        })
        .collect()
}

#[test]
fn imports_all_callbacks_and_preserves_mask_paths_and_cleanup_order() {
    if !crate::test_support::clang_available() {
        return;
    }
    let results = import(FIXTURE);
    let event = summary(&results, 0);
    assert_eq!(event.function(), "cb_event");
    assert_eq!(event.source_revision(), PINNED_FLUENT_BIT_REVISION);
    assert_eq!(event.paths().len(), 4);
    let mut guards = BTreeSet::new();
    for path in event.paths() {
        guards.insert(
            path.operations
                .iter()
                .filter_map(|op| {
                    if let SourceOperation::Guard { taken, .. } = op {
                        Some(*taken)
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>(),
        );
        let writes: Vec<_> = path
            .operations
            .iter()
            .filter_map(|op| {
                if let SourceOperation::Write { place, .. } = op {
                    Some(place)
                } else {
                    None
                }
            })
            .collect();
        assert_eq!(writes.len(), 4);
        for (place, field) in writes.iter().zip(["fd", "mask", "data", "fired_count"]) {
            assert!(
                matches!(&place.expression, SourceExpression::Field { name, .. } if name == field)
            );
        }
        assert!(calls(path).is_empty());
        assert_eq!(path.operations.last(), Some(&SourceOperation::Return));
    }
    assert_eq!(
        guards,
        BTreeSet::from([
            vec![true, true],
            vec![true, false],
            vec![false, true],
            vec![false, false]
        ])
    );
    let timeout = summary(&results, 1);
    assert_eq!(timeout.paths().len(), 3);
    assert_eq!(
        calls(&timeout.paths()[0]),
        [
            "send",
            "__errno_location",
            "perror",
            "evutil_closesocket",
            "event_del",
            "event_free",
            "mk_mem_free"
        ]
    );
    assert_eq!(
        calls(&timeout.paths()[1]),
        [
            "send",
            "__errno_location",
            "evutil_closesocket",
            "event_del",
            "event_free",
            "mk_mem_free"
        ]
    );
    assert_eq!(calls(&timeout.paths()[2]), ["send"]);
    assert_eq!(
        summary(&results, 2).paths()[0].operations,
        [SourceOperation::Return]
    );
}

#[test]
fn aliases_are_bound_to_parameter_and_prior_reads_not_variable_names() {
    if !crate::test_support::clang_available() {
        return;
    }
    let results = import(FIXTURE);
    let original = summary(&results, 0);
    let renamed = import(
        &FIXTURE
            .replace(
                "struct mk_event *event = data",
                "struct mk_event *argument_alias = data",
            )
            .replace("event->data", "argument_alias->data")
            .replace("event->fd", "argument_alias->fd")
            .replace("fired->data = event", "fired->data = argument_alias"),
    );
    assert_eq!(original, summary(&renamed, 0));
    let path = &original.paths()[0];
    let reads: Vec<_> = path
        .operations
        .iter()
        .filter_map(|op| {
            if let SourceOperation::Read { place } = op {
                Some(place)
            } else {
                None
            }
        })
        .collect();
    assert!(
        matches!(&reads[0].expression, SourceExpression::Field { name, base } if name == "data" && matches!(&base.expression, SourceExpression::Cast { value, .. } if value.expression == SourceExpression::Parameter { index: 2 }))
    );
    assert!(
        matches!(&reads[1].expression, SourceExpression::Field { name, base } if name == "ctx" && matches!(&base.expression, SourceExpression::Cast { value, .. } if matches!(value.expression, SourceExpression::Symbol { .. })))
    );
    let different = import(&FIXTURE.replace("fired->fd   = event->fd", "fired->fd   = fd"));
    assert_ne!(original.paths(), summary(&different, 0).paths());
}

#[test]
fn unsupported_even_unreachable_operations_fail_the_whole_callback() {
    if !crate::test_support::clang_available() {
        return;
    }
    for injected in [
        "while (0) {}",
        "goto done; done: return;",
        "if (0) { send(fd, data, 1, 0); unknown(data); }",
        "return; while (0) {}",
        "static int hidden;",
        "int uninitialized; if (uninitialized) return;",
    ] {
        let source = FIXTURE.replace(
            "/* Compose mask */",
            &format!("{injected}\n/* Compose mask */"),
        );
        let source = format!("void unknown(void *);\n{source}");
        assert!(
            matches!(import(&source)[0], MonkeySourceResult::Unknown { .. }),
            "accepted {injected}"
        );
    }
}

#[test]
fn field_qualifiers_bitfields_and_signature_mismatch_remain_unknown() {
    if !crate::test_support::clang_available() {
        return;
    }
    for (old, new) in [
        ("int fired_count;", "volatile int fired_count;"),
        ("int fired_count;", "_Atomic int fired_count;"),
        ("int fired_count;", "int fired_count : 4;"),
        ("short flags, void *data", "int flags, void *data"),
    ] {
        let results = import(&FIXTURE.replace(old, new));
        assert!(
            matches!(results[0], MonkeySourceResult::Unknown { .. }),
            "accepted {new}"
        );
    }
}

#[test]
fn source_changes_are_retained_and_never_claim_successful_release() {
    if !crate::test_support::clang_available() {
        return;
    }
    let results = import(&FIXTURE.replace(
        "evutil_closesocket(ev_map->pipe[1]);",
        "evutil_closesocket(ev_map->pipe[0]);",
    ));
    let original = import(FIXTURE);
    assert_ne!(summary(&results, 1), summary(&original, 1));
    let altered = import(&FIXTURE.replace("ret == -1", "ret != -1"));
    assert_ne!(summary(&altered, 1), summary(&original, 1));
    let removed = import(&FIXTURE.replace("event_free(ev_map->event);", ""));
    assert!(!calls(&summary(&removed, 1).paths()[0]).contains(&"event_free"));
}

fn with_event_body(body: &str) -> String {
    let prefix = FIXTURE.split("static void cb_event(").next().unwrap();
    format!("{prefix}\nstatic void cb_event(int fd, short flags, void *data) {{ {body} }}")
}

#[test]
fn opaque_calls_and_indirect_writes_invalidate_escaped_local_values() {
    if !crate::test_support::clang_available() {
        return;
    }
    for body in [
        "int x = 1; int *p = &x; *p = 2; if (x) return;",
        "int x = 1; send(fd, &x, 4, 0); if (x) return;",
        "int x = 1; int *p; { int y = 2; p = &y; } if (*p) return;",
    ] {
        let results = import(&with_event_body(body));
        if body.contains("int y") {
            // Source projection retains the ended storage; it does not assert
            // this dangling-pointer source path is safe.
            let path = &summary(&results, 0).paths()[0];
            let end = path
                .operations
                .iter()
                .position(|op| matches!(op, SourceOperation::LocalEnd { local: 5 }))
                .unwrap();
            let read = path
                .operations
                .iter()
                .position(|op| matches!(op, SourceOperation::Read { .. }))
                .unwrap();
            assert!(end < read);
        } else {
            assert!(
                matches!(results[0], MonkeySourceResult::Unknown { .. }),
                "accepted {body}"
            );
        }
    }
}

#[test]
fn does_not_invent_order_for_multiple_calls_or_call_and_memory_read() {
    if !crate::test_support::clang_available() {
        return;
    }
    for body in [
        "if (evutil_closesocket(fd) == event_del(data)) return;",
        "struct mk_event *e = data; if (evutil_closesocket(fd) == e->fd) return;",
        "send(evutil_closesocket(fd), data, 1, 0);",
    ] {
        assert!(
            matches!(
                import(&with_event_body(body))[0],
                MonkeySourceResult::Unknown { .. }
            ),
            "accepted {body}"
        );
    }
}

#[test]
fn import_limits_bound_branches_and_symbolic_substitution() {
    if !crate::test_support::clang_available() {
        return;
    }
    for body in [
        "if (flags) {}".repeat(7),
        format!("int x = 1; {}", "x = x | x;".repeat(13)),
    ] {
        assert!(matches!(
            import(&with_event_body(&body))[0],
            MonkeySourceResult::Unknown { .. }
        ));
    }
}

#[test]
fn absent_callbacks_and_wrong_revision_do_not_create_summaries() {
    if !crate::test_support::clang_available() {
        return;
    }
    let results = import(&with_event_body("return;"));
    assert!(matches!(results[1], MonkeySourceResult::Unknown { .. }));
    assert!(matches!(results[2], MonkeySourceResult::Unknown { .. }));
    assert!(
        import_monkey_callbacks(
            &C11TranslationUnit {
                source: FIXTURE,
                ..Default::default()
            },
            "wrong"
        )
        .is_err()
    );
}

#[test]
fn symbolic_references_require_prior_producers_with_matching_types() {
    let read = SourceOperation::Read {
        place: value("int".into(), SourceExpression::Parameter { index: 0 }),
    };
    for (operation, c_type) in [(0, "int"), (1, "int"), (0, "short")] {
        let path = MonkeySourcePath {
            operations: vec![
                read.clone(),
                SourceOperation::Guard {
                    condition: value(c_type.into(), SourceExpression::Symbol { operation }),
                    taken: true,
                },
            ],
        };
        assert_eq!(
            check_path_symbols(&path).is_ok(),
            operation == 0 && c_type == "int"
        );
    }
    let path = MonkeySourcePath {
        operations: vec![
            SourceOperation::Return,
            SourceOperation::Guard {
                condition: value("int".into(), SourceExpression::Symbol { operation: 0 }),
                taken: true,
            },
        ],
    };
    assert!(check_path_symbols(&path).is_err());
}

#[test]
fn nested_blocks_end_storage_and_early_returns_keep_only_executed_effects() {
    if !crate::test_support::clang_available() {
        return;
    }
    let results = import(&with_event_body(
        "if (flags) { int x = 1; return; } int y = 2; perror(\"later\");",
    ));
    let paths = summary(&results, 0).paths();
    assert_eq!(paths.len(), 2);
    assert!(calls(&paths[0]).is_empty());
    assert_eq!(calls(&paths[1]), ["perror"]);
    for path in paths {
        assert_eq!(
            path.operations
                .iter()
                .filter(|op| matches!(op, SourceOperation::LocalWrite { .. }))
                .count(),
            1
        );
        assert_eq!(
            path.operations
                .iter()
                .filter(|op| matches!(op, SourceOperation::LocalEnd { .. }))
                .count(),
            1
        );
        assert_eq!(path.operations.last(), Some(&SourceOperation::Return));
        check_path_symbols(path).unwrap();
    }
}

#[test]
fn increment_and_compound_assignment_evaluate_nested_lvalue_once() {
    if !crate::test_support::clang_available() {
        return;
    }
    for statement in ["map->ctx->fired_count++;", "map->ctx->fired_count |= 1;"] {
        let results = import(&with_event_body(&format!(
            "struct ev_map *map = data; {statement}"
        )));
        let path = &summary(&results, 0).paths()[0];
        let reads: Vec<_> = path
            .operations
            .iter()
            .filter_map(|op| {
                if let SourceOperation::Read { place } = op {
                    Some(place)
                } else {
                    None
                }
            })
            .collect();
        assert_eq!(reads.len(), 2);
        assert!(
            matches!(&reads[0].expression, SourceExpression::Field { name, .. } if name == "ctx")
        );
        let write = path
            .operations
            .iter()
            .find_map(|op| {
                if let SourceOperation::Write { place, .. } = op {
                    Some(place)
                } else {
                    None
                }
            })
            .unwrap();
        assert_eq!(reads[1], write);
    }
}

fn wrappers(source: &str) -> Vec<MonkeySourceResult> {
    import_monkey_wrappers(
        &C11TranslationUnit {
            source,
            ..Default::default()
        },
        PINNED_FLUENT_BIT_REVISION,
    )
    .unwrap()
}

#[test]
fn wrapper_import_retains_nullable_and_registration_failure_exits() {
    if !crate::test_support::clang_available() {
        return;
    }
    let results = wrappers(include_str!("../fixtures/monkey/wrappers.c"));
    assert_eq!(results.len(), 15);
    let create = summary(&results, 2);
    assert_eq!(create.paths().len(), 4, "{create:?}");
    assert_eq!(calls(&create.paths()[0]), ["_mk_event_init"]);
    assert_eq!(
        calls(&create.paths()[1]),
        ["_mk_event_init", "mk_mem_alloc_z"]
    );
    assert_eq!(
        calls(&create.paths()[2]),
        [
            "_mk_event_init",
            "mk_mem_alloc_z",
            "event_base_new",
            "mk_mem_alloc_z",
            "mk_mem_free"
        ]
    );
    assert_eq!(
        calls(&create.paths()[3]),
        [
            "_mk_event_init",
            "mk_mem_alloc_z",
            "event_base_new",
            "mk_mem_alloc_z"
        ]
    );
    for path in create.paths() {
        assert!(matches!(
            path.operations.last(),
            Some(SourceOperation::ReturnValue { .. })
        ));
    }
    let update = summary(&results, 4);
    assert_eq!(update.paths().len(), 24);
    let mut endings = BTreeSet::new();
    for path in update.paths() {
        let functions = calls(path);
        assert_eq!(&functions[..3], ["event_del", "event_free", "event_new"]);
        // Registration failure does not invent a second event_free cleanup.
        assert_eq!(
            functions
                .iter()
                .filter(|name| **name == "event_free")
                .count(),
            1
        );
        endings.insert(functions.last().copied().unwrap());
        check_path_symbols(path).unwrap();
    }
    assert_eq!(endings, BTreeSet::from(["event_new", "event_add"]));
    assert_eq!(
        calls(&summary(&results, 3).paths()[0]),
        ["event_base_free", "mk_mem_free", "mk_mem_free"]
    );
    assert!(matches!(&results[5], MonkeySourceResult::Summary(_)));
}

#[test]
fn wrapper_mutations_cannot_erase_unsupported_paths_or_invent_return_values() {
    if !crate::test_support::clang_available() {
        return;
    }
    let original = include_str!("../fixtures/monkey/wrappers.c");
    for source in [
        original.replace("return ctx;", "if (size) return ctx;"),
        original.replace("return ctx;", "return ctx; while (0) {}"),
        original.replace("ctx->base = event_base_new();", "struct mk_event_ctx **escaped = &ctx; mk_mem_free(escaped); ctx->base = event_base_new();"),
    ] {
        assert!(matches!(&wrappers(&source)[2], MonkeySourceResult::Unknown { .. }));
    }
    let source = original.replace("if (ret < 0)", "if (ret > 0)");
    assert_ne!(
        summary(&wrappers(original), 4),
        summary(&wrappers(&source), 4)
    );
}

#[test]
fn wrapper_constants_and_stack_initializers_retain_source_values() {
    if !crate::test_support::clang_available() {
        return;
    }
    let original = include_str!("../fixtures/monkey/wrappers.c");
    let results = wrappers(original);
    assert_eq!(
        calls(&summary(&results, 0).paths()[0]),
        ["evutil_socketpair"]
    );
    let changed = wrappers(&original.replace("SOCK_STREAM = 1", "SOCK_STREAM = 7"));
    assert_ne!(summary(&results, 0), summary(&changed, 0));
    let wait = summary(&results, 13);
    assert_eq!(wait.paths().len(), 3);
    assert_eq!(
        calls(&wait.paths()[2]),
        [
            "event_new",
            "event_add",
            "_mk_event_wait_with_flags",
            "event_free"
        ]
    );
    for path in wait.paths() {
        assert!(path.operations.iter().any(|op| matches!(op, SourceOperation::LocalStart { c_type, .. } if c_type == "struct timeval")));
        assert!(path.operations.iter().any(|op| matches!(op, SourceOperation::LocalWrite { value: SourceValue { expression: SourceExpression::Aggregate { elements }, .. }, .. } if elements.len() == 2)));
        check_path_symbols(path).unwrap();
    }
    for replacement in [
        "struct timeval timev = {timeout / 1000};",
        "int timev[timeout];",
    ] {
        let changed = original.replace(
            "struct timeval timev = {timeout / 1000, (timeout % 1000) * 1000};",
            replacement,
        );
        assert!(matches!(
            &wrappers(&changed)[13],
            MonkeySourceResult::Unknown { .. }
        ));
    }
}

#[test]
fn stack_socketpair_cells_remain_memory_reads_after_opaque_calls() {
    if !crate::test_support::clang_available() {
        return;
    }
    let original = include_str!("../fixtures/monkey/wrappers.c");
    let original = original.replace("_mk_event_channel_create", "_fixture_channel_create");
    let source = format!(
        "{original}\nint _mk_event_channel_create(int *out) {{ int fd[2]; int ret = mk_event_libevent_socketpair(fd); if (ret == -1) return -1; *out = fd[1]; return fd[0]; }}"
    );
    let results = wrappers(&source);
    let channel = summary(&results, 9);
    assert_eq!(channel.paths().len(), 2);
    assert_eq!(
        channel.paths()[1]
            .operations
            .iter()
            .filter(|op| matches!(op, SourceOperation::Read { .. }))
            .count(),
        2
    );
    assert!(
        channel.paths()[0]
            .operations
            .iter()
            .any(|op| matches!(op, SourceOperation::LocalEnd { local: 1 }))
    );
    let source = source.replace("int fd[2];", "int fd[2] = {1, 2};");
    assert!(matches!(
        &wrappers(&source)[9],
        MonkeySourceResult::Unknown { .. }
    ));
}

#[test]
fn aggregate_symbol_validation_rejects_nested_forward_reference() {
    let path = MonkeySourcePath {
        operations: vec![SourceOperation::LocalWrite {
            local: 0,
            value: value(
                "struct timeval".into(),
                SourceExpression::Aggregate {
                    elements: vec![value(
                        "long".into(),
                        SourceExpression::Symbol { operation: 0 },
                    )],
                },
            ),
        }],
    };
    assert!(check_path_symbols(&path).is_err());
}

#[test]
fn wrapper_assertions_retain_abort_paths_and_stop_following_calls() {
    if !crate::test_support::clang_available() {
        return;
    }
    let results = wrappers(include_str!("../fixtures/monkey/wrappers.c"));
    for index in [5, 6, 7, 9] {
        let imported = summary(&results, index);
        assert!(
            imported
                .paths()
                .iter()
                .any(|path| matches!(path.operations.last(), Some(SourceOperation::Terminate)))
        );
        assert!(imported.paths().iter().any(|path| matches!(
            path.operations.last(),
            Some(SourceOperation::ReturnValue { .. })
        )));
        for path in imported.paths() {
            if calls(path).contains(&"abort") {
                assert_eq!(calls(path).last(), Some(&"abort"));
                assert_eq!(path.operations.last(), Some(&SourceOperation::Terminate));
            }
            check_path_symbols(path).unwrap();
        }
    }
}

#[test]
fn single_iteration_macros_do_not_erase_control_or_terminal_failures() {
    if !crate::test_support::clang_available() {
        return;
    }
    let prefix = "void stop(void) __attribute__((noreturn)); void opaque(void);";
    let source = format!(
        "{prefix} int _mk_event_init(void) {{ do {{ if (1) stop(); opaque(); }} while (0); return 4; }}"
    );
    let results = wrappers(&source);
    let imported = summary(&results, 1);
    assert_eq!(imported.paths().len(), 2);
    assert_eq!(calls(&imported.paths()[0]), ["stop"]);
    assert_eq!(
        imported.paths()[0].operations.last(),
        Some(&SourceOperation::Terminate)
    );
    assert_eq!(calls(&imported.paths()[1]), ["opaque"]);
    assert!(imported.paths()[1].operations.iter().any(|op| matches!(op, SourceOperation::Guard { taken: false, condition: SourceValue { expression: SourceExpression::Integer { value }, .. } } if value == "0")));
    for replacement in ["while (1)", "while (0 + 0)"] {
        assert!(matches!(
            &wrappers(&source.replace("while (0)", replacement))[1],
            MonkeySourceResult::Unknown { .. }
        ));
    }
    for body in [
        "stop(); while (0) {}",
        "do { break; } while (0)",
        "do { continue; } while (0)",
    ] {
        let source = format!("{prefix} int _mk_event_init(void) {{ {body}; return 0; }}");
        assert!(matches!(
            &wrappers(&source)[1],
            MonkeySourceResult::Unknown { .. }
        ));
    }
    let source = "int stop(void) __attribute__((noreturn)); int _mk_event_init(void) { int x = stop(); return x; }";
    assert!(matches!(
        &wrappers(source)[1],
        MonkeySourceResult::Unknown { .. }
    ));
}

#[test]
fn stack_typedefs_require_the_actual_int_element_type() {
    if !crate::test_support::clang_available() {
        return;
    }
    for ty in ["long", "volatile int", "int[3]"] {
        let typedef = if ty == "int[3]" {
            "typedef int slot_t[3];".to_string()
        } else {
            format!("typedef {ty} slot_t;")
        };
        let source = format!("{typedef} int _mk_event_init(void) {{ slot_t slots[2]; return 0; }}");
        assert!(matches!(
            &wrappers(&source)[1],
            MonkeySourceResult::Unknown { .. }
        ));
    }
}
