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

//! Complete main-source definition enumeration (unit **E01-F1**,
//! `docs/reports/remaining-e01-f1/README.md`).
//!
//! `docs/reports/remaining-e01/README.md` boundary **E01-B1** recorded that a
//! definition produced by macro expansion, and a K&R definition, were absent
//! from the audited inventory entirely — no obligation *and no residual
//! `Unknown` row*, so the reported "N checked / M Unknown" split was not a
//! complete function denominator.
//!
//! These tests are the repair's acceptance:
//!
//! * both shapes are now **counted**, object-like and function-like macros
//!   included;
//! * a K&R definition is counted and carries an explicit unanalyzable row with
//!   a classified reason code — the analyzer is *not* extended to read it;
//! * a macro-expanded definition is counted and, because it is an ordinary
//!   definition after preprocessing, the **unmodified** analyzer reads it;
//! * adversarial near-definitions — prototypes, old-style declarations,
//!   function-pointer declarators, macro invocations that are not definitions,
//!   and header definitions — are **not** counted;
//! * a token stream whose definition boundaries cannot be established refuses
//!   the unit's completeness claim with a named ground rather than guessing;
//! * the enumeration is monotone: every historical raw-scan name survives.

use qcore_c_gateway::legacy_c::{
    AnalysisStatus, C11TranslationUnit, EnumerationRefusal, LegacyFunctionScope, ResourceContracts,
    UnanalyzableForm, UnrestrictedLegacyResult, analyze_unrestricted_c11,
    enumerate_source_definitions, legacy_c_source_functions,
};
use qcore_schemas::reason::{ReasonCategory, ReasonSubject};

fn clang_available() -> bool {
    match std::env::var("QCORE_CLANG") {
        Ok(path) => !path.trim().is_empty(),
        Err(_) => std::process::Command::new("clang")
            .arg("--version")
            .output()
            .is_ok_and(|output| output.status.success()),
    }
}

fn unit(source: &str) -> C11TranslationUnit<'_> {
    C11TranslationUnit {
        source,
        function_scope: LegacyFunctionScope::MainSourceDefinitions,
        ..C11TranslationUnit::default()
    }
}

// ---------------------------------------------------------------------------
// 1. Macro-expanded definitions are counted
// ---------------------------------------------------------------------------

#[test]
fn function_like_macro_definition_is_enumerated() {
    if !clang_available() {
        return;
    }
    let source =
        "#define DEFN(n) int n(int v) { return v; }\nDEFN(run)\nint plain(void) { return 0; }\n";
    // The historical raw scan sees only `plain`: this is exactly E01-B1.
    assert_eq!(legacy_c_source_functions(source), ["plain"]);
    let inventory = enumerate_source_definitions(&unit(source));
    assert_eq!(inventory.definitions(), ["plain", "run"]);
    assert_eq!(inventory.absent_from_raw_scan(), ["run"]);
    assert!(inventory.unanalyzable().is_empty());
    assert!(inventory.is_complete(), "{:?}", inventory.refusals());
    assert!(inventory.supports_unit_safe());
}

#[test]
fn object_like_macro_definition_is_enumerated() {
    if !clang_available() {
        return;
    }
    let source = "#define BODY int hidden_by_macro(void) { return 7; }\nBODY\n";
    assert!(legacy_c_source_functions(source).is_empty());
    let inventory = enumerate_source_definitions(&unit(source));
    assert_eq!(inventory.definitions(), ["hidden_by_macro"]);
    assert_eq!(inventory.absent_from_raw_scan(), ["hidden_by_macro"]);
    assert!(inventory.is_complete(), "{:?}", inventory.refusals());
}

