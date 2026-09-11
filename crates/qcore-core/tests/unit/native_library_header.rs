//! The exported-header level of `source.native-library.v1` (card
//! `NATIVE-LIBRARY-HEADER`).
//!
//! Every refusal below is a *named* one and has its own row, so the subset
//! boundary cannot decay into a lenient reading one condition at a time. The
//! rows are grouped as the module is: the directive layer, the declaration
//! reader, the one-directional totality, the digest gate, and the comparison
//! whose disagreements **refuse**.
use super::*;

use crate::native_library::{NativeLibraries, load_native_library_manifest};

const ARTIFACT_HASH: &str =
    "sha256:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
const HEADER_DIGEST: &str =
    "sha256:1111111111111111111111111111111111111111111111111111111111111111";
const OTHER_DIGEST: &str =
    "sha256:2222222222222222222222222222222222222222222222222222222222222222";

/// A manifest naming one exported header, with the entry lines the caller gives.
fn manifest_text(entries: &str) -> String {
    format!(
        "native_library manifest v1;\n\
         library demo;\n\
         version 0.1.0;\n\
         artifact libdemo.a;\n\
         hash {ARTIFACT_HASH};\n\
         data_model lp64;\n\
         unwinding abort;\n\
         allocator_custody closed;\n\
         thread_posture single_caller;\n\
         header include/demo.h;\n\
         header_hash {HEADER_DIGEST};\n\
         {entries}\n"
    )
}

/// The same manifest with **no** header binding — the wave-19/wave-20 world.
fn headerless_text(entries: &str) -> String {
    format!(
        "native_library manifest v1;\n\
         library demo;\n\
         version 0.1.0;\n\
         artifact libdemo.a;\n\
         hash {ARTIFACT_HASH};\n\
         data_model lp64;\n\
         unwinding abort;\n\
         allocator_custody closed;\n\
         thread_posture single_caller;\n\
         {entries}\n"
    )
}

fn libraries(text: &str) -> NativeLibraries {
    let mut sources = BTreeMap::new();
    sources.insert("demo.qcorelib".to_owned(), text.to_owned());
    NativeLibraries::load(&sources).expect("the manifest loads")
}

/// Run the header gate over one manifest text and one header text, with the
/// digest the manifest declares.
fn check(entries: &str, header: &str) -> Result<(), HeaderFailure> {
    check_with_digest(entries, header, HEADER_DIGEST)
}

fn check_with_digest(entries: &str, header: &str, measured: &str) -> Result<(), HeaderFailure> {
    let text = manifest_text(entries);
    let libraries = libraries(&text);
    let mut sources = BTreeMap::new();
    sources.insert(
        "include/demo.h".to_owned(),
        HeaderSource {
            reference: "include/demo.h",
            text: header,
            measured_digest: measured,
        },
    );
    check_header_bindings(&libraries, &sources)
}

/// The refusal a failure carries, or a panic naming what it was instead.
fn refusal(failure: HeaderFailure) -> HeaderRefusal {
    match failure {
        HeaderFailure::Unreadable(refusal) => refusal,
        other => panic!("expected an unreadable-header refusal, got {other:?}"),
    }
}

fn disagreement(failure: HeaderFailure) -> HeaderDisagreement {
    match failure {
        HeaderFailure::Disagreement { disagreement, .. } => disagreement,
        other => panic!("expected a disagreement, got {other:?}"),
    }
}

/// A guarded header wrapping the caller's body, in the shape a real exported
/// header has.
fn guarded(body: &str) -> String {
    format!("#ifndef DEMO_H\n#define DEMO_H\n{body}\n#endif /* DEMO_H */\n")
}

// ---------------------------------------------------------------------------
// The payoff: a manifest bound to a header that agrees
// ---------------------------------------------------------------------------

#[test]
fn a_manifest_and_the_header_it_names_agree() {
    let header = guarded(
        "#include <stdint.h>\n\
         /* The exported surface. */\n\
         uint8_t *demo_alloc(uintptr_t size);\n\
         void demo_fill(uint8_t *buffer, uint32_t value);\n\
         int32_t demo_sum(const uint8_t *buffer, uint32_t count);\n\
         void demo_release(uint8_t *buffer);\n",
    );
    let entries = "contract fn demo_alloc(uintptr_t) -> uint8_t * from contracts/demo.qcore;\n\
                   contract fn demo_fill(uint8_t *, uint32_t) -> void from contracts/demo.qcore;\n\
                   contract fn demo_sum(const uint8_t *, uint32_t) -> int32_t from contracts/demo.qcore;\n\
                   contract fn demo_release(uint8_t *) -> void from contracts/demo.qcore;";
    assert_eq!(check(entries, &header), Ok(()));
}

