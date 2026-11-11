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

//! Conditional-group brace balance in the raw token scan (unit
//! **E01-RAW-COND**, `docs/reports/remaining-e01-raw-cond/README.md`).
//!
//! `docs/reports/remaining-e01-f1-raw/README.md` §3.2 recorded the residual
//! this file closes. `librdkafka/src/rdkafka.c`'s `rd_kafka_dir_is_empty`
//! contains
//!
//! ```c
//! #if defined(__sun)
//!         if (S_ISREG(st.st_mode) || …) {
//! #else
//!         if (d->d_type == DT_REG || …) {
//! #endif
//!                 closedir(dir);
//!                 return rd_false;
//!         }
//! ```
//!
//! Two arms each open a brace and one `}` closes, because exactly one arm
//! survives preprocessing. The raw scan kept **both** arms — E01-F1 §1.4 makes
//! that load-bearing, so a definition guarded out by `#if 0` never leaves the
//! denominator — so it counted two opens against one close and stayed one level
//! deep for the rest of the file, hiding 15 definitions.
//!
//! # The rule
//!
//! A `#if … #endif` group's resulting file-scope brace depth is **the greatest
//! depth its arms reach**, each arm scanned from the depth the group began at,
//! and a group with no `#else` carrying an implicit empty arm that reaches the
//! entry depth.
//!
//! * **No conditional is ever evaluated and no arm is ever selected.** The
//!   controlling expressions are not even read.
//! * **Agreement is exactness.** When every arm reaches the same depth — the
//!   `rd_kafka_dir_is_empty` case — the greatest depth *is* that depth, so the
//!   group is decided exactly whichever arm survives.
//! * **Disagreement is refused, not guessed.** The greatest depth dominates
//!   every arm selection, so the scan can only *hide* definitions, never invent
//!   one, and [`ConditionalScanRefusal::ArmDepthDisagreement`] is recorded so
//!   the enumeration's completeness claim is refused.
//! * **Monotonicity is preserved.** Every arm is still scanned, from the outer
//!   depth, so a definition written in any arm keeps its row.

