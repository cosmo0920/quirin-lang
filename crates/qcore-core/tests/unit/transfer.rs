//! Card `CT46-SENDSYNC`: the transferability judgment.
//!
//! These tests pin totality over the kind vocabulary, the per-kind
//! classification, the whole 14 x 5 admission matrix (13 x 4 before card
//! `WRITE-LEND-TASKS` grew both vocabularies by one), and the two negative
//! properties that make the judgment worth having: it never admits a kind
//! whose class refuses it, and no mode anywhere requests the mutex protocol.

use super::*;

/// Totality: every constructor is in `ALL`, and `ALL` has no duplicates.
///
/// `transferable` is a `match` with no wildcard, so the compiler already
/// forces an arm per constructor; this test forces `ALL` to stay in step with
/// it, which is what the matrix tests below quantify over.
#[test]
fn kind_inventory_is_complete_and_duplicate_free() {
    let mut seen = std::collections::BTreeSet::new();
    for kind in TransferKind::ALL {
        assert!(seen.insert(kind), "duplicate kind in ALL: {kind:?}");
    }
    assert_eq!(seen.len(), TransferKind::ALL.len());
    assert_eq!(TransferKind::ALL.len(), 14);
}

#[test]
fn declared_tag_inventory_is_complete_and_duplicate_free() {
    let mut seen = std::collections::BTreeSet::new();
    for tag in DeclaredTypeTag::ALL {
        assert!(seen.insert(tag), "duplicate tag in ALL: {tag:?}");
    }
    assert_eq!(seen.len(), 15);
}

#[test]
fn mode_inventory_is_complete_and_duplicate_free() {
    let mut seen = std::collections::BTreeSet::new();
    for mode in TransferMode::ALL {
        assert!(seen.insert(mode), "duplicate mode in ALL: {mode:?}");
    }
    assert_eq!(seen.len(), 5);
}

/// The judgment is total: it answers for every kind, and `of_declared_type`
/// answers for every declared tag. A panic or a hang here would be a
/// totality failure; a missing arm is a compile error.
#[test]
fn judgment_is_total_over_both_vocabularies() {
    for kind in TransferKind::ALL {
        let class = transferable(kind);
        // Every class is one of the three, and `sendable`/`shareable_under`
        // agree with it.
        match class {
            TransferClass::Duplicated => {
                assert!(class.sendable());
                assert_eq!(class.shareable_under(), None);
            }
            TransferClass::MovedOnly {
                shareable_under,
                mutably_shareable_under,
            } => {
                assert!(class.sendable());
                assert_eq!(class.shareable_under(), shareable_under);
                assert_eq!(class.mutably_shareable_under(), mutably_shareable_under);
            }
            TransferClass::Never(_) => {
                assert!(!class.sendable());
                assert_eq!(class.shareable_under(), None);
            }
        }
    }
    for tag in DeclaredTypeTag::ALL {
        // Total, and never claims more than the elaborated spelling could be.
        let kind = of_declared_type(tag);
        assert!(TransferKind::ALL.contains(&kind));
    }
}

/// The per-kind classification, written out. This is the table the report
/// carries; if an arm changes, this test names it.
#[test]
fn per_kind_classification_is_pinned() {
    use NotTransferable::*;
    use TransferClass::*;
    let expected = [
        (TransferKind::Unit, Never(ResultPositionOnly)),
        (TransferKind::I32, Duplicated),
        (
            TransferKind::OwnedBuffer,
            MovedOnly {
                shareable_under: Some(ShareProtocol::CountedReadShares),
                mutably_shareable_under: Some(ShareProtocol::DisjointWriteLeases),
            },
        ),
        (TransferKind::ReadLoan, Never(NotAValue)),
        (TransferKind::WriteLease, Never(NotAValue)),
        (TransferKind::ThreadScope, Never(OwedToCreatingFrame)),
        (TransferKind::TaskHandle, Never(OwedToCreatingFrame)),
        (TransferKind::TaskResult, Never(OwedToCreatingFrame)),
        (TransferKind::ErrorValue, Never(OwedToCreatingFrame)),
        (TransferKind::KnownErrorTag, Never(OwedToCreatingFrame)),
        (TransferKind::MutexGeneration, Never(NotAValue)),
        (TransferKind::Guard, Never(NotAValue)),
        (TransferKind::ProtectedFact, Never(NotAValue)),
        (TransferKind::Unmodeled, Never(UnmodeledType)),
    ];
    assert_eq!(expected.len(), TransferKind::ALL.len());
    for (kind, class) in expected {
        assert_eq!(transferable(kind), class, "classification of {kind:?}");
    }
}

