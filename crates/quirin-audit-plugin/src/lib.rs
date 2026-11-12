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

//! The SDK for **external `quirin-audit` subcommand plugins** (card
//! W45-AUDIT-SUBCOMMAND-PLUGINS; contract specification §C11.2).
//!
//! `quirin-audit NAME ARGS…`, for a `NAME` that is no built-in subcommand,
//! runs the executable `quirin-audit-NAME` found on a **declared** search path
//! (`--plugin-path DIR`, then [`PLUGIN_PATH_VARIABLE`]; never `PATH`), out of
//! process, cargo-style. The host writes one [`Request`] as JSON to the
//! plugin's standard input and reads one [`Response`] as JSON from its standard
//! output. This crate is the plugin's side of that protocol:
//!
//! * [`Request`] and [`Response`], the versioned wire types
//!   ([`PROTOCOL`] version [`PROTOCOL_VERSION`]), and the host-side
//!   [`parse_response`], which tells a version mismatch from a malformed
//!   answer;
//! * [`run`], the entry point a plugin's `main` calls: read the request,
//!   call the handler, write the response;
//! * the response builders ([`Response::push_artifact`] computes each
//!   artifact's SHA-256) and [`verify_artifacts`], the check the host applies;
//! * [`testing`], a harness that runs a plugin binary as the host would;
//! * [`json_text`], ordered, indented JSON text with ASCII escapes, for
//!   records compared byte for byte.
//!
//! What stays in the host: dispatch, the search path, the cleared
//! environment, quotas, confinement of the working directory, and the
//! **firewall**. A plugin may *claim* a verdict ([`VerdictClaim`]), but a claim
//! decides nothing by itself: the host classifies it with the same firewall
//! that classifies checker plugins, so an uncertified `Safe` or `Unsafe` is
//! `Pending` (`plugin-claim-uncertified`) and every failure, timeout, quota
//! breach, malformed answer or version mismatch is `Unknown`. Nothing this
//! crate does can change that. It links no core analysis crate; the schema
//! crate supplies SHA-256 only.

use std::collections::{BTreeMap, BTreeSet};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use serde::{Deserialize, Serialize};
use serde_json::Value;

pub mod json_text;
pub mod testing;

/// The protocol's name, carried in every request and response.
pub const PROTOCOL: &str = "quirin-audit-plugin";

/// The only protocol version this SDK speaks. A response of any other version
/// is `Unknown` with the named reason `protocol-version-mismatch`.
pub const PROTOCOL_VERSION: u32 = 1;

/// The executable name prefix: `quirin-audit NAME` runs `quirin-audit-NAME`.
pub const EXECUTABLE_PREFIX: &str = "quirin-audit-";

/// The environment variable holding further plugin search directories
/// (`std::env::split_paths` syntax), searched after every `--plugin-path`.
pub const PLUGIN_PATH_VARIABLE: &str = "QUIRIN_AUDIT_PLUGIN_PATH";

/// The only variables of the host's environment a plugin receives, when set.
/// Everything else is cleared; `--plugin-env NAME` passes one more by name,
/// and the invocation record lists every name passed. `PATH` is passed so a
/// plugin can run the tools it names (`git`, `cmake`, `clang`); it is never
/// used to find the plugin itself.
pub const ENVIRONMENT_ALLOW_LIST: [&str; 7] =
    ["PATH", "HOME", "TMPDIR", "LANG", "LC_ALL", "LC_CTYPE", "TZ"];

/// The slot-wait marker (card MEASURE-QUOTA-AND-INCREMENTAL, requirement R7
/// of `docs/plans/gate-time-reduction/README.md`): while a file of this name
/// exists in the plugin's output directory (its working directory), the
/// plugin declares that all of its work is waiting for a gate slot
/// (`scripts/gate_slot.py`). The host does not count that time against the
/// wall-time quota, up to its slot-wait limit (`--slot-wait-limit-ms`,
/// default the overall wall bound, so a long queue is not capped at the
/// analysis quota), and records it apart from the analysis time. The
/// declaration is the plugin's, so two limits bound what it can buy: the
/// slot-wait limit, and the overall wall bound ([`DEFAULT_WALL_LIMIT_MS`],
/// `--wall-limit-ms`), after which the run is stopped whatever it declares.
/// The host removes a stale marker before it spawns the plugin.
pub const SLOT_WAIT_MARKER: &str = ".quirin-audit-slot-wait";

