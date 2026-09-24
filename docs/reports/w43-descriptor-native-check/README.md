# W43-DESCRIPTOR-NATIVE-CHECK: descriptor lifecycles checked on native executions

Card W43-DESCRIPTOR-NATIVE-CHECK, 2026-09-25, on base `f8b41209`, with main
`0a07cb57` merged at `662af52f` (the measured tip). **Phase A is
complete. Phase B is waiting** for W43-DESCRIPTOR-RESOURCE-TRACKING to land its
libc descriptor contracts on main.

The tracking card's descriptor contracts are trusted assumptions. This card
builds the evidence that validates them against real executions, the way
`fluent_bit/memory.qcore` was checked against 145 executions of `flb_mem.h`
([cycle19-native-contracts](../cycle19-native-contracts/README.md)). It has
five parts:

* a native observer that interposes on every descriptor-creating,
  descriptor-releasing and descriptor-using libc entry point;
* a record schema;
* a Rust replay that evaluates a QCore lifecycle policy through the shared
  predicate interpreter;
* a Lean model of the observation-level properties;
* 77 pinned runs of the pinned Fluent Bit build, Monkey sockets and libc,
  including 36 deterministic failure injections.

**Native observations are evidence separate from static proof and from
mapping.** Nothing here is a static verdict. No `Unknown` moves, and the
corpus arbiter is byte-identical ([§9](#9-corpus-arbiter)).

## Headline

* **Harness.** [`harness/descriptor_observer.c`](harness/descriptor_observer.c)
  uses `-Wl,--wrap` link wrappers, the method of the cycle-19 allocator and
  ChunkIO observation hooks. It wraps 112 entry points.
  * **Kinds:** fd, `FILE *`, `DIR *`, dl handle, addrinfo list, mapping,
    `popen` stream and ifaddrs list.
  * **Recorded per event:** call, arguments, result, errno, and the injected
    flag. Also each operand's kind, identity and **generation** (a reused
    number or address is a new generation), and the call site, which a short
    call stack accompanies for constructors, failures and releases. The
    `fdopen` / `fopen` / `fileno` / `opendir` / `dirfd` relations are recorded
    too.
  * **Failure injection** is deterministic (`call:n:errno`) and can be
    restricted to one caller function's address range. Every one of the 36
    injections landed in its target function.
* **Build identity.** The targets are rebuilt from a byte-identical copy of the
  pinned checkout (`66910c10`, 12,127 files). They use the pinned configure
  command, and all 1,415 compile-database entries match the pinned database.
  Seven configure-generated files differ only in their checkout path and
  enclosing git hash; the build records them. The pinned checkout is only
  read.
* **Runs: 77, 76 accepted as evidence, 6,730 records.**
  * 35 upstream `tests/internal` runs: pipe, file, network, fstore and utils,
    one process per test case.
  * 16 runs of the actual `fluent-bit` binary: dummy, tcp input with
    filesystem storage, the HTTP server, and dynamic plugin loading.
  * 6 Monkey `mk_socket.c` driver runs.
  * 20 libc-probe runs.

  The one rejected run aborted in a heap **double free**
  ([§4.3](#43-storage-scan-failure-double-free-f3)). It has no exit record,
  so it is not descriptor evidence.
* **Properties.** Over every accepted run:
  * **Hold:** no use after release (4,890 checks), release matches kind
    (1,217), leak set well defined (76), generation discipline (1,680),
    counted dl references (20), sentinel validity (1,289), relations (168) and
    mapping extents (63).
  * **Violated: each generation is released at most once.** 49 violations in
    11 runs, from two upstream double closes
    ([§4.1](#41-engine-channel-double-close-f1),
    [§4.2](#42-scheduler-timer-double-close-f2)). Each second close was
    answered with `EBADF`.
  * **Violated: operand identity valid.** 37 runs call `close(-1)`
    ([§4.1](#41-engine-channel-double-close-f1)).
* **Lean.** [`QCore.DescriptorNativeObservation`](../../../proof/QCore/DescriptorNativeObservation.lean)
  is audited by `AuditDescriptorNativeObservation.lean`: 25 theorems, no
  placeholders, no project axioms. It proves the four required properties
  over accepted traces: at-most-once release, no use after release, release
  matching the acquired kind, and a well-defined leak set with an origin for
  every leaked generation.
* **Follow-ups.** This card registers 8 and retires none (see the list in [§10](#10-follow-ups)).

## 1. Method and identity

| Piece | Where |
|---|---|
| Observer (C) | [`harness/descriptor_observer.c`](harness/descriptor_observer.c) |
| Monkey socket driver | [`harness/monkey_socket_driver.c`](harness/monkey_socket_driver.c) |
| libc probe | [`harness/libc_descriptor_probe.c`](harness/libc_descriptor_probe.c) |
| Fixture plugin DSO | [`harness/flb_out_qdesc_plugin.c`](harness/flb_out_qdesc_plugin.c) |
| Build / relink / interposition audit | [`scripts/build_targets.py`](scripts/build_targets.py), [`build.json`](build.json) |
| Compile-database identity | [`scripts/compare_compile_db.py`](scripts/compare_compile_db.py) |
| Scenarios and runner | [`scripts/run_scenarios.py`](scripts/run_scenarios.py) |
| Aggregation | [`scripts/aggregate.py`](scripts/aggregate.py), [`aggregate.json`](aggregate.json) |
| Pinned records | [`records/observations.tar.gz`](records/observations.tar.gz) (SHA-256 `0f8f9397…`), [`records/manifest.json`](records/manifest.json) |
| Replay re-check | [`scripts/verify_records.py`](scripts/verify_records.py) |
| Policy (QCore) | [`library/descriptor_native_observation.qcore`](../../../providers/quirin-provider-shared/contracts/library/descriptor_native_observation.qcore) |
| Replay (Rust) | [`qcore_contract::descriptor_observation`](../../../crates/qcore-contract/src/descriptor_observation.rs), example `check_descriptor_observations` |
| Model (Lean) | [`QCore.DescriptorNativeObservation`](../../../proof/QCore/DescriptorNativeObservation.lean) |

**Interposition.**
* **Relinking.** Each target is relinked statically from its CMake objects and
  the `fluent-bit-bin` static archives, so `--wrap` reaches every reference in
  Fluent Bit, ChunkIO, Monkey, c-ares, librdkafka and the other bundled
  archives.
* **Audit.** After linking, `build_targets.py` lists each binary's imported
  libc symbols and intersects them with a descriptor universe of 68 names.
  Every imported member is wrapped, so the audit reports no gaps.
  * `fork` and descriptor-affecting raw `syscall` numbers are recorded as
    out-of-model events. None occurred.
  * The observer stops in a forked child.
* **Not interposed:** references inside shared libraries (libc's own internal
  calls, libssl, libcurl, libsystemd, libyaml and libsasl2).

**Ordering under threads.**
* **Releases** (`close`, `fclose`, `closedir`, `dlclose`, `munmap`,
  `dup2`/`dup3`) run their real call while holding the observer lock, so a
  freed number cannot be re-acquired and recorded ahead of its release.
* **Acquisitions** are recorded after the call.
* **Uses** are recorded before the call. A use recorded after a release
  therefore started after it, and a use recorded before a release but executed
  after it is missed. The ordering can produce false negatives, never false
  positives.
* Signals are blocked while the lock is held.

## 2. Record schema (version 1)

One JSON line per event is written with the real `write(2)`. The fields are
`schema_version`, `sequence` (contiguous), `pid`, `thread`, `call`, `site`
(`{module, offset}`), `args`, `result`, `errno` and `injected`, plus optional
`path`, `stack` (sites) and `injections` (on `start` only), and `operands`.

Each operand has a `role`, a `kind`, an `id`, a `generation` and, for
mappings, an `extent`. The roles are:

* `inherited`: descriptors open at start, and stdio streams owning 0–2;
* `acquired`;
* `derived`: the descriptor a new `FILE`/`DIR` owns;
* `adopted`: `fdopen`/`fdopendir`;
* `used`;
* `released`;
* `released_child`: a stream's descriptor released with it;
* `replaced`: `dup2` onto a live number;
* `related`: the `fileno`/`dirfd` result;
* `live`: on `exit`.

The Rust replay deserializes with `deny_unknown_fields` and rejects a run
(it is not evidence) in any of these cases:

* an unknown call;
* an operand shape the call alphabet does not allow;
* a noncontiguous sequence;
* a foreign pid;
* a record after `exit`;
* a missing `exit`;
* an injected record that reports success;
* an observer generation stamp that differs from the replayed generation.

A false policy clause is recorded as a **violation** of its property, and the
replay continues with a stated recovery.

## 3. Properties and results

The policy has one parameterless rule per property. Rust derives the facts and
the shared `check_contract_facts` interpreter evaluates the clause.
`policy_text_drives_the_result` shows that weakening a clause removes its
violation and that an unsupplied fact fails closed.

| Property | Checks | Violations | Runs violated / checked |
|---|---:|---:|---:|
| `release_at_most_once` | 1,266 | **49** | 11 / 51 |
| `no_use_after_release` | 4,890 | 0 | 0 / 75 |
| `release_matches_kind` (incl. custody: a stream's descriptor released only through the stream) | 1,217 | 0 | 0 / 51 |
| `leak_set_well_defined` (observer live set = replayed; one generation per identity; every live generation has an origin) | 76 | 0 | 0 / 76 |
| `generation_fresh` (acquire only a non-live identity; generation + 1; a kernel `EBADF` on an observed-live close also fails here) | 1,680 | 0 | 0 / 76 |
| `counted_reference` (dl handles) | 20 | 0 | 0 / 20 |
| `sentinel_validity` (failure returns the call's sentinel, acquires nothing, errno where required; success returns a valid identity and acquires what the call makes) | 1,289 | 0 | 0 / 57 |
| `relation_holds` (`fileno`/`dirfd` return the owner's descriptor; `fdopen` adopts a live caller-owned descriptor; `dup` distinct; `dup2` returns its target) | 168 | 0 | 0 / 21 |
| `operand_identity_valid` (no release/use receives `-1`/`NULL`) | 6,193 | **37** | 37 / 76 |
| `mapping_extent` (`munmap`/`mremap` name the recorded base and length) | 63 | 0 | 0 / 26 |

`untracked_uses` and `untracked_releases`, which count identities never seen
acquired, are **0** in every run. The observer and the kernel never disagreed
about liveness, and no acquisition hit a number or address the replay held
live.

## 4. Findings

Each finding below was observed in pinned runs and its mechanism was then read
in the pinned source. They are native-observation results with the scope
stated, not static verdicts. None is presented as an upstream defect beyond
what its evidence establishes.

### 4.1 Engine channel double close (F1)

**Observed.** Every run that reaches full engine teardown shows the same
sequence (11 runs). `flb_config_exit` (`src/flb_config.c:607`) closes
descriptor 14 through `config->ch_event.fd`, and at `:617` closes the same
generation again through `config->ch_manager[0]`, which returns `EBADF`.

**Mechanism.** `flb_engine.c:1201` creates the manager channel with
`mk_event_channel_create(evl, &ch_manager[0], &ch_manager[1], &ch_event)`,
which stores `ch_manager[0]` in `ch_event.fd`. The two names are one
descriptor generation.

**Sentinel close.** When the engine never started, `ch_event.fd` is `-1`, and
the guard `if (config->ch_event.fd)` tests nonzero rather than validity. The
same statement then calls `close(-1)`. That is the `operand_identity_valid`
violation in 37 runs: every upstream test and early-exit run that runs
`flb_config_exit`.

**Consequence.** Only `EBADF` was observed. A thread opening a descriptor
between the two closes would lose it to the second close; that is inferred
from the code and **not observed**.

### 4.2 Scheduler timer double close (F2)

**Observed.** 38 second closes in 10 runs. `flb_sched_destroy`
(`src/flb_scheduler.c:843`) calls `mk_event_timeout_destroy(&timer->event)`
for each timer, which closes the timer descriptor. It then calls
`flb_sched_timer_destroy(timer)`. That calls `flb_sched_timer_cb_disable`, and
because `timer->timer_fd` was never reset, it closes the same generation again
(`mk_event_epoll.c:350`). The recorded stacks show both callers, on output
worker threads and on the main thread.

**Consequence.** Only `EBADF` was observed. The worker-thread instance runs
while other threads are live, so the reuse hazard of §4.1 applies. It is
inferred, not observed.

### 4.3 Storage-scan failure double free (F3)

This finding is out of the descriptor scope; the harness surfaced it.

**Observed.** Injecting `opendir` → `EACCES` into the first storage scan makes
the engine report "storage creation failed". The process then aborts with
glibc's `free(): double free detected`.

**Reproduced without injection.** The same abort occurs when the configured
`storage.path` exists without read permission (mode `0300`). The backtrace is
`cio_destroy` ← `flb_storage_destroy` (`src/flb_storage.c:1000`) ←
`flb_engine_shutdown`.

**Mechanism.** `flb_storage_create`'s `cio_load` failure branch
(`src/flb_storage.c:727–732`) calls `cio_destroy(ctx->cio)` without clearing
`ctx->cio`, unlike the storage-backlog branch below it. Shutdown then destroys
the context again.

**Scope.** This is a heap-lifetime finding with a validated natural trigger.
The run is kept, rejected, as `engine.tcp_filesystem_storage.inject-opendir`.

### 4.4 Monkey bind-failure socket leak (F4)

**Observed.** The driver calls the actual `mk_socket_server` on a port that is
already listening. `bind` fails with `EADDRINUSE`, the function returns `-1`,
and its socket (`mk_socket_create`, `mk_socket.c:125`) is live at exit.

**Mechanism.** At `mk_socket.c:336–340` the failure path frees the address
list and returns without `close(socket_fd)`.

**Reachability.** No pinned Fluent Bit configuration reaches Monkey's server
sockets: its HTTP server now uses `flb_downstream`. This is a Monkey library
error path, not a Fluent Bit one.

### 4.5 Proxy handle never released (F5)

This one is an observation, not a defect claim.

**Observed.** `-e qdesc-invalid.so` (no `flb-` prefix) routes to
`flb_plugin_proxy_create`. It `dlopen`s the file, then ignores
`flb_plugin_proxy_register`'s failure. The handle stays on `config->proxies`,
which `flb_config_exit` never walks, so it is live at exit.

**Harm.** In the CLI the process ends there. For an embedding that calls
`flb_destroy` and keeps running, the handle and the proxy allocations would
outlive the context. That is inferred and not measured.

## 5. Live at exit

Only non-inherited generations are listed; inherited stdio and descriptors 0–2
are always live.

| Origin | Runs | Reading |
|---|---:|---|
| `mmap` in `backtrace_alloc` ← `flb_stacktrace_init` | 15 | libbacktrace's process-lifetime arena; intentional, not a defect |
| `epoll_create1` and two `pipe`s in `flb_create` | 2 | `plugin_missing` and the injected-`dlopen` run exit from `flb_main_run` right after the failure without `flb_destroy`; process exit follows; not a defect |
| `dlopen` in `flb_plugin_proxy_create` | 1 | §4.5 |
| `socket` in `mk_socket_create` ← `mk_socket_server` | 1 | §4.4 |

Every other run, including every graceful engine shutdown, releases every
descriptor, stream, directory, handle, list and mapping it acquired.

## 6. Coverage

Executions per callee, summed over all 77 runs. "Natural failures" counts
records carrying a nonzero errno that were not injected.

| Callee | Executions | Natural failures | Injected | Reached from |
|---|---:|---:|---:|---|
| `open` | 169 | 23 | 3 | Fluent Bit, tests, probe |
| `socket` | 85 | 20 | 5 | Fluent Bit, tests, Monkey, probe |
| `accept4` | 57 | 25 | 4 | Fluent Bit, tests, Monkey, probe |
| `pipe` | 169 | 0 | 2 | Fluent Bit, tests, probe |
| `epoll_create1` | 91 | 20 | 2 | Fluent Bit, tests, probe |
| `timerfd_create` | 97 | 0 | 0 | Fluent Bit |
| `fopen` / `fclose` | 57 / 71 | 22 | 1 | Fluent Bit, tests, probe |
| `opendir` / `closedir` | 50 / 47 | 20 | 2 | Fluent Bit (ChunkIO scan), tests, probe |
| `dlopen` / `dlsym` / `dlclose` | 66 / 24 / 42 | — | 2 | Fluent Bit (`flb_plugin_load`, proxy), probe |
| `getaddrinfo` / `freeaddrinfo` | 64 / 41 | — | 3 | Fluent Bit, tests, Monkey, probe |
| `mmap` / `munmap` | 83 / 44 | 20 | 3 | Fluent Bit (ChunkIO, libbacktrace), tests, probe |
| `close` | 904 | — | — | all |
| `dup`, `dup2`, `fcntl(F_DUPFD_CLOEXEC)`, `fdopen`, `fdopendir`, `dirfd`, `fileno`, `openat`, `pipe2`, `socketpair`, `eventfd`, `tmpfile`, `mremap` | 19–40 each | per probe | 1 each where a constructor | **probe only**; the observed Fluent Bit paths do not call them |
| `accept`, `dup3`, `memfd_create`, `mkstemp`, `popen`/`pclose`, `getifaddrs`/`freeifaddrs` | 0 | | | **not observed** |

## 7. Boundaries

* **Observed, not proved.** These are recorded executions only. They say
  nothing about unexercised paths, other configurations, other hosts (Linux
  x86_64, glibc, Clang 21.1.8), or concurrency schedules other than those
  recorded.
* **The records are trusted observer output.** They are not authenticated.
* **Not interposed:** shared-library internals, `poll`/`select` descriptor
  arrays, and raw syscalls outside the recorded descriptor numbers.
* **Model and replay are linked by review.** The Lean model proves properties
  of the model. Its correspondence to the Rust replay is maintained by review
  ([trust boundary 28](../../agents/evidence.md#constraint-28)).
* **Generation reuse.** A reused number or address is a new generation, and
  neither the model nor the replay can tell which generation a caller meant.
  This is the same shape as `CT50-PROOF-ADDRESS-REUSE-UNPROVED`, which stays
  open.
* **Injection is part of the harness.** Injected failures return the sentinel
  without calling libc. For `getaddrinfo`, `EAI_SYSTEM` sets errno to `EIO`
  and every other EAI code sets none. An injected failure is labelled in its
  record and counted separately.
* **Scratch paths.** The upstream fstore test writes its fixed `/tmp/flb-fstore`
  path; every other scenario keeps its scratch inside the worktree.

## 8. Phase B: waiting

Main at `f8b41209` does not contain W43-DESCRIPTOR-RESOURCE-TRACKING, so no
libc descriptor contract has landed to bind to.

The records already carry what Phase B needs: every execution of each declared
callee, with its result, errno, sentinel, kind and relation facts. They are
pinned, and `verify_records.py` replays them under any policy.

When the integrator resumes the card, it will do three things:

* merge main;
* derive each landed contract's clauses as facts over that callee's records;
* report every contradiction as a finding.

A contradiction will not be resolved by weakening the observation.
`W43-DESC-NATIVE-PHASE-B` tracks it.

## 9. Corpus arbiter

This card changes no gateway, no contract selected for the corpus, and no
verdict-bearing code path. It ran the arbiter once at the measured tip
`662af52f`, with `docs/reports/w43-indirect-access-general/scripts/run_audit.py`
(main's current runner). The record is [`arbiter.json`](arbiter.json).

* **Against the main baseline.** Main moved the corpus baseline to
  `docs/reports/w43-indirect-access-general/artifacts/` (root alias
  `agent-a8d24f7f717a987c1`). `baseline_identity.py` reports the
  verdict-bearing diagnostics **byte-identical** (`4cfc7fe1…`), and
  `chunk-contracts` and `monkey-source` are also byte-identical.
  `contracts` and `source-effects` differ from that frozen baseline. Every
  differing leaf lies in modules that main's CONTRACT-INHERIT-QCORE-PROVIDER-NAMING
  changed, or in identity digests that follow from them, and none mentions
  this card's module. The integrator recorded the same two differences on
  main's combined tree.
* **Against a same-source BEFORE run.** Main `0a07cb57` was built in its own
  temporary checkout and audited the same way. Every one of the five artifacts
  is identical to the AFTER run once the checkout root is rewritten: four are
  byte-identical, and `contracts` differs only in its root spelling. The
  first BEFORE attempt failed because that checkout lacked the libevent
  submodule (a pin mismatch). It is kept as `measure/arbiter-before-main`,
  discarded as evidence, and was rerun after the submodule was initialized.
  The temporary checkout has been removed.

## 10. Follow-ups

**Registered (8)** in `named_followups`, each with its discharge condition:

* `W43-DESC-NATIVE-PHASE-B`
* `W43-DESC-NATIVE-COVERAGE-GAPS`
* `W43-DESC-FLB-CONFIG-CHANNEL-DOUBLE-CLOSE`
* `W43-DESC-FLB-SCHED-TIMER-DOUBLE-CLOSE`
* `W43-DESC-FLB-STORAGE-SCAN-DOUBLE-FREE`
* `W43-DESC-MONKEY-BIND-FAILURE-LEAK`
* `W43-DESC-FLB-PROXY-HANDLE-UNRELEASED`
* `W43-DESC-OBSERVER-ORDERING-BOUNDARY`

**Retired: none.** `W43-LIBC-DESCRIPTOR-LIFECYCLE` belongs to the tracking
card. `CT50-PROOF-ADDRESS-REUSE-UNPROVED` is unchanged.

## 11. Validation and reproduction

The gate was measured at `662af52f` with `scripts/run_gate.py` and
`CARGO_INCREMENTAL=0`. Every exit was captured to a file. The records are
[`gate.json`](gate.json) and [`gate-facts.json`](gate-facts.json), and the
logs are in worktree-private `measure/gate-4` and `measure/gate-7`; `measure/gate-8` reran the Python suite and docs index after the final matrix regeneration (330 tests, exit 0).

| Step | Result |
|---|---|
| `git submodule update --init --recursive` | exit 0 |
| `cargo test --workspace --no-fail-fast` | **5,270 passed / 0 failed / 11 ignored** over 282 summaries |
| `cargo fmt --all -- --check`; `cargo clippy --workspace --all-targets -- -D warnings` | exit 0 / exit 0 |
| `reevaluation_probe`, `reevaluation_followup` | exit 0 / exit 0 |
| `python3 scripts/check_lean_audits.py` | **173 Lean audits** passed (one added: `AuditDescriptorNativeObservation.lean`) |
| `python3 -m unittest discover -s scripts/tests` (after the final matrix regeneration) | **330** tests, exit 0 |
| `python3 scripts/build_docs_index.py --check` | exit 0 |
| provider `cargo test ... --workspace --locked --offline --no-fail-fast` | **124 passed / 0 / 0** over 39 summaries |
| provider fmt, provider clippy `-D warnings` | exit 0 / exit 0 |
| isolated package mutation control | 1 passed, exit 0 |
| `python3 providers/contracts.py check` | exit 0 |
| `providers/package.py build`, then `verify` for all five packages | exit 0 (build) and exit 0 (each of the five) |
| `python3 -m unittest discover -s providers/tests` | 13 tests, exit 0 |

The failed attempts are kept and are not evidence:

* `measure/gate-2`, on the pre-merge tree, failed one test.
  `contract_loader.rs` still pinned 108 modules and 207 rejected declarations.
  The pin was updated deliberately: the corpus now has 109 modules, and the
  rejected count went 206 → 216 after main's 207 → 206.
* `measure/gate-3` was stopped when main moved.
* `measure/gate-5` ran the Python suite before the matrix was regenerated,
  and its two derived-facts checks failed as expected.

The acceptance matrix was regenerated with
`build_matrix.py . --gate docs/reports/w43-descriptor-native-check/gate.json`.
`clang-version.txt` beside the gate is this checkout's own recording. The
matrix has 321 follow-ups, 173 Lean audits and 109 contract modules.

```sh
# 1. Private copy and build of the pinned profile (the pinned checkout is only read).
python3 private/w43-descriptor/copy_source.py <pin>/fluent-bit-clang-source private/w43-descriptor/fluent-bit-source
cmake -S private/w43-descriptor/fluent-bit-source -B private/w43-descriptor/fluent-bit-build \
  -DCMAKE_C_COMPILER=/usr/bin/clang -DCMAKE_CXX_COMPILER=/usr/bin/clang++ -DCMAKE_C_STANDARD=17 \
  -DCMAKE_C_FLAGS:STRING=-std=gnu17 -DCMAKE_EXPORT_COMPILE_COMMANDS=ON -DFLB_DEV=ON -DFLB_EXAMPLES=ON
python3 docs/reports/w43-descriptor-native-check/scripts/compare_compile_db.py \
  <pin>/fluent-bit-clang-source <pin>/fluent-bit-clang-production \
  "$PWD/private/w43-descriptor/fluent-bit-source" "$PWD/private/w43-descriptor/fluent-bit-build"
cmake --build private/w43-descriptor/fluent-bit-build --target fluent-bit-bin flb-it-pipe flb-it-network \
  flb-it-fstore flb-it-utils flb-it-file -j14
# 2. Relink observed targets, run, aggregate, pin.
CARGO_INCREMENTAL=0 cargo build --release -p qcore-contract --example check_descriptor_observations
python3 docs/reports/w43-descriptor-native-check/scripts/build_targets.py <pin>/fluent-bit-clang-source \
  private/w43-descriptor/fluent-bit-source private/w43-descriptor/fluent-bit-build OUT/targets
python3 docs/reports/w43-descriptor-native-check/scripts/run_scenarios.py OUT/targets OUT/run \
  --source-root private/w43-descriptor/fluent-bit-source
python3 docs/reports/w43-descriptor-native-check/scripts/aggregate.py OUT/run OUT/aggregate.json
python3 docs/reports/w43-descriptor-native-check/scripts/pin_records.py OUT docs/reports/w43-descriptor-native-check
# 3. Re-check the pinned records alone (no native rebuild needed).
python3 docs/reports/w43-descriptor-native-check/scripts/verify_records.py docs/reports/w43-descriptor-native-check SCRATCH
```

`<pin>` is `/home/cosmo/Documents/GitHub/qcore-c/build/cycle19/c-gateway-wave-pin`.

**Recorded measurement.** The pinned run is
`private/w43-descriptor/final-20260925b` in this worktree. Two earlier
complete measurements were superseded:

* `final-20260925`, 71 runs, before the Monkey driver and a corrected scenario
  note;
* the `dev*` iterations, which found three observer artifacts (an `openat`
  injection source, an `EAI_SYSTEM` injection errno and an overlong Unix
  socket path). All three were fixed before the pinned run.

`verify_records.py` over the pinned archive reproduced all 77 run results and
the aggregate byte for byte (exit 0).

**Regression pins.**
* `crates/qcore-contract/tests/unit/descriptor_observation.rs` has 16 tests:
  * 13 synthetic tests, covering each property, each rejection, and policy
    mutation;
  * 3 that replay pinned real runs (`engine.dummy_null`, `probe.libc`,
    `monkey.bind_in_use`) from `tests/fixtures/descriptor/`.
* The Lean executable anchors (`double_close_rejected`,
  `use_after_close_rejected`, `stream_released_as_directory_rejected`,
  `counted_dl_references_accepted`, `live_identity_reacquisition_rejected`)
  have the same shapes as those Rust tests.

## 12. Files likely to conflict

* `compiler/tests/qcore_contract_corpus.rs` and `compiler/tests/contract_loader.rs`:
  module count 108 → 109, the new entries, and rejected declarations 206 → 216.
  The tracking card also edits these tables.
* `providers/contract-load-path.json` and
  `providers/quirin-provider-shared/src/lib.rs`: the new shared module.
* `proof/QCore.lean`: one import.
* `crates/qcore-contract/src/lib.rs`: one `pub mod`.
* `docs/STATUS.md`, `docs/reviews/r0_3/acceptance_matrix.json`,
  `docs/development/increments/wave-43.md` and its index, and
  `docs/reports/INDEX.md`.
