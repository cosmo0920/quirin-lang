# O02 — One capacity boundary family, observed natively — 2026-09-08

A **native evidence** card, closed by a Rust implementation, six recorded
native sequences and an audited Lean model.

**Closing evidence.**

| Deliverable | Contents |
|---|---|
| [`crates/qcore-core/src/capacity_boundary.rs`](../../../crates/qcore-core/src/capacity_boundary.rs) | the grow relation **decided in Rust**, plus the recorded-trace validator: four separated quantities, four grow calls, the epoch ledger, eleven refusal codes |
| [`crates/qcore-core/tests/fixtures/file_capacity_observation.c`](../../../crates/qcore-core/tests/fixtures/file_capacity_observation.c) | the native fixture: six fixed sequences of **real, unsuppressed** syscalls in exclusive temporary directories |
| [`crates/qcore-core/tests/unit/capacity_boundary.rs`](../../../crates/qcore-core/tests/unit/capacity_boundary.rs) | **42 tests** — the relation, the recorded traces, the mutation matrix, and the compile-and-run of all six native scenarios under ASan/UBSan/LSan |
| [`proof/QCore/CapacityBoundary.lean`](../../../proof/QCore/CapacityBoundary.lean) | the matching Lean model, **30 theorems**, six of them decided over the whole checked domain |
| [`proof/AuditCapacityBoundary.lean`](../../../proof/AuditCapacityBoundary.lean) | the axiom audit — every theorem depends only on `propext`, and two on nothing at all |
| [`logs/native-traces.txt`](logs/native-traces.txt) | the 68 recorded trace records of one full run of the six scenarios |

**Nothing is wired into a production audit route.** `crates/qcore-core` is the
dependency-free L1 kernel and no analyzer, driver, contract or audit path calls
`grow_poststate` or `validate_capacity_trace`. No `.qcore` module was added, so
the contract corpus stays at 59; no diagnostic code was added or changed (the
refusal codes are module-local keys, not diagnostics); `qcore-schemas` is
untouched; no CT mapping moved; nothing is promoted.

## 1. What was selected, and why

> **The file-backed chunk *grow* boundary** — `ftruncate(2)`,
> `fallocate(2)`/`posix_fallocate(3)` and an extending `write`, and the
> kernel refusing the growth for want of capacity (the `ENOSPC`/`EDQUOT`/`EFBIG`
> class), on `x86_64` Linux.

The card offered capacity **or** metadata. Capacity was chosen, and metadata
deliberately left alone, for four reasons — the first three are properties of
the boundary, the fourth is what the Cycle 19 record already holds.

**1. A single success bit is provably not enough here.** A grow call can move
the file *length* without reserving any *backing*, so the bytes it "added" fault
at the first store rather than at the call. This is not a hypothesis: ChunkIO's
own resize path documents it in a comment on the allocating branch —
`ftruncate()` "will not fail and upon `memcpy()` over the mmap area it will
trigger a 'Bus Error' crashing the program" (`lib/chunkio/src/cio_file_unix.c`,
current upstream checkout `af1cdd759`; quoted as the *motivation* for the shape
of this family — nothing in this card imports, pins or claims anything about
that source).

**2. The mapping is not a witness.** A mapping may be longer than the file it
is over. The recorded scenario 5 acquires a live 8192-byte mapping over a
4096-byte file and it succeeds. A live mapping epoch therefore says nothing
about the capacity behind it — which is exactly what a forged "successful
mapping" would assert.

**3. The three calls disagree about how they report.** `posix_fallocate(3)`
returns the error number **and does not set `errno`** (`man 3 posix_fallocate`,
Linux man-pages 6.17, 2026-02-08: "Note that `errno` is not set"). Reading
`errno` there is a premise violation, not a detail; the recorded scenario 3
observes the return value `27` with `errno` untouched.

