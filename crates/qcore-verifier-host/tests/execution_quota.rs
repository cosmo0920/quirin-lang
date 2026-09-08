//! Adversarial fixtures for the per-execution process quota and the refusal
//! grounds P03 recorded as implemented-but-never-executed
//! (`docs/reports/remaining-p03-f/README.md`).
//!
//! The quota's whole point is that it counts *this execution's tree*, not the
//! invoking account: P02's `RLIMIT_NPROC` binds only by being below the
//! account's existing usage, so it cannot express "this package may hold four
//! tasks" at all. Every positive case below therefore measures the kernel's own
//! counters for a cgroup that existed for exactly one execution, and every
//! negative case asserts a pre-spawn refusal with the package demonstrably
//! never run.

use qcore_verifier_host::*;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

struct Fixture {
    root: PathBuf,
}

impl Fixture {
    fn new(tag: &str) -> Self {
        let root = std::env::temp_dir().join(format!(
            "qcore-p03f-{tag}-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir(&root).unwrap();
        fs::create_dir(root.join("package")).unwrap();
        fs::create_dir(root.join("writable")).unwrap();
        Self { root }
    }

    fn package(&self) -> PathBuf {
        self.root.join("package")
    }

    fn writable(&self) -> PathBuf {
        self.root.join("writable")
    }

    #[cfg(unix)]
    fn script(&self, body: &str) {
        let path = self.package().join("provider.sh");
        fs::write(&path, format!("#!/bin/sh\n{body}\n")).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).unwrap();
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
            self.package().join("response.bin"),
            encode_protocol_frame(&response, DEFAULT_MAX_FRAME_BYTES).unwrap(),
        )
        .unwrap();
    }

    /// A marker inside the package root. The confining profile makes the
    /// package root read-only, so a refused execution that had actually
    /// spawned would have spawned *unconfined* and left this behind. Its
    /// absence is what separates "refused before spawning" from "spawned and
    /// denied".
    fn ran_marker(&self) -> PathBuf {
        self.package().join("ran.marker")
    }

