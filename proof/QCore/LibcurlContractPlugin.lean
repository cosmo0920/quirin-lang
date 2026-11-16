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

import QCore.VerdictSemigroup

/-!
# The libcurl contract plugin

Card **W48-LIBCURL-CONTRACT-PLUGIN** (user decision 2026-09-28; contract
specification §C11.1 and §C11.2). The `quirin-audit libcurl` plugin carries
libcurl's contracts, checks each binding (one libcurl function) against a
pinned release with its own checker, and its answers reach the audit only
through the checker-plugin firewall of `QCore.VerdictSemigroup` (`firewall`)
and the join. This is the model of `crates/quirin-audit-plugin-libcurl`
(`checker::answer`, `lib::claim`, `provider::propose`) and of the plugin-
specific premises the host's composition rests on.

## What is proved

* **The checker's decision.** A binding's answer is `Safe` exactly when the
  prepared tree matches the pin, the binding carries a contract and every one
  of its obligations holds; a pin mismatch, a binding without a contract, or
  one failed obligation makes it `Unknown`. The checker never answers a
  violation.
* **What the audit publishes.** On both routes — the subcommand's claim (a
  certificate reference no trusted checker reads, `QP016`) and the checker
  package's answer (no evidence, `QP008`) — an established binding is
  `Pending`, never `Safe`; an unestablished one is `Unknown`; a failed run
  (spawn, timeout, quota, a version or pin mismatch the run refuses) is
  `Unknown`; nothing the plugin answers is ever a provisional `Unsafe`.
* **Composition.** A caller's published value is the fold of its own
  analysis and the links of the bound contracts it applies: it is never below
  the host's own value (an `Unsafe` host stays `Unsafe`, an `Unknown` stays at
  least `Unknown`), a caller that applies any bound contract is never `Safe`,
  a caller that applies none (the `typecheck-gcc.h` warning stubs are never
  bound) keeps exactly the host's value, and the reasoned join keeps every
  binding's reasons.

## The premises (not proved here)

* Each obligation shape implies its contract clause of the C function it is
  read from (`OnlyIn` of a copying call means the argument is borrowed, a
  `Returns` list bounds the result, ...): the checker's own soundness, stated
  per binding in `checker.rs` and in the contract module's comments.
* The named assumptions of every claim (`upstream-release`,
  `release-build`, `default-allocators`, `libc-c-standard-abi`,
  `heap-kind`).
* That the Rust computes these functions: pinned by the crate's tests.
-/

namespace QCore.LibcurlContractPlugin

open QCore.VerdictSemigroup

/-- The checker's own answer for one binding. There is no violation. -/
inductive Answer where
  | established
  | notEstablished
  deriving DecidableEq, Repr

/-- What the checker sees for one binding: whether the prepared tree matches
the pin, whether the binding carries a contract (the vocabulary can state
it), and each obligation's outcome. -/
structure Binding where
  pinMatches : Bool
  contracted : Bool
  obligations : List Bool
  deriving DecidableEq, Repr

/-- `checker::answer`. -/
def answer (b : Binding) : Answer :=
  if b.pinMatches && b.contracted && b.obligations.all id then .established
  else .notEstablished

/-- A binding is established exactly when the pin matches, it carries a
contract and every obligation holds. -/
theorem established_iff (b : Binding) :
    answer b = .established ↔
      b.pinMatches = true ∧ b.contracted = true ∧ ∀ o ∈ b.obligations, o = true := by
  unfold answer
  split
  · rename_i c
    simp only [Bool.and_eq_true, List.all_eq_true, id_eq] at c
    exact ⟨fun _ => ⟨c.1.1, c.1.2, c.2⟩, fun _ => rfl⟩
  · rename_i c
    constructor
    · intro h; cases h
    · rintro ⟨p, k, os⟩
      exact absurd (by simp only [Bool.and_eq_true, List.all_eq_true, id_eq]; exact ⟨⟨p, k⟩, os⟩) c

/-- A tree that does not match the pin establishes nothing. -/
theorem a_pin_mismatch_establishes_nothing (b : Binding) (mismatch : b.pinMatches = false) :
    answer b = .notEstablished := by
  simp [answer, mismatch]

/-- A binding without a contract (the vocabulary cannot state it) is never
established. -/
theorem an_uncontracted_binding_is_not_established (b : Binding)
    (none : b.contracted = false) : answer b = .notEstablished := by
  simp [answer, none]

/-- One failed obligation is enough to establish nothing. -/
theorem a_failed_obligation_establishes_nothing (b : Binding) (failed : false ∈ b.obligations) :
    answer b = .notEstablished := by
  cases h : answer b with
  | notEstablished => rfl
  | established =>
    have := ((established_iff b).mp h).2.2 false failed
    cases this

/-- A run of the plugin: completed with the binding's facts, or failed (a
spawn failure, a timeout, a quota breach, a nonzero exit, a protocol-version
mismatch, a tarball or tree the run refuses as not the pin). -/
inductive Run where
  | completed (b : Binding)
  | failed
  deriving DecidableEq, Repr

/-- The subcommand route (`lib::claim` classified by `classify_claim`): an
established binding is a `Safe` proposal offering a certificate reference no
trusted-core checker reads; otherwise an `Unknown` proposal. -/
def claimOutcome : Run → Outcome
  | .failed => .failure
  | .completed b =>
    match answer b with
    | .established => .answered (.safe (.offered .noChecker))
    | .notEstablished => .answered .unknown

