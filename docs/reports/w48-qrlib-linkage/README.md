# W48-QRLIB-LINKAGE: `.qrlib` as the C linkage manifest of the FFI route

Card 5 of the `.qr` track, base `d11179c9` (= local `main` = origin/main; the
worktree's HEAD was already that commit, so no reset or fast-forward was
needed). `.qrlib` becomes the C linkage manifest of `source.ffi.v1`,
**generated on every build** from the artifact's own exported symbol table,
which the card reads from bytes for ELF, Mach-O and COFF/PE. Grammar:
implemented grammars [§G02d.2.1](../../QCORE_IMPLEMENTED_GRAMMARS_r0_3.md#g02d21-the-generated-manifest-v2),
§G02d.3, §G02u; semantics: specification
[§L14.8](../../QCORE_SPEC_r0_3.md#l148-c-linkage-through-generated-manifests-and-its-limitations)
(normative limitations) and §L20; contract specification §C04.12. The
prediction was frozen before any source change ([PREDICTION.md](PREDICTION.md)).

## Mechanism

* **Readers** (`crates/qcore-c-gateway/src/linkage/`, every reader a function
  of `&[u8]`, bounds-checked through `bytes.rs`):

  | Format | Module | What is read |
  |---|---|---|
  | `ar` (all three layouts) | `archive.rs` | members, GNU `/`/`/SYM64/` index, BSD `__.SYMDEF[ SORTED]`/`__.SYMDEF_64` (with `#1/LEN` names), COFF first and second linker members, `//` long names; thin archives refused |
  | ELF | `elf.rs` | relocatable `.symtab` (global/weak/local/undefined, `STT_GNU_IFUNC`); shared `.dynsym` with `.gnu.version`/`.gnu.version_d` (default `@@` vs hidden `@`), hidden visibility as not exported, `DT_SONAME`; 32/64-bit, both byte orders |
  | GNU `ld` script | `ld_script.rs` | `GROUP`/`INPUT`/`AS_NEEDED`/`OUTPUT_FORMAT`, comments; `-l`, relative paths, other commands refused |
  | Mach-O | `macho.rs` | object `nlist` (`N_EXT`, `N_PEXT`, `N_WEAK_DEF`, undefined), dylib export trie (`LC_DYLD_EXPORTS_TRIE`/`LC_DYLD_INFO`, weak, re-export, resolver) and `LC_ID_DYLIB`; fat refused |
  | `.tbd` | `tbd.rs` | v3/v4 YAML subset and v5 JSON; per-target exports and weak symbols, re-exported documents followed inside the file, install name and current version |
  | COFF/PE | `coff.rs` | object symbol tables (external, weak external, common, static), short import records (by name / no prefix / undecorate / export-as / **ordinal**), the PE export directory with **forwarders**; `/bigobj`/LTCG refused |

  `artifact.rs` recognises the file the link names, follows a script to every
  file it names (one level; mixed static/shared refused), checks each file is
  for the recorded target's format and machine, marks archive definitions the
  index lists, and folds the union. `resolve` (in `mod.rs`) admits exactly one
  linkable definition and refuses by name: missing (stripped), undefined,
  local-only, ambiguous, not indexed, hidden-version-only, ordinal-only; the
  generator adds not-exported-by-runtime and forwarder for a Windows import.
* **Generator** (`generate.rs`): `generate(inputs, loader, runtime)` builds a
  `NativeLibraryManifest` with a linkage section and renders it with the
  kernel's `render_manifest`, then reads it back and refuses a round-trip
  mismatch; `check_stale(stored, …)` regenerates and names the first differing
  line. Every declaration of every contract module given becomes one entry
  (marked `#c_ffi` declarations through `ffi_declarations`, struct positions
  by tag), resolved at its object-level name (`_` prefix on Mach-O).
* **Build path** (`link.rs`, `crates/qcorec/src/qrlib.rs`): `link::plan`
  re-hashes the artifact and every script input, re-reads the script and
  requires it to name exactly the pinned inputs, and returns the one path to
  hand the linker; `LinkPlan::admit` refuses any other. `quirin qrlib generate`
  / `quirin qrlib check` take the declared inputs as flags (none defaulted);
  `quirin build` regenerates each `--native-library` from its own declared
  inputs (stale refused), checks with `--contracts`, emits, plans and links
  with the recorded Clang (`-std=c11 -O2 -Wall -Wextra -Werror -pedantic
  -ffp-contract=off`). References resolve against the manifest's directory.
* **Kernel** (`crates/qcore-core/src/native_library.rs`): the v2 grammar
  (below), `EntryType` (ABI, `struct TAG`, `[const] struct TAG *`),
  `ExportFacts`, `LinkageSection`, `RuntimePremise`, `DataModel::Llp64`,
  `render_manifest`, `check_ffi_agreement`; `float`/`double` in
  `parse_c_declaration_type` for both versions. `native_library_header.rs`
  projects `float`/`double` and refuses a struct entry under the bounded reader
  (`StructEntry`, `Q063`).
* **FFI composition** (`driver.rs::resolve_ffi_links`, `ffi.rs`): after callee
  admission, every callee is resolved through the declaring file's
  `extern library` view of the selection; the result is an `FfiLink` per callee
  in `FfiProgram::links`. `check_ffi_program` refuses a called function without
  one (`Q066`); `derive_assumptions` emits `BuilderIncludes` (include path
  only) and one `FfiAssumption::Linked` per called function;
  `certificate_carries_linkage` is the `Q067` tripwire; `ffi_verdict` names
  `linked:NAME` among the verdict's assumptions.

## Decisions as implemented

| Decision | Implementation |
|---|---|
| 1. Generated on every build | `generate` from the artifact's bytes (sha256, symbol table, script inputs), the header bytes (digest), the recorded target (triple, format, `lp64`/`llp64`) and the declared inputs (library, version, linkage, three premises, contract modules, Windows OS build); `check_stale` and `quirin build` refuse a stored manifest that differs byte for byte. The checker still reads no file. |
| 2. Linkage is the exported symbol table | the card's readers, never `nm`; missing/undefined/local-only/duplicate refused; defined weak admitted and recorded `weak`; ELF default version recorded (`version GLIBC_2.2.5`) and compared by the stale check; hidden-only refused; archive index required. |
| 3. Two kinds, every platform | `linkage static` (ELF/Mach-O/COFF archives, scripts of archives) states no runtime; `linkage dynamic` (ELF `.so` or scripts of them, `.tbd`, dylib trie, COFF import library) pins the stub and states `runtime soname …`/`install_name …`/`dll … os_build …`. Windows imports by name only; the runtime DLL's export directory is read and a forwarder or absence refused. |
| 4. Test targets | libm static and dynamic on Linux (run); `ntdll.dll` through `ntdll.lib` (wired, the function chosen on the CI image from two candidates by the by-name/non-forwarder checks); a C `.a`, a `#![no_std]` Rust `staticlib` (run on Linux) and an MSVC `.lib` (wired). |
| 5. Composition | every FFI callee resolves through a generated manifest bound with `extern library NAME;`; `BuilderIncludesAndLinks` replaced; `C_ABI_TYPE` gains `float`, `double`, `struct TAG`, `struct TAG *`, `const struct TAG *` (the last because the FFI route's pointer is `ptr<const, …>`); the manifest header digest must equal the `struct!` read; `quirin build` links exactly the pinned artifact after re-hashing it; the unsafe-FFI assumption unchanged. |
| 6. Spec limitations | §L14.8, 12 normative items (runtime premise, weak interposition, versions, `ifunc`, a static artifact's own references, `ld` scripts, forwarders and API sets, archive indexes and refused shapes, libm ulps → §L14.7 item 11, build premises, evidence). |
| 7. Output | the card's programs print only through `std.fmt`/`std.io`; no debug print was added. |

**Scope statement.** Decision 1 is applied to the FFI route, where a v1
(hand-written) manifest now refuses (`Q065`). The §G02d.1–§G02d.3 extern routes
keep loading v1 manifests byte for byte, because their fixtures state manifests
over artifacts no test builds; retiring v1 there is `W48-QRLIB-V1-RETIREMENT`.
Every callee of an FFI file — marked or an unmarked custody-free scalar
contract — is resolved through the manifest, since leaving the unmarked ones on
the builder premise would not replace it.

## Manifest grammar changes

`native_library manifest v2;` adds `linkage`, `format`, `target` (mandatory,
once), repeatable `input REF sha256:… [as_needed]` and `runtime …`, `data_model
llp64`, and a mandatory per-entry clause `export SYMBOL [version V]
(global|weak) [ifunc] [dll DLL] in FILE`; it refuses `unchecked fn` and
`include`. Both versions spell `float`/`double`; struct spellings are v2 only.
Each violation is a named `ManifestRefusal` (`WrongVersionField`,
`UncheckedInGenerated`, `MissingExport`, `MalformedExport`, `MalformedInput`,
`MalformedRuntime`, `RuntimeInStaticLinkage`, `MissingRuntime`), mapped to
`Q039`. A generated libm manifest: [artifacts/libm-dynamic.qrlib.txt](artifacts/libm-dynamic.qrlib.txt).

## Items

| Item | Status |
|---|---|
| 1. Readers with fixtures | **done**: nine formats, `crates/qcore-c-gateway/tests/unit/linkage.rs` (23 tests) over LLVM-built fixtures checked in by [make_fixtures.sh](make_fixtures.sh) (deterministic, rebuilt twice identical) plus byte-built PE and dylib images |
| 2. Generator, stale refusal, build path | **done**: `linkage_generate.rs` (9 tests), `quirin qrlib generate/check`, `quirin build` (exercised by hand on libm and on the README flow) |
| 3. Grammar | **done**: `native_library_v2.rs` (6 tests); grammars, language and contract specs amended |
| 4. FFI composition and verdict | **done**: `resolve_ffi_links`, `FfiLink`, `Linked`, `Q067` tripwire |
| 5. End to end | Linux **run**; Windows **wired, not run**; macOS **written, not run** |

Still refused (each by name): v1 manifests on the FFI route; the shapes of
`W48-QRLIB-READER-SHAPES`; `long`-typed C functions (`W48-QRLIB-LONG-SPELLING`);
glibc's static `floor` (link failure, §L14.8 item 5).

## Corrections

| What | Before | After | Reason |
|---|---|---|---|
| `crates/qcore-core/tests/unit/native_library_header.rs` `a_floating_result_is_read_and_cannot_be_spelled` | `UnspellableResult { projection: "f64" }` | renamed `…_and_now_spelled`: a `uint64_t` transcription is a `Result` disagreement, `double` and `float` agree | `W48-QR-FLOAT-QRLIB-SPELLING` discharged |
| `compiler/tests/native_library_header.rs` `the_reader_divergences_are_named_and_pinned`, item 4 | `("agreed", "UnspellableResult")` | `("agreed", "Disagreement")` | the same |
| `crates/qcore-core/tests/unit/native_library.rs` | entry types compared as `AbiType` | `EntryType::Abi(…)`, `export: None` asserted | compile-only |
| `compiler/tests/ffi_struct_import.rs`, `float_ffi.rs`, `usable_records.rs` | admitted FFI programs called C with no manifest and linked the library's `.c` source | they bind `extern library`, build the library into an archive (`tests/support/qrlib.rs`), select its generated manifest, and link the verified archive | decision 5: an FFI call with no manifest binding refuses |
| `examples/ffi/header_sum.qr`, `record_passing.qr`, `examples/ffi/README.md` | no binding | `extern library qr_header;` / `qr_point;`, the build flow documented and replayed | the same |
| `docs/QCORE_CONTRACT_SPEC_r0_3.md` §C04.12 | listed `f32`/`f64` among refused `#c_ffi` types | they are §G02t scalars since the float card | stale text in the amended section |

## Evidence

**Negative controls** (`compiler/tests/qrlib_linkage.rs`, first): an FFI call
with no `extern library` (`Q065`) and a binding with nothing selected (`Q065`);
a hand-written v1 manifest (`Q065`); a callee the bound manifest does not bind
(`Q066`); an entry spelling `int64_t` where the contract has `int32_t`
(`Q041`); a manifest header one comment line longer than the `struct!` header
(`Q064`); a stored manifest after the library was rebuilt from changed source
(stale) and the link plan refusing the changed archive (`DigestMismatch`) and
any other path (`OtherArtifact`); a certificate stripped of its `Linked`
premises (`certificate_carries_linkage` false). Linux: `ldexp` through the
static libm (not defined there: refused), a `GLIBC_*` version edited in a stored
libm manifest (stale). Reader/generator level (from bytes): a stripped, a
local-only (`objcopy --localize-symbol`), an undefined and a duplicated symbol;
an archive with no index; a hidden-version-only symbol; an ordinal-only import;
a forwarder and an absent runtime export; a missing OS build; a file for
another machine or format; a script with `-l`, a relative path, another
command or mixed kinds; truncated inputs (no panic); every v2 grammar
violation.

**Fixture identity.** Card 0's `freeze_fixtures.py` over every tracked
`.qr`/`.qv`/`.qrlib` file, debug `quirin` at the base (`1e29353a…`) and after
(`8608bf42…`), 702 files each ([BEFORE](artifacts/fixtures-before.json),
[AFTER](artifacts/fixtures-after.json), [comparison](artifacts/fixtures-compare.txt)):
**700 byte-identical**; the two `examples/ffi` programs, which gain `extern
library`, keep exit 1 and empty stdout and move their refusal from `Q063` (no
recorded target) to `Q065` (the binding resolves before route selection and
the CLI selects no manifest). No other admission or refusal moved.

**Native results on Linux** (Clang 21.1.8, [summary](artifacts/native-libm.json)):
three builds each — `-O0` and `-O2` with `-fsanitize=undefined,integer
-fno-sanitize=float-divide-by-zero -fno-sanitize-recover=all`, and plain
`-O2`, all `-std=c11 -Wall -Wextra -Werror -pedantic -ffp-contract=off`:

| Row | Artifact | Result |
|---|---|---|
| libm static | `libm.a` script → `libm-2.43.a`, `libmvec.a` (3 files pinned) | `ilogb`, `sqrt`, `fabs`, `fmax`, `nextafter`: `10`, `0x1.6a09e667f3bcdp+0`, `3.5`, `2`, `0x1.0000000000001p+0`, equal to the reference, exit 0, 0 reports; every entry `weak`, no version; `--trace` names the script and both archives |
| libm dynamic | `libm.so` script → `libm.so.6`, `libmvec.so.1` (`AS_NEEDED`) | the same plus `floor` (`2`, recorded `ifunc`); every entry `weak`, `version GLIBC_2.2.5`; `runtime soname libm.so.6`; `--trace` names the script and `libm.so.6` |
| C archive | card-built `libqr_demo.a` (struct by `const` pointer, scalar) | `42`, `42`, exit 0, 0 reports |
| Rust `staticlib` | `rustc --crate-type staticlib -C panic=abort`, `#![no_std]`, no header | `qr_rust_add`, `qr_rust_scale`: `42`, `6`, exit 0, 0 reports |

Each outcome is an exact IEEE operation, so this platform's own libm reference
equals the language's; no cross-platform identity is claimed (§L14.7 item 11).
The existing FFI examples (`header_sum`, `record_passing`, the float FFI row)
now link their generated archives under the same three builds, unchanged
output. By hand, `quirin build` over `libm.so` and over the README flow built
and ran; the same build after rebuilding `libqr_header.a` refused as stale.

**Windows (wired, not run).** `.github/workflows/ci.yml`'s `msvc-core` step
gains `cargo test -p qcore-c-gateway --lib linkage::` and `cargo test -p
qcore-compiler --test qrlib_linkage -- --nocapture`: the from-bytes readers,
`an_msvc_static_library_links_through_its_generated_manifest` (`cl`/`lib`
through `cc`, `format coff`, `data_model llp64`, linked by
`windows_c::run_msvc_with_libraries`) and
`one_ntdll_function_links_through_the_sdk_import_library` (`ntdll.lib` from
`LIB`, `System32\ntdll.dll`'s export directory, OS build from `cmd /c ver`,
outcomes `0` and `5` for `STATUS_ACCESS_DENIED`). `W48-QRLIB-WINDOWS-RUN`.

**macOS path for card 5m (written, not run).** From the repository root on a
Mac with the Command Line Tools:

```sh
cargo test -p qcore-c-gateway --lib linkage::
cargo test -p qcore-compiler --test qrlib_linkage --test ffi_struct_import \
    --test float_ffi --test usable_records
```

`libm_links_through_the_sdk_text_stub_under_three_builds` generates a dynamic
manifest from `$(xcrun --show-sdk-path)/usr/lib/libm.tbd` (runtime premise
`/usr/lib/libSystem.B.dylib`), links the stub and runs the six libm calls under
the three builds; the `cfg(unix)` rows build Mach-O archives (C and Rust) and
link them the same way. By hand: `quirin qrlib generate --linkage dynamic
--artifact $(xcrun --show-sdk-path)/usr/lib/libm.tbd …` then `quirin build`.
`W48-QRLIB-MACOS`.

**Lean** (`QCore.QrlibLinkage`, [AuditQrlibLinkage.lean](../../../proof/AuditQrlibLinkage.lean),
18 theorems, axioms only `propext`/`Classical.choice`/`Quot.sound`):
admission (`an_admitted_binding_names_an_exported_defined_symbol`,
`an_admitted_binding_is_the_only_linkable_one`, and `decide` rows over the
fixture's facts for weak, default-version, missing, undefined, local-only,
duplicate, hidden-only, unindexed and ordinal-only); generation
(`generation_reads_only_the_hash_and_the_bound_names` — the manifest is a
function of the artifact's hash, the records of the bound names and the
declared inputs — `every_generated_entry_names_an_exported_defined_symbol`,
`the_generated_manifest_pins_the_hash`, `a_freshly_generated_manifest_is_not_stale`,
`another_artifact_makes_the_stored_manifest_stale`,
`one_refused_binding_refuses_the_manifest`); publication
(`a_publication_names_every_called_functions_artifact`,
`an_ffi_publication_is_never_plain_safe`, `an_unlinked_call_is_not_published`,
`every_composed_verdict_keeps_the_linkage_premise`). The byte readers are not
modelled; they are pinned by the Rust tests over the fixtures.

## Prediction and outcome

| Measure | Predicted | Outcome |
|---|---|---|
| items | 1–4 complete, 5 Linux complete, 60% | **as predicted** |
| readers | nine formats from bytes, LLVM-built fixtures, 70% | **as predicted** (PE and dylib byte-built in the test instead: no `lld-link`/`ld64.lld` here) |
| fixture freeze | 700 identical; the two `examples/ffi` move `Q023` → `Q065`, 70% | 700 identical; the two move **`Q063`** → `Q065` (the CLI refused them earlier than predicted, on the missing recorded target) |
| existing tests changed | 4–8 files, 65% | **6** (2 behavioural float rows, 1 compile-only, 3 FFI test files) |
| libm | six functions both kinds, 70% | dynamic six; **static five** — glibc's static `floor` does not link into a dynamic executable (not predicted; §L14.8 item 5) |
| `ldexp` from `libm.a` refused | 90% | **refused** |
| Lean | 10–18 theorems, 75% | **18** |
| corpus arbiter | identical `2fc3af8c…`, 95% | **byte-identical** |
| gate first run | green, 35% | **not green**: clippy's `cloned_ref_to_slice_refs` in a new unit test; every other step 0 on the first run |
| provider gate | not needed, 85% | not needed (`providers/` untouched) |

## Gate

Script `private/w48-qrlib-linkage/gate.sh` (each exit to a file, no step
piped), logs `private/w48-qrlib-linkage/gate/`. The first run (at `537ff0d9`)
had clippy 101 on one new unit test's `&[pointer.clone()]`; after the fix
(`a46d6129`, a test file only) clippy, fmt and the whole test step were rerun
at that tip:

| Step | Command | Exit | Count |
|---|---|---|---|
| submodule | `git submodule update --init --recursive` | 0 | |
| fmt | `cargo fmt --all -- --check` | 0 | |
| tests | `python3 scripts/run_test_binaries.py` | 0 | 396 units, 409 summaries; **6,971 passed, 0 failed, 11 ignored** (`private/test-binaries/20260930T111228Z-2877687/summary.json`) |
| clippy | `cargo clippy --all-targets -- -D warnings` | 0 | (101 on the first run, above) |
| probe | `cargo run -p qcore-compiler --example reevaluation_probe` | 0 | |
| follow-up probe | `cargo run -p qcore-compiler --example reevaluation_followup` | 0 | |
| Lean | `python3 scripts/check_lean_audits.py` | 0 | **251 audits** (250 + `AuditQrlibLinkage.lean`) |
| matrix | `python3 docs/reports/remaining-d02/build_matrix.py . --gate docs/reports/w48-qrlib-linkage/gate.json` | MATRIX | |
| docs index | `python3 scripts/build_docs_index.py` | INDEX | |
| Python | `python3 -m unittest discover -s scripts/tests` | PYTHON | |

`providers/` is unchanged, so the provider gate is not due and was not run.
Clang 21.1.8 recorded in this worktree. Exits in [gate/](gate/), record in
[gate.json](gate.json).

## Corpus arbiter

`QCORE_TYPED_PATH_PARITY=OUT/parity.jsonl python3 scripts/run_pinned_audit.py
OUT 66910c10 --inputs-root …/build/cycle19 --jobs 8`, AFTER only, with the
card's release `quirin-audit` (`3ebeb068…`): exit 0.
`resource-diagnostics.json` sha256 `2fc3af8c39f0cf04…` (602 Safe / 10 Unsafe /
4,222 Unknown) is **byte-identical** to the base's archive (W48-QR-FLOATING-POINT's
AFTER), and so are `resource-diagnostics.chunk-contracts.json` (`cbbcfd9c…`)
and `monkey-source.json` (`57acfc58…`)
([identity](artifacts/arbiter-corpus.identity)). No C-audit path moved.

## Follow-ups

Registered (`named_followups` 649 → 657, [followups.json](followups.json),
applied by the idempotent [apply_ledger.py](apply_ledger.py)): retired
`W48-QR-FLOAT-QRLIB-SPELLING`; registered `W48-QRLIB-WINDOWS-RUN` (S),
`W48-QRLIB-MACOS` (M, card 5m), `W48-QRLIB-V1-RETIREMENT` (M),
`W48-QRLIB-STATIC-REFERENCES` (M), `W48-QRLIB-HEADER-DECLARATIONS` (M),
`W48-QRLIB-READER-SHAPES` (M), `W48-QRLIB-API-SETS` (S),
`W48-QRLIB-LONG-SPELLING` (S), `W48-QRLIB-CLI-CONTRACTS` (S).

## Files likely to conflict with 5m

`crates/qcore-c-gateway/src/linkage/{macho,tbd,artifact}.rs` and
`tests/unit/linkage.rs` (real SDK stubs), `compiler/tests/qrlib_linkage.rs`
(the macOS row), `tests/support/qrlib.rs` (Apple `ar`/`ranlib` through `cc`),
`.github/workflows/ci.yml` (the new job), `docs/development/platform-evidence.md`,
specification §L14.8, grammars §G02d.2.1.

## Scripts

* [make_fixtures.sh](make_fixtures.sh) — the reader fixtures.
* [apply_ledger.py](apply_ledger.py) with [followups.json](followups.json).
* Card 0's `freeze_fixtures.py`/`compare_fixtures.py` for the freeze.
* Native summary: `QCORE_W48_QRLIB_NATIVE_SUMMARY=FILE cargo test -p
  qcore-compiler --test qrlib_linkage libm_links`.
