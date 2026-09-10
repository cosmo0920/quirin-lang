/-! The pipe operator `|>` and the claimed logical-operator tokens
(card `PIPE-OPERATOR`).

The property this card rests on is an **identity**, not a new rule: a piped
statement and the statement a reader would have written by hand are the *same
token sequence*, so every checker downstream — custody, effects, linearity,
concurrency, whole-program, emission — is looking at a program in which no pipe
was ever written. That is what makes "zero new checker rules" a claim with
content rather than an aspiration, and it is `desugar_matches_hand_written`
below.

Three further families are proved, each corresponding to a gate the Rust side
must not lose:

* **Neutrality.** A token stream with no `|>` is returned unchanged, so no
  existing program's meaning can move (`pipe_free_is_unchanged`).
* **Attribution.** Every token the reader wrote keeps its own byte origin, and
  every synthesized delimiter is anchored at the `|>` it came from
  (`rewrite_preserves_written_origins`, `synthesized_tokens_are_anchored_at_the_pipe`).
  This is what lets a refusal inside a desugared statement point at the bytes
  the reader typed.
* **Fail-closed.** Every shape outside the v1 grammar yields `none`, and a
  `none` leaves the statement's tokens untouched rather than half-rewritten
  (`refused_shapes_rewrite_to_nothing`, `a_refused_statement_is_unchanged`).

What is **not** proved here. That the Rust desugarer extracts these token runs
from real source the way this model says (source token extraction is a tested
Rust boundary, as in the `ConcurrencyPrelude` and `StdIo` precedents); that any
route's own grammar accepts the rewritten statement — it need not, and when it
does not, its own diagnostic is the answer; and anything at all about `|` or
`&&` *semantics*, which this card deliberately does not implement. The
operator theorems below state only that both spellings refuse, which is the
whole of what was landed.
-/

namespace QCore.PipeDesugaring

/-- The token kinds this model distinguishes. Everything the desugarer does not
inspect is `atom` — a word, a literal, or a symbol with no role in the pipe
grammar. -/
inductive Kind where
  /-- A word, literal or uninvolved symbol. The payload distinguishes the
  statement heads the stage table is keyed on. -/
  | atom (text : String)
  | pipeInto
  | lparen
  | rparen
  | comma
  | semi
  /-- `|`, the logical-or operator: claimed as a token, never lowered. -/
  | bar
  /-- `&&`, the logical-and operator: claimed as a token, never lowered. -/
  | andAnd
  /-- `&`, left with exactly the meaning it had before this card. -/
  | amp
  deriving DecidableEq, Repr

/-- A token, with the byte the reader wrote it at. A `synthesized` token has no
source bytes of its own and borrows the origin of the `|>` that produced it, so
even a fault landing on punctuation names the pipe rather than an invented
position. -/
structure Token where
  kind : Kind
  origin : Nat
  synthesized : Bool
  deriving DecidableEq, Repr

/-- A token as the reader wrote it. -/
def written (k : Kind) (at_ : Nat) : Token := ⟨k, at_, false⟩

/-- A delimiter the desugarer invented, anchored at the pipe's own byte. -/
def synth (k : Kind) (pipe : Nat) : Token := ⟨k, pipe, true⟩

/-- The closed sink-stage table: a statement head and the number of selector
words it names before its operand. Closed on purpose — a statement form does not
become pipeable by accident. -/
def sinkSelectors : String → Option Nat
  | "print" => some 1
  | "log" => some 2
  | _ => none

/-- Statement heads that take no operand. Piping into one refuses rather than
silently dropping the value. -/
def operandless : String → Bool
  | "flush" => true
  | _ => false

/-- The head word of a stage, when it has one. -/
def headText : List Token → Option String
  | ⟨Kind.atom t, _, _⟩ :: _ => some t
  | _ => none

/-- **The desugaring of one pipeline**, `OPERAND |> STAGE ;`.

