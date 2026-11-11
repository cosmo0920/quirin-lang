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

//! Schema-only adversarial exchange fixture, not a plugin or source verifier.
use qcore_schemas::reactor::*;

fn main() {
    let mode = std::env::args().nth(1).unwrap_or_else(|| "valid".into());
    let mut proposal = ReactorCallbackProposal {
        schema_version: REACTOR_CALLBACK_PROPOSAL_VERSION,
        callbacks: vec![ReactorCallbackRecord {
            operation_index: 1,
            event: EventKeyRecord {
                id: 1,
                generation: 1,
            },
            summary: CallbackSummaryRecord {
                argument: 9,
                argument_mode: ArgumentModeRecord::Owned,
                trigger_reasons: vec![ActivationReasonRecord::Read],
                payload_authority: vec![PayloadAuthorityRecord {
                    payload: 7,
                    kind: PayloadAccessKindRecord::Read,
                }],
                may_defer: false,
                may_spawn_tasks: false,
            },
        }],
    };
    match mode.as_str() {
        "valid" => {}
        "widen" => {
            proposal.callbacks[0].summary.payload_authority[0].kind = PayloadAccessKindRecord::Write
        }
        "generation" => proposal.callbacks[0].event.generation = 2,
        "omit" => proposal.callbacks.clear(),
        "duplicate" => proposal.callbacks.push(proposal.callbacks[0].clone()),
        "version" => proposal.schema_version += 1,
        _ => {
            eprintln!("unknown fixture mode");
            std::process::exit(2);
        }
    }
    serde_json::to_writer(std::io::stdout().lock(), &proposal).expect("write proposal");
}
