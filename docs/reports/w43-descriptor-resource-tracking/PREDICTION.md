# W43-DESCRIPTOR-RESOURCE-TRACKING: prediction, frozen before implementation

Written on 2026-09-25 on base `f8b41209`, before any loader, gateway, Lean or
contract change. It is not edited after implementation starts; the report
compares the outcome against it.

## Inventory it rests on

Measured by [`scripts/descriptor_inventory.py`](scripts/descriptor_inventory.py)
over `docs/reports/w43-libc-result-alias/artifacts/diagnostics.json.gz`, and by
[`scripts/source_call_sites.py`](scripts/source_call_sites.py) over the 185
audited units' own source text.

* 19 descriptor/handle callees carry `call to uncontracted function` leaves:
  **80 leaves in 38 functions, 5 sole-blocked** (`open`, `close`, `socket`,
  `dlopen`, `dlclose`, one each). The brief's "`fclose` sole-blocks 6" is
  "blocks 6"; its sole-blocked count is 0.
* Spelled in source but carrying no leaf of their own: `accept` (2), `pipe` (1),
  `fdopen` (1), `fileno` (3, one `nested call` leaf), `mmap` (2), `munmap` (1).
  Every other name in the brief's list (`dup`, `dup2`, `epoll_create1`,
  `eventfd`, `timerfd_create`, `signalfd`, `openat`, `creat`, `mkstemp`,
  `popen`/`pclose`, `socketpair`, `inotify_init1`, …) is spelled nowhere in the
  audited units.

## Design the prediction assumes

* Kind-indexed custody on `ResourceFunctionContract`, heap by default, so every
  existing declaration loads to exactly its present contract.
* A `descriptor` kind carried by an `i32`, whose failure value is `-1`, with
  `sentinel`/`valid` as its nullability words, and `stream`, `directory`,
  `dynamic_library` and `file_tree` pointer kinds.
* Validity guards (`fd < 0`, `fd == -1`, `fd != -1`, `fd >= 0`, `fd > -1`
  exact; `fd > 0` narrows its true edge only) only on names the function binds
  to a descriptor. A null test (`if (fd)`, `if (!fd)`) of a descriptor refines
  nothing.
* A tracked descriptor read by any custody-neutral scalar route (member store,
  aggregate store, the unbound scalar-field no-op, publication as a scalar
  source) is refused, and one passed at a `Borrowed` position of a callee no
  declaration covers is refused.
* W4 never publishes a summary whose result or out slot is a non-heap kind,
  and never validates a heap-kinded summary against a body that releases a
  formal with a non-heap release.

## Class (a): vocabulary and gateway, no new declaration

**Prediction: all five artifacts byte-identical to BEFORE.** No contract
selected at the base carries a kind, so no site is non-heap, no function binds
a descriptor name, and every new rule is inert. Any movement in class (a) is a
defect of this card, not a finding.

## Class (b): declarations

Planned declarations, all in `providers/quirin-provider-libc`: `open`
(variadic tail), `socket`, `close`, `fopen`, `fclose`, `fileno` (neutral: its
result is a borrowed number), `opendir`, `closedir`, `dlopen`, `dlclose`,
`fts_open`, `fts_close`, and `accept4` if the ABI gate reads its prototype.
Refused by name: `getaddrinfo`/`freeaddrinfo` (the out list is valid on a zero
status, which needs a status witness), `fcntl` (one premise per declaration,
and the corpus spells five commands), `glob`/`globfree` (in-place lifecycle of
a caller object plus a callback), `pipe` (an array out slot), `fdopen`
(the descriptor moves into the stream on success only), `accept`, `mmap`/
`munmap` (pointer sentinel `MAP_FAILED`), `regcomp`/`regfree` (in-place,
already registered).

Predicted per-function movement:

* **Unknown → Safe: 0.** Every function naming a planned callee keeps another
  blocker: `mk_socket_create` (`mk_utils_libc_error`), `rd_kafka_open_cb_generic`
  (`fcntl`), `rd_kafka_socket_cb_generic` (`fcntl`, `fprintf`),
  `rd_kafka_open_cb_linux` and `rd_kafka_socket_cb_linux` (a returned result
  that may be the sentinel, under a contract-free exit that may not return
  one), `mk_plugin_load_dynamic` (returns a possibly-null handle),
  `net_plain_close` and `rd_dl_close` (release an untracked formal),
  `mk_socket_accept` (`sizeof`).
* **Unknown → provisional Unsafe: 1**, `ctr_random_get`. Its guard is
  `if (fd > 0)`, whose false edge includes descriptor `0`, so the `open` site
  stays live on the fallback path to `return ret;`: one new `leak` leaf
  (**272 → 273**). Witness status: reachable only if `open` returns descriptor
  0, i.e. with standard input closed. A candidate, not an upstream defect.
* **Leaf movement.** Of the 80 descriptor leaves, the ones at planned callees
  (about 60) leave `call to uncontracted function`; most become an
  `unknown_alias` on an untracked actual (a formal, a global such as `stderr`,
  or a result of an uncontracted wrapper) or vanish where the actual is tracked.
  `unsupported` falls by roughly 50–65 and `unknown_alias` rises by roughly
  20–35.
* **No `invalid_release` or `use_after_release` leaf** in the corpus: no
  audited function hands a handle of one kind to another kind's release.
* **No new scalar-route refusal** in the corpus: the only tracked descriptors
  are locals of the functions above, none of which stores one into a member.

## Class (c): wrappers

Monkey and librdkafka wrap `close` only as macros (`rd_socket_close`), and
Fluent Bit's `flb_socket_close` is a macro; `flb_pipe_close` accepts the
sentinel, so it is not a forwarding wrapper. Fluent Bit core units are not in
the corpus. **Prediction: no wrapper is declared; candidates are reported.**

## Class (d): violation candidates

The only candidate predicted is the `ctr_random_get` leak. No double close, no
use after close, no kind-mismatched release in the corpus.

## Totals predicted

Functions 257 / 84 / 4,493 → **257 / 85 / 4,492** (Safe / provisional Unsafe /
Unknown). Units unchanged (`ctr_random.c` is already Unknown and would become
provisional Unsafe only if its unit status follows the function: predicted
**one unit move**, Unknown → provisional Unsafe). Leak 272 → 273.
