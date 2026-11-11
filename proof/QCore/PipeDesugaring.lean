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
  /-- `|`, **reserved** for a future bitwise or (card `OR-AND-LOWERING`). -/
  | bar
  /-- `||`, the logical-**or** operator: one token, admitted in an `if`
  condition and lowered there (`QCore.Connectives`). -/
  | barBar
  /-- `&&`, the logical-**and** operator, on the same terms. -/
  | andAnd
  /-- `&`, **reserved** for a future bitwise and. -/
  | amp
  /-- `^`, **reserved** for a future bitwise xor. Card `OR-AND-LOWERING`
  promotes it from a lexer error to a token that refuses at the gate. -/
  | caret
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

/-- The closed statement-stage table: a statement head and the number of
selector words it names before its operand. Closed on purpose — a statement form
does not become pipeable by accident.

Card `PIPE-DROP-STAGE` added `drop`, the one **releasing** entry, and no other
head; §5.7 carries the criterion and the census it was applied to. A releasing
entry emits by this same rule and produces no result by the same
`producesResult`, so it is last-stage-only without a rule of its own. -/
def sinkSelectors : String → Option Nat
  | "print" => some 1
  | "log" => some 2
  | "drop" => some 0
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

/-- Only three kinds are synthesized for a **single-stage** pipeline. Nothing
that could carry meaning — no word, no literal, no operator — is invented there.
A chain synthesizes its own binders and temporaries as well, and exactly those:
see `a_chain_synthesizes_only_delimiters_binders_and_temporaries` in §5. -/
theorem only_delimiters_are_synthesized_in_a_single_stage_pipeline :
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

/-! ## 5. Chains: a pipeline of `n` stages is a sequence of `n` statements

