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

/-! Unit `E01-F1-RAW`: the raw token scan's logical-line directive rule.

This models the lexical relation of `qcore_c_gateway::legacy_c::tokenize` — the
tokenizer behind `legacy_c_source_functions`, the historical raw scan every
other main-source inventory in this repository is measured against.

`docs/reports/remaining-e01-f1/README.md` §2.2 diagnosed the defect this unit
repairs: **every** `#` was read as a directive introducer and discarded to the
end of the *physical* line. On a macro's backslash-continuation line the `#`
stringification operator therefore swallowed the rest of that line — a closing
brace included — the file-scope brace depth never returned to zero, and every
later definition in the file was invisible to the scan. That hid 146 of
`librdkafka/src/rdkafka.c`'s 165 definitions.

The rule implemented instead, and modelled here, is C11's:

* **5.1.1.2 phase 2** deletes each backslash immediately followed by a new-line,
  joining two physical lines into one *logical* line, before any preprocessing
  token is formed;
* **6.10** reads a directive only when `#` is the first preprocessing token of a
  logical line — nothing but white space and comments since the last undeleted
  new-line — and the directive then runs to the end of that logical line.

What is proved here:

* **totality and exclusivity of the classification** — every symbol in every
  state lands in exactly one lexical class, `#` introduces a directive *iff* it
  is line-initial, and nothing else ever introduces one;
* **a directive contributes no token** — single-line and continued alike, so a
  macro replacement list contributes no brace and perturbs no file-scope depth,
  which is the property the raw scan's definition finder rests on;
* **a non-line-initial `#` is a punctuator** — `#` stringification and `##`
  pasting are emitted, and the rest of their line survives;
* **white space and comments do not clear line-initiality, and a splice does not
  restore it** — the two directions that make the state flag correct;
* **non-vacuity against the defect** — the reduced `rdkafka.c` shape is decided
  both ways: the pre-repair rule counts the `{` the macro's continuation line
  opened, loses the `}` that closed it, and ends at residual depth one, while
  the repaired rule counts neither and returns to zero;
* **the repair removes nothing outside a directive** — on directive-free input
  the two rules agree, which is what makes the measured phantom-name
  disappearance (`docs/reports/remaining-e01-f1-raw/README.md` §1.3) an
  accounting statement about directive text rather than a lost definition;
* **exhaustive agreement with the Rust lexer** over every symbol string of
  length at most three, tied by a checksum both sides compute independently.

What is **not** proved here, and stays a Rust boundary: that the Rust byte
lexer's *other* branches (identifier and number runs, string and character
literals, the two comment scanners) behave as the symbols below abstract them,
and that `function_names` reads the resulting token stream as the definition
finder this repository documents. Those are pinned behaviourally by
`crates/qcore-c-gateway/tests/e01_f1_raw_tokenizer.rs`. The checksum ties the
directive rule over the seven single-byte symbols, not the whole byte lexer. -/
namespace QCore.RawDirectiveLexing

/-- The lexical alphabet, abstracting the Rust byte lexer to the classes the
directive rule distinguishes.

`ident` stands for one complete identifier or number run and `semi` for one
ordinary punctuator; the two brace symbols are singled out because file-scope
brace depth is what the defect corrupted. -/
inductive Sym where
  /-- `#`. -/
  | hash
  /-- A new-line that phase 2 did not delete. -/
  | newline
  /-- `\`. Followed by `newline` it is a splice; otherwise a punctuator. -/
  | backslash
  /-- Horizontal white space. -/
  | space
  /-- `{`. -/
  | lbrace
  /-- `}`. -/
  | rbrace
  /-- One ordinary punctuator, standing for every other single byte. -/
  | semi
  /-- One complete identifier or number run. -/
  | ident
  /-- A `//` comment introducer: white space running to the end of its logical
  line. -/
  | lineComment
  /-- A `/* … */` comment, carrying whether its body contains a new-line. -/
  | blockComment (spansLines : Bool)
  deriving DecidableEq, Repr

/-- A produced token. White space, comments and directives produce nothing. -/
inductive Tok where
  | hash
  | backslash
  | lbrace
  | rbrace
  | semi
  | ident
  deriving DecidableEq, Repr, BEq

