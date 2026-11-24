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

//! Unit tests of rule 77's decision tables, bound to the Lean vectors of
//! `QCore.BorrowedPathLocal`.

use super::*;
use crate::custody_free_copy::ALL_USES;

const LEAN: &str = include_str!("../../../../proof/QCore/BorrowedPathLocal.lean");

fn spelled(subject: Subject, used: Use) -> (&'static str, &'static str) {
    let subject = match subject {
        Subject::Root => "root",
        Subject::Local => "local",
    };
    let (_, used) =
        crate::custody_free_copy::spelled(crate::custody_free_copy::Subject::Copy, used);
    (subject, used)
}

fn decision(decision: Decision) -> &'static str {
    match decision {
        Decision::Admit => "admit",
        Decision::Withhold => "withhold",
    }
}

/// The rows of the Lean list `def NAME ... := [ ... ]`, each split into its
/// comma-separated fields with any leading `.` dropped.
fn lean_rows(name: &str) -> Vec<Vec<String>> {
    let lean = LEAN.replace("\r\n", "\n");
    let block = lean
        .split(&format!("def {name} "))
        .nth(1)
        .expect("the Lean vectors are present");
    let block = &block[block.find(":=").expect("definition") + 2..];
    let block = &block[..block.find(" ]\n").expect("end of list")];
    block
        .lines()
        .map(|line| line.trim().trim_start_matches(['[', ',']).trim().to_owned())
        .filter(|line| line.starts_with('('))
        .map(|row| {
            row.trim_start_matches('(')
                .trim_end_matches(')')
                .split(',')
                .map(|field| {
                    field
                        .trim()
                        .trim_start_matches('.')
                        .replace(['\u{ab}', '\u{bb}'], "")
                })
                .collect()
        })
        .collect()
}

#[test]
fn the_decision_table_is_the_vectors() {
    let mut seen = Vec::new();
    for subject in [Subject::Root, Subject::Local] {
        for used in ALL_USES {
            let (s, u) = spelled(subject, used);
            let expected = DECISION_VECTORS
                .iter()
                .find(|(vs, vu, _)| *vs == s && *vu == u)
                .map(|(_, _, d)| *d)
                .expect("every pair has a vector");
            assert_eq!(decision(decide(subject, used)), expected, "{s} {u}");
            seen.push((s, u));
        }
    }
    assert_eq!(seen.len(), DECISION_VECTORS.len());
}

#[test]
fn the_lean_decision_vectors_match_the_shared_table() {
    let rust: Vec<Vec<String>> = DECISION_VECTORS
        .iter()
        .map(|(s, u, d)| vec![(*s).to_owned(), (*u).to_owned(), (*d).to_owned()])
        .collect();
    assert_eq!(lean_rows("decisionVectors"), rust);
}

#[test]
fn the_value_and_binding_tables_are_the_vectors() {
    let kinds = [
        ("null", Written::Null),
        ("literal", Written::Literal),
        ("load", Written::Load),
        ("copy", Written::Copy),
        ("other", Written::Other),
    ];
    for ((name, kind), (vname, admits)) in kinds.iter().zip(VALUE_VECTORS) {
        assert_eq!(name, vname);
        assert_eq!(value_admits(*kind), *admits, "{name}");
    }
    let rust: Vec<Vec<String>> = VALUE_VECTORS
        .iter()
        .map(|(name, admits)| vec![(*name).to_owned(), admits.to_string()])
        .collect();
    assert_eq!(lean_rows("valueVectors"), rust);
    for (targets_empty, identity, widened, expected) in BINDING_VECTORS {
        assert_eq!(untracked(*targets_empty, *identity, *widened), *expected);
    }
    let rust: Vec<Vec<String>> = BINDING_VECTORS
        .iter()
        .map(|(t, i, w, u)| vec![t.to_string(), i.to_string(), w.to_string(), u.to_string()])
        .collect();
    assert_eq!(lean_rows("bindingVectors"), rust);
}

#[test]
fn the_transfer_decisions_need_both_facts() {
    for admitted in [false, true] {
        for binding in [false, true] {
            assert_eq!(
                guard_reads_untracked(admitted, binding),
                admitted && binding
            );
            assert_eq!(
                return_reads_untracked(admitted, binding),
                admitted && binding
            );
        }
    }
}
