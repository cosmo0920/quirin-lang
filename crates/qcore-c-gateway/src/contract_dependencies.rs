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

//! Card **VERDICT-CONTRACT-DEPENDENCIES** (gateway rules 13f–13h): the
//! contract declarations each analysed row's derivation consulted, and what
//! the unit's bodies made of the declarations they define.
//!
//! # Where the set comes from (rule 13f)
//!
//! The audit opens a [`Recording`] around one unit's analysis. While it is
//! open, every lookup of a name in a resource environment is noted
//! ([`qcore_schemas::contract_consultation`]) in the log of the function whose
//! work is running: [`within`] opens one around every CFG build, every W4
//! inference and every per-function analysis, keyed by that function. Inner
//! logs are also the enclosing function's, so a body one function's analysis
//! reads on behalf of another (a nested inference) counts for both. The row's
//! own CFG, the instructions its verdict was solved over, is read as well
//! ([`cfg_callees`]): every contract the solver applied is carried there, so
//! that set is in the row's dependencies whatever path looked it up. A name
//! the unit gave an in-unit summary (a W4 inference, an inferred disposition,
//! a ranked or neutral recursion) is not a declaration: its own
//! consultations are added to every row that consulted it, transitively.
//!
//! The record is evidence only. Nothing in the analysis reads it, it is filled
//! only inside an open recording, and a run that opens none analyses exactly
//! as before: no verdict, diagnostic or parity row can depend on it.
//!
//! # Body standing (rule 13g)
//!
//! A declaration is *validated* in a unit only by positive evidence from that
//! unit's own body: a W4 summary equal to the declaration
//! ([`validated`]`(…, "W4")`), the body analysed under the declaration's entry
//! custody, matching every exit and `Safe` with no diagnostic (`"entry"`), or
//! a structural check that admitted it (rules 11r, 12w, 13a, 13c, 13d). A
//! withdrawal in the unit (ABI or mode agreement, a contradicted disposition,
//! a summary conflict, a refutation, a check the body failed) is a failure.
//! Evidence taken under hypothetical holds ([`hypothetical`]: rule 10i's
//! second analysis with every held declaration in force) is not recorded,
//! because those holds are not in force.
//!
//! Card VERDICT-BODY-STANDING-CLAUSE-SCOPE (rules 13i–13k): a unit's
//! `validated` is evidence for the clauses that evidence covers, not for the
//! whole declaration ([`crate::contract_clause_standing`]); a W4 summary equal
//! to the custody projection of a declaration with container or counted-user
//! clauses is recorded too, covering the custody clauses only.

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};

use qcore_schemas::contract::ResourceContracts;
use qcore_schemas::contract_consultation::{self, Consulted};

use crate::legacy_c::{LegacyCfg, LegacyInstruction, LegacyProgramSummary};

/// What one unit's analysis recorded, for [`row_dependencies`] and
/// [`UnitRecord::body_outcomes`].
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct UnitRecord {
    /// Exact first authoritative C stream analyzed in this recording. Home
    /// replay and hypothetical analyses cannot replace the primary stream.
    pub primary_source_sha256: Option<String>,
    /// Authoritative token stream, for selected site observations only.
    pub primary_token_sha256: Option<String>,
    pub reading_sites: Vec<crate::reading_site_evidence::Observation>,
    /// Exact detached effective-environment bytes for retained observations.
    /// Evidence-only: not a source contract, predicate, certificate or consumer.
    pub reading_environments: BTreeMap<String, String>,
    pub reading_site_omissions: BTreeMap<String, usize>,
    /// S3 source member/stability facts, per analyzed function. Evidence only;
    /// runtime projection treats each as a closed-world fact needing coverage.
    pub lifetime_members: BTreeMap<String, BTreeSet<String>>,
    pub lifetime_prerequisites: BTreeMap<String, BTreeSet<String>>,
    /// Per function, every name its work looked up.
    pub functions: BTreeMap<String, Consulted>,
    /// Per function, every identifier its body spells in a stream a CFG
    /// build of it read (unit rewrites, indirect-call candidates and minted
    /// names included). A lookup counts for the function only when the name
    /// it asked for (a minted name read back to its declaration) is one of
    /// them: a pass that scans the whole unit inside one function's build
    /// asks for every other function's callees too.
    pub bodies: BTreeMap<String, BTreeSet<String>>,
    /// Names the unit gave an in-unit summary (never a declaration).
    pub inferred: BTreeSet<String>,
    /// Declared names whose body the unit's derivation scope holds.
    pub examined: BTreeSet<String>,
    /// Declared names, with the evidence that validated them here.
    pub validated: BTreeMap<String, BTreeSet<String>>,
    /// Declared names withdrawn from this unit's call sites, with why.
    pub withdrawn: BTreeMap<String, BTreeSet<String>>,
    /// Declared names whose body failed a check here, with why: a subset of
    /// the reasons of [`Self::withdrawn`] that speak about the body.
    pub failed: BTreeMap<String, BTreeSet<String>>,
    /// Card **CG22-OP-CALLBACKS-AND-SERVED-QUEUES** (rule 12n): the unit's
    /// function-pointer member facts, recorded when the environment holds a
    /// member invocation, for the run's closure. `None` otherwise.
    pub callback_members: Option<crate::callback_member_closure::MemberFacts>,
    /// Card **OP-CONSTRUCTOR-DECLARATIONS** (rules 12o, 13w, 14e): the unit's
    /// classified stores into the members a result state names, recorded
    /// when the environment holds a result state. `None` otherwise.
    pub constructed_states: Option<crate::constructed_state::StateFacts>,
}

