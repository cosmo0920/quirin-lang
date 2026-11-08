# FLUENT-BIT-LINK-PREPARE

Card base `1e522ca5d` (origin/main). Brief: round 27. This card is the
prepare step that the user's decision on BODY-SUMMARY-REUSE-LINK-STANDING
("Option A (rec.)", 2026-11-06) names as its second prerequisite. Under that
decision the cross-unit link resolution is derived from the linked Fluent Bit
executable and fed to rule 4c unchanged. The card changes inputs and evidence
only. No analyzer, gateway, contract, provider, profile or Lean file changes,
and nothing reads what it writes. [PREDICTION.md](PREDICTION.md) is `none` on
every route.

## Where the inputs come from

- **`current` route** (Fluent Bit 046accf6): `quirin-audit prepare`
  (`crates/qcore-audit/src/prepare.rs`) writes
  `build/cycle19/clang/<commit>/`. It clones the checkout, configures CMake
  with Clang and `-std=gnu17` (`FLB_DEV`, `FLB_EXAMPLES`), and builds only
  `cmetrics-static` and the generated-input targets. The route reads that
  tree's `compile_commands.json` and the separate Monkey database
  (`monkey/monkey-build`, a libevent configuration of a copy of `lib/monkey`).
  Before this card nothing was linked: the user's production tree has only
  `buildvm` and `minilua` in `bin/`.
- **Pinned corpus, system and slice** (`crates/quirin-audit-plugin-measure/inputs/live.json`):
  the corpus reads `build/cycle19/c-gateway-wave-pin/` at 66910c10. System
  and slice read earlier private preparations
  (`private/w45-fb/w46-external-callee-contracts/attempt-1`,
  `private/w47-tail/attempt-1/prep`), pinned by database digest. None of
  them is linked, and this card relinks none of them. They are frozen
  measurement inputs, and a relink would need its own pinned preparation
  (follow-up `LINK-PREPARE-PINNED-ROUTE-INPUTS`).

## What changed

`quirin-audit prepare [clang-build] … --link [--build-jobs N]` (new,
explicit; default off). Without `--link`, prepare writes the same files as
at base. With it, after the existing steps,
[`link_resolution.rs`](../../../crates/qcore-audit/src/link_resolution.rs)
does the following:

1. It builds the `fluent-bit-bin` target in **the same build tree** whose
   database the route reads (`cmake --build BUILD --target fluent-bit-bin
   -jN`). A tracked source file that changes during the build stops the step.
2. It reads CMake's link command
   (`src/CMakeFiles/fluent-bit-bin.dir/link.txt`, the Makefile generator).
   The step stops on any flag whose effect on resolution it does not settle:
   `-flto`, `--version-script`, `--wrap`, `--defsym`, `muldefs` /
   `--allow-multiple-definition`, `-shared`, `-Bsymbolic`, `--dynamic-list`,
   `-fuse-ld`, `--just-symbols`. The driver's own `-###` names the linker,
   and the step requires GNU ld.
3. It runs the same link again into a side path, with `-Wl,-Map=…
   -Wl,--cref -Wl,--no-demangle` and `LC_ALL=C`. The side output must equal
   the built executable **byte for byte**, or the step stops. So the map
   describes the executable that was built.
4. It reads the map: the `LOAD` inputs, the archive members included, every
   `.text*` input section's address range, and the cross-reference table
   (for each global symbol, the definition's file, then the referencing
   files). It cross-checks the map against an independent reading of ELF
   symbol tables (a minimal ELF64 and `ar` reader, no external tool):
   - every linked object's symbol table: definitions and references by
     binding;
   - the executable's `.symtab`, `.dynsym`, `.dynamic` (`DF_1_PIE`,
     `DT_NEEDED`) and the dynamic relocations against named symbols.

   An archive member that the memory map or the table names but that was not
   among the included members stops the step. LLVM bitcode inputs (LTO) stop
   it too.
5. It checks each linked object against its database entry instead of
   assuming they match. Each linked input is matched to a compile-database
   record by digest (`output`). The record's own command is then run again,
   with only its `-o` redirected and `PWD` set to the record's directory as
   `make` sets it. The result must reproduce the object byte for byte. A
   mismatch is a recorded refusal of that unit (`home object is not
   reproduced by its compile database entry`).

It writes `link-resolution/` beside the build:

