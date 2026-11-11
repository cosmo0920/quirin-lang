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

//! S3 lifetime summaries. May registrations are retained across calls; must
//! endings are intersected across exits. A released object with an unended
//! registration forwards an obligation instead of becoming Safe by omission.
//! This is an abstract model; source extraction and run closure are separate.
use crate::loop_lifetime_source::{OperationKind, SourceFacts};
use qcore_schemas::loop_lifetime::RegisteredProtocol;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub use qcore_schemas::loop_lifetime::{ForwardedLifetime, Path, Registration, Root};
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Ending {
    Remove(Path, Path),
    Destroy(Path),
}
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Requirement {
    pub object: Path,
    pub holder: Path,
    pub reason: String,
}
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ReleaseEffect {
    pub object: Path,
    /// Concrete holders destroyed before this particular release, on every
    /// represented path. Later postconditions cannot discharge this effect.
    pub destroyed_before: BTreeSet<Path>,
}
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Summary {
    /// Complete bounded trace; absence grants no ordered discharge.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ordered: Option<Vec<OrderedStep>>,
    pub may_register: BTreeSet<Registration>,
    pub may_release: BTreeSet<ReleaseEffect>,
    pub must_end: BTreeSet<Ending>,
    pub requirements: BTreeSet<Requirement>,
    pub open: BTreeSet<String>,
    /// A summary preserves possible poisoning of records its caller supplied.
    pub may_poison: BTreeSet<Path>,
    pub poison_all: bool,
}
/// Primitive steps carry order across a call without guessing from must-end sets.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum OrderedStep {
    Register(Registration),
    Remove(Option<Path>, Option<Path>),
    Destroy(Path),
    Release(Path),
}
impl OrderedStep {
    fn event(&self) -> Event {
        match self {
            Self::Register(r) => Event::Register(r.clone()),
            Self::Remove(o, h) => Event::Remove(o.clone(), h.clone()),
            Self::Destroy(h) => Event::Destroy(h.clone()),
            Self::Release(o) => Event::Release(o.clone()),
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Event {
    Register(Registration),
    Remove(Option<Path>, Option<Path>),
    Destroy(Path),
    Release(Path),
    Call(Summary),
}

fn may_name(_actual: Option<&Path>, _record: &Path) -> bool {
    // Formal and result values can alias, including results of different
    // functions. No freshness/disjointness evidence is carried here.
    true
}
fn concrete(path: &Path) -> bool {
    !matches!(path.root, Root::Lifted(_))
}

fn remove(rs: &mut BTreeSet<Registration>, object: Option<&Path>, holder: Option<&Path>) {
    *rs = rs
        .iter()
        .filter_map(|r| {
            if !r.poisoned
                && concrete(&r.object)
                && concrete(&r.holder)
                && object == Some(&r.object)
                && holder == Some(&r.holder)
            {
                None
            } else {
                let mut kept = r.clone();
                if may_name(object, &r.object) {
                    kept.poisoned = true;
                }
                Some(kept)
            }
        })
        .collect();
}
fn end(rs: &mut BTreeSet<Registration>, ending: &Ending) {
    match ending {
        Ending::Remove(object, holder) => remove(rs, Some(object), Some(holder)),
        Ending::Destroy(holder) => {
            if concrete(holder) {
                rs.retain(|r| r.holder != *holder);
            }
        }
    }
}
/// One complete abstract path. An equality is usable only after the source
/// reader canonicalizes both paths using a checked derivation; this model
/// never equates distinct paths merely because their candidates overlap.
#[must_use]
pub fn path(events: &[Event]) -> Summary {
    // A differing/unknown exit trace uses the old conservative may/must fold.
    // Bounds are admission limits, never truncation of a trace.
    let mut trace = Vec::new();
    let mut inherited = BTreeSet::new();
    for event in events {
        let step = match event {
            Event::Register(r) => OrderedStep::Register(r.clone()),
            Event::Remove(o, h) => OrderedStep::Remove(o.clone(), h.clone()),
            Event::Destroy(h) => OrderedStep::Destroy(h.clone()),
            Event::Release(o) => OrderedStep::Release(o.clone()),
            Event::Call(summary) => {
                let Some(steps) = &summary.ordered else {
                    return unordered_path(events);
                };
                if !summary.open.is_empty() {
                    return unordered_path(events);
                }
                inherited.extend(summary.requirements.iter().cloned());
                if trace.len().saturating_add(steps.len()) > 4096 {
                    let mut out = unordered_path(events);
                    out.open
                        .insert("ordered lifetime trace exceeds 4096 steps".into());
                    return out;
                }
                trace.extend(steps.iter().cloned());
                continue;
            }
        };
        if trace.len() == 4096 {
            let mut out = unordered_path(events);
            out.open
                .insert("ordered lifetime trace exceeds 4096 steps".into());
            return out;
        }
        trace.push(step);
    }
    let mut out = unordered_path(&trace.iter().map(OrderedStep::event).collect::<Vec<_>>());
    out.requirements.extend(inherited);
    out.ordered = Some(trace);
    out
}

fn unordered_path(events: &[Event]) -> Summary {
    let mut out = Summary::default();
    for e in events {
        match e {
            Event::Register(r) => {
                out.must_end.remove(&Ending::Destroy(r.holder.clone()));
                out.may_register.insert(r.clone());
            }
            Event::Remove(object, holder) => {
                if let Some(object) = object {
                    out.may_poison.insert(object.clone());
                } else {
                    out.poison_all = true;
                }
                remove(&mut out.may_register, object.as_ref(), holder.as_ref());
                if let (Some(object), Some(holder)) = (object, holder) {
                    out.must_end
                        .insert(Ending::Remove(object.clone(), holder.clone()));
                }
            }
            Event::Destroy(holder) => {
                if !concrete(holder) {
                    out.open.insert(
                        "a destroy of one unresolved holder cannot end a whole footprint".into(),
                    );
                    continue;
                }
                let ending = Ending::Destroy(holder.clone());
                end(&mut out.may_register, &ending);
                out.must_end.insert(ending);
            }
            Event::Release(object) => {
                out.may_release.insert(ReleaseEffect {
                    object: object.clone(),
                    destroyed_before: out
                        .must_end
                        .iter()
                        .filter_map(|e| match e {
                            Ending::Destroy(p) if concrete(p) => Some(p.clone()),
                            _ => None,
                        })
                        .collect(),
                });
                for r in &out.may_register {
                    if may_name(Some(object), &r.object) {
                        out.requirements.insert(Requirement {
                            object: r.object.clone(),
                            holder: r.holder.clone(),
                            reason: if r.poisoned {
                                "retention-removal-poisoned"
                            } else {
                                "lifetime-precondition-undischarged"
                            }
                            .into(),
                        });
                    }
                }
            }
            Event::Call(summary) => {
                // Must endings happen before the callee's surviving may records.
                // A callee's release precondition cannot be discharged using one
                // of that same callee's postconditions.
                for release in &summary.may_release {
                    let mut composed = release.clone();
                    for ending in &out.must_end {
                        if let Ending::Destroy(holder) = ending
                            && concrete(holder)
                        {
                            composed.destroyed_before.insert(holder.clone());
                        }
                    }
                    for record in &out.may_register {
                        if may_name(Some(&release.object), &record.object)
                            && !(concrete(&record.holder)
                                && release.destroyed_before.contains(&record.holder))
                        {
                            out.requirements.insert(Requirement {
                                object: record.object.clone(), holder: record.holder.clone(),
                                reason: "lifetime-precondition-undischarged: callee may release an incoming registration before its holder ends".into(),
                            });
                        }
                    }
                    out.may_release.insert(composed);
                }
                out.requirements
                    .extend(summary.requirements.iter().cloned());
                out.open.extend(summary.open.iter().cloned());
                out.may_register = out
                    .may_register
                    .iter()
                    .map(|r| {
                        let mut kept = r.clone();
                        if summary.poison_all
                            || summary
                                .may_poison
                                .iter()
                                .any(|p| may_name(Some(p), &r.object))
                        {
                            kept.poisoned = true;
                        }
                        kept
                    })
                    .collect();
                out.may_poison.extend(summary.may_poison.iter().cloned());
                out.poison_all |= summary.poison_all;
                // Removal order and incoming poison cannot be recovered from a
                // must-ending set. Only destroys discharge across this abstraction;
                // exact removals stay within their ordered in-body trace.
                for ending in &summary.must_end {
                    if matches!(ending, Ending::Destroy(_)) {
                        end(&mut out.may_register, ending);
                        out.must_end.insert(ending.clone());
                    }
                }
                for record in &summary.may_register {
                    // Just as for a local Register, a new registration after
                    // a destroy makes that earlier ending insufficient.
                    out.must_end.remove(&Ending::Destroy(record.holder.clone()));
                    out.may_register.insert(record.clone());
                }
            }
        }
    }
    out
}
/// Join every exit. Empty or unread exits do not prove a must-ending.
#[must_use]
pub fn join(paths: &[Summary]) -> Summary {
    let Some(first) = paths.first() else {
        return Summary {
            open: BTreeSet::from(["no complete lifetime exit".into()]),
            ..Default::default()
        };
    };
    let mut out = first.clone();
    for p in &paths[1..] {
        if out.ordered != p.ordered {
            out.ordered = None;
        }
        out.may_register.extend(p.may_register.iter().cloned());
        out.may_release.extend(p.may_release.iter().cloned());
        out.requirements.extend(p.requirements.iter().cloned());
        out.open.extend(p.open.iter().cloned());
        out.may_poison.extend(p.may_poison.iter().cloned());
        out.poison_all |= p.poison_all;
        out.must_end = out.must_end.intersection(&p.must_end).cloned().collect();
    }
    out
}
fn substitute_path(
    path: &Path,
    callee_scope: &str,
    actuals: &[Option<Path>],
    result: Option<&Path>,
    footprint: Option<&str>,
) -> Option<Path> {
    let replacement = match &path.root {
        Root::Formal { scope, index } if scope == callee_scope => actuals.get(*index)?.clone(),
        Root::Result { scope } if scope == callee_scope => result.cloned(),
        Root::Formal { .. } | Root::Result { .. } => return None,
        Root::Lifted(_) => Some(path.clone()),
    };
    let mut replacement = match replacement {
        Some(p) => p,
        None => Path {
            root: Root::Lifted(footprint?.into()),
            fields: Vec::new(),
        },
    };
    // A lifted root already denotes the whole closed footprint; member suffix
    // remains explicit, rather than identifying unrelated runtime objects.
    if !matches!(path.root, Root::Lifted(_)) {
        replacement.fields.extend(path.fields.iter().cloned());
    }
    Some(replacement)
}
/// Substitute actuals, lifting only under an explicitly closed footprint.
/// Failure produces open evidence and preserves the unresolved obligation.
#[must_use]
pub fn substitute(
    summary: &Summary,
    callee_scope: &str,
    actuals: &[Option<Path>],
    result: Option<&Path>,
    closed_footprint: Option<&str>,
) -> Summary {
    let mut out = Summary {
        open: summary.open.clone(),
        poison_all: summary.poison_all,
        ..Default::default()
    };
    let sub = |p: &Path| substitute_path(p, callee_scope, actuals, result, closed_footprint);
    // A lifted/unknown actual is not an exact runtime registration key.
    let exact = |p: &Path| sub(p).filter(concrete);
    out.ordered = summary.ordered.as_ref().and_then(|steps| {
        steps
            .iter()
            .map(|step| {
                Some(match step {
                    OrderedStep::Register(r) => OrderedStep::Register(Registration {
                        object: exact(&r.object)?,
                        holder: exact(&r.holder)?,
                        poisoned: r.poisoned,
                    }),
                    OrderedStep::Remove(o, h) => OrderedStep::Remove(
                        match o {
                            Some(o) => Some(exact(o)?),
                            None => None,
                        },
                        match h {
                            Some(h) => Some(exact(h)?),
                            None => None,
                        },
                    ),
                    OrderedStep::Destroy(h) => OrderedStep::Destroy(exact(h)?),
                    OrderedStep::Release(o) => OrderedStep::Release(exact(o)?),
                })
            })
            .collect()
    });
    for release in &summary.may_release {
        if let Some(object) = sub(&release.object) {
            out.may_release.insert(ReleaseEffect {
                object,
                destroyed_before: release
                    .destroyed_before
                    .iter()
                    .filter_map(&sub)
                    .filter(concrete)
                    .collect(),
            });
        } else {
            out.open.insert("unresolved may-release root".into());
            // Preserve the obligation rather than silently drop the effect.
            out.may_release.insert(release.clone());
        }
    }
    for object in &summary.may_poison {
        if let Some(object) = sub(object) {
            out.may_poison.insert(object);
        } else {
            out.poison_all = true;
        }
    }
    for r in &summary.may_register {
        match (sub(&r.object), sub(&r.holder)) {
            (Some(object), Some(holder)) => {
                out.may_register.insert(Registration {
                    object,
                    holder,
                    poisoned: r.poisoned,
                });
            }
            _ => {
                out.open
                    .insert("unresolved registration root without a closed footprint".into());
            }
        }
    }
    for e in &summary.must_end {
        match e {
            Ending::Remove(o, h) => {
                if let (Some(object), Some(holder)) = (sub(o), sub(h)) {
                    // Type lifting covers may-effects; it never establishes a must
                    // ending of every object sharing the same record type.
                    if !matches!(object.root, Root::Lifted(_))
                        && !matches!(holder.root, Root::Lifted(_))
                    {
                        out.must_end.insert(Ending::Remove(object, holder));
                    }
                }
            }
            Ending::Destroy(h) => {
                if let Some(holder) = sub(h)
                    && !matches!(holder.root, Root::Lifted(_))
                {
                    out.must_end.insert(Ending::Destroy(holder));
                }
            }
        }
    }
    for r in &summary.requirements {
        match (sub(&r.object), sub(&r.holder)) {
            (Some(object), Some(holder)) => {
                out.requirements.insert(Requirement {
                    object,
                    holder,
                    reason: r.reason.clone(),
                });
            }
            _ => {
                out.open
                    .insert("unresolved lifetime precondition root".into());
                out.requirements.insert(r.clone());
            }
        }
    }
    out
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Standing {
    Validated,
    DeclaredAssumption {
        premise: String,
        reasons: BTreeSet<String>,
    },
    Withdrawn {
        reasons: BTreeSet<String>,
    },
}
/// Check all available registration/store sites. A contradictory typed field
/// withdraws; unread evidence becomes an explicit premise rather than a proof.
#[must_use]
pub fn standing(
    protocol: &RegisteredProtocol,
    units: &[&SourceFacts],
    unread: &BTreeMap<String, String>,
) -> Standing {
    let mut reasons = BTreeSet::new();
    let mut contradictions = BTreeSet::new();
    let mut registrations = 0;
    let key = |fields: &[String]| format!("{}.{}", protocol.record, fields.join("."));
    for unit in units {
        reasons.extend(unit.open.iter().cloned());
        if protocol.holder.len() == 1 {
            match unit.fields.get(&key(&protocol.holder)) {
                Some(ty) if !ty.contains('*') || ty.contains('(') => {
                    contradictions.insert(format!(
                        "{} has non-holder-pointer type {ty}",
                        key(&protocol.holder)
                    ));
                }
                _ => {}
            }
        }
        for (function, facts) in &unit.functions {
            reasons.extend(facts.open.iter().map(|why| format!("{function}: {why}")));
            for op in &facts.operations {
                if op.kind == OperationKind::Register {
                    match &op.object {
                        Some(object)
                            if object.record == protocol.record
                                && object.fields == protocol.interior =>
                        {
                            registrations += 1;
                            if facts
                                .calls
                                .iter()
                                .filter(|call| call.offset == op.offset)
                                .any(|call| call.actuals != call.typed_effect_actuals)
                            {
                                reasons.insert(format!("{function}: registration field interpretation is not independently typed"));
                            }
                            match &op.holder {
                                Some(holder)
                                    if holder.root == object.root
                                        && holder.record == object.record
                                        && holder.fields == protocol.holder => {}
                                _ => {
                                    reasons.insert(format!("{function}: registration holder lacks a derived footprint equality"));
                                }
                            }
                        }
                        None => {
                            reasons.insert(format!(
                                "{function}: registration has an unresolved held object"
                            ));
                        }
                        _ => {}
                    }
                }
            }
            for (target, _) in &facts.stores {
                if target.is_none() {
                    reasons.insert(format!(
                        "{function}: unresolved member store cannot exclude the declared holder"
                    ));
                }
                if target
                    .as_ref()
                    .is_some_and(|p| p.record == protocol.record && p.fields == protocol.holder)
                {
                    reasons.insert(format!("{function}: holder store requires initialization-before-publication/stability evidence"));
                }
            }
        }
    }
    if !contradictions.is_empty() {
        return Standing::Withdrawn {
            reasons: contradictions,
        };
    }
    if registrations == 0 {
        reasons.insert("no registration of the declared interior read".into());
    }
    reasons.extend(
        unread
            .iter()
            .map(|(unit, why)| format!("unread {unit}: {why}")),
    );
    if reasons.is_empty() {
        Standing::Validated
    } else {
        Standing::DeclaredAssumption {
            premise: format!("loop-protocol-unchecked:{}", key(&protocol.interior)),
            reasons,
        }
    }
}
/// Demote only. Every unresolved root forwards its named obligations to all
/// of its dependent callers; an already refused row never becomes admitted.
#[must_use]
pub fn fold(
    initially_refused: &BTreeSet<String>,
    summaries: &BTreeMap<String, Summary>,
    callers: &BTreeMap<String, BTreeSet<String>>,
) -> BTreeSet<String> {
    let mut refused = initially_refused.clone();
    refused.extend(
        summaries
            .iter()
            .filter(|(_, s)| !s.requirements.is_empty() || !s.open.is_empty())
            .map(|(n, _)| n.clone()),
    );
    loop {
        let before = refused.len();
        for callee in refused.clone() {
            if let Some(parents) = callers.get(&callee) {
                refused.extend(parents.iter().cloned());
            }
        }
        if refused.len() == before {
            break;
        }
    }
    refused
}

#[cfg(test)]
mod tests {
    use super::*;
    fn p(root: usize, field: &str) -> Path {
        Path {
            root: Root::Formal {
                scope: "unit-config-source:f".into(),
                index: root,
            },
            fields: vec![field.into()],
        }
    }
    fn reg() -> Registration {
        Registration {
            object: p(0, "event"),
            holder: p(0, "evl"),
            poisoned: false,
        }
    }
    #[test]
    fn a_callee_registration_invalidates_an_earlier_destroy_like_a_local_one() {
        let r = reg();
        let local = path(&[
            Event::Destroy(r.holder.clone()),
            Event::Register(r.clone()),
            Event::Release(r.object.clone()),
        ]);
        let called = path(&[
            Event::Destroy(r.holder.clone()),
            Event::Call(path(&[Event::Register(r.clone())])),
            Event::Release(r.object.clone()),
        ]);
        assert_eq!(local, called);
        assert!(!called.requirements.is_empty());
        assert!(
            called
                .may_release
                .iter()
                .all(|r| r.destroyed_before.is_empty())
        );
    }
    #[test]
    fn unmatched_then_matched_removal_keeps_poison_until_destroy() {
        let r = reg();
        let events = [
            Event::Register(r.clone()),
            Event::Remove(Some(r.object.clone()), Some(p(1, "evl"))),
            Event::Remove(Some(r.object.clone()), Some(r.holder.clone())),
            Event::Release(r.object.clone()),
        ];
        let s = path(&events);
        assert_eq!(s.may_register.len(), 1);
        assert!(
            s.requirements
                .iter()
                .any(|r| r.reason == "retention-removal-poisoned")
        );
        let mut ended = events[..3].to_vec();
        ended.push(Event::Destroy(r.holder));
        ended.push(Event::Release(r.object));
        assert!(path(&ended).requirements.is_empty());
    }
    #[test]
    fn one_branch_ending_is_not_a_must_ending() {
        let r = reg();
        let ended = path(&[Event::Remove(Some(r.object), Some(r.holder))]);
        let joined = join(&[ended, Summary::default()]);
        assert!(joined.must_end.is_empty());
    }
    #[test]
    fn lifting_a_lookup_keeps_registration_but_never_proves_a_must_ending() {
        let r = reg();
        let s = path(&[Event::Register(r.clone())]);
        let lifted = substitute(
            &s,
            "unit-config-source:f",
            &[None],
            None,
            Some("collector.event"),
        );
        assert_eq!(lifted.may_register.len(), 1);
        let removed = path(&[Event::Remove(Some(r.object), Some(r.holder))]);
        assert!(
            substitute(
                &removed,
                "unit-config-source:f",
                &[None],
                None,
                Some("collector.event")
            )
            .must_end
            .is_empty()
        );
        assert!(
            !substitute(&s, "unit-config-source:f", &[None], None, None)
                .open
                .is_empty()
        );
    }
    #[test]
    fn one_unknown_holder_destroy_cannot_end_a_closed_type_footprint() {
        let record = Registration {
            object: Path {
                root: Root::Lifted("run:collector.event".into()),
                fields: vec!["event".into()],
            },
            holder: Path {
                root: Root::Lifted("run:collector.event".into()),
                fields: vec!["evl".into()],
            },
            poisoned: false,
        };
        let summary = path(&[
            Event::Register(record.clone()),
            Event::Destroy(record.holder.clone()),
            Event::Release(record.object.clone()),
        ]);
        assert_eq!(summary.may_register.len(), 1);
        assert!(!summary.requirements.is_empty());
        assert!(!summary.open.is_empty());
    }
    #[test]
    fn a_result_release_may_alias_a_formal_registration() {
        let record = reg();
        let result = Path {
            root: Root::Result {
                scope: "another-source:other".into(),
            },
            fields: Vec::new(),
        };
        assert!(
            !path(&[Event::Register(record), Event::Release(result)])
                .requirements
                .is_empty()
        );
    }
    #[test]
    fn a_callee_release_checks_the_callers_incoming_registration_in_order() {
        let record = reg();
        let release = path(&[Event::Release(record.object.clone())]);
        assert_eq!(release.may_release.len(), 1);
        assert!(
            !path(&[Event::Register(record.clone()), Event::Call(release)])
                .requirements
                .is_empty()
        );
        let destroy_first = path(&[
            Event::Destroy(record.holder.clone()),
            Event::Release(record.object.clone()),
        ]);
        assert!(
            path(&[Event::Register(record.clone()), Event::Call(destroy_first)])
                .requirements
                .is_empty()
        );
        let release_first = path(&[
            Event::Release(record.object.clone()),
            Event::Destroy(record.holder.clone()),
        ]);
        assert!(
            !path(&[Event::Register(record), Event::Call(release_first)])
                .requirements
                .is_empty()
        );
    }
    #[test]
    fn a_lifted_destroy_prefix_cannot_discharge_an_incoming_registration() {
        let mut record = reg();
        record.holder.root = Root::Lifted("closed-footprint".into());
        let forged = Summary {
            may_release: BTreeSet::from([ReleaseEffect {
                object: record.object.clone(),
                destroyed_before: BTreeSet::from([record.holder.clone()]),
            }]),
            ..Default::default()
        };
        assert!(
            !path(&[Event::Register(record), Event::Call(forged)])
                .requirements
                .is_empty()
        );
    }
    /// Shared with QCore.LoopLifetimeSummary.destroyVectors. Destroying the
    /// exact concrete holder ends a record regardless of poison; a different
    /// holder or a lifted footprint cannot do so.
    #[test]
    fn shared_destroy_vectors() {
        for (poisoned, same_holder, lifted, survives) in [
            (false, true, false, false),
            (true, true, false, false),
            (false, false, false, true),
            (true, false, false, true),
            (false, true, true, true),
            (true, true, true, true),
        ] {
            let mut record = reg();
            record.poisoned = poisoned;
            if lifted {
                record.holder.root = Root::Lifted("footprint".into());
            }
            let holder = if same_holder {
                record.holder.clone()
            } else {
                p(1, "evl")
            };
            let mut records = BTreeSet::from([record]);
            end(&mut records, &Ending::Destroy(holder));
            assert_eq!(!records.is_empty(), survives);
        }
    }
    #[test]
    fn fold_preserves_old_refusals_and_demotes_all_callers() {
        let summaries = BTreeMap::from([(
            "leaf".into(),
            Summary {
                open: BTreeSet::from(["unjoined worker".into()]),
                ..Default::default()
            },
        )]);
        let callers = BTreeMap::from([
            ("leaf".into(), BTreeSet::from(["middle".into()])),
            ("middle".into(), BTreeSet::from(["root".into()])),
        ]);
        let refused = fold(&BTreeSet::from(["old".into()]), &summaries, &callers);
        assert_eq!(
            refused,
            BTreeSet::from(["leaf".into(), "middle".into(), "root".into(), "old".into()])
        );
    }
}

/// Per-unit source summaries. Cross-unit calls without a separately bound
/// home remain open. This routine reads every selected function/call and never
/// obtains a footprint from a host call's instance flag.
pub fn derive_source(
    unit: &SourceFacts,
    protocols: &[RegisteredProtocol],
    closed: &BTreeSet<String>,
    _contracts: &qcore_schemas::contract::ResourceContracts,
) -> BTreeMap<String, Summary> {
    let key =
        |p: &RegisteredProtocol| format!("{}:{}.{}", unit.identity, p.record, p.interior.join("."));
    let mut relevant = unit
        .functions
        .iter()
        .filter(|(_, f)| {
            f.operations.iter().any(|op| {
                op.object
                    .as_ref()
                    .is_some_and(|o| protocols.iter().any(|p| p.record == o.record))
            }) || f.calls.iter().any(|call| {
                call.actuals
                    .iter()
                    .flatten()
                    .any(|o| protocols.iter().any(|p| p.record == o.record))
            })
        })
        .map(|(name, _)| name.clone())
        .collect::<BTreeSet<_>>();
    loop {
        let before = relevant.len();
        for (name, f) in &unit.functions {
            if f.calls.iter().any(|call| relevant.contains(&call.callee)) {
                relevant.insert(name.clone());
            }
        }
        if relevant.len() == before {
            break;
        }
    }
    let mut summaries = BTreeMap::new();
    let mut converged = false;
    for _ in 0..unit.functions.len().saturating_add(1) {
        let before = summaries.clone();
        for (name, f) in &unit.functions {
            if !relevant.contains(name) {
                continue;
            }
            let scope = format!("{}:{name}", unit.identity);
            let symbolic = |p: &crate::loop_lifetime_source::Path| -> Option<Path> {
                let root = if let Some(index) = f.parameters.iter().position(|id| id == &p.root) {
                    Root::Formal {
                        scope: scope.clone(),
                        index,
                    }
                } else if f
                    .returned
                    .iter()
                    .flatten()
                    .any(|r| r.root == p.root && r.fields.is_empty())
                {
                    Root::Result {
                        scope: scope.clone(),
                    }
                } else {
                    let protocol = protocols.iter().find(|protocol| {
                        protocol.record == p.record && closed.contains(&key(protocol))
                    })?;
                    Root::Lifted(key(protocol))
                };
                Some(Path {
                    root,
                    fields: p.fields.clone(),
                })
            };
            let mut timed: Vec<(u64, Event)> = Vec::new();
            let mut open = f.open.clone();
            if !f.stores.is_empty() {
                open.insert("member stores prevent holder snapshot stability".into());
            }
            let primitive_offsets = f
                .operations
                .iter()
                .map(|op| op.offset)
                .collect::<BTreeSet<_>>();
            let mut release_footprints = BTreeSet::new();
            for op in &f.operations {
                match op.kind {
                    OperationKind::Register => {
                        if let (Some(object), Some(holder)) = (&op.object, &op.holder) {
                            if protocols
                                .iter()
                                .any(|p| p.record == object.record && p.interior == object.fields)
                            {
                                if let (Some(object), Some(holder)) =
                                    (symbolic(object), symbolic(holder))
                                {
                                    timed.push((
                                        op.offset,
                                        Event::Register(Registration {
                                            object,
                                            holder,
                                            poisoned: false,
                                        }),
                                    ));
                                } else {
                                    open.insert("registration root has no closed footprint".into());
                                }
                            }
                        } else {
                            open.insert("registration has unresolved source operands".into());
                        }
                    }
                    OperationKind::Remove => {
                        // A conditional removal may poison and need not end
                        // anything. Preserve that alternative instead of
                        // pretending it dominates a later release.
                        if op.conditional {
                            timed.push((op.offset, Event::Remove(None, None)));
                        } else {
                            timed.push((
                                op.offset,
                                Event::Remove(
                                    op.object.as_ref().and_then(symbolic),
                                    op.holder.as_ref().and_then(symbolic),
                                ),
                            ));
                        }
                    }
                    OperationKind::Destroy => {
                        if !op.conditional {
                            if let Some(holder) = op.holder.as_ref().and_then(symbolic) {
                                timed.push((op.offset, Event::Destroy(holder)));
                            } else {
                                open.insert("destroy has unresolved holder root".into());
                            }
                        }
                    }
                    OperationKind::Release => {
                        if let Some(object) = &op.object {
                            if let Some(released) = symbolic(object) {
                                timed.push((op.offset, Event::Release(released.clone())));
                                // Incoming registrations may already be poisoned.
                                // A cross-function removal alone never discharges
                                // this obligation in the current abstraction.
                                for p in protocols.iter().filter(|p| p.record == object.record) {
                                    let mut interior = released.clone();
                                    interior.fields.extend(p.interior.iter().cloned());
                                    let mut holder = released.clone();
                                    holder.fields.extend(p.holder.iter().cloned());
                                    release_footprints.insert((released.clone(), interior, holder));
                                }
                            } else if protocols.iter().any(|p| p.record == object.record) {
                                open.insert("released record has an unresolved root".into());
                            }
                        } else {
                            open.insert("release has an unresolved object".into());
                        }
                    }
                }
            }
            for call in &f.calls {
                if primitive_offsets.contains(&call.offset) {
                    continue;
                }
                if relevant.contains(&call.callee) {
                    if let Some(callee) = before.get(&call.callee) {
                        let actuals = call
                            .typed_effect_actuals
                            .iter()
                            .map(|p| p.as_ref().and_then(symbolic))
                            .collect::<Vec<_>>();
                        let callee_scope = format!("{}:{}", unit.identity, call.callee);
                        // Different record types need different footprints;
                        // no guessed universal lift key is supplied here.
                        let mut effect = substitute(callee, &callee_scope, &actuals, None, None);
                        // A same-unit external definition does not establish
                        // which body a call executes in the linked program.
                        if !unit
                            .functions
                            .get(&call.callee)
                            .is_some_and(|f| f.internal_linkage)
                        {
                            effect.ordered = None;
                        }
                        let effect = if call.conditional {
                            join(&[effect, Summary::default()])
                        } else {
                            effect
                        };
                        timed.push((call.offset, Event::Call(effect)));
                    } else {
                        open.insert(format!("lifetime summary unavailable for {}", call.callee));
                    }
                } else {
                    // Custody-only declarations are not heap frame proofs.
                    // An unread call may change the record's holder member.
                    open.insert(format!(
                        "unproved holder frame across call: {}",
                        call.callee
                    ));
                }
            }
            timed.sort_by_key(|(offset, _)| *offset);
            let mut summary = path(&timed.into_iter().map(|(_, e)| e).collect::<Vec<_>>());
            summary.open.extend(open);
            if !f.complete_straight_line || !summary.open.is_empty() {
                let had_ending = !summary.must_end.is_empty()
                    || summary
                        .may_release
                        .iter()
                        .any(|effect| !effect.destroyed_before.is_empty());
                summary.ordered = None;
                summary.must_end.clear();
                summary.may_release = summary
                    .may_release
                    .into_iter()
                    .map(|mut effect| {
                        effect.destroyed_before.clear();
                        effect
                    })
                    .collect();
                if had_ending {
                    summary.open.insert(
                        "must-ending/prefix needs complete CFG and stable holder snapshots".into(),
                    );
                }
            }
            for (released, interior, holder) in release_footprints {
                if summary.may_release.iter().any(|effect| {
                    effect.object == released && !effect.destroyed_before.contains(&holder)
                }) {
                    summary.requirements.insert(Requirement {
                        object: interior, holder,
                        reason: "lifetime-precondition-undischarged: released record requires its registration ended before release".into(),
                    });
                }
            }
            summaries.insert(name.clone(), summary);
        }
        if summaries == before {
            converged = true;
            break;
        }
    }
    if !converged {
        for summary in summaries.values_mut() {
            summary.open.insert(
                "lifetime summary fixed point did not converge within its function bound".into(),
            );
            summary.ordered = None;
            summary.must_end.clear();
        }
    }
    summaries
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProtocolEvidence {
    pub host: String,
    pub protocol: RegisteredProtocol,
    pub standing: Standing,
    pub guard: Option<qcore_schemas::loop_lifetime::ConditionGuard>,
}
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RunEvidence {
    pub protocols: Vec<ProtocolEvidence>,
    /// Outer key is the complete source/input/configuration identity; inner
    /// function names are local to that exact reading, never global homes.
    pub summaries: BTreeMap<String, BTreeMap<String, Summary>>,
    /// Fully qualified graph and demote-only closure. Ambiguous cross-unit
    /// homes remain open in summaries rather than acquiring name-only edges.
    pub callers: BTreeMap<String, BTreeSet<String>>,
    pub refused: BTreeSet<String>,
}
/// Run-wide evidence, always separated from a local body verdict. Conditional
/// type protocols remain unresolved until a particular record's instance is
/// correlated with the hosting call's formal; one flag never controls a type.
pub fn run_evidence(
    run: &crate::member_run_facts::RunMemberFacts,
    contracts: &qcore_schemas::contract::ResourceContracts,
) -> RunEvidence {
    let units = run
        .units
        .iter()
        .filter_map(|u| u.lifetime_source.as_ref())
        .collect::<Vec<_>>();
    let mut out = RunEvidence::default();
    for (host, declared) in contracts.lifetime_declarations() {
        let Some(lifetime) = &declared.lifetime else {
            continue;
        };
        for protocol in &lifetime.registered {
            out.protocols.push(ProtocolEvidence {
                host: host.clone(),
                protocol: protocol.clone(),
                standing: standing(protocol, &units, &run.unread),
                guard: None,
            });
        }
        for block in &lifetime.blocks {
            for clause in &block.clauses {
                if let qcore_schemas::loop_lifetime::LocalClause::Registered(protocol) = clause {
                    let mut status = standing(protocol, &units, &run.unread);
                    if !matches!(status, Standing::Withdrawn { .. }) {
                        status=Standing::DeclaredAssumption{premise:format!("loop-protocol-instance-correlation:{}:{}",host,block.guard.condition),reasons:BTreeSet::from(["conditional protocol needs a checked record.instance == hosting actual relation; a function-local flag is not a type-wide guard".into()])};
                    }
                    out.protocols.push(ProtocolEvidence {
                        host: host.clone(),
                        protocol: protocol.clone(),
                        standing: status,
                        guard: Some(block.guard.clone()),
                    });
                }
            }
        }
    }
    let protocols = out
        .protocols
        .iter()
        .map(|p| p.protocol.clone())
        .collect::<Vec<_>>();
    for unit in &units {
        let closed = out
            .protocols
            .iter()
            .filter(|p| p.guard.is_none() && matches!(p.standing, Standing::Validated))
            .map(|p| {
                format!(
                    "{}:{}.{}",
                    unit.identity,
                    p.protocol.record,
                    p.protocol.interior.join(".")
                )
            })
            .collect();
        out.summaries.insert(
            unit.identity.clone(),
            derive_source(unit, &protocols, &closed, contracts),
        );
    }
    let mut qualified = BTreeMap::new();
    for unit in &units {
        if let Some(summaries) = out.summaries.get(&unit.identity) {
            for (function, summary) in summaries {
                let caller = format!("{}:{function}", unit.identity);
                qualified.insert(caller.clone(), summary.clone());
                if let Some(body) = unit.functions.get(function) {
                    for call in &body.calls {
                        if summaries.contains_key(&call.callee) {
                            let callee = format!("{}:{}", unit.identity, call.callee);
                            out.callers
                                .entry(callee)
                                .or_default()
                                .insert(caller.clone());
                        }
                    }
                }
            }
        }
    }
    let unresolved_roots = qualified
        .iter()
        .filter(|(key, summary)| {
            !summary.may_register.is_empty() && !out.callers.contains_key(*key)
        })
        .map(|(key, _)| key.clone())
        .collect();
    out.refused = fold(&unresolved_roots, &qualified, &out.callers);
    out
}

/// Exact occurrence/configuration namespace carried by this body's scoped
/// effect operands, including when its outgoing effects were later ended.
pub fn body_scope(body: &crate::legacy_c::LegacyFunctionSummary) -> Option<String> {
    let mut scopes = BTreeSet::new();
    for node in &body.cfg.nodes {
        if let crate::legacy_c::LegacyInstruction::ContractCall {
            retention: Some(operands),
            ..
        } = &node.instruction
        {
            for proof in operands.forwarded.values() {
                if let Root::Formal { scope, .. } = &proof.registration.object.root {
                    scopes.insert(scope.clone());
                }
            }
        }
    }
    (scopes.len() == 1)
        .then(|| scopes.into_iter().next())
        .flatten()
}
