//! Bounded extern-call semantics: the `source.extern-call.v1` model.
//!
//! Card `source.extern-call.v1` (`docs/design/EXTERN_CALL_SEMANTICS_V1.md`).
//! This module owns three things and deliberately nothing else:
//!
//! 1. **Admission** ([`bridge_contract`]). A complete resource-contract v1
//!    declaration goes in; either one [`ExternCallee`] inside the closed v1
//!    bridging table comes out, or a named [`BridgeRefusal`] does. Every C04
//!    mode outside §3.1 of the design is an explicit refusal constructor, not
//!    an unhandled case: `resource_owned`, `resource_released`, `resource_out`,
//!    `resource_stored`, `resource_removed`, `resource_return_reallocated`, a
//!    nonnull owned return and a nullable consumed parameter each have their
//!    own refusal, so widening the profile means deleting a refusal on purpose.
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
//! What a result through this module means. A contract admitted here is a
//! **declared assumption** (`ContractTrust::Declared`, contract spec §C06): it
//! is never a proof about the C implementation, its layout, its aliasing or
//! its thread-safety. Two premises are recorded on every certificate:
//! [`ExternAssumption::DeclaredContract`] for each callee, and
//! [`ExternAssumption::NoExtentTransport`], because C04 pointers are extentless
//! `u8` projections — the declaration cannot say how many bytes the callee
//! touches, and this profile does not check it.
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use crate::ResourceId;
use crate::typed_errors::{Effect, EffectRow};

/// The C ABI projection of contract spec §C04.1, complete. `Unit` is a result
/// type only, exactly as in the loader.
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
}

/// The admitted v1 bridging of the result (design §3.2).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CallResult {
    /// `resource_return_none()`: `unit` or a scalar, data only.
    Data(AbiType),
    /// `resource_return_owned(nullable)`: fresh custody behind the mandatory
    /// null adapter of design §3.3.
    OwnedNullable,
}

/// One admitted callee. Only [`bridge_contract`] constructs this, so a value of
/// this type always names a shape inside the closed v1 table.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExternCallee {
    pub name: String,
    pub parameters: Vec<CallParameter>,
    pub result: CallResult,
    pub result_abi: AbiType,
}

/// Why a complete, loadable C04 declaration is still not callable in v1. Each
/// constructor names one excluded shape; none is a catch-all.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BridgeRefusal {
    /// `resource_owned`: the call retains custody it never took. Follow-on.
    OwnedParameter { parameter: usize },
    /// `resource_released`: release through an extern call is the cleanup
    /// relation territory contract spec §C10 reserves. Follow-on.
    ReleasedParameter { parameter: usize },
    /// `resource_out`: fresh custody through a pointer-to-pointer destination.
    OutParameter { parameter: usize },
    /// `resource_stored`: owner-indexed custody. Follow-on.
    StoredParameter { parameter: usize },
    /// `resource_removed`: owner-indexed custody. Follow-on.
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
    /// `resource_return_reallocated`: identity replacement. Follow-on.
    ReallocatedReturn,
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
                "parameter {parameter} declares `resource_owned`, which `source.extern-call.v1` does not call"
            ),
            Self::ReleasedParameter { parameter } => write!(
                f,
                "parameter {parameter} declares `resource_released`; release through an extern call is reserved for the cleanup relation"
            ),
            Self::OutParameter { parameter } => write!(
                f,
                "parameter {parameter} declares `resource_out`, which `source.extern-call.v1` does not call"
            ),
            Self::StoredParameter { parameter } => write!(
                f,
                "parameter {parameter} declares `resource_stored`, which `source.extern-call.v1` does not call"
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
            Self::ReallocatedReturn => {
                f.write_str("`resource_return_reallocated` is not callable in v1")
            }
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
    let mut parameters = Vec::with_capacity(contract.parameters.len());
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
            ContractParameterMode::Owned { .. } => {
                return Err(if abi.is_pointer() {
                    BridgeRefusal::OwnedParameter { parameter: index }
                } else {
                    BridgeRefusal::ScalarCustody { parameter: index }
                });
            }
            ContractParameterMode::Released { .. } => {
                return Err(if abi.is_pointer() {
                    BridgeRefusal::ReleasedParameter { parameter: index }
                } else {
                    BridgeRefusal::ScalarCustody { parameter: index }
                });
            }
            ContractParameterMode::Out { .. } => {
                return Err(if abi.is_pointer() {
                    BridgeRefusal::OutParameter { parameter: index }
                } else {
                    BridgeRefusal::ScalarCustody { parameter: index }
                });
            }
            ContractParameterMode::Stored { .. } => {
                return Err(if abi.is_pointer() {
                    BridgeRefusal::StoredParameter { parameter: index }
                } else {
                    BridgeRefusal::ScalarCustody { parameter: index }
                });
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
        ContractReturnMode::Reallocated { .. } => return Err(BridgeRefusal::ReallocatedReturn),
    };
    Ok(ExternCallee {
        name: name.to_owned(),
        parameters,
        result,
        result_abi: contract.result,
    })
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
}

