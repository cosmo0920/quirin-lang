use super::*;
use crate::chunk_lifecycle::{
    ChunkBackend, ChunkProgram, ChunkResidency, check_chunk_program, execute_chunk_prefix,
};

fn import(body: &str) -> Vec<ChunkSourceResult> {
    import_chunk_source(
        &C11TranslationUnit {
            source: &format!(
                "struct flb_input_chunk {{ int busy; int other; }};\n\
                 struct cio_chunk {{ int lock; }};\n{body}"
            ),
            ..Default::default()
        },
        PINNED_FLUENT_BIT_REVISION,
    )
    .unwrap()
}

fn release_summary() -> ChunkSourceSummary {
    let results = import(
        "int release(struct flb_input_chunk *ic) {\n\
         if (ic->busy == 0) { return -1; }\n\
         ic->busy = 0; return 0; }",
    );
    let [ChunkSourceResult::Summary(summary)] = results.as_slice() else {
        panic!("expected source summary: {results:?}")
    };
    summary.clone()
}

fn prefix() -> Vec<ChunkOperation> {
    vec![
        ChunkOperation::CreateContext { context: 3 },
        ChunkOperation::CreateStream {
            context: 3,
            stream: 4,
        },
        ChunkOperation::OpenChunk {
            context: 3,
            stream: 4,
            chunk: 7,
            backend: ChunkBackend::File,
            residency: ChunkResidency::Up,
        },
    ]
}

#[test]
fn derives_both_returns_and_instantiates_the_actual_identity() {
    let summary = release_summary();
    assert_eq!(summary.field(), ChunkSourceField::Busy);
    assert_eq!(summary.paths()[0].return_value, -1);
    assert_eq!(summary.paths()[0].branches, [true]);
    assert!(summary.paths()[0].writes.is_empty());
    assert_eq!(summary.paths()[1].return_value, 0);
    assert_eq!(summary.paths()[1].branches, [false]);
    assert_eq!(summary.paths()[1].writes, [false]);

    let mut operations = prefix();
    let state = execute_chunk_prefix(&operations).unwrap().final_state;
    assert_eq!(summary.instantiate(&state, 7).unwrap(), (-1, vec![]));
    assert!(summary.instantiate(&state, 8).is_err());
    operations.push(ChunkOperation::SetBusy {
        chunk: 7,
        busy: true,
    });
    let state = execute_chunk_prefix(&operations).unwrap().final_state;
    let (result, effects) = summary.instantiate(&state, 7).unwrap();
    assert_eq!(result, 0);
    assert_eq!(
        effects,
        [ChunkOperation::SetBusy {
            chunk: 7,
            busy: false
        }]
    );
    operations.extend(effects);
    operations.push(ChunkOperation::DestroyContext { context: 3 });
    let certificate = check_chunk_program(&ChunkProgram { operations }).unwrap();
    assert!(!certificate.final_state.chunks[&7].live);
    assert!(summary.instantiate(&certificate.final_state, 7).is_err());
}

#[test]
fn releasing_wrapper_busy_does_not_discharge_chunkio_lock() {
    let summary = release_summary();
    let mut operations = prefix();
    operations.push(ChunkOperation::Lock { chunk: 7 });
    operations.push(ChunkOperation::SetBusy {
        chunk: 7,
        busy: true,
    });
    let state = execute_chunk_prefix(&operations).unwrap().final_state;
    operations.extend(summary.instantiate(&state, 7).unwrap().1);
    let state = execute_chunk_prefix(&operations).unwrap().final_state;
    assert!(state.chunks[&7].locked);
    assert!(!state.chunks[&7].busy);
    operations.push(ChunkOperation::DestroyContext { context: 3 });
    assert!(check_chunk_program(&ChunkProgram { operations }).is_err());
}

