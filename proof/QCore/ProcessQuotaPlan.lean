-- SPDX-License-Identifier: Apache-2.0
--
-- Licensed under the Apache License, Version 2.0 (the "License");
-- you may not use this file except in compliance with the License.
-- You may obtain a copy of the License at
--
--     http://www.apache.org/licenses/LICENSE-2.0
--
-- Unless required by applicable law or agreed to in writing, software
-- distributed under the License is distributed on an "AS IS" BASIS,
-- WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
-- See the License for the specific language governing permissions and
-- limitations under the License.

/-! Card `P03-F`: the declared-limit-kind plan of the verifier host's process
execution profiles.

This models the *decision* made by `enforceable_limit_kinds`,
`process_quota::declared_kind_role` and `ProcessProvider::enforced_limits` in
`crates/qcore-verifier-host`: given a target, an execution profile and the set
of limit kinds a manifest declares, which mechanism installs each kind, and
what happens to a kind no mechanism installs.

The property worth proving here is *refusal completeness*: no declared kind is
ever silently dropped. Under an enforcing profile every declared kind either
lands in a mechanism's plan or refuses the whole execution by name; under the
historical unenforcing profile every declared kind is named in the
`unenforced_kinds` list instead, which is that profile's own honest answer and
not a third, silent behaviour.

The second property is the *direction* of the mechanism-availability override
(`MechanismProbe`): it can turn an enforcing outcome into a refusal and never
the reverse, so no use of it can produce an execution that ran with weaker
enforcement than the kernel offered.

The third, in the last section, is that the per-execution cgroups of one host
process are named apart for every clock reading (card
`EXECUTION-QUOTA-LOAD-INDEPENDENCE`).

What is **not** modelled here, and stays a Rust and OS boundary: that the
cgroup v2 `pids` controller actually bounds a process tree (measured in
`docs/reports/remaining-p03-f/README.md`), that Landlock and the namespace
unshare do what they claim, and that the Rust tables below are the tables the
Rust code really uses — the enumeration is a transcription, pinned on the Rust
side by `crates/qcore-verifier-host/tests/execution_quota.rs`. -/
namespace QCore.ProcessQuotaPlan

/-- The three execution profiles the verifier host offers. -/
inductive Profile where
  /-- The historical path: a wall-clock deadline, output caps and a
  process-group kill, and no `setrlimit` at all. -/
  | unenforcedLegacy
  /-- Linux only: every declared kind installed with `setrlimit`. -/
  | linuxRlimit
  /-- Linux only: everything the limit profile installs, plus filesystem and
  network confinement and a per-execution process quota. -/
  | linuxConfined
  deriving DecidableEq, Repr

namespace Profile

def all : List Profile := [.unenforcedLegacy, .linuxRlimit, .linuxConfined]

/-- Mirrors `ProcessExecutionProfile::name`. -/
def name : Profile → String
  | .unenforcedLegacy => "unenforced-legacy"
  | .linuxRlimit => "linux-rlimit-v1"
  | .linuxConfined => "linux-confined-v1"

/-- Mirrors `ProcessExecutionProfile::installs_rlimits`. -/
def installsRlimits : Profile → Bool
  | .unenforcedLegacy => false
  | .linuxRlimit => true
  | .linuxConfined => true

/-- Mirrors `process_quota::profile_installs_quota`. -/
def installsQuota : Profile → Bool
  | .linuxConfined => true
  | _ => false

/-- The recorded default. Deliberately a definition, so a change to it is a
change to a proved statement. -/
def default : Profile := .unenforcedLegacy

end Profile

/-- Whether the host runs on the one target these profiles are qualified for. -/
inductive Target where
  | linux
  | other
  deriving DecidableEq, Repr

/-- The declarable limit kinds. `foreign` stands for every kind name outside
the closed table — `network_bandwidth_bps` in the Rust tests — which no
profile can enforce. -/
inductive Kind where
  | memoryBytes
  | cpuSeconds
  | coreBytes
  | maxFileBytes
  | maxOpenFiles
  /-- `max_processes`: `RLIMIT_NPROC`, counted **per real UID**. -/
  | maxProcesses
  /-- `max_execution_processes`: a cgroup quota counted **per process tree**. -/
  | maxExecutionProcesses
  | foreign
  deriving DecidableEq, Repr

namespace Kind

def all : List Kind :=
  [ .memoryBytes, .cpuSeconds, .coreBytes, .maxFileBytes, .maxOpenFiles,
    .maxProcesses, .maxExecutionProcesses, .foreign ]

/-- The manifest spelling, identical to the Rust kind names. -/
def spelling : Kind → String
  | .memoryBytes => "memory_bytes"
  | .cpuSeconds => "cpu_seconds"
  | .coreBytes => "core_bytes"
  | .maxFileBytes => "max_file_bytes"
  | .maxOpenFiles => "max_open_files"
  | .maxProcesses => "max_processes"
  | .maxExecutionProcesses => "max_execution_processes"
  | .foreign => "<outside the closed table>"

