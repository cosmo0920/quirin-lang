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

//! The raw tokenizer's logical-line directive rule (unit **E01-F1-RAW**,
//! `docs/reports/remaining-e01-f1-raw/README.md`).
//!
//! `docs/reports/remaining-e01-f1/README.md` §2.2 diagnosed, and deliberately
//! did not repair, a lexical defect in the gateway's raw token scan
//! (`legacy_c_source_functions`, via `legacy_c::tokenize`): **every** `#` was
//! read as a directive introducer and discarded to the end of the *physical*
//! line. Two things follow, and both were wrong:
//!
//! * a `#` on a macro's backslash-continuation line — the stringification
//!   operator of `&(#ENUM)[18]` — swallowed the rest of that line, closing
//!   brace included, so the scan's file-scope brace depth never returned to
//!   zero and every later definition in the file was invisible. That hid 146
//!   of `librdkafka/src/rdkafka.c`'s 165 definitions;
//! * a `#define` continued with `\`-newline had only its *first* line skipped,
//!   so the rest of its replacement list was tokenized as ordinary file-scope
//!   source.
//!
//! The rule implemented instead is C11's: a directive begins with `#` as the
//! first preprocessing token of a **logical** line (nothing but white space and
//! comments before it since the last new-line phase 2 did not delete), and it
//! runs to the end of that logical line. A `#` anywhere else is an ordinary
//! punctuator.
//!
//! This file is the adversarial matrix for that rule. Every case is stated
//! against the *public* raw scan, so a regression is visible at the API the
//! rest of the repository consumes.

use qcore_c_gateway::legacy_c::{
    C11TranslationUnit, LegacyFunctionScope, enumerate_source_definitions,
    legacy_c_source_functions,
};

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
// 1. The measured defect
// ---------------------------------------------------------------------------

/// The exact `librdkafka/src/rdkafka.c` shape, byte-for-byte in its
/// load-bearing parts: a two-line `#define` whose continuation line carries a
/// stringification `#` *and* the closing brace of a compound initializer.
///
/// Before the repair the raw scan returned nothing at all here. It now returns
/// both definitions that follow the table, and the enumeration agrees with it
/// exactly — `absent_from_raw_scan` is empty, which is the whole point of this
/// unit.
#[test]
fn a_stringification_on_a_continuation_line_is_not_a_directive() {
    let source = "\
struct entry { int code; const char *name; };\n\
#define _ERR_DESC(ENUM, DESC)                                                  \\\n\
        [ENUM - RD_KAFKA_RESP_ERR__BEGIN] = {ENUM, &(#ENUM)[18] /*pfx*/, DESC}\n\
static const struct entry table[] = { _ERR_DESC(1, \"one\"), _ERR_DESC(2, \"two\") };\n\
int after_the_table(void) { return table[0].code; }\n\
int also_after(void) { return 0; }\n";
    assert_eq!(
        legacy_c_source_functions(source),
        ["after_the_table", "also_after"],
        "the continuation line's `#` must not swallow the closing brace"
    );
}

/// The same shape with the brace desynchronisation made explicit: without the
/// repair the scan's depth ends above zero and *everything* after the macro is
/// lost, however many definitions follow.
#[test]
fn a_desynchronised_brace_depth_no_longer_hides_the_rest_of_the_file() {
    let source = "\
#define WRAP(E)   \\\n\
        {E, &(#E)[0]}\n\
static const struct pair table[] = { WRAP(1) };\n\
int one(void) { return 1; }\n\
int two(void) { return 2; }\n\
int three(void) { return 3; }\n";
    assert_eq!(legacy_c_source_functions(source), ["one", "three", "two"]);
}

/// Token pasting is the other non-line-initial `#`, and `##` is two of them.
/// Neither introduces a directive, on the first line of a macro or a later one.
#[test]
fn token_pasting_is_not_a_directive() {
    let source = "\
#define JOIN(a, b) a##b\n\
#define JOIN3(a, b, c)   \\\n\
        a ## b ## c\n\
int after_pasting(void) { return 0; }\n";
    assert_eq!(legacy_c_source_functions(source), ["after_pasting"]);
}

// ---------------------------------------------------------------------------
// 2. Macro bodies contribute no token and no brace — uniformly
// ---------------------------------------------------------------------------

