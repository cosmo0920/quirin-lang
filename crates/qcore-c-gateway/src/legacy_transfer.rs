//! Card GATEWAY-ANALYSIS-PATH-OPTIMIZATION: the abstract state and the
//! transfer functions: pointers, derivations, entry custody, exit
//! observations and the flow analysis over a CFG.
//!
//! Moved verbatim from `legacy_c.rs` (only visibility widened to
//! `pub(super)` where the parent or a sibling module reads an item).

use super::*;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(super) struct AbstractPointer {
    pub(super) targets: BTreeSet<usize>,
    pub(super) unknown: bool,
    pub(super) may_be_null: bool,
    /// Card W43-LIBC-RESULT-ALIAS: whether this binding *is* its targets'
    /// identity or a non-owning pointer derived from them by a declared
    /// [`ResourceReturnMode::Derived`] result. Copied by every alias, joined by
    /// [`Derivation::join`].
    pub(super) derivation: Derivation,
}

/// How a pointer binding relates to the resource identities it targets.
///
/// Card W43-LIBC-RESULT-ALIAS. A `Derived` binding shares its base's targets,
/// so releasing the base invalidates it through the ordinary site status, but
/// it owns nothing: a release, escape, consumption, store or reallocation
/// through it is never applied to the base (see [`derived_custody_refusal`]),
/// its null test narrows only itself, and its nullness never flows back onto
/// the base's site.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) enum Derivation {
    /// The binding denotes its targets' identities themselves.
    #[default]
    Identity,
    /// The binding was produced by a declared derived result, or (card
    /// W43-INDEX-TRAVERSAL) by pointer arithmetic on a binding.
    Derived {
        extent: DerivedExtent,
        const_dropped: bool,
        /// Card W43-RESULT-ALIAS-RETURNED: whether some derivation step on
        /// the way here may return NULL for a non-null base (`strchr`). It is
        /// the *relative* nullness a derived-result summary states, kept apart
        /// from [`AbstractPointer::may_be_null`], which also carries the base's
        /// own nullness. It is joined by disjunction and never narrowed, so it
        /// can only over-state the summary's `nullable`. No custody rule reads
        /// it.
        nullable: bool,
        /// Card W43-INDEX-TRAVERSAL: what pointer arithmetic did to the binding.
        stepped: Stepped,
    },
    /// Different paths disagree. Treated as derived wherever that refuses, and
    /// never as derived where that would accuse.
    Mixed,
}

/// Card **W43-INDEX-TRAVERSAL**: what pointer arithmetic did to a derived
/// binding's position in its object.
///
/// `No` is every derivation card W43-LIBC-RESULT-ALIAS made, and behaves
/// exactly as it did. The two stepped values describe a binding that went
/// through at least one `P++`, `P += E`, `P = Q ± E` or `*P++`:
///
/// * `Forward` — every step moved it forward by a positive literal amount, so
///   it lies strictly after the pointer it started from. Releasing it is an
///   `invalid_release` candidate, as an interior result's release is.
/// * `Any` — it may lie anywhere in its object, the start included: some step
///   went backward or by a variable amount, or a path joined it with the base
///   identity itself (a loop's entry edge). Releasing it is refused, never
///   accused, because it may be the value the allocator returned.
///
/// The join keeps the two families apart: a stepped binding joined with an
/// unstepped derived one is `Mixed`, so the lattice stays a lattice (see
/// [`Derivation::join`]) and no existing derived result changes its answer.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum Stepped {
    #[default]
    No,
    Forward,
    Any,
}

impl Derivation {
    pub(super) const fn is_identity(self) -> bool {
        matches!(self, Self::Identity)
    }

    /// Card **W43-INDEX-TRAVERSAL**: the derivation of `P ± E` over a binding
    /// of this derivation. Always interior; a mixed base stays mixed.
    pub(super) fn step(self, forward: bool) -> Self {
        let movement = if forward {
            Stepped::Forward
        } else {
            Stepped::Any
        };
        match self {
            Self::Identity => Self::Derived {
                extent: DerivedExtent::Interior,
                const_dropped: false,
                // Arithmetic on a non-null pointer never makes it null.
                nullable: false,
                stepped: movement,
            },
            Self::Derived {
                extent,
                const_dropped,
                nullable,
                stepped,
            } => Self::Derived {
                // Card SLOT-MOVE-AND-INTERIOR-BORROW (rule 12y): arithmetic on
                // a borrowed pointer stays inside the parent's object and keeps
                // the borrow; every other derivation becomes interior.
                extent: if extent == DerivedExtent::Borrowed {
                    DerivedExtent::Borrowed
                } else {
                    DerivedExtent::Interior
                },
                const_dropped,
                nullable,
                // A declared interior result stepped forward still lies after
                // its base's start; anything that may be at the start stays so.
                stepped: stepped.max(movement),
            },
            Self::Mixed => Self::Mixed,
        }
    }

    /// Join two paths' derivations. A definitely-null binding carries no
    /// relation, so it takes the other path's; otherwise equal derivations are
    /// kept, `const_dropped` and the relative `nullable` are joined by
    /// disjunction within one extent, and any other disagreement is `Mixed`.
    ///
    /// Card **W43-INDEX-TRAVERSAL** adds one join and keeps the rest: an
    /// identity and a *stepped* pointer into it join to a stepped pointer that
    /// may be anywhere in the object (`Any`), which is what a loop's header
    /// sees for `for (p = s; *p; p++)`. Two stepped pointers join to the
    /// larger movement. A stepped and an unstepped derivation, and an identity
    /// and an unstepped derivation, still join to `Mixed`.
    pub(super) fn join(one: &AbstractPointer, other: &AbstractPointer) -> Self {
        let definitely_null = |pointer: &AbstractPointer| {
            pointer.targets.is_empty() && !pointer.unknown && pointer.may_be_null
        };
        if definitely_null(other) {
            return one.derivation;
        }
        if definitely_null(one) {
            return other.derivation;
        }
        Self::join_derivations(one.derivation, other.derivation)
    }

    pub(super) fn join_derivations(one: Self, other: Self) -> Self {
        match (one, other) {
            (left, right) if left == right => left,
            (
                Self::Identity,
                Self::Derived {
                    extent: DerivedExtent::Interior,
                    const_dropped,
                    nullable,
                    stepped: Stepped::Forward | Stepped::Any,
                },
            )
            | (
                Self::Derived {
                    extent: DerivedExtent::Interior,
                    const_dropped,
                    nullable,
                    stepped: Stepped::Forward | Stepped::Any,
                },
                Self::Identity,
            ) => Self::Derived {
                extent: DerivedExtent::Interior,
                const_dropped,
                // The identity path carries no relative nullness of its own.
                nullable,
                stepped: Stepped::Any,
            },
            (
                Self::Derived {
                    extent,
                    const_dropped,
                    nullable,
                    stepped,
                },
                Self::Derived {
                    extent: other_extent,
                    const_dropped: other_dropped,
                    nullable: other_nullable,
                    stepped: other_stepped,
                },
            ) if extent == other_extent
                && ((stepped == Stepped::No) == (other_stepped == Stepped::No)) =>
            {
                Self::Derived {
                    extent,
                    const_dropped: const_dropped || other_dropped,
                    nullable: nullable || other_nullable,
                    stepped: stepped.max(other_stepped),
                }
            }
            _ => Self::Mixed,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(super) struct AbstractState {
    pub(super) pointers: BTreeMap<String, AbstractPointer>,
    pub(super) resources: BTreeMap<usize, u8>,
    pub(super) nullable_sites: BTreeSet<usize>,
    pub(super) conditional_replacements: BTreeMap<usize, BTreeSet<usize>>,
    pub(super) contained: BTreeMap<usize, BTreeSet<usize>>,
    /// O00 blocker B08: `owner site -> field name -> child sites` recorded by
    /// an admitted aggregate field store. Deliberately separate from
    /// `contained`: escaping the owner escapes its live linked children, while
    /// *releasing* the owner does not cascade, because a plain C field store
    /// carries no declared destructor obligation (premise P-B08-SHALLOW).
    pub(super) field_links: BTreeMap<usize, BTreeMap<String, BTreeSet<usize>>>,
    /// Must-held physical link values; explicit NULL differs from absent/unknown.
    pub(super) lookup_links: BTreeMap<(usize, String, String), Option<usize>>,
    /// Identity-refined lookup aliases still obey their parent's borrow lifetime.
    pub(super) lookup_aliases: BTreeSet<String>,
    /// Card **B08-FIELD-LINK-JOIN-SILENT-UNRESOLVED** (rule 11c): the
    /// `(owner, field, child)` triples of `field_links` that may not hold on
    /// every path reaching here (linked on one side of a join). Always a
    /// subset of `field_links`; empty in every function whose joins meet no
    /// one-sided link, which keeps the join, and with it the work-list
    /// schedule, as it was there (constraint 6).
    pub(super) partial_links: BTreeMap<usize, BTreeMap<String, BTreeSet<usize>>>,
    /// Custody retained after a rejected effect; its residual is not a proven
    /// leak because the unmodeled action may have discharged it.
    pub(super) unresolved_effect_sites: BTreeSet<usize>,
    pub(super) widened: bool,
    /// Whether some instruction on *this* path was refused or reported an
    /// unresolved effect (O04). It is the per-path counterpart of the
    /// function-wide `Unknown` status: a path carrying it can support no
    /// custody claim at the exits it reaches. Joined by disjunction, so it is
    /// monotone and the work list still terminates.
    pub(super) unmodeled: bool,
    /// Card **W43-INDEXED-RESOURCE-COLLECTIONS**: the element ranges of every
    /// array site an element operation touched. A site absent here was never
    /// stored into, so all its elements are unfilled. Empty in every function
    /// that lowers no element operation, which keeps the join, and with it
    /// the work-list schedule, exactly as it was there (constraint 6).
    pub(super) collections: BTreeMap<usize, crate::indexed_collection::Collection>,
    /// Card **W43-INDEXED-RESOURCE-COLLECTIONS**: `name < bound` facts a
    /// counted loop's body edge established, dropped when either name is
    /// written. Recorded only while some collection exists.
    pub(super) index_bounds: BTreeMap<String, BTreeSet<crate::indexed_collection::Term>>,
    /// Card **W43-INDEXED-RESOURCE-COLLECTIONS**: bound names whose clamped
    /// value `max(0, name)` is zero — a counted loop left with its index still
    /// zero, so it ran no iteration. Dropped when the name is written.
    pub(super) clamped_zero: BTreeSet<String>,
    /// Card **W43-INDEXED-RESOURCE-COLLECTIONS**: the collection names
    /// (indices, bounds, counters) this path binds to zero, kept by the
    /// collection rules themselves so that a vacuous path cannot weaken them.
    pub(super) index_zero: BTreeSet<String>,
    /// Card **W43-INDEXED-RESOURCE-COLLECTIONS**: no execution reaches here for
    /// the collection facts — a counted loop's body edge whose bound is zero
    /// when clamped, which its never-negative index cannot be below. The
    /// collection part of this state claims nothing and joins as the
    /// identity; the edge, and every other map on it, is kept.
    pub(super) collection_vacuous: bool,
    /// Card **W45-FB-RETRY-LIFETIME**: the sites this path published into a
    /// caller's container ([`container_publication`]). Joined by intersection,
    /// so a site is published only when every arriving path published it.
    /// Empty in every function that calls no declared publication, which keeps
    /// the join, and with it the work-list schedule, as it was (constraint 6).
    pub(super) published: BTreeSet<usize>,
    /// Card **CG25-RESOURCE-KINDS** (rule 11v): `status name -> (site,
    /// storage)` for an in-place initialiser whose status the name still
    /// holds. Joined by intersection; empty in every function that calls no
    /// declared in-place initialiser, which keeps the join as it was.
    pub(super) status_witnesses: BTreeMap<String, (usize, String)>,
    /// Card **CALLEE-RETENTION-CLASS1** (rule 12t): what a holder retains on
    /// this path ([`retained_by_route`]). Joined by union, so a retention one
    /// arriving path recorded is kept. Empty in every function that calls no
    /// retaining callee, which keeps the join as it was.
    pub(super) retained: BTreeSet<(crate::retained_by::HeldObject, crate::retained_by::HeldBy)>,
    /// Card **RETENTION-UNMATCHED-REMOVAL-POISONS** (rule 12t): the records
    /// of `retained` a removal through another or an undecided holder
    /// poisoned ([`crate::retained_by::remove`]); a poisoned record ends only
    /// when its holder ends. Joined by union, so a record poisoned on one
    /// arriving path stays poisoned. Empty in every function that calls no
    /// removal, which keeps the join as it was.
    pub(super) poisoned: BTreeSet<crate::retained_by::Record>,
    pub(super) forwarded_lifetimes:
        BTreeMap<crate::retained_by::Record, crate::loop_lifetime_summary::ForwardedLifetime>,
    /// Card **SLOT-MOVE-AND-INTERIOR-BORROW** (rules 12x, 12y): names whose
    /// value a slot move or a borrow's end invalidated; a read refuses.
    /// Joined by union; empty in every function that calls no slot-moving or
    /// borrowed-result callee, which keeps the join as it was.
    pub(super) invalidated: BTreeMap<String, slot_move_route::Invalid>,
    /// Card **SLOT-MOVE-AND-INTERIOR-BORROW** (rule 12y): the borrowed results
    /// each name holds. Joined by union.
    pub(super) borrows: BTreeMap<String, BTreeSet<slot_move_route::Borrow>>,
    /// Card **REFCOUNT-RESOURCE-KIND** (rule 13b): the reference families
    /// this path holds, by site, with their counts. Joined by site, a count
    /// the arriving paths disagree on becoming undecided. Empty in every
    /// function that calls no keep, which keeps the join as it was.
    pub(super) references: BTreeMap<usize, crate::reference_count::Family>,
    /// Card **CG22-CALLBACK-REGISTRY-CONTRACT** (rules 13w, 13x): the
    /// registrations in force on this path, by holder. A registration not
    /// made alike on every arriving path joins to undecided. Empty in every
    /// function that calls no registering callee, which keeps the join as it
    /// was.
    pub(super) registries: BTreeMap<callback_route::RegistryKey, callback_route::Registration>,
    /// Card **OP-CONSTRUCTOR-DECLARATIONS** (rules 12o, 13w, 14e): the
    /// states a constructor stated of a tracked result, by site and member.
    /// A state not held alike on every arriving path is undecided (dropped).
    /// Empty in every function that calls no constructor with a stated
    /// state, which keeps the join as it was.
    pub(super) constructed: BTreeMap<(usize, String), super::constructed_route::ObjectState>,
    /// Card **FB-TEARDOWN-DESTROY-DECLARATIONS** (rule 30 at a direct call):
    /// the sites a direct call took through a path it leaves `#stale` (the
    /// member still designates them); a later release or hand-off of one
    /// refuses as a double release. Joined by union; empty in every function
    /// that calls no such declaration, which keeps the join as it was.
    pub(super) member_path_taken: BTreeSet<usize>,
    /// Card **MK-EVENT-DESCRIPTOR-QUANT** (rule 38): the descriptor cells of
    /// this path. Empty in every function that calls no declaration with a
    /// cell state, which keeps the join as it was.
    pub(super) cells: rule_38_descriptor_cells::Cells<rule_38_descriptor_cells::cells::Key>,
    /// Card **MK-EVENT-DESCRIPTOR-QUANT** (rule 38): a plain name holding a
    /// cell-creating call's result, with the identities it witnesses and the
    /// creation's class. Joined by intersection.
    pub(super) cell_witnesses:
        BTreeMap<String, (Vec<u64>, qcore_schemas::descriptor_cell::WitnessGuard)>,
    /// Card **MK-EVENT-CELL-CROSS-FUNCTION** (rule 38c): the facts of the
    /// records with a declared cell protocol this path read. Empty in every
    /// environment that declares no protocol, which keeps the join as it was.
    pub(super) cell_protocols: BTreeMap<
        super::descriptor_cells_route::ProtocolRoot,
        super::descriptor_cells_route::HostedFact,
    >,
}

/// The scalar facts one abstract state proves: a name bound to the null
/// constant (the lowering of `i = 0`) is zero, and a counted loop's body edge
/// bounds its index.
impl crate::indexed_collection::Scalars for AbstractState {
    fn is_zero(&self, name: &str) -> bool {
        self.index_zero.contains(name)
    }

    fn clamped_zero(&self, name: &str) -> bool {
        self.clamped_zero.contains(name)
    }

    fn upper_bounds(&self, name: &str) -> Vec<crate::indexed_collection::Term> {
        self.index_bounds
            .get(name)
            .map(|bounds| bounds.iter().cloned().collect())
            .unwrap_or_default()
    }
}

pub(super) const MAX_ALIAS_TARGETS: usize = 8;

impl AbstractState {
    /// Join an incoming edge's state into this one; returns whether anything
    /// changed.
    ///
    /// Card **W43-INDEX-TRAVERSAL**: `scalars` are the function's declared
    /// scalar names ([`TraversalNames::scalars`]). A declared scalar's binding
    /// carries at most a branch fact and never a custody claim, so when one
    /// path has no binding for it and the other a binding with no target, the
    /// join has no fact either — the absent binding, which admits both branches
    /// of a later test and escapes nothing — rather than the unknown, possibly
    /// null *pointer* the generic arm makes of it. A scalar binding that does
    /// carry a target (a descriptor a contracted call produced) keeps the
    /// generic arm.
    pub(super) fn join(&mut self, incoming: &Self, scalars: &BTreeSet<String>) -> bool {
        let before = self.clone();
        let variables = self
            .pointers
            .keys()
            .chain(incoming.pointers.keys())
            .cloned()
            .collect::<BTreeSet<_>>();
        for variable in variables {
            let one_sided_scalar_fact = scalars.contains(&variable)
                && match (
                    self.pointers.get(&variable),
                    incoming.pointers.get(&variable),
                ) {
                    (Some(pointer), None) | (None, Some(pointer)) => pointer.targets.is_empty(),
                    _ => false,
                };
            if one_sided_scalar_fact {
                self.pointers.remove(&variable);
                continue;
            }
            match (
                self.pointers.get(&variable).cloned(),
                incoming.pointers.get(&variable),
            ) {
                (Some(mut current), Some(other)) => {
                    current.derivation = Derivation::join(&current, other);
                    current.targets.extend(other.targets.iter().copied());
                    current.unknown |= other.unknown;
                    current.may_be_null |= other.may_be_null;
                    if current.targets.len() > MAX_ALIAS_TARGETS {
                        current.targets.clear();
                        current.unknown = true;
                        self.widened = true;
                    }
                    self.pointers.insert(variable, current);
                }
                (Some(mut current), None) => {
                    current.unknown = true;
                    current.may_be_null = true;
                    self.pointers.insert(variable, current);
                }
                (None, Some(other)) => {
                    let mut joined = other.clone();
                    joined.unknown = true;
                    joined.may_be_null = true;
                    self.pointers.insert(variable, joined);
                }
                (None, None) => {}
            }
        }
        for (&site, &status) in &incoming.resources {
            self.resources
                .entry(site)
                .and_modify(|current| *current |= status)
                .or_insert(status);
        }
        self.nullable_sites
            .extend(incoming.nullable_sites.iter().copied());
        self.unresolved_effect_sites
            .extend(incoming.unresolved_effect_sites.iter().copied());
        for pointer in self.pointers.values() {
            // Card W43-LIBC-RESULT-ALIAS: a derived pointer's nullness is its
            // own (`strchr` returns NULL for a live base), never its base's.
            if pointer.may_be_null && !matches!(pointer.derivation, Derivation::Derived { .. }) {
                self.nullable_sites.extend(pointer.targets.iter().copied());
            }
        }
        for (&site, replaced) in &incoming.conditional_replacements {
            self.conditional_replacements
                .entry(site)
                .or_default()
                .extend(replaced.iter().copied());
        }
        for (&owner, children) in &incoming.contained {
            self.contained
                .entry(owner)
                .or_default()
                .extend(children.iter().copied());
        }
        self.lookup_links
            .retain(|key, value| incoming.lookup_links.get(key) == Some(value));
        self.lookup_aliases
            .extend(incoming.lookup_aliases.iter().cloned());
        self.join_field_links(incoming);
        container_publication::join(&mut self.published, &incoming.published);
        in_place_custody::join_witnesses(&mut self.status_witnesses, &incoming.status_witnesses);
        self.retained.extend(incoming.retained.iter().cloned());
        self.poisoned.extend(incoming.poisoned.iter().cloned());
        self.forwarded_lifetimes.clear();
        for record in &self.retained {
            let left = before.forwarded_lifetimes.get(record);
            let right = incoming.forwarded_lifetimes.get(record);
            let proof = match (
                before.retained.contains(record),
                incoming.retained.contains(record),
            ) {
                (true, true) => left.filter(|proof| Some(*proof) == right),
                (true, false) => left,
                (false, true) => right,
                (false, false) => None,
            };
            if let Some(proof) = proof {
                self.forwarded_lifetimes
                    .insert(record.clone(), proof.clone());
            }
        }
        slot_move_route::join(self, incoming);
        reference_route::join(self, incoming);
        callback_route::join(&mut self.registries, &incoming.registries);
        super::constructed_route::join(&mut self.constructed, &incoming.constructed);
        self.member_path_taken
            .extend(incoming.member_path_taken.iter().copied());
        // Card MK-EVENT-CELL-CROSS-FUNCTION (rule 38c), read against each
        // side's own table before the tables join.
        super::descriptor_cells_route::join_protocols(self, incoming);
        // Card MK-EVENT-DESCRIPTOR-QUANT (rule 38).
        if !self.cells.is_empty() || !incoming.cells.is_empty() {
            self.cells.join(&incoming.cells);
        }
        super::descriptor_cells_route::join_witnesses(
            &mut self.cell_witnesses,
            &incoming.cell_witnesses,
        );
        self.widened |= incoming.widened;
        self.unmodeled |= incoming.unmodeled;
        self.join_collections(&before, incoming);
        *self != before
    }

    /// Card **W43-INDEXED-RESOURCE-COLLECTIONS**: join the element ranges of
    /// each collection, each side under its own scalar facts (`before` is
    /// this side as it was before the pointer join forgot them). Two sides
    /// that do not collapse to one form make the collection `Unknown`, which
    /// is absorbing, so the work list still terminates. Index facts are
    /// intersected.
    pub(super) fn join_collections(&mut self, before: &Self, incoming: &Self) {
        use crate::indexed_collection::{Collection, Ranges};
        // A vacuous side reaches here with no execution: the other side's
        // collection facts stand.
        if incoming.collection_vacuous && !before.collection_vacuous {
            self.restore_collection_facts(before);
            return;
        }
        if before.collection_vacuous {
            self.restore_collection_facts(incoming);
            self.collection_vacuous = incoming.collection_vacuous;
            return;
        }
        self.clamped_zero = before
            .clamped_zero
            .intersection(&incoming.clamped_zero)
            .cloned()
            .collect();
        self.index_zero = before
            .index_zero
            .intersection(&incoming.index_zero)
            .cloned()
            .collect();
        self.index_bounds = intersect_bounds(&before.index_bounds, &incoming.index_bounds);
        if before.collections.is_empty() && incoming.collections.is_empty() {
            return;
        }
        let sites = before
            .collections
            .keys()
            .chain(incoming.collections.keys())
            .copied()
            .collect::<BTreeSet<_>>();
        for site in sites {
            let mine = before.collections.get(&site).cloned().unwrap_or_default();
            let theirs = incoming.collections.get(&site).cloned().unwrap_or_default();
            let joined = match (&mine, &theirs) {
                (Collection::Known(left), Collection::Known(right)) => {
                    Ranges::join(left, before, right, incoming)
                        .map_or(Collection::Unknown, Collection::Known)
                }
                _ => Collection::Unknown,
            };
            self.collections.insert(site, joined);
        }
    }

    pub(super) fn restore_collection_facts(&mut self, from: &Self) {
        self.collections = from.collections.clone();
        self.index_bounds = from.index_bounds.clone();
        self.clamped_zero = from.clamped_zero.clone();
        self.index_zero = from.index_zero.clone();
        self.collection_vacuous = from.collection_vacuous;
    }
}

pub(super) fn intersect_bounds(
    left: &BTreeMap<String, BTreeSet<crate::indexed_collection::Term>>,
    right: &BTreeMap<String, BTreeSet<crate::indexed_collection::Term>>,
) -> BTreeMap<String, BTreeSet<crate::indexed_collection::Term>> {
    left.iter()
        .filter_map(|(name, bounds)| {
            let common = bounds
                .intersection(right.get(name)?)
                .cloned()
                .collect::<BTreeSet<_>>();
            (!common.is_empty()).then(|| (name.clone(), common))
        })
        .collect()
}

/// C02-P-ALIAS: the may-alias facts in force on entry to one node.
///
/// They are computed by [`alias_facts`], a **separate** fixed point over the
/// same CFG, and never live in [`AbstractState`]. That separation is deliberate
/// and was forced by measurement: carrying them in the custody state changed
/// how often `AbstractState::join` reported a change, which reordered the
/// custody work list, and `report_leaks` runs on *intermediate* iterates of an
/// exit node rather than on its settled state. Two leak diagnostics appeared in
/// one corpus function from that reordering alone, with the rule's own refusal
/// disabled. Keeping the facts out of the custody state makes this card
/// scheduling-neutral; the underlying order-sensitivity of in-loop leak
/// reporting is recorded as follow-on `C02-P-ALIAS-LEAK-ORDER` and deliberately
/// not repaired here.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(super) struct NodeAliasFacts {
    /// Names released on some path reaching this node. A *may* fact, joined by
    /// union. Deliberately never cleared by a rebinding: reassigning the name
    /// that released an object does not resurrect the object.
    pub(super) released: BTreeSet<String>,
    /// Name pairs a pointer-inequality guard proved distinct on *every* path
    /// reaching this node, each stored with its two names in ascending order.
    /// A *must* fact, joined by intersection and dropped when either of its
    /// names is rebound.
    pub(super) distinct: BTreeSet<(String, String)>,
}

impl NodeAliasFacts {
    /// Join an incoming edge's facts. Returns whether anything changed.
    pub(super) fn absorb(&mut self, incoming: &Self) -> bool {
        let before = self.clone();
        self.released.extend(incoming.released.iter().cloned());
        self.distinct
            .retain(|pair| incoming.distinct.contains(pair));
        *self != before
    }

    /// Whether a pointer-inequality guard proved these two names distinct here.
    pub(super) fn proved_distinct(&self, one: &str, other: &str) -> bool {
        let pair = if one <= other {
            (one.to_owned(), other.to_owned())
        } else {
            (other.to_owned(), one.to_owned())
        };
        self.distinct.contains(&pair)
    }
}

/// The plain name one instruction *releases*, if any.
///
/// Exactly the sites at which [`apply_resource_action`] runs with `escape`
/// false: an explicit `Release`, and every declared `Released` position of a
/// contracted call. Consuming a value is deliberately outside v1
/// (`C02-P-ALIAS-CONSUME`).
pub(super) fn released_names(instruction: &LegacyInstruction) -> Vec<&str> {
    // Only a *plain name* is related by this rule: releasing `p->field`
    // releases a member, not the name `p`.
    let plain = |place: &str| crate::may_alias::place_root(place) == Some(place);
    match instruction {
        LegacyInstruction::Release { source } if plain(source) => vec![source.as_str()],
        LegacyInstruction::ContractCall {
            arguments,
            contract,
            ..
        } => contract
            .parameters
            .iter()
            .zip(arguments)
            .filter_map(|(mode, argument)| {
                matches!(mode, ResourceParameterMode::Released { .. })
                    .then_some(argument.as_deref()?)
                    .filter(|place| plain(place))
            })
            .collect(),
        _ => Vec::new(),
    }
}

/// The names one instruction binds, whose guard facts it therefore invalidates.
///
/// Exactly the destinations [`transfer`] and [`create_resource`] write.
pub(super) fn rebound_names(instruction: &LegacyInstruction) -> Vec<&str> {
    let mut names = match instruction {
        LegacyInstruction::Acquire { destination, .. }
        | LegacyInstruction::Alias { destination, .. }
        | LegacyInstruction::OpaqueValue { destination, .. }
        | LegacyInstruction::VaArgPointer { destination }
        | LegacyInstruction::ForgetScalar { destination }
        | LegacyInstruction::PointerStep { destination, .. }
        | LegacyInstruction::Clear { destination } => vec![destination.as_str()],
        LegacyInstruction::IndirectScalarAccess {
            destination: Some(destination),
            ..
        } => vec![destination.as_str()],
        LegacyInstruction::CountedLoopStep { index } => vec![index.as_str()],
        LegacyInstruction::ElementStore {
            index: crate::indexed_collection::IndexSpelling::PostIncrement(counter),
            ..
        } => vec![counter.as_str()],
        LegacyInstruction::ContractCall {
            destination,
            arguments,
            contract,
            ..
        } => {
            let mut names = destination.as_deref().into_iter().collect::<Vec<_>>();
            names.extend(contract.parameters.iter().zip(arguments).filter_map(
                |(mode, argument)| {
                    matches!(mode, ResourceParameterMode::Out { .. })
                        .then_some(argument.as_deref()?)
                },
            ));
            names
        }
        _ => Vec::new(),
    };
    // A guard is about two plain names, so only a plain destination can
    // invalidate one; `*out` and `p->field` name no such value.
    names.retain(|name| crate::may_alias::place_root(name) == Some(*name));
    names
}

/// C02-P-ALIAS: the may-alias facts on entry to every node of one CFG.
///
/// A separate forward fixed point, independent of custody: which releases have
/// run and which pointer inequalities a branch proved depend only on the CFG's
/// shape and its instructions, never on the abstract state. Releases only grow
/// and guards only shrink, both over finite sets, so it terminates.
pub(super) fn alias_facts(cfg: &LegacyCfg) -> Vec<NodeAliasFacts> {
    let mut entry = vec![None::<NodeAliasFacts>; cfg.nodes.len()];
    entry[cfg.entry] = Some(NodeAliasFacts::default());
    let mut work = VecDeque::from([cfg.entry]);
    while let Some(node_id) = work.pop_front() {
        let Some(facts) = entry[node_id].clone() else {
            continue;
        };
        let node = &cfg.nodes[node_id];
        let mut out = facts;
        for name in released_names(&node.instruction) {
            out.released.insert(name.to_owned());
        }
        for name in rebound_names(&node.instruction) {
            out.distinct
                .retain(|(left, right)| left != name && right != name);
        }
        for (edge, &successor) in node.successors.iter().enumerate() {
            let mut edge_out = out.clone();
            if let LegacyInstruction::BranchOnDistinct {
                left,
                right,
                true_is_distinct,
            } = &node.instruction
            {
                let distinct = if edge == 0 {
                    *true_is_distinct
                } else {
                    !*true_is_distinct
                };
                if distinct {
                    let pair = if left <= right {
                        (left.clone(), right.clone())
                    } else {
                        (right.clone(), left.clone())
                    };
                    edge_out.distinct.insert(pair);
                }
            }
            let changed = match &mut entry[successor] {
                Some(current) => current.absorb(&edge_out),
                slot @ None => {
                    *slot = Some(edge_out);
                    true
                }
            };
            if changed {
                work.push_back(successor);
            }
        }
    }
    entry.into_iter().map(Option::unwrap_or_default).collect()
}

/// C02-P-ALIAS: the provenance discharge.
///
/// `place` is bound here to exactly one live identity **this body acquired**.
/// A body-made acquisition returns an object distinct from every object live
/// when it ran, so the projection's own identity for `place` answers the
/// question and no peer's release can reach it. A caller-held entry site is
/// deliberately *not* provenance: two formals carrying declared entry custody
/// are exactly the pair this card exists for.
pub(super) fn distinct_by_provenance(
    state: &AbstractState,
    caller_sites: &BTreeSet<usize>,
    place: &str,
) -> bool {
    let Some(pointer) = state.pointers.get(place) else {
        return false;
    };
    if pointer.unknown || pointer.may_be_null || pointer.targets.len() != 1 {
        return false;
    }
    pointer.targets.iter().all(|site| {
        !caller_sites.contains(site)
            && state.resources.get(site) == Some(&LIVE)
            && !state.unresolved_effect_sites.contains(site)
    })
}

/// ALIAS-LOCALS: whether the projection's own bindings show these two names
/// denote one object here. See [`crate::may_alias::definitely_aliased`].
///
/// Read-only against the custody state, exactly like [`distinct_by_provenance`],
/// so it cannot perturb the custody work list.
pub(super) fn definitely_aliased(state: &AbstractState, one: &str, other: &str) -> bool {
    let (Some(left), Some(right)) = (state.pointers.get(one), state.pointers.get(other)) else {
        return false;
    };
    fn denote(pointer: &AbstractPointer) -> crate::may_alias::Denotation<'_> {
        crate::may_alias::Denotation {
            targets: &pointer.targets,
            unknown: pointer.unknown,
            may_be_null: pointer.may_be_null,
        }
    }
    crate::may_alias::definitely_aliased(denote(left), denote(right))
}

/// ALIAS-LOCALS: the pointer-inequality discharge, with the definite-alias
/// exception.
///
/// A `BranchOnDistinct` edge discharges the pair only while the projection has
/// not itself decided that the two names denote one object. When it has, the
/// branch and the identity model contradict each other; refusing is strictly
/// the more refusing of the two answers, and it is the only one that does not
/// let a source-level comparison overrule the projection's own bindings.
pub(super) fn distinct_by_guard(
    state: &AbstractState,
    alias: &NodeAliasFacts,
    place: &str,
    peer: &str,
) -> bool {
    alias.proved_distinct(place, peer) && !definitely_aliased(state, place, peer)
}

/// C02-P-ALIAS: refuse each modeled read whose base may denote an object some
/// peer's release already destroyed.
///
/// Reported as the existing `UnknownAlias` — no diagnostic code is added,
/// changed or downgraded — naming the read place and the peer that compromises
/// it, in the `owner/source` composite spelling this module already uses. The
/// diagnostic poisons the path, so the exit it reaches supports no custody
/// claim, and the call's own effects still apply exactly as before: this rule
/// can turn `Safe` into `Unknown` and can never hide a leak.
pub(super) fn require_unaliased_reads(
    function: &str,
    line: usize,
    reads: &[AliasPeers],
    state: &AbstractState,
    caller_sites: &BTreeSet<usize>,
    alias: &NodeAliasFacts,
    diagnostics: &mut BTreeSet<LegacyDiagnostic>,
) {
    for read in reads {
        if distinct_by_provenance(state, caller_sites, &read.place) {
            continue;
        }
        for peer in &read.peers {
            if alias.released.contains(peer) && !distinct_by_guard(state, alias, &read.place, peer)
            {
                diagnostics.insert(LegacyDiagnostic {
                    function: function.to_owned(),
                    line,
                    kind: LegacyDiagnosticKind::UnknownAlias {
                        variable: format!("{}/{peer}", read.place),
                    },
                });
            }
        }
    }
}

impl AbstractState {
    /// Join the B08 field-link relation.
    ///
    /// The union alone would be unsound in the escaping direction: a child
    /// linked on one path only would be carried away by an escape of the
    /// aggregate on a path that never stored it, hiding a leak. Card
    /// **B08-FIELD-LINK-JOIN-SILENT-UNRESOLVED** (rule 11c): every child in
    /// the symmetric difference is recorded as a *partial* link, which the
    /// escape refuses and a member read declines. It used to be recorded as
    /// an unresolved effect that no diagnostic reported, so the exit stayed
    /// `Safe` with the child's leak withheld. The union and the partial
    /// triples both only grow, so the work list still terminates.
    pub(super) fn join_field_links(&mut self, incoming: &Self) {
        field_link_join_route::join(self, incoming);
    }
}

/// Custody the caller hands to a function's formal parameters, derived from the
/// contract in force for that function.
///
/// Parameter resources are *caller-owned*: they are seeded live at entry and
/// exempt from exit-leak reporting, because discharging them is the caller's
/// obligation, not the callee's. Their exit disposition is what the summary
/// claims about the call, and [`crate::legacy_summary`] checks that claim.
#[derive(Clone, Debug, Default)]
pub(crate) struct EntryCustody {
    /// Rule 36a. Internal checked-frame activation, never contract data.
    pub(crate) allow_self_init_frame: bool,
    pub(super) state: AbstractState,
    /// Abstract site bound to each formal parameter, when the contract gives
    /// that parameter observable custody. `None` for `Borrowed`/`Out` modes.
    pub(crate) parameter_sites: Vec<Option<usize>>,
    /// Place a formal declared `Out` writes through, as `*name`. Nothing is
    /// bound at entry — the callee *initializes* the slot — but naming it is
    /// what makes `*name = …` a modeled store instead of an unmodeled one.
    pub(crate) out_slots: Vec<Option<String>>,
    /// Whether the contract's result may be null. A body may return a
    /// possibly-null place only when its own contract says the caller has to
    /// null-check the result; otherwise the escape reports an unresolved alias.
    pub(crate) result_nullable: bool,
    /// Card **W43-RESULT-ALIAS-RETURNED**: the *name-only* sites of the
    /// borrowed formals a derived result may be derived from, keyed by site to
    /// the formal's position.
    ///
    /// Each such formal is bound to one site so that a pointer derived from it
    /// carries that site as its target, but the site is **not a resource**: it
    /// has no status, so every custody-bearing use of the formal (a release, a
    /// consumption, an escape, an `Owned` position, an aggregate owner, a
    /// bound read) fails the live-identity check it already has and refuses.
    /// The formal therefore stays exactly as borrowed as the summary claims,
    /// and nothing is exempted from leak reporting because nothing is owned.
    /// Empty for every contract whose result is not `Derived`.
    pub(crate) derived_bases: BTreeMap<usize, usize>,
    /// Card **W43-DESCRIPTOR-RESOURCE-TRACKING**: the non-heap kind of each
    /// formal site, from the contract the body is checked against. A probe
    /// contract names none, so every probed formal is heap, and a body that
    /// releases one with another kind's release refuses its own summary.
    pub(crate) site_kinds: BTreeMap<usize, ResourceKind>,
    /// Card **DETACH-RESULT-WRAPPER-COMPOSITION** (rule 33): the selection a
    /// checked selection frame's own row is analysed under. `None` for every
    /// other entry.
    pub(crate) selection: Option<selection_route::SelectionEntry>,
}

impl EntryCustody {
    /// Build the entry state a contract prescribes, or `None` when the
    /// parameter list and the contract cannot be lined up exactly.
    pub(crate) fn build(
        cfg: &LegacyCfg,
        parameters: &[Option<String>],
        contract: &ResourceFunctionContract,
    ) -> Option<Self> {
        if contract.parameters.len() != parameters.len() {
            return None;
        }
        let base = first_free_site(cfg);
        let mut custody = Self {
            allow_self_init_frame: false,
            state: AbstractState::default(),
            parameter_sites: vec![None; parameters.len()],
            out_slots: vec![None; parameters.len()],
            result_nullable: match contract.result {
                ResourceReturnMode::None => false,
                ResourceReturnMode::Owned { nullable }
                | ResourceReturnMode::Reallocated { nullable, .. }
                | ResourceReturnMode::Removed { nullable, .. } => nullable,
                // Card W43-RESULT-ALIAS-RETURNED (amending W43-LIBC-RESULT-ALIAS,
                // which answered `None` here): a derived result escapes
                // nothing, so no escape consults this flag.
                ResourceReturnMode::Derived { .. } => false,
            },
            derived_bases: BTreeMap::new(),
            site_kinds: BTreeMap::new(),
            selection: None,
        };
        if let ResourceReturnMode::Derived { parameter, .. } = contract.result {
            // The base must be a borrowed, named formal: the summary claims the
            // body performs no custody effect on it.
            if contract.parameters.get(parameter) != Some(&ResourceParameterMode::Borrowed) {
                return None;
            }
            let name = parameters.get(parameter)?.as_ref()?;
            custody.seed_derived_base(parameter, name, base.checked_add(parameter)?);
        }
        for (index, mode) in contract.parameters.iter().copied().enumerate() {
            let nullable = match mode {
                // Nothing is bound: the body may not treat the formal as a
                // resource it already owns.
                ResourceParameterMode::Borrowed => continue,
                // An `Out` formal is *written* by the callee. Nothing is bound
                // at entry either, but the slot it writes through is named so
                // the store can be modeled and observed at every exit.
                ResourceParameterMode::Out { .. } => {
                    let name = parameters[index].as_ref()?;
                    custody.out_slots[index] = Some(format!("*{name}"));
                    continue;
                }
                ResourceParameterMode::Owned { nullable }
                | ResourceParameterMode::Consumed { nullable }
                | ResourceParameterMode::Released { nullable }
                | ResourceParameterMode::Stored { nullable, .. }
                | ResourceParameterMode::Removed { nullable, .. } => nullable,
            };
            let name = parameters[index].as_ref()?;
            let site = base.checked_add(index)?;
            let kind = contract.parameter_kind(index);
            if !kind.is_heap() {
                custody.site_kinds.insert(site, kind);
            }
            custody.state.resources.insert(site, LIVE);
            custody.state.pointers.insert(
                name.clone(),
                AbstractPointer {
                    targets: BTreeSet::from([site]),
                    unknown: false,
                    may_be_null: nullable,
                    derivation: Derivation::Identity,
                },
            );
            if nullable {
                custody.state.nullable_sites.insert(site);
            }
            custody.parameter_sites[index] = Some(site);
        }
        // A Removed formal is supplied inside its owner's aggregate, not in
        // direct custody. Bind every formal first so argument order cannot
        // change the owner identity. Unsupported owner shapes fail closed.
        for (index, mode) in contract.parameters.iter().copied().enumerate() {
            if let ResourceParameterMode::Removed {
                owner_parameter, ..
            } = mode
            {
                if index == owner_parameter
                    || !matches!(
                        contract.parameters.get(owner_parameter),
                        Some(ResourceParameterMode::Owned { nullable: false })
                    )
                {
                    return None;
                }
                let child = custody.parameter_sites[index]?;
                let owner = custody
                    .parameter_sites
                    .get(owner_parameter)
                    .copied()
                    .flatten()?;
                custody.state.resources.insert(child, CONTAINED);
                custody
                    .state
                    .contained
                    .entry(owner)
                    .or_default()
                    .insert(child);
            }
        }
        Some(custody)
    }

