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

//! Card **CG19-ATOMIC-SOURCE-ROUTE** (gateway rule 11i): C atomics, read from
//! Clang's typed AST, routed to the core weak-memory model
//! (`qcore_core::weak_memory`; gap entries W01 and W09, evidence Q10, Q34,
//! Q35, Q39 and Q46).
//!
//! # What is routed
//!
//! [`ATOMIC_FORM_TABLE`] lists every atomic builtin this route names, with
//! its [`FormClass`]. Clang lowers `<stdatomic.h>`'s generic functions to the
//! `__c11_atomic_*` builtins (`atomic_load(p)` is `__c11_atomic_load(p, 5)`),
//! so the table covers them through their builtin names; GNU's `__atomic_*`
//! forms are listed beside them. The `_Atomic` qualifier's operator forms are
//! read by [`classify_qualified`]: an lvalue conversion (Clang's
//! `AtomicToNonAtomic` cast over the read) is a sequentially consistent load,
//! a simple assignment to an `_Atomic` lvalue (its value converted by
//! `NonAtomicToAtomic`) a sequentially consistent store (C11 6.5.16.1p2 with
//! 7.17.7), and a compound assignment or increment a read-modify-write.
//!
//! A routed access ([`RoutedAtomic`]) is mapped to the model's own
//! vocabulary ([`RoutedAtomic::event`]) with an order the model represents
//! for that operation ([`qcore_core::event_order_represented`]): a load, a
//! store and the non-atomic initialisation `__c11_atomic_init`
//! (`AtomicLoad`/`AtomicStore`/`AtomicInit`, card CG19), and, since card
//! **CG28-WEAK-MEMORY-EVENTS** (gateway rule 11t), a read-modify-write
//! (exchange, fetch-and-op, op-and-fetch, test-and-set, and the `_Atomic`
//! compound assignment and increment: `AtomicReadModifyWrite`), a
//! compare-exchange with its success and failure orders and its strength
//! (`AtomicCompareExchange`), and a thread or signal fence (`AtomicFence`).
//!
//! # What is refused, by name
//!
//! `memory_order_consume` stays out of the model's scope (CG28's decision:
//! the model has no dependency ordering, and reading consume as acquire would
//! add happens-before edges the program lacks, unsound for race freedom), so
//! a consume load, read-modify-write or compare-exchange order is refused by
//! name ([`AtomicRouteRefusal`]); a consume *fence* is an acquire fence by
//! C11 7.17.4.1p2's own definition. The legacy `__sync` builtins are GCC full
//! barriers with no C11 order and are refused. An order C11 7.17.7 does not
//! allow for the operation (a release load, an acquire store, a failure order
//! that is release, acq_rel or stronger than the success order) is refused;
//! so is an order or a weak flag the route cannot evaluate, a generic
//! by-pointer load, store or exchange (`__atomic_load(p, &out, o)`), a
//! pointer- or aggregate-valued atomic, and every builtin the table does not
//! list.
//!
//! # What a routed access is not
//!
//! A routed access is one event of one candidate execution. The model checks
//! **one supplied graph** ([`SINGLE_GRAPH_ASSUMPTION`]): nothing here is race
//! freedom over every execution (W07), and nothing is liveness or eventual
//! observation (W11). Only the atomic-publication verdict, which also runs
//! card CG29's enumerator over its extracted shape, claims race freedom over
//! every execution, within bounds ([`ALL_EXECUTIONS_ASSUMPTION`]). Route G admits a routed plain access only for
//! its custody, which is order-insensitive; it publishes no ordering claim.
//!
//! The Lean model is `QCore.AtomicSourceRoute`, bound to [`ATOMIC_FORM_TABLE`],
//! [`decode_order`] and [`route_order`] by the shared-vector tests in
//! `tests/unit/atomic_source_route.rs`.

use serde_json::Value;

use qcore_core::{
    CompareExchangeStrength, FenceScope, MemoryEventKind, MemoryLocation, MemoryOrder,
    compare_exchange_failure_admitted, event_order_represented,
};

use crate::typed_c_ast::{children, kind};

/// The assumption every verdict built on a routed access carries, unless
/// its program was also checked over every execution
/// ([`ALL_EXECUTIONS_ASSUMPTION`]).
pub const SINGLE_GRAPH_ASSUMPTION: &str = "the weak-memory model checked one supplied candidate execution graph; this is not race freedom over every execution of the program (W07, CG29) and claims no liveness or eventual observation (W11)";

