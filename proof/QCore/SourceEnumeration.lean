/-! Unit `E01-F1`: the main-source function *denominator*.

This models the enumeration relation of
`qcore_c_gateway::legacy_c::enumeration` — which function definitions of a
translation unit are counted, which of them any implemented analysis reads, and
when the inventory refuses to claim it is complete.

`docs/reports/remaining-e01/README.md` boundary **E01-B1** recorded that two
real definition shapes escaped the audited inventory entirely — a definition
produced by macro expansion, and a K&R definition — so a unit's reported
"N checked / M Unknown" split was not a complete function denominator. A third
cause was found while implementing the repair and is described in
`docs/reports/remaining-e01-f1/README.md` §2.2: the raw token scan can lose a
definition that *is* spelled in the main source. `absentFromRawScan` below is
named for what it means — the raw scan did not see it — and does not
distinguish the causes.

What is proved here:

* **monotonicity** — the new enumeration is a superset of the historical raw
  token scan, so no unit can gain a verdict by losing a row, and the growth is
  non-vacuous (witnesses where it grows strictly);
* **honest accounting of the growth** — the denominator is exactly the analysed
  names plus the unanalyzable ones, the two classes are disjoint, and an
  `absentFromRawScan` row is one the raw scan never had;
* **fail-closed unit `Safe`** — a whole-unit `Safe` claim survives only an
  established definition set with nothing unread in it, in both directions;
* **refusal never manufactures a verdict** — every completeness refusal yields
  `Unknown`, never `Safe` and never a provisional `Unsafe`, and the two
  environment grounds never carry a category whose subject is the analysed
  source (§L21.1 rule 1);
* **the file-scope grammar decides the adversarial cases** — a prototype, an
  old-style declaration, a K&R definition, a pointer declarator list, an
  unreadable declarator, an unbalanced construct, an expression context, a
  function typedef and a declaration specifier in the name position each land
  where the scanner says they do.

What is **not** proved here, and stays a Rust boundary: that Clang's line
markers attribute expanded tokens to the invoking file (an observation of the
tool, pinned behaviourally by
`crates/qcore-c-gateway/tests/e01_source_enumeration.rs`), that the gateway's
tokenizer produces the token lists transcribed below, and that the analyzer's
own body finder reads exactly the prototype-shaped definitions. The corpora
below are transcriptions of that test matrix, not extractions from it. -/
namespace QCore.SourceEnumeration

/-- A definition name. -/
abbrev Name := String

/-- Why an inventory does not claim to be the complete definition set.
Mirrors `EnumerationRefusal`. -/
inductive Refusal where
  /-- The line-marked preprocessing did not run. -/
  | expansionUnavailable
  /-- The line-marked output does not name its primary input. -/
  | primaryInputUnidentified
  /-- A file-scope delimiter has no match. -/
  | unbalancedDelimiters
  /-- A file-scope construct is neither a definition nor a declaration this
  scanner reads. -/
  | unclassifiedFileScopeConstruct
  deriving DecidableEq, Repr

/-- What the file-scope scanner decides a `name ( … )` construct is.
Mirrors `FileScopeConstruct`. -/
inductive Construct where
  /-- `f(…) { … }`: the shape the analyzer's body finder reads. -/
  | prototypeDefinition
  /-- `f(a, b) int a; … { … }`: counted, read by nothing. -/
  | identifierListDefinition
  /-- A declaration, a function-pointer declarator, an attribute. -/
  | notADefinition
  /-- Boundaries could not be established here. -/
  | unclassified (ground : Refusal)
  deriving DecidableEq, Repr

/-- The §L21.1 subject of a reason row, as `ReasonSubject`. -/
inductive Subject where
  | analysedSource
  | analyser
  | environment
  deriving DecidableEq, Repr

/-- The two §L21.1 categories this unit emits, as `ReasonCategory`. -/
inductive Category where
  | unsupportedSyntax
  | environmentUnavailable
  deriving DecidableEq, Repr

def Category.subject : Category → Subject
  | .unsupportedSyntax => .analysedSource
  | .environmentUnavailable => .environment

/-- `EnumerationRefusal::category`. -/
def Refusal.category : Refusal → Category
  | .expansionUnavailable => .environmentUnavailable
  | .primaryInputUnidentified => .environmentUnavailable
  | .unbalancedDelimiters => .unsupportedSyntax
  | .unclassifiedFileScopeConstruct => .unsupportedSyntax

