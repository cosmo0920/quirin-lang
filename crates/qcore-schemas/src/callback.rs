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

//! Callback contracts (card **CG22-CALLBACK-REGISTRY-CONTRACT**; contract
//! spec §C04.19, grammar G05, gateway rules 13u–13z).
//!
//! USER DECISION 2026-10-05 (`W48-CALLEE-DECISION-TABLE` class 1, shape (c)):
//! callbacks over retained elements go through callback contracts. A callee
//! that calls a function pointer its caller handed it states which pointer,
//! when and with what:
//!
//! ```text
//! ensures normal resource_callback_invoked(f, a, _, c);         // during the call only
//! ensures normal resource_callback_table(t, a, _, c);           // handlers read from t
//! ensures normal resource_callback_registered(f, h);            // h keeps f
//! ensures normal resource_callback_registry_invoked(h, element); // h calls what it keeps
//! ensures normal resource_callback_unregistered(h);
//! ensures normal resource_element_added(e, h);
//! ensures normal resource_elements_dropped(h);
//! ensures normal resource_released_if_constructed(h);
//! ```
//!
//! A declaration carrying any of these is **never** an entry of
//! [`crate::contract::ResourceContracts::functions`]: it is withheld with the
//! retaining declarations ([`crate::retained_by::RetainingContract`]), so a
//! consumer that knows nothing of callbacks sees an uncontracted callee, and a
//! callback is never read as a benign borrowed pointer.

use serde::{Deserialize, Serialize};

/// What the callback receives at one of its positions.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "actual")]
pub enum CallbackActual {
    /// The actual the caller passed at this parameter of the declaration,
    /// unchanged.
    Parameter { index: usize },
    /// `_`: a value the callee supplies itself (an index, a pointer into an
    /// object it was handed, a buffer of its own), naming no object of the
    /// caller's but through the declaration's own parameters.
    Supplied,
    /// `element` (a registry invocation only): each element the holder holds.
    Element,
    /// Card **CG22-OP-CALLBACKS-AND-SERVED-QUEUES** (§C04.19.1, rule 12m):
    /// `supplied_owned`, a live object the callee supplies and owns, whose
    /// custody it hands to the callback at each invocation (an op
    /// `rd_kafka_q_serve0` dequeued). The callback must take it on every
    /// return; the caller's state never holds it.
    SuppliedOwned,
}

/// Card **CG22-OP-CALLBACKS-AND-SERVED-QUEUES** (§C04.19.3, rule 12n): a
/// function-pointer member `TAG.FIELD`, the dotted path `FIELD` read from a
/// `struct TAG` object (`rd_kafka_op_s.rko_op_cb`,
/// `rd_kafka_op_s.rko_u.leaders.cb`).
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct CallbackMember {
    pub tag: String,
    /// The member path below the record, segments joined by `.`.
    pub path: String,
}

impl CallbackMember {
    /// The spelling `TAG.FIELD`.
    #[must_use]
    pub fn spelling(&self) -> String {
        format!("{}.{}", self.tag, self.path)
    }
}

/// Whose member a member invocation reads.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "object")]
pub enum MemberObject {
    /// The object the parameter at this index designates.
    Parameter { index: usize },
    /// Each object the callee supplies at its `supplied_owned` position.
    SuppliedOwned,
}

/// Who keeps a registered callback.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "holder")]
pub enum CallbackHolder {
    /// The object the parameter at this index designates (an in-place
    /// registration: `rd_list_init(rl, …, free_cb)`).
    Parameter { index: usize },
    /// The object the call returns, which `resource_return_owned(nonnull)`
    /// hands the caller (a constructor: `rd_list_new(…, free_cb)`).
    Result,
}

