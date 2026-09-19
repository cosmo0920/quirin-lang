# B01–B04 integration

Implementation and integration record for the four Cycle-19 call-position cards,
requested 2026-09-20. Base: `8e633261fd216a76d4d25390b0dadeda891c59aa`.

## Scope and dispatch

B01 return expressions, B02 branch conditions, B03 loop/switch conditions, and
B04 nested call arguments are assigned to GPT-5.6 Sol agents at medium effort
in isolated worktrees. Three worker slots are available; B04 follows the first
completed worker. The user explicitly selected these four cards. The planning
recommendation to pair B02 with W42-INDIRECT-ACCESS is not an authorization to
claim that separate card complete: indirect-access blockers remain in scope
only where an individual B card explicitly models and measures them.

## Baseline reproduction

The integrator rebuilt the release audit binary from the unchanged base and
ran it against the existing pinned source, compile database and ChunkIO package.
The audit exited 0; its command, binary digest, source commit and timestamps are
in [baseline-run.json](baseline-run.json). Both verdict-bearing artifacts
(`diagnostics` and `chunk-contracts`) reproduce the wave-37 baseline byte for
byte. `source` is also byte-identical. The two remaining artifacts differ in
checkout/configuration identities: their complete structural comparison is
[baseline-artifact-diff.json](baseline-artifact-diff.json), including normalized
contract-module keys. No module content, selection or manifest entry changed.
The raw comparison is [baseline-byte-identity.txt](baseline-byte-identity.txt).

Private raw artifacts and logs are in
`/home/cosmo/Documents/GitHub/qcore-c/private/b-series/baseline/`.
These are measured baseline evidence, not an AFTER result or a gate for the
four implementations. Raw units remain unpromoted; provisional Unsafe findings
are not validated upstream defects. Lean rule models retain the extraction and
Rust correspondence premises of evidence rule 28.

## Review requirements

The integrator reviewed the draft Rust and Lean rules before acceptance.
B01 must reject calls skipped by ternary/short-circuit evaluation, retain the
synthetic-name collision guard, avoid a synthetic escape for resource-free
returns, and cover the opaque-field arm responsible for the measured getter
admissions. B02 must preserve both unknown-result branch successors and every
call effect; its old releasing-call refusal pin needs an explicit replacement
assertion. B03 must distinguish abrupt loop exits from condition-false exits
and test `continue`, including the `for` increment. B04 must retain every
sibling nested call's liveness and alias obligations and justify arbitrary
sibling order under its declared identity-effect premises.

The final integration record below must say how these requirements were
checked; this list itself is not verification evidence.

## Integration state

Implementation, review and combined measurement are complete. The final standing gate is recorded below; main integration follows this evidence commit.

## Integration review and merge attribution

The source merges are reachable commits: B02 `340d0b9d`, B03 `e73e96a2`,
B01 `70017e35`, and B04 `77443afe`. The intermediate corpus audit used
`77443afeb236a3818a4044fba6e6e2465a43a39e`; final measurement uses `18b142b8`. The first combined build found a
missed diagnostic-example consumer of B04's `Option` → `Vec` change; the
integrator changed its printer to iterate every nested call. This does not
change analyzer semantics. The final standing gate records the corrected
source commit independently. Exact merge identities are listed in
`integration-history.txt`. Later report and matrix commits change no Rust or
Lean.

B02/B03 conflicted only in the shared helper's documentation and test list.
The integrator retained the controlling-expression documentation, the complete
B02/B03 behavior and B02's `_Alignof` negative control. B01 and B04 merged
without semantic conflict. Competing isolated matrix snapshots are not merged
arithmetically: the final measured tree regenerates the matrix. Combined
focused checks passed **427 unit tests and 69 source tests**, exit 0; private
log `private/b-series/all-b-merge-tests.log`. Earlier merge checks passed 49
source tests (B02/B03) and 47 source tests (B01/B02/B03).

Review changed B01's draft before acceptance: conditional calls stay refused,
the reserved-name guard precedes every new route, and scalar/opaque results
use a `NoOp` exit rather than a synthetic escape. Its direct opaque-field arm
is bounded to one member selection. The nonvacuous production-shaped typedef
fixture fails when that admission is removed; a simpler field fixture had
already been admitted by the older neutral-expression rule and was insufficient.
The ordinary opaque-read classifier retains alias obligations and the
untracked-base premise without establishing resource identity.

B03's proof distinguishes normal condition-false exits from abrupt body exits;
source tests exercise repeated effects, exits, `continue` and `for` increments,
and the `do`/`while` break bypass. B04 applies and validates every neutral sibling
call; its tests make the second sibling carry the liveness and may-alias
obligations. Its Lean identity theorem quantifies over arbitrary finite sibling
lists. These models do not extract the Rust implementation or prove all C
semantics: the correspondence and extraction premises remain explicit.

B02 initially omitted its module from the aggregate `QCore.lean` import despite
passing its individual audit. Follow-up `056e3a43` adds the import and repeats
all 146 isolated Lean audits, including `AuditAll`; the combined run covers all
three newly added modules. B04 extends an existing audit rather than adding one.

