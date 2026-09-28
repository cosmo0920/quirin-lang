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

use std::collections::{BTreeMap, BTreeSet};

use qcore_c_gateway::condvar_census::{
    Fact, ReadAccess, ReadHeld, ReadJoin, ReadLoopBody, ReadRefusal, ReadSectionEvent, ReadSite,
    ReleaseReading, SyncReading, SyncSite, SyncTables, read_program,
};
use qcore_c_gateway::thread_entry_census::{CensusFile, HopTables};
use qcore_core::condvar_join::{
    Absence, Held, JoinEvidence, KeptRefusal, LiveThread, LoopBody, Pairing, Phase, PredicateLoop,
    PublishSite, ReadLicence, ReleaseSite, SectionEvent, SignalSite, SyncVerdict, WaitSite,
    publication_verdict, quiescence_verdict, read_verdict, release_verdict, signal_verdict,
    wait_verdict,
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

fn evidence(join: &ReadJoin) -> JoinEvidence {
    match join {
        ReadJoin::CheckedSuccess => JoinEvidence::CheckedSuccess,
        ReadJoin::Unchecked => JoinEvidence::Unchecked,
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
    let reading = read_program(files, hops, tables);
    let program = crate::loop_owner_census::thread_program(&reading.census);
    let derived = derive(&program);
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
        release_site(release, &program, &derived, &retainers)
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
                                let one = release_site(release);
                                merged.live.extend(one.live);
                                merged.unresolved.extend(one.unresolved);
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
                    quiescence_verdict(&phase),
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
            let verdict = release_verdict(&site);
            ReleaseVerdict {
                reading: reading_release.clone(),
                site,
                verdict,
            }
        })
        .collect();
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

/// The kernel's release site of one reading: every live thread's paths, and
/// the threads each one starts, whose joins before it ends are not derived.
fn release_site(
    release: &ReleaseReading,
    program: &ThreadProgram,
    derived: &[Derived],
    retainers: &BTreeSet<String>,
) -> ReleaseSite {
    let mut site = ReleaseSite {
        live: Vec::new(),
        unresolved: release.unresolved.clone(),
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
                .map(|(join, witness)| (evidence(join), witness.clone()))
                .collect(),
        });
        let children: BTreeSet<String> = descendants(thread, program, derived)
            .into_iter()
            .filter(|child| whole || retainers.contains(plain(child)))
            .collect();
        if !children.is_empty() {
            site.unresolved.insert(format!(
                "`{thread}` starts {} thread(s) (e.g. `{}`) whose joins before it ends are not derived",
                children.len(),
                children.iter().next().cloned().unwrap_or_default()
            ));
        }
    }
    site
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