#[test]
fn a_contract_less_entry_is_compared_by_the_same_gate() {
    let header = guarded("int32_t demo_add(int32_t left, int32_t right);");
    let entries = "unchecked fn demo_add(int32_t, int32_t) -> int32_t;";
    assert_eq!(check(entries, &header), Ok(()));
}

#[test]
fn a_manifest_naming_no_header_is_unaffected_by_any_supplied_text() {
    let text = headerless_text("unchecked fn demo_add(int32_t, int32_t) -> int32_t;");
    let libraries = libraries(&text);
    // Not merely "no source matched": there is a source, and it contradicts
    // every entry. A manifest that did not opt in is not checked against it.
    let mut sources = BTreeMap::new();
    sources.insert(
        "include/demo.h".to_owned(),
        HeaderSource {
            reference: "include/demo.h",
            text: "void demo_add(void);",
            measured_digest: HEADER_DIGEST,
        },
    );
    assert_eq!(check_header_bindings(&libraries, &sources), Ok(()));
}

// ---------------------------------------------------------------------------
// The digest gate
// ---------------------------------------------------------------------------

#[test]
fn a_header_the_manifest_names_and_nobody_supplied_refuses() {
    let text = manifest_text("unchecked fn demo_add(int32_t, int32_t) -> int32_t;");
    let libraries = libraries(&text);
    let failure = check_header_bindings(&libraries, &BTreeMap::new()).unwrap_err();
    assert_eq!(
        refusal(failure),
        HeaderRefusal::NotSupplied {
            manifest: "demo.qcorelib".to_owned(),
            reference: "include/demo.h".to_owned(),
        }
    );
}

#[test]
fn a_measured_digest_that_is_not_the_declared_one_refuses() {
    let header = guarded("int32_t demo_add(int32_t left, int32_t right);");
    let failure = check_with_digest(
        "unchecked fn demo_add(int32_t, int32_t) -> int32_t;",
        &header,
        OTHER_DIGEST,
    )
    .unwrap_err();
    assert_eq!(
        refusal(failure),
        HeaderRefusal::DigestMismatch {
            manifest: "demo.qcorelib".to_owned(),
            reference: "include/demo.h".to_owned(),
            declared: HEADER_DIGEST.to_owned(),
            measured: OTHER_DIGEST.to_owned(),
        }
    );
}

#[test]
fn the_digest_is_compared_before_the_text_is_read_at_all() {
    // The header text is unreadable in three separate ways. The digest gate
    // still decides, so a substituted header cannot be diagnosed as a reader
    // boundary of the header the manifest meant.
    let failure = check_with_digest(
        "unchecked fn demo_add(int32_t, int32_t) -> int32_t;",
        "#pragma once\n#if\nvoid demo_add();\n",
        OTHER_DIGEST,
    )
    .unwrap_err();
    assert!(matches!(
        refusal(failure),
        HeaderRefusal::DigestMismatch { .. }
    ));
}

// ---------------------------------------------------------------------------
// The directive layer
// ---------------------------------------------------------------------------

#[test]
fn a_directive_outside_the_v1_subset_refuses_the_whole_header() {
    for directive in [
        "pragma once",
        "line 40 \"other.h\"",
        "error nope",
        "warning hm",
    ] {
        let header = format!("#{directive}\nint32_t demo_add(int32_t, int32_t);\n");
        let failure = check(
            "unchecked fn demo_add(int32_t, int32_t) -> int32_t;",
            &header,
        )
        .unwrap_err();
        let keyword = directive.split_whitespace().next().expect("a keyword");
        assert_eq!(
            refusal(failure),
            HeaderRefusal::UnsupportedDirective {
                manifest: "demo.qcorelib".to_owned(),
                reference: "include/demo.h".to_owned(),
                directive: keyword.to_owned(),
            },
            "`#{directive}` must refuse by name"
        );
    }
}

