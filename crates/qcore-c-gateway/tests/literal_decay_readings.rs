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

//! W48-BU-SINGLE-LEAF-BODIES (gateway rule 35) — controls for a string-literal
//! dereference read as constant data (arm 1) and an array member decayed at a
//! `Borrowed` call actual read as rule 14c's element address (arm 2)
//! (`docs/reports/w48-bu-single-leaf-bodies/README.md`).
//!
//! The libc modules are loaded from their production provider files, byte for
//! byte; `sl_*` are this file's own test assumptions, not a model of any real
//! library. Every fixture goes through the public gateway entry point:
//!
//! * **accepting:** `XXH32_digest_endian`'s scalar-array actual, the same in a
//!   condition and as a statement, a `char` array member and one under a
//!   nested `.` member; `rd_kafka_mock_cluster_io_set_event`'s
//!   `rd_assert(!*"...")` in its glibc expansion, its `if` form and a plain
//!   condition;
//! * **refusing:** a pointer-element, a two-dimensional and a custody-taking
//!   decayed actual, a decayed member of a bound root, a subscripted literal
//!   and a literal store keep the refusal, and a leak beside a literal read is
//!   still reported.
//!
//! The candidates in these fixtures are fixture findings, not claims about any
//! upstream body.

use std::collections::BTreeMap;

use qcore_c_gateway::legacy_c::{
    C11TranslationUnit, LegacyDiagnosticKind, ResourceContracts, UnrestrictedLegacyResult,
    analyze_unrestricted_c11,
};
use qcore_contract::contract_loader::load_resource_contract_sources;

#[path = "support/text.rs"]
mod text;
use text::normalize_newlines;

const LIBC_STDLIB: &str =
    include_str!("../../../providers/quirin-provider-libc/contracts/libc/stdlib.qv");
const LIBC_MEMORY: &str =
    include_str!("../../../providers/quirin-provider-libc/contracts/libc/memory.qv");

const TEST_CONTRACTS: &str = r#"module contract;

extern fn sl_read(p: ptr<const, u8>, n: i32) -> i32
contract {
    requires resource_contract_v1;
    ensures normal resource_borrowed(p);
    ensures normal resource_borrowed(n);
    ensures normal resource_return_none();
};

extern fn sl_take(p: ptr<mut, u8>) -> unit
contract {
    requires resource_contract_v1;
    ensures normal resource_consumed(p, nonnull);
    ensures normal resource_return_none();
};
"#;

const DECLARATIONS: &str = "\
typedef __SIZE_TYPE__ size_t; \
void *malloc(size_t size); void free(void *ptr); \
extern void __assert_fail (const char *__assertion, const char *__file, \
unsigned int __line, const char *__function) __attribute__ ((__noreturn__)); \
int sl_read(const void *p, int n); \
void sl_take(void *p);\n\
#define NULL ((void *)0)\n";

const INDIRECT: &str = "indirect memory access is outside the resource projection";
const BRANCH: &str = "unmodeled call, aggregate or indirect access in branch condition";

fn contracts() -> ResourceContracts {
    load_resource_contract_sources(&BTreeMap::from([
        ("libc/stdlib.qv".to_owned(), normalize_newlines(LIBC_STDLIB)),
        ("libc/memory.qv".to_owned(), normalize_newlines(LIBC_MEMORY)),
        ("sl.qv".to_owned(), TEST_CONTRACTS.to_owned()),
    ]))
    .unwrap_or_else(|error| panic!("contract text rejected: {error}"))
}