/-- Every refusal ground. -/
def Refusal.all : List Refusal :=
  [.expansionUnavailable, .primaryInputUnidentified, .unbalancedDelimiters,
   .unclassifiedFileScopeConstruct]

/-! ## The file-scope grammar -/

/-- The gateway's declaration-specifier table, abridged to the keywords the
adversarial corpus below exercises. A parameter list containing any of these is
a prototype list, never a K&R identifier list. -/
def declarationSpecifiers : List String :=
  ["void", "char", "short", "int", "long", "float", "double", "signed",
   "unsigned", "struct", "union", "enum", "const", "volatile", "static"]

/-- Punctuation the gateway tokenizer emits as single-character tokens; every
other token in these corpora starts an identifier. -/
def punctuation : List String :=
  ["(", ")", "{", "}", ";", ",", "*", "[", "]", "=", "+", "-", "&"]

/-- `is_identifier`, over the finite token alphabet the corpora use. -/
def isIdentifier (token : String) : Bool :=
  !token.isEmpty && !punctuation.contains token

/-- The tail of `is_identifier_list`: `, ident` repeated to the end. -/
def identifierListTail : List String → Bool
  | [] => true
  | "," :: next :: rest =>
      isIdentifier next && !declarationSpecifiers.contains next &&
        identifierListTail rest
  | _ => false

/-- `is_identifier_list`: one or more plain identifiers separated by commas,
none of them a declaration specifier. -/
def isIdentifierList : List String → Bool
  | [] => false
  | first :: rest =>
      isIdentifier first && !declarationSpecifiers.contains first &&
        identifierListTail rest

/-- One K&R declaration: a nonempty run free of `(`, `)`, `{` and `}`
terminated by `;`. `none` refuses; `some rest` is what follows the `;`. -/
def takeDeclarationFrom : List String → Nat → Option (List String)
  | [], _ => none
  | ";" :: rest, taken => if taken = 0 then none else some rest
  | "(" :: _, _ => none
  | ")" :: _, _ => none
  | "{" :: _, _ => none
  | "}" :: _, _ => none
  | _ :: rest, taken => takeDeclarationFrom rest (taken + 1)

def takeDeclaration (tokens : List String) : Option (List String) :=
  takeDeclarationFrom tokens 0

/-- The K&R declaration-list loop, bounded by the input length. -/
def declarationList : Nat → List String → Construct
  | 0, _ => .unclassified .unclassifiedFileScopeConstruct
  | fuel + 1, tokens =>
      match takeDeclaration tokens with
      | none => .unclassified .unclassifiedFileScopeConstruct
      | some rest =>
          match rest with
          | "{" :: _ => .identifierListDefinition
          | [] => .unclassified .unclassifiedFileScopeConstruct
          | _ => declarationList fuel rest

/-- `starts_a_declaration`: whether the token immediately before a file-scope
name can end the declaration specifiers of a definition.

This gates the K&R arm only. An identifier list is indistinguishable from an
*expression* argument list — `sizeof(a), sizeof(b)` inside a file-scope
initialiser table has the same token shape as `f(a, b) int a; { … }` — so the
arm is entered only from a declaration context: nothing, an identifier or `*`
(the return type), or the `;`/`}` that ended the previous declaration. -/
def startsADeclaration : Option String → Bool
  | none => true
  | some ";" => true
  | some "}" => true
  | some "*" => true
  | some text => isIdentifier text

/-- `classify_file_scope`, given the token before the name, the tokens strictly
inside the parentheses and the tokens after the closing one. -/
def classify (name : String) (previous : Option String)
    (parameters after : List String) : Construct :=
  match after with
  | "{" :: _ => .prototypeDefinition
  | ";" :: _ => .notADefinition
  | [] => .notADefinition
  -- A *declarator suffix*, not a K&R declaration list: `typedef T (name)(args);`
  -- and a `T (name)(args)` function-pointer parameter both put a parenthesised
  -- name in front of another declarator, and a K&R declaration list can never
  -- begin with `(` or `[`.
  | "(" :: _ => .notADefinition
  | "[" :: _ => .notADefinition
  | _ =>
      if declarationSpecifiers.contains name then .notADefinition
      else if !startsADeclaration previous then .notADefinition
      else if isIdentifierList parameters then declarationList after.length after
      else .notADefinition