/// Card **CG22-OP-CALLBACKS-AND-SERVED-QUEUES** (rule 12n): record the unit's
/// function-pointer member facts (outside every hypothetical analysis).
/// Record source facts a scoped lifetime call actually used.
pub(crate) fn lifetime_evidence(function: &str, members: BTreeSet<String>) {
    with_evidence(|record| {
        record
            .lifetime_members
            .entry(function.to_owned())
            .or_default()
            .extend(members)
    });
}

pub(crate) fn lifetime_entry_evidence(function: &str, prerequisites: BTreeSet<String>) {
    with_evidence(|record| {
        record
            .lifetime_prerequisites
            .entry(function.to_owned())
            .or_default()
            .extend(prerequisites)
    });
}

pub fn member_evidence(facts: crate::callback_member_closure::MemberFacts) {
    with_evidence(|record| record.callback_members = Some(facts));
}

/// Card **OP-CONSTRUCTOR-DECLARATIONS** (rules 12o, 13w, 14e): record the
/// unit's classified stores into the stated members (outside every
/// hypothetical analysis).
pub fn state_evidence(facts: crate::constructed_state::StateFacts) {
    with_evidence(|record| record.constructed_states = Some(facts));
}

/// What one unit's bodies made of one declaration they define.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum BodyOutcome {
    /// Positive evidence from the body and no failure here.
    Validated,
    /// The body was analysed here and nothing validated the declaration.
    Unvalidated,
    /// The body failed a check here.
    Failed,
}

impl BodyOutcome {
    /// The report spelling.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Validated => "validated",
            Self::Unvalidated => "unvalidated",
            Self::Failed => "failed",
        }
    }
}

impl UnitRecord {
    /// Every declaration this unit examined a body of, with what it made of
    /// it. A failure wins over any validation; validation needs evidence.
    #[must_use]
    pub fn body_outcomes(&self) -> BTreeMap<String, BodyOutcome> {
        self.examined
            .iter()
            .chain(self.validated.keys())
            .chain(self.failed.keys())
            .map(|name| {
                let outcome = if self.failed.contains_key(name) {
                    BodyOutcome::Failed
                } else if self.validated.contains_key(name) {
                    BodyOutcome::Validated
                } else {
                    BodyOutcome::Unvalidated
                };
                (name.clone(), outcome)
            })
            .collect()
    }
}

/// One row's dependencies: declared names by standing, and the in-unit
/// summaries whose own dependencies were added.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RowDependencies {
    /// S3 member/stability facts, including inferred summaries' dependencies.
    pub lifetime_members: BTreeSet<String>,
    pub lifetime_prerequisites: BTreeSet<String>,
    pub lifetime_effects: BTreeSet<crate::loop_lifetime_summary::ForwardedLifetime>,
    pub lifetime_roots: BTreeSet<String>,
    /// Declarations in force at the consultation.
    pub declarations: BTreeSet<String>,
    /// Declarations held (awaiting review, or a pin mismatch).
    pub held: BTreeSet<String>,
    /// Declarations withdrawn in this unit or at this call site.
    pub withdrawn: BTreeSet<String>,
    /// In-unit summaries consulted, transitively.
    pub summaries: BTreeSet<String>,
    /// Card NULL-SAFETY-PROPERTY (rule 14n): the named null premises the
    /// row's work and its in-unit summaries' work relied on, transitively.
    pub null_premises: BTreeSet<String>,
}

#[derive(Debug, Default)]
struct Recorder {
    record: UnitRecord,
    /// Depth of [`hypothetical`] analyses running now.
    hypothetical: usize,
}

thread_local! {
    static RECORDER: RefCell<Option<Recorder>> = const { RefCell::new(None) };
}

fn recording() -> bool {
    RECORDER.with(|recorder| recorder.borrow().is_some())
}

fn with_recorder(change: impl FnOnce(&mut Recorder)) {
    RECORDER.with(|recorder| {
        if let Some(recorder) = recorder.borrow_mut().as_mut() {
            change(recorder);
        }
    });
}

/// Evidence is kept only outside a hypothetical analysis.
fn with_evidence(change: impl FnOnce(&mut UnitRecord)) {
    with_recorder(|recorder| {
        if recorder.hypothetical == 0 {
            change(&mut recorder.record);
        }
    });
}

/// One unit's recording, opened by the audit around the unit's analysis.
#[must_use = "a recording is read with `finish`"]
pub struct Recording {
    enclosing_log: Option<Option<Consulted>>,
    previous: Option<Recorder>,
}

impl Recording {
    /// Open a recording on this thread.
    pub fn start() -> Self {
        let previous = RECORDER.with(|recorder| recorder.borrow_mut().replace(Recorder::default()));
        Self {
            enclosing_log: Some(contract_consultation::open()),
            previous,
        }
    }

    /// Close the recording and return what it recorded. The names looked up
    /// outside every function's work (the unit's own passes) are not a row's.
    pub fn finish(mut self) -> UnitRecord {
        self.restore().record
    }

    fn restore(&mut self) -> Recorder {
        if let Some(enclosing) = self.enclosing_log.take() {
            let _unit_level = contract_consultation::close(enclosing);
        }
        let previous = self.previous.take();
        RECORDER
            .with(|recorder| std::mem::replace(&mut *recorder.borrow_mut(), previous))
            .unwrap_or_default()
    }
}

impl Drop for Recording {
    fn drop(&mut self) {
        if self.enclosing_log.is_some() {
            let _ = self.restore();
        }
    }
}

