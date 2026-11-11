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

//! Bounded extern-call semantics: the `source.extern-call.v1` model.
//!
//! Card `source.extern-call.v1` (`docs/design/EXTERN_CALL_SEMANTICS_V1.md`).
//! This module owns three things and deliberately nothing else:
//!
//! 1. **Admission** ([`bridge_contract`]). A complete resource-contract v1
//!    declaration goes in; either one [`ExternCallee`] inside the closed v1
//!    bridging table comes out, or a named [`BridgeRefusal`] does. Every C04
//!    mode outside the admitted table is an explicit refusal constructor, not
//!    an unhandled case: `resource_released`, `resource_removed`,
//!    `resource_return_reallocated`, a nonnull owned return, a nullable
//!    consumed parameter and a `resource_owned` parameter that owns nothing
//!    each have their own refusal, so widening the profile means deleting a
//!    refusal on purpose.
//!
//!    Card `EXTERN-F1` adds exactly two modes to that table. `resource_out`
//!    (the callee writes fresh custody through a `ptr<mut, ptr<mut, u8>>`
//!    slot) reuses the `succeeds` adapter discipline unchanged: the out write
//!    materializes a checked owned binding only under the success control, and
//!    the nonnull form stays refused so no acquisition arm is unchecked.
//!    `resource_stored` (custody of the passed `own<Buffer>` becomes indexed
//!    beneath a declared owner) removes the identity from the caller's ledger
//!    permanently with no owned return; the residual accounting beneath that
//!    owner is a **named declared premise**
//!    ([`ExternAssumption::StoredCustodyUnderDeclaredOwner`]), never a checked
//!    fact. Contract spec §C04.2 requires a stored parameter's owner to be a
//!    *distinct `Owned` parameter*, so admitting `resource_stored` forces
//!    `resource_owned` to be admitted in exactly that position and nowhere
//!    else.
//!    Card `EXTERN-ERR-REL` adds the adapter's error-arm dispositions, which is
//!    what makes `resource_stored` reachable from a single source module: a
//!    store needs two simultaneously live identities, and `EXTERN-F1`'s adapter
//!    could abandon nothing. The arm is not a second custody rule. The
//!    sequential lowering [`crate::cleanup_lowering`] already releases every
//!    identity live before a failing acquisition, in reverse acquisition order,
//!    on that acquisition's cleanup edge; the arm names the declared consuming
//!    calls that edge performs, and admission requires it to be exactly that
//!    edge. The v1 abandonment refusal is narrowed to the undispositioned case
//!    and keeps its code, its message subject and its named identity.
//!
//!    Card `EXTERN-MODES` adds two more modes and keeps one refusal on measured
//!    grounds. `resource_released(p, nonnull)` is the mode the error arm was
//!    describing all along: contract spec §C04.2 separates *discharging* an
//!    obligation from *transferring* it, and the arm previously had only
//!    transfer available. The caller-side ledger delta of the two is identical,
//!    so nothing about leak accounting moves; what changes is that a release is
//!    published as a release and premised by name
//!    ([`ExternAssumption::DeclaredRelease`]), because §C04.2 warns that a
//!    false release declaration invalidates a safety conclusion while a false
//!    consumption declaration only relocates an obligation.
//!    `resource_return_reallocated(p, nullable)` is the one departing mode
//!    whose *failure* arm the declaration states (§C04.3: a null failure
//!    preserves the incoming custody, a success replaces it exactly once), so
//!    its subject stays live on the error arm and must be dispositioned there.
//!    [`ExternDiagnostic::AdapterDepartingArgument`] refuses a consumption
//!    beside an acquisition precisely because ordinary declarations are silent
//!    on that arm; a reallocation is not silent, which is why it is admitted
//!    without weakening that refusal. `resource_removed` stays refused: see
//!    [`BridgeRefusal::RemovedParameter`] for the measured ground.
//! 2. **Derivation** ([`derived_custody`], [`derived_effects`]). The custody
//!    delta and effect row a call site imposes are a *function* of the admitted
//!    callee, so a certificate consumer can repeat the derivation instead of
//!    believing it.
//! 3. **Call-site checking** ([`check_extern_program`]). Argument arity and
//!    form against the declared modes, and the mandatory nullable-owned-return
//!    adapter. Liveness, leaks, resource flow and the caller's declared effect
//!    row stay with the established sequential passes
//!    (`qcore_frontend::whole_program`); this checker never re-decides them
//!    under a second diagnostic code.
//!
//!    Card `UNSAFE-LINKAGE` adds a second admission table beside
//!    [`bridge_contract`], for the case where there is **no contract at all**:
//!    [`unchecked_callee`] admits an `unsafe extern fn` declaration so that an
//!    external library can be linked without one. Its admitted surface is
//!    scalars by value in and a scalar or `unit` out — every pointer, in every
//!    spelling and position, is a named [`UncheckedRefusal`] — so an unchecked
//!    call is handed no custody and cannot change the caller's ledger whatever
//!    the callee does. The callee is marked [`ExternTrust::UncheckedLinkage`]
//!    and carried on every certificate as
//!    [`ExternAssumption::UncheckedLinkage`], by name and declared ABI. See
//!    `docs/design/UNSAFE_LINKAGE_V1.md`.
//!
//!    Card `UNSAFE-LINKAGE-ARTIFACT` sharpens that premise where, and only
//!    where, a `source.native-library.v1` manifest names the waived symbol
//!    ([`crate::native_library`]): [`manifest_unchecked_callee`] attaches the
//!    manifest's artifact identity and ABI premises, and the callee then
//!    contributes [`ExternAssumption::ManifestUncheckedLinkage`] — the same
//!    waiver, now naming *which build* it was waived from. Nothing else moves:
//!    the admitted table is the one above, decided before any manifest is
//!    consulted, and a waiver no manifest names keeps the bare premise
//!    unchanged.
//!
//! What a result through this module means. A contract admitted here is a
//! **declared assumption** (`ContractTrust::Declared`, contract spec §C06): it
//! is never a proof about the C implementation, its layout, its aliasing or
//! its thread-safety. Two premises are recorded on every certificate:
//! [`ExternAssumption::DeclaredContract`] for each callee, and
//! [`ExternAssumption::NoExtentTransport`], because C04 pointers are extentless
//! `u8` projections — the declaration cannot say how many bytes the callee
//! touches, and this profile does not check it. A callee admitted through
//! unchecked linkage contributes [`ExternAssumption::UncheckedLinkage`] in place
//! of the first of those, because it has no contract to declare; that premise is
//! weaker still — it claims nothing at all about the callee, and printing it is
//! the point.
//!
//!    Card `PIPE-SCALAR-RESULT` adds one thing to the call-site checker and
//!    nothing to the admission tables: a **non-acquiring call's scalar result
//!    may be bound**, and a bound scalar may be passed at a declared `Data`
//!    parameter. Both live in [`ValueId`], a namespace disjoint from
//!    [`ResourceId`]: no operation in this module moves a name between the two,
//!    [`derived_custody`] never sees a scalar, and the ledger this checker
//!    maintains is untouched by either form. The widening is exactly
//!    [`binds_scalar`] — nothing owned, nothing nullable-owned, no pointer, no
//!    `unit` — so the mandatory nullable-owned-return adapter and every
//!    acquisition rule are byte-unchanged, and there is no second way to spell
//!    an acquisition.
//!
//!    Card `STD-IO-COMPOSE` adds two more things to the same namespace and
//!    still nothing to the admission tables. [`ExternEvent::Bind`] is a scalar
//!    **literal** binding, `let NAME = INTEGER;`, which calls nothing and so
//!    can carry no custody at all; and a scalar binding may carry a *declared
//!    outcome*, [`ScalarBinding::outcome`], on exactly the
//!    [`ExternAdapter::succeeds`] precedent — a literal control written at the
//!    site, believed by the reference execution and **compared against the real
//!    callee at run time** by the emitted C, which exits 70 rather than
//!    silently observing a different number. Neither touches the ledger, and
//!    an outcome is optional: a binding without one is exactly the binding card
//!    `PIPE-SCALAR-RESULT` shipped.
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use crate::ResourceId;
use crate::native_library;
use crate::scalar_types::{Cmp, Op, RecordedTarget, ScalarType};
use crate::scalar_values;
use crate::typed_errors::{Effect, EffectRow};

/// Identity of a bound scalar value (card `PIPE-SCALAR-RESULT`).
///
/// A **separate namespace** from [`ResourceId`], and deliberately so. A scalar
/// binding carries data — contract spec §C04.1's "a scalar may carry a
/// descriptor number as data, but cannot carry descriptor custody" — so nothing
/// in this module converts one into the other: [`ExternArgument::Value`] is
/// admitted only at [`CallParameter::Data`], [`derived_custody`] filters `Data`
/// parameters out, and the live ledger of [`check_extern_program`] is never
/// read or written by a scalar binding. The two id spaces are both `u64` and may
/// collide numerically; they are never compared.
pub type ValueId = u64;

/// The C ABI projection of contract spec §C04.1, as far as **this** pass
/// projects it. `Unit` is a result type only, exactly as in the loader.
///
/// This table is independent of `qcore_contract::contract_loader`'s and is
/// deliberately narrower than §C04.1: card NEIGHBOURS added `f32`/`f64` to the
/// loader's vocabulary and card CONSTRUCTOR-RESIDUALS-ABI-TYPEDEF added
/// `fnptr`, and neither is parsed here. A spelling absent from this table is
/// refused by `Q033`, which is the refusing direction — the *contract* is
/// waived at that surface, the ABI is not — so widening §C04.1 never widens
/// what unchecked linkage admits. (The doc comment claimed the table was
/// "complete" from NEIGHBOURS onward, when it already was not; corrected here
/// rather than left to rot. No behaviour moved with this comment.)
///
/// Card `W48-QR-FLOATING-POINT` adds `f32`/`f64` here, deliberately: a float
/// is now a §G02t value with its own checked rules, so a declaration may state
/// one and a bound float result is a value of the namespace. `fnptr` and the
/// quantity types stay outside this table.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum AbiType {
    Bool,
    I8,
    I16,
    I32,
    I64,
    Isize,
    U8,
    U16,
    U32,
    U64,
    Usize,
    /// IEEE binary32, C `float` (card `W48-QR-FLOATING-POINT`).
    F32,
    /// IEEE binary64, C `double` (card `W48-QR-FLOATING-POINT`).
    F64,
    PtrMutU8,
    PtrConstU8,
    PtrMutPtrMutU8,
    Unit,
}

impl AbiType {
    /// Parse the loader's exact spelling. Anything else is not an ABI type.
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        Some(match text.split_whitespace().collect::<String>().as_str() {
            "bool" => Self::Bool,
            "i8" => Self::I8,
            "i16" => Self::I16,
            "i32" => Self::I32,
            "i64" => Self::I64,
            "isize" => Self::Isize,
            "u8" => Self::U8,
            "u16" => Self::U16,
            "u32" => Self::U32,
            "u64" => Self::U64,
            "usize" => Self::Usize,
            "f32" => Self::F32,
            "f64" => Self::F64,
            "ptr<mut,u8>" => Self::PtrMutU8,
            "ptr<const,u8>" => Self::PtrConstU8,
            "ptr<mut,ptr<mut,u8>>" => Self::PtrMutPtrMutU8,
            "unit" => Self::Unit,
            _ => return None,
        })
    }

    #[must_use]
    pub const fn is_pointer(self) -> bool {
        matches!(
            self,
            Self::PtrMutU8 | Self::PtrConstU8 | Self::PtrMutPtrMutU8
        )
    }

    /// A `mut` pointer parameter is the only source of the derived `Mut`
    /// effect. `ptr<const, u8>` and every scalar are not.
    #[must_use]
    pub const fn is_mutable_pointer(self) -> bool {
        matches!(self, Self::PtrMutU8 | Self::PtrMutPtrMutU8)
    }

    /// The §G02t type a value of this ABI spelling has (card
    /// `W48-QR-SCALAR-TYPES-R3`): the eleven scalar spellings are exactly the
    /// §G02t table, and a pointer or `unit` is no value at all.
    #[must_use]
    pub const fn scalar(self) -> Option<ScalarType> {
        Some(match self {
            Self::Bool => ScalarType::Bool,
            Self::I8 => ScalarType::I8,
            Self::I16 => ScalarType::I16,
            Self::I32 => ScalarType::I32,
            Self::I64 => ScalarType::I64,
            Self::Isize => ScalarType::Isize,
            Self::U8 => ScalarType::U8,
            Self::U16 => ScalarType::U16,
            Self::U32 => ScalarType::U32,
            Self::U64 => ScalarType::U64,
            Self::Usize => ScalarType::Usize,
            Self::F32 => ScalarType::F32,
            Self::F64 => ScalarType::F64,
            Self::PtrMutU8 | Self::PtrConstU8 | Self::PtrMutPtrMutU8 | Self::Unit => return None,
        })
    }

    /// The stdint spelling the backend prints. Linkage and layout equality with
    /// the real C declaration remain the builder's premise.
    #[must_use]
    pub const fn c_spelling(self) -> &'static str {
        match self {
            Self::Bool => "_Bool",
            Self::I8 => "int8_t",
            Self::I16 => "int16_t",
            Self::I32 => "int32_t",
            Self::I64 => "int64_t",
            Self::Isize => "intptr_t",
            Self::U8 => "uint8_t",
            Self::U16 => "uint16_t",
            Self::U32 => "uint32_t",
            Self::U64 => "uint64_t",
            Self::Usize => "uintptr_t",
            Self::F32 => "float",
            Self::F64 => "double",
            Self::PtrMutU8 => "uint8_t *",
            Self::PtrConstU8 => "const uint8_t *",
            Self::PtrMutPtrMutU8 => "uint8_t **",
            Self::Unit => "void",
        }
    }
}

/// Every resource-contract v1 parameter mode (§C04.2), so that admission is a
/// total match and each exclusion is deliberate.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ContractParameterMode {
    Borrowed,
    Owned { nullable: bool },
    Consumed { nullable: bool },
    Released { nullable: bool },
    Out { nullable: bool },
    Stored { owner: usize, nullable: bool },
    Removed { owner: usize, nullable: bool },
}

/// Every resource-contract v1 result mode (§C04.3).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ContractReturnMode {
    None,
    Owned { nullable: bool },
    Reallocated { parameter: usize, nullable: bool },
}

/// A complete loaded declaration, in this module's vocabulary. Construction
/// from the loader's `ResourceFunctionContract` lives in `qcore-contract`;
/// this crate stays dependency-free.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DeclaredContract {
    pub parameters: Vec<(AbiType, ContractParameterMode)>,
    pub result: AbiType,
    pub result_mode: ContractReturnMode,
}

