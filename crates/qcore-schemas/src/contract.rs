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
    /// Card W43-LIBC-RESULT-ALIAS: the result is **derived from** the pointer
    /// passed at `parameter` and owns nothing.
    ///
    /// It never acquires, discharges or transfers custody of the base.
    /// Releasing the result is not a release of the base, and releasing the
    /// base invalidates the result. `nullable` states whether the call may
    /// return NULL in place of a derived pointer; `const_dropped` records that
    /// the base parameter is declared `ptr<const, u8>` while the result is
    /// `ptr<mut, u8>` (`strchr`'s `const char *` in, `char *` out). The loader
    /// admits this mode only on a `Borrowed` pointer parameter and a pointer
    /// result, both of depth one.
    Derived {
        parameter: usize,
        extent: DerivedExtent,
        nullable: bool,
        const_dropped: bool,
    },
}

/// Which part of the base object a [`ResourceReturnMode::Derived`] result may
/// designate.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DerivedExtent {
    /// The result is the base pointer's own value (`memcpy` returns `dst`).
    Exact,
    /// The result points somewhere into the base object (`strchr`, `memchr`).
    Interior,
}

/// Card W43-DESCRIPTOR-RESOURCE-TRACKING: **which** obligation a custody
/// position creates or touches.
///
/// Before this card every tracked resource had one kind, so `free(fopen(…))`
/// and `fclose(malloc(…))` would both have discharged. A release now discharges
/// only an obligation of its own kind; a release of another kind is a
/// violation *candidate* and discharges nothing.
///
/// [`ResourceKind::Heap`] is the default, and it is what every declaration that
/// names no kind means, so every contract written before this card keeps its
/// meaning exactly. It is also the kind of every allocation-family wrapper the
/// corpus already declares: those declarations mix allocators (`free`,
/// `cfl_sds_destroy`, `flb_free`) inside one heap kind, as they did before.
///
/// [`ResourceKind::Descriptor`] is the one kind carried by an **integer** (a C
/// `int` file descriptor, whose failure value is `-1`, not NULL); every other
/// kind is carried by a pointer. See [`ResourceKind::is_integer`].
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum ResourceKind {
    /// An allocation from the C heap or an allocator wrapper over it.
    #[default]
    Heap,
    /// A POSIX file descriptor (`open`, `socket`, `accept4`; `close`).
    Descriptor,
    /// A `FILE *` stream (`fopen`; `fclose`).
    Stream,
    /// A `DIR *` directory stream (`opendir`; `closedir`).
    Directory,
    /// A dynamic-library handle (`dlopen`; `dlclose`).
    DynamicLibrary,
}

impl ResourceKind {
    /// Every kind, in declaration order.
    pub const ALL: [Self; 5] = [
        Self::Heap,
        Self::Descriptor,
        Self::Stream,
        Self::Directory,
        Self::DynamicLibrary,
    ];

    #[must_use]
    pub const fn is_heap(&self) -> bool {
        matches!(self, Self::Heap)
    }

    /// Whether this kind's identity is carried by an integer rather than a
    /// pointer. Its failure value is then a **sentinel** (`-1`), not NULL, so a
    /// contract states its validity with `sentinel`/`valid` rather than
    /// `nullable`/`nonnull`, and a null test of it (`if (fd)`) refines nothing.
    #[must_use]
    pub const fn is_integer(self) -> bool {
        matches!(self, Self::Descriptor)
    }

    /// The contract-module spelling. `heap` is never written: it is spelled by
    /// the absence of a kind clause.
    #[must_use]
    pub const fn spelling(self) -> &'static str {
        match self {
            Self::Heap => "heap",
            Self::Descriptor => "descriptor",
            Self::Stream => "stream",
            Self::Directory => "directory",
            Self::DynamicLibrary => "dynamic_library",
        }
    }

    /// The kind a contract-module spelling names, if any.
    #[must_use]
    pub fn from_spelling(spelling: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|kind| kind.spelling() == spelling)
    }
}

#[derive(Clone, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ResourceFunctionContract {
    pub parameters: Vec<ResourceParameterMode>,
    pub result: ResourceReturnMode,
    /// Card W43-DESCRIPTOR-RESOURCE-TRACKING: the **non-heap** kind of each
    /// custody-bearing parameter, by index. A parameter absent here is heap.
    /// Omitted from the serialized form when empty, so every contract without
    /// a kind serializes byte for byte as it did before the card. Never holds
    /// [`ResourceKind::Heap`]: [`Self::with_parameter_kind`] normalizes it away,
    /// so two contracts that mean the same thing compare equal.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub parameter_kinds: BTreeMap<usize, ResourceKind>,
    /// Card W43-DESCRIPTOR-RESOURCE-TRACKING: the kind of an owned or
    /// reallocated result. Heap by default, and omitted when heap.
    #[serde(default, skip_serializing_if = "ResourceKind::is_heap")]
    pub result_kind: ResourceKind,
}

