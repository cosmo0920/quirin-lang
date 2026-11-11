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

//! Card CG22-CALLBACK-REGISTRY-CONTRACT: the callback clauses (contract
//! specification §C04.19) of resource vocabulary version 1, read
//! total-or-reject (grammar G05).

use std::collections::BTreeMap;

use qcore_contract::contract_loader::{check_source_assumptions, load_resource_contract_sources};
use qcore_schemas::callback::{CallbackActual, CallbackClause, CallbackHolder};
use qcore_schemas::contract::ResourceContracts;

fn load(text: &str) -> Result<ResourceContracts, String> {
    load_resource_contract_sources(&BTreeMap::from([(
        "callbacks.qv".to_owned(),
        format!("module contract;\n{text}"),
    )]))
}

const CALLBACKS: &str = "
extern fn each(reader: ptr<mut, u8>, cb: fnptr, context: ptr<mut, u8>) -> i32
contract {
    requires resource_contract_v1;
    ensures normal resource_borrowed(reader);
    ensures normal resource_borrowed(cb);
    ensures normal resource_borrowed(context);
    ensures normal resource_callback_invoked(cb, reader, _, context);
    ensures normal resource_return_none();
};

extern fn dispatch(reader: ptr<mut, u8>, table: ptr<mut, u8>, context: ptr<mut, u8>) -> i32
contract {
    requires resource_contract_v1;
    ensures normal resource_borrowed(reader);
    ensures normal resource_borrowed(table);
    ensures normal resource_borrowed(context);
    ensures normal resource_callback_table(table, reader, _, context);
    ensures normal resource_return_none();
};

extern fn once(control: ptr<mut, u8>, routine: fnptr) -> i32
contract {
    requires resource_contract_v1;
    ensures normal resource_borrowed(control);
    ensures normal resource_borrowed(routine);
    ensures normal resource_callback_invoked(routine);
    ensures normal resource_return_none();
};

extern fn list_init(rl: ptr<mut, u8>, size: i32, free_cb: fnptr) -> unit
contract {
    requires resource_contract_v1;
    ensures normal resource_borrowed(rl);
    ensures normal resource_borrowed(size);
    ensures normal resource_borrowed(free_cb);
    ensures normal resource_callback_registered(free_cb, rl);
    ensures normal resource_return_none();
};

extern fn list_new(size: i32, free_cb: fnptr) -> ptr<mut, u8>
contract {
    requires resource_contract_v1;
    ensures normal resource_borrowed(size);
    ensures normal resource_borrowed(free_cb);
    ensures normal resource_callback_registered(free_cb, result);
    ensures normal resource_return_owned(nonnull);
};

extern fn list_add(rl: ptr<mut, u8>, elem: ptr<mut, u8>) -> unit
contract {
    requires resource_contract_v1;
    ensures normal resource_borrowed(rl);
    ensures normal resource_borrowed(elem);
    ensures normal resource_element_added(elem, rl);
    ensures normal resource_return_none();
};

extern fn list_destroy(rl: ptr<mut, u8>) -> unit
contract {
    requires resource_contract_v1;
    ensures normal resource_borrowed(rl);
    ensures normal resource_callback_registry_invoked(rl, element);
    ensures normal resource_elements_dropped(rl);
    ensures normal resource_callback_unregistered(rl);
    ensures normal resource_released_if_constructed(rl);
    ensures normal resource_return_none();
};

extern fn packer_init(pk: ptr<mut, u8>, data: ptr<mut, u8>, callback: fnptr) -> unit
contract {
    requires resource_contract_v1;
    ensures normal resource_borrowed(pk);
    ensures normal resource_borrowed(data);
    ensures normal resource_borrowed(callback);
    ensures normal resource_retained_by(data, pk);
    ensures normal resource_callback_registered(callback, pk);
    ensures normal resource_return_none();
};
";

#[test]
fn callback_clauses_load_withheld() {
    let contracts = load(CALLBACKS).expect("loads");
    for name in [
        "each",
        "dispatch",
        "once",
        "list_init",
        "list_new",
        "list_add",
        "list_destroy",
        "packer_init",
    ] {
        assert!(contracts.contract(name).is_none(), "{name} is withheld");
        assert!(
            contracts.retaining_contract(name).is_some(),
            "{name} is kept"
        );
    }
    let each = contracts.retaining_contract("each").expect("withheld");
    assert_eq!(
        each.callbacks,
        [CallbackClause::Invoked {
            callback: 1,
            actuals: vec![
                CallbackActual::Parameter { index: 0 },
                CallbackActual::Supplied,
                CallbackActual::Parameter { index: 2 },
            ],
        }]
    );
    assert!(!each.retains());
    let once = contracts.retaining_contract("once").expect("withheld");
    assert_eq!(
        once.callbacks,
        [CallbackClause::Invoked {
            callback: 1,
            actuals: vec![],
        }]
    );
    let new = contracts.retaining_contract("list_new").expect("withheld");
    assert_eq!(
        new.callbacks,
        [CallbackClause::Registered {
            callback: 1,
            holder: CallbackHolder::Result,
        }]
    );
    let destroy = contracts
        .retaining_contract("list_destroy")
        .expect("withheld");
    assert_eq!(destroy.callbacks.len(), 4);
    assert_eq!(
        destroy.callbacks[0],
        CallbackClause::RegistryInvoked {
            holder: 0,
            actuals: vec![CallbackActual::Element],
        }
    );
    let packer = contracts
        .retaining_contract("packer_init")
        .expect("withheld");
    assert!(packer.retains());
    assert_eq!(packer.callbacks.len(), 1);
    check_source_assumptions(&contracts).expect("the retained texts reload");
}

