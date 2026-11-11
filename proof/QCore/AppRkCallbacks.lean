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

import QCore.OpCallbacks

/-!
# Application rk_conf callbacks (card OP-APP-RK-CALLBACKS-VIA-POLL-CB)

The model of the user's decision of 2026-10-14 ("Build the agent's
recommended set") on gateway rule 12n and contract specification C04.19.3 /
C04.19.4: the callbacks an application registers in `rk->rk_conf` are read
like `rko_op_cb`, closed over the run's registrations. Abstract rule theorems,
not an extraction of the Rust analyzer
(`crates/qcore-c-gateway/src/callback_member_closure.rs`,
`crates/qcore-c-gateway/src/member_run_facts.rs`,
`crates/qcore-audit/src/member_prepass.rs`,
`crates/qcore-contract/src/contract_constructed_state.rs`).

* **Registrations are stores (Q2).** A declared setter
  `resource_callback_member_registered(conf, TAG.FIELD, cb)` contributes, at
  every call the run's units make to it, the reading of its actual, exactly
  as a store into the member would. The member closes only when every unit is
  read and every reading (store or registration) is closed
  (`a_registration_closes_the_member_only_with_every_run_store`), and its
  candidates are exactly the read ones
  (`the_registered_set_is_exactly_the_read_and_registered_sets`).
* **A stated default is a candidate (D1).** `quant live #result: TAG.FIELD
  |-> f;` and its lower level `resource_callback_member_registered(result,
  TAG.FIELD, f)` mean the same (`the_function_shorthand_and_its_parts_mean_the_same`,
  `both_function_levels_are_one_canonical_set`), and the stated function joins
  the run set (`a_stated_default_is_a_candidate`).
* **The run's set decides (R1, E1).** A unit decides over the run's set read
  by the pre-pass: an open set never decides
  (`an_open_run_member_never_decides`); a closed, empty run set invokes
  nothing (`a_closed_empty_run_set_invokes_nothing`); a closed non-empty one
  decides as rule 13u does (`a_closed_run_set_decides_as_the_unit_rule`). The
  unit-level reading keeps refusing an empty set
  (`a_unit_empty_set_stays_refused`).
* **Skipping a unit is sound only where it names nothing (F1).** The pre-pass
  reads every unit whose preprocessed text names a token of the member's
  closure; a unit that names none adds nothing, so skipping it keeps the run
  set (`skipping_a_silent_unit_keeps_the_run_set`), while an unread unit that
  names one keeps the member open (`an_unread_naming_unit_keeps_it_open`).
* **Keys (Q3).** A member is keyed by its innermost named record: a store
  through an embedded record and a store through a pointer to that record
  reach one key (`an_embedded_store_and_a_direct_store_share_the_key`), and
  normalising twice changes nothing (`normalise_is_idempotent`).
-/

namespace QCore.AppRkCallbacks

open QCore.CallbackContracts (Decision)
open QCore.OpCallbacks (Closure UnitReading runClosure SuppliedClass decideSupplied memberDecision)

/-! ## Registrations are stores (Q2) -/

/-- The run's closure over the units' readings and the declared
registrations' readings (each registration contributes its actual's reading,
as a store does). -/
def registeredClosure (units : List UnitReading) (registrations : List Closure) : Closure :=
  runClosure (units ++ registrations.map some)

/-- A registration's own reading is part of the run set: if the run closes,
every registered designator is a candidate. -/
theorem the_registered_set_is_exactly_the_read_and_registered_sets
    {us : List UnitReading} {regs : List Closure} {rs : List String}
    (h : registeredClosure us regs = .closed rs) (x : String) :
    x ∈ rs ↔ ∃ ns, (some (Closure.closed ns) ∈ us ∨ Closure.closed ns ∈ regs) ∧ x ∈ ns := by
  rw [QCore.OpCallbacks.the_run_set_is_exactly_the_read_sets h x]
  constructor
  · rintro ⟨ns, hm, hx⟩
    rcases List.mem_append.mp hm with hu | hr
    · exact ⟨ns, Or.inl hu, hx⟩
    · obtain ⟨c, hc, he⟩ := List.mem_map.mp hr
      cases he
      exact ⟨ns, Or.inr hc, hx⟩
  · rintro ⟨ns, hm | hm, hx⟩
    · exact ⟨ns, List.mem_append.mpr (Or.inl hm), hx⟩
    · exact ⟨ns, List.mem_append.mpr (Or.inr (List.mem_map.mpr ⟨_, hm, rfl⟩)), hx⟩