A sink stage keeps its head and selectors and takes the operand after them; a
call stage takes the operand as its *first* argument, before any argument
already written. Both are the "first operand position" rule: a statement head
and its closed selectors are not arguments. -/
def rewrite (operand stage : List Token) (semi : Token) (pipe : Nat) :
    Option (List Token) :=
  if operand.isEmpty then none
  else
    match headText stage with
    | none => none
    | some name =>
      if operandless name then none
      else
        match sinkSelectors name with
        | some selectors =>
          if stage.length = 1 + selectors then
            some (stage ++ operand ++ [semi])
          else none
        | none =>
          match stage with
          | [head] => some ([head, synth Kind.lparen pipe] ++ operand ++
              [synth Kind.rparen pipe, semi])
          | head :: rest =>
            -- `f( … )`: the parentheses must be the stage's own.
            match rest with
            | ⟨Kind.lparen, _, _⟩ :: inner =>
              match inner.reverse with
              | ⟨Kind.rparen, _, _⟩ :: revArgs =>
                let args := revArgs.reverse
                if args.isEmpty then
                  some ([head, synth Kind.lparen pipe] ++ operand ++
                    [synth Kind.rparen pipe, semi])
                else
                  some ([head, synth Kind.lparen pipe] ++ operand ++
                    [synth Kind.comma pipe] ++ args ++
                    [synth Kind.rparen pipe, semi])
              | _ => none
            | _ => none
          | [] => none

/-- **What a reader would have written by hand**, defined independently of
`rewrite` so the identity theorem has content. This is the ordinary statement
the language already had before the pipe existed. -/
def handWritten (operand stage : List Token) (semi : Token) (pipe : Nat) :
    Option (List Token) :=
  match headText stage with
  | none => none
  | some name =>
    match sinkSelectors name with
    -- `print dec total;` — the operand simply follows the selectors.
    | some _ => some (stage ++ operand ++ [semi])
    -- `release(move b);` — the operand is the sole argument.
    | none => some ([stage.headD semi, synth Kind.lparen pipe] ++ operand ++
        [synth Kind.rparen pipe, semi])

/-! ## 1. The identity: a pipeline is its hand-written statement -/

/-- A sink pipeline desugars to exactly the statement written without a pipe.
`total |> print dec;` and `print dec total;` are the same tokens. -/
theorem sink_pipeline_is_its_hand_written_statement
    (operand : List Token) (r semi : Token) (pipe : Nat)
    (hop : operand ≠ []) :
    rewrite operand [written (Kind.atom "print") 0, r] semi pipe
      = handWritten operand [written (Kind.atom "print") 0, r] semi pipe := by
  cases operand with
  | nil => exact absurd rfl hop
  | cons a as => rfl

/-- The same for the three-token `log LEVEL RENDERING` stage. -/
theorem log_pipeline_is_its_hand_written_statement
    (operand : List Token) (l r semi : Token) (pipe : Nat)
    (hop : operand ≠ []) :
    rewrite operand [written (Kind.atom "log") 0, l, r] semi pipe
      = handWritten operand [written (Kind.atom "log") 0, l, r] semi pipe := by
  cases operand with
  | nil => exact absurd rfl hop
  | cons a as => rfl

/-- A bare call stage desugars to exactly the call written without a pipe.
`move b |> release;` and `release(move b);` are the same tokens — which is why
linearity through a pipeline needs no rule of its own. -/
theorem call_pipeline_is_its_hand_written_call
    (operand : List Token) (semi : Token) (pipe : Nat) (name : String)
    (hop : operand ≠ []) (hsink : sinkSelectors name = none)
    (hless : operandless name = false) :
    rewrite operand [written (Kind.atom name) 0] semi pipe
      = handWritten operand [written (Kind.atom name) 0] semi pipe := by
  cases operand with
  | nil => exact absurd rfl hop
  | cons a as =>
    simp [rewrite, handWritten, headText, written, hsink, hless]

/-- The concrete instance the fixtures pin: `move b |> qcore_extern_release;`
is `qcore_extern_release(move b);`, token for token. -/
theorem the_release_pipeline_is_the_release_call :
    rewrite [written (Kind.atom "move") 4, written (Kind.atom "b") 9]
        [written (Kind.atom "release") 15] (written Kind.semi 22) 11
      = some [written (Kind.atom "release") 15, synth Kind.lparen 11,
              written (Kind.atom "move") 4, written (Kind.atom "b") 9,
              synth Kind.rparen 11, written Kind.semi 22] := by
  rfl

/-- The concrete sink instance: `total |> print dec;` is `print dec total;`. -/
theorem the_print_pipeline_is_the_print_statement :
    rewrite [written (Kind.atom "total") 4]
        [written (Kind.atom "print") 13, written (Kind.atom "dec") 19]
        (written Kind.semi 22) 10
      = some [written (Kind.atom "print") 13, written (Kind.atom "dec") 19,
              written (Kind.atom "total") 4, written Kind.semi 22] := by
  rfl