#[test]
fn typed_ast_preserves_else_and_ordered_writes() {
    let results = import(
        "int toggle(struct cio_chunk *ch) {\n\
         if (ch->lock != 0) { ch->lock = 0; } else { ch->lock = !0; }\n\
         if (!ch->lock) { ch->lock = 1; ch->lock = 0; }\n\
         return ch->lock; }",
    );
    let [ChunkSourceResult::Summary(summary)] = results.as_slice() else {
        panic!("{results:?}")
    };
    assert_eq!(summary.paths()[0].writes, [true]);
    assert_eq!(summary.paths()[0].return_value, 1);
    assert_eq!(summary.paths()[1].writes, [false, true, false]);
    assert_eq!(summary.paths()[1].return_value, 0);
}

#[test]
fn rejects_unmodeled_effects_even_on_unexecuted_paths() {
    for body in [
        "extern void opaque(void); int f(struct flb_input_chunk *p) { opaque(); return p->busy; }",
        "extern void opaque(void); int f(struct flb_input_chunk *p) { if (0) opaque(); return p->busy; }",
        "extern void opaque(void); int f(struct flb_input_chunk *p) { return p->busy; opaque(); }",
        "int f(struct flb_input_chunk *p) { p->other = 0; return p->busy; }",
        "int f(struct flb_input_chunk *p) { struct flb_input_chunk *q = p; q->busy = 0; return 0; }",
        "int f(struct flb_input_chunk *p) { while(p->busy) p->busy = 0; return 0; }",
        "int f(struct flb_input_chunk *p) { p->busy = 2; return 0; }",
        "int f(struct flb_input_chunk *p) { if(p->busy) return 0; }",
        "int f(struct flb_input_chunk *p) { return (p->busy++, 0); }",
    ] {
        assert!(
            matches!(import(body).as_slice(), [ChunkSourceResult::Unknown { .. }]),
            "unexpected acceptance: {body}"
        );
    }
}

#[test]
fn source_configuration_and_parameter_qualifiers_are_not_erased() {
    let source = "struct flb_input_chunk { int busy; };\n\
        #ifdef INCLUDE_F\n\
        int f(struct flb_input_chunk *p) { p->busy = 0; return 0; }\n\
        #endif\n";
    let mut unit = C11TranslationUnit {
        source,
        ..Default::default()
    };
    assert!(matches!(
        import_chunk_source(&unit, PINNED_FLUENT_BIT_REVISION)
            .unwrap()
            .as_slice(),
        [ChunkSourceResult::Unknown { .. }]
    ));
    unit.definitions.push("INCLUDE_F".into());
    assert!(matches!(
        import_chunk_source(&unit, PINNED_FLUENT_BIT_REVISION)
            .unwrap()
            .as_slice(),
        [ChunkSourceResult::Summary(_)]
    ));
    assert!(import_chunk_source(&unit, "other revision").is_err());
    assert!(matches!(
        import("int f(volatile struct flb_input_chunk *p) { return p->busy; }").as_slice(),
        [ChunkSourceResult::Unknown { .. }]
    ));
}

#[test]
fn exact_ast_definition_selection_does_not_confuse_name_prefixes() {
    let results = import(
        "int f(struct flb_input_chunk *p) { return p->busy; }\n\
         int f_extra(struct flb_input_chunk *p) { return !p->busy; }",
    );
    let [
        ChunkSourceResult::Summary(first),
        ChunkSourceResult::Summary(second),
    ] = results.as_slice()
    else {
        panic!("{results:?}")
    };
    assert_eq!(first.function(), "f");
    assert_eq!(first.paths()[1].return_value, 1);
    assert_eq!(second.function(), "f_extra");
    assert_eq!(second.paths()[1].return_value, 0);
}

#[test]
fn bitfields_and_volatile_members_cannot_receive_boolean_field_summaries() {
    for declaration in ["int busy : 1", "volatile int busy", "_Atomic int busy"] {
        let source = format!(
            "struct flb_input_chunk {{ {declaration}; }};\n\
             int f(struct flb_input_chunk *p) {{ p->busy = 1; return p->busy; }}"
        );
        let results = import_chunk_source(
            &C11TranslationUnit {
                source: &source,
                ..Default::default()
            },
            PINNED_FLUENT_BIT_REVISION,
        )
        .unwrap();
        assert!(
            matches!(results.as_slice(), [ChunkSourceResult::Unknown { .. }]),
            "{declaration}"
        );
    }
}
