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

//! Caller composition through the canonical Monkey removal declaration.
//! These results are relative to declared contracts, not backend body proofs.
use std::collections::BTreeMap;

use qcore_c_gateway::legacy_c::{
    AnalysisStatus, C11TranslationUnit, LegacyProgramSummary, UnrestrictedLegacyResult,
    analyze_unrestricted_c11, check_legacy_safety_certificate,
};
use qcore_contract::contract_loader::load_resource_contract_sources;

fn check(body: &str) -> LegacyProgramSummary {
    let contracts = load_resource_contract_sources(&BTreeMap::from([
        (
            "monkey/event_loop.qv".into(),
            include_str!("../../../providers/qcore-provider-libevent/contracts/monkey/event_loop.qv")
                .into(),
        ),
        (
            "allocation.qv".into(),
            "module contract;
extern fn allocate(n: u64) -> ptr<mut, u8> contract { requires resource_contract_v1; ensures normal resource_borrowed(n); ensures normal resource_return_owned(nullable); };
extern fn drop(p: ptr<mut, u8>) -> unit contract { requires resource_contract_v1; ensures normal resource_released(p, nullable); ensures normal resource_return_none(); };".into(),
        ),
    ]))
    .unwrap();
    let source = format!(
        "struct loop; struct event {{ int state; }}; struct collector {{ struct loop *evl; struct event event; }};
int mk_event_add(struct loop *, int, int, unsigned int, void *);
int mk_event_timeout_disable(struct loop *, void *);
struct loop *mk_event_loop_create(int); void mk_event_loop_destroy(struct loop *);
void *allocate(unsigned long); void drop(void *);
{body}"
    );
    let result = analyze_unrestricted_c11(
        &C11TranslationUnit {
            source: &source,
            ..Default::default()
        },
        "caller",
        &contracts,
    )
    .unwrap();
    match result {
        UnrestrictedLegacyResult::Safe(certificate) => {
            check_legacy_safety_certificate(&certificate).unwrap();
            certificate.summary
        }
        UnrestrictedLegacyResult::Unknown {
            partial: Some(summary),
            ..
        }
        | UnrestrictedLegacyResult::Unsafe(summary) => summary,
        other => panic!("missing caller result: {other:?}"),
    }
}

#[test]
fn the_canonical_disable_contract_ends_only_the_exact_unpoisoned_registration() {
    let good = check(
        "void caller(struct collector *c) { mk_event_add(c->evl, 1, 1, 1, &c->event); mk_event_timeout_disable(c->evl, &c->event); }",
    );
    assert_eq!(
        good.functions["caller"].status,
        AnalysisStatus::Safe,
        "{:?}",
        good.functions["caller"].diagnostics
    );
    let poisoned = check(
        "void caller(struct collector *c, struct collector *other) { mk_event_add(c->evl, 1, 1, 1, &c->event); mk_event_timeout_disable(other->evl, &c->event); mk_event_timeout_disable(c->evl, &c->event); }",
    );
    let row = &poisoned.functions["caller"];
    assert_eq!(row.status, AnalysisStatus::Unknown);
    assert!(format!("{:?}", row.diagnostics).contains("retention-removal-poisoned"));
}

#[test]
fn disabling_before_release_discharges_the_callers_retained_object() {
    let prefix = "void caller(void) { struct loop *loop = mk_event_loop_create(1); if (!loop) return; void *event = allocate(16); if (!event) { mk_event_loop_destroy(loop); return; } mk_event_add(loop, 1, 1, 1, event);";
    let good = check(&format!(
        "{prefix} mk_event_timeout_disable(loop, event); drop(event); mk_event_loop_destroy(loop); }}"
    ));
    assert_eq!(
        good.functions["caller"].status,
        AnalysisStatus::Safe,
        "{:?}",
        good.functions["caller"].diagnostics
    );
    let bad = check(&format!(
        "{prefix} drop(event); mk_event_timeout_disable(loop, event); mk_event_loop_destroy(loop); }}"
    ));
    let row = &bad.functions["caller"];
    assert_eq!(row.status, AnalysisStatus::Unknown);
    assert!(format!("{:?}", row.diagnostics).contains("retention-released-while-held"));
}

#[test]
fn inferred_removal_only_helpers_transport_order_in_the_actual_caller() {
    let definitions = "static void first(struct loop *l, void *e) { mk_event_timeout_disable(l, e); } static void exact(struct loop *l, void *e) { first(l, e); } static void wrong_then_right(struct loop *l, struct loop *other, void *e) { first(other, e); first(l, e); }";
    let good = check(&format!(
        "{definitions} void caller(struct collector *c) {{ mk_event_add(c->evl, 1, 1, 1, &c->event); exact(c->evl, &c->event); }}"
    ));
    assert_eq!(
        good.functions["caller"].status,
        AnalysisStatus::Safe,
        "{:?}",
        good.functions["caller"].diagnostics
    );
    let bad = check(&format!(
        "{definitions} void caller(struct collector *c, struct collector *other) {{ mk_event_add(c->evl, 1, 1, 1, &c->event); wrong_then_right(c->evl, other->evl, &c->event); }}"
    ));
    let row = &bad.functions["caller"];
    assert_eq!(row.status, AnalysisStatus::Unknown);
    assert!(format!("{:?}", row.diagnostics).contains("retention-removal-poisoned"));
    let external = check(
        "void external_remove(struct loop *l, void *e) { mk_event_timeout_disable(l, e); } void caller(struct collector *c) { mk_event_add(c->evl, 1, 1, 1, &c->event); external_remove(c->evl, &c->event); }",
    );
    assert_ne!(external.functions["caller"].status, AnalysisStatus::Safe);
}

#[test]
fn ordered_removal_helpers_never_repair_early_release_or_erase_mixed_effects() {
    let definitions =
        "static void exact(struct loop *l, void *e) { mk_event_timeout_disable(l, e); }";
    let prefix = "void caller(void) { struct loop *loop = mk_event_loop_create(1); if (!loop) return; void *event = allocate(16); if (!event) { mk_event_loop_destroy(loop); return; } mk_event_add(loop, 1, 1, 1, event);";
    let good = check(&format!(
        "{definitions} {prefix} exact(loop, event); drop(event); mk_event_loop_destroy(loop); }}"
    ));
    assert_eq!(
        good.functions["caller"].status,
        AnalysisStatus::Safe,
        "{:?}",
        good.functions["caller"].diagnostics
    );
    let early = check(&format!(
        "{definitions} {prefix} drop(event); exact(loop, event); mk_event_loop_destroy(loop); }}"
    ));
    assert_eq!(early.functions["caller"].status, AnalysisStatus::Unknown);
    assert!(
        format!("{:?}", early.functions["caller"].diagnostics)
            .contains("retention-released-while-held")
    );
    let mixed = check(&format!(
        "{definitions} static void mixed(struct loop *l, void *e) {{ mk_event_timeout_disable(l, e); drop(e); }} {prefix} mixed(loop, event); mk_event_loop_destroy(loop); }}"
    ));
    assert_ne!(mixed.functions["caller"].status, AnalysisStatus::Safe);
    let volatile = check(&format!(
        "typedef void * volatile event_ptr; static void changing(struct loop *l, event_ptr e) {{ mk_event_timeout_disable(l, e); }} {prefix} changing(loop, event); drop(event); mk_event_loop_destroy(loop); }}"
    ));
    assert_ne!(volatile.functions["caller"].status, AnalysisStatus::Safe);
}
