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

//! Card **W48-RK-LOG-CALLER-FRAME** (gateway rule 32a): a caller-frame
//! declaration serves only callers outside its own frame.
//!
//! # The gap
//!
//! Rule 32's caller-frame declarations (`rd_kafka_log0`, the static
//! `rd_kafka_log_buf`) state what a *caller* sees of the logger: the
//! identity on its own custody, under three named premises. They say nothing
//! about the logger's own body: the reviewed `logger-effects-v1` model's
//! fifteen obligations are all still Open, and the projection only classifies
//! them as not the caller's. Yet `rd_kafka_log0`'s own body calls
//! `rd_kafka_log_buf`, and that call read the sibling's caller-frame
//! declaration like any other caller's, so `rd_kafka_log0`'s own row became
//! `Safe` on the frame's own declaration, whose body standing is a declared
//! assumption (its body never validates it, rule 13i).
//!
//! # The rule
//!
//! Inside the body of a member of a reviewed caller-frame projection
//! (`qcore_contract::caller_frame::FRAMES`), every call to a member of the same
//! frame, the function's own recursive call included, is **held**: rewritten
//! to a marked, undeclared name before any reader sees the stream, exactly as
//! a refuted declaration is held (the LOADER-ABI technique), so W4
//! derivation, W4 validation and a certificate recheck reproduce it. The
//! statement refuses with [`hold_description`], whose reason key is
//! [`CALLER_FRAME_SELF_RELIANCE`] (category `incomplete-obligation-coverage`:
//! the frame's own obligations have no derivation), and is `Unknown`: it is
//! the tool's decision, never `Pending`. Each member's row is therefore
//! decided by its body; callers outside the frame still read the declaration
//! with its premises.
//!
//! The hold applies only where the callee's declaration is in force: without
//! it the call is the ordinary uncontracted (or W4-summarised) call, which is
//! already decided by the callee's body.
//!
//! Monotone in the refusing direction: a held call is an unmodelled call, so
//! no verdict moves towards `Safe` or towards `Unsafe`. Lean:
//! `QCore.RkLogCallerFrame` (`a_member_never_reads_a_member_declaration`,
//! `a_held_member_row_is_never_safe`, `an_outside_caller_reads_the_declaration`,
//! `the_hold_never_certifies`).
//!
//! # The package and its host
//!
//! Card RULE32A-PILOT: which calls are held is decided by rule 32a's package,
//! `rule-32a-caller-frame-hold` (registry row `32a`, the rule interface's
//! token hold), asked through `arm_obligations::consult` (card
//! ARM-OBLIGATIONS: every package admission in the gateway is read there; the
//! hold's admission carries no obligation, so it needs no unit frame and is
//! its lowering, as before). This module is its host
//! ([`FrameHoldHost`]): it finds each registered member's own body, the call
//! positions inside it (`is_call_position`), and reads the frames
//! (`caller_frame::FRAMES`) and whether a callee's declaration is in force
//! for the package; the held spelling and the refusal text stay here. The
//! host asks only inside a member's body: the package holds no call of any
//! other caller (`within_one_frame` needs the caller to be a member), so a
//! unit that defines no member is unchanged and unasked, as before.
//!
//! The hold is not switchable (`off = "not-switchable"`): a switch list
//! naming `32a` is refused, and `gateway_rules::enabled` answers `true` for a
//! hold under every list, a refused one included, so no list can remove it
//! (Lean `QCore.CallerFrameHoldSwitch`).

use qcore_contract::caller_frame;
use qcore_gateway_rule_api::traits::{HoldQuery, Host, RulePackage, TokenHold};
use qcore_schemas::contract::ResourceContracts;
use rule_32a_caller_frame_hold::{CallerFrameHold, FrameHost};

use super::declared_abi::is_call_position;
use super::{Token, find_function_body, matching};

/// The stable reason key of a held call: the key rule 32a's package
/// reserves (`rule_32a_caller_frame_hold::SELF_RELIANCE`).
pub const CALLER_FRAME_SELF_RELIANCE: &str = rule_32a_caller_frame_hold::SELF_RELIANCE.key;

/// The prefix a held callee's call sites are rewritten with, reserved for
/// this rewrite (`reserved_spelling` escapes a source that spells it).
pub(crate) const HELD_PREFIX: &str = "qcore_caller_frame_self__";

