# Providers rules

Current rule text, routed by the [rule router](README.md), which says what to
read. Rule numbers and anchors are stable; backticked paths are repository-relative.

<!-- generated:rules:start — edit docs/agents/rules/<id>.md, then run python3 scripts/gen_rule_docs.py -->

<a id="constraint-16"></a>

<!-- preserved:constraint-16:start -->
16. **P02-FLAKE:** six provider/host rows are load- or parallelism-sensitive
    and are verified in isolation, never weakened: the `RLIMIT_NPROC`
    conformance test (it counts every process of the real UID, and keeps a
    deterministic companion), `vp04_vp07_vp08_process_lifecycle_and_payload_budgets`
    (wall-clock, measured failing at load 50 and passing at load 37 on the same
    binary), the confined-fixture write test (`Io(Broken pipe)` under load),
    `real_packages_preserve_unknown_and_reject_mutations` (`ExecutableFileBusy`:
    a sibling test in the same binary executes the file it mutates), and
    `enforced_process_limits::memory_growth_beyond_the_declared_limit_is_bounded`
    (added wave 20; the same `Io(Broken pipe)` shape as its sibling row, at
    load ~40). `remaining-p03` §9.1, `remaining-p03-f`,
    `wave17-gen-ctreasons` §4.1, `wave17-corpus`.
    **The sixth, landed at the wave-35 close in card
    `VERIFIER-HOST-CONTROL-BUDGET`'s own words:**

    > `closed_input_does_not_mask_descendant_timeout_or_accept_a_partial_request`
    > (`crates/qcore-verifier-host/tests/standalone_host.rs`) is a **sixth**
    > load-sensitive row, and the only one sensitive at the *top* of the range:
    > its first arm's `assert!(start.elapsed() < Duration::from_secs(1))`, a
    > fixed one-second guard over a 40 ms deadline, was measured failing 3 of 6
    > repetitions at 1-minute load ≈100 on a 16-CPU host
    > (`docs/reports/verifier-host-control-budget/README.md` §5). Its *other*
    > historical sensitivity — the large-response admitting control failing when
    > the host was **least** loaded — was a transport defect, not a host
    > condition, and was repaired by card `VERIFIER-HOST-CONTROL-BUDGET`; do not
    > re-file that half here.

    **This replaces, rather than re-issues, the wave-34 paragraph** that
    recorded the row beside the five as not-a-load-flake
    (`HOST-ROW-STARVATION-DESCRIPTION`, card `CT-CTWIN-LANE`'s request, and the
    reason it is replaced is that the mechanism is now *known*). What that
    paragraph got right is kept and what it got wrong is named. Right: the
    large-response failure was real, was not a flake to re-roll, and was **not**
    a weakness in the `VERIFIER-HOST-CLOSED-INPUT-DEADLINE` repair, whose
    assertions were never weakened — `VERIFIER-HOST-CONTROL-BUDGET` changed
    `crates/qcore-verifier-host/src/lib.rs` **only**, leaving the test file
    untouched. Wrong: "deterministically 3/3 in isolation" was three samples of
    a biased coin — 22 pre-repair isolated repetitions measured **7 pass / 15
    fail** — and card `MUTEX-SHARE-CROSS-ARMS` independently saw the row red in
    isolation, red under a full suite **and** green under a full suite on one
    host in one session. The mechanism was never a host throughput limit as
    such: an **8 KiB** pipe capacity against a **1 ms** backoff quantum put a
    ceiling on the 8 MiB-each-way arm, repaired with `poll(2)`, after which the
    control passes **38/38**. Two successors carry what stays open —
    `VERIFIER-HOST-CONTROL-MARGIN-UNPINNED` (nothing pins the 42–112 ms margin
    against the 1,000 ms budget, which is exactly how the defect survived wave
    33 behind a published `0 failed`) and `HOST-PIPE-CAPACITY-UNEXPLAINED` (the
    8 KiB is itself unexplained, so the defect's *size* is a function of an
    unexplained number). The standing instruction is unchanged and now applies
    to the first arm: a gate report **states which it saw**, and nothing is
    discharged by shrinking the response, raising the deadline or deleting a
    control (invariant 1). A seventh row is **measured but deliberately not
    added**: the `RLIMIT_NPROC` row above could not be cleanly verified during
    wave 35 at all, because it counts every process of the *real UID* and the
    wave's five sibling cards were **inside its measurement**, not ambient load
    (`VERIFIER-HOST-NPROC-ROW-UNVERIFIABLE-UNDER-WAVE`). `remaining-p03` §9.1,
    `verifier-host-control-budget` §5 and §7.1, `mutex-share-cross-arms` §10,
    `stability-reeval-1` §2.1 and §7.1, `docs-compact-6` §8,
    `c-gateway-return-call-w4`.
    **A second, independent measurement of the same sixth row, by card
    `W36-CORPUS-CONTRACTS-3` (wave 36).** The card proposed this row's
    text as *adding* the row — "joins this list as its **sixth** row" — and
    that half is **declined**, because the row had already been the sixth
    since the wave-35 close and a list cannot gain the same member twice; the
    card offered the alternative reading itself and asked only that the
    measurement land somewhere either way. It lands here, in the card's own
    numbers: `closed_input_does_not_mask_descendant_timeout_or_accept_a_partial_request`
    (`crates/qcore-verifier-host/tests/standalone_host.rs:175`) asserts a
    **one-second wall-clock bound** over an 8 MiB pipe write and a process
    spawn against a 40 ms declared timeout, **failed once inside a full gate at
    load average ~40 on 16 cores** with sibling cards building, and passed
    **12 of 12** re-run in isolation at load 23.7 to 27.6. Verify it in
    isolation; never weaken the bound. This is the row's **first** measurement
    at the middle of the load range — wave 35 measured it at ≈100 and at ≈6.5 —
    and it is post-`poll(2)`-repair, so the repair narrows the failure window
    without closing it (`STANDALONE-HOST-CLOSED-INPUT-LOAD`).
    **Load is not a threshold for this row, and wave 36 is the wave that shows
    it.** Three further observations sit beside the red one, all post-repair:
    card `JOINED-PROVENANCE-CALL-FRAMES` saw it **green twice** in the full
    `--no-fail-fast` suite at load **≈38 on 16 cores** with sibling cards
    building — *below* the ≈40 at which its sibling saw red, and it explicitly
    did **not** reproduce that red — and the wave-36 close saw it green again in
    its own full-suite gate. So one wave produced red at ≈40, green at ≈38,
    green 12/12 isolated at 23.7–27.6, and green in a fourth full suite. Two
    green runs at high load are consistent with **both** a fixed row and a
    load-sensitive one, which is the reason this row stays on the list and the
    reason no load figure here may be read as a bound. Do not "resolve" the
    disagreement by picking the run that suits a conclusion; the disagreement is
    the finding.
    A **seventh** row is separately named and deliberately not added, because
    card `C-GATEWAY-INDIRECT-CENSUS-2` saw constraint 16's signature and family
    on a test that is **none of the six**: `LOAD-SENSITIVE-ROW-UNNAMED`. Until
    that test is identified and measured in isolation, this list is known
    incomplete rather than assumed complete. `w36-corpus-contracts-3` §8.1,
    `c-gateway-indirect-census-2` §10.1.
    **Two ThreadSanitizer rows, measured and then repaired rather than listed
    (card `W45-TSAN-ROWS-LOAD-SENSITIVE`, wave 45).**
    `overlapping_writers_are_reported_under_every_schedule` and
    `removing_the_join_edge_is_reported_under_every_schedule`
    (`compiler/tests/task_route_tsan.rs`) each failed one integration gate in
    the runner's parallel phase and passed 5 of 5 alone. On the base binary the
    whole file measured red in **0 of 10** runs alone and **1 of 40, 2 of 64,
    13 of 96 and 42 of 128** at **8, 16, 24 and 32** concurrent copies. Both
    causes were in the harness, not in the detector. The forcing header's exit
    barrier took and released an instrumented spinlock on every poll (clang
    instruments atomics even in `no_sanitize("thread")` code), so a worker
    waiting a few milliseconds for a descheduled partner wrote enough
    ThreadSanitizer trace to evict its own body's accesses, and TSan v3 drops a
    race whose earlier stack it cannot restore. The join stand-in slept 50 ms
    instead of waiting, so a late worker produced a `heap-use-after-free`
    report, or none. After the repair: **0 of 10** alone and **0 of 40 / 64 /
    96 / 128** at the same loads. The binary is **not** in the runner's
    `EXCLUSIVE` table and these rows are not on this list: two held rows force
    the load on every run. No assertion was weakened. Retires
    `TSAN-JOIN-EDGE-ROW-LOAD-SENSITIVE`. `w45-tsan-rows-load-sensitive`.
    **The seventh row, identified and repaired, and the verifier-host
    fixtures' shared mechanisms (card `EXECUTION-QUOTA-LOAD-INDEPENDENCE`,
    wave 48).** `LOAD-SENSITIVE-ROW-UNNAMED` is
    `qcore-verifier-host::test::execution_quota`. Its four gate signatures are
    three races, none a load threshold. On the base binary, CPU load up to
    load1 ≈ 212, up to 12 concurrent copies and up to 7,228 same-UID tasks
    moved no row. Each race was measured on its own dimension:
    - **ETXTBSY.** A test thread's `fs::write` of `provider.sh` was
      duplicated into a sibling thread's forked, not-yet-`exec`ed child.
      Base: **6–10 of 20** runs under a close-delay shim. Fixed: **0 of 50**.
    - **`Io(Broken pipe)`.** The fixtures never read their request, and the
      host's first write could come after the fixture exited, either because
      the host was descheduled or because `spawn` waits until every
      concurrently forked sibling has `exec`ed. Base: **2 of 20 at a 10 ms
      host stall, 3 of 10 at 20 ms**. Fixed: **0 of 50** at 20 and at 50 ms.
    - **`quota_observed` None.** This was `EEXIST` on the per-execution cgroup
      name `qcore-exec-{pid}-{nanos}`: two executions of one process could
      read the same nanosecond. That is a **product defect** (a spurious
      fail-closed refusal) and is repaired in
      `process_quota::linux::session_name` with a per-process sequence number
      (Lean `session_keys_are_injective_in_the_sequence`). Base: **18 of 20**
      with `CLOCK_REALTIME` rounded to 1 ms. Fixed: **0 of 50**.
    - **Not an RLIMIT_NPROC row.** The binary has no `max_processes` row and
      its quota does not count the account.

    The same two fixture mechanisms were this list's confined-fixture write
    row and `memory_growth_beyond_the_declared_limit_is_bounded` (both
    `Io(Broken pipe)`), and they gave ETXTBSY in every spawning row of
    `enforced_process_limits`, the RLIMIT_NPROC row included. Base: **15 of
    20** under the shim, **1–2 of 20** at a 20 ms stall. Fixed: **0 of 150**
    each. The RLIMIT_NPROC row did not move at 3,000 extra same-UID processes
    (0 of 15). Every verifier-host fixture now reads its whole request first
    and is written by a separate process (`tests/support/process_fixture.rs`).
    These rows are therefore **repaired rather than listed**, as the TSan rows
    were. The wall-clock rows (`vp04_vp07_vp08`, `standalone_host`'s first
    arm) and `real_packages_preserve_unknown_and_reject_mutations` are
    untouched and stay. No assertion was weakened: the storm row no longer
    accepts `Io`, and the per-tree row now asserts `Completed`.
    `execution-quota-load-independence`.
    **One fixture writer for every crate (card
    `PROCESS-FIXTURES-FS-WRITE-OUTSIDE-HOST-CRATE`, wave 48).** A test or
    example that writes or copies an executable which it, or a process it
    starts, then runs writes it with `tests/support/process_fixture.rs`
    (`write_executable`, `write_executable_mode`, `copy_executable`, included
    with `#[path]`; the verifier-host copy re-exports it), never with
    `fs::write` or `fs::copy`. Data files (manifests, frames, source) stay
    `fs::write`. The census found 16 files with executable writes, the
    ledger's six and ten more, and routed all of them, including the binaries
    with one test or one spawning test, where no sibling forks today. On the
    base, under a close-delay shim plus a forked-child stall,
    `subcommand_plugins` deviated in **15 of 50** runs and
    `cycle20_verifier_plugins` in **1 of 50** (3 of 70 with a 20-run pilot),
    each with `ETXTBSY`. `standalone_host`, whose only spawning test writes
    and spawns on one thread, deviated in **0 of 50**. The tip gave **0 of
    50** for each. `real_packages_preserve_unknown_and_reject_mutations`'s
    own rewrites now use the writer too, but the row stays listed: its
    recorded `ExecutableFileBusy` was not reproduced. The production
    `package::build` write is tracked as
    `PLUGIN-PACKAGE-BUILD-WRITES-EXECUTABLE-IN-PROCESS`.
    `process-fixtures-fs-write`.
    **The host decides from consumption, not from write order (card
    `HOST-UNREAD-REQUEST-ACCEPTANCE-BY-SCHEDULING`, wave 48).** A provider
    that exits successfully without consuming its whole request is refused
    with `ProcessOutcome::RequestUnconsumed` (`request-unconsumed`), whatever
    came first. Before the card it was refused (`Io(Broken pipe)`) when it
    exited before the host's write, and **accepted** when the frame fit the
    pipe and the write landed first. Mechanism
    (`crates/qcore-verifier-host/src/request_consumption.rs`):
    - The host creates the request pipe and holds no read end while it
      writes, so a broken pipe still means that no reader is left. A broken
      pipe is now the same `RequestUnconsumed` refusal, also at the deadline
      for a live provider (where it was `Io`).
    - Once the whole frame is written, and before closing its end, the host
      takes a probe. On Linux this is a second read end opened through
      `/proc/self/fd/<write end>`. Off Linux it is a read end kept from the
      spawn, so the pipe cannot break there, and a live provider at its
      deadline is a `Timeout`.
    - Once the direct child has exited and both output pipes are at end of
      file, the host refuses if the frame was not wholly delivered or
      `FIONREAD` on the probe is non-zero. A probe that cannot be taken or
      read refuses as `Io`. An unsuccessful exit keeps its precedence
      (`Terminated`).
    - A provider that reads exactly its frame and exits before the host
      closes its end is accepted.

    The census found no real provider that ignores its request: every Rust
    runtime reads the frame and its end of file, and every non-reading
    fixture was already expected to fail. Measured on the probe binary
    `tests/request_consumption.rs` (five non-consuming and four consuming
    providers, frames under and over the pipe capacity), with no deviation
    under host stalls, `LD_PRELOAD` delays of the first request write and of
    its close, and both. The base binary deviated (the figures are in the
    report). Lean: `QCore.RequestConsumption`
    (`decide_is_a_function_of_consumption`,
    `a_non_consuming_provider_is_never_decoded`). A descendant that keeps
    the request pipe with both output pipes closed was outside that claim;
    the next paragraph closes it. `host-unread-request-acceptance`.
    **Only the request pipe's last holder decides (card
    `HOST-REQUEST-READ-BY-DETACHED-DESCENDANT`, wave 48).** A descendant that
    inherited the request pipe as its stdin, with both outputs closed, can
    still read after the direct child's exit and the outputs' end of file. The
    count at that moment was therefore not final, and on the base the same
    provider was accepted or refused by timing. The decision now waits,
    bounded by the declared deadline, until no process other than the host
    holds the pipe (`request_consumption::holders`):
    - While the host still writes, it holds no read end, so a write that does
      not break proves another reader; the decision waits for the write to
      finish or break.
    - Once the frame is written, the host opens a write end through
      `/proc/self/fd/<probe>`, closes the probe and polls the write end.
      `POLLERR` is set exactly when no reader is left; then no reader can
      appear and `FIONREAD` is final. Otherwise it reopens the probe and closes
      the write end, so a reader waiting for end of file still gets it, and
      checks again after its backoff.
    - A pipe still held at the deadline is refused as
      `ProcessOutcome::RequestHeldByDescendant`
      (`request-held-by-descendant`), distinct from `RequestUnconsumed` and
      from `WallClockTimeout`. A failed check refuses as `Io`.
    - Off Linux the other holders cannot be observed (`/dev/fd` is a `dup`).
      By the user's decision ("keep previous macOS"), the host decides there
      exactly as before the card, on the probe's count at the direct child's
      exit. Only this shape, a descendant that reads on its own schedule,
      stays timing-dependent there (never executed;
      `HOST-REQUEST-HOLDERS-OFF-LINUX-UNOBSERVABLE`).

    The host waits rather than refusing at once, because a refusal whenever
    another process holds the pipe still decides by timing for a descendant
    that reads its whole request and exits; the wait decides on the final
    consumption. The census found no provider or fixture with the shape (a
    non-interactive `sh` gives a background job `/dev/null` as stdin), so no
    existing Linux assertion moves and no wait is added where no other
    process holds the pipe. Measured on the probe binary's nine new cases
    under the earlier card's stalls and `LD_PRELOAD` delays: the base binary
    gave the same descendant both outcomes (194 deviating case-runs in 120
    runs), the fixed one none in 300 runs. Lean: `QCore.RequestConsumption`
    (`decide_is_a_function_of_consumption_and_holders`,
    `a_held_request_is_never_decoded`,
    `the_wait_decides_from_the_final_consumption`).
    `host-request-detached-descendant`.