/// Card CG29-EXECUTION-COVERAGE-AND-RECLAMATION (gateway rule 11z): the
/// assumption a `Safe` atomic-publication verdict carries in place of
/// [`SINGLE_GRAPH_ASSUMPTION`]. The extracted shape (one initialised flag,
/// one publisher, one guarded consumer) was checked over every candidate
/// execution by `qcore_core::execution_coverage` within the stated bounds.
pub const ALL_EXECUTIONS_ASSUMPTION: &str = "every candidate execution of the extracted publication fragment (one flag initialisation, one publisher, one consumer guarded by its load; threads <= 2, events <= 7, values <= 2 per location, candidates <= 16) was checked by qcore_core::execution_coverage (W07, CG29): race freedom over all its executions within these bounds; the invocation bounds stay caller-declared, and no liveness or eventual observation is claimed (W11)";

/// What one listed atomic form does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FormClass {
    /// A plain atomic load: `AtomicLoad`.
    Load,
    /// A plain atomic store: `AtomicStore`.
    Store,
    /// The non-atomic initialisation of an atomic object: `AtomicInit`.
    Init,
    /// A generic by-pointer load, store or exchange: the value travels
    /// through a second pointer this route does not read. Refused.
    ByPointer,
    /// Exchange, fetch-and-op, op-and-fetch and test-and-set:
    /// `AtomicReadModifyWrite` (CG28).
    ReadModifyWrite,
    /// Compare-exchange, strong or weak: `AtomicCompareExchange` (CG28).
    CompareExchange,
    /// A thread fence: `AtomicFence` of thread scope (CG28).
    Fence,
    /// A signal fence: `AtomicFence` of signal scope (CG28).
    SignalFence,
    /// The legacy `__sync` builtins: GCC full barriers, not C11 memory
    /// orders. Refused (CG28).
    LegacySync,
}

/// Every atomic builtin this route names (`QCore.AtomicSourceRoute.formTable`).
/// A builtin not listed here is refused as unlisted ([`AtomicRouteRefusal::Unlisted`]).
pub const ATOMIC_FORM_TABLE: [(&str, FormClass); 51] = [
    ("__c11_atomic_load", FormClass::Load),
    ("__atomic_load_n", FormClass::Load),
    ("__c11_atomic_store", FormClass::Store),
    ("__atomic_store_n", FormClass::Store),
    ("__c11_atomic_init", FormClass::Init),
    ("__atomic_load", FormClass::ByPointer),
    ("__atomic_store", FormClass::ByPointer),
    ("__c11_atomic_exchange", FormClass::ReadModifyWrite),
    ("__atomic_exchange_n", FormClass::ReadModifyWrite),
    ("__atomic_exchange", FormClass::ByPointer),
    ("__c11_atomic_fetch_add", FormClass::ReadModifyWrite),
    ("__c11_atomic_fetch_sub", FormClass::ReadModifyWrite),
    ("__c11_atomic_fetch_and", FormClass::ReadModifyWrite),
    ("__c11_atomic_fetch_or", FormClass::ReadModifyWrite),
    ("__c11_atomic_fetch_xor", FormClass::ReadModifyWrite),
    ("__c11_atomic_fetch_nand", FormClass::ReadModifyWrite),
    ("__c11_atomic_fetch_max", FormClass::ReadModifyWrite),
    ("__c11_atomic_fetch_min", FormClass::ReadModifyWrite),
    ("__atomic_fetch_add", FormClass::ReadModifyWrite),
    ("__atomic_fetch_sub", FormClass::ReadModifyWrite),
    ("__atomic_fetch_and", FormClass::ReadModifyWrite),
    ("__atomic_fetch_or", FormClass::ReadModifyWrite),
    ("__atomic_fetch_xor", FormClass::ReadModifyWrite),
    ("__atomic_fetch_nand", FormClass::ReadModifyWrite),
    ("__atomic_fetch_max", FormClass::ReadModifyWrite),
    ("__atomic_fetch_min", FormClass::ReadModifyWrite),
    ("__atomic_add_fetch", FormClass::ReadModifyWrite),
    ("__atomic_sub_fetch", FormClass::ReadModifyWrite),
    ("__atomic_and_fetch", FormClass::ReadModifyWrite),
    ("__atomic_or_fetch", FormClass::ReadModifyWrite),
    ("__atomic_xor_fetch", FormClass::ReadModifyWrite),
    ("__atomic_nand_fetch", FormClass::ReadModifyWrite),
    ("__atomic_max_fetch", FormClass::ReadModifyWrite),
    ("__atomic_min_fetch", FormClass::ReadModifyWrite),
    ("__atomic_test_and_set", FormClass::ReadModifyWrite),
    ("__sync_fetch_and_add", FormClass::LegacySync),
    ("__sync_fetch_and_sub", FormClass::LegacySync),
    ("__sync_add_and_fetch", FormClass::LegacySync),
    ("__sync_sub_and_fetch", FormClass::LegacySync),
    ("__sync_lock_test_and_set", FormClass::LegacySync),
    (
        "__c11_atomic_compare_exchange_strong",
        FormClass::CompareExchange,
    ),
    (
        "__c11_atomic_compare_exchange_weak",
        FormClass::CompareExchange,
    ),
    ("__atomic_compare_exchange_n", FormClass::CompareExchange),
    ("__atomic_compare_exchange", FormClass::CompareExchange),
    ("__sync_bool_compare_and_swap", FormClass::LegacySync),
    ("__sync_val_compare_and_swap", FormClass::LegacySync),
    ("__c11_atomic_thread_fence", FormClass::Fence),
    ("__c11_atomic_signal_fence", FormClass::SignalFence),
    ("__atomic_thread_fence", FormClass::Fence),
    ("__atomic_signal_fence", FormClass::SignalFence),
    ("__sync_synchronize", FormClass::LegacySync),
];