- `link-resolution.json` (`quirin-link-resolution/1`): the artifact. It holds
  - the executable's path, digest and image facts;
  - the link command and `link.txt` digest, the map command and map digest;
  - the toolchain: driver and linker paths, resolved paths, SHA-256 and
    versions;
  - the compile database's digest;
  - every linked input with its digest and configuration;
  - every database configuration: command digest, object, linked or not,
    reproduction result and `current` route unit key;
  - one record per function symbol referenced across linked inputs: the
    definition's file, input, configuration, binding, visibility and type;
    every linked definition; the referencing inputs; the executable entries;
    whether the address lies in the definer's text, is exported, or carries
    dynamic relocations; the decision.
- `link-map.candidate.json`: the candidate map **in rule 4c's existing
  format** (`schema_version`, `premise`, `symbols[{function, home_unit}]`;
  `cross_unit_links.rs` loads it unchanged). The premise names the
  executable, database and artifact digests and says what is not
  established.
- `link-map.candidate.refusals.json` (`quirin-link-map-refusals/1`): every
  other referenced function symbol, with its reason and detail.
- `fluent-bit.map` and `link-build.log`.

`prepare` (without a step) adds a `link_resolution` record (paths, digests,
counts) to `prepare.json`.

### Why the reading is complete

The cross-reference table lists every global symbol the link saw, with the
file holding the chosen definition first. The step reads it under the
following checks:

- the map is verified to describe the built executable (step 3);
- every referenced or defining file it names is a parsed linked input
  (step 4);
- each symbol's definition and references are recomputed from the inputs'
  own symbol tables and must agree: the table's first file defines the name,
  its other linked files are exactly the inputs whose tables reference it,
  and an executable symbol of the name lies in that definer's text. A
  disagreement refuses the symbol (`link map and symbol tables disagree`).

Locals (statics) never cross a unit and are not listed. Versioned shared
names (`name@VERSION`, 12,305) are the shared objects' version aliases and
are skipped. The unversioned name of the same function is classified.

### Derivation rule

A function symbol gets a derived home only when every condition below holds
(checked in this order; the first that fails is the refusal):

1. the image is an executable (`ET_EXEC`, or `ET_DYN` with `DF_1_PIE`), not
   a shared object;
2. the definition is in the executable, not in a shared object;
3. the map and the symbol tables agree;
4. the defining object matches exactly one database configuration;
5. it is not an indirect function;
6. the definer's binding is strong global (not weak, common or unique);
7. it is the only definition among all linked objects;
8. no dynamic relocation in the executable names it (the executable never
   resolves it at run time);
9. the defining object is reproduced by its database entry;
10. that configuration is a unit the `current` route selects (`audit.json`'s
    selections over the profile's source roots, by the route's own
    `select_configurations`);
11. the route's unit key (`library/file`) names that one configuration.

The home is that key, exactly as the route spells `home_unit`. Hidden
visibility is recorded and is no refusal: it removes the export, and the
static link still chooses the one definition. Archive order does not decide
any derived name: condition 7 counts every linked definition, and rule 4c's
own census refuses a name defined in more than one route unit. No
LTO, version script, `--wrap` or `-Bsymbolic` occurs in this link. The step
would stop on one, so none of the brief's stop conditions arose.

## The run

[`scripts/run_link_prepare.sh`](scripts/run_link_prepare.sh): `quirin-audit
prepare clang-build --fluent-bit ~/…/fluent-bit --revision 046accf6… --clang
/usr/lib/llvm-21/bin/clang --clangxx …/clang++ --link --build-jobs 6` into
`private/link-prepare/run-1`, through `gate_slot.py audit --memory-budget 4G`.
Exit 0. The command ran 3 min 17 s (417 s user CPU). The `fluent-bit-bin`
build took 77.6 s after configure (45 s), and reproducing the 992 objects
took 43.5 s. Receipt peak 832 MB for the process tree. The slot wait before
admission was 5 h 8 min (follow-up `GATE-SLOT-SHORT-BUILD-WAIT`).

| Fact | Value |
|---|---|
| Executable | `bin/fluent-bit`, PIE (`ET_DYN`, `DF_1_PIE`), 45.8 MB, linked by `clang++` with `-rdynamic` |
| Executable SHA-256 | `e8990075ef89ee8502420571a7a18acbc366504c9991b71766198768171d0682` |
| Linker | GNU ld 2.46 (`/usr/bin/x86_64-linux-gnu-ld.bfd`, SHA-256 `97f48d93…ef6075`) |
| Driver | Ubuntu clang 21.1.8 (`/usr/lib/llvm-21/bin/clang`, SHA-256 `412bbe8c…ae8e932`) |
| Side relink equal to the executable | yes |
| Linked inputs | 1,006: 992 database objects (one configuration each), 5 C runtime start files, 9 `libbacktrace.a` members (an external project, no database entry) |
| Database configurations | 1,419; 992 linked; **992 reproduced byte for byte, 0 not** |
| Shared objects | 14 (libc, libm, libssl, libcrypto, libcurl, libyaml, libsystemd, libsasl2, libz, libzstd, libstdc++, libgcc_s, libmvec, ld.so) |
| Cross-reference symbols | 30,671; 12,305 versioned aliases skipped |
| Function symbols referenced across linked inputs | 6,417 |