The first combined gate at `77443afe` is a failed build, not test evidence:
`cargo test` and Clippy exited 101 on the diagnostic example's stale optional
nested-call pattern, and Lean exited 1 on downstream `DereferencedResult`
references to B04's renamed premise. There were zero Rust result summaries.
Logs and every exit remain in `private/b-series/combined-gate/`. The example
printer repair is `9831463a`; the Lean repair updates downstream names to the
one-level rule instead of retaining a misleading "only one call" name with a
new meaning. Corrected measurements are recorded separately below.


The first compiled combined Rust run also exposed two old expectations. B01
incorrectly admitted `return 1 + counted(p)` using a locally defined recursive
callee's provisional summary; `d7ed1a45` restricts only its additional scalar-call
arm to a callee without a local definition, retains the existing W4 refusal,
and adds a negative test and matching Lean premise. The earlier combined corpus
run is therefore intermediate evidence; the repaired analyzer is remeasured.
B04 intentionally changes the old O04 sibling-call refusal; its source-level pin
must become a positive sibling case while retaining the deeper-nesting refusal.
The original failing logs remain separate from the final gate.


## Final corpus result and prediction miss

The repaired audit at `18b142b8` exited 0. It measured **221 Safe / 84
provisional Unsafe / 4,529 Unknown projected functions**: eight Unknown → Safe,
zero other function moves and zero unit moves. Units remain **2 Safe / 42
provisional Unsafe / 137 Unknown / 4 data-only**, with raw units unpromoted.
Diagnostics are leak **272**, unknown_alias **4,309**, unsupported **49,735**,
summary_conflict **1**. Against the byte-reproduced baseline, **18 leaves are
removed and none added or exchanged**. B01 contributes twelve return leaves
and seven movers; guarded B02 contributes six branch leaves and
`flb_input_chunk_down`. B03 and B04 contribute no corpus delta.

The original combined forecast got all eight mover identities right but missed
the final leaf count: **57 predicted, 18 measured**. Its first intermediate run
matched 57, but the local-definition guards restored 39 branch refusals. The
full coordinate multiset is in `artifacts/guard-correction-delta.json`; this is
a scope correction, not an unexplained re-pin. B01's original zero-move
forecast and B02's original wrong mover name remain visible in their reports.

All five final artifacts are archived with deterministic gzip, full hashes,
source commit, binary hash and command in `artifacts/`. `chunk-contracts`,
`source`, `source-effects` and `contracts` are byte-identical to the integrator's
fresh BEFORE; only `diagnostics` changes. Private final raw artifacts and log:
`/home/cosmo/Documents/GitHub/qcore-c/private/b-series/repaired-after/`.
The intermediate `combined-after/` run remains private and is not the final
baseline. The final blocker census is `blocker-census.txt`; the dominant
indirect+branch pair remains **196** functions. No registered follow-up retires,
and the 269-entry registry and CT mapping/proof/native counts remain unchanged.
Full-language conformance remains false.

## Final standing gate

Frozen source `18b142b8d055a385985c68b392f449d60d3e2b99` passed the complete
combined gate: **4,845 Rust passed / 0 failed / 10 ignored**, **248 strict
result summaries across 246 targets** (237 running plus 9 doc), and **148 Lean
audits**. The source hashes in `source-hashes.json` still match after all report
merges. Submodule initialization, workspace tests with `--no-fail-fast`, fmt,
all-target Clippy with `-D warnings`, both reevaluation examples and Lean all
exited 0. Exact command arrays, timings and private absolute log paths are in
`gate-facts.json`. The run used an elevated process outside the sandbox's ptrace
restriction so LeakSanitizer could execute, without disabling it.

The acceptance matrix is regenerated from `gate.json`, then the Python evidence
suite passed **305 tests**, exit 0, against that matrix. Its captured result is in `gate-facts.json`.
The 122 provider tests are carried evidence, not a new provider run. Clang
21.1.8 was invoked in this checkout during the combined gate. Every B card has
an `integration-gate.json`; its note distinguishes the card's borrowed colocated
Clang recording from this actual combined recording. Isolated gates remain
frozen with their own counts. The failed compiled intermediate gate measured
4,840 passed / 2 failed / 10 ignored; its failures and repairs above are retained,
not counted as final validation.

The four report reconciliation lists are decided item by item in
`reconciliation.json`, including explicit declines of overbroad amendment text
and standalone increment proposals. No follow-up retires and no umbrella
residual is introduced. Attribution is independently checked by
`scripts/verify_attribution.py`; its output is archived in `artifacts/attribution.txt`.

The residual sweeps before and after final report reconciliation are byte-identical;
the archived `residuals-*.txt` retain the existing untracked-name inventory. No
retirement or new unresolved name is hidden by this integration. Reconciliation
checks pass for all 27 items and 26 content claims. An initial reconciliation
check rejected the deferred-decision enum `open`; changing it to the supported
`deferred` spelling preserved every ground and the repeated check passed.
