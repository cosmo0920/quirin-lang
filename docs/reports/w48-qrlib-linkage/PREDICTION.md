# W48-QRLIB-LINKAGE: prediction

Frozen before any source change, at base `d11179c9` (= local `main` =
origin/main; the worktree's HEAD was already that commit, so no reset or
fast-forward was needed). Taken first: the BEFORE fixture freeze
(`artifacts/fixtures-before.json`, card 0's `freeze_fixtures.py`, debug
`quirin` built at the base, sha256 `1e29353a02b2b241…`, 702 files — check 395
exit 0 / 307 exit 1, emit 186 / 516).

## Where the base stands

* `.qrlib` (`native_library manifest v1`) is a **hand-written pin**: the
  artifact hash is declared, never opened (`native_library.rs`'s module
  documentation argues why), `C_ABI_TYPE` has the §C04.1 integers and three
  byte pointers but no `float`/`double` and no struct, and `data_model` admits
  `lp64` only.
* `source.ffi.v1` never reads a manifest. Every FFI certificate carries the
  trusted `BuilderIncludesAndLinks { directories }` premise, so which code
  supplies a `#c_ffi` callee is the builder's word.
* No reader of any object, archive, shared object, text stub or import library
  exists in the tree; `nm`-like facts are nowhere derived.
* libm on this host: `libm.a` and `libm.so` are GNU `ld` scripts
  (`GROUP ( …/libm-2.43.a …/libmvec.a )`, `GROUP ( …/libm.so.6 AS_NEEDED (
  …/libmvec.so.1 ) )`); in `libm.so.6` `ilogb`, `sqrt`, `fabs`, `fmax`,
  `nextafter` are weak and versioned `@@GLIBC_2.2.5`, `floor`/`trunc` are
  `STT_GNU_IFUNC`; in `libm-2.43.a` the same functions are weak (floor an
  IFUNC) and `ldexp`/`copysign` are **not defined at all** (they live in
  `libc.a`).

## Plan the prediction is about

1. **Readers** (`qcore-c-gateway`, new `linkage` module, every reader a function
   of `&[u8]`): `ar` archives in the GNU/SysV, BSD and COFF layouts with their
   symbol indexes (`/`, `/SYM64/`, `__.SYMDEF[ SORTED]`, the COFF second linker
   member); ELF relocatable `.symtab`, shared `.dynsym` with `.gnu.version` /
   `.gnu.version_d` and `DT_SONAME`; GNU `ld` scripts; Mach-O object `nlist`
   and the dylib export trie; `.tbd` text stubs (v3/v4 YAML, v5 JSON); COFF
   object symbol tables, short import records and the PE export directory
   (forwarders). One resolver over the union: missing, undefined, local-only,
   ambiguous, hidden-version-only, ordinal-only and forwarder each refuse by
   name; a defined weak symbol is admitted with its weakness recorded.
2. **Generator** (`quirin qrlib generate`, library API): manifest v2 derived
   from the artifact bytes (sha256, symbol facts, `ld`-script inputs), the
   header digest, the recorded target (triple, data model) and the declared
   inputs (library, version, linkage kind, the three ABI premises, the contract
   modules whose every declaration becomes a `contract fn` entry, the Windows
   OS build). `quirin qrlib check` regenerates from a stored manifest's own
   declared inputs and refuses a byte difference as **stale**. `quirin build`
   and a `LinkPlan` API hash every pinned file immediately before linking and
   hand the linker exactly those paths.
3. **Grammar v2**: `linkage`, `format`, `target`, `input`, `runtime`, the
   per-entry `export SYMBOL [version V] global|weak [ifunc] in FILE` clause;
   `float`, `double`, `struct TAG`, `struct TAG *`, `const struct TAG *` in
   `C_ABI_TYPE`; `data_model llp64`. v1 keeps loading byte for byte.
4. **FFI composition**: every callee of an FFI file resolves through a v2
   manifest the file binds with `extern library NAME;` (Q065/Q066/Q041/Q064
   widened, no code allocated); `BuilderIncludesAndLinks` becomes
   `BuilderIncludes` plus one `LinkedSymbol` premise per callee (static:
   artifact@sha256; dynamic: stub@sha256 plus the named runtime premise), the
   unsafe-FFI assumption unchanged.
5. **End to end**: libm (static through `libm.a`'s script, dynamic through
   `libm.so`'s) on Linux, a card-built C `.a` and a `#![no_std]` Rust
   `staticlib`; MSVC `.lib` and one `ntdll.dll` function wired into
   `msvc-core`; the macOS path written for card 5m.

## Predictions

| Measure | Prediction | Confidence |
|---|---|---|
| items completed | 1–4 complete; 5 Linux complete, Windows wired not run, macOS written | 60% |
| readers | all nine formats read from bytes with unit tests on Linux; Mach-O/COFF fixtures card-built with LLVM tools and checked in | 70% |
| fixture freeze (702) | byte-identical except the two `examples/ffi/*.qr` programs, which gain `extern library NAME;` and move from `Q023` to `Q065` through the CLI (attributed) | 70% |
| existing tests changed | the pinned unspellable-float rows in `native_library_header.rs` (core unit and compiler) move to agreement/disagreement; `ffi_struct_import.rs`, `float_ffi.rs`, `usable_records.rs` gain generated manifests for their admitted calls; 4–8 files | 65% |
| libm on Linux | `ilogb`, `sqrt`, `fabs`, `floor`, `fmax`, `nextafter` run static and dynamic at `-O0`/`-O2` UBSan and plain `-O2`, outcomes equal to the reference (exact IEEE operations, so this platform's own reference equals the language's), 0 sanitizer reports | 70% |
| `ldexp` through `libm.a` | refused at generation: not defined in the pinned artifact | 90% |
| Lean | one module, 10–18 theorems, no placeholder, no project axiom | 75% |
| corpus arbiter | byte-identical `2fc3af8c…` (no C-audit path touched) | 95% |
| gate first run | green | 35% |
| provider gate | not needed (`providers/` untouched) | 85% |