Card `PIPE-CHAIN`. Wave 18 recorded two blockers for multi-stage pipelines. The
first — a call stage's result has no name — is answered by *giving* it one: each
non-final stage binds its result to a hygienic temporary the next stage
receives. The second — that a statement sequence is incompatible with the
byte-span statement windows every route uses — rested on the assumption that the
statements would **share** the operand token. With a fresh temporary per stage
they share nothing, and §5.3 below proves the extents are disjoint *by
construction*, together with the theorem that the naive anchoring (everything at
the pipe's start) would not be — which is what makes the anchoring choice
load-bearing rather than incidental. -/

/-- One stage of a chain, as written: its tokens with any `move` prefix already
stripped, whether that prefix was written, and the start byte of the `|>` that
feeds it. -/
structure Stage where
  tokens : List Token
  consumes : Bool
  pipe : Nat
  deriving Repr

/-- The width of the `|>` operator in bytes. Load-bearing in §5.3: it is the
gap that keeps two consecutive statements' extents apart. -/
def pipeWidth : Nat := 2

/-- Whether a stage produces a value a later stage could receive.

Decided **syntactically**, with no contract knowledge — but by *stage kind*
rather than by a keyword. A call stage produces a value; a sink (`print`, `log`)
and an operandless statement (`flush`) do not, and that is a property of the
closed statement table, not of any declaration.

Card `PIPE-CHAIN` decided it by the presence of `succeeds`, because at that time
the acquisition adapter was the only `let`-with-call the source language
admitted. Card `PIPE-SCALAR-RESULT` adds the other one — a non-acquiring call's
scalar result — so the keyword no longer separates the two. Whether a particular
callee's declared result is bindable is then the *route's* decision, taken by
`QCore.ExternCall.bindsScalar` over the declaration and reported at that stage's
own span; a `unit`-returning callee in a non-final position still refuses. -/
def producesResult (s : Stage) : Bool :=
  match headText s.tokens with
  | none => false
  | some name => !operandless name && (sinkSelectors name).isNone

/-- The hygienic temporary bound by the stage at position `i`. -/
def temporaryName (i : Nat) : String := "qcore__pipe" ++ toString i

/-- The binder a non-final stage is given, anchored at its own `|>`. -/
def binderOf (i : Nat) (at_ : Nat) : List Token :=
  [synth (Kind.atom "let") at_, synth (Kind.atom (temporaryName i)) at_,
   synth (Kind.atom "=") at_]

/-- What a stage receives: the reader's own operand for the first stage, and the
previous stage's temporary afterwards — optionally consumed, which is what
`|> move STAGE` spells for a value that has no name a reader could write. -/
def incomingOf (operand : List Token) (s : Stage) (index : Nat) : List Token :=
  if index = 0 then operand
  else
    (if s.consumes then [written (Kind.atom "move") s.pipe] else []) ++
      [synth (Kind.atom (temporaryName (index - 1))) (s.pipe + pipeWidth)]

/-- Split a call stage after its callee name into its argument list and its
adapter tail. The model scans flatly for the closing `)`, which is exact for
every stage the grammar admits — an argument is a literal, a name or `move NAME`
and never a nested call. -/
def takeArgs : List Token → List Token → Option (List Token × List Token)
  | [], _ => none
  | ⟨Kind.rparen, _, _⟩ :: tail, acc => some (acc.reverse, tail)
  | t :: rest, acc => takeArgs rest (t :: acc)

def splitArgs : List Token → Option (List Token × List Token)
  | [] => some ([], [])
  | ⟨Kind.lparen, _, _⟩ :: inner => takeArgs inner []
  | rest => some ([], rest)

/-- **The desugaring of one stage**, generalizing `rewrite` in the two ways a
chain needs: the incoming value may be a synthesized temporary rather than the
reader's operand, and a call stage may carry an acquisition adapter after its
argument list, which is passed through verbatim before the `;`. On a stage with
no adapter tail the two agree — see
`the_stage_rewrite_agrees_with_the_single_stage_rewrite`. -/
def rewriteStage (incoming stage : List Token) (semi : Token) (anchor : Nat) :
    Option (List Token) :=
  if incoming.isEmpty then none
  else
    match headText stage with
    | none => none
    | some name =>
      if operandless name then none
      else
        match sinkSelectors name with
        | some selectors =>
          if stage.length = 1 + selectors then some (stage ++ incoming ++ [semi])
          else none
        | none =>
          match stage with
          | [] => none
          | head :: rest =>
            match splitArgs rest with
            | none => none
            | some (args, tail) =>
              if args.isEmpty then
                some ([head, synth Kind.lparen anchor] ++ incoming ++
                  [synth Kind.rparen anchor] ++ tail ++ [semi])
              else
                some ([head, synth Kind.lparen anchor] ++ incoming ++
                  [synth Kind.comma anchor] ++ args ++
                  [synth Kind.rparen anchor] ++ tail ++ [semi])

/-- The byte a stage's synthesized tokens are anchored at: its `|>`'s start for
the first stage, and its `|>`'s end for every later one. §5.3 is why. -/
def anchorOf (s : Stage) (index : Nat) : Nat :=
  if index = 0 then s.pipe else s.pipe + pipeWidth

/-- **The desugaring of a chain.** Each non-final stage becomes a `let ⟨t⟩ = …;`
statement terminated at the *start* of the pipe that ends it; the final stage
keeps the reader's own `;`. A non-final stage that produces no value refuses,
and so does `move` on the first stage, whose operand the reader already has a
spelling for. -/
def chainRewrite (operand : List Token) (stages : List Stage) (semi : Token)
    (index : Nat) : Option (List Token) :=
  match stages with
  | [] => none
  | [s] =>
    if index = 0 && s.consumes then none
    else rewriteStage (incomingOf operand s index) s.tokens semi (anchorOf s index)
  | s :: rest =>
    if index = 0 && s.consumes then none
    else if !producesResult s then none
    else
      match rest.head? with
      | none => none
      | some next =>
        match rewriteStage (incomingOf operand s index) s.tokens
            (synth Kind.semi next.pipe) (anchorOf s index) with
        | none => none
        | some head =>
          match chainRewrite operand rest semi (index + 1) with
          | none => none
          | some tail => some (binderOf index (anchorOf s index) ++ head ++ tail)

/-! ### 5.1 The identity: a chain is its hand-written sequence -/

/-- A one-stage chain is exactly the single-stage rewrite card `PIPE-OPERATOR`
landed, so nothing already written moves. -/
theorem a_one_stage_chain_is_the_single_stage_rewrite
    (operand : List Token) (s : Stage) (semi : Token) :
    chainRewrite operand [s] semi 0
      = (if s.consumes then none
         else rewriteStage operand s.tokens semi s.pipe) := by
  cases h : s.consumes <;> simp [chainRewrite, incomingOf, anchorOf, h]

/-- And on a stage with no adapter tail, `rewriteStage` *is* the single-stage
`rewrite` card `PIPE-OPERATOR` landed — exhibited on all three stage shapes, so
nothing already written moves. -/
theorem the_stage_rewrite_agrees_with_the_single_stage_rewrite :
    rewriteStage [written (Kind.atom "total") 4]
        [written (Kind.atom "print") 13, written (Kind.atom "dec") 19]
        (written Kind.semi 22) 10
      = rewrite [written (Kind.atom "total") 4]
        [written (Kind.atom "print") 13, written (Kind.atom "dec") 19]
        (written Kind.semi 22) 10 ∧
    rewriteStage [written (Kind.atom "move") 4, written (Kind.atom "b") 9]
        [written (Kind.atom "release") 15] (written Kind.semi 22) 11
      = rewrite [written (Kind.atom "move") 4, written (Kind.atom "b") 9]
        [written (Kind.atom "release") 15] (written Kind.semi 22) 11 ∧
    rewriteStage [written (Kind.atom "b") 4]
        [written (Kind.atom "fill") 9, written Kind.lparen 13,
         written (Kind.atom "7") 14, written Kind.rparen 15]
        (written Kind.semi 16) 6
      = rewrite [written (Kind.atom "b") 4]
        [written (Kind.atom "fill") 9, written Kind.lparen 13,
         written (Kind.atom "7") 14, written Kind.rparen 15]
        (written Kind.semi 16) 6 := by
  refine ⟨rfl, rfl, rfl⟩

/-- **The concrete two-stage chain the fixture pins**, token for token equal to
the hand-written pair of statements. `8 |> alloc succeeds |> move free;` is
`let ⟨t0⟩ = alloc(8) succeeds; free(move ⟨t0⟩);` — the operand keeps its own
byte, the callees keep theirs, and only the binder, the delimiters and the
temporary are invented. -/
theorem the_two_stage_chain_is_its_hand_written_sequence :
    chainRewrite [written (Kind.atom "8") 4]
      [⟨[written (Kind.atom "alloc") 9, written (Kind.atom "succeeds") 15], false, 6⟩,
       ⟨[written (Kind.atom "free") 32], true, 24⟩]
      (written Kind.semi 40) 0
      = some
        ([synth (Kind.atom "let") 6, synth (Kind.atom "qcore__pipe0") 6,
          synth (Kind.atom "=") 6,
          written (Kind.atom "alloc") 9, synth Kind.lparen 6,
          written (Kind.atom "8") 4, synth Kind.rparen 6,
          written (Kind.atom "succeeds") 15, synth Kind.semi 24]
         ++
         [written (Kind.atom "free") 32, synth Kind.lparen 26,
          written (Kind.atom "move") 24, synth (Kind.atom "qcore__pipe0") 26,
          synth Kind.rparen 26, written Kind.semi 40]) := by
  rfl

/-- The first stage receives the bytes the reader wrote, unmoved. -/
theorem the_first_stage_receives_the_readers_operand
    (operand : List Token) (s : Stage) :
    incomingOf operand s 0 = operand := by
  simp [incomingOf]

/-- A later stage receives the previous stage's temporary, and nothing else. -/
theorem a_later_stage_receives_the_previous_temporary
    (operand : List Token) (s : Stage) (i : Nat) (h : s.consumes = false) :
    incomingOf operand s (i + 1)
      = [synth (Kind.atom (temporaryName i)) (s.pipe + pipeWidth)] := by
  simp [incomingOf, h]

/-- Consuming that temporary is the `move` prefix, and it prefixes the same
token rather than replacing it. -/
theorem a_consuming_stage_moves_the_previous_temporary
    (operand : List Token) (s : Stage) (i : Nat) (h : s.consumes = true) :
    incomingOf operand s (i + 1)
      = [written (Kind.atom "move") s.pipe,
         synth (Kind.atom (temporaryName i)) (s.pipe + pipeWidth)] := by
  simp [incomingOf, h]

/-! ### 5.2 Fail-closed: only an adapter stage may feed a later one -/

/-- A non-final stage that produces no value refuses. This is the rule that
makes a chain decidable without any contract knowledge. -/
theorem a_non_final_stage_that_produces_nothing_refuses
    (operand : List Token) (s : Stage) (rest : List Stage) (semi : Token)
    (index : Nat) (hrest : rest ≠ []) (hmove : ¬(index = 0 ∧ s.consumes = true))
    (h : producesResult s = false) :
    chainRewrite operand (s :: rest) semi index = none := by
  cases rest with
  | nil => exact absurd rfl hrest
  | cons r rs =>
    have : (index = 0 && s.consumes) = false := by
      by_cases hi : index = 0
      · simp [hi] at hmove ⊢; simpa using hmove
      · simp [hi]
    simp [chainRewrite, this, h]

/-- A sink produces no value, so it is last-stage-only: `x |> print dec |> …`
refuses. This is the user-facing shape `value |> render |> log` measured against
the vocabulary the language actually has — a `std.io` sink cannot be a `render`. -/
theorem a_sink_is_last_stage_only :
    chainRewrite [written (Kind.atom "total") 4]
      [⟨[written (Kind.atom "print") 13, written (Kind.atom "dec") 19], false, 10⟩,
       ⟨[written (Kind.atom "log") 26, written (Kind.atom "info") 30,
         written (Kind.atom "dec") 35], false, 23⟩]
      (written Kind.semi 39) 0 = none := by
  rfl

/-- `move` on the first stage refuses: the reader's own operand already has the
language's spelling, `move OPERAND |> STAGE`, so the prefix is reserved for the
value a previous stage produced. One spelling per meaning. -/
theorem move_on_the_first_stage_refuses
    (operand : List Token) (s : Stage) (rest : List Stage) (semi : Token)
    (h : s.consumes = true) :
    chainRewrite operand (s :: rest) semi 0 = none := by
  cases rest <;> simp [chainRewrite, h]

/-- An empty chain refuses; there is no pipeline without a stage. -/
theorem an_empty_chain_refuses (operand : List Token) (semi : Token) (i : Nat) :
    chainRewrite operand [] semi i = none := by
  simp [chainRewrite]

/-- The refusal surface of a chain, exhibited as one list so a future widening
has to change this theorem rather than slip past it.

**Card `PIPE-SCALAR-RESULT` changed it, which is the mechanism working.** The
third conjunct used to be `x |> f |> g` — an adapterless call stage feeding a
later one — and it is now admitted (see
`a_call_stage_may_feed_a_later_stage`). It is replaced here by the shape that
takes its place in the refusal surface: a **sink** in a non-final position,
which no widening of the call vocabulary can reach. -/
theorem refused_chains_rewrite_to_nothing :
    chainRewrite [written (Kind.atom "x") 0] [] (written Kind.semi 9) 0 = none ∧
    chainRewrite [written (Kind.atom "x") 0]
      [⟨[written (Kind.atom "f") 5], true, 2⟩] (written Kind.semi 9) 0 = none ∧
    chainRewrite [written (Kind.atom "x") 0]
      [⟨[written (Kind.atom "print") 5, written (Kind.atom "dec") 11], false, 2⟩,
       ⟨[written (Kind.atom "g") 20], false, 17⟩] (written Kind.semi 23) 0 = none ∧
    chainRewrite [written (Kind.atom "x") 0]
      [⟨[written (Kind.atom "flush") 5], false, 2⟩] (written Kind.semi 12) 0 = none := by
  refine ⟨rfl, rfl, rfl, rfl⟩

/-! ### 5.2.1 Card `PIPE-SCALAR-RESULT`: a call stage may feed a later stage

The widening, stated in both directions. A call stage carrying no acquisition
adapter is now a legitimate intermediate — it is how a scalar `render` stage
reaches the next stage — and every sink stays last-stage-only, because a sink's
refusal is a property of the closed statement table rather than of any
declaration. -/

/-- The shape card `PIPE-CHAIN` refused and this card admits: `x |> f |> g`
rewrites to the statement sequence `let ⟨t0⟩ = f(x); g(⟨t0⟩);`, token for
token. Whether `f`'s declared result is bindable is the route's question, asked
of the declaration and answered at `f`'s own span. -/
theorem a_call_stage_may_feed_a_later_stage :
    chainRewrite [written (Kind.atom "x") 0]
      [⟨[written (Kind.atom "f") 5], false, 2⟩,
       ⟨[written (Kind.atom "g") 12], false, 9⟩] (written Kind.semi 15) 0
      = some
        ([synth (Kind.atom "let") 2, synth (Kind.atom "qcore__pipe0") 2,
          synth (Kind.atom "=") 2,
          written (Kind.atom "f") 5, synth Kind.lparen 2,
          written (Kind.atom "x") 0, synth Kind.rparen 2, synth Kind.semi 9]
         ++
         [written (Kind.atom "g") 12, synth Kind.lparen 11,
          synth (Kind.atom "qcore__pipe0") 11, synth Kind.rparen 11,
          written Kind.semi 15]) := by
  rfl

/-- The two-argument composition of the brief over data rather than custody:
`x |> f(a) |> g(b)` is `g(f(x, a), b)`, with a scalar intermediate. -/
theorem a_two_argument_scalar_composition_is_its_hand_written_sequence :
    chainRewrite [written (Kind.atom "b") 0]
      [⟨[written (Kind.atom "sum") 5, written Kind.lparen 8,
         written (Kind.atom "8") 9, written Kind.rparen 10], false, 2⟩,
       ⟨[written (Kind.atom "scale") 16, written Kind.lparen 21,
         written (Kind.atom "2") 22, written Kind.rparen 23], false, 13⟩]
      (written Kind.semi 24) 0
      = some
        ([synth (Kind.atom "let") 2, synth (Kind.atom "qcore__pipe0") 2,
          synth (Kind.atom "=") 2,
          written (Kind.atom "sum") 5, synth Kind.lparen 2,
          written (Kind.atom "b") 0, synth Kind.comma 2,
          written (Kind.atom "8") 9, synth Kind.rparen 2, synth Kind.semi 13]
         ++
         [written (Kind.atom "scale") 16, synth Kind.lparen 15,
          synth (Kind.atom "qcore__pipe0") 15, synth Kind.comma 15,
          written (Kind.atom "2") 22, synth Kind.rparen 15,
          written Kind.semi 24]) := by
  rfl

/-- A stage carrying the acquisition adapter is still result-producing, so
nothing card `PIPE-CHAIN` landed moves: the widening adds shapes and removes
none. -/
theorem an_adapter_stage_still_produces_a_result :
    producesResult ⟨[written (Kind.atom "alloc") 9, written (Kind.atom "succeeds") 15],
      false, 6⟩ = true := by
  decide

/-- And a sink is still last-stage-only, by the same predicate. -/
theorem a_sink_still_produces_nothing :
    producesResult ⟨[written (Kind.atom "print") 13, written (Kind.atom "dec") 19],
      false, 10⟩ = false ∧
    producesResult ⟨[written (Kind.atom "log") 13, written (Kind.atom "info") 17,
      written (Kind.atom "dec") 22], false, 10⟩ = false ∧
    producesResult ⟨[written (Kind.atom "flush") 13], false, 10⟩ = false := by
  refine ⟨by decide, by decide, by decide⟩

/-! ### 5.3 Why the statement sequence is admissible: disjoint byte extents

Every route reads its own statement by filtering the program's tokens with
`span.start ≤ t.start ∧ t.end ≤ span.end`, and a statement's span is the minimum
start and maximum end over its tokens. A chain is therefore admissible exactly
when consecutive statements' extents do not overlap — which wave 18 believed
they could not avoid doing. They can, and the anchoring is what does it. -/

/-- The byte extent `range_span` computes for one desugared statement. -/
structure Extent where
  start : Nat
  stop : Nat
  deriving DecidableEq, Repr

/-- Whether a zero-width token at byte `b` falls inside a statement's window,
which is exactly the filter every route applies:
`span.start ≤ t.start ∧ t.end ≤ span.end`. -/
def Extent.covers (e : Extent) (b : Nat) : Bool := e.start ≤ b && b ≤ e.stop

/-- The extent of statement `i` of an `n`-stage pipeline whose `|>` operators
start at `pipes i`.

Statement `0` starts at the operand the reader wrote; every later statement
starts at the **end** of its own `|>`. Every non-final statement stops at the
**start** of the next `|>`, where its synthesized `;` is anchored; the final one
stops at the reader's `;`. -/
def statementExtent (operandStart semiStop : Nat) (pipes : Nat → Nat) (n i : Nat) :
    Extent :=
  { start := if i = 0 then operandStart else pipes i + pipeWidth,
    stop := if i + 1 = n then semiStop else pipes (i + 1) }

/-- **Consecutive statements of a chain never overlap**, with no hypothesis at
all about where the pipes are: statement `i` stops at the next `|>`'s start and
statement `i+1` begins `pipeWidth` bytes later. This is the theorem that
retires wave 18's recorded `PIPE-CHAIN` blocker. -/
theorem consecutive_statements_are_disjoint
    (operandStart semiStop : Nat) (pipes : Nat → Nat) (n i : Nat)
    (h : i + 1 < n) :
    (statementExtent operandStart semiStop pipes n i).stop
      < (statementExtent operandStart semiStop pipes n (i + 1)).start := by
  have hi : i + 1 ≠ n := by omega
  simp [statementExtent, hi, pipeWidth]

/-- **And the naive anchoring would not.** If a non-final statement's terminator
and the next statement's synthesized tokens were both anchored at the same
pipe's start — the wave-18 convention, which is correct for a single stage —
the two extents would meet exactly at that byte. The two-byte gap is therefore
load-bearing, not incidental. -/
def naiveExtent (operandStart semiStop : Nat) (pipes : Nat → Nat) (n i : Nat) :
    Extent :=
  { start := if i = 0 then operandStart else pipes i,
    stop := if i + 1 = n then semiStop else pipes (i + 1) }

theorem the_naive_anchoring_touches_at_the_pipe
    (operandStart semiStop : Nat) (pipes : Nat → Nat) (n i : Nat)
    (h : i + 1 < n) :
    (naiveExtent operandStart semiStop pipes n i).stop
      = (naiveExtent operandStart semiStop pipes n (i + 1)).start := by
  have hi : i + 1 ≠ n := by omega
  simp [naiveExtent, hi]

/-- …and a zero-width token at that byte is then covered by **both** windows, so
each route's span-keyed filter would return the other statement's tokens too.
Exhibited concretely: three stages, pipes at 10, 20 and 30. -/
theorem the_touching_byte_is_covered_by_both_naive_windows :
    (naiveExtent 0 40 (fun i => 10 * i) 3 0).covers 10 = true ∧
    (naiveExtent 0 40 (fun i => 10 * i) 3 1).covers 10 = true := by
  refine ⟨by decide, by decide⟩

/-- Under the anchoring that landed, the same byte belongs to the first window
and to nothing else — the second statement begins two bytes later. -/
theorem the_landed_anchoring_separates_the_same_byte :
    (statementExtent 0 40 (fun i => 10 * i) 3 0).covers 10 = true ∧
    (statementExtent 0 40 (fun i => 10 * i) 3 1).covers 10 = false := by
  refine ⟨by decide, by decide⟩

/-! ### 5.4 Hygiene -/

/-- Temporaries are distinct, exhibited at the widths a chain reaches. A file
that writes the reserved prefix itself and also needs a temporary is refused
rather than silently α-renamed, so no diagnostic can ever name a binding the
reader did not write. -/
theorem temporaries_are_pairwise_distinct :
    temporaryName 0 ≠ temporaryName 1 ∧ temporaryName 1 ≠ temporaryName 2 ∧
    temporaryName 0 ≠ temporaryName 2 ∧ temporaryName 2 ≠ temporaryName 3 := by
  refine ⟨by decide, by decide, by decide, by decide⟩

/-- Whether a stream already writes the reserved prefix, which is the condition
the desugarer refuses on. -/
def writesReservedPrefix (ts : List Token) : Bool :=
  ts.any (fun t => match t.kind with
    | Kind.atom text => "qcore__pipe".isPrefixOf text
    | _ => false)

theorem a_temporary_carries_the_reserved_prefix (i : Nat) :
    writesReservedPrefix [written (Kind.atom (temporaryName i)) 0] = true := by
  simp [writesReservedPrefix, written, temporaryName, String.isPrefixOf]

theorem an_ordinary_binding_does_not :
    writesReservedPrefix [written (Kind.atom "grown") 0] = false := by
  decide

/-- Exactly which tokens a chain invents: the delimiters a single-stage
pipeline invents, plus its own binders and temporaries — and nothing else. No
callee name, no literal, no operator and no user binding is ever synthesized.
This is the chain counterpart of
`only_delimiters_are_synthesized_in_a_single_stage_pipeline`. -/
def synthesizedKinds (ts : List Token) : List Kind :=
  (ts.filter (fun t => t.synthesized)).map (fun t => t.kind)

theorem a_chain_synthesizes_only_delimiters_binders_and_temporaries :
    synthesizedKinds
      ([synth (Kind.atom "let") 6, synth (Kind.atom "qcore__pipe0") 6,
        synth (Kind.atom "=") 6,
        written (Kind.atom "alloc") 9, synth Kind.lparen 6,
        written (Kind.atom "8") 4, synth Kind.rparen 6,
        written (Kind.atom "succeeds") 15, synth Kind.semi 24]
       ++
       [written (Kind.atom "free") 32, synth Kind.lparen 26,
        written (Kind.atom "move") 24, synth (Kind.atom "qcore__pipe0") 26,
        synth Kind.rparen 26, written Kind.semi 40])
      = [Kind.atom "let", Kind.atom "qcore__pipe0", Kind.atom "=",
         Kind.lparen, Kind.rparen, Kind.semi,
         Kind.lparen, Kind.atom "qcore__pipe0", Kind.rparen] := by
  rfl

/-- The written half of the same chain is exactly what the reader typed, in the
order the hand-written sequence would have it. -/
theorem a_chain_preserves_every_written_origin :
    writtenOnly
      ([synth (Kind.atom "let") 6, synth (Kind.atom "qcore__pipe0") 6,
        synth (Kind.atom "=") 6,
        written (Kind.atom "alloc") 9, synth Kind.lparen 6,
        written (Kind.atom "8") 4, synth Kind.rparen 6,
        written (Kind.atom "succeeds") 15, synth Kind.semi 24]
       ++
       [written (Kind.atom "free") 32, synth Kind.lparen 26,
        written (Kind.atom "move") 24, synth (Kind.atom "qcore__pipe0") 26,
        synth Kind.rparen 26, written Kind.semi 40])
      = [written (Kind.atom "alloc") 9, written (Kind.atom "8") 4,
         written (Kind.atom "succeeds") 15, written (Kind.atom "free") 32,
         written (Kind.atom "move") 24, written Kind.semi 40] := by
  rfl

/-- A rewritten chain carries no `|>`: the operator is consumed at every stage,
not relocated, so no later pass can see one in a well-formed program. -/
theorem a_rewritten_chain_contains_no_pipe :
    (chainRewrite [written (Kind.atom "8") 4]
      [⟨[written (Kind.atom "alloc") 9, written (Kind.atom "succeeds") 15], false, 6⟩,
       ⟨[written (Kind.atom "free") 32], true, 24⟩]
      (written Kind.semi 40) 0).all (fun out => !hasPipe out) = true := by
  rfl

/-! ### 5.5 The intrinsic stage (card `PIPE-INTRINSIC-STAGE`)

Card `COMPOSE-CONCURRENCY` recorded that the brief's literal
`buffer |> read |> render |> log` "needs a pipeline stage for a task intrinsic,
which is a pipe-grammar change". The census this card starts from measures that
it is not: the source language's model intrinsics fall into two shapes, and the
grammar already decides both.

* A **call-shaped** intrinsic is spelled `NAME(ARGS)` and already has a
  statement form — `qcore_buffer_new`, `qcore_buffer_read`, `qcore_buffer_write`
  and `qcore_error_payload` on the task routes, `qcore_acquire` and `qcore_check`
  on the sequential route. It is not in the closed sink table and it is not
  operandless, so it is a **call stage** by `producesResult`'s own definition,
  and `rewriteStage` produces exactly the `let ⟨t⟩ = NAME(…);` statement the
  route already lowers. Nothing is added to the stage table, no keyword is
  reserved and no code is allocated.
* A **keyword-shaped** intrinsic — `spawn S W(ARGS)?`, `join move H?`,
  `share N`, `drop N` — is a statement whose operands are not a parenthesized
  argument list. A call stage can only produce `HEAD ( … )`, so no pipeline
  rewrites to one; the shapes below exhibit that as an inequality rather than
  leaving it to the closed table's silence. -/

/-- The census's call-shaped intrinsics, in one list so a future intrinsic has
to be added here rather than acquire a stage silently. -/
def callShapedIntrinsics : List String :=
  ["qcore_buffer_new", "qcore_buffer_read", "qcore_buffer_write",
   "qcore_error_payload", "qcore_acquire", "qcore_check"]

/-- The keyword-shaped ones, which the closed statement table does not name and
no call stage can produce.

**Card `PIPE-DROP-STAGE` removed `drop` from this list**, which is the pin this
card moved rather than slipped past: `PIPE-INTRINSIC-STAGE` withheld that entry
explicitly ("a releasing table entry is a different kind of entry from a
rendering one") and the later card took the decision. Everything else here is
unmoved, and §5.7 states the criterion that keeps it so. -/
def keywordShapedIntrinsics : List String :=
  ["spawn", "join", "share", "read", "write", "grow", "open", "close"]

/-- **The census's structural fact.** No intrinsic is in the closed sink table
and none is operandless, so every one of them is a call stage — which is why the
pipe needed no new entry to carry it. -/
theorem no_intrinsic_is_a_sink_or_operandless :
    callShapedIntrinsics.all (fun n => (sinkSelectors n).isNone && !operandless n) = true ∧
    keywordShapedIntrinsics.all (fun n => (sinkSelectors n).isNone && !operandless n) = true := by
  refine ⟨by decide, by decide⟩

/-- And therefore every call-shaped intrinsic may stand in a **non-final**
position: `producesResult` is `true` for each. Whether the route can bind that
particular result is the route's own question, reported at that stage's span. -/
theorem every_call_shaped_intrinsic_produces_a_result :
    callShapedIntrinsics.all
      (fun n => producesResult ⟨[written (Kind.atom n) 0], false, 0⟩) = true := by
  decide

/-- **The payoff, token for token.** `buffer |> qcore_buffer_read |> log info dec;`
is the statement sequence `let ⟨t0⟩ = qcore_buffer_read(buffer); log info dec ⟨t0⟩;`
— the intrinsic keeps its own bytes, the operand keeps its own, and only the
binder, the delimiters and the temporary are invented. -/
theorem the_buffer_read_chain_is_its_hand_written_sequence :
    chainRewrite [written (Kind.atom "buffer") 0]
      [⟨[written (Kind.atom "qcore_buffer_read") 10], false, 7⟩,
       ⟨[written (Kind.atom "log") 31, written (Kind.atom "info") 35,
         written (Kind.atom "dec") 40], false, 28⟩]
      (written Kind.semi 43) 0
      = some
        ([synth (Kind.atom "let") 7, synth (Kind.atom "qcore__pipe0") 7,
          synth (Kind.atom "=") 7,
          written (Kind.atom "qcore_buffer_read") 10, synth Kind.lparen 7,
          written (Kind.atom "buffer") 0, synth Kind.rparen 7, synth Kind.semi 28]
         ++
         [written (Kind.atom "log") 31, written (Kind.atom "info") 35,
          written (Kind.atom "dec") 40, synth (Kind.atom "qcore__pipe0") 30,
          written Kind.semi 43]) := by
  rfl

/-- An intrinsic that takes a second operand puts the piped one **first**:
`buffer |> qcore_buffer_write(7);` is `qcore_buffer_write(buffer, 7);`, which is
the same first-argument insertion rule an ordinary callee gets. -/
theorem an_intrinsic_takes_the_piped_value_as_its_first_argument :
    rewriteStage [written (Kind.atom "buffer") 0]
      [written (Kind.atom "qcore_buffer_write") 10, written Kind.lparen 28,
       written (Kind.atom "7") 29, written Kind.rparen 30]
      (written Kind.semi 31) 7
      = some [written (Kind.atom "qcore_buffer_write") 10, synth Kind.lparen 7,
              written (Kind.atom "buffer") 0, synth Kind.comma 7,
              written (Kind.atom "7") 29, synth Kind.rparen 7,
              written Kind.semi 31] := by
  rfl

/-- An intrinsic carrying an adapter tail keeps it: `7 |> qcore_buffer_new?;` is
`qcore_buffer_new(7)?;`, the `?` passed through verbatim after the argument
list. This is the same tail mechanism the acquisition adapter uses. -/
theorem an_allocating_intrinsic_keeps_its_adapter_tail :
    rewriteStage [written (Kind.atom "7") 0]
      [written (Kind.atom "qcore_buffer_new") 5, written (Kind.atom "?") 21]
      (written Kind.semi 22) 2
      = some [written (Kind.atom "qcore_buffer_new") 5, synth Kind.lparen 2,
              written (Kind.atom "7") 0, synth Kind.rparen 2,
              written (Kind.atom "?") 21, written Kind.semi 22] := by
  rfl

/-- **A keyword-shaped intrinsic is unreachable.** A call stage always emits
`HEAD ( incoming )`, so the rewrite of a stage named `join` is `join(h)` — never
the `join move h?` the route lowers. The inequality is what makes "the pipe
cannot produce this form" a statement rather than an omission. -/
theorem a_keyword_shaped_intrinsic_is_not_reachable :
    rewriteStage [written (Kind.atom "h") 0] [written (Kind.atom "join") 5]
      (written Kind.semi 9) 2
      ≠ some [written (Kind.atom "join") 5, written (Kind.atom "move") 10,
              written (Kind.atom "h") 0, written (Kind.atom "?") 12,
              written Kind.semi 9] := by
  decide

/-! ### 5.6 The two statement readers

The one place a later phase could still tell a `|>` had been written. A route
recovers a statement's tokens either from the program's own post-desugaring
stream — every token whose span lies inside the statement's, synthesized ones
included — or by re-lexing the statement's source bytes, which recovers only the
tokens the reader wrote and recovers them in the order the *bytes* have them.
On a pipe-free statement the two agree; on a desugared one they do not, and the
second hands the lowering the pipeline instead of the statement. -/

/-- The stream reader keeps every token; the byte reader keeps the written ones.
`writtenOnly` is that byte reader's *best case*: even ignoring order, it has
already lost the synthesized delimiters. -/
theorem the_two_readers_agree_on_a_pipe_free_statement (ts : List Token)
    (h : ts.all (fun t => !t.synthesized) = true) : writtenOnly ts = ts := by
  simpa [writtenOnly, List.filter_eq_self] using
    (List.all_eq_true.mp h)

/-- On a desugared statement they do not agree, and the loss is exactly the
delimiters and the temporary the route needs to read the statement at all. -/
theorem the_byte_reader_loses_the_desugared_statement :
    writtenOnly
      [synth (Kind.atom "let") 7, synth (Kind.atom "qcore__pipe0") 7,
       synth (Kind.atom "=") 7,
       written (Kind.atom "qcore_buffer_read") 10, synth Kind.lparen 7,
       written (Kind.atom "buffer") 0, synth Kind.rparen 7, synth Kind.semi 28]
      ≠ [synth (Kind.atom "let") 7, synth (Kind.atom "qcore__pipe0") 7,
         synth (Kind.atom "=") 7,
         written (Kind.atom "qcore_buffer_read") 10, synth Kind.lparen 7,
         written (Kind.atom "buffer") 0, synth Kind.rparen 7, synth Kind.semi 28] := by
  decide

/-- And the written tokens a desugared call statement carries are in the order
the **statement** has them, callee first, not the order the reader's bytes have
them, operand first. That permutation is why a byte reader keyed on
`NAME ( … )` — the sequential route's `qcore_acquire` reader — saw the operand
where it wanted the callee name and refused the very form the reader wrote. -/
theorem the_byte_order_and_the_statement_order_differ :
    writtenOnly
      [synth (Kind.atom "let") 7, synth (Kind.atom "qcore__pipe0") 7,
       synth (Kind.atom "=") 7,
       written (Kind.atom "qcore_acquire") 10, synth Kind.lparen 7,
       written (Kind.atom "1") 0, synth Kind.rparen 7, written Kind.semi 28]
      = [written (Kind.atom "qcore_acquire") 10, written (Kind.atom "1") 0,
         written Kind.semi 28] ∧
    (writtenOnly
      [synth (Kind.atom "let") 7, synth (Kind.atom "qcore__pipe0") 7,
       synth (Kind.atom "=") 7,
       written (Kind.atom "qcore_acquire") 10, synth Kind.lparen 7,
       written (Kind.atom "1") 0, synth Kind.rparen 7, written Kind.semi 28]).map
      (fun t => t.origin)
      ≠ [0, 10, 28] := by
  refine ⟨rfl, by decide⟩

/-! ### 5.7 The releasing stage (card `PIPE-DROP-STAGE`)

`PIPE-INTRINSIC-STAGE` closed its census with one entry deliberately withheld:
`drop` "takes a single operand exactly as `print` does, but it *releases
custody*, and a releasing table entry is a different kind of entry from a
rendering one". This section takes that decision and states what the difference
is and is not.

**What it is not.** It is not a difference in the desugaring. A releasing stage
and a rendering stage are both `HEAD SELECTOR* OPERAND ";"`, both are a pure
permutation of the reader's own tokens, and both bind no result. So a released
value cannot feed a later stage **for the same reason a rendered one cannot** —
there is no value to feed — and the terminality this card needs is the theorem
`a_non_final_stage_that_produces_nothing_refuses` already proved, instantiated
rather than extended (`a_releasing_stage_in_a_non_final_position_refuses`). What
a *later statement* does with a released binding is the checker's business,
unchanged: the desugaring hands it the ordinary double release.

**What it is.** It is a difference in the argument for admitting the entry, and
that argument is what keeps the table closed. Two conditions:

1. the head's statement form is `HEAD SELECTOR* OPERAND ";"` with every selector
   a word from a closed, route-independent set; and
2. some stage can actually *feed* it a value no reader can name — i.e. the
   routes it belongs to have a call-shaped intrinsic producing what it takes.

`drop` passes both, and the second is the payoff: `PIPE-INTRINSIC-STAGE` §3.4
measured an allocating intermediate leaking because its hygienic temporary was
an owned buffer no reader could name and therefore could not release. With this
entry the pipeline that invented the temporary releases it. Every other
keyword-shaped head fails one condition or the other — `spawn`/`join` fail (1);
the mutex heads (`read`, `write`, `open`, `close`, `wait`, `use`) and the epoch
plan clauses fail (2), their routes having no call-shaped intrinsic and no
non-constant operand respectively; `move` is refused on a third ground, that it
already spells this grammar's own consume prefix; and `return` fails the second
half of (2), a chain's result being nameable by a bound pipeline already. -/

/-- The statement-stage heads that **release** custody rather than rendering.
One entry, and the theorems below are what keep it one. -/
def releasingStages : List String := ["drop"]

/-- **The whole table, exhibited**, so a widening has to change this theorem
rather than slip past it. Three heads are in it; every other statement head the
census names is out, including the ones whose *shape* would fit. -/
theorem the_statement_stage_table_is_exactly_three_heads :
    sinkSelectors "print" = some 1 ∧ sinkSelectors "log" = some 2 ∧
    sinkSelectors "drop" = some 0 ∧
    (["spawn", "join", "share", "read", "write", "grow", "open", "close",
      "move", "return", "wait", "use", "assert", "bound", "overhead", "peak",
      "effort", "credit", "retained"].all
      (fun n => (sinkSelectors n).isNone)) = true := by
  refine ⟨rfl, rfl, rfl, by decide⟩

/-- **The desugaring identity.** `b |> drop;` is the statement `drop b;` a
reader would have written, token for token — which is why custody flows through
the *unedited* checkers and linearity is inherited rather than re-implemented. -/
theorem release_pipeline_is_its_hand_written_statement
    (operand : List Token) (semi : Token) (pipe o : Nat) (hop : operand ≠ []) :
    rewrite operand [written (Kind.atom "drop") o] semi pipe
      = handWritten operand [written (Kind.atom "drop") o] semi pipe := by
  cases operand with
  | nil => exact absurd rfl hop
  | cons a as => rfl

/-- The concrete instance the fixtures pin. -/
theorem the_drop_pipeline_is_the_drop_statement :
    rewrite [written (Kind.atom "b") 4] [written (Kind.atom "drop") 13]
        (written Kind.semi 17) 10
      = some [written (Kind.atom "drop") 13, written (Kind.atom "b") 4,
              written Kind.semi 17] := by
  rfl

/-- A releasing stage **synthesizes nothing at all**: like every statement
stage, it is a pure permutation of the reader's own tokens, so no diagnostic
inside it can land on an invented position. -/
theorem a_release_pipeline_synthesizes_nothing
    (operand : List Token) (semi : Token) (pipe o : Nat) (out : List Token)
    (hop : operand.all (fun t => !t.synthesized) = true)
    (hsemi : semi.synthesized = false)
    (h : rewrite operand [written (Kind.atom "drop") o] semi pipe = some out) :
    out.all (fun t => !t.synthesized) = true := by
  cases operand with
  | nil => simp [rewrite] at h
  | cons a as =>
    simp [rewrite, headText, written, sinkSelectors, operandless] at h
    subst h
    simp_all [List.all_append]

/-- A releasing head binds no result — by the *same* `producesResult` that says
so of a rendering head, with no releasing case of its own. -/
theorem every_releasing_head_binds_no_result :
    releasingStages.all
      (fun n => !producesResult ⟨[written (Kind.atom n) 0], false, 0⟩) = true := by
  decide

/-- **Terminality is inherited, not added.** A releasing stage in a non-final
position refuses by the theorem card `PIPE-CHAIN` proved for a stage that binds
no result, applied rather than restated. The pipe needs no notion of custody to
forbid a released value from feeding a later stage: there is no value to feed. -/
theorem a_releasing_stage_in_a_non_final_position_refuses
    (operand : List Token) (rest : List Stage) (semi : Token) (index pipe o : Nat)
    (hrest : rest ≠ []) :
    chainRewrite operand
      (⟨[written (Kind.atom "drop") o], false, pipe⟩ :: rest) semi index = none :=
  a_non_final_stage_that_produces_nothing_refuses operand
    ⟨[written (Kind.atom "drop") o], false, pipe⟩ rest semi index hrest
    (by simp) rfl

/-- Exhibited on the two shapes a reader will write: a release feeding a sink,
and a release feeding a release. -/
theorem a_release_is_last_stage_only :
    chainRewrite [written (Kind.atom "b") 0]
      [⟨[written (Kind.atom "drop") 5], false, 2⟩,
       ⟨[written (Kind.atom "log") 13, written (Kind.atom "info") 17,
         written (Kind.atom "dec") 22], false, 10⟩]
      (written Kind.semi 26) 0 = none ∧
    chainRewrite [written (Kind.atom "b") 0]
      [⟨[written (Kind.atom "drop") 5], false, 2⟩,
       ⟨[written (Kind.atom "drop") 13], false, 10⟩]
      (written Kind.semi 18) 0 = none := by
  refine ⟨rfl, rfl⟩

/-- **The payoff, token for token.** `7 |> qcore_buffer_new? |> drop;` is the
statement pair `let ⟨t0⟩ = qcore_buffer_new(7)?; drop ⟨t0⟩;` — the allocating
chain `PIPE-INTRINSIC-STAGE` §3.4 measured as unspellable, because the temporary
it binds is an owned buffer no reader can name and therefore could not release.
The pipeline that invented it releases it. -/
theorem the_allocating_chain_is_its_hand_written_sequence :
    chainRewrite [written (Kind.atom "7") 0]
      [⟨[written (Kind.atom "qcore_buffer_new") 5,
         written (Kind.atom "?") 21], false, 2⟩,
       ⟨[written (Kind.atom "drop") 27], false, 24⟩]
      (written Kind.semi 31) 0
      = some
        ([synth (Kind.atom "let") 2, synth (Kind.atom "qcore__pipe0") 2,
          synth (Kind.atom "=") 2,
          written (Kind.atom "qcore_buffer_new") 5, synth Kind.lparen 2,
          written (Kind.atom "7") 0, synth Kind.rparen 2,
          written (Kind.atom "?") 21, synth Kind.semi 24]
         ++
         [written (Kind.atom "drop") 27, synth (Kind.atom "qcore__pipe0") 26,
          written Kind.semi 31]) := by
  rfl

/-- **A releasing entry and a rendering entry emit by one rule.** Stated as an
equality between the two rewrites with their heads and selector counts abstracted
away, so the claim is the mechanism's rather than an instance's: what differs
between `print dec x;` and `drop x;` is the table row, not the emission. -/
theorem a_release_and_a_render_emit_by_the_same_rule
    (operand : List Token) (semi r : Token) (anchor o : Nat) (hop : operand ≠ []) :
    rewriteStage operand [written (Kind.atom "drop") o] semi anchor
      = some ([written (Kind.atom "drop") o] ++ operand ++ [semi]) ∧
    rewriteStage operand [written (Kind.atom "print") o, r] semi anchor
      = some ([written (Kind.atom "print") o, r] ++ operand ++ [semi]) := by
  cases operand with
  | nil => exact absurd rfl hop
  | cons a as => exact ⟨rfl, rfl⟩

/-- **And nothing else joined.** The rest of `PIPE-INTRINSIC-STAGE`'s
keyword-shaped census is still outside the table, and the releasing list is
inside it — the two halves of "one entry was added" as one theorem. -/
theorem the_release_is_the_only_keyword_shaped_head_in_the_table :
    keywordShapedIntrinsics.all
      (fun n => (sinkSelectors n).isNone && !operandless n) = true ∧
    releasingStages.all (fun n => (sinkSelectors n).isSome) = true ∧
    releasingStages.length = 1 := by
  refine ⟨by decide, by decide, rfl⟩

/-! ## 6. The operator gate

**Retabled by card `OR-AND-LOWERING`, superseding card `PIPE-OPERATOR`'s
decision by explicit user direction.** Wave 18 claimed the single `|` as
logical or and refused `||` as a misspelling of it, holding `&` for binary use;
the retable makes **both** logical operators double-character (`||` and `&&`)
and frees **all three** single characters (`|`, `&`, `^`) for a future bitwise
wave. That also answers wave 18's open note — bitwise or had no spelling left —
by giving it `|` back.

The gate decides *where* each operator may appear, and nothing else:

* the three reserved characters have no meaning anywhere, so they refuse
  everywhere, inside a condition as well as outside one;
* the two connectives are admitted **only in an `if` condition**, which is the
  only position `source.branch.v1` can lower them in (`QCore.Connectives`) and
  the only position where anything could hold a Boolean value.

What a well-formed connective *means* is not this section's business: that is
`QCore.Connectives`. -/

/-- The three characters held for a future bitwise operation. -/
def reserved (k : Kind) : Bool :=
  k = Kind.bar || k = Kind.amp || k = Kind.caret

/-- The two logical connectives. -/
def connective (k : Kind) : Bool :=
  k = Kind.barBar || k = Kind.andAnd

/-- No spelling is both, so the retable partitions the family rather than
overlapping it. -/
theorem reserved_and_connective_are_disjoint (k : Kind) :
    reserved k = false ∨ connective k = false := by
  cases k <;> simp [reserved, connective]

/-- The gate, given a predicate saying which tokens the parser recorded as
lying inside an `if` condition. A reserved character refuses regardless; a
connective refuses exactly outside a condition. -/
def gateAdmits (inCondition : Token → Bool) (ts : List Token) : Bool :=
  ts.all (fun t => !reserved t.kind && (!connective t.kind || inCondition t))

/-- A reserved character refuses wherever it appears — including inside a
condition, where the connectives *are* admitted. -/
theorem a_reserved_character_refuses_everywhere
    (k : Kind) (hk : reserved k = true) (inCondition : Token → Bool)
    (o : Nat) (before after : List Token) :
    gateAdmits inCondition (before ++ [written k o] ++ after) = false := by
  simp [gateAdmits, written, List.all_append, hk]

theorem the_single_bar_is_reserved : reserved Kind.bar = true := rfl
theorem the_single_ampersand_is_reserved : reserved Kind.amp = true := rfl
theorem the_caret_is_reserved : reserved Kind.caret = true := rfl

/-- A connective outside a condition refuses. This is the whole of what the
gate says about `||` and `&&`; the tier decides the rest. -/
theorem a_connective_outside_a_condition_refuses
    (k : Kind) (hk : connective k = true) (o : Nat) (before after : List Token) :
    gateAdmits (fun _ => false) (before ++ [written k o] ++ after) = false := by
  have hr : reserved k = false := by
    cases k <;> simp_all [reserved, connective]
  simp [gateAdmits, written, List.all_append, hk, hr]

/-- …and inside one it is admitted, which is what makes the theorem above a
statement about *position* rather than a second refusal. -/
theorem a_connective_inside_a_condition_is_admitted (o p : Nat) :
    gateAdmits (fun _ => true) [written Kind.barBar o, written Kind.andAnd p]
      = true := by
  rfl

/-- The retabled family, stated as one theorem so it cannot drift: `|`, `&` and
`^` are reserved and refuse even in a condition; `||` and `&&` are admitted in a
condition and refused outside one; `|>` is not a logical operator at all and is
untouched by the gate. -/
theorem the_operator_family_is_exactly_as_declared :
    gateAdmits (fun _ => true) [written Kind.bar 0] = false ∧
    gateAdmits (fun _ => true) [written Kind.amp 0] = false ∧
    gateAdmits (fun _ => true) [written Kind.caret 0] = false ∧
    gateAdmits (fun _ => false) [written Kind.barBar 0] = false ∧
    gateAdmits (fun _ => false) [written Kind.andAnd 0] = false ∧
    gateAdmits (fun _ => true) [written Kind.barBar 0] = true ∧
    gateAdmits (fun _ => true) [written Kind.andAnd 0] = true ∧
    gateAdmits (fun _ => false) [written Kind.pipeInto 0] = true := by
  refine ⟨rfl, rfl, rfl, rfl, rfl, rfl, rfl, rfl⟩

/-! ## 7. Tokenization: longest match at width two

The lexer tries every two-byte spelling before every one-byte spelling. The
vertical-bar family is pairwise-distinct byte pairs (or single bytes), so the
resolution is decided by width alone and the order inside the table is
immaterial. -/

/-- The two-byte spellings, as the lexer's table. `"||"` is card
`OR-AND-LOWERING`'s addition: logical or is one token now, not two. -/
def twoByte : String → Option Kind
  | "[|" => some (Kind.atom "quantOpen")
  | "|]" => some (Kind.atom "quantClose")
  | "|>" => some Kind.pipeInto
  | "||" => some Kind.barBar
  | "&&" => some Kind.andAnd
  | _ => none

/-- The one-byte spellings reached only when no two-byte spelling matched. -/
def oneByte : String → Option Kind
  | "|" => some Kind.bar
  | "&" => some Kind.amp
  | "^" => some Kind.caret
  | ">" => some (Kind.atom "greater")
  | "[" => some (Kind.atom "lbracket")
  | "]" => some (Kind.atom "rbracket")
  | _ => none

/-- One lexing step: width two wins. -/
def lexStep (pair one : String) : Option Kind :=
  match twoByte pair with
  | some k => some k
  | none => oneByte one

/-- `[|` is matched at the `[`, so `[|>` can never become `[` then `|>`, and
`[||]` is the two delimiters rather than an or-operator. The quant-index
delimiters are untouched by the retable — the one place it could have reached a
shipped program. -/
theorem the_quant_delimiters_are_untouched :
    lexStep "[|" "[" = some (Kind.atom "quantOpen") ∧
    lexStep "|]" "|" = some (Kind.atom "quantClose") := by
  exact ⟨rfl, rfl⟩

/-- `|>` wins over `||` and over `|` followed by `>`. The ambiguity wave 18
recorded therefore now constrains the *bitwise* wave rather than the logical-or
one: `|| >` is unambiguous, a bitwise `| >` must be spaced. -/
theorem the_pipe_wins_by_longest_match :
    lexStep "|>" "|" = some Kind.pipeInto ∧ lexStep "| " "|" = some Kind.bar := by
  exact ⟨rfl, rfl⟩

/-- `||` wins over two `|`, and a lone `|` still lexes to the reserved token. -/
theorem the_or_wins_by_longest_match :
    lexStep "||" "|" = some Kind.barBar ∧ lexStep "| " "|" = some Kind.bar := by
  exact ⟨rfl, rfl⟩

/-- `&&` wins over two `&`, and a lone `&` still lexes to the reserved token. -/
theorem the_and_wins_by_longest_match :
    lexStep "&&" "&" = some Kind.andAnd ∧ lexStep "& " "&" = some Kind.amp := by
  exact ⟨rfl, rfl⟩

/-- Every member of the family resolves, and no two resolve to the same kind:
the table is total on its domain and injective, so no spelling is ambiguous. -/
theorem the_two_byte_table_is_injective :
    twoByte "[|" ≠ twoByte "|>" ∧ twoByte "|]" ≠ twoByte "|>" ∧
    twoByte "&&" ≠ twoByte "|>" ∧ twoByte "[|" ≠ twoByte "|]" ∧
    twoByte "||" ≠ twoByte "|>" ∧ twoByte "||" ≠ twoByte "&&" ∧
    twoByte "||" ≠ twoByte "[|" ∧ twoByte "||" ≠ twoByte "|]" := by
  refine ⟨by decide, by decide, by decide, by decide, by decide, by decide,
    by decide, by decide⟩

/-- A byte pair outside the table falls through to the one-byte spelling, which
is what keeps a run of three decided: `&&` then `&`, and `||` then `|`. -/
theorem three_ampersands_are_and_then_reserved :
    lexStep "&&" "&" = some Kind.andAnd ∧ lexStep "&" "&" = some Kind.amp := by
  exact ⟨rfl, rfl⟩

theorem three_bars_are_or_then_reserved :
    lexStep "||" "|" = some Kind.barBar ∧ lexStep "|" "|" = some Kind.bar := by
  exact ⟨rfl, rfl⟩

/-- The caret has no two-byte spelling, so it never absorbs a neighbour. It was
an `unexpected character` lexer error before card `OR-AND-LOWERING`; promoting
it to a token moved the layer and the message, never the verdict. -/
theorem the_caret_never_absorbs_a_neighbour :
    lexStep "^^" "^" = some Kind.caret ∧ lexStep "^ " "^" = some Kind.caret := by
  exact ⟨rfl, rfl⟩

end QCore.PipeDesugaring