**4. Metadata could not satisfy acceptance item 1.** The card requires
acquisition epochs, state transitions, retries and teardown. The chmod-class
boundary already has two recorded Cycle 19 observations (a suppressed chmod
retaining descriptor custody, and a null-chmod success), and it has no epoch
structure of its own to record: it is a one-shot with no length, no backing and
no mapping. Capacity has four separable quantities that the Cycle 19
residency/transaction record already showed can disagree — logical length 4
with a mapped header length of 7 after a rollback.

The two families are **not** combined and neither is covered exhaustively.

## 2. The four separated quantities

| Quantity | Moved by | Never established by |
|---|---|---|
| file **length** `S` | every grow call | a mapping |
| reserved **backing** `B` | allocating calls only | `ftruncate` |
| **mapping** epoch `(generation, address, length)` | `mmap`/`mremap`/`munmap` | a length change |
| **logical** content `L` | the caller's own transfers | any grow call |

Two rules follow, and both are proved over the checked domain: **capacity is
never content** (no resizing call advances `L`), and **no arm authorises a
store it did not reserve**.

## 3. Why a real refusal, and not a suppressed wrapper

O03 recorded that a suppressed wrapper returns a fixed error *without reaching
the kernel*, so its post-state is the wrapper's, not the platform's. This card
takes the other route: every syscall in the fixture reaches the kernel, and the
capacity refusal is the kernel's own, obtained by lowering `RLIMIT_FSIZE` so
that the grow request exceeds the enforced maximum file size and the kernel
answers `EFBIG` (`man 2 ftruncate`: "`EFBIG` The argument `length` is larger
than the maximum file size").

`ENOSPC` and `EDQUOT` are **classified and not observed**: filling a real
device or a quota is not available to this harness without privileges. They sit
in the same capacity class as the observed `EFBIG` in the relation, and that
classification is a *documented* claim, not a measured one. The recorded
evidence covers `EFBIG` only.

`SIGXFSZ` is taken off its default disposition by the fixture, and this is
itself part of the boundary: left at the default, the kernel terminates the
process at the boundary and **no teardown runs at all**, so no post-state is
observable. The relation records that as a premise (`signal_handled`) and
poisons the transfer arm when it does not hold.

## 4. Acceptance item 1 — epochs, transitions, retries, teardown

Six fixed sequences, each in its own exclusive directory, each a fresh process
under AddressSanitizer, UndefinedBehaviorSanitizer and LeakSanitizer. Across
the six runs **16 endpoint epochs are acquired and 16 released** — 6 descriptor
epochs and 10 mapping epochs, of which 3 are released by their replacement in a
`mremap` and 7 by an explicit `munmap`. Every run ends with its fixture file
removed and its directory empty; every run exits 0 with no sanitizer finding.

| # | Sequence | Observed |
|---|---|---|
| 0 | map, `ftruncate` 4096 → 8192, remap, teardown | The grant moves the length to 8192 and leaves `st_blocks` at 8: the file is twice as long and **not one byte more is backed**. The mapping is remapped to a new epoch at a new address; the content length stays at 4096. |
| 1 | map, `posix_fallocate` to 8192, remap, store at 8191, teardown | The allocating grant moves `st_blocks` from 8 to 16, so the added range is writable authority; the content length reaches 8192. |
| 2 | map, limit → 4096, grow, **repeat**, limit → 32768, grow, remap, teardown | Both attempts return `-1`/`EFBIG` with the length unchanged at 4096; the mapping epoch acquired before them survives both untouched. The identical request is granted **only** after the recorded limit change, and only then is the mapping replaced. |
| 3 | map, limit → 4096, `posix_fallocate`, limit restored, teardown | The call returns `27` as its **return value** with `errno` untouched. Length and blocks are unchanged, and the mapping epoch is released at its own length of 4096. |
| 4 | map, limit → 4100, `pwrite` 8 bytes at 4096, `pwrite` 1 byte at 4100, teardown | The first transfer moves **4 of the 8** requested bytes (`ret = 4`, length 4096 → 4100, blocks 8 → 16); the content advances by exactly 4. The second returns `-1`/`EFBIG` with the length unchanged. |
| 5 | map 4096, unmap, map **8192** over the 4096-byte file, read byte 0, teardown | The longer mapping succeeds and is live. It witnesses 4096 usable bytes and records 4096 bytes past the end of the file, which the fixture does not touch. |

Retries are recorded, not prescribed: the relation's answer after a
capacity-class refusal is `RetryAfterCapacityChange`, and the validator refuses
a trace in which the identical request is granted with no recorded capacity
change (scenario 2 with its limit change deleted). After a partial transfer the
answer is `RetryFromObservedLength` — the remainder is re-derived from the
observed length, never re-issued whole.

## 5. Acceptance item 2 — the mutation matrix

The validator refuses **27 rows over 19 Rust tests**: 18 mutations of the two
recorded traces and 9 malformed inputs. The three families the card names:

| Family | Mutations refused |
|---|---|
| **lost residuals** | the release deleted; the residual count misdeclared; a *failed* release counted as a release (it retains the epoch, and the declaration has to say so) |
| **forged successful mappings** | a mapping recording a file length the ledger never observed; a remap onto a length that a deleted grant never produced; a release at a length other than the acquisition length (which would leave part of the mapping in place); content claimed inside the hole a length-only grant created |
| **confused generations** | a released generation acquired again; a generation that goes backwards; a remap from an epoch that is not live; a release of an epoch that is not live |

Four further families are refused: forged grants (a refusal that grew the file;
a repeat granted with no capacity change), forged reservations (a refusal
recorded as having reserved the whole request), unobserved outcomes, and
partial transfers counted whole.

The Lean model carries the same matrix independently: `every_mutation_is_refused`
decides 14 mutated traces, `the_mutation_matrix_is_not_vacuous` pins that they
are 14 distinct traces and none of them is the accepted one, and
`the_recorded_trace_is_accepted` supplies the non-vacuity for every refusal.

## 6. Acceptance item 3 — exactly what is claimed

Result boundary: artifact = the recorded native executable built from
`file_capacity_observation.c` and the relation that validated its traces;
property = observed length, backing, mapping-epoch and content transitions
across a grow boundary, its capacity refusal, its retries and its teardown;
semantic profile = `x86_64` Linux 7.0.0-30-generic, Ubuntu clang 21.1.8,
`-std=c11 -O0 -g -fsanitize=address,undefined` with `detect_leaks=1`, a 4096-byte
page, the system temporary directory on **tmpfs**; scope = six exclusive
one-file sequences; bounds = one 4096-byte seed file grown to at most 8192
bytes, one descriptor epoch and at most two mapping epochs per process, at most
8 transferred bytes, `RLIMIT_FSIZE` soft limits of 4096, 4100 and 32768, no
concurrency, no injected or suppressed syscall; assumptions = trusted compiler
and fixture, exclusive fixture directories, the cited man-pages as the
documentation of record; evidence version = the trace vocabulary in
`capacity_boundary.rs`. `ENOSPC` and `EDQUOT` were **not** observed. No static
Safe/Unsafe certificate, no durability claim and no filesystem model follows,
and nothing about other sizes, other filesystems, other targets or other
callers is claimed.

## 7. Evidence, and what it costs

The pinned Cycle 19 native route was **not** available here: it requires the
clean Fluent Bit checkout at `66910c10a4d7eafa810b84229cf2dcf7b0f26f97` and the
developer checkout has moved to `af1cdd759`, so `cycle19_native_observations`
refuses to build. The fixture is therefore self-contained and makes **no claim
about any pinned Cycle 19 unit** — it observes the syscall boundary that
ChunkIO's resize path uses, not ChunkIO. Re-running this family against the
pinned ChunkIO wrapper, once that checkout is available again, is follow-on
**O02-U1**.

The 42 Rust tests split as: 20 relation tests (5 of them decided over the whole
`target × premise × observation × outcome` domain), 19 validator and mutation
tests, 2 recorded-trace acceptance tests, and 1 native test that compiles the
fixture once and runs all six scenarios.

The 30 Lean theorems split as: 3 domain-infrastructure, 6 standing invariants
over the checked domain, 12 named arms, 2 mapping-witness, and 7 ledger and
mutation-matrix theorems.

## 8. Gates

Measured on this host, before and after.

| Gate | Before | After |
|---|---|---|
| `cargo test --workspace` | 1387 passed, 0 failed, 1 ignored | 1429 passed, 0 failed, 1 ignored |
| `cargo fmt --all -- --check` | clean | clean |
| `cargo clippy --all-targets -D warnings` | clean | clean |
| `reevaluation_probe` | exit 0 | exit 0 |
| `reevaluation_followup` | exit 0 | exit 0 |
| `python3 -B -m unittest discover -s scripts/tests` | 89 passed | 89 passed |
| `python3 scripts/check_lean_audits.py` | 32 audits | 33 audits |

No Python changed, so the Python suite is unchanged; the 89 here is this
checkout's count, which is above the 81 the third delegated increment recorded
because the Windows and `std.concurrency` commits landed since.

`providers/` was not touched and its suite was not re-run.

## 9. Open, and deliberately not done

* **O02-U1** — the same family against the pinned ChunkIO `cio_file_resize`
  wrapper, once the pinned checkout is restored. Nothing here is evidence about
  that wrapper.
* **O02-U2** — a real `ENOSPC`/`EDQUOT` observation. Both are classified from
  documentation only; a small dedicated filesystem or a quota would be needed,
  and the harness has no privileges for either.
* **O02-U3** — the metadata family. Deliberately untouched: this card commits
  to one family, and combining them is explicitly out of scope.
* **O02-U4** — no production route consumes the relation. Wiring it into the C
  gateway needs the same relational branch analysis O03 recorded as its U3.
* The `fallocate(2)` arm of the relation is decided but not natively observed:
  the fixture uses `posix_fallocate(3)`, whose reporting divergence is the point
  of scenario 3.
* Multiple simultaneously live mapping epochs are outside the Lean ledger's
  reduction (the Rust ledger holds any number); the recorded scenarios keep at
  most one live at a time.

## 10. For the integrator

Proposed ledger sentences:

> Capacity grow boundary (O02): one boundary family from the Cycle 19 failure
> matrix — growing a file-backed chunk and the kernel refusing it for want of
> capacity — is specified and decided in `qcore-core::capacity_boundary`
> (42 tests) with a matching Lean model (`QCore.CapacityBoundary`, 30
> theorems, `AuditCapacityBoundary.lean`). Six real, unsuppressed Linux
> sequences acquire and release 16 endpoint epochs under ASan/UBSan/LSan: a
> length-only grant doubles the file while backing nothing more; an allocating
> grant backs what it grew; two identical requests are refused with `EFBIG`
> and the length unchanged, and granted only after a recorded capacity change;
> `posix_fallocate` returns its error and leaves `errno` untouched; a transfer
> moves 4 of 8 requested bytes; and a mapping twice the length of the file is
> live and witnesses nothing past its end. Twenty-seven Rust refusal rows and
> fourteen Lean mutation rows reject lost residuals, forged successful
> mappings, confused generations and forged grants. `ENOSPC`/`EDQUOT` are
> classified, not observed; no production route consumes the relation; the
> contract corpus, CT coverage and the pinned 185-unit artifacts are
> untouched, and nothing is promoted. Evidence:
> `docs/reports/remaining-o02/README.md`.

Decisions the integrator may want to revisit:

1. **No `.qcore` module was added**, to leave the corpus pin at 59 modules. The
   Rust validator is the policy. If the wave wants a declarative runtime policy
   for this family, adding one is a deliberate pin update.
2. **No diagnostic code was allocated.** The `Q610`–`Q677` bands are fully
   assigned and this is not a source checker; refusals carry module-local keys
   (`capacity.lost-residual`, …).
3. **The native test is Linux-gated** (`#[cfg(target_os = "linux")]`): it
   depends on `RLIMIT_FSIZE`, `mremap(2)` and `st_blocks`. On macOS and Windows
   the 41 non-native tests still run.
