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

//! Resource-contract data model shared by the legacy-C analyzer, the
//! contract loader, and (future) providers.
//!
//! Moved verbatim from `qcore-compiler`'s `legacy_c` module (W0 of
//! `docs/GENERALIZATION_PLAN.md`). Types only: the analyzer and the
//! source-assumption recheck stay in the compiler crate.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::contract_lookup_record::{LookupRecorder, RecorderSlot};

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
    /// The exact item restored from its checked owner is returned. This is
    /// the same identity, never a fresh allocation or an interior borrow.
    /// Requires a matching `Removed` parameter and a non-null owned parent.
    Removed {
        parameter: usize,
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
    /// Card **SLOT-MOVE-AND-INTERIOR-BORROW** (contract spec §C04.3.4, gateway
    /// rule 12y; USER DECISION 2026-10-05, `W48-CALLEE-DECISION-TABLE` class
    /// 8): the result is **borrowed from the parent** the base designates — an
    /// interior pointer, a child object the parent owns, or a child the call
    /// creates and links into the parent. It owns nothing; it is valid only
    /// while the parent lives and is not mutated through another path, so a
    /// consumer ends it at a parent release or at any call that may mutate the
    /// parent, and refuses every later use; a release of it is refused, never
    /// accused; a write through it is a write to the parent.
    Borrowed,
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
    /// Card **CG25-RESOURCE-KINDS**: a dynamic-library handle (`dlopen`;
    /// `dlclose`). Pointer-carried, failure value NULL. Not a heap pointer:
    /// `free` does not unload a library and `dlclose` does not free memory.
    DynamicLibrary,
    /// Card **CG25-RESOURCE-KINDS**: a compiled POSIX regular expression
    /// (`regcomp`; `regfree`). **In place**: the obligation lives in the
    /// caller's `regex_t` storage, which the call initialises, not in a
    /// returned pointer. Its failure value is a nonzero status from the
    /// initialising call. See [`ResourceKind::is_in_place`].
    Regex,
    /// Card **REFCOUNT-RESOURCE-KIND** (§C04.17, gateway rule 13b): one
    /// reference a function holds on a reference-counted object. Pointer-
    /// carried. Created only by `resource_reference_kept` and discharged only
    /// by `resource_reference_dropped`, never by a heap release; no
    /// `resource_kind` clause spells it, because its obligation is counted per
    /// object rather than owned per pointer.
    Reference,
}

/// Card **CG25-RESOURCE-KINDS**: what carries a kind's identity.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResourceCarrier {
    /// A pointer value; the failure value is NULL.
    Pointer,
    /// An `int`; the failure value is the sentinel `-1`.
    Integer,
    /// Caller storage the call initialises in place; the failure value is a
    /// nonzero status returned by the initialising call.
    Storage,
}

impl ResourceKind {
    /// Every kind, in declaration order.
    pub const ALL: [Self; 7] = [
        Self::Heap,
        Self::Descriptor,
        Self::Stream,
        Self::Directory,
        Self::DynamicLibrary,
        Self::Regex,
        Self::Reference,
    ];

    #[must_use]
    pub const fn is_heap(&self) -> bool {
        matches!(self, Self::Heap)
    }

    /// Card **CG25-RESOURCE-KINDS**: what carries this kind's identity.
    #[must_use]
    pub const fn carrier(self) -> ResourceCarrier {
        match self {
            Self::Descriptor => ResourceCarrier::Integer,
            Self::Regex => ResourceCarrier::Storage,
            Self::Heap
            | Self::Stream
            | Self::Directory
            | Self::DynamicLibrary
            | Self::Reference => ResourceCarrier::Pointer,
        }
    }

    /// Card **CG25-RESOURCE-KINDS**: whether this kind is initialised into
    /// caller storage. Its obligation is bound to the storage a `&NAME`
    /// actual designates, its initialisation can fail only by a nonzero
    /// status, and the storage is custody, never data, while it is tracked.
    #[must_use]
    pub const fn is_in_place(self) -> bool {
        matches!(self.carrier(), ResourceCarrier::Storage)
    }

