# Persistent Cycle 19 build inputs

The optional Monkey audit database must describe the forced Libevent build.
Copying only `compile_commands.json` out of `/tmp` is insufficient: its source
files, working directories and generated include paths must also survive.

From the QCore-C repository root, prepare the pinned source copy and configure
its headers and commands in the ignored, persistent `build/cycle19` directory:

```sh
python3 -B scripts/prepare_cycle19_monkey.py --fluent-bit ../fluent-bit
cargo run --quiet -p qcore-compiler --example cycle19_fluent_bit_libraries_audit -- \
  ../fluent-bit build/cycle19/monkey-build/compile_commands.json
```

The setup uses the declared Fluent Bit profile pin, checks a clean Monkey source
subtree, and retains a byte-identified copy. CMake explicitly selects Libevent
and disables the other event loops. `qcore-monkey-inputs.json` records source
tracked-source hashes, separate generated-header hashes, CMake arguments,
database identity and paths; `configure.log` records
configuration output. Source code in the upstream checkout is not edited.

The setup is repeatable with unchanged inputs. It refuses an unrecognized
output directory or a changed source copy; use `--output` with a fresh persistent
directory when intentionally changing build inputs. `--cmake` can select an
installed CMake executable. The default output is relative to the repository,
not the shell's temporary directory. Do not commit generated platform-specific
commands, headers or copied third-party source; commit this setup recipe instead.

The normal Fluent Bit audit also needs the upstream checkout's own
`build/compile_commands.json`. The Monkey setup does not create that separate
build configuration. Existing pinned source/build and contract checks continue
to apply. Historical reports retain their original temporary paths as evidence
of those runs; use this recipe for new runs.

This configures the build but does not claim native binaries or sanitizer runs.
If a native observation harness needs built libraries, build the configured tree
with `cmake --build build/cycle19/monkey-build --parallel 2` first and retain its
separate build/observation evidence.
