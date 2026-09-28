use super::*;
use crate::concurrency::{ConcurrencyDiagnostic, check_concurrent};

fn keys(names: &[&str]) -> BTreeSet<Key> {
    names.iter().map(|name| (*name).to_owned()).collect()
}

fn pairing() -> Pairing {
    let mut pairing = Pairing::default();
    pairing.record("tail_pause_test.cond", "tail_pause_test.mutex");
    pairing
}

fn wait(timed: bool, body: LoopBody) -> WaitSite {
    WaitSite {
        condvar: "tail_pause_test.cond".to_owned(),
        mutex: "tail_pause_test.mutex".to_owned(),
        timed,
        held: Held::Yes,
        predicate_loop: Some(PredicateLoop {
            predicate: keys(&["tail_pause_test.done"]),
            under_mutex: true,
            body,
        }),
    }
}

#[test]
fn a_predicate_loop_under_its_mutex_is_admitted() {
    // NC2's await (`in_tail.c:2563-2569`) and the worker's (`flb_input_thread.c:708-711`).
    assert!(wait(true, LoopBody::WaitThenBreakOnNonzero).admits_under(&pairing()));
    assert!(wait(false, LoopBody::WaitOnly).admits_under(&pairing()));
    assert!(wait(true, LoopBody::WaitOnly).admits_under(&pairing()));
}

trait Admits {
    fn admits_under(&self, pairing: &Pairing) -> bool;
}

impl Admits for WaitSite {
    fn admits_under(&self, pairing: &Pairing) -> bool {
        wait_verdict(self, pairing).admits()
    }
}

#[test]
fn a_wait_outside_a_predicate_loop_is_a_kept_refusal() {
    let mut site = wait(false, LoopBody::WaitOnly);
    site.predicate_loop = None;
    assert_eq!(
        wait_verdict(&site, &pairing()),
        SyncVerdict::Refused(KeptRefusal::WaitOutsidePredicateLoop)
    );
    // `while (1) wait(...)` rechecks nothing.
    let mut forever = wait(false, LoopBody::WaitOnly);
    forever.predicate_loop.as_mut().unwrap().predicate.clear();
    assert_eq!(
        wait_verdict(&forever, &pairing()),
        SyncVerdict::Refused(KeptRefusal::WaitOutsidePredicateLoop)
    );
}

#[test]
fn a_wait_without_its_mutex_or_with_an_unread_body_is_not_admitted() {
    let mut unheld = wait(false, LoopBody::WaitOnly);
    unheld.held = Held::No;
    assert!(matches!(
        wait_verdict(&unheld, &pairing()),
        SyncVerdict::NotDischarged { .. }
    ));
    let mut unknown = wait(false, LoopBody::WaitOnly);
    unknown.held = Held::Unknown;
    assert!(matches!(
        wait_verdict(&unknown, &pairing()),
        SyncVerdict::Unresolved { .. }
    ));
    let mut outside = wait(false, LoopBody::WaitOnly);
    outside.predicate_loop.as_mut().unwrap().under_mutex = false;
    assert!(matches!(
        wait_verdict(&outside, &pairing()),
        SyncVerdict::NotDischarged { .. }
    ));
    assert!(!wait(false, LoopBody::Other).admits_under(&pairing()));
    // A break on a nonzero untimed wait is not a shape this lift names.
    assert!(!wait(false, LoopBody::WaitThenBreakOnNonzero).admits_under(&pairing()));
}

#[test]
fn one_condvar_with_two_mutexes_refuses_every_site() {
    let mut pairing = pairing();
    pairing.record("tail_pause_test.cond", "other.mutex");
    assert_eq!(
        wait_verdict(&wait(false, LoopBody::WaitOnly), &pairing),
        SyncVerdict::Refused(KeptRefusal::CondvarWithTwoMutexes)
    );
    let signal = SignalSite {
        condvar: "tail_pause_test.cond".to_owned(),
        broadcast: false,
        held: keys(&["tail_pause_test.mutex"]),
        held_unknown: false,
    };
    assert_eq!(
        signal_verdict(&signal, &pairing),
        SyncVerdict::Refused(KeptRefusal::CondvarWithTwoMutexes)
    );
}

