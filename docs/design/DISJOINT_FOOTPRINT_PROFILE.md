# `DisjointFootprintsV1` — derived access footprints and alias identity

Card T09 (W6, concurrency lane). Status: **design plus a bounded model-level
prototype**. The prototype is `crates/qcore-core/src/disjoint_footprint.rs` with
`proof/QCore/DisjointFootprint.lean`; it is not wired into source lowering, the
production audit routes or any other checker. The follow-on implementation units
that would wire it are listed in §8.

Evidence record: [`../reports/remaining-t09/README.md`](../reports/remaining-t09/README.md).

---

## 1. The premise this profile exists to discharge

`crates/qcore-core/src/concurrency.rs` and
`crates/qcore-core/src/mutex_invariant.rs` both take
`AccessFootprint { resource, start, end }` as a **declared** interval. T02's
`CheckedMutexInvariantV1` discharged two of `TrustedExplicitTrace`'s premises
and had to introduce a new one to do it:

> `DeclaredFootprintShape` — that the declared intervals correspond to real C
> layout.

`docs/reports/remaining-t02/README.md` §6 records this as "the honest residue".
Nothing in the repository derives an interval today: a caller writes
`AccessFootprint::new(7, 0, 16)` and the checker believes the numbers.

T05 (counted read shares) is this card's nominal dependency and is **not present
in this worktree** (see the evidence report §1.1); its recorded open boundaries
— disjoint slicing, byte-size accounting and separation-logic fractions — are
exactly the space this card occupies from the footprint side. The share algebra
and the footprint algebra are orthogonal: T02's pointwise ledger conserves
*shares* over *atoms*, and says nothing about where an atom's boundaries came
from. This profile supplies the boundaries.

The claim of this card is deliberately narrow: for the shapes below, the
interval is **computed and checked** rather than declared, so a later increment
can replace `DeclaredFootprintShape` with the three narrower, explicitly named
premises of §6 for those shapes only.

---

## 2. Model

### 2.1 Identities

| Identity | Meaning | Reuse rule |
|---|---|---|
| `AllocationId` | one allocation lifetime | never a reusable address |
| `Generation` | which generation of that allocation | strictly increasing; a retired generation is stale forever |
| `HandleId` | one derived footprint | globally fresh at mint, retired handles included |
| `RecordId` / `FieldId` | a record type and one of its members | named by a layout contract |
| `DescriptorId` | one OS/library descriptor | a handle, never a byte range |
| `ProtocolCellId` | the shared logical state two descriptors advance | declared |

### 2.2 Extents and derivation

An `Allocate { alloc, generation, shape, count }` fixes the extent for that
generation. `shape` is either `Scalar { size }` or `Record { record }`, and a
record's element size **is** the validated layout contract's `size`: a record
allocation whose contract is missing or refused is itself refused, so the extent
is never guessed.

`DeriveSlice { handle, alloc, generation, elem_offset, elem_count, declared_bytes }`
checks, in order:

1. the handle identity is fresh;
2. the allocation exists and is live;
3. `generation` is the live generation (otherwise `StaleGeneration`);
4. `elem_count > 0`;
5. `elem_offset + elem_count <= count`, computed in `u64` so a `u32` sum cannot
   wrap into bounds (`SliceOutOfBounds`);
6. the byte interval is **computed**:
   `[elem_offset * elem_size, (elem_offset + elem_count) * elem_size)`;
7. `declared_bytes`, when supplied, is compared against the computed interval
   (`DerivedFootprintMismatch`). A supplied interval is never used in place of
   the derivation — this is the same derive-and-compare discipline T02 applies
   to synchronization witnesses.

### 2.3 The disjointness decision

One function, `decide_disjoint`, decides or refuses; there is no other route to
a judgement. Its rules apply in this order:

| Situation | Outcome |
|---|---|
| one memory footprint, one descriptor footprint | `IncomparableFootprintKinds` (`Q663`) |
| two memory footprints, different `AllocationId` | `CrossAllocationDisjointnessUnsupported` (`Q662`) |
| two memory footprints, different `Generation` | `StaleGeneration` (`Q652`) |
| two memory footprints, one bit-field storage unit | `SharedBitfieldUnit` (`Q661`) |
| two memory footprints, otherwise | interval arithmetic; overlap is `OverlappingFootprints` (`Q660`) |
| two descriptors, same `ProtocolCellId` | `SharedProtocolCell` (`Q664`) |
| two descriptors, distinct cells | disjoint |

**Cross-allocation disjointness is a refusal, not a free answer.** Two distinct
`AllocationId`s are two distinct declared lifetimes, but concluding that their
bytes cannot alias needs an allocator model and a non-overlap argument this
profile does not have. Answering "disjoint" there would smuggle in exactly the
kind of unproven non-aliasing the card excludes. The acceptance clause is
therefore stated only *against one underlying generation identity*, and the
cross-allocation question is `Q662` with `CrossAllocationAliasProof` also
present as an explicit `Unsupported` construct.