<!-- preserved:constraint-16:end -->

<a id="constraint-17"></a>

<!-- preserved:constraint-17:start -->
17. **EXTERN-F1-STORE-REACH:** the abandonment gate refuses a second live
    acquisition and was deliberately not weakened, which is why
    `resource_stored` is checked but unreachable from a single source module.
    `extern-call-f1`, `remaining-extern-err-rel`.
<!-- preserved:constraint-17:end -->

<a id="constraint-18"></a>

<!-- preserved:constraint-18:start -->
18. Provider executables embed their manifest template and installed manifests
    are compared field-for-field against it, so **all five packages must be
    rebuilt and verified** after any change to the template or to a
    `qcore-schemas` major version; `providers/package.py --locked` is part of
    CI. `remaining-p01-f2`, `remaining-p01-f3`, `remaining-p03-f`.
    Provider-owned contracts live in each artifact crate's `contracts/` tree.
    Edit those sources, run `python3 providers/contracts.py sync` to update
    package inventories, and verify `python3 providers/contracts.py check`.
    Do not recreate top-level copies: `contract/corpus.json` indexes the
    canonical provider sources and the shared `providers/quirin-provider-*/contracts`
    modules (libc only in `quirin-provider-libc`). Preserve byte
    parity when reorganizing files, and
    update corpus declaration/clause pins deliberately for semantic changes.
    Package builds include these contracts and check their inventories.
