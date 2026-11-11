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

namespace QCore.ConstantDoWhile

/-! Card `C-GATEWAY-CONSTANT-DO-WHILE`: the constant `do … while (0)` exit.

`do S while (0)` executes `S` exactly once (C11 §6.8.5.2): the controlling
expression is evaluated after the body, compares equal to `0`, and the loop
terminates. It *is* sequential composition, and the CFG without a back edge is
its exact semantics rather than an approximation of it.

The gateway's CFG builder previously gave every `do`-`while` header both
`body_entry` and `continuation` as successors, whatever its condition. On the
pinned Fluent Bit corpus that back edge is not rare: the `do { … } while (0)`
macro idiom wraps cleanup and error paths throughout librdkafka and monkey, so
a release inside one looked repeatable and the corpus carried a provisional
`double_release` row for `rd_kafka_jwt_b64_decode_payload`, whose `RD_IF_FREE`
expansion is exactly this shape.

Removing an edge is the **admitting** direction, which invariant 1 governs, so
this module states both halves. `removing_the_back_edge_only_removes_paths` is
the conservativity claim — the new CFG admits a subset of the old one's paths,
so no diagnostic can be *created* by this change. The non-vacuity claims
(`the_back_edge_admits_a_second_execution`,
`the_removed_paths_are_real`) say the rule is not decoration: the old model
really does admit executing the body twice, and that second execution carries
the second release the corpus reported.

Constraint 5a governs the key. `Condition` distinguishes only the single
literal token `0` from everything else, mirroring `condition_is_literal_zero`
in `legacy_c.rs`. Nothing here evaluates an expression, folds a constant or
reads a binding's value, and `a_non_literal_condition_is_unchanged` is the
statement that every other condition keeps the CFG it had.
-/

/-- The controlling expression of a `do … while (E)`, as the gateway's CFG
builder classifies it. `literalZero` is the single literal token `0`; `other`
is every other token sequence, including one that happens to evaluate to zero.
This two-case split is the whole of the rule's key (constraint 5a). -/
inductive Condition where
  | literalZero
  | other
  deriving DecidableEq, Repr

/-- A trace: the sequence of resource-relevant atoms an execution performs. -/
abbrev Trace := List Nat

/-- `repeatTrace body n` is `n` back-to-back executions of the body. -/
def repeatTrace (body : Trace) : Nat → Trace
  | 0 => []
  | n + 1 => body ++ repeatTrace body n

@[simp] theorem repeatTrace_one (body : Trace) : repeatTrace body 1 = body := by
  simp [repeatTrace]

@[simp] theorem repeatTrace_two (body : Trace) :
    repeatTrace body 2 = body ++ body := by
  simp [repeatTrace]

/-- The CFG the gateway built before this card: the header's successors are
`[body_entry, continuation]`, so the fragment admits any positive number of
executions of the body. -/
def BackEdgeAdmits (body t : Trace) : Prop :=
  ∃ n, 0 < n ∧ t = repeatTrace body n

/-- The CFG this card builds. A literal-zero condition has `continuation` as
its only successor, so the fragment is sequential composition: exactly one
execution of the body. Every other condition keeps the back edge. -/
def Admits (cond : Condition) (body t : Trace) : Prop :=
  match cond with
  | .literalZero => t = body
  | .other => BackEdgeAdmits body t

/-! ### `do S while (0)` ≡ `S` -/

/-- The rule itself: the paths of `do S while (0)` are exactly the paths of
`S`. This is the C11 §6.8.5.2 statement, and it is an equality of admitted
sets rather than a bound on one. -/
theorem constant_do_while_is_its_body (body t : Trace) :
    Admits Condition.literalZero body t ↔ t = body := Iff.rfl

/-- Stated against the old model instead: the constant exit admits exactly the
one-iteration path the back-edge model already had. -/
theorem constant_do_while_is_one_iteration (body t : Trace) :
    Admits Condition.literalZero body t ↔ t = repeatTrace body 1 := by
  simp [Admits]

/-! ### Direction: this change can only withdraw, never accuse -/

/-- Conservativity, the half invariant 1 constrains. Every path the new CFG
admits was already admitted by the old one, for **every** condition. A
diagnostic derived from the new CFG is therefore derived from a path the old
CFG also had: removing the back edge can withdraw a refusal but can never
manufacture one. -/
theorem removing_the_back_edge_only_removes_paths
    (cond : Condition) (body t : Trace) (h : Admits cond body t) :
    BackEdgeAdmits body t := by
  cases cond with
  | literalZero => exact ⟨1, Nat.one_pos, by rw [repeatTrace_one]; exact h⟩
  | other => exact h

