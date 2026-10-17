# W44-CALLEE-CROSS-UNIT-SUMMARY

Card base `fa04aab0fd43b242d2a996da2c6c507f1eb2a113`; prediction
[`PREDICTION.md`](PREDICTION.md) committed first at `d92349a56`.

## Route boundary found before implementation

The W4 inference (`qcore-c-gateway::legacy_summary`) derives validated
summaries inside one preprocessed translation unit. Its `ResourceSummaryRecord`
has a function, trust label and contract, but no home unit or source identity.
`LegacySafetyCertificate` re-derives that same unit only. The audit's binding
adds a compile-task and loaded-environment identity, again for one unit.

Transporting a record by name and source digest would be unsound. A C external
call can resolve to an interposed definition; a C99 external `inline` text can
be bypassed; and multiple configurations can define the same name. The
current compile database is a set of units, not a linked-program symbol table.
The run must therefore take **an explicit conditional premise** identifying
the one external definition that each call resolves to. That premise must be
retained in the verdict's assumptions and in the certificate's expected
context. Absent, ambiguous, static-only and external-inline-only identities
must remain uncontracted.

The second boundary is certificate containment: candidate home functions can
be Safe while their translation unit is Unknown, so a whole-unit Safe
certificate cannot authenticate them. A route needs per-function home evidence
holding the preprocessed source, request and loaded environment, and must
re-run W4 over that home unit. The caller certificate must then independently
re-derive the home summary before it re-derives the caller under its imported
copy. `ResourceContracts.functions` cannot carry that copy: source-assumption
replay checks it exactly against the `.qv` declarations. Imports need their
own channel and identity.

The W48 BEFORE/AFTER calibration is 6 call-only corpus functions at the route
ceiling and 10 with held position calls; system and slice 0. It is a dated
upper bound, not this card's current-base measurement.

## Implemented conditional route

The opt-in `QCORE_CROSS_UNIT_LINK_MAP` selects a versioned JSON link claim.
Its exact bytes are SHA-256 pinned. With no map, the original analysis path
remains in force. The audit first settles its refutation table, then reads
every selected configuration's main-source definitions. It exports a named
home only if its W4 row and derived summary are Safe, no declaration exists,
no second selected configuration defines the same name, and no file-scope
declaration gives it static or inline semantics. A failed or incomplete
second phase, or a changed refutation table, leaves the first-phase results
in force. Checkpoint composition refuses this opt-in mode.

Imports live in a separate inferred channel, with the map digest in the
environment. Caller-local definitions take precedence. Before issuing a Safe
certificate, the gateway reconstructs the home's raw, line-marked and
preprocessed source scope and its request digest, checks the loaded home
environment digest, re-runs W4 over the home, and compares the exact derived
contract. Certificate replay does the same before re-deriving the caller.
The published sidecar and each pinned unit report retain the named link
premise and map digest; configured current reports retain them in the
assumptions sidecar. Lean `QCore.CrossUnitSummary` models the conditional
resolution, uniqueness, declaration precedence and fail-closed identity
checks. The [corpus map](link-map.corpus.json) names five Safe homes serving
three call-only Unknown callers at the exact base; the
[refusal control](link-map-refusal.json) names an absent home.

The map is a **premise**, not evidence that a linker chose those definitions.
It must describe the actual linked program and interposition environment for
any conditional Safe result to apply. Source-only presence or uniqueness
cannot establish that fact.

## Measurement

The read-only BEFORE artifact is the detached `fa04aab0f` worktree's
`/tmp/qcore-rk-outer-base/private/rk-outer/measure-base/`. Its `head` records
`fa04aab0fd43b242d2a996da2c6c507f1eb2a113`; `corpus/run.json` records
pinned source `66910c10`, jobs 4, `/usr/bin/clang`, binary SHA-256
`336609c835dad7762e3729bfc699af561a61d1704f318e2db0f6fbb57e145ff7`.
All three route exits are 0. The system and slice invocation sidecars name
the same checkouts, compile databases and selections used by this card's
`scripts/measure.sh`. The BEFORE counts from the reports are:

| route | Safe | Pending | provisional Unsafe | Unknown | functions |
| --- | ---: | ---: | ---: | ---: | ---: |
| corpus | 941 | 0 | 9 | 3,884 | 4,834 |
| system | 97 | 0 | 4 | 347 | 448 |
| slice | 52 | 4 | 1 | 347 | 404 |

The exact-base corpus has three Unknown callers whose *only* diagnostic
leaves are uncontracted calls to Safe other-unit home rows:
`cio_chunk_hash` (`cio_file_hash`), `ctxTypeID_to_size`
(`LZ4_sizeofState`, `LZ4_sizeofStateHC`) and
`rd_kafka_share_consumer_closed_err` (`rd_kafka_error_code`,
`rd_kafka_error_destroy`). These are candidates, not predicted guaranteed
promotions: export and caller replay can still refuse them. The report has
10,249 `call to uncontracted function` leaves, but no typed parity stream,
so the complete held-position-call census remains to be measured.

AFTER measurement and full standing gate are pending the scheduled private
heavy-run slot.
