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

//! Card OPENSSL-PEM-CALLBACK-ARM (gateway rule 57, contract specification
//! §C04.5 "Beside a synchronous invocation"): a literal-argument premise
//! stands beside synchronous invocations that are a declaration's only
//! retaining clauses; every other retaining clause beside a premise rejects
//! the module.

use std::collections::BTreeMap;

use qcore_contract::contract_loader::{literal_argument_premises, load_resource_contract_sources};
use qcore_schemas::callback::{CallbackActual, CallbackClause};
use qcore_schemas::contract::ResourceContracts;

fn sources(text: &str) -> BTreeMap<String, String> {
    BTreeMap::from([(
        "premise_callbacks.qv".to_owned(),
        format!("module contract;\n{text}"),
    )])
}

fn load(text: &str) -> Result<ResourceContracts, String> {
    load_resource_contract_sources(&sources(text))
}

/// The PEM reader's shape: a premise on `x`, the password callback invoked
/// with three values the callee supplies and the caller's `u`.
const READER: &str = "
extern fn read_pem(out: ptr<mut, u8>, x: ptr<mut, ptr<mut, u8>>, cb: fnptr, u: ptr<mut, u8>) -> ptr<mut, u8>
contract {
    requires resource_contract_v1;
    requires resource_literal_argument(x, 0);
    ensures normal resource_borrowed(out);
    ensures normal resource_borrowed(x);
    ensures normal resource_borrowed(cb);
    ensures normal resource_borrowed(u);
    ensures normal resource_callback_invoked(cb, _, _, _, u);
    ensures normal resource_return_owned(nullable);
};
";

#[test]
fn a_premise_stands_beside_a_synchronous_invocation() {
    let contracts = load(READER).expect("a premise beside an invocation loads");
    // Withheld from the function map like every callback declaration.
    assert!(contracts.functions().get("read_pem").is_none());
    let declared = contracts
        .retaining()
        .get("read_pem")
        .expect("a retaining declaration");
    assert_eq!(
        declared.callbacks,
        vec![CallbackClause::Invoked {
            callback: 2,
            actuals: vec![
                CallbackActual::Supplied,
                CallbackActual::Supplied,
                CallbackActual::Supplied,
                CallbackActual::Parameter { index: 3 },
            ],
            null_default: false,
        }]
    );
    // The premise is rederived from the text, as for a plain declaration.
    let premises = literal_argument_premises(&sources(READER)).expect("premises");
    let premise = &premises["read_pem"];
    assert_eq!(premise.len(), 1);
    assert_eq!(premise[0].parameter, 1);
    assert_eq!(premise[0].literal, vec!["0".to_owned()]);
    assert!(premise[0].pointer);
}

#[test]
fn every_other_retaining_clause_beside_a_premise_rejects() {
    let invoked = "    ensures normal resource_callback_invoked(cb, _, _, _, u);\n";
    for (replacement, why) in [
        (
            "    ensures normal resource_retained_by(u, out);\n",
            "a retention alone",
        ),
        (
            "    ensures normal resource_callback_invoked(cb, _, _, _, u);\n    ensures normal resource_retained_by(u, out);\n",
            "an invocation beside a retention",
        ),
        (
            "    ensures normal resource_callback_registered(cb, out);\n",
            "a registration",
        ),
        (
            "    ensures normal resource_callback_table(out, _, _, _, u);\n",
            "a table dispatch",
        ),
        (
            "    ensures normal resource_callback_invoked(cb, _, _, _, u);\n    ensures normal resource_callback_registry_invoked(out, _, _, _, u);\n",
            "an invocation beside a registry invocation",
        ),
    ] {
        let text = READER.replacen(invoked, replacement, 1);
        assert_ne!(text, READER, "{why}: the replacement applied");
        let error = load(&text).expect_err(why);
        assert!(
            error.contains("stands only beside") || error.contains("synchronous invocation only"),
            "{why}: {error}"
        );
        // Total-or-reject: the rederivation refuses too, never a partial set.
        assert!(literal_argument_premises(&sources(&text)).is_err(), "{why}");
    }
}

#[test]
fn the_premise_still_rejects_its_own_malformations_beside_an_invocation() {
    for (from, to) in [
        (
            "resource_literal_argument(x, 0)",
            "resource_literal_argument(absent, 0)",
        ),
        (
            "resource_literal_argument(x, 0)",
            "resource_literal_argument(x, 0x0)",
        ),
        (
            "    requires resource_literal_argument(x, 0);\n",
            "    requires resource_literal_argument(x, 0);\n    requires resource_literal_argument(x, 0);\n",
        ),
    ] {
        let text = READER.replacen(from, to, 1);
        assert!(load(&text).is_err(), "{to}");
    }
}

#[test]
fn an_invocation_without_a_premise_is_unchanged() {
    let text = READER.replacen("    requires resource_literal_argument(x, 0);\n", "", 1);
    let contracts = load(&text).expect("loads");
    assert!(contracts.retaining().contains_key("read_pem"));
    assert!(
        !literal_argument_premises(&sources(&text))
            .expect("premises")
            .contains_key("read_pem")
    );
}