### 2.4 Staleness

`Reallocate` requires a strictly greater generation, retires the previous one
and marks every handle of that generation stale. `Destroy` retires the live
generation and the allocation is dead forever. A stale handle is refused at
`AssertDisjoint` and at `DeriveField`, **before** any interval is compared, so a
stale alias can never be answered "disjoint" by coincidence of numbers.

### 2.5 Layout contracts

A `LayoutContract { record, size, fields }` is validated whole, before any event
runs, against eleven structural rules whose failures are the `LayoutDefect`
vocabulary: `ZeroSize`, `NoFields`, `DuplicateRecord`, `DuplicateField`,
`ZeroFieldSize`, `FieldOutsideRecord`, `AddressableFieldsOverlap`,
`BitfieldUnitStorageDisagrees`, `BitfieldBitsOutsideStorage`,
`BitfieldBitsOverlap`, `BitfieldUnitsOverlap`.

There is **no partial admission**: a contract that fails any rule is refused
entirely, and every derivation naming that record is refused with
`LayoutContractMissing`. A record allocation with no contract at all is refused
at `Allocate`, because its element size is unknown.

`FieldStorage` is the C11 memory-location distinction, not a naming convention:

* `Addressable` — the field owns its byte range;
* `BitfieldUnit { unit, bit_offset, bit_width }` — the field occupies bits
  inside a **storage unit**. Every field of one unit must declare the same byte
  range, their bit ranges must not overlap, and two units may not share bytes.

A field footprint's byte interval is the base element's start plus the declared
offset; a bit-field's interval is its *storage unit's* bytes, with the bit range
kept for reporting and never used in the decision.

### 2.6 Descriptor aliases versus memory slices

This is acceptance item 3, and it is stated as a separation rather than a
comparison.

A memory slice is a set of bytes. Disjointness of two slices is a statement
about that set, and it is exactly what lets two tasks mutate the two slices
independently.

A descriptor is a *handle*, and two descriptors over one file, socket, chunk or
connection share a **protocol cell**: a logical state — file offset, reference
count, connection phase, ring position — that either handle can advance. Three
consequences:

1. **Byte disjointness says nothing about a cell.** Two descriptors may touch
   disjoint bytes, or no bytes at all, and still both advance one offset.
2. **Cell identity is the whole decision.** Two descriptors over one cell are
   never disjoint (`Q664`), whatever their byte footprints.
3. **The kinds do not mix.** `decide_disjoint` on one memory footprint and one
   descriptor is `IncomparableFootprintKinds` (`Q663`), in both argument orders.
   Refusing the question is the point: an implementation that answered
   "disjoint" would let a proof about bytes license independent use of two
   descriptors, which is the CT42 failure.

The cell itself is a **declaration**
(`FootprintAssumption::DeclaredProtocolCellIdentity`). This profile does not
discover which descriptors alias; it records the declared alias and enforces its
consequence. Deriving cell identity from source (`dup`, `fdopen`, a chunk handle
table) is follow-on unit F4.

---

## 3. Acceptance, item by item

### 3.1 "Prove interval bounds/disjointness against one underlying generation identity"

* Bounds: `sliceBytes_within_extent` (Lean) and `a_slice_past_the_declared_extent_is_refused`,
  `an_element_range_that_wraps_a_u32_is_refused_rather_than_wrapping` (Rust).
* Disjointness: `range_disjoint_iff_bytes_disjoint` proves the element→byte
  encoding is **faithful in both directions** over a nonzero element size, so
  the encoding neither over- nor under-approximates;
  `disjoint_intervals_share_no_byte` gives the semantic soundness statement
  (an admitted judgement means no byte is in both) and
  `overlapping_intervals_share_a_byte` its non-vacuous converse.
* One generation: `admitted_disjointness_has_a_ground` proves every admitted
  judgement carries one of exactly two grounds, the memory one requiring a
  common `alloc` **and** a common `gen`.
  `cross_allocation_disjointness_is_refused` and `stale_generation_is_refused`
  close the other directions.

### 3.2 "Overlaps and stale aliases reject; struct fields and bit-fields require distinct explicit layout contracts"

* Overlaps: `Q660`, with a positive control that slices touching at exactly one
  boundary are admitted, and `nonempty_interval_is_not_disjoint_from_itself`.
* Stale aliases: `reallocation_retires_every_handle_of_the_old_generation`,
  `destruction_retires_every_handle`, `a_stale_handle_decides_nothing`, plus
  `retirement_preserves_other_allocations` so retirement is not a blunt
  instrument.