/-- Mirrors `LINUX_RLIMIT_V1_KINDS`. -/
def isRlimitKind : Kind → Bool
  | .foreign => false
  | .maxExecutionProcesses => false
  | _ => true

/-- Mirrors `PROCESS_QUOTA_KINDS`. -/
def isQuotaKind : Kind → Bool
  | .maxExecutionProcesses => true
  | _ => false

end Kind

/-- What installs one declared kind. Mirrors `DeclaredKindRole`. -/
inductive Role where
  | rlimit
  | processQuota
  deriving DecidableEq, Repr

/-- Mirrors `process_quota::declared_kind_role`. `none` is a refusal at every
call site, never a skipped limit. -/
def role (target : Target) (profile : Profile) (kind : Kind) : Option Role :=
  match target with
  | .other => none
  | .linux =>
    if profile.installsRlimits && kind.isRlimitKind then some .rlimit
    else if profile.installsQuota && kind.isQuotaKind then some .processQuota
    else none

/-- Mirrors `enforceable_rlimit_kinds`. -/
def enforceableRlimitKinds (target : Target) (profile : Profile) : List Kind :=
  Kind.all.filter (fun k => role target profile k == some .rlimit)

/-- Mirrors `enforceable_limit_kinds`. -/
def enforceableKinds (target : Target) (profile : Profile) : List Kind :=
  Kind.all.filter (fun k => (role target profile k).isSome)

/-- One manifest's `limits.additional`: kinds with declared values. -/
abbrev Declared := List (Kind × Nat)

/-- Why a declaration refused. Each is a pre-spawn refusal with no weaker
fallback. -/
inductive Reason where
  /-- No mechanism of this profile installs this kind. -/
  | noMechanism
  /-- A quota of zero admits no process at all, not even the entrypoint. -/
  | zeroQuota
  /-- A mechanism the profile needs is unavailable on this host. -/
  | mechanismUnavailable
  deriving DecidableEq, Repr

/-- What one profile decides for one manifest. There is no fourth constructor:
a declared kind is enforced, named unenforced, or refuses the execution. -/
inductive Outcome where
  /-- Every declared kind has a mechanism. The two lists partition the
  declaration. -/
  | enforced (rlimits : List Kind) (quota : List Kind)
  /-- The historical profile installs nothing and names every declared kind it
  did not enforce. -/
  | unenforced (kinds : List Kind)
  /-- Refused before spawning, naming the offending kind. -/
  | refused (kind : Kind) (reason : Reason)
  deriving DecidableEq, Repr

/-- The first declared kind this profile cannot place, if any. -/
def firstUnplaceable (target : Target) (profile : Profile) : Declared → Option Kind
  | [] => none
  | (kind, _) :: rest =>
    if (role target profile kind).isSome then firstUnplaceable target profile rest
    else some kind

/-- The first declared quota whose value is zero, if any. -/
def firstZeroQuota (target : Target) (profile : Profile) : Declared → Option Kind
  | [] => none
  | (kind, value) :: rest =>
    if role target profile kind == some .processQuota && value == 0 then some kind
    else firstZeroQuota target profile rest

/-- Mirrors `ProcessProvider::enforced_limits`, over declared kinds only.

The historical profile is checked first, because it does not refuse: it enforces
nothing and says so. Every other profile places every declared kind or refuses
by name. -/
def decide (target : Target) (profile : Profile) (declared : Declared) : Outcome :=
  if profile == .unenforcedLegacy then
    .unenforced (declared.map Prod.fst)
  else match firstUnplaceable target profile declared with
    | some kind => .refused kind .noMechanism
    | none =>
      match firstZeroQuota target profile declared with
      | some kind => .refused kind .zeroQuota
      | none =>
        .enforced
          (declared.map Prod.fst |>.filter (fun k => role target profile k == some .rlimit))
          (declared.map Prod.fst |>.filter (fun k => role target profile k == some .processQuota))

/-- Whether a mechanism is taken from the kernel or forced absent. Mirrors
`MechanismProbe`; there is deliberately no variant forcing availability. -/
inductive Probe where
  | asProbed
  | forcedUnavailable
  deriving DecidableEq, Repr

namespace Probe

def all : List Probe := [.asProbed, .forcedUnavailable]

/-- Mirrors `MechanismProbe::forced_refusal`: `some` exactly for the forced
variant. -/
def refuses : Probe → Bool
  | .asProbed => false
  | .forcedUnavailable => true

end Probe

