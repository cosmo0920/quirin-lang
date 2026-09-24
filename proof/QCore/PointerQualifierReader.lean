/-!
# The declared-ABI reader's qualifier-after-the-last-`*` step

Card **W43-LIBC-RESULT-ALIAS**, discharging `W43-LIBC-POINTER-QUALIFIER-READER`.
`QCore.DeclaredAbiAgreement` takes the C side's projection as given, and
`QCore.TypedefProjection` models how a typedef name is followed. This module
models the one step this card changed in
`crates/qcore-c-gateway/src/legacy_declared_abi.rs`: how a **parameter**
declarator's words are read before its pointer depth is counted.

## What was refused, and why it is a reader boundary

`project_declarator` counts only *trailing* `*` words and refuses a declarator
whose head still contains a `*`. A parameter spelled
`const char *__restrict __format` — glibc's `printf`, `memcpy`, `strcpy` and
others — ends in a qualifier, so it has no trailing star and a star in its head,
and it projected to nothing: every such declaration was withdrawn with
`abi-unreadable-declaration` at every call site.

## The rule

A qualifier after a parameter declarator's **last** `*` qualifies the parameter
object itself, and "each parameter declared with qualified type is taken as
having the unqualified version of its declared type" (C11 6.7.6.3p15). The
reader now drops exactly those qualifiers, and nothing else.

## What is proved

* **The widening only adds.** Every parameter the earlier reader projected is
  projected identically, so no comparison that agreed or disagreed before can
  have moved for a declarator that was already read.
* **It reads the unqualified type** — as an equation, for any run of trailing
  qualifiers: C11 6.7.6.3p15 in this vocabulary.
* **It is strict**, with the measured glibc witness `const char *__restrict`,
  which was unreadable and is now `ptr<const, u8>`; and the pointee's
  constness is kept, so it does not become `ptr<mut, u8>`.
* **Exactly one position is widened**: a qualifier *between* two `*` still
  refuses, and a declarator with no `*` is read exactly as before.
* **It is idempotent**, so reading twice decides nothing new.

## What this is not

A model of a **decision**, as its two neighbours are. The word list is the
declarator the reader extracted from the token stream (after the parameter name
and any array suffix are handled); that it is the parameter C declares is the
faithful-extraction premise of trust boundary 28. Pointee readability is
modelled by `QCore.TypedefProjection` and abstracted here. The result declarator
is not widened: 6.7.6.3p15 speaks of parameters, and the Rust applies the step
to parameters only.
-/

namespace QCore.PointerQualifierReader

/-- One word of a parameter declarator. `const` is kept apart from the other
qualifiers (`volatile`, `restrict`, `__restrict`, `__restrict__`) because it is
the one whose position before the stars the projection reads. -/
inductive Word where
  | star
  | const
  | qualifier
  | specifier
  deriving DecidableEq, Repr

/-- The qualifier table `QUALIFIERS`. -/
def isQualifier : Word → Bool
  | .const => true
  | .qualifier => true
  | _ => false

/-- Whether a word list contains a `*`. -/
def hasStar : List Word → Bool
  | [] => false
  | w :: ws => w == .star || hasStar ws

/-- Whether a word list contains `const`. -/
def hasConst : List Word → Bool
  | [] => false
  | w :: ws => w == .const || hasConst ws

/-! A declarator is modelled **reversed**, last word first, because every rule
here reads it from its end. -/

/-- Split off the trailing stars: their number, and the rest (still reversed). -/
def countStars : List Word → Nat × List Word
  | .star :: rest => ((countStars rest).1 + 1, (countStars rest).2)
  | ws => (0, ws)

/-- The earlier reader, `project_declarator`: pointer depth and whether the
pointee is `const`, or `none` when a `*` remains in the head. Depth `0` is a
scalar, whose own reading `project_scalar` owns and this model abstracts. -/
def project (rev : List Word) : Option (Nat × Bool) :=
  let split := countStars rev
  if hasStar split.2 then none
  else if split.1 = 0 then some (0, false)
  else some (split.1, hasConst split.2)

/-- Drop leading (that is, trailing-in-C) qualifiers. -/
def stripQualifiers : List Word → List Word
  | [] => []
  | w :: ws => if isQualifier w then stripQualifiers ws else w :: ws

/-- `drop_parameter_object_qualifiers`: drop the qualifiers after the last `*`,
and only when a `*` is what they follow. -/
def dropObjectQualifiers (rev : List Word) : List Word :=
  match stripQualifiers rev with
  | .star :: rest => .star :: rest
  | _ => rev

/-- The reader after this card, for a parameter. -/
def projectParameter (rev : List Word) : Option (Nat × Bool) :=
  project (dropObjectQualifiers rev)

/-! ### Lemmas -/

theorem stripQualifiers_of_not_qualifier {w : Word} (h : isQualifier w = false)
    (ws : List Word) : stripQualifiers (w :: ws) = w :: ws := by
  simp [stripQualifiers, h]

theorem hasStar_of_strip_star {rev rest : List Word}
    (h : stripQualifiers rev = .star :: rest) : hasStar rev = true := by
  induction rev with
  | nil => simp [stripQualifiers] at h
  | cons w ws ih =>
      cases w <;> simp_all [stripQualifiers, isQualifier, hasStar]

theorem strip_qualifiers_then_star (qs rest : List Word)
    (hq : ∀ w ∈ qs, isQualifier w = true) :
    stripQualifiers (qs ++ .star :: rest) = .star :: rest := by
  induction qs with
  | nil => simp [stripQualifiers, isQualifier]
  | cons q qs ih =>
      have hq1 : isQualifier q = true := hq q (by simp)
      have hrest : ∀ w ∈ qs, isQualifier w = true := fun w hw => hq w (by simp [hw])
      simp [stripQualifiers, hq1, ih hrest]

