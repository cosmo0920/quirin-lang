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

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResourceEntry {
    id: ResourceId,
    resource_type: String,
    footprint: Footprint,
    owner: String,
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

    pub fn owner(&self) -> &str {
        &self.owner
    }

    pub fn acquisition_order(&self) -> u64 {
        self.acquisition_order
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum BorrowKind {
    Shared,
    Mutable,
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
    Branch {
        then_ops: Vec<OwnershipOp>,
        else_ops: Vec<OwnershipOp>,
    },
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct OwnershipState {
    bindings: BTreeMap<String, Binding>,
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
        match self.bindings.get(owner) {
            Some(Binding::Owned(resource)) => Some(*resource),
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
        for (name, binding) in &self.bindings {
            match binding {
                Binding::Owned(resource) => {
                    let Some(entry) = self.ledger.get(resource) else {
                        return Err(OwnershipError::LedgerInvariantViolation);
                    };
                    if entry.owner != *name {
                        return Err(OwnershipError::LedgerInvariantViolation);
                    }
                }
                Binding::Borrowed { resource, .. } => {
                    if !self.ledger.contains_key(resource) {
                        return Err(OwnershipError::LedgerInvariantViolation);
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
            OwnershipOp::Read(place) => self.read(place),
            OwnershipOp::Write(place) => self.write(place),
            OwnershipOp::Move { from, to } => self.move_owned(from, to),
            OwnershipOp::Copy(place) => self.copy_owned(place),
            OwnershipOp::Drop(place) => self.release(place, false),
            OwnershipOp::TransferOut(place) => self.release(place, true),
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
            OwnershipOp::Branch { then_ops, else_ops } => self.branch(then_ops, else_ops),
        }
    }

    fn acquire(
        &mut self,
        id: ResourceId,
        owner: &str,
        resource_type: &str,
        footprint: &Footprint,
    ) -> Result<(), OwnershipError> {
        if self.bindings.contains_key(owner) {
            return Err(OwnershipError::BindingAlreadyExists(owner.into()));
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
                owner: owner.into(),
                acquisition_order,
            },
        );
        self.bindings.insert(owner.into(), Binding::Owned(id));
        self.live = next_live;
        Ok(())
    }

    fn read(&self, place: &str) -> Result<(), OwnershipError> {
        match self.binding(place)? {
            Binding::Owned(resource) => {
                if self.has_mutable_borrow(*resource) {
                    Err(OwnershipError::OwnerAccessWhileMutablyBorrowed(
                        place.into(),
                    ))
                } else {
                    Ok(())
                }
            }
            Binding::Borrowed { .. } => Ok(()),
            Binding::Moved => Err(OwnershipError::UseAfterMove(place.into())),
            Binding::Dropped => Err(OwnershipError::UseAfterDrop(place.into())),
        }
    }

    fn write(&self, place: &str) -> Result<(), OwnershipError> {
        match self.binding(place)? {
            Binding::Owned(resource) => {
                if self.has_any_borrow(*resource) {
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
            Binding::Moved => Err(OwnershipError::UseAfterMove(place.into())),
            Binding::Dropped => Err(OwnershipError::UseAfterDrop(place.into())),
        }
    }

    fn move_owned(&mut self, from: &str, to: &str) -> Result<(), OwnershipError> {
        if self.bindings.contains_key(to) {
            return Err(OwnershipError::BindingAlreadyExists(to.into()));
        }
        let resource = self.require_owned(from)?;
        if self.has_any_borrow(resource) {
            return Err(OwnershipError::MoveWhileBorrowed(from.into()));
        }
        let entry = self
            .ledger
            .get_mut(&resource)
            .ok_or(OwnershipError::LedgerInvariantViolation)?;
        entry.owner = to.into();
        self.bindings.insert(from.into(), Binding::Moved);
        self.bindings.insert(to.into(), Binding::Owned(resource));
        Ok(())
    }

    fn copy_owned(&self, place: &str) -> Result<(), OwnershipError> {
        self.require_owned(place)?;
        Err(OwnershipError::CopyOfOwned(place.into()))
    }

    fn release(&mut self, place: &str, transfer: bool) -> Result<(), OwnershipError> {
        let resource = self.require_owned_for_drop(place)?;
        if self.has_any_borrow(resource) {
            return Err(OwnershipError::DropWhileBorrowed(place.into()));
        }
        let entry = self
            .ledger
            .remove(&resource)
            .ok_or(OwnershipError::LedgerInvariantViolation)?;
        self.live = self.live.checked_sub(&entry.footprint)?;
        self.bindings.insert(
            place.into(),
            if transfer {
                Binding::Moved
            } else {
                Binding::Dropped
            },
        );
        Ok(())
    }

    fn borrow(
        &mut self,
        owner: &str,
        borrower: &str,
        kind: BorrowKind,
        body: &[OwnershipOp],
    ) -> Result<(), OwnershipError> {
        if self.bindings.contains_key(borrower) {
            return Err(OwnershipError::BindingAlreadyExists(borrower.into()));
        }
        let resource = self.require_owned(owner)?;
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
        self.bindings.insert(borrower.into(), loan.clone());
        self.execute(body)?;
        if self.bindings.remove(borrower) != Some(loan) {
            return Err(OwnershipError::LedgerInvariantViolation);
        }
        Ok(())
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

    fn binding(&self, place: &str) -> Result<&Binding, OwnershipError> {
        self.bindings
            .get(place)
            .ok_or_else(|| OwnershipError::UnknownBinding(place.into()))
    }

    fn require_owned(&self, place: &str) -> Result<ResourceId, OwnershipError> {
        match self.binding(place)? {
            Binding::Owned(resource) => Ok(*resource),
            Binding::Borrowed { .. } => Err(OwnershipError::MoveOfBorrow(place.into())),
            Binding::Moved => Err(OwnershipError::UseAfterMove(place.into())),
            Binding::Dropped => Err(OwnershipError::UseAfterDrop(place.into())),
        }
    }

    fn require_owned_for_drop(&self, place: &str) -> Result<ResourceId, OwnershipError> {
        match self.binding(place)? {
            Binding::Owned(resource) => Ok(*resource),
            Binding::Borrowed { .. } => Err(OwnershipError::DropOfBorrow(place.into())),
            Binding::Moved => Err(OwnershipError::UseAfterMove(place.into())),
            Binding::Dropped => Err(OwnershipError::DoubleDrop(place.into())),
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
                    kind: BorrowKind::Mutable,
                } if *loaned == resource
            )
        })
    }

    fn owned_places(&self) -> Vec<String> {
        self.bindings
            .iter()
            .filter(|(_, binding)| matches!(binding, Binding::Owned(_)))
            .map(|(name, _)| name.clone())
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
mod tests {
    use super::*;

    fn acquire(id: ResourceId, owner: &str, amount: u64) -> OwnershipOp {
        OwnershipOp::Acquire {
            id,
            owner: owner.into(),
            resource_type: "Buffer".into(),
            footprint: Footprint::single("memory", amount),
        }
    }

    #[test]
    fn allocate_use_and_drop_reaches_empty_normal_exit() {
        let state = check_ownership(&[
            acquire(1, "buffer", 64),
            OwnershipOp::Read("buffer".into()),
            OwnershipOp::Drop("buffer".into()),
        ])
        .unwrap();
        assert_eq!(state.live(), &Footprint::zero());
        assert_eq!(state.ledger().count(), 0);
    }

    #[test]
    fn move_preserves_identity_and_footprint() {
        let state = execute_ownership(&[
            acquire(7, "source", 32),
            OwnershipOp::Move {
                from: "source".into(),
                to: "destination".into(),
            },
        ])
        .unwrap();
        assert_eq!(state.owner_resource("source"), None);
        assert_eq!(state.owner_resource("destination"), Some(7));
        assert_eq!(state.live().get("memory"), 32);
        assert_eq!(state.ledger().next().unwrap().owner(), "destination");
        state.validate_invariant().unwrap();
    }

    #[test]
    fn transfer_out_is_not_a_normal_path_leak() {
        check_ownership(&[
            acquire(1, "result", 8),
            OwnershipOp::TransferOut("result".into()),
        ])
        .unwrap();
    }

    #[test]
    fn rejects_use_after_move() {
        let error = check_ownership(&[
            acquire(1, "a", 1),
            OwnershipOp::Move {
                from: "a".into(),
                to: "b".into(),
            },
            OwnershipOp::Read("a".into()),
        ])
        .unwrap_err();
        assert_eq!(error, OwnershipError::UseAfterMove("a".into()));
    }

    #[test]
    fn rejects_double_drop() {
        let error = check_ownership(&[
            acquire(1, "a", 1),
            OwnershipOp::Drop("a".into()),
            OwnershipOp::Drop("a".into()),
        ])
        .unwrap_err();
        assert_eq!(error, OwnershipError::DoubleDrop("a".into()));
    }

    #[test]
    fn rejects_owned_copy() {
        let error =
            check_ownership(&[acquire(1, "a", 1), OwnershipOp::Copy("a".into())]).unwrap_err();
        assert_eq!(error, OwnershipError::CopyOfOwned("a".into()));
    }

    #[test]
    fn rejects_normal_path_leak() {
        let error = check_ownership(&[acquire(1, "a", 1)]).unwrap_err();
        assert_eq!(error, OwnershipError::NormalPathLeak(vec!["a".into()]));
    }

    #[test]
    fn shared_borrow_ends_before_drop() {
        check_ownership(&[
            acquire(1, "a", 1),
            OwnershipOp::BorrowShared {
                owner: "a".into(),
                borrower: "view".into(),
                body: vec![OwnershipOp::Read("view".into())],
            },
            OwnershipOp::Drop("a".into()),
        ])
        .unwrap();
    }

    #[test]
    fn multiple_shared_borrows_may_overlap() {
        check_ownership(&[
            acquire(1, "a", 1),
            OwnershipOp::BorrowShared {
                owner: "a".into(),
                borrower: "left".into(),
                body: vec![OwnershipOp::BorrowShared {
                    owner: "a".into(),
                    borrower: "right".into(),
                    body: vec![
                        OwnershipOp::Read("left".into()),
                        OwnershipOp::Read("right".into()),
                    ],
                }],
            },
            OwnershipOp::Drop("a".into()),
        ])
        .unwrap();
    }

    #[test]
    fn mutable_borrow_is_exclusive_and_permits_write() {
        check_ownership(&[
            acquire(1, "a", 1),
            OwnershipOp::BorrowMutable {
                owner: "a".into(),
                borrower: "exclusive".into(),
                body: vec![OwnershipOp::Write("exclusive".into())],
            },
            OwnershipOp::Drop("a".into()),
        ])
        .unwrap();

        let error = check_ownership(&[
            acquire(1, "a", 1),
            OwnershipOp::BorrowShared {
                owner: "a".into(),
                borrower: "shared".into(),
                body: vec![OwnershipOp::BorrowMutable {
                    owner: "a".into(),
                    borrower: "exclusive".into(),
                    body: vec![],
                }],
            },
        ])
        .unwrap_err();
        assert_eq!(
            error,
            OwnershipError::MutableBorrowWhileBorrowed("a".into())
        );
    }

    #[test]
    fn rejects_move_during_shared_borrow() {
        let error = check_ownership(&[
            acquire(1, "a", 1),
            OwnershipOp::BorrowShared {
                owner: "a".into(),
                borrower: "view".into(),
                body: vec![OwnershipOp::Move {
                    from: "a".into(),
                    to: "b".into(),
                }],
            },
        ])
        .unwrap_err();
        assert_eq!(error, OwnershipError::MoveWhileBorrowed("a".into()));
    }

    #[test]
    fn branch_states_must_agree() {
        let error = check_ownership(&[
            acquire(1, "a", 1),
            OwnershipOp::Branch {
                then_ops: vec![OwnershipOp::Drop("a".into())],
                else_ops: vec![],
            },
        ])
        .unwrap_err();
        assert!(matches!(
            error,
            OwnershipError::BranchOwnershipMismatch { .. }
        ));
    }

    #[test]
    fn live_vector_is_componentwise_ledger_sum() {
        let footprint =
            Footprint::from_axes([("memory".into(), 16), ("descriptors".into(), 1)]).unwrap();
        let state = execute_ownership(&[
            OwnershipOp::Acquire {
                id: 1,
                owner: "file".into(),
                resource_type: "MappedFile".into(),
                footprint,
            },
            acquire(2, "scratch", 4),
        ])
        .unwrap();
        assert_eq!(state.live().get("memory"), 20);
        assert_eq!(state.live().get("descriptors"), 1);
        state.validate_invariant().unwrap();
    }
}
