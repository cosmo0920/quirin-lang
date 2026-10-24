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

use qcore_contract::caller_frame;
use qcore_schemas::contract::ResourceContracts;

use super::declared_abi::is_call_position;
use super::{Token, find_function_body, matching};

/// The stable reason key of a held call.
pub const CALLER_FRAME_SELF_RELIANCE: &str = "caller-frame-self-reliance";

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

/// Rewrite every call, inside a frame member's own body, to a member of the
/// same frame whose declaration is in force. `None` means the stream is
/// unchanged, which is every unit that defines no member.
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
    let mut rewritten: Option<Vec<Token>> = None;
    for index in 0..tokens.len() {
        if tokens.get(index + 1).is_none_or(|token| token.text != "(") {
            continue;
        }
        let name = tokens[index].text.as_str();
        if !caller_frame::is_frame_member(name)
            || contracts.contract(name).is_none()
            || !is_call_position(tokens, index)
        {
            continue;
        }
        let inside = bodies.iter().any(|(owner, (start, end))| {
            index > *start && index < *end && caller_frame::within_one_frame(owner, name)
        });
        if !inside {
            continue;
        }
        let target = rewritten.get_or_insert_with(|| tokens.to_vec());
        target[index].text = format!("{HELD_PREFIX}{name}");
    }
    rewritten
}