/// Default overall wall bound: seven days. The wall-time quota counts analysis
/// time only (gate-slot waits are excluded), so this separate bound on the
/// run's whole elapsed time, analysis and declared waits together, is what
/// stops a plugin that declares waits for ever (card GATE-SLOT-FIFO-ORDER,
/// requirement R7 of `docs/plans/gate-time-reduction/README.md`). It is raised
/// to the quota when the quota is larger.
pub const DEFAULT_WALL_LIMIT_MS: u64 = 7 * 24 * 60 * 60 * 1000;

/// Default wall-time quota: thirty minutes.
pub const DEFAULT_WALL_TIME_MS: u64 = 30 * 60 * 1000;

/// Default quota on the response (standard output), in bytes.
pub const DEFAULT_STDOUT_BYTES: u64 = 16 << 20;

/// Default quota on the captured standard error, in bytes.
pub const DEFAULT_STDERR_BYTES: u64 = 16 << 20;

/// A file the host names, with the SHA-256 of its bytes when the request was
/// made.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FileIdentity {
    pub path: String,
    pub sha256: String,
}

/// The audit a plugin runs beside, when the host was given one (from the
/// `.qo` checkpoints it verified); every field is `None` otherwise.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AuditIdentity {
    /// The audited source revision.
    pub revision: Option<String>,
    /// SHA-256 of the checkpoints' contract-set identity (canonical JSON).
    pub contract_set_sha256: Option<String>,
    /// SHA-256 of the refutation table the checkpoints were analysed under
    /// (contract specification §C06.4).
    pub refuted_table_sha256: Option<String>,
}

/// The quotas the host enforces. A breach kills the plugin's process group
/// and makes the run `Unknown`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Quotas {
    /// The analysis-time quota: declared slot waits are not counted.
    pub wall_time_ms: u64,
    pub stdout_bytes: u64,
    pub stderr_bytes: u64,
    /// The most declared slot-wait time the wall-time quota excludes. Absent
    /// from a request of a host that predates it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub slot_wait_limit_ms: Option<u64>,
    /// The overall wall bound on the run, analysis and declared waits
    /// together. Absent from a request of a host that predates it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wall_limit_ms: Option<u64>,
}

impl Default for Quotas {
    fn default() -> Self {
        Self {
            wall_time_ms: DEFAULT_WALL_TIME_MS,
            stdout_bytes: DEFAULT_STDOUT_BYTES,
            stderr_bytes: DEFAULT_STDERR_BYTES,
            slot_wait_limit_ms: None,
            wall_limit_ms: None,
        }
    }
}

/// What the host sends on the plugin's standard input.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Request {
    pub protocol: String,
    pub protocol_version: u32,
    /// The subcommand name (`libevent` for `quirin-audit-libevent`).
    pub subcommand: String,
    /// The arguments after the name, verbatim; the plugin's `argv[1..]` too.
    pub arguments: Vec<String>,
    /// The host's working directory. The plugin's own working directory is
    /// [`Self::output`], so a relative path argument is resolved against this.
    pub invocation_directory: String,
    /// The declared output directory (canonical): the plugin's working
    /// directory, and the only place its artifacts may lie.
    pub output: String,
    /// The report the host writes after the plugin exits.
    pub report: String,
    /// The contract load path, when the host was given one.
    pub load_path: Option<FileIdentity>,
    /// The selected provider crates, each on the load path.
    pub providers: Vec<String>,
    pub identity: AuditIdentity,
    /// An existing audit report whose verdicts are the host's.
    pub audit_report: Option<FileIdentity>,
    /// Verified `.qo` checkpoints, each with its content digest.
    pub checkpoints: Vec<FileIdentity>,
    pub quotas: Quotas,
}