#[test]
fn conditional_regions_that_do_not_close_refuse_the_whole_header() {
    let header = "#ifdef SOMETHING\nint32_t demo_add(int32_t, int32_t);\n";
    let failure = check(
        "unchecked fn demo_add(int32_t, int32_t) -> int32_t;",
        header,
    )
    .unwrap_err();
    assert_eq!(
        refusal(failure),
        HeaderRefusal::UnbalancedConditional {
            manifest: "demo.qcorelib".to_owned(),
            reference: "include/demo.h".to_owned(),
        }
    );
}

#[test]
fn a_declaration_only_inside_a_conditional_region_refuses_naming_that_reason() {
    let header =
        guarded("#ifdef DEMO_WITH_ADD\nint32_t demo_add(int32_t left, int32_t right);\n#endif\n");
    let failure = check(
        "unchecked fn demo_add(int32_t, int32_t) -> int32_t;",
        &header,
    )
    .unwrap_err();
    assert_eq!(
        refusal(failure),
        HeaderRefusal::ConditionalDeclaration {
            manifest: "demo.qcorelib".to_owned(),
            reference: "include/demo.h".to_owned(),
            symbol: "demo_add".to_owned(),
        },
        "a conditional-only declaration is not the same statement as an absent one"
    );
}

#[test]
fn the_include_guard_is_read_at_depth_zero_and_a_second_conditional_is_not() {
    // Both declarations sit inside the guard. Only the one inside the *inner*
    // conditional is unread, which is what makes the guard rule a rule about
    // the guard rather than about the first conditional it meets.
    let header = guarded(
        "int32_t demo_add(int32_t left, int32_t right);\n\
         #if defined(DEMO_EXTRA)\n\
         int32_t demo_extra(int32_t value);\n\
         #endif\n",
    );
    assert_eq!(
        check(
            "unchecked fn demo_add(int32_t, int32_t) -> int32_t;",
            &header
        ),
        Ok(())
    );
    assert_eq!(
        refusal(check("unchecked fn demo_extra(int32_t) -> int32_t;", &header).unwrap_err()),
        HeaderRefusal::ConditionalDeclaration {
            manifest: "demo.qcorelib".to_owned(),
            reference: "include/demo.h".to_owned(),
            symbol: "demo_extra".to_owned(),
        }
    );
}

#[test]
fn an_unguarded_header_reads_its_top_level_declarations() {
    let header = "int32_t demo_add(int32_t left, int32_t right);\n";
    assert_eq!(
        check(
            "unchecked fn demo_add(int32_t, int32_t) -> int32_t;",
            header
        ),
        Ok(())
    );
}

#[test]
fn an_ifndef_that_is_not_the_file_wide_guard_is_a_conditional_region() {
    // The `#ifndef` closes before the end of the file, so it is a conditional
    // region and not a guard. Reading it as one would admit exactly the
    // declarations v1 refuses to decide.
    let header = "#ifndef DEMO_H\n#define DEMO_H\nint32_t demo_add(int32_t, int32_t);\n#endif\nint32_t demo_tail(int32_t);\n";
    assert_eq!(
        refusal(
            check(
                "unchecked fn demo_add(int32_t, int32_t) -> int32_t;",
                header
            )
            .unwrap_err()
        ),
        HeaderRefusal::ConditionalDeclaration {
            manifest: "demo.qcorelib".to_owned(),
            reference: "include/demo.h".to_owned(),
            symbol: "demo_add".to_owned(),
        }
    );
}

#[test]
fn an_include_is_never_followed_so_a_type_it_defines_is_unreadable() {
    let header = guarded("#include <other.h>\nother_handle_t demo_open(int32_t slot);");
    let failure = check("unchecked fn demo_open(int32_t) -> uintptr_t;", &header).unwrap_err();
    assert_eq!(
        refusal(failure),
        HeaderRefusal::UnreadableResult {
            manifest: "demo.qcorelib".to_owned(),
            reference: "include/demo.h".to_owned(),
            symbol: "demo_open".to_owned(),
            spelling: "other_handle_t".to_owned(),
        }
    );
}

#[test]
fn a_symbol_the_header_defines_as_a_macro_is_not_an_exported_symbol() {
    let header = guarded("#define demo_add(a, b) ((a) + (b))");
    let failure = check(
        "unchecked fn demo_add(int32_t, int32_t) -> int32_t;",
        &header,
    )
    .unwrap_err();
    assert_eq!(
        refusal(failure),
        HeaderRefusal::MacroSymbol {
            manifest: "demo.qcorelib".to_owned(),
            reference: "include/demo.h".to_owned(),
            symbol: "demo_add".to_owned(),
        }
    );
}

