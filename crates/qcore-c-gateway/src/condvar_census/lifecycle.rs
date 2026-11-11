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

//! Lifecycle readings for the C5 census (cards **W47-FLB-LIB-LIFECYCLE** and
//! **W47-FLB-LIB-STRICT-PAIRING**, gateway rule 10q as amended, contract
//! specification §C04.11).
//!
//! The declared lifecycle functions (`requires resource_lifecycle(ctx,
//! start|stop|destroy)`) are read two ways, and both are *readings*: the
//! kernel (`qcore_core::paired_lifecycle`) decides, through the adapter.
//!
//! * **The bodies** ([`LifecycleBodyReading`]): each declared stop is run by
//!   the path reader after a declared start that returned 0 (L2) and, as
//!   observations of what the body does outside the strict pair (card
//!   W47-FLB-LIB-STRICT-PAIRING), from a never-started context, after a stop
//!   that followed it and after a start that returned nonzero — recording,
//!   per followed path, every join it executed and how the reader classifies
//!   it (the first join of the thread the start began, a repeated join, a
//!   join of an identifier no start began, an underived identity). Each
//!   declared destroy is run after a successful start the same way, and each
//!   declared start's own paths that return nonzero record the threads they
//!   leave without a join (L7).
//! * **The callers** ([`LifecycleCallReading`]): on a test root's paths, each
//!   call of a declared function with its context's key and the context's
//!   history before the call (every lifecycle event since a creator's result
//!   was assigned to that key). A release records the histories of every
//!   context on each path reaching it.
//!
//! A context's key is a field key (`TYPE.FIELD` for `B->FIELD`/`B.FIELD` with
//! `B` of declared struct type `TYPE`) or a frame-local name
//! (`FUNCTION::NAME`); anything else has no key and no history. A context
//! enters the created state where a creator's result is assigned to its key
//! (P-LIFECYCLE-INSTANCE, stated: one key names one instance at a time). A key
//! with no history is not derived: nothing is decided from it.

use std::collections::{BTreeMap, BTreeSet};

use qcore_contract::contract_loader::LifecycleOperation;

use super::{C5, Global, KEYWORDS, Val, texts};
use crate::legacy_c::is_identifier;

/// The declared lifecycle functions, with the parameter naming the context,
/// and the creators whose result starts a context's history.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LifecycleRoles {
    pub functions: BTreeMap<String, (usize, LifecycleOperation)>,
    pub creators: BTreeSet<String>,
}

impl LifecycleRoles {
    /// The declared functions of one operation, in name order.
    #[must_use]
    pub fn of(&self, operation: LifecycleOperation) -> Vec<&str> {
        self.functions
            .iter()
            .filter(|(_, (_, op))| *op == operation)
            .map(|(name, _)| name.as_str())
            .collect()
    }
}

/// What a declared start returned on one path.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ReadStartResult {
    Zero,
    NonZero,
    Unknown,
}

/// One lifecycle event of a context's history.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ReadLifeEvent {
    /// A creator's result was assigned to the key.
    Create,
    Start(ReadStartResult),
    Stop,
    Destroy,
}

/// A context's history on one path, each event with its site.
pub type LifeHistory = Vec<(ReadLifeEvent, String)>;

/// How the path reader classifies one join a body executed.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ReadTraceJoin {
    /// The first join of a thread this path started.
    First,
    /// A join of a thread this path had already joined.
    Repeated,
    /// A join of an identifier naming no thread this path started.
    NotStarted,
    /// The joined identity is not derived.
    Unresolved,
}

/// The state a declared stop's body is run from.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum LifecycleScenario {
    /// A never-started context (outside the strict pair: L5 forbids it; read
    /// as an observation).
    Unstarted,
    /// After the declared start returned 0 (L2).
    AfterStart { start: String },
    /// After a stop that followed a start that returned 0 (outside the strict
    /// pair: L3 forbids it; read as an observation).
    AfterStop { start: String },
    /// After the declared start returned nonzero (outside the strict pair: L4
    /// forbids it; read as an observation).
    AfterFailedStart { start: String },
    /// A declared destroy after the declared start returned 0 (L6's reading).
    DestroyAfterStart { start: String },
    /// Card W47-FLB-LIB-STRICT-PAIRING: the declared start's own run, on its
    /// paths that return nonzero (L7: nothing it began is left unjoined).
    FailedStartReturn { start: String },
}

/// One followed path of a body run from a scenario's state.
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct BodyPathReading {
    pub joins: Vec<(ReadTraceJoin, String)>,
    /// What the reader left open on the path, from the scenario's start.
    pub unresolved: BTreeSet<String>,
    /// Card W47-FLB-LIB-STRICT-PAIRING: the threads the path began and left
    /// without a join ([`LifecycleScenario::FailedStartReturn`] only; empty
    /// for a stop or destroy).
    pub left_unjoined: BTreeSet<String>,
}

/// One declared body, run from one scenario's state.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LifecycleBodyReading {
    /// The declared function run (a stop, or a destroy).
    pub function: String,
    pub scenario: LifecycleScenario,
    /// Every followed path; empty when no path reaches the scenario's state.
    pub paths: Vec<BodyPathReading>,
    /// The start's own unresolved reasons, when the scenario starts with one.
    pub start_unresolved: BTreeSet<String>,
}