impl Request {
    /// A request of this protocol version with nothing but the name,
    /// arguments and directories set (the harness's and tests' starting
    /// point; the host fills every field).
    #[must_use]
    pub fn new(subcommand: &str, arguments: Vec<String>, invocation: &Path, output: &Path) -> Self {
        Self {
            protocol: PROTOCOL.into(),
            protocol_version: PROTOCOL_VERSION,
            subcommand: subcommand.into(),
            arguments,
            invocation_directory: invocation.display().to_string(),
            output: output.display().to_string(),
            report: output
                .join(format!("{EXECUTABLE_PREFIX}{subcommand}.report.json"))
                .display()
                .to_string(),
            load_path: None,
            providers: Vec::new(),
            identity: AuditIdentity::default(),
            audit_report: None,
            checkpoints: Vec::new(),
            quotas: Quotas::default(),
        }
    }

    /// Parse a request, checking the protocol and its version before the
    /// shape.
    ///
    /// # Errors
    ///
    /// Another protocol or version, or a malformed request.
    pub fn from_slice(bytes: &[u8]) -> Result<Self, ProtocolError> {
        let value = versioned(bytes)?;
        serde_json::from_value(value).map_err(|error| ProtocolError::Malformed(error.to_string()))
    }

    /// The output directory.
    #[must_use]
    pub fn output_directory(&self) -> PathBuf {
        PathBuf::from(&self.output)
    }

    /// `path` resolved as the user meant it: absolute as given, relative
    /// against the host's working directory (not the plugin's).
    #[must_use]
    pub fn resolve_argument(&self, path: &str) -> PathBuf {
        Path::new(&self.invocation_directory).join(path)
    }
}

/// The plugin's overall result.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum Outcome {
    /// The plugin did its work; its artifacts and claims follow.
    Completed,
    /// The plugin refused or failed, with its reason. The run is `Unknown`.
    Failed(String),
}

/// One file the plugin produced, inside the output directory.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Artifact {
    /// Relative to the output directory, or absolute inside it.
    pub path: String,
    pub sha256: String,
    /// What the file is (`compile-database`, `audit-configuration`, …).
    pub kind: String,
}

/// A diagnostic's severity.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    Info,
    Warning,
    Error,
}

/// A message the plugin reports; it decides no verdict.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Diagnostic {
    pub severity: Severity,
    pub code: String,
    pub message: String,
}

/// The verdict a plugin proposes. There is no `Pending`: that value is the
/// host's classification of an unchecked claim, never a plugin's answer.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProposedVerdict {
    Safe,
    Unsafe,
    Unknown,
}

/// A certificate or witness a claim offers, by reference.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CertificateReference {
    pub kind: String,
    pub path: String,
    pub sha256: String,
}

/// A verdict claim: the full tuple of hard invariant 2 (artifact, property,
/// semantic profile, scope, bounds, assumptions, evidence version), the
/// proposed verdict and its reason, and any certificates offered.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VerdictClaim {
    /// What the claim is about; to join an audit report's row, its `file`.
    pub artifact: String,
    pub property: String,
    pub semantic_profile: String,
    pub scope: String,
    pub bounds: String,
    pub assumptions: BTreeMap<String, String>,
    pub evidence_version: u32,
    pub verdict: ProposedVerdict,
    pub reason: String,
    pub certificates: Vec<CertificateReference>,
}

/// What the plugin writes on its standard output: this and nothing else.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Response {
    pub protocol: String,
    pub protocol_version: u32,
    pub outcome: Outcome,
    pub artifacts: Vec<Artifact>,
    pub diagnostics: Vec<Diagnostic>,
    pub claims: Vec<VerdictClaim>,
}

impl Response {
    /// A completed response with nothing in it yet.
    #[must_use]
    pub fn completed() -> Self {
        Self {
            protocol: PROTOCOL.into(),
            protocol_version: PROTOCOL_VERSION,
            outcome: Outcome::Completed,
            artifacts: Vec::new(),
            diagnostics: Vec::new(),
            claims: Vec::new(),
        }
    }

