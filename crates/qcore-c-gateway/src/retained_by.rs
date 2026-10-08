//! Card **CALLEE-RETENTION-CLASS1**, stage (b) (gateway rule 12t): whether a
//! pointer a callee retains outlives its holder, decided from what the caller
//! can see (contract spec §C04.14).
//!
//! USER DECISION 2026-10-05: the caller keeps custody of a retained pointer
//! `P`, but must establish that `P`'s object outlives the holder, or that the
//! holder is destroyed or reset before `P` is released; otherwise the result
//! is `Pending` or `Unknown`, never `Safe`. Route G decides "outlives" from
//! exactly these facts, and refuses everything else (`Unknown`, the named
//! reason [`UNDECIDED`]):
//!
//! * **A static `P`** (a string literal) outlives every holder.
//! * **A frame holder**: `&L` for `L` one automatic, non-pointer, non-array
//!   object of the calling function that the body names only as a whole
//!   `&L` actual of a retaining or resetting callee. `L`'s storage ends when
//!   the function returns, and no copy of what it holds is ever made. A `P`
//!   the caller of this function supplied (an unrebound formal this body
//!   holds no custody of) outlives the frame. A `P` this body tracks (one
//!   known live identity) is recorded as held by `L` until `L` is reset or
//!   the function returns.
//! * **A tracked heap holder**: a name bound to one known live identity this
//!   body tracks. Whatever it retains is recorded as held until the holder is
//!   reset or released; a holder still holding at an exit (returned, stored,
//!   leaked) outlives the frame, and the exit refuses.
//! * **Everything else is undecided**: a holder the caller supplied (a
//!   formal, a member reached through one), the `global` holder or a static
//!   object for a non-static `P`, an unresolved, ambiguous or derived actual.
//!
//! While a record says `P` is held, a release or a transfer of `P` is refused
//! ([`RELEASED_WHILE_HELD`]): a pointer left dangling in a live holder is a
//! hazard only if the holder reads it, so it is never accused (no `Unsafe`).
//!
//! The Lean model is `QCore.RetainedBy`; [`DECISION_VECTORS`] and
//! [`TRACE_VECTORS`] are shared with it.

/// The prefix a retaining callee's custody contract is installed under for
/// one classification (reserved: a source spelling it is escaped first).
pub(crate) const RETAINED_PREFIX: &str = "qcore_retained__";

/// The stable reason key of a retention whose lifetime the body does not
/// decide.
pub const UNDECIDED: &str = "retention-undecided";

/// The stable reason key of a release or transfer of a pointer a live holder
/// still retains.
pub const RELEASED_WHILE_HELD: &str = "retention-released-while-held";

/// Card **REFCOUNT-REFERENCE-RETURNING-DECLARATIONS** (rule 13e,
/// P-RETAINED-PREDEFINED): C11 6.4.2.2's predefined identifier and its two
/// GNU spellings. Each denotes a static array, so, like a string literal, it
/// outlives every holder (`variadic_nested`'s T6 reads them the same way).
pub const PREDEFINED_IDENTIFIERS: &[&str] = &["__func__", "__FUNCTION__", "__PRETTY_FUNCTION__"];

/// The minted name of `callee`'s custody contract.
pub(crate) fn retained_name(callee: &str) -> String {
    format!("{RETAINED_PREFIX}{callee}")
}

/// The callee a minted name stands for.
pub(crate) fn retained_callee(name: &str) -> Option<&str> {
    name.strip_prefix(RETAINED_PREFIX)
}

/// What the retained pointer's actual denotes.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Retained {
    /// Storage of static duration (a string literal).
    Static,
    /// The object an unrebound formal designates, which this body holds no
    /// custody of: the caller supplied it, so it outlives this frame.
    CallerObject(String),
    /// One known live identity this body tracks.
    Tracked(usize),
    /// Anything else.
    Other,
}

/// What the holder's actual denotes.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Holder {
    /// `&L`, `L` an automatic object of this frame named only at holder
    /// positions.
    Frame(String),
    /// One known live identity this body tracks.
    Tracked(usize),
    /// The `global` holder, or storage of static duration.
    Global,
    /// Anything else, a caller-supplied holder included.
    Foreign,
}

/// What a recorded retention keys on.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum HeldObject {
    /// A tracked identity: its release or transfer is refused while held.
    Site(usize),
    /// A caller's object: this body cannot release it, so only the exit
    /// check reads it.
    Name(String),
}

