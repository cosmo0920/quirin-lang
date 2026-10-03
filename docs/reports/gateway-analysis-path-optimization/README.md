# GATEWAY-ANALYSIS-PATH-OPTIMIZATION: the analysis paths mapped, made explicit and faster, verdict-identical

Card GATEWAY-ANALYSIS-PATH-OPTIMIZATION (wave 48, round 18; refactor). Base:
local `main` at `2ecd39531`, which did not move during the card (the
concurrent CG10-INDIRECT-ACCESS-SOLE-RESIDUALS had not landed; nothing was
rebased). No new gateway rule, no new semantics, no Lean change (286 audits
unchanged). [PREDICTION.md](PREDICTION.md) was frozen at `c6e61630d`, before
the first implementation commit; the user's amendment (the Clang layer first)
arrived before the freeze and is folded into it.

**Outcome.** The pinned corpus arbiter runs in @@CORPUS@@ (median of three,
`--jobs 4`, parity stream on), the system route in @@SYSTEM@@ and the
tail-pause slice in @@SLICE@@; `run_test_binaries.py`'s run phase
@@TESTS@@. Every `resource-diagnostics.json` and sidecar is byte-identical to
BEFORE on all three routes at every step (695 / 9 / 4,130 corpus,
`d2f5a3f7…`; 85 / 4 / 359 system; the slice unchanged). The one exception to
a raw comparison is not the card's: the BEFORE binary's own corpus parity
stream varies between runs in one row (section 4,
`GAPO-PARITY-DECISIONS-NONDETERMINISTIC`).

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

| Step | Commit | Change | Corpus / system / slice wall (s) | Identity |
|---|---|---|---|---|
| BEFORE | `2ecd39531` | | 658.2, 616.4, @@B3@@ / 172.2, 166.4, @@B3S@@ / 60.9, 59.8, @@B3L@@ | (reference) |
| 1 | `f36e78f48` | contract-environment readings once per module text (qcore-contract) | 357.4 / 79.4 / 27.4 | identical |
| 2 | `16a5f5ef1` | unit AST and record-layout dump reused across a unit's scopes (`clang_reuse.rs`) | 362.6 / 80.1 / 28.0 | identical |
| 3 | `1e844a4dd` | a condition's pure null/distinct tests asked once | @@S3@@ / 80.5 / 28.4 | identical |
| 4 | `5ef531965` | statement and return fallbacks as two tables, one dispatcher each (`legacy_route_table.rs`) | @@S4@@ / 83.2 / 27.8 | identical |
| 6 | `368bbc9ed` | the unreachable second copy of the seven call holds removed | @@S6@@ | @@I6@@ |
| 5 | `@@C5@@` | `legacy_c.rs` split into eight modules, move-only | @@S5@@ | @@I5@@ |

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

@@MEASUREMENT@@

## 6. Gate

@@GATE@@

## Files

Scripts (`scripts/`): `measure.sh`, `measure-binary.sh`, `measure-atomic.sh`,
`step-measure.sh`, `step-measure2.sh` (route runs), `identity.py` (identity
proof), `parity_diff.py`, `clang-census.sh` + `clang-census-run.sh` +
`clang_census.py` (Clang census), `gdb_sampler.py` + `profile-run.sh` +
`profile.py` (profile), `route-census-run.sh`, `tests-timing.sh`,
`build.sh`, `cargo-step.sh`, `split.py`, `move_check.py`, `update_ledger.py`.
Artifacts (`artifacts/`): the identity proofs, the Clang census and profile
summaries, the route census, the move check and the timing table.
