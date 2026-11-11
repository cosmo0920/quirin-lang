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

//! Adversarial fixtures for the target-qualified enforced process profile
//! (`docs/reports/remaining-p02/README.md`).
//!
//! Each fixture is a shell provider written by the test. The point of every
//! case is that the *host* bounds it: the package is hostile or broken, and the
//! execution still terminates with a scoped `Unknown` and a recorded reason.
//! Nothing here claims filesystem or network confinement, which is a separate
//! card.
#![cfg(unix)]
#![cfg_attr(not(target_os = "linux"), allow(dead_code, unused_imports))]

use qcore_verifier_host::*;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

#[path = "support/process_fixture.rs"]
mod process_fixture;
use process_fixture::{READ_REQUEST, write_executable};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

struct Fixture {
    root: PathBuf,
}

impl Fixture {
    fn new(tag: &str) -> Self {
        let root = std::env::temp_dir().join(format!(
            "qcore-p02-{tag}-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir(&root).unwrap();
        Self { root }
    }

    /// The fixture reads its whole request before `body` runs, and its
    /// executable is written without a write descriptor in this process
    /// (`support/process_fixture.rs` says which load-dependent failure each
    /// closes).
    fn script(&self, body: &str) {
        write_executable(
            &self.root.join("provider.sh"),
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
            self.root.join("response.bin"),
            encode_protocol_frame(&response, DEFAULT_MAX_FRAME_BYTES).unwrap(),
        )
        .unwrap();
    }

    fn path(&self) -> &Path {
        &self.root
    }

    fn marker(&self) -> PathBuf {
        self.root.join("descendant.marker")
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
        plugin_id: PluginId::new("example.bounded").unwrap(),
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
        plugin_id: PluginId::new("example.bounded").unwrap(),
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
        limits: PluginLimits::default(),
        trust_class: PluginTrustClass::FirstParty,
        certificate_checker: "example.untrusted_claims".into(),
        lean_theorem_family: "QCore.VerifierPlugin".into(),
    }
}

fn enforced(manifest: PluginManifest, fixture: &Fixture) -> UnverifiedProvider {
    UnverifiedProvider::new(manifest, fixture.path())
        .unwrap()
        .with_execution_profile(ProcessExecutionProfile::LinuxRlimitV1)
}

fn legacy(manifest: PluginManifest, fixture: &Fixture) -> UnverifiedProvider {
    UnverifiedProvider::new(manifest, fixture.path()).unwrap()
}

/// Installs the fixture as a package that actually *verifies*, and returns the
/// registrable provider.
///
/// A registry takes only verified providers (card `P01-F3`), so the script
/// carries the manifest template its installed manifest binds to, exactly as a
/// shipped provider executable does. The envelope follows the script body; the
/// shell never parses past its own last command, so the template is inert to
/// `sh` and is bytes to the host, which never executes the package to read it.
fn registrable(
    manifest: PluginManifest,
    fixture: &Fixture,
    profile: ProcessExecutionProfile,
) -> ProcessProvider {
    let script = fixture.path().join("provider.sh");
    let mut bytes = fs::read(&script).unwrap();
    // `exit` before the envelope: `sh` parses commands one at a time, so it
    // stops before the marker lines, which are not shell syntax.
    bytes.extend(b"exit $?\n");
    bytes.extend(envelope(&serde_json::to_string(&manifest).unwrap()).into_bytes());
    bytes.push(b'\n');
    write_executable(&script, &bytes);
    let mut installed = manifest;
    installed.package_digest = sha256_hex(&fs::read(&script).unwrap());
    fs::write(
        fixture.path().join("verifier-plugin.json"),
        serde_json::to_vec(&installed).unwrap(),
    )
    .unwrap();
    verify_executable_package(fixture.path())
        .expect("the installed fixture verifies")
        .with_execution_profile(profile)
}

/// Grows a shell variable to 16 MiB, then answers.
const GROW_16MIB_THEN_ANSWER: &str =
    "i=0\ns=x\nwhile [ $i -lt 24 ]; do s=\"$s$s\"; i=$((i+1)); done\n/bin/cat response.bin";

fn applied(record: &ProcessExecutionRecord, kind: &str) -> Option<u64> {
    record
        .enforced
        .as_ref()?
        .rlimits
        .iter()
        .find(|limit| limit.kind == kind)
        .map(|limit| limit.soft)
}

/// 1. Memory growth beyond the declared limit is bounded, and the *same*
///    fixture completes under the legacy profile — so the bound comes from the
///    enforcement, not from the fixture.
#[test]
#[cfg(target_os = "linux")]
fn memory_growth_beyond_the_declared_limit_is_bounded() {
    let fixture = Fixture::new("memory");
    let request = request();
    fixture.response(&request);
    fixture.script(GROW_16MIB_THEN_ANSWER);
    let mut manifest = manifest();
    manifest.limits.timeout_ms = 20_000;
    manifest.limits.memory_bytes = 8 * 1024 * 1024;

    let provider = enforced(manifest.clone(), &fixture);
    let started = Instant::now();
    let failure = provider.verify(&request).unwrap_err();
    let elapsed = started.elapsed();
    let record = provider.last_execution().unwrap();
    assert_eq!(record.outcome, ProcessOutcome::Terminated, "{failure}");
    // Killed by the limit, not by the wall-clock deadline it never reached.
    assert!(elapsed < Duration::from_secs(10), "{elapsed:?}");
    assert!(record.exit_code != Some(0));
    assert_eq!(applied(&record, "memory_bytes"), Some(8 * 1024 * 1024));
    assert_eq!(
        record.enforced.as_ref().unwrap().profile,
        ProcessExecutionProfile::LinuxRlimitV1
    );
    assert!(
        record
            .enforced
            .as_ref()
            .unwrap()
            .unenforced_kinds
            .is_empty()
    );

    // Control: the historical profile installs no `setrlimit`, so the same
    // allocation succeeds and the run answers normally.
    let control = legacy(manifest, &fixture);
    let response = control.verify(&request).unwrap();
    assert_eq!(response.verdict, ProposedVerdict::Unknown);
    let control_record = control.last_execution().unwrap();
    assert_eq!(control_record.outcome, ProcessOutcome::Completed);
    assert!(control_record.enforced.as_ref().unwrap().rlimits.is_empty());
    assert_eq!(
        control_record.enforced.as_ref().unwrap().unenforced_kinds,
        vec!["memory_bytes".to_owned()]
    );
}

/// 2. Output past the declared cap is bounded and reported as an output-limit
///    failure, never as a decoded response.
#[test]
#[cfg(target_os = "linux")]
fn output_flooding_beyond_the_cap_is_bounded() {
    let fixture = Fixture::new("output");
    let request = request();
    fixture.script(
        "b=\"\"\ni=0\nwhile [ $i -lt 10 ]; do b=\"${b}0123456789\"; i=$((i+1)); done\n\
         i=0\nwhile [ $i -lt 2000 ]; do printf '%s' \"$b\"; i=$((i+1)); done",
    );
    let mut manifest = manifest();
    manifest.limits.timeout_ms = 20_000;
    manifest.limits.output_bytes = 4096;

    let provider = enforced(manifest, &fixture);
    let started = Instant::now();
    assert_eq!(provider.verify(&request), Err(ProviderFailure::OutputLimit));
    assert!(started.elapsed() < Duration::from_secs(10));
    let record = provider.last_execution().unwrap();
    assert_eq!(record.outcome, ProcessOutcome::OutputLimit);
    assert_eq!(record.enforced.as_ref().unwrap().stdout_bytes, 4096);
    // The host stops reading at the cap instead of buffering 200 KiB.
    assert!(
        record.stdout_bytes <= 4096 + 4 + 8192,
        "{}",
        record.summary()
    );

    let mut registry = VerifierRegistry::default();
    registry
        .register(Arc::new(registrable(
            manifest_for_registry(),
            &fixture,
            ProcessExecutionProfile::LinuxRlimitV1,
        )))
        .unwrap();
    let outcome = registry.verify(&request);
    assert_eq!(outcome.verdict, ProposedVerdict::Unknown);
    assert_eq!(outcome.diagnostics[0].code, "QP006");
    assert!(outcome.certificate.is_none());
}

fn manifest_for_registry() -> PluginManifest {
    let mut manifest = manifest();
    manifest.limits.timeout_ms = 20_000;
    manifest.limits.output_bytes = 4096;
    manifest
}

/// 3a. A wall-clock hang is killed within tolerance of the declared deadline.
#[test]
#[cfg(target_os = "linux")]
fn a_wall_clock_hang_is_killed_within_tolerance() {
    let fixture = Fixture::new("wall");
    let request = request();
    fixture.script("/bin/sleep 30");
    let mut manifest = manifest();
    manifest.limits.timeout_ms = 300;
    // A generous CPU budget: an idle sleeper burns no CPU, so only the wall
    // clock can end this run.
    manifest.limits.additional.insert("cpu_seconds".into(), 60);

    let provider = enforced(manifest, &fixture);
    let started = Instant::now();
    assert_eq!(provider.verify(&request), Err(ProviderFailure::Timeout));
    let elapsed = started.elapsed();
    assert!(elapsed >= Duration::from_millis(300), "{elapsed:?}");
    assert!(elapsed < Duration::from_secs(5), "{elapsed:?}");
    let record = provider.last_execution().unwrap();
    assert_eq!(record.outcome, ProcessOutcome::WallClockTimeout);
    assert_eq!(record.enforced.as_ref().unwrap().wall_clock_ms, 300);
    assert_eq!(applied(&record, "cpu_seconds"), Some(60));
}

/// 3b. A CPU spin is killed by `RLIMIT_CPU` well before the wall-clock
///     deadline, so the two limit kinds are independently enforced.
#[test]
#[cfg(target_os = "linux")]
fn a_cpu_spin_is_killed_by_the_declared_cpu_limit() {
    let fixture = Fixture::new("cpu");
    let request = request();
    fixture.script("while :; do :; done");
    let mut manifest = manifest();
    manifest.limits.timeout_ms = 60_000;
    manifest.limits.additional.insert("cpu_seconds".into(), 1);

    let provider = enforced(manifest, &fixture);
    let started = Instant::now();
    let failure = provider.verify(&request).unwrap_err();
    let elapsed = started.elapsed();
    assert_ne!(failure, ProviderFailure::Timeout);
    assert!(elapsed < Duration::from_secs(30), "{elapsed:?}");
    let record = provider.last_execution().unwrap();
    assert_eq!(record.outcome, ProcessOutcome::Terminated, "{failure}");
    // SIGXCPU, or SIGKILL once the soft and hard limits coincide.
    assert!(
        matches!(record.signal, Some(libc_sigkill) if libc_sigkill == 9)
            || matches!(record.signal, Some(sigxcpu) if sigxcpu == 24),
        "{}",
        record.summary()
    );
    assert_eq!(applied(&record, "cpu_seconds"), Some(1));
}

/// 4a. A descendant that outlives its parent does not outlive the execution:
///     the host kills the child's whole process group on every exit path.
#[test]
#[cfg(target_os = "linux")]
fn descendants_do_not_outlive_the_execution() {
    let fixture = Fixture::new("descendant");
    let request = request();
    fixture.script("(/bin/sleep 1; : > descendant.marker) &\nexit 0");
    let mut manifest = manifest();
    manifest.limits.timeout_ms = 300;

    let provider = enforced(manifest, &fixture);
    // The direct child exits immediately, but the descendant holds the output
    // pipes open, so the run ends on the deadline and the group is killed.
    assert_eq!(provider.verify(&request), Err(ProviderFailure::Timeout));
    let record = provider.last_execution().unwrap();
    assert!(record.enforced.as_ref().unwrap().process_group_kill);
    std::thread::sleep(Duration::from_millis(1500));
    assert!(!fixture.marker().exists());
}

/// 4b. A declared `max_processes` limit is installed, and a package that
///     declares one below its own usage cannot fork at all.
#[test]
#[cfg(target_os = "linux")]
fn a_declared_process_limit_stops_the_fixture_from_forking() {
    let fixture = Fixture::new("nproc");
    let request = request();
    fixture.script("(/bin/sleep 1; : > descendant.marker) &\nexit 0");
    let mut manifest = manifest();
    manifest.limits.timeout_ms = 5_000;
    // RLIMIT_NPROC counts every process of the *real UID*, not of this process
    // tree, so a value below the account's existing usage fails every fork.
    // That is what makes it enforceable here and what makes it a weak
    // per-execution bound in general; the group kill above is the real
    // containment for descendants.
    manifest.limits.additional.insert("max_processes".into(), 1);

    let provider = enforced(manifest, &fixture);
    let failure = provider.verify(&request).unwrap_err();
    assert_ne!(failure, ProviderFailure::Timeout);
    let record = provider.last_execution().unwrap();
    assert_eq!(applied(&record, "max_processes"), Some(1));
    assert_eq!(record.outcome, ProcessOutcome::Terminated, "{failure}");
    std::thread::sleep(Duration::from_millis(1500));
    assert!(!fixture.marker().exists());
}

/// 5. A declared limit kind the profile cannot enforce is refused before
///    anything is spawned. The refusal is explicit and is never a downgrade to
///    the unenforcing profile.
#[test]
#[cfg(target_os = "linux")]
fn a_declared_but_unenforceable_limit_kind_is_refused() {
    let fixture = Fixture::new("refusal");
    let request = request();
    fixture.response(&request);
    fixture.script(": > descendant.marker\n/bin/cat response.bin");
    let mut manifest = manifest();
    manifest
        .limits
        .additional
        .insert("network_bandwidth_bps".into(), 1_000_000);

    let provider = enforced(manifest.clone(), &fixture);
    let precheck = provider.enforced_limits().unwrap_err();
    assert!(
        precheck.to_string().contains("network_bandwidth_bps")
            && precheck.to_string().contains("linux-rlimit-v1"),
        "{precheck}"
    );
    let failure = provider.verify(&request).unwrap_err();
    assert_eq!(failure, precheck);
    let record = provider.last_execution().unwrap();
    assert_eq!(record.outcome, ProcessOutcome::RefusedByProfile);
    assert!(record.enforced.is_none());
    // Refused *before* spawning: the fixture never ran.
    assert!(!fixture.marker().exists());

    let mut registry = VerifierRegistry::default();
    registry
        .register(Arc::new(registrable(
            manifest.clone(),
            &fixture,
            ProcessExecutionProfile::LinuxRlimitV1,
        )))
        .unwrap();
    let outcome = registry.verify(&request);
    assert_eq!(outcome.verdict, ProposedVerdict::Unknown);
    assert_eq!(outcome.diagnostics[0].code, "QP006");
    assert!(
        outcome.diagnostics[0]
            .message
            .contains("network_bandwidth_bps")
    );
    assert!(outcome.certificate.is_none());

    // The legacy profile still runs it, and says so: it enforces neither the
    // unrepresented kind nor `memory_bytes`.
    let control = legacy(manifest, &fixture);
    assert_eq!(
        control.verify(&request).unwrap().verdict,
        ProposedVerdict::Unknown
    );
    let control_record = control.last_execution().unwrap();
    assert_eq!(
        control_record.enforced.as_ref().unwrap().unenforced_kinds,
        vec![
            "memory_bytes".to_owned(),
            "network_bandwidth_bps".to_owned()
        ]
    );
    assert!(fixture.marker().exists());
}

/// 6. A well-behaved provider under generous limits still completes, and the
///    record states exactly what was enforced.
#[test]
#[cfg(target_os = "linux")]
fn a_well_behaved_provider_completes_under_the_enforced_profile() {
    let fixture = Fixture::new("wellbehaved");
    let request = request();
    fixture.response(&request);
    fixture.script("/bin/cat response.bin");
    let mut manifest = manifest();
    manifest.limits.timeout_ms = 20_000;
    manifest.limits.memory_bytes = 256 * 1024 * 1024;
    manifest.limits.additional.insert("cpu_seconds".into(), 30);
    manifest
        .limits
        .additional
        .insert("max_file_bytes".into(), 1024 * 1024);
    manifest
        .limits
        .additional
        .insert("max_open_files".into(), 256);

    let provider = enforced(manifest, &fixture);
    let response = provider.verify(&request).unwrap();
    assert_eq!(response.verdict, ProposedVerdict::Unknown);
    assert_eq!(response.request_id, request.request_id);
    let record = provider.last_execution().unwrap();
    assert_eq!(record.outcome, ProcessOutcome::Completed);
    assert!(record.detail.is_empty());
    assert_eq!(record.exit_code, Some(0));
    let enforced = record.enforced.as_ref().unwrap();
    assert!(enforced.unenforced_kinds.is_empty());
    assert!(enforced.process_group_kill);
    assert_eq!(enforced.stderr_bytes, PROVIDER_STDERR_BYTES);
    assert_eq!(
        enforced
            .rlimits
            .iter()
            .map(|limit| (limit.kind.as_str(), limit.soft, limit.source))
            .collect::<Vec<_>>(),
        vec![
            ("core_bytes", 0, LimitSource::ProfileDefault),
            ("cpu_seconds", 30, LimitSource::Declared),
            ("max_file_bytes", 1024 * 1024, LimitSource::Declared),
            ("max_open_files", 256, LimitSource::Declared),
            ("memory_bytes", 256 * 1024 * 1024, LimitSource::Declared),
        ]
    );
    assert!(record.summary().contains("linux-rlimit-v1"));
}

/// An undeclared CPU budget is derived from the wall clock rather than left
/// out, and the derivation is recorded as derived, not as a declaration.
#[test]
#[cfg(target_os = "linux")]
fn an_undeclared_cpu_budget_is_derived_and_labeled() {
    let fixture = Fixture::new("derived");
    let mut manifest = manifest();
    manifest.limits.timeout_ms = 2_500;
    let provider = enforced(manifest, &fixture);
    let enforced = provider.enforced_limits().unwrap();
    let cpu = enforced
        .rlimits
        .iter()
        .find(|limit| limit.kind == "cpu_seconds")
        .unwrap();
    assert_eq!(cpu.soft, 4);
    assert_eq!(cpu.source, LimitSource::DerivedFromWallClock);
}

/// The enforceable-kind inventory is the profile's published contract, and the
/// legacy profile publishes an empty one rather than an optimistic list.
#[test]
fn the_enforceable_kind_inventory_is_explicit() {
    assert_eq!(
        enforceable_limit_kinds(ProcessExecutionProfile::UnenforcedLegacy),
        Vec::<&str>::new()
    );
    let linux = enforceable_limit_kinds(ProcessExecutionProfile::LinuxRlimitV1);
    if cfg!(target_os = "linux") {
        assert_eq!(
            linux,
            vec![
                "core_bytes",
                "cpu_seconds",
                "max_file_bytes",
                "max_open_files",
                "max_processes",
                "memory_bytes",
            ]
        );
    } else {
        // Off Linux the profile enforces nothing and must refuse rather than
        // claim a bound it cannot install.
        assert!(linux.is_empty());
    }
}

/// Off Linux the profile is unavailable and every execution is refused; on
/// Linux it is available. Either way there is no silent third behavior.
#[test]
fn the_profile_is_target_qualified() {
    let fixture = Fixture::new("target");
    let provider = enforced(manifest(), &fixture);
    let limits = provider.enforced_limits();
    if cfg!(target_os = "linux") {
        assert!(limits.is_ok());
    } else {
        let failure = limits.unwrap_err();
        assert!(failure.to_string().contains("unavailable on this target"));
    }
}

/// The default profile is unchanged, so no existing caller moves without an
/// explicit selection.
#[test]
fn the_default_profile_is_the_unenforced_legacy_one() {
    let fixture = Fixture::new("default");
    let provider = UnverifiedProvider::new(manifest(), fixture.path()).unwrap();
    assert_eq!(
        provider.execution_profile(),
        ProcessExecutionProfile::UnenforcedLegacy
    );
    assert!(provider.last_execution().is_none());
    assert!(provider.enforced_limits().unwrap().rlimits.is_empty());
}
