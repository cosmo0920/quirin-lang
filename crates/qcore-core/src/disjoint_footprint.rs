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

//! `DisjointFootprintsV1` — derived access footprints and alias identity.
//!
//! [`crate::concurrency`] and [`crate::mutex_invariant`] both take an
//! [`AccessFootprint`] on trust: an interval a caller *declares*, recorded on
//! the certificate as [`ConcurrencyAssumption::DeclaredFootprintShape`]. This
//! module is the bounded prototype of the profile that *derives* those
//! intervals instead, so that a later increment can discharge that premise for
//! the shapes it covers.
//!
//! What is decided here:
//!
//! * **Interval bounds and disjointness against one allocation generation.**
//!   A slice names an allocation, the generation it was derived under, and an
//!   element range; the checker computes the byte interval itself, refuses a
//!   range that leaves the declared extent, and decides overlap by interval
//!   arithmetic. A caller may supply the byte interval too, in which case it is
//!   compared, never believed.
//! * **Stale aliases.** Reallocation advances the generation and destruction
//!   ends it; every handle of a retired generation is retired with it and every
//!   later use of one is refused.
//! * **Record fields and bit-fields.** A field footprint exists only under an
//!   explicit, internally validated layout contract. Two fields sharing one
//!   bit-field storage unit are one C11 memory location and are **never**
//!   disjoint, even when their bit ranges are.
//! * **Descriptor aliases.** Two descriptors that name one protocol cell are
//!   never disjoint whatever their byte footprints are, and a memory footprint
//!   and a descriptor footprint are *incomparable*: the checker refuses the
//!   question rather than answering "disjoint".
//!
//! What is deliberately **not** decided, and refused rather than approximated:
//!
//! * disjointness across two allocation identities — that needs an allocator
//!   model this profile does not have, so it is a refusal
//!   ([`FootprintDiagnostic::CrossAllocationDisjointnessUnsupported`]), not a
//!   free "distinct objects do not alias", unless both were declared fresh
//!   (card `CG18-FOOTPRINT-CONNECTION`, below), which the certificate names;
//! * unions, flexible array members, variable-length arrays, computed indices,
//!   pointer arithmetic outside a declared extent, nested anonymous aggregates,
//!   type-punned aliases, overlapping copies and atomic reclamation.
//!
//! What stays a premise is named on the certificate: the declared element size
//! and count are the real object extent
//! ([`FootprintAssumption::AllocationExtentDeclared`]), a layout contract is the
//! target ABI's real layout ([`FootprintAssumption::DeclaredRecordLayout`]), and
//! a declared protocol cell is really the shared logical state
//! ([`FootprintAssumption::DeclaredProtocolCellIdentity`]) — stamped for a
//! [`FootprintEvent::DeclareDescriptorAlias`] and **not** for a
//! [`FootprintEvent::DeriveDescriptorAlias`], whose cell a deciding model
//! derived (card `CT42-CELL-ROUTE`).
//! [`FootprintAssumption::DeclaredDisjointness`] is the premise this profile
//! discharges: the checker never emits it and
//! [`recheck_disjoint_footprint_certificate`] refuses any certificate carrying
//! it.
//!
//! This is a model-level module. The source routes that reach it are
//! `source.socket.v1`'s `disjoint NAME NAME;` question, through
//! [`crate::socket_footprint`] (card `CT42-CELL-ROUTE`), which hands this
//! checker a derived-alias projection and reports its `SharedProtocolCell`
//! (`Q664`) unedited, `source.bitfield-concurrency.v1`, and the custody
//! gateway's unordered field pairs (cards `CG13-UNION-AND-BITFIELD-PLACES` and
//! `CG18-FOOTPRINT-CONNECTION`, gateway rules 10w and 11k).
//!
//! # Card `CG18-FOOTPRINT-CONNECTION`
//!
//! Four additions, each a new event that no older trace contains, so every
//! older certificate is unchanged:
//!
//! * **Stated cross-allocation disjointness** (M21).
//!   [`FootprintEvent::DeclareFreshAllocation`] records that an allocator
//!   contract declares a generation fresh; two allocation identities are
//!   decided disjoint only when both live generations were, and the
//!   certificate then carries
//!   [`FootprintAssumption::DistinctAllocationsDisjoint`]. Every other pair
//!   keeps `Q662`.
//! * **Views** (M22). [`FootprintEvent::DeriveView`]: a character view needs
//!   no premise, an effective-type view carries
//!   [`FootprintAssumption::EffectiveTypeDeclared`], and a punned view is the
//!   excluded [`UnsupportedFootprint::TypePunnedAlias`].
//! * **Byte transfers** (M26). [`FootprintEvent::ByteCopy`] and
//!   [`FootprintEvent::ByteFill`] check the byte ranges of `memcpy`,
//!   `memmove` and `memset`; an overlapping `memcpy` stays the excluded
//!   [`UnsupportedFootprint::OverlappingCopy`], and every admitted transfer
//!   carries [`FootprintAssumption::ByteRepresentation`].
//! * **Mutex invariants** (W08) compose with derived footprints in
//!   [`crate::footprint_mutex`].
//!
//! No new diagnostic code: each refusal is a condition of an existing one
//! (`Q650`, `Q652`, `Q653`, `Q655`, `Q659`, `Q662`, `Q667`).

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

pub use crate::concurrency::AccessFootprint;

/// One allocation lifetime. Generation-indexed like a `SharedResourceId`:
/// never a reusable address.
pub type AllocationId = u32;
/// Which generation of an allocation a handle was derived under. A handle of a
/// retired generation is stale forever.
pub type Generation = u32;
/// One derived footprint. Minted once and never reused, retired handles
/// included.
pub type HandleId = u32;
/// A record type named by a layout contract.
pub type RecordId = u32;
/// One member of a record.
pub type FieldId = u32;
/// One OS/library descriptor: a handle, not a byte range.
pub type DescriptorId = u32;
/// The shared logical state two descriptor aliases can both advance.
pub type ProtocolCellId = u32;

/// The element shape of an allocation. `Record` requires a layout contract to
/// exist before the allocation is admitted, because its element size is the
/// contract's size.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ElementShape {
    /// A scalar or opaque byte block of this many bytes.
    Scalar { size: u32 },
    /// A record whose byte layout comes from a validated layout contract.
    Record { record: RecordId },
}

/// How one field occupies storage. The distinction is C11's memory-location
/// rule, not a naming convention: adjacent bit-fields of one unit are one
/// memory location and cannot be written independently.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum FieldStorage {
    /// The field owns its byte range outright.
    Addressable,
    /// The field occupies `bit_width` bits at `bit_offset` inside the storage
    /// unit `unit`. Every field of one unit must declare the same byte range.
    BitfieldUnit {
        unit: u32,
        bit_offset: u32,
        bit_width: u32,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct FieldLayout {
    pub field: FieldId,
    pub byte_offset: u32,
    pub byte_size: u32,
    pub storage: FieldStorage,
}

/// An explicit, whole-record layout declaration. There is no partial
/// admission: a contract that fails any structural rule is refused entirely and
/// every field derivation naming that record is refused with it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LayoutContract {
    pub record: RecordId,
    pub size: u32,
    pub fields: Vec<FieldLayout>,
}

