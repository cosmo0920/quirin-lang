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

/-!
# An embedded list handed to a list helper is a borrowed container operation (card CG10-EMBEDDED-LIST-BORROWED-CONTAINER, gateway rule 69)

The abstract rule implemented by
`crates/qcore-c-gateway/src/legacy_embedded_container.rs`
(`embedded_container`, `container_refusal`,
`Parser::embedded_container_attribution`,
`Parser::embedded_container_return_attribution`) over the shared reading
`Parser::reclassified_with_borrowed_actuals`.

USER DECISION (2026-11-20): "Read embedded lists as borrowed container
operations". A list helper handed `& R -> M …` (`R` a formal) is a `Borrowed`
operation on the caller's container: the body takes no custody of the record
or the list, and the list's elements stay the container's. The decision does
not decide soundness, and this module records why the rule admits nothing:

* **The container model** (`ContainerState`, `apply`). A helper either reads
  the container (`borrowedRead`: every position `Borrowed`, no container
  clause, the identity), invokes the container's registration over its
  elements (`destroyElements`: `rd_list_destroy`, `rd_list_clear`), or adds
  an element (`addElement`: `rd_list_add`). A registration the body does not
  hold is `unknownReg`. `a_borrowed_only_helper_changes_no_custody`;
  `a_releasing_helper_is_not_a_noop` and
  `the_noop_reading_misses_a_use_after_release` are the **counterexample**:
  reading a releasing helper as a borrow leaves a released element live, so a
  later use the real call makes a use after release is judged sound;
  `an_unknown_registration_decides_no_destroy` and
  `an_element_added_to_an_undecided_container_has_no_one_outcome` show why an
  undecided registration (a holder reached through a formal, rule 13w) decides
  neither effect.
* **The rule** (`reattribute`). It is asked only for exactly the gate's
  refusal, only when the first call has an embedded container actual, and
  only when the reading is a container custody refusal of that call about
  that actual; it changes at most the refusal's description, never its
  writes, never a non-refusal, never a verdict, hence never a function's
  status (`the_status_of_a_function_is_unchanged`), and never admits
  (`the_rule_never_admits`).
* **The actual** (`embeddedContainer`) over a token alphabet, bound to the
  Rust's `CONTAINER_VECTORS` (`container_vectors_hold`).

## What this is not

A model of the classifier or of route G. Whether the reading's answer is a
container custody refusal is the Rust's (`container_refusal` and its unit
test); that the reading leaves no state behind (its counters restored, its
instruction discarded) is reviewed and bound by tests; faithful token
extraction is trust boundary 28.
-/

namespace QCore.EmbeddedListContainer

/-! ## The container model -/

/-- What the container's registration does to an element it is invoked on. -/
inductive Registration where
  /-- Every candidate releases its `element` (rule 13x). -/
  | releases
  /-- Every candidate is the identity on custody. -/
  | identity
  /-- No registration this body holds (a holder through a formal, rule 13w). -/
  | unknownReg
  deriving DecidableEq, Repr

/-- The custody facts one body's analysis keeps: the objects it may still use
(`live`), the elements the container holds, and the registration. -/
structure ContainerState where
  live : List Nat
  elements : List Nat
  registration : Registration
  deriving DecidableEq, Repr

/-- One list helper call on the container. -/
inductive Helper where
  /-- Every position `Borrowed`, no container clause (`cfl_list_size`). -/
  | borrowedRead
  /-- The registration invoked over every element, which are then dropped. -/
  | destroyElements
  /-- `e` becomes an element. -/
  | addElement (e : Nat)
  deriving DecidableEq, Repr

/-- The call's effect, `none` where the state does not decide it. -/
def apply : Helper → ContainerState → Option ContainerState
  | .borrowedRead, s => some s
  | .destroyElements, s =>
      match s.registration with
      | .releases => some { s with live := s.live.filter (· ∉ s.elements), elements := [] }
      | .identity => some { s with elements := [] }
      | .unknownReg => none
  | .addElement e, s =>
      match s.registration with
      | .releases => some { s with live := s.live.filter (· ≠ e), elements := e :: s.elements }
      | .identity => some { s with elements := e :: s.elements }
      | .unknownReg => none

/-- The reading the decision does **not** license: every helper as a borrow. -/
def asBorrow (_ : Helper) (s : ContainerState) : Option ContainerState := some s

/-- A use of `x` after the call is sound exactly when `x` is still live. -/
def useSound (x : Nat) (s : ContainerState) : Bool := s.live.contains x

theorem a_borrowed_only_helper_changes_no_custody (s : ContainerState) :
    apply .borrowedRead s = some s := rfl

/-- The witness: one live element `7` in a container whose registration
releases its elements. -/
def witness : ContainerState := ⟨[7], [7], .releases⟩

theorem a_releasing_helper_is_not_a_noop :
    apply .destroyElements witness ≠ asBorrow .destroyElements witness := by decide

theorem the_noop_reading_misses_a_use_after_release :
    (asBorrow .destroyElements witness).map (useSound 7) = some true ∧
      (apply .destroyElements witness).map (useSound 7) = some false := by decide