/-- The decision with a mechanism probe applied. A forced-unavailable mechanism
refuses before the kernel is asked; otherwise the decision is unchanged. -/
def decideWithProbe (target : Target) (profile : Profile) (probe : Probe)
    (declared : Declared) : Outcome :=
  if profile.installsQuota && probe.refuses then
    .refused .maxExecutionProcesses .mechanismUnavailable
  else decide target profile declared

/-- The two kinds whose confusion this card exists to prevent. -/
def perUidKind : Kind := .maxProcesses
def perTreeKind : Kind := .maxExecutionProcesses


/-! ## The proved relation

The decidable half is decided; the list-shaped half is proved by induction. -/

/-- The default profile is the historical, unenforcing one. A change to the
recorded default is a change to this statement. -/
theorem the_recorded_default_is_the_unenforcing_profile :
    Profile.default = Profile.unenforcedLegacy := rfl

/-- The default profile installs neither mechanism. -/
theorem the_default_profile_installs_nothing :
    Profile.default.installsRlimits = false ∧ Profile.default.installsQuota = false := by
  decide

/-- Exactly one profile installs a per-execution quota. -/
theorem exactly_one_profile_installs_a_quota :
    Profile.all.filter Profile.installsQuota = [Profile.linuxConfined] := by decide

/-- The per-UID kind and the per-tree kind are distinct, and neither mechanism
table claims the other's kind. This is the confusion the card exists to
prevent: a manifest can never obtain a per-tree bound while declaring the
per-UID one, or the reverse. -/
theorem the_per_uid_and_per_tree_kinds_are_never_confused :
    perUidKind ≠ perTreeKind
      ∧ perUidKind.isRlimitKind = true ∧ perUidKind.isQuotaKind = false
      ∧ perTreeKind.isRlimitKind = false ∧ perTreeKind.isQuotaKind = true := by
  decide

/-- No kind is both an rlimit kind and a quota kind, so no kind takes two
roles. -/
theorem the_two_mechanism_tables_are_disjoint :
    ∀ k ∈ Kind.all, ¬ (k.isRlimitKind = true ∧ k.isQuotaKind = true) := by decide

/-- Off Linux no profile places any kind. -/
theorem off_linux_no_profile_places_any_kind :
    ∀ p ∈ Profile.all, ∀ k ∈ Kind.all, role .other p k = none := by decide

/-- Off Linux the enforceable set is empty for every profile. -/
theorem off_linux_nothing_is_enforceable :
    ∀ p ∈ Profile.all, enforceableKinds .other p = [] := by decide

/-- The historical profile places nothing even on Linux. -/
theorem the_legacy_profile_places_nothing :
    ∀ k ∈ Kind.all, role .linux .unenforcedLegacy k = none := by decide

/-- The limit profile places exactly the six `setrlimit` kinds. -/
theorem the_limit_profile_places_exactly_the_rlimit_table :
    enforceableKinds .linux .linuxRlimit
      = [.memoryBytes, .cpuSeconds, .coreBytes, .maxFileBytes, .maxOpenFiles,
         .maxProcesses] := by decide

/-- The limit profile has no role at all for the quota kind, which is why a
manifest declaring it under that profile refuses. -/
theorem the_limit_profile_refuses_the_quota_kind :
    role .linux .linuxRlimit perTreeKind = none := by decide

/-- The confining profile places the same six `setrlimit` kinds, plus the quota
kind and nothing else. This is the deliberate pin update recorded in
`confined_process_execution.rs`: the `setrlimit` tables stay equal, and the
total enforceable sets differ by exactly one kind. -/
theorem the_confining_profile_extends_the_table_by_exactly_the_quota_kind :
    enforceableRlimitKinds .linux .linuxConfined
        = enforceableRlimitKinds .linux .linuxRlimit
      ∧ enforceableKinds .linux .linuxConfined
        = enforceableKinds .linux .linuxRlimit ++ [perTreeKind] := by
  decide

/-- No kind outside the closed table is placed by any profile on any target. -/
theorem a_foreign_kind_is_never_placed :
    ∀ t ∈ [Target.linux, Target.other], ∀ p ∈ Profile.all,
      role t p .foreign = none := by decide

/-- Every profile name is distinct, so a record naming a profile identifies
it. -/
theorem profile_names_are_pairwise_distinct :
    (Profile.all.map Profile.name).eraseDups = Profile.all.map Profile.name := by
  decide

/-- Every kind spelling is distinct, so a refusal naming a kind identifies it.
In particular the per-UID and per-tree spellings differ. -/
theorem kind_spellings_are_pairwise_distinct :
    (Kind.all.map Kind.spelling).eraseDups = Kind.all.map Kind.spelling
      ∧ perUidKind.spelling ≠ perTreeKind.spelling := by
  decide

/-! ### Refusal completeness -/