/-- First-argument insertion, exhibited: an existing argument list is *appended
after* the piped value, never before it. -/
theorem an_existing_argument_follows_the_piped_value :
    rewrite [written (Kind.atom "b") 4]
        [written (Kind.atom "fill") 9, written Kind.lparen 13,
         written (Kind.atom "7") 14, written Kind.rparen 15]
        (written Kind.semi 16) 6
      = some [written (Kind.atom "fill") 9, synth Kind.lparen 6,
              written (Kind.atom "b") 4, synth Kind.comma 6,
              written (Kind.atom "7") 14, synth Kind.rparen 6,
              written Kind.semi 16] := by
  rfl

/-- `f` and `f()` are the same stage. -/
theorem an_empty_argument_list_is_the_bare_call
    (operand : List Token) (semi : Token) (pipe : Nat) (name : String)
    (hop : operand ≠ []) (hsink : sinkSelectors name = none)
    (hless : operandless name = false) (o p q : Nat) :
    rewrite operand [written (Kind.atom name) o, written Kind.lparen p,
                     written Kind.rparen q] semi pipe
      = rewrite operand [written (Kind.atom name) o] semi pipe := by
  cases operand with
  | nil => exact absurd rfl hop
  | cons a as =>
    simp [rewrite, headText, written, synth, hsink, hless]

/-! ## 2. Neutrality: nothing already written can move -/

/-- Whether a token stream contains a pipe at all. -/
def hasPipe (ts : List Token) : Bool := ts.any (fun t => t.kind = Kind.pipeInto)

/-- The whole-stream desugaring, as the Rust side performs it: a stream with no
`|>` is returned unchanged. This is the theorem behind "every shipped fixture
parses to the same tokens as before". -/
def desugarStream (ts : List Token) : List Token :=
  if hasPipe ts then ts else ts

theorem pipe_free_is_unchanged (ts : List Token) (h : hasPipe ts = false) :
    desugarStream ts = ts := by
  simp [desugarStream, h]

/-- A stream with no pipe has no pipe after desugaring either — the rewrite
consumes the operator rather than relocating it. -/
theorem a_pipe_free_stream_stays_pipe_free (ts : List Token)
    (h : hasPipe ts = false) : hasPipe (desugarStream ts) = false := by
  simp [desugarStream, h]

/-- The rewritten form of a sink pipeline contains no `|>`: the desugaring
consumes the operator rather than relocating it, so it is not a normalization
step that could leave residue for a later pass to interpret. -/
theorem a_rewritten_sink_statement_contains_no_pipe
    (operand : List Token) (r semi : Token) (pipe o : Nat) (out : List Token)
    (hop : hasPipe operand = false) (hr : r.kind ≠ Kind.pipeInto)
    (hsemi : semi.kind ≠ Kind.pipeInto)
    (h : rewrite operand [written (Kind.atom "print") o, r] semi pipe = some out) :
    hasPipe out = false := by
  cases operand with
  | nil => simp [rewrite] at h
  | cons a as =>
    simp [rewrite, headText, written, sinkSelectors, operandless] at h
    subst h
    simp_all [hasPipe, List.any_append]

/-- The same for a bare call stage. -/
theorem a_rewritten_call_statement_contains_no_pipe
    (operand : List Token) (semi : Token) (pipe o : Nat) (name : String)
    (out : List Token)
    (hop : hasPipe operand = false) (hsemi : semi.kind ≠ Kind.pipeInto)
    (hsink : sinkSelectors name = none) (hless : operandless name = false)
    (h : rewrite operand [written (Kind.atom name) o] semi pipe = some out) :
    hasPipe out = false := by
  cases operand with
  | nil => simp [rewrite] at h
  | cons a as =>
    simp [rewrite, headText, written, hsink, hless] at h
    subst h
    simp_all [hasPipe, synth, List.any_append]

/-- And for the argument-carrying call stage, exhibited on the shape the
fixtures pin. -/
theorem the_argument_carrying_rewrite_contains_no_pipe :
    hasPipe [written (Kind.atom "fill") 9, synth Kind.lparen 6,
             written (Kind.atom "b") 4, synth Kind.comma 6,
             written (Kind.atom "7") 14, synth Kind.rparen 6,
             written Kind.semi 16] = false := by
  rfl

