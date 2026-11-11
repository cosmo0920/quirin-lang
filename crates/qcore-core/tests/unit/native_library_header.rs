// SPDX-License-Identifier: Apache-2.0
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

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
    sources.insert("demo.qrlib".to_owned(), text.to_owned());
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
            manifest: "demo.qrlib".to_owned(),
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
            manifest: "demo.qrlib".to_owned(),
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
                manifest: "demo.qrlib".to_owned(),
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
            manifest: "demo.qrlib".to_owned(),
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
            manifest: "demo.qrlib".to_owned(),
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
            manifest: "demo.qrlib".to_owned(),
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
            manifest: "demo.qrlib".to_owned(),
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
            manifest: "demo.qrlib".to_owned(),
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
            manifest: "demo.qrlib".to_owned(),
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
            manifest: "demo.qrlib".to_owned(),
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
            manifest: "demo.qrlib".to_owned(),
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
            manifest: "demo.qrlib".to_owned(),
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
            manifest: "demo.qrlib".to_owned(),
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
            manifest: "demo.qrlib".to_owned(),
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
            manifest: "demo.qrlib".to_owned(),
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
            manifest: "demo.qrlib".to_owned(),
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

/// Card `W48-QRLIB-LINKAGE` correction (`W48-QR-FLOAT-QRLIB-SPELLING`): this
/// row pinned `double` as read and **unspellable** (`UnspellableResult`, the
/// projection `f64`). The manifest now spells `float` and `double`, so a read
/// float is compared: a `uint64_t` transcription disagrees, and `double`
/// agrees.
#[test]
fn a_floating_result_is_read_and_now_spelled() {
    let header = guarded("double demo_now(void);");
    let failure = check("unchecked fn demo_now() -> uint64_t;", &header).unwrap_err();
    assert_eq!(
        disagreement(failure),
        HeaderDisagreement::Result {
            header: vec![AbiType::F64],
            manifest: AbiType::U64,
        }
    );
    assert_eq!(check("unchecked fn demo_now() -> double;", &header), Ok(()));
    let single = guarded("float demo_half(float value);");
    assert_eq!(
        check("unchecked fn demo_half(float) -> float;", &single),
        Ok(())
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
            // Card `NATIVE-LIBRARY-HEADER-DECORATION` adds a third, repeatable
            // part; declaring none is the wave-22 binding unchanged.
            includes: Vec::new(),
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

// ---------------------------------------------------------------------------
// Card `NATIVE-LIBRARY-HEADER-DECORATION`: declared include texts, and the
// derivation of erasable declaration decorations from what those texts say.
//
// Every row here is about the *derivation*, not about a manifest: which names
// the reader concludes are erasable from a text it was given, and which it
// refuses to conclude anything about. A manifest's only contribution is which
// hash-identified texts may be read, and the rows that pin *that* are the
// manifest-grammar ones at the end.
// ---------------------------------------------------------------------------

const DECORATION_DIGEST: &str =
    "sha256:3333333333333333333333333333333333333333333333333333333333333333";

/// A visibility text in the shape Libevent's `event2/visibility.h` has: one
/// name, five `#define`s across a nested conditional structure, and an `#else`
/// on every group.
fn visibility(bodies: [&str; 5]) -> String {
    let [sun, gnu, msvc, unknown, static_build] = bodies;
    format!(
        "#ifndef DEMO_VISIBILITY_H\n\
         #define DEMO_VISIBILITY_H\n\
         #if defined(DEMO_SHARED)\n\
         # if defined(__SUNPRO_C)\n\
         #  define DEMO_EXPORT {sun}\n\
         # elif defined(__GNUC__)\n\
         #  define DEMO_EXPORT {gnu}\n\
         # elif defined(_MSC_VER)\n\
         #  define DEMO_EXPORT {msvc}\n\
         # else\n\
         #  define DEMO_EXPORT {unknown}\n\
         # endif\n\
         #else\n\
         # define DEMO_EXPORT {static_build}\n\
         #endif\n\
         #endif /* DEMO_VISIBILITY_H */\n"
    )
}

/// The real shape: `__global`, a visibility attribute, a `__declspec`, and two
/// empty bodies.
fn real_visibility() -> String {
    visibility([
        "__global",
        "__attribute__((visibility(\"default\")))",
        "__declspec(dllexport)",
        "",
        "",
    ])
}

/// A decorated header declaring `demo_alloc`, including the text named.
fn decorated(include: &str) -> String {
    guarded(&format!(
        "#include {include}\n\
         DEMO_EXPORT\n\
         uint8_t *demo_alloc(uintptr_t size);\n"
    ))
}

fn read_decorated(visibility_text: &str) -> HeaderReadout {
    let included = BTreeMap::from([("\"demo_visibility.h\"".to_owned(), visibility_text)]);
    read_header_with_includes(
        &decorated("\"demo_visibility.h\""),
        &included,
        &BTreeSet::from(["demo_alloc"]),
    )
    .expect("the texts are inside the directive subset")
}

#[test]
fn the_decoration_is_derived_from_the_declared_texts_own_defines() {
    let readout = read_decorated(&real_visibility());
    assert_eq!(
        readout.erasures.iter().cloned().collect::<Vec<_>>(),
        ["DEMO_EXPORT"],
        "one name, read out of the text rather than declared beside it"
    );
    assert_eq!(
        readout.followed,
        BTreeSet::from(["\"demo_visibility.h\"".to_owned()])
    );
    let HeaderReading::Declared(declaration) = &readout.readings["demo_alloc"] else {
        panic!("the decorated declaration is read: {:?}", readout.readings);
    };
    assert_eq!(
        declaration.result,
        Ok(HeaderProjection::Spellings(BTreeSet::from([
            AbiType::PtrMutU8
        ])))
    );
}

#[test]
fn without_the_declared_text_the_decoration_refuses_by_name() {
    // The wave-22 reading of the same header: the `#include` is not declared,
    // so it is not followed and the token is a type this reader does not commit
    // to. This is the row every other row in this group is a departure from.
    let readings = read_header(
        &decorated("\"demo_visibility.h\""),
        &BTreeSet::from(["demo_alloc"]),
    )
    .expect("the header itself is inside the subset");
    let HeaderReading::Declared(declaration) = &readings["demo_alloc"] else {
        panic!("the declaration is found");
    };
    let Err(spelling) = &declaration.result else {
        panic!("an undeclared decoration must refuse: {declaration:?}");
    };
    assert!(spelling.contains("DEMO_EXPORT"), "{spelling}");
}

#[test]
fn erasing_the_derived_name_equals_expanding_it() {
    // The justification of the whole derivation, as an equality of readings:
    // each of the bodies the text contains, written where the macro stood, reads
    // exactly as the macro form does. One vocabulary erases both, which is what
    // makes the claim checkable rather than asserted.
    let through_the_macro = read_decorated(&real_visibility()).readings;
    for expansion in [
        "",
        "__global",
        "__attribute__((visibility(\"default\")))",
        "__declspec(dllexport)",
    ] {
        let expanded = guarded(&format!(
            "{expansion}\nuint8_t *demo_alloc(uintptr_t size);\n"
        ));
        let readings = read_header(&expanded, &BTreeSet::from(["demo_alloc"]))
            .unwrap_or_else(|refusal| panic!("`{expansion}`: {refusal}"));
        assert_eq!(readings, through_the_macro, "expansion `{expansion}`");
    }
}

#[test]
fn every_body_must_be_erasable_for_the_name_to_be() {
    // The reader decides no condition, so it cannot know which arm is taken.
    // Each row below poisons exactly one arm with a decoration that changes the
    // declared type, the calling convention or the exported symbol name.
    for poison in [
        "__attribute__((mode(__DI__)))",
        "__attribute__((aligned(16)))",
        "__attribute__((vector_size(16)))",
        "__attribute__((ms_abi))",
        "__attribute__((alias(\"other\")))",
        "__declspec(align(16))",
        "int",
    ] {
        for arm in 0..5usize {
            let mut bodies = ["", "", "", "", ""];
            bodies[arm] = poison;
            assert!(
                read_decorated(&visibility(bodies)).erasures.is_empty(),
                "`{poison}` in arm {arm} must withdraw the derivation"
            );
        }
    }
}

#[test]
fn the_erasable_attribute_vocabulary_is_closed_and_pinned() {
    // The positive direction of the row above: exactly these names are erased,
    // and each of them changes neither the declared type, the parameter vector
    // nor the linkage name.
    for attribute in [
        "visibility(\"default\")",
        "deprecated",
        "unused",
        "used",
        "warn_unused_result",
        "malloc",
        "nonnull",
        "format(printf, 1, 2)",
        "always_inline",
        "noinline",
        "noreturn",
        "nothrow",
        "hot",
        "cold",
    ] {
        let body = format!("__attribute__(({attribute}))");
        assert_eq!(
            read_decorated(&visibility(["", &body, "", "", ""]))
                .erasures
                .len(),
            1,
            "`{attribute}` is in the vocabulary"
        );
    }
}

#[test]
fn a_group_with_no_else_leaves_a_path_undefined() {
    // The all-paths condition. The text below defines the name in every arm it
    // writes; what it does not have is an arm for the case where no condition
    // holds, and the reader will not supply one from a configuration it cannot
    // see.
    let text = "#ifndef DEMO_VISIBILITY_H\n\
                #define DEMO_VISIBILITY_H\n\
                #if defined(DEMO_SHARED)\n\
                # define DEMO_EXPORT\n\
                #elif defined(DEMO_STATIC)\n\
                # define DEMO_EXPORT\n\
                #endif\n\
                #endif\n";
    assert!(read_decorated(text).erasures.is_empty());
}

#[test]
fn a_definition_that_cannot_reach_the_use_is_not_derived() {
    // Definedness is a question about the text *preceding* the first use. A
    // definition after it reaches nothing.
    let header = guarded(
        "DEMO_EXPORT\n\
         uint8_t *demo_alloc(uintptr_t size);\n\
         #define DEMO_EXPORT\n",
    );
    let readout = read_header_with_includes(&header, &BTreeMap::new(), &BTreeSet::new())
        .expect("the header is inside the subset");
    assert!(readout.erasures.is_empty());
}

#[test]
fn a_function_like_or_undefined_decoration_is_not_derived() {
    for text in [
        // A function-like macro's expansion depends on its arguments at each
        // use site, and this reader evaluates no expansion.
        "#ifndef V_H\n#define V_H\n#define DEMO_EXPORT(x) x\n#endif\n",
        // `#undef` anywhere withdraws it: this reader tracks no scope for a
        // macro, and the conservative reading is the refusing one.
        "#ifndef V_H\n#define V_H\n#define DEMO_EXPORT\n#undef DEMO_EXPORT\n#endif\n",
    ] {
        assert!(read_decorated(text).erasures.is_empty(), "{text}");
    }
}

#[test]
fn a_chained_definition_stands_on_a_name_already_derived() {
    // `#define A B` is erasable when `B` is, which is how a real library's
    // second export macro is written. The least fixed point is what reads it.
    let text = "#ifndef V_H\n\
                #define V_H\n\
                #define DEMO_BASE __attribute__((visibility(\"default\")))\n\
                #define DEMO_EXPORT DEMO_BASE\n\
                #endif\n";
    assert_eq!(
        read_decorated(text).erasures,
        BTreeSet::from(["DEMO_BASE".to_owned(), "DEMO_EXPORT".to_owned()])
    );
}

#[test]
fn a_declared_text_is_spliced_at_most_once() {
    // Which is what an include guard does, and what keeps a cycle finite. The
    // text below includes itself.
    let text = "#ifndef V_H\n\
                #define V_H\n\
                #include \"demo_visibility.h\"\n\
                #define DEMO_EXPORT\n\
                #endif\n";
    assert_eq!(
        read_decorated(text).erasures,
        BTreeSet::from(["DEMO_EXPORT".to_owned()])
    );
}

#[test]
fn a_spliced_texts_typedefs_are_read_as_the_headers_own() {
    // Splicing is not a macro table: the text becomes part of the one text the
    // reader reads, so a `typedef` one file away stops being unreadable.
    let text = "#ifndef V_H\n#define V_H\ntypedef uint32_t demo_size_t;\n#endif\n";
    let included = BTreeMap::from([("\"demo_visibility.h\"".to_owned(), text)]);
    let header = guarded(
        "#include \"demo_visibility.h\"\n\
         int32_t demo_sum(demo_size_t count);\n",
    );
    let readout = read_header_with_includes(&header, &included, &BTreeSet::from(["demo_sum"]))
        .expect("the texts are inside the subset");
    let HeaderReading::Declared(declaration) = &readout.readings["demo_sum"] else {
        panic!("the declaration is read: {:?}", readout.readings);
    };
    assert_eq!(
        declaration.parameters,
        [Ok(HeaderProjection::Spellings(BTreeSet::from([
            AbiType::U32
        ])))]
    );
}

#[test]
fn a_spliced_text_is_read_under_the_same_directive_subset() {
    // A spliced text is part of the one text being read, so its own directives
    // are read under the same rules and its `#error` refuses the whole reading
    // rather than half of it.
    let included = BTreeMap::from([(
        "\"demo_visibility.h\"".to_owned(),
        "#ifndef V_H\n#define V_H\n#error unsupported\n#endif\n",
    )]);
    let refusal = read_header_with_includes(
        &decorated("\"demo_visibility.h\""),
        &included,
        &BTreeSet::new(),
    )
    .expect_err("a spliced `#error` is outside the subset");
    assert_eq!(
        refusal,
        HeaderTextRefusal::UnsupportedDirective {
            directive: "error".to_owned()
        }
    );
}

#[test]
fn an_undeclared_include_is_not_followed_and_nothing_about_that_changed() {
    // The set is closed in both directions, and the undeclared direction is
    // *quiet*: refusing on every undeclared `#include` would make the level
    // unusable on any header that includes `<stdint.h>`.
    let readout = read_header_with_includes(
        &guarded("#include <stdint.h>\nuint8_t *demo_alloc(uintptr_t size);\n"),
        &BTreeMap::new(),
        &BTreeSet::from(["demo_alloc"]),
    )
    .expect("the header is inside the subset");
    assert!(readout.followed.is_empty());
    assert!(matches!(
        readout.readings["demo_alloc"],
        HeaderReading::Declared(_)
    ));
}

#[test]
fn a_prefix_decoration_outside_the_vocabulary_no_longer_passes_unread() {
    // A finding of this card, repaired in it. The specifier prefix is read by
    // walking backwards from the declarator name, and a decoration ending in
    // `)` stopped that walk *silently* — so `__attribute__((mode(__DI__)))`,
    // which changes the declared type, was ignored rather than refused. The
    // erasable vocabulary is deleted from the stream before this point, so
    // anything still standing here is a decoration this reader does not commit
    // to.
    let header = guarded("__attribute__((mode(__DI__))) uint8_t *demo_alloc(uintptr_t size);");
    let readings =
        read_header(&header, &BTreeSet::from(["demo_alloc"])).expect("the file is in the subset");
    let HeaderReading::Declared(declaration) = &readings["demo_alloc"] else {
        panic!("the declaration is found");
    };
    let Err(spelling) = &declaration.result else {
        panic!("a type-changing prefix decoration must refuse: {declaration:?}");
    };
    assert!(spelling.contains("mode"), "{spelling}");
}

// --- The manifest's side: which texts, and nothing about what they say ---

const DECORATED_ENTRY: &str =
    "contract fn demo_alloc(uintptr_t) -> uint8_t * from contracts/demo.qcore;";

/// A manifest naming one exported header and one declared include text.
fn decorated_manifest() -> String {
    format!(
        "{}\ninclude \"demo_visibility.h\" from include/demo_visibility.h {DECORATION_DIGEST};\n{DECORATED_ENTRY}\n",
        manifest_text("").trim_end()
    )
}

fn check_decorated(
    manifest: &str,
    header: &str,
    include_text: &str,
    include_digest: &str,
) -> Result<(), HeaderFailure> {
    let libraries = libraries(manifest);
    let sources = BTreeMap::from([
        (
            "include/demo.h".to_owned(),
            HeaderSource {
                reference: "include/demo.h",
                text: header,
                measured_digest: HEADER_DIGEST,
            },
        ),
        (
            "include/demo_visibility.h".to_owned(),
            HeaderSource {
                reference: "include/demo_visibility.h",
                text: include_text,
                measured_digest: include_digest,
            },
        ),
    ]);
    check_header_bindings(&libraries, &sources)
}

#[test]
fn a_manifest_declaring_the_visibility_text_reads_the_decorated_header() {
    assert_eq!(
        check_decorated(
            &decorated_manifest(),
            &decorated("\"demo_visibility.h\""),
            &real_visibility(),
            DECORATION_DIGEST,
        ),
        Ok(())
    );
}

#[test]
fn the_include_identity_is_published_with_the_header_identity() {
    let manifest = load_native_library_manifest(&decorated_manifest()).expect("the manifest loads");
    assert_eq!(
        manifest.header.expect("a header is bound").includes,
        vec![IncludeIdentity {
            spelling: "\"demo_visibility.h\"".to_owned(),
            reference: "include/demo_visibility.h".to_owned(),
            digest: DECORATION_DIGEST.to_owned(),
        }],
        "the texts a decision rests on are named by the record a certificate carries"
    );
}

#[test]
fn a_declared_include_texts_digest_decides_before_it_is_read() {
    // Exactly as the header's own digest does, and for the same reason: a
    // decision that rests on a text must name that text by content or it cannot
    // be rederived.
    assert_eq!(
        refusal(
            check_decorated(
                &decorated_manifest(),
                &decorated("\"demo_visibility.h\""),
                &real_visibility(),
                OTHER_DIGEST,
            )
            .expect_err("a substituted include text refuses"),
        ),
        HeaderRefusal::IncludeDigestMismatch {
            manifest: "demo.qrlib".to_owned(),
            spelling: "\"demo_visibility.h\"".to_owned(),
            reference: "include/demo_visibility.h".to_owned(),
            declared: DECORATION_DIGEST.to_owned(),
            measured: OTHER_DIGEST.to_owned(),
        }
    );
}

#[test]
fn a_declared_include_text_that_is_not_supplied_refuses_by_name() {
    let libraries = libraries(&decorated_manifest());
    let header = decorated("\"demo_visibility.h\"");
    let sources = BTreeMap::from([(
        "include/demo.h".to_owned(),
        HeaderSource {
            reference: "include/demo.h",
            text: &header,
            measured_digest: HEADER_DIGEST,
        },
    )]);
    assert_eq!(
        refusal(check_header_bindings(&libraries, &sources).expect_err("the text is missing")),
        HeaderRefusal::IncludeNotSupplied {
            manifest: "demo.qrlib".to_owned(),
            spelling: "\"demo_visibility.h\"".to_owned(),
            reference: "include/demo_visibility.h".to_owned(),
        }
    );
}

#[test]
fn a_declared_include_text_nothing_reaches_refuses_by_name() {
    // A declared text nothing reads is a statement with no effect, and this
    // level's content is that its statements have effects — the same totality
    // argument the contract slice's unresolved binding rests on.
    assert_eq!(
        refusal(
            check_decorated(
                &decorated_manifest(),
                // The header includes something else entirely.
                &decorated("<stdint.h>"),
                &real_visibility(),
                DECORATION_DIGEST,
            )
            .expect_err("an unreached declared text refuses"),
        ),
        HeaderRefusal::IncludeNotReached {
            manifest: "demo.qrlib".to_owned(),
            spelling: "\"demo_visibility.h\"".to_owned(),
        }
    );
}

#[test]
fn a_text_supplied_for_a_reference_nobody_declares_refuses_by_name() {
    let libraries = libraries(&decorated_manifest());
    let header = decorated("\"demo_visibility.h\"");
    let visibility_text = real_visibility();
    let sources = BTreeMap::from([
        (
            "include/demo.h".to_owned(),
            HeaderSource {
                reference: "include/demo.h",
                text: &header,
                measured_digest: HEADER_DIGEST,
            },
        ),
        (
            "include/demo_visibility.h".to_owned(),
            HeaderSource {
                reference: "include/demo_visibility.h",
                text: &visibility_text,
                measured_digest: DECORATION_DIGEST,
            },
        ),
        (
            "include/undeclared.h".to_owned(),
            HeaderSource {
                reference: "include/undeclared.h",
                text: "",
                measured_digest: OTHER_DIGEST,
            },
        ),
    ]);
    assert_eq!(
        refusal(check_header_bindings(&libraries, &sources).expect_err("an extra text refuses")),
        HeaderRefusal::SourceNotDeclared {
            reference: "include/undeclared.h".to_owned(),
        }
    );
}

#[test]
fn a_manifest_naming_no_header_is_still_unaffected_by_supplied_texts() {
    // The transport totality above is gated on a header being bound, so the
    // wave-22 row it could have broken is re-asserted here rather than assumed:
    // a selection naming no header is byte-identical to what that card
    // published, whatever texts are supplied.
    let text = headerless_text(DECORATED_ENTRY);
    let libraries = libraries(&text);
    let sources = BTreeMap::from([(
        "include/anything.h".to_owned(),
        HeaderSource {
            reference: "include/anything.h",
            text: "int demo_alloc(void);",
            measured_digest: OTHER_DIGEST,
        },
    )]);
    assert_eq!(check_header_bindings(&libraries, &sources), Ok(()));
}

#[test]
fn the_include_statements_grammar_refuses_by_name() {
    use crate::native_library::ManifestRefusal;

    assert!(load_native_library_manifest(&decorated_manifest()).is_ok());
    for (statement, expected) in [
        (
            "include \"demo_visibility.h\" include/demo_visibility.h;".to_owned(),
            ManifestRefusal::MalformedIncludeBinding {
                statement: "include \"demo_visibility.h\" include/demo_visibility.h".to_owned(),
            },
        ),
        (
            format!(
                "include demo_visibility.h from include/demo_visibility.h {DECORATION_DIGEST};"
            ),
            ManifestRefusal::MalformedIncludeSpelling {
                spelling: "demo_visibility.h".to_owned(),
            },
        ),
        (
            "include \"demo_visibility.h\" from include/demo_visibility.h md5:00;".to_owned(),
            ManifestRefusal::MalformedHash {
                hash: "md5:00".to_owned(),
            },
        ),
    ] {
        let text = format!(
            "{}\n{statement}\n{DECORATED_ENTRY}\n",
            manifest_text("").trim_end()
        );
        assert_eq!(load_native_library_manifest(&text), Err(expected));
    }
    // The same spelling twice has no defined meaning.
    let repeated = format!(
        "{}\ninclude \"demo_visibility.h\" from include/a.h {DECORATION_DIGEST};\n\
         include \"demo_visibility.h\" from include/b.h {DECORATION_DIGEST};\n{DECORATED_ENTRY}\n",
        manifest_text("").trim_end()
    );
    assert_eq!(
        load_native_library_manifest(&repeated),
        Err(ManifestRefusal::IncludeBoundTwice {
            spelling: "\"demo_visibility.h\"".to_owned(),
        })
    );
    // And a declared text with no header to read it into names a text nothing
    // opens.
    let orphan = format!(
        "{}\ninclude \"demo_visibility.h\" from include/demo_visibility.h {DECORATION_DIGEST};\n{DECORATED_ENTRY}\n",
        headerless_text("").trim_end()
    );
    assert_eq!(
        load_native_library_manifest(&orphan),
        Err(ManifestRefusal::IncludeWithoutHeader {
            spelling: "\"demo_visibility.h\"".to_owned(),
        })
    );
}
