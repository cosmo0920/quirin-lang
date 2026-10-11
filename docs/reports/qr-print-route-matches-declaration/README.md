# QR-PRINT-ROUTE-MATCHES-DECLARATION: the print-call route reads a call, not a name

Card QR-PRINT-ROUTE-MATCHES-DECLARATION (wave 48, card-high, 2026-10-11), on
the user's request to launch it. Base: local `main` at `da6c1f448`, which it
stayed at. [PREDICTION.md](PREDICTION.md) was committed at `dce78f389`, before
any change to `crates/`, `providers/` or `proof/`. Authority: the ledger entry
of the same name (registered by
[W45-CALLEE-CASCADE-RESIDUAL round 3](../w45-callee-cascade-residual-r3/README.md),
section 2, "Withdrawn after measurement"). No gateway rule (the reserved `15v`
is unused); rule SOURCE-SCALAR-CALLS-V1 in
[source-routes](../../agents/source-routes.md) is amended in place.

## 1. The defect, reproduced by reading

`qcore_frontend::source_scalar_calls::has_print_calls` (base
`source_scalar_calls.rs:94-98`) was `lexical_tokens().windows(2).any(print, ()`,
and `compile_elaborated_source` (base `driver.rs:2056`) sent every module it
matched, below the concurrent, extern, event, buffer, HTTP, string and socket
routes, to `compile_scalar_calls`, whose reader refuses every `extern fn`
(`Q020 … only source function declarations are supported`). The declaration
`extern fn print(item: …) -> …` holds the pair, so `librdkafka/cjson.qv`
failed `qcore_contract_corpus`'s compile regression; round 3 withdrew it
(`214d1334f`). The route census below shows the base routes all 214 tracked
contract modules through `Contract`/`Sequential`; none held the pair after the
withdrawal.

## 2. The fix

**Primary guard: selection reads a call** (`print_call_selection`, which
replaces `has_print_calls`). Each `print (` pair is classified against the
source function bodies' spans (`SourceBlock::span`, braces included) and the
previous lexical token:

| where the pair is | classification | result |
| --- | --- | --- |
| outside every source function body (a declaration name, an `extern fn print(…)`, a contract clause, a record field) | not a call | selects nothing; later routes decide |
| inside a body, previous token `{`, `;` or `}` (a statement head) | a call | `source.scalar-calls.v1`, as before |
| inside a body anywhere else (`let x = print(1);`, `f(print(1));`) | ambiguous | `Q020` at the selection point, never routed |
| a call, in a module that also declares something named `print` | ambiguous | `Q020` at the selection point |

Comments are not lexical tokens and a string literal is not a word, so neither
could select before and neither can now. The driver's guard keeps its place in
the route order, so a module captured by an earlier route is untouched.

**Secondary guard: a contract module is never compiled on the scalar-calls
route.** `compile_scalar_calls` refuses a `module contract;` module at entry
(`Q020`, naming the reason), whichever of its three selection sites (the
record-and-helpers guard, the print-call guard, `has_scalar_calls`) reached it.

**Why the selector is primary.** The defect is the selector reading a
declaration name, which misroutes a library or executable module declaring
`extern fn print` as much as a contract module; the contract-mode guard alone
would fix only cjson's instance. The guard is cheap and fail-closed, so both
are implemented; on the census no tracked module reaches it, and
`a_contract_module_is_never_compiled_on_the_scalar_route` drives its arm (a
contract module with two source functions and a call between them, which
`has_scalar_calls` selects).

Tests: [`compiler/tests/qr_print_route_selection.rs`](../../../compiler/tests/qr_print_route_selection.rs)
(seven: the cjson-shaped contract module compiles in `Contract` mode; the
printing example and a call in a nested arm block still select the route;
`print` as a value and a call beside a declared `print` refuse at selection; a
comment plus an `extern fn print` declaration does not select; the contract
guard refuses) and
[`compiler/tests/qr_print_route_census.rs`](../../../compiler/tests/qr_print_route_census.rs)
`every_tracked_print_pair_is_classified_as_pinned` (always run: of the tracked
sources holding the pair, exactly `examples/scalar/printing.qr` selects and
exactly `librdkafka/cjson.qv` holds it outside every body; none is ambiguous).

