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

//! Card SUPPLIED-OWNED-RETURN-CODE-HANDOFF: a formal a callback takes
//! depending on its return code (contract specification §C04.19.5), read
//! total-or-reject (grammar G05, G05d), in the spellings of the user's
//! decisions of 2026-10-15: `#` binders and value keywords, `@` C names, the
//! module-local `alias`, and
//! `quant live #rko: #owner |-> #callback when #result != @NAME;`.

use std::collections::BTreeMap;

use qcore_contract::contract_loader::load_resource_contract_sources;
use qcore_frontend::lexer::MAPS_TO;
use qcore_schemas::contract::ResourceContracts;
use qcore_schemas::handoff::{GuardOp, HandOff, HandOffGuard};

fn load(text: &str) -> Result<ResourceContracts, String> {
    load_resource_contract_sources(&BTreeMap::from([(
        "handoff.qv".to_owned(),
        format!("module contract;\n{text}"),
    )]))
}

/// An op callback `(rk, rkq, rko) -> u32` whose op is handed by `GUARD`.
fn op_cb(name: &str, guard: &str) -> String {
    format!(
        "
extern fn {name}(rk: ptr<mut, u8>, rkq: ptr<mut, u8>, rko: ptr<mut, u8>) -> u32
contract {{
    requires resource_contract_v1;
    ensures normal resource_borrowed(rk);
    ensures normal resource_borrowed(rkq);
    ensures normal resource_owned(rko, nonnull);
    ensures normal resource_return_none();
    quant live #rko: #owner {MAPS_TO} #callback when #result {guard};
}};
"
    )
}

fn handoff(contracts: &ResourceContracts, name: &str) -> HandOff {
    let declared = contracts.retaining_contract(name).expect("withheld");
    assert!(
        contracts.contract(name).is_none(),
        "a hand-off is never in the function map"
    );
    declared.handoff.clone().expect("a hand-off")
}

#[test]
fn the_keep_only_and_not_pass_forms_load_withheld() {
    let contracts = load(&format!(
        "{}{}",
        op_cb("keeps", "== @RD_KAFKA_OP_RES_KEEP"),
        op_cb("passes", "!= @RD_KAFKA_OP_RES_PASS")
    ))
    .expect("loads");
    assert_eq!(
        handoff(&contracts, "keeps"),
        HandOff {
            parameter: 2,
            guard: HandOffGuard {
                op: GuardOp::Eq,
                constant: "RD_KAFKA_OP_RES_KEEP".into()
            }
        }
    );
    assert_eq!(
        handoff(&contracts, "passes"),
        HandOff {
            parameter: 2,
            guard: HandOffGuard {
                op: GuardOp::Ne,
                constant: "RD_KAFKA_OP_RES_PASS".into()
            }
        }
    );
}

/// The user's decision: both spellings load to one canonical value.
#[test]
fn an_alias_and_its_name_read_identically() {
    let direct = load(&op_cb("cb", "!= @RD_KAFKA_OP_RES_PASS")).expect("loads");
    let aliased = load(&format!(
        "alias #PASS = @RD_KAFKA_OP_RES_PASS;\n{}",
        op_cb("cb", "!= #PASS")
    ))
    .expect("loads");
    assert_eq!(
        direct.retaining_contract("cb"),
        aliased.retaining_contract("cb")
    );
}

#[test]
fn every_misuse_rejects_the_module() {
    let rejects = |text: &str, needle: &str| {
        let error = load(text).expect_err(text);
        assert!(error.contains(needle), "{text}: {error}");
    };
    // An alias the module does not declare, and an alias in another module.
    rejects(&op_cb("cb", "!= #PASS"), "quant-live-handoff-alias");
    // A number, a bare word, an ordering.
    rejects(&op_cb("cb", "!= 0"), "quant-live-handoff-constant");
    rejects(
        &op_cb("cb", "!= RD_KAFKA_OP_RES_PASS"),
        "quant-live-handoff-constant",
    );
    rejects(
        &op_cb("cb", "< @RD_KAFKA_OP_RES_PASS"),
        "quant-live-handoff-guard",
    );
    // The handed formal must be stated live (`resource_owned(rko, nonnull)`).
    rejects(
        &op_cb("cb", "!= @K").replace("resource_owned(rko, nonnull)", "resource_borrowed(rko)"),
        "quant-live-unbacked",
    );
    // The guarded result is an integer code with no resource.
    rejects(
        &op_cb("cb", "!= @K").replace(") -> u32", ") -> ptr<mut, u8>"),
        "quant-live-handoff-result",
    );
    // One hand-off per declaration.
    rejects(
        &op_cb("cb", "!= @K").replace(
            "    ensures normal resource_return_none();",
            &format!(
                "    ensures normal resource_return_none();\n    quant live #rko: #owner {MAPS_TO} #callback when #result == @J;"
            ),
        ),
        "one hand-off",
    );
    // A parameter named like a reserved keyword, beside a `#` form.
    rejects(
        &op_cb("cb", "!= @K").replace("rkq", "callback"),
        "rename the parameter `callback`",
    );
    rejects(
        &op_cb("cb", "!= @K").replace("rkq", "owner"),
        "rename the parameter `owner`",
    );
    // Alias collisions: a reserved keyword, a binder of the module.
    rejects(
        &format!("alias #owner = @K;\n{}", op_cb("cb", "!= #owner")),
        "alias-collision",
    );
    rejects(
        &format!("alias #rkq = @K;\n{}", op_cb("cb", "!= #rkq")),
        "alias-collision",
    );
    // Declared twice, or with a right-hand side that is no `@` name.
    rejects(
        &format!("alias #P = @K;\nalias #P = @K;\n{}", op_cb("cb", "!= #P")),
        "declared once per module",
    );
    rejects(
        &format!("alias #P = K;\n{}", op_cb("cb", "!= #P")),
        "`@` name",
    );
    rejects(
        &format!("alias #P = #Q;\n{}", op_cb("cb", "!= #P")),
        "`@` name",
    );
}

