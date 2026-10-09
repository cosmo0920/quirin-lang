import QCore.DeepUnitAst

/-!
# Every Clang JSON transport and the nesting it reads (card DEEP-AST-TRANSPORTS-AND-WALL-TIME, rules 13q–13t)

The abstract rules implemented by
`crates/qcore-c-gateway/src/deep_ast_transports.rs` (the inventory and its
controls), `deep_unit_ast::values_from_slice` and
`crates/qcore-c-gateway/src/function_reading_reuse.rs`.

* **Rule 13q.** A transport of bound `band` reads a document as rule 13o reads
  the unit index (`QCore.DeepUnitAst.reads`): the default reading is kept, the
  band from 128 to 256 levels is added, nothing beyond. The filtered stream
  is read when its deepest value is: a stream is admitted exactly when every
  value of it is (`a_stream_is_read_iff_every_value_is`).
* **Rule 13r.** A transport of bound `default` reads only what the default
  parse reads. A transport of bound `ownStack` reads without a limit on its
  own stack (recorded, not changed).
* **Readers.** A reader whose stack use is at most `base + perLevel · d` at
  depth `d`, measured within `budget` at the bound, fits at every admitted
  depth (`fits_at_the_bound_fits_within_it`).
* **Rule 13s.** The per-function call table built by one walk answers every
  name exactly as the per-callee walk (`call_exits::resolve`) does
  (`the_table_is_the_walk`).
* **Rule 13t.** The line tree holds, at every node, the begin and end line
  the walk computed there, in the same child order, so a walk reading it is
  the walk computing them (`the_annotated_walk_is_the_walk`).

## What this is not

A proof that the measured stack figures hold for every input of a given
depth (they are measurements, bound to the tests), that `serde_json` reads
the same value with and without its limit (the library's contract, as in
`QCore.DeepUnitAst`), or that Clang's JSON is faithful (trust boundary 28).
-/

namespace QCore.DeepAstTransports

open QCore.DeepUnitAst (defaultLimit admittedDepth defaultReads reads)

/-- How a transport's parse treats nesting. -/
inductive Bound where
  | band
  | default
  | ownStack
  deriving DecidableEq, Repr

/-- Shared with `deep_ast_transports::TRANSPORTS` (sites and bounds). -/
def transports : List (String × Bound) := [
  ("typed_access_path::AstIndex::from_json", .band),
  ("typed_c_ast::unit_declarations", .band),
  ("typed_c_ast::translation_unit_ast", .band),
  ("typed_c_ast::ast_declarations", .default),
  ("c_numeric_obligations::typed_ast", .default),
  ("ffi_layout::read_struct_layout", .default),
  ("typed_dispatch_census::unit_facts", .ownStack),
  ("index_extent::analyze_bounds_preprocessed", .ownStack)
]

/-- Whether a transport of bound `b` reads a well-formed document of depth `d`. -/
def readsUnder : Bound → Nat → Bool
  | .band, d => reads d
  | .default, d => defaultReads d
  | .ownStack, _ => true

/-- Rule 13o's reading, restated: within the bound and nothing beyond. -/
theorem reads_iff_within_the_bound (d : Nat) : reads d = true ↔ d ≤ admittedDepth := by
  unfold reads defaultReads admittedDepth defaultLimit
  constructor
  · intro h
    simp only [Bool.or_eq_true, decide_eq_true_eq] at h
    omega
  · intro h
    simp only [Bool.or_eq_true, decide_eq_true_eq]
    omega

/-- Rule 13q never removes a reading the default parse made. -/
theorem the_band_keeps_every_default_reading {d : Nat} (h : defaultReads d = true) :
    readsUnder .band d = true := by
  simp [readsUnder, reads, h]

/-- Rules 13q, 13r: a transport with a bound reads nothing beyond 256 levels. -/
theorem nothing_beyond_the_bound {b : Bound} {d : Nat} (hb : b ≠ .ownStack)
    (beyond : admittedDepth < d) : readsUnder b d = false := by
  cases b with
  | band =>
    have : ¬ reads d = true := fun h => by
      have := (reads_iff_within_the_bound d).mp h; omega
    simpa [readsUnder] using this
  | default =>
    have nd : ¬ d < defaultLimit := by
      unfold admittedDepth at beyond; unfold defaultLimit at *; omega
    simp [readsUnder, defaultReads, nd]
  | ownStack => exact absurd rfl hb

