# Providers rules

Current reference, routed by [AGENTS.md](../../AGENTS.md). Rule numbers are stable.
Paths in backticks are repository-relative. Read every applicable rule,
including cross-topic rules, before editing; the index is not a substitute.

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
<!-- preserved:constraint-26:end -->

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