#[test]
fn a_symbol_both_declared_and_defined_as_a_macro_is_still_a_macro() {
    // A caller reaches the macro, not the declaration, so the declaration is
    // not what the manifest's entry describes.
    let header = guarded(
        "int32_t demo_add(int32_t left, int32_t right);\n#define demo_add(a, b) ((a) + (b))",
    );
    assert!(matches!(
        refusal(
            check(
                "unchecked fn demo_add(int32_t, int32_t) -> int32_t;",
                &header
            )
            .unwrap_err()
        ),
        HeaderRefusal::MacroSymbol { .. }
    ));
}

// ---------------------------------------------------------------------------
// The declaration reader
// ---------------------------------------------------------------------------

#[test]
fn a_bound_symbol_the_header_does_not_declare_refuses() {
    let header = guarded("int32_t demo_other(int32_t value);");
    let failure = check(
        "unchecked fn demo_add(int32_t, int32_t) -> int32_t;",
        &header,
    )
    .unwrap_err();
    assert_eq!(
        refusal(failure),
        HeaderRefusal::UndeclaredSymbol {
            manifest: "demo.qcorelib".to_owned(),
            reference: "include/demo.h".to_owned(),
            symbol: "demo_add".to_owned(),
        }
    );
}

#[test]
fn declarations_the_manifest_does_not_name_are_ignored() {
    // The one-directional totality: a header is the library's whole exported
    // surface, so a declaration no entry binds is not a partial binding. The
    // unnamed declarations here are variadic, `static`, macro-decorated and
    // unreadable — every shape that refuses when it *is* named.
    let header = guarded(
        "int32_t demo_add(int32_t left, int32_t right);\n\
         int demo_printf(const char *format, ...);\n\
         static int demo_helper(int value);\n\
         DEMO_EXPORT int demo_decorated(int value);\n\
         other_t demo_opaque(void);\n",
    );
    assert_eq!(
        check(
            "unchecked fn demo_add(int32_t, int32_t) -> int32_t;",
            &header
        ),
        Ok(())
    );
}

#[test]
fn a_static_declaration_is_not_an_exported_symbol() {
    let header = guarded("static int32_t demo_add(int32_t left, int32_t right);");
    let failure = check(
        "unchecked fn demo_add(int32_t, int32_t) -> int32_t;",
        &header,
    )
    .unwrap_err();
    assert_eq!(
        refusal(failure),
        HeaderRefusal::InternalDeclaration {
            manifest: "demo.qcorelib".to_owned(),
            reference: "include/demo.h".to_owned(),
            symbol: "demo_add".to_owned(),
        },
        "internal linkage is a statement about the artifact, not a reader boundary"
    );
}

#[test]
fn a_static_inline_definition_is_refused_for_the_same_reason() {
    let header = guarded(
        "static inline int32_t demo_add(int32_t left, int32_t right)\n{\n    return left + right;\n}\n",
    );
    assert!(matches!(
        refusal(
            check(
                "unchecked fn demo_add(int32_t, int32_t) -> int32_t;",
                &header
            )
            .unwrap_err()
        ),
        HeaderRefusal::InternalDeclaration { .. }
    ));
}

#[test]
fn an_inline_definition_without_static_is_read() {
    // msgpack's shape: `inline` carries no linkage statement this reader
    // objects to, and the definition supplies the declaration.
    let header = guarded(
        "inline int32_t demo_add(int32_t left, int32_t right)\n{\n    return left + right;\n}\n",
    );
    assert_eq!(
        check(
            "unchecked fn demo_add(int32_t, int32_t) -> int32_t;",
            &header
        ),
        Ok(())
    );
}

#[test]
fn an_empty_parameter_list_specifies_nothing_and_refuses() {
    let header = guarded("int32_t demo_now();");
    let failure = check("unchecked fn demo_now() -> int32_t;", &header).unwrap_err();
    assert_eq!(
        refusal(failure),
        HeaderRefusal::UnspecifiedParameters {
            manifest: "demo.qcorelib".to_owned(),
            reference: "include/demo.h".to_owned(),
            symbol: "demo_now".to_owned(),
        },
        "a manifest entry asserts a parameter vector, so there is something to contradict"
    );
}

#[test]
fn a_void_parameter_list_is_an_empty_vector_and_agrees() {
    let header = guarded("int32_t demo_now(void);");
    assert_eq!(
        check("unchecked fn demo_now() -> int32_t;", &header),
        Ok(())
    );
}