/// The class of a listed builtin, `None` for an unlisted name.
#[must_use]
pub fn form_class(name: &str) -> Option<FormClass> {
    ATOMIC_FORM_TABLE
        .iter()
        .find(|(listed, _)| *listed == name)
        .map(|(_, class)| *class)
}

/// Whether `name` is an atomic builtin this route answers for: a listed one,
/// or one of the builtin families (`__atomic_`, `__c11_atomic_`, `__sync_`)
/// it refuses as unlisted. Route G consults the route only for these names.
#[must_use]
pub fn names_atomic_builtin(name: &str) -> bool {
    form_class(name).is_some()
        || name.starts_with("__atomic_")
        || name.starts_with("__c11_atomic_")
        || name.starts_with("__sync_")
}

/// A C11 memory order, decoded from Clang's builtin numbering
/// (`QCore.AtomicSourceRoute.decodeOrder`). `consume` and `acq_rel` are
/// decoded so they can be refused by name; the model represents neither for
/// a plain load or store.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceOrder {
    Relaxed,
    Consume,
    Acquire,
    Release,
    AcquireRelease,
    SequentiallyConsistent,
}

/// Clang's numbering: `__ATOMIC_RELAXED` 0 … `__ATOMIC_SEQ_CST` 5, which the
/// `memory_order` enumerators of `<stdatomic.h>` equal. Any other value is
/// no order.
#[must_use]
pub fn decode_order(value: i64) -> Option<SourceOrder> {
    match value {
        0 => Some(SourceOrder::Relaxed),
        1 => Some(SourceOrder::Consume),
        2 => Some(SourceOrder::Acquire),
        3 => Some(SourceOrder::Release),
        4 => Some(SourceOrder::AcquireRelease),
        5 => Some(SourceOrder::SequentiallyConsistent),
        _ => None,
    }
}

/// The operation a routed access performs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Operation {
    Load,
    Store,
    Init,
    /// CG28: exchange, fetch-and-op, op-and-fetch, test-and-set, and the
    /// `_Atomic` compound assignment and increment.
    ReadModifyWrite,
    /// CG28: a compare-exchange.
    CompareExchange,
    /// CG28: `atomic_thread_fence`.
    ThreadFence,
    /// CG28: `atomic_signal_fence`.
    SignalFence,
}

/// The model order of an operation's (success) order, or its refusal
/// (`QCore.AtomicSourceRoute.routeOrder`). C11 7.17.7.1p2: a store's order is
/// not consume, acquire or acq_rel; 7.17.7.2p2: a load's is not release or
/// acq_rel; an initialisation is not an atomic operation and takes no order;
/// a read-modify-write, a compare-exchange's success and a fence take any
/// order (card CG28). Consume is out of the model's scope (CG28's decision):
/// a consume load, read-modify-write or compare-exchange is refused by name,
/// because reading it as acquire would be a strengthening the program did not
/// ask for and unsound for race freedom; a consume fence is an acquire fence
/// by C11 7.17.4.1p2's own definition.
pub fn route_order(
    name: &str,
    operation: Operation,
    order: SourceOrder,
) -> Result<MemoryOrder, AtomicRouteRefusal> {
    use Operation as O;
    use SourceOrder as S;
    let general = matches!(
        operation,
        O::ReadModifyWrite | O::CompareExchange | O::ThreadFence | O::SignalFence
    );
    match (operation, order) {
        (O::Init, _) => Err(AtomicRouteRefusal::InvalidOrder(name.to_owned())),
        (O::Load, S::Consume) => Err(AtomicRouteRefusal::ConsumeLoad(name.to_owned())),
        (O::ReadModifyWrite | O::CompareExchange, S::Consume) => {
            Err(AtomicRouteRefusal::ConsumeOrder(name.to_owned()))
        }
        (O::ThreadFence | O::SignalFence, S::Consume) => Ok(MemoryOrder::Acquire),
        (_, S::Relaxed) => Ok(MemoryOrder::Relaxed),
        (O::Load, S::Acquire) => Ok(MemoryOrder::Acquire),
        (O::Store, S::Release) => Ok(MemoryOrder::Release),
        (O::Load | O::Store, S::SequentiallyConsistent) => Ok(MemoryOrder::SequentiallyConsistent),
        (_, S::Acquire) if general => Ok(MemoryOrder::Acquire),
        (_, S::Release) if general => Ok(MemoryOrder::Release),
        (_, S::AcquireRelease) if general => Ok(MemoryOrder::AcquireRelease),
        (_, S::SequentiallyConsistent) if general => Ok(MemoryOrder::SequentiallyConsistent),
        _ => Err(AtomicRouteRefusal::InvalidOrder(name.to_owned())),
    }
}