/// Derived custody delta. A function of the admitted callee alone.
#[must_use]
pub fn derived_custody(callee: &ExternCallee) -> Vec<ExternCustody> {
    callee
        .parameters
        .iter()
        .filter_map(|parameter| match parameter {
            CallParameter::Data(_) => None,
            CallParameter::Borrowed(_) => Some(ExternCustody::Borrowed),
            CallParameter::Consumed => Some(ExternCustody::Consumed),
        })
        .collect()
}

/// Derived effect row (design §3.4): `Io` always, plus `Alloc` exactly when the
/// contract returns owned custody, plus `Mut` exactly when some parameter is a
/// `mut` pointer. The caller's declared row must admit this; that containment
/// check belongs to the established typed-error pass, not to this module.
#[must_use]
pub fn derived_effects(callee: &ExternCallee) -> EffectRow {
    let mut effects = BTreeSet::from([Effect::Io]);
    if callee.result == CallResult::OwnedNullable {
        effects.insert(Effect::Alloc);
    }
    if callee.parameters.iter().any(|parameter| match parameter {
        CallParameter::Data(abi) | CallParameter::Borrowed(abi) => abi.is_mutable_pointer(),
        CallParameter::Consumed => true,
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
}

/// The mandatory nullable-owned-return adapter (design §3.3): the null arm is
/// branched on at the call site and takes the declared typed error exit.
/// `succeeds` is the fixture's literal outcome control; both arms are
/// exercisable, and the emitted C checks the real callee against it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExternAdapter {
    pub resource: ResourceId,
    pub succeeds: bool,
    pub error: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExternCall {
    pub callee: String,
    pub arguments: Vec<ExternArgument>,
    /// Present exactly for a `resource_return_owned(nullable)` callee.
    pub adapter: Option<ExternAdapter>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ExternEvent {
    Call(ExternCall),
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
}

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
    /// Q025: the adapter's error arm would abandon custody this profile cannot
    /// release. The bounded v1 adapter admits a raise only where no acquired
    /// identity is live; releasing on the error arm is a named follow-on.
    AdapterAbandonsCustody { event: usize, resource: ResourceId },
    /// Q024: an acquisition reuses an identity the trace already introduced.
    DuplicateAcquisition { event: usize, resource: ResourceId },
}

impl ExternDiagnostic {
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            Self::UnknownCallee { .. } => "Q023",
            Self::ArgumentArity { .. }
            | Self::ArgumentForm { .. }
            | Self::DuplicateAcquisition { .. } => "Q024",
            Self::AdapterMissing { .. }
            | Self::AdapterUnexpected { .. }
            | Self::AdapterUnknownError { .. }
            | Self::AdapterAbandonsCustody { .. } => "Q025",
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
            | Self::DuplicateAcquisition { event, .. } => *event,
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
                "a `resource_return_owned(nullable)` call requires the checked null adapter `succeeds BOOL else raise ERROR`",
            ),
            Self::AdapterUnexpected { .. } => {
                f.write_str("this callee returns no custody, so it takes no null adapter")
            }
            Self::AdapterUnknownError { error, .. } => {
                write!(f, "the adapter raises undeclared error type `{error}`")
            }
            Self::AdapterAbandonsCustody { resource, .. } => write!(
                f,
                "the adapter's error arm would abandon live resource {resource}; releasing custody on an extern error arm is not in v1"
            ),
            Self::DuplicateAcquisition { resource, .. } => {
                write!(f, "resource identity {resource} is acquired twice")
            }
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
    /// C04 pointers are extentless `u8` projections, so no declaration in this
    /// profile promises how many bytes a callee touches.
    NoExtentTransport,
    /// Which object file provides each symbol is the builder's premise.
    LinkageSuppliedByBuilder,
}

/// Evidence a checked module publishes. Every field is re-derivable from the
/// program alone; [`recheck_extern_certificate`] does exactly that.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExternCallCertificate {
    pub entry: String,
    /// Admitted call sites, in trace order, as (callee, derived custody).
    pub calls: Vec<(String, Vec<ExternCustody>)>,
    /// Identities the adapters introduce, in trace order.
    pub acquisitions: Vec<ResourceId>,
    /// Identities consumed by a call, in trace order.
    pub consumptions: Vec<ResourceId>,
    /// Union of the derived rows of every call site.
    pub effects: EffectRow,
    pub assumptions: Vec<ExternAssumption>,
}