* Struct fields: no layout contract, no field footprint — there is no other
  constructor. `addressable_fields_with_disjoint_ranges_are_disjoint` is the
  admission and `field_stays_inside_its_element` the containment.
* Bit-fields: `shared_bitfield_unit_is_never_disjoint` holds for *every* pair of
  byte ranges and *every* pair of bit ranges, so it is not an accident of
  overlap; `the_bitfield_rule_precedes_the_interval_rule` pins that the refusal
  reason is the storage unit rather than a generic overlap; and
  `distinct_bitfield_units_are_decided_by_bytes` is its positive control.

### 3.3 "State separately how descriptor aliases sharing a protocol cell differ from memory-slice disjointness"

§2.6, with `descriptors_sharing_a_cell_are_never_disjoint`,
`descriptors_over_distinct_cells_are_disjoint` and
`memory_and_descriptor_are_incomparable`, and the Rust scenario
`memory_disjointness_does_not_transfer_to_a_protocol_cell` in which one program
admits two disjoint slices of a buffer and refuses two descriptors over the cell
that buffer belongs to.

---

## 4. Refusal vocabulary

`Q650`–`Q667`, inside the `Q6xx` concurrency range of
`docs/QCORE_SPEC_r0_3.md` §L21. `Q600` (bounded concurrency and scoped tasks)
and `Q610`–`Q623` (T02) are **not** reused; `named_refusals_occupy_the_q630_to_q647_range`
pins the whole table and asserts the absence of the reserved codes.

| Code | Diagnostic |
|---|---|
| `Q650` | `InvalidExtent` (zero element size, zero count, empty slice, unrepresentable byte extent) |
| `Q651` | `UnknownAllocation` |
| `Q652` | `StaleGeneration` |
| `Q653` | `SliceOutOfBounds` |
| `Q654` | `IdentityNotFresh` |
| `Q655` | `UnknownHandle` |
| `Q656` | `LayoutContractMissing` |
| `Q657` | `InconsistentLayoutContract` |
| `Q658` | `UnknownField` |
| `Q659` | `ElementShapeMismatch` |
| `Q660` | `OverlappingFootprints` |
| `Q661` | `SharedBitfieldUnit` |
| `Q662` | `CrossAllocationDisjointnessUnsupported` |
| `Q663` | `IncomparableFootprintKinds` |
| `Q664` | `SharedProtocolCell` |
| `Q665` | `DerivedFootprintMismatch` |
| `Q666` | `DestroyedAllocation` |
| `Q667` | `UnsupportedFootprintConstruct` |

Ten shapes are **represented refusals** rather than omissions, each a real
`FootprintEvent::Unsupported` so that "the profile refuses it" is testable:
`UnionMember`, `FlexibleArrayMember`, `VariableLengthArray`, `ComputedIndex`,
`PointerArithmeticOutsideDeclaredExtent`, `NestedAnonymousAggregate`,
`TypePunnedAlias`, `OverlappingCopy`, `CrossAllocationAliasProof`,
`AtomicReclamation`.

---

## 5. Certificate and recheck

```rust
FootprintCertificate {
    profile, assumptions, checked_events,
    derived_footprints, admitted_disjointness,
    retired_generations, live_allocations,
    validated_layout_contracts, protocol_cells,
}
```

`recheck_disjoint_footprint_certificate` re-runs the checker independently and
refuses any of the nine fields the re-derivation does not reproduce. It refuses
`FootprintAssumption::DeclaredDisjointness` **before** comparing anything, so a
certificate cannot buy the discharged premise by otherwise matching — the same
pinned negative shape as T02's
`a_checked_certificate_never_carries_trusted_guard_assumptions`.

`DerivedFootprint::export` is the bridge into the existing checkers, and it is a
three-way `FootprintExport` rather than an `Option`: an ordinary memory range
projects onto `AccessFootprint`; a bit-field unit returns
`SharedMemoryLocation { unit }` because exporting its interval alone would lose
the memory-location fact; a descriptor returns `NotMemory`. Silently exporting a
bit-field interval into `check_concurrent` would let the older checker call two
bit-fields of one unit unrelated, which is exactly the defect `Q661` exists to
prevent.

---

## 6. What stays a premise

Three, each named on the certificate and each emitted only when its declaration
is actually used:

| Assumption | What is assumed |
|---|---|
| `AllocationExtentDeclared` | the declared element size and count are the real object extent |
| `DeclaredRecordLayout` | a used layout contract is the target ABI's real layout — offsets, sizes, and which members share a bit-field storage unit |
| `DeclaredProtocolCellIdentity` | a declared protocol cell really is the shared logical state, and no undeclared descriptor also advances it |

`DeclaredDisjointness` is the premise the profile discharges and is never
emitted.

