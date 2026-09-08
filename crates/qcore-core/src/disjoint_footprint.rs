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
//!   free "distinct objects do not alias";
//! * unions, flexible array members, variable-length arrays, computed indices,
//!   pointer arithmetic outside a declared extent, nested anonymous aggregates,
//!   type-punned aliases, overlapping copies and atomic reclamation.
//!
//! What stays a premise is named on the certificate: the declared element size
//! and count are the real object extent
//! ([`FootprintAssumption::AllocationExtentDeclared`]), a layout contract is the
//! target ABI's real layout ([`FootprintAssumption::DeclaredRecordLayout`]), and
//! a declared protocol cell is really the shared logical state
//! ([`FootprintAssumption::DeclaredProtocolCellIdentity`]).
//! [`FootprintAssumption::DeclaredDisjointness`] is the premise this profile
//! discharges: the checker never emits it and
//! [`recheck_disjoint_footprint_certificate`] refuses any certificate carrying
//! it.
//!
//! This is a model-level module. No source lowering, no production audit route
//! and no other checker calls it; composing it with `CheckedMutexInvariantV1`
//! is the follow-on unit recorded in
//! `docs/design/DISJOINT_FOOTPRINT_PROFILE.md`.

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
    DeclareDescriptorAlias {
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
}

impl FootprintCertificate {
    /// Assumption names as plain strings, so a free-form assumption map cannot
    /// smuggle the discharged premise past the typed set.
    pub fn assumption_names(&self) -> BTreeSet<&'static str> {
        self.assumptions
            .iter()
            .map(|assumption| match assumption {
                FootprintAssumption::AllocationExtentDeclared => "AllocationExtentDeclared",
                FootprintAssumption::DeclaredRecordLayout => "DeclaredRecordLayout",
                FootprintAssumption::DeclaredProtocolCellIdentity => "DeclaredProtocolCellIdentity",
                FootprintAssumption::DeclaredDisjointness => "DeclaredDisjointness",
            })
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
/// profile and is refused.
fn decide_disjoint(
    event: usize,
    left: HandleId,
    right: HandleId,
    left_footprint: DerivedFootprint,
    right_footprint: DerivedFootprint,
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

            FootprintEvent::DeclareDescriptorAlias {
                handle,
                descriptor,
                cell,
            } => {
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
                assumptions.insert(FootprintAssumption::DeclaredProtocolCellIdentity);
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
                match decide_disjoint(index, *left, *right, resolved[0], resolved[1]) {
                    Err(diagnostic) => diagnostics.push(diagnostic),
                    Ok((false, _)) => {
                        diagnostics.push(FootprintDiagnostic::OverlappingFootprints {
                            event: index,
                            left: *left,
                            right: *right,
                        });
                    }
                    Ok((true, judgement)) => judgements.push(judgement),
                }
            }

            FootprintEvent::Unsupported { construct } => {
                diagnostics.push(FootprintDiagnostic::UnsupportedFootprintConstruct {
                    event: index,
                    construct: *construct,
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
    let checks: [(&'static str, bool); 9] = [
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
