//! Rule 32a's token hold `caller-frame-hold`: a call inside its own caller
//! frame is held.

use qcore_gateway_rule_api::traits::{Hold, HoldQuery, Host, RulePackage, TokenHold};
use qcore_gateway_rule_api::{Answer, ReasonCategory, ReasonKey};

/// The reason key of a held call, reserved by rule 32a's registry row: the
/// frame's own obligations have no derivation, so the held statement is the
/// analyser's refusal (`Unknown`), never `Pending`.
pub const SELF_RELIANCE: ReasonKey = ReasonKey {
    key: "caller-frame-self-reliance",
    category: ReasonCategory::IncompleteObligationCoverage,
};

/// What the hold reads of its host, in names only.
pub trait FrameHost: Host {
    /// The registered caller frames, each the member set of one reviewed
    /// caller-frame projection (its `declarations`).
    fn frames(&self) -> &[&[&str]];
    /// Whether `function`'s declaration is in force in the unit's contract
    /// environment. Without it a call is the ordinary uncontracted (or
    /// summarised) call, already decided by the callee's body, so there is
    /// no declaration to hold.
    fn declaration_in_force(&self, function: &str) -> bool;
}

/// The package: rule 32a at the token-hold position.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CallerFrameHold;

impl RulePackage for CallerFrameHold {
    fn id(&self) -> &'static str {
        crate::ID
    }
}

impl<H: FrameHost + ?Sized> TokenHold<H> for CallerFrameHold {
    fn ask(&self, host: &H, query: &HoldQuery<'_, H>) -> Answer<Hold> {
        decide(
            host.frames(),
            query.caller,
            query.callee,
            host.declaration_in_force(query.callee),
        )
    }
}

/// Whether `caller`'s body calling `callee` stays inside one frame: both are
/// members of the same registered frame (`caller == callee` included).
#[must_use]
pub fn within_one_frame(frames: &[&[&str]], caller: &str, callee: &str) -> bool {
    frames
        .iter()
        .any(|members| members.contains(&caller) && members.contains(&callee))
}

/// The hold's decision on one call: held under [`SELF_RELIANCE`] when the
/// call stays inside one frame and the callee's declaration is in force,
/// otherwise a refusal (not rule 32a's shape: the call reads whatever it
/// read before, a declaration for a caller outside the frame included). The
/// admission carries no obligation: it is itself the refusal of a reading.
#[must_use]
pub fn decide(frames: &[&[&str]], caller: &str, callee: &str, in_force: bool) -> Answer<Hold> {
    if in_force && within_one_frame(frames, caller, callee) {
        Answer::admit(Hold {
            reason: SELF_RELIANCE,
        })
    } else {
        Answer::Refuse
    }
}

/// The decision for one registered frame, `(caller is a member, callee is a
/// member, callee's declaration in force, held)`. The Lean model's
/// `holdVectors` (`QCore.CallerFrameHoldSwitch`) is this table, and a test
/// holds the two equal.
pub const DECISION_VECTORS: &[(bool, bool, bool, bool)] = &[
    (false, false, false, false),
    (false, false, true, false),
    (false, true, false, false),
    (false, true, true, false),
    (true, false, false, false),
    (true, false, true, false),
    (true, true, false, false),
    (true, true, true, true),
];

#[cfg(test)]
#[path = "../tests/unit/hold.rs"]
mod tests;
