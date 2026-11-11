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
# External `quirin-audit` subcommand plugins

Card **W45-AUDIT-SUBCOMMAND-PLUGINS** (contract specification §C11.2). An
external subcommand `quirin-audit-NAME` runs out of process; the host observes
either a **failure** (a spawn failure, a timeout, a quota breach, a nonzero
exit, a malformed response, a protocol-version mismatch, a failure the plugin
reports, or an artifact that does not verify) or a **completed** run with its
verdict claims. Each claim is classified by the checker-plugin firewall of
`QCore.VerdictSemigroup` (`firewall`), and a row of the result is the join of
the host's own verdict for the artifact, the run's verdict (`Unknown` for a
failure, nothing otherwise) and the classified claims about it. A failed
run's claims are joined nowhere. This is the model of
`crates/qcore-audit/src/subcommand_plugins.rs` (`join`, `classify_claim`,
`classify_run`).

## What is proved

* **No plugin makes a row `Safe` without an accepted certificate.** A row is
  `Safe` exactly when the host's verdict is `Safe`, the run completed, and every
  claim about the artifact is a `Safe` proposal whose certificate the audit's
  own checker accepted.
* **The command line certifies nothing.** With no checker (every claim's
  evidence absent or unchecked, as on the command line today), a row is `Safe`
  exactly when the host's verdict is `Safe` and the plugin claimed nothing
  about the artifact; an uncertified `Safe` or `Unsafe` claim alone makes a
  `Safe` host row `Pending`.
* **A failed plugin is `Unknown`**, or the host's `Unsafe`, whatever it
  claimed.
* **Monotonicity.** The join never lowers the host's verdict (an `Unsafe`
  stays `Unsafe`), adding a claim never lowers a row, a higher host verdict
  never gives a lower row, and the result's verdict is `Safe` only when every
  row is and the run completed.

## What this is not

That the host classifies a run and a claim as the Rust does — which failure is
which, and that no Rust path constructs a certified claim (`CoreCertificate`
has no public constructor) — is pinned by
`crates/qcore-audit/tests/subcommand_plugins.rs` and the module's unit tests,
not proved here. The trusted checkers' soundness is the firewall's premise.
-/

namespace QCore.SubcommandPlugins

open QCore.VerdictSemigroup

/-- What the host observed of one plugin run. -/
inductive Run where
  | failure
  | completed (claims : List Proposal)
  deriving DecidableEq, Repr

/-- The run's own contribution: `Unknown` for a failure, the identity
otherwise. -/
def Run.value : Run → V
  | .failure => .unknown
  | .completed _ => .safe

/-- The claims joined into the result: none of a failed run. -/
def Run.joined : Run → List Proposal
  | .failure => []
  | .completed claims => claims

/-- The classified claims. -/
def Run.classified (r : Run) : List V :=
  r.joined.map fun p => firewall (.answered p)

/-- A row of the result: the host's verdict, the run's and the claims'. -/
def row (host : V) (r : Run) : V :=
  V.join host (V.join r.value (fold r.classified))

/-- The result's verdict over its rows. -/
def result (rows : List V) (r : Run) : V :=
  V.join (fold rows) r.value

theorem join_eq_safe_iff (a b : V) : V.join a b = .safe ↔ a = .safe ∧ b = .safe := by
  cases a <;> cases b <;> decide

/-- A row is `Safe` exactly when the host's verdict is `Safe`, the run
completed and every claim about the artifact carries a certificate the audit's
checker accepted. -/
theorem a_row_is_safe_only_with_accepted_certificates (host : V) (r : Run) :
    row host r = .safe ↔
      host = .safe ∧ ∃ claims, r = .completed claims ∧
        ∀ p ∈ claims, p = .safe (.offered .accepted) := by
  cases r with
  | failure => simp [row, join_eq_safe_iff, Run.value]
  | completed claims =>
    simp only [row, join_eq_safe_iff, Run.value, Run.classified, Run.joined, true_and,
      fold_eq_safe_iff, List.mem_map, forall_exists_index, and_imp,
      forall_apply_eq_imp_iff₂, a_plugin_is_safe_only_with_an_accepted_certificate]
    constructor
    · rintro ⟨h, all⟩
      exact ⟨h, claims, rfl, fun p mem => by simpa using all p mem⟩
    · rintro ⟨h, _, ⟨⟩, all⟩
      exact ⟨h, fun p mem => by simpa using all p mem⟩

