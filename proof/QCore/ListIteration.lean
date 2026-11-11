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
# Intrusive circular-list iteration (card W45-FB-BACKLOG-CUSTODY)

The abstract rule implemented by `crates/qcore-c-gateway/src/legacy_list_iteration.rs`
(gateway constraint 5l). The Monkey and CFL list macros expand to

```text
for (C = (H)->next; C != (H); C = C->next)                       plain
for (C = (H)->next, N = C->next; C != (H); C = N, N = C->next)   safe
```

A plain iteration reads `C->next` **after** its body; a safe iteration read
`N = C->next` **before** it. Unlinking a node clears its links
(`mk_list_del`), so a read of a cleared link has no successor to give.

The model:

* `Ring` is the list of nodes linked after the head, in order.
* `next r c` is what reading `c->next` yields: `some (some d)` for a linked
  successor, `some none` for the head (the end), and `none` when `c` is not
  linked — the read of a cleared link, which the model treats as stuck.
* `Body` is what the loop body does to the list: nothing, unlink the current
  node, or unlink the saved successor.
* `runPlain`/`runSafe` run the iteration with fuel and return the visited
  nodes, or `none` when a step reads a cleared link.
* `admitted` is rule 4 of the gateway: a plain iteration refuses a body that
  unlinks the current node; a safe one admits that and refuses unlinking the
  saved successor. Unlinking a node the plain form does not hold (its
  successor) is admitted: the step then reads the updated link.

## What is proved

* **Safe iteration preserves well-formedness under unlinking the current
  node, and visits every node exactly once** (`safe_unlinking_visits_each_once`,
  `unlinking_preserves_distinctness`, `the_saved_successor_stays_linked`).
* **Plain iteration does not survive unlinking the current node**
  (`plain_unlinking_reads_a_cleared_link`), and a safe iteration does not
  survive unlinking its saved successor (`unlinking_the_successor_reads_a_cleared_link`).
* **Everything rule 4 admits runs to the end of every well-formed ring without
  reading a cleared link** (`an_admitted_body_never_reads_a_cleared_link`), and
  a keeping body, or a safe body unlinking its current node, visits the whole
  ring in order (`plain_keep_visits`, `safe_keep_visits`,
  `safe_unlinking_visits_each_once`); everything it refuses has a ring on
  which it is stuck (`a_refused_body_has_a_stuck_ring`).
* **An element binding moves no custody** (`element_binding_moves_no_custody`):
  rule 3 unbinds a name and changes no resource status.

## What this is not

A model of the **decision**, as the gateway's other Lean modules are. That
the lexical reader recognises exactly the preprocessed macro spellings, and
the premises the gateway states but does not prove (P-ILI-BODY checked
lexically, P-ILI-UNTRACKED, P-ILI-CALLEE-FRAME), are review premises pinned by
`crates/qcore-c-gateway/tests/intrusive_list_iteration.rs`.
-/

namespace QCore.ListIteration

/-- The nodes linked after the head, in order. -/
abbrev Ring := List Nat

/-- Reading `c->next`: `none` when `c` is not linked (a cleared link). -/
def next : Ring → Nat → Option (Option Nat)
  | [], _ => none
  | x :: rest, c => if x = c then some rest.head? else next rest c

/-- Unlinking one node. -/
def remove : Ring → Nat → Ring
  | [], _ => []
  | x :: xs, c => if x = c then xs else x :: remove xs c

/-- A well-formed ring: every node is linked at most once. -/
def distinct : Ring → Prop
  | [] => True
  | x :: xs => x ∉ xs ∧ distinct xs

/-- What a body does to the ring. -/
inductive Body where
  | keep
  | unlinkCurrent
  | unlinkSuccessor
  deriving DecidableEq, Repr

/-- The two iteration kinds. -/
inductive Kind where
  | plain
  | safe
  deriving DecidableEq, Repr

/-- The body's effect, given the current node and the successor it could see. -/
def apply : Body → Ring → Nat → Option Nat → Ring
  | .keep, r, _, _ => r
  | .unlinkCurrent, r, c, _ => remove r c
  | .unlinkSuccessor, r, _, some n => remove r n
  | .unlinkSuccessor, r, _, none => r

/-- A plain iteration: the body runs, then `C = C->next`. -/
def runPlain (b : Body) : Nat → Ring → Option Nat → Option (List Nat)
  | 0, _, _ => none
  | _ + 1, _, none => some []
  | fuel + 1, r, some c =>
    match next r c with
    | none => none
    | some n =>
      match next (apply b r c n) c with
      | none => none
      | some n' => (runPlain b fuel (apply b r c n) n').map (c :: ·)

