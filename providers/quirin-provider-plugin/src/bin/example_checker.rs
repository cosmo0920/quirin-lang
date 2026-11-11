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

//! A worked checker plugin written with the SDK (card
//! W45-USER-CONTRACT-PLUGINS): it answers every request for the `acme`
//! library's pool contracts with a `Safe` **claim** and no certificate.
//!
//! The audit's firewall reads that as `Pending` (`plugin-claim-uncertified`):
//! a claim with nothing the audit can check is not a proof. A checker that
//! wants its `Safe` to count must return evidence a trusted-core checker in
//! the audit replays, and no such checker exists for user contracts yet.

use qcore_schemas::plugin::{
    PluginManifest, ProposedVerdict, VerificationRequestV2, VerificationResponseV2,
};
use quirin_provider_plugin::checker;

fn propose(
    request: &VerificationRequestV2,
    manifest: &PluginManifest,
) -> Option<VerificationResponseV2> {
    let functions = checker::requested_functions(request);
    if functions.is_empty() || !functions.iter().all(|name| name.starts_with("acme_pool_")) {
        return None;
    }
    Some(checker::respond(
        request,
        manifest,
        ProposedVerdict::Safe,
        "every pool function returns its item to the pool it came from (asserted, not certified)",
        None,
    ))
}

fn main() {
    // The markers make the compiled-in template one contiguous, locatable blob
    // in the executable, so the host binds the installed manifest to it
    // without executing this program.
    if let Err(error) = checker::run(
        concat!(
            "<<<qcore.provider-template.v1:begin>>>\n",
            include_str!("example-checker-template.json"),
            "<<<qcore.provider-template.v1:end>>>"
        ),
        propose,
    ) {
        eprintln!("checker: {error}");
        std::process::exit(2);
    }
}
