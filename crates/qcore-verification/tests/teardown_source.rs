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

//! Trusted replay for the two process-transported teardown capabilities.
//!
//! Every case here is a refusal or an identity check. Nothing in this file
//! establishes ring membership, exclusivity, allocation provenance or the
//! `free`/libc binding: those stay the pinned contracts' declared premises,
//! and acceptance always terminates in the core evaluating those contracts
//! over its own import.
#[path = "fixtures/teardown_source.rs"]
mod fixture;
use fixture::*;

use qcore_c_gateway::teardown_source::{
    FieldDestructorSourceResult, ListUnlinkSourceResult, import_field_destructor_sources,
    import_list_unlink_sources,
};
use qcore_schemas::teardown_summary::{
    FieldReleaseCaseRecord, LinkWriteRecord, TeardownContractWitness, TeardownFamilyInput,
    TeardownProposalRecord,
};
use qcore_schemas::witness::ContractObservationWitness;
use qcore_schemas::{SemanticWitness, TargetProfile};
use qcore_verification::field_destructor::declared_owned_fields;
use qcore_verification::teardown_source::{
    FIELD_DESTRUCTOR_SEMANTIC_PROFILE, FieldDestructorSourceTask, LIST_UNLINK_SEMANTIC_PROFILE,
    ListUnlinkSourceTask,
};
use qcore_verifier_host::WitnessChecker;

/// The reading the core itself computes, obtained without going through the
/// provider: `check` accepts exactly this and nothing else.
fn unlink_reading(task: &ListUnlinkSourceTask) -> TeardownProposalRecord {
    let certificate = task.evaluate().unwrap();
    let cases = certificate
        .cases()
        .iter()
        .map(|case| qcore_schemas::teardown_summary::UnlinkCaseRecord {
            neighbors_alias: case.neighbors_alias,
            helper_calls: case.helper_calls as u32,
            writes: case
                .writes
                .iter()
                .map(|(node, field, value)| LinkWriteRecord {
                    node: *node,
                    field: field.clone(),
                    value: *value,
                })
                .collect(),
        })
        .collect();
    TeardownProposalRecord::ListUnlink(qcore_schemas::teardown_summary::ListUnlinkRecord {
        function: UNLINK_ENTRY.into(),
        helper_function: UNLINK_HELPER.into(),
        source_revision: revision(),
        node_c_type: NODE_C_TYPE.into(),
        cases,
        profile_assumptions: Default::default(),
    })
}

fn destructor_reading(task: &FieldDestructorSourceTask) -> TeardownProposalRecord {
    let certificate = task.evaluate().unwrap();
    TeardownProposalRecord::FieldDestructor(
        qcore_schemas::teardown_summary::FieldDestructorRecord {
            function: DESTRUCTOR_FUNCTION.into(),
            source_revision: revision(),
            root_c_type: ROOT_C_TYPE.into(),
            declared_owned_fields: OWNED_FIELDS.iter().map(|f| (*f).to_owned()).collect(),
            cases: certificate
                .cases()
                .iter()
                .map(|case| FieldReleaseCaseRecord {
                    initially_live: case.initially_live.clone(),
                    released_in_order: case.released_in_order.clone(),
                    source_path: case.source_path as u32,
                })
                .collect(),
            profile_assumptions: Default::default(),
        },
    )
}

fn witness(digest: &str, summary: TeardownProposalRecord) -> SemanticWitness {
    SemanticWitness::TeardownContract(Box::new(TeardownContractWitness {
        task_digest: digest.into(),
        summary,
    }))
}