/-- A named unplaceable kind is one the manifest actually declared, and it
really has no role. A refusal names a declared kind, never an invented one. -/
theorem an_unplaceable_kind_is_declared_and_has_no_role
    (t : Target) (p : Profile) :
    ∀ d : Declared, ∀ k, firstUnplaceable t p d = some k →
      (∃ v, (k, v) ∈ d) ∧ role t p k = none := by
  intro d
  induction d with
  | nil => intro k h; simp [firstUnplaceable] at h
  | cons e rest ih =>
    intro k h
    by_cases he : (role t p e.fst).isSome = true
    · rw [firstUnplaceable, ite_eq_left he] at h
      obtain ⟨⟨v, hv⟩, hrole⟩ := ih k h
      exact ⟨⟨v, List.mem_cons_of_mem _ hv⟩, hrole⟩
    · rw [firstUnplaceable, ite_eq_right he] at h
      have hk : e.fst = k := Option.some.inj h
      refine ⟨⟨e.snd, ?_⟩, ?_⟩
      · have : (k, e.snd) = e := by rw [← hk]
        exact this ▸ List.mem_cons_self ..
      · rw [← hk]; exact Option.not_isSome_iff_eq_none.mp he

/-- A refused zero quota also names a declared kind, and that kind really is a
quota the profile installs. -/
theorem a_zero_quota_kind_is_declared_and_is_a_quota
    (t : Target) (p : Profile) :
    ∀ d : Declared, ∀ k, firstZeroQuota t p d = some k →
      (k, 0) ∈ d ∧ role t p k = some .processQuota := by
  intro d
  induction d with
  | nil => intro k h; simp [firstZeroQuota] at h
  | cons e rest ih =>
    intro k h
    by_cases he : (role t p e.fst == some Role.processQuota && e.snd == 0) = true
    · rw [firstZeroQuota, ite_eq_left he] at h
      have hk : e.fst = k := Option.some.inj h
      have hq : role t p e.fst = some .processQuota := by
        have := (Bool.and_eq_true _ _).mp he
        simpa using this.1
      have hz : e.snd = 0 := by
        have := (Bool.and_eq_true _ _).mp he
        simpa using this.2
      refine ⟨?_, hk ▸ hq⟩
      have : e = (k, 0) := by
        rw [← hk, ← hz]
      exact this ▸ List.mem_cons_self ..
    · rw [firstZeroQuota, ite_eq_right he] at h
      obtain ⟨hv, hrole⟩ := ih k h
      exact ⟨List.mem_cons_of_mem _ hv, hrole⟩

/-- A declaration with no unplaceable kind places every kind it declares. -/
theorem no_unplaceable_kind_means_every_declared_kind_is_placed
    (t : Target) (p : Profile) :
    ∀ d : Declared, firstUnplaceable t p d = none →
      ∀ k v, (k, v) ∈ d → (role t p k).isSome = true := by
  intro d
  induction d with
  | nil => intro _ k v hv; simp at hv
  | cons e rest ih =>
    intro h k v hv
    by_cases he : (role t p e.fst).isSome = true
    · rw [firstUnplaceable, ite_eq_left he] at h
      rcases List.mem_cons.mp hv with h1 | h2
      · subst h1; simpa using he
      · exact ih h k v h2
    · rw [firstUnplaceable, ite_eq_right he] at h
      simp at h

/-- The historical profile never refuses and never enforces: it names every
declared kind as unenforced. That is its own honest answer, not a silent third
behaviour. -/
theorem the_legacy_profile_names_every_declared_kind_unenforced
    (t : Target) (d : Declared) :
    decide t .unenforcedLegacy d = .unenforced (d.map Prod.fst) := by
  simp [decide]

/-- An enforcing profile that produced a plan found no unplaceable kind. -/
theorem an_enforced_plan_left_no_unplaceable_kind
    (t : Target) (p : Profile) (d : Declared) (r q : List Kind)
    (hp : p ≠ .unenforcedLegacy) (h : decide t p d = .enforced r q) :
    firstUnplaceable t p d = none := by
  have hpb : (p == Profile.unenforcedLegacy) = false := by
    simpa using beq_eq_false_iff_ne.mpr hp
  cases hf : firstUnplaceable t p d with
  | none => rfl
  | some k => rw [decide, ite_eq_right (by simp [hpb]), hf] at h; exact absurd h (by simp)

/-- An enforcing profile that produced a plan found no zero quota. -/
theorem an_enforced_plan_left_no_zero_quota
    (t : Target) (p : Profile) (d : Declared) (r q : List Kind)
    (hp : p ≠ .unenforcedLegacy) (h : decide t p d = .enforced r q) :
    firstZeroQuota t p d = none := by
  have hpb : (p == Profile.unenforcedLegacy) = false := by
    simpa using beq_eq_false_iff_ne.mpr hp
  have hnone := an_enforced_plan_left_no_unplaceable_kind t p d r q hp h
  cases hz : firstZeroQuota t p d with
  | none => rfl
  | some k =>
    rw [decide, ite_eq_right (by simp [hpb]), hnone, hz] at h
    exact absurd h (by simp)

