# AOM-BINDING-POISON — pre-registered prediction

**Written and committed on the implemented tip, before any AFTER audit existed,
and not edited afterwards.** It follows card AOM-DESTROY-SIBLINGS' convention:
every prediction names the direction it expects, the mechanism it rests on, and
— where the direction is genuinely uncertain — says so instead of guessing.

## 0. What is implemented, exactly

`LegacyInstruction::Unknown` now carries `RefusedWrites`. Before this card every
refused statement executed

```rust
for pointer in state.pointers.values_mut() {
    pointer.unknown = true;
    pointer.may_be_null = true;
}
```

— it poisoned **every** binding in the state, including bindings its own text
cannot have written. `state.resources` was not touched then and is not touched
now.

`RefusedWrites::Unbounded` keeps exactly that, and is the answer for every
refusal that does **not** stand for one complete statement: `goto` to an
unresolved label, a malformed for-loop clause, `break`/`continue` outside their
construct, a stray or duplicate label, and every refused `return`, `if`,
`while`, `for`, `do`, `switch` or `case` **condition**, whose node the CFG
builder classifies without handing the tokens to `Parser::classify_simple`.

`RefusedWrites::Sparing(set)` is emitted only from `classify_simple`, whose
callers hand it the complete token run of one statement (or one for-loop init or
increment clause). The set is the closed, positive list of places the statement
spares, and a place is on it only when all three premises hold:

1. it is a plain name — not `NAME.f`, `NAME->f`, `NAME[0]` or `*NAME`, each of
   which denotes storage another lvalue can reach;
2. `frame_protected_locals` recognised it as an object of **automatic** storage
   duration declared by this body, or a formal, that is not also a file-scope
   object and that **no `&` anywhere in the body** is followed by;
3. `may_write_names` finds no occurrence of it in a write position in the
   refused text — `=` and every compound assignment, `++`/`--`, and a `.`
   member store; a subscript base, a `->` base, a by-value argument and an
   operand are reads.

Everything outside that list keeps the full poison, and the refusal itself is
untouched: the `Unsupported` diagnostic still stands, `is_path_poisoning` still
sets `unmodeled` on the path, and `classify_diagnostics` still makes the
function at best `Unknown`.

## 1. Structural claims — expected to hold with no exception

These are not statistical expectations; each rests on a code path, and a single
counterexample refutes it.

* **P1. No function anywhere in the 185-unit corpus moves to `Safe`.** The
  repair only changes the transfer of `LegacyInstruction::Unknown` nodes. A
  function containing one carries an `Unsupported` diagnostic, and
  `classify_diagnostics` maps `Unsupported` to at least `Unknown`. A function
  with no refusal has no such node, so its own analysis is bit-for-bit
  unchanged.
* **P2. No W4 summary gets stronger.** Every path through a refusal sets
  `state.unmodeled` (via `is_path_poisoning` on the `Unsupported`), and
  `observe_residual` maps `unmodeled` to `ExitCustody::Unknown`. So the states
  this card changes can publish no custody claim at all, and a caller cannot be
  moved by a strengthened callee summary.
* **P3. No unit is promoted.** 2 `Safe` / 42 provisional `Unsafe` / 137
  `Unknown` / 4 data-only stays as a *floor*: nothing becomes `Safe`. Movement
  in the `Unknown` ↔ `Unsafe` direction is possible and predicted below.
* **P4. `summary_conflict` stays 0** in all 136 reporting units.
* **P5. The contract corpus does not move**: 73 modules, 18 manifest-selected,
  39 v1 declarations, 145 rejected. This card lands no `.qcore` module.
* **P6. Both audit runs exit 0**, zero gateway failures in all six scopes, 185
  units, and the BEFORE run reproduces card AOM-DESTROY-SIBLINGS' AFTER
  artifacts — the authoritative baseline per that report's §12 — byte for byte
  in `diagnostics.json` and `chunk-contracts.json`.

## 2. The named target — `flb_input_chunk_get_direct_routes`