/// A single-line macro body containing braces was already invisible to the
/// scan; a multi-line one now is too. Both are checked together so the two can
/// never drift apart again: a macro's braces perturb no file-scope depth.
///
/// This is the deliberate behavioural extension of the repair, recorded in
/// `docs/reports/remaining-e01-f1-raw/README.md` §1.3: continuation lines of a
/// directive are part of that directive.
#[test]
fn macro_bodies_never_perturb_file_scope_brace_depth() {
    let single = "\
#define BLOCK do { int local = 0; (void) local; } while (0)\n\
int subject(void) { return 0; }\n";
    assert_eq!(legacy_c_source_functions(single), ["subject"]);

    let multi = "\
#define BLOCK             \\\n\
        do {              \\\n\
            int local = 0;\\\n\
            (void) local; \\\n\
        } while (0)\n\
int subject(void) { return 0; }\n";
    assert_eq!(legacy_c_source_functions(multi), ["subject"]);

    // An unbalanced macro body — a real and common C idiom — used to leave the
    // depth permanently raised from its continuation line. It no longer can.
    let unbalanced = "\
#define OPEN_SCOPE(name)  \\\n\
        if (name) {       \\\n\
            do_something();\n\
int still_visible(void) { return 0; }\n";
    assert_eq!(legacy_c_source_functions(unbalanced), ["still_visible"]);
}

/// The measured cost of the same rule, stated rather than hidden: a
/// *function-like macro that expands to a whole definition* used to contribute
/// a phantom name — its own formal parameter — because its continuation line
/// was tokenized as file-scope source. The name never denoted a definition
/// anywhere: the analyzer's body finder cannot find `n` in the preprocessed
/// unit, so it was a row nothing could discharge.
///
/// It is gone now, and the *real* definition the macro produces is enumerated
/// in its place by E01-F1's expansion reader, so the audited denominator does
/// not lose a definition — it loses a name that was never one.
#[test]
fn a_macro_parameter_is_no_longer_a_phantom_definition() {
    let source = "\
#define DEFN(n)                    \\\n\
        int n(void) { return 0; }\n\
DEFN(produced)\n\
int written(void) { return 1; }\n";
    assert_eq!(
        legacy_c_source_functions(source),
        ["written"],
        "the macro's formal `n` is not a definition"
    );
    if !clang_available() {
        return;
    }
    let inventory = enumerate_source_definitions(&unit(source));
    assert_eq!(inventory.definitions(), ["produced", "written"]);
    assert_eq!(inventory.absent_from_raw_scan(), ["produced"]);
}

// ---------------------------------------------------------------------------
// 3. `#` that is not a directive and not an operator: literals and comments
// ---------------------------------------------------------------------------

/// A `#` inside a string literal, a character constant or a comment is text,
/// not a token. Each is checked with a following definition, so a lexer that
/// mis-terminated any of them would lose it.
#[test]
fn hashes_in_literals_and_comments_are_inert() {
    let in_string = "\
const char *fragment = \"#define NOT_A_DIRECTIVE 1\";\n\
int after_string(void) { return 0; }\n";
    assert_eq!(legacy_c_source_functions(in_string), ["after_string"]);

    let in_char = "int hash_char(void) { return '#' == 35; }\nint after_char(void) { return 0; }\n";
    assert_eq!(
        legacy_c_source_functions(in_char),
        ["after_char", "hash_char"]
    );

    let in_line_comment = "\
// # this is not a directive and neither is # this\n\
int after_line_comment(void) { return 0; }\n";
    assert_eq!(
        legacy_c_source_functions(in_line_comment),
        ["after_line_comment"]
    );

    let in_block_comment = "\
/* #define HIDDEN { \n\
   # still inside the comment } */\n\
int after_block_comment(void) { return 0; }\n";
    assert_eq!(
        legacy_c_source_functions(in_block_comment),
        ["after_block_comment"]
    );

    // A string literal that *starts* a line must not be read as anything but a
    // literal, and the `#` after it on the same line is a punctuator, not a
    // directive introducer.
    let after_literal = "const char *a = \"x\"; int b = 0; \nint after(void) { return 0; }\n";
    assert_eq!(legacy_c_source_functions(after_literal), ["after"]);
}