/-- The two lists an enforced plan carries are exactly the declared kinds
filtered by role. -/
theorem an_enforced_plan_is_the_declaration_split_by_role
    (t : Target) (p : Profile) (d : Declared) (r q : List Kind)
    (hp : p ≠ .unenforcedLegacy) (h : decide t p d = .enforced r q) :
    r = (d.map Prod.fst).filter (fun k => role t p k == some .rlimit)
      ∧ q = (d.map Prod.fst).filter (fun k => role t p k == some .processQuota) := by
  have hpb : (p == Profile.unenforcedLegacy) = false := by
    simpa using beq_eq_false_iff_ne.mpr hp
  have hnone := an_enforced_plan_left_no_unplaceable_kind t p d r q hp h
  have hzero := an_enforced_plan_left_no_zero_quota t p d r q hp h
  rw [decide, ite_eq_right (by simp [hpb]), hnone, hzero] at h
  exact ⟨(Outcome.enforced.inj h).1.symm, (Outcome.enforced.inj h).2.symm⟩

/-- **Refusal completeness for the enforcing profiles.** Under a profile that is
not the historical one, a declaration is either fully placed or refused by the
name of a kind it actually declared. There is no outcome in which a declared
kind is silently dropped. -/
theorem an_enforcing_profile_places_or_refuses_by_name
    (t : Target) (p : Profile) (d : Declared) (hp : p ≠ .unenforcedLegacy) :
    (∃ r q, decide t p d = .enforced r q)
      ∨ (∃ k reason, decide t p d = .refused k reason ∧ ∃ v, (k, v) ∈ d) := by
  have hpb : (p == Profile.unenforcedLegacy) = false := by
    simpa using beq_eq_false_iff_ne.mpr hp
  cases hf : firstUnplaceable t p d with
  | some k =>
    refine Or.inr ⟨k, .noMechanism, ?_, ?_⟩
    · rw [decide, ite_eq_right (by simp [hpb]), hf]
    · exact (an_unplaceable_kind_is_declared_and_has_no_role t p d k hf).1
  | none =>
    cases hz : firstZeroQuota t p d with
    | some k =>
      refine Or.inr ⟨k, .zeroQuota, ?_, ?_⟩
      · rw [decide, ite_eq_right (by simp [hpb]), hf, hz]
      · exact ⟨0, (a_zero_quota_kind_is_declared_and_is_a_quota t p d k hz).1⟩
    | none =>
      exact Or.inl ⟨_, _, by rw [decide, ite_eq_right (by simp [hpb]), hf, hz]⟩

/-- **An enforced plan covers every declared kind.** Every kind the manifest
declared lands in one of the two mechanism lists, so an accepted plan drops
nothing. -/
theorem an_enforced_plan_covers_every_declared_kind
    (t : Target) (p : Profile) (d : Declared) (r q : List Kind)
    (hp : p ≠ .unenforcedLegacy) (h : decide t p d = .enforced r q) :
    ∀ k v, (k, v) ∈ d → k ∈ r ∨ k ∈ q := by
  intro k v hv
  have hnone := an_enforced_plan_left_no_unplaceable_kind t p d r q hp h
  have hplaced :=
    no_unplaceable_kind_means_every_declared_kind_is_placed t p d hnone k v hv
  obtain ⟨hr, hq⟩ := an_enforced_plan_is_the_declaration_split_by_role t p d r q hp h
  have hmem : k ∈ d.map Prod.fst := List.mem_map.mpr ⟨(k, v), hv, rfl⟩
  rcases Option.isSome_iff_exists.mp hplaced with ⟨rol, hrol⟩
  cases rol with
  | rlimit => exact Or.inl (hr ▸ List.mem_filter.mpr ⟨hmem, by simp [hrol]⟩)
  | processQuota => exact Or.inr (hq ▸ List.mem_filter.mpr ⟨hmem, by simp [hrol]⟩)

/-- Each mechanism list holds only the kinds that mechanism installs, so the
plan is a partition rather than merely a pair of lists. -/
theorem an_enforced_plan_places_each_kind_in_its_own_mechanism
    (t : Target) (p : Profile) (d : Declared) (r q : List Kind)
    (hp : p ≠ .unenforcedLegacy) (h : decide t p d = .enforced r q) :
    (∀ k ∈ r, role t p k = some .rlimit)
      ∧ (∀ k ∈ q, role t p k = some .processQuota) := by
  obtain ⟨hr, hq⟩ := an_enforced_plan_is_the_declaration_split_by_role t p d r q hp h
  refine ⟨fun k hk => ?_, fun k hk => ?_⟩
  · have := List.mem_filter.mp (hr ▸ hk); simpa using this.2
  · have := List.mem_filter.mp (hq ▸ hk); simpa using this.2