/// The admitted v1 bridging of one parameter (design §3.2).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CallParameter {
    /// A scalar passed by value. Data only: it may carry a descriptor number,
    /// never descriptor custody (§C04.1).
    Data(AbiType),
    /// An admitted pointer bound to a borrowed model Buffer.
    Borrowed(AbiType),
    /// An admitted `ptr<mut, u8>` bound to a consumed `own<Buffer>`.
    Consumed,
    /// `resource_stored(p, owner, nonnull)` on `ptr<mut, u8>` (card
    /// `EXTERN-F1`). The passed `own<Buffer>` leaves the caller's ledger
    /// permanently and becomes indexed beneath the parameter at `owner`. What
    /// happens to it afterwards is the declared owner's business and is not
    /// checked here.
    Stored { owner: usize },
    /// `resource_owned(p, nonnull)` on `ptr<mut, u8>`, designated as the owner
    /// of at least one admitted [`CallParameter::Stored`] parameter of the same
    /// contract (contract spec §C04.2). The caller supplies live custody and
    /// keeps the identity, so this is a loan with a liveness precondition.
    Owner,
    /// `resource_out(p, nullable)` on `ptr<mut, ptr<mut, u8>>` (card
    /// `EXTERN-F1`). The callee writes fresh custody through the slot; the
    /// caller's binding materializes only on the adapter's success arm.
    OutSlot,
    /// `resource_released(p, nonnull)` on `ptr<mut, u8>` (card `EXTERN-MODES`).
    /// The declared contract **discharges** the obligation rather than moving
    /// it somewhere this profile stops looking. The caller-side ledger delta is
    /// the same as a consumption; what differs is what the declaration claims,
    /// and contract spec §C04.2 is explicit that "a false declaration here can
    /// invalidate a safety conclusion" — so the claim is recorded by name as
    /// [`ExternAssumption::DeclaredRelease`] rather than folded into the
    /// consumption premise.
    Released,
    /// The subject of a `resource_return_reallocated(p, nullable)` declaration
    /// (card `EXTERN-MODES`): a `resource_owned(p, nonnull)` parameter on
    /// `ptr<mut, u8>` that the *result mode* designates. Contract spec §C04.3
    /// fixes both arms — a null failure preserves the incoming custody, and a
    /// successful replacement consumes the old resource exactly once and
    /// establishes the result identity — so this is the one departing argument
    /// whose status on the failure arm the declaration actually states.
    ReallocSubject,
}

/// The admitted v1 bridging of the result (design §3.2).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CallResult {
    /// `resource_return_none()`: `unit` or a scalar, data only.
    Data(AbiType),
    /// `resource_return_owned(nullable)`: fresh custody behind the mandatory
    /// null adapter of design §3.3.
    OwnedNullable,
    /// `resource_return_reallocated(p, nullable)` (card `EXTERN-MODES`): the
    /// identity at parameter `p` is replaced by the result identity on the
    /// success arm and preserved untouched on the null arm. The same mandatory
    /// adapter of design §3.3 branches it, so neither arm is unchecked.
    ReallocatedNullable { parameter: usize },
}

/// How much is known about a callee. Card `UNSAFE-LINKAGE` adds the second
/// arm, and the type exists so that the two are a *total match* everywhere a
/// callee is consulted: there is no representation of an uncontracted callee
/// that is not marked as one.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ExternTrust {
    /// A resource-contract v1 declaration was loaded, admitted by
    /// [`bridge_contract`] and is carried as
    /// [`ExternAssumption::DeclaredContract`].
    Contracted,
    /// No contract exists. The caller declared the ABI signature and nothing
    /// else, through the explicit `unsafe extern fn` surface, and the callee is
    /// carried as [`ExternAssumption::UncheckedLinkage`] on every certificate
    /// that names it. Constructed only by [`unchecked_callee`], whose admitted
    /// surface carries no custody at all (see [`UncheckedRefusal`]).
    UncheckedLinkage,
}

/// One admitted callee. Only [`bridge_contract`] and [`unchecked_callee`]
/// construct this, so a value of this type always names a shape inside a closed
/// v1 table — which one is recorded in [`ExternCallee::trust`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExternCallee {
    pub name: String,
    /// Whether a contract was admitted for this callee, or nothing was.
    pub trust: ExternTrust,
    pub parameters: Vec<CallParameter>,
    pub result: CallResult,
    pub result_abi: AbiType,
    /// Index of the single admitted [`CallParameter::OutSlot`], if any. A
    /// callee has at most one acquisition source: an owned nullable return or
    /// one out slot, never both and never two slots.
    pub out_parameter: Option<usize>,
    /// Index of the [`CallParameter::ReallocSubject`], if the result mode is
    /// `resource_return_reallocated`. A reallocation is a third acquisition
    /// source and, like the other two, is exclusive with them.
    pub realloc_parameter: Option<usize>,
    /// Card `UNSAFE-LINKAGE-ARTIFACT`: the library manifest this callee is
    /// resident in, when one names it.
    ///
    /// Only [`manifest_unchecked_callee`] ever sets this, so it is `None` for
    /// every contracted callee and for every waiver no manifest names — which
    /// is what keeps a bare waiver's premise byte-identical to the one card
    /// `UNSAFE-LINKAGE` published. Its presence sharpens
    /// [`ExternAssumption::UncheckedLinkage`] into
    /// [`ExternAssumption::ManifestUncheckedLinkage`], which names *which build*
    /// was waived; its absence is itself part of the trust statement — **no
    /// build was named**.
    pub residence: Option<native_library::ManifestResidence>,
}

impl ExternCallee {
    /// Whether a call to this callee establishes fresh custody, through any
    /// admitted acquisition source. Exactly the callees for which the
    /// `succeeds BOOL else raise ERROR` adapter is mandatory.
    #[must_use]
    pub const fn acquires(&self) -> bool {
        matches!(
            self.result,
            CallResult::OwnedNullable | CallResult::ReallocatedNullable { .. }
        ) || self.out_parameter.is_some()
    }
}

/// Whether a call to this callee may bind its result to a scalar local (card
/// `PIPE-SCALAR-RESULT`).
///
/// The whole widening is this predicate, and it is deliberately narrow. The
/// callee must establish no fresh custody through **any** of the three
/// acquisition sources — so an acquisition keeps its one spelling, the mandatory
/// `succeeds BOOL else raise ERROR` adapter — and its declared result must be a
/// custody-free scalar of the closed §C04.1 ABI vocabulary. `unit` is excluded
/// because there is no value to bind, and every pointer is excluded because a
/// pointer result outside `resource_return_owned`/`resource_return_reallocated`
/// is a value the declaration says nothing about: binding it would put an
/// unmodelled address into a body under a name, which is the laundering this
/// card exists to make impossible.
#[must_use]
pub fn binds_scalar(callee: &ExternCallee) -> bool {
    !callee.acquires()
        && matches!(
            callee.result,
            CallResult::Data(abi) if abi != AbiType::Unit && !abi.is_pointer()
        )
}

/// Why a complete, loadable C04 declaration is still not callable in v1. Each
/// constructor names one excluded shape; none is a catch-all.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BridgeRefusal {
    /// `resource_owned` on a parameter that is not the designated owner of any
    /// admitted `resource_stored` parameter of the same contract. Outside that
    /// one position the mode is still not callable.
    OwnedParameter { parameter: usize },
    /// `resource_owned(p, nullable)` in owner position: storing beneath an
    /// owner that may be absent has no modeled meaning.
    NullableOwnerParameter { parameter: usize },
    /// `resource_owned` in owner position on other than `ptr<mut, u8>`.
    UnsupportedOwnerPointer { parameter: usize },
    /// `resource_released(p, nullable)` (card `EXTERN-MODES`): v1 releases
    /// nonnull arguments only, exactly as it consumes nonnull arguments only.
    /// A nullable release discharges nothing when the modeled value is null
    /// (contract spec §C04.2), so admitting it would make the discharge
    /// conditional on a value this profile does not model.
    NullableReleasedParameter { parameter: usize },
    /// `resource_released` on other than `ptr<mut, u8>`.
    UnsupportedReleasedPointer { parameter: usize },
    /// `resource_out(p, nonnull)`: a trusted non-nullness assertion on the
    /// written slot, with no checked branch. Excluded for the same reason as
    /// `resource_return_owned(nonnull)`: every acquisition arm stays checked.
    NonnullOutParameter { parameter: usize },
    /// `resource_out` on other than `ptr<mut, ptr<mut, u8>>`.
    UnsupportedOutPointer { parameter: usize },
    /// `resource_stored(p, owner, nullable)`: v1 stores nonnull arguments only,
    /// exactly as it consumes nonnull arguments only.
    NullableStoredParameter { parameter: usize },
    /// `resource_stored` on other than `ptr<mut, u8>`.
    UnsupportedStoredPointer { parameter: usize },
    /// `resource_stored(p, owner, _)` whose designated owner is not a
    /// `resource_owned` parameter of the same contract (contract spec §C04.2).
    StoredOwnerNotOwned { parameter: usize, owner: usize },
    /// `resource_stored(p, owner, _)` whose designated owner is the stored
    /// parameter itself.
    StoredOwnerIsItself { parameter: usize },
    /// More than one acquisition source in one declaration: two out slots, or
    /// an out slot beside an owned return. A call establishes at most one
    /// fresh identity, so the adapter binds exactly one.
    MultipleAcquisitions,
    /// A `resource_stored` parameter beside an acquisition source. The stored
    /// mode is admitted with no owned return: custody leaves the checked world
    /// and nothing comes back in the same call.
    StoredWithAcquisition { parameter: usize },
    /// `resource_removed`: owner-indexed custody flowing back *out* of a
    /// declared owner. Card `EXTERN-MODES` measured this mode and deliberately
    /// kept the refusal. A removal has to name a child this profile stopped
    /// tracking when it was stored: `EXTERN-F1` records the residual beneath an
    /// owner as the premise
    /// [`ExternAssumption::StoredCustodyUnderDeclaredOwner`], not as a modeled
    /// owner-to-child index, and a source body has no expression that reads one
    /// back. Admitting removal would therefore mint a fresh caller identity out
    /// of a premise, which is an unchecked acquisition. Follow-on
    /// `EXTERN-MODES-REMOVED-INDEX`.
    RemovedParameter { parameter: usize },
    /// `resource_consumed(p, nullable)`: v1 admits the nonnull form only, so
    /// every consumed argument is a live identity.
    NullableConsumedParameter { parameter: usize },
    /// A custody mode on a scalar. The loader already rejects this; repeating
    /// it here keeps admission total rather than assuming an upstream check.
    ScalarCustody { parameter: usize },
    /// `resource_borrowed` on `ptr<mut, ptr<mut, u8>>`: the pointer-to-pointer
    /// projection exists for `resource_out` only and carries no Buffer.
    UnsupportedBorrowedPointer { parameter: usize },
    /// `resource_consumed` on other than `ptr<mut, u8>`.
    UnsupportedConsumedPointer { parameter: usize },
    /// `resource_return_owned(nonnull)`: a trusted non-nullness assertion with
    /// no checked branch. Excluded to keep every acquisition arm checked.
    NonnullOwnedReturn,
    /// `resource_return_reallocated(p, nonnull)`: a trusted non-nullness
    /// assertion with no checked branch. Excluded for the same reason as
    /// `resource_return_owned(nonnull)`; here it would additionally erase the
    /// preserving failure arm contract spec §C04.3 defines.
    NonnullReallocatedReturn,
    /// `resource_return_reallocated(p, _)` whose `p` is not a parameter of this
    /// declaration.
    ReallocSubjectOutOfRange { parameter: usize },
    /// `resource_return_reallocated(p, _)` whose `p` is not declared
    /// `resource_owned`. Contract spec §C04.3 requires exactly that.
    ReallocSubjectNotOwned { parameter: usize },
    /// `resource_return_reallocated(p, _)` whose subject is `resource_owned(p,
    /// nullable)`: a null incoming subject supplies no live custody, so there
    /// is nothing for the success arm to replace exactly once.
    NullableReallocSubject { parameter: usize },
    /// A reallocation subject on other than `ptr<mut, u8>`.
    UnsupportedReallocSubjectPointer { parameter: usize },
    /// A reallocated result whose declared result type is not `ptr<mut, u8>`.
    UnsupportedReallocatedResultType { result: AbiType },
    /// An owned result whose declared result type is not `ptr<mut, u8>`.
    UnsupportedOwnedResultType { result: AbiType },
    /// A `unit` parameter, which the loader's vocabulary never produces.
    UnitParameter { parameter: usize },
}

impl fmt::Display for BridgeRefusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::OwnedParameter { parameter } => write!(
                f,
                "parameter {parameter} declares `resource_owned` but owns no `resource_stored` parameter of this contract; outside owner position the mode is not callable"
            ),
            Self::NullableOwnerParameter { parameter } => write!(
                f,
                "parameter {parameter} is a declared store owner but is `nullable`; an absent owner indexes nothing"
            ),
            Self::UnsupportedOwnerPointer { parameter } => write!(
                f,
                "parameter {parameter} is a declared store owner on other than `ptr<mut, u8>`"
            ),
            Self::NullableReleasedParameter { parameter } => write!(
                f,
                "parameter {parameter} declares `resource_released(_, nullable)`; v1 releases nonnull arguments only"
            ),
            Self::UnsupportedReleasedPointer { parameter } => write!(
                f,
                "parameter {parameter} releases other than `ptr<mut, u8>`"
            ),
            Self::NonnullOutParameter { parameter } => write!(
                f,
                "parameter {parameter} declares `resource_out(_, nonnull)`, which asserts the written slot is non-null with no checked branch"
            ),
            Self::UnsupportedOutPointer { parameter } => write!(
                f,
                "parameter {parameter} declares `resource_out` on other than `ptr<mut, ptr<mut, u8>>`"
            ),
            Self::NullableStoredParameter { parameter } => write!(
                f,
                "parameter {parameter} declares `resource_stored(_, _, nullable)`; v1 stores nonnull arguments only"
            ),
            Self::UnsupportedStoredPointer { parameter } => write!(
                f,
                "parameter {parameter} declares `resource_stored` on other than `ptr<mut, u8>`"
            ),
            Self::StoredOwnerNotOwned { parameter, owner } => write!(
                f,
                "parameter {parameter} is stored beneath parameter {owner}, which is not a `resource_owned` parameter of this contract"
            ),
            Self::StoredOwnerIsItself { parameter } => write!(
                f,
                "parameter {parameter} is declared stored beneath itself"
            ),
            Self::MultipleAcquisitions => f.write_str(
                "the declaration has more than one acquisition source; a call establishes at most one fresh identity",
            ),
            Self::StoredWithAcquisition { parameter } => write!(
                f,
                "parameter {parameter} is stored beneath a declared owner in a call that also establishes fresh custody; the stored mode is admitted with no owned return"
            ),
            Self::RemovedParameter { parameter } => write!(
                f,
                "parameter {parameter} declares `resource_removed`, which `source.extern-call.v1` does not call"
            ),
            Self::NullableConsumedParameter { parameter } => write!(
                f,
                "parameter {parameter} declares `resource_consumed(_, nullable)`; v1 consumes nonnull arguments only"
            ),
            Self::ScalarCustody { parameter } => write!(
                f,
                "parameter {parameter} carries a custody mode on a scalar; a scalar carries data, never custody"
            ),
            Self::UnsupportedBorrowedPointer { parameter } => write!(
                f,
                "parameter {parameter} borrows `ptr<mut, ptr<mut, u8>>`, which projects no model Buffer"
            ),
            Self::UnsupportedConsumedPointer { parameter } => write!(
                f,
                "parameter {parameter} consumes other than `ptr<mut, u8>`"
            ),
            Self::NonnullOwnedReturn => f.write_str(
                "`resource_return_owned(nonnull)` asserts non-nullness with no checked branch",
            ),
            Self::NonnullReallocatedReturn => f.write_str(
                "`resource_return_reallocated(_, nonnull)` asserts non-nullness with no checked branch, and erases the failure arm that preserves the incoming custody",
            ),
            Self::ReallocSubjectOutOfRange { parameter } => write!(
                f,
                "the reallocated result names parameter {parameter}, which this declaration does not have"
            ),
            Self::ReallocSubjectNotOwned { parameter } => write!(
                f,
                "the reallocated result names parameter {parameter}, which is not a `resource_owned` parameter of this contract"
            ),
            Self::NullableReallocSubject { parameter } => write!(
                f,
                "the reallocation subject at parameter {parameter} is `nullable`; an absent subject supplies no custody to replace"
            ),
            Self::UnsupportedReallocSubjectPointer { parameter } => write!(
                f,
                "the reallocation subject at parameter {parameter} is on other than `ptr<mut, u8>`"
            ),
            Self::UnsupportedReallocatedResultType { result } => write!(
                f,
                "a reallocated result requires `ptr<mut, u8>`, not `{}`",
                result.c_spelling()
            ),
            Self::UnsupportedOwnedResultType { result } => write!(
                f,
                "an owned result requires `ptr<mut, u8>`, not `{}`",
                result.c_spelling()
            ),
            Self::UnitParameter { parameter } => {
                write!(f, "parameter {parameter} is declared `unit`")
            }
        }
    }
}

