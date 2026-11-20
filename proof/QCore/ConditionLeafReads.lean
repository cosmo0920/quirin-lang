/-!
AG3-CONDITION-RETURN-CALLS (gateway rule 63): two readings of a controlling
expression the condition desugaring (rule 5h) refused on a call-bearing leaf.

(1) A residual read beside identity calls. The leaf's calls lower to
instructions; the residual (each call replaced by a literal) is read by the
typed condition read. C leaves the order of the calls and the residual's
operand reads unspecified. The rule admits the read only when every lowered
instruction is the identity on the abstract state, so every interleaving of
the calls and the read reaches the same state and checks the read's
obligations against the same state. The read refines no edge.

(2) A named refusal for an uncontracted call. The refusal node keeps its
kind, its writes and its successors; only its description changes, so the
verdict, which never reads a description, is unchanged.

Extraction (Clang tokens, the classifier's answers, the typed reader) and the
Rust-to-Lean correspondence are premises (trust boundary 28), not proved here.
-/
namespace QCore.ConditionLeafReads

/-- One lowered instruction over an abstract state `σ`: its transfer and the
    obligation it checks against the state it runs in. -/
structure Instr (σ : Type) where
  transfer : σ → σ
  check : σ → Bool

/-- The identity on the abstract state (an identity contract call, an actual
    read with no destination). -/
def Instr.identity {σ : Type} (i : Instr σ) : Prop := ∀ s, i.transfer s = s

def run {σ : Type} : List (Instr σ) → σ → σ
  | [], s => s
  | i :: is, s => run is (i.transfer s)

/-- The obligations every instruction checks, each against the state it runs in. -/
def checks {σ : Type} : List (Instr σ) → σ → List Bool
  | [], _ => []
  | i :: is, s => i.check s :: checks is (i.transfer s)

theorem run_identities {σ : Type} (is : List (Instr σ))
    (h : ∀ i ∈ is, i.identity) (s : σ) : run is s = s := by
  induction is generalizing s with
  | nil => rfl
  | cons i rest ih =>
    simp only [run]
    rw [h i (by simp) s]
    exact ih (fun j hj => h j (by simp [hj])) s

theorem checks_identities {σ : Type} (is : List (Instr σ))
    (h : ∀ i ∈ is, i.identity) (s : σ) : checks is s = is.map (fun i => i.check s) := by
  induction is generalizing s with
  | nil => rfl
  | cons i rest ih =>
    simp only [checks, List.map]
    rw [h i (by simp) s]
    rw [ih (fun j hj => h j (by simp [hj])) s]

/-- Any order of a list of identity instructions reaches the same state. -/
theorem every_order_reaches_the_same_state {σ : Type} (is js : List (Instr σ))
    (perm : is.Perm js) (h : ∀ i ∈ is, i.identity) (s : σ) :
    run is s = run js s := by
  rw [run_identities is h s,
    run_identities js (fun j hj => h j ((perm.mem_iff).mpr hj)) s]

/-- Any order of a list of identity instructions checks every obligation
    against the entry state, so the checks are the same multiset. -/
theorem every_order_checks_the_same_obligations {σ : Type} (is js : List (Instr σ))
    (perm : is.Perm js) (h : ∀ i ∈ is, i.identity) (s : σ) :
    (checks is s).Perm (checks js s) := by
  rw [checks_identities is h s,
    checks_identities js (fun j hj => h j ((perm.mem_iff).mpr hj)) s]
  exact perm.map _

/-- The rule's order (the calls, then the residual read) and C's other
    order (the read first) reach the same state and the read checks its
    obligation against the same state, when every call is an identity. -/
theorem the_read_sees_the_same_state_in_either_order {σ : Type}
    (calls : List (Instr σ)) (read : Instr σ)
    (h : ∀ i ∈ calls, i.identity) (s : σ) :
    run calls s = s ∧ read.check (run calls s) = read.check s := by
  have := run_identities calls h s
  exact ⟨this, by rw [this]⟩

/-- The rule's admission: every lowered instruction an identity, and a read
    the typed condition read decided. -/
def admits (allIdentity readDecided switchPosition : Bool) : Bool :=
  allIdentity && readDecided && !switchPosition

theorem admission_needs_identity_calls (r w : Bool) : admits false r w = false := by
  simp [admits]

theorem admission_needs_a_decided_read (a w : Bool) : admits a false w = false := by
  simp [admits]

theorem a_switch_keeps_its_refusal (a r : Bool) : admits a r true = false := by
  simp [admits]

theorem admission_exactly (a r w : Bool) :
    admits a r w = true ↔ a = true ∧ r = true ∧ w = false := by
  cases a <;> cases r <;> cases w <;> simp [admits]

/-- A branch on a read keeps both edges with the post-read state: the leaf's
    value (whose call operands the residual replaced by a literal) never
    prunes a path. -/
def readBranch {σ : Type} (s : σ) : List (Bool × σ) := [(true, s), (false, s)]

theorem the_read_refines_no_edge {σ : Type} (s : σ) (edge : Bool) :
    (edge, s) ∈ readBranch s := by
  cases edge <;> simp [readBranch]

/-! (2) The named refusal. -/

/-- A refusal node: its description, its writes and its successors. -/
structure Refusal (ω ν : Type) where
  description : String
  writes : ω
  successors : List ν

/-- The verdict a refusal contributes: it reads the writes and successors and
    is `Unknown`; the description is presentation. -/
inductive Verdict where
  | unknown
  deriving DecidableEq

def verdictOf {ω ν : Type} (_ : Refusal ω ν) : Verdict := .unknown

def renamed {ω ν : Type} (r : Refusal ω ν) (description : String) : Refusal ω ν :=
  { r with description := description }

theorem the_attribution_never_admits {ω ν : Type} (r : Refusal ω ν) (d : String) :
    verdictOf (renamed r d) = .unknown := rfl

theorem the_attribution_keeps_the_verdict {ω ν : Type} (r : Refusal ω ν) (d : String) :
    verdictOf (renamed r d) = verdictOf r := rfl

theorem the_attribution_keeps_writes_and_edges {ω ν : Type} (r : Refusal ω ν) (d : String) :
    (renamed r d).writes = r.writes ∧ (renamed r d).successors = r.successors :=
  ⟨rfl, rfl⟩

/-- A refusal's text as the refused-operand hand-on rule reads it: a body and
    the position it names. The rule classifies a refusal as a controlling
    expression's by the position alone. -/
inductive Place where
  | branch | loop | forLoop | doWhile | statement
  deriving DecidableEq

structure Text where
  body : String
  place : Place

def isConditionRefusal (t : Text) : Bool := t.place != .statement

/-- The generic text and the named text of one position share the place. -/
def generic (p : Place) : Text := { body := "unmodeled call, aggregate or indirect access", place := p }
def named (callee : String) (p : Place) : Text :=
  { body := "call to uncontracted function " ++ callee, place := p }

theorem the_named_text_keeps_the_condition_class (callee : String) (p : Place) :
    isConditionRefusal (named callee p) = isConditionRefusal (generic p) := rfl

/-- The attribution applies only to the classifier's exact uncontracted text
    for the refused call's own callee. -/
def attributes (exactText sameCallee descriptor switchPosition : Bool) : Bool :=
  exactText && sameCallee && !descriptor && !switchPosition

theorem attribution_needs_the_exact_text (c d w : Bool) :
    attributes false c d w = false := by simp [attributes]

theorem a_descriptor_read_keeps_the_text (e c w : Bool) :
    attributes e c true w = false := by simp [attributes]

end QCore.ConditionLeafReads