/-- **A setter registration closes the member only together with every run
store**: one unread unit, one open store or one open registration keeps it
open. -/
theorem a_registration_closes_the_member_only_with_every_run_store
    {us : List UnitReading} {regs : List Closure} {rs : List String}
    (h : registeredClosure us regs = .closed rs) :
    (∀ u ∈ us, ∃ ns, u = some (.closed ns)) ∧ (∀ r ∈ regs, ∃ ns, r = .closed ns) := by
  have all : ∀ u ∈ us ++ regs.map some, ∃ ns, u = some (.closed ns) := by
    unfold registeredClosure at h
    generalize us ++ regs.map some = xs at h
    induction xs generalizing rs with
    | nil => intro u hu; cases hu
    | cons u rest ih =>
      rcases u with _ | (ns | _)
      · simp [runClosure] at h
      · cases hr : runClosure rest with
        | opened => simp [runClosure, hr] at h
        | closed rs' =>
          intro v hv
          rcases List.mem_cons.mp hv with he | hv
          · exact ⟨ns, he⟩
          · exact ih hr v hv
      · simp [runClosure] at h
  refine ⟨fun u hu => all u (List.mem_append.mpr (Or.inl hu)), fun r hr => ?_⟩
  obtain ⟨ns, he⟩ := all (some r) (List.mem_append.mpr (Or.inr (List.mem_map.mpr ⟨r, hr, rfl⟩)))
  exact ⟨ns, Option.some.inj he⟩

/-- An unread unit keeps the registered member open. -/
theorem an_unread_unit_keeps_the_registered_member_open
    {us : List UnitReading} (regs : List Closure) (h : none ∈ us) :
    registeredClosure us regs = .opened :=
  QCore.OpCallbacks.an_unread_unit_keeps_the_member_open (List.mem_append.mpr (Or.inl h))

/-! ## The run's set decides (R1, E1) -/

/-- The decision of a member invocation over the run's set (the pre-pass):
open refuses, a closed empty set invokes nothing (the identity), a closed
non-empty set is decided as rule 13u decides it. -/
def runDecision (c : Closure) (classOf : String → SuppliedClass) : Decision :=
  match c with
  | .opened => .refused
  | .closed [] => .identity
  | .closed names => decideSupplied false (names.map classOf)

/-- **An open run member never decides.** -/
theorem an_open_run_member_never_decides (classOf : String → SuppliedClass) :
    runDecision .opened classOf = .refused := rfl

/-- **E1: a closed, empty run set invokes nothing.** -/
theorem a_closed_empty_run_set_invokes_nothing (classOf : String → SuppliedClass) :
    runDecision (.closed []) classOf = .identity := rfl

/-- A closed non-empty run set decides exactly as the unit rule does over the
same set. -/
theorem a_closed_run_set_decides_as_the_unit_rule {names : List String} (h : names ≠ [])
    (classOf : String → SuppliedClass) :
    runDecision (.closed names) classOf = memberDecision (.closed names) classOf := by
  cases names with
  | nil => exact absurd rfl h
  | cons _ _ => rfl

/-- The unit-level reading (no pre-pass) keeps refusing an empty set: E1 is
read only over the run's closed set. -/
theorem a_unit_empty_set_stays_refused (classOf : String → SuppliedClass) :
    memberDecision (.closed []) classOf = .refused := rfl

/-- The fold over the pre-pass: a unit decided over the run's set itself, so
the fold keeps a decision exactly when the run set is closed. -/
theorem the_fold_keeps_a_run_decision_exactly_when_closed (c : Closure) :
    QCore.OpCallbacks.foldKeeps c c = true ↔ ∃ ns, c = .closed ns := by
  cases c with
  | opened => simp [QCore.OpCallbacks.foldKeeps]
  | closed ns =>
    simp only [QCore.OpCallbacks.foldKeeps, List.all_eq_true]
    exact ⟨fun _ => ⟨ns, rfl⟩, fun _ x hx => by simpa using hx⟩

/-! ## Skipping a unit (F1) -/

/-- A unit the pre-pass skips because its preprocessed text names no token of
the member's closure graph: under C's rule that a member is reached only
through its record's declaration and a function only through its name, its
facts hold no store into the member (the hypothesis), so its reading is the
empty closed set. -/
theorem skipping_a_silent_unit_keeps_the_run_set (us : List UnitReading) :
    runClosure (some (.closed []) :: us) = runClosure us :=
  QCore.OpCallbacks.a_read_unit_without_a_store_adds_nothing us

