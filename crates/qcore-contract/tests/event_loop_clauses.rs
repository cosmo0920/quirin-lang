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

//! Card FB-EVENT-LOOP-RETENTION-AND-REGISTRATION: the holder path, the
//! removal of one registration and the registration forwarded through an id
//! (contract specification §C04.14, §C04.19.3, grammar G05; USER DECISIONS
//! 2026-10-16), read total-or-reject, and the two provider modules that use
//! them.

use std::collections::BTreeMap;

use qcore_contract::contract_loader::{check_source_assumptions, load_resource_contract_sources};
use qcore_schemas::callback::{CallbackClause, CallbackMember, IdLink};
use qcore_schemas::contract::ResourceContracts;
use qcore_schemas::retained_by::{PathRetention, Removal};

fn load(text: &str) -> Result<ResourceContracts, String> {
    load_resource_contract_sources(&BTreeMap::from([(
        "event_loop.qv".to_owned(),
        format!("module contract;\n{text}"),
    )]))
}

const DECLARED: &str = "
extern fn start(coll: ptr<mut, u8>, config: ptr<mut, u8>) -> i32
contract {
    requires resource_contract_v1;
    ensures normal resource_borrowed(coll);
    ensures normal resource_borrowed(config);
    ensures normal resource_retained_by(coll, coll.evl);
    ensures normal resource_return_none();
};

extern fn del(loop: ptr<mut, u8>, event: ptr<mut, u8>) -> i32
contract {
    requires resource_contract_v1;
    ensures normal resource_borrowed(loop);
    ensures normal resource_borrowed(event);
    quant live #loop: #holds(event) |-> #absent;
    ensures normal resource_return_none();
};