## 3. Nothing else moves: the route census

[`qr_print_route_census.rs`](../../../compiler/tests/qr_print_route_census.rs)
`route_census_of_every_tracked_source` (ignored; run by
[`scripts/census.sh`](scripts/census.sh), compared by
[`scripts/census_compare.py`](scripts/census_compare.py)) compiles every
tracked `.qr`/`.qv`/`.qcore`/`.qcorelib` (718 files, by `git ls-files`) as a
root with filesystem import loading, without a recorded target and with a
64-bit one, and records the mode, execution profile and checked-module variant
(the route), entry, counts, the emitted C in full and every formatted
diagnostic.

| comparison | rows | moved |
| --- | ---: | --- |
| base (`census-before`) → fix, contracts unchanged (`census-fix`) | 1,436 | **0**: every route, outcome and emitted C byte-identical ([artifact](artifacts/census-before-fix.json)) |
| fix → fix plus the declaration (`census-after`) | 1,436 | **2**: `cjson.qv` in both configurations, `declarations` +1 and the token pair now present; still admitted `Contract` / `Sequential` ([artifact](artifacts/census-fix-after.json)) |

Base routes (no-target configuration): 214 contract modules admitted on the
sequential route, 14 executables on `source.scalar-calls.v1`, 5 refusals on
it, and every other route as listed in the artifact's `routes` table; the fix
moves none. **No fixture changes route, so there is no correction to name.**

Coverage of programs that are not tracked files. The census reads files; the
`.qr` programs written inline in Rust tests are covered by the suite (section
6, unchanged pins). A scan of tracked test sources for `print(` program text
finds every occurrence at a statement head (after `{`, `;`, or at the start
of a body string a helper places after `{`); the others are comments and
`replace("print dec r;", "print(r);")`, whose result is a statement too.
Native observations compile tracked examples, which the census covers.

## 4. cJSON's static `print`, re-declared

`librdkafka/cjson.qv` is restored byte for byte to its text before the
withdrawal (`214d1334f^`): the declaration, its comments and the module digest
`7fbb1d74…` in `contracts.json` are round 3's. The body (`cJSON.c:1234-1299`)
was read at both revisions; lines 1234-1310 and the whole file are
byte-identical at `66910c10` and `046accf6`. It allocates through
`hooks->allocate`, renders, then shrinks through `hooks->reallocate` or copies
into a second allocation and releases the first; every failure path releases
what it allocated and returns NULL: `resource_return_owned(nullable)` under the
`cjson_allocator_hooks` premise, three borrowed formals.

