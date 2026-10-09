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