    /// Whether this kind's identity is carried by an integer rather than a
    /// pointer. Its failure value is then a **sentinel** (`-1`), not NULL, so a
    /// contract states its validity with `sentinel`/`valid` rather than
    /// `nullable`/`nonnull`, and a null test of it (`if (fd)`) refines nothing.
    #[must_use]
    pub const fn is_integer(self) -> bool {
        matches!(self.carrier(), ResourceCarrier::Integer)
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
            Self::Regex => "regex",
            Self::Reference => "reference",
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
    /// Card **LERI-HELD-AND-LINK-MEMBER-PREMISES** (gateway rule 46): the link
    /// member an element destroyer's unlink removes, by the index of the
    /// parameter it unlinks and releases (`resource_unpublished(p, MEMBER)`,
    /// contract specification §C04.8): the call runs `mk_list_del(&p->MEMBER)`.
    /// Omitted when empty, so every contract without one serializes byte for
    /// byte as before.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub link_members: BTreeMap<usize, String>,
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
    /// Card **LIST-ENTRY-RELEASE-IDENTITY** (gateway rule 43): the call may
    /// unlink, and then release or hand on, objects linked in the container
    /// this parameter reaches; the container itself is borrowed and outlives
    /// the call. Only a derived summary states it (a body that took custody
    /// of an element of a container rooted at this formal); a consumer that
    /// does not apply it MUST refuse the call, and one that does forgets every
    /// object it had published, since the call may have released any of them.
    ElementsReleased,
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
        if !self.link_members.is_empty() {
            debug.field("link_members", &self.link_members);
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
            link_members: BTreeMap::new(),
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
    /// (with their link members) and counted-user clauses — the part a derived custody summary can
    /// state, and so the part it is compared with.
    #[must_use]
    pub fn custody_projection(&self) -> Self {
        let mut projection = self.clone();
        projection.container_effects.clear();
        projection.counted_users = CountedUserClauses::default();
        projection.link_members.clear();
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

/// What resolving a review marker takes (card
/// W47-REVIEW-HOLD-CONCRETE-REASON, contract spec §C06.1): read from the
/// marker's structure, never from its reason's prose.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ReviewDisposition {
    /// A plain marker: the resolution it offers is a person's review and then
    /// unwrapping the clause (or contract) as written.
    #[default]
    AcceptableAsWrittenAfterReview,
    /// `…, rewrite_before_acceptance)`: the marker's author states that the
    /// text as written is not what the body does, so unwrapping it is not a
    /// resolution; the text must be rewritten first.
    MustBeRewrittenBeforeAcceptance,
}

impl ReviewDisposition {
    /// The published category spelling.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::AcceptableAsWrittenAfterReview => "acceptable-as-written-after-review",
            Self::MustBeRewrittenBeforeAcceptance => "must-be-rewritten-before-acceptance",
        }
    }

    /// `true` for the default (plain) marker; its serialized form omits the
    /// field, so every environment written before the card is byte-identical.
    #[must_use]
    pub fn is_plain(&self) -> bool {
        *self == Self::AcceptableAsWrittenAfterReview
    }
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
    /// Card W47-REVIEW-HOLD-CONCRETE-REASON: whether the marker's author
    /// states that the text must be rewritten before it can be accepted.
    #[serde(default, skip_serializing_if = "ReviewDisposition::is_plain")]
    pub disposition: ReviewDisposition,
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
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub lifetime_holder_frames: Vec<crate::loop_lifetime::HolderFrame>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub lifetime_effects: Vec<crate::loop_lifetime::ForwardedLifetime>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub lifetime_removals: Vec<crate::retained_by::Removal>,
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
            lifetime_effects: Vec::new(),
            lifetime_removals: Vec::new(),
            lifetime_holder_frames: Vec::new(),
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

/// Independently expected home identity for one conditional cross-unit import.
/// The audit derives this from its pinned configured home, and the caller's
/// trust-context hash binds it separately from the certificate's home witness.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CrossUnitImportIdentity {
    pub raw_source_sha256: String,
    pub preprocessed_source_sha256: String,
    pub scope_evidence_sha256: String,
    pub analysis_request_sha256: String,
    pub environment_sha256: String,
}

#[derive(Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResourceContracts {
    /// Card MEASURE-LOOKUP-RECORD-REUSE: the lookup recorder attached to this
    /// instance and its clones, if any ([`Self::with_lookup_recorder`]).
    /// Never serialized or deserialized, equal to every other slot, absent
    /// from `Debug`; first, so an equality comparison records its read.
    #[serde(skip_deserializing, skip_serializing_if = "RecorderSlot::serialized")]
    lookups: RecorderSlot,
    functions: BTreeMap<String, ResourceFunctionContract>,
    /// Validated summaries imported from another translation unit.  These are
    /// never declared contracts: a gateway certificate must carry and replay
    /// the home body and the explicit link-resolution premise for each entry.
    /// An empty map preserves the historical serialized environment.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    inferred_imports: BTreeMap<String, ResourceFunctionContract>,
    /// Expected identities of those imports. Included in the caller's audit
    /// trust hash, so a changed home cannot replay by changing its own digest.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    cross_unit_import_identities: BTreeMap<String, CrossUnitImportIdentity>,
    /// SHA-256 of the caller-supplied linked-program resolution table. This
    /// is a named assumption, bound into audit trust-context identity.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    link_resolution_sha256: Option<String>,
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
    /// Card W45-REFUTED-CONTRACT-WITHDRAWAL: declarations some analysed body
    /// refutes, found by an audit's first phase (contract spec §C06.4). Unlike
    /// a pin mismatch the declaration stays in `functions`, so its own body is
    /// still checked against it; every call site outside that body is held.
    /// Omitted when empty, so every environment without a refutation
    /// serializes byte for byte as it did before the card.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    refuted_by_body: crate::contract_refutation::RefutationTable,
    /// Card CG23-RELATIONAL-POSTCONDITIONS: return-code dispositions, keyed
    /// by function name and disjoint from `functions`, so a consumer that
    /// knows nothing of them sees an uncontracted callee. Omitted when empty,
    /// so every environment without one serializes byte for byte as before.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    relational: BTreeMap<String, crate::relational_post::RelationalContract>,
    /// Card CALLEE-RETENTION-CLASS1: declarations with retention clauses
    /// (§C04.14), keyed by function name and disjoint from `functions`, so a
    /// consumer that knows nothing of retention sees an uncontracted callee and
    /// never reads a retained pointer as borrowed. Omitted when empty.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    retaining: BTreeMap<String, crate::retained_by::RetainingContract>,
    /// S3-only declarations. Unaware raw retaining-table consumers cannot
    /// project away conditional prerequisites through another effect route.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    guarded_retaining: BTreeMap<String, crate::retained_by::RetainingContract>,
    /// Declaration identity survives withdrawal; it grants no prerequisite
    /// or effect, and prevents an ordinary summary from erasing its interface.
    #[serde(default, skip_serializing_if = "BTreeSet::is_empty")]
    declared_guarded: BTreeSet<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    inferred_lifetimes: BTreeMap<String, crate::loop_lifetime::InferredLifetimeSummary>,
    /// Cross-unit effects remain separate from ordinary borrowed imports.
    /// Gateway admission replays the exact home before consuming these records.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    lifetime_imports: BTreeMap<String, crate::loop_lifetime::InferredLifetimeSummary>,
    /// Card CELL-PROTOCOL-RELEASE-WINDOW-FRAMES (gateway rule 38g, C06.1):
    /// set only by [`Self::with_human_holds_in_force`], the environment a
    /// human act would produce by accepting every declared callee frame as
    /// written. Omitted when false, so every environment serializes as before.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    cell_frames_accepted: bool,
    /// Card CONTRACT-SUPERSESSION (contract spec §C06.6): per function, the
    /// declarations a newer-stamped declaration superseded, retained with
    /// their texts. The declaration in force is the function's ordinary entry.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    superseded: BTreeMap<String, Vec<crate::contract_supersession::SupersededDeclaration>>,
    /// Card CONTRACT-SUPERSESSION: functions declared by more than one module
    /// whose stamps do not decide between them. None of them is in force in
    /// any table; every call relying on one is held (`contract-conflict`).
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    contract_conflict: BTreeMap<String, crate::contract_supersession::ContractConflict>,
    /// Card CELL-GUARD-RESULT-RANGE-CHECKED (contract spec C09.4, gateway rule
    /// 38e): the pin-derived integer result sets of the bounds environment an
    /// audit route installed, with their standing; rule 38 reads one only
    /// while it is in force. Omitted when empty, so every environment without
    /// one serializes byte for byte as before.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    result_ranges: BTreeMap<String, crate::result_range::ResultRangeFact>,
    /// Card MEMBER-EFFECT-SUMMARY (gateway rule 38j): the validated contracts
    /// of the bodies that release members through a formal they keep, each
    /// with its member effect. Never in [`Self::functions`]: only the
    /// direct-call route that applies the effect reads one. Omitted when
    /// empty, so every environment without one serializes as before.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    member_effects: BTreeMap<String, crate::member_effect::MemberEffectSummary>,
}

