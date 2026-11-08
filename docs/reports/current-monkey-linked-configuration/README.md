# CURRENT-MONKEY-LINKED-CONFIGURATION

Card base `a0ac2cae9` (FLUENT-BIT-LINK-PREPARE's tip). Brief: round 29.
It discharges `CURRENT-MONKEY-CONFIGURATION-NOT-LINKED`. User decision
(2026-11-08, verbatim): "Analyse linked config". The `current` route is to
analyse `lib/monkey` under Fluent Bit's own configuration, the one linked
into `bin/fluent-bit`, so that its homes can be derived. Before this card it
kept the 24 Monkey homes refused.

This commit holds the census (scope item 1). Later commits add the route
change, the checks and the records.

## Census

[`scripts/census.py`](scripts/census.py) reads the user's preparation at
Fluent Bit 046accf6 and writes nothing there. Output:
[`census.json`](census.json). Both databases are read:

- **standalone**: `monkey/monkey-build/compile_commands.json`, SHA-256
  `8c6779d0…61658075`. This is the forced-Libevent copy that the route reads
  through `--monkey-database`.
- **linked**: the `lib/monkey` entries of
  `fluent-bit-clang-production/compile_commands.json`, SHA-256
  `93e9e67e…ff99583`. This is the run's `--compile-database`, and its
  objects are what `bin/fluent-bit` links (FLUENT-BIT-LINK-PREPARE).

A unit is a Monkey unit by the route's own rule (`is_monkey_unit`): a `.c`
file under `mk_core/` but not `mk_core/deps/`, under `mk_server/`, or under
`tls/`. Each database has exactly one configuration per file. The census
preprocesses each command with Clang (`-E`, output under the worktree's
`private/`) and compares the results after rewriting each tree's roots. For
every top-level function definition in the unit's own text, or in a `.c`
file the unit includes textually (the event backend), it records whether the
preprocessed tokens are equal in both configurations, differ, or exist in
only one.

| Unit | Databases | Unit's functions equal / differ / only in one | What differs |
|---|---|---|---|
| `mk_core/mk_event.c` | both | 15 / 11 / 6 standalone-only | event backend: epoll (`mk_event_epoll.c`) when linked, Libevent (`mk_event_libevent.c` and the bundled `event2/` headers) when standalone. The 11 backend statics `_mk_event_{add,backend,channel_create,channel_destroy,del,inject,loop_create,loop_destroy,timeout_create,timeout_destroy,wait_2}` differ. `_mk_event_update`, `_mk_event_wait_with_flags`, `cb_event`, `cb_timeout`, `cb_wait_2_timeout` and `mk_event_libevent_socketpair` exist only in the standalone configuration |
| `mk_server/mk_lib.c` | both | 24 / 2 / 0 | `mk_lib_worker` and `mk_start` expand `mk_event_foreach` over the backend's `struct mk_event_ctx` (`events[i].data.ptr` for epoll, `fired[i].data` for Libevent) |
| `mk_server/mk_server.c` | both | 10 / 2 / 0 | `mk_server_loop_balancer` and `mk_server_worker_loop`: the same `mk_event_foreach` expansion |
| `mk_core/mk_file.c`, `mk_server/{mk_cache,mk_clock,mk_config,mk_fifo,mk_header,mk_http,mk_http_parser,mk_http_thread,mk_kernel,mk_mimetype,mk_net,mk_plugin,mk_scheduler,mk_socket,mk_stream,mk_user,mk_utils,mk_vhost,monkey}.c` (20 units) | both | every function equal (237 in all) | declarations only: the backend header (`mk_event_epoll.h` and `<sys/epoll.h>`, or `mk_event_libevent.h` and `event2/`) and therefore the layout of `struct mk_event_ctx`; no function text |
| `mk_core/{mk_iov,mk_memory,mk_rconf,mk_string,mk_utils}.c` (5 units) | both | every function equal (54 in all) | nothing: the whole preprocessed unit is token-equal |
| `tls/tls_openssl.c` | linked only | 27 linked-only | compiled under `MK_HAVE_TLS` (OpenSSL); it defines the `mk_tls_*` API |
| `tls/tls_stub.c` | standalone only | 5 standalone-only | the `MK_TLS=OFF` stub of `mk_tls_{enabled,exit,init,thread_init,transport}`. The linked configuration does not compile it: `tls/tls_openssl.c` defines those names there |

Totals: 30 files. 28 are in both databases, 1 is only linked and 1 is only
standalone. 5 of the 28 are whole-unit token-equal. 25 have equal text for
every unit function, and 3 compile different function text. All differences
come from the event backend and the TLS choice. No header inline function
differs in any unit.

How the commands differ, apart from paths:

- **Macros.** Every linked command adds Fluent Bit's global definitions
  (`FLB_*`, `JSMN_*`, `MPACK_EXTENSIONS=1`, `MSGPACK_EMBED_STACK_SIZE=64`,
  `NGHTTP2_STATICLIB`, `__FLB_FILENAME__=__FILE__`) and `MK_HAVE_REGEX` and
  `MK_HAVE_TLS`. The standalone `mk_core` commands add
  `MK_EVENT_LOOP_LIBEVENT`. The standalone `mk_server` units get it from the
  generated `mk_core_info.h`, which defines it. Both have the same
  `MK_HAVE_*` feature probes otherwise (accept4, backtrace, eventfd, memmem,
  memrchr, stat.h, sys/uio.h, timerfd_create, unistd.h, valgrind) and
  `MK_THREADS_POSIX`.
- **Includes.** The linked commands add Fluent Bit's include path, which
  names no Monkey header. The standalone commands add the bundled
  Libevent's `include/` (source and build). Monkey's generated headers
  (`mk_info.h`, `mk_env.h`, `mk_core_info.h`) come from each build's own
  `include/monkey`.
- **Other words.** Linked commands add a second `-Wall`, `-fPIC` and
  `-Wa,--noexecstack`. Standalone commands use `-fPIC` in `mk_server/` only
  and never `--noexecstack`. Both end with `-g -std=gnu17` (after
  `-std=gnu99`), and `-Wextra` is in both.
- **Plugins and the rest.** Neither configuration has a plugin among the
  route's units, because `plugins/` is outside the Monkey unit rule. Both
  databases also compile `api/` and `deps/{flb_libco,rbtree,regex}`. The
  linked one adds `plugins/{dirlisting,mandril}` and `mk_bin/`. The
  standalone one adds the bundled Libevent (`mk_core/deps/libevent`, 28 files
  in 4 configurations each). None of these is a Monkey unit of the route in
  either configuration (follow-up `MONKEY-LINKED-NON-ROUTE-UNITS`).

## Decision: replace, with every standalone unit accounted for

The brief allowed two shapes, replace or add alongside, and asked for a
stop if fail-closed and the denominator rule did not settle the choice. They
settle it as follows.

- **Alongside defeats the decision.** If the run carried both
  configurations, every Monkey function would be defined in two route units
  (`monkey-libevent/lib/monkey/<f>` and `monkey/lib/monkey/<f>`). Rule 4c's
  selection (`LinkMap::select` in `cross_unit_links.rs`) imports a home only
  when exactly one route unit defines the name. So every Monkey home would
  stay refused, which is what the user decided against. Changing that census
  is LINK-STANDING's code, which this card must leave alone.
- **The standalone configuration is not a program.** It is a forced-Libevent,
  `MK_WITHOUT_BIN`, `MK_TLS=OFF` static library that nothing links. Its
  readings describe code that `fluent-bit` does not run (the census: the
  Libevent backend, `tls_stub.c`). The Monkey contracts the run loads were
  reviewed against the epoll backend, which is the linked configuration
  ("Contract and profile checks" below).
- **The denominator rule is kept by accounting, not by analysis.** Every unit
  of the standalone database still has a row or a recorded replacement:
  - 28 units have a linked counterpart (the same file). Each is replaced by
    `monkey/lib/monkey/<f>`, analysed under the linked command. The
    replacement is listed in the report's assumptions.
  - `tls/tls_stub.c` has none. It stays as
    `monkey-libevent/lib/monkey/tls/tls_stub.c`, an `Unknown` unit with
    reason `not linked: …` (`environment-unavailable`). Its definitions are
    `not_projected`. It is never analysed under the standalone command.
  - `tls/tls_openssl.c` is linked only and is a new unit.

  Unit keys stay unambiguous. `monkey/…` names only the linked configuration
  and `monkey-libevent/…` only the standalone one. The link step keys the
  same `monkey/…` units, so condition 11 holds.

## What changed

- `quirin-audit current --monkey-linked` (or `"monkey_linked": true` in
  `--config`; both forms must agree, and the flag may not repeat).
  `linked_monkey_configurations` (`current_route.rs`) selects the run's own
  compile database records under `<checkout>/lib/monkey` by the unchanged
  Monkey unit rule (`is_monkey_unit`), refusing aliased spellings. The group
  is labelled `monkey` (`LINKED_MONKEY_LABEL`) and placed after the profile's
  or the selection's groups.
  - With `--monkey-database`, that database is read only to account for its
    units, through `monkey_source_root`, which still requires
    `-DMK_EVENT_LOOP_LIBEVENT`. Units without a counterpart become
    `TaskInput::NotLinked` tasks (`CurrentAudit::evaluate_not_linked`, the
    absent-unit evaluation with its own reason) in a trailing
    `monkey-libevent` group. The member pre-pass and rule 31's reach then
    read the compile database alone, because the standalone units are not
    part of the run.
  - The run refuses when the database has no Monkey unit, when a group
    already uses the `monkey` or `monkey-libevent` label, or when a group
    already selects a Monkey unit (one unit, one configuration).
  - The report's assumptions (`.contracts.json`) record
    `monkey_configuration`: the analysed units, the standalone database, the
    replacements and the not-linked units. The invocation records
    `"monkey_linked": true`. Both are written only when the flag is given, so
    every other run is byte for byte as before.
  - In the cross-unit export phase, a not-linked task counts as scanned with
    no export and no definition. The link cannot choose a definition from a
    unit that is not linked, so it does not make the definition census
    incomplete. An absent unit (no record, not known to be unlinked) still
    does.
- `prepare --run` passes `--monkey-linked` with the prepared Monkey
  database.
- The link step (`link_resolution.rs`, `route_units`) gives each linked
  Monkey configuration its route key `monkey/lib/monkey/<f>`. It refuses a
  configuration that would receive two keys. The artifact gains a
  `route_selection` record, and the candidate map's premise says that a
  `monkey/` home is a unit only of a `--monkey-linked` run. Nothing else in
  the step changes. The derivation conditions are those of
  FLUENT-BIT-LINK-PREPARE, applied to a selection that now includes Monkey.
- Not changed: `pinned_route.rs`; the Monkey unit rule; profiles,
  `audit.json`, selections, contracts, providers and locks.

## Contract and profile checks

Each input keyed to the standalone configuration was checked against the
linked one. Nothing was re-pinned.

| Input | Keyed to | Linked run | Outcome |
|---|---|---|---|
| Provider pins (`qcore-provider-libevent/contracts.lock`, 54 Monkey sites; others) | source text | unchanged | A pin's identity is the token-normalised text of every site in the checkout. The scanner never preprocesses ("a site guarded by `#if` counts whatever the configuration", `c_identity.rs`). The linked run reads the same checkout, so every pin stands or is withheld exactly as before. |
| `monkey/event_loop.qv` (`mk_event_*`) | reviewed on the epoll backend, "the analysed configuration" | matches | The review read `mk_event_epoll.c`, which the linked configuration compiles. The standalone run analysed `mk_event.c` with the Libevent backend under a contract reviewed on epoll. The linked run removes that mismatch. |
| `monkey/result_range.qv` (`mk_event_channel_create`) | epoll chain; the lock records every backend's site | matches | Same as above. |
| `monkey/{intrusive_list,memory,string,vhost_set,logging}.qv` | bodies in `mk_fifo.c`, `mk_http_thread.c`, header inlines, `mk_string.c`, `mk_vhost.c`, `mk_utils.c` | matches | The census shows equal text for these bodies in both configurations. `logging.qv`'s comment names `monkey-libevent/mk_core/mk_utils.c` as the audited home of `mk_print`. The linked unit `monkey/lib/monkey/mk_core/mk_utils.c` is token-equal, and the comment is not machine-read. |
| `monkey/scalar_observation.qv` (`event_base_get_features`, `event_get_struct_event_size`, `event_get_version_number`) | the bundled Libevent | unused | No Monkey unit calls them in either configuration. They stay in force and are unused. They are neither refused nor re-pinned. |
| `monkey/wrapper_paths.qv`, `wait_*_policy.qv`, `composition.qv` (Libevent adapter, `requires selected_unix_libevent_source_profile`) | the standalone configuration | not loaded | Not in the user's manifest (`cycle19-resource-contracts.json`). They are read by `quirin-audit pinned` only. |
| Profile `monkey` library (`typed_import`: the Libevent wrappers `cb_event`, `mk_event_libevent_socketpair`, `_mk_event_*`) | the standalone configuration | not read | The current route does not read a library's `typed_import`. The linked label `monkey` is a report label only. The census shows that 6 of these functions do not exist in the linked configuration, so a later reader of `typed_import` for linked units would have to refuse them. |
| `contract/profiles/fluent-bit/library-providers.json` (`monkey-libevent` → `qcore-provider-libevent`) | the standalone label | refused | A linked run under per-library provider scoping resolves the `monkey` library, which the table does not scope, so the run refuses instead of reusing the Libevent scope. The user's run does not use the table. Follow-up `LIBRARY-PROVIDERS-LINKED-MONKEY-SCOPE`. |
| S3 lifetime protocols | source-, configuration- and function-qualified identities, rederived per run | rederived | No static record names a current-route Monkey unit. The corpus's S3 artifacts (`fb-event-loop-lifetime-s3`) are pinned-corpus evidence. |
| Eight-home premise and contributions 34/35 (`monkey-libevent/…` homes and rows) | the corpus profile | out of scope | They are pinned-route inputs. The corpus does not read `--monkey-linked`. |

Pinned routes: the corpus (`quirin-audit pinned`) has its own Monkey
selection and is untouched. System and slice read explicit selections with
no Monkey database. None reads this selection.

## The linked units match the link artifact (scope item 4)

[`scripts/run_link_prepare.sh`](scripts/run_link_prepare.sh) runs
FLUENT-BIT-LINK-PREPARE's recipe with the tip binary (`e5925e6b0`, SHA-256
`df45f98e…b390df02`): `prepare clang-build --link --build-jobs 6` of Fluent
Bit 046accf6, twice on one fresh private output, under one audit slot
(4 GiB). Both passes exited 0, in 2 min 33 s and 1 min 59 s, with peak RSS
272 MiB. The second configure's database (`4b7b4589…dae74910`) equals the
user's (`93e9e67e…ff99583`) command for command after the path prefix
(FLUENT-BIT-LINK-PREPARE's `compare_database.py`: 1,419 of 1,419
identical). Both passes give the same decisions (`same_decisions.py`). The
records below come from the second pass. The executable and map digests
differ from FLUENT-BIT-LINK-PREPARE's because the output path is compiled
into the `-g` objects.