Baseline (card AOM-DESTROY-SIBLINGS' AFTER): provisional `Unsafe`, **92**
diagnostics, **1** leak leaf (L19544), **12** `unknown_alias: unresolved
resource identity \`result\``.

`result` is declared `struct flb_chunk_direct_route *result;` in that body; the
only `&` in the body is `&view` in `input_chunk_metadata_view(ic, &view)` (every
other `&` is bitwise), so `result` is frame-protected. The refused statements
between the acquisition at L19357 and the thirteen call sites — the identifier
loop's `result[index].id = read_id;` and its siblings, and the label and plugin
loops — mention `result` only as a **subscript base**, which `may_write_names`
reads as a read.

**PREDICTED:**

* the **12** `unknown_alias: unresolved resource identity \`result\`` leaves are
  **removed**;
* the six leak leaves AOM-DESTROY-SIBLINGS retired at L19384, L19392,
  **L19399**, L19434, L19489 and L19505 **stay retired, and their mechanism
  converts from withdrawal-by-refusal to a real discharge.** This is the
  *primary* arm of this card: `CONSTRUCTOR-RESIDUALS-LEAK-271` was decided by
  that card by refusal, leaving the declaration's discharging arm unexercised
  anywhere in the corpus; the prediction is that it now fires. The leak *count*
  for those six does not move — they were already gone — so the payoff is a
  change of mechanism, not of number;
* **L19544 stays**: the success exit, where `*routes = result` is refused
  (`store through \`*routes\` without an out-parameter contract`) and no destroy
  call runs. `* routes = result ;` writes `routes`, not `result`, so the
  sparing rule does not touch this leaf either;
* the function stays provisional **`Unsafe`** on that one leaf; diagnostics
  **92 → 80**.

**Recorded as NOT expected:** L19544 retiring; the function moving
`Unsafe → Unknown`; any leaf being added in this function.

`flb_input_chunk_map`'s L18929 `unknown_alias: unresolved resource identity
\`direct_routes\`` is predicted to **stay**: that body contains
`&direct_routes`, so premise 2 removes the name from the frame set and the full
poison still applies there.

## 3. Corpus totals — the honest uncertainty

Two mechanisms move in **opposite** directions and both are real:

* **Withdrawing (fewer accusations).** A modelled release whose actual was
  poisoned now resolves and discharges, so a residual that used to survive to an
  exit retires.
* **Accusing (more accusations).** `mark_unresolved_effect` marks **every**
  resource in the state when the pointer it is handed is `unknown`, and
  `report_leaks` skips any site so marked. That blanket marking is what a
  poisoned actual used to trigger, and it suppressed leaks for resources
  entirely unrelated to the call. With the binding resolved, only that binding's
  own targets are marked, so leaks that were suppressed by an unrelated call
  come back.

**PREDICTED:**

* `unknown_alias` **falls substantially** — this is the direct product of the
  repair. Predicted range: a fall of **200 to 1,500** from 4,421.
* `unsupported` is **unchanged to within ±5** of 49,781. The refusals themselves
  are not touched; the only `Unsupported` this card could move is the
  `alias set exceeded 8 targets and was widened` row, whose alias sets can
  differ now that fewer bindings are poisoned.
* `leak` **moves, and I predict a net increase**, because the accusing mechanism
  is unconditional (it fires wherever a refusal preceded a modelled release of
  *anything*) while the withdrawing one needs the release to be both modelled
  and reached. Predicted range: **266 → between 266 and 450**. A *decrease* is
  possible and would refute this arm; a value above 450 is possible and would
  mean the blanket suppression was carrying far more of the corpus than this
  card expected.
* **Unit and function status moves are predicted in the `Unknown` → `Unsafe`
  direction** (new leaves) and in the `Unsafe` → `Unknown` direction (retired
  leaves), with the first predicted to dominate. **Zero** moves to `Safe` (P1,
  P3).

**If the measured diff is large and heterogeneous,** the repair will be bounded
— by narrowing the admitted refusal classes — and **re-measured**, with both
measurements reported. The card's own instruction is that an argued smaller
repair beats an unattributable big one, and the boundary in §0 is already the
first such narrowing: conditions and construct-level refusals are outside it.

## 4. Pinned expectations already moved, deliberately, before this run

Three workspace tests were re-pinned while implementing, each in the *accusing*
or neutral direction, none weakening a gate. They are listed here so the AFTER
measurement cannot be mistaken for their cause:

1. `legacy_c::tests::braced_declarations_retain_following_cleanup_without_certifying_their_effects`
   — was: a following double cleanup reports `UnknownAlias`; now: it reports
   **`DoubleRelease`** and the function is **`Unsafe`** rather than `Unknown`.
   Strictly more accusing. The `braced` refusal and the non-`Safe` verdict are
   re-asserted.
2. `c03_neutral_frame_recursion_probe::undeclared_neutral_recursion_leaves_the_component_and_its_callers_unknown`
   — the caller no longer reports `unknown_alias` for a pointer an uncontracted
   call received **by value**. Every verdict in that probe is unchanged.
3. `publication_custody::a_publishing_helper_no_longer_launders_its_callers_release`
   — the same shape: the collateral `unknown_alias:h` leaves the diagnostic
   vector; the uncontracted-call refusal and both `Unknown` verdicts stay, so
   the helper still does not launder its caller's release (constraint 10).

## 5. Lean

A Lean module is predicted to be **needed** — unlike AOM-DESTROY-SIBLINGS, this
card changes the Rust admission machinery — and to state the fail-closed
directions as theorems: `Unbounded` poisons everything (the pre-repair relation
is the degenerate case of the new one), a spared place is never one the refusal
may write, and the refusal's own status effect is unchanged.