| Decision | Symbols |
|---|---:|
| **derived** | **1,384** |
| refused: home configuration is not a unit of the current route | 4,286 |
| refused: defined in a shared object | 704 |
| refused: home unit key names more than one route configuration | 26 |
| refused: defining object has no unique compile database entry | 17 |
| refused: weak, duplicate, indirect, interposable, disagreeing, not reproduced | 0 |

The 4,286 non-route homes are Fluent Bit core, plugins, LuaJIT, Monkey's
fluent-bit configuration and the other bundled libraries, which the
`current` route does not analyse. The 26 ambiguous ones are all in
`fluent-bit-chunk/src/flb_input_chunk.c`: the route selects its static and
its shared configuration under one key, and only the static one is linked.
The 17 are `libbacktrace` functions. No weak, duplicate or interposable
definition is referenced across inputs. In this PIE, linked with
`--export-dynamic`, the executable carries 879 dynamic relocations against
named symbols. Each of the 704 shared-object functions has at least one.
None of the 5,713 functions defined in the executable has any, so the
static link binds every call to them. Every derived definition has default
visibility.

## The configuration the user's run reads

[`scripts/compare_database.py`](scripts/compare_database.py) compared
`run-1`'s database (SHA-256 `8cc891f0…829251f`) with the user's
(`93e9e67e…ff99583`, the run's `compile_database_sha256`). Both have the
same 1,419 (file, output, directory) configurations, but no command is
identical word for word:

- 714 differ only in word order;
- 667 lack the user's `-I…/lib/yyjson-0.12.0/src`;
- 38 lack both that include and `-DMK_HAVE_REGEX`.

[`scripts/configure_twice.sh`](scripts/configure_twice.sh) explains this.
Configuring a fresh tree with prepare's exact arguments gives `run-1`'s
database. Configuring the same tree a second time gives **exactly the
user's database**: all 1,419 commands are identical after the path prefix
is rewritten. The user's tree was prepared twice, and Fluent Bit's CMake
configuration does not converge in one pass. So `prepare` is not
idempotent: a fresh prepare and a reused one emit different databases for
one revision (follow-up `PREPARE-CONFIGURE-CONVERGENCE`).

The artifact binds the database digest, so an artifact never stands for
another database. To link exactly the user's configuration, the card ran
the same command again on the same output (`run-1-second`). That reuses the
tree: prepare configures it a second time, `make` rebuilds the objects
whose flags changed, and the step links and reads again.
Exit 0 after 2 min 13 s (374 s user CPU; slot wait 2 h 30 min). The second
configure took 4.9 s, the rebuild and link 74.2 s, and the reproduction
37.3 s. The process tree peaked at 952 MB. Results:

- The tree's database (SHA-256 `6ab94a13…f9dd3de3`) equals the user's
  command for command after the prefix rewrite.
- All 992 linked objects reproduce from these commands.
- The executable is byte-identical to the first configure's
  (`e8990075…171d0682`): the extra include path and `MK_HAVE_REGEX` change
  no linked object. The map is identical too (`27f18cb5…7b549`).
- Every decision is identical: the same 1,384 derived names with the same
  homes, and the same 5,033 refusals with the same reasons
  ([`scripts/same_decisions.py`](scripts/same_decisions.py)). Only the
  premise's digests differ.

The records below come from this run, whose database is the user's.

## Coverage of the census population

[`scripts/coverage.py`](scripts/coverage.py) compares the candidate map with
BODY-SUMMARY-REUSE's census (`census.json`, cross-unit population) and its
165-home experiment map. Output: [`coverage.json`](coverage.json).

| Census cross-unit population | Callees | Sites |
|---|---:|---:|
| Derived home (caller configuration linked) | **132** | **383** |
| Refused: home configuration is not a unit of the current route (Monkey) | 24 | 55 |
| **Total** | **156** | **438** |

| 165-home experiment map | Names |
|---|---:|
| Derived, same home as the experiment | 133 |
| Refused: Monkey home | 24 |
| Not listed (same-unit callees no other linked input references) | 8 |
| Derived with a different home | 0 |

The 24 refused callees are Monkey functions (`mk_fifo_send`, `mk_iov_send`,
`mk_string_copy_substr`, and others). The `current` route analyses Monkey from
the separate libevent database. The executable links `lib/monkey` compiled
under Fluent Bit's own configuration, a different configuration of the same
files. Linking does not establish that the analysed configuration is the one
that runs, so those homes are refused. Their 55 call sites are all in Monkey
callers, which are likewise outside this database
(follow-up `CURRENT-MONKEY-CONFIGURATION-NOT-LINKED`).

Every derived census callee's home equals the experiment map's choice
(no "home differs" outcome), and every one of its 383 sites is in a caller
whose route configuration is linked into the executable. The 8 experiment
names the artifact does not list are same-unit callees (census reasons
`same-unit-external` / `same-unit-internal`), which no other linked input
references. Of the 7 cross-unit sole-blocked functions, 5 have every
blocker derived: `ctxTypeID_to_size`, `cio_chunk_hash`, `cio_chunk_is_up`,
`rd_kafka_share_consumer_closed_err` and
`rd_kafka_transport_ssl_threadid_cb`. The other 2, `mk_mq_send` and
`net_plain_writev`, are blocked by Monkey homes.

## What the next card needs

BODY-SUMMARY-REUSE-LINK-STANDING (rule 40, Lean `QCore.CrossUnitSummary`
extended) adopts a derived resolution. From this artifact it gets:

- **Home identity.** The definer's database configuration (file, directory,
  command digest, object digest, reproduced) and its route key, bound to
  the executable, database and toolchain digests.
