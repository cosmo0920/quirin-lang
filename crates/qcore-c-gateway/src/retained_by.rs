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
//!
//! Card **FB-EVENT-LOOP-RETENTION-AND-REGISTRATION** (USER DECISIONS
//! 2026-10-16, "Tracked holder path" and "R3: quant state"): a holder reached
//! through a member of an unrebound caller formal is a **path holder**
//! ([`Holder::Path`]); a retention there is recorded as held by that path and,
//! like a tracked holder, refuses at an exit unless a removal of exactly that
//! registration (`quant live #H: #holds(P) |-> #absent;`, [`Event::Remove`])
//! drops it first. A removal drops only the exactly matching record; an
//! unmatched removal drops nothing. A pointer into a member of an object
//! (`&X->f`) is held as that interior ([`HeldObject::SiteInterior`]), and a
//! release of the object while an interior is held refuses as a release of a
//! held object does. The Lean model is `QCore.RetentionPath`, bound by
//! [`PATH_DECISION_VECTORS`] and [`PATH_TRACE_VECTORS`].
//!
//! Card **RETENTION-UNMATCHED-REMOVAL-POISONS** (USER DECISION 2026-10-17,
//! "Yes, small fix card now"): Monkey's `_mk_event_del` clears the event's
//! registered status even through a loop that does not hold it, so a later
//! removal through the right loop does nothing. A removal therefore drops
//! only the exactly matching record that is not poisoned, and **poisons**
//! every other record whose held object it may name ([`remove`]); a poisoned
//! record ends only when its holder ends ([`end_holder`]: the loop's destroy,
//! a released tracked holder, a declared reset), refuses at an exit as
//! [`POISONED`], and a release of its object while it is held refuses.

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
    /// Card FB-EVENT-LOOP-RETENTION-AND-REGISTRATION: a member of one known
    /// live identity this body tracks (`&X->f`), the members outermost first.
    TrackedInterior(usize, Vec<String>),
    /// Anything else.
    Other,
}

/// Card **FB-EVENT-LOOP-RETENTION-AND-REGISTRATION**: what a path holder
/// starts at.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PathRoot {
    /// An unrebound formal of the calling function, which this body names
    /// only through member accesses.
    Formal(String),
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
    /// Card FB-EVENT-LOOP-RETENTION-AND-REGISTRATION: the object a member
    /// path below a formal holds (`coll->evl`), resolved at the call.
    Path(PathRoot, Vec<String>),
    /// Anything else, a caller-supplied holder included.
    Foreign,
}

/// What a recorded retention keys on.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum HeldObject {
    /// A tracked identity: its release or transfer is refused while held.
    Site(usize),
    /// A caller's object: this body cannot release it, so only the exit
    /// check reads it. Card FB-EVENT-LOOP-RETENTION-AND-REGISTRATION: an
    /// interior of a caller's object is named by its dotted path
    /// (`coll.event`).
    Name(String),
    /// Card FB-EVENT-LOOP-RETENTION-AND-REGISTRATION: a member of a tracked
    /// identity. Its release or transfer is refused as the identity's.
    SiteInterior(usize, Vec<String>),
}

impl HeldObject {
    /// The tracked identity the record keeps alive, if any: a release or a
    /// transfer of it while held refuses.
    #[must_use]
    pub fn site(&self) -> Option<usize> {
        match self {
            Self::Site(site) | Self::SiteInterior(site, _) => Some(*site),
            Self::Name(_) => None,
        }
    }
}

/// What holds a recorded retention.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum HeldBy {
    /// An automatic object of this frame: dies at every exit.
    Frame(String),
    /// A tracked identity: holds until reset or released.
    Site(usize),
    /// Card FB-EVENT-LOOP-RETENTION-AND-REGISTRATION: the object at a member
    /// path below a formal; holds until a removal of exactly this record or a
    /// reset of that same path, and outlives the frame.
    Path(PathRoot, Vec<String>),
}

