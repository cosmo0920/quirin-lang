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

use super::*;
use crate::legacy_c::C11TranslationUnit;
use crate::typed_source::{SourceImportProfile, SourceResult, import_source_functions};
use crate::verifier_plugins::chunkio::{
    PINNED_FLUENT_BIT_REVISION,
    teardown_source::{LIST_UNLINK_CONTRACT, check_list_unlink_source},
};
const FIXTURE: &str = "struct mk_list { struct mk_list *prev; struct mk_list *next; }; static inline void __mk_list_del(struct mk_list *prev, struct mk_list *next) { prev->next=next; next->prev=prev; } static inline void mk_list_del(struct mk_list *entry) { __mk_list_del(entry->prev,entry->next); entry->prev=(void *)0; entry->next=(void *)0; }";
fn check(c: &str, contract: &str) -> Result<ListUnlinkCertificate, String> {
    let results = import_source_functions(
        &C11TranslationUnit {
            source: c,
            ..Default::default()
        },
        PINNED_FLUENT_BIT_REVISION,
        &["mk_list_del", "__mk_list_del"],
        &SourceImportProfile {
            records: &["mk_list"],
            sizeof_types: &[],
            signature_constraints: &[],
            restricted_calls: &[],
        },
    )?;
    match results.as_slice() {
        [SourceResult::Summary(outer), SourceResult::Summary(helper)] => {
            check_list_unlink(outer, helper, contract, "struct mk_list *", "ListNode")
        }
        _ => Err(format!("{results:?}")),
    }
}
#[test]
fn complete_unlink_retains_alias_cases_and_all_four_writes() {
    if !crate::test_support::clang_available() {
        return;
    }
    let cert = check_list_unlink_source(
        &C11TranslationUnit {
            source: FIXTURE,
            ..Default::default()
        },
        PINNED_FLUENT_BIT_REVISION,
    )
    .unwrap();
    assert_eq!(cert.cases().len(), 2);
    assert_eq!(
        cert.cases()[0].writes,
        vec![
            (2, "next".into(), Some(3)),
            (3, "prev".into(), Some(2)),
            (1, "prev".into(), None),
            (1, "next".into(), None)
        ]
    );
    assert_eq!(
        cert.cases()[1].writes,
        vec![
            (2, "next".into(), Some(2)),
            (2, "prev".into(), Some(2)),
            (1, "prev".into(), None),
            (1, "next".into(), None)
        ]
    );
    assert!(
        check_list_unlink_source(
            &C11TranslationUnit {
                source: FIXTURE,
                ..Default::default()
            },
            "wrong"
        )
        .is_err()
    );
}
#[test]
fn unlink_rejects_wrong_neighbor_shadow_helper_and_partial_effects() {
    if !crate::test_support::clang_available() {
        return;
    }
    for (old, new) in [
        ("prev->next=next;", "prev->next=prev;"),
        ("next->prev=prev;", ""),
        ("entry->prev,entry->next", "entry->next,entry->prev"),
        ("entry->prev,entry->next", "entry,entry->next"),
        ("entry->next=(void *)0;", ""),
        ("entry->next=(void *)0;", "entry->prev=(void *)0;"),
        (
            "entry->next=(void *)0;",
            "entry->next=(void *)0; entry->prev=(void *)0;",
        ),
        (
            "entry->next=(void *)0;",
            "if (0) { entry->next=(void *)0; }",
        ),
        ("prev->next=next;", "while (0) {} prev->next=next;"),
        ("struct mk_list *prev;", "struct mk_list *volatile prev;"),
    ] {
        assert!(
            check(&FIXTURE.replace(old, new), LIST_UNLINK_CONTRACT).is_err(),
            "accepted {new}"
        );
    }
}
#[test]
fn unlink_contract_signatures_premises_and_predicates_are_checked() {
    if !crate::test_support::clang_available() {
        return;
    }
    for (old, new) in [
        ("requires entry_is_distinct_from_neighbors;", ""),
        ("requires exclusive_link_access;", "requires unknown;"),
        ("ptr<mut, ListNode>", "borrow<'call, mut, ListNode> use one"),
        ("-> unit", "-> i32"),
        (
            "ensures normal no_object_release;",
            "ensures error no_object_release;",
        ),
    ] {
        assert!(
            check(FIXTURE, &LIST_UNLINK_CONTRACT.replace(old, new)).is_err(),
            "accepted {new}"
        );
    }
    for (i, _) in LIST_UNLINK_CONTRACT.match_indices("ensures normal ") {
        let j = i + LIST_UNLINK_CONTRACT[i..].find(';').unwrap();
        for predicate in ["false", "unsupported_predicate"] {
            let changed = format!(
                "{}ensures normal {predicate}{}",
                &LIST_UNLINK_CONTRACT[..i],
                &LIST_UNLINK_CONTRACT[j..]
            );
            assert!(check(FIXTURE, &changed).is_err());
        }
    }
}
