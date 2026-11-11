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

//! Card CG10-INDIRECT-ACCESS-SOLE-BLOCKERS (rule 12j): the decision table, its
//! Lean binding and the storage reader.

use super::*;

fn storage(code: &str) -> RootStorage {
    match code {
        "automatic" => RootStorage::Automatic,
        "parameter" => RootStorage::Parameter,
        "specified" => RootStorage::Specified,
        "outside" => RootStorage::Outside,
        other => panic!("unknown storage {other}"),
    }
}

fn access(code: &str) -> Access {
    match code {
        "scalar-read" => Access::ScalarRead,
        "scalar-store" => Access::ScalarStore,
        "other" => Access::Other,
        other => panic!("unknown access {other}"),
    }
}

#[test]
fn the_decision_vectors_hold() {
    for &(dot_only, root, use_, expected) in DECISION_VECTORS {
        assert_eq!(
            decide(dot_only, storage(root), access(use_)).code(),
            expected,
            "{dot_only} {root} {use_}"
        );
    }
}

#[test]
fn only_a_dot_only_scalar_place_on_automatic_storage_is_admitted() {
    for dot_only in [false, true] {
        for root in ["automatic", "parameter", "specified", "outside"] {
            for use_ in ["scalar-read", "scalar-store", "other"] {
                let admitted = decide(dot_only, storage(root), access(use_)) == Decision::Admit;
                assert_eq!(
                    admitted,
                    dot_only && matches!(root, "automatic" | "parameter") && use_ != "other",
                    "{dot_only} {root} {use_}"
                );
            }
        }
    }
}

/// `camelCase` Lean constructor to the Rust code (`scalarRead` → `scalar-read`).
fn kebab(name: &str) -> String {
    let mut code = String::new();
    for character in name.trim().trim_start_matches('.').chars() {
        if character.is_ascii_uppercase() {
            code.push('-');
            code.push(character.to_ascii_lowercase());
        } else {
            code.push(character);
        }
    }
    code
}

#[test]
fn the_lean_decision_vectors_are_the_rust_table() {
    let lean = include_str!("../../../../proof/QCore/RecordObjectPlace.lean").replace("\r\n", "\n");
    let block = lean
        .split("def decisionVectors ")
        .nth(1)
        .expect("decisionVectors in the Lean model");
    let block = &block[block.find(":=").expect("definition") + 2..];
    let block = &block[..block.find(" ]\n").expect("end of list")];
    let rows = block
        .lines()
        .map(|line| line.trim().trim_start_matches(['[', ',']).trim().to_owned())
        .filter(|line| line.starts_with('('))
        .map(|row| {
            let fields = row
                .trim_start_matches('(')
                .trim_end_matches(')')
                .split(',')
                .map(str::trim)
                .collect::<Vec<_>>();
            assert_eq!(fields.len(), 4, "{row}");
            (
                fields[0] == "true",
                kebab(fields[1]),
                kebab(fields[2]),
                kebab(fields[3]),
            )
        })
        .collect::<Vec<_>>();
    let rust = DECISION_VECTORS
        .iter()
        .map(|&(d, s, a, p)| (d, s.to_owned(), a.to_owned(), p.to_owned()))
        .collect::<Vec<_>>();
    assert_eq!(rows, rust);
}

#[test]
fn the_storage_reader_names_each_declaration_class() {
    let function = serde_json::json!({
        "kind": "FunctionDecl",
        "inner": [
            {"kind": "ParmVarDecl", "id": "0x1", "name": "a"},
            {"kind": "CompoundStmt", "inner": [
                {"kind": "DeclStmt", "inner": [
                    {"kind": "VarDecl", "id": "0x2", "name": "b"},
                    {"kind": "VarDecl", "id": "0x3", "name": "c", "storageClass": "static"},
                    {"kind": "VarDecl", "id": "0x4", "name": "d", "storageClass": "extern"},
                    {"kind": "VarDecl", "id": "0x5", "name": "e", "storageClass": "register"},
                    {"kind": "VarDecl", "id": "0x6", "name": "f", "tls": "static"}
                ]}
            ]}
        ]
    });
    assert_eq!(root_storage(&function, "0x1"), RootStorage::Parameter);
    assert_eq!(root_storage(&function, "0x2"), RootStorage::Automatic);
    for decl in ["0x3", "0x4", "0x5", "0x6"] {
        assert_eq!(
            root_storage(&function, decl),
            RootStorage::Specified,
            "{decl}"
        );
    }
    assert_eq!(root_storage(&function, "0x9"), RootStorage::Outside);
}