/// Close a function's log when its work ends, early returns and unwinding
/// included.
struct Scope {
    function: String,
    enclosing: Option<Option<Consulted>>,
    /// Card ARM-OBLIGATIONS-UNIT-GRAIN: the function was pushed on the
    /// attribution's active stack.
    active: bool,
}

impl Drop for Scope {
    fn drop(&mut self) {
        let Some(enclosing) = self.enclosing.take() else {
            return;
        };
        let mine = contract_consultation::close(enclosing);
        contract_consultation::absorb(&mine);
        let function = std::mem::take(&mut self.function);
        if self.active {
            attribution::work(&function, &mine);
            attribution::leave();
        }
        with_recorder(|recorder| {
            recorder
                .record
                .functions
                .entry(function)
                .or_default()
                .absorb(&mine);
        });
    }
}

/// Run `work` as `function`'s: every lookup it makes is noted under that
/// function. The identity when no recording and no attribution is open.
pub(crate) fn within<R>(function: &str, work: impl FnOnce() -> R) -> R {
    let active = attribution::open();
    if !recording() && !active {
        return work();
    }
    if active {
        attribution::enter(function);
    }
    let _scope = Scope {
        function: function.to_owned(),
        enclosing: Some(contract_consultation::open()),
        active,
    };
    work()
}

/// Record the identifiers of `function`'s body, the braced block opening at
/// `start` in `tokens`. Nothing without a recording or an attribution.
pub(crate) fn body(function: &str, tokens: &[crate::legacy_c::Token], start: usize) {
    let active = attribution::open();
    if !recording() && !active {
        return;
    }
    let mut depth = 0usize;
    let mut names = BTreeSet::new();
    for token in tokens.iter().skip(start) {
        match token.text.as_str() {
            "{" => depth += 1,
            "}" => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    break;
                }
            }
            text if text
                .chars()
                .next()
                .is_some_and(|first| first == '_' || first.is_ascii_alphabetic())
                && !names.contains(text) =>
            {
                names.insert(text.to_owned());
            }
            _ => {}
        }
    }
    if active {
        attribution::body(function, &names);
    }
    with_recorder(|recorder| {
        recorder
            .record
            .bodies
            .entry(function.to_owned())
            .or_default()
            .extend(names);
    });
}

/// Run `work` as an analysis under hypothetical holds: its lookups are still
/// noted (over-approximation), its body evidence is not.
pub(crate) fn hypothetical<R>(work: impl FnOnce() -> R) -> R {
    if !recording() {
        return work();
    }
    with_recorder(|recorder| recorder.hypothetical += 1);
    struct Leave;
    impl Drop for Leave {
        fn drop(&mut self) {
            with_recorder(|recorder| recorder.hypothetical -= 1);
        }
    }
    let _leave = Leave;
    work()
}

/// Card DECLARED-ASSUMPTION-BODY-CHECKS (rule 13n): whether body
/// evidence is being collected now: a recording is open and no hypothetical
/// analysis is running. An evidence-only check runs only then.
pub(crate) fn collecting() -> bool {
    RECORDER.with(|recorder| {
        recorder
            .borrow()
            .as_ref()
            .is_some_and(|recorder| recorder.hypothetical == 0)
    })
}

/// Card DECLARED-ASSUMPTION-BODY-CHECKS (rule 13n): run `work` with the
/// recording suspended and a fresh consultation log that is discarded, so an
/// evidence-only check adds no consultation, body spelling or evidence to any
/// row or function of the unit. Its result is recorded by the caller.
pub(crate) fn unrecorded<R>(work: impl FnOnce() -> R) -> R {
    struct Resume {
        recorder: Option<Recorder>,
        log: Option<Option<Consulted>>,
    }
    impl Drop for Resume {
        fn drop(&mut self) {
            if let Some(enclosing) = self.log.take() {
                let _discarded = contract_consultation::close(enclosing);
            }
            let recorder = self.recorder.take();
            RECORDER.with(|current| *current.borrow_mut() = recorder);
        }
    }
    let recorder = RECORDER.with(|current| current.borrow_mut().take());
    let _resume = Resume {
        recorder,
        log: Some(contract_consultation::open()),
    };
    work()
}

/// Card **CURRENT-RUN-WORKER-SPIN** (rule 4c, amended): run `work` in a
/// fresh recording and return what it recorded, to be written into later
/// recordings by [`absorb_record`] exactly as running `work` there would
/// have written it.
///
/// The fresh recording's primary stream is a digest no token stream has, so
/// a reading site `work` analyses is omitted as a foreign stream, as it is
/// when `work` runs inside a unit's recording whose primary stream is
/// another unit's. Its unit-level consultations are discarded, as a unit's
/// are when its recording finishes.
pub(crate) fn captured<R>(work: impl FnOnce() -> R) -> (R, UnitRecord) {
    let recording = Recording::start();
    with_recorder(|recorder| {
        let foreign = "cross-unit-home-replay".to_owned();
        recorder.record.primary_source_sha256 = Some(foreign.clone());
        recorder.record.primary_token_sha256 = Some(foreign);
    });
    let result = work();
    let mut record = recording.finish();
    record.primary_source_sha256 = None;
    record.primary_token_sha256 = None;
    (result, record)
}