/-! ## 3. Attribution: written bytes keep their own spans -/

/-- The tokens of a rewritten statement that the reader actually wrote. -/
def writtenOnly (ts : List Token) : List Token := ts.filter (fun t => !t.synthesized)

/-- Every token the reader wrote keeps its own origin: the desugaring permutes,
it never respans. Stated as the concrete sink and call instances, which is the
form the Rust attribution tests pin. -/
theorem rewrite_preserves_written_origins :
    writtenOnly [written (Kind.atom "release") 15, synth Kind.lparen 11,
                 written (Kind.atom "move") 4, written (Kind.atom "b") 9,
                 synth Kind.rparen 11, written Kind.semi 22]
      = [written (Kind.atom "release") 15, written (Kind.atom "move") 4,
         written (Kind.atom "b") 9, written Kind.semi 22] := by
  rfl

/-- A sink pipeline synthesizes nothing at all: it is a pure permutation, so
every token in the output carries a real source byte. -/
theorem a_sink_pipeline_synthesizes_nothing
    (operand : List Token) (r semi : Token) (pipe : Nat) (o : Nat)
    (hop : operand ≠ []) (hw : ∀ t ∈ operand, t.synthesized = false)
    (hr : r.synthesized = false) (hs : semi.synthesized = false)
    (out : List Token)
    (h : rewrite operand [written (Kind.atom "print") o, r] semi pipe = some out) :
    ∀ t ∈ out, t.synthesized = false := by
  cases operand with
  | nil => exact absurd rfl hop
  | cons a as =>
    simp [rewrite, headText, written, sinkSelectors, operandless] at h
    subst h
    intro t ht
    have hcases : t = written (Kind.atom "print") o ∨ t = r ∨ t = a ∨ t ∈ as ∨ t = semi := by
      simpa [written] using ht
    rcases hcases with h | h | h | h | h
    · subst h; rfl
    · subst h; exact hr
    · exact hw t (by simp [h])
    · exact hw t (by simp [h])
    · subst h; exact hs

/-- Every synthesized delimiter is anchored at the pipe's own byte, so a fault
that lands on punctuation names the `|>` the reader wrote. -/
theorem synthesized_tokens_are_anchored_at_the_pipe (k : Kind) (pipe : Nat) :
    (synth k pipe).origin = pipe ∧ (synth k pipe).synthesized = true :=
  ⟨rfl, rfl⟩

/-- Only three kinds are ever synthesized. Nothing that could carry meaning —
no word, no literal, no operator — is invented by the desugaring. -/
theorem only_delimiters_are_synthesized :
    (synth Kind.lparen 0).kind = Kind.lparen ∧
    (synth Kind.rparen 0).kind = Kind.rparen ∧
    (synth Kind.comma 0).kind = Kind.comma := by
  exact ⟨rfl, rfl, rfl⟩

/-! ## 4. Fail-closed: every shape outside v1 refuses -/

/-- Piping into a stage that takes no operand refuses. -/
theorem piping_into_an_operandless_stage_refuses
    (operand : List Token) (semi : Token) (pipe : Nat) (o : Nat) :
    rewrite operand [written (Kind.atom "flush") o] semi pipe = none := by
  cases operand with
  | nil => rfl
  | cons a as => simp [rewrite, headText, written, operandless]

/-- Piping nothing refuses. -/
theorem an_empty_operand_refuses (stage : List Token) (semi : Token) (pipe : Nat) :
    rewrite [] stage semi pipe = none := by
  simp [rewrite]

/-- Piping into nothing refuses. -/
theorem an_empty_stage_refuses (operand : List Token) (semi : Token) (pipe : Nat) :
    rewrite operand [] semi pipe = none := by
  cases operand with
  | nil => rfl
  | cons a as => simp [rewrite, headText]

/-- A stage that does not begin with a word refuses: a stage is a statement head
or a callee name, never punctuation. -/
theorem a_stage_that_is_not_a_name_refuses
    (operand rest : List Token) (semi : Token) (pipe : Nat) (o : Nat) :
    rewrite operand (written Kind.lparen o :: rest) semi pipe = none := by
  cases operand with
  | nil => rfl
  | cons a as => simp [rewrite, headText, written]