/// Alias and layout shapes this profile refuses rather than approximates. Each
/// is a real event, so "the profile refuses it" is testable instead of the
/// model quietly lacking a way to say it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum UnsupportedFootprint {
    UnionMember,
    FlexibleArrayMember,
    VariableLengthArray,
    ComputedIndex,
    PointerArithmeticOutsideDeclaredExtent,
    NestedAnonymousAggregate,
    TypePunnedAlias,
    OverlappingCopy,
    CrossAllocationAliasProof,
    AtomicReclamation,
}

/// How a view reads the bytes of the handle it is derived from (card
/// `CG18-FOOTPRINT-CONNECTION`, gap entry M22). A view never changes the
/// bytes: it names the type through which they are read, and so the premise
/// the read rests on.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ViewKind {
    /// A character-typed lvalue. C11 6.5p7's last bullet permits it over an
    /// object of any effective type, so it carries no premise.
    Character,
    /// An lvalue whose type the caller states is compatible with the
    /// object's effective type (C11 6.5p6-7). Stamps
    /// [`FootprintAssumption::EffectiveTypeDeclared`].
    EffectiveType,
    /// Any other lvalue type: a type-punned alias. Refused as
    /// [`UnsupportedFootprint::TypePunnedAlias`] (`Q667`), never admitted.
    Punned,
}

/// One admitted `memcpy`/`memmove`/`memset` byte range (card
/// `CG18-FOOTPRINT-CONNECTION`, gap entry M26), recorded on the certificate.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ByteTransferJudgement {
    /// A copy of `bytes` bytes from the start of `source` to the start of
    /// `destination`. `may_overlap` is `memmove`'s licence; without it the
    /// two ranges were decided disjoint.
    Copy {
        event: usize,
        destination: HandleId,
        source: HandleId,
        bytes: u32,
        may_overlap: bool,
    },
    /// A fill of the first `bytes` bytes of `destination`.
    Fill {
        event: usize,
        destination: HandleId,
        bytes: u32,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FootprintEvent {
    /// Introduce generation `generation` of `alloc` with `count` elements.
    Allocate {
        alloc: AllocationId,
        generation: Generation,
        shape: ElementShape,
        count: u32,
    },
    /// Retire the live generation and introduce `generation` in its place.
    /// Every handle of the old generation becomes stale.
    Reallocate {
        alloc: AllocationId,
        generation: Generation,
        count: u32,
    },
    /// End the allocation. Its generation is retired and dead forever.
    Destroy {
        alloc: AllocationId,
    },
    /// Derive a byte interval from an element range. `declared_bytes`, when
    /// present, is *compared* against the computed interval.
    DeriveSlice {
        handle: HandleId,
        alloc: AllocationId,
        generation: Generation,
        elem_offset: u32,
        elem_count: u32,
        declared_bytes: Option<(u32, u32)>,
    },
    /// Derive a field footprint from a slice covering exactly one element of a
    /// record-shaped allocation.
    DeriveField {
        handle: HandleId,
        base: HandleId,
        field: FieldId,
    },
    /// Declare that `descriptor` advances the shared logical state `cell`.
    ///
    /// The cell is the caller's **say-so**, so the checker stamps the
    /// certificate with [`FootprintAssumption::DeclaredProtocolCellIdentity`].
    DeclareDescriptorAlias {
        handle: HandleId,
        descriptor: DescriptorId,
        cell: ProtocolCellId,
    },
    /// Record that `descriptor` advances the shared logical state `cell`, where
    /// the cell was **derived** by a model that decided the program's
    /// connection structure rather than declared by anyone (card
    /// `CT42-CELL-ROUTE`, wave 39; the one producer in this tree is
    /// [`crate::socket_footprint::footprint_program`], which runs only over a
    /// program [`crate::std_socket::check_socket_program`] admitted).
    ///
    /// Handled exactly as [`Self::DeclareDescriptorAlias`] is — the same
    /// handle, the same footprint, the same `protocol_cells` row, the same
    /// [`AssertDisjoint`](Self::AssertDisjoint) decision — with one difference
    /// that is the whole point of the variant: the checker stamps **no**
    /// `DeclaredProtocolCellIdentity` for it. That premise reads "a declared
    /// protocol cell really is the shared logical state, and no undeclared
    /// descriptor also advances it", and for a derived cell both halves have a
    /// ground outside anyone's declaration: the pairing is read off the
    /// transport model's own decided connection structure, and that model
    /// admits no external peer (`source.socket.v1`'s `Q085`,
    /// `SOCKET-EXTERNAL-PEER`) and enumerates every socket of the program. The
    /// T09 edit this variant is (`CT42-CELL-DERIVED-PREMISE`) therefore
    /// **narrows** the certificate's assumption set for derived cells instead
    /// of relabelling the premise; what remains is the deriving model's own
    /// admission, which is the route's evidence and not this checker's
    /// premise.
    DeriveDescriptorAlias {
        handle: HandleId,
        descriptor: DescriptorId,
        cell: ProtocolCellId,
    },
    /// The checked obligation. Both handles must be live, and the answer must
    /// be *decidable* in this profile.
    AssertDisjoint {
        left: HandleId,
        right: HandleId,
    },
    Unsupported {
        construct: UnsupportedFootprint,
    },
    /// Card `CG18-FOOTPRINT-CONNECTION` (M21): the live generation of `alloc`
    /// is an object an allocator contract declares fresh — C11 7.22.3p1's
    /// "a pointer to an object disjoint from any other object". Two
    /// allocations whose live generations are both declared fresh are
    /// decided disjoint under the named premise
    /// [`FootprintAssumption::DistinctAllocationsDisjoint`], stamped only when
    /// a judgement uses it; every other pair of allocation identities keeps
    /// the `Q662` refusal. A reallocation's new generation is not fresh until
    /// declared again.
    DeclareFreshAllocation {
        alloc: AllocationId,
    },
    /// Card `CG18-FOOTPRINT-CONNECTION` (M22): a view of `base`'s bytes
    /// through an lvalue of the given kind. The view has `base`'s footprint;
    /// no field is derived through it. A punned view is refused (`Q667`), and
    /// a base that is a bit-field unit or a descriptor is refused (`Q659`).
    DeriveView {
        handle: HandleId,
        base: HandleId,
        view: ViewKind,
    },
    /// Card `CG18-FOOTPRINT-CONNECTION` (M26): `memcpy` (`may_overlap`
    /// false) or `memmove` (true) of `bytes` bytes from the start of `source`
    /// to the start of `destination`. Both ranges must lie inside their
    /// handles (`Q653`); a `memcpy` whose ranges share a byte is the excluded
    /// [`UnsupportedFootprint::OverlappingCopy`] (`Q667`), and one across two
    /// allocations needs [`FootprintAssumption::DistinctAllocationsDisjoint`]
    /// (else `Q662`). Only a property that claims byte-level safety asks
    /// this; custody contracts for these calls are unchanged.
    ByteCopy {
        destination: HandleId,
        source: HandleId,
        bytes: u32,
        may_overlap: bool,
    },
    /// Card `CG18-FOOTPRINT-CONNECTION` (M26): `memset` of the first `bytes`
    /// bytes of `destination` (`Q653` when they leave it).
    ByteFill {
        destination: HandleId,
        bytes: u32,
    },
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FootprintProgram {
    /// Layout contracts available to this trace. Validated before any event
    /// runs; an invalid contract is refused whole.
    pub layout_contracts: Vec<LayoutContract>,
    pub events: Vec<FootprintEvent>,
}

/// A footprint the checker computed. Never a caller-supplied interval.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum DerivedFootprint {
    Memory {
        alloc: AllocationId,
        generation: Generation,
        byte_start: u32,
        byte_end: u32,
        /// `Some(unit)` when this range is a bit-field storage unit rather than
        /// a range of separate memory locations.
        unit: Option<u32>,
        /// The declared bit range inside the unit, kept for reporting. It never
        /// participates in the disjointness decision.
        bits: Option<(u32, u32)>,
    },
    Descriptor {
        descriptor: DescriptorId,
        cell: ProtocolCellId,
    },
}

/// The result of asking a derived footprint for the interval the bounded
/// concurrency checkers consume. Explicit rather than an `Option`, so a
/// bit-field unit cannot be silently exported as an ordinary interval.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FootprintExport {
    Interval(AccessFootprint),
    /// The bytes are one C11 memory location shared with the other fields of
    /// `unit`; exporting the interval alone would lose that.
    SharedMemoryLocation {
        unit: u32,
    },
    /// A descriptor has no byte interval.
    NotMemory,
}

