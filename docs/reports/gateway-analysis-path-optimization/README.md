# GATEWAY-ANALYSIS-PATH-OPTIMIZATION: the analysis paths mapped, made explicit and faster, verdict-identical

Card GATEWAY-ANALYSIS-PATH-OPTIMIZATION (wave 48, round 18; refactor). Base:
local `main` at `2ecd39531`; when CG10-INDIRECT-ACCESS-SOLE-RESIDUALS landed
(`170aba7b0`, rule 12k) the card was rebased onto it without conflicts: rule
12k's two hooks stay in `legacy_c.rs` (they rewrite the final instruction and
the return's evaluation, so they are not rows of the fallback tables; the
route map lists them), the split re-proved a move, and the identity proof was
taken again against the new base (section 5). No new gateway rule, no new
semantics, no Lean change (287 audits after the rebase).
[PREDICTION.md](PREDICTION.md) was frozen at `c6e61630d` (`6a76ee04b` after
the rebase), before the first implementation commit; the user's amendment
(the Clang layer first) arrived before the freeze and is folded into it.
Step 7 was added after the suite's Clang census and is not in the prediction.

**Outcome.** Medians of three at the original base, `--jobs 4`, parity stream
on: the pinned corpus arbiter **616.4 → 364.1 s (−40.9 %)**, the system route
**166.9 → 79.6 s (−52.3 %)**, the tail-pause slice **60.1 → 27.6 s
(−54.2 %)**; `run_test_binaries.py` run phase **300.7 → 238.8 s (−20.6 %)**,
wall 308.6 → 245.8 s. After the rebase (one run each): corpus 759.2 → 448.9 s,
system 192.7 → 63.1 s, slice 69.3 → 21.4 s. Every `resource-diagnostics.json`
and sidecar is byte-identical to BEFORE on all three routes at every step
(695 / 9 / 4,130 corpus, `d2f5a3f7…`, at the original base; 756 / 9 / 4,069,
`ce57d194…`, after the rebase; system and slice totals unchanged). The one
exception to a raw comparison is not the card's: the BEFORE binary's own
corpus parity stream varies between runs in one row (section 4,
`GAPO-PARITY-DECISIONS-NONDETERMINISTIC`).

**Prediction against outcome.** Predicted corpus −30 % to −45 % (measured
−40.9 %), system −25 % to −45 % (−52.3 %, above the range), slice −15 % to
−40 % (−54.2 %, above), suite run phase −5 % to −25 % (−20.6 %); step 1
predicted −25 % to −40 % of corpus wall (measured −46 % on its own run);
step 2 predicted −3 % to −10 % (measured within noise: the corpus repeats
only 7 whole-unit AST launches); steps 3-6 predicted no measurable change
(none measured). Outputs byte-identical, as predicted.

## 1. Where the time went (task 1)

The [route map](ROUTE-MAP.md) gives the consultation order of every
statement, condition, return, declaration and call route with its rule, entry
condition, inputs and decision, the Clang launch sites, the measured Clang
census and the profile. Its headline, measured on the BEFORE binary:

| Share of worker wall time | Corpus | System route |
|---|---:|---:|
| Contract environment re-read (declared module texts parsed and lexed per CFG build) | **42.9 %** | **59.5 %** |
| serde_json parsing Clang's whole-unit AST | 16.7 % | 2.4 % |
| Waiting on Clang processes | 3.9 % | 2.7 % |
| The analysis proper | 36.5 % | 35.4 % |

The Clang layer is not where the arbiter spends its time: 1,549 launches,
90 Clang-seconds against about 2,630 worker-seconds (3.4 %), of which 601
launches (26.9 s) repeat an earlier launch's arguments and input. The cost
was a *shared input re-derived per consultation*: every CFG build (one per
function per W4 derivation round, plus its row) asked nine contract-loader
readers, and each re-parsed every declared module text of the environment.

Route census (`route-census` build over the three routes,
[summary](artifacts/route-census.txt)): every row of the two fallback tables
is reached on every route; four never answer on any of them: the statement
table's two 11u rows (VLA declaration, `container_of` value) and the return
table's 11u and 12f rows. They are kept: their rules have unit fixtures, and
hard invariant 3 removes a route only with a replacement's parity. The most
productive rows are 10s (9,704 corpus statements), W44 (6,404) and 11g
(5,985).

## 2. The steps

Each step is its own commit, measured on all three routes and compared with
`scripts/identity.py` (every output file by sha256, raw and after erasing the
run's own name and the `preparation:` durations; each parity stream as a
multiset with Clang node addresses erased).