/// Write a [`captured`] record into the open recording: every log and set
/// is joined, every count added, an observation kept once, and a unit fact
/// (`callback_members`, `constructed_states`) replaced, as the recording
/// functions above write them. Evidence is written only outside a
/// hypothetical analysis; the primary stream is never replaced. Nothing
/// without an open recording.
pub(crate) fn absorb_record(delta: &UnitRecord) {
    with_recorder(|recorder| {
        let record = &mut recorder.record;
        for (function, consulted) in &delta.functions {
            record
                .functions
                .entry(function.clone())
                .or_default()
                .absorb(consulted);
        }
        for (function, names) in &delta.bodies {
            record
                .bodies
                .entry(function.clone())
                .or_default()
                .extend(names.iter().cloned());
        }
        record.inferred.extend(delta.inferred.iter().cloned());
        if recorder.hypothetical != 0 {
            return;
        }
        for observation in &delta.reading_sites {
            if !record.reading_sites.contains(observation) {
                record.reading_sites.push(observation.clone());
            }
        }
        for (digest, bytes) in &delta.reading_environments {
            record
                .reading_environments
                .entry(digest.clone())
                .or_insert_with(|| bytes.clone());
        }
        for (reason, count) in &delta.reading_site_omissions {
            *record
                .reading_site_omissions
                .entry(reason.clone())
                .or_default() += count;
        }
        let join = |into: &mut BTreeMap<String, BTreeSet<String>>,
                    from: &BTreeMap<String, BTreeSet<String>>| {
            for (name, values) in from {
                into.entry(name.clone())
                    .or_default()
                    .extend(values.iter().cloned());
            }
        };
        join(&mut record.lifetime_members, &delta.lifetime_members);
        join(
            &mut record.lifetime_prerequisites,
            &delta.lifetime_prerequisites,
        );
        join(&mut record.validated, &delta.validated);
        join(&mut record.withdrawn, &delta.withdrawn);
        join(&mut record.failed, &delta.failed);
        record.examined.extend(delta.examined.iter().cloned());
        if let Some(facts) = &delta.callback_members {
            record.callback_members = Some(facts.clone());
        }
        if let Some(facts) = &delta.constructed_states {
            record.constructed_states = Some(facts.clone());
        }
    });
}

/// Card HANDOFF-CANDIDATE-OWN-CALLBACKS: note that the current work relied on
/// the declaration `name` through a decided callback invocation (the body
/// need not spell it). Card ARM-OBLIGATIONS-UNIT-GRAIN: the note also reaches
/// every open attribution for every function whose work is running, where a
/// declining probe's rollback of the consultation log cannot erase it.
pub(crate) fn relied(name: &str) {
    contract_consultation::note_relied(name);
    attribution::relied(name);
    // Card MEASURE-LOOKUP-RECORD-REUSE: the unit's lookup record (evidence).
    crate::lookup_record::relied(name);
}

/// Card **ARM-OBLIGATIONS-UNIT-GRAIN**: which rows of a unit reach a
/// declaration, for the carriage of a rule package admission's obligations
/// (`crate::arm_obligations`).
///
/// While a unit frame is open (`crate::arm_obligations::Frame`), every
/// function's work ([`within`]) and every body a CFG build reads ([`body`])
/// is noted in the frame's [`Attribution`], in **every** run: the audit's
/// [`Recording`] is neither needed nor read, so a row's carriage is the same
/// whether a recording is open or not (the certificate recheck reruns the
/// analysis outside one). The sources are rule 13f's: a function's own name,
/// every identifier its body spells in a stream a build of it read (minted
/// and held spellings read back to their declaration), every callee its row's
/// solved CFG applies, and every decided callback candidate its work relied
/// on. Unlike rule 13f's set the closure follows every in-unit function that
/// has work in the frame (an inferred summary, a standing checked
/// specification, a declaration withdrawn here), and a function with work but
/// no recorded body contributes every name its work looked up, unfiltered.
pub(crate) mod attribution {
    use std::cell::RefCell;
    use std::collections::{BTreeMap, BTreeSet};

    use qcore_schemas::contract::ResourceContracts;
    use qcore_schemas::contract_consultation::{self, Consulted};

    use super::{Spelling, read};

    /// One unit frame's notes.
    #[derive(Clone, Debug, Default, PartialEq, Eq)]
    pub(crate) struct Attribution {
        /// Per function, every name its work looked up.
        pub(crate) work: BTreeMap<String, Consulted>,
        /// Per function, every identifier its body spells in a stream a CFG
        /// build of it read.
        pub(crate) bodies: BTreeMap<String, BTreeSet<String>>,
        /// Per function, every decided callback candidate its work relied
        /// on, never rolled back.
        pub(crate) relied: BTreeMap<String, BTreeSet<String>>,
    }

    thread_local! {
        static STACK: RefCell<Vec<Attribution>> = const { RefCell::new(Vec::new()) };
        static ACTIVE: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
    }

    /// Whether a unit frame's attribution is open on this thread.
    pub(crate) fn open() -> bool {
        STACK.with(|stack| !stack.borrow().is_empty())
    }

    /// Open one unit frame's attribution; its depth closes it.
    pub(crate) fn start() -> usize {
        STACK.with(|stack| {
            let mut stack = stack.borrow_mut();
            stack.push(Attribution::default());
            stack.len()
        })
    }

    /// Close the attribution opened at `depth` (and any left open inside it
    /// by an unwind) and return its notes.
    pub(crate) fn finish(depth: usize) -> Attribution {
        STACK.with(|stack| {
            let mut stack = stack.borrow_mut();
            let mut mine = Attribution::default();
            while stack.len() >= depth {
                let Some(top) = stack.pop() else {
                    break;
                };
                if stack.len() + 1 == depth {
                    mine = top;
                }
            }
            mine
        })
    }

    pub(super) fn enter(function: &str) {
        ACTIVE.with(|active| active.borrow_mut().push(function.to_owned()));
    }

    pub(super) fn leave() {
        ACTIVE.with(|active| {
            active.borrow_mut().pop();
        });
    }