impl std::error::Error for BridgeRefusal {}

/// Admit one complete declaration into the closed v1 bridging table, or refuse
/// it by name. Deterministic: the same contract always yields the same callee,
/// which is what lets a certificate consumer repeat the derivation.
pub fn bridge_contract(
    name: &str,
    contract: &DeclaredContract,
) -> Result<ExternCallee, BridgeRefusal> {
    // Owner position is a property of the *contract*, not of one parameter:
    // `resource_owned` is admitted exactly where some `resource_stored`
    // parameter designates it (contract spec §C04.2) and refused everywhere
    // else, so this pass runs before per-parameter admission.
    //
    // Card `EXTERN-MODES` adds the second such position: contract spec §C04.3
    // requires a `resource_return_reallocated(p, _)` subject to be `Owned`, so
    // the *result* mode also designates an owner. Reading it here keeps the
    // per-parameter pass a single total match over one precomputed set.
    let realloc_subject = match contract.result_mode {
        ContractReturnMode::Reallocated { parameter, .. } => {
            let Some((_, mode)) = contract.parameters.get(parameter) else {
                return Err(BridgeRefusal::ReallocSubjectOutOfRange { parameter });
            };
            if !matches!(mode, ContractParameterMode::Owned { .. }) {
                return Err(BridgeRefusal::ReallocSubjectNotOwned { parameter });
            }
            Some(parameter)
        }
        ContractReturnMode::None | ContractReturnMode::Owned { .. } => None,
    };
    let designated_owners = contract
        .parameters
        .iter()
        .filter_map(|(_, mode)| match mode {
            ContractParameterMode::Stored { owner, .. } => Some(*owner),
            _ => None,
        })
        .chain(realloc_subject)
        .collect::<BTreeSet<_>>();
    let mut parameters = Vec::with_capacity(contract.parameters.len());
    let mut out_parameter = None;
    let mut stored_parameter = None;
    for (index, (abi, mode)) in contract.parameters.iter().enumerate() {
        if *abi == AbiType::Unit {
            return Err(BridgeRefusal::UnitParameter { parameter: index });
        }
        let bridged = match mode {
            ContractParameterMode::Borrowed => {
                if abi.is_pointer() {
                    if *abi == AbiType::PtrMutPtrMutU8 {
                        return Err(BridgeRefusal::UnsupportedBorrowedPointer { parameter: index });
                    }
                    CallParameter::Borrowed(*abi)
                } else {
                    CallParameter::Data(*abi)
                }
            }
            ContractParameterMode::Consumed { nullable } => {
                if !abi.is_pointer() {
                    return Err(BridgeRefusal::ScalarCustody { parameter: index });
                }
                if *nullable {
                    return Err(BridgeRefusal::NullableConsumedParameter { parameter: index });
                }
                if *abi != AbiType::PtrMutU8 {
                    return Err(BridgeRefusal::UnsupportedConsumedPointer { parameter: index });
                }
                CallParameter::Consumed
            }
            ContractParameterMode::Owned { nullable } => {
                if !abi.is_pointer() {
                    return Err(BridgeRefusal::ScalarCustody { parameter: index });
                }
                if !designated_owners.contains(&index) {
                    return Err(BridgeRefusal::OwnedParameter { parameter: index });
                }
                // A reallocation subject and a store owner are both `Owned`
                // positions, but they are not the same position: the subject's
                // custody is replaced by the result, the owner's is kept. The
                // result mode decides which, and a parameter that is both is
                // refused below as a store beside an acquisition.
                if realloc_subject == Some(index) {
                    if *nullable {
                        return Err(BridgeRefusal::NullableReallocSubject { parameter: index });
                    }
                    if *abi != AbiType::PtrMutU8 {
                        return Err(BridgeRefusal::UnsupportedReallocSubjectPointer {
                            parameter: index,
                        });
                    }
                    CallParameter::ReallocSubject
                } else {
                    if *nullable {
                        return Err(BridgeRefusal::NullableOwnerParameter { parameter: index });
                    }
                    if *abi != AbiType::PtrMutU8 {
                        return Err(BridgeRefusal::UnsupportedOwnerPointer { parameter: index });
                    }
                    CallParameter::Owner
                }
            }
            ContractParameterMode::Released { nullable } => {
                if !abi.is_pointer() {
                    return Err(BridgeRefusal::ScalarCustody { parameter: index });
                }
                if *nullable {
                    return Err(BridgeRefusal::NullableReleasedParameter { parameter: index });
                }
                if *abi != AbiType::PtrMutU8 {
                    return Err(BridgeRefusal::UnsupportedReleasedPointer { parameter: index });
                }
                CallParameter::Released
            }
            ContractParameterMode::Out { nullable } => {
                if !abi.is_pointer() {
                    return Err(BridgeRefusal::ScalarCustody { parameter: index });
                }
                if !*nullable {
                    return Err(BridgeRefusal::NonnullOutParameter { parameter: index });
                }
                if *abi != AbiType::PtrMutPtrMutU8 {
                    return Err(BridgeRefusal::UnsupportedOutPointer { parameter: index });
                }
                if out_parameter.replace(index).is_some() {
                    return Err(BridgeRefusal::MultipleAcquisitions);
                }
                CallParameter::OutSlot
            }
            ContractParameterMode::Stored { owner, nullable } => {
                if !abi.is_pointer() {
                    return Err(BridgeRefusal::ScalarCustody { parameter: index });
                }
                if *nullable {
                    return Err(BridgeRefusal::NullableStoredParameter { parameter: index });
                }
                if *abi != AbiType::PtrMutU8 {
                    return Err(BridgeRefusal::UnsupportedStoredPointer { parameter: index });
                }
                if *owner == index {
                    return Err(BridgeRefusal::StoredOwnerIsItself { parameter: index });
                }
                if !matches!(
                    contract.parameters.get(*owner),
                    Some((_, ContractParameterMode::Owned { .. }))
                ) {
                    return Err(BridgeRefusal::StoredOwnerNotOwned {
                        parameter: index,
                        owner: *owner,
                    });
                }
                stored_parameter.get_or_insert(index);
                CallParameter::Stored { owner: *owner }
            }
            ContractParameterMode::Removed { .. } => {
                return Err(if abi.is_pointer() {
                    BridgeRefusal::RemovedParameter { parameter: index }
                } else {
                    BridgeRefusal::ScalarCustody { parameter: index }
                });
            }
        };
        parameters.push(bridged);
    }
    let result = match contract.result_mode {
        ContractReturnMode::None => CallResult::Data(contract.result),
        ContractReturnMode::Owned { nullable: false } => {
            return Err(BridgeRefusal::NonnullOwnedReturn);
        }
        ContractReturnMode::Owned { nullable: true } => {
            if contract.result != AbiType::PtrMutU8 {
                return Err(BridgeRefusal::UnsupportedOwnedResultType {
                    result: contract.result,
                });
            }
            CallResult::OwnedNullable
        }
        ContractReturnMode::Reallocated {
            nullable: false, ..
        } => return Err(BridgeRefusal::NonnullReallocatedReturn),
        ContractReturnMode::Reallocated {
            parameter,
            nullable: true,
        } => {
            if contract.result != AbiType::PtrMutU8 {
                return Err(BridgeRefusal::UnsupportedReallocatedResultType {
                    result: contract.result,
                });
            }
            CallResult::ReallocatedNullable { parameter }
        }
    };
    let acquires_by_result = matches!(
        result,
        CallResult::OwnedNullable | CallResult::ReallocatedNullable { .. }
    );
    if acquires_by_result && out_parameter.is_some() {
        return Err(BridgeRefusal::MultipleAcquisitions);
    }
    if let Some(parameter) = stored_parameter
        && (acquires_by_result || out_parameter.is_some())
    {
        return Err(BridgeRefusal::StoredWithAcquisition { parameter });
    }
    Ok(ExternCallee {
        name: name.to_owned(),
        trust: ExternTrust::Contracted,
        parameters,
        result,
        result_abi: contract.result,
        out_parameter,
        realloc_parameter: realloc_subject,
        // A contracted callee names no artifact: the manifest slice card
        // `UNSAFE-LINKAGE-ARTIFACT` landed binds contract-less entries only.
        residence: None,
    })
}

/// Why an `unsafe extern fn` declaration is still not callable (card
/// `UNSAFE-LINKAGE`). Each constructor names one excluded shape; none is a
/// catch-all, so widening the unchecked surface means deleting a refusal on
/// purpose.
///
/// The common ground of all three is that the admitted unchecked surface must
/// carry **no custody**. A callee with no contract has unknown effects, so the
/// only positions over which "unknown callee" is still a decidable statement
/// about the caller's ledger are the ones that hand it nothing to hold. See
/// `docs/design/UNSAFE_LINKAGE_V1.md` §3.1 for the four failure modes a pointer
/// argument opens and why none of them has an assumption text to record.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UncheckedRefusal {
    /// A pointer parameter. Handing unknown code a capability is not made
    /// checkable by any assumption: nothing bounds what it reads, nothing ends
    /// the loan at the return, `const` constrains this declaration rather than
    /// the implementation, and nothing stops the pointer being published to
    /// another thread. `ptr<const, u8>` is refused for exactly these reasons,
    /// not by oversight.
    PointerParameter { parameter: usize, abi: AbiType },
    /// A pointer result. Custody arriving from an uncontracted callee has no
    /// declared release, so binding it would open an obligation nothing in this
    /// profile could discharge.
    PointerResult { abi: AbiType },
    /// A `unit` parameter, which the ABI vocabulary admits as a result type
    /// only.
    UnitParameter { parameter: usize },
}

impl fmt::Display for UncheckedRefusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::PointerParameter { parameter, abi } => write!(
                f,
                "parameter {parameter} is `{}`; unchecked linkage passes scalars by value only, because no assumption makes a pointer handed to unknown code checkable",
                abi.c_spelling()
            ),
            Self::PointerResult { abi } => write!(
                f,
                "the result is `{}`; unchecked linkage returns scalars and `void` only, because custody from an uncontracted callee has no declared release",
                abi.c_spelling()
            ),
            Self::UnitParameter { parameter } => {
                write!(f, "parameter {parameter} is declared `unit`")
            }
        }
    }
}

impl std::error::Error for UncheckedRefusal {}

/// Admit one `unsafe extern fn` declaration into the closed unchecked table, or
/// refuse it by name (card `UNSAFE-LINKAGE`).
///
/// There is no contract here and none is consulted: the caller declares the ABI
/// signature and this function decides only whether that signature can cross a
/// boundary nothing checks. Deterministic, exactly as [`bridge_contract`] is,
/// so a certificate consumer can repeat the admission.
pub fn unchecked_callee(
    name: &str,
    parameters: &[AbiType],
    result: AbiType,
) -> Result<ExternCallee, UncheckedRefusal> {
    let mut bridged = Vec::with_capacity(parameters.len());
    for (index, abi) in parameters.iter().enumerate() {
        if *abi == AbiType::Unit {
            return Err(UncheckedRefusal::UnitParameter { parameter: index });
        }
        if abi.is_pointer() {
            return Err(UncheckedRefusal::PointerParameter {
                parameter: index,
                abi: *abi,
            });
        }
        bridged.push(CallParameter::Data(*abi));
    }
    if result.is_pointer() {
        return Err(UncheckedRefusal::PointerResult { abi: result });
    }
    Ok(ExternCallee {
        name: name.to_owned(),
        trust: ExternTrust::UncheckedLinkage,
        parameters: bridged,
        // Data, so `acquires()` is false: an unchecked call takes no adapter,
        // binds no identity and reaches the existing plain-call arm of the
        // backend and the reference interpreter unchanged.
        result: CallResult::Data(result),
        result_abi: result,
        out_parameter: None,
        realloc_parameter: None,
        residence: None,
    })
}

/// Admit one `unsafe extern fn` declaration that a library manifest names, or
/// refuse it by name (card `UNSAFE-LINKAGE-ARTIFACT`).
///
/// This is [`unchecked_callee`] with the manifest's residence attached, and it
/// is deliberately nothing more. **The admitted surface does not move**: the
/// same closed table decides, so a manifest cannot buy a pointer for a
/// contract-less entry — an artifact hash says *which code*, not what it does
/// with your pointer (`docs/design/UNSAFE_LINKAGE_V1.md` §4.2). What the
/// manifest changes is only the premise the callee contributes.
///
/// The manifest's own type gate
/// ([`native_library::check_manifest_agreement`]) is a separate decision and
/// belongs to the driver, because it compares the *declaration* against the
/// manifest rather than deciding what may cross.
pub fn manifest_unchecked_callee(
    name: &str,
    parameters: &[AbiType],
    result: AbiType,
    residence: native_library::ManifestResidence,
) -> Result<ExternCallee, UncheckedRefusal> {
    let mut callee = unchecked_callee(name, parameters, result)?;
    callee.residence = Some(residence);
    Ok(callee)
}

/// Attach a manifest's contract slice residence to an **already admitted**
/// contracted callee (card `NATIVE-LIBRARY-CONTRACT-SLICE`).
///
/// The callee must have come out of [`bridge_contract`]: the unchanged C04
/// admission table decides what may cross, first and alone, so a manifest
/// binding cannot widen the contracted surface any more than it could widen the
/// waived one. What it adds is the artifact premise, stacked beside the
/// declared-contract premise the callee already contributes.
///
/// Deliberately takes the callee by value and returns it, so there is no way to
/// attach a residence to a callee some other code path is still holding as
/// unattributed.
#[must_use]
pub fn manifest_contracted_callee(
    mut callee: ExternCallee,
    residence: native_library::ManifestResidence,
) -> ExternCallee {
    debug_assert_eq!(
        callee.trust,
        ExternTrust::Contracted,
        "a contract slice residence attaches to a contracted callee only"
    );
    debug_assert!(
        residence.contract.is_some(),
        "a contract slice residence names the contract module it bound"
    );
    callee.residence = Some(residence);
    callee
}

