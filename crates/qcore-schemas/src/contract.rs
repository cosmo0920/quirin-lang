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
}

impl ResourceKind {
    /// Every kind, in declaration order.
    pub const ALL: [Self; 4] = [Self::Heap, Self::Descriptor, Self::Stream, Self::Directory];

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
    /// Card W45-FB-RETRY-LIFETIME: what the call does to an intrusive
    /// container's membership of the object a `resource_borrowed` parameter
    /// designates (contract specification §C04.8). Omitted when empty, so every
    /// contract without one serializes byte for byte as before.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub container_effects: BTreeMap<usize, ContainerEffect>,
    /// Card W45-FB-RETRY-LIFETIME: the counted-user clauses (contract
    /// specification §C04.7). Omitted when empty.
    #[serde(default, skip_serializing_if = "CountedUserClauses::is_empty")]
    pub counted_users: CountedUserClauses,
}

/// Card **W45-FB-RETRY-LIFETIME**: an intrusive-container effect on the object
/// an argument designates (contract specification §C04.8).
///
/// The parameter keeps its `resource_borrowed` custody mode for every rule that
/// reads modes (the ABI projection, the admission of a member address). The
/// effect is what moves custody: a consumer that does not apply it MUST refuse
/// the call rather than read it as a borrow.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "effect")]
pub enum ContainerEffect {
    /// `resource_published(entry, container)`: the object whose member (or
    /// which) `entry` designates is linked into the container reached through
    /// `container_parameter`, and the container holds it from then on.
    Published { container_parameter: usize },
    /// `resource_unpublished(entry)`: the object is unlinked from the container
    /// that holds it, and custody returns to the caller.
    Unpublished,
}

/// Card **W45-FB-RETRY-LIFETIME**: one counted-user operation on the object a
/// `resource_borrowed` pointer parameter designates (§C04.7).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UserEffect {
    /// `resource_user_added(p)`: one user is added; the caller owes its drop.
    Added,
    /// `resource_user_dropped(p)`: one user is dropped and no release check
    /// runs, so the object may be left unreferenced until one does.
    Dropped,
    /// `resource_user_dropped_checked(p)`: one user is dropped and the release
    /// check runs; the object may be destroyed.
    DroppedChecked,
    /// `resource_user_release_check(p)`: the release check alone; the object
    /// may be destroyed when it has no user left.
    ReleaseCheck,
}

/// Card **W45-FB-RETRY-LIFETIME**: one declared counter operation: its effect
/// and the member that holds the count (`resource_user_added(task, users)`
/// counts in `task->users`).
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct UserOperation {
    pub effect: UserEffect,
    pub counter: String,
}

/// Card **W45-FB-RETRY-LIFETIME**: `resource_user_exit(p, RETURNED, DELTA)`.
///
/// On every normal return whose value is the integer `returned`, the call has
/// added `delta` users (net: `-1`, `0` or `1`) to the object parameter
/// `parameter` designates. A body defined in the analysed unit is checked
/// against it; a caller applies it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct UserExitClause {
    pub parameter: usize,
    pub returned: i64,
    pub delta: i8,
}

/// Card **W45-FB-RETRY-LIFETIME**: every counted-user clause of one declaration.
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct CountedUserClauses {
    /// The operation performed on each parameter, by index.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub effects: BTreeMap<usize, UserOperation>,
    /// `resource_user_neutral()`: the call adds, drops and checks no user of
    /// any object.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub neutral: bool,
    /// The declared exit clauses, in declaration order.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub exits: Vec<UserExitClause>,
}

impl CountedUserClauses {
    /// Whether the declaration states nothing about counted users.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.effects.is_empty() && !self.neutral && self.exits.is_empty()
    }
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
        if !self.container_effects.is_empty() {
            debug.field("container_effects", &self.container_effects);
        }
        if !self.counted_users.is_empty() {
            debug.field("counted_users", &self.counted_users);
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
            container_effects: BTreeMap::new(),
            counted_users: CountedUserClauses::default(),
        }
    }

    /// Card W45-FB-RETRY-LIFETIME: whether the call has an intrusive-container
    /// effect. Every route that admits a call as the identity on custody
    /// because its modes are all `Borrowed` MUST refuse such a call instead.
    #[must_use]
    pub fn has_container_effect(&self) -> bool {
        !self.container_effects.is_empty()
    }

    /// Card W45-FB-RETRY-LIFETIME: the same contract without its container
    /// and counted-user clauses — the part a derived custody summary can
    /// state, and so the part it is compared with.
    #[must_use]
    pub fn custody_projection(&self) -> Self {
        let mut projection = self.clone();
        projection.container_effects.clear();
        projection.counted_users = CountedUserClauses::default();
        projection
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
    /// Card W45-CONTRACT-DEFERRED-REVIEW: a declared contract whose text marks
    /// it, or one of its clauses, as **awaiting human review**
    /// (`resource_review_pending`, contract spec §C06).
    ///
    /// It is loaded and validated total-or-reject exactly like a declared
    /// contract, but it is neither an assumption nor evidence: it justifies
    /// neither `Safe` nor `Unsafe`, and no validation can promote it. It
    /// differs from [`Self::Proposed`] on both counts — a proposal awaits the
    /// *core's* validation and is only barred from justifying `Safe`. A person
    /// resolves it by editing the text, which changes its digest.
    PendingHumanReview,
}

/// Which part of a declaration a review marker defers (card
/// W45-CONTRACT-DEFERRED-REVIEW).
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "scope")]
pub enum ReviewScope {
    /// `requires resource_review_pending("REASON");` — the whole contract.
    Contract,
    /// `ensures normal resource_review_pending(CLAUSE, "REASON");` — one
    /// clause, recorded as its canonical text (`ensures normal CLAUSE`).
    Clause { clause: String },
}