/// Card CG28: a compare-exchange's failure order, given its routed success
/// order (`QCore.AtomicSourceRoute.routeFailureOrder`). C11 7.17.7.4p2: not
/// release or acq_rel, and no stronger than the success order
/// ([`qcore_core::compare_exchange_failure_admitted`]); consume is refused by
/// name, as for every non-fence operation.
pub fn route_failure_order(
    name: &str,
    success: MemoryOrder,
    failure: SourceOrder,
) -> Result<MemoryOrder, AtomicRouteRefusal> {
    let failure = match failure {
        SourceOrder::Consume => return Err(AtomicRouteRefusal::ConsumeOrder(name.to_owned())),
        SourceOrder::Relaxed => MemoryOrder::Relaxed,
        SourceOrder::Acquire => MemoryOrder::Acquire,
        SourceOrder::SequentiallyConsistent => MemoryOrder::SequentiallyConsistent,
        SourceOrder::Release | SourceOrder::AcquireRelease => {
            return Err(AtomicRouteRefusal::InvalidOrder(name.to_owned()));
        }
    };
    if compare_exchange_failure_admitted(success, failure) {
        Ok(failure)
    } else {
        Err(AtomicRouteRefusal::InvalidOrder(name.to_owned()))
    }
}

/// One routed atomic access.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RoutedAtomic<'a> {
    /// The builtin's name, or the qualified form's row name.
    pub form: String,
    pub operation: Operation,
    /// `None` for [`Operation::Init`], which is not an atomic operation; a
    /// compare-exchange's success order.
    pub order: Option<MemoryOrder>,
    /// The pointer operand (the `_Atomic` lvalue for a qualified form; the
    /// fence call itself for a fence, which has no object).
    pub object: &'a Value,
    /// The stored, operand or desired value; `None` for a load, a fence, a
    /// test-and-set and an increment (whose operand is implicit).
    pub value: Option<&'a Value>,
    /// The C type of the value loaded or stored, as Clang spells it (empty
    /// for a fence).
    pub value_type: String,
    /// CG28: a compare-exchange's failure order.
    pub failure_order: Option<MemoryOrder>,
    /// CG28: a compare-exchange's strength.
    pub strength: Option<CompareExchangeStrength>,
    /// CG28: a compare-exchange's `expected` pointer operand.
    pub expected: Option<&'a Value>,
    /// CG28: `value` points at the desired value (the generic
    /// `__atomic_compare_exchange`) instead of being it.
    pub value_by_pointer: bool,
}

/// CG28: the values one candidate execution gives a routed access's event.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EventValues {
    /// A load's or store's value, an initialisation's initial value.
    Plain(i64),
    ReadModifyWrite {
        read: i64,
        written: i64,
    },
    CompareExchange {
        expected: i64,
        observed: i64,
        desired: i64,
        succeeded: bool,
    },
    Fence,
}

impl RoutedAtomic<'_> {
    /// The model event this access is, at `location` with `values`
    /// (`QCore.AtomicSourceRoute.eventOf`); `None` when the values are not
    /// those of the access's operation.
    #[must_use]
    pub fn event(&self, location: MemoryLocation, values: EventValues) -> Option<MemoryEventKind> {
        Some(match (self.operation, self.order, values) {
            (Operation::Load, Some(order), EventValues::Plain(value)) => {
                MemoryEventKind::AtomicLoad {
                    location,
                    value,
                    order,
                }
            }
            (Operation::Store, Some(order), EventValues::Plain(value)) => {
                MemoryEventKind::AtomicStore {
                    location,
                    value,
                    order,
                }
            }
            (Operation::Init, None, EventValues::Plain(value)) => {
                MemoryEventKind::AtomicInit { location, value }
            }
            (
                Operation::ReadModifyWrite,
                Some(order),
                EventValues::ReadModifyWrite { read, written },
            ) => MemoryEventKind::AtomicReadModifyWrite {
                location,
                read_value: read,
                written_value: written,
                order,
            },
            (
                Operation::CompareExchange,
                Some(success),
                EventValues::CompareExchange {
                    expected,
                    observed,
                    desired,
                    succeeded,
                },
            ) => MemoryEventKind::AtomicCompareExchange {
                location,
                expected,
                observed,
                desired,
                success,
                failure: self.failure_order?,
                strength: self.strength?,
                succeeded,
            },
            (Operation::ThreadFence, Some(order), EventValues::Fence) => {
                MemoryEventKind::AtomicFence {
                    order,
                    scope: FenceScope::Thread,
                }
            }
            (Operation::SignalFence, Some(order), EventValues::Fence) => {
                MemoryEventKind::AtomicFence {
                    order,
                    scope: FenceScope::Signal,
                }
            }
            _ => return None,
        })
    }

    /// The event of a plain access (CG19's `eventOf`); `None` for any other
    /// operation, whose event takes [`EventValues`] through [`Self::event`].
    #[must_use]
    pub fn event_kind(&self, location: MemoryLocation, value: i64) -> Option<MemoryEventKind> {
        self.event(location, EventValues::Plain(value))
    }

    /// Placeholder values of this access's own shape.
    #[must_use]
    pub fn shape_values(&self) -> EventValues {
        match self.operation {
            Operation::Load | Operation::Store | Operation::Init => EventValues::Plain(0),
            Operation::ReadModifyWrite => EventValues::ReadModifyWrite {
                read: 0,
                written: 0,
            },
            Operation::CompareExchange => EventValues::CompareExchange {
                expected: 0,
                observed: 0,
                desired: 0,
                succeeded: true,
            },
            Operation::ThreadFence | Operation::SignalFence => EventValues::Fence,
        }
    }
}

