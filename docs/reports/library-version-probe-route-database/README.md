# LIBRARY-VERSION-PROBE-ROUTE-DATABASE

Card for the follow-up `LIBRARY-VERSION-PROBE-ROUTE-DATABASE` (registered by
[OPENSSL-PROVIDER-PLUGIN](../openssl-provider-plugin/README.md)), launched by
the user on 2026-11-17. Base `32336c33d`. Rule 53 is amended in place.

## 1. The gap

Rule 53's probe preprocesses `#include <HEADER>` under "every include
configuration of the audited build". At base those configurations came only
from `QCORE_LIBRARY_VERSION_COMPILE_DATABASE`, or else from Clang's default
search path. `quirin-audit current --compile-database`, the configured route
(`quirin-audit CONFIG.json`) and `pin` never passed their own database. A
build that bundles a library and passes `-I` for it therefore had its
version-ranged contract judged against the system headers. The base reader
was also not total. An entry with no `arguments` or `command`, an empty
database, or a dangling `-I` was read as one configuration with no include
flags, which is the default search path. It also split `command` on
whitespace.

## 2. Change

`crates/qcore-audit/src/library_version.rs`:

- `ProbeDatabases::{None, Route(Vec<PathBuf>)}` names the databases a run
  analyses units from. `include_configurations(&ProbeDatabases) ->
  Configurations { source, sets: Result<..> }` resolves them in this order:
  1. The database `QCORE_LIBRARY_VERSION_COMPILE_DATABASE` names. It is an
     explicit override, recorded as `override
     QCORE_LIBRARY_VERSION_COMPILE_DATABASE: compile-database:PATH`.
  2. The run's own databases, recorded as `compile-database:PATH[ + ...]`.
  3. `clang-default-search-path`, used only with no database. This is the
     same string as before.
- The databases are read by the routes' own reader
  (`compile_db::compile_entries`, then `shell_words`). The probe refuses
  rather than guesses in any of these cases:
  - the database cannot be read, or holds no compile command;
  - the reader refuses a record (a relative directory, or no command);
  - an include flag names no directory;
  - a command carries a flag that moves the header search in a way the probe
    does not reproduce (`UNREAD_SEARCH_FLAGS`: `-nostdinc`, `-nostdlibinc`,
    `-nobuiltininc`, `--sysroot`, `-isysroot`, `-iprefix`/`-iwithprefix`,
    `-I-`, `--include-directory`, `-iframework`, `-F`).

  Each of these is an error. `probe_under` turns it into
  `library-version-unreadable` (Unknown). It is never the default search
  path.
- `include_flags` skips the compiler word. Relative directories are joined
  with `Path::join`, so the platform separator is used. No Unix-only path is
  introduced.

Plumbing:

- `contract_revision::apply_pins_probing` takes the databases.
  `apply_pins` keeps its signature and passes `ProbeDatabases::None`, which
  serves the contracts route and the examples.
- The configured route (`lib.rs`) passes its `compile_database`, to the run
  and to every library slice.
- The current route passes its main database, which defaults to
  `<checkout>/build/compile_commands.json`. When Monkey's units are analysed
  standalone it also passes the Monkey database. The route uses the new
  `CurrentAudit::with_probe_databases`, which keeps the databases for
  `activate_slice_contracts`.
- The pinned route passes `build/compile_commands.json` and the Monkey
  database. It states no range, so this is only consistency.
- `pin --compile-database FILE`: without `--derive-results` it is the
  probe's database for a pin, `--regenerate` or `--untrack-tip` of resource
  locks. It is refused with `--bounds`, `--outer`, `--check`, `--revision`,
  `--stamp-only`, `--verify-contracts`, `--track-tip` and `--drop-revision`.
  With `--bounds --derive-results` it keeps its derivation meaning. An
  unreadable database refuses the pin and writes no lock.