| Fact | Value |
|---|---|
| Configurations / linked / reproduced byte for byte | 1,419 / 992 / 992 (0 not reproduced) |
| Function symbols referenced across linked inputs | 6,417 |
| **Derived** | **1,557** (base map 1,384) |
| Refused: home not a unit of the current route | 4,113 (base 4,286) |
| Refused: shared object / no unique database entry / ambiguous key | 704 / 17 / 26 (unchanged) |

[`scripts/compare_maps.py`](scripts/compare_maps.py) compares this map with
the base map committed by FLUENT-BIT-LINK-PREPARE (same user database). The
only change is 173 names going from "not a unit of the current route" to
derived, all under `monkey/`. No other name changes decision or home
([`map-comparison.json`](map-comparison.json)).

[`scripts/match_link.py`](scripts/match_link.py) writes
[`link-match.json`](link-match.json). The artifact keys 29 configurations as
`monkey/` route units. For each of the 29, all of the following hold:

- it is linked, its object was reproduced byte for byte (`equal`), and it is
  the configuration of exactly one linked input;
- the user's database has exactly one record of the same file;
- that record's command, with the user's path prefix rewritten to the run's,
  has the artifact's `command_sha256`, and its directory is the artifact's;
- the source bytes at the user's checkout equal those of the run's clone.