#[test]
fn a_variadic_declaration_refuses() {
    let header = guarded("int32_t demo_log(const uint8_t *format, ...);");
    let failure = check(
        "contract fn demo_log(const uint8_t *) -> int32_t from contracts/demo.qcore;",
        &header,
    )
    .unwrap_err();
    assert_eq!(
        refusal(failure),
        HeaderRefusal::VariadicDeclaration {
            manifest: "demo.qcorelib".to_owned(),
            reference: "include/demo.h".to_owned(),
            symbol: "demo_log".to_owned(),
        }
    );
}

#[test]
fn two_declarations_that_project_differently_are_ambiguous() {
    let header = guarded(
        "int32_t demo_add(int32_t left, int32_t right);\n\
         int64_t demo_add(int32_t left, int32_t right);\n",
    );
    let failure = check(
        "unchecked fn demo_add(int32_t, int32_t) -> int32_t;",
        &header,
    )
    .unwrap_err();
    assert_eq!(
        refusal(failure),
        HeaderRefusal::AmbiguousDeclaration {
            manifest: "demo.qcorelib".to_owned(),
            reference: "include/demo.h".to_owned(),
            symbol: "demo_add".to_owned(),
        }
    );
}

#[test]
fn two_identical_declarations_compose_to_one_reading() {
    let header = guarded(
        "int32_t demo_add(int32_t left, int32_t right);\n\
         int32_t demo_add(int32_t left, int32_t right);\n",
    );
    assert_eq!(
        check(
            "unchecked fn demo_add(int32_t, int32_t) -> int32_t;",
            &header
        ),
        Ok(())
    );
}

#[test]
fn an_export_decoration_macro_makes_the_declaration_unreadable_by_name() {
    // The libevent shape, and the census's single dominant reason.
    let header = guarded("DEMO_EXPORT_SYMBOL\nint32_t demo_add(int32_t left, int32_t right);");
    let failure = check(
        "unchecked fn demo_add(int32_t, int32_t) -> int32_t;",
        &header,
    )
    .unwrap_err();
    assert_eq!(
        refusal(failure),
        HeaderRefusal::UnreadableResult {
            manifest: "demo.qcorelib".to_owned(),
            reference: "include/demo.h".to_owned(),
            symbol: "demo_add".to_owned(),
            spelling: "DEMO_EXPORT_SYMBOL int32_t".to_owned(),
        },
        "the refusal names the token, which is what makes the census actionable"
    );
}

#[test]
fn a_nested_declarator_parameter_is_unreadable() {
    let header = guarded("int32_t demo_apply(int (*callback)(void *));");
    assert!(matches!(
        refusal(check("unchecked fn demo_apply(uintptr_t) -> int32_t;", &header).unwrap_err()),
        HeaderRefusal::UnreadableParameter { parameter: 0, .. }
    ));
}

#[test]
fn an_undecorated_third_party_declaration_shape_is_inside_the_subset() {
    // ChunkIO's `chunkio/chunkio.h`, transcribed: plain declarations over
    // `struct` tags, `char *`, `char **`, `size_t` and `int`, with no export
    // decoration. The census measured 13 of that header's 15 declarations
    // inside v1; the header itself is a pinned Cycle 19 *build input* rather
    // than a file of this repository, so the shape is pinned here.
    let header = guarded(
        "#include <chunkio/cio_info.h>\n\
         struct cio_ctx *cio_create(struct cio_options *options);\n\
         void cio_destroy(struct cio_ctx *ctx);\n\
         int cio_load(struct cio_ctx *ctx, char *chunk_extension);\n\
         int cio_set_realloc_size_hint(struct cio_ctx *ctx, size_t realloc_size_hint);\n\
         int cio_meta_read(struct cio_chunk *ch, char **meta_buf, int *meta_len);\n",
    );
    let entries = "contract fn cio_create(uint8_t *) -> uint8_t * from contracts/demo.qcore;\n\
                   contract fn cio_destroy(uint8_t *) -> void from contracts/demo.qcore;\n\
                   contract fn cio_load(uint8_t *, uint8_t *) -> int32_t from contracts/demo.qcore;\n\
                   contract fn cio_set_realloc_size_hint(uint8_t *, uintptr_t) -> int32_t from contracts/demo.qcore;\n\
                   contract fn cio_meta_read(uint8_t *, uint8_t **, uint8_t *) -> int32_t from contracts/demo.qcore;";
    assert_eq!(check(entries, &header), Ok(()));
}

