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

//! Adversarial fixtures for the target-qualified confined process profile
//! (`docs/reports/remaining-p03/README.md`).
//!
//! Every case here is a hostile package: it tries to reach a listening server,
//! or to read or write outside the paths an explicit policy admits. The point
//! of each pair of cases is that the *kernel* stops it — each hostile fixture
//! is also run under the historical profile, where it succeeds, so the bound is
//! demonstrably the confinement and not the fixture.
//!
//! What this profile does not confine is enumerated in
//! `Confinement::unconfined` and asserted below; nothing here claims a general
//! OS sandbox.
#![cfg(target_os = "linux")]

use qcore_verifier_host::*;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::ErrorKind;
use std::net::TcpListener;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

#[path = "support/process_fixture.rs"]
mod process_fixture;
use process_fixture::{READ_REQUEST, write_executable};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// A fixture root with three disjoint areas: the package the host executes, a
/// directory the policy admits for writing, and a directory the policy never
/// names.
struct Fixture {
    root: PathBuf,
}

impl Fixture {
    fn new(tag: &str) -> Self {
        let root = std::env::temp_dir().join(format!(
            "qcore-p03-{tag}-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir(&root).unwrap();
        fs::create_dir(root.join("package")).unwrap();
        fs::create_dir(root.join("writable")).unwrap();
        fs::create_dir(root.join("outside")).unwrap();
        fs::write(root.join("outside/secret.txt"), b"classified\n").unwrap();
        Self { root }
    }

    fn package(&self) -> PathBuf {
        self.root.join("package")
    }

    fn writable(&self) -> PathBuf {
        self.root.join("writable")
    }

    fn outside(&self) -> PathBuf {
        self.root.join("outside")
    }

    /// The fixture reads its whole request before `body` runs, and its
    /// executable is written without a write descriptor in this process
    /// (`support/process_fixture.rs` says which load-dependent failure each
    /// closes).
    fn script(&self, body: &str) {
        write_executable(
            &self.package().join("provider.sh"),
            format!("#!/bin/sh\n{READ_REQUEST}\n{body}\n").as_bytes(),
        );
    }

    /// Writes the valid response frame `provider.sh` can emit with
    /// `/bin/cat response.bin`.
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
            self.package().join("response.bin"),
            encode_protocol_frame(&response, DEFAULT_MAX_FRAME_BYTES).unwrap(),
        )
        .unwrap();
    }

    /// A marker inside the package root. Nothing the confined profile runs can
    /// create it, so it answers exactly one question: did the package run at
    /// all? A refusal happens before the spawn, where the package would still
    /// have been unconfined.
    fn ran_marker(&self) -> PathBuf {
        self.package().join("ran.marker")
    }

    /// Installs the fixture as a package that actually *verifies*, so it can be
    /// registered: a registry takes only verified providers (card `P01-F3`).
    /// The manifest template follows the script's last command — inert to `sh`,
    /// and bytes the host reads without executing anything.
    fn install(&self, manifest: PluginManifest) {
        let script = self.package().join("provider.sh");
        let mut bytes = fs::read(&script).unwrap();
        // `exit` before the envelope: `sh` parses commands one at a time, so
        // it stops before the marker lines, which are not shell syntax.
        bytes.extend(b"exit $?\n");
        bytes.extend(envelope(&serde_json::to_string(&manifest).unwrap()).into_bytes());
        bytes.push(b'\n');
        write_executable(&script, &bytes);
        let mut installed = manifest;
        installed.package_digest = sha256_hex(&fs::read(&script).unwrap());
        fs::write(
            self.package().join("verifier-plugin.json"),
            serde_json::to_vec(&installed).unwrap(),
        )
        .unwrap();
    }