/// A named refusal of the route.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AtomicRouteRefusal {
    ConsumeLoad(String),
    /// CG28: `memory_order_consume` on a read-modify-write or a
    /// compare-exchange (success or failure).
    ConsumeOrder(String),
    /// CG28: a legacy `__sync` builtin.
    LegacySync(String),
    InvalidOrder(String),
    UnevaluatedOrder(String),
    /// CG28: a GNU compare-exchange whose weak flag is not an integer constant.
    UnevaluatedStrength(String),
    ByPointer(String),
    NonScalarValue(String),
    UnsupportedShape(String),
    Unlisted(String),
}

impl AtomicRouteRefusal {
    /// The refusal's text, naming the form and the owning card.
    #[must_use]
    pub fn reason(&self) -> String {
        match self {
            Self::ConsumeLoad(name) => format!(
                "memory_order_consume load `{name}` is not represented by the weak-memory model (CG28)"
            ),
            Self::ConsumeOrder(name) => format!(
                "memory_order_consume on `{name}` is outside the weak-memory model: dependency ordering is not modelled and acquire would be a strengthening (CG28)"
            ),
            Self::LegacySync(name) => format!(
                "legacy `{name}` is a GCC full barrier with no C11 memory order and is not routed to the weak-memory model (CG28)"
            ),
            Self::InvalidOrder(name) => {
                format!(
                    "memory order of atomic `{name}` is not valid for its operation (C11 7.17.7)"
                )
            }
            Self::UnevaluatedOrder(name) => format!(
                "memory order of atomic `{name}` is not an integer constant the atomic route evaluates (CG19)"
            ),
            Self::UnevaluatedStrength(name) => format!(
                "weak flag of compare-exchange `{name}` is not an integer constant the atomic route evaluates (CG28)"
            ),
            Self::ByPointer(name) => format!(
                "generic by-pointer atomic `{name}` is not routed to the weak-memory model (CG19)"
            ),
            Self::NonScalarValue(name) => format!(
                "atomic `{name}` of a pointer or aggregate value is not routed to the weak-memory model (CG19)"
            ),
            Self::UnsupportedShape(name) => format!(
                "atomic `{name}` has an operand shape the atomic route does not read (CG19)"
            ),
            Self::Unlisted(name) => format!("unlisted atomic form `{name}` (CG19)"),
        }
    }
}

fn type_of(node: &Value) -> Option<&str> {
    node["type"]["desugaredQualType"]
        .as_str()
        .or_else(|| node["type"]["qualType"].as_str())
}

fn strip_value(mut node: &Value) -> &Value {
    while matches!(
        kind(node),
        "ParenExpr" | "ImplicitCastExpr" | "ConstantExpr"
    ) {
        let [child] = children(node) else { break };
        node = child;
    }
    node
}

/// The order operand's value: an integer literal, a folded constant, or an
/// enumeration constant `enum_value` evaluates. `None` for anything else.
fn order_value(node: &Value, enum_value: &dyn Fn(&Value) -> Option<i64>) -> Option<i64> {
    let mut cursor = node;
    loop {
        if kind(cursor) == "ConstantExpr"
            && let Some(value) = cursor["value"].as_str()
        {
            return value.parse().ok();
        }
        match kind(cursor) {
            "ParenExpr" | "ImplicitCastExpr" | "ConstantExpr" => {
                let [child] = children(cursor) else {
                    return None;
                };
                cursor = child;
            }
            _ => break,
        }
    }
    match kind(cursor) {
        "IntegerLiteral" => cursor["value"].as_str()?.parse().ok(),
        "DeclRefExpr" if cursor["referencedDecl"]["kind"].as_str() == Some("EnumConstantDecl") => {
            enum_value(cursor)
        }
        _ => None,
    }
}