<!-- preserved:constraint-18:end -->

Amended by card CONTRACT-LOCKS (user decision 1 of 2026-10-24): the corpus
declaration/clause pins are generated data, not literal tables. Each
load-path crate keeps a `contract-inventory.lock` beside its `Cargo.toml`
(per module: SHA-256, `extern fn` declarations in source order with their
elaborated clause counts and the per-file totals, `contract inherit` edges as
written, the premise manifest and binding pins; per crate: its three views).
`providers/contracts.py sync` writes it with the loaders' own parser and
`check` refuses one differing by a byte; `compiler/tests/qcore_contract_corpus.rs`
and `providers/tests/test_contract_load_path.py` read the locks, and the
corpus test recomputes each one from the tree. A contract change therefore
carries its regenerated lock in the same commit, reviewed as a diff of data;
never edit a lock by hand to make a test pass. It is not the
`contracts.lock`/`bounds-contracts.lock`/`outer-contracts.lock` pin locks,
which only `quirin-audit pin` writes. [Report](../reports/contract-locks/README.md).

Amended by card CONTRACT-SUPERSESSION (user decisions 9 and 9a, contract spec
[C06.6](../QCORE_CONTRACT_SPEC_r0_3.md#c066-contract-supersession-across-bundles)):
the pin locks are format version 4. Every entry carries the `pinned_at` stamp
`quirin-audit pin` wrote when it reviewed the entry's current text (a re-pin
carries a later stamp), and every lock names the Fluent Bit revision its lock
set is tied to; `pin --verify-contracts` (so `providers/contracts.py check`)
refuses a provider lock below version 4, untied or with an entry unstamped. A
function two selected modules declare is decided by those stamps instead of
refused; an undecided one is held as `contract-conflict` (`Unknown`). The live
lock set is the only one an audit reads; `providers/pin-lock-window.json`
retains at most three sets (the live one and up to two archived under
`providers/pin-lock-sets/<revision>/`), maintained only by `pin --window-*`:
a re-base adds a set and retires the oldest, and a fourth is refused. Never
edit a stamp, a tie or the window by hand. [Report](../reports/contract-supersession/README.md).

Amended by card CONTRACT-COUNT-PINS-FROM-LOCKS (gate-time requirement R3): the
inventory locks are schema 2. Each `extern fn` also records its
`resource_table` (`functions`, `pending_review`, `relational` or `retaining`:
where the resource-contract v1 loader puts it when its module loads alone;
`null` when that loader refuses the module), a function-map declaration also
its `resource_mode` (the loaded contract, serialized), and each module its
`other_declarations` (the non-`extern fn` names); `sync` refuses tables that
disagree with the crate's resource view. The function-map modes added after
base 25508ccaf are pinned by `resource_mode` alone; the literal mode table in
`contract_loader.rs` keeps every name it held then. The aggregate contract counts the
Rust and Python tests assert (corpus modules, resource-view modules and source
assumptions, v1 and rejected declarations, the loader's four tables) are sums
of these locks, read by one reader per language held to shared cases
(`tests/support/contract_locks.rs`, `scripts/contract_corpus.py`
`lock_totals`, `tests/support/contract_locks_cases.json`), never literals
outside `providers/`: a contract card's count movement is its lock diff.
[Report](../reports/contract-count-pins-from-locks/README.md).

<a id="constraint-19"></a>

<!-- preserved:constraint-19:start -->
19. **E03-B deliberately supersedes E03's `QP012`** unequal-premise-set refusal
    with the premise union (differing *values* still refuse). Do not restore
    the old refusal. `remaining-e03-b`, `remaining-e03-b-o`.
<!-- preserved:constraint-19:end -->

<a id="constraint-20"></a>

<!-- preserved:constraint-20:start -->
20. P04's in-process counter route was deliberately **not** removed (live
    production callers), and routes P09-A..F are retained with named gaps.
    Removal requires invariant 3 parity first. `remaining-p09`,
    `remaining-p09-routes`.

<!-- preserved:constraint-20:end -->

<a id="constraint-26"></a>

<!-- preserved:constraint-26:start -->
26. Process providers have process-group containment plus the opt-in
    target-qualified `linux-rlimit-v1` and `linux-confined-v1` profiles — this
    is limit and two-resource-class confinement, **not an OS sandbox** (no
    seccomp, no other namespace). The recorded default remains
    `unenforced-legacy`. `remaining-p02`, `remaining-p03`, `remaining-p03-f`.
    **The quota join is fail-closed (card `QUOTA-PROCS-DESCRIPTOR-DUP-FAIL-OPEN`,
    wave 48).** A declared `max_execution_processes` quota is either joined by
    the child or the execution is refused before spawning. No step between
    creating the per-execution cgroup and the child's `pre_exec` join may fall
    back to a spawn without the join:
    - A failed duplicate of the cgroup's `cgroup.procs` descriptor (`EMFILE`,
      `ENFILE`, `ENOMEM`) is `RefusedByProfile`, with no enforced limits, no
      observation and the cgroup and errno named. It was mapped to "no join",
      so the child ran outside its cgroup while the record named the quota.
    - The join's `write` succeeds only for the whole buffer; a short count is
      a failed join (`EIO`).
    - A record carries `quota_observed` only when `spawn` returned a child.
      The join is the first `pre_exec` step and a failed step fails the spawn,
      so an observation implies the join. A refusal after the cgroup exists,
      or a failed spawn, records none.

    `MechanismAvailability::quota_join_descriptor` makes the duplicate fail
    with `EMFILE` at its real call site. Like the other three overrides, it
    can only add a refusal. Lean: `QCore.ProcessQuotaPlan`
    (`an_observation_implies_the_join` and five more).
    `quota-procs-descriptor-dup-fail-open`.
