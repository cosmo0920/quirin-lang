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

//! Condition-variable publication and join-before-release over one program
//! (card **W47-CONDVAR-JOIN-PUBLICATION**, C4–C5, gateway rule 10q, contract
//! specification §C04.10).
//!
//! [`qcore_c_gateway::condvar_census::read_program`] reads the files
//! lexically; this adapter derives the thread contexts with the loop-owner
//! census's kernel program ([`crate::loop_owner_census::thread_program`] and
//! [`qcore_core::loop_owner::derive`], reused rather than duplicated), pairs
//! every condvar with its mutex, reads each condvar's predicate and results,
//! and decides every site with [`qcore_core::condvar_join`]. It lives here, not
//! in the gateway, because the gateway depends on no kernel crate.
//!
//! Card **W47-CONDVAR-JOIN-STOP-PATHS** adds three decisions, each the
//! kernel's: a null test's null arm is dropped only where
//! `null_branch_excluded` admits the reader's evidence (the decider passed to
//! [`qcore_c_gateway::condvar_census::read_program_with`]); an unchecked first
//! join carries its four POSIX exclusions ([`ThreadFacts`], derived from the
//! program-wide thread sites and the loop-owner contexts) and counts only
//! where `join_succeeds` admits them; and every thread a live retaining
//! thread starts is decided from that thread's own entry run to its exit
//! ([`qcore_core::condvar_join::DescendantJoin`]).

use std::collections::{BTreeMap, BTreeSet};

use qcore_c_gateway::condvar_census::{
    Fact, NullTestReading, ReadAccess, ReadHeld, ReadJoin, ReadLoopBody, ReadRefusal,
    ReadSectionEvent, ReadSite, ReleaseReading, SyncReading, SyncSite, SyncTables,
    read_program_lifecycle,
};
use qcore_c_gateway::thread_entry_census::{CensusFile, HopTables};
use qcore_core::condvar_join::{
    Absence, DescendantEvidence, DescendantJoin, Held, JoinEvidence, JoinExclusions, KeptRefusal,
    LiveThread, LoopBody, NullBranchEvidence, Pairing, Phase, PredicateLoop, PublishSite,
    ReadLicence, ReleaseSite, SectionEvent, SignalSite, SyncVerdict, WaitSite, join_succeeds,
    null_branch_excluded, publication_verdict, quiescence_verdict, read_verdict, release_premises,
    release_verdict, signal_verdict, wait_verdict,
};
use qcore_core::loop_owner::{Derived, ThreadProgram, derive};

pub use qcore_c_gateway::condvar_census::FLUENT_BIT_SYNC;

/// The answer at one C4 site.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SiteVerdict {
    pub file: String,
    pub line: usize,
    pub function: String,
    pub callee: String,
    /// `wait`, `signal`, `init`, `destroy`, `read` or `refused`.
    pub kind: &'static str,
    /// The condvar, or the field read.
    pub object: Option<String>,
    pub verdict: SyncVerdict,
}

/// The answer at one C5 release.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReleaseVerdict {
    pub reading: ReleaseReading,
    pub site: ReleaseSite,
    pub verdict: SyncVerdict,
    /// The named premises the decision rests on beyond trust boundary 28's
    /// (card W47-CONDVAR-JOIN-STOP-PATHS).
    pub premises: BTreeSet<&'static str>,
    /// Card W47-FLB-LIB-LIFECYCLE: the paired-lifecycle preconditions some
    /// path reaching the release violated (its gate refuses the release).
    pub lifecycle_violations: BTreeSet<String>,
    /// Card W47-FLB-LIB-STRICT-PAIRING: the release decided apart on the
    /// paths that keep the lifecycle, when lifecycle roles were given.
    pub lifecycle_split: Option<LifecycleSplitVerdict>,
}

/// A release decided on the paths that conform to the strict pair (card
/// W47-FLB-LIB-STRICT-PAIRING). The release's own verdict is the join over
/// every path; this only says what the conforming paths alone decide.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LifecycleSplitVerdict {
    pub conforming_paths: usize,
    pub undetermined_paths: usize,
    pub refused_paths: usize,
    /// The verdict over the conforming paths (`None` when none conforms).
    pub conforming_verdict: Option<SyncVerdict>,
    /// The conforming paths' release site (their live threads, descendants
    /// and unresolved reasons) and the premises its decision names.
    pub conforming_site: Option<ReleaseSite>,
    pub conforming_premises: BTreeSet<&'static str>,
}

/// Which live threads a release concerns (binding premise P-RETAIN, stated):
/// the engine context (`flb_destroy`) is retained by every live thread; the
/// test's published storage and the fixture file only by the threads that run
/// the tail input's or the test's own callbacks, derived from the census
/// contexts of the functions defined under [`RETAINING_CODE`].
pub const RETAINING_CODE: &[&str] = &["plugins/in_tail/", "tests/runtime/in_tail.c"];

