# CG09-TYPED-SOURCE-CONTROL: route T's control statements, admitted per consumer

Card CG09 of the [Clang gap card queue](../../plans/clang-gap-cards/README.md)
(wave 48, round 7).

- **Base:** `fe9c5b48`, main after CG14's integration. The brief named
  `60cb0895` or later.
- **Scope:** gap entries C01, C02 and C03, with evidence Q06, Q07 and Q12.
- **Route:** **T**, the bounded typed-source importer
  `crates/qcore-c-gateway/src/typed_source.rs`. Route G is unchanged.
- **Rule:** [gateway 11l](../../agents/gateway.md#constraint-11l).
- **Prediction:** frozen before any importer change was committed
  ([PREDICTION.md](PREDICTION.md), commit `aa7be23f`).

## 1. What changed

**T edges, not G's CFG.** The brief preferred lowering T's statements through
the gateway's existing CFG. That was declined. G's CFG compiles to custody
instructions (`LegacyInstruction`), while T's product is a finite set of typed
operation paths (`SourcePath`), so there is no shared representation to lower
into. T therefore gains its own edges.

**Reading.** The importer reads:

- `switch` / `case` / `default`, with fallthrough;
- `break` and `continue`;
- forward `goto` and labels;
- `while`;
- a `for` that is not a canonical scan;
- a `do` whose condition is not the literal zero.

The new logic is in `typed_source_control.rs`, declared as a child module of
`typed_source.rs` so that it sees the importer's private `Stmt`, `Expr` and
`Path`. `typed_source.rs` gains thin hooks:

- three `Stmt` variants (`Switch`, `Label`, `Jump`) and the `Path::jump` field;
- the block-entry, skip, `do … while (0)`, `switch`/`label`/`jump` and derive
  hooks;
- the `_` arm of `Importer::statement`;
- the canonical scan moved into `Importer::scan_loop`, unchanged apart from
  indentation and the destructuring of its five parts;
- `MAX_DEPTH` and `MAX_NODES` named;
- the entry points `import_source_functions_admitting_control` and
  `import_source_functions_from_profile_admitting_control`.

The older entry points delegate to these with the legacy admission.

**Reading is not admission.** Every importer call now also passes a
`ControlAdmission`, the consumer's list of the forms it decides.

- It is validated against `CONTROL_TABLE` before any function is read. An
  unknown spelling, a duplicate row or a **back-edge** form fails the whole
  call.
- A form the consumer does not list is refused by name, for example
  `` typed-source control `switch` is not admitted by the legacy typed-source
  consumer ``.
- `ControlAdmission::LEGACY` lists nothing.

**Back-edges are refused, everywhere.** A general loop or a `goto` to a label
already read closes a cycle. T's paths are finite, so a cycle is admissible
only with an invariant or summary the consumer supplies. T can check none (card
CG27 owns them), so no admission can list one. The refusal names the form, for
example `` typed-source control `loop while` has a back-edge, and the legacy
typed-source consumer supplies no invariant or summary for it (CG27) ``. A `for`
that fails the canonical scan appends the scan shape's own reason.

**Acyclic forms are lowered into T's existing vocabulary.**

- **`switch`.**
  - The selector is evaluated **once**. Its type is Clang's promoted integer
    type.
  - Each case label is Clang's constant expression under Clang's own
    `IntegralCast`, read through T's expression reader and never evaluated on
    the host. It must read nothing and call nothing.
  - Each label is an entry path. A `case` carries `Guard(selector == c,
    taken)`. `default` carries every case guard not taken. With no `default`,
    one more path carries every case guard not taken and runs nothing.
  - From its entry, a path runs every later statement (fallthrough) until a
    `break`, which the `switch` takes.
- **`do … while (0)`.** It takes `break` before its condition, and takes
  `continue` into it (a guard on the literal zero, not taken). This is why
  `continue` is acyclic in route T: every real loop is refused at its own node
  first.
- **Forward `goto`.** Statements are skipped until a block resumes the path at
  its label.

**Lifetimes.** Every path that leaves a block ends that block's locals
(`LocalEnd`), innermost first. That holds whether it leaves by the block's end,
by `return`, or by a jump. It follows from the existing block-exit code, which
now also runs for jumping paths.

A new check, `ControlAdmission::check_local_lifetimes`, runs under a consumer
that lists any control form. It refuses any path that uses a local's storage
after its `LocalEnd`, on every path, jump or not.

The legacy admission skips it. The first gate showed why: the check had run
for every consumer, and it flipped the pinned
`opaque_calls_and_indirect_writes_invalidate_escaped_local_values`. That test
states that T's projection *retains* ended storage (`{ int y = 2; p = &y; }
if (*p)`) for the legacy consumer to decide. Keeping it keeps every legacy
import byte-identical to base.

**Refused shapes.** Each of these is refused by name
(`CG09-CONTROL-SHAPE-RESIDUALS`):

- a `goto` whose label no enclosing block reaches later, such as a jump into an
  `if` arm, a loop or a `switch` body;
- a `goto` that skips a declaration into its own scope;
- a declaration or an ordinary label at a switch body's top level;
- a case label below that level;
- a GNU case range;
- a selector that is not a promoted integer;
- a label on a declaration.

**Budgets.** The budgets are unchanged: 64 paths, 512 operations, depth 64 and
2,048 nodes. Every new path goes through the same limits, and exceeding one
refuses the function. A 70-case `switch` is refused as
`callback path limit exceeded`.

## 2. Consumers decide

| Import site (consumer) | Control admission | Why |
|---|---|---|
| `import_monkey_wrappers` | **`MONKEY_WRAPPER_CONTROL`**: `switch`, `break`, `continue`, `goto` | Its only reader is `pinned_route`'s projection report: "opaque calls and custody Unknown", with `monkey-source.json` marked `source_projection_only`. It certifies nothing, so a lowered path discharges nothing. |
| The other 19 production sites (CG01's table: the Monkey callbacks, `observed_wait_inputs`, `callback_task_handoff`, call-use, counter, teardown/field-destructor, the close compositions, cleanup replay) | LEGACY | CG01's census found readers there that accept input undecided. For example, `cycle18_callback_obligations::timeout_steps` ignores guards and writes while discharging `AllPathExecutionCoverage`, and `field_destructor`'s replay reads any non-`==` binary as `!=`. A lowered form adds only operations those readers already see (guards, `LocalEnd`), but in new path shapes: several not-taken guards on one path, and fallthrough paths that repeat a later case's operations. No reader was read for those shapes, and a LEGACY admission needs no justification beyond that. |

- **Back-edge forms:** refused in every consumer, including the wrapper
  projection.
- **Size:** 0 functions (§4). The per-site census is the existing follow-up,
  `CG01-T-CONSUMER-ADMISSIONS`, which is amended.

## 3. Cited claims

| Claim | Reproduced? |
|---|---|
| C01: T has no `WhileStmt`; `for` is limited to canonical scans; `do` to a literal zero (Q12) | **Yes.** CG00's `c01_t_has_no_general_loops` passed at base: `unmodeled callback statement WhileStmt`, `scan body requires one conditional early return`, `do loop requires a literal zero condition` ([log](artifacts/before-controls.log)). |
| C02: `switch`/`case`/`default` absent from T (Q12) | **Yes.** `c02_t_has_no_switch` passed at base: `unmodeled callback statement SwitchStmt`. |
| C03: `goto`/label/`break`/`continue` absent from T (Q12) | **Yes.** `c03_t_has_no_jumps` passed at base: `unmodeled callback statement GotoStmt` and `…BreakStmt`. |
| Q06/Q07: G has general loops, `switch` and jumps | **Yes.** The same tests' G rows (`while`, `switch`, `goto` fixtures) are `Safe` at base and after. G is unchanged. |
| Q12 line range `798–960` | **Approximate.** At base, `Importer::statement` spans 978–1148. The content is as quoted. |

The BEFORE run used base's `typed_source.rs` and `clang_gap_phase1.rs`,
checked out over the card's tree, and exited 0 with 3 of 3 passing. The
flipped controls are named in §5.

## 4. Measurements

AFTER_MEASUREMENTS

## 5. Evidence

**Rust.** `crates/qcore-c-gateway/tests/unit/source_control.rs` has 11 tests:

- the legacy consumer refuses each form by name, and the canonical scan and
  `do … while (0)` are unchanged;
- every back-edge is refused in an admitting consumer, and invalid admissions
  fail the import;
- the selector is read once, with an unsigned selector converting by Clang's
  cast;
- an inner `break` leaves only the inner `switch`;
- `do … while (0)` takes `break` and `continue`;
- a `goto` out of two nested scopes ends `y`, `x`, `r` in that order, before
  the label's `return`;
- the unsupported shapes are refused by name, with a positive control (a jump
  out of a `switch` into a later block);
- the path budget;
- the Lean binding.

**The three refusal controls the brief names:**

1. **A `goto` out of a scope holding a live local.**
   `int *p = 0; { int x = 1; p = &x; if (a) goto out; } out: return *p;` is
   refused as `local storage used after its lifetime ended`. Without the later
   use it is admitted, and `x` ends on both paths. The same dangling use with
   no jump is refused under an admitting consumer, and projected under the
   legacy one.
2. **A `while` with no decidable invariant in a consumer that needs one.**
   The admitting test consumer refuses `while`, general `for`, general `do` and
   backward `goto` by name. An admission listing `loop while` or
   `goto backward` fails validation.
3. **`switch` fallthrough into a case that releases twice.**
   `switch (a) { case 1: q_free(p); case 2: q_free(p); break; }` is refused by
   name under the legacy admission, which every release-deciding consumer uses.
   Under the admitting projection, the `case 1` path carries both
   `q_free(p)` calls on the same parameter, so no deciding consumer could miss
   the second.

**Flipped controls** (CG00's `tests/clang_gap_phase1.rs`):

| Before | After |
|---|---|
| `c01_t_has_no_general_loops` | `c01_t_reads_general_loops_and_refuses_each_back_edge_by_name`: the three loops refused by name under the legacy and an admitting consumer |
| `c02_t_has_no_switch` | `c02_t_lowers_switch_where_the_consumer_admits_it`: legacy named refusal; admitted as 2 paths |
| `c03_t_has_no_jumps` | `c03_t_lowers_forward_jumps_where_the_consumer_admits_them`: `goto` legacy named, admitted as 2 paths; `break` in a loop refused with the loop, and in a `switch` or `do … while (0)` admitted |

The G assertions in each are unchanged.

A fourth pin flips, in `compiler/tests/unit/monkey_source.rs`:
`single_iteration_macros_do_not_erase_control_or_terminal_failures` asserted
that the wrapper import refuses `do { break; } while (0)` and
`do { continue; } while (0)`. The wrapper projection now admits both. `break`
imports as one path with no condition guard, and `continue` as one path whose
literal-zero guard is not taken. The Monkey callbacks, which use the legacy
admission, refuse each by name. `while (0) {}` stays refused, now by name as a
back-edge. `quirin-source-analysis`'s
`monkey_wrapper_control_admits_exactly_the_acyclic_forms` checks that the
wrapper lists exactly the acyclic rows of `CONTROL_TABLE`.

**Lean.** `QCore.TypedSourceControl` is registered in the new
`AuditTypedSourceControl.lean`, with 31 theorems, no `sorry`, and only the
standard axioms (`propext`, `Quot.sound`).

- *Unlisted statements refused:*
  - `unlisted_statement_refused`;
  - `while_is_a_loop`, `unscanned_for_is_a_loop`, `nonzero_do_is_a_loop` and
    `scan_for_is_legacy`;
  - `switch_is_read`, `indirect_goto_refused` and
    `case_outside_switch_refused`;
  - on the admission side, `legacy_admits_nothing`, `unlisted_form_refused`
    and `invalid_admission_refuses`.
- *A back-edge without an invariant is refused:* `back_edge_refused`,
  `loops_refused`, `cycles_are_back_edges`, `acyclic_forms` and
  `valid_row_acyclic`.
- *A jump out of a scope ends its locals:*
  - `jump_out_ends_locals` and `inner_scope_ends_first`;
  - `jump_then_use_refused`, over the lifetime checker, with
    `ended_use_refused`, `lifetimesOk_append`, `endedAfter_keeps` and
    `endedAfter_ends`.
- *`switch` fallthrough composition:*
  - `fallthrough_composes`: before the first `break`, the run from an entry
    is the concatenation of the later cases' statements;
  - `break_stops`, `case_entry_lowered` and `no_default_path`;
  - `switch_vectors_hold`, over 6 vectors, one of them the double release.
- *Budget exhaustion is never Safe:* `exhausted_unknown`,
  `exhausted_never_safe` (under the join `Safe < Pending < Unknown < Unsafe`)
  and `one_path_too_many`.

**Binding.** The module is mechanically bound.
`cg09_lean_vectors_match_the_rust_tables` reads `controlTable`,
`legacyStatements`, `controlStatements` and `budgets` from the `.lean` file and
compares them with `CONTROL_TABLE`, `LEGACY_STATEMENTS`, `CONTROL_STATEMENTS`
and `MAX_PATHS`/`MAX_OPERATIONS`/`MAX_DEPTH`/`MAX_NODES`. It also parses the 6
`switchVectors`, renders each body as C, imports it through T with an
admitting consumer, and asserts that the importer's paths equal the vector's
expected paths: the same guards and the same calls, in order. The
Clang-JSON-to-statement reading remains a review premise.

## 6. §8 axes

| Axis | Covered (under an admitting consumer) | Stays refused |
|---|---|---|
| Statement | `switch`/`case`/`default` with fallthrough, `break`, `continue` (out of `do … while (0)`), forward `goto` and labels | `while`, a general `for` or `do`, backward `goto`: every consumer (CG27); indirect `goto` (generic) |
| Scope | a jump out of any number of blocks; a jump into a later block with no skipped declaration | a jump into an `if` arm, loop or `switch` body; skipping a declaration into its scope |
| Selector type | promoted integer types (enum and `char` selectors are promoted by Clang) | any other |
| Case label | integer, enumerator, negated and CG01 literals, under Clang's conversion | GNU ranges; a label below the body's top level |
| Evaluation order | selector evaluated once, before every guard; statements in source order | nothing new |
| Lifetime | `LocalEnd` on every exit, innermost first | a use of ended storage |
| Budget | unchanged | exceeding any budget |
| Boundary, evidence, verdict | no verdict moves; the one admitting consumer certifies nothing | 19 consumers, by name |

## 7. Gate

GATE_TABLE

## 8. Follow-ups

- **Retired: `CG09-TYPED-SOURCE-CONTROL`.** Every DISCHARGE clause holds:
  - BEFORE reproduces the cited refusals;
  - the prediction is frozen;
  - AFTER has no moving function;
  - the card absorbs no follow-up;
  - there are Rust refusal controls and audited, mechanically bound Lean.

  Loops are read and refused by name. Admitting them needs an invariant or a
  summary, which is CG27's scope (C06). The brief allows the consumer to
  refuse the loop.
- **Registered: `CG09-CONTROL-SHAPE-RESIDUALS`.** The refused shapes of §1.
  Size: 0 functions.
- **Amended:**
  - `CG01-T-CONSUMER-ADMISSIONS`: the same 19 sites refuse the 4 acyclic
    forms by name. The census still to do and the new path shapes are named.
    Size: 0 functions.
  - `CG27-LOOPS-RECURSION-BUDGETS`: route T reads its loops and refuses each
    back-edge by name. A T invariant or summary lands there as a checked
    admission class.
- **The ledger stays at 666 entries** (one retired, one registered), with
  `dimensions.named_followups` equal to the list length. The edits are made by
  [scripts/update_ledger.py](scripts/update_ledger.py).