    /// A failed response.
    #[must_use]
    pub fn failed(reason: impl Into<String>) -> Self {
        Self {
            outcome: Outcome::Failed(reason.into()),
            ..Self::completed()
        }
    }

    /// Record `path` (relative to `output`, or absolute inside it) as an
    /// artifact, with the SHA-256 of its bytes now.
    ///
    /// # Errors
    ///
    /// An unreadable file.
    pub fn push_artifact(&mut self, output: &Path, path: &Path, kind: &str) -> Result<(), String> {
        let absolute = output.join(path);
        let bytes =
            std::fs::read(&absolute).map_err(|error| format!("{}: {error}", absolute.display()))?;
        let spelled = path
            .strip_prefix(output)
            .unwrap_or(path)
            .display()
            .to_string();
        self.artifacts.push(Artifact {
            path: spelled,
            sha256: qcore_schemas::sha256_hex(&bytes),
            kind: kind.into(),
        });
        Ok(())
    }

    /// Record a diagnostic.
    pub fn push_diagnostic(&mut self, severity: Severity, code: &str, message: impl Into<String>) {
        self.diagnostics.push(Diagnostic {
            severity,
            code: code.into(),
            message: message.into(),
        });
    }

    /// Record a verdict claim (the host classifies it; see the crate docs).
    pub fn push_claim(&mut self, claim: VerdictClaim) {
        self.claims.push(claim);
    }

    /// Structural checks beyond the JSON shape: digests are SHA-256 hex, the
    /// claim tuple's names are not empty, artifact paths are unique.
    ///
    /// # Errors
    ///
    /// The first violation.
    pub fn validate(&self) -> Result<(), String> {
        let mut paths = BTreeSet::new();
        for artifact in &self.artifacts {
            is_sha256(&artifact.sha256).then_some(()).ok_or_else(|| {
                format!("artifact `{}`: `sha256` is not SHA-256 hex", artifact.path)
            })?;
            if artifact.path.is_empty() || !paths.insert(artifact.path.as_str()) {
                return Err(format!(
                    "artifact path `{}` is empty or repeated",
                    artifact.path
                ));
            }
        }
        for claim in &self.claims {
            for (field, value) in [
                ("artifact", &claim.artifact),
                ("property", &claim.property),
                ("semantic_profile", &claim.semantic_profile),
                ("scope", &claim.scope),
                ("bounds", &claim.bounds),
            ] {
                if value.trim().is_empty() {
                    return Err(format!("a verdict claim has an empty `{field}`"));
                }
            }
            for certificate in &claim.certificates {
                if !is_sha256(&certificate.sha256) {
                    return Err(format!(
                        "certificate `{}`: `sha256` is not SHA-256 hex",
                        certificate.path
                    ));
                }
            }
        }
        Ok(())
    }
}

fn is_sha256(text: &str) -> bool {
    text.len() == 64
        && text
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

/// Why a request or response was refused.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ProtocolError {
    /// Another protocol, or another version of this one.
    VersionMismatch {
        protocol: Option<String>,
        version: Option<u64>,
    },
    /// Not the shape this version defines.
    Malformed(String),
}

impl ProtocolError {
    /// The named reason code the host records.
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            Self::VersionMismatch { .. } => "protocol-version-mismatch",
            Self::Malformed(_) => "malformed-response",
        }
    }
}

impl std::fmt::Display for ProtocolError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::VersionMismatch { protocol, version } => write!(
                formatter,
                "protocol mismatch: expected `{PROTOCOL}` version {PROTOCOL_VERSION}, found `{}` \
                 version {}",
                protocol.as_deref().unwrap_or("(none)"),
                version.map_or_else(|| "(none)".into(), |version| version.to_string())
            ),
            Self::Malformed(reason) => write!(formatter, "malformed: {reason}"),
        }
    }
}

