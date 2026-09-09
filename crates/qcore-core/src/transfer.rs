//! Card `CT46-SENDSYNC`. Which kinds of thing may cross a task-creation
//! boundary, derived from the kind itself rather than enumerated per profile.
//!
//! # What this is, and what it is not
//!
//! QCore is not Rust, and this module deliberately does not introduce a
//! `Send`/`Sync` *trait* or any user-declarable marker. What CT46 calls
//! "Send/Sync" is here a **closed judgment over the implemented kind
//! vocabulary**: given a kind, [`transferable`] says whether a value of that
//! kind may cross a task boundary carrying its own custody (the "Send"
//! direction, [`TransferClass::Duplicated`] and [`TransferClass::MovedOnly`]),
//! and, separately, whether the parent may keep it while a child also reaches
//! it (the "Sync" direction, [`MovedOnly::shareable_under`]).
//!
//! The judgment is **total**: every constructor of every closed enum that can
//! classify a name at a creation site has exactly one arm, and
//! [`TransferKind`] is the union of those enums. Nothing falls off the end of
//! a match into an implicit refusal. That totality is the point: the existing
//! admission is a three-arm whitelist with a catch-all, so a kind that is
//! refused and a kind nobody thought about are indistinguishable in it.
//!
//! This module **decides nothing on its own**. It is a judgment consulted
//! beside the existing frontend enumeration and the core's own independent
//! re-check, and the three must agree; see `§ Composition` below. It adds no
//! admission: every kind it admits was already admitted, and it can only cause
//! a refusal, never an acceptance.
//!
//! # The per-kind argument
//!
//! Each arm below is sound only because a rule that already exists governs
//! that kind. The judgment restates those rules in one place; it does not
//! establish them.
//!
//! * **`i32` — duplicated.** A copy capture reads a slot's value. Nothing is
//!   split, because the ledger records no resource for a scalar at all
//!   (`scoped_tasks`, module header): a copied `i32` has no identity to be
//!   held twice. Both the "Send" and the "Sync" question are vacuous, which is
//!   why [`TransferClass::Duplicated`] answers them together.
//!
//! * **`own<Buffer>` — moved, shared only under a protocol.** This is exactly
//!   CT46's "Send but not Sync" shape. Moving is sound because
//!   `ScopedTaskLedger::use_resource` demands exclusive authority and parks the
//!   resource under `Custodian::Reservation`, so a failed creation rolls the
//!   parent binding back and a successful one consumes it permanently. Sharing
//!   is *not* sound on its own — two tasks reaching one Buffer is a data race
//!   unless something serialises them — and is therefore admitted only under
//!   [`ShareProtocol::CountedReadShares`], whose gates are `lend`'s share
//!   budget (positive, at most `FULL_READ_AUTHORITY` outstanding, at most
//!   `MAX_READERS` holders) together with the write gate: `WriteBuffer`
//!   requires exclusive authority, so no write can occur while any share is
//!   out. That protocol makes the sharing safe by removing mutation, not by
//!   ordering it.
//!
//! * **`borrow<Buffer>` — not a value.** A read loan is the *result* of a
//!   lending capture, never a source of one. `AbstractValue::Loan::resource`
//!   returns `None`, which is what structurally prevents a loan from being
//!   dropped, returned or re-lent, and the checker separately refuses it as a
//!   result and as an entry parameter. It has no custody to send and no
//!   authority to sub-share.
//!
//! * **Thread scopes, task handles, `Result`s and error values — not values
//!   that cross.** Each is bound to the frame that created it and must be
//!   consumed there: a scope is a lexical region, a handle is joinable only by
//!   its creator, a `Result` must be matched or propagated before the frame
//!   ends, and an error value's payload is taken exactly once. Sending any of
//!   them would move a duty out of the frame that owes it.
//!
//! * **Mutex generations, guards and guard-scoped facts — not values at all.**
//!   These have no denotation in the value grammar. A mutex generation fixes
//!   its protected footprint at creation and parks the creator's full share
//!   over it, so there is no second authority to hand anywhere. A guard is
//!   minted fresh by each `Lock` and its binder is body-local, so it cannot be
//!   named at a creation site outside its own section. A fact is guard-scoped
//!   and each assertion mints a fresh identity, so a fact that outlived its
//!   guard is already refused. This is the arm CT46 cares about most, and it
//!   is a **represented refusal**: see [`ShareProtocol::CheckedMutex`].
//!
//! * **Every other declared type form — unmodeled.** The declared-type
//!   grammar has fifteen tags (`SourceTypeKind`); this profile elaborates
//!   three of them and only at fixed spellings. The rest are refused as
//!   unmodeled rather than silently absent, and [`of_declared_type`] is total
//!   over that tag set so a new tag cannot be added without an arm here.
//!
//! # Composition (fail-closed)
//!
//! The frontend computes this judgment *and* its existing enumeration at every
//! capture, and requires them to agree. Disagreement is a refusal with its own
//! message, never a silent preference for either side. So this module cannot
//! widen admission: if it admitted something the enumeration refuses, the
//! result is a refusal, and if it refuses something the enumeration admits,
//! the result is also a refusal. It is a strictly fail-closed second opinion.
//!
//! # What this does not establish
//!
//! Nothing here is a data-race proof, a native-refinement claim, or a
//! statement about C layout. It says which kinds the implemented rules already
//! govern well enough to let cross a boundary, and names the rule for each.
//! In particular it does **not** supply a mutex-mediated capture: no capture
//! mode produces [`ShareProtocol::CheckedMutex`], and that is recorded as a
//! refusal rather than an omission precisely so the gap is nameable.

