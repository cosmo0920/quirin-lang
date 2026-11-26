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

//! Card CONNECT-ADOPT-DECLARED-DEPENDENCIES: the `linkage_contract_v1` forms
//! of grammar G05l parse unambiguously beside every existing `quant` and `#`
//! form, and every misplacement is refused by name.

use qcore_frontend::contract_inclusion::scan_contract_inclusion;
use qcore_frontend::frontend::{SourceContractClauseKind, parse_source_program};

fn module(contract: &str) -> String {
    format!(
        "module contract;\n\nextern fn acme_new(size: usize) -> ptr<mut, u8>\ncontract {{\n{contract}\n}};\n"
    )
}

fn clauses(contract: &str) -> Vec<(SourceContractClauseKind, String)> {
    let program = parse_source_program(&module(contract)).expect("parses");
    program.declarations[0]
        .value
        .function_contract
        .as_ref()
        .unwrap()
        .clauses
        .iter()
        .map(|clause| {
            (
                clause.value.kind.clone(),
                clause.value.expression_source.clone(),
            )
        })
        .collect()
}

fn refused(contract: &str) -> String {
    parse_source_program(&module(contract))
        .expect_err("refused")
        .message
}

#[test]
fn the_users_example_shape_parses_to_flat_clauses() {
    // The user's shape: a `premises` block holding a `link` block beside the
    // other premises (here an existing `quant live` result-state form).
    let read = clauses(
        "    requires resource_contract_v1;\n    requires linkage_contract_v1;\n    premises {\n        \
         link { quant #link libc::malloc; quant #link libc::free; };\n        \
         quant live #result |-> #fresh;\n    };\n    ensures normal resource_borrowed(size);",
    );
    let links: Vec<&str> = read
        .iter()
        .filter(|(kind, _)| *kind == SourceContractClauseKind::QuantLink)
        .map(|(_, expression)| expression.as_str())
        .collect();
    assert_eq!(links, ["libc::malloc", "libc::free"]);
    assert_eq!(read.len(), 6, "the blocks group; they add no clause");
    assert!(matches!(read[4].0, SourceContractClauseKind::QuantLive));
}

#[test]
fn a_single_link_clause_is_one_link() {
    let read = clauses("    requires linkage_contract_v1;\n    quant #link libevent::mk_list_del;");
    assert_eq!(
        read[1],
        (
            SourceContractClauseKind::QuantLink,
            "libevent::mk_list_del".into()
        )
    );
    // A bare name (made nameable by a module-level import) parses too.
    let bare = clauses("    requires linkage_contract_v1;\n    quant #link free;");
    assert_eq!(
        bare[1],
        (SourceContractClauseKind::QuantLink, "free".into())
    );
}

#[test]
fn misplaced_or_malformed_linkage_forms_are_refused() {
    // `link` outside `premises`.
    assert!(refused("    link { quant #link libc::free; };").starts_with("linkage-link-placement"));
    // Nested `premises`.
    assert!(
        refused("    premises { premises { quant #link libc::free; }; };")
            .starts_with("linkage-premises-placement")
    );
    // A `link` block holds only links.
    assert!(
        refused("    premises { link { requires resource_contract_v1; }; };")
            .starts_with("linkage-link-entry")
    );
    // An empty block.
    assert!(refused("    premises { };").starts_with("linkage-block"));
    // The terminator: a block ends `};`.
    assert!(refused("    premises { quant #link libc::free; }").starts_with("linkage-block"));
    // A trailing `.` is no terminator.
    assert!(!refused("    quant #link libc::free.").is_empty());
    // Two targets in one entry, or a nested qualifier.
    assert!(
        refused("    quant #link libc::free, libc::malloc;").starts_with("linkage-link-target")
    );
    assert!(refused("    quant #link a::b::c;").starts_with("linkage-link-target"));
    // `#link` is reserved at the start of a clause: no condition is named so.
    assert!(refused("    quant #link = (#p.q != 0);").starts_with("linkage-reserved"));
    assert!(
        refused("    quant #link in { requires resource_contract_v1; } : [[local]];")
            .starts_with("linkage-reserved")
    );
    // No link inside a `[[local]]` block.
    assert!(
        refused(
            "    quant #c = (#size.x != 0);\n    quant #c in { quant #link libc::free; } : [[local]];"
        )
        .starts_with("local-condition-clause")
    );
}

#[test]
fn link_as_a_binder_inside_a_formula_is_unaffected() {
    // `#link` and the sort `link` inside a proposition stay formula syntax.
    let read = clauses(
        "    quant proposition #p(#owner: function<link, object>, #f: predicate<link>) = \
         forall (#link: link) { #f(#link) };",
    );
    assert!(matches!(
        read[0].0,
        SourceContractClauseKind::QuantProposition
    ));
}

#[test]
fn a_module_level_link_import_is_a_directive() {
    let text = "module contract;\n#include libc::free as quant #link;\n\n\
        extern fn acme_free(p: ptr<mut, u8>) -> unit contract {\n    requires resource_contract_v1;\n    \
        requires linkage_contract_v1;\n    quant #link free;\n};\n";
    let syntax = scan_contract_inclusion(text).unwrap();
    assert_eq!(syntax.link_imports.len(), 1);
    assert_eq!(syntax.link_imports[0].namespace, "libc");
    assert_eq!(syntax.link_imports[0].function, "free");
    assert_eq!(syntax.bare_links.len(), 1);
    assert_eq!(syntax.bare_links[0].name, "free");
    for bad in [
        "module contract;\n#include libc::free as #link;\n",
        "module contract;\n#include libc::free;\n",
        "module contract;\n#include libc::free as quant #outer;\n",
    ] {
        assert!(scan_contract_inclusion(bad).is_err(), "{bad}");
    }
}
