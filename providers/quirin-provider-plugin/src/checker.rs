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

//! The runtime a checker plugin is written against.
//!
//! A checker is an ordinary provider executable: it embeds its manifest
//! template in its own bytes (so the host binds the installed manifest to it
//! without running it), reads one protocol-v1 frame carrying an evidence-v2
//! request from standard input, and writes one response frame. This module is
//! the shared provider transport (`quirin_provider_shared::runtime`) with a
//! checker's proposal function plugged in:
//!
//! ```ignore
//! fn main() {
//!     let template = concat!(
//!         "<<<qcore.provider-template.v1:begin>>>\n",
//!         include_str!("../checker-template.json"),
//!         "<<<qcore.provider-template.v1:end>>>"
//!     );
//!     if let Err(error) = quirin_provider_plugin::checker::run(template, propose) {
//!         eprintln!("checker: {error}");
//!         std::process::exit(2);
//!     }
//! }
//! ```
//!
//! What a checker proposes is a **claim**, not a verdict. The audit's firewall
//! counts a `Safe` only with a certificate its own trusted checker replays,
//! and an `Unsafe` only with a witness its own trusted checker validates;
//! anything else a checker says is `Pending` (a claim) or `Unknown` (a
//! failure). There is no way to write a checker that the audit trusts.

use qcore_schemas::plugin::{
    PluginDiagnostic, PluginManifest, ProposedEvidenceV2, ProposedVerdict, VerificationRequestV2,
    VerificationResponseV2,
};

/// A checker's proposal: the response to one request, or `None` for the
/// transport's own `Unknown` ("no semantic witness for this capability").
pub type Propose = fn(&VerificationRequestV2, &PluginManifest) -> Option<VerificationResponseV2>;

/// Serve one request with `propose`.
///
/// # Errors
///
/// A damaged template, an installed manifest that differs from it, or a
/// malformed request frame.
pub fn run(template: &str, propose: Propose) -> Result<(), String> {
    quirin_provider_shared::runtime::run_with_v2(template, Some(propose))
}

/// A response to `request` from `manifest`'s plugin, with one diagnostic
/// under the plugin's own code space.
#[must_use]
pub fn respond(
    request: &VerificationRequestV2,
    manifest: &PluginManifest,
    verdict: ProposedVerdict,
    note: &str,
    evidence: Option<ProposedEvidenceV2>,
) -> VerificationResponseV2 {
    VerificationResponseV2 {
        protocol_version: request.protocol_version,
        request_id: request.request_id.clone(),
        plugin_id: manifest.plugin_id.clone(),
        verdict,
        diagnostics: vec![PluginDiagnostic {
            code: format!("{}.note", manifest.plugin_id),
            message: note.to_owned(),
            fact_id: None,
        }],
        evidence,
    }
}

/// The function symbols a request's obligations name, in order.
#[must_use]
pub fn requested_functions(request: &VerificationRequestV2) -> Vec<&str> {
    request
        .obligations
        .iter()
        .filter_map(|obligation| match &obligation.subject {
            qcore_schemas::obligation::ObligationSubject::Function { symbol, .. } => {
                Some(symbol.as_str())
            }
            _ => None,
        })
        .collect()
}