#[test]
fn an_inline_function_pointer_parameter_stays_outside_the_subset() {
    // The other two of ChunkIO's fifteen: a function-pointer parameter written
    // at the parameter position rather than behind a typedef. Every nested
    // declarator refuses, as it does in the gateway's reader.
    let header =
        guarded("int cio_qsort(struct cio_ctx *ctx, int (*compar)(const void *, const void *));");
    assert!(matches!(
        refusal(
            check(
                "contract fn cio_qsort(uint8_t *, uint8_t *) -> int32_t from contracts/demo.qcore;",
                &header,
            )
            .unwrap_err()
        ),
        HeaderRefusal::UnreadableParameter { parameter: 1, .. }
    ));
}

// ---------------------------------------------------------------------------
// Read, and unspellable in a manifest
// ---------------------------------------------------------------------------

#[test]
fn a_function_pointer_parameter_is_read_and_cannot_be_spelled() {
    // msgpack's `msgpack_packer_init` shape, exactly.
    let header = guarded(
        "typedef int (*demo_write)(void *data, const char *buffer, unsigned long length);\n\
         typedef struct demo_packer { void *data; demo_write callback; } demo_packer;\n\
         void demo_packer_init(demo_packer *packer, void *data, demo_write callback);\n",
    );
    let failure = check(
        "contract fn demo_packer_init(uint8_t *, uint8_t *, uint8_t *) -> void from contracts/demo.qcore;",
        &header,
    )
    .unwrap_err();
    assert_eq!(
        refusal(failure),
        HeaderRefusal::UnspellableParameter {
            manifest: "demo.qcorelib".to_owned(),
            reference: "include/demo.h".to_owned(),
            symbol: "demo_packer_init".to_owned(),
            parameter: 2,
            projection: "fnptr",
        },
        "the type is read; what is missing is a manifest spelling for it"
    );
}

#[test]
fn the_aggregate_typedef_beside_it_is_read_as_a_pointer() {
    // The first two parameters of the same declaration project and agree, which
    // is what makes the row above a *vocabulary* boundary rather than a reader
    // one: the reader committed to the aggregate the typedef defines.
    let header = guarded(
        "typedef struct demo_packer { int field; } demo_packer;\n\
         void demo_packer_reset(demo_packer *packer, void *data);\n",
    );
    assert_eq!(
        check(
            "contract fn demo_packer_reset(uint8_t *, uint8_t *) -> void from contracts/demo.qcore;",
            &header,
        ),
        Ok(())
    );
}

#[test]
fn a_floating_result_is_read_and_cannot_be_spelled() {
    let header = guarded("double demo_now(void);");
    let failure = check("unchecked fn demo_now() -> uint64_t;", &header).unwrap_err();
    assert_eq!(
        refusal(failure),
        HeaderRefusal::UnspellableResult {
            manifest: "demo.qcorelib".to_owned(),
            reference: "include/demo.h".to_owned(),
            symbol: "demo_now".to_owned(),
            projection: "f64",
        }
    );
}

#[test]
fn a_long_double_stays_unreadable_rather_than_folded_onto_a_spelling() {
    let header = guarded("long double demo_now(void);");
    assert!(matches!(
        refusal(check("unchecked fn demo_now() -> uint64_t;", &header).unwrap_err()),
        HeaderRefusal::UnreadableResult { .. }
    ));
}

// ---------------------------------------------------------------------------
// The set-valued projection
// ---------------------------------------------------------------------------

#[test]
fn a_plain_char_admits_both_of_its_byte_spellings() {
    let header = guarded("void demo_take(char value);");
    for spelling in ["int8_t", "uint8_t"] {
        assert_eq!(
            check(
                &format!("unchecked fn demo_take({spelling}) -> void;"),
                &header
            ),
            Ok(()),
            "plain `char` leaves signedness implementation-defined (C11 6.2.5p15)"
        );
    }
}

#[test]
fn an_unsigned_long_admits_both_of_its_lp64_spellings() {
    let header = guarded("void demo_take(unsigned long value);");
    for spelling in ["uint64_t", "uintptr_t"] {
        assert_eq!(
            check(
                &format!("unchecked fn demo_take({spelling}) -> void;"),
                &header
            ),
            Ok(()),
            "a faithful `size_t` transcription is not a disagreement on LP64"
        );
    }
}

