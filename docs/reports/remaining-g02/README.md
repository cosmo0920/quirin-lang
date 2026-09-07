# G02 source-filename-sensitive preprocessing transport — 2026-09-07

The unrestricted-C gateway fed Clang through standard input for every phase.
Preprocessing is not a function of the source text alone, so that transport
answered several constructs for a translation unit nobody compiled. This card
inventories the transport, measures the divergence against a direct configured
Clang run, transports the declared input where the fix is transport-only, and
rejects — with a diagnostic naming the construct — where the answer would have
to be guessed.

Nothing here promotes a raw unit, changes a verdict rule, or claims that the
gateway's staged preprocess/syntax/AST pipeline is equivalent to a direct
one-shot compilation. The card's predecessor `CompileTaskIdentity` work (G01)
is **not** present on this branch: no `CompileTaskIdentity` type and no
`docs/reports/remaining-g01/` exist at `6c2782f`. Acceptance item 3 is therefore
discharged against the identity record that does exist,
`qcore_c_gateway::boolean_source::BooleanSourceIdentity`.

## 1. Inventory of the current transport (before this card)

All three Clang phases spawned `clang --driver-mode=gcc … -` and wrote the text
to the child's standard input. No phase ever called `Command::current_dir`, so
the effective working directory was whatever directory the gateway process
happened to be started in.

| Phase | Site | Input given to Clang | Text fed | Working directory |
|---|---|---|---|---|
| Preprocess (`-E -P`) | `crates/qcore-c-gateway/src/legacy_c.rs::validate_unrestricted_c11` | `-` (stdin) | the unit's raw source | gateway process cwd |
| Syntax (`-fsyntax-only`) | same function | `-` (stdin) | the *preprocessed* text | gateway process cwd |
| Typed AST (`-ast-dump=json`) | `crates/qcore-c-gateway/src/typed_c_ast.rs::ast_declarations` | `-` (stdin) | the *preprocessed* text | gateway process cwd |

`crates/qcore-audit/src/compile_db.rs` compensated for one consequence by
synthesizing a search path from the source's parent directory — `-I<parent>` in
`compiler_options_compatibility`, `-iquote<parent>` in `compiler_options`. That
compensation is *ordered after* the directory Clang uses for a standard-input
unit, so it loses whenever that directory holds a same-named header.

### Affected constructs, measured

Measured directly with `clang 21.1.8`, comparing a direct run on the real path
against the gateway's standard-input projection with the compensating search
path:

| Construct | Direct compile | Gateway (stdin) | Class |
|---|---|---|---|
| `__FILE__` | the input word as spelled | `"<stdin>"` | silent wrong value |
| `__FILE_NAME__` | `unit.c` | `"<stdin>"` | silent wrong value |
| `__BASE_FILE__` | the input word as spelled | `"<stdin>"` | silent wrong value |
| `#include "relative.h"` | includer's own directory wins | **the gateway's working directory wins** | silent wrong header |
| `__builtin_FILE()` / `__builtin_FILE_NAME()` | resolved at code generation against the real input | survives preprocessing, then resolved against `<stdin>` | no answer available |
| `#line 7 "custom.c"` | `"custom.c"`, line 8 | `"custom.c"`, line 8 | **not** divergent — self-contained |
| Diagnostic paths | real path | `<stdin>` | message text only; not projected |

`#line` is listed because the card named it: it is self-contained in the source
and expands identically under both transports, so it needs neither transport nor
rejection.

### The divergence in the pinned corpus

This is not hypothetical for the corpus this project audits. The recorded CMake
compile command for `lib/cmetrics/src/cmt_counter.c` contains

```
-D__FLB_FILENAME__=__FILE__ -D__CMT_FILENAME__=__FILE__
```

Preprocessing that unit both ways produces 5,564 lines either way, but:

* direct compile: 12 occurrences of the real absolute source path;
* standard-input projection: 12 occurrences of `"<stdin>"`.

## 2. Differential results

Two differentials were written and then run against the *old* transport to
confirm each one actually catches the defect (a temporary switch restored the
previous behavior; it is not part of the committed change).

**Byte-exact preprocessing differential** —
`crates/qcore-audit/tests/compile_options.rs::filename_sensitive_preprocessing_matches_the_recorded_compile_command`
compares the gateway's preprocessed output against a direct Clang run using the
same input word, the same directory and the same options. Against the old
transport it fails with:

```
left:  "const char *file = \"<stdin>\";\nconst char *name = \"<stdin>\";\n
        const char *base = \"<stdin>\";\nint observed = 5;\n"
right: "const char *file = \"src/unit.c\";\nconst char *name = \"unit.c\";\n
        const char *base = \"src/unit.c\";\nint observed = 5;\n"
```