    /// Card **W43-RESULT-ALIAS-RETURNED**: bind the borrowed formal at
    /// `index` to the name-only `site` (see [`Self::derived_bases`]).
    ///
    /// The binding may be null — a borrowed formal can be — so a null test on
    /// it keeps both edges, and no path is deleted by the seeding. The site is
    /// deliberately left out of `state.resources`.
    pub(crate) fn seed_derived_base(&mut self, index: usize, name: &str, site: usize) {
        self.state.pointers.insert(
            name.to_owned(),
            AbstractPointer {
                targets: BTreeSet::from([site]),
                unknown: false,
                may_be_null: true,
                derivation: Derivation::Identity,
            },
        );
        self.derived_bases.insert(site, index);
    }

    /// Sites the caller — not this body — is responsible for discharging at a
    /// given exit: every formal the contract bound, plus whatever each out
    /// slot holds there, because an `Out` contract hands that resource back.
    ///
    /// The out part is read from the exit state rather than fixed up front, so
    /// a resource overwritten in the slot (`*out = p; *out = q;`) loses its
    /// exemption and is reported as the leak it is.
    pub(super) fn exempt_sites(&self, state: &AbstractState) -> BTreeSet<usize> {
        let mut sites = self
            .parameter_sites
            .iter()
            .flatten()
            .copied()
            .collect::<BTreeSet<_>>();
        for place in self.out_slots.iter().flatten() {
            if let Some(pointer) = state.pointers.get(place) {
                sites.extend(pointer.targets.iter().copied());
            }
        }
        sites
    }

    /// Places a store may legitimately target in this analysis.
    pub(super) fn sanctioned_places(&self) -> BTreeSet<&str> {
        self.out_slots
            .iter()
            .flatten()
            .map(String::as_str)
            .collect()
    }
}

/// The first abstract site that no acquisition in `cfg` already uses.
pub(crate) fn first_free_site(cfg: &LegacyCfg) -> usize {
    cfg.nodes
        .iter()
        .flat_map(|node| instruction_produced_sites(&node.instruction))
        .max()
        .map_or(0, |site| site.saturating_add(1))
}

/// What one reachable exit of a function does to the custody it was given.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ExitObservation {
    /// Disposition of each bound formal parameter site at this exit.
    pub(crate) parameters: Vec<Option<SiteDisposition>>,
    pub(crate) returned: ReturnedValue,
    /// What each declared out slot holds at this exit.
    pub(crate) out_slots: Vec<Option<OutSlotValue>>,
    /// Containment whose *owner* is a site the caller holds — a formal, or a
    /// resource handed back through an out slot. A summary may only claim such
    /// an attachment through an explicit `Stored` mode; anything else here is
    /// a transfer invisible at the call site, and refutes the summary.
    pub(crate) caller_containment: BTreeMap<usize, BTreeSet<usize>>,
    /// Reallocation replacements that this exit left unresolved.
    pub(crate) pending_replacements: BTreeMap<usize, BTreeSet<usize>>,
    /// Card **W43-RESULT-ALIAS-RETURNED**: whether some custody effect reached
    /// a name-only derived base ([`EntryCustody::derived_bases`]) on the way to
    /// this exit — the site acquired a status (a nullable release, an escape, a
    /// cascade through an aggregate that held it) or an unresolved effect. A
    /// derived-result summary claims its base is borrowed, so it requires this
    /// to be `false` at every exit. Always `false` when there is no such base.
    pub(crate) derived_base_touched: bool,
    /// Card **DETACH-RESULT-WRAPPER-COMPOSITION** (rule 33): the selected
    /// child's disposition at this exit, under a selection entry only.
    pub(crate) selected: Option<SiteDisposition>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SiteDisposition {
    Live,
    Released,
    Escaped,
    Contained,
    /// The site left the state on this path (a refuted nullable acquisition).
    Absent,
    /// Several dispositions joined; no single custody claim describes it.
    Mixed,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ReturnedValue {
    /// The exit returns no modeled resource place (including a null constant).
    None,
    Site {
        site: usize,
        nullable: bool,
        disposition: SiteDisposition,
        /// Card W43-DESCRIPTOR-RESOURCE-TRACKING: the returned identity's kind.
        kind: ResourceKind,
    },
    /// The returned place denotes more than one site, or an unknown one.
    Ambiguous,
    /// Card **W43-RESULT-ALIAS-RETURNED**: a null pointer constant, observed
    /// only on a CFG built for derived-result validation (elsewhere such an
    /// exit is a `NoOp` and reads [`Self::None`]). Every consumer but the
    /// derived-result check treats it exactly as `None`.
    Null,
    /// Card **W43-RESULT-ALIAS-RETURNED**: a derived binding over exactly one
    /// site. Before this card it read [`Self::Ambiguous`], and every consumer
    /// but the derived-result check still treats it so.
    Derived {
        site: usize,
        extent: DerivedExtent,
        /// The binding's relative nullness (see [`Derivation::Derived`]).
        nullable: bool,
        const_dropped: bool,
    },
}

/// What an out slot holds at one exit. `Unwritten` is deliberately distinct
/// from `Null`: a slot the callee never stored to leaves the caller's variable
/// as it was, which no `Out` mode describes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum OutSlotValue {
    /// The slot was never stored to on this path.
    Unwritten,
    /// The slot definitely holds a null pointer.
    Null,
    Site {
        site: usize,
        nullable: bool,
        disposition: SiteDisposition,
        /// Card W43-DESCRIPTOR-RESOURCE-TRACKING: the delivered identity's kind.
        kind: ResourceKind,
    },
    /// The slot denotes more than one site, or an unknown one.
    Ambiguous,
}

pub(super) fn observe_out_slot(
    state: &AbstractState,
    place: &str,
    kinds: &BTreeMap<usize, ResourceKind>,
) -> OutSlotValue {
    match state.pointers.get(place) {
        None => OutSlotValue::Unwritten,
        // Card W43-LIBC-RESULT-ALIAS: a derived pointer is not the identity it
        // targets, so a slot holding one delivers no custody a summary could
        // name.
        Some(pointer)
            if pointer.unknown
                || pointer.targets.len() > 1
                || !pointer.derivation.is_identity() =>
        {
            OutSlotValue::Ambiguous
        }
        // Card W43-INDEXED-RESOURCE-COLLECTIONS: an array still owing elements
        // delivers more than the identity a summary can name.
        Some(pointer)
            if pointer
                .targets
                .first()
                .is_some_and(|site| carries_collection(state, *site)) =>
        {
            OutSlotValue::Ambiguous
        }
        Some(pointer) => match pointer.targets.first().copied() {
            None => OutSlotValue::Null,
            Some(site) => OutSlotValue::Site {
                site,
                nullable: pointer.may_be_null || state.nullable_sites.contains(&site),
                disposition: site_disposition(state, site),
                kind: kind_of(kinds, site),
            },
        },
    }
}

pub(super) fn site_disposition(state: &AbstractState, site: usize) -> SiteDisposition {
    if state.unresolved_effect_sites.contains(&site) {
        return SiteDisposition::Mixed;
    }
    match state.resources.get(&site).copied() {
        None => SiteDisposition::Absent,
        Some(LIVE) => SiteDisposition::Live,
        Some(RELEASED) => SiteDisposition::Released,
        Some(ESCAPED) => SiteDisposition::Escaped,
        Some(CONTAINED) => SiteDisposition::Contained,
        Some(_) => SiteDisposition::Mixed,
    }
}

/// Containment a caller could observe: the declared `Stored` relation, plus
/// every B08 field link. A field link whose owner the caller holds is a
/// transfer invisible at the call site, so it must reach
/// [`crate::legacy_summary`]'s observation check and refute any summary that
/// does not declare it. The store admission already refuses a caller-held
/// aggregate, so this is defence in depth rather than a reachable case.
pub(super) fn caller_visible_containment(
    state: &AbstractState,
) -> BTreeMap<usize, BTreeSet<usize>> {
    let mut containment = state.contained.clone();
    for (owner, fields) in &state.field_links {
        let children = containment.entry(*owner).or_default();
        for linked in fields.values() {
            children.extend(linked.iter().copied());
        }
    }
    containment
}

pub(super) fn observe_exit(
    node: &LegacyCfgNode,
    state: &AbstractState,
    entry: &EntryCustody,
    // Card W43-INDEX-TRAVERSAL: the declared scalars, whose target-free
    // binding returns no custody (see the `EscapeIfOwned` transfer arm).
    scalars: &BTreeSet<String>,
    kinds: &BTreeMap<usize, ResourceKind>,
) -> ExitObservation {
    let returned = match &node.instruction {
        LegacyInstruction::ExactFormalReturn { source, parameter } => {
            match exact_formal_return::classify(state, source, *parameter, &entry.derived_bases) {
                exact_formal_return::Flow::Null => ReturnedValue::Null,
                exact_formal_return::Flow::Exact(site) => ReturnedValue::Derived {
                    site,
                    extent: DerivedExtent::Exact,
                    nullable: false,
                    const_dropped: false,
                },
                exact_formal_return::Flow::Refuse => ReturnedValue::Ambiguous,
            }
        }

        LegacyInstruction::EscapeIfOwned { source } if source == return_call::NULL_RETURN_PLACE => {
            ReturnedValue::Null
        }
        LegacyInstruction::EscapeIfOwned { source } | LegacyInstruction::Escape { source } => {
            match state.pointers.get(source) {
                None => ReturnedValue::None,
                Some(pointer) if scalars.contains(source) && pointer.targets.is_empty() => {
                    ReturnedValue::None
                }
                // Card W43-RESULT-ALIAS-RETURNED: a derived binding over one
                // resolved site is reported as such; only the derived-result
                // check reads it, and everything else treats it as ambiguous.
                Some(AbstractPointer {
                    targets,
                    unknown: false,
                    derivation:
                        Derivation::Derived {
                            extent,
                            const_dropped,
                            nullable,
                            // Card W43-INDEX-TRAVERSAL (integration): a pointer that
                            // may sit at its object's start (`Any`) is not reported as
                            // derived, so no caller can be accused of releasing it.
                            stepped: Stepped::No | Stepped::Forward,
                        },
                    ..
                }) if targets.len() == 1 => ReturnedValue::Derived {
                    site: *targets.first().expect("one target"),
                    extent: *extent,
                    nullable: *nullable,
                    const_dropped: *const_dropped,
                },
                // Card W43-LIBC-RESULT-ALIAS: nor does a returned derived
                // pointer (whose escape the transfer already refused).
                Some(pointer)
                    if pointer.unknown
                        || pointer.targets.len() > 1
                        || !pointer.derivation.is_identity() =>
                {
                    ReturnedValue::Ambiguous
                }
                Some(pointer) => match pointer.targets.first().copied() {
                    // A definitely-null place returns no resource.
                    None => ReturnedValue::None,
                    Some(site) => ReturnedValue::Site {
                        site,
                        nullable: pointer.may_be_null || state.nullable_sites.contains(&site),
                        disposition: site_disposition(state, site),
                        kind: kind_of(kinds, site),
                    },
                },
            }
        }
        _ => ReturnedValue::None,
    };
    // Card W43-INDEXED-RESOURCE-COLLECTIONS: a returned array still owing
    // elements is more than the identity a summary can name.
    let returned = match returned {
        ReturnedValue::Site { site, .. } if carries_collection(state, site) => {
            ReturnedValue::Ambiguous
        }
        other => other,
    };
    let exempt = entry.exempt_sites(state);
    ExitObservation {
        parameters: entry
            .parameter_sites
            .iter()
            .map(|site| site.map(|site| site_disposition(state, site)))
            .collect(),
        returned,
        out_slots: entry
            .out_slots
            .iter()
            .map(|place| {
                place
                    .as_deref()
                    .map(|place| observe_out_slot(state, place, kinds))
            })
            .collect(),
        caller_containment: caller_visible_containment(state)
            .iter()
            .filter(|(owner, children)| exempt.contains(owner) && !children.is_empty())
            .map(|(owner, children)| (*owner, children.clone()))
            .collect(),
        pending_replacements: state.conditional_replacements.clone(),
        derived_base_touched: entry.derived_bases.keys().any(|site| {
            state.resources.contains_key(site) || state.unresolved_effect_sites.contains(site)
        }),
        selected: entry
            .selection
            .as_ref()
            .map(|selection| site_disposition(state, selection.phantom)),
    }
}

/// `ALIAS-LOCALS-BORROW-LIVENESS`: refuse every `Borrowed` actual the
/// projection's own bindings show is a **definitely released** identity.
///
/// Card ALIAS-LOCALS recorded that a `Borrowed` position had no implemented
/// precondition at all, so a body could hand a callee an identity this
/// projection records `RELEASED` and be certified `Safe`. The decision is
/// [`crate::borrow_liveness::borrowed_actual_liveness`]; this function only
/// locates the positions and publishes the refusal.
///
/// Three deliberate bounds, each recorded in
/// `docs/reports/remaining-borrow-liveness/README.md`:
///
/// * **Settled states only.** The pass runs after the work list is stable and
///   reads `incoming[node]` — the state *entering* the call, before any of this
///   call's own parameter effects. A precondition is evaluated in the
///   pre-state, so `f(p /*released*/, p /*borrowed*/)` is not refused here; the
///   release side already has its own diagnostics.
/// * **Plain names only.** A `BASE->FIELD` bound read and a member-address
///   actual are governed by O04-READ-BOUND / O04-B09B10, whose admissions
///   already require the identity to be `LIVE` (`bound_read_resolves`,
///   `resolved_aggregate_owner`). Restating it here would double-report against
///   another card's channel without adding a refusal.
/// * **Read-only.** Nothing is written to any `AbstractState`, so the custody
///   fixed point, the leak inventory and every per-exit residual are unchanged
///   by construction (`C02-P-ALIAS-LEAK-ORDER` undisturbed). The refusal
///   therefore does not set `unmodeled`, exactly as ALIAS-LOCALS's obligations
///   do not — the same recorded `ALIAS-LOCALS-EXIT-CUSTODY` boundary, since a
///   read changes no site's disposition.
///
/// The published claim is the existing `UnknownAlias` naming the actual. No
/// diagnostic code is added, changed or downgraded, and the rule can only turn
/// `Safe` into `Unknown`.
/// Card **SCALAR-ROUTE-EXIT-EDGE**: report each loop header whose exit edge the
/// value route called infeasible and the work list retained anyway.
///
/// Read from the **settled** incoming states, exactly as
/// [`require_live_borrowed_actuals`] is and for the same reason: the arm is
/// reached on intermediate iterates of a header whose fixed point later widens
/// the deciding binding to `unknown` (a body that writes the scalar takes
/// `ForgetScalar`, and `AbstractState::join`'s `(Some, None)` arm then makes it
/// `unknown`), so a diagnostic emitted inside the work list would depend on the
/// work-list schedule — the hazard constraint 6 records.
///
/// A `BranchOnNull` node's transfer function is empty and it is never a release
/// site, so its settled incoming state *is* the state the edge was refined
/// from; the probe below therefore asks exactly the question the work list
/// asked, on the state the fixed point settled at.
pub(super) fn report_retained_loop_exit_edges(
    cfg: &LegacyCfg,
    incoming: &[Option<AbstractState>],
    kinds: &BTreeMap<usize, ResourceKind>,
    diagnostics: &mut BTreeSet<LegacyDiagnostic>,
) {
    for (node_id, node) in cfg.nodes.iter().enumerate() {
        // An unreached header supports no claim in either direction.
        let (Some(edge), Some(state)) = (node.loop_exit_edge, incoming[node_id].as_ref()) else {
            continue;
        };
        let mut probe = state.clone();
        if matches!(
            refine_edge(&node.instruction, edge, &mut probe, kinds),
            Some((_, PointerRefinement::Infeasible))
        ) {
            diagnostics.insert(LegacyDiagnostic {
                function: cfg.function.clone(),
                line: node.line,
                kind: LegacyDiagnosticKind::Unsupported {
                    description: LOOP_EXIT_EDGE_RETAINED.to_owned(),
                },
            });
        }
    }
}

/// Card W43-LIBC-RESULT-ALIAS: the derived-pointer violation candidates, read
/// from the settled incoming states.
///
/// A *use* is an actual at any declared position of a contracted call (its
/// admitted nested calls included) other than an `Out` slot, which the call
/// writes rather than reads, and a released or returned place. Only a binding
/// that is derived on every reaching path and resolves to exactly one identity
/// is examined; every other binding supports no claim and keeps whatever
/// answer the transfer function already gave it.
///
/// * The identity carries the `RELEASED` bit: the base was released on some
///   path reaching this use, so the derived value is indeterminate there —
///   `UseAfterRelease`. This is the `DoubleRelease` reading of the same bit.
/// * Otherwise, an interior pointer handed to a release — `InvalidRelease`.
///
/// Nothing here changes a state; it only reports.
pub(super) fn report_derived_pointer_violations(
    cfg: &LegacyCfg,
    incoming: &[Option<AbstractState>],
    acquisition_lines: &BTreeMap<usize, usize>,
    owners: &CollectionOwners<'_>,
    diagnostics: &mut BTreeSet<LegacyDiagnostic>,
) {
    for (node_id, node) in cfg.nodes.iter().enumerate() {
        let Some(state) = incoming[node_id].as_ref() else {
            continue;
        };
        let mut uses: Vec<(ResourceParameterMode, &str)> = Vec::new();
        match element_call_view(node, state, owners) {
            LegacyInstruction::ContractCall {
                arguments,
                contract,
                nested,
                ..
            } => {
                let calls = std::iter::once((arguments, contract)).chain(
                    nested
                        .iter()
                        .map(|nested| (&nested.arguments, &nested.contract)),
                );
                for (arguments, contract) in calls {
                    for (mode, argument) in contract.parameters.iter().zip(arguments) {
                        if matches!(mode, ResourceParameterMode::Out { .. }) {
                            continue;
                        }
                        if let Some(place) = argument.as_deref() {
                            uses.push((*mode, place));
                        }
                    }
                }
            }
            LegacyInstruction::Release { source } => {
                uses.push((ResourceParameterMode::Released { nullable: false }, source));
            }
            LegacyInstruction::Escape { source } | LegacyInstruction::EscapeIfOwned { source } => {
                uses.push((ResourceParameterMode::Consumed { nullable: true }, source));
            }
            _ => continue,
        }
        for (mode, place) in uses {
            let Some(pointer) = state.pointers.get(place) else {
                continue;
            };
            if !matches!(pointer.derivation, Derivation::Derived { .. })
                || pointer.unknown
                || pointer.targets.len() != 1
            {
                continue;
            }
            let Some(site) = pointer.targets.first().copied() else {
                continue;
            };
            let Some(status) = state.resources.get(&site).copied() else {
                continue;
            };
            let kind = if status & RELEASED != 0
                && matches!(
                    pointer.derivation,
                    Derivation::Derived {
                        extent: DerivedExtent::Borrowed,
                        ..
                    }
                ) {
                // Card SLOT-MOVE-AND-INTERIOR-BORROW (rule 12y): a borrow its
                // parent's release ended is refused, never accused.
                LegacyDiagnosticKind::UnknownAlias {
                    variable: place.to_owned(),
                }
            } else if status & RELEASED != 0 {
                LegacyDiagnosticKind::UseAfterRelease {
                    acquisition_line: acquisition_line(acquisition_lines, site),
                }
            } else if derived_release_is_accusable(mode, pointer) {
                LegacyDiagnosticKind::InvalidRelease {
                    acquisition_line: acquisition_line(acquisition_lines, site),
                }
            } else {
                continue;
            };
            diagnostics.insert(LegacyDiagnostic {
                function: cfg.function.clone(),
                line: node.line,
                kind,
            });
        }
    }
}

/// Card **W43-INDIRECT-ACCESS-GENERAL**: a typed scalar access whose base is
/// settled on exactly one identity that was released on some path reaching
/// it is a `use_after_release` violation candidate — the reading
/// [`report_derived_pointer_violations`] and `DoubleRelease` give the same
/// bit. Read from the **settled** states (constraint 6), and never through a
/// site under an unresolved effect, whose fate the projection declined to
/// commit to. It only reports; the transfer has already refused the access.
pub(super) fn report_indirect_scalar_violations(
    cfg: &LegacyCfg,
    incoming: &[Option<AbstractState>],
    acquisition_lines: &BTreeMap<usize, usize>,
    diagnostics: &mut BTreeSet<LegacyDiagnostic>,
) {
    for (node_id, node) in cfg.nodes.iter().enumerate() {
        let Some(state) = incoming[node_id].as_ref() else {
            continue;
        };
        let LegacyInstruction::IndirectScalarAccess { bases, .. } = &node.instruction else {
            continue;
        };
        for base in bases {
            let Some(pointer) = state.pointers.get(base) else {
                continue;
            };
            if pointer.unknown || pointer.targets.len() != 1 {
                continue;
            }
            let Some(site) = pointer.targets.first().copied() else {
                continue;
            };
            if state.unresolved_effect_sites.contains(&site) {
                continue;
            }
            let Some(status) = state.resources.get(&site).copied() else {
                continue;
            };
            if status & RELEASED != 0 {
                // Card SLOT-MOVE-AND-INTERIOR-BORROW (rule 12y): through a
                // borrow, refused, never accused.
                let kind = if matches!(
                    pointer.derivation,
                    Derivation::Derived {
                        extent: DerivedExtent::Borrowed,
                        ..
                    }
                ) {
                    LegacyDiagnosticKind::UnknownAlias {
                        variable: base.clone(),
                    }
                } else {
                    LegacyDiagnosticKind::UseAfterRelease {
                        acquisition_line: acquisition_line(acquisition_lines, site),
                    }
                };
                diagnostics.insert(LegacyDiagnostic {
                    function: cfg.function.clone(),
                    line: node.line,
                    kind,
                });
            }
        }
    }
}

pub(super) fn require_live_borrowed_actuals(
    cfg: &LegacyCfg,
    incoming: &[Option<AbstractState>],
    owners: &CollectionOwners<'_>,
    diagnostics: &mut BTreeSet<LegacyDiagnostic>,
) {
    for (node_id, node) in cfg.nodes.iter().enumerate() {
        // An unreached node supports no claim in either direction.
        let Some(state) = incoming[node_id].as_ref() else {
            continue;
        };
        let LegacyInstruction::ContractCall {
            arguments,
            contract,
            nested,
            ..
        } = element_call_view(node, state, owners)
        else {
            continue;
        };
        // Card CONSTRUCTOR-RESIDUALS, a fail-closed repair of a **pre-existing**
        // hole this card's own subject made reachable, and measured at the base
        // tree before any of this card's rules existed.
        //
        // Card O04-TRACE's admitted nested call carries its own modeled
        // actuals, and card BORROW-LIVENESS's precondition only ever looked at
        // the enclosing `ContractCall`'s. So
        //
        // ```c
        // s_close(h); s_note(s_label(h));      /* base: Safe, no diagnostic  */
        // s_close(h); s_label(h);              /* base: Unknown, UnknownAlias */
        // ```
        //
        // answered differently for the same released identity at the same
        // declared `Borrowed` mode, purely because one of them was nested. The
        // nested call's declared effects are the identity on the abstract state
        // (premises N6/N7), which is exactly why its *reads* need the same
        // precondition and get no other channel to raise it: `transfer` applies
        // its effects and produces nothing.
        //
        // Both are examined against the **same settled pre-state**, which is
        // sound in the ordering sense for the same reason O04-TRACE composes
        // them at all: an admitted nested call changes nothing, so the state
        // entering the nested call and the state entering the outer call are
        // the same state.
        let calls = std::iter::once((arguments, contract)).chain(
            nested
                .iter()
                .map(|nested| (&nested.arguments, &nested.contract)),
        );
        for (arguments, contract) in calls {
            // Card BORROW-RESIDUALS (`BORROW-LIVENESS-PRESTATE`): what this call's
            // own declared effects destroy, resolved once against the same settled
            // pre-state. Only `Released` destroys, exactly as only the `RELEASED`
            // bit does in `borrow_liveness::custody_definitely_ended`; `Consumed`,
            // `Stored` and `Removed` move or detach custody, and `Out` writes a
            // slot.
            let destroying = contract
                .parameters
                .iter()
                .enumerate()
                .map(|(index, mode)| {
                    matches!(mode, ResourceParameterMode::Released { .. })
                        .then(|| arguments.get(index).and_then(Option::as_deref))
                        .flatten()
                        .and_then(|place| state.pointers.get(place))
                        .map(denote_pointer)
                })
                .collect::<Vec<_>>();
            for (index, mode) in contract.parameters.iter().enumerate() {
                if !matches!(mode, ResourceParameterMode::Borrowed) {
                    continue;
                }
                let Some(source) = arguments.get(index).and_then(Option::as_deref) else {
                    continue;
                };
                let denotation = state.pointers.get(source).map(denote_pointer);
                // The path rule, on plain names only: a composite place carries
                // O04-READ-BOUND's and O04-B09B10's strictly stronger `LIVE`
                // requirement in this same pre-state (`BORROW-LIVENESS-PLACES`).
                let path_refuses = crate::may_alias::place_root(source) == Some(source)
                    && crate::borrow_liveness::borrowed_actual_liveness(
                        crate::borrow_liveness::BorrowSite {
                            denotation,
                            resources: &state.resources,
                            unresolved: &state.unresolved_effect_sites,
                        },
                    ) == crate::borrow_liveness::BorrowLiveness::DefinitelyNotLive;
                // The same-call rule, on every place the projection settled: no
                // other channel asks this call's own effects about its own reads,
                // so there is nothing here to duplicate.
                let call_refuses = crate::borrow_liveness::borrow_destroyed_by_this_call(
                    crate::borrow_liveness::SameCallSite {
                        borrowed: denotation,
                        destroying: &destroying,
                    },
                ) == crate::borrow_liveness::SameCallBorrow::DestroyedByThisCall;
                if path_refuses || call_refuses {
                    diagnostics.insert(LegacyDiagnostic {
                        function: cfg.function.clone(),
                        line: node.line,
                        kind: LegacyDiagnosticKind::UnknownAlias {
                            variable: source.to_owned(),
                        },
                    });
                }
            }
        }
    }
}

/// Card **W43-INDEXED-RESOURCE-COLLECTIONS**: the call a settled-state pass
/// must examine at `node` — an element store's inner call when its array
/// resolved (so the call ran), and otherwise the node's own instruction.
pub(super) fn element_call_view<'a>(
    node: &'a LegacyCfgNode,
    state: &AbstractState,
    owners: &CollectionOwners<'_>,
) -> &'a LegacyInstruction {
    match &node.instruction {
        LegacyInstruction::ElementStore {
            base,
            call: Some(call),
            ..
        } if owners.owner(state, base).is_some() => call,
        // Card W45-SUB-OBJECT-STORES: a stored call runs once its bases pass.
        LegacyInstruction::SubObjectStore {
            value: sub_object_store::StoredValue::Call(call),
            ..
        } if sub_object_store::call_runs(&node.instruction, state) => call,
        other => other,
    }
}