#[test]
fn a_specifier_multiset_reads_the_same_type_however_it_is_spelled() {
    let header = guarded("void demo_take(long unsigned int value);");
    assert_eq!(
        check("unchecked fn demo_take(uintptr_t) -> void;", &header),
        Ok(())
    );
}

#[test]
fn signedness_and_width_never_cross() {
    let header = guarded("void demo_take(unsigned long value);");
    for spelling in ["int64_t", "intptr_t", "uint32_t", "uint16_t"] {
        assert!(
            matches!(
                check(
                    &format!("unchecked fn demo_take({spelling}) -> void;"),
                    &header
                ),
                Err(HeaderFailure::Disagreement { .. })
            ),
            "`{spelling}` is not a reading of `unsigned long`"
        );
    }
}

// ---------------------------------------------------------------------------
// Disagreement — a refusal, never a withdrawal
// ---------------------------------------------------------------------------

#[test]
fn an_arity_disagreement_refuses() {
    let header = guarded("int32_t demo_add(int32_t left, int32_t right);");
    let failure = check("unchecked fn demo_add(int32_t) -> int32_t;", &header).unwrap_err();
    assert_eq!(
        disagreement(failure),
        HeaderDisagreement::ParameterCount {
            header: 2,
            manifest: 1,
        }
    );
}

#[test]
fn a_parameter_disagreement_refuses_and_names_both_readings() {
    let header = guarded("int32_t demo_add(int32_t left, int64_t right);");
    let failure = check(
        "unchecked fn demo_add(int32_t, int32_t) -> int32_t;",
        &header,
    )
    .unwrap_err();
    assert_eq!(
        disagreement(failure),
        HeaderDisagreement::Parameter {
            parameter: 1,
            header: vec![AbiType::I64, AbiType::Isize],
            manifest: AbiType::I32,
        }
    );
}

#[test]
fn a_result_disagreement_refuses() {
    let header = guarded("void demo_add(int32_t left, int32_t right);");
    let failure = check(
        "unchecked fn demo_add(int32_t, int32_t) -> int32_t;",
        &header,
    )
    .unwrap_err();
    assert_eq!(
        disagreement(failure),
        HeaderDisagreement::Result {
            header: vec![AbiType::Unit],
            manifest: AbiType::I32,
        }
    );
}

#[test]
fn a_pointer_disagreement_in_constness_refuses() {
    let header = guarded("int32_t demo_sum(const uint8_t *buffer, uint32_t count);");
    let failure = check(
        "contract fn demo_sum(uint8_t *, uint32_t) -> int32_t from contracts/demo.qcore;",
        &header,
    )
    .unwrap_err();
    assert_eq!(
        disagreement(failure),
        HeaderDisagreement::Parameter {
            parameter: 0,
            header: vec![AbiType::PtrConstU8],
            manifest: AbiType::PtrMutU8,
        }
    );
}

#[test]
fn a_disagreement_never_withdraws_to_the_headerless_reading() {
    // The rule: withdraw only where the weaker admissible reading refuses. The
    // weaker reading here is the manifest **without** a header — which is
    // accepted, and which is exhibited beside the refusal so the claim is not a
    // statement about an empty set.
    let header = guarded("void demo_add(int32_t left, int32_t right);");
    let entries = "unchecked fn demo_add(int32_t, int32_t) -> int32_t;";
    assert!(matches!(
        check(entries, &header),
        Err(HeaderFailure::Disagreement { .. })
    ));
    let weaker = libraries(&headerless_text(entries));
    assert_eq!(check_header_bindings(&weaker, &BTreeMap::new()), Ok(()));
}

// ---------------------------------------------------------------------------
// Lexical layer, and determinism
// ---------------------------------------------------------------------------

#[test]
fn comments_and_line_splicing_do_not_change_a_reading() {
    let plain = guarded("int32_t demo_add(int32_t left, int32_t right);");
    let decorated = guarded(
        "/* a block\n   comment */ int32_t /* inner */ demo_add( // trailing\n\
         int32_t left, int\\\n32_t right);",
    );
    let entries = "unchecked fn demo_add(int32_t, int32_t) -> int32_t;";
    assert_eq!(check(entries, &plain), Ok(()));
    assert_eq!(check(entries, &decorated), Ok(()));
}