/// Every kind of thing a name can denote at a task-creation boundary in the
/// implemented source language.
///
/// This is the union of the closed enums that actually classify bindings —
/// the executable scalar grammar, the lowering's binding grammar, and the
/// mutex route's three binders, which have no value type at all — plus one
/// arm for the declared type forms this profile does not elaborate. The
/// judgment's totality is stated against *this* enum, and
/// [`of_declared_type`] connects it to the declared-type tag set.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum TransferKind {
    /// The unit value. It is a result type, never a parameter or capture.
    Unit,
    /// An immutable 32-bit integer.
    I32,
    /// A heap Buffer whose custody is affine.
    OwnedBuffer,
    /// A scoped read loan of a Buffer someone else still owns.
    ReadLoan,
    /// A `thread scope` region.
    ThreadScope,
    /// A `spawn` handle, joinable by its creator only.
    TaskHandle,
    /// A worker outcome awaiting a `match` or `?`.
    TaskResult,
    /// A bound error value, possibly still holding its payload.
    ErrorValue,
    /// A single known error tag.
    KnownErrorTag,
    /// A mutex generation declared by `let m = mutex guards r;`.
    MutexGeneration,
    /// A guard minted by one `with lock` acquisition.
    Guard,
    /// A guard-scoped refinement fact.
    ProtectedFact,
    /// A declared type form this profile does not elaborate.
    Unmodeled,
}

impl TransferKind {
    /// Every constructor, in declaration order.
    ///
    /// Exhaustiveness of the judgment is checked against this list, so an
    /// added constructor that no arm handles is a compile error here and a
    /// test failure in the totality tests.
    pub const ALL: [TransferKind; 13] = [
        TransferKind::Unit,
        TransferKind::I32,
        TransferKind::OwnedBuffer,
        TransferKind::ReadLoan,
        TransferKind::ThreadScope,
        TransferKind::TaskHandle,
        TransferKind::TaskResult,
        TransferKind::ErrorValue,
        TransferKind::KnownErrorTag,
        TransferKind::MutexGeneration,
        TransferKind::Guard,
        TransferKind::ProtectedFact,
        TransferKind::Unmodeled,
    ];
}

/// The declared-type tags of the source grammar, mirrored here so that
/// [`of_declared_type`] can be total over them without this crate depending on
/// the frontend. The frontend asserts the correspondence.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DeclaredTypeTag {
    Atomic,
    Named,
    Tuple,
    Array,
    Slice,
    Pointer,
    Owned,
    Borrow,
    Quant,
    AccessQuant,
    Result,
    Function,
    Never,
    Task,
    Mutex,
}

impl DeclaredTypeTag {
    /// Every constructor, in the frontend's declaration order.
    pub const ALL: [DeclaredTypeTag; 15] = [
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
        DeclaredTypeTag::Result,
        DeclaredTypeTag::Function,
        DeclaredTypeTag::Never,
        DeclaredTypeTag::Task,
        DeclaredTypeTag::Mutex,
    ];
}

