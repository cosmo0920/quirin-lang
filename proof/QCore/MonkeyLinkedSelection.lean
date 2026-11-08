/-! Card CURRENT-MONKEY-LINKED-CONFIGURATION: the `current` route's Monkey
units under `--monkey-linked`.

`linked_monkey` (`crates/qcore-audit/src/current_route.rs`) selects Monkey's
units from the run's own compile database, which is the configuration linked
into `fluent-bit`, and keys them under the label `monkey`. When the standalone
(forced-Libevent) Monkey database is also given, it is only accounted for.
Each of its units is **replaced** by the linked unit of the same file, or, if
the linked configuration does not compile that file, it stays in the
denominator as a **not-linked** `Unknown` unit under its old
`monkey-libevent` key. The standalone configuration is never analysed. In the
cross-unit export phase, a not-linked unit exports and defines nothing,
because the link cannot choose a definition from it.

What is proved here:

* **nothing vanishes**: every standalone file yields a row, either the linked
  unit that replaced it or its own not-linked row, and every linked file is
  analysed;
* **exactly one reading**: no file is both analysed and not-linked, and a
  file is not-linked exactly when it is standalone and not linked;
* **keys name one configuration**: when each database lists a file once, the
  run's unit keys are pairwise distinct (the two labels differ), which is
  what the link step's condition 11 requires;
* **a not-linked unit is never `Safe`**: its row is `Unknown`, and it never
  supplies a home or a definition to the definition census;
* **the census stays complete only on the analysed units**: a not-linked row
  never makes the census incomplete, and a skipped analysed row always does;
* **the accounting does not change the analysed set**: the analysed rows are
  exactly the linked units, with or without a standalone database;
* **vectors**: the card's test fixture and the user's census shape (28
  shared files, `tls_openssl.c` linked only, `tls_stub.c` standalone only),
  in miniature.

What is **not** proved here, and stays a Rust boundary (trust boundary 28):
that `is_monkey_unit` and the located-path selection pick exactly these files,
and that the compile database lists what the build compiled. These are pinned
by `a_linked_monkey_run_replaces_or_keeps_every_standalone_unit`
(`crates/qcore-audit/tests/current_cli.rs`) and
`route_units_key_linked_monkey_units_like_a_linked_run`
(`crates/qcore-audit/src/link_resolution.rs`). -/
namespace QCore.MonkeyLinkedSelection

/-- A checkout-relative Monkey file. -/
abbrev File := String

/-- The two Monkey configurations' report labels. -/
inductive Label where
  | linked
  | standalone
  deriving DecidableEq, Repr

/-- A unit key: `label/file`. -/
structure Key where
  label : Label
  file : File
  deriving DecidableEq, Repr

/-- A row of the run's denominator. -/
inductive Row where
  /-- Analysed under its own (linked) compile record. -/
  | analysed (key : Key)
  /-- A standalone unit the linked configuration does not compile. -/
  | notLinked (key : Key)
  deriving DecidableEq, Repr

def Row.key : Row → Key
  | .analysed key => key
  | .notLinked key => key

/-- The verdict a row may publish. An analysed row's verdict is the
analyser's; a not-linked row's is fixed. -/
inductive Verdict where
  | safe
  | pending
  | unknown
  | violation
  deriving DecidableEq, Repr

def notLinkedVerdict : Verdict := .unknown

/-- The route's accounting. The linked files are analysed under `monkey`.
Each standalone file not among them becomes a not-linked row under
`monkey-libevent`. A standalone file among them gets no row of its own,
because the linked unit replaces it. -/
def account (linked standalone : List File) : List Row :=
  linked.map (fun f => Row.analysed ⟨.linked, f⟩) ++
    (standalone.filter (fun f => !linked.contains f)).map
      (fun f => Row.notLinked ⟨.standalone, f⟩)

/-- Without a standalone database: only the linked units. -/
theorem without_a_standalone_database_only_linked_units (linked : List File) :
    account linked [] = linked.map (fun f => Row.analysed ⟨.linked, f⟩) := by
  simp [account]

theorem every_linked_unit_is_analysed {linked standalone : List File} {f : File}
    (h : f ∈ linked) : Row.analysed ⟨.linked, f⟩ ∈ account linked standalone := by
  simp only [account, List.mem_append, List.mem_map]
  exact Or.inl ⟨f, h, rfl⟩