/// The original callee a rewritten name stands for.
pub(crate) fn held_callee(function: &str) -> Option<&str> {
    function.strip_prefix(HELD_PREFIX)
}

/// The refusal a held call site carries.
#[must_use]
pub fn hold_description(callee: &str) -> String {
    format!(
        "call to `{callee}` inside its own caller frame: a caller-frame declaration serves only \
         callers outside the frame, whose own bodies decide their rows \
         ({CALLER_FRAME_SELF_RELIANCE})"
    )
}

/// The callee a held call site's refusal names, if `description` is one.
#[must_use]
pub fn held_call(description: &str) -> Option<&str> {
    description
        .strip_prefix("call to `")?
        .strip_suffix(&format!(
            "` inside its own caller frame: a caller-frame declaration serves only callers outside \
         the frame, whose own bodies decide their rows ({CALLER_FRAME_SELF_RELIANCE})"
        ))
}

/// The token range of `function`'s own body, if the unit defines one.
fn own_body_range(tokens: &[Token], function: &str) -> Option<(usize, usize)> {
    let (open, _) = find_function_body(tokens, function)?;
    let close = matching(tokens, open, "{", "}")?;
    Some((open, close))
}

/// The gateway as rule 32a's host: the unit's contract environment and the
/// registered caller frames, read in names only.
pub(crate) struct FrameHoldHost<'c> {
    contracts: &'c ResourceContracts,
}

impl Host for FrameHoldHost<'_> {
    type Token = Token;
    type Decision = ();
    type Statement = ();
    type Condition = ();
    type Node = ();

    fn token_text(token: &Token) -> &str {
        &token.text
    }

    fn token_line(token: &Token) -> usize {
        token.line
    }

    fn refusal(_: &()) -> Option<&str> {
        None
    }
}

impl FrameHost for FrameHoldHost<'_> {
    fn frames(&self) -> &[&[&str]] {
        caller_frame::FRAMES
    }

    fn declaration_in_force(&self, function: &str) -> bool {
        self.contracts.contract(function).is_some()
    }
}

/// The rule packages at this hold, in consultation order.
const HOLDS: &[CallerFrameHold] = &[CallerFrameHold];

/// Whether rule 32a's package holds `call` (callee first) in `caller`'s
/// body. Asked through the switch, which never removes a hold.
fn held_by_rule(host: &FrameHoldHost<'_>, caller: &str, call: &[Token]) -> bool {
    let query = HoldQuery::<FrameHoldHost<'_>> {
        caller,
        callee: &call[0].text,
        call,
        line: call[0].line,
    };
    crate::arm_obligations::consult(
        HOLDS,
        |package| package.id(),
        |package| crate::gateway_rules::enabled(package.id()),
        |_, package| TokenHold::ask(package, host, &query),
    )
    .is_some()
}

/// Rewrite every call, inside a frame member's own body, that rule 32a's
/// package holds: a call to a member of the same frame whose declaration is
/// in force. `None` means the stream is unchanged, which is every unit that
/// defines no member.
pub(crate) fn hold_call_sites(
    tokens: &[Token],
    contracts: &ResourceContracts,
) -> Option<Vec<Token>> {
    let bodies = caller_frame::FRAMES
        .iter()
        .flat_map(|members| members.iter())
        .filter_map(|member| own_body_range(tokens, member).map(|range| (*member, range)))
        .collect::<Vec<_>>();
    if bodies.is_empty() {
        return None;
    }
    let host = FrameHoldHost { contracts };
    let mut rewritten: Option<Vec<Token>> = None;
    for index in 0..tokens.len() {
        if tokens.get(index + 1).is_none_or(|token| token.text != "(") {
            continue;
        }
        let mut owners = bodies
            .iter()
            .filter(|(_, (start, end))| index > *start && index < *end)
            .map(|(owner, _)| *owner)
            .peekable();
        if owners.peek().is_none() || !is_call_position(tokens, index) {
            continue;
        }
        let close = matching(tokens, index + 1, "(", ")").unwrap_or(index + 1);
        let call = &tokens[index..=close];
        if !owners.any(|owner| held_by_rule(&host, owner, call)) {
            continue;
        }
        let name = tokens[index].text.as_str();
        let target = rewritten.get_or_insert_with(|| tokens.to_vec());
        target[index].text = format!("{HELD_PREFIX}{name}");
    }
    rewritten
}