/// What holds a recorded retention.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum HeldBy {
    /// An automatic object of this frame: dies at every exit.
    Frame(String),
    /// A tracked identity: holds until reset or released.
    Site(usize),
}

/// The decision at one retaining call.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Decision {
    /// The retained object outlives the holder: nothing to record.
    Outlives,
    /// Recorded until the holder is reset, released or dies.
    Record(HeldObject, HeldBy),
    /// Undecided: the call refuses ([`UNDECIDED`]).
    Undecided,
}

/// Rule 12t's decision table.
#[must_use]
pub fn decide(retained: &Retained, holder: &Holder) -> Decision {
    match (retained, holder) {
        (Retained::Static, _) => Decision::Outlives,
        (Retained::Other, _) | (_, Holder::Foreign | Holder::Global) => Decision::Undecided,
        (Retained::CallerObject(_), Holder::Frame(_)) => Decision::Outlives,
        (Retained::CallerObject(name), Holder::Tracked(site)) => {
            Decision::Record(HeldObject::Name(name.clone()), HeldBy::Site(*site))
        }
        (Retained::Tracked(site), Holder::Frame(name)) => {
            Decision::Record(HeldObject::Site(*site), HeldBy::Frame(name.clone()))
        }
        (Retained::Tracked(site), Holder::Tracked(holder)) => {
            Decision::Record(HeldObject::Site(*site), HeldBy::Site(*holder))
        }
    }
}

/// The refusal of an undecided retention.
#[must_use]
pub fn undecided_description(callee: &str, retained: &str, holder: &str) -> String {
    format!(
        "call to `{callee}` retains `{retained}` in `{holder}`, whose lifetime this body does not \
         decide against it ({UNDECIDED})"
    )
}

/// The refusal of a release or transfer of a held object.
#[must_use]
pub fn released_while_held_description(name: &str) -> String {
    format!(
        "`{name}` is released or transferred while a live holder retains it ({RELEASED_WHILE_HELD})"
    )
}

/// The refusal of an exit at which a tracked holder still retains something.
#[must_use]
pub fn exit_description(name: &str) -> String {
    format!("a holder retaining `{name}` may outlive this function's frame ({UNDECIDED})")
}

/// One event of a path, for the shared trace vectors: what the transfer
/// does to the records.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Event {
    /// A retaining call with its decision.
    Retain(Decision),
    /// A release or transfer of a tracked identity.
    Release(usize),
    /// A reset of a frame holder.
    ResetFrame(String),
    /// A reset of a tracked holder.
    ResetSite(usize),
    /// A release of a tracked holder.
    ReleaseHolder(usize),
}

/// The outcome of a path: `Safe` only when no event refused and no tracked
/// holder still holds at the exit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    Safe,
    Refused,
}

/// Replay one path's events over the records, as route G's transfer does.
#[must_use]
pub fn replay(events: &[Event]) -> Outcome {
    let mut held: Vec<(HeldObject, HeldBy)> = Vec::new();
    for event in events {
        match event {
            Event::Retain(Decision::Outlives) => {}
            Event::Retain(Decision::Undecided) => return Outcome::Refused,
            Event::Retain(Decision::Record(object, by)) => held.push((object.clone(), by.clone())),
            Event::Release(site) => {
                if held
                    .iter()
                    .any(|(object, _)| *object == HeldObject::Site(*site))
                {
                    return Outcome::Refused;
                }
                // A released holder holds nothing any more.
                held.retain(|(_, by)| *by != HeldBy::Site(*site));
            }
            Event::ResetFrame(name) => held.retain(|(_, by)| *by != HeldBy::Frame(name.clone())),
            Event::ResetSite(site) | Event::ReleaseHolder(site) => {
                held.retain(|(_, by)| *by != HeldBy::Site(*site));
            }
        }
    }
    // A frame holder dies at the exit; a tracked holder still holding
    // outlives the frame.
    if held.iter().any(|(_, by)| matches!(by, HeldBy::Site(_))) {
        Outcome::Refused
    } else {
        Outcome::Safe
    }
}