/// The premise one admitted callee contributes, as a **total match on its
/// trust** and, within the waived arm, on whether a manifest names it.
///
/// Three arms, no fourth: a contracted callee contributes its declared
/// contract, a waived callee no manifest names contributes the bare waiver, and
/// a manifest-resident waived callee contributes the sharpened one. There is no
/// arm that contributes nothing, which is what makes the inventory total.
#[must_use]
pub fn assumption_for_callee(callee: &ExternCallee) -> ExternAssumption {
    match (callee.trust, callee.residence.as_ref()) {
        (ExternTrust::Contracted, _) => ExternAssumption::DeclaredContract(callee.name.clone()),
        (ExternTrust::UncheckedLinkage, None) => ExternAssumption::UncheckedLinkage {
            callee: callee.name.clone(),
            signature: declared_signature(callee),
        },
        (ExternTrust::UncheckedLinkage, Some(residence)) => {
            ExternAssumption::ManifestUncheckedLinkage {
                callee: callee.name.clone(),
                signature: declared_signature(callee),
                residence: residence.clone(),
            }
        }
    }
}

/// The artifact premise a **contracted** manifest-resident callee contributes
/// *in addition to* its declared-contract premise (card
/// `NATIVE-LIBRARY-CONTRACT-SLICE`), or `None` when no manifest binds it.
///
/// Kept separate from [`assumption_for_callee`] rather than folded into it,
/// because the contracted route stacks its premises: the declared contract is
/// what the code does, the residence is which code it is, and neither is
/// derivable from the other. [`assumptions_for_callee`] is the total inventory
/// of the two.
#[must_use]
pub fn residence_assumption_for_callee(callee: &ExternCallee) -> Option<ExternAssumption> {
    match (callee.trust, callee.residence.as_ref()) {
        (ExternTrust::Contracted, Some(residence)) => {
            residence
                .contract
                .as_ref()
                .map(|module| ExternAssumption::ManifestResidentContract {
                    callee: callee.name.clone(),
                    contract_module: module.clone(),
                    residence: residence.clone(),
                })
        }
        _ => None,
    }
}

/// Every premise one admitted callee contributes, in publication order.
///
/// One for every callee — [`assumption_for_callee`]'s total match — plus, for a
/// contracted callee a manifest binds, the artifact premise beside it. The
/// first element is therefore always the callee's trust premise, and the
/// inventory is never empty: there is no callee that rests on nothing.
#[must_use]
pub fn assumptions_for_callee(callee: &ExternCallee) -> Vec<ExternAssumption> {
    let mut premises = vec![assumption_for_callee(callee)];
    premises.extend(residence_assumption_for_callee(callee));
    premises
}

/// The declared signature of an admitted callee, rendered from the ABI
/// projection alone. This is the second half of the unchecked-linkage record:
/// the premise names *this identity with this ABI*, so a reader can tell two
/// symbols of the same name apart, and a rederivation reproduces it exactly.
#[must_use]
pub fn declared_signature(callee: &ExternCallee) -> String {
    let parameters = callee
        .parameters
        .iter()
        .map(|parameter| {
            let abi = match parameter {
                CallParameter::Data(abi) | CallParameter::Borrowed(abi) => *abi,
                CallParameter::Consumed
                | CallParameter::Stored { .. }
                | CallParameter::Owner
                | CallParameter::Released
                | CallParameter::ReallocSubject => AbiType::PtrMutU8,
                CallParameter::OutSlot => AbiType::PtrMutPtrMutU8,
            };
            abi.c_spelling().to_owned()
        })
        .collect::<Vec<_>>()
        .join(", ");
    format!("({parameters}) -> {}", callee.result_abi.c_spelling())
}

/// The custody a call site imposes on its resource-bearing arguments, in
/// declaration order and excluding data parameters. This is exactly the
/// `parameters` vector of the interprocedural summary the caller consumes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExternCustody {
    /// A lexical loan; the caller keeps the same identity.
    Borrowed,
    /// Ownership leaves the caller through the callee.
    Consumed,
    /// Ownership leaves the caller and becomes indexed beneath the declared
    /// owner argument. The caller's ledger delta is exactly a consumption; the
    /// owner designation is recorded on the certificate and premised, not
    /// checked.
    Stored,
    /// A loan with a liveness precondition: the caller supplies live custody
    /// for a declared store owner and keeps the identity.
    Owner,
    /// The declared contract discharges the obligation. The caller-side delta
    /// is the same as [`Self::Consumed`]; the difference is what is claimed,
    /// recorded as [`ExternAssumption::DeclaredRelease`].
    Released,
    /// The identity is replaced by the call's result on the success arm and
    /// preserved on the failure arm (contract spec §C04.3). Unlike every other
    /// departing mode, both arms are declared, which is why the adapter admits
    /// it where it refuses a consumption beside an acquisition.
    Reallocated,
}

/// Derived custody delta over the *resource-bearing arguments*, in declaration
/// order. Data parameters carry no custody, and an out slot carries no caller
/// identity at all — the identity it establishes is the adapter's, recorded in
/// [`ExternCallCertificate::out_acquisitions`] — so both are absent here and
/// this vector stays aligned with the caller's argument identity list.
#[must_use]
pub fn derived_custody(callee: &ExternCallee) -> Vec<ExternCustody> {
    callee
        .parameters
        .iter()
        .filter_map(|parameter| match parameter {
            CallParameter::Data(_) | CallParameter::OutSlot => None,
            CallParameter::Borrowed(_) => Some(ExternCustody::Borrowed),
            CallParameter::Consumed => Some(ExternCustody::Consumed),
            CallParameter::Stored { .. } => Some(ExternCustody::Stored),
            CallParameter::Owner => Some(ExternCustody::Owner),
            CallParameter::Released => Some(ExternCustody::Released),
            CallParameter::ReallocSubject => Some(ExternCustody::Reallocated),
        })
        .collect()
}

/// Derived effect row (design §3.4): `Io` always, plus `Alloc` exactly when the
/// call establishes fresh custody through either admitted acquisition source,
/// plus `Mut` exactly when some parameter is a `mut` pointer. The caller's
/// declared row must admit this; that containment check belongs to the
/// established typed-error pass, not to this module.
///
/// Card `UNSAFE-LINKAGE`: an unchecked callee derives the **whole modeled row**
/// unconditionally. Nothing is known about it, so the row cannot honestly be
/// narrower than everything this profile can name, and the practical effect is
/// that an entry calling one must declare `effects { io, alloc, mut }` — the
/// opt-in stays visible in the entry's own signature, not only at the
/// declaration.
#[must_use]
pub fn derived_effects(callee: &ExternCallee) -> EffectRow {
    if callee.trust == ExternTrust::UncheckedLinkage {
        return EffectRow::new(BTreeSet::from([Effect::Io, Effect::Alloc, Effect::Mut]));
    }
    let mut effects = BTreeSet::from([Effect::Io]);
    if callee.acquires() {
        effects.insert(Effect::Alloc);
    }
    if callee.parameters.iter().any(|parameter| match parameter {
        CallParameter::Data(abi) | CallParameter::Borrowed(abi) => abi.is_mutable_pointer(),
        CallParameter::Consumed
        | CallParameter::Stored { .. }
        | CallParameter::Owner
        | CallParameter::OutSlot
        | CallParameter::Released
        | CallParameter::ReallocSubject => true,
    }) {
        effects.insert(Effect::Mut);
    }
    EffectRow::new(effects)
}

/// One argument at a checked call site.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExternArgument {
    /// A scalar literal. Data only.
    Data(i64),
    /// A live model Buffer lent for the duration of the call.
    Borrow(ResourceId),
    /// A live `own<Buffer>` moved into the callee.
    Consume(ResourceId),
    /// A live `own<Buffer>` moved into the callee's declared owner. Spelled
    /// `move NAME` in source, exactly like a consumption: the caller's ledger
    /// delta is identical and the destination is declared by the contract, not
    /// by the call site.
    Store(ResourceId),
    /// A live identity supplied as a declared store owner, kept by the caller.
    Owner(ResourceId),
    /// The declared out slot. It carries no caller identity: the identity it
    /// establishes is the adapter's.
    OutSlot,
    /// A live `own<Buffer>` the declared contract discharges. Spelled
    /// `move NAME` in source, exactly like a consumption or a store: which of
    /// the three it is comes from the declared mode, never from the call site,
    /// so this profile adds no reserved word.
    Release(ResourceId),
    /// A live `own<Buffer>` supplied as the subject of a declared
    /// reallocation. It survives the failure arm and is replaced on the
    /// success arm.
    Realloc(ResourceId),
    /// A float literal at a declared float parameter (card
    /// `W48-QR-FLOATING-POINT`), read once at that parameter's type: its bit
    /// pattern. Data only, like [`Self::Data`].
    Float { ty: ScalarType, bits: i128 },
    /// A bound scalar local, read at a declared `Data` parameter (card
    /// `PIPE-SCALAR-RESULT`). Data, exactly as [`Self::Data`] is: it names a
    /// [`ValueId`], carries no custody, and is admitted at no other parameter
    /// mode.
    Value(ValueId),
}

/// One disposition on an adapter's error arm (card `EXTERN-ERR-REL`): a call
/// that discharges exactly one still-live identity before the raise.
///
/// This is not a new custody rule. `qcore_core::cleanup_lowering` already
/// releases every identity live before a failing acquisition, in reverse
/// acquisition order, on that acquisition's cleanup edge. What this profile
/// lacked was a *rendering* of that edge: an extern-acquired identity is
/// released only by calling a declared `resource_consumed` callee, and the
/// profile cannot invent one. The error arm therefore names the calls the
/// cleanup edge performs, and [`check_extern_program`] requires them to be
/// exactly that edge — same identities, same reverse-acquisition order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExternRelease {
    pub callee: String,
    pub arguments: Vec<ExternArgument>,
}

/// The mandatory nullable-owned-return adapter (design §3.3): the null arm is
/// branched on at the call site and takes the declared typed error exit.
/// `succeeds` is the fixture's literal outcome control; both arms are
/// exercisable, and the emitted C checks the real callee against it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExternAdapter {
    pub resource: ResourceId,
    pub succeeds: bool,
    /// The error arm's dispositions, in the order the handler writes them.
    /// Empty is the v1 shape: an error arm that abandons nothing because
    /// nothing else is live.
    pub releases: Vec<ExternRelease>,
    pub error: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExternCall {
    pub callee: String,
    pub arguments: Vec<ExternArgument>,
    /// Present exactly for a `resource_return_owned(nullable)` callee.
    pub adapter: Option<ExternAdapter>,
    /// The scalar local this call's result is bound to (card
    /// `PIPE-SCALAR-RESULT`), present exactly for a call that
    /// [`binds_scalar`]. Exclusive with `adapter` by construction: an
    /// acquisition binds an identity, a scalar binding binds a value, and a
    /// callee cannot be both.
    pub binding: Option<ScalarBinding>,
}

/// One bound scalar result: the value identity it introduces and, optionally,
/// the number the site declares the call returns (card `STD-IO-COMPOSE`).
///
/// The optional half is the [`ExternAdapter::succeeds`] control one step
/// further: `succeeds true` declares *which arm* a nullable owned return takes,
/// and `succeeds INTEGER` declares *which number* a scalar result carries. Both
/// are fixture-literal outcome controls in the `qcore_acquire`/`qcore_check`
/// sense — a controlled model operation, never a claim about what a production
/// callee computes — and neither is believed by the emitted program, which
/// compares the real result against the control and exits 70 on disagreement.
///
/// `None` is the card-`PIPE-SCALAR-RESULT` binding: the value is bound and its
/// number is not modeled, so nothing may read it as data.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScalarBinding {
    pub value: ValueId,
    /// Card `W48-QR-SCALAR-TYPES-R3`: a number of the result's own type, of
    /// any width (before it, an `i32` literal whatever the result's type).
    pub outcome: Option<i128>,
}

/// One scalar literal binding, `let NAME = INTEGER;` (card
/// `EXTERN-SCALAR-LITERAL`, folded into `STD-IO-COMPOSE`).
///
/// It calls nothing, so it derives no effect, admits no argument and can carry
/// no custody in any position — the narrowest possible widening of the value
/// namespace, and the one that makes a rendered value able to originate in the
/// module that renders it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScalarLiteral {
    pub value: ValueId,
    pub literal: i32,
}

/// `let NAME = ["-"] FLOAT;` — a float literal binding (card
/// `W48-QR-FLOATING-POINT`): an `f64`, the literal read once at `f64`. An `f32`
/// literal is bound with `let NAME = FLOAT as f32;`, a value statement that
/// reads the literal at `f32` directly.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FloatBinding {
    pub value: ValueId,
    pub bits: i128,
}

/// One operand of a value statement (card `W48-QR-SCALAR-TYPES-R3`): a bound
/// value of the namespace, an integer literal — typed by its context, `i32`
/// when nothing gives it one — or `true`/`false`; and (card
/// `W48-QR-FLOATING-POINT`) a float literal, `f64` when nothing types it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ValueOperand {
    Value(ValueId),
    Integer(i128),
    Bool(bool),
    Float(crate::scalar_floats::FloatLiteral),
}

/// The one §G02t operation a value statement performs (card
/// `W48-QR-SCALAR-TYPES-R3`). One operation per statement: a longer
/// computation is a sequence of bindings, each checked on its own.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ValueOperation {
    /// `a OP b`, checked at the operands' one type.
    Binary(Op, ValueOperand, ValueOperand),
    /// `a CMP b`, producing a `bool`.
    Compare(Cmp, ValueOperand, ValueOperand),
    /// `-a`, checked.
    Negate(ValueOperand),
    /// `a as TYPE`, total or checked.
    Convert(ValueOperand, ScalarType),
}

/// `let NAME = VALUE_EXPRESSION;` — a value statement (card
/// `W48-QR-SCALAR-TYPES-R3`): a new value of the namespace computed from bound
/// values and literals. It calls nothing and carries no custody. Whether its
/// number is known at check time — every operand known — or only at run time
/// is the checker's to decide, and so is whether it can fault.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ValueStatement {
    pub value: ValueId,
    pub operation: ValueOperation,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ExternEvent {
    Call(ExternCall),
    /// `let NAME = INTEGER;` — see [`ScalarLiteral`].
    Bind(ScalarLiteral),
    /// `let NAME = FLOAT;` — see [`FloatBinding`] (card
    /// `W48-QR-FLOATING-POINT`).
    BindFloat(FloatBinding),
    /// A value statement — see [`ValueStatement`] (card
    /// `W48-QR-SCALAR-TYPES-R3`).
    Compute(ValueStatement),
    Return,
}