/// The card's own rule: the denominator grows, the analyzer does not. A
/// macro-expanded definition is an ordinary definition *after* preprocessing,
/// so the unmodified analyzer reads it — and its evidence becomes visible.
#[test]
fn a_definition_the_raw_scan_missed_is_analysed_by_the_unmodified_analyzer() {
    if !clang_available() {
        return;
    }
    let source = "#define DEFN(n) int n(int v) { return v; }\nDEFN(run)\n";
    let unit = unit(source);
    match analyze_unrestricted_c11(&unit, "run", &ResourceContracts::default()) {
        Ok(UnrestrictedLegacyResult::Safe(certificate)) => {
            assert_eq!(certificate.checked_functions, ["run"]);
        }
        other => panic!("expected the expanded definition to be analysed: {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// 2. K&R definitions are counted and carry an honest unanalyzable row
// ---------------------------------------------------------------------------

#[test]
fn knr_definition_with_declarations_is_enumerated_but_not_analysed() {
    if !clang_available() {
        return;
    }
    let source =
        "int knr(a, b)\nint a;\nint b;\n{ return a + b; }\nint plain(void) { return 0; }\n";
    assert_eq!(legacy_c_source_functions(source), ["plain"]);
    let inventory = enumerate_source_definitions(&unit(source));
    assert_eq!(inventory.definitions(), ["knr", "plain"]);
    assert!(inventory.absent_from_raw_scan().is_empty());
    let unanalyzable = inventory.unanalyzable();
    assert_eq!(unanalyzable.len(), 1);
    assert_eq!(unanalyzable[0].function(), "knr");
    assert_eq!(
        unanalyzable[0].form(),
        UnanalyzableForm::IdentifierListParameters
    );
    assert_eq!(
        unanalyzable[0].form().category(),
        ReasonCategory::UnsupportedSyntax
    );
    // The row classifies through the closed §L21.1 mapping, not by defaulting.
    assert_eq!(
        ReasonCategory::classify(unanalyzable[0].form().reason()),
        Ok(ReasonCategory::UnsupportedSyntax)
    );
    assert!(inventory.is_complete(), "{:?}", inventory.refusals());
    // Enumerated, unanalysed: the unit cannot claim `Safe` over this scope.
    assert!(!inventory.supports_unit_safe());
    assert!(
        inventory
            .withheld_safe_reason()
            .is_some_and(|reason| reason.contains("knr"))
    );

    // The analyzer is unchanged: it simply never finds a K&R body, so the name
    // stays out of the projection instead of acquiring an unearned status.
    //
    // **Deliberate pin update, unit E01-TAIL, residual E01-F1-DENOM.** This
    // arm previously asserted the *gateway* published `Safe` over this unit,
    // with the whole-unit claim withheld only later, in the audit's projection
    // layer. That left every other caller of `analyze_unrestricted_c11` — and
    // the certificate itself — holding a `Safe` over a definition nothing read.
    // The gateway now refuses it on its own ground. Both halves this arm was
    // protecting are kept and read out of the retained partial summary: `knr`
    // still acquires no status at all, and `plain` is still analysed and clean.
    match analyze_unrestricted_c11(&unit(source), "plain", &ResourceContracts::default()) {
        Ok(UnrestrictedLegacyResult::Unknown {
            reason,
            partial: Some(summary),
            ..
        }) => {
            assert_eq!(summary.functions.keys().collect::<Vec<_>>(), vec!["plain"]);
            assert_eq!(summary.status, AnalysisStatus::Safe);
            assert!(
                !summary.functions.contains_key("knr"),
                "the K&R name must acquire no status"
            );
            assert!(reason.contains("`knr`"), "{reason}");
        }
        other => panic!("unexpected analysis outcome: {other:?}"),
    }
}

/// A K&R definition whose parameters need no declarations at all (they default
/// to `int`) still has an identifier list, but no declaration list — so the
/// body follows `)` directly and it is an ordinary prototype-shaped definition
/// to every scanner here. It must still be counted, and it is: by the
/// historical rule, unchanged.
#[test]
fn knr_definition_without_declarations_keeps_its_historical_row() {
    if !clang_available() {
        return;
    }
    let source = "int implicit(a, b) { return a + b; }\n";
    assert_eq!(legacy_c_source_functions(source), ["implicit"]);
    let inventory = enumerate_source_definitions(&unit(source));
    assert_eq!(inventory.definitions(), ["implicit"]);
    assert!(inventory.unanalyzable().is_empty());
    assert!(inventory.is_complete(), "{:?}", inventory.refusals());
}

#[test]
fn a_macro_expanded_knr_definition_is_counted_as_unanalyzable() {
    if !clang_available() {
        return;
    }
    let source = "#define OLD(n) int n(a) int a; { return a; }\nOLD(both)\n";
    assert!(legacy_c_source_functions(source).is_empty());
    let inventory = enumerate_source_definitions(&unit(source));
    assert_eq!(inventory.definitions(), ["both"]);
    assert_eq!(inventory.unanalyzable().len(), 1);
    assert_eq!(inventory.unanalyzable()[0].function(), "both");
    // It is not reported as a macro-expanded *analysable* definition: nothing
    // reads it, so it never enters the analyzer's selection as one.
    assert!(inventory.absent_from_raw_scan().is_empty());
}

// ---------------------------------------------------------------------------
// 3. Adversarial near-definitions are NOT counted
// ---------------------------------------------------------------------------

#[test]
fn near_definitions_are_not_counted() {
    if !clang_available() {
        return;
    }
    let source = "\
int declared_only(void);\n\
int old_style_declaration(a, b);\n\
extern int external_thing(int, char *);\n\
int (*file_scope_pointer)(int);\n\
struct pair { int a; int b; };\n\
static struct pair initialised = { 1, 2 };\n\
typedef int (*callback)(void *);\n\
int subject(void) { return declared_only(); }\n";
    let inventory = enumerate_source_definitions(&unit(source));
    assert_eq!(
        inventory.definitions(),
        ["subject"],
        "only the one real definition may be counted"
    );
    assert!(inventory.unanalyzable().is_empty());
    assert!(inventory.is_complete(), "{:?}", inventory.refusals());
}

/// A file-scope macro invocation that declares rather than defines must not be
/// counted, and — because the expansion is read — must not refuse completeness
/// either.
#[test]
fn a_macro_invocation_that_is_not_a_definition_is_not_counted() {
    if !clang_available() {
        return;
    }
    let source = "\
#define DECLARE(n) int n(void);\n\
#define ENTRY(a, b) { a, b }\n\
struct pair { int x; int y; };\n\
DECLARE(later)\n\
static struct pair table[] = { ENTRY(1, 2), ENTRY(3, 4) };\n\
int subject(void) { return 0; }\n";
    let inventory = enumerate_source_definitions(&unit(source));
    assert_eq!(inventory.definitions(), ["subject"]);
    assert!(inventory.absent_from_raw_scan().is_empty());
    assert!(inventory.is_complete(), "{:?}", inventory.refusals());
}

/// Boundary **E01-B2** is unchanged: a header definition, including one a
/// header's own macro produces, stays outside the audited main-source scope.
#[test]
fn header_definitions_stay_out_of_scope() {
    if !clang_available() {
        return;
    }
    let directory = std::env::temp_dir().join(format!(
        "qcore-e01f1-{}-{:?}",
        std::process::id(),
        std::thread::current().id()
    ));
    std::fs::create_dir_all(&directory).unwrap();
    let header = directory.join("e01f1.h");
    std::fs::write(
        &header,
        "static int header_defined(void) { return 1; }\n#define HDR(n) int n(void) { return 2; }\nHDR(header_macro_defined)\n",
    )
    .unwrap();
    let source = "#include \"e01f1.h\"\nint subject(void) { return 0; }\n";
    let with_header = C11TranslationUnit {
        source,
        include_paths: vec![directory.to_string_lossy().into_owned()],
        function_scope: LegacyFunctionScope::MainSourceDefinitions,
        ..C11TranslationUnit::default()
    };
    let inventory = enumerate_source_definitions(&with_header);
    assert_eq!(inventory.definitions(), ["subject"]);
    assert!(inventory.is_complete(), "{:?}", inventory.refusals());
    std::fs::remove_dir_all(&directory).unwrap();
}

/// A macro *defined* in a header but *invoked* in the main source produces a
/// main-source definition, and is counted: the invocation site decides scope.
#[test]
fn a_header_macro_invoked_in_the_main_source_is_counted() {
    if !clang_available() {
        return;
    }
    let directory = std::env::temp_dir().join(format!(
        "qcore-e01f1-invoke-{}-{:?}",
        std::process::id(),
        std::thread::current().id()
    ));
    std::fs::create_dir_all(&directory).unwrap();
    std::fs::write(
        directory.join("e01f1_invoke.h"),
        "#define HDR(n) int n(void) { return 2; }\n",
    )
    .unwrap();
    let source = "#include \"e01f1_invoke.h\"\nHDR(from_main)\nint subject(void) { return 0; }\n";
    let with_header = C11TranslationUnit {
        source,
        include_paths: vec![directory.to_string_lossy().into_owned()],
        function_scope: LegacyFunctionScope::MainSourceDefinitions,
        ..C11TranslationUnit::default()
    };
    let inventory = enumerate_source_definitions(&with_header);
    assert_eq!(inventory.definitions(), ["from_main", "subject"]);
    assert_eq!(inventory.absent_from_raw_scan(), ["from_main"]);
    std::fs::remove_dir_all(&directory).unwrap();
}

/// The measured third cause, reduced from `librdkafka/src/rdkafka.c`
/// (`docs/reports/remaining-e01-f1/README.md` §2.2): a `#` stringification
/// operator on a macro's *continuation* line made the raw tokenizer discard the
/// rest of that line, closing brace included, so the scan's file-scope brace
/// depth never returned to zero and every later definition in the file was
/// invisible to it.
///
/// **Deliberate pin update, unit E01-F1-RAW**
/// (`docs/reports/remaining-e01-f1-raw/README.md`). This test asserted that the
/// raw scan *loses* both definitions and that reading the expansion recovers
/// them. The tokenizer defect is now repaired at its source, so the raw scan
/// finds them itself and `absent_from_raw_scan` is empty here.
///
/// The load-bearing half is kept and strengthened: the enumerated denominator
/// is still exactly the two definitions after the table, and the two mechanisms
/// — raw scan and expansion reader — now **agree** on this shape instead of
/// disagreeing by two rows. `crates/qcore-c-gateway/tests/e01_f1_raw_tokenizer.rs`
/// holds the lexical matrix that keeps the raw half honest.
#[test]
fn a_raw_scan_brace_desynchronisation_no_longer_hides_definitions() {
    if !clang_available() {
        return;
    }
    let source = "\
struct entry { int code; const char *name; };\n\
#define ENTRY(E, D)                                                           \\\n\
        [E] = {E, &(#E)[2], D}\n\
static const struct entry table[] = { ENTRY(1, \"one\"), ENTRY(2, \"two\") };\n\
int after_the_table(void) { return table[0].code; }\n\
int also_after(void) { return 0; }\n";
    assert_eq!(
        legacy_c_source_functions(source),
        ["after_the_table", "also_after"],
        "E01-F1-RAW repaired the scan that used to lose these"
    );
    let inventory = enumerate_source_definitions(&unit(source));
    assert_eq!(inventory.definitions(), ["after_the_table", "also_after"]);
    assert!(
        inventory.absent_from_raw_scan().is_empty(),
        "the two mechanisms now agree on this shape: {:?}",
        inventory.absent_from_raw_scan()
    );
    assert!(inventory.is_complete(), "{:?}", inventory.refusals());
}

// ---------------------------------------------------------------------------
// 4. Enumeration ambiguity refuses the completeness claim
// ---------------------------------------------------------------------------

/// A K&R declarator list this scanner cannot read — a function-pointer
/// parameter declaration — is not silently dropped and not silently counted:
/// the unit's completeness claim is refused with a named ground.
#[test]
fn an_unreadable_declarator_list_refuses_completeness() {
    if !clang_available() {
        return;
    }
    let source = "int hook(g)\nint (*g)(void);\n{ return g(); }\nint subject(void) { return 0; }\n";
    let inventory = enumerate_source_definitions(&unit(source));
    assert_eq!(
        inventory.refusals(),
        [EnumerationRefusal::UnclassifiedFileScopeConstruct]
    );
    assert!(!inventory.is_complete());
    assert!(!inventory.supports_unit_safe());
    // The definitions it *did* establish are still reported.
    assert_eq!(inventory.definitions(), ["subject"]);
}

/// A file-scope initialiser table whose entries look exactly like K&R
/// declarators — `sizeof(a), sizeof(b)` has the same token shape as
/// `f(a, b) int a; { … }` — must be read as the expression it is. Refusing
/// completeness here would refuse it across ordinary C.
#[test]
fn an_expression_context_is_neither_counted_nor_ambiguous() {
    if !clang_available() {
        return;
    }
    let source = "\
struct pair { int x; int y; };\n\
static const unsigned long sizes[] = { sizeof(struct pair), sizeof(int) };\n\
enum { FIRST = 1, SECOND = 2 };\n\
static const int chosen[] = { FIRST, SECOND };\n\
int subject(void) { return (int) sizes[0] + chosen[0]; }\n";
    let inventory = enumerate_source_definitions(&unit(source));
    assert_eq!(inventory.definitions(), ["subject"]);
    assert!(
        inventory.is_complete(),
        "an expression context must not refuse completeness: {:?}",
        inventory.refusals()
    );
}

/// Two idioms measured on the pinned corpus that put a *parenthesised name* in
/// front of another declarator: a function typedef (`librdkafka`'s
/// `rdkafka_admin.c`) and a function-pointer parameter (`rdkafka_partition.c`).
/// Both have the K&R token shape in a declaration context; neither is a
/// definition, and a K&R declaration list can never begin with `(` or `[`.
#[test]
fn a_parenthesised_declarator_is_neither_counted_nor_ambiguous() {
    if !clang_available() {
        return;
    }
    let source = "\
typedef int (compare_cb_t)(const void *a, const void *b);\n\
int subject(int (cmp)(const void *a, const void *b), unsigned int (hash)(const void *a)) {\n\
    return cmp(0, 0) + (int) hash(0);\n\
}\n";
    let inventory = enumerate_source_definitions(&unit(source));
    assert_eq!(inventory.definitions(), ["subject"]);
    assert!(
        inventory.is_complete(),
        "a parenthesised declarator must not refuse completeness: {:?}",
        inventory.refusals()
    );
}

/// The guard is not a blanket exemption: a real declarator this scanner cannot
/// read still refuses, because its name sits in a declaration context.
#[test]
fn the_expression_guard_does_not_hide_a_real_unreadable_declarator() {
    if !clang_available() {
        return;
    }
    let source = "int hook(g)\nint (*g)(void);\n{ return g(); }\nint subject(void) { return 0; }\n";
    let inventory = enumerate_source_definitions(&unit(source));
    assert_eq!(
        inventory.refusals(),
        [EnumerationRefusal::UnclassifiedFileScopeConstruct]
    );
}

/// Preprocessing that cannot run leaves the historical raw scan in place and
/// refuses the claim on an environment ground — never a source-subject one.
#[test]
fn an_unavailable_expansion_refuses_completeness_on_an_environment_ground() {
    let source =
        "#define DEFN(n) int n(void) { return 0; }\nDEFN(run)\nint plain(void) { return 0; }\n";
    let broken = C11TranslationUnit {
        source,
        // An `-I` argument Clang rejects makes the enumeration-only
        // preprocessing fail without touching the rest of the pipeline.
        preprocessor_options: vec![qcore_c_gateway::legacy_c::CPreprocessorOption::Include(
            "\0".into(),
        )],
        function_scope: LegacyFunctionScope::MainSourceDefinitions,
        ..C11TranslationUnit::default()
    };
    let inventory = enumerate_source_definitions(&broken);
    assert_eq!(
        inventory.refusals(),
        [EnumerationRefusal::ExpansionUnavailable]
    );
    assert_eq!(inventory.definitions(), ["plain"]);
    assert_eq!(
        EnumerationRefusal::ExpansionUnavailable.category(),
        ReasonCategory::EnvironmentUnavailable
    );
    assert_eq!(
        EnumerationRefusal::ExpansionUnavailable
            .category()
            .subject(),
        ReasonSubject::Environment,
        "an environment failure must never be typed as a source defect"
    );
}

#[test]
fn every_refusal_ground_carries_a_classified_reason() {
    for refusal in [
        EnumerationRefusal::ExpansionUnavailable,
        EnumerationRefusal::PrimaryInputUnidentified,
        EnumerationRefusal::UnbalancedDelimiters,
        EnumerationRefusal::UnclassifiedFileScopeConstruct,
    ] {
        assert_eq!(
            ReasonCategory::classify(refusal.reason()),
            Ok(refusal.category()),
            "{} must classify to its own category",
            refusal.as_str()
        );
    }
}

// ---------------------------------------------------------------------------
// 5. Monotonicity: the denominator never shrinks
// ---------------------------------------------------------------------------

/// A definition guarded out by `#if 0` is absent from the expansion but present
/// in the raw scan. The union keeps it, so no historical row disappears and no
/// unit can become `Safe` because a name left the denominator.
#[test]
fn compiled_out_definitions_keep_their_historical_row() {
    if !clang_available() {
        return;
    }
    let source = "#if 0\nvoid hidden(void) {}\n#endif\nvoid subject(void) { return; }\n";
    assert_eq!(legacy_c_source_functions(source), ["hidden", "subject"]);
    let inventory = enumerate_source_definitions(&unit(source));
    assert_eq!(inventory.definitions(), ["hidden", "subject"]);
}

#[test]
fn the_enumeration_is_a_superset_of_the_raw_scan() {
    if !clang_available() {
        return;
    }
    for source in [
        "int a(void) { return 0; }\n",
        "#if 0\nint gone(void) { return 0; }\n#endif\nint b(void) { return 0; }\n",
        "#define DEFN(n) int n(void) { return 0; }\nDEFN(c)\nint d(void) { return 0; }\n",
        "int knr(x) int x; { return x; }\nint e(void) { return 0; }\n",
        "extern \"C\" {\nvoid first(void) {}\n}\n",
        "int f(void);\nint g(void) { return f(); }\n",
    ] {
        let raw = legacy_c_source_functions(source);
        let inventory = enumerate_source_definitions(&unit(source));
        for name in &raw {
            assert!(
                inventory.definitions().contains(name),
                "raw scan name `{name}` disappeared from {source:?}"
            );
        }
    }
}
