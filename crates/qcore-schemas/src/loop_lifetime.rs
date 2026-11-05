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
    /// Explicit named assumption; never inferred from absence of local writes.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub entry_snapshot_stable: bool,
    pub bindings: Vec<ConditionBinding>,
    pub blocks: Vec<LocalBlock>,
    pub registered: Vec<RegisteredProtocol>,
    /// Card MK-EVENT-CELL-CROSS-FUNCTION (rule 38c): `quant cell` record
    /// protocols, one per record. Omitted when empty, so every declaration
    /// before the card serializes as it did.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub cells: Vec<crate::descriptor_cell::CellProtocol>,
    /// Card CELL-PROTOCOL-RELEASE-WINDOW-FRAMES (rule 38g): `quant cell
    /// CALLEE: TAG |-> #untouched;` callee frames, sorted. Omitted when
    /// empty, so every declaration before the card serializes as it did.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub frames: Vec<crate::descriptor_cell::CellFrame>,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Root {
    Formal { scope: String, index: usize },
    Result { scope: String },
    Lifted(String),
}
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Path {
    pub root: Root,
    pub fields: Vec<String>,
}
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Registration {
    pub object: Path,
    pub holder: Path,
    pub poisoned: bool,
}
/// A body forwards this exact caller-owned registration under an explicitly
/// attributed declared protocol. It is not a discharged caller release check.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ForwardedLifetime {
    pub host: String,
    pub premise: String,
    pub registration: Registration,
}

/// A source-checked preservation frame for one exact formal-rooted holder.
/// It conveys no ordinary custody or record-wide immutability guarantee.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct HolderFrame {
    pub parameter: usize,
    pub record: String,
    pub fields: Vec<String>,
}

/// Same-unit inference retains custody and its outgoing lifetime effects as
/// one record. Ordinary custody consumers cannot read this as a bare borrow.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct InferredLifetimeSummary {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub holder_frames: Vec<HolderFrame>,
    pub contract: crate::contract::ResourceFunctionContract,
    pub effects: Vec<ForwardedLifetime>,
    /// A removal-only trace in source execution order. Mixed effect classes
    /// require a richer consumer and are withheld, never reordered.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub removals: Vec<crate::retained_by::Removal>,
}