/// A complete checked module: the admitted callees and the entry's call trace.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExternProgram {
    pub entry: String,
    pub callees: BTreeMap<String, ExternCallee>,
    pub events: Vec<ExternEvent>,
    /// Error names this module declares. The adapter must name one.
    pub errors: BTreeSet<String>,
    /// The `extern library NAME;` declarations this module's source file made,
    /// resolved against the manifest selection (card
    /// `NATIVE-LIBRARY-SOURCE-BINDING`), in the order the file declared them.
    ///
    /// Empty is the pre-card world and every existing program's state: an
    /// unbound module publishes no binding premise and its certificate is
    /// byte-identical.
    ///
    /// It is a field of the **program** rather than a derived fact about the
    /// callees because it is not one: a file binds a build whether or not it
    /// calls anything that build exports, and that act is what a reader of the
    /// certificate has to be able to see. Keeping it here is also what makes it
    /// rederivable — [`recheck_extern_certificate`] re-derives from the program
    /// alone.
    pub libraries: Vec<native_library::SourceLibraryBinding>,
    /// Whether the entry's row declares `throws<ArithmeticError>` (card
    /// `W48-QR-SCALAR-TYPES-R3`): the permission a value statement needs when
    /// its operands are not all known and its operation can fault. Never
    /// inferred; `false` for every program before the card.
    pub raises: bool,
    /// The builder's recorded target, which alone gives `isize`/`usize` values
    /// a range (card `W48-QR-SCALAR-TYPES-R3`).
    pub target: Option<RecordedTarget>,
    /// The comparison effects the entry's row declares (card
    /// `W48-QR-FLOATING-POINT`): `partial_eq` permits a float `==`/`!=`,
    /// `partial_ord` a float ordering. Never inferred; empty for every program
    /// before the card.
    pub float_effects: crate::scalar_types::FloatEffects,
}

/// Why a value of the namespace is refused at a position (card
/// `W48-QR-SCALAR-TYPES-R3`). Each is a condition of `Q024`, the value
/// namespace's existing code, widened; none allocates a code. Defined in
/// [`crate::scalar_values`] since card `W48-QR-SCALAR-TYPES-R4`, which shares
/// the value statement's decision with the task route.
pub use crate::scalar_values::ValueRefusal;

/// Refusals this checker owns. Custody liveness, leaks, resource flow and the
/// caller's declared effect row are checked by the established sequential
/// passes under their own codes and are deliberately absent here.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ExternDiagnostic {
    /// Q023: the call names no admitted declared contract.
    UnknownCallee { event: usize, name: String },
    /// Q024: the call passes a different number of arguments than the
    /// declaration has parameters.
    ArgumentArity {
        event: usize,
        expected: usize,
        actual: usize,
    },
    /// Q024: an argument's form does not match its declared parameter mode.
    ArgumentForm {
        event: usize,
        parameter: usize,
        expected: CallParameter,
    },
    /// Q025: an owned nullable return with no null adapter at the call site.
    AdapterMissing { event: usize },
    /// Q025: an adapter on a call that returns no custody.
    AdapterUnexpected { event: usize },
    /// Q025: the adapter's error arm does not name a declared error type.
    AdapterUnknownError { event: usize, error: String },
    /// Q025: the adapter's error arm would abandon live custody. Card
    /// `EXTERN-ERR-REL` narrows *when* this fires — an identity the error arm
    /// explicitly releases is no longer abandoned — without weakening what it
    /// means: an identity that reaches the raise still live is still refused,
    /// with the same code and the same named identity.
    AdapterAbandonsCustody { event: usize, resource: ResourceId },
    /// Q025: an acquiring call also carries departing custody, so that
    /// custody's status on the failure arm is undecided — the declaration
    /// speaks only about the normal exit. Refused rather than assumed either
    /// way; this shape was already refused in v1, as abandonment.
    AdapterDepartingArgument { event: usize, resource: ResourceId },
    /// Q023: an error-arm disposition names a callee with no admitted declared
    /// contract. Kept distinct from [`Self::UnknownCallee`] so a refusal says
    /// which position it came from.
    ReleaseUnknownCallee { event: usize, name: String },
    /// Q024: an error-arm disposition passes the wrong number of arguments.
    ReleaseArity {
        event: usize,
        expected: usize,
        actual: usize,
    },
    /// Q024: an error-arm disposition's argument does not match its declared
    /// parameter mode.
    ReleaseArgumentForm {
        event: usize,
        parameter: usize,
        expected: CallParameter,
    },
    /// Q025: an error-arm disposition establishes fresh custody. The error arm
    /// discharges obligations; it never opens a new one, because a second
    /// acquisition there would need its own checked adapter.
    ReleaseAcquires { event: usize, name: String },
    /// Q025: an error-arm disposition stores custody beneath a declared owner
    /// instead of releasing it. That converts an obligation into a premise on
    /// a path taken because something already failed, so it is refused.
    ReleaseStores { event: usize, name: String },
    /// Q025: an error-arm disposition does not discharge exactly one identity.
    /// A disposition is a release; a call that consumes nothing releases
    /// nothing, and one that consumes two is not one disposition.
    ReleaseNotSingular {
        event: usize,
        name: String,
        discharged: usize,
    },
    /// Q025: an error-arm disposition names an identity that is not live at
    /// that point on the error arm — already released by an earlier
    /// disposition, already consumed, or already stored.
    ReleaseNotLive { event: usize, resource: ResourceId },
    /// Q025: the error arm releases the right identities in an order the
    /// established cleanup edge does not perform. `qcore_core::cleanup_lowering`
    /// releases live identities in reverse acquisition order; a handler that
    /// states another order describes a different program than the sequential
    /// lowering of the same module.
    ReleaseOrder {
        event: usize,
        expected: Vec<ResourceId>,
        actual: Vec<ResourceId>,
    },
    /// Q024: an acquisition reuses an identity the trace already introduced.
    DuplicateAcquisition { event: usize, resource: ResourceId },
    /// Q024: a stored argument names the same identity as its declared owner.
    /// Custody indexed beneath itself has no modeled meaning.
    StoredUnderItself { event: usize, resource: ResourceId },
    /// Q024: the call binds its result, but the declared result is not a
    /// custody-free scalar — a `unit`, or a pointer the declaration says
    /// nothing about (card `PIPE-SCALAR-RESULT`).
    ResultNotScalar {
        event: usize,
        name: String,
        result: AbiType,
    },
    /// Q025: the call binds its result to a scalar local, but it establishes
    /// fresh custody. An acquisition has exactly one spelling — the mandatory
    /// `succeeds BOOL else raise ERROR` adapter — and this card does not add a
    /// second one.
    ResultBindingOnAcquisition { event: usize, name: String },
    /// Q024: a scalar binding reuses a value identity the trace already bound.
    DuplicateValueBinding { event: usize, value: ValueId },
    /// Q024: an argument reads a scalar value no earlier call in this trace
    /// bound. The mirror of [`Self::UnknownCallee`] for the value namespace,
    /// and the reason a scalar can never be conjured at a call site.
    ValueNotBound {
        event: usize,
        parameter: usize,
        value: ValueId,
    },
    /// Q024: the §G02t typing of the value namespace refuses (card
    /// `W48-QR-SCALAR-TYPES-R3`) — see [`ValueRefusal`].
    Value { event: usize, refusal: ValueRefusal },
}

impl ExternDiagnostic {
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            Self::UnknownCallee { .. } | Self::ReleaseUnknownCallee { .. } => "Q023",
            Self::ArgumentArity { .. }
            | Self::ArgumentForm { .. }
            | Self::DuplicateAcquisition { .. }
            | Self::StoredUnderItself { .. }
            | Self::ReleaseArity { .. }
            | Self::ReleaseArgumentForm { .. }
            | Self::ResultNotScalar { .. }
            | Self::DuplicateValueBinding { .. }
            | Self::ValueNotBound { .. }
            | Self::Value { .. } => "Q024",
            Self::ResultBindingOnAcquisition { .. } => "Q025",
            Self::AdapterMissing { .. }
            | Self::AdapterUnexpected { .. }
            | Self::AdapterUnknownError { .. }
            | Self::AdapterAbandonsCustody { .. }
            | Self::AdapterDepartingArgument { .. }
            | Self::ReleaseAcquires { .. }
            | Self::ReleaseStores { .. }
            | Self::ReleaseNotSingular { .. }
            | Self::ReleaseNotLive { .. }
            | Self::ReleaseOrder { .. } => "Q025",
        }
    }

    /// Index of the offending event, for source attribution.
    #[must_use]
    pub const fn event(&self) -> usize {
        match self {
            Self::UnknownCallee { event, .. }
            | Self::ArgumentArity { event, .. }
            | Self::ArgumentForm { event, .. }
            | Self::AdapterMissing { event }
            | Self::AdapterUnexpected { event }
            | Self::AdapterUnknownError { event, .. }
            | Self::AdapterAbandonsCustody { event, .. }
            | Self::AdapterDepartingArgument { event, .. }
            | Self::DuplicateAcquisition { event, .. }
            | Self::StoredUnderItself { event, .. }
            | Self::ReleaseUnknownCallee { event, .. }
            | Self::ReleaseArity { event, .. }
            | Self::ReleaseArgumentForm { event, .. }
            | Self::ReleaseAcquires { event, .. }
            | Self::ReleaseStores { event, .. }
            | Self::ReleaseNotSingular { event, .. }
            | Self::ReleaseNotLive { event, .. }
            | Self::ReleaseOrder { event, .. }
            | Self::ResultNotScalar { event, .. }
            | Self::ResultBindingOnAcquisition { event, .. }
            | Self::DuplicateValueBinding { event, .. }
            | Self::ValueNotBound { event, .. }
            | Self::Value { event, .. } => *event,
        }
    }
}

impl fmt::Display for ExternDiagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownCallee { name, .. } => write!(
                f,
                "`{name}` has no admitted declared contract in the selected extern contract environment"
            ),
            Self::ArgumentArity {
                expected, actual, ..
            } => write!(
                f,
                "the declared contract has {expected} parameters; the call passes {actual} arguments"
            ),
            Self::ArgumentForm {
                parameter,
                expected,
                ..
            } => write!(
                f,
                "argument {parameter} does not bridge to its declared mode {expected:?}"
            ),
            Self::AdapterMissing { .. } => f.write_str(
                "a call that establishes fresh custody — a `resource_return_owned(nullable)` result or a `resource_out(_, nullable)` slot — requires the checked adapter `succeeds BOOL else raise ERROR`",
            ),
            Self::AdapterUnexpected { .. } => {
                f.write_str("this callee returns no custody, so it takes no null adapter")
            }
            Self::AdapterUnknownError { error, .. } => {
                write!(f, "the adapter raises undeclared error type `{error}`")
            }
            Self::AdapterAbandonsCustody { resource, .. } => write!(
                f,
                "the adapter's error arm would abandon live resource {resource}; every other live identity must be released by an explicit disposition before the raise"
            ),
            Self::AdapterDepartingArgument { resource, .. } => write!(
                f,
                "the call both establishes fresh custody and moves live resource {resource} out; what became of {resource} on the failure arm is not declared"
            ),
            Self::ReleaseUnknownCallee { name, .. } => write!(
                f,
                "the error-arm disposition calls `{name}`, which has no admitted declared contract in the selected extern contract environment"
            ),
            Self::ReleaseArity {
                expected, actual, ..
            } => write!(
                f,
                "an error-arm disposition's declared contract has {expected} parameters; the call passes {actual} arguments"
            ),
            Self::ReleaseArgumentForm {
                parameter,
                expected,
                ..
            } => write!(
                f,
                "error-arm disposition argument {parameter} does not bridge to its declared mode {expected:?}"
            ),
            Self::ReleaseAcquires { name, .. } => write!(
                f,
                "the error-arm disposition calls `{name}`, which establishes fresh custody; an error arm discharges obligations and never opens one"
            ),
            Self::ReleaseStores { name, .. } => write!(
                f,
                "the error-arm disposition calls `{name}`, which stores custody beneath a declared owner instead of releasing it"
            ),
            Self::ReleaseNotSingular {
                name, discharged, ..
            } => write!(
                f,
                "the error-arm disposition calls `{name}`, which discharges {discharged} identities; a disposition releases exactly one"
            ),
            Self::ReleaseNotLive { resource, .. } => write!(
                f,
                "the error-arm disposition names resource {resource}, which is not live at that point on the error arm"
            ),
            Self::ReleaseOrder {
                expected, actual, ..
            } => write!(
                f,
                "the error arm releases {actual:?}; the established cleanup edge of this acquisition releases {expected:?}, in reverse acquisition order"
            ),
            Self::DuplicateAcquisition { resource, .. } => {
                write!(f, "resource identity {resource} is acquired twice")
            }
            Self::StoredUnderItself { resource, .. } => write!(
                f,
                "resource identity {resource} is stored beneath itself; a declared owner must be a distinct live identity"
            ),
            Self::ResultNotScalar { name, result, .. } => write!(
                f,
                "`{name}` declares the result `{}`, which is not a custody-free scalar; only a scalar result may be bound to a local",
                result.c_spelling()
            ),
            Self::ResultBindingOnAcquisition { name, .. } => write!(
                f,
                "`{name}` establishes fresh custody, so its result is an identity and not a scalar; it takes the checked adapter `succeeds BOOL else raise ERROR`"
            ),
            Self::DuplicateValueBinding { value, .. } => {
                write!(f, "scalar value {value} is bound twice")
            }
            Self::ValueNotBound {
                parameter, value, ..
            } => write!(
                f,
                "argument {parameter} reads scalar value {value}, which no earlier call in this trace bound"
            ),
            Self::Value { refusal, .. } => refusal.fmt(f),
        }
    }
}

impl std::error::Error for ExternDiagnostic {}