/// How a capture asks to cross the boundary. These mirror the four capture
/// keywords the parser accepts; `write` is parseable and has never had a
/// lowering, so it appears here as a mode the judgment refuses for every kind
/// rather than as a mode that silently does not exist.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum TransferMode {
    /// `copy PLACE`
    Copy,
    /// `move PLACE`
    Move,
    /// `read PLACE share S`
    Lend,
    /// `write PLACE`
    Write,
}

impl TransferMode {
    /// Every constructor, in the parser's keyword order.
    pub const ALL: [TransferMode; 4] = [
        TransferMode::Copy,
        TransferMode::Move,
        TransferMode::Lend,
        TransferMode::Write,
    ];
}

/// A discipline under which a kind that is not freely shareable may
/// nevertheless be reached by more than one task.
///
/// A protocol is named here only when the checker actually decides it. The
/// mutex variant is a **represented refusal**: it names the discipline CT46
/// asks for, and no capture mode produces it, so a module that needs it is
/// refused by name rather than accepted for the wrong reason.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ShareProtocol {
    /// Counted read shares: a positive share out of a fixed authority budget,
    /// at most two holders, and no write while any share is outstanding. This
    /// protocol makes sharing safe by removing mutation for the window, so it
    /// admits readers only.
    CountedReadShares,
    /// Mutex-mediated shared access, which would order mutation instead of
    /// removing it. No implemented capture mode produces this protocol; see
    /// [`SharingGap`].
    CheckedMutex,
}

/// Why a kind cannot cross a boundary.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum NotTransferable {
    /// It is not a value: it has no custody to send and no authority to share.
    NotAValue,
    /// It is bound to the frame that created it and owes that frame a duty
    /// (a join, a match, a payload take) which sending it would strand.
    OwedToCreatingFrame,
    /// It is a declared type form this profile does not elaborate.
    UnmodeledType,
    /// It is a result-only kind and never occupies a capture position.
    ResultPositionOnly,
}

/// The transfer class of a kind: the whole of what the judgment says about it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum TransferClass {
    /// Crosses by duplication. Both sides end up with independent values and
    /// no custody is split, so the shareability question does not arise.
    Duplicated,
    /// Crosses by move, consuming the parent binding. Sharing — the parent
    /// keeping it while a child also reaches it — is admitted only under the
    /// named protocol, and refused outright when that field is `None`.
    ///
    /// This is CT46's "Send but not Sync" class.
    MovedOnly {
        shareable_under: Option<ShareProtocol>,
    },
    /// Does not cross in any direction.
    Never(NotTransferable),
}

impl TransferClass {
    /// Whether a value of this class may cross carrying its own custody.
    /// CT46 calls this direction `Send`.
    pub fn sendable(self) -> bool {
        matches!(
            self,
            TransferClass::Duplicated | TransferClass::MovedOnly { .. }
        )
    }

    /// The protocol under which the parent may keep this kind while a child
    /// also reaches it, if any. CT46 calls this direction `Sync`; `None` means
    /// the kind is not shareable at all, and a `Duplicated` kind answers
    /// `None` because duplication leaves nothing shared.
    pub fn shareable_under(self) -> Option<ShareProtocol> {
        match self {
            TransferClass::MovedOnly { shareable_under } => shareable_under,
            TransferClass::Duplicated | TransferClass::Never(_) => None,
        }
    }
}

/// **The judgment.** Total over [`TransferKind`].
///
/// Each arm's justification is in the module header; this function is the
/// single place the classification lives, so a new kind cannot be admitted by
/// adding a match arm somewhere else.
pub const fn transferable(kind: TransferKind) -> TransferClass {
    match kind {
        // Scalars duplicate. The ledger records no resource for them, so
        // there is no identity that could be held twice.
        TransferKind::I32 => TransferClass::Duplicated,

        // The unit value is a result type. It never occupies a capture or
        // parameter position, so it is refused there by position, not by
        // any property of the value.
        TransferKind::Unit => TransferClass::Never(NotTransferable::ResultPositionOnly),

        // CT46's subject kind: affine custody that moves, and is shared only
        // under the counted-read-share protocol.
        TransferKind::OwnedBuffer => TransferClass::MovedOnly {
            shareable_under: Some(ShareProtocol::CountedReadShares),
        },

        // A loan is the product of lending, never a source of it.
        TransferKind::ReadLoan => TransferClass::Never(NotTransferable::NotAValue),

        // Bound to the frame that created them, and owing it a duty.
        TransferKind::ThreadScope
        | TransferKind::TaskHandle
        | TransferKind::TaskResult
        | TransferKind::ErrorValue
        | TransferKind::KnownErrorTag => TransferClass::Never(NotTransferable::OwedToCreatingFrame),

        // No denotation in the value grammar at all.
        TransferKind::MutexGeneration | TransferKind::Guard | TransferKind::ProtectedFact => {
            TransferClass::Never(NotTransferable::NotAValue)
        }

        TransferKind::Unmodeled => TransferClass::Never(NotTransferable::UnmodeledType),
    }
}