#[test]
fn the_core_admits_only_its_own_unlink_reading() {
    if !clang_available() {
        return;
    }
    let import = unlink_import(UNLINK_SOURCE, vec![]);
    let task = unlink_task(&import, UNLINK_CONTRACT);
    let digest = task.task().task_digest();
    let accepted = unlink_reading(&task);
    assert_eq!(
        task.check(task.task(), &witness(&digest, accepted.clone()))
            .unwrap(),
        task.task().coverage_for(task.capability())
    );
    // The transported request carries the two bodies and no derived reading.
    let request = task.request(qcore_schemas::PluginId::new("qcore.chunkio").unwrap());
    let value = serde_json::to_value(request.teardown_source.as_ref().unwrap()).unwrap();
    assert!(value.get("cases").is_none() && value.get("writes").is_none());
    assert_eq!(value["family"]["family"], "list_unlink");
    assert!(value["family"]["entry"]["paths"].is_array());
    request.validate().unwrap();

    let TeardownProposalRecord::ListUnlink(original) = accepted else {
        panic!("unlink reading")
    };
    // Twelve forged readings, each refused against the core's own certificate.
    type UnlinkForge = Box<dyn Fn(&mut qcore_schemas::teardown_summary::ListUnlinkRecord)>;
    let forgeries: Vec<(&str, UnlinkForge)> = vec![
        ("emptied cases", Box::new(|r| r.cases.clear())),
        (
            "dropped alias case",
            Box::new(|r| {
                r.cases.truncate(1);
            }),
        ),
        (
            "duplicated case",
            Box::new(|r| {
                let first = r.cases[0].clone();
                r.cases.push(first);
            }),
        ),
        ("swapped alias classes", Box::new(|r| r.cases.swap(0, 1))),
        (
            "relabelled alias flag",
            Box::new(|r| r.cases[0].neighbors_alias = true),
        ),
        (
            "inflated helper call count",
            Box::new(|r| r.cases[0].helper_calls = 2),
        ),
        (
            "erased helper call",
            Box::new(|r| r.cases[0].helper_calls = 0),
        ),
        (
            "dropped an entry null write",
            Box::new(|r| {
                r.cases[0].writes.pop();
            }),
        ),
        (
            "reordered writes",
            Box::new(|r| r.cases[0].writes.swap(0, 2)),
        ),
        (
            "retargeted a link write",
            Box::new(|r| r.cases[0].writes[0].node = 3),
        ),
        (
            "changed a written value",
            Box::new(|r| r.cases[0].writes[0].value = None),
        ),
        (
            "renamed the helper",
            Box::new(|r| r.helper_function = "memcpy".into()),
        ),
    ];
    for (name, forge) in forgeries {
        let mut forged = original.clone();
        forge(&mut forged);
        assert_ne!(forged, original, "{name} did not change the reading");
        assert!(
            task.check(
                task.task(),
                &witness(&digest, TeardownProposalRecord::ListUnlink(forged))
            )
            .is_err(),
            "{name} was admitted"
        );
    }
    // Identity: another family's proposal, another witness kind, a witness
    // bound to another task, and a task the checker did not derive.
    assert!(
        task.check(
            task.task(),
            &witness(
                &digest,
                destructor_reading(&destructor_task(
                    &destructor_import(DESTRUCTOR_SOURCE, vec![]),
                    DESTRUCTOR_CONTRACT
                ))
            )
        )
        .is_err()
    );
    assert!(
        task.check(
            task.task(),
            &SemanticWitness::ContractObservation(ContractObservationWitness {
                task_digest: digest.clone(),
                runs: Vec::new(),
            })
        )
        .is_err()
    );
    assert!(
        task.check(
            task.task(),
            &witness(&"f".repeat(64), unlink_reading(&task))
        )
        .is_err()
    );
    let mut foreign = task.task().clone();
    foreign.unit.translation_unit = "other.c".into();
    assert!(
        task.check(&foreign, &witness(&digest, unlink_reading(&task)))
            .is_err()
    );
}

