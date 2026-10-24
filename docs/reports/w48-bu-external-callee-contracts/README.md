# W48-BU-EXTERNAL-CALLEE-CONTRACTS

Completed on `codex/w48-bu-external-callee-contracts`: the two predicted
conditional custody gains are measured, all raw differences are attributed,
and the full standing/provider gate passes after the approved test-pin repair.

This contract-only card appends libc-owned custody declarations for `writev`
and `thrd_current`. It declines `mtx_destroy`, whose actual global mutex needs
an admitted in-place mutex kind and an owner protocol across init/destroy
bodies. No gateway rule, analysis route, semantic effect kind or Lean definition
changes. Rule 38 is not used.

The launch BASE is `8991b7b58fcbd33da082bbb44e819d47a0fb2561`; the held BASE
analyzer was built from documentation-only descendant
`15cc50053af74ee560233e5a26cfafbc83b40046`. `base-code-data-equality.json`
binds its code and data to launch BASE. The implementation is
`5dbe4a381acd5c3197057030ff3f9735caa72335` on
`codex/w48-bu-external-callee-contracts`. The prediction was committed at
`a9e61adee` before any contract, pin or test edit. No result from another card
is used as this card's BASE or TIP measurement.

## Resolution and admitted assumptions

[RESOLUTION.md](RESOLUTION.md), the twelve configured preprocessing probes,
and `BASE-targets.json` distinguish declarations, raw discovered source names,
and analyzed bodies. The selected corpus calls resolve to POSIX `writev` and
system C11 `thrd_current`/`mtx_destroy` with `WITH_C11THREADS=1`. The system and
tail-pause configurations resolve the same declarations, but do not select the
three target bodies. The corpus TinyCTHREAD unit has **zero analyzed functions**
and **30 raw discovered names**: its implementations are compiled out. Those
names do not establish analyzed home bodies. No selected analyzed body for
these external primitives is replaced by a declaration.

`writev` borrows its descriptor, vector and count and returns no custody. Its
normal-return custody premise includes the reachable buffers: no transfer or
retention. It makes no guarantee about output success, complete writes, errno
preservation, buffer bounds, null safety, purity or cancellation. The actual
POSIX ABI must still agree with `ssize_t (int, const struct iovec *, int)`.

`thrd_current` has no resource arguments or custody result. The configured LP64
headers define `thrd_t` through `__thrd_t` as `unsigned long int`, supporting
this scalar `u64` projection. An incompatible scalar or pointer ABI withdraws
the declaration. The identifier creates no owning thread handle or proof of
thread lifetime, joining or synchronization.

These are named, revision-flexible **declared standard/ABI premises**, not
proofs of libc bodies, source extraction, Rust–Lean correspondence or a whole
C program. `mtx_destroy` remains Unknown: Borrowed would omit the mutex
lifecycle, while heap Released would describe the wrong obligation. The
configured `thrd_success=0` does not supply a missing mutex kind or global
owner protocol. The existing `CG25-IN-PLACE-RESIDUAL-PAIRS` follow-up remains
open, with a dated configuration clarification proposed for the integrator.

## Deliberate data and pin changes

Only two declaration sections are appended, preserving all earlier file bytes.
They add six clauses for `writev` and three for `thrd_current`.
`qcore_contract_corpus.rs` pins both counts; the 138-module corpus count is
unchanged. The resource loader's selected/v1/rejected expectation changes
`(66,495,331)` to `(66,497,331)`, and its expected-mode table explicitly records
both new declarations. The source count stays 66.

The actual held BASE CLI generated the libc lock update. Its 130 existing
flexible entries and all other fields are unchanged; exactly two entries are
added, for a total of 132. `libc-lock-attribution.json` records the generator,
whole-file hashes and new digests. Inventory synchronization, pin-check and
pin verification all exit 0. No policy or provider-owned declaration is moved
between crates; the libc include/inherit ownership boundary is unchanged.

The six production-data controls check normal custody and conservative refusal:
actual POSIX/C11 headers; removing each declaration; a live allocation leak;
released vector storage; a closed descriptor; pointer-valued thread-ID ABI;
and the undeclared global mutex destroy. Safe certificates independently
recheck. The synthetic Unsafe controls are not upstream defect claims.

## Own fresh native measurements

All six fresh native invocations exit 0. [native-complete.json](native-complete.json)
records exact commands, source identities, UTC windows, binary/artifact hashes,
input guards and each route's counts.