/// One tracked pointer's binding, as [`crate::may_alias::Denotation`] reads it.
///
/// Borrowed rather than copied, so that neither of the two read-only decisions
/// above can reach a mutable projection state.
pub(super) fn denote_pointer(pointer: &AbstractPointer) -> crate::may_alias::Denotation<'_> {
    crate::may_alias::Denotation {
        targets: &pointer.targets,
        unknown: pointer.unknown,
        may_be_null: pointer.may_be_null,
    }
}

pub(super) fn classify_diagnostics(diagnostics: &[LegacyDiagnostic]) -> AnalysisStatus {
    // Card W45-USER-CONTRACT-PLUGINS: the fold of each diagnostic's verdict.
    verdict::classify(diagnostics)
}

/// Card **W43-INDEXED-RESOURCE-COLLECTIONS**: what decides a collection's
/// owner: `resolved_aggregate_owner` (one known, non-null, live identity this
/// body acquired, not a caller's) of **heap** kind. A stream, directory or
/// descriptor identity is never a collection (integration of
/// W43-DESCRIPTOR-RESOURCE-TRACKING), so an element operation over one keeps
/// its fallback refusal.
pub(super) struct CollectionOwners<'a> {
    pub(super) caller_sites: &'a BTreeSet<usize>,
    pub(super) kinds: &'a BTreeMap<usize, ResourceKind>,
}

impl CollectionOwners<'_> {
    pub(super) fn owner(&self, state: &AbstractState, base: &str) -> Option<usize> {
        resolved_aggregate_owner(state, base, self.caller_sites)
            .filter(|site| kind_of(self.kinds, *site).is_heap())
    }
}

impl TransferContext<'_> {
    pub(super) fn collection_owners(&self) -> CollectionOwners<'_> {
        CollectionOwners {
            caller_sites: &self.caller_sites,
            kinds: &self.site_kinds,
        }
    }
}

/// The parts of the contract in force that the transfer function consults.
///
/// Both fields default to the most conservative setting, which is what the
/// contract-free analysis uses: no store through a pointer is modeled, and a
/// returned place may not be null. A contract only ever *adds* permissions
/// here, and every permission it adds is re-checked at the exits.
#[derive(Default)]
pub(super) struct TransferContext<'a> {
    pub(super) allow_self_init_frame: bool,
    /// No new link state is carried by functions without a checked call.
    pub(super) lookup_tracking: bool,
    pub(super) lookup_pointer_records: BTreeMap<String, String>,
    /// Out-parameter slots the contract in force declares. A store to any
    /// other indirect place stays unmodeled.
    pub(super) sanctioned: BTreeSet<&'a str>,
    /// Whether the contract's result mode obliges callers to null-check.
    pub(super) result_nullable: bool,
    /// Sites the *caller* holds at entry. An aggregate store never targets one
    /// of these: a transfer into a caller-owned aggregate is invisible at the
    /// call site and is refused rather than modeled (O00 blocker B08).
    pub(super) caller_sites: BTreeSet<usize>,
    /// Card **CTWIN-PUBLISH**: the destinations whose object outlives the call.
    /// Storing an identity *this body* holds into one of them is a publication
    /// the projection does not model, and is refused; see
    /// [`reject_publication`].
    pub(super) publications: PublicationFacts,
    /// Card **W43-RESULT-ALIAS-RETURNED**: the name-only sites of the formals
    /// a derived result may be derived from ([`EntryCustody::derived_bases`]).
    /// Returning a derived binding over one of them escapes nothing.
    pub(super) derived_bases: BTreeSet<usize>,
    pub(super) exact_return_bases: BTreeMap<usize, usize>,
    /// Card **W43-INDEX-TRAVERSAL**: the declared scalar and pointer names a
    /// copy from an unbound source is read against.
    pub(super) traversal: TraversalNames,
    /// Card **W43-DESCRIPTOR-RESOURCE-TRACKING**: the kind of every non-heap
    /// site this analysis can hold, the CFG's produced sites and the entry
    /// custody's formal sites together. A site absent here is heap.
    pub(super) site_kinds: BTreeMap<usize, ResourceKind>,
    /// Card **W43-INDEXED-RESOURCE-COLLECTIONS**: the scalar names this CFG's
    /// element operations and counted loops read. Empty in every function
    /// with none, so no other function's state gains a fact.
    pub(super) collection_names: BTreeSet<String>,
    /// Card **DETACH-RESULT-WRAPPER-COMPOSITION** (rule 33): the selection
    /// entry's facts; `None` in every other analysis.
    pub(super) selection: Option<selection_route::SelectionEntry>,
}

/// The kind of `site` under `kinds`: heap unless recorded otherwise.
pub(super) fn kind_of(kinds: &BTreeMap<usize, ResourceKind>, site: usize) -> ResourceKind {
    // Card CG04-LOOP-SITE-GENERATIONS: every generation of a site has its kind.
    kinds
        .get(&crate::loop_generations::base(site))
        .copied()
        .unwrap_or_default()
}

/// Card W43-DESCRIPTOR-RESOURCE-TRACKING: whether `place` is bound here to at
/// least one identity carried by an integer. Such a value is custody, not
/// data, whatever its declared C type.
pub(super) fn carries_integer_identity(
    state: &AbstractState,
    place: &str,
    kinds: &BTreeMap<usize, ResourceKind>,
) -> bool {
    state.pointers.get(place).is_some_and(|pointer| {
        pointer
            .targets
            .iter()
            .any(|site| kind_of(kinds, *site).is_integer())
    })
}

/// The scalar names a CFG's element operations and counted loops read.
pub(super) fn collection_names(cfg: &LegacyCfg) -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    for node in &cfg.nodes {
        match &node.instruction {
            LegacyInstruction::ElementStore { index, .. }
            | LegacyInstruction::ElementRelease { index, .. }
            | LegacyInstruction::BranchOnElementNull { index, .. } => {
                names.extend(index.name().map(str::to_owned));
            }
            LegacyInstruction::CountedLoopHeader { index, bound } => {
                names.insert(index.clone());
                names.extend(bound.name().map(str::to_owned));
            }
            LegacyInstruction::CountedLoopStep { index } => {
                names.insert(index.clone());
            }
            _ => {}
        }
    }
    names
}

pub(super) fn analyze_cfg(cfg: LegacyCfg) -> LegacyFunctionSummary {
    analyze_cfg_with_entry(cfg, &EntryCustody::default()).0
}

pub(crate) fn analyze_cfg_with_entry(
    cfg: LegacyCfg,
    entry_custody: &EntryCustody,
) -> (LegacyFunctionSummary, Vec<ExitObservation>) {
    // Card NULL-SAFETY-PROPERTY (rule 14n): the null premises this analysis
    // relies on are its summary's, whatever route publishes it.
    let ((mut summary, observations), premises) =
        crate::null_premise::collecting(|| analyze_cfg_with_entry_collected(cfg, entry_custody));
    summary.null_premises = premises;
    (summary, observations)
}

fn analyze_cfg_with_entry_collected(
    cfg: LegacyCfg,
    entry_custody: &EntryCustody,
) -> (LegacyFunctionSummary, Vec<ExitObservation>) {
    let context = TransferContext {
        allow_self_init_frame: entry_custody.allow_self_init_frame,
        lookup_tracking: cfg.nodes.iter().any(|node| node.lookup.is_some())
            || entry_custody.selection.is_some(),
        lookup_pointer_records: cfg
            .nodes
            .iter()
            .find_map(|node| {
                node.lookup
                    .as_ref()
                    .map(|lookup| lookup.pointer_records.clone())
            })
            .unwrap_or_default(),
        sanctioned: entry_custody.sanctioned_places(),
        result_nullable: entry_custody.result_nullable,
        // A derived base's name-only site is caller-held too: no aggregate
        // store may target it (it also fails every live-identity check).
        caller_sites: entry_custody
            .parameter_sites
            .iter()
            .flatten()
            .copied()
            .chain(entry_custody.derived_bases.keys().copied())
            // Rule 33: the selected child belongs to the caller's parent.
            .chain(entry_custody.selection.as_ref().map(|s| s.phantom))
            .collect(),
        publications: cfg.publications.clone(),
        collection_names: collection_names(&cfg),
        derived_bases: entry_custody.derived_bases.keys().copied().collect(),
        exact_return_bases: entry_custody.derived_bases.clone(),
        selection: entry_custody.selection.clone(),
        traversal: cfg.traversal.clone(),
        site_kinds: descriptor_custody::site_kinds(cfg.nodes.iter().map(|node| &node.instruction))
            .into_iter()
            .chain(entry_custody.site_kinds.clone())
            .collect(),
    };
    // C02-P-ALIAS: computed once, before the custody work list starts, and only
    // read below. See [`alias_facts`].
    let alias = alias_facts(&cfg);
    let joined = joined_provenance::derive(&cfg);
    // Card SUP07-REFUSED-TEST-ACCUSATION (rule 12e): provenance over the tie
    // names, derived only where they differ from the conditional names.
    let tie = refused_test::tie_facts(&cfg);
    let mut incoming = vec![None::<AbstractState>; cfg.nodes.len()];
    incoming[cfg.entry] = Some(entry_custody.state.clone());
    let mut work = VecDeque::from([cfg.entry]);
    let mut diagnostics = BTreeSet::new();
    let mut steps = 0usize;
    let acquisition_lines = cfg
        .nodes
        .iter()
        .flat_map(|node| {
            instruction_produced_sites(&node.instruction)
                .into_iter()
                .map(|site| (site, node.line))
        })
        .collect::<BTreeMap<_, _>>();

    while let Some(node_id) = work.pop_front() {
        steps += 1;
        let Some(state) = incoming[node_id].clone() else {
            continue;
        };
        let node = &cfg.nodes[node_id];
        let mut local = BTreeSet::new();
        // Card CG04-LOOP-SITE-GENERATIONS: a re-acquisition shifts its site's
        // generations first, so every read below and the transfer see the
        // earlier generation under its own identity.
        let (state, shifted) =
            loop_generations_route::shift(&cfg.function, node, state, &mut local);
        let conditional_release = joined_provenance::release_site(node, &state, &joined[node_id]);
        // Card W44-LEAK-PIN-AUDIT: the live identities this statement hands
        // on, read from the state it runs in (`refused_operands`).
        let handed = refused_operands::read_sites(&state, &cfg.publications.line_reads, node);
        let mut output = transfer(
            &cfg.function,
            node,
            state,
            &acquisition_lines,
            &context,
            &alias[node_id],
            &mut local,
        );
        // A path that reached a refusal supports no per-exit custody claim.
        // Recorded before the local diagnostics are merged, so the flag tracks
        // *this* node rather than anything reported elsewhere in the function.
        joined_provenance::finish_release(conditional_release, &mut output, &local);
        refused_operands::apply(&mut output, &handed, &local);
        loop_generations_route::settle_refused(&shifted, &mut output, &local);
        // Card B08-FIELD-LINK-JOIN-SILENT-UNRESOLVED: partial triples stay a
        // subset of the links.
        field_link_join_route::prune(&mut output);
        output.unmodeled |= local.iter().any(is_path_poisoning);
        diagnostics.append(&mut local);
        if node.is_exit {
            let exempt = entry_custody.exempt_sites(&output);
            report_leaks(
                &cfg.function,
                node.line,
                &output,
                &acquisition_lines,
                &exempt,
                &mut diagnostics,
            );
            // Card CALLEE-RETENTION-CLASS1 (rule 12t).
            retained_by_route::report_exit(&cfg.function, node.line, &output, &mut diagnostics);
            // Card MK-EVENT-DESCRIPTOR-QUANT (rule 38).
            super::descriptor_cells_route::report_exit(
                &cfg.function,
                node.line,
                &output,
                &mut diagnostics,
            );
            // Card REFCOUNT-RESOURCE-KIND (rule 13b): what each family owes.
            reference_route::report_exit(
                &cfg.function,
                node.line,
                &output,
                &acquisition_lines,
                &mut diagnostics,
            );
        }
        for (edge, &successor) in node.successors.iter().enumerate() {
            let mut edge_output = output.clone();
            // Card MK-EVENT-CELL-CROSS-FUNCTION (rule 38c): a guard test's
            // edge introduces what the protocol says on it.
            if let Some(step) = cfg.cell_tests.get(&node_id) {
                super::descriptor_cells_route::refine_protocol(step, edge, &mut edge_output);
            }
            if let Some((source, refinement)) = refine_edge(
                &node.instruction,
                edge,
                &mut edge_output,
                &context.site_kinds,
            ) {
                match refinement {
                    // Card **SCALAR-ROUTE-EXIT-EDGE**. A loop header's exit
                    // edge is never removed, whatever the value route decides.
                    //
                    // `Infeasible` elsewhere is ordinary path sensitivity: an
                    // `if`'s two arms rejoin at the same continuation, so
                    // pruning one orphans nothing. A *loop header*'s two
                    // successors do not rejoin — everything after the loop
                    // hangs off this one edge — so deleting it leaves the exit
                    // node with no incoming state at all, `report_leaks` never
                    // runs there, and `classify_diagnostics` can return `Safe`
                    // for a body that leaks, double-releases or releases after
                    // escape *after* the loop. That is the direction invariant
                    // 1 forbids, and it was measured: see
                    // `crates/qcore-c-gateway/tests/constant_do_while.rs`.
                    //
                    // The deletion would be exact only if the loop really
                    // cannot terminate, and this route cannot know that. Its
                    // must-fact is a *modelled value*, not a token, so it rests
                    // on two premises the analyzer does not verify: that every
                    // write the body performs to the deciding name reaches the
                    // header as a forget, an opaque value or a poison
                    // (constraint 10a's `Sparing`, in a role 10a does not
                    // record), and that the object is not modified
                    // asynchronously — which is **false for a `volatile`
                    // object**, where C11 §5.1.2.3 lets `while (!flag) { }`
                    // terminate. Constraint 5a already forbids deciding
                    // control flow from a computed value; this is that rule
                    // applied to the one place that still did.
                    //
                    // The edge is therefore retained carrying the *unrefined*
                    // state, and the header reports `Unsupported`, so the
                    // function is `Unknown` rather than either a false `Safe`
                    // or a fabricated `Unsafe` about a path the model says it
                    // does not have. The sibling *back* edge is deliberately
                    // untouched (`SCALAR-ROUTE-BACK-EDGE-UNDEFENDED`): removing
                    // it deletes a loop body, which is what card
                    // `C-GATEWAY-CONSTANT-DO-WHILE` proved for a token, and it
                    // needs its own measured re-pin rather than being folded in
                    // here.
                    //
                    // The refusal itself is **not** reported here. This arm is
                    // reached on intermediate iterates of a header whose fixed
                    // point later widens the deciding binding to `unknown`, so a
                    // diagnostic emitted inside the work list would depend on
                    // the work-list schedule — constraint 6's recorded hazard.
                    // It is reported from the settled states by
                    // [`report_retained_loop_exit_edges`] instead, exactly as
                    // `require_live_borrowed_actuals` reads its must-fact.
                    // Retaining the edge is itself monotone, so the fixed point
                    // stays order-independent.
                    PointerRefinement::Infeasible if node.loop_exit_edge == Some(edge) => {
                        edge_output = output.clone();
                        edge_output.unmodeled = true;
                    }
                    PointerRefinement::Infeasible => continue,
                    PointerRefinement::Unknown => {
                        edge_output.unmodeled = true;
                        diagnostics.insert(LegacyDiagnostic {
                            function: cfg.function.clone(),
                            line: node.line,
                            kind: LegacyDiagnosticKind::UnknownAlias {
                                variable: source.to_owned(),
                            },
                        });
                        // Card SUP07-REFUSED-TEST-ACCUSATION (rule 12e): on
                        // the null edge, a site provenance ties to the tested
                        // name is NULL or absent; the refusal above stays.
                        refused_test::settle_absent_edge(
                            &node.instruction,
                            edge,
                            &output,
                            tie.as_ref().map_or(&joined[node_id], |tie| &tie[node_id]),
                            &mut edge_output,
                        );
                    }
                    PointerRefinement::Refined => {}
                }
            }
            // Card W43-INDEXED-RESOURCE-COLLECTIONS: an element null test
            // refines the tested element on each edge, and a counted loop's
            // header carries its bound into the body and its exit equation
            // out of the loop.
            if !refine_collection_edge(node, edge, &mut edge_output, &context.collection_owners()) {
                continue;
            }
            let changed = match &mut incoming[successor] {
                Some(current) => current.join(&edge_output, &cfg.traversal.scalars),
                slot @ None => {
                    *slot = Some(edge_output);
                    true
                }
            };
            if changed {
                work.push_back(successor);
            }
        }
    }

    if incoming.iter().flatten().any(|state| state.widened) {
        diagnostics.insert(LegacyDiagnostic {
            function: cfg.function.clone(),
            line: 0,
            kind: LegacyDiagnosticKind::Unsupported {
                description: format!(
                    "alias set exceeded {MAX_ALIAS_TARGETS} targets and was widened"
                ),
            },
        });
    }

    // ALIAS-LOCALS-BORROW-LIVENESS: the `Borrowed` liveness precondition, read
    // from the settled incoming states for the same reason the exit pass below
    // is: it is a *must* fact, and a must fact read on an intermediate iterate
    // would depend on the work-list schedule. Read-only — see
    // [`crate::borrow_liveness`].
    require_live_borrowed_actuals(
        &cfg,
        &incoming,
        &context.collection_owners(),
        &mut diagnostics,
    );
    report_retained_loop_exit_edges(&cfg, &incoming, &context.site_kinds, &mut diagnostics);
    // Card CG04-LOOP-SITE-GENERATIONS: the re-acquisitions an earlier
    // generation reaches, from the settled states (parity only).
    loop_generations_route::record_exposure(&cfg, &incoming);
    // Card CG10-BOUND-UNTRACKED-ROOTS: every chain root the never-bound
    // premise does not cover, from the settled states (parity only).
    bound_untracked_route::record_census(&cfg, &incoming);
    // Card W43-DESCRIPTOR-RESOURCE-TRACKING: the kind-mismatched release and the
    // use-after-close candidates, from the same settled states.
    report_kind_violations(
        &cfg,
        &incoming,
        &acquisition_lines,
        &context.site_kinds,
        &mut diagnostics,
    );
    // Card W43-LIBC-RESULT-ALIAS: the two derived-pointer violation candidates,
    // read from the same settled states for the same reason — a binding's
    // derivation can still widen to `Mixed` on a later iterate.
    report_derived_pointer_violations(
        &cfg,
        &incoming,
        &acquisition_lines,
        &context.collection_owners(),
        &mut diagnostics,
    );
    // Card W43-INDIRECT-ACCESS-GENERAL: a typed scalar access through a base
    // released on some reaching path, from the same settled states.
    report_indirect_scalar_violations(&cfg, &incoming, &acquisition_lines, &mut diagnostics);
    // Card W43-INDEXED-RESOURCE-COLLECTIONS: element double releases,
    // element uses after release, overwritten live elements and element
    // operations through a released array, from the same settled states.
    report_collection_violations(
        &cfg,
        &incoming,
        &acquisition_lines,
        &context.collection_owners(),
        &mut diagnostics,
    );
    // Card W44-LEAK-PIN-AUDIT: the leak-witness census (a measurement only).
    #[cfg(feature = "indirect-census")]
    leak_witness::record(
        &cfg,
        &incoming,
        &acquisition_lines,
        &context,
        &alias,
        &joined,
        entry_custody,
        &diagnostics,
    );

    // Read the settled exit states once the work list is stable, so that a
    // summary describes the joined behavior of every reachable return rather
    // than an intermediate iterate. Diagnostics are already recorded above;
    // this pass only observes.
    let mut observations = Vec::new();
    let mut exits = Vec::new();
    let mut lifetime_effects = BTreeSet::new();
    let mut withheld = Vec::new();
    for (node_id, node) in cfg.nodes.iter().enumerate() {
        if !node.is_exit {
            continue;
        }
        let Some(state) = incoming[node_id].clone() else {
            continue;
        };
        let mut ignored = BTreeSet::new();
        // Card CG04-LOOP-SITE-GENERATIONS: as in the work list.
        let (state, shifted) =
            loop_generations_route::shift(&cfg.function, node, state, &mut ignored);
        let conditional_release = joined_provenance::release_site(node, &state, &joined[node_id]);
        // Card W44-LEAK-PIN-AUDIT: the refused hand-on rule, as in the work list.
        let handed = refused_operands::read_sites(&state, &cfg.publications.line_reads, node);
        let mut output = transfer(
            &cfg.function,
            node,
            state,
            &acquisition_lines,
            &context,
            &alias[node_id],
            &mut ignored,
        );
        joined_provenance::finish_release(conditional_release, &mut output, &ignored);
        refused_operands::apply(&mut output, &handed, &ignored);
        loop_generations_route::settle_refused(&shifted, &mut output, &ignored);
        output.unmodeled |= ignored.iter().any(is_path_poisoning);
        // Card W43-INDEXED-RESOURCE-COLLECTIONS: an element obligation left at
        // this settled exit.
        report_collection_exit(
            &cfg.function,
            node.line,
            &output,
            &entry_custody.exempt_sites(&output),
            &acquisition_lines,
            &mut diagnostics,
        );
        for record in &output.retained {
            if let Some(proof) = output.forwarded_lifetimes.get(record) {
                let mut proof = proof.clone();
                proof.registration.poisoned |= output.poisoned.contains(record);
                lifetime_effects.insert(proof);
            }
        }
        observations.push(observe_exit(
            node,
            &output,
            entry_custody,
            &context.traversal.scalars,
            &context.site_kinds,
        ));
        exits.push(observe_residual(
            node_id,
            node,
            &output,
            &acquisition_lines,
            entry_custody,
        ));
        withheld.push((
            node.line,
            loop_generations_route::withheld(&output, &entry_custody.exempt_sites(&output)),
        ));
        // Card B08-FIELD-LINK-JOIN-SILENT-UNRESOLVED: the floor under every
        // rule, from the settled exit state.
        field_link_join_route::exit_backstop(
            &cfg.function,
            node.line,
            &output,
            &entry_custody.exempt_sites(&output),
            &mut diagnostics,
        );
    }

    let diagnostics = diagnostics.into_iter().collect::<Vec<_>>();
    let status = classify_diagnostics(&diagnostics);
    // Card CG04-LOOP-SITE-GENERATIONS: the census of Safe results resting on
    // a possibly-live identity whose effect is unresolved (parity only).
    loop_generations_route::record_withheld(&cfg.function, status, &withheld, &acquisition_lines);
    (
        LegacyFunctionSummary {
            function: cfg.function.clone(),
            status,
            diagnostics,
            fixed_point_steps: steps,
            cfg,
            exits,
            counted_users: Vec::new(),
            refutations: Vec::new(),
            null_premises: Vec::new(),
            lifetime_effects,
            rule_obligations: crate::arm_obligations::Carried::default(),
        },
        observations,
    )
}

