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

//! Card `CT46-SENDSYNC`: the transferability judgment.
//!
//! These tests pin totality over the kind vocabulary, the per-kind
//! classification, the whole 17 x 6 admission matrix (13 x 4 before card
//! `WRITE-LEND-TASKS` grew both vocabularies by one, 14 x 5 before card
//! `MUTEX-SHARE-CROSS` grew them by two and one, and 16 x 6 before card
//! `W48-QR-SCALAR-TYPES-R4` added the fixed-width scalar kind), and the negative properties
//! that make the judgment worth having: it never admits a kind whose class
//! refuses it, each sharing protocol is reachable by exactly one mode, and no
//! kind is *lent* under the mutex protocol.

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
    // Card `MUTEX-SHARE-CROSS`: 14 -> 16, adding the checked-mutex route's
    // payload (`ProtectedResource`) and what a mutex-mediated crossing
    // delivers (`MutexShare`). Card `W48-QR-SCALAR-TYPES-R4`: 16 -> 17,
    // adding the fixed-width scalars other than `i32` (`FixedScalar`).
    assert_eq!(TransferKind::ALL.len(), 17);
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
    // Card `MUTEX-SHARE-CROSS`: 5 -> 6, adding `share PLACE under MUTEX`.
    assert_eq!(seen.len(), 6);
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
        (TransferKind::FixedScalar, Duplicated),
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
        (
            TransferKind::ProtectedResource,
            MovedOnly {
                shareable_under: Some(ShareProtocol::CheckedMutex),
                mutably_shareable_under: Some(ShareProtocol::CheckedMutex),
            },
        ),
        (TransferKind::MutexShare, Never(NotAValue)),
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
        vec![
            TransferKind::I32,
            // Card `W48-QR-SCALAR-TYPES-R4`: the other fixed-width scalars,
            // on `i32`'s footing.
            TransferKind::FixedScalar,
            TransferKind::OwnedBuffer,
            // Card `MUTEX-SHARE-CROSS`: the checked-mutex route's payload is
            // the second sendable kind. Custody of it really moves in the
            // model, over a derivable synchronization edge.
            TransferKind::ProtectedResource,
        ],
        "a scalar, a Buffer and a protected resource cross carrying something"
    );

    let shareable: Vec<_> = TransferKind::ALL
        .into_iter()
        .filter(|kind| transferable(*kind).shareable_under().is_some())
        .collect();
    assert_eq!(
        shareable,
        vec![TransferKind::OwnedBuffer, TransferKind::ProtectedResource],
        "two kinds a protocol can share, and they share under different ones"
    );

    // The CT46 shape itself, now in both of its forms — and the point of the
    // pin is that the two protocols are *different*. The Buffer shares by
    // removing mutation; the protected resource shares by ordering it.
    assert_eq!(
        transferable(TransferKind::OwnedBuffer).shareable_under(),
        Some(ShareProtocol::CountedReadShares)
    );
    assert_eq!(
        transferable(TransferKind::ProtectedResource).shareable_under(),
        Some(ShareProtocol::CheckedMutex)
    );
    assert_ne!(
        transferable(TransferKind::OwnedBuffer).shareable_under(),
        transferable(TransferKind::ProtectedResource).shareable_under(),
        "the two sharing disciplines must not become interchangeable"
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
            (TransferKind::FixedScalar, Copy),
            (TransferKind::OwnedBuffer, Move),
            (TransferKind::OwnedBuffer, Lend),
            (TransferKind::OwnedBuffer, LendWrite),
            // Card `MUTEX-SHARE-CROSS`: the fifth and sixth cells. The move
            // arm has no source spelling on the checked-mutex route today and
            // is a stated boundary, not an exercised admission.
            (TransferKind::ProtectedResource, Move),
            (TransferKind::ProtectedResource, ShareUnderMutex),
        ],
        "exactly the seven admitted capture forms"
    );
    assert_eq!(
        admits(TransferKind::ProtectedResource, ShareUnderMutex),
        Ok(AdmittedTransfer::ShareUnderMutex(
            ShareProtocol::CheckedMutex
        ))
    );
    // The cross-protocol refusals, which are what "refuses when it crosses any
    // other way" means: each sharing mode requests one named protocol, and a
    // kind sharing under a different one refuses with that one named.
    assert_eq!(
        admits(TransferKind::OwnedBuffer, ShareUnderMutex),
        Err(ProtocolUnavailable(ShareProtocol::CountedReadShares)),
        "an own<Buffer> has no mutex to cross under; it shares by removing mutation"
    );
    assert_eq!(
        admits(TransferKind::ProtectedResource, Lend),
        Err(ProtocolUnavailable(ShareProtocol::CheckedMutex)),
        "a protected resource does not cross as a counted read share"
    );
    assert_eq!(
        admits(TransferKind::ProtectedResource, LendWrite),
        Err(ProtocolUnavailable(ShareProtocol::CheckedMutex)),
        "nor as a disjoint byte partition"
    );
    assert_eq!(
        admits(TransferKind::ProtectedResource, Copy),
        Err(ModeNotAdmittedForKind)
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
        for mode in [Copy, Move, Lend, LendWrite, ShareUnderMutex] {
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
                // Card `MUTEX-SHARE-CROSS`: a mutex-mediated crossing is
                // admitted only under the protocol the class names in *both*
                // sharing directions, because a guard holder may read and
                // write the same bytes.
                Ok(AdmittedTransfer::ShareUnderMutex(protocol)) => {
                    assert_eq!(
                        class.shareable_under(),
                        Some(protocol),
                        "{kind:?} mutex-shared under a protocol its class does not name"
                    );
                    assert_eq!(
                        class.mutably_shareable_under(),
                        Some(protocol),
                        "{kind:?} mutex-shared for reading but not for writing"
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

/// The whole three-way boundary decision: 17 x 6 x 17 = 1,734 cells, of which
/// exactly seven admit (980 and four before card `MUTEX-SHARE-CROSS`; 1,536
/// and six before card `W48-QR-SCALAR-TYPES-R4`). This is
/// the function the frontend cross-checks against, so its admitted set is the
/// load-bearing claim.
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
                TransferKind::FixedScalar,
                TransferMode::Copy,
                TransferKind::FixedScalar
            ),
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
            // Card `MUTEX-SHARE-CROSS`. The move arm is admitted by the class
            // and has no source spelling on the checked-mutex route, so it is
            // a stated boundary rather than an exercised admission; the mutex
            // crossing below is this card's subject, and what it delivers is a
            // `MutexShare` — never the payload itself.
            (
                TransferKind::ProtectedResource,
                TransferMode::Move,
                TransferKind::ProtectedResource
            ),
            (
                TransferKind::ProtectedResource,
                TransferMode::ShareUnderMutex,
                TransferKind::MutexShare
            ),
        ],
        "exactly the seven admitted capture forms, and nothing else in 1,734 cells"
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

/// Card `MUTEX-SHARE-CROSS`, the headline pin, **moved deliberately**.
///
/// Card `CT46-SENDSYNC` pinned the opposite of this test: the mutex protocol
/// was named by the judgment and requestable by nothing, and that test said in
/// its own doc comment that "adding a mutex-mediated capture mode later must
/// update this test". This card adds that mode, so the test is updated rather
/// than deleted, and it now pins the two facts that replaced the one it
/// pinned: the protocol is requestable for exactly one kind, and by exactly
/// one mode.
#[test]
fn the_mutex_protocol_is_requestable_for_exactly_one_kind() {
    // The half that stays open, unchanged and still `false`: the *task*
    // route's Buffer has no mutex to cross under. Residual
    // `MUTEX-SHARE-CROSS-TASK-BUFFER`.
    assert_eq!(
        MUTEX_SHARING_GAP,
        SharingGap {
            kind: TransferKind::OwnedBuffer,
            protocol: ShareProtocol::CheckedMutex,
            requestable: false,
        }
    );
    // The half this card closes.
    assert_eq!(
        MUTEX_SHARE_CROSSING,
        SharingGap {
            kind: TransferKind::ProtectedResource,
            protocol: ShareProtocol::CheckedMutex,
            requestable: true,
        }
    );

    // Quantified, in both directions, so neither constant can drift away from
    // the judgment: exactly one kind shares under the mutex protocol, and
    // exactly one (kind, mode) pair anywhere in the matrix yields it.
    let mutex_shared: Vec<_> = TransferKind::ALL
        .into_iter()
        .filter(|kind| transferable(*kind).shareable_under() == Some(ShareProtocol::CheckedMutex))
        .collect();
    assert_eq!(mutex_shared, vec![MUTEX_SHARE_CROSSING.kind]);

    let mutex_admitted: Vec<_> = TransferKind::ALL
        .into_iter()
        .flat_map(|kind| TransferMode::ALL.map(move |mode| (kind, mode)))
        .filter(|(kind, mode)| {
            matches!(
                admits(*kind, *mode),
                Ok(AdmittedTransfer::ShareUnderMutex(
                    ShareProtocol::CheckedMutex
                ))
            )
        })
        .collect();
    assert_eq!(
        mutex_admitted,
        vec![(
            MUTEX_SHARE_CROSSING.kind,
            ShareProtocol::CheckedMutex.requested_by()
        )],
        "the mutex protocol is reachable by exactly one capture form"
    );

    // The old shape specifically, and it still holds: no kind is *lent* under
    // the mutex protocol. A mutex-mediated crossing is its own admitted
    // transfer, not a read lend wearing the mutex's name — which is what keeps
    // the read-share route's ledger rules from being asked to carry a mutex.
    for kind in TransferKind::ALL {
        for mode in TransferMode::ALL {
            assert_ne!(
                admits(kind, mode),
                Ok(AdmittedTransfer::LendShare(ShareProtocol::CheckedMutex)),
                "{kind:?} under {mode:?} lent under a mutex"
            );
        }
    }

    // Every protocol is requested by exactly one mode, and no other mode
    // anywhere in the matrix can reach it. This is the property that makes
    // "refuses when it crosses any other way" quantified rather than
    // exemplified.
    for protocol in ShareProtocol::ALL {
        let requester = protocol.requested_by();
        for mode in TransferMode::ALL {
            if mode == requester {
                continue;
            }
            for kind in TransferKind::ALL {
                if let Ok(admitted) = admits(kind, mode) {
                    let named = match admitted {
                        AdmittedTransfer::LendShare(p)
                        | AdmittedTransfer::LendWriteRange(p)
                        | AdmittedTransfer::ShareUnderMutex(p) => Some(p),
                        AdmittedTransfer::Duplicate | AdmittedTransfer::MoveCustody => None,
                    };
                    assert_ne!(
                        named,
                        Some(protocol),
                        "{kind:?} reached {protocol:?} through {mode:?}, not {requester:?}"
                    );
                }
            }
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

// ---------------------------------------------------------------------------
// Card `W48-QR-TASK-RECORD-CAPTURES`: the aggregate kind, decided from its
// leaves. The matrices below are re-decided **whole** over every leaf list of
// length at most two drawn from the complete kind vocabulary (1 + 17 + 289 =
// 307 records) and every mode — and the flat matrices above are untouched.
// ---------------------------------------------------------------------------

/// Every leaf list of length 0, 1 and 2 over the whole kind vocabulary.
fn short_leaf_lists() -> Vec<Vec<TransferKind>> {
    let mut lists = vec![vec![]];
    for a in TransferKind::ALL {
        lists.push(vec![a]);
        for b in TransferKind::ALL {
            lists.push(vec![a, b]);
        }
    }
    lists
}

/// The aggregate judgment is the conjunction of its leaves: `copy` of a record
/// is admitted exactly when every leaf admits `copy`, and `move` exactly when
/// every leaf admits the mode the record asks of it and some leaf moves.
#[test]
fn the_aggregate_judgment_is_the_conjunction_of_its_leaves() {
    let lists = short_leaf_lists();
    assert_eq!(lists.len(), 307);
    for leaves in lists {
        let copy = admits_aggregate(&leaves, TransferMode::Copy);
        let every_leaf_copies = leaves
            .iter()
            .all(|leaf| admits(*leaf, TransferMode::Copy).is_ok());
        assert_eq!(copy.is_ok(), every_leaf_copies, "{leaves:?}");
        if let Ok(admitted) = copy {
            assert_eq!(admitted, AdmittedTransfer::Duplicate);
        }
        let moved = admits_aggregate(&leaves, TransferMode::Move);
        let every_leaf_crosses = leaves
            .iter()
            .all(|leaf| admits(*leaf, leaf_mode(*leaf, TransferMode::Move)).is_ok());
        let some_leaf_moves = leaves.iter().any(|leaf| {
            admits(*leaf, leaf_mode(*leaf, TransferMode::Move)) == Ok(AdmittedTransfer::MoveCustody)
        });
        assert_eq!(
            moved.is_ok(),
            every_leaf_crosses && some_leaf_moves,
            "{leaves:?}"
        );
        if let Ok(admitted) = moved {
            assert_eq!(admitted, AdmittedTransfer::MoveCustody);
        }
        // The class agrees with the admission in both modes.
        let class = aggregate_class(&leaves);
        assert_eq!(
            copy.is_ok(),
            class == TransferClass::Duplicated,
            "{leaves:?}"
        );
        assert_eq!(
            moved.is_ok(),
            matches!(class, TransferClass::MovedOnly { .. }),
            "{leaves:?}"
        );
    }
}

/// A refusal names the first leaf that fails, with that leaf's own flat
/// refusal — never a later leaf and never a leaf that crosses.
#[test]
fn an_aggregate_refusal_names_the_first_failing_leaf() {
    for leaves in short_leaf_lists() {
        for mode in [TransferMode::Copy, TransferMode::Move] {
            if let Err(AggregateRefusal::Leaf { index, refusal }) = admits_aggregate(&leaves, mode)
            {
                let leaf = leaves[index];
                assert_eq!(admits(leaf, leaf_mode(leaf, mode)), Err(refusal));
                for earlier in &leaves[..index] {
                    assert!(admits(*earlier, leaf_mode(*earlier, mode)).is_ok());
                }
            }
        }
    }
    // The case the task route meets: a record owning a Buffer, asked to copy.
    assert_eq!(
        admits_aggregate(
            &[TransferKind::I32, TransferKind::OwnedBuffer],
            TransferMode::Copy
        ),
        Err(AggregateRefusal::Leaf {
            index: 1,
            refusal: TransferRefusal::ModeNotAdmittedForKind
        })
    );
    // A copy record asked to move is refused for its mode, naming no leaf.
    assert_eq!(
        admits_aggregate(
            &[TransferKind::I32, TransferKind::FixedScalar],
            TransferMode::Move
        ),
        Err(AggregateRefusal::ModeNotAdmittedForRecord)
    );
}

/// `write` is refused by name for every record, before any leaf is asked; and
/// no sharing mode admits a record whole.
#[test]
fn a_write_capture_of_a_record_is_refused_by_name() {
    for leaves in short_leaf_lists() {
        assert_eq!(
            admits_aggregate(&leaves, TransferMode::Write),
            Err(AggregateRefusal::WriteCaptureUnimplemented),
            "{leaves:?}"
        );
        for mode in [
            TransferMode::Lend,
            TransferMode::LendWrite,
            TransferMode::ShareUnderMutex,
        ] {
            assert_eq!(
                admits_aggregate(&leaves, mode),
                Err(AggregateRefusal::NoWholeRecordProtocol),
                "{leaves:?} {mode:?}"
            );
        }
    }
}

/// The aggregate boundary is decided exactly once: every (record, mode,
/// parameter record) cell is admitted or refused, and an admitted one delivers
/// the record type it left as, so another record type refuses.
#[test]
fn the_aggregate_boundary_is_decided_exactly_once() {
    for leaves in short_leaf_lists() {
        for mode in TransferMode::ALL {
            let alone = admits_aggregate(&leaves, mode);
            let same = admits_aggregate_capture(&"P", &leaves, mode, &"P");
            let other = admits_aggregate_capture(&"P", &leaves, mode, &"Q");
            assert_eq!(same, alone, "{leaves:?} {mode:?}");
            match alone {
                Ok(_) => assert_eq!(other, Err(AggregateRefusal::ReceivedKindMismatch)),
                Err(refusal) => assert_eq!(other, Err(refusal)),
            }
        }
    }
}

/// A one-leaf record never admits more than its leaf does in the mode the
/// record asks of it.
#[test]
fn a_one_leaf_record_never_admits_more_than_its_leaf() {
    for kind in TransferKind::ALL {
        for mode in TransferMode::ALL {
            if admits_aggregate(&[kind], mode).is_ok() {
                assert!(
                    admits(kind, leaf_mode(kind, mode)).is_ok(),
                    "{kind:?} {mode:?}"
                );
            }
        }
    }
}
