# W48, W44 and outer-contract stage 2 integration

This record measures the three cards together on main after W48 merge
`44e6701d6`, W44 merge `868bbcc0f`, stage 2 merge `d882f6e00`, and the
stage 2 evidence commit `ad526b58`. The measured source tip is
`ad526b5871c8cd05596c63d110932923431915d7`. The freshly built release
`quirin-audit` has SHA-256
`e93676faa20557c1dae09a12880c06292a60a2bcf523780f192fd8a80c3cc5f9`.
The opt-in W44 [link map](../w44-cross-unit-summary/link-map.corpus.json) has
SHA-256 `ddd0576cde87a5782f1fe0b9c4ce4efad4042b1ee54a483a471a3576f0698e7e`.
Its linked-program resolution, type, calling-convention and interposition
statement remains an explicit assumption. Stage 2's loaded-library
restrictions remain named human-review conditions for Pending; this
measurement does not certify a deployed plugin.

## Merged measurements

The corpus used pinned source `66910c10`, `--jobs 4`, the W44 link map,
`QCORE_OUTER_PROJECTION`, `QCORE_NULL_PROPERTY` and typed path parity.
`python3 scripts/run_pinned_audit.py` exited 0. Its run record and 2.2 GiB
parity trace are under `private/w48-w44-rk-integration/measure-combined/`.
Static custody is **955 Safe / 9 provisional Unsafe / 3,870 Unknown** over
4,834 functions. Against W48's isolated tip, W44 adds exactly its three
named Unknown-to-Safe rows: `ctxTypeID_to_size`,
`rd_kafka_share_consumer_closed_err` and `cio_chunk_hash`. The comparator
aligns all 4,834 functions, finds 41 changed diagnostic rows (the three
status moves and 38 still-Unknown callers with deeper obligations), and no
regression. Against W44's isolated tip, W48 adds exactly its 11 named
`rdkafka_admin.c` pointer-return rows and no other diagnostic change.
Private comparisons are `w44-on-w48-diff.json` and `w48-on-w44-diff.json`.

The all-loaded runtime corpus custody projection is **923 Safe / 32 Pending /
3,870 Unknown / 9 provisional Unsafe**. Null is **2,386 Safe / 163 Pending /
2,354 Unknown / 18 Unsafe** and combined is **806 Safe / 33 Pending /
4,055 Unknown / 27 Unsafe**. Static null and combined are respectively
2,549 / 2,354 / 18 and 839 / 4,055 / 27, with zero Pending. Both single-loader
scenarios have the same counts as all-loaded. The stage 2 comparison aligns
all 228 moved rows in each scenario: none added, lost or changed its
static-to-runtime status pair. Exactly the stage 2 card's 32 corpus custody
rows remain Pending; no newly Safe W48 or W44 row adds a Pending move.
`outer-stage2-vs-merged.json` is the private per-row record.

The merged system and tail-pause selections were run by
[`measure_routes.sh`](measure_routes.sh) with the same binary and both opt-in
settings; both exited 0. System static custody remains **97 Safe / 4
provisional Unsafe / 347 Unknown** and runtime null **209 Safe / 2 Pending /
243 Unknown**. All 448 configuration rows match W48's status and diagnostic
projection exactly. Its static custody/null rows and four runtime moved rows
match stage 2. Slice static custody remains **52 Safe / 4 Pending / 347
Unknown / 1 provisional Unsafe** and runtime null **174 Safe / 16 Pending /
216 Unknown**. All 404 configuration rows match W48's status and diagnostic
projection exactly; its static custody/null rows and 16 runtime moved rows
match stage 2. Private evidence is `system-row-check.json`,
`slice-row-check.json`, `outer-stage2-vs-merged-system.json`,
`outer-stage2-vs-merged-slice.json`, route diagnostics, projections and exits
under `private/w48-w44-rk-integration/`.

## Gate and scope

The first merged standing-gate attempt at `ad526b58` is retained under
`private/w44xu/gate-final-integration-1/`. Submodule, Rust runner (513 units,
526 summaries, 8,230 passed / 0 failed / 15 ignored), formatting, Clippy,
both reevaluation examples, and Lean audits (337/337) exited 0. Python's
374-test suite exited 1 on its one matrix-section assertion: the merged proof
tree had 337 audits but the matrix still said 336. The matrix was regenerated
from the merged tree, and all 27 focused matrix-section tests then passed.
This attempt is not a green gate. The corrected full-gate rerun at the same
source tip, `private/w44xu/gate-final-integration-2/`, exited 0 with all
eight steps 0: submodule, Rust runner (513 units, 526 summaries, 8,230 passed
/ 0 failed / 15 ignored), format, Clippy, both reevaluation examples, 337
Lean audits and 374 Python tests. The run's per-unit log directory is
`private/test-binaries/20261017T124438Z-631528/`. The
[`gate.json`](gate.json) and each card's `integration/gate.json` identify
the measured tip and all three merge commits. The acceptance matrix was
regenerated from the merged gate, recording 8,230 workspace tests, 337
Lean audits and 133 provider tests. The first attempt remains in the report
and private logs as a failed gate.
After the gate records, STATUS, matrix and index were written, the full
374-test scripts Python suite exited 0 again; the docs index check, JSON
parse and `git diff --check` also exited 0. `write_gate.py` requires the
recorded gated source tip and matching provider tree before writing a gate.

The merged `providers/` tree is byte-identical to stage 2 tip `a0ec4b37a`.
Its provider gate under the stage 2 worktree's
`private/rk-outer/gate-providers-v3/` passed 133 Rust tests, 25 Python tests,
contract checks, formatting, Clippy, package build and verification of all
five packages. The W48 and W44 cards did not change provider files.
