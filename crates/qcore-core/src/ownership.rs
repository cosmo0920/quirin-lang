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

use std::collections::BTreeMap;
use std::fmt;

/// A symbolic identity allocated to one tracked resource acquisition.
pub type ResourceId = u64;

/// An exact, componentwise live-resource quantity.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Footprint(BTreeMap<String, u64>);

impl Footprint {
    pub fn zero() -> Self {
        Self::default()
    }

    pub fn single(axis: impl Into<String>, amount: u64) -> Self {
        let mut axes = BTreeMap::new();
        if amount != 0 {
            axes.insert(axis.into(), amount);
        }
        Self(axes)
    }

    pub fn from_axes(
        axes: impl IntoIterator<Item = (String, u64)>,
    ) -> Result<Self, OwnershipArithmeticError> {
        let mut result = Self::zero();
        for (axis, amount) in axes {
            let old = result.get(&axis);
            let new = old
                .checked_add(amount)
                .ok_or(OwnershipArithmeticError::Overflow)?;
            if new != 0 {
                result.0.insert(axis, new);
            }
        }
        Ok(result)
    }

    pub fn get(&self, axis: &str) -> u64 {
        self.0.get(axis).copied().unwrap_or(0)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&str, u64)> {
        self.0.iter().map(|(axis, amount)| (axis.as_str(), *amount))
    }

    fn checked_add(&self, other: &Self) -> Result<Self, OwnershipArithmeticError> {
        let mut result = self.clone();
        for (axis, amount) in &other.0 {
            let sum = result
                .get(axis)
                .checked_add(*amount)
                .ok_or(OwnershipArithmeticError::Overflow)?;
            if sum == 0 {
                result.0.remove(axis);
            } else {
                result.0.insert(axis.clone(), sum);
            }
        }
        Ok(result)
    }

    fn checked_sub(&self, other: &Self) -> Result<Self, OwnershipArithmeticError> {
        let mut result = self.clone();
        for (axis, amount) in &other.0 {
            let remainder = result
                .get(axis)
                .checked_sub(*amount)
                .ok_or_else(|| OwnershipArithmeticError::Underflow(axis.clone()))?;
            if remainder == 0 {
                result.0.remove(axis);
            } else {
                result.0.insert(axis.clone(), remainder);
            }
        }
        Ok(result)
    }
}

/// A field of a named aggregate, at any depth — the owner path
/// `BASE.FIELD.….FIELD` (card `AGGREGATE-OWNED-FIELD`, generalised by card
/// `W48-QR-USABLE-RECORDS`).
///
/// **This is the field-path owner vocabulary constraint 15k recorded as
/// missing.** Before card `AGGREGATE-OWNED-FIELD` every owner-addressing field
/// of this IR was a plain `String`, so `p.buf` had nothing to be spelled into
/// and [`check_ownership`] could not be *asked* about a field. That card made
/// the path one level deep — a name and one field — measured sufficient then
/// and recorded as the boundary `AGGREGATE-NESTED`. Card
/// `W48-QR-USABLE-RECORDS` generalises **this** type rather than adding a
/// second path type beside it: `fields` is the non-empty sequence of field
/// names from the base down, so `o.inner.buf` is one path and a one-level path
/// is exactly the path it always was. It is a **type** rather than a spelling
/// convention: a plain `String` owner in an existing operation is always
/// [`Place::Name`], so no name containing a `.` can be mistaken for a path and
/// no path can be forged by spelling one.
///
/// The derived order is the order the one-level type had for one-level paths
/// (base, then field), which is what keeps [`OwnershipError::NormalPathLeak`]'s
/// published list identical for every program written before the card.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct FieldPath {
    pub base: String,
    /// The field names from the base down; never empty.
    pub fields: Vec<String>,
}

impl FieldPath {
    /// The one-level path `BASE.FIELD`.
    #[must_use]
    pub fn new(base: impl Into<String>, field: impl Into<String>) -> Self {
        Self {
            base: base.into(),
            fields: vec![field.into()],
        }
    }

    /// The path `BASE.F1.….Fn` (card `W48-QR-USABLE-RECORDS`).
    ///
    /// # Panics
    ///
    /// When `fields` is empty: a path names at least one field, and an empty
    /// one would be the base itself spelled as a field.
    #[must_use]
    pub fn nested(base: impl Into<String>, fields: Vec<String>) -> Self {
        assert!(!fields.is_empty(), "a field path names at least one field");
        Self {
            base: base.into(),
            fields,
        }
    }

    /// The `BASE.FIELD` spelling a diagnostic names the path by.
    #[must_use]
    pub fn spelling(&self) -> String {
        format!("{}.{}", self.base, self.fields.join("."))
    }
}

/// An owner place of the cycle-3 IR: a plain binding name, or one field of a
/// named aggregate (card `AGGREGATE-OWNED-FIELD`).
///
/// Every operation this IR carried before the card names its owners by
/// `String`, and every one of those is a [`Self::Name`]: the binding map is
/// keyed by this type so that a field owner and a plain owner cannot collide,
/// and so that the ledger's owner column can point at a field. The variant
/// order is load-bearing for one published list only — [`OwnershipError::NormalPathLeak`]
/// names plain owners before field owners — and for a program that composes no
/// aggregate the list is exactly the list it always was.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Place {
    Name(String),
    Field(FieldPath),
}

impl Place {
    fn name(name: &str) -> Self {
        Self::Name(name.to_owned())
    }