fn declaration(parameters: &str, result: &str, clauses: &str) -> String {
    format!(
        "extern fn k({parameters}) -> {result}\ncontract {{\n    requires resource_contract_v1;\n{clauses}}};\n"
    )
}

#[test]
fn every_misplaced_callback_clause_rejects_the_module() {
    let cases = [
        // The invoked parameter is not a function pointer.
        (
            "p: ptr<mut, u8>",
            "unit",
            "    ensures normal resource_borrowed(p);\n    ensures normal resource_callback_invoked(p);\n    ensures normal resource_return_none();\n",
        ),
        // A callback as its own actual.
        (
            "f: fnptr",
            "unit",
            "    ensures normal resource_borrowed(f);\n    ensures normal resource_callback_invoked(f, f);\n    ensures normal resource_return_none();\n",
        ),
        // `element` outside a registry invocation.
        (
            "f: fnptr",
            "unit",
            "    ensures normal resource_borrowed(f);\n    ensures normal resource_callback_invoked(f, element);\n    ensures normal resource_return_none();\n",
        ),
        // An unknown actual.
        (
            "f: fnptr",
            "unit",
            "    ensures normal resource_borrowed(f);\n    ensures normal resource_callback_invoked(f, q);\n    ensures normal resource_return_none();\n",
        ),
        // A callback invoked and registered.
        (
            "f: fnptr, h: ptr<mut, u8>",
            "unit",
            "    ensures normal resource_borrowed(f);\n    ensures normal resource_borrowed(h);\n    ensures normal resource_callback_invoked(f);\n    ensures normal resource_callback_registered(f, h);\n    ensures normal resource_return_none();\n",
        ),
        // A table that is not a depth-one pointer.
        (
            "t: ptr<mut, ptr<mut, u8>>",
            "unit",
            "    ensures normal resource_borrowed(t);\n    ensures normal resource_callback_table(t);\n    ensures normal resource_return_none();\n",
        ),
        // An owned holder.
        (
            "f: fnptr, h: ptr<mut, u8>",
            "unit",
            "    ensures normal resource_borrowed(f);\n    ensures normal resource_owned(h, nonnull);\n    ensures normal resource_callback_registered(f, h);\n    ensures normal resource_return_none();\n",
        ),
        // A `result` holder without an owned non-null result.
        (
            "f: fnptr",
            "ptr<mut, u8>",
            "    ensures normal resource_borrowed(f);\n    ensures normal resource_callback_registered(f, result);\n    ensures normal resource_return_owned(nullable);\n",
        ),
        // A holder that is its own element.
        (
            "h: ptr<mut, u8>",
            "unit",
            "    ensures normal resource_borrowed(h);\n    ensures normal resource_element_added(h, h);\n    ensures normal resource_return_none();\n",
        ),
        // A duplicate registry clause.
        (
            "h: ptr<mut, u8>",
            "unit",
            "    ensures normal resource_borrowed(h);\n    ensures normal resource_elements_dropped(h);\n    ensures normal resource_elements_dropped(h);\n    ensures normal resource_return_none();\n",
        ),
        // A registration and a registry use on one holder.
        (
            "f: fnptr, h: ptr<mut, u8>",
            "unit",
            "    ensures normal resource_borrowed(f);\n    ensures normal resource_borrowed(h);\n    ensures normal resource_callback_registered(f, h);\n    ensures normal resource_callback_unregistered(h);\n    ensures normal resource_return_none();\n",
        ),
        // A retained element.
        (
            "e: ptr<mut, u8>, h: ptr<mut, u8>",
            "unit",
            "    ensures normal resource_borrowed(e);\n    ensures normal resource_borrowed(h);\n    ensures normal resource_retained_by(e, h);\n    ensures normal resource_element_added(e, h);\n    ensures normal resource_return_none();\n",
        ),
        // Beside a reference clause.
        (
            "f: fnptr, p: ptr<mut, u8>",
            "unit",
            "    ensures normal resource_borrowed(f);\n    ensures normal resource_borrowed(p);\n    ensures normal resource_reference_kept(p);\n    ensures normal resource_callback_invoked(f, p);\n    ensures normal resource_return_none();\n",
        ),
        // Beside a variadic tail.
        (
            "f: fnptr",
            "unit",
            "    ensures normal resource_borrowed(f);\n    ensures normal resource_callback_invoked(f);\n    ensures normal resource_variadic_data_tail();\n    ensures normal resource_return_none();\n",
        ),
        // A parameter spelled like an actual word.
        (
            "f: fnptr, element: ptr<mut, u8>",
            "unit",
            "    ensures normal resource_borrowed(f);\n    ensures normal resource_borrowed(element);\n    ensures normal resource_callback_invoked(f, element);\n    ensures normal resource_return_none();\n",
        ),
        // A malformed registration.
        (
            "f: fnptr",
            "unit",
            "    ensures normal resource_borrowed(f);\n    ensures normal resource_callback_registered(f);\n    ensures normal resource_return_none();\n",
        ),
    ];
    for (parameters, result, clauses) in cases {
        let text = declaration(parameters, result, clauses);
        assert!(load(&text).is_err(), "must reject:\n{text}");
    }
}
