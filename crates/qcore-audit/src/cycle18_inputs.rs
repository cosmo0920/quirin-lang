//! In-memory inputs from fresh configured audits, never deserialized reports.
use qcore_c_gateway::typed_source::SourceSummary;
use qcore_schemas::sha256_hex;
use qcore_verification::event_composition_context::ValidatedEventCompositionContext;
use qcore_verification::event_payload::{EventPayloadProjection, EventPayloadWriteAuthority};
use qcore_verification::monkey_wait_projection::MonkeyWaitSourceProjection;
use qcore_verification::timeout_teardown::{
    TimeoutTeardownEvidence, TimeoutTeardownSourceProjection,
};
use std::path::PathBuf;

pub(crate) struct SourceIdentity {
    pub revision: String,
    pub compile_task: String,
    pub source: String,
    pub profile: String,
    pub build: String,
}

impl SourceIdentity {
    pub fn from_source(
        source: &crate::observed_wait_inputs::WaitSourceInputs,
    ) -> Result<Self, String> {
        Ok(Self {
            revision: source.revision.clone(),
            compile_task: source.compile_task.digest()?,
            source: source.source_sha256.clone(),
            profile: source.profile.manifest_digest().into(),
            build: source.build_sha256.clone(),
        })
    }
}

pub(crate) struct ObservedWait {
    pub projection: MonkeyWaitSourceProjection,
    pub context: ValidatedEventCompositionContext,
    pub observations: Vec<u8>,
    pub bindings: Vec<u8>,
    pub summaries: Vec<SourceSummary>,
}

pub(crate) struct Payload {
    pub projection: EventPayloadProjection,
    pub context: ValidatedEventCompositionContext,
    pub observations: Vec<u8>,
    pub bindings: Vec<u8>,
    pub allocation: Vec<u8>,
    pub authority: EventPayloadWriteAuthority,
    pub summaries: Vec<SourceSummary>,
}

pub(crate) struct Timeout {
    pub projection: TimeoutTeardownSourceProjection,
    pub evidence: TimeoutTeardownEvidence,
    pub summaries: Vec<SourceSummary>,
}

pub(crate) struct Failures {
    pub observations: Vec<u8>,
}

pub(crate) struct PreparedAudit<T> {
    pub source_identity: SourceIdentity,
    pub evidence: T,
    report: PathBuf,
    report_bytes: Vec<u8>,
    pub protected_files: Vec<PathBuf>,
    pub protected_directories: Vec<PathBuf>,
    snapshots: Vec<(PathBuf, String)>,
}

impl<T> PreparedAudit<T> {
    pub fn new(
        source_identity: SourceIdentity,
        evidence: T,
        report: PathBuf,
        report_bytes: Vec<u8>,
        protected_files: Vec<PathBuf>,
        protected_directories: Vec<PathBuf>,
    ) -> Result<Self, String> {
        let snapshots = protected_files
            .iter()
            .map(|path| {
                std::fs::read(path)
                    .map(|bytes| (path.clone(), sha256_hex(&bytes)))
                    .map_err(|error| format!("{}: {error}", path.display()))
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self {
            source_identity,
            evidence,
            report,
            report_bytes,
            protected_files,
            protected_directories,
            snapshots,
        })
    }

    pub fn recheck(&self) -> Result<(), String> {
        for (path, expected) in &self.snapshots {
            let actual = std::fs::read(path).map_err(|error| error.to_string())?;
            if sha256_hex(&actual) != *expected {
                return Err(format!("prepared audit input changed: {}", path.display()));
            }
        }
        Ok(())
    }

    pub fn publish(&self) -> Result<(), String> {
        self.recheck()?;
        crate::observed_wait::reject_report_aliases(
            &self.report,
            &self.protected_files,
            &self.protected_directories,
        )?;
        crate::wait_failures::write_atomic(&self.report, &self.report_bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT: AtomicU64 = AtomicU64::new(0);

    fn identity() -> SourceIdentity {
        SourceIdentity {
            revision: String::new(),
            compile_task: String::new(),
            source: String::new(),
            profile: String::new(),
            build: String::new(),
        }
    }

    fn directory() -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "qcore-prepared-cycle18-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed),
        ));
        std::fs::create_dir(&path).unwrap();
        path
    }

    #[test]
    fn changed_evidence_between_preparation_and_publication_refuses() {
        let root = directory();
        let input = root.join("input");
        let output = root.join("output");
        std::fs::write(&input, b"verified evidence").unwrap();
        let audit = PreparedAudit::new(
            identity(),
            (),
            output.clone(),
            b"report".to_vec(),
            vec![input.clone()],
            vec![],
        )
        .unwrap();
        std::fs::write(&input, b"replacement").unwrap();
        assert!(audit.publish().unwrap_err().contains("input changed"));
        assert!(!output.exists());
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn prepared_report_cannot_replace_its_input() {
        let root = directory();
        let input = root.join("input");
        std::fs::write(&input, b"verified evidence").unwrap();
        let audit = PreparedAudit::new(
            identity(),
            (),
            input.clone(),
            b"report".to_vec(),
            vec![input.clone()],
            vec![],
        )
        .unwrap();
        assert!(audit.publish().unwrap_err().contains("aliases"));
        assert_eq!(std::fs::read(&input).unwrap(), b"verified evidence");
        std::fs::remove_dir_all(root).unwrap();
    }
}