/// Whether a diagnostic means the path that produced it can support no custody
/// claim. `Leak` is deliberately absent: it is reported *at* an exit by
/// [`report_leaks`], and it is exactly the residual this ledger describes
/// rather than a reason to refuse describing it.
pub(super) fn is_path_poisoning(diagnostic: &LegacyDiagnostic) -> bool {
    matches!(
        diagnostic.kind,
        LegacyDiagnosticKind::Unsupported { .. }
            | LegacyDiagnosticKind::UnknownAlias { .. }
            | LegacyDiagnosticKind::SummaryConflict { .. }
            | LegacyDiagnosticKind::DoubleRelease { .. }
            | LegacyDiagnosticKind::ReleaseAfterEscape { .. }
            | LegacyDiagnosticKind::CustodyConflict { .. }
            | LegacyDiagnosticKind::InvalidRelease { .. }
            | LegacyDiagnosticKind::UseAfterRelease { .. }
            | LegacyDiagnosticKind::KindMismatchedRelease { .. }
            | LegacyDiagnosticKind::UseAfterClose { .. }
    )
}

/// Card **W43-INDEXED-RESOURCE-COLLECTIONS**: one edge's refinement. `false`
/// when the edge is infeasible.
pub(super) fn refine_collection_edge(
    node: &LegacyCfgNode,
    edge: usize,
    state: &mut AbstractState,
    owners: &CollectionOwners<'_>,
) -> bool {
    match &node.instruction {
        LegacyInstruction::BranchOnElementNull {
            base,
            index,
            true_is_null,
            ..
        } => {
            let Some(owner) = owners.owner(state, base) else {
                return true;
            };
            let assume_null = if edge == 0 {
                *true_is_null
            } else {
                !*true_is_null
            };
            let mut feasible = true;
            with_collection(state, owner, |ranges, facts| {
                if ranges.test_null(&index.term(), assume_null, facts)
                    == crate::indexed_collection::TestOutcome::Infeasible
                {
                    feasible = false;
                }
                true
            });
            feasible
        }
        LegacyInstruction::CountedLoopHeader { index, bound } => {
            if edge == 0 {
                // The index is never negative, so a bound that is zero when
                // clamped admits no iteration: the body edge's collection
                // facts are vacuous.
                if crate::indexed_collection::le(
                    bound,
                    &crate::indexed_collection::Term::Const(0),
                    state,
                ) {
                    state.collection_vacuous = true;
                }
                state
                    .index_bounds
                    .entry(index.clone())
                    .or_default()
                    .insert(bound.clone());
            } else {
                state.index_bounds.remove(index);
                // `index = max(0, bound)` here: an index still zero means the
                // loop ran no iteration, so the bound is zero when clamped.
                if crate::indexed_collection::Scalars::is_zero(state, index)
                    && let Some(name) = bound.name()
                {
                    state.clamped_zero.insert(name.to_owned());
                }
                let owners = state.collections.keys().copied().collect::<Vec<_>>();
                for owner in owners {
                    if let Some(crate::indexed_collection::Collection::Known(mut ranges)) =
                        state.collections.get(&owner).cloned()
                    {
                        let collection = if ranges.substitute(index, bound, state) {
                            crate::indexed_collection::Collection::Known(ranges)
                        } else {
                            crate::indexed_collection::Collection::Unknown
                        };
                        state.collections.insert(owner, collection);
                    }
                }
            }
            true
        }
        _ => true,
    }
}

/// The acquisition line of the lowest site in `sites`, for a diagnostic.
pub(super) fn element_line(
    acquisition_lines: &BTreeMap<usize, usize>,
    sites: &BTreeSet<usize>,
) -> usize {
    sites
        .first()
        .map_or(0, |site| acquisition_line(acquisition_lines, *site))
}

/// Card **W43-INDEXED-RESOURCE-COLLECTIONS**: the element violation
/// candidates, read from the **settled** incoming states (constraint 6). The
/// transfer function has already refused or poisoned each of these paths; this
/// only reports. Every candidate is provisional `Unsafe` under the existing
/// policy and never a validated upstream defect.
///
/// * an element operation through an array settled on one identity released
///   on a reaching path — `use_after_release` (the array's own acquisition);
/// * releasing an element provably released already — `double_release`;
/// * testing an element provably released — `use_after_release`;
/// * storing over an element provably holding a live, non-null resource —
///   `leak` of that resource.
pub(super) fn report_collection_violations(
    cfg: &LegacyCfg,
    incoming: &[Option<AbstractState>],
    acquisition_lines: &BTreeMap<usize, usize>,
    owners: &CollectionOwners<'_>,
    diagnostics: &mut BTreeSet<LegacyDiagnostic>,
) {
    for (node_id, node) in cfg.nodes.iter().enumerate() {
        let Some(state) = incoming[node_id]
            .as_ref()
            .filter(|state| !state.collection_vacuous)
        else {
            continue;
        };
        let (base, index) = match &node.instruction {
            LegacyInstruction::ElementStore { base, index, .. }
            | LegacyInstruction::ElementRelease { base, index, .. }
            | LegacyInstruction::BranchOnElementNull { base, index, .. } => (base, index),
            _ => continue,
        };
        let mut report = |kind| {
            diagnostics.insert(LegacyDiagnostic {
                function: cfg.function.clone(),
                line: node.line,
                kind,
            });
        };
        if let Some(pointer) = state.pointers.get(base)
            && !pointer.unknown
            && pointer.targets.len() == 1
            && let Some(site) = pointer.targets.first().copied()
            && !state.unresolved_effect_sites.contains(&site)
            && state
                .resources
                .get(&site)
                .is_some_and(|status| status & RELEASED != 0)
        {
            report(LegacyDiagnosticKind::UseAfterRelease {
                acquisition_line: acquisition_line(acquisition_lines, site),
            });
            continue;
        }
        let Some(owner) = owners.owner(state, base) else {
            continue;
        };
        let crate::indexed_collection::Collection::Known(mut ranges) =
            state.collections.get(&owner).cloned().unwrap_or_default()
        else {
            continue;
        };
        match &node.instruction {
            LegacyInstruction::ElementRelease { .. } => {
                if let crate::indexed_collection::ReleaseOutcome::Again(sites) =
                    ranges.release(&index.term(), state)
                {
                    report(LegacyDiagnosticKind::DoubleRelease {
                        acquisition_line: element_line(acquisition_lines, &sites),
                    });
                }
            }
            LegacyInstruction::BranchOnElementNull { .. } => {
                if let crate::indexed_collection::TestOutcome::Released(sites) =
                    ranges.test_null(&index.term(), true, state)
                {
                    report(LegacyDiagnosticKind::UseAfterRelease {
                        acquisition_line: element_line(acquisition_lines, &sites),
                    });
                }
            }
            LegacyInstruction::ElementStore { .. } => {
                let slot = crate::indexed_collection::Slot::Null;
                if let crate::indexed_collection::StoreOutcome::Overwrote(sites) =
                    ranges.store(&index.term(), slot, state)
                {
                    report(LegacyDiagnosticKind::Leak {
                        acquisition_line: element_line(acquisition_lines, &sites),
                    });
                }
            }
            _ => {}
        }
    }
}

/// Card **W43-INDEXED-RESOURCE-COLLECTIONS**: what one settled exit leaves in
/// its collections. An array released, still held, or refuted at this exit
/// while a live segment remains leaks those elements — no name but the array
/// holds them (a `leak` candidate at the exit, named by an element's
/// acquisition). An array that escaped carries its elements with it, as a
/// member link's children escape with their aggregate. A collection whose
/// ranges are `Unknown`, or whose array escaped on some paths only, supports
/// no claim: the exit is refused unless its path is already refused.
pub(super) fn report_collection_exit(
    function: &str,
    line: usize,
    state: &AbstractState,
    // Sites the caller receives through a declared out slot: an array handed
    // back that way carries its elements with it, exactly as a returned one.
    exempt: &BTreeSet<usize>,
    acquisition_lines: &BTreeMap<usize, usize>,
    diagnostics: &mut BTreeSet<LegacyDiagnostic>,
) {
    if state.collection_vacuous {
        return;
    }
    for (owner, collection) in &state.collections {
        let status = state.resources.get(owner).copied().unwrap_or(0);
        let status = if exempt.contains(owner) && status == LIVE {
            ESCAPED
        } else {
            status
        };
        let escaped = status & ESCAPED != 0;
        let unresolved = state.unresolved_effect_sites.contains(owner);
        let undecided = match collection {
            crate::indexed_collection::Collection::Unknown => true,
            crate::indexed_collection::Collection::Known(ranges) => {
                match ranges.live_sites(state) {
                    None => false,
                    Some(_) if escaped && status == ESCAPED => false,
                    Some(_) if escaped || unresolved => true,
                    Some(sites) => {
                        diagnostics.insert(LegacyDiagnostic {
                            function: function.to_owned(),
                            line,
                            kind: LegacyDiagnosticKind::Leak {
                                acquisition_line: element_line(acquisition_lines, &sites),
                            },
                        });
                        false
                    }
                }
            }
        };
        if undecided && !state.unmodeled {
            diagnostics.insert(LegacyDiagnostic {
                function: function.to_owned(),
                line,
                kind: LegacyDiagnosticKind::Unsupported {
                    description: COLLECTION_UNDECIDED.to_owned(),
                },
            });
        }
    }
}

/// Card **W43-INDEXED-RESOURCE-COLLECTIONS**: the refusal an exit reports when
/// a collection's element obligations could not be settled on its path.
pub(crate) const COLLECTION_UNDECIDED: &str =
    "indexed collection element ranges are not established at this exit";

/// Whether any collection at this state keeps a custody claim from being made:
/// an `Unknown` one, or one that may still owe under an array that did not
/// escape exactly.
pub(super) fn collection_blocks_claim(state: &AbstractState) -> bool {
    !state.collection_vacuous
        && state.collections.iter().any(|(owner, collection)| {
            matches!(collection, crate::indexed_collection::Collection::Unknown)
                || (collection.may_owe(state) && state.resources.get(owner) != Some(&ESCAPED))
        })
}

/// Whether `site` carries a collection that may still owe, or whose ranges are
/// unknown: its exit value is then not the plain identity a summary names.
pub(super) fn carries_collection(state: &AbstractState, site: usize) -> bool {
    !state.collection_vacuous
        && state
            .collections
            .get(&site)
            .is_some_and(|collection| collection.may_owe(state))
}

/// Read one exit's custody residual out of its settled state.
pub(super) fn observe_residual(
    node_id: LegacyNodeId,
    node: &LegacyCfgNode,
    state: &AbstractState,
    acquisition_lines: &BTreeMap<usize, usize>,
    entry: &EntryCustody,
) -> LegacyExitResidual {
    let exempt = entry.exempt_sites(state);
    let sites = state
        .resources
        .keys()
        .copied()
        .filter(|site| !exempt.contains(site))
        .map(|site| LegacyResidualSite {
            site,
            acquisition_line: acquisition_line(acquisition_lines, site),
            disposition: site_disposition(state, site),
        })
        .collect::<Vec<_>>();
    let indeterminate = state.unmodeled
        || state.widened
        // Card W43-INDEXED-RESOURCE-COLLECTIONS: element obligations this
        // record's per-site dispositions cannot name.
        || collection_blocks_claim(state)
        || sites
            .iter()
            .any(|site| site.disposition == SiteDisposition::Mixed);
    let custody = if indeterminate {
        ExitCustody::Unknown
    } else if sites
        .iter()
        .any(|site| site.disposition == SiteDisposition::Live)
    {
        ExitCustody::Residual
    } else {
        ExitCustody::Discharged
    };
    LegacyExitResidual {
        node: node_id,
        line: node.line,
        custody,
        sites,
    }
}

/// Card **CTWIN-PUBLISH**: report a store of a tracked identity into an object
/// that outlives the call, and poison the path exactly as any other unmodeled
/// construct does.
///
/// **Why refusal rather than an escape.** Modeling the store as an
/// [`LegacyInstruction::Escape`] would be fail-closed in one direction only.
/// A published-then-released body would become a `ReleaseAfterEscape`, but a
/// published-and-*not*-released body — measured `Unsafe` with a `Leak` before
/// this card — would become **`Safe`**, because the escape discharges the
/// residual. That is a promotion, and it asserts an accounting no contract
/// states: unlike a return or a declared consuming position, a store into a
/// file-scope object hands custody to no named party. Both directions are
/// therefore withdrawn: this projection can say nothing about a resource whose
/// custody left through an unmodeled publication protocol.
pub(super) fn reject_publication(
    function: &str,
    line: usize,
    place: &str,
    state: &mut AbstractState,
    diagnostics: &mut BTreeSet<LegacyDiagnostic>,
) {
    diagnostics.insert(LegacyDiagnostic {
        function: function.to_owned(),
        line,
        kind: LegacyDiagnosticKind::Unsupported {
            description: format!(
                "publication of a tracked resource into the static-storage object `{place}` is outside the resource projection"
            ),
        },
    });
    for pointer in state.pointers.values_mut() {
        pointer.unknown = true;
        pointer.may_be_null = true;
    }
}

/// Report an indirect store the contract in force does not sanction, and
/// poison the state exactly as an unmodeled construct would. The importer
/// *names* `*out` so an out-parameter contract can describe it; without such a
/// contract the store is as opaque as it was before out slots existed.
pub(super) fn reject_unsanctioned_store(
    function: &str,
    line: usize,
    place: &str,
    state: &mut AbstractState,
    diagnostics: &mut BTreeSet<LegacyDiagnostic>,
) {
    diagnostics.insert(LegacyDiagnostic {
        function: function.to_owned(),
        line,
        kind: LegacyDiagnosticKind::Unsupported {
            description: format!("store through `{place}` without an out-parameter contract"),
        },
    });
    for pointer in state.pointers.values_mut() {
        pointer.unknown = true;
        pointer.may_be_null = true;
    }
}

/// The transfer function: the pre-card transfer, plus card
/// **W43-INDEXED-RESOURCE-COLLECTIONS**'s element operations and the three
/// effects every other instruction has on collections. In a function with no
/// element operation the state carries no collection, and this is exactly
/// [`transfer_base`].
/// Card **CG25-RESOURCE-KINDS** (rule 11v): every transfer, then the status
/// witnesses: a name the instruction writes witnesses nothing any more, and an
/// in-place initialiser's status destination starts witnessing its site.
pub(super) fn transfer(
    function: &str,
    node: &LegacyCfgNode,
    state: AbstractState,
    acquisition_lines: &BTreeMap<usize, usize>,
    context: &TransferContext<'_>,
    alias: &NodeAliasFacts,
    diagnostics: &mut BTreeSet<LegacyDiagnostic>,
) -> AbstractState {
    // Card SLOT-MOVE-AND-INTERIOR-BORROW (rules 12x, 12y): reads of
    // invalidated names refuse, and mutations end the borrows they may reach.
    let lookup_result = context
        .lookup_tracking
        .then(|| lookup_route::resolve(node, &state, &context.site_kinds))
        .flatten();
    let lookup_store = context
        .lookup_tracking
        .then(|| {
            lookup_route::stored_link(
                node,
                &state,
                &context.caller_sites,
                &context.lookup_pointer_records,
            )
        })
        .flatten();
    let mut state = state;
    if context.lookup_tracking {
        lookup_route::prepare_removed(node, &mut state);
    }
    let state = slot_move_route::before(
        function,
        node,
        state,
        &context.traversal.scalars,
        diagnostics,
    );
    let mut output = transfer_dispatch(
        function,
        node,
        state,
        acquisition_lines,
        context,
        alias,
        diagnostics,
    );
    slot_move_route::after(node, &mut output);
    // Card MK-EVENT-CELL-CROSS-FUNCTION (rule 38c): a call that may reach a
    // record with a fact forwards it under its protocol, or refuses. A
    // cell-bearing call is rule 38's own.
    if !output.cell_protocols.is_empty()
        && let LegacyInstruction::ContractCall {
            arguments,
            nested,
            retention,
            ..
        } = &node.instruction
        && retention.as_ref().is_none_or(|r| r.cells.is_none())
    {
        let unread = !nested.is_empty() || arguments.iter().any(Option::is_none);
        let named = arguments.iter().flatten().cloned().collect::<Vec<_>>();
        let probe = output.clone();
        super::descriptor_cells_route::touch(
            function,
            node.line,
            |root| {
                unread
                    || named.iter().any(|name| match root {
                        super::descriptor_cells_route::ProtocolRoot::Name(formal) => name == formal,
                        super::descriptor_cells_route::ProtocolRoot::Site(site) => {
                            probe.pointers.get(name).is_none_or(|pointer| {
                                pointer.unknown || pointer.targets.contains(site)
                            })
                        }
                    })
            },
            &mut output,
            diagnostics,
        );
    }
    if context.lookup_tracking {
        lookup_route::after(node, &mut output, lookup_result, lookup_store);
    }
    // Card DETACH-RESULT-WRAPPER-COMPOSITION (rule 33).
    if let Some(selection) = &context.selection {
        selection_route::materialize(node, &mut output, selection);
    }
    in_place_custody::forget_witnesses(&node.instruction, &mut output, writes_scalar);
    in_place_custody::record_witness(&node.instruction, &mut output);
    // Card MK-EVENT-DESCRIPTOR-QUANT (rule 38): the call's result witnesses
    // its creations' class until the name is written.
    super::descriptor_cells_route::forget_witnesses(&node.instruction, &mut output, writes_scalar);
    if let LegacyInstruction::ContractCall {
        retention: Some(retention),
        destination,
        ..
    } = &node.instruction
        && let Some(cells) = &retention.cells
    {
        super::descriptor_cells_route::record_witness(cells, destination.as_deref(), &mut output);
    }
    output
}

pub(super) fn transfer_dispatch(
    function: &str,
    node: &LegacyCfgNode,
    state: AbstractState,
    acquisition_lines: &BTreeMap<usize, usize>,
    context: &TransferContext<'_>,
    alias: &NodeAliasFacts,
    diagnostics: &mut BTreeSet<LegacyDiagnostic>,
) -> AbstractState {
    let base = |node: &LegacyCfgNode,
                state: AbstractState,
                diagnostics: &mut BTreeSet<LegacyDiagnostic>| {
        transfer_base(
            function,
            node,
            state,
            acquisition_lines,
            context,
            alias,
            diagnostics,
        )
    };
    match &node.instruction {
        // Card W45-SUB-OBJECT-STORES.
        LegacyInstruction::SelfInitCall {
            base: actual,
            fallback,
            ..
        } => {
            if self_link_init::admits(&state, actual, context.allow_self_init_frame) {
                state
            } else {
                base(
                    &LegacyCfgNode {
                        instruction: *fallback.clone(),
                        ..node.clone()
                    },
                    state,
                    diagnostics,
                )
            }
        }
        LegacyInstruction::TrackedSelfInit {
            base: root,
            protected_fields,
            compatible_roots,
            access,
            fallback,
            ..
        } => {
            if let Some(output) = tracked_self_init::apply(&state, root).or_else(|| {
                tracked_self_init::apply_frame(
                    &state,
                    root,
                    protected_fields,
                    compatible_roots,
                    context,
                )
            }) {
                let mut checked = BTreeSet::new();
                let read_state = base(&with_instruction(node, access), state.clone(), &mut checked);
                if checked.is_empty() && read_state == state {
                    return output;
                }
            }
            base(&with_instruction(node, fallback), state, diagnostics)
        }
        LegacyInstruction::SubObjectStore { .. } => {
            sub_object_store::transfer(function, node, state, context, alias, diagnostics, &base)
        }
        // Card CG11-POINTER-VALUED-STORES (rule 10v).
        LegacyInstruction::PointerStore { .. } => {
            pointer_store_flow::transfer(function, node, state, diagnostics, &base)
        }
        LegacyInstruction::ElementStore {
            base: array,
            index,
            call,
            fallback,
        } => {
            let Some(owner) = context.collection_owners().owner(&state, array) else {
                return base(&with_instruction(node, fallback), state, diagnostics);
            };
            let place = crate::indexed_collection::place_spelling(array, &index.term());
            let (mut state, slot) = match call {
                None => (state, Some(crate::indexed_collection::Slot::Null)),
                Some(call) => {
                    let call_node = with_instruction(node, call);
                    let mut state = state;
                    // The inner call is a call like any other: it may not be
                    // handed a collection that still owes an element.
                    refuse_collection_owner_uses(function, &call_node, &mut state, diagnostics);
                    let mut state = base(&call_node, state, diagnostics);
                    let slot = take_element_result(&mut state, &place);
                    (state, slot)
                }
            };
            let stored = slot.is_some_and(|slot| {
                with_collection(&mut state, owner, |ranges, facts| {
                    ranges.store(&index.term(), slot, facts)
                        != crate::indexed_collection::StoreOutcome::Refused
                })
            });
            if !stored {
                refuse_element(function, node.line, &place, owner, &mut state, diagnostics);
            }
            if let crate::indexed_collection::IndexSpelling::PostIncrement(counter) = index {
                step_collections(&mut state, counter);
            }
            state
        }
        LegacyInstruction::ElementRelease {
            base: array,
            index,
            nullable,
            fallback,
            ..
        } => {
            let Some(owner) = context.collection_owners().owner(&state, array) else {
                return base(&with_instruction(node, fallback), state, diagnostics);
            };
            let mut state = state;
            let place = crate::indexed_collection::place_spelling(array, &index.term());
            let mut again = false;
            let released = with_collection(&mut state, owner, |ranges, facts| {
                match ranges.release(&index.term(), facts) {
                    crate::indexed_collection::ReleaseOutcome::Released { may_be_null } => {
                        *nullable || !may_be_null
                    }
                    crate::indexed_collection::ReleaseOutcome::Null => *nullable,
                    crate::indexed_collection::ReleaseOutcome::Again(_) => {
                        again = true;
                        false
                    }
                    crate::indexed_collection::ReleaseOutcome::Refused => false,
                }
            });
            if again {
                // A `double_release` candidate, reported from the settled
                // states by `report_collection_violations`. The ranges are kept
                // as they were, so the settled state still shows the element
                // released; the path supports no further custody claim.
                state.unmodeled = true;
            } else if !released {
                refuse_element(function, node.line, &place, owner, &mut state, diagnostics);
            }
            state
        }
        LegacyInstruction::BranchOnElementNull {
            base: array,
            index,
            fallback,
            ..
        } => {
            let Some(owner) = context.collection_owners().owner(&state, array) else {
                return base(&with_instruction(node, fallback), state, diagnostics);
            };
            let mut state = state;
            let place = crate::indexed_collection::place_spelling(array, &index.term());
            let mut released = false;
            let decided = [true, false].into_iter().all(|assume_null| {
                let mut probe = state.clone();
                with_collection(&mut probe, owner, |ranges, facts| {
                    match ranges.test_null(&index.term(), assume_null, facts) {
                        crate::indexed_collection::TestOutcome::Refined
                        | crate::indexed_collection::TestOutcome::Infeasible => true,
                        crate::indexed_collection::TestOutcome::Released(_) => {
                            released = true;
                            false
                        }
                        crate::indexed_collection::TestOutcome::Refused => false,
                    }
                })
            });
            if released {
                // A `use_after_release` candidate, reported from the settled
                // states; the ranges are kept, as for a second release.
                state.unmodeled = true;
            } else if !decided {
                refuse_element(function, node.line, &place, owner, &mut state, diagnostics);
            }
            state
        }
        LegacyInstruction::CountedLoopStep { index } => {
            let mut state = state;
            state.pointers.remove(index);
            step_collections(&mut state, index);
            state
        }
        LegacyInstruction::CountedLoopHeader { .. } => state,
        _ => {
            let mut state = state;
            if !state.collections.is_empty() {
                refuse_collection_owner_uses(function, node, &mut state, diagnostics);
            }
            let mut output = base(node, state, diagnostics);
            if !context.collection_names.is_empty() {
                // The zero fact of a collection name: `NAME = 0` sets it, any
                // other write by name drops it.
                forget_written_bounds(&node.instruction, &mut output);
                if let LegacyInstruction::Clear { destination } = &node.instruction
                    && context.collection_names.contains(destination)
                {
                    output.index_zero.insert(destination.clone());
                }
                for site in instruction_produced_sites(&node.instruction) {
                    retire_reacquired_collection(&mut output, site);
                }
            }
            output
        }
    }
}

