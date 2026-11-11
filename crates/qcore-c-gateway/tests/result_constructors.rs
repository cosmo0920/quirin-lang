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

//! Rule 37: unevaluated allocation operands preserve constructor field links.
use qcore_c_gateway::legacy_c::{
    AnalysisStatus, C11TranslationUnit, LegacyFunctionSummary, ResourceContracts,
    UnrestrictedLegacyResult, analyze_unrestricted_c11, check_legacy_safety_certificate,
};
use qcore_contract::contract_loader::load_resource_contract_sources;
use std::collections::BTreeMap;

fn contracts(nullable: bool) -> ResourceContracts {
    let allocation = if nullable { "nullable" } else { "nonnull" };
    let text = format!(
        r#"module contract;
extern fn allocate(size: usize) -> ptr<mut, u8> contract {{
 requires resource_contract_v1;
 ensures normal resource_borrowed(size);
 ensures normal resource_return_owned({allocation});
}};
extern fn duplicate(s: ptr<const, u8>) -> ptr<mut, u8> contract {{
 requires resource_contract_v1;
 ensures normal resource_borrowed(s);
 ensures normal resource_return_owned(nullable);
}};
extern fn release(p: ptr<mut, u8>) -> unit contract {{
 requires resource_contract_v1;
 ensures normal resource_released(p, nullable);
 ensures normal resource_return_none();
}};
"#
    );
    load_resource_contract_sources(&BTreeMap::from([("constructor.qv".into(), text)])).unwrap()
}
fn run(body: &str, nullable: bool) -> LegacyFunctionSummary {
    let source = format!(
        r#"typedef __SIZE_TYPE__ size_t;
 struct envelope {{ char *name; int state; }};
 void *allocate(size_t size); char *duplicate(const char *s); void release(void *p);
 {body}"#
    );
    let result = analyze_unrestricted_c11(
        &C11TranslationUnit {
            source: &source,
            ..Default::default()
        },
        "build",
        &contracts(nullable),
    )
    .unwrap();
    let summary = match &result {
        UnrestrictedLegacyResult::Safe(c) => {
            check_legacy_safety_certificate(c).unwrap();
            &c.summary
        }
        UnrestrictedLegacyResult::Unsafe(s)
        | UnrestrictedLegacyResult::Unknown {
            partial: Some(s), ..
        } => s,
        other => panic!("missing program: {other:?}"),
    };
    summary.functions["build"].clone()
}
#[test]
fn allocation_size_dereference_is_not_a_constructor_escape() {
    let f = run(
        "struct envelope *build(const char *name) { struct envelope *e; e=allocate(sizeof(*e)); e->name=duplicate(name); e->state=3; return e; }",
        false,
    );
    assert_eq!(f.status, AnalysisStatus::Safe, "{f:#?}");
}
#[test]
fn a_nullable_constructor_owner_still_refuses() {
    let f = run(
        "struct envelope *build(const char *name) { struct envelope *e; e=allocate(sizeof(*e)); e->name=duplicate(name); return e; }",
        true,
    );
    assert_ne!(f.status, AnalysisStatus::Safe, "{f:#?}");
}
#[test]
fn a_guarded_nullable_owner_can_construct() {
    let f = run(
        "struct envelope *build(const char *name) { struct envelope *e; e=allocate(sizeof(*e)); if (!e) return 0; e->name=duplicate(name); return e; }",
        true,
    );
    assert_eq!(f.status, AnalysisStatus::Safe, "{f:#?}");
}
#[test]
fn a_shallow_release_leaves_the_child_obligation() {
    let f = run(
        "void build(const char *name) { struct envelope *e; e=allocate(sizeof(*e)); e->name=duplicate(name); release(e); }",
        false,
    );
    assert_ne!(f.status, AnalysisStatus::Safe, "{f:#?}");
}
#[test]
fn an_evaluated_owner_dereference_keeps_the_refusal() {
    let f = run(
        "struct envelope *build(const char *name) { struct envelope *e; e=allocate(sizeof(*e)); *e=(struct envelope){0}; e->name=duplicate(name); return e; }",
        false,
    );
    assert_ne!(f.status, AnalysisStatus::Safe, "{f:#?}");
}

#[test]
fn a_member_release_after_sizeof_keeps_its_insulated_owner() {
    let f = run(
        "void build(const char *name) { struct envelope *e; e=allocate(sizeof(*e)); e->name=duplicate(name); release(e->name); release(e); }",
        false,
    );
    assert_eq!(f.status, AnalysisStatus::Safe, "{f:#?}");
}
#[test]
fn a_second_member_release_never_reuses_the_old_link() {
    let f = run(
        "void build(const char *name) { struct envelope *e; e=allocate(sizeof(*e)); e->name=duplicate(name); release(e->name); release(e->name); release(e); }",
        false,
    );
    assert_ne!(f.status, AnalysisStatus::Safe, "{f:#?}");
}
#[test]
fn overwriting_the_member_does_not_discharge_its_first_child() {
    let f = run(
        "struct envelope *build(const char *name) { struct envelope *e; e=allocate(sizeof(*e)); e->name=duplicate(name); e->name=duplicate(name); return e; }",
        false,
    );
    assert_ne!(f.status, AnalysisStatus::Safe, "{f:#?}");
}

#[test]
fn a_checked_sizeof_does_not_hide_an_owner_alias() {
    let f = run(
        "struct envelope *build(const char *name) { struct envelope *e; struct envelope *other; e=allocate(sizeof(*e)); other=e; e->name=duplicate(name); return other; }",
        false,
    );
    assert_ne!(f.status, AnalysisStatus::Safe, "{f:#?}");
}
