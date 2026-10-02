/-!
# A refused null test never establishes a leak (card MONKEY-VHOST-PROPERTY-CUSTODY)

The abstract rule implemented by `crates/qcore-c-gateway/src/refused_null_test.rs`
and its hook in `legacy_c::refine_pointer_null`, gateway rule 12e.

A leak accusation at an exit claims that an identity is live there, so on a
path where it may be NULL it needs that identity's non-null-ness established.
A null test establishes it on its non-null edge and refutes the identity on
its null edge, but only when the test is decided (the tested binding is one
live identity). Rule 10b refuses any other test: the binding is narrowed and
every resource kept. The null edge of a refused test therefore establishes
nothing about a nullable identity the binding may denote, so an accusation
there would rest on a fact no step established.

The rule withholds exactly those identities on exactly that edge
(`nullEdgeStep`). The model below follows one nullable identity through a
binding join and a null test (Monkey's `mk_http_error_page` shape) and reads
the verdict of the null edge's exit.

## What this is not

A model of the decision and of one path shape. That the flow's denotation,
nullable set and status bits are what the Rust reads, and the correspondence
with the compiled Rust, are review premises (trust boundary 28), pinned by
`crates/qcore-c-gateway/tests/unit/refused_null_test.rs` and the fixtures in
`crates/qcore-c-gateway/tests/monkey_vhost_property_custody.rs`.
-/

namespace QCore.RefusedNullTest

inductive NullEdgeStep where
  | keep
  | withhold
  deriving DecidableEq, Repr

/-- The decision for one site on the null edge of a refused null test. -/
def nullEdgeStep (denoted nullable possiblyLive : Bool) : NullEdgeStep :=
  if denoted && nullable && possiblyLive then .withhold else .keep

/-- Exactly a denoted, nullable, possibly-live site is withheld. -/
theorem withhold_iff (d n l : Bool) :
    nullEdgeStep d n l = .withhold ↔ d = true ∧ n = true ∧ l = true := by
  cases d <;> cases n <;> cases l <;> simp [nullEdgeStep]

/-- A site the tested binding cannot denote keeps its accusation. -/
theorem an_undenoted_site_is_kept (n l : Bool) : nullEdgeStep false n l = .keep := by
  cases n <;> cases l <;> rfl

/-- A non-nullable site keeps its accusation: its non-null-ness does not come
from the test. -/
theorem a_non_nullable_site_is_kept (d l : Bool) : nullEdgeStep d false l = .keep := by
  cases d <;> cases l <;> rfl

abbrev NullEdgeVector := Bool × Bool × Bool × NullEdgeStep

def nullEdgeVectors : List NullEdgeVector := [
  (false, false, false, .keep),
  (false, false, true, .keep),
  (false, true, false, .keep),
  (false, true, true, .keep),
  (true, false, false, .keep),
  (true, false, true, .keep),
  (true, true, false, .keep),
  (true, true, true, .withhold)
]

theorem null_edge_vectors_hold :
    nullEdgeVectors.all (fun v => nullEdgeStep v.1 v.2.1 v.2.2.1 == v.2.2.2) = true := by
  decide

/-! ## One nullable identity through a join and a null test -/

/-- What the flow knows of the nullable identity at the test. -/
structure Fact where
  /-- The identity may be live (it was acquired on some reaching path). -/
  live : Bool
  /-- The tested binding resolves to exactly this one identity. -/
  decided : Bool
  /-- An earlier step withheld it (an unresolved effect). -/
  withheld : Bool
  deriving DecidableEq, Repr

inductive Verdict where
  | safe
  | provisionalUnsafe
  | unknown
  deriving DecidableEq, Repr

/-- The null edge of `if (!binding) return;`. A decided test refutes the
identity (NULL there); a refused test keeps it (rule 10b) and, under rule
12e, withholds it; `rule12e = false` is the reading before the card. -/
def nullEdge (rule12e : Bool) (f : Fact) : Fact :=
  if f.decided then { f with live := false }
  else if rule12e && f.live then { f with withheld := true }
  else f

/-- The exit's verdict for the identity: a live, unwithheld identity is a
leak; a refused test on the path makes the rest `Unknown`. -/
def exitVerdict (refusedOnPath : Bool) (f : Fact) : Verdict :=
  if f.live && !f.withheld then .provisionalUnsafe
  else if refusedOnPath then .unknown
  else .safe

/-- `mk_http_error_page`: the copy may be live, the binding joined with a
refused statement's value, the test refused. -/
def joined : Fact := { live := true, decided := false, withheld := false }

/-- Non-vacuity: before the rule the null edge accused the copy. -/
theorem the_old_reading_accused_the_null_edge :
    exitVerdict true (nullEdge false joined) = .provisionalUnsafe := by
  decide

/-- Under the rule the null edge is `Unknown`: neither `Unsafe` nor `Safe`. -/
theorem the_null_edge_is_unknown :
    exitVerdict true (nullEdge true joined) = .unknown := by
  decide

theorem the_null_edge_is_never_safe (f : Fact) :
    exitVerdict true (nullEdge true f) ≠ .safe := by
  rcases f with ⟨l, d, w⟩
  cases l <;> cases d <;> cases w <;> decide

/-- A decided test is unchanged by the rule: it refutes the identity. -/
theorem a_decided_test_is_unchanged (f : Fact) (h : f.decided = true) :
    nullEdge true f = nullEdge false f := by
  simp [nullEdge, h]

/-- The rule only withholds: it never makes a withheld or dead identity live,
and never turns an `Unknown` or `Safe` into an accusation. -/
theorem the_rule_never_adds_an_accusation (refused : Bool) (f : Fact)
    (h : exitVerdict refused (nullEdge true f) = .provisionalUnsafe) :
    exitVerdict refused (nullEdge false f) = .provisionalUnsafe := by
  rcases f with ⟨l, d, w⟩
  cases l <;> cases d <;> cases w <;> cases refused <;> simp_all [nullEdge, exitVerdict]

/-- A refused test establishes no leak: on its null edge a live identity is
never accused. -/
theorem a_refused_test_establishes_no_leak (refused : Bool) (f : Fact)
    (h : f.decided = false) :
    exitVerdict refused (nullEdge true f) ≠ .provisionalUnsafe := by
  rcases f with ⟨l, d, w⟩
  cases l <;> cases w <;> cases refused <;> simp_all [nullEdge, exitVerdict]

end QCore.RefusedNullTest