/-- What a symbol is, in a given line-initial state. -/
inductive Class where
  /-- Deleted by phase 2 together with the new-line that follows it. -/
  | splice
  /-- Ends the logical line and restores line-initiality. -/
  | lineTerminator
  /-- White space or a comment; `crossesLine` records whether it carried a
  new-line, which restores line-initiality. -/
  | whiteSpace (crossesLine : Bool)
  /-- A `#` that is the first preprocessing token of its logical line. -/
  | directiveIntroducer
  /-- Anything that produces a token. -/
  | punctuation (tok : Tok)
  deriving DecidableEq, Repr

/-- The classification. `atStart` is true exactly while nothing but white space
and comments has been seen since the last undeleted new-line. The splice
decision needs the following symbol, so it is passed in; `none` is end of
input. -/
def classify (atStart : Bool) (sym : Sym) (next : Option Sym) : Class :=
  match sym with
  | .backslash => if next = some .newline then .splice else .punctuation .backslash
  | .newline => .lineTerminator
  | .space => .whiteSpace false
  | .lineComment => .whiteSpace false
  | .blockComment spans => .whiteSpace spans
  | .hash => if atStart then .directiveIntroducer else .punctuation .hash
  | .lbrace => .punctuation .lbrace
  | .rbrace => .punctuation .rbrace
  | .semi => .punctuation .semi
  | .ident => .punctuation .ident

mutual

/-- The repaired lexer. `atStart` carries line-initiality. -/
def lex (atStart : Bool) : List Sym → List Tok
  | [] => []
  | .backslash :: .newline :: rest => lex atStart rest
  | .newline :: rest => lex true rest
  | .space :: rest => lex atStart rest
  | .lineComment :: rest => skipLogicalLine rest
  | .blockComment spans :: rest => lex (atStart || spans) rest
  | .hash :: rest => if atStart then skipLogicalLine rest else .hash :: lex false rest
  | .backslash :: rest => .backslash :: lex false rest
  | .lbrace :: rest => .lbrace :: lex false rest
  | .rbrace :: rest => .rbrace :: lex false rest
  | .semi :: rest => .semi :: lex false rest
  | .ident :: rest => .ident :: lex false rest

/-- Consume the rest of a *logical* line — a directive's body, or a `//`
comment's — producing nothing, then resume at line start. Spliced new-lines do
not end it. -/
def skipLogicalLine : List Sym → List Tok
  | [] => []
  | .backslash :: .newline :: rest => skipLogicalLine rest
  | .newline :: rest => lex true rest
  | _ :: rest => skipLogicalLine rest

end

mutual

/-- The pre-repair lexer, for the non-vacuity theorems: every `#` is a directive
introducer, the scan runs to the next *physical* new-line, and phase 2 does not
happen. This is `tokenize` as it stood before unit E01-F1-RAW. -/
def lexBefore : List Sym → List Tok
  | [] => []
  | .newline :: rest => lexBefore rest
  | .space :: rest => lexBefore rest
  | .lineComment :: rest => skipPhysicalLine rest
  | .blockComment _ :: rest => lexBefore rest
  | .hash :: rest => skipPhysicalLine rest
  | .backslash :: rest => .backslash :: lexBefore rest
  | .lbrace :: rest => .lbrace :: lexBefore rest
  | .rbrace :: rest => .rbrace :: lexBefore rest
  | .semi :: rest => .semi :: lexBefore rest
  | .ident :: rest => .ident :: lexBefore rest

/-- To the next new-line, splices ignored. -/
def skipPhysicalLine : List Sym → List Tok
  | [] => []
  | .newline :: rest => lexBefore rest
  | _ :: rest => skipPhysicalLine rest

end

/-- File-scope brace depth of a token list, saturating at zero exactly as
`function_names` does. This is the quantity the defect corrupted. -/
def depth : List Tok → Nat → Nat
  | [], d => d
  | .lbrace :: rest, d => depth rest (d + 1)
  | .rbrace :: rest, d => depth rest (d - 1)
  | _ :: rest, d => depth rest d

