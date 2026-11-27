# W48 batch 2 and RK interceptor candidate integration

Integration measured source `854920e3e12a86c34231d4b46fa42918e29044ca`.
W48 merge `654e2b7ee` follows isolated tip `b0a8afbfe`; RK merge `854920e3e`
follows isolated tip `83c3369af`. The only merge conflicts were independent
STATUS and increment insertions and acceptance-matrix provenance. Both cards'
tracking amendments and historical measurements are preserved.

The first W48 integration attempt is retained under
`private/w48-bottom-up-batch2/gate-integration-w48-1/`. Submodule setup exited
128 because sandboxed Git metadata was read-only. Rust ran every one of 513
units and 526 summaries, with 8,128 passes, 107 failures and 15 ignored,
exit 101. Native sanitizer execution and user-namespace confinement were
blocked by the sandbox. Format, Clippy, both reevaluation probes, 337 Lean
audits, 374 Python tests, documentation index and Clang recording exited 0.
The separately recorded submodule retry exited 0. No sanitizer, confinement
policy or assertion was disabled.

The complete host-access W48 rerun at `654e2b7ee`,
`private/w48-bottom-up-batch2/gate-integration-w48-2/`, exited 0 at all ten
steps: 513 units, 526 summaries, **8,235 Rust passed / 0 failed / 15 ignored**,
**337 Lean audits** and **374 Python tests**.

The combined host-access standing gate at `854920e3e`,
`private/w48-bottom-up-batch2/gate-integration-combined-1/`, also exited 0 at
all ten steps: **8,240 Rust passed / 0 failed / 15 ignored**, 513 units and
526 summaries, **338 Lean audits** and **374 Python tests**. Commands were
`git submodule update --init --recursive`,
`python3 scripts/run_test_binaries.py --log-dir <gate>/test-binaries`,
`cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings`,
both compiler reevaluation examples, `python3 scripts/check_lean_audits.py`,
`python3 -m unittest discover -s scripts/tests`,
`python3 scripts/build_docs_index.py --check` and `clang --version`.
The existing W48 gate helper sets debug-symbol and incremental-cache options
without disabling assertions or tests.

The fresh provider gate under
`private/w48-batch2-rk-integration/gate-providers-1/` passed all twelve steps:
**133 Rust passed / 0 failed / 1 ignored**, **25 Python tests**, locked/offline
workspace check and tests, formatting, Clippy, contract inventory check,
package build and verification of **all five packages**. The provider tree
is byte-identical to RK tip `83c3369af`. The merged gate is recorded in
`gate.json` and both cards' `integration/gate.json`; the merged counts differ
from their isolated gates, so those records retain the merged source and both
merge identities explicitly.

The fresh release build exited 0 using
`cargo build --release --locked --offline -p qcore-audit --bin quirin-audit -j 3`
with the dedicated W48 target directory. The immutable combined binary is
`private/w48-batch2-rk-integration/quirin-audit-854920e3e`, SHA-256
`daf942d7d29cbf6e8460691b9683b458bae1804d75851ab9089c7d0a39b82727`.
Its build log, exit and source identity are retained beside it.

## Combined measurements

All three fresh runs exit **0**, with four audit workers. Corpus uses pinned
66910c10; system uses 046accf6 and slice uses 34155955. Their complete commands,
selected environments, source/configuration identities, six-home link map and
binary digests are archived under `artifacts/merged/`. Raw logs and reports are
at `private/w48-batch2-rk-integration/measure-combined-1/`. The measurement
helper asserts the source HEAD and immutable binary remain unchanged after
each route. The actual-link/type/calling-convention/interposition premise of
the W44 map remains an explicit assumption of every conditional result.

| Route | Static custody Safe / Pending / Unknown / provisional Unsafe | Runtime custody, all loaded |
| --- | --- | --- |
| Corpus, 4,834 rows | 959 / 0 / 3,866 / 9 | 927 / 32 / 3,866 / 9 |
| System, 448 rows | 97 / 0 / 347 / 4 | 97 / 0 / 347 / 4 |
| Slice, 404 rows | 52 / 4 / 347 / 1 | 52 / 4 / 347 / 1 |