/// Card MEASURE-LOOKUP-RECORD-REUSE: the derived `Debug` output, field for
/// field, without the recorder slot; formatting reads the whole environment.
impl std::fmt::Debug for ResourceContracts {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.lookups.whole("debug");
        formatter
            .debug_struct("ResourceContracts")
            .field("functions", &self.functions)
            .field("inferred_imports", &self.inferred_imports)
            .field(
                "cross_unit_import_identities",
                &self.cross_unit_import_identities,
            )
            .field("link_resolution_sha256", &self.link_resolution_sha256)
            .field("source_assumptions", &self.source_assumptions)
            .field("inclusion_evidence", &self.inclusion_evidence)
            .field("pending_review", &self.pending_review)
            .field("revision", &self.revision)
            .field("pin_mismatch", &self.pin_mismatch)
            .field("refuted_by_body", &self.refuted_by_body)
            .field("relational", &self.relational)
            .field("retaining", &self.retaining)
            .field("guarded_retaining", &self.guarded_retaining)
            .field("declared_guarded", &self.declared_guarded)
            .field("inferred_lifetimes", &self.inferred_lifetimes)
            .field("lifetime_imports", &self.lifetime_imports)
            .field("cell_frames_accepted", &self.cell_frames_accepted)
            .field("superseded", &self.superseded)
            .field("contract_conflict", &self.contract_conflict)
            .field("result_ranges", &self.result_ranges)
            .field("member_effects", &self.member_effects)
            .finish()
    }
}

impl ResourceContracts {
    /// Card MEASURE-LOOKUP-RECORD-REUSE: a copy of this environment with a
    /// fresh lookup recorder, which records every read of the copy and of
    /// its clones; the copy answers every read exactly as this one does.
    #[must_use]
    pub fn with_lookup_recorder(&self) -> (Self, Arc<LookupRecorder>) {
        let recorder = LookupRecorder::new();
        let mut copy = self.clone();
        copy.lookups = RecorderSlot::holding(recorder.clone());
        (copy, recorder)
    }

    /// Card MEASURE-LOOKUP-RECORD-REUSE: SHA-256 of this environment
    /// restricted to `names`: for each name, its entry in every name-keyed
    /// table (or `null`), and the environment's scalars (the link-resolution
    /// digest and the callee-frame acceptance). Contract texts
    /// (`source_assumptions`) and inclusion provenance are keyed by module and
    /// reachable only through whole-map accessors, so a consumer that read
    /// them recorded a whole-map read; the entries here are the elaborated
    /// ones, so an included or inherited parent's change moves the entry of
    /// every name that inherits it. Not a read: nothing is recorded.
    ///
    /// # Errors
    ///
    /// An entry that does not serialize.
    pub fn restricted_sha256(&self, names: &BTreeSet<String>) -> Result<String, String> {
        fn entry<V: Serialize>(
            map: &BTreeMap<String, V>,
            name: &str,
        ) -> Result<serde_json::Value, String> {
            map.get(name)
                .map(serde_json::to_value)
                .transpose()
                .map(|value| value.unwrap_or(serde_json::Value::Null))
                .map_err(|error| error.to_string())
        }
        let mut entries = serde_json::Map::new();
        for name in names {
            let tables = serde_json::json!({
                "functions": entry(&self.functions, name)?,
                "inferred_imports": entry(&self.inferred_imports, name)?,
                "cross_unit_import_identities": entry(&self.cross_unit_import_identities, name)?,
                "pending_review": entry(&self.pending_review, name)?,
                "revision": entry(&self.revision, name)?,
                "pin_mismatch": entry(&self.pin_mismatch, name)?,
                "refuted_by_body": entry(&self.refuted_by_body, name)?,
                "relational": entry(&self.relational, name)?,
                "retaining": entry(&self.retaining, name)?,
                "guarded_retaining": entry(&self.guarded_retaining, name)?,
                "declared_guarded": self.declared_guarded.contains(name),
                "inferred_lifetimes": entry(&self.inferred_lifetimes, name)?,
                "lifetime_imports": entry(&self.lifetime_imports, name)?,
                "superseded": entry(&self.superseded, name)?,
                "contract_conflict": entry(&self.contract_conflict, name)?,
                "result_ranges": entry(&self.result_ranges, name)?,
                "member_effects": entry(&self.member_effects, name)?,
            });
            entries.insert(name.clone(), tables);
        }
        let value = serde_json::json!({
            "schema": "quirin-restricted-environment/1",
            "scalars": {
                "link_resolution_sha256": self.link_resolution_sha256,
                "cell_frames_accepted": self.cell_frames_accepted,
            },
            "names": entries,
        });
        serde_json::to_vec(&value)
            .map(|bytes| crate::sha256_hex(&bytes))
            .map_err(|error| error.to_string())
    }