extern fn set(ins: ptr<mut, u8>, cb: fnptr, seconds: i64, config: ptr<mut, u8>) -> i32
contract {
    requires resource_contract_v1;
    ensures normal resource_borrowed(ins);
    ensures normal resource_borrowed(cb);
    ensures normal resource_borrowed(seconds);
    ensures normal resource_borrowed(config);
    ensures normal resource_callback_member_registered(#id, coll_s.cb_collect, cb) where #id = #result in ins.collectors;
    ensures normal resource_return_none();
};
";

#[test]
fn the_three_forms_load_and_are_withheld() {
    let contracts = load(DECLARED).expect("loads");
    for name in ["start", "del", "set"] {
        assert!(contracts.contract(name).is_none(), "{name} withheld");
    }
    let start = contracts.retaining_contract("start").expect("start");
    assert!(start.retentions.is_empty());
    assert_eq!(
        start.path_retentions,
        [PathRetention {
            retained: 0,
            root: 0,
            fields: vec!["evl".into()],
        }]
    );
    let del = contracts.retaining_contract("del").expect("del");
    assert_eq!(
        del.removals,
        [Removal {
            retained: 1,
            holder: 0,
        }]
    );
    assert!(del.retains());
    let set = contracts.retaining_contract("set").expect("set");
    assert_eq!(
        set.callbacks,
        [CallbackClause::MemberRegistered {
            object: 0,
            member: CallbackMember {
                tag: "coll_s".into(),
                path: "cb_collect".into(),
            },
            callback: 1,
            by_id: Some(IdLink {
                binder: "id".into(),
                list: vec!["collectors".into()],
            }),
        }]
    );
    check_source_assumptions(&contracts).expect("the retained texts reload to the same tables");
}

fn declaration(clauses: &[&str], parameters: &str, result: &str) -> String {
    format!(
        "extern fn f({parameters}) -> {result}\ncontract {{\n    requires resource_contract_v1;\n{}\n}};\n",
        clauses
            .iter()
            .map(|clause| format!("    {clause};"))
            .collect::<Vec<_>>()
            .join("\n")
    )
}

#[test]
fn every_malformed_form_rejects_the_whole_module() {
    let pointers = "a: ptr<mut, u8>, b: ptr<mut, u8>";
    let borrowed = [
        "ensures normal resource_borrowed(a)",
        "ensures normal resource_borrowed(b)",
    ];
    let with = |extra: &[&str]| {
        let mut clauses = borrowed.to_vec();
        clauses.extend_from_slice(extra);
        clauses.push("ensures normal resource_return_none()");
        clauses.iter().map(|c| (*c).to_owned()).collect::<Vec<_>>()
    };
    let cases: Vec<(Vec<String>, &str, &str)> = vec![
        // A holder path starts at a parameter: a record tag there rejects.
        (with(&["ensures normal resource_retained_by(a, coll_s.evl)"]), pointers, "i32"),
        // No guard on a holder path.
        (with(&["ensures normal resource_retained_by(a, b.evl) when n == 1"]), pointers, "i32"),
        // A holder path's formal is a borrowed pointer.
        (
            vec![
                "ensures normal resource_borrowed(a)".into(),
                "ensures normal resource_released(b, nonnull)".into(),
                "ensures normal resource_retained_by(a, b.evl)".into(),
                "ensures normal resource_return_none()".into(),
            ],
            pointers,
            "i32",
        ),
        // A removal's shape: `#H: #holds(P) |-> #absent`, exactly.
        (with(&["quant live #a: #holds(b) |-> #moved"]), pointers, "i32"),
        (with(&["quant live #a: #holds(b) -> #absent"]), pointers, "i32"),
        (with(&["quant live #a: #holds(b, a) |-> #absent"]), pointers, "i32"),
        // A holder does not hold itself; a pair is removed once.
        (with(&["quant live #a: #holds(a) |-> #absent"]), pointers, "i32"),
        (
            with(&[
                "quant live #a: #holds(b) |-> #absent",
                "quant live #a: #holds(b) |-> #absent",
            ]),
            pointers,
            "i32",
        ),
        // A declaration retains a pair or removes it, not both.
        (
            with(&[
                "ensures normal resource_retained_by(b, a)",
                "quant live #a: #holds(b) |-> #absent",
            ]),
            pointers,
            "i32",
        ),
        // The id is bound by its own `where` clause, to `#result`, in a path.
        (
            vec![
                "ensures normal resource_borrowed(a)".into(),
                "ensures normal resource_borrowed(cb)".into(),
                "ensures normal resource_callback_member_registered(#id, t.cb, cb) where #other = #result in a.list".into(),
                "ensures normal resource_return_none()".into(),
            ],
            "a: ptr<mut, u8>, cb: fnptr",
            "i32",
        ),
        (
            vec![
                "ensures normal resource_borrowed(a)".into(),
                "ensures normal resource_borrowed(cb)".into(),
                "ensures normal resource_callback_member_registered(#id, t.cb, cb) where #id = #value in a.list".into(),
                "ensures normal resource_return_none()".into(),
            ],
            "a: ptr<mut, u8>, cb: fnptr",
            "i32",
        ),
        (
            vec![
                "ensures normal resource_borrowed(a)".into(),
                "ensures normal resource_borrowed(cb)".into(),
                "ensures normal resource_callback_member_registered(#id, t.cb, cb) where #id = #result in a".into(),
                "ensures normal resource_return_none()".into(),
            ],
            "a: ptr<mut, u8>, cb: fnptr",
            "i32",
        ),
        // `#result` is an integer result.
        (
            vec![
                "ensures normal resource_borrowed(a)".into(),
                "ensures normal resource_borrowed(cb)".into(),
                "ensures normal resource_callback_member_registered(#id, t.cb, cb) where #id = #result in a.list".into(),
                "ensures normal resource_return_none()".into(),
            ],
            "a: ptr<mut, u8>, cb: fnptr",
            "unit",
        ),
    ];
    for (clauses, parameters, result) in cases {
        let clauses = clauses.iter().map(String::as_str).collect::<Vec<_>>();
        let text = declaration(&clauses, parameters, result);
        assert!(load(&text).is_err(), "accepted:\n{text}");
    }
}

#[test]
fn the_provider_modules_load() {
    let root = concat!(env!("CARGO_MANIFEST_DIR"), "/../../providers");
    for module in [
        "qcore-provider-libevent/contracts/monkey/event_loop.qv",
        "quirin-provider-fluent-bit/contracts/fluent_bit/input_collector_registration.qv",
    ] {
        let text = std::fs::read_to_string(format!("{root}/{module}")).expect("module");
        let contracts =
            load_resource_contract_sources(&BTreeMap::from([(module.to_owned(), text)]))
                .unwrap_or_else(|error| panic!("{module}: {error}"));
        check_source_assumptions(&contracts).expect("reloads");
    }
}