    fn with_top(change: impl FnOnce(&mut Attribution)) {
        STACK.with(|stack| {
            if let Some(top) = stack.borrow_mut().last_mut() {
                change(top);
            }
        });
    }

    pub(super) fn work(function: &str, consulted: &Consulted) {
        with_top(|top| {
            top.work
                .entry(function.to_owned())
                .or_default()
                .absorb(consulted);
        });
    }

    pub(super) fn body(function: &str, names: &BTreeSet<String>) {
        with_top(|top| {
            top.bodies
                .entry(function.to_owned())
                .or_default()
                .extend(names.iter().cloned());
        });
    }

    pub(super) fn relied(name: &str) {
        let active = ACTIVE.with(|active| active.borrow().clone());
        if active.is_empty() {
            return;
        }
        STACK.with(|stack| {
            for attribution in stack.borrow_mut().iter_mut() {
                for function in &active {
                    attribution
                        .relied
                        .entry(function.clone())
                        .or_default()
                        .insert(name.to_owned());
                }
            }
        });
    }

    /// The names `function`'s derivation consulted directly, read back to
    /// their declarations: its own name, its body's spellings (every name
    /// its work looked up when no body was recorded), its relied-on
    /// candidates and `callees` (its row's applied callees).
    fn direct(
        attribution: &Attribution,
        function: &str,
        callees: Option<&BTreeSet<String>>,
        environment: &ResourceContracts,
    ) -> BTreeSet<String> {
        let mut spellings = BTreeSet::new();
        let work = attribution.work.get(function);
        match attribution.bodies.get(function) {
            Some(body) => spellings.extend(body.iter().cloned()),
            None => {
                if let Some(work) = work {
                    spellings.extend(work.found.iter().chain(&work.missed).cloned());
                }
            }
        }
        if let Some(work) = work {
            spellings.extend(work.relied.iter().cloned());
        }
        if let Some(relied) = attribution.relied.get(function) {
            spellings.extend(relied.iter().cloned());
        }
        if let Some(callees) = callees {
            spellings.extend(callees.iter().cloned());
        }
        let mut names = BTreeSet::new();
        names.insert(function.to_owned());
        for spelling in spellings {
            match read(&spelling, environment) {
                Spelling::Plain(name) | Spelling::Held(name) | Spelling::Withdrawn(name) => {
                    names.insert(name);
                }
                Spelling::Other => {}
            }
        }
        names
    }

    /// For every name of `targets`, the functions whose derivation reaches
    /// it: the function itself, and every function with work in the frame or
    /// a row of `rows` (with its applied callees) whose direct names contain
    /// a function that reaches it, transitively.
    #[must_use]
    pub(crate) fn reaching(
        attribution: &Attribution,
        rows: &BTreeMap<String, BTreeSet<String>>,
        environment: &ResourceContracts,
        targets: &BTreeSet<String>,
    ) -> BTreeMap<String, BTreeSet<String>> {
        // Reading a spelling back asks the environment; keep those lookups
        // out of every enclosing log (rule 13f evidence is unchanged).
        let enclosing = contract_consultation::open();
        let nodes = attribution
            .work
            .keys()
            .chain(attribution.bodies.keys())
            .chain(rows.keys())
            .cloned()
            .collect::<BTreeSet<_>>();
        let mut callers: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
        for node in &nodes {
            for name in direct(attribution, node, rows.get(node), environment) {
                callers.entry(name).or_default().insert(node.clone());
            }
        }
        let _discarded = contract_consultation::close(enclosing);
        targets
            .iter()
            .map(|target| {
                let mut reached = BTreeSet::new();
                let mut queue = vec![target.clone()];
                while let Some(name) = queue.pop() {
                    for caller in callers.get(&name).into_iter().flatten() {
                        if reached.insert(caller.clone()) {
                            queue.push(caller.clone());
                        }
                    }
                }
                (target.clone(), reached)
            })
            .collect()
    }
}

/// `name` was given an in-unit summary.
pub(crate) fn inferred(name: &str) {
    with_recorder(|recorder| {
        recorder.record.inferred.insert(name.to_owned());
    });
}

/// The unit's derivation scope holds the body of the declared `name`.
pub(crate) fn examined(name: &str) {
    with_evidence(|record| {
        record.examined.insert(name.to_owned());
    });
}

/// `name`'s body validated its declaration here, by `by`.
pub(crate) fn validated(name: &str, by: &str) {
    with_evidence(|record| {
        record
            .validated
            .entry(name.to_owned())
            .or_default()
            .insert(by.to_owned());
    });
}

/// `name`'s declaration was withdrawn from this unit's call sites for `why`;
/// `body` says whether the reason is the body's failing a check.
pub(crate) fn withdrawn(name: &str, why: &str, body: bool) {
    with_evidence(|record| {
        record
            .withdrawn
            .entry(name.to_owned())
            .or_default()
            .insert(why.to_owned());
        if body {
            record
                .failed
                .entry(name.to_owned())
                .or_default()
                .insert(why.to_owned());
        }
    });
}

/// Whether `environment` declares `name` in force (any of its three tables).
fn in_force(environment: &ResourceContracts, name: &str) -> bool {
    environment.functions().contains_key(name)
        || environment.relational().contains_key(name)
        || environment.retaining().contains_key(name)
        || environment.guarded_retaining().contains_key(name)
        || environment.lifetime_summary(name).is_some()
}

/// Whether `environment` holds `name` for a human act.
fn held(environment: &ResourceContracts, name: &str) -> bool {
    environment.pending_review().contains_key(name) || environment.pin_mismatch().contains_key(name)
}

