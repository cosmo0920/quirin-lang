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

//! Bounded provider transport with optional capability-specific v2 proposals. Linked by each artifact
//! crate from `quirin-provider-shared`, so no host/compiler dependency enters its dependency graph.
use qcore_schemas::plugin::*;
use qcore_schemas::sha256_hex;
use serde::Serialize;
use std::io::{Read, Write};

/// Strip the marker envelope `main` wraps the compiled-in template with.
///
/// The markers are located by the JSON object they surround rather than by
/// their own spelling, deliberately: a host binds an installed manifest to this
/// template by finding **exactly one** marker in the executable's bytes
/// (`qcore_verifier_host::embedded_template`), so this file must not place a
/// second copy of a marker in the binary. The markers carry no brace, and the
/// template is one JSON object, so the outermost braces delimit it exactly.
fn embedded(template: &str) -> Result<&str, String> {
    let start = template
        .find('{')
        .ok_or("embedded manifest template envelope carries no JSON object")?;
    let end = template
        .rfind('}')
        .ok_or("embedded manifest template envelope carries no JSON object")?;
    if end < start {
        return Err("embedded manifest template envelope is damaged".into());
    }
    Ok(&template[start..=end])
}

fn contract_manifest(contracts: &str) -> Result<&str, String> {
    let manifest = embedded(contracts)?;
    let _: serde_json::Value = serde_json::from_str(manifest).map_err(|e| e.to_string())?;
    Ok(manifest)
}