/// `decisionVectors` of `QCore.RetainedBy`, row by row: retained class,
/// holder class, whether the call records, refuses or decides outright.
pub const DECISION_VECTORS: &[(&str, &str, &str)] = &[
    ("static", "frame", "outlives"),
    ("static", "tracked", "outlives"),
    ("static", "global", "outlives"),
    ("static", "foreign", "outlives"),
    ("caller", "frame", "outlives"),
    ("caller", "tracked", "record"),
    ("caller", "global", "undecided"),
    ("caller", "foreign", "undecided"),
    ("tracked", "frame", "record"),
    ("tracked", "tracked", "record"),
    ("tracked", "global", "undecided"),
    ("tracked", "foreign", "undecided"),
    ("other", "frame", "undecided"),
    ("other", "tracked", "undecided"),
    ("other", "global", "undecided"),
    ("other", "foreign", "undecided"),
];

/// `traceVectors` of `QCore.RetainedBy`: the brief's controls as event
/// paths, and whether each is `Safe`.
pub const TRACE_VECTORS: &[(&str, bool)] = &[
    // P (site 1) held by the frame holder L, L reset, then P released.
    ("frame-reset-then-release", true),
    // P released while L still holds it.
    ("release-before-reset", false),
    // P held by a tracked holder (site 2) that is released first.
    ("holder-released-then-release", true),
    // The tracked holder outlives P: P released while held.
    ("holder-outlives-p", false),
    // The tracked holder still holds at the exit.
    ("holder-at-exit", false),
    // A frame holder dies at the exit with P still live.
    ("frame-dies-at-exit", true),
    // A string literal in the global holder.
    ("static-in-global", true),
    // A tracked P in the global holder.
    ("tracked-in-global", false),
    // A caller-supplied holder.
    ("unknown-holder", false),
];

/// The event path a [`TRACE_VECTORS`] row names.
#[must_use]
pub fn trace(name: &str) -> Vec<Event> {
    let frame = || Holder::Frame("L".into());
    match name {
        "frame-reset-then-release" => vec![
            Event::Retain(decide(&Retained::Tracked(1), &frame())),
            Event::ResetFrame("L".into()),
            Event::Release(1),
        ],
        "release-before-reset" => vec![
            Event::Retain(decide(&Retained::Tracked(1), &frame())),
            Event::Release(1),
            Event::ResetFrame("L".into()),
        ],
        "holder-released-then-release" => vec![
            Event::Retain(decide(&Retained::Tracked(1), &Holder::Tracked(2))),
            Event::ReleaseHolder(2),
            Event::Release(1),
        ],
        "holder-outlives-p" => vec![
            Event::Retain(decide(&Retained::Tracked(1), &Holder::Tracked(2))),
            Event::Release(1),
            Event::ReleaseHolder(2),
        ],
        "holder-at-exit" => vec![Event::Retain(decide(
            &Retained::CallerObject("src".into()),
            &Holder::Tracked(2),
        ))],
        "frame-dies-at-exit" => vec![Event::Retain(decide(&Retained::Tracked(1), &frame()))],
        "static-in-global" => vec![Event::Retain(decide(&Retained::Static, &Holder::Global))],
        "tracked-in-global" => vec![Event::Retain(decide(
            &Retained::Tracked(1),
            &Holder::Global,
        ))],
        "unknown-holder" => vec![Event::Retain(decide(
            &Retained::CallerObject("src".into()),
            &Holder::Foreign,
        ))],
        _ => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn retained(name: &str) -> Retained {
        match name {
            "static" => Retained::Static,
            "caller" => Retained::CallerObject("p".into()),
            "tracked" => Retained::Tracked(1),
            _ => Retained::Other,
        }
    }

    fn holder(name: &str) -> Holder {
        match name {
            "frame" => Holder::Frame("L".into()),
            "tracked" => Holder::Tracked(2),
            "global" => Holder::Global,
            _ => Holder::Foreign,
        }
    }

    #[test]
    fn decision_vectors_bind_the_table() {
        for (p, h, expected) in DECISION_VECTORS {
            let label = match decide(&retained(p), &holder(h)) {
                Decision::Outlives => "outlives",
                Decision::Record(..) => "record",
                Decision::Undecided => "undecided",
            };
            assert_eq!(label, *expected, "{p} {h}");
        }
    }

    #[test]
    fn trace_vectors_bind_the_replay() {
        for (name, safe) in TRACE_VECTORS {
            let outcome = replay(&trace(name));
            assert_eq!(outcome == Outcome::Safe, *safe, "{name}");
        }
    }

    #[test]
    fn minted_names_round_trip() {
        assert_eq!(retained_callee(&retained_name("f")), Some("f"));
        assert_eq!(retained_callee("f"), None);
    }
}
