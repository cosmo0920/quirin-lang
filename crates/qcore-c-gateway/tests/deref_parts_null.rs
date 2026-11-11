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

//! Card NULL-DEREF-QTT-GROUNDING (USER DECISION 2026-10-11, "Two levels"):
//! the null-dereference property reads only canonical parts, so a declaration
//! written as the shorthand `quant deref #B;`, as its decomposition, or as a
//! mix gives byte-identical null rows (verdict, checks, clause standing,
//! dependencies) on a real Clang AST, and the existence part alone
//! (`quant null-deref #B;`) gives the same null rows as both.

use std::collections::BTreeMap;

use qcore_c_gateway::legacy_c::C11TranslationUnit;
use qcore_c_gateway::null_deref::{NullFunctionRow, analyze_null_c11};
use qcore_contract::bounds_contract::load_bounds_contract_sources;
use qcore_schemas::contract::ResourceContracts;

const SOURCE: &str = "\
struct S { int a; struct S *next; };
int take(struct S *s);
int tested(struct S *s) { if (!s) return 0; return s->a; }
int take(struct S *s) { return s->a + (s->next != 0); }
int caller(struct S *s) { if (!s) return 0; return take(s) + take(s->next); }
";

fn clang_available() -> bool {
    match qcore_c_gateway::clang_command() {
        Ok(_) => true,
        Err(reason) => {
            assert!(
                std::env::var_os("QCORE_CLANG").is_none(),
                "explicit QCORE_CLANG configuration is invalid: {reason}"
            );
            eprintln!("skipping Clang-dependent control: {reason}");
            false
        }
    }
}

fn rows(parts: &str) -> Vec<NullFunctionRow> {
    let module = format!(
        "module contract;
extern fn take(s: ptr<mut, u8>) -> i32
contract {{
    requires bounds_contract_v1;
{parts}
}};
extern fn tested(s: ptr<mut, u8>) -> i32
contract {{
    requires bounds_contract_v1;
{tested}
}};
",
        tested = parts
    );
    let bounds = load_bounds_contract_sources(&BTreeMap::from([("parts.qv".to_owned(), module)]))
        .expect("the fixture bounds contracts load");
    let unit = C11TranslationUnit {
        source: SOURCE,
        ..Default::default()
    };
    let functions = ["tested", "take", "caller"]
        .iter()
        .map(|f| (*f).to_owned())
        .collect::<Vec<_>>();
    analyze_null_c11(&unit, &functions, &ResourceContracts::default(), &bounds)
        .unwrap_or_else(|error| panic!("fixture rejected: {error}"))
        .functions
}

fn rendered(rows: &[NullFunctionRow]) -> String {
    serde_json::to_string(rows).expect("rows serialize")
}

#[test]
fn the_shorthand_and_its_decomposition_give_identical_null_rows() {
    if !clang_available() {
        return;
    }
    let shorthand = rows("    quant deref #s;");
    let forms = [
        "    quant null-deref #s;
    quant checks s, Idx #i: #i == 0;
    quant live #s;",
        "    quant live #s;
    quant deref #s;",
        "    quant checks s, Idx #i: #i == 0;
    quant null-deref #s;
    quant deref #s;",
        // The existence part alone: the null property reads nothing else.
        "    quant null-deref #s;",
    ];
    for form in forms {
        assert_eq!(rendered(&rows(form)), rendered(&shorthand), "`{form}`");
    }
    // The standing is the existence part's, on canonical parts: `tested`
    // tests first (stronger than necessary), `take` dereferences first.
    let standing = |row: &NullFunctionRow| {
        row.clause_standing
            .iter()
            .map(|s| s.standing)
            .collect::<Vec<_>>()
    };
    assert_eq!(standing(&shorthand[0]), vec!["stronger-than-necessary"]);
    assert_eq!(standing(&shorthand[1]), vec!["necessary"]);
    // The caller's second actual is possibly null: a candidate, whichever
    // form declared the clause.
    assert_eq!(shorthand[2].verdict, "Unknown", "{shorthand:#?}");
}
