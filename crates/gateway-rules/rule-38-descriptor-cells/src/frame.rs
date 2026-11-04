//! Gateway rule 38g (card CELL-PROTOCOL-RELEASE-WINDOW-FRAMES, grammar G05k,
//! contract specification §C04.6.4): callee frames over a record's declared
//! cell protocol.
//!
//! USER DECISION 2026-11-04 (verbatim, on the options (a) callee frame
//! declarations, (b) a member-actual narrowing, (c) keep the refusal): "(a)
//! Callee frames". A call in the window between an end's close and the
//! holder's release admits only where a declared or body-checked clause
//! states that the callee reads no end member of the record; (b) is not
//! chosen (unsound under `container_of`), and a call without such a clause
//! keeps refusing.
//!
//! **The clause.** `quant cell CALLEE: TAG |-> #untouched;` states that
//! `CALLEE`, with everything it calls, neither reads nor writes any member a
//! declared protocol of `struct TAG` names (each cell's and each guard's
//! first member), on every object of that type, however the callee reaches
//! it. It is a frame over the record's type, never over the call's actuals:
//! a callee handed `&ins->_head` may recover `ins` by `container_of`, and the
//! clause still covers it.
//!
//! **Its standing in a unit** ([`body_check`], [`standing`]). Where the unit
//! defines the callee's body and every body it calls, none names a protocol
//! member of the record, and none uses a construct the host's reader does not
//! read (a whole-record value, a pointer reinterpreted as a scalar or byte
//! pointer, `offsetof`, a call by pointer or out of the unit, recursion), the
//! frame is **checked**. A body that names a protocol member **refutes** it.
//! Otherwise it is **declared** only: a deferred-review contract, held for a
//! human act (`awaiting-human-review`, `Pending`), never a premise and never
//! `Safe`; the analysis with every held declaration in force (C06.1) reads it
//! **accepted**.
//!
//! **Its reading** ([`window`]). Only where a call that may reach a record
//! would refuse because the record does not keep its protocol (rule 38c's
//! `descriptor-cell-protocol-broken`: the release window, an end closed and
//! its sentinel not stored): a checked or accepted frame passes the record's
//! fact through the call unchanged; a declared one passes it and holds the
//! call; a refuted one refuses (`descriptor-cell-frame-refuted`); none
//! refuses as before. A record that keeps its protocol is read as rule 38c
//! reads it, whatever its frames. The Lean model is
//! `QCore.CellProtocolReleaseWindowFrames`.

use std::collections::{BTreeMap, BTreeSet};

use crate::cells::{Cells, Refusal};
use crate::protocol::{Fact, Protocol, adopt, per_end_word, table_of, vector_spec};

/// What a unit's bodies say of one callee's frame over one record.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum BodyCheck {
    /// Every body reached is in the unit, read in full, and names no
    /// protocol member of the record.
    Established,
    /// Some body reached is missing, recursive, or uses a construct the
    /// reader does not read, and none names a protocol member.
    Unestablished,
    /// Some body reached names a protocol member of the record.
    Refuted,
}

/// A frame's standing at one call, in the order [`join`] reads.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Standing {
    /// Declared and established by the unit's bodies.
    Checked,
    /// Declared, not established, and accepted by the human act the second
    /// analysis models (C06.1); never in a published first analysis.
    Accepted,
    /// Declared, not established here: held for human review.
    Declared,
    /// No frame of the callee over the record is declared.
    Absent,
    /// Declared and refuted by a body the unit analyses.
    Refuted,
}

/// The standing of a frame: whether one is declared, what the unit's bodies
/// say, and whether the human act is in force. A refutation is never lifted
/// by acceptance.
#[must_use]
pub const fn standing(declared: bool, check: BodyCheck, accepted: bool) -> Standing {
    if !declared {
        return Standing::Absent;
    }
    match check {
        BodyCheck::Established => Standing::Checked,
        BodyCheck::Refuted => Standing::Refuted,
        BodyCheck::Unestablished if accepted => Standing::Accepted,
        BodyCheck::Unestablished => Standing::Declared,
    }
}

/// The standing of a call that runs several callees (a call with nested
/// calls in its actuals): the weakest of theirs.
#[must_use]
pub fn join(standings: impl IntoIterator<Item = Standing>) -> Standing {
    standings.into_iter().max().unwrap_or(Standing::Checked)
}