    /// Card MEASURE-LOOKUP-RECORD-REUSE: whether any name-keyed table holds
    /// `name` (the check of a recorded `found`). Not a read.
    #[must_use]
    pub fn holds_name(&self, name: &str) -> bool {
        self.functions.contains_key(name)
            || self.inferred_imports.contains_key(name)
            || self.cross_unit_import_identities.contains_key(name)
            || self.pending_review.contains_key(name)
            || self.revision.contains_key(name)
            || self.pin_mismatch.contains_key(name)
            || self.refuted_by_body.contains_key(name)
            || self.relational.contains_key(name)
            || self.retaining.contains_key(name)
            || self.guarded_retaining.contains_key(name)
            || self.declared_guarded.contains(name)
            || self.inferred_lifetimes.contains_key(name)
            || self.lifetime_imports.contains_key(name)
            || self.superseded.contains_key(name)
            || self.contract_conflict.contains_key(name)
            || self.result_ranges.contains_key(name)
            || self.member_effects.contains_key(name)
    }

    #[must_use]
    pub fn new(contracts: impl IntoIterator<Item = (impl Into<String>, ResourceApiKind)>) -> Self {
        Self {
            lookups: RecorderSlot::default(),
            functions: contracts
                .into_iter()
                .map(|(name, kind)| (name.into(), ResourceFunctionContract::from(kind)))
                .collect(),
            inferred_imports: BTreeMap::new(),
            cross_unit_import_identities: BTreeMap::new(),
            link_resolution_sha256: None,
            source_assumptions: BTreeMap::new(),
            inclusion_evidence: BTreeMap::new(),
            pending_review: BTreeMap::new(),
            revision: BTreeMap::new(),
            pin_mismatch: BTreeMap::new(),
            refuted_by_body: BTreeMap::new(),
            relational: BTreeMap::new(),
            retaining: BTreeMap::new(),
            guarded_retaining: BTreeMap::new(),
            declared_guarded: BTreeSet::new(),
            inferred_lifetimes: BTreeMap::new(),
            lifetime_imports: BTreeMap::new(),
            cell_frames_accepted: false,
            superseded: BTreeMap::new(),
            contract_conflict: BTreeMap::new(),
            result_ranges: BTreeMap::new(),
            member_effects: BTreeMap::new(),
        }
    }

    /// Card MEMBER-EFFECT-SUMMARY (rule 38j): `name`'s validated contract
    /// with its member effect, if it has one.
    #[must_use]
    pub fn member_effect(&self, name: &str) -> Option<&crate::member_effect::MemberEffectSummary> {
        let found = self.member_effects.get(name);
        self.lookups.name(name, found.is_some());
        found
    }

    /// Card MEMBER-EFFECT-SUMMARY: every member-effect summary in force.
    #[must_use]
    pub fn member_effects(&self) -> &BTreeMap<String, crate::member_effect::MemberEffectSummary> {
        self.lookups.whole("member_effects");
        &self.member_effects
    }

    /// Card MEMBER-EFFECT-SUMMARY: install `name`'s member-effect summary. A
    /// name the ordinary table or a guarded interface already holds keeps
    /// that entry and gets none: a declaration is never replaced by an
    /// inference.
    pub fn insert_member_effect(
        &mut self,
        name: impl Into<String>,
        summary: crate::member_effect::MemberEffectSummary,
    ) {
        let name = name.into();
        self.lookups.name(&name, self.functions.contains_key(&name));
        if self.functions.contains_key(&name) || self.has_guarded_interface(&name) {
            return;
        }
        self.member_effects.insert(name, summary);
    }

    /// Card CELL-GUARD-RESULT-RANGE-CHECKED (rule 38e): the pin-derived result
    /// sets the route installed, with their standing.
    #[must_use]
    pub fn result_ranges(&self) -> &BTreeMap<String, crate::result_range::ResultRangeFact> {
        self.lookups.whole("result_ranges");
        &self.result_ranges
    }

    /// Card CELL-GUARD-RESULT-RANGE-CHECKED: `name`'s result set while it is
    /// in force; `None` for one held, withdrawn, unchecked or absent.
    #[must_use]
    pub fn result_range_in_force(
        &self,
        name: &str,
    ) -> Option<&crate::result_range::ResultRangeFact> {
        let found = self.result_ranges.get(name);
        self.lookups.name(name, found.is_some());
        found.filter(|fact| fact.standing == crate::result_range::ResultRangeStanding::InForce)
    }

    /// Card CELL-GUARD-RESULT-RANGE-CHECKED: install the route's result sets.
    pub fn set_result_ranges(
        &mut self,
        ranges: BTreeMap<String, crate::result_range::ResultRangeFact>,
    ) {
        self.result_ranges = ranges;
    }

    /// Card CONTRACT-SUPERSESSION (contract spec §C06.6): the declarations
    /// retained as superseded, per function.
    #[must_use]
    pub fn superseded(
        &self,
    ) -> &BTreeMap<String, Vec<crate::contract_supersession::SupersededDeclaration>> {
        self.lookups.whole("superseded");
        &self.superseded
    }

