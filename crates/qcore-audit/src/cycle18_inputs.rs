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
use std::process::Command;

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
    pub projection: MonkeyWaitSourceProjection,
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
    checkout_revisions: Vec<(PathBuf, String)>,
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
            checkout_revisions: Vec::new(),
        })
    }

    pub fn with_checkout_revision(mut self, checkout: PathBuf, revision: String) -> Self {
        self.checkout_revisions.push((checkout, revision));
        self
    }

    pub fn report(&self) -> &std::path::Path {
        &self.report
    }

    pub fn recheck(&self) -> Result<(), String> {
        for (path, expected) in &self.snapshots {
            let actual = std::fs::read(path).map_err(|error| error.to_string())?;
            if sha256_hex(&actual) != *expected {
                return Err(format!("prepared audit input changed: {}", path.display()));
            }
        }
        for (checkout, expected) in &self.checkout_revisions {
            let output = Command::new("git")
                .arg("-C")
                .arg(checkout)
                .args(["rev-parse", "--verify", "HEAD^{commit}"])
                .output()
                .map_err(|error| format!("{}: {error}", checkout.display()))?;
            if !output.status.success()
                || String::from_utf8(output.stdout)
                    .map_err(|error| error.to_string())?
                    .trim()
                    != expected
            {
                return Err(format!(
                    "prepared audit checkout revision changed: {}",
                    checkout.display()
                ));
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

    #[test]
    fn changed_checkout_revision_refuses_even_when_selected_bytes_are_unchanged() {
        let root = directory();
        let input = root.join("input");
        let output = root.join("output");
        std::fs::write(&input, b"verified evidence").unwrap();
        let git = |arguments: &[&str]| {
            let status = Command::new("git")
                .args(arguments)
                .current_dir(&root)
                .status()
                .unwrap();
            assert!(status.success());
        };
        git(&["init", "-q"]);
        git(&["add", "input"]);
        git(&[
            "-c",
            "user.name=QCore Test",
            "-c",
            "user.email=qcore@example.invalid",
            "commit",
            "-qm",
            "initial",
        ]);
        let revision = String::from_utf8(
            Command::new("git")
                .args(["rev-parse", "HEAD"])
                .current_dir(&root)
                .output()
                .unwrap()
                .stdout,
        )
        .unwrap()
        .trim()
        .to_owned();
        let audit = PreparedAudit::new(
            identity(),
            (),
            output.clone(),
            b"report".to_vec(),
            vec![input],
            vec![],
        )
        .unwrap()
        .with_checkout_revision(root.clone(), revision);
        git(&[
            "-c",
            "user.name=QCore Test",
            "-c",
            "user.email=qcore@example.invalid",
            "commit",
            "--allow-empty",
            "-qm",
            "identity changed",
        ]);
        assert!(audit.recheck().unwrap_err().contains("revision changed"));
        assert!(!output.exists());
        std::fs::remove_dir_all(root).unwrap();
    }
}