/// Whether a C type is a non-pointer arithmetic value the model's integer
/// values abstract: no `*`, no record, no array.
fn scalar_value_type(text: &str) -> bool {
    !text.is_empty()
        && !text.contains('*')
        && !text.contains('[')
        && !text.contains("struct ")
        && !text.contains("union ")
        && text != "void"
}

/// The class of an atomic node: an `AtomicExpr`, or a direct call of a
/// listed fence or `__sync` builtin. `None` for any other node.
#[must_use]
pub fn atomic_node_name(node: &Value) -> Option<String> {
    match kind(node) {
        "AtomicExpr" => node["name"].as_str().map(str::to_owned),
        "CallExpr" => {
            let callee = children(node).first()?;
            let callee = strip_value(callee);
            if kind(callee) != "DeclRefExpr" {
                return None;
            }
            let name = callee["referencedDecl"]["name"].as_str()?;
            names_atomic_builtin(name).then(|| name.to_owned())
        }
        _ => None,
    }
}

/// The enumerator resolver of a route that has only the function's AST
/// (routes G and T): `memory_order_consume` answers 1, its standard value,
/// and every other enumerator is unevaluated. Fail-closed whatever the unit's
/// enumeration says: 1 is admitted for no operation
/// (`QCore.AtomicSourceRoute.order_one_never_routed`), so the answer only
/// names a consume load as one.
#[must_use]
pub fn consume_only(node: &Value) -> Option<i64> {
    (node["referencedDecl"]["kind"].as_str() == Some("EnumConstantDecl")
        && node["referencedDecl"]["name"].as_str() == Some("memory_order_consume"))
    .then_some(1)
}