/-- A unit that names a token but whose AST is not read (a C++ or assembly
unit, a failed preprocessing) keeps the member open. -/
theorem an_unread_naming_unit_keeps_it_open {us : List UnitReading} :
    runClosure (none :: us) = .opened := rfl

/-! ## A stated default (D1)

The member state's value is `absent` (no callback) or a function `f`. -/

/-- A returned object: the pointer value and what each function-pointer
member holds (`none`: `absent`; `some f`: the function `f`). -/
abbrev Result := QCore.OpCallbacks.Result

/-- A canonical part: liveness, or a member's value. -/
inductive Part where
  | live
  | maps (m : String) (v : Option String)
  deriving DecidableEq, Repr

/-- What a part says of the returned object. -/
def Part.holds : Part → Result → Prop
  | .live, r => QCore.DerefDecomposition.liveness r.ptr
  | .maps m v, r => r.member m = v

/-- A written form: the shorthand `quant live #result: m |-> v`, or one of
its two parts (`quant live #result;`,
`resource_callback_member_registered(result, m, v)`). -/
inductive Form where
  | shorthand (m : String) (v : Option String)
  | live
  | registered (m : String) (v : Option String)
  deriving DecidableEq, Repr

/-- The loader's elaboration into canonical parts. -/
def elaborate : Form → List Part
  | .shorthand m v => [.live, .maps m v]
  | .live => [.live]
  | .registered m v => [.maps m v]

/-- What a list of written forms says. -/
def meaning (fs : List Form) (r : Result) : Prop :=
  ∀ f ∈ fs, ∀ p ∈ elaborate f, p.holds r

/-- **The shorthand with a function value and its two parts mean the
same.** -/
theorem the_function_shorthand_and_its_parts_mean_the_same (m f : String) (r : Result) :
    meaning [.shorthand m (some f)] r ↔ meaning [.live, .registered m (some f)] r := by
  simp [meaning, elaborate]

/-- The shorthand means the result is live and its member holds `f`. -/
theorem the_function_shorthand_is_live_and_holds_f (m f : String) (r : Result) :
    meaning [.shorthand m (some f)] r ↔
      QCore.DerefDecomposition.liveness r.ptr ∧ r.member m = some f := by
  simp [meaning, elaborate, Part.holds]

/-- The `absent` reading of `QCore.OpCallbacks` is the `none` value here. -/
theorem the_absent_value_is_the_earlier_absent_state (m : String) (r : Result) :
    meaning [.shorthand m none] r ↔ QCore.OpCallbacks.meaning [.shorthand m] r := by
  simp [meaning, elaborate, Part.holds, QCore.OpCallbacks.meaning,
    QCore.OpCallbacks.elaborate, QCore.OpCallbacks.Part.holds]

/-- The canonical set of a list of forms: whether liveness is stated and each
member's stated values. -/
def canonical (fs : List Form) : Bool × (String → Option String → Bool) :=
  (fs.any (fun f => (elaborate f).contains .live),
   fun x v => fs.any (fun f => (elaborate f).contains (.maps x v)))

/-- **Both levels, and both together, are one canonical set**, for a function
value as for `absent`. -/
theorem both_function_levels_are_one_canonical_set (m : String) (v : Option String) :
    canonical [.shorthand m v] = canonical [.live, .registered m v] ∧
      canonical [.shorthand m v, .live, .registered m v] = canonical [.shorthand m v] := by
  constructor <;>
    · refine Prod.ext ?_ (funext fun x => funext fun w => ?_)
      · simp [canonical, elaborate]
      · by_cases hx : x = m <;> by_cases hw : w = v <;> simp [canonical, elaborate, hx, hw]

/-- A stated default is a registration of the constructor's result: its
reading is the closed set holding the function (`absent`: the empty one). -/
def defaultReading : Option String → Closure
  | none => .closed []
  | some f => .closed [f]

/-- **A stated default is a candidate** whenever the run closes. -/
theorem a_stated_default_is_a_candidate {us : List UnitReading} {regs : List Closure}
    {rs : List String} (f : String) (h : registeredClosure us (defaultReading (some f) :: regs) = .closed rs) :
    f ∈ rs := by
  rw [the_registered_set_is_exactly_the_read_and_registered_sets h f]
  exact ⟨[f], Or.inr (List.mem_cons_self ..), List.mem_singleton_self f⟩

/-! ## Keys (Q3)