/// CT46's own vocabulary: exactly one kind is sendable-but-not-freely-
/// shareable, and it is the Buffer. `i32` is sendable and has nothing to
/// share; everything else is neither.
#[test]
fn send_and_sync_directions_separate_exactly_one_kind() {
    let sendable: Vec<_> = TransferKind::ALL
        .into_iter()
        .filter(|kind| transferable(*kind).sendable())
        .collect();
    assert_eq!(
        sendable,
        vec![TransferKind::I32, TransferKind::OwnedBuffer],
        "only a scalar and a Buffer cross carrying anything"
    );

    let shareable: Vec<_> = TransferKind::ALL
        .into_iter()
        .filter(|kind| transferable(*kind).shareable_under().is_some())
        .collect();
    assert_eq!(
        shareable,
        vec![TransferKind::OwnedBuffer],
        "the Buffer is the only kind a protocol can share"
    );

    // The CT46 shape itself: Send, not freely Sync, shareable only under a
    // named protocol that removes mutation.
    assert_eq!(
        transferable(TransferKind::OwnedBuffer).shareable_under(),
        Some(ShareProtocol::CountedReadShares)
    );
}

/// The whole 14 x 5 admission matrix, written out. Four cells admit; the
/// other 66 refuse, each with its own reason.
#[test]
fn admission_matrix_is_pinned() {
    use TransferMode::*;
    use TransferRefusal::*;
    let admitted: Vec<_> = TransferKind::ALL
        .into_iter()
        .flat_map(|kind| TransferMode::ALL.map(move |mode| (kind, mode)))
        .filter(|(kind, mode)| admits(*kind, *mode).is_ok())
        .collect();
    assert_eq!(
        admitted,
        vec![
            (TransferKind::I32, Copy),
            (TransferKind::OwnedBuffer, Move),
            (TransferKind::OwnedBuffer, Lend),
            (TransferKind::OwnedBuffer, LendWrite),
        ],
        "exactly the four admitted capture forms"
    );
    assert_eq!(
        admits(TransferKind::I32, Copy),
        Ok(AdmittedTransfer::Duplicate)
    );
    assert_eq!(
        admits(TransferKind::OwnedBuffer, Move),
        Ok(AdmittedTransfer::MoveCustody)
    );
    assert_eq!(
        admits(TransferKind::OwnedBuffer, Lend),
        Ok(AdmittedTransfer::LendShare(
            ShareProtocol::CountedReadShares
        ))
    );
    assert_eq!(
        admits(TransferKind::OwnedBuffer, LendWrite),
        Ok(AdmittedTransfer::LendWriteRange(
            ShareProtocol::DisjointWriteLeases
        ))
    );

    // A `write` capture is refused for every kind, including the admitted
    // ones, and refused for its own reason rather than for the kind's.
    for kind in TransferKind::ALL {
        assert_eq!(
            admits(kind, Write),
            Err(WriteCaptureUnimplemented),
            "write capture of {kind:?}"
        );
    }

    // Wrong mode on a transferable kind is a mode refusal, not a kind one.
    assert_eq!(admits(TransferKind::I32, Move), Err(ModeNotAdmittedForKind));
    assert_eq!(admits(TransferKind::I32, Lend), Err(ModeNotAdmittedForKind));
    assert_eq!(
        admits(TransferKind::I32, LendWrite),
        Err(ModeNotAdmittedForKind),
        "a duplicated kind has no bytes whose write authority could cross"
    );
    assert_eq!(
        admits(TransferKind::OwnedBuffer, Copy),
        Err(ModeNotAdmittedForKind),
        "copying affine custody would duplicate an identity"
    );

    // Every non-transferable kind refuses in every non-write mode, carrying
    // the class's own reason.
    for kind in TransferKind::ALL {
        let TransferClass::Never(reason) = transferable(kind) else {
            continue;
        };
        for mode in [Copy, Move, Lend, LendWrite] {
            assert_eq!(
                admits(kind, mode),
                Err(KindNotTransferable(reason)),
                "{kind:?} under {mode:?}"
            );
        }
    }
}

/// The guard, fact and mutex arms specifically: CT46's "shared wrapper"
/// vocabulary never crosses, in any mode, for the same reason.
#[test]
fn guards_facts_and_mutexes_never_cross() {
    for kind in [
        TransferKind::Guard,
        TransferKind::ProtectedFact,
        TransferKind::MutexGeneration,
    ] {
        assert_eq!(
            transferable(kind),
            TransferClass::Never(NotTransferable::NotAValue)
        );
        for mode in TransferMode::ALL {
            assert!(admits(kind, mode).is_err(), "{kind:?} under {mode:?}");
        }
    }
}

