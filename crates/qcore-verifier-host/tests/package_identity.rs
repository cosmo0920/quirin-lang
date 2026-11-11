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

//! Adversarial fixtures for the time-of-check to time-of-use window between
//! package verification and dispatch (card `P01-F3`).
//!
//! P01-F2 bound an installed manifest to the template embedded in the
//! executable's bytes, and recorded the residual honestly: *"package files may
//! change between verification and dispatch"*. These cases substitute the
//! executable after a package has verified — in place, by replacement, and by
//! removal — and require the next spawn to refuse rather than run whatever is
//! there now.
//!
//! Every case measures that the substituted package **never ran**: each
//! replacement writes a marker into the package directory, so a spawn that
//! happened at all is visible, not merely inferred from the returned error.
#![cfg(unix)]

use qcore_verifier_host::*;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

// Only `write_executable` is used here: these fixtures already read their
// request with `/bin/cat > /dev/null` before answering.
#[allow(dead_code)]
#[path = "support/process_fixture.rs"]
mod process_fixture;
use std::time::{SystemTime, UNIX_EPOCH};

struct Fixture {
    root: PathBuf,
}

impl Fixture {
    fn new(tag: &str) -> Self {
        let root = std::env::temp_dir().join(format!(
            "qcore-p01f3-{tag}-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir(&root).unwrap();
        Self { root }
    }

    fn package(&self) -> &std::path::Path {
        &self.root
    }

    fn executable(&self) -> PathBuf {
        self.root.join("provider.sh")
    }

    /// A marker only a *spawned* package can leave. Its absence separates
    /// "refused before spawning" from "spawned and then rejected".
    fn ran_marker(&self) -> PathBuf {
        self.root.join("ran.marker")
    }

    fn response(&self, request: &VerificationRequest) {
        let response = VerificationResponse {
            protocol_version: request.protocol_version,
            request_id: request.request_id.clone(),
            plugin_id: request.plugin_id.clone(),
            verdict: ProposedVerdict::Unknown,
            diagnostics: vec![],
            evidence: None,
        };
        fs::write(
            self.root.join("response.bin"),
            encode_protocol_frame(&response, DEFAULT_MAX_FRAME_BYTES).unwrap(),
        )
        .unwrap();
    }

    /// Writes `body` as the package executable and installs the package so it
    /// verifies: the manifest template follows an `exit`, which `sh` never
    /// parses past, and the installed manifest carries the file's real digest.
    fn install(&self, body: &str) -> PluginManifest {
        let template = manifest();
        let mut bytes = format!("#!/bin/sh\n{body}\nexit $?\n").into_bytes();
        bytes.extend(envelope(&serde_json::to_string(&template).unwrap()).into_bytes());
        bytes.push(b'\n');
        self.write_executable(&bytes);
        let mut installed = template;
        installed.package_digest = sha256_hex(&fs::read(self.executable()).unwrap());
        fs::write(
            self.root.join("verifier-plugin.json"),
            serde_json::to_vec(&installed).unwrap(),
        )
        .unwrap();
        installed
    }

    /// Written without a write descriptor in this process, so a sibling
    /// test's concurrent spawn cannot make this file's `execve` fail with
    /// `ETXTBSY` (`support/process_fixture.rs`). An existing file keeps its
    /// inode, which `rewrite_in_place` asserts.
    fn write_executable(&self, bytes: &[u8]) {
        process_fixture::write_executable(&self.executable(), bytes);
    }

    /// Rewrites the executable **in place**: `write` truncates the same inode,
    /// so the path still names the verified file and only its content moved.
    fn rewrite_in_place(&self, body: &str) {
        let before = fs::metadata(self.executable()).unwrap();
        self.write_executable(format!("#!/bin/sh\n{body}\n").as_bytes());
        let after = fs::metadata(self.executable()).unwrap();
        use std::os::unix::fs::MetadataExt;
        assert_eq!(
            (before.dev(), before.ino()),
            (after.dev(), after.ino()),
            "the in-place rewrite must keep the inode, or it is a replacement case"
        );
    }

    /// Replaces the executable: a *different* file at the same path.
    fn replace(&self, body: &str) {
        let before = fs::metadata(self.executable()).unwrap();
        fs::remove_file(self.executable()).unwrap();
        self.write_executable(format!("#!/bin/sh\n{body}\n").as_bytes());
        let after = fs::metadata(self.executable()).unwrap();
        use std::os::unix::fs::MetadataExt;
        assert_ne!(
            (before.dev(), before.ino()),
            (after.dev(), after.ino()),
            "the replacement must land on a new inode"
        );
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn request() -> VerificationRequest {
    VerificationRequest {
        protocol_version: 1,
        evidence_version: 1,
        request_id: "request".into(),
        plugin_id: PluginId::new("example.identity").unwrap(),
        capability: VerifierCapability::ResourceContracts,
        target: TargetProfile {
            library: "example".into(),
            revision: "source".into(),
            profile: "default".into(),
            dialect: "qcore".into(),
            platform: "generic".into(),
            backend: "native".into(),
        },
        unit: UnitIdentity {
            source_digest: "a".repeat(64),
            build_digest: "b".repeat(64),
            translation_unit: "unit.qcore".into(),
        },
        facts: vec![],
        assumptions: BTreeMap::new(),
    }
}

fn manifest() -> PluginManifest {
    PluginManifest {
        plugin_id: PluginId::new("example.identity").unwrap(),
        package_version: "0.1.0".into(),
        protocol_versions: BTreeSet::from([1]),
        evidence_versions: BTreeSet::from([1]),
        entrypoint: PluginEntrypoint::Executable {
            path: "provider.sh".into(),
        },
        package_digest: "0".repeat(64),
        targets: vec![SupportedTarget {
            library: "example".into(),
            revisions: BTreeSet::from(["source".into()]),
            profiles: BTreeSet::from(["default".into()]),
            dialects: BTreeSet::from(["qcore".into()]),
            platforms: BTreeSet::from(["generic".into()]),
            backends: BTreeSet::from(["native".into()]),
        }],
        capabilities: BTreeSet::from([VerifierCapability::ResourceContracts]),
        required_foreign_summaries: BTreeSet::new(),
        required_observation_hooks: BTreeSet::new(),
        filesystem_inputs: BTreeSet::new(),
        network_access: false,
        limits: PluginLimits {
            timeout_ms: 5_000,
            ..PluginLimits::default()
        },
        trust_class: PluginTrustClass::FirstParty,
        certificate_checker: "example.untrusted_claims".into(),
        lean_theorem_family: "QCore.VerifierPlugin".into(),
    }
}

/// The package the substitutions install: it answers nothing and records that
/// it ran. If any refusal below is not a refusal, this marker appears.
const HOSTILE: &str = ": > ran.marker";

/// A verified package answers, and the record carries the measured residual
/// window rather than claiming the window is closed.
#[test]
fn a_verified_package_answers_and_records_its_residual_window() {
    let fixture = Fixture::new("baseline");
    let request = request();
    fixture.response(&request);
    fixture.install("/bin/cat > /dev/null\n/bin/cat response.bin");

    let provider = verify_executable_package(fixture.package()).expect("the package verifies");
    assert!(provider.verified_executable().is_some());
    for _ in 0..3 {
        provider.verify(&request).expect("the package answers");
        let record = provider.last_execution().unwrap();
        assert_eq!(record.outcome, ProcessOutcome::Completed);
        // Measured, not assumed: the window is a positive number of
        // microseconds, and its magnitude is reported in the card's README.
        let window = record
            .identity_window_us
            .expect("a verified execution measures its window");
        println!("identity window: {window} us");
        assert!(window < 1_000_000);
    }
}

/// The file is replaced between verification and dispatch. The next spawn must
/// refuse, and the replacement must never run.
#[test]
fn a_replaced_executable_is_refused_at_the_next_spawn() {
    let fixture = Fixture::new("replaced");
    let request = request();
    fixture.response(&request);
    fixture.install("/bin/cat > /dev/null\n/bin/cat response.bin");
    let provider = verify_executable_package(fixture.package()).expect("the package verifies");
    provider.verify(&request).expect("the package answers");

    fixture.replace(HOSTILE);
    let failure = provider.verify(&request).unwrap_err();
    let ProviderFailure::Protocol(message) = &failure else {
        panic!("expected a pre-spawn refusal, got {failure:?}");
    };
    assert!(
        message.contains("was replaced since verification"),
        "{message}"
    );
    let record = provider.last_execution().unwrap();
    assert_eq!(record.outcome, ProcessOutcome::RefusedByIdentity);
    assert!(record.enforced.is_none());
    assert!(record.identity_window_us.is_none());
    assert!(
        !fixture.ran_marker().exists(),
        "the substituted package must never be spawned"
    );
}

/// Byte-identical content at a fresh inode is still a replacement: what an
/// `execve` on the path would load is not the file that was verified.
#[test]
fn a_byte_identical_replacement_is_still_refused() {
    let fixture = Fixture::new("identical");
    let request = request();
    fixture.response(&request);
    fixture.install("/bin/cat > /dev/null\n/bin/cat response.bin");
    let provider = verify_executable_package(fixture.package()).expect("the package verifies");
    provider.verify(&request).expect("the package answers");

    let bytes = fs::read(fixture.executable()).unwrap();
    fs::remove_file(fixture.executable()).unwrap();
    fixture.write_executable(&bytes);
    assert_eq!(
        sha256_hex(&fs::read(fixture.executable()).unwrap()),
        provider.manifest().package_digest,
        "the replacement is byte-identical, so only file identity can refuse it"
    );
    let failure = provider.verify(&request).unwrap_err();
    assert!(
        failure
            .to_string()
            .contains("was replaced since verification"),
        "{failure}"
    );
    assert_eq!(
        provider.last_execution().unwrap().outcome,
        ProcessOutcome::RefusedByIdentity
    );
}

/// The same inode is rewritten in place, so the path identity check cannot see
/// it: the content comparison through the held descriptor must.
#[test]
fn an_in_place_rewrite_is_refused_at_the_next_spawn() {
    let fixture = Fixture::new("rewritten");
    let request = request();
    fixture.response(&request);
    fixture.install("/bin/cat > /dev/null\n/bin/cat response.bin");
    let provider = verify_executable_package(fixture.package()).expect("the package verifies");
    provider.verify(&request).expect("the package answers");

    fixture.rewrite_in_place(HOSTILE);
    let failure = provider.verify(&request).unwrap_err();
    assert!(
        failure.to_string().contains("changed since verification"),
        "{failure}"
    );
    let record = provider.last_execution().unwrap();
    assert_eq!(record.outcome, ProcessOutcome::RefusedByIdentity);
    assert!(!fixture.ran_marker().exists());
}

/// A *validly templated* substitute — a package that would verify on its own —
/// is refused too. Substitution is refused because the identity moved, not
/// because the replacement looked malformed.
#[test]
fn a_differently_templated_but_valid_package_is_refused_at_spawn() {
    let fixture = Fixture::new("retemplated");
    let request = request();
    fixture.response(&request);
    fixture.install("/bin/cat > /dev/null\n/bin/cat response.bin");
    let provider = verify_executable_package(fixture.package()).expect("the package verifies");
    provider.verify(&request).expect("the package answers");

    // Install a *different*, self-consistent package at the same path: it
    // verifies from scratch, and it still cannot serve this provider's request.
    let reinstalled = fixture.install(&format!("{HOSTILE}\n/bin/cat response.bin"));
    assert_ne!(
        reinstalled.package_digest,
        provider.manifest().package_digest
    );
    verify_executable_package(fixture.package()).expect("the substitute verifies on its own");

    let failure = provider.verify(&request).unwrap_err();
    assert!(
        failure.to_string().contains("since verification"),
        "{failure}"
    );
    assert_eq!(
        provider.last_execution().unwrap().outcome,
        ProcessOutcome::RefusedByIdentity
    );
    assert!(!fixture.ran_marker().exists());
}

/// The executable is removed outright.
#[test]
fn a_removed_executable_is_refused_at_the_next_spawn() {
    let fixture = Fixture::new("removed");
    let request = request();
    fixture.response(&request);
    fixture.install("/bin/cat > /dev/null\n/bin/cat response.bin");
    let provider = verify_executable_package(fixture.package()).expect("the package verifies");
    provider.verify(&request).expect("the package answers");

    fs::remove_file(fixture.executable()).unwrap();
    let failure = provider.verify(&request).unwrap_err();
    assert!(
        failure
            .to_string()
            .contains("was replaced since verification"),
        "{failure}"
    );
    assert_eq!(
        provider.last_execution().unwrap().outcome,
        ProcessOutcome::RefusedByIdentity
    );
}

/// Through a registry the refusal is a scoped `Unknown` under `QP006` with no
/// certificate — never a program `Unsafe`, and never a downgrade to running the
/// substitute.
#[test]
fn a_substituted_package_is_unknown_through_a_registry() {
    let fixture = Fixture::new("registry");
    let request = request();
    fixture.response(&request);
    fixture.install("/bin/cat > /dev/null\n/bin/cat response.bin");
    let mut registry = VerifierRegistry::default();
    registry
        .register(Arc::new(
            verify_executable_package(fixture.package()).expect("the package verifies"),
        ))
        .unwrap();
    let answered = registry.verify(&request);
    assert_eq!(answered.verdict, ProposedVerdict::Unknown);

    fixture.replace(HOSTILE);
    let outcome = registry.verify(&request);
    assert_eq!(outcome.verdict, ProposedVerdict::Unknown);
    assert!(outcome.certificate.is_none());
    assert_eq!(outcome.diagnostics[0].code, "QP006");
    assert!(!fixture.ran_marker().exists());
}

/// The unverified loader is a different type, and it re-checks nothing: it has
/// nothing to re-check. That is exactly why it cannot be registered.
#[test]
fn an_unverified_provider_carries_no_identity_to_recheck() {
    let fixture = Fixture::new("unverified");
    let request = request();
    fixture.response(&request);
    fixture.install("/bin/cat > /dev/null\n/bin/cat response.bin");
    let provider = UnverifiedProvider::load(fixture.package()).expect("the manifest loads");
    assert!(provider.verified_executable().is_none());
    provider.verify(&request).expect("the package answers");
    assert!(
        provider
            .last_execution()
            .unwrap()
            .identity_window_us
            .is_none(),
        "an unverified execution measures no window, because it re-checked nothing"
    );

    // The bridge verifies rather than asserting: the same package becomes a
    // registrable provider only by passing the same gate.
    let verified = provider.into_verified().expect("the package verifies");
    assert!(verified.verified_executable().is_some());

    // And a package that does not verify stays unverified.
    let broken = UnverifiedProvider::load(fixture.package()).expect("the manifest loads");
    fixture.rewrite_in_place(HOSTILE);
    let Err(error) = broken.into_verified() else {
        panic!("a rewritten executable must not verify");
    };
    assert!(error.message.contains("digest mismatch"), "{error}");
}

/// The execution profile is orthogonal: a policy applied to an unverified
/// package returns an unverified package, so applying a policy is not a way
/// around the gate.
#[test]
fn applying_an_execution_policy_does_not_verify_a_package() {
    let fixture = Fixture::new("policy");
    fixture.install("/bin/cat > /dev/null\n/bin/cat response.bin");
    let policy = ExecutionPolicy::default();
    let provider = policy
        .apply_unverified(UnverifiedProvider::load(fixture.package()).unwrap())
        .expect("the default policy applies");
    assert!(provider.verified_executable().is_none());
}

/// A symbolically linked entrypoint is checked and executed at the same path.
///
/// Verification canonicalizes the entrypoint, so the descriptor, the re-check
/// and the `execve` all name the resolved file. Re-pointing the link therefore
/// cannot select a different executable behind the check's back — the verified
/// file is what runs — and rewriting the *resolved* file is refused as changed
/// content, as it is for a plain entrypoint.
#[test]
fn a_symbolic_entrypoint_is_checked_where_it_is_executed() {
    let fixture = Fixture::new("symlink");
    let request = request();
    fixture.response(&request);
    fixture.install("/bin/cat > /dev/null\n/bin/cat response.bin");

    // `provider.sh` becomes a link to `real.sh`, which holds the real bytes.
    let real = fixture.package().join("real.sh");
    fs::rename(fixture.executable(), &real).unwrap();
    std::os::unix::fs::symlink("real.sh", fixture.executable()).unwrap();
    let provider = verify_executable_package(fixture.package()).expect("the package verifies");
    assert_eq!(
        provider.verified_executable().unwrap().path(),
        real.canonicalize().unwrap()
    );
    provider.verify(&request).expect("the package answers");

    // Re-pointing the link cannot substitute the executable: the verified,
    // canonical file is what the spawn names.
    process_fixture::write_executable(
        &fixture.package().join("decoy.sh"),
        format!("#!/bin/sh\n{HOSTILE}\n").as_bytes(),
    );
    fs::remove_file(fixture.executable()).unwrap();
    std::os::unix::fs::symlink("decoy.sh", fixture.executable()).unwrap();
    provider
        .verify(&request)
        .expect("the verified file still answers");
    assert!(
        !fixture.ran_marker().exists(),
        "the decoy must never be spawned"
    );

    // Rewriting the resolved file is still refused.
    process_fixture::write_executable(&real, format!("#!/bin/sh\n{HOSTILE}\n").as_bytes());
    let failure = provider.verify(&request).unwrap_err();
    assert!(
        failure.to_string().contains("changed since verification"),
        "{failure}"
    );
    assert_eq!(
        provider.last_execution().unwrap().outcome,
        ProcessOutcome::RefusedByIdentity
    );
    assert!(!fixture.ran_marker().exists());
}