#[test]
fn the_core_admits_only_its_own_destructor_reading() {
    if !clang_available() {
        return;
    }
    let import = destructor_import(DESTRUCTOR_SOURCE, vec![]);
    let task = destructor_task(&import, DESTRUCTOR_CONTRACT);
    let digest = task.task().task_digest();
    let accepted = destructor_reading(&task);
    assert_eq!(
        task.check(task.task(), &witness(&digest, accepted.clone()))
            .unwrap(),
        task.task().coverage_for(task.capability())
    );
    let request = task.request(qcore_schemas::PluginId::new("qcore.chunkio").unwrap());
    let value = serde_json::to_value(request.teardown_source.as_ref().unwrap()).unwrap();
    assert!(value.get("cases").is_none() && value.get("released_in_order").is_none());
    assert_eq!(value["family"]["family"], "field_destructor");
    request.validate().unwrap();

    let TeardownProposalRecord::FieldDestructor(original) = accepted else {
        panic!("destructor reading")
    };
    assert_eq!(original.cases.len(), 9);
    type Forge = Box<dyn Fn(&mut qcore_schemas::teardown_summary::FieldDestructorRecord)>;
    let forgeries: Vec<(&str, Forge)> = vec![
        ("emptied cases", Box::new(|r| r.cases.clear())),
        (
            "dropped the fully live state",
            Box::new(|r| {
                r.cases.pop();
            }),
        ),
        (
            "duplicated a state",
            Box::new(|r| {
                let last = r.cases.last().unwrap().clone();
                r.cases.push(last);
            }),
        ),
        ("reordered states", Box::new(|r| r.cases.swap(0, 8))),
        (
            "claimed an unreleased field was released",
            Box::new(|r| {
                r.cases[0].released_in_order.push("backend".into());
            }),
        ),
        (
            "hid a real release",
            Box::new(|r| {
                r.cases[8].released_in_order.pop();
            }),
        ),
        (
            "reordered a release sequence",
            Box::new(|r| r.cases[8].released_in_order.swap(0, 3)),
        ),
        (
            "relabelled a live set",
            Box::new(|r| r.cases[1].initially_live.push("backend__name".into())),
        ),
        (
            "retargeted a source path",
            Box::new(|r| r.cases[8].source_path = 0),
        ),
        (
            "widened the declared inventory",
            Box::new(|r| r.declared_owned_fields.push("absent".into())),
        ),
        (
            "renamed the function",
            Box::new(|r| r.function = "cio_memfs_open".into()),
        ),
        (
            "moved the revision",
            Box::new(|r| r.source_revision = "0".repeat(40)),
        ),
        (
            "changed the root C type",
            Box::new(|r| r.root_c_type = "struct cio_file *".into()),
        ),
    ];
    for (name, forge) in forgeries {
        let mut forged = original.clone();
        forge(&mut forged);
        assert_ne!(forged, original, "{name} did not change the reading");
        assert!(
            task.check(
                task.task(),
                &witness(&digest, TeardownProposalRecord::FieldDestructor(forged))
            )
            .is_err(),
            "{name} was admitted"
        );
    }
}

#[test]
fn a_false_contract_is_not_bought_by_a_correct_reading() {
    if !clang_available() {
        return;
    }
    // The unlink contract asserting three writes instead of four is false for
    // this body. The core evaluates the contract itself, so the reading a
    // provider would faithfully compute cannot discharge the obligation.
    let import = unlink_import(UNLINK_SOURCE, vec![]);
    let honest = unlink_task(&import, UNLINK_CONTRACT);
    let false_contract = UNLINK_CONTRACT.replace(
        "ensures normal write_count == 4;",
        "ensures normal write_count == 3;",
    );
    let task = unlink_task(&import, &false_contract);
    assert!(task.evaluate().is_err());
    let digest = task.task().task_digest();
    assert!(
        task.check(task.task(), &witness(&digest, unlink_reading(&honest)))
            .is_err()
    );

    // Same for the destructor: a contract that permits retaining the backend
    // is refused by the core's own evaluation whatever the proposal says.
    let import = destructor_import(DESTRUCTOR_SOURCE, vec![]);
    let honest = destructor_task(&import, DESTRUCTOR_CONTRACT);
    let false_contract = DESTRUCTOR_CONTRACT.replace(
        "ensures normal live__backend == 0;",
        "ensures normal live__backend == 1;",
    );
    let task = destructor_task(&import, &false_contract);
    assert!(task.evaluate().is_err());
    let digest = task.task().task_digest();
    assert!(
        task.check(task.task(), &witness(&digest, destructor_reading(&honest)))
            .is_err()
    );
}

