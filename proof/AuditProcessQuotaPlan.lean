import QCore.ProcessQuotaPlan

-- Card `P03-F`: the declared-limit-kind plan of the verifier host's execution
-- profiles. Model of `crates/qcore-verifier-host/src/process_quota.rs` and the
-- `enforced_limits` decision in `crates/qcore-verifier-host/src/lib.rs`.
--
-- What is proved here is the *plan*: which mechanism installs which declared
-- kind, that no declared kind is ever silently dropped, and that the
-- mechanism-availability override can only widen refusal. What is **not**
-- proved here, and is measured in `docs/reports/remaining-p03-f/README.md`
-- instead: that cgroup v2's `pids` controller actually bounds a process tree,
-- that Landlock and the namespace unshare confine what they claim, and that
-- these tables are transcriptions of the Rust tables — the latter is pinned on
-- the Rust side by `crates/qcore-verifier-host/tests/execution_quota.rs`.

-- The recorded default does not move, and installs neither mechanism.
#print axioms QCore.ProcessQuotaPlan.the_recorded_default_is_the_unenforcing_profile
#print axioms QCore.ProcessQuotaPlan.the_default_profile_installs_nothing
#print axioms QCore.ProcessQuotaPlan.exactly_one_profile_installs_a_quota

-- The per-UID `RLIMIT_NPROC` kind and the per-tree cgroup quota kind are
-- distinct and never take each other's role.
#print axioms QCore.ProcessQuotaPlan.the_per_uid_and_per_tree_kinds_are_never_confused
#print axioms QCore.ProcessQuotaPlan.the_two_mechanism_tables_are_disjoint
#print axioms QCore.ProcessQuotaPlan.kind_spellings_are_pairwise_distinct

-- Target qualification: off Linux nothing is enforceable by any profile.
#print axioms QCore.ProcessQuotaPlan.off_linux_no_profile_places_any_kind
#print axioms QCore.ProcessQuotaPlan.off_linux_nothing_is_enforceable
#print axioms QCore.ProcessQuotaPlan.off_linux_an_enforcing_profile_refuses_any_declaration

-- Which profile places what, including the deliberate pin update: the two
-- `setrlimit` tables stay equal and the enforceable sets differ by exactly the
-- quota kind.
#print axioms QCore.ProcessQuotaPlan.the_legacy_profile_places_nothing
#print axioms QCore.ProcessQuotaPlan.the_limit_profile_places_exactly_the_rlimit_table
#print axioms QCore.ProcessQuotaPlan.the_limit_profile_refuses_the_quota_kind
#print axioms QCore.ProcessQuotaPlan.the_confining_profile_extends_the_table_by_exactly_the_quota_kind
#print axioms QCore.ProcessQuotaPlan.a_foreign_kind_is_never_placed
#print axioms QCore.ProcessQuotaPlan.profile_names_are_pairwise_distinct

-- Refusal completeness: a refusal names a kind the manifest declared, an
-- accepted plan covers every declared kind, each kind sits in its own
-- mechanism's list, and no accepted plan carries a zero quota.
#print axioms QCore.ProcessQuotaPlan.an_unplaceable_kind_is_declared_and_has_no_role
#print axioms QCore.ProcessQuotaPlan.a_zero_quota_kind_is_declared_and_is_a_quota
#print axioms QCore.ProcessQuotaPlan.no_unplaceable_kind_means_every_declared_kind_is_placed
#print axioms QCore.ProcessQuotaPlan.the_legacy_profile_names_every_declared_kind_unenforced
#print axioms QCore.ProcessQuotaPlan.an_enforced_plan_left_no_unplaceable_kind
#print axioms QCore.ProcessQuotaPlan.an_enforced_plan_left_no_zero_quota
#print axioms QCore.ProcessQuotaPlan.an_enforced_plan_is_the_declaration_split_by_role
#print axioms QCore.ProcessQuotaPlan.an_enforcing_profile_places_or_refuses_by_name
#print axioms QCore.ProcessQuotaPlan.an_enforced_plan_covers_every_declared_kind
#print axioms QCore.ProcessQuotaPlan.an_enforced_plan_places_each_kind_in_its_own_mechanism
#print axioms QCore.ProcessQuotaPlan.an_enforced_plan_has_no_zero_quota

-- The availability override is one-directional: it can only widen refusal.
#print axioms QCore.ProcessQuotaPlan.the_probe_can_only_force_a_mechanism_absent
#print axioms QCore.ProcessQuotaPlan.the_default_probe_leaves_the_decision_unchanged
#print axioms QCore.ProcessQuotaPlan.a_forced_probe_only_refuses
#print axioms QCore.ProcessQuotaPlan.a_forced_probe_refuses_every_confined_execution
#print axioms QCore.ProcessQuotaPlan.a_forced_probe_grants_no_profile_a_mechanism