Documentation: [rule 53](../../agents/rules/53.md) (then
`gen_rule_docs.py`), [contract specification C06.2](../../QCORE_CONTRACT_SPEC_r0_3.md),
`pin`'s usage text and module documentation.

There is no Lean change. `QCore.LibraryVersionRange.decide` takes the probed
version as `Option Version`. Which configurations feed the probe is a premise
of the Rust side; rule 53 states it and does not prove it. The in-force
decision is unchanged.

## 3. Tests (shipped entry points)

[`crates/qcore-audit/tests/library_version_route_database.rs`](../../../crates/qcore-audit/tests/library_version_route_database.rs)
drives the `quirin-audit` binary.

The fixture has three header sets:

- a "system" set at 2.3.1, a directory on `C_INCLUDE_PATH`, which Clang
  searches after every `-I` on every platform, so the test needs no host
  library;
- a bundled set at 2.5.0, named by `-Ibundled` in the database;
- a second bundled set at 3.0.0 in `other/`.

The checkout is a git repository with two units. The user crate states
`since "2.0"`.

The tests check:

- **pin**: Without the flag the pin records 2.3.1 (the system set). With
  `--compile-database` it records 2.5.0, and the recorded header lies under
  `bundled/`. Each of these refuses and leaves the lock byte-identical: a
  missing database, a `--sysroot` configuration, and two disagreeing
  configurations. `--check`, `--outer` and `--bounds` combined with the flag
  are refused.
- **current**: Pinned from the database, the declaration is in force and the
  call is `Safe`. The record names the database. On the default search path
  the probe would read 2.3.1 and hold the call `Pending`. Two units
  configured `-Ibundled` and `-Iother` give `Unknown` with
  `library-version-ambiguous`. A `--sysroot` command on the other unit gives
  `Unknown` with `library-version-unreadable`. With the variable set to a
  database that has no `-I`, the record starts with `override ...`, the
  probe reads 2.3.1 and the call is `Pending` with `library-version-changed`.
- **configured**: The declaration is in force at 2.5.0. Two disagreeing
  units give `Unknown` with `library-version-ambiguous`.

Unit tests in `library_version.rs` cover the refused flags and the total
reading. The existing `library_version_ranges.rs` (contracts route, variable
as override) passes unchanged.

## 4. Measured: OpenSSL on the prepared Fluent Bit build

Inputs: `quirin-audit prepare clang-build` of Fluent Bit `046accf6` from the
local checkout, written to `private/measure-openssl/fb/`. The production
database has 1412 records and about 50 distinct include-flag lists (Python
count, relative directories not resolved). No record carries an
`UNREAD_SEARCH_FLAGS` flag, and no include directory names OpenSSL: the
build uses the system `/usr/include` headers.

- **Lock unchanged.** The run was `pin --load-path
  providers/user-crates-load-path.json --crate quirin-provider-openssl
  --source-root docs/reports/openssl-provider-plugin/evidence/fixture
  --compile-database <prepared>/compile_commands.json`, after `package.py
  checkers`. It exited 0 in 4.0 s and printed no `= openssl: version pinned`
  line. `providers/quirin-provider-openssl/contracts.lock` is byte-identical:
  sha256 `4a681612e94c…` before and after. The version stays 3.5.5 from
  `/usr/include/openssl/opensslv.h`, with the same digest. The log is
  `private/measure-openssl/pin.log`.