#[test]
fn incompatible_targets_and_inventories_refuse_derivation() {
    if !clang_available() {
        return;
    }
    let import = unlink_import(UNLINK_SOURCE, vec![]);
    let base = unlink_target(&import);
    for (name, target) in [
        (
            "the destructor profile",
            TargetProfile {
                profile: FIELD_DESTRUCTOR_SEMANTIC_PROFILE.into(),
                ..base.clone()
            },
        ),
        (
            "an unsupported dialect",
            TargetProfile {
                dialect: "c99".into(),
                ..base.clone()
            },
        ),
        (
            "a moved revision",
            TargetProfile {
                revision: "0".repeat(40),
                ..base.clone()
            },
        ),
        (
            "a foreign platform",
            TargetProfile {
                platform: "plan9".into(),
                ..base.clone()
            },
        ),
        (
            "an emulated backend",
            TargetProfile {
                backend: "emulated".into(),
                ..base.clone()
            },
        ),
        (
            "an empty library",
            TargetProfile {
                library: String::new(),
                ..base.clone()
            },
        ),
    ] {
        assert!(
            ListUnlinkSourceTask::derive(
                &import,
                "mk_list.c",
                UNLINK_CONTRACT,
                NODE_QCORE_TYPE,
                target
            )
            .is_err(),
            "{name} was accepted"
        );
    }
    assert!(
        ListUnlinkSourceTask::derive(&import, "", UNLINK_CONTRACT, NODE_QCORE_TYPE, base.clone())
            .is_err()
    );
    assert!(
        ListUnlinkSourceTask::derive(&import, "mk_list.c", "", NODE_QCORE_TYPE, base.clone())
            .is_err()
    );
    assert!(ListUnlinkSourceTask::derive(&import, "mk_list.c", UNLINK_CONTRACT, "", base).is_err());

    // The destructor's projected inventory must be the contract's own.
    assert_eq!(
        declared_owned_fields(DESTRUCTOR_CONTRACT, DESTRUCTOR_FUNCTION).unwrap(),
        OWNED_FIELDS.iter().map(|f| (*f).to_owned()).collect()
    );
    for fields in [
        vec!["backend"],
        vec![
            "backend",
            "backend__buf_data",
            "backend__meta_data",
            "backend__name",
            "backend__extra",
        ],
        vec!["backend", "backend", "backend__buf_data", "backend__name"],
    ] {
        let FieldDestructorSourceResult::Import(import) =
            destructor_import_result(DESTRUCTOR_SOURCE, vec![], &fields)
        else {
            // A projection the importer already refuses is equally fine.
            continue;
        };
        assert!(
            FieldDestructorSourceTask::derive(
                &import,
                "cio_memfs.c",
                DESTRUCTOR_CONTRACT,
                ROOT_QCORE_TYPE,
                destructor_target(&import),
            )
            .is_err(),
            "{fields:?} was accepted as the contract's inventory"
        );
    }
}

#[test]
fn changed_inputs_produce_different_tasks_and_cross_task_witnesses_refuse() {
    if !clang_available() {
        return;
    }
    let import = unlink_import(UNLINK_SOURCE, vec![]);
    let task = unlink_task(&import, UNLINK_CONTRACT);
    let reading = unlink_reading(&task);
    for other in [
        unlink_task(
            &unlink_import(
                &format!("// changed actual source\n{UNLINK_SOURCE}"),
                vec![],
            ),
            UNLINK_CONTRACT,
        ),
        unlink_task(
            &unlink_import(UNLINK_SOURCE, vec!["QCORE_OTHER_BUILD=1".into()]),
            UNLINK_CONTRACT,
        ),
        unlink_task(
            &import,
            &UNLINK_CONTRACT.replace("requires entry_is_live;", "requires entry_is_live ;"),
        ),
    ] {
        assert_ne!(other.task().task_digest(), task.task().task_digest());
        // Neither direction admits the other's witness.
        assert!(
            other
                .check(
                    other.task(),
                    &witness(&task.task().task_digest(), reading.clone())
                )
                .is_err()
        );
        assert!(
            task.check(
                task.task(),
                &witness(&other.task().task_digest(), unlink_reading(&other))
            )
            .is_err()
        );
    }
    // A different declared QCore node type is a different question.
    let renamed = ListUnlinkSourceTask::derive(
        &import,
        "mk_list.c",
        &UNLINK_CONTRACT.replace("ListNode", "OtherNode"),
        "OtherNode",
        unlink_target(&import),
    )
    .unwrap();
    assert_ne!(renamed.task().task_digest(), task.task().task_digest());
}

