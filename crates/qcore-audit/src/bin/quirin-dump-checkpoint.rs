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

fn main() -> std::process::ExitCode {
    // Card RULE-SWITCH-ENTRY-POINTS: the dump records no rule set.
    qcore_c_gateway::gateway_rules::exit_if_unrecorded("quirin-dump-checkpoint");
    qcore_audit::dump_checkpoint_cli::main(std::env::args_os().skip(1).collect())
}