| Route | Units | Analyzed rows | BASE Safe / Unknown / Pending / Unsafe | TIP Safe / Unknown / Pending / Unsafe | Raw / not projected |
|---|---:|---:|---|---|---:|
| Pinned corpus | 185 | 4,834 | 999 / 3,826 / 0 / 9 | 1,001 / 3,824 / 0 / 9 | 4,921 / 87 |
| System | 15 | 448 | 99 / 345 / 0 / 4 | 99 / 345 / 0 / 4 | 454 / 6 |
| Tail-pause | 16 | 404 | 57 / 342 / 4 / 1 | 57 / 342 / 4 / 1 | 406 / 2 |

All unit statuses are unchanged: corpus 8 Safe, 166 Unknown, 7 provisional
Unsafe and 4 data-only; system 13 Unknown / 2 provisional Unsafe; slice 15
Unknown / 1 provisional Unsafe. Existing Unsafe observations are not promoted
to validated upstream defects. Every source/function inventory is unchanged.

Function tallies count analyzed rows, not whole-source safety coverage. Every
selected unit remains in the unit denominator. Raw discovered names,
`not_projected`, compiled-out bodies, unread units and provider support are
reported separately. No safety percentage is inferred. The fresh BASE planner
has 2,200 candidate bodies, same/cross baseline closure 6/6 and conditional
ceiling 383. Each target scored one direct leaf and one possible cross caller;
these conditional ceilings were not promised gains.

Exactly two analyzed rows change verdict, matching the committed prediction:

| Artifact | BASE → TIP | Sole removed unsupported leaf | Unchanged null scope |
|---|---|---|---|
| `librdkafka/tinycthread_extra.c#thrd_current_id` (unit 141) | Unknown → Safe | uncontracted `thrd_current`, origin 69 | unconditional, no null assumptions |
| `monkey-libevent/mk_core/mk_iov.c#mk_iov_send` (unit 158) | Unknown → Safe | uncontracted `writev`, origin 116 | conditional on inferred quant null-deref `#mk_io` |

[verdict-changes.json](verdict-changes.json) preserves actual full BASE/TIP
contexts and verdict tuples; [diagnostic-leaf-trades.json](diagnostic-leaf-trades.json)
preserves both removed leaves. There are no added diagnostics: corpus unsupported
27,465→27,463, unknown_alias 11,002, leak 40 and summary_conflict 2 unchanged.
System and slice have no diagnostic trades.

All 4,921 corpus inner null dictionaries are exactly unchanged, as are the 454
system and 406 slice dictionaries. Corpus static combined Safe 859→861,
Unknown 4,035→4,033, provisional Unsafe 27 unchanged. In the all-loaded outer
scenario custody Safe 957→959 / Unknown 3,838→3,836 (Pending 30 / Unsafe 9
unchanged), and combined Safe 824→826 / Unknown 4,040→4,038 (Pending 30 /
Unsafe 27 unchanged). The two other loaded scenarios have exactly the same
+2 Safe / −2 Unknown custody and combined movement; every other outer/null
field is unchanged. System and slice outer records are unchanged.

Four rows gain declaration-dependency annotations: these two promoted rows,
plus `mk_channel_stream_write` and `mk_channel_write` in `mk_server/mk_stream.c`.
The latter two stay Unknown with unchanged diagnostics. `writev` has three
dependent rows, one Safe; `thrd_current` has one dependent row, one Safe.
Consulted declarations 443→445, declared-assumption declarations 329→331 and
Safe rows resting on declared assumptions 423→425. These are annotations of
named premises, not discharged implementation proofs.

The possible cross callers do not gain: `rd_kafka_transport_ssl_threadid_cb`
keeps the `thrd_current_id` leaf at `rdkafka_ssl.c:2048`; `net_plain_writev`
keeps the `mk_iov_send` leaf at `mk_server/mk_net.c:64`. Import-name sets are
unchanged. `rd_kafka_sasl_cyrus_global_term` keeps `mtx_destroy` at
`rdkafka_sasl_cyrus.c:691`. [conditional-callers-and-mutex.json](conditional-callers-and-mutex.json)
preserves the complete rows. No TIP planner result or arithmetic ceiling gain
is claimed.

## Identity and artifact attribution

