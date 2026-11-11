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

//! The SDK's harness runs the example plugin as the host would.

use std::path::{Path, PathBuf};

use quirin_audit_plugin::testing::invoke;
use quirin_audit_plugin::{Outcome, PROTOCOL_VERSION, ProposedVerdict, Request};

fn scratch(name: &str) -> PathBuf {
    let directory = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!(
        "quirin-audit-plugin-harness-{}-{name}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&directory);
    std::fs::create_dir_all(&directory).unwrap();
    directory.canonicalize().unwrap()
}

fn example() -> &'static Path {
    Path::new(env!("CARGO_BIN_EXE_quirin-audit-example"))
}

#[test]
fn the_example_writes_its_artifact_and_proposes_a_claim() {
    let output = scratch("claim");
    let request = Request::new(
        "example",
        vec!["--claim".into(), "safe".into()],
        &output,
        &output,
    );
    let invocation = invoke(example(), &request).unwrap();
    assert!(invocation.status.success(), "{}", invocation.stderr);
    let (response, artifacts) = invocation.verified(&request).unwrap();
    assert_eq!(response.protocol_version, PROTOCOL_VERSION);
    assert_eq!(response.outcome, Outcome::Completed);
    assert_eq!(artifacts.len(), 1);
    assert_eq!(artifacts[0].relative, "example.txt");
    assert_eq!(response.claims[0].verdict, ProposedVerdict::Safe);
    std::fs::remove_dir_all(&output).unwrap();
}

#[test]
fn a_bad_argument_is_a_failed_response_and_exit_1() {
    let output = scratch("bad");
    let request = Request::new("example", vec!["--nope".into()], &output, &output);
    let invocation = invoke(example(), &request).unwrap();
    assert_eq!(invocation.status.code(), Some(1));
    let response = invocation.response.as_ref().unwrap();
    assert!(matches!(&response.outcome, Outcome::Failed(reason) if reason.contains("--nope")));
    std::fs::remove_dir_all(&output).unwrap();
}

#[test]
fn a_request_of_another_version_is_refused_by_the_plugin() {
    let output = scratch("version");
    let mut request = Request::new("example", Vec::new(), &output, &output);
    request.protocol_version = PROTOCOL_VERSION + 1;
    let invocation = invoke(example(), &request).unwrap();
    assert_eq!(invocation.status.code(), Some(1));
    let response = invocation.response.as_ref().unwrap();
    assert!(
        matches!(&response.outcome, Outcome::Failed(reason) if reason.contains("protocol mismatch"))
    );
    assert!(!output.join("example.txt").exists());
    std::fs::remove_dir_all(&output).unwrap();
}

#[test]
fn a_tampered_artifact_does_not_verify() {
    let output = scratch("tamper");
    let request = Request::new("example", Vec::new(), &output, &output);
    let invocation = invoke(example(), &request).unwrap();
    std::fs::write(output.join("example.txt"), "changed\n").unwrap();
    let error = invocation.verified(&request).unwrap_err();
    assert!(error.contains("not the declared"), "{error}");
    std::fs::remove_dir_all(&output).unwrap();
}
