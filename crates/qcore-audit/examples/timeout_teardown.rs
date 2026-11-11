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

use std::path::PathBuf;

fn main() -> Result<(), String> {
    // Card RULE-SWITCH-ENTRY-POINTS: this example records no gateway rule set.
    qcore_c_gateway::gateway_rules::exit_if_unrecorded("example timeout_teardown");
    let mut arguments = std::env::args_os().skip(1);
    let config = arguments
        .next()
        .ok_or("usage: timeout_teardown CONFIG.json")?;
    if arguments.next().is_some() {
        return Err("usage: timeout_teardown CONFIG.json".into());
    }
    qcore_audit::timeout_teardown::run_timeout_teardown_config(&PathBuf::from(config))
}