#[test]
fn unmodeled_bodies_keep_their_functions_unknown_at_import() {
    if !clang_available() {
        return;
    }
    // A call the unlink family does not admit, a helper with the wrong arity,
    // and an entry that returns a value all lose the projection at import.
    for source in [
        UNLINK_SOURCE
            .replace(
                "static inline void mk_list_del",
                "void mk_list_touch(struct mk_list *); static inline void mk_list_del",
            )
            .replace(
                "entry->prev=(void *)0;",
                "mk_list_touch(entry); entry->prev=(void *)0;",
            ),
        // A helper with a third parameter is not the declared two-cell
        // helper, so the pair cannot be reassociated and nothing projects.
        "struct mk_list { struct mk_list *prev; struct mk_list *next; }; \
         static inline void __mk_list_del(struct mk_list *prev, struct mk_list *next, int flag) \
         { (void)flag; prev->next=next; next->prev=prev; } \
         static inline void mk_list_del(struct mk_list *entry) \
         { __mk_list_del(entry->prev,entry->next,0); entry->prev=(void *)0; entry->next=(void *)0; }"
            .to_owned(),
    ] {
        let result = import_list_unlink_sources(
            &qcore_c_gateway::legacy_c::C11TranslationUnit {
                source: &source,
                ..Default::default()
            },
            &revision(),
            UNLINK_ENTRY,
            UNLINK_HELPER,
            &["mk_list"],
            NODE_C_TYPE,
        )
        .unwrap();
        assert!(
            matches!(result, ListUnlinkSourceResult::Unknown { .. }),
            "an unmodeled unlink body was projected"
        );
    }
    // Naming one function twice, or an empty node type, refuse outright.
    assert!(
        import_list_unlink_sources(
            &qcore_c_gateway::legacy_c::C11TranslationUnit {
                source: UNLINK_SOURCE,
                ..Default::default()
            },
            &revision(),
            UNLINK_ENTRY,
            UNLINK_ENTRY,
            &["mk_list"],
            NODE_C_TYPE,
        )
        .is_err()
    );
    assert!(
        import_list_unlink_sources(
            &qcore_c_gateway::legacy_c::C11TranslationUnit {
                source: UNLINK_SOURCE,
                ..Default::default()
            },
            &revision(),
            UNLINK_ENTRY,
            UNLINK_HELPER,
            &["mk_list"],
            "",
        )
        .is_err()
    );
    // The destructor family admits `free` and nothing else.
    let result = import_field_destructor_sources(
        &qcore_c_gateway::legacy_c::C11TranslationUnit {
            source: &DESTRUCTOR_SOURCE
                .replace(
                    "void free(void *);",
                    "void free(void *); void cio_free(void *);",
                )
                .replace("free(mf->name);", "cio_free(mf->name);"),
            ..Default::default()
        },
        &revision(),
        &destructor_selection(&OWNED_FIELDS),
    )
    .unwrap();
    assert!(matches!(
        result,
        FieldDestructorSourceResult::Unknown { .. }
    ));
    assert!(
        import_field_destructor_sources(
            &qcore_c_gateway::legacy_c::C11TranslationUnit {
                source: DESTRUCTOR_SOURCE,
                ..Default::default()
            },
            &revision(),
            &destructor_selection(&[]),
        )
        .is_err()
    );
}

#[test]
fn a_projection_carries_exactly_one_obligation_of_its_own_family() {
    if !clang_available() {
        return;
    }
    let plugin = qcore_schemas::PluginId::new("qcore.chunkio").unwrap();
    let unlink = unlink_task(&unlink_import(UNLINK_SOURCE, vec![]), UNLINK_CONTRACT);
    let destructor = destructor_task(
        &destructor_import(DESTRUCTOR_SOURCE, vec![]),
        DESTRUCTOR_CONTRACT,
    );
    let request = unlink.request(plugin.clone());
    request.validate().unwrap();
    assert_eq!(request.obligations.len(), 1);
    assert_eq!(
        request.obligations[0].property,
        qcore_schemas::obligation::ObligationProperty::ListUnlinkContract
    );
    // The destructor family's obligation cannot carry an unlink projection.
    let mut crossed = request.clone();
    crossed.obligations = destructor.request(plugin.clone()).obligations;
    assert!(crossed.validate().is_err());
    // Two projections never travel together.
    let mut both = request.clone();
    both.counter_effect = serde_json::from_str(
        r#"{"schema_version":1,"function":"mk_list_del","source_revision":"x",
            "signature":"unsigned long (struct cio_ctx *)","root_c_type":"struct cio_ctx *",
            "operations":[],"profile_assumptions":{}}"#,
    )
    .unwrap();
    assert!(both.validate().is_err());
    // An unsupported projection version refuses in both directions.
    let mut moved = request.clone();
    moved.teardown_source.as_mut().unwrap().schema_version = 2;
    assert!(moved.validate().is_err());
    let mut absent = request;
    absent.teardown_source = None;
    assert!(absent.validate().is_err());
    // The two families are distinct questions with distinct tasks.
    assert_ne!(unlink.task().task_digest(), destructor.task().task_digest());
    assert_ne!(
        unlink.task().semantic_profile,
        destructor.task().semantic_profile
    );
    assert_eq!(unlink.task().semantic_profile, LIST_UNLINK_SEMANTIC_PROFILE);
    // The unlink task names both checked bodies in its obligation scope.
    let qcore_schemas::obligation::ObligationScope::Functions { symbols } =
        &unlink.task().obligations[0].scope
    else {
        panic!("function scope")
    };
    assert!(symbols.contains(UNLINK_ENTRY) && symbols.contains(UNLINK_HELPER));
    // A destructor family projection cannot be read under the unlink family.
    assert!(matches!(
        destructor.request(plugin).teardown_source.unwrap().family,
        TeardownFamilyInput::FieldDestructor { .. }
    ));
}