The BASE and TIP analyzers, complete typed environments, runtime selection,
compile databases, runner, profile, link map and input identities are held
privately and hashed in the receipts. All three typed environments were
independently exported from their actual builds. Exact byte reconstruction
passes for BASE plus two ordinary function entries, two flexible revision
entries and the literal two source suffixes. All other environment fields are
unchanged. Negative checks change a source comment or `writev` arity and obtain
a different digest. These checks establish the recorded preimages, not C truth.

The three `*-complete-attribution.json` receipts account for **every JSON
field**: zero unexplained fields, zero erased fields, zero masked fields.
Raw differences and full five-slot artifact checks remain private with hashes
and paths in the receipts. Corpus has all five actual artifacts; system and
slice have four actual files, with `monkey-source.json` absent on both sides.
Unchanged corpus monkey-source, source-effects and chunk-contract artifacts
are byte-identical; system/slice source-effects and chunk-contract files are
also byte-identical.

The shipped replay scripts explain identity changes through exact producer
preimages. Production source/proof Git blobs (1,523) are identical between
actual analyzer sources. Current compile-task build IDs are reconstructed
from the 31 actual typed task preimages and canonical CFG inventories; pinned
corpus legacy build IDs remain unchanged. Null context assumptions and scopes
are replayed with the changed function-table digest, and evidence versions
retain their actual build commits. Full declaration metadata and the two new
empty premise graphs/digests are reconstructed exactly.

The 45 corpus, four system and one slice effective reading environments are
reconstructed byte-for-byte from BASE plus the two declarations/source suffixes
and checked cross-unit environment hashes. All 122 / 24 / 2 typed receipt-ID
preimages are replayed; the restored receipt sets equal actual BASE. Changed
receipt ordering explains apparent span/predicate/route index changes without
removing any observation. These are identity checks, not claims that the C
reader or truth transport is proved.

Each route also has one empty Monkey lifetime-summary key appearing in two
callback-prepass locations. Replaying its recorded preprocessing command and
only the actual `MONKEY_BUILT` date/time expansion reproduces both complete
hash keys; payloads remain `{}`. The BASE/TIP local expansions are respectively
2026-10-23 23:53:47 / 2026-10-24 10:13:01 (corpus), 2026-10-24 01:18:35 /
11:36:42 (system), and 01:26:19 / 11:44:27 (slice). Each is within its actual
native window. The exact bytes and unchanged producer are bound by
`*-time-key-attribution.json`; no general timestamp mask is used.

`contract_dependencies` stores serialized contexts in a `BTreeSet`. After
reversing exact identity preimages, the comparison reapplies that same canonical
ordering only to its `hypothesis_evidence.contexts` arrays. Both system
`flb_output` compile configurations remain present; their build hashes change
sort order. The unchanged producer and both actual unique sorted arrays are
checked. The bounded inverse comparison is supplementary field accounting;
all published verdicts refer to the actual native artifacts.

The private corpus mirror uses the same prepared absolute paths for BASE and
TIP. Its four compile-database changes are exact literal prefix replacements;
`mirror-byte-transform-proof.json` retains the byte proof. The first corpus
attempt failed because the old compile database path spellings selected no
cmetrics units; it supplies no baseline or coverage claim. That failed attempt,
the canceled queued retry, and the successful fresh BASE are retained. Path
spelling can affect `__FILE__`; no equivalence to the original spelling is
claimed. The standalone Monkey copy is bound by 29 selected C files and seven
adapter/header files to the pinned Fluent Bit copy; it has no separate Git
identity. The slice is archive-import commit `34155955` (tree `8d1f3cd7`); the
absent original `90978ac6` object prevents an ancestry claim.

## Validation and retained attempts

The final gate runs on clean source
`36e91795e35b76db6ac89fd8af4d4e926612e7d8`, after the user-approved repair of
the pre-existing provider-view pin. Launch BASE and measured TIP both have 66
resource-view modules; the old Python test expected 65. The repair changes that
expectation to 66, explains `librdkafka/log_caller_frame.qv`, and explicitly
asserts its presence. All other assertions remain. [pin-repair-commit.json](pin-repair-commit.json)
binds authorization, patch and revision. This test-only commit does not change
production code, declarations, lock bytes or native inputs. Native TIP artifacts
remain labeled with their actual analyzer source `5dbe4a381`, not the later
gate or report commit.

