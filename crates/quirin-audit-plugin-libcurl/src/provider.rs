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

//! The C11.1 checker package: `quirin-libcurl-checker`, a process provider
//! bound to the plugin's user contract crate (`quirin-provider-libcurl`).
//!
//! It is the provider transport of `quirin_provider_shared::runtime` (one
//! protocol-v1 frame carrying an evidence-v2 request in, one response frame
//! out, the installed manifest bound to the template compiled into the
//! executable), written again here because the root workspace cannot link
//! the providers workspace (follow-up: move it into an SDK both can use).
//! For each request it answers the bound function with the plugin's own
//! checker over the pinned files installed in the package's `source/`
//! directory (its declared filesystem input), verified against the pin
//! first: a tree that does not match is `Unknown` for every function.
//!
//! A `Safe` answer carries no evidence the host can replay, so the host's
//! firewall publishes it as `Pending` (`QP008`, `plugin-claim-uncertified`).
//! The checker's reasons travel in the response's diagnostics.

use std::io::{Read, Write};

use qcore_schemas::obligation::ObligationSubject;
use qcore_schemas::plugin::{
    PLUGIN_PROTOCOL_VERSION, PluginDiagnostic, PluginManifest, ProposedVerdict,
    VerificationRequestV2, VerificationResponseV2,
};

use crate::checker::{self, Answer};

/// The package's filesystem input: the pinned files, at their release paths.
pub const SOURCE_INPUT: &str = "source";

/// The manifest template the checker executable embeds (without its
/// envelope; the executable adds the markers).
pub const TEMPLATE: &str = include_str!("../checker-template.json");

/// The JSON object inside the envelope: from the first `{` to the last `}`.
/// The markers are deliberately not spelled here, so the executable carries
/// them exactly once.
fn embedded(template: &str) -> Result<&str, String> {
    let start = template
        .find('{')
        .ok_or("the embedded template carries no JSON object")?;
    let end = template
        .rfind('}')
        .ok_or("the embedded template carries no JSON object")?;
    Ok(&template[start..=end])
}

/// The installed manifest, which must equal the embedded template with this
/// executable's digest.
fn installed(template: &str) -> Result<PluginManifest, String> {
    let mut expected: PluginManifest =
        serde_json::from_str(embedded(template)?).map_err(|error| error.to_string())?;
    expected.validate().map_err(|error| error.to_string())?;
    let executable = std::env::current_exe().map_err(|error| error.to_string())?;
    expected.package_digest =
        qcore_schemas::sha256_hex(&std::fs::read(executable).map_err(|error| error.to_string())?);
    let mut bytes = Vec::new();
    std::fs::File::open("verifier-plugin.json")
        .map_err(|error| error.to_string())?
        .take(64 * 1024 + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| error.to_string())?;
    if bytes.len() > 64 * 1024 {
        return Err("package manifest exceeds 64 KiB".into());
    }
    let installed: PluginManifest =
        serde_json::from_slice(&bytes).map_err(|error| error.to_string())?;
    installed.validate().map_err(|error| error.to_string())?;
    if installed != expected {
        return Err(
            "the installed manifest differs from the executable's identity, scope, versions or digest"
                .into(),
        );
    }
    Ok(installed)
}

fn read_frame(input: &mut impl Read, limit: usize) -> Result<Vec<u8>, String> {
    let mut header = [0; 4];
    input
        .read_exact(&mut header)
        .map_err(|error| format!("frame header: {error}"))?;
    let size = u32::from_be_bytes(header) as usize;
    if size == 0 || size > limit {
        return Err("frame exceeds the supported length".into());
    }
    let mut payload = vec![0; size];
    input
        .read_exact(&mut payload)
        .map_err(|error| format!("frame payload: {error}"))?;
    let mut extra = [0];
    if input.read(&mut extra).map_err(|error| error.to_string())? != 0 {
        return Err("trailing bytes or multiple request frames".into());
    }
    Ok(payload)
}

fn frame(response: &VerificationResponseV2, limit: usize) -> Result<Vec<u8>, String> {
    let bytes = serde_json::to_vec(response).map_err(|error| error.to_string())?;
    if bytes.len() > limit || u32::try_from(bytes.len()).is_err() {
        return Err("response exceeds the supported length".into());
    }
    let mut frame = u32::try_from(bytes.len())
        .expect("checked above")
        .to_be_bytes()
        .to_vec();
    frame.extend(bytes);
    Ok(frame)
}