/// A `//` comment ending in a backslash continues onto the next line: phase 2
/// splices before phase 3 comments. The spliced line is comment text, so a
/// definition spelled there is genuinely not a definition — and, critically,
/// its braces do not count.
#[test]
fn a_spliced_line_comment_swallows_its_continuation() {
    let source = "\
// the next line is still a comment  \\\n\
int commented_out(void) { return 0; }\n\
int real(void) { return 1; }\n";
    assert_eq!(legacy_c_source_functions(source), ["real"]);
}

// ---------------------------------------------------------------------------
// 4. Real directives keep working, before and after a continued one
// ---------------------------------------------------------------------------

/// The directive that *follows* a continued macro is still a directive. A lexer
/// that consumed one line too many, or too few, breaks exactly here.
#[test]
fn a_directive_after_a_continued_macro_is_still_a_directive() {
    let source = "\
#define CONTINUED(a, b)   \\\n\
        ((a) + (b))       \\\n\
        /* and a trailing comment */\n\
#define PLAIN 1\n\
#undef PLAIN\n\
int subject(void) { return CONTINUED(1, 2); }\n";
    assert_eq!(legacy_c_source_functions(source), ["subject"]);
}

/// Nested conditional groups, including a `#if 0` arm that spells a definition
/// and an `#else` arm that spells another. The scan is not a preprocessor: it
/// keeps *both* arms' definitions, which is E01-F1's monotonicity base and is
/// unchanged here. What must hold is that the directive lines themselves
/// contribute nothing.
#[test]
fn nested_conditionals_contribute_no_token() {
    let source = "\
#if defined(A)\n\
#  if defined(B)\n\
int inner(void) { return 0; }\n\
#  else\n\
int alternative(void) { return 1; }\n\
#  endif\n\
#elif defined(C)\n\
int elif_arm(void) { return 2; }\n\
#endif\n\
int always(void) { return 3; }\n";
    assert_eq!(
        legacy_c_source_functions(source),
        ["alternative", "always", "elif_arm", "inner"]
    );
}

/// A directive may be indented, may have white space between `#` and its name,
/// and may follow a multi-line block comment — the `#` is still the first
/// preprocessing token of its logical line in all three.
#[test]
fn indented_and_comment_preceded_directives_are_directives() {
    let source = "\
    #   define INDENTED { not_a_brace }\n\
/* a comment\n\
   spanning lines */ #define AFTER_COMMENT { nor_this }\n\
int subject(void) { return 0; }\n";
    assert_eq!(legacy_c_source_functions(source), ["subject"]);
}

/// The converse, and the reason the flag is cleared by every token: a `#` that
/// follows real code on the same line is **not** a directive, so it must not
/// discard the rest of that line. Here the discarded text would have contained
/// the closing brace of `subject`.
#[test]
fn a_hash_after_code_on_the_same_line_is_not_a_directive() {
    let source = "int subject(void) { int x = 0; # (void) x; }\nint after(void) { return 0; }\n";
    assert_eq!(legacy_c_source_functions(source), ["after", "subject"]);
}

// ---------------------------------------------------------------------------
// 5. Nothing that already worked stops working
// ---------------------------------------------------------------------------

/// Pre-existing raw-scan behaviour that other units pin, re-checked here so
/// this file fails first if the lexer regresses: an `extern "C"` linkage block
/// is entered, a `#if 0` body keeps its historical row, a prototype is not a
/// definition, and a block-macro statement inside a body is not one either.
#[test]
fn the_established_raw_scan_behaviour_is_unchanged() {
    assert_eq!(
        legacy_c_source_functions(
            "extern \"C\" {\nvoid first(void) {}\nint second(void) { return 0; }\n}"
        ),
        ["first", "second"]
    );
    assert_eq!(
        legacy_c_source_functions(
            "#if 0\nvoid hidden(void) {}\n#endif\nvoid subject(void) { return; }\n"
        ),
        ["hidden", "subject"]
    );
    assert_eq!(
        legacy_c_source_functions(
            "int declared(void);\nint defined(void) { return declared(); }\n"
        ),
        ["defined"]
    );
    assert_eq!(
        legacy_c_source_functions(
            "void subject(void) {\n    cfl_list_foreach(item, list) { use(item); }\n}\n"
        ),
        ["subject"]
    );
}

// ---------------------------------------------------------------------------
// 6. The residual this repair did not close, closed by unit E01-RAW-COND
// ---------------------------------------------------------------------------

