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

//! Old in-process teardown routes versus the new process-transported ones.
//!
//! Both routes import the same typed bodies and evaluate the same pinned
//! contracts in the trusted core. The archived parity is certificate equality
//! on accepted functions and refusal agreement on rejected ones; the
//! in-process adapters are retained, not replaced, by this increment.
use qcore_compiler::field_destructor::FieldDestructorCertificate;
use qcore_compiler::legacy_c::C11TranslationUnit;
use qcore_compiler::list_unlink_contract::ListUnlinkCertificate;
use qcore_compiler::teardown_source_capability::{
    FIELD_DESTRUCTOR_SEMANTIC_PROFILE, FieldDestructorSelection, FieldDestructorSourceResult,
    FieldDestructorSourceTask, LIST_UNLINK_SEMANTIC_PROFILE, ListUnlinkSourceResult,
    ListUnlinkSourceTask, import_field_destructor_sources, import_list_unlink_sources,
};
use qcore_compiler::verifier_plugins::chunkio::{
    PINNED_FLUENT_BIT_REVISION,
    teardown_source::{
        LIST_UNLINK_CONTRACT, MEMORY_BACKEND_CONTRACT, check_list_unlink_source,
        check_memory_backend_source,
    },
};
use qcore_schemas::TargetProfile;

const NODE_C_TYPE: &str = "struct mk_list *";
const NODE_QCORE_TYPE: &str = "ListNode";
const UNLINK_SOURCE: &str = "struct mk_list { struct mk_list *prev; struct mk_list *next; }; static inline void __mk_list_del(struct mk_list *prev, struct mk_list *next) { prev->next=next; next->prev=prev; } static inline void mk_list_del(struct mk_list *entry) { __mk_list_del(entry->prev,entry->next); entry->prev=(void *)0; entry->next=(void *)0; }";

const ROOT_C_TYPE: &str = "struct cio_chunk *";
const ROOT_QCORE_TYPE: &str = "Chunk";
const OWNED_FIELDS: [&str; 4] = [
    "backend",
    "backend__buf_data",
    "backend__meta_data",
    "backend__name",
];
const DESTRUCTOR_SOURCE: &str = "
struct cio_memfs { char *name; char *buf_data; char *meta_data; };
struct cio_chunk { void *backend; };
void free(void *);
void cio_memfs_close(struct cio_chunk *ch) {
    struct cio_memfs *mf = ch->backend;
    if (!mf) { return; }
    free(mf->name);
    free(mf->buf_data);
    free(mf->meta_data);
    free(mf);
}";

fn clang_available() -> bool {
    match qcore_c_gateway::clang_command() {
        Ok(_) => true,
        Err(reason) => {
            assert!(
                std::env::var_os("QCORE_CLANG").is_none(),
                "explicit QCORE_CLANG is invalid: {reason}"
            );
            eprintln!("skipping Clang-dependent teardown parity test: {reason}");
            false
        }
    }
}

fn unit(source: &str) -> C11TranslationUnit<'_> {
    C11TranslationUnit {
        source,
        ..Default::default()
    }
}

fn target(profile: &str) -> TargetProfile {
    TargetProfile {
        library: "chunkio".into(),
        revision: PINNED_FLUENT_BIT_REVISION.into(),
        profile: profile.into(),
        dialect: "c11".into(),
        platform: std::env::consts::OS.into(),
        backend: "native".into(),
    }
}

fn unlink_route(source: &str) -> ListUnlinkSourceResult {
    import_list_unlink_sources(
        &unit(source),
        PINNED_FLUENT_BIT_REVISION,
        "mk_list_del",
        "__mk_list_del",
        &["mk_list"],
        NODE_C_TYPE,
    )
    .unwrap()
}

fn destructor_route(source: &str) -> FieldDestructorSourceResult {
    import_field_destructor_sources(
        &unit(source),
        PINNED_FLUENT_BIT_REVISION,
        // Exactly the selection `check_memory_backend_source` passes.
        &FieldDestructorSelection {
            function: "cio_memfs_close",
            records: &["cio_chunk", "cio_memfs"],
            signature_constraints: &[("cio_memfs_close", "void", &["struct cio_chunk *"])],
            restricted_calls: &["free"],
            root_c_type: ROOT_C_TYPE,
            declared_owned_fields: &OWNED_FIELDS,
        },
    )
    .unwrap()
}

