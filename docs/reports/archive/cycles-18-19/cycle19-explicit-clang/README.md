# Explicit Clang Cycle19 audit — 2026-09-07

The gateway requires Clang. The prior command checkpoint completed an inventory
but refused all compilation commands before resource analysis. This change
prepares explicit Clang builds and admits the actual supported compiler options.

## Run

```sh
python3 -B scripts/prepare_cycle19_clang_audit.py --clang clang --clangxx clang++ --run
```

The helper resolves local HEAD once, prepares an independent Fluent Bit clone
and forced-Libevent Monkey inputs, and sets `QCORE_CLANG` for the audit. Default
output is `build/cycle19/clang/<resolved commit>`; `--fluent-bit`, `--revision`
and `--output` select other inputs. Without `--run`, it prints the replay command.
`audit-command.json` records arguments, environment and working directory.
The developer's GCC build is left intact. An explicitly reused output directory
with a different revision or compiler is rejected; use a fresh output.

The example accepts `--compile-database PATH` before positional arguments. Both
selected databases must resolve to the configured Clang C driver. The original
GCC command rejects once with preparation instructions, before per-unit work;
it is never silently rewritten to use Clang. GCC 15 on this host reported a
GNU C23 default in the macro probe, making implicit compiler substitution
particularly inappropriate. See [GCC dialect options](https://gcc.gnu.org/onlinedocs/gcc/C-Dialect-Options.html).

## Transport and repeat preparation

- Both CMake preparations select explicit C/C++ compilers. In addition to
  `CMAKE_C_STANDARD=17`, they set `CMAKE_C_FLAGS=-std=gnu17`: CMake may omit a
  generated `-std` when it knows the compiler default already matches. The
  audit preserves the final dialect from each complete command, including
  later target-specific standard options. Repeat preparation remains usable;
  identical generated commands/binaries across reconfiguration are not claimed.
- `-Wextra` is explicitly transported through preprocessing, syntax checking
  and typed AST import, and remains in ordered compile-task identity.
- Dollar bytes inside one quoted C string definition are preserved literally,
  including the filename replacement in these databases. The adapter does not
  execute Make/shell expressions, erase the macro, or guess its intended value.
  Active shell expansion and unquoted unresolved expressions still reject.
  `-D` token and ordering behavior is described by [GCC preprocessing options](https://gcc.gnu.org/onlinedocs/gcc/Preprocessor-Options.html).
- Compiler checks resolve executable identity without executing arbitrary
  database programs. C++/CL drivers and mismatched compiler caches reject.
  CMake's FILEPATH, STRING and UNINITIALIZED compiler cache entries are handled.
- Distinct configurations remain separate obligations. Historical provider,
  source-specific and native observation sidecars remain explicitly Unknown.

## Scope and validation

The native preparation builds `cmetrics-static`, not the entire Fluent Bit
executable. Results bind literal compilation-database arguments. GCC equivalence,
extra Make expansion stages, general source-filename semantics, native lifecycle
observations and whole-library safety are not established by this resource audit.

All 857 workspace tests, fmt, Clippy with warnings denied, both reevaluation
examples, Lean build and 20 Lean audits pass. Direct Clang preprocessing and AST
parity now include `-Wextra`. A new regression checks literal-dollar preservation
and rejects unresolved alternatives; the Cycle19 integration test exercises the
explicit compiler and separate database. Compiler selection/transport changes do
not change Lean language/resource definitions.

## Verified run after repeat preparation

Source: `9f0fa5730fba1761ebaf14bcd31220a3a9be9d6e`, independently prepared
under `build/cycle19/explicit-clang-verified`. Preparation and repeat preparation
used the same resolved revision; the upstream developer checkout advanced again
while validation was in progress, and its new HEAD was not substituted mid-run.

**185 source files / 186 configurations: 2 Safe / 43 provisional Unsafe /
137 Unknown / 4 data-only / 0 gateway failures.** All 186 have accepted compile
task identities; there are **zero compiler-command refusals**. Unknown now
reflects resource-analysis limits. The extra configuration is a second wrapper
command. These are scoped resource projections under declared assumptions,
not proven upstream defects or a whole-program safety certificate.

The verified audit ran after repeat preparation completed. Compressed reports,
provenance, command metadata and validation logs are archived alongside
`validation.json`. Similar counts to older reports are not revision-only or
full source/build parity: the compiler configuration and source revision differ.