/-- A safe iteration: `N = C->next` before the body, then `C = N`. -/
def runSafe (b : Body) : Nat → Ring → Option Nat → Option (List Nat)
  | 0, _, _ => none
  | _ + 1, _, none => some []
  | fuel + 1, r, some c =>
    match next r c with
    | none => none
    | some n => (runSafe b fuel (apply b r c n) n).map (c :: ·)

/-- Run either kind. -/
def run : Kind → Body → Nat → Ring → Option Nat → Option (List Nat)
  | .plain => runPlain
  | .safe => runSafe

/-- Rule 4: what each kind admits. -/
def admitted : Kind → Body → Bool
  | .plain, .unlinkCurrent => false
  | .safe, .unlinkSuccessor => false
  | _, _ => true

-- ## The list facts

theorem next_absent {r : Ring} {c : Nat} (h : c ∉ r) : next r c = none := by
  induction r with
  | nil => rfl
  | cons x xs ih =>
    have hx : x ≠ c := fun e => h (e ▸ List.mem_cons_self)
    have hxs : c ∉ xs := fun m => h (List.mem_cons_of_mem x m)
    simp [next, hx, ih hxs]

theorem next_after_prefix (pre rest : Ring) (c : Nat) (h : c ∉ pre) :
    next (pre ++ c :: rest) c = some rest.head? := by
  induction pre with
  | nil => simp [next]
  | cons x xs ih =>
    have hx : x ≠ c := fun e => h (e ▸ List.mem_cons_self)
    have hxs : c ∉ xs := fun m => h (List.mem_cons_of_mem x m)
    simp [next, hx, ih hxs]

theorem mem_of_mem_remove {r : Ring} {c y : Nat} (h : y ∈ remove r c) : y ∈ r := by
  induction r with
  | nil => simp [remove] at h
  | cons x xs ih =>
    by_cases hx : x = c
    · simp [remove, hx] at h
      exact List.mem_cons_of_mem x h
    · simp [remove, hx] at h
      rcases h with h | h
      · exact h ▸ List.mem_cons_self
      · exact List.mem_cons_of_mem x (ih h)

/-- Unlinking a node keeps the ring well formed. -/
theorem unlinking_preserves_distinctness {r : Ring} (c : Nat) (h : distinct r) :
    distinct (remove r c) := by
  induction r with
  | nil => simp [remove, distinct]
  | cons x xs ih =>
    by_cases hx : x = c
    · simp [remove, hx]
      exact h.2
    · simp only [remove, hx]
      exact ⟨fun m => h.1 (mem_of_mem_remove m), ih h.2⟩

/-- Unlinking the current node of a well-formed ring leaves its successor
linked. -/
theorem the_saved_successor_stays_linked (c d : Nat) (rest : Ring) :
    d ∈ remove (c :: d :: rest) c := by
  simp [remove]

-- ## Safe iteration under unlinking

/-- A safe iteration whose body unlinks every current node visits each node
exactly once, in order. -/
theorem safe_unlinking_visits_each_once :
    ∀ (r : Ring), distinct r →
      runSafe .unlinkCurrent (r.length + 1) r r.head? = some r := by
  intro r
  induction r with
  | nil => intro _; rfl
  | cons c rest ih =>
    intro h
    have hstep : next (c :: rest) c = some rest.head? := by simp [next]
    have hrem : remove (c :: rest) c = rest := by simp [remove]
    simp [runSafe, hstep, apply, hrem, ih h.2]

-- ## Keeping bodies visit the ring

theorem plain_keep_visits (suf : Ring) :
    ∀ (pre : Ring), (∀ x ∈ suf, x ∉ pre) → distinct suf →
      runPlain .keep (suf.length + 1) (pre ++ suf) suf.head? = some suf := by
  induction suf with
  | nil => intro _ _ _; rfl
  | cons c rest ih =>
    intro pre hdis hd
    have hc : c ∉ pre := hdis c List.mem_cons_self
    have hstep : next (pre ++ c :: rest) c = some rest.head? := next_after_prefix pre rest c hc
    have hre : pre ++ c :: rest = (pre ++ [c]) ++ rest := by simp
    have hdis' : ∀ x ∈ rest, x ∉ pre ++ [c] := by
      intro x hx hm
      rcases List.mem_append.mp hm with hp | hp
      · exact hdis x (List.mem_cons_of_mem c hx) hp
      · have : x = c := by simpa using hp
        exact hd.1 (this ▸ hx)
    have hrec := ih (pre ++ [c]) hdis' hd.2
    rw [← hre] at hrec
    simp [runPlain, apply, hstep, hrec]