/// One call of a declared lifecycle function on a test root's paths.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LifecycleCallReading {
    pub root: String,
    pub file: String,
    pub line: usize,
    /// The census name of the calling function.
    pub caller: String,
    pub callee: String,
    pub operation: LifecycleOperation,
    /// The context's key, when it has one.
    pub key: Option<String>,
    /// The context's history before the call, per path state; `None` when the
    /// key has none (not derived).
    pub before: BTreeSet<Option<LifeHistory>>,
}

/// The start result a returned value is.
pub(super) fn start_result(value: Option<&Val>) -> ReadStartResult {
    match value {
        Some(Val::Const(0)) => ReadStartResult::Zero,
        Some(Val::Const(_) | Val::NonZero) => ReadStartResult::NonZero,
        _ => ReadStartResult::Unknown,
    }
}

impl C5<'_> {
    /// A context's key from its argument's words in `definition`'s frame: a
    /// field key of a declared struct base, or a frame-local name.
    fn context_key(&self, definition: usize, words: &[&str]) -> Option<String> {
        match words {
            [name] if is_identifier(name) && !KEYWORDS.contains(name) => {
                Some(format!("{}::{name}", self.definitions[definition].census))
            }
            [base, "->" | ".", field] if is_identifier(field) => {
                self.types.get(*base).map(|ty| format!("{ty}.{field}"))
            }
            _ => None,
        }
    }

    /// A declared lifecycle call at `name_index`: record it, with its
    /// context's history on every incoming state, and return its key (when
    /// it has one) and operation. `None` when no roles are tracked or the
    /// callee declares none.
    pub(super) fn lifecycle_call(
        &mut self,
        definition: usize,
        name: &str,
        args: &[(usize, usize)],
        name_index: usize,
        current: &[((Global, super::Env), Option<Val>)],
    ) -> Option<(String, LifecycleOperation)> {
        let roles = self.roles?;
        let (parameter, operation) = *roles.functions.get(name)?;
        let def = &self.definitions[definition];
        let scan = &self.scans[def.file];
        let key = args
            .get(parameter)
            .and_then(|range| self.context_key(definition, &texts(&scan.tokens, *range)));
        let line = scan.tokens[name_index].line;
        let entry = self
            .lifecycle_calls
            .entry((scan.path.clone(), line, name.to_owned()))
            .or_insert_with(|| LifecycleCallReading {
                root: self.root.clone(),
                file: scan.path.clone(),
                line,
                caller: def.census.clone(),
                callee: name.to_owned(),
                operation,
                key: key.clone(),
                before: BTreeSet::new(),
            });
        for ((global, _), _) in current {
            entry.before.insert(
                key.as_ref()
                    .and_then(|key| global.histories.get(key).cloned()),
            );
        }
        key.map(|key| (key, operation))
    }

    /// `KEY = CREATOR(…)` (or a declaration `TYPE *NAME = CREATOR(…)`): the
    /// key and the creator's site.
    pub(super) fn created_key(
        &self,
        definition: usize,
        words: &[&str],
        range: (usize, usize),
    ) -> Option<(String, String)> {
        let roles = self.roles?;
        let position = words.iter().position(|w| *w == "=")?;
        let creator = words.get(position + 1)?;
        if !roles.creators.contains(*creator) || words.get(position + 2) != Some(&"(") {
            return None;
        }
        let left = &words[..position];
        let key = self.context_key(definition, left).or_else(|| match left {
            // A declaration: the declared name after its type or `*`.
            [.., before, name]
                if is_identifier(name)
                    && (*before == "*" || is_identifier(before))
                    && !left.iter().any(|w| matches!(*w, "->" | "." | "[")) =>
            {
                self.context_key(definition, &[*name])
            }
            _ => None,
        })?;
        Some((key, self.site(definition, range.0 + position + 1)))
    }

    /// Run `function` from `global` with the join trace cleared, and return
    /// each resulting state with its value.
    fn traced(&mut self, function: usize, mut global: Global) -> Vec<(Global, Option<Val>)> {
        global.trace.clear();
        self.budget = Some(super::ENTRY_STEP_BUDGET);
        self.run_function(function, global).into_iter().collect()
    }
}

fn body_paths(states: &[(Global, Option<Val>)], before: &BTreeSet<String>) -> Vec<BodyPathReading> {
    let mut paths: BTreeSet<BodyPathReading> = BTreeSet::new();
    for (global, _) in states {
        paths.insert(BodyPathReading {
            joins: global.trace.clone(),
            unresolved: global.unresolved.difference(before).cloned().collect(),
            left_unjoined: BTreeSet::new(),
        });
    }
    paths.into_iter().collect()
}