/-! ## 1. The classification is a total function into disjoint classes -/

/-- Every (state, symbol, lookahead) triple is classified. -/
theorem classify_is_total (atStart : Bool) (sym : Sym) (next : Option Sym) :
    ∃ c, classify atStart sym next = c := ⟨_, rfl⟩

/-- And classified once: two classes assigned to the same triple are equal. -/
theorem classify_is_single_valued
    {atStart : Bool} {sym : Sym} {next : Option Sym} {c d : Class}
    (hc : classify atStart sym next = c) (hd : classify atStart sym next = d) : c = d := by
  subst hc; subst hd; rfl

/-- The rule, forwards: a line-initial `#` is a directive introducer. -/
theorem hash_at_line_start_is_a_directive (next : Option Sym) :
    classify true .hash next = .directiveIntroducer := rfl

/-- The rule, backwards: a `#` that is not line-initial is a punctuator. This is
the half the defect got wrong. -/
theorem hash_after_a_token_is_a_punctuator (next : Option Sym) :
    classify false .hash next = .punctuation .hash := rfl

/-- Stated as the biconditional the repair is. -/
theorem directive_iff_line_initial (atStart : Bool) (next : Option Sym) :
    classify atStart .hash next = .directiveIntroducer ↔ atStart = true := by
  cases atStart <;> simp [classify]

/-- Only `#` can introduce a directive. -/
theorem only_hash_introduces_a_directive
    (atStart : Bool) (sym : Sym) (next : Option Sym)
    (h : classify atStart sym next = .directiveIntroducer) : sym = .hash := by
  cases sym with
  | hash => rfl
  | backslash => by_cases hn : next = some .newline <;> simp [classify, hn] at h
  | newline => simp [classify] at h
  | space => simp [classify] at h
  | lbrace => simp [classify] at h
  | rbrace => simp [classify] at h
  | semi => simp [classify] at h
  | ident => simp [classify] at h
  | lineComment => simp [classify] at h
  | blockComment spans => simp [classify] at h

/-- A backslash is a splice exactly when a new-line follows it. -/
theorem splice_iff_newline_follows (atStart : Bool) (next : Option Sym) :
    classify atStart .backslash next = .splice ↔ next = some .newline := by
  by_cases h : next = some .newline <;> simp [classify, h]

/-! ## 2. A directive contributes no token -/

/-- A symbol run with no line boundary and no backslash: the shape a directive's
replacement list has between its splices. The backslash exclusion is a modelling
restriction, not a claim: `rdkafkaShape` below carries real backslashes and is
decided outright. -/
def PlainRun (s : List Sym) : Prop := Sym.newline ∉ s ∧ Sym.backslash ∉ s

theorem skipLogicalLine_plain :
    ∀ {body : List Sym}, PlainRun body →
      ∀ rest, skipLogicalLine (body ++ .newline :: rest) = lex true rest := by
  intro body
  induction body with
  | nil => intro _ rest; simp [skipLogicalLine]
  | cons sym body ih =>
      intro h rest
      have hbody : PlainRun body := ⟨fun m => h.1 (by simp [m]), fun m => h.2 (by simp [m])⟩
      have hn : sym ≠ Sym.newline := fun e => h.1 (by simp [e])
      have hb : sym ≠ Sym.backslash := fun e => h.2 (by simp [e])
      cases sym <;> simp_all [skipLogicalLine, ih hbody rest]

/-- **The load-bearing theorem.** A directive on a single logical line produces
no token at all, whatever its replacement list contains — braces included. -/
theorem a_directive_contributes_no_token
    {body : List Sym} (h : PlainRun body) (rest : List Sym) :
    lex true (.hash :: body ++ .newline :: rest) = lex true rest := by
  have : lex true (.hash :: (body ++ .newline :: rest))
      = skipLogicalLine (body ++ .newline :: rest) := by simp [lex]
  simpa [this] using skipLogicalLine_plain h rest