/// One node with another instruction (an element operation's inner call or
/// its fallback refusal), at the same line and with the same successors.
pub(super) fn with_instruction(
    node: &LegacyCfgNode,
    instruction: &LegacyInstruction,
) -> LegacyCfgNode {
    LegacyCfgNode {
        instruction: instruction.clone(),
        ..node.clone()
    }
}

/// The abstract sites an instruction creates, including those of an element
/// store's inner call.
pub(crate) fn instruction_produced_sites(instruction: &LegacyInstruction) -> Vec<usize> {
    match instruction {
        LegacyInstruction::Acquire { site, .. } => vec![*site],
        // Card W45-CONTAINER-ROOT-REACH: a foreign unlink's element site.
        LegacyInstruction::ContractCall {
            produced_sites,
            container,
            ..
        } => produced_sites
            .iter()
            .copied()
            .chain(container.iter().flat_map(|operands| {
                container_publication::foreign_sites(operands).collect::<Vec<_>>()
            }))
            .collect(),
        LegacyInstruction::ElementStore {
            call: Some(call), ..
        } => instruction_produced_sites(call),
        // Card W45-SUB-OBJECT-STORES: a stored call's sites.
        LegacyInstruction::SubObjectStore {
            value: sub_object_store::StoredValue::Call(call),
            ..
        } => instruction_produced_sites(call),
        _ => Vec::new(),
    }
}

/// Shift every site an instruction creates (card W43-INDEXED-RESOURCE-
/// COLLECTIONS keeps the recursion combiner's disjointness for inner calls).
pub(crate) fn offset_produced_sites(instruction: &mut LegacyInstruction, offset: usize) {
    match instruction {
        LegacyInstruction::Acquire { site, .. } => *site += offset,
        LegacyInstruction::ContractCall {
            produced_sites,
            container,
            ..
        } => {
            for site in produced_sites {
                *site += offset;
            }
            // Card W45-CONTAINER-ROOT-REACH: a foreign unlink's element site.
            if let Some(operands) = container {
                container_publication::offset_foreign_sites(operands, offset);
            }
        }
        LegacyInstruction::ElementStore {
            call: Some(call), ..
        } => offset_produced_sites(call, offset),
        // Card W45-SUB-OBJECT-STORES: a stored call's sites.
        LegacyInstruction::SubObjectStore {
            value: sub_object_store::StoredValue::Call(call),
            ..
        } => offset_produced_sites(call, offset),
        _ => {}
    }
}

/// Take the acquired element out of its temporary binding: the one fresh
/// identity the inner call bound at `place`, with its nullability. Its site
/// leaves the resource map, because its custody now belongs to the collection.
/// `None` when the call bound anything else.
pub(super) fn take_element_result(
    state: &mut AbstractState,
    place: &str,
) -> Option<crate::indexed_collection::Slot> {
    let binding = state.pointers.remove(place)?;
    let site = match binding
        .targets
        .iter()
        .copied()
        .collect::<Vec<_>>()
        .as_slice()
    {
        [site] if !binding.unknown && binding.derivation.is_identity() => *site,
        _ => {
            mark_unresolved_effect(state, &binding);
            return None;
        }
    };
    if state.resources.get(&site) != Some(&LIVE) || state.unresolved_effect_sites.contains(&site) {
        mark_unresolved_effect(state, &binding);
        return None;
    }
    let may_be_null = binding.may_be_null || state.nullable_sites.remove(&site);
    state.resources.remove(&site);
    state.conditional_replacements.remove(&site);
    for alias in state.pointers.values_mut() {
        if alias.targets.remove(&site) {
            alias.unknown = true;
            alias.may_be_null = true;
        }
    }
    Some(crate::indexed_collection::Slot::Live {
        may_be_null,
        sites: BTreeSet::from([site]),
    })
}

/// Run `operation` on the known ranges of `owner`'s collection (an absent one
/// is all unfilled) under this state's scalar facts, and write the ranges
/// back. An `Unknown` collection answers `false` without running it.
pub(super) fn with_collection(
    state: &mut AbstractState,
    owner: usize,
    operation: impl FnOnce(
        &mut crate::indexed_collection::Ranges,
        &dyn crate::indexed_collection::Scalars,
    ) -> bool,
) -> bool {
    if state.collection_vacuous {
        return true;
    }
    let mut ranges = match state.collections.get(&owner).cloned().unwrap_or_default() {
        crate::indexed_collection::Collection::Known(ranges) => ranges,
        crate::indexed_collection::Collection::Unknown => return false,
    };
    let answer = operation(&mut ranges, state);
    state
        .collections
        .insert(owner, crate::indexed_collection::Collection::Known(ranges));
    answer
}

/// An element operation whose base resolved but whose ranges could not be
/// proved: `UnknownAlias` on the element place, and the collection is
/// `Unknown` from here on. The path supports no custody claim.
pub(super) fn refuse_element(
    function: &str,
    line: usize,
    place: &str,
    owner: usize,
    state: &mut AbstractState,
    diagnostics: &mut BTreeSet<LegacyDiagnostic>,
) {
    diagnostics.insert(LegacyDiagnostic {
        function: function.to_owned(),
        line,
        kind: LegacyDiagnosticKind::UnknownAlias {
            variable: place.to_owned(),
        },
    });
    state
        .collections
        .insert(owner, crate::indexed_collection::Collection::Unknown);
}

/// The modeled `name++` of an index or counter: forget its scalar fact and
/// rename every collection bound `name + k` to `name + (k - 1)`.
pub(super) fn step_collections(state: &mut AbstractState, name: &str) {
    state.pointers.remove(name);
    state.index_bounds.remove(name);
    state.clamped_zero.remove(name);
    state.index_zero.remove(name);
    for bounds in state.index_bounds.values_mut() {
        bounds.retain(|bound| bound.name() != Some(name));
    }
    let owners = state.collections.keys().copied().collect::<Vec<_>>();
    for owner in owners {
        if let Some(crate::indexed_collection::Collection::Known(mut ranges)) =
            state.collections.get(&owner).cloned()
            && ranges.mentions(name)
        {
            ranges.step(name, state);
            state
                .collections
                .insert(owner, crate::indexed_collection::Collection::Known(ranges));
        }
    }
}

/// Whether `instruction` may write the scalar `name` other than by a modeled
/// step. Every name a collection bound reads is a slot no other lvalue
/// designates (the classifier's premise), so only a write *by name* reaches
/// it: a rebinding, or a refusal whose frame does not spare it.
pub(super) fn writes_scalar(instruction: &LegacyInstruction, name: &str) -> bool {
    if rebound_names(instruction).contains(&name) {
        return true;
    }
    // Card CG11 (rule 10v): either reading of a pointer store may run.
    if let Some(readings) = pointer_store_flow::readings(instruction) {
        return readings.iter().any(|reading| writes_scalar(reading, name));
    }
    match instruction {
        LegacyInstruction::Unknown { writes, .. } => writes.poisons(name),
        LegacyInstruction::IndirectScalarAccess {
            frame: Some(frame), ..
        } => frame.poisons(name),
        _ => false,
    }
}

/// A write to a scalar a collection bound or an index fact reads: the
/// collection is `Unknown` and the fact is dropped.
pub(super) fn forget_written_bounds(instruction: &LegacyInstruction, state: &mut AbstractState) {
    let owners = state.collections.keys().copied().collect::<Vec<_>>();
    for owner in owners {
        let written = match state.collections.get(&owner) {
            Some(crate::indexed_collection::Collection::Known(ranges)) => ranges
                .points()
                .iter()
                .filter_map(crate::indexed_collection::Term::name)
                .any(|name| writes_scalar(instruction, name)),
            _ => false,
        };
        if written {
            state
                .collections
                .insert(owner, crate::indexed_collection::Collection::Unknown);
        }
    }
    state
        .index_bounds
        .retain(|name, _| !writes_scalar(instruction, name));
    state
        .clamped_zero
        .retain(|name| !writes_scalar(instruction, name));
    state
        .index_zero
        .retain(|name| !writes_scalar(instruction, name));
    for bounds in state.index_bounds.values_mut() {
        bounds.retain(|bound| {
            bound
                .name()
                .is_none_or(|name| !writes_scalar(instruction, name))
        });
    }
}

/// A site acquired again (an allocation inside a loop) is a new array: its
/// elements are unfilled. If the old array may still owe an element, that
/// obligation cannot be followed, so the collection is `Unknown` instead.
pub(super) fn retire_reacquired_collection(state: &mut AbstractState, site: usize) {
    let Some(collection) = state.collections.get(&site) else {
        return;
    };
    if collection.may_owe(state) {
        state
            .collections
            .insert(site, crate::indexed_collection::Collection::Unknown);
    } else {
        state.collections.remove(&site);
    }
}

/// Card **W43-INDEXED-RESOURCE-COLLECTIONS**: a collection that may still owe
/// an element is handed to a call only to be released (the element
/// obligations are then checked at the exits) or consumed (they leave with
/// it, as a member link's do). Any other position — a borrow, an owned or
/// stored parameter, a reallocation — could move, overwrite or release
/// elements the ranges do not see, so it refuses with `UnknownAlias` before
/// the call's own effects apply, and the collection is `Unknown`.
pub(super) fn refuse_collection_owner_uses(
    function: &str,
    node: &LegacyCfgNode,
    state: &mut AbstractState,
    diagnostics: &mut BTreeSet<LegacyDiagnostic>,
) {
    let LegacyInstruction::ContractCall {
        arguments,
        contract,
        nested,
        ..
    } = &node.instruction
    else {
        return;
    };
    let calls = std::iter::once((arguments, contract)).chain(
        nested
            .iter()
            .map(|nested| (&nested.arguments, &nested.contract)),
    );
    let mut refused = Vec::new();
    for (arguments, contract) in calls {
        for (index, argument) in arguments.iter().enumerate() {
            let Some(place) = argument.as_deref() else {
                continue;
            };
            let transfers = matches!(
                contract.parameters.get(index),
                Some(
                    ResourceParameterMode::Released { .. } | ResourceParameterMode::Consumed { .. }
                )
            ) && !matches!(
                contract.result,
                ResourceReturnMode::Reallocated { parameter, .. } if parameter == index
            );
            if transfers {
                continue;
            }
            let Some(binding) = state.pointers.get(place) else {
                continue;
            };
            for site in &binding.targets {
                if state
                    .collections
                    .get(site)
                    .is_some_and(|collection| collection.may_owe(state))
                {
                    refused.push((place.to_owned(), *site));
                }
            }
        }
    }
    for (place, site) in refused {
        refuse_element(function, node.line, &place, site, state, diagnostics);
    }
}

