# W48-QR-C-STRUCT-IMPORT: importing a C struct into `.qr`

Card 1 of the `.qr` language track (base `96764798` = origin/main; the
worktree's HEAD was already that commit, so no reset or fast-forward was
needed). All five items of the brief are done. Grammar: implemented grammars
[§G02u](../../QCORE_IMPLEMENTED_GRAMMARS_r0_3.md#g02u-imported-c-structs-sourceffiv1);
semantics: specification §L14.6, §L20, §L21; contract specification §C04.12.
The prediction was frozen before any source change ([PREDICTION.md](PREDICTION.md)),
after the BEFORE fixture freeze and the base census.

## Decisions as implemented

| Decision (user) | Implementation |
|---|---|
| 1. `#import std.ffi.unsafe;` | A bundled module (`FFI_MODULE`), granted by no other: a `struct!` in a file without it refuses at `Q032`, the `std.extern.unsafe` gate widened (§L21). On the `UNSAFE-LINKAGE` precedent the two imports stay independent, so a **call** still needs `#import std.extern;` (`Q022`), and a line its output module. A file importing it is selected by `source.ffi.v1` before every other route. |
| 2. File-wide trust mark | `FfiAssumption::UnsafeFfi { structs, functions }` heads every certificate's assumption list, naming each imported struct (`Header = struct qr_header` with its header digest) and each `#c_ffi` function called, beside `RecordedCompiler`, `DeclaredContract` per callee and `BuilderIncludesAndLinks`. The published verdict is an `AssumedVerdict` (new in `qcore_schemas::verdict`: a value plus named assumptions; the join takes the larger value and the union of the assumptions) — `Safe` under all of them. `driver::ffi_verdict` is the tripwire: a certificate without the mark, or one that would be plain `Safe`, refuses at `Q035`. `quirin check` prints `…; Safe under the unsafe-ffi assumption, never plain Safe`, a `verdict:` line, and the assumptions. Not the `Unsafe` verdict. |
| 3. `struct! { type T = c"struct tag" from "h.h"; fields { … } }`, checked against the recorded compiler | Read by the frontend at declaration position (`DeclarationKind::ImportedStruct`, `source_ffi::imported_structs`). The header resolves against the declaring file's own directory. The layout comes from a new gateway reader, `qcore_c_gateway::ffi_layout`, which runs the gateway's Clang only when it **is** the recorded compiler (the recorded target's `compiler` = first `--version` line, `triple` = `-dumpmachine`), over a probe unit including the header: the typed AST (`-ast-dump=json`, members, written and canonical types, attributes) and Clang's record layout (`-fdump-record-layouts-complete`, offsets, size, alignment), which must name the same members. A listed field's type is compared by canonical type against the §G02t C11 spelling's canonical type under the same compiler; its **offset** is compared through the listing order, which must be the layout order — the listing states no offset (decision 3's "never declared by hand"), so a listing that places a field before one at a lower offset refuses (`Q064`). Unlisted members are opaque. |
| 4. `[| quant #c_ffi |]` on FFI definitions in `.qv` | A declaration attribute parsed before `extern fn` (`SourceDeclaration::c_ffi`), not one of the declaration's tokens. The loader reads a marked declaration by a closed FFI table (§C04.12: §G02t scalars, `c"struct TAG"`, `ptr<const, c"struct TAG">`, `unit` result; borrowed parameters and `resource_return_none()` only) and **withholds** it from the function map; `contract_loader::ffi_declarations` rederives it for `source.ffi.v1` alone. A struct-carrying call through an unmarked contract refuses at `Q033`; an unmarked declaration naming a C struct rejects its module; the marker in a `.qr` refuses at `Q033`. |
| 5. Output through `std.fmt`/`std.io`/`std.logger` | Lines are §G02g's own reader (`std_io_statement_tokens`) and card 0's renderings (`scalar_types::render`/`c_rendering`); no debug print anywhere, in fixtures, tests or emitted C (`direct_stdio_calls` is empty). |

## Items

| Item | Status |
|---|---|
| 1. Grammar and loader; census; card 4's lexical space | **done**. Census below. `|copy x, y|` lexes as `Pipe … Pipe`, and `[|`/`|]` only with an adjacent bracket (`the_marker_tokens_leave_the_capture_list_spelling_free`). |
| 2. Layout derivation; refusals; layout facts and compiler identity in the certificate | **done**. Every refusal of the brief has a test row: type mismatch, offset (order) mismatch, unknown tag, bit-field, flexible array member, union, `packed` and `_Alignas` attributes, anonymous member, a listed member with no §G02t equivalent (a `char[3]`); plus no recorded target and another compiler. `CheckedStruct.facts` publishes size, alignment, every member (offset, written and canonical type, equivalents), header reference, resolved path, directory, SHA-256, compiler and triple. |
| 3. Records from imported structs; real C type; `_Static_assert`s | **done**. Construct-then-read over the listed fields; the C `#include`s the header, `memset`s a constructed object then stores the listed fields, and asserts `sizeof`, each listed `offsetof` and each listed member's type (`_Generic`). |
| 4. FFI calls through marked contracts; the real library natively | **done**: by `const` pointer and by value, a struct returned by value, scalars both ways; `examples/ffi/` is a real header, a real `.c` and two marked contracts. |
| 5. Verdict plumbing | **done**, as decision 2 above. |

Kept refused, each with a follow-up (below): `ptr<mut, STRUCT>`, a listed member
of a non-§G02t type, layout attributes/bit-fields, a second function or record
across `.qr` calls, library-mode FFI modules, any other route's statement in an
FFI file, rendering a C result with no declared outcome (`Q043`).

## Diagnostics

No code is allocated (`Q-NAMESPACE-DECISION` icebox); every widening is stated
in §L21: `Q032` (FFI without its import), `Q033` (the marker outside a `.qv`;
a struct call through an unmarked contract), `Q034` (`unsafe extern fn` in an
FFI file), `Q035` (the plain-`Safe` tripwire), `Q063` (the header or compile
context does not resolve, or a shape the reader does not state), `Q064` (a
listing the recorded compiler contradicts); `Q051`–`Q053`, `Q022`–`Q027`,
`Q028`–`Q030` and `Q043` carry their existing conditions on the new route.
`docs/agents/diagnostics.md` is already over its 32 KiB cap, so its table was
not grown; §L21 and this report carry the widenings.

## Corrections

| What | Before | After | Why |
|---|---|---|---|
| `frontend::SourceDeclaration` | no attribute | `c_ffi: Option<Span>` | decision 4; constructed only by the parser |
| `DeclarationKind` | — | `ImportedStruct` | decision 3; `is_declaration_start` recognises `struct !` and `[| quant` (both parse errors before) |
| contract loader, an unmarked declaration whose type holds a string literal | "unsupported parameter type or usage annotation" | "a C struct type is admitted only on a declaration marked `[| quant #c_ffi |]`" | no tracked module has one (census) |
| `check_extern_declaration_signatures` | every admitted module | skipped for `FfiV1`, which reads its own declarations | `ptr<const, Header>` is that route's vocabulary |
| The brief names `typed_c_ast.rs` as the reader | — | the reader is a new sibling, `ffi_layout.rs`, on the same Clang (`clang::command`, JSON AST, the record-layout dump `typed_c_ast::record_layout_dump` also uses) | `typed_c_ast`'s transports read preprocessed text with no include path and take no compiler identity; the layout must be read with the header's directory on the path, by the recorded compiler |
| Where the marker gate runs | first written in the per-file import gate | the driver, on the bundle's mode | the per-file staged text no longer carries `module contract;`, so the first version refused the published `.qv` (caught by the AFTER freeze) |

## Evidence

**Census** ([`census_ffi_spelling.py`](census_ffi_spelling.py), token level,
all tracked `.qr`/`.qv`/`.qrlib`/`.qcore`/`.qc`): at the base, **0** `struct !`,
`c"…"`, `[|`, `|]`, `quant # c_ffi`, `c_ffi`, `std.ffi` import and `| copy`
in 632 files, and none of `struct`, `c`, `from`, `fields`, `c_ffi`, `ffi` is a
reserved word ([base](artifacts/census-base.json)); at the tip every hit is in
the two new example files ([tip](artifacts/census-tip.json)). No word reserved.

**Fixture identity.** Card 0's `freeze_fixtures.py` over every tracked
`.qr`/`.qv`/`.qrlib` file, debug `quirin` at the base (`a1583969…`, 632 files)
and at the tip ([BEFORE](artifacts/fixtures-before.json),
[AFTER](artifacts/fixtures-after.json), [comparison](artifacts/fixtures-compare.txt)):
**all 632 existing files are byte-identical** in exit, stdout and stderr of
`check` and `emit`; the only entries are the two new files. Through the bare
CLI (no option), `header_sum.qr` refuses both commands at `Q063` (no recorded
target), and `contracts/qr_header.qv` checks as a contract module (`check` 0,
`emit` 1, like every contract module). No existing refusal became an admission
or the reverse.

**Native** ([summary](artifacts/native-ffi.json)): `examples/ffi/header_sum.qr`,
emitted and built with `qr_header.c` by Clang 21.1.8, `-std=c11 -Wall -Wextra
-Werror -pedantic`, at `-O0` and `-O2` under `-fsanitize=undefined,integer
-fno-sanitize-recover=all` and at plain `-O2`: stdout `40`, `0x02`, `42`,
`true` and stderr `[INFO] 30`, equal to the reference observation, exit 0,
**0** sanitizer reports, in all three builds. A false declared outcome exits 70
before its line. Three drifted headers (a member moved, a member retyped, the
size grown) each fail the build on a `_Static_assert`.

**Negative controls** (`compiler/tests/ffi_struct_import.rs`, each a row):
no import (`Q032`); a type mismatch and an offset-order mismatch (`Q064`); an
unknown tag, a header that does not resolve, no recorded target, another
compiler, a union, a bit-field, a flexible array, `packed`, `_Alignas`, an
anonymous member, a listed `char[3]` (`Q063`); an opaque read and an opaque
write (`Q051`); a struct call through an unmarked contract (`Q033`) and an
unmarked contract naming the struct (`Q023`, the selection does not load); the
marker in a `.qr` (`Q033`); a marked contract called from a non-FFI file
(`Q023`, withheld); a marked declaration with custody, `ptr<mut, …>`, a union
or a spaced `c "…"` (load refused); a C result rendered with no declared outcome
(`Q043`); a call without `std.extern` (`Q022`); a line outside the effect row
(`Q029`); and the plain-`Safe` claim — the certificate with the trust mark
dropped, with every assumption dropped, and with a struct dropped from the mark
each refused by `ffi_verdict`, and a certificate whose published offset was
edited refused by its recheck (`LayoutDiffers`). The CLI test
(`crates/qcorec/tests/cli.rs::cli_prints_the_unsafe_ffi_verdict_and_its_premises`)
checks the printed verdict and premises, and the `Q063` refusal without
`--recorded-target`.

**Lean** (`QCore.FfiStructImport`, [audit](../../../proof/AuditFfiStructImport.lean),
16 theorems, no placeholder, no project axiom): layout agreement
(`checked_fields_agree`, `every_listed_read_is_at_the_recorded_offset`: every
admitted read is at the recorded offset with a type the member admits), layout
order (`checked_offsets_ascend`), opacity (`an_opaque_field_is_unreachable`,
via `checked_names`), no plain `Safe` (`an_ffi_file_never_publishes_plain_safe`,
`the_tripwire_rules_out_plain_safe`, `an_ffi_publication_passes_the_tripwire`),
the join keeps the assumption (`join_keeps_the_assumption`, `foldl_keeps`,
`every_composed_verdict_carries_the_assumption`,
`a_composition_with_an_ffi_part_is_never_plain_safe`), and non-vacuity over the
example's own layout (`the_example_listing_checks`, a mistyped, a reordered and
an unequivalent listing each refused, by `decide`). The verdict half reuses
`QCore.VerdictSemigroup.V.join`. The Rust binding is the shared example layout
and the test rows, not an extraction theorem; `qcore_schemas::verdict`'s unit
test replays the join-keeps-the-assumption property over every value.

## Prediction and outcome

GATE_TABLE_PLACEHOLDER

## Gate

GATE_PLACEHOLDER

## Corpus arbiter

ARBITER_PLACEHOLDER

## Follow-ups

Ledger ([followups.json](followups.json), applied by the idempotent
[`apply_ledger.py`](apply_ledger.py)): registers `W48-QR-FFI-MUT-POINTER`,
`W48-QR-FFI-NESTED-AND-ARRAY-FIELDS`, `W48-QR-FFI-LAYOUT-ATTRIBUTES`,
`W48-QR-FFI-CLI-CONTRACT-SELECTION`, `W48-QR-FFI-ROUTE-SCOPE` and
`W48-QR-FFI-HEADER-IDENTITY-AT-BUILD`, each with its size and discharge, and
amends `W48-QR-TYPES-MSVC-NATIVE` (the FFI emission and the Clang-only reader):
625 → 631. The increment record `wave-48.md` is at 33 KiB after this card's
paragraph (the brief names that file); the integrator may open `wave-48` part 2.

## Files likely to conflict with cards 2–4

`crates/qcore-core/src/ffi.rs` and `ffi_c_emit.rs` (new: records, calls,
lines, the certificate — card 2's writes, nesting and passing extend these);
`crates/qcore-frontend/src/source_ffi.rs` (new: the statement reader) and
`frontend.rs` (`DeclarationKind::ImportedStruct`, `SourceDeclaration::c_ffi`,
`is_declaration_start`); `crates/qcore-frontend/src/module_imports.rs` (the
`Q032` gate); `crates/qcore-verification/src/driver.rs` (route selection first,
`compile_ffi_source`, `ffi_verdict`, `CheckedSourceModule::Ffi`, the marker
gate); `crates/qcore-contract/src/contract_loader.rs` (`load_ffi_function`,
`ffi_declarations`); `crates/qcore-c-gateway/src/ffi_layout.rs`;
`crates/qcore-schemas/src/verdict.rs` (`AssumedVerdict`, which cards 3–4's
compositions should publish through); `crates/qcorec/src/main.rs` (the verdict
and assumption lines); implemented grammars §G02u and §G07.13, specification
§L14.6/§L20/§L21, contract specification §C04.12; `proof/QCore.lean`. Card 4's
capture list meets this card only lexically: keep `|copy …|` away from an
adjacent `[`/`]`.

## Scripts

* [`census_ffi_spelling.py`](census_ffi_spelling.py) — the constraint-14 census.
* Card 0's `freeze_fixtures.py` and `compare_fixtures.py`, unchanged.
* [`apply_ledger.py`](apply_ledger.py) — applies `followups.json`, idempotently.
* The native summary: `QCORE_W48_FFI_NATIVE_SUMMARY=native-ffi.json cargo test
  -p qcore-compiler --test ffi_struct_import the_example_runs` (written under
  `compiler/`, then moved to `artifacts/`).