/// Written by hand so that a contract with no kind prints exactly as the
/// derived implementation did before card W43-DESCRIPTOR-RESOURCE-TRACKING:
/// the corpus audit renders a `SummaryConflict` through this text, and a
/// vocabulary addition must not move a verdict-bearing byte.
impl std::fmt::Debug for ResourceFunctionContract {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut debug = f.debug_struct("ResourceFunctionContract");
        debug
            .field("parameters", &self.parameters)
            .field("result", &self.result);
        if !self.parameter_kinds.is_empty() {
            debug.field("parameter_kinds", &self.parameter_kinds);
        }
        if !self.result_kind.is_heap() {
            debug.field("result_kind", &self.result_kind);
        }
        debug.finish()
    }
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
            parameter_kinds: BTreeMap::new(),
            result_kind: ResourceKind::Heap,
        }
    }

    /// The same contract with parameter `index` of kind `kind`.
    #[must_use]
    pub fn with_parameter_kind(mut self, index: usize, kind: ResourceKind) -> Self {
        if kind.is_heap() {
            self.parameter_kinds.remove(&index);
        } else {
            self.parameter_kinds.insert(index, kind);
        }
        self
    }

    /// The same contract with an owned or reallocated result of kind `kind`.
    #[must_use]
    pub const fn with_result_kind(mut self, kind: ResourceKind) -> Self {
        self.result_kind = kind;
        self
    }

    /// The kind of parameter `index`: heap unless declared otherwise.
    #[must_use]
    pub fn parameter_kind(&self, index: usize) -> ResourceKind {
        self.parameter_kinds
            .get(&index)
            .copied()
            .unwrap_or(ResourceKind::Heap)
    }

    /// Whether any position of this contract names a non-heap kind.
    #[must_use]
    pub fn has_non_heap_kind(&self) -> bool {
        !self.parameter_kinds.is_empty() || !self.result_kind.is_heap()
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

/// The identity of one contract module on a published load path (card
/// **CONTRACT-INHERIT**): which crate, at which version, published which
/// module, with which exact bytes.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ContractModuleIdentity {
    /// The owning crate's package name.
    pub crate_name: String,
    /// The owning crate's version.
    pub crate_version: String,
    /// The module's logical path under the crate's `contracts/` tree.
    pub module: String,
    /// SHA-256 of the module's source text as written.
    pub sha256: String,
}

/// One module an inheriting module reached through `#include`, retained with
/// its exact text so the elaboration can be replayed without a load path.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct IncludedContractModule {
    pub identity: ContractModuleIdentity,
    pub source: String,
}

/// One step of an inheritance chain: the parent declaration and the module
/// that declares it.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ContractInheritanceLink {
    pub declaration: String,
    pub module: ContractModuleIdentity,
}

/// The provenance of one selected module that uses contract inclusion or
/// inheritance (card **CONTRACT-INHERIT**).
///
/// The environment's source assumption for that module is its **elaborated**
/// text: the text a person would have written by hand. This record says where
/// each inherited contract came from, and retains every text needed to
/// reproduce the elaboration, so a certificate checker can replay it and a
/// reviewer can read the chain. An inherited contract is a trusted assumption
/// of the same strength as a hand-written one; this record is its provenance,
/// not evidence that it is true.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContractInclusionEvidence {
    /// The selected module itself.
    pub module: ContractModuleIdentity,
    /// The selected module's text as written, directives included.
    pub source: String,
    /// Every module reached through `#include`, transitively, keyed by its
    /// `crate:module` target spelling.
    pub included: BTreeMap<String, IncludedContractModule>,
    /// For each inheriting declaration, its chain from the immediate parent to
    /// the declaration whose contract is written out.
    pub inherited: BTreeMap<String, Vec<ContractInheritanceLink>>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResourceContracts {
    functions: BTreeMap<String, ResourceFunctionContract>,
    // Exact trusted inputs travel with the environment into safety certificates.
    source_assumptions: BTreeMap<String, String>,
    /// Card CONTRACT-INHERIT: provenance of every selected module that used
    /// contract inclusion, keyed like `source_assumptions`. Omitted from the
    /// serialized form when empty, so every environment without inclusion
    /// serializes byte-for-byte as it did before the card.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    inclusion_evidence: BTreeMap<String, ContractInclusionEvidence>,
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
            inclusion_evidence: BTreeMap::new(),
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

    /// Provenance of every selected module that used contract inclusion
    /// (card CONTRACT-INHERIT). Empty for every environment loaded without it.
    #[must_use]
    pub fn inclusion_evidence(&self) -> &BTreeMap<String, ContractInclusionEvidence> {
        &self.inclusion_evidence
    }

    /// Declares the inclusion provenance of this environment's elaborated
    /// source assumptions. The contract loader rechecks it by replaying the
    /// elaboration from the retained texts.
    pub fn set_inclusion_evidence(
        &mut self,
        evidence: BTreeMap<String, ContractInclusionEvidence>,
    ) {
        self.inclusion_evidence = evidence;
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
