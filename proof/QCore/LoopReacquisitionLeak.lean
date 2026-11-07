import QCore.LoopSiteGenerations

/-!
# A leak joined across a refused re-acquisition (card LOOP-REACQUISITION-LEAK-ACCUSATION)

The abstract rule implemented by
`crates/qcore-c-gateway/src/legacy_loop_reacquisition_leak.rs` and its hook in
`report_leaks` (`legacy_transfer.rs`), gateway rule 42.

Rule 11a (`QCore.LoopSiteGenerations`) gives an acquisition site three
identities: the **current** generation, the **previous** one and the **older**
summary. An earlier generation (previous or older) exists only because the
site's node ran again while it was mentioned, so every witness of an earlier
generation's leak passes that **re-acquisition** with the earlier generation in
hand: it joins two generations of one acquisition. Where the settled analysis
refuses that re-acquisition (`loop re-acquisition: an earlier generation that
may be live is still held`, `Unsupported`), the function already reports that
it cannot follow the site's generations across it. A leak only an earlier
generation of such a site reports is therefore not a validated reachable
violation: it is reported under the refusal instead (the function is `Unknown`,
never `Safe`). A leak any current generation reports, or any earlier
generation of a site whose re-acquisition is not refused, is published exactly
as before.

## What is proved

* `published` keeps every report a current generation makes
  (`a_current_report_is_published`) and every report an earlier generation of
  an unrefused site makes (`an_unrefused_site_is_published`): a true leak in
  the iteration that acquired it, and rule 11a's own loop leaks, stand.
* It **only withdraws** (`the_rule_only_withdraws`), exactly where every
  reporter is an earlier generation of a refused, reported site
  (`withdrawn_iff_every_reporter_is_withdrawn`), and more reporters never
  withdraw more (`more_reporters_never_withdraw`, monotone, so the work-list
  order does not matter).
* **No `Safe`**: a withdrawal needs the refusal reported in the same function,
  so its verdict is `Unknown` or, with another accusation, provisional
  `Unsafe` (`a_withdrawal_is_never_safe`, `a_fully_withdrawn_function_is_unknown`).
* `withdrawVectors` and `shapeVectors` are shared with the Rust tables
  (`tests/unit/legacy_loop_reacquisition_leak.rs`,
  `tests/loop_reacquisition_leak.rs`).

## What this is not

A proof about the Rust (trust boundary 28): that `report_leaks` hands each
reported identity's generation to this decision, and that the refused set is
read from the settled states (rule 11a's decision is monotone, so the settled
state decides as the last iterate of the node did and that iterate reported the
refusal), are review premises bound by the shared vectors and fixtures. It
says nothing about whether an execution reaches a withdrawn exit: a withdrawn
leak is unvalidated, not refuted.
-/

namespace QCore.LoopReacquisitionLeak

open QCore.LoopSiteGenerations (Generation generation base previous older tag)

/-- What the rule reads about one acquisition site: whether the settled
analysis refuses its re-acquisition, and whether the function reported that
refusal at the site's line. -/
structure Refusals where
  refused : Nat → Bool
  reported : Nat → Bool

/-- One reporting identity is withdrawn exactly when it is an earlier
generation of a site whose re-acquisition is refused and reported. -/
def withdrawn (r : Refusals) (identity : Nat) : Bool :=
  generation identity != .current && r.refused (base identity) && r.reported (base identity)

/-- A leak key (exit line, acquisition line) is published when some identity
that reported it on any iterate is not withdrawn. -/
def published (r : Refusals) (reporters : List Nat) : Bool :=
  reporters.any (fun identity => !withdrawn r identity)

/-! ## What stays -/

theorem a_current_report_is_published (r : Refusals) (reporters : List Nat) (i : Nat)
    (hi : i ∈ reporters) (hc : generation i = .current) : published r reporters = true := by
  unfold published
  rw [List.any_eq_true]
  exact ⟨i, hi, by simp [withdrawn, hc]⟩

theorem an_unrefused_site_is_published (r : Refusals) (reporters : List Nat) (i : Nat)
    (hi : i ∈ reporters) (hr : r.refused (base i) = false) : published r reporters = true := by
  unfold published
  rw [List.any_eq_true]
  exact ⟨i, hi, by simp [withdrawn, hr]⟩

theorem an_unreported_refusal_withdraws_nothing (r : Refusals) (reporters : List Nat) (i : Nat)
    (hi : i ∈ reporters) (hr : r.reported (base i) = false) : published r reporters = true := by
  unfold published
  rw [List.any_eq_true]
  exact ⟨i, hi, by simp [withdrawn, hr]⟩