/-- Split a token list that begins at `(` into its contents and its tail.
`none` is an unbalanced construct, exactly as `matching` returning `None`. -/
def splitParenFrom : List String → Nat → List String →
    Option (List String × List String)
  | [], _, _ => none
  | ")" :: rest, 0, acc => some (acc.reverse, rest)
  | ")" :: rest, depth + 1, acc => splitParenFrom rest depth (")" :: acc)
  | "(" :: rest, depth, acc => splitParenFrom rest (depth + 1) ("(" :: acc)
  | token :: rest, depth, acc => splitParenFrom rest depth (token :: acc)

def splitParen (tokens : List String) : Option (List String × List String) :=
  match tokens with
  | "(" :: rest => splitParenFrom rest 0 []
  | _ => none

/-- The whole file-scope decision for `name`, preceded by `previous` and
followed by `tokens`. -/
def fileScopeNamed (name : String) (previous : Option String)
    (tokens : List String) : Construct :=
  match splitParen tokens with
  | none => .unclassified .unbalancedDelimiters
  | some (parameters, after) => classify name previous parameters after

/-- The decision for an ordinary function name in a declaration context, which
is where every definition in the corpus below sits. -/
def fileScopeAfter (previous : Option String) (tokens : List String) :
    Construct :=
  fileScopeNamed "subject" previous tokens

/-- The decision in the ordinary declaration context (a return type precedes
the name). -/
def fileScope (tokens : List String) : Construct :=
  fileScopeAfter (some "int") tokens

/-! ## The inventory -/

/-- The enumerated inventory of one unit, mirroring
`SourceDefinitionInventory`. -/
structure Inventory where
  definitions : List Name
  unanalyzable : List Name
  absentFromRawScan : List Name
  refusals : List Refusal
  deriving DecidableEq, Repr

/-- Duplicate-free view of a list, keeping the last occurrence. Models the
`BTreeSet` the Rust inventory collects into. -/
def dedup : List Name → List Name
  | [] => []
  | name :: rest =>
      if name ∈ dedup rest then dedup rest else name :: dedup rest

theorem mem_dedup {name : Name} : ∀ names : List Name,
    name ∈ dedup names ↔ name ∈ names := by
  intro names
  induction names with
  | nil => simp [dedup]
  | cons head rest ih =>
      by_cases h : head ∈ dedup rest
      · simp only [dedup, ite_eq_left h, List.mem_cons]
        constructor
        · exact fun member => Or.inr (ih.mp member)
        · rintro (rfl | member)
          · exact h
          · exact ih.mpr member
      · simp only [dedup, ite_eq_right h, List.mem_cons]
        exact or_congr Iff.rfl ih

/-- `enumerate_source_definitions`, over its three inputs: the historical raw
token scan of the unpreprocessed source, and the prototype-shaped and
identifier-list definitions the line-marked expansion attributes to the primary
input. -/
def build (raw prototype identifierList : List Name) (refusals : List Refusal) :
    Inventory :=
  { definitions :=
      dedup (raw ++ prototype) ++
        dedup (identifierList.filter
          (fun n => decide (n ∉ dedup (raw ++ prototype))))
    unanalyzable :=
      dedup (identifierList.filter
        (fun n => decide (n ∉ dedup (raw ++ prototype))))
    absentFromRawScan := dedup (prototype.filter (fun n => decide (n ∉ raw)))
    refusals := refusals }

/-- The set the unmodified analyzer reads: the enumerated names it also finds
as prototype-shaped definitions in the preprocessed unit. -/
def analysed (raw prototype : List Name) : List Name := dedup (raw ++ prototype)

/-- Fail-closed (`supports_unit_safe`): a whole-unit `Safe` claim needs an
established definition set with nothing unread in it. -/
def Inventory.supportsUnitSafe (inventory : Inventory) : Bool :=
  inventory.refusals.isEmpty && inventory.unanalyzable.isEmpty

/-- The verdict a completeness refusal yields. It is `Unknown`, never a
provisional `Unsafe`: an inventory the analysis could not establish is not a
validated reachable violation. -/
inductive Verdict where
  | unknown
  | provisionalUnsafe
  | certifiedSafe
  deriving DecidableEq, Repr

def refusalVerdict (_ : Refusal) : Verdict := .unknown

/-! ## Monotonicity: the denominator never shrinks -/