Also outside every statement here: source-to-model extraction, threads,
schedules, the C memory model, the allocator, atomic reclamation, and any claim
that the *set* of declared descriptors over a cell is complete.

---

## 7. What this does not do

* It is **model-level only**. No source syntax, no lowering, no production audit
  route, no CT case mapped, no raw Cycle 19 unit promoted, no diagnostic of any
  existing pass changed.
* It does not compose with `CheckedMutexInvariantV1`, so
  `DeclaredFootprintShape` is **not** discharged by this card. §8 F3 is that
  unit.
* Slices derive from an allocation only. Sub-slicing a slice, and the algebraic
  laws a partition would need (`slice_of_slice`, associativity of splitting),
  are follow-on F1.
* Nothing here reads a real C record. The layout numbers are declared.
* `qcore-schemas` is untouched, so a footprint or layout contract has no wire
  record; F5 specifies one.

---

## 8. Follow-on implementation units

| Unit | Scope | Blocked on |
|---|---|---|
| **F1 — source slice surface** | `slice a[lo..hi]` typing and lowering onto this checker; sub-slicing and the partition laws; captures that lend two slices to two children. **Partly landed, elsewhere:** card `CT07-SLICE-PERMISSIONS` gives `source.string.v1` a **read** sub-range loan (`borrow_utf8(b, lo, hi)`) with an exact disjointness decision and sub-slicing that narrows the loan, but it lands on the *sequential* route's own window rather than on this checker, and it lends no write and crosses no task boundary. So CT07's read half is now expressible — and since wave 29 so is the "mutated independently" half, on the same sequential route: card `SLICE-PERMISSIONS-WRITE-LEND` lands `lend(b, lo, hi)` with write authority, the exclusive-overlap rule and rebasing decided by the ownership IR's new range-carrying borrow node (`docs/reports/slice-permissions-write-lend/README.md`). What still does not exist is the *task-crossing* capture this checker is for | T04's loan states; a Buffer write statement (T04 §7); the wave-29 lend for the sequential mutating half — a capture lending two slices to two children remains open |
| **F2 — real record layout import** | `qcore-c-gateway` derives a `LayoutContract` from the Clang typed AST for one target, replacing declared numbers; `DeclaredRecordLayout` narrows to the importer's own premise | G02's typed transport; a pinned target triple |
| **F3 — composition with `CheckedMutexInvariantV1`** | a mutex's `protects` list is supplied as derived handles rather than declared intervals; `DeclaredFootprintShape` narrows to the three premises of §6 for the derived shapes, and stays in force for the rest | this card; an agreed `AllocationId ↔ SharedResourceId` identity rule. **Landed at model level, card `CG18-FOOTPRINT-CONNECTION` (wave 48):** `qcore_core::footprint_mutex` composes the two checkers unedited under the identity rule *a resource is the allocation whose live handle exports exactly the named interval*; the composition is total or refused (every named interval derived, else `Q665`; every pair of two resources both declared fresh, else `Q662`), so `DeclaredFootprintShape` is dropped only when nothing is left declared, and moves to the footprint premises. `source.mutex.guard.v1` reaches it (`qcore_frontend::source_mutex_footprint`) without changing the route's own certificate ([report](../reports/cg18-footprint-connection/README.md)) |
| **F4 — protocol-cell derivation** | derive cell identity for `dup`/`fdopen`/chunk-handle aliases from source instead of declaring it; connect to `qcore-core::event_reactor`'s descriptor model. **Partly landed, elsewhere:** card `CT42-PROTOCOL-CELL` (wave 31) derives the **connection cell** of an admitted `source.socket.v1` program — `qcore_core::socket_footprint` reads the pairing off the transport model's own decided facts, projects one `DeclareDescriptorAlias` per socket onto this checker unedited, and the two ends of a completed connection are refused as `SharedProtocolCell` (`Q664`) while the listener decides disjoint (`docs/reports/ct42-protocol-cell/README.md`). What still does not exist is the dup-like alias this row was written for: no source form creates two names over one descriptor slot's open description, so `dup`/`fdopen`/chunk-handle derivation and the `event_reactor` connection remain open | a descriptor-alias source importer |
| **F5 — schema record** | a `qcore-schemas` record for footprints, layout contracts and cells, so a process provider can propose a derivation for trusted replay | a major-version bump under the documented policy; a consumer |
| **F6 — CT integration** | fixtures and manifest entries for CT07/CT08/CT09/CT10/CT42 | F1 (CT07–CT10 need executed source stages); F4 (CT42) |

CT candidates are recorded in the evidence report and are **not** applied:
`docs/reevaluation/ct_manifest.json` stays at 15/50 mapped, because no case here
has executed source-stage evidence.
