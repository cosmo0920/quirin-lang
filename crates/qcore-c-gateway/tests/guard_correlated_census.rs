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

//! Card **TAIL-FILE-DEFINITION-CENSUS-INCOMPLETE** (rule 49): the definition
//! census of a unit whose conditional groups disagree only across groups with
//! the same guard (`docs/reports/tail-file-definition-census-incomplete/`).
//!
//! Fluent Bit 90978ac6's `plugins/in_tail/tail_file.c` (`tag_compose`, lines
//! 1255–1370) opens `else {` inside `#ifdef FLB_HAVE_REGEX … #endif` and
//! closes it inside a second `#ifdef FLB_HAVE_REGEX … #endif`. E01-RAW-COND
//! refuses each group alone; guard correlation reads both as kept or both as
//! skipped, so the census is decided and complete.

use qcore_c_gateway::legacy_c::guard_correlation::{
    CorrelationRefusal, WORLD_BUDGET, correlated_source_scan, guard_of,
};
use qcore_c_gateway::legacy_c::{
    C11TranslationUnit, ConditionalScanRefusal, LegacyFunctionScope, enumerate_source_definitions,
    legacy_c_source_scan,
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

/// `tag_compose` reduced to its load-bearing lines, followed by definitions
/// the raw scan hid (as `flb_tail_file_exists` and the rest of the file were).
const TAG_COMPOSE: &str = "\
static int before(void) { return 0; }\n\
#ifdef FLB_HAVE_REGEX\n\
static int tag_compose(char *tag, struct flb_regex *tag_regex, char *fname)\n\
#else\n\
static int tag_compose(char *tag, char *fname)\n\
#endif\n\
{\n\
    int i;\n\
#ifdef FLB_HAVE_REGEX\n\
    if (tag_regex) {\n\
        if (n <= 0) {\n\
            return -1;\n\
        }\n\
    }\n\
    else {\n\
#endif\n\
        p = strchr(tag, '*');\n\
        if (!p) {\n\
            return -1;\n\
        }\n\
#ifdef FLB_HAVE_REGEX\n\
    }\n\
#endif\n\
    return 0;\n\
}\n\
\n\
static inline int flb_tail_file_exists(int *st)\n\
{\n\
    return 0;\n\
}\n\
int flb_tail_file_remove_all(int *ctx)\n\
{\n\
    return 0;\n\
}\n";

fn with_guard_line(source: &str, from: &str, to: &str) -> String {
    source.replace(from, to)
}

// ---------------------------------------------------------------------------
// 1. The measured shape (positive)
// ---------------------------------------------------------------------------

#[test]
fn the_tag_compose_shape_is_refused_by_the_raw_scan_alone() {
    let scan = legacy_c_source_scan(TAG_COMPOSE);
    assert_eq!(
        scan.refusals(),
        [ConditionalScanRefusal::ArmDepthDisagreement]
    );
    // The scan carries the greatest depth, so everything after the body is
    // hidden from it.
    assert!(
        !scan
            .definitions()
            .contains(&"flb_tail_file_remove_all".to_owned())
    );
}

#[test]
fn guard_correlation_decides_the_tag_compose_shape() {
    assert_eq!(
        correlated_source_scan(TAG_COMPOSE).expect("decided"),
        [
            "before",
            "flb_tail_file_exists",
            "flb_tail_file_remove_all",
            "tag_compose"
        ]
    );
}

#[test]
fn the_census_of_the_tag_compose_shape_is_complete() {
    if !clang_available() {
        return;
    }
    let inventory = enumerate_source_definitions(&unit(TAG_COMPOSE));
    assert!(inventory.is_complete(), "{:?}", inventory.refusals());
    assert!(inventory.supports_unit_safe());
    for name in [
        "before",
        "flb_tail_file_exists",
        "flb_tail_file_remove_all",
        "tag_compose",
    ] {
        assert!(inventory.definitions().contains(&name.to_owned()), "{name}");
    }
    // The names the raw scan hid are now raw-scan names, not expansion-only.
    assert!(inventory.absent_from_raw_scan().is_empty());
}

#[test]
fn ifndef_correlates_with_ifdef() {
    let source = TAG_COMPOSE.replacen(
        "#ifdef FLB_HAVE_REGEX\n}\n#endif",
        "#ifndef FLB_HAVE_REGEX\n#else\n}\n#endif",
        1,
    );
    assert_ne!(source, TAG_COMPOSE);
    assert!(
        correlated_source_scan(&source)
            .expect("decided")
            .contains(&"flb_tail_file_remove_all".to_owned())
    );
}

#[test]
fn guard_lines_are_read_narrowly() {
    assert_eq!(guard_of("#ifdef A"), Some(("A".to_owned(), true)));
    assert_eq!(
        guard_of("#  ifndef\tB_1 // note"),
        Some(("B_1".to_owned(), false))
    );
    assert_eq!(
        guard_of("#ifdef A /* note */"),
        Some(("A".to_owned(), true))
    );
    assert_eq!(guard_of("#ifdef \\\nA"), Some(("A".to_owned(), true)));
    for other in [
        "#if defined(A)",
        "#if A",
        "#ifdef A B",
        "#ifdef(A)",
        "#ifdef 1A",
        "# /* x */ ifdef A",
        "#ifdef A /* open",
        "#elifdef A",
    ] {
        assert_eq!(guard_of(other), None, "{other}");
    }
}

// ---------------------------------------------------------------------------
// 2. Negatives: arms that genuinely disagree stay refused
// ---------------------------------------------------------------------------

#[test]
fn the_same_shape_behind_an_expression_stays_refused() {
    let source = with_guard_line(
        TAG_COMPOSE,
        "#ifdef FLB_HAVE_REGEX",
        "#if defined(FLB_HAVE_REGEX)",
    );
    assert_eq!(
        correlated_source_scan(&source),
        Err(CorrelationRefusal::ArmsDisagree)
    );
    if clang_available() {
        let inventory = enumerate_source_definitions(&unit(&source));
        assert!(!inventory.is_complete());
        assert_eq!(
            inventory
                .refusals()
                .iter()
                .map(|refusal| refusal.as_str())
                .collect::<Vec<_>>(),
            ["conditional-arm-depth-disagreement"]
        );
    }
}

#[test]
fn an_elif_makes_a_guard_group_ordinary() {
    let source = TAG_COMPOSE.replacen("else {\n#endif", "else {\n#elif defined(OTHER)\n#endif", 1);
    assert_ne!(source, TAG_COMPOSE);
    assert_eq!(
        correlated_source_scan(&source),
        Err(CorrelationRefusal::ArmsDisagree)
    );
}

#[test]
fn an_ordinary_group_inside_a_correlated_region_still_must_agree() {
    let source = TAG_COMPOSE.replacen(
        "p = strchr(tag, '*');\n",
        "p = strchr(tag, '*');\n#if X\n        {\n#endif\n",
        1,
    );
    assert_eq!(
        correlated_source_scan(&source),
        Err(CorrelationRefusal::ArmsDisagree)
    );
}

#[test]
fn different_guards_are_followed_in_every_configuration() {
    // Arms that leave different definition sets: where `A` is defined and `B`
    // is not, `g` sits inside a brace; elsewhere it is at file scope. Every
    // configuration is followed, so `g` is collected and nothing is hidden.
    let source = "\
#ifdef A\n\
int wrap(void) {\n\
#endif\n\
int g(void) { return 0; }\n\
#ifdef B\n\
}\n\
#endif\n\
int h(void) { return 1; }\n";
    assert_eq!(
        legacy_c_source_scan(source).refusals(),
        [ConditionalScanRefusal::ArmDepthDisagreement]
    );
    assert_eq!(
        correlated_source_scan(source).expect("decided"),
        ["g", "h", "wrap"]
    );
}

#[test]
fn a_define_between_the_groups_costs_precision_not_soundness() {
    let source = TAG_COMPOSE.replacen(
        "p = strchr(tag, '*');\n",
        "p = strchr(tag, '*');\n#undef FLB_HAVE_REGEX\n",
        1,
    );
    // After the `#undef` the second group splits again: in the world that kept
    // the first group's brace and skips the second one, the later definitions
    // are inside a brace, but the other worlds reach them at file scope.
    let names = correlated_source_scan(&source).expect("decided");
    assert!(names.contains(&"flb_tail_file_remove_all".to_owned()));
}

#[test]
fn too_many_worlds_refuse() {
    let mut source = String::from("int body(void) {\n");
    for index in 0..=WORLD_BUDGET {
        source.push_str(&format!("#ifdef G{index}\n{{\n#endif\n"));
    }
    source.push_str("}\nint after(void) { return 0; }\n");
    assert_eq!(
        correlated_source_scan(&source),
        Err(CorrelationRefusal::WorldBudget)
    );
}

#[test]
fn an_unstructured_stream_is_refused() {
    for source in ["#endif\n", "#ifdef A\n", "#ifdef A\n#else\n#else\n#endif\n"] {
        assert_eq!(
            correlated_source_scan(source),
            Err(CorrelationRefusal::UnstructuredGroup),
            "{source:?}"
        );
    }
}

#[test]
fn extern_c_linkage_braces_are_read_as_function_names_reads_them() {
    let source = "\
#ifdef __cplusplus\n\
extern \"C\" {\n\
#endif\n\
int f(void) {\n\
#ifdef A\n\
  if (a) {\n\
#endif\n\
  return 0;\n\
#ifdef A\n\
  }\n\
#endif\n\
}\n\
int g(void) { return 1; }\n\
#ifdef __cplusplus\n\
}\n\
#endif\n";
    assert_eq!(correlated_source_scan(source).expect("decided"), ["f", "g"]);
}

// ---------------------------------------------------------------------------
// 3. Exhaustive agreement with the Lean model
// ---------------------------------------------------------------------------

/// `QCore.GuardCorrelatedCensus.alphabet`, one line of C each; a `defn` at
/// position `k` is the definition `dk`.
const ALPHABET: [&str; 10] = [
    "{",
    "}",
    "DEFN",
    "#define R",
    "#ifdef A",
    "#ifndef A",
    "#if B",
    "#elif C",
    "#else",
    "#endif",
];

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

fn render(items: &[usize]) -> String {
    let mut text = String::new();
    for (position, item) in items.iter().enumerate() {
        if ALPHABET[*item] == "DEFN" {
            text.push_str(&format!("int d{position}(void) {{ }}\n"));
        } else {
            text.push_str(ALPHABET[*item]);
            text.push('\n');
        }
    }
    text
}

fn observation(items: &[usize]) -> u64 {
    match correlated_source_scan(&render(items)) {
        Err(CorrelationRefusal::UnstructuredGroup) => 1,
        Err(CorrelationRefusal::ArmsDisagree) => 2,
        Err(CorrelationRefusal::WorldBudget) => 3,
        Ok(names) => {
            4 + names
                .iter()
                .map(|name| 1u64 << name[1..].parse::<u32>().expect("a probe name"))
                .sum::<u64>()
        }
    }
}

/// `QCore.GuardCorrelatedCensus.domainChecksum 4`: the model and this
/// implementation agree over every item string of length at most four.
#[test]
fn the_lean_guard_domain_checksum_reproduces() {
    let mut domain = Vec::new();
    for length in 0..=4 {
        domain.extend(strings(length));
    }
    assert_eq!(domain.len(), 11_111, "the domain must match `domain 4`");
    let mut accumulator: u64 = 13;
    for items in &domain {
        accumulator = (accumulator * 1_000_033 + observation(items)) % 1_000_003;
    }
    assert_eq!(
        accumulator, 673_782,
        "rule 49's guard correlation and QCore.GuardCorrelatedCensus disagree over the length-4 domain"
    );
}

/// The Lean vectors of section 6, on rendered C.
#[test]
fn the_lean_vectors_hold() {
    // [item indices], expected observation (as in the Lean theorems).
    let lb = 0;
    let rb = 1;
    let defn = 2;
    let reset = 3;
    let ifdef = 4;
    let plain = 6;
    let alt = 7;
    let endif = 9;
    let tag = [lb, ifdef, lb, endif, ifdef, rb, endif, rb, defn];
    assert_eq!(
        correlated_source_scan(&render(&tag)),
        Ok(vec!["d8".to_owned()])
    );
    let tag_plain = [lb, plain, lb, endif, plain, rb, endif, rb, defn];
    assert_eq!(
        correlated_source_scan(&render(&tag_plain)),
        Err(CorrelationRefusal::ArmsDisagree)
    );
    let tag_elif = [lb, ifdef, lb, alt, endif, ifdef, rb, endif, rb, defn];
    assert_eq!(
        correlated_source_scan(&render(&tag_elif)),
        Err(CorrelationRefusal::ArmsDisagree)
    );
    let tag_reset = [lb, ifdef, lb, endif, reset, ifdef, rb, endif, rb, defn];
    assert_eq!(
        correlated_source_scan(&render(&tag_reset)),
        Ok(vec!["d9".to_owned()])
    );
}