/// Whether `environment` declares `name` at all.
pub fn declared(environment: &ResourceContracts, name: &str) -> bool {
    in_force(environment, name) || held(environment, name)
}

/// How a consulted spelling reads.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Spelling {
    /// A plain name, or a minted name standing for the declaration `0`.
    Plain(String),
    /// A call site held for a human act.
    Held(String),
    /// A call site whose declaration is withdrawn there.
    Withdrawn(String),
    /// Nothing a declaration answers for.
    Other,
}

/// The declared name after `rest`'s first `__`-separated prefix that leaves
/// one, or `rest` itself when it is declared.
fn declared_suffix(rest: &str, environment: &ResourceContracts) -> Option<String> {
    if declared(environment, rest) {
        return Some(rest.to_owned());
    }
    let mut from = 0;
    while let Some(at) = rest[from..].find("__") {
        let candidate = &rest[from + at + 2..];
        if declared(environment, candidate) {
            return Some(candidate.to_owned());
        }
        from += at + 1;
    }
    None
}

fn read(spelling: &str, environment: &ResourceContracts) -> Spelling {
    use crate::legacy_c::{
        declared_abi, declared_modes, literal_premise, pin_hold, refuted_contract, review_hold,
    };
    let held = [review_hold::HELD_PREFIX, pin_hold::HELD_PREFIX];
    let withdrawn = [
        refuted_contract::HELD_PREFIX,
        crate::legacy_c::caller_frame_hold::HELD_PREFIX,
        crate::legacy_c::conflict_hold::HELD_PREFIX,
        declared_abi::WITHDRAWN_PREFIX,
        declared_modes::WITHDRAWN_PREFIX,
        literal_premise::UNMET_PREFIX,
    ];
    let minted = [
        crate::relational_post::CLASS_PREFIX,
        crate::retained_by::RETAINED_PREFIX,
        crate::legacy_c::member_effect::MINTED_PREFIX,
    ];
    for prefix in held {
        if let Some(rest) = spelling.strip_prefix(prefix) {
            return declared_suffix(rest, environment).map_or(Spelling::Other, Spelling::Held);
        }
    }
    for prefix in withdrawn {
        if let Some(rest) = spelling.strip_prefix(prefix) {
            return declared_suffix(rest, environment).map_or(Spelling::Other, Spelling::Withdrawn);
        }
    }
    for prefix in minted {
        if let Some(rest) = spelling.strip_prefix(prefix) {
            return declared_suffix(rest, environment)
                .map_or_else(|| Spelling::Plain(rest.to_owned()), Spelling::Plain);
        }
    }
    if spelling.starts_with("qcore_") || spelling.starts_with("__qcore_") {
        return Spelling::Other;
    }
    Spelling::Plain(spelling.to_owned())
}

/// Whether a consulted `spelling` is one of the function's own: the body
/// spells it, or it is a minted name whose declaration the body spells.
fn spoken(spelling: &str, body: &BTreeSet<String>, environment: &ResourceContracts) -> bool {
    if body.contains(spelling) {
        return true;
    }
    match read(spelling, environment) {
        Spelling::Plain(name) | Spelling::Held(name) | Spelling::Withdrawn(name) => {
            body.contains(&name)
        }
        Spelling::Other => false,
    }
}

/// Every callee an instruction of `cfg` applies a contract of.
#[must_use]
pub fn cfg_callees(cfg: &LegacyCfg) -> BTreeSet<String> {
    fn visit(instruction: &LegacyInstruction, out: &mut BTreeSet<String>) {
        match instruction {
            LegacyInstruction::ContractCall {
                function, nested, ..
            } => {
                out.insert(function.clone());
                out.extend(nested.iter().map(|call| call.function.clone()));
            }
            LegacyInstruction::ElementStore { call, fallback, .. } => {
                if let Some(call) = call {
                    visit(call, out);
                }
                visit(fallback, out);
            }
            LegacyInstruction::ElementRelease {
                callee, fallback, ..
            } => {
                out.insert(callee.clone());
                visit(fallback, out);
            }
            _ => {}
        }
    }
    let mut out = BTreeSet::new();
    for node in &cfg.nodes {
        visit(&node.instruction, &mut out);
    }
    out
}

