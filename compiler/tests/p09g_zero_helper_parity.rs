//! P09-G parity: the zero-helper convenience wrapper versus the surviving
//! explicit-helpers entry point.
//!
//! `chunk_source::import_chunk_source(unit, revision)` is a convenience
//! overload that delegates to
//! `chunk_source::import_chunk_source_with_helpers(unit, revision, &[])`.
//! This harness measures, on every source shape its two in-tree consumers
//! actually pass, that both entry points produce byte-identical serialised
//! results and byte-identical rejection text. It is temporary: its measurement
//! is archived under `docs/reports/remaining-p09-routes/` in the commit that
//! adds it, and it is removed in the commit that deletes the wrapper.
use qcore_compiler::chunk_source::{ChunkSourceResult, import_chunk_source};
use qcore_compiler::external_integration::PINNED_FLUENT_BIT_REVISION;
use qcore_compiler::legacy_c::C11TranslationUnit;
use qcore_compiler::verifier_plugins::chunkio::chunk_source::import_chunk_source_with_helpers;

const PRELUDE: &str = "struct flb_input_chunk { int busy; int other; };\n\
                       struct cio_chunk { int lock; };\n";

/// Exactly the bodies the two consuming tests import today.
fn cases() -> Vec<(&'static str, String, &'static str, Vec<String>)> {
    let with_prelude = |body: &str| format!("{PRELUDE}{body}");
    vec![
        (
            "release_summary",
            with_prelude(
                "int release(struct flb_input_chunk *ic) {\n\
                 if (ic->busy == 0) { return -1; }\n\
                 ic->busy = 0; return 0; }",
            ),
            PINNED_FLUENT_BIT_REVISION,
            Vec::new(),
        ),
        (
            "busy_summary",
            with_prelude(
                "int flb_input_chunk_release_lock(struct flb_input_chunk *ic) { if(ic->busy == 0) return -1; ic->busy = 0; return 0; }",
            ),
            PINNED_FLUENT_BIT_REVISION,
            Vec::new(),
        ),
        (
            "lock_summary",
            with_prelude(
                "int cio_chunk_unlock(struct cio_chunk *ch) { if (ch->lock == 0) { return -1; } ch->lock = 0; return 0; }",
            ),
            PINNED_FLUENT_BIT_REVISION,
            Vec::new(),
        ),
        (
            "volatile_pointer_is_unknown",
            with_prelude("int f(volatile struct flb_input_chunk *p) { return p->busy; }"),
            PINNED_FLUENT_BIT_REVISION,
            Vec::new(),
        ),
        (
            "definition_absent_after_preprocessing",
            "struct flb_input_chunk { int busy; };\n\
             #ifdef INCLUDE_F\n\
             int f(struct flb_input_chunk *p) { p->busy = 0; return 0; }\n\
             #endif\n"
                .into(),
            PINNED_FLUENT_BIT_REVISION,
            Vec::new(),
        ),
        (
            "definition_present_after_preprocessing",
            "struct flb_input_chunk { int busy; };\n\
             #ifdef INCLUDE_F\n\
             int f(struct flb_input_chunk *p) { p->busy = 0; return 0; }\n\
             #endif\n"
                .into(),
            PINNED_FLUENT_BIT_REVISION,
            vec!["INCLUDE_F".to_owned()],
        ),
        (
            "revision_mismatch_rejects",
            with_prelude("int f(struct flb_input_chunk *p) { p->busy = 0; return 0; }"),
            "other revision",
            Vec::new(),
        ),
    ]
}

fn unit<'a>(source: &'a str, definitions: &[String]) -> C11TranslationUnit<'a> {
    C11TranslationUnit {
        source,
        definitions: definitions.to_vec(),
        ..Default::default()
    }
}

fn render(result: &Result<Vec<ChunkSourceResult>, String>) -> String {
    match result {
        Ok(results) => serde_json::to_string(results).expect("serialisable results"),
        Err(reason) => serde_json::to_string(&serde_json::json!({ "error": reason }))
            .expect("serialisable rejection"),
    }
}

fn clang_available() -> bool {
    match qcore_c_gateway::clang_command() {
        Ok(_) => true,
        Err(reason) => {
            assert!(
                std::env::var_os("QCORE_CLANG").is_none(),
                "explicit QCORE_CLANG is invalid: {reason}"
            );
            eprintln!("skipping Clang-dependent P09-G parity harness: {reason}");
            false
        }
    }
}

#[test]
fn the_zero_helper_wrapper_and_the_explicit_empty_helper_call_agree_everywhere() {
    if !clang_available() {
        return;
    }
    let mut archive = Vec::new();
    for (name, source, revision, definitions) in cases() {
        let old = import_chunk_source(&unit(&source, &definitions), revision);
        let new = import_chunk_source_with_helpers(&unit(&source, &definitions), revision, &[]);
        let old_rendered = render(&old);
        let new_rendered = render(&new);
        assert_eq!(old_rendered, new_rendered, "{name}");
        // The measurement must not be vacuous: every case has to reach a real
        // decision, not a shared early error, except the deliberate mismatch.
        if name == "revision_mismatch_rejects" {
            assert!(old.is_err() && new.is_err(), "{name} must reject on both");
        } else {
            assert!(
                old.is_ok() && new.is_ok(),
                "{name} must import on both routes"
            );
        }
        archive.push(serde_json::json!({
            "case": name,
            "revision": revision,
            "definitions": definitions,
            "outcome": if old.is_ok() { "results" } else { "rejected" },
            "results": if old.is_ok() {
                serde_json::Value::Number(serde_json::Number::from(
                    old.as_ref().map(Vec::len).unwrap_or_default() as u64,
                ))
            } else {
                serde_json::Value::Null
            },
            "identical": old_rendered == new_rendered,
            "serialized_bytes": old_rendered.len(),
            "digest": qcore_schemas::sha256_hex(old_rendered.as_bytes()),
        }));
    }
    let summaries = archive
        .iter()
        .filter(|entry| entry["outcome"] == "results")
        .count();
    assert_eq!(summaries, 6, "six importing cases and one rejection");
    if let Some(path) = std::env::var_os("QCORE_P09G_PARITY_OUT") {
        let document = serde_json::json!({
            "measurement": "p09-g zero-helper wrapper parity",
            "old_route": "qcore_compiler::chunk_source::import_chunk_source(unit, revision)",
            "new_route": "qcore_compiler::verifier_plugins::chunkio::chunk_source::import_chunk_source_with_helpers(unit, revision, &[])",
            "pinned_revision": PINNED_FLUENT_BIT_REVISION,
            "clang": qcore_c_gateway::clang_command().map(|c| format!("{c:?}")).unwrap_or_default(),
            "cases": archive,
        });
        std::fs::write(
            &path,
            serde_json::to_vec_pretty(&document).expect("serialisable archive"),
        )
        .expect("archive written");
    }
}
