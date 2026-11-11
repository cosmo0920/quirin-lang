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
//! * **`protected N` — moved, shared only under a verified mutex** (card
//!   `MUTEX-SHARE-CROSS`). The checked-mutex route's payload, and the second
//!   "Send but not Sync" kind. Custody moves, because the model has
//!   `TransferCustody` admitted only over a *derivable* synchronization edge.
//!   Sharing is admitted under [`ShareProtocol::CheckedMutex`] in **both**
//!   directions — a guard holder may read and write the same bytes another
//!   task will later reach — and that is sound not because this module says so
//!   but because `mutex_invariant`'s own rule leaves a protected footprint no
//!   unguarded access at all. See [`MUTEX_SHARE_CROSSING`].
//!
//! * **Mutex generations, guards and guard-scoped facts — not values at all.**
//!   These have no denotation in the value grammar. A mutex generation fixes
//!   its protected footprint at creation and parks the creator's full share
//!   over it, so there is no second authority to hand anywhere. A guard is
//!   minted fresh by each `Lock` and its binder is body-local, so it cannot be
//!   named at a creation site outside its own section. A fact is guard-scoped
//!   and each assertion mints a fresh identity, so a fact that outlived its
//!   guard is already refused. The *wrapper* is what CT46 cares about most,
//!   and it stays a non-value: what crosses under card `MUTEX-SHARE-CROSS` is
//!   the payload the generation protects, never the generation itself.
//!
//! * **Every other declared type form — unmodeled.** The declared-type
//!   grammar has fifteen tags (`SourceTypeKind`); this profile elaborates
//!   three of them and only at fixed spellings. The rest are refused as
//!   unmodeled rather than silently absent, and [`of_declared_type`] is total
//!   over that tag set so a new tag cannot be added without an arm here.
//!
//! * **A record — an aggregate, decided from its leaves** (card
//!   `W48-QR-TASK-RECORD-CAPTURES`). [`admits_aggregate`] asks each leaf the
//!   flat judgment in the mode the record asks of it ([`leaf_mode`]); the
//!   section above that function states the whole-record conditions.
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
//! Card `MUTEX-SHARE-CROSS` supplies the mutex-mediated capture this module
//! once recorded as missing, for [`TransferKind::ProtectedResource`] and for
//! that kind alone. The judgment's part of it is the classification — *may
//! this kind cross this way* — and nothing more: whether the named generation
//! exists, protects this very payload and verifies is decided against the
//! unedited checker at the crossing. The task route's `own<Buffer>` still has
//! no mutex to cross under, so [`MUTEX_SHARING_GAP`] stays open and stays
//! `false`.

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
    /// An immutable value of any other type of the §G02t fixed-width table —
    /// `bool`, `i8`, `i16`, `i64`, `isize`, `u8`, `u16`, `u32`, `u64`, `usize`
    /// (card `W48-QR-SCALAR-TYPES-R4`). It is on `i32`'s footing exactly: a
    /// copy capture reads a slot's value, and the ledger records no resource
    /// for a scalar, so there is no identity to hold twice.
    FixedScalar,
    /// A heap Buffer whose custody is affine.
    OwnedBuffer,
    /// A scoped read loan of a Buffer someone else still owns.
    ReadLoan,
    /// A scoped range write lease of a Buffer someone else still owns (card
    /// `WRITE-LEND-TASKS`). Like a loan it is the *product* of a crossing,
    /// never a source of one.
    WriteLease,
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
    /// A resource declared by `let r = protected N;` on the checked-mutex
    /// route (card `MUTEX-SHARE-CROSS`). This is CT46's payload: custody of it
    /// really does move between tasks in the model
    /// (`mutex_invariant::canTransfer`, over a derivable synchronization
    /// witness), and two tasks may reach it at once **only** under a verified
    /// mutex protocol — never by a counted read share and never by a byte
    /// partition. So it is "Send but not Sync" with
    /// [`ShareProtocol::CheckedMutex`] as its one sharing discipline, in both
    /// the read and the write direction: ordering mutation is exactly what a
    /// mutex does that the other two protocols do not.
    ProtectedResource,
    /// What a mutex-mediated crossing delivers (card `MUTEX-SHARE-CROSS`): the
    /// right to reach a protected resource in the child, and only under a
    /// guard of the declared generation. Like [`TransferKind::ReadLoan`] and
    /// [`TransferKind::WriteLease`] it is the **product** of a crossing and
    /// never a source of one.
    MutexShare,
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
    pub const ALL: [TransferKind; 17] = [
        TransferKind::Unit,
        TransferKind::I32,
        TransferKind::FixedScalar,
        TransferKind::OwnedBuffer,
        TransferKind::ReadLoan,
        TransferKind::WriteLease,
        TransferKind::ThreadScope,
        TransferKind::TaskHandle,
        TransferKind::TaskResult,
        TransferKind::ErrorValue,
        TransferKind::KnownErrorTag,
        TransferKind::ProtectedResource,
        TransferKind::MutexShare,
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
    /// `lend PLACE LOW..HIGH` (card `WRITE-LEND-TASKS`): a range write lend.
    /// Distinct from [`TransferMode::Write`] on purpose — the plain `write`
    /// capture still parses, still has no lowering, and is still refused by
    /// name for every kind, so CT47's pinned refusal is untouched. The ranged
    /// form is a different request: it names the exact bytes whose write
    /// authority crosses, which is what makes the disjointness question
    /// decidable at the boundary.
    LendWrite,
    /// `share PLACE under MUTEX` (card `MUTEX-SHARE-CROSS`): the crossing CT46
    /// asks for. The capture names the payload **and** the protocol that
    /// protects it, so the request is explicit at the source rather than
    /// inferred from what happens to be in scope. It is the only mode that
    /// requests [`ShareProtocol::CheckedMutex`], and — unlike every other
    /// sharing mode — the protocol it names is *verified* rather than assumed:
    /// see [`MUTEX_SHARE_CROSSING`].
    ShareUnderMutex,
}