/-- An enforced plan carries no zero quota: a bound admitting no process at all
never reaches a spawn. -/
theorem an_enforced_plan_has_no_zero_quota
    (t : Target) (p : Profile) (d : Declared) (r q : List Kind)
    (hp : p ≠ .unenforcedLegacy) (h : decide t p d = .enforced r q) :
    ∀ k, role t p k = some .processQuota → (k, 0) ∉ d := by
  have hzero := an_enforced_plan_left_no_zero_quota t p d r q hp h
  intro k _ hmem
  have : firstZeroQuota t p d ≠ none := by
    clear hzero h
    induction d with
    | nil => simp at hmem
    | cons e rest ih =>
      by_cases he : (role t p e.fst == some Role.processQuota && e.snd == 0) = true
      · rw [firstZeroQuota, ite_eq_left he]; simp
      · rw [firstZeroQuota, ite_eq_right he]
        rcases List.mem_cons.mp hmem with h1 | h2
        · exact absurd (by rw [← h1] at *; simp_all) he
        · exact ih h2
  exact this hzero

/-- Off Linux, any nonempty declaration refuses under an enforcing profile —
never an enforced plan. -/
theorem off_linux_an_enforcing_profile_refuses_any_declaration
    (p : Profile) (d : Declared) (hp : p ≠ .unenforcedLegacy) (hd : d ≠ []) :
    ∃ k, decide .other p d = .refused k .noMechanism := by
  have hpb : (p == Profile.unenforcedLegacy) = false := by
    simpa using beq_eq_false_iff_ne.mpr hp
  cases d with
  | nil => exact absurd rfl hd
  | cons e rest =>
    exact ⟨e.fst, by rw [decide, ite_eq_right (by simp [hpb])]; simp [firstUnplaceable, role]⟩

/-! ### The override's direction -/

/-- The probe has exactly two inhabitants and only one of them refuses: there
is no variant that forces a mechanism *available*. -/
theorem the_probe_can_only_force_a_mechanism_absent :
    Probe.all = [Probe.asProbed, Probe.forcedUnavailable]
      ∧ Probe.all.filter Probe.refuses = [Probe.forcedUnavailable]
      ∧ Probe.asProbed.refuses = false := by
  decide

/-- The default probe changes nothing: with no override, the decision is the
kernel's. -/
theorem the_default_probe_leaves_the_decision_unchanged
    (t : Target) (p : Profile) (d : Declared) :
    decideWithProbe t p .asProbed d = decide t p d := by
  simp [decideWithProbe, Probe.refuses]

/-- **The override is one-directional.** A forced-unavailable mechanism can only
produce a refusal: whenever a forced decision is an enforced plan, the unforced
decision is that same plan. The override can never turn a refusal into an
enforced plan, and can never enlarge what an enforced plan installs. -/
theorem a_forced_probe_only_refuses
    (t : Target) (p : Profile) (d : Declared) (r q : List Kind)
    (h : decideWithProbe t p .forcedUnavailable d = .enforced r q) :
    decide t p d = .enforced r q := by
  by_cases hq : p.installsQuota = true
  · rw [decideWithProbe, ite_eq_left (by simp [hq, Probe.refuses])] at h
    exact absurd h (by simp)
  · rw [decideWithProbe, ite_eq_right (by simp [Bool.eq_false_iff.mp (by simpa using hq)])] at h
    exact h

/-- Stated the other way: under a confining profile a forced probe always
refuses, whatever the declaration was. -/
theorem a_forced_probe_refuses_every_confined_execution
    (t : Target) (d : Declared) :
    decideWithProbe t .linuxConfined .forcedUnavailable d
      = .refused perTreeKind .mechanismUnavailable := by
  simp [decideWithProbe, Probe.refuses, Profile.installsQuota, perTreeKind]

/-- The override cannot grant a profile a mechanism it does not install: it
leaves that profile's decision alone. -/
theorem a_forced_probe_grants_no_profile_a_mechanism
    (t : Target) (p : Profile) (d : Declared) (hp : p.installsQuota = false) :
    decideWithProbe t p .forcedUnavailable d = decide t p d := by
  simp [decideWithProbe, hp]

/-! ### Per-execution cgroup names

Card `EXECUTION-QUOTA-LOAD-INDEPENDENCE`. `process_quota::linux::session_name`
names one execution's cgroup by the host process id, a sequence number drawn
from a per-process counter, and the wall clock. Before the card the name was
the process id and the clock only: concurrent executions of one host process
that read the same nanosecond got the same name, and the second `mkdir` was
refused with `EEXIST` before spawning, which a loaded test gate measured.

