import QCore.AppRkCallbacks

/-!
# Unread C++ units scoped to the members they name (card CALLBACK-MEMBER-NON-C-UNITS)

Gateway rule 12n, amended in place. The member-facts pre-pass leaves a unit
unread when it names a token of some invoked member's closure graph but its
facts are not read. Before this card every unread unit opened every member.
Now a C++ unit whose preprocessed text (its own command, `-E`) is the whole
unit opens only a member whose own graph tokens it names, or one with a graph
token it was never checked against; every other unread unit (a failed
preprocessing, an assembly unit, a C unit whose AST was not read, a C++ unit
reading a precompiled header or module) still opens every member. Abstract
rule theorems, not an extraction of the Rust analyzer
(`crates/qcore-c-gateway/src/member_run_facts.rs` `RunMemberFacts::opens`,
`crates/qcore-audit/src/member_prepass.rs` `scoped_cxx`).

The reading's premise is the one under which the pre-pass already skips a C
unit naming no token (`QCore.AppRkCallbacks.skipping_a_silent_unit_keeps_the_run_set`):
a unit holding a store the member's closure reads names a token of the
member's graph in its text (`Reaches`). It is a hypothesis of the theorems
below, not an axiom.

* An undecided unit keeps the member open
  (`an_undecided_unit_keeps_the_member_open`), as does a whole-text unit naming
  a graph token (`a_named_graph_token_keeps_the_member_open`) or one never
  checked for a graph token (`an_unchecked_graph_token_keeps_the_member_open`).
* Under the premise, a scoped unit that keeps the member's set holds no store
  into it (`a_scoped_unit_naming_no_graph_token_holds_no_store`), so it adds
  nothing to the run set
  (`a_scoped_unit_naming_no_graph_token_keeps_the_run_set`).
* **The closure stays a superset of every store site:** when the run's set is
  closed, every designator any unit stores, read or unread, is in it
  (`the_closed_run_set_covers_every_store`).
* What the pre-pass records equals what it checks
  (`the_recorded_reading_is_the_checked_reading`), and the Rust vectors hold
  (`scoped_vectors_hold`).
-/

namespace QCore.MemberNonCUnits

open QCore.OpCallbacks (Closure UnitReading runClosure)

/-- How an unread unit's text was read: `undecided` (its preprocessing
failed, an assembly unit, a C unit whose AST was not read, a C++ unit whose
command or text reads a precompiled header or module), or the whole text of a
C++ unit, with which tokens it names. -/
inductive Text where
  | undecided
  | whole (names : String → Bool)

/-- An unread unit: its text, and the designators its own stores put into the
member (unknown to the pre-pass; the theorems quantify over them). -/
structure Unread where
  text : Text
  stores : List String

/-- What the pre-pass records of a whole-text unit: the checked tokens it
names (the Rust `scoped` entry). -/
def recorded (names : String → Bool) (checked : List String) : List String :=
  checked.filter names

/-- The Rust `RunMemberFacts::opens` over what is recorded: an unscoped unit
(`none`) opens every member; a scoped one opens a member with a graph token
outside `checked` or one it names. -/
def opensRecorded (scope : Option (List String)) (checked graph : List String) : Bool :=
  match scope with
  | none => true
  | some named => !(graph.all (fun t => checked.contains t)) || graph.any (fun t => named.contains t)

/-- The pre-pass's reading of a unit's text for one member's graph. -/
def opens (u : Text) (checked graph : List String) : Bool :=
  match u with
  | .undecided => opensRecorded none checked graph
  | .whole names => opensRecorded (some (recorded names checked)) checked graph

/-- The unit's reading of the member: unread (`none`) when it opens it,
otherwise the empty closed set. -/
def reading (u : Unread) (checked graph : List String) : UnitReading :=
  if opens u.text checked graph then none else some (.closed [])

/-- The reading's premise for a whole-text unit: a unit holding a store into
the member names a token of the member's graph. -/
def Reaches (u : Unread) (graph : List String) : Prop :=
  ∀ names, u.text = .whole names → u.stores ≠ [] → ∃ t ∈ graph, names t = true

/-- What is recorded is what is checked: the reading over the unit's own text
and over its recorded tokens agree. -/
theorem the_recorded_reading_is_the_checked_reading (names : String → Bool)
    (checked graph : List String) :
    opens (.whole names) checked graph =
      opensRecorded (some (recorded names checked)) checked graph := rfl

/-- **An undecided unit keeps the member open**, whatever it names. -/
theorem an_undecided_unit_keeps_the_member_open (stores : List String)
    (checked graph : List String) (rs : List UnitReading) :
    runClosure (rs ++ [reading ⟨.undecided, stores⟩ checked graph]) = .opened := by
  apply QCore.OpCallbacks.an_unread_unit_keeps_the_member_open
  simp [reading, opens, opensRecorded]

/-- A whole-text unit naming a checked graph token keeps the member open. -/
theorem a_named_graph_token_keeps_the_member_open {names : String → Bool}
    {checked graph : List String} {t : String} (hg : t ∈ graph) (hc : t ∈ checked)
    (hn : names t = true) :
    opens (.whole names) checked graph = true := by
  have hr : (recorded names checked).contains t = true :=
    List.contains_iff_mem.mpr (List.mem_filter.mpr ⟨hc, hn⟩)
  have hany : (graph.any fun t' => (recorded names checked).contains t') = true :=
    List.any_eq_true.mpr ⟨t, hg, hr⟩
  simp only [opens, opensRecorded, hany, Bool.or_true]