/-- A splice inside a directive keeps the directive going, so a continuation
line contributes no token either. This is exactly the line the defect tokenized
as file-scope source. -/
theorem skipLogicalLine_across_a_splice :
    ∀ {head : List Sym}, PlainRun head →
      ∀ cont, skipLogicalLine (head ++ .backslash :: .newline :: cont)
        = skipLogicalLine cont := by
  intro head
  induction head with
  | nil => intro _ cont; simp [skipLogicalLine]
  | cons sym head ih =>
      intro h cont
      have hhead : PlainRun head := ⟨fun m => h.1 (by simp [m]), fun m => h.2 (by simp [m])⟩
      have hn : sym ≠ Sym.newline := fun e => h.1 (by simp [e])
      have hb : sym ≠ Sym.backslash := fun e => h.2 (by simp [e])
      cases sym <;> simp_all [skipLogicalLine, ih hhead cont]

/-- The continued directive, end to end. -/
theorem a_continued_directive_contributes_no_token
    {head tail : List Sym} (hh : PlainRun head) (ht : PlainRun tail) (rest : List Sym) :
    lex true (.hash :: (head ++ .backslash :: .newline :: (tail ++ .newline :: rest)))
      = lex true rest := by
  have step : lex true (.hash :: (head ++ .backslash :: .newline :: (tail ++ .newline :: rest)))
      = skipLogicalLine (head ++ .backslash :: .newline :: (tail ++ .newline :: rest)) := by
    simp [lex]
  rw [step, skipLogicalLine_across_a_splice hh (tail ++ .newline :: rest),
    skipLogicalLine_plain ht rest]

/-- Corollary, in the quantity that matters: a directive's braces do not move
file-scope depth. -/
theorem a_directive_body_moves_no_brace_depth
    {body : List Sym} (h : PlainRun body) (rest : List Sym) (d : Nat) :
    depth (lex true (.hash :: body ++ .newline :: rest)) d = depth (lex true rest) d := by
  rw [a_directive_contributes_no_token h]

/-- And for the continued form, which is the defect's own shape. -/
theorem a_continued_directive_body_moves_no_brace_depth
    {head tail : List Sym} (hh : PlainRun head) (ht : PlainRun tail)
    (rest : List Sym) (d : Nat) :
    depth (lex true (.hash :: (head ++ .backslash :: .newline :: (tail ++ .newline :: rest)))) d
      = depth (lex true rest) d := by
  rw [a_continued_directive_contributes_no_token hh ht]

/-! ## 3. State transitions: what does and does not restore line-initiality -/

/-- A new-line restores line-initiality. -/
theorem a_newline_starts_a_logical_line (atStart : Bool) (rest : List Sym) :
    lex atStart (.newline :: rest) = lex true rest := by cases atStart <;> simp [lex]

/-- A splice does **not**: the logical line continues. -/
theorem a_splice_preserves_line_initiality (atStart : Bool) (rest : List Sym) :
    lex atStart (.backslash :: .newline :: rest) = lex atStart rest := by
  cases atStart <;> simp [lex]

/-- The consequence that is the whole defect: a `#` reached across a splice,
after real code, is a punctuator. -/
theorem a_hash_across_a_splice_after_code_is_a_punctuator (rest : List Sym) :
    lex true (.ident :: .backslash :: .newline :: .hash :: rest)
      = .ident :: .hash :: lex false rest := by
  simp [lex]

/-- White space preserves line-initiality, so an indented directive is one. -/
theorem white_space_preserves_line_initiality (atStart : Bool) (rest : List Sym) :
    lex atStart (.space :: rest) = lex atStart rest := by cases atStart <;> simp [lex]

/-- A block comment that spans lines restores line-initiality, so a `#` after it
is a directive. -/
theorem a_block_comment_that_spans_lines_starts_a_logical_line
    (atStart : Bool) (rest : List Sym) :
    lex atStart (.blockComment true :: rest) = lex true rest := by
  cases atStart <;> simp [lex]

/-- A block comment that does not span lines preserves the state it found. -/
theorem a_block_comment_within_a_line_preserves_the_state
    (atStart : Bool) (rest : List Sym) :
    lex atStart (.blockComment false :: rest) = lex atStart rest := by
  cases atStart <;> simp [lex]

