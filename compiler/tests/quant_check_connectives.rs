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

//! Card CHUNKIO-META-LEN-BOUNDS-CONTRACTS: the connectives `&&` and `||` of a
//! `quant checks P, Idx #i: PREDICATE;` clause (G05c) are the predicate's own,
//! read literally by the bounds reader, so the frontend's operator gate
//! (`Q038`) admits them there and nowhere else in a contract module. Before
//! the card a tracked module carrying a quant check (chunkio's
//! `meta_len_bounds.qv`) failed the contract corpus's compiler regression on
//! its `&&`; QUANT-INDEX-CHECKS' probe declarations were never compiled.

use qcore_compiler::{CompileOptions, SourceFile, compile_files};

fn module(clause: &str) -> String {
    format!(
        "module contract;
extern fn get_meta_len(map: ptr<mut, u8>) -> u16
contract {{
    requires bounds_contract_v1;
{clause}
}};
"
    )
}

fn codes(text: &str) -> Vec<String> {
    match compile_files(
        &[SourceFile::new("bounds.qv", text)],
        &CompileOptions::default(),
    ) {
        Ok(_) => Vec::new(),
        Err(diagnostics) => diagnostics.into_iter().map(|d| d.code).collect(),
    }
}

#[test]
fn a_quant_checks_predicate_keeps_its_connectives() {
    for clause in [
        "    quant checks map, Idx #i: 22 <= #i && #i < 24;",
        "    quant checks map, Idx #i: #i == 22 || #i == 23;",
        "    quant checks map, Idx #i: (0 <= #i && #i < 4) || (22 <= #i && #i < 24);",
    ] {
        assert_eq!(codes(&module(clause)), Vec::<String>::new(), "{clause}");
    }
}

#[test]
fn a_connective_anywhere_else_in_a_contract_still_refuses() {
    for clause in [
        "    requires bounds_extent(map, 24, elements) && bounds_index(map, 24);",
        "    ensures normal bounds_frame_reads(map) || bounds_frame_writes(map);",
    ] {
        assert_eq!(codes(&module(clause)), ["Q038"], "{clause}");
    }
    // A single bar or ampersand inside a quant check is still no connective.
    assert!(
        codes(&module("    quant checks map, Idx #i: 22 <= #i & #i < 24;"))
            .contains(&"Q038".to_owned())
    );
}

#[test]
fn typed_quant_formulas_own_only_their_validated_connectives() {
    for clause in [
        "quant proposition #p(#x: integer) = ((#x == 0) || (#x == 1));",
        "quant condition entry #p(#x: integer) = ((#x == 0) || (#x == 1));",
        "quant condition normal #p(#x: integer) = ((#x == 0) && (#x == 0));",
        "quant effect normal #e(#b: state, #a: state, #x: integer) from #b to #a when (#x == 0) reads () writes () frame (#x == 0) = ((#x == 0) && (#x == 0));",
    ] {
        assert!(codes(&module(clause)).is_empty(), "{clause}");
    }
    for clause in [
        "quant proposition #p(#x: integer) = ((#x == 0) || (#missing == 1));",
        "quant condition entry #p(#x: integer) = ((#x == 0) & (#x == 1));",
        "quant condition normal #p(#x: integer) = ((#x == 0) || 7);",
    ] {
        assert!(!codes(&module(clause)).is_empty(), "{clause}");
    }
}