theorem raw_is_kept (raw prototype identifierList : List Name)
    (refusals : List Refusal) {name : Name} (member : name ∈ raw) :
    name ∈ (build raw prototype identifierList refusals).definitions := by
  simp only [build, List.mem_append]
  exact Or.inl ((mem_dedup _).mpr (List.mem_append_left _ member))

theorem prototype_is_kept (raw prototype identifierList : List Name)
    (refusals : List Refusal) {name : Name} (member : name ∈ prototype) :
    name ∈ (build raw prototype identifierList refusals).definitions := by
  simp only [build, List.mem_append]
  exact Or.inl ((mem_dedup _).mpr (List.mem_append_right _ member))

theorem identifier_list_is_kept (raw prototype identifierList : List Name)
    (refusals : List Refusal) {name : Name} (member : name ∈ identifierList) :
    name ∈ (build raw prototype identifierList refusals).definitions := by
  by_cases hanalysed : name ∈ analysed raw prototype
  · simp only [build, List.mem_append]
    exact Or.inl hanalysed
  · simp only [build, List.mem_append]
    refine Or.inr ((mem_dedup _).mpr ?_)
    simp only [List.mem_filter, decide_eq_true_eq]
    exact ⟨member, hanalysed⟩

theorem unanalyzable_is_counted (raw prototype identifierList : List Name)
    (refusals : List Refusal) {name : Name}
    (member : name ∈ (build raw prototype identifierList refusals).unanalyzable) :
    name ∈ (build raw prototype identifierList refusals).definitions := by
  simp only [build, List.mem_append] at *
  exact Or.inr member

/-- The denominator is exactly the analysed names plus the unanalyzable ones:
nothing is counted that is in neither class, and nothing in either class is
left out. -/
theorem the_denominator_is_exactly_its_two_classes
    (raw prototype identifierList : List Name) (refusals : List Refusal) :
    (build raw prototype identifierList refusals).definitions =
      analysed raw prototype ++
        (build raw prototype identifierList refusals).unanalyzable := rfl

/-- An unanalyzable definition is never also one the analyzer reads: the two
classes are disjoint by construction, so a row cannot be both read and unread. -/
theorem unanalyzable_is_not_analysed (raw prototype identifierList : List Name)
    (refusals : List Refusal) {name : Name}
    (member : name ∈ (build raw prototype identifierList refusals).unanalyzable) :
    name ∉ analysed raw prototype := by
  simp only [build] at member
  have := (mem_dedup _).mp member
  simp only [List.mem_filter, decide_eq_true_eq] at this
  exact this.2

/-- A macro-expanded definition is one the raw scan never had, so E01-B1's
growth is real growth and not a relabelling. -/
theorem absent_from_raw_scan_is_new (raw prototype identifierList : List Name)
    (refusals : List Refusal) {name : Name}
    (member : name ∈ (build raw prototype identifierList refusals).absentFromRawScan) :
    name ∉ raw := by
  simp only [build] at member
  have := (mem_dedup _).mp member
  simp only [List.mem_filter, decide_eq_true_eq] at this
  exact this.2

/-- A macro-expanded definition is nevertheless in the denominator and in the
analysed class: the unmodified analyzer reads it. -/
theorem absent_from_raw_scan_is_analysed (raw prototype identifierList : List Name)
    (refusals : List Refusal) {name : Name}
    (member : name ∈ (build raw prototype identifierList refusals).absentFromRawScan) :
    name ∈ analysed raw prototype := by
  simp only [build] at member
  have := (mem_dedup _).mp member
  simp only [List.mem_filter] at this
  exact (mem_dedup _).mpr (List.mem_append_right _ this.1)

/-! ## Fail-closed unit `Safe` -/

theorem a_refused_inventory_cannot_support_safe (inventory : Inventory)
    (refused : inventory.refusals ≠ []) :
    inventory.supportsUnitSafe = false := by
  cases h : inventory.refusals with
  | nil => exact absurd h refused
  | cons head tail => simp [Inventory.supportsUnitSafe, h]

theorem an_unread_definition_cannot_support_safe (inventory : Inventory)
    (unread : inventory.unanalyzable ≠ []) :
    inventory.supportsUnitSafe = false := by
  cases h : inventory.unanalyzable with
  | nil => exact absurd h unread
  | cons head tail => simp [Inventory.supportsUnitSafe, h]