pub(super) fn transfer_base(
    function: &str,
    node: &LegacyCfgNode,
    mut state: AbstractState,
    acquisition_lines: &BTreeMap<usize, usize>,
    context: &TransferContext<'_>,
    // C02-P-ALIAS: the may-alias facts on entry to this node, precomputed by
    // [`alias_facts`]. Read only; nothing here writes them, which is what keeps
    // the custody work list scheduled exactly as it was before this card.
    alias: &NodeAliasFacts,
    diagnostics: &mut BTreeSet<LegacyDiagnostic>,
) -> AbstractState {
    // The old route stays authoritative wherever it discharges. Only its
    // flow refusal consults the independently read scalar plan. Both probes
    // start from the same state and isolate every observable they can move:
    // state, diagnostics, null-premise frames and the consultation log. These
    // scalar instructions allocate no site and invoke no call.
    if let Some(candidate) = scalar_precedence::alternative(&node.instruction) {
        let probe = |instruction: LegacyInstruction| {
            let saved = qcore_schemas::contract_consultation::snapshot();
            let mut local = BTreeSet::new();
            let probe_node = LegacyCfgNode {
                instruction,
                ..node.clone()
            };
            let (output, premises) = crate::null_premise::collecting(|| {
                transfer_base(
                    function,
                    &probe_node,
                    state.clone(),
                    acquisition_lines,
                    context,
                    alias,
                    &mut local,
                )
            });
            qcore_schemas::contract_consultation::restore(&saved);
            (output, local, premises)
        };
        let original = probe(scalar_precedence::original(&node.instruction));
        let (output, local, premises) = if original.1.is_empty() {
            original
        } else {
            let typed = probe(candidate.clone());
            if typed.1.is_empty() { typed } else { original }
        };
        diagnostics.extend(local);
        for premise in premises {
            crate::null_premise::note(function, premise.line, &premise.variable);
        }
        return output;
    }
    // A slot name denotes the caller's original pointer only while that
    // formal is unchanged. Tracking its reassignment would need pointer-to-
    // pointer alias analysis; reject it rather than writing a fictitious slot.
    let destination = match &node.instruction {
        LegacyInstruction::Acquire { destination, .. }
        | LegacyInstruction::Alias { destination, .. }
        | LegacyInstruction::OpaqueValue { destination, .. }
        | LegacyInstruction::VaArgPointer { destination }
        | LegacyInstruction::ForgetScalar { destination }
        | LegacyInstruction::PointerStep { destination, .. }
        | LegacyInstruction::Clear { destination } => Some(destination.as_str()),
        LegacyInstruction::ContractCall { destination, .. }
        | LegacyInstruction::IndirectScalarAccess { destination, .. } => destination.as_deref(),
        _ => None,
    };
    let changes_slot_identity = |place: &str| {
        context
            .sanctioned
            .iter()
            .any(|slot| slot.strip_prefix('*') == Some(place))
    };
    let overwritten = destination.is_some_and(changes_slot_identity)
        || match &node.instruction {
            LegacyInstruction::ContractCall {
                arguments,
                contract,
                ..
            } => contract
                .parameters
                .iter()
                .zip(arguments)
                .any(|(mode, argument)| {
                    matches!(mode, ResourceParameterMode::Out { .. })
                        && argument.as_deref().is_some_and(changes_slot_identity)
                }),
            _ => false,
        };
    if overwritten {
        diagnostics.insert(LegacyDiagnostic {
            function: function.to_owned(),
            line: node.line,
            kind: LegacyDiagnosticKind::Unsupported {
                description: "reassignment of an out-parameter pointer".to_owned(),
            },
        });
        for pointer in state.pointers.values_mut() {
            pointer.unknown = true;
            pointer.may_be_null = true;
        }
        return state;
    }
    // CTWIN-RESIDUALS, shape `CTWIN-PUBLISH-OUT`: a declared `Out` slot whose
    // actual *is* one of those objects acquires the resource directly into it.
    // That is the `Acquire` arm below seen from the callee's side — the
    // identity is published at the moment it is created — and it was measured
    // invisible: `y_fill(&g_published); y_release(g_published);` was `Safe`
    // with no diagnostic, the publish-then-free hazard in the out shape, and
    // the same body without the release reported a `Leak` the projection is in
    // no position to claim. Refused *before* the call's effects apply, so no
    // site exists to carry forward.
    if let LegacyInstruction::ContractCall {
        arguments,
        contract,
        ..
    } = &node.instruction
        && let Some(place) = contract
            .parameters
            .iter()
            .zip(arguments)
            .filter(|(mode, _)| matches!(mode, ResourceParameterMode::Out { .. }))
            .filter_map(|(_, argument)| argument.as_deref())
            .find(|argument| context.publications.destinations.contains(*argument))
    {
        let place = place.to_owned();
        // The call's declared effects never apply, so no identity it names may
        // keep a residual claim: a `Released` argument would otherwise stay
        // live to the exit and be reported as a leak the call was going to
        // discharge. Marking exactly the arguments' own identities keeps an
        // unrelated resource's residual intact.
        let named = arguments
            .iter()
            .flatten()
            .filter_map(|argument| state.pointers.get(argument).cloned())
            .collect::<Vec<_>>();
        for pointer in &named {
            mark_unresolved_effect(&mut state, pointer);
        }
        reject_publication(function, node.line, &place, &mut state, diagnostics);
        return state;
    }
    // CTWIN-PUBLISH: a store of an identity *this body* holds into an object
    // that outlives the call is a publication this projection does not model.
    // Checked here, before any effect of the instruction applies, and
    // flow-sensitively: naming such a destination is not the refusal, storing
    // a tracked identity into it is. See [`reject_publication`] for why both
    // directions are withdrawn rather than an escape recorded, and
    // [`crate::publication`] for how the destination set is read.
    if let Some(place) =
        destination.filter(|place| context.publications.destinations.contains(*place))
    {
        let published = match &node.instruction {
            // The value is whatever `source` denotes here — **any** identity
            // this projection tracks, caller-held or body-acquired.
            //
            // Card CTWIN-RESIDUALS withdrew the caller-held exclusion card
            // CTWIN-PUBLISH left open as `CTWIN-PUBLISH-BORROWED`, on a
            // measured ground rather than a wider notion of safety: a body that
            // publishes a *borrowed* formal was `Safe` with no diagnostic, so
            // W4 published `Borrowed` as its summary for that formal, and a
            // caller that acquired, called it and released was `Safe` too —
            // which is the wave-16 headline body with one level of indirection
            // through a helper, and the rule that refuses it inlined must
            // refuse it composed. `Borrowed` asserts that the caller's
            // obligations are unchanged across the call, and a publication
            // falsifies exactly that. The faithful alternative modes —
            // `Escaped`, `Consumed` — are the escape model CTWIN-PUBLISH §3
            // proved promotes, so the only fail-closed answer is withdrawal.
            //
            // The rule therefore reads no provenance, exactly as it reads no
            // per-site status: a formal the projection tracks no identity for
            // still publishes nothing, so a body analyzed with no entry custody
            // keeps its pre-existing answer.
            // Card W43-INDEX-TRAVERSAL: a stepped pointer into a tracked
            // object stored into such a destination publishes that object's
            // interior exactly as a copy publishes it.
            // Card W43-DESCRIPTOR-RESOURCE-TRACKING: a declared scalar that
            // holds a tracked descriptor is custody, so the scalar-source
            // exemption does not cover it.
            LegacyInstruction::Alias { source, .. }
            | LegacyInstruction::PointerStep { source, .. }
                if !context.publications.scalar_sources.contains(source)
                    || carries_integer_identity(&state, source, &context.site_kinds) =>
            {
                state
                    .pointers
                    .get(source)
                    .filter(|pointer| !pointer.targets.is_empty())
                    .cloned()
            }
            // An acquisition bound directly to such an object publishes it at
            // the moment it is created. Refused *before* the site exists, so
            // there is nothing to carry forward.
            LegacyInstruction::Acquire { .. } => Some(AbstractPointer::default()),
            LegacyInstruction::ContractCall { contract, .. } => {
                (!matches!(contract.result, ResourceReturnMode::None))
                    .then(AbstractPointer::default)
            }
            // `Clear` stores the null constant and `OpaqueValue` a value this
            // function never tracks; neither publishes a resource identity.
            _ => None,
        };
        if let Some(pointer) = published {
            // The published identities carry an unresolved effect from here on,
            // exactly as an unresolved callee effect does: the projection can
            // no longer say whether they are released, so a residual leak that
            // *depends on* this publication stays `Unknown` instead of being
            // reported as a validated violation (the w4-nullable-removal rule).
            mark_unresolved_effect(&mut state, &pointer);
            reject_publication(function, node.line, place, &mut state, diagnostics);
            return state;
        }
    }
    // A store through a pointer is modeled only where the contract in force
    // declares that slot an out parameter. `ContractCall` handles its own
    // destination below, because a call whose result carries no custody never
    // touched the slot in the first place.
    if let LegacyInstruction::Alias {
        destination,
        source: _,
    }
    | LegacyInstruction::Clear { destination } = &node.instruction
        && is_indirect_place(destination)
        && !context.sanctioned.contains(destination.as_str())
    {
        reject_unsanctioned_store(function, node.line, destination, &mut state, diagnostics);
        return state;
    }
    // Card W43-LIBC-RESULT-ALIAS: a derived pointer stored through a sanctioned
    // out slot would be observed at the exit as delivering its base's
    // identity, which a W4 summary could then publish as fresh `Out` custody.
    // The store is refused and the slot left as it was.
    if let LegacyInstruction::Alias {
        destination,
        source,
    } = &node.instruction
        && is_indirect_place(destination)
        && let Some(pointer) = state
            .pointers
            .get(source)
            .filter(|pointer| !pointer.derivation.is_identity())
            .cloned()
    {
        diagnostics.insert(LegacyDiagnostic {
            function: function.to_owned(),
            line: node.line,
            kind: LegacyDiagnosticKind::UnknownAlias {
                variable: destination.clone(),
            },
        });
        mark_unresolved_effect(&mut state, &pointer);
        return state;
    }
    // Forwarded Out arguments denote indirect writes just like *out = value.
    // A local alias or an unbound pointer must never gain a fabricated slot.
    if let LegacyInstruction::ContractCall {
        arguments,
        contract,
        ..
    } = &node.instruction
    {
        for (mode, argument) in contract.parameters.iter().zip(arguments) {
            if !matches!(mode, ResourceParameterMode::Out { .. })
                && argument.as_deref().is_some_and(changes_slot_identity)
            {
                diagnostics.insert(LegacyDiagnostic {
                    function: function.to_owned(),
                    line: node.line,
                    kind: LegacyDiagnosticKind::Unsupported {
                        description: "out-slot pointer passed without a modeled Out effect".into(),
                    },
                });
                for pointer in state.pointers.values_mut() {
                    pointer.unknown = true;
                    pointer.may_be_null = true;
                }
                return state;
            }
            if matches!(mode, ResourceParameterMode::Out { .. })
                && let Some(place) = argument
                && is_indirect_place(place)
                && !context.sanctioned.contains(place.as_str())
            {
                reject_unsanctioned_store(function, node.line, place, &mut state, diagnostics);
                return state;
            }
        }
    }
    // O04-READ-BOUND: every admitted bound *read* of an aggregate member
    // requires the same flow-sensitive premise as a store, plus the member
    // actually carrying the field link an admitted store recorded. A read
    // through a released, escaped, refuted, ambiguous, nullable or caller-held
    // aggregate refuses and poisons the path, and so does a member this body
    // never linked or whose binding no longer agrees with its link. This is the
    // C00 premise in its read direction.
    if let LegacyInstruction::ContractCall {
        arguments,
        contract,
        aggregate_reads,
        aggregate_consumed,
        aggregate_addresses,
        container,
        ..
    } = &node.instruction
        && let Some(owner) = aggregate_reads
            .iter()
            .find(|place| !bound_read_resolves(&state, place, &context.caller_sites))
            .and_then(|place| crate::aggregate_store::place_owner(place))
            // O04-B09B10: the same C00 premise for a member *address*. The
            // member itself carries no link — it is declared with no `*` — so
            // only the aggregate's own liveness is required, not a field link.
            .or_else(|| {
                aggregate_addresses
                    .iter()
                    .map(String::as_str)
                    .find(|owner| {
                        resolved_aggregate_owner(&state, owner, &context.caller_sites).is_none()
                            // Card W45-FB-RETRY-LIFETIME: the unlink of an
                            // object this path published is the one use of a
                            // published object's member address.
                            && !container_publication::unlinks_published(
                                container.as_deref(),
                                owner,
                                &state,
                            )
                    })
            })
    {
        diagnostics.insert(LegacyDiagnostic {
            function: function.to_owned(),
            line: node.line,
            kind: LegacyDiagnosticKind::UnknownAlias {
                variable: owner.to_owned(),
            },
        });
        // Card W45-FB-BACKLOG-CUSTODY: the refused call hands on what its
        // contract says it may move (`refused_operands::withdraw_refused_call`),
        // before the bindings are poisoned: the actuals at custody positions,
        // the entries of its container effects, and the bound reads it takes.
        // A `Borrowed` read the contract moves nothing through is not handed
        // on: the refusal is about the read's liveness, not the call's effect.
        refused_operands::withdraw_refused_call(
            &mut state,
            contract
                .parameters
                .iter()
                .zip(arguments)
                .filter(|(mode, _)| !matches!(mode, ResourceParameterMode::Borrowed))
                .filter_map(|(_, argument)| argument.as_deref())
                .chain(
                    container
                        .iter()
                        .flat_map(|operands| operands.0.iter())
                        .filter_map(|operand| match operand {
                            container_publication::Operand::Published {
                                entry: container_publication::Entry::Name(name),
                                ..
                            }
                            | container_publication::Operand::Unpublished {
                                entry: container_publication::Entry::Name(name),
                                ..
                            } => Some(name.as_str()),
                            _ => None,
                        }),
                )
                .chain(aggregate_consumed.iter().map(String::as_str)),
        );
        for pointer in state.pointers.values_mut() {
            pointer.unknown = true;
            pointer.may_be_null = true;
        }
        return state;
    }
    // C02-P-ALIAS: every modeled read of a pointer this rule relates to another
    // declared name refuses when that peer was released on this path and
    // neither discharge applies. Reported before the instruction's own effects,
    // which still apply unchanged: this can only add an `UnknownAlias`, never
    // suppress a leak or a double release.
    match &node.instruction {
        LegacyInstruction::ContractCall { alias_reads, .. } => require_unaliased_reads(
            function,
            node.line,
            alias_reads,
            &state,
            &context.caller_sites,
            alias,
            diagnostics,
        ),
        LegacyInstruction::OpaqueValue {
            alias_read: Some(read),
            ..
        } => require_unaliased_reads(
            function,
            node.line,
            std::slice::from_ref(read),
            &state,
            &context.caller_sites,
            alias,
            diagnostics,
        ),
        LegacyInstruction::ScalarMemberStore {
            alias_read: Some(read),
            ..
        } => require_unaliased_reads(
            function,
            node.line,
            std::slice::from_ref(read),
            &state,
            &context.caller_sites,
            alias,
            diagnostics,
        ),
        LegacyInstruction::IndirectScalarAccess { alias_reads, .. } => require_unaliased_reads(
            function,
            node.line,
            alias_reads,
            &state,
            &context.caller_sites,
            alias,
            diagnostics,
        ),
        _ => {}
    }
    // W42-INDIRECT-ACCESS: a scalar destination store dereferences its base.
    // Require one settled live identity after the may-alias obligation above,
    // without mutating any projection map.  Every declined shape has already
    // retained the lexical refusal; this is the flow-sensitive refusal for the
    // one typed shape the classifier lowered explicitly.
    if let LegacyInstruction::ScalarMemberStore { base, .. } = &node.instruction
        && !scalar_member_store_base_resolves(&state, base)
    {
        diagnostics.insert(LegacyDiagnostic {
            function: function.to_owned(),
            line: node.line,
            kind: LegacyDiagnosticKind::UnknownAlias {
                variable: base.clone(),
            },
        });
        // The unresolved dereference refuses this function, but the typed
        // statement still writes only a scalar field from a scalar-only RHS.
        // It cannot rebind any pointer or resource identity, so retain the
        // incoming frame and keep later residual/double-release evidence.
        return state;
    }
    // Card W44-INDIRECT-CALL-ARGUMENT-ACCESS: the second half of an admitted
    // call actual's obligation. Custody-neutral: it only reports.
    if let LegacyInstruction::ArgumentReads {
        names,
        pointer_bases,
        pointer_places,
    } = &node.instruction
    {
        for variable in call_argument_access::refused_reads(
            &state,
            &context.site_kinds,
            names,
            pointer_bases,
            pointer_places,
        ) {
            diagnostics.insert(LegacyDiagnostic {
                function: function.to_owned(),
                line: node.line,
                kind: LegacyDiagnosticKind::UnknownAlias { variable },
            });
        }
        return state;
    }
    // W43-INDIRECT-ACCESS-GENERAL: every base the typed scalar statement
    // dereferences must pass the dereference obligation here, after the
    // may-alias obligations above. A slot an out-parameter contract sanctions
    // is never written by this route: it keeps the out-slot machinery's own
    // decision, so a `*B` store naming one refuses. Admitted or refused, the
    // statement writes only a scalar, so every projection map is preserved;
    // the one effect is forgetting the scalar destination's old fact.
    if let LegacyInstruction::IndirectScalarAccess {
        bases,
        destination,
        frame,
        chains,
        borrowed_destination,
        readback,
        alias_reads,
        ..
    } = &node.instruction
    {
        // Keep the original instruction and all read obligations. A checked
        // borrowed view can expose only name-only inference sites, and only
        // after the original-state alias obligation has discharged.
        let mut alias_failures = BTreeSet::new();
        if node.lookup_read.is_some() {
            require_unaliased_reads(
                function,
                node.line,
                alias_reads,
                &state,
                &context.caller_sites,
                alias,
                &mut alias_failures,
            );
        }
        let view = alias_failures
            .is_empty()
            .then(|| lookup_route::read_view(node, &state, &context.derived_bases))
            .flatten();
        let read_state = view.as_ref().unwrap_or(&state);
        let mut read_failed = !alias_failures.is_empty();
        diagnostics.extend(alias_failures);
        // Card CG12-VOID-VIEW-READBACK (rule 11j): the member's identity,
        // read on the state entering the node, where the readback resolves.
        let read_back = readback
            .as_deref()
            .filter(|_| *borrowed_destination)
            .and_then(|place| {
                void_view_route::readback_binding(function, node.line, &state, place, context)
            });
        for base in bases {
            let sanctioned = context.sanctioned.contains(format!("*{base}").as_str());
            // Card NULL-SAFETY-PROPERTY (rule 14n): a premised base is read
            // non-null here, under a named premise.
            if sanctioned
                || !null_premise_route::base_resolves(function, node.line, read_state, base)
            {
                read_failed = true;
                diagnostics.insert(LegacyDiagnostic {
                    function: function.to_owned(),
                    line: node.line,
                    kind: LegacyDiagnosticKind::UnknownAlias {
                        variable: base.clone(),
                    },
                });
            }
        }
        // Card W44-INDIRECT-CHAIN-ACCESS: every pointer a member chain reads
        // through keeps its own obligation; the decision is
        // `indirect_chain_access::chain_resolves`, over this state read-only.
        for chain in chains {
            if let Err(variable) =
                null_premise_route::chain_resolves_premised(function, node.line, read_state, chain)
            {
                read_failed = true;
                diagnostics.insert(LegacyDiagnostic {
                    function: function.to_owned(),
                    line: node.line,
                    kind: LegacyDiagnosticKind::UnknownAlias { variable },
                });
            }
        }
        let lookup_binding = if !read_failed && view.is_some() {
            node.lookup_read.as_ref().map(|description| {
                derive_pointer(
                    state.pointers.get(&description.parent),
                    DerivedExtent::Borrowed,
                    true,
                    true,
                )
            })
        } else {
            None
        };
        if let Some(destination) = destination {
            if *borrowed_destination {
                state.pointers.insert(
                    destination.clone(),
                    lookup_binding.or(read_back).unwrap_or(AbstractPointer {
                        targets: BTreeSet::new(),
                        unknown: true,
                        may_be_null: true,
                        derivation: Derivation::Identity,
                    }),
                );
            } else {
                state.pointers.remove(destination);
            }
        }
        if let Some(frame) = frame {
            for (place, pointer) in state.pointers.iter_mut() {
                if frame.poisons(place) {
                    pointer.unknown = true;
                    pointer.may_be_null = true;
                }
            }
        }
        return state;
    }
    // Card W43-INDEX-TRAVERSAL: `destination = source ± offset`. The result
    // keeps `source`'s targets and is derived from them with an interior
    // extent, so it owns nothing: every custody position refuses or accuses
    // it, and the base's own obligation is untouched. No resource status,
    // marker, containment or field link changes.
    if let LegacyInstruction::PointerStep {
        destination,
        source,
        offset,
    } = &node.instruction
    {
        let before = state.pointers.get(destination).cloned();
        match state.pointers.get(source).cloned() {
            // Arithmetic on a pointer into an object no reaching path bound
            // stays inside that object: the result is unbound too.
            None => {
                state.pointers.remove(destination);
            }
            Some(pointer) => {
                let stepped = AbstractPointer {
                    derivation: pointer.derivation.step(*offset == StepOffset::Forward),
                    ..pointer
                };
                state.pointers.insert(destination.clone(), stepped);
            }
        }
        // Stepping a name in place must not lose the only name of a live
        // tracked object: `p = malloc(n); p++; free(p - 1);` would otherwise
        // leave the allocation named by nothing but arithmetic the projection
        // does not evaluate, and a later leak or credit would be a guess.
        // Such a step refuses, and the object's fate is left unresolved, so no
        // leak is reported and no release is credited for it.
        if destination == source
            && let Some(before) = before
            && before.derivation.is_identity()
        {
            let lost = before
                .targets
                .iter()
                .copied()
                .filter(|site| {
                    state
                        .resources
                        .get(site)
                        .is_some_and(|status| status & LIVE != 0)
                        && !state.pointers.iter().any(|(name, pointer)| {
                            name != destination
                                && pointer.derivation.is_identity()
                                && pointer.targets.contains(site)
                        })
                        && !state
                            .field_links
                            .values()
                            .any(|fields| fields.values().any(|children| children.contains(site)))
                        && !state
                            .contained
                            .values()
                            .any(|children| children.contains(site))
                })
                .collect::<BTreeSet<_>>();
            if !lost.is_empty() {
                diagnostics.insert(LegacyDiagnostic {
                    function: function.to_owned(),
                    line: node.line,
                    kind: LegacyDiagnosticKind::UnknownAlias {
                        variable: destination.clone(),
                    },
                });
                mark_unresolved_effect(
                    &mut state,
                    &AbstractPointer {
                        targets: lost,
                        ..AbstractPointer::default()
                    },
                );
            }
        }
        return state;
    }
    // O00 blocker B08: every store into an admitted aggregate field requires
    // its aggregate to resolve, here and now, to exactly one live identity
    // *this* body acquired. A released, escaped, refuted, ambiguous, nullable
    // or caller-held aggregate refuses and poisons the path; this is the C00
    // premise in its store direction, checked flow-sensitively rather than
    // syntactically.
    let stored_field = match written_field_place(&node.instruction) {
        None => None,
        Some(place) => {
            let owner = crate::aggregate_store::place_owner(place).unwrap_or_default();
            match resolved_aggregate_owner(&state, owner, &context.caller_sites) {
                Some(site) => {
                    let field = crate::aggregate_store::place_field(place).unwrap_or_default();
                    Some((site, field.to_owned(), place.to_owned()))
                }
                None => {
                    diagnostics.insert(LegacyDiagnostic {
                        function: function.to_owned(),
                        line: node.line,
                        kind: LegacyDiagnosticKind::UnknownAlias {
                            variable: owner.to_owned(),
                        },
                    });
                    for pointer in state.pointers.values_mut() {
                        pointer.unknown = true;
                        pointer.may_be_null = true;
                    }
                    return state;
                }
            }
        }
    };
    // Card RULE34-PILOT: the transfer position (route map §10). An admitted
    // contract call's container operands, ordered by the transfer arms around
    // the call's own reading ([`crate::transfer_rules`]; rule 34 moves an
    // element destroyer's unlink before it). A call that declares an unlink of
    // a parameter it releases, which no transfer rule orders (rule 34 off), is
    // refused before any of its effects apply: the host's order would release
    // before unlinking. As for any call whose declared effects never apply, its
    // arguments' own identities lose their residual claim, so a release the
    // call was going to make is never reported as a leak, and every binding is
    // poisoned, as an uncontracted call's refusal poisons them.
    let ordered_container = match &node.instruction {
        LegacyInstruction::ContractCall {
            function: called,
            arguments,
            contract,
            container: Some(operands),
            ..
        } => match crate::transfer_rules::container_order(called, contract, operands) {
            Some(ordered) => Some(ordered),
            None => {
                let named = arguments
                    .iter()
                    .flatten()
                    .filter_map(|argument| state.pointers.get(argument).cloned())
                    .collect::<Vec<_>>();
                for pointer in &named {
                    mark_unresolved_effect(&mut state, pointer);
                }
                diagnostics.insert(LegacyDiagnostic {
                    function: function.to_owned(),
                    line: node.line,
                    kind: LegacyDiagnosticKind::Unsupported {
                        description: crate::transfer_rules::unordered_destroyer(called),
                    },
                });
                for pointer in state.pointers.values_mut() {
                    pointer.unknown = true;
                    pointer.may_be_null = true;
                }
                return state;
            }
        },
        _ => None,
    };
    match &node.instruction {
        LegacyInstruction::Acquire { destination, site } => {
            state.resources.insert(*site, LIVE);
            state.pointers.insert(
                destination.clone(),
                AbstractPointer {
                    targets: BTreeSet::from([*site]),
                    unknown: false,
                    may_be_null: false,
                    derivation: Derivation::Identity,
                },
            );
        }
        // Card W43-INDEX-TRAVERSAL: a copy of a declared scalar, or of a
        // declared single-level pointer object, that no reaching path bound
        // carries nothing this projection tracks — a scalar has no custody, and
        // an unbound pointer denotes an object no modeled statement bound, the
        // reading an unbound base already gets at a dereference or a `Borrowed`
        // position. The copy is therefore unbound too, rather than an unknown,
        // possibly-null binding with an unresolved-alias refusal. Any later
        // custody use of it (a release, an escape, an `Owned` position) finds
        // no binding and refuses exactly as the source would have.
        LegacyInstruction::Alias {
            destination,
            source,
        } if !state.pointers.contains_key(source)
            && ((context.traversal.scalars.contains(destination)
                && context.traversal.scalars.contains(source))
                || (context.traversal.pointers.contains(destination)
                    && context.traversal.pointers.contains(source))) =>
        {
            state.pointers.remove(destination);
        }
        LegacyInstruction::Alias {
            destination,
            source,
        } => {
            let pointer = state.pointers.get(source).cloned().unwrap_or_else(|| {
                diagnostics.insert(LegacyDiagnostic {
                    function: function.to_owned(),
                    line: node.line,
                    kind: LegacyDiagnosticKind::UnknownAlias {
                        variable: source.clone(),
                    },
                });
                AbstractPointer {
                    targets: BTreeSet::new(),
                    unknown: true,
                    may_be_null: true,
                    derivation: Derivation::Identity,
                }
            });
            state.pointers.insert(destination.clone(), pointer);
        }
        LegacyInstruction::Clear { destination } => {
            state.pointers.insert(
                destination.clone(),
                AbstractPointer {
                    may_be_null: true,
                    ..AbstractPointer::default()
                },
            );
        }
        LegacyInstruction::Release { source }
            if kind_mismatch(&state, source, ResourceKind::Heap, &context.site_kinds) =>
        {
            refuse_kind_mismatch(
                function,
                node.line,
                source,
                ResourceParameterMode::Released { nullable: false },
                &mut state,
                diagnostics,
            );
        }
        LegacyInstruction::Escape { source }
            if kind_mismatch(&state, source, ResourceKind::Heap, &context.site_kinds) =>
        {
            refuse_kind_mismatch(
                function,
                node.line,
                source,
                ResourceParameterMode::Consumed { nullable: false },
                &mut state,
                diagnostics,
            );
        }
        LegacyInstruction::Release { source } => {
            apply_resource_action(
                function,
                node.line,
                source,
                &mut state,
                acquisition_lines,
                diagnostics,
                false,
                false,
            );
        }
        LegacyInstruction::ExactFormalReturn { source, parameter } => {
            exact_formal_return::apply(
                &mut state,
                source,
                *parameter,
                &context.exact_return_bases,
                diagnostics,
                function,
                node.line,
            );
        }
        LegacyInstruction::Escape { source } => {
            apply_resource_action(
                function,
                node.line,
                source,
                &mut state,
                acquisition_lines,
                diagnostics,
                true,
                false,
            );
        }
        // Card W43-RESULT-ALIAS-RETURNED. Under an entry whose result is
        // derived from a borrowed formal, returning a derived binding over that
        // formal's name-only site hands the caller the non-owning relation the
        // summary states and escapes nothing: no status changes and no
        // obligation of this body is discharged. `observe_exit` reports it and
        // the summary check decides it. Every other binding keeps the rule
        // below, which refuses a derived escape.
        LegacyInstruction::EscapeIfOwned { source }
            if state.pointers.get(source).is_some_and(|pointer| {
                !pointer.unknown
                    && matches!(pointer.derivation, Derivation::Derived { .. })
                    && pointer.targets.len() == 1
                    && pointer
                        .targets
                        .iter()
                        .all(|site| context.derived_bases.contains(site))
            }) => {}
        LegacyInstruction::EscapeIfOwned { source } => {
            // Card W43-INDEX-TRAVERSAL: returning a declared scalar whose
            // binding carries no target (a counter's zero fact) returns no
            // custody, exactly as returning one with no binding does.
            let scalar_fact = context.traversal.scalars.contains(source)
                && state
                    .pointers
                    .get(source)
                    .is_some_and(|pointer| pointer.targets.is_empty());
            if state.pointers.contains_key(source) && !scalar_fact {
                apply_resource_action(
                    function,
                    node.line,
                    source,
                    &mut state,
                    acquisition_lines,
                    diagnostics,
                    true,
                    // Returning a possibly-null place is an unresolved alias
                    // unless this function's own result mode already obliges
                    // its callers to null-check what they get back.
                    context.result_nullable,
                );
            }
        }
        LegacyInstruction::ContractCall {
            function: called,
            destination,
            arguments,
            contract,
            produced_sites,
            // All revalidated above, before any effect of this call applies.
            aggregate_reads: _,
            aggregate_consumed: _,
            aggregate_addresses: _,
            alias_reads: _,
            nested,
            // Ordered above, at the transfer position.
            container: _,
            retention,
        } => {
            // Card O04-TRACE: the admitted nested call's declared effects, in
            // order, before the enclosing call's. It produces nothing, so its
            // produced-site vector is empty by construction.
            for nested in nested {
                apply_contract_call(
                    function,
                    node.line,
                    &nested.function,
                    None,
                    &nested.arguments,
                    &nested.contract,
                    &[],
                    &mut state,
                    acquisition_lines,
                    &context.site_kinds,
                    diagnostics,
                );
            }
            // Card W48-SCHED-CLEANUP-PRODUCTION (rule 34, asked at the
            // transfer position since RULE34-PILOT): an element destroyer's
            // unlink, before the call's own reading, so that the declared
            // release ends the custody the unlink took.
            let (destroyers, container) = match ordered_container {
                Some((before, after)) => (Some(before), Some(after)),
                None => (None, None),
            };
            if let Some(operands) = destroyers
                .as_ref()
                .filter(|operands| !operands.0.is_empty())
            {
                container_publication::apply(
                    function,
                    node.line,
                    called,
                    operands,
                    &mut state,
                    diagnostics,
                );
            }
            let container = container.as_ref().filter(|operands| !operands.0.is_empty());
            let unsanctioned = destination
                .as_deref()
                .filter(|place| is_indirect_place(place) && !context.sanctioned.contains(place));
            match unsanctioned {
                // A result that carries no custody never wrote the slot, so
                // the call's parameter effects still apply and the unmodeled
                // destination is simply ignored — the pre-out-slot behavior.
                Some(_) if matches!(contract.result, ResourceReturnMode::None) => {
                    apply_contract_call(
                        function,
                        node.line,
                        called,
                        None,
                        arguments,
                        contract,
                        produced_sites,
                        &mut state,
                        acquisition_lines,
                        &context.site_kinds,
                        diagnostics,
                    );
                }
                // Custody would be created in a place this analysis cannot
                // account for: refuse the whole call.
                Some(_) => {
                    diagnostics.insert(LegacyDiagnostic {
                        function: function.to_owned(),
                        line: node.line,
                        kind: LegacyDiagnosticKind::Unsupported {
                            description: format!(
                                "discarded owned result of resource API `{called}`"
                            ),
                        },
                    });
                    for pointer in state.pointers.values_mut() {
                        pointer.unknown = true;
                        pointer.may_be_null = true;
                    }
                }
                None => apply_contract_call(
                    function,
                    node.line,
                    called,
                    destination.as_deref(),
                    arguments,
                    contract,
                    produced_sites,
                    &mut state,
                    acquisition_lines,
                    &context.site_kinds,
                    diagnostics,
                ),
            }
            // Card W45-FB-RETRY-LIFETIME: the declared container effect, after
            // the call's borrowed reading.
            if let Some(operands) = container {
                container_publication::apply(
                    function,
                    node.line,
                    called,
                    operands,
                    &mut state,
                    diagnostics,
                );
            }
            // Card CALLEE-RETENTION-CLASS1 (rule 12t): the declared resets and
            // retentions, after the call's borrowed reading.
            if let Some(operands) = retention {
                retained_by_route::apply(
                    function,
                    node.line,
                    called,
                    operands,
                    destination.as_deref(),
                    &mut state,
                    diagnostics,
                );
                // Card REFCOUNT-RESOURCE-KIND (rule 13b): the keeps and drops.
                if !operands.references.is_empty() {
                    reference_route::apply(
                        function,
                        node.line,
                        called,
                        &operands.references,
                        destination.as_deref(),
                        produced_sites,
                        acquisition_lines,
                        &mut state,
                        diagnostics,
                    );
                }
                // Card CG22-CALLBACK-REGISTRY-CONTRACT (rules 13w, 13x): the
                // registry effects.
                if let Some(callbacks) = &operands.callbacks {
                    callback_route::apply(
                        function,
                        node.line,
                        called,
                        callbacks,
                        destination.as_deref(),
                        &mut state,
                        acquisition_lines,
                        diagnostics,
                    );
                }
            }
        }
        LegacyInstruction::OpaqueValue {
            destination,
            // Revalidated above, before the binding takes effect.
            alias_read: _,
        } => {
            state.pointers.insert(
                destination.clone(),
                AbstractPointer {
                    targets: BTreeSet::new(),
                    unknown: true,
                    may_be_null: true,
                    derivation: Derivation::Identity,
                },
            );
        }
        // Card VA-LOOP-AND-POINTER-ARG (rule 12c): a derived binding over no
        // base: owns nothing, refused at every custody position.
        LegacyInstruction::VaArgPointer { destination } => {
            state.pointers.insert(
                destination.clone(),
                derive_pointer(None, DerivedExtent::Exact, true, false),
            );
        }
        LegacyInstruction::ForgetScalar { destination } => {
            state.pointers.remove(destination);
        }
        LegacyInstruction::ScalarMemberStore { .. } => {}
        // Handled completely before this match: they return there.
        LegacyInstruction::IndirectScalarAccess { .. } | LegacyInstruction::PointerStep { .. } => {}
        // Card W44-INDIRECT-CALL-ARGUMENT-ACCESS: handled before this match too.
        LegacyInstruction::ArgumentReads { .. } => {}
        LegacyInstruction::AggregateStore { place } => {
            // The stored value carries no tracked identity, so the member binds
            // nothing. Whatever it previously linked is unlinked below, which
            // is what makes an overwritten resource visible as a residual.
            state.pointers.remove(place);
        }
        LegacyInstruction::BranchOnNull { .. }
        | LegacyInstruction::BranchOnValidity { .. }
        | LegacyInstruction::BranchOnDistinct { .. } => {}
        // Card MK-EVENT-CELL-CROSS-FUNCTION (rule 38c).
        LegacyInstruction::CellProtocolStep { step } => {
            super::descriptor_cells_route::apply_step(
                function,
                node.line,
                step,
                &mut state,
                diagnostics,
            );
        }
        LegacyInstruction::Unknown {
            description,
            writes,
        } => {
            diagnostics.insert(LegacyDiagnostic {
                function: function.to_owned(),
                line: node.line,
                kind: LegacyDiagnosticKind::Unsupported {
                    description: description.clone(),
                },
            });
            // CTWIN-RESIDUALS, shape `CTWIN-PUBLISH-INDIRECT`. The statement
            // keeps the refusal it already had — the attribution stays with the
            // gate that read it — and additionally withdraws the residual claim
            // on whatever identity it stored, because the destination is a
            // continued access on an object that outlives the call. Only the
            // sites the statement's own operands denote are marked, so an
            // unrelated resource's residual is untouched; a statement that
            // names no tracked identity marks nothing, and nothing here can
            // turn a refusal into an admission.
            //
            // The value test is the direct arm's, exactly: a source must denote
            // at least one site and must not be a declared scalar. Presence in
            // the binding map is *not* enough — an ordinary scalar assignment
            // puts a name there with no targets — and refusing on that would be
            // refusal without a property. Measured: `crc32c_table[k][n] = crc;`
            // in `librdkafka/crc32c.c` is the corpus's own witness, and it
            // publishes nothing.
            if let Some(publication) = context.publications.indirect.get(&node.line) {
                let published = publication
                    .sources
                    .iter()
                    .filter(|source| {
                        !context.publications.scalar_sources.contains(*source)
                            || carries_integer_identity(&state, source, &context.site_kinds)
                    })
                    .filter_map(|source| state.pointers.get(source))
                    .filter(|pointer| !pointer.targets.is_empty())
                    .cloned()
                    .collect::<Vec<_>>();
                if !published.is_empty() {
                    for pointer in &published {
                        mark_unresolved_effect(&mut state, pointer);
                    }
                    reject_publication(
                        function,
                        node.line,
                        &publication.place,
                        &mut state,
                        diagnostics,
                    );
                }
            }
            // Card AOM-BINDING-POISON. The refusal itself is unchanged — the
            // `Unsupported` diagnostic above still stands, still poisons the
            // path and still makes this function `Unknown` — and so is every
            // resource state: `state.resources` is not touched here and never
            // was. What is bounded is the collateral damage to *bindings*.
            //
            // `Unbounded` is the pre-repair behaviour and the answer for every
            // refusal that has no complete expression or statement text. A
            // `Sparing` refusal preserves only plain automatic names of this
            // body, never address-taken, that the refused text cannot write; see [`RefusedWrites`].
            for (place, pointer) in state.pointers.iter_mut() {
                if writes.poisons(place) {
                    pointer.unknown = true;
                    pointer.may_be_null = true;
                }
            }
        }
        LegacyInstruction::NoOp => {}
        // Card W43-INDEXED-RESOURCE-COLLECTIONS: decided by [`transfer`]
        // before this function is reached; as a base instruction a counted
        // loop's header is a `NoOp` and its step a forget.
        LegacyInstruction::CountedLoopHeader { .. } => {}
        LegacyInstruction::CountedLoopStep { index } => {
            state.pointers.remove(index);
        }
        LegacyInstruction::ElementStore { .. }
        | LegacyInstruction::ElementRelease { .. }
        | LegacyInstruction::BranchOnElementNull { .. } => {
            unreachable!("element operations are transferred by `transfer`")
        }
        // Card W45-SUB-OBJECT-STORES: decided by [`transfer`].
        LegacyInstruction::SubObjectStore { .. } => {
            unreachable!("sub-object stores are transferred by `transfer`")
        }
        // Card CG11-POINTER-VALUED-STORES: decided by [`transfer`].
        LegacyInstruction::PointerStore { .. } => {
            unreachable!("pointer stores are transferred by `transfer`")
        }
        LegacyInstruction::SelfInitCall { .. } | LegacyInstruction::TrackedSelfInit { .. } => {
            unreachable!("checked initializer calls are transferred by dispatch")
        }
    }
    // O04-B09B10: a custody-taking member read handed the linked child to the
    // callee, whose contract effect above released or consumed it. The member
    // now holds a value this projection must not denote again, so drop both the
    // place binding and the field link. A second read of the same member then
    // fails `bound_read_resolves` — conjunct 2 for the binding, conjunct 3 for
    // the link — instead of releasing the same identity twice, and an escape of
    // the aggregate no longer cascades to a child it no longer holds.
    if let LegacyInstruction::ContractCall {
        aggregate_consumed, ..
    } = &node.instruction
    {
        for place in aggregate_consumed {
            state.pointers.remove(place);
            let (Some(owner), Some(field)) = (
                crate::aggregate_store::place_owner(place),
                crate::aggregate_store::place_field(place),
            ) else {
                continue;
            };
            let Some(owner_site) = resolved_aggregate_owner(&state, owner, &context.caller_sites)
            else {
                continue;
            };
            if let Some(fields) = state.field_links.get_mut(&owner_site) {
                fields.remove(field);
                if fields.is_empty() {
                    state.field_links.remove(&owner_site);
                }
            }
        }
    }
    if let Some((owner_site, field, place)) = stored_field {
        // Card W43-LIBC-RESULT-ALIAS: a derived pointer stored into a member
        // would link the *base* under the aggregate, so escaping the aggregate
        // would cascade to an object the member does not own. Refuse the link.
        if let Some(pointer) = state
            .pointers
            .get(&place)
            .filter(|pointer| !pointer.derivation.is_identity())
            .cloned()
        {
            diagnostics.insert(LegacyDiagnostic {
                function: function.to_owned(),
                line: node.line,
                kind: LegacyDiagnosticKind::UnknownAlias {
                    variable: place.clone(),
                },
            });
            mark_unresolved_effect(&mut state, &pointer);
            state.pointers.remove(&place);
            if let Some(fields) = state.field_links.get_mut(&owner_site) {
                fields.remove(&field);
                if fields.is_empty() {
                    state.field_links.remove(&owner_site);
                }
            }
            return state;
        }
        let children = state
            .pointers
            .get(&place)
            .map(|pointer| pointer.targets.clone())
            .unwrap_or_default();
        let entry = state.field_links.entry(owner_site).or_default();
        if children.is_empty() {
            entry.remove(&field);
        } else {
            entry.insert(field.clone(), children);
        }
        if entry.is_empty() {
            state.field_links.remove(&owner_site);
        }
        // Card B08-FIELD-LINK-JOIN-SILENT-UNRESOLVED: the store holds here.
        field_link_join_route::stored(&mut state, owner_site, &field);
    }
    state
}

/// The admitted aggregate field place an instruction stores into, if any.
pub(super) fn written_field_place(instruction: &LegacyInstruction) -> Option<&str> {
    let place = match instruction {
        LegacyInstruction::AggregateStore { place } => place.as_str(),
        LegacyInstruction::Alias { destination, .. } | LegacyInstruction::Clear { destination } => {
            destination.as_str()
        }
        LegacyInstruction::ContractCall { destination, .. } => destination.as_deref()?,
        _ => return None,
    };
    crate::aggregate_store::is_field_place(place).then_some(place)
}