/-- Rule 13r: a default transport keeps the default reading exactly. -/
theorem a_default_transport_is_unchanged (d : Nat) :
    readsUnder .default d = defaultReads d := rfl

/-- Exactly three transports read the band, and they are the unit index, the
analysed-unit consultations and the closed world. -/
theorem the_band_transports :
    (transports.filter (fun t => t.2 == .band)).map Prod.fst =
      ["typed_access_path::AstIndex::from_json", "typed_c_ast::unit_declarations",
       "typed_c_ast::translation_unit_ast"] := by
  decide

/-- The depth of a filtered stream: its deepest value's. -/
def streamDepth (depths : List Nat) : Nat := depths.foldr max 0

theorem streamDepth_le_iff (depths : List Nat) (n : Nat) :
    streamDepth depths ≤ n ↔ ∀ d ∈ depths, d ≤ n := by
  induction depths with
  | nil => simp [streamDepth]
  | cons d rest ih =>
    simp only [streamDepth, List.foldr_cons] at *
    constructor
    · intro h e he
      rcases List.mem_cons.mp he with rfl | he
      · omega
      · exact ih.mp (by omega) e he
    · intro h
      have h1 := h d (List.mem_cons_self ..)
      have h2 := ih.mpr (fun e he => h e (List.mem_cons_of_mem _ he))
      omega

/-- Rule 13q on the filtered stream: it is read exactly when every value of it
is within the band. -/
theorem a_stream_is_read_iff_every_value_is (depths : List Nat) :
    reads (streamDepth depths) = true ↔ ∀ d ∈ depths, reads d = true := by
  rw [reads_iff_within_the_bound, streamDepth_le_iff]
  simp only [reads_iff_within_the_bound]

/-- A reader's stack use at depth `d`, linear in the depth. -/
def fits (base perLevel budget d : Nat) : Prop := base + perLevel * d ≤ budget

/-- A reader measured within its budget at the bound fits at every admitted
depth. -/
theorem fits_at_the_bound_fits_within_it {base perLevel budget d : Nat}
    (atBound : fits base perLevel budget admittedDepth) (within : d ≤ admittedDepth) :
    fits base perLevel budget d := by
  unfold fits at *
  have : perLevel * d ≤ perLevel * admittedDepth := Nat.mul_le_mul_left _ within
  omega

/-! ## Rule 13s: the call table -/

/-- What one node of a body walk says, in `call_exits::resolve`'s terms: an
object declaration of a name, a `noreturn` function declaration (by id), or a
reference to a name's declaration (`d`, abstract). -/
inductive Event (D : Type) where
  | object (name : String)
  | noReturn (id : String)
  | reference (name : String) (declaration : D)

/-- One name's facts: whether an object declares it, and its references in
walk order. -/
structure Facts (D : Type) where
  shadowed : Bool
  references : List D

/-- The per-callee walk: what `resolve` gathers for `callee`. -/
def gather {D : Type} (callee : String) : List (Event D) → Facts D
  | [] => ⟨false, []⟩
  | .object n :: rest =>
    let f := gather callee rest
    ⟨(n == callee) || f.shadowed, f.references⟩
  | .noReturn _ :: rest => gather callee rest
  | .reference n d :: rest =>
    let f := gather callee rest
    ⟨f.shadowed, if n == callee then d :: f.references else f.references⟩

/-- The table: every name's facts from one pass over the walk (a finite map,
modelled as a function), as `function_reading_reuse::call_resolutions`
builds it. -/
def table {D : Type} : List (Event D) → String → Facts D
  | [] => fun _ => ⟨false, []⟩
  | .object n :: rest => fun name =>
    let f := table rest name
    if n == name then ⟨true, f.references⟩ else f
  | .noReturn _ :: rest => table rest
  | .reference n d :: rest => fun name =>
    let f := table rest name
    if n == name then ⟨f.shadowed, d :: f.references⟩ else f

