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

//! Card **RETURN-FAMILY-CONDITIONAL-CONSTANT-PRUNING** (gateway rule 74z):
//! family S of W45-RETURN-EXPRESSION-FAMILIES before an accessed
//! conditional whose condition is an integer constant expression the reader
//! evaluates exactly, end to end through the real gateway.
//!
//! At the base rule 74x made both arms bases whatever the condition, so
//! `long w(const struct s *q) { const struct s *p = 0; return (0 ? p : q)->n; }`
//! was `Unknown` (`unknown_alias:p`), though C never evaluates `p` (C11
//! 6.5.15p4). Every row asserts the status and the complete set of
//! diagnostic keys.
//!
//! A module of `return_family_untracked_conditional_access.rs`, whose
//! helpers it uses, so the suite gains no test target.

use super::{clang_available, expect};

const P: &[&str] = &["unknown_alias:p"];

/// `p` a null local, `q` and `r` unbound formals.
fn null_p(expression: &str) -> String {
    format!(
        "long w(long k, const struct s *q, const struct s *r) {{ \
         const struct s *p = 0; return {expression}; }}"
    )
}

/// The admitted fixtures: a decided condition drops the null `p` of the arm
/// C never evaluates, and the evaluated arm is an unbound formal. At the
/// base every row was `Unknown` (`unknown_alias:p`).
#[test]
fn a_decided_condition_prunes_the_unevaluated_arm() {
    if !clang_available() {
        return;
    }
    for (label, expression) in [
        ("constant false", "(0 ? p : q)->n"),
        ("constant true", "(1 ? q : p)->n"),
        ("parenthesised difference", "((2 - 2) ? p : q)->n"),
        ("unsigned comparison", "(-1 < 0u ? p : q)->n"),
        ("member subscript", "(0 ? p : q)->a[0]"),
        ("cast around", "((const struct s *)(0 ? p : q))->n"),
        ("decided inner, undecided outer", "(k ? (0 ? p : r) : q)->n"),
        ("decided outer, nested else", "(0 ? p : 1 ? q : p)->n"),
    ] {
        expect(label, &null_p(expression), "Safe", &[]);
    }
    // A macro is expanded before the reader sees the tokens, so a macro
    // that expands to `0` is read as the literal `0` C compiles in this
    // translation unit (report §2: a user decision is requested).
    expect(
        "macro expanding to a literal",
        "\n#define N 0\nlong w(const struct s *q) { const struct s *p = 0; \
         return (N ? p : q)->n; }",
        "Safe",
        &[],
    );
}

/// The refused fixtures: every condition the reader does not decide keeps
/// rule 74x's reading, both arms, so the null `p` stays a base.
#[test]
fn an_undecided_condition_keeps_both_arms() {
    if !clang_available() {
        return;
    }
    for (label, expression) in [
        ("formal condition", "(k ? p : q)->n"),
        ("sizeof", "(sizeof(int) - 4 ? p : q)->n"),
        ("division by zero", "(1 / 0 ? p : q)->n"),
        ("cast", "((int)0 ? p : q)->n"),
        ("signed overflow", "(2147483647 + 1 ? p : q)->n"),
        ("shift out of range", "(1 << 32 ? p : q)->n"),
        ("nested, inner undecided", "(1 ? (k ? p : r) : q)->n"),
    ] {
        expect(label, &null_p(expression), "Unknown", P);
    }
    expect(
        "difference of a local",
        "long w(const struct s *q) { int x = 0; const struct s *p = 0; \
         return (x - x ? p : q)->n; }",
        "Unknown",
        P,
    );
    // The reader reads the preprocessed token stream, so a macro's name
    // never reaches it: a macro that expands to a name is a name.
    expect(
        "macro expanding to a name",
        "\n#define N k\nlong w(long k, const struct s *q) { const struct s *p = 0; \
         return (N ? p : q)->n; }",
        "Unknown",
        P,
    );
    expect(
        "enumeration constant",
        "enum { N = 0 }; long w(const struct s *q) { const struct s *p = 0; \
         return (N ? p : q)->n; }",
        "Unknown",
        P,
    );
}

/// What pruning keeps: the evaluated arm's base, an old test's base inside
/// the untaken arm, and the arms of a conditional nothing accesses.
#[test]
fn pruning_keeps_every_other_obligation() {
    if !clang_available() {
        return;
    }
    for (label, expression) in [
        ("evaluated arm null", "(1 ? p : q)->n"),
        ("old test in the untaken arm", "(0 ? p->next : q)->n"),
        ("unaccessed conditional, arms accessed", "0 ? p->n : q->n"),
    ] {
        expect(label, &null_p(expression), "Unknown", P);
    }
}