    fn policy(&self) -> ConfinementPolicy {
        ConfinementPolicy::linux_runtime_defaults()
            .admitting_read_write(self.writable().to_string_lossy().into_owned())
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::set_permissions(self.outside(), fs::Permissions::from_mode(0o700));
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn request() -> VerificationRequest {
    VerificationRequest {
        protocol_version: 1,
        evidence_version: 1,
        request_id: "request".into(),
        plugin_id: PluginId::new("example.confined").unwrap(),
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
        plugin_id: PluginId::new("example.confined").unwrap(),
        package_version: "0.1.0".into(),
        protocol_versions: BTreeSet::from([1]),
        evidence_versions: BTreeSet::from([1]),
        entrypoint: PluginEntrypoint::Executable {
            path: "provider.sh".into(),
        },
        package_digest: "a".repeat(64),
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
        // Generous enough that an interpreter starts: this suite is about
        // confinement, and a limit kill would answer a different question.
        limits: PluginLimits {
            timeout_ms: 30_000,
            memory_bytes: 4 * 1024 * 1024 * 1024,
            ..PluginLimits::default()
        },
        trust_class: PluginTrustClass::FirstParty,
        certificate_checker: "example.untrusted_claims".into(),
        lean_theorem_family: "QCore.VerifierPlugin".into(),
    }
}

fn confined(fixture: &Fixture) -> UnverifiedProvider {
    UnverifiedProvider::new(manifest(), fixture.package())
        .unwrap()
        .with_execution_profile(ProcessExecutionProfile::LinuxConfinedV1)
        .with_confinement_policy(fixture.policy())
}

fn legacy(fixture: &Fixture) -> UnverifiedProvider {
    UnverifiedProvider::new(manifest(), fixture.package()).unwrap()
}

fn confinement(record: &ProcessExecutionRecord) -> Confinement {
    record
        .enforced
        .as_ref()
        .expect("a completed confined execution records what it enforced")
        .confinement
        .clone()
        .expect("the confining profile records its confinement")
}

/// The interpreter the network fixture uses. `python3` is already a standing
/// gate dependency of this repository, and the host clears the environment, so
/// the fixture needs an absolute path.
fn python3() -> PathBuf {
    for candidate in ["/usr/bin/python3", "/bin/python3", "/usr/local/bin/python3"] {
        if Path::new(candidate).exists() {
            return PathBuf::from(candidate);
        }
    }
    panic!("python3 is required to exercise the network confinement fixture");
}

/// 1. A fixture that tries to reach a listening localhost server cannot, and
///    the identical fixture reaches it under the historical profile.
///
///    The fixture separates "the interpreter could not run" from "the connect
///    failed" with distinct exit codes, so the confined case cannot pass
///    vacuously.
#[test]
fn a_confined_fixture_cannot_reach_a_listening_localhost_server() {
    let fixture = Fixture::new("network");
    let request = request();
    fixture.response(&request);
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let port = listener.local_addr().unwrap().port();
    let python = python3();
    let python = python.display();
    fixture.script(&format!(
        "{python} -c 'pass' || exit 4\n\
         {python} -c \"import socket; socket.create_connection(('127.0.0.1', {port}), 5).close()\" \
         || exit 3\n\
         /bin/cat response.bin"
    ));

    // A connect to a listening socket completes through the backlog, so
    // draining the listener afterwards counts exactly the reachable attempts.
    let drain = |listener: &TcpListener| {
        let mut accepted = 0;
        loop {
            match listener.accept() {
                Ok(_) => accepted += 1,
                Err(error) if error.kind() == ErrorKind::WouldBlock => return accepted,
                Err(error) => panic!("{error}"),
            }
        }
    };

    let provider = confined(&fixture);
    let failure = provider.verify(&request).unwrap_err();
    let record = provider.last_execution().unwrap();
    assert_eq!(record.outcome, ProcessOutcome::Terminated, "{failure}");
    // Exit 3, not 4: the interpreter started and only the connect failed.
    assert_eq!(record.exit_code, Some(3), "{}", record.summary());
    std::thread::sleep(Duration::from_millis(200));
    assert_eq!(drain(&listener), 0, "the confined child reached the server");
    let confinement = confinement(&record);
    assert_eq!(
        confinement.network_mechanism,
        "unshare(CLONE_NEWUSER|CLONE_NEWNET)"
    );
    assert!(!confinement.network_access_declared);

    // Control: the same fixture, unconfined, reaches the same server and
    // answers normally. Without this the case above proves nothing.
    let control = legacy(&fixture);
    let response = control.verify(&request).unwrap();
    assert_eq!(response.verdict, ProposedVerdict::Unknown);
    let control_record = control.last_execution().unwrap();
    assert_eq!(control_record.outcome, ProcessOutcome::Completed);
    assert!(
        control_record
            .enforced
            .as_ref()
            .unwrap()
            .confinement
            .is_none()
    );
    assert!(
        drain(&listener) >= 1,
        "the control never reached the server"
    );
}

/// 2. A fixture that reads a file outside the admitted paths cannot, and the
///    identical fixture reads it under the historical profile.
#[test]
fn a_confined_fixture_cannot_read_outside_the_admitted_paths() {
    let fixture = Fixture::new("read");
    let request = request();
    fixture.response(&request);
    let secret = fixture.outside().join("secret.txt");
    fixture.script(&format!(
        "/bin/cat {} > /dev/null 2>&1 || exit 5\n/bin/cat response.bin",
        secret.display()
    ));

    let provider = confined(&fixture);
    let failure = provider.verify(&request).unwrap_err();
    let record = provider.last_execution().unwrap();
    assert_eq!(record.outcome, ProcessOutcome::Terminated, "{failure}");
    assert_eq!(record.exit_code, Some(5), "{}", record.summary());
    // The path is not in the admitted inventory at all.
    assert!(
        !confinement(&record)
            .paths
            .iter()
            .any(|path| secret.starts_with(&path.path))
    );

    let control = legacy(&fixture);
    assert_eq!(
        control.verify(&request).unwrap().verdict,
        ProposedVerdict::Unknown
    );
    assert_eq!(
        control.last_execution().unwrap().outcome,
        ProcessOutcome::Completed
    );
}

/// 3. A fixture that writes outside the admitted paths cannot, and the file it
///    tried to create does not exist afterwards.
#[test]
fn a_confined_fixture_cannot_write_outside_the_admitted_paths() {
    let fixture = Fixture::new("write");
    let request = request();
    fixture.response(&request);
    let stolen = fixture.outside().join("stolen.txt");
    fixture.script(&format!(
        "echo taken > {} 2>/dev/null || exit 6\n/bin/cat response.bin",
        stolen.display()
    ));

    let provider = confined(&fixture);
    let failure = provider.verify(&request).unwrap_err();
    let record = provider.last_execution().unwrap();
    assert_eq!(record.outcome, ProcessOutcome::Terminated, "{failure}");
    assert_eq!(record.exit_code, Some(6), "{}", record.summary());
    assert!(!stolen.exists(), "the confined child wrote outside");

    let control = legacy(&fixture);
    assert_eq!(
        control.verify(&request).unwrap().verdict,
        ProposedVerdict::Unknown
    );
    assert!(stolen.exists(), "the control never wrote outside");
}

/// 4. The admitted paths still work: the package executes, reads its own
///    response and writes into the one directory the policy admits.
#[test]
fn an_admitted_path_still_works_under_confinement() {
    let fixture = Fixture::new("admitted");
    let request = request();
    fixture.response(&request);
    let allowed = fixture.writable().join("answer.txt");
    fixture.script(&format!(
        "echo written > {} || exit 7\n/bin/cat {} > /dev/null || exit 8\n/bin/cat response.bin",
        allowed.display(),
        allowed.display()
    ));

    let provider = confined(&fixture);
    let response = provider.verify(&request).unwrap();
    assert_eq!(response.verdict, ProposedVerdict::Unknown);
    assert_eq!(response.request_id, request.request_id);
    let record = provider.last_execution().unwrap();
    assert_eq!(record.outcome, ProcessOutcome::Completed);
    assert_eq!(record.exit_code, Some(0));
    assert_eq!(fs::read_to_string(&allowed).unwrap(), "written\n");
    let confinement = confinement(&record);
    assert!(
        confinement
            .paths
            .iter()
            .any(|path| path.path == fixture.writable().to_string_lossy()
                && path.access == PathAccess::ReadWrite
                && path.state == PathState::Admitted)
    );
}

/// 5. The package root is admitted read-and-execute, not read-and-write: the
///    package cannot rewrite its own executable or drop files beside it.
#[test]
fn the_package_root_is_admitted_read_and_execute_only() {
    let fixture = Fixture::new("packageroot");
    let request = request();
    fixture.response(&request);
    // The write runs in a subshell: a redirection the kernel denies must not
    // end the fixture before it can report that it was denied.
    fixture.script("( echo x > ran.marker ) 2>/dev/null && exit 9\n/bin/cat response.bin");

    let provider = confined(&fixture);
    assert_eq!(
        provider.verify(&request).unwrap().verdict,
        ProposedVerdict::Unknown
    );
    assert!(!fixture.ran_marker().exists());
    let record = provider.last_execution().unwrap();
    let confinement = confinement(&record);
    let root = confinement.paths.first().unwrap();
    assert_eq!(root.path, fixture.package().to_string_lossy());
    assert_eq!(root.access, PathAccess::ReadExecute);
    assert_eq!(root.state, PathState::Admitted);
}

/// 6. The confining profile *composes* with the limit profile: the same
///    `setrlimit` bounds are installed, and a memory-hungry package is still
///    killed by them.
#[test]
fn the_confined_profile_composes_with_the_limit_profile() {
    let fixture = Fixture::new("compose");
    let request = request();
    fixture.response(&request);
    fixture.script(
        "i=0\ns=x\nwhile [ $i -lt 24 ]; do s=\"$s$s\"; i=$((i+1)); done\n/bin/cat response.bin",
    );
    let mut manifest = manifest();
    manifest.limits.memory_bytes = 8 * 1024 * 1024;
    manifest.limits.additional.insert("cpu_seconds".into(), 30);

    let provider = UnverifiedProvider::new(manifest.clone(), fixture.package())
        .unwrap()
        .with_execution_profile(ProcessExecutionProfile::LinuxConfinedV1)
        .with_confinement_policy(fixture.policy());
    let failure = provider.verify(&request).unwrap_err();
    let record = provider.last_execution().unwrap();
    assert_eq!(record.outcome, ProcessOutcome::Terminated, "{failure}");
    let enforced = record.enforced.as_ref().unwrap();
    assert!(enforced.confinement.is_some());
    assert!(enforced.process_group_kill);
    assert!(enforced.unenforced_kinds.is_empty());

    // The composed rlimit plan is exactly the limit profile's plan.
    let limits_only = UnverifiedProvider::new(manifest, fixture.package())
        .unwrap()
        .with_execution_profile(ProcessExecutionProfile::LinuxRlimitV1);
    let reference = limits_only.enforced_limits().unwrap();
    assert_eq!(enforced.rlimits, reference.rlimits);
    assert!(reference.confinement.is_none());
    // Deliberate pin update, card `P03-F`: the two profiles' *`setrlimit`*
    // tables are still exactly equal — that is the composition claim, and it is
    // the assertion above and this one. What is no longer equal is the total
    // enforceable set: the confining profile additionally installs
    // `max_execution_processes` as a cgroup quota, which the limit profile
    // refuses. The load-bearing negative half is kept below: the limit profile
    // must not claim the quota kind.
    assert_eq!(
        enforceable_rlimit_kinds(ProcessExecutionProfile::LinuxConfinedV1),
        enforceable_rlimit_kinds(ProcessExecutionProfile::LinuxRlimitV1)
    );
    assert!(
        !enforceable_limit_kinds(ProcessExecutionProfile::LinuxRlimitV1)
            .contains(&MAX_EXECUTION_PROCESSES)
    );
    let confined_kinds = enforceable_limit_kinds(ProcessExecutionProfile::LinuxConfinedV1);
    assert!(confined_kinds.contains(&MAX_EXECUTION_PROCESSES));
    let extra: Vec<_> = confined_kinds
        .iter()
        .filter(|kind| {
            !enforceable_limit_kinds(ProcessExecutionProfile::LinuxRlimitV1).contains(kind)
        })
        .copied()
        .collect();
    assert_eq!(extra, vec![MAX_EXECUTION_PROCESSES]);
    // The composed execution declared no quota, so it installed none, and its
    // record says exactly that rather than leaving the question open.
    assert!(enforced.process_quota.is_none());
}

/// 7. Unavailable enforcement is a refusal before the spawn, not a weaker run:
///    a confining profile with no policy admits nothing and runs nothing.
#[test]
fn a_confining_profile_without_a_policy_is_refused_before_spawning() {
    let fixture = Fixture::new("nopolicy");
    let request = request();
    fixture.response(&request);
    fixture.script(": > ran.marker\n/bin/cat response.bin");

    let provider = UnverifiedProvider::new(manifest(), fixture.package())
        .unwrap()
        .with_execution_profile(ProcessExecutionProfile::LinuxConfinedV1);
    let precheck = provider.enforced_limits().unwrap_err();
    assert!(
        precheck
            .to_string()
            .contains("requires an explicit confinement policy"),
        "{precheck}"
    );
    assert_eq!(provider.verify(&request).unwrap_err(), precheck);
    let record = provider.last_execution().unwrap();
    assert_eq!(record.outcome, ProcessOutcome::RefusedByProfile);
    assert!(record.enforced.is_none());
    // Nothing ran: had it run, it would have run *unconfined*, and the marker
    // write into the package root would have succeeded.
    assert!(!fixture.ran_marker().exists());

    let mut registry = VerifierRegistry::default();
    // A registry takes only *verified* providers (card `P01-F3`).
    fixture.install(manifest());
    registry
        .register(Arc::new(
            verify_executable_package(fixture.package())
                .expect("the installed fixture verifies")
                .with_execution_profile(ProcessExecutionProfile::LinuxConfinedV1),
        ))
        .unwrap();
    let outcome = registry.verify(&request);
    assert_eq!(outcome.verdict, ProposedVerdict::Unknown);
    assert_eq!(outcome.diagnostics[0].code, "QP006");
    assert!(outcome.certificate.is_none());
    assert!(!fixture.ran_marker().exists());
}

/// 8. A policy handed to a profile that enforces none is refused in the other
///    direction, so a caller cannot hold a reviewed policy and an unconfined
///    execution at the same time.
#[test]
fn a_policy_under_a_non_confining_profile_is_refused() {
    let fixture = Fixture::new("mismatch");
    let request = request();
    fixture.response(&request);
    fixture.script(": > ran.marker\n/bin/cat response.bin");

    for profile in [
        ProcessExecutionProfile::UnenforcedLegacy,
        ProcessExecutionProfile::LinuxRlimitV1,
    ] {
        let provider = UnverifiedProvider::new(manifest(), fixture.package())
            .unwrap()
            .with_execution_profile(profile)
            .with_confinement_policy(fixture.policy());
        let failure = provider.enforced_limits().unwrap_err();
        assert!(
            failure
                .to_string()
                .contains("enforces no confinement policy"),
            "{failure}"
        );
        assert_eq!(provider.verify(&request).unwrap_err(), failure);
        assert_eq!(
            provider.last_execution().unwrap().outcome,
            ProcessOutcome::RefusedByProfile
        );
        assert!(!fixture.ran_marker().exists());
    }
}

/// 9. A policy the kernel cannot install is refused before the spawn too. An
///    unopenable admitted path is the reachable instance of "the mechanism
///    could not be installed"; the ABI and namespace probes fail into the same
///    refusal.
#[test]
fn an_uninstallable_policy_is_refused_before_spawning() {
    let fixture = Fixture::new("uninstallable");
    let request = request();
    fixture.response(&request);
    fixture.script(": > ran.marker\n/bin/cat response.bin");
    // A dangling symlink: the policy check sees a path that exists, and the
    // kernel refuses to open it when the rule is installed.
    let unreachable = fixture.root.join("dangling");
    std::os::unix::fs::symlink(fixture.root.join("never-created"), &unreachable).unwrap();

    let provider = UnverifiedProvider::new(manifest(), fixture.package())
        .unwrap()
        .with_execution_profile(ProcessExecutionProfile::LinuxConfinedV1)
        .with_confinement_policy(
            fixture
                .policy()
                .admitting_read_only(unreachable.to_string_lossy().into_owned()),
        );
    // The path exists, so it is admitted by the policy and the failure is the
    // kernel's, not a missing-path record.
    let described = provider.enforced_limits().unwrap();
    assert!(
        described
            .confinement
            .as_ref()
            .unwrap()
            .paths
            .iter()
            .any(|path| path.path == unreachable.to_string_lossy()
                && path.state == PathState::Admitted)
    );
    let failure = provider.verify(&request).unwrap_err();
    assert!(
        failure.to_string().contains("could not open admitted path"),
        "{failure}"
    );
    let record = provider.last_execution().unwrap();
    assert_eq!(record.outcome, ProcessOutcome::RefusedByProfile);
    assert!(record.enforced.is_none());
    assert!(!fixture.ran_marker().exists());
}

/// 10. An admitted path that does not exist grants nothing and is recorded as
///     absent, so a report cannot quote the policy as if it were the reach.
#[test]
fn an_absent_admitted_path_grants_nothing_and_is_recorded() {
    let fixture = Fixture::new("absent");
    let missing = fixture.root.join("never-created");
    let provider = UnverifiedProvider::new(manifest(), fixture.package())
        .unwrap()
        .with_execution_profile(ProcessExecutionProfile::LinuxConfinedV1)
        .with_confinement_policy(
            fixture
                .policy()
                .admitting_read_write(missing.to_string_lossy().into_owned()),
        );
    let confinement = provider
        .enforced_limits()
        .unwrap()
        .confinement
        .expect("the confining profile describes its confinement");
    let recorded = confinement
        .paths
        .iter()
        .find(|path| path.path == missing.to_string_lossy())
        .unwrap();
    assert_eq!(recorded.state, PathState::Absent);
    assert_eq!(recorded.access, PathAccess::ReadWrite);
}

/// 11. The evidence records what the kernel actually offered, including the
///     probe results, and states its own boundary.
#[test]
fn the_evidence_records_the_achieved_confinement_and_its_boundary() {
    let fixture = Fixture::new("evidence");
    let request = request();
    fixture.response(&request);
    fixture.script("/bin/cat response.bin");

    let provider = confined(&fixture);
    provider.verify(&request).unwrap();
    let record = provider.last_execution().unwrap();
    let confinement = confinement(&record);
    assert_eq!(confinement.filesystem_mechanism, "landlock");
    assert!(confinement.landlock_abi >= confinement.landlock_min_abi);
    assert!(!confinement.kernel_release.is_empty());
    assert_ne!(confinement.kernel_release, "unknown");
    assert_ne!(confinement.handled_access_fs, "0x0");
    assert!(!confinement.unconfined.is_empty());
    let boundary = confinement.unconfined.join("\n");
    assert!(boundary.contains("seccomp"));
    assert!(boundary.contains("Unix-domain socket"));
    assert!(record.summary().contains("linux-confined-v1"));
    assert!(record.summary().contains("landlock"));
    assert!(record.summary().contains("admitted_paths="));

    // Serialized evidence carries the confinement; the profiles that confine
    // nothing say so by omitting the key entirely, which keeps their records
    // byte-identical to the ones written before this field existed.
    let json = serde_json::to_value(record.enforced.as_ref().unwrap()).unwrap();
    assert!(json["confinement"]["landlock_abi"].is_number());
    let unconfined = UnverifiedProvider::new(manifest(), fixture.package())
        .unwrap()
        .with_execution_profile(ProcessExecutionProfile::LinuxRlimitV1)
        .enforced_limits()
        .unwrap();
    assert!(unconfined.confinement.is_none());
    let json = serde_json::to_value(&unconfined).unwrap();
    assert!(json.get("confinement").is_none());
    assert!(unconfined.summary().contains("confinement=none"));
}

/// 12. The default profile is unchanged and confines nothing, so no existing
///     caller moves without an explicit selection.
#[test]
fn the_default_profile_still_confines_nothing() {
    let fixture = Fixture::new("default");
    let provider = UnverifiedProvider::new(manifest(), fixture.package()).unwrap();
    assert_eq!(
        provider.execution_profile(),
        ProcessExecutionProfile::UnenforcedLegacy
    );
    assert!(provider.confinement_policy().is_none());
    assert!(!ProcessExecutionProfile::default().confines());
    assert!(!ProcessExecutionProfile::LinuxRlimitV1.confines());
    assert!(ProcessExecutionProfile::LinuxConfinedV1.confines());
    assert!(ProcessExecutionProfile::LinuxConfinedV1.installs_rlimits());
    assert_eq!(
        ProcessExecutionProfile::LinuxConfinedV1.name(),
        "linux-confined-v1"
    );
    assert!(provider.enforced_limits().unwrap().confinement.is_none());
}