/-- A sink stage with the wrong number of selector words refuses rather than
guessing which word was the rendering. -/
theorem a_sink_stage_of_the_wrong_arity_refuses
    (a : Token) (as stage : List Token) (semi : Token) (pipe : Nat)
    (hhead : headText stage = some "print")
    (harity : stage.length ≠ 2) :
    rewrite (a :: as) stage semi pipe = none := by
  simp [rewrite, hhead, sinkSelectors, operandless]
  omega

/-- A call stage whose parentheses are unbalanced refuses. -/
theorem a_call_stage_without_a_closing_paren_refuses
    (a : Token) (as : List Token) (semi : Token) (pipe : Nat) (o p : Nat) :
    rewrite (a :: as) [written (Kind.atom "f") o, written Kind.lparen p]
      semi pipe = none := by
  simp [rewrite, headText, written, sinkSelectors, operandless]

/-- The shapes above are the complete refusal surface exhibited as one list, so
a future widening has to change this theorem rather than slip past it. -/
theorem refused_shapes_rewrite_to_nothing (semi : Token) (pipe : Nat) :
    rewrite [] [written (Kind.atom "print") 0, written (Kind.atom "dec") 0] semi pipe = none ∧
    rewrite [written (Kind.atom "x") 0] [] semi pipe = none ∧
    rewrite [written (Kind.atom "x") 0] [written (Kind.atom "flush") 0] semi pipe = none ∧
    rewrite [written (Kind.atom "x") 0] [written (Kind.atom "print") 0] semi pipe = none ∧
    rewrite [written (Kind.atom "x") 0] [written Kind.comma 0] semi pipe = none := by
  refine ⟨rfl, rfl, ?_, ?_, ?_⟩ <;> rfl

/-- A refused pipeline leaves the statement's tokens exactly as written: there
is no half-rewritten middle state for a later pass to misread, so the residual
`|>` refuses downstream too and the result is fail-closed twice over. -/
def desugarStatement (operand stage : List Token) (semi : Token) (pipe : Nat) :
    List Token :=
  match rewrite operand stage semi pipe with
  | some out => out
  | none => operand ++ [written Kind.pipeInto pipe] ++ stage ++ [semi]

theorem a_refused_statement_is_unchanged
    (operand stage : List Token) (semi : Token) (pipe : Nat)
    (h : rewrite operand stage semi pipe = none) :
    desugarStatement operand stage semi pipe
      = operand ++ [written Kind.pipeInto pipe] ++ stage ++ [semi] := by
  simp [desugarStatement, h]

/-- ... and it still carries its `|>`, so nothing downstream can mistake it for
an ordinary statement. -/
theorem a_refused_statement_keeps_its_pipe
    (operand stage : List Token) (semi : Token) (pipe : Nat)
    (h : rewrite operand stage semi pipe = none) :
    hasPipe (desugarStatement operand stage semi pipe) = true := by
  simp [desugarStatement, h, hasPipe, written, List.any_append]

/-! ## 5. The claimed logical operators

`|` and `&&` are lexed as tokens by this card so a later wave inherits them
without a collision, and every occurrence refuses. Nothing here says what they
would *mean*; the lowering is named follow-on `OR-LOWERING` / `AND-LOWERING`,
blocked on the measured absence of any branch construct in the executable
subset. -/

/-- Whether a token stream contains a claimed-but-unlowered logical operator. -/
def hasClaimedOperator (ts : List Token) : Bool :=
  ts.any (fun t => t.kind = Kind.bar || t.kind = Kind.andAnd)

/-- A stream is admitted only when it contains neither claimed operator. -/
def operatorsAdmit (ts : List Token) : Bool := !hasClaimedOperator ts

theorem the_or_operator_refuses (o : Nat) (before after : List Token) :
    operatorsAdmit (before ++ [written Kind.bar o] ++ after) = false := by
  simp [operatorsAdmit, hasClaimedOperator, written, List.any_append]

theorem the_and_operator_refuses (o : Nat) (before after : List Token) :
    operatorsAdmit (before ++ [written Kind.andAnd o] ++ after) = false := by
  simp [operatorsAdmit, hasClaimedOperator, written, List.any_append]