/-- No plugin output raises a row to `Safe` without an accepted certificate:
a completed run with any claim lacking one, or a failure, leaves it non-`Safe`. -/
theorem no_plugin_output_is_safe_without_an_accepted_certificate (host : V) (r : Run)
    (uncertified : r = .failure ∨
      ∃ p ∈ r.joined, p ≠ .safe (.offered .accepted)) :
    row host r ≠ .safe := by
  intro safe
  obtain ⟨_, claims, rfl, all⟩ := (a_row_is_safe_only_with_accepted_certificates host r).mp safe
  rcases uncertified with h | ⟨p, mem, ne⟩
  · cases h
  · exact ne (all p mem)

/-- Evidence the command line can offer: none, or none it has a checker for. -/
def Unchecked : Evidence → Prop
  | .none => True
  | .offered .noChecker => True
  | _ => False

/-- A claim the command line sees: every proposal's evidence is unchecked. -/
def Uncheckable : Proposal → Prop
  | .safe e => Unchecked e
  | .violated e => Unchecked e
  | .unknown => True

/-- On the command line (no checker), a row is `Safe` exactly when the host's
verdict is `Safe` and the completed plugin claimed nothing about the
artifact. -/
theorem without_a_checker_only_an_unclaimed_safe_row_is_safe (host : V) (claims : List Proposal)
    (cli : ∀ p ∈ claims, Uncheckable p) :
    row host (.completed claims) = .safe ↔ host = .safe ∧ claims = [] := by
  rw [a_row_is_safe_only_with_accepted_certificates]
  constructor
  · rintro ⟨h, _, ⟨⟩, all⟩
    refine ⟨h, ?_⟩
    cases claims with
    | nil => rfl
    | cons p ps =>
      have accepted := all p (List.mem_cons_self ..)
      have := cli p (List.mem_cons_self ..)
      subst accepted
      simp [Uncheckable, Unchecked] at this
  · rintro ⟨h, rfl⟩
    exact ⟨h, [], rfl, by simp⟩

/-- An uncertified `Safe` or `Unsafe` claim on a `Safe` host row is
`Pending`. -/
theorem an_uncertified_claim_makes_a_safe_row_pending :
    row .safe (.completed [.safe .none]) = .pending ∧
      row .safe (.completed [.safe (.offered .noChecker)]) = .pending ∧
      row .safe (.completed [.violated .none]) = .pending ∧
      row .safe (.completed [.violated (.offered .noChecker)]) = .pending := by
  decide

/-- A failed plugin joined with a `Safe` host row is `Unknown`; with any host
verdict, it is `Unknown` or the host's `Unsafe`. -/
theorem a_failed_plugin_is_unknown (host : V) :
    row .safe .failure = .unknown ∧ V.unknown.rank ≤ (row host .failure).rank := by
  refine ⟨by decide, ?_⟩
  cases host <;> decide

/-- The join never lowers the host's verdict. -/
theorem the_join_never_lowers_the_host (host : V) (r : Run) :
    host.rank ≤ (row host r).rank :=
  le_join_left _ _

/-- No claim lowers an existing `Unsafe`. -/
theorem no_claim_lowers_an_existing_unsafe (r : Run) :
    row .provisionalUnsafe r = .provisionalUnsafe := by
  unfold row
  generalize V.join r.value (fold r.classified) = rest
  cases rest <;> decide

/-- Adding a claim never lowers a row. -/
theorem adding_a_claim_never_lowers (host : V) (claims more : List Proposal) :
    (row host (.completed claims)).rank ≤ (row host (.completed (claims ++ more))).rank := by
  simp only [row, join_rank, Run.value, Run.classified, Run.joined, List.map_append]
  have := adding_a_link_never_lowers (claims.map fun p => firewall (.answered p))
    (more.map fun p => firewall (.answered p))
  omega

/-- A higher host verdict never gives a lower row. -/
theorem a_row_is_monotone_in_the_host (host host' : V) (r : Run)
    (le : host.rank ≤ host'.rank) : (row host r).rank ≤ (row host' r).rank := by
  simp only [row, join_rank]
  omega

/-- The result's verdict is `Safe` only when every row is and the run
completed. -/
theorem the_result_is_safe_only_when_every_row_is (rows : List V) (r : Run) :
    result rows r = .safe ↔ (∀ x ∈ rows, x = .safe) ∧ ∃ claims, r = .completed claims := by
  cases r <;> simp [result, join_eq_safe_iff, fold_eq_safe_iff, Run.value]

end QCore.SubcommandPlugins