- **Caller scope.** Rule 4c applies a map to every caller of the run. The
  premise covers only callers whose configuration is linked into the
  executable. The artifact gives `linked` for every configuration, so the
  route can withhold an import from an unlinked caller. On `current` these
  are the Monkey units (another database) and the shared configuration of
  `flb_input_chunk.c`. No derived census site has such a caller, but the
  route must check it rather than rely on the census.
- **Configuration identity.** The route keys homes by `library/file`, which
  cannot name one of two configurations of a file (the 26 ambiguous names).
- **Database identity.** The artifact must be produced after the final
  configure of the tree the run reads (see above). The route should compare
  the artifact's database digest with its own.
- **Call-type check.** The artifact does **not** carry function types. ELF
  symbol tables have none, and the call-site declaration is not in any
  object. The check that Clang's call type equals the home definition's
  type needs the caller's declaration in force at the call and the home's
  definition. Both are in the units the route already preprocesses. The
  objects are built with `-g`, so the home's DWARF subprogram type is
  available as a cross-check, but this card does not read it.

## Cost and fit

- Time: about 2 min of build and reproduction at 6 jobs on a configured
  tree (77.6 s link build plus 43.5 s reproduction). A reused tree rebuilds
  only what changed.
- Disk: the linked production tree is 302 MB (the user's configured-only
  tree is 81 MB). The artifact directory is 19 MB: map 9.8 MB, artifact
  8.1 MB, refusals 1.5 MB, candidate map 0.2 MB. The independent clone is
  375 MB. This is well inside the ~15 GB test storage budget. The step adds
  nothing to the Rust test build.
- Memory: peak process tree 832 MB, far inside the 4 GiB admission.
- The step runs on the measurement host as a prepare step. It needs GNU ld
  and the Makefile generator; it refuses anything else.

## Tests

In `crates/qcore-audit/src/link_resolution.rs`:

- `archive_members_resolve_short_and_long_names`: GNU `ar` short and long
  names, duplicates kept in order, thin archives refused;
- `map_reader_takes_members_loads_text_and_cross_references`: the map
  reader on included members (long labels), `LOAD`, wrapped section names,
  `*fill*`, and cross-reference entries (long names, references, a shared
  definition). A map without `--cref` is refused;
- `a_small_link_derives_only_unique_strong_reproduced_route_homes`: a real
  Clang plus GNU ld link (skipped without them). Strong homes are derived;
  `main` (referenced by the C runtime) is derived; a weak definition with a
  linked strong twin is refused as a duplicate; `puts` and `__cxa_finalize`
  are refused as shared; an object built at `-O2` but recorded at `-O0` is
  refused as not reproduced; a home outside the route is refused; a static
  function is not listed. The candidate map and refusal list match.