impl TransferMode {
    /// Every constructor, in the parser's keyword order.
    pub const ALL: [TransferMode; 6] = [
        TransferMode::Copy,
        TransferMode::Move,
        TransferMode::Lend,
        TransferMode::Write,
        TransferMode::LendWrite,
        TransferMode::ShareUnderMutex,
    ];
}

impl ShareProtocol {
    /// Every constructor, in declaration order.
    pub const ALL: [ShareProtocol; 3] = [
        ShareProtocol::CountedReadShares,
        ShareProtocol::CheckedMutex,
        ShareProtocol::DisjointWriteLeases,
    ];

    /// The sharing mode that requests this protocol, and only this protocol.
    ///
    /// Card `MUTEX-SHARE-CROSS` states the correspondence as a function so the
    /// judgment can be written once: **a sharing mode is admitted exactly when
    /// the kind's own sharing discipline is the protocol that mode requests.**
    /// Before this card the read-lend arm accepted whatever protocol the kind
    /// happened to name, which was harmless only because one kind shared at
    /// all; with two sharing disciplines in each direction it would have let a
    /// `read … share N` capture of a mutex-protected payload through under the
    /// mutex's name. It cannot now: the protocols are compared.
    pub const fn requested_by(self) -> TransferMode {
        match self {
            ShareProtocol::CountedReadShares => TransferMode::Lend,
            ShareProtocol::CheckedMutex => TransferMode::ShareUnderMutex,
            ShareProtocol::DisjointWriteLeases => TransferMode::LendWrite,
        }
    }
}