/-- The checker-package route (`provider::propose` through
`verify_plugin_claim`): an established binding is a `Safe` proposal with no
evidence; otherwise `Unknown`. -/
def packageOutcome : Run → Outcome
  | .failed => .failure
  | .completed b =>
    match answer b with
    | .established => .answered (.safe .none)
    | .notEstablished => .answered .unknown

/-- What the audit publishes for a binding on either route. -/
def published (route : Run → Outcome) (r : Run) : V := firewall (route r)

/-- The two routes publish the same value for every run. -/
theorem the_routes_agree (r : Run) : published claimOutcome r = published packageOutcome r := by
  cases r with
  | failed => rfl
  | completed b => cases h : answer b <;> simp [published, claimOutcome, packageOutcome, h, firewall]

/-- Nothing the plugin answers is published `Safe`. -/
theorem never_published_safe (r : Run) : published claimOutcome r ≠ .safe := by
  cases r with
  | failed => simp [published, claimOutcome, firewall]
  | completed b => cases h : answer b <;> simp [published, claimOutcome, h, firewall]

/-- Nothing the plugin answers is published as a provisional `Unsafe`. -/
theorem never_published_unsafe (r : Run) : published claimOutcome r ≠ .provisionalUnsafe := by
  cases r with
  | failed => simp [published, claimOutcome, firewall]
  | completed b => cases h : answer b <;> simp [published, claimOutcome, h, firewall]

/-- An established binding is published `Pending`, the uncertified claim. -/
theorem an_established_binding_is_pending (b : Binding) (h : answer b = .established) :
    published claimOutcome (.completed b) = .pending := by
  simp [published, claimOutcome, h, firewall]

/-- An unestablished binding is published `Unknown`. -/
theorem an_unestablished_binding_is_unknown (b : Binding) (h : answer b = .notEstablished) :
    published claimOutcome (.completed b) = .unknown := by
  simp [published, claimOutcome, h, firewall]

/-- A failed run, a pin mismatch included, is published `Unknown`. -/
theorem a_failed_run_is_unknown : published claimOutcome .failed = .unknown := rfl

theorem a_pin_mismatch_is_unknown (b : Binding) (mismatch : b.pinMatches = false) :
    published claimOutcome (.completed b) = .unknown :=
  an_unestablished_binding_is_unknown b (a_pin_mismatch_establishes_nothing b mismatch)

/-- Every published value is `Pending` or `Unknown`. -/
theorem published_is_pending_or_unknown (r : Run) :
    published claimOutcome r = .pending ∨ published claimOutcome r = .unknown := by
  cases r with
  | failed => exact Or.inr rfl
  | completed b => cases h : answer b <;> simp [published, claimOutcome, h, firewall]

/-! ## Composition into a caller -/

/-- A caller's published value: the fold of its own analysis and the
published values of the bound contracts it applies. -/
def caller (host : V) (links : List Run) : V :=
  fold (host :: links.map (published claimOutcome))

/-- The join never lowers the host's verdict. -/
theorem caller_never_below_host (host : V) (links : List Run) :
    host.rank ≤ (caller host links).rank :=
  every_link_is_below_the_fold (List.mem_cons_self ..)

/-- An `Unsafe` host stays `Unsafe`. -/
theorem an_unsafe_host_stays_unsafe (links : List Run) :
    caller .provisionalUnsafe links = .provisionalUnsafe := by
  have h := caller_never_below_host .provisionalUnsafe links
  revert h
  cases caller .provisionalUnsafe links <;> simp [V.rank]

/-- A caller that applies any bound contract is never `Safe`. -/
theorem a_bound_caller_is_never_safe (host : V) (r : Run) (rs : List Run) (member : r ∈ rs) :
    caller host rs ≠ .safe :=
  a_non_safe_link_keeps_the_chain_non_safe
    (List.mem_cons_of_mem _ (List.mem_map_of_mem member)) (never_published_safe r)

/-- A caller that applies no bound contract — a call to a `typecheck-gcc.h`
warning stub is one, since no stub is bound — keeps exactly the host's
value. -/
theorem an_unbound_caller_keeps_the_host_value (host : V) : caller host [] = host := by
  simp [caller, fold, join_safe]

/-- A caller whose host analysis is otherwise `Safe` and that applies only
established bindings is `Pending`: the move `curl_global_init`'s one caller
makes (`rd_http_global_init`, Unknown -> Pending). -/
theorem a_safe_host_over_established_bindings_is_pending (bs : List Binding) (nonempty : bs ≠ [])
    (all : ∀ b ∈ bs, answer b = .established) :
    caller .safe (bs.map Run.completed) = .pending := by
  induction bs with
  | nil => exact absurd rfl nonempty
  | cons b rest ih =>
    have hb : published claimOutcome (.completed b) = .pending :=
      an_established_binding_is_pending b (all b (List.mem_cons_self ..))
    by_cases e : rest = []
    · subst e; simp [caller, fold, hb, safe_join, join_safe]
    · have hr := ih e (fun b' m => all b' (List.mem_cons_of_mem _ m))
      simp only [caller, fold, List.map_cons, List.map_map] at hr ⊢
      rw [safe_join] at hr ⊢
      rw [hb, hr]; rfl

/-- The reasoned join keeps every binding's reasons beside the host's. -/
theorem every_reason_is_kept {R : Type} (host link : Reasoned R) (r : R)
    (member : r ∈ link.reasons) : r ∈ (Reasoned.join host link).reasons :=
  Reasoned.join_keeps_reasons host link r (Or.inr member)

end QCore.LibcurlContractPlugin