/-- A graph token the unit was never checked against keeps the member open. -/
theorem an_unchecked_graph_token_keeps_the_member_open {names : String → Bool}
    {checked graph : List String} {t : String} (hg : t ∈ graph) (hc : t ∉ checked) :
    opens (.whole names) checked graph = true := by
  have hn : ¬ checked.contains t = true := fun h => hc (List.contains_iff_mem.mp h)
  have hall : (graph.all fun t' => checked.contains t') = false :=
    List.all_eq_false.mpr ⟨t, hg, hn⟩
  simp only [opens, opensRecorded, hall, Bool.not_false, Bool.true_or]

/-- **Under the premise, a scoped unit that keeps the member's set holds no
store into it.** -/
theorem a_scoped_unit_naming_no_graph_token_holds_no_store {u : Unread}
    {names : String → Bool} {checked graph : List String} (ht : u.text = .whole names)
    (hr : Reaches u graph) (ho : opens u.text checked graph = false) :
    u.stores = [] := by
  rw [ht] at ho
  cases hs : u.stores with
  | nil => rfl
  | cons s ss =>
    obtain ⟨t, hg, hn⟩ := hr names ht (by rw [hs]; exact List.cons_ne_nil s ss)
    cases hc : checked.contains t with
    | false =>
      have := an_unchecked_graph_token_keeps_the_member_open (names := names) hg
        (fun h => by rw [List.contains_iff_mem.mpr h] at hc; cases hc)
      rw [this] at ho
      cases ho
    | true =>
      have := a_named_graph_token_keeps_the_member_open hg (List.contains_iff_mem.mp hc) hn
      rw [this] at ho
      cases ho

/-- **A scoped unit naming no graph token keeps the run set**: its reading
is the empty closed set, which adds nothing. -/
theorem a_scoped_unit_naming_no_graph_token_keeps_the_run_set {u : Unread}
    {checked graph : List String} (ho : opens u.text checked graph = false)
    (rs : List UnitReading) :
    runClosure (reading u checked graph :: rs) = runClosure rs := by
  rw [show reading u checked graph = some (.closed []) by simp [reading, ho]]
  exact QCore.AppRkCallbacks.skipping_a_silent_unit_keeps_the_run_set rs

/-- A closed run set read no unread unit. -/
theorem a_closed_run_read_every_unit {us : List UnitReading} {rs : List String}
    (h : runClosure us = .closed rs) : none ∉ us := by
  intro hn
  rw [QCore.OpCallbacks.an_unread_unit_keeps_the_member_open hn] at h
  cases h

/-- **The closure stays a superset of every store site.** The read units'
readings are exact (each its own stores), and each unread unit is read by the
pre-pass under the premise. When the run's set is closed, every designator
any unit stores is in it. -/
theorem the_closed_run_set_covers_every_store {read : List (List String)}
    {unread : List Unread} {checked graph rs : List String}
    (hr : ∀ u ∈ unread, Reaches u graph)
    (h : runClosure (read.map (fun s => some (Closure.closed s)) ++
      unread.map (fun u => reading u checked graph)) = .closed rs) :
    (∀ s ∈ read, ∀ x ∈ s, x ∈ rs) ∧ (∀ u ∈ unread, ∀ x ∈ u.stores, x ∈ rs) := by
  have hx := QCore.OpCallbacks.the_run_set_is_exactly_the_read_sets h
  have hnone := a_closed_run_read_every_unit h
  constructor
  · intro s hs x hxs
    exact (hx x).mpr ⟨s, List.mem_append.mpr (Or.inl (List.mem_map.mpr ⟨s, hs, rfl⟩)), hxs⟩
  · intro u hu x hxs
    have hmem : reading u checked graph ∈
        read.map (fun s => some (Closure.closed s)) ++
          unread.map (fun u => reading u checked graph) :=
      List.mem_append.mpr (Or.inr (List.mem_map.mpr ⟨u, hu, rfl⟩))
    have ho : opens u.text checked graph = false := by
      cases hop : opens u.text checked graph with
      | false => rfl
      | true =>
        have : reading u checked graph = none := by simp [reading, hop]
        exact absurd (this ▸ hmem) hnone
    cases ht : u.text with
    | undecided =>
      rw [ht] at ho
      simp [opens, opensRecorded] at ho
    | whole names =>
      have := a_scoped_unit_naming_no_graph_token_holds_no_store ht (hr u hu) ho
      rw [this] at hxs
      cases hxs

/-- `SCOPED_VECTORS` (`member_run_facts.rs`): the recorded reading (`none`
not scoped), the checked tokens, a member's graph tokens, and whether the unit
opens the member. -/
def scopedVectors : List (Option (List String) × List String × List String × Bool) := [
  (none, ["cmt"], ["cmt"], true),
  (none, [], [], true),
  (some [], ["cmt"], ["cmt"], false),
  (some ["event"], ["cmt", "event"], ["cmt"], false),
  (some ["cmt", "event"], ["cmt", "event"], ["cmt"], true),
  (some [], ["cmt"], ["cmt", "cio_options"], true),
  (some ["event"], ["event", "rd_kafka_s"], ["rd_kafka_s", "rd_kafka_conf_s"], true),
  (some ["event"], ["event", "rd_kafka_s", "rd_kafka_conf_s"], ["rd_kafka_s", "rd_kafka_conf_s"], false)
]

theorem scoped_vectors_hold :
    scopedVectors.all (fun (n, c, g, o) => opensRecorded n c g == o) = true := by
  decide

end QCore.MemberNonCUnits