/-- A `//` comment runs to the end of its *logical* line, so a spliced
continuation stays commented and its braces never count. -/
theorem a_line_comment_swallows_its_continuation
    {body : List Sym} (h : PlainRun body) (atStart : Bool) (rest : List Sym) :
    lex atStart (.lineComment :: body ++ .newline :: rest) = lex true rest := by
  have : lex atStart (.lineComment :: (body ++ .newline :: rest))
      = skipLogicalLine (body ++ .newline :: rest) := by cases atStart <;> simp [lex]
  simpa [this] using skipLogicalLine_plain h rest

/-- Every token clears line-initiality, which is what makes a `#` after code on
the same line a punctuator. -/
theorem a_token_clears_line_initiality (atStart : Bool) (rest : List Sym) :
    lex atStart (.ident :: .hash :: rest) = .ident :: .hash :: lex false rest := by
  cases atStart <;> simp [lex]

/-! ## 4. A non-line-initial `#` is emitted, and `##` is two of them -/

/-- Stringification: the `#` survives as a token and the rest of its line is not
discarded. -/
theorem stringification_is_emitted (rest : List Sym) :
    lex false (.hash :: rest) = .hash :: lex false rest := by simp [lex]

/-- Token pasting is two punctuators, never a directive. -/
theorem pasting_is_two_punctuators (rest : List Sym) :
    lex false (.hash :: .hash :: rest) = .hash :: .hash :: lex false rest := by simp [lex]

/-! ## 5. Non-vacuity against the measured defect -/

/-- The reduced `librdkafka/src/rdkafka.c` shape of
`docs/reports/remaining-e01-f1/README.md` §2.2, in this alphabet:

```
#define ENTRY(E)  \
        {E, &(#E)[0]}
static const struct p t[] = { ENTRY(1) };
int after(void) { return 0; }
```

The macro's continuation line carries a stringification `#` *and* the closing
brace of the initializer. -/
def rdkafkaShape : List Sym :=
  -- `#define ENTRY(E)` then a splice
  [.hash, .ident, .space, .ident, .semi, .space, .backslash, .newline,
  -- the continuation line: `{E, &(#E)[0]}`
   .space, .lbrace, .ident, .semi, .hash, .ident, .semi, .rbrace, .newline,
  -- `static const struct p t[] = { ENTRY(1) };`
   .ident, .space, .lbrace, .ident, .rbrace, .semi, .newline,
  -- `int after(void) { return 0; }`
   .ident, .space, .lbrace, .ident, .semi, .rbrace, .newline]

/-- The repaired lexer returns to depth zero on that shape. -/
theorem the_repaired_lexer_balances_the_measured_shape :
    depth (lex true rdkafkaShape) 0 = 0 := by decide

/-- The pre-repair lexer does not: the continuation line's `#` swallowed the
closing brace, so the residual depth is one and every later definition sits at
depth one, where the definition finder does not look. -/
theorem the_pre_repair_lexer_desynchronises_the_measured_shape :
    depth (lexBefore rdkafkaShape) 0 = 1 := by decide

/-- Stated as the difference, so nothing here is vacuous. -/
theorem the_repair_changes_the_measured_shape :
    depth (lex true rdkafkaShape) 0 ≠ depth (lexBefore rdkafkaShape) 0 := by decide

/-- Under the repair both constructs after the macro contribute a balanced brace
pair, and the macro contributes none. -/
theorem the_repaired_lexer_sees_the_later_definitions :
    (lex true rdkafkaShape).count .lbrace = 2
      ∧ (lex true rdkafkaShape).count .rbrace = 2 := by decide

/-- The pre-repair lexer's brace counts on the same shape, decided rather than
asserted: it counts the `{` the macro's continuation line opened — because it
tokenized that line as ordinary source — and loses the `}` that closed it,
because the same line's stringification `#` discarded the rest of the line.
Three opens against two closes is exactly the residual depth above. -/
theorem the_pre_repair_lexer_counts_an_unclosed_macro_brace :
    (lexBefore rdkafkaShape).count .lbrace = 3
      ∧ (lexBefore rdkafkaShape).count .rbrace = 2 := by decide

/-! ## 6. The repair removes nothing outside a directive -/