/// The JSON object, once its protocol and version are this SDK's.
fn versioned(bytes: &[u8]) -> Result<Value, ProtocolError> {
    let value: Value = serde_json::from_slice(bytes)
        .map_err(|error| ProtocolError::Malformed(error.to_string()))?;
    let Some(object) = value.as_object() else {
        return Err(ProtocolError::Malformed("not a JSON object".into()));
    };
    let protocol = object
        .get("protocol")
        .and_then(Value::as_str)
        .map(str::to_owned);
    let version = object.get("protocol_version").and_then(Value::as_u64);
    if protocol.as_deref() != Some(PROTOCOL) || version != Some(u64::from(PROTOCOL_VERSION)) {
        return Err(ProtocolError::VersionMismatch { protocol, version });
    }
    Ok(value)
}

/// The host's parse of a plugin's standard output: the protocol and version
/// first (a mismatch is named), then the exact shape, then [`Response::validate`].
///
/// # Errors
///
/// A version mismatch or a malformed response.
pub fn parse_response(bytes: &[u8]) -> Result<Response, ProtocolError> {
    let value = versioned(bytes)?;
    let response: Response = serde_json::from_value(value)
        .map_err(|error| ProtocolError::Malformed(error.to_string()))?;
    response.validate().map_err(ProtocolError::Malformed)?;
    Ok(response)
}

/// An artifact the host checked.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct VerifiedArtifact {
    /// The canonical path.
    pub path: String,
    /// The path relative to the output directory.
    pub relative: String,
    pub sha256: String,
    pub bytes: u64,
    pub kind: String,
}

/// Check every artifact: it exists as a regular file whose canonical path lies
/// inside `output` (canonical) and is none of `reserved`, and its bytes have
/// the declared SHA-256.
///
/// # Errors
///
/// The first artifact that fails.
pub fn verify_artifacts(
    response: &Response,
    output: &Path,
    reserved: &[PathBuf],
) -> Result<Vec<VerifiedArtifact>, String> {
    let mut verified = Vec::new();
    for artifact in &response.artifacts {
        let spelled = output.join(&artifact.path);
        let canonical = spelled
            .canonicalize()
            .map_err(|error| format!("artifact `{}`: {error}", artifact.path))?;
        let Ok(relative) = canonical.strip_prefix(output) else {
            return Err(format!(
                "artifact `{}` resolves to {}, outside the declared output {}",
                artifact.path,
                canonical.display(),
                output.display()
            ));
        };
        if relative.as_os_str().is_empty() || !canonical.is_file() {
            return Err(format!(
                "artifact `{}` is not a regular file",
                artifact.path
            ));
        }
        if reserved.contains(&canonical) {
            return Err(format!(
                "artifact `{}` is a file the host writes",
                artifact.path
            ));
        }
        let bytes = std::fs::read(&canonical)
            .map_err(|error| format!("artifact `{}`: {error}", artifact.path))?;
        let digest = qcore_schemas::sha256_hex(&bytes);
        if digest != artifact.sha256 {
            return Err(format!(
                "artifact `{}` has SHA-256 {digest}, not the declared {}",
                artifact.path, artifact.sha256
            ));
        }
        verified.push(VerifiedArtifact {
            path: canonical.display().to_string(),
            relative: relative.display().to_string(),
            sha256: digest,
            bytes: bytes.len() as u64,
            kind: artifact.kind.clone(),
        });
    }
    Ok(verified)
}