/// One callback clause, positions indexing the declaration's parameters.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "clause")]
pub enum CallbackClause {
    /// Rule 13u: during the call only, the function pointer at `callback`
    /// is called zero or more times with `actuals`; nothing keeps it.
    Invoked {
        callback: usize,
        actuals: Vec<CallbackActual>,
    },
    /// Rule 13v: during the call only, function pointers read from the array
    /// the parameter at `table` designates are called zero or more times with
    /// `actuals`.
    Table {
        table: usize,
        actuals: Vec<CallbackActual>,
    },
    /// Rule 13w: after the call `holder` keeps the function pointer at
    /// `callback` (a null pointer: no callback), replacing any earlier one.
    Registered {
        callback: usize,
        holder: CallbackHolder,
    },
    /// Rule 13w: during the call the callback `holder` keeps (if any) is
    /// called zero or more times with `actuals`.
    RegistryInvoked {
        holder: usize,
        actuals: Vec<CallbackActual>,
    },
    /// Rule 13w: after the call `holder` keeps no registration.
    Unregistered { holder: usize },
    /// Rule 13x: the object `element` designates becomes an element `holder`
    /// holds.
    ElementAdded { element: usize, holder: usize },
    /// Rule 13x: after the call `holder` holds no element.
    ElementsDropped { holder: usize },
    /// Rule 13x: the call releases `holder`'s object exactly when its
    /// registration in force was made by a constructor ([`CallbackHolder::Result`]).
    ReleasedIfConstructed { holder: usize },
    /// Card **CG22-OP-CALLBACKS-AND-SERVED-QUEUES** (rule 12n): during the
    /// call the function pointer stored in `object`'s `member` is called zero
    /// or more times with `actuals`. Its candidates are the run's closed set
    /// of stores into `member` (premise `P-CG22-MEMBER-CLOSED-WORLD`).
    MemberInvoked {
        object: MemberObject,
        member: CallbackMember,
        actuals: Vec<CallbackActual>,
    },
    /// Card **OP-APP-RK-CALLBACKS-VIA-POLL-CB** (rule 12n; user decision Q2):
    /// after the call the object the parameter at `object` designates holds
    /// the function pointer at `callback` in its `member`
    /// (`resource_callback_member_registered(conf, TAG.FIELD, cb)`, a setter).
    /// For the run's closure every call of the declaration is a store of its
    /// `callback` actual into `member`; a declared assumption.
    ///
    /// Card **FB-EVENT-LOOP-RETENTION-AND-REGISTRATION** (rule 12n; USER
    /// DECISION 2026-10-16, "Result + list"): with `by_id`, the clause is
    /// `resource_callback_member_registered(#id, TAG.FIELD, F) where #id =
    /// #result in P.LIST;` and the record holding the member is the one the
    /// call links into `object`'s member path `by_id.list` and names by the
    /// integer it returns; `object` is the list's formal. Consumers read it
    /// as the setter form; the run's closure reads it as a store of
    /// `callback`'s actual.
    MemberRegistered {
        object: usize,
        member: CallbackMember,
        callback: usize,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        by_id: Option<IdLink>,
    },
}

/// Card **FB-EVENT-LOOP-RETENTION-AND-REGISTRATION**: the `where #ID =
/// #result in P.LIST` binding of a registration forwarded through an id.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct IdLink {
    /// The binder's name, without its `#` (`id`).
    pub binder: String,
    /// The member path of the list below the formal, outermost first.
    pub list: Vec<String>,
}

impl CallbackClause {
    /// The stable keyword of the clause.
    #[must_use]
    pub const fn keyword(&self) -> &'static str {
        match self {
            Self::Invoked { .. } => "resource_callback_invoked",
            Self::Table { .. } => "resource_callback_table",
            Self::Registered { .. } => "resource_callback_registered",
            Self::RegistryInvoked { .. } => "resource_callback_registry_invoked",
            Self::Unregistered { .. } => "resource_callback_unregistered",
            Self::ElementAdded { .. } => "resource_element_added",
            Self::ElementsDropped { .. } => "resource_elements_dropped",
            Self::ReleasedIfConstructed { .. } => "resource_released_if_constructed",
            Self::MemberInvoked { .. } => "resource_callback_member_invoked",
            Self::MemberRegistered { .. } => "resource_callback_member_registered",
        }
    }

    /// Whether the clause belongs to a registry (rules 13w, 13x): it needs a
    /// registration in force at its holder.
    #[must_use]
    pub const fn needs_registration(&self) -> bool {
        matches!(
            self,
            Self::RegistryInvoked { .. }
                | Self::Unregistered { .. }
                | Self::ElementAdded { .. }
                | Self::ElementsDropped { .. }
                | Self::ReleasedIfConstructed { .. }
        )
    }
}