/// Premises a certificate through this route carries. None of them is checked
/// by this module; naming them is the point.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ExternAssumption {
    /// The named callee's declared contract is a trusted assumption about a C
    /// implementation this repository never analysed (`ContractTrust::Declared`).
    DeclaredContract(String),
    /// Card `UNSAFE-LINKAGE`. The named callee has **no contract at all**: an
    /// external symbol was linked under an explicit `unsafe extern fn`
    /// declaration that states its ABI signature and nothing else. Every result
    /// a program obtains through it holds only relative to this premise, and
    /// the premise says exactly what it is — *this identity, with this declared
    /// ABI, was linked and not checked*.
    ///
    /// Nothing about the callee's behavior is claimed here, not even the
    /// weakened claims a declared contract makes: no custody, no effect, no
    /// termination, no thread-safety. What the profile does establish is that
    /// the caller handed it nothing to hold — the admitted unchecked surface is
    /// scalars by value in and out ([`unchecked_callee`]) — so the caller's own
    /// ledger is unchanged by the call whatever the callee does.
    UncheckedLinkage { callee: String, signature: String },
    /// Card `UNSAFE-LINKAGE-ARTIFACT`. The strongest honest form of the waiver
    /// above: the named callee has no contract **and** a library manifest
    /// declares it, so the premise names *which build* was waived and under
    /// which stated ABI premises — level L1 of the linkage spectrum
    /// (`docs/design/UNSAFE_LINKAGE_V1.md` §1.1, §4.2).
    ///
    /// It says strictly more than [`Self::UncheckedLinkage`] and strictly less
    /// than [`Self::DeclaredContract`]: still nothing whatever about the
    /// callee's behavior — no custody, no effect, no termination — but the
    /// waiver is now attached to a named artifact rather than to whatever the
    /// linker happened to supply. The identity is **stated, not probed**: see
    /// [`crate::native_library`] for why hashing a candidate file at check time
    /// would be a stronger-looking statement that is not a stronger one.
    ///
    /// The two variants are deliberately distinct rather than one variant with
    /// an optional field, so a bare waiver stays a valid weaker statement
    /// instead of becoming a malformed sharpened one, and a reader can tell
    /// them apart by inspection.
    ManifestUncheckedLinkage {
        callee: String,
        signature: String,
        residence: native_library::ManifestResidence,
    },
    /// Card `NATIVE-LIBRARY-CONTRACT-SLICE`. The trust artifact of level **L3**
    /// of the linkage spectrum (`docs/design/UNSAFE_LINKAGE_V1.md` §1.1):
    /// the named callee **has** a declared contract, and a
    /// `source.native-library.v1` manifest binds its symbol to that contract
    /// and to a named build. The premise reads *contracted `callee` per
    /// contract `contract_module`, from artifact `…` under stated ABI
    /// premises*.
    ///
    /// This premise **stacks with** [`Self::DeclaredContract`] and never
    /// replaces it. The two say independent things: the contract premise is
    /// about what the code does, and this one is about which code it is. A
    /// certificate through a manifest-bound contracted callee therefore carries
    /// both, and dropping either would be a weaker statement published by
    /// accident.
    ///
    /// That is the one deliberate asymmetry with
    /// [`Self::ManifestUncheckedLinkage`], which *replaces*
    /// [`Self::UncheckedLinkage`]: there the manifest restates the very thing
    /// the bare premise carried (the signature), so one record is the whole
    /// statement and two would be one statement told twice. Here it restates
    /// nothing the contract premise says.
    ///
    /// The artifact identity is a **stated pin**, exactly as at L1: nothing is
    /// hashed, opened or searched for at check time
    /// (`docs/reports/wave19-unsafe-linkage-artifact/README.md` §5). That the
    /// declared contract describes the named build is the premise; agreement of
    /// the manifest's header declaration with the contract's ABI projection is
    /// *checked*, and a disagreement withdraws the binding rather than
    /// reinterpreting it.
    ManifestResidentContract {
        callee: String,
        contract_module: String,
        residence: native_library::ManifestResidence,
    },
    /// Card `NATIVE-LIBRARY-SOURCE-BINDING`. The source file itself declared
    /// `extern library NAME;`, and the premise names what that resolved to: the
    /// name the author wrote, the manifest that declared it, and the artifact
    /// identity beside the ABI premises of that build.
    ///
    /// It is a premise about **selection**, not about behaviour, and it stacks
    /// with every callee premise rather than replacing any of them: the callee
    /// premises say what each symbol was read as, and this one says which build
    /// this *file* declared its extern surface to come from. A module that binds
    /// a library and calls nothing it exports still carries this premise, which
    /// is the point — the binding is a trust act whether or not a call site
    /// consumed it, and a certificate that dropped it would be silent about a
    /// statement the source made.
    ///
    /// As at every other level, the identity is a **stated pin**: the resolution
    /// is from a name to a declaration, and nothing is hashed, opened or
    /// searched for. What it buys is that a later selection resolving the same
    /// name to a different build is a *visible* difference in the published
    /// evidence.
    SourceBoundLibrary(native_library::SourceLibraryBinding),
    /// C04 pointers are extentless `u8` projections, so no declaration in this
    /// profile promises how many bytes a callee touches.
    NoExtentTransport,
    /// Which object file provides each symbol is the builder's premise.
    LinkageSuppliedByBuilder,
    /// The named callee declares a `resource_out(_, nullable)` slot. That the
    /// callee actually writes it is the declaration's claim, not a checked
    /// fact: the emitted program initializes the slot to null and compares the
    /// observed nullness against the declared control, which refutes a
    /// disagreeing implementation for the observed run and establishes nothing
    /// beyond it.
    OutSlotWrittenByCallee(String),
    /// The named callee declares a `resource_stored(_, owner, nonnull)`
    /// parameter. The stored identity leaves this profile's accounting
    /// permanently; that the declared owner keeps, and eventually releases, it
    /// is a declared premise and is discharged nowhere in this repository.
    StoredCustodyUnderDeclaredOwner(String),
    /// The named callee declares `resource_released(_, nonnull)`. Contract spec
    /// §C04.2: "the trusted call contract discharges the resource obligation. A
    /// false declaration here can invalidate a safety conclusion." This profile
    /// checks that the caller supplied a live identity and stopped using it; it
    /// never checks that the C implementation destroys anything. Recorded
    /// separately from [`Self::DeclaredContract`] so a reader can see which
    /// callees a leak-freedom reading actually depends on.
    DeclaredRelease(String),
    /// The named callee declares `resource_return_reallocated(_, nullable)`.
    /// Both arms of contract spec §C04.3 are premised: that a null result left
    /// the incoming custody intact, and that a non-null result replaced it
    /// exactly once. The emitted program compares the observed nullness against
    /// the declared control, which refutes a disagreeing implementation for the
    /// observed run and establishes nothing beyond it.
    DeclaredReallocation(String),
}

/// Evidence a checked module publishes. Every field is re-derivable from the
/// program alone; [`recheck_extern_certificate`] does exactly that.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExternCallCertificate {
    pub entry: String,
    /// Admitted call sites, in trace order, as (callee, derived custody).
    pub calls: Vec<(String, Vec<ExternCustody>)>,
    /// Identities the adapters introduce, in trace order, from either
    /// acquisition source.
    pub acquisitions: Vec<ResourceId>,
    /// The subset of `acquisitions` established through a declared out slot
    /// rather than an owned return, in trace order.
    pub out_acquisitions: Vec<ResourceId>,
    /// Identities consumed by a call, in trace order.
    pub consumptions: Vec<ResourceId>,
    /// Identities a declared `resource_released` parameter discharged, in trace
    /// order. Kept apart from `consumptions` because the two declare different
    /// things: a consumption moves an obligation, a release discharges it, and
    /// contract spec §C04.2 warns that a false release invalidates a safety
    /// conclusion. A reader that treats them as one loses that distinction.
    pub releases: Vec<ResourceId>,
    /// `(replaced, established)` identities of every successful declared
    /// reallocation, in trace order. A reallocation that took its failure arm
    /// contributes nothing here: the subject was preserved, not replaced.
    pub reallocations: Vec<(ResourceId, ResourceId)>,
    /// `(child, owner)` identities whose custody left the caller's ledger
    /// beneath a declared owner, in trace order. The owner is still the
    /// caller's; what it does with the child is the premise recorded as
    /// [`ExternAssumption::StoredCustodyUnderDeclaredOwner`].
    pub stored: Vec<(ResourceId, ResourceId)>,
    /// Per acquisition, in trace order: the identity the adapter binds and the
    /// identities its error arm releases before the raise, in the order it
    /// releases them. An acquisition whose error arm abandons nothing because
    /// nothing else was live records an empty vector, so a reader can tell the
    /// two apart.
    pub error_dispositions: Vec<(ResourceId, Vec<ResourceId>)>,
    /// `(callee, value)` of every call whose scalar result was bound, in trace
    /// order (card `PIPE-SCALAR-RESULT`). Published beside the custody fields
    /// rather than folded into them, because it is the one field of this
    /// certificate that names no resource identity: a reader can see at a
    /// glance which calls moved data and which moved custody.
    pub scalar_bindings: Vec<(String, ValueId)>,
    /// `(value, literal)` of every scalar literal binding, in trace order (card
    /// `EXTERN-SCALAR-LITERAL`). Published beside `scalar_bindings` rather than
    /// merged into it: one names a call that produced a number and the other
    /// names a number written at the site, and a reader of the evidence must be
    /// able to tell those apart.
    pub literal_bindings: Vec<(ValueId, i32)>,
    /// `(value, bits)` of every float literal binding, in trace order (card
    /// `W48-QR-FLOATING-POINT`). Empty for every program before the card.
    pub float_literal_bindings: Vec<(ValueId, i128)>,
    /// The comparison effects the value statements used (card
    /// `W48-QR-FLOATING-POINT`), each admitted by the entry's row.
    pub float_effects: crate::scalar_types::FloatEffects,
    /// `(value, outcome)` of every scalar binding whose site declared what the
    /// call returns (card `STD-IO-COMPOSE`), in trace order. A binding absent
    /// here carries no modeled number, which is what makes it unreadable as
    /// data.
    pub declared_outcomes: Vec<(ValueId, i128)>,
    /// The §G02t type of every bound value, by identity (card
    /// `W48-QR-SCALAR-TYPES-R3`): a call's result type, `i32` for a literal
    /// binding, and a value statement's result type.
    pub value_types: BTreeMap<ValueId, ScalarType>,
    /// Every value statement, in trace order (card `W48-QR-SCALAR-TYPES-R3`),
    /// with what the checker decided about it.
    pub computations: Vec<ValueComputation>,
    /// The recorded target this certificate depends on: present exactly when a
    /// value statement or a declared outcome is checked at `isize`/`usize`,
    /// whose range only the builder's record supplies. A named assumption.
    pub target_assumption: Option<RecordedTarget>,
    /// Union of the derived rows of every call site, error-arm dispositions
    /// included: a release the error arm performs is a call this module makes.
    pub effects: EffectRow,
    pub assumptions: Vec<ExternAssumption>,
    /// The identities live on this module's **normal** path after each event, in
    /// trace order and acquisition order, one entry per event of
    /// [`ExternProgram::events`] (card `STD-FMT-BORROW`).
    ///
    /// This is the checker's own ledger, published rather than recomputed. It
    /// exists so that a *composing* route can decide whether a name is live at a
    /// step of its own program without re-implementing one custody rule: a
    /// composed borrowed render reads this list and nothing else. Every entry is
    /// the state after the event at the same index, so a reader that wants the
    /// state *before* event `i` reads entry `i - 1` and the empty ledger at
    /// `i = 0`.
    ///
    /// It says nothing about an error arm: a raise leaves this trace, and the
    /// identities an arm disposes of are `error_dispositions`.
    pub live_after: Vec<Vec<ResourceId>>,
}

/// What the checker decided about one value statement (card
/// `W48-QR-SCALAR-TYPES-R3`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ValueComputation {
    pub value: ValueId,
    /// The result's type (`bool` for a comparison).
    pub ty: ScalarType,
    /// The operation's own type — its operands' — which is the result's for
    /// everything but a comparison and a conversion.
    pub operand_ty: ScalarType,
    /// The number, when every operand is known at check time.
    pub known: Option<i128>,
    /// Whether a run can raise `ArithmeticError` here: an operand is known only
    /// at run time and the operation can fault. Admitted only under the row's
    /// `throws<ArithmeticError>`.
    pub raises: bool,
}

/// The §G02t checking of the value namespace (card `W48-QR-SCALAR-TYPES-R3`):
/// each value's type and, where the source determined it, its number. The
/// decision itself is [`crate::scalar_values::Decider`], shared with the task
/// route since card `W48-QR-SCALAR-TYPES-R4`.
#[derive(Default)]
struct ValueTyping {
    types: BTreeMap<ValueId, ScalarType>,
    known: BTreeMap<ValueId, i128>,
    decider: scalar_values::Decider,
}

impl ValueTyping {
    /// The width-bearing range check of `ty` (see [`scalar_values::Decider::range`]).
    fn range(
        &mut self,
        ty: ScalarType,
        target: Option<&RecordedTarget>,
    ) -> Result<(i128, i128), ValueRefusal> {
        self.decider.range(ty, target)
    }

    /// One operand as the shared decision sees it, or `None` for a value that
    /// was never bound.
    fn resolve(&self, operand: ValueOperand) -> Option<scalar_values::Operand> {
        Some(match operand {
            ValueOperand::Value(value) => scalar_values::Operand::Typed {
                ty: *self.types.get(&value)?,
                known: self.known.get(&value).copied(),
            },
            ValueOperand::Integer(literal) => scalar_values::Operand::Integer(literal),
            ValueOperand::Bool(value) => scalar_values::Operand::Bool(value),
            ValueOperand::Float(literal) => scalar_values::Operand::Float(literal),
        })
    }

    /// Check one value statement against the row's permission.
    fn compute(
        &mut self,
        statement: &ValueStatement,
        raises_permitted: bool,
        target: Option<&RecordedTarget>,
    ) -> Result<ValueComputation, Option<ValueRefusal>> {
        let operation = match statement.operation {
            ValueOperation::Binary(op, a, b) => scalar_values::Operation::Binary(
                op,
                self.resolve(a).ok_or(None)?,
                self.resolve(b).ok_or(None)?,
            ),
            ValueOperation::Compare(cmp, a, b) => scalar_values::Operation::Compare(
                cmp,
                self.resolve(a).ok_or(None)?,
                self.resolve(b).ok_or(None)?,
            ),
            ValueOperation::Negate(a) => {
                scalar_values::Operation::Negate(self.resolve(a).ok_or(None)?)
            }
            ValueOperation::Convert(a, to) => {
                scalar_values::Operation::Convert(self.resolve(a).ok_or(None)?, to)
            }
        };
        let decision = self
            .decider
            .decide(operation, raises_permitted, target)
            .map_err(Some)?;
        self.types.insert(statement.value, decision.ty);
        if let Some(known) = decision.known {
            self.known.insert(statement.value, known);
        }
        Ok(ValueComputation {
            value: statement.value,
            ty: decision.ty,
            operand_ty: decision.operand_ty,
            known: decision.known,
            raises: decision.raises,
        })
    }
}

/// Remove one identity from the acquisition-ordered live list, if present.
fn remove_live(live: &mut Vec<ResourceId>, resource: ResourceId) {
    if let Some(position) = live.iter().position(|candidate| *candidate == resource) {
        live.remove(position);
    }
}