/// Whether an admitted bound read of `place` denotes, here and now, exactly the
/// identity an admitted store linked into that member (card O04-READ-BOUND).
///
/// Three conjuncts, all flow-sensitive and all fail-closed:
///
/// 1. the **owner** resolves through [`resolved_aggregate_owner`] — one live
///    identity this body acquired, never a caller-held, nullable or
///    unresolved-effect one. A read through a released or escaped aggregate can
///    therefore never be admitted, which is card C00's lesson in this direction;
/// 2. the **place** is bound to a definite, non-empty target set. A member this
///    body never stored into carries no identity to denote;
/// 3. the place's targets are **exactly** the field link the store recorded
///    under that owner. A binding that has drifted from its link — through a
///    path join, a refutation or any other route — is refused rather than
///    believed;
/// 4. every linked identity is itself **live** and carries no unresolved
///    effect. A member left pointing at an identity this body already released
///    or published is a dangling read, and admitting it is exactly the mistake
///    card C00 records: the whole point of premise 4 is that a read through a
///    released pointer must never reach `Safe`.
pub(super) fn bound_read_resolves(
    state: &AbstractState,
    place: &str,
    caller_sites: &BTreeSet<usize>,
) -> bool {
    let (Some(owner), Some(field)) = (
        crate::aggregate_store::place_owner(place),
        crate::aggregate_store::place_field(place),
    ) else {
        return false;
    };
    let Some(owner_site) = resolved_aggregate_owner(state, owner, caller_sites) else {
        return false;
    };
    let Some(pointer) = state.pointers.get(place) else {
        return false;
    };
    if pointer.unknown || pointer.targets.is_empty() || !pointer.derivation.is_identity() {
        return false;
    }
    if state
        .field_links
        .get(&owner_site)
        .and_then(|fields| fields.get(field))
        != Some(&pointer.targets)
    {
        return false;
    }
    // Card B08-FIELD-LINK-JOIN-SILENT-UNRESOLVED: a link that may not hold
    // on every path denotes no identity on every path.
    if field_link_join_route::partial_field(state, owner_site, field) {
        return false;
    }
    pointer.targets.iter().all(|site| {
        state.resources.get(site) == Some(&LIVE) && !state.unresolved_effect_sites.contains(site)
    })
}

/// The single live identity an admitted aggregate base denotes here, when the
/// projection can name exactly one and this body acquired it.
pub(super) fn resolved_aggregate_owner(
    state: &AbstractState,
    owner: &str,
    caller_sites: &BTreeSet<usize>,
) -> Option<usize> {
    let pointer = state.pointers.get(owner)?;
    // Card W43-LIBC-RESULT-ALIAS: a derived pointer is not the aggregate's own
    // identity, so it resolves no owner.
    if pointer.unknown
        || pointer.may_be_null
        || pointer.targets.len() != 1
        || !pointer.derivation.is_identity()
    {
        return None;
    }
    let site = pointer.targets.first().copied()?;
    if caller_sites.contains(&site)
        || state.nullable_sites.contains(&site)
        || state.unresolved_effect_sites.contains(&site)
    {
        return None;
    }
    (state.resources.get(&site) == Some(&LIVE)).then_some(site)
}

/// Whether a scalar member-store base denotes one identity that is safe to
/// dereference at this point.  Caller provenance is immaterial: unlike an
/// aggregate link store, this instruction neither creates nor transfers an
/// identity.  Nullability, ambiguity, release/escape and unresolved effects
/// all refuse.
pub(super) fn scalar_member_store_base_resolves(state: &AbstractState, base: &str) -> bool {
    let Some(pointer) = state.pointers.get(base) else {
        return false;
    };
    // Card SLOT-MOVE-AND-INTERIOR-BORROW (rule 12y): a store through a
    // non-null borrow is a write to its parent, admitted where the parent is
    // the caller's object (no target) or one live identity.
    if matches!(
        pointer.derivation,
        Derivation::Derived {
            extent: DerivedExtent::Borrowed,
            ..
        }
    ) && !pointer.may_be_null
    {
        return match pointer.targets.iter().copied().collect::<Vec<_>>()[..] {
            [] => pointer.unknown,
            [site] => {
                !pointer.unknown
                    && !state.nullable_sites.contains(&site)
                    && !state.unresolved_effect_sites.contains(&site)
                    && state.resources.get(&site) == Some(&LIVE)
            }
            _ => false,
        };
    }
    // Card W43-LIBC-RESULT-ALIAS: nor is a derived pointer a store base.
    if pointer.unknown
        || pointer.may_be_null
        || pointer.targets.len() != 1
        || !pointer.derivation.is_identity()
    {
        return false;
    }
    let Some(site) = pointer.targets.first() else {
        return false;
    };
    !state.nullable_sites.contains(site)
        && !state.unresolved_effect_sites.contains(site)
        && state.resources.get(site) == Some(&LIVE)
}

/// Card **W43-INDIRECT-ACCESS-GENERAL**: whether a typed scalar access may
/// dereference `base` at this point. Exactly three bindings are admitted:
///
/// 1. **no binding on any reaching path.** No modeled statement ever bound the
///    name, so the object it designates is none this projection tracks: a
///    formal's caller-provided pointer, an automatic array, or a name whose only
///    binding statement was itself refused, which already keeps the function
///    `Unknown`. Its nullness and extent are outside the property, exactly as
///    they are for an unbound name at a `Borrowed` position;
/// 2. **a non-null binding with no tracked target**: an untracked or
///    derived-untracked pointer after its null test, or an opaque member value
///    after its null test. The only other way to lose every target, widening,
///    is itself refused wherever it happens;
/// 3. **exactly one known, non-null, live identity** with neither a nullable
///    nor an unresolved-effect marker, identity or derived alike: a derived
///    pointer dereferences its base's allocation.
///
/// A possibly-null binding (it must be guarded), a mixed derivation, an
/// ambiguous, released, escaped or unresolved identity, and an unknown binding
/// that still carries a tracked target all refuse.
pub(super) fn indirect_scalar_base_resolves(state: &AbstractState, base: &str) -> bool {
    let Some(pointer) = state.pointers.get(base) else {
        return true;
    };
    if pointer.may_be_null || matches!(pointer.derivation, Derivation::Mixed) {
        return false;
    }
    if pointer.targets.is_empty() {
        return pointer.unknown;
    }
    if pointer.unknown || pointer.targets.len() != 1 {
        return false;
    }
    let Some(site) = pointer.targets.first() else {
        return false;
    };
    !state.nullable_sites.contains(site)
        && !state.unresolved_effect_sites.contains(site)
        && state.resources.get(site) == Some(&LIVE)
}

/// Card **W44-INDIRECT-CHAIN-ACCESS**: the read-only view of one state the
/// chain obligation decides on. It reads bindings, statuses, markers and field
/// links and changes nothing.
impl crate::indirect_chain_access::ChainView for AbstractState {
    fn binding(&self, place: &str) -> Option<crate::indirect_chain_access::LinkBinding<'_>> {
        self.pointers
            .get(place)
            .map(|pointer| crate::indirect_chain_access::LinkBinding {
                targets: &pointer.targets,
                unknown: pointer.unknown,
                may_be_null: pointer.may_be_null,
                identity: pointer.derivation.is_identity(),
            })
    }

    fn exactly_live(&self, site: usize) -> bool {
        !self.nullable_sites.contains(&site)
            && !self.unresolved_effect_sites.contains(&site)
            && self.resources.get(&site) == Some(&LIVE)
    }

    // Card CG10-BOUND-UNTRACKED-ROOTS (rule 11d): the per-path premise, read
    // off this state (`legacy_bound_untracked.rs`).
    fn bound_untracked(&self, root: &str) -> bool {
        bound_untracked_route::premise(self, root, true) == crate::bound_untracked::Premise::Holds
    }

    fn field_link(&self, owner: usize, field: &str) -> Option<&BTreeSet<usize>> {
        // Card B08-FIELD-LINK-JOIN-SILENT-UNRESOLVED: a link that may not
        // hold on every path is a link that equals no binding, so the chain
        // refuses at it rather than reading it as untracked.
        static NO_CHILD: BTreeSet<usize> = BTreeSet::new();
        if field_link_join_route::partial_field(self, owner, field) {
            return Some(&NO_CHILD);
        }
        self.field_links.get(&owner)?.get(field)
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn apply_resource_action(
    function: &str,
    line: usize,
    source: &str,
    state: &mut AbstractState,
    acquisition_lines: &BTreeMap<usize, usize>,
    diagnostics: &mut BTreeSet<LegacyDiagnostic>,
    escape: bool,
    nullable: bool,
) {
    let pointer = state
        .pointers
        .get(source)
        .cloned()
        .unwrap_or(AbstractPointer {
            targets: BTreeSet::new(),
            unknown: true,
            may_be_null: true,
            derivation: Derivation::Identity,
        });
    // Card W43-LIBC-RESULT-ALIAS: never release, escape or consume a base
    // through a pointer derived from it.
    if !pointer.derivation.is_identity() {
        let mode = if escape {
            ResourceParameterMode::Consumed { nullable }
        } else {
            ResourceParameterMode::Released { nullable }
        };
        derived_custody_refusal(function, line, source, &pointer, mode, state, diagnostics);
        return;
    }
    let unresolved = pointer.unknown
        || pointer.targets.len() > 1
        || (!nullable && (pointer.targets.is_empty() || pointer.may_be_null));
    if unresolved {
        diagnostics.insert(LegacyDiagnostic {
            function: function.to_owned(),
            line,
            kind: LegacyDiagnosticKind::UnknownAlias {
                variable: source.to_owned(),
            },
        });
    }
    let unresolved_nullability = !nullable
        && pointer
            .targets
            .iter()
            .any(|site| state.nullable_sites.contains(site));
    if unresolved_nullability {
        diagnostics.insert(LegacyDiagnostic {
            function: function.to_owned(),
            line,
            kind: LegacyDiagnosticKind::Unsupported {
                description: format!("nullable resource `{source}` used by a non-null contract"),
            },
        });
    }
    if unresolved || unresolved_nullability {
        // One source action cannot discharge every possible selected target.
        // Preserve the residual obligations when target identity is unknown.
        mark_unresolved_effect(state, &pointer);
        return;
    }
    if !cascade_is_unambiguous(state, &pointer.targets) {
        diagnostics.insert(LegacyDiagnostic {
            function: function.to_owned(),
            line,
            kind: LegacyDiagnosticKind::UnknownAlias {
                variable: source.to_owned(),
            },
        });
        mark_unresolved_effect(state, &pointer);
        return;
    }
    for site in pointer.targets {
        let status = state.resources.get(&site).copied().unwrap_or(0);
        // Card W45-FB-RETRY-LIFETIME: a published object belongs to its
        // container; transferring it again is refused, not accused. A release
        // of it keeps the `release_after_escape` report below.
        if escape
            && container_publication::refuse_escape_of_published(
                function,
                line,
                source,
                site,
                state,
                diagnostics,
            )
        {
            let published = AbstractPointer {
                targets: BTreeSet::from([site]),
                ..AbstractPointer::default()
            };
            mark_unresolved_effect(state, &published);
            state.unmodeled = true;
            continue;
        }
        // Card CALLEE-RETENTION-CLASS1 (rule 12t): a pointer a live holder
        // retains is neither released nor handed on; the refusal is a hazard,
        // never an accusation.
        if retained_by_route::refuse_release_of_held(
            function,
            line,
            source,
            site,
            state,
            diagnostics,
        ) || super::descriptor_cells_route::refuse_release_of_holder(
            function,
            line,
            source,
            site,
            state,
            diagnostics,
        ) {
            let held = AbstractPointer {
                targets: BTreeSet::from([site]),
                ..AbstractPointer::default()
            };
            mark_unresolved_effect(state, &held);
            state.unmodeled = true;
            continue;
        }
        // Card FB-TEARDOWN-DESTROY-DECLARATIONS (rule 30 at a direct call):
        // an object a call took through a path it left stale is not the
        // caller's to release or hand on again; the refusal is a hazard,
        // never an accusation.
        if member_path_route::refuse_release_of_taken(
            function,
            line,
            source,
            site,
            state,
            diagnostics,
        ) {
            let held = AbstractPointer {
                targets: BTreeSet::from([site]),
                ..AbstractPointer::default()
            };
            mark_unresolved_effect(state, &held);
            state.unmodeled = true;
            continue;
        }
        // Card CG22-CALLBACK-REGISTRY-CONTRACT (rule 13x): a holder whose
        // registry owns elements is released only through the registry.
        if !escape
            && callback_route::refuse_release_of_registry(
                function,
                line,
                source,
                site,
                state,
                diagnostics,
            )
        {
            let held = AbstractPointer {
                targets: BTreeSet::from([site]),
                ..AbstractPointer::default()
            };
            mark_unresolved_effect(state, &held);
            state.unmodeled = true;
            continue;
        }
        if escape && status != LIVE {
            diagnostics.insert(LegacyDiagnostic {
                function: function.to_owned(),
                line,
                kind: LegacyDiagnosticKind::CustodyConflict {
                    acquisition_line: acquisition_line(acquisition_lines, site),
                },
            });
            continue;
        }
        if status & CONTAINED != 0 {
            diagnostics.insert(LegacyDiagnostic {
                function: function.to_owned(),
                line,
                kind: LegacyDiagnosticKind::CustodyConflict {
                    acquisition_line: acquisition_line(acquisition_lines, site),
                },
            });
        }
        if escape {
            state
                .resources
                .insert(site, (status & !(LIVE | CONTAINED)) | ESCAPED);
        } else {
            if status & RELEASED != 0 {
                diagnostics.insert(LegacyDiagnostic {
                    function: function.to_owned(),
                    line,
                    kind: LegacyDiagnosticKind::DoubleRelease {
                        acquisition_line: acquisition_line(acquisition_lines, site),
                    },
                });
            }
            if status & ESCAPED != 0 {
                diagnostics.insert(LegacyDiagnostic {
                    function: function.to_owned(),
                    line,
                    kind: LegacyDiagnosticKind::ReleaseAfterEscape {
                        acquisition_line: acquisition_line(acquisition_lines, site),
                    },
                });
            }
            state
                .resources
                .insert(site, (status & !(LIVE | CONTAINED)) | RELEASED);
            // Card CALLEE-RETENTION-CLASS1 (rule 12t): a released holder
            // holds nothing any more.
            retained_by_route::holder_released(state, site);
        }
        // Card B08-FIELD-LINK-JOIN-SILENT-UNRESOLVED: the escape census, and
        // the refusal of an escape that does not carry a possibly-live child
        // on every path.
        if escape {
            field_link_join_route::record_escape_exposure(function, line, state, site);
        }
        let refused = cascade_resource_action(state, site, escape);
        field_link_join_route::report_escape_refusal(function, line, &refused, diagnostics);
    }
}

pub(super) fn mark_unresolved_effect(state: &mut AbstractState, pointer: &AbstractPointer) {
    let mut pending = if pointer.unknown {
        state.resources.keys().copied().collect::<BTreeSet<_>>()
    } else {
        pointer.targets.clone()
    };
    let mut visited = BTreeSet::new();
    while let Some(site) = pending.pop_first() {
        if visited.insert(site) {
            state.unresolved_effect_sites.insert(site);
            if let Some(children) = state.contained.get(&site) {
                pending.extend(children);
            }
            if let Some(fields) = state.field_links.get(&site) {
                pending.extend(fields.values().flatten().copied());
            }
        }
    }
}

pub(super) fn cascade_is_unambiguous(state: &AbstractState, roots: &BTreeSet<usize>) -> bool {
    let mut pending = roots.clone();
    let mut visited = BTreeSet::new();
    while let Some(owner) = pending.pop_first() {
        if !visited.insert(owner) {
            return false;
        }
        for child in state.contained.get(&owner).into_iter().flatten() {
            if state.resources.get(child) != Some(&CONTAINED)
                || state
                    .contained
                    .iter()
                    .any(|(other, children)| *other != owner && children.contains(child))
            {
                return false;
            }
            pending.insert(*child);
        }
    }
    true
}

/// Returns the linked children an escape refused (card
/// B08-FIELD-LINK-JOIN-SILENT-UNRESOLVED); the caller reports them.
pub(super) fn cascade_resource_action(
    state: &mut AbstractState,
    owner: usize,
    escape: bool,
) -> BTreeSet<usize> {
    let mut pending = state.contained.remove(&owner).unwrap_or_default();
    let mut visited = BTreeSet::new();
    while let Some(site) = pending.pop_first() {
        if !visited.insert(site) {
            continue;
        }
        if let Some(children) = state.contained.remove(&site) {
            pending.extend(children);
        }
        let status = state.resources.get(&site).copied().unwrap_or(0);
        let terminal = if escape { ESCAPED } else { RELEASED };
        state
            .resources
            .insert(site, (status & !(LIVE | CONTAINED)) | terminal);
    }
    if escape {
        cascade_field_links_on_escape(state, owner)
    } else {
        BTreeSet::new()
    }
}

/// B08: publishing an aggregate publishes the resources its admitted members
/// link, so a live linked child escapes with its owner rather than being
/// reported as a leak. A child in any other state is left exactly as it is and
/// recorded as an unresolved effect: a member still pointing at a released
/// identity when its aggregate escapes is a hazard this projection will not
/// decide in either direction.
///
/// Card **B08-FIELD-LINK-JOIN-SILENT-UNRESOLVED** (rule 11c): a child that
/// may be live and is not linked on every path, or whose status is mixed, is
/// not escaped; it keeps its status with an unresolved effect and is
/// returned, so the caller reports the refusal. Before the card such a child
/// was escaped, or left unresolved, silently. See
/// [`field_link_join_route::escape_cascade`].
///
/// **Releasing** an owner deliberately does not reach this function
/// (premise P-B08-SHALLOW): a declared `resource_released` release names one
/// identity, and a plain C field store carries no declared destructor
/// obligation, so a child still live there stays live and is reported.
pub(super) fn cascade_field_links_on_escape(
    state: &mut AbstractState,
    owner: usize,
) -> BTreeSet<usize> {
    field_link_join_route::escape_cascade(state, owner)
}

#[allow(clippy::too_many_arguments)]
pub(super) fn apply_contract_call(
    function: &str,
    line: usize,
    called: &str,
    destination: Option<&str>,
    arguments: &[Option<String>],
    contract: &ResourceFunctionContract,
    produced_sites: &[usize],
    state: &mut AbstractState,
    acquisition_lines: &BTreeMap<usize, usize>,
    kinds: &BTreeMap<usize, ResourceKind>,
    diagnostics: &mut BTreeSet<LegacyDiagnostic>,
) {
    // A restored result needs exact membership before any parameter effect.
    // Snapshot its binding now, including when the destination aliases an actual.
    let restored = if let ResourceReturnMode::Removed {
        parameter,
        nullable,
    } = contract.result
    {
        match super::detach_route::prepare(
            function,
            line,
            called,
            arguments,
            contract,
            parameter,
            nullable,
            state,
            diagnostics,
        ) {
            Some(pointer) => Some(pointer),
            None => return,
        }
    } else {
        None
    };
    // Card W43-LIBC-RESULT-ALIAS: a derived pointer owns nothing, so no
    // custody-bearing parameter effect is ever applied through one. Each such
    // position is skipped below; the one accusable shape (an interior pointer
    // handed to a release) is reported from the settled states by
    // `report_derived_pointer_violations`, and every other one refuses here.
    let mut derived_positions = BTreeSet::new();
    for (index, mode) in contract.parameters.iter().copied().enumerate() {
        if matches!(
            mode,
            ResourceParameterMode::Borrowed | ResourceParameterMode::Out { .. }
        ) {
            continue;
        }
        let Some(source) = arguments.get(index).and_then(Option::as_deref) else {
            continue;
        };
        let Some(pointer) = state.pointers.get(source).cloned() else {
            continue;
        };
        if pointer.derivation.is_identity() {
            continue;
        }
        derived_positions.insert(index);
        derived_custody_refusal(function, line, source, &pointer, mode, state, diagnostics);
    }
    if let ResourceReturnMode::Reallocated { parameter, .. } = contract.result
        && derived_positions.contains(&parameter)
    {
        return;
    }
    // Reallocation consumes at most one proven live original on success.
    // A null old value carries no custody only when its input contract permits it.
    // Validate it before any other parameter effect or result allocation.
    if let ResourceReturnMode::Reallocated { parameter, .. } = contract.result {
        let input_nullable = matches!(
            contract.parameters.get(parameter),
            Some(ResourceParameterMode::Owned { nullable: true })
        );
        let source = arguments.get(parameter).and_then(Option::as_deref);
        let old = source
            .and_then(|source| state.pointers.get(source))
            .cloned();
        let valid = old.as_ref().is_some_and(|old| {
            !old.unknown
                && old.targets.len() <= 1
                // Card W43-DESCRIPTOR-RESOURCE-TRACKING: a replacement relation
                // is specified for the declared kind only.
                && old
                    .targets
                    .iter()
                    .all(|site| kind_of(kinds, *site) == contract.parameter_kind(parameter))
                && (input_nullable || (old.targets.len() == 1 && !old.may_be_null))
                && old.targets.iter().all(|site| {
                    state.resources.get(site) == Some(&LIVE)
                        && (input_nullable || !state.nullable_sites.contains(site))
                        && !state.unresolved_effect_sites.contains(site)
                })
        });
        if !valid {
            diagnostics.insert(LegacyDiagnostic {
                function: function.to_owned(),
                line,
                kind: LegacyDiagnosticKind::UnknownAlias {
                    variable: source.unwrap_or(called).to_owned(),
                },
            });
            mark_unresolved_effect(
                state,
                &old.unwrap_or(AbstractPointer {
                    unknown: true,
                    ..Default::default()
                }),
            );
            return;
        }
    }
    let mut sites = produced_sites.iter().copied();
    // Apply input custody effects before assigning a return value. This is
    // essential when the destination aliases a consumed/reallocated argument.
    for (index, mode) in contract.parameters.iter().copied().enumerate() {
        let source = arguments.get(index).and_then(Option::as_deref);
        if matches!(contract.result, ResourceReturnMode::Removed { parameter, .. } if parameter == index)
            && restored
                .as_ref()
                .is_some_and(|pointer| pointer.targets.is_empty())
        {
            // Checked null item: no removal, no ownership transfer.
            continue;
        }
        if derived_positions.contains(&index) {
            continue;
        }
        // Card W43-DESCRIPTOR-RESOURCE-TRACKING: a custody-bearing position
        // touches only an obligation of its own kind. A release of another
        // kind discharges nothing and is accused from the settled states; any
        // other mismatched custody position refuses.
        if !matches!(
            mode,
            ResourceParameterMode::Borrowed | ResourceParameterMode::Out { .. }
        ) && let Some(source) = source
            && kind_mismatch(state, source, contract.parameter_kind(index), kinds)
        {
            refuse_kind_mismatch(function, line, source, mode, state, diagnostics);
            continue;
        }
        // An aggregate owner that is itself a derived pointer relates the child
        // to an object the owner does not own.
        if let ResourceParameterMode::Stored {
            owner_parameter, ..
        }
        | ResourceParameterMode::Removed {
            owner_parameter, ..
        } = mode
            && derived_positions.contains(&owner_parameter)
        {
            if let Some(source) = source {
                let child = state
                    .pointers
                    .get(source)
                    .cloned()
                    .unwrap_or(AbstractPointer {
                        unknown: true,
                        ..AbstractPointer::default()
                    });
                diagnostics.insert(LegacyDiagnostic {
                    function: function.to_owned(),
                    line,
                    kind: LegacyDiagnosticKind::UnknownAlias {
                        variable: source.to_owned(),
                    },
                });
                mark_unresolved_effect(state, &child);
            }
            continue;
        }
        match mode {
            ResourceParameterMode::Borrowed | ResourceParameterMode::Out { .. } => {}
            ResourceParameterMode::Owned { nullable } => {
                let Some(source) = source else {
                    report_unsupported_contract(function, line, called, diagnostics);
                    continue;
                };
                require_live_resource(function, line, source, nullable, state, diagnostics);
            }
            ResourceParameterMode::Consumed { nullable } => {
                let Some(source) = source else {
                    report_unsupported_contract(function, line, called, diagnostics);
                    continue;
                };
                apply_resource_action(
                    function,
                    line,
                    source,
                    state,
                    acquisition_lines,
                    diagnostics,
                    true,
                    nullable,
                );
            }
            ResourceParameterMode::Released { nullable } => {
                let Some(source) = source else {
                    report_unsupported_contract(function, line, called, diagnostics);
                    continue;
                };
                apply_resource_action(
                    function,
                    line,
                    source,
                    state,
                    acquisition_lines,
                    diagnostics,
                    false,
                    nullable,
                );
            }
            ResourceParameterMode::Stored {
                owner_parameter,
                nullable,
            } => {
                let owner = arguments.get(owner_parameter).and_then(Option::as_deref);
                let (Some(source), Some(owner)) = (source, owner) else {
                    report_unsupported_contract(function, line, called, diagnostics);
                    continue;
                };
                attach_contained_resource(
                    function,
                    line,
                    owner,
                    source,
                    nullable,
                    state,
                    acquisition_lines,
                    diagnostics,
                );
            }
            ResourceParameterMode::Removed {
                owner_parameter,
                nullable,
            } => {
                let owner = arguments.get(owner_parameter).and_then(Option::as_deref);
                let (Some(source), Some(owner)) = (source, owner) else {
                    report_unsupported_contract(function, line, called, diagnostics);
                    continue;
                };
                detach_contained_resource(
                    function,
                    line,
                    owner,
                    source,
                    nullable,
                    state,
                    acquisition_lines,
                    diagnostics,
                );
            }
        }
    }

    match contract.result {
        ResourceReturnMode::Removed { .. } => {
            if let (Some(destination), Some(pointer)) = (destination, restored) {
                if is_indirect_place(destination) {
                    report_unsupported_contract(function, line, called, diagnostics);
                    mark_unresolved_effect(state, &pointer);
                } else {
                    state.pointers.insert(destination.to_owned(), pointer);
                }
            }
        }
        ResourceReturnMode::None => {
            // The result carries no custody, so the destination holds no
            // resource afterwards. Dropping the binding — rather than leaving
            // the previous one in place — keeps a stale alias from covering for
            // a resource the assignment actually dropped.
            if let Some(destination) = destination {
                state.pointers.remove(destination);
            }
        }
        ResourceReturnMode::Owned { nullable } => {
            let (Some(destination), Some(site)) = (destination, sites.next()) else {
                report_unsupported_contract(function, line, called, diagnostics);
                return;
            };
            create_resource(state, destination, site, nullable);
            // Card REFCOUNT-RESOURCE-KIND (rule 13b): a reference-returning
            // call starts an acquired family.
            if contract.result_kind == ResourceKind::Reference {
                reference_route::acquired(state, site);
            }
        }
        ResourceReturnMode::Reallocated {
            parameter,
            nullable,
        } => {
            let source = arguments.get(parameter).and_then(Option::as_deref);
            let (Some(destination), Some(source), Some(site)) = (destination, source, sites.next())
            else {
                report_unsupported_contract(function, line, called, diagnostics);
                return;
            };
            let Some(old) = state.pointers.get(source).cloned() else {
                diagnostics.insert(LegacyDiagnostic {
                    function: function.to_owned(),
                    line,
                    kind: LegacyDiagnosticKind::UnknownAlias {
                        variable: source.to_owned(),
                    },
                });
                return;
            };
            let input_nullable = matches!(
                contract.parameters.get(parameter),
                Some(ResourceParameterMode::Owned { nullable: true })
            );
            if old.unknown || old.targets.len() > 1 || (old.targets.is_empty() && !input_nullable) {
                diagnostics.insert(LegacyDiagnostic {
                    function: function.to_owned(),
                    line,
                    kind: LegacyDiagnosticKind::UnknownAlias {
                        variable: source.to_owned(),
                    },
                });
                mark_unresolved_effect(state, &old);
                return;
            }
            create_resource(state, destination, site, nullable);
            state
                .conditional_replacements
                .insert(site, old.targets.clone());
            if !nullable {
                finalize_replacement_success(state, site);
            }
        }
        ResourceReturnMode::Derived {
            parameter,
            extent,
            nullable,
            const_dropped,
        } => {
            // A discarded derived result binds nothing and owns nothing, which
            // is exactly a discarded `None`. One written through an out slot
            // is refused, as a derived `*out = p;` is in `transfer`.
            if let Some(destination) = destination.filter(|place| is_indirect_place(place)) {
                let base = arguments
                    .get(parameter)
                    .and_then(Option::as_deref)
                    .and_then(|base| state.pointers.get(base))
                    .cloned()
                    .unwrap_or(AbstractPointer {
                        unknown: true,
                        ..AbstractPointer::default()
                    });
                diagnostics.insert(LegacyDiagnostic {
                    function: function.to_owned(),
                    line,
                    kind: LegacyDiagnosticKind::UnknownAlias {
                        variable: destination.to_owned(),
                    },
                });
                mark_unresolved_effect(state, &base);
            } else if let Some(destination) = destination {
                let base = arguments
                    .get(parameter)
                    .and_then(Option::as_deref)
                    .and_then(|base| state.pointers.get(base))
                    .cloned();
                let derived = derive_pointer(base.as_ref(), extent, nullable, const_dropped);
                state.pointers.insert(destination.to_owned(), derived);
            }
        }
    }

    for (index, mode) in contract.parameters.iter().copied().enumerate() {
        if let ResourceParameterMode::Out { nullable } = mode {
            let source = arguments.get(index).and_then(Option::as_deref);
            let (Some(destination), Some(site)) = (source, sites.next()) else {
                report_unsupported_contract(function, line, called, diagnostics);
                continue;
            };
            create_resource(state, destination, site, nullable);
        }
    }
}

/// The binding a declared derived result gives its destination (card
/// W43-LIBC-RESULT-ALIAS).
///
/// It shares the base binding's targets, so releasing the base invalidates it
/// through the ordinary site status, and it creates no site and changes no
/// resource. A base this projection does not track (an untracked name, a
/// string literal, the address of a plain name) gives an *unknown* derived
/// binding: nothing it could denote is resolved, so it supports no claim, and
/// it is still a derived binding, so it can never be escaped or released as if
/// it owned something. A derived pointer derived again stays derived: interior
/// unless both steps are exact, `const_dropped` if either step dropped it,
/// relatively `nullable` if either step may return NULL (card
/// W43-RESULT-ALIAS-RETURNED), and `Mixed` over a mixed base.
pub(super) fn derive_pointer(
    base: Option<&AbstractPointer>,
    extent: DerivedExtent,
    nullable: bool,
    const_dropped: bool,
) -> AbstractPointer {
    let Some(base) = base else {
        return AbstractPointer {
            targets: BTreeSet::new(),
            unknown: true,
            // Card SLOT-MOVE-AND-INTERIOR-BORROW (rule 12y): a borrowed
            // result's nullness is the declaration's own (§C04.3.1 rule 4).
            may_be_null: extent != DerivedExtent::Borrowed || nullable,
            derivation: Derivation::Derived {
                extent,
                const_dropped,
                nullable,
                stepped: Stepped::No,
            },
        };
    };
    let derivation = match base.derivation {
        Derivation::Identity => Derivation::Derived {
            extent,
            const_dropped,
            nullable,
            stepped: Stepped::No,
        },
        Derivation::Derived {
            extent: base_extent,
            const_dropped: base_dropped,
            nullable: base_nullable,
            // Card W43-INDEX-TRAVERSAL: a result derived from a stepped
            // pointer keeps its step, so it never joins an unstepped family
            // and a result that may sit at the object's start stays `Any`.
            stepped,
        } => Derivation::Derived {
            // Card SLOT-MOVE-AND-INTERIOR-BORROW (rule 12y): a pointer derived
            // from a borrow, or borrowed from a derived pointer, is a borrow.
            extent: if base_extent == DerivedExtent::Borrowed || extent == DerivedExtent::Borrowed {
                DerivedExtent::Borrowed
            } else if base_extent == DerivedExtent::Exact && extent == DerivedExtent::Exact {
                DerivedExtent::Exact
            } else {
                DerivedExtent::Interior
            },
            const_dropped: const_dropped || base_dropped,
            nullable: nullable || base_nullable,
            stepped,
        },
        Derivation::Mixed => Derivation::Mixed,
    };
    AbstractPointer {
        targets: base.targets.clone(),
        unknown: base.unknown,
        // Card SLOT-MOVE-AND-INTERIOR-BORROW (rule 12y): a borrowed result's
        // nullness is the declaration's own (§C04.3.1 rule 4).
        may_be_null: if matches!(
            derivation,
            Derivation::Derived {
                extent: DerivedExtent::Borrowed,
                ..
            }
        ) {
            nullable
        } else {
            base.may_be_null || nullable
        },
        derivation,
    }
}

/// Whether a custody-bearing position may *accuse* a derived pointer rather
/// than refuse: exactly an interior pointer that resolves to one identity,
/// handed to a release. Freeing a pointer into an object is not freeing the
/// object (C11 7.22.3.3p2 requires the value an allocator returned). An exact
/// alias is the base's own value, so releasing it is a release of the base
/// that this projection declines to credit, and is refused instead.
pub(super) fn derived_release_is_accusable(
    mode: ResourceParameterMode,
    pointer: &AbstractPointer,
) -> bool {
    // Card W43-INDEX-TRAVERSAL: a stepped pointer that may sit at its
    // object's start (`Stepped::Any`) may be the value the allocator
    // returned, so its release is refused rather than accused.
    matches!(mode, ResourceParameterMode::Released { .. })
        && matches!(
            pointer.derivation,
            Derivation::Derived {
                extent: DerivedExtent::Interior,
                stepped: Stepped::No | Stepped::Forward,
                ..
            }
        )
        && !pointer.unknown
        && pointer.targets.len() == 1
}

/// A derived pointer at a custody-bearing position (card W43-LIBC-RESULT-ALIAS).
///
/// The effect is never applied: the base's obligation stays exactly as it was.
/// The accusable shape changes nothing here and is reported from the settled
/// states. Every other shape refuses: an `UnknownAlias`, and the pointer's
/// targets carry an unresolved effect, so no residual that depends on this
/// statement is claimed either way.
pub(super) fn derived_custody_refusal(
    function: &str,
    line: usize,
    source: &str,
    pointer: &AbstractPointer,
    mode: ResourceParameterMode,
    state: &mut AbstractState,
    diagnostics: &mut BTreeSet<LegacyDiagnostic>,
) {
    if derived_release_is_accusable(mode, pointer) {
        // Reported from the settled states. The path carries a violation
        // candidate, so its exits support no custody claim.
        state.unmodeled = true;
        return;
    }
    diagnostics.insert(LegacyDiagnostic {
        function: function.to_owned(),
        line,
        kind: LegacyDiagnosticKind::UnknownAlias {
            variable: source.to_owned(),
        },
    });
    mark_unresolved_effect(state, pointer);
}

/// Card W43-DESCRIPTOR-RESOURCE-TRACKING: whether `source` is bound to some
/// identity whose kind is not `expected`.
pub(super) fn kind_mismatch(
    state: &AbstractState,
    source: &str,
    expected: ResourceKind,
    kinds: &BTreeMap<usize, ResourceKind>,
) -> bool {
    state.pointers.get(source).is_some_and(|pointer| {
        pointer
            .targets
            .iter()
            .any(|site| kind_of(kinds, *site) != expected)
    })
}

/// Card W43-DESCRIPTOR-RESOURCE-TRACKING: a custody position met an identity
/// of another kind. The effect is never applied. A release that resolves to
/// one identity is left for the settled pass to accuse
/// ([`report_kind_violations`]), and the path carries a violation candidate;
/// every other shape refuses with `UnknownAlias`, and the identities carry an
/// unresolved effect so no residual depending on this call is claimed.
pub(super) fn refuse_kind_mismatch(
    function: &str,
    line: usize,
    source: &str,
    mode: ResourceParameterMode,
    state: &mut AbstractState,
    diagnostics: &mut BTreeSet<LegacyDiagnostic>,
) {
    let pointer = state.pointers.get(source).cloned().unwrap_or_default();
    if matches!(mode, ResourceParameterMode::Released { .. })
        && pointer.derivation.is_identity()
        && !pointer.unknown
        && pointer.targets.len() == 1
    {
        state.unmodeled = true;
        return;
    }
    diagnostics.insert(LegacyDiagnostic {
        function: function.to_owned(),
        line,
        kind: LegacyDiagnosticKind::UnknownAlias {
            variable: source.to_owned(),
        },
    });
    mark_unresolved_effect(state, &pointer);
}

/// Card W43-DESCRIPTOR-RESOURCE-TRACKING: refine one branch edge.
///
/// `None` for an instruction that is not a branch on a single name, and for a
/// null test of a name bound to an integer-carried identity, which refines
/// nothing (descriptor `0` is valid, so `if (fd)` says nothing about custody).
/// A validity test narrows exactly as a null test does, and only a binding that
/// resolves to one integer-carried identity: anything else is a refusal.
pub(super) fn refine_edge<'a>(
    instruction: &'a LegacyInstruction,
    edge: usize,
    state: &mut AbstractState,
    kinds: &BTreeMap<usize, ResourceKind>,
) -> Option<(&'a str, PointerRefinement)> {
    match instruction {
        LegacyInstruction::BranchOnNull {
            source,
            true_is_null,
        } => {
            let assume_null = if edge == 0 {
                *true_is_null
            } else {
                !*true_is_null
            };
            // Card MK-EVENT-DESCRIPTOR-QUANT (rule 38): a null test of a
            // cell-creating call's status witness decides its class.
            if super::descriptor_cells_route::refine(
                source,
                super::descriptor_cells_route::EdgeFact::Zero(assume_null),
                state,
            ) {
                return Some((source, PointerRefinement::Refined));
            }
            // Card CG25-RESOURCE-KINDS (rule 11v): a null test of an in-place
            // initialiser's status witness decides its storage: zero is
            // success (the obligation is certain), nonzero is failure (none).
            if let Some(storage) = in_place_custody::witnessed_storage(state, source) {
                return Some((source, refine_pointer_null(state, &storage, !assume_null)));
            }
            if carries_integer_identity(state, source, kinds) {
                return None;
            }
            Some((source, refine_pointer_null(state, source, assume_null)))
        }
        LegacyInstruction::BranchOnValidity {
            source,
            when_true,
            when_false,
        } => {
            let fact = descriptor_custody::edge_fact(*when_true, *when_false, edge);
            // Card MK-EVENT-DESCRIPTOR-QUANT (rule 38): a validity test of a
            // cell-creating call's descriptor-valued witness decides its class.
            if state.cell_witnesses.contains_key(source.as_str()) {
                let valid = match fact {
                    ValidityFact::Valid => Some(true),
                    ValidityFact::Invalid => Some(false),
                    ValidityFact::Unrefined => None,
                };
                if let Some(valid) = valid {
                    super::descriptor_cells_route::refine(
                        source,
                        super::descriptor_cells_route::EdgeFact::Valid(valid),
                        state,
                    );
                }
                return Some((source, PointerRefinement::Refined));
            }
            Some((source, refine_validity(state, source, fact, kinds)))
        }
        _ => None,
    }
}