The earlier sandbox gate is retained as **failed**, exit 1. Root Rust measured
8,485 passed / 108 failed / 15 ignored across 546 units and 559 summaries; 47
units failed, with no build/capture problem. Failure classification was 91
LeakSanitizer-under-ptrace failures, 15 Operation-not-permitted failures and two
generic exit-assert failures (`ct_corpus`, `source_scalar_calls`); the latter
are not retrospectively called LeakSanitizer failures. Provider Rust measured
132 passed / 1 failed / 1 ignored; its confined-profile test could not unshare
user/network namespaces. Provider Python measured 25 tests, 24 passing and the
one stale 65/66 pin failure. Every other gate command passed, including 369
Lean audits and 374 root Python tests. Exact commands, per-command exits,
source states and private logs for both attempts are in
[validation-receipt.json](validation-receipt.json). The unconfined retry keeps
sanitizers enabled and reruns every command; it does not skip failed tests or
relax namespace assertions.

The full unconfined standing/provider gate passes: 8,593 Rust tests (546 units / 559 summaries, 0 failed, 15 ignored), 369 Lean audits, 374 root Python tests, 133 provider Rust tests (0 failed, 1 ignored) and 25 provider Python tests; fmt, Clippy, both reevaluation examples, inventory and all five package builds/verifies exit 0.

All 19 final command exits are **0**. The command arrays below use exactly
the recorded arguments; the receipt also retains the environment, timestamps
and source-before/after guards. Heavy work holds the shared card lock, with
`CARGO_BUILD_JOBS=6`, native/test jobs 4 and the 40-GiB disk floor.

| Command | Exit | Evidence |
|---|---:|---|
| `clang --version` | 0 | `gate-2-unconfined/clang-version.txt` |
| `python3 scripts/run_test_binaries.py --jobs 4 --log-dir $P/gate-2-unconfined/rust-units` | 0 | `gate-2-unconfined/rust.log` |
| `cargo fmt --all -- --check` | 0 | `gate-2-unconfined/fmt.log` |
| `cargo clippy --all-targets -- -D warnings` | 0 | `gate-2-unconfined/clippy.log` |
| `cargo run -p qcore-compiler --example reevaluation_probe` | 0 | `gate-2-unconfined/reevaluation-probe.log` |
| `cargo run -p qcore-compiler --example reevaluation_followup` | 0 | `gate-2-unconfined/reevaluation-followup.log` |
| `python3 scripts/check_lean_audits.py` | 0 | `gate-2-unconfined/lean.log` |
| `python3 -m unittest discover -s scripts/tests` | 0 | `gate-2-unconfined/python.log` |
| `cargo test --manifest-path providers/Cargo.toml --workspace --locked --offline --no-fail-fast` | 0 | `gate-2-unconfined/provider-rust.log` |
| `cargo fmt --manifest-path providers/Cargo.toml --all -- --check` | 0 | `gate-2-unconfined/provider-fmt.log` |
| `cargo clippy --manifest-path providers/Cargo.toml --workspace --all-targets --locked --offline -- -D warnings` | 0 | `gate-2-unconfined/provider-clippy.log` |
| `python3 providers/contracts.py check` | 0 | `gate-2-unconfined/inventory.log` |
| `python3 -m unittest discover -s providers/tests` | 0 | `gate-2-unconfined/provider-python.log` |
| `python3 providers/package.py build --output $P/gate-2-unconfined/packages --target-dir $P/provider-release-target` | 0 | `gate-2-unconfined/packages.log` |
| `python3 providers/package.py verify $P/gate-2-unconfined/packages/qcore.libevent` | 0 | `gate-2-unconfined/verify-qcore.libevent.log` |
| `python3 providers/package.py verify $P/gate-2-unconfined/packages/qcore.chunkio` | 0 | `gate-2-unconfined/verify-qcore.chunkio.log` |
| `python3 providers/package.py verify $P/gate-2-unconfined/packages/qcore.cmetrics` | 0 | `gate-2-unconfined/verify-qcore.cmetrics.log` |
| `python3 providers/package.py verify $P/gate-2-unconfined/packages/qcore.ctraces` | 0 | `gate-2-unconfined/verify-qcore.ctraces.log` |
| `python3 providers/package.py verify $P/gate-2-unconfined/packages/qcore.librdkafka` | 0 | `gate-2-unconfined/verify-qcore.librdkafka.log` |