/// The parameter-name reservation is scoped to declarations with a `#` form:
/// the serve family's `callback` formal (no `#` form) still loads.
#[test]
fn a_reserved_name_without_a_hash_form_still_loads() {
    load(
        "
extern fn serve(rkq: ptr<mut, u8>, callback: fnptr) -> i32
contract {
    requires resource_contract_v1;
    ensures normal resource_borrowed(rkq);
    ensures normal resource_borrowed(callback);
    ensures normal resource_return_none();
};
",
    )
    .expect("no `#` form, no reservation");
}

/// A member invocation may read the handed (`resource_owned`) formal, and no
/// other owned formal.
#[test]
fn a_member_invocation_reads_only_the_handed_owned_formal() {
    let with = op_cb("cb", "!= @K").replace(
        "    ensures normal resource_return_none();",
        "    ensures normal resource_callback_member_invoked(rko, op.cb, _, _, rko);\n    ensures normal resource_return_none();",
    );
    let declared = load(&with).expect("loads");
    assert_eq!(
        declared
            .retaining_contract("cb")
            .expect("withheld")
            .callbacks
            .len(),
        1
    );
    let other = with
        .replace("resource_borrowed(rkq)", "resource_owned(rkq, nonnull)")
        .replace("member_invoked(rko, op.cb", "member_invoked(rkq, op.cb");
    assert!(
        load(&other)
            .expect_err("an owned object that is not handed")
            .contains("the formal a hand-off hands")
    );
}

/// An `alias` belongs to a contract module.
#[test]
fn an_alias_outside_a_contract_module_is_a_parse_error() {
    let error = qcore_frontend::frontend::parse_source_program(
        "module library;\nalias #P = @K;\nfn f() -> i32 { return 0; }\n",
    )
    .expect_err("not a contract module");
    assert!(
        error.message.contains("contract module"),
        "{}",
        error.message
    );
}

/// The alias a declaration spells is part of its pinned contract text: a
/// changed alias target changes that declaration's digest (an alias never
/// changes meaning under an unchanged pin), and no other declaration's. The
/// declaration's `@` names, through the alias too, are the names `quirin-audit
/// pin` adds the enumerations of.
#[test]
fn an_alias_is_part_of_the_digest_of_every_declaration_spelling_it() {
    let module = |target: &str| {
        BTreeMap::from([(
            "handoff.qv".to_owned(),
            format!(
                "module contract;\nalias #PASS = @{target};\n{}{}",
                op_cb("aliased", "!= #PASS"),
                op_cb("named", "== @RD_KAFKA_OP_RES_KEEP")
            ),
        )])
    };
    let before =
        qcore_contract::contract_revision::declaration_digests(&module("RD_KAFKA_OP_RES_PASS"))
            .expect("digests");
    let after =
        qcore_contract::contract_revision::declaration_digests(&module("RD_KAFKA_OP_RES_HANDLED"))
            .expect("digests");
    assert_ne!(before["aliased"], after["aliased"]);
    assert_eq!(before["named"], after["named"]);
    let names =
        qcore_contract::contract_revision::declaration_c_names(&module("RD_KAFKA_OP_RES_PASS"))
            .expect("names");
    assert_eq!(
        names["aliased"].iter().collect::<Vec<_>>(),
        vec!["RD_KAFKA_OP_RES_PASS"]
    );
    assert_eq!(
        names["named"].iter().collect::<Vec<_>>(),
        vec!["RD_KAFKA_OP_RES_KEEP"]
    );
}
