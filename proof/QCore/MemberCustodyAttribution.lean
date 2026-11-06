/-!
# The gate's refusal of member custody through a formal parent names its premise (card MEMBER-EFFECT-SUMMARY step 1, gateway rule 38i)

The abstract rule implemented by
`crates/qcore-c-gateway/src/legacy_member_custody_attribution.rs`
(`Parser::member_custody_attribution`).

Asked last, only for exactly the lexical gate's refusal (`indirect memory
access is outside the resource projection`) of a statement with no braces
that reads no tracked descriptor, whose base `P` is a formal the body never
rebinds. Three shapes are read: a whole-statement call with `P->F` at a
`Released` position of a contract in force (release), the same with
`&P->F[.G…]` (address), and `P->F = V` with `V` custody-carrying (store). The
two call shapes are confirmed by the classification with every memory-reading
actual read as at a `Borrowed` position: the name is taken only when that
reading **admits** the call. Otherwise the refusal stays byte for byte.

This module proves that the rule is a re-attribution: it changes at most the
description of the gate's refusal, never its writes, never any other node,
never the verdict of any node, hence never the status of a function (the join
of its nodes' verdicts, `Safe < Pending < Unknown < Unsafe`); and each name is
taken only under every one of its premises.

## What this is not

A model of the classifier or of the lexical shape reader. Whether a statement
has one of the shapes, whether a name is declared a pointer, and what the
borrowed reading answers are the Rust's; that the reading leaves no state
behind (its counters are restored and its instruction discarded) is reviewed
and bound by tests; faithful token extraction is trust boundary 28.
-/

namespace QCore.MemberCustodyAttribution

/-- The four-valued verdict and its order `Safe < Pending < Unknown < Unsafe`. -/
inductive Verdict where
  | safe
  | pending
  | unknown
  | «unsafe»
  deriving DecidableEq, Repr

namespace Verdict

def rank : Verdict → Nat
  | .safe => 0
  | .pending => 1
  | .unknown => 2
  | .«unsafe» => 3

/-- The join (the composition of verdicts). -/
def join (a b : Verdict) : Verdict := if a.rank ≤ b.rank then b else a

end Verdict

/-- The three premises. -/
inductive Premise where
  /-- `member-release-through-formal`. -/
  | release
  /-- `member-address-release-through-formal`. -/
  | address
  /-- `member-store-through-formal`. -/
  | store
  deriving DecidableEq, Repr

/-- A refusal's description, as far as the rule distinguishes it. -/
inductive Description where
  /-- `indirect memory access is outside the resource projection`. -/
  | gate
  /-- One of the rule's three names. -/
  | named (premise : Premise)
  /-- A held call's refusal (review or pin hold): its verdict is `Pending`. -/
  | held (callee : String)
  /-- Every other refusal. -/
  | other (text : String)
  deriving DecidableEq, Repr

/-- One lowered node: a refusal with its writes, or anything else. -/
inductive Node where
  | refusal (description : Description) (writes : List String)
  | admitted (name : String)
  deriving DecidableEq, Repr

/-- The verdict one node contributes (`legacy_verdict::diagnostic_verdict`):
a held call is `Pending`, every other refusal `Unknown`; an admitted node
contributes nothing (`Safe`, the unit of the join). -/
def verdict : Node → Verdict
  | .refusal (.held _) _ => .pending
  | .refusal _ _ => .unknown
  | .admitted _ => .safe

/-- A function's status: the join of its nodes' verdicts. -/
def status (nodes : List Node) : Verdict := nodes.foldl (fun v n => v.join (verdict n)) .safe

/-- What the Rust establishes about the statement before naming it. -/
structure Facts where
  /-- The statement has braces or reads a tracked descriptor. -/
  excluded : Bool
  /-- The shape the lexical reader recognised, if any. -/
  shape : Option Premise
  /-- The base is a formal parameter. -/
  formal : Bool
  /-- The typed root reader answers the formal never bound. -/
  neverBound : Bool
  /-- The call's contract in force states `Released` at the actual's
  position, with no review or pin hold (the call shapes). -/
  releasedPosition : Bool
  /-- The value is custody-carrying (the store shape). -/
  custodyValue : Bool
  deriving DecidableEq, Repr

/-- The premises a shape needs besides the reading. -/
def shapeHolds (facts : Facts) : Premise → Bool
  | .release => facts.releasedPosition
  | .address => facts.releasedPosition
  | .store => facts.custodyValue

/-- Whether a shape is confirmed by the borrowed reading (the call shapes). -/
def confirmedByReading : Premise → Bool
  | .release => true
  | .address => true
  | .store => false

/-- Whether the borrowed reading admits the call (`some (admitted _)`); a
refusal or no reading (an actual neither reader recognises) does not. -/
def readingAdmits : Option Node → Bool
  | some (.admitted _) => true
  | _ => false

/-- The name the statement takes, if any. -/
def name (facts : Facts) (reading : Option Node) : Option Premise :=
  match facts.shape with
  | none => none
  | some premise =>
      if !facts.excluded && facts.formal && facts.neverBound && shapeHolds facts premise
          && (!confirmedByReading premise || readingAdmits reading) then
        some premise
      else
        none

/-- The rule. -/
def reattribute (facts : Facts) (reading : Option Node) : Node → Node
  | .refusal .gate writes =>
      match name facts reading with
      | some premise => .refusal (.named premise) writes
      | none => .refusal .gate writes
  | n => n

theorem only_the_gate_refusal_is_read (facts : Facts) (reading : Option Node) (n : Node)
    (h : ∀ w, n ≠ .refusal .gate w) : reattribute facts reading n = n := by
  cases n with
  | refusal d w =>
      cases d with
      | gate => exact absurd rfl (h w)
      | named _ => rfl
      | held _ => rfl
      | other _ => rfl
  | admitted _ => rfl

theorem the_result_is_the_gate_refusal_or_a_name (facts : Facts) (reading : Option Node)
    (w : List String) :
    reattribute facts reading (.refusal .gate w) = .refusal .gate w ∨
      ∃ p, name facts reading = some p ∧
        reattribute facts reading (.refusal .gate w) = .refusal (.named p) w := by
  simp only [reattribute]
  cases h : name facts reading with
  | none => exact Or.inl rfl
  | some p => exact Or.inr ⟨p, rfl, rfl⟩

theorem the_writes_are_kept (facts : Facts) (reading : Option Node) (w : List String) :
    ∃ d, reattribute facts reading (.refusal .gate w) = .refusal d w := by
  rcases the_result_is_the_gate_refusal_or_a_name facts reading w with h | ⟨p, _, h⟩
  · exact ⟨_, h⟩
  · exact ⟨_, h⟩

theorem a_refusal_stays_a_refusal (facts : Facts) (reading : Option Node) (d : Description)
    (w : List String) : ∃ d', reattribute facts reading (.refusal d w) = .refusal d' w := by
  cases d with
  | gate => exact the_writes_are_kept facts reading w
  | named _ => exact ⟨_, rfl⟩
  | held _ => exact ⟨_, rfl⟩
  | other _ => exact ⟨_, rfl⟩

theorem the_verdict_of_every_node_is_unchanged (facts : Facts) (reading : Option Node)
    (n : Node) : verdict (reattribute facts reading n) = verdict n := by
  cases n with
  | admitted _ => rfl
  | refusal d w =>
      cases d with
      | gate =>
          rcases the_result_is_the_gate_refusal_or_a_name facts reading w with h | ⟨p, _, h⟩
          · rw [h]
          · rw [h]; rfl
      | named _ => rfl
      | held _ => rfl
      | other _ => rfl

theorem the_status_of_a_function_is_unchanged (facts : Node → Facts)
    (reading : Node → Option Node) (nodes : List Node) :
    status (nodes.map fun n => reattribute (facts n) (reading n) n) = status nodes := by
  unfold status
  suffices ∀ v : Verdict, List.foldl (fun (v : Verdict) n => v.join (verdict n)) v
      (nodes.map fun n => reattribute (facts n) (reading n) n) =
      List.foldl (fun (v : Verdict) n => v.join (verdict n)) v nodes from this _
  induction nodes with
  | nil => intro v; rfl
  | cons n rest ih =>
      intro v
      simp only [List.map_cons, List.foldl_cons, the_verdict_of_every_node_is_unchanged]
      exact ih _

theorem a_name_needs_every_premise (facts : Facts) (reading : Option Node) (p : Premise)
    (h : name facts reading = some p) :
    facts.shape = some p ∧ facts.excluded = false ∧ facts.formal = true ∧
      facts.neverBound = true ∧ shapeHolds facts p = true ∧
      (confirmedByReading p = true → readingAdmits reading = true) := by
  unfold name at h
  cases hs : facts.shape with
  | none => simp [hs] at h
  | some q =>
      simp only [hs] at h
      split at h
      · rename_i hc
        cases h
        simp only [Bool.and_eq_true, Bool.not_eq_true', Bool.or_eq_true] at hc
        obtain ⟨⟨⟨⟨he, hf⟩, hn⟩, hsh⟩, hr⟩ := hc
        refine ⟨rfl, he, hf, hn, hsh, ?_⟩
        intro hcr
        rcases hr with hr | hr
        · rw [hcr] at hr; cases hr
        · exact hr
      · cases h

theorem an_excluded_statement_keeps_the_gate_refusal (facts : Facts) (reading : Option Node)
    (w : List String) (h : facts.excluded = true) :
    reattribute facts reading (.refusal .gate w) = .refusal .gate w := by
  rcases the_result_is_the_gate_refusal_or_a_name facts reading w with e | ⟨p, hn, _⟩
  · exact e
  · have := (a_name_needs_every_premise facts reading p hn).2.1
    rw [h] at this; cases this

theorem a_non_formal_base_keeps_the_gate_refusal (facts : Facts) (reading : Option Node)
    (w : List String) (h : facts.formal = false) :
    reattribute facts reading (.refusal .gate w) = .refusal .gate w := by
  rcases the_result_is_the_gate_refusal_or_a_name facts reading w with e | ⟨p, hn, _⟩
  · exact e
  · have := (a_name_needs_every_premise facts reading p hn).2.2.1
    rw [h] at this; cases this

theorem a_rebound_formal_keeps_the_gate_refusal (facts : Facts) (reading : Option Node)
    (w : List String) (h : facts.neverBound = false) :
    reattribute facts reading (.refusal .gate w) = .refusal .gate w := by
  rcases the_result_is_the_gate_refusal_or_a_name facts reading w with e | ⟨p, hn, _⟩
  · exact e
  · have := (a_name_needs_every_premise facts reading p hn).2.2.2.1
    rw [h] at this; cases this

theorem an_unrecognised_shape_keeps_the_gate_refusal (facts : Facts) (reading : Option Node)
    (w : List String) (h : facts.shape = none) :
    reattribute facts reading (.refusal .gate w) = .refusal .gate w := by
  simp [reattribute, name, h]

theorem a_refusing_reading_keeps_a_call_shape_unnamed (facts : Facts) (d : Description)
    (w w' : List String) (p : Premise) (hp : confirmedByReading p = true)
    (hs : facts.shape = some p) :
    reattribute facts (some (.refusal d w')) (.refusal .gate w) = .refusal .gate w := by
  rcases the_result_is_the_gate_refusal_or_a_name facts (some (.refusal d w')) w with
    e | ⟨q, hn, _⟩
  · exact e
  · have all := a_name_needs_every_premise facts _ q hn
    rw [hs] at all
    cases all.1
    have := all.2.2.2.2.2 hp
    cases this

theorem no_reading_keeps_a_call_shape_unnamed (facts : Facts) (w : List String) (p : Premise)
    (hp : confirmedByReading p = true) (hs : facts.shape = some p) :
    reattribute facts none (.refusal .gate w) = .refusal .gate w := by
  rcases the_result_is_the_gate_refusal_or_a_name facts none w with e | ⟨q, hn, _⟩
  · exact e
  · have all := a_name_needs_every_premise facts _ q hn
    rw [hs] at all
    cases all.1
    have := all.2.2.2.2.2 hp
    cases this

theorem a_release_needs_a_released_position (facts : Facts) (reading : Option Node)
    (w : List String) (hs : facts.shape = some .release) (h : facts.releasedPosition = false) :
    reattribute facts reading (.refusal .gate w) = .refusal .gate w := by
  rcases the_result_is_the_gate_refusal_or_a_name facts reading w with e | ⟨q, hn, _⟩
  · exact e
  · have all := a_name_needs_every_premise facts _ q hn
    rw [hs] at all
    cases all.1
    have := all.2.2.2.2.1
    simp [shapeHolds, h] at this

theorem a_store_needs_a_custody_value (facts : Facts) (reading : Option Node)
    (w : List String) (hs : facts.shape = some .store) (h : facts.custodyValue = false) :
    reattribute facts reading (.refusal .gate w) = .refusal .gate w := by
  rcases the_result_is_the_gate_refusal_or_a_name facts reading w with e | ⟨q, hn, _⟩
  · exact e
  · have all := a_name_needs_every_premise facts _ q hn
    rw [hs] at all
    cases all.1
    have := all.2.2.2.2.1
    simp [shapeHolds, h] at this

theorem a_named_leaf_is_never_pending (facts : Facts) (reading : Option Node) (n : Node) :
    verdict (reattribute facts reading n) = .pending → verdict n = .pending := by
  rw [the_verdict_of_every_node_is_unchanged]; exact id

theorem the_attribution_is_idempotent (facts : Facts) (reading : Option Node) (n : Node) :
    reattribute facts reading (reattribute facts reading n) = reattribute facts reading n := by
  cases n with
  | admitted _ => rfl
  | refusal d w =>
      cases d with
      | gate =>
          rcases the_result_is_the_gate_refusal_or_a_name facts reading w with h | ⟨p, _, h⟩
          · rw [h, h]
          · rw [h]; rfl
      | named _ => rfl
      | held _ => rfl
      | other _ => rfl

/-- `free(opts->a);` in `cmt_opts_exit`-shaped body: the reading admits, the
leaf is named and stays `Unknown`. -/
example :
    reattribute ⟨false, some .release, true, true, true, false⟩ (some (.admitted "free"))
        (.refusal .gate ["opts"]) =
      .refusal (.named .release) ["opts"] := by decide

/-- The same statement whose borrowed reading still refuses keeps the gate. -/
example :
    reattribute ⟨false, some .release, true, true, true, false⟩
        (some (.refusal (.other "x") [])) (.refusal .gate ["opts"]) =
      .refusal .gate ["opts"] := by decide

/-- `map->unit = cfl_sds_create(unit);`: a store needs no reading. -/
example :
    reattribute ⟨false, some .store, true, true, false, true⟩ none (.refusal .gate ["map"]) =
      .refusal (.named .store) ["map"] := by decide

end QCore.MemberCustodyAttribution