/// Check one lowered module's call sites. Returns a certificate or every
/// refusal found, so a caller reports the whole cascade rather than the first.
pub fn check_extern_program(
    program: &ExternProgram,
) -> Result<ExternCallCertificate, Vec<ExternDiagnostic>> {
    let mut diagnostics = Vec::new();
    let mut calls = Vec::new();
    let mut acquisitions = Vec::new();
    let mut consumptions = Vec::new();
    let mut live = BTreeSet::<ResourceId>::new();
    let mut introduced = BTreeSet::<ResourceId>::new();
    let mut effects = BTreeSet::new();
    for (index, event) in program.events.iter().enumerate() {
        let ExternEvent::Call(call) = event else {
            continue;
        };
        let Some(callee) = program.callees.get(&call.callee) else {
            diagnostics.push(ExternDiagnostic::UnknownCallee {
                event: index,
                name: call.callee.clone(),
            });
            continue;
        };
        effects.extend(derived_effects(callee).iter().cloned());
        if call.arguments.len() != callee.parameters.len() {
            diagnostics.push(ExternDiagnostic::ArgumentArity {
                event: index,
                expected: callee.parameters.len(),
                actual: call.arguments.len(),
            });
            continue;
        }
        let mut consumed_here = Vec::new();
        for (parameter, (argument, declared)) in
            call.arguments.iter().zip(&callee.parameters).enumerate()
        {
            let admitted = matches!(
                (argument, declared),
                (ExternArgument::Data(_), CallParameter::Data(_))
                    | (ExternArgument::Borrow(_), CallParameter::Borrowed(_))
                    | (ExternArgument::Consume(_), CallParameter::Consumed)
            );
            if !admitted {
                diagnostics.push(ExternDiagnostic::ArgumentForm {
                    event: index,
                    parameter,
                    expected: *declared,
                });
                continue;
            }
            if let ExternArgument::Consume(resource) = argument {
                consumed_here.push(*resource);
            }
        }
        match (&call.adapter, callee.result) {
            (None, CallResult::OwnedNullable) => {
                diagnostics.push(ExternDiagnostic::AdapterMissing { event: index });
            }
            (Some(_), CallResult::Data(_)) => {
                diagnostics.push(ExternDiagnostic::AdapterUnexpected { event: index });
            }
            (Some(adapter), CallResult::OwnedNullable) => {
                if !program.errors.contains(&adapter.error) {
                    diagnostics.push(ExternDiagnostic::AdapterUnknownError {
                        event: index,
                        error: adapter.error.clone(),
                    });
                }
                // Adapter totality with a bounded error arm: the null path
                // takes the typed error exit, and v1 admits that exit only
                // where it abandons nothing.
                if let Some(resource) = live.iter().next().copied() {
                    diagnostics.push(ExternDiagnostic::AdapterAbandonsCustody {
                        event: index,
                        resource,
                    });
                }
                if !introduced.insert(adapter.resource) {
                    diagnostics.push(ExternDiagnostic::DuplicateAcquisition {
                        event: index,
                        resource: adapter.resource,
                    });
                }
            }
            (None, CallResult::Data(_)) => {}
        }
        for resource in &consumed_here {
            live.remove(resource);
            consumptions.push(*resource);
        }
        if let (Some(adapter), CallResult::OwnedNullable) = (&call.adapter, callee.result) {
            acquisitions.push(adapter.resource);
            // A failed acquisition never makes the identity live; the checked
            // trace keeps the static identity so the caller's passes still see
            // one binding, and the reference/native runs take the error arm.
            if adapter.succeeds {
                live.insert(adapter.resource);
            }
        }
        calls.push((callee.name.clone(), derived_custody(callee)));
    }
    if !diagnostics.is_empty() {
        return Err(diagnostics);
    }
    let mut assumptions = program
        .callees
        .keys()
        .map(|name| ExternAssumption::DeclaredContract(name.clone()))
        .collect::<Vec<_>>();
    assumptions.push(ExternAssumption::NoExtentTransport);
    assumptions.push(ExternAssumption::LinkageSuppliedByBuilder);
    assumptions.sort();
    Ok(ExternCallCertificate {
        entry: program.entry.clone(),
        calls,
        acquisitions,
        consumptions,
        effects: EffectRow::new(effects),
        assumptions,
    })
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