impl DerivedFootprint {
    /// Project onto [`AccessFootprint`] for the bounded concurrency checkers.
    /// See [`FootprintExport`] for the two cases that do not project.
    pub const fn export(self) -> FootprintExport {
        match self {
            Self::Memory {
                alloc,
                byte_start,
                byte_end,
                unit: None,
                ..
            } => FootprintExport::Interval(AccessFootprint::new(alloc, byte_start, byte_end)),
            Self::Memory {
                unit: Some(unit), ..
            } => FootprintExport::SharedMemoryLocation { unit },
            Self::Descriptor { .. } => FootprintExport::NotMemory,
        }
    }
}

/// One admitted disjointness judgement, named on the certificate so a reader
/// can see which pairs were decided and under which generation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum DisjointnessJudgement {
    /// Decided by interval arithmetic inside one allocation generation.
    Memory {
        event: usize,
        left: HandleId,
        right: HandleId,
        alloc: AllocationId,
        generation: Generation,
    },
    /// Decided by protocol-cell identity, not by any byte range.
    Descriptor {
        event: usize,
        left: HandleId,
        right: HandleId,
        left_cell: ProtocolCellId,
        right_cell: ProtocolCellId,
    },
    /// Card `CG18-FOOTPRINT-CONNECTION` (M21): two distinct allocations whose
    /// live generations were both declared fresh. Decided only under
    /// [`FootprintAssumption::DistinctAllocationsDisjoint`], which the
    /// certificate then carries.
    DistinctAllocations {
        event: usize,
        left: HandleId,
        right: HandleId,
        left_alloc: AllocationId,
        right_alloc: AllocationId,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum FootprintProfile {
    DisjointFootprintsV1,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum FootprintAssumption {
    /// The declared element size and count are the real extent of the object.
    AllocationExtentDeclared,
    /// A used layout contract matches the target ABI's real record layout.
    DeclaredRecordLayout,
    /// A declared protocol cell really is the shared logical state.
    DeclaredProtocolCellIdentity,
    /// The premise this profile discharges. The checker never emits it and
    /// recheck refuses a certificate that carries it.
    DeclaredDisjointness,
    /// `P-DISTINCT-ALLOCATIONS-DISJOINT` (card `CG18-FOOTPRINT-CONNECTION`,
    /// M21): two allocations an allocator contract declared fresh are
    /// distinct objects while both are live (C11 7.22.3p1). Stamped only when
    /// an admitted judgement crossed two allocation identities.
    DistinctAllocationsDisjoint,
    /// `P-EFFECTIVE-TYPE` (M22): a view's type is compatible with the
    /// effective type of the bytes it reads (C11 6.5p6-7).
    EffectiveTypeDeclared,
    /// `P-BYTE-REPRESENTATION` (M26): the bytes a `memcpy`/`memmove`/`memset`
    /// writes are read back only through an lvalue for which they are a
    /// valid representation (C11 6.2.6.1p5, 6.5p6). The same premise the
    /// Borrowed `memset`/`memcpy` declarations and a character-typed store
    /// rest on (`W43-INDIRECT-BYTE-STORE-REPRESENTATION-PREMISE`).
    ByteRepresentation,
}

impl FootprintAssumption {
    /// The premise's published name.
    pub const fn premise_name(self) -> &'static str {
        match self {
            Self::AllocationExtentDeclared => "AllocationExtentDeclared",
            Self::DeclaredRecordLayout => "DeclaredRecordLayout",
            Self::DeclaredProtocolCellIdentity => "DeclaredProtocolCellIdentity",
            Self::DeclaredDisjointness => "DeclaredDisjointness",
            Self::DistinctAllocationsDisjoint => "P-DISTINCT-ALLOCATIONS-DISJOINT",
            Self::EffectiveTypeDeclared => "P-EFFECTIVE-TYPE",
            Self::ByteRepresentation => "P-BYTE-REPRESENTATION",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum FootprintDiagnostic {
    /// `Q650` family: a zero element size, a zero count, a zero-length slice or
    /// a byte extent that does not fit the interval representation.
    InvalidExtent {
        event: usize,
        alloc: AllocationId,
    },
    UnknownAllocation {
        event: usize,
        alloc: AllocationId,
    },
    /// A handle, or an event, naming a generation that is no longer live.
    StaleGeneration {
        event: usize,
        alloc: AllocationId,
        named: Generation,
        live: Option<Generation>,
    },
    SliceOutOfBounds {
        event: usize,
        alloc: AllocationId,
        elem_end: u64,
        count: u32,
    },
    /// Handle identities are never reused, retired ones included.
    IdentityNotFresh {
        event: usize,
        handle: HandleId,
    },
    UnknownHandle {
        event: usize,
        handle: HandleId,
    },
    LayoutContractMissing {
        event: usize,
        record: RecordId,
    },
    /// The contract failed a structural rule; the whole contract is refused.
    InconsistentLayoutContract {
        record: RecordId,
        reason: LayoutDefect,
    },
    UnknownField {
        event: usize,
        record: RecordId,
        field: FieldId,
    },
    /// A field derivation over a scalar allocation, or over a base that is not
    /// exactly one element.
    ElementShapeMismatch {
        event: usize,
        handle: HandleId,
    },
    OverlappingFootprints {
        event: usize,
        left: HandleId,
        right: HandleId,
    },
    /// Two fields of one bit-field storage unit are one memory location.
    SharedBitfieldUnit {
        event: usize,
        left: HandleId,
        right: HandleId,
        unit: u32,
    },
    /// Two distinct allocation identities: this profile decides disjointness
    /// only against one underlying generation identity.
    CrossAllocationDisjointnessUnsupported {
        event: usize,
        left: AllocationId,
        right: AllocationId,
    },
    /// A memory footprint and a descriptor footprint. Refusing the question is
    /// the point: byte disjointness says nothing about a protocol cell.
    IncomparableFootprintKinds {
        event: usize,
        left: HandleId,
        right: HandleId,
    },
    SharedProtocolCell {
        event: usize,
        left: HandleId,
        right: HandleId,
        cell: ProtocolCellId,
    },
    /// A supplied byte interval disagrees with the derived one.
    DerivedFootprintMismatch {
        event: usize,
        handle: HandleId,
        declared: (u32, u32),
        derived: (u32, u32),
    },
    DestroyedAllocation {
        event: usize,
        alloc: AllocationId,
    },
    UnsupportedFootprintConstruct {
        event: usize,
        construct: UnsupportedFootprint,
    },
}

/// Why a layout contract was refused. Enumerated so the refusal is testable
/// rather than a boolean.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum LayoutDefect {
    ZeroSize,
    NoFields,
    DuplicateRecord,
    DuplicateField,
    ZeroFieldSize,
    FieldOutsideRecord,
    AddressableFieldsOverlap,
    BitfieldUnitStorageDisagrees,
    BitfieldBitsOutsideStorage,
    BitfieldBitsOverlap,
    BitfieldUnitsOverlap,
}

impl FootprintDiagnostic {
    /// The named refusals of this profile occupy `Q650`-`Q667`. `Q600` and
    /// `Q610`-`Q623` belong to the bounded concurrency model and the checked
    /// mutex-invariant profile and are not reused here.
    pub const fn code(&self) -> &'static str {
        match self {
            Self::InvalidExtent { .. } => "Q650",
            Self::UnknownAllocation { .. } => "Q651",
            Self::StaleGeneration { .. } => "Q652",
            Self::SliceOutOfBounds { .. } => "Q653",
            Self::IdentityNotFresh { .. } => "Q654",
            Self::UnknownHandle { .. } => "Q655",
            Self::LayoutContractMissing { .. } => "Q656",
            Self::InconsistentLayoutContract { .. } => "Q657",
            Self::UnknownField { .. } => "Q658",
            Self::ElementShapeMismatch { .. } => "Q659",
            Self::OverlappingFootprints { .. } => "Q660",
            Self::SharedBitfieldUnit { .. } => "Q661",
            Self::CrossAllocationDisjointnessUnsupported { .. } => "Q662",
            Self::IncomparableFootprintKinds { .. } => "Q663",
            Self::SharedProtocolCell { .. } => "Q664",
            Self::DerivedFootprintMismatch { .. } => "Q665",
            Self::DestroyedAllocation { .. } => "Q666",
            Self::UnsupportedFootprintConstruct { .. } => "Q667",
        }
    }
}

impl fmt::Display for FootprintDiagnostic {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}: {self:?}", self.code())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FootprintCertificate {
    /// Always [`FootprintProfile::DisjointFootprintsV1`].
    pub profile: FootprintProfile,
    /// Never contains [`FootprintAssumption::DeclaredDisjointness`]; its
    /// absence is this profile's discharge claim.
    pub assumptions: BTreeSet<FootprintAssumption>,
    pub checked_events: usize,
    /// Every footprint the checker computed, live and retired alike.
    pub derived_footprints: BTreeMap<HandleId, DerivedFootprint>,
    pub admitted_disjointness: Vec<DisjointnessJudgement>,
    pub retired_generations: Vec<(AllocationId, Generation)>,
    pub live_allocations: Vec<(AllocationId, Generation)>,
    pub validated_layout_contracts: Vec<RecordId>,
    pub protocol_cells: BTreeMap<DescriptorId, ProtocolCellId>,
    /// Card `CG18-FOOTPRINT-CONNECTION`: every allocation generation declared
    /// fresh, in declaration order.
    pub fresh_allocations: Vec<(AllocationId, Generation)>,
    /// Every admitted view: `(view, base, kind)`.
    pub derived_views: Vec<(HandleId, HandleId, ViewKind)>,
    /// Every admitted byte transfer.
    pub byte_transfers: Vec<ByteTransferJudgement>,
}

impl FootprintCertificate {
    /// Assumption names as plain strings, so a free-form assumption map cannot
    /// smuggle the discharged premise past the typed set.
    pub fn assumption_names(&self) -> BTreeSet<&'static str> {
        self.assumptions
            .iter()
            .map(|assumption| assumption.premise_name())
            .collect()
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum FootprintRecheckError {
    /// Re-running the checker on the same program produced diagnostics.
    ProgramRejected(Vec<FootprintDiagnostic>),
    /// A field of the supplied certificate is not what re-derivation produced.
    FieldMismatch { field: &'static str },
    /// The supplied certificate names a premise this profile discharges.
    ForbiddenAssumption { assumption: FootprintAssumption },
}

// ---------------------------------------------------------------------------
// Layout contract validation. Total or reject: a defective contract is refused
// whole, and every field derivation naming that record is refused with it.
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug)]
struct UnitExtent {
    byte_offset: u32,
    byte_size: u32,
}

/// Validate one contract, returning its defects. An empty result means the
/// contract is admissible.
fn layout_defects(contract: &LayoutContract) -> BTreeSet<LayoutDefect> {
    let mut defects = BTreeSet::new();
    if contract.size == 0 {
        defects.insert(LayoutDefect::ZeroSize);
    }
    if contract.fields.is_empty() {
        defects.insert(LayoutDefect::NoFields);
    }

    let mut seen = BTreeSet::new();
    for field in &contract.fields {
        if !seen.insert(field.field) {
            defects.insert(LayoutDefect::DuplicateField);
        }
        if field.byte_size == 0 {
            defects.insert(LayoutDefect::ZeroFieldSize);
        }
        let end = u64::from(field.byte_offset) + u64::from(field.byte_size);
        if end > u64::from(contract.size) {
            defects.insert(LayoutDefect::FieldOutsideRecord);
        }
        if let FieldStorage::BitfieldUnit {
            bit_offset,
            bit_width,
            ..
        } = field.storage
        {
            let bits = u64::from(bit_offset) + u64::from(bit_width);
            if bit_width == 0 || bits > u64::from(field.byte_size) * 8 {
                defects.insert(LayoutDefect::BitfieldBitsOutsideStorage);
            }
        }
    }

    // Every field of one bit-field unit must declare the same storage bytes,
    // and their bit ranges must not overlap.
    let mut units = BTreeMap::<u32, UnitExtent>::new();
    let mut unit_bits = BTreeMap::<u32, Vec<(u32, u32)>>::new();
    for field in &contract.fields {
        if let FieldStorage::BitfieldUnit {
            unit,
            bit_offset,
            bit_width,
        } = field.storage
        {
            let extent = UnitExtent {
                byte_offset: field.byte_offset,
                byte_size: field.byte_size,
            };
            match units.get(&unit) {
                Some(existing)
                    if existing.byte_offset != extent.byte_offset
                        || existing.byte_size != extent.byte_size =>
                {
                    defects.insert(LayoutDefect::BitfieldUnitStorageDisagrees);
                }
                Some(_) => {}
                None => {
                    units.insert(unit, extent);
                }
            }
            let bits = unit_bits.entry(unit).or_default();
            for (offset, width) in bits.iter().copied() {
                if bit_offset < offset + width && offset < bit_offset + bit_width {
                    defects.insert(LayoutDefect::BitfieldBitsOverlap);
                }
            }
            bits.push((bit_offset, bit_width));
        }
    }

    // Addressable fields may not overlap each other or any bit-field unit, and
    // two bit-field units may not share bytes.
    let mut occupied = Vec::<(u64, u64, Option<u32>)>::new();
    for field in &contract.fields {
        let start = u64::from(field.byte_offset);
        let end = start + u64::from(field.byte_size);
        let unit = match field.storage {
            FieldStorage::Addressable => None,
            FieldStorage::BitfieldUnit { unit, .. } => Some(unit),
        };
        if let Some(unit) = unit
            && occupied.iter().any(|(_, _, other)| *other == Some(unit))
        {
            // Already represented by an earlier field of the same unit.
            continue;
        }
        for (other_start, other_end, other_unit) in occupied.iter().copied() {
            if start < other_end && other_start < end {
                match (unit, other_unit) {
                    (Some(_), Some(_)) => {
                        defects.insert(LayoutDefect::BitfieldUnitsOverlap);
                    }
                    _ => {
                        defects.insert(LayoutDefect::AddressableFieldsOverlap);
                    }
                }
            }
        }
        occupied.push((start, end, unit));
    }

    defects
}

#[derive(Clone, Debug)]
struct AllocationState {
    generation: Generation,
    shape: ElementShape,
    elem_size: u32,
    count: u32,
    live: bool,
}

#[derive(Clone, Debug)]
struct HandleState {
    footprint: DerivedFootprint,
    /// A slice handle covering exactly one element records which element, so a
    /// field derivation can check the base.
    single_element: bool,
    retired: bool,
}

/// Decide disjointness of two derived footprints, or refuse the question.
///
/// This is the whole algebra, kept in one function so the Lean model has one
/// counterpart. `Ok(true)` means the checker proved them disjoint; `Ok(false)`
/// means it proved them overlapping; `Err` means the question is outside the
/// profile and is refused. `fresh` is card `CG18-FOOTPRINT-CONNECTION`'s
/// premise input: both footprints' live allocation generations were declared
/// fresh. It is read only for two distinct allocation identities, and a
/// judgement it admits is [`DisjointnessJudgement::DistinctAllocations`]
/// (`QCore.DisjointFootprint.decideMemoryWith`).
fn decide_disjoint(
    event: usize,
    left: HandleId,
    right: HandleId,
    left_footprint: DerivedFootprint,
    right_footprint: DerivedFootprint,
    fresh: bool,
) -> Result<(bool, DisjointnessJudgement), FootprintDiagnostic> {
    match (left_footprint, right_footprint) {
        (
            DerivedFootprint::Memory {
                alloc: left_alloc,
                generation: left_generation,
                byte_start: left_start,
                byte_end: left_end,
                unit: left_unit,
                ..
            },
            DerivedFootprint::Memory {
                alloc: right_alloc,
                generation: right_generation,
                byte_start: right_start,
                byte_end: right_end,
                unit: right_unit,
                ..
            },
        ) => {
            if left_alloc != right_alloc && fresh {
                return Ok((
                    true,
                    DisjointnessJudgement::DistinctAllocations {
                        event,
                        left,
                        right,
                        left_alloc,
                        right_alloc,
                    },
                ));
            }
            if left_alloc != right_alloc {
                return Err(
                    FootprintDiagnostic::CrossAllocationDisjointnessUnsupported {
                        event,
                        left: left_alloc,
                        right: right_alloc,
                    },
                );
            }
            // Both handles are live, so they share the live generation; the
            // comparison is defensive rather than reachable.
            if left_generation != right_generation {
                return Err(FootprintDiagnostic::StaleGeneration {
                    event,
                    alloc: left_alloc,
                    named: right_generation,
                    live: Some(left_generation),
                });
            }
            // The bit-field rule precedes the interval rule: two fields of one
            // storage unit are one memory location, whatever their bit ranges.
            if let (Some(left_unit), Some(right_unit)) = (left_unit, right_unit)
                && left_unit == right_unit
            {
                return Err(FootprintDiagnostic::SharedBitfieldUnit {
                    event,
                    left,
                    right,
                    unit: left_unit,
                });
            }
            let disjoint = left_end <= right_start || right_end <= left_start;
            Ok((
                disjoint,
                DisjointnessJudgement::Memory {
                    event,
                    left,
                    right,
                    alloc: left_alloc,
                    generation: left_generation,
                },
            ))
        }
        (
            DerivedFootprint::Descriptor {
                cell: left_cell, ..
            },
            DerivedFootprint::Descriptor {
                cell: right_cell, ..
            },
        ) => {
            if left_cell == right_cell {
                return Err(FootprintDiagnostic::SharedProtocolCell {
                    event,
                    left,
                    right,
                    cell: left_cell,
                });
            }
            Ok((
                true,
                DisjointnessJudgement::Descriptor {
                    event,
                    left,
                    right,
                    left_cell,
                    right_cell,
                },
            ))
        }
        _ => Err(FootprintDiagnostic::IncomparableFootprintKinds { event, left, right }),
    }
}

/// Check a footprint derivation trace under `DisjointFootprintsV1`.
///
/// Fail-closed: every unresolved extent, missing layout contract, stale handle
/// and undecidable disjointness question is a refusal. A refused event never
/// mints a handle and never changes a generation.
pub fn check_disjoint_footprint_program(
    program: &FootprintProgram,
) -> Result<FootprintCertificate, Vec<FootprintDiagnostic>> {
    let mut diagnostics = Vec::<FootprintDiagnostic>::new();

    // ---- layout contracts, validated before any event runs ----
    let mut contracts = BTreeMap::<RecordId, LayoutContract>::new();
    let mut validated = Vec::<RecordId>::new();
    for contract in &program.layout_contracts {
        if contracts.contains_key(&contract.record) {
            diagnostics.push(FootprintDiagnostic::InconsistentLayoutContract {
                record: contract.record,
                reason: LayoutDefect::DuplicateRecord,
            });
            continue;
        }
        let defects = layout_defects(contract);
        if defects.is_empty() {
            contracts.insert(contract.record, contract.clone());
            validated.push(contract.record);
        } else {
            for reason in defects {
                diagnostics.push(FootprintDiagnostic::InconsistentLayoutContract {
                    record: contract.record,
                    reason,
                });
            }
        }
    }

    let mut allocations = BTreeMap::<AllocationId, AllocationState>::new();
    let mut handles = BTreeMap::<HandleId, HandleState>::new();
    let mut minted_handles = BTreeSet::<HandleId>::new();
    let mut retired = Vec::<(AllocationId, Generation)>::new();
    let mut derived = BTreeMap::<HandleId, DerivedFootprint>::new();
    let mut judgements = Vec::<DisjointnessJudgement>::new();
    let mut protocol_cells = BTreeMap::<DescriptorId, ProtocolCellId>::new();
    let mut assumptions = BTreeSet::<FootprintAssumption>::new();
    // Card CG18-FOOTPRINT-CONNECTION.
    let mut fresh = Vec::<(AllocationId, Generation)>::new();
    let mut views = Vec::<(HandleId, HandleId, ViewKind)>::new();
    let mut transfers = Vec::<ByteTransferJudgement>::new();

    for (index, event) in program.events.iter().enumerate() {
        match event {
            FootprintEvent::Allocate {
                alloc,
                generation,
                shape,
                count,
            } => {
                if allocations.contains_key(alloc) {
                    diagnostics.push(FootprintDiagnostic::IdentityNotFresh {
                        event: index,
                        handle: *alloc,
                    });
                    continue;
                }
                let elem_size = match shape {
                    ElementShape::Scalar { size } => *size,
                    ElementShape::Record { record } => match contracts.get(record) {
                        Some(contract) => contract.size,
                        None => {
                            diagnostics.push(FootprintDiagnostic::LayoutContractMissing {
                                event: index,
                                record: *record,
                            });
                            continue;
                        }
                    },
                };
                let bytes = u64::from(elem_size) * u64::from(*count);
                if elem_size == 0 || *count == 0 || bytes > u64::from(u32::MAX) {
                    diagnostics.push(FootprintDiagnostic::InvalidExtent {
                        event: index,
                        alloc: *alloc,
                    });
                    continue;
                }
                allocations.insert(
                    *alloc,
                    AllocationState {
                        generation: *generation,
                        shape: *shape,
                        elem_size,
                        count: *count,
                        live: true,
                    },
                );
                assumptions.insert(FootprintAssumption::AllocationExtentDeclared);
            }

            FootprintEvent::Reallocate {
                alloc,
                generation,
                count,
            } => {
                let Some(state) = allocations.get(alloc) else {
                    diagnostics.push(FootprintDiagnostic::UnknownAllocation {
                        event: index,
                        alloc: *alloc,
                    });
                    continue;
                };
                if !state.live {
                    diagnostics.push(FootprintDiagnostic::DestroyedAllocation {
                        event: index,
                        alloc: *alloc,
                    });
                    continue;
                }
                if *generation <= state.generation {
                    diagnostics.push(FootprintDiagnostic::StaleGeneration {
                        event: index,
                        alloc: *alloc,
                        named: *generation,
                        live: Some(state.generation),
                    });
                    continue;
                }
                let bytes = u64::from(state.elem_size) * u64::from(*count);
                if *count == 0 || bytes > u64::from(u32::MAX) {
                    diagnostics.push(FootprintDiagnostic::InvalidExtent {
                        event: index,
                        alloc: *alloc,
                    });
                    continue;
                }
                let old = state.generation;
                retire_generation(&mut handles, *alloc, old);
                retired.push((*alloc, old));
                let state = allocations
                    .get_mut(alloc)
                    .expect("allocation presence checked above");
                state.generation = *generation;
                state.count = *count;
            }

            FootprintEvent::Destroy { alloc } => {
                let Some(state) = allocations.get_mut(alloc) else {
                    diagnostics.push(FootprintDiagnostic::UnknownAllocation {
                        event: index,
                        alloc: *alloc,
                    });
                    continue;
                };
                if !state.live {
                    diagnostics.push(FootprintDiagnostic::DestroyedAllocation {
                        event: index,
                        alloc: *alloc,
                    });
                    continue;
                }
                let generation = state.generation;
                state.live = false;
                retire_generation(&mut handles, *alloc, generation);
                retired.push((*alloc, generation));
            }

            FootprintEvent::DeriveSlice {
                handle,
                alloc,
                generation,
                elem_offset,
                elem_count,
                declared_bytes,
            } => {
                if !minted_handles.insert(*handle) {
                    diagnostics.push(FootprintDiagnostic::IdentityNotFresh {
                        event: index,
                        handle: *handle,
                    });
                    continue;
                }
                let Some(state) = allocations.get(alloc) else {
                    diagnostics.push(FootprintDiagnostic::UnknownAllocation {
                        event: index,
                        alloc: *alloc,
                    });
                    continue;
                };
                if !state.live {
                    diagnostics.push(FootprintDiagnostic::DestroyedAllocation {
                        event: index,
                        alloc: *alloc,
                    });
                    continue;
                }
                if *generation != state.generation {
                    diagnostics.push(FootprintDiagnostic::StaleGeneration {
                        event: index,
                        alloc: *alloc,
                        named: *generation,
                        live: Some(state.generation),
                    });
                    continue;
                }
                if *elem_count == 0 {
                    diagnostics.push(FootprintDiagnostic::InvalidExtent {
                        event: index,
                        alloc: *alloc,
                    });
                    continue;
                }
                let elem_end = u64::from(*elem_offset) + u64::from(*elem_count);
                if elem_end > u64::from(state.count) {
                    diagnostics.push(FootprintDiagnostic::SliceOutOfBounds {
                        event: index,
                        alloc: *alloc,
                        elem_end,
                        count: state.count,
                    });
                    continue;
                }
                let byte_start = u64::from(*elem_offset) * u64::from(state.elem_size);
                let byte_end = elem_end * u64::from(state.elem_size);
                // Guaranteed by the extent check at `Allocate`, but computed
                // rather than assumed.
                if byte_end > u64::from(u32::MAX) || byte_start >= byte_end {
                    diagnostics.push(FootprintDiagnostic::InvalidExtent {
                        event: index,
                        alloc: *alloc,
                    });
                    continue;
                }
                let byte_start = byte_start as u32;
                let byte_end = byte_end as u32;
                if let Some(declared) = declared_bytes
                    && *declared != (byte_start, byte_end)
                {
                    diagnostics.push(FootprintDiagnostic::DerivedFootprintMismatch {
                        event: index,
                        handle: *handle,
                        declared: *declared,
                        derived: (byte_start, byte_end),
                    });
                    continue;
                }
                let footprint = DerivedFootprint::Memory {
                    alloc: *alloc,
                    generation: *generation,
                    byte_start,
                    byte_end,
                    unit: None,
                    bits: None,
                };
                handles.insert(
                    *handle,
                    HandleState {
                        footprint,
                        single_element: *elem_count == 1,
                        retired: false,
                    },
                );
                derived.insert(*handle, footprint);
            }

            FootprintEvent::DeriveField {
                handle,
                base,
                field,
            } => {
                if !minted_handles.insert(*handle) {
                    diagnostics.push(FootprintDiagnostic::IdentityNotFresh {
                        event: index,
                        handle: *handle,
                    });
                    continue;
                }
                let Some(base_state) = handles.get(base) else {
                    diagnostics.push(FootprintDiagnostic::UnknownHandle {
                        event: index,
                        handle: *base,
                    });
                    continue;
                };
                if base_state.retired {
                    let (alloc, named) = match base_state.footprint {
                        DerivedFootprint::Memory {
                            alloc, generation, ..
                        } => (alloc, generation),
                        DerivedFootprint::Descriptor { descriptor, .. } => (descriptor, 0),
                    };
                    diagnostics.push(FootprintDiagnostic::StaleGeneration {
                        event: index,
                        alloc,
                        named,
                        live: allocations.get(&alloc).map(|state| state.generation),
                    });
                    continue;
                }
                let DerivedFootprint::Memory {
                    alloc,
                    generation,
                    byte_start,
                    ..
                } = base_state.footprint
                else {
                    diagnostics.push(FootprintDiagnostic::ElementShapeMismatch {
                        event: index,
                        handle: *base,
                    });
                    continue;
                };
                if !base_state.single_element {
                    diagnostics.push(FootprintDiagnostic::ElementShapeMismatch {
                        event: index,
                        handle: *base,
                    });
                    continue;
                }
                let Some(state) = allocations.get(&alloc) else {
                    diagnostics.push(FootprintDiagnostic::UnknownAllocation {
                        event: index,
                        alloc,
                    });
                    continue;
                };
                let ElementShape::Record { record } = state.shape else {
                    diagnostics.push(FootprintDiagnostic::ElementShapeMismatch {
                        event: index,
                        handle: *base,
                    });
                    continue;
                };
                let Some(contract) = contracts.get(&record) else {
                    diagnostics.push(FootprintDiagnostic::LayoutContractMissing {
                        event: index,
                        record,
                    });
                    continue;
                };
                let Some(layout) = contract.fields.iter().find(|entry| entry.field == *field)
                else {
                    diagnostics.push(FootprintDiagnostic::UnknownField {
                        event: index,
                        record,
                        field: *field,
                    });
                    continue;
                };
                let field_start = u64::from(byte_start) + u64::from(layout.byte_offset);
                let field_end = field_start + u64::from(layout.byte_size);
                // The contract was validated against the record size and the
                // slice against the extent, so this cannot exceed the object.
                if field_end > u64::from(u32::MAX) {
                    diagnostics.push(FootprintDiagnostic::InvalidExtent {
                        event: index,
                        alloc,
                    });
                    continue;
                }
                let (unit, bits) = match layout.storage {
                    FieldStorage::Addressable => (None, None),
                    FieldStorage::BitfieldUnit {
                        unit,
                        bit_offset,
                        bit_width,
                    } => (Some(unit), Some((bit_offset, bit_width))),
                };
                let footprint = DerivedFootprint::Memory {
                    alloc,
                    generation,
                    byte_start: field_start as u32,
                    byte_end: field_end as u32,
                    unit,
                    bits,
                };
                handles.insert(
                    *handle,
                    HandleState {
                        footprint,
                        single_element: false,
                        retired: false,
                    },
                );
                derived.insert(*handle, footprint);
                assumptions.insert(FootprintAssumption::DeclaredRecordLayout);
            }

            // One arm for both alias forms, so a derived alias cannot drift
            // from a declared one in anything but the premise it stamps: the
            // `declared` flag is the only thing the two variants decide
            // differently (card `CT42-CELL-ROUTE`).
            FootprintEvent::DeclareDescriptorAlias {
                handle,
                descriptor,
                cell,
            }
            | FootprintEvent::DeriveDescriptorAlias {
                handle,
                descriptor,
                cell,
            } => {
                let declared = matches!(event, FootprintEvent::DeclareDescriptorAlias { .. });
                if !minted_handles.insert(*handle) {
                    diagnostics.push(FootprintDiagnostic::IdentityNotFresh {
                        event: index,
                        handle: *handle,
                    });
                    continue;
                }
                let footprint = DerivedFootprint::Descriptor {
                    descriptor: *descriptor,
                    cell: *cell,
                };
                handles.insert(
                    *handle,
                    HandleState {
                        footprint,
                        single_element: false,
                        retired: false,
                    },
                );
                derived.insert(*handle, footprint);
                protocol_cells.insert(*descriptor, *cell);
                // A declared cell is the caller's premise and is named on the
                // certificate; a derived cell's ground is the deriving model's
                // admission and stamps nothing here (`CT42-CELL-DERIVED-PREMISE`).
                if declared {
                    assumptions.insert(FootprintAssumption::DeclaredProtocolCellIdentity);
                }
            }

            FootprintEvent::AssertDisjoint { left, right } => {
                let mut resolved = Vec::with_capacity(2);
                let mut failed = false;
                for handle in [*left, *right] {
                    match handles.get(&handle) {
                        None => {
                            diagnostics.push(FootprintDiagnostic::UnknownHandle {
                                event: index,
                                handle,
                            });
                            failed = true;
                        }
                        Some(state) if state.retired => {
                            let (alloc, named) = match state.footprint {
                                DerivedFootprint::Memory {
                                    alloc, generation, ..
                                } => (alloc, generation),
                                DerivedFootprint::Descriptor { descriptor, .. } => (descriptor, 0),
                            };
                            diagnostics.push(FootprintDiagnostic::StaleGeneration {
                                event: index,
                                alloc,
                                named,
                                live: allocations.get(&alloc).map(|state| state.generation),
                            });
                            failed = true;
                        }
                        Some(state) => resolved.push(state.footprint),
                    }
                }
                if failed {
                    continue;
                }
                let both_fresh = is_fresh(&fresh, resolved[0]) && is_fresh(&fresh, resolved[1]);
                match decide_disjoint(index, *left, *right, resolved[0], resolved[1], both_fresh) {
                    Err(diagnostic) => diagnostics.push(diagnostic),
                    Ok((false, _)) => {
                        diagnostics.push(FootprintDiagnostic::OverlappingFootprints {
                            event: index,
                            left: *left,
                            right: *right,
                        });
                    }
                    Ok((true, judgement)) => {
                        if matches!(judgement, DisjointnessJudgement::DistinctAllocations { .. }) {
                            assumptions.insert(FootprintAssumption::DistinctAllocationsDisjoint);
                        }
                        judgements.push(judgement);
                    }
                }
            }

            FootprintEvent::Unsupported { construct } => {
                diagnostics.push(FootprintDiagnostic::UnsupportedFootprintConstruct {
                    event: index,
                    construct: *construct,
                });
            }

            // ---- card CG18-FOOTPRINT-CONNECTION ----
            FootprintEvent::DeclareFreshAllocation { alloc } => {
                let Some(state) = allocations.get(alloc) else {
                    diagnostics.push(FootprintDiagnostic::UnknownAllocation {
                        event: index,
                        alloc: *alloc,
                    });
                    continue;
                };
                if !state.live {
                    diagnostics.push(FootprintDiagnostic::DestroyedAllocation {
                        event: index,
                        alloc: *alloc,
                    });
                    continue;
                }
                let key = (*alloc, state.generation);
                if !fresh.contains(&key) {
                    fresh.push(key);
                }
            }

            FootprintEvent::DeriveView { handle, base, view } => {
                if !minted_handles.insert(*handle) {
                    diagnostics.push(FootprintDiagnostic::IdentityNotFresh {
                        event: index,
                        handle: *handle,
                    });
                    continue;
                }
                let Some(footprint) =
                    resolve_live(index, *base, &handles, &allocations, &mut diagnostics)
                else {
                    continue;
                };
                if !byte_addressable(footprint) {
                    diagnostics.push(FootprintDiagnostic::ElementShapeMismatch {
                        event: index,
                        handle: *base,
                    });
                    continue;
                }
                match view {
                    ViewKind::Punned => {
                        diagnostics.push(FootprintDiagnostic::UnsupportedFootprintConstruct {
                            event: index,
                            construct: UnsupportedFootprint::TypePunnedAlias,
                        });
                        continue;
                    }
                    ViewKind::EffectiveType => {
                        assumptions.insert(FootprintAssumption::EffectiveTypeDeclared);
                    }
                    ViewKind::Character => {}
                }
                // A view is never a field base: `single_element` stays false.
                handles.insert(
                    *handle,
                    HandleState {
                        footprint,
                        single_element: false,
                        retired: false,
                    },
                );
                derived.insert(*handle, footprint);
                views.push((*handle, *base, *view));
            }

            FootprintEvent::ByteCopy {
                destination,
                source,
                bytes,
                may_overlap,
            } => {
                let target = resolve_live(
                    index,
                    *destination,
                    &handles,
                    &allocations,
                    &mut diagnostics,
                );
                let origin = resolve_live(index, *source, &handles, &allocations, &mut diagnostics);
                let (Some(target), Some(origin)) = (target, origin) else {
                    continue;
                };
                let Some(target_range) =
                    byte_range(index, *destination, target, *bytes, &mut diagnostics)
                else {
                    continue;
                };
                let Some(origin_range) =
                    byte_range(index, *source, origin, *bytes, &mut diagnostics)
                else {
                    continue;
                };
                if !*may_overlap {
                    let both_fresh = is_fresh(&fresh, target) && is_fresh(&fresh, origin);
                    match decide_disjoint(
                        index,
                        *destination,
                        *source,
                        target_range,
                        origin_range,
                        both_fresh,
                    ) {
                        Err(diagnostic) => {
                            diagnostics.push(diagnostic);
                            continue;
                        }
                        Ok((false, _)) => {
                            diagnostics.push(FootprintDiagnostic::UnsupportedFootprintConstruct {
                                event: index,
                                construct: UnsupportedFootprint::OverlappingCopy,
                            });
                            continue;
                        }
                        Ok((true, judgement)) => {
                            if matches!(
                                judgement,
                                DisjointnessJudgement::DistinctAllocations { .. }
                            ) {
                                assumptions
                                    .insert(FootprintAssumption::DistinctAllocationsDisjoint);
                            }
                        }
                    }
                }
                assumptions.insert(FootprintAssumption::ByteRepresentation);
                transfers.push(ByteTransferJudgement::Copy {
                    event: index,
                    destination: *destination,
                    source: *source,
                    bytes: *bytes,
                    may_overlap: *may_overlap,
                });
            }

            FootprintEvent::ByteFill { destination, bytes } => {
                let Some(target) = resolve_live(
                    index,
                    *destination,
                    &handles,
                    &allocations,
                    &mut diagnostics,
                ) else {
                    continue;
                };
                if byte_range(index, *destination, target, *bytes, &mut diagnostics).is_none() {
                    continue;
                }
                assumptions.insert(FootprintAssumption::ByteRepresentation);
                transfers.push(ByteTransferJudgement::Fill {
                    event: index,
                    destination: *destination,
                    bytes: *bytes,
                });
            }
        }
    }

    if !diagnostics.is_empty() {
        return Err(diagnostics);
    }

    let live_allocations = allocations
        .iter()
        .filter(|(_, state)| state.live)
        .map(|(alloc, state)| (*alloc, state.generation))
        .collect();

    Ok(FootprintCertificate {
        profile: FootprintProfile::DisjointFootprintsV1,
        assumptions,
        checked_events: program.events.len(),
        derived_footprints: derived,
        admitted_disjointness: judgements,
        retired_generations: retired,
        live_allocations,
        validated_layout_contracts: validated,
        protocol_cells,
        fresh_allocations: fresh,
        derived_views: views,
        byte_transfers: transfers,
    })
}

/// Card CG18: whether a memory footprint's allocation generation was declared
/// fresh. A descriptor is never fresh.
fn is_fresh(fresh: &[(AllocationId, Generation)], footprint: DerivedFootprint) -> bool {
    match footprint {
        DerivedFootprint::Memory {
            alloc, generation, ..
        } => fresh.contains(&(alloc, generation)),
        DerivedFootprint::Descriptor { .. } => false,
    }
}

/// Card CG18: a footprint whose bytes a view or a byte transfer may name:
/// memory that is not a bit-field storage unit.
const fn byte_addressable(footprint: DerivedFootprint) -> bool {
    matches!(footprint, DerivedFootprint::Memory { unit: None, .. })
}

/// Card CG18: resolve a handle a view or a byte transfer names, refusing an
/// unknown or retired one exactly as [`FootprintEvent::AssertDisjoint`] does.
fn resolve_live(
    event: usize,
    handle: HandleId,
    handles: &BTreeMap<HandleId, HandleState>,
    allocations: &BTreeMap<AllocationId, AllocationState>,
    diagnostics: &mut Vec<FootprintDiagnostic>,
) -> Option<DerivedFootprint> {
    match handles.get(&handle) {
        None => {
            diagnostics.push(FootprintDiagnostic::UnknownHandle { event, handle });
            None
        }
        Some(state) if state.retired => {
            let (alloc, named) = match state.footprint {
                DerivedFootprint::Memory {
                    alloc, generation, ..
                } => (alloc, generation),
                DerivedFootprint::Descriptor { descriptor, .. } => (descriptor, 0),
            };
            diagnostics.push(FootprintDiagnostic::StaleGeneration {
                event,
                alloc,
                named,
                live: allocations.get(&alloc).map(|state| state.generation),
            });
            None
        }
        Some(state) => Some(state.footprint),
    }
}

/// Card CG18 (M26): the first `bytes` bytes of `footprint`, or a refusal: a
/// bit-field unit or a descriptor (`Q659`), zero bytes (`Q650`), or a range
/// past the handle (`Q653`, `elem_end` the requested bytes and `count` the
/// handle's).
fn byte_range(
    event: usize,
    handle: HandleId,
    footprint: DerivedFootprint,
    bytes: u32,
    diagnostics: &mut Vec<FootprintDiagnostic>,
) -> Option<DerivedFootprint> {
    let DerivedFootprint::Memory {
        alloc,
        generation,
        byte_start,
        byte_end,
        unit: None,
        ..
    } = footprint
    else {
        diagnostics.push(FootprintDiagnostic::ElementShapeMismatch { event, handle });
        return None;
    };
    if bytes == 0 {
        diagnostics.push(FootprintDiagnostic::InvalidExtent { event, alloc });
        return None;
    }
    let length = byte_end - byte_start;
    if bytes > length {
        diagnostics.push(FootprintDiagnostic::SliceOutOfBounds {
            event,
            alloc,
            elem_end: u64::from(bytes),
            count: length,
        });
        return None;
    }
    Some(DerivedFootprint::Memory {
        alloc,
        generation,
        byte_start,
        byte_end: byte_start + bytes,
        unit: None,
        bits: None,
    })
}

fn retire_generation(
    handles: &mut BTreeMap<HandleId, HandleState>,
    alloc: AllocationId,
    generation: Generation,
) {
    for state in handles.values_mut() {
        if let DerivedFootprint::Memory {
            alloc: handle_alloc,
            generation: handle_generation,
            ..
        } = state.footprint
            && handle_alloc == alloc
            && handle_generation == generation
        {
            state.retired = true;
        }
    }
}

/// Independently re-derive the certificate and refuse anything the supplied one
/// claims that re-derivation does not reproduce.
pub fn recheck_disjoint_footprint_certificate(
    program: &FootprintProgram,
    certificate: &FootprintCertificate,
) -> Result<(), FootprintRecheckError> {
    // The discharged premise is refused before anything is compared, so a
    // forged certificate cannot buy it by otherwise matching.
    if certificate
        .assumptions
        .contains(&FootprintAssumption::DeclaredDisjointness)
    {
        return Err(FootprintRecheckError::ForbiddenAssumption {
            assumption: FootprintAssumption::DeclaredDisjointness,
        });
    }
    let rederived = check_disjoint_footprint_program(program)
        .map_err(FootprintRecheckError::ProgramRejected)?;
    let checks: [(&'static str, bool); 12] = [
        ("profile", certificate.profile == rederived.profile),
        (
            "assumptions",
            certificate.assumptions == rederived.assumptions,
        ),
        (
            "checked_events",
            certificate.checked_events == rederived.checked_events,
        ),
        (
            "derived_footprints",
            certificate.derived_footprints == rederived.derived_footprints,
        ),
        (
            "admitted_disjointness",
            certificate.admitted_disjointness == rederived.admitted_disjointness,
        ),
        (
            "retired_generations",
            certificate.retired_generations == rederived.retired_generations,
        ),
        (
            "live_allocations",
            certificate.live_allocations == rederived.live_allocations,
        ),
        (
            "validated_layout_contracts",
            certificate.validated_layout_contracts == rederived.validated_layout_contracts,
        ),
        (
            "protocol_cells",
            certificate.protocol_cells == rederived.protocol_cells,
        ),
        (
            "fresh_allocations",
            certificate.fresh_allocations == rederived.fresh_allocations,
        ),
        (
            "derived_views",
            certificate.derived_views == rederived.derived_views,
        ),
        (
            "byte_transfers",
            certificate.byte_transfers == rederived.byte_transfers,
        ),
    ];
    for (field, matches) in checks {
        if !matches {
            return Err(FootprintRecheckError::FieldMismatch { field });
        }
    }
    Ok(())
}

#[cfg(test)]
#[path = "../tests/unit/disjoint_footprint.rs"]
mod tests;