/// Card **RETENTION-UNMATCHED-REMOVAL-POISONS**: the stable reason key of a
/// record a removal through another (or an undecided) holder poisoned, still
/// held at an exit.
pub const POISONED: &str = "retention-removal-poisoned";

/// One recorded retention: what is held, and by what.
pub type Record = (HeldObject, HeldBy);

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
        // Card FB-EVENT-LOOP-RETENTION-AND-REGISTRATION: an interior is held
        // as the identity's member; a path holder records like a tracked one.
        (Retained::TrackedInterior(site, fields), holder) => {
            let object = HeldObject::SiteInterior(*site, fields.clone());
            match holder {
                Holder::Frame(name) => Decision::Record(object, HeldBy::Frame(name.clone())),
                Holder::Tracked(holder) => Decision::Record(object, HeldBy::Site(*holder)),
                Holder::Path(root, fields) => {
                    Decision::Record(object, HeldBy::Path(root.clone(), fields.clone()))
                }
                Holder::Global | Holder::Foreign => Decision::Undecided,
            }
        }
        (Retained::CallerObject(name), Holder::Path(root, fields)) => Decision::Record(
            HeldObject::Name(name.clone()),
            HeldBy::Path(root.clone(), fields.clone()),
        ),
        (Retained::Tracked(site), Holder::Path(root, fields)) => Decision::Record(
            HeldObject::Site(*site),
            HeldBy::Path(root.clone(), fields.clone()),
        ),
    }
}

/// Card **FB-EVENT-LOOP-RETENTION-AND-REGISTRATION** (rule 12t), corrected
/// by card **RETENTION-UNMATCHED-REMOVAL-POISONS**: what a removal names. Each
/// side is `None` when its actual is not one a record keys on.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RemovalKey {
    /// The held object the removal names.
    pub object: Option<HeldObject>,
    /// The holder it removes from.
    pub holder: Option<HeldBy>,
}

/// The removal of `retained` from `holder`, each side decided or not.
#[must_use]
pub fn removal_key(retained: &Retained, holder: &Holder) -> RemovalKey {
    let object = match retained {
        Retained::CallerObject(name) => Some(HeldObject::Name(name.clone())),
        Retained::Tracked(site) => Some(HeldObject::Site(*site)),
        Retained::TrackedInterior(site, fields) => {
            Some(HeldObject::SiteInterior(*site, fields.clone()))
        }
        Retained::Static | Retained::Other => None,
    };
    let holder = match holder {
        Holder::Frame(name) => Some(HeldBy::Frame(name.clone())),
        Holder::Tracked(site) => Some(HeldBy::Site(*site)),
        Holder::Path(root, fields) => Some(HeldBy::Path(root.clone(), fields.clone())),
        Holder::Global | Holder::Foreign => None,
    };
    RemovalKey { object, holder }
}

/// Card **RETENTION-UNMATCHED-REMOVAL-POISONS**: whether a removal naming
/// `object` (`None`: undecided) may clear the status of `held`'s object. Only
/// two distinct tracked identities are known to be distinct objects; a
/// caller's object may be any other caller's object or a tracked one.
#[must_use]
pub fn may_name(object: Option<&HeldObject>, held: &HeldObject) -> bool {
    match object {
        None => true,
        Some(object) => match (object.site(), held.site()) {
            (Some(left), Some(right)) => left == right,
            _ => true,
        },
    }
}

/// Card **RETENTION-UNMATCHED-REMOVAL-POISONS** (rule 12t): one removal over
/// the records. The exactly matching record drops unless it is poisoned;
/// every record still held whose object the removal may name is poisoned.
pub fn remove(
    held: &mut std::collections::BTreeSet<Record>,
    poisoned: &mut std::collections::BTreeSet<Record>,
    key: &RemovalKey,
) {
    if let (Some(object), Some(holder)) = (&key.object, &key.holder) {
        let record = (object.clone(), holder.clone());
        if !poisoned.contains(&record) {
            held.remove(&record);
        }
    }
    for record in held.iter() {
        if may_name(key.object.as_ref(), &record.0) {
            poisoned.insert(record.clone());
        }
    }
}