/// What a call that may reach a record whose fact does not keep its
/// protocol makes of a frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Reading {
    /// The fact passes the call unchanged.
    Pass,
    /// The fact passes the call unchanged and the call is held for human
    /// review (`awaiting-human-review`).
    Held,
    /// The call refuses.
    Refused(Refusal),
}

/// Rule 38g's reading of a frame in the release window.
#[must_use]
pub const fn window(standing: Standing) -> Reading {
    match standing {
        Standing::Checked | Standing::Accepted => Reading::Pass,
        Standing::Declared => Reading::Held,
        Standing::Absent => Reading::Refused(Refusal::ProtocolBroken),
        Standing::Refuted => Reading::Refused(Refusal::FrameRefuted),
    }
}

/// The members a record's declared protocols name: the first member of every
/// cell path and every guard, without its index (`ch_events[0]` names
/// `ch_events`, `event.fd` names `event`).
#[must_use]
pub fn protocol_members<'a>(protocols: impl IntoIterator<Item = &'a Protocol>) -> BTreeSet<String> {
    let first = |path: &str| {
        let head = path.split('.').next().unwrap_or(path);
        head.split('[').next().unwrap_or(head).to_owned()
    };
    protocols
        .into_iter()
        .flat_map(|protocol| {
            protocol
                .cells
                .iter()
                .map(|(cell, _)| first(cell))
                .chain(std::iter::once(first(&protocol.guard)))
                .collect::<Vec<_>>()
        })
        .collect()
}

/// One function body as the host reads it from Clang's AST of the unit.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Body {
    /// Every member access: the record type's tag and the member's name.
    pub accesses: Vec<(String, String)>,
    /// Every direct callee, by name.
    pub calls: Vec<String>,
    /// Every construct the reader does not read, with what it is.
    pub unread: Vec<String>,
}

/// What `callee`'s bodies, as `bodies` reads them in one unit, say of its
/// frame over `record`'s protocol `members`: refuted where any body reached
/// names a member of `record`, else established where every body reached is
/// present, read in full and not recursive, else unestablished.
#[must_use]
pub fn body_check(
    bodies: &impl Fn(&str) -> Option<Body>,
    callee: &str,
    record: &str,
    members: &BTreeSet<String>,
) -> BodyCheck {
    fn visit(
        bodies: &impl Fn(&str) -> Option<Body>,
        name: &str,
        record: &str,
        members: &BTreeSet<String>,
        stack: &mut Vec<String>,
        done: &mut BTreeMap<String, BodyCheck>,
    ) -> BodyCheck {
        if let Some(found) = done.get(name) {
            return *found;
        }
        if stack.iter().any(|on| on == name) {
            // Recursion: no body is checked through itself.
            return BodyCheck::Unestablished;
        }
        let Some(body) = bodies(name) else {
            return BodyCheck::Unestablished;
        };
        let mut found = if body
            .accesses
            .iter()
            .any(|(tag, member)| tag == record && members.contains(member))
        {
            BodyCheck::Refuted
        } else if body.unread.is_empty() {
            BodyCheck::Established
        } else {
            BodyCheck::Unestablished
        };
        stack.push(name.to_owned());
        for call in &body.calls {
            found = found.max(visit(bodies, call, record, members, stack, done));
        }
        stack.pop();
        if stack.is_empty() || found != BodyCheck::Unestablished {
            done.insert(name.to_owned(), found);
        }
        found
    }
    visit(
        bodies,
        callee,
        record,
        members,
        &mut Vec::new(),
        &mut BTreeMap::new(),
    )
}

/// `QCore.CellProtocolReleaseWindowFrames.bodyVectors`: bodies spelled
/// `NAME = ITEM, ITEM; NAME = …` (the first body is the callee), each item a
/// member access `TAG.MEMBER`, a call `->NAME` or an unread construct `?`,
/// checked for the frame over `rec`, whose protocol members are `ch` and
/// `event`.
pub const BODY_VECTORS: &[(&str, &str)] = &[
    ("f = mk_list.prev, ->g; g = mk_list.next", "established"),
    ("f = rec.ch", "refuted"),
    ("f = rec.flags", "established"),
    ("f = other.ch", "established"),
    ("f = ->g", "unestablished"),
    ("f = ->f", "unestablished"),
    ("f = ?", "unestablished"),
    ("f = ?, ->g; g = rec.event", "refuted"),
    ("f = ->g, ->h; g = rec.flags; h = ->g", "established"),
    ("f = ->g; g = ->f", "unestablished"),
    ("f = ->g; g = ->f, rec.ch", "refuted"),
];