**Verdict-level differential** —
`crates/qcore-audit/tests/configured_cli.rs::quoted_includes_resolve_against_the_source_not_the_audit_working_directory`
puts `effect.h` beside the source (expanding to a call to an uncontracted
`opaque()`) and a shadowing `effect.h` in a second directory, then runs the
`qcore-audit` binary from that second directory. Against the old transport the
shadow won and the audit published:

```
assertion `left == right` failed: started in …/shadow
  left: String("Safe")
 right: "Unknown"
```

That is the sharpest form of the defect: a **`Safe` verdict decided by where the
audit process was started**, for a translation unit the recorded compile command
never produces. It is repaired.

## 3. What was fixed, and what is rejected

`CSourceInput` (`crates/qcore-c-gateway/src/legacy_c.rs`) makes the input
identity an explicit part of a `C11TranslationUnit`, with three declared
variants.

### (a) Fixed by transport — `CSourceInput::DeclaredFile { spelled, directory }`

Clang is run from `directory` and handed `spelled` exactly as the compilation
database records the input argument, so `__FILE__`, `__FILE_NAME__`,
`__BASE_FILE__` and the includer-directory rule for quoted includes answer as
they do for the recorded compile command. This is transport only: no expansion
is computed, guessed or normalized by this project.

* The synthesized `-iquote <parent>` compensation is **removed** from
  `compiler_options`; the real includer directory supersedes it.
* The declared source bytes must equal the file's bytes, or the unit is
  rejected — analyzing text that differs from the file Clang reads would record
  one unit's identity against another unit's bytes.
* A non-absolute directory, or an input word that is not a source path, rejects.
* Path spelling is **preserved, not normalized**. `__FILE__` is the input word;
  rewriting it to a canonical path would be a different program. This card makes
  no claim that any path normalization establishes semantic equivalence.

Selected by `crates/qcore-audit/src/compile_db.rs::compiler_options`, i.e. the
configured audit driver (`crates/qcore-audit/src/lib.rs`,
`crates/qcore-audit/src/source_contract.rs`).

### (b) Fail closed — `CSourceInput::AnonymousText` (the default)

A unit with no declared file is still transported through standard input; its
declared identity is the text plus its options, and quoted includes resolve
against the gateway process's recorded working directory because an anonymous
text has no includer directory of its own. What it cannot answer, it rejects:

* a file-name macro expanding to `"<stdin>"` rejects with
  `LegacyCError::UnsupportedInputIdentity`, naming `__FILE__`, `__FILE_NAME__`
  and `__BASE_FILE__` and the reason (`this unit declares no input file`).

### (b) Fail closed — deferred source-location builtins, in every declared mode

`__builtin_FILE` and `__builtin_FILE_NAME` survive preprocessing and are
answered by the frontend, which this gateway *always* feeds preprocessed text
through standard input — including for a `DeclaredFile` unit. There is no
transport in this design that makes them true, so they are rejected by name
rather than projected under a guessed file. This is the one place the fix is a
rejection even though the input identity is declared, and it is stated as such
rather than papered over.

### Scanning

Detection runs over the **preprocessed** output, so a construct arriving through
an included header is seen exactly like one spelled in the source; string
literals and comments are lexed, so `"<stdin>"` matches only a real expansion.

### `CSourceInput::HistoricalStdin` — the pinned compatibility route

`compiler_options_compatibility` — the pinned Cycle 19 Fluent Bit adapter whose
185-unit artifacts are archived and are **not reproducible in this worktree**
(the audit needs an external checkout) — keeps its previous projection verbatim,
including `<stdin>` file names and the `-I<parent>` compensation, and is exempt
from both new rejections. This is a declared assumption recorded in the type's
own documentation, not an equivalence claim: those archived artifacts were
produced under this transport and moving them without being able to re-measure
them would be worse than naming the boundary. Migrating that route is open work.

Text *derived* from a unit (the ChunkIO probe units in
`compiler/src/verifier_plugins/chunkio/{memory_close,filesystem_chunk_close}.rs`
append a probe to the source) drops a `DeclaredFile` identity via
`CSourceInput::for_derived_text`, because those bytes are not the declared file.
That is fail-closed: the derived unit then rejects what it cannot answer.

## 4. Recorded identity equals effective identity (acceptance 3)

`boolean_source::input_identity` already bound `std::env::current_dir()`, which
remains correct for the syntax and typed-AST phases — they still read
preprocessed text from standard input in the gateway process's directory. A
declared file makes *preprocessing* run somewhere else on a named input, so the
identity now additionally binds that input under a `qcore.declared-input.v1:`
suffix carrying `(spelled, directory)`. Every path the tools actually saw is
recorded, and no recorded path is one the tools did not see.

The suffix is appended only for `DeclaredFile`, so units transported through
standard input keep their historical identity bytes — the same convention the
existing `qcore.ordered-preprocessor.v1:` suffix uses.

## 5. Tests added (6)

`crates/qcore-audit/tests/compile_options.rs` (3):

