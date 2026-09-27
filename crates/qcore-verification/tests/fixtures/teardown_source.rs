//! Shared inputs for the two process-transported ChunkIO teardown families.
//!
//! The C fixtures are the ones the existing in-process unit tests import, so
//! both routes read the same bodies. Nothing here grants any premise: the
//! declared revision is the profile pin and every contract premise stays the
//! pinned contract's own.
use qcore_c_gateway::legacy_c::C11TranslationUnit;
use qcore_c_gateway::teardown_source::{
    FieldDestructorSelection, FieldDestructorSourceImport, FieldDestructorSourceResult,
    ListUnlinkSourceImport, ListUnlinkSourceResult, import_field_destructor_sources,
    import_list_unlink_sources,
};
use qcore_contract::profile_loader::load_profile_directory;
use qcore_schemas::TargetProfile;
use qcore_verification::teardown_source::{
    FIELD_DESTRUCTOR_SEMANTIC_PROFILE, FieldDestructorSourceTask, LIST_UNLINK_SEMANTIC_PROFILE,
    ListUnlinkSourceTask,
};
use std::path::PathBuf;

pub const UNLINK_ENTRY: &str = "mk_list_del";
pub const UNLINK_HELPER: &str = "__mk_list_del";
pub const NODE_C_TYPE: &str = "struct mk_list *";
pub const NODE_QCORE_TYPE: &str = "ListNode";
pub const UNLINK_SOURCE: &str = "struct mk_list { struct mk_list *prev; struct mk_list *next; }; static inline void __mk_list_del(struct mk_list *prev, struct mk_list *next) { prev->next=next; next->prev=prev; } static inline void mk_list_del(struct mk_list *entry) { __mk_list_del(entry->prev,entry->next); entry->prev=(void *)0; entry->next=(void *)0; }";
pub const UNLINK_CONTRACT: &str = include_str!(
    "../../../../providers/qcore-provider-chunkio/contracts/chunkio/list_unlink.qv"
);

pub const DESTRUCTOR_FUNCTION: &str = "cio_memfs_close";
pub const ROOT_C_TYPE: &str = "struct cio_chunk *";
pub const ROOT_QCORE_TYPE: &str = "Chunk";
pub const DESTRUCTOR_SOURCE: &str = "
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
pub const DESTRUCTOR_CONTRACT: &str = include_str!(
    "../../../../providers/qcore-provider-chunkio/contracts/chunkio/memory_backend.qv"
);
/// The inventory `memory_backend.qcore` declares, in the order the importer
/// is handed it. `derive` refuses a projection that names any other set.
pub const OWNED_FIELDS: [&str; 4] = [
    "backend",
    "backend__buf_data",
    "backend__meta_data",
    "backend__name",
];

pub fn clang_available() -> bool {
    match qcore_c_gateway::clang_command() {
        Ok(_) => true,
        Err(reason) => {
            assert!(
                std::env::var_os("QCORE_CLANG").is_none(),
                "explicit QCORE_CLANG is invalid: {reason}"
            );
            eprintln!("skipping Clang-dependent teardown test: {reason}");
            false
        }
    }
}

/// The declared source pin, read from the same profile the other routes use.
pub fn revision() -> String {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    load_profile_directory(&root.join("contract/profiles/fluent-bit"))
        .unwrap()
        .pin_for("chunkio")
        .unwrap()
        .git_commit
        .clone()
}

pub fn unlink_import_result(source: &str, definitions: Vec<String>) -> ListUnlinkSourceResult {
    import_list_unlink_sources(
        &C11TranslationUnit {
            source,
            definitions,
            ..Default::default()
        },
        &revision(),
        UNLINK_ENTRY,
        UNLINK_HELPER,
        &["mk_list"],
        NODE_C_TYPE,
    )
    .unwrap()
}

pub fn unlink_import(source: &str, definitions: Vec<String>) -> ListUnlinkSourceImport {
    match unlink_import_result(source, definitions) {
        ListUnlinkSourceResult::Import(import) => *import,
        other => panic!("missing typed unlink import: {other:?}"),
    }
}

pub fn unlink_target(import: &ListUnlinkSourceImport) -> TargetProfile {
    TargetProfile {
        library: "chunkio".into(),
        revision: import.source_revision().into(),
        profile: LIST_UNLINK_SEMANTIC_PROFILE.into(),
        dialect: "c11".into(),
        platform: std::env::consts::OS.into(),
        backend: "native".into(),
    }
}

pub fn unlink_task(import: &ListUnlinkSourceImport, contract: &str) -> ListUnlinkSourceTask {
    ListUnlinkSourceTask::derive(
        import,
        "mk_list.c",
        contract,
        NODE_QCORE_TYPE,
        unlink_target(import),
    )
    .unwrap()
}

pub fn destructor_import_result(
    source: &str,
    definitions: Vec<String>,
    fields: &[&str],
) -> FieldDestructorSourceResult {
    import_field_destructor_sources(
        &C11TranslationUnit {
            source,
            definitions,
            ..Default::default()
        },
        &revision(),
        &destructor_selection(fields),
    )
    .unwrap()
}

/// The selection the in-process `check_memory_backend_source` route uses, so
/// both routes import exactly the same body under the same profile.
pub fn destructor_selection<'a>(fields: &'a [&'a str]) -> FieldDestructorSelection<'a> {
    FieldDestructorSelection {
        function: DESTRUCTOR_FUNCTION,
        records: &["cio_chunk", "cio_memfs"],
        signature_constraints: &[(DESTRUCTOR_FUNCTION, "void", &["struct cio_chunk *"])],
        restricted_calls: &["free"],
        root_c_type: ROOT_C_TYPE,
        declared_owned_fields: fields,
    }
}

pub fn destructor_import(source: &str, definitions: Vec<String>) -> FieldDestructorSourceImport {
    match destructor_import_result(source, definitions, &OWNED_FIELDS) {
        FieldDestructorSourceResult::Import(import) => *import,
        other => panic!("missing typed destructor import: {other:?}"),
    }
}

pub fn destructor_target(import: &FieldDestructorSourceImport) -> TargetProfile {
    TargetProfile {
        library: "chunkio".into(),
        revision: import.source_revision().into(),
        profile: FIELD_DESTRUCTOR_SEMANTIC_PROFILE.into(),
        dialect: "c11".into(),
        platform: std::env::consts::OS.into(),
        backend: "native".into(),
    }
}

pub fn destructor_task(
    import: &FieldDestructorSourceImport,
    contract: &str,
) -> FieldDestructorSourceTask {
    FieldDestructorSourceTask::derive(
        import,
        "cio_memfs.c",
        contract,
        ROOT_QCORE_TYPE,
        destructor_target(import),
    )
    .unwrap()
}
