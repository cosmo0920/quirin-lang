//! Card **GAPO-RETURN-CORRECTNESS** (gateway rule 15y): a call to a callee
//! declared `resource_no_return()` ends its path at every position a route
//! lowered it to, not only where rule 11o's statement hook saw it.
//!
//! Rule 11o's hook reads the instruction `classify_simple` (and rule 12h)
//! produced for an expression statement. A no-return call lowered later — by
//! W44's call actuals, 10u's substituted call, 10y's nested calls, 11w's
//! widened tail, an elaborated or connective piece, a return evaluation, a
//! condition's call or plan, a `for` clause — reached the CFG as an ordinary
//! interior node with a continuation, so the path after it was analysed as
//! if C continued (ROUTE-MAP finding 4). After the CFG is built,
//! [`Parser::settle_no_return_nodes`] reads every node:
//!
//! * a node with no successor (rule 11o's path end, an exit) is unchanged;
//! * a node whose instruction calls no declared no-return callee is
//!   unchanged;
//! * a node that is exactly one discarded whole call of such a callee, with
//!   no nested call and no loop exit edge, takes rule 11o's decision
//!   ([`call_exits::decide`] over this body's Clang resolution): a path end
//!   (its successors removed; it is not an exit, and its own instruction is
//!   still transferred, so the call's preconditions are checked) where the
//!   declaration is `noreturn`, the refusal otherwise;
//! * every other node holding such a call (stored, nested, inside an element
//!   or sub-object store, a loop header) is refused with unbounded writes: no
//!   position lets the path after a no-return call continue.
//!
//! It never admits: a path end removes successors only where rule 11o's own
//! premises hold, and a refusal replaces an admission. Every settled node is
//! a parity row (`position: "no-return-node"`).

use std::collections::BTreeSet;

use serde_json::json;

use super::{LegacyCfg, LegacyInstruction, Parser, RefusedWrites};
use crate::call_exits::{self, Decision};
use crate::typed_access_path::record_row;

/// The refusal of a no-return call whose node is not its discarded whole
/// call.
pub(super) fn embedded_description(callee: &str) -> String {
    format!(
        "no-return call `{callee}` is not the whole discarded call of its node, so its path \
         cannot end there (rule 15y)"
    )
}

/// Every callee `instruction` calls: its own call, nested calls and the
/// calls of the instructions it carries. The match names every variant, so
/// a new call-bearing variant does not compile until it is read here.
pub(super) fn called(instruction: &LegacyInstruction, out: &mut Vec<String>) {
    match instruction {
        LegacyInstruction::ContractCall {
            function, nested, ..
        } => {
            out.push(function.clone());
            out.extend(nested.iter().map(|nested| nested.function.clone()));
        }
        LegacyInstruction::ElementStore { call, fallback, .. } => {
            if let Some(call) = call {
                called(call, out);
            }
            called(fallback, out);
        }
        LegacyInstruction::ElementRelease {
            callee, fallback, ..
        } => {
            out.push(callee.clone());
            called(fallback, out);
        }
        LegacyInstruction::BranchOnElementNull { fallback, .. } => called(fallback, out),
        LegacyInstruction::SubObjectStore {
            value, fallback, ..
        } => {
            if let super::sub_object_store::StoredValue::Call(call) = value {
                called(call, out);
            }
            called(fallback, out);
        }
        LegacyInstruction::PointerStore {
            access, fallback, ..
        } => {
            called(access, out);
            called(fallback, out);
        }
        LegacyInstruction::SelfInitCall { function, .. }
        | LegacyInstruction::TrackedSelfInit { function, .. } => {
            out.push(function.clone());
        }
        LegacyInstruction::Acquire { .. }
        | LegacyInstruction::Alias { .. }
        | LegacyInstruction::Clear { .. }
        | LegacyInstruction::Release { .. }
        | LegacyInstruction::Escape { .. }
        | LegacyInstruction::EscapeIfOwned { .. }
        | LegacyInstruction::ExactFormalReturn { .. }
        | LegacyInstruction::BranchOnNull { .. }
        | LegacyInstruction::BranchOnValidity { .. }
        | LegacyInstruction::BranchOnResult { .. }
        | LegacyInstruction::BranchOnDistinct { .. }
        | LegacyInstruction::CellProtocolStep { .. }
        | LegacyInstruction::OpaqueValue { .. }
        | LegacyInstruction::VaArgPointer { .. }
        | LegacyInstruction::ForgetScalar { .. }
        | LegacyInstruction::ScalarMemberStore { .. }
        | LegacyInstruction::IndirectScalarAccess { .. }
        | LegacyInstruction::CountedLoopHeader { .. }
        | LegacyInstruction::CountedLoopStep { .. }
        | LegacyInstruction::PointerStep { .. }
        | LegacyInstruction::ArgumentReads { .. }
        | LegacyInstruction::AggregateStore { .. }
        | LegacyInstruction::NoOp
        | LegacyInstruction::Unknown { .. } => {}
    }
}

impl Parser<'_> {
    /// Settle every node of `cfg` that calls a declared no-return callee and
    /// still has a successor (module documentation).
    pub(super) fn settle_no_return_nodes(&self, cfg: &mut LegacyCfg) {
        let declared = self
            .declared_no_return
            .get_or_init(|| call_exits::declared_no_return(self.contracts));
        if declared.is_empty() {
            return;
        }
        for node in &mut cfg.nodes {
            if node.successors.is_empty() {
                continue;
            }
            let mut callees = Vec::new();
            called(&node.instruction, &mut callees);
            let hits = callees
                .iter()
                .filter(|callee| declared.contains(*callee))
                .cloned()
                .collect::<BTreeSet<_>>();
            let Some(first) = hits.iter().next().cloned() else {
                continue;
            };
            let whole = match &node.instruction {
                LegacyInstruction::ContractCall {
                    function,
                    destination: None,
                    nested,
                    ..
                } if nested.is_empty()
                    && callees.len() == 1
                    && node.loop_exit_edge.is_none()
                    && !node.is_exit =>
                {
                    Some(function.clone())
                }
                _ => None,
            };
            let (decision, resolution, description) = match &whole {
                Some(function) => {
                    let resolution = self.call_exit_resolution(function);
                    let decision = call_exits::decide(true, true, resolution);
                    (
                        decision,
                        Some(resolution),
                        call_exits::unestablished_description(function, resolution),
                    )
                }
                None => (Decision::Refused, None, embedded_description(&first)),
            };
            record_row(json!({
                "function": self.typed_path_function,
                "line": node.line,
                "position": "no-return-node",
                "callees": hits,
                "whole_discarded_call": whole.is_some(),
                "resolution": resolution.map(|resolution| format!("{resolution:?}")),
                "decision": format!("{decision:?}"),
                "new_admitted": false,
            }));
            match decision {
                Decision::PathEnd => node.successors.clear(),
                Decision::Refused | Decision::Continue => {
                    node.instruction = LegacyInstruction::Unknown {
                        description,
                        writes: RefusedWrites::Unbounded,
                    };
                }
            }
        }
    }
}
