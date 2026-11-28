# K2-BUILTIN-BIT-COUNTS: a bit-count builtin under a nonzero guard

Card of round 41 (brief: option (a) of the user's decision of 2026-11-28),
base `287dce8ed` (r86). Follow-up `K2-BUILTIN-BIT-COUNTS`, registered by
[K2-NO-TYPED-ACCESS](../archive/wave-48/k2-no-typed-access/README.md) §2.
Gateway rule [94](../../agents/rules/94.md); Lean `QCore.BitCountBuiltins`.

**Decision (verbatim from the brief).** "USER DECISION 2026-11-28 on
K2-BUILTIN-BIT-COUNTS: option **(a)**, obligation-bearing: call =
`__builtin_c{l,t}z{,l,ll}(x)` and (path proves x != 0) implies
custody_neutral(call). Rule 10s stands: a node with a C fault case is never
custody-neutral without its obligation discharged."

## 1. Probe (commit `089018a58`)

[probes/](probes/): one function `w` per line over the generic `d_*`/`k_*`
fixture contracts, through the scratch harness
[zz_probe.rs.txt](probes/zz_probe.rs.txt) ([build.sh](probes/build.sh),
[probe.sh](probes/probe.sh)).

* [probe1](probes/probe1.txt) → [base](probes/probe1.base.out) (the base
  binary built from `287dce8ed`): every guarded and unguarded form of every
  width is `Unknown`. A whole-call or statement call gives ``call to
  uncontracted function `__builtin_clz` `` (the brief's base refusal); a call
  under `?:`, a cast or `>>` gives the return position's ``unmodeled call,
  aggregate or indirect access in return expression`` (the text both
  production rows carry on `current`, K2-NO-TYPED-ACCESS's re-census:
  `snappy.c:254` for `log2_floor`, `lz4.c:589/603/614/650` for
  `LZ4_NbCommonBytes`).
* [probe2](probes/probe2.txt) → [base](probes/probe2.base.out): the same
  shapes with a pure contracted stand-in (`p_clz`, [extra.qv](probes/extra.qv):
  `resource_borrowed(x)`, `resource_return_none()`) are `Safe` in every
  shape, the loop included. So the refusal is the missing contract alone:
  once the call is read as the identity on custody, every older route
  decides the rest.
* [probe3](probes/probe3.txt) → [base](probes/probe3.base.out): the near
  misses, all `Unknown`. These base answers were taken from the card tree
  with the hook forced to the base environment (`bit_count_environment =
  None`), which is base behaviour: the hook is the module's only entry.

## 2. Where the builtins are modelled

Not in a provider: `__builtin_clz` and its kin are the compiler's, declared
by no header and by no library, and no provider models any compiler builtin.
The gateway models them in its own tables: `__builtin_expect` in
`legacy_condition_desugar`, the `__builtin_mem*` whole-object writers in
`constructed_state::WHOLE_WRITERS`, the type operators in
`legacy_return_call::NOT_A_CALLEE`. Rule 94's table is
`bit_count_builtins::BUILTINS`: the six names, each operand type, and the one
C fault case (zero operand). The contract a discharged call is read with
(`neutral_contract`: the parameter `Borrowed`, `ResourceReturnMode::None`, no
other clause) is built there and enters only the environment of the function
whose every site is discharged; it is never in the unit's environment, so no
caller and no ABI gate sees a declaration of a builtin (the measured row's
`contract_dependencies` name no declaration, §5).

## 3. Rule 94

`crates/qcore-c-gateway/src/bit_count_builtins.rs`; one delimited hook at
the top of `legacy_c::build_function_cfg_with`, the single entry of every CFG
build (W4 derivation, validation and the recursion abstraction included).

For each bit-count call in the function's Clang AST
(`BuiltinFnToFnPtr` naming a table row), `site_facts` reads:

1. **the operand**: one named object through parentheses and integer
   conversions to an unsigned type of no lower rank (`widening_keeps_nonzero`:
   every value of the source is below `2^N` in magnitude, so its value
   modulo `2^N` is nonzero when it is);
2. **stability**: a parameter or an initialised automatic local of a
   non-volatile integer type that no `&`, `=`, `op=`, `++` or `--` in the
   function touches, whose initializer does not hold the call, in a function
   without `asm`. Its value is fixed from its declaration on, so the value a
   guard tested is the value the call reads;
3. **the guards on the path**: the selected arm of `?:`, the right operand of
   `&&` / `||`, a branch of `if`, or the statements after an earlier
   `if (C) S` of an enclosing block whose `S` always returns. `proves` reads
   `C` over the object (`x`, `x != 0`, `0 != x`, `x > 0`, `0 < x`; `x == 0`,
   `x <= 0`, `!x` on the false edge; `&&` true / `||` false through either
   operand; the read through any integer conversion, which maps zero to
   zero). The statement guards need structured flow: no label or `goto` in
   the function, no `case`/`default` inside the guarded region.

`decide` admits a site with a named, stable operand and a guard that proves
`x ≠ 0` on its edge. When every site is admitted and the analysed tokens
and the AST agree on the sites (callee and line, as a multiset), the CFG is
built with `neutral_contract` under each called bit count. Otherwise nothing
changes: every site keeps the base refusal byte for byte, and the reason is
written only to the typed-path parity stream (position `bit-count-builtin`).
The per-function all-or-nothing reading is narrower than per site (a guarded
call beside an unguarded one stays refused) and was chosen because a
per-site hold would need the token stream and every typed route that reads
a builtin callee to agree on which site is held; no measured function mixes
the two (§5: every bit-count row has either only guarded or only unguarded
sites).

**`assert` is no guard.** `LZ4_NbCommonBytes`'s `assert(val != 0)` is
`((void)0)` under `NDEBUG` (the analysed configuration); with assertions on
it is a call (`__assert_fail` on the false edge), and a guard is only a
branch the rule reads.

## 4. Lean

`proof/QCore/BitCountBuiltins.lean`, audit `proof/AuditBitCountBuiltins.lean`
(21 theorems; axioms `propext`, `Classical.choice`, `Quot.sound` at most, no
`sorry`, no project axiom). The builtins are modelled with their undefined
case (`eval` is `none` at `0`: `undefined_at_zero`, `defined_off_zero`); the
neutral contract is the identity on any custody state; `proves_sound`: a
condition that `proves` `x ≠ 0` on an edge and took that edge has `x ≠ 0`,
over every C value of the other subexpressions; `admitted_exactly_when`,
`an_admitted_site_has_a_nonzero_operand` and `an_admitted_call_is_defined`
give the admission only under the nonzero premise, and the refusals
(`an_unguarded_site_is_refused`, `an_unstable_object_is_refused`,
`a_narrowed_operand_is_refused`, `a_statement_guard_needs_structured_flow`).
`builtinNames`, `proofVectors` and `decisionVectors` are compared line for
line by `tests/unit/bit_count_builtins.rs`.

## 5. Measurement

Opt-in `current --monkey-linked` (181 units) and the fluent-bit-system route
over this host's preparation `private/fb-046accf6` (Fluent Bit 046accf6),
base `287dce8ed` against tip `edf56d4e9`, each from its own `git archive`
tree and release binary (base `43dabbee…`, tip `a73b562c…`,
[artifacts/binaries.sha256](artifacts/binaries.sha256)); scripts
[scripts/run.sh](scripts/run.sh), [scripts/opt-in-current.sh](scripts/opt-in-current.sh)
(FB-RETRY-OR6-ELEMENT-IDENTITY's, the provider-selection script's path moved
to its archive), [scripts/artifacts.sh](scripts/artifacts.sh); raw reports in
`private/k2bc/m/{run,sys}-{base,tip}/`, digests in
[artifacts/raw-sha256.txt](artifacts/raw-sha256.txt). All four runs exit 0.
A first opt-in attempt of each side refused before analysis (`a contract load
path is given but no provider is selected`: the copied script named the
provider-selection script at its pre-archive path); it is kept in
`private/k2bc/m/run-{base,tip}.attempt1-no-providers/` and is not evidence.
Pinned routes (`quirin-audit measure --declared`) cannot run on this host (no
`build/cycle19` inputs); nothing is recorded for them.

| Route | Safe | Pending | Unknown | Unsafe |
|---|---:|---:|---:|---:|
| opt-in current, base | 1,209 | 2 | 3,586 | 7 |
| opt-in current, tip | **1,210** | 2 | **3,585** | 7 |
| system, base and tip | 124 | 0 | 320 | 4 |

**Every moved and changed row**
([artifacts/current-moved.jsonl](artifacts/current-moved.jsonl),
[artifacts/current-changed.txt](artifacts/current-changed.txt),
[artifacts/current-bit-count-rows.json](artifacts/current-bit-count-rows.json)):

* **`log2_floor`** (`snappy.c`) **Unknown → Safe**, as predicted. Base leaf:
  `unmodeled call, aggregate or indirect access in return expression` at its
  `return n == 0 ? -1 : 31 ^ __builtin_clz(n);`. Justification: the one
  bit-count site is the `?:`'s false arm, `n == 0` false proves `n ≠ 0` there,
  `n` is a `u32` formal never written and never address-taken, and the body
  has no other site; the call is then the identity on custody and the row's
  remaining obligations discharge. Its `contract_dependencies` name no
  declaration and no summary.
* **`compress_fragment`** (`snappy.c`), Unknown → Unknown: loses `call to
  uncontracted function log2_floor` (15 leaves → 14). Justification: the
  callee's body now validates (above), so its W4 summary is in force for the
  same-unit call; the row keeps its other 14 leaves. The predicted leaf trade
  is a leaf removal.
* Unchanged, with their base leaves byte for byte: `LZ4_NbCommonBytes`
  (`lz4.c`, four `unmodeled call, aggregate or indirect access in return
  expression` leaves: unguarded, and the 32-bit arms narrow `val`),
  `find_lsb_set_non_zero` and `find_lsb_set_non_zero64` (``call to
  uncontracted function `__builtin_ctz` `` / `` `__builtin_ctzll` ``:
  unguarded).

No other row moved or changed on either route; system: no moved and no
changed row; its null-property stream is identical row for row apart from
the hypothesis context's build-identity and assumptions digests (the binary
differs). Prediction against measured: the one predicted move happened, no
unpredicted move, the `compress_fragment` change is the leaf the prediction
allowed for. Corpus and twelve-home remain unmeasured here.

## 6. Tests

* `crates/qcore-c-gateway/tests/k2_no_typed_access.rs` section (3), six
  tests (19 in the file): every width admitted under a `?:` guard, a widening
  operand, a same-rank signed operand, a `const` local, `&&`, `||`, a
  connective guard, the four statement guards; and every near miss with its
  exact base keys: each unguarded width, LZ4's spelling, the wrong arm,
  another object, `x >= 0`, `x == 1`, a branch that does not return, a write
  after the test or in a loop, a narrowing cast, `volatile`, a static local,
  a global, an address-taken object, a label, a `case` label, and a mixed
  body.
* `crates/qcore-c-gateway/tests/unit/bit_count_builtins.rs` (7 tests, the
  crate's unit tests): the table and both vector sets equal to Lean's, the
  neutral contract's clauses, conditions that prove nothing, the refusal
  keys, the site matching.

## 7. Shared-file hook points

| File | Hook |
|---|---|
| `crates/qcore-c-gateway/src/legacy_c.rs` | `build_function_cfg_with`: two lines between `// K2-BUILTIN-BIT-COUNTS hook (rule 94): begin` / `end`, shadowing `contracts` |
| `crates/qcore-c-gateway/src/lib.rs` | `mod bit_count_builtins;` and its `#[cfg(test)]` unit-test module, after `type_query_positions` |
| `crates/qcore-c-gateway/tests/k2_no_typed_access.rs` | section (3) appended; one doc-comment sentence |
| generated | `docs/agents/gateway.md`, `docs/agents/README.md` (rule 94), `proof/QCore.lean` |

## 8. Follow-ons

Amended in [integration-amendments.json](integration-amendments.json)
(`K2-BUILTIN-BIT-COUNTS`, what remains): a caller-established precondition
(`LZ4_NbCommonBytes`, `find_lsb_set_non_zero{,64}`: every in-unit caller
tests the argument nonzero first), a written object (a loop that clears the
lowest bit), the narrowed 32-bit LZ4 arms, a per-site reading of a mixed
body, and guards beyond the listed forms (`x == K` with `K ≠ 0`).

## Increment paragraph

**K2-BUILTIN-BIT-COUNTS** (user decision 2026-11-28, option (a); gateway rule 94, Lean `QCore.BitCountBuiltins`; [report](README.md)). A call of a bit-count builtin `__builtin_c{l,t}z{,l,ll}(x)`, undefined at `x = 0`, is read as the identity on custody (its one scalar `Borrowed`, no result custody) only when every such call of the function reads a stable automatic integer object through widening conversions and a guard on its path proves `x ≠ 0` on the edge it is reached by (a `?:` arm, the right operand of `&&`/`||`, an `if` branch, or the block after `if (C) return`, the statement forms on structured flow only); the contract enters only that function's CFG environment (`bit_count_builtins.rs`, one hook in `legacy_c::build_function_cfg_with`), and every undischarged site keeps its base refusal byte for byte. The builtins are modelled in the gateway's own builtin table, as `__builtin_expect` and the `__builtin_mem*` writers are, not in a provider. `assert(x != 0)` is no guard. Measured base `287dce8ed` against tip `edf56d4e9` over this host's 046accf6 preparation: opt-in current 1,209/2/3,586/7 → 1,210/2/3,585/7 (`log2_floor` Unknown → Safe, exactly the prediction; `compress_fragment` loses its `call to uncontracted function log2_floor` leaf and stays Unknown); `LZ4_NbCommonBytes` and `find_lsb_set_non_zero{,64}` unchanged (unguarded); system 124/0/320/4 unchanged.
