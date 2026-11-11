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

//! Card SLOT-MOVE-AND-INTERIOR-BORROW: the slot clauses (contract
//! specification §C04.15, §C04.16) and the borrowed result (§C04.3.4) of
//! resource vocabulary version 1, read total-or-reject (grammar G05).

use std::collections::BTreeMap;

use qcore_contract::contract_loader::{check_source_assumptions, load_resource_contract_sources};
use qcore_schemas::contract::{DerivedExtent, ResourceContracts, ResourceReturnMode};
use qcore_schemas::slot_move::{DerivedOut, SlotMove};

fn load(text: &str) -> Result<ResourceContracts, String> {
    load_resource_contract_sources(&BTreeMap::from([(
        "slots.qv".to_owned(),
        format!("module contract;\n{text}"),
    )]))
}

const SLOTS: &str = "
extern fn cat(slot: ptr<mut, ptr<mut, u8>>, text: ptr<const, u8>) -> unit
contract {
    requires resource_contract_v1;
    ensures normal resource_borrowed(slot);
    ensures normal resource_borrowed(text);
    ensures normal resource_slot_moved(slot, nullable);
    ensures normal resource_return_none();
};

extern fn parse(nptr: ptr<const, u8>, end: ptr<mut, ptr<mut, u8>>) -> i64
contract {
    requires resource_contract_v1;
    ensures normal resource_borrowed(nptr);
    ensures normal resource_borrowed(end);
    ensures normal resource_out_derived(end, nptr, nullable);
    ensures normal resource_return_none();
};

extern fn child(parent: ptr<const, u8>) -> ptr<mut, ptr<const, u8>>
contract {
    requires resource_contract_v1;
    ensures normal resource_borrowed(parent);
    ensures normal resource_return_borrowed_from(parent, nullable);
};
";

#[test]
fn slot_clauses_load_withheld_and_the_borrowed_result_loads_plainly() {
    let contracts = load(SLOTS).expect("loads");
    assert!(contracts.contract("cat").is_none());
    assert!(contracts.contract("parse").is_none());
    let cat = contracts.retaining_contract("cat").expect("withheld");
    assert_eq!(
        cat.slot_moves,
        [SlotMove {
            slot: 0,
            nullable: true
        }]
    );
    assert!(!cat.retains());
    let parse = contracts.retaining_contract("parse").expect("withheld");
    assert_eq!(
        parse.derived_outs,
        [DerivedOut {
            slot: 1,
            base: 0,
            nullable: true,
            const_dropped: true
        }]
    );
    let child = contracts.contract("child").expect("in the function map");
    assert_eq!(
        child.result,
        ResourceReturnMode::Derived {
            parameter: 0,
            extent: DerivedExtent::Borrowed,
            nullable: true,
            const_dropped: true,
        }
    );
    // The retained texts reload to the same declarations.
    check_source_assumptions(&contracts).expect("the source assumptions recheck");
}

fn rejects(text: &str, why: &str) {
    assert!(load(text).is_err(), "{why}: must reject\n{text}");
}

#[test]
fn every_misplaced_slot_clause_rejects_the_module() {
    let declaration = |parameters: &str, clauses: &str, result: &str| {
        format!(
            "extern fn f({parameters}) -> {result}\ncontract {{\n    requires resource_contract_v1;\n{clauses}}};\n"
        )
    };
    rejects(
        &declaration(
            "s: ptr<mut, u8>",
            "    ensures normal resource_borrowed(s);\n    ensures normal resource_slot_moved(s, nullable);\n    ensures normal resource_return_none();\n",
            "unit",
        ),
        "a moved slot of depth one",
    );
    rejects(
        &declaration(
            "s: ptr<mut, ptr<const, u8>>",
            "    ensures normal resource_borrowed(s);\n    ensures normal resource_slot_moved(s, nullable);\n    ensures normal resource_return_none();\n",
            "unit",
        ),
        "a moved slot to const",
    );
    rejects(
        &declaration(
            "s: ptr<mut, ptr<mut, u8>>",
            "    ensures normal resource_out(s, nullable);\n    ensures normal resource_slot_moved(s, nullable);\n    ensures normal resource_return_none();\n",
            "unit",
        ),
        "a moved slot that is not borrowed",
    );
    rejects(
        &declaration(
            "s: ptr<mut, ptr<mut, u8>>",
            "    ensures normal resource_borrowed(s);\n    ensures normal resource_slot_moved(s, maybe);\n    ensures normal resource_return_none();\n",
            "unit",
        ),
        "an unknown nullability word",
    );
    rejects(
        &declaration(
            "s: ptr<mut, ptr<mut, u8>>, p: ptr<const, u8>",
            "    ensures normal resource_borrowed(s);\n    ensures normal resource_borrowed(p);\n    ensures normal resource_slot_moved(s, nullable);\n    ensures normal resource_out_derived(s, p, nullable);\n    ensures normal resource_return_none();\n",
            "unit",
        ),
        "a slot named by two slot clauses",
    );
    rejects(
        &declaration(
            "s: ptr<mut, ptr<mut, u8>>",
            "    ensures normal resource_borrowed(s);\n    ensures normal resource_out_derived(s, s, nullable);\n    ensures normal resource_return_none();\n",
            "unit",
        ),
        "a derived out slot that is its own base",
    );
    rejects(
        &declaration(
            "s: ptr<mut, ptr<mut, u8>>, p: ptr<mut, ptr<mut, u8>>",
            "    ensures normal resource_borrowed(s);\n    ensures normal resource_borrowed(p);\n    ensures normal resource_out_derived(s, p, nullable);\n    ensures normal resource_return_none();\n",
            "unit",
        ),
        "a derived out slot over a base of depth two",
    );
    rejects(
        &declaration(
            "s: ptr<mut, ptr<mut, u8>>",
            "    ensures normal resource_on_return(eq, 0, resource_slot_moved(s, nullable));\n    ensures normal resource_on_return(ne, 0, resource_borrowed(s));\n    ensures normal resource_return_none();\n",
            "i32",
        ),
        "a slot move inside a return-code disposition (not admitted yet)",
    );
    rejects(
        &declaration(
            "s: ptr<mut, ptr<mut, u8>>",
            "    ensures normal resource_borrowed(s);\n    ensures normal resource_slot_moved(s, nullable);\n    ensures normal resource_variadic_data_tail();\n    ensures normal resource_return_none();\n",
            "unit",
        ),
        "a slot clause beside a variadic tail",
    );
    rejects(
        &declaration(
            "p: ptr<mut, ptr<mut, u8>>",
            "    ensures normal resource_borrowed(p);\n    ensures normal resource_return_borrowed_from(p, nullable);\n",
            "ptr<mut, u8>",
        ),
        "a borrowed result over a base of depth two",
    );
    rejects(
        &declaration(
            "p: ptr<mut, u8>",
            "    ensures normal resource_borrowed(p);\n    ensures normal resource_return_borrowed_from(p, nullable);\n",
            "i32",
        ),
        "a borrowed scalar result",
    );
    rejects(
        &declaration(
            "p: ptr<mut, u8>",
            "    ensures normal resource_owned(p, nonnull);\n    ensures normal resource_return_borrowed_from(p, nullable);\n",
            "ptr<mut, u8>",
        ),
        "a borrowed result over a custody-bearing base",
    );
}