/// Simulate one adapter's error arm over the identities live when the
/// acquisition is attempted.
///
/// The arm is admitted only when it reproduces this acquisition's established
/// cleanup edge exactly: the same identities, each released by one call to a
/// declared `resource_consumed` callee, in reverse acquisition order. Anything
/// left live when the raise is reached is [`ExternDiagnostic::AdapterAbandonsCustody`],
/// which is the v1 refusal with its meaning intact.
///
/// Returns the identities the arm released, in order, and the effects its calls
/// derive. On refusal it returns every diagnostic found rather than the first.
#[allow(clippy::too_many_arguments)]
fn check_error_arm(
    program: &ExternProgram,
    event: usize,
    live: &[ResourceId],
    values: &BTreeSet<ValueId>,
    releases: &[ExternRelease],
    diagnostics: &mut Vec<ExternDiagnostic>,
    effects: &mut BTreeSet<Effect>,
) -> Vec<ResourceId> {
    let mut remaining = live.to_vec();
    let mut released = Vec::new();
    for release in releases {
        let Some(callee) = program.callees.get(&release.callee) else {
            diagnostics.push(ExternDiagnostic::ReleaseUnknownCallee {
                event,
                name: release.callee.clone(),
            });
            continue;
        };
        effects.extend(derived_effects(callee).iter().cloned());
        if callee.acquires() {
            diagnostics.push(ExternDiagnostic::ReleaseAcquires {
                event,
                name: callee.name.clone(),
            });
            continue;
        }
        if callee
            .parameters
            .iter()
            .any(|parameter| matches!(parameter, CallParameter::Stored { .. }))
        {
            diagnostics.push(ExternDiagnostic::ReleaseStores {
                event,
                name: callee.name.clone(),
            });
            continue;
        }
        if release.arguments.len() != callee.parameters.len() {
            diagnostics.push(ExternDiagnostic::ReleaseArity {
                event,
                expected: callee.parameters.len(),
                actual: release.arguments.len(),
            });
            continue;
        }
        let mut discharged = Vec::new();
        let mut malformed = false;
        for (parameter, (argument, declared)) in
            release.arguments.iter().zip(&callee.parameters).enumerate()
        {
            let admitted = matches!(
                (argument, declared),
                (ExternArgument::Data(_), CallParameter::Data(_))
                    | (ExternArgument::Borrow(_), CallParameter::Borrowed(_))
                    | (ExternArgument::Consume(_), CallParameter::Consumed)
                    | (ExternArgument::Owner(_), CallParameter::Owner)
                    | (ExternArgument::Release(_), CallParameter::Released)
                    | (ExternArgument::Value(_), CallParameter::Data(_))
            );
            if !admitted {
                diagnostics.push(ExternDiagnostic::ReleaseArgumentForm {
                    event,
                    parameter,
                    expected: *declared,
                });
                malformed = true;
                continue;
            }
            match argument {
                // A disposition discharges by either departing mode the arm
                // admits. `resource_released` is the mode that says what the
                // cleanup edge means; `resource_consumed` remains admitted
                // because the caller-side obligation is discharged either way,
                // and the certificate records which one ran.
                ExternArgument::Consume(resource) | ExternArgument::Release(resource) => {
                    discharged.push(*resource);
                }
                // Every identity the arm merely reads must still be live there;
                // the error arm is a path, not a place where a dead binding
                // becomes readable again.
                ExternArgument::Borrow(resource) | ExternArgument::Owner(resource) => {
                    if !remaining.contains(resource) {
                        diagnostics.push(ExternDiagnostic::ReleaseNotLive {
                            event,
                            resource: *resource,
                        });
                        malformed = true;
                    }
                }
                // A scalar the arm reads must have been bound before the
                // acquisition it disposes of, exactly as an identity it reads
                // must be live: the arm is a path out of this body, not a
                // place where an unbound name becomes readable.
                ExternArgument::Value(value) => {
                    if !values.contains(value) {
                        diagnostics.push(ExternDiagnostic::ValueNotBound {
                            event,
                            parameter,
                            value: *value,
                        });
                        malformed = true;
                    }
                }
                ExternArgument::Data(_)
                | ExternArgument::Float { .. }
                | ExternArgument::Store(_)
                | ExternArgument::OutSlot
                | ExternArgument::Realloc(_) => {}
            }
        }
        if malformed {
            continue;
        }
        if discharged.len() != 1 {
            diagnostics.push(ExternDiagnostic::ReleaseNotSingular {
                event,
                name: callee.name.clone(),
                discharged: discharged.len(),
            });
            continue;
        }
        let resource = discharged[0];
        let Some(position) = remaining
            .iter()
            .position(|candidate| *candidate == resource)
        else {
            diagnostics.push(ExternDiagnostic::ReleaseNotLive { event, resource });
            continue;
        };
        remaining.remove(position);
        released.push(resource);
    }
    // The cleanup edge of `qcore_core::cleanup_lowering` releases the live set
    // in reverse acquisition order; a handler that states another order
    // describes a different program than the sequential lowering of the same
    // module, so the two are required to agree rather than reconciled.
    let expected = live.iter().rev().copied().collect::<Vec<_>>();
    if !released.is_empty() && released != expected[..released.len().min(expected.len())] {
        diagnostics.push(ExternDiagnostic::ReleaseOrder {
            event,
            expected,
            actual: released.clone(),
        });
    }
    for resource in remaining {
        diagnostics.push(ExternDiagnostic::AdapterAbandonsCustody { event, resource });
    }
    released
}

/// Check one lowered module's call sites. Returns a certificate or every
/// refusal found, so a caller reports the whole cascade rather than the first.
pub fn check_extern_program(
    program: &ExternProgram,
) -> Result<ExternCallCertificate, Vec<ExternDiagnostic>> {
    let mut diagnostics = Vec::new();
    let mut calls = Vec::new();
    let mut acquisitions = Vec::new();
    let mut out_acquisitions = Vec::new();
    let mut consumptions = Vec::new();
    let mut releases = Vec::new();
    let mut reallocations = Vec::new();
    let mut stored = Vec::new();
    let mut error_dispositions = Vec::new();
    let mut scalar_bindings = Vec::new();
    let mut literal_bindings = Vec::new();
    let mut float_literal_bindings = Vec::new();
    let mut declared_outcomes = Vec::new();
    // Acquisition order, because the established cleanup edge releases in
    // reverse acquisition order and the error arm is required to match it.
    let mut live = Vec::<ResourceId>::new();
    let mut introduced = BTreeSet::<ResourceId>::new();
    // The scalar namespace (card `PIPE-SCALAR-RESULT`). Deliberately not a
    // second ledger: a value is never removed, never ordered and never
    // released, because binding one imposes no obligation. It exists only so
    // that reading a value that was never bound is a refusal.
    let mut values = BTreeSet::<ValueId>::new();
    // Card `W48-QR-SCALAR-TYPES-R3`: the namespace's §G02t typing — each
    // value's type and, where the source determined it, its number.
    let mut typing = ValueTyping::default();
    // Card `W48-QR-FLOATING-POINT`: the row's comparison effects, never
    // inferred.
    typing.decider.permitted = program.float_effects;
    let mut computations = Vec::new();
    let target = program.target.as_ref();
    let mut effects = BTreeSet::new();
    // One ledger snapshot per event, taken after it (card `STD-FMT-BORROW`).
    // The per-event body is a labelled block rather than the loop body itself so
    // that *every* exit from it — the early ones included — reaches the snapshot
    // below: a missed snapshot would silently shift a later index's liveness.
    let mut live_after: Vec<Vec<ResourceId>> = Vec::with_capacity(program.events.len());
    for (index, event) in program.events.iter().enumerate() {
        'event: {
            let call = match event {
                ExternEvent::Call(call) => call,
                // Card `EXTERN-SCALAR-LITERAL`. A literal binding names no callee,
                // so it reaches none of the admission, bridging, custody or effect
                // machinery below: the only thing it can do wrong is reuse a value
                // identity, which is the same refusal a bound call's result gets.
                ExternEvent::Bind(literal) => {
                    if !values.insert(literal.value) {
                        diagnostics.push(ExternDiagnostic::DuplicateValueBinding {
                            event: index,
                            value: literal.value,
                        });
                    }
                    literal_bindings.push((literal.value, literal.literal));
                    typing.types.insert(literal.value, ScalarType::I32);
                    typing
                        .known
                        .insert(literal.value, i128::from(literal.literal));
                    break 'event;
                }
                // Card `W48-QR-FLOATING-POINT`: a float literal binding, an
                // `f64` known at check time; like an integer one it touches
                // no ledger.
                ExternEvent::BindFloat(binding) => {
                    if !values.insert(binding.value) {
                        diagnostics.push(ExternDiagnostic::DuplicateValueBinding {
                            event: index,
                            value: binding.value,
                        });
                    }
                    float_literal_bindings.push((binding.value, binding.bits));
                    typing.types.insert(binding.value, ScalarType::F64);
                    typing.known.insert(binding.value, binding.bits);
                    break 'event;
                }
                // Card `W48-QR-SCALAR-TYPES-R3`: a value statement. Like a
                // literal binding it names no callee and touches no ledger;
                // what it can do wrong is §G02t's, decided by `ValueTyping`.
                ExternEvent::Compute(statement) => {
                    if !values.insert(statement.value) {
                        diagnostics.push(ExternDiagnostic::DuplicateValueBinding {
                            event: index,
                            value: statement.value,
                        });
                        break 'event;
                    }
                    match typing.compute(statement, program.raises, target) {
                        Ok(computation) => computations.push(computation),
                        Err(Some(refusal)) => {
                            diagnostics.push(ExternDiagnostic::Value {
                                event: index,
                                refusal,
                            });
                        }
                        Err(None) => {
                            let operands = match statement.operation {
                                ValueOperation::Binary(_, a, b)
                                | ValueOperation::Compare(_, a, b) => vec![a, b],
                                ValueOperation::Negate(a) | ValueOperation::Convert(a, _) => {
                                    vec![a]
                                }
                            };
                            for (parameter, operand) in operands.into_iter().enumerate() {
                                if let ValueOperand::Value(value) = operand
                                    && !typing.types.contains_key(&value)
                                {
                                    diagnostics.push(ExternDiagnostic::ValueNotBound {
                                        event: index,
                                        parameter,
                                        value,
                                    });
                                }
                            }
                        }
                    }
                    break 'event;
                }
                ExternEvent::Return => break 'event,
            };
            let Some(callee) = program.callees.get(&call.callee) else {
                diagnostics.push(ExternDiagnostic::UnknownCallee {
                    event: index,
                    name: call.callee.clone(),
                });
                break 'event;
            };
            effects.extend(derived_effects(callee).iter().cloned());
            if call.arguments.len() != callee.parameters.len() {
                diagnostics.push(ExternDiagnostic::ArgumentArity {
                    event: index,
                    expected: callee.parameters.len(),
                    actual: call.arguments.len(),
                });
                break 'event;
            }
            let mut consumed_here = Vec::new();
            let mut released_here = Vec::new();
            let mut stored_here = Vec::new();
            let mut realloc_here = None;
            for (parameter, (argument, declared)) in
                call.arguments.iter().zip(&callee.parameters).enumerate()
            {
                let admitted = matches!(
                    (argument, declared),
                    (ExternArgument::Data(_), CallParameter::Data(_))
                    | (ExternArgument::Float { .. }, CallParameter::Data(_))
                    | (ExternArgument::Borrow(_), CallParameter::Borrowed(_))
                    | (ExternArgument::Consume(_), CallParameter::Consumed)
                    | (ExternArgument::Store(_), CallParameter::Stored { .. })
                    | (ExternArgument::Owner(_), CallParameter::Owner)
                    | (ExternArgument::OutSlot, CallParameter::OutSlot)
                    | (ExternArgument::Release(_), CallParameter::Released)
                    | (ExternArgument::Realloc(_), CallParameter::ReallocSubject)
                    // Card `PIPE-SCALAR-RESULT`: a bound scalar is admitted at
                    // a declared `Data` parameter and at no other mode, so the
                    // value namespace cannot reach a custody position.
                    | (ExternArgument::Value(_), CallParameter::Data(_))
                );
                if !admitted {
                    diagnostics.push(ExternDiagnostic::ArgumentForm {
                        event: index,
                        parameter,
                        expected: *declared,
                    });
                    continue;
                }
                match (argument, declared) {
                    (ExternArgument::Value(value), CallParameter::Data(abi)) => {
                        if !values.contains(value) {
                            diagnostics.push(ExternDiagnostic::ValueNotBound {
                                event: index,
                                parameter,
                                value: *value,
                            });
                        } else if let (Some(declared), Some(supplied)) =
                            (abi.scalar(), typing.types.get(value).copied())
                            && declared != supplied
                        {
                            // Card `W48-QR-SCALAR-TYPES-R3`: no implicit
                            // conversion at a declared parameter either.
                            diagnostics.push(ExternDiagnostic::Value {
                                event: index,
                                refusal: ValueRefusal::ArgumentType {
                                    parameter,
                                    declared,
                                    supplied,
                                },
                            });
                        }
                    }
                    // Card `W48-QR-SCALAR-TYPES-R3`: a literal at a declared
                    // scalar parameter is range-checked against that
                    // parameter's type — never truncated by the C call. The
                    // width of `isize`/`usize` is the recorded target's; with
                    // none, the literal is passed as it always was.
                    // Card `W48-QR-FLOATING-POINT`: an integer literal is never
                    // a float, and a float literal is a value of exactly the
                    // parameter's float type.
                    (ExternArgument::Data(_), CallParameter::Data(abi))
                        if abi.scalar().is_some_and(ScalarType::is_float) =>
                    {
                        diagnostics.push(ExternDiagnostic::Value {
                            event: index,
                            refusal: ValueRefusal::LiteralKind {
                                ty: abi.scalar().expect("a float parameter"),
                                float: false,
                            },
                        });
                    }
                    (ExternArgument::Float { ty, bits }, CallParameter::Data(abi)) => {
                        match abi.scalar() {
                            Some(declared) if declared == *ty && ty.contains(*bits, None) => {}
                            Some(declared) if declared.is_float() => {
                                diagnostics.push(ExternDiagnostic::Value {
                                    event: index,
                                    refusal: ValueRefusal::ArgumentType {
                                        parameter,
                                        declared,
                                        supplied: *ty,
                                    },
                                });
                            }
                            Some(declared) => diagnostics.push(ExternDiagnostic::Value {
                                event: index,
                                refusal: ValueRefusal::LiteralKind {
                                    ty: declared,
                                    float: true,
                                },
                            }),
                            None => diagnostics.push(ExternDiagnostic::ArgumentForm {
                                event: index,
                                parameter,
                                expected: *declared,
                            }),
                        }
                    }
                    (ExternArgument::Data(literal), CallParameter::Data(abi)) => {
                        if let Some(ty) = abi.scalar()
                            && !(ty.is_target_width() && target.is_none())
                        {
                            let literal = i128::from(*literal);
                            if let Err(refusal) =
                                typing.range(ty, target).and_then(|(low, high)| {
                                    if low <= literal && literal <= high {
                                        Ok(())
                                    } else {
                                        Err(ValueRefusal::LiteralOutOfRange { literal, ty })
                                    }
                                })
                            {
                                diagnostics.push(ExternDiagnostic::Value {
                                    event: index,
                                    refusal,
                                });
                            }
                        }
                    }
                    (ExternArgument::Value(value), _) => {
                        if !values.contains(value) {
                            diagnostics.push(ExternDiagnostic::ValueNotBound {
                                event: index,
                                parameter,
                                value: *value,
                            });
                        }
                    }
                    (ExternArgument::Consume(resource), _) => consumed_here.push(*resource),
                    (ExternArgument::Release(resource), _) => released_here.push(*resource),
                    (ExternArgument::Realloc(resource), _) => realloc_here = Some(*resource),
                    (ExternArgument::Store(child), CallParameter::Stored { owner }) => {
                        // The declared owner is a position in the same call, so the
                        // owner identity is read from the argument the declaration
                        // designates rather than guessed. An argument-form refusal
                        // there has already been reported, so nothing is inferred.
                        let Some(ExternArgument::Owner(owner)) = call.arguments.get(*owner) else {
                            continue;
                        };
                        if child == owner {
                            diagnostics.push(ExternDiagnostic::StoredUnderItself {
                                event: index,
                                resource: *child,
                            });
                            continue;
                        }
                        stored_here.push((*child, *owner));
                    }
                    _ => {}
                }
            }
            let acquires = callee.acquires();
            match (&call.adapter, acquires) {
                (None, true) => {
                    diagnostics.push(ExternDiagnostic::AdapterMissing { event: index });
                }
                (Some(_), false) => {
                    diagnostics.push(ExternDiagnostic::AdapterUnexpected { event: index });
                }
                (Some(adapter), true) => {
                    if !program.errors.contains(&adapter.error) {
                        diagnostics.push(ExternDiagnostic::AdapterUnknownError {
                            event: index,
                            error: adapter.error.clone(),
                        });
                    }
                    // Adapter totality with a bounded error arm: the null path
                    // takes the typed error exit, and that exit is admitted only
                    // where every other live identity is explicitly dispositioned
                    // first. The departures of *this* call have already left the
                    // ledger below-order — they are recorded after the adapter — so
                    // the arm sees exactly the identities the cleanup edge would.
                    // A declaration says what holds on its *normal* exit only, so an
                    // acquiring call that also carries departing custody leaves that
                    // custody's status undecided on the failure arm: the arm can
                    // neither release it (the callee may already have) nor leave it
                    // (the callee may not have). That shape is refused by name
                    // rather than resolved by assumption; it was already refused in
                    // v1, as abandonment, and stays refused here.
                    //
                    // Card `EXTERN-MODES` names the one exception, and it is an
                    // exception because the declaration stops being silent: contract
                    // spec §C04.3 fixes both arms of a reallocation, so the subject
                    // is *known* to survive a null result. It therefore stays on the
                    // arm's live list and the arm must dispose of it like any other
                    // live identity — which is the same rule, not a relaxation of
                    // it. A released argument is not the exception: `resource_released`
                    // constrains the normal exit only, exactly as consumption does.
                    let departing = consumed_here
                        .iter()
                        .copied()
                        .chain(released_here.iter().copied())
                        .chain(stored_here.iter().map(|(child, _)| *child))
                        .collect::<Vec<_>>();
                    let mut arm_live = live.clone();
                    for resource in &departing {
                        if let Some(position) =
                            arm_live.iter().position(|candidate| candidate == resource)
                        {
                            arm_live.remove(position);
                            diagnostics.push(ExternDiagnostic::AdapterDepartingArgument {
                                event: index,
                                resource: *resource,
                            });
                        }
                    }
                    let released = check_error_arm(
                        program,
                        index,
                        &arm_live,
                        &values,
                        &adapter.releases,
                        &mut diagnostics,
                        &mut effects,
                    );
                    error_dispositions.push((adapter.resource, released));
                    if !introduced.insert(adapter.resource) {
                        diagnostics.push(ExternDiagnostic::DuplicateAcquisition {
                            event: index,
                            resource: adapter.resource,
                        });
                    }
                }
                (None, false) => {}
            }
            // Card `PIPE-SCALAR-RESULT`. Checked after the adapter and before any
            // ledger movement, because a scalar binding is not a ledger event: it
            // adds a name to a namespace this checker never removes from and never
            // compares against `live`.
            if let Some(binding) = call.binding {
                let value = binding.value;
                if callee.acquires() {
                    diagnostics.push(ExternDiagnostic::ResultBindingOnAcquisition {
                        event: index,
                        name: callee.name.clone(),
                    });
                } else if binds_scalar(callee) {
                    if !values.insert(value) {
                        diagnostics.push(ExternDiagnostic::DuplicateValueBinding {
                            event: index,
                            value,
                        });
                    }
                    scalar_bindings.push((callee.name.clone(), value));
                    // Card `W48-QR-SCALAR-TYPES-R3`: the value has the declared
                    // result's type, and a declared outcome is a value of it —
                    // range-checked, the width of `isize`/`usize` read from the
                    // recorded target. With no target the outcome of a
                    // target-width result is carried unchecked, as before the
                    // card, and any statement that would compute with it
                    // refuses for want of the width.
                    let ty = callee
                        .result_abi
                        .scalar()
                        .expect("a scalar binding has a scalar result");
                    typing.types.insert(value, ty);
                    if let Some(outcome) = binding.outcome {
                        declared_outcomes.push((value, outcome));
                        let checked = if ty.is_target_width() && target.is_none() {
                            Ok(outcome)
                        } else if ty.is_float() {
                            // Card `W48-QR-FLOATING-POINT`: a float outcome is
                            // a bit pattern of the result's type.
                            if ty.contains(outcome, None) {
                                Ok(outcome)
                            } else {
                                Err(ValueRefusal::OutcomeOutOfRange {
                                    callee: callee.name.clone(),
                                    outcome,
                                    ty,
                                })
                            }
                        } else {
                            typing.range(ty, target).and_then(|(low, high)| {
                                if low <= outcome && outcome <= high {
                                    Ok(outcome)
                                } else {
                                    Err(ValueRefusal::OutcomeOutOfRange {
                                        callee: callee.name.clone(),
                                        outcome,
                                        ty,
                                    })
                                }
                            })
                        };
                        match checked {
                            Ok(outcome) => {
                                typing.known.insert(value, outcome);
                            }
                            Err(refusal) => diagnostics.push(ExternDiagnostic::Value {
                                event: index,
                                refusal,
                            }),
                        }
                    }
                } else {
                    diagnostics.push(ExternDiagnostic::ResultNotScalar {
                        event: index,
                        name: callee.name.clone(),
                        result: callee.result_abi,
                    });
                }
            }
            for resource in &consumed_here {
                remove_live(&mut live, *resource);
                consumptions.push(*resource);
            }
            // A released identity leaves the caller's ledger exactly as a consumed
            // one does; the difference is the declaration, recorded separately.
            for resource in &released_here {
                remove_live(&mut live, *resource);
                releases.push(*resource);
            }
            // A stored identity leaves the caller's ledger exactly as a consumed
            // one does. What distinguishes it is where it went, which is recorded
            // and premised rather than checked.
            for (child, owner) in &stored_here {
                remove_live(&mut live, *child);
                stored.push((*child, *owner));
            }
            if let Some(adapter) = &call.adapter
                && acquires
            {
                acquisitions.push(adapter.resource);
                if callee.out_parameter.is_some() {
                    out_acquisitions.push(adapter.resource);
                }
                // A failed acquisition never makes the identity live; the checked
                // trace keeps the static identity so the caller's passes still see
                // one binding, and the reference/native runs take the error arm.
                if adapter.succeeds {
                    // A successful reallocation replaces the subject exactly once
                    // (contract spec §C04.3): the old identity stops being live at
                    // the same step the new one starts. On the failure arm neither
                    // happens, which is why the subject was still on the arm's live
                    // list and had to be dispositioned there.
                    if let Some(subject) = realloc_here {
                        remove_live(&mut live, subject);
                        reallocations.push((subject, adapter.resource));
                    }
                    live.push(adapter.resource);
                }
            }
            calls.push((callee.name.clone(), derived_custody(callee)));
        }
        live_after.push(live.clone());
    }
    debug_assert_eq!(
        live_after.len(),
        program.events.len(),
        "every event contributes exactly one ledger snapshot"
    );
    if !diagnostics.is_empty() {
        return Err(diagnostics);
    }
    let mut assumptions = Vec::new();
    for (name, callee) in &program.callees {
        // A total match on the callee's trust ([`assumption_for_callee`]), so
        // there is no arm that contributes no premise: a contracted callee is
        // carried as its declared contract, an uncontracted one as unchecked
        // linkage — sharpened to name the artifact when a library manifest
        // declares it (card `UNSAFE-LINKAGE-ARTIFACT`). Card `UNSAFE-LINKAGE`;
        // the Lean theorem `assumption_totality` states the same sentence over
        // the model, and `QCore.NativeLibrary.manifest_assumption_totality`
        // states it again with the manifest in the model.
        // Card `NATIVE-LIBRARY-CONTRACT-SLICE` adds the second element of this
        // inventory and removes none: a contracted callee a manifest binds
        // carries its declared-contract premise **and** the artifact premise,
        // stacked. `QCore.NativeLibraryContract.the_contract_premise_is_never_replaced`
        // states that the first is still there.
        assumptions.extend(assumptions_for_callee(callee));
        if callee.trust == ExternTrust::UncheckedLinkage {
            // An unchecked callee has no contract, so none of the
            // contract-shaped premises below can apply to it; its admitted
            // surface has no pointer in any position.
            continue;
        }
        if callee.out_parameter.is_some() {
            assumptions.push(ExternAssumption::OutSlotWrittenByCallee(name.clone()));
        }
        if callee
            .parameters
            .iter()
            .any(|parameter| matches!(parameter, CallParameter::Stored { .. }))
        {
            assumptions.push(ExternAssumption::StoredCustodyUnderDeclaredOwner(
                name.clone(),
            ));
        }
        if callee
            .parameters
            .iter()
            .any(|parameter| matches!(parameter, CallParameter::Released))
        {
            assumptions.push(ExternAssumption::DeclaredRelease(name.clone()));
        }
        if callee.realloc_parameter.is_some() {
            assumptions.push(ExternAssumption::DeclaredReallocation(name.clone()));
        }
    }
    // Card `NATIVE-LIBRARY-SOURCE-BINDING`: one premise per resolved
    // `extern library` declaration. It is derived from the program's own binding
    // list and from nothing else, so a recheck reproduces it; an unbound module
    // contributes none and its certificate is unchanged.
    for binding in &program.libraries {
        assumptions.push(ExternAssumption::SourceBoundLibrary(binding.clone()));
    }
    assumptions.push(ExternAssumption::NoExtentTransport);
    assumptions.push(ExternAssumption::LinkageSuppliedByBuilder);
    assumptions.sort();
    Ok(ExternCallCertificate {
        entry: program.entry.clone(),
        calls,
        acquisitions,
        out_acquisitions,
        consumptions,
        releases,
        reallocations,
        stored,
        error_dispositions,
        scalar_bindings,
        literal_bindings,
        float_literal_bindings,
        float_effects: typing.decider.used,
        declared_outcomes,
        value_types: typing.types,
        computations,
        target_assumption: if typing.decider.used_target {
            program.target.clone()
        } else {
            None
        },
        effects: EffectRow::new(effects),
        assumptions,
        live_after,
    })
}

