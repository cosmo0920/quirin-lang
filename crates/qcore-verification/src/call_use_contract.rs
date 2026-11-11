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

//! The single decision procedure for source call-result evidence.
//!
//! Passing this check records which cleanup call results a body ignores and
//! which it guards, in operation order. It never certifies successful OS
//! cleanup, descriptor or mapping custody, or discharge of any resource: a
//! recorded `close` call is a recorded call, nothing more.
//!
//! This body was relocated verbatim from the in-process ChunkIO adapter so
//! that the retained in-process route and the moved process route share one
//! decision procedure rather than two that could drift. The adapter now
//! selects the policy and delegates here; its own certificate type, its serde
//! shape and every error string are unchanged.
use qcore_c_gateway::source_call_use::{PathCallUse, analyze_source_call_uses};
use qcore_c_gateway::typed_source::SourceSummary;
use serde::Serialize;

/// Which contract policy decides one body, and which callees its facts count.
/// This is caller-supplied selection, exactly as the contract text is; it is
/// bound into a verification task's `contract_sources` on the moved route.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub struct CallResultUsePolicy {
    /// The declared contract policy name evaluated per path.
    pub policy: String,
    /// The primary cleanup callee whose result use is counted.
    pub first: String,
    /// The optional second callee (the wrapper family's native close).
    pub second: Option<String>,
    /// The exact C signature the imported body must have.
    pub signature: String,
}

/// The certificate the decision procedure issues. `status` is deliberately a
/// sentence about what is *not* established.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct CallResultUseCertificate {
    source: SourceSummary,
    calls: Vec<PathCallUse>,
    contract_source: String,
    status: String,
}

impl CallResultUseCertificate {
    pub fn source(&self) -> &SourceSummary {
        &self.source
    }
    pub fn calls(&self) -> &[PathCallUse] {
        &self.calls
    }
    pub fn contract_source(&self) -> &str {
        &self.contract_source
    }
    pub fn status(&self) -> &str {
        &self.status
    }
}

/// The status sentence every certificate carries, kept as one constant so the
/// two routes cannot drift apart on it.
pub const CALL_RESULT_USE_STATUS: &str =
    "checked source result-use facts; OS success/discharge and full filesystem custody Unknown";

pub fn check_call_result_use(
    source: &SourceSummary,
    contract: &str,
    selection: &CallResultUsePolicy,
) -> Result<CallResultUseCertificate, String> {
    let calls = analyze_source_call_uses(source)?;
    let CallResultUsePolicy {
        policy,
        first,
        second,
        signature,
    } = selection;
    if source.signature() != signature {
        return Err("filesystem result-use signature mismatch".into());
    }
    let any = calls
        .iter()
        .any(|path| path.calls.iter().any(|call| &call.function == first));
    for path in &calls {
        let selected: Vec<_> = path
            .calls
            .iter()
            .filter(|call| &call.function == first)
            .collect();
        if selected.iter().any(|call| call.result_type != "int") {
            return Err("cleanup result is outside selected int profile".into());
        }
        let mut facts = vec![
            ("source_has_cleanup_attempts", i128::from(any)),
            ("cleanup_calls", selected.len() as i128),
            (
                "ignored_results",
                selected.iter().filter(|c| c.uses.is_empty()).count() as i128,
            ),
            (
                "guarded_results",
                selected.iter().filter(|c| !c.guard_uses.is_empty()).count() as i128,
            ),
        ];
        if let Some(second) = second {
            let close: Vec<_> = path
                .calls
                .iter()
                .filter(|call| &call.function == second)
                .collect();
            if close.iter().any(|call| call.result_type != "int") {
                return Err("native close result is outside int profile".into());
            }
            let free: Vec<_> = path
                .calls
                .iter()
                .filter(|call| call.function == "free")
                .collect();
            facts.extend([
                ("close_calls", close.len() as i128),
                (
                    "ignored_close_results",
                    close.iter().filter(|c| c.uses.is_empty()).count() as i128,
                ),
                ("free_calls", free.len() as i128),
                (
                    "cleanup_order_retained",
                    i128::from(selected.iter().all(|unmap| {
                        close.iter().all(|close| {
                            unmap.operation < close.operation
                                && free.iter().all(|free| close.operation < free.operation)
                        })
                    })),
                ),
            ]);
        }
        qcore_contract::contract_observation::check_contract_facts(contract, policy, &facts)?;
    }
    Ok(CallResultUseCertificate {
        source: source.clone(),
        calls,
        contract_source: contract.into(),
        status: CALL_RESULT_USE_STATUS.into(),
    })
}