<!-- preserved:constraint-26:end -->

<!-- generated:rules:end -->

## Amendment: included contract parents in packages (CONTRACT-INHERIT)

Constraint 18's package verification gains one rule. A packaged contract that
uses `#include "crate:module";` must pin every module it reaches — crate,
crate version, module path, SHA-256 — in its inventory's `includes` object and
install the exact texts under `includes/<crate>/<module>`;
`providers/package.py verify` refuses an unpinned directive, a changed or
missing parent, an unused pin and an unpinned `includes/` tree. Card
`CONTRACT-INHERIT-ADOPTION` builds the pins from the real crates' published
sets (below); since `CONTRACT-INHERIT-QCORE-PROVIDER-NAMING`, `qcore.librdkafka`
(2 pins) and `qcore.libevent` (1) include libc modules. [Report](../reports/contract-inherit/README.md).

**Load path (CONTRACT-INHERIT-ADOPTION).** Each `quirin-provider-*` crate
records `NAMESPACE` and `PUBLISHED` beside `CONTRACTS`; a contract-use edge is
a real Cargo dependency (`quirin-provider-fluent-bit` → `quirin-provider-libc`).
`python3 providers/contracts.py sync` also writes
`providers/contract-load-path.json` from Cargo metadata and those constants,
pinning every published module's SHA-256, and computes an inventory's
`includes` pins from it; `check` refuses a stale file, and `package.py build`
installs the pinned texts. After editing a published module (today
`libc/memory.qcore` and `libc/string.qcore`) or a crate's constants, run
`sync`: core consumers refuse a stale pin (`QI014`).
[Report](../reports/contract-inherit-adoption/README.md).