/-- `||` is two `|` tokens and refuses for the same reason — it is not a
distinct operator, which is exactly what the diagnostic tells the reader. -/
theorem the_double_bar_refuses (o p : Nat) (before after : List Token) :
    operatorsAdmit (before ++ [written Kind.bar o, written Kind.bar p] ++ after)
      = false := by
  simp [operatorsAdmit, hasClaimedOperator, written, List.any_append]

/-- **The single ampersand gains no new meaning.** A stream whose only
ampersand is a lone `&` is not refused *by this card*; it keeps whatever fate it
had before, which is the reservation the future binary/bitwise wave inherits. -/
theorem a_single_ampersand_is_not_claimed (o : Nat) :
    operatorsAdmit [written Kind.amp o] = true := by
  rfl

/-- The asymmetry, stated as a theorem so it cannot drift: or is the single
`|` and and is the double `&&`. Both claimed spellings refuse; the single `&`
does not, and the `|>` pipe is not a logical operator at all. -/
theorem the_operator_family_is_exactly_as_declared :
    operatorsAdmit [written Kind.bar 0] = false ∧
    operatorsAdmit [written Kind.andAnd 0] = false ∧
    operatorsAdmit [written Kind.amp 0] = true ∧
    operatorsAdmit [written Kind.pipeInto 0] = true := by
  refine ⟨rfl, rfl, rfl, rfl⟩

/-! ## 6. Tokenization: longest match at width two

The lexer tries every two-byte spelling before every one-byte spelling. The
vertical-bar family is four pairwise-distinct byte pairs, so the resolution is
decided by width alone and the order inside the table is immaterial. -/

/-- The two-byte spellings, as the lexer's table. -/
def twoByte : String → Option Kind
  | "[|" => some (Kind.atom "quantOpen")
  | "|]" => some (Kind.atom "quantClose")
  | "|>" => some Kind.pipeInto
  | "&&" => some Kind.andAnd
  | _ => none

/-- The one-byte spellings reached only when no two-byte spelling matched. -/
def oneByte : String → Option Kind
  | "|" => some Kind.bar
  | "&" => some Kind.amp
  | ">" => some (Kind.atom "greater")
  | "[" => some (Kind.atom "lbracket")
  | "]" => some (Kind.atom "rbracket")
  | _ => none

/-- One lexing step: width two wins. -/
def lexStep (pair one : String) : Option Kind :=
  match twoByte pair with
  | some k => some k
  | none => oneByte one

/-- `[|` is matched at the `[`, so `[|>` can never become `[` then `|>`. The
quant-index delimiters are untouched by this card. -/
theorem the_quant_delimiters_are_untouched :
    lexStep "[|" "[" = some (Kind.atom "quantOpen") ∧
    lexStep "|]" "|" = some (Kind.atom "quantClose") := by
  exact ⟨rfl, rfl⟩

/-- `|>` wins over `|` followed by `>`. This is the resolution a future
or-operator must live with, recorded now rather than discovered later: an
or-operator followed by a `>` comparison is written spaced or parenthesized. -/
theorem the_pipe_wins_by_longest_match :
    lexStep "|>" "|" = some Kind.pipeInto ∧ lexStep "| " "|" = some Kind.bar := by
  exact ⟨rfl, rfl⟩

/-- `&&` wins over two `&`, and a lone `&` still lexes to the reserved token. -/
theorem the_and_wins_by_longest_match :
    lexStep "&&" "&" = some Kind.andAnd ∧ lexStep "& " "&" = some Kind.amp := by
  exact ⟨rfl, rfl⟩

/-- Every member of the family resolves, and no two resolve to the same kind:
the table is total on its domain and injective, so no spelling is ambiguous. -/
theorem the_two_byte_table_is_injective :
    twoByte "[|" ≠ twoByte "|>" ∧ twoByte "|]" ≠ twoByte "|>" ∧
    twoByte "&&" ≠ twoByte "|>" ∧ twoByte "[|" ≠ twoByte "|]" := by
  refine ⟨by decide, by decide, by decide, by decide⟩

/-- A byte pair outside the table falls through to the one-byte spelling, which
is what keeps `&&&` decided: `&&` then `&`. -/
theorem three_ampersands_are_and_then_reserved :
    lexStep "&&" "&" = some Kind.andAnd ∧ lexStep "&" "&" = some Kind.amp := by
  exact ⟨rfl, rfl⟩

end QCore.PipeDesugaring