/// A **second, distinct** desynchronisation cause, reduced from the real
/// `librdkafka/src/rdkafka.c` body of `rd_kafka_dir_is_empty` (pinned Fluent
/// Bit `66910c10`, lines 7080-7118). Two conditional arms each open a brace and
/// only one `}` follows, because exactly one arm survives preprocessing:
///
/// ```c
/// #if defined(__sun)
///         if (S_ISREG(st.st_mode)) {
/// #else
///         if (d->d_type == DT_REG) {
/// #endif
///                 return rd_false;
///         }
/// ```
///
/// The raw scan is deliberately **not** a preprocessor — E01-F1 §1.4 makes
/// keeping both arms load-bearing, so that a definition guarded out by `#if 0`
/// never leaves the denominator — so at this unit's checkpoint it counted both
/// opens and one close and stayed one level deep for the rest of the file. That
/// residual was recorded as boundary **E01-F1-RAW-COND** and is closed by unit
/// **E01-RAW-COND** (`docs/reports/remaining-e01-raw-cond/README.md`) *without*
/// evaluating a conditional: both arms of this group leave the same file-scope
/// depth, so that depth is the group's depth whichever arm survives, and no arm
/// is selected.
///
/// **Deliberate pin update.** The load-bearing half is kept and strengthened:
/// the enumerated denominator is still exactly these three definitions, the raw
/// scan is still a subset of it, and the two mechanisms now *agree* on this
/// shape instead of disagreeing by one row. The refusal direction — a group
/// whose arms do *not* agree — is pinned in
/// `crates/qcore-c-gateway/tests/e01_raw_cond.rs`.
#[test]
fn a_conditional_brace_imbalance_is_decided_when_the_arms_agree() {
    let source = "\
int before_the_imbalance(void) { return 0; }\n\
int subject(const char *path) {\n\
#if defined(__sun)\n\
        if (path[0] == 'a') {\n\
#else\n\
        if (path[1] == 'b') {\n\
#endif\n\
                return 1;\n\
        }\n\
        return 0;\n\
}\n\
int after_the_imbalance(void) { return 2; }\n";
    // Both arms leave the same file-scope depth, so the group contributes that
    // depth once and the scan returns to zero after the body.
    assert_eq!(
        legacy_c_source_functions(source),
        ["after_the_imbalance", "before_the_imbalance", "subject"],
        "E01-RAW-COND: agreeing arms are decided without selecting one"
    );
    if !clang_available() {
        return;
    }
    // The audited denominator is unchanged; the two mechanisms now agree.
    let inventory = enumerate_source_definitions(&unit(source));
    assert!(
        inventory
            .definitions()
            .contains(&"after_the_imbalance".to_owned()),
        "the enumeration must still cover every definition: {:?}",
        inventory.definitions()
    );
    assert!(
        inventory.absent_from_raw_scan().is_empty(),
        "the raw scan and the enumeration agree on this shape: {:?}",
        inventory.absent_from_raw_scan()
    );
    assert!(inventory.is_complete());
}

/// The repair must not make the enumeration and the raw scan disagree in the
/// dangerous direction. For every shape here the raw scan stays a **subset** of
/// the enumerated denominator, which is what keeps a unit from becoming `Safe`
/// by losing a row.
#[test]
fn the_enumeration_is_still_a_superset_of_the_repaired_raw_scan() {
    if !clang_available() {
        return;
    }
    for source in [
        "int a(void) { return 0; }\n",
        "#if 0\nint gone(void) { return 0; }\n#endif\nint b(void) { return 0; }\n",
        "#define DEFN(n) int n(void) { return 0; }\nDEFN(c)\nint d(void) { return 0; }\n",
        "#define DEFN(n)  \\\n        int n(void) { return 0; }\nDEFN(c2)\nint d2(void) { return 0; }\n",
        "#define ENTRY(E)  \\\n        {E, &(#E)[0]}\nstatic const struct p t[] = { ENTRY(1) };\nint e(void) { return 0; }\n",
        "int knr(x) int x; { return x; }\nint f(void) { return 0; }\n",
        "extern \"C\" {\nvoid g(void) {}\n}\n",
    ] {
        let raw = legacy_c_source_functions(source);
        let inventory = enumerate_source_definitions(&unit(source));
        for name in &raw {
            assert!(
                inventory.definitions().contains(name),
                "raw scan name `{name}` is absent from the denominator for {source:?}"
            );
        }
    }
}