/-- Both directions: `Safe` is admitted exactly when the set is established and
fully read. The rule adds no refusal beyond the two named ones. -/
theorem safe_is_admitted_exactly_when_established_and_read
    (inventory : Inventory) :
    inventory.supportsUnitSafe = true ↔
      (inventory.refusals = [] ∧ inventory.unanalyzable = []) := by
  cases hr : inventory.refusals <;> cases hu : inventory.unanalyzable <;>
    simp [Inventory.supportsUnitSafe, hr, hu]

/-! ## Refusals never manufacture a verdict -/

theorem a_refusal_never_yields_unsafe (refusal : Refusal) :
    refusalVerdict refusal ≠ Verdict.provisionalUnsafe := by
  cases refusal <;> simp [refusalVerdict]

theorem a_refusal_never_yields_safe (refusal : Refusal) :
    refusalVerdict refusal ≠ Verdict.certifiedSafe := by
  cases refusal <;> simp [refusalVerdict]

/-- §L21.1 rule 1 for this unit's vocabulary: the two grounds that name a tool
failure carry an environment category, whose subject is never the analysed
source. -/
theorem an_environment_ground_says_nothing_about_the_source :
    ∀ refusal ∈ [Refusal.expansionUnavailable,
        Refusal.primaryInputUnidentified],
      refusal.category.subject = Subject.environment := by decide

theorem a_token_stream_ground_names_the_analysed_source :
    ∀ refusal ∈ [Refusal.unbalancedDelimiters,
        Refusal.unclassifiedFileScopeConstruct],
      refusal.category.subject = Subject.analysedSource := by decide

theorem no_ground_is_both_environment_and_source :
    ∀ refusal ∈ Refusal.all,
      ¬(refusal.category.subject = Subject.environment ∧
        refusal.category.subject = Subject.analysedSource) := by decide

theorem the_refusal_vocabulary_is_complete (refusal : Refusal) :
    refusal ∈ Refusal.all := by cases refusal <;> decide

theorem both_subjects_are_reached :
    Refusal.all.map (fun refusal => refusal.category.subject) =
      [Subject.environment, Subject.environment, Subject.analysedSource,
       Subject.analysedSource] := by decide

/-! ## The file-scope grammar decides the adversarial corpus -/

/-- `int subject(void) { … }` — the historical shape, counted. -/
theorem a_prototype_definition_is_counted :
    fileScope ["(", "void", ")", "{", "return", "0", ";", "}"] =
      Construct.prototypeDefinition := by decide

/-- `int declared_only(void);` — a declaration, never counted. -/
theorem a_prototype_declaration_is_not_counted :
    fileScope ["(", "void", ")", ";"] = Construct.notADefinition := by decide

/-- `int old_style(a, b);` — an old-style declaration, never counted. Its
parameter list *is* an identifier list, so only the terminator separates it
from a definition; the scanner reads the terminator. -/
theorem an_old_style_declaration_is_not_counted :
    fileScope ["(", "a", ",", "b", ")", ";"] = Construct.notADefinition := by
  decide

/-- `int knr(a, b) int a; int b; { … }` — counted, and counted as unread. -/
theorem a_knr_definition_is_counted_as_unread :
    fileScope ["(", "a", ",", "b", ")", "int", "a", ";", "int", "b", ";",
      "{", "return", "a", ";", "}"] = Construct.identifierListDefinition := by
  decide

/-- `int knr(p) struct pair *p; { … }` — pointer declarators are read. -/
theorem a_pointer_declarator_list_is_read :
    fileScope ["(", "p", ")", "struct", "pair", "*", "p", ";", "{", "}"] =
      Construct.identifierListDefinition := by decide

/-- A specifier-bearing parameter list is never an identifier list, so a
prototype can never be mistaken for a K&R declarator. -/
theorem a_typed_parameter_list_is_never_an_identifier_list :
    isIdentifierList ["int", ",", "char"] = false := by decide

/-- `int hook(g) int (*g)(void); { … }` — a declarator list this scanner does
not read. It is neither counted nor dropped: the claim is refused. -/
theorem an_unreadable_declarator_list_refuses :
    fileScope ["(", "g", ")", "int", "(", "*", "g", ")", "(", "void", ")", ";",
      "{", "}"] =
      Construct.unclassified Refusal.unclassifiedFileScopeConstruct := by decide

/-- A parameter list whose parenthesis never closes cannot be classified. -/
theorem an_unbalanced_construct_refuses :
    fileScope ["(", "a", ",", "b"] =
      Construct.unclassified Refusal.unbalancedDelimiters := by decide