Every custody status and complete diagnostic equals the completed W48 batch 2
measurement, including duplicate configuration rows. Against the completed RK
measurement, the four custody gains remain `gf2_matrix_times`,
`gf2_matrix_square`, `LZ4_compressHC` and `LZ4_compressHC2`. Only the two LZ4
wrappers also move combined custody/null from Unknown to Safe; both matrix
helpers remain combined Unknown. All **4,921 / 454 / 406** complete null-property
result objects are unchanged on corpus/system/slice respectively. Those are
the null and combined row sets; custody has the denominators shown above.

All-loaded runtime combined counts, in Safe / Pending / Unknown / Unsafe
order, are **808 / 33 / 4,053 / 27**, **63 / 2 / 385 / 4**, and
**31 / 2 / 372 / 1**. These retain the null property's own findings and do not
turn a provisional resource violation into a validated upstream defect.
Every runtime moved-row object is identical to RK on each of the three
scenarios (`all`, `flb_external_plugin`, `rk_plugin_library`); the scenario
totals incorporate W48's custody gains. No new runtime Pending movement or
Pending-to-Safe promotion occurs. All three loader pins and the exact candidate
inventory digest are in force on every route: **14 lists / 14 aliases**, zero
held loaders, static list invocation still unsupported. No deployment callback
membership is certified.

## Full artifact and integration review

`comparison/` retains the duplicate-preserving status/full-diagnostic and
null/runtime comparisons; all checks exit 0. `candidate-selection.json` retains
the exact source/digest standing. `corpus-five-diff.json.gz` carries the full
five-artifact diff against the retained W48 baseline, including raw contract
keys. `corpus-artifact-review.json` accounts for every difference:

* All 185 diagnostic units differ only in the imported homes' environment
  identity. Their six home names, raw/preprocessed source hashes, map digest,
  assumptions and every other field are unchanged.
* Three source-effect rows change configuration and derived build/task,
  obligation, witness and evidence identities. Their complete remaining
  fields agree after the documented checkout-prefix/identity comparison.
* The 64 normalized contract-source modules have identical text with no key
  collision. Raw keys retain 60 checkout-prefix additions and 60 removals;
  the manifest and audit selection are unchanged. Contract dependency,
  revision, pending-review and link sidecars differ only by recorded checkout
  prefixes and the corresponding home environment identities.
* Source wrappers/callbacks and chunk results/teardown are unchanged.

`configured-artifact-review.json` checks every report field over system's 15
and slice's 16 units. Complete contracts agree after checkout-prefix comparison;
configuration/resource-contract-source identities and review-hold module
prefixes are the only report changes. Configured chunk and source-effect
artifacts are byte-identical. The current route emits no monkey-source
artifact; its invocation artifact is retained instead. Both baseline and fresh
primary reports are compressed under `artifacts/` without rewriting outputs.
The outer definitions and inventory are checked separately by the runtime
projection and candidate-selection evidence; the corpus resource-contract
source inventory above has its own 64-module scope.

The measurement, comparison, archive and full-field review recipes are retained
in `scripts/`. Their historical private paths identify the exact retained
inputs used here. The matrix was regenerated from this integration's `gate.json`:
797 named follow-ups and 338 Lean audits. The required fresh post-matrix Python
replay passes **374 tests**, exit 0; raw log and exit are
`private/w48-batch2-rk-integration/closeout/python-post-matrix.{log,exit}`,
with a compressed log and command outcomes retained under `gate/`. Final
documentation-index and staged/unstaged whitespace checks are recorded there.
Neither card retires the outstanding detach decision or the
static interceptor-list closure. The two cards' tracking amendments remain
applied; the reconciliation residual sweep finds no unregistered residual
originating in either card (its broader scan is retained privately).
