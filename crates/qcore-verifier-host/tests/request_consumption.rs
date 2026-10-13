//! A process provider's verdict is decided by whether it consumed its request,
//! never by the order of the host's write and the provider's exit (card
//! `HOST-UNREAD-REQUEST-ACCEPTANCE-BY-SCHEDULING`).
//!
//! Each case is one fixture shape crossed with a request frame under or over
//! the pipe capacity. The card's harness runs this binary repeatedly under a
//! host stall and under `LD_PRELOAD` delays of the host's first request write
//! and of its close, which move the provider's exit to either side of the
//! write; every run must give the same outcome.
#![cfg(unix)]

use qcore_verifier_host::*;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

// Only `write_executable` is used: each case spells its own reading, or its
// deliberate lack of one.
#[allow(dead_code)]
#[path = "support/process_fixture.rs"]
mod process_fixture;
use process_fixture::write_executable;

/// Over every pipe capacity a host can grow to (Linux's default
/// `pipe-max-size` is 1 MiB), so the frame cannot be buffered whole.
const LARGE: usize = 8 * 1024 * 1024;

fn request(id_bytes: usize) -> VerificationRequest {
    VerificationRequest {
        protocol_version: 1,
        evidence_version: 1,
        request_id: "r".repeat(id_bytes),
        plugin_id: PluginId::new("example.consumption").unwrap(),
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
        plugin_id: PluginId::new("example.consumption").unwrap(),
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
            // Generous: the harness stalls the host for tens of milliseconds
            // at a time, and no case here is about the deadline.
            timeout_ms: 20_000,
            output_bytes: 2 * LARGE,
            ..PluginLimits::default()
        },
        trust_class: PluginTrustClass::FirstParty,
        certificate_checker: "example.untrusted_claims".into(),
        lean_theorem_family: "QCore.VerifierPlugin".into(),
    }
}

/// One package directory holding `provider.sh` and the valid response frame
/// for the case's request, so a non-reading fixture can still answer
/// correctly: only its consumption distinguishes it from a reading one.
struct Case {
    root: PathBuf,
    request: VerificationRequest,
    response: VerificationResponse,
    frame_bytes: usize,
}

impl Case {
    fn new(tag: &str, id_bytes: usize) -> Self {
        let root = std::env::temp_dir().join(format!(
            "qcore-consumption-{tag}-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir(&root).unwrap();
        let request = request(id_bytes);
        let response = VerificationResponse {
            protocol_version: request.protocol_version,
            request_id: request.request_id.clone(),
            plugin_id: request.plugin_id.clone(),
            verdict: ProposedVerdict::Unknown,
            evidence: None,
            diagnostics: vec![],
        };
        let limit = manifest().limits.output_bytes;
        fs::write(
            root.join("response.bin"),
            encode_protocol_frame(&response, limit).unwrap(),
        )
        .unwrap();
        let frame_bytes = encode_protocol_frame(&request, limit).unwrap().len();
        Self {
            root,
            request,
            response,
            frame_bytes,
        }
    }

    /// Runs `body` as the provider and returns the result with its record.
    fn run(
        &self,
        body: &str,
    ) -> (
        Result<VerificationResponse, ProviderFailure>,
        ProcessExecutionRecord,
    ) {
        write_executable(
            &self.root.join("provider.sh"),
            format!("#!/bin/sh\n{body}\n").as_bytes(),
        );
        let provider = UnverifiedProvider::new(manifest(), &self.root).unwrap();
        let result = provider.verify(&self.request);
        let record = provider.last_execution().expect("the execution is recorded");
        (result, record)
    }
}

impl Drop for Case {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

/// A provider that exits successfully without consuming its whole request is
/// refused, whichever of its exit and the host's write came first.
fn refused(tag: &str, id_bytes: usize, body: &str) {
    let case = Case::new(tag, id_bytes);
    let (result, record) = case.run(body);
    assert!(result.is_err(), "{tag}: an unconsumed request was accepted");
    assert_ne!(record.outcome, ProcessOutcome::Completed, "{tag}");
    assert_eq!(record.exit_code, Some(0), "{tag}: {record:?}");
}

/// A provider that consumes its whole request and answers is accepted.
fn accepted(tag: &str, id_bytes: usize, body: impl Fn(&Case) -> String) {
    let case = Case::new(tag, id_bytes);
    let (result, record) = case.run(&body(&case));
    assert_eq!(result, Ok(case.response.clone()), "{tag}: {record:?}");
    assert_eq!(record.outcome, ProcessOutcome::Completed, "{tag}");
}

const ANSWER: &str = "/bin/cat response.bin";

#[test]
fn a_provider_that_never_reads_a_small_request_is_refused() {
    refused("silent-small", 64, ANSWER);
}

#[test]
fn a_provider_that_never_reads_a_small_request_and_exits_late_is_refused() {
    // The sleep lets the host's write land in the pipe buffer before the exit:
    // the branch that used to be accepted.
    refused(
        "silent-small-late",
        64,
        &format!("/bin/sleep 0.05\n{ANSWER}"),
    );
}

#[test]
fn a_provider_that_never_reads_a_large_request_is_refused() {
    refused("silent-large", LARGE, ANSWER);
}

#[test]
fn a_provider_that_closes_its_input_and_answers_is_refused() {
    refused("closed-small", 64, &format!("exec 0<&-\n{ANSWER}"));
}

#[test]
fn a_provider_that_reads_part_of_its_request_is_refused() {
    refused(
        "partial-small",
        64,
        &format!("/bin/dd bs=1 count=4 of=/dev/null 2>/dev/null\n{ANSWER}"),
    );
}

#[test]
fn a_provider_that_reads_its_request_to_end_of_file_is_accepted() {
    accepted("eof-small", 64, |_| format!("/bin/cat > /dev/null\n{ANSWER}"));
}

#[test]
fn a_provider_that_reads_a_large_request_to_end_of_file_is_accepted() {
    accepted("eof-large", LARGE, |_| {
        format!("/bin/cat > /dev/null\n{ANSWER}")
    });
}

#[test]
fn a_provider_that_reads_exactly_its_frame_and_exits_before_the_close_is_accepted() {
    // Reads the frame's exact length and never waits for end of file, so it
    // can exit before the host has closed its end of the pipe.
    accepted("exact-small", 64, |case| {
        format!(
            "/usr/bin/head -c {} > /dev/null\n{ANSWER}",
            case.frame_bytes
        )
    });
}

#[test]
fn a_provider_that_reads_exactly_a_large_frame_is_accepted() {
    accepted("exact-large", LARGE, |case| {
        format!(
            "/usr/bin/head -c {} > /dev/null\n{ANSWER}",
            case.frame_bytes
        )
    });
}