theorem an_unknown_registration_decides_no_destroy (s : ContainerState)
    (h : s.registration = .unknownReg) : apply .destroyElements s = none := by
  simp [apply, h]

theorem an_unknown_registration_decides_no_add (s : ContainerState) (e : Nat)
    (h : s.registration = .unknownReg) : apply (.addElement e) s = none := by
  simp [apply, h]

/-- Resolving an undecided registration either way gives two different states:
the element's custody passes (`releases`) or is only retained (`identity`). -/
theorem an_element_added_to_an_undecided_container_has_no_one_outcome :
    apply (.addElement 3) ⟨[3], [], .releases⟩ ≠
      (apply (.addElement 3) ⟨[3], [], .identity⟩).map
        (fun s => { s with registration := .releases }) := by decide

/-! ## The actual -/

/-- The token alphabet the recogniser distinguishes. -/
inductive Tok where
  | amp
  | arrow
  | dot
  | name (n : String)
  | other (t : String)
  deriving DecidableEq, Repr

/-- `. N` / `-> N` hops to the end. -/
def hops : List Tok → Bool
  | [] => true
  | .dot :: .name _ :: rest => hops rest
  | .arrow :: .name _ :: rest => hops rest
  | _ => false

/-- `& R hop+` with `R` a formal. -/
def embeddedContainer (formal : String → Bool) : List Tok → Bool
  | .amp :: .name r :: rest => formal r && !rest.isEmpty && hops rest
  | _ => false

/-- The Rust's formals in `CONTAINER_VECTORS`. -/
def vectorFormal (n : String) : Bool := n = "c" || n = "k" || n = "s"

/-- `CONTAINER_VECTORS` (`legacy_embedded_container.rs`), `->` one token. -/
def containerVectors : List (List Tok × Bool) :=
  [ ([.amp, .name "c", .arrow, .name "list"], true),
    ([.amp, .name "c", .arrow, .name "in", .dot, .name "on_new"], true),
    ([.amp, .name "k", .arrow, .name "conf", .dot, .name "in", .dot, .name "on_new"], true),
    ([.amp, .name "k", .arrow, .name "confp", .arrow, .name "waiters"], true),
    ([.amp, .name "s", .dot, .name "list"], true),
    ([.amp, .name "c"], false),
    ([.name "c", .arrow, .name "list"], false),
    ([.amp, .name "c", .arrow, .name "list", .other "[", .other "0", .other "]"], false),
    ([.amp, .name "l", .arrow, .name "list"], false) ]

theorem container_vectors_hold :
    containerVectors.all (fun (v : List Tok × Bool) => embeddedContainer vectorFormal v.1 == v.2) =
      true := by decide

theorem a_bare_frame_holder_is_not_embedded (formal : String → Bool) (r : String) :
    embeddedContainer formal [.amp, .name r] = false := by
  simp [embeddedContainer]

theorem a_non_formal_root_is_not_embedded (formal : String → Bool) (r : String) (rest : List Tok)
    (h : formal r = false) : embeddedContainer formal (.amp :: .name r :: rest) = false := by
  simp [embeddedContainer, h]

/-! ## The rule -/

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

/-- The container custody reasons the rule names. -/
inductive Reason where
  /-- `callback-registry-undecided` (rule 13w). -/
  | registry
  /-- `callback-element-custody-undecided` (rule 13x). -/
  | element
  /-- `retention-undecided` (rule 12t). -/
  | retention
  deriving DecidableEq, Repr

/-- A refusal's description, as far as the rule distinguishes it. -/
inductive Description where
  /-- `indirect memory access is outside the resource projection`. -/
  | gate
  /-- A container custody refusal of `callee` about the actual `actual`. -/
  | container (reason : Reason) (callee : String) (actual : String)
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

/-- The verdict one node contributes: a held call is `Pending`, every other
refusal `Unknown`; an admitted node contributes nothing. -/
def verdict : Node → Verdict
  | .refusal (.held _) _ => .pending
  | .refusal _ _ => .unknown
  | .admitted _ => .safe

/-- A function's status: the join of its nodes' verdicts. -/
def status (nodes : List Node) : Verdict := nodes.foldl (fun v n => v.join (verdict n)) .safe

/-- What the rule knows about the statement's first call. -/
structure Call where
  callee : String
  /-- The spelled embedded container actuals of the call. -/
  containers : List String
  /-- The statement has braces or reads a tracked descriptor. -/
  excluded : Bool
  deriving DecidableEq, Repr

/-- The rule: `reading` is the classification with every memory-reading
actual read (`none` when an actual is recognised by neither reader). -/
def reattribute (call : Call) (reading : Option Node) : Node → Node
  | .refusal .gate writes =>
      if call.excluded || call.containers.isEmpty then
        .refusal .gate writes
      else
        match reading with
        | some (.refusal (.container r f a) _) =>
            if f = call.callee ∧ a ∈ call.containers then .refusal (.container r f a) writes
            else .refusal .gate writes
        | _ => .refusal .gate writes
  | n => n