    /// Card CONTRACT-SUPERSESSION: the functions withheld because their
    /// declarations' stamps do not decide between them.
    #[must_use]
    pub fn contract_conflict(
        &self,
    ) -> &BTreeMap<String, crate::contract_supersession::ContractConflict> {
        self.lookups.whole("contract_conflict");
        &self.contract_conflict
    }

    /// Declares the environment's supersession record. The contract loader
    /// sets it from the source transform and rechecks it (a withheld
    /// function is in no table).
    pub fn set_supersession(
        &mut self,
        superseded: BTreeMap<String, Vec<crate::contract_supersession::SupersededDeclaration>>,
        conflicts: BTreeMap<String, crate::contract_supersession::ContractConflict>,
    ) {
        self.superseded = superseded;
        self.contract_conflict = conflicts;
    }

    pub fn insert(&mut self, name: impl Into<String>, kind: ResourceApiKind) {
        self.insert_contract(name, ResourceFunctionContract::from(kind));
    }

    pub fn insert_contract(&mut self, name: impl Into<String>, contract: ResourceFunctionContract) {
        let name = name.into();
        self.lookups.name(&name, self.functions.contains_key(&name));
        // An ordinary overlay has no representation of guarded entry
        // obligations. Withdrawal must be explicit before replacing one.
        if !self.has_guarded_interface(&name) {
            self.functions.insert(name, contract);
        }
    }

    /// Install the contracts of independently replayable cross-unit imports.
    /// The gateway certificate checker refuses a nonempty import map without
    /// matching home evidence; this setter conveys no authority by itself.
    pub fn set_lifetime_imports(
        &mut self,
        imports: BTreeMap<String, crate::loop_lifetime::InferredLifetimeSummary>,
    ) {
        self.lifetime_imports = imports;
    }
    #[must_use]
    pub fn lifetime_imports(
        &self,
    ) -> &BTreeMap<String, crate::loop_lifetime::InferredLifetimeSummary> {
        self.lookups.whole("lifetime_imports");
        &self.lifetime_imports
    }
    #[must_use]
    pub fn lifetime_summary(
        &self,
        name: &str,
    ) -> Option<&crate::loop_lifetime::InferredLifetimeSummary> {
        let found = self
            .inferred_lifetimes
            .get(name)
            .or_else(|| self.lifetime_imports.get(name));
        self.lookups.name(name, found.is_some());
        found
    }
    pub fn lifetime_summaries(
        &self,
    ) -> impl Iterator<Item = (&String, &crate::loop_lifetime::InferredLifetimeSummary)> {
        self.lookups.whole("lifetime_summaries");
        self.inferred_lifetimes.iter().chain(&self.lifetime_imports)
    }

    pub fn set_inferred_imports(&mut self, imports: BTreeMap<String, ResourceFunctionContract>) {
        self.inferred_imports = imports;
    }

    #[must_use]
    pub fn inferred_imports(&self) -> &BTreeMap<String, ResourceFunctionContract> {
        self.lookups.whole("inferred_imports");
        &self.inferred_imports
    }

    pub fn set_cross_unit_import_identities(
        &mut self,
        identities: BTreeMap<String, CrossUnitImportIdentity>,
    ) {
        self.cross_unit_import_identities = identities;
    }

    #[must_use]
    pub fn cross_unit_import_identities(&self) -> &BTreeMap<String, CrossUnitImportIdentity> {
        self.lookups.whole("cross_unit_import_identities");
        &self.cross_unit_import_identities
    }

    pub fn set_link_resolution_sha256(&mut self, digest: Option<String>) {
        self.link_resolution_sha256 = digest;
    }

    #[must_use]
    pub fn link_resolution_sha256(&self) -> Option<&str> {
        self.link_resolution_sha256.as_deref()
    }

    /// Preserve the separately validated cross-unit import channel when a
    /// gateway pass rebuilds an environment to withdraw ordinary declarations
    /// or to prove a recursive component. The caller's certificate still
    /// replays every imported home and its link-resolution premise.
    pub fn carry_cross_unit_imports(&mut self, from: &Self) {
        from.lookups.whole("carry_cross_unit_imports");
        self.inferred_imports = from.inferred_imports.clone();
        self.lifetime_imports = from.lifetime_imports.clone();
        self.cross_unit_import_identities = from.cross_unit_import_identities.clone();
        self.link_resolution_sha256 = from.link_resolution_sha256.clone();
    }

    /// Exact source inputs, declared as assumptions rather than verified facts.
    #[must_use]
    pub fn source_assumptions(&self) -> &BTreeMap<String, String> {
        self.lookups.whole("source_assumptions");
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
        self.lookups.whole("inclusion_evidence");
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
        self.lookups.whole("pending_review");
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
        self.lookups.whole("declared_revisions");
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
        self.lookups.name(name, self.revision.contains_key(name));
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
        self.lookups.whole("pin_mismatch");
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
        // Card CALLEE-RETENTION-CLASS1: a pinned return-code disposition or
        // retaining declaration is withheld from its own table the same way,
        // and restored to it by [`Self::with_human_holds_in_force`].
        let (contract, relational, retaining) = if let Some(contract) = self.functions.remove(name)
        {
            (contract, None, None)
        } else if let Some(relational) = self.relational.remove(name) {
            let first = relational
                .classes
                .first()
                .map(|class| class.contract.clone())
                .ok_or_else(|| format!("`{name}` states no return-code class"))?;
            (first, Some(relational), None)
        } else if let Some(retaining) = self
            .retaining
            .remove(name)
            .or_else(|| self.guarded_retaining.remove(name))
        {
            (retaining.contract.clone(), None, Some(retaining))
        } else {
            return Err(format!("`{name}` is not a contract in force"));
        };
        self.pin_mismatch.insert(
            name.to_owned(),
            crate::contract_revision::PinMismatch {
                module,
                contract,
                reason,
                lock,
                detail,
                relational,
                retaining,
            },
        );
        Ok(())
    }