/// The request's identity checks: versions, plugin, a concrete supported
/// target.
fn validate(request: &VerificationRequestV2, manifest: &PluginManifest) -> Result<(), String> {
    request.validate().map_err(|error| error.to_string())?;
    let base = request.base();
    if base.protocol_version != PLUGIN_PROTOCOL_VERSION
        || !manifest.evidence_versions.contains(&base.evidence_version)
        || base.plugin_id != manifest.plugin_id
    {
        return Err("unsupported request identity or version".into());
    }
    let target = &base.target;
    if [
        &target.library,
        &target.revision,
        &target.profile,
        &target.dialect,
        &target.platform,
        &target.backend,
    ]
    .iter()
    .any(|value| value.is_empty() || value.as_str() == "*")
        || !manifest.supports(base.capability, target)
    {
        return Err("unsupported or nonconcrete target profile or capability".into());
    }
    Ok(())
}

/// The function symbols a request's obligations name.
#[must_use]
pub fn requested_functions(request: &VerificationRequestV2) -> Vec<String> {
    request
        .obligations
        .iter()
        .filter_map(|obligation| match &obligation.subject {
            ObligationSubject::Function { symbol, .. } => Some(symbol.clone()),
            _ => None,
        })
        .collect()
}

/// The checker's answer to a request, over the pinned files under
/// `source` (relative to the working directory, the package root).
#[must_use]
pub fn propose(
    request: &VerificationRequestV2,
    manifest: &PluginManifest,
    source: &std::path::Path,
) -> VerificationResponseV2 {
    let functions = requested_functions(request);
    let mut diagnostics = Vec::new();
    let mut note = |code: &str, message: String| {
        diagnostics.push(PluginDiagnostic {
            code: format!("{}.{code}", manifest.plugin_id),
            message,
            fact_id: None,
        });
    };
    let verdict = match checker::Tree::open(source) {
        Err(reason) => {
            note("pin-mismatch", reason);
            ProposedVerdict::Unknown
        }
        Ok(_) if functions.is_empty() => {
            note("no-function", "the request names no function".into());
            ProposedVerdict::Unknown
        }
        Ok(tree) => {
            let mut all_safe = true;
            for function in &functions {
                if checker::is_typecheck_stub(function) {
                    all_safe = false;
                    note(
                        "typecheck-stub",
                        format!(
                            "`{function}` is a typecheck-gcc.h warning stub, which this plugin \
                             never binds (W48-LIBCURL-TYPECHECK-STUBS)"
                        ),
                    );
                    continue;
                }
                let Some(binding) = checker::binding(function) else {
                    all_safe = false;
                    note(
                        "unbound",
                        format!("`{function}` is not a libcurl function this plugin checks"),
                    );
                    continue;
                };
                let outcome = checker::answer(&tree, binding);
                if outcome.answer != Answer::Safe {
                    all_safe = false;
                }
                for reason in outcome.reasons {
                    note(
                        if outcome.answer == Answer::Safe {
                            "established"
                        } else {
                            "not-established"
                        },
                        format!("`{function}`: {reason}"),
                    );
                }
            }
            if all_safe {
                ProposedVerdict::Safe
            } else {
                ProposedVerdict::Unknown
            }
        }
    };
    VerificationResponseV2 {
        protocol_version: request.protocol_version,
        request_id: request.request_id.clone(),
        plugin_id: manifest.plugin_id.clone(),
        verdict,
        diagnostics,
        evidence: None,
    }
}

/// Serve one request: the checker executable's whole run.
///
/// # Errors
///
/// A damaged template, an installed manifest that differs from it, or a
/// malformed or refused request frame.
pub fn run(template: &str) -> Result<(), String> {
    let manifest = installed(template)?;
    let arguments: Vec<_> = std::env::args().skip(1).collect();
    if arguments == ["--verify-package"] {
        println!(
            "{}",
            serde_json::to_string(&manifest).map_err(|error| error.to_string())?
        );
        return Ok(());
    }
    if !arguments.is_empty() {
        return Err("expected no arguments or --verify-package".into());
    }
    let limit = usize::try_from(manifest.limits.output_bytes).unwrap_or(usize::MAX);
    let payload = read_frame(&mut std::io::stdin().lock(), limit)?;
    let selector: serde_json::Value =
        serde_json::from_slice(&payload).map_err(|error| error.to_string())?;
    if selector
        .get("evidence_version")
        .and_then(serde_json::Value::as_u64)
        != Some(2)
    {
        return Err("unsupported evidence version".into());
    }
    let request: VerificationRequestV2 =
        serde_json::from_slice(&payload).map_err(|error| error.to_string())?;
    validate(&request, &manifest)?;
    let response = propose(&request, &manifest, std::path::Path::new(SOURCE_INPUT));
    std::io::stdout()
        .lock()
        .write_all(&frame(&response, limit)?)
        .map_err(|error| error.to_string())
}
