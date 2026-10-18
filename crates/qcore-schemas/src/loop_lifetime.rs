//! Portable S3 lifetime protocol and scoped-condition syntax. No source fact
//! or declared condition is a proof: admission belongs to the run checker.
use serde::{Deserialize, Serialize};

/// A read-only Boolean binding, true exactly when an integer member is nonzero.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ConditionBinding {
    pub name: String,
    pub parameter: usize,
    pub members: Vec<String>,
}

/// A block activates only under this polarity of its bound condition.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ConditionGuard {
    pub condition: String,
    pub negated: bool,
}

/// A record-type footprint. The binder is local to this clause, never the
/// hosting function's formal or the root of an enclosing condition binding.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct RegisteredProtocol {
    pub binder: String,
    pub record: String,
    pub interior: Vec<String>,
    pub holder: Vec<String>,
}

/// The supported inner syntax of a local condition block.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum LocalClause {
    Prerequisite(String),
    Registered(RegisteredProtocol),
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct LocalBlock {
    pub guard: ConditionGuard,
    pub clauses: Vec<LocalClause>,
}

/// Kept separately from ordinary custody declarations: a consumer that knows
/// nothing of condition/lifetime protocols must see an uncontracted callee.
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct LoopLifetimeContract {
    pub bindings: Vec<ConditionBinding>,
    pub blocks: Vec<LocalBlock>,
    pub registered: Vec<RegisteredProtocol>,
}