#[test]
fn the_null_default_spelling_loads_as_a_marked_invocation() {
    let invoked = "resource_callback_invoked(cb, _, _, _, u)";
    let text = READER.replacen(
        invoked,
        "resource_callback_invoked_or_default(cb, _, _, _, u)",
        1,
    );
    let contracts = load(&text).expect("the null default loads beside a premise");
    let declared = &contracts.retaining()["read_pem"];
    assert_eq!(
        declared.callbacks,
        vec![CallbackClause::Invoked {
            callback: 2,
            actuals: vec![
                CallbackActual::Supplied,
                CallbackActual::Supplied,
                CallbackActual::Supplied,
                CallbackActual::Parameter { index: 3 },
            ],
            null_default: true,
        }]
    );
    assert_eq!(
        declared.callbacks[0].keyword(),
        "resource_callback_invoked_or_default"
    );
    // One clause per callback: both spellings on one callback reject.
    let both = text.replacen(
        "    ensures normal resource_callback_invoked_or_default(cb, _, _, _, u);\n",
        "    ensures normal resource_callback_invoked_or_default(cb, _, _, _, u);\n    ensures normal resource_callback_invoked(cb, _, _, _, u);\n",
        1,
    );
    assert_ne!(both, text);
    assert!(load(&both).is_err());
    // Its placement is the invocation's: a callback that is not `fnptr`.
    let misplaced = text.replacen("_or_default(cb,", "_or_default(u,", 1);
    assert_ne!(misplaced, text);
    assert!(load(&misplaced).is_err());
}

/// Card OPENSSL-BIO-READER-INVOCATIONS (rule 57 amended, §C04.5 "Beside a
/// slot invocation"): the BIO slots a reader runs through `out`.
const SLOT_CALLBACK: &str = "    ensures normal resource_callback_slot_invoked(out, bio_st.bio_callback_ex, _, _, _, _, _, _, _, _);\n";
const SLOT_READ: &str =
    "    ensures normal resource_callback_slot_invoked(out, bio_method_st.meth_read, _, _, _);\n";

#[test]
fn a_premise_stands_beside_a_slot_invocation() {
    let invoked = "    ensures normal resource_callback_invoked(cb, _, _, _, u);\n";
    // Beside the synchronous invocation (the PEM readers' shape) and alone
    // (`d2i_PKCS12_bio`'s).
    let beside = READER.replacen(invoked, &format!("{invoked}{SLOT_CALLBACK}{SLOT_READ}"), 1);
    let alone = READER.replacen(invoked, &format!("{SLOT_CALLBACK}{SLOT_READ}"), 1);
    for (text, invocations) in [(&beside, 3), (&alone, 2)] {
        assert_ne!(text, READER);
        let contracts = load(text).expect("a premise beside a slot invocation loads");
        assert!(contracts.functions().get("read_pem").is_none());
        let declared = &contracts.retaining()["read_pem"];
        assert_eq!(declared.callbacks.len(), invocations);
        let slots = declared
            .callbacks
            .iter()
            .filter(|clause| matches!(clause, CallbackClause::MemberInvoked { slot: true, .. }))
            .count();
        assert_eq!(slots, 2);
        // The premise is rederived from the text, so route G withdraws the
        // declaration, slot invocations included, at every unmet call.
        let premises = literal_argument_premises(&sources(text)).expect("premises");
        assert_eq!(premises["read_pem"].len(), 1);
        assert_eq!(premises["read_pem"][0].parameter, 1);
    }
}

#[test]
fn a_plain_member_invocation_or_a_slot_registration_beside_a_premise_still_rejects() {
    let invoked = "    ensures normal resource_callback_invoked(cb, _, _, _, u);\n";
    for (replacement, why) in [
        (
            "    ensures normal resource_callback_member_invoked(out, bio_st.bio_callback_ex, _, _, _, _, _, _, _, _);\n",
            "a plain member invocation",
        ),
        (
            "    ensures normal resource_callback_slot_registered(out, bio_st.bio_callback_ex, cb);\n",
            "a slot registration",
        ),
        (
            "    ensures normal resource_callback_member_registered(out, bio_st.bio_callback_ex, cb);\n",
            "a member registration",
        ),
    ] {
        // Beside an admitted slot invocation (the callback `cb` is then the
        // registration's own, not also invoked).
        let text = READER.replacen(invoked, &format!("{SLOT_READ}{replacement}"), 1);
        assert_ne!(text, READER, "{why}");
        let error = load(&text).expect_err(why);
        assert!(
            error.contains("stands only beside") || error.contains("synchronous invocation only"),
            "{why}: {error}"
        );
        assert!(literal_argument_premises(&sources(&text)).is_err(), "{why}");
        // Without the premise the same clauses do not reject on the
        // premise's ground, so the refusal above is the premise's.
        let unpremised = text.replacen("    requires resource_literal_argument(x, 0);\n", "", 1);
        if let Err(other) = load(&unpremised) {
            assert!(
                !other.contains("synchronous invocation only"),
                "{why}: {other}"
            );
        }
    }
}