/// The dependencies of every row of `summary`, from the unit's `record`,
/// classified against `environment`: the environment the unit was analysed
/// under (rule 13f).
#[must_use]
pub fn row_dependencies(
    record: &UnitRecord,
    summary: &LegacyProgramSummary,
    environment: &ResourceContracts,
) -> BTreeMap<String, RowDependencies> {
    // Every spelling a function's work consulted: its log, its row's CFG
    // and its own name (its declaration is its entry contract).
    let empty = BTreeSet::new();
    let direct = |function: &str| {
        let mut spellings = BTreeSet::new();
        spellings.insert(function.to_owned());
        let body = record.bodies.get(function).unwrap_or(&empty);
        if let Some(consulted) = record.functions.get(function) {
            spellings.extend(
                consulted
                    .found
                    .iter()
                    .chain(&consulted.missed)
                    .filter(|spelling| spoken(spelling, body, environment))
                    .cloned(),
            );
            // Card HANDOFF-CANDIDATE-OWN-CALLBACKS: a decided callback
            // invocation's candidates are the row's, spelled or not.
            spellings.extend(consulted.relied.iter().cloned());
        }
        if let Some(row) = summary.functions.get(function) {
            spellings.extend(cfg_callees(&row.cfg));
        }
        spellings
    };
    summary
        .functions
        .keys()
        .map(|function| {
            let mut dependencies = RowDependencies::default();
            let mut queue = vec![function.clone()];
            let mut visited = BTreeSet::new();
            while let Some(current) = queue.pop() {
                if !visited.insert(current.clone()) {
                    continue;
                }
                // Card NULL-SAFETY-PROPERTY (rule 14n): its null premises,
                // and transitively every in-unit summary's.
                if let Some(consulted) = record.functions.get(&current) {
                    dependencies
                        .null_premises
                        .extend(consulted.null_premises.iter().cloned());
                }
                if let Some(members) = record.lifetime_members.get(&current) {
                    dependencies
                        .lifetime_members
                        .extend(members.iter().cloned());
                }
                if let Some(prerequisites) = record.lifetime_prerequisites.get(&current) {
                    dependencies
                        .lifetime_prerequisites
                        .extend(prerequisites.iter().cloned());
                }
                if let Some(row) = summary.functions.get(&current) {
                    dependencies
                        .lifetime_effects
                        .extend(row.lifetime_effects.iter().cloned());
                    for effect in &row.lifetime_effects {
                        if let crate::loop_lifetime_summary::Root::Formal { scope, .. } =
                            &effect.registration.object.root
                        {
                            dependencies.lifetime_roots.insert(scope.clone());
                        }
                    }
                    dependencies.null_premises.extend(
                        row.null_premises
                            .iter()
                            .map(|premise| premise.name(&current)),
                    );
                }
                for spelling in direct(&current) {
                    match read(&spelling, environment) {
                        Spelling::Held(name) => {
                            dependencies.held.insert(name);
                        }
                        Spelling::Withdrawn(name) => {
                            dependencies.withdrawn.insert(name);
                        }
                        Spelling::Plain(name) => {
                            if held(environment, &name) {
                                dependencies.held.insert(name.clone());
                            } else if in_force(environment, &name) {
                                // A refuted declaration still governs its own
                                // body; every other call site of it is held.
                                let refuted = name != *function
                                    && environment.refuted_by_body().contains_key(&name);
                                if refuted || record.withdrawn.contains_key(&name) {
                                    dependencies.withdrawn.insert(name.clone());
                                } else {
                                    dependencies.declarations.insert(name.clone());
                                }
                            }
                            // A declaration withdrawn here may be replaced by
                            // an inference; either way an in-unit summary's
                            // own dependencies are this row's.
                            if record.inferred.contains(&name) && name != *function {
                                dependencies.summaries.insert(name.clone());
                                queue.push(name);
                            }
                        }
                        Spelling::Other => {}
                    }
                }
            }
            (function.clone(), dependencies)
        })
        .collect()
}

/// `name`'s body failed a check here for `why`, without its declaration
/// being withdrawn (a dropped guard, a summary conflict).
pub(crate) fn failed(name: &str, why: &str) {
    with_evidence(|record| {
        record
            .failed
            .entry(name.to_owned())
            .or_default()
            .insert(why.to_owned());
    });
}

/// Every declaration in force in `before` and no longer in `after`, as
/// withdrawn here for `why` (a body failure when `body`).
pub(crate) fn withdrawn_between(
    before: &ResourceContracts,
    after: &ResourceContracts,
    why: &str,
    body: bool,
) {
    if !recording() {
        return;
    }
    let names = before
        .functions()
        .keys()
        .chain(before.relational().keys())
        .chain(before.retaining().keys())
        .chain(before.guarded_retaining().keys())
        .filter(|name| !in_force(after, name))
        .cloned()
        .collect::<BTreeSet<_>>();
    for name in names {
        withdrawn(&name, why, body);
    }
}

/// The body evidence of one unit's final derivation (rule 13g): `input` is
/// the environment the unit was analysed under, `scope` its derivation scope,
/// `derivation` the derivation its rows were analysed over, `mode` the names
/// the mode gate withdrew and `specifications` the withdrawn checked
/// specifications. Card DECLARED-ASSUMPTION-BODY-CHECKS (rule 13m):
/// `declared_entry` is what the declared-entry check established in this unit.
pub(crate) fn unit_evidence(
    input: &ResourceContracts,
    scope: &[String],
    derivation: &crate::legacy_summary::CustodyDerivation,
    mode: &[String],
    specifications: &BTreeMap<String, crate::legacy_c::checked_spec::Withdrawal>,
    declared_entry: &crate::legacy_c::declared_entry_check::DeclaredEntry,
) {
    if !recording() {
        return;
    }
    for name in input.refuted_by_body().keys() {
        withdrawn(name, "C06.4", true);
    }
    for name in mode {
        withdrawn(name, "mode", true);
    }
    for (name, why) in specifications {
        match why {
            crate::legacy_c::checked_spec::Withdrawal::NoBody => {
                withdrawn(name, "C06.3-no-body", false);
            }
            _ => withdrawn(name, "C06.3", true),
        }
    }
    let (confirmed, contradicted) = derivation.relational_evidence();
    for name in confirmed {
        if input.relational().contains_key(name) {
            validated(name, "11r");
        }
    }
    for name in contradicted {
        withdrawn(name, "11r", true);
    }
    let environment = derivation.environment();
    for name in scope {
        if !in_force(input, name) {
            continue;
        }
        examined(name);
        let Some(declared) = input.functions().get(name) else {
            continue;
        };
        if environment.functions().get(name) != Some(declared) {
            continue;
        }
        // A conflict is a failure; no conflict with a summary means the
        // validated summary is the declaration's custody projection. Card
        // VERDICT-BODY-STANDING-CLAUSE-SCOPE (rule 13i): W4 covers exactly the
        // clauses that projection compares, so a container or counted-user
        // clause beside them stays uncovered (`contract_clause_standing::covers`).
        // Card DECLARED-ASSUMPTION-BODY-CHECKS (rule 13m): a conflict the
        // declaration's own entry check settled is `13m` evidence instead.
        use crate::legacy_c::declared_entry_check::{Recorded, recorded};
        match recorded(
            derivation.conflict(name).is_some(),
            derivation.inferred(name).is_some(),
            declared_entry.refinements.contains(name),
        ) {
            Recorded::Failed => failed(name, "conflict"),
            Recorded::Evidence(by) => validated(name, by),
            Recorded::Nothing => {}
        }
    }
}