theorem strip_of_no_star {rev : List Word} (h : hasStar rev = false) :
    ∀ rest, stripQualifiers rev ≠ .star :: rest := by
  intro rest heq
  have := hasStar_of_strip_star heq
  simp_all

/-! ### The widening only adds -/

/-- Pointwise: every parameter the earlier reader projected is projected
identically. -/
theorem the_widening_only_adds (rev : List Word) {x : Nat × Bool}
    (h : project rev = some x) : projectParameter rev = some x := by
  unfold projectParameter dropObjectQualifiers
  cases rev with
  | nil => simp_all [stripQualifiers]
  | cons w ws =>
      cases hw : isQualifier w with
      | false =>
          rw [stripQualifiers_of_not_qualifier hw]
          cases w <;> simp_all
      | true =>
          -- A declarator ending in a qualifier has no trailing star, so the
          -- earlier reader projected it only when it contains no `*` at all.
          have hstar : countStars (w :: ws) = (0, w :: ws) := by
            cases w <;> simp_all [isQualifier, countStars]
          have hno : hasStar (w :: ws) = false := by
            unfold project at h
            rw [hstar] at h
            cases hs : hasStar (w :: ws) <;> simp_all
          cases hsq : stripQualifiers (w :: ws) with
          | nil => simp_all
          | cons v vs =>
              cases v with
              | star => exact absurd hsq (strip_of_no_star hno vs)
              | const => simp_all
              | qualifier => simp_all
              | specifier => simp_all

/-- No reading is lost: a declarator the widened reader refuses was refused
before too. -/
theorem the_widening_loses_no_reading (rev : List Word)
    (h : projectParameter rev = none) : project rev = none := by
  cases hp : project rev with
  | none => rfl
  | some x =>
      have := the_widening_only_adds rev hp
      simp_all

/-! ### It reads the unqualified type -/

/-- C11 6.7.6.3p15 in this vocabulary: a run of qualifiers after the last `*`
is read exactly as its absence. -/
theorem the_widening_reads_the_unqualified_type (qs rest : List Word)
    (hq : ∀ w ∈ qs, isQualifier w = true) :
    projectParameter (qs ++ .star :: rest) = project (.star :: rest) := by
  unfold projectParameter dropObjectQualifiers
  rw [strip_qualifiers_then_star qs rest hq]

/-- A declarator with no `*` is read exactly as before. -/
theorem a_declarator_without_a_star_is_unchanged (rev : List Word)
    (h : hasStar rev = false) : projectParameter rev = project rev := by
  unfold projectParameter dropObjectQualifiers
  cases hs : stripQualifiers rev with
  | nil => rfl
  | cons v vs =>
      cases v with
      | star => exact absurd hs (strip_of_no_star h vs)
      | const => rfl
      | qualifier => rfl
      | specifier => rfl

/-- Reading twice decides nothing new. -/
theorem drop_is_idempotent (rev : List Word) :
    dropObjectQualifiers (dropObjectQualifiers rev) = dropObjectQualifiers rev := by
  unfold dropObjectQualifiers
  cases hs : stripQualifiers rev with
  | nil => simp [hs]
  | cons v vs =>
      cases v with
      | star => simp [stripQualifiers, isQualifier]
      | const => simp [hs]
      | qualifier => simp [hs]
      | specifier => simp [hs]

/-! ### Exactly one position is widened -/

/-- A qualifier *between* two `*` qualifies an inner pointer, which this
vocabulary cannot spell: still refused, whatever follows the last `*`. -/
theorem a_qualifier_between_two_stars_still_refuses (qs rest : List Word) {q : Word}
    (hqs : ∀ w ∈ qs, isQualifier w = true) (hq : isQualifier q = true)
    (hrest : hasStar rest = true) :
    projectParameter (qs ++ .star :: q :: rest) = none := by
  rw [the_widening_reads_the_unqualified_type qs (q :: rest) hqs]
  have hc : countStars (.star :: q :: rest) = (1, q :: rest) := by
    cases q <;> simp_all [isQualifier, countStars]
  unfold project
  rw [hc]
  simp [hasStar, hrest]

/-! ### Witnesses -/

/-- glibc's `const char *__restrict __format`, name dropped, reversed. -/
def glibcFormat : List Word := [.qualifier, .star, .specifier, .const]

/-- It was unreadable: the measured `abi-unreadable-declaration`. -/
theorem the_glibc_format_parameter_was_unreadable : project glibcFormat = none := by
  decide

/-- And it is now `ptr<const, u8>`: one level over a `const` pointee. -/
theorem the_glibc_format_parameter_is_now_read :
    projectParameter glibcFormat = some (1, true) := by
  decide

/-- The widening is strict. -/
theorem the_widening_is_strict :
    ∃ rev, project rev = none ∧ projectParameter rev ≠ none :=
  ⟨glibcFormat, by decide, by decide⟩

/-- The pointee's constness survives: dropping the object's qualifier does not
turn `const char *__restrict` into `ptr<mut, u8>`. -/
theorem the_pointee_constness_is_kept :
    projectParameter glibcFormat ≠ some (1, false) := by
  decide

/-- `void *const p`: a constant pointer to mutable bytes is `ptr<mut, u8>`. -/
theorem a_const_pointer_to_mutable_bytes_is_mutable :
    projectParameter [.const, .star, .specifier] = some (1, false) := by
  decide

/-- `void *__restrict *__restrict p`: refused, as the Rust table pins. -/
theorem a_restrict_between_two_stars_refuses :
    projectParameter [.qualifier, .star, .qualifier, .star, .specifier] = none := by
  decide

end QCore.PointerQualifierReader