    /// The spelling a diagnostic names this place by: the name itself, or
    /// `BASE.FIELD`.
    #[must_use]
    pub fn spelling(&self) -> String {
        match self {
            Self::Name(name) => name.clone(),
            Self::Field(path) => path.spelling(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResourceEntry {
    id: ResourceId,
    resource_type: String,
    footprint: Footprint,
    owner: Place,
    acquisition_order: u64,
}

impl ResourceEntry {
    pub fn id(&self) -> ResourceId {
        self.id
    }

    pub fn resource_type(&self) -> &str {
        &self.resource_type
    }

    pub fn footprint(&self) -> &Footprint {
        &self.footprint
    }

    /// The place that owns this resource: a plain binding, or — since card
    /// `AGGREGATE-OWNED-FIELD` — one field of an aggregate.
    pub fn owner(&self) -> &Place {
        &self.owner
    }

    /// [`Self::owner`]'s spelling, for the readers that key by it.
    #[must_use]
    pub fn owner_spelling(&self) -> String {
        self.owner.spelling()
    }

    pub fn acquisition_order(&self) -> u64 {
        self.acquisition_order
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum BorrowKind {
    Shared,
    Mutable,
    /// **Write authority over `[low, high)` of the resource's footprint on
    /// `axis`, genuinely transferred** (card `SLICE-PERMISSIONS-WRITE-LEND`).
    ///
    /// This is the footprint fraction card `STD-BUFFER-SLICE` predicted a
    /// borrow node would need, carried as a half-open interval rather than an
    /// amount because the question it answers — do two lends conflict? — is
    /// about *which* bytes, not how many. While such a loan is live the owner
    /// keeps authority over every byte outside `[low, high)` and none inside
    /// it, which is what distinguishes a lend from the read exclusion card
    /// `CT07-SLICE-PERMISSIONS` landed: there the owner *kept* the write
    /// authority and the route excluded its use; here the authority itself
    /// moves for the loan's lexical extent.
    MutableRange {
        axis: String,
        low: u64,
        high: u64,
    },
}

/// Do the half-open intervals `[low_a, high_a)` and `[low_b, high_b)` share a
/// point? The same decision `qcore_core::std_string::ranges_overlap` publishes
/// for `u32` bounds, restated on the ownership IR's own `u64` axis so the two
/// layers cannot drift by a conversion. An empty interval overlaps nothing.
///
/// Public since card `WRITE-LEND-TASKS`: the scoped-task ledger's range write
/// leases (`crate::scoped_tasks`) decide their exclusive-overlap rule by
/// **calling this function** rather than restating it, so the sequential
/// route's lend and the task-crossing lease cannot drift apart. The rule
/// itself still lives here, in the IR, per the amended constraint 15h.
pub fn axes_overlap(low_a: u64, high_a: u64, low_b: u64, high_b: u64) -> bool {
    low_a.max(low_b) < high_a.min(high_b)
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum Binding {
    Owned(ResourceId),
    Borrowed {
        resource: ResourceId,
        kind: BorrowKind,
    },
    Moved,
    Dropped,
    /// A named aggregate whose owned fields, in declaration order, are the
    /// [`Place::Field`] bindings `BASE.FIELD` beside it (card
    /// `AGGREGATE-OWNED-FIELD`). The aggregate itself owns no resource: each
    /// field does, and the aggregate is what a whole-record `Drop` walks.
    ///
    /// Card `W48-QR-USABLE-RECORDS`: `fields` are the owned **leaves**, each a
    /// path relative to the base, in declaration order at every level
    /// (depth-first), so a record holding a record holding a buffer lists
    /// `["inner", "buf"]`. A one-level aggregate lists one-element paths,
    /// exactly the fields it always listed.
    Aggregate {
        fields: Vec<Vec<String>>,
    },
}

/// One direct field of a nested composition (card `W48-QR-USABLE-RECORDS`):
/// an owned buffer moved into an owned field, or a whole aggregate moved into
/// a field whose type is a record with an owned field.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ComposePart {
    /// The buffer `from` owns moves into `OWNER.field`.
    Buffer { field: String, from: String },
    /// The aggregate `from` moves under `OWNER.field`: each of its owned leaves
    /// `from.L` — live, moved out or released — becomes `OWNER.field.L` in the
    /// same state, and `from` is moved.
    Record { field: String, from: String },
}

/// Reduced cycle-3 ownership IR. Borrow nodes delimit lexical loan lifetimes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OwnershipOp {
    Acquire {
        id: ResourceId,
        owner: String,
        resource_type: String,
        footprint: Footprint,
    },
    Read(String),
    Write(String),
    Move {
        from: String,
        to: String,
    },
    Copy(String),
    Drop(String),
    /// Transfer an owned value to the caller or another enclosing owner.
    TransferOut(String),
    BorrowShared {
        owner: String,
        borrower: String,
        body: Vec<OwnershipOp>,
    },
    BorrowMutable {
        owner: String,
        borrower: String,
        body: Vec<OwnershipOp>,
    },
    /// Lend write authority over `[low, high)` of `owner`'s footprint on
    /// `axis` to `borrower` for the lexical extent of `body` (card
    /// `SLICE-PERMISSIONS-WRITE-LEND`).
    ///
    /// Distinct from [`Self::BorrowMutable`] in exactly one way: a second live
    /// lend of the same resource is **admitted when its range is disjoint**
    /// from every live loan's and refused when it is not, so two parties can
    /// hold write authority over two disjoint ranges at once. A whole-resource
    /// borrow of either kind still conflicts with any non-empty lend, and any
    /// lend with it.
    ///
    /// The lend's bound against the ledger footprint (`high <= extent`) is
    /// deliberately **not** decided here: the admitting route decides it at
    /// check time against the extent the acquisition declared, exactly as the
    /// read card decided its range, so one fault produces one refusal.
    LendRange {
        owner: String,
        borrower: String,
        axis: String,
        low: u64,
        high: u64,
        body: Vec<OwnershipOp>,
    },
    /// A write whose footprint the emitting route resolved to `[low, high)` on
    /// `axis` — through the owner, refused exactly when the interval shares a
    /// point with a live loan's; through a range lend, admitted exactly when
    /// it stays inside that lend's own range (the rebasing law, enforced in
    /// the IR rather than trusted). A write the route cannot resolve stays a
    /// rangeless [`Self::Write`], which conflicts with everything — the
    /// fail-closed direction.
    WriteRange {
        place: String,
        axis: String,
        low: u64,
        high: u64,
    },
    /// The read counterpart of [`Self::WriteRange`]. An empty interval is a
    /// read that touches no byte (an extent query), which is why it is
    /// admitted beside any loan while still checking the binding is live.
    ReadRange {
        place: String,
        axis: String,
        low: u64,
        high: u64,
    },
    Branch {
        then_ops: Vec<OwnershipOp>,
        else_ops: Vec<OwnershipOp>,
    },
    /// Bind `owner` as an aggregate whose owned fields take custody of the
    /// named bindings **by move**, in the order given — the declaration order
    /// the admitting route publishes (card `AGGREGATE-OWNED-FIELD`).
    ///
    /// The construction rule of the four: each `(field, from)` moves the
    /// resource `from` owns into the field owner `owner.field`, exactly as a
    /// [`Self::Move`] would into a plain name, so `from` is [`Binding::Moved`]
    /// afterwards and a later use of it refuses. A binding composed into two
    /// fields therefore refuses at the second (`UseAfterMove`), and a field
    /// named twice refuses as a rebound live name. An aggregate that composes
    /// **no** owned field is a shape no route emits — a record of scalars
    /// contributes no operation at all, which is what keeps every program
    /// written before the card byte-identical — but the IR admits it as the
    /// degenerate aggregate: its `Drop` releases nothing and its `Copy` still
    /// refuses.
    Compose {
        owner: String,
        fields: Vec<(String, String)>,
    },
    /// A read through an owned field: [`Self::Read`] at a path.
    ReadField(FieldPath),
    /// A write through an owned field: [`Self::Write`] at a path.
    WriteField(FieldPath),
    /// **The partial-move rule.** The field's resource leaves the aggregate for
    /// the plain binding `to`; the field is a hole ([`Binding::Moved`]) that a
    /// later whole-record `Drop` tolerates and a later access refuses.
    MoveOut {
        from: FieldPath,
        to: String,
    },
    /// A release of one owned field, leaving a hole the whole-record `Drop`
    /// tolerates; a second release of it is a `DoubleDrop`.
    DropField(FieldPath),
    /// **The nested construction rule** (card `W48-QR-USABLE-RECORDS`):
    /// [`Self::Compose`] generalised to a field whose type is a record with an
    /// owned field. Bind `owner` as an aggregate built from `parts`, in the
    /// order given (the declaration order the admitting route publishes); a
    /// [`ComposePart::Buffer`] is exactly `Compose`'s move, and a
    /// [`ComposePart::Record`] moves a whole aggregate under the field, its
    /// leaves keeping their states. `Compose { owner, fields }` is this
    /// operation with every part a buffer, and is decided by the same code.
    ComposeNested {
        owner: String,
        parts: Vec<ComposePart>,
    },
    /// **The refill rule** (card `W48-QR-USABLE-RECORDS`): a write of a whole
    /// owned field. The buffer `from` owns moves into the leaf `into`, which
    /// must be a **hole** — moved out or released — of a live aggregate: a
    /// write over a field still holding its buffer would lose that buffer's
    /// release duty, and is refused (`OverwriteOfLiveField`). The only move
    /// *into* a field besides the construction's.
    RefillField {
        from: String,
        into: FieldPath,
    },
    /// **The live-record rule** (card `W48-QR-USABLE-RECORDS`): a read or
    /// write of a **scalar** field of an aggregate. Owns nothing and moves
    /// nothing; it asks only that the aggregate is still live — a scalar field
    /// of a record that was dropped or moved is a use after drop or move, like
    /// every other access to it.
    AccessRecord(String),
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct OwnershipState {
    bindings: BTreeMap<Place, Binding>,
    ledger: BTreeMap<ResourceId, ResourceEntry>,
    live: Footprint,
    next_acquisition_order: u64,
}

impl OwnershipState {
    pub fn live(&self) -> &Footprint {
        &self.live
    }

    pub fn ledger(&self) -> impl Iterator<Item = &ResourceEntry> {
        self.ledger.values()
    }

    pub fn owner_resource(&self, owner: &str) -> Option<ResourceId> {
        match self.bindings.get(&Place::name(owner)) {
            Some(Binding::Owned(resource)) => Some(*resource),
            _ => None,
        }
    }

    /// The resource an owned field holds, if the field is live (card
    /// `AGGREGATE-OWNED-FIELD`).
    #[must_use]
    pub fn field_resource(&self, path: &FieldPath) -> Option<ResourceId> {
        match self.bindings.get(&Place::Field(path.clone())) {
            Some(Binding::Owned(resource)) => Some(*resource),
            _ => None,
        }
    }

    /// The owned fields of aggregate `owner` in declaration order, if `owner`
    /// is an aggregate binding — since card `W48-QR-USABLE-RECORDS` its owned
    /// **leaves**, each spelled relative to `owner` (`inner.buf`).
    #[must_use]
    pub fn aggregate_fields(&self, owner: &str) -> Option<Vec<String>> {
        match self.bindings.get(&Place::name(owner)) {
            Some(Binding::Aggregate { fields }) => {
                Some(fields.iter().map(|leaf| leaf.join(".")).collect())
            }
            _ => None,
        }
    }

    pub fn validate_invariant(&self) -> Result<(), OwnershipError> {
        let mut aggregate = Footprint::zero();
        for entry in self.ledger.values() {
            aggregate = aggregate.checked_add(&entry.footprint)?;
            if self.bindings.get(&entry.owner) != Some(&Binding::Owned(entry.id)) {
                return Err(OwnershipError::LedgerInvariantViolation);
            }
        }
        if aggregate != self.live {
            return Err(OwnershipError::LedgerInvariantViolation);
        }
        for (place, binding) in &self.bindings {
            match binding {
                Binding::Owned(resource) => {
                    let Some(entry) = self.ledger.get(resource) else {
                        return Err(OwnershipError::LedgerInvariantViolation);
                    };
                    if entry.owner != *place {
                        return Err(OwnershipError::LedgerInvariantViolation);
                    }
                }
                Binding::Borrowed { resource, .. } => {
                    if !self.ledger.contains_key(resource) {
                        return Err(OwnershipError::LedgerInvariantViolation);
                    }
                }
                // Every declared owned field of an aggregate has a field
                // binding beside it — live, moved out or released — and an
                // aggregate is never itself a field.
                Binding::Aggregate { fields } => {
                    let Place::Name(base) = place else {
                        return Err(OwnershipError::LedgerInvariantViolation);
                    };
                    for field in fields {
                        let key = Place::Field(FieldPath::nested(base.clone(), field.clone()));
                        if !matches!(
                            self.bindings.get(&key),
                            Some(Binding::Owned(_) | Binding::Moved | Binding::Dropped)
                        ) {
                            return Err(OwnershipError::LedgerInvariantViolation);
                        }
                    }
                }
                Binding::Moved | Binding::Dropped => {}
            }
        }
        Ok(())
    }

    fn execute(&mut self, operations: &[OwnershipOp]) -> Result<(), OwnershipError> {
        for operation in operations {
            self.execute_one(operation)?;
            self.validate_invariant()?;
        }
        Ok(())
    }

    fn execute_one(&mut self, operation: &OwnershipOp) -> Result<(), OwnershipError> {
        match operation {
            OwnershipOp::Acquire {
                id,
                owner,
                resource_type,
                footprint,
            } => self.acquire(*id, owner, resource_type, footprint),
            OwnershipOp::Read(place) => self.read(&Place::name(place)),
            OwnershipOp::Write(place) => self.write(&Place::name(place)),
            OwnershipOp::Move { from, to } => self.move_owned(&Place::name(from), &Place::name(to)),
            OwnershipOp::Copy(place) => self.copy_owned(&Place::name(place)),
            OwnershipOp::Drop(place) => self.release(&Place::name(place), false),
            OwnershipOp::TransferOut(place) => self.release(&Place::name(place), true),
            OwnershipOp::BorrowShared {
                owner,
                borrower,
                body,
            } => self.borrow(owner, borrower, BorrowKind::Shared, body),
            OwnershipOp::BorrowMutable {
                owner,
                borrower,
                body,
            } => self.borrow(owner, borrower, BorrowKind::Mutable, body),
            OwnershipOp::LendRange {
                owner,
                borrower,
                axis,
                low,
                high,
                body,
            } => self.lend_range(owner, borrower, axis, *low, *high, body),
            OwnershipOp::WriteRange {
                place,
                axis,
                low,
                high,
            } => self.write_range(place, axis, *low, *high),
            OwnershipOp::ReadRange {
                place,
                axis,
                low,
                high,
            } => self.read_range(place, axis, *low, *high),
            OwnershipOp::Branch { then_ops, else_ops } => self.branch(then_ops, else_ops),
            OwnershipOp::Compose { owner, fields } => self.compose(owner, fields),
            OwnershipOp::ReadField(path) => self.read(&Place::Field(path.clone())),
            OwnershipOp::WriteField(path) => self.write(&Place::Field(path.clone())),
            OwnershipOp::MoveOut { from, to } => {
                self.move_owned(&Place::Field(from.clone()), &Place::name(to))
            }
            OwnershipOp::DropField(path) => self.release(&Place::Field(path.clone()), false),
            OwnershipOp::ComposeNested { owner, parts } => self.compose_parts(owner, parts),
            OwnershipOp::RefillField { from, into } => self.refill(from, into),
            OwnershipOp::AccessRecord(base) => self.access_record(base),
        }
    }

    fn acquire(
        &mut self,
        id: ResourceId,
        owner: &str,
        resource_type: &str,
        footprint: &Footprint,
    ) -> Result<(), OwnershipError> {
        let owner = Place::name(owner);
        if self.bindings.contains_key(&owner) {
            return Err(OwnershipError::BindingAlreadyExists(owner.spelling()));
        }
        if self.ledger.contains_key(&id) {
            return Err(OwnershipError::DuplicateResourceId(id));
        }
        let next_live = self.live.checked_add(footprint)?;
        let acquisition_order = self.next_acquisition_order;
        self.next_acquisition_order = self
            .next_acquisition_order
            .checked_add(1)
            .ok_or(OwnershipArithmeticError::Overflow)?;
        self.ledger.insert(
            id,
            ResourceEntry {
                id,
                resource_type: resource_type.into(),
                footprint: footprint.clone(),
                owner: owner.clone(),
                acquisition_order,
            },
        );
        self.bindings.insert(owner, Binding::Owned(id));
        self.live = next_live;
        Ok(())
    }

    /// **The construction rule** (card `AGGREGATE-OWNED-FIELD`): see
    /// [`OwnershipOp::Compose`]. Every precondition is checked before any
    /// binding moves, so a refused composition names its one fault.
    fn compose(&mut self, owner: &str, fields: &[(String, String)]) -> Result<(), OwnershipError> {
        let parts: Vec<ComposePart> = fields
            .iter()
            .map(|(field, from)| ComposePart::Buffer {
                field: field.clone(),
                from: from.clone(),
            })
            .collect();
        self.compose_parts(owner, &parts)
    }

    /// **The construction rule, over a tree of fields** (card
    /// `W48-QR-USABLE-RECORDS`): see [`OwnershipOp::ComposeNested`]. A buffer
    /// part is the one-level move [`OwnershipOp::Compose`] always made; a
    /// record part moves every leaf of an aggregate under the field, keeping
    /// each leaf's state, so a hole stays a hole and a live buffer stays live
    /// with its ledger owner re-keyed to the new path. Every precondition of
    /// every part is checked before any binding moves, so a refused
    /// composition names its one fault and changes nothing.
    fn compose_parts(&mut self, owner: &str, parts: &[ComposePart]) -> Result<(), OwnershipError> {
        let base = Place::name(owner);
        if self.bindings.contains_key(&base) {
            return Err(OwnershipError::BindingAlreadyExists(owner.into()));
        }
        // (the owner's leaf, the source place it comes from, the source's
        // binding) for every leaf this composition moves, plus the sources.
        let mut moves: Vec<(Vec<String>, Place, Binding)> = Vec::new();
        let mut sources: Vec<&str> = Vec::with_capacity(parts.len());
        let mut named: Vec<&str> = Vec::with_capacity(parts.len());
        for part in parts {
            let (field, from) = match part {
                ComposePart::Buffer { field, from } | ComposePart::Record { field, from } => {
                    (field, from)
                }
            };
            if named.contains(&field.as_str()) {
                return Err(OwnershipError::BindingAlreadyExists(
                    FieldPath::new(owner, field.clone()).spelling(),
                ));
            }
            named.push(field);
            // A source moved twice refuses at its second occurrence exactly as
            // a second `Move` of it would: the first move is recorded below,
            // so here the check is against the sources already taken.
            if sources.contains(&from.as_str()) {
                return Err(OwnershipError::UseAfterMove(from.clone()));
            }
            sources.push(from);
            match part {
                ComposePart::Buffer { .. } => {
                    let resource = self.require_owned(&Place::name(from))?;
                    if self.has_any_borrow(resource) {
                        return Err(OwnershipError::MoveWhileBorrowed(from.clone()));
                    }
                    moves.push((
                        vec![field.clone()],
                        Place::name(from),
                        Binding::Owned(resource),
                    ));
                }
                ComposePart::Record { .. } => {
                    let leaves = match self.binding(&Place::name(from))? {
                        Binding::Aggregate { fields } => fields.clone(),
                        Binding::Moved => return Err(OwnershipError::UseAfterMove(from.clone())),
                        Binding::Dropped => return Err(OwnershipError::UseAfterDrop(from.clone())),
                        Binding::Borrowed { .. } => {
                            return Err(OwnershipError::MoveOfBorrow(from.clone()));
                        }
                        Binding::Owned(_) => {
                            return Err(OwnershipError::NotAnAggregate(from.clone()));
                        }
                    };
                    for leaf in leaves {
                        let place = Place::Field(FieldPath::nested(from.clone(), leaf.clone()));
                        let binding = self
                            .bindings
                            .get(&place)
                            .cloned()
                            .ok_or(OwnershipError::LedgerInvariantViolation)?;
                        if let Binding::Owned(resource) = binding
                            && self.has_any_borrow(resource)
                        {
                            return Err(OwnershipError::MoveWhileBorrowed(place.spelling()));
                        }
                        let mut path = vec![field.clone()];
                        path.extend(leaf);
                        moves.push((path, place, binding));
                    }
                }
            }
        }
        for (path, source, binding) in &moves {
            let place = Place::Field(FieldPath::nested(owner, path.clone()));
            if let Binding::Owned(resource) = binding {
                let entry = self
                    .ledger
                    .get_mut(resource)
                    .ok_or(OwnershipError::LedgerInvariantViolation)?;
                entry.owner = place.clone();
            }
            // The source keeps no custody: a plain buffer and every leaf of a
            // moved aggregate are `Moved`, so a later use of either is a use
            // after move.
            self.bindings.insert(source.clone(), Binding::Moved);
            self.bindings.insert(place, binding.clone());
        }
        for part in parts {
            if let ComposePart::Record { from, .. } = part {
                self.bindings.insert(Place::name(from), Binding::Moved);
            }
        }
        self.bindings.insert(
            base,
            Binding::Aggregate {
                fields: moves.into_iter().map(|(path, _, _)| path).collect(),
            },
        );
        Ok(())
    }

    /// **The refill rule** (card `W48-QR-USABLE-RECORDS`): see
    /// [`OwnershipOp::RefillField`].
    fn refill(&mut self, from: &str, into: &FieldPath) -> Result<(), OwnershipError> {
        let leaves = match self.binding(&Place::name(&into.base))? {
            Binding::Aggregate { fields } => fields.clone(),
            Binding::Moved => return Err(OwnershipError::UseAfterMove(into.base.clone())),
            Binding::Dropped => return Err(OwnershipError::UseAfterDrop(into.base.clone())),
            _ => return Err(OwnershipError::NotAnAggregate(into.base.clone())),
        };
        if !leaves.contains(&into.fields) {
            return Err(OwnershipError::UnknownBinding(into.spelling()));
        }
        let place = Place::Field(into.clone());
        match self.bindings.get(&place) {
            Some(Binding::Moved | Binding::Dropped) => {}
            Some(Binding::Owned(_)) => {
                return Err(OwnershipError::OverwriteOfLiveField(into.spelling()));
            }
            _ => return Err(OwnershipError::LedgerInvariantViolation),
        }
        let resource = self.require_owned(&Place::name(from))?;
        if self.has_any_borrow(resource) {
            return Err(OwnershipError::MoveWhileBorrowed(from.to_owned()));
        }
        let entry = self
            .ledger
            .get_mut(&resource)
            .ok_or(OwnershipError::LedgerInvariantViolation)?;
        entry.owner = place.clone();
        self.bindings.insert(Place::name(from), Binding::Moved);
        self.bindings.insert(place, Binding::Owned(resource));
        Ok(())
    }

    /// **The live-record rule** (card `W48-QR-USABLE-RECORDS`): see
    /// [`OwnershipOp::AccessRecord`].
    fn access_record(&self, base: &str) -> Result<(), OwnershipError> {
        match self.binding(&Place::name(base))? {
            Binding::Aggregate { .. } => Ok(()),
            Binding::Moved => Err(OwnershipError::UseAfterMove(base.to_owned())),
            Binding::Dropped => Err(OwnershipError::UseAfterDrop(base.to_owned())),
            Binding::Owned(_) | Binding::Borrowed { .. } => {
                Err(OwnershipError::NotAnAggregate(base.to_owned()))
            }
        }
    }

    fn read(&self, place: &Place) -> Result<(), OwnershipError> {
        match self.binding(place)? {
            Binding::Owned(resource) => {
                if self.has_mutable_borrow(*resource) {
                    Err(OwnershipError::OwnerAccessWhileMutablyBorrowed(
                        place.spelling(),
                    ))
                } else {
                    Ok(())
                }
            }
            // A rangeless access through a **range** lend could name any byte,
            // including one outside the lent range, so it is refused: a route
            // that holds a range lend states every access's footprint or does
            // not access at all — the fail-closed direction.
            Binding::Borrowed {
                kind: BorrowKind::MutableRange { .. },
                ..
            } => Err(OwnershipError::AccessOutsideLendRange(place.spelling())),
            Binding::Borrowed { .. } => Ok(()),
            Binding::Moved => Err(OwnershipError::UseAfterMove(place.spelling())),
            Binding::Dropped => Err(OwnershipError::UseAfterDrop(place.spelling())),
            // An aggregate is addressed field by field; the record itself is
            // not an owner a read can name.
            Binding::Aggregate { .. } => {
                Err(OwnershipError::AggregateAtOwnerPosition(place.spelling()))
            }
        }
    }

    fn write(&self, place: &Place) -> Result<(), OwnershipError> {
        match self.binding(place)? {
            Binding::Owned(resource) => {
                if self.has_any_borrow(*resource) {
                    Err(OwnershipError::OwnerMutationWhileBorrowed(place.spelling()))
                } else {
                    Ok(())
                }
            }
            Binding::Borrowed {
                kind: BorrowKind::Mutable,
                ..
            } => Ok(()),
            // The rangeless-write rule of the range lend: see `read` above.
            Binding::Borrowed {
                kind: BorrowKind::MutableRange { .. },
                ..
            } => Err(OwnershipError::AccessOutsideLendRange(place.spelling())),
            Binding::Borrowed {
                kind: BorrowKind::Shared,
                ..
            } => Err(OwnershipError::WriteThroughSharedBorrow(place.spelling())),
            Binding::Moved => Err(OwnershipError::UseAfterMove(place.spelling())),
            Binding::Dropped => Err(OwnershipError::UseAfterDrop(place.spelling())),
            Binding::Aggregate { .. } => {
                Err(OwnershipError::AggregateAtOwnerPosition(place.spelling()))
            }
        }
    }

    /// A move between plain owners, and — since card `AGGREGATE-OWNED-FIELD`
    /// — the **partial move** out of an owned field: `from` may be a field
    /// path, in which case the field becomes a hole the whole-record drop
    /// tolerates. `to` is always a plain name: a move *into* a field is the
    /// construction rule's alone (`Compose`), because v1 has no field write.
    fn move_owned(&mut self, from: &Place, to: &Place) -> Result<(), OwnershipError> {
        if self.bindings.contains_key(to) {
            return Err(OwnershipError::BindingAlreadyExists(to.spelling()));
        }
        let resource = self.require_owned(from)?;
        if self.has_any_borrow(resource) {
            return Err(OwnershipError::MoveWhileBorrowed(from.spelling()));
        }
        let entry = self
            .ledger
            .get_mut(&resource)
            .ok_or(OwnershipError::LedgerInvariantViolation)?;
        entry.owner = to.clone();
        self.bindings.insert(from.clone(), Binding::Moved);
        self.bindings.insert(to.clone(), Binding::Owned(resource));
        Ok(())
    }

    /// **The copy rule.** An owned resource is never copied, and — card
    /// `AGGREGATE-OWNED-FIELD` — neither is an aggregate that declares an
    /// owned field, whatever the state of its fields: the refusal is about the
    /// type, and a record whose every field has been moved out is still not a
    /// value two bindings may hold.
    fn copy_owned(&self, place: &Place) -> Result<(), OwnershipError> {
        match self.binding(place)? {
            Binding::Owned(_) | Binding::Aggregate { .. } => {
                Err(OwnershipError::CopyOfOwned(place.spelling()))
            }
            Binding::Borrowed { .. } => Err(OwnershipError::MoveOfBorrow(place.spelling())),
            Binding::Moved => Err(OwnershipError::UseAfterMove(place.spelling())),
            Binding::Dropped => Err(OwnershipError::UseAfterDrop(place.spelling())),
        }
    }

    fn release(&mut self, place: &Place, transfer: bool) -> Result<(), OwnershipError> {
        if let Some(Binding::Aggregate { fields }) = self.bindings.get(place) {
            if transfer {
                // Transferring a whole aggregate to the caller is not a shape
                // this revision models; refused rather than read as a
                // transfer of each field.
                return Err(OwnershipError::AggregateAtOwnerPosition(place.spelling()));
            }
            let Place::Name(base) = place else {
                return Err(OwnershipError::LedgerInvariantViolation);
            };
            return self.drop_through(base, &fields.clone());
        }
        let resource = self.require_owned_for_drop(place)?;
        if self.has_any_borrow(resource) {
            return Err(OwnershipError::DropWhileBorrowed(place.spelling()));
        }
        let entry = self
            .ledger
            .remove(&resource)
            .ok_or(OwnershipError::LedgerInvariantViolation)?;
        self.live = self.live.checked_sub(&entry.footprint)?;
        self.bindings.insert(
            place.clone(),
            if transfer {
                Binding::Moved
            } else {
                Binding::Dropped
            },
        );
        Ok(())
    }

    /// **The drop-through-aggregate rule** (card `AGGREGATE-OWNED-FIELD`).
    ///
    /// `drop p;` releases every **live** owned field of `p`, in declaration
    /// order, and nothing twice: a field already moved out or released is a
    /// hole, tolerated rather than refused, because the hole is exactly what
    /// the partial-move rule left behind and the reader who wrote the move
    /// expects the record's remaining duties — not one more — to be
    /// discharged here. Every field is checked for a live loan before any is
    /// released, so a refused drop releases nothing. The aggregate binding is
    /// [`Binding::Dropped`] afterwards, so a second `drop p;` is a
    /// `DoubleDrop` of `p` and a later field access is a use after drop.
    /// `QCore.AggregateOwnedField.drop_through_releases_exactly_the_live_fields`
    /// is the Lean statement.
    ///
    /// Card `W48-QR-USABLE-RECORDS`: `fields` are the aggregate's owned
    /// leaves, so the walk is over the **tree** of fields — declaration order
    /// at every level, depth-first — and releases each live leaf once, however
    /// deep (`QCore.UsableRecords.drop_through_tree_releases_exactly_the_live_leaves`,
    /// over the leaf list this binding holds; the unit test
    /// `dropping_a_nested_aggregate_releases_every_live_leaf_once` runs it).
    fn drop_through(&mut self, base: &str, fields: &[Vec<String>]) -> Result<(), OwnershipError> {
        let mut releasing = Vec::new();
        for field in fields {
            let path = Place::Field(FieldPath::nested(base, field.clone()));
            match self.bindings.get(&path) {
                Some(Binding::Owned(resource)) => {
                    if self.has_any_borrow(*resource) {
                        return Err(OwnershipError::DropWhileBorrowed(path.spelling()));
                    }
                    releasing.push((path, *resource));
                }
                Some(Binding::Moved | Binding::Dropped) => {}
                _ => return Err(OwnershipError::LedgerInvariantViolation),
            }
        }
        for (path, resource) in releasing {
            let entry = self
                .ledger
                .remove(&resource)
                .ok_or(OwnershipError::LedgerInvariantViolation)?;
            self.live = self.live.checked_sub(&entry.footprint)?;
            self.bindings.insert(path, Binding::Dropped);
        }
        self.bindings.insert(Place::name(base), Binding::Dropped);
        Ok(())
    }

    fn borrow(
        &mut self,
        owner: &str,
        borrower: &str,
        kind: BorrowKind,
        body: &[OwnershipOp],
    ) -> Result<(), OwnershipError> {
        if self.bindings.contains_key(&Place::name(borrower)) {
            return Err(OwnershipError::BindingAlreadyExists(borrower.into()));
        }
        let resource = self.require_owned(&Place::name(owner))?;
        match kind {
            BorrowKind::Shared if self.has_mutable_borrow(resource) => {
                return Err(OwnershipError::SharedBorrowWhileMutablyBorrowed(
                    owner.into(),
                ));
            }
            BorrowKind::Mutable if self.has_any_borrow(resource) => {
                return Err(OwnershipError::MutableBorrowWhileBorrowed(owner.into()));
            }
            _ => {}
        }
        let loan = Binding::Borrowed { resource, kind };
        self.bindings.insert(Place::name(borrower), loan.clone());
        self.execute(body)?;
        if self.bindings.remove(&Place::name(borrower)) != Some(loan) {
            return Err(OwnershipError::LedgerInvariantViolation);
        }
        Ok(())
    }

    /// Open one write lend of `[low, high)` on `axis`, run `body` under it, and
    /// close it (card `SLICE-PERMISSIONS-WRITE-LEND`).
    ///
    /// The admission is the exclusivity rule: the new range must share no point
    /// with any live loan of the resource. Against another range lend that is
    /// `axes_overlap` on the same axis; against a whole-resource borrow of
    /// either kind it is "the lend is non-empty", because a whole borrow covers
    /// every byte. An **empty** lend conflicts with nothing — a consequence of
    /// the interval decision, not a case beside it.
    fn lend_range(
        &mut self,
        owner: &str,
        borrower: &str,
        axis: &str,
        low: u64,
        high: u64,
        body: &[OwnershipOp],
    ) -> Result<(), OwnershipError> {
        if self.bindings.contains_key(&Place::name(borrower)) {
            return Err(OwnershipError::BindingAlreadyExists(borrower.into()));
        }
        let resource = match self.binding(&Place::name(owner))? {
            Binding::Owned(resource) => *resource,
            // A loan is not an owner: re-lending is refused rather than given a
            // transitive reading this revision does not model.
            Binding::Borrowed { .. } => return Err(OwnershipError::LendOfBorrow(owner.into())),
            Binding::Moved => return Err(OwnershipError::UseAfterMove(owner.into())),
            Binding::Dropped => return Err(OwnershipError::UseAfterDrop(owner.into())),
            // A lend addresses one owner's bytes; an aggregate is not one.
            Binding::Aggregate { .. } => {
                return Err(OwnershipError::AggregateAtOwnerPosition(owner.into()));
            }
        };
        for binding in self.bindings.values() {
            let Binding::Borrowed {
                resource: loaned,
                kind,
            } = binding
            else {
                continue;
            };
            if *loaned != resource {
                continue;
            }
            match kind {
                // A whole-resource borrow holds every byte, so any non-empty
                // lend shares one with it. The refusal is the existing
                // whole-borrow conflict, because a lend *is* a mutable borrow.
                BorrowKind::Shared | BorrowKind::Mutable => {
                    if low < high {
                        return Err(OwnershipError::MutableBorrowWhileBorrowed(owner.into()));
                    }
                }
                BorrowKind::MutableRange {
                    axis: held_axis,
                    low: held_low,
                    high: held_high,
                } => {
                    if held_axis == axis && axes_overlap(low, high, *held_low, *held_high) {
                        return Err(OwnershipError::OverlappingLends(owner.into()));
                    }
                }
            }
        }
        let loan = Binding::Borrowed {
            resource,
            kind: BorrowKind::MutableRange {
                axis: axis.to_owned(),
                low,
                high,
            },
        };
        self.bindings.insert(Place::name(borrower), loan.clone());
        self.execute(body)?;
        if self.bindings.remove(&Place::name(borrower)) != Some(loan) {
            return Err(OwnershipError::LedgerInvariantViolation);
        }
        Ok(())
    }

    /// A write whose byte footprint the route resolved. Through the owner it is
    /// range-wise: refused exactly when the interval shares a point with a live
    /// loan's. Through a range lend it is the **rebasing law**: admitted
    /// exactly when it stays inside that lend's own range.
    fn write_range(
        &self,
        place: &str,
        axis: &str,
        low: u64,
        high: u64,
    ) -> Result<(), OwnershipError> {
        match self.binding(&Place::name(place))? {
            Binding::Owned(resource) => {
                if self.has_loan_overlapping(*resource, axis, low, high, false) {
                    Err(OwnershipError::OwnerMutationWhileBorrowed(place.into()))
                } else {
                    Ok(())
                }
            }
            Binding::Borrowed {
                kind: BorrowKind::Mutable,
                ..
            } => Ok(()),
            Binding::Borrowed {
                kind: BorrowKind::Shared,
                ..
            } => Err(OwnershipError::WriteThroughSharedBorrow(place.into())),
            Binding::Borrowed {
                kind:
                    BorrowKind::MutableRange {
                        axis: held_axis,
                        low: held_low,
                        high: held_high,
                    },
                ..
            } => {
                if held_axis == axis && *held_low <= low && high <= *held_high {
                    Ok(())
                } else {
                    Err(OwnershipError::AccessOutsideLendRange(place.into()))
                }
            }
            Binding::Moved => Err(OwnershipError::UseAfterMove(place.into())),
            Binding::Dropped => Err(OwnershipError::UseAfterDrop(place.into())),
            Binding::Aggregate { .. } => {
                Err(OwnershipError::AggregateAtOwnerPosition(place.into()))
            }
        }
    }

    /// The read counterpart. An owner read conflicts only with loans that hold
    /// **write** authority — a whole mutable borrow or a range lend — and only
    /// on a shared point; a shared borrow blocks no read. An empty interval
    /// (an extent query) overlaps nothing and is admitted beside any loan,
    /// while the liveness of the binding is still checked.
    fn read_range(
        &self,
        place: &str,
        axis: &str,
        low: u64,
        high: u64,
    ) -> Result<(), OwnershipError> {
        match self.binding(&Place::name(place))? {
            Binding::Owned(resource) => {
                if self.has_loan_overlapping(*resource, axis, low, high, true) {
                    Err(OwnershipError::OwnerAccessWhileMutablyBorrowed(
                        place.into(),
                    ))
                } else {
                    Ok(())
                }
            }
            Binding::Borrowed {
                kind: BorrowKind::Shared | BorrowKind::Mutable,
                ..
            } => Ok(()),
            Binding::Borrowed {
                kind:
                    BorrowKind::MutableRange {
                        axis: held_axis,
                        low: held_low,
                        high: held_high,
                    },
                ..
            } => {
                if held_axis == axis && *held_low <= low && high <= *held_high {
                    Ok(())
                } else {
                    Err(OwnershipError::AccessOutsideLendRange(place.into()))
                }
            }
            Binding::Moved => Err(OwnershipError::UseAfterMove(place.into())),
            Binding::Dropped => Err(OwnershipError::UseAfterDrop(place.into())),
            Binding::Aggregate { .. } => {
                Err(OwnershipError::AggregateAtOwnerPosition(place.into()))
            }
        }
    }

    /// Whether any live loan of `resource` shares a point with `[low, high)` on
    /// `axis`. `mutable_only` restricts the question to loans carrying write
    /// authority, which is the read direction's conflict set. A whole-resource
    /// borrow covers every point, so it overlaps exactly when the interval is
    /// non-empty.
    fn has_loan_overlapping(
        &self,
        resource: ResourceId,
        axis: &str,
        low: u64,
        high: u64,
        mutable_only: bool,
    ) -> bool {
        self.bindings.values().any(|binding| {
            let Binding::Borrowed {
                resource: loaned,
                kind,
            } = binding
            else {
                return false;
            };
            if *loaned != resource {
                return false;
            }
            match kind {
                BorrowKind::Shared => !mutable_only && low < high,
                BorrowKind::Mutable => low < high,
                BorrowKind::MutableRange {
                    axis: held_axis,
                    low: held_low,
                    high: held_high,
                } => held_axis == axis && axes_overlap(low, high, *held_low, *held_high),
            }
        })
    }

    fn branch(
        &mut self,
        then_ops: &[OwnershipOp],
        else_ops: &[OwnershipOp],
    ) -> Result<(), OwnershipError> {
        let mut then_state = self.clone();
        let mut else_state = self.clone();
        then_state.execute(then_ops)?;
        else_state.execute(else_ops)?;
        if then_state.bindings != else_state.bindings
            || then_state.ledger != else_state.ledger
            || then_state.live != else_state.live
        {
            return Err(OwnershipError::BranchOwnershipMismatch {
                then_owned: then_state.owned_places(),
                else_owned: else_state.owned_places(),
            });
        }
        then_state.next_acquisition_order = then_state
            .next_acquisition_order
            .max(else_state.next_acquisition_order);
        *self = then_state;
        Ok(())
    }

    /// The binding at `place`. A field path whose base is bound but is not an
    /// aggregate is refused by name (`NotAnAggregate`), so a reader is told
    /// "`p` has no fields" rather than "`p.buf` is unknown"; every other
    /// absence is the unknown binding it always was.
    fn binding(&self, place: &Place) -> Result<&Binding, OwnershipError> {
        if let Some(binding) = self.bindings.get(place) {
            return Ok(binding);
        }
        if let Place::Field(path) = place
            && let Some(base) = self.bindings.get(&Place::name(&path.base))
            && !matches!(base, Binding::Aggregate { .. })
        {
            return Err(OwnershipError::NotAnAggregate(path.base.clone()));
        }
        Err(OwnershipError::UnknownBinding(place.spelling()))
    }

    fn require_owned(&self, place: &Place) -> Result<ResourceId, OwnershipError> {
        match self.binding(place)? {
            Binding::Owned(resource) => Ok(*resource),
            Binding::Borrowed { .. } => Err(OwnershipError::MoveOfBorrow(place.spelling())),
            Binding::Moved => Err(OwnershipError::UseAfterMove(place.spelling())),
            Binding::Dropped => Err(OwnershipError::UseAfterDrop(place.spelling())),
            Binding::Aggregate { .. } => {
                Err(OwnershipError::AggregateAtOwnerPosition(place.spelling()))
            }
        }
    }

    fn require_owned_for_drop(&self, place: &Place) -> Result<ResourceId, OwnershipError> {
        match self.binding(place)? {
            Binding::Owned(resource) => Ok(*resource),
            Binding::Borrowed { .. } => Err(OwnershipError::DropOfBorrow(place.spelling())),
            Binding::Moved => Err(OwnershipError::UseAfterMove(place.spelling())),
            Binding::Dropped => Err(OwnershipError::DoubleDrop(place.spelling())),
            // Unreachable from `release`, which walks an aggregate before it
            // asks; kept total in the refusing direction.
            Binding::Aggregate { .. } => {
                Err(OwnershipError::AggregateAtOwnerPosition(place.spelling()))
            }
        }
    }

    fn has_any_borrow(&self, resource: ResourceId) -> bool {
        self.bindings.values().any(
            |binding| matches!(binding, Binding::Borrowed { resource: loaned, .. } if *loaned == resource),
        )
    }

    fn has_mutable_borrow(&self, resource: ResourceId) -> bool {
        self.bindings.values().any(|binding| {
            matches!(
                binding,
                Binding::Borrowed {
                    resource: loaned,
                    // A range lend carries write authority, so for every rule
                    // that asks "is there a mutable borrow" — the owner's
                    // rangeless read, a candidate shared borrow — it is one.
                    kind: BorrowKind::Mutable | BorrowKind::MutableRange { .. },
                } if *loaned == resource
            )
        })
    }

    /// Every live owner's spelling — plain names first, then `BASE.FIELD`
    /// paths — which is what **the leak rule that sees into the aggregate**
    /// (card `AGGREGATE-OWNED-FIELD`) reports: an unreleased field is an owner
    /// like any other, and `NormalPathLeak` names it by its path.
    fn owned_places(&self) -> Vec<String> {
        self.bindings
            .iter()
            .filter(|(_, binding)| matches!(binding, Binding::Owned(_)))
            .map(|(place, _)| place.spelling())
            .collect()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OwnershipArithmeticError {
    Overflow,
    Underflow(String),
}

impl fmt::Display for OwnershipArithmeticError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Overflow => f.write_str("resource quantity arithmetic overflow"),
            Self::Underflow(axis) => write!(f, "resource quantity underflow on axis `{axis}`"),
        }
    }
}

impl std::error::Error for OwnershipArithmeticError {}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OwnershipError {
    UnknownBinding(String),
    BindingAlreadyExists(String),
    DuplicateResourceId(ResourceId),
    CopyOfOwned(String),
    UseAfterMove(String),
    UseAfterDrop(String),
    DoubleDrop(String),
    MoveOfBorrow(String),
    DropOfBorrow(String),
    MoveWhileBorrowed(String),
    DropWhileBorrowed(String),
    OwnerAccessWhileMutablyBorrowed(String),
    OwnerMutationWhileBorrowed(String),
    WriteThroughSharedBorrow(String),
    SharedBorrowWhileMutablyBorrowed(String),
    MutableBorrowWhileBorrowed(String),
    /// Two write lends of one resource would share a byte (card
    /// `SLICE-PERMISSIONS-WRITE-LEND`). The exclusivity rule: a second lend is
    /// admitted exactly when its range is disjoint from every live loan's.
    OverlappingLends(String),
    /// A lend names a loan as its owner; re-lending is not modelled.
    LendOfBorrow(String),
    /// An access through a range lend lies outside — or does not state — the
    /// lent range. The rebasing law, enforced rather than trusted.
    AccessOutsideLendRange(String),
    /// An aggregate name at a position that takes a plain owner — a read, a
    /// write, a whole move, a transfer, a borrow or a lend of the record
    /// itself (card `AGGREGATE-OWNED-FIELD`). A record with an owned field is
    /// addressed field by field; its only whole-record operations are `Drop`
    /// (which walks its fields) and `Copy` (refused).
    AggregateAtOwnerPosition(String),
    /// A field path whose base is a binding that has no fields.
    NotAnAggregate(String),
    /// A whole write of an owned field that still holds its buffer (card
    /// `W48-QR-USABLE-RECORDS`): the buffer it holds would lose its release
    /// duty. A field is refilled only when it is a hole.
    OverwriteOfLiveField(String),
    BranchOwnershipMismatch {
        then_owned: Vec<String>,
        else_owned: Vec<String>,
    },
    NormalPathLeak(Vec<String>),
    LedgerInvariantViolation,
    Arithmetic(OwnershipArithmeticError),
}

impl fmt::Display for OwnershipError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownBinding(name) => write!(f, "unknown ownership binding `{name}`"),
            Self::BindingAlreadyExists(name) => {
                write!(f, "ownership binding `{name}` already exists")
            }
            Self::DuplicateResourceId(id) => write!(f, "resource identity `{id}` is already live"),
            Self::CopyOfOwned(name) => write!(f, "owned resource `{name}` cannot be copied"),
            Self::UseAfterMove(name) => write!(f, "use of `{name}` after move"),
            Self::UseAfterDrop(name) => write!(f, "use of `{name}` after drop"),
            Self::DoubleDrop(name) => write!(f, "resource `{name}` is dropped more than once"),
            Self::MoveOfBorrow(name) => write!(f, "borrow `{name}` cannot be moved as an owner"),
            Self::DropOfBorrow(name) => write!(f, "borrow `{name}` cannot be dropped as an owner"),
            Self::MoveWhileBorrowed(name) => write!(f, "cannot move `{name}` while it is borrowed"),
            Self::DropWhileBorrowed(name) => write!(f, "cannot drop `{name}` while it is borrowed"),
            Self::OwnerAccessWhileMutablyBorrowed(name) => {
                write!(
                    f,
                    "cannot access owner `{name}` through an active mutable borrow"
                )
            }
            Self::OwnerMutationWhileBorrowed(name) => {
                write!(f, "cannot mutate owner `{name}` while it is borrowed")
            }
            Self::WriteThroughSharedBorrow(name) => {
                write!(f, "cannot write through shared borrow `{name}`")
            }
            Self::SharedBorrowWhileMutablyBorrowed(name) => {
                write!(f, "cannot shared-borrow `{name}` during a mutable borrow")
            }
            Self::MutableBorrowWhileBorrowed(name) => {
                write!(
                    f,
                    "cannot mutably borrow `{name}` while another borrow is active"
                )
            }
            Self::OverlappingLends(name) => {
                write!(
                    f,
                    "two write lends of `{name}` would share a byte; a write lend holds \
                     exclusive authority over its range, so a second lend is admitted only \
                     where its range is disjoint from every live loan's"
                )
            }
            Self::LendOfBorrow(name) => {
                write!(f, "loan `{name}` cannot itself be lent")
            }
            Self::AccessOutsideLendRange(name) => {
                write!(
                    f,
                    "an access through lend `{name}` lies outside — or does not state — the \
                     range it was lent"
                )
            }
            Self::AggregateAtOwnerPosition(name) => {
                write!(
                    f,
                    "`{name}` is a record with an owned field and is addressed field by field; \
                     the record itself is dropped as a whole or not at all, and is never read, \
                     written, moved, lent or transferred as one owner"
                )
            }
            Self::NotAnAggregate(name) => {
                write!(f, "`{name}` has no fields to address")
            }
            Self::OverwriteOfLiveField(name) => {
                write!(
                    f,
                    "`{name}` still holds its buffer, and a write of the whole field would lose \
                     that buffer's release; an owned field is written only when it is a hole — \
                     moved out or released first"
                )
            }
            Self::BranchOwnershipMismatch { .. } => {
                f.write_str("branch ownership states do not join")
            }
            Self::NormalPathLeak(owners) => {
                write!(
                    f,
                    "live resources escape normal exit: {}",
                    owners.join(", ")
                )
            }
            Self::LedgerInvariantViolation => {
                f.write_str("resource ledger and live vector are inconsistent")
            }
            Self::Arithmetic(error) => error.fmt(f),
        }
    }
}

impl std::error::Error for OwnershipError {}

impl From<OwnershipArithmeticError> for OwnershipError {
    fn from(error: OwnershipArithmeticError) -> Self {
        Self::Arithmetic(error)
    }
}

/// Executes the ownership IR while retaining the final state for inspection.
pub fn execute_ownership(operations: &[OwnershipOp]) -> Result<OwnershipState, OwnershipError> {
    let mut state = OwnershipState::default();
    state.execute(operations)?;
    Ok(state)
}

/// The maximum value `axis` ever reaches in the live footprint while these
/// operations run — **the same number a scan of every prefix gives**, computed
/// in one pass (card `LOOP-UNROLL-BUDGET`).
///
/// The maximum over every prefix of a sequence is the maximum of the values the
/// sequence passes through, so this is an equality rather than an
/// approximation; `the_one_pass_peak_is_the_number_a_full_prefix_scan_gives`
/// pins it. A prefix the model refuses ends the walk, exactly as a refused
/// prefix ended the scan — the maximum reported is the maximum over the
/// prefixes that executed, and nothing about a refused one is invented.
///
/// **Why it exists.** Card `LOOP-V1` narrowed a scan of every prefix to the
/// prefixes that can *be* the maximum, which is linear in the number of
/// acquisitions — but each of those prefixes was still re-executed from the
/// start, so a body acquiring once per iteration cost a quadratic number of
/// ownership steps. Measured at a bound of 1024 that was **56.4 s** of check
/// time against **0.39 s** for this one pass (0.20 s with the computation
/// removed entirely, which lower-bounds it), and it is the measurement that
/// makes `source.loop.v1`'s absolute ceiling defensible at 4096 rather than
/// 1024.
#[must_use]
pub fn peak_live_axis(operations: &[OwnershipOp], axis: &str) -> u64 {
    let mut state = OwnershipState::default();
    let mut peak = state.live.get(axis);
    for operation in operations {
        if state.execute_one(operation).is_err() || state.validate_invariant().is_err() {
            break;
        }
        peak = peak.max(state.live.get(axis));
    }
    peak
}

/// Checks the operations and rejects every live local at a normal function exit.
pub fn check_ownership(operations: &[OwnershipOp]) -> Result<OwnershipState, OwnershipError> {
    let state = execute_ownership(operations)?;
    if state.ledger.is_empty() {
        debug_assert_eq!(state.live, Footprint::zero());
        Ok(state)
    } else {
        Err(OwnershipError::NormalPathLeak(state.owned_places()))
    }
}

#[cfg(test)]
#[path = "../tests/unit/ownership.rs"]
mod tests;