#[test]
fn a_hash_inside_a_string_literal_is_not_a_directive() {
    let header = guarded(
        "static const char demo_note[] = \"#pragma once\";\nint32_t demo_add(int32_t, int32_t);",
    );
    assert_eq!(
        check(
            "unchecked fn demo_add(int32_t, int32_t) -> int32_t;",
            &header
        ),
        Ok(())
    );
}

#[test]
fn the_reading_is_a_function_of_the_text_and_the_named_symbols() {
    let header =
        guarded("int32_t demo_add(int32_t left, int32_t right);\nvoid demo_tick(uint32_t value);");
    let symbols = BTreeSet::from(["demo_add", "demo_tick"]);
    let first = read_header(&header, &symbols).expect("the header reads");
    let second = read_header(&header, &symbols).expect("the header reads");
    assert_eq!(first, second);
    // Naming fewer symbols reads fewer rows and changes none of the readings it
    // shares — the manifest selects, and selection is not interpretation.
    let narrowed = read_header(&header, &BTreeSet::from(["demo_add"])).expect("the header reads");
    assert_eq!(narrowed.len(), 1);
    assert_eq!(narrowed["demo_add"], first["demo_add"]);
}

#[test]
fn every_named_symbol_gets_a_row_even_when_the_header_names_none_of_them() {
    let readings = read_header(
        &guarded("int32_t demo_other(void);"),
        &BTreeSet::from(["demo_add", "demo_tick"]),
    )
    .expect("the header reads");
    assert_eq!(readings.len(), 2);
    assert_eq!(readings["demo_add"], HeaderReading::Absent);
    assert_eq!(readings["demo_tick"], HeaderReading::Absent);
}

#[test]
fn a_projection_admits_exactly_its_own_spellings() {
    let projection = HeaderProjection::Spellings(BTreeSet::from([AbiType::I8, AbiType::U8]));
    assert!(projection.admits(AbiType::I8));
    assert!(projection.admits(AbiType::U8));
    assert!(!projection.admits(AbiType::I16));
    // An unspellable projection admits nothing at all, so no manifest entry can
    // agree with one by accident.
    let unspellable = HeaderProjection::Unspellable("fnptr");
    for spelling in [
        AbiType::Bool,
        AbiType::I8,
        AbiType::I16,
        AbiType::I32,
        AbiType::I64,
        AbiType::Isize,
        AbiType::U8,
        AbiType::U16,
        AbiType::U32,
        AbiType::U64,
        AbiType::Usize,
        AbiType::PtrMutU8,
        AbiType::PtrConstU8,
        AbiType::PtrMutPtrMutU8,
        AbiType::Unit,
    ] {
        assert!(!unspellable.admits(spelling));
    }
}

// ---------------------------------------------------------------------------
// The manifest grammar's two new fields
// ---------------------------------------------------------------------------

#[test]
fn the_two_header_fields_are_optional_together_and_mandatory_together() {
    let with_both = load_native_library_manifest(&manifest_text(
        "unchecked fn demo_add(int32_t, int32_t) -> int32_t;",
    ))
    .expect("both fields load");
    assert_eq!(
        with_both.header,
        Some(HeaderIdentity {
            reference: "include/demo.h".to_owned(),
            digest: HEADER_DIGEST.to_owned(),
        })
    );
    let with_neither = load_native_library_manifest(&headerless_text(
        "unchecked fn demo_add(int32_t, int32_t) -> int32_t;",
    ))
    .expect("neither field loads");
    assert_eq!(with_neither.header, None);
    for (present, missing) in [
        ("header include/demo.h;".to_owned(), "header_hash"),
        (format!("header_hash {HEADER_DIGEST};"), "header"),
    ] {
        let text = format!(
            "{}\n{present}\n",
            headerless_text("unchecked fn demo_add(int32_t, int32_t) -> int32_t;").trim_end()
        );
        assert_eq!(
            load_native_library_manifest(&text),
            Err(crate::native_library::ManifestRefusal::MissingField { field: missing })
        );
    }
}

#[test]
fn a_header_digest_outside_the_admitted_algorithm_rejects_the_manifest() {
    let text = format!(
        "{}\nheader include/demo.h;\nheader_hash md5:0123456789abcdef0123456789abcdef;\n",
        headerless_text("unchecked fn demo_add(int32_t, int32_t) -> int32_t;").trim_end()
    );
    assert_eq!(
        load_native_library_manifest(&text),
        Err(crate::native_library::ManifestRefusal::MalformedHash {
            hash: "md5:0123456789abcdef0123456789abcdef".to_owned(),
        })
    );
}