theorem only_the_gate_refusal_is_read (call : Call) (reading : Option Node) (n : Node)
    (h : ∀ w, n ≠ .refusal .gate w) : reattribute call reading n = n := by
  cases n with
  | refusal d w =>
      cases d with
      | gate => exact absurd rfl (h w)
      | container _ _ _ => rfl
      | held _ => rfl
      | other _ => rfl
  | admitted _ => rfl

theorem the_result_is_the_gate_refusal_or_a_container_refusal (call : Call)
    (reading : Option Node) (w : List String) :
    reattribute call reading (.refusal .gate w) = .refusal .gate w ∨
      ∃ r a, a ∈ call.containers ∧
        reattribute call reading (.refusal .gate w) = .refusal (.container r call.callee a) w := by
  simp only [reattribute]
  split
  · exact Or.inl rfl
  · split
    · rename_i r f a _
      split
      · rename_i h
        obtain ⟨hf, ha⟩ := h
        subst hf
        exact Or.inr ⟨r, a, ha, rfl⟩
      · exact Or.inl rfl
    · exact Or.inl rfl

theorem the_writes_are_kept (call : Call) (reading : Option Node) (w : List String) :
    ∃ d, reattribute call reading (.refusal .gate w) = .refusal d w := by
  rcases the_result_is_the_gate_refusal_or_a_container_refusal call reading w with h | ⟨r, a, _, h⟩
  · exact ⟨_, h⟩
  · exact ⟨_, h⟩

theorem the_rule_never_admits (call : Call) (reading : Option Node) (n : Node) (name : String)
    (h : reattribute call reading n = .admitted name) : n = .admitted name := by
  cases n with
  | admitted m => exact h
  | refusal d w =>
      cases d with
      | gate =>
          rcases the_result_is_the_gate_refusal_or_a_container_refusal call reading w with
            h' | ⟨r, a, _, h'⟩
          · rw [h'] at h; cases h
          · rw [h'] at h; cases h
      | container _ _ _ => cases h
      | held _ => cases h
      | other _ => cases h

theorem the_verdict_of_every_node_is_unchanged (call : Call) (reading : Option Node) (n : Node) :
    verdict (reattribute call reading n) = verdict n := by
  cases n with
  | admitted _ => rfl
  | refusal d w =>
      cases d with
      | gate =>
          rcases the_result_is_the_gate_refusal_or_a_container_refusal call reading w with
            h | ⟨r, a, _, h⟩
          · rw [h]
          · rw [h]; rfl
      | container _ _ _ => rfl
      | held _ => rfl
      | other _ => rfl

theorem the_status_of_a_function_is_unchanged (call : Node → Call) (reading : Node → Option Node)
    (nodes : List Node) :
    status (nodes.map fun n => reattribute (call n) (reading n) n) = status nodes := by
  unfold status
  suffices ∀ v : Verdict, List.foldl (fun (v : Verdict) n => v.join (verdict n)) v
      (nodes.map fun n => reattribute (call n) (reading n) n) =
      List.foldl (fun (v : Verdict) n => v.join (verdict n)) v nodes from this _
  induction nodes with
  | nil => intro v; rfl
  | cons n rest ih =>
      intro v
      simp only [List.map_cons, List.foldl_cons, the_verdict_of_every_node_is_unchanged]
      exact ih _

theorem no_embedded_actual_keeps_the_gate_refusal (call : Call) (reading : Option Node)
    (w : List String) (h : call.containers = []) :
    reattribute call reading (.refusal .gate w) = .refusal .gate w := by
  simp [reattribute, h]

theorem an_excluded_statement_keeps_the_gate_refusal (call : Call) (reading : Option Node)
    (w : List String) (h : call.excluded = true) :
    reattribute call reading (.refusal .gate w) = .refusal .gate w := by
  simp [reattribute, h]

theorem an_admitting_reading_keeps_the_gate_refusal (call : Call) (name : String)
    (w : List String) :
    reattribute call (some (.admitted name)) (.refusal .gate w) = .refusal .gate w := by
  simp only [reattribute]
  split <;> rfl

theorem a_refusal_about_another_place_keeps_the_gate_refusal (call : Call) (r : Reason)
    (a : String) (w w' : List String) (h : a ∉ call.containers) :
    reattribute call (some (.refusal (.container r call.callee a) w')) (.refusal .gate w) =
      .refusal .gate w := by
  simp only [reattribute]
  split
  · rfl
  · simp [h]

/-- `rd_list_destroy(&conf->interceptors.on_new)`: the leaf names the
registry and stays `Unknown`. -/
example :
    reattribute ⟨"rd_list_destroy", ["& conf - > interceptors . on_new"], false⟩
        (some (.refusal (.container .registry "rd_list_destroy"
          "& conf - > interceptors . on_new") ["conf"]))
        (.refusal .gate ["conf"]) =
      .refusal (.container .registry "rd_list_destroy" "& conf - > interceptors . on_new")
        ["conf"] := by decide

end QCore.EmbeddedListContainer