/// Route one atomic builtin node (`AtomicExpr`, or a builtin fence call).
/// `enum_value` evaluates a `memory_order` enumeration constant; a route that
/// cannot pass `|_| None` and refuses such an order by name.
pub fn classify_builtin<'a>(
    node: &'a Value,
    enum_value: &dyn Fn(&Value) -> Option<i64>,
) -> Result<RoutedAtomic<'a>, AtomicRouteRefusal> {
    let Some(name) = atomic_node_name(node) else {
        return Err(AtomicRouteRefusal::Unlisted(kind(node).to_owned()));
    };
    let class = form_class(&name).ok_or_else(|| AtomicRouteRefusal::Unlisted(name.clone()))?;
    let operation = match class {
        FormClass::Load => Operation::Load,
        FormClass::Store => Operation::Store,
        FormClass::Init => Operation::Init,
        FormClass::ReadModifyWrite => Operation::ReadModifyWrite,
        FormClass::CompareExchange => Operation::CompareExchange,
        FormClass::Fence => Operation::ThreadFence,
        FormClass::SignalFence => Operation::SignalFence,
        FormClass::ByPointer => return Err(AtomicRouteRefusal::ByPointer(name)),
        FormClass::LegacySync => return Err(AtomicRouteRefusal::LegacySync(name)),
    };
    let evaluate = |order: &Value| {
        let value = order_value(order, enum_value)
            .ok_or_else(|| AtomicRouteRefusal::UnevaluatedOrder(name.clone()))?;
        decode_order(value).ok_or_else(|| AtomicRouteRefusal::InvalidOrder(name.clone()))
    };
    let shape = || AtomicRouteRefusal::UnsupportedShape(name.clone());
    if matches!(operation, Operation::ThreadFence | Operation::SignalFence) {
        // Card CG28: a fence is a builtin call `NAME(order)`, not an
        // `AtomicExpr`; it has no object and no value.
        let [_, order] = children(node) else {
            return Err(shape());
        };
        if kind(node) != "CallExpr" {
            return Err(shape());
        }
        let order = route_order(&name, operation, evaluate(order)?)?;
        return Ok(RoutedAtomic {
            form: name,
            operation,
            order: Some(order),
            object: node,
            value: None,
            value_type: String::new(),
            failure_order: None,
            strength: None,
            expected: None,
            value_by_pointer: false,
        });
    }
    if kind(node) != "AtomicExpr" {
        return Err(shape());
    }
    // Clang's `AtomicExpr` children, in its storage order: the pointer, the
    // order, the first value, the failure order, the second value, the weak
    // flag, as many as the builtin takes. `__c11_atomic_init` has no order:
    // the pointer, then the value. A compare-exchange's first value is the
    // `expected` pointer and its second the desired value.
    let (object, order, value, expected, failure, weak) = match (operation, children(node)) {
        (Operation::Load, [object, order]) => (object, Some(order), None, None, None, None),
        (Operation::Store | Operation::ReadModifyWrite, [object, order, value]) => {
            (object, Some(order), Some(value), None, None, None)
        }
        // `__atomic_test_and_set`: its written value is implicit.
        (Operation::ReadModifyWrite, [object, order]) => {
            (object, Some(order), None, None, None, None)
        }
        (Operation::Init, [object, value]) => (object, None, Some(value), None, None, None),
        (Operation::CompareExchange, [object, success, expected, failure, desired]) => (
            object,
            Some(success),
            Some(desired),
            Some(expected),
            Some(failure),
            None,
        ),
        (Operation::CompareExchange, [object, success, expected, failure, desired, weak]) => (
            object,
            Some(success),
            Some(desired),
            Some(expected),
            Some(failure),
            Some(weak),
        ),
        _ => return Err(shape()),
    };
    let order = match order {
        None => None,
        Some(order) => Some(route_order(&name, operation, evaluate(order)?)?),
    };
    let failure_order = match (failure, order) {
        (Some(failure), Some(success)) => {
            Some(route_failure_order(&name, success, evaluate(failure)?)?)
        }
        _ => None,
    };
    let strength = match (operation, name.as_str()) {
        (Operation::CompareExchange, "__c11_atomic_compare_exchange_strong") => {
            Some(CompareExchangeStrength::Strong)
        }
        (Operation::CompareExchange, "__c11_atomic_compare_exchange_weak") => {
            Some(CompareExchangeStrength::Weak)
        }
        (Operation::CompareExchange, _) => {
            let flag = weak
                .and_then(|weak| order_value(weak, &|_| None))
                .ok_or_else(|| AtomicRouteRefusal::UnevaluatedStrength(name.clone()))?;
            Some(if flag == 0 {
                CompareExchangeStrength::Strong
            } else {
                CompareExchangeStrength::Weak
            })
        }
        _ => None,
    };
    // The generic `__atomic_compare_exchange` passes the desired value by
    // pointer; its value type is the pointee's: the named object's own
    // (desugared) type for `&x`, else the pointer type's pointee spelling.
    let value_by_pointer = name == "__atomic_compare_exchange";
    let value_type = match value {
        None => type_of(node).unwrap_or_default().to_owned(),
        Some(value) if value_by_pointer => {
            let named = (kind(value) == "UnaryOperator" && value["opcode"].as_str() == Some("&"))
                .then(|| children(value).first())
                .flatten()
                .map(strip_value)
                .and_then(type_of);
            named
                .or_else(|| pointee(type_of(value).unwrap_or_default()))
                .ok_or_else(|| AtomicRouteRefusal::NonScalarValue(name.clone()))?
                .to_owned()
        }
        Some(value) => type_of(value).unwrap_or_default().to_owned(),
    };
    if !scalar_value_type(&value_type) {
        return Err(AtomicRouteRefusal::NonScalarValue(name));
    }
    // The expected value is read and, on failure, written through a pointer
    // to the same scalar type.
    if let Some(expected) = expected
        && !pointee(type_of(expected).unwrap_or_default()).is_some_and(scalar_value_type)
    {
        return Err(AtomicRouteRefusal::NonScalarValue(name));
    }
    let routed = RoutedAtomic {
        form: name,
        operation,
        order,
        object,
        value,
        value_type,
        failure_order,
        strength,
        expected,
        value_by_pointer,
    };
    debug_assert!(
        routed
            .event(0, routed.shape_values())
            .is_some_and(|event| event_order_represented(&event))
    );
    Ok(routed)
}

/// The pointee of a pointer type spelling, `None` for a non-pointer.
fn pointee(text: &str) -> Option<&str> {
    Some(text.trim_end().strip_suffix('*')?.trim_end())
}

/// The `_Atomic` operator form a node is, when it is one
/// (`QCore.AtomicSourceRoute.qualifiedTable`).
pub const QUALIFIED_FORMS: [(&str, Option<FormClass>); 4] = [
    ("_Atomic lvalue conversion", Some(FormClass::Load)),
    ("_Atomic assignment", Some(FormClass::Store)),
    (
        "_Atomic compound assignment",
        Some(FormClass::ReadModifyWrite),
    ),
    (
        "_Atomic increment or decrement",
        Some(FormClass::ReadModifyWrite),
    ),
];

fn atomic_type(node: &Value) -> bool {
    type_of(node).is_some_and(|text| text.starts_with("_Atomic(") || text.contains(" _Atomic("))
}

/// The value type `T` of an `_Atomic(T)` lvalue.
fn atomic_value_type(node: &Value) -> Option<&str> {
    type_of(node)?
        .trim()
        .strip_prefix("_Atomic(")?
        .strip_suffix(')')
}