/// Lift the actual declaration/summary consultation closure to its atom IDs.
/// This adds no lookup and is never read by the analyzer.
pub fn hypothesis_closure(
    dependencies: &RowDependencies,
    graphs: &BTreeMap<String, qcore_schemas::hypothesis::Graph>,
) -> BTreeMap<String, BTreeSet<String>> {
    dependencies
        .declarations
        .iter()
        .chain(&dependencies.held)
        .chain(&dependencies.withdrawn)
        .filter_map(|name| {
            graphs
                .get(name)
                .map(|graph| (name.clone(), graph.atoms.keys().cloned().collect()))
        })
        .filter(|(_, atoms): &(_, BTreeSet<String>)| !atoms.is_empty())
        .collect()
}

/// Record the primary stream before cross-unit replay can enter another one.
pub(crate) fn primary_source(source: &str) {
    with_recorder(|r| {
        if r.record.primary_source_sha256.is_none() {
            r.record.primary_source_sha256 = Some(qcore_schemas::sha256_hex(source.as_bytes()));
            r.record.primary_token_sha256 = Some(crate::reading_site_evidence::token_digest(
                &crate::legacy_c::tokenize(source),
            ));
        }
    });
}

/// Probe-local capture state, restored together with consultation state.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ReadingCheckpoint {
    length: usize,
    omissions: BTreeMap<String, usize>,
}

pub(crate) fn reading_checkpoint() -> Option<ReadingCheckpoint> {
    RECORDER.with(|r| {
        r.borrow().as_ref().map(|r| ReadingCheckpoint {
            length: r.record.reading_sites.len(),
            omissions: r.record.reading_site_omissions.clone(),
        })
    })
}

pub(crate) fn restore_reading_checkpoint(saved: &Option<ReadingCheckpoint>) {
    if let Some(saved) = saved {
        with_recorder(|r| {
            r.record.reading_sites.truncate(saved.length);
            let kept = r
                .record
                .reading_sites
                .iter()
                .map(|observation| observation.environment_sha256.as_str())
                .collect::<BTreeSet<_>>();
            r.record
                .reading_environments
                .retain(|sha, _| kept.contains(sha.as_str()));
            r.record.reading_site_omissions.clone_from(&saved.omissions);
        });
    }
}

/// Instantiation can refuse after classification. Keep captures only when its
/// final acceptance succeeds, independently of the caller's wider probe.
pub(crate) struct ReadingTransaction(Option<ReadingCheckpoint>, bool);
impl ReadingTransaction {
    pub(crate) fn start() -> Self {
        Self(reading_checkpoint(), false)
    }
    pub(crate) fn commit(&mut self) {
        self.1 = true;
    }
}
impl Drop for ReadingTransaction {
    fn drop(&mut self) {
        if !self.1 {
            restore_reading_checkpoint(&self.0);
        }
    }
}

pub(crate) fn reading_primary() -> Option<(String, String)> {
    RECORDER.with(|r| {
        let r = r.borrow();
        let r = r.as_ref()?;
        Some((
            r.record.primary_source_sha256.clone()?,
            r.record.primary_token_sha256.clone()?,
        ))
    })
}

pub(crate) fn reading_omitted(reason: &str) {
    with_evidence(|r| *r.reading_site_omissions.entry(reason.into()).or_default() += 1);
}

pub(crate) fn reading_observed(
    observation: crate::reading_site_evidence::Observation,
    canonical_environment: String,
) {
    with_evidence(|r| {
        if qcore_schemas::sha256_hex(canonical_environment.as_bytes())
            != observation.environment_sha256
        {
            *r.reading_site_omissions
                .entry("effective-environment-digest-mismatch".into())
                .or_default() += 1;
            return;
        }
        r.reading_environments
            .entry(observation.environment_sha256.clone())
            .or_insert(canonical_environment);
        if !r.reading_sites.contains(&observation) {
            r.reading_sites.push(observation);
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Card DECLARED-ASSUMPTION-BODY-CHECKS (rule 13n): work run unrecorded
    /// adds no consultation, body spelling or evidence anywhere, and the
    /// recording resumes afterwards.
    #[test]
    fn unrecorded_work_leaves_no_trace() {
        assert!(!collecting());
        let recording = Recording::start();
        assert!(collecting());
        within("f", || {
            contract_consultation::note("before", true);
            unrecorded(|| {
                assert!(!collecting());
                contract_consultation::note("hidden", true);
                within("g", || contract_consultation::note("nested", true));
                validated("hidden", "13m");
                examined("hidden");
            });
            contract_consultation::note("after", true);
        });
        hypothetical(|| assert!(!collecting()));
        let record = recording.finish();
        let f = &record.functions["f"];
        assert!(f.found.contains("before") && f.found.contains("after"));
        assert!(!f.found.contains("hidden") && !f.found.contains("nested"));
        assert!(!record.functions.contains_key("g"));
        assert!(record.validated.is_empty() && record.examined.is_empty());
        assert!(!collecting());
    }
}