/// Run every declared stop (and destroy) from each scenario's state. `c5`
/// carries the program's reading context with the join trace on.
pub(super) fn read_bodies(c5: &mut C5<'_>, roles: &LifecycleRoles) -> Vec<LifecycleBodyReading> {
    let definition = |c5: &C5<'_>, name: &str| c5.by_census.get(name).copied();
    let mut out = Vec::new();
    let starts = roles.of(LifecycleOperation::Start);
    let stops = roles.of(LifecycleOperation::Stop);
    let destroys = roles.of(LifecycleOperation::Destroy);
    for stop_name in &stops {
        let Some(stop) = definition(c5, stop_name) else {
            continue;
        };
        let states = c5.traced(stop, Global::default());
        out.push(LifecycleBodyReading {
            function: (*stop_name).to_owned(),
            scenario: LifecycleScenario::Unstarted,
            paths: body_paths(&states, &BTreeSet::new()),
            start_unresolved: BTreeSet::new(),
        });
        for start_name in &starts {
            let Some(start) = definition(c5, start_name) else {
                continue;
            };
            let started = c5.traced(start, Global::default());
            let start_unresolved: BTreeSet<String> = started
                .iter()
                .flat_map(|(g, _)| g.unresolved.iter().cloned())
                .collect();
            let (mut succeeded, mut failed) = (Vec::new(), Vec::new());
            for (global, value) in started {
                match start_result(value.as_ref()) {
                    ReadStartResult::Zero => succeeded.push(global),
                    ReadStartResult::NonZero => failed.push(global),
                    ReadStartResult::Unknown => {
                        succeeded.push(global.clone());
                        failed.push(global);
                    }
                }
            }
            let mut after_start = Vec::new();
            let mut after_stop = Vec::new();
            for global in succeeded {
                let before = global.unresolved.clone();
                let first = c5.traced(stop, global);
                after_start.extend(body_paths(&first, &before));
                for (again, _) in first {
                    let before = again.unresolved.clone();
                    let second = c5.traced(stop, again);
                    after_stop.extend(body_paths(&second, &before));
                }
            }
            let mut after_failed = Vec::new();
            for global in failed {
                let before = global.unresolved.clone();
                let states = c5.traced(stop, global);
                after_failed.extend(body_paths(&states, &before));
            }
            let dedup = |paths: Vec<BodyPathReading>| -> Vec<BodyPathReading> {
                paths
                    .into_iter()
                    .collect::<BTreeSet<_>>()
                    .into_iter()
                    .collect()
            };
            for (scenario, paths) in [
                (
                    LifecycleScenario::AfterStart {
                        start: (*start_name).to_owned(),
                    },
                    after_start,
                ),
                (
                    LifecycleScenario::AfterStop {
                        start: (*start_name).to_owned(),
                    },
                    after_stop,
                ),
                (
                    LifecycleScenario::AfterFailedStart {
                        start: (*start_name).to_owned(),
                    },
                    after_failed,
                ),
            ] {
                out.push(LifecycleBodyReading {
                    function: (*stop_name).to_owned(),
                    scenario,
                    paths: dedup(paths),
                    start_unresolved: start_unresolved.clone(),
                });
            }
        }
    }
    // Card W47-FLB-LIB-STRICT-PAIRING: each declared start's paths that
    // return nonzero, with the threads each leaves without a join (L7).
    for start_name in &starts {
        let Some(start) = definition(c5, start_name) else {
            continue;
        };
        let started = c5.traced(start, Global::default());
        let start_unresolved: BTreeSet<String> = started
            .iter()
            .flat_map(|(g, _)| g.unresolved.iter().cloned())
            .collect();
        let mut paths = BTreeSet::new();
        for (global, value) in &started {
            if start_result(value.as_ref()) != ReadStartResult::NonZero {
                continue;
            }
            paths.insert(BodyPathReading {
                joins: global.trace.clone(),
                unresolved: global.unresolved.clone(),
                left_unjoined: global
                    .threads
                    .iter()
                    .filter(|(_, joined)| matches!(joined, super::Joined::Live(_)))
                    .map(|(thread, _)| thread.clone())
                    .collect(),
            });
        }
        out.push(LifecycleBodyReading {
            function: (*start_name).to_owned(),
            scenario: LifecycleScenario::FailedStartReturn {
                start: (*start_name).to_owned(),
            },
            paths: paths.into_iter().collect(),
            start_unresolved,
        });
    }
    for destroy_name in &destroys {
        let Some(destroy) = definition(c5, destroy_name) else {
            continue;
        };
        for start_name in &starts {
            let Some(start) = definition(c5, start_name) else {
                continue;
            };
            let started = c5.traced(start, Global::default());
            let start_unresolved: BTreeSet<String> = started
                .iter()
                .flat_map(|(g, _)| g.unresolved.iter().cloned())
                .collect();
            let mut paths = BTreeSet::new();
            for (global, value) in started {
                if start_result(value.as_ref()) == ReadStartResult::NonZero {
                    continue;
                }
                let before = global.unresolved.clone();
                paths.extend(body_paths(&c5.traced(destroy, global), &before));
            }
            out.push(LifecycleBodyReading {
                function: (*destroy_name).to_owned(),
                scenario: LifecycleScenario::DestroyAfterStart {
                    start: (*start_name).to_owned(),
                },
                paths: paths.into_iter().collect(),
                start_unresolved,
            });
        }
    }
    out
}