    /// Card W45-REFUTED-CONTRACT-WITHDRAWAL: the declarations some analysed
    /// body refutes, keyed by function name. Each is still in
    /// [`Self::functions`]; its call sites are held.
    #[must_use]
    pub fn refuted_by_body(&self) -> &crate::contract_refutation::RefutationTable {
        self.lookups.whole("refuted_by_body");
        &self.refuted_by_body
    }

    /// Record an audit's refutation table. Only a name of the function map
    /// is kept: a table entry for a declaration not in force here (withheld
    /// for review or a pin, or never declared) has no call site to hold.
    pub fn set_refuted_by_body(&mut self, table: &crate::contract_refutation::RefutationTable) {
        for name in table.keys() {
            self.lookups
                .name(name, self.functions.contains_key(name.as_str()));
        }
        self.refuted_by_body = table
            .iter()
            .filter(|(name, _)| self.functions.contains_key(*name))
            .map(|(name, refutations)| (name.clone(), refutations.clone()))
            .collect();
    }

    /// Card W45-REVIEW-HOLD-PREEMPTS-STATEMENT-REFUSAL: this environment with
    /// every declaration withheld for a human act — awaiting review
    /// ([`Self::pending_review`]) or for a pin mismatch
    /// ([`Self::pin_mismatch`]) — in force as written, and nothing held.
    /// `None` when nothing is withheld for either reason.
    ///
    /// It is the environment the named human act would produce if it accepted
    /// every held declaration as written. The gateway analyses under it only
    /// to find the reasons that would still stand, and only ever joins them
    /// onto a `Pending` row; it never publishes a verdict from it. The
    /// refutation table is kept as it is: a name it lists stays held.
    #[must_use]
    pub fn with_human_holds_in_force(&self) -> Option<Self> {
        self.lookups.whole("with_human_holds_in_force");
        // Card CELL-PROTOCOL-RELEASE-WINDOW-FRAMES (rule 38g): a declared
        // callee frame holds the calls it would admit for a human act too.
        let frames_held = !self.cell_frames_accepted && !self.cell_frames().is_empty();
        if self.pending_review.is_empty() && self.pin_mismatch.is_empty() && !frames_held {
            return None;
        }
        let mut resolved = self.clone();
        resolved.cell_frames_accepted = true;
        for (name, pending) in std::mem::take(&mut resolved.pending_review) {
            resolved.functions.insert(name, pending.contract);
        }
        for (name, withheld) in std::mem::take(&mut resolved.pin_mismatch) {
            // Card CALLEE-RETENTION-CLASS1: each declaration back in its own
            // table, never a disposition or a retention read as plain.
            if let Some(relational) = withheld.relational {
                resolved.relational.insert(name, relational);
            } else if let Some(retaining) = withheld.retaining {
                resolved.insert_retaining(name, retaining);
            } else {
                resolved.functions.insert(name, withheld.contract);
            }
        }
        Some(resolved)
    }

    /// The contract of `name` in the function map. Card
    /// VERDICT-CONTRACT-DEPENDENCIES: the lookup is noted in the thread's
    /// consultation log when one is open (evidence only; it changes nothing
    /// this returns).
    #[must_use]
    pub fn contract(&self, name: &str) -> Option<&ResourceFunctionContract> {
        // A declared contract always wins, even if a redundant import was
        // offered. The gateway refuses conflicting evidence at admission.
        let found = if self.has_guarded_interface(name) {
            None
        } else {
            self.functions
                .get(name)
                .or_else(|| self.inferred_imports.get(name))
        };
        crate::contract_consultation::note(name, found.is_some());
        self.lookups.name(name, found.is_some());
        found
    }

    #[must_use]
    pub fn functions(&self) -> &BTreeMap<String, ResourceFunctionContract> {
        self.lookups.whole("functions");
        &self.functions
    }

    /// Card CG23-RELATIONAL-POSTCONDITIONS: the return-code dispositions in
    /// force, keyed by function name. None of them is in [`Self::functions`].
    #[must_use]
    pub fn relational(&self) -> &BTreeMap<String, crate::relational_post::RelationalContract> {
        self.lookups.whole("relational");
        &self.relational
    }

    /// The return-code disposition of `name`, if it has one.
    #[must_use]
    pub fn relational_contract(
        &self,
        name: &str,
    ) -> Option<&crate::relational_post::RelationalContract> {
        let found = self.relational.get(name);
        crate::contract_consultation::note(name, found.is_some());
        self.lookups.name(name, found.is_some());
        found
    }

    /// Install a return-code disposition for `name`. The name leaves the
    /// function map: one name has one reading.
    pub fn insert_relational(
        &mut self,
        name: impl Into<String>,
        contract: crate::relational_post::RelationalContract,
    ) {
        let name = name.into();
        self.lookups.name(&name, self.functions.contains_key(&name));
        self.functions.remove(&name);
        self.relational.insert(name, contract);
    }

    /// Mark the disposition of `name` withdrawn for `reason` (its own body
    /// contradicts it). Every call site then refuses.
    pub fn withdraw_relational(&mut self, name: &str, reason: String) {
        self.lookups.name(name, self.relational.contains_key(name));
        if let Some(contract) = self.relational.get_mut(name) {
            contract.withdrawn = Some(reason);
        }
    }

    /// Card CALLEE-RETENTION-CLASS1: the declarations with retention clauses,
    /// keyed by function name. None of them is in [`Self::functions`].
    #[must_use]
    pub fn retaining(&self) -> &BTreeMap<String, crate::retained_by::RetainingContract> {
        self.lookups.whole("retaining");
        &self.retaining
    }

