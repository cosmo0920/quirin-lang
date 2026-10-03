# Prediction (frozen before implementation)

Card GATEWAY-ANALYSIS-PATH-OPTIMIZATION, base `2ecd39531`. Written after the
route map's reading passes, the BEFORE corpus run and the wall-clock profile
of the corpus arbiter, before any implementation commit. (A first draft of the
step-2 reuse module was written while the BEFORE runs queued; it was set
aside uncommitted, out of the tree, until this file was committed.)

## Hard rule, restated

Every step is verdict-identical: the corpus, system and tail-pause slice
`resource-diagnostics.json`, every sidecar, `audit.log`/console output and
the typed-path parity streams (as multisets with Clang node addresses erased)
are **byte-identical** to BEFORE; no test or pin is edited; no Lean changes.
An optimization that would change any output is out of scope and becomes a
registered finding.

## Evidence the prediction rests on

* BEFORE corpus (`private/gapo/before1`): `resource-diagnostics.json`
  `d2f5a3f7…` (= main), 658 s wall at `--jobs 4` with the parity stream on.
* Profile of the same arbiter under `scripts/gdb_sampler.py` (1,206 samples,
  4,802 busy worker-thread samples; coordinator excluded): **42.9 %** of
  worker time is the contract environment's declared module texts being
  re-parsed and re-lexed (`qcore_contract::contract_loader::*` →
  `qcore_frontend::lexer::lex`), asked by `build_function_cfg_with` (32.9 %),
  `counted_users::evaluate_function` (7.5 %) and
  `call_exits::declared_no_return` (2.1 %); **16.7 %** is serde_json parsing
  Clang's AST output; **3.9 %** is waiting on Clang processes; the rest
  (36.5 %) is the analysis proper.
* The Clang map (ROUTE-MAP section 7): the whole-unit typed AST is launched
  and parsed once per *source scope*, and one unit runs under several scopes
  (analysis, the review-standing settle of a `Pending` row, the certificate
  self-check of a `Safe` unit, the audit's admission recheck); the
  record-layout dump is launched once per *consultation*; the pinned route
  runs the same line-marked `-E` twice per unit.

## Steps (each its own commit, each verified byte-identical on all three routes)

1. **Contract-environment readings once per module text** (qcore-contract):
   the nine `declared_*` / `*_obligations` / `*_tails` readers that share the
   reject → parse → mode → per-declaration `load_function` skeleton read one
   cached per-module reading, keyed by the exact (path, text) pair, preserving
   every error string and the order errors are met in. Expected: most of the
   42.9 % disappears. **Predicted corpus wall: −25 % to −40 %**; system route
   similar; slice −15 % to −35 %.
2. **Clang unit AST and record-layout dump reused across scopes** (gateway,
   new module `clang_reuse.rs`): keyed by the whole launch identity (Clang
   resolution environment, dialect, frontend request, analysed text compared
   byte for byte), successes only, thread-local, two units per thread, never
   for a text that could read a file. Expected: fewer AST launches and parses
   on Safe/Pending units and fewer layout launches. **Predicted: a further
   −3 % to −10 %** of corpus wall.
3. **Pure condition predicates computed once per condition**
   (`null_condition`, `distinct_condition` in the `if`/`while`/`for`/`do`
   arms). Predicted: below measurement noise (< 1 %).
4. **One documented dispatcher table** for the statement fallbacks
   (`expression_statement` rows 7–21 of ROUTE-MAP 2.1) and the return
   fallbacks (ROUTE-MAP 4, step 3), in exactly the current order, in a new
   module. No speed change predicted.
5. **Split `legacy_c.rs`** along the route map into cohesive child modules,
   move-only (after rebasing onto CG10-INDIRECT-ACCESS-SOLE-RESIDUALS). No
   speed change.
6. **Dead code**: the seven call holds repeated after the uncontracted arm
   (LC:8811-8879) are unreachable (same pure predicates, same callee, already
   returned before the gate). Removed only with the identity runs as the
   parity evidence; if the rebase makes this unclear, kept and registered.

Not done, and why (each registered if it stays open):

* **A cross-run Clang cache.** The measured Clang process share of arbiter
  worker time is 3.9 %; a fail-closed content-addressed cache (Clang binary
  identity, every flag, the input digest) would save at most that plus part of
  the parse, and its correctness surface (stale entries, provenance) is larger
  than its gain. Registered as a follow-up with the measurement.
* Hoisting `condition_access`, the repeated `classify_simple` probes of a
  refused return, or `elaboration_node` walks: they write parity rows or
  consume sites, so their repetition is observable; registered.
* `ClangIdentity::capture` caching: tests swap the executable behind one
  `QCORE_CLANG` path; an in-process cache could answer a stale identity.
  Registered.

## Totals predicted

* Corpus arbiter wall (median of three, `--jobs 4`, parity on): **−30 % to
  −45 %** against BEFORE.
* System route: **−25 % to −45 %**. Tail-pause slice: **−15 % to −40 %**.
* `run_test_binaries.py` run phase: **−5 % to −25 %** (the heaviest test
  binaries are source-import and route-parity tests whose cost is per-import
  Clang launches, which steps 1-2 touch only in part).
* Outputs: **byte-identical** on all three routes at every step; parity
  multisets identical; every test passes unchanged; Lean audits unchanged
  (286).
