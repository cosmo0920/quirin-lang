# W43-DESCRIPTOR-NATIVE-CHECK: descriptor lifecycles checked on native executions

> Historical checkpoint: source filenames, runnable references or discovery scripts in this archive were updated by the [extension migration](../extension-physical-migration/README.md). Recorded results, hashes and gate counts describe the original revision, not a new run of the maintained files. Use [current status](../../STATUS.md) for present measurements.


Card W43-DESCRIPTOR-NATIVE-CHECK, 2026-09-25, on base `f8b41209`. Phase A was
measured with main `0a07cb57` merged. **Phase B** binds the records to the libc
descriptor contracts that W43-DESCRIPTOR-RESOURCE-TRACKING landed on main
`d2dd61c1`, merged here at `cf3d1b75`. **Both phases are complete.** The
gate and arbiter figures in §9 and §11 are measured on the merged tree.

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
* 81 pinned runs of the pinned Fluent Bit build, Monkey sockets, ctraces and
  libc, including 36 deterministic failure injections;
* (Phase B) a Rust binding that checks every recorded execution of a declared
  libc callee against that callee's **loaded** contract.

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
* **Runs: 81, 80 accepted as evidence, 7,416 records.**
  * 35 upstream `tests/internal` runs: pipe, file, network, fstore and utils,
    one process per test case.
  * 18 runs of the actual `fluent-bit` binary: dummy, tcp input with
    filesystem storage, the HTTP server, dynamic plugin loading, and (Phase B)
    `event_type` traces with standard input open and closed.
  * 6 Monkey `mk_socket.c` driver runs.
  * 2 (Phase B) runs of the actual `ctr_random_get`, called directly.
  * 20 libc-probe runs.

  The one rejected run aborted in a heap **double free**
  ([§4.3](#43-storage-scan-failure-double-free-f3)). It has no exit record,
  so it is not descriptor evidence.
* **Properties.** Over every accepted run:
  * **Hold:** no use after release (5,459 checks), release matches kind
    (1,338), leak set well defined (80), generation discipline (1,827),
    counted dl references (20), sentinel validity (1,375), relations (168) and
    mapping extents (63).
  * **Violated: each generation is released at most once.** 61 violations in
    13 runs, from two upstream double closes
    ([§4.1](#41-engine-channel-double-close-f1),
    [§4.2](#42-scheduler-timer-double-close-f2)). Each second close was
    answered with `EBADF`.
  * **Violated: operand identity valid.** 37 runs call `close(-1)`
    ([§4.1](#41-engine-channel-double-close-f1)).
* **Phase B: 0 contradictions.** 3,149 recorded executions of the 18 declared
  libc callees the runs reach were bound to their loaded contracts. Of these,
  1,562 are the eight descriptor-family callees: `open`, `close`, `socket`,
  `fopen`, `fclose`, `fileno`, `opendir` and `closedir`. No execution
  contradicts a contract ([§8](#8-phase-b-the-landed-contracts-against-the-executions)).
  * **Caller breaks, not contract faults:** 98 `close` executions broke the
    `valid` precondition: 37 `close(-1)` and 61 second closes.
  * **`open` returned 0 once**, in the direct `ctr_random_get` run with
    standard input closed. That run leaves descriptor 0 live at exit, which is
    the native witness the tracking card's provisional Unsafe lacked. The real
    binary reached `ctr_random_get` 10 times but never with a free descriptor
    0, because `flb_create`'s data pipe takes it first and is then never
    closed ([§4.6](#46-data-pipe-zero-guard-f6)).
* **Refusals.** The recorded shapes support every refusal ground of the
  tracking card that an execution can speak to. They add one ground for
  `dlopen`: the loader returns an already-live handle. For `accept4` the
  executions follow the `socket` shape exactly; its refusal rests on the ABI
  reader, which executions cannot address.
* **Lean.** [`QCore.DescriptorNativeObservation`](../../../proof/QCore/DescriptorNativeObservation.lean)
  is audited by `AuditDescriptorNativeObservation.lean`: 35 theorems, no
  placeholders, no project axioms. Phase A's 25 prove the four required
  properties over accepted traces: at-most-once release, no use after release,
  release matching the acquired kind, and a well-defined leak set with an
  origin for every leaked generation. Phase B's 10 model the owned-result
  binding rule, show that a caller's precondition break is a rejected
  lifecycle step, and state the `fd > 0` shape.
* **Follow-ups.** This card registers 9 and retires its own Phase B follow-up
  (see the list in [§10](#10-follow-ups)).

## 1. Method and identity

| Piece | Where |
|---|---|
| Observer (C) | [`harness/descriptor_observer.c`](harness/descriptor_observer.c) |
| Monkey socket driver | [`harness/monkey_socket_driver.c`](harness/monkey_socket_driver.c) |
| ctraces random driver (Phase B) | [`harness/ctraces_random_driver.c`](harness/ctraces_random_driver.c) |
| libc probe | [`harness/libc_descriptor_probe.c`](harness/libc_descriptor_probe.c) |
| Fixture plugin DSO | [`harness/flb_out_qdesc_plugin.c`](harness/flb_out_qdesc_plugin.c) |
| Build / relink / interposition audit | [`scripts/build_targets.py`](scripts/build_targets.py), [`build.json`](build.json) |
| Compile-database identity | [`scripts/compare_compile_db.py`](scripts/compare_compile_db.py) |
| Scenarios and runner | [`scripts/run_scenarios.py`](scripts/run_scenarios.py) |
| Aggregation | [`scripts/aggregate.py`](scripts/aggregate.py), [`aggregate.json`](aggregate.json) |
| Pinned records | [`records/observations.tar.gz`](records/observations.tar.gz) (SHA-256 `d83d1377…`), [`records/manifest.json`](records/manifest.json) |
| Replay and binding re-check | [`scripts/verify_records.py`](scripts/verify_records.py) |
| Contract binding (Rust, Phase B) | [`qcore_contract::descriptor_contract_binding`](../../../crates/qcore-contract/src/descriptor_contract_binding.rs), example `bind_descriptor_contracts`, [`scripts/bind_contracts.py`](scripts/bind_contracts.py), [`binding.json`](binding.json) |
| Policy (QCore) | [`library/descriptor_native_observation.qv`](../../../providers/quirin-provider-shared/contracts/library/descriptor_native_observation.qv) |
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
| `release_at_most_once` | 1,399 | **61** | 13 / 55 |
| `no_use_after_release` | 5,459 | 0 | 0 / 78 |
| `release_matches_kind` (incl. custody: a stream's descriptor released only through the stream) | 1,338 | 0 | 0 / 55 |
| `leak_set_well_defined` (observer live set = replayed; one generation per identity; every live generation has an origin) | 80 | 0 | 0 / 80 |
| `generation_fresh` (acquire only a non-live identity; generation + 1; a kernel `EBADF` on an observed-live close also fails here) | 1,827 | 0 | 0 / 80 |
| `counted_reference` (dl handles) | 20 | 0 | 0 / 20 |
| `sentinel_validity` (failure returns the call's sentinel, acquires nothing, errno where required; success returns a valid identity and acquires what the call makes) | 1,375 | 0 | 0 / 61 |
| `relation_holds` (`fileno`/`dirfd` return the owner's descriptor; `fdopen` adopts a live caller-owned descriptor; `dup` distinct; `dup2` returns its target) | 168 | 0 | 0 / 21 |
| `operand_identity_valid` (no release/use receives `-1`/`NULL`) | 6,895 | **37** | 37 / 80 |
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
sequence (13 runs). `flb_config_exit` (`src/flb_config.c:607`) closes
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

**Observed.** 48 second closes in 12 runs. `flb_sched_destroy`
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

### 4.6 Data-pipe zero guard (F6)

This finding is Phase B's; the stdin-closed `event_type` scenario surfaced it.

**Observed.** Fluent Bit is started with descriptor 0 closed, as a service
launched without standard input would be. `flb_create`'s
`flb_pipe_create(config->ch_data)` (`src/flb_lib.c:181`) then returns the pair
`[0, 3]`, and both descriptors are live at exit
(`engine.event_type_traces_stdin_closed`).

**Mechanism.** `flb_config_exit` guards the pipe's destroy with
`if (config->ch_data[0])` (`src/flb_config.c:611`), a nonzero test. A pipe
whose read end is descriptor 0 is therefore never destroyed. This is the same
guard class as `ctr_random_get`'s `fd > 0` and the `ch_event.fd` test in
§4.1.

**Consequence.** In the CLI the process exits right after, so the leak
outlives nothing. An embedding that calls `flb_destroy` and keeps running
would keep both descriptors; that is inferred, not measured. The same
startup is why the real binary never hands descriptor 0 to `ctr_random_get`
(§8.2).

## 5. Live at exit

Only non-inherited generations are listed; inherited stdio and descriptors 0–2
are always live.

| Origin | Runs | Reading |
|---|---:|---|
| `mmap` in `backtrace_alloc` ← `flb_stacktrace_init` | 17 | libbacktrace's process-lifetime arena; intentional, not a defect |
| `epoll_create1` and two `pipe`s in `flb_create` | 2 | `plugin_missing` and the injected-`dlopen` run exit from `flb_main_run` right after the failure without `flb_destroy`; process exit follows; not a defect |
| `pipe` in `flb_create` (`ch_data`, descriptors 0 and 3) | 1 | §4.6 |
| `open` in `ctr_random_get` (descriptor 0) | 1 | §8.2 |
| `dlopen` in `flb_plugin_proxy_create` | 1 | §4.5 |
| `socket` in `mk_socket_create` ← `mk_socket_server` | 1 | §4.4 |

Every other run, including every graceful engine shutdown, releases every
descriptor, stream, directory, handle, list and mapping it acquired.

## 6. Coverage

Executions per callee, summed over all 81 runs. "Natural failures" counts
records carrying a nonzero errno that were not injected.

| Callee | Executions | Natural failures | Injected | Reached from |
|---|---:|---:|---:|---|
| `open` | 181 | 23 | 3 | Fluent Bit (incl. `ctr_random_get`), tests, ctraces driver, probe |
| `socket` | 85 | 20 | 5 | Fluent Bit, tests, Monkey, probe |
| `accept4` | 57 | 25 | 4 | Fluent Bit, tests, Monkey, probe |
| `pipe` | 205 | 0 | 2 | Fluent Bit, tests, probe |
| `epoll_create1` | 101 | 20 | 2 | Fluent Bit, tests, probe |
| `timerfd_create` | 121 | 0 | 0 | Fluent Bit |
| `fopen` / `fclose` | 59 / 74 | 22 | 1 | Fluent Bit, tests, probe |
| `opendir` / `closedir` | 50 / 47 | 20 | 2 | Fluent Bit (ChunkIO scan), tests, probe |
| `dlopen` / `dlsym` / `dlclose` | 66 / 24 / 42 | — | 2 | Fluent Bit (`flb_plugin_load`, proxy), probe |
| `getaddrinfo` / `freeaddrinfo` | 64 / 41 | — | 3 | Fluent Bit, tests, Monkey, probe |
| `mmap` / `munmap` | 85 / 44 | 20 | 3 | Fluent Bit (ChunkIO, libbacktrace), tests, probe |
| `close` | 1,031 | — | — | all |
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

## 8. Phase B: the landed contracts against the executions

W43-DESCRIPTOR-RESOURCE-TRACKING landed on main `d2dd61c1`, which this card
merged at `cf3d1b75`. Its descriptor contracts live in
`providers/quirin-provider-libc/contracts/libc/`:

* `open` and `close` in `unistd.qcore`;
* `socket` in `socket.qcore`;
* `fopen`, `fclose` and `fileno` in `stdio.qcore`;
* `opendir` and `closedir` in `dirent.qcore`.

They are trusted assumptions: `resource_return_owned(sentinel|nullable)` with
`resource_return_kind(descriptor|stream|directory)` for the constructors,
`resource_released(valid|nonnull)` with `resource_kind(...)` for the
releases, and a neutral `fileno`.

### 8.1 Method

[`qcore_contract::descriptor_contract_binding`](../../../crates/qcore-contract/src/descriptor_contract_binding.rs)
loads the four modules with the resource-v1 loader and checks each recorded
execution against the **loaded record** of its callee's contract. It uses no
per-callee table:

| Contract position | What every recorded execution must show |
|---|---|
| owned result of kind `K` | A failing execution returns `K`'s failure value (`-1` for the integer `descriptor` kind, NULL otherwise), is admitted only if the contract says the result may fail, and acquires nothing. Any other execution acquires exactly one identity, of kind `K`, equal to the result, not live before, and (descriptor) nonnegative. A new stream or directory may derive its own descriptor. |
| released parameter of kind `K` | The record releases exactly one identity, of kind `K`; a stream or directory release may end its own descriptor with it. |
| borrowed parameters, `none` result | The record acquires, adopts, derives, replaces and releases nothing. |

A failure of any of these is a **contradiction**, meaning the contract states
something an execution did not do.

A caller that breaks a release's precondition is recorded separately, as a
**caller observation**, and is not a contradiction. The precondition is a
`valid`/`nonnull` identity that is live and not owned by a stream or
directory. The Lean theorems `release_of_a_dead_identity_is_rejected` and
`release_of_another_kind_is_rejected` state that such a call is a rejected
lifecycle step.

The binding refuses rather than passes three cases:

* a contract shape it cannot bind, such as a heap-kind owned result, a
  reallocated or derived result, or an out, stored or consumed parameter;
* injected failures, which are counted but never bound;
* calls without a loaded contract, which get a descriptive shape profile and
  nothing else.

The unit tests are in
[`descriptor_contract_binding.rs`](../../../crates/qcore-contract/tests/unit/descriptor_contract_binding.rs):
12 of them.

* **Faithful executions** of every landed contract bind cleanly.
* **Contradictions:** each mutated execution contradicts. The mutations are a
  wrong kind, a wrong identity, a missing acquisition, a failure that acquires,
  a result of `-2`, a stale owned result, a neutral call that acquires or
  releases, and a release of another kind.
* **Caller observations:** a caller break yields one, not a contradiction.
* **Fail-closed:**
  * a hypothetical `valid` result is contradicted by one `-1`, which the
    landed `sentinel` admits;
  * an unbindable heap shape fails closed.
* **Pinned real runs:** three tests replay pinned runs.

`verify_records.py` re-derives every run's `binding.json` and the
[`binding.json`](binding.json) aggregate byte for byte.

### 8.2 Results over the 80 accepted runs

**0 contradictions.** 18 declared callees are reached, for 3,149 executions
(10 of them injected).

| Callee | Executions | Successes / failures (sentinel or NULL) | Injected | Contradictions | Notes |
|---|---:|---:|---:|---:|---|
| `open` | 181 | 155 / 23 | 3 | 0 | **one success returned descriptor 0** (below) |
| `socket` | 85 | 60 / 20 | 5 | 0 | |
| `fopen` | 58 | 35 / 22 | 1 | 0 | every success derives the stream's descriptor |
| `opendir` | 49 | 28 / 20 | 1 | 0 | every success derives the directory's descriptor |
| `close` | 1,031 | — | — | 0 | caller observations: 37 `invalid_argument` (`close(-1)`, §4.1), 61 `not_live` (the second closes of §4.1 and §4.2); no close of a live descriptor failed |
| `fclose` | 73 | — | — | 0 | each ended the stream's descriptor with it; none failed |
| `closedir` | 47 | — | — | 0 | likewise |
| `fileno` | 38 | — | — | 0 | never acquired; the replay's `relation_holds` shows each result is the stream's own descriptor |
| `read`, `write`, `recv`, `send`, `listen`, `setsockopt`, `getsockopt`, `getpeername`, `fprintf`, `fflush` | 1,587 | — | — | 0 | neutral: no custody moved |

A contract's declared failure mode is exercised whenever a failure was
observed. `open`, `socket`, `fopen` and `opendir` all failed naturally
(`ENOENT` for the path and directory constructors, `EAFNOSUPPORT` for `socket`) in addition to the injected failures.
None of the three releases was observed failing on a live identity, so "a
failed release still releases" (the Linux reading both contracts state) was
**not exercised**.

**`ctr_random_get` and descriptor 0.** The tracking card reports
`ctr_random_get` provisional Unsafe. Its `if (fd > 0)` guard skips `read` and
`close` when `open` returns 0, and the card recorded the witness status as
"reachable only if `open` returns 0; not observed natively". The pinned build
compiles that arm: its configure test leaves `CTR_HAVE_GETRANDOM` empty, so
the `/dev/urandom` route is the live Linux path.

* **In none of the 77 Phase A runs** did `open` or `socket` return 0.
  Descriptor 0 was always inherited.
* **Through the real binary** (`engine.event_type_traces`, `in_event_type`
  generating traces), `ctr_random_get` ran 5 times and each `open` descriptor
  was closed. With descriptor 0 closed at spawn
  (`engine.event_type_traces_stdin_closed`), it ran 5 more times and `open`
  returned 54 every time. `flb_create`'s data pipe had already taken
  descriptor 0, and that pipe is itself never closed (§4.6).
* **Called directly** (`ctraces.random.stdin_closed`), with the driver's own
  `stdin` stream closed first, `open` **returned 0**. The landed `open`
  contract admits 0 as a valid descriptor, and the binding records it as a
  success, not a contradiction. The guard then skipped `read`/`close`, and
  descriptor 0 generation 2 is **live at exit**.

This is a native witness of the provisional Unsafe under its stated
precondition, a process without standard input. It is recorded as
`W43-DESC-CTR-RANDOM-ZERO-DESCRIPTOR`, not as an upstream defect claim. The
Lean theorem `zero_descriptor_is_acquired_and_missed_by_a_positive_guard`
states the shape. The companion `stdin_open` run closes its descriptor.

### 8.3 The refused callees against their executions

The tracking card refused several callees by name. The shape profiles in
[`binding.json`](binding.json) (`unbound`) record what those callees did. This
is evidence only: nothing is declared.

| Refused callee | Tracking card's ground | What the executions show | Verdict on the refusal |
|---|---|---|---|
| `getaddrinfo` / `freeaddrinfo` | the list is valid exactly when the call returns 0 | 41/41 returns of 0 acquired a list, the 20 nonzero (EAI) returns acquired nothing, and the result never equals the list | **ground confirmed.** A nullable or owned result would misstate both arms. |
| `fcntl` | a creator only for `F_DUPFD`/`F_DUPFD_CLOEXEC` | 38 executions with commands 2, 3 and 4 (`F_SETFD`, `F_GETFL`, `F_SETFL`) acquired nothing; all 18 non-injected `F_DUPFD_CLOEXEC` executions acquired a descriptor | **ground confirmed.** The creator effect is command-dependent. |
| `accept4` | `__SOCKADDR_ARG` is unreadable to the declared-ABI reader | 25 returns of `-1` acquired nothing; 28 successes each acquired one descriptor equal to the result, never live before | the executions follow the landed `socket` shape exactly. **The semantic reading would hold, but the refusal rests on the ABI reader, which executions cannot address**, so it stays necessary. |
| `dlopen` / `dlclose` | withdrawn for the cast-return false leak (`RETURN-CAST-PLACE-NOT-ESCAPED`) | 43 successes; in 20 runs the second `dlopen` of one object returned the **already-live** handle, and the replay's `counted_reference` holds. That is 19 probe runs loading `libm` twice, plus the real binary in `engine.dynamic_plugin_twice`, which loaded the same plugin DSO twice through `flb_plugin_load` | **a further, semantic ground.** A single-owner `owned(nullable)` result would misstate the second acquisition as fresh custody. |
| `pipe`, `socketpair` | an `int[2]` out slot | 197 + 19 successes returned 0 and acquired two descriptors that are not the result | **ground confirmed.** |
| `fdopen` | transfer into the stream on success only | 18 successes adopted the descriptor; the 19 NULL returns adopted nothing, and the caller then closed its own descriptor | **ground confirmed.** |
| `mmap` / `munmap` | pointer sentinel `MAP_FAILED` | 20 failures returned `MAP_FAILED` (recorded as `-1`) and acquired nothing | **ground confirmed.** |
| `fts_open`/`fts_close`, `glob`/`globfree` | unreadable prototype; in-place lifecycle plus callback | **not executed** by any run | no evidence either way. |

No execution shows any refusal to be unnecessary on semantic grounds. The one
refusal whose ground executions cannot reach is `accept4`, and its recorded
behaviour matches the declared `socket` contract.

## 9. Corpus arbiter

This card changes no gateway, no contract selected for the corpus and no
verdict-bearing code path. The arbiter ran once at the measured tip
`c18a0675` (main `d2dd61c1` merged), with main's runner
`docs/reports/w43-indirect-access-general/scripts/run_audit.py`. The record is
[`arbiter.json`](arbiter.json); Phase A's record is kept in it under
`phase_a`.

* **Against the combined baseline.** The baseline is
  `docs/reports/w43-descriptor-resource-tracking/integration/artifacts/`, with
  the root alias this worktree → `hungry-thompson-1740da`.
  `baseline_identity.py` reports the verdict-bearing diagnostics
  **byte-identical**. `chunk-contracts` and `monkey-source` are also
  byte-identical, and `contracts` is identical after the alias.
  `source-effects` differs in 53 leaves. All of them lie under digest-keyed
  `bounds` entries (obligation ids and build, configuration and task digests),
  and none mentions this card's module.
* **Against a same-source BEFORE run.** Main `d2dd61c1` was built in its own
  temporary checkout (libevent submodule initialized) and audited the same
  way. Its `source-effects` is **byte-identical** to this card's, so the
  53-leaf difference is main's own and not this card's. All five artifacts
  equal the AFTER run once the checkout root is rewritten: four are
  byte-identical, and `contracts` differs only in its root spelling. The
  temporary checkout has been removed.

The card is verdict-neutral.

## 10. Follow-ups

**Registered (9)** in `named_followups`, each with its discharge condition:

* `W43-DESC-NATIVE-COVERAGE-GAPS`
* `W43-DESC-FLB-CONFIG-CHANNEL-DOUBLE-CLOSE`
* `W43-DESC-FLB-SCHED-TIMER-DOUBLE-CLOSE`
* `W43-DESC-FLB-STORAGE-SCAN-DOUBLE-FREE`
* `W43-DESC-MONKEY-BIND-FAILURE-LEAK`
* `W43-DESC-FLB-PROXY-HANDLE-UNRELEASED`
* `W43-DESC-OBSERVER-ORDERING-BOUNDARY`
* `W43-DESC-CTR-RANDOM-ZERO-DESCRIPTOR` (Phase B)
* `W43-DESC-FLB-CONFIG-DATA-PIPE-ZERO-GUARD` (Phase B)

**Retired (1): `W43-DESC-NATIVE-PHASE-B`.** It was registered with the Phase A
report and is discharged by §8, where every landed contract is bound to its
callee's records and every contradiction would be reported. There are none.
The retirement is recorded in [`retired-followups.json`](retired-followups.json).
The tracking card's `W43-DESCRIPTOR-REFUSED-SHAPES`, `RETURN-CAST-PLACE-NOT-ESCAPED`
and `W43-DESCRIPTOR-ACCUSING-ARMS-UNEXERCISED-IN-CORPUS` are unchanged: this
card declares nothing. `CT50-PROOF-ADDRESS-REUSE-UNPROVED` is also unchanged.

## 11. Validation and reproduction

The gate was measured on the merged tree at `c18a0675` with
`scripts/run_gate.py` and `CARGO_INCREMENTAL=0`. The dev profile was reclaimed
after the test step. Every exit was captured to a file. The records are
[`gate.json`](gate.json) and [`gate-facts.json`](gate-facts.json). The logs
are in worktree-private `measure/gate-b3` (submodules, clang, tests),
`measure/gate-b4` (lint, probes, Lean and the provider gate) and
`measure/gate-b7` (Python and docs index after the final matrix regeneration).

| Step | Result |
|---|---|
| `git submodule update --init --recursive` | exit 0 |
| `cargo test --workspace --no-fail-fast` | **5,353 passed / 0 failed / 11 ignored** over 286 summaries |
| `cargo fmt --all -- --check`; `cargo clippy --workspace --all-targets -- -D warnings` | exit 0 / exit 0 |
| `reevaluation_probe`, `reevaluation_followup` | exit 0 / exit 0 |
| `python3 scripts/check_lean_audits.py` | **176 Lean audits** passed |
| `python3 -m unittest discover -s scripts/tests` (after the final matrix regeneration) | **330** tests, exit 0 |
| `python3 scripts/build_docs_index.py --check` | exit 0 |
| provider `cargo test ... --workspace --locked --offline --no-fail-fast` | **124 passed / 0 / 0** over 39 summaries |
| provider fmt, provider clippy `-D warnings` | exit 0 / exit 0 |
| isolated package mutation control | 1 passed, exit 0 |
| `python3 providers/contracts.py check` | exit 0 |
| `providers/package.py build`, then `verify` for all five packages | exit 0 (build) and exit 0 (each of the five) |
| `python3 -m unittest discover -s providers/tests` | 13 tests, exit 0 |

The failed attempt is kept and is not evidence. `measure/gate-b1` + `gate-b2`
at `3ff08bdb` passed every step except workspace clippy (exit 101): one
`unnecessary_get_then_check` in the new binding. The fix, `c18a0675`, changes
that expression only, and the whole gate was rerun as `gate-b3` + `gate-b4`.
`measure/gate-b5` ran the Python suite before the matrix was regenerated, and
its two derived-facts checks failed as expected. `gate-b6` was the first run
after the regeneration. `gate-b8` confirms the Python suite and the docs index
after the last one.
Phase A's gate at `662af52f` (5,270 / 0 / 11, 173 Lean audits, 330 Python
tests, provider 124 / 0 / 0) is superseded by this one.

The acceptance matrix was regenerated with
`build_matrix.py . --gate docs/reports/w43-descriptor-native-check/gate.json`.
`clang-version.txt` beside the gate is this checkout's own recording.

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
# 2. Relink observed targets, run, aggregate, bind (Phase B), pin.
CARGO_INCREMENTAL=0 cargo build --release -p qcore-contract --example check_descriptor_observations \
  --example bind_descriptor_contracts
python3 docs/reports/w43-descriptor-native-check/scripts/build_targets.py <pin>/fluent-bit-clang-source \
  private/w43-descriptor/fluent-bit-source private/w43-descriptor/fluent-bit-build OUT/targets
python3 docs/reports/w43-descriptor-native-check/scripts/run_scenarios.py OUT/targets OUT/run \
  --source-root private/w43-descriptor/fluent-bit-source
python3 docs/reports/w43-descriptor-native-check/scripts/aggregate.py OUT/run OUT/aggregate.json
python3 docs/reports/w43-descriptor-native-check/scripts/bind_contracts.py OUT/run OUT/binding.json
python3 docs/reports/w43-descriptor-native-check/scripts/pin_records.py OUT docs/reports/w43-descriptor-native-check
# 3. Re-check the pinned records alone (no native rebuild needed).
python3 docs/reports/w43-descriptor-native-check/scripts/verify_records.py docs/reports/w43-descriptor-native-check SCRATCH
```

`<pin>` is `/home/cosmo/Documents/GitHub/qcore-c/build/cycle19/c-gateway-wave-pin`.

**Recorded measurement.** The pinned run is
`private/w43-descriptor/final-20260925d` in this worktree: 81 runs, including
Phase B's `event_type` and `ctr_random_get` scenarios. It supersedes three
earlier complete measurements:

* `final-20260925b`, 77 runs, pinned by Phase A at `662af52f`;
* `final-20260925c`, 79 runs, pinned at `ad5a0f21` before the `event_type`
  scenarios existed;
* `final-20260925`, 71 runs, before the Monkey driver.

The `dev*` iterations found four harness artifacts, all fixed before the
pinned runs: an `openat` injection source, an `EAI_SYSTEM` injection errno, an
overlong Unix socket path, and a driver that closed descriptor 0 directly
instead of through its stream.

`verify_records.py` over the pinned archive reproduced all 81 run results, the
aggregate, every run's `binding.json` and the binding aggregate byte for byte
(exit 0). Phase A's records are those of its commit.

**Regression pins.**
* `crates/qcore-contract/tests/unit/descriptor_observation.rs` has 16 tests:
  * 13 synthetic tests, covering each property, each rejection, and policy
    mutation;
  * 3 that replay real runs pinned by Phase A (`engine.dummy_null`,
    `probe.libc`, `monkey.bind_in_use`, from `final-20260925b`).
* `crates/qcore-contract/tests/unit/descriptor_contract_binding.rs` has 12
  tests: 10 synthetic, 1 over two Phase A runs, and 1 over
  `ctraces.random.stdin_closed` (from `final-20260925c`). The fixtures are in
  `tests/fixtures/descriptor/`.
* The Lean executable anchors have the same shapes as those Rust tests:
  * the lifecycle anchors `double_close_rejected`, `use_after_close_rejected`,
    `stream_released_as_directory_rejected`, `counted_dl_references_accepted`
    and `live_identity_reacquisition_rejected`;
  * the binding anchors `open_failure_binding_anchor`,
    `open_success_binding_anchor` and `wrong_kind_success_binding_anchor`.

## 12. Files likely to conflict

* `compiler/tests/qcore_contract_corpus.rs`, `compiler/tests/contract_loader.rs`
  and `scripts/tests/test_contract_corpus.py`: on main's figures, module count
  110 → 111, the new entries, `quirin-provider-*` modules 38 → 39, and
  rejected declarations 206 → 216 (selection 47 and v1 declarations 152
  unchanged).
* `providers/contract-load-path.json` and
  `providers/quirin-provider-shared/src/lib.rs`: the new shared module.
* `proof/QCore.lean`: one import.
* `crates/qcore-contract/src/lib.rs`: two `pub mod`s.
* `docs/STATUS.md`, `docs/reviews/r0_3/acceptance_matrix.json`,
  `docs/development/increments/wave-43.md` and its index, and
  `docs/reports/INDEX.md`.
