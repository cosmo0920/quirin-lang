# W43-LIBC-RESTRICT-PROTOTYPES: ten `__restrict` libc prototypes, and the array-bound qualifier

Card W43-LIBC-RESTRICT-PROTOTYPES, 2026-09-25, on base `f8b41209` (main after
the CONTRACT-INHERIT-ADOPTION integration). It discharges
`W43-LIBC-RESTRICT-PROTOTYPES-UNDECLARED`.

W43-LIBC-RESULT-ALIAS taught the declared-ABI reader the qualifier after a
parameter's last `*`. That made fourteen glibc prototypes readable. It declared
none of them. The printf family is card W43-LIBC-PRINTF-FAMILY's. This card
handles the other ten:

* **stat and time:** `stat`, `strftime`, `gettimeofday`;
* **sockets:** `getsockopt`, `getpeername`;
* **dynamic loading:** `dlsym`;
* **threads and signals:** `cnd_timedwait`, `pthread_sigmask`;
* **regex:** `regexec`, `regerror`.

`regexec` had a second reader boundary, a qualifier inside an array bound, and
this card reads it with Lean parity.

A [prediction](PREDICTION.md) was frozen and committed (`ddca44c0`) before any
contract module was edited and before the arbiter ran.
[§5](#5-prediction-against-outcome) compares it with the outcome.

## Headline

* **Declared: 10, refused: 0.** Every declaration is the neutral element:
  every parameter is `resource_borrowed` and the result is
  `resource_return_none()`. Every one is a trusted assumption (trust boundary
  28) and lives only in `providers/quirin-provider-libc`. Eight are appended to
  existing modules. `regexec` and `regerror` go in the new `libc/regex.qcore`,
  because no existing module matches `<regex.h>`.
* **Reader.** A bound written as a run of qualifiers followed by at most one
  size word is now read as the adjusted pointer (C11 6.7.6.3p7, then p15).
  glibc's `regmatch_t __pmatch[_Restrict_arr_ _REGEX_NELTS (__nmatch)]`
  preprocesses to `[__restrict __nmatch]` under Clang's C17. `regexec` now
  agrees in its one unit.

  The same edit narrows one shape. The W43-LIBC-RESULT-ALIAS reader dropped a
  qualifier at the end of an array's *element* declarator, so it read
  `char *const p[]` as `char **`. That array now refuses, exactly as its pointer
  spelling `char *const *p` does.

  Lean parity: 13 audited theorems added to `QCore.PointerQualifierReader`.
* **Census.** The census tool ran over the 185 captured units, once built from
  the base and once from this card
  ([abi-evidence.json](abi-evidence.json)).
  * The 130 selected declarations: all 10,686 (unit, name) rows keep their
    outcome.
  * The ten new ones:
    * 571 rows agree and 70 are undeclared (`stat`);
    * 93 rows withdraw, all of them `getpeername`, where `_GNU_SOURCE` spells
      the `__SOCKADDR_ARG` transparent union;
    * `regexec` moves from withdrawn to agreed.
* **Arbiter, in three classes.** BEFORE reproduces the W43-LIBC-RESULT-ALIAS
  diagnostics byte for byte.
  * **(a) The reader change alone** leaves all five artifacts byte-identical
    to BEFORE.
  * **(b) The declarations** remove 36 leaves and add 2, all attributed.
    **3 functions move from Unknown to Safe**: `mk_plugin_load_symbol`,
    `mk_signal_thread_sigpipe_safe` and `rd_regex_exec`.
  * **(c) Violation candidates: none.**

  Units do not move. Leak stays **272**. `unsupported` goes 48,930 → 48,898 and
  `unknown_alias` 4,657 → 4,655.
* **Out of scope, on record.** The `__SOCKADDR_ARG` transparent union is left
  unread ([§2.3](#23-the-transparent-union-left-out-of-scope)), so
  `getpeername` stays withdrawn in 93 units, its only pinned call site
  included.
* **Follow-ups 299 → 300.** One is retired. Two are registered:
  `W43-LIBC-GETPEERNAME-SOCKADDR-UNION` and `W43-LIBC-DLSYM-HANDLE-LIFETIME`.

| Measure | BEFORE (W43-LIBC-RESULT-ALIAS) | AFTER | Delta |
|---|---:|---:|---:|
| Functions Safe / provisional Unsafe / Unknown | 257 / 84 / 4,493 | **260 / 84 / 4,490** | +3 / 0 / −3 |
| Units Safe / provisional Unsafe / Unknown / data-only | 2 / 42 / 137 / 4 | 2 / 42 / 137 / 4 | 0 |
| `unsupported` | 48,930 | **48,898** | −32 |
| `unknown_alias` | 4,657 | **4,655** | −2 |
| `leak` / `summary_conflict` / `double_release` | 272 / 1 / 0 | 272 / 1 / 0 | 0 |
| `invalid_release` / `use_after_release` | 0 / 0 | 0 / 0 | 0 |
| `call to uncontracted function` leaves / functions / sole | 11,652 / 2,688 / 384 | 11,626 / 2,681 / 383 | −26 / −7 / −1 |
| Unknown functions blocked only by one uncontracted callee | 295 (230 callees) | 294 (229 callees) | −1 |
| return-expression leaves / functions / sole | 499 / 422 / 146 | 498 / 421 / 145 | −1 / −1 / −1 |
| branch-condition leaves / functions / sole | 8,712 / 2,408 / 47 | 8,709 / 2,407 / 48 | −3 / −1 / +1 |
| Contract corpus modules / manifest-selected / resource-v1 declarations | 108 / 45 / 130 | **109 / 46 / 140** | +1 / +1 / +10 |

The census rows count Unknown functions, as [blocker-census.txt](blocker-census.txt)
and [sole-callees.txt](sole-callees.txt) do. The BEFORE rows are
W43-LIBC-RESULT-ALIAS' own census files.

## 1. Declarations and their assumptions

Each prototype was read from the unit's own preprocessed glibc header (Ubuntu,
x86-64, LP64). The ABI spellings are the ones W43-LIBC-RESULT-ALIAS measured in
`measurement/restrict14.qcore`. Each module's comment carries the prototypes,
the custody reading and a "what no declaration here says" list. Each
"deliberately NOT declared" paragraph that named these functions now points to
the new section. Nothing earlier in any module is reordered.

| Module | Function | Custody reading | Faithfulness check |
|---|---|---|---|
| `sys_stat` | `stat(file, buf) -> i32` | Reads the path and fills the caller's `struct stat`, exactly `fstat`'s shape | `buf` is an out-pointer into caller storage: a borrowed record, not custody |
| `time` | `strftime(s, maxsize, format, tp) -> usize` | Writes at most `maxsize` bytes into the caller's buffer and reads `*tp` | Scalar result |
| `time` | `gettimeofday(tv, tz) -> i32` | Writes the caller's `struct timeval`; `tz` is obsolete and may be NULL | `tv` is a borrowed record |
| `socket` | `getsockopt(fd, level, optname, optval, optlen) -> i32` | Uses a descriptor; the kernel copies the option value and length into caller storage | Value copies, never a pointer the caller would own. `optval` is `void *`, not a pointer-to-pointer, so `W43-LIBC-OUT-POINTER-ALIAS`' shape does not arise |
| `socket` | `getpeername(fd, addr, len) -> i32` | Copies the peer address and its length into caller storage | As above. Withdrawn where `_GNU_SOURCE` spells the transparent union (§2.3) |
| `dlfcn` | `dlsym(handle, name) -> ptr<mut, u8>` | Returns a symbol address, `resource_return_none()` | See below |
| `threads` | `cnd_timedwait(cond, mutex, time_point) -> i32` | `cnd_wait` with a deadline: it unlocks the mutex, waits, and relocks it before returning (C11 7.26.3.5) | Same statement as `cnd_wait` and the `pthread_mutex` pair. It states no lock state, so it agrees with the existing lock contracts and leaves constraint 27's model untouched |
| `threads` | `pthread_sigmask(how, newmask, oldmask) -> i32` | Reads the new mask and writes the old one through caller pointers, either of which may be NULL | Value reads and writes of a `sigset_t`, like `sigemptyset` |
| `regex` (new) | `regexec(preg, string, nmatch, pmatch, eflags) -> i32` | Reads the compiled pattern and the string, and writes at most `nmatch` match records (`NULL` when `nmatch` is 0) | The compiled pattern stays the caller's and is unchanged. `regcomp`/`regfree` stay undeclared (`W43-LIBC-IN-PLACE-LIFECYCLE`) |
| `regex` (new) | `regerror(errcode, preg, errbuf, errbuf_size) -> usize` | Writes at most `errbuf_size` bytes of a message and returns the size it needs | Scalar result |

**`dlsym`'s result.** The brief asked for `resource_return_none()` to be
justified or the function refused. It is justified on three grounds:

* The result is **not fresh custody**. Nothing is allocated for the caller, and
  no release function applies to a symbol address.
* It is **not derived from an argument**. It points into neither `name`'s
  string nor any C object `handle` points to. The handle is opaque, and
  `RTLD_DEFAULT`/`RTLD_NEXT` point to nothing.
* So the caller acquires no obligation. That is the precedent `dlerror`,
  `gai_strerror` and `__errno_location` already set for a pointer result with
  no custody.

One thing is **not stated**: the address dies when `dlclose` closes the handle
that found it. In the pinned corpus this cannot yet make a verdict Safe,
because `dlopen`/`dlclose` stay undeclared (`W43-LIBC-DESCRIPTOR-LIFECYCLE`).
Any function that closes a handle therefore keeps an uncontracted call. The
gap is registered as `W43-LIBC-DLSYM-HANDLE-LIFETIME`, and a card that
declares `dlclose` must revisit it.

**Nothing verified here.** A wrong assumption in this table can make a `Safe`
below wrong. These premises join the verdict's assumption set.

## 2. The reader: array bounds

### 2.1 The rule and the Rust

The change is in `crates/qcore-c-gateway/src/legacy_declared_abi.rs`.

* **`array_bound_is_read`** replaces the old test, which read only an empty or
  plain decimal bound. A bound is now read when it is:
  * a run of `QUALIFIERS`,
  * followed by at most one size word, which is either a decimal constant or an
    identifier. The identifier may not be `static`, a reserved word or an
    arithmetic type word.

  `static`, `[*]`, a multi-token size and a qualifier after the size still
  refuse. No measured prototype needs any of them, so they stay refused rather
  than being admitted untested.
* **In `project_parameter`,** `drop_parameter_object_qualifiers` now applies
  only to a non-array parameter. An array parameter's own qualifiers are the
  bound's, which were just read. A qualifier left at the end of the element
  declarator qualifies the inner pointer, so `project_declarator` refuses it.

The census tool shares this code, so the arbiter and the census measure the
same reader.

Thirteen new rows in `crates/qcore-c-gateway/tests/loader_abi.rs` pin the rule:

* **Newly read:**
  * `[__restrict]`;
  * glibc `regexec`'s `[__restrict n]`, over an aggregate typedef;
  * `[const 4]`;
  * several qualifiers in one bound;
  * an array of pointers with a qualified bound.
* **Constness:** the element's constness is kept, and a mutable declaration
  refuses it.
* **Still refused:** `[static 4]`, `[*]`, `[__restrict n + 1]` and `[4 const]`.
* **The narrowing:** `char *const p[]` and `char *__restrict p[__restrict]`
  refuse.

### 2.2 Lean parity

`proof/QCore/PointerQualifierReader.lean` gains an array-bound section with
13 theorems, all registered in `proof/AuditPointerQualifierReader.lean`:

* **The rule as an equation:**
  `the_array_parameter_is_the_adjusted_pointer`. A read array parameter is
  exactly the pointer parameter that carries the bound's qualifiers after its
  `*`. That is 6.7.6.3p7 composed with the existing p15 step.
* **Conservativity:**
  * `a_bound_read_before_is_read_now`;
  * `the_array_widening_only_adds`. For an element that
    `dropObjectQualifiers` leaves unchanged, every array parameter read before
    is read identically.
* **The narrowing, and exactly where it is:**
  * `an_array_of_qualified_pointers_refuses`. Wherever the earlier reader's
    drop changed the element, the array now refuses.
  * `the_array_and_pointer_spellings_agree_on_it`.
  * `the_earlier_reader_read_it_as_two_levels`.
* **What still refuses:** `a_static_bound_still_refuses`,
  `a_star_bound_still_refuses`, `a_qualifier_after_the_size_refuses` and
  `a_multi_word_size_refuses`.
* **Non-vacuity:** the measured witness goes from unreadable to `(1, false)`
  (`the_regexec_match_array_was_unreadable`,
  `the_regexec_match_array_is_now_read`), and
  `an_array_element_constness_is_kept`.

The module stays placeholder-free and uses no project axioms. The
Rust–Lean correspondence is maintained by review, not proved (trust boundary
28).

### 2.3 The transparent union, left out of scope

Under `_GNU_SOURCE`, glibc declares
`typedef union { struct sockaddr *__restrict __sockaddr__; … } __SOCKADDR_ARG __attribute__ ((__transparent_union__));`.
Reading it would take three things:

* a new typedef class: an untagged union whose members are all single-level
  object pointers of one constness, carrying the attribute;
* its own Lean model;
* its own attribution class.

Its only measured yield is `getpeername`'s one leaf. That leaf sits in
`mk_socket_ip_str`, which has seven other blockers. The brief allowed the union
only if it was cheap, and it is not, so it is left out. The declaration is
still shipped: it is faithful, and it agrees in 12 units. In the 93
`_GNU_SOURCE` units, including monkey's `mk_socket.c` (which defines
`_GNU_SOURCE` at line 20), it is withdrawn fail-closed. The residual is
`W43-LIBC-GETPEERNAME-SOCKADDR-UNION`, beside `W43-LIBC-ABI-READER-SHAPES`'
`__CONST_SOCKADDR_ARG` for `bind`/`connect`.

## 3. Arbiter

All runs use
[`../w43-return-expression/scripts/run_audit.py`](../w43-return-expression/scripts/run_audit.py)
with the baseline's command shape, Clang (`/usr/bin/clang`, Ubuntu 21.1.8) and
the wave-20 ChunkIO package. The contract modules are read from disk at run
time, so one card binary serves both card stages.

| Run | Binary (sha256) | Declarations | Exit | Window (UTC) | Record |
|---|---|---|---:|---|---|
| BEFORE | `ecd9b2c1…` (base `f8b41209`) | base | 0 | 16:03:16–16:12:13 | [before-run.json](before-run.json) |
| READER | `c22de562…` (`ddca44c0`) | base | 0 | 16:21:00–16:28:56 | [reader-run.json](reader-run.json) |
| DECL | `c22de562…` | + the ten | 0 | 16:30:10–16:38:48 | [decl-run.json](decl-run.json) |
| AFTER | `c22de562…` (rebuilt at `fb0bec93`, identical digest) | the ten | 0 | 16:39:32–16:47:16 | [after-run.json](after-run.json), [artifacts/run.json](artifacts/run.json) |

The windows fall on 2026-09-24 in UTC, which is the evening before this
report's local date.

**BEFORE vs the W43-LIBC-RESULT-ALIAS baseline**
([before-identity.json](before-identity.json)):

* **Byte-identical:** diagnostics, chunk-contracts and source.
* **`contracts`** differs by the checkout root and by CONTRACT-INHERIT-ADOPTION's
  inclusion record, crate:module keys and `resource_string.qcore`. That
  difference is confirmed verdict-neutral.
* **`source-effects`** carries the checkout-path digests the brief lists.

**READER vs BEFORE:** all five artifacts are byte-identical. This is class (a).

**AFTER vs DECL:** all five artifacts are byte-identical. The binary rebuilt at
the implementation commit has DECL's digest: `quirin-provider-libc` is not
linked into the audit.

**AFTER vs BEFORE:** chunk-contracts, source-effects and source are
byte-identical. `contracts` differs in three places
([`scripts/contracts_diff.py`](scripts/contracts_diff.py)):

* the manifest text, which has one added line;
* the source text of `dlfcn`, `process`, `socket`, `sys_stat`, `threads` and
  `time`;
* one added source, `libc/regex.qcore`.

A private capture run used the tee wrapper
([`../w43-libc-result-alias/scripts/capture_run.sh`](../w43-libc-result-alias/scripts/capture_run.sh))
with the base binary. It supplied the 185 line-marker-stripped units for the
census and the `-E -P` texts for attribution. The pairing is 180 paired units,
plus the same 5 unpaired units as before, which project no function.

### 3.1 Attribution

[`../w43-libc-contracts/scripts/attribute.py`](../w43-libc-contracts/scripts/attribute.py)
was run on READER → DECL, with only the ten new declarations as its module
list. It exits 0 with `attribution_complete: true`
([attribution.json.gz](attribution.json.gz)). No unit status moves, and every
unit field moves only by its own leaf delta.
[`scripts/classes.py`](scripts/classes.py) splits the movement
([classes.json](classes.json)).

| Class | Removed | Added | Function moves |
|---|---:|---:|---|
| (a) reader change alone | 0 | 0 | none; all five artifacts identical |
| (b) declared-callee | 26 | — | 2 Unknown → Safe |
| (b) newly-safe-callee | 2 | — | none |
| (b) same-line | 6 | 2 | 1 Unknown → Safe |
| (b) same-function | 2 | 0 | none |
| (c) violation candidates | — | **0** | leak, `double_release`, `invalid_release`, `use_after_release`: none added |
| **total** | **36** | **2** | **3 Unknown → Safe**; `unsupported` −32, `unknown_alias` −2 |

**Per function, leaf by leaf:**

| Declared name | Leaves decided | Where | Other movement on its lines or functions |
|---|---:|---|---|
| `pthread_sigmask` | 10 | `rd_kafka_new` (3), `rd_kafka_background_thread_create` (3), `rd_kafka_broker_add` (3), `mk_signal_thread_sigpipe_safe` (1) | `mk_signal_thread_sigpipe_safe` → **Safe**, and its caller `mk_sched_launch_worker_loop` loses the newly-safe-callee leaf (38 remain) |
| `stat` | 3 | `cio_os_isdir`, `cio_utils_recursive_delete`, `mk_rconf_read` | 4 same-line removals: branch conditions `if (!stat(dir, &st))` (`cio_os_mkpath`), `if (stat(path, &target) == -1)` (`mk_file_get_info`) and `if (stat(path, &sb) == 0)` (`mk_utils_register_pid`), plus the lexical indirect access at `ret = stat(cf->path, &st)` (`cio_file_native_get_size`). One same-function `unknown_alias` on `ret` in `cio_utils_recursive_delete` |
| `strftime` | 3 | `format_metric`, `mk_clock_headers_preset`, `mk_clock_log_set_time` | +1 same-line `nested call localtime_r` in `mk_clock_log_set_time`, whose modelled `strftime` takes `localtime_r(&utime, &result)` as an argument. That function's count is unchanged at 4 |
| `gettimeofday` | 3 | `rd_kafka_log_print`, `rd_seed`, `cnd_timedwait_ms` | — |
| `dlsym` | 2 | `rd_dl_sym`, `mk_plugin_load_symbol` | `mk_plugin_load_symbol` → **Safe**. Its caller `mk_plugin_load` loses the newly-safe-callee leaf and one same-function `unknown_alias` on `plugin` (53 remain) |
| `getsockopt` | 1 | `mk_net_conn_create` (provisional Unsafe, stays) | 1 same-line lexical indirect access at `getsockopt(channel->fd, …)` (`mk_net_conn_write`) |
| `regexec` | 1 | `rd_regex_match` | 1 same-line return-expression refusal: `return regexec(&re->re, str, 0, NULL, 0) != REG_NOMATCH;`, so `rd_regex_exec` → **Safe** |
| `regerror` | 1 | `rd_regex_match` | — |
| `cnd_timedwait` | 1 | `cnd_timedwait_ms` | — |
| `getpeername` | 1 **renamed** | `mk_socket_ip_str` | +1 `call to getpeername whose declared contract was withdrawn … (abi-unreadable-declaration)`. That function's count is unchanged at 8 |

**The three functions that move Unknown → Safe.** Each is Safe **relative to
the declared premises**, which is invariant 2's reading and not a proof of the
C:

| Function | Unit | Its only BEFORE blocker |
|---|---|---|
| `mk_plugin_load_symbol` | `mk_server/mk_plugin.c` | `dlsym`, in `dlerror(); s = dlsym(handler, symbol); if (dlerror() != NULL) return NULL; return s;` |
| `mk_signal_thread_sigpipe_safe` | `mk_server/mk_scheduler.c` | `pthread_sigmask(SIG_BLOCK, &set, &old)`, after the already-declared `sigemptyset`/`sigaddset` |
| `rd_regex_exec` | `rdregex.c` | the return expression `regexec(&re->re, …) != REG_NOMATCH`, which the return route now models because its call is contracted |

`rd_regex_match` keeps `regcomp`/`regfree`, which are the in-place lifecycle
residual. `cnd_timedwait_ms` keeps eleven other leaves. No provisional-Unsafe
function moves: `mk_rconf_read` and `mk_net_conn_create` each lose one
non-leak leaf.

**Leak pin (constraint 7).** The count stays **272**, with no leak leaf added
or withdrawn. This is the fifth declarative arbiter in a row to leave the pin
alone. Every declaration borrows every argument and releases nothing. The new
baseline is [artifacts/](artifacts/) ([manifest.json](artifacts/manifest.json)).
`docs/reports/w43-libc-result-alias/artifacts/` is its frozen predecessor and
is not edited.

## 4. Pin changes, each deliberate

* `providers/quirin-provider-libc`:
  * `contracts/libc/regex.qcore` is new;
  * eight declarations are appended to `dlfcn`, `socket`, `sys_stat`, `threads`
    and `time`;
  * comment pointers are updated in those five modules and in `process`;
  * `src/lib.rs` `CONTRACTS` goes from 14 to 15 entries, and the crate doc
    names `<regex.h>`;
  * the crate README and `providers/README.md` count 15 modules.
  * `PUBLISHED` (`memory`, `string`) is untouched, so no published digest moves.
* `providers/contract-load-path.json` is written by `python3 providers/contracts.py sync`.
  The only change is `libc/regex.qcore` added to the libc crate's module list.
* `cycle19-resource-contracts.json` (schema 2): 45 → 46 modules, with `regex`
  inserted after `process`.
* `contract/corpus.json`: unchanged. The libc crate's `contracts/` directory
  is already a root.
* `compiler/tests/qcore_contract_corpus.rs`:
  * corpus size 108 → 109;
  * ten clause pins, each 2 plus 1 per parameter, in logical-path order.
* `compiler/tests/contract_loader.rs`:
  * corpus modules 108 → 109;
  * (selected, v1 declarations, rejected) (45, 130, 207) → **(46, 140, 207)**;
  * two `source_assumptions` pins 45 → 46;
  * the old-table parity list gains ten all-borrowed entries.
* `crates/qcore-c-gateway/tests/{prod_contracts,neighbours}.rs`: the
  `source_assumptions` pins 45 → 46.
* `scripts/tests/test_contract_corpus.py`: modules 108 → 109, and the
  `quirin-provider-*` share 36 → 37. The placement test passes unchanged: all
  ten names are in `libc-names.txt`, and every declaration is inside the libc
  crate.

No assertion is weakened.

## 5. Prediction against outcome

| Prediction | Outcome |
|---|---|
| P1: the reader alone moves nothing | **Held.** All five artifacts are identical, and 0 of 10,686 census rows moved |
| P2: `regexec` becomes readable | **Held.** Its one unit's row moved from withdrawn to agreed, and the leaf was decided |
| P3: `getpeername` stays withdrawn in `mk_socket.c` and its leaf is renamed | **Held** |
| P4: 25 of 26 leaves removed | **Held.** 26 were removed, 1 of them (`getpeername`) re-added as the rename |
| P5: exactly two Unknown → Safe, and nothing else | **Missed by one.** `rd_regex_exec` also moved, through its return expression. The prediction expected `&re->re` to keep a refusal. Both named functions moved, and both callers lost exactly one newly-safe-callee leaf and stayed Unknown |
| P6: small same-line movement; lines with member-read actuals keep a refusal; net −25 to −32 | **Partly missed.** Three of the six member-read lines lost their refusal: `cf->path`, `channel->fd` and `&re->re` in the return. The others kept theirs: `server->path_conf_root`, `host->documentroot.data`, `rktrans->rktrans_s` and `&re->re` in `rd_regex_comp`'s `regerror` call. The added leaf was the nested `localtime_r`, not the predicted `sizeof` or borrowed-expression gate. Net −34, two beyond the range. `unknown_alias` −2, inside ±3 |
| P7: the union is left out | **Held** |
| P8: leak 272, no candidates | **Held** |
| P9: `dlsym` declared | **Held.** Justified in §1 |

The misses sit in one place. The prediction under-estimated how often the
gateway admits a member-read actual once the call on that line is contracted.
Every extra move goes toward fewer refusals, and each is individually
attributed above.

## 6. Validation

Measured in this worktree (`agent-a8d59e97c933911a0`) by
[`scripts/run_gate.py`](scripts/run_gate.py), which is W43-LIBC-RESULT-ALIAS'
runner unchanged. It sets `CARGO_BUILD_JOBS=6` and `RUST_TEST_THREADS=2`, turns
debug info off, and captures every exit to a file, never through a pipe.

The core and provider gates ran on the tree of the implementation commit
`fb0bec93`. Later commits change documents, the ledger, report scripts and
evidence only. Logs are worktree-private under `measure/`. The generator input
is [gate.json](gate.json), with the [Clang recording](clang-version.txt) taken
in this checkout beside it.

| Command | Exit | Result | Log |
|---|---:|---|---|
| `git submodule update --init --recursive` | 0 | libevent at `d82464a2…` | `measure/gate/submodules.log` |
| `clang --version` | 0 | Ubuntu clang 21.1.8 | `measure/gate/clang.log` |
| `cargo test --workspace --no-fail-fast` | **101** | **5,207 passed / 1 failed / 11 ignored**, 278 summaries; see below | `measure/gate/test.log` |
| `cargo test -p qcore-compiler --test task_route_tsan -- --exact removing_the_join_edge_is_reported_under_every_schedule` (×5, isolated, [`scripts/isolated_rerun.py`](scripts/isolated_rerun.py)) | 0 ×5 | 5 of 5 pass, at a 1-minute load of 22–24 | `measure/gate-tsan-isolated/` |
| `cargo fmt --all -- --check` | 0 | clean | `measure/gate/fmt.log` |
| `cargo clippy --workspace --all-targets -- -D warnings` | 0 | clean | `measure/gate/clippy.log` |
| `cargo run -p qcore-compiler --example reevaluation_probe` | 0 | | `measure/gate/probe.log` |
| `cargo run -p qcore-compiler --example reevaluation_followup` | 0 | | `measure/gate/followup.log` |
| `python3 scripts/check_lean_audits.py --jobs 4` | 0 | **170** audits | `measure/gate/lean.log` |
| `python3 docs/reports/remaining-d02/build_matrix.py . --gate docs/reports/w43-libc-restrict-prototypes/gate.json` | 0 | 300 follow-ups, 109 contract modules, 170 Lean audits | `measure/build-matrix-2.log` |
| `python3 scripts/build_docs_index.py`, then `--check` | 0 / 0 | index regenerated and matching | `measure/docs-index-check-2.log` |
| `python3 -m unittest discover -s scripts/tests` (after the final regeneration) | 0 | **330** tests | `measure/python-final.log` |
| `cargo test --manifest-path providers/Cargo.toml --workspace --locked --offline --no-fail-fast` | 0 | **117 passed / 0 failed / 0 ignored**, 39 summaries | `measure/provider-gate/ptest.log` |
| `… -p qcore-provider-conformance --test packages real_packages_preserve_unknown_and_reject_mutations … -- --exact --test-threads=1` | 0 | 1 passed (isolated, constraint 16) | `measure/provider-gate/pisolated.log` |
| `cargo fmt --manifest-path providers/Cargo.toml --all -- --check` | 0 | clean | `measure/provider-gate/pfmt.log` |
| `cargo clippy --manifest-path providers/Cargo.toml --workspace --all-targets --locked --offline -- -D warnings` | 0 | clean | `measure/provider-gate/pclippy.log` |
| `python3 providers/contracts.py check` | 0 | 72 provider-owned, inventories unchanged | `measure/provider-gate/pcontracts.log` |
| `python3 providers/package.py build --output measure/provider-gate/packages` | 0 | 5 packages | `measure/provider-gate/pbuild.log` |
| `python3 providers/package.py verify …/<id>` (×5: chunkio, cmetrics, ctraces, libevent, librdkafka) | 0 ×5 | | `measure/provider-gate/pverify.log` |
| `python3 -m unittest discover -s providers/tests` | 0 | 12 tests | `measure/provider-gate/ppython.log` |

**The one core failure, stated as seen.**
`removing_the_join_edge_is_reported_under_every_schedule`
(`compiler/tests/task_route_tsan.rs`) failed in the full workspace run. It
missed its `ct07-case` cell under `forced-together`: ThreadSanitizer reported
a thread leak rather than the data race the row asserts. The ledger already
names this cell as load-sensitive, `TSAN-JOIN-EDGE-ROW-LOAD-SENSITIVE`
("misses its ct07-case forced-together cell under concurrent whole-workspace
load, and passes it 20 of 20 in isolation").

This card touches no emission, task route or TSan harness. The same binary
passed the test 5 of 5 times in isolation.

The full-suite exit therefore stays **101**, and it is reported as 101. The
isolated reruns are recorded beside it; they do not replace it. Nothing was
weakened.

**Counts.** This card adds no Rust test function. It adds 13 table rows inside
`loader_abi.rs`' existing admission test, and 13 theorems inside an existing
audit file, so the audit-file count is unchanged at 170. The base's own gate
figures at `f8b41209` were not remeasured here, so no delta is stated by
subtraction.

**Focused runs before the gate**, which are measurements and not gate steps:

* `cargo test -p qcore-c-gateway` passed 1,137 / 0 on the reader change.
* The four re-pinned test binaries passed 10 + 4 + 12 + 14.
* `test_contract_corpus` passed 7 / 7.
* The Lean module and its audit built with exit 0.

## 7. Ledger and status

* **Retired:** `W43-LIBC-RESTRICT-PROTOTYPES-UNDECLARED`. It is preserved
  verbatim with its ground in [retired-followups.json](retired-followups.json).
* **Registered** ([followups.json](followups.json)):
  * `W43-LIBC-GETPEERNAME-SOCKADDR-UNION`;
  * `W43-LIBC-DLSYM-HANDLE-LIFETIME`.
* **Follow-ups 299 → 300.** `corpus.cycle19_raw_units` now points at this
  report's artifacts. Its previous contents are in
  [previous-corpus-row.json](previous-corpus-row.json).
* `docs/STATUS.md`'s corpus, contract-corpus and follow-up rows are updated
  from this arbiter's primary evidence.
* The increment paragraph opens [`wave-43.md`](../../development/increments/wave-43.md).
  `wave-41-continuation.md` is over its 32 KiB cap, and the index now points at
  the new file.
* `W43-LIBC-ABI-READER-SHAPES`, `W43-LIBC-OUT-POINTER-ALIAS` and
  `W43-LIBC-IN-PLACE-LIFECYCLE` stay open. Their `getpeername`,
  pointer-to-pointer and `regcomp` aspects are unchanged by this card.

## 8. For the integrator

* **Constraint 7:** the leak-pin chain can record W43-LIBC-RESTRICT-PROTOTYPES
  as the fifth consecutive declarative arbiter to leave the pin at 272: 10
  declarations and 3 function moves.
* **Constraint 8:** the reader paragraph can record that a qualifier inside an
  array parameter's bound is now read (C11 6.7.6.3p7), and that an array of
  qualified pointers refuses like its pointer spelling. `static`, `[*]`,
  multi-token sizes and a result's trailing qualifier still refuse. This card
  leaves rule text to the integrator, as its predecessors did.
* **Files likely to conflict:**
  * `docs/reviews/r0_3/acceptance_matrix.json`, `docs/STATUS.md`,
    `docs/reports/INDEX.md` and `docs/development/increments/README.md`. Another
    card may also open a new record file. Keep one current record and merge
    the paragraphs newest first.
  * `compiler/tests/qcore_contract_corpus.rs` and
    `compiler/tests/contract_loader.rs`: declaration and count pins.
    W43-LIBC-PRINTF-FAMILY adds `libc/stdio.qcore` pins in the same tables,
    and the counts compose additively.
  * `crates/qcore-c-gateway/tests/{prod_contracts,neighbours}.rs` and
    `scripts/tests/test_contract_corpus.py`: the count pins, only if another
    card adds a module.
  * `providers/contract-load-path.json`: regenerate it with
    `providers/contracts.py sync` after merging, never by hand.
  * `crates/qcore-c-gateway/src/legacy_declared_abi.rs`: `project_parameter`,
    if another card edits the reader.
  * `providers/quirin-provider-libc/contracts/libc/*.qcore`: this card appends
    to `dlfcn`, `socket`, `sys_stat`, `threads` and `time`, and edits comments
    in `process`. It does not touch `stdio.qcore` or `string.qcore`.
* The `contracts` artifact embeds this worktree's path in its source keys, as
  earlier isolated cards' artifacts did.

## Reproduction

```sh
python3 docs/reports/w43-return-expression/scripts/run_audit.py <before> f8b41209        # base binary, saved as measure/base-bin
python3 docs/reports/w43-libc-restrict-prototypes/scripts/identity.py <before> docs/reports/w43-libc-result-alias/artifacts <stem>
bash docs/reports/w43-libc-result-alias/scripts/capture_run.sh <capture-run>            # tee wrapper, base binary
python3 docs/reports/w43-libc-result-alias/scripts/strip.py
python3 docs/reports/w43-return-expression/scripts/run_audit.py <reader> ddca44c0        # card binary, base declarations
sh docs/reports/w43-libc-restrict-prototypes/scripts/census.sh manifest cycle19-resource-contracts.json   # at the base declarations
sh docs/reports/w43-libc-restrict-prototypes/scripts/census.sh new10 <the ten declarations> --
sh docs/reports/w43-libc-restrict-prototypes/scripts/census.sh manifest-after cycle19-resource-contracts.json
python3 docs/reports/w43-libc-restrict-prototypes/scripts/census_summary.py abi-evidence.json manifest new10 manifest-after
python3 docs/reports/w43-return-expression/scripts/run_audit.py <decl> ddca44c0+declarations
python3 docs/reports/w43-return-expression/scripts/run_audit.py <after> fb0bec93
python3 docs/reports/w43-libc-contracts/scripts/unit_texts.py <before>/resource-diagnostics.json <capture>/ppP <unit-texts>
python3 docs/reports/w43-libc-contracts/scripts/attribute.py <reader> <decl> <unit-texts> <attribution.json> <the ten declarations>
python3 docs/reports/w43-libc-restrict-prototypes/scripts/classes.py <before> <reader> <decl> <attribution.json> classes.json
python3 docs/reports/w43-libc-restrict-prototypes/scripts/contracts_diff.py <before> <after>
python3 docs/reports/w43-libc-contracts/scripts/archive.py <after> docs/reports/w43-libc-restrict-prototypes/artifacts
python3 docs/plans/cycle19-custody-cards/blocker_census.py docs/reports/w43-libc-restrict-prototypes/artifacts
python3 docs/reports/w43-return-expression/scripts/sole_callees.py docs/reports/w43-libc-restrict-prototypes/artifacts 30
python3 docs/reports/w43-libc-restrict-prototypes/scripts/update_ledger.py
```

"The ten declarations" is the appended blocks plus `regex.qcore`, concatenated
into one measurement module. It is kept privately as
`measure/drafts/census10.qcore.draft`. The private raw outputs are in this
worktree's `measure/`, under `before/`, `reader/`, `decl/`, `after/`,
`capture-run/` and `capture/` (wrapper captures and plain units), `census/`,
`unit-texts/`, `gate/` and `provider-gate/`.