/// Route one `_Atomic` operator form, `None` for a node that is none. The
/// qualified forms are sequentially consistent (C11 6.5.2.4p2, 6.5.16.2p3,
/// 7.17.7): the lvalue conversion is a load, the simple assignment a store,
/// and (card CG28) the compound assignment and the increment or decrement
/// read-modify-writes.
#[must_use]
pub fn classify_qualified(node: &Value) -> Option<Result<RoutedAtomic<'_>, AtomicRouteRefusal>> {
    let routed = |form: &str, operation, object, value, value_type: String| {
        if !scalar_value_type(&value_type) {
            return Some(Err(AtomicRouteRefusal::NonScalarValue(form.to_owned())));
        }
        Some(Ok(RoutedAtomic {
            form: form.to_owned(),
            operation,
            order: Some(MemoryOrder::SequentiallyConsistent),
            object,
            value,
            value_type,
            failure_order: None,
            strength: None,
            expected: None,
            value_by_pointer: false,
        }))
    };
    match (kind(node), node["opcode"].as_str(), children(node)) {
        ("ImplicitCastExpr", _, [read])
            if node["castKind"].as_str() == Some("AtomicToNonAtomic") =>
        {
            let form = QUALIFIED_FORMS[0].0;
            let value_type = type_of(node).unwrap_or_default().to_owned();
            // The read under the conversion is Clang's `LValueToRValue` of
            // the `_Atomic` lvalue.
            let object = if kind(read) == "ImplicitCastExpr"
                && read["castKind"].as_str() == Some("LValueToRValue")
            {
                children(read).first()?
            } else {
                if !scalar_value_type(&value_type) {
                    return Some(Err(AtomicRouteRefusal::NonScalarValue(form.to_owned())));
                }
                return Some(Err(AtomicRouteRefusal::UnsupportedShape(form.to_owned())));
            };
            routed(form, Operation::Load, object, None, value_type)
        }
        ("BinaryOperator", Some("="), [left, right]) if atomic_type(left) => {
            let form = QUALIFIED_FORMS[1].0;
            let value = if kind(right) == "ImplicitCastExpr"
                && right["castKind"].as_str() == Some("NonAtomicToAtomic")
            {
                children(right).first()?
            } else {
                return Some(Err(AtomicRouteRefusal::UnsupportedShape(form.to_owned())));
            };
            let value_type = type_of(value).unwrap_or_default().to_owned();
            routed(form, Operation::Store, left, Some(value), value_type)
        }
        ("CompoundAssignOperator", _, [left, right]) if atomic_type(left) => {
            let form = QUALIFIED_FORMS[2].0;
            let value_type = atomic_value_type(left).unwrap_or_default().to_owned();
            routed(
                form,
                Operation::ReadModifyWrite,
                left,
                Some(right),
                value_type,
            )
        }
        ("UnaryOperator", Some("++" | "--"), [operand]) if atomic_type(operand) => {
            let form = QUALIFIED_FORMS[3].0;
            let value_type = atomic_value_type(operand).unwrap_or_default().to_owned();
            routed(form, Operation::ReadModifyWrite, operand, None, value_type)
        }
        _ => None,
    }
}

/// Every atomic node under `node`, outermost first, each with its route
/// answer: builtins through [`classify_builtin`], qualified forms through
/// [`classify_qualified`]. A consumer that must answer for every atomic in a
/// subtree (the typed-source import, the publication extractor) walks this.
pub fn atomic_nodes<'a>(
    node: &'a Value,
    enum_value: &dyn Fn(&Value) -> Option<i64>,
    out: &mut Vec<Result<RoutedAtomic<'a>, AtomicRouteRefusal>>,
) {
    if atomic_node_name(node).is_some() {
        out.push(classify_builtin(node, enum_value));
    } else if let Some(answer) = classify_qualified(node) {
        out.push(answer);
    }
    for child in children(node) {
        atomic_nodes(child, enum_value, out);
    }
}

/// Route T's refusal of an `_Atomic`-qualified type: its own arm, apart
/// from the `volatile`/`restrict` arm (absorbs
/// CT41-TYPED-SOURCE-QUALIFIER-ARM-STILL-UNDISCRIMINATING). No typed-source
/// consumer reads an atomic access, so the arm names the route that does.
#[must_use]
pub fn typed_source_qualifier_refusal(c_type: &str) -> String {
    format!(
        "atomic-qualified type `{c_type}` is read by the atomic source route, which no typed-source consumer decides (CG19)"
    )
}

/// Route T's refusal of an atomic builtin node, `None` for any other node:
/// the route's named refusal, or, for a routed plain access, that no
/// typed-source consumer decides one.
#[must_use]
pub fn typed_source_refusal(node: &Value) -> Option<String> {
    atomic_node_name(node)?;
    Some(match classify_builtin(node, &consume_only) {
        Ok(routed) => format!(
            "atomic `{}` is routed to the weak-memory model, which no typed-source consumer decides (CG19)",
            routed.form
        ),
        Err(refusal) => refusal.reason(),
    })
}

#[cfg(test)]
#[path = "../tests/unit/atomic_source_route.rs"]
mod tests;