/-- Nothing vanishes: a standalone file is either replaced by the linked unit
of the same file or kept as its own not-linked row. -/
theorem every_standalone_unit_is_accounted {linked standalone : List File} {f : File}
    (h : f ∈ standalone) :
    (f ∈ linked ∧ Row.analysed ⟨.linked, f⟩ ∈ account linked standalone) ∨
      (f ∉ linked ∧ Row.notLinked ⟨.standalone, f⟩ ∈ account linked standalone) := by
  by_cases hl : f ∈ linked
  · exact Or.inl ⟨hl, every_linked_unit_is_analysed hl⟩
  · refine Or.inr ⟨hl, ?_⟩
    simp only [account, List.mem_append, List.mem_map, List.mem_filter]
    refine Or.inr ⟨f, ⟨h, ?_⟩, rfl⟩
    simpa using hl

/-- A file is not-linked exactly when it is standalone and not linked. -/
theorem not_linked_iff {linked standalone : List File} {f : File} :
    Row.notLinked ⟨.standalone, f⟩ ∈ account linked standalone ↔
      f ∈ standalone ∧ f ∉ linked := by
  simp only [account, List.mem_append, List.mem_map, List.mem_filter]
  constructor
  · rintro (⟨g, _, hg⟩ | ⟨g, ⟨hs, hn⟩, hg⟩)
    · cases hg
    · cases hg
      exact ⟨hs, by simpa using hn⟩
  · rintro ⟨hs, hn⟩
    exact Or.inr ⟨f, ⟨hs, by simpa using hn⟩, rfl⟩

/-- Every row is a linked analysis or a standalone not-linked row. -/
theorem rows_carry_their_configuration {linked standalone : List File} {row : Row}
    (h : row ∈ account linked standalone) :
    (∃ f ∈ linked, row = .analysed ⟨.linked, f⟩) ∨
      (∃ f ∈ standalone, f ∉ linked ∧ row = .notLinked ⟨.standalone, f⟩) := by
  simp only [account, List.mem_append, List.mem_map, List.mem_filter] at h
  rcases h with ⟨f, hf, rfl⟩ | ⟨f, ⟨hs, hn⟩, rfl⟩
  · exact Or.inl ⟨f, hf, rfl⟩
  · exact Or.inr ⟨f, hs, by simpa using hn, rfl⟩

/-- Exactly one reading: no file is both analysed and not-linked. -/
theorem no_file_is_read_twice {linked standalone : List File} {f : File} :
    ¬ (Row.analysed ⟨.linked, f⟩ ∈ account linked standalone ∧
        Row.notLinked ⟨.standalone, f⟩ ∈ account linked standalone) := by
  rintro ⟨_, hn⟩
  have h := (not_linked_iff.mp hn).2
  rcases rows_carry_their_configuration
      (row := Row.analysed ⟨.linked, f⟩) (by assumption) with ⟨g, hg, he⟩ | ⟨_, _, _, he⟩
  · cases he
    exact h hg
  · cases he

/-- The standalone configuration is never analysed. -/
theorem no_standalone_unit_is_analysed {linked standalone : List File} {f : File} :
    Row.analysed ⟨.standalone, f⟩ ∉ account linked standalone := by
  intro h
  rcases rows_carry_their_configuration h with ⟨_, _, he⟩ | ⟨_, _, _, he⟩ <;> cases he

/-- The accounting does not change the analysed set. -/
theorem analysed_rows_are_exactly_the_linked_units (linked standalone : List File) (k : Key) :
    Row.analysed k ∈ account linked standalone ↔ Row.analysed k ∈ account linked [] := by
  simp only [account, List.mem_append, List.mem_map, List.mem_filter, List.filter_nil,
    List.map_nil, List.not_mem_nil, or_false]
  constructor
  · rintro (h | ⟨_, _, h⟩)
    · exact h
    · cases h
  · exact Or.inl

/-- The verdict a row publishes, given the analyser's verdicts. -/
def published (analyser : Key → Verdict) : Row → Verdict
  | .analysed key => analyser key
  | .notLinked _ => notLinkedVerdict

/-- A not-linked unit is never `Safe`, whatever the analyser says. -/
theorem a_not_linked_unit_is_unknown (analyser : Key → Verdict) (key : Key) :
    published analyser (.notLinked key) = .unknown := rfl

theorem a_not_linked_unit_is_never_safe (analyser : Key → Verdict) (key : Key) :
    published analyser (.notLinked key) ≠ .safe := by
  simp [published, notLinkedVerdict]