/-- Rule 13s: the table answers every name as the per-callee walk does. -/
theorem the_table_is_the_walk {D : Type} (events : List (Event D)) (name : String) :
    table events name = gather name events := by
  induction events with
  | nil => rfl
  | cons e rest ih =>
    cases e with
    | object n =>
      simp only [table, gather, ih]
      by_cases h : n = name
      · subst h; simp
      · have hb : (n == name) = false := beq_false_of_ne h
        cases gather name rest
        simp [hb]
    | noReturn _ => simp only [table, gather, ih]
    | reference n d =>
      simp only [table, gather, ih]
      by_cases h : n = name
      · subst h; simp
      · have hb : (n == name) = false := beq_false_of_ne h
        cases gather name rest
        simp [hb]

/-- The resolution of one name from its facts and a decision on its
references (`call_exits::resolution_of`, abstract): unresolved when shadowed
or unreferenced, as both readers answer. -/
def resolution {D R : Type} (unresolved : R) (decide : List D → R) (f : Facts D) : R :=
  if f.shadowed || f.references.isEmpty then unresolved else decide f.references

theorem the_table_resolves_as_the_walk {D R : Type} (unresolved : R) (decide : List D → R)
    (events : List (Event D)) (name : String) :
    resolution unresolved decide (table events name) =
      resolution unresolved decide (gather name events) := by
  rw [the_table_is_the_walk]

/-! ## Rule 13t: the line tree -/

/-- A tree of nodes with children in order. -/
inductive Tree (α : Type) where
  | node (value : α) (children : List (Tree α))

/-- The walk of `elaboration_node`, abstract: `info` gives a node's lines,
`prune` drops a subtree, `pick` takes a node (and stops below it); otherwise
the children are walked in order. -/
def walk {α I : Type} (info : α → I) (prune pick : α → I → Bool) : Tree α → List α
  | .node a cs =>
    if prune a (info a) then []
    else if pick a (info a) then [a]
    else walkList info prune pick cs
where
  walkList (info : α → I) (prune pick : α → I → Bool) : List (Tree α) → List α
    | [] => []
    | c :: cs => walk info prune pick c ++ walkList info prune pick cs

/-- The line tree: each node paired with its lines, built once. -/
def annotate {α I : Type} (info : α → I) : Tree α → Tree (α × I)
  | .node a cs => .node (a, info a) (annotateList info cs)
where
  annotateList (info : α → I) : List (Tree α) → List (Tree (α × I))
    | [] => []
    | c :: cs => annotate info c :: annotateList info cs

/-- The walk that reads the lines stored in the tree. -/
def walkStored {α I : Type} (prune pick : α → I → Bool) (t : Tree (α × I)) : List α :=
  (walk (fun p : α × I => p.2) (fun p i => prune p.1 i) (fun p i => pick p.1 i) t).map Prod.fst

mutual
theorem annotated_walk {α I : Type} (info : α → I) (prune pick : α → I → Bool) :
    (t : Tree α) →
      (walk (fun p : α × I => p.2) (fun p i => prune p.1 i) (fun p i => pick p.1 i)
        (annotate info t)).map Prod.fst = walk info prune pick t
  | .node a cs => by
    simp only [annotate, walk]
    by_cases hp : prune a (info a) <;> by_cases hk : pick a (info a) <;>
      simp [hp, hk, annotated_walkList info prune pick cs]
theorem annotated_walkList {α I : Type} (info : α → I) (prune pick : α → I → Bool) :
    (cs : List (Tree α)) →
      (walk.walkList (fun p : α × I => p.2) (fun p i => prune p.1 i) (fun p i => pick p.1 i)
        (annotate.annotateList info cs)).map Prod.fst = walk.walkList info prune pick cs
  | [] => by simp [annotate.annotateList, walk.walkList]
  | c :: cs => by
    simp only [annotate.annotateList, walk.walkList, List.map_append,
      annotated_walk info prune pick c, annotated_walkList info prune pick cs]
end

/-- Rule 13t: a walk reading the line tree is the walk computing the lines. -/
theorem the_annotated_walk_is_the_walk {α I : Type} (info : α → I) (prune pick : α → I → Bool)
    (t : Tree α) : walkStored prune pick (annotate info t) = walk info prune pick t :=
  annotated_walk info prune pick t

end QCore.DeepAstTransports
