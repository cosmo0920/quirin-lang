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

//! A test harness for subcommand plugins: run a plugin binary on a request
//! the way the host does — cleared environment (the allow-list only), working
//! directory the output, the request on standard input — and read back its
//! parsed response and verified artifacts.
//!
//! The harness enforces no quota and joins no verdict: it is for a plugin's
//! own tests. What the host does with the answer is the host's
//! (`quirin-audit`, `crates/qcore-audit/src/subcommand_plugins.rs`).

use std::io::Write;
use std::path::Path;
use std::process::{Command, ExitStatus, Stdio};

use crate::{
    ENVIRONMENT_ALLOW_LIST, ProtocolError, Request, Response, VerifiedArtifact, parse_response,
    verify_artifacts,
};

/// One run of a plugin.
#[derive(Debug)]
pub struct Invocation {
    pub status: ExitStatus,
    pub stdout: Vec<u8>,
    pub stderr: String,
    pub response: Result<Response, ProtocolError>,
}

impl Invocation {
    /// The response, which must parse, and its artifacts, which must verify
    /// against the request's output directory.
    ///
    /// # Errors
    ///
    /// A malformed or mismatched response, or an artifact that does not
    /// verify.
    pub fn verified(
        &self,
        request: &Request,
    ) -> Result<(&Response, Vec<VerifiedArtifact>), String> {
        let response = self.response.as_ref().map_err(ToString::to_string)?;
        let output = Path::new(&request.output)
            .canonicalize()
            .map_err(|error| format!("{}: {error}", request.output))?;
        let artifacts = verify_artifacts(response, &output, &[])?;
        Ok((response, artifacts))
    }
}

/// Run `executable` on `request` (its arguments are the request's).
///
/// # Errors
///
/// The plugin could not be started or its pipes failed.
pub fn invoke(executable: &Path, request: &Request) -> std::io::Result<Invocation> {
    let mut command = Command::new(executable);
    command
        .args(&request.arguments)
        .env_clear()
        .current_dir(&request.output)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    for name in ENVIRONMENT_ALLOW_LIST {
        if let Some(value) = std::env::var_os(name) {
            command.env(name, value);
        }
    }
    let mut child = command.spawn()?;
    let bytes = serde_json::to_vec(request).map_err(std::io::Error::other)?;
    let mut stdin = child.stdin.take().expect("piped stdin");
    let writer = std::thread::spawn(move || stdin.write_all(&bytes));
    let output = child.wait_with_output()?;
    // A plugin that exits without reading its request breaks the pipe; that
    // is its answer to judge, not a harness failure.
    let _ = writer.join();
    Ok(Invocation {
        status: output.status,
        response: parse_response(&output.stdout),
        stdout: output.stdout,
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
    })
}
