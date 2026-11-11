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

//! `quirin-audit example [--claim safe|unsafe|unknown]`: the worked example of
//! a subcommand plugin (docs/development/audit-subcommand-plugins.md).
//!
//! It writes `example.txt` into its output directory (its working
//! directory), records it as an artifact, and with `--claim` proposes a
//! verdict about it. The host classifies that claim: without a certificate
//! the audit checks, a proposed `Safe` or `Unsafe` is `Pending`.

use std::collections::BTreeMap;
use std::path::Path;
use std::process::ExitCode;

use quirin_audit_plugin::{ProposedVerdict, Request, Response, Severity, VerdictClaim, run};

fn answer(request: &Request) -> Result<Response, String> {
    let mut claim = None;
    let mut arguments = request.arguments.iter();
    while let Some(argument) = arguments.next() {
        match argument.as_str() {
            "--claim" => {
                claim = Some(match arguments.next().map(String::as_str) {
                    Some("safe") => ProposedVerdict::Safe,
                    Some("unsafe") => ProposedVerdict::Unsafe,
                    Some("unknown") => ProposedVerdict::Unknown,
                    other => {
                        return Err(format!("--claim takes safe|unsafe|unknown, not {other:?}"));
                    }
                });
            }
            other => return Err(format!("unknown argument `{other}`")),
        }
    }
    let output = request.output_directory();
    let text = format!(
        "subcommand {} with {} argument(s)\n",
        request.subcommand,
        request.arguments.len()
    );
    std::fs::write(output.join("example.txt"), text).map_err(|error| error.to_string())?;
    let mut response = Response::completed();
    response.push_artifact(&output, Path::new("example.txt"), "example")?;
    response.push_diagnostic(Severity::Info, "example", "wrote example.txt");
    if let Some(verdict) = claim {
        response.push_claim(VerdictClaim {
            artifact: "example.txt".into(),
            property: "example-property".into(),
            semantic_profile: "quirin.audit-plugin-example.v1".into(),
            scope: "the one file".into(),
            bounds: "none".into(),
            assumptions: BTreeMap::new(),
            evidence_version: 1,
            verdict,
            reason: "the example plugin proposes what it was told to".into(),
            certificates: Vec::new(),
        });
    }
    Ok(response)
}

fn main() -> ExitCode {
    run(answer)
}