/// Map a declared-type tag onto the kind vocabulary. Total over
/// [`DeclaredTypeTag`].
///
/// The tag alone never decides an admission: `Owned` and `Borrow` are tags,
/// and only the exact spellings `own<Buffer>` and `borrow<Buffer>` elaborate,
/// so this function reports the *most* a tag could be and the caller still has
/// to have elaborated the spelling. `Atomic` covers every width the lexer
/// knows, of which only `i32` and `unit` elaborate, so it maps to `Unmodeled`
/// and the elaborated scalar is classified directly instead.
pub const fn of_declared_type(tag: DeclaredTypeTag) -> TransferKind {
    match tag {
        DeclaredTypeTag::Task => TransferKind::TaskHandle,
        DeclaredTypeTag::Result => TransferKind::TaskResult,
        DeclaredTypeTag::Mutex => TransferKind::MutexGeneration,
        // Every remaining tag either has no elaborated spelling on this route
        // (`Named`, `Tuple`, `Array`, `Slice`, `Pointer`, `Quant`,
        // `AccessQuant`, `Function`, `Never`) or is a tag whose elaborated
        // spellings are classified from the elaborated scalar rather than the
        // tag (`Atomic`, `Owned`, `Borrow`).
        DeclaredTypeTag::Atomic
        | DeclaredTypeTag::Named
        | DeclaredTypeTag::Tuple
        | DeclaredTypeTag::Array
        | DeclaredTypeTag::Slice
        | DeclaredTypeTag::Pointer
        | DeclaredTypeTag::Owned
        | DeclaredTypeTag::Borrow
        | DeclaredTypeTag::Quant
        | DeclaredTypeTag::AccessQuant
        | DeclaredTypeTag::Function
        | DeclaredTypeTag::Never => TransferKind::Unmodeled,
    }
}

/// What a capture is admitted to do, when it is admitted.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AdmittedTransfer {
    /// The child receives an independent copy; the parent keeps its binding.
    Duplicate,
    /// The child receives custody; the parent binding is consumed on success
    /// and restored on failed creation.
    MoveCustody,
    /// The child receives read authority under the named protocol; the parent
    /// keeps the binding and gets the authority back at join.
    LendShare(ShareProtocol),
}

/// Why a capture is refused.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum TransferRefusal {
    /// The kind does not cross at all, for the stated reason.
    KindNotTransferable(NotTransferable),
    /// The kind crosses, but not in the way this mode asks. A `copy` of an
    /// affine Buffer and a `move` of a scalar are both this.
    ModeNotAdmittedForKind,
    /// The kind is shareable only under a protocol this route cannot supply.
    /// The only inhabitant today is a request that would need
    /// [`ShareProtocol::CheckedMutex`]; see [`SharingGap`].
    ProtocolUnavailable(ShareProtocol),
    /// A `write` capture. The keyword parses and has never had a lowering.
    WriteCaptureUnimplemented,
    /// The transfer is admitted, but what it delivers is not what the worker
    /// declared it receives — lending a Buffer to an `own<Buffer>` parameter,
    /// or moving one to a `borrow<Buffer>` parameter.
    ReceivedKindMismatch,
}