/// Soundness of the composition: the operational judgment never admits a mode
/// for a kind whose class refuses it. Quantified over the whole matrix, so it
/// is not a restatement of one arm.
#[test]
fn admission_never_contradicts_the_class() {
    for kind in TransferKind::ALL {
        let class = transferable(kind);
        for mode in TransferMode::ALL {
            match admits(kind, mode) {
                Ok(AdmittedTransfer::Duplicate) => {
                    assert_eq!(class, TransferClass::Duplicated);
                }
                Ok(AdmittedTransfer::MoveCustody) => {
                    assert!(
                        matches!(class, TransferClass::MovedOnly { .. }),
                        "{kind:?} moved without a moving class"
                    );
                }
                Ok(AdmittedTransfer::LendShare(protocol)) => {
                    assert_eq!(
                        class.shareable_under(),
                        Some(protocol),
                        "{kind:?} lent under a protocol its class does not name"
                    );
                }
                Ok(AdmittedTransfer::LendWriteRange(protocol)) => {
                    assert_eq!(
                        class.mutably_shareable_under(),
                        Some(protocol),
                        "{kind:?} write-leased under a protocol its class does not name"
                    );
                }
                Err(_) => {}
            }
            // An admission implies the class is sendable, always.
            if admits(kind, mode).is_ok() {
                assert!(class.sendable(), "{kind:?} admitted but not sendable");
            }
        }
    }
}

/// What arrives: a duplicate and a move arrive as themselves, a lend always
/// arrives as a loan.
#[test]
fn received_kind_is_pinned() {
    assert_eq!(
        received_kind(TransferKind::I32, AdmittedTransfer::Duplicate),
        TransferKind::I32
    );
    assert_eq!(
        received_kind(TransferKind::OwnedBuffer, AdmittedTransfer::MoveCustody),
        TransferKind::OwnedBuffer
    );
    assert_eq!(
        received_kind(
            TransferKind::OwnedBuffer,
            AdmittedTransfer::LendShare(ShareProtocol::CountedReadShares)
        ),
        TransferKind::ReadLoan,
        "lending an owned Buffer delivers a loan, never a Buffer"
    );
    assert_eq!(
        received_kind(
            TransferKind::OwnedBuffer,
            AdmittedTransfer::LendWriteRange(ShareProtocol::DisjointWriteLeases)
        ),
        TransferKind::WriteLease,
        "write-leasing an owned Buffer delivers a lease, never a Buffer"
    );
}

/// The whole three-way boundary decision: 14 x 5 x 14 = 980 cells, of which
/// exactly four admit. This is the function the frontend cross-checks
/// against, so its admitted set is the load-bearing claim.
#[test]
fn capture_admission_matrix_is_pinned() {
    let admitted: Vec<_> = TransferKind::ALL
        .into_iter()
        .flat_map(|place| {
            TransferMode::ALL.into_iter().flat_map(move |mode| {
                TransferKind::ALL
                    .into_iter()
                    .map(move |parameter| (place, mode, parameter))
            })
        })
        .filter(|(place, mode, parameter)| admits_capture(*place, *mode, *parameter).is_ok())
        .collect();
    assert_eq!(
        admitted,
        vec![
            (TransferKind::I32, TransferMode::Copy, TransferKind::I32),
            (
                TransferKind::OwnedBuffer,
                TransferMode::Move,
                TransferKind::OwnedBuffer
            ),
            (
                TransferKind::OwnedBuffer,
                TransferMode::Lend,
                TransferKind::ReadLoan
            ),
            (
                TransferKind::OwnedBuffer,
                TransferMode::LendWrite,
                TransferKind::WriteLease
            ),
        ],
        "exactly the four admitted capture forms, and nothing else in 980 cells"
    );
}

/// The two shapes a `ReceivedKindMismatch` catches. Both are transfers the
/// kind judgment admits and the boundary still refuses, so the mismatch arm
/// is doing real work rather than restating the kind refusal.
#[test]
fn a_delivered_kind_that_the_worker_did_not_declare_refuses() {
    // Lending to an `own<Buffer>` parameter: admitted as a transfer, refused
    // because a lend delivers a loan.
    assert_eq!(
        admits(TransferKind::OwnedBuffer, TransferMode::Lend),
        Ok(AdmittedTransfer::LendShare(
            ShareProtocol::CountedReadShares
        ))
    );
    assert_eq!(
        admits_capture(
            TransferKind::OwnedBuffer,
            TransferMode::Lend,
            TransferKind::OwnedBuffer
        ),
        Err(TransferRefusal::ReceivedKindMismatch)
    );
    // Moving to a `borrow<Buffer>` parameter: admitted as a transfer, refused
    // because a move delivers custody.
    assert_eq!(
        admits(TransferKind::OwnedBuffer, TransferMode::Move),
        Ok(AdmittedTransfer::MoveCustody)
    );
    assert_eq!(
        admits_capture(
            TransferKind::OwnedBuffer,
            TransferMode::Move,
            TransferKind::ReadLoan
        ),
        Err(TransferRefusal::ReceivedKindMismatch)
    );
}