1. `filename_sensitive_preprocessing_matches_the_recorded_compile_command` —
   byte-exact equality with a direct Clang run over `__FILE__`,
   `__FILE_NAME__`, `__BASE_FILE__` and a quoted include shadowed on the regular
   search path; also pins that the historical projection answers `<stdin>` three
   times, so the divergence stays visible rather than being deleted.
2. `units_without_a_declared_file_reject_constructs_they_cannot_answer` —
   anonymous file-name macros reject naming the macro family; `__builtin_FILE`
   rejects under both anonymous and declared identities; the historical route
   still accepts both.
3. `a_declared_input_must_be_the_file_that_holds_the_declared_source` — changed
   bytes, a removed file, a relative directory and an option-shaped input word
   all reject.

`crates/qcore-audit/tests/configured_cli.rs` (3):

4. `quoted_includes_resolve_against_the_source_not_the_audit_working_directory` —
   the false-`Safe` regression above, run from both the shadowing directory and
   a neutral one.
5. `source_file_name_macros_answer_with_the_recorded_input` — end-to-end smoke
   that a `__FILE__` unit still audits through the configured CLI.
6. `deferred_source_location_builtins_reject_instead_of_answering_for_the_wrong_file` —
   the rejection is reported by the CLI, naming the builtin.

The existing fixtures now publish the unit's text at the path its compile
command records, since a declared input is analyzed as the file Clang reads.

## 6. Standing gate

| Gate | Result |
|---|---|
| `cargo test --workspace` | **825 passed**, 0 failed (819 before, +6 new) |
| `cargo fmt --all -- --check` | clean |
| `cargo clippy --all-targets -- -D warnings` | clean |
| `cargo run -p qcore-compiler --example reevaluation_probe` | exit 0 |
| `cargo run -p qcore-compiler --example reevaluation_followup` | exit 0 |
| `cd providers && cargo test --workspace --locked --offline` | **45 passed**, 0 failed |
| `python3 -m unittest discover -s scripts/tests` | **43 passed** |
| `cd proof && lake build` | success (62 jobs) |
| `python3 scripts/check_lean_audits.py` | **20 Lean audits passed** |

## 7. CMetrics configured audit — unchanged

```sh
python3 -B providers/package.py build --output build/cycle19/provider-packages-unit \
  --target-dir providers/target/conformance-artifacts
python3 -B scripts/prepare_cmetrics_clang_audit.py --fluent-bit <pinned checkout>
cargo run --quiet -p qcore-audit -- contract/profiles/cmetrics-counter/audit.json
```

Result, identical to the pre-change baseline:

```
cmetrics/lib/cmetrics/src/cmt_counter.c: Unsafe (7/7)
  resource diagnostics: leak=2, unknown_alias=2, unsupported=36
source contracts: 1 checked function contract(s) / 6 Unknown; no whole-unit promotion
1 units: 0 Safe / 1 provisional Unsafe / 0 Unknown / 0 data-only / 0 gateway failures
```

Console output diffs clean against the baseline. **No diagnostic changed**, per
function or otherwise: `cmt_counter_add`'s three reported preprocessed lines are
still 5522, 5526 and 5529, and the remaining 37 are byte-identical, because the
`-E -P` output has the same 5,564 lines under both transports — only the
contents of 12 string literals moved from `"<stdin>"` to the real source path.

The serialized report has exactly **10 changed JSON paths**, all identity
digests belonging to the single checked function `cmt_counter_allow_reset`:

| Path | Why |
|---|---|
| `…/import_identity/preprocessed_source_digest` | the 12 `__FILE__` expansions are now the real path |
| `…/import_identity/configuration_digest` | now binds the declared input (`qcore.declared-input.v1:`) |
| `…/evidence/unit/build_digest`, `…/task/unit/build_digest`, `…/task/contract_sources/qcore:boolean-field-import:v1` | derived from the two digests above |
| `…/bounds/resource_contracts:…` (removed/added), `…/evidence/coverage[0]`, `…/task/obligations[0]/obligation_id` | the obligation id is derived from the same identity |
| `…/witness/task_digest` | derived from the task |

`cmt_counter_allow_reset` stays **checked**; the other six functions stay
**Unknown**; the raw unit stays **provisional Unsafe** and **unpromoted**.

## 8. Open boundaries

* The pinned Cycle 19 compatibility route still runs on `HistoricalStdin`; its
  185-unit artifacts were not re-measured and are not reproducible here.
* Syntax and typed-AST phases still read preprocessed text from standard input.
  That is why the source-location builtins are rejected rather than transported;
  a single-invocation frontend path is not attempted here.
* `analyze_legacy_c11` remains a text-only entry point with no declared file.
* Arbitrary compiler arguments, response files and compiler substitution stay
  out of scope, unchanged.
* Filename sensitivity beyond the constructs inventoried above (for example
  `__has_include` search behavior under exotic search-path configurations) is
  not surveyed.