/-- On input with no `#`, no `\` and no `//`, the two rules agree token for
token: the repair is confined to directive text and phase-2 splices. -/
theorem the_rules_agree_on_directive_free_input :
    ∀ s : List Sym, Sym.hash ∉ s → Sym.backslash ∉ s → Sym.lineComment ∉ s →
      ∀ atStart, lex atStart s = lexBefore s := by
  intro s
  induction s with
  | nil => intro _ _ _ atStart; cases atStart <;> simp [lex, lexBefore]
  | cons sym rest ih =>
      intro h1 h2 h3 atStart
      have r1 : Sym.hash ∉ rest := fun m => h1 (by simp [m])
      have r2 : Sym.backslash ∉ rest := fun m => h2 (by simp [m])
      have r3 : Sym.lineComment ∉ rest := fun m => h3 (by simp [m])
      have hs1 : sym ≠ Sym.hash := fun e => h1 (by simp [e])
      have hs2 : sym ≠ Sym.backslash := fun e => h2 (by simp [e])
      have hs3 : sym ≠ Sym.lineComment := fun e => h3 (by simp [e])
      cases sym with
      | hash => exact absurd rfl hs1
      | backslash => exact absurd rfl hs2
      | lineComment => exact absurd rfl hs3
      | newline => simp only [lex, lexBefore]; exact ih r1 r2 r3 true
      | space => simp only [lex, lexBefore]; exact ih r1 r2 r3 atStart
      | blockComment spans =>
          simp only [lex, lexBefore]; exact ih r1 r2 r3 (atStart || spans)
      | lbrace => simp only [lex, lexBefore, ih r1 r2 r3 false]
      | rbrace => simp only [lex, lexBefore, ih r1 r2 r3 false]
      | semi => simp only [lex, lexBefore, ih r1 r2 r3 false]
      | ident => simp only [lex, lexBefore, ih r1 r2 r3 false]

/-! ## 7. Exhaustive agreement with the Rust lexer -/

/-- The digest alphabet: the seven symbols that map to exactly one byte each in
the Rust lexer, so a symbol string and a byte string denote the same input.
Identifier runs are excluded because the Rust lexer merges adjacent identifier
bytes into one token; comments are excluded because they are multi-byte. -/
def digestAlphabet : List Sym :=
  [.hash, .newline, .backslash, .space, .lbrace, .rbrace, .semi]

/-- Every symbol string over `digestAlphabet` of length exactly `n`. -/
def strings : Nat → List (List Sym)
  | 0 => [[]]
  | n + 1 => (strings n).flatMap fun s => digestAlphabet.map fun c => c :: s

/-- Every such string of length at most `n`, shortest first. -/
def domain (n : Nat) : List (List Sym) :=
  (List.range (n + 1)).flatMap strings

/-- A token's code, matching the Rust side's transcription. -/
def Tok.code : Tok → Nat
  | .hash => 1
  | .backslash => 2
  | .lbrace => 3
  | .rbrace => 4
  | .semi => 5
  | .ident => 6

/-- A checksum of one token list. -/
def tokenChecksum (toks : List Tok) : Nat :=
  toks.foldl (fun acc t => (acc * 31 + t.code) % 1000003) 7

/-- The whole-domain checksum. Rust computes the same fold over the same domain
by running the real `tokenize` on the corresponding byte strings; the two
numbers agreeing is the tie between this model and the implementation. -/
def domainChecksum (n : Nat) : Nat :=
  (domain n).foldl (fun acc s => (acc * 1000033 + tokenChecksum (lex true s)) % 1000003) 11

set_option maxRecDepth 100000 in
/-- The domain is the size it claims: 1 + 7 + 49 + 343 strings. -/
theorem the_digest_domain_is_exhaustive : (domain 3).length = 400 := by decide

set_option maxRecDepth 4000000 in
/-- The checksum, decided in the kernel. `crates/qcore-c-gateway`'s
`the_lean_domain_checksum_reproduces` recomputes it from the real byte
tokenizer and asserts this exact number. -/
theorem the_domain_checksum_is_pinned : domainChecksum 3 = 478977 := by decide


end QCore.RawDirectiveLexing
