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
        run_waiter(
            &[(false, Wakeup::Spurious), (false, Wakeup::Signalled)],
            true
        ),
        None
    );
    // Only an observed predicate claims.
    assert_eq!(
        run_waiter(
            &[(false, Wakeup::Spurious), (true, Wakeup::Signalled)],
            true
        ),
        Some(true)
    );
    // A timeout that breaks leaves without a claim.
    assert_eq!(
        run_waiter(
            &[(false, Wakeup::Spurious), (false, Wakeup::TimedOut)],
            true
        ),
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
        descendants: Vec::new(),
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
        descendants: Vec::new(),
    };
    assert!(matches!(
        release_verdict(&mixed),
        SyncVerdict::NotDischarged { .. }
    ));
}

#[test]
fn an_unresolved_descendant_withholds_the_discharge() {
    let mut site = live(JoinEvidence::CheckedSuccess);
    site.unresolved
        .insert("the children of `flb_lib_worker`".to_owned());
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
    assert_eq!(SyncVerdict::Discharged.join(unresolved.clone()), unresolved);
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
    assert!(
        check_concurrent(&good).is_ok(),
        "{:?}",
        check_concurrent(&good)
    );
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

// ------------------------------------------ W47-CONDVAR-JOIN-STOP-PATHS

fn all_excluded() -> JoinExclusions {
    JoinExclusions {
        joinable: Ok(()),
        not_busy: Ok(()),
        not_self: Ok(()),
        no_cycle: Ok(()),
    }
}

#[test]
fn an_unchecked_first_join_counts_only_with_every_posix_case_excluded() {
    assert!(join_succeeds(&all_excluded()).is_ok());
    // Negative controls first: each case left open refuses on its own.
    let detached = JoinExclusions {
        joinable: Err("the wrapper sets PTHREAD_CREATE_DETACHED".to_owned()),
        ..all_excluded()
    };
    let busy = JoinExclusions {
        not_busy: Err("the join at x.c:1 runs on 6 other threads".to_owned()),
        ..all_excluded()
    };
    let own = JoinExclusions {
        not_self: Err("the joining thread's own entry".to_owned()),
        ..all_excluded()
    };
    let cycle = JoinExclusions {
        no_cycle: Err("pthread_self() at y.c:2".to_owned()),
        ..all_excluded()
    };
    for (exclusions, case) in [
        (&detached, "not joinable"),
        (&busy, "already joined or awaited"),
        (&own, "a self join"),
        (&cycle, "a join cycle"),
    ] {
        let reason = join_succeeds(exclusions).unwrap_err();
        assert!(reason.contains(case), "{reason}");
        let verdict = release_verdict(&live(JoinEvidence::UncheckedFirst(exclusions.clone())));
        match verdict {
            SyncVerdict::NotDischarged { reason } => {
                assert!(
                    reason.contains("is not checked") && reason.contains(case),
                    "{reason}"
                );
            }
            other => panic!("{other:?}"),
        }
    }
    // With every case excluded the unchecked first join admits, and the
    // decision names P-POSIX-JOIN.
    let site = live(JoinEvidence::UncheckedFirst(all_excluded()));
    assert_eq!(release_verdict(&site), SyncVerdict::Discharged);
    assert_eq!(release_premises(&site), BTreeSet::from(["P-POSIX-JOIN"]));
    assert!(release_premises(&live(JoinEvidence::CheckedSuccess)).is_empty());
}

#[test]
fn a_repeated_join_never_ends_a_thread() {
    match release_verdict(&live(JoinEvidence::Repeated)) {
        SyncVerdict::NotDischarged { reason } => {
            assert!(reason.contains("joined a second time"), "{reason}");
        }
        other => panic!("{other:?}"),
    }
    // A repeated join ranks before an unchecked first join: it is the reason.
    let site = ReleaseSite {
        live: vec![LiveThread {
            thread: "flb_lib_worker".to_owned(),
            paths: vec![
                (JoinEvidence::CheckedSuccess, "main".to_owned()),
                (
                    JoinEvidence::UncheckedFirst(JoinExclusions {
                        not_busy: Err("busy".to_owned()),
                        ..all_excluded()
                    }),
                    "status".to_owned(),
                ),
                (JoinEvidence::Repeated, "after do_start".to_owned()),
            ],
        }],
        ..ReleaseSite::default()
    };
    match release_verdict(&site) {
        SyncVerdict::NotDischarged { reason } => {
            assert!(reason.contains("after do_start"), "{reason}");
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn a_descendant_blocks_the_release_until_its_join_is_derived() {
    let with = |evidence: DescendantEvidence| {
        let mut site = live(JoinEvidence::CheckedSuccess);
        site.descendants.push(DescendantJoin {
            parent: "flb_lib_worker".to_owned(),
            child: "input_thread".to_owned(),
            evidence,
        });
        release_verdict(&site)
    };
    assert_eq!(with(DescendantEvidence::Joined), SyncVerdict::Discharged);
    assert!(matches!(
        with(DescendantEvidence::Unresolved(
            "the engine loop is not read".to_owned()
        )),
        SyncVerdict::Unresolved { .. }
    ));
    match with(DescendantEvidence::NeverJoined(
        "no join names it".to_owned(),
    )) {
        SyncVerdict::NotDischarged { reason } => {
            assert!(reason.contains("never joined"), "{reason}");
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn a_null_arm_is_excluded_only_with_its_whole_derivation() {
    let derived = NullBranchEvidence {
        dominating_dereference: Some("src/flb_lib.c:1047".to_owned()),
        local_defeat: None,
        base_type: Some("struct flb_lib_ctx".to_owned()),
        stores: vec![("src/flb_lib.c:174".to_owned(), Ok(()))],
        unresolved: Vec::new(),
    };
    assert!(null_branch_excluded(&derived).is_ok());
    // Negative controls: every missing part keeps the null arm.
    for (evidence, words) in [
        (
            NullBranchEvidence {
                dominating_dereference: None,
                ..derived.clone()
            },
            "no earlier dereference",
        ),
        (
            NullBranchEvidence {
                local_defeat: Some("`ctx` is assigned (line 3)".to_owned()),
                ..derived.clone()
            },
            "is assigned",
        ),
        (
            NullBranchEvidence {
                base_type: None,
                ..derived.clone()
            },
            "type is not read",
        ),
        (
            NullBranchEvidence {
                unresolved: vec!["x.c:9: the address of `c->config` is taken".to_owned()],
                ..derived.clone()
            },
            "not ruled out",
        ),
        (
            NullBranchEvidence {
                stores: vec![(
                    "x.c:12".to_owned(),
                    Err("`c` is used before the store".to_owned()),
                )],
                ..derived.clone()
            },
            "not a pre-publication store",
        ),
    ] {
        let reason = null_branch_excluded(&evidence).unwrap_err();
        assert!(reason.contains(words), "{reason}");
    }
}