/// Card **RETENTION-UNMATCHED-REMOVAL-POISONS** (rule 12t): a holder that
/// ends (a loop's destroy, a released tracked holder, a declared reset) holds
/// nothing any more, poisoned records included.
pub fn end_holder(
    held: &mut std::collections::BTreeSet<Record>,
    poisoned: &mut std::collections::BTreeSet<Record>,
    by: &HeldBy,
) {
    held.retain(|(_, holder)| holder != by);
    poisoned.retain(|(_, holder)| holder != by);
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

/// Card **RETENTION-UNMATCHED-REMOVAL-POISONS**: the refusal of an exit at
/// which a poisoned record is still held.
#[must_use]
pub fn poisoned_exit_description(name: &str) -> String {
    format!(
        "a holder retaining `{name}` may outlive this function's frame: a removal through \
         another or an undecided holder cleared its registration status, so no later removal \
         ends it ({POISONED})"
    )
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
    /// Card FB-EVENT-LOOP-RETENTION-AND-REGISTRATION: a removal of one
    /// registration; card RETENTION-UNMATCHED-REMOVAL-POISONS: it poisons
    /// every other record it may name ([`remove`]).
    Remove(RemovalKey),
    /// Card RETENTION-UNMATCHED-REMOVAL-POISONS: the holder's destroy (a
    /// loop's, at any holder): it ends every record held there.
    Destroy(HeldBy),
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
    let mut held: std::collections::BTreeSet<Record> = std::collections::BTreeSet::new();
    let mut poisoned: std::collections::BTreeSet<Record> = std::collections::BTreeSet::new();
    for event in events {
        match event {
            Event::Retain(Decision::Outlives) => {}
            Event::Retain(Decision::Undecided) => return Outcome::Refused,
            Event::Retain(Decision::Record(object, by)) => {
                held.insert((object.clone(), by.clone()));
            }
            Event::Release(site) => {
                if held.iter().any(|(object, _)| object.site() == Some(*site)) {
                    return Outcome::Refused;
                }
                // A released holder holds nothing any more.
                end_holder(&mut held, &mut poisoned, &HeldBy::Site(*site));
            }
            Event::ResetFrame(name) => {
                end_holder(&mut held, &mut poisoned, &HeldBy::Frame(name.clone()));
            }
            Event::ResetSite(site) | Event::ReleaseHolder(site) => {
                end_holder(&mut held, &mut poisoned, &HeldBy::Site(*site));
            }
            Event::Remove(key) => remove(&mut held, &mut poisoned, key),
            Event::Destroy(by) => end_holder(&mut held, &mut poisoned, by),
        }
    }
    // A frame holder dies at the exit; a tracked or path holder still holding
    // outlives the frame.
    if held
        .iter()
        .any(|(_, by)| matches!(by, HeldBy::Site(_) | HeldBy::Path(..)))
    {
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

/// `pathDecisionVectors` of `QCore.RetentionPath`: retained class, holder
/// class, decision, for the classes card FB-EVENT-LOOP-RETENTION-AND-REGISTRATION
/// adds (a path holder; an interior of a tracked identity).
pub const PATH_DECISION_VECTORS: &[(&str, &str, &str)] = &[
    ("static", "path", "outlives"),
    ("caller", "path", "record"),
    ("tracked", "path", "record"),
    ("interior", "path", "record"),
    ("other", "path", "undecided"),
    ("interior", "frame", "record"),
    ("interior", "tracked", "record"),
    ("interior", "global", "undecided"),
    ("interior", "foreign", "undecided"),
];

/// `pathTraceVectors` of `QCore.RetentionPath`: the card's controls as event
/// paths, and whether each is `Safe`.
pub const PATH_TRACE_VECTORS: &[(&str, bool)] = &[
    // A caller's object registered at a path and still held at the exit.
    ("path-held-at-exit", false),
    // Registered at a path, then removed from exactly that path.
    ("path-removed", true),
    // Removed from another path: the record stays.
    ("path-removed-elsewhere", false),
    // A removal whose object is undecided drops nothing.
    ("path-removal-undecided", false),
    // A removal with no record is the identity.
    ("removal-without-record", true),
    // An interior held by a tracked loop; the object released while held.
    ("interior-released-while-held", false),
    // The tracked loop released first (destroyed before deregistration):
    // the record ends with it, and the object's release decides.
    ("loop-destroyed-then-release", true),
    // Two registrations at one path; removing one leaves the other.
    ("one-of-two-removed", false),
    // Card RETENTION-UNMATCHED-REMOVAL-POISONS: the old sequence. Removed
    // through another path (poisoned), then through its own: still held.
    ("unmatched-then-matched", false),
    // The same with a tracked object and a tracked loop, then the object's
    // release: refused (released while held).
    ("unmatched-matched-release", false),
    // Removed through its own loop, then released: decides.
    ("matched-then-release", true),
    // The loop destroyed, then the object released: decides.
    ("destroy-then-release", true),
    // Poisoned, then its loop destroyed, then released: decides.
    ("poisoned-destroyed-release", true),
    // A path record poisoned, then the path's loop destroyed: decides.
    ("poisoned-path-destroyed", true),
    // A removal whose holder is undecided poisons the record of its object.
    ("undecided-holder-poisons", false),
    // A removal whose object is undecided poisons every record.
    ("undecided-object-poisons", false),
    // A removal of another tracked identity poisons nothing.
    ("other-identity-untouched", true),
];

/// The event path a [`PATH_TRACE_VECTORS`] row names.
#[must_use]
pub fn path_trace(name: &str) -> Vec<Event> {
    let path = || Holder::Path(PathRoot::Formal("coll".into()), vec!["evl".into()]);
    let other = || Holder::Path(PathRoot::Formal("config".into()), vec!["evl".into()]);
    let caller = || Retained::CallerObject("coll.event".into());
    let interior = || Retained::TrackedInterior(1, vec!["event".into()]);
    match name {
        "path-held-at-exit" => vec![Event::Retain(decide(&caller(), &path()))],
        "path-removed" => vec![
            Event::Retain(decide(&caller(), &path())),
            Event::Remove(removal_key(&caller(), &path())),
        ],
        "path-removed-elsewhere" => vec![
            Event::Retain(decide(&caller(), &path())),
            Event::Remove(removal_key(&caller(), &other())),
        ],
        "path-removal-undecided" => vec![
            Event::Retain(decide(&caller(), &path())),
            Event::Remove(removal_key(&Retained::Other, &path())),
        ],
        "removal-without-record" => vec![Event::Remove(removal_key(&caller(), &path()))],
        "interior-released-while-held" => vec![
            Event::Retain(decide(&interior(), &Holder::Tracked(2))),
            Event::Release(1),
            Event::ReleaseHolder(2),
        ],
        "loop-destroyed-then-release" => vec![
            Event::Retain(decide(&interior(), &Holder::Tracked(2))),
            Event::ReleaseHolder(2),
            Event::Release(1),
        ],
        "one-of-two-removed" => vec![
            Event::Retain(decide(&caller(), &path())),
            Event::Retain(decide(
                &Retained::CallerObject("coll.timer".into()),
                &path(),
            )),
            Event::Remove(removal_key(&caller(), &path())),
        ],
        "unmatched-then-matched" => vec![
            Event::Retain(decide(&caller(), &path())),
            Event::Remove(removal_key(&caller(), &other())),
            Event::Remove(removal_key(&caller(), &path())),
        ],
        "unmatched-matched-release" => vec![
            Event::Retain(decide(&interior(), &Holder::Tracked(2))),
            Event::Remove(removal_key(&interior(), &Holder::Tracked(3))),
            Event::Remove(removal_key(&interior(), &Holder::Tracked(2))),
            Event::Release(1),
            Event::ReleaseHolder(2),
            Event::ReleaseHolder(3),
        ],
        "matched-then-release" => vec![
            Event::Retain(decide(&interior(), &Holder::Tracked(2))),
            Event::Remove(removal_key(&interior(), &Holder::Tracked(2))),
            Event::Release(1),
            Event::ReleaseHolder(2),
        ],
        "destroy-then-release" => vec![
            Event::Retain(decide(&interior(), &Holder::Tracked(2))),
            Event::Destroy(HeldBy::Site(2)),
            Event::Release(1),
        ],
        "poisoned-destroyed-release" => vec![
            Event::Retain(decide(&interior(), &Holder::Tracked(2))),
            Event::Remove(removal_key(&interior(), &Holder::Tracked(3))),
            Event::Destroy(HeldBy::Site(2)),
            Event::Release(1),
            Event::ReleaseHolder(3),
        ],
        "poisoned-path-destroyed" => vec![
            Event::Retain(decide(&caller(), &path())),
            Event::Remove(removal_key(&caller(), &other())),
            Event::Destroy(HeldBy::Path(
                PathRoot::Formal("coll".into()),
                vec!["evl".into()],
            )),
        ],
        "undecided-holder-poisons" => vec![
            Event::Retain(decide(&caller(), &path())),
            Event::Remove(removal_key(&caller(), &Holder::Foreign)),
            Event::Remove(removal_key(&caller(), &path())),
        ],
        "undecided-object-poisons" => vec![
            Event::Retain(decide(&caller(), &path())),
            Event::Remove(removal_key(&Retained::Other, &path())),
            Event::Remove(removal_key(&caller(), &path())),
        ],
        "other-identity-untouched" => vec![
            Event::Retain(decide(&interior(), &Holder::Tracked(2))),
            Event::Remove(removal_key(
                &Retained::TrackedInterior(4, vec!["event".into()]),
                &Holder::Tracked(3),
            )),
            Event::Remove(removal_key(&interior(), &Holder::Tracked(2))),
            Event::Release(1),
        ],
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
    fn path_vectors_bind_the_table_and_the_replay() {
        let retained = |name: &str| match name {
            "static" => Retained::Static,
            "caller" => Retained::CallerObject("p".into()),
            "tracked" => Retained::Tracked(1),
            "interior" => Retained::TrackedInterior(1, vec!["event".into()]),
            _ => Retained::Other,
        };
        let holder = |name: &str| match name {
            "path" => Holder::Path(PathRoot::Formal("h".into()), vec!["evl".into()]),
            "frame" => Holder::Frame("L".into()),
            "tracked" => Holder::Tracked(2),
            "global" => Holder::Global,
            _ => Holder::Foreign,
        };
        for (p, h, expected) in PATH_DECISION_VECTORS {
            let label = match decide(&retained(p), &holder(h)) {
                Decision::Outlives => "outlives",
                Decision::Record(..) => "record",
                Decision::Undecided => "undecided",
            };
            assert_eq!(label, *expected, "{p} {h}");
        }
        for (name, safe) in PATH_TRACE_VECTORS {
            assert!(!path_trace(name).is_empty(), "{name}");
            assert_eq!(replay(&path_trace(name)) == Outcome::Safe, *safe, "{name}");
        }
    }

    #[test]
    fn minted_names_round_trip() {
        assert_eq!(retained_callee(&retained_name("f")), Some("f"));
        assert_eq!(retained_callee("f"), None);
    }
}