/// Run one body vector; the outcome word.
#[must_use]
pub fn run_body_vector(trace: &str) -> &'static str {
    let mut bodies: BTreeMap<String, Body> = BTreeMap::new();
    let mut first = None;
    for part in trace.split(';') {
        let Some((name, items)) = part.split_once('=') else {
            return "malformed";
        };
        let name = name.trim().to_owned();
        let mut body = Body::default();
        for item in items.split(',').map(str::trim) {
            if item == "?" {
                body.unread.push("?".to_owned());
            } else if let Some(call) = item.strip_prefix("->") {
                body.calls.push(call.to_owned());
            } else if let Some((tag, member)) = item.split_once('.') {
                body.accesses.push((tag.to_owned(), member.to_owned()));
            } else {
                return "malformed";
            }
        }
        first.get_or_insert_with(|| name.clone());
        bodies.insert(name, body);
    }
    let Some(first) = first else {
        return "malformed";
    };
    let members = BTreeSet::from(["ch".to_owned(), "event".to_owned()]);
    match body_check(&|name| bodies.get(name).cloned(), &first, "rec", &members) {
        BodyCheck::Established => "established",
        BodyCheck::Unestablished => "unestablished",
        BodyCheck::Refuted => "refuted",
    }
}

/// `QCore.CellProtocolReleaseWindowFrames.windowVectors`: rule 38f's per-end
/// traces (`protocol::PER_END_SET_VECTORS`' spelling) with one more step,
/// `call STANDING` (`checked`, `accepted`, `declared`, `refuted`, `none`): a
/// call that may reach the record, whose callee's frame over it stands so.
/// `held` is a trace that keeps the protocol (or ends it at the release)
/// with a call held for review on the way.
pub const WINDOW_VECTORS: &[(&str, &str)] = &[
    ("pos/L1/A0: test live; close c; call checked; free", "holds"),
    (
        "pos/L1/A0: test live; close c; call accepted; free",
        "holds",
    ),
    ("pos/L1/A0: test live; close c; call declared; free", "held"),
    (
        "pos/L1/A0: test live; close c; call refuted",
        "frame-refuted",
    ),
    ("pos/L1/A0: test live; close c; call none", "broken"),
    ("pos/L1/A0: test live; close c; call none; free", "broken"),
    ("pos/L1/A0: test live; call none", "holds"),
    ("pos/L1/A0: test live; call refuted", "holds"),
    ("pos/L1/A0: test live; close c; call checked", "broken"),
    (
        "pos/L1/A0: test live; close c; call checked; close c",
        "double-release",
    ),
    (
        "pos/L1/A0: test live; close c; call checked; store c 0",
        "holds",
    ),
    (
        "pos/L1/A0: test live; close c; call declared; call checked; free",
        "held",
    ),
    (
        "pos/L1/A0: test live; close c; call declared; call none",
        "broken",
    ),
    (
        "pos/L0: test live; close c; join; call checked; free",
        "zero-undecided",
    ),
    ("pos/L1/A0: test live; free", "owner-released"),
    ("pos/L1/A0: test live; call checked; free", "holds"),
];