fn clang_available() -> bool {
    match qcore_c_gateway::clang_command() {
        Ok(_) => true,
        Err(reason) => {
            assert!(
                std::env::var_os("QCORE_CLANG").is_none(),
                "explicit QCORE_CLANG configuration is invalid: {reason}"
            );
            eprintln!("skipping Clang-dependent W48-BU-SINGLE-LEAF-BODIES test: {reason}");
            false
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
enum Outcome {
    Safe,
    Unsafe,
    Unknown(Vec<String>),
}

fn analyze(body: &str) -> Outcome {
    let source = format!("{DECLARATIONS}{body}\n");
    let result = analyze_unrestricted_c11(
        &C11TranslationUnit {
            source: &source,
            ..Default::default()
        },
        "f",
        &contracts(),
    )
    .unwrap_or_else(|error| panic!("fixture rejected by Clang: {error:?}\n{source}"));
    match result {
        UnrestrictedLegacyResult::Safe(_) => Outcome::Safe,
        UnrestrictedLegacyResult::Unsafe(_) => Outcome::Unsafe,
        UnrestrictedLegacyResult::Unknown { partial, .. } => Outcome::Unknown(
            partial
                .and_then(|summary| summary.functions.get("f").cloned())
                .map(|function| {
                    function
                        .diagnostics
                        .iter()
                        .map(|diagnostic| match &diagnostic.kind {
                            LegacyDiagnosticKind::Unsupported { description } => {
                                description.clone()
                            }
                            other => format!("{other:?}"),
                        })
                        .collect()
                })
                .unwrap_or_default(),
        ),
    }
}

const ACCEPTING: &[(&str, &str)] = &[
    (
        "XXH32_digest_endian's scalar array member at a borrowed actual",
        "struct st { unsigned v; unsigned mem32[4]; unsigned memsize; };\n\
         int f(const struct st *s) {\n return sl_read(s->mem32, s->memsize);\n}",
    ),
    (
        "the same actual in a condition",
        "struct st { unsigned v; unsigned mem32[4]; unsigned memsize; };\n\
         int f(const struct st *s) {\n if (sl_read(s->mem32, 1)) return 1;\n return 0;\n}",
    ),
    (
        "a char array member in a statement",
        "struct st { char name[16]; int n; };\n\
         int f(struct st *s) {\n int r;\n r = sl_read(s->name, s->n);\n return r;\n}",
    ),
    (
        "an array member under a nested member",
        "struct in { unsigned mem[4]; }; struct st { struct in i; int n; };\n\
         int f(struct st *s) {\n return sl_read(s->i.mem, s->n);\n}",
    ),
    (
        "rd_assert(!*\"...\") in its glibc expansion",
        "void f(int n) {\n ((void) sizeof ((!*\"lit\") ? 1 : 0), __extension__ ({ if (!*\"lit\") ; \
         else __assert_fail (\"!*\\\"lit\\\"\", \"f.c\", 3, __extension__ __PRETTY_FUNCTION__); }));\n}",
    ),
    (
        "rd_kafka_mock_cluster_io_set_event's shape",
        "void f(int *a, int n) {\n int i;\n for (i = 0; i < n; i++) {\n if (a[i] == 3) {\n \
         a[i] |= 1;\n return;\n }\n }\n if (!*\"not found\") ; else __assert_fail(\"x\", \"f.c\", 9, __func__);\n}",
    ),
    (
        "a literal read in a plain condition",
        "int f(int n) {\n if (!*\"lit\") return 1;\n return n;\n}",
    ),
];

#[test]
fn each_accepting_fixture_is_safe() {
    if !clang_available() {
        return;
    }
    for (label, body) in ACCEPTING {
        assert_eq!(analyze(body), Outcome::Safe, "{label}");
    }
}

fn refused_with(outcome: &Outcome, description: &str) -> bool {
    matches!(outcome, Outcome::Unknown(leaves) if leaves.iter().any(|leaf| leaf == description))
}

#[test]
fn a_pointer_or_two_dimensional_element_keeps_the_refusal() {
    if !clang_available() {
        return;
    }
    let pointers = analyze(
        "struct st { char *names[4]; int n; };\n\
         int f(struct st *s) {\n return sl_read(s->names, s->n);\n}",
    );
    assert!(refused_with(&pointers, INDIRECT), "{pointers:?}");
    let matrix = analyze(
        "struct st { char m[4][4]; int n; };\n\
         int f(struct st *s) {\n return sl_read(s->m, s->n);\n}",
    );
    assert!(refused_with(&matrix, INDIRECT), "{matrix:?}");
}

#[test]
fn a_decayed_member_of_a_bound_root_keeps_the_refusal() {
    if !clang_available() {
        return;
    }
    for body in [
        "struct st { char name[16]; int n; };\n\
         int f(void) {\n int r;\n struct st *s = malloc(sizeof *s);\n if (!s) return 0;\n \
         r = sl_read(s->name, 1);\n free(s);\n return r;\n}",
        "struct st { char name[16]; int n; };\n\
         int f(void) {\n int r;\n struct st *s = malloc(sizeof *s);\n if (!s) return 0;\n \
         free(s);\n r = sl_read(s->name, 1);\n return r;\n}",
    ] {
        let outcome = analyze(body);
        assert!(refused_with(&outcome, INDIRECT), "{outcome:?}");
    }
}

#[test]
fn a_decayed_member_at_a_custody_position_is_never_safe() {
    if !clang_available() {
        return;
    }
    for body in [
        "struct st { char name[16]; int n; };\n void f(struct st *s) {\n sl_take(s->name);\n}",
        "struct st { char name[16]; int n; };\n void f(struct st *s) {\n free(s->name);\n}",
    ] {
        let outcome = analyze(body);
        assert_ne!(outcome, Outcome::Safe, "{body}");
    }
}

#[test]
fn a_subscripted_or_stored_literal_keeps_the_refusal() {
    if !clang_available() {
        return;
    }
    let subscript = analyze("int f(int n) {\n if (\"lit\"[1] == 'i') return 1;\n return n;\n}");
    assert!(refused_with(&subscript, BRANCH), "{subscript:?}");
    let store = analyze("void f(void) {\n *\"lit\" = 'x';\n}");
    assert_ne!(store, Outcome::Safe, "{store:?}");
}

#[test]
fn a_leak_beside_a_literal_read_is_still_reported() {
    if !clang_available() {
        return;
    }
    let outcome = analyze(
        "int f(void) {\n void *p = malloc(4);\n if (!*\"lit\") return 1;\n free(p);\n return 0;\n}",
    );
    assert_eq!(outcome, Outcome::Unsafe);
}