So every unit that the user's `--monkey-linked` run analyses has exactly the
compile input whose object is linked into `fluent-bit`. LINK-STANDING's
conditions can therefore derive Monkey homes with no special case. The
artifact binds its own database digest. On the user's run, the comparison
(after the prefix) is LINK-STANDING's database-identity check.

Census coverage ([`coverage.json`](coverage.json), FLUENT-BIT-LINK-PREPARE's
`coverage.py` on this map):

| Census cross-unit population | Callees | Sites |
|---|---:|---:|
| Derived, home equal to the experiment map's | 132 | 383 |
| Derived, Monkey home (previously refused) | 24 | 55 |
| Refused | 0 | 0 |

All 438 sites now have a caller whose route configuration is linked (the 55
Monkey sites' callers are now `monkey/` units). `coverage.py` labels the 24
Monkey homes "home differs from the experiment map" because the experiment
named them under `monkey-libevent/`. Of the 24, 20 are the same file under
the linked label. Their bodies have equal text in both configurations, but
`mk_event_init` calls the backend's `_mk_event_init`, which is now epoll's.
The other 4 (`mk_tls_exit`, `mk_tls_init`,
`mk_tls_thread_init`, `mk_tls_transport`) are a **different body**: the
experiment's home was `tls/tls_stub.c`, and the linked home is
`tls/tls_openssl.c`, the OpenSSL TLS layer that the linked configuration
compiles under `MK_HAVE_TLS`. A summary imported for them describes the
OpenSSL code. The two sole-blocked Monkey functions now have their blocker
derived: `mk_mq_send` (`mk_fifo_send` →
`monkey/lib/monkey/mk_server/mk_fifo.c`) and `net_plain_writev`
(`mk_iov_send` → `monkey/lib/monkey/mk_core/mk_iov.c`). Both homes are
token-equal in the two configurations. So every one of the 7 sole-blocked
functions has every blocker derived.

| Record (second pass) | SHA-256 |
|---|---|
| Compile database | `4b7b458916257c5cdd3f0f3709da5ab938af0ef475ae40f5fdb7514adae74910` |
| Executable `bin/fluent-bit` | `de736fa76c4de90c2f196ed238e385e7e41f103a589140eabd060e4d5dc61a34` |
| Link map (`--cref`) | `2eb17eac6fae6f2aea47719d306a6c3aaa8b933c648237ef4cca109f455a714c` |
| Artifact `link-resolution.json` (kept under the worktree's `private/link-prepare/`) | `219bbadf9c7bba975fd7890ff1f58e10dc0dd88767bb6c6de497d73f82b5a339` |
| `link-map.candidate.json` | `771455d6c20195c11a5295a8ce7e60dab2591a1615f76e3b09189bf17600cdd5` |
| `link-map.candidate.refusals.json` | `e3647ce675f208ef4ffdacb923b0b7ffc302fb45912cbd956a560f44790a71b3` |
| [`link-map.candidate.json.gz`](link-map.candidate.json.gz) | `edbde6c338ec5b60233135ccbb5d51b47dfa88288827ae805b0b8dacc3a1cb2c` |
| [`link-map.candidate.refusals.json.gz`](link-map.candidate.refusals.json.gz) | `ebabfba3ed2b7a47a6c589441b264d71a3b3c05fe196e3fc200e823272d1d919` |

TBD-PROBES

## Tests and Lean

- `a_linked_monkey_run_replaces_or_keeps_every_standalone_unit`
  (`tests/current_cli.rs`), a real Clang run. The linked units are keyed
  `monkey/…`. `mk_core/deps/` and `mk_bin/` are excluded. A shared file is
  replaced and leaves no `monkey-libevent` row. A standalone-only file is a
  not-linked `Unknown` unit with its definitions `not_projected`. The
  replaced unit is analysed like any unit. The assumptions and invocation
  records are as described above. A run without a standalone database
  accounts for nothing. The standalone-only route is unchanged and records
  nothing. Refusals: no Monkey unit in the database, a repeated flag, a
  configuration that disagrees, a group labelled `monkey`, and a group that
  already selects a Monkey unit.
- `route_units_key_linked_monkey_units_like_a_linked_run`
  (`link_resolution.rs`): the link step keys exactly the linked Monkey
  units, under the same keys.
- Lean `QCore.MonkeyLinkedSelection` with `AuditMonkeyLinkedSelection.lean`
  models the accounting. It proves the following: nothing vanishes (every
  standalone file is replaced or not-linked, and every linked file is
  analysed); no file is read twice; the standalone configuration is never
  analysed; the analysed set does not depend on the standalone database;
  keys are pairwise distinct when each database lists a file once; a
  not-linked row is `Unknown`, never `Safe`, defines nothing and never makes
  the census incomplete, while an unscanned analysed row does. Two vectors
  cover the test fixture and the user's census shape. File selection and
  the compile database's faithfulness are trust-boundary-28 premises,
  pinned by the two tests.

TBD-GATE

## Follow-ups

TBD-FOLLOWUPS

