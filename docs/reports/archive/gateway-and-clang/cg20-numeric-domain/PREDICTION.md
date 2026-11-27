# CG20-NUMERIC-DOMAIN: frozen prediction

Frozen before the implementation commit, on base `b6a3dc42` (local `main` =
`origin/main`, wave-48 rounds 1–8 complete). The BEFORE arbiter run
(`scripts/measure.sh before`, binary built at `b6a3dc42`) was still in flight
when this file was committed; its totals and digests are recorded in the
report, not here, and a BEFORE that differs from the baseline below stops the
card. No AFTER run had been made.

## Baseline the BEFORE must reproduce

| Route | Totals | `resource-diagnostics.json` |
|---|---|---|
| corpus `66910c10` (`--jobs 4`) | 629 / 10 / 4,195 | `3d53a5d0…` |
| system | 44 / 2 / 188 | `79404fed…` |
| tail-pause slice | 29 / 1 / 239 / 1 Pending | `93cc660c…` |

## Census: decisions that depend on an integer value today

Read at the base, by consumer (none of them is a general C integer domain):

- **G, validity and sentinel refinements:** `legacy_descriptor_custody.rs`
  (`sentinel_literal`, an `i8` literal against `<`, `==`, `!=`), the NULL /
  `-1` splits of CG04, `legacy_literal_premise.rs` (declared literal
  spellings). Literals only, compared at their spelled value.
- **G, counts and bounds:** `legacy_counted_users.rs` (`literal_value`),
  `index_traversal.rs` (`is_positive_literal`), CG08's Clang-evaluated
  `sizeof` constants (`cast_representation::clang_constant`).
- **G, switch values (C04):** every `case` edge is kept whatever the switch
  value (Q07; CG00's `c04_switch_values_do_not_prune_cases` pins the
  provisional `Unsafe`).
- **G/T, branch correlation (C05):** none beyond NULL/validity/distinct/index
  facts (`c05_branch_correlation_is_not_tracked`).
- **T:** CG01's `typed_source_nodes::checked_binary` with the stated LP64
  premise `lp64_integer` (admitted only by the Monkey wrapper projection);
  enumerator tables (`typed_source::enum_constants`).
- **Verification:** `cycle18_callback_obligations::evaluate_integer` (the
  absorbed `CG01-CYCLE18-EVALUATOR-WIDTH`): strips every cast and evaluates
  `|`, `&`, checked `+` and unchecked unary `-` in `i128`, whatever the C
  types.
- **Lexical ABI reader:** `legacy_declared_abi.rs` reads plain `long` with an
  LP64 premise on every target (the MSVC ABI reader's report says so).
- **Core:** `scalar_types::checked`/`convert` (W48-QR-SCALAR-TYPES): checked
  arithmetic; N03–N06 BEFORE controls in `qcore-core/tests/clang_gap_core.rs`.

## What the card changes (stated before implementing)

1. `c_numeric` (gateway): the target integer model imported from the
   analysing Clang's predefined macros for the unit's flags (and a declared
   `--target`), never a fixed table; the kernel (C interpretation of the core
   table: unsigned wrap, signed overflow, `/ %` zero and `MIN / -1`, shift
   count, negative shifts, conversions); interval discharge.
2. `c_numeric_obligations` (gateway): a **new property** beside custody, the
   function's integer UB and unstated implementation-defined obligations over
   Clang's typed AST, verdict `Safe` (all discharged) or `Unknown`, never
   `Unsafe`. No custody consumer reads it.
3. `cycle18_callback_obligations::evaluate_integer` evaluates through the C
   types (each cast and operator at its Clang type) and refuses a value that
   depends on an unestablished width.
4. `legacy_declared_abi`: plain `long` / `unsigned long` is unreadable when
   the analysing Clang's target model is not LP64 (the reader's LP64 premise
   is removed for every non-LP64 target, not only cdecl positions).
5. Lean `QCore.CNumeric`, registered in `AuditCNumeric.lean`.

C04 (switch-value pruning) and C05 (branch correlation) are precision work
and are **not** implemented; their BEFORE controls stay green unchanged and
both are re-registered.

## Predictions

- **Custody, all three routes: byte-identical** to the BEFORE (status
  totals and `resource-diagnostics.json` digest). Reason: no custody
  consumer reads the new property; the ABI reader change only fires when
  Clang reports a non-LP64 target, and the corpus, system and slice run on
  `x86_64-pc-linux-gnu` (LP64); the cycle18 evaluator is not on these routes.
- **Reevaluation probes:** exit 0, unchanged output (the pinned cycle18
  constants are all `int`-range values).
- **New property, named controls** (Clang 21, both targets):
  - `x / 0` → `DivisionByZero` *Violated*, verdict `Unknown`;
  - `INT_MIN / -1` → `QuotientOverflow` *Violated*, `Unknown`;
  - `1 << 32` at `int` → `ShiftCount` *Violated*, `Unknown`;
  - `u < -1` → a `mixed-signedness-comparison` event, no obligation,
    `Safe`;
  - `2147483647L + 1L` → *Discharged* under `x86_64-pc-linux-gnu` (`Safe`),
    `SignedOverflow` *Violated* under `x86_64-pc-windows-msvc` (`Unknown`);
  - `long + unsigned int` computed at `long` under LP64 and at
    `unsigned long` under LLP64, as Clang records it, matching the Lean
    usual-conversion vectors.
- **W45-FB-SCHED-RANDOM-RANGE:** the reading will *not* establish the range
  premise (`scheduler.cap`/`scheduler.base` are read through `atoi` into the
  config with no ordering check), so the entry is re-registered, not closed;
  the new property reports `random_uniform` `Unknown` with open
  `SignedOverflow` (`max - min + 1`, `range * copies`) and
  `DivisionByZero` (`RAND_MAX / range`, `ra / copies`) obligations.
- **CG01-CYCLE18-EVALUATOR-WIDTH:** closed by change 3, no verdict move.
- **Windows:** the MSVC ABI fixtures (`memcmp`, `strncmp`, `strnlen`: `size_t`
  is `unsigned long long` on Win64) are predicted unaffected; they cannot be
  run on this host, and the report says so.