/// A plugin's `main`: read the request from standard input, answer it with
/// `handler`, write the response to standard output. Exits 0 for a completed
/// response and 1 for a failed one (a refused request, a handler error, or a
/// handler's own [`Response::failed`]).
///
/// Write nothing else to standard output: the host reads it as the response.
/// Diagnostics for a person go to standard error, which the host captures to
/// a file.
pub fn run<F>(handler: F) -> ExitCode
where
    F: FnOnce(&Request) -> Result<Response, String>,
{
    let mut input = Vec::new();
    let response = match std::io::stdin().read_to_end(&mut input) {
        Err(error) => Response::failed(format!("could not read the request: {error}")),
        Ok(_) => match Request::from_slice(&input) {
            Err(error) => Response::failed(format!("request refused: {error}")),
            Ok(request) => handler(&request).unwrap_or_else(Response::failed),
        },
    };
    let failed = matches!(response.outcome, Outcome::Failed(_));
    if let Outcome::Failed(reason) = &response.outcome {
        eprintln!("{reason}");
    }
    let mut stdout = std::io::stdout().lock();
    let written = serde_json::to_writer(&mut stdout, &response)
        .map_err(|error| error.to_string())
        .and_then(|()| stdout.write_all(b"\n").map_err(|error| error.to_string()))
        .and_then(|()| stdout.flush().map_err(|error| error.to_string()));
    if let Err(error) = written {
        eprintln!("could not write the response: {error}");
        return ExitCode::FAILURE;
    }
    if failed {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_response_round_trips_and_validates() {
        let mut response = Response::completed();
        response.push_diagnostic(Severity::Info, "note", "hello");
        response.push_claim(VerdictClaim {
            artifact: "a.c".into(),
            property: "resource-custody".into(),
            semantic_profile: "example.v1".into(),
            scope: "whole-unit".into(),
            bounds: "single-path".into(),
            assumptions: BTreeMap::new(),
            evidence_version: 2,
            verdict: ProposedVerdict::Safe,
            reason: "said so".into(),
            certificates: Vec::new(),
        });
        let bytes = serde_json::to_vec(&response).unwrap();
        assert_eq!(parse_response(&bytes).unwrap(), response);
    }

    #[test]
    fn another_version_is_a_named_mismatch_before_the_shape() {
        let bytes = br#"{"protocol": "quirin-audit-plugin", "protocol_version": 2, "x": 1}"#;
        let error = parse_response(bytes).unwrap_err();
        assert_eq!(error.code(), "protocol-version-mismatch");
        let bytes = br#"{"protocol_version": 1}"#;
        assert_eq!(
            parse_response(bytes).unwrap_err().code(),
            "protocol-version-mismatch"
        );
        let bytes = br#"{"protocol": "quirin-audit-plugin", "protocol_version": 1, "outcome": "completed"}"#;
        assert_eq!(
            parse_response(bytes).unwrap_err().code(),
            "malformed-response"
        );
        assert_eq!(
            parse_response(b"not json").unwrap_err().code(),
            "malformed-response"
        );
    }

    #[test]
    fn a_pending_proposal_and_an_unknown_field_are_malformed() {
        let mut value = serde_json::to_value(Response::completed()).unwrap();
        value["claims"] = serde_json::json!([{
            "artifact": "a", "property": "p", "semantic_profile": "s", "scope": "x",
            "bounds": "b", "assumptions": {}, "evidence_version": 1, "verdict": "Pending",
            "reason": "r", "certificates": []
        }]);
        let bytes = serde_json::to_vec(&value).unwrap();
        assert_eq!(
            parse_response(&bytes).unwrap_err().code(),
            "malformed-response"
        );
        let mut value = serde_json::to_value(Response::completed()).unwrap();
        value["extra"] = serde_json::json!(true);
        let bytes = serde_json::to_vec(&value).unwrap();
        assert_eq!(
            parse_response(&bytes).unwrap_err().code(),
            "malformed-response"
        );
    }

    #[test]
    fn a_bad_digest_or_empty_tuple_field_is_malformed() {
        let mut response = Response::completed();
        response.artifacts.push(Artifact {
            path: "x".into(),
            sha256: "abc".into(),
            kind: "k".into(),
        });
        assert!(response.validate().is_err());
        let mut response = Response::completed();
        response.push_claim(VerdictClaim {
            artifact: "a".into(),
            property: " ".into(),
            semantic_profile: "s".into(),
            scope: "x".into(),
            bounds: "b".into(),
            assumptions: BTreeMap::new(),
            evidence_version: 1,
            verdict: ProposedVerdict::Unknown,
            reason: String::new(),
            certificates: Vec::new(),
        });
        assert!(response.validate().is_err());
    }
}