A member key is a record and a path. The record table says, for a record and
a field, which named record the field holds by value (`some R`), if any. A key
is normalised by restarting at that record whenever a path segment holds one,
so it ends keyed by its innermost named record. -/

/-- A record table: the named record a field holds by value. -/
abbrev Table := String → String → Option String

/-- Normalise a key `(tag, path)`: restart at the innermost named record. -/
def normalise (t : Table) : String → List String → String × List String
  | tag, [] => (tag, [])
  | tag, [f] => (tag, [f])
  | tag, a :: b :: rest =>
      match t tag a with
      | some r => normalise t r (b :: rest)
      | none => (tag, a :: b :: rest)

/-- **A store through an embedded record and a store through a pointer to
that record reach one key** (`rk->rk_conf.dr_msg_cb` and `conf->dr_msg_cb`
are both `rd_kafka_conf_s.dr_msg_cb`). -/
theorem an_embedded_store_and_a_direct_store_share_the_key (t : Table) (tag a r b : String)
    (rest : List String) (h : t tag a = some r) :
    normalise t tag (a :: b :: rest) = normalise t r (b :: rest) := by
  simp [normalise, h]

/-- A normalised key's head segment holds no named record by value. -/
theorem a_normal_key_restarts_nowhere (t : Table) (tag : String) (path : List String) :
    ∀ a b rest, (normalise t tag path).2 = a :: b :: rest →
      t (normalise t tag path).1 a = none := by
  induction path generalizing tag with
  | nil => intro a b rest h; simp [normalise] at h
  | cons x xs ih =>
    intro a b rest h
    cases xs with
    | nil => simp [normalise] at h
    | cons y ys =>
      cases hx : t tag x with
      | none =>
        simp only [normalise, hx] at h ⊢
        simp only [List.cons.injEq] at h
        obtain ⟨rfl, -⟩ := h
        exact hx
      | some r =>
        simp only [normalise, hx] at h ⊢
        exact ih r a b rest h

/-- **Normalising twice changes nothing.** -/
theorem normalise_is_idempotent (t : Table) (tag : String) (path : List String) :
    normalise t (normalise t tag path).1 (normalise t tag path).2 = normalise t tag path := by
  generalize hk : normalise t tag path = k
  obtain ⟨k1, k2⟩ := k
  have hn := a_normal_key_restarts_nowhere t tag path
  rw [hk] at hn
  simp only at hn ⊢
  match k2, hn with
  | [], _ => rfl
  | [_], _ => rfl
  | a :: b :: rest, hn => simp [normalise, hn a b rest rfl]

/-- `KEY_VECTORS` (`callback_member_closure.rs`): a table, a key and its
normal form. -/
def keyVectors : List (List (String × String × String) × (String × List String) × (String × List String)) := [
  ([("rd_kafka_s", "rk_conf", "rd_kafka_conf_s")], ("rd_kafka_s", ["rk_conf", "dr_msg_cb"]),
    ("rd_kafka_conf_s", ["dr_msg_cb"])),
  ([("rd_kafka_s", "rk_conf", "rd_kafka_conf_s")], ("rd_kafka_conf_s", ["dr_msg_cb"]),
    ("rd_kafka_conf_s", ["dr_msg_cb"])),
  ([], ("rd_kafka_conf_s", ["sasl", "oauthbearer", "token_refresh_cb"]),
    ("rd_kafka_conf_s", ["sasl", "oauthbearer", "token_refresh_cb"])),
  ([("a", "b", "c"), ("c", "d", "e")], ("a", ["b", "d", "f"]), ("e", ["f"]))
]

/-- The table a vector names. -/
def tableOf (rows : List (String × String × String)) : Table :=
  fun tag field => (rows.find? (fun (t, f, _) => t == tag && f == field)).map (·.2.2)

theorem key_vectors_hold :
    keyVectors.all (fun (rows, (tag, path), expected) =>
      normalise (tableOf rows) tag path == expected) = true := by
  decide

/-- `RUN_DECISION_VECTORS` (`member_run_facts.rs`): a run closure (`none`
open) and whether a run decision over identity candidates is admitted. -/
def runDecisionVectors : List (Option (List String) × Bool) := [
  (none, false),
  (some [], true),
  (some ["a"], true),
  (some ["a", "b"], true)
]

theorem run_decision_vectors_hold :
    runDecisionVectors.all (fun (c, admitted) =>
      (runDecision (match c with | none => .opened | some ns => .closed ns)
        (fun _ => .identity) == .identity) == admitted) = true := by
  decide

end QCore.AppRkCallbacks