`quirin-audit pin --crate qcore-provider-librdkafka --function print` at both
trees ([`scripts/pin.sh`](scripts/pin.sh), logs `private/qrpr/pin-p1/`, all
steps 0 but the whole-tree checks) records one definition site,
`lib/librdkafka-2.15.0/src/cJSON.c`, declaration `c36c98da…`, body
`f1b509f8…`, contract `45f1497d…` — round 3's recorded identity. `pin --check
--crate qcore-provider-librdkafka` exits 0 at both trees and `pin
--verify-contracts` 0. The whole-tree `pin --check` exits 1 at both, as on the
base: its one row not in force is the libevent crate's tip-mode `mk_vhost_set`.
`providers/contracts.py sync` and `check` exit 0.

Standing in the corpus (a1's contracts sidecar): `pinned-in-force`,
body standing **declared assumption** (its own row is Unknown in `cJSON.c`:
`unvalidated_units` 1, `validated_units` 0), the premise carried, three rows,
two Safe.

Pins moved deliberately: `contract_loader.rs` v1 declarations 393 → 394 and
`print`'s previous mode (owned nullable) in the preservation table;
`qcore_contract_corpus.rs` `("librdkafka/cjson.qv", "print", 6)`. The corpus
module count (131) is unchanged.

## 5. Measurement

Binaries: BEFORE `private/qrpr/bin/before` (`0be034f6…`, the base), AFTER
`private/qrpr/bin/fix` (`54d41ce4…`, the fix commit `85a385162`); each corpus
`run.json`'s `binary_sha256` equals its binary. Every route under the shared
lock, `--jobs 4`, the null property selected on every run
([`scripts/measure.sh`](scripts/measure.sh), [`scripts/runs.py`](scripts/runs.py)).

| run | corpus | system | tail-pause slice |
| --- | --- | --- | --- |
| BEFORE (base binary, base contracts) | 933 / 9 / 3,892, `cdb0267b…` (the brief's) | 93 / 4 / 351, `3d72210a…` | 52 / 1 / 347, 4 Pending, `ec693cbb…` |
| a0 (fix binary, base contracts) | 933 / 9 / 3,892, `cdb0267b…`, byte-identical | identical digest | identical digest |
| a1 (fix binary, `print` declared) | **935 / 9 / 3,890**, `bdf5f1c8…` | 93 / 4 / 351, no row moves | 52 / 1 / 347, 4 Pending, no row moves |

a1's corpus movers ([artifact](artifacts/compare-a1-corpus.json)), both
attributed to the declaration and both Unknown → Safe, each losing exactly
``call to uncontracted function `print` `` and gaining nothing:
`cJSON.c::cJSON_Print`, `cJSON.c::cJSON_PrintUnformatted`; each rests on the
one declaration `print` (`contract_dependencies`). No Safe → non-Safe move; no
leak or violation added; `print`'s own row is Unknown on both sides. The system
and slice digests move without a row moving: each unit's
`compile_configuration` records the contract sources' digest
(`resource_contract_sources`, `inventory_sha256`).

Null property: corpus 1,113 Safe unconditional / 1,435 conditional / 2,355
Unknown / 18 Unsafe; system 84 / 125 / 245; slice 71 / 103 / 232 — identical on
all three runs. a0's null streams are byte-identical to BEFORE's; a1's differ
in every row's `identity` (the resource contracts' digest) and, in the corpus,
in the custody and combined fields of the two movers; no null verdict or check
moves ([runs-a1](artifacts/runs-a1.txt)).

Corpus wall time 519.6 s BEFORE, 513.4 s a0, 517.3 s a1.

Prediction against outcome: every prediction held (census 0 and 2 moved
rows; a0 identical; a1 +2 with the named movers; system, slice and the null
property unmoved; standing a declared assumption).

## 6. Lean

`QCore` had no model of source route selection (`QCore.ScalarCalls` models the
route's checked semantics). New `QCore.PrintRouteSelection`, audited in
`AuditPrintRouteSelection.lean`: the token-site classification and the
contract guard, with `declaration_name_never_selects` (a module whose `print`
tokens all lie outside bodies is left to the later routes),
`selected_has_a_call`, `declared_print_never_selected`,
`ambiguous_occurrence_refuses`, `contract_module_never_compiled`, and vectors
for the three shapes (the cjson declaration, the printing example, `print` as
a value) plus the declared-`print` and contract variants. Premise (trust
boundary 28): the Rust function computes this classification over the
frontend's tokens and body spans; agreement is checked by the tests above, not
proved.

## 7. Gate

[`gate.json`](gate.json), exits in [`gate/`](gate/); scripts
[`gate.sh`](scripts/gate.sh), [`provider-gate.sh`](scripts/provider-gate.sh),
[`write_gate_json.py`](scripts/write_gate_json.py),
[`finish-gate.sh`](scripts/finish-gate.sh); every step under the shared lock,
nothing piped, logs under `private/qrpr/`.

Standing gate g2, every code step at `734edb8a9` (the records commit; the
code is that of `b2e840be9`): `cargo check --workspace --all-targets` 0,
submodule 0, fmt 0, clippy `--locked` 0; `run_test_binaries.py` 483 units,
496 summaries, **7,903 passed, 1 failed**, 13 ignored, exit 101
(`private/qrpr/tests-gate-g2/`). The one failure is
`qcore-verifier-host::test::execution_quota`
`the_quota_binds_without_depending_on_the_accounts_process_count`, a row of
the runner's `EXCLUSIVE` table (its RLIMIT_NPROC rows count every process of
the real UID, so other sessions on the host move it); constraint 16 applied:
re-run in isolation, **13 passed, 0 failed**, exit 0
(`private/qrpr/isolation-execution-quota.log`, `gate/04-isolation-execution-quota.exit`);
the card does not touch that crate. The earlier full run g1 (code at
`86a40b40c`, before the fmt-only commit) was 7,904 passed, 0 failed, exit 0,
its one red step `cargo fmt --check` on the new test file, fixed in
`b2e840be9`. Both reevaluation probes 0; `check_lean_audits.py` **314** passed
(313 + `AuditPrintRouteSelection`); the Python suite 365 tests, 1 failure
before the matrix regeneration (the committed matrix's derived Lean count
313), then `build_matrix.py --gate gate.json` 0 and **365 OK** (09b);
`build_docs_index.py` and `--check` 0.

Provider gate g1 (at `86a40b40c`; `providers/` unchanged since): `cargo check
--locked` 0, `cargo test` 129 passed, fmt 0, clippy 0, `contracts.py check` 0,
package build 0 and verify 0 for all five packages, `providers/tests` 25 OK.
Pins: `pin` and `pin --check --crate` 0 at both revisions, `pin
--verify-contracts` 0 (section 4).

## 8. Follow-ups

* Retired: `QR-PRINT-ROUTE-MATCHES-DECLARATION` (its DISCHARGE holds: the
  selection reads a call expression, a contract module is never compiled on
  the route, and `print` is declared as recorded and measured, +2).
* Amended: `W45-CALLEE-CASCADE-RESIDUAL` (print declared; on round 3's
  [`rank.py`](../w45-callee-cascade-residual-r3/scripts/rank.py) the
  population of Unknown functions blocked by uncontracted calls alone is
  481 -> 479, decided by one callee alone 329 -> 327;
  [artifact](artifacts/rank-a1.json)).
* Registered: `QR-LEXICAL-SELECTORS-DECLARATION-NAMES`. Other routes'
  selectors still read the whole token stream (rule 14's exclusions cover
  library binding spans and socket hosts only), and the defect class is live
  there: a contract module declaring `extern fn qcore_buffer_new(size: usize)
  -> ptr<mut, u8>` is refused `Q015 concurrency requires an explicit #import
  std.concurrency;` at the declaration name, while the same module naming
  `flb_noop` is admitted as a `Contract` module ([probe](artifacts/probe/),
  `quirin check` on this card's tip). No tracked module is affected (the
  census), and this card's contract guard covers only
  `source.scalar-calls.v1`; extending the body-scoped reading to every
  selector is outside this card.

Ledger: [`scripts/update_ledger.py`](scripts/update_ledger.py); follow-ups 763
-> 763 (one retired, one registered).

**Integrator re-run (2026-10-11).** Gate g2's test step exited 101 on the
load-sensitive `execution_quota` row (`LOAD-SENSITIVE-ROW-UNNAMED`). The
integrator re-ran `python3 scripts/run_test_binaries.py` on tip `c1eda60c9`
under the shared heavy lock, with no other suite running: exit 0, 483 units,
496 summaries, 7,904 passed, 0 failed, 13 ignored
(`private/test-binaries/20261011T040845Z-518670/`).