/-- Every other condition keeps precisely the CFG it had. This is why the rule
moves nothing outside the literal-`0` spelling. -/
theorem a_non_literal_condition_is_unchanged (body t : Trace) :
    Admits Condition.other body t ↔ BackEdgeAdmits body t := Iff.rfl

/-! ### Non-vacuity: the removed paths are real, and they carried the row -/

/-- The old model really does admit executing the body a second time. -/
theorem the_back_edge_admits_a_second_execution (body : Trace) :
    BackEdgeAdmits body (body ++ body) :=
  ⟨2, by omega, by simp⟩

/-- And the constant exit really does refuse it. The `body ≠ []` premise is
necessary rather than decorative: an empty body's second execution is
observationally its first. -/
theorem the_constant_exit_refuses_the_second_execution
    {body : Trace} (h : body ≠ []) :
    ¬ Admits Condition.literalZero body (body ++ body) := by
  intro hEq
  have hlen : (body ++ body).length = body.length :=
    congrArg List.length hEq
  rw [List.length_append] at hlen
  exact h (List.eq_nil_of_length_eq_zero (by omega))

/-- So the two models genuinely part, and they part in the withdrawing
direction. -/
theorem the_removed_paths_are_real {body : Trace} (h : body ≠ []) :
    ∃ t, BackEdgeAdmits body t ∧ ¬ Admits Condition.literalZero body t :=
  ⟨body ++ body, the_back_edge_admits_a_second_execution body,
    the_constant_exit_refuses_the_second_execution h⟩

/-! ### The corpus row this rule withdraws

`RD_IF_FREE(converted_src, rd_free)` expands to
`do { if ((converted_src)) rd_free(converted_src); } while (0)`, whose
resource-relevant content is one release. Atom `0` is that release. -/

/-- How many times a trace performs atom `r`. -/
def releases (r : Nat) (t : Trace) : Nat := t.count r

/-- The body of the corpus's `RD_IF_FREE` expansion: a single release. -/
def releaseBody : Trace := [0]

/-- Under the back edge the release is performed twice — which is the
`repeated release of acquisition` the pinned corpus reported. -/
theorem the_back_edge_admits_two_releases :
    releases 0 (repeatTrace releaseBody 2) = 2 := by decide

/-- Under the constant exit it is performed once, which is what the C does. -/
theorem the_constant_exit_admits_exactly_one_release :
    releases 0 releaseBody = 1 := by decide

/-- Named for the report: the doubled release is a path the old CFG had and
the C does not, so withdrawing the row is a repair and not a loosening. -/
theorem the_repeated_release_is_a_path_the_c_does_not_have :
    BackEdgeAdmits releaseBody (releaseBody ++ releaseBody) ∧
      ¬ Admits Condition.literalZero releaseBody
        (releaseBody ++ releaseBody) :=
  ⟨the_back_edge_admits_a_second_execution releaseBody,
    the_constant_exit_refuses_the_second_execution (by decide)⟩

/-! ### The successor list itself

Stated directly against the shape `legacy_c.rs` writes into
`self.nodes[header].successors`, so a refactor that reorders or re-derives
that list is measured against these facts rather than against prose. -/

/-- The gateway's successor list for the controlling-expression node. -/
def headerSuccessors (cond : Condition) (bodyEntry continuation : Nat) :
    List Nat :=
  match cond with
  | .literalZero => [continuation]
  | .other => [bodyEntry, continuation]

/-- A literal-zero header carries no edge back into the body. -/
theorem the_literal_zero_header_has_no_back_edge
    {bodyEntry continuation : Nat} (h : bodyEntry ≠ continuation) :
    bodyEntry ∉ headerSuccessors Condition.literalZero bodyEntry continuation := by
  simp [headerSuccessors, h]

/-- Every other header keeps both successors, in the order it had. -/
theorem every_other_header_keeps_both_successors
    (bodyEntry continuation : Nat) :
    headerSuccessors Condition.other bodyEntry continuation =
      [bodyEntry, continuation] := rfl

/-- The exit edge is never the one removed. The loop always reaches its
continuation, so this rule cannot strand a body or truncate a function's
analysed coverage — it removes a path, never a destination. -/
theorem the_exit_edge_is_never_removed
    (cond : Condition) (bodyEntry continuation : Nat) :
    continuation ∈ headerSuccessors cond bodyEntry continuation := by
  cases cond <;> simp [headerSuccessors]

/-- The successor list never grows: the new list is contained in the old one,
which is `removing_the_back_edge_only_removes_paths` at the edge level. -/
theorem the_successor_list_never_grows
    (cond : Condition) (bodyEntry continuation node : Nat)
    (h : node ∈ headerSuccessors cond bodyEntry continuation) :
    node ∈ headerSuccessors Condition.other bodyEntry continuation := by
  cases cond <;> simp_all [headerSuccessors]

end QCore.ConstantDoWhile