/-! ## It only withdraws, and exactly where every reporter is withdrawn -/

/-- Published after the rule implies reported before it (some reporter). -/
theorem the_rule_only_withdraws (r : Refusals) (reporters : List Nat)
    (h : published r reporters = true) : reporters ≠ [] := by
  intro hnil
  simp [hnil, published] at h

theorem withdrawn_iff_every_reporter_is_withdrawn (r : Refusals) (reporters : List Nat) :
    published r reporters = false ↔ ∀ i ∈ reporters, withdrawn r i = true := by
  unfold published
  constructor
  · intro h i hi
    have := List.any_eq_false.mp h i hi
    simpa using this
  · intro h
    apply List.any_eq_false.mpr
    intro i hi
    simp [h i hi]

theorem more_reporters_never_withdraw (r : Refusals) (reporters more : List Nat)
    (h : published r reporters = true) : published r (reporters ++ more) = true := by
  unfold published at *
  rw [List.any_append, h]
  rfl

/-- A withdrawn identity names a refused and reported site. -/
theorem a_withdrawal_names_a_reported_refusal (r : Refusals) (i : Nat)
    (h : withdrawn r i = true) : r.refused (base i) = true ∧ r.reported (base i) = true := by
  unfold withdrawn at h
  simp only [Bool.and_eq_true] at h
  exact ⟨h.1.2, h.2⟩

/-- Non-vacuity: an earlier generation of a refused, reported site is withdrawn
(both earlier generations, of any site below the tag). -/
theorem an_earlier_generation_of_a_refused_site_is_withdrawn (r : Refusals) (site : Nat)
    (hs : site < tag) (hr : r.refused site = true) (hp : r.reported site = true) :
    withdrawn r (previous site) = true ∧ withdrawn r (older site) = true := by
  have hbp : base (previous site) = site := by
    unfold base previous; rw [Nat.add_mod_right, Nat.mod_eq_of_lt hs]
  have hbo : base (older site) = site := by
    unfold base older
    rw [show site + 2 * tag = site + tag + tag by omega, Nat.add_mod_right, Nat.add_mod_right,
      Nat.mod_eq_of_lt hs]
  have hgp : generation (previous site) = .previous := by
    unfold generation previous
    have : ¬ site + tag < tag := by omega
    have : site + tag < 2 * tag := by omega
    simp [*]
  have hgo : generation (older site) = .older := by
    unfold generation older
    have : ¬ site + 2 * tag < tag := by omega
    have : ¬ site + 2 * tag < 2 * tag := by omega
    simp [*]
  constructor
  · simp [withdrawn, hbp, hgp, hr, hp]
  · simp [withdrawn, hbo, hgo, hr, hp]

/-! ## No `Safe` -/

/-- The three verdicts `classify_diagnostics` publishes. -/
inductive Verdict where
  | safe
  | «unsafe»
  | unknown
  deriving DecidableEq, Repr

def classify (violation refusal : Bool) : Verdict :=
  if violation then .«unsafe» else if refusal then .unknown else .safe

/-- A function's verdict after the rule: `keys` are its leak keys' reporters,
`others` whether it has another accusation, `refusal` whether it reports a
refusal. -/
def verdictAfter (r : Refusals) (keys : List (List Nat)) (others refusal : Bool) : Verdict :=
  classify (others || keys.any (published r)) refusal

/-- Whenever the rule withdraws a key, the function reported the refusal (a
withdrawn reporter names one), so the verdict is never `Safe`. -/
theorem a_withdrawal_is_never_safe (r : Refusals) (keys : List (List Nat)) (others : Bool)
    (k : List Nat) (_hk : k ∈ keys) (hne : k ≠ []) (hw : published r k = false) :
    verdictAfter r keys others true ≠ .safe := by
  have _ := (withdrawn_iff_every_reporter_is_withdrawn r k).mp hw
  have _ := hne
  unfold verdictAfter classify
  cases others <;> cases (keys.any (published r)) <;> simp

/-- The refusal the rule needs is one a withdrawn reporter names, so the
function's refusal flag is set whenever something is withdrawn. -/
theorem a_withdrawn_key_has_its_refusal (r : Refusals) (k : List Nat) (hne : k ≠ [])
    (hw : published r k = false) : ∃ i ∈ k, r.reported (base i) = true := by
  obtain ⟨i, hi⟩ := List.exists_mem_of_ne_nil k hne
  exact ⟨i, hi, (a_withdrawal_names_a_reported_refusal r i
    ((withdrawn_iff_every_reporter_is_withdrawn r k).mp hw i hi)).2⟩