theorem safe_keep_visits (suf : Ring) :
    ∀ (pre : Ring), (∀ x ∈ suf, x ∉ pre) → distinct suf →
      runSafe .keep (suf.length + 1) (pre ++ suf) suf.head? = some suf := by
  induction suf with
  | nil => intro _ _ _; rfl
  | cons c rest ih =>
    intro pre hdis hd
    have hc : c ∉ pre := hdis c List.mem_cons_self
    have hstep : next (pre ++ c :: rest) c = some rest.head? := next_after_prefix pre rest c hc
    have hre : pre ++ c :: rest = (pre ++ [c]) ++ rest := by simp
    have hdis' : ∀ x ∈ rest, x ∉ pre ++ [c] := by
      intro x hx hm
      rcases List.mem_append.mp hm with hp | hp
      · exact hdis x (List.mem_cons_of_mem c hx) hp
      · have : x = c := by simpa using hp
        exact hd.1 (this ▸ hx)
    have hrec := ih (pre ++ [c]) hdis' hd.2
    rw [← hre] at hrec
    simp [runSafe, apply, hstep, hrec]

theorem remove_after (pre rest : Ring) (c d : Nat) (hp : d ∉ pre) (hcd : c ≠ d) :
    remove (pre ++ c :: d :: rest) d = pre ++ c :: rest := by
  induction pre with
  | nil => simp [remove, hcd]
  | cons x xs ih =>
    have hx : x ≠ d := fun e => hp (e ▸ List.mem_cons_self)
    have hxs : d ∉ xs := fun m => hp (List.mem_cons_of_mem x m)
    simp [remove, hx, ih hxs]