The name is modelled as the triple it formats. That the Rust spelling
(`qcore-exec-{pid}-{sequence}-{nanos}`, decimal fields between hyphens) decodes
back to that triple is pinned on the Rust side by
`per_execution_cgroup_names_never_collide_within_one_process`. The clock is an
arbitrary function of the draw: nothing below assumes it is monotone, or that
two readings differ. -/

/-- The fields one per-execution cgroup name is built from. -/
structure SessionKey where
  pid : Nat
  sequence : Nat
  nanos : Nat
  deriving DecidableEq, Repr

/-- The key of the session that drew sequence number `n` in process `pid`,
with `clock n` the wall-clock reading it took. -/
def sessionKey (pid : Nat) (clock : Nat → Nat) (n : Nat) : SessionKey :=
  ⟨pid, n, clock n⟩

/-- The key before the card: the process id and the clock reading only. -/
def legacySessionKey (pid : Nat) (clock : Nat → Nat) (n : Nat) : Nat × Nat :=
  (pid, clock n)

/-- Sessions of one process are keyed apart by their sequence numbers alone,
whatever the clock read. -/
theorem session_keys_are_injective_in_the_sequence
    (pid : Nat) (clock : Nat → Nat) :
    Function.Injective (sessionKey pid clock) := by
  intro m n h
  exact congrArg SessionKey.sequence h

/-- The first `count` sessions a process draws have pairwise distinct keys, for
every clock. -/
theorem the_sessions_of_one_process_have_pairwise_distinct_keys
    (pid : Nat) (clock : Nat → Nat) (count : Nat) :
    ((List.range count).map (sessionKey pid clock)).Nodup :=
  List.Pairwise.map (sessionKey pid clock)
    (fun _ _ hne heq => hne (session_keys_are_injective_in_the_sequence pid clock heq))
    List.nodup_range

/-- **What the card repaired.** Two sessions of one process whose clock
readings coincide had the same legacy key, and have distinct keys now. -/
theorem a_repeated_clock_reading_collides_only_the_legacy_key
    (pid : Nat) (clock : Nat → Nat) {m n : Nat}
    (hmn : m ≠ n) (hclock : clock m = clock n) :
    legacySessionKey pid clock m = legacySessionKey pid clock n
      ∧ sessionKey pid clock m ≠ sessionKey pid clock n := by
  refine ⟨by simp [legacySessionKey, hclock], ?_⟩
  intro h
  exact hmn (session_keys_are_injective_in_the_sequence pid clock h)

/-! ### The quota join and the record that names it

Card `QUOTA-PROCS-DESCRIPTOR-DUP-FAIL-OPEN`. One execution that declares a
quota creates its cgroup, then the parent duplicates the cgroup's
`cgroup.procs` write descriptor for the child, and the child's first
`pre_exec` step writes itself into the cgroup with it. Before the card a failed
duplicate was mapped to "no join": the child was spawned without the join, ran
outside its cgroup, and the record still carried an observation of the empty
cgroup under limits naming the quota. `verify_process` also observed the
cgroup of every execution that had created one, spawned or not.

The model is the step function of `ProcessProvider::verify_process` and
`run_process`: which steps succeeded, and what the record then says. A failed
`pre_exec` step aborts the spawn, which is `std::process::Command`'s contract;
the join is the first step, so a spawn that returned a child had joined. That
the Rust follows this step order is pinned on the Rust side by
`crates/qcore-verifier-host/tests/execution_quota.rs`
(`a_failed_join_descriptor_refuses_before_spawning`,
`an_identity_refusal_after_the_cgroup_exists_records_no_observation`,
`a_failed_spawn_records_no_observation`). -/

/-- What succeeded in one execution the host could spawn. A field is read only
where the steps before it ran. -/
structure ExecutionSteps where
  /-- The execution declares a quota, so its cgroup was created. -/
  quotaDeclared : Bool
  /-- The parent's duplicate of the cgroup's `cgroup.procs` descriptor. -/
  descriptorObtained : Bool
  /-- The child's join write, when the join is installed. -/
  joinWritten : Bool
  /-- Every other step up to and including `execve` (the identity re-check,
  the confinement, the `setrlimit` plan, the exec). -/
  restSucceeded : Bool
  deriving DecidableEq, Repr

/-- Where an execution ended. -/
inductive SpawnOutcome where
  /-- Refused before anything was forked (`RefusedByProfile`). -/
  | refusedBeforeSpawn
  /-- No child was returned: a `pre_exec` step, the exec or a pre-spawn check
  failed (`SpawnFailed`, `RefusedByIdentity`, a pre-spawn `Protocol`). -/
  | notSpawned
  /-- `spawn` returned a child that is running the package. -/
  | spawned
  deriving DecidableEq, Repr

/-- The parts of `ProcessExecutionRecord` this section is about. -/
structure QuotaRecord where
  outcome : SpawnOutcome
  /-- `enforced` is present and its `process_quota` names the quota. -/
  namesQuota : Bool
  /-- `quota_observed` is present. -/
  observed : Bool
  deriving DecidableEq, Repr