    fn policy(&self) -> ConfinementPolicy {
        ConfinementPolicy::linux_runtime_defaults()
            .admitting_read_write(self.writable().to_string_lossy().into_owned())
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
        plugin_id: PluginId::new("example.quota").unwrap(),
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
        plugin_id: PluginId::new("example.quota").unwrap(),
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

// ---------------------------------------------------------------------------
// Portable: the plan, the target qualification and the override's direction.
// These run on every target, including the Windows CI job, which is where the
// non-Linux refusal ground is actually executed rather than argued.
// ---------------------------------------------------------------------------

/// Refusal ground 1, executed: off Linux both enforcing profiles are
/// unavailable and every execution is refused, including the confining one,
/// which P03's Linux-only suite could never reach.
#[test]
fn both_enforcing_profiles_are_target_qualified() {
    let fixture = Fixture::new("target");
    for profile in [
        ProcessExecutionProfile::LinuxRlimitV1,
        ProcessExecutionProfile::LinuxConfinedV1,
    ] {
        let mut provider = ProcessProvider::new(manifest(), fixture.package())
            .unwrap()
            .with_execution_profile(profile);
        if profile.confines() {
            provider = provider.with_confinement_policy(fixture.policy());
        }
        let limits = provider.enforced_limits();
        if cfg!(target_os = "linux") {
            assert!(limits.is_ok(), "{:?}", limits.err());
        } else {
            let failure = limits.unwrap_err();
            assert!(
                failure.to_string().contains("unavailable on this target"),
                "{failure}"
            );
            // Off Linux nothing enforceable exists, so a declared kind of any
            // shape refuses rather than being silently dropped.
            assert!(enforceable_limit_kinds(profile).is_empty());
        }
    }
}

/// Refusal ground 1 again, this time for the *quota* specifically: off Linux a
/// declared quota kind cannot be enforced by any profile.
#[test]
fn a_declared_quota_is_refused_off_linux() {
    let fixture = Fixture::new("target-quota");
    let mut manifest = manifest();
    manifest
        .limits
        .additional
        .insert(MAX_EXECUTION_PROCESSES.into(), 4);
    let provider = ProcessProvider::new(manifest, fixture.package())
        .unwrap()
        .with_execution_profile(ProcessExecutionProfile::LinuxConfinedV1)
        .with_confinement_policy(fixture.policy());
    let limits = provider.enforced_limits();
    if cfg!(target_os = "linux") {
        // On Linux the answer depends on whether this host delegates a cgroup
        // subtree; either way it is never "ran without the quota".
        match limits {
            Ok(enforced) => assert_eq!(
                enforced.process_quota.as_ref().map(|quota| quota.max_processes),
                Some(4)
            ),
            Err(failure) => assert!(
                failure.to_string().contains(MAX_EXECUTION_PROCESSES),
                "{failure}"
            ),
        }
    } else {
        let failure = limits.unwrap_err();
        assert!(
            failure.to_string().contains("unavailable on this target"),
            "{failure}"
        );
    }
}

/// The quota kind is enforceable by exactly one profile, and the other two
/// refuse it rather than running the package with a bound nothing installs.
#[test]
fn only_the_confining_profile_accepts_the_quota_kind() {
    let fixture = Fixture::new("kinds");
    let mut manifest = manifest();
    manifest
        .limits
        .additional
        .insert(MAX_EXECUTION_PROCESSES.into(), 4);
    for profile in [
        ProcessExecutionProfile::UnenforcedLegacy,
        ProcessExecutionProfile::LinuxRlimitV1,
    ] {
        let provider = ProcessProvider::new(manifest.clone(), fixture.package())
            .unwrap()
            .with_execution_profile(profile);
        match profile {
            // The legacy profile enforces no declared kind at all and says so
            // in `unenforced_kinds` rather than pretending otherwise.
            ProcessExecutionProfile::UnenforcedLegacy => {
                let enforced = provider.enforced_limits().unwrap();
                assert!(
                    enforced
                        .unenforced_kinds
                        .contains(&MAX_EXECUTION_PROCESSES.to_owned())
                );
                assert!(enforced.process_quota.is_none());
                assert!(enforced.summary().contains("process_quota=none"));
            }
            _ => {
                if cfg!(target_os = "linux") {
                    let failure = provider.enforced_limits().unwrap_err();
                    assert!(
                        failure.to_string().contains("installs no per-execution process quota"),
                        "{failure}"
                    );
                    assert!(failure.to_string().contains("linux-confined-v1"), "{failure}");
                }
            }
        }
    }
}

/// A zero quota admits no process at all, including the package's own
/// entrypoint, so it is refused rather than installed as written.
#[test]
#[cfg(target_os = "linux")]
fn a_zero_quota_is_refused() {
    let fixture = Fixture::new("zero");
    let mut manifest = manifest();
    manifest
        .limits
        .additional
        .insert(MAX_EXECUTION_PROCESSES.into(), 0);
    let provider = ProcessProvider::new(manifest, fixture.package())
        .unwrap()
        .with_execution_profile(ProcessExecutionProfile::LinuxConfinedV1)
        .with_confinement_policy(fixture.policy());
    let failure = provider.enforced_limits().unwrap_err();
    assert!(failure.to_string().contains("no process at all"), "{failure}");
}

/// The availability override can only *remove* a mechanism. There is no
/// variant that forces one available, so no use of it — here or anywhere —
/// can produce an execution that ran with weaker enforcement than the kernel
/// offered.
#[test]
fn the_availability_override_is_one_directional() {
    assert_eq!(MechanismProbe::default(), MechanismProbe::AsProbed);
    assert_eq!(MechanismProbe::AsProbed.forced_refusal("landlock"), None);
    assert!(
        MechanismProbe::ForcedUnavailable
            .forced_refusal("landlock")
            .is_some()
    );
    // Every variant either asks the kernel or refuses; a `match` that is
    // exhaustive over exactly these two is the structural half of the claim.
    for probe in [MechanismProbe::AsProbed, MechanismProbe::ForcedUnavailable] {
        match probe {
            MechanismProbe::AsProbed => assert!(probe.forced_refusal("m").is_none()),
            MechanismProbe::ForcedUnavailable => assert!(probe.forced_refusal("m").is_some()),
        }
    }
    let fixture = Fixture::new("override-direction");
    let provider = ProcessProvider::new(manifest(), fixture.package()).unwrap();
    assert_eq!(
        provider.mechanism_availability(),
        MechanismAvailability::default()
    );
    assert!(!provider.mechanism_availability().any_forced());
}

// ---------------------------------------------------------------------------
// Linux: the mechanism itself, and the refusal grounds executed with a forced
// override or a genuinely restricted environment.
// ---------------------------------------------------------------------------

/// Whether this host delegates a writable cgroup v2 subtree with the `pids`
/// controller. Where it does not, the quota mechanism is unavailable and the
/// profile refuses — which is itself asserted below, so no case here is
/// silently skipped.
#[cfg(target_os = "linux")]
fn quota_available(fixture: &Fixture) -> Option<ProcessQuota> {
    let mut manifest = manifest();
    manifest
        .limits
        .additional
        .insert(MAX_EXECUTION_PROCESSES.into(), 4);
    ProcessProvider::new(manifest, fixture.package())
        .unwrap()
        .with_execution_profile(ProcessExecutionProfile::LinuxConfinedV1)
        .with_confinement_policy(fixture.policy())
        .enforced_limits()
        .ok()
        .and_then(|enforced| enforced.process_quota)
}

/// The quota bounds a fork storm at the declared number of tasks, and the
/// kernel's own counters say so. Where the mechanism is unavailable, the same
/// declaration is refused before the spawn instead — never run unbounded.
#[test]
#[cfg(target_os = "linux")]
fn a_declared_quota_bounds_a_fork_storm_in_this_execution_only() {
    let fixture = Fixture::new("storm");
    let request = request();
    fixture.response(&request);
    // Fork 40 sleepers, count how many started, then answer. `$!` is only set
    // by a successful background start, so the counter is the shell's own
    // observation of what the kernel allowed.
    // The storm runs in a subshell so a fork the quota refuses kills only the
    // storm; the package then still answers, which shows the bound held
    // without the fixture simply dying.
    fixture.script(
        "(i=0; while [ $i -lt 40 ]; do /bin/sleep 3 & i=$((i+1)); done) 2>/dev/null\n\
         /bin/cat response.bin",
    );
    let mut manifest = manifest();
    manifest
        .limits
        .additional
        .insert(MAX_EXECUTION_PROCESSES.into(), 4);

    let provider = ProcessProvider::new(manifest.clone(), fixture.package())
        .unwrap()
        .with_execution_profile(ProcessExecutionProfile::LinuxConfinedV1)
        .with_confinement_policy(fixture.policy());
    let Some(quota) = quota_available(&fixture) else {
        // Refusal ground 6, executed on a host without a delegated subtree.
        let failure = provider.verify(&request).unwrap_err();
        assert!(
            failure.to_string().contains(MAX_EXECUTION_PROCESSES),
            "{failure}"
        );
        let record = provider.last_execution().unwrap();
        assert_eq!(record.outcome, ProcessOutcome::RefusedByProfile);
        assert!(record.enforced.is_none());
        assert!(record.quota_observed.is_none());
        assert!(!fixture.ran_marker().exists());
        return;
    };
    assert_eq!(quota.mechanism, "cgroup-v2-pids");
    assert_eq!(quota.max_processes, 4);
    assert!(quota.parent_subtree_control.split_whitespace().any(|c| c == "pids"));

    let outcome = provider.verify(&request);
    let record = provider.last_execution().unwrap();
    let observed = record
        .quota_observed
        .clone()
        .expect("a quota execution records what the kernel counted");
    assert_eq!(observed.max_processes, 4);
    assert_eq!(observed.mechanism, "cgroup-v2-pids");
    // The kernel never let the tree exceed the declared bound...
    if let Some(peak) = observed.peak_processes {
        assert!(peak <= 4, "peak {peak} exceeded the declared quota of 4");
        assert!(peak >= 1, "the child itself is a member of its own cgroup");
    }
    // ...and it refused the forks beyond it. The fixture asks for 40 tasks
    // against a quota of 4, so refusals are not a scheduling artifact.
    assert!(
        observed.quota_rejections.unwrap_or(0) > 0,
        "a 40-deep fork storm under a quota of 4 must be refused at least once: {}",
        observed.summary()
    );
    assert!(observed.released, "{}", observed.summary());
    assert!(!std::path::Path::new(&observed.cgroup_path).exists());
    assert!(record.summary().contains("process_quota="));
    // The run itself completed or was terminated by its own failed forks;
    // either way it is never a program `Unsafe`, and the bound held.
    assert!(
        matches!(
            record.outcome,
            ProcessOutcome::Completed | ProcessOutcome::Terminated | ProcessOutcome::Io
        ),
        "{:?}: {outcome:?}",
        record.outcome
    );

    // The control: the identical fixture under a quota above what it asks for
    // reaches far more than four tasks. Without this the row above would show
    // only that the fixture is small, not that the quota bound it.
    let mut generous = manifest.clone();
    generous
        .limits
        .additional
        .insert(MAX_EXECUTION_PROCESSES.into(), 64);
    let control = ProcessProvider::new(generous, fixture.package())
        .unwrap()
        .with_execution_profile(ProcessExecutionProfile::LinuxConfinedV1)
        .with_confinement_policy(fixture.policy());
    control.verify(&request).unwrap();
    let control = control
        .last_execution()
        .unwrap()
        .quota_observed
        .expect("the control installs its own quota");
    assert_eq!(control.quota_rejections, Some(0));
    if let (Some(bounded), Some(free)) = (observed.peak_processes, control.peak_processes) {
        assert!(
            free > bounded,
            "the same fixture reached {free} tasks unbounded and {bounded} under a quota of 4"
        );
        assert!(free > 4, "the control must exceed the bounded run's quota");
    }
    assert!(control.released, "{}", control.summary());
}

/// The control: the same fixture with no declared quota installs none and is
/// not bounded by one. Without this the row above would prove nothing about
/// the mechanism.
#[test]
#[cfg(target_os = "linux")]
fn the_same_fixture_without_a_declared_quota_installs_none() {
    let fixture = Fixture::new("storm-control");
    let request = request();
    fixture.response(&request);
    fixture.script("/bin/sleep 0.05 &\n/bin/sleep 0.05 &\n/bin/cat response.bin");

    let provider = ProcessProvider::new(manifest(), fixture.package())
        .unwrap()
        .with_execution_profile(ProcessExecutionProfile::LinuxConfinedV1)
        .with_confinement_policy(fixture.policy());
    provider.verify(&request).unwrap();
    let record = provider.last_execution().unwrap();
    assert_eq!(record.outcome, ProcessOutcome::Completed);
    assert!(record.quota_observed.is_none());
    let enforced = record.enforced.as_ref().unwrap();
    assert!(enforced.process_quota.is_none());
    assert!(enforced.summary().contains("process_quota=none"));
    // Serialized evidence omits both optional keys entirely, so a record
    // written before these fields existed is byte-identical.
    let json = serde_json::to_value(enforced).unwrap();
    assert!(json.get("process_quota").is_none());
    let json = serde_json::to_value(&record).unwrap();
    assert!(json.get("quota_observed").is_none());
}

/// The quota is per *tree*, not per UID: it binds at a value far above the
/// account's usage, which `RLIMIT_NPROC` cannot do. This is the deterministic
/// companion to P02's load-flaky `max_processes` fixture — that assertion is
/// deliberately unchanged, and this one measures the same intent without
/// depending on the account's process count at all.
#[test]
#[cfg(target_os = "linux")]
fn the_quota_binds_without_depending_on_the_accounts_process_count() {
    let fixture = Fixture::new("per-tree");
    let request = request();
    fixture.response(&request);
    fixture.script("(/bin/sleep 1; : > ../writable/descendant.marker) &\n/bin/cat response.bin");
    let mut manifest = manifest();
    // Two tasks: the entrypoint and exactly one descendant. The account's own
    // process count is in the hundreds and is irrelevant to this bound, which
    // is the whole difference from `RLIMIT_NPROC`.
    manifest
        .limits
        .additional
        .insert(MAX_EXECUTION_PROCESSES.into(), 2);

    let provider = ProcessProvider::new(manifest, fixture.package())
        .unwrap()
        .with_execution_profile(ProcessExecutionProfile::LinuxConfinedV1)
        .with_confinement_policy(fixture.policy());
    if quota_available(&fixture).is_none() {
        assert!(provider.verify(&request).is_err());
        assert_eq!(
            provider.last_execution().unwrap().outcome,
            ProcessOutcome::RefusedByProfile
        );
        return;
    }
    let _ = provider.verify(&request);
    let record = provider.last_execution().unwrap();
    let observed = record.quota_observed.unwrap();
    assert_eq!(observed.max_processes, 2);
    if let Some(peak) = observed.peak_processes {
        assert!(peak <= 2, "peak {peak} exceeded the declared quota of 2");
    }
    assert!(observed.released, "{}", observed.summary());
    // The descendant is killed with the group before it can write, exactly as
    // P02's fixture asserts for its own mechanism.
    std::thread::sleep(Duration::from_millis(1500));
    assert!(!fixture.writable().join("descendant.marker").exists());
}

/// Refusal ground 6, executed deterministically: a forced-unavailable quota
/// mechanism refuses before the spawn, records no enforced limits and never
/// runs the package.
#[test]
#[cfg(target_os = "linux")]
fn a_forced_unavailable_quota_mechanism_refuses_before_spawning() {
    let fixture = Fixture::new("forced-quota");
    let request = request();
    fixture.response(&request);
    fixture.script(": > ran.marker\n/bin/cat response.bin");
    let mut manifest = manifest();
    manifest
        .limits
        .additional
        .insert(MAX_EXECUTION_PROCESSES.into(), 4);

    let provider = ProcessProvider::new(manifest, fixture.package())
        .unwrap()
        .with_execution_profile(ProcessExecutionProfile::LinuxConfinedV1)
        .with_confinement_policy(fixture.policy())
        .with_mechanism_availability(MechanismAvailability {
            process_quota: MechanismProbe::ForcedUnavailable,
            ..MechanismAvailability::default()
        });
    let precheck = provider.enforced_limits().unwrap_err();
    assert!(precheck.to_string().contains("availability override"), "{precheck}");
    assert!(
        precheck
            .to_string()
            .contains("not a statement about this kernel"),
        "{precheck}"
    );
    assert_eq!(provider.verify(&request).unwrap_err(), precheck);
    let record = provider.last_execution().unwrap();
    assert_eq!(record.outcome, ProcessOutcome::RefusedByProfile);
    assert!(record.enforced.is_none());
    assert!(record.quota_observed.is_none());
    assert!(!fixture.ran_marker().exists());
}

/// Refusal ground 4, executed: a kernel mechanism the profile needs is absent.
/// Both halves — Landlock and the user namespace — refuse before the spawn,
/// with no downgrade to an unconfined run, and the registry sees `Unknown`
/// under `QP006` with no certificate.
#[test]
#[cfg(target_os = "linux")]
fn a_missing_confinement_mechanism_refuses_before_spawning() {
    let fixture = Fixture::new("forced-mechanism");
    let request = request();
    fixture.response(&request);
    fixture.script(": > ran.marker\n/bin/cat response.bin");

    for (label, availability) in [
        (
            "Landlock",
            MechanismAvailability {
                landlock: MechanismProbe::ForcedUnavailable,
                ..MechanismAvailability::default()
            },
        ),
        (
            "user and network namespace",
            MechanismAvailability {
                user_namespace: MechanismProbe::ForcedUnavailable,
                ..MechanismAvailability::default()
            },
        ),
    ] {
        let provider = ProcessProvider::new(manifest(), fixture.package())
            .unwrap()
            .with_execution_profile(ProcessExecutionProfile::LinuxConfinedV1)
            .with_confinement_policy(fixture.policy())
            .with_mechanism_availability(availability);
        assert!(provider.mechanism_availability().any_forced());
        let precheck = provider.enforced_limits().unwrap_err();
        assert!(precheck.to_string().contains(label), "{precheck}");
        assert!(
            precheck.to_string().contains("cannot confine this execution"),
            "{precheck}"
        );
        assert_eq!(provider.verify(&request).unwrap_err(), precheck);
        let record = provider.last_execution().unwrap();
        assert_eq!(record.outcome, ProcessOutcome::RefusedByProfile);
        assert!(record.enforced.is_none());
        assert!(!fixture.ran_marker().exists());

        let mut registry = VerifierRegistry::default();
        registry
            .register(Arc::new(
                ProcessProvider::new(manifest(), fixture.package())
                    .unwrap()
                    .with_execution_profile(ProcessExecutionProfile::LinuxConfinedV1)
                    .with_confinement_policy(fixture.policy())
                    .with_mechanism_availability(availability),
            ))
            .unwrap();
        let outcome = registry.verify(&request);
        assert_eq!(outcome.verdict, ProposedVerdict::Unknown);
        assert_eq!(outcome.diagnostics[0].code, "QP006");
        assert!(outcome.certificate.is_none());
        assert!(!fixture.ran_marker().exists());
    }
}

/// A forced mechanism never *enables* anything: with the override set, a
/// profile that needs the mechanism refuses, and — critically — a profile that
/// does not need it is unaffected, so the override cannot be used to make an
/// execution weaker.
#[test]
#[cfg(target_os = "linux")]
fn a_forced_override_never_weakens_an_execution() {
    let fixture = Fixture::new("forced-weaken");
    let request = request();
    fixture.response(&request);
    fixture.script("/bin/cat response.bin");

    let all_forced = MechanismAvailability {
        landlock: MechanismProbe::ForcedUnavailable,
        user_namespace: MechanismProbe::ForcedUnavailable,
        process_quota: MechanismProbe::ForcedUnavailable,
    };
    // The confining profile refuses outright rather than running unconfined.
    let confined = ProcessProvider::new(manifest(), fixture.package())
        .unwrap()
        .with_execution_profile(ProcessExecutionProfile::LinuxConfinedV1)
        .with_confinement_policy(fixture.policy())
        .with_mechanism_availability(all_forced);
    assert!(confined.verify(&request).is_err());
    assert_eq!(
        confined.last_execution().unwrap().outcome,
        ProcessOutcome::RefusedByProfile
    );
    // The limit profile installs no confinement and no quota, so the override
    // changes nothing about it: it still installs exactly the same rlimits.
    let forced = ProcessProvider::new(manifest(), fixture.package())
        .unwrap()
        .with_execution_profile(ProcessExecutionProfile::LinuxRlimitV1)
        .with_mechanism_availability(all_forced);
    let plain = ProcessProvider::new(manifest(), fixture.package())
        .unwrap()
        .with_execution_profile(ProcessExecutionProfile::LinuxRlimitV1);
    assert_eq!(
        forced.enforced_limits().unwrap(),
        plain.enforced_limits().unwrap()
    );
    forced.verify(&request).unwrap();
    assert_eq!(
        forced.last_execution().unwrap().outcome,
        ProcessOutcome::Completed
    );
}

/// The configured route: a written selection reaches the provider, and its
/// fail-closed checks are the provider's own, made earlier.
#[test]
fn a_written_selection_reaches_the_provider() {
    let fixture = Fixture::new("configured");
    // The recorded default keeps the historical profile and confines nothing.
    let provider = ExecutionPolicy::default()
        .apply(ProcessProvider::new(manifest(), fixture.package()).unwrap())
        .unwrap();
    assert_eq!(
        provider.execution_profile(),
        ProcessExecutionProfile::UnenforcedLegacy
    );
    assert!(provider.confinement_policy().is_none());

    let selected: ExecutionPolicy = serde_json::from_str(
        r#"{"version":1,"profile":"linux-confined-v1",
            "confinement":{"base":"linux-runtime-defaults","read_write":["/dev/null"]}}"#,
    )
    .unwrap();
    let provider = selected
        .apply(ProcessProvider::new(manifest(), fixture.package()).unwrap())
        .unwrap();
    assert_eq!(
        provider.execution_profile(),
        ProcessExecutionProfile::LinuxConfinedV1
    );
    assert!(provider.confinement_policy().is_some());

    // A policy under a non-confining profile is refused by the configuration
    // as well as by the provider, so both directions of P03's ground 3 hold.
    let mismatched: ExecutionPolicy = serde_json::from_str(
        r#"{"version":1,"profile":"linux-rlimit-v1",
            "confinement":{"base":"linux-runtime-defaults"}}"#,
    )
    .unwrap();
    let failure = mismatched.validate().unwrap_err();
    assert!(
        failure.to_string().contains("enforces no confinement policy"),
        "{failure}"
    );
    assert!(mismatched
        .apply(ProcessProvider::new(manifest(), fixture.package()).unwrap())
        .is_err());
}
