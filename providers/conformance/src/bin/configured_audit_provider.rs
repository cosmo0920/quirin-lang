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

//! Schemas-only fixture accepting concrete revisions pinned by the audit host.
use quirin_provider_shared::{runtime, source_effect};

fn main() {
    if let Err(error) = runtime::run_with_v2(
        concat!(
            "<<<qcore.provider-template.v1:begin>>>\n",
            include_str!(
                "../../tests/fixtures/configured_boolean_source/audit-provider-template.json"
            ),
            "<<<qcore.provider-template.v1:end>>>"
        ),
        Some(source_effect::propose),
    ) {
        eprintln!("configured audit provider: {error}");
        std::process::exit(2);
    }
}