/// **The operational judgment.** Given a kind and the mode a capture asks
/// for, decide. Total over [`TransferKind`] x [`TransferMode`] — all
/// 13 x 4 = 52 pairs.
pub const fn admits(
    kind: TransferKind,
    mode: TransferMode,
) -> Result<AdmittedTransfer, TransferRefusal> {
    // A `write` capture is refused for every kind before anything else is
    // asked, because there is no lowering for it at all.
    if matches!(mode, TransferMode::Write) {
        return Err(TransferRefusal::WriteCaptureUnimplemented);
    }
    match transferable(kind) {
        TransferClass::Never(reason) => Err(TransferRefusal::KindNotTransferable(reason)),
        TransferClass::Duplicated => match mode {
            TransferMode::Copy => Ok(AdmittedTransfer::Duplicate),
            // Moving or lending a duplicated kind is not a custody question:
            // there is no custody. The mode simply does not apply.
            TransferMode::Move | TransferMode::Lend | TransferMode::Write => {
                Err(TransferRefusal::ModeNotAdmittedForKind)
            }
        },
        TransferClass::MovedOnly { shareable_under } => match mode {
            TransferMode::Move => Ok(AdmittedTransfer::MoveCustody),
            TransferMode::Lend => match shareable_under {
                Some(protocol) => Ok(AdmittedTransfer::LendShare(protocol)),
                None => Err(TransferRefusal::ProtocolUnavailable(
                    ShareProtocol::CheckedMutex,
                )),
            },
            // Copying affine custody would duplicate an identity.
            TransferMode::Copy | TransferMode::Write => {
                Err(TransferRefusal::ModeNotAdmittedForKind)
            }
        },
    }
}

/// What the child ends up holding, given the parent's kind and an admitted
/// transfer.
///
/// Transferability alone does not decide a capture: the boundary also has to
/// agree on *what arrives*. Lending an `own<Buffer>` hands the child a
/// `borrow<Buffer>` and not a Buffer, which is the one place where the
/// parent's kind and the worker's declared parameter kind deliberately
/// differ. Stating that as a function keeps the asymmetry in the judgment
/// instead of hard-coding it at the call site.
pub const fn received_kind(source: TransferKind, admitted: AdmittedTransfer) -> TransferKind {
    match admitted {
        // A duplicate arrives as the same kind it left as.
        AdmittedTransfer::Duplicate => source,
        // Custody arrives intact.
        AdmittedTransfer::MoveCustody => source,
        // Read authority arrives as a loan, whatever it was lent from.
        AdmittedTransfer::LendShare(_) => TransferKind::ReadLoan,
    }
}

/// The whole boundary decision: the parent holds `place`, the capture asks for
/// `mode`, and the worker declares it receives `parameter`. All three must
/// agree.
///
/// This is the function the frontend cross-checks its own enumeration
/// against. It is total over `place` x `mode` x `parameter`.
pub const fn admits_capture(
    place: TransferKind,
    mode: TransferMode,
    parameter: TransferKind,
) -> Result<AdmittedTransfer, TransferRefusal> {
    match admits(place, mode) {
        Err(refusal) => Err(refusal),
        Ok(admitted) => {
            // `==` is not `const` for this type, so compare discriminants the
            // long way; both sides are plain C-like constructors.
            if received_kind(place, admitted) as u8 == parameter as u8 {
                Ok(admitted)
            } else {
                Err(TransferRefusal::ReceivedKindMismatch)
            }
        }
    }
}

/// The gap CT46 names, stated as data rather than as prose.
///
/// CT46 asks for a payload that is sendable but not freely shareable to be
/// shared *through a verified mutex protocol*. The judgment supplies the
/// classification half of that: [`TransferKind::OwnedBuffer`] is exactly the
/// sendable-not-freely-shareable kind, and
/// [`ShareProtocol::CheckedMutex`] is exactly the discipline that would admit
/// it. What is missing is the *capture form*: no [`TransferMode`] requests
/// mutex mediation, so no source text can ask for it and nothing can grant it.
///
/// This value exists so the residue is testable, not merely written down.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SharingGap {
    /// The kind that is sendable but not freely shareable.
    pub kind: TransferKind,
    /// The protocol that would admit sharing it with mutation.
    pub protocol: ShareProtocol,
    /// Whether any capture mode can request that protocol today.
    pub requestable: bool,
}

/// The one recorded sharing gap. `requestable` is `false`, and a test pins
/// that no `(kind, mode)` pair anywhere in the judgment yields
/// [`ShareProtocol::CheckedMutex`], so this constant cannot drift away from
/// the judgment silently.
pub const MUTEX_SHARING_GAP: SharingGap = SharingGap {
    kind: TransferKind::OwnedBuffer,
    protocol: ShareProtocol::CheckedMutex,
    requestable: false,
};

#[cfg(test)]
#[path = "../tests/unit/transfer.rs"]
mod tests;