use qcore_c_gateway::legacy_c::{
    C11TranslationUnit, ConditionalScanRefusal, LegacyFunctionScope, enumerate_source_definitions,
    legacy_c_source_functions, legacy_c_source_scan,
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

fn names(source: &str) -> Vec<String> {
    legacy_c_source_functions(source)
}

// ---------------------------------------------------------------------------
// 1. The measured shape
// ---------------------------------------------------------------------------

/// The exact `rd_kafka_dir_is_empty` shape, reduced to its load-bearing parts:
/// a conditional inside a function body whose two arms each open the brace of
/// an `if`, with a single `}` after `#endif`.
///
/// Before this unit the scan stayed one level deep for the rest of the file.
#[test]
fn the_exact_rdkafka_shape_no_longer_desynchronises_the_file() {
    let source = "\
int before(void) { return 0; }\n\
int rd_kafka_dir_is_empty(const char *path) {\n\
        while (readdir(dir)) {\n\
#if defined(__sun)\n\
                if (S_ISREG(st.st_mode) || S_ISDIR(st.st_mode) ||\n\
                    S_ISLNK(st.st_mode)) {\n\
#else\n\
                if (d->d_type == DT_REG || d->d_type == DT_LNK ||\n\
                    d->d_type == DT_DIR) {\n\
#endif\n\
                        closedir(dir);\n\
                        return 0;\n\
                }\n\
        }\n\
        return 1;\n\
}\n\
int after_one(void) { return 1; }\n\
int after_two(void) { return 2; }\n";
    assert_eq!(
        names(source),
        ["after_one", "after_two", "before", "rd_kafka_dir_is_empty"],
        "both arms reach the same depth, so the group is decided exactly"
    );
    let scan = legacy_c_source_scan(source);
    assert!(scan.is_exact(), "{:?}", scan.refusals());
}

/// The residual measured on the pinned corpus was fifteen definitions hidden
/// *after* one such body. All of them come back, and none of them is a new
/// name: the scan finds what was always written there.
#[test]
fn every_definition_after_an_agreeing_group_is_visible_again() {
    let mut source = String::from(
        "int subject(void) {\n\
#if defined(A)\n\
        if (a) {\n\
#else\n\
        if (b) {\n\
#endif\n\
                return 1;\n\
        }\n\
        return 0;\n\
}\n",
    );
    let mut expected = vec!["subject".to_owned()];
    for index in 0..15 {
        source.push_str(&format!(
            "int later_{index:02}(void) {{ return {index}; }}\n"
        ));
        expected.push(format!("later_{index:02}"));
    }
    expected.sort();
    assert_eq!(names(&source), expected);
}

// ---------------------------------------------------------------------------
// 2. Agreement, in every shape that agrees
// ---------------------------------------------------------------------------

/// Arms that agree are decided exactly whether they open, close, balance or
/// leave the depth alone — including an arm that closes an outer brace, which
/// the entry depth makes representable.
#[test]
fn arms_that_agree_are_decided_exactly() {
    // Both arms balanced: the classic guarded definition.
    assert_eq!(
        names(
            "#ifdef A\nint guarded(void) { return 0; }\n#else\nint other(void) { return 1; }\n#endif\nint after(void) { return 2; }\n"
        ),
        ["after", "guarded", "other"]
    );
    // Both arms open one brace, closed after the group.
    assert_eq!(
        names(
            "int subject(void) {\n#if A\n        while (a) {\n#else\n        while (b) {\n#endif\n        }\n        return 0;\n}\nint after(void) { return 1; }\n"
        ),
        ["after", "subject"]
    );
    // Both arms close the body's brace and each opens the next definition.
    assert_eq!(
        names(
            "int subject(void) {\n#if A\n        return 0;\n}\nint tail(void) {\n#else\n        return 1;\n}\nint tail(void) {\n#endif\n        return 2;\n}\nint after(void) { return 3; }\n"
        ),
        ["after", "subject", "tail"]
    );
    for source in [
        "#ifdef A\nint guarded(void) { return 0; }\n#endif\nint after(void) { return 1; }\n",
        "#if 0\nint dead(void) { return 0; }\n#endif\nint after(void) { return 1; }\n",
    ] {
        assert!(legacy_c_source_scan(source).is_exact(), "{source:?}");
    }
}

/// An `#elif` chain is a group with three or more arms, and the implicit empty
/// arm is only absent when an `#else` covers the last outcome.
#[test]
fn elif_chains_are_one_group_with_one_contribution() {
    let source = "\
int subject(void) {\n\
#if defined(A)\n\
        if (a) {\n\
#elif defined(B)\n\
        if (b) {\n\
#elif defined(C)\n\
        if (c) {\n\
#else\n\
        if (d) {\n\
#endif\n\
                return 1;\n\
        }\n\
        return 0;\n\
}\n\
int after(void) { return 2; }\n";
    assert_eq!(names(source), ["after", "subject"]);
    assert!(legacy_c_source_scan(source).is_exact());
    // Four balanced arms, each carrying its own definition: every one keeps a
    // row and the group contributes nothing.
    let balanced = "\
#if defined(A)\n\
int one(void) { return 1; }\n\
#elif defined(B)\n\
int two(void) { return 2; }\n\
#elif defined(C)\n\
int three(void) { return 3; }\n\
#else\n\
int four(void) { return 4; }\n\
#endif\n\
int after(void) { return 5; }\n";
    assert_eq!(
        names(balanced),
        ["after", "four", "one", "three", "two"],
        "E01-F1 §1.4 monotonicity: every arm keeps its definitions"
    );
    assert!(legacy_c_source_scan(balanced).is_exact());
}

/// Groups nest, and an inner group's contribution is computed before the arm
/// that contains it is measured.
#[test]
fn nested_groups_compose() {
    let source = "\
int subject(void) {\n\
#if defined(OUTER)\n\
        if (outer) {\n\
#  if defined(INNER)\n\
                x = 1;\n\
#  else\n\
                x = 2;\n\
#  endif\n\
#else\n\
        if (!outer) {\n\
#endif\n\
                return 1;\n\
        }\n\
        return 0;\n\
}\n\
int after(void) { return 2; }\n";
    assert_eq!(names(source), ["after", "subject"]);
    assert!(legacy_c_source_scan(source).is_exact());
    // An inner group that disagrees makes its arm's depth undecided, and the
    // refusal survives to the outer group.
    let inner_disagrees = "\
int subject(void) {\n\
#if defined(OUTER)\n\
#  if defined(INNER)\n\
        if (inner) {\n\
#  endif\n\
        return 1;\n\
#else\n\
        return 2;\n\
#endif\n\
}\n\
int after(void) { return 3; }\n";
    assert_eq!(
        legacy_c_source_scan(inner_disagrees).refusals(),
        [ConditionalScanRefusal::ArmDepthDisagreement]
    );
}

// ---------------------------------------------------------------------------
// 3. Disagreement: refused, never guessed
// ---------------------------------------------------------------------------

/// A single arm that opens a brace with no `#else` is the unbalanced adversary:
/// the group's outcomes are "one brace" and "no brace", so it is refused. The
/// scan takes the greater depth, which hides the later definition rather than
/// inventing anything.
#[test]
fn an_unbalanced_single_arm_is_refused() {
    let source = "\
int subject(void) {\n\
#if defined(OPEN_SCOPE)\n\
        if (a) {\n\
#endif\n\
        return 0;\n\
}\n\
int after(void) { return 1; }\n";
    let scan = legacy_c_source_scan(source);
    assert_eq!(
        scan.refusals(),
        [ConditionalScanRefusal::ArmDepthDisagreement]
    );
    assert!(!scan.is_exact());
    // Dominating, so the later definition is hidden — the fail-closed
    // direction — and never a phantom.
    assert_eq!(scan.definitions(), ["subject"]);
}

/// Two arms that disagree are refused in both directions, and the recorded
/// depth is the greatest of them.
#[test]
fn arms_that_disagree_are_refused_and_dominated() {
    for source in [
        // One arm opens, the other does not.
        "int subject(void) {\n#if A\n        if (a) {\n#else\n        if (b) ;\n#endif\n        return 0;\n}\nint after(void) { return 1; }\n",
        // The reverse order: the deeper arm is second.
        "int subject(void) {\n#if A\n        if (a) ;\n#else\n        if (b) {\n#endif\n        return 0;\n}\nint after(void) { return 1; }\n",
        // Three arms, two of which agree.
        "int subject(void) {\n#if A\n        if (a) {\n#elif B\n        if (b) {\n#else\n        if (c) ;\n#endif\n        return 0;\n}\nint after(void) { return 1; }\n",
    ] {
        let scan = legacy_c_source_scan(source);
        assert_eq!(
            scan.refusals(),
            [ConditionalScanRefusal::ArmDepthDisagreement],
            "{source:?}"
        );
        assert!(
            !scan.definitions().contains(&"after".to_owned()),
            "the greatest arm depth must dominate, hiding rather than inventing: {source:?}"
        );
    }
}

/// An arm that closes more than it opens is refused too, and the recorded depth
/// is the *entry* depth rather than the lowered one. This is the direction that
/// matters for soundness: summing the arms, which is what the scan did before
/// this unit, lowered the depth below every arm selection and turned a
/// block-macro statement inside the body into a **phantom definition**.
#[test]
fn a_closing_arm_is_refused_and_never_lowers_the_depth() {
    let source = "\
int subject(void) {\n\
#if 0\n\
}\n\
#endif\n\
        cfl_list_foreach(item, list) { use(item); }\n\
        return 0;\n\
}\n\
int after(void) { return 1; }\n";
    let scan = legacy_c_source_scan(source);
    assert_eq!(
        scan.refusals(),
        [ConditionalScanRefusal::ArmDepthDisagreement]
    );
    // No `cfl_list_foreach`: the group's depth dominates both outcomes, so the
    // block-macro statement stays inside the body where it is written.
    assert_eq!(scan.definitions(), ["after", "subject"]);
}

/// The same shape with the arm's definitions kept: an arm that closes the body
/// and opens a new definition contributes that definition, because every arm is
/// scanned from the depth the group began at.
#[test]
fn a_closing_arm_still_contributes_its_own_definitions() {
    let source = "\
int subject(void) {\n\
#if 0\n\
        return -1;\n\
}\n\
int commented_out(void) {\n\
#endif\n\
        return 0;\n\
}\n\
int after(void) { return 1; }\n";
    let scan = legacy_c_source_scan(source);
    assert_eq!(scan.definitions(), ["after", "commented_out", "subject"]);
    // The arm closes one brace and opens one, so it reaches the entry depth and
    // agrees with the implicit empty arm: nothing is refused.
    assert!(scan.is_exact(), "{:?}", scan.refusals());
}

/// Structural anomalies are a separate, named ground. None of them can occur in
/// a unit Clang preprocessed successfully, so each is a refusal rather than a
/// behaviour anyone relies on.
#[test]
fn unstructured_groups_are_refused() {
    for source in [
        // `#endif` with no open group.
        "int a(void) { return 0; }\n#endif\nint b(void) { return 1; }\n",
        // `#else` with no open group.
        "int a(void) { return 0; }\n#else\nint b(void) { return 1; }\n",
        // A group that is never closed.
        "int a(void) { return 0; }\n#if X\nint b(void) { return 1; }\n",
        // `#elif` after `#else`.
        "#if X\nint a(void) { return 0; }\n#else\nint b(void) { return 1; }\n#elif Y\nint c(void) { return 2; }\n#endif\n",
        // A second `#else`.
        "#if X\nint a(void) { return 0; }\n#else\nint b(void) { return 1; }\n#else\nint c(void) { return 2; }\n#endif\n",
    ] {
        let scan = legacy_c_source_scan(source);
        assert!(
            scan.refusals()
                .contains(&ConditionalScanRefusal::UnstructuredGroup),
            "{source:?} refusals {:?}",
            scan.refusals()
        );
    }
}

// ---------------------------------------------------------------------------
// 4. The wave-9 lexical rule is preserved
// ---------------------------------------------------------------------------

/// A conditional directive **inside a macro continuation** is not a directive
/// at all: E01-F1-RAW's rule reads a directive only when `#` is the first
/// preprocessing token of a *logical* line, and a `#define` continued with `\`
/// carries its whole replacement list. Group tracking must not resurrect it.
#[test]
fn a_conditional_inside_a_macro_continuation_is_not_a_group() {
    let source = "\
#define WEIRD(x)                     \\\n\
#if x                                \\\n\
        do_something(x);             \\\n\
#endif\n\
int after(void) { return 0; }\n";
    let scan = legacy_c_source_scan(source);
    assert_eq!(
        scan.definitions(),
        ["after"],
        "the whole replacement list is directive text"
    );
    assert!(
        scan.is_exact(),
        "no group was opened, so nothing is refused: {:?}",
        scan.refusals()
    );
}

/// The same in the other direction: an `#endif`-looking run that is *not* line
/// initial closes no group, and a real group around it is still decided.
#[test]
fn a_hash_after_code_opens_and_closes_no_group() {
    let source = "\
#define PASTE(a, b) a ## b\n\
int subject(void) { int x = 0; x = x ## 1; return x; }\n\
#if defined(A)\n\
int one(void) { return 1; }\n\
#endif\n\
int after(void) { return 2; }\n";
    let scan = legacy_c_source_scan(source);
    assert_eq!(scan.definitions(), ["after", "one", "subject"]);
    assert!(scan.is_exact(), "{:?}", scan.refusals());
}

/// Directive recognition is spelling-exact and total: white space, an indent, a
/// block comment and a phase-2 splice between `#` and the word are all legal,
/// and every other directive word is opaque text that opens no group.
#[test]
fn directive_words_are_recognised_exactly() {
    for source in [
        "int subject(void) {\n#   if A\n        if (a) {\n#   else\n        if (b) {\n#   endif\n        }\n        return 0;\n}\nint after(void) { return 1; }\n",
        "int subject(void) {\n  #if A\n        if (a) {\n  #else\n        if (b) {\n  #endif\n        }\n        return 0;\n}\nint after(void) { return 1; }\n",
        "int subject(void) {\n# /*c*/ if A\n        if (a) {\n# /*c*/ else\n        if (b) {\n# /*c*/ endif\n        }\n        return 0;\n}\nint after(void) { return 1; }\n",
        "int subject(void) {\n#\\\nif A\n        if (a) {\n#\\\nelse\n        if (b) {\n#\\\nendif\n        }\n        return 0;\n}\nint after(void) { return 1; }\n",
    ] {
        let scan = legacy_c_source_scan(source);
        assert_eq!(scan.definitions(), ["after", "subject"], "{source:?}");
        assert!(scan.is_exact(), "{source:?} {:?}", scan.refusals());
    }
    // Non-conditional directives open and close nothing, and an unknown word is
    // not silently treated as one.
    let opaque = "\
#include <stdio.h>\n\
#define X 1\n\
#undef X\n\
#pragma once\n\
#line 42 \"other.c\"\n\
#\n\
#ifdefined A\n\
int after(void) { return 0; }\n";
    let scan = legacy_c_source_scan(opaque);
    assert_eq!(scan.definitions(), ["after"]);
    assert!(scan.is_exact(), "{:?}", scan.refusals());
}

// ---------------------------------------------------------------------------
// 5. Exhaustive agreement with the Lean model
// ---------------------------------------------------------------------------

/// The six items `QCore.ConditionalGroupDepth.alphabet` enumerates, rendered as
/// one line of C each so that every directive is line-initial.
const ALPHABET: [&str; 6] = ["{", "}", "#if A", "#elif B", "#else", "#endif"];

/// Every item string of length exactly `length`, in the order
/// `QCore.ConditionalGroupDepth.strings` builds them: for each shorter string,
/// each alphabet item is consed on the front.
fn strings(length: usize) -> Vec<Vec<usize>> {
    if length == 0 {
        return vec![Vec::new()];
    }
    let mut out = Vec::new();
    for shorter in strings(length - 1) {
        for item in 0..ALPHABET.len() {
            let mut next = vec![item];
            next.extend(shorter.iter().copied());
            out.push(next);
        }
    }
    out
}

/// Render an item string, optionally followed by `closes` closing braces and a
/// probe definition that is a file-scope definition exactly when the scan is at
/// depth zero where it stands.
fn render(items: &[usize], closes: Option<usize>) -> String {
    let mut text = String::new();
    for item in items {
        text.push_str(ALPHABET[*item]);
        text.push('\n');
    }
    if let Some(closes) = closes {
        for _ in 0..closes {
            text.push_str("}\n");
        }
        text.push_str("int probe(void) { return 0; }\n");
    }
    text
}

/// The least number of closing braces that brings the scan back to file scope,
/// or 5 — `QCore.ConditionalGroupDepth.probeDepth`. For a stream with no open
/// group this is the file-scope brace depth itself.
fn probe_depth(items: &[usize]) -> u64 {
    for closes in 0..=4 {
        if legacy_c_source_functions(&render(items, Some(closes)))
            .iter()
            .any(|name| name == "probe")
        {
            return closes as u64;
        }
    }
    5
}

/// The Lean model of this rule (`proof/QCore/ConditionalGroupDepth.lean`) and
/// this implementation agree over every item string of length at most four:
/// both fold the same checksum of the observed depth and the two refusal
/// grounds. A divergence in either direction moves the number, so neither side
/// can be changed alone.
#[test]
fn the_lean_conditional_domain_checksum_reproduces() {
    let mut domain = Vec::new();
    for length in 0..=4 {
        domain.extend(strings(length));
    }
    assert_eq!(domain.len(), 1555, "the domain must match `domain 4`");

    let mut accumulator: u64 = 13;
    for items in &domain {
        let scan = legacy_c_source_scan(&render(items, None));
        let disagreed = scan
            .refusals()
            .contains(&ConditionalScanRefusal::ArmDepthDisagreement);
        let unstructured = scan
            .refusals()
            .contains(&ConditionalScanRefusal::UnstructuredGroup);
        let observation =
            probe_depth(items) * 4 + u64::from(disagreed) * 2 + u64::from(unstructured);
        accumulator = (accumulator * 1_000_033 + observation) % 1_000_003;
    }
    assert_eq!(
        accumulator, 360_626,
        "the gateway's conditional-group rule and QCore.ConditionalGroupDepth disagree over the length-4 domain"
    );
}

/// The Lean model omits an ordinary token because it proves it is the identity
/// on the scan state (`other_is_the_identity`). This pins the same thing of the
/// implementation, so omitting it from the domain costs nothing.
#[test]
fn an_ordinary_token_is_the_identity() {
    for items in strings(3) {
        let plain = render(&items, Some(0));
        let mut spaced = String::new();
        for item in &items {
            spaced.push_str(ALPHABET[*item]);
            spaced.push('\n');
            spaced.push_str(";\n");
        }
        spaced.push_str("int probe(void) { return 0; }\n");
        assert_eq!(
            legacy_c_source_functions(&plain),
            legacy_c_source_functions(&spaced),
            "an ordinary token changed the scan for {items:?}"
        );
    }
}

// ---------------------------------------------------------------------------
// 6. Monotonicity and the denominator
// ---------------------------------------------------------------------------

/// E01-F1 §1.4's load-bearing property: every arm is scanned from the depth the
/// group began at, so a definition guarded out by `#if 0` keeps its row and the
/// raw half of the union still contributes it.
#[test]
fn every_arm_keeps_its_definitions() {
    assert_eq!(
        names("#if 0\nint dead(void) { return 0; }\n#else\nint live(void) { return 1; }\n#endif\n"),
        ["dead", "live"]
    );
    assert_eq!(
        names(
            "int subject(void) {\n#if 0\n        return 0;\n}\nint guarded_tail(void) {\n#else\n        return 1;\n}\nint guarded_tail(void) {\n#endif\n        return 2;\n}\n"
        ),
        ["guarded_tail", "subject"]
    );
}

/// The raw scan stays a subset of the enumerated denominator on every shape
/// this unit touches, which is what keeps a unit from becoming `Safe` by losing
/// a row.
#[test]
fn the_enumeration_is_still_a_superset_of_the_group_aware_raw_scan() {
    if !clang_available() {
        return;
    }
    for source in [
        "int subject(void) {\n#if defined(A)\n        if (a) {\n#else\n        if (b) {\n#endif\n        }\n        return 0;\n}\nint after(void) { return 1; }\n",
        "#if 0\nint dead(void) { return 0; }\n#endif\nint b(void) { return 0; }\n",
        "int subject(void) {\n#if defined(OPEN)\n        if (a) {\n#endif\n        return 0;\n}\nint after(void) { return 1; }\n",
        "#if defined(A)\nint one(void) { return 1; }\n#elif defined(B)\nint two(void) { return 2; }\n#else\nint three(void) { return 3; }\n#endif\n",
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

/// A refused conditional group refuses the *enumeration completeness claim*,
/// because after it the raw half — the only half that carries a `#if 0`
/// definition — is a proper subset. A refused claim withholds a whole-unit
/// `Safe` verdict.
#[test]
fn a_refused_group_refuses_the_completeness_claim() {
    if !clang_available() {
        return;
    }
    let refused = "\
int subject(void) {\n\
#if defined(OPEN_SCOPE)\n\
        if (a) {\n\
#endif\n\
        return 0;\n\
}\n\
int after(void) { return 1; }\n";
    let inventory = enumerate_source_definitions(&unit(refused));
    assert!(!inventory.is_complete());
    assert!(!inventory.supports_unit_safe());
    assert_eq!(
        inventory
            .refusals()
            .iter()
            .map(|refusal| refusal.as_str())
            .collect::<Vec<_>>(),
        ["conditional-arm-depth-disagreement"]
    );
    assert!(
        inventory
            .withheld_safe_reason()
            .is_some_and(|reason| reason.contains("conditional group's arms"))
    );
    // The definition the raw scan could not place is still in the denominator:
    // the expansion reader supplies it, and it is reported as absent from the
    // raw scan rather than silently dropped.
    assert!(inventory.definitions().contains(&"after".to_owned()));
    assert_eq!(inventory.absent_from_raw_scan(), ["after"]);

    // A group whose arms agree keeps the claim established.
    let agreeing = "\
int subject(void) {\n\
#if defined(A)\n\
        if (a) {\n\
#else\n\
        if (b) {\n\
#endif\n\
        }\n\
        return 0;\n\
}\n\
int after(void) { return 1; }\n";
    let inventory = enumerate_source_definitions(&unit(agreeing));
    assert!(inventory.is_complete());
    assert!(inventory.supports_unit_safe());
    assert!(inventory.absent_from_raw_scan().is_empty());
}