    /// Explicit S3/introspection access. This table supplies no ready effects.
    #[must_use]
    pub fn guarded_retaining(&self) -> &BTreeMap<String, crate::retained_by::RetainingContract> {
        self.lookups.whole("guarded_retaining");
        &self.guarded_retaining
    }
    pub fn carry_guarded_identities(&mut self, from: &Self) {
        from.lookups.whole("carry_guarded_identities");
        self.declared_guarded
            .extend(from.declared_guarded.iter().cloned());
    }
    #[must_use]
    pub fn declared_guarded(&self) -> &BTreeSet<String> {
        self.lookups.whole("declared_guarded");
        &self.declared_guarded
    }
    #[must_use]
    pub fn has_guarded_interface(&self, name: &str) -> bool {
        let found = self.declared_guarded.contains(name)
            || self.guarded_retaining.contains_key(name)
            || self
                .pin_mismatch
                .get(name)
                .and_then(|p| p.retaining.as_ref())
                .is_some_and(|c| c.is_condition_bearing());
        self.lookups.name(name, found);
        found
    }
    pub fn lifetime_declarations(
        &self,
    ) -> impl Iterator<Item = (&String, &crate::retained_by::RetainingContract)> {
        self.lookups.whole("lifetime_declarations");
        self.retaining
            .iter()
            .chain(&self.guarded_retaining)
            .filter(|(_, c)| c.lifetime.is_some())
    }
    /// Card **MK-EVENT-CELL-CROSS-FUNCTION** (gateway rule 38c, §C04.6.3):
    /// every record's declared cell protocols with the declarations stating
    /// them. Two declarations stating different protocols for one record leave
    /// that record out: no protocol is read for it (fail-closed). Card
    /// **CELL-PROTOCOL-PER-END-GUARDS** (rule 38f): a record's protocols are
    /// one per guard, compared as a whole, in the loader's sorted order.
    #[must_use]
    pub fn cell_protocols(
        &self,
    ) -> BTreeMap<String, (BTreeSet<String>, Vec<crate::descriptor_cell::CellProtocol>)> {
        self.lookups.whole("cell_protocols");
        type Stated = (BTreeSet<String>, Vec<crate::descriptor_cell::CellProtocol>);
        let mut out: BTreeMap<String, Stated> = BTreeMap::new();
        let mut conflicting = BTreeSet::new();
        for (host, declaration) in self.lifetime_declarations() {
            let mut stated: BTreeMap<String, Vec<crate::descriptor_cell::CellProtocol>> =
                BTreeMap::new();
            for protocol in declaration.lifetime.iter().flat_map(|l| &l.cells) {
                stated
                    .entry(protocol.record.clone())
                    .or_default()
                    .push(protocol.clone());
            }
            for (record, mut protocols) in stated {
                protocols.sort();
                match out.get_mut(&record) {
                    Some((hosts, known)) if *known == protocols => {
                        hosts.insert(host.clone());
                    }
                    Some(_) => {
                        conflicting.insert(record);
                    }
                    None => {
                        out.insert(record, (BTreeSet::from([host.clone()]), protocols));
                    }
                }
            }
        }
        out.retain(|record, _| !conflicting.contains(record));
        out
    }
    /// Card **CELL-PROTOCOL-RELEASE-WINDOW-FRAMES** (gateway rule 38g,
    /// §C04.6.4): every record's declared callee frames with the declarations
    /// stating them. A declaration states a record's frames beside its
    /// protocol, so the frames are compared as the protocols are: a record
    /// keeps its frames only where it keeps its protocol
    /// ([`Self::cell_protocols`]) and every declaration stating that protocol
    /// states the same callees (fail-closed: otherwise none).
    #[must_use]
    pub fn cell_frames(&self) -> BTreeMap<String, BTreeSet<String>> {
        self.lookups.whole("cell_frames");
        let protocols = self.cell_protocols();
        let mut out: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
        let mut seen = BTreeSet::new();
        let mut conflicting = BTreeSet::new();
        for (_, declaration) in self.lifetime_declarations() {
            let Some(lifetime) = &declaration.lifetime else {
                continue;
            };
            let records = lifetime
                .cells
                .iter()
                .map(|protocol| protocol.record.clone())
                .collect::<BTreeSet<_>>();
            for record in records {
                let callees = lifetime
                    .frames
                    .iter()
                    .filter(|frame| frame.record == record)
                    .map(|frame| frame.callee.clone())
                    .collect::<BTreeSet<_>>();
                if seen.insert(record.clone()) {
                    out.insert(record, callees);
                } else if out.get(&record) != Some(&callees) {
                    conflicting.insert(record);
                }
            }
        }
        out.retain(|record, callees| {
            !conflicting.contains(record) && protocols.contains_key(record) && !callees.is_empty()
        });
        out
    }

    /// Whether a human act's acceptance of every declared callee frame is in
    /// force (rule 38g; only [`Self::with_human_holds_in_force`] sets it).
    #[must_use]
    pub fn cell_frames_accepted(&self) -> bool {
        self.cell_frames_accepted
    }