fn package(template: &str) -> Result<PluginManifest, String> {
    let mut expected: PluginManifest =
        serde_json::from_str(embedded(template)?).map_err(|e| e.to_string())?;
    expected.validate().map_err(|e| e.to_string())?;
    let executable = std::env::current_exe().map_err(|e| e.to_string())?;
    expected.package_digest = sha256_hex(&std::fs::read(executable).map_err(|e| e.to_string())?);
    let mut bytes = Vec::new();
    std::fs::File::open("verifier-plugin.json")
        .map_err(|e| e.to_string())?
        .take(64 * 1024 + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() > 64 * 1024 {
        return Err("package manifest exceeds 64 KiB".into());
    }
    let installed: PluginManifest = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
    installed.validate().map_err(|e| e.to_string())?;
    if installed != expected {
        return Err("installed manifest differs from the executable's package identity, scope, versions or digest".into());
    }
    Ok(installed)
}

fn read_frame(input: &mut impl Read, limit: usize) -> Result<Vec<u8>, String> {
    let mut header = [0; 4];
    input
        .read_exact(&mut header)
        .map_err(|e| format!("frame header: {e}"))?;
    let size = u32::from_be_bytes(header) as usize;
    if size == 0 || size > limit {
        return Err("frame exceeds the supported length".into());
    }
    let mut payload = vec![0; size];
    input
        .read_exact(&mut payload)
        .map_err(|e| format!("frame payload: {e}"))?;
    let mut extra = [0];
    if input.read(&mut extra).map_err(|e| e.to_string())? != 0 {
        return Err("trailing bytes or multiple request frames".into());
    }
    Ok(payload)
}

fn frame(response: &impl Serialize, limit: usize) -> Result<Vec<u8>, String> {
    let bytes = serde_json::to_vec(response).map_err(|e| e.to_string())?;
    if bytes.len() > limit || bytes.len() > u32::MAX as usize {
        return Err("response exceeds the supported length".into());
    }
    let mut frame = (bytes.len() as u32).to_be_bytes().to_vec();
    frame.extend(bytes);
    Ok(frame)
}

fn validate(request: &VerificationRequest, manifest: &PluginManifest) -> Result<(), String> {
    request.validate().map_err(|e| e.to_string())?;
    if request.protocol_version != PLUGIN_PROTOCOL_VERSION
        || !manifest
            .evidence_versions
            .contains(&request.evidence_version)
        || request.plugin_id != manifest.plugin_id
    {
        return Err("unsupported request identity or version".into());
    }
    let target = &request.target;
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
        || !manifest.supports(request.capability, target)
    {
        return Err("unsupported or nonconcrete target profile/capability".into());
    }
    Ok(())
}

type V2Proposal = fn(&VerificationRequestV2, &PluginManifest) -> Option<VerificationResponseV2>;

fn response(
    payload: &[u8],
    manifest: &PluginManifest,
    propose: Option<V2Proposal>,
) -> Result<Vec<u8>, String> {
    // Inspect the evidence selector, then deserialize the original bytes into
    // the strict versioned schema (including duplicate/unknown field checks).
    let selector: serde_json::Value = serde_json::from_slice(payload).map_err(|e| e.to_string())?;
    match selector
        .get("evidence_version")
        .and_then(|value| value.as_u64())
    {
        Some(1) => {
            let request: VerificationRequest =
                serde_json::from_slice(payload).map_err(|e| e.to_string())?;
            validate(&request, manifest)?;
            frame(
                &VerificationResponse {
                    protocol_version: 1,
                    request_id: request.request_id,
                    plugin_id: manifest.plugin_id.clone(),
                    verdict: ProposedVerdict::Unknown,
                    diagnostics: vec![PluginDiagnostic {
                        code: format!("{}.semantic_evidence_unavailable", manifest.plugin_id),
                        message:
                            "protocol v1 facts lack core-derived obligations and semantic witnesses"
                                .into(),
                        fact_id: None,
                    }],
                    evidence: None,
                },
                manifest.limits.output_bytes,
            )
        }
        Some(2) => {
            let request: VerificationRequestV2 =
                serde_json::from_slice(payload).map_err(|e| e.to_string())?;
            request.validate().map_err(|e| e.to_string())?;
            validate(&request.base(), manifest)?;
            if let Some(response) = propose.and_then(|propose| propose(&request, manifest)) {
                return frame(&response, manifest.limits.output_bytes);
            }
            frame(&VerificationResponseV2 {
                protocol_version: 1,
                request_id: request.request_id,
                plugin_id: manifest.plugin_id.clone(),
                verdict: ProposedVerdict::Unknown,
                diagnostics: vec![PluginDiagnostic {
                    code: format!("{}.semantic_evidence_unavailable", manifest.plugin_id),
                    message: "this executable package does not implement semantic witnesses for the requested capability".into(),
                    fact_id: None,
                }],
                evidence: None,
            }, manifest.limits.output_bytes)
        }
        _ => Err("unsupported evidence version".into()),
    }
}

pub fn run(template: &str) -> Result<(), String> {
    run_with_v2(template, None)
}

pub fn run_with_v2(template: &str, propose: Option<V2Proposal>) -> Result<(), String> {
    run_with_v2_and_contracts(template, None, propose)
}

pub fn run_with_contracts(template: &str, contracts: &str) -> Result<(), String> {
    run_with_v2_and_contracts(template, Some(contracts), None)
}

pub fn run_with_v2_and_contracts(
    template: &str,
    contracts: Option<&str>,
    propose: Option<V2Proposal>,
) -> Result<(), String> {
    let manifest = package(template)?;
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args == ["--verify-package"] {
        println!(
            "{}",
            serde_json::to_string(&manifest).map_err(|e| e.to_string())?
        );
        return Ok(());
    }
    if args == ["--contract-manifest"] {
        let contracts = contracts.ok_or("provider has no embedded contract inventory")?;
        println!("{}", contract_manifest(contracts)?);
        return Ok(());
    }
    if !args.is_empty() {
        return Err("expected no arguments, --verify-package, or --contract-manifest".into());
    }
    let payload = read_frame(&mut std::io::stdin().lock(), manifest.limits.output_bytes)?;
    let response = response(&payload, &manifest, propose)?;
    std::io::stdout()
        .lock()
        .write_all(&response)
        .map_err(|e| e.to_string())
}