/-- The definition census of the cross-unit export phase. An analysed row
contributes its scanned definitions, or is incomplete when unscanned. A
not-linked row contributes nothing and is complete. -/
def censusEntry (scan : Key → Option (List String)) : Row → Option (List String)
  | .analysed key => scan key
  | .notLinked _ => some []

def censusComplete (scan : Key → Option (List String)) (rows : List Row) : Bool :=
  rows.all (fun row => (censusEntry scan row).isSome)

theorem a_not_linked_unit_defines_nothing (scan : Key → Option (List String)) (key : Key) :
    censusEntry scan (.notLinked key) = some [] := rfl

theorem a_not_linked_unit_never_makes_the_census_incomplete
    (scan : Key → Option (List String)) (rows : List Row) (key : Key) :
    censusComplete scan (rows ++ [.notLinked key]) = censusComplete scan rows := by
  simp [censusComplete, censusEntry]

theorem an_unscanned_analysed_unit_makes_the_census_incomplete
    (scan : Key → Option (List String)) (rows : List Row) (key : Key)
    (h : scan key = none) (hm : Row.analysed key ∈ rows) :
    censusComplete scan rows = false := by
  unfold censusComplete
  rw [List.all_eq_false]
  exact ⟨_, hm, by simp [censusEntry, h]⟩

/-- Keys name one configuration: with each database listing a file once,
the run's unit keys are pairwise distinct. -/
theorem keys_name_one_configuration {linked standalone : List File}
    (hl : linked.Nodup) (hs : standalone.Nodup) :
    ((account linked standalone).map Row.key).Nodup := by
  simp only [account, List.map_append, List.map_map]
  refine List.nodup_append.mpr ⟨?_, ?_, ?_⟩
  · exact hl.map _ (fun a b hab heq => hab (by
      simp only [Function.comp, Row.key, Key.mk.injEq] at heq
      exact heq.2))
  · exact (hs.filter _).map _ (fun a b hab heq => hab (by
      simp only [Function.comp, Row.key, Key.mk.injEq] at heq
      exact heq.2))
  · intro a ha b hb heq
    simp only [List.mem_map, Function.comp] at ha hb
    rcases ha with ⟨f, _, rfl⟩
    rcases hb with ⟨g, _, rfl⟩
    simp [Row.key] at heq

/-! ## Vectors -/

/-- The card's test fixture: `mk_event.c` is in both databases,
`mk_fifo.c` and `tls_openssl.c` are linked only, and `tls_stub.c` is
standalone only. -/
def fixtureLinked : List File :=
  ["lib/monkey/mk_core/mk_event.c", "lib/monkey/mk_server/mk_fifo.c",
    "lib/monkey/tls/tls_openssl.c"]

def fixtureStandalone : List File :=
  ["lib/monkey/mk_core/mk_event.c", "lib/monkey/tls/tls_stub.c"]

theorem fixture_vector :
    account fixtureLinked fixtureStandalone =
      [.analysed ⟨.linked, "lib/monkey/mk_core/mk_event.c"⟩,
        .analysed ⟨.linked, "lib/monkey/mk_server/mk_fifo.c"⟩,
        .analysed ⟨.linked, "lib/monkey/tls/tls_openssl.c"⟩,
        .notLinked ⟨.standalone, "lib/monkey/tls/tls_stub.c"⟩] := by
  decide

/-- The user's census in miniature: a shared file, the linked-only TLS
backend and the standalone-only stub. The denominator keeps three rows for
the three files, and only the stub is not-linked. -/
theorem census_shape_vector :
    account ["mk_core/mk_event.c", "tls/tls_openssl.c"]
        ["mk_core/mk_event.c", "tls/tls_stub.c"] =
      [.analysed ⟨.linked, "mk_core/mk_event.c"⟩,
        .analysed ⟨.linked, "tls/tls_openssl.c"⟩,
        .notLinked ⟨.standalone, "tls/tls_stub.c"⟩] := by
  decide

/-- Non-vacuity: a not-linked row exists, and the analysed rows can be any
verdict while it stays `Unknown`. -/
theorem not_linked_is_inhabited :
    ∃ row ∈ account fixtureLinked fixtureStandalone, ∃ key, row = .notLinked key :=
  ⟨.notLinked ⟨.standalone, "lib/monkey/tls/tls_stub.c"⟩, by rw [fixture_vector]; simp,
    ⟨.standalone, "lib/monkey/tls/tls_stub.c"⟩, rfl⟩

end QCore.MonkeyLinkedSelection
