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

//! Card W43-ABI-ENUM-AND-CONST-PP (gateway rules 12u and 12v): the declared-ABI
//! reader's two widenings, through the same census the analysis applies.
//!
//! * 12u: an enumeration parameter or result is its underlying integer type,
//!   which Clang states for the analysed unit; anything Clang does not state
//!   exactly refuses, and `int` is never assumed.
//! * 12v: a pointer to a pointer to const (`const char **`) is
//!   `ptr<mut, ptr<const, u8>>`; a qualifier on the slot still refuses.

use std::collections::BTreeMap;

use qcore_c_gateway::legacy_c::{AbiOutcome, AbiRefusal, declared_abi_census};
use qcore_contract::contract_loader::load_resource_contract_sources;
use qcore_schemas::contract::ResourceContracts;

fn contracts(signature: &str, parameters: &[&str], result: &str) -> ResourceContracts {
    let mut clauses = String::new();
    for parameter in parameters {
        clauses.push_str(&format!(
            "    ensures normal resource_borrowed({parameter});\n"
        ));
    }
    let module = format!(
        "module contract;\n\nextern fn {signature}\ncontract {{\n    requires resource_contract_v1;\n{clauses}    ensures normal {result};\n}};\n"
    );
    load_resource_contract_sources(&BTreeMap::from([("k.qcore".to_owned(), module)]))
        .expect("the fixture module loads")
}

fn outcome(c: &str, signature: &str, parameters: &[&str], result: &str) -> AbiOutcome {
    let source = format!("{c}\nvoid w(void) {{ }}\n");
    let rows = declared_abi_census(&source, &contracts(signature, parameters, result));
    assert_eq!(rows.len(), 1, "{rows:?}");
    rows[0].outcome
}

fn clang_available() -> bool {
    if qcore_c_gateway::clang_command().is_err() {
        eprintln!("skipping: no Clang");
        return false;
    }
    true
}

const NONE: &str = "resource_return_none()";

#[test]
fn an_enumeration_without_a_negative_value_is_unsigned_int() {
    if !clang_available() {
        return;
    }
    let c = "typedef enum { A, B } e_t;\nint k(e_t v);";
    let (host, other) = if cfg!(windows) {
        ("i32", "u32")
    } else {
        ("u32", "i32")
    };
    assert_eq!(
        outcome(c, &format!("k(v: {host}) -> i32"), &["v"], NONE),
        AbiOutcome::Agreed
    );
    // The opposite signedness is never admitted: Clang's target answer wins.
    assert_eq!(
        outcome(c, &format!("k(v: {other}) -> i32"), &["v"], NONE),
        AbiOutcome::Withdrawn(AbiRefusal::ParameterTypeDisagreement)
    );
}

#[test]
fn an_enumeration_with_a_negative_value_is_int_tagged_or_through_a_chain() {
    if !clang_available() {
        return;
    }
    let tagged = "enum err { E_FAIL = -1, E_OK = 0 };\nenum err k(enum err v);";
    assert_eq!(
        outcome(tagged, "k(v: i32) -> i32", &["v"], NONE),
        AbiOutcome::Agreed
    );
    let chain = "typedef enum err { E_FAIL = -1, E_OK } err_t;\ntypedef err_t my_err;\nmy_err k(const my_err v);";
    assert_eq!(
        outcome(chain, "k(v: i32) -> i32", &["v"], NONE),
        AbiOutcome::Agreed
    );
    assert_eq!(
        outcome(chain, "k(v: u32) -> i32", &["v"], NONE),
        AbiOutcome::Withdrawn(AbiRefusal::ParameterTypeDisagreement)
    );
    assert_eq!(
        outcome(chain, "k(v: i32) -> u32", &["v"], NONE),
        AbiOutcome::Withdrawn(AbiRefusal::ReturnTypeDisagreement)
    );
}

#[test]
fn a_wide_or_fixed_enumeration_is_its_own_width() {
    if !clang_available() {
        return;
    }
    let wide = "enum big { BIG = 0x100000000 };\nint k(enum big v);";
    let host = if cfg!(windows) { "i32" } else { "u64" };
    assert_eq!(
        outcome(wide, &format!("k(v: {host}) -> i32"), &["v"], NONE),
        AbiOutcome::Agreed
    );
    // The pointer-width reading a `long` also admits is not the exact one.
    assert_eq!(
        outcome(wide, "k(v: usize) -> i32", &["v"], NONE),
        AbiOutcome::Withdrawn(AbiRefusal::ParameterTypeDisagreement)
    );
}