/-- A plain iteration whose body unlinks the node after the current one never
reads a cleared link: the step reads the link the unlink updated. -/
theorem plain_unlinking_the_next_node_runs :
    ∀ (fuel : Nat) (suf pre : Ring), suf.length ≤ fuel → (∀ x ∈ suf, x ∉ pre) →
      distinct suf →
      ∃ v, runPlain .unlinkSuccessor (fuel + 1) (pre ++ suf) suf.head? = some v := by
  intro fuel
  induction fuel with
  | zero =>
    intro suf pre hlen _ _
    cases suf with
    | nil => exact ⟨[], rfl⟩
    | cons c rest => simp at hlen
  | succ m ih =>
    intro suf pre hlen hdis hd
    match suf, hlen, hdis, hd with
    | [], _, _, _ => exact ⟨[], rfl⟩
    | [c], _, hdis, _ =>
      have hc : c ∉ pre := hdis c List.mem_cons_self
      have hstep : next (pre ++ [c]) c = some none := by
        simpa using next_after_prefix pre [] c hc
      cases m with
      | zero => exact ⟨[c], by simp [runPlain, apply, hstep]⟩
      | succ k => exact ⟨[c], by simp [runPlain, apply, hstep]⟩
    | c :: d :: rest, hlen, hdis, hd =>
      have hc : c ∉ pre := hdis c List.mem_cons_self
      have hdp : d ∉ pre := hdis d (List.mem_cons_of_mem c List.mem_cons_self)
      have hcd : c ≠ d := fun e => hd.1 (e ▸ List.mem_cons_self)
      have hstep : next (pre ++ c :: d :: rest) c = some (some d) := by
        simpa using next_after_prefix pre (d :: rest) c hc
      have hrem := remove_after pre rest c d hdp hcd
      have hstep' : next (pre ++ c :: rest) c = some rest.head? :=
        next_after_prefix pre rest c hc
      have hre : pre ++ c :: rest = (pre ++ [c]) ++ rest := by simp
      have hdis' : ∀ x ∈ rest, x ∉ pre ++ [c] := by
        intro x hx hm
        rcases List.mem_append.mp hm with hp | hp
        · exact hdis x (List.mem_cons_of_mem c (List.mem_cons_of_mem d hx)) hp
        · have : x = c := by simpa using hp
          exact hd.1 (this ▸ List.mem_cons_of_mem d hx)
      have hlen' : rest.length ≤ m := by simp at hlen; omega
      obtain ⟨v, hv⟩ := ih rest (pre ++ [c]) hlen' hdis' hd.2.2
      rw [← hre] at hv
      exact ⟨c :: v, by simp [runPlain, apply, hstep, hrem, hstep', hv]⟩

/-- Everything rule 4 admits runs to the end of every well-formed ring without
reading a cleared link. -/
theorem an_admitted_body_never_reads_a_cleared_link (k : Kind) (b : Body) (r : Ring)
    (hadm : admitted k b = true) (hd : distinct r) :
    ∃ v, run k b (r.length + 1) r r.head? = some v := by
  have hnil : (∀ x ∈ r, x ∉ ([] : Ring)) := by intro x _ h; cases h
  cases k <;> cases b <;> simp [admitted] at hadm
  · exact ⟨r, by simpa [run] using plain_keep_visits r [] hnil hd⟩
  · simpa [run] using plain_unlinking_the_next_node_runs r.length r [] (Nat.le_refl _) hnil hd
  · exact ⟨r, by simpa [run] using safe_keep_visits r [] hnil hd⟩
  · exact ⟨r, by simpa [run] using safe_unlinking_visits_each_once r hd⟩

-- ## What rule 4 refuses

/-- A plain iteration that unlinks its current node reads a cleared link. -/
theorem plain_unlinking_reads_a_cleared_link (c : Nat) (rest : Ring) (fuel : Nat)
    (hd : distinct (c :: rest)) :
    runPlain .unlinkCurrent (fuel + 1) (c :: rest) (some c) = none := by
  have hstep : next (c :: rest) c = some rest.head? := by simp [next]
  have hrem : remove (c :: rest) c = rest := by simp [remove]
  have hgone : next rest c = none := next_absent hd.1
  simp [runPlain, hstep, apply, hrem, hgone]

/-- A safe iteration that unlinks its saved successor reads a cleared link at
the next step. -/
theorem unlinking_the_successor_reads_a_cleared_link (c d : Nat) (rest : Ring)
    (fuel : Nat) (hd : distinct (c :: d :: rest)) :
    runSafe .unlinkSuccessor (fuel + 2) (c :: d :: rest) (some c) = none := by
  have hcd : c ≠ d := fun e => hd.1 (e ▸ List.mem_cons_self)
  have hstep : next (c :: d :: rest) c = some (some d) := by simp [next]
  have hrem : remove (c :: d :: rest) d = c :: rest := by simp [remove, hcd]
  have hdr : d ∉ rest := hd.2.1
  have hgone : next (c :: rest) d = none := by
    apply next_absent
    intro m
    rcases List.mem_cons.mp m with e | e
    · exact hcd e.symm
    · exact hdr e
  simp [runSafe, hstep, apply, hrem, hgone]

/-- Every body rule 4 refuses is stuck on some well-formed ring. -/
theorem a_refused_body_has_a_stuck_ring (k : Kind) (b : Body)
    (hadm : admitted k b = false) :
    ∃ r : Ring, distinct r ∧ run k b (r.length + 1) r r.head? = none := by
  cases k <;> cases b <;> simp [admitted] at hadm
  · -- plain, unlinkCurrent
    refine ⟨[0], by simp [distinct], ?_⟩
    simpa [run] using plain_unlinking_reads_a_cleared_link 0 [] 1 (by simp [distinct])
  · -- safe, unlinkSuccessor
    refine ⟨[0, 1], by simp [distinct], ?_⟩
    simpa [run] using unlinking_the_successor_reads_a_cleared_link 0 1 [] 1
      (by simp [distinct])

/-- The two refusals rule 4 makes, read off `admitted`. -/
theorem plain_refuses_unlinking_the_current_node :
    admitted .plain .unlinkCurrent = false ∧ admitted .plain .unlinkSuccessor = true ∧
      admitted .plain .keep = true := by
  decide

theorem safe_admits_unlinking_the_current_node :
    admitted .safe .unlinkCurrent = true ∧ admitted .safe .unlinkSuccessor = false := by
  decide

-- ## Rule 3: an element binding moves no custody

/-- The part of the abstract state rule 3 touches: which site a name is bound
to, and each site's status. -/
structure State where
  binding : Nat → Option Nat
  status : Nat → Nat

/-- Rule 3 lowers an element read to an access whose destination is left
unbound. -/
def bindElement (s : State) (x : Nat) : State :=
  { s with binding := fun y => if y = x then none else s.binding y }

theorem element_binding_moves_no_custody (s : State) (x : Nat) :
    (bindElement s x).status = s.status ∧ (bindElement s x).binding x = none ∧
      ∀ y, y ≠ x → (bindElement s x).binding y = s.binding y := by
  refine ⟨rfl, by simp [bindElement], ?_⟩
  intro y hy
  simp [bindElement, hy]

end QCore.ListIteration