`$P` is the private evidence root below. The root Rust summary is
`gate-2-unconfined/rust-units/summary.json` (one log per unit); provider counts
sum the 43 top-level libtest result lines, including doc-tests. Submodule init
completed before preparation; the pinned libevent revision is
`d82464a277d0f42703702c4dfd9af6af38595a83`. Focused controls in
`final-campaign-2/focused-1.log` pass all six tests with configured Clang 21.


The interpreted effects reuse the unchanged audited
`DeclaredAbiAgreement`, `BorrowLiveness`, `CallInstantiation` and
`KindedCustody` definitions. `existing-lean-boundary.json` and the actual audit
receipts bind their files and audit registrations. No project axiom,
placeholder or new vacuous theorem is introduced.

The first focused compiler attempt exposed a missing expected-mode table entry
for the two declarations. Its failing log is retained; the table was completed
explicitly and the retry ran both compiler binaries with `--no-fail-fast`:
10 loader tests and five corpus tests passed. No assertion was weakened.

The final post-gate input rehash checks all 46,547
original primary-input files and 4 host-header files: zero content/path
changes. Both Clang command identities are unchanged. All 14,706 original
regular files in the private mirror, including Git metadata, are unchanged.
The raw after receipt also lists 13 after-only file symlinks because the before
manifest inventoried regular files and the after enumerator follows file
symlinks. All 13 resolve to original byte-identical regular targets. Ten
tracked Fluent Bit link texts are additionally bound by the pinned clean Git
identity before/after. Three Monkey build-library link texts have no original
before snapshot; current donor text and target bytes agree, but temporal
link-text equality is not claimed. These records are retained in
[inputs-after.json](inputs-after.json) and
[mirror-symlink-scope-review.json](mirror-symlink-scope-review.json); the before
manifest is not retroactively changed.

All card-owned background controllers have finished: the native campaign's
native portion exits 0 (its enclosing controller exits 1 for the retained
sandbox gate failure), and the final unconfined gate controller exits 0.
Every native child invocation and final gate command has a completed receipt.
The final documentation commit follows the clean tested source and changes
only this report directory; no native artifact is relabeled with that commit.


## Integration and follow-ups

`integration-amendments.json` proposes exact ledger text and one newest-first
increment paragraph for the integrator. Root STATUS, increment and acceptance
matrix files are not edited by this card. Historical checkpoint claims remain
owned by their reports. No merge, push, route deletion or optimistic fallback
is performed.

Likely conflict files are the two libc contract modules, their shared lock,
`compiler/tests/contract_loader.rs` and `compiler/tests/qcore_contract_corpus.rs`.
The new gateway test and this report are additive. The approved provider test repair also touches
`providers/tests/test_contract_load_path.py`. Append the dated observations to
existing `CG25-IN-PLACE-RESIDUAL-PAIRS` and `W48-BOTTOM-UP-RESIDUAL-PLAN`;
retire neither and add no new follow-up (matrix count delta zero). Source
binding, libc implementation truth and Rust–Lean correspondence retain their
existing open boundaries.

Private evidence root:
`/home/cosmo/.codex/worktrees/c1e4/quirin-lang/private/w48-bu-external-callee-contracts/`.
The `scripts/` helpers accept `--evidence-root` pointing to that retained tree.
For example, from the repository root:

```sh
P=/home/cosmo/.codex/worktrees/c1e4/quirin-lang/private/w48-bu-external-callee-contracts
R=docs/reports/w48-bu-external-callee-contracts/scripts
python3 "$R/attribute_environments.py" --evidence-root "$P"
python3 "$R/derive_identities.py" --evidence-root "$P"
python3 "$R/attribute_readings.py" --evidence-root "$P"
python3 "$R/check_contract_metadata.py" --evidence-root "$P"
python3 "$R/attribute_time_key.py" --evidence-root "$P" --replay-output-root /tmp/w48-time-replay-fresh
python3 "$R/finalize_attribution.py" --evidence-root "$P"
python3 "$R/collect_validation.py" --evidence-root "$P"
```

The time replay directory must be fresh. These commands reconstruct private
receipts; they do not rerun the measured analyzer or replace its artifacts.
[evidence-catalog.json](evidence-catalog.json) binds the published compact
receipts, replay scripts and private log/artifact locations. Raw records are
retained privately rather than replaced by a normalized verdict export.