| Step | Commit (pre-rebase) | Change | Corpus / system / slice wall (s) | Identity |
|---|---|---|---|---|
| BEFORE | `2ecd39531` | | 658.2, 616.4, 613.3 / 172.2, 166.4, 166.9 / 60.9, 59.8, 60.1 | (reference) |
| 1 | `f36e78f48` | contract-environment readings once per module text (qcore-contract) | 357.4 / 79.4 / 27.4 | identical |
| 2 | `16a5f5ef1` | unit AST and record-layout dump reused across a unit's scopes (`clang_reuse.rs`) | 362.6 / 80.1 / 28.0 | identical |
| 3 | `1e844a4dd` | a condition's pure null/distinct tests asked once | 366.7 (re-run, `step3b`) / 80.5 / 28.4 | identical |
| 4 | `5ef531965` | statement and return fallbacks as two tables, one dispatcher each (`legacy_route_table.rs`) | 364.9 (re-run, `step4b`) / 83.2 / 27.8 | identical |
| 6 | `368bbc9ed` | the unreachable second copy of the seven call holds removed | 368.7 / 80.1 / 28.6 | identical |
| 5 | `ea2c026e0` | `legacy_c.rs` split into eight modules, move-only | 364.1, 363.7, 366.4 / 79.0, 79.6, 79.8 / 27.9, 27.6, 27.4 (the AFTER runs) | identical (three runs) |
| 7 | `ccc9320c9` | a filtered-AST launch identical to an earlier one on the thread answered by it (`clang_reuse::LaunchCache`) | 358.9 / 47.5 / 22.2 (one run) | identical |
| rebased | `b15d525a7` on `170aba7b0` | the whole card over rule 12k's main (BEFORE `main2` = `170aba7b0`: 759.2 / 192.7 / 69.3) | 448.9 / 63.1 / 21.4 (one run) | identical to `main2` |

**Step 1** (`crates/qcore-contract/src/contract_loader.rs`). The nine
`declared_*` / `*_obligations` / `*_tails` readers shared one skeleton:
reject the retired directive, reject an unresolved inclusion, parse, check the
mode, then `load_function` each declaration, collecting one field. They now
read one cached reading per (path, text) pair: the same rejections, parse,
mode check and per-declaration loads up to the first failure, with every
error string formatted and met in the same order. The reading is a pure
function of the pair (nothing it calls reads anything else), so sharing it
changes no answer. Corpus −46 %, system −54 %, slice −55 %.

**Step 2** (`crates/qcore-c-gateway/src/clang_reuse.rs`, hooks in
`typed_access_path.rs`). A unit is analysed under several source scopes; each
re-launched Clang for the whole-unit AST, and every layout consultation
re-launched the record-layout dump. Both are now kept per worker thread (two
units) and reused by a later scope with the same launch identity: the Clang
resolution environment, the dialect, the full frontend request and the
analysed text compared byte for byte; only successes are kept; a text that
spells `#include`/`#import`/`#embed`/`__has_include` (which could read a file
the key does not hold) is never reused. The corpus has few repeated AST
launches (7) and 82 repeated layout launches, so the measured wall-time
change is within noise; it is kept because it is verdict-identical and
removes those launches.

**Step 3.** `null_condition` and `distinct_condition` read only fields no
statement mutates (checked: no assignment to `descriptor_names`,
`store_bases`, `typed_model`, `typed_types`, `callee_types` or
`alias_classes` anywhere in the parser); each is now asked once per
`if`/`while`/`for`/`do` condition instead of up to four times. Below noise.

**Step 4** (`legacy_route_table.rs`). The fifteen statement fallbacks of
`expression_statement` and the eight return fallbacks of the `return` arm are
two `const` tables in exactly the order the code had, each row with its rule
and its own entry condition, asked by one dispatcher each; a unit test pins
the order. The routes asked before classification and the ones that rewrite
the classified instruction stay inline (they are not fallbacks of a refusal).
The measurement-only `route-census` feature records each row's consultations.

**Step 6.** After the uncontracted-call arm of `classify_simple_base`, seven
`if` blocks re-asked the call holds (review, pin and refutation holds,
escaped spelling, literal premise, ABI and mode withdrawals) for the same
callee the same pure name readers had already answered before the lexical
gate; they could never fire and are removed (the identity runs are the parity
evidence of invariant 3).