/// One review marker as written: what it defers, why, and who owns it.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ReviewDeferral {
    #[serde(flatten)]
    pub scope: ReviewScope,
    /// The mandatory free-text reason, exactly as the string literal spells it.
    pub reason: String,
    /// The optional reviewer or owner, exactly as its string literal spells it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub owner: Option<String>,
}

/// A declared contract that is withheld because it awaits human review (card
/// W45-CONTRACT-DEFERRED-REVIEW, contract spec §C06).
///
/// The loader validates the declaration in full — every mode, kind and ABI
/// rule — and records the contract it *would* apply, so the text is checked,
/// digested and visible. It is deliberately **not** an entry of
/// [`ResourceContracts::functions`], so a consumer that knows nothing of
/// review markers sees an uncontracted callee: the fail-closed default.
/// Consumers that know of it attribute the refusal to the pending review.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PendingHumanReview {
    /// Always [`ContractTrust::PendingHumanReview`]; carried so that the
    /// serialized record states its own status.
    pub trust: ContractTrust,
    /// The retained source-assumption key of the module that declares it.
    pub module: String,
    /// The contract the declaration states, withheld from every consumer.
    pub contract: ResourceFunctionContract,
    /// Every marker the declaration carries, in clause order: either one
    /// contract-level marker or one or more clause-level markers.
    pub deferrals: Vec<ReviewDeferral>,
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
    /// Card W45-CONTRACT-DEFERRED-REVIEW: declarations awaiting human review,
    /// keyed by function name and disjoint from `functions`. Omitted when
    /// empty, so every environment without a marker serializes byte for byte
    /// as it did before the card.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pending_review: BTreeMap<String, PendingHumanReview>,
    /// Card W45-CONTRACT-REVISION-MODE: the revision markers of every
    /// declaration that carries one, keyed by function name. A declaration
    /// absent here is pinned by default. Omitted when empty.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    revision: BTreeMap<String, crate::contract_revision::DeclaredRevision>,
    /// Card W45-CONTRACT-REVISION-MODE: pinned declarations withheld because
    /// the audited source does not match their lock, keyed by function name
    /// and disjoint from `functions`. Omitted when empty.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pin_mismatch: BTreeMap<String, crate::contract_revision::PinMismatch>,
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
            pending_review: BTreeMap::new(),
            revision: BTreeMap::new(),
            pin_mismatch: BTreeMap::new(),
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

    /// Declarations awaiting human review (card W45-CONTRACT-DEFERRED-REVIEW),
    /// keyed by function name. None of them is in [`Self::functions`].
    #[must_use]
    pub fn pending_review(&self) -> &BTreeMap<String, PendingHumanReview> {
        &self.pending_review
    }

    /// Declares the environment's pending reviews. The contract loader
    /// rechecks them against the retained source text, exactly as it rechecks
    /// the function map.
    pub fn set_pending_review(&mut self, pending: BTreeMap<String, PendingHumanReview>) {
        self.pending_review = pending;
    }

    /// Card W45-CONTRACT-REVISION-MODE: the revision markers of every
    /// declaration that carries one. Every other declaration is pinned.
    #[must_use]
    pub fn declared_revisions(
        &self,
    ) -> &BTreeMap<String, crate::contract_revision::DeclaredRevision> {
        &self.revision
    }

    /// Declares the environment's revision markers. The contract loader
    /// rechecks them against the retained source text.
    pub fn set_declared_revisions(
        &mut self,
        revision: BTreeMap<String, crate::contract_revision::DeclaredRevision>,
    ) {
        self.revision = revision;
    }

    /// The effective revision mode of a declaration: pinned unless its
    /// markers make it flexible.
    #[must_use]
    pub fn revision_mode(&self, name: &str) -> crate::contract_revision::RevisionMode {
        self.revision
            .get(name)
            .map_or(crate::contract_revision::RevisionMode::Pinned, |declared| {
                declared.mode
            })
    }

    /// Card W45-CONTRACT-REVISION-MODE: pinned declarations withheld because
    /// the audited source does not match their lock. None of them is in
    /// [`Self::functions`].
    #[must_use]
    pub fn pin_mismatch(&self) -> &BTreeMap<String, crate::contract_revision::PinMismatch> {
        &self.pin_mismatch
    }

    /// Withhold one pinned declaration of the function map because its pin
    /// does not match: the contract leaves `functions` and is recorded under
    /// [`Self::pin_mismatch`] with its module, reason and detail.
    ///
    /// # Errors
    ///
    /// Refuses a name that is not in the function map (already withheld,
    /// pending review, or never declared) and a declaration that is not
    /// pinned: a flexible contract is never withheld for its pin.
    pub fn withhold_for_pin_mismatch(
        &mut self,
        name: &str,
        module: String,
        reason: crate::contract_revision::PinMismatchReason,
        lock: Option<String>,
        detail: String,
    ) -> Result<(), String> {
        if self.revision_mode(name) != crate::contract_revision::RevisionMode::Pinned {
            return Err(format!("`{name}` is flexible and has no pin to mismatch"));
        }
        let contract = self
            .functions
            .remove(name)
            .ok_or_else(|| format!("`{name}` is not a contract in force"))?;
        self.pin_mismatch.insert(
            name.to_owned(),
            crate::contract_revision::PinMismatch {
                module,
                contract,
                reason,
                lock,
                detail,
            },
        );
        Ok(())
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