theorem a_fully_withdrawn_function_is_unknown (r : Refusals) (keys : List (List Nat))
    (hw : ∀ k ∈ keys, published r k = false) : verdictAfter r keys false true = .unknown := by
  have : keys.any (published r) = false := List.any_eq_false.mpr (fun k hk => by simp [hw k hk])
  simp [verdictAfter, classify, this]

/-- The rule never turns a non-`Safe` verdict into `Safe`. `refusal` is the
function's refusal flag, which every reported re-acquisition refusal sets
(`hlink`); `keys` are its nonempty leak keys. Before the rule every key is an
accusation; after it the verdict is `Safe` only if it already was. -/
theorem the_rule_moves_no_verdict_to_safe (r : Refusals) (keys : List (List Nat))
    (others refusal : Bool) (hk : ∀ k ∈ keys, k ≠ [])
    (hlink : ∀ i, r.reported i = true → refusal = true)
    (h : verdictAfter r keys others refusal = .safe) :
    classify (others || !keys.isEmpty) refusal = .safe := by
  cases hr : refusal
  · cases ho : others
    · cases keys with
      | nil => simp [classify]
      | cons k rest =>
        exfalso
        have hne : k ≠ [] := hk k (by simp)
        have hpub : published r k = false := by
          cases hp : published r k
          · rfl
          · simp [verdictAfter, classify, ho, hp] at h
        obtain ⟨i, _, hi⟩ := a_withdrawn_key_has_its_refusal r k hne hpub
        have := hlink (base i) hi
        simp_all
    · simp [verdictAfter, classify, ho] at h
  · exfalso
    unfold verdictAfter classify at h
    split at h
    · exact Verdict.noConfusion h
    · simp at h

/-! ## Shared vectors -/

/-- `(identity, refused, reported, withdrawn)`, shared with
`WITHDRAW_VECTORS` of `legacy_loop_reacquisition_leak.rs` (site 7). -/
def withdrawVectors : List (Nat × Bool × Bool × Bool) := [
  (7, true, true, false),
  (536870919, true, true, true),
  (1073741831, true, true, true),
  (536870919, false, true, false),
  (536870919, true, false, false),
  (1073741831, false, false, false),
  (7, false, false, false)
]

def vectorRefusals (refused reported : Bool) : Refusals :=
  { refused := fun _ => refused, reported := fun _ => reported }

theorem withdraw_vectors_hold :
    withdrawVectors.all (fun (i, f, p, w) => withdrawn (vectorRefusals f p) i == w) = true := by
  decide

theorem the_vectors_name_site_seven :
    previous 7 = 536870919 ∧ older 7 = 1073741831 := by decide

/-- The fixtures' verdicts, shared with `SHAPE_VECTORS` of
`tests/loop_reacquisition_leak.rs`. -/
def shapeVectors : List (String × Verdict) := [
  ("snappy-shape", .«unsafe»),
  ("refused-earlier-generation", .unknown),
  ("refused-current-generation-leak", .«unsafe»),
  ("unrefused-loop-leak", .«unsafe»),
  ("no-reacquisition-loop-leak", .«unsafe»),
  ("no-reacquisition-loop-safe", .safe)
]

/-- The model's verdict of each fixture, from its leak keys' reporters: `c` a
current generation, `p` the previous one (site 7), the site refused and
reported where the fixture's re-acquisition is. -/
def shapeVerdict : String → Verdict
  | "snappy-shape" =>
      verdictAfter (vectorRefusals true true) [[7], [previous 7], [previous 7], [previous 7]] false true
  | "refused-earlier-generation" =>
      verdictAfter (vectorRefusals true true) [[previous 7], [previous 7]] false true
  | "refused-current-generation-leak" =>
      verdictAfter (vectorRefusals true true) [[previous 7], [7], [previous 7]] false true
  | "unrefused-loop-leak" =>
      verdictAfter (vectorRefusals false false) [[previous 7]] false false
  | "no-reacquisition-loop-leak" =>
      verdictAfter (vectorRefusals false false) [[7]] false false
  | "no-reacquisition-loop-safe" =>
      verdictAfter (vectorRefusals false false) [] false false
  | _ => .unknown

theorem shape_vectors_hold :
    shapeVectors.all (fun (s, v) => shapeVerdict s == v) = true := by
  decide

end QCore.LoopReacquisitionLeak