**Step 5** (the split, done last). `scripts/split.py` cuts eight cohesive
segments out of `legacy_c.rs` by item anchors (re-runnable after a rebase)
into child modules: `legacy_tokens.rs` (token stream and definition scan),
`legacy_parser_census.rs` (census hooks, `indirect-census` only),
`legacy_condition_routes.rs`, `legacy_return_routes.rs`, `legacy_classify.rs`
(the statement classifier), `legacy_token_shapes.rs`, `legacy_cfg_builder.rs`
and `legacy_transfer.rs` (abstract state and transfer). `legacy_c.rs` goes
from 15,613 to 5,150 lines. The only edit to moved text is `pub(super)` on
items the parent or a sibling reads; `scripts/move_check.py` proves the
commit is a move: 10,267 non-blank lines left the parent and the same lines
arrived in the modules, except 16 lines rustfmt re-wrapped after the prefix
pushed them past the width limit (the same characters plus 15 trailing
commas) ([output](artifacts/move-check-step5.txt)). Statement and return
fallback order, file layout and the module map are in
[architecture](../../development/architecture.md).

**Step 7** (after the suite's Clang census; not in the frozen prediction).
The census of `run_test_binaries.py` (ROUTE-MAP section 7.3) found the suite,
unlike the arbiter, dominated by Clang: 91,035 launches, 49,185 of which
repeat an earlier launch's arguments and input, 23,720 of them filtered-AST
dumps the source importers ask again of the same text (`cio_ctx` 2,327
times). `typed_c_ast::ast_declarations` now answers a launch identical to an
earlier successful one on the same thread (executable, every argument,
standard input, resolution environment; never a text that may read a file,
never a command with its own environment or directory; 32 per thread;
`clang_reuse::LaunchCache`). One run each against step 6: corpus 368.7 → 358.9 s, system route
80.1 → 47.5 s, slice 28.6 → 22.2 s (the configured routes import chunk and
Monkey sources through the same importers); the three-run medians above are
of the step-5 tip, so they understate the final tip on the system route and
the slice.

## 3. Not done, and why

* **A cross-run Clang cache** (the amendment's item 3): Clang processes are
  3–4 % of arbiter worker time, the preprocessing launches read include
  files from disk (their output is not a function of arguments and input),
  and a fail-closed key would have to cover every file read. Registered
  (`GAPO-CLANG-CROSS-RUN-CACHE`).
* **Reusing the repeated preprocessing** (the pinned route runs the same
  line-marked `-E` twice per unit; validation again for Unsafe coordinates):
  those launches read include files, and the preparation's Clang-invocation
  count is a reported figure. Registered under `GAPO-REMAINING-REDERIVATIONS`.
* **Caching `ClangIdentity::capture`**: tests swap the executable behind one
  `QCORE_CLANG` path, so a cache keyed by the environment could answer a stale
  identity. Registered under the same entry.
* **Tables for the condition routes and `classify_simple`'s internal
  chain**: their entries thread sites, lowerings and parity rows; ROUTE-MAP is
  their authority (`GAPO-DISPATCHER-COVERAGE`).

## 4. Findings (registered, none fixed)

* `GAPO-PARITY-DECISIONS-NONDETERMINISTIC`: the BEFORE binary's corpus
  parity stream differs between runs in one `typed-place` row at
  `cio_file_st_get_meta_len` line 2223 (626,469 or 626,470 rows over four
  runs; the leaked place is `st` in one run and `u` in another): rule 12j's
  place decisions go into a thread-local list some consumers neither flush
  nor clear, so a decision left by one consultation is flushed into the next
  statement row the same worker writes. `identity.py` compares that one row
  apart; the rest of every stream is identical.
* `GAPO-RETURN-PROBE-COUNTER-RESTORATION`, `GAPO-RETURN-POSITION-ROUTE-GAPS`,
  `GAPO-ROUTE-ORDER-FINDINGS`: ordering and state observations from the map
  (ROUTE-MAP section 9).
* `GAPO-REMAINING-REDERIVATIONS`: the next targets, with the AFTER profile
  (section 5).
* A measurement hazard of this card, corrected: the first step-3 and step-4
  corpus runs measured the BEFORE binary, because a concurrent run of this
  worktree rewrote `target/release/quirin-audit` between `measure-binary.sh`'s
  copy and its run (run.json's `binary_sha256` showed it). Those two runs are
  discarded and re-measured with `measure-atomic.sh`, which copies and runs in
  one lock acquisition (`step3b`, `step4b`).

## 5. Measurement

Timing ([table](artifacts/timing.json), `scripts/timing_table.py`): each
route's wall time measured inside the shared heavy lock (waiting excluded),
BEFORE and AFTER binaries interleaved (`final1`, `before3`, `final2`,
`final3`), the same host, `--jobs 4`, parity stream on.

| Route | BEFORE runs (s) | median | AFTER runs (s) | median | change |
|---|---|---:|---|---:|---:|
| corpus | 658.2, 616.4, 613.3 | 616.4 | 364.1, 363.7, 366.4 | 364.1 | −40.9 % |
| system | 172.2, 166.4, 166.9 | 166.9 | 79.0, 79.6, 79.8 | 79.6 | −52.3 % |
| slice | 60.9, 59.8, 60.1 | 60.1 | 27.9, 27.6, 27.4 | 27.6 | −54.2 % |

`run_test_binaries.py` (default jobs, `scripts/tests-timing.sh`): BEFORE at
`2ecd39531` (checked out for the runs) walls 421.0, 308.6, 307.1 s, run phases
300.7, 302.3, 300.5 s; AFTER at the step-7 tip walls 339.0, 245.6, 245.8 s,
run phases 238.8, 238.8, 239.5 s (the third is the first gate's suite run);
the first run of each side includes the rebuild. Median run phase
300.7 → 238.8 s (−20.6 %). All six runs pass (7,550 BEFORE, 7,554 AFTER: the
card's four new unit tests).

Identity ([proofs](artifacts/identity/)): at every step every output file is
byte-identical after erasing the run's own directory name (`audit.log`,
`resource-diagnostics.invocation.json`) and the `preparation:` durations
(console); every `resource-diagnostics*.json`, `monkey-source.json` and the
slice's sidecars are raw-identical. Parity streams: system and slice
identical as multisets in every run; the corpus stream identical except the
`GAPO-PARITY-DECISIONS-NONDETERMINISTIC` row, which five runs of the BEFORE
binary wrote four times (626,470 rows) and once not (626,469), carrying `st`
in some runs and `u` in others; `identity.py` compares that row apart and the
rest of the stream must match. The route-census build (`census4`) is
identical too. After the rebase, `main2` (BEFORE at `170aba7b0`) and
`rebased` agree on every file (`ce57d194…`, 756 / 9 / 4,069) and on every
parity stream but that row ([proof](artifacts/identity-rebased.json)).

Profile after ([corpus](artifacts/profile-corpus-after.json), the step-5
binary): the contract environment fell from 42.9 % to 0.7 % of worker time;
AST JSON parsing is now 35.2 %, waiting on Clang 8.0 %, the analysis 56.2 %;
the top self-time functions are `literal_premise::restrict_to_declared_literals`
(11.7 %, a whole-unit token pass per CFG build), a `read` (10.6 %),
`typed_access_path::node_line` (8.1 %) and `build_function_cfg_with` (7.9 %)
(`GAPO-REMAINING-REDERIVATIONS`).

## 6. Gate

Standing gate on the rebased tip (`scripts/gate.sh g2`, every step under the
shared lock, exits in [gate/](gate/), [gate.json](gate.json)):

| Step | Command | Exit | Result |
|---|---|---:|---|
| 00 | `cargo check --workspace --all-targets` | 0 | |
| 01 | submodule update (`--init --recursive`) | 0 | |
| 02 | `cargo fmt --all -- --check` | 0 | clean |
| 03 | `cargo clippy --locked --all-targets -- -D warnings` | 0 | clean |
| 04 | `python3 scripts/run_test_binaries.py` | 0 | 441 units, 454 summaries; **7,565 passed**, 0 failed, 11 ignored (`private/gapo/tests-gate-g2/`) |
| 05 | `cargo run -p qcore-compiler --example reevaluation_probe` | 0 | |
| 06 | `cargo run -p qcore-compiler --example reevaluation_followup` | 0 | |
| 07 | `python3 scripts/check_lean_audits.py` | 0 | **287** audits (unchanged) |
| 08-10 | Python suite and `build_docs_index.py --check`, after the matrix regeneration | see [gate/](gate/) | |

The first gate (`gate-g1`, pre-rebase tip on `2ecd39531`) exited 0 at every
code step (7,554 passed, 286 audits). Also run: `cargo clippy -p
qcore-c-gateway --all-targets --features route-census,indirect-census -- -D
warnings`, exit 0 (the card's census feature). Not part of the gate and
pre-existing in files the card does not touch: the `provenance-rounds-census`
feature build fails clippy's `private_interfaces` lint in
`provenance_rounds::record`. The provider gate is not owed: `providers/` is
unchanged.

## Files

Scripts (`scripts/`): `measure.sh`, `measure-binary.sh`, `measure-atomic.sh`,
`step-measure.sh`, `step-measure2.sh` (route runs), `identity.py` (identity
proof), `parity_diff.py`, `clang-census.sh` + `clang-census-run.sh` +
`clang_census.py` (Clang census), `gdb_sampler.py` + `profile-run.sh` +
`profile.py` (profile), `route-census-run.sh`, `tests-timing.sh`,
`build.sh`, `cargo-step.sh`, `split.py`, `move_check.py`, `update_ledger.py`,
`timing_table.py`, `rebase-measure.sh`, `gate.sh`, `finish-gate.sh`.
Artifacts (`artifacts/`): the identity proofs, the Clang census and profile
summaries, the route census, the move check and the timing table.