/// The boundary decision never contradicts the kind judgment: whenever it
/// admits, `admits` admitted too, with the same transfer.
#[test]
fn capture_admission_refines_kind_admission() {
    for place in TransferKind::ALL {
        for mode in TransferMode::ALL {
            for parameter in TransferKind::ALL {
                if let Ok(admitted) = admits_capture(place, mode, parameter) {
                    assert_eq!(
                        admits(place, mode),
                        Ok(admitted),
                        "{place:?}/{mode:?}/{parameter:?} admitted more than the kind judgment"
                    );
                    assert_eq!(received_kind(place, admitted), parameter);
                }
            }
        }
    }
}

/// The recorded gap: the mutex protocol is named by the judgment and
/// requestable by nothing. This pins the honest boundary CT46 closes at, so
/// that adding a mutex-mediated capture mode later must update this test.
#[test]
fn the_mutex_protocol_is_named_and_unreachable() {
    assert_eq!(
        MUTEX_SHARING_GAP,
        SharingGap {
            kind: TransferKind::OwnedBuffer,
            protocol: ShareProtocol::CheckedMutex,
            requestable: false,
        }
    );

    // Quantified: no (kind, mode) pair anywhere yields the mutex protocol,
    // either as an admission or as the class's sharing discipline.
    for kind in TransferKind::ALL {
        assert_ne!(
            transferable(kind).shareable_under(),
            Some(ShareProtocol::CheckedMutex),
            "{kind:?} claims mutex sharing"
        );
        for mode in TransferMode::ALL {
            assert_ne!(
                admits(kind, mode),
                Ok(AdmittedTransfer::LendShare(ShareProtocol::CheckedMutex)),
                "{kind:?} under {mode:?} admitted mutex sharing"
            );
        }
    }
}

/// The gap constant cannot drift: its kind really is the sendable-not-freely-
/// shareable one, and the protocol it names really is the one no mode reaches.
#[test]
fn the_gap_constant_agrees_with_the_judgment() {
    let class = transferable(MUTEX_SHARING_GAP.kind);
    assert!(class.sendable(), "the gap's kind must be sendable");
    assert_ne!(
        class.shareable_under(),
        Some(MUTEX_SHARING_GAP.protocol),
        "the gap's protocol must be the one the kind does NOT have"
    );
    assert!(
        class.shareable_under().is_some(),
        "the gap's kind is shareable under some other protocol, which is what \
         makes it the interesting one"
    );
}

/// `of_declared_type` maps the three tags with a denotation and refuses the
/// rest as unmodeled. It is deliberately conservative: `Owned` and `Borrow`
/// are tags whose only elaborated spellings are `own<Buffer>` and
/// `borrow<Buffer>`, and the elaborated scalar is classified directly, so the
/// tag alone must not claim them.
#[test]
fn declared_tags_map_conservatively() {
    assert_eq!(
        of_declared_type(DeclaredTypeTag::Task),
        TransferKind::TaskHandle
    );
    assert_eq!(
        of_declared_type(DeclaredTypeTag::Result),
        TransferKind::TaskResult
    );
    assert_eq!(
        of_declared_type(DeclaredTypeTag::Mutex),
        TransferKind::MutexGeneration
    );
    for tag in [
        DeclaredTypeTag::Atomic,
        DeclaredTypeTag::Named,
        DeclaredTypeTag::Tuple,
        DeclaredTypeTag::Array,
        DeclaredTypeTag::Slice,
        DeclaredTypeTag::Pointer,
        DeclaredTypeTag::Owned,
        DeclaredTypeTag::Borrow,
        DeclaredTypeTag::Quant,
        DeclaredTypeTag::AccessQuant,
        DeclaredTypeTag::Function,
        DeclaredTypeTag::Never,
    ] {
        assert_eq!(of_declared_type(tag), TransferKind::Unmodeled, "{tag:?}");
    }
    // No declared tag is sendable on its own. The elaborated scalar decides.
    for tag in DeclaredTypeTag::ALL {
        assert!(
            !transferable(of_declared_type(tag)).sendable(),
            "declared tag {tag:?} claimed sendability without elaboration"
        );
    }
}