#[test]
fn a_signal_is_admitted_only_under_the_paired_mutex() {
    let held = SignalSite {
        condvar: "tail_pause_test.cond".to_owned(),
        broadcast: false,
        held: keys(&["tail_pause_test.mutex"]),
        held_unknown: false,
    };
    assert_eq!(signal_verdict(&held, &pairing()), SyncVerdict::Discharged);
    let without = SignalSite {
        held: BTreeSet::new(),
        ..held.clone()
    };
    assert_eq!(
        signal_verdict(&without, &pairing()),
        SyncVerdict::Refused(KeptRefusal::SignalWithoutMutex)
    );
    let broadcast = SignalSite {
        broadcast: true,
        ..held.clone()
    };
    assert_eq!(
        signal_verdict(&broadcast, &pairing()),
        SyncVerdict::Refused(KeptRefusal::Broadcast)
    );
    let unpaired = SignalSite {
        condvar: "lonely.cond".to_owned(),
        ..held
    };
    assert!(matches!(
        signal_verdict(&unpaired, &pairing()),
        SyncVerdict::Unresolved { .. }
    ));
}

#[test]
fn results_must_precede_the_predicate_in_the_signalling_section() {
    let predicate = keys(&["tail_pause_test.done"]);
    let good = PublishSite {
        section: vec![
            SectionEvent::Write("tail_pause_test.pause_result".to_owned()),
            SectionEvent::Write("tail_pause_test.done".to_owned()),
            SectionEvent::Signal,
        ],
        predicate: predicate.clone(),
        unguarded_writes: Vec::new(),
    };
    assert_eq!(publication_verdict(&good), SyncVerdict::Discharged);
    let late = PublishSite {
        section: vec![
            SectionEvent::Write("tail_pause_test.done".to_owned()),
            SectionEvent::Write("tail_pause_test.pause_result".to_owned()),
            SectionEvent::Signal,
        ],
        ..good.clone()
    };
    assert!(matches!(
        publication_verdict(&late),
        SyncVerdict::NotDischarged { .. }
    ));
    let early_signal = PublishSite {
        section: vec![
            SectionEvent::Signal,
            SectionEvent::Write("tail_pause_test.done".to_owned()),
        ],
        ..good.clone()
    };
    assert!(matches!(
        publication_verdict(&early_signal),
        SyncVerdict::NotDischarged { .. }
    ));
    let unguarded = PublishSite {
        unguarded_writes: vec!["tail_pause_test.pause_result".to_owned()],
        ..good
    };
    assert!(matches!(
        publication_verdict(&unguarded),
        SyncVerdict::NotDischarged { .. }
    ));
}

#[test]
fn nc8_a_wakeup_before_publication_waits_again_and_claims_nothing() {
    // A spurious or early signal before `done`: the loop rechecks and waits.
    assert_eq!(
        waiter_step(false, Wakeup::Spurious, true),
        WaiterStep::Rewait
    );
    assert_eq!(
        waiter_step(false, Wakeup::Signalled, true),
        WaiterStep::Rewait
    );
    assert_eq!(
        run_waiter(&[(false, Wakeup::Spurious), (false, Wakeup::Signalled)], true),
        None
    );
    // Only an observed predicate claims.
    assert_eq!(
        run_waiter(&[(false, Wakeup::Spurious), (true, Wakeup::Signalled)], true),
        Some(true)
    );
    // A timeout that breaks leaves without a claim.
    assert_eq!(
        run_waiter(&[(false, Wakeup::Spurious), (false, Wakeup::TimedOut)], true),
        Some(false)
    );
    // A loop that ignores the timeout rechecks instead.
    assert_eq!(run_waiter(&[(false, Wakeup::TimedOut)], false), None);
    assert!(!read_verdict(&ReadLicence::None("no predicate observed".to_owned())).admits());
    assert!(read_verdict(&ReadLicence::PredicateUnderMutex).admits());
}

fn live(evidence: JoinEvidence) -> ReleaseSite {
    ReleaseSite {
        live: vec![LiveThread {
            thread: "flb_lib_worker".to_owned(),
            paths: vec![(evidence, "path".to_owned())],
        }],
        unresolved: BTreeSet::new(),
    }
}