/// Run one window vector; the outcome word.
#[must_use]
pub fn run_window_vector(trace: &str) -> &'static str {
    let Some((spec, trace)) = trace.split_once(':') else {
        return "malformed";
    };
    let Some(protocol) = vector_spec(spec) else {
        return "malformed";
    };
    let mut cells: Cells<String> = Cells::default();
    let mut fact: Option<Fact> = None;
    let mut live_edge: Option<bool> = None;
    let mut held = false;
    for step in trace.split(';').map(str::trim) {
        let outcome: Result<(), Refusal> =
            match step.split_whitespace().collect::<Vec<_>>().as_slice() {
                ["test", edge] => {
                    let live = *edge == "live";
                    live_edge = Some(live);
                    fact = Some(Fact::introduced(&protocol, live, 100));
                    Ok(())
                }
                ["close", cell] => {
                    let fact = fact.get_or_insert_with(|| Fact::reached(&protocol));
                    adopt(&mut cells, fact, cell);
                    let key = (*cell).to_owned();
                    match cells.get(&key) {
                        None => Err(Refusal::Undecided),
                        Some((identity, _)) => {
                            let released = cells.release(Some(&key), |_| true).map(|_| ());
                            if released.is_ok() {
                                fact.stale_identity(identity);
                            }
                            released
                        }
                    }
                }
                ["store", cell, value] => {
                    let fact = fact.get_or_insert_with(|| Fact::reached(&protocol));
                    fact.store(cell, value.parse::<i64>().ok(), table_of(&cells))
                }
                ["join"] => {
                    let (Some(current), Some(live)) = (fact.as_mut(), live_edge) else {
                        return "malformed";
                    };
                    let mut other = Fact::introduced(&protocol, !live, 100);
                    let mut other_cells: Cells<String> = Cells::default();
                    let keys = protocol
                        .cells
                        .iter()
                        .filter(|(cell, _)| cells.get(cell).is_some())
                        .map(|(cell, _)| (cell.clone(), cell.clone()))
                        .collect::<BTreeMap<_, _>>();
                    current.normalize(&mut cells, &keys);
                    other.normalize(&mut other_cells, &keys);
                    *current = current.join(&other);
                    cells.join_keeping_stale(&other_cells, |_| true);
                    Ok(())
                }
                ["call"] => call(&mut fact, &mut cells, Standing::Absent, &mut held),
                ["call", word] => {
                    let Some(standing) = standing_word(word) else {
                        return "malformed";
                    };
                    call(&mut fact, &mut cells, standing, &mut held)
                }
                ["free"] => fact
                    .take()
                    .map_or(Ok(()), |current| current.release_holder(table_of(&cells))),
                _ => return "malformed",
            };
        if let Err(refusal) = outcome {
            return window_word(refusal);
        }
    }
    match &fact {
        Some(fact) if !fact.holds(table_of(&cells)) => "broken",
        _ if held => "held",
        _ => "holds",
    }
}

/// One call that may reach the record: rule 38c's forward where the record
/// keeps its protocol, else rule 38g's reading of the frame.
fn call(
    fact: &mut Option<Fact>,
    cells: &mut Cells<String>,
    standing: Standing,
    held: &mut bool,
) -> Result<(), Refusal> {
    match fact.take() {
        Some(current) if !current.holds(table_of(cells)) => match window(standing) {
            Reading::Pass => {
                *fact = Some(current);
                Ok(())
            }
            Reading::Held => {
                *held = true;
                *fact = Some(current);
                Ok(())
            }
            Reading::Refused(refusal) => Err(refusal),
        },
        // Forwarded: read again from the protocol after the call.
        _ => {
            *cells = Cells::default();
            Ok(())
        }
    }
}

fn standing_word(word: &str) -> Option<Standing> {
    Some(match word {
        "checked" => Standing::Checked,
        "accepted" => Standing::Accepted,
        "declared" => Standing::Declared,
        "refuted" => Standing::Refuted,
        "none" => Standing::Absent,
        _ => return None,
    })
}