/// A discipline under which a kind that is not freely shareable may
/// nevertheless be reached by more than one task.
///
/// A protocol is named here only when the checker actually decides it, and
/// each one is requested by exactly one capture mode
/// ([`ShareProtocol::requested_by`]) — so a kind whose discipline is one
/// protocol refuses a mode that requests another, with its own discipline
/// named. That is what keeps the three from being interchangeable.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ShareProtocol {
    /// Counted read shares: a positive share out of a fixed authority budget,
    /// at most two holders, and no write while any share is outstanding. This
    /// protocol makes sharing safe by removing mutation for the window, so it
    /// admits readers only.
    CountedReadShares,
    /// Mutex-mediated shared access (card `MUTEX-SHARE-CROSS`): the protocol
    /// **orders** mutation instead of removing it, so a guard holder may read
    /// *and* write the very same bytes another task will later reach. It is
    /// the only protocol admitted in both sharing directions, and the only one
    /// whose verification is a separate question from its classification — the
    /// generation must exist and protect this very payload. Requested by
    /// [`TransferMode::ShareUnderMutex`] and by nothing else; see
    /// [`MUTEX_SHARE_CROSSING`].
    CheckedMutex,
    /// Disjoint range write leases (card `WRITE-LEND-TASKS`): each holder
    /// receives write authority over its own byte range, the ranges are
    /// pairwise disjoint by the wave-29 overlap decision, and the owner holds
    /// no whole-payload authority — read included — while any lease is out.
    /// This protocol makes concurrent mutation safe by **partitioning** the
    /// bytes, where [`ShareProtocol::CountedReadShares`] removes mutation and
    /// [`ShareProtocol::CheckedMutex`] would order it.
    DisjointWriteLeases,
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
    /// named protocols, and refused outright where a field is `None`:
    /// `shareable_under` is the read-sharing discipline (CT46's "Sync"
    /// direction), and `mutably_shareable_under` (card `WRITE-LEND-TASKS`)
    /// the discipline under which more than one party may hold **write**
    /// authority at once — by partitioning the bytes, never by trusting a
    /// schedule.
    ///
    /// This is CT46's "Send but not Sync" class.
    MovedOnly {
        shareable_under: Option<ShareProtocol>,
        mutably_shareable_under: Option<ShareProtocol>,
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
            TransferClass::MovedOnly {
                shareable_under, ..
            } => shareable_under,
            TransferClass::Duplicated | TransferClass::Never(_) => None,
        }
    }

    /// The protocol under which more than one party may hold **write**
    /// authority over this kind at once, if any (card `WRITE-LEND-TASKS`).
    /// Distinct from [`Self::shareable_under`] on purpose: read sharing
    /// removes mutation for the window, mutable sharing partitions the bytes,
    /// and a consumer must be able to ask for exactly one of them.
    pub fn mutably_shareable_under(self) -> Option<ShareProtocol> {
        match self {
            TransferClass::MovedOnly {
                mutably_shareable_under,
                ..
            } => mutably_shareable_under,
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
        // there is no identity that could be held twice. Card
        // `W48-QR-SCALAR-TYPES-R4` adds the rest of the fixed-width table on
        // exactly that ground.
        TransferKind::I32 | TransferKind::FixedScalar => TransferClass::Duplicated,

        // The unit value is a result type. It never occupies a capture or
        // parameter position, so it is refused there by position, not by
        // any property of the value.
        TransferKind::Unit => TransferClass::Never(NotTransferable::ResultPositionOnly),

        // CT46's subject kind: affine custody that moves, shared for reading
        // only under the counted-read-share protocol, and shared for writing
        // only under the disjoint-range-lease protocol (card
        // `WRITE-LEND-TASKS`) — whose gates are the ledger's: nonempty
        // pairwise-disjoint ranges by the wave-29 overlap decision, at most
        // `MAX_WRITERS` holders, and no whole-payload authority for anyone —
        // the owner's read included — while any lease is out.
        TransferKind::OwnedBuffer => TransferClass::MovedOnly {
            shareable_under: Some(ShareProtocol::CountedReadShares),
            mutably_shareable_under: Some(ShareProtocol::DisjointWriteLeases),
        },

        // A loan is the product of lending, never a source of it; a lease is
        // the product of leasing, on the same footing; a mutex share is the
        // product of a mutex-mediated crossing, on the same footing again.
        TransferKind::ReadLoan => TransferClass::Never(NotTransferable::NotAValue),
        TransferKind::WriteLease => TransferClass::Never(NotTransferable::NotAValue),
        TransferKind::MutexShare => TransferClass::Never(NotTransferable::NotAValue),

        // Card `MUTEX-SHARE-CROSS`: CT46's payload. Custody moves — the model
        // has `TransferCustody`, admitted only over a *derivable*
        // synchronization edge, never a caller-supplied flag. Sharing is
        // admitted only under `CheckedMutex`, in **both** directions, because a
        // mutex orders mutation instead of removing it (`CountedReadShares`)
        // or partitioning it (`DisjointWriteLeases`): a holder of the guard may
        // read *and* write the very same bytes another task will later reach.
        // What makes that sound is not this classification but the checked
        // model's own rule — a footprint a live generation protects has no
        // unguarded access at all (`canAccessUnguarded` requires
        // `protectorOf f` to be `none`) — which is the rule the Lean theorem
        // `aSharedPayloadIsReachableOnlyThroughTheProtocol` lifts to every
        // point of every interleaving.
        TransferKind::ProtectedResource => TransferClass::MovedOnly {
            shareable_under: Some(ShareProtocol::CheckedMutex),
            mutably_shareable_under: Some(ShareProtocol::CheckedMutex),
        },

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
    /// The child receives write authority over a declared byte range under
    /// the named protocol (card `WRITE-LEND-TASKS`); the parent keeps the
    /// binding, holds no whole-payload authority while the lease is out, and
    /// gets the authority back at join. The range itself is not the
    /// judgment's business — which bytes, and whether they are disjoint from
    /// every outstanding lease, is the ledger's decision by the wave-29 rule.
    LendWriteRange(ShareProtocol),
    /// The child may reach the payload under the named protocol, and only
    /// under it (card `MUTEX-SHARE-CROSS`). Nothing is duplicated, no custody
    /// crosses, and no authority is parked with the child: what crosses is the
    /// right to *acquire* the declared generation and reach the payload inside
    /// the section that acquisition opens. The parent keeps its binding and is
    /// held to the same rule for the whole time — which is why this is the one
    /// admitted transfer under which both parties may write the same bytes.
    ShareUnderMutex(ShareProtocol),
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
/// 17 x 6 = 102 pairs (13 x 4 originally; 14 x 5 after card
/// `WRITE-LEND-TASKS`; 16 x 6 after card `MUTEX-SHARE-CROSS`; 17 x 6 after
/// card `W48-QR-SCALAR-TYPES-R4`).
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
            // Moving, lending, leasing or mutex-sharing a duplicated kind is
            // not a custody question: there is no custody. The mode simply
            // does not apply.
            TransferMode::Move
            | TransferMode::Lend
            | TransferMode::Write
            | TransferMode::LendWrite
            | TransferMode::ShareUnderMutex => Err(TransferRefusal::ModeNotAdmittedForKind),
        },
        TransferClass::MovedOnly {
            shareable_under,
            mutably_shareable_under,
        } => match mode {
            TransferMode::Move => Ok(AdmittedTransfer::MoveCustody),
            // Card `MUTEX-SHARE-CROSS`: a sharing mode is admitted exactly
            // when the kind's own sharing discipline **is** the protocol that
            // mode requests. A kind that shares under a different protocol
            // refuses with that protocol named, which is what keeps the
            // disciplines from being interchangeable: an `own<Buffer>` may not
            // cross under a mutex (it shares by removing mutation), and a
            // `protected N` resource may not cross as a counted read share (it
            // shares by ordering mutation). Before this card the arm took
            // whatever protocol the kind named.
            TransferMode::Lend => match shareable_under {
                Some(ShareProtocol::CountedReadShares) => Ok(AdmittedTransfer::LendShare(
                    ShareProtocol::CountedReadShares,
                )),
                Some(other) => Err(TransferRefusal::ProtocolUnavailable(other)),
                None => Err(TransferRefusal::ProtocolUnavailable(
                    ShareProtocol::CheckedMutex,
                )),
            },
            // A mutex-mediated crossing is admitted exactly when the kind's
            // read-sharing discipline is the checked mutex. The admission is
            // the *classification* half only: whether the named generation
            // exists, protects this very payload, and verifies is decided by
            // the unedited mutex checker at the crossing (`MUTEX_SHARE_CROSSING`).
            TransferMode::ShareUnderMutex => match shareable_under {
                Some(ShareProtocol::CheckedMutex) => Ok(AdmittedTransfer::ShareUnderMutex(
                    ShareProtocol::CheckedMutex,
                )),
                Some(other) => Err(TransferRefusal::ProtocolUnavailable(other)),
                None => Err(TransferRefusal::ProtocolUnavailable(
                    ShareProtocol::CheckedMutex,
                )),
            },
            // A range write lend is admitted exactly under the byte-partition
            // protocol (card `WRITE-LEND-TASKS`); a kind whose mutable sharing
            // is some other discipline refuses with that discipline named, and
            // a kind with none refuses for the mode, because no protocol would
            // ever admit it.
            TransferMode::LendWrite => match mutably_shareable_under {
                Some(ShareProtocol::DisjointWriteLeases) => Ok(AdmittedTransfer::LendWriteRange(
                    ShareProtocol::DisjointWriteLeases,
                )),
                Some(other) => Err(TransferRefusal::ProtocolUnavailable(other)),
                None => Err(TransferRefusal::ModeNotAdmittedForKind),
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
        // Range write authority arrives as a lease, whatever it was leased
        // from — the second place where the parent's kind and the worker's
        // declared parameter kind differ on purpose.
        AdmittedTransfer::LendWriteRange(_) => TransferKind::WriteLease,
        // A mutex-mediated crossing arrives as a mutex share — the third place
        // where the parent's kind and what the child receives differ on
        // purpose. The parent keeps `protected N`; the child receives the
        // right to reach it under the declared generation, and nothing else.
        AdmittedTransfer::ShareUnderMutex(_) => TransferKind::MutexShare,
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

// ---------------------------------------------------------------------------
// Card `W48-QR-TASK-RECORD-CAPTURES`: the aggregate kind.
//
// A record crosses a spawn boundary as its **leaves** — its scalar fields and
// its owned `Buffer<T>` fields, depth-first in declaration order — because the
// task route represents a record binding as exactly those leaves, one task
// slot each, and every custody question about the record is then the
// existing per-resource question about an owned leaf. So the aggregate adds
// no rule of its own over a leaf: it asks each leaf the flat judgment above.
//
// What the aggregate does decide is which mode each leaf is asked in, and
// which whole-record requests exist at all:
//
// * `copy` of a record is `copy` of every leaf. It is admitted exactly when
//   every leaf is duplicated — a record without owned fields, on
//   [`TransferKind::FixedScalar`]'s footing — and otherwise the first leaf
//   that is not is named, with its own refusal (the missing contract).
// * `move` of a record moves every leaf that moves and duplicates every leaf
//   that duplicates ([`leaf_mode`]): custody of the owned leaves transfers and
//   the scalars travel with them. It is admitted exactly when every leaf
//   crosses in its induced mode **and** some leaf carries custody. A record
//   whose every leaf is duplicated is a copy record, and `move` of it is
//   refused for its mode exactly as `move` of an `i32` is.
// * `write` is refused **by name**, before any leaf is asked — the flat
//   judgment's `WriteCaptureUnimplemented`, unchanged.
// * A **sharing** mode — `read … share`, `lend … LOW..HIGH`, `share … under`
//   — names a protocol, and a record shares none whole: a read share or a
//   write lease reaches **one** owned leaf through a field capture
//   (`read r.buf share N`, `lend r.buf LOW..HIGH`), which is the flat
//   judgment of that leaf's kind.
//
// This is the transferability half. Whether the owned leaves are live, lent
// or leased on the path that reaches the creation is the ledger's decision,
// leaf by leaf, exactly as for separate `own<Buffer>` captures.
// ---------------------------------------------------------------------------

/// The mode an aggregate capture asks of one leaf (card
/// `W48-QR-TASK-RECORD-CAPTURES`): a record `move` duplicates a leaf that
/// duplicates and moves every other; every other mode is asked of each leaf as
/// it is.
pub const fn leaf_mode(leaf: TransferKind, mode: TransferMode) -> TransferMode {
    match (transferable(leaf), mode) {
        (TransferClass::Duplicated, TransferMode::Move) => TransferMode::Copy,
        _ => mode,
    }
}

/// The class of an aggregate, decided from its leaves: a leaf that never
/// crosses decides it; otherwise it is moved when some leaf is, and
/// duplicated when every leaf is. A moved aggregate is shareable under no
/// protocol — sharing is per leaf, through a field capture.
pub fn aggregate_class(leaves: &[TransferKind]) -> TransferClass {
    let mut moved = false;
    for leaf in leaves {
        match transferable(*leaf) {
            TransferClass::Never(reason) => return TransferClass::Never(reason),
            TransferClass::MovedOnly { .. } => moved = true,
            TransferClass::Duplicated => {}
        }
    }
    if moved {
        TransferClass::MovedOnly {
            shareable_under: None,
            mutably_shareable_under: None,
        }
    } else {
        TransferClass::Duplicated
    }
}

/// Why an aggregate capture is refused (card `W48-QR-TASK-RECORD-CAPTURES`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AggregateRefusal {
    /// A `write` capture: refused by name before any leaf is asked.
    WriteCaptureUnimplemented,
    /// The leaf at `index` (in leaf order) does not cross in the mode the
    /// capture asks of it; `refusal` is that leaf's own flat refusal, which
    /// names the contract it lacks.
    Leaf {
        index: usize,
        refusal: TransferRefusal,
    },
    /// Every leaf is duplicated, so the record crosses by `copy` and a `move`
    /// has no custody to carry — `move` of an `i32`, one level up.
    ModeNotAdmittedForRecord,
    /// The mode names a sharing protocol, and a record shares none whole: an
    /// owned leaf is shared by its own field capture.
    NoWholeRecordProtocol,
    /// The capture is admitted, but it delivers a record of another type than
    /// the one the worker declares it receives.
    ReceivedKindMismatch,
}

/// **The aggregate judgment.** Admission of a record whose leaves are
/// `leaves` in `mode`: the conjunction of each leaf's own admission in the
/// mode the record asks of it ([`leaf_mode`]), with the two whole-record
/// conditions above — `move` needs a leaf that carries custody, and a sharing
/// mode has no whole-record protocol.
pub fn admits_aggregate(
    leaves: &[TransferKind],
    mode: TransferMode,
) -> Result<AdmittedTransfer, AggregateRefusal> {
    match mode {
        TransferMode::Write => return Err(AggregateRefusal::WriteCaptureUnimplemented),
        TransferMode::Lend | TransferMode::LendWrite | TransferMode::ShareUnderMutex => {
            return Err(AggregateRefusal::NoWholeRecordProtocol);
        }
        TransferMode::Copy | TransferMode::Move => {}
    }
    let mut moved = false;
    for (index, leaf) in leaves.iter().enumerate() {
        match admits(*leaf, leaf_mode(*leaf, mode)) {
            Ok(AdmittedTransfer::MoveCustody) => moved = true,
            Ok(_) => {}
            Err(refusal) => return Err(AggregateRefusal::Leaf { index, refusal }),
        }
    }
    match (mode, moved) {
        (TransferMode::Move, true) => Ok(AdmittedTransfer::MoveCustody),
        (TransferMode::Move, false) => Err(AggregateRefusal::ModeNotAdmittedForRecord),
        _ => Ok(AdmittedTransfer::Duplicate),
    }
}

/// The whole boundary decision for a record: the parent holds a record of type
/// `place` with leaves `leaves`, the capture asks for `mode`, and the worker
/// declares it receives a record of type `parameter`. An admitted aggregate
/// transfer — a duplicate or a custody move — delivers the record type it
/// left as, so the two types must be the same type. Record types are nominal:
/// two record types with the same leaves are still two types.
pub fn admits_aggregate_capture<T: PartialEq>(
    place: &T,
    leaves: &[TransferKind],
    mode: TransferMode,
    parameter: &T,
) -> Result<AdmittedTransfer, AggregateRefusal> {
    let admitted = admits_aggregate(leaves, mode)?;
    if place == parameter {
        Ok(admitted)
    } else {
        Err(AggregateRefusal::ReceivedKindMismatch)
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

/// The sharing gap **that remains open**, and is narrower than it was.
///
/// Card `CT46-SENDSYNC` recorded this as the whole gap: the task route's
/// [`TransferKind::OwnedBuffer`] is sendable-but-not-freely-shareable, a mutex
/// is the discipline that would let two tasks mutate it, and no capture mode
/// could request one. Card `MUTEX-SHARE-CROSS` supplies the capture mode — but
/// **not for this kind**. The task route has no mutex at all, so an
/// `own<Buffer>` still shares only by removing mutation, and
/// `admits(OwnedBuffer, ShareUnderMutex)` refuses with `CountedReadShares`
/// named. `requestable` therefore stays `false`, deliberately, and the residual
/// is `MUTEX-SHARE-CROSS-TASK-BUFFER`.
pub const MUTEX_SHARING_GAP: SharingGap = SharingGap {
    kind: TransferKind::OwnedBuffer,
    protocol: ShareProtocol::CheckedMutex,
    requestable: false,
};

/// The half card `MUTEX-SHARE-CROSS` closes: CT46's composition, as data.
///
/// [`TransferKind::ProtectedResource`] is the checked-mutex route's payload —
/// sendable (custody moves over a derivable synchronization edge) and not
/// freely shareable — and `share PLACE under MUTEX` is a capture mode that
/// requests [`ShareProtocol::CheckedMutex`] for it. So `requestable` is `true`,
/// and a test pins that the judgment agrees: the pair
/// `(ProtectedResource, ShareUnderMutex)` admits, and it is the **only** pair
/// anywhere in the matrix that yields the mutex protocol.
///
/// Requestable is not the same as granted. The judgment answers the
/// classification question only — *may this kind cross this way at all* — and
/// deliberately cannot see whether the generation the capture names exists,
/// protects this very payload, or verifies. Those are decided at the crossing
/// against the **unedited** `CheckedMutexInvariantV1` checker, and a capture
/// whose protocol is absent, mismatched or unverified refuses there.
pub const MUTEX_SHARE_CROSSING: SharingGap = SharingGap {
    kind: TransferKind::ProtectedResource,
    protocol: ShareProtocol::CheckedMutex,
    requestable: true,
};

#[cfg(test)]
#[path = "../tests/unit/transfer.rs"]
mod tests;
