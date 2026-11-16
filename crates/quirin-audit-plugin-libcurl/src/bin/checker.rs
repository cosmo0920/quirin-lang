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

//! `quirin-libcurl-checker`: the checker package `quirin-audit libcurl
//! install` installs beside the libcurl user contract crate. See
//! `quirin_audit_plugin_libcurl::provider`.

fn main() {
    // The markers make the compiled-in template one contiguous, locatable
    // blob in the executable, so the host binds the installed manifest to it
    // without running this program.
    if let Err(error) = quirin_audit_plugin_libcurl::provider::run(concat!(
        "<<<qcore.provider-template.v1:begin>>>\n",
        include_str!("../../checker-template.json"),
        "<<<qcore.provider-template.v1:end>>>"
    )) {
        eprintln!("quirin-libcurl-checker: {error}");
        std::process::exit(2);
    }
}