**Executable crates (CONTRACT-INHERIT-QCORE-PROVIDER-NAMING, the user's option
B).** The five `qcore-provider-*` crates are on the load path too: their
namespace is the package name without `qcore-provider-` (`librdkafka`,
`libevent`, …), recorded as `NAMESPACE`/`PUBLISHED` in `src/main.rs`, which
`contracts.py` reads; `qcore-provider-conformance` owns no contracts and is
excluded by name. `qcore-provider-{librdkafka,libevent}` → `quirin-provider-libc`
are real Cargo edges, pinned exactly in the conformance dependency test beside
the data-crate edge. Their packages carry include pins, `package.py build`
installs the parents and `verify` accepts them; the real-package mutation test
refuses a changed and a missing included parent.
[Report](../reports/contract-inherit-qcore-provider-naming/README.md).

**User contract crates (W45-USER-CONTRACT-PLUGINS).** A user's contract
crate lives outside this repository on a user load path that `extends`
`providers/contract-load-path.json`; the core reader validates both together.
The SDK `providers/quirin-provider-plugin` is a workspace member that owns no
contracts: `providers/contracts.py` lists it with the conformance harness as a
non-contract package, so it is not on the load path, and the conformance
dependency pin admits it with the shared transport as its only first-party
dependency. Its checker packages are built and verified by the SDK
(`quirin-plugin package`/`verify`), not by `providers/package.py`, which still
builds exactly the five first-party packages.
[Report](../reports/w45-user-contract-plugins/README.md).
**`package::build` holds no write descriptor on the executable (card
`PLUGIN-PACKAGE-BUILD-WRITES-EXECUTABLE-IN-PROCESS`).** While the caller
holds a write descriptor, any thread's fork copies it into a child that keeps
it until its own `exec`. An `execve` of the package then fails with
`ETXTBSY`. Neither close-on-exec nor a temporary file and `rename` closes
that window. So on Unix the SDK pipes the bytes it digested to a separate,
single-threaded `tee -- FILE`, which has exited before `build` continues.
`build` then:
- refuses unless the file is a regular file holding exactly those bytes;
- sets `0o755` by path;
- still writes the installed manifest last, so any writer failure is an
  `Err` and leaves nothing that verifies.

