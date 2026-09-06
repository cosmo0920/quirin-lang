//! Resource-contract data model shared by the legacy-C analyzer, the
//! contract loader, and (future) providers.
//!
//! Moved verbatim from `qcore-compiler`'s `legacy_c` module (W0 of
//! `docs/GENERALIZATION_PLAN.md`). Types only: the analyzer and the
//! source-assumption recheck stay in the compiler crate.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResourceApiKind {
    Acquire,
    Release,
    Escape,
    /// A trusted resource-neutral call. Internal definitions never establish
    /// this contract automatically; a neutral summary for an internal function
    /// has to be inferred and validated like any other (see [`ContractTrust`]).
    Neutral,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResourceParameterMode {
    /// The call observes the argument without requiring resource custody.
    Borrowed,
    /// The caller must retain a known live resource across the call.
    Owned { nullable: bool },
    /// Custody moves to the callee, without claiming that the callee destroys it.
    Consumed { nullable: bool },
    /// The trusted call destroys the resource.
    Released { nullable: bool },
    /// The call initializes the pointed-to destination with fresh custody.
    Out { nullable: bool },
    /// The callee stores this resource under the resource passed at
    /// `owner_parameter`. Destruction or transfer of the owner cascades to it.
    Stored {
        owner_parameter: usize,
        nullable: bool,
    },
    /// The callee removes this resource from the aggregate passed at
    /// `owner_parameter`, restoring direct custody to the caller.
    Removed {
        owner_parameter: usize,
        nullable: bool,
    },
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResourceReturnMode {
    #[default]
    None,
    Owned {
        nullable: bool,
    },
    /// On success the result replaces `parameter`; on failure the old resource
    /// remains live. A nullable result therefore requires path refinement
    /// before the replacement can be treated as complete.
    Reallocated {
        parameter: usize,
        nullable: bool,
    },
}

#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ResourceFunctionContract {
    pub parameters: Vec<ResourceParameterMode>,
    pub result: ResourceReturnMode,
}

impl ResourceFunctionContract {
    #[must_use]
    pub fn new(
        parameters: impl IntoIterator<Item = ResourceParameterMode>,
        result: ResourceReturnMode,
    ) -> Self {
        Self {
            parameters: parameters.into_iter().collect(),
            result,
        }
    }
}

impl From<ResourceApiKind> for ResourceFunctionContract {
    fn from(kind: ResourceApiKind) -> Self {
        match kind {
            ResourceApiKind::Acquire => {
                Self::new([], ResourceReturnMode::Owned { nullable: false })
            }
            ResourceApiKind::Release => Self::new(
                [ResourceParameterMode::Released { nullable: false }],
                ResourceReturnMode::None,
            ),
            ResourceApiKind::Escape => Self::new(
                [ResourceParameterMode::Consumed { nullable: false }],
                ResourceReturnMode::None,
            ),
            ResourceApiKind::Neutral => Self::default(),
        }
    }
}

/// How much trust a [`ResourceFunctionContract`] carries.
///
/// The three producers of a contract — a loaded `.qcore` module, the C
/// gateway's interprocedural inference (W4 of `docs/GENERALIZATION_PLAN.md`),
/// and a verifier plugin — emit the *same* data with different trust labels.
/// The label is not a verdict: `Declared` is a trusted assumption that no
/// checker discharged, `InferredValidated` records that the gateway re-ran its
/// intraprocedural analyzer against the summary, and `Proposed` is unchecked
/// until the certificate firewall validates it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContractTrust {
    /// Loaded from a trusted `.qcore` contract module. An assumption.
    Declared,
    /// Inferred from a function body and validated by re-running the
    /// intraprocedural analyzer against the inferred summary.
    InferredValidated,
    /// Proposed by a verifier plugin. Unchecked until the core validates it;
    /// never sufficient on its own for a `Safe` verdict.
    Proposed,
}

/// One custody summary with its provenance. Inferred summaries, loaded
/// contracts, and provider proposals share this record shape so a consumer
/// can route them by trust label rather than by origin-specific types.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ResourceSummaryRecord {
    pub function: String,
    pub trust: ContractTrust,
    pub contract: ResourceFunctionContract,
}

impl ResourceSummaryRecord {
    #[must_use]
    pub fn new(
        function: impl Into<String>,
        trust: ContractTrust,
        contract: ResourceFunctionContract,
    ) -> Self {
        Self {
            function: function.into(),
            trust,
            contract,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResourceContracts {
    functions: BTreeMap<String, ResourceFunctionContract>,
    // Exact trusted inputs travel with the environment into safety certificates.
    source_assumptions: BTreeMap<String, String>,
}

impl ResourceContracts {
    #[must_use]
    pub fn new(contracts: impl IntoIterator<Item = (impl Into<String>, ResourceApiKind)>) -> Self {
        Self {
            functions: contracts
                .into_iter()
                .map(|(name, kind)| (name.into(), ResourceFunctionContract::from(kind)))
                .collect(),
            source_assumptions: BTreeMap::new(),
        }
    }

    pub fn insert(&mut self, name: impl Into<String>, kind: ResourceApiKind) {
        self.functions
            .insert(name.into(), ResourceFunctionContract::from(kind));
    }

    pub fn insert_contract(&mut self, name: impl Into<String>, contract: ResourceFunctionContract) {
        self.functions.insert(name.into(), contract);
    }

    /// Exact source inputs, declared as assumptions rather than verified facts.
    #[must_use]
    pub fn source_assumptions(&self) -> &BTreeMap<String, String> {
        &self.source_assumptions
    }

    /// Declares the exact trusted contract sources this environment was
    /// loaded from. Assumptions, not verified facts; the compiler's loader
    /// rechecks them against the declared text.
    pub fn set_source_assumptions(&mut self, sources: BTreeMap<String, String>) {
        self.source_assumptions = sources;
    }

    #[must_use]
    pub fn contract(&self, name: &str) -> Option<&ResourceFunctionContract> {
        self.functions.get(name)
    }

    #[must_use]
    pub fn functions(&self) -> &BTreeMap<String, ResourceFunctionContract> {
        &self.functions
    }
}