/-- A token run that does not open a parameter list is refused rather than
read as a definition. -/
theorem a_construct_that_does_not_open_a_parameter_list_refuses :
    fileScope ["=", "1", ";"] =
      Construct.unclassified Refusal.unbalancedDelimiters := by decide

/-- `… = { sizeof(a), sizeof(b) };` — an expression argument list has exactly
the K&R token shape, and would refuse the whole unit's completeness claim if the
scanner entered the K&R arm from an expression context. It does not: the
declaration-context guard reads it as the operand it is. -/
theorem an_expression_context_is_neither_counted_nor_ambiguous :
    fileScopeAfter (some "{") ["(", "a", ")", ",", "sizeof", "(", "b", ")",
      "}", ";"] = Construct.notADefinition := by decide

theorem a_comma_context_is_neither_counted_nor_ambiguous :
    fileScopeAfter (some ",") ["(", "b", ")", "}", ";"] =
      Construct.notADefinition := by decide

/-- The guard is not a blanket exemption: from a declaration context, a real
declarator run this scanner cannot read still refuses. -/
theorem the_guard_does_not_hide_a_real_unreadable_declarator :
    fileScopeAfter (some "int") ["(", "g", ")", "int", "(", "*", "g", ")",
      "(", "void", ")", ";", "{", "}"] =
      Construct.unclassified Refusal.unclassifiedFileScopeConstruct := by decide

/-- `typedef T (name)(args);` — a function typedef, measured in `librdkafka`'s
`rdkafka_admin.c`. A parenthesised name in front of another declarator is not a
definition, and a K&R declaration list can never begin with `(`. -/
theorem a_function_typedef_declarator_is_not_a_definition :
    fileScopeNamed "rd_kafka_resp_err_t" (some "typedef")
      ["(", "cb_t", ")", "(", "int", "a", ")", ";"] =
      Construct.notADefinition := by decide

/-- `unsigned int (hash)(const void *a)` as a parameter, measured in
`librdkafka`'s `rdkafka_partition.c`: the name position holds the declaration
specifier `int`, which is never a function name. -/
theorem a_specifier_in_the_name_position_is_not_a_definition :
    fileScopeNamed "int" (some "unsigned")
      ["(", "hash", ")", "(", "const", "void", "*", "a", ")", ")", "{", "}"] =
      Construct.notADefinition := by decide

/-- Every declaration-context predecessor the guard admits, and a
representative expression-context predecessor it refuses. -/
theorem the_declaration_contexts_are_exactly_these :
    ([none, some ";", some "}", some "*", some "int"].map startsADeclaration =
      [true, true, true, true, true]) ∧
    ([some ",", some "{", some "=", some "(", some "["].map startsADeclaration =
      [false, false, false, false, false]) := by decide

/-! ## Non-vacuity: the repair actually adds rows -/

/-- The E01-B1 witness: a unit whose only definition is macro-produced had an
*empty* historical denominator and now has one row, read by the unmodified
analyzer. -/
theorem the_macro_case_grows_the_denominator :
    build [] ["run"] [] [] =
      { definitions := ["run"], unanalyzable := [], absentFromRawScan := ["run"],
        refusals := [] } := by decide

/-- The other E01-B1 witness: a K&R definition is counted, carries an
unanalyzable row, and the unit cannot be published `Safe`. -/
theorem the_knr_case_grows_the_denominator_and_withholds_safe :
    build ["plain"] ["plain"] ["knr"] [] =
      { definitions := ["plain", "knr"], unanalyzable := ["knr"],
        absentFromRawScan := [], refusals := [] } ∧
    (build ["plain"] ["plain"] ["knr"] []).supportsUnitSafe = false := by decide

/-- Monotonicity is not vacuous in the other direction either: a definition the
expansion cannot see, because `#if 0` removed it, keeps its historical row. -/
theorem a_compiled_out_definition_keeps_its_row :
    (build ["hidden", "subject"] ["subject"] [] []).definitions =
      ["hidden", "subject"] := by decide

/-- An unavailable expansion leaves the historical scan intact and refuses. -/
theorem an_unavailable_expansion_keeps_the_historical_scan :
    build ["plain"] [] [] [Refusal.expansionUnavailable] =
      { definitions := ["plain"], unanalyzable := [], absentFromRawScan := [],
        refusals := [Refusal.expansionUnavailable] } ∧
    (build ["plain"] [] [] [Refusal.expansionUnavailable]).supportsUnitSafe
      = false := by decide