const fn window_word(refusal: Refusal) -> &'static str {
    match refusal {
        Refusal::FrameRefuted => "frame-refuted",
        other => per_end_word(other),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_body_vectors_hold() {
        for (trace, expected) in BODY_VECTORS {
            assert_eq!(run_body_vector(trace), *expected, "{trace}");
        }
    }

    #[test]
    fn the_window_vectors_hold() {
        for (trace, expected) in WINDOW_VECTORS {
            assert_eq!(run_window_vector(trace), *expected, "{trace}");
        }
    }

    /// Only a checked or accepted frame passes unheld; acceptance never
    /// lifts a refutation; no declaration, no frame.
    #[test]
    fn only_a_checked_or_accepted_frame_passes() {
        use BodyCheck::{Established, Refuted, Unestablished};
        for check in [Established, Unestablished, Refuted] {
            for accepted in [false, true] {
                assert_eq!(standing(false, check, accepted), Standing::Absent);
                let reading = window(standing(true, check, accepted));
                assert_eq!(
                    reading == Reading::Pass,
                    check == Established || (check == Unestablished && accepted),
                    "{check:?} {accepted}"
                );
                if check == Refuted {
                    assert_eq!(reading, Reading::Refused(Refusal::FrameRefuted));
                }
            }
        }
        assert_eq!(
            window(Standing::Absent),
            Reading::Refused(Refusal::ProtocolBroken)
        );
        assert_eq!(
            join([Standing::Checked, Standing::Declared]),
            Standing::Declared
        );
        assert_eq!(
            join([Standing::Declared, Standing::Absent]),
            Standing::Absent
        );
        assert_eq!(join([]), Standing::Checked);
    }

    /// The members are the first member of every cell and guard.
    #[test]
    fn the_protocol_members_are_every_cell_and_guard_head() {
        let mut thread = crate::protocol::per_end_protocol(crate::protocol::Guard::EqualOne);
        thread.cells = vec![
            (
                "notification_channels[0]".to_owned(),
                "notification_channels[0]".to_owned(),
            ),
            (
                "notification_event.fd".to_owned(),
                "notification_channels[0]".to_owned(),
            ),
        ];
        thread.guard = "notification_channels_initialized".to_owned();
        assert_eq!(
            protocol_members([&thread]).into_iter().collect::<Vec<_>>(),
            [
                "notification_channels",
                "notification_channels_initialized",
                "notification_event"
            ]
        );
    }

    /// The Lean vectors are the Rust ones, entry for entry, each Rust trace
    /// spelled in the Lean model's form.
    #[test]
    fn the_vectors_are_the_lean_ones() {
        const LEAN: &str =
            include_str!("../../../../proof/QCore/CellProtocolReleaseWindowFrames.lean");
        let block = |name: &str| {
            LEAN.split(&format!("def {name}"))
                .nth(1)
                .and_then(|rest| rest.split("\n]").next())
                .expect("the Lean vectors")
                .lines()
                .map(str::trim)
                .filter(|line| line.starts_with('('))
                .map(|line| line.trim_end_matches(',').to_owned())
                .collect::<Vec<_>>()
        };
        let index = |table: &[&str], word: &str| {
            table
                .iter()
                .position(|known| *known == word)
                .unwrap_or_else(|| panic!("{word}"))
        };
        let functions = ["f", "g", "h"];
        let tags = ["rec", "other", "mk_list"];
        let members = ["ch", "event", "flags", "prev", "next"];
        let bodies = BODY_VECTORS
            .iter()
            .map(|(trace, outcome)| {
                let spelled = trace
                    .split(';')
                    .map(|part| {
                        let (name, items) = part.split_once('=').expect("a body");
                        let (mut accesses, mut calls, mut unread) = (Vec::new(), Vec::new(), false);
                        for item in items.split(',').map(str::trim) {
                            if item == "?" {
                                unread = true;
                            } else if let Some(call) = item.strip_prefix("->") {
                                calls.push(index(&functions, call).to_string());
                            } else {
                                let (tag, member) = item.split_once('.').expect("an access");
                                accesses.push(format!(
                                    "({}, {})",
                                    index(&tags, tag),
                                    index(&members, member)
                                ));
                            }
                        }
                        format!(
                            "({}, ⟨[{}], [{}], {unread}⟩)",
                            index(&functions, name.trim()),
                            accesses.join(", "),
                            calls.join(", ")
                        )
                    })
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("([{spelled}], .{outcome})")
            })
            .collect::<Vec<_>>();
        assert_eq!(block("bodyVectors"), bodies);
        let windows = WINDOW_VECTORS
            .iter()
            .map(|(trace, outcome)| {
                let (spec, steps) = trace.split_once(':').expect("a spec");
                let spec = match spec {
                    "pos/L1/A0" => "⟨.positive, some fromOne, some absZero⟩",
                    "pos/L0" => "⟨.positive, some fromZero, none⟩",
                    other => panic!("{other}"),
                };
                let steps = steps
                    .split(';')
                    .map(
                        |step| match step.split_whitespace().collect::<Vec<_>>().as_slice() {
                            ["test", "live"] => ".s .testLive".to_owned(),
                            ["test", "dead"] => ".s .testDead".to_owned(),
                            ["close", "c"] => ".s (.close 0)".to_owned(),
                            ["store", "c", value] => format!(".s (.store 0 {value})"),
                            ["call", "none"] => ".f .absent".to_owned(),
                            ["call", standing] => format!(".f .{standing}"),
                            [word] => format!(".s .{word}"),
                            other => panic!("{other:?}"),
                        },
                    )
                    .collect::<Vec<_>>()
                    .join(", ");
                let outcome = match *outcome {
                    "frame-refuted" => "frameRefuted",
                    "double-release" => "doubleRelease",
                    "owner-released" => "ownerReleased",
                    "zero-undecided" => "zeroUndecided",
                    other => other,
                };
                format!("({spec}, [{steps}], .{outcome})")
            })
            .collect::<Vec<_>>();
        assert_eq!(block("windowVectors"), windows);
    }
}