    /// Enforce the partition even for deserialized inputs, before any legacy
    /// prepass can read the raw retaining table.
    pub fn validate_lifetime_partition(&self) -> Result<(), String> {
        self.lookups.whole("validate_lifetime_partition");
        if self.lifetime_imports.iter().any(|(name, summary)| {
            (summary.effects.is_empty() && summary.removals.is_empty())
                || (!summary.effects.is_empty() && !summary.removals.is_empty())
                || summary.contract.result != ResourceReturnMode::None
                || summary
                    .contract
                    .parameters
                    .iter()
                    .any(|p| !matches!(p, ResourceParameterMode::Borrowed))
                || summary.holder_frames.iter().any(|f| {
                    f.parameter >= summary.contract.parameters.len()
                        || f.record.is_empty()
                        || f.fields.len() != 1
                })
                || summary.removals.len() > 4096
                || summary.removals.iter().any(|r| {
                    r.retained >= summary.contract.parameters.len()
                        || r.holder >= summary.contract.parameters.len()
                })
                || self.inferred_imports.contains_key(name)
                || self.functions.contains_key(name)
                || self.inferred_lifetimes.contains_key(name)
                || self.has_guarded_interface(name)
                || self.retaining.contains_key(name)
        }) || self.retaining.values().any(|c| c.is_condition_bearing())
            || self.declared_guarded.iter().any(|name| {
                self.functions.contains_key(name)
                    || self.inferred_lifetimes.contains_key(name)
                    || self.retaining.contains_key(name)
            })
            || self.pin_mismatch.iter().any(|(name, p)| {
                p.retaining
                    .as_ref()
                    .is_some_and(|c| c.is_condition_bearing())
                    && !self.declared_guarded.contains(name)
            })
            || self.guarded_retaining.iter().any(|(name, c)| {
                !c.is_condition_bearing()
                    || !self.declared_guarded.contains(name)
                    || self.functions.contains_key(name)
                    || self.inferred_lifetimes.contains_key(name)
                    || self.retaining.contains_key(name)
            })
        {
            return Err("guarded lifetime declarations are not in their exclusive S3 table".into());
        }
        Ok(())
    }

    #[must_use]
    pub fn inferred_lifetimes(
        &self,
    ) -> &BTreeMap<String, crate::loop_lifetime::InferredLifetimeSummary> {
        self.lookups.whole("inferred_lifetimes");
        &self.inferred_lifetimes
    }
    pub fn insert_inferred_lifetime(
        &mut self,
        name: String,
        summary: crate::loop_lifetime::InferredLifetimeSummary,
    ) {
        if self.has_guarded_interface(&name) {
            return;
        }
        self.functions.remove(&name);
        self.inferred_lifetimes.insert(name, summary);
    }

    /// The retaining declaration of `name`, if it has one.
    #[must_use]
    pub fn retaining_contract(&self, name: &str) -> Option<&crate::retained_by::RetainingContract> {
        // Conditional prerequisites require the explicit S3 consumer. An
        // unconditional type footprint changes no custody/callback effect;
        // only the forwarding reader consumes that separately scoped assertion.
        let found = self.retaining.get(name).filter(|c| {
            c.lifetime
                .as_ref()
                .is_none_or(|l| l.blocks.is_empty() && l.bindings.is_empty())
        });
        crate::contract_consultation::note(name, found.is_some());
        self.lookups.name(name, found.is_some());
        found
    }

    /// S3 declarations are withheld from legacy custody/callback consumers.
    /// Reading this table is not evidence that its guarded obligations hold.
    #[must_use]
    pub fn lifetime_contract(&self, name: &str) -> Option<&crate::retained_by::RetainingContract> {
        let found = self
            .guarded_retaining
            .get(name)
            .or_else(|| self.retaining.get(name))
            .filter(|c| c.lifetime.is_some());
        crate::contract_consultation::note(name, found.is_some());
        self.lookups.name(name, found.is_some());
        found
    }

    /// Card CALLEE-RETENTION-CLASS1: carry `from`'s return-code dispositions
    /// and retaining declarations into this environment, except the names
    /// `withdrawn` holds. An environment rebuilt from another's function map
    /// (a unit's withdrawals) keeps them this way, so a withdrawal of one name
    /// never drops every other declaration of those tables.
    pub fn carry_side_declarations(&mut self, from: &Self, withdrawn: impl Fn(&str) -> bool) {
        from.lookups.whole("carry_side_declarations");
        self.declared_guarded
            .extend(from.declared_guarded.iter().cloned());
        self.inferred_lifetimes.extend(
            from.inferred_lifetimes
                .iter()
                .filter(|(name, _)| !withdrawn(name))
                .map(|(name, summary)| (name.clone(), summary.clone())),
        );
        for (name, contract) in &from.relational {
            if !withdrawn(name) {
                self.relational.insert(name.clone(), contract.clone());
            }
        }
        // Card CELL-GUARD-RESULT-RANGE-CHECKED: the route's derived result
        // sets travel with the side tables.
        for (name, fact) in &from.result_ranges {
            if !withdrawn(name) {
                self.result_ranges.insert(name.clone(), fact.clone());
            }
        }
        for (name, contract) in from.retaining.iter().chain(&from.guarded_retaining) {
            if !withdrawn(name) {
                self.insert_retaining(name.clone(), contract.clone());
            }
        }
    }

    /// Card **SLOT-MOVE-AND-INTERIOR-BORROW** (gateway rule 13a): remove
    /// `name`'s declaration from the function map and the retaining table, so
    /// every call site of this environment sees an uncontracted callee. Used
    /// for a translation unit whose own body of `name` does not validate a
    /// declared slot or borrowed-result clause.
    pub fn withdraw_declaration(&mut self, name: &str) {
        self.lookups.name(name, self.functions.contains_key(name));
        self.inferred_lifetimes.remove(name);
        self.functions.remove(name);
        self.retaining.remove(name);
        self.guarded_retaining.remove(name);
    }

    /// Install a retaining declaration for `name`. The name leaves the
    /// function map: one name has one reading.
    pub fn insert_retaining(
        &mut self,
        name: impl Into<String>,
        contract: crate::retained_by::RetainingContract,
    ) {
        let name = name.into();
        if !contract.is_condition_bearing() && self.has_guarded_interface(&name) {
            return;
        }
        self.functions.remove(&name);
        self.inferred_lifetimes.remove(&name);
        if contract.is_condition_bearing() {
            self.declared_guarded.insert(name.clone());
            self.retaining.remove(&name);
            self.guarded_retaining.insert(name, contract);
        } else {
            self.guarded_retaining.remove(&name);
            self.retaining.insert(name, contract);
        }
    }
}