#[test]
fn both_unlink_routes_produce_the_identical_certificate() {
    if !clang_available() {
        return;
    }
    let old: ListUnlinkCertificate =
        check_list_unlink_source(&unit(UNLINK_SOURCE), PINNED_FLUENT_BIT_REVISION).unwrap();
    let ListUnlinkSourceResult::Import(import) = unlink_route(UNLINK_SOURCE) else {
        panic!("the unlink lost its typed import on the new route")
    };
    let task = ListUnlinkSourceTask::derive(
        &import,
        "mk_list.c",
        LIST_UNLINK_CONTRACT,
        NODE_QCORE_TYPE,
        target(LIST_UNLINK_SEMANTIC_PROFILE),
    )
    .unwrap();
    let recomputed = task.evaluate().unwrap();
    assert_eq!(
        serde_json::to_value(&old).unwrap(),
        serde_json::to_value(&recomputed).unwrap()
    );
    // Both neighbor-alias classes and all four writes survive the transport.
    assert_eq!(recomputed.cases().len(), 2);
    assert_eq!(
        recomputed.cases()[0].writes,
        vec![
            (2, "next".into(), Some(3)),
            (3, "prev".into(), Some(2)),
            (1, "prev".into(), None),
            (1, "next".into(), None),
        ]
    );
    assert!(recomputed.cases().iter().all(|case| case.helper_calls == 1));
}

#[test]
fn both_destructor_routes_produce_the_identical_certificate() {
    if !clang_available() {
        return;
    }
    let old: FieldDestructorCertificate =
        check_memory_backend_source(&unit(DESTRUCTOR_SOURCE), PINNED_FLUENT_BIT_REVISION).unwrap();
    let FieldDestructorSourceResult::Import(import) = destructor_route(DESTRUCTOR_SOURCE) else {
        panic!("the destructor lost its typed import on the new route")
    };
    let task = FieldDestructorSourceTask::derive(
        &import,
        "cio_memfs.c",
        MEMORY_BACKEND_CONTRACT,
        ROOT_QCORE_TYPE,
        target(FIELD_DESTRUCTOR_SEMANTIC_PROFILE),
    )
    .unwrap();
    let recomputed = task.evaluate().unwrap();
    assert_eq!(
        serde_json::to_value(&old).unwrap(),
        serde_json::to_value(&recomputed).unwrap()
    );
    assert_eq!(recomputed.cases().len(), 9);
    assert_eq!(
        recomputed.cases().last().unwrap().released_in_order,
        [
            "backend__name",
            "backend__buf_data",
            "backend__meta_data",
            "backend"
        ]
    );
}

#[test]
fn both_routes_refuse_the_same_changed_unlink_sources() {
    if !clang_available() {
        return;
    }
    for (old_text, new_text) in [
        // The helper relinks the wrong neighbor.
        ("prev->next=next;", "prev->next=prev;"),
        // The entry forgets to null one of its own link cells.
        ("entry->next=(void *)0;", ""),
        // The entry writes a neighbor cell the helper already owns.
        (
            "entry->prev=(void *)0;",
            "entry->prev->next=entry; entry->prev=(void *)0;",
        ),
        // The helper is never called.
        ("__mk_list_del(entry->prev,entry->next);", ""),
    ] {
        let changed = UNLINK_SOURCE.replace(old_text, new_text);
        assert!(
            check_list_unlink_source(&unit(&changed), PINNED_FLUENT_BIT_REVISION).is_err(),
            "old route accepted `{new_text}`"
        );
        // The new route either loses the projection at import or refuses the
        // contract at replay; in neither case can a witness discharge it.
        let refused = match unlink_route(&changed) {
            ListUnlinkSourceResult::Unknown { .. } => true,
            ListUnlinkSourceResult::Import(import) => match ListUnlinkSourceTask::derive(
                &import,
                "mk_list.c",
                LIST_UNLINK_CONTRACT,
                NODE_QCORE_TYPE,
                target(LIST_UNLINK_SEMANTIC_PROFILE),
            ) {
                Ok(task) => task.evaluate().is_err(),
                Err(_) => true,
            },
        };
        assert!(refused, "new route accepted `{new_text}`");
    }
}

#[test]
fn both_routes_refuse_the_same_changed_destructor_sources() {
    if !clang_available() {
        return;
    }
    for (old_text, new_text) in [
        // A declared owned field is retained.
        ("free(mf->meta_data);", ""),
        // The backend itself is retained.
        ("free(mf);", ""),
        // The parent is released before a live child.
        (
            "free(mf->name);\n    free(mf->buf_data);",
            "free(mf);\n    free(mf->buf_data);",
        ),
        // One field is released twice.
        ("free(mf->buf_data);", "free(mf->name); free(mf->buf_data);"),
    ] {
        let changed = DESTRUCTOR_SOURCE.replace(old_text, new_text);
        assert_ne!(changed, DESTRUCTOR_SOURCE, "`{new_text}` did not apply");
        assert!(
            check_memory_backend_source(&unit(&changed), PINNED_FLUENT_BIT_REVISION).is_err(),
            "old route accepted `{new_text}`"
        );
        let refused = match destructor_route(&changed) {
            FieldDestructorSourceResult::Unknown { .. } => true,
            FieldDestructorSourceResult::Import(import) => match FieldDestructorSourceTask::derive(
                &import,
                "cio_memfs.c",
                MEMORY_BACKEND_CONTRACT,
                ROOT_QCORE_TYPE,
                target(FIELD_DESTRUCTOR_SEMANTIC_PROFILE),
            ) {
                Ok(task) => task.evaluate().is_err(),
                Err(_) => true,
            },
        };
        assert!(refused, "new route accepted `{new_text}`");
    }
}
