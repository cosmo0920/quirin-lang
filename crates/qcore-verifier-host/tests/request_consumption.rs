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

//! A process provider's verdict is decided by whether it consumed its request,
//! never by the order of the host's write and the provider's exit (card
//! `HOST-UNREAD-REQUEST-ACCEPTANCE-BY-SCHEDULING`).
//!
//! Each case is one fixture shape crossed with a request frame under or over
//! the pipe capacity. The card's harness runs this binary repeatedly under a
//! host stall and under `LD_PRELOAD` delays of the host's first request write
//! and of its close, which move the provider's exit to either side of the
//! write; every run must give the same outcome.
//!
//! Card `HOST-REQUEST-READ-BY-DETACHED-DESCENDANT` adds the cases whose
//! request pipe a background descendant keeps after the direct child's exit,
//! with both outputs closed: the host decides only as the pipe's last holder,
//! so a descendant that reads the whole request is accepted wherever its reads
//! fall, and one that still holds the pipe at the deadline is refused.
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
        self.run_within(body, manifest().limits.timeout_ms)
    }

    /// Runs `body` under a declared deadline of `timeout_ms`.
    fn run_within(
        &self,
        body: &str,
        timeout_ms: u64,
    ) -> (
        Result<VerificationResponse, ProviderFailure>,
        ProcessExecutionRecord,
    ) {
        write_executable(
            &self.root.join("provider.sh"),
            format!("#!/bin/sh\n{body}\n").as_bytes(),
        );
        let mut manifest = manifest();
        manifest.limits.timeout_ms = timeout_ms;
        let provider = UnverifiedProvider::new(manifest, &self.root).unwrap();
        let result = provider.verify(&self.request);
        let record = provider
            .last_execution()
            .expect("the execution is recorded");
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
    assert!(
        matches!(result, Err(ProviderFailure::RequestUnconsumed(_))),
        "{tag}: {result:?}"
    );
    assert_eq!(record.outcome, ProcessOutcome::RequestUnconsumed, "{tag}");
    assert_eq!(record.exit_code, Some(0), "{tag}: {record:?}");
    assert!(
        record.detail.starts_with("request-unconsumed: "),
        "{record:?}"
    );
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
    accepted("eof-small", 64, |_| {
        format!("/bin/cat > /dev/null\n{ANSWER}")
    });
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

// Card `HOST-REQUEST-READ-BY-DETACHED-DESCENDANT`: a descendant that inherited
// the request pipe as its stdin, with both output pipes closed, so the host's
// output pipes reach end of file when the direct child exits while the
// descendant can still read. Whether its reads land before that moment is
// scheduling; the host's outcome must not be.

/// The declared deadline of a case whose descendant keeps the request pipe
/// past the direct child's exit. Long enough for the harness's stalls, short
/// enough that fifty runs are affordable; the deadline is the only bound a
/// held request pipe has.
const HELD_DEADLINE_MS: u64 = 1_000;

/// Lends the provider's stdin, the request pipe, to a background subshell
/// running `descendant` with its stdout and stderr on `/dev/null`, then runs
/// `direct` in the direct child, which no longer holds the pipe itself.
///
/// The explicit `<&3` is needed: a non-interactive shell gives an
/// asynchronous list `/dev/null` as stdin unless the list redirects it.
fn lend_request(descendant: &str, direct: &str) -> String {
    format!(
        "exec 3<&0\n\
         (exec 3<&- >/dev/null 2>/dev/null; {descendant}) <&3 &\n\
         exec 3<&- 0<&-\n\
         {direct}"
    )
}

/// The same, but the descendant leaves the provider's process group and
/// session (`setsid -f`), so the host's group kill cannot reach it. It ends on
/// its own, three deadlines later.
fn lend_request_detached(direct: &str) -> String {
    format!(
        "exec 3<&0\n\
         /usr/bin/setsid -f /bin/sh -c 'exec >/dev/null 2>/dev/null; exec /bin/sleep 3' <&3\n\
         exec 3<&- 0<&-\n\
         {direct}"
    )
}

/// What the host decides off Linux for one of this card's cases. There the
/// pipe's other holders cannot be observed, and the host decides, as before
/// the card, on the bytes left unread at the direct child's exit (the user's
/// decision, "keep previous macOS"; `HOST-REQUEST-HOLDERS-OFF-LINUX-UNOBSERVABLE`).
/// Never executed: no macOS host has run this binary
/// (`HOST-REQUEST-PROBE-OFF-LINUX-UNEXECUTED`).
#[derive(Clone, Copy)]
enum OffLinux {
    /// Nothing reads the request, so it is refused as unconsumed.
    Unconsumed,
    /// A descendant reads it on its own schedule: accepted when its reads land
    /// before the decision, refused as unconsumed otherwise. This is the one
    /// shape that stays timing-dependent off Linux.
    EitherWay,
}

/// Asserts the off-Linux outcome `expected`.
fn decided_as_before_off_linux(
    tag: &str,
    expected: OffLinux,
    case: &Case,
    result: &Result<VerificationResponse, ProviderFailure>,
    record: &ProcessExecutionRecord,
) {
    let unconsumed = matches!(result, Err(ProviderFailure::RequestUnconsumed(_)))
        && record.outcome == ProcessOutcome::RequestUnconsumed;
    let completed =
        result.as_ref() == Ok(&case.response) && record.outcome == ProcessOutcome::Completed;
    match expected {
        OffLinux::Unconsumed => assert!(unconsumed, "{tag}: {result:?}"),
        OffLinux::EitherWay => assert!(unconsumed || completed, "{tag}: {result:?}"),
    }
    assert_eq!(record.exit_code, Some(0), "{tag}: {record:?}");
}

/// On Linux, a provider whose request pipe is still held by a descendant when
/// the host would decide is refused, at the deadline, as held: the host
/// decides only as the pipe's last holder.
fn held(tag: &str, id_bytes: usize, body: &str, off_linux: OffLinux) {
    let case = Case::new(tag, id_bytes);
    let (result, record) = case.run_within(body, HELD_DEADLINE_MS);
    if !cfg!(target_os = "linux") {
        decided_as_before_off_linux(tag, off_linux, &case, &result, &record);
        return;
    }
    assert!(
        matches!(result, Err(ProviderFailure::RequestHeldByDescendant(_))),
        "{tag}: {result:?}"
    );
    assert_eq!(
        record.outcome,
        ProcessOutcome::RequestHeldByDescendant,
        "{tag}"
    );
    assert_eq!(record.exit_code, Some(0), "{tag}: {record:?}");
    assert!(
        record.detail.starts_with("request-held-by-descendant: "),
        "{record:?}"
    );
    // Refused by the deadline, not before it: the host waited for the holder.
    assert!(record.wall_time_ms >= HELD_DEADLINE_MS, "{record:?}");
}

/// On Linux, a provider whose descendant reads the whole request, wherever its
/// reads fall, is accepted: the host waits until the descendant is gone.
fn read_by_descendant(tag: &str, id_bytes: usize, body: &str) {
    let case = Case::new(tag, id_bytes);
    let (result, record) = case.run(body);
    if !cfg!(target_os = "linux") {
        decided_as_before_off_linux(tag, OffLinux::EitherWay, &case, &result, &record);
        return;
    }
    assert_eq!(result, Ok(case.response.clone()), "{tag}: {record:?}");
    assert_eq!(record.outcome, ProcessOutcome::Completed, "{tag}");
}

#[test]
fn a_descendant_holding_an_unread_request_is_refused() {
    held(
        "held-silent",
        64,
        &lend_request("exec /bin/sleep 30", ANSWER),
        OffLinux::Unconsumed,
    );
}

// `setsid -f` is util-linux's. What this case adds over `held-silent`, a
// holder outside the process group, matters only to the Linux holder check.
#[cfg(target_os = "linux")]
#[test]
fn a_detached_descendant_holding_an_unread_request_is_refused() {
    held(
        "held-detached",
        64,
        &lend_request_detached(ANSWER),
        OffLinux::Unconsumed,
    );
}

#[test]
fn a_descendant_holding_an_undelivered_large_request_is_refused() {
    held(
        "held-large",
        LARGE,
        &lend_request("exec /bin/sleep 30", ANSWER),
        OffLinux::Unconsumed,
    );
}

#[test]
fn a_descendant_that_reads_the_request_and_keeps_holding_it_is_refused() {
    held(
        "held-after-reading",
        64,
        &lend_request("/bin/cat > /dev/null; exec /bin/sleep 30", ANSWER),
        OffLinux::EitherWay,
    );
}

#[test]
fn a_descendant_that_holds_an_unread_request_briefly_is_refused() {
    refused(
        "held-briefly",
        64,
        &lend_request("exec /bin/sleep 0.05", ANSWER),
    );
}

#[test]
fn a_descendant_that_reads_the_whole_request_after_the_exit_is_accepted() {
    read_by_descendant(
        "descendant-reads",
        64,
        &lend_request("exec /bin/cat > /dev/null", ANSWER),
    );
}

#[test]
fn a_descendant_that_reads_the_whole_request_late_is_accepted() {
    read_by_descendant(
        "descendant-reads-late",
        64,
        &lend_request("/bin/sleep 0.05; exec /bin/cat > /dev/null", ANSWER),
    );
}

#[test]
fn a_descendant_that_reads_a_whole_large_request_is_accepted() {
    read_by_descendant(
        "descendant-reads-large",
        LARGE,
        &lend_request("exec /bin/cat > /dev/null", ANSWER),
    );
}

#[test]
fn a_background_descendant_with_stdin_from_dev_null_is_accepted() {
    accepted("background-dev-null", 64, |_| {
        format!(
            "/bin/cat > /dev/null\n\
             (exec /bin/sleep 30) </dev/null >/dev/null 2>/dev/null &\n\
             {ANSWER}"
        )
    });
}