Off Unix the write is unchanged. Package bytes, digest, manifest and mode are
unchanged. **USER DECISION (2026-10-14): "Accept tee on PATH".** The new Unix
requirement is `tee` on `PATH`; it is documented on `build` and named in its
`Err`. `providers/package.py` and `quirin-audit prepare`'s copy are tracked
separately (`PROVIDER-PACKAGE-PY-WRITES-EXECUTABLE-IN-PROCESS`,
`PREPARE-COPY-KEEPS-EXECUTABLE-MODE-IN-PROCESS`).
[Report](../reports/plugin-package-build-write/README.md).

**Resource view (W44-PROVIDER-RESOURCE-VIEW).** Each load-path crate record
also carries `resource_view`: the owned modules the resource-contract loader
accepts, which `quirin-audit --provider NAME` selects for a whole crate and a
contracts-only run with no selection loads for every crate (`"selection":
"all-loaded"`). It is never a hand list: `providers/contracts.py sync`
computes it by loading each owned module through `quirin-audit`'s own
selection and loader (`cargo run -p qcore-audit --example
provider_resource_views`, then each view and all views together), and `check`
recomputes it and refuses a view that names a refused module, omits an
accepted one or is missing. The Rust load-path reader refuses an unsorted view
or one naming an unowned module (`QI014`). Packages are unchanged: the view
lives only in the load path. The union of the resource views must equal
`cycle19-resource-contracts.json`'s selected modules; current counts are in
[STATUS](../STATUS.md). [Report](../reports/w44-provider-resource-view/README.md).