/// Card W43-DESCRIPTOR-RESOURCE-TRACKING: narrow a descriptor's validity.
pub(super) fn refine_validity(
    state: &mut AbstractState,
    source: &str,
    fact: ValidityFact,
    kinds: &BTreeMap<usize, ResourceKind>,
) -> PointerRefinement {
    let assume_invalid = match fact {
        ValidityFact::Unrefined => return PointerRefinement::Refined,
        ValidityFact::Invalid => true,
        ValidityFact::Valid => false,
    };
    let Some(pointer) = state.pointers.get(source).cloned() else {
        // Nothing bound: an ordinary integer here, exactly as an untracked
        // name is in a null test.
        return PointerRefinement::Refined;
    };
    if pointer.targets.is_empty() && !pointer.unknown {
        // A number with no identity (after `fd = 0;`): nothing to narrow.
        return PointerRefinement::Refined;
    }
    if !pointer.derivation.is_identity()
        || pointer.unknown
        || pointer.targets.len() > 1
        || pointer
            .targets
            .iter()
            .any(|site| !kind_of(kinds, *site).is_integer())
    {
        return PointerRefinement::Unknown;
    }
    refine_pointer_null(state, source, assume_invalid)
}

/// Card **W43-DESCRIPTOR-RESOURCE-TRACKING**: the two kind violation
/// candidates, read from the settled incoming states for constraint 6's
/// reason — a binding's targets can still grow on a later iterate.
///
/// * A release whose actual resolves to exactly one identity of another kind:
///   `KindMismatchedRelease`. `refuse_kind_mismatch` already withheld the
///   effect, so the identity's own obligation stays open.
/// * A non-heap identity that resolves to exactly one site carrying the
///   `RELEASED` bit, used at a `Borrowed` position: `UseAfterClose` — the
///   `DoubleRelease` reading of the same bit. The heap kind keeps the
///   pre-existing `ALIAS-LOCALS-BORROW-LIVENESS` refusal and is not accused.
///
/// Nothing here changes a state; it only reports.
pub(super) fn report_kind_violations(
    cfg: &LegacyCfg,
    incoming: &[Option<AbstractState>],
    acquisition_lines: &BTreeMap<usize, usize>,
    kinds: &BTreeMap<usize, ResourceKind>,
    diagnostics: &mut BTreeSet<LegacyDiagnostic>,
) {
    for (node_id, node) in cfg.nodes.iter().enumerate() {
        let Some(state) = incoming[node_id].as_ref() else {
            continue;
        };
        let mut uses: Vec<(ResourceParameterMode, ResourceKind, &str)> = Vec::new();
        match &node.instruction {
            LegacyInstruction::ContractCall {
                arguments,
                contract,
                nested,
                ..
            } => {
                let calls = std::iter::once((arguments, contract)).chain(
                    nested
                        .iter()
                        .map(|nested| (&nested.arguments, &nested.contract)),
                );
                for (arguments, contract) in calls {
                    for (index, (mode, argument)) in
                        contract.parameters.iter().zip(arguments).enumerate()
                    {
                        if let Some(place) = argument.as_deref() {
                            uses.push((*mode, contract.parameter_kind(index), place));
                        }
                    }
                }
            }
            LegacyInstruction::Release { source } => uses.push((
                ResourceParameterMode::Released { nullable: false },
                ResourceKind::Heap,
                source,
            )),
            _ => continue,
        }
        for (mode, expected, place) in uses {
            let Some(pointer) = state.pointers.get(place) else {
                continue;
            };
            if !pointer.derivation.is_identity() || pointer.unknown || pointer.targets.len() != 1 {
                continue;
            }
            let Some(site) = pointer.targets.first().copied() else {
                continue;
            };
            let acquired = kind_of(kinds, site);
            let kind = match mode {
                ResourceParameterMode::Released { .. } if acquired != expected => {
                    LegacyDiagnosticKind::KindMismatchedRelease {
                        acquisition_line: acquisition_line(acquisition_lines, site),
                        acquired,
                        released: expected,
                    }
                }
                // Card REFCOUNT-RESOURCE-KIND: a reference used after its
                // last drop is refused (rule 13b), never accused, as the
                // heap's use after release is.
                ResourceParameterMode::Borrowed
                    if !acquired.is_heap()
                        && acquired != ResourceKind::Reference
                        && state
                            .resources
                            .get(&site)
                            .is_some_and(|status| status & RELEASED != 0) =>
                {
                    LegacyDiagnosticKind::UseAfterClose {
                        acquisition_line: acquisition_line(acquisition_lines, site),
                        kind: acquired,
                    }
                }
                _ => continue,
            };
            diagnostics.insert(LegacyDiagnostic {
                function: cfg.function.clone(),
                line: node.line,
                kind,
            });
        }
    }
}

pub(super) fn create_resource(
    state: &mut AbstractState,
    destination: &str,
    site: usize,
    nullable: bool,
) {
    // Acquisition sites may repeat in loops; retain prior uncertainty rather
    // than assuming this abstract site is a fresh runtime generation.
    state.resources.insert(site, LIVE);
    if nullable {
        state.nullable_sites.insert(site);
    } else {
        // Re-executing a non-null acquisition site in a loop is a strong
        // update. A predecessor on which the destination did not yet exist
        // must not permanently poison the fresh result as nullable.
        state.nullable_sites.remove(&site);
    }
    state.pointers.insert(
        destination.to_owned(),
        AbstractPointer {
            targets: BTreeSet::from([site]),
            unknown: false,
            may_be_null: nullable,
            derivation: Derivation::Identity,
        },
    );
}

#[allow(clippy::too_many_arguments)]
pub(super) fn attach_contained_resource(
    function: &str,
    line: usize,
    owner: &str,
    source: &str,
    nullable: bool,
    state: &mut AbstractState,
    acquisition_lines: &BTreeMap<usize, usize>,
    diagnostics: &mut BTreeSet<LegacyDiagnostic>,
) {
    let owner_live = require_live_resource(function, line, owner, false, state, diagnostics);
    let child_live = require_live_resource(function, line, source, nullable, state, diagnostics);
    let Some(owners) = state.pointers.get(owner).cloned() else {
        return;
    };
    let Some(children) = state.pointers.get(source).cloned() else {
        return;
    };
    if !owner_live || !child_live {
        mark_unresolved_effect(state, &children);
        return;
    }
    if owners.unknown || children.unknown || owners.targets.len() != 1 || children.targets.len() > 1
    {
        diagnostics.insert(LegacyDiagnostic {
            function: function.to_owned(),
            line,
            kind: LegacyDiagnosticKind::UnknownAlias {
                variable: format!("{owner}/{source}"),
            },
        });
        mark_unresolved_effect(state, &children);
        return;
    }
    let owner_site = *owners.targets.first().expect("one owner target");
    for child in children.targets {
        let status = state.resources.get(&child).copied().unwrap_or(0);
        if status & LIVE == 0 || status & CONTAINED != 0 || child == owner_site {
            diagnostics.insert(LegacyDiagnostic {
                function: function.to_owned(),
                line,
                kind: LegacyDiagnosticKind::CustodyConflict {
                    acquisition_line: acquisition_line(acquisition_lines, child),
                },
            });
            continue;
        }
        state.resources.insert(child, (status & !LIVE) | CONTAINED);
        state.contained.entry(owner_site).or_default().insert(child);
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn detach_contained_resource(
    function: &str,
    line: usize,
    owner: &str,
    source: &str,
    nullable: bool,
    state: &mut AbstractState,
    acquisition_lines: &BTreeMap<usize, usize>,
    diagnostics: &mut BTreeSet<LegacyDiagnostic>,
) {
    let owner_live = require_live_resource(function, line, owner, false, state, diagnostics);
    let Some(owners) = state.pointers.get(owner).cloned() else {
        return;
    };
    let Some(children) = state.pointers.get(source).cloned() else {
        diagnostics.insert(LegacyDiagnostic {
            function: function.to_owned(),
            line,
            kind: LegacyDiagnosticKind::UnknownAlias {
                variable: source.to_owned(),
            },
        });
        return;
    };
    if !owner_live
        || owners.unknown
        || children.unknown
        || owners.targets.len() != 1
        // A call detaches one child, not every possible target of an alias
        // joined from different paths. Their correlation is not represented.
        || children.targets.len() > 1
        || (!nullable && (children.targets.is_empty() || children.may_be_null))
    {
        diagnostics.insert(LegacyDiagnostic {
            function: function.to_owned(),
            line,
            kind: LegacyDiagnosticKind::UnknownAlias {
                variable: format!("{owner}/{source}"),
            },
        });
        return;
    }
    let owner_site = *owners.targets.first().expect("one owner target");
    for child in children.targets {
        let containing_owners = state
            .contained
            .iter()
            .filter_map(|(owner, children)| children.contains(&child).then_some(*owner))
            .collect::<BTreeSet<_>>();
        let status = state.resources.get(&child).copied().unwrap_or(0);
        // Every represented path must still own this exact child through the
        // owner. A joined CONTAINED|LIVE state includes an already-detached
        // path; removing the CONTAINED bit would erase its repeated removal.
        // The union of two different possible owners is not proof that the
        // requested owner contains the child on every represented path.
        if containing_owners != BTreeSet::from([owner_site]) || status != CONTAINED {
            diagnostics.insert(LegacyDiagnostic {
                function: function.to_owned(),
                line,
                kind: LegacyDiagnosticKind::CustodyConflict {
                    acquisition_line: acquisition_line(acquisition_lines, child),
                },
            });
            continue;
        }
        if let Some(contained) = state.contained.get_mut(&owner_site) {
            contained.remove(&child);
            if contained.is_empty() {
                state.contained.remove(&owner_site);
            }
        }
        state.resources.insert(child, (status & !CONTAINED) | LIVE);
    }
}

pub(super) fn require_live_resource(
    function: &str,
    line: usize,
    source: &str,
    nullable: bool,
    state: &AbstractState,
    diagnostics: &mut BTreeSet<LegacyDiagnostic>,
) -> bool {
    let Some(pointer) = state.pointers.get(source) else {
        diagnostics.insert(LegacyDiagnostic {
            function: function.to_owned(),
            line,
            kind: LegacyDiagnosticKind::UnknownAlias {
                variable: source.to_owned(),
            },
        });
        return false;
    };
    // Card W43-LIBC-RESULT-ALIAS: a derived pointer is not the live identity
    // an `Owned` position or an aggregate owner requires.
    let invalid = pointer.unknown
        || !pointer.derivation.is_identity()
        || (!nullable && (pointer.targets.is_empty() || pointer.may_be_null))
        || (!nullable
            && pointer
                .targets
                .iter()
                .any(|site| state.nullable_sites.contains(site)))
        || pointer.targets.iter().any(|site| {
            state.resources.get(site) != Some(&LIVE) || state.unresolved_effect_sites.contains(site)
        });
    if invalid {
        diagnostics.insert(LegacyDiagnostic {
            function: function.to_owned(),
            line,
            kind: LegacyDiagnosticKind::UnknownAlias {
                variable: source.to_owned(),
            },
        });
    }
    !invalid
}

pub(super) fn report_unsupported_contract(
    function: &str,
    line: usize,
    called: &str,
    diagnostics: &mut BTreeSet<LegacyDiagnostic>,
) {
    diagnostics.insert(LegacyDiagnostic {
        function: function.to_owned(),
        line,
        kind: LegacyDiagnosticKind::Unsupported {
            description: format!("arguments do not match resource contract for `{called}`"),
        },
    });
}

pub(super) fn finalize_replacement_success(state: &mut AbstractState, site: usize) {
    state.nullable_sites.remove(&site);
    if let Some(replaced) = state.conditional_replacements.remove(&site) {
        for old in replaced {
            let status = state.resources.get(&old).copied().unwrap_or(0);
            state.resources.insert(old, (status & !LIVE) | RELEASED);
        }
    }
}

pub(super) enum PointerRefinement {
    Refined,
    Infeasible,
    Unknown,
}

/// Card **SCALAR-ROUTE-EXIT-EDGE**: the refusal a loop header reports when the
/// value route called its exit edge infeasible and the edge was retained
/// anyway. It is `Unsupported`, so [`classify_diagnostics`] reads `Unknown` —
/// the fail-closed answer for a path whose absence the analyzer cannot prove.
pub(crate) const LOOP_EXIT_EDGE_RETAINED: &str =
    "loop exit edge retained: a modeled value cannot remove it";

/// The null edge of a decided null test: `site` does not exist on it. Its
/// status, nullability, replacement, collection and incoming aggregate edges
/// go, every sibling obligation is retained, and every alias that named it
/// loses it and may be null. Shared by [`refine_pointer_null`] and card
/// SUP07-REFUSED-TEST-ACCUSATION's rule 12e (`refused_test::settle_absent_edge`),
/// so the two cannot drift apart.
pub(super) fn remove_null_site(state: &mut AbstractState, site: usize) {
    state.resources.remove(&site);
    // Card REFCOUNT-RESOURCE-KIND: a null reference holds no reference.
    state.references.remove(&site);
    state.nullable_sites.remove(&site);
    state.conditional_replacements.remove(&site);
    // Card W43-INDEXED-RESOURCE-COLLECTIONS: a null array has no elements.
    state.collections.remove(&site);
    // The null alternative has no child at this site. Remove precisely
    // its incoming aggregate edges, retaining every sibling obligation.
    state.contained.retain(|_, children| {
        children.remove(&site);
        !children.is_empty()
    });
    state.field_links.retain(|_, fields| {
        fields.retain(|_, children| {
            children.remove(&site);
            !children.is_empty()
        });
        !fields.is_empty()
    });
    for alias in state.pointers.values_mut() {
        if alias.targets.remove(&site) {
            alias.may_be_null = true;
        }
    }
}

pub(super) fn refine_pointer_null(
    state: &mut AbstractState,
    source: &str,
    assume_null: bool,
) -> PointerRefinement {
    let Some(pointer) = state.pointers.get(source).cloned() else {
        // The reduced importer has no scalar type environment. An untracked
        // identifier may be an ordinary integer condition, so leave it alone.
        return PointerRefinement::Refined;
    };
    // Card W43-LIBC-RESULT-ALIAS. A derived pointer's nullness says nothing
    // about its base: `strchr` returns NULL for a live base, and a null base is
    // outside every declared derived result. Narrow the derived binding alone
    // and keep every site and every other alias exactly as they were. Both
    // edges stay feasible. A binding that is derived on only some paths keeps
    // the identity refusal, because on the others a null test *would* decide
    // the site.
    match pointer.derivation {
        Derivation::Identity => {}
        Derivation::Derived { .. } => {
            let narrowed = state.pointers.get_mut(source).expect("existing binding");
            if assume_null {
                narrowed.targets.clear();
                narrowed.unknown = false;
                narrowed.may_be_null = true;
            } else {
                narrowed.may_be_null = false;
            }
            return PointerRefinement::Refined;
        }
        Derivation::Mixed => return PointerRefinement::Unknown,
    }
    if pointer.unknown || pointer.targets.len() > 1 {
        // Nullness is independent of identity. A guard narrows its own
        // binding, never another alias or a resource's existence. Keep the
        // identity refusal and every obligation on both edges.
        //
        // Card SUP07-REFUSED-TEST-ACCUSATION: a site the tested name is tied
        // to is settled on the null edge by rule 12e in the work list
        // (`refused_test::settle_absent_edge`), never here: this refinement
        // has no provenance, and withholding every nullable site the binding
        // may denote (MONKEY-VHOST-PROPERTY-CUSTODY's interim rule, replaced)
        // also withdrew accusations established on other paths.
        let narrowed = state.pointers.get_mut(source).expect("existing binding");
        if assume_null {
            narrowed.targets.clear();
            narrowed.unknown = false;
            narrowed.may_be_null = true;
        } else {
            narrowed.may_be_null = false;
        }
        return PointerRefinement::Unknown;
    }
    let Some(site) = pointer.targets.iter().next().copied() else {
        return if assume_null {
            PointerRefinement::Refined
        } else {
            PointerRefinement::Infeasible
        };
    };
    if !pointer.may_be_null && !state.nullable_sites.contains(&site) {
        return if assume_null {
            PointerRefinement::Infeasible
        } else {
            PointerRefinement::Refined
        };
    }
    if assume_null {
        remove_null_site(state, site);
    } else {
        finalize_replacement_success(state, site);
        for alias in state.pointers.values_mut() {
            // Card W43-LIBC-RESULT-ALIAS: a non-null base does not make a
            // pointer derived from it non-null.
            if alias.targets.contains(&site) && alias.derivation.is_identity() {
                alias.may_be_null = false;
            }
        }
    }
    PointerRefinement::Refined
}

pub(super) fn acquisition_line(acquisition_lines: &BTreeMap<usize, usize>, site: usize) -> usize {
    // Card CG04-LOOP-SITE-GENERATIONS: every generation of a site was acquired
    // at its line.
    acquisition_lines
        .get(&crate::loop_generations::base(site))
        .copied()
        .unwrap_or(0)
}

pub(super) fn report_leaks(
    function: &str,
    line: usize,
    state: &AbstractState,
    acquisition_lines: &BTreeMap<usize, usize>,
    // Caller-owned parameter resources are discharged by the caller; a live
    // formal at exit is the interface, not a leak.
    exempt: &BTreeSet<usize>,
    diagnostics: &mut BTreeSet<LegacyDiagnostic>,
) {
    for (&site, &status) in &state.resources {
        // Card REFCOUNT-RESOURCE-KIND (rule 13b): a reference family's
        // balance is `reference_route::report_exit`'s.
        if status & LIVE != 0
            && !exempt.contains(&site)
            && !state.unresolved_effect_sites.contains(&site)
            && !state.references.contains_key(&site)
        {
            diagnostics.insert(LegacyDiagnostic {
                function: function.to_owned(),
                line,
                kind: LegacyDiagnosticKind::Leak {
                    acquisition_line: acquisition_line(acquisition_lines, site),
                },
            });
        }
    }
}