/-- The child became a member of its cgroup: the join was installed and its
write succeeded. -/
def ExecutionSteps.joined (s : ExecutionSteps) : Bool :=
  s.quotaDeclared && s.descriptorObtained && s.joinWritten

/-- `spawn` returns a child when every installed `pre_exec` step succeeds,
the join first, and the exec succeeds. -/
def ExecutionSteps.spawnsWith (s : ExecutionSteps) (joinInstalled : Bool) : Bool :=
  (!joinInstalled || s.joinWritten) && s.restSucceeded

/-- The host after the card. A quota whose descriptor could not be duplicated
refuses before spawning; otherwise the join is installed exactly when a quota
was declared, and the cgroup is observed only when `spawn` returned a child. -/
def recordOf (s : ExecutionSteps) : QuotaRecord :=
  if s.quotaDeclared && !s.descriptorObtained then
    ⟨.refusedBeforeSpawn, false, false⟩
  else if s.spawnsWith s.quotaDeclared then
    ⟨.spawned, s.quotaDeclared, s.quotaDeclared⟩
  else
    ⟨.notSpawned, s.quotaDeclared, false⟩

/-- The host before the card: a failed duplicate only dropped the join, and
every created cgroup was observed. -/
def legacyRecordOf (s : ExecutionSteps) : QuotaRecord :=
  if s.spawnsWith (s.quotaDeclared && s.descriptorObtained) then
    ⟨.spawned, s.quotaDeclared, s.quotaDeclared⟩
  else
    ⟨.notSpawned, s.quotaDeclared, s.quotaDeclared⟩

/-- **A failed duplicate refuses before spawning.** The record is the
profile refusal: no limits named, no observation. -/
theorem a_failed_join_descriptor_refuses_before_spawning
    (s : ExecutionSteps) (hq : s.quotaDeclared = true)
    (hd : s.descriptorObtained = false) :
    recordOf s = ⟨.refusedBeforeSpawn, false, false⟩ := by
  simp [recordOf, hq, hd]

/-- **An observation implies the join.** No record carries an observation of a
cgroup its child did not join. -/
theorem an_observation_implies_the_join
    (s : ExecutionSteps) (h : (recordOf s).observed = true) :
    s.joined = true := by
  obtain ⟨q, d, j, r⟩ := s
  cases q <;> cases d <;> cases j <;> cases r <;>
    simp_all [recordOf, ExecutionSteps.spawnsWith, ExecutionSteps.joined]

/-- **A running package under a named quota joined it.** A record of a spawned
child whose limits name the quota is a record of a child inside its cgroup. -/
theorem a_spawned_record_naming_a_quota_implies_the_join
    (s : ExecutionSteps) (hs : (recordOf s).outcome = .spawned)
    (hn : (recordOf s).namesQuota = true) :
    s.joined = true := by
  obtain ⟨q, d, j, r⟩ := s
  cases q <;> cases d <;> cases j <;> cases r <;>
    simp_all [recordOf, ExecutionSteps.spawnsWith, ExecutionSteps.joined]

/-- **A failed join never runs the package**, and leaves no observation. -/
theorem a_failed_join_never_spawns
    (s : ExecutionSteps) (hq : s.quotaDeclared = true) (hj : s.joinWritten = false) :
    (recordOf s).outcome ≠ .spawned ∧ (recordOf s).observed = false := by
  obtain ⟨q, d, j, r⟩ := s
  cases q <;> cases d <;> cases j <;> cases r <;>
    simp_all [recordOf, ExecutionSteps.spawnsWith]

/-- **Nothing else moved.** An execution that declares no quota, or whose
duplicate succeeded and whose child was spawned, has the record it had before
the card. -/
theorem the_repair_changes_only_failed_quota_executions
    (s : ExecutionSteps)
    (h : s.quotaDeclared = false ∨
      (s.descriptorObtained = true ∧ (recordOf s).outcome = .spawned)) :
    recordOf s = legacyRecordOf s := by
  obtain ⟨q, d, j, r⟩ := s
  cases q <;> cases d <;> cases j <;> cases r <;>
    simp_all [recordOf, legacyRecordOf, ExecutionSteps.spawnsWith]

/-- **What the card repaired.** Before it, a failed duplicate spawned a child
outside its cgroup and recorded an observation under limits naming the quota. -/
theorem the_legacy_host_observed_a_quota_its_child_never_joined :
    ∃ s : ExecutionSteps,
      (legacyRecordOf s).outcome = .spawned ∧ (legacyRecordOf s).namesQuota = true ∧
        (legacyRecordOf s).observed = true ∧ s.joined = false :=
  ⟨⟨true, false, true, true⟩, by decide⟩

end QCore.ProcessQuotaPlan