/// Every unchecked-linkage premise the program's own callee inventory demands,
/// derived from the program alone (card `UNSAFE-LINKAGE`).
///
/// This is the *demand* side of the assumption discipline, computed
/// independently of any certificate, so a publication boundary can compare a
/// published inventory against it rather than believe one.
#[must_use]
pub fn required_unchecked_assumptions(program: &ExternProgram) -> Vec<ExternAssumption> {
    program
        .callees
        .iter()
        .filter(|(_, callee)| callee.trust == ExternTrust::UncheckedLinkage)
        .map(|(_, callee)| assumption_for_callee(callee))
        .collect()
}

/// Whether a published certificate carries an unchecked-linkage premise for
/// every uncontracted callee its own program declares.
///
/// True by construction of [`check_extern_program`], and asserted anyway at the
/// driver's publication boundary: a certificate that lost a premise between
/// derivation and publication is refused rather than published. Asserting an
/// invariant that cannot fail is the point — the day it can, it is caught here
/// instead of in a `Safe` that stopped mentioning what it rests on.
#[must_use]
pub fn certificate_carries_unchecked_linkage(
    program: &ExternProgram,
    certificate: &ExternCallCertificate,
) -> bool {
    required_unchecked_assumptions(program)
        .iter()
        .all(|required| certificate.assumptions.contains(required))
}

/// Every **artifact** premise the program's own callee inventory demands for a
/// manifest-bound contracted callee, derived from the program alone (card
/// `NATIVE-LIBRARY-CONTRACT-SLICE`).
///
/// The contracted twin of [`required_unchecked_assumptions`], and separate from
/// it for the same reason the premises are separate: a certificate could carry
/// every declared-contract premise and still have lost the statement of *which
/// build* each contract was read against.
#[must_use]
pub fn required_residence_assumptions(program: &ExternProgram) -> Vec<ExternAssumption> {
    program
        .callees
        .values()
        .filter_map(residence_assumption_for_callee)
        .collect()
}

/// Whether a published certificate carries the artifact premise of every
/// manifest-bound contracted callee its own program declares, **and** the
/// declared-contract premise beside it.
///
/// The second half is the load-bearing one: the premises stack, so a
/// certificate that published the artifact premise *instead of* the contract
/// premise would be refused here rather than read as an equivalent statement.
#[must_use]
pub fn certificate_carries_residence_premises(
    program: &ExternProgram,
    certificate: &ExternCallCertificate,
) -> bool {
    program.callees.values().all(|callee| {
        let Some(required) = residence_assumption_for_callee(callee) else {
            return true;
        };
        certificate.assumptions.contains(&required)
            && certificate
                .assumptions
                .contains(&ExternAssumption::DeclaredContract(callee.name.clone()))
    })
}

/// Every **source-binding** premise the program's own binding list demands,
/// derived from the program alone (card `NATIVE-LIBRARY-SOURCE-BINDING`).
///
/// Separate from the two inventories above for the same reason they are separate
/// from each other: a certificate could carry every callee premise and still have
/// lost the statement of which build this *file* declared its extern surface to
/// come from. An unbound module demands nothing here.
#[must_use]
pub fn required_source_binding_assumptions(program: &ExternProgram) -> Vec<ExternAssumption> {
    program
        .libraries
        .iter()
        .cloned()
        .map(ExternAssumption::SourceBoundLibrary)
        .collect()
}

/// Whether a published certificate carries a source-binding premise for every
/// `extern library` declaration its own program records.
///
/// True by construction of [`check_extern_program`], and asserted anyway at the
/// driver's publication boundary, on the [`certificate_carries_unchecked_linkage`]
/// model: the day it can fail, it is caught here rather than inside a `Safe`
/// that stopped saying which build it was resolved against.
#[must_use]
pub fn certificate_carries_source_bindings(
    program: &ExternProgram,
    certificate: &ExternCallCertificate,
) -> bool {
    required_source_binding_assumptions(program)
        .iter()
        .all(|required| certificate.assumptions.contains(required))
}

/// Why an independent recheck refused a published certificate.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ExternRecheckFailure {
    /// The program the certificate claims to describe does not check at all.
    Refused(Vec<ExternDiagnostic>),
    /// The program checks, but to different evidence than was published.
    Mismatch,
}

impl fmt::Display for ExternRecheckFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Refused(diagnostics) => write!(
                f,
                "the recorded program does not check: {}",
                diagnostics
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join("; ")
            ),
            Self::Mismatch => {
                f.write_str("the certificate differs from an independent rederivation")
            }
        }
    }
}

impl std::error::Error for ExternRecheckFailure {}

/// Rederive the certificate from the program and compare it field for field.
/// A publisher that cannot reproduce its own evidence has published nothing.
pub fn recheck_extern_certificate(
    program: &ExternProgram,
    certificate: &ExternCallCertificate,
) -> Result<(), ExternRecheckFailure> {
    let rederived = check_extern_program(program).map_err(ExternRecheckFailure::Refused)?;
    if &rederived == certificate {
        Ok(())
    } else {
        Err(ExternRecheckFailure::Mismatch)
    }
}

#[cfg(test)]
#[path = "../tests/unit/extern_call.rs"]
mod tests;