#[test]
fn an_enumeration_clang_cannot_state_refuses() {
    if !clang_available() {
        return;
    }
    // An incomplete enumeration has no integer type the probe can read.
    let incomplete = "enum later;\nint k(enum later v);";
    assert_eq!(
        outcome(incomplete, "k(v: u32) -> i32", &["v"], NONE),
        AbiOutcome::Withdrawn(if cfg!(windows) {
            AbiRefusal::ParameterTypeDisagreement
        } else {
            AbiRefusal::UnreadableDeclaration
        })
    );
    // A source that spells the probe prefix could forge an answer: refused.
    let forged = "typedef enum { A } e_t;\nint __qcore_abi_enum_probe_0;\nint k(e_t v);";
    assert_eq!(
        outcome(forged, "k(v: u32) -> i32", &["v"], NONE),
        AbiOutcome::Withdrawn(AbiRefusal::UnreadableDeclaration)
    );
}

#[test]
fn a_pointer_to_an_enumeration_is_an_object_pointer() {
    if !clang_available() {
        return;
    }
    let c = "typedef enum { A, B } e_t;\nint k(const e_t *v);";
    assert_eq!(
        outcome(c, "k(v: ptr<const, u8>) -> i32", &["v"], NONE),
        AbiOutcome::Agreed
    );
}

#[test]
fn a_pointer_to_a_pointer_to_const_has_its_own_spelling() {
    let direct = "int k(const char **v);";
    assert_eq!(
        outcome(
            direct,
            "k(v: ptr<mut, ptr<const, u8>>) -> i32",
            &["v"],
            NONE
        ),
        AbiOutcome::Agreed
    );
    assert_eq!(
        outcome(direct, "k(v: ptr<mut, ptr<mut, u8>>) -> i32", &["v"], NONE),
        AbiOutcome::Withdrawn(AbiRefusal::ParameterTypeDisagreement)
    );
    let unqualified = "int k(char **v);";
    assert_eq!(
        outcome(
            unqualified,
            "k(v: ptr<mut, ptr<const, u8>>) -> i32",
            &["v"],
            NONE
        ),
        AbiOutcome::Withdrawn(AbiRefusal::ParameterTypeDisagreement)
    );
    let void = "int k(const void **v);";
    assert_eq!(
        outcome(void, "k(v: ptr<mut, ptr<const, u8>>) -> i32", &["v"], NONE),
        AbiOutcome::Agreed
    );
    // An array of pointers to const is adjusted to the same type.
    let array = "int k(const char *v[]);";
    assert_eq!(
        outcome(array, "k(v: ptr<mut, ptr<const, u8>>) -> i32", &["v"], NONE),
        AbiOutcome::Agreed
    );
    // Through a typedef whose const is the innermost pointee's.
    let chain = "typedef const char *cstr;\nint k(cstr *v);";
    assert_eq!(
        outcome(chain, "k(v: ptr<mut, ptr<const, u8>>) -> i32", &["v"], NONE),
        AbiOutcome::Agreed
    );
    // A result.
    let result = "const char **k(int v);";
    assert_eq!(
        outcome(
            result,
            "k(v: i32) -> ptr<mut, ptr<const, u8>>",
            &["v"],
            "resource_return_owned(nullable)"
        ),
        AbiOutcome::Agreed
    );
}

#[test]
fn a_qualifier_on_the_slot_still_refuses() {
    for c in [
        // Between the two `*`.
        "int k(char *const *v);",
        "int k(const char *const *v);",
        // On a typedef name that stands for a pointer: `char *const *`.
        "typedef char *str;\nint k(const str *v);",
    ] {
        assert_eq!(
            outcome(c, "k(v: ptr<mut, ptr<const, u8>>) -> i32", &["v"], NONE),
            AbiOutcome::Withdrawn(AbiRefusal::UnreadableDeclaration),
            "{c}"
        );
    }
    // A chain link qualifying a pointer: never the pointer-to-const reading.
    let link = "typedef const char *cstr;\ntypedef const cstr ccstr;\nint k(ccstr *v);";
    assert_ne!(
        outcome(link, "k(v: ptr<mut, ptr<const, u8>>) -> i32", &["v"], NONE),
        AbiOutcome::Agreed
    );
}