/// Releases whose storage every live thread retains.
pub const WHOLE_ENGINE_RELEASES: &[&str] = &["`flb_destroy`"];

/// The census of one program.
#[derive(Clone, Debug)]
pub struct Census {
    pub reading: SyncReading,
    pub program: ThreadProgram,
    pub derived: Vec<Derived>,
    pub pairing: Pairing,
    /// Per condvar: the loop predicates' keys and exit facts.
    pub predicates: BTreeMap<String, (BTreeSet<String>, BTreeSet<Fact>)>,
    /// Per condvar: the results its signalling sections write.
    pub results: BTreeMap<String, BTreeSet<String>>,
    pub sites: Vec<SiteVerdict>,
    pub releases: Vec<ReleaseVerdict>,
    /// The threads (plain entry names) that run code under [`RETAINING_CODE`].
    pub retainers: BTreeSet<String>,
    /// The program-wide thread facts the POSIX exclusions rest on.
    pub facts: ThreadFacts,
    /// Card W47-FLB-LIB-LIFECYCLE: the lifecycle decisions, when declared
    /// lifecycle obligations were given.
    pub lifecycle: Option<crate::lifecycle_census::LifecycleCensus>,
}

/// The kernel's input for one null test (card W47-CONDVAR-JOIN-STOP-PATHS).
#[must_use]
pub fn null_branch_evidence(test: &NullTestReading) -> NullBranchEvidence {
    NullBranchEvidence {
        dominating_dereference: test.dominating_dereference.clone(),
        local_defeat: test.local_defeat.clone(),
        base_type: test.base_type.clone(),
        stores: test.stores.clone(),
        unresolved: test.unresolved.clone(),
    }
}

/// One join or detach site: `(site, call, identity, the threads the enclosing
/// function may run on, whether some path to it is unresolved)`.
pub type IdSite = (
    String,
    &'static str,
    Result<BTreeSet<String>, String>,
    BTreeSet<String>,
    bool,
);

/// Program-wide facts about thread identifiers (card
/// W47-CONDVAR-JOIN-STOP-PATHS): which threads may run each join, detach and
/// `pthread_self` site, and which spawn wrappers create joinable threads.
#[derive(Clone, Debug, Default)]
pub struct ThreadFacts {
    /// `(site, call, identity, the threads the enclosing function may run
    /// on, whether some path to it is unresolved)`, for every reached site.
    pub id_sites: Vec<IdSite>,
    /// `(site, the threads it may run on, unresolved)` of every `pthread_self`
    /// whose value may be kept.
    pub published_self: Vec<(String, BTreeSet<String>, bool)>,
    /// Each wrapper's joinability.
    pub wrappers: BTreeMap<String, Result<String, String>>,
    /// The test-root threads (`T` and the roots' own labels).
    pub root_threads: BTreeSet<String>,
    /// Spawn wrappers' own internal entries (`mk_utils_worker_entry`).
    pub wrapper_entries: BTreeMap<String, String>,
}

impl ThreadFacts {
    fn new(
        reading: &SyncReading,
        program: &ThreadProgram,
        derived: &[Derived],
        tables: &SyncTables,
    ) -> Self {
        let id_of: BTreeMap<&str, usize> = program
            .functions
            .iter()
            .enumerate()
            .map(|(id, label)| (label.as_str(), id))
            .collect();
        let threads_of = |function: &str| -> Option<(BTreeSet<String>, bool)> {
            let id = *id_of.get(function)?;
            let threads: BTreeSet<String> = derived[id]
                .contexts
                .iter()
                .map(|c| plain(&c.thread).to_owned())
                .collect();
            let unresolved = !derived[id].unresolved.is_empty();
            // A function no root reaches and no unresolved path may reach
            // never runs in this program.
            (!threads.is_empty() || unresolved).then_some((threads, unresolved))
        };
        let mut id_sites = Vec::new();
        for site in &reading.thread_sites {
            if let Some((threads, unresolved)) = threads_of(&site.function) {
                id_sites.push((
                    format!("{}:{}", site.file, site.line),
                    site.call,
                    site.identity.clone(),
                    threads,
                    unresolved,
                ));
            }
        }
        let mut published_self = Vec::new();
        for site in &reading.self_sites {
            if site.published
                && let Some((threads, unresolved)) = threads_of(&site.function)
            {
                published_self.push((format!("{}:{}", site.file, site.line), threads, unresolved));
            }
        }
        let wrappers = reading
            .wrappers
            .iter()
            .map(|w| (w.wrapper.clone(), w.joinable.clone()))
            .collect();
        let mut root_threads: BTreeSet<String> = BTreeSet::from(["T".to_owned()]);
        for (id, _) in &reading.census.roots {
            root_threads.insert(plain(&reading.census.functions[*id]).to_owned());
        }
        let mut wrapper_entries = BTreeMap::new();
        for (spawner, entry) in &program.spawns {
            let spawner = plain(&program.functions[*spawner]);
            if let Some((wrapper, _, _)) = tables.spawns.iter().find(|(n, _, _)| *n == spawner)
                && *wrapper != "pthread_create"
            {
                wrapper_entries.insert(
                    plain(&program.functions[*entry]).to_owned(),
                    (*wrapper).to_owned(),
                );
            }
        }
        Self {
            id_sites,
            published_self,
            wrappers,
            root_threads,
            wrapper_entries,
        }
    }

