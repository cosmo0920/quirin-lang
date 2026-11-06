/-!
# The member-release half of a member-effect summary (card MEMBER-EFFECT-SUMMARY step 2, gateway rule 38j)

The abstract rule implemented by
`crates/qcore-c-gateway/src/legacy_member_effect.rs` (`BodyReading::read`,
`Parser::member_effect_call`, `minted_environment`, `published_effect`) and
`crates/qcore-schemas/src/member_effect.rs`.

A body's statements touching the places (members) below one formal base are
read as a list of events. Because the check refuses every loop around an event
and every `goto`/`setjmp`, each execution of the body runs a **sublist** of
that list (a conditional skips some events, nothing repeats). A release event
`R(B->F)` and a call event `g(B)` / `g(&B->F)` (a callee whose member effect
names the place) release a place; a null test and a null store touch it
without using its object; anything else is a use.

The check for one place (`placeOk`): when some event releases it, every event
before the first release is a null test or does not touch it, and every event
after is a null store or does not touch it (so it has exactly one release).

Proved here:

* **The body is sound** (`body_sound`): every path of a body whose check
  holds is free of use after release and of double release for that place
  (`safe`: after the first release, nothing touches the place but null
  stores).
* **The cascade is sound** (`cascade_sound`): replacing each call event of a
  path by any trace of its callee preserves that freedom, provided the
  callee's trace releases only what its effect states, touches only what its
  characterised reading touches, and is itself safe for each place it
  releases (which `body_sound` gives for the callee's own body).
* **The summary is withheld** (`a_member_effect_never_enters_the_function_table`,
  `an_ordinary_reader_sees_no_contract`): installing a member-effect summary
  leaves every ordinary lookup unchanged, so a reader that does not apply
  the effect sees an uncontracted function.

## What this is not

A model of the lexical reader: that an occurrence is a test, a store, a
release or a call event, that the body has no loop around an event, and that
a callee's characterised reading lists every member it touches are the Rust's
(trust boundary 28, and the stated premises of rule 5k: distinct members own
distinct objects, a stored pointer is tracked only through a bound place or
sub-custody).
-/

namespace QCore.MemberEffectSummary

/-- A member path below the base. -/
abbrev Place := String

/-- One event of a body over the places below one base. -/
inductive Ev where
  /-- A null test of the place. -/
  | test (p : Place)
  /-- A store of a null pointer constant into the place. -/
  | store (p : Place)
  /-- The place's object handed to a release position. -/
  | release (p : Place)
  /-- A call whose effect releases `rel` and whose callee touches `touch`. -/
  | call (rel : List Place) (touch : List Place)
  /-- Any other use of the place. -/
  | use (p : Place)
  /-- An event touching no place below the base. -/
  | other
  deriving DecidableEq, Repr

def releases (p : Place) : Ev → Bool
  | .release q => q == p
  | .call rel _ => rel.contains p
  | _ => false

def touches (p : Place) : Ev → Bool
  | .test q => q == p
  | .store q => q == p
  | .release q => q == p
  | .use q => q == p
  | .call rel touch => rel.contains p || touch.contains p
  | .other => false

def isTest (p : Place) : Ev → Bool
  | .test q => q == p
  | _ => false

def isStore (p : Place) : Ev → Bool
  | .store q => q == p
  | _ => false

theorem releases_touches (p : Place) (e : Ev) (h : releases p e = true) :
    touches p e = true := by
  cases e <;> simp_all [releases, touches]

theorem a_release_is_no_store (p : Place) (e : Ev) (h : releases p e = true) :
    isStore p e = false := by
  cases e <;> simp_all [releases, isStore]

theorem a_test_releases_nothing (p : Place) (e : Ev) (h : isTest p e = true) :
    releases p e = false := by
  cases e <;> simp_all [releases, isTest]

theorem a_store_releases_nothing (p : Place) (e : Ev) (h : isStore p e = true) :
    releases p e = false := by
  cases e <;> simp_all [releases, isStore]

/-- After the release: only null stores touch the place. -/
def checkAfter (p : Place) : List Ev → Bool
  | [] => true
  | e :: rest => (!touches p e || isStore p e) && checkAfter p rest

/-- Before the release: only null tests touch the place; the first releasing
event switches to `checkAfter`. -/
def checkBefore (p : Place) : List Ev → Bool
  | [] => true
  | e :: rest =>
      if releases p e then checkAfter p rest
      else (!touches p e || isTest p e) && checkBefore p rest

/-- The check for one place (`read_base`'s P2 for a place with an event). -/
def placeOk (p : Place) (b : List Ev) : Bool :=
  !(b.any (releases p)) || checkBefore p b

/-- Freedom from use after release and from double release on one trace:
after the first release, nothing touches the place but null stores. -/
def safe (p : Place) : List Ev → Bool
  | [] => true
  | e :: rest => if releases p e then checkAfter p rest else safe p rest

/-- The sublist-closed invariant the check implies. -/
def inv (p : Place) : List Ev → Bool
  | [] => true
  | e :: rest =>
      if releases p e then checkAfter p rest
      else (!touches p e || isTest p e || isStore p e) && inv p rest

theorem checkAfter_append (p : Place) (xs ys : List Ev) :
    checkAfter p (xs ++ ys) = (checkAfter p xs && checkAfter p ys) := by
  induction xs with
  | nil => simp [checkAfter]
  | cons x xs ih => simp [checkAfter, ih, Bool.and_assoc]

theorem checkAfter_has_no_release (p : Place) (s : List Ev) (h : checkAfter p s = true) :
    ∀ x ∈ s, releases p x = false := by
  induction s with
  | nil => intro x hx; cases hx
  | cons e rest ih =>
      intro x hx
      simp only [checkAfter, Bool.and_eq_true, Bool.or_eq_true, Bool.not_eq_true'] at h
      obtain ⟨he, hr⟩ := h
      rcases List.mem_cons.mp hx with rfl | hx
      · cases hrel : releases p x
        · rfl
        · rcases he with he | he
          · rw [releases_touches p x hrel] at he; cases he
          · rw [a_release_is_no_store p x hrel] at he; cases he
      · exact ih hr x hx

theorem safe_of_no_release (p : Place) (s : List Ev) (h : ∀ x ∈ s, releases p x = false) :
    safe p s = true := by
  induction s with
  | nil => rfl
  | cons e rest ih =>
      simp only [safe, h e (List.mem_cons_self ..)]
      exact ih (fun x hx => h x (List.mem_cons_of_mem _ hx))

theorem safe_append_of_no_release (p : Place) (xs ys : List Ev)
    (h : ∀ x ∈ xs, releases p x = false) : safe p (xs ++ ys) = safe p ys := by
  induction xs with
  | nil => rfl
  | cons x xs ih =>
      simp only [List.cons_append, safe, h x (List.mem_cons_self ..)]
      exact ih (fun y hy => h y (List.mem_cons_of_mem _ hy))

theorem safe_append (p : Place) (xs ys : List Ev) (hx : safe p xs = true)
    (hy : checkAfter p ys = true) : safe p (xs ++ ys) = true := by
  induction xs with
  | nil =>
      exact safe_of_no_release p ys (checkAfter_has_no_release p ys hy)
  | cons x xs ih =>
      simp only [List.cons_append, safe]
      simp only [safe] at hx
      cases hr : releases p x
      · simp only [hr, Bool.false_eq_true, ↓reduceIte] at hx ⊢
        exact ih hx
      · simp only [hr, ↓reduceIte] at hx ⊢
        rw [checkAfter_append, hx, hy]; rfl

theorem checkAfter_sublist (p : Place) {s b : List Ev} (hs : s.Sublist b)
    (hb : checkAfter p b = true) : checkAfter p s = true := by
  induction hs with
  | slnil => rfl
  | cons a _ ih =>
      simp only [checkAfter, Bool.and_eq_true] at hb
      exact ih hb.2
  | cons_cons a _ ih =>
      simp only [checkAfter, Bool.and_eq_true] at hb ⊢
      exact ⟨hb.1, ih hb.2⟩

/-- After the release a trace's events are null stores or untouching, which
`inv`'s before-mode admits as well. -/
theorem inv_of_checkAfter (p : Place) (s : List Ev) (h : checkAfter p s = true) :
    inv p s = true := by
  induction s with
  | nil => rfl
  | cons e rest ih =>
      simp only [checkAfter, Bool.and_eq_true] at h
      have hnr := checkAfter_has_no_release p (e :: rest) (by simp [checkAfter, h.1, h.2]) e
        (List.mem_cons_self ..)
      simp only [inv, hnr]
      simp only [Bool.or_eq_true, Bool.not_eq_true'] at h
      rcases h.1 with h1 | h1
      · simp [h1, ih h.2]
      · simp [h1, ih h.2]

theorem inv_sublist (p : Place) {s b : List Ev} (hs : s.Sublist b) (hb : inv p b = true) :
    inv p s = true := by
  induction hs with
  | slnil => rfl
  | cons a hsub ih =>
      simp only [inv] at hb
      cases hr : releases p a
      · simp only [hr, Bool.false_eq_true, ↓reduceIte, Bool.and_eq_true] at hb
        exact ih hb.2
      · simp only [hr, ↓reduceIte] at hb
        exact inv_sublist_after p hsub hb
  | cons_cons a hsub ih =>
      simp only [inv] at hb ⊢
      cases hr : releases p a
      · simp only [hr, Bool.false_eq_true, ↓reduceIte, Bool.and_eq_true] at hb ⊢
        exact ⟨hb.1, ih hb.2⟩
      · simp only [hr, ↓reduceIte] at hb ⊢
        exact checkAfter_sublist p hsub hb
where
  inv_sublist_after (p : Place) {s b : List Ev} (hs : s.Sublist b)
      (hb : checkAfter p b = true) : inv p s = true :=
    inv_of_checkAfter p s (checkAfter_sublist p hs hb)

theorem inv_of_checkBefore (p : Place) (b : List Ev) (h : checkBefore p b = true) :
    inv p b = true := by
  induction b with
  | nil => rfl
  | cons e rest ih =>
      simp only [checkBefore] at h
      simp only [inv]
      cases hr : releases p e
      · simp only [hr, Bool.false_eq_true, ↓reduceIte, Bool.and_eq_true, Bool.or_eq_true]
          at h ⊢
        refine ⟨?_, ih h.2⟩
        rcases h.1 with h1 | h1
        · exact Or.inl (Or.inl h1)
        · exact Or.inl (Or.inr h1)
      · simp only [hr, ↓reduceIte] at h ⊢
        exact h

theorem safe_of_inv (p : Place) (s : List Ev) (h : inv p s = true) : safe p s = true := by
  induction s with
  | nil => rfl
  | cons e rest ih =>
      simp only [inv] at h
      simp only [safe]
      cases hr : releases p e
      · simp only [hr, Bool.false_eq_true, ↓reduceIte, Bool.and_eq_true] at h ⊢
        exact ih h.2
      · simp only [hr, ↓reduceIte] at h ⊢
        exact h

/-- **The body is sound**: every path of a checked body is free of use after
release and of double release for the place. -/
theorem body_sound (p : Place) (b s : List Ev) (hb : placeOk p b = true) (hs : s.Sublist b) :
    safe p s = true := by
  simp only [placeOk, Bool.or_eq_true, Bool.not_eq_true'] at hb
  rcases hb with hb | hb
  · apply safe_of_no_release
    intro x hx
    have hxb := hs.subset hx
    cases hr : releases p x
    · rfl
    · have : b.any (releases p) = true := List.any_eq_true.mpr ⟨x, hxb, hr⟩
      rw [hb] at this; cases this
  · exact safe_of_inv p s (inv_sublist p hs (inv_of_checkBefore p b hb))

/-- What a call event stands for: a trace of its callee, over the caller's
places. Every other event stands for itself. -/
structure Expansion (p : Place) where
  exp : Ev → List Ev
  /-- A non-call event is itself. -/
  itself : ∀ e, (∀ rel touch, e ≠ .call rel touch) → exp e = [e]
  /-- A callee touches only what its characterised reading lists. -/
  quiet : ∀ e, touches p e = false → ∀ x ∈ exp e, touches p x = false
  /-- A callee releases only what its effect states. -/
  stated : ∀ e, releases p e = false → ∀ x ∈ exp e, releases p x = false
  /-- A callee's trace is safe for each place it releases (its own body
  check, by `body_sound`). -/
  sound : ∀ e, releases p e = true → safe p (exp e) = true

theorem checkAfter_of_untouching (p : Place) (xs : List Ev)
    (h : ∀ x ∈ xs, touches p x = false) : checkAfter p xs = true := by
  induction xs with
  | nil => rfl
  | cons x xs ih =>
      simp only [checkAfter, h x (List.mem_cons_self ..), Bool.not_false, Bool.true_or,
        Bool.true_and]
      exact ih (fun y hy => h y (List.mem_cons_of_mem _ hy))

theorem quiet_checkAfter (p : Place) (E : Expansion p) (s : List Ev)
    (h : checkAfter p s = true) : checkAfter p (s.flatMap E.exp) = true := by
  induction s with
  | nil => rfl
  | cons e rest ih =>
      simp only [checkAfter, Bool.and_eq_true] at h
      simp only [List.flatMap_cons, checkAfter_append, ih h.2, Bool.and_true]
      simp only [Bool.or_eq_true, Bool.not_eq_true'] at h
      rcases h.1 with ht | hst
      · -- An untouching event's expansion touches nothing.
        exact checkAfter_of_untouching p _ (E.quiet e ht)
      · -- A null store is itself.
        have hne : ∀ rel touch, e ≠ .call rel touch := by
          intro rel touch heq; subst heq; simp [isStore] at hst
        rw [E.itself e hne]
        simp [checkAfter, hst]

theorem flatten_no_release (p : Place) (E : Expansion p) (e : Ev) (h : releases p e = false) :
    ∀ x ∈ E.exp e, releases p x = false :=
  E.stated e h

/-- The flattened invariant: the trace with each call event replaced by its
callee's trace is safe. -/
theorem flatten_safe_of_inv (p : Place) (E : Expansion p) (s : List Ev) (h : inv p s = true) :
    safe p (s.flatMap E.exp) = true := by
  induction s with
  | nil => rfl
  | cons e rest ih =>
      simp only [inv] at h
      simp only [List.flatMap_cons]
      cases hr : releases p e
      · simp only [hr, Bool.false_eq_true, ↓reduceIte, Bool.and_eq_true] at h
        rw [safe_append_of_no_release p _ _ (flatten_no_release p E e hr)]
        exact ih h.2
      · simp only [hr, ↓reduceIte] at h
        exact safe_append p _ _ (E.sound e hr) (quiet_checkAfter p E rest h)

/-- **The cascade is sound**: on every path of a checked caller, replacing
each call event by a trace of its callee keeps the place free of use after
release and of double release. -/
theorem cascade_sound (p : Place) (E : Expansion p) (b s : List Ev) (hb : placeOk p b = true)
    (hs : s.Sublist b) : safe p (s.flatMap E.exp) = true := by
  simp only [placeOk, Bool.or_eq_true, Bool.not_eq_true'] at hb
  rcases hb with hb | hb
  · apply safe_of_no_release
    intro x hx
    rcases List.mem_flatMap.mp hx with ⟨e, he, hxe⟩
    have heb := hs.subset he
    cases hr : releases p e
    · exact E.stated e hr x hxe
    · have : b.any (releases p) = true := List.any_eq_true.mpr ⟨e, heb, hr⟩
      rw [hb] at this; cases this
  · exact flatten_safe_of_inv p E s (inv_sublist p hs (inv_of_checkBefore p b hb))

/-- A second release of a place is refused by the check. -/
theorem two_releases_are_refused (p : Place) :
    placeOk p [.release p, .release p] = false := by
  simp [placeOk, checkBefore, checkAfter, releases, touches, isStore]

/-- A use after a callee released the place is refused by the check. -/
theorem a_use_after_a_call_is_refused (p : Place) :
    placeOk p [.call [p] [], .use p] = false := by
  simp [placeOk, checkBefore, checkAfter, releases, touches, isStore]

/-- A callee touching a place this body released before the call is refused. -/
theorem a_callee_touching_a_released_place_is_refused (p q : Place) :
    placeOk p [.release p, .call [q] [p]] = false := by
  simp [placeOk, checkBefore, checkAfter, releases, touches, isStore]

/-- `metric_release_storage`: releases, then null stores, is admitted. -/
example : placeOk "a" [.release "a", .release "b", .store "a", .store "b"] = true := by decide

/-- `cmt_opts_exit`: each member under its own null test. -/
example : placeOk "a" [.test "a", .release "a", .test "b", .release "b"] = true := by decide

/-! ## The summary is withheld from the ordinary table -/

/-- An environment: the ordinary function table and the member-effect table. -/
structure Env (C S : Type) where
  functions : List (String × C)
  effects : List (String × S)

def Env.contract {C S : Type} (env : Env C S) (name : String) : Option C :=
  (env.functions.find? (fun entry => entry.1 == name)).map Prod.snd

/-- `ResourceContracts::insert_member_effect`: a name the ordinary table holds
keeps that entry and gets none. -/
def Env.insertEffect {C S : Type} (env : Env C S) (name : String) (s : S) : Env C S :=
  if env.functions.any (fun entry => entry.1 == name) then env
  else { env with effects := (name, s) :: env.effects }

theorem a_member_effect_never_enters_the_function_table {C S : Type} (env : Env C S)
    (name : String) (s : S) : (env.insertEffect name s).functions = env.functions := by
  unfold Env.insertEffect
  split <;> rfl

theorem an_ordinary_reader_sees_no_contract {C S : Type} (env : Env C S) (name n : String)
    (s : S) : (env.insertEffect name s).contract n = env.contract n := by
  simp [Env.contract, a_member_effect_never_enters_the_function_table]

end QCore.MemberEffectSummary