`steps_and_flags_parse_and_misuse_refuses` (in `prepare.rs`) now also
covers: `--link` is off by default, `--build-jobs` needs `--link`, a positive
count, and no repeat, and `prepare monkey --link` is refused.

No Lean change: the card adds no semantics, and the derived-resolution
theorem belongs to the next card. `PREPARE-COPY-KEEPS-EXECUTABLE-MODE-IN-PROCESS`
is unaffected: the link step writes no executable from the preparing
process. The linker writes the side image, which is never executed.

## Records

- [`coverage.json`](coverage.json): census coverage, per callee.
- [`link-map.candidate.json.gz`](link-map.candidate.json.gz) and
  [`link-map.candidate.refusals.json.gz`](link-map.candidate.refusals.json.gz):
  the candidate map and refusal list of the run whose database is the
  user's (gzip; digests below). The full artifact and the map stay in the
  card worktree under `private/link-prepare/`.
- [`scripts/`](scripts/): the run, coverage, database comparison and
  configure-twice scripts.
- [`gate.json`](gate.json): the card gate.
- [`integration-amendments.json`](integration-amendments.json).

| Record (run whose database is the user's) | SHA-256 |
|---|---|
| Compile database (in the card's output) | `6ab94a135d79959e480b6e69838b89c5cc92ed8366c57dd970b22b34f9dd3de3` |
| Executable `bin/fluent-bit` | `e8990075ef89ee8502420571a7a18acbc366504c9991b71766198768171d0682` |
| Link map (`--cref`) | `27f18cb534d7da8ee15ad19793fc70d82a129b8e947fb71faa9236bf2257b549` |
| `link.txt` | `1e0e475e220f9698da314fbb8eebe59d768ca469619d174364fd2544aba34d72` |
| Artifact `link-resolution.json` | `c6535626faae2e51dc3a092520bfea8e439ec158c0d39377da8841bd38690c9b` |
| `link-map.candidate.json` | `a528582ae6660cb40e129d324d79cf10724280bddbedfbe85bf9619fb7e93fbd` |
| `link-map.candidate.refusals.json` | `7ad302b86c2e26ea239665338dbee6f0c1708527ffbbea34f8421ea6c3cfdd14` |
| `link-map.candidate.json.gz` | `dcbda27196aa6c7f8b902779830394cb3b2447ab2ab085770dc00287763f8d1a` |
| `link-map.candidate.refusals.json.gz` | `9ebde0ba6849f285d00bfb679cd361f72c771f3c3b16ee5e391415c8ec482531` |

First-configure run (database `8cc891f0…829251f`): artifact
`e454b0cf…a054e`, candidate map `3048625e…1c3c5`, refusals
`0e0430ca…937a3`. They are kept in the card worktree
(`private/link-prepare/run-1/link-resolution.first-configure`).

## Gate

`python3 scripts/gate_scope.py --base 1e522ca5d --out private/gate-1` at
`1f50c5f58`: **exit 0**, scoped mode (packages `qcore-audit`,
`qcore-compiler` and `qcorec`, plus five test targets that read the tree).
The record is [`gate.json`](gate.json).

| Step | Exit | Counts |
|---|---:|---|
| submodules | 0 | 1 submodule |
| fmt | 0 | |
| clippy (and the two extra packages) | 0 | |
| rust-suite | 0 | 199 units, 2,611 passed, 0 failed, 4 ignored (8,902 s including the slot wait) |
| reevaluation probe / follow-up | 0 / 0 | |
| python | 0 | 685 tests |

No Lean step is planned (no Lean path) and no provider gate (no provider
path).

## Integrator measurement (2026-11-08)

Declared measurement, base `baseline:7b726587bdf6a8213f451fa1c78297bb5e476ef7`, tip `rev:b1aabb1e6`, run from the
main checkout (`private/measure/link-prepare-20261108T001639Z`): exit 0, all three routes
complete, no unmatched rows, declared result **match**.

- corpus: no row moved; 280 rows changed content only.
- system: no row moved; 0 rows changed content only.
- slice: no row moved; 0 rows changed content only.

[Comparison](integration/native-comparison.json.gz) (gzip of
`native-comparison.json`, sha256
`f0d2d49c64c0e729c6cee508b9561c87e1321bd567eabfbea3ef14d2e26c2683`).

Every content change is CURRENT-RUN-WORKER-SPIN's and CALLBACK-MEMBER-NON-C-UNITS's (landed after the baseline): nothing reads the link artifact on these routes.