    /// The four POSIX exclusions of the unchecked first join of `thread`
    /// (started through `wrapper`) by `joiner` (a test root's `T`, or a
    /// parent thread's plain entry name).
    #[must_use]
    pub fn exclusions(&self, thread: &str, wrapper: &str, joiner: &str) -> JoinExclusions {
        let joiners: BTreeSet<String> = if joiner == "T" {
            self.root_threads.clone()
        } else {
            BTreeSet::from([joiner.to_owned()])
        };
        let may_name = |identity: &Result<BTreeSet<String>, String>| match identity {
            Ok(set) => set.contains(thread),
            Err(_) => true,
        };
        let joinable = match self.wrappers.get(wrapper) {
            None => Err(format!(
                "`{thread}` was started through `{wrapper}`, which is not read"
            )),
            Some(Err(why)) => Err(format!("`{wrapper}`: {why}")),
            Some(Ok(_)) => self
                .id_sites
                .iter()
                .find(|(_, call, identity, _, _)| *call == "detach" && may_name(identity))
                .map_or(Ok(()), |(site, _, identity, _, _)| {
                    Err(match identity {
                        Ok(_) => format!("the detach at {site} names `{thread}`"),
                        Err(why) => format!("the detach at {site} may name `{thread}` ({why})"),
                    })
                }),
        };
        let busy: Vec<String> = self
            .id_sites
            .iter()
            .filter(|(_, call, identity, _, _)| *call == "join" && may_name(identity))
            .filter_map(|(site, _, identity, threads, unresolved)| {
                let others: Vec<&String> =
                    threads.iter().filter(|t| !joiners.contains(*t)).collect();
                let named = match identity {
                    Ok(_) => format!("the join at {site} names `{thread}`"),
                    Err(why) => format!("the join at {site} may name `{thread}` ({why})"),
                };
                if let Some(other) = others.first() {
                    Some(format!(
                        "{named} and runs on {} other thread(s) (e.g. `{other}`)",
                        others.len()
                    ))
                } else if *unresolved {
                    Some(format!("{named} and some path to it is unresolved"))
                } else {
                    None
                }
            })
            .collect();
        let not_busy = if busy.is_empty() {
            Ok(())
        } else {
            Err(format!(
                "{} join site(s) may join it off the joining thread, the joined instance not \
                 derived: {}",
                busy.len(),
                busy.join("; ")
            ))
        };
        let not_self = if joiners.contains(thread) {
            Err(format!("`{thread}` is the joining thread's own entry"))
        } else {
            Ok(())
        };
        let keepers: Vec<&String> = self
            .published_self
            .iter()
            .filter(|(_, threads, unresolved)| {
                *unresolved || threads.iter().any(|t| joiners.contains(t))
            })
            .map(|(site, _, _)| site)
            .collect();
        let no_cycle = if keepers.is_empty() {
            Ok(())
        } else {
            Err(format!(
                "{} `pthread_self()` site(s) may keep the joining thread's identifier: {}",
                keepers.len(),
                keepers
                    .iter()
                    .map(|s| s.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            ))
        };
        JoinExclusions {
            joinable,
            not_busy,
            not_self,
            no_cycle,
        }
    }

    /// Whether no reached join site may name `thread`.
    fn never_joined(&self, thread: &str) -> Result<(), String> {
        for (site, call, identity, _, _) in &self.id_sites {
            if *call != "join" {
                continue;
            }
            match identity {
                Ok(set) if set.contains(thread) => {
                    return Err(format!("the join at {site} names it"));
                }
                Ok(_) => {}
                Err(why) => return Err(format!("the join at {site} is not read ({why})")),
            }
        }
        Ok(())
    }
}

fn refusal(kind: ReadRefusal) -> KeptRefusal {
    match kind {
        ReadRefusal::Broadcast => KeptRefusal::Broadcast,
        ReadRefusal::C11Condvar => KeptRefusal::C11Condvar,
        ReadRefusal::Rwlock => KeptRefusal::Rwlock,
        ReadRefusal::TryOrTimedLock => KeptRefusal::TryOrTimedLock,
        ReadRefusal::StaticInitializer => KeptRefusal::StaticInitializer,
        ReadRefusal::RecursiveOrDynamicMutex => KeptRefusal::RecursiveOrDynamicMutex,
    }
}

fn held(held: ReadHeld) -> Held {
    match held {
        ReadHeld::Yes => Held::Yes,
        ReadHeld::No => Held::No,
        ReadHeld::Unknown => Held::Unknown,
    }
}

fn body(body: ReadLoopBody) -> LoopBody {
    match body {
        ReadLoopBody::WaitOnly => LoopBody::WaitOnly,
        ReadLoopBody::WaitThenBreakOnNonzero => LoopBody::WaitThenBreakOnNonzero,
        ReadLoopBody::Other => LoopBody::Other,
    }
}

fn evidence(join: &ReadJoin, thread: &str, joiner: &str, facts: &ThreadFacts) -> JoinEvidence {
    match join {
        ReadJoin::CheckedSuccess => JoinEvidence::CheckedSuccess,
        ReadJoin::Unchecked => JoinEvidence::Unchecked,
        ReadJoin::UncheckedFirst { wrapper } => {
            JoinEvidence::UncheckedFirst(facts.exclusions(thread, wrapper, joiner))
        }
        ReadJoin::Repeated => JoinEvidence::Repeated,
        ReadJoin::Failed => JoinEvidence::Failed,
        ReadJoin::Absent {
            timed_out: true, ..
        } => JoinEvidence::Absent(Absence::AfterTimeout),
        ReadJoin::Absent { awaited: true, .. } => JoinEvidence::Absent(Absence::AfterPublication),
        ReadJoin::Absent { .. } => JoinEvidence::Absent(Absence::Nothing),
    }
}

/// The struct type of a field key (`T.F` → `T`).
fn struct_of(key: &str) -> &str {
    key.split_once('.').map_or(key, |(ty, _)| ty)
}

/// Read, derive and decide every site of one program.
#[must_use]
pub fn census(files: &[CensusFile], hops: &HopTables, tables: &SyncTables) -> Census {
    census_inner(files, hops, tables, None)
}

/// [`census`] under declared lifecycle obligations (card
/// W47-FLB-LIB-LIFECYCLE): each clause checked against the declared bodies,
/// each declared call against the contract, and every release refused where
/// a path reaching it violated a lifecycle precondition.
#[must_use]
pub fn census_with_lifecycle(
    files: &[CensusFile],
    hops: &HopTables,
    tables: &SyncTables,
    declared: &BTreeMap<String, qcore_contract::contract_loader::DeclaredLifecycle>,
    creators: &[&str],
) -> Census {
    let roles = crate::lifecycle_census::roles(declared, creators);
    census_inner(files, hops, tables, Some(&roles))
}

fn census_inner(
    files: &[CensusFile],
    hops: &HopTables,
    tables: &SyncTables,
    roles: Option<&qcore_c_gateway::condvar_census::LifecycleRoles>,
) -> Census {
    let reading = read_program_lifecycle(
        files,
        hops,
        tables,
        &|test| null_branch_excluded(&null_branch_evidence(test)),
        roles,
    );
    let program = crate::loop_owner_census::thread_program(&reading.census);
    let derived = derive(&program);
    let facts = ThreadFacts::new(&reading, &program, &derived, tables);
    let unread: BTreeMap<&str, &str> = reading
        .functions
        .iter()
        .filter_map(|f| {
            f.unresolved
                .as_deref()
                .map(|why| (f.function.as_str(), why))
        })
        .collect();

    // Pairing, predicates and waiter functions.
    let mut pairing = Pairing::default();
    let mut predicates: BTreeMap<String, (BTreeSet<String>, BTreeSet<Fact>)> = BTreeMap::new();
    let mut waiters: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for site in &reading.sites {
        if let ReadSite::Wait(wait) = &site.site
            && let (Some(condvar), Some(mutex)) = (&wait.condvar, &wait.mutex)
        {
            pairing.record(condvar, mutex);
            waiters
                .entry(condvar.clone())
                .or_default()
                .insert(site.function.clone());
            if let Some(predicate_loop) = &wait.predicate_loop {
                let entry = predicates.entry(condvar.clone()).or_default();
                entry.0.extend(predicate_loop.predicate.iter().cloned());
                if let Some(fact) = &predicate_loop.exit_fact {
                    entry.1.insert(fact.clone());
                }
            }
        }
    }
    // Results: the non-predicate fields of the condvar's struct written in a
    // signalling section of its paired mutex.
    let mut results: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let mut signallers: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for site in &reading.sites {
        if let ReadSite::Signal(signal) = &site.site
            && let Some(condvar) = &signal.condvar
        {
            signallers
                .entry(condvar.clone())
                .or_default()
                .insert(site.function.clone());
            let Some(mutex) = pairing.mutex(condvar) else {
                continue;
            };
            let predicate = predicates
                .get(condvar)
                .map(|p| p.0.clone())
                .unwrap_or_default();
            for event in signal.sections.get(mutex).into_iter().flatten() {
                if let ReadSectionEvent::Write(key) = event
                    && struct_of(key) == struct_of(condvar)
                    && !predicate.contains(key)
                    && key != condvar
                    && key != mutex
                {
                    results
                        .entry(condvar.clone())
                        .or_default()
                        .insert(key.clone());
                }
            }
        }
    }
    let once: BTreeMap<&str, &Result<String, String>> = reading
        .signal_once
        .iter()
        .map(|s| (s.condvar.as_str(), &s.derived))
        .collect();
    // Guarded writes outside the paired mutex (after the fresh phase).
    let guarded_of = |condvar: &str| -> BTreeSet<String> {
        let mut keys = predicates
            .get(condvar)
            .map(|p| p.0.clone())
            .unwrap_or_default();
        keys.extend(results.get(condvar).cloned().unwrap_or_default());
        keys
    };
    let unguarded = |condvar: &str| -> Vec<String> {
        let Some(mutex) = pairing.mutex(condvar) else {
            return Vec::new();
        };
        let keys = guarded_of(condvar);
        reading
            .sites
            .iter()
            .filter_map(|site| match &site.site {
                ReadSite::Access(access)
                    if access.write
                        && keys.contains(&access.key)
                        && !access.fresh
                        && !access.held.contains(mutex) =>
                {
                    Some(format!("{} at {}:{}", access.key, site.file, site.line))
                }
                _ => None,
            })
            .collect()
    };
    let releases_by_site: BTreeMap<(String, usize), Vec<&ReleaseReading>> = {
        let mut map: BTreeMap<(String, usize), Vec<&ReleaseReading>> = BTreeMap::new();
        for release in &reading.releases {
            map.entry((release.file.clone(), release.line))
                .or_default()
                .push(release);
        }
        map
    };
    let retainers = retainers(&program, &derived, &reading);
    let release_site = |release: &ReleaseReading| -> ReleaseSite {
        release_site(release, &program, &derived, &retainers, &facts, &reading)
    };

    let mut sites = Vec::new();
    for site in &reading.sites {
        let unread_reason = unread.get(site.function.as_str()).copied();
        let (kind, object, verdict) = match &site.site {
            ReadSite::Refused(kind) => ("refused", None, SyncVerdict::Refused(refusal(*kind))),
            ReadSite::Wait(wait) => {
                let verdict = match (&wait.condvar, &wait.mutex, unread_reason) {
                    (_, _, Some(why)) => {
                        SyncVerdict::unresolved(format!("the body is not read: {why}"))
                    }
                    (Some(condvar), Some(mutex), None) => wait_verdict(
                        &WaitSite {
                            condvar: condvar.clone(),
                            mutex: mutex.clone(),
                            timed: wait.timed,
                            held: held(wait.held),
                            predicate_loop: wait.predicate_loop.as_ref().map(|l| PredicateLoop {
                                predicate: l.predicate.clone(),
                                under_mutex: l.under_mutex,
                                body: body(l.body),
                            }),
                        },
                        &pairing,
                    ),
                    _ => SyncVerdict::unresolved(
                        "the wait's condvar or mutex is not a field-keyed object",
                    ),
                };
                ("wait", wait.condvar.clone(), verdict)
            }
            ReadSite::Signal(signal) => {
                let verdict = match (&signal.condvar, unread_reason) {
                    (_, Some(why)) => {
                        SyncVerdict::unresolved(format!("the body is not read: {why}"))
                    }
                    (Some(condvar), None) => {
                        let signalled = signal_verdict(
                            &SignalSite {
                                condvar: condvar.clone(),
                                broadcast: signal.broadcast,
                                held: signal.held.clone(),
                                held_unknown: signal.held_unknown,
                            },
                            &pairing,
                        );
                        if signalled.admits() {
                            let mutex = pairing.mutex(condvar).cloned().unwrap_or_default();
                            match signal.sections.get(&mutex) {
                                Some(section) => publication_verdict(&PublishSite {
                                    section: section
                                        .iter()
                                        .map(|event| match event {
                                            ReadSectionEvent::Write(key) => {
                                                SectionEvent::Write(key.clone())
                                            }
                                            ReadSectionEvent::Signal => SectionEvent::Signal,
                                        })
                                        .collect(),
                                    predicate: predicates
                                        .get(condvar)
                                        .map(|p| p.0.clone())
                                        .unwrap_or_default(),
                                    unguarded_writes: unguarded(condvar),
                                }),
                                None => SyncVerdict::unresolved(
                                    "the signalling critical section is not one block",
                                ),
                            }
                        } else {
                            signalled
                        }
                    }
                    (None, None) if signal.broadcast => {
                        SyncVerdict::Refused(KeptRefusal::Broadcast)
                    }
                    (None, None) => {
                        SyncVerdict::unresolved("the signalled condvar is not a field-keyed object")
                    }
                };
                (
                    if signal.broadcast {
                        "broadcast"
                    } else {
                        "signal"
                    },
                    signal.condvar.clone(),
                    verdict,
                )
            }
            ReadSite::Lifecycle(lifecycle) => {
                // Card W47-FLB-LIB-LIFECYCLE: after a join, the paths reaching
                // the destroy carry their lifecycle violations too.
                let mut violations = BTreeSet::new();
                let phase = match (&lifecycle.condvar, unread_reason) {
                    (_, Some(why)) => Phase::Unknown(format!("the body is not read: {why}")),
                    (None, None) => {
                        Phase::Unknown("the condvar is not a field-keyed object".to_owned())
                    }
                    (Some(_), None) if lifecycle.fresh => Phase::BeforePublication,
                    (Some(condvar), None) => {
                        let exit_facts = predicates.get(condvar).map(|p| &p.1);
                        let observed = exit_facts.is_some_and(|facts| {
                            !facts.is_empty()
                                && facts.iter().all(|f| lifecycle.observed_ever.contains(f))
                        });
                        let single = once.get(condvar.as_str()).is_some_and(|d| d.is_ok());
                        if observed && single {
                            Phase::AfterLastSignal
                        } else if let Some(found) =
                            releases_by_site.get(&(site.file.clone(), site.line))
                        {
                            // After a join: every root's reading at this site.
                            let mut merged = ReleaseSite::default();
                            for release in found {
                                violations
                                    .extend(crate::lifecycle_census::release_violations(release));
                                let one = release_site(release);
                                merged.live.extend(one.live);
                                merged.unresolved.extend(one.unresolved);
                                merged.descendants.extend(one.descendants);
                            }
                            Phase::AfterJoin(merged)
                        } else if observed {
                            Phase::Unknown(format!(
                                "no single-signal derivation for `{condvar}`: {}",
                                once.get(condvar.as_str())
                                    .and_then(|d| d.as_ref().err())
                                    .map_or("no signal site", String::as_str)
                            ))
                        } else {
                            Phase::Unknown("no quiescent phase is derived".to_owned())
                        }
                    }
                };
                (
                    if lifecycle.init { "init" } else { "destroy" },
                    lifecycle.condvar.clone(),
                    qcore_core::paired_lifecycle::lifecycle_gate(&violations)
                        .join(quiescence_verdict(&phase)),
                )
            }
            ReadSite::Access(access) => {
                // Only a waiter's read of a guarded field is a C4 site.
                let Some((condvar, licence)) = guarded_read(
                    site,
                    access,
                    &pairing,
                    &predicates,
                    &results,
                    &waiters,
                    &signallers,
                    &once,
                    &reading,
                ) else {
                    continue;
                };
                let verdict = match unread_reason {
                    Some(why) => SyncVerdict::unresolved(format!("the body is not read: {why}")),
                    None => read_verdict(&licence),
                };
                let _ = condvar;
                ("read", Some(access.key.clone()), verdict)
            }
        };
        sites.push(SiteVerdict {
            file: site.file.clone(),
            line: site.line,
            function: site.function.clone(),
            callee: site.callee.clone(),
            kind,
            object,
            verdict,
        });
    }
    let releases = reading
        .releases
        .iter()
        .map(|reading_release| {
            let site = release_site(reading_release);
            // Card W47-FLB-LIB-LIFECYCLE: a violated lifecycle precondition
            // refuses the release first; it never discharges one.
            let lifecycle_violations = crate::lifecycle_census::release_violations(reading_release);
            let verdict = qcore_core::paired_lifecycle::lifecycle_gate(&lifecycle_violations)
                .join(release_verdict(&site));
            let premises = release_premises(&site);
            // Card W47-FLB-LIB-STRICT-PAIRING: the conforming paths decided
            // apart.
            let lifecycle_split =
                crate::lifecycle_census::release_split(reading_release).map(|split| {
                    let conforming_site = split.conforming.as_ref().map(release_site);
                    LifecycleSplitVerdict {
                        conforming_paths: split.conforming_paths,
                        undetermined_paths: split.undetermined_paths,
                        refused_paths: split.refused_paths,
                        conforming_verdict: conforming_site.as_ref().map(release_verdict),
                        conforming_premises: conforming_site
                            .as_ref()
                            .map(release_premises)
                            .unwrap_or_default(),
                        conforming_site,
                    }
                });
            ReleaseVerdict {
                reading: reading_release.clone(),
                site,
                verdict,
                premises,
                lifecycle_violations,
                lifecycle_split,
            }
        })
        .collect();
    let lifecycle = roles.map(|roles| crate::lifecycle_census::decide(&reading, roles));
    Census {
        reading,
        program,
        derived,
        pairing,
        predicates,
        results,
        sites,
        releases,
        retainers,
        facts,
        lifecycle,
    }
}

fn plain(label: &str) -> &str {
    label.rsplit_once("::").map_or(label, |(_, name)| name)
}

/// The threads whose derived contexts reach a function defined under
/// [`RETAINING_CODE`] (a test-table root excepted: it is the releasing thread).
fn retainers(
    program: &ThreadProgram,
    derived: &[Derived],
    reading: &SyncReading,
) -> BTreeSet<String> {
    let roots: BTreeSet<usize> = reading.census.roots.iter().map(|(id, _)| *id).collect();
    let mut out = BTreeSet::new();
    for (id, label) in program.functions.iter().enumerate() {
        let file = reading
            .defined_in
            .get(label)
            .map_or(label.as_str(), String::as_str);
        let in_code = RETAINING_CODE.iter().any(|prefix| file.starts_with(prefix));
        if !in_code || roots.contains(&id) {
            continue;
        }
        for context in &derived[id].contexts {
            if !roots
                .iter()
                .any(|root| program.functions[*root] == context.thread)
                && context.thread != "T"
            {
                out.insert(plain(&context.thread).to_owned());
            }
        }
    }
    out
}

/// The licence of a guarded read, when the access is one: a read (not a
/// write, not the wait loop's own condition) of a predicate or result field
/// of a condvar, in a function that waits on it.
#[allow(clippy::too_many_arguments)]
fn guarded_read(
    site: &SyncSite,
    access: &ReadAccess,
    pairing: &Pairing,
    predicates: &BTreeMap<String, (BTreeSet<String>, BTreeSet<Fact>)>,
    results: &BTreeMap<String, BTreeSet<String>>,
    waiters: &BTreeMap<String, BTreeSet<String>>,
    signallers: &BTreeMap<String, BTreeSet<String>>,
    once: &BTreeMap<&str, &Result<String, String>>,
    reading: &SyncReading,
) -> Option<(String, ReadLicence)> {
    if access.write || access.in_wait_condition {
        return None;
    }
    let condvar = waiters.iter().find_map(|(condvar, functions)| {
        let guarded = predicates
            .get(condvar)
            .is_some_and(|p| p.0.contains(&access.key))
            || results
                .get(condvar)
                .is_some_and(|r| r.contains(&access.key));
        (functions.contains(&site.function) && guarded).then(|| condvar.clone())
    })?;
    let Some(mutex) = pairing.mutex(&condvar) else {
        return Some((
            condvar,
            ReadLicence::Unknown("the condvar is not paired with one mutex".to_owned()),
        ));
    };
    let facts = predicates
        .get(&condvar)
        .map(|p| p.1.clone())
        .unwrap_or_default();
    if facts.is_empty() {
        return Some((
            condvar,
            ReadLicence::Unknown("the wait loop's predicate is not one atom".to_owned()),
        ));
    }
    let observed_under = access
        .observed
        .get(mutex)
        .is_some_and(|seen| facts.iter().all(|f| seen.contains(f)));
    // The predicate itself read under the mutex is the recheck that licenses.
    let is_predicate = predicates
        .get(&condvar)
        .is_some_and(|p| p.0.contains(&access.key));
    if access.held.contains(mutex) && (observed_under || is_predicate) {
        return Some((condvar, ReadLicence::PredicateUnderMutex));
    }
    // A final value: observed before, a single once-signaller, and every
    // write of the field in that signaller.
    let observed_ever = facts.iter().all(|f| access.observed_ever.contains(f));
    let single = once.get(condvar.as_str()).is_some_and(|d| d.is_ok());
    let writers_in_signaller = signallers.get(&condvar).is_some_and(|functions| {
        reading.sites.iter().all(|other| match &other.site {
            ReadSite::Access(write) if write.write && write.key == access.key && !write.fresh => {
                functions.contains(&other.function)
            }
            _ => true,
        })
    });
    if observed_ever && single && writers_in_signaller {
        return Some((condvar, ReadLicence::StablePublication));
    }
    let reason = if !access.held.contains(mutex) {
        "read outside the mutex without a derived final value"
    } else {
        "read under the mutex without the predicate observed since it was acquired"
    };
    Some((condvar, ReadLicence::None(reason.to_owned())))
}

/// The kernel's release site of one reading: every live thread's paths (the
/// joiner is the test root's thread), and every thread each one starts, decided
/// from the parent's own entry run (card W47-CONDVAR-JOIN-STOP-PATHS).
fn release_site(
    release: &ReleaseReading,
    program: &ThreadProgram,
    derived: &[Derived],
    retainers: &BTreeSet<String>,
    facts: &ThreadFacts,
    reading: &SyncReading,
) -> ReleaseSite {
    let mut site = ReleaseSite {
        live: Vec::new(),
        unresolved: release.unresolved.clone(),
        descendants: Vec::new(),
    };
    let whole = WHOLE_ENGINE_RELEASES.contains(&release.what.as_str());
    for (thread, paths) in &release.live {
        if !whole && !retainers.contains(thread) {
            continue;
        }
        site.live.push(LiveThread {
            thread: thread.clone(),
            paths: paths
                .iter()
                .map(|(join, witness)| (evidence(join, thread, "T", facts), witness.clone()))
                .collect(),
        });
        let family: BTreeSet<String> = descendants(thread, program, derived)
            .into_iter()
            .filter(|child| whole || retainers.contains(plain(child)))
            .collect();
        for child in family {
            for parent in parents(&child, thread, program, derived) {
                let evidence = descendant_evidence(&parent, &child, facts, reading);
                site.descendants.push(DescendantJoin {
                    parent,
                    child: plain(&child).to_owned(),
                    evidence,
                });
            }
        }
    }
    site.descendants.sort();
    site.descendants.dedup();
    site
}

/// The direct parents of `child` among `root` and its descendants: the
/// threads whose contexts reach a spawner of `child`.
fn parents(
    child: &str,
    root: &str,
    program: &ThreadProgram,
    derived: &[Derived],
) -> BTreeSet<String> {
    let mut family = descendants(root, program, derived);
    family.insert(root.to_owned());
    let family: BTreeSet<&str> = family.iter().map(|t| plain(t)).collect();
    let mut out = BTreeSet::new();
    for (spawner, entry) in &program.spawns {
        if program.functions[*entry] != child && plain(&program.functions[*entry]) != plain(child) {
            continue;
        }
        for context in &derived[*spawner].contexts {
            let thread = plain(&context.thread);
            if family.contains(thread) {
                out.insert(thread.to_owned());
            }
        }
    }
    if out.is_empty() {
        out.insert(plain(root).to_owned());
    }
    out
}

/// Whether `parent` is derived to join `child` before it ends: from the
/// parent's entry run to its exit, every path joined it (checked, or an
/// unchecked first join whose POSIX cases are excluded for `parent`), with
/// nothing unresolved; or no join site of the program may name it.
fn descendant_evidence(
    parent: &str,
    child: &str,
    facts: &ThreadFacts,
    reading: &SyncReading,
) -> DescendantEvidence {
    let child = plain(child);
    if let Some(wrapper) = facts.wrapper_entries.get(child) {
        return DescendantEvidence::Unresolved(format!(
            "`{child}` is the entry through which `{wrapper}` runs every thread it starts; each \
             such thread is decided under its own entry"
        ));
    }
    if facts.never_joined(child).is_ok() {
        return DescendantEvidence::NeverJoined("no join site of the program names it".to_owned());
    }
    let Some(exit) = reading.entry_exits.get(parent) else {
        return DescendantEvidence::Unresolved(format!(
            "`{parent}`'s entry is not run by the path reader"
        ));
    };
    if let Some(reason) = exit.unresolved.iter().next() {
        return DescendantEvidence::Unresolved(format!(
            "{} reason(s) on `{parent}`'s path to its exit, e.g. {reason}",
            exit.unresolved.len()
        ));
    }
    let Some(paths) = exit.live.get(child) else {
        return DescendantEvidence::Unresolved(format!(
            "the path reader does not see `{parent}` start `{child}`"
        ));
    };
    for (join, witness) in paths {
        match evidence(join, child, parent, facts) {
            JoinEvidence::CheckedSuccess => {}
            JoinEvidence::UncheckedFirst(exclusions) => {
                if let Err(why) = join_succeeds(&exclusions) {
                    return DescendantEvidence::Unresolved(format!(
                        "the join at {witness} is not checked, and {why}"
                    ));
                }
            }
            other => {
                return DescendantEvidence::Unresolved(format!(
                    "`{child}` reaches `{parent}`'s exit as {other:?} ({witness})"
                ));
            }
        }
    }
    DescendantEvidence::Joined
}

/// The threads a thread starts, transitively, from the derivation.
fn descendants(thread: &str, program: &ThreadProgram, derived: &[Derived]) -> BTreeSet<String> {
    let named = |label: &str| {
        label == thread
            || label
                .rsplit_once("::")
                .is_some_and(|(_, name)| name == thread)
    };
    let mut threads: BTreeSet<String> = program
        .functions
        .iter()
        .filter(|label| named(label))
        .cloned()
        .collect();
    if threads.is_empty() {
        return BTreeSet::from([format!("{thread} (not in the inventory)")]);
    }
    let start = threads.clone();
    loop {
        let before = threads.len();
        for (spawner, entry) in &program.spawns {
            if derived[*spawner]
                .contexts
                .iter()
                .any(|context| threads.contains(&context.thread))
            {
                threads.insert(program.functions[*entry].clone());
            }
        }
        if threads.len() == before {
            break;
        }
    }
    threads.difference(&start).cloned().collect()
}
