use std::collections::BTreeMap;

use super::*;

fn bundle(contracts: impl IntoIterator<Item = SourceContract>) -> ExternalSourceBundle {
    let mut files = BTreeMap::<String, String>::new();
    for contract in contracts {
        let contents = files.entry(contract.file.to_owned()).or_default();
        for fragment in contract.fragments {
            contents.push_str(fragment);
            contents.push('\n');
        }
    }
    ExternalSourceBundle {
        source_revision: PINNED_FLUENT_BIT_REVISION.to_owned(),
        files,
    }
}

#[test]
fn derives_only_the_audited_librdkafka_mechanisms() {
    let source = bundle(librdkafka_contracts());
    let certificate =
        derive_library_summary_profile(AuditedLibrary::Librdkafka2150, &source).unwrap();
    assert_eq!(certificate.checked_files.len(), 3);
    assert_eq!(certificate.checked_contracts.len(), 3);
    assert!(
        certificate
            .profile
            .summaries
            .contains(&LibraryMechanismSummary::ReferenceCount)
    );
    assert!(
        certificate
            .profile
            .summaries
            .contains(&LibraryMechanismSummary::TypedQueue)
    );
    assert!(
        certificate
            .profile
            .summaries
            .contains(&LibraryMechanismSummary::ThreadHandoff)
    );
    assert!(
        !certificate
            .profile
            .summaries
            .contains(&LibraryMechanismSummary::AtomicPublication)
    );
    assert!(
        !certificate
            .profile
            .summaries
            .contains(&LibraryMechanismSummary::Reclamation)
    );
}

#[test]
fn derives_cfl_arena_and_chunkio_nested_teardown_profiles() {
    let cfl =
        derive_library_summary_profile(AuditedLibrary::Cfl, &bundle(cfl_contracts())).unwrap();
    assert_eq!(
        cfl.profile.summaries,
        BTreeSet::from([
            LibraryMechanismSummary::IntrusiveList,
            LibraryMechanismSummary::Arena,
            LibraryMechanismSummary::CascadingDestructor,
        ])
    );

    let chunk =
        derive_library_summary_profile(AuditedLibrary::ChunkIo, &bundle(chunk_io_contracts()))
            .unwrap();
    assert_eq!(
        chunk.profile.summaries,
        BTreeSet::from([
            LibraryMechanismSummary::IntrusiveList,
            LibraryMechanismSummary::CascadingDestructor,
        ])
    );
}

#[test]
fn source_profiles_fail_closed_on_revision_file_or_contract_drift() {
    let mut source = bundle(librdkafka_contracts());
    source.source_revision = "unpinned".into();
    assert!(
        derive_library_summary_profile(AuditedLibrary::Librdkafka2150, &source)
            .unwrap_err()
            .contains(&LibrarySourceDiagnostic::SourceRevisionMismatch)
    );

    let mut source = bundle(cfl_contracts());
    source.files.remove(CFL_FILES[0]);
    assert!(
        derive_library_summary_profile(AuditedLibrary::Cfl, &source)
            .unwrap_err()
            .contains(&LibrarySourceDiagnostic::MissingFile(CFL_FILES[0].into()))
    );

    let mut source = bundle(chunk_io_contracts());
    source
        .files
        .get_mut(CHUNK_IO_FILES[2])
        .unwrap()
        .retain(|character| character != '_');
    assert!(
        derive_library_summary_profile(AuditedLibrary::ChunkIo, &source)
            .unwrap_err()
            .contains(&LibrarySourceDiagnostic::ContractMismatch {
                file: CHUNK_IO_FILES[2].into(),
                contract: "chunkio-chunk-destructor".into(),
            })
    );
}
