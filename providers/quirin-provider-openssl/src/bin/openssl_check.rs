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

//! `quirin-openssl-check`: the checker plugin bound to every OpenSSL contract
//! of this crate (card OPENSSL-PROVIDER-PLUGIN), on the SDK's checker runtime.
//! What it checks is `quirin_provider_openssl::check`.

fn main() {
    // The markers make the compiled-in template one contiguous, locatable
    // blob, so the host binds the installed manifest to this executable
    // without running it.
    if let Err(error) = quirin_provider_plugin::checker::run(
        concat!(
            "<<<qcore.provider-template.v1:begin>>>\n",
            include_str!("openssl-check-template.json"),
            "<<<qcore.provider-template.v1:end>>>"
        ),
        quirin_provider_openssl::check::answer,
    ) {
        eprintln!("quirin-openssl-check: {error}");
        std::process::exit(2);
    }
}