#[test]
fn nc2_a_checked_join_of_every_live_thread_discharges_the_release() {
    assert_eq!(
        release_verdict(&live(JoinEvidence::CheckedSuccess)),
        SyncVerdict::Discharged
    );
    assert_eq!(
        release_verdict(&ReleaseSite::default()),
        SyncVerdict::Discharged
    );
}

#[test]
fn nc4_nc5_nc9_publication_timeout_and_early_release_are_rejected() {
    for evidence in [
        JoinEvidence::Absent(Absence::AfterPublication),
        JoinEvidence::Absent(Absence::AfterTimeout),
        JoinEvidence::Absent(Absence::Nothing),
        JoinEvidence::Unchecked,
        JoinEvidence::Failed,
    ] {
        assert!(
            matches!(
                release_verdict(&live(evidence.clone())),
                SyncVerdict::NotDischarged { .. }
            ),
            "{evidence:?}"
        );
    }
    // One bad path among good ones is enough.
    let mixed = ReleaseSite {
        live: vec![LiveThread {
            thread: "flb_lib_worker".to_owned(),
            paths: vec![
                (JoinEvidence::CheckedSuccess, "normal".to_owned()),
                (JoinEvidence::Unchecked, "status branch".to_owned()),
            ],
        }],
        unresolved: BTreeSet::new(),
    };
    assert!(matches!(
        release_verdict(&mixed),
        SyncVerdict::NotDischarged { .. }
    ));
}

#[test]
fn an_unresolved_descendant_withholds_the_discharge() {
    let mut site = live(JoinEvidence::CheckedSuccess);
    site.unresolved.insert("the children of `flb_lib_worker`".to_owned());
    assert!(matches!(
        release_verdict(&site),
        SyncVerdict::Unresolved { .. }
    ));
    assert!(!release_verdict(&site).admits());
}

#[test]
fn quiescent_phases_and_the_per_unit_route() {
    assert!(quiescence_verdict(&Phase::BeforePublication).admits());
    assert!(quiescence_verdict(&Phase::AfterLastSignal).admits());
    assert!(!quiescence_verdict(&Phase::Unknown("no phase".to_owned())).admits());
    assert!(!quiescence_verdict(&Phase::AfterJoin(live(JoinEvidence::Unchecked))).admits());
    assert!(!per_unit_route_verdict().admits());
}

#[test]
fn verdicts_compose_by_severity() {
    let unresolved = SyncVerdict::unresolved("a");
    let not = SyncVerdict::not_discharged("b");
    let refused = SyncVerdict::Refused(KeptRefusal::Broadcast);
    assert_eq!(
        SyncVerdict::Discharged.join(unresolved.clone()),
        unresolved
    );
    assert_eq!(unresolved.clone().join(not.clone()), not);
    assert_eq!(not.clone().join(refused.clone()), refused);
    assert_eq!(
        join_all([SyncVerdict::unresolved("a"), SyncVerdict::unresolved("c")]),
        SyncVerdict::Unresolved {
            reasons: keys(&["a", "c"])
        }
    );
    assert_eq!(join_all(Vec::new()), SyncVerdict::Discharged);
}

#[test]
fn the_condvar_edge_orders_the_snapshot_in_the_concurrent14_checker() {
    let good = publication_trace(TraceShape {
        read_under_mutex: true,
        join_before_release: true,
    });
    assert!(check_concurrent(&good).is_ok(), "{:?}", check_concurrent(&good));
    let unlocked = check_concurrent(&publication_trace(TraceShape {
        read_under_mutex: false,
        join_before_release: true,
    }))
    .unwrap_err();
    assert!(
        unlocked
            .iter()
            .any(|diagnostic| matches!(diagnostic, ConcurrencyDiagnostic::DataRace { .. })),
        "{unlocked:?}"
    );
    let early = check_concurrent(&publication_trace(TraceShape {
        read_under_mutex: true,
        join_before_release: false,
    }))
    .unwrap_err();
    assert!(
        early.iter().any(|diagnostic| matches!(
            diagnostic,
            ConcurrencyDiagnostic::NonquiescentDestruction { .. }
        )),
        "{early:?}"
    );
}