**Bounds view (CHUNKIO-META-LEN-BOUNDS-CONTRACTS, gateway rule
[12r](gateway.md#constraint-12r)).** A bounds contract v1 module (C09.1) lives
in its library's provider crate like a resource module and is listed in the
crate record's generated `bounds_view`: the owned modules the bounds loader
accepts, computed by the same probe and written and checked by
`providers/contracts.py`. The two views are disjoint (the Rust reader refuses a
module in both), so no resource selection loads a bounds module. Its pins live
in the crate's `bounds-contracts.lock` (C06.2's lock format), written only by
`quirin-audit pin --bounds` after review at every pinned revision and verified
with the resource locks by `pin --verify-contracts` (so by `contracts.py
check`); a bounds declaration whose pin is not in force is held
(`contract-pin-mismatch`). Packages carry the module through the unchanged
inventory; no audit route reads contract modules from a package.
[Report](../reports/chunkio-meta-len-bounds-contracts/README.md).
Card CG16-BOUNDS-TRANSPORT-AND-CHUNKIO-NEIGHBOURS (gateway rules
[14i](gateway.md#constraint-14i), [14j](gateway.md#constraint-14j)): `pin
--bounds --track-tip REF` and `--untrack-tip` write tip entries in a bounds lock
as in a resource lock; a bounds module never uses `#include` or `inherit` (the
probe leaves one out of every view). `qcore-provider-chunkio`'s bounds view is
`chunkio/file_st_bounds.qv` and `chunkio/meta_len_bounds.qv` (five pinned
declarations). [Report](../reports/cg16-bounds-transport-and-chunkio-neighbours/README.md).

**Outer view (OUTER-CONTRACT, gateway rule [31](gateway.md#constraint-31)).**
An outer contract v1 module (C04.20) lives in the provider crate that owns its
loaders and is listed in the crate record's generated `outer_view`: every owned
module in neither other view that names `outer_contract_v1`, loaded together by
the outer loader (a citation needs its definition), so a crate whose outer
modules do not load together refuses the probe. The three views are disjoint.
Its pins live in the crate's `outer-contracts.lock`, written only by
`quirin-audit pin --outer` after review at every pinned revision and verified
with the other locks by `pin --verify-contracts`; a loader whose pin is not in
force covers nothing in the runtime projection and reaches every record.
`quirin-provider-fluent-bit`'s outer view is `fluent_bit/outer.qv` (two pinned
loaders), and `qcore-provider-librdkafka`'s is `librdkafka/outer.qv` (one pinned
loader). [Reports](../reports/outer-contract/README.md) and
[stage 2](../reports/rk-plugin-library-outer-envelope/README.md).