/-! ## Card CURRENT-RUN-DEFECTS: a unit whose every definition is compiled out

`SourceDefinitionInventory::build_excluded_only`: the completeness claim
stands, the expansion attributes no definition to the primary input, and the
raw scan names some. librdkafka's `tinycthread.c` under `WITH_C11THREADS` is
the instance. Its published reason states that cause; the theorems below say
the stated cause moves no row: the denominator stays exactly the raw scan
(never data-only), no row is analysed, and the unit verdict stays `Unknown`
(Cycle 19, E01-TAIL-EXCUSE). -/

/-- The Rust predicate over the enumeration's inputs. -/
def buildExcludedOnly (raw prototype identifierList : List Name)
    (refusals : List Refusal) : Bool :=
  refusals.isEmpty && prototype.isEmpty && identifierList.isEmpty && !raw.isEmpty

/-- The unit verdict the audit publishes for such a unit: the analysis has no
definition to enter, so it is `Unknown`, whatever reason it states. -/
def buildExcludedVerdict (_ : List Name) : Verdict := .unknown

theorem build_excluded_only_is_the_raw_scan (raw : List Name) :
    (build raw [] [] []).definitions = dedup raw := by
  simp [build, dedup]

/-- Never data-only: a non-empty raw scan keeps a non-empty denominator. -/
theorem a_build_excluded_unit_is_never_data_only (raw prototype identifierList : List Name)
    (refusals : List Refusal)
    (excluded : buildExcludedOnly raw prototype identifierList refusals = true) :
    (build raw prototype identifierList refusals).definitions ≠ [] := by
  simp only [buildExcludedOnly, Bool.and_eq_true, Bool.not_eq_true',
    List.isEmpty_eq_false_iff] at excluded
  obtain ⟨⟨⟨_, _⟩, _⟩, nonempty⟩ := excluded
  obtain ⟨name, rest, rfl⟩ := List.exists_cons_of_ne_nil nonempty
  intro empty
  have member := raw_is_kept (name :: rest) prototype identifierList refusals
    (List.mem_cons_self)
  rw [empty] at member
  exact List.not_mem_nil member

/-- Every row of such a unit is a raw-scan row the expansion does not have:
none is unanalyzable and none is new, so each stays in `not_projected`. -/
theorem a_build_excluded_unit_has_only_raw_rows (raw prototype identifierList : List Name)
    (refusals : List Refusal)
    (excluded : buildExcludedOnly raw prototype identifierList refusals = true) :
    (build raw prototype identifierList refusals).unanalyzable = [] ∧
      (build raw prototype identifierList refusals).absentFromRawScan = [] ∧
      ∀ name ∈ (build raw prototype identifierList refusals).definitions,
        name ∈ raw ∧ name ∉ prototype := by
  simp only [buildExcludedOnly, Bool.and_eq_true, List.isEmpty_iff] at excluded
  obtain ⟨⟨⟨rfl, rfl⟩, rfl⟩, _⟩ := excluded
  refine ⟨by simp [build, dedup], by simp [build, dedup], ?_⟩
  intro name member
  rw [build_excluded_only_is_the_raw_scan] at member
  exact ⟨(mem_dedup raw).mp member, List.not_mem_nil⟩

theorem a_build_excluded_unit_is_never_safe (raw : List Name) :
    buildExcludedVerdict raw ≠ Verdict.certifiedSafe := by
  simp [buildExcludedVerdict]

theorem a_build_excluded_unit_is_never_unsafe (raw : List Name) :
    buildExcludedVerdict raw ≠ Verdict.provisionalUnsafe := by
  simp [buildExcludedVerdict]

/-- The `tinycthread.c` shape, and the precision cases: one live definition,
no definition, a refused expansion. -/
theorem build_excluded_only_witnesses :
    buildExcludedOnly ["_cnd_timedwait_win32", "mtx_lock"] [] [] [] = true ∧
    buildExcludedOnly ["hidden", "subject"] ["subject"] [] [] = false ∧
    buildExcludedOnly [] [] [] [] = false ∧
    buildExcludedOnly ["plain"] [] [] [Refusal.expansionUnavailable] = false := by
  decide

end QCore.SourceEnumeration