- **Current route record.** `quirin-audit current` (release binary) ran
  over the prepared checkout with `contract/profiles/fluent-bit`, the
  prepared `--compile-database`, the opt-in contract selection
  (`opt_in_providers.py`, the profile's modules plus
  `quirin-provider-openssl`) and a one-unit selection
  (`lib/librdkafka-2.15.0/src/rdcrc32.c`). The version record is computed
  before any unit, over the whole database, so the unit choice does not
  affect it. The run exited 0. The record:
  - `configurations` is `compile-database:<prepared>/compile_commands.json`;
  - the probe read **50** include configurations, all resolving
    `/usr/include/openssl/opensslv.h` (sha256 `7acbe889…`) at **3.5.5**;
  - all 12 declarations are in force, with audited = pinned = 3.5.5 and no
    reason.

  These are the version, header and digest the lock pins, and the version the
  OpenSSL card measured through the variable (49 configurations then, on the
  integrator's preparation). On this build the record changes only in its
  `configurations` source string. Output:
  `private/measure-openssl/current-small/`. A first attempt with the debug
  binary on `rdkafka_ssl.c` was stopped after 54 minutes (CPU time on a
  loaded host). It is discarded, not evidence.

## 5. Routes that do not move (by construction)

The probe runs only when some loaded declaration states a range: `evaluate`
returns before reading any configuration when `library_versions()` is empty.
At base, `resource_library_version` appears only in the seven modules of
`providers/quirin-provider-openssl`. That crate is listed only in
`providers/user-crates-load-path.json`. It is in neither
`providers/contract-load-path.json` nor `cycle19-resource-contracts.json`
(0 matches each). So the corpus, system and slice routes and every report or
lock without a range do no probe and write no record. The only change on
their path is `apply_pins` → `apply_pins_probing`, whose added argument is
read only inside that skipped branch, so their outputs are byte-identical.
The contracts route and the shipped examples keep `ProbeDatabases::None`,
which gives the base string `clang-default-search-path`.

## 6. Gate

Command: `python3 scripts/gate_scope.py --base 32336c33d --out
private/gate-1/gate`, run through `private/toolchain/env.sh` on `b318bd791`.
Record: [`gate/gate-1.json`](gate/gate-1.json). Clang:
[`gate/clang-version.txt`](gate/clang-version.txt). The scope is qcore-audit
and its reverse dependencies (qcore-compiler, qcorec, the libcurl and
OpenSSL audit plugins) plus six test targets. There are no Lean or provider
steps.

| Step | Exit | Counts |
|---|---:|---|
| submodules, fmt, clippy (4 invocations) | 0, 0, 0 | |
| Rust suite (`run_test_binaries.py`) | 101 | 213 units, 216 summaries; 2706 passed, 1 failed, 8 ignored |
| `reevaluation_probe`, `reevaluation_followup` | 0, 0 | |
| Python suite | 0 | 779 tests |

The one failure is
`qcore-compiler::test::cycle20_verifier_plugins::vp04_vp07_vp08_process_lifecycle_and_payload_budgets`
(`assertion failed: elapsed < Duration::from_millis(500)`). It is a
constraint-16 timing row in the runner's `EXCLUSIVE` phase. It failed while
three other cards' Fluent Bit audits loaded the host, and this card touches
no verifier-plugin code. Following constraint 16 it was verified in
isolation, not weakened. That binary alone exited 0: 20 passed, 0 failed
(`private/gate-1/isolation-cycle20-all.log`), and the test alone also exited
0 (`private/gate-1/isolation-cycle20.log`). The gate's overall exit is
therefore 1. The landing gate is the arbiter.

## 7. Follow-ups

Registered in [`integration-amendments.json`](integration-amendments.json),
which also retires `LIBRARY-VERSION-PROBE-ROUTE-DATABASE`:

- `LIBRARY-VERSION-PROBE-CONTRACTS-ROUTE-INCLUDES`: the contracts route takes
  `-I` but has no database, so its probe still uses the default search path.
- `LIBRARY-VERSION-PROBE-UNIT-SCOPE`: the probe reads the whole database. A
  unit that never includes the library can therefore make the version
  ambiguous, which is conservative. The probe also does not reproduce
  `-include`/`-imacros`/`-D`/`--target`, a stated premise.

Concurrent work: LIBCURL-PROVIDER-PLUGIN relies on this probe for its range
and should pass `--compile-database` to `pin` where its build bundles libcurl.
