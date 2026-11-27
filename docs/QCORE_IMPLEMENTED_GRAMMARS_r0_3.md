# Quirin implemented input grammars — revision 0.3

> **Name.** The language is **Quirin**; its command is `quirin`. The `QCore`
> spelling is retained for the core component — Rust namespaces, the emitted-C
> surface, the diagnostics codes, the legacy `.qcore` alias and this file's name —
> and those component identifiers are unchanged by the branding. Current contract inputs use `.qv`, executable/library inputs use `.qr`, and native ABI manifests use `.qrlib`; legacy names remain warned compatibility aliases.

> **Repository integration:** 2026-09-07. Normative requirements are adopted from
> the supplied review; implementation status is reconciled with `main` in the
> [acceptance ledger](reviews/r0_3/REVIEW.md). The original archive findings
> are historical observations, not a claim that later merged repairs are absent.
> Full revision-0.3 conformance is not claimed.

Historical `.qcore`/`.qc` census counts below describe their original checkpoints. Current files carry `.qv`, `.qr`, or `.qrlib` names while preserving those recorded source bytes and observations.

- **Source scope:** Supplied archive review, reconciled with the implementation records in the [acceptance ledger](reviews/r0_3/REVIEW.md), the [language specification](QCORE_SPEC_r0_3.md), and the [contract specification](QCORE_CONTRACT_SPEC_r0_3.md).
- **Purpose:** Specify the exact input grammars, profile selection gates, and semantic side conditions accepted by each implemented compiler, contract loader, and audit consumer — distinguishing implemented subsets from the broader target [design BNF](QCORE_DESIGN_GRAMMAR_r0_3.bnf).

---

## EBNF notation, lexical primitives, and module modes

Every grammar block in this reference uses ISO-style Extended Backus–Naur Form (EBNF) over the token stream produced by `qcore_frontend::lexer` (or the dedicated manifest/JSON reader noted by the section). Lexical recognition or structural parsing alone never establishes semantic admissibility; each route enforces its own total-or-reject lowering and model checks.

### Meta-syntax conventions

| Notation | Meaning |
|---|---|
| `"token"` | Exact keyword, operator, or punctuation literal |
| `A , B` | Concatenation (`A` followed by `B`) |
| `A \| B` | Alternation (`A` or `B`) |
| `[ A ]` | Optional occurrence (zero or one `A`) |
| `{ A }` | Repetition (zero or more occurrences of `A`) |
| `( A )` | Grouping |
| `A+` | One or more occurrences of `A` (equivalent to `A , { A }`) |
| `(* note *)` | Normative inline annotation or cross-reference |
| `;` | End of EBNF production rule |

### Lexical primitives

| Terminal | Lexical form and admissibility rules |
|---|---|
| `IDENT` / `NAME` | ASCII identifier `[A-Za-z_][A-Za-z0-9_]*` unless a consumer narrows it (for example, `id` in formal contract clauses requires `[a-z][a-z0-9_]*` of 1–128 bytes). |
| `NAT` / `INTEGER` / `DIGITS` | Unsigned decimal digit sequence `[0-9]+`. Signed positions write `[ "-" ], NAT` explicitly. Consumers range-check values against their target domain (`i32`, `u32`, `u64`, `i128`, `u128`, or unbounded `integer`) and refuse overflow rather than wrapping. |
| `FLOAT` | `DIGITS, ".", DIGITS, [ EXPONENT ] \| DIGITS, EXPONENT`, where `EXPONENT = ( "e" \| "E" ), [ "+" \| "-" ], DIGITS` (one token when a digit follows the optional sign; `1.5e` and `1e+` lex as a number followed by identifier `e`). |
| `STRING` / `STRING-TOKEN` | Double-quoted literal `"..."`. Relative import paths, inclusion directives, and review notes require raw single-line strings with no backslash escapes or control characters; `source.string.v1`, `source.http.v1`, and `G06b` decode `\n`, `\t`, `\r`, `\0`, `\\`, `\"`, and `\xHH` into validated UTF-8 bytes. |
| `BOOL` | `"true" \| "false"`. |
| Comments & trivia | Whitespace, `//` line comments, and non-nested `/* ... */` block comments are skipped in `.qr` and `.qv` sources. Native library manifests (`.qrlib`) use `#` to end-of-line comments. Byte spans are half-open UTF-8 byte ranges. |

### Source file extensions and module modes

| Extension | Legacy alias | Required mode / role | Notes |
|---|---|---|---|
| `.qr` | `.qcore` | `module executable;` (default) or `module library;` | Runtime/library source checked or compiled to C11 by `quirin check` / `quirin emit`. Incompatible mode/suffix or ambiguous stem pairing refuses at `Q403`. |
| `.qv` | `.qcore`, `.qc` | `module contract;` | Entry-free external C contract, premise, bounds, null-dereference, cell-protocol, or observation-policy module. |
| `.qrlib` | `.qcorelib` | `native_library manifest v1;` or `v2;` | Native C library artifact, ABI premise, header, and symbol-table linkage manifest (`G02d.2`, `G02d.2.1`). |
| `.qgd` | — | `guarded_decomposition_v1;` | Standalone readable guarded-decomposition document (`G05-guarded`). |
| `.json` | — | `schema_version: 1` or `2` | Audit service configuration (`G08`), contract load-path / resource manifests (`G05a`), or recorded target (`G02t`). |

### Bundled `#import` capability registry

Bundled semantic capabilities require an explicit, file-local `#import <module>;` directive (bare `import <module>;` remains a compatibility alias). Capabilities never transfer across files in a multi-file compilation and never imply one another:

| Bundled module | Gated capability | Missing-import diagnostic | Primary grammar |
|---|---|---|---|
| `std.ffi.unsafe` | `struct! { ... }` C layout import and file-wide unsafe-FFI assumption | `Q032` | [G02u](#g02u-imported-c-structs-sourceffiv1) |
| `std.concurrency` | `concurrency checked;`, `thread scope`, `spawn`/`join`, `mutex`, `epoch`, `Buffer` intrinsics | `Q015` | [G02a](#g02a-scoped-copy-task-execution)–[G02f](#g02f-flat-epoch-bound-over-a-checked-task-region), [G09](#g09-concurrent-field-footprint-questions) |
| `std.extern` | Calling declared `extern fn` symbols | `Q022` | [G02d](#g02d-bounded-extern-calls), [G02i-extern-io](#g02i-composed-extern-calls-and-rendered-lines), [G02u](#g02u-imported-c-structs-sourceffiv1) |
| `std.extern.unsafe` | Declaring uncontracted `unsafe extern fn` waivers | `Q032` | [G02d.1](#g02d1-unchecked-linkage) |
| `std.fmt` | Scalar rendering selectors (`dec`, `hex`) on line statements | `Q015` | [G02g](#g02g-rendered-lines-stdfmt-stdio-and-stdlogger), [G02t](#g02t-fixed-width-scalar-types) |
| `std.io` | Standard-output line statements (`print ...;`, `print(expr);`) | `Q015` / `Q020` | [G02g](#g02g-rendered-lines-stdfmt-stdio-and-stdlogger), [G02s](#g02s-scalar-functions-and-explicit-output-sourcescalar-callsv1) |
| `std.logger` | Standard-error `log`, `flush`, and diagnostic threshold `log_level` | `Q015` | [G02g](#g02g-rendered-lines-stdfmt-stdio-and-stdlogger), [G02k-event](#g02k-the-checked-event-surface) |
| `std.buffer` | Owned byte buffers (`bytes`, `extent`, `lend(b, lo, hi)`, byte indexing) | `Q015` | [G02k-buffer](#g02k-an-owned-byte-buffer-stdbuffer), [G02r](#g02r-resource-aware-byte-helpers-sourceresource-callsv1) |
| `std.string` | Immutable UTF-8 string views (`"..."`, `len`, slicing, `concat`, `borrow_utf8`) | `Q015` | [G07.9](#g079-immutable-utf-8-string-views-v1-sourcestringv1), [G07.10](#g0710-a-checked-http11-message-subset-sourcehttpv1) |
| `std.event` | Single-threaded event reactor (`event open/close`, `endpoints`, `watch`, `dispatch`) | `Q015` | [G02k-event](#g02k-the-checked-event-surface), [G07.11](#g0711-a-checked-socket-as-a-reactor-registered-event-source) |
| `std.socket` | Loopback TCP/UDP sockets (`tcp`, `udp`, `bind`, `listen`, `connect`, `accept`, `send`/`recv`, `disjoint`) | `Q015` | [G07.11](#g0711-a-checked-socket-as-a-reactor-registered-event-source) |
| `std.http` | Checked HTTP/1.1 messages (`request`, `response`, `header`, `content`) | `Q015` | [G07.10](#g0710-a-checked-http11-message-subset-sourcehttpv1) |

---

## Profile-to-grammar navigation and table of contents

### Part I — Structural Source Envelope, Formal Clauses, and Module Bundles

| Section | Profile / Consumer | Mode | Diagnostic band | Summary |
|---|---|---|---|---|
| [G01](#g01-structural-source-envelope) | `source.structural` | `.qr`, `.qv` | `Q000`, `Q010`–`Q015` | Module headers, top-level declarations, contract blocks, and per-file capability gates |
| [G01a](#g01a-runtime-import-bundle) | Runtime bundle resolver | `.qr` | `Q014`, `Q403` | Bundled `#import std.*;` and recursive relative `#import "./file.qr";` resolution |
| [G01p](#g01p-typed-contract-propositions-graph-version-3) | Hypothesis graph v3 | `.qv` | `QF002` | `quant proposition #id(...) = formula;` first-order integer/uninterpreted predicates |
| [G01c](#g01c-explicit-entry-and-exit-conditions) | Hypothesis graph v4 / Boolean profile | `.qv` | — | `quant condition entry\|normal\|any\|error(E) #id(...) = formula;` |
| [G01t](#g01t-typed-theorem-instantiation-obligations) | `quant_theorem::instantiate` | `.qv` | — | `quant theorem #id(...) using ref(...) requires (...) yields formula;` |
| [G01e](#g01e-typed-conditional-own-operation-effects) | `quant_effect` | `.qv` | — | `quant effect <sel> #id(...) from #s to #s' when ... reads (...) writes (...) frame ... = ...;` |

### Part II — Sequential `.qr` Execution Grammars

| Section | Profile ID | Required `#import` | Diagnostic band | Summary |
|---|---|---|---|---|
| [G02](#g02-source-execution-profile) | `source.exec12` | *(none)* | `Q020`–`Q021`, `Q1xx`–`Q5xx` | Base sequential model (`qcore_acquire`, `qcore_check`, `drop`, `move`, zero-arg calls) |
| [G02s](#g02s-scalar-functions-and-explicit-output-sourcescalar-callsv1) | `source.scalar-calls.v1` | `std.io` *(for IO)* | `Q020`–`Q021`, `Q028`, `Q030` | Direct acyclic scalar/record functions, checked arithmetic, `ArithmeticError`, `print` |
| [G02t](#g02t-fixed-width-scalar-types) | Cross-route scalar tier | *(per route)* | `Q020` *(+ route band)* | Fixed-width integers (`bool`, `i8`..`usize`), `as` casts, `ArithmeticError`, IEEE `f32`/`f64` |
| [G07.13](#g0713-the-first-source-level-aggregate-sourcerecordv1) | `source.record.v1` | *(none)* | `Q051`–`Q055` / `Q020` / `Q600` | Nominal `type R { ... }` records, nested paths, field writes, owned `bytes`/`Buffer` fields |
| [G02k-buffer](#g02k-an-owned-byte-buffer-stdbuffer) | `source.buffer.v1` | `std.buffer` | `Q051`–`Q057` | Owned `bytes(N)`, `extent(b)`, indexed loads/stores, copy binds, disjoint `lend(b, lo, hi)` |
| [G02r](#g02r-resource-aware-byte-helpers-sourceresource-callsv1) | `source.resource-calls.v1` | `std.buffer` | `Q020`–`Q021`, `Q051`–`Q057`, `Q100` | Direct helper functions passing `own<bytes> use one`, `borrow<bytes>`, and `lend<bytes>` |
| [G07.9](#g079-immutable-utf-8-string-views-v1-sourcestringv1) | `source.string.v1` | `std.string` | `Q068`–`Q073`, `Q075`–`Q079` | Immutable UTF-8 views, `len`, `s[lo:hi]`, static `concat`, range-wise `borrow_utf8` |
| [G02h](#g02h-the-pipe-operator-and-the-operator-token-family) | `source.pipe.v1` *(desugaring)* | *(none)* | `Q036`–`Q038` | Parse-time `\|>` pipeline desugaring and operator token gate (`\|`, `&`, `^`, `\|\|`, `&&`) |
| [G02i-branch](#g02i-the-source-branching-construct) | `source.branch.v1` | *(none)* | `Q044`–`Q046`, `Q103`, `Q202` | Two-sorted check-time `if / else` conditions and four-model custody-join arms |
| [G07.12](#g0712-the-checked-bounded-counted-loop-sourceloopv1) | `source.loop.v1` | *(none)* | `Q097`–`Q099` | Bounded `for i in 0..BOUND` loops and `[#unroll]` three-layer iteration budgets |
| [G02g](#g02g-rendered-lines-stdfmt-stdio-and-stdlogger) | `source.std-io.v1` | `std.fmt`, `std.io`, `std.logger` | `Q028`–`Q031` | Deterministic rendered scalar lines (`print`, `log`, `log_level`, `flush`) |
| [G02k-event](#g02k-the-checked-event-surface) | `source.event.v1` | `std.event` | `Q058`–`Q062`, `Q074` | Single-threaded event reactor (`event open/close`, `endpoints`, `watch`, `wake`, `dispatch`) |
| [G07.11](#g0711-a-checked-socket-as-a-reactor-registered-event-source) | `source.socket.v1` | `std.socket` *(+ `std.event`)* | `Q080`–`Q089`, `Q664` | Checked loopback TCP/UDP sockets, IPv4/IPv6 endpoints, reactor loans, `disjoint` cells |
| [G07.10](#g0710-a-checked-http11-message-subset-sourcehttpv1) | `source.http.v1` | `std.http`, `std.socket` | `Q090`–`Q096`, `Q664` | Checked RFC 9112 HTTP/1.1 requests/responses, static framing, verified `Content-Length` |

### Part III — Quantitative and Concurrent `.qr` Execution Grammars

| Section | Profile ID | Required `#import` / Marker | Diagnostic band | Summary |
|---|---|---|---|---|
| [G03](#g03-source-parameter-usage-grades) | Usage grade parser | *(any `.qr` / `.qv`)* | `Q100` | `use zero \| one \| affine \| relevant \| many \| N \| [lo, hi]` parameter grades |
| [G04](#g04-closed-quantitative-indices) | `index.closed` | *(any `.qr` / `.qv`)* | `Q001`, `Q016` | `quant<...>` and `quant[\| ... \|]` credit/live indices and `where` constraints |
| [G02a](#g02a-scoped-copy-task-execution) | `source.tasks.copy.v1` | `std.concurrency` + `concurrency checked;` | `Q020`, `Q300`, `Q500`, `Q600` | `thread scope`, `spawn`/`join`, `Result` recovery, scalar `copy`, and task records |
| [G02v](#g02v-closures-with-explicit-capture-lists) | Closure lifting (`source.tasks.*`) | `std.concurrency` + `concurrency checked;` | `Q000`, `Q011`, `Q013`, `Q038`, `Q600` | Inline `spawn s \|captures\| -> T { ... }` closures lifted to hidden leaf workers |
| [G02b](#g02b-owned-buffer-task-execution) | `ScopedOwnedTasksV1` | `std.concurrency` + `concurrency checked;` | `Q020`, `Q300`, `Q600` | Heap-allocated `own<Buffer<T>>` move captures, `qcore_buffer_new`/`read`, task ordinals |
| [G02bb](#g02bb-scoped-read-loan) | `ScopedReadLoansV1` | `std.concurrency` + `concurrency checked;` | `Q020`, `Q600` | `borrow<Buffer<T>>` worker parameters and `read PLACE share FULL` scoped loans |
| [G02bc-shares](#g02bc-counted-read-shares-and-the-checked-buffer-write) | `ScopedReadSharesV1` | `std.concurrency` + `concurrency checked;` | `Q020`, `Q600` | Fractional `share S` (`FULL = 1024`), `lend PLACE lo..hi` write leases, byte/whole writes |
| [G02bc-errors](#g02bc-tagged-error-payload) | `ScopedErrorPayloadV1` | `std.concurrency` + `concurrency checked;` | `Q300`, `Q600` | Uniform `Variant(own<Buffer>)` error payloads and `qcore_error_payload` extraction |
| [G02c](#g02c-guarded-fixed-footprint-mutex) | `source.mutex.guard.v1` | `std.concurrency` | `Q600`, `Q610`–`Q627` | Lexical `with lock`, `open`/`close`, `rank`, `grow`, `fact`, `under`, `share ... under` |
| [G02e](#g02e-flat-epoch-quantitative-bound) | `source.epoch.bound.v1` | `std.concurrency` | `Q600`, `Q630`–`Q641` | Check-only `thread scope s epoch { ... }` memory/effort accounting over declared shapes |
| [G02f](#g02f-flat-epoch-bound-over-a-checked-task-region) | `source.epoch.tasks.v1` | `std.concurrency` + `concurrency checked;` | `Q300`–`Q309`, `Q600`, `Q630`–`Q641` | Epoch plan reconciled with real scoped tasks, credit error recovery, `error region` epilogue |
| [G02j](#g02j-composed-scoped-tasks-and-rendered-lines) | `source.std-io-tasks.v1` | `std.concurrency` + output modules | `Q050`, `Q030`, `Q600` | Scoped tasks composed with parent/worker rendered lines (`qcore.obs-reordering.v1`) |
| [G09](#g09-concurrent-field-footprint-questions) | `source.bitfield-concurrency.v1` | `std.concurrency` + `concurrency checked;` | `Q600`, `Q661` | Property-only `field<off, sz>` / `bits<...>` concurrent write disjointness check |

### Part IV — Native C Interoperability and `.qrlib` Linkage Grammars

- [G02d. Bounded extern calls (`source.extern-call.v1`)](#g02d-bounded-extern-calls) (`Q022`–`Q027`)
- [G02i-extern-io. Composed extern calls and rendered lines (`source.std-io-extern.v1`)](#g02i-composed-extern-calls-and-rendered-lines) (`Q043`)
- [G02d.1. Unchecked linkage (`std.extern.unsafe`)](#g02d1-unchecked-linkage) (`Q032`–`Q035`)
- [G02d.2. Native-library manifest v1 (`source.native-library.v1`)](#g02d2-the-native-library-manifest) (`Q039`–`Q042`, `Q047`–`Q049`, `Q063`–`Q064`)
- [G02d.2.1. Generated native-library manifest v2 (`.qrlib`)](#g02d21-the-generated-manifest-v2) (`Q039`)
- [G02d.3. Source library binding (`extern library NAME;`)](#g02d3-the-source-library-binding) (`Q065`–`Q067`)
- [G02d.4. Delayed custody results (`source.quant-variant.v1`)](#g02d4-delayed-custody-results-sourcequant-variantv1)
- [G02u. Imported C structs (`source.ffi.v1`, `std.ffi.unsafe`)](#g02u-imported-c-structs-sourceffiv1) (`Q022`–`Q035`, `Q051`–`Q053`, `Q063`–`Q067`)

### Part V — `.qv` Contract Loader and Property Clause Grammars

- [G05. Resource contract loader v1 (`contract.resource.v1`)](#g05-resource-loader-v1) (`QF001`, `QF002`, `QF005`)
  - [G05-formula. Symbolic premise statements](#g05-formula-symbolic-premise-statements) (`QF001`)
  - [G05-flow. Proposed premise expressions (`resource_premise_flow_v1`)](#g05-flow-proposed-premise-expressions)
  - [G05-lifecycle. Typed symbolic lifecycle clauses (`resource_lifecycle_v1`)](#g05-lifecycle-typed-symbolic-lifecycle-clauses)
  - [G05-guarded. Standalone readable guarded decomposition (`.qgd`)](#g05-guarded-standalone-readable-guarded-decomposition)
  - [G05-guarded-source. Explicit guarded source embedding](#g05-guarded-source-explicit-guarded-source-embedding) (`QF003`)
- [G05a. Contract inclusion and inheritance (`#include`, `contract inherit`)](#g05a-contract-inclusion-and-inheritance) (`QI001`–`QI020`)
- [G05b. Bounds contract v1 (`bounds_contract_v1`)](#g05b-bounds-contract-v1)
- [G05c. Quantified index checks (`quant checks`)](#g05c-quantified-index-checks)
- [G05d. Null-dereference, dereference shorthand, and `quant live` checks](#g05d-null-dereference-checks)
- [G05e. Null-property member, result, and result-range declarations](#g05e-null-property-member-and-result-declarations)
- [G05f. Scoped member invariants (`[[stable]]`, `[[scope]]`)](#g05f-scoped-member-invariants)
- [G05g. Outer contract v1 (`outer_contract_v1`, `quant #outer`)](#g05g-outer-contract-v1) (`QF004`)
- [G05h. Local condition and registered-lifetime clauses (`[[local]]`)](#g05h-local-condition-and-registered-lifetime-clauses)
- [G05i. Declared cell protocols (`quant cell`)](#g05i-declared-cell-protocols)
- [G05k. Callee frames over a declared cell protocol (`#untouched`)](#g05k-callee-frames-over-a-declared-cell-protocol)
- [G05j. C library result sets and written sets (`result_set`, `written_set`)](#g05j-c-library-result-sets)
- [G05l. List-finiteness premises (`quant list ... |-> #finite`)](#g05l-list-finiteness-premises)
- [G05m. Contract linkage v1 (`linkage_contract_v1`, `quant #link`, `premises { link { ... }; };`)](#g05m-contract-linkage-v1) (`QL001`–`QL009`)

### Part VI — Observation Policies, Service Configuration, Existing-C Subsets, and Extended BNF

- [G06. Observation predicate grammar (`contract.observation`)](#g06-observation-predicate-grammar)
  - [G06a. Bounded observed timed-wait contract consumer](#g06a-bounded-observed-timed-wait-contract-consumer)
  - [G06b. Typed executable policy predicates (`check_typed_contract_facts`)](#g06b-typed-executable-policy-predicates)
- [G08. Source-contract service configuration v1 & v2 (JSON)](#g08-source-contract-service-configuration-v1)
- [Configured C audit composition (`source_contracts`)](#configured-c-audit-composition)
- [G10. Existing C: C11 `_Generic` selections](#g10-existing-c-generic-selections)
- [G11. Existing C: indexed `child`/`next` lookup](#g11-existing-c-indexed-childnext-lookup)
- [G12. Existing C: checked call-free condition composition](#g12-existing-c-checked-call-free-condition-composition)
- [G07. Extended design grammar and conformance boundary (`QCORE_DESIGN_GRAMMAR_r0_3.bnf`)](#g07-extended-grammar-and-conformance)

---

# Part I — Structural Source Envelope, Formal Clauses, and Module Bundles

<a id="g01-structural-source-envelope"></a>

## G01. Structural source envelope

### Purpose and module modes

`G01` (`source.structural`) defines the top-level file envelope recognized by the Quirin front end before route-specific semantic elaboration. It governs module headers, bundled capability imports, runtime file imports, library bindings, top-level declaration categories, and structural `contract { ... }` blocks across `.qr` (executable or library) and `.qv` (contract) inputs.

- **Optional header:** A `module_header` may appear at most once per file and must precede all imports and declarations. Omitting the header selects `executable` mode.
- **Contract modules:** `module contract;` files are entry-free; declaring `contract` mode does not by itself install an observation policy or a C resource summary. Contract inclusion across provider crates uses `#include` ([G05a](#g05a-contract-inclusion-and-inheritance)), resolved exclusively by the contract loader before `G01` runs.
- **Multi-file combination:** Multiple compilation inputs share one global declaration namespace, but header constraints and capability imports are checked **per file before combining declarations**. Combined compilation mode resolves as `executable` if any root is executable, else `library` if any root is a library, else `contract`.

### Consolidated EBNF

```ebnf
source          = [ module_header ], { top_level_item } ;
module_header   = "module", ( "executable" | "library" | "contract" ), ";" ;

top_level_item  = standard_import
                | runtime_import
                | library_binding
                | declaration ;

standard_import = [ "#" ], "import", IDENT, { ".", IDENT }, ";" ;
runtime_import  = "#", "import", QUOTED_RELATIVE_QR_PATH, ";" ;     (* §G01a *)
library_binding = "extern", "library", IDENT, ";" ;                 (* §G02d.3 *)

contract        = "contract", "{", { clause }, "}" ;
clause          = "requires", NONEMPTY_EXPRESSION_TOKENS, ";"
                | "ensures", selector, NONEMPTY_EXPRESSION_TOKENS, ";"
                | "quant", selector, NONEMPTY_EXPRESSION_TOKENS, ";"
                | quant_formal_clause                               (* §G01p/c/t/e, Part V *)
                | linkage_clause ;                                  (* §G05m *)
selector        = "normal"
                | "any"
                | "error", "(", NONEMPTY_TYPE_TOKENS, ")" ;
```

### Structural retention vs. semantic admissibility

- **Balanced token regions:** `declaration`, `NONEMPTY_EXPRESSION_TOKENS`, and `NONEMPTY_TYPE_TOKENS` denote balanced structural parser regions rather than universally type-checked expressions. Broad statement and type classification retains forms that downstream execution routes subsequently reject; trial parsing is therefore **not** a sound way to probe executable capabilities.
- **Recognized declaration categories:**
  1. Concurrency profile marker (`concurrency checked;`)
  2. Verifier scope (`verify scope ...`)
  3. Nominal dimension (`dimension D : Idx + Ordered;`)
  4. Quantitative axis (`quant axis a : credit<...> | live<...> | cost.total<...> | cost.peak<...>;`)
  5. Type alias (`type Name = ...;`) and source record (`type Name { ... }`, [G07.13](#g0713-the-first-source-level-aggregate-sourcerecordv1))
  6. Resource type (`resource type Name { ... }`)
  7. Nominal error type (`error Name { Variant; Variant(own<Buffer>); }`)
  8. Constant (`const ...`)
  9. Imported C struct (`struct! { ... }`, [G02u](#g02u-imported-c-structs-sourceffiv1)) and source library binding (`extern library Name;`, [G02d.3](#g02d3-the-source-library-binding))
  10. External function (`[unsafe] extern fn ...;`) and source function (`fn ... { ... }`)
- **Structured task records (`frontend_tasks.rs`):** Structural parsing records `spawn S worker(...)` with ordered `copy`, `move`, `read ... share ...`, `lend ... LOW..HIGH`, and `write` place captures, `join move H`, and trailing `?` propagation, resolving live scopes, matching-arity source workers, capture roots, and handle binders. Typed statements preserve fresh binder IDs after resolving initializers. Structural task records do not widen `G02`'s sequential profile; actual task execution is checked on [G02a](#g02a-scoped-copy-task-execution)–[G02bc-errors](#g02bc-errors).
- **Per-file bundled import rules:**
  - Imports may appear after the optional `module_header` and between top-level declarations; they apply to the entire file in which they are written and are idempotent when repeated.
  - Unknown bundled module names refuse at `Q014`.
  - Using gated syntax without its file-local `#import` refuses (`Q015` for `std.concurrency`, `std.fmt`, `std.io`, `std.logger`, `std.buffer`, `std.string`, `std.event`, `std.socket`, `std.http`; `Q022` for `std.extern`; `Q032` for `std.extern.unsafe` and `std.ffi.unsafe`), including inside uncalled functions and non-executable modules.
  - Importing vocabulary never discharges operation checks: scoped-task modules still require `concurrency checked;`. Package resolution, import aliases, wildcards, and reexports are unsupported.

### Examples

```qcore
#import std.concurrency;
concurrency checked;

fn worker(x: i32) -> i32 {
    return x;
}

fn main() -> i32 effects { throws<SpawnError>, spawn, join, may_block } {
    thread scope s {
        let h = spawn s worker(copy 7)?;
        let out = join move h?;
        return out;
    }
}
```

### Implementation & Provenance

- **Implementation:** `crates/qcore-frontend/src/frontend.rs` (declaration validation, contract parsing, parameter parsing, `elaborate_source`), `crates/qcore-frontend/src/frontend_tasks.rs`, `crates/qcore-frontend/src/module_imports.rs`.
- **Historical reference:** [Task foundations record](reports/w6-source-task-foundations/README.md).

---

<a id="g01a-runtime-import-bundle"></a>

### G01a. Runtime import bundle

#### Purpose and module mode

`G01a` specifies top-level bundled module imports and relative `.qr` file imports resolved into a deterministic, acyclic source bundle before route checking.

#### Consolidated EBNF

```ebnf
standard_import = [ "#" ], "import", IDENT, { ".", IDENT }, ";" ;
runtime_import  = "#", "import", QUOTED_RELATIVE_RUNTIME_PATH, ";" ;
```

#### Semantic side conditions, bounds, and resolution rules

- **Path syntax:** `QUOTED_RELATIVE_RUNTIME_PATH` must be non-empty, contain no backslash escapes, backslashes, control characters, or drive prefixes, and name a relative `.qr` file. Omitting the extension appends `.qr` (`"./helper"` resolves `"./helper.qr"`); any explicit extension other than `.qr` refuses. Bare `import "./helper";` without `#` is rejected.
- **No search fallback:** Directives resolve recursively relative to the importing file's own directory, with no package, directory, environment, or legacy-extension (`.qcore`) fallback. Contract source (`module contract;`) must not use runtime file imports; contract inclusion remains `#include` ([G05a](#g05a-contract-inclusion-and-inheritance)).
- **Frontend vs. driver entry points:**
  - `inspect_source_imports` extracts mode, header, and import metadata only.
  - `parse_source_program` rejects unresolved runtime `#import "..."` directives (`Q000`).
  - `compile_files` and `compile_files_with_registry` resolve against caller-supplied snapshots only, treating all supplied files as roots.
  - `compile_source_bundle_with_registry` separates explicit roots from an available snapshot pool and selects either snapshot-only virtual resolution or filesystem loading (used by CLI `quirin check` and `quirin emit`).
- **Filesystem & snapshot hygiene:** Supplied snapshots take precedence over disk at the same lookup identity; duplicate snapshot identities refuse even if byte-identical. Filesystem mode resolves physical symlink targets before simplifying `..` and refuses unresolved parent steps.
- **Mode and capability isolation:** Unmarked or `module library;` dependencies contribute declarations in source-order depth-first postorder (every dependency completed before its importer), but never contribute executable root intent (`module executable;` or `module contract;` dependencies refuse). Every file's `#import std.*;` capabilities are checked independently: calling an `extern fn` declared in an imported helper still requires `#import std.extern;` in the calling file (`Q022`).
- **Bundle bounds and diagnostics:** Maximum **1,024** supplied/selected modules, **16 MiB** total retained UTF-8 source bytes, and recursive traversal depth **128**. Missing, unreadable, cyclic, ambiguous, wrong-mode, or bound-exceeding imports refuse at `Q014` (filename/mode incompatibility refuses at `Q403`).

#### Example

```qcore
// main.qr
#import std.io;
#import "./math_helpers";
#import "../shared/constants.qr";

fn main() -> i32 effects { io } {
    let v = add_offset(10);
    print(v);
    return 0;
}
```

#### Implementation & Provenance

- **Specification:** [Runtime source imports](QCORE_SPEC_r0_3.md#runtime-source-imports).
- **Implementation:** `crates/qcore-frontend/src/module_imports.rs`, `crates/qcore-verification/src/driver.rs` (`CompiledModule::source_bundle`).

---

<a id="g01p-typed-contract-propositions-graph-version-3"></a>

### G01p. Typed contract propositions (graph version 3)

#### Purpose and module mode

Inside a function `contract { ... }` block (typically in `.qv` contract/model modules), `quant proposition` declares a named, explicitly sorted first-order formula over mathematical integers and uninterpreted value/predicate/function parameters.

- **Sole hypothesis atom form:** Since card `SP6-SYMBOLIC-HYPOTHESIS-ATOMS`, `quant proposition` is the **only** admitted hypothesis atom declaration; the legacy prose string form `resource_hypothesis(ID, STRING)` is refused on every route with `QF002` ([G05](#g05-resource-loader-v1)).
- **Premise rendering bridge:** A typed atom whose parameters are registered C04.18.9 terms in their rendered sorts (value sorts `obj`, `cls`, `fn`, `meth`, `res`, `run`, plus `#runs: predicate<fn, run>`) serves as a checked typed rendering of a symbolic premise formula ([contract spec §C04.18.9](QCORE_CONTRACT_SPEC_r0_3.md), `qcore_frontend::premise_formula::check_rendering`).
- **Obligation discipline:** Standalone formulas in a declared model are **entry obligations**, never self-establishing facts. Strict resource and bounds loaders refuse unbound standalone entry obligations rather than installing a summary that ignores them.

#### Consolidated EBNF

```ebnf
proposition = "quant", "proposition", "#", id, "(", [ parameters ], ")",
              "=", formula, ";" ;
parameters  = parameter, { ",", parameter } ;
parameter   = "#", id, ":", sort ;
sort        = id
            | "predicate", "<", [ ids ], ">"
            | "function", "<", ids, ">" ;
ids         = id, { ",", id } ;

formula     = "#", id, "(", [ arguments ], ")"
            | term, ( "==" | "!=" | "<" | "<=" | ">" | ">=" ), term
            | "!", formula
            | "(", formula, ")"
            | "(", formula, ( "&&" | "||" | "=>" ), formula, ")"
            | ( "forall" | "exists" ), "(", "#", id, ":", id, ")",
              "{", formula, "}" ;

arguments   = term, { ",", term } ;
term        = sum ;
sum         = product, { ( "+" | "-" ), product } ;
product     = primary, { "*", primary } ;
primary     = "#", id
            | INTEGER
            | "-", primary
            | "#", id, "(", [ arguments ], ")"
            | "(", term, ")" ;
```

#### Semantic side conditions and quantitative bounds

- **Identifiers and literals:**
  - `id` is 1–128 lowercase ASCII characters `[a-z][a-z0-9_]*` beginning with a letter. The prefix `legacy_` is reserved; duplicate formal IDs refuse.
  - `INTEGER` is `0` or a nonzero-leading decimal literal of at most 128 digits.
- **Sorting and scoping:**
  - `predicate` and `function` are higher-order sort constructors for formal parameters, never value sorts and never quantifiable inside `forall`/`exists`.
  - `predicate<S1, ..., Sn>` takes ordered value-sort arguments (`predicate<>` is nullary). `function<A1, ..., An, R>` takes argument sorts `A1..An` and returns value sort `R` (`function<R>` is nullary).
  - Every predicate and function symbol in `formula` or `term` must be an explicitly bound `#` parameter; calls require exact arity and matching value sorts.
  - `==` and `!=` require equal value sorts (`!=` lowers to negated equality). Arithmetic (`+`, `-`, `*`, unary `-`) and ordering (`<`, `<=`, `>`, `>=`) require the built-in value sort `integer` (mathematical unbounded $\mathbb{Z}$, never wrapping or asserting C overflow safety).
  - Free names, duplicate parameter names, shadowing of active `#` binders, prose strings, implicit symbols, and binary connectives without enclosing parentheses (`(A && B)`) refuse.
  - Uninterpreted quantifier value sorts may denote empty or infinite domains; no C width, field layout, constant value, or non-emptiness is inferred from parameter names.
- **Graph version 3 encoding:** One `quant proposition` creates one graph atom keyed by its `#id`. Any hypothesis graph containing formal atoms uses schema **version 3** and records ungrouped formal IDs in its canonical sorted `requirements` list (omitted when empty; graph versions 1 and 2 refuse formal atoms and `requirements`). Exact formal ASTs and `requirements` participate in graph digests and source replay.

| Resource / Complexity Bound | Maximum |
|---|---|
| Source bytes per `quant proposition` | 65,536 bytes |
| Formal parameters | 64 |
| Call arguments per application | 16 |
| Combined formula + term AST nodes | 256 nodes |
| AST depth (root at depth 0) | 32 |
| Parser work fuel (backtracking never replenishes) | 131,072 steps |

#### Example

```qcore
module contract;

extern fn counter_inc(c: ptr<mut, u8>) -> i32 contract {
    quant proposition #monotonic_step(
        #before: integer,
        #after: integer,
        #valid: predicate<integer>
    ) = (#valid(#before) => ((#after == #before + 1) && (#after > #before)));
};
```

#### Implementation & Provenance

- **Cards:** `SP6-SYMBOLIC-HYPOTHESIS-ATOMS` (2026-10-26 formal hypothesis track).
- **Implementation:** `qcore_frontend::quant_proposition`, `qcore_schemas::quant_proposition`, `qcore_contract::hypothesis`, `quant_proposition_inventory`, `qcore_frontend::premise_formula::check_rendering`.
- **Lean model:** `QCore.QuantProposition`.

---

<a id="g01c-explicit-entry-and-exit-conditions"></a>

### G01c. Explicit entry and exit conditions

#### Purpose and module mode

Inside a function `contract { ... }` block, `quant condition` states a typed mathematical assertion anchored to an explicit function entry or exit selector (user decision 2026-10-26).

#### Consolidated EBNF

```ebnf
condition          = "quant", "condition", condition_selector,
                     "#", id, "(", [ parameters ], ")", "=", formula, ";" ;
condition_selector = "entry"
                   | "normal"
                   | "any"
                   | "error", "(", error_type, ")" ;
```

*(Here `id`, `parameters`, and `formula` — along with their AST, parameter, and parser-fuel bounds — are inherited verbatim from [G01p](#g01p-typed-contract-propositions-graph-version-3).)*

#### Semantic side conditions, graph v4 rules, and bounded Boolean consumer

- **Selector semantics:**
  - `entry`: caller prerequisite at function entry.
  - `normal`: postcondition on every normal C/Quirin return (a negative C integer return code is a `normal` return unless mapped by an error adapter).
  - `error(E)`: postcondition on the exact nominal typed error exit `E` (`error_type` is 1–1,024 bytes, non-blank, free of control characters, with no subtype hierarchy).
  - `any`: postcondition across every terminating `normal` or `error` exit.
  - **Partial correctness:** None of the four selectors asserts termination. Before/after states, return values, and trace projections must be passed as explicit `#` parameters and bound by the consumer; missing interpretations are never treated as negation evidence.
- **Cross-clause uniqueness and Graph v4:**
  - `qcore_contract::quant_condition::declaration` rejects duplicate `#id` names across `condition`, `proposition`, `effect`, and `theorem` clauses in the same contract.
  - Strict resource-v1, bounds-v1, and local-condition loaders refuse `quant condition` clauses.
  - Hypothesis-graph inspection records `entry` conditions as `entry-condition` atoms in **graph version 4**, unconditionally placing every `entry-condition` ID in canonical sorted `requirements` even if referenced by metadata groups/refinements (grouping cannot discharge an entry obligation; graph versions 1–3 refuse v4 atoms). Exit conditions are validated without assuming entry guarantees as implicit antecedents.
- **Bounded Boolean source-contract reader (`qcore_contract::boolean_condition_profile`):**
  - **Entry roles:** Admits exact typed formulas for modeled-object liveness, caller-exclusive access (`modeled(env, o)` over mapped profile objects; `during(env, e)` over the call interval), and Boolean prestate membership (`old_field` unit or `old_busy`/`old_lock` `i32` in `{0, 1}`). Every entry definition's canonical SHA-256 is recorded in the task's explicit assumptions.
  - **`normal` exit assertions:** Binds `integer` parameters only to established ghosts (`old_*`, unprefixed post-fields, `i32` `result`, and helper `field_*` writes), admitting `==`, `!=`, `+`, `-`, and unary `-` with integer literals under checked `i64` evaluation. Declaring an unused parameter does not satisfy the obligation to constrain all exit fields, `result`, and helper writes.
- **Lexical status:** `quant`, `condition`, `entry`, `normal`, `any`, and `error` are contextual ordinary words here; no global reserved word or lexer token is added. Distinct from `quant #guard = (...)` ([G05h](#g05h-local-condition-and-registered-lifetime-clauses)).

#### Example

```qcore
module contract;

extern fn flag_reset(s: ptr<mut, u8>) -> i32 contract {
    quant condition entry #pre_bit(#old_busy: integer) =
        ((#old_busy == 0) || (#old_busy == 1));
    quant condition normal #post_clear(#busy: integer, #result: integer) =
        ((#busy == 0) && (#result == 0));
};
```

#### Implementation & Provenance

- **Implementation:** `qcore_schemas::quant_condition`, `qcore_frontend::quant_condition`, `qcore_contract::quant_condition`, `qcore_contract::boolean_condition_profile`.
- **Lean model & audit:** `QCore.QuantCondition`, `AuditQuantCondition.lean`.

---

<a id="g01t-typed-theorem-instantiation-obligations"></a>

### G01t. Typed theorem instantiation obligations

#### Purpose and module mode

Inside a function `contract { ... }` block, `quant theorem` records an explicit instantiation request against a named formal theorem schema, retaining all substituted prerequisite obligations and open proof/source bindings.

#### Consolidated EBNF

```ebnf
theorem   = "quant", "theorem", "#", id, "(", [ parameters ], ")",
            "using", reference, "(", [ symbols ], ")",
            "requires", "(", [ formula, { ",", formula } ], ")",
            "yields", formula, ";" ;
symbols   = "#", id, { ",", "#", id } ;
reference = word, { ".", word } ;
```

*(Here `id`, `parameters`, and `formula` follow [G01p](#g01p-typed-contract-propositions-graph-version-3); `word` is a case-preserving ASCII identifier `[A-Za-z_][A-Za-z0-9_]*`.)*

#### Semantic side conditions and bounds

- **Reference and arity limits:**
  - `reference`: non-empty dot-separated ASCII identifier segments (`word`), at most **128 bytes** per segment and **1,024 bytes** overall, case-preserving. Spelling a theorem name supplies zero proof authority.
  - `symbols`: at most **64** declared value, predicate, or function `#id` symbols from `parameters`.
  - `requires (...)`: at most **32** prerequisite formulas.
  - Overall clause limits: at most **65,536** source bytes, **64** parameters, and **256** combined formula/term AST nodes at maximum depth **32**.
- **Conservative instantiation (`qcore_contract::quant_theorem::instantiate`):**
  - Checks an explicitly supplied `TheoremSchema` against the clause's typed signature, exact `reference`, symbol argument arity/sorts, and exact substituted `requires` and `yields` formulas.
  - Substitution is strictly capture-avoiding: bound-variable capture refuses, and distinct quantified binder spellings are **not** silently alpha-equated.
  - Model and audit artifact references require well-formed paths and SHA-256 digests, but instantiation performs **no** filesystem read, Lean kernel check, or registry trust elevation: the sealed, serialize-only result leaves both `model_proof_binding` and `source_binding` **open**.
  - Strict resource-v1, bounds-v1, and local-condition loaders refuse `quant theorem` clauses.

#### Example

```qcore
module contract;

extern fn step_counter(p: ptr<mut, u8>) -> unit contract {
    quant theorem #step_nonneg(#n: integer, #next: integer)
        using QCore.CounterModel.step_preserves_nonneg(#n, #next)
        requires (#n >= 0, #next == #n + 1)
        yields (#next >= 0);
};
```

#### Implementation & Provenance

- **Implementation:** `qcore_schemas::quant_theorem`, `qcore_frontend::quant_theorem`, `qcore_contract::quant_theorem`.
- **Lean model & audit:** `QCore.QuantTheorem`, `AuditQuantTheorem.lean`.

---

<a id="g01e-typed-conditional-own-operation-effects"></a>

### G01e. Typed conditional own-operation effects

#### Purpose and module mode

Inside a function `contract { ... }` block, `quant effect` specifies a typed relational transition for an operation's own step between explicit entry and exit `state` binders, naming its precondition guard (`when`), `object` read/write footprints, preserved `frame` formula, and post-state relation (`=`).

#### Consolidated EBNF

```ebnf
effect   = "quant", "effect", selector, "#", id, "(", [ parameters ], ")",
           "from", "#", id, "to", "#", id,
           "when", formula,
           "reads", "(", [ arguments ], ")",
           "writes", "(", [ arguments ], ")",
           "frame", formula,
           "=", formula, ";" ;
selector = "normal" | "any" | "error", "(", word, ")" ;
```

*(Here `id`, `parameters`, `arguments`, and `formula` follow [G01p](#g01p-typed-contract-propositions-graph-version-3); `word` is an ASCII identifier.)*

#### Semantic side conditions and bounds

- **State and footprint typing:**
  - The `from #s` and `to #s'` binders must be **distinct** parameters declared in `parameters` with value sort `state`.
  - Every term in `reads(...)` and `writes(...)` must have value sort `object` and conservative root depth at most **2**.
  - `reads` and `writes` may overlap (representing read-modify-write operations), with at most **64** terms in each list. Structurally identical duplicate terms within the same list refuse; distinct terms are **not** assumed non-aliasing without separate disjointness evidence.
  - Guard (`when`), `frame`, and transition (`=`) formulas are typed over the shared `parameters` signature.
- **Overall bounds:** At most **65,536** source bytes, **64** parameters, and **256** combined formula/term AST nodes at maximum depth **32**.
- **Authority boundary:** Declaring an effect states a conditional own-step relation only; it establishes no execution occurrence, atomicity, C11 memory order, cross-thread synchronization, whole-call frame, or C call summary. Strict resource-v1, bounds-v1, and local-condition loaders refuse `quant effect` clauses.

#### Example

```qcore
module contract;

extern fn cell_write(c: ptr<mut, u8>, v: i32) -> unit contract {
    quant effect normal #write_cell(
        #s0: state,
        #s1: state,
        #cell: object,
        #val: function<state, object, integer>,
        #v: integer
    )
    from #s0 to #s1
    when (#v >= 0)
    reads ()
    writes (#cell)
    frame (forall (#other: object) { (#other != #cell => (#val(#s1, #other) == #val(#s0, #other))) })
    = (#val(#s1, #cell) == #v);
};
```

#### Implementation & Provenance

- **Implementation:** `qcore_schemas::quant_effect`, `qcore_frontend::quant_effect`.
- **Lean model & audit:** `QCore.QuantEffect`, `AuditQuantEffect.lean`.

---

# Part II — Sequential `.qr` Execution Grammars

<a id="g02-source-execution-profile"></a>

## G02. Base sequential resource profile (`source.exec12`)

### Purpose and module mode

`source.exec12` is the baseline sequential `.qr` execution profile (`executable` mode by default, or entry-free `library` mode). It requires no `#import` unless optional diagnostic logging (`log_level`, `#import std.logger;`) is configured, and serves as the sequential fallback route when no specialized stdlib, scalar-call, extern, FFI, or concurrent route is selected. Check-time `if / else` branching ([G02i-branch](#g02i-the-source-branching-construct)) and sequential `error region` blocks (`source.region.v1`) lower directly onto this profile's multi-model sequential IR.

### Consolidated EBNF

```ebnf
body        = "{", { statement }, "}" ;
statement   = acquisition
            | constant
            | check
            | direct_call
            | drop
            | move
            | if_stmt                                           (* §G02i-branch *)
            | region_stmt                                       (* source.region.v1 *)
            | return ;

constant    = "let", IDENT, [ ":", "i32" ], "=", [ "-" ], NAT, ";" ;
acquisition = "let", IDENT, "=", "qcore_acquire", "(",
              RESOURCE_ID, ",", BOOL, ",", ERROR_NAME, ")", ";" ;
check       = "qcore_check", "(", BOOL, ",", ERROR_NAME, ")", ";" ;
direct_call = FUNCTION_NAME, "(", ")", ";" ;
drop        = "drop", RESOURCE_BINDING, ";" ;
move        = "move", RESOURCE_BINDING, ";" ;
return      = "return", [ [ "-" ], NAT ], ";" ;
BOOL        = "true" | "false" ;

region_stmt = "error", "region", IDENT, "{",
                { acquisition | drop | "credit", NAT, ";" | "effort", NAT, ";" },
                "qcore_check", "(", "false", ",", ERROR_NAME, ")", ";",
              "}", "catch", "(", ERROR_NAME, ")",
              [ "quant", "open", IDENT, "as", IDENT ],
              "{", { "discharge", IDENT, ";" }, "}" ;
```

### Semantic side conditions, annotations, and boundaries

- **Checked annotations (`SOURCE-ANNOTATION-ENFORCEMENT`, `W48-QR-SCALAR-TYPES-R2`, `R3`):**
  - **Local bindings:** Optional `: i32` on an integer literal `let` is checked against the full signed 32-bit range (`-2147483648 ..= 2147483647`). Mismatched or unsupported local type annotations and malformed local heads refuse at `Q020`. Because `qcore_acquire` establishes custody identity without publishing a nominal payload type, **every** explicit type annotation on its `let` binding (including `own<T>`) refuses at `Q020`, even inside an `error region`. Local `use` grade annotations refuse at `Q100`.
  - **Function parameters:** Even though `direct_call` passes zero arguments and the executable entry takes none, every function parameter annotation is validated against the [G02t](#g02t-fixed-width-scalar-types) scalar table (`isize`/`usize` require `--recorded-target`). Non-scalar parameter types (`Widget`, `fn() -> i32`, `own<T>`, `borrow<T>`) refuse at `Q020`.
  - **Uncalled `extern fn` declarations:** On every route of an `executable` or `library` module, all `extern fn` signatures are validated even if never called: parameter types must belong to the `§C04.1` ABI vocabulary ([G02t](#g02t-fixed-width-scalar-types) scalars or custody pointers) and result types to that vocabulary or `unit`; anything else (`Widget`, `own<T>`, `quant<...>`) refuses at `Q020`. (`module contract;` files are exempt and validated by their contract loader.)
- **Controlled model intrinsics and custody:**
  - `RESOURCE_ID` must fit `u32`. `ERROR_NAME` must name a declared `error` type; if the module declares none, the driver synthesizes a default `Failure` error type.
  - `qcore_acquire` and `qcore_check` require literal `BOOL` controls; they are controlled model operations, not general `malloc` calls or runtime predicates.
  - `drop` releases live custody; `move` transfers a live resource to the modeled function result payload (`CompiledModule.result_profile`), not an external OS handle.
  - Result annotations on executable functions must be `i32` (requiring a range-checked signed integer literal `return`) or `unit` (requiring bare `return;`, encoded as `0` in the reduced ABI).
- **Direct calls and C11 emission:**
  - `direct_call` must resolve to an unshadowed, zero-parameter source function in an acyclic call graph.
  - For C emission, every reachable callee must be observationally pure in the reduced model: empty resource trace, normal exit, and no moved custody payload (net-zero aggregate resource flow alone is insufficient).
- **Excluded forms:** General arithmetic expressions, argument-passing calls, indirect calls, extern calls (routed to [G02d](#g02d-bounded-extern-calls) ahead of `G02`), loops, lambdas, threading, and non-empty inline `contract { ... }` blocks (`Q021`; empty `contract {}` is accepted).

### Example

```qcore
error Failure { Failed; }

fn main() -> i32 {
    let item = qcore_acquire(1, true, Failure);
    qcore_check(true, Failure);
    drop item;
    return 0;
}
```

### Implementation & Provenance

- **Cards:** `SOURCE-ANNOTATION-ENFORCEMENT`, `QUANT-REGION-SOURCE`, `W48-QR-SCALAR-TYPES-R2`, `W48-QR-SCALAR-TYPES-R3`.
- **Implementation:** `compiler/src/driver.rs`, `crates/qcore-frontend/src/whole_program.rs`, `crates/qcore-frontend/src/source_region.rs`.

---

<a id="g02s-scalar-functions-and-explicit-output-sourcescalar-callsv1"></a>
<a id="g02s-pure-scalar-functions-sourcescalar-callsv1"></a>

## G02s. Scalar functions and explicit output (`source.scalar-calls.v1`)

### Purpose and route selection

`source.scalar-calls.v1` compiles direct, acyclic multi-function `.qr` programs over fixed-width scalars, IEEE floats, scalar records, checked arithmetic with optional `ArithmeticError` propagation/recovery, and deterministic standard-output / logger lines.

- **Route selection:** Selected after concurrent, FFI, extern, event, buffer, string, HTTP, and socket routes, and **before** `source.std-io.v1`, whenever:
  1. Retained tokens contain the function-style `print(` form;
  2. A function contains a non-literal `return` or `let`, a typed local annotation, or an argument-taking call to a source function;
  3. A module mixes `std.io` line statements with scalar-call syntax that `source.std-io.v1` refuses (leaving every module admitted by `source.std-io.v1` byte-identical); or
  4. A module declares a buffer-free record ([G07.13](#g0713-the-first-source-level-aggregate-sourcerecordv1)) alongside more than one function (`W48-QR-USABLE-RECORDS`).
- **Required `#import`s:** Pure scalar/record modules need no import. `print(expr);` requires `#import std.io;`. Line statements require `#import std.io;` (`print`), `#import std.logger;` (`log`, `flush`), and `#import std.fmt;` (`dec`, `hex`).

### Consolidated EBNF

```ebnf
function   = "fn", NAME, "(", [ parameter, { ",", parameter } ], ")", "->", result,
             [ "effects", "{", [ effect, { ",", effect } ], "}" ],
             [ "contract", "{", "}" ],
             block ;

parameter  = NAME, ":", ( SCALAR | RECORD ) ;
result     = SCALAR | "unit" | RECORD ;
SCALAR     = "bool"
           | "i8" | "i16" | "i32" | "i64" | "isize"
           | "u8" | "u16" | "u32" | "u64" | "usize"
           | "f32" | "f64" ;                                    (* §G02t *)

effect     = "io"
           | "throws", "<", "ArithmeticError", ">"
           | "partial_eq"
           | "partial_ord" ;                                    (* each at most once *)

block      = "{", { statement }, end, "}" ;
end        = "return", [ expression ], ";"
           | "raise", ( NAME | "ArithmeticError", "::", VARIANT ), ";"
           | "match", call, "{", arm, arm, "}" ;                (* one Ok, one Err *)
arm        = "Result", "::", "Ok", "{", "value", ":", NAME, "}", "=>", block
           | "Result", "::", "Err", "{", "error", ":", NAME, "}", "=>", block ;
VARIANT    = "Overflow" | "DivisionByZero" | "OutOfRange" ;

statement  = "let", NAME, [ ":", ( SCALAR | RECORD ) ], "=", expression, ";"
           | call, [ "?" ], ";"
           | "print", "(", expression, ")", ";"                 (* legacy i32-only builtin *)
           | "print", RENDERING, OPERAND, ";"                   (* §G02g reader *)
           | "log", LEVEL, RENDERING, OPERAND, ";"              (* §G02g reader *)
           | "flush", ";" ;

expression = sum, [ CMP, sum ] ;                                (* non-associative *)
sum        = product, { ( "+" | "-" ), product } ;
product    = cast, { ( "*" | "/" | "%" ), cast } ;
cast       = postfix, { "as", SCALAR } ;
postfix    = unary, { ".", NAME } ;                             (* record field path *)
unary      = "-", unary
           | NAT | FLOAT | "true" | "false"
           | RECORD, "{", [ field_init, { ",", field_init }, [ "," ] ], "}"
           | call, [ "?" ]
           | NAME
           | "(", expression, ")" ;
field_init = NAME, ":", expression ;
CMP        = "==" | "!=" | "<" | "<=" | ">" | ">=" ;
call       = FUNCTION_NAME, "(", [ expression, { ",", expression } ], ")" ;
OPERAND    = [ "-" ], NAT | NAME ;
```

### Semantic side conditions, effects, and bounds

- **Whole-module checking and evaluation order:**
  - Every function in the module is type-checked, including uncalled helpers and `module library;` definitions. Calls must be direct and the call graph strictly acyclic.
  - Locals are immutable and non-shadowing within a function; initializers resolve before binding the new name. Operands and call arguments evaluate strictly left to right; binary `+`, `-`, `*`, `/`, `%`, and `as` associate left; comparisons (`CMP`) are non-associative.
  - An executable entry takes zero parameters and returns `i32` or `unit`. `unit`-returning calls cannot appear inside value expressions. Library modules check all definitions without publishing a closed observation or standalone `main`.
- **Effect row inference vs. explicit permissions:**
  - `io` is **inferred** when `effects { ... }` is omitted and the function directly or transitively prints, logs, or flushes; `effects { io }` permits IO, while `effects {}` forbids direct and transitive IO (`Q020`).
  - `throws<ArithmeticError>`, `partial_eq` (float `==`/`!=`), and `partial_ord` (float `<`/`<=`/`>`/`>=`) are **never inferred** (`Q020`). On this route, a caller must also declare every `partial_eq`/`partial_ord` effect its callees derive.
- **Output forms:**
  - `print(EXPRESSION);` evaluates an `i32` expression and writes decimal digits + `\n` to stdout. It is restricted to pure-`i32` modules and **refuses** (`Q020`) if the module uses any [G02t](#g02t-fixed-width-scalar-types) extension (non-`i32` types, comparisons, `as` conversions, `bool`, floats, or line statements).
  - `print RENDERING OPERAND;`, `log LEVEL RENDERING OPERAND;`, and `flush;` (including `x |> print dec;` pipe sinks, [G02h](#g02h-the-pipe-operator-and-the-operator-token-family)) use [G02g](#g02g-rendered-lines-stdfmt-stdio-and-stdlogger)'s reader and render any `SCALAR` at its own type (`borrow NAME` refuses at `Q028`; `log_level` refuses at `Q030`).
- **Checked arithmetic and C emission:**
  - Literal-only subexpressions fold during type checking: a constant fault (`Overflow`, `DivisionByZero`, `OutOfRange`, including `MIN / -1` and `MIN % -1`) refuses at check time (`Q020`) in **every** function, regardless of `throws<ArithmeticError>`.
  - Without `throws<ArithmeticError>`, closed-entry evaluation faults refuse at check time (`Q020`), and emitted C range-tests operands before computing, exiting with status `86` and empty streams on fault. With `throws<ArithmeticError>`, non-literal faults return error tags (`1`..`3`) consumed via `?` or `match` (propagating out of `main` exits `80 + tag`; see [G02t](#g02t-fixed-width-scalar-types)).
  - File-scope C fragments merge through `c_program::assemble` (deduplicating the IO prelude and emitting one `main`); scalar functions still cannot call functions owned by another profile.
- **Records across calls (`W48-QR-USABLE-RECORDS`):**
  - Records of scalars (and nested records of scalars) pass and return **by value** (`copy` semantics; emitted as `struct qcore_record_NAME` in topological dependency order). Custody wrappers (`own<R>`, `borrow<R>`, `ptr<const, R>`, `move p`) refuse (`Q020`).
  - Construction supplies every declared field once in any order; `.f1.f2` reads nested fields; arithmetic/comparison/casts/rendering on a whole record refuse (`Q020`).
  - Records containing an owned `bytes` field (`W48-QR-OWNED-RECORD-ACROSS-CALL`), field mutations (`W48-QR-SCALAR-CALLS-FIELD-WRITE`), and record return types on functions declaring `throws<ArithmeticError>` (`W48-QR-RECORD-THROWS-RESULT`) refuse at `Q020`.

| Compiler / Evaluator Bound | Maximum |
|---|---|
| Functions per module | 256 |
| Expression, call, parser-nesting, or binary-chain depth | 64 |
| IR expression nodes per module | 16,384 |
| Lexical tokens per function | 16,384 |
| Closed-entry evaluated expression steps | 100,000 |

### Examples

```qcore
#import std.fmt;
#import std.io;

type Pair { lo: i16, hi: i16 }

fn span(p: Pair) -> i32 {
    return (p.hi as i32) - (p.lo as i32);
}

fn checked_scale(x: i32, k: i32) -> i32 effects { throws<ArithmeticError> } {
    return x * k;
}

fn main() -> i32 effects { io } {
    let p = Pair { lo: -10, hi: 32 };
    let d = span(p);
    match checked_scale(d, 2) {
        Result::Ok { value: total } => {
            print dec total;
            return 0;
        }
        Result::Err { error: _e } => {
            return 1;
        }
    }
}
```

### Implementation & Provenance

- **Cards:** `SOURCE-SCALAR-CALLS-V1`, `W48-QR-SCALAR-TYPES`, `W48-QR-SCALAR-TYPES-R2`, `W48-QR-FLOATING-POINT`, `W48-QR-USABLE-RECORDS`.
- **Implementation:** `crates/qcore-frontend/src/source_scalar_calls.rs`.
- **Report:** [Implementation and evidence](reports/source-scalar-calls-v1/README.md).

---

<a id="g02t-fixed-width-scalar-types"></a>
<a id="g02t-fixed-width-scalar-types-aritherror-and-ieee-floating-point"></a>

## G02t. Fixed-width scalar types, `ArithError`, and IEEE floating point

### Purpose

`G02t` promotes the contract ABI scalar table ([G05](#g05-resource-loader-v1)) and IEEE 754 binary32/binary64 floating point into first-class `.qr` types, together with checked `as` conversions, the built-in `ArithmeticError` typed error discipline (abbreviated `ArithError` in navigation summaries; always spelled `ArithmeticError` in source), and the shared single-operation `value_statement` grammar used across non-expression routes.

### Scalar type and rendering table

| Quirin type | Mathematical / bit domain | Emitted C11 type | `dec` rendering (`std.fmt`) | `hex` rendering (`std.fmt`) |
|---|---|---|---|---|
| `bool` | `false`, `true` | `bool` (`<stdbool.h>`) | `false` / `true` | *refused* |
| `i8`, `i16`, `i32`, `i64` | Signed two's-complement $-2^{w-1} \dots 2^{w-1}-1$ | `int8_t` .. `int64_t` | Signed decimal | `0x` + 2 / 4 / 8 / 16 uppercase hex digits (two's complement) |
| `u8`, `u16`, `u32`, `u64` | Unsigned $0 \dots 2^w - 1$ | `uint8_t` .. `uint64_t` | Unsigned decimal | `0x` + 2 / 4 / 8 / 16 uppercase hex digits |
| `isize` | Signed range of recorded pointer width $W \in \{32, 64\}$ | `ptrdiff_t` | Signed decimal | `0x` + $W/4$ uppercase hex digits |
| `usize` | Unsigned $0 \dots 2^W - 1$ of recorded pointer width $W$ | `size_t` | Unsigned decimal | `0x` + $W/4$ uppercase hex digits |
| `f32` | All IEEE 754 binary32 bit patterns | `float` | Shortest round-trip (ECMAScript layout; `-0`, `nan`, `inf`, `-inf`) | Normalized C hex float (`0x1.8p+1`, `-0x0p+0`, `nan`, `inf`, `-inf`) |
| `f64` | All IEEE 754 binary64 bit patterns | `double` | Shortest round-trip (ECMAScript layout; `-0`, `nan`, `inf`, `-inf`) | Normalized C hex float (`0x1p-1074`, `-0x0p+0`, `nan`, `inf`, `-inf`) |

### Consolidated EBNF across routes

```ebnf
SCALAR          = INT_SCALAR | "bool" | FLOAT_SCALAR ;
INT_SCALAR      = "i8" | "i16" | "i32" | "i64" | "isize"
                | "u8" | "u16" | "u32" | "u64" | "usize" ;
FLOAT_SCALAR    = "f32" | "f64" ;

FLOAT           = DIGITS, ".", DIGITS, [ EXPONENT ]
                | DIGITS, EXPONENT ;                            (* single lexer token *)
EXPONENT        = ( "e" | "E" ), [ "+" | "-" ], DIGITS ;

(* Shared single-operation value statement on extern, task, buffer, and composed string routes *)
value_statement = "let", NAME, "=", value_expr, ";" ;
value_expr      = operand, ( "+" | "-" | "*" | "/" | "%" ), operand
                | operand, ( "==" | "!=" | "<" | "<=" | ">" | ">=" ), operand
                | "-", NAME
                | operand, "as", SCALAR ;
operand         = NAME | [ "-" ], INTEGER | [ "-" ], FLOAT | "true" | "false" ;

(* Typed payloads on concurrent task and mutex routes (§G02b, §G02c) *)
buffer_type     = ( "own" | "borrow" | "lend" ), "<", "Buffer", [ "<", SCALAR, ">" ], ">" ;
protected_decl  = "let", NAME, "=", "protected", ( NAT | SCALAR ), ";" ;
```

### Typing, checked integer arithmetic, and `as` conversions

- **Zero implicit conversions:** Mixed-type binary operands, mismatched `let` annotations, mismatched call/spawn arguments, and mismatched `return` values refuse. Integer literals take their expected integer type from context (defaulting to `i32` when unconstrained) and must fit that type's range (including `MIN`). Float literals take their expected float type from context (defaulting to `f64`, or the target float type of `FLOAT as f32`); an integer literal is never a float and a float literal is never an integer.
- **Exact checked integer arithmetic:** `+`, `-`, `*`, `/`, `%`, and unary `-` (`0 - x`, succeeding on unsigned types only at `0`) compute exact mathematical integers and fault if the result lies outside the type's range, on `/ 0` or `% 0`, or on signed `MIN / -1` and `MIN % -1`. Division truncates toward zero; no operation wraps. Emitted C tests operand bounds explicitly (without compiler builtins) **before** executing C arithmetic.
- **`EXPR as TYPE` conversions:** `as` binds tighter than binary operators and looser than unary `-` (`as` was already reserved; scalar type names after `as` are skipped by reference collection). Integer/float conversions are **total** when widening (every source value fits) and **checked** (`OutOfRange` on failure) when narrowing or changing signedness — never bit-reinterpreting or modularly truncating. Nothing converts to or from `bool`.
- **`bool` restrictions:** Produced by comparisons; supports `==` and `!=` only (no ordering, arithmetic, conversion, or `hex` rendering).
- **Target-width types (`isize`, `usize`):** Require `quirin --recorded-target FILE` (a closed 3-key JSON record: `pointer_width` `32` or `64`, `triple`, `compiler`). Without a recorded target, naming `isize` or `usize` refuses; with one, the result publishes `recorded target: ...` in its declared assumptions and emitted C `_Static_assert`s `SIZE_MAX`, `PTRDIFF_MIN`, and `PTRDIFF_MAX`.

### Built-in `ArithmeticError` discipline

- **Variants and tags:** Payload-free built-in `error` type with variants `Overflow` (tag `1`), `DivisionByZero` (tag `2`), and `OutOfRange` (tag `3`). Cannot be redeclared by user code (`Q013`).
- **Static fold vs. dynamic raise:** Literal-only subexpressions (and all operations on `source.buffer.v1`) fold at check time and refuse statically (`Q020` / `Q024` / `Q053`) even if `throws<ArithmeticError>` is declared (`QCore.ScalarTypedErrors.static_fault_is_dynamic_raise`). Operations with runtime-varying operands (`G02s` parameters/locals/calls, `G02d` undeclared extern outcomes, `G02a` worker parameters/joined outcomes/`Buffer` reads) require `throws<ArithmeticError>` in the enclosing function's `effects` row (`Q020` / `Q024` / `Q300` otherwise) and dynamically raise the classified variant.
- **Consumption and C ABI:** On `G02s` and `G02a`, calls/joins that may raise `ArithmeticError` must be consumed via `?` (requiring caller `throws<ArithmeticError>`) or canonical two-arm `Result::Ok` / `Result::Err` `match`, or raised explicitly via `raise ArithmeticError::VARIANT;` / `raise err;`. `?` or `match` on an infallible callee refuses. Emitted `G02s` functions return `int` tag (`0` ok, `1..3` error) + out-pointer; unhandled propagation from `main` exits with status `80 + tag`. On `G02a`, `ArithmeticError` uses the ordinary scope-exit + typed-error capsule (`Q600` if an owned `Buffer` is leaked on the raising edge) and reaches the parent unchanged through `join`.
- **Contextual name resolution:** `ArithmeticError` is skipped by reference collection only when immediately followed by `::` (or inside `throws<...>`).

### IEEE 754 floating point (`f32` and `f64`)

- **Single-rounding literals:** Read once at their target format (`f32` or `f64`); literals overflowing to `±inf` or nonzero literals underflowing to `0.0` refuse. No `nan`, `inf`, or hex-float source literals exist (write `0.0 / 0.0` or `1.0 / 0.0`).
- **Arithmetic and effects:** `+`, `-`, `*`, `/` and unary `-` follow IEEE 754 round-to-nearest-even and never fault (`NaN` and `±inf` propagate as values); `%` on floats refuses. Float comparisons require explicit `partial_eq` (`==`, `!=`) or `partial_ord` (`<`, `<=`, `>`, `>=`) in `effects { ... }`.
- **Float ↔ integer casts:** Integer/`f64` → `f32` and integer → `f64` round to nearest-even and never fault; `f32 as f64` is exact; `float as INT` truncates toward zero and faults (`OutOfRange`) on `NaN`, `±inf`, or out-of-target-range values (tested against exact hex thresholds before C casting).
- **Environment & rendering:** Emitted C includes `<float.h>` and `<string.h>`, emits `#pragma STDC FP_CONTRACT OFF` and one helper per float operation, `_Static_assert`s IEEE binary32/binary64 parameters and `FLT_EVAL_METHOD == 0`, `#error`s on `__FAST_MATH__` or finite-math-only, renders `dec` (Dragon4/ECMAScript) and normalized `hex` without `libc` formatting, and publishes the IEEE floating-point premise in result assumptions ([spec §L14.7](QCORE_SPEC_r0_3.md)).

### Cross-route scalar & float support summary

| Route / Construct | Supported `G02t` capabilities and route-specific rules |
|---|---|
| **Entry-only route gate** (`G02g`, `G02k-buffer`, `G02k-event`, `G07.9`, `G07.11`, `G07.10`) | Declaring any function besides the entry refuses at the route's own code (`Q030`, `Q051`, `Q058`, `Q068`, `Q080`, `Q090`), closing unread-helper annotations. |
| **`source.buffer.v1` & composed `source.string.v1`** ([G02k-buffer](#g02k-an-owned-byte-buffer-stdbuffer), [G07.9](#g079-immutable-utf-8-string-views-v1-sourcestringv1)) | Typed scalar/float environment (`let x = 1.5;`, `let x = 1.5 as f32;`, conversions, `value_statement` with `partial_eq`/`partial_ord`, record fields, integer-typed `for` bounds/indices). Byte indices, stored bytes, slice bounds, and `lend` bounds require `i32` (`Q052` / `Q077`); float loop bounds refuse on every route (`Q098`). Unused scalar locals emit `(void) NAME;` for `-Werror`. |
| **`source.extern-call.v1` & `source.std-io-extern.v1`** ([G02d](#g02d-bounded-extern-calls), [G02i-extern-io](#g02i-composed-extern-calls-and-rendered-lines)) | All `SCALAR` ABI parameters/results (`f32`/`f64` included), range/bit-exact `succeeds` outcomes, `let x = FLOAT;`, `value_statement` (runtime operands require `throws<ArithmeticError>`, exiting `80 + tag`), and composed rendering of known values (`Q024`, `Q043`). `G02d.1` waivers also admit `f32`/`f64`. |
| **`source.ffi.v1` & `.qrlib`** ([G02u](#g02u-imported-c-structs-sourceffiv1), [G02d.2.1](#g02d21-the-generated-manifest-v2)) | `struct!` fields, `#c_ffi` parameters/results, `.qr` helpers, and `.qrlib` v1/v2 `float`/`double` C ABI types. |
| **`source.tasks.*`, `G02v`, `G02f`, `G02j`** ([G02a](#g02a-scoped-copy-task-execution)–[G02f](#g02f-flat-epoch-bound-over-a-checked-task-region), [G02j](#g02j-composed-scoped-tasks-and-rendered-lines)) | Worker parameters/results, `copy` captures (`TransferKind::FixedScalar`), closures, record leaves, `value_statement`, composed parent/worker renderings, and F02 fixed-width integer/bool worker outcomes (`QCORE_TASK_RESULT TASK TYPE NUMBER`). Executable/task entry return types stay `i32` or `unit`. |
| **Typed `Buffer<T>` & `protected T`** ([G02b](#g02b-owned-buffer-task-execution), [G02c](#g02c-guarded-fixed-footprint-mutex)) | `own/borrow/lend<Buffer<T>>` (`own<Buffer>` = `own<Buffer<i32>>`) with byte lease extent `sizeof(T)` (1, 2, 4, 8, or target `W/8`; byte writes into `Buffer<bool>` refused at `Q600`, admitted on `Buffer<f32>`/`Buffer<f64>`; typed payload runs sit outside F02 encoding). `protected T` allocates `sizeof(T)` bytes (`protected bool` masks low bit). Error payloads `Variant(own<Buffer>)` remain `i32`-only (`Q300`). |

### Example

```qcore
#import std.fmt;
#import std.io;

fn hypot_sq(x: f64, y: f64) -> f64 {
    return x * x + y * y;
}

fn main() -> i32 effects { io, partial_ord } {
    let h = hypot_sq(1.5, 2.0);
    let ok = h > 6.0;
    print dec h;       // stdout: 6.25
    print hex h;       // stdout: 0x1.9p+2
    print dec ok;      // stdout: true
    return 0;
}
```

### Implementation & Provenance

- **Cards:** `W48-QR-SCALAR-TYPES`, `R2`–`R6`, `W48-QR-FLOATING-POINT`, `W48-QR-FLOAT-IEEE-LEAN-MODEL`, `W48-QR-FLOAT-BUFFER-PAYLOADS`.
- **Implementation:** `crates/qcore-core/src/scalar_types.rs`, `crates/qcore-core/src/scalar_values.rs`, `crates/qcore-frontend/src/source_value_statement.rs`.
- **Lean models:** `QCore.ScalarTypes`, `QCore.ScalarTypedErrors`, `QCore.ScalarTasks`, `QCore.ScalarLoopBounds`, `QCore.ScalarPayloads`, `QCore.ScalarEpochTasks`, `QCore.ScalarComposedString`, `QCore.Ieee754`, `QCore.Ieee754Round`, `QCore.Ieee754Ops`, `QCore.FloatTypes`.

---

<a id="g0713-the-first-source-level-aggregate-sourcerecordv1"></a>

## G07.13. Source records (`source.record.v1`)

### Purpose and cross-route support

`source.record.v1` defines Quirin's nominal source-level product aggregate: `type NAME { ... }`. Like `if` and `for`, it is a core language construct requiring **no `#import`**, **zero new reserved words**, and **zero dedicated diagnostic codes** (attributing refusals to the admitting route's pass band).

Records are supported across three `.qr` execution families (plus C layout imports via `struct!` in [G02u](#g02u-imported-c-structs-sourceffiv1)):

| Admitting route | Allowed field types | Supported operations | Route selection & custody notes |
|---|---|---|---|
| **`source.buffer.v1`** ([G02k-buffer](#g02k-an-owned-byte-buffer-stdbuffer)) | Any [G02t](#g02t-fixed-width-scalar-types) `SCALAR` (`bool`, ints, `f32`/`f64`), `"bytes"`, or another declared `RECORD` | Construction, `.path` scalar read or owned-`bytes` partial move out, `.path = val` scalar write or hole refill (`mut`), `.path[i]` byte load/store, `extent(r.path)`, `drop r.path;`, `drop r;`, scalar-record copy `let q = r;` | Single-function entry only (`Q051`–`Q057`). Selected for any single-function module declaring `type NAME { ... }` or holding `"bytes"`. |
| **`source.scalar-calls.v1`** ([G02s](#g02s-scalar-functions-and-explicit-output-sourcescalar-callsv1)) | Any `SCALAR` or scalar-only `RECORD` (`"bytes"` refused at `Q020`) | Construction, `.path` nested field reads, `let` bindings, by-value function parameters and `return` results | Multi-function buffer-free modules (`Q020`). Immutable plain values; field writes and `throws<ArithmeticError>` record returns refuse. |
| **`source.tasks.*` & `G02j`** ([G02a](#g02a-scoped-copy-task-execution), [G02j](#g02j-composed-scoped-tasks-and-rendered-lines)) | Any `SCALAR`, `own<Buffer[<T>]>`, or declared `RECORD` (`"bytes"` refused at `Q500`) | Construction, `.path` read/move-out, `.path = val` write/refill (`mut`), `let q = r;` (scalar records), `drop r[.path];`, `qcore_buffer_read/write(r.path, ...)`, whole/field `spawn` captures, worker `return [move] r;`, `G02j` `print/log ... r.path;` | Decomposed into depth-first leaf slots (`Q020`, `Q300`, `Q600`, `Q050`). Refused on `source.epoch.tasks.v1` (`Q500`). |

### Consolidated EBNF

```ebnf
record_decl = "type", NAME, "{", field, { ",", field }, [ "," ], "}" ;
field       = NAME, ":", field_type ;
field_type  = SCALAR                                            (* §G02t, floats included *)
            | "bytes"                                           (* source.buffer.v1 only *)
            | "own", "<", "Buffer", [ "<", SCALAR, ">" ], ">"   (* source.tasks.* only *)
            | RECORD_NAME ;                                     (* acyclic nested record *)

path        = ".", NAME, { ".", NAME } ;

(* Statement forms on source.buffer.v1 and source.tasks.* (G02s embeds construction/paths in expressions) *)
record_stmt = "let", NAME, "=", RECORD_NAME, "{", assign, { ",", assign }, [ "," ], "}", ";"
            | "let", NAME, "=", NAME, path, ";"                 (* scalar copy or owned move-out *)
            | "let", NAME, "=", NAME, ";"                       (* copy bind: scalar-only records *)
            | NAME, path, "=", VALUE, ";"                       (* scalar write or owned hole refill *)
            | "let", NAME, "=", NAME, path, "[", OPERAND, "]", ";"      (* source.buffer.v1 *)
            | NAME, path, "[", OPERAND, "]", "=", OPERAND, ";"          (* source.buffer.v1 *)
            | "let", NAME, "=", "extent", "(", NAME, path, ")", ";"     (* source.buffer.v1 *)
            | "drop", NAME, [ path ], ";" ;

assign      = NAME, ":", VALUE ;
VALUE       = [ "-" ], INTEGER | [ "-" ], FLOAT | "true" | "false" | NAME ;
```

### Semantic side conditions, custody paths, and bounds

- **Declaration syntax and formation bounds:**
  - Introduced by the existing reserved keyword `type` followed by `NAME {` (disambiguated from `type NAME = ...;` aliases by `{`, and from `resource type NAME {` by an earlier parser branch; `record` was rejected by the constraint-14 census to preserve archived `Q000` probe pins).
  - Each record declares **1 to 32** uniquely named fields (`Q051` duplicate/malformed; `Q053` count > 32).
  - Nested records may appear in either declaration order, provided the containment graph is **acyclic** (`Q051`), at most **8** levels deep, and flattens to at most **256** total leaf fields (`Q053`).
- **Total construction and exact field typing:**
  - Construction supplies every declared field **once by name** in any order (`Q051` on missing, extra, or duplicate fields; `Q013` on undeclared `RECORD_NAME`).
  - Field initializers and writes must match the field's exact `G02t` or record type with no implicit conversion (`Q052` type mismatch, `Q053` out-of-range literal, `Q051` missing `--recorded-target` for `isize`/`usize`).
- **Path-extended ownership for owned fields (`AGGREGATE-OWNED-FIELD`, `W48-QR-USABLE-RECORDS`):**
  - Supplying an acquired buffer binding (`bytes` on `source.buffer.v1`, `own<Buffer<T>>` on `source.tasks.*`) or an owning nested record at construction **moves** custody into `RECORD.PATH` (`qcore_core::ownership::Place`).
  - `let x = r.path;` copies a scalar or scalar sub-record, and **moves out** an owned leaf (leaving a **hole**; reading or dropping a hole refuses at `Q054` / `Q600`; whole reads of owning sub-records and whole writes of any sub-record refuse at `Q051`).
  - `r.path = val;` mutates a scalar leaf or **refills** an owned hole (`OverwriteOfLiveField` `Q054` / `Q600` if the leaf is still live) and derives effect `mut`. On `source.buffer.v1`, reading or writing even a scalar field of an owning record requires the record to be live (`UseAfterDrop` `Q054` after `drop r;`).
  - `drop r;` releases every live owned leaf depth-first in declaration order; leaking any owned leaf at exit reports `RECORD.PATH` (`Q054` / `Q600`).
  - `let q = r;` copies scalar-only records and refuses records declaring any owned field (`CopyOfOwned` `Q054`, even if all owned leaves were moved out).
- **Lexical `.` adjacency and C11 emission:**
  - Because `Float` tokens begin only at digits, `p.x` always lexes as `Word Dot Word` (leaving IPv4 `127.0.0.1` `Float Dot Float` untouched); numeric paths such as `p.1.2` refuse at `Q051`.
  - Emitted C defines `struct qcore_record_NAME` with `f_FIELD` members in topological order before holders and emits designated initializers `{ .f_x = ... }` so C keywords never collide.
- **Remaining boundaries (`source.buffer.v1`):** `string` view fields (`AGGREGATE-VIEW-FIELD`), `lend(...)` in an entry that constructs an owning record (`OWNED-FIELD-LEND-WINDOW`, `Q051`), and owned-field operations inside `for` loop bodies (`OWNED-FIELD-LOOP-BODY`, `Q051`).

### Example

```qcore
#import std.buffer;
#import std.fmt;
#import std.io;

type Header { version: u8, tag: i32 }
type Packet { head: Header, payload: bytes }

fn main() -> unit effects { io, alloc, mut } {
    let raw = bytes(4);
    raw[0] = 79;
    raw[1] = 75;

    let h = Header { version: 1, tag: 200 };
    let pkt = Packet { head: h, payload: raw };

    pkt.head.tag = 204;
    let first = pkt.payload[0];
    let code = pkt.head.tag;
    print dec first;            // stdout: 79
    print dec code;             // stdout: 204

    drop pkt;                   // releases pkt.payload
    return;
}
```

### Implementation & Provenance

- **Cards:** `AGGREGATES-V1`, `AGGREGATE-OWNED-FIELD`, `OWNED-FIELD-ROUTE-REACH`, `COMPOSED-WINDOW-REACH`, `W48-QR-SCALAR-TYPES-R3`, `W48-QR-FLOAT-BUFFER-PAYLOADS`, `W48-QR-USABLE-RECORDS`, `W48-QR-TASK-RECORD-CAPTURES`.
- **Implementation:** `crates/qcore-frontend/src/source_record.rs`, `crates/qcore-core/src/source_record.rs`, `crates/qcore-core/src/ownership.rs`.
- **Lean models:** `QCore.OwnedFieldCopyReach`, `QCore.TaskRecordCaptures`.

---

<a id="g02k-an-owned-byte-buffer-stdbuffer"></a>
<a id="g02k-buffer"></a>

## G02k-buffer. Owned byte buffers and disjoint write lends (`std.buffer`, `source.buffer.v1`)

### Purpose and required `#import`

`source.buffer.v1` (`std.buffer`) provides a source-level, extent-carrying, zero-initialized owned byte buffer with check-time bounds verification, disjoint range write lends, bounded counted loops, records, and typed check-time scalar/float operations.

- **Required `#import`s:** `#import std.buffer;` (`Q015` without it when acquiring, measuring, lending, or indexing a buffer; `drop` alone is ungated). Output statements additionally require `#import std.io;`, `#import std.logger;`, and `#import std.fmt;`.
- **Tier distinction:** Implements the self-contained C11 source tier of [`STD_BUFFER_V1.md`](design/STD_BUFFER_V1.md) §1 (no extern boundary, extent carried in the ownership ledger), distinct from the native-library-backed C04 pointer tier and from concurrent task `Buffer<T>` ([G02b](#g02b-owned-buffer-task-execution)).
- **Route selection:** Selected after concurrent, FFI, extern, and event routes, and **before** `source.string.v1` (unless `borrow_utf8` bridges them, [G07.9](#g079-immutable-utf-8-string-views-v1-sourcestringv1)) and `source.std-io.v1`. Models a single zero-parameter `unit` entry (`Q051` if helper functions exist; multi-function byte helpers use [G02r](#g02r-resource-aware-byte-helpers-sourceresource-callsv1)).

### Consolidated EBNF

```ebnf
module    = { import }, { error_decl | record_decl | entry } ;
import    = [ "#" ], "import",
            ( "std.buffer" | "std.fmt" | "std.io" | "std.logger" ), ";" ;
entry     = "fn", NAME, "(", ")", "->", "unit",
            [ "effects", "{", [ EFFECT, { ",", EFFECT } ], "}" ],
            "{", { stmt }, "}" ;

stmt      = "let", NAME, "=", [ "-" ], INTEGER, ";"                 (* i32 binding *)
          | "let", NAME, "=", [ "-" ], FLOAT, ";"                   (* f64 binding, §G02t *)
          | "let", NAME, "=", [ "-" ], FLOAT, "as", SCALAR, ";"     (* single-round float *)
          | "let", NAME, "=", OPERAND, "as", SCALAR, ";"            (* checked conversion *)
          | value_statement                                         (* single §G02t op *)
          | "let", NAME, "=", NAME, ";"                             (* copy bind *)
          | "let", NAME, "=", "bytes", "(", INTEGER, ")", ";"       (* acquire 1..=4096 *)
          | "let", NAME, "=", "extent", "(", target, ")", ";"       (* bind byte length *)
          | "let", NAME, "=", "lend", "(", NAME, ",", OPERAND, ",", OPERAND, ")", ";"
          | "let", NAME, "=", target, "[", OPERAND, "]", ";"        (* byte load, i32 *)
          | target, "[", OPERAND, "]", "=", OPERAND, ";"            (* byte store, 0..=255 *)
          | "drop", target, ";"                                     (* release owner *)
          | record_stmt                                             (* §G07.13 *)
          | [ unroll_attr ], "for", NAME, "in", "0", "..", BOUND, "{", { stmt }, "}"
          | "print", RENDERING, OPERAND, ";"
          | "log", LEVEL, RENDERING, OPERAND, ";"
          | "flush", ";"
          | "return", ";" ;

target    = NAME | NAME, path ;                                     (* buffer/loan or owned field *)
OPERAND   = [ "-" ], INTEGER | NAME ;
BOUND     = INTEGER | NAME ;
EFFECT    = "io" | "alloc" | "mut" | "partial_eq" | "partial_ord" ;
```

### Semantic side conditions, disjoint write lends, and diagnostics

- **Transported extent and inherited custody:**
  - `bytes(N)` requires an integer literal `N` in **`1 ..= 4096`** (`Q051` non-literal, `Q053` out of range) and lowers to `OwnershipOp::Acquire { footprint: Footprint::single("bytes", N) }`. `extent(b)` reads `N` back from the custody ledger as an `i32`.
  - Leak at return (`NormalPathLeak`), double release (`DoubleDrop`), use after release (`UseAfterDrop`), rebound live names (`BindingAlreadyExists`), and copy of owned resources (`CopyOfOwned`) are decided by the inherited cycle-3 `qcore_core::ownership` checker and reported at `Q054`.
- **Check-time constant decidability and typed environment:**
  - Every scalar/float binding carries one [G02t](#g02t-fixed-width-scalar-types) type (`i32` for integer `let`, `extent`, byte loads, and literal loop bounds). Every operation is folded at check time (`Q053` on arithmetic/conversion fault).
  - Byte indices, stored byte values (`0 ..= 255`), and `lend` bounds require type `i32` (`Q052` for other scalar types — convert explicitly with `as i32`; `Q053` for index outside `[0, extent)` or byte outside `0..=255`; `Q055` if a buffer or loan is passed at an operand/render position).
  - `let q = p;` (copy bind) copies scalars and scalar-only records and refuses buffers, write loans, and owning records (`Q054`). It does **not** select `source.buffer.v1` on its own, avoiding collision with `let s = tcp;` ([G07.11](#g0711-a-checked-socket-as-a-reactor-registered-event-source)).
- **Disjoint write lends (`let sub = lend(b, LOW, HIGH);`, cards `SLICE-PERMISSIONS-WRITE-LEND`, `LOOPS-ON-COMPOSED-ROUTES`):**
  - Lends exclusive read/write authority over `[LOW, HIGH)` of owned buffer `b` (`0 <= LOW <= HIGH <= extent(b)`; `LOW` and `HIGH` must belong to the lowering's constant tier — literals, literal `let`, `extent`, or constant conversions/value statements, refusing byte-derived bounds at `Q053`).
  - `sub[i]` checks `0 <= i < HIGH - LOW` (`Q053`) and rebases to `b[LOW + i]`; `extent(sub)` yields `HIGH - LOW`.
  - **Window & disjointness (`QCore.WriteLend`, `QCore.LoopWindows`):** Each loan's window runs from `lend(...)` through its last top-level use (extending across all iterations of any `for` loop whose body names `sub`). Two lends of `b` may be live simultaneously iff their byte intervals are disjoint (`max low < min high`; `OverlappingLends` `Q054` otherwise). While a lend is live, owner reads/writes outside all live lends succeed, owner reads/writes overlapping a live lend refuse (`Q054`), and `drop b;` refuses (`DropWhileBorrowed` `Q054`).
  - Window-shape restrictions (`Q051` / `Q052`): Crossed (non-nested, overlapping-lifetime) lend windows (`Q051`), acquisitions or drops inside a live lend window or lending-entry loop body (`Q051`), per-iteration `lend(...)` inside a `for` body (`Q051`), lending a record field (`Q051`), re-lending a loan (`Q052`), and `drop sub;` on a loan (`Q052`, loans expire at last use) all refuse.
- **Reserved vs. contextual words:** `bytes` and `extent` are reserved (`buffer` was left unreserved to protect 555 existing corpus bindings). `lend` is **contextual** — recognized only immediately before `(` in a `let` initializer, so `let lend = 5;` remains valid.
- **Derived effects and C11 emission:**
  - `bytes(N)` and `drop` derive `alloc`; byte/field stores derive `mut`; `print`/`log`/`flush` derive `io`; float comparisons require `partial_eq`/`partial_ord` (`Q057` / `Q052` if undeclared; `Q056` certificate recheck).
  - Emits self-contained C11 using `calloc` (zero-initialized), displaced `const` loan pointers, and bounds-checking witnesses that `abort()` if ever violated; stdout and stderr are compared per stream against reference execution.

### Example

```qcore
#import std.buffer;
#import std.fmt;
#import std.io;

fn main() -> unit effects { io, alloc, mut } {
    let b = bytes(8);
    let left = lend(b, 0, 4);
    let right = lend(b, 4, 8);

    for i in 0..4 {
        left[i] = 65;
        right[i] = 90;
    }

    let a = b[0];               // lend windows closed after the loop
    let z = b[7];
    print dec a;                // stdout: 65
    print dec z;                // stdout: 90
    drop b;
    return;
}
```

### Implementation & Provenance

- **Cards:** `STD-BUFFER`, `SLICE-PERMISSIONS-WRITE-LEND`, `LOOPS-ON-COMPOSED-ROUTES`, `OWNED-FIELD-ROUTE-REACH`, `COMPOSED-WINDOW-REACH`, `W48-QR-SCALAR-TYPES-R3`, `R5`, `W48-QR-FLOAT-BUFFER-PAYLOADS`.
- **Implementation:** `crates/qcore-frontend/src/source_std_buffer.rs`, `crates/qcore-core/src/std_buffer.rs`.
- **Design & reports:** [`design/STD_BUFFER_V1.md`](design/STD_BUFFER_V1.md), [Wave 22 std.buffer report](reports/archive/waves-20-39/wave22-std-buffer/README.md).
- **Lean models:** `QCore.WriteLend`, `QCore.LoopWindows`, `QCore.ScalarLoopBounds`.

---

<a id="g02r-resource-aware-byte-helpers-sourceresource-callsv1"></a>
<a id="g02r-resource-aware-functions-sourceresource-callsv1"></a>

## G02r. Resource-aware byte helpers (`source.resource-calls.v1`)

### Purpose and required `#import`

`source.resource-calls.v1` extends `std.buffer` across direct, acyclic `.qr` helper function boundaries, checking affine `own<bytes> use one` transfers, call-scoped `borrow<bytes>` read loans, call-scoped `lend<bytes>` read/write loans, minimum extent preconditions, and transitive `alloc`/`mut` effects.

- **Required `#import`:** `#import std.buffer;` (`Q015` without it).
- **Type distinction:** Always spelled `bytes` (whitespace inside `< ... >` is insignificant); concurrent `Buffer` ([G02b](#g02b-owned-buffer-task-execution)) is a distinct resource and refuses on this route (`Q020`).

### Consolidated EBNF

```ebnf
helper_fn  = "fn", NAME, "(", [ parameter, { ",", parameter } ], ")", "->", result,
             [ "effects", "{", effect, { ",", effect }, "}" ],
             "{", { helper_stmt }, return_stmt, "}" ;

parameter  = NAME, ":", ( "i32"
                        | "own", "<", "bytes", ">", "use", grade_one
                        | "borrow", "<", "bytes", ">"
                        | "lend", "<", "bytes", ">" ) ;
grade_one  = "one" | "1" | "[", "1", ",", "1", "]" ;
result     = "unit" | "i32" | "own", "<", "bytes", ">" ;
effect     = "alloc" | "mut" ;                                  (* no trailing comma *)

helper_stmt = "let", NAME, "=", [ "-" ], INTEGER, ";"
            | "let", NAME, "=", "bytes", "(", INTEGER, ")", ";"
            | "let", NAME, "=", "extent", "(", NAME, ")", ";"
            | "let", NAME, "=", NAME, "[", OPERAND, "]", ";"
            | NAME, "[", OPERAND, "]", "=", OPERAND, ";"
            | "drop", NAME, ";"
            | "let", NAME, "=", call, ";"
            | call, ";" ;                                       (* unit-returning callee *)

call        = FUNCTION_NAME, "(", [ argument, { ",", argument } ], ")" ;
argument    = OPERAND
            | "move", NAME
            | "borrow", NAME
            | "lend", "(", NAME, ")" ;
return_stmt = "return", ";"
            | "return", OPERAND, ";"
            | "return", "move", NAME, ";" ;
OPERAND     = [ "-" ], INTEGER | NAME ;
```

### Semantic side conditions, aliasing, extent obligations, and bounds

- **Call graph and reachability:**
  - Calls must be direct; shadowing a helper name with a parameter or local makes the call indirect and refuses (`Q020`).
  - The call graph must be **acyclic**, and **every** declared helper in the module must be reachable from the selected entry (`Q020` for unreachable helpers or library-only modules).
- **Custody, loans, and alias discipline:**
  - `move NAME` transfers one live `own<bytes>` slot into an `own<bytes> use one` parameter, leaving the caller binding stale. `return move NAME;` transfers an owned buffer back to the caller, where it **must** be bound by `let` (`Q054`).
  - `borrow NAME` passes a call-scoped read loan (`borrow<bytes>`, permitting byte reads and `extent`); `lend(NAME)` passes a call-scoped read/write loan (`lend<bytes>`, permitting byte reads, byte stores, and `extent`).
  - Loans exist for one call only and cannot be returned, stored, subloaned, or relended. One owner may appear multiple times in a single call's argument list **only** when every occurrence is `borrow NAME` (`Q054` if combined with `move` or `lend`).
- **Derived minimum extents and zero-owner allocation safety:**
  - Each helper derives the minimum byte extent required of every `bytes` parameter from its checked indices and transitive calls; every call site must prove the passed buffer's actual extent meets that minimum (`Q053`).
  - Transitive `alloc` and `mut` effects are inferred when `effects { ... }` is omitted and checked when written (`Q057`; no trailing comma, at most one `effects` clause, empty/nonempty `contract` blocks refuse at `Q020`/`Q021`).
  - Because `calloc` failure exits with status `86` without unwinding, `bytes(N)` or any `alloc`-performing helper call is admitted **only** when zero other live owners exist in the caller or callee at that point (`Q054`).
- **Excluded forms:** Branches, loops, records, extern calls, concurrent `Buffer`, local `use` grades, borrowed return types, computed extents/indices, and cross-profile calls.

| Resource-Helper Bound | Maximum |
|---|---|
| Functions per module | 256 |
| Parameters per function | 256 |
| Call DAG depth | 64 |
| Source IR statements | 16,384 |
| Expanded DAG items (visited stmts + buffer/ownership ops) | 32,768 |

### Example

```qcore
#import std.buffer;

fn fill_header(buf: lend<bytes>, tag: i32) -> unit effects { mut } {
    buf[0] = tag;
    buf[1] = 1;
    return;
}

fn first_byte(buf: borrow<bytes>) -> i32 {
    let b = buf[0];
    return b;
}

fn make_packet() -> own<bytes> effects { alloc, mut } {
    let p = bytes(4);
    fill_header(lend(p), 65);
    return move p;
}

fn main() -> i32 effects { alloc, mut } {
    let pkt = make_packet();
    let head = first_byte(borrow pkt);
    drop pkt;
    return head;
}
```

### Implementation & Provenance

- **Card:** `RESOURCE-HELPER-FUNCTIONS`.
- **Implementation:** `crates/qcore-frontend/src/source_resource_calls.rs`.
- **Report & Lean:** [Resource helper functions report](reports/resource-helper-functions/README.md) (Lean decision lemmas for alias compatibility, affine slots, extent composition, acyclicity, transitive effects, and zero-owner allocation failure).

---

<a id="g079-immutable-utf-8-string-views-v1-sourcestringv1"></a>

## G07.9. Immutable UTF-8 string views, `concat`, and `borrow_utf8` (`std.string`, `source.string.v1`)

### Purpose and required `#import`s

`source.string.v1` (`std.string`) implements immutable, custody-free `{ ptr, len }` UTF-8 string views, `O(1)` byte lengths, code-point-checked slicing, compile-time `concat` composition, range-wise `borrow_utf8` bridges over `std.buffer` storage, and length-bounded stream output.

- **Required `#import`s:** `#import std.string;` (`Q015` without it; quoted strings in declaration metadata such as `verify scope` need no import). Rendering text requires `#import std.io;` (`print`) or `#import std.logger;` (`log`), **without** needing `std.fmt` unless scalar `dec`/`hex` line selectors are also used. Bridging a byte buffer requires `#import std.buffer;`.
- **Route selection:** Selected after concurrent, FFI, extern, event, and unbridged `source.buffer.v1` routes (and **ahead** of `source.buffer.v1` when `borrow_utf8(...)` is spelled, and ahead of `source.std-io.v1` when a constructed string literal, inline slice, `len`, `concat`, or `borrow_utf8` appears in a function body; bare `print bogus;` without string constructs stays on `source.std-io.v1`).

### Consolidated EBNF

```ebnf
module    = { import }, { error_decl | record_decl | entry } ;
import    = [ "#" ], "import",
            ( "std.string" | "std.buffer" | "std.fmt" | "std.io" | "std.logger" ), ";" ;
entry     = "fn", NAME, "(", ")", "->", "unit",
            [ "effects", "{", [ EFFECT, { ",", EFFECT } ], "}" ],
            "{", { stmt }, "}" ;

stmt      = "let", NAME, [ ":", "string" ], "=", STRING-TOKEN, ";"
          | "let", NAME, [ ":", "string" ], "=", NAME, "[", OPERAND, ":", OPERAND, "]", ";"
          | "let", NAME, [ ":", "string" ], "=", "concat", "(", PIECE, ",", PIECE, { ",", PIECE }, ")", ";"
          | "let", NAME, [ ":", "string" ], "=", "borrow_utf8", "(", NAME, ")", ";"
          | "let", NAME, [ ":", "string" ], "=", "borrow_utf8", "(", NAME, ",", OPERAND, ",", OPERAND, ")", ";"
          | "let", NAME, "=", "len", "(", NAME, ")", ";"
          | "let", NAME, "=", [ "-" ], INTEGER, ";"
          | BUFFER_STMT                                         (* §G02k-buffer, when bytes(N) is acquired *)
          | "print", TEXT, ";"                                  (* selector-free UTF-8 output *)
          | "log", LEVEL, TEXT, ";"                             (* selector-free UTF-8 log *)
          | "print", RENDERING, OPERAND, ";"
          | "log", LEVEL, RENDERING, OPERAND, ";"
          | "log_level", LEVEL, ";"
          | "flush", ";"
          | "return", ";" ;

TEXT      = STRING-TOKEN | NAME | NAME, "[", OPERAND, ":", OPERAND, "]" ;
PIECE     = TEXT | ( "dec" | "hex" ), "(", NAME, ")" ;
OPERAND   = [ "-" ], INTEGER | NAME ;
EFFECT    = "io" | "alloc" | "mut" | "partial_eq" | "partial_ord" ;
```

### Semantic side conditions, `borrow_utf8` windows, and diagnostics

- **Contextual keywords (zero new reserved words):** `len`, `concat`, and `borrow_utf8` are recognized only immediately before `(` (preserving `len` as an identifier in contracts); `string` is recognized only after `:` on `let`.
- **Literal decoding and UTF-8 slicing (`Q068`–`Q073`):**
  - `STRING-TOKEN` admits escapes `\\`, `\"`, `\n`, `\t`, `\r`, `\0`, and `\xHH` only; decoded payloads must be well-formed UTF-8 of at most **4,096 bytes** (`Q070`). Views are `{ ptr, len }` and may contain embedded `\0` bytes (never `strlen`-measured).
  - `len(s)` binds `s`'s byte count as a check-time `i32`.
  - `s[low:high]` (bound or inline in `TEXT`/`PIECE`) requires check-time `i32` bounds (`Q069` / `Q077` if non-`i32` or non-constant) satisfying `0 <= low <= high <= len(s)` and falling on UTF-8 code-point boundaries (`Q071` — never rounded).
- **Static `concat` (`STRING-CONCAT`, `Q077`):**
  - Requires **at least two** `PIECE`s and produces a `static const` byte array at compile time (max **4,096 bytes**, `Q077`), deriving **neither `alloc` nor `mut`**. `dec(NAME)` and `hex(NAME)` format check-time-constant scalars using `std.fmt`'s exact rules.
- **Range-wise `borrow_utf8` and embedded `std.buffer` (`STRING-CONCAT`, `CT07-SLICE-PERMISSIONS`, `LOOPS-ON-COMPOSED-ROUTES`, `W48-QR-SCALAR-TYPES-R6`, `W48-QR-FLOAT-BUFFER-PAYLOADS`):**
  - Modules acquiring `bytes(N)` embed `G02k-buffer`'s statements (including conversions, `value_statement`, and typed `for` loops; `lend(...)` is refused on the composed route at `Q077`, `SLICE-PERMISSIONS-COMPOSED-LEND`).
  - `borrow_utf8` takes **1 or 3** arguments (`borrow_utf8(b)` is identical to `borrow_utf8(b, 0, extent)`). Bounds satisfy `0 <= low <= high <= extent(b)` (`Q077`).
  - **Two static checks:** (1) UTF-8 validity of `b[low..high]` is proved at check time from `b`'s constant write history (`Q079`, including code points split by `low`/`high`); (2) inside the borrow's **window** (from `borrow_utf8` to the last read of the view or any slice derived from it, extended across loop iterations), stores into `[low, high)` and any `drop b;` refuse (`Q078`), while owner stores into disjoint bytes (`max low < min high`) and overlapping read borrows succeed.
- **Text vs. scalar output (`STRING-SLICE-IO`, `Q075`–`Q076`):**
  - Closed-word separation: `print`/`log` followed by `dec` or `hex` renders a scalar (`print dec s;` refuses at `Q072`); without a selector (`print s;`, `s |> print;`, `s[0:4] |> log info;`), it writes exact UTF-8 bytes via length-bounded `fwrite` (`\0` bytes intact) and derives `io` (`Q073` if undeclared; `Q075` malformed text line; `Q076` non-string name).
- **Excluded follow-ons:** Freezing a buffer into an owned string (`STRING-SLICE-FREEZE`), runtime-varying pieces or borrows (`STRING-SLICE-CONCAT-RUNTIME`, `STRING-SLICE-BORROW-VARYING`), C04/FFI string ABI transport (`STRING-SLICE-ABI`), string equality/search, single-byte indexing on `string`, and mutable `{ ptr, len, cap }` slices.

### Example

```qcore
#import std.string;
#import std.buffer;
#import std.io;

fn main() -> unit effects { io, alloc, mut } {
    let b = bytes(8);
    b[0] = 79;                  // 'O'
    b[1] = 75;                  // 'K'

    let status = borrow_utf8(b, 0, 2);
    b[4] = 33;                  // disjoint byte write outside [0, 2) is admitted

    let count = len(status);
    let line = concat("status=", status, " (", dec(count), " bytes)");
    line |> print;              // stdout: status=OK (2 bytes)

    drop b;                     // borrow window ended at `concat`
    return;
}
```

### Implementation & Provenance

- **Cards:** `STRING-SLICE`, `STRING-SLICE-IO`, `STRING-CONCAT`, `CT07-SLICE-PERMISSIONS`, `LOOPS-ON-COMPOSED-ROUTES`, `W48-QR-SCALAR-TYPES-R6`, `W48-QR-FLOAT-BUFFER-PAYLOADS`.
- **Implementation:** `crates/qcore-frontend/src/source_std_string.rs`, `crates/qcore-core/src/std_string.rs`.
- **Design & reports:** [`design/STRING_SLICE_V1.md`](design/STRING_SLICE_V1.md), `docs/reports/archive/language-and-tooling/string-slice/README.md`, `docs/reports/archive/language-and-tooling/string-slice-io/README.md`, `docs/reports/archive/language-and-tooling/string-concat/README.md`, `docs/reports/archive/language-and-tooling/ct07-slice-permissions/README.md`.
- **Lean models:** `QCore.StringSliceIo`, `QCore.StringConcat`, `QCore.LoopWindows`, `QCore.ScalarComposedString`.

---

<a id="g02h-the-pipe-operator-and-the-operator-token-family"></a>
## G02h. The pipe operator and the operator token family

### Consumer and Desugaring Model

`source.pipe.v1` is a **parse-time token desugaring, not a standalone execution profile**: no source file selects it, and no semantic route observes `|>` directly. Before the delimiter map is built and before structural parsing (`G01`) runs, every well-formed pipeline statement is rewritten into the exact token stream of the ordinary statement(s) a programmer would have written by hand. Consequently, name resolution, custody/ownership checking, effect rows, usage linearity, concurrency ledgers, whole-program checking, reference execution, and C11 emission all inspect ordinary statements. A piped `.qr` program and its hand-written twin are `syntax_equivalent` and emit byte-identical C11.

### Consolidated EBNF

```ebnf
pipe_stmt   = [ "let", NAME, "=" ], OPERAND, ( "|>", STAGE )+, ";" ;
OPERAND     = token, { token } ;                     (* verbatim: `x`, `-7`, `move b`, `s[0:4]` *)

STAGE       = [ "move" ], STAGE_BODY ;
STAGE_BODY  = "print", [ RENDERING ]                 (* G02g / G07.9 sink; 0 or 1 selector *)
            | "log", LEVEL, [ RENDERING ]            (* G02g / G07.9 sink; 1 or 2 selectors *)
            | "drop"                                 (* ownership release; final stage only *)
            | NAME, [ "(", [ arg, { ",", arg } ], ")" ], [ ADAPTER ] ;

ADAPTER     = "succeeds", BOOL, "else", { disposition }, "raise", NAME ;
RENDERING   = "dec" | "hex" ;
LEVEL       = "trace" | "debug" | "info" | "warn" | "error" | "fatal" ;
```

### Canonical Desugaring Table

| Written pipeline | Desugared statement sequence |
|---|---|
| `total \|> print dec;` | `print dec total;` |
| `total \|> log fatal dec;` | `log fatal dec total;` |
| `s \|> print;` | `print s;` |
| `s[0:4] \|> log info;` | `log info s[0:4];` |
| `b \|> drop;` | `drop b;` |
| `x \|> f;` and `x \|> f();` | `f(x);` |
| `b \|> f(7);` | `f(b, 7);` |
| `move b \|> release;` | `release(move b);` |
| `let r = x \|> f succeeds true else raise E;` | `let r = f(x) succeeds true else raise E;` |
| `8 \|> alloc succeeds true else raise E \|> move free;` | `let qcore__pipe0 = alloc(8) succeeds true else raise E;` `free(move qcore__pipe0);` |
| `7 \|> qcore_buffer_new? \|> drop;` | `let qcore__pipe0 = qcore_buffer_new(7)?;` `drop qcore__pipe0;` |

### Semantic Side Conditions and Operator Token Family

- **Multi-stage chains and hygienic temporaries:**
  - A pipeline of $n$ stages desugars into $n$ statements: each non-final stage binds its output to a synthesized temporary `qcore__pipeN`, which becomes the incoming operand of stage $n+1$; the final stage retains the outer wrapper (`let NAME = ...;` or bare statement).
  - Custody flows through `qcore__pipeN` under the unedited checkers: an allocating intermediate stage whose result is never consumed or dropped fails at the route's leak diagnostic (`Q202` / `Q600`).
  - **Refusal-enforced hygiene (`Q036`):** Single-stage pipelines allocate zero temporaries. Any file requiring multi-stage temporaries that also writes an identifier starting with `qcore__pipe` is refused at `Q036` rather than silently $\alpha$-renamed.
- **Stage value production vs. statement sinks:**
  - Whether a stage can feed a successor is decided syntactically by stage kind: **call stages** (`NAME(...)`, including call-shaped model intrinsics) produce bindable expressions, whereas **sink stages** (`print`, `log`, `drop`) and operandless statements (`flush`) produce no value and are refused in non-final position (`Q037`) or under `let NAME = ...` (`Q036`).
  - Whether a specific callee's return type is actually bindable (for example, refusing a `unit`-returning `extern fn` or `qcore_buffer_write` mid-chain) is decided by the **target route** at its own diagnostic code (`Q024`, `Q050`), never by `G02h`.
  - Sink selector arities are inclusive ranges (`print`: `0..=1`, `log`: `1..=2`); routes that require `dec`/`hex` (such as `source.std-io.v1`) refuse selectorless scalar `x |> print;` at their own code (`Q030`).
- **Intrinsic stage admissibility:**
  - Call-shaped model intrinsics (`qcore_buffer_new`, `qcore_buffer_read`, `qcore_buffer_write`, `qcore_error_payload`, `qcore_acquire`, `qcore_check`) match the `NAME(...)` call alternative directly; sequential routes read the post-desugaring token stream so `qcore_acquire` / `qcore_check` work identically piped or unpiped.
  - Keyword-shaped statements (`spawn`, `join`, `share`, `read`, `write`, `grow`, `open`, `close`, `bound`, `overhead`) are **not** pipeline stages (`Q036`); `"drop"` is the sole admitted keyword-shaped release stage (`final`-stage-only).
- **First-argument insertion and `move` forwarding:**
  - Quirin pipelines use **first-argument insertion** (`x |> f(7)` $\mapsto$ `f(x, 7)`).
  - Two spellings consume an incoming resource: `move OPERAND |> STAGE` for stage 1 (`|> move STAGE` on stage 1 refuses at `Q036`), and `|> move STAGE` for stage $k \ge 2$ (consuming `qcore__pipe{k-2}`). Inside an `ADAPTER`'s `else` disposition list, a bare `move` names the piped incoming temporary so multi-acquisition chains can dispose of intermediate resources on failure.
- **Span attribution:** User-written tokens preserve their exact byte spans. Synthesized punctuation and temporaries take zero-width spans anchored at the originating `|>` (`start` for stage 1, `end` for later stages, keeping adjacent statement spans disjoint); in multi-stage `let NAME = ...` chains, the outer `let NAME =` is re-anchored at the final `|>`.
- **The vertical-bar and bitwise/logical operator token family:**
  Two-byte operators are lexed by longest match before single-byte tokens:

  | Spelling | Lexer token | Admitted grammatical positions | Outside admitted positions |
  |---|---|---|---|
  | `[\|` / `\|]` | `QuantOpen` / `QuantClose` | Quantitative index delimiters ([G04](#g04-closed-quantitative-indices)) and loop `[#unroll]` attributes ([G07.12](#g0712-the-checked-bounded-counted-loop-sourceloopv1)) | `Q000` if mismatched |
  | `\|>` | `PipeInto` | Pipeline stage separator (desugared before parsing) | `Q036` / `Q045` (in `if` condition) |
  | `\|\|` | `PipePipe` | Short-circuit logical OR in `if` conditions ([G02i-branch](#g02i-the-source-branching-construct)) and zero-capture spawn closures `\|\|` ([G02v](#g02v-closures-with-explicit-capture-lists)) | `Q038` |
  | `&&` | `AndAnd` | Short-circuit logical AND in `if` conditions (tighter than `\|\|`) | `Q038` |
  | `\|` | `Pipe` | Parenthesized bitwise OR in `if` comparison operands ([G02i-branch](#g02i-the-source-branching-construct)), and spawn closure capture delimiters `\|...\|` ([G02v](#g02v-closures-with-explicit-capture-lists)) | `Q038` |
  | `^` | `Caret` | Parenthesized bitwise XOR in `if` comparison operands (between `&` and `\|`) | `Q038` |
  | `&` | `Ampersand` | Parenthesized bitwise AND in `if` comparison operands (tighter than `^`) | `Q038` |

  Note: `[|>` lexes as `[|` followed by `>`; `[||]` lexes as `[|` followed by `|]`; `(1 | 2) > 0` requires spacing or parentheses so `|` and `>` do not form `|>` (`1 |> 2` inside `if (...)` refuses at `Q045`).
- **Diagnostics:** `Q036` malformed pipeline shape, `move` on stage 1, bare `move` without a binding operand, assignment pipeline (`PIPE-ASSIGN`), or `qcore__pipe` prefix collision; `Q037` operandless stage (`flush`) or non-final sink stage; `Q038` operator token (`|`, `&`, `^`, `||`, `&&`) outside an admitted position.

### Implementation & Provenance

- **Cards:** `PIPE-OPERATOR`, `PIPE-CHAIN`, `PIPE-SCALAR-RESULT`, `PIPE-INTRINSIC-STAGE`, `PIPE-DROP-STAGE`, `OR-AND-LOWERING`, `BITWISE-CONDITIONS`, `STRING-SLICE-IO`, `W48-QR-CLOSURES`.
- **Implementation:** `crates/qcore-frontend/src/pipe.rs`, `crates/qcore-frontend/src/lexer.rs`.
- **Reports:** `docs/reports/archive/early-waves/wave19-or-and-lowering/README.md`, `docs/reports/archive/waves-20-39/wave20-bitwise-conditions/README.md`.

---

<a id="g02i-the-source-branching-construct"></a>
## G02i-branch. The source branching construct (`source.branch.v1`)

### Consumer and Route Scope

`source.branch.v1` checks and lowers `if (...) { ... } else ...` conditional statements on the base sequential route ([G02](#g02-source-execution-profile)). (`source.scalar-calls.v1` additionally admits typed runtime scalar `if`/`else` conditions over `bool` variables and scalar comparisons as specified in [G02s](#g02s-scalar-functions-and-explicit-output-sourcescalar-callsv1) and [G02t](#g02t-fixed-width-scalar-types); this section specifies the foundational two-sorted check-time branch construct and four-model custody join.) No `#import` directive and no new reserved words are required (`if`, `else`, `true`, `false` were already reserved). `else if` chains flatten into right-nested branches, and nested `if` statements inside arms are admitted.

### Consolidated Two-Sorted EBNF

```ebnf
branch_stmt = "if", "(", cond, ")", block, [ "else", ( block | branch_stmt ) ] ;

(* Sort 1: Condition tier — precedence levels 1..4 *)
cond        = conj, { "||", conj } ;                 (* 1: logical OR, left-associative *)
conj        = neg,  { "&&", neg  } ;                 (* 2: logical AND, left-associative *)
neg         = "!", neg | cmp ;                       (* 3: logical NOT, right-associative *)
cmp         = int_expr, [ cmp_op, int_expr ] ;       (* 4: comparison, non-associative *)

(* Sort 2: Integer tier — precedence levels 5..7 *)
int_expr    = xor_expr, { "|", xor_expr } ;          (* 5: bitwise OR, left-associative *)
xor_expr    = and_expr, { "^", and_expr } ;          (* 6: bitwise XOR, left-associative *)
and_expr    = atom,     { "&", atom     } ;          (* 7: bitwise AND, left-associative *)

atom        = "(", cond, ")"
            | "true" | "false"
            | [ "-" ], INTEGER
            | NAME ;                                 (* check-time i32 constant binding *)

cmp_op      = "==" | "!=" | "<" | "<=" | ">" | ">=" ;
```

### Semantic Side Conditions, Precedence Discipline, and Custody Join

- **Strict two-sorted typing (zero implicit coercion):**
  - Levels 1–4 produce a **condition**; levels 5–7 produce a signed 32-bit **integer**; `atom` produces whichever sort its spelling denotes. Neither sort coerces to the other: `if (mask & 4)` and `if (true | false)` both refuse at `Q045`.
  - On `source.branch.v1`, `NAME` in `atom` must resolve to a top-level check-time constant binding `let NAME [: i32] = INTEGER;` (`Q013` if unbound; `Q045` if bound to a resource identity or non-integer).
- **C-compatible precedence rule (`Q045` on C-divergent unparenthesized mixes):**
  - Precedence is declared only where it matches C11 (`&&` tighter than `||`; `&` tighter than `^` tighter than `|`, all left-associative).
  - Where C11's precedence conflicts with two-sortedness (C11 binds `!` tighter than `<` and `|`/`^`/`&` looser than `==`), unparenthesized forms such as `!a < b` and `a | b == c` are **refused** (`Q045`); write `!(a < b)` and `(a | b) == c`. Comparisons are non-associative (`1 < 2 < 3` refuses at `Q045`), and `|>` inside a condition refuses at `Q045`.
- **Structural connective and negation lowering:**
  - `if (A || B) T else E` lowers to `if (A) { T } else if (B) { T } else { E }`.
  - `if (A && B) T else E` lowers to `if (A) { if (B) { T } else { E } } else { E }`.
  - `if (!C) { A } else { B }` lowers by swapping the arms to `if (C) { B } else { A }`.
  - Both arms are validated in full and emitted as real C11 basic blocks computing the comparison (`if (INT32_C(3) < INT32_C(5)) { goto ...; }`), cross-checked by the translation-validation triangle and native execution.
- **Admitted arm statements and continuation hoisting:**

  | Arm statement | Status | Rule |
  |---|---|---|
  | `qcore_check(BOOL, Error);` | Admitted | Custody-neutral check |
  | `drop NAME;` / `move NAME;` | Admitted | Validated across surviving arms at the join |
  | Nested `if (...) { ... }` / empty arm | Admitted | Missing `else` is an empty fallthrough arm |
  | `return ...;` | Admitted | Discharges exit obligations at its own exit (`Q202` if leaking) and leaves the join |
  | Zero-arg `f();` | Admitted | Admitted conditionally only when `f`'s summary resource flow is neutral (`Q044` otherwise, `IF-ARM-CALL-FLOW`) |
  | `let NAME = ...;` | Refused (`Q044`) | Arm-local bindings have no post-join scope |
  | `let r = qcore_acquire(...);` | Refused (`Q103`) | Divergent resource-identity ledgers (`IF-ARM-ACQUIRE`) |
  | `while`, `match`, `defer`, `raise` | Refused | Retain their existing pass refusals (`Q500`, etc.) |

  When exactly one arm returns, statements written *after* the `if` are **moved** (never duplicated) into the surviving fallthrough arm before checking; nested one-sided early returns that would require duplicating the continuation, or statements written after an `if/else` where both arms return, refuse at `Q044` (`IF-ARM-RETURN-NESTED`).
- **Five-model join discipline:** Surviving fallthrough arms must agree on ordered live lists and transferred sets across cleanup lowering (`SourceOp::Branch`), ownership (`OwnershipOp::Branch`, `Q046` / `Q103`), typed errors (`ErrorOp::Branch`), and resource flow (`Flow::join_branches`), while usage (`UsageExpr::Branch`) joins arm demands.

### Example

```qcore
error CheckFailed;

fn main() -> i32 {
    let flags = 6;
    let a = qcore_acquire(1);
    if ((flags & 2) == 2 && !(flags < 0)) {
        qcore_check(true, CheckFailed);
        drop a;
    } else {
        drop a;
        return 1;
    }
    return 0;
}
```

### Implementation & Provenance

- **Cards:** `IF-STATEMENT`, `OR-AND-LOWERING`, `BITWISE-CONDITIONS`, `IF-ARMS`.
- **Implementation:** `crates/qcore-frontend/src/source_branch.rs`, `crates/qcore-core/src/ownership.rs`, `crates/qcore-core/src/cleanup_lowering.rs`.
- **Design & reports:** [`design/SOURCE_BRANCH_V1.md`](design/SOURCE_BRANCH_V1.md), `docs/reports/archive/early-waves/wave19-if-statement/README.md`, `docs/reports/archive/waves-20-39/wave20-bitwise-conditions/README.md`, `docs/reports/archive/waves-20-39/wave20-if-arms/README.md`.

---

<a id="g0712-the-checked-bounded-counted-loop-sourceloopv1"></a>
## G07.12. The checked bounded counted loop (`source.loop.v1`)

### Consumer and Admitting Routes

`source.loop.v1` is a **control-flow construct**, not a bundled module: it requires no `#import`, reserves `for`, treats `in` as a contextual keyword, and reads `..` as two adjacent `Dot` tokens. It is admitted on:
1. **`source.buffer.v1`** ([G02k-buffer](#g02k-an-owned-byte-buffer-stdbuffer)), including record and fixed-width scalar compositions on that route; and
2. **`source.string.v1` buffer-embedding entries** ([G07.9](#g079-immutable-utf-8-string-views-v1-sourcestringv1), card `LOOPS-ON-COMPOSED-ROUTES`), where `borrow_utf8` windows are checked across all loop iterations (`QCore.LoopWindows`).

Every other execution route retains its existing nested-block refusal (`LOOP-ROUTE-WIDENING`).

### Consolidated EBNF

```ebnf
loop_stmt   = [ unroll_attr ], "for", NAME, "in", "0", "..", BOUND,
              "{", { body_stmt }, "}" ;

unroll_attr = "[|", "#", "unroll", ",", "where", "#", "bound", "<", INTEGER, "|]" ;
BOUND       = INTEGER | NAME ;
```

### Three-Layer Unroll Budget and Semantic Side Conditions

- **Check-time constant `BOUND` (`Q098`):**
  - Low endpoint must be the literal `0` (`Q097` for any non-zero start, `LOOP-RANGE-LOW`).
  - `BOUND` must evaluate at lowering time to a positive integer in `1..=4096`: an integer literal, a `let` constant, `extent(buf)`, or a checked scalar conversion / `value_statement` over check-time constants of any [G02t](#g02t-fixed-width-scalar-types) integer type (excluding `bool`, and excluding `isize`/`usize` without a recorded target). Bounds read dynamically from a buffer byte, record field, or copy binding refuse at `Q098` (`LOOP-BOUND-FROM-BYTE`).
- **Three-layer iteration budget (`Q097`, `Q098`):**

  | Budget layer | Limit | Enforcement rule |
  |---|---|---|
  | **1. Absolute per-loop ceiling** | `4,096` | Inviolable maximum; `#bound < N` with `N > 4097` refuses *before* inspecting `BOUND` (`#bound < 4097` licenses up to `4,096`, matching `std.buffer`'s max extent). Vacuous clauses (`#bound < 1`, `#bound < 0`) also refuse up front. |
  | **2. Default unattributed budget** | `256` | A `for` without `unroll_attr` admits `BOUND` in `1..=256`; larger bounds refuse at `Q098` with the exact `[|#unroll, where #bound < ...|]` licence printed in the diagnostic. |
  | **3. Statement-adjacent `[|#unroll, ...|]`** | Strict `< N` | Licenses the single immediately following `for` when `BOUND < N` (`Q098` if `BOUND >= N`). Malformed, duplicate, dangling, or non-`for`-preceding attributes refuse at `Q097` (`Q000` if `[|` is unterminated). |
  | **Entry total unrolled events** | `8,192` | Total unrolled operations across the entire entry (including nested licensed loops) may not exceed `8,192` (`Q098`). |

  The `[|#unroll, ...|]` attribute **erases completely** after checking: attributed and unattributed loops of the same bound emit byte-identical C11 and identical certificates.
- **Scoping, nesting, and per-iteration custody neutrality (`Q097`, `Q099`):**
  - The induction variable `NAME` takes `0, 1, ..., BOUND - 1` at `BOUND`'s integer type, is immutable inside the body, and is out of scope after `}`. Body-local bindings do not escape and may not shadow outer entry bindings (`Q097`).
  - Nested `for` loops are admitted; `break`, `continue`, and `return` inside a loop body refuse by name at `Q097`.
  - **Net identity custody (`Q099`):** On `source.buffer.v1`, a single iteration's net custody effect must be the identity (acquiring and dropping within the same iteration is admitted; leaking an acquisition or dropping an outer buffer refuses at `Q099`). On `source.string.v1`'s composed route, body statements are restricted to pure `source.buffer.v1` statements with no body acquisitions/releases, no `lend(...)`, and no per-iteration `borrow_utf8` (`Q077`), while stores into bytes covered by an outer live `borrow_utf8` window refuse at `Q078`.
- **Asymmetric check vs. emission:** The checker unrolls the exact iterations so the certificate and reference run predict every byte, while the C11 backend emits a genuine `for (...)` loop, making native execution an independent witness of the unrolling.

### Example

```qcore
#import std.buffer;
#import std.fmt;
#import std.io;

fn main() -> unit effects { io, alloc, mut } {
    let buf = bytes(512);
    let n = extent(buf);
    [|#unroll, where #bound < 513|]
    for i in 0..n {
        buf[i] = 1;
    }
    let head = buf[0];
    print dec head;
    drop buf;
    return;
}
```

### Implementation & Provenance

- **Cards:** `LOOP-V1`, `LOOP-UNROLL-BUDGET`, `LOOPS-ON-COMPOSED-ROUTES`, `W48-QR-SCALAR-TYPES-R5`.
- **Implementation:** `crates/qcore-frontend/src/source_loop.rs`, `crates/qcore-frontend/src/source_std_buffer.rs`, `crates/qcore-frontend/src/source_std_string.rs`.
- **Reports & Lean:** `docs/reports/archive/language-and-tooling/loop-v1/README.md`, `docs/reports/archive/language-and-tooling/loop-unroll-budget/README.md`, `QCore.LoopWindows`.

---

<a id="g02g-rendered-lines-stdfmt-stdio-and-stdlogger"></a>
## G02g. Rendered lines: `std.fmt`, `std.io`, and `std.logger` (`source.std-io.v1`)

### Consumer and Capability Split

`source.std-io.v1` is the standalone sequential output route for modules that render scalar lines without buffers, strings, sockets, events, tasks, or extern calls. Each of the three bundled output modules gates its exact capability (`Q015` when missing):
- **`std.io`**: `print ...;` to standard output.
- **`std.logger`**: `log LEVEL ...;` to standard error, `flush;` (`fflush` sink handoff), and `log_level LEVEL;` (system-trace diagnostic sink threshold).
- **`std.fmt`**: scalar rendering selectors `dec` and `hex` (`flush;` and `log_level` need no `std.fmt`).

Note: The same line statements also compose into `source.scalar-calls.v1` ([G02s](#g02s-scalar-functions-and-explicit-output-sourcescalar-callsv1)), `source.buffer.v1` ([G02k-buffer](#g02k-an-owned-byte-buffer-stdbuffer)), `source.string.v1` ([G07.9](#g079-immutable-utf-8-string-views-v1-sourcestringv1)), `source.event.v1` ([G02k-event](#g02k-the-checked-event-surface)), `source.socket.v1` ([G07.11](#g0711-a-checked-socket-as-a-reactor-registered-event-source)), `source.std-io-extern.v1` ([G02i-extern-io](#g02i-composed-extern-calls-and-rendered-lines)), and `source.std-io-tasks.v1` ([G02j](#g02j-composed-scoped-tasks-and-rendered-lines)), which take route precedence when their features appear.

### Consolidated EBNF

```ebnf
stdio_module = [ module_header ], { import_stmt }, { error_decl | entry_fn } ;
import_stmt  = ( "#import" | "import" ),
               ( "std.fmt" | "std.io" | "std.logger" ), ";" ;

entry_fn     = "fn", NAME, "(", ")", "->", "unit",
               [ "effects", "{", EFFECT, { ",", EFFECT }, "}" ],
               "{", { stmt }, "}" ;

stmt         = "let", NAME, "=", [ "-" ], INTEGER, ";"
             | "print", RENDERING, OPERAND, ";"
             | "log", LEVEL, RENDERING, OPERAND, ";"
             | "log_level", LEVEL, ";"
             | "flush", ";"
             | "return", ";" ;

RENDERING    = "dec" | "hex" ;
LEVEL        = "trace" | "debug" | "info" | "warn" | "error" | "fatal" ;
OPERAND      = [ "-" ], INTEGER | NAME | "borrow", NAME ;
EFFECT       = "io" | "alloc" | "mut" ;
```

### Semantic Side Conditions and Stream Observation Rules

- **Operand custody exclusion (`Q028`, `Q030`):**
  - On `source.std-io.v1`, a bare `OPERAND` is an `i32` literal or a local `let` bound to an `i32` literal (`Q030` if `let` binds anything else; `Q028` if `NAME` is not a local integer binding). Because bare operands cannot denote buffers or handles, rendering cannot consume, leak, or retain custody.
  - **`borrow NAME` (`STD-FMT-BORROW`):** Renders a live resource's **model identity** (`rN`) — never raw pointers, descriptors, or object bytes — and is admitted **only** on routes whose checked custody model supplies and proves `NAME` live at that statement (namely `source.std-io-extern.v1`, [G02i-extern-io](#g02i-composed-extern-calls-and-rendered-lines)). On `source.std-io.v1` there is no custody model, so every `borrow NAME` refuses structurally at `Q028`. Because `borrow` precedes `NAME`, `x |> print dec` can never synthesize a borrowed render.
- **Keyword choices:** Highest log severity is spelled `fatal` (preserving `error` as the error-type declaration keyword). Unreserved words in `RENDERING` or `LEVEL` positions are collected as identifier references and fail closed at `Q013` before `Q030`.
- **Effects and per-stream comparison (`Q029`):**
  - Any `print`, `log`, `flush`, or `log_level` derives `{ io }` (empty `{}` if none appear; never `alloc` or `mut`); the entry's `effects { ... }` clause must include `io` (`Q029`).
  - Reference validation compares `stdout` against the reference `stdout` projection and `stderr` against `stderr` **independently per stream** (no cross-stream ordering between `stdout` and `stderr` is modeled or claimed). `flush;` emits `fflush` (sink handoff, not `fsync` crash durability). `log_level LEVEL;` configures the at-most-once hosted system-trace sink threshold without filtering source-authored `log` statements.

### Example

```qcore
#import std.fmt;
#import std.io;
#import std.logger;

fn main() -> unit effects { io } {
    let total = 42;
    print dec total;            // stdout: 42
    print hex 255;              // stdout: 0x000000FF
    log warn dec total;         // stderr: [WARN] 42
    flush;                      // sink handoff; writes no line
    return;
}
```

### Implementation & Provenance

- **Cards:** `STD-MODULES`, `STD-FMT-BORROW`, `W48-QR-SCALAR-TYPES`.
- **Implementation:** `crates/qcore-frontend/src/source_std_io.rs`, `crates/qcore-core/src/std_io.rs`.
- **Reports:** `docs/reports/archive/early-waves/wave17-std-modules/README.md`, `docs/reports/wave18-std-fmt-borrow/README.md`.

---

<a id="g02k-the-checked-event-surface"></a>
## G02k-event. The checked event surface (`source.event.v1`)

### Consumer and Reactor Projection Model

`source.event.v1` exposes the single-threaded event-reactor custody model (`qcore_core::event_reactor`) to `.qr` programs behind `#import std.event;`, composed with `source.std-io.v1` rendered lines. Modules using both `std.socket` and `std.event` select `source.socket.v1` ([G07.11](#g0711-a-checked-socket-as-a-reactor-registered-event-source)); modules using only `std.event` (plus optional output modules) select `source.event.v1`.

### Consolidated EBNF

```ebnf
event_module = [ module_header ], { import_stmt }, entry_fn ;
import_stmt  = ( "#import" | "import" ),
               ( "std.event" | "std.logger" | "std.io" | "std.fmt" ), ";" ;

entry_fn     = "fn", NAME, "(", ")", "->", "i32",
               [ "effects", "{", effect, { ",", effect }, "}" ],
               "{", { stmt }, "}" ;

stmt         = "event", "open", ";"
             | "event", "close", ";"
             | "endpoints", NAME, NAME, ";"
             | "watch", NAME, NAME, interest, ";"
             | "unwatch", NAME, ";"
             | "wake", NAME, [ byte ], ";"
             | "dispatch", "ready", NAME, ";"
             | "let", NAME, "=", "dispatch", "ready", NAME, ";"
             | "dispatch", "idle", ";"
             | "close", NAME, ";"
             | "log_level", level, ";"
             | "print", rendering, operand, ";"
             | "log", level, rendering, operand, ";"
             | "flush", ";"
             | "return", [ "-" ], INTEGER, ";" ;

interest     = "read" | "write" ;
byte         = INTEGER ;                             (* 0..=255; out-of-range refused *)
rendering    = "dec" | "hex" ;
operand      = INTEGER | NAME ;                      (* drained byte binding only *)
level        = "trace" | "debug" | "info" | "warn" | "error" | "fatal" ;
effect       = "io" | "alloc" | "mut" ;
```

### Projection Table and Determinism Boundary

| Source statement | Projected `EventOperation` sequence | Notes |
|---|---|---|
| `event open;` / `event close;` | `CreateBase` / `FreeBase` | Single live reactor context |
| `endpoints A B;` | `CreateSocketPair` (success branch) | Acquires paired local descriptors `A`, `B` |
| `watch E D INTEREST;` | `Assign(D)`, `Add` | Registers `E` watching descriptor `D` at priority `0` |
| `unwatch E;` | `Delete`, `FreeEvent` | Retires registration cookie `E` |
| `wake D [BYTE];` | *(no reactor op)* | Induces readiness on `D`'s peer; optional `BYTE` in `0..=255` |
| `[let V =] dispatch ready E;` | `BeginDispatch`, `Activate`, `CallbackEnter`, `CallbackExit`, `EndDispatch` | Verifies `poll`/`WSAPoll` readiness; `let V =` binds drained byte |
| `dispatch idle;` | `BeginDispatch`, `EndDispatch` | Verifies zero ready registrations |
| `close D;` | `CloseSocket` | Releases descriptor `D` |
| `print` / `log` / `flush` / `log_level` | *(no reactor op)* | Erases cleanly (`QCore.ComposeEventLines`) |

- **Single value form (`COMPOSE-EVENT-LINES`, `Q074`):** `let V = dispatch ready E;` is the sole `let` form on this route, binding the `0..=255` byte drained by `E`'s callback (carrying platform premise `EVENT-V1-STREAM-BYTE-ORDER`, verified at runtime). Rendering an endpoint or registration custody name — bare or via `borrow NAME` — refuses at `Q074`.
- **Determinism boundary (`Q061`) and `write` interest:**
  - Readiness must be induced by the program's own `wake` statements. Any `dispatch` turn at which **two or more** registrations are simultaneously ready refuses by name at `Q061` because OS `poll(2)`/`WSAPoll` callback ordering within a ready set is implementation-defined.
  - Registering `write` interest is fully custody-checked (`Q060`) and accepted as **check-only** (emitting no C11).
- **Diagnostics:** `Q058` entry/statement shape; `Q059` unbound custody name; `Q060` inherited `event_reactor` (`L00`–`L12`) refusal; `Q061` multi-ready or uninduced determinism violation; `Q062` undeclared `{ io }` effect; `Q074` composed value/rendering refusal.

### Example

```qcore
#import std.event;
#import std.fmt;
#import std.io;

fn main() -> i32 effects { io } {
    event open;
    endpoints reader writer;
    watch on_read reader read;
    wake writer 99;
    let msg = dispatch ready on_read;
    msg |> print dec;
    unwatch on_read;
    close reader;
    close writer;
    event close;
    return 0;
}
```

### Implementation & Provenance

- **Cards:** `STD-EVENT`, `COMPOSE-EVENT-LINES`.
- **Implementation:** `crates/qcore-frontend/src/source_std_event.rs`, `crates/qcore-core/src/event_reactor.rs`, `crates/qcore-core/src/std_event.rs`.
- **Design & Lean:** [`design/STD_EVENT_V1.md`](design/STD_EVENT_V1.md), [`../examples/events/README.md`](../examples/events/README.md), `QCore.ComposeEventLines`.

---

<a id="g0711-a-checked-socket-as-a-reactor-registered-event-source"></a>
## G07.11. Checked loopback sockets and reactor-registered event sources (`source.socket.v1`)

### Consumer and Vocabulary Composition

`source.socket.v1` implements the `std.socket` checked loopback TCP/UDP transport surface (`Q080`–`Q089`), including declared IPv4/IPv6 loopback endpoints (`SOCKET-INET-ADDRESSES`), UTF-8 string literal payloads (`SOCKET-STRING-PAYLOAD`), protocol-cell disjointness queries (`CT42-CELL-ROUTE`, `Q664`), and reactor-registered socket loans (`SOCKET-EVENT-COMPOSE` with `#import std.event;`). A module using both `std.socket` and `std.event` selects `source.socket.v1`; a module also declaring `std.http` messages selects `source.http.v1` ([G07.10](#g0710-a-checked-http11-message-subset-sourcehttpv1)).

### Consolidated EBNF

```ebnf
socket_module = [ module_header ], { import_stmt }, { error_decl | entry_fn } ;
import_stmt   = ( "#import" | "import" ),
                ( "std.socket" | "std.event" | "std.string"
                | "std.fmt" | "std.io" | "std.logger" ), ";" ;

entry_fn      = "fn", NAME, "(", ")", "->", "unit",
                [ "effects", "{", EFFECT, { ",", EFFECT }, "}" ],
                "{", { socket_stmt }, "}" ;

socket_stmt   = "let", NAME, "=", [ "-" ], INTEGER, ";"
              | "let", NAME, "=", ( "tcp" | "udp" ), ";"
              | "bind", NAME, [ ADDRESS ], ";"
              | "listen", NAME, ";"
              | "connect", NAME, ( NAME | ADDRESS ), ";"
              | "let", NAME, "=", "accept", NAME, ";"
              | "send", NAME, ( PAYLOAD | STRING-TOKEN ), ";"
              | "recv", NAME, ( INTEGER | STRING-TOKEN ), ";"
              | "let", NAME, "=", "recv", NAME, ";"
              | "sendto", NAME, ( NAME | ADDRESS ), PAYLOAD, ";"
              | "recvfrom", NAME, INTEGER, ";"
              | "let", NAME, "=", "recvfrom", NAME, ";"
              | "close", NAME, ";"
              | "disjoint", NAME, NAME, ";"
              (* Reactor composition — requires `#import std.event;` *)
              | "event", "open", ";"
              | "event", "close", ";"
              | "watch", NAME, NAME, INTEREST, ";"
              | "unwatch", NAME, ";"
              | "dispatch", "ready", NAME, ";"
              | "dispatch", "idle", ";"
              (* Rendered lines — gated by std.io / std.logger / std.fmt *)
              | "print", RENDERING, OPERAND, ";"
              | "log", LEVEL, RENDERING, OPERAND, ";"
              | "flush", ";"
              | "return", ";" ;

PAYLOAD       = "[", OPERAND, { ",", OPERAND }, "]" ;
OPERAND       = [ "-" ], INTEGER | NAME ;
ADDRESS       = IPV4_ADJACENT, ":", PORT
              | "[", IPV6_ADJACENT, "]", ":", PORT ;
PORT          = INTEGER | NAME ;
INTEREST      = "read" | "write" ;
RENDERING     = "dec" | "hex" ;
LEVEL         = "trace" | "debug" | "info" | "warn" | "error" | "fatal" ;
EFFECT        = "io" | "alloc" | "mut" ;
```

### Semantic Side Conditions, Reactor Loans, and Disjointness

- **Loopback-only determinism and address resolution (`Q080`, `Q084`, `Q085`):**
  - Programs execute on one thread over loopback (`127.0.0.1` or `[::1]`). `ADDRESS` tokens must be lexically adjacent (`127.0.0.1:8080`, `[::1]:8080`); hostnames refuse at `Q080` (`SOCKET-DNS`), out-of-range ports/bytes/counts (`recv` count `1..=64` bytes, payload bytes `0..=255`) refuse at `Q084`, and `connect`/`sendto` targeting an address not bound by this same entry refuses at `Q085` (`SOCKET-EXTERNAL-PEER`).
  - Every transmitted byte is check-time-constant (`PAYLOAD` or `STRING-TOKEN`, the latter gated by `#import std.string;`), allowing the checker to decide every `recv` / `recvfrom` outcome before emission (`Q085` when committed send history does not satisfy a receive) and verify received bytes at runtime.
- **Reactor registration as a lexical shared loan (`SOCKET-EVENT-COMPOSE`, `Q083`):**
  - `wake` and `endpoints` are intentionally absent here: readiness is induced by `connect`, `send`, and `sendto`, and descriptors come from `tcp`, `udp`, and `accept`.
  - `watch E S INTEREST;` projects to `OwnershipOp::BorrowShared` of socket `S` to registration `E` until `unwatch E;`. Consequently, the unedited ownership checker refuses `close S;` and state-mutating `bind`/`listen`/`connect` while `S` is watched (`Q083`), while admitting `send`, `recv`, and `accept`.
  - Registration spans are **strictly lexical** (LIFO retirement; crossed spans refuse, `SOCKET-EVENT-CROSSED-SPANS`). `dispatch ready E;` observes level-triggered readiness and drains nothing; `write` interest is checked in full and admitted as check-only (no C11 emitted).
- **Protocol-cell disjointness (`disjoint A B;`, `CT42-CELL-ROUTE`):**
  - Asks `qcore_core::socket_footprint` (`DisjointFootprintsV1`) whether live sockets `A` and `B` have disjoint derived protocol cells (`Q081` if not sockets, `Q083`/`Q082` if closed, `Q664` `SharedProtocolCell` when they share a cell, such as two endpoints or aliasing custodians).
- **Diagnostics:** `Q080` entry/statement/address shape; `Q081` non-constant operand or non-socket custody name; `Q082` TCP/UDP state-machine violation; `Q083` inherited ownership or reactor (`L00`–`L12`) custody refusal; `Q084` byte/payload/count/port range; `Q085` unsatisfied receive or external peer; `Q086` socket/value crossing; `Q087` undeclared `{ io }` effect; `Q088` certificate recheck; `Q089` backend refusal; `Q664` shared protocol cell.

### Example

```qcore
#import std.socket;
#import std.event;
#import std.fmt;
#import std.io;

fn main() -> unit effects { io } {
    event open;
    let listener = tcp;
    bind listener 127.0.0.1:19080;
    listen listener;

    let client = tcp;
    connect client 127.0.0.1:19080;
    let server = accept listener;
    disjoint client server;

    watch readable server read;
    send client [65];
    dispatch ready readable;
    let got = recv server;
    unwatch readable;

    print dec got;
    close server;
    close client;
    close listener;
    event close;
    return;
}
```

### Implementation & Provenance

- **Cards:** `QUIRIN-SOCKET-V1`, `SOCKET-INET-ADDRESSES`, `SOCKET-EVENT-COMPOSE`, `HTTP-QUIRIN-V1` (`SOCKET-STRING-PAYLOAD`), `CT42-CELL-ROUTE`.
- **Implementation:** `crates/qcore-frontend/src/source_socket.rs`, `crates/qcore-core/src/std_socket.rs`, `crates/qcore-core/src/socket_address.rs`, `crates/qcore-core/src/socket_footprint.rs`.
- **Reports:** `docs/reports/archive/language-and-tooling/quirin-socket-v1/README.md`, `docs/reports/archive/language-and-tooling/socket-inet-addresses/README.md`, `docs/reports/archive/language-and-tooling/socket-event-compose/README.md`, `docs/reports/ct42-cell-route/README.md`.

---

<a id="g0710-a-checked-http11-message-subset-sourcehttpv1"></a>
## G07.10. A checked HTTP/1.1 message subset (`source.http.v1`)

### Consumer and Scope Boundary

`source.http.v1` implements `#import std.http;` (`Q090`–`Q096`) on top of `source.socket.v1`'s unedited loopback transport. It checks an **executed RFC 9112 HTTP/1.1 subset with explicit boundaries** (never full HTTP/1.1 conformance): messages are constructed with compiler-synthesized CRLF framing and check-time-verified `Host` and `Content-Length` invariants, then transmitted and verified over loopback TCP sockets.

### Consolidated EBNF

```ebnf
http_module = [ module_header ], { import_stmt }, entry_fn ;
import_stmt = ( "#import" | "import" ),
              ( "std.http" | "std.socket" | "std.string"
              | "std.fmt" | "std.io" | "std.logger" ), ";" ;

entry_fn    = "fn", NAME, "(", ")", "->", "unit",
              [ "effects", "{", EFFECT, { ",", EFFECT }, "}" ],
              "{", { http_stmt }, "}" ;

http_stmt   = "let", NAME, "=", "request", STRING-TOKEN, STRING-TOKEN, STRING-TOKEN, ";"
            | "let", NAME, "=", "response", STRING-TOKEN, INTEGER, STRING-TOKEN, ";"
            | "header", NAME, STRING-TOKEN, VALUE, ";"
            | "content", NAME, VALUE, ";"
            | "let", NAME, "=", STRING-TOKEN, ";"              (* requires std.string *)
            | "let", NAME, "=", "concat", "(", VALUE, { ",", VALUE }, ")", ";"
            | "let", NAME, "=", "len", "(", NAME, ")", ";"     (* requires std.string *)
            | "send", NAME, NAME, ";"                          (* socket, message/view *)
            | "recv", NAME, NAME, ";"                          (* verified against message/view *)
            | SOCKET-STMT ;                                    (* G07.11 non-reactor statements *)

VALUE       = STRING-TOKEN
            | NAME                                             (* bound string view *)
            | ( "dec" | "hex" ), "(", NAME, ")" ;              (* formatted integer binding *)

EFFECT      = "io" | "alloc" | "mut" ;
```

### Semantic Side Conditions and Unforgeable Framing

- **Reserved message words:** `request`, `response`, `header`, and `content` (following RFC 9110 §6.4's `content` terminology; `send`, `recv`, `let`, `close` are reused). `SOCKET-STMT` delegates directly to `source_socket::socket_statement` (reactor composition with `std.event` remains outside `source.http.v1`, `HTTP-V1-EVENT-COMPOSE`).
- **Compiler-constructed framing and RFC 9112 gates (`Q091`–`Q093`):**
  - Source programs never write `\r\n` framing; the compiler emits CRLF after the start line, after each header field, and after the header block.
  - Every component `VALUE` resolves statically at check time to `static const` bytes (max **4,096 bytes**, `concat` requiring $\ge 2$ pieces, `Q090`), deriving neither `alloc` nor `mut`.
  - **`Q091` (start line):** Protocol version must be `"HTTP/1.1"`; request method must be an RFC 9110 `token`; request target must be origin-form (`/path`); status code must be a valid 3-digit status.
  - **`Q092` (header fields & injection exclusion):** Header field names must be `token`s (written `STRING-TOKEN` literals; computed field names are `HTTP-V1-COMPUTED-FIELD-NAME`); **no** start-line or header component — literal or `concat`-computed — may contain `CR` (`\r`) or `LF` (`\n`), making header/framing injection unrepresentable; requests must declare **exactly one** `Host` header (RFC 9112 §3.2).
  - **`Q093` (framing & `Content-Length`):** Any declared `Content-Length` header value must equal the exact UTF-8 byte count of the message's `content` (whether spelled as a literal or computed via `dec(len(body))`).
- **Fine-grained `#import` gates (`Q015`):**
  - `#import std.http;` gates `request`, `response`, `header`, `content`, and inline `dec(N)`/`hex(N)` at `VALUE` positions (needing neither `std.string` nor `std.fmt`).
  - `#import std.string;` is required when binding `let s = "...";`, `let s = concat(...);`, or `let n = len(s);`.
  - `#import std.socket;` gates socket creation, connection, `send`, `recv`, and `close`.
- **Diagnostics & explicit non-claims:** `Q090` shape / payload $> 4,096$ B / `concat` $< 2$ args; `Q091` start line; `Q092` header syntax, `CR`/`LF`, or missing/duplicate `Host`; `Q093` `Content-Length` mismatch; `Q094` non-message name at message position; `Q095` inherited `source.socket.v1` refusal; `Q096` undeclared `{ io }`. Parsing arbitrary untrusted wire input (`HTTP-V1-PARSE`), chunked transfer coding (`HTTP-V1-CHUNKED`), persistent/pipelined connections (`HTTP-V1-PERSISTENT`), `100 Continue`, external peers, and TLS (`HTTP-V1-TLS`) remain named follow-ons.

### Example

```qcore
#import std.http;
#import std.socket;
#import std.string;

fn main() -> unit effects { io } {
    let body = "{\"ok\":true}";
    let n = len(body);

    let req = request "POST" "/v1/health" "HTTP/1.1";
    header req "Host" "127.0.0.1:19081";
    header req "Content-Length" dec(n);
    content req body;

    let res = response "HTTP/1.1" 200 "OK";
    header res "Content-Length" "2";
    content res "ok";

    let listener = tcp;
    bind listener 127.0.0.1:19081;
    listen listener;
    let client = tcp;
    connect client 127.0.0.1:19081;
    let server = accept listener;

    send client req;
    recv server req;
    send server res;
    recv client res;

    close server;
    close client;
    close listener;
    return;
}
```

### Implementation & Provenance

- **Cards:** `HTTP-QUIRIN-V1`, `HTTP-V1-COMPUTED-LENGTH`.
- **Implementation:** `crates/qcore-frontend/src/source_http.rs`, `crates/qcore-core/src/std_http.rs`.
- **Reports:** `docs/reports/archive/language-and-tooling/http-quirin-v1/README.md`, `docs/reports/archive/language-and-tooling/http-v1-computed-length/README.md`, [`../examples/http/README.md`](../examples/http/README.md).

---

# Part III — Quantitative and Concurrent .qr Execution Grammars

Part III specifies the surface grammars for quantitative parameter grades and closed indices (`G03`, `G04`), followed by the checked concurrency profiles gated behind `#import std.concurrency;`: scoped tasks with scalar/record copies, inline spawn closures, heap-allocated `Buffer<T>` custody, fractional read shares, disjoint range write leases, and uniform error payloads (`G02a`–`G02bc`, `G02j`), guarded fixed-footprint mutexes (`G02c`), flat-epoch quantitative accounting (`G02e`, `G02f`), and concurrent bit-field footprint checks (`G09`).

---

<a id="g03-source-parameter-usage-grades"></a>
## G03. Source parameter usage grades

### Consumer and Grade Lattice

`G03` is parsed on function and external parameters across `.qr` and `.qv` modules (`frontend.rs`, evaluated over `qcore_core::usage`). Every grade denotes a closed interval $[\text{lo}, \text{hi}] \subseteq \mathbb{N} \cup \{\infty\}$ constraining how many times the callee's dynamic execution paths may consume or observe the parameter.

### Consolidated EBNF

```ebnf
usage_annotation = "use", grade ;
grade            = "zero"
                 | "one"
                 | "affine"
                 | "relevant"
                 | "many"
                 | NAT
                 | "[", NAT, ",", ( NAT | "inf" ), "]" ;
```

### Grade Denotation and Side Conditions

| Written grade | Interval meaning | Semantic discipline |
|---|---|---|
| `use zero` | $[0, 0]$ | Erased / ghost / compile-time-only parameter |
| `use one` | $[1, 1]$ | Strictly linear: consumed on every exit path exactly once |
| `use affine` | $[0, 1]$ | At most once: may be dropped unused or consumed once |
| `use relevant` | $[1, \infty]$ | At least once: must be used on every path |
| `use many` | $[0, \infty]$ | Unrestricted duplicable usage |
| `use N` | $[N, N]$ | Exact natural count $N \in 0..=2^{64}-1$ |
| `use [lo, hi]` | $[\text{lo}, \text{hi}]$ | Explicit interval with $\text{lo} \in 0..=2^{64}-1$ and $\text{lo} \le \text{hi}$ |

- **Bounds and symbolic exclusion:** Finite bounds must fit unsigned `u64` and satisfy `lo <= hi` (`Q000` / `Q100` otherwise). Symbolic arithmetic expressions are not admitted in source parameter `use` annotations; while `qcore_core::usage` implements checked interval addition, scalar multiplication, branch hull ($\sqcup$), and subtyping containment, that semantic algebra does not widen `G03`'s surface grammar.

### Implementation & Provenance

- **Implementation:** `crates/qcore-frontend/src/frontend.rs`, `crates/qcore-core/src/usage.rs`.
- **Specification cross-reference:** [Language specification §L07](QCORE_SPEC_r0_3.md#l07-usage-grades).

---

<a id="g04-closed-quantitative-indices"></a>
## G04. Closed quantitative indices (`index.closed`)

### Consumer and Delimiter Alias

`G04` (`index.closed`, `qcore_frontend::quant_index`) parses and resolves closed natural-number `quant<...>` and `quant[| ... |]` types over declared module `quant axis` dimensions (`Q001`, `Q016`). The axis environment is built and validated unconditionally on every module, even when no `quant` type occurrence appears.

### Consolidated EBNF

```ebnf
quant_type   = "quant", "<",  quant_body, ">"
             | "quant", "[|", quant_body, "|]" ;

quant_body   = ( scalar_index | vector_index ), [ where_clause ] ;
scalar_index = expr ;
vector_index = ( "credit" | "live" ), ",", "{", field, { ",", field }, "}" ;
field        = IDENT, "=", expr ;

where_clause = "where", where_item, { ( "," | "and" ), where_item } ;
where_item   = IDENT, ":=", expr
             | expr, relation, expr
             | "true" ;

relation     = "==" | "!=" | "<" | "<=" | ">" | ">=" ;
expr         = product, { "+", product } ;
product      = atom, [ "*", atom ] ;
atom         = NAT
             | IDENT
             | "(", expr, ")"
             | ( "min" | "max" | "monus" ), "(", expr, ",", expr, ")" ;
```

### Semantic Side Conditions and Evaluation Limits

- **Matched delimiter rule:** `[|` and `|]` are atomic lexer tokens introduced so `where`-clause `<` / `>` comparisons do not collide visually with outer angle brackets. Both spellings produce identical closed-index types, and the opener fixes the required closer (`quant[| ... >` refuses at `Q000`, never repaired).
- **Linear-product restriction:** In `atom * atom`, at least one operand must parse syntactically to a **literal node** (including a parenthesized literal); multiplying two `IDENT`s is refused even if `where` definitions bind both to constants, and unparenthesized `a * b * c` chains are refused.
- **Axis typing and closed `u128` arithmetic (`Q001`, `Q016`):**
  - `scalar_index` is shorthand for `credit` and requires the module to declare **exactly one** `credit` axis. `vector_index` fields must be non-empty, uniquely named, mode-compatible (`credit` vs. `live`), and dimension-checked.
  - Local `IDENT := expr` bindings are pure, fresh, and closed (forward references allowed; dependency cycles, runtime variable references, and unreferenced bindings without an inferable dimension refuse). Every `relation` item must evaluate to `true`.
  - Evaluation uses checked `u128` arithmetic: `monus(a, b)` requires `b <= a` and computes `a - b` (refusing invalid subtraction rather than saturating to `0`); overflow refuses rather than wrapping.
  - `quant<cost, ...>` is refused as an unimplemented profile; `quant<access, ...>` is skipped by `quant_index` for separate handling (never treated as verified source access).
- **Explicit parser guards:** Atom nesting depth is capped at **32** and addition chains at **64** terms (`Q001`).

### Example

```qcore
module contract;
dimension Chunk : Idx + Ordered;
quant axis chunks : credit<Count<Chunk>>;

extern fn keep(q: quant<n where n := 5> use one) -> unit;
extern fn hold(q: quant[|credit, { chunks = 2 * n } where n := 4, n > 0|] use one) -> unit;
```

### Implementation & Provenance

- **Cards:** `R05`, `QUANT-BRACKET-DELIMITERS`.
- **Implementation:** `crates/qcore-frontend/src/quant_index.rs`, `crates/qcore-frontend/src/frontend.rs`.

---

<a id="g02a-scoped-copy-task-execution"></a>
## G02a. Scoped-copy task execution (`source.tasks.copy.v1`)

### Consumer and Profile Requirements

`source.tasks.copy.v1` checks and compiles structured fork/join task modules behind `#import std.concurrency;` and an explicit `concurrency checked;` module declaration. Every function in the module is checked: the entry takes zero parameters and returns `i32` or `unit`; non-recursive, task-free leaf workers accept parameters of any [G02t](#g02t-fixed-width-scalar-types) scalar type or nominal [G07.13](#g0713-the-first-source-level-aggregate-sourcerecordv1) task record (`W48-QR-SCALAR-TYPES-R4`, `W48-QR-TASK-RECORD-CAPTURES`) and return any scalar, `unit`, or task record.

### Consolidated EBNF

```ebnf
task_module   = [ module_header ],
                ( "#import" | "import" ), "std.concurrency", ";",
                "concurrency", "checked", ";",
                { error_decl | record_decl | worker_fn }, entry_fn ;

record_decl   = "type", RECORD_NAME, "{",
                field_decl, { ",", field_decl }, [ "," ], "}" ;
field_decl    = NAME, ":", ( SCALAR_TYPE | owned_buf | RECORD_NAME ) ;
owned_buf     = "own", "<", "Buffer", [ "<", SCALAR_TYPE, ">" ], ">" ;

worker_fn     = "fn", NAME, "(", [ worker_params ], ")", "->", worker_ret,
                [ "effects", "{", task_effect, { ",", task_effect }, "}" ],
                "{", { worker_stmt }, "}" ;
worker_params = worker_param, { ",", worker_param } ;
worker_param  = NAME, ":", SCALAR_TYPE
              | NAME, ":", RECORD_NAME, [ "use", "one" ]
              | G02b_buffer_param ;                          (* §G02b–§G02bc *)
worker_ret    = "unit" | SCALAR_TYPE | RECORD_NAME | owned_buf ;

task_effect   = "spawn" | "join" | "may_block" | "alloc" | "mut"
              | "throws", "<", ERROR_NAME, ">" ;

scope_block   = "thread", "scope", SCOPE_NAME, "{", { scope_stmt }, "}" ;

spawn_expr    = "spawn", SCOPE_NAME, WORKER_NAME, "(", [ captures ], ")"
              | "spawn", SCOPE_NAME, closure ;               (* §G02v *)
captures      = capture, { ",", capture } ;
capture       = ( "copy" | "move" ), NAME, [ field_path ]
              | "read", NAME, [ field_path ], "share", SHARE (* §G02bb–§G02bc *)
              | "lend", NAME, [ field_path ], LOW, "..", HIGH ;

scope_stmt    = "let", NAME, "=", spawn_expr, [ "?" ], ";"
              | "let", NAME, "=", "join", "move", HANDLE_NAME, [ "?" ], ";"
              | "join", "move", HANDLE_NAME, ";"             (* infallible unit worker only *)
              | "let", NAME, "=", RESULT_NAME, "?", ";"
              | match_result
              | record_or_scalar_stmt
              | "raise", ( ERROR_BOUND_NAME | ERROR_NAME, "::", VARIANT ), ";"
              | "return", [ "move" ], [ expr_atom ], ";" ;

match_result  = "match", ( spawn_expr | "join", "move", HANDLE_NAME | RESULT_NAME ), "{",
                  ok_arm, err_arm | err_arm, ok_arm,
                "}" ;
ok_arm        = "Result::Ok",  "{", "value", ":", NAME, "}", "=>", "{", { scope_stmt }, "}" ;
err_arm       = "Result::Err", "{", "error", ":", NAME, "}", "=>", "{", { scope_stmt }, "}" ;
```

### Semantic Side Conditions and Record-Across-Task Rules

- **Explicit lifecycle and failure recovery (`Q300`, `Q600`):**
  - `spawn` dynamically succeeds or raises `SpawnError`; `join move h` consumes the task handle once and yields the worker's typed `Result`; `?` propagates errors (or extracts infallible `Result`s when the worker's declared error set is empty).
  - There is **no implicit join**: scope exits, `return`, `raise`, and `?` failure edges refuse if any spawned handle or unconsumed `Result` remains live (`Q600`). Handled `SpawnError`s inside `match spawn ...` do not require `throws<SpawnError>` unless re-raised.
- **Scalar and record captures (`W48-QR-SCALAR-TYPES-R4`, `W48-QR-TASK-RECORD-CAPTURES`):**
  - **Leaf decomposition:** A task record binding expands to one task slot per scalar or `own<Buffer<T>>` leaf in depth-first declaration order (`bytes` fields from `std.buffer` refuse at `Q500`, `WRITE-LEND-TASKS-EXTENT`; records in composed epoch modules refuse at `Q500`, `W48-QR-TASK-RECORD-EPOCH`).
  - **Whole-record vs. field captures:** Copyable scalar records cross `spawn` via `copy r` (into `param: R`); records containing any `own<Buffer<T>>` leaf cross via `move r` (into `param: R use one`, transferring every owned leaf and duplicating scalar leaves; restored whole on spawn failure). `write` captures and whole-record `read`/`lend` shares refuse by name at `Q600`.
  - **Per-field captures & holes:** `copy r.f` copies a scalar leaf; `move r.buf` moves out one owned leaf, leaving a **hole** in `r` that refuses reads/drops until refilled by `r.buf = fresh_buf;` (deriving `mut`). Two distinct `own<Buffer<T>>` fields of `r` are independent resources and may be leased (`lend r.a ...`, `lend r.b ...`) to two concurrent workers simultaneously.
  - **Record results:** `return r;` (copyable) or `return move r;` (owned-bearing) packs leaves through `join` (`QCORE_TASK_RESULT TASK RECORD N`; F02 reference encoding refuses record entry outcomes rather than flattening them, `W48-QR-TASK-RECORD-F02`).
- **Native host support:** Emits C11 threads on Linux, pthread adapter on macOS, and winpthreads-derived C11 subset on Windows (other targets refuse explicitly).

### Example

```qcore
#import std.concurrency;
concurrency checked;

fn worker(x: i32) -> i32 {
    return x;
}

fn main() -> i32 effects { throws<SpawnError>, spawn, join, may_block } {
    let x = 42;
    thread scope s {
        match spawn s worker(copy x) {
            Result::Ok { value: h } => {
                let y = join move h?;
                return y;
            }
            Result::Err { error: failure } => {
                raise failure;
            }
        }
    }
}
```

### Implementation & Provenance

- **Cards:** `W6-TASK-RECOVERY`, `CT47-COPY-CAPTURE`, `W48-QR-SCALAR-TYPES-R4`, `W48-QR-TASK-RECORD-CAPTURES`.
- **Implementation:** `crates/qcore-frontend/src/frontend_tasks.rs`, `crates/qcore-core/src/scoped_tasks.rs`, `crates/qcore-core/src/transfer.rs`.
- **Reports:** `docs/reports/w6-task-recovery/README.md`, `docs/reports/archive/wave-48/w48-qr-task-record-captures/README.md`.

---

<a id="g02v-closures-with-explicit-capture-lists"></a>
## G02v. Closures with explicit capture lists

### Consumer and Lifting Model

On scoped-task routes ([G02a](#g02a-scoped-copy-task-execution)–[G02bc-errors](#g02bc-tagged-error-payload) and [G02j](#g02j-composed-scoped-tasks-and-rendered-lines)), `spawn SCOPE` may write its leaf worker inline as an explicit-capture closure (`W48-QR-CLOSURES`). There is **zero implicit capture**: every accessed binding from the enclosing function must be listed in `|...|` with its explicit transfer mode, and the parser lifts each closure into a hidden named worker `ENCLOSING#closureN` before semantic checking, emitting C11 byte-identical to a hand-written worker (`QCore.Closures`).

### Consolidated EBNF

```ebnf
closure_spawn = "spawn", SCOPE_NAME, closure ;
closure       = capture_list, "->", worker_ret,
                [ "effects", "{", task_effect, { ",", task_effect }, "}" ],
                "{", { worker_stmt }, "}" ;

capture_list  = "||"                                         (* zero captures: PipePipe token *)
              | "|", closure_cap, { ",", closure_cap }, "|" ;

closure_cap   = G02a_capture, [ "as", NAME ] ;
G02a_capture  = ( "copy" | "move" ), PLACE
              | "read", PLACE, "share", SHARE
              | "lend", PLACE, LOW, "..", HIGH
              | "write", PLACE ;                             (* parsed, refused at Q600 *)
```

### Semantic Side Conditions

- **Explicit capture scope and binder naming (`Q011`, `Q013`):**
  - Parameter binder defaults to `PLACE`'s root name (`copy bonus` binds `bonus`) or final field segment (`copy r.f` binds `f`), overridable via `as NAME` when two field captures share a segment name. Capturing the same place twice or colliding binder names refuses at `Q011`; referencing any uncaptured outer local refuses at `Q013` naming the missing capture.
  - Parameter types are determined uniquely from `PLACE` and mode at the spawn site (`T` for `copy`, `R use one` or `own<Buffer<T>> use one` for `move`, `borrow<Buffer<T>>` for `read ... share`, `lend<Buffer<T>>` for `lend ...`).
- **Required return type & spawn-only placement (`Q000`, `Q038`, `Q600`):**
  - `-> worker_ret` is mandatory (`Q000` if omitted); effect rows (`throws<...>`, `alloc`, `mut`, `io` on `G02j`) follow identical leaf-worker rules.
  - Closures are admitted **only immediately after `spawn SCOPE`**: first-class closure values (`let f = |...| ...;`) refuse at `Q038`/`Q000` (`W48-QR-CLOSURE-VALUES`), nested spawns inside a closure refuse by the leaf-worker rule (`Q600`), and closures on epoch, mutex, bitfield, or FFI routes refuse by name at `Q600`. Capture lists never use `[` or `]` (`[|`/`|]` remain quant delimiters).

### Example

```qcore
#import std.concurrency;
concurrency checked;

type Job { count: i32 }

fn main() -> i32 effects { throws<SpawnError>, spawn, join, may_block } {
    let bonus = 7;
    let job = Job { count: 35 };
    thread scope lanes {
        let h = spawn lanes |copy bonus, copy job.count as base| -> i32 {
            let sum = base + bonus;
            return sum;
        }?;
        let total = join move h?;
        return total;
    }
}
```

### Implementation & Provenance

- **Cards:** `W48-QR-CLOSURES`.
- **Implementation:** `crates/qcore-frontend/src/closures.rs`, `crates/qcore-frontend/src/frontend_tasks.rs`.
- **Reports & Lean:** `docs/reports/archive/wave-48/w48-qr-closures/README.md`, `QCore.Closures`.

---

<a id="g02b-owned-buffer-task-execution"></a>
## G02b. Owned-buffer task execution (`ScopedOwnedTasksV1`)

### Consumer and Heap Generation Identity

`ScopedOwnedTasksV1` extends [G02a](#g02a-scoped-copy-task-execution) with heap-allocated single-payload buffers `own<Buffer>` (default `i32` payload) and typed `own<Buffer<T>>` for any [G02t](#g02t-fixed-width-scalar-types) scalar or IEEE float `T` (`W48-QR-SCALAR-TYPES-R6`, `W48-QR-FLOAT-BUFFER-PAYLOADS`). Each successful `qcore_buffer_new` mints a fresh runtime generation `task * 2^32 + ordinal` (`task` $\in 0..2^{32}-1$, `ordinal` $\in 1..2^{32}-1$), keeping heap resource identity disjoint from local variable slots.

### Consolidated EBNF Additions

```ebnf
buffer_type   = "own", "<", "Buffer", [ "<", SCALAR_TYPE, ">" ], ">" ;
worker_param  = NAME, ":", buffer_type, "use", "one" ;       (* linear grade mandatory *)

alloc_stmt    = "let", NAME, [ ":", buffer_type ], "=",
                "qcore_buffer_new", "(", OPERAND, ")", "?" , ";"
              | "match", "qcore_buffer_new", "(", OPERAND, ")", "{", ... "}" ;

read_stmt     = "let", NAME, "=", "qcore_buffer_read", "(", NAME, [ path ], ")", ";" ;
drop_stmt     = "drop", NAME, [ path ], ";" ;
return_owned  = "return", "move", NAME, ";" ;
```

### Semantic Side Conditions

- **Linearity and spawn rollback (`Q020`, `Q300`, `Q600`):**
  - Owned parameters must write `use one`; root `main` may not take or return `own<Buffer<T>>`.
  - `qcore_buffer_new(v)` requires `alloc` and must either propagate `AllocError` with `?` (requiring `throws<AllocError>`) or handle both `Result` arms via `match`. Failed allocations mint no generation and require any previously live owners to be dropped on the error path.
  - `spawn s worker(move buf)` consumes `buf` on the `Ok` path and **restores `buf` intact** on the `SpawnError` path; `join move h?` delivers returned custody into a fresh binding without reviving the consumed `buf` name.
  - `qcore_buffer_read(buf)` reads the payload without consuming ownership; `drop buf;` and `return move buf;` consume ownership. Every normal and error exit must account for all owned bindings, handles, and `Result`s.

### Example

```qcore
#import std.concurrency;
concurrency checked;

fn worker(buffer: own<Buffer<i64>> use one) -> own<Buffer<i64>> {
    return move buffer;
}

fn main() -> i32 effects { alloc, throws<AllocError>, spawn, join, may_block } {
    let seed: i64 = 42;
    let buffer = qcore_buffer_new(seed)?;
    thread scope s {
        match spawn s worker(move buffer) {
            Result::Ok { value: handle } => {
                let returned = join move handle?;
                let value = qcore_buffer_read(returned);
                drop returned;
                let code = value as i32;
                return code;
            }
            Result::Err { error: failure } => {
                drop buffer;
                return 0;
            }
        }
    }
}
```

### Implementation & Provenance

- **Cards:** `W6-OWNED-SOURCE-TASKS`, `W6-WORKER-ALLOCATION`, `W48-QR-SCALAR-TYPES-R6`, `W48-QR-FLOAT-BUFFER-PAYLOADS`.
- **Implementation:** `crates/qcore-frontend/src/frontend_tasks.rs`, `crates/qcore-core/src/scoped_tasks.rs`.
- **Reports:** `docs/reports/w6-owned-source-tasks/README.md`, `docs/reports/w6-worker-allocation/README.md`.

---

<a id="g02bb-scoped-read-loan"></a>
## G02bb. Scoped read loan (`ScopedReadLoansV1`)

### Consumer and Whole-Share Borrowing

`ScopedReadLoansV1` extends [G02b](#g02b-owned-buffer-task-execution) with a scoped read loan of a parent-owned `Buffer<T>` to a child worker, reclaimed strictly at the creator's `join`. It is selected whenever a checked worker declares a `borrow<Buffer<T>>` parameter without fractional shares or buffer writes.

### Consolidated EBNF Additions

```ebnf
loan_param    = NAME, ":", "borrow", "<", "Buffer", [ "<", SCALAR_TYPE, ">" ], ">" ;
loan_capture  = "read", PLACE, "share", "FULL" ;
worker_read   = "let", NAME, "=", "qcore_buffer_read", "(", LOAN_NAME, ")", ";" ;
```

### Semantic Side Conditions

- **Ungraded non-value loan:** `borrow<Buffer<T>>` carries **no** `use` grade: the worker may call `qcore_buffer_read(payload)` any number of times and cannot return, copy, move, drop, byte-index, or re-lend `payload` (`Q020` / `Q600`).
- **Lender freeze until `join`:** While `share FULL` is outstanding, the parent retains whole-payload `qcore_buffer_read` authority over its own `Buffer<T>` while losing all exclusive authority (mutation, `drop`, `move`, or a second loan all refuse naming `LoanOutstanding`, `Q600`). Failed `spawn` opens no loan and restores exclusive authority immediately; successful `spawn` holds the loan until `join` completes.

### Example

```qcore
#import std.concurrency;
concurrency checked;

fn reader(payload: borrow<Buffer>) -> i32 {
    let seen = qcore_buffer_read(payload);
    return seen;
}

fn main() -> i32 effects { alloc, throws<AllocError>, throws<SpawnError>, spawn, join, may_block } {
    let buffer = qcore_buffer_new(42)?;
    thread scope s {
        let h = spawn s reader(read buffer share FULL)?;
        let parent_view = qcore_buffer_read(buffer);
        let child_view = join move h?;
        drop buffer;
        return child_view;
    }
}
```

### Implementation & Provenance

- **Cards:** `T04` (`REMAINING-T04`), `W48-QR-SCALAR-TYPES-R6`.
- **Implementation:** `crates/qcore-frontend/src/frontend_tasks.rs`, `crates/qcore-core/src/scoped_tasks.rs`.
- **Reports:** `docs/reports/remaining-t04/README.md`.

---

<a id="g02bc-counted-read-shares-and-the-checked-buffer-write"></a>
## G02bc-shares. Counted read shares, range write leases, and checked `Buffer` writes (`ScopedReadSharesV1`)

### Consumer and Fractional / Disjoint Authority Model

`ScopedReadSharesV1` extends [G02bb](#g02bb-scoped-read-loan) along two dual axes over `own<Buffer<T>>`:
1. **Counted read shares (`T05`):** Fractional `read PLACE share S` captures split a per-resource read budget of `FULL` (`1024`, matching `FULL_ACCESS`) across up to **two** concurrent readers (`S1 + S2 <= 1024`), while whole-payload `qcore_buffer_write(buf, val)` demands reconstructed full exclusive authority.
2. **Disjoint range write leases (`WRITE-LEND-TASKS`, `WRITE-LEND-TASKS-EXERCISE`):** `lend PLACE LOW..HIGH` captures transfer exclusive byte-range write authority over `[LOW, HIGH)` (within `0..sizeof(T)`) to concurrent `lend<Buffer<T>>` workers, exercised via rebased byte accesses `qcore_buffer_write(lease, idx, byte)` and `qcore_buffer_read(lease_or_owner, idx)`.

### Consolidated EBNF Additions

```ebnf
lease_param   = NAME, ":", "lend", "<", "Buffer", [ "<", SCALAR_TYPE, ">" ], ">" ;

share_capture = "read", PLACE, "share", share_expr ;
share_expr    = "FULL"
              | NAT
              | "(", share_expr, ")"
              | share_expr, "+", share_expr ;

lease_capture = "lend", PLACE, LOW_BOUND, "..", HIGH_BOUND ;

whole_write   = "let", NAME, "=", "qcore_buffer_write", "(", OWNER_PLACE, ",", OPERAND, ")", ";" ;
byte_write    = "let", NAME, "=", "qcore_buffer_write", "(", LEASE_NAME, ",", CONST_IDX, ",", BYTE_VAL, ")", ";" ;
byte_read     = "let", NAME, "=", "qcore_buffer_read",  "(", ( LEASE_NAME | OWNER_NAME ), ",", CONST_IDX, ")", ";" ;
```

### Semantic Side Conditions

- **Counted read shares (`share S`):**
  - `share_expr` must evaluate to $1 \le S \le 1024$ (`0`, $> 1024$, or named share constants refuse at `Q600`). At most **two** simultaneous read borrowers are admitted per resource, with total lent share $\le 1024$.
  - Owner whole-payload `qcore_buffer_read(buf)` stays admitted during read loans, whereas `qcore_buffer_write(buf, val)`, `drop`, and `move` refuse until **every** outstanding borrower has been joined (`Q600`).
- **Range write leases (`lend PLACE LOW..HIGH` $\to$ `lend<Buffer<T>>`):**
  - `lend` is contextual (recognized before `<Buffer` in worker parameters and at capture heads; a worker takes **at most one** `lend<Buffer<T>>` parameter, `Q020`).
  - Bounds must satisfy `0 <= LOW < HIGH <= sizeof(T)` (`bool` payloads refuse byte writes; `isize`/`usize` payloads require a recorded target; empty `LOW == HIGH` leases refuse as `InvalidLease`, `Q600`).
  - **Disjointness & complete owner exclusion:** Two concurrent leases on the same `Buffer<T>` must satisfy `ownership::axes_overlap == false` (`OverlappingLeases`, max two writers `TooManyWriters`, `Q600`), and read shares and write leases on the same resource never mix. While **any** write lease is outstanding, the owner holds **zero** whole-payload or byte-read authority (`LeaseOutstanding`, `Q600`) until `join` reclaims the lease (`QCore.WriteLendTasks.no_byte_has_two_write_authorities`).
  - **Split worker/creator footprint check (`QCore.WriteLendTasksExercise`):** Arity selects byte operations: `qcore_buffer_write(lease, idx, byte)` (3 args, `lease` only — owners have no 3-arg byte write) and `qcore_buffer_read(name, idx)` (2 args, `lease` or unleased `owner` — `borrow<Buffer>` read shares have no byte read, `Q020`). `idx` is a check-time constant relative to `lease`'s `LOW` base (`0` for owner); the worker summarizes its maximum accessed offset, and the creator's `spawn` site verifies `LOW + max_end <= HIGH` (`Q600`). Emitted C writes `((unsigned char *)&buffer->payload)[base + idx]` (distinct C11 §3.14 byte locations).

### Example

```qcore
#import std.concurrency;
concurrency checked;

fn paint_pair(slice: lend<Buffer<u32>>) -> unit {
    let w0 = qcore_buffer_write(slice, 0, 17);
    let w1 = qcore_buffer_write(slice, 1, 34);
    return;
}

fn main() -> i32 effects { alloc, throws<AllocError>, throws<SpawnError>, spawn, join, may_block } {
    let init: u32 = 0;
    let buf = qcore_buffer_new(init)?;
    thread scope s {
        let h_lo = spawn s paint_pair(lend buf 0..2)?;
        let h_hi = spawn s paint_pair(lend buf 2..4)?;
        join move h_lo;
        join move h_hi;
        let b0 = qcore_buffer_read(buf, 0);
        let b2 = qcore_buffer_read(buf, 2);
        drop buf;
        let sum = (b0 as i32) + (b2 as i32);
        return sum;
    }
}
```

### Implementation & Provenance

- **Cards:** `T05` (`REMAINING-T05`), `WRITE-LEND-TASKS`, `WRITE-LEND-TASKS-EXERCISE`, `TASK-ROUTE-TSAN`, `TSAN-ROWS-2`, `W48-QR-SCALAR-TYPES-R6`.
- **Implementation:** `crates/qcore-frontend/src/frontend_tasks.rs`, `crates/qcore-core/src/scoped_tasks.rs`.
- **Reports & Lean:** `docs/reports/remaining-t05/README.md`, `docs/reports/archive/language-and-tooling/write-lend-tasks/README.md`, `docs/reports/write-lend-tasks-exercise/README.md`, `QCore.WriteLendTasks`, `QCore.WriteLendTasksExercise`.

---

<a id="g02bc-tagged-error-payload"></a>
<a id="g02bc-errors"></a>
## G02bc-errors. Tagged error payload (`ScopedErrorPayloadV1`)

### Consumer and Uniform Failure-Custody Transfer

`ScopedErrorPayloadV1` extends [G02b](#g02b-owned-buffer-task-execution) so a failing worker can transfer custody of **one `own<Buffer>`** back to its parent inside a nominal error variant across the completion capsule and `join`. It is selected whenever any checked error declaration carries a payload variant.

### Consolidated EBNF Additions

```ebnf
payload_error_decl = "error", ERROR_NAME, "{",
                     payload_variant, { payload_variant }, "}" ;
payload_variant    = VARIANT_NAME, "(", "own", "<", "Buffer", ">", ")", ";" ;

raise_payload      = "raise", ERROR_NAME, "::", VARIANT_NAME, "(", BUFFER_NAME, ")", ";" ;
take_payload       = "let", NAME, "=", "qcore_error_payload", "(", ERROR_BINDING, ")", ";" ;
```

### Semantic Side Conditions

- **Uniform error-row rule (`Q300`, `Q600`):** Every variant of an `error E { ... }` type must either carry `(own<Buffer>)` or carry no payload; mixed payload/payload-free variants, scalar payloads, `borrow<Buffer>` payloads, or multi-field payloads refuse at the declaration. Builtin `AllocError` and `SpawnError` are always payload-free, and the root entry `main` may never declare `throws<E>` for a payload-carrying `E`.
- **Single-take extraction (`qcore_error_payload`):**
  - `raise E::Variant(buf);` consumes `buf` on the worker's error exit (all other worker-owned buffers must be dropped beforehand).
  - In the parent's `Result::Err { error: failure }` arm on `join move h`, `let recovered = qcore_error_payload(failure);` extracts the `own<Buffer>` **exactly once**. A second `qcore_error_payload(failure)` refuses, and leaving the `Err` arm (or re-raising `failure`) before extracting the payload refuses as an unreleased obligation (`Q600`). Once extracted, `recovered` is an ordinary `own<Buffer>` governed by linear drop/move rules.

### Example

```qcore
#import std.concurrency;
concurrency checked;

error Recovery { Failed(own<Buffer>); }

fn worker(payload: own<Buffer> use one) -> i32 effects { alloc, throws<Recovery> } {
    match qcore_buffer_new(7) {
        Result::Ok { value: scratch } => {
            let seen = qcore_buffer_read(payload);
            drop payload;
            drop scratch;
            return seen;
        }
        Result::Err { error: exhausted } => {
            raise Recovery::Failed(payload);
        }
    }
}

fn main() -> i32 effects { alloc, throws<AllocError>, spawn, join, may_block } {
    let buffer = qcore_buffer_new(42)?;
    thread scope s {
        match spawn s worker(move buffer) {
            Result::Ok { value: handle } => {
                match join move handle {
                    Result::Ok { value: seen } => { return seen; }
                    Result::Err { error: failure } => {
                        let recovered = qcore_error_payload(failure);
                        let value = qcore_buffer_read(recovered);
                        drop recovered;
                        return value;
                    }
                }
            }
            Result::Err { error: refused } => {
                drop buffer;
                return 0;
            }
        }
    }
}
```

### Implementation & Provenance

- **Cards:** `T06` (`REMAINING-T06`).
- **Implementation:** `crates/qcore-frontend/src/frontend_tasks.rs`, `crates/qcore-core/src/scoped_tasks.rs`.
- **Reports:** `docs/reports/archive/early-waves/remaining-t06/README.md`.

---

<a id="g02c-guarded-fixed-footprint-mutex"></a>
## G02c. Guarded fixed-footprint mutex (`source.mutex.guard.v1`)

### Consumer and Route Precedence

`CheckedMutexGuardsV1` (`source.mutex.guard.v1`) lowers lexical lock guards, invariant open/close protocols, refinement facts, opt-in lock ranks, monotonic footprint growth, guard-local child joins, and `share ... under` task crossings onto `CheckedMutexInvariantV1` (`qcore_core::mutex_invariant`, `Q610`–`Q627`). Selected by `mutex [rank N] guards ...` ahead of scoped-task routes, it does not mix with `G02a`'s `spawn`/`join` expressions (`Q600`).

### Consolidated EBNF

```ebnf
mutex_module = [ module_header ],
               ( "#import" | "import" ), "std.concurrency", ";",
               { error_decl }, entry_fn ;

entry_fn     = "fn", NAME, "(", ")", "->", "unit", "{", root_scope, "}" ;
root_scope   = "thread", "scope", NAME, "{", { outer_stmt }, "}" ;

outer_stmt   = "let", NAME, "=", "protected", ( NAT | SCALAR_TYPE ), ";"
             | "let", NAME, "=", "mutex", "guards", NAME, ";"
             | "let", NAME, "=", "mutex", "rank", NAT, "guards", NAME, ";"
             | "let", NAME, "=", "fact", NAME, ";"
             | ( "read" | "write" ), NAME, ";"
             | "wait", NAME, ";"
             | "use", NAME, ";"
             | child_scope
             | "with", "lock", NAME, "as", NAME, "{", { inner_stmt }, "}" ;

inner_stmt   = "open", NAME, ";"
             | "close", NAME, [ "unchanged" ], ";"
             | "grow", NAME, "guards", NAME, ";"
             | ( "read" | "write" ), NAME, "with", NAME, ";"
             | "let", NAME, "=", "borrow", NAME, "with", NAME, ";"
             | "drop", NAME, ";"
             | "assert", NAME, "with", NAME, ";"
             | "use", NAME, ";"
             | "wait", NAME, ";"
             | child_scope
             | "thread", "scope", NAME, "under", NAME, "{", { child_stmt }, "}"
             | "raise", NAME, "::", NAME, ";" ;

child_scope  = "thread", "scope", NAME, [ "share", NAME, "under", NAME ],
               "{", { child_stmt }, "}" ;
child_stmt   = (* any `outer_stmt` or `inner_stmt` except `let` resource/mutex/fact
                  declarations and `drop` of a parent borrow *) ;
```

### Semantic Side Conditions and Native Emission Shapes

- **Fixed/grown footprint, `open`/`close` witnesses, and guard borrows (`Q610`–`Q627`):**
  - `let buf = protected (NAT | SCALAR_TYPE);` (`isize`/`usize` requires a recorded target, `Q600`) creates a shared resource; `let m = mutex guards buf;` consumes `FULL_ACCESS` over `buf`, refusing duplicate mutexes (`Q617`) and unguarded `read`/`write` (`Q611`).
  - Inside `with lock m as g { ... }`, `g` is lexically scoped (`Q013` outside). Writing requires `open g;` first (`Q613`), and normal exit or `raise` requires the invariant closed (`Q614`) and all `borrow buf with g` handles dropped (`Q615`).
  - `close g;` supplies a declared restoration witness (recording `InvariantPredicateDeclared` on the certificate); `close g unchanged;` supplies a **checked** witness admitted only when every operation since `open g;` was a `read` or `use` of a fact (`Q600` otherwise, never downgraded).
  - **`grow g guards r;` (`CT34-GROWTH`):** Requires `g`'s invariant open (`Q626`), `r` unprotected (`Q617`), creator full share over `r` (`Q621`), and mints a fresh invariant identity (`Q627`), permanently adding `r` to `m`'s footprint (`close g unchanged;` is refused in a grown section).
- **Guard-scoped facts (`T03-F`):** `let f = fact buf;` declares a refinement fact; `assert f with g;` mints an active fact valid only until `g`'s critical section ends (`use f;` after unlock or in a child task refuses at `Q616`; unasserted `use f;` refuses at `Q600`).
- **Opt-in lock ranks (`CT33-RANKS`):** Without `rank`, nested `with lock` refuses at `Q622`. Declaring `mutex rank N guards r;` (`N` $\in 0..=2^{32}-1$) enables module-wide lock ordering: **every** mutex in the module must then declare a rank (`Q625`), and acquiring a mutex of rank $\le$ the currently held rank refuses at `Q624` (establishing acyclicity of declared lock acquisitions, not general liveness/deadlock freedom).
- **Child scopes, `under g`, and `share buf under m` (`T03-F`, `T03-GUARD-JOIN`, `MUTEX-SHARE-CROSS`):**
  - Plain `thread scope c { ... }` inside a critical section refuses (`Q600 JoinWhileGuardHeld`). Opt-in `thread scope c under g { ... }` (`GuardLocalJoin`) admits joining inside `g` only when `g` is the sole held lock, closed with no live borrow, and `c` (and its descendants) touches no mutex, guard, fact, or shared resource.
  - **`thread scope d share buf under m { ... }` (`CT46` / rule 15o):** Transfers `MutexShare` over `buf` across the child task boundary (`qcore_core::transfer` checks protocol request `MutexGuard`, crossing gates check absent/mismatched mutex footprint at `Q600`, and unguarded child access hits `Q611`).
  - **Two native C11 emission shapes (`MUTEX-SHARE-CROSS-EMISSION`):** `mutex_c_emit::classify_trace` emits C11 (`mtx_t`, plus `thrd_t` for children) for (1) single-task traces and (2) non-nested fork/join traces whose child events are contiguous between `Spawn` and `Join` (with adjacent operation/log emission, witnessing a real thread boundary without claiming contention or C11 weak-memory race proofs). Traces with guard-local joins, nested spawns, interleaved parent/child events, `wait`, or child `raise` exits check cleanly and publish **without C**.

### Pinned Canonical Example (`source_mutex.rs`)

```qcore
#import std.concurrency;
error Fault { Broken; }
fn main() -> unit {
    thread scope s {
        let buf = protected 4;
        let m = mutex guards buf;
        with lock m as g {
            open g;
            write buf with g;
            close g;
            let b = borrow buf with g;
            read buf with g;
            drop b;
        }
        with lock m as g2 { read buf with g2; }
        let f = fact buf;
        with lock m as g3 {
            assert f with g3;
            open g3;
            read buf with g3;
            use f;
            close g3 unchanged;
        }
        thread scope c {
            with lock m as gc { read buf with gc; }
        }
        thread scope d share buf under m {
            with lock m as gd { read buf with gd; }
        }
    }
}
```

### Implementation & Provenance

- **Cards:** `T03`, `T03-F`, `T03-GUARD-JOIN`, `CT33-RANKS`, `CT34-GROWTH`, `MUTEX-SHARE-CROSS`, `MUTEX-SHARE-CROSS-EMISSION`, `MUTEX-SHARE-CROSS-ARMS`, `W48-QR-SCALAR-TYPES-R6`.
- **Implementation:** `crates/qcore-frontend/src/source_mutex.rs`, `crates/qcore-core/src/mutex_invariant.rs`, `crates/qcore-backend-c/src/mutex_c_emit.rs`, `crates/qcore-frontend/tests/source_mutex.rs`.
- **Reports & Lean:** `docs/reports/remaining-t03/README.md`, `docs/reports/archive/early-waves/remaining-ct33-ranks/README.md`, `docs/reports/archive/early-waves/remaining-ct34-growth/README.md`, `docs/reports/mutex-share-cross/README.md`, `docs/reports/mutex-share-cross-emission/README.md`, `QCore.MutexShareCrossEmission`, `QCore.MutexShareCrossArms`.

---

<a id="g02e-flat-epoch-quantitative-bound"></a>
## G02e. Flat-epoch quantitative bound (`source.epoch.bound.v1`)

### Consumer and Check-Only Accounting Model

`source.epoch.bound.v1` is selected ahead of non-epoch concurrency routes when a `thread scope` carries the `epoch` keyword **without** `concurrency checked;`. It lowers declared memory and effort quantities onto `FlatEpochBoundV1` (`qcore_core::epoch_accounting`, `Q630`–`Q641`) as a **check-only** accounting verification (emitting no C11 and running no native execution).

### Consolidated EBNF

```ebnf
epoch_module  = [ module_header ],
                ( "#import" | "import" ), "std.concurrency", ";",
                entry_fn ;
entry_fn      = "fn", NAME, "(", ")", "->", "unit", "{", epoch_scope, "}" ;
epoch_scope   = "thread", "scope", NAME, "epoch", "{", { header | worker_block }, "}" ;

header        = shared_cell | allocation | overhead | latency_claim ;
shared_cell   = "let", NAME, "=", "share", NAT, ";" ;
allocation    = "bound", NAT, "effort", NAT, ";" ;
overhead      = "overhead", "scope", NAT, "thread", NAT, [ "measured", NAT ], ";" ;
latency_claim = "bound", "effort", "as", "latency", ";" ;

worker_block  = "thread", "scope", NAME, "{", { worker_clause }, "}" ;
worker_clause = "read", NAME, ";"
              | "write", NAME, ";"
              | "peak", NAT, ";"
              | "effort", NAT, ";"
              | "credit", NAT, "effort", NAT, ";"
              | "retained", NAT, ";"
              | "join", "(", ( "move" | "drop" | "close" ), ")", ";"
              | error_arm
              | worker_block ;

error_arm     = "error", "(", ( "move" | "drop" | "close" ), ")", ";"
              | "error", "(", "move", NAT, "effort", NAT, ")", ";" ;
```

### Semantic Side Conditions

- **Cardinality and `u64` bounds:** Every `NAT` is range-checked at `u64` (never wrapped). `allocation` and `overhead` appear **exactly once** per epoch; `credit` appears **exactly once** per worker; `peak`, `effort`, `retained`, and `join(...)` appear at most once per worker. Parentheses on `join(...)` are mandatory so the scoped-task scanner does not claim bare `join`.
- **Represented model refusals (`Q630`–`Q641`):**
  - `join(move)` transfers the child's retained result to the creator; `join(drop)` releases it at join; `join(close)` claims liveness ended without transfer/release (`Q634`); omitting `join(...)` leaves an unjoined worker (`Q635`).
  - `write NAME;` (`SharedMutation`) and nested `worker_block` (`NestedTask`) refuse at `Q632`; reading an undeclared cell refuses at `Q631`; duplicate `read NAME;` in one worker or `error(...)` on this declared-shape route refuses at `Q630`; `bound effort as latency;` refuses at `Q639` (no schedule/wall-clock model); `overhead ... measured RUNS;` refuses at `Q640`.
  - Certificates carry declared premises `DeclaredTaskFootprints`, `DeclaredRuntimeOverhead`, `DisjointSharedStorageCells`, and `NoScheduleModel`.

### Example

```qcore
#import std.concurrency;

fn main() -> unit {
    thread scope ep epoch {
        let table = share 64;
        bound 512 effort 100;
        overhead scope 32 thread 16;

        thread scope w1 {
            read table;
            credit 128 effort 40;
            peak 96;
            effort 30;
            retained 32;
            join(drop);
        }
    }
}
```

### Implementation & Provenance

- **Cards:** `T08-EPOCH-SRC` (`REMAINING-T08-EPOCH-SRC`), `CT23-CREDIT-ERROR-PATH`.
- **Implementation:** `crates/qcore-frontend/src/source_epoch.rs`, `crates/qcore-core/src/epoch_accounting.rs`.
- **Reports:** `docs/reports/archive/early-waves/remaining-t08-epoch-src/README.md`.

---

<a id="g02f-flat-epoch-bound-over-a-checked-task-region"></a>
## G02f. Flat-epoch bound over a checked task region (`source.epoch.tasks.v1`)

### Consumer and Plan-to-Task Reconciliation

`source.epoch.tasks.v1` is selected ahead of [G02e](#g02e-flat-epoch-quantitative-bound) when a module carries **both** `concurrency checked;` and a `thread scope ... epoch { ... }` plan as the entry's first statement, followed by a real scoped-task region ([G02a](#g02a-scoped-copy-task-execution)–[G02bc-errors](#g02bc-tagged-error-payload)) and an optional `error region` cleanup epilogue. Emission and native execution are byte-for-byte those of the underlying scoped-task route, while `FlatEpochBoundV1`, `EpochCreditRecoveryV1`, and `error_region` check the quantitative plan and cleanup duties on top.

### Consolidated EBNF

```ebnf
composed_entry  = "fn", NAME, "(", ")", "->", ( "unit" | "i32" ),
                  [ "effects", "{", task_effect, { ",", task_effect }, "}" ],
                  "{", epoch_plan, { task_or_region_stmt }, "}" ;

epoch_plan      = "thread", "scope", NAME, "epoch", "{", { plan_header | worker_row }, "}" ;
plan_header     = allocation | overhead | latency_claim ;     (* G02e headers minus `shared_cell` *)

worker_row      = "thread", "scope", WORKER_NAME, "{", { row_clause }, "}" ;
row_clause      = "move", ";" | "copy", ";" | "read", ";"
                | "peak", NAT, ";" | "effort", NAT, ";"
                | "credit", NAT, "effort", NAT, ";"
                | "retained", NAT, ";"
                | "join", "(", ( "move" | "drop" | "close" ), ")", ";"
                | error_arm ;

region_epilogue = "error", "region", REGION_NAME, "{",
                    { "credit", NAT, ";"
                    | "effort", NAT, ";"
                    | "let", NAME, "=", "qcore_buffer_new", "(", NAT, ")", "?", ";"
                    | "drop", NAME, ";"
                    | "move", NAME, ";" },
                    "qcore_check", "(", "false", ",", ERROR_NAME, ")", ";",
                  "}",
                  "catch", "(", ERROR_NAME, ")",
                  [ "quant", "open", REGION_NAME, "as", PACK_BINDING ],
                  "{", { "discharge", PACK_BINDING, ";" }, "}" ;
```

### Semantic Side Conditions

- **1-to-1 `TrySpawn` correspondence (`Q600`, `Q630`–`Q641`):**
  - Rows naming `WORKER_NAME` match that worker's `TrySpawn` sites in source order (`Q600` if mismatched). Ordered `move;`, `copy;`, and place-free `read;` (`REGION-JOINS-LOAN`, matching `read PLACE share N` captures without contributing epoch bytes because creator-owned buffers sit outside the epoch allocation) must match the spawn's capture modes.
  - `retained BYTES;` is required iff the worker returns `own<Buffer<T>>`. `join(...)` is optional and, if written, must equal the disposal derived from the checked task IR.
- **Creator error-edge credit recovery (`CT23-CREDIT-ERROR-PATH`):**
  - Optional `error_arm` (`error(drop);` default when omitted) specifies how derived `spawn_error` and fallible `join` edges treat the row's credit grant: `error(move);` restores all credit, `error(move B effort E);` restores `(B, E)` (`Q636` if exceeding the grant; restoration does not reduce `granted_credits`), `error(drop);` spends all credit, and `error(close);` refuses at `Q635`.
- **Optional `error region` epilogue (`REGION-CREDIT-JOINS`, `REGION-JOINS-DUTIES`, `REGION-JOINS-LOAN`, `CT24-CLEANUP-TASK-COMPOSITION`):**
  - Admitted either as a direct task-scope epilogue **after every `spawn` and `join`** (`Q600` if any spawn/join span follows it) or inside the `Result::Ok` arm of a direct `match spawn`.
  - Supports local `credit`/`effort` remainders, fresh `qcore_buffer_new(N)?` acquisitions, and `move NAME;` **adoption** of a live parent `own<Buffer>` (for example, destroying an owner after workers holding counted read shares have joined — CT25).
  - Pack obligations are checked at `Q302`–`Q309` (`quant open ... as p { discharge p; }`), and the region erases cleanly into straight-line `drop NAME;` statements checked and emitted by the unedited task route.

### Example

```qcore
#import std.concurrency;
concurrency checked;
error Abort;

fn reader(b: borrow<Buffer>) -> i32 {
    let v = qcore_buffer_read(b);
    return v;
}

fn main() -> i32 effects { alloc, throws<AllocError>, throws<SpawnError>, spawn, join, may_block } {
    thread scope plan epoch {
        bound 256 effort 64;
        overhead scope 16 thread 8;
        thread scope reader {
            read;
            credit 64 effort 16;
            error(move);
        }
    }
    let owner = qcore_buffer_new(9)?;
    thread scope s {
        let h = spawn s reader(read owner share 512)?;
        let out = join move h?;
        error region cleanup {
            credit 12;
            move owner;
            qcore_check(false, Abort);
        } catch (Abort) quant open cleanup as pack {
            discharge pack;
        }
        return out;
    }
}
```

### Implementation & Provenance

- **Cards:** `T08-COMPOSE`, `CT23-CREDIT-ERROR-PATH`, `REGION-CREDIT-JOINS`, `REGION-JOINS-DUTIES`, `REGION-JOINS-LOAN`, `CT24-CLEANUP-TASK-COMPOSITION`, `W48-QR-SCALAR-TYPES-R6`.
- **Implementation:** `crates/qcore-frontend/src/source_epoch_tasks.rs`, `crates/qcore-core/src/epoch_credit_recovery.rs`, `crates/qcore-core/src/error_region.rs`.
- **Reports:** `docs/reports/archive/early-waves/remaining-t08-compose/README.md`, `docs/reports/archive/language-and-tooling/ct23-credit-error-path/README.md`, `docs/reports/archive/language-and-tooling/region-credit-joins/README.md`, `docs/reports/region-joins-duties/README.md`, `docs/reports/archive/language-and-tooling/region-joins-loan/README.md`.

---

<a id="g02j-composed-scoped-tasks-and-rendered-lines"></a>
## G02j. Composed scoped tasks and rendered lines (`source.std-io-tasks.v1`)

### Consumer and Windowed Observation Relation

`source.std-io-tasks.v1` is selected for modules satisfying `has_source_tasks && has_source_std_io && !has_source_extern` (after epoch and mutex routes, and before standalone task routes). It composes the full scoped-task surface ([G02a](#g02a-scoped-copy-task-execution)–[G02bc-errors](#g02bc-tagged-error-payload), including closures [G02v](#g02v-closures-with-explicit-capture-lists) and task records) with [G02g](#g02g-rendered-lines-stdfmt-stdio-and-stdlogger)'s rendered scalar lines (`print`, `log`, `flush`).

### Consolidated EBNF Additions

```ebnf
composed_task_stmt = task_stmt
                   | "print", RENDERING, TASK_OPERAND, ";"
                   | "log", LEVEL, RENDERING, TASK_OPERAND, ";"
                   | "flush", ";" ;

TASK_OPERAND       = [ "-" ], INTEGER
                   | NAME                                    (* live scalar slot *)
                   | NAME, field_path ;                      (* scalar leaf of a task record *)

task_effect        = "alloc" | "mut" | "spawn" | "join" | "may_block"
                   | "throws", "<", ERROR_NAME, ">"
                   | "io" ;                                  (* admitted on this route *)
```

### Semantic Side Conditions and `qcore.obs-reordering.v1`

- **Parent vs. worker placement (`OBS-REORDERING`, `Q050`):**
  - **Parent-only lines:** Because worker task-trace events are buffered privately and replayed by the parent thread at `join`, parent `print`/`log` lines have a deterministic byte-for-byte stream order.
  - **Spawned worker lines:** Admitted when the worker's **own** `effects { ... }` row declares `io` (`Q050` if missing, or if a line appears in an unspawned helper function). Placing a line inside a worker declares the windowed observation relation **`qcore.obs-reordering.v1`**, comparing streams window-by-window (bounded by points where zero workers are live) and per-emitter thread.
- **Operands (`Q050`):** Any [G02t](#g02t-fixed-width-scalar-types) scalar slot or scalar record leaf `r.f` computed by the task model (`bool` refuses `hex` at `Q050`); owned `Buffer`s, loans, leases, whole records, and `borrow NAME` refuse (`Q050`). Erasing source lines leaves the exact underlying task trace (`QCore.ComposedTasks.erasing_the_source_lines_leaves_the_task_trace`).

### Example

```qcore
#import std.concurrency;
#import std.fmt;
#import std.io;
concurrency checked;

fn worker(x: i32) -> i32 effects { io } {
    print dec x;
    return x;
}

fn main() -> i32 effects { io, throws<SpawnError>, spawn, join, may_block } {
    thread scope s {
        let h = spawn s worker(copy 7)?;
        let v = join move h?;
        v |> print dec;
        return 0;
    }
}
```

### Implementation & Provenance

- **Cards:** `COMPOSE-CONCURRENCY`, `OBS-REORDERING`, `W48-QR-SCALAR-TYPES-R4`, `W48-QR-TASK-RECORD-CAPTURES`.
- **Implementation:** `crates/qcore-frontend/src/source_std_io_tasks.rs`, `crates/qcore-core/src/obs_reordering.rs`.
- **Design & Lean:** [`design/OBS_REORDERING_V1.md`](design/OBS_REORDERING_V1.md), `QCore.ComposedTasks`.

---

<a id="g09-concurrent-field-footprint-questions"></a>
## G09. Concurrent field-footprint questions (`source.bitfield-concurrency.v1`)

### Consumer and Property-Only Scope

`source.bitfield-concurrency.v1` is a **property-only** checker (emitting no C11 and claiming neither task failure recovery nor lifetime safety) that routes explicit byte-field and C bit-field record layouts to `DisjointFootprintsV1` (`Q650`–`Q667`).

### Consolidated EBNF

```ebnf
bitfield_module = [ module_header ],
                  ( "#import" | "import" ), "std.concurrency", ";",
                  "concurrency", "checked", ";",
                  layout_record, worker_fn, { worker_fn }, entry_fn ;

layout_record   = "type", RECORD_NAME, "{",
                  layout_field, { ",", layout_field }, [ "," ], "}" ;
layout_field    = NAME, ":",
                  ( "field", "<", BYTE_OFFSET, ",", BYTE_SIZE, ">"
                  | "bits", "<", STORAGE_UNIT, ",", BIT_OFFSET, ",", BIT_WIDTH,
                    ",", BYTE_OFFSET, ",", BYTE_SIZE, ">" ) ;

worker_fn       = "fn", NAME, "(", PARAM, ":", "i32", ")", "->", "unit",
                  "effects", "{", "mut", "}",
                  "{", "write", PARAM, ";", "return", ";", "}" ;

entry_fn        = "fn", NAME, "(", ")", "->", "unit",
                  "effects", "{", "spawn", ",", "join", ",", "mut", "}",
                  "{",
                    "let", OBJ, "=", RECORD_NAME, ";",
                    "thread", "scope", SCOPE, "{",
                      spawn_write, spawn_write, { spawn_write },
                      join_stmt,   join_stmt,   { join_stmt },
                    "}",
                    "return", ";",
                  "}" ;

spawn_write     = "let", HANDLE, "=", "spawn", SCOPE, WORKER,
                  "(", "write", OBJ, ".", FIELD, ")", "?", ";" ;
join_stmt       = "let", RES, "=", "join", "move", HANDLE, "?", ";" ;
```

### Semantic Side Conditions

- **Contextual layout keywords:** `field` and `bits` are recognized contextually inside `layout_record` without reserving new global keywords. Layout accuracy remains an explicit declared ABI premise on the certificate.
- **Storage-unit race rule (`Q661`):** Concurrent `write OBJ.FIELD` spawns into byte-overlapping fields — or into **two `bits<...>` bit-fields sharing the same `STORAGE_UNIT`, even when their bit ranges `[BIT_OFFSET, BIT_OFFSET + BIT_WIDTH)` are disjoint** — refuse at `Q661` (`SameBitfieldStorageUnit`, reflecting C11 §3.14 non-atomic read-modify-write storage units). Admitted programs establish pairwise byte/storage-unit disjointness across the successful `spawn`/`join` window only.

### Example

```qcore
#import std.concurrency;
concurrency checked;

type PacketHeader {
    flags: bits<0, 0, 4, 0, 2>,
    length: field<2, 2>,
}

fn touch(slot: i32) -> unit effects { mut } {
    write slot;
    return;
}

fn main() -> unit effects { spawn, join, mut } {
    let pkt = PacketHeader;
    thread scope s {
        let h1 = spawn s touch(write pkt.flags)?;
        let h2 = spawn s touch(write pkt.length)?;
        let r1 = join move h1?;
        let r2 = join move h2?;
    }
    return;
}
```

### Implementation & Provenance

- **Cards:** `CT10-SOURCE-LAYOUT`.
- **Implementation:** `crates/qcore-frontend/src/source_bitfield.rs`, `crates/qcore-core/src/disjoint_footprints.rs`.
- **Reports:** `docs/reports/ct10-source-layout/README.md`.

---

# Part IV — Native C Interoperability and .qrlib Linkage Grammars

Part IV specifies the source profiles and declarative linkage manifests through which `.qr` executable and library modules call external C symbols, bind native build artifacts, and exchange C `struct` records across the FFI boundary. Every route in this part enforces explicit bundled module imports (`std.extern`, `std.extern.unsafe`, `std.ffi.unsafe`), separates scalar data bindings from custody-bearing resource identities, and records every external contract, unchecked waiver, header digest, and linkage binding as an auditable premise on the published certificate.

---

<a id="g02d-bounded-extern-calls"></a>
## G02d. Bounded contracted extern calls (`source.extern-call.v1`)

### Consumer and Route Selection

`source.extern-call.v1` checks and lowers flat sequential `.qr` entry functions that invoke contracted external C functions. Route selection chooses `source.extern-call.v1` after `source.ffi.v1` ([G02u](#g02u-imported-c-structs-sourceffiv1)), after every concurrent route (`source.tasks.v1`, `source.mutex.v1`, `source.epoch.tasks.v1`, `source.bitfield-concurrency.v1`), and after the composed `std.io` + `extern` route ([G02i-extern-io](#g02i-composed-extern-calls-and-rendered-lines)), whenever a source file declares at least one `extern fn` and a function body calls it without also emitting `std.io` / `std.logger` lines. A module that mixes `extern` calls with task spawns, mutex locks, or error regions stays on its concurrent route and retains that route's refusal surface.

Every module on this route lowers simultaneously into two checked representations — no partial lowering or optimistic fallback exists:
1. An `ExternProgram` validated by `qcore_core::extern_call` for custody transitions, error-arm cleanup edges, scalar/float type rules, and certificate premises.
2. A `FunctionSemantics` bundle validated by all established sequential passes (`qcore_core::ownership`, `qcore_core::cleanup_lowering`, `qcore_core::error_region`, `qcore_core::resource_flow`, `qcore_core::typed_errors`).

### Consolidated Grammar

```ebnf
extern_module    = [ module_header ],
                   { import_stmt | library_binding },
                   { error_decl | extern_decl | entry_fn } ;

import_stmt      = ( "#import" | "import" ), "std.extern", ";" ;
library_binding  = "extern", "library", NAME, ";" ;                 (* optional, §G02d.3 *)

error_decl       = "error", ERROR_NAME, ";" ;
extern_decl      = "extern", "fn", NAME, "(", [ param_list ], ")",
                   "->", result_abi_type, ";" ;
param_list       = param, { ",", param } ;
param            = NAME, ":", param_abi_type ;

param_abi_type   = scalar_abi_type | pointer_abi_type ;
result_abi_type  = "unit" | param_abi_type ;
scalar_abi_type  = "bool"
                 | "i8" | "i16" | "i32" | "i64" | "isize"
                 | "u8" | "u16" | "u32" | "u64" | "usize"
                 | "f32" | "f64" ;
pointer_abi_type = "ptr", "<", "mut", ",", "u8", ">"
                 | "ptr", "<", "const", ",", "u8", ">"
                 | "ptr", "<", "mut", ",", "ptr", "<", "mut", ",", "u8", ">", ">" ;

entry_fn         = "fn", NAME, "(", ")", "->", "unit",
                   [ "effects", "{", effect_list, "}" ],
                   "{", { extern_stmt }, "}" ;
effect_list      = effect, { ",", effect } ;
effect           = "io" | "alloc" | "mut"
                 | "partial_eq" | "partial_ord"
                 | "throws", "<", "ArithmeticError", ">" ;

extern_stmt      = acquisition_stmt
                 | scalar_binding
                 | value_statement
                 | float_literal
                 | scalar_literal
                 | plain_call
                 | "return", ";" ;

acquisition_stmt = "let", NAME, "=", CALLEE, "(", [ arg_list ], ")",
                   "succeeds", BOOL,
                   "else", { disposition_call }, "raise", ERROR_NAME, ";" ;
disposition_call = CALLEE, "(", [ arg_list ], ")" ;

scalar_binding   = "let", NAME, "=", CALLEE, "(", [ arg_list ], ")",
                   [ "succeeds", declared_scalar ], ";" ;
declared_scalar  = [ "-" ], NAT | [ "-" ], FLOAT | BOOL ;

value_statement  = "let", NAME, "=", value_expr, ";" ;
value_expr       = value_operand, ( "+" | "-" | "*" | "/" | "%" ), value_operand
                 | value_operand, ( "==" | "!=" | "<" | "<=" | ">" | ">=" ), value_operand
                 | "-", NAME
                 | value_operand, "as", scalar_abi_type ;
value_operand    = NAME | [ "-" ], NAT | [ "-" ], FLOAT | BOOL ;

float_literal    = "let", NAME, "=", [ "-" ], FLOAT, ";" ;          (* binds f64 *)
scalar_literal   = "let", NAME, "=", [ "-" ], NAT, ";" ;            (* binds i32 *)
plain_call       = CALLEE, "(", [ arg_list ], ")", ";" ;

arg_list         = arg, { ",", arg } ;
arg              = [ "-" ], NAT
                 | [ "-" ], FLOAT
                 | BOOL
                 | NAME
                 | "move", NAME
                 | "out" ;
BOOL             = "true" | "false" ;
```

`out` is a reserved positional marker denoting the callee's declared output slot, never a local variable binding. `succeeds` reuses the single outcome keyword across acquisitions (`succeeds true` / `succeeds false`, discriminated by `else … raise ERROR;`) and scalar bindings (`succeeds INTEGER` / `succeeds FLOAT` / `succeeds BOOL`, terminated directly by `;`), adding zero new reserved words beyond `out`.

### Semantic Side Conditions and Refusal Boundaries

1. **External contract binding (never inline).**
   - Calling an `extern fn` requires `#import std.extern;` (or compatibility `import std.extern;`); calling without the import refuses (`Q015` / `Q022`).
   - A callee's contract is never written inline on the `.qr` declaration: an inline `contract { … }` block on a called `extern fn` is refused (`Q027`). Instead, contracts are supplied through `CompileOptions::extern_contracts` and loaded total-or-reject via [G05](#g05-resource-loader-v1).
   - Every executable or library module's `extern fn` declarations are type-checked even when uncalled (`W48-INERT-EXTERN-DECLARATION-TYPES`): parameters must be §G02t scalars or the three C04.1 pointers above, and results those or `unit` (otherwise refused at `Q020`).
   - For every called `extern fn`, parameter count, parameter names, parameter ABI types, and result ABI type must match the selected contract declaration exactly (`Q027`), and the callee must be present in the selected contract environment (`Q023`).

2. **Admitted vs. refused C04 custody shapes (`Q024`).**
   The bridge (`qcore_contract::extern_bridge`) admits an exact closed subset of [G05](#g05-resource-loader-v1) parameter and result modes and refuses every other shape by name (`Q024`):

   | Position / Mode | Admitted Signature & Combination Rules | Refused Shapes (`Q024`) |
   | --- | --- | --- |
   | **Scalar data** | `resource_borrowed(p)` on any `scalar_abi_type`; `resource_return_none()` on `unit` or `scalar_abi_type` | Any custody mode on a scalar; `fnptr` or `va_list`; non-heap `resource_kind` (including `descriptor`) |
   | **Borrowed pointer** | `resource_borrowed(p)` on `ptr<mut, u8>` or `ptr<const, u8>` | `resource_borrowed` on `ptr<mut, ptr<mut, u8>>` or `ptr<mut, ptr<const, u8>>` |
   | **Consumed / Released** | `resource_consumed(p, nonnull)` or `resource_released(p, nonnull)` on `ptr<mut, u8>` | `nullable` consumed or released parameters; consumption or release on `ptr<const, u8>` or double pointers |
   | **Owned return** | `resource_return_owned(nullable)` returning `ptr<mut, u8>` | `resource_return_owned(nonnull)`; owned return on `ptr<const, u8>`, double pointers, or scalars; derived/borrowed/removed returns |
   | **Out-slot acquisition** | `resource_out(p, nullable)` on `ptr<mut, ptr<mut, u8>>` with `resource_return_none()` | `resource_out(p, nonnull)`; out slot on any other pointer type; two out slots; out slot beside an owned or reallocated return |
   | **Indexed store** | `resource_stored(p, owner, nonnull)` on `ptr<mut, u8>` beneath a distinct `resource_owned(owner, nonnull)` on `ptr<mut, u8>` | `nullable` stored or owner parameter; `const` pointer on stored or owner; self-ownership; `resource_owned` parameter that neither owns a store nor is reallocated; store beside an acquisition; `resource_removed` |
   | **Reallocation** | `resource_return_reallocated(p, nullable)` returning `ptr<mut, u8>` over `resource_owned(p, nonnull)` on `ptr<mut, u8>` | `resource_return_reallocated(p, nonnull)`; subject out of range, not `Owned`, `nullable`, or not `ptr<mut, u8>`; reallocation beside an out slot or another departing parameter |

   *Why `resource_removed` and `nullable` release/consume are refused:* `EXTERN-F1` records stored child custody beneath `owner` as a certificate premise rather than a modeled owner-to-child map, so admitting `resource_removed` would mint a live caller identity out of an unmodeled premise. `resource_released(p, nullable)` and `resource_consumed(p, nullable)` are refused because a null value discharges nothing (§C04.2), and this profile models live non-null identities rather than nullable variables.

3. **Argument bridging and certificate separation of `move NAME`.**
   - `arg` forms map strictly by the parameter's declared contract mode (`Q024` on mismatch):
     - Literal numbers / booleans or bound value `NAME` → `Data` (scalar parameter).
     - Bare `NAME` → `Borrowed` pointer or `Owned` store-owner (resource liveness checked; binding remains live).
     - `move NAME` → `Consumed`, `Stored`, `Released`, or `ReallocSubject` (which of the four departing modes ran is determined solely by the contract declaration).
     - `out` → `OutSlot` (no caller binding read; the `let` binder receives the acquired identity on the success arm).
   - Even though `move NAME` spells all four departing modes in `.qr` syntax, the certificate distinguishes them: `resource_released` is published under `releases`, `resource_consumed` and `resource_stored` as transfers/premises, and `resource_return_reallocated` as a `(replaced, established)` pair, preserving §C04.2's distinction between obligation relocation and safety-critical destruction.

4. **Acquisitions, reallocation liveness, and `EXTERN-ERR-REL` error-arm cleanup (`Q025`).**
   - Every call that establishes fresh custody — `resource_return_owned(nullable)`, `resource_out(p, nullable)`, or `resource_return_reallocated(p, nullable)` — **must** use `acquisition_stmt` with `succeeds BOOL else { disposition_call } raise ERROR_NAME;` (`Q025`). Calling an acquiring function via `scalar_binding` or `plain_call` refuses (`Q025`), and using `acquisition_stmt` on a non-acquiring function refuses (`Q025`).
   - No departing `move` argument may accompany an acquiring call *except* the reallocation subject of `resource_return_reallocated`: under §C04.3, null failure preserves the incoming subject while non-null success replaces it once. Consequently, the reallocation subject stays live on the failure arm and **must** be released in the `else` handler alongside earlier live identities, whereas on the success arm it is replaced by the newly bound identity.
   - **Exact cleanup rendering (`EXTERN-ERR-REL`):** Between `else` and `raise ERROR_NAME;`, the `disposition_call` sequence must match the established sequential cleanup edge (`qcore_core::cleanup_lowering`) **exactly** — releasing every identity live immediately before the failing call (including a reallocation subject), each **once**, in **reverse acquisition order**, using declared `resource_consumed` or `resource_released` callees (`Q025`). A disposition call that acquires, stores, discharges zero or multiple identities, names a non-live identity, runs out of reverse acquisition order, or derives effects outside the entry's declared row is refused (`Q025`). Because the error arm is an exit path, resolving disposition arguments does **not** remove bindings from the continuing success path.
   - **Runtime witness check (exit 70):** `succeeds BOOL` is a literal fixture outcome control, never trusted at runtime. Emitted C initializes out-slot pointers to `NULL`, invokes the C callee, compares actual nullness against `succeeds BOOL`, and immediately calls `exit(70)` on disagreement before executing either branch.

5. **Typed scalar namespace, value statements, and declared scalar outcomes.**
   - **Disjoint namespaces:** Resource identities (`bindings`) and scalar/float values (`values`) occupy separate maps. Rebinding a live resource name as a scalar or vice versa refuses (`Q012` at name resolution, backed by `Q026` in lowering). Passing a resource binding at a scalar parameter or a scalar binding at a pointer parameter refuses (`Q024`). Unused scalar bindings are data and never constitute resource leaks.
   - **Scalar result bindings (`PIPE-SCALAR-RESULT`, `W48-QR-SCALAR-TYPES-R3`, `W48-QR-FLOATING-POINT`):** `let x = callee(args) [succeeds OUTCOME];` requires a non-acquiring callee whose result is a custody-free `scalar_abi_type`. `unit` and all pointer results (even `ptr<…, u8>` with `resource_return_none()`) refuse (`Q024`). If `succeeds OUTCOME` is present, `OUTCOME` is range-checked (or converted bit-for-bit at `f32`/`f64`) against the declared return type (`Q024`), and emitted C checks the actual C return value against `OUTCOME` (bitwise for floats), exiting `70` **before** writing the call trace line:
     - Without `succeeds`: `value CALLEE vN ARG…`
     - With `succeeds OUTCOME`: `value CALLEE vN ARG… -> OUTCOME`
   - **Literals and `value_statement`:** `let n = 42;` binds `i32`; `let x = 1.5;` binds `f64`. `value_statement` evaluates one §G02t binary arithmetic/comparison, unary `-NAME`, or `as` conversion. Operations on check-time-known values (literals, `succeeds`-declared call results, and folded operations over them) fold at check time and refuse on overflow/division-by-zero/out-of-range (`Q024`). Operations reading run-time-only call results (bound without `succeeds`) that can fault require `throws<ArithmeticError>` in the entry's `effects` row (`Q024` / `Q029`) and emit `q_scalar_try_*` helpers exiting `80 + tag` on fault. Float comparisons require `partial_eq` (`==`, `!=`) or `partial_ord` (`<`, `<=`, `>`, `>=`) in `effects`. `isize`/`usize` computation requires `--recorded-target`.

6. **Derived effect row and scope bounds.**
   - Entry functions take zero parameters, return `unit`, contain a flat statement sequence (no nested `if`, `for`, `match`, or `thread scope` blocks, `Q026`), and end with `return;`.
   - Every extern call derives `{io}`, plus `alloc` if it acquires custody, plus `mut` if any parameter is `ptr<mut, …>`. The entry's declared `effects { … }` row must cover every called function on both the normal path and all `else` cleanup arms (`Q025` / `Q029`); an omitted `effects` row is empty and admits no extern calls.
   - Pointer ABI types carry no byte length (`ExternAssumption::NoExtentTransport`); extent safety of the C callee remains part of the declared contract assumption.

### Concrete Example

```qr
module executable;
#import std.extern;

error AllocFailed;

extern fn buf_new(cap: usize) -> ptr<mut, u8>;
extern fn buf_grow(buf: ptr<mut, u8>, cap: usize) -> ptr<mut, u8>;
extern fn buf_checksum(buf: ptr<const, u8>, seed: u32) -> u32;
extern fn buf_free(buf: ptr<mut, u8>) -> unit;

fn main() -> unit effects { io, alloc, mut } {
    let b0 = buf_new(64) succeeds true else raise AllocFailed;
    // Reallocation keeps b0 on the failure cleanup arm and replaces it on success:
    let b1 = buf_grow(move b0, 128) succeeds true else buf_free(move b0) raise AllocFailed;
    let sum = buf_checksum(b1, 7) succeeds 42;
    let next_seed = sum + 1;
    let _ = buf_checksum(b1, next_seed);
    buf_free(move b1);
    return;
}
```

**Implementation & Provenance:** `crates/qcore-frontend/src/source_extern.rs`, `crates/qcore-core/src/extern_call.rs`, `crates/qcore-contract/src/extern_bridge.rs`; language spec [§L14](QCORE_SPEC_r0_3.md) & contract spec [§C04.1–C04.3](QCORE_CONTRACT_SPEC_r0_3.md#c041-abi-projection); cards `EXTERN-CALL-V1`, `EXTERN-F1`, `EXTERN-ERR-REL`, `EXTERN-MODES`, `PIPE-SCALAR-RESULT`, `EXTERN-SCALAR-LITERAL`, `EXTERN-SCALAR-OUTCOME`, `W48-QR-SCALAR-TYPES-R3`, `W48-QR-FLOATING-POINT`; reports [extern-call-v1](reports/archive/language-and-tooling/extern-call-v1/README.md), [extern-call-f1](reports/archive/language-and-tooling/extern-call-f1/README.md), [remaining-extern-err-rel](reports/archive/early-waves/remaining-extern-err-rel/README.md), [remaining-extern-modes](reports/archive/early-waves/remaining-extern-modes/README.md).

---

<a id="g02i-composed-extern-calls-and-rendered-lines"></a>
## G02i-extern-io. Composed extern calls and rendered lines (`source.std-io-extern.v1`)

### Consumer and Route Selection

`source.std-io-extern.v1` composes two sequential models — bounded contracted extern calls ([G02d](#g02d-bounded-extern-calls)) and standard-IO line rendering (`source.std-io.v1`, §G02g) — without introducing any new statement syntax. Route selection triggers when a module satisfies the conjunction of both selectors (it calls a declared `extern fn` **and** writes at least one `print`, `log`, `log_level`, or `flush` statement), evaluated **after** `source.ffi.v1` and all concurrent routes and **before** `source.extern-call.v1` and `source.std-io.v1` individually. A module using only one feature set stays on its single-feature route byte-for-byte; a module mixing either with task spawns or locks stays on the concurrent route and refuses there.

### Consolidated Grammar

```ebnf
composed_module = [ module_header ],
                  { composed_import | library_binding },
                  { error_decl | extern_decl | unsafe_extern_decl | composed_entry } ;

composed_import = ( "#import" | "import" ),
                  ( "std.extern" | "std.extern.unsafe"
                  | "std.fmt" | "std.io" | "std.logger" ), ";" ;

composed_entry  = "fn", NAME, "(", ")", "->", "unit",
                  [ "effects", "{", effect, { ",", effect }, "}" ],
                  "{", { composed_stmt }, "}" ;

composed_stmt   = extern_stmt                       (* §G02d, lowered by source_extern *)
                | io_stmt ;                         (* §G02g, lowered by source_std_io *)

io_stmt         = "print", rendering, io_operand, ";"
                | "log", log_level, rendering, io_operand, ";"
                | "log_level", log_level, ";"
                | "flush", ";" ;

rendering       = "dec" | "hex" ;
log_level       = "trace" | "debug" | "info" | "warn" | "error" | "fatal" ;
io_operand      = [ "-" ], NAT
                | [ "-" ], FLOAT
                | BOOL
                | NAME                              (* resolves against G02d value namespace *)
                | "borrow", NAME ;                  (* resolves against G02d custody namespace *)
```

### Semantic Side Conditions and Refusal Boundaries

1. **Disjoint statement dispatch (skip-set lowering).**
   - Every statement in `composed_entry` is dispatched by its leading keyword alone: `print`, `log`, `log_level`, and `flush` go to `source_std_io` (with extern spans skipped); `return;` is shared as the mandatory single terminal return; every other statement goes to `source_extern` (with IO spans skipped). Neither lowering sees statements outside its vocabulary, so neither refusal surface shifts.
   - Bundled capability imports remain per-statement (`Q015`): `std.extern` for extern calls, `std.io` for `print`, `std.logger` for `log` / `log_level` / `flush`, and `std.fmt` whenever `dec` or `hex` is named.

2. **Scalar rendering operands and check-time outcome determinism (`Q043`).**
   - A bare `NAME` in `io_operand` resolves exclusively against the extern lowering's **value** namespace (never its custody namespace).
   - The operand must be bound by an earlier step to a §G02t scalar or float (`W48-QR-SCALAR-TYPES-R3`, `W48-QR-FLOATING-POINT`), and its value **must be check-time-known** — originating from a literal binding (`let n = 42;`, `let x = 1.5;`), a `scalar_binding` with an explicit `succeeds OUTCOME` tail, or a `value_statement` folded over known values.
   - Rendering a bound extern call result that has **no** `succeeds` clause is refused (`Q043`, "no model computed"), because the reference interpreter cannot predict an unmodeled C return value.
   - Rendering an `isize` or `usize` binding without `--recorded-target` refuses (`Q043`); `bool` admits `dec` (`true`/`false`) and refuses `hex` (`Q030` / `Q043`).

3. **Borrowed resource identity rendering (`borrow NAME`, card `STD-FMT-BORROW`, `Q043`).**
   - `borrow NAME` resolves exclusively against the extern model's **custody** namespace (`ever_bound`) and renders the resource's deterministic model identity number (`rN`) — never its runtime pointer address, pointee bytes, or extent.
   - **Liveness gate:** `borrow NAME` is admitted if and only if the extern checker's published custody ledger holds that identity **live** at that exact composed step. An unknown name, an identity from a `succeeds false` failed acquisition, or an identity already departed via `move NAME` (consumed, released, stored, or reallocated) before the rendering statement is refused (`Q043`).
   - **Zero custody mutation & non-publication:** Rendering `borrow NAME` opens no obligation, discharges none, and leaves the custody ledger identical before and after the line. Writing the model number to `stdout`/`stderr` is not a §L21 constraint-10 pointer publication because streams are write-only byte sinks outside the custody graph.
   - Note that pipeline desugaring (`x |> print dec;`) produces `print dec x;` without `borrow` and rejects extra sink words (`Q036`), so `borrow NAME` must be written directly.

4. **Effect union and per-stream observation.**
   - The composed entry's derived effect row is the **set union** of the extern model's derived row and the `std.io` model's `{io}` row, checked against the entry's single declared `effects { … }` clause (`Q029`).
   - Observations are compared per stream: `stdout` interleaves `print` lines and extern call-trace lines (`acquire`, `value`, `call`, …) in exact sequential program order; `stderr` carries `log` lines. Relative ordering between `stdout` and `stderr` is unmodeled.

### Concrete Example

```qr
module executable;
#import std.extern;
#import std.fmt;
#import std.io;
#import std.logger;

error OpenFailed;

extern fn session_open(port: u16) -> ptr<mut, u8>;
extern fn session_code(s: ptr<const, u8>) -> i32;
extern fn session_close(s: ptr<mut, u8>) -> unit;

fn main() -> unit effects { io, alloc, mut } {
    let sess = session_open(8080) succeeds true else raise OpenFailed;
    print dec borrow sess;                  // renders live model identity (1)
    let status = session_code(sess) succeeds 200;
    let next = status + 4;
    log info dec next;                      // stderr: [INFO] 204
    session_close(move sess);
    // `print dec borrow sess;` here would refuse with Q043 (identity departed)
    flush;
    return;
}
```

**Implementation & Provenance:** `crates/qcore-frontend/src/source_composed.rs`, `crates/qcore-frontend/src/source_extern.rs` (`lower_source_extern_skipping`), `crates/qcore-frontend/src/source_std_io.rs`; cards `STD-IO-COMPOSE`, `STD-FMT-BORROW`, `W48-QR-SCALAR-TYPES-R3`, `W48-QR-FLOATING-POINT`.

---

<a id="g02d1-unchecked-linkage"></a>
### G02d.1. Unchecked scalar linkage (`unsafe extern fn`)

#### Consumer and Selection Mechanism

Card `UNSAFE-LINKAGE` widens the declaration surface of [G02d](#g02d-bounded-extern-calls) and [G02i-extern-io](#g02i-composed-extern-calls-and-rendered-lines) so a `.qr` module can link and call a scalar-only external C symbol that has **no `.qv` contract**, under an explicit, file-local waiver import and a recorded certificate premise. It adds no new statement form, argument syntax, or reserved word (`unsafe` was already reserved). Without the waiver, calling an uncontracted symbol continues to refuse at `Q023`.

#### Consolidated Grammar

```ebnf
unsafe_extern_module = [ module_header ],
                       ( "#import" | "import" ), "std.extern", ";",
                       ( "#import" | "import" ), "std.extern.unsafe", ";",
                       { other_import | library_binding },
                       { error_decl | extern_decl | unsafe_extern_decl | entry_fn } ;

unsafe_extern_decl   = "unsafe", "extern", "fn", NAME,
                       "(", [ unsafe_param, { ",", unsafe_param } ], ")",
                       "->", unsafe_result_type, ";" ;

unsafe_param         = NAME, ":", unsafe_param_type ;
unsafe_param_type    = "bool"
                     | "i8" | "i16" | "i32" | "i64" | "isize"
                     | "u8" | "u16" | "u32" | "u64" | "usize"
                     | "f32" | "f64" ;
unsafe_result_type   = "unit" | unsafe_param_type ;
```

#### Semantic Side Conditions and Refusal Boundaries

- **Dual orthogonal imports (`Q015`, `Q032`):** Neither `std.extern` nor `std.extern.unsafe` implies the other. `std.extern` grants the extern call vocabulary without waiving contracts; `std.extern.unsafe` grants the `unsafe extern fn` declaration waiver without granting calls. Declaring an `unsafe extern fn` — even if never called in the body — requires `std.extern.unsafe` in the declaring file (`Q032`), because declaring an uncontracted symbol is itself the trust act.
- **Strictly pointer-free ABI (`Q033`):** Every parameter must be in `unsafe_param_type` (which excludes `unit`) and the result in `unsafe_result_type` (`f32`/`f64` included since `W48-QR-FLOATING-POINT`). **Every pointer type — including `ptr<const, u8>` — is refused in every position (`Q033`)**, as is any non-ABI type. Because no pointer crosses the boundary, an unchecked call cannot alter the caller's custody ledger regardless of callee behavior. Note that `unsafe extern fn` is completely refused on `source.ffi.v1` (`Q034`, [G02u](#g02u-imported-c-structs-sourceffiv1)).
- **No shadowing of checkable contracts (`Q034`):** An `unsafe extern fn` may not carry an inline `contract { … }` block (`Q034`), and may not name a symbol that is also declared in the selected `.qv` contract environment (`Q034`) — if a contract exists in the selection, the contracted route governs regardless of signature agreement.
- **Maximal derived effect row (`{io, alloc, mut}`):** Because nothing is proved about an uncontracted callee's side effects, calling one unconditionally derives `{io, alloc, mut}` (plus float comparison effects if used in the body), which the entry's `effects` row must declare (`Q029`). Modules may freely mix `extern fn` (contracted) and `unsafe extern fn` (waived) declarations; a module whose callees are all `unsafe extern fn` compiles under `quirin check` and `quirin emit` with no `--extern-contracts` selection at all.
- **Mandatory certificate premise (`Q035`):** Every published certificate must record each `unsafe extern fn` callee alongside its declared scalar ABI signature as an `unchecked_linkage` premise; a certificate omitting one fails recheck (`Q035`).

#### Concrete Example

```qr
module executable;
#import std.extern;
#import std.extern.unsafe;

unsafe extern fn c_crc32_step(acc: u32, byte: u8) -> u32;

fn main() -> unit effects { io, alloc, mut } {
    let h = c_crc32_step(0, 65) succeeds 2212294583;
    let _ = c_crc32_step(h, 66);
    return;
}
```

**Implementation & Provenance:** `crates/qcore-frontend/src/source_extern.rs`, `crates/qcore-core/src/extern_call.rs`; design [`docs/design/UNSAFE_LINKAGE_V1.md`](design/UNSAFE_LINKAGE_V1.md); card `UNSAFE-LINKAGE`, `W48-QR-FLOATING-POINT`; report [wave18-unsafe-linkage](reports/archive/early-waves/wave18-unsafe-linkage/README.md).

---

<a id="g02d2-the-native-library-manifest"></a>
### G02d.2. The native-library manifest (`.qrlib` v1)

#### Consumer and Selection Mechanism

A `.qrlib` v1 manifest (`source.native-library.v1`) is a standalone declarative artifact-pinning file selected by the builder (`quirin --native-library MANIFEST`, `CompileOptions::native_libraries`). It adds no `.qr` source syntax or reserved words. Instead, it upgrades call-site certificate premises:
- For an `unsafe extern fn` ([G02d.1](#g02d1-unchecked-linkage)), the premise strengthens from *"symbol X, unchecked"* to *"symbol X, unchecked, from artifact Y@sha256 under ABI premises Z"*.
- For a contracted `extern fn` ([G02d](#g02d-bounded-extern-calls)), a **stacked** artifact premise (*"contracted X per contract C, from artifact Y@sha256 under ABI premises Z"*) is recorded **beside** the declared contract premise — never replacing it (one states what the code does; the other states which binary build supplies it).

Selecting no manifest is the default on [G02d](#g02d-bounded-extern-calls) and [G02d.1](#g02d1-unchecked-linkage), leaving existing programs byte-identical.

#### Consolidated Grammar

```ebnf
manifest_v1      = "native_library", "manifest", "v1", ";",
                   { field_v1 | include_stmt | entry_v1 } ;

field_v1         = ( "library" | "version" | "artifact" ), TOKEN, ";"
                 | "hash", sha256_digest, ";"
                 | "data_model", "lp64", ";"
                 | "unwinding", "abort", ";"
                 | "allocator_custody", "closed", ";"
                 | "thread_posture", "single_caller", ";"
                 | "header", TOKEN, ";"
                 | "header_hash", sha256_digest, ";" ;

include_stmt     = "include", include_spelling, "from", TOKEN, sha256_digest, ";" ;
include_spelling = "<", TOKEN, ">" | '"', TOKEN, '"' ;

entry_v1         = waived_entry | contract_entry ;

waived_entry     = "unchecked", "fn", NAME,
                   "(", [ c_abi_type, { ",", c_abi_type } ], ")",
                   "->", c_abi_type, ";" ;

contract_entry   = "contract", "fn", NAME,
                   "(", [ c_abi_type, { ",", c_abi_type } ], ")",
                   "->", c_abi_type, "from", MODULE_REF, ";" ;

c_abi_type       = "_Bool"
                 | "int8_t" | "int16_t" | "int32_t" | "int64_t" | "intptr_t"
                 | "uint8_t" | "uint16_t" | "uint32_t" | "uint64_t" | "uintptr_t"
                 | "float" | "double"
                 | "void"
                 | c_pointer_type ;

c_pointer_type   = "uint8_t", [ " " ], "*"
                 | "const", "uint8_t", [ " " ], "*"
                 | "uint8_t", [ " " ], "**" ;

sha256_digest    = "sha256:", 64 * LOWER_HEX ;
MODULE_REF       = TOKEN ;   (* exact contract selection key, never normalized *)
```

`#` introduces a line comment to end-of-line; statements are `;`-terminated and may span lines; `TOKEN` is a non-empty whitespace-free, `;`-free, `#`-free token.

#### Semantic Side Conditions and Refusal Boundaries

1. **Total-or-reject field discipline (`Q039`).**
   - `native_library manifest v1;` must be the first statement, and at least one symbol entry must appear (`Q039`).
   - The 8 core fields (`library`, `version`, `artifact`, `hash`, `data_model`, `unwinding`, `allocator_custody`, `thread_posture`) are **mandatory** and each may appear at most once (`Q039`). `v1` admits `data_model lp64;` only (`llp64` and `struct TAG` types are `v2`-only, `Q039`).
   - The 2 header-binding fields (`header`, `header_hash`) are **optional as a pair**: both must be present or neither; writing one without the other rejects the manifest as a missing-field error (`Q039`).

2. **Entry separation, C ABI projections, and bidirectional slice totality.**
   - Entry types use C `<stdint.h>` / `<stdbool.h>` / `float` / `double` spellings rather than Quirin spellings (`float` and `double` joined both v1 and v2 in `W48-QR-FLOAT-QRLIB-SPELLING`). `void` is valid only as a return type (empty `()` denotes zero parameters).
   - `unchecked fn` and `contract fn` are distinguished strictly by keyword: `unchecked fn` rejects `c_pointer_type` in every position and rejects any `from MODULE` tail (`Q039`); `contract fn` requires `from MODULE_REF` and admits pointers (`Q039`). No symbol may be bound twice (`Q039`).
   - **Bidirectional contract slice totality (`Q047`):** Every `contract fn … from MODULE_REF;` entry must resolve to a declared function in the selected contract module `MODULE_REF` (compared byte-for-byte without path normalization), **and** every declaration in any contract module named by the manifest must be bound by a `contract fn` entry in the manifest (`Q047`).
   - **Cross-checks and diagnostics:**
     - `Q040`: selected binding is not a function.
     - `Q041`: `unsafe extern fn` signature in `.qr` contradicts the manifest's `unchecked fn` projection.
     - `Q042`: symbol declared `unchecked fn` in the manifest is also present in the selected `.qv` contract environment.
     - `Q048`: contracted `extern fn` contradicts the manifest's `contract fn` projection — the contract is **withdrawn** for the unit and calls fall to `Q023`.
     - `Q049`: published certificate omits either of the two stacked premises for a manifest-bound contracted call.

3. **Exported C header verification and decoration derivation (`Q063`, `Q064`).**
   - When `header` and `header_hash` are present (card `NATIVE-LIBRARY-HEADER`), the driver reads the referenced `.h` file (resolved relative to the manifest's directory via `CompileOptions::native_library_headers`), verifies its SHA-256 against `header_hash` (`Q063` on mismatch or missing text), parses the header through the bounded preprocessor-free reader (`qcore_core::native_library_header`, admitting comments, line splices, a whole-file `#ifndef`/`#define`/`#endif` include guard, the standard `<stdint.h>`/`<stddef.h>` LP64 prelude, and depth-zero `typedef` and function declarations), and verifies every manifest entry's C signature against the header's own declaration (`Q063` if outside the bounded C reader subset; `Q064` if the header contradicts the manifest entry). Extra declarations in the header not named by the manifest are ignored.
   - **Included decoration headers (`NATIVE-LIBRARY-HEADER-DECORATION`):** Repeatable `include SPELLING from REFERENCE sha256:…;` statements require `header` to be present and bind exact `#include` operands (`<event2/visibility.h>` or `"local.h"`, delimiters included) to supplied file references and digests. The statement never asserts that a macro is erasable; instead, the reader **derives** from the included text's own `#define`s that an object-like export-decoration macro expands on every conditional branch exclusively to the reader's published vocabulary of type-neutral visibility/calling-convention decorations and is never `#undef`ed. Non-derivable macros, undeclared `#include`s, unreached `include` bindings, or digest mismatches refuse (`Q063`).

#### Concrete Example (`.qrlib` v1)

```text
# Hand-authored v1 native library manifest with checked header + visibility include
native_library manifest v1;
library demo_codec;
version 1.2.0;
artifact libdemo_codec.a;
hash sha256:9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08;
data_model lp64;
unwinding abort;
allocator_custody closed;
thread_posture single_caller;
header include/demo_codec.h;
header_hash sha256:4b227777d4dd1fc61c6f884f48641d02b4d121d3fd328cb08b5531fcacdabf8a;
include "demo_export.h" from include/demo_export.h sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855;

unchecked fn demo_version_major() -> uint32_t;
contract fn demo_buf_new(uintptr_t) -> uint8_t * from contracts/demo_codec.qv;
contract fn demo_buf_free(uint8_t *) -> void from contracts/demo_codec.qv;
```

**Implementation & Provenance:** `crates/qcore-core/src/native_library.rs`, `crates/qcore-core/src/native_library_header.rs`; design [`docs/design/NATIVE_LIBRARY_LINKAGE_V1.md`](design/NATIVE_LIBRARY_LINKAGE_V1.md), [`docs/design/NATIVE_LIBRARY_HEADER_V1.md`](design/NATIVE_LIBRARY_HEADER_V1.md), spec [§L14.4](QCORE_SPEC_r0_3.md); cards `UNSAFE-LINKAGE-ARTIFACT`, `NATIVE-LIBRARY-CONTRACT-SLICE`, `NATIVE-LIBRARY-HEADER`, `NATIVE-LIBRARY-HEADER-DECORATION`, `W48-QR-FLOAT-QRLIB-SPELLING`; reports [wave19](reports/archive/early-waves/wave19-unsafe-linkage-artifact/README.md), [wave20](reports/archive/waves-20-39/wave20-native-library-contract-slice/README.md), [wave22](reports/archive/waves-20-39/wave22-native-library-header/README.md), [decoration](reports/archive/language-and-tooling/native-library-header-decoration/README.md).

---

<a id="g02d21-the-generated-manifest-v2"></a>
#### G02d.2.1. The symbol-generated manifest (`.qrlib` v2)

##### Consumer and Generation Discipline

`.qrlib` v2 (`native_library manifest v2;`, card `W48-QRLIB-LINKAGE`) is the **mandatory** linkage manifest format for `source.ffi.v1` ([G02u](#g02u-imported-c-structs-sourceffiv1)) and may also bind contracted callees on other routes. Unlike v1, a v2 manifest is **generated on every build before linking and never written by hand**:
- `quirin qrlib generate` (`qcore_c_gateway::linkage::generate`) inspects the binary artifact's own symbol tables (via Quirin's internal ELF, Mach-O/TBD, and COFF/PE readers — never external `nm`), follows GNU `ld` linker scripts, hashes all artifacts and headers, and emits canonical v2 text.
- `quirin qrlib check` and `quirin build` regenerate the v2 manifest from the build inputs and refuse any stored `.qrlib` file that differs by even a single byte as **stale**.
- Immediately prior to invoking the system linker, `quirin build` (`LinkPlan`) re-verifies every pinned SHA-256 digest and passes the exact pinned artifact paths to the linker.

##### Consolidated Grammar

```ebnf
manifest_v2   = "native_library", "manifest", "v2", ";",
                { field_v2 | entry_v2 } ;

field_v2      = ( "library" | "version" | "artifact" | "target" ), TOKEN, ";"
              | "hash", sha256_digest, ";"
              | "linkage", ( "static" | "dynamic" ), ";"
              | "format", ( "elf" | "macho" | "coff" ), ";"
              | "input", TOKEN, sha256_digest, [ "as_needed" ], ";"   (* repeatable *)
              | "runtime", runtime_spec, ";"                          (* repeatable *)
              | "data_model", ( "lp64" | "llp64" ), ";"
              | "unwinding", "abort", ";"
              | "allocator_custody", "closed", ";"
              | "thread_posture", "single_caller", ";"
              | "header", TOKEN, ";"
              | "header_hash", sha256_digest, ";" ;

runtime_spec  = "soname", TOKEN, "in", TOKEN
              | "install_name", TOKEN, [ "current_version", TOKEN ]
              | "dll", TOKEN, "os_build", TOKEN ;

entry_v2      = "contract", "fn", NAME,
                "(", [ c_type_v2, { ",", c_type_v2 } ], ")",
                "->", c_type_v2,
                "from", MODULE_REF,
                "export", export_clause, ";" ;

export_clause = TOKEN,                                (* object-level symbol, e.g. _sqrt *)
                [ "version", TOKEN ],                 (* default ELF symbol version *)
                ( "global" | "weak" ),
                [ "ifunc" ],                          (* STT_GNU_IFUNC *)
                [ "dll", TOKEN ],                     (* COFF import DLL name *)
                "in", TOKEN ;                         (* defining file or archive(member) *)

c_type_v2     = c_abi_type                            (* §G02d.2, float/double included *)
              | "struct", TAG
              | "struct", TAG, [ " " ], "*"
              | "const", "struct", TAG, [ " " ], "*" ;
```

##### Semantic Side Conditions and Refusal Boundaries

- **Derived vs. declared metadata:**
  - *Declared by build configuration:* `library`, `version`, `linkage`, the three ABI posture premises (`unwinding`, `allocator_custody`, `thread_posture` — never defaulted), the `header` path, the selected `.qv` contract modules (every declaration of each module becomes one `entry_v2`, making the slice total by construction), and on Windows the `os_build` string.
  - *Derived from artifacts and recorded target:* `hash` (SHA-256 of `artifact`), `input` lines in script order for every file referenced by an `ld` linker script (preserving `AS_NEEDED` as `as_needed`), `format` (`elf` / `macho` / `coff`), `target` triple, `data_model` (`llp64` on 64-bit COFF, `lp64` on 64-bit Unix), `header_hash`, `runtime` library identities, and every entry's `export` symbol resolution record.
- **Exported symbol-table validation (generation refusals):** Every bound symbol must be exported and defined in the pinned artifact. Generation refuses by name if a symbol is missing/stripped, undefined, local-only, multiply defined, hidden-version-only, omitted from an archive symbol index, imported by ordinal only, or (on Windows) absent from the runtime DLL's named export table or exported as a DLL forwarder. Defined weak symbols are admitted and recorded as `weak`.
- **Static vs. dynamic linkage invariants (`Q039`):**
  - `linkage static;` pins code archives (ELF/Mach-O/COFF `.a`/`.lib` or `ld` scripts of archives) and **forbids** `runtime` statements (`Q039`).
  - `linkage dynamic;` pins link-time stubs (ELF `.so` or scripts thereof, Mach-O `.dylib` export tries or `.tbd` text stubs — matching `arm64e-macos` when `arm64-macos` is unlisted — or COFF import libraries) and **requires at least one** `runtime` premise (`soname`, `install_name`, or `dll … os_build …`) naming the OS-loaded shared library (`Q039`).
  - Mixing archive and shared inputs in a script, mismatching artifact format vs. `linkage`, static linkage resolving to a COFF import, or dynamic COFF linkage resolving to raw code refuses.
- **Strict v1 / v2 separation (`Q039`):** Writing `unchecked fn` or `include … from …` inside a v2 manifest, omitting `linkage`/`format`/`target` or an entry's `export` clause in v2, or writing v2 statements or `struct TAG` types in a v1 manifest refuses (`Q039`).

##### Concrete Example (`.qrlib` v2)

```text
native_library manifest v2;
library qr_pkt;
version 0.4.0;
artifact libqr_pkt.a;
hash sha256:6b86b273ff34fce19d6b804eff5a3f5747ada4eaa22f1d49c01e52ddb7875b4b;
linkage static;
format elf;
target x86_64-unknown-linux-gnu;
data_model lp64;
unwinding abort;
allocator_custody closed;
thread_posture single_caller;
header qr_pkt.h;
header_hash sha256:d4735e3a265e16eee03f59718b9b5d03019c07d8b6c51f90da3a666eec13ab35;

contract fn qr_pkt_checksum(const struct qr_pkt *) -> uint32_t from contracts/qr_pkt.qv export qr_pkt_checksum global in libqr_pkt.a(qr_pkt.o);
```

**Implementation & Provenance:** `crates/qcore-core/src/native_library.rs`, `crates/qcore-c-gateway/src/linkage.rs`; spec [§L14.8](QCORE_SPEC_r0_3.md); cards `W48-QRLIB-LINKAGE`, `W48-QR-FLOAT-QRLIB-SPELLING`; report [w48-qrlib-linkage](reports/archive/wave-48/w48-qrlib-linkage/README.md).

---

<a id="g02d3-the-source-library-binding"></a>
### G02d.3. The source library binding (`extern library`)

#### Consumer and Selection Mechanism

Before card `NATIVE-LIBRARY-SOURCE-BINDING`, manifest resolution matched symbol names across the builder's entire `--native-library` selection without the `.qr` source file stating which library supplied its extern declarations. `extern library NAME;` is an import-adjacent top-level directive that binds a single `.qr` input file's extern surface to specific selected `.qrlib` manifests.

- **On `source.ffi.v1` ([G02u](#g02u-imported-c-structs-sourceffiv1)):** Mandatory whenever the file calls any C function, and must resolve to a generated **v2** manifest (`Q065`, `Q066`).
- **On all other routes ([G02d](#g02d-bounded-extern-calls), [G02d.1](#g02d1-unchecked-linkage), [G02i-extern-io](#g02i-composed-extern-calls-and-rendered-lines)):** Opt-in; a file writing zero `extern library` directives preserves selection-wide symbol resolution byte-for-byte (and never triggers manifest loading if none is referenced).

#### Consolidated Grammar

```ebnf
library_binding = "extern", "library", NAME, ";" ;
```

Both `extern` and `library` were existing source words (`extern fn`, `module library;`); the two-token sequence `extern library` at top-level declaration position reserves **zero** new keywords, and `library` remains a legal ordinary identifier everywhere else.

#### Semantic Side Conditions and Refusal Boundaries

- **Exact, unnormalized matching (`Q065`):** `NAME` is compared byte-for-byte against the `library` field of the builder's selected `.qrlib` manifests. An unbound `NAME` matching zero manifests, matching two selected manifests, repeated `extern library NAME;` lines in one file, or writing `extern library` when no `--native-library` was passed refuses (`Q065`).
- **No silent fallback to unbound manifests (`Q066`):** In a file that writes at least one `extern library` directive, declaring an `extern fn` whose symbol is bound by another selected manifest that *this file did not name* refuses (`Q066`) rather than silently dropping the build identity premise. Conversely, a symbol bound by *no* selected manifest at all keeps its ordinary premise (except on `source.ffi.v1`, where every called C symbol must be bound by a named v2 manifest, `Q066`).
- **Per-file scope & pre-route validation:** Resolution is strictly per `.qr` input file — another file's `extern library` binding in a multi-file compilation neither grants nor restricts this file's declarations. Every written `extern library` directive is validated **before** route selection, even in a file that declares zero `extern fn`s.
- **Stacked certificate premise (`Q067`):** Every published certificate for a module containing `extern library NAME;` must carry a `SourceLibraryBinding` premise (`library@sha256` plus its ABI posture fields) for every bound library (`Q067`), even if zero symbols from that library were called.

#### Concrete Example

```qr
module executable;
#import std.extern;
extern library demo_codec;

error AllocError;

extern fn demo_buf_new(size: usize) -> ptr<mut, u8>;
extern fn demo_buf_free(buf: ptr<mut, u8>) -> unit;

fn main() -> unit effects { io, alloc, mut } {
    let p = demo_buf_new(32) succeeds true else raise AllocError;
    demo_buf_free(move p);
    return;
}
```

**Implementation & Provenance:** `crates/qcore-core/src/native_library.rs` (`SourceLibraryBinding`), `crates/qcore-frontend/src/frontend.rs`; cards `NATIVE-LIBRARY-SOURCE-BINDING`, `W48-QRLIB-LINKAGE`; report [native-library-source-binding](reports/archive/language-and-tooling/native-library-source-binding/README.md).

---

<a id="g02d4-delayed-custody-results-sourcequant-variantv1"></a>
### G02d.4. Delayed custody results (`source.quant-variant.v1`)

#### Consumer and Invocation

`source.quant-variant.v1` allows a `module library;` importing `#import std.extern;` to adapt a signed 32-bit C return-code disposition ([§C04.13.2](QCORE_CONTRACT_SPEC_r0_3.md#c04132-packing-a-return-code-disposition)) into a generation-indexed, move-only `quant variant` package ([§L09.1](QCORE_SPEC_r0_3.md#l091-quant-variants)) that can be moved across private helper functions and opened by a delayed `match`:

```sh
quirin check --contracts compiler/tests/fixtures/quant_variant/take.qv compiler/tests/fixtures/quant_variant/delayed.qr
quirin emit  --contracts compiler/tests/fixtures/quant_variant/take.qv compiler/tests/fixtures/quant_variant/delayed.qr -o delayed.c
```

#### Consolidated EBNF Subset (over `QCORE_DESIGN_GRAMMAR_r0_3.bnf`)

```ebnf
qv_library_module = "module", "library", ";",
                    ( "#import" | "import" ), "std.extern", ";",
                    { quant_variant_decl | mapped_extern_decl | qv_fn_decl } ;

quant_variant_decl = "quant", "variant", VARIANT_NAME, "(", SUBJ_IDENT, ")", "{",
                       quant_arm, { quant_arm },
                     "}" ;
quant_arm          = TAG_NAME, [ "(", "code", ":", "i32", ")" ],
                     "carries", ( "none" | "custody", "(", SUBJ_IDENT, ")" ), ";" ;

mapped_extern_decl = "extern", "fn", NAME, "(", SUBJ_IDENT, ":", "ptr", "<", "mut", ",", "u8", ">", ")",
                     "->", "i32",
                     "contract", "{",
                       "requires", "resource_contract_v1", ";",
                       { c0413_on_return_clause },
                       "ensures", "normal", "resource_return_none", "(", ")", ";",
                       { "quant", "variant", "on_return", "(", rel_op, ",", INT, ")",
                         VARIANT_NAME, "::", TAG_NAME, ";" },
                     "}", ";" ;

qv_param           = NAME, ":", "own", "<", "Buffer", ">", "use", "one"
                   | NAME, ":", "ptr", "<", "mut", ",", "u8", ">"
                   | NAME, ":", "quant", "variant", VARIANT_NAME, "(", SUBJ_NAME, ")" ;
```

#### Semantic Side Conditions and C ABI Boundary

- **Declaration & contract agreement:** Each `quant variant` precedes its mapped `extern fn` (single `ptr<mut, u8>` formal, `-> i32` result, arms carrying `none` or `custody(p)` plus optional `code: i32`). Inline `quant variant on_return(...)` clauses and the `--contracts` selected `C04.13` `.qv` declaration must agree on ABI, comparison classes, and full non-null `resource_consumed` / `resource_owned` dispositions (no assumption is inferred from tag names; `protocol(...)` and `using #effect` arms are refused).
- **Acyclic functions, generation matching, and delayed `match`:**
  - All functions (checked even when uncalled) form an acyclic call graph returning `i32`. `own<Buffer>` formals accept distinct, non-null, `free`-compatible heap allocations; helpers taking `p: ptr<mut, u8>` alongside `v: quant variant V(p)` require `v` to match `p`'s exact actual generation at every call site (raw `p` alone grants zero release authority).
  - Admitted body statements: pointer alias `let q = p;`, adapter call `let v = take(q);`, package move `let w = move v;`, scalar `let`, helper calls (passing subjects by name and packages/owners with `move`), `drop` of a live owner, exhaustive consuming `match [move] v { V::Tag => { ... } V::Retained { code: rc } => { ... } }` (max nesting depth **32**, continuing arms agreeing on the surviving frame), and scalar `return`. Every exit path must discharge or transfer all custody and open or transfer every sealed package.
- **Exported C ABI (`qv_entry_NAME`):** Owner-only entry functions export as `qv_entry_NAME` (assuming 8-bit bytes and 32-bit signed C `int`); the adapter calls the underlying C symbol once, classifies the return code, and passes private helpers the package by value plus a reference to the subject's pointer slot so matching the empty arm never reads a freed pointer.

**Implementation & Provenance:** [`compiler/tests/fixtures/quant_variant/delayed.qr`](../compiler/tests/fixtures/quant_variant/delayed.qr), [`take.qv`](../compiler/tests/fixtures/quant_variant/take.qv), [`caller.c`](../compiler/tests/fixtures/quant_variant/caller.c), `compiler/tests/quant_variant.rs`, [`proof/QCore/QuantVariant.lean`](../proof/QCore/QuantVariant.lean).

---

<a id="g02u-imported-c-structs-sourceffiv1"></a>
## G02u. Imported C structs (`struct!`, `source.ffi.v1`)

### Consumer and Route Selection

`source.ffi.v1` enables `.qr` programs to import real C `struct TAG` types from C headers, construct and mutate their listed scalar/float fields, pass and return them across C calls and by-value `.qr` helper functions, and render known outcomes via `std.io` / `std.logger`.

**Highest-priority route selection:** `has_source_ffi` runs **before every other route** in the compiler. Any module that writes `#import std.ffi.unsafe;` **or** contains a `struct!` declaration is routed unconditionally to `source.ffi.v1`. This prevents an FFI file from ever reaching a route that could publish a plain `Safe` verdict without the **unsafe-FFI assumption**, and guarantees that unsupported constructs in an FFI file refuse directly at `Q026` rather than falling through. Under §L14.6 and §L20, every verdict emitted on this route is at best *"Safe under the unsafe-ffi assumption"* (naming every imported struct, every called `#c_ffi` function, and `linked:LIBRARY` premises) — never plain `Safe` (note: this is an explicit conditional `Safe` assumption, not the `Unsafe` verdict).

### Consolidated Grammar

#### `.qr` Source Grammar (`source.ffi.v1`)

```ebnf
ffi_module    = [ module_header ],
                "#import", "std.ffi.unsafe", ";",
                { ffi_import | library_binding },
                { struct_import | ffi_extern | ffi_entry | ffi_helper } ;

ffi_import    = ( "#import" | "import" ),
                ( "std.extern" | "std.fmt" | "std.io" | "std.logger" ), ";" ;
library_binding = "extern", "library", NAME, ";" ;                  (* required for C calls, §G02d.3 *)

struct_import = "struct", "!", "{",
                "type", NAME, "=", C_STRUCT_LIT, "from", STRING, ";",
                "fields", "{", ffi_field, { ",", ffi_field }, [ "," ], "}",
                "}" ;
C_STRUCT_LIT  = "c", STRING ;        (* adjacent tokens, no whitespace; STRING = "struct TAG" *)
ffi_field     = NAME, ":", ffi_scalar ;

ffi_scalar    = "bool"
              | "i8" | "i16" | "i32" | "i64" | "isize"
              | "u8" | "u16" | "u32" | "u64" | "usize"
              | "f32" | "f64" ;

ffi_extern    = "extern", "fn", NAME,
                "(", [ ffi_param, { ",", ffi_param } ], ")",
                "->", ffi_result, ";" ;
ffi_param     = NAME, ":",
                ( ffi_scalar | RECORD_NAME | "ptr", "<", "const", ",", RECORD_NAME, ">" ) ;
ffi_result    = "unit" | ffi_scalar | RECORD_NAME ;

ffi_entry     = "fn", NAME, "(", ")", "->", "unit",
                [ "effects", "{", ffi_effect, { ",", ffi_effect }, "}" ],
                "{", { ffi_stmt }, "}" ;

ffi_helper    = "fn", NAME,
                "(", [ helper_param, { ",", helper_param } ], ")",
                [ "->", ( "unit" | ffi_scalar | RECORD_NAME ) ],
                [ "effects", "{", ffi_effect, { ",", ffi_effect }, "}" ],
                "{", { ffi_stmt }, "}" ;
helper_param  = NAME, ":", ( ffi_scalar | RECORD_NAME ) ;           (* by value only *)
ffi_effect    = "io" | "alloc" | "mut" ;

ffi_stmt      = "let", NAME, "=", RECORD_NAME, "{", field_init, { ",", field_init }, [ "," ], "}", ";"
              | "let", NAME, "=", NAME, ".", NAME, ";"              (* listed field read *)
              | NAME, ".", NAME, "=", ffi_value, ";"                (* listed field write; derives mut *)
              | "let", NAME, "=", NAME, ";"                         (* by-value scalar or record copy *)
              | [ "let", NAME, "=" ], CALLEE, "(", [ ffi_value, { ",", ffi_value } ], ")",
                [ "succeeds", ffi_outcome ], ";"
              | io_line_stmt                                        (* print / log / log_level / flush, §G02g *)
              | "return", [ ffi_value ], ";" ;                      (* bare in entry; value in non-unit helper *)

field_init    = NAME, ":", ffi_value ;
ffi_value     = [ "-" ], INTEGER | [ "-" ], FLOAT | "true" | "false" | NAME ;
ffi_outcome   = [ "-" ], INTEGER
              | [ "-" ], FLOAT
              | "true" | "false"
              | RECORD_NAME, "{", literal_init, { ",", literal_init }, [ "," ], "}" ;
literal_init  = NAME, ":", ( [ "-" ], INTEGER | [ "-" ], FLOAT | "true" | "false" ) ;
```

#### Paired `.qv` Contract Attribute Grammar (§C04.12)

```ebnf
c_ffi_contract_decl = "[|", "quant", "#", "c_ffi", "|]",
                      "extern", "fn", IDENT,
                      "(", [ c_ffi_param, { ",", c_ffi_param } ], ")",
                      "->", c_ffi_result,
                      "contract", "{",
                          "requires", "resource_contract_v1", ";",
                          { "ensures", "normal", "resource_borrowed", "(", IDENT, ")", ";" },
                          "ensures", "normal", "resource_return_none", "(", ")", ";",
                      "}", ";" ;

c_ffi_param         = IDENT, ":", ( ffi_scalar | C_STRUCT_LIT | "ptr", "<", "const", ",", C_STRUCT_LIT, ">" ) ;
c_ffi_result        = "unit" | ffi_scalar | C_STRUCT_LIT ;
```

Lexically, `[|` and `|]` form single delimiter tokens only when `[` and `|` (or `|` and `]`) are immediately adjacent (leaving closure capture lists `|copy x|` untouched), and `c"struct TAG"` requires the identifier `c` to immediately abut the string literal with zero intervening bytes.

### Semantic Side Conditions and Refusal Boundaries

1. **File-scoped trust import (`Q032`).**
   - Every `.qr` file containing `struct!` or checked on `source.ffi.v1` must explicitly write `#import std.ffi.unsafe;` (`Q032` if omitted, including across multi-file bundles: one file's import never licenses another file). Calling C still requires `#import std.extern;` (`Q015` / `Q022`), and output lines require `std.io` / `std.logger` / `std.fmt` (`Q015`).

2. **Compiler-derived layout verification (`Q051`–`Q053`, `Q063`, `Q064`).**
   - `.qr` source never writes byte offsets, sizes, or alignments. Instead, `from "HEADER.h"` resolves strictly relative to the declaring `.qr` file's directory (single candidate path, no include-path search).
   - `--recorded-target` is **mandatory** and must match the running Clang (`compiler` equal to Clang's first `--version` line, `triple` equal to `-dumpmachine`). `qcore_c_gateway::ffi_layout` invokes that Clang's AST and record layout engine to extract the complete `struct TAG`.
   - Each listed field (1 to 32 fields, distinct names; `Q051` / `Q053`) must be a direct member of `struct TAG` whose canonical Clang type equals the canonical Clang type of the `.qr` field's C11 projection (`int64_t` and `ptrdiff_t` are both `long` on LP64, so `i64` and `isize` both match `long`, while `long long` matches neither; `f32`/`f64` match `float`/`double`), listed in **increasing layout offset order**.
   - **Refusals:** Missing/mismatched `--recorded-target`, unreadable header, incomplete/unknown `struct TAG`, `union`, bit-fields, flexible array members, anonymous members, any alignment/packing attribute (`packed`, `aligned`, `#pragma pack`), or a listed member lacking a §G02t equivalent refuses at `Q063`. A listed field that is absent from `struct TAG`, has a different canonical C type, or appears out of layout order refuses at `Q064`.
   - **Opaque unlisted members:** C struct members of any type may be left unlisted as opaque padding/payload bytes. Construction zero-initializes the entire C struct (`memset` / zeroed storage) before writing listed fields; attempting to read, write, or construct an unlisted or unknown field name refuses (`Q051`).

3. **Record operations and `.qr` by-value helper functions (`W48-QR-USABLE-RECORDS`).**
   - Construction (`let r = Hdr { a: 1, b: 2 };`) must supply every listed field exactly once (`Q051`) with an exact type match (`Q052`) and range-checked literals (`Q053`).
   - `r.field = val;` mutates a listed field with an exact-type value (`Q051` / `Q052` / `Q053`) and derives the `mut` effect (`Q029`). `let r2 = r;` copies a record or scalar by value. Nested imported struct fields remain a separate follow-on (`W48-QR-FFI-NESTED-AND-ARRAY-FIELDS`).
   - **`.qr` helpers:** Non-entry `fn` declarations take §G02t scalars and imported records **by value** (`ptr<…>` parameters on `.qr` helpers refuse at `Q026`) and return `unit`, a scalar, or an imported record by value. If `effects` is omitted on a helper, its effect row is inferred; if written, it must cover the body (`Q029`). Helpers are checked per call site over copied values, with an **acyclic call graph of depth at most 16**; unreachable/uncalled helpers refuse (`Q026`), and helper calls may **not** write `succeeds` (`Q025`). Helpers emit as `static RESULT qr_fn_NAME(…)`.

4. **Contracted `#c_ffi` calls, generated `.qrlib` v2 linkage, and outcomes.**
   - Any C call carrying an imported struct (by value, `ptr<const, NAME>`, or by-value struct return) is admitted **only** if its `.qv` contract declaration carries `[| quant #c_ffi |]` with matching `c"struct TAG"` spellings (`Q033` if unmarked; `[| quant #c_ffi |]` in a non-contract file also refuses at `Q033`). Unmarked custody-free scalar C contracts (`resource_borrowed` on all parameters, `resource_return_none()`) may also be called. Mutable struct pointers (`ptr<mut, NAME>`), pointer returns, and `unsafe extern fn` refuse (`Q024` / `Q034`).
   - **Mandatory v2 `.qrlib` linkage (`Q041`, `Q047`, `Q064`–`Q067`):** Every called C function must resolve via `extern library NAME;` in the declaring `.qr` file to an entry in a **generated v2 `.qrlib` manifest** ([G02d.2.1](#g02d21-the-generated-manifest-v2)) naming the exact contract module and C signature (`Q065` if missing or v1; `Q066` if unbound; `Q041` on module/signature mismatch; `Q047` if non-total), and for any struct-carrying callee the manifest's `header_hash` must equal the SHA-256 of the header read by `struct!` (`Q064`). Every certificate must carry the linkage premise (`Q067`), surfacing `linked:NAME` in the verdict.
   - **Declared outcomes & C emission:** C return values are unknown until declared via `succeeds LIT` (scalar/float, compared bit-for-bit on floats) or `succeeds NAME { field: LIT, … }` (struct, every listed field literal). Emitted C `#include`s the real C header without redeclaring `struct TAG`, emits `_Static_assert`s on `sizeof`, `offsetof`, and `_Generic` member types so post-check header drift fails C compilation, passes `ptr<const, NAME>` as `&r_NAME` and by-value structs as `r_NAME`, compares actual C results against `succeeds` (exiting `70` on mismatch), and refuses rendering of undetermined results (`Q043`).

### Concrete Example

```c
/* pkt.h */
#include <stdint.h>
struct qr_pkt {
    uint16_t magic;
    uint16_t payload_len;
    void *opaque_Internal; /* unlisted opaque member — zeroed on construction */
    double weight;
};
uint32_t qr_pkt_score(const struct qr_pkt *p);
```

```qv
// contracts/pkt.qv
module contract;

[| quant #c_ffi |]
extern fn qr_pkt_score(p: ptr<const, c"struct qr_pkt">) -> u32
contract {
    requires resource_contract_v1;
    ensures normal resource_borrowed(p);
    ensures normal resource_return_none();
};
```

```qr
// app.qr
module executable;
#import std.ffi.unsafe;
#import std.extern;
#import std.fmt;
#import std.io;
extern library qr_pkt;

struct! {
    type Packet = c"struct qr_pkt" from "pkt.h";
    fields {
        magic: u16,
        payload_len: u16,
        weight: f64,
    }
}

extern fn qr_pkt_score(p: ptr<const, Packet>) -> u32;

fn prepare(len: u16) -> Packet effects { mut } {
    let p = Packet { magic: 1, payload_len: 0, weight: 2.5 };
    p.payload_len = len;
    return p;
}

fn main() -> unit effects { io, mut } {
    let pkt = prepare(64);
    let s = qr_pkt_score(pkt) succeeds 65;
    print dec s;
    return;
}
```

**Implementation & Provenance:** `crates/qcore-frontend/src/source_ffi.rs`, `crates/qcore-core/src/ffi.rs`, `crates/qcore-c-gateway/src/ffi_layout.rs`, `crates/qcore-contract/src/contract_loader.rs` (`ffi_declarations`); language spec [§L14.6, §L14.8, §L20](QCORE_SPEC_r0_3.md), contract spec [§C04.12](QCORE_CONTRACT_SPEC_r0_3.md#c0412-ffi-function-definitions--quant-c_ffi-); cards `W48-QR-C-STRUCT-IMPORT`, `W48-QR-USABLE-RECORDS`, `W48-QR-FLOATING-POINT`, `W48-QRLIB-LINKAGE`; reports [w48-qr-c-struct-import](reports/archive/wave-48/w48-qr-c-struct-import/README.md), [w48-qrlib-linkage](reports/archive/wave-48/w48-qrlib-linkage/README.md).

---

# Part V — Contract Loader and Property Clause Grammars (`.qv`)

Part V defines the declarative `.qv` (legacy `.qcore`) contract grammars consumed by Quirin's verification, C-gateway, bounds, null-dereference, runtime-plugin, and lifetime loaders. Every contract loader operates **total-or-reject** at module granularity: each vocabulary is gated by an explicit `requires *_contract_v1;` selector, unknown or cross-vocabulary clauses reject the entire module rather than dropping unread facts, and every loaded declaration remain a **declared assumption** (or specification to check against a C body) rather than self-proving C source truth.

---

<a id="g05-resource-loader-v1"></a>
## G05. External resource contract loader v1 (`resource_contract_v1`)

### Consumer and Selection Mechanism

`crates/qcore-contract/src/contract_loader.rs` is the total-or-reject bridge from `.qv` resource contract modules to the legacy-C resource custody analyzer (`qcore-c-gateway`), the `.qr` extern call bridge (`qcore_contract::extern_bridge`, [G02d](#g02d-bounded-extern-calls)), and independent certificate replay.

- **Selector:** Every `extern fn` declaration in a module passed to this loader **must** carry `requires resource_contract_v1;` exactly once. Declarations lacking the marker, modules holding other top-level items (besides `alias #NAME = @ENUM;` and pre-elaborated [G05a](#g05a-contract-inclusion-and-inheritance) `#include` directives), and clauses belonging to observation ([G06](#g06-observation-predicate-grammar)), bounds ([G05b](#g05b-bounds-contract-v1)), or outer ([G05g](#g05g-outer-contract-v1)) vocabularies reject the entire module atomically.
- **Function-map withholding discipline:** Plain custody, kind, derived-result, variadic-tail, and no-return declarations populate `ResourceContracts::functions`. Declarations carrying richer relational, retention, callback, slot, reference-count, hand-off, member-path, descriptor-cell, review-hold, or `#c_ffi` clauses are validated in full and **withheld from the plain function map** into dedicated side tables so that any consumer unaware of a richer effect sees an uncontracted callee (`Unknown`) and never silently weakens it to `resource_borrowed`.

### Consolidated Grammar

```ebnf
resource_module   = "module", "contract", ";",
                    { include_directive },                            (* elaborated first, §G05a *)
                    { alias_declaration },
                    resource_decl, { resource_decl | alias_declaration } ;

alias_declaration = "alias", "#", IDENT, "=", "@", IDENT, ";" ;

resource_decl     = [ c_ffi_attr ],
                    "extern", "fn", IDENT, "(", [ param_list ], ")",
                    "->", result_abi,
                    ( "contract", "{", resource_clause, { resource_clause }, "}"
                    | "contract", "inherit", parent_ref ),            (* elaborated first, §G05a *)
                    ";" ;

c_ffi_attr        = "[|", "quant", "#", "c_ffi", "|]" ;               (* §C04.12 / §G02u *)
param_list        = parameter, { ",", parameter } ;
parameter         = IDENT, ":", parameter_abi ;

(* ---- ABI Type Projection (§C04.1) ---- *)
parameter_abi     = scalar_abi
                  | pointer_abi
                  | "fnptr"
                  | "va_list"
                  | c_struct_abi ;                                    (* admitted under c_ffi_attr only *)
result_abi        = "unit"
                  | scalar_abi
                  | pointer_abi
                  | "fnptr"
                  | C_STRUCT_LIT ;                                    (* admitted under c_ffi_attr only *)

scalar_abi        = "bool"
                  | "i8" | "i16" | "i32" | "i64" | "isize"
                  | "u8" | "u16" | "u32" | "u64" | "usize"
                  | "f32" | "f64" ;                                   (* C enums project to underlying integer *)

pointer_abi       = "ptr", "<", "mut", ",", "u8", ">"
                  | "ptr", "<", "const", ",", "u8", ">"
                  | "ptr", "<", "mut", ",", "ptr", "<", "mut", ",", "u8", ">", ">"
                  | "ptr", "<", "mut", ",", "ptr", "<", "const", ",", "u8", ">", ">" ;

c_struct_abi      = C_STRUCT_LIT | "ptr", "<", "const", ",", C_STRUCT_LIT, ">" ;
C_STRUCT_LIT      = "c", STRING ;                                     (* exact text "struct TAG", §G02u *)

(* ---- Clause Envelope ---- *)
resource_clause   = "requires", req_clause, ";"
                  | "ensures", "normal", ens_clause, ";"
                  | quant_resource_clause
                  | condition_binding                                 (* §G05h *)
                  | condition_block                                   (* §G05h *)
                  | cell_protocol_clause                              (* §G05i *)
                  | cell_frame_clause                                 (* §G05k *)
                  | link_clause                                       (* §G05m *)
                  | premises_block ;                                  (* §G05m *)

(* ---- Precondition Clauses (requires) ---- *)
req_clause        = "resource_contract_v1"
                  | literal_argument_premise
                  | "resource_body_checked"
                  | contract_review_marker
                  | contract_revision_marker
                  | library_version_range
                  | "linkage_contract_v1"                             (* §G05m, §C15.2 *)
                  | named_premise
                  | hypothesis_all
                  | hypothesis_decomposition
                  | premise_flow                                      (* §G05-flow *)
                  | lifecycle_clause                                  (* §G05-lifecycle *)
                  | guarded_symbol_body                               (* §G05-guarded-source *)
                  | guarded_source_body                               (* §G05-guarded-source *)
                  | discriminant_clause
                  | loop_owner_clause
                  | thread_sync_clause
                  | paired_lifecycle_clause ;

literal_argument_premise = "resource_literal_argument", "(", IDENT, ",", canonical_int, ")" ;
contract_review_marker   = "resource_review_pending", "(", review_note, ")" ;
contract_revision_marker = "resource_revision", "(", revision_mode, ")" ;
revision_mode            = "pinned" | "flexible" ;
review_note              = STRING, [ ",", STRING ], [ ",", "rewrite_before_acceptance" ] ;

library_version_range    = "resource_library_version", "(", library_name, ",",
                           ( "since" | "documented" ), STRING,
                           [ ",", "removed", STRING ], ")" ;

  (* card CONNECT-ADOPT-DECLARED-DEPENDENCIES: the written form of card
     QUIRIN-AUDIT-CONNECT, `resource_contract_dependency(...)`, is retired and
     refused by name (`QL005`); a dependency is a `linkage_contract_v1` link,
     §G05m, whose identity the load-path manifest records *)

named_premise            = "resource_premise", "(", premise_name, ",", premise_formula, ")" ;
hypothesis_all           = "resource_premise_all", "(", premise_name, ",",
                           premise_name, { ",", premise_name }, ")" ;
hypothesis_decomposition = "resource_premise_decomposition", "(", premise_name, ",",
                           premise_name, { ",", premise_name }, ")" ;

discriminant_clause      = "resource_discriminant", "(", IDENT, ",", member_path, ",", canonical_nat, ")" ;
loop_owner_clause        = "resource_loop_owner", "(", IDENT, ",", ( "read" | "mutate" ), ")" ;
thread_sync_clause       = "resource_thread_sync", "(", IDENT, ",", IDENT, ",",
                           ( "cond_wait" | "cond_timedwait" ), ")"
                         | "resource_thread_sync", "(", IDENT, ",",
                           ( "cond_signal" | "cond_init" | "cond_destroy" | "join" ), ")" ;
paired_lifecycle_clause  = "resource_lifecycle", "(", IDENT, ",", ( "start" | "stop" | "destroy" ), ")" ;

(* ---- Postcondition Clauses (ensures normal) ---- *)
ens_clause        = resource_predicate
                  | guarded_release_body
                  | arm_predicate
                  | return_code_disposition
                  | member_cell_predicate
                  | clause_review_marker
                  | clause_revision_marker ;

clause_review_marker   = "resource_review_pending", "(", resource_predicate, ",", review_note, ")" ;
clause_revision_marker = "resource_revision", "(", resource_predicate, ",", revision_mode, ")" ;

resource_predicate = param_mode_predicate
                   | result_mode_predicate
                   | kind_predicate
                   | tail_or_exit_predicate
                   | counted_user_predicate
                   | container_predicate
                   | retention_predicate
                   | slot_predicate
                   | reference_predicate
                   | callback_predicate
                   | result_state_predicate
                   | member_helper_predicate ;

param_mode_predicate  = "resource_borrowed", "(", IDENT, ")"
                      | ( "resource_owned" | "resource_consumed"
                        | "resource_released" | "resource_out" ),
                        "(", IDENT, ",", validity, ")"
                      | ( "resource_stored" | "resource_removed" ),
                        "(", IDENT, ",", IDENT, ",", nullability, ")" ;

guarded_release_body  = "resource_released", "(", IDENT, ",", "valid", ")", member_equality ;

result_mode_predicate = "resource_return_none", "(", ")"
                      | "resource_return_owned", "(", validity, ")"
                      | "resource_return_reallocated", "(", IDENT, ",", nullability, ")"
                      | "resource_return_removed", "(", IDENT, ",", nullability, ")"
                      | "resource_return_alias", "(", IDENT, ",", nullability, ")"
                      | "resource_return_interior", "(", IDENT, ",", nullability, ")"
                      | "resource_return_borrowed_from", "(", IDENT, ",", nullability, ")" ;

nullability           = "nullable" | "nonnull" ;
validity              = nullability | "sentinel" | "valid" ;

kind_predicate        = "resource_kind", "(", IDENT, ",", param_kind_name, ")"
                      | "resource_kind_failure_use", "(", IDENT, ",", IDENT, ")"
                        (* card CG25-IN-PLACE-REGEX-REGERROR-FAILURE-EDGE, contract
                           specification C04.6.1: the second IDENT an in-place kind *)
                      | "resource_return_kind", "(", return_kind_name, ")" ;
param_kind_name       = "descriptor" | "stream" | "directory"
                      | "dynamic_library" | "regex" | "thread_key" ;
return_kind_name      = "descriptor" | "stream" | "directory"
                      | "dynamic_library" | "thread_key" | "reference" ;

tail_or_exit_predicate = "resource_variadic_data_tail", "(", ")"
                       | "resource_variadic_borrowed_tail", "(", ")"
                       | "resource_no_return", "(", ")" ;

return_code_disposition = "resource_on_return", "(", relation_op, ",", canonical_int, ",",
                          disposition_predicate, ")" ;
relation_op           = "eq" | "ne" | "lt" | "le" | "gt" | "ge" ;
disposition_predicate = "resource_borrowed", "(", IDENT, ")"
                      | ( "resource_owned" | "resource_consumed"
                        | "resource_released" | "resource_out" ),
                        "(", IDENT, ",", validity, ")" ;

counted_user_predicate = ( "resource_user_added" | "resource_user_dropped"
                         | "resource_user_dropped_checked" | "resource_user_release_check" ),
                         "(", IDENT, ",", IDENT, ")"
                       | "resource_user_neutral", "(", ")"
                       | "resource_user_exit", "(", IDENT, ",", canonical_int, ",",
                         ( "added" | "restored" | "dropped" ), ")" ;

container_predicate   = "resource_published", "(", IDENT, ",", IDENT, ")"
                      | "resource_unpublished", "(", IDENT, [ ",", IDENT ], ")" ;

retention_predicate   = "resource_retained_by", "(", IDENT, ",",
                        ( IDENT | "global" | "result" ), ")", [ retention_guard ]
                      | "resource_retained_by", "(", IDENT, ",", holder_path, ")"
                      | "resource_holder_reset", "(", IDENT, ")" ;
holder_path           = IDENT, ".", member_path ;
retention_guard       = "when", IDENT, guard_op, canonical_int ;
guard_op              = "==" | "<" | ">" | "<=" | ">=" ;

slot_predicate        = "resource_slot_moved", "(", IDENT, ",", nullability, ")"
                      | "resource_out_derived", "(", IDENT, ",", IDENT, ",", nullability, ")" ;

reference_predicate   = "resource_reference_kept", "(", IDENT, ")"
                      | "resource_reference_dropped", "(", IDENT, [ ",", "nullable" ], ")" ;

callback_predicate    = "resource_callback_invoked", "(", IDENT, { ",", callback_actual }, ")"
                      | "resource_callback_invoked_or_default", "(", IDENT, { ",", callback_actual }, ")"
                      | "resource_callback_table", "(", IDENT, { ",", callback_actual }, ")"
                      | "resource_callback_registered", "(", IDENT, ",", ( IDENT | "result" ), ")"
                      | "resource_callback_registry_invoked", "(", IDENT, { ",", ( callback_actual | "element" ) }, ")"
                      | ( "resource_callback_unregistered" | "resource_elements_dropped"
                        | "resource_released_if_constructed" ), "(", IDENT, ")"
                      | "resource_element_added", "(", IDENT, ",", IDENT, ")"
                      | "resource_callback_member_invoked", "(", ( IDENT | "supplied_owned" ), ",",
                        IDENT, ".", member_path, { ",", callback_actual }, ")"
                      | "resource_callback_member_registered", "(", IDENT, ",",
                        IDENT, ".", member_path, ",", IDENT, ")"
                      | "resource_callback_member_registered", "(", "#", IDENT, ",",
                        IDENT, ".", member_path, ",", IDENT, ")",
                        "where", "#", IDENT, "=", "#", "result", "in", holder_path
                      | "resource_callback_slot_invoked", "(", IDENT, ",",
                        IDENT, ".", IDENT, { ",", callback_actual }, ")"
                      | "resource_callback_slot_registered", "(", IDENT, ",",
                        IDENT, ".", IDENT, ",", IDENT, ")" ;
callback_actual       = IDENT | "_" | "supplied_owned" | "supplied_nullable" ;

result_state_predicate = "resource_discriminant_set", "(", "result", ",",
                         IDENT, ".", member_path, ",", IDENT, ",", canonical_nat, ")"
                       | "resource_callback_member_registered", "(", "result", ",",
                         IDENT, ".", member_path, ",", ( IDENT | "#", "absent" ), ")" ;

arm_predicate         = ( "resource_arm_released", "(", IDENT, ",", member_path, ")"
                        | "resource_arm_empty", "(", IDENT, ")" ),
                        ( "when", member_path, "==", canonical_int | "otherwise" ) ;

member_helper_predicate = "resource_member_helper", "(", IDENT, ",", IDENT, ".", member_path, ",", IDENT, ")" ;

member_cell_predicate = "resource_member_kind", "(", IDENT, ",", IDENT, ".", member_path, ",", cell_kind, ")"
                      | "resource_member_released", "(", IDENT, ",", IDENT, ".", member_path, ",", "stale", ")"
                      | "resource_on_return", "(", ( "eq" | "ne" ), ",", ( "0" | "-", "1" ), ",",
                        ( ( "resource_member_out" | "resource_member_stale_or_kept" ),
                          "(", IDENT, ",", IDENT, ".", member_path, ")"
                        | "resource_member_alias", "(", IDENT, ",", IDENT, ".", member_path, ",", IDENT, ")" ),
                        ")" ;

(* ---- Quantitative Clauses Admitted in Resource v1 ---- *)
quant_resource_clause = proposition_clause                            (* §G01p: quant proposition #id(...) = ...; *)
                      | result_state_clause
                      | handoff_clause
                      | removal_clause
                      | member_path_clause
                      | descriptor_cell_clause ;

result_state_clause   = "quant", "live", "#", "result",
                        [ ":", IDENT, ".", member_path, MAPS_TO, ( "#", "absent" | IDENT ) ], ";" ;

handoff_clause        = "quant", "live", "#", IDENT, ":", "#", "owner", MAPS_TO, "#", "callback",
                        "when", "#", "result", ( "==" | "!=" ), enum_const_ref, ";" ;

removal_clause        = "quant", "live", "#", IDENT, ":", "#", "holds", "(", IDENT, ")",
                        MAPS_TO, "#", "absent", [ member_equality ], ";" ;

member_path_clause    = "quant", "live", "#", IDENT, ":", IDENT, ".", member_path, MAPS_TO,
                        ( path_fate | "{", "#", ( "released" | "moved" | "stale" ), ",", "#", "kept", "}" ),
                        [ "when", "#", "result", ( "==" | "!=" ), enum_const_ref
                        | "when", "#", IDENT, ( "==" | "!=" | "<" | ">" | "<=" | ">=" ),
                          ( canonical_int | enum_const_ref ) ], ";" ;
path_fate             = "#", ( "released" | "moved" | "stale" | "kept" ) ;

descriptor_cell_clause = "quant", "live", "#", IDENT, ":",
                         ( IDENT, ".", member_path | "#", "pointee" ), MAPS_TO,
                         cell_value, [ witness_guard ], ";" ;
cell_kind             = "descriptor" ;
cell_value            = "#", "fresh", "(", cell_kind, ")"
                      | "#", "stale", "(", cell_kind, ")"
                      | "{", "#", "stale", "(", cell_kind, ")", ",", "#", "kept", "}"
                      | "#", "same", "(", IDENT, ")" ;
witness_guard         = "when", "#", "result",
                        ( "!=", "-", "1" | "==", "-", "1" | "==", "0" | "!=", "0" ) ;
member_equality       = "when", "#", IDENT, ":", IDENT, ".", member_path, "==", "#", IDENT ;

(* ---- Lexical & Shared Nonterminals ---- *)
enum_const_ref        = "@", IDENT | "#", IDENT ;
member_path           = IDENT, { ".", IDENT } ;
MAPS_TO               = "|->" ;
premise_name          = [a-z][a-z0-9_]* ;
library_name          = [a-z_][a-z0-9_]* ;
canonical_nat         = "0" | [1-9][0-9]* ;
canonical_int         = "0" | [ "-" ], [1-9][0-9]* ;
```

### Retired Spellings and Parse-Time Format Bans (`QF001`, `QF002`, `QF005`)

The contract format enforces strict bans on retired prose and comment carriers across **all** contract loaders (never skipped or downgraded):

| Retired Form | Replacement | Refusal / Diagnostic |
| --- | --- | --- |
| `requires resource_premise(NAME, "PROSE");` (or any premise statement containing a `STRING` token) | Symbolic `premise_formula` ([G05-formula](#g05-formula-symbolic-premise-statements)) | Rejected at parse across every route with **`QF001`** (`SP5-GLOBAL-PROSE-PREMISE-BAN`). |
| `resource_hypothesis(ID, "PROSE")` (under any introducer or arguments) | Typed `quant proposition #id(...) = formula;` (§G01p) | Rejected at parse across every route with **`QF002`** (`SP6-SYMBOLIC-HYPOTHESIS-ATOMS`). |
| Non-symbolic strings in `resource_guarded_symbol_v1` / `resource_guarded_decomposition_v1` | Unquoted `guarded_formula` and `source_clause` ([G05-guarded-source](#g05-guarded-source-explicit-guarded-source-embedding)) | Rejected at parse with **`QF003`** (`SP6-GUARDED-SYMBOL-PROSE`). |
| Prose strings in `quant #outer [[contract]] NAME, "PROSE"` | Unquoted `premise_formula` ([G05g](#g05g-outer-contract-v1)) | Rejected at parse with **`QF004`** (`SP8-OUTER-CONTRACT-SYMBOLIC-STATEMENTS`). |
| Non-canonical strings in formal-binding JSON manifests (`environment.variables`, `proposition`, `quantifiers`, `conclusion`) | Canonical `DIGEST` or sorted `TERM_LIST` JSON, qualified `A.B` proposition name, exact atom parameters & formula JSON | Rejected on every load route with **`QF005`** (`SP10-FORMAL-BINDING-DESCRIPTIONS`). |
| `// qcore-premise: ...` machine-parsed comment directive anywhere in module text | `requires resource_literal_argument(p, LIT);` | Whole module rejected by `contract_loader.rs` (`RETIRED_PREMISE_DIRECTIVE`). |

### Semantic Side Conditions by Clause Family

1. **Core ABI, Enumerations, Double Pointers, `fnptr`, `va_list`, and Undeclarable Names (§C04.1).**
   - Every parameter and the result must appear in **exactly one** base mode (or covered across every class of a `resource_on_return` disposition). Parameter names must be unique; no usage annotations, generic brackets, `effects` rows, `ensures any`, or `ensures error(...)` are admitted.
   - **Canonical integer literals:** `canonical_int` forbids leading zeros (`00`, `010`), `-0`, `+`, hex/binary prefixes, and integer suffixes (`0L`, `0u`).
   - **C Enumerations (rule 12u, `W43-ABI-ENUM-AND-CONST-PP`):** C `enum` parameters and results (tagged, untagged, or typedef'd) have no dedicated contract keyword; they are spelled as the exact fixed-width integer (`i32`, `u32`, `i64`, `u64`, …) matching the underlying integer type Clang reports for that enum in the analyzed unit (never assumed `int`, never `isize`/`usize`).
   - **Pointers to pointers to `const` (rule 12v, `W43-ABI-ENUM-AND-CONST-PP`):** `ptr<mut, ptr<const, u8>>` reads qualifiers outside-in: an unqualified mutable outer slot pointing to a `const` inner pointee (`const char **`, `const void **`). `char **` and `const char **` do not match each other; slot-qualified `char *const *` has no spelling and refuses; `resource_out` requires `ptr<mut, ptr<mut, u8>>` (except `resource_out_derived`, which admits both double-pointer spellings).
   - **`fnptr` and `va_list`:** Both are non-`ptr<…>` ABI tokens and therefore admit `resource_borrowed` **only** (never custody modes or owned returns). `va_list` (`__builtin_va_list` / `__gnuc_va_list`) is a **parameter-only** type and is rejected in `result_abi`.
   - **`UNDECLARABLE_C_NAMES`:** Functions with non-standard control flow or Clang AST pseudo-call shapes (`setjmp`, `longjmp`, `siglongjmp`, `setcontext`, `pthread_exit`, `thrd_exit`, `__builtin_choose_expr`, `__builtin_offsetof`, …) are unconditionally rejected regardless of clauses.
   - **C source & body agreement (`LOADER-ABI`, `LOADER-ABI-MODES`):** Consumers withdraw a declaration in any C unit whose own prototype disagrees in arity, parameter/result ABI projection, or variadic status, or whose validated body summary contradicts a parameter/result custody duty.

2. **Custody Modes, Derived/Borrowed/Removed Results, Resource Kinds, Variadic Tails, and `resource_no_return()`.**
   - **Stored/Removed (`resource_stored`, `resource_removed`):** Second `IDENT` (`owner`) must be a distinct `resource_owned` parameter. `resource_return_removed(item, n)` (§C04.3.5) requires matching `resource_removed(item, parent, n)` and `resource_owned(parent, nonnull)` on distinct mutable depth-one heap pointers, all other parameters `resource_borrowed`, no rich clauses/tails, and — for physical C body verification — the `detach_list_representation` named premise.
   - **Derived & Borrowed Results (§C04.3.1, §C04.3.4):** `resource_return_alias(p, n)` and `resource_return_interior(p, n)` require `p` to be `resource_borrowed` on `ptr<mut, u8>` or `ptr<const, u8>` with a depth-one pointer result (`const_dropped` recorded if `ptr<const, u8>` → `ptr<mut, u8>`). `resource_return_borrowed_from(p, n)` (`SLOT-MOVE-AND-INTERIOR-BORROW`) additionally admits `ptr<mut, ptr<mut, u8>>` and `ptr<mut, ptr<const, u8>>` results (arrays of parent-owned children) and withholds the declaration into the retaining table if paired with slot/retention clauses.
   - **Resource Kinds (§C04.6, §C04.6.1):** `heap` is spelled solely by omitting a kind clause (writing `heap` rejects). `descriptor` requires an `i32` position and **must** spell validity `sentinel` (`-1` failure) or `valid` (never `nullable`/`nonnull`; conversely `sentinel`/`valid` on any non-descriptor position rejects). In-place `regex` stands on `ptr<mut, u8>` (or `ptr<const, u8>` for borrowed use): `resource_out(p, nullable)` + `resource_kind(p, regex)` on an `i32` `resource_return_none()` initializer, `resource_released(p, nonnull)` + `resource_kind(p, regex)` finalizer, or `resource_borrowed(p)` + `resource_kind(p, regex)` use. `thread_key` (`pthread_key_t`) also admits by-value `u32` finalizers and uses (`resource_released(k, nonnull)` / `resource_borrowed(k)` + `resource_kind(k, thread_key)`). `reference` is valid **only** in `resource_return_kind(reference)` beside `resource_return_owned` (writing `resource_kind(p, reference)` on a parameter rejects).
   - **Failure use (§C04.6.1):** The failure use `resource_kind_failure_use(p, KIND)` (card CG25-IN-PLACE-REGEX-REGERROR-FAILURE-EDGE, contract specification C04.6.1, gateway rule 74n) carries static conditions of its own: `p` names a parameter of the declaration whose mode is `resource_borrowed`, whose type is `ptr<const, u8>` or `ptr<mut, u8>`, and which carries `resource_kind(p, KIND)` of the same kind; `KIND` is an in-place kind (`regex`, `thread_key`), never `heap`, `reference` or a pointer- or integer-carried kind; and a parameter carries at most one failure use. An initialiser (`resource_out`) or finaliser (`resource_released`) position, a by-value `u32` position, a position without the kind or of another kind, and a duplicate each reject the whole module. It is a predicate, never a string: a premise statement is refused (QF001). It loads as `ResourceFunctionContract::failure_uses`, omitted from the serialized form and the diagnostic text when empty.
   - **Variadic Tails & No-Return (§C04.2, §C04.3.3):** At most one of `resource_variadic_data_tail()` or `resource_variadic_borrowed_tail()` per declaration (requires C `...` prototype). `resource_no_return()` appears at most once, requires `-> unit`, `resource_return_none()`, and `resource_borrowed` on all parameters, and forbids counted-user, container, loop-owner, thread-sync, and lifecycle clauses alongside it.

3. **Literal-Argument Premises, Review/Revision Markers, Version Bounds, and Obligation Clauses.**
   - **`resource_literal_argument(p, LIT)` (§C04.5):** `p` may be any parameter type (including pointer `0`, which structurally matches `0`, `(0)`, `0L`, and — for `ptr<…>`/`fnptr` at `0` — `((void *) 0)`). At most one premise per parameter. Beside retaining/callback clauses, admitted **only** when every retaining clause is a synchronous `resource_callback_invoked`, `resource_callback_invoked_or_default`, or opaque `resource_callback_slot_invoked` (gateway rule 57).
   - **Declared Contract Dependencies (§C15.2, §G05m):** `requires linkage_contract_v1;` and the links it admits are read by the resource v1 loader; the retired written form `resource_contract_dependency(...)` rejects the whole module (`QL005`). Inheritance copies neither a link nor the marker; the inherit directive is itself the child's link.
   - **Review & Revision Markers (§C06.1–C06.3):** `review_note` strings must be non-blank single-token literals with no backslash escapes or control characters. Either one contract-level `requires resource_review_pending(…);` **or** clause-level `ensures normal resource_review_pending(PRED, …);` wrappers (never both, never nested). Likewise, at most one `requires resource_revision(MODE);`, and `ensures normal resource_revision(PRED, MODE);` may not restate the contract-level mode or nest with review markers (unmarked default is `pinned`). `requires resource_library_version(lib, since|documented "M.N[.P]" [, removed "M.N[.P]"]);` appears at most once with `removed` strictly greater than the lower bound. `requires resource_body_checked;` appears at most once.
   - **Concurrency & Lifecycle Obligations (§C04.9–C04.11):** At most one `resource_loop_owner` (pointer parameter), `resource_thread_sync` (`ptr<mut, u8>` condvar and distinct `ptr<mut, u8>` mutex for waits; `u64`/`usize` thread for `join`), or `resource_lifecycle` (`ptr<mut, u8>` context); none may be wrapped in a clause-level review marker.
   - **Premises & Hypothesis Graphs (§C04.18):** `resource_premise(NAME, FORMULA)` requires unique `NAME`s per declaration and a valid [G05-formula](#g05-formula-symbolic-premise-statements). `resource_premise_all` and `resource_premise_decomposition` take non-empty, unique, strictly increasing atom IDs referencing `quant proposition #id` (§G01p) definitions (`legacy_` prefix reserved; `resource_premise_decomposition` attaches to an existing named premise in graph v2).

4. **Return-Code Dispositions, Counted Users, Container Publication, Retentions, Slots, and Reference Counts.**
   - **`resource_on_return(OP, VAL, PRED)` (§C04.13):** Requires a signed integer result with `resource_return_none()`, canonical `int`-fitting `VAL`, and exit classes that are **pairwise disjoint and partition all integers** (`VAL - 1`, `VAL`, `VAL + 1` check). Conditional parameters must agree in entry custody demand across classes; classes may not all produce identical contracts; incompatible with in-place kinds, review/checked markers, literal premises, variadic tails, and counted-user/container/sync/lifecycle clauses.
   - **Counted Users & Containers (§C04.7, §C04.8):** Operate on `resource_borrowed` pointer parameters — **except** an *element destroyer*, where `resource_unpublished(p [, link_member])` sits beside `resource_released(p, …)` (and optionally member-path states, rules 34/43/46; `link_member` is forbidden without `resource_released`). `resource_user_neutral()` stands alone; a parameter takes either one user operation or `resource_user_exit` clauses (requiring integer return), never both.
   - **Retentions, Holder Paths & Removals (§C04.14):** `resource_retained_by(P, H)` requires `P` (and `H` if a formal) to be distinct depth-one `resource_borrowed` pointers with no kind (`result` holder requires `resource_return_owned(nonnull)`). Optional `when X OP K` (not admitted on `holder_path` `FORMAL.FIELD…`) requires `X` to be a distinct integer parameter and `K` to partition `X`'s range (`==`, `<`, `>`, `<=`, `>=` only). `quant live #H: #holds(P) |-> #absent;` removes one registration on distinct borrowed depth-one pointers `H` and `P` (never beside a retention of the same pair; `holds` and `absent` forbidden as parameter names).
   - **Slots & Reference Counts (§C04.15–C04.17):** `resource_slot_moved(s, n)` requires `ptr<mut, ptr<mut, u8>>`; `resource_out_derived(s, base, n)` requires double-pointer `s` and distinct depth-one `base` (all `resource_borrowed`, unkinded). At most one `reference_predicate` per declaration on a depth-one unkinded `resource_borrowed` pointer: `resource_reference_kept(p)` requires `resource_return_alias(p, nonnull)` or `resource_return_none()`; `resource_reference_dropped(p [, nullable])` requires `resource_return_none()`.

5. **Callbacks, Result States, Hand-Offs, Member-Path States, Helper Destroyers, and Descriptor Cells (§C04.19, §C04.6.2).**
   - **Callbacks (§C04.19):** `fnptr` parameters are `resource_borrowed` and named by at most one invocation or registration; holders, tables, and elements are depth-one unkinded `resource_borrowed` pointers (`_` and `element` forbidden as parameter names when used; `supplied_owned` at most once in `resource_callback_invoked` or `resource_callback_member_invoked`; `supplied_nullable` anywhere `_` stands). Slot forms (`resource_callback_slot_invoked` / `resource_callback_slot_registered`) require depth-one borrowed `IDENT` object/holder, `TAG.FIELD` with **exactly one** `.`, and forbid `supplied_owned`. Callback clauses may stand beside `reference_predicate` **only** when every reference clause is `resource_reference_dropped` and every callback clause is `resource_callback_slot_invoked`.
   - **Discriminants & Arm Releases (§C04.19.2):** `requires resource_discriminant(p, path, mask);` (at most one, on a released depth-one unkinded pointer) must pair with `resource_arm_released` / `resource_arm_empty` clauses guarded by `when path == VAL` (masked bits non-cleared) or `otherwise`.
   - **Constructor Result States (`result_state_clause`, §C04.19.4):** Requires `resource_return_owned(nonnull)`; `#absent` (never bare `absent`) or a non-parameter function designator names the default callback (`quant-live-absent`, `quant-live-value`).
   - **Return-Code Hand-Offs (`handoff_clause`, §C04.19.5):** `#P` names a `resource_owned(P, nonnull)` depth-one pointer over an integer/enum `resource_return_none()` result; constant is `@ENUM` (resolved per unit by Clang) or module `alias #NAME = @ENUM;`.
   - **Member-Path States & Helper Destroyers (`member_path_clause`, §C04.19.6, rules 50–51):** `#P` is `resource_borrowed`, `resource_released`, or the declaration's `handoff_clause` formal (`#released`, `#moved`, `#stale`, `#kept`, or `{#released|#moved|#stale, #kept}`). A `#result` guard must be the exact complement of a same-formal `handoff_clause`; a `#PARAM` guard compares a distinct integer formal against a range-partitioning literal, `@ENUM`, or `#ALIAS`. Under **rule 50** (*helper destroyer*), `resource_callback_member_invoked(P, TAG.FIELD, …)` may stand beside member-path states when all bind the same `resource_released` formal `P` with no hand-off. Under **rule 51**, at most one `resource_member_helper(P, TAG.PATH, H)` may stand beside an unguarded `#stale` state on released `P` with no callbacks or hand-off.
   - **Descriptor Cells (`descriptor_cell_clause`, §C04.6.2):** `#P` is a depth-one unkinded `resource_borrowed` pointer (`#S: #pointee` on `ptr<mut, u8>` `int *` slot); `#fresh(descriptor)` and `#same(S)` require a `witness_guard` from a single pair (`!= -1`/`== -1` or `== 0`/`!= 0`) over a signed integer `resource_return_none()` result. `guarded_release` (`when #P: TAG.FIELD == #X`) guards an `i32` formal stating `resource_kind(X, descriptor)` and every `#holds` removal of the declaration.

### Concrete Examples

```qv
module contract;

alias #OP_PASS = @RD_KAFKA_OP_RES_PASS;

// 1. Descriptor creator with variadic data tail and library version bound:
extern fn openat(dirfd: i32, path: ptr<const, u8>, flags: i32) -> i32
contract {
    requires resource_contract_v1;
    requires resource_library_version(libc, since "2.4");
    ensures normal resource_borrowed(dirfd);
    ensures normal resource_borrowed(path);
    ensures normal resource_borrowed(flags);
    ensures normal resource_variadic_data_tail();
    ensures normal resource_return_owned(sentinel);
    ensures normal resource_return_kind(descriptor);
};

// 2. Callback hand-off and complementary member-path fate with symbolic premise:
extern fn rd_kafka_poll_cb(rk: ptr<mut, u8>, rkq: ptr<mut, u8>, rko: ptr<mut, u8>) -> i32
contract {
    requires resource_contract_v1;
    requires resource_premise(rk_plugin_callback_closure, #app_callbacks |-> #neutral);
    ensures normal resource_borrowed(rk);
    ensures normal resource_borrowed(rkq);
    ensures normal resource_owned(rko, nonnull);
    quant live #rko: #owner |-> #callback when #result != #OP_PASS;
    quant live #rko: rd_kafka_op_s.rko_u.leaders.partitions |-> {#moved, #kept} when #result == #OP_PASS;
    ensures normal resource_return_none();
};

// 3. Descriptor cell creation with slot alias and witness guard:
extern fn mk_event_channel_create(
    loop: ptr<mut, u8>,
    r_fd: ptr<mut, u8>,
    w_fd: ptr<mut, u8>,
    data: ptr<mut, u8>
) -> i32
contract {
    requires resource_contract_v1;
    ensures normal resource_borrowed(loop);
    ensures normal resource_borrowed(r_fd);
    ensures normal resource_borrowed(w_fd);
    ensures normal resource_borrowed(data);
    ensures normal resource_retained_by(data, loop);
    quant live #r_fd: #pointee |-> #fresh(descriptor) when #result == 0;
    quant live #w_fd: #pointee |-> #fresh(descriptor) when #result == 0;
    quant live #data: mk_event.fd |-> #same(r_fd) when #result == 0;
    ensures normal resource_return_none();
};
```

**Implementation & Provenance:** `crates/qcore-contract/src/contract_loader.rs` and submodules (`contract_counted_users.rs`, `contract_in_place.rs`, `contract_relational.rs`, `contract_retained_by.rs`, `contract_slot_move.rs`, `contract_reference_count.rs`, `contract_callback.rs`, `contract_arm_release.rs`, `contract_constructed_state.rs`, `contract_handoff.rs`, `contract_member_path.rs`, `contract_descriptor_cell.rs`), `crates/qcore-frontend/src/premise_formula.rs`; contract spec [§C04–C06](QCORE_CONTRACT_SPEC_r0_3.md).

---

<a id="g05-formula-symbolic-premise-statements"></a>
### G05-formula. Symbolic premise statements

#### Consumer and Selection Mechanism

`qcore_frontend::premise_formula` parses, sort-checks, and canonically prints the symbolic formula occupying the second argument of `requires resource_premise(NAME, FORMULA);` (across resource-v1, bounds-v1 member premises, local-condition blocks, and model contracts) as well as the header statement of `quant #outer [[contract]] NAME, FORMULA;` ([G05g](#g05g-outer-contract-v1)). Since `SP5-GLOBAL-PROSE-PREMISE-BAN` and `SP8-OUTER-CONTRACT-SYMBOLIC-STATEMENTS`, prose string literals in these positions are banned at parse (`QF001` / `QF004`).

#### Consolidated Grammar

```ebnf
premise_formula = implication ;

implication     = conjunction, [ "->", implication ] ;              (* right-associative *)
conjunction     = maps_to, { "&&", maps_to } ;                      (* left-associative *)
maps_to         = relation, [ "|->", relation ] ;                   (* non-associative, no chaining *)
relation        = set_expr, [ ( "in" | "==" ), set_expr ] ;         (* non-associative, no chaining *)
set_expr        = primary, { "|", primary }                         (* set union *)
                | primary, "\\", primary ;                          (* set difference: never mixed or chained *)

primary         = "(", implication, ")"
                | "quant", "all", "#", term_name, "in", set_expr, ":", implication
                | "#", term_name, [ "(", implication, [ ",", implication ], ")" ]
                | CLASS_CONSTANT
                | "{", "}" ;

term_name       = [a-z][a-z0-9_]* ;
CLASS_CONSTANT  = [A-Z][A-Z0-9_]* ;
```

#### Semantic Side Conditions, Lexical Rules, and Bounds

- **Lexical tokens and `\` (`Backslash`):** Every token reuses the standard Quirin lexer (`#`, `|->`, `->`, `&&`, `==`, `|`, `{`, `}`, `(`, `)`, `:`, `,`, and contextual words `quant`, `all`, `in`), plus `\` (`Symbol::Backslash`), added specifically for set difference (`A \ B`). `\` has no multi-byte operator combinations (`\|` lexes as `\` then `|`, while `|->`, `||->`, `->|`, `!->` keep longest-match priority). `-` (`Minus`) is rejected inside `premise_formula` so set difference has one canonical spelling. A `quant all #x in S: BODY` body extends as far rightward as possible.
- **Explicit AST & token bounds:** At most **1,024 tokens**, **nesting depth 48**, and **512 AST nodes** per formula; exceeding any bound rejects the module.
- **Sort judgment (§C04.18.9):** Every `#term_name` must either be a registered C04.18.9 vocabulary symbol applied to the exact arity and sorts of its signature, or a variable bound by an enclosing `quant all` (shadowing a registered symbol or an outer variable is rejected). Every `CLASS_CONSTANT` must be a registered uppercase class constant, and the root expression must have sort `Prop`. Inside `F |-> E`, `|->` states that every invocation of callback (or callback set) `F` during the call satisfies effect specification `E`.
- **Operator gate (`Q038`) & canonical printing:** `crates/qcore-frontend/src/pipe.rs` (`premise_formula_spans`) exempts `&&` and `|` from the contract-module operator gate (`Q038`) **only** inside `resource_premise` and `quant #outer [[contract]]` clauses accepted by this reader. Digest computation and rendering always emit the **canonical printing** (single spaces around binary operators, `: ` after quantifier domain, `, ` between arguments, minimal precedence-preserving parentheses).

#### Concrete Example

```qv
requires resource_premise(
    served_queue,
    #op_callbacks |-> #takes(#held_on(#served)) == {}
);

requires resource_premise(
    ssl_slot_closure,
    quant all #cb in #registered_slots \ #verified_slots: #cb |-> #neutral
);
```

**Implementation & Provenance:** `crates/qcore-frontend/src/premise_formula.rs`, `crates/qcore-frontend/src/pipe.rs`; contract spec [§C04.18.9](QCORE_CONTRACT_SPEC_r0_3.md); cards `SP1-SYMBOLIC-OPENSSL-PREMISES`, `SP2-SYMBOLIC-LIBRDKAFKA-PREMISES`, `SP3-SYMBOLIC-FLUENT-BIT-LIBC-PREMISES`, `SP4-SYMBOLIC-LIBEVENT-PREMISES`, `SP5-GLOBAL-PROSE-PREMISE-BAN`.

---

<a id="g05-flow-proposed-premise-expressions"></a>
### G05-flow. Proposed premise expressions

#### Consumer and Selection Mechanism

`crates/qcore-contract/src/premise_flow.rs` validates contextual `requires resource_premise_flow_v1(…);` metadata clauses inside `resource_contract_v1`, `bounds_contract_v1` (explicit bounds view), and resource-type model contract views after full base-vocabulary validation. These clauses express structured three-valued premise composition over declared §G01p graph atoms without introducing executable source expressions or proof authority.

#### Consolidated Grammar

```ebnf
premise_flow_clause = "requires", premise_flow, ";" ;
premise_flow        = "resource_premise_flow_v1", "(", flow_id, ",", flow_expr, ")" ;

flow_expr           = atom_id
                    | "flow", "(", flow_id, ")"
                    | "(", flow_expr, flow_operator, flow_expr, ")" ;

flow_operator       = "|->" | "||->" | "!->" | "->|" ;
flow_id             = [a-z][a-z0-9_]* ;
atom_id             = [a-z][a-z0-9_]* ;
```

#### Semantic Side Conditions and Refusal Boundaries

- **Mandatory parenthesization:** Every binary `flow_operator` application **must** carry its own enclosing parentheses `(A OP B)` — no operator precedence or associativity is inferred.
- **Longest-match operators & contextual heads:** `||->`, `!->`, and `->|` are lexed atomically by longest match alongside `|->` (`MapsTo`). `resource_premise_flow_v1` and `flow` remain ordinary contextual words (zero reserved keywords added). Not admitted in `.qr` source expressions, S3 local-condition blocks ([G05h](#g05h-local-condition-and-registered-lifetime-clauses)), or `#c_ffi` declarations.
- **Reference & acyclicity checks (§C04.18.3):** `atom_id` must resolve to a declared §G01p proposition atom in the same declaration's hypothesis graph; `flow(ID)` resolves only to another `resource_premise_flow_v1` entry in the **same declaration**. Missing IDs, duplicate `flow_id` entries, reference cycles, DAG expansion blowup beyond C04.18.3 bounds, and terminal `->|` composition nested below the root (directly or transitively via `flow(ID)`) reject the entire module. Inherited flow/atom IDs are literal metadata unaffected by formal parameter renaming.

#### Concrete Example

```qv
quant proposition #env_ready(#c: cls) = #initialized(#c);
quant proposition #op_step(#f: fn, #r: run) = #runs(#f, #r);
quant proposition #post_clean(#r: run) = #neutral_run(#r);
requires resource_premise_flow_v1(step_flow, (#env_ready |-> #op_step));
requires resource_premise_flow_v1(full_flow, (flow(step_flow) ->| #post_clean));
```

**Implementation & Provenance:** `crates/qcore-frontend/src/premise_flow.rs`, `crates/qcore-contract/src/premise_flow.rs`, `crates/qcore-contract/src/premise_metadata_bridge.rs`; contract spec [§C04.18.3](QCORE_CONTRACT_SPEC_r0_3.md).

---

<a id="g05-lifecycle-typed-symbolic-lifecycle-clauses"></a>
### G05-lifecycle. Typed symbolic lifecycle clauses

#### Consumer and Selection Mechanism

`crates/qcore-frontend/src/contract_lifecycle.rs` and `crates/qcore-contract/src/contract_lifecycle.rs` parse and validate `requires resource_lifecycle_v1(…);` symbolic intent metadata across `resource_contract_v1`, `bounds_contract_v1`, and explicit resource-type model views. Like [G05-flow](#g05-flow-proposed-premise-expressions), these clauses record structured evidence-only lifecycle transitions and carry no standalone proof or custody-discharge authority.

#### Consolidated Grammar

```ebnf
lifecycle_req_clause = "requires", lifecycle_clause, ";" ;
lifecycle_clause     = "resource_lifecycle_v1", "(", entry_id, ",",
                       lifecycle_operator, lifecycle_target, ")" ;

lifecycle_operator   = "|^+" | "|^-" ;
lifecycle_target     = ( "resource" | "hypothesis" ), "(", target_id, ")" ;

entry_id             = [a-z][a-z0-9_]* ;
target_id            = [a-z][a-z0-9_]* ;
```

#### Semantic Side Conditions and Refusal Boundaries

- **Atomic operators:** `|^+` (establishment/activation intent) and `|^-` (retirement/discharge intent) are lexed atomically by longest match and are rejected in `.qr` source expressions, `G05-flow` expressions, and non-participating contract vocabularies.
- **Strict flat target structure (§C04.18.4):** The operand after `|^+` or `|^-` must be typed explicitly as `resource(target_id)` or `hypothesis(target_id)`. Bare identifiers, dotted paths, nested lifecycle operators, and trailing tokens reject the whole module. All IDs obey §C04.18.4 reference, size, and formal-renaming-invariant inheritance rules.

#### Concrete Example

```qv
quant proposition #loop_active(#o: obj) = #active(#o);
requires resource_lifecycle_v1(activate_evl, |^+ hypothesis(loop_active));
requires resource_lifecycle_v1(retire_handle, |^- resource(evl_handle));
```

**Implementation & Provenance:** `crates/qcore-frontend/src/contract_lifecycle.rs`, `crates/qcore-contract/src/contract_lifecycle.rs`; contract spec [§C04.18.4](QCORE_CONTRACT_SPEC_r0_3.md).

---

<a id="g05-guarded-standalone-readable-guarded-decomposition"></a>
### G05-guarded. Standalone readable guarded decomposition

#### Consumer and Selection Mechanism

`crates/qcore-frontend/src/guarded_decomposition.rs` and `crates/qcore-contract/src/guarded_decomposition.rs` read standalone `.qgd` (`guarded_decomposition_v1`) documents. A `.qgd` file is **not** a `.qr` program or a `.qv` contract module; it is a standalone human-auditable guarded decomposition artifact (or the canonical rendered projection of [G05-guarded-source](#g05-guarded-source-explicit-guarded-source-embedding)) verified against a supplied typed §G01p proposition map and a fully replayed property-specific source hypothesis graph.

#### Consolidated Grammar

```ebnf
guarded_document = "guarded_decomposition_v1", ";",
                   guarded_clause, { guarded_clause } ;

guarded_clause   = "(", environment_id, "∧", prerequisite_id,
                     { "∧", prerequisite_id }, ")", "|->", operation_id, ";"
                 | condition_id, "!->", operation_id, ";"
                 | operation_id, "||->", case_id, "/", case_id, { "/", case_id }, ";"
                 | case_id, "->|", postcondition_id, ";" ;

environment_id   = [a-z][a-z0-9_]* ;
prerequisite_id  = [a-z][a-z0-9_]* ;
condition_id     = [a-z][a-z0-9_]* ;
operation_id     = [a-z][a-z0-9_]* ;
case_id          = [a-z][a-z0-9_]* ;
postcondition_id = [a-z][a-z0-9_]* ;
```

#### Semantic Side Conditions and Refusal Boundaries

- **Exact Unicode conjunction & flat pairing (§C04.18.5):** In standalone `.qgd` files, the conjunction symbol inside an enabling guard is **exact Unicode `∧` (U+2227)**, and `|->`, `!->`, `||->`, `->|`, `/` are atomic tokens with only surrounding whitespace ignored.
- **Complete pairing & totality:** No nested expressions, implicit precedence, `flow(...)` references, lifecycle operators, singleton `||->` cases (at least two `/`-separated `case_id`s are required), or unpaired symbols are admitted. Every `operation_id` must be guarded (`|->`, optionally inhibited by `!->`) and split into cases (`||->`), and every `case_id` must map to a distinct terminal (`->|`), with every ID bound in the supplied role/proposition map and verified under §C04.18.5 admission-direction lowering and positive-refutation checks.

#### Concrete Example (`.qgd`)

```text
guarded_decomposition_v1;
(ctx_env ∧ pool_live ∧ key_valid) |-> parse_entry;
pool_corrupt !-> parse_entry;
parse_entry ||-> case_hit / case_miss;
case_hit ->| post_borrowed_entry;
case_miss ->| post_null_unchanged;
```

**Implementation & Provenance:** `crates/qcore-frontend/src/guarded_decomposition.rs`, `crates/qcore-contract/src/guarded_decomposition.rs`; contract spec [§C04.18.5](QCORE_CONTRACT_SPEC_r0_3.md).

---

<a id="g05-guarded-source-explicit-guarded-source-embedding"></a>
### G05-guarded-source. Explicit guarded source embedding

#### Consumer and Selection Mechanism

`crates/qcore-contract/src/guarded_metadata_bridge.rs` embeds a complete guarded decomposition and its typed role/formula symbol table directly inside a `.qv` declaration's `requires` clauses. Every identifier starting with `resource_guarded_` is reserved exclusively to this total-or-reject reader (including unknown future version suffixes).

#### Consolidated Grammar

```ebnf
guarded_symbol        = "requires", guarded_symbol_body, ";" ;
guarded_symbol_body   = "resource_guarded_symbol_v1", "(", symbol_id, ",",
                        guarded_role, ",", guarded_formula,
                        { ",", graph_atom_id }, ")" ;

guarded_source        = "requires", guarded_source_body, ";" ;
guarded_source_body   = "resource_guarded_decomposition_v1", "(", entry_id, ",",
                        source_guarded_clause, { ",", source_guarded_clause }, ")" ;

guarded_role          = "environment" | "prerequisite" | "operation"
                      | "condition" | "case" | "postcondition" ;

guarded_formula       = formula ;   (* §G01p formula production, unquoted *)

source_guarded_clause = "(", environment_id, "&&", prerequisite_id,
                          { "&&", prerequisite_id }, ")", "|->", operation_id
                      | condition_id, "!->", operation_id
                      | operation_id, "||->", case_id, "/", case_id, { "/", case_id }
                      | case_id, "->|", postcondition_id ;
```

#### Semantic Side Conditions, Bounds, and `QF003` Prose Ban

- **Prose ban (`QF003`, card `SP6-GUARDED-SYMBOL-PROSE`):** Both `guarded_formula` and `source_guarded_clause` are **unquoted symbolic syntax**. Any `STRING` token anywhere inside `resource_guarded_symbol_v1` or `resource_guarded_decomposition_v1` (under any clause introducer) rejects the module at parse with **`QF003`**.
- **Formula typing:** `guarded_formula` is a headless §G01p `formula` whose free `#name` identifiers are registered §C04.18.9 / SP6 / SP7 terms typed in their rendered sorts (`obj`, `cls`, `fn`, `meth`, `res`, `run`, `#runs: predicate<fn, run>`, …). Its canonical §G01p printing (bounded at **4,096 bytes**) forms the formal binding's proposition.
- **Deterministic `.qgd` projection:** From `resource_guarded_decomposition_v1`, the loader synthesizes the standalone [G05-guarded](#g05-guarded-standalone-readable-guarded-decomposition) text (`guarded_decomposition_v1;` followed by one `;`-terminated line per `source_guarded_clause` with `&&` rendered as `∧`), validates it with the G05-guarded checker, and records its SHA-256 as the readable digest.
- **Size & cardinality bounds:** Each participating declaration requires **exactly one** `resource_guarded_decomposition_v1` clause and a complete `resource_guarded_symbol_v1` map covering all referenced symbols. Each source clause is bounded at **131,072 bytes**; document, identifier, symbol-count, and dependency bounds follow §C04.18.5. Sealed bridge v2 remains scoped to one resource module and declaration.

#### Concrete Example

```qv
quant proposition #atom_pool(#p: obj) = #live_pool(#p);
requires resource_guarded_symbol_v1(ctx_env, environment, #valid_ctx(#ctx));
requires resource_guarded_symbol_v1(pool_ok, prerequisite, #live_pool(#pool), atom_pool);
requires resource_guarded_symbol_v1( bad_state, condition, #corrupted(#pool) );
requires resource_guarded_symbol_v1(lookup_op, operation, #runs(#callee, #run));
requires resource_guarded_symbol_v1(hit_case, case, #found_item(#run));
requires resource_guarded_symbol_v1(miss_case, case, !#found_item(#run));
requires resource_guarded_symbol_v1(post_ok, postcondition, #preserved_custody(#pool));
requires resource_guarded_symbol_v1(post_nil, postcondition, #unchanged_pool(#pool));
requires resource_guarded_decomposition_v1(
    lookup_guard,
    (ctx_env && pool_ok) |-> lookup_op,
    bad_state !-> lookup_op,
    lookup_op ||-> hit_case / miss_case,
    hit_case ->| post_ok,
    miss_case ->| post_nil
);
```

**Implementation & Provenance:** `crates/qcore-contract/src/guarded_metadata_bridge.rs`, `crates/qcore-frontend/src/premise_formula.rs`; contract spec [§C04.18.6](QCORE_CONTRACT_SPEC_r0_3.md); card `SP6-GUARDED-SYMBOL-PROSE`.

---

<a id="g05a-contract-inclusion-and-inheritance"></a>
## G05a. Contract inclusion (`#include`) and inheritance (`contract inherit`)

### Consumer and Elaboration Discipline

`crates/qcore-frontend/src/contract_inclusion.rs` and `crates/qcore-contract/src/contract_inclusion.rs` implement a pre-consumer **elaboration layer** for `resource_contract_v1` modules, resolving cross-crate `#include` directives against the published provider load path (`providers/contract-load-path.json`) and expanding `contract inherit` declarations before [G05](#g05-resource-loader-v1) parses the module.

- **Fail-closed in the raw frontend:** G01's structural parser (`parse_source_program`) deliberately **refuses** both `#include` and `contract inherit` with a `Q000` parse error naming the contract loader, ensuring an unelaborated module can never reach any consumer.
- **Disallowed in `bounds_contract_v1`:** [G05b](#g05b-bounds-contract-v1) explicitly rejects `#include` and `contract inherit` (gateway rule [14j](agents/gateway.md#constraint-14j)).

### Consolidated Grammar

#### `.qv` Directive and Inheritance Grammar

```ebnf
inheriting_module = "module", "contract", ";",
                    { include_directive },
                    contract_item, { contract_item } ;

include_directive = "#", "include", INCLUDE_STRING, ";" ;
(* INCLUDE_STRING lexical content:
     include_target = crate_name, ":", module_relpath ;
     crate_name     = [a-z0-9], { [a-z0-9_-] } ;
     module_relpath = path_segment, { "/", path_segment }, ( ".qv" | ".qcore" ) ;
     path_segment   = [a-z0-9_-]+ ; *)

contract_item     = alias_declaration
                  | resource_decl_block
                  | inheriting_fn ;

inheriting_fn     = "extern", "fn", IDENT, "(", [ param_list ], ")",
                    "->", result_abi, "contract", "inherit", parent_ref, ";" ;

parent_ref        = [ IDENT, "::" ], IDENT ;       (* at most one namespace qualifier *)
```

#### Schema-2 Resource Contract Selection Manifest (JSON)

```ebnf
resource_manifest_v2 = "{",
                         '"schema_version":', "2", ",",
                         '"load_path":', JSON_REL_PATH, ",",
                         '"modules":', "[", manifest_entry, { ",", manifest_entry }, "]",
                       "}" ;

manifest_entry       = JSON_REL_PATH               (* relative file path, contains no ":" *)
                     | JSON_MODULE_REF ;           (* "crate_name:module_relpath" *)
```

`include` and `inherit` are **contextual** words (recognized only immediately after `#` in a contract module's header prefix and immediately after `contract` on an `extern fn`), reserving zero words.

### Semantic Side Conditions and `QI***` Refusal Boundaries

1. **Placement, dependency graph, and namespace derivation (`QI***`, §C15):**
   - Every `#include` directive must follow `module contract;` and precede all declarations (`QI***`).
   - The target `crate_name` must be either the includer's own provider crate or an explicit dependency in the load path, and `module_relpath` must belong to that crate's published resource contract set. Repeated includes and include cycles reject (`QI***`).
   - **Namespace mapping:** An `#include` string names the Cargo **package** (`quirin-provider-libc` or legacy `qcore-provider-libevent`), whereas `NS::parent` uses the derived **namespace** — stripping `quirin-provider-` or `qcore-provider-` and mapping `-` to `_` (e.g., `quirin-provider-libc` → `libc`, `qcore-provider-libevent` → `libevent`). If two packages strip to the same namespace remainder, loading refuses (`QI017`).

2. **Parent resolution, renaming, and post-elaboration equality:**
   - Unqualified `inherit NAME;` resolves to the unique declaration `NAME` in the current module; qualified `inherit NS::NAME;` resolves to the unique declaration `NAME` across the directly `#include`d modules of namespace `NS`.
   - The parent must be a concrete `extern fn … contract { … };` block (inheritance chains / cycles and trailing clauses after `inherit PARENT` reject).
   - Child and parent must agree in arity, parameter ABI types, and result ABI type, and positional substitution of child parameter names for parent parameter names must capture no identifier (`QI***`).
   - **Clause copying vs. stripping:** Elaboration blanks `#include` directives and splices the parent's renamed clause block into the child — preserving custody modes, kinds, literal premises, named premises, hypothesis/flow/lifecycle symbols (literal metadata, unrenamed), review markers, `resource_body_checked`, and loop-owner/thread-sync/lifecycle obligations, while **stripping** parent `resource_revision` markers (contract-level dropped, clause-level unwrapped) and `resource_library_version` bounds so the child carries its own pin/version posture. Finally, G05 loads both and checks that the child's loaded contract record equals the parent's.

### Concrete Example

```qv
module contract;
#include "quirin-provider-libc:libc/memory.qv";
#include "quirin-provider-libc:libc/string.qv";

extern fn flb_malloc(size: usize) -> ptr<mut, u8>
    contract inherit libc::malloc;

extern fn flb_calloc(nmemb: usize, size: usize) -> ptr<mut, u8>
    contract inherit libc::calloc;

extern fn flb_free(ptr: ptr<mut, u8>) -> unit
    contract inherit libc::free;

extern fn flb_strdup(s: ptr<const, u8>) -> ptr<mut, u8>
    contract inherit libc::strdup;
```

**Implementation & Provenance:** `crates/qcore-frontend/src/contract_inclusion.rs`, `crates/qcore-contract/src/contract_inclusion.rs`, `crates/qcore-contract/src/load_path_manifest.rs`; contract spec [§C15](QCORE_CONTRACT_SPEC_r0_3.md); cards `CONTRACT-INHERIT`, `CONTRACT-INHERIT-QCORE-PROVIDER-NAMING`, `CONTRACT-INHERIT-ADOPTION`.

---

<a id="g05b-bounds-contract-v1"></a>

## G05b. Bounds contract v1 (`c.bounds.v1`)

### Consumer and Vocabulary Separation

`bounds_contract_v1` is a standalone, total-or-reject contract loader (`crates/qcore-contract/src/bounds_contract.rs`) over the `G01` structural contract envelope, implementing contract specification [§C09.1](QCORE_CONTRACT_SPEC_r0_3.md#c091-reviewed-index-and-extent-contracts-planned). Its predicates (`bounds_*`) and the resource loader's predicates (`resource_*`) are disjoint, so each loader rejects modules belonging to the other. Bounds modules are published in a provider crate's `bounds_view` on the contract load path (never in its `resource_view`) and pinned using `C06.2`'s lock format in `bounds-contracts.lock`.

Unlike `G05a`, **no `#include` or `contract inherit` exists for bounds contracts**: the bounds loader rejects inclusion and inheritance directives, generated bounds views omit modules carrying them, and load-path selections naming one in a bounds view fail closed.

### Consolidated Grammar

```ebnf
bounds_module = "module", "contract", ";", bounds_fn, { bounds_fn } ;
bounds_fn     = "extern", "fn", IDENT, "(", [ parameter, { ",", parameter } ], ")",
                "->", result_abi, "contract", "{", bounds_clause,
                { bounds_clause }, "}", ";" ;

bounds_clause = "requires", "bounds_contract_v1", ";"
              | "requires", bounds_pre, ";"
              | "requires", "bounds_review_pending", "(", review_note, ")", ";"
              | "requires", "bounds_review_pending", "(", bounds_pre, ",", review_note, ")", ";"
              | "ensures", "normal", bounds_post, ";"
              | "ensures", "normal", "bounds_review_pending", "(", bounds_post, ",", review_note, ")", ";"
              | quant_check           (* G05c *)
              | null_deref            (* G05d *)
              | deref                 (* G05d *)
              | live                  (* G05d *)
              | bounds_hypothesis_metadata ;

bounds_pre    = "bounds_extent", "(", IDENT, ",", count, ",", unit, ")"
              | "bounds_index", "(", IDENT, ",", count, ")"
              | "bounds_global_extent", "(", IDENT, ",", DIGITS, ",", unit, ")"
              | member_clause         (* G05e, G05f *)
              | named_premise         (* G05f: only beside a member_clause *)
              | "bounds_revision", "(", "flexible", ")" ;   (* G05j *)

bounds_post   = "bounds_return_extent", "(", count, ",", unit, ")"
              | "bounds_preserves_extents", "(", ")"
              | "bounds_frame_reads", "(", IDENT, ")"
              | "bounds_frame_writes", "(", IDENT, ")"
              | "bounds_frame_moves", "(", IDENT, ")"
              | "bounds_return_derived", "(", IDENT, ",", count, ",", count, ",", unit, ")"
              | "nonnull_result", "(", ")"                          (* G05e *)
              | "nonnull_result_range", "(", signed, ",", signed, ")" (* G05e *)
              | "result_set", "(", set, ")"                         (* G05j *)
              | "written_set", "(", IDENT, [ "[", DIGITS, "]" ], ",", set, ")",
                "when", "#", "result", "==", signed ;               (* G05j *)

count         = IDENT | DIGITS ;
unit          = "elements" | "bytes" ;
```

`parameter` (including single-level pointers and the slot type `ptr<mut, ptr<mut, u8>>` representing C `T **`), `result_abi`, and `review_note` reuse `G05`'s productions, except that floating-point parameter/result types are excluded.

### Semantic Side Conditions and Closed-Frame Discipline

Every violation below rejects the **entire bounds module**:

1. **Selector and review markers:** Every declaration `MUST` contain exactly one `requires bounds_contract_v1;` selector, and only `requires`, `ensures normal`, and the admitted `quant` bounds clauses are accepted. Review-hold markers (`bounds_review_pending`) are either a single contract-level marker or clause-level wrappers around predicate calls—never both in one declaration, and never nested around another marker or the version selector.
2. **Extents and indices:**
   - `bounds_extent(p, count, unit)` names a pointer parameter `p`; `bounds_index(i, count)` names a scalar parameter `i`; `bounds_global_extent(obj, N, unit)` names an external C object `obj` (never a formal parameter) with a canonical unsigned decimal `DIGITS` (no sign, suffix, base, or leading zero).
   - A `count` written as `IDENT` `MUST` name a scalar parameter of the same declaration; `unit = elements` counts elements of the C pointee type declared in the audited C signature.
   - `bounds_return_extent(count, unit)` requires a pointer result type. At most one clause may govern each parameter, global object, return, or `bounds_preserves_extents()`.
3. **Closed pointer/slot frames and derived return bounds (`CG24`):**
   - `bounds_frame_reads(p)`, `bounds_frame_writes(p)`, and `bounds_frame_moves(p)` name a pointer or slot (`ptr<mut, ptr<mut, u8>>`) parameter.
   - **Closed-frame rule:** Any declaration that states at least one frame clause `MUST` state **exactly one** frame clause for **every** pointer and slot parameter in its signature. Because a slot parameter (`T **`) is permitted *only* in frame clauses (never in `bounds_extent` or derived-return relations), any declaration with a slot parameter `MUST` declare a complete closed frame.
   - `bounds_preserves_extents()` and `bounds_frame_moves(_)` are mutually exclusive.
   - `bounds_return_derived(base, offset, length, unit)` requires a pointer result, names a pointer parameter `base` that is **not** `bounds_frame_moves`, takes `count` expressions for `offset` and `length`, excludes `bounds_return_extent`, and admits at most one relation per `base` (multiple `bounds_return_derived` clauses over distinct bases form a multi-base relation).

### Concrete Example

```qv
module contract;

extern fn cio_meta_write(
    ch: ptr<mut, u8>,
    buf: ptr<const, u8>,
    len: usize
) -> i32
contract {
    requires bounds_contract_v1;
    requires bounds_frame_writes(ch);
    requires bounds_extent(buf, len, bytes);
    ensures normal bounds_frame_reads(buf);
};
```

**Implementation & Provenance:** `crates/qcore-contract/src/bounds_contract.rs`, `providers/qcore-provider-chunkio/contracts/chunkio/meta_len_bounds.qv`; contract spec [§C09.1](QCORE_CONTRACT_SPEC_r0_3.md#c091-reviewed-index-and-extent-contracts-planned), gateway rules [11e](agents/gateway.md#constraint-11e), [11s](agents/gateway.md#constraint-11s), [12r](agents/gateway.md#constraint-12r), [14j](agents/gateway.md#constraint-14j); cards `CG16-INDEX-AND-OFFSET-EXTENTS`, `CG16-BOUNDS-TRANSPORT-AND-CHUNKIO-NEIGHBOURS`, `CG24-ALIAS-EXTENT-FRAME-VOCABULARY`, `CHUNKIO-META-LEN-BOUNDS-CONTRACTS`.

---

<a id="g05c-quantified-index-checks"></a>

## G05c. Quantified index checks (`quant checks`)

### Consumer and Structural Gate

`quant checks` extends `G05b` to specify exact valid index intervals on a pointer parameter (`crates/qcore-contract/src/bounds_quant.rs`, semantic decision in `qcore_core::index_quant`; contract specification [§C09.2](QCORE_CONTRACT_SPEC_r0_3.md#c092-quantified-index-checks-quant-checks)). The structural parser (`G01`) retains `quant checks` as `SourceContractClauseKind::QuantCheck` (with no exit selector), and the frontend operator gate (`Q038`) permits `&&` and `||` inside a `quant checks` predicate and nowhere else in a `.qv` contract module outside formula/set clauses. `resource_*` consumers reject `QuantCheck` clauses.

### Consolidated Grammar

```ebnf
quant_check = "quant", "checks", IDENT, ",", "Idx", "#", IDENT, ":", q_or, ";" ;
q_or        = q_and, { "||", q_and } ;
q_and       = q_primary, { "&&", q_primary } ;
q_primary   = "(", q_or, ")" | q_side, q_relation, q_side ;
q_side      = "#", IDENT | q_term ;
q_term      = [ "-" ], q_operand, { ( "+" | "-" ), q_operand } ;
q_operand   = DIGITS | IDENT ;
q_relation  = "==" | "<" | ">" | "<=" | ">=" ;
```

### Semantic Side Conditions and Bounds

Every violation below rejects the **entire module** with its named diagnostic:

- **Subject and binder (`quant-check-subject`, `quant-check-binder`, `quant-check-shape`):** The subject `IDENT` `MUST` be a single-level pointer parameter (`ptr<mut, u8>` or `ptr<const, u8>`, never a scalar, slot, or undeclared name), and the binder `MUST` be `Idx #VAR`. At most one `quant checks` clause is allowed per formal, and never on a formal that also carries `bounds_extent` (`quant-check-with-extent`).
- **Variable and relational form (`quant-check-variable`, `quant-check-chained`, `quant-check-operator`):** Every `#` reference `MUST` name `VAR`, which stands alone on **exactly one side** of each comparison. Comparisons cannot chain, and `!=`, `!`, `*`, and all unlisted operators are refused.
- **Linear bound terms (`quant-check-term`, `quant-check-overflow`):** Each `q_operand` is a canonical unsigned `DIGITS` or a scalar parameter of the same declaration (never a C macro, enum constant, or global). Each `q_term` mentions at most one parameter with unit coefficient (`±1`), contains no parentheses, and `MUST NOT` overflow the integer model.
- **Size, boundedness, and non-emptiness (`quant-check-size`, `quant-check-unbounded`, `quant-check-empty`):** Predicates are bounded at **512 tokens**, **64 nested groups**, and **64 disjunctive-normal-form intervals**. Every disjunct `MUST` bound `#VAR` both below and above, and any purely constant predicate `MUST` admit at least one integer index.
- **Review markers:** Only contract-level `bounds_review_pending("note")` applies (clause-level wrappers wrap predicate calls, not `quant` clauses).

### Concrete Example

```qv
module contract;

extern fn read_window(buf: ptr<const, u8>, len: usize) -> i32
contract {
    requires bounds_contract_v1;
    quant checks buf, Idx #i: #i >= 0 && #i < len;
};
```

**Implementation & Provenance:** `crates/qcore-contract/src/bounds_quant.rs`, `crates/qcore-core/src/index_quant.rs`; contract spec [§C09.2](QCORE_CONTRACT_SPEC_r0_3.md#c092-quantified-index-checks-quant-checks), gateway rule [12l](agents/gateway.md#constraint-12l); card `QUANT-INDEX-CHECKS`.

---

<a id="g05d-null-dereference-checks"></a>

## G05d. Null-dereference, three-part dereference, and `quant live` clauses

### Consumer, QTT Grounding, and Two-Level Elaboration

`G05d` defines two related syntactic families using `quant`:

1. **Formal parameter dereference clauses in `bounds_contract_v1` modules** ([contract §C09.3](QCORE_CONTRACT_SPEC_r0_3.md#c093-null-dereference-the-property-cnullderefv1-and-quant-null-deref), [language §L07.4](QCORE_SPEC_r0_3.md#l07-usage-grades)):
   - `quant null-deref #B;` projects a `§L07` `relevant` (`[1, inf]`) dereference-demand grade onto the contract boundary: every terminating path dereferences `B`'s entry pointee before any `NULL` test, obligating callers to pass an existing (non-`NULL`) identity.
   - `quant deref #B;` is the **two-level shorthand** decomposing a pointer dereference into its three orthogonal projections (`crates/qcore-contract/src/deref_parts.rs`):

   | Surface clause | Elaborated canonical parts | Consuming property / backing |
   |---|---|---|
   | `quant deref #B;` | `quant null-deref #B;` · `quant checks B, Idx #i: #i == 0;` · `quant live #B;` | Split across null-deref, bounds, and custody-backed liveness |
   | `quant null-deref #B;` | Existence part alone (`QuantNullDeref`) | Read only by `c.null.deref.v1` (`for_bounds()` strips it) |
   | `quant checks B, Idx #i: P;` | Extent part (`QuantCheck`, `G05c`) | Read by `c.bounds.v1` |
   | `quant live #B;` | Liveness part alone (`QuantLive`) | Admitted on audit routes iff `resource_contract_v1` declares `resource_owned(B, ...)` |

2. **Constructed result states, return-code hand-offs, and member-path states in `resource_contract_v1` modules** ([contract §C04.19.4–§C04.19.6](QCORE_CONTRACT_SPEC_r0_3.md#c04194-a-constructors-result-state)):
   Uses the dedicated ASCII maps-to token **`|->`** (`Symbol::MapsTo`, chosen over `->` and `>->` to match mathematical `↦` without colliding with return arrows, `|>`, or `||`) and `#`-prefixed state keywords (`#absent`, `#owner`, `#callback`, `#moved`, `#kept`).

### Consolidated Grammar

```ebnf
(* 1. Formal parameter clauses inside bounds_contract_v1 (G05b) *)
null_deref = "quant", "null", "-", "deref", "#", IDENT, ";" ;
deref      = "quant", "deref", "#", IDENT, ";" ;
live       = "quant", "live", "#", IDENT, ";" ;

(* Neutral bounds-v1 hypothesis metadata attachment *)
bounds_hypothesis_metadata = "requires", ( hypothesis_atom | hypothesis_decomposition ), ";" ;

(* 2. Constructed state, hand-off, and member-path clauses inside resource_contract_v1 (G05) *)
MAPS_TO             = "|->" ;
result_state_clause = "quant", "live", "#", "result",
                      [ ":", IDENT, ".", member_path, MAPS_TO, ( "#", "absent" | IDENT ) ], ";" ;
handoff_clause      = "quant", "live", "#", IDENT, ":", "#", "owner", MAPS_TO, "#", "callback",
                      "when", "#", "result", ( "==" | "!=" ), ( "@", IDENT | "#", IDENT ), ";" ;
member_path_clause  = "quant", "live", "#", IDENT, ":", IDENT, ".", member_path, MAPS_TO,
                      "{", state_item, { ",", state_item }, "}",
                      [ "when", "#", "result", ( "==" | "!=" ), ( "@", IDENT | "#", IDENT ) ], ";" ;
state_item          = "#", ( "moved" | "kept" | "released" | "absent" ) ;
```

### Semantic Side Conditions

- **Bounds-module formal clauses (`quant-null-deref-*`, `quant-deref-*`, `quant-live-*`):**
  - Target `#IDENT` `MUST` be a single-level pointer parameter (`ptr<mut, u8>` or `ptr<const, u8>`), at most one `null-deref`, `deref`, and `live` per formal; unknown `quant WORD #IDENT;` forms fail (`quant-shorthand-unknown`).
  - Overlapping explicit and `deref`-elaborated parts for the same formal merge idempotently when identical (ignoring bound index-variable renaming) and reject on disagreement or alongside `bounds_extent` (`quant-deref-conflict`).
  - `text_sha256` pins exact written text, whereas `canonical_sha256` hashes signature + sorted canonical parts.
  - On audit routes, `quant live #B;` requires an in-force `resource_owned` declaration on `B` (`quant-live-unbacked`, `quant-live-contradicts-custody`).
- **Resource-module `|->` clauses (`quant-live-result-shape`, `quant-live-maps-to`, `quant-live-absent`, `quant-live-value`):**
  - Bare `quant live #P;` on a formal parameter is rejected in `resource_contract_v1` (formal liveness belongs to bounds modules); bare `#result` restates `resource_return_owned(nonnull)`.
  - Left of `|->` in `#result` state `MUST` be `TAG.FIELD` (`->` is rejected naming `|->`); right of `|->` `MUST` be `#absent` (bare `absent`, `NULL`, `nullptr`, literals, parameters, and `result` all reject) or a single C function designator `f`, elaborating to `quant live #result;` plus `resource_callback_member_registered(result, TAG.FIELD, #absent | f)`.
  - `#P: #owner |-> #callback when #result ...` states a conditional callback custody hand-off backed by `resource_owned(P, nonnull)`; `#P: TAG.path |-> {...}` states post-call member-path dispositions (`G05`).
- **Bounds hypothesis metadata:** Validates against an existing named bounds premise on the same declaration without `hypothesis_all` or solver discharge.

### Concrete Examples

```qv
// Bounds/null-deref module:
module contract;
extern fn parse_header(hdr: ptr<const, u8>) -> i32
contract {
    requires bounds_contract_v1;
    quant deref #hdr;
};
```

```qv
// Resource-v1 constructor & callback hand-off clauses:
quant live #result: rd_kafka_conf_s.log_cb |-> rd_kafka_log_print;
quant live #rko: #owner |-> #callback when #result != @RD_KAFKA_OP_RES_PASS;
```

**Implementation & Provenance:** `crates/qcore-contract/src/{bounds_contract,deref_parts,contract_constructed_state,contract_member_path}.rs`; `QCore.{NullDerefGrade,DerefDecomposition,AppRkCallbacks}`; cards `NULL-SAFETY-PROPERTY`, `NULL-DEREF-QTT-GROUNDING`, `OP-CONSTRUCTOR-DECLARATIONS`, `OP-APP-RK-CALLBACKS-VIA-POLL-CB`, `SUPPLIED-OWNED-RETURN-CODE-HANDOFF`, `OP-CALLBACK-MEMBER-PATH-RELEASE`.

---

<a id="g05e-null-property-member-and-result-declarations"></a>

## G05e. Null-property member, result, and result-range declarations

### Consumer and Relation to Resource Contracts

`G05e` extends `bounds_contract_v1` (`G05b`) with member non-nullness, pointer-result non-nullness, and scalar result-range postconditions consumed **exclusively** by the `c.null.deref.v1` analyzer (`BoundsContracts::for_bounds()` strips them from bounds identity, and `resource_*` loaders reject them). When a function has both a `resource_contract_v1` declaration and a null-property declaration in its crate's `bounds_view`, the pair forms a checked `null_deref_beside` view relation.

### Consolidated Grammar

```ebnf
bounds_pre  = … | "nonnull_member", "(", IDENT, ",", IDENT, ")" ;
bounds_post = … | "nonnull_result", "(", ")"
                | "nonnull_result_range", "(", signed, ",", signed, ")" ;
signed      = [ "-" ], DIGITS ;
```

### Semantic Side Conditions

- **`nonnull_member(TAG, MEMBER)` (`nonnull-member-shape`):** First `IDENT` is a C struct tag (never a formal parameter name of the declaration) and second `IDENT` is a member name; at most one plain clause per `(TAG, MEMBER)` per declaration (`requires` only; unified with `G05f` qualifiers below). Legacy retired names `null_member_nonnull` and `null_result_nonnull` have no aliases and reject.
- **`nonnull_result()` (`nonnull-result-pointer`):** `ensures normal` postcondition on a pointer-returning function, at most one per declaration.
- **`nonnull_result_range(LO, HI)` (`nonnull-result-range-*`):** `ensures normal` postcondition on a scalar-returning function; `LO` and `HI` are canonical signed 64-bit decimal integers (no leading zeros, suffixes, bases, or `-0`) satisfying `LO <= HI`, at most one per declaration (e.g., `cfl_list_size` returning `[0, ...]` in `cfl/null_deref.qv`).

**Implementation & Provenance:** `crates/qcore-contract/src/bounds_contract.rs`; contract spec [§C09.3](QCORE_CONTRACT_SPEC_r0_3.md#c093-null-dereference-the-property-cnullderefv1-and-quant-null-deref), gateway rules [14t](agents/gateway.md#constraint-14t), [14u](agents/gateway.md#constraint-14u), [15q](agents/gateway.md#constraint-15q); cards `NULL-CANDIDATE-TRIAGE`, `NULL-MEMBER-INVARIANT-SCOPE`, `NULL-TRIAGE-UNRESOLVED-PATHS`.

---

<a id="g05f-scoped-member-invariants"></a>

## G05f. Scoped member invariants (`where quant #scope [[stable | scope]]`)

### Consumer and Unified Qualifier Surface

`G05f` unifies plain constructor-hosted member non-nullness (`14t`), type-wide stable member invariants (`14x`), and function-scoped parameter member preconditions (`14z`) under a single `nonnull_member(...)` syntax with an explicit `where quant #scope [[...]]` qualifier (`QCore.NullMemberQualifier`), and permits a `G05` named premise (`resource_premise`) alongside any `member_clause`.

### Consolidated Grammar

```ebnf
bounds_pre       = … | member_clause | named_premise ;
member_clause    = "nonnull_member", "(", IDENT, ",", IDENT, ")"
                 | "nonnull_member", "(", IDENT, ",", IDENT, ")", member_qualifier
                 | "nonnull_member", "(", IDENT, ",", IDENT, ",", IDENT, ")", member_qualifier ;
member_qualifier = "where", "quant", "#", "scope", "[", "[", ( "stable" | "scope" ), "]", "]" ;
```

### Semantic Side Conditions

1. **Arity-to-qualifier matching (`nonnull-member-*`):**
   - Two arguments `(TAG, MEMBER)` take either no qualifier (`BoundsClause::NonnullMember`) or `where quant #scope [[stable]]` (`BoundsClause::NonnullMemberStable`), both hosted as type-level declarations on the struct's constructor.
   - Three arguments `(TAG, MEMBER, PARAM)` `MUST` carry `where quant #scope [[scope]]` (`BoundsClause::NonnullMemberScope`), and `[[scope]]` `MUST` carry three arguments where `PARAM` names a pointer formal of the host function (`nonnull-member-parameter-needs-scope`, `nonnull-member-scope-needs-parameter`, `nonnull-member-scope-parameter`).
2. **Placement, uniqueness, and retired spellings:** Admitted only under `requires` (`nonnull-member-requires`), at most one plain and one `[[stable]]` per `(TAG, MEMBER)` and one `[[scope]]` per `(PARAM, TAG, MEMBER)` per declaration. The retired surface names `nonnull_member_stable` and `nonnull_member_scope` reject everywhere (`nonnull-member-retired-spelling`) while remaining reserved words (note: internal audit assumption/check labels retain `nonnull_member_stable` / `nonnull_member_scope(...)` as non-surface identifiers).
3. **Premised member facts (`premise-beside-member`):** A `named_premise` (`resource_premise(...)`) is admitted in a bounds declaration **only** when that declaration also contains at least one `member_clause` (plain, `[[stable]]`, or `[[scope]]`); writing a premise alone or beside only `quant null-deref` / `bounds_*` clauses rejects the module.

### Concrete Example

```qv
module contract;

extern fn rd_kafka_broker_op(rkb: ptr<mut, u8>) -> unit
contract {
    requires bounds_contract_v1;
    requires nonnull_member(rd_kafka_broker_s, rkb_rk, rkb) where quant #scope [[scope]];
};
```

**Implementation & Provenance:** `crates/qcore-contract/src/bounds_contract.rs`; `QCore.NullMemberQualifier`; contract spec [§C09.3](QCORE_CONTRACT_SPEC_r0_3.md#c093-null-dereference-the-property-cnullderefv1-and-quant-null-deref), gateway rules [14t](agents/gateway.md#constraint-14t), [14x](agents/gateway.md#constraint-14x)–[14z](agents/gateway.md#constraint-14z); cards `NULL-MEMBER-INVARIANT-SCOPE`, `LIBRDKAFKA-PLUGIN-PREMISE-REMAINDER`, `NONNULL-MEMBER-UNIFIED-QUALIFIER`.

---

<a id="g05g-outer-contract-v1"></a>

## G05g. Outer contract v1 (`outer_contract_v1`)

### Consumer and Runtime-Projection Purpose

Modules whose declarations select `requires outer_contract_v1;` are loaded by `crates/qcore-contract/src/outer_contract.rs` to state closed-world envelope assumptions about dynamically loaded code (`dlopen` plugins) in the separate **runtime projection** ([contract §C04.20](QCORE_CONTRACT_SPEC_r0_3.md#c0420-outer-contracts-code-loaded-at-runtime), gateway rule [31](agents/gateway.md#constraint-31)). Clauses beginning `quant #outer` parse as `SourceContractClauseKind::QuantOuter` and are rejected by every other consumer.

### Consolidated Grammar

```ebnf
outer_declaration = "extern", "fn", IDENT, "(", parameters, ")", "->", type,
                    "contract", "{", outer_clause, { outer_clause }, "}", ";" ;
outer_clause      = "requires", "outer_contract_v1", ";"
                  | "quant", "#", "outer", outer_form, ";" ;
outer_form        = "[[", "contract", "]]", NAME, [ ",", premise_formula ]
                  | "where", "#", IDENT, "in", "#", "loaded", "[[", "runtime", "]]",
                    [ "when", "#", "result", ( "==" | "!=" ), [ "-" ], INTEGER ]
                  | "#", "loaded", ":", envelope ;
envelope          = IDENT, ".", IDENT, { ".", IDENT }, MAPS_TO, "#", ( "kept" | "registered" )
                  | "#", "premise", IDENT ;
```

`NAME` is a lowercase identifier `[a-z][a-z0-9_]*`, and `premise_formula` is `G05-formula`'s symbolic formula (`qcore_frontend::premise_formula::outer_header`; the operator gate `Q038` admits `&&` and `|` inside valid formula statements).

### Semantic Side Conditions

Every violation below rejects the **entire selection**:

- **Selector and symbolic header (`outer-clause-foreign`, `outer-contract-*`, `QF004`):** Each declaration `MUST` carry `requires outer_contract_v1;`, no other `requires`/`ensures` clauses, and **exactly one** `[[contract]] NAME [, formula]` header. Any header statement that is or contains a string literal rejects with **`QF004`** on every route; ill-sorted formulas reject at parse and loader (`outer-statement`).
- **Runtime binder conditions (`outer-runtime-*`, `outer-binder-*`, `outer-guard-result`):** At least one `where #P in #loaded [[runtime]]` clause is required, each binding a distinct pointer parameter `P`; an optional `when #result ==/!= INT` guard requires an integer return type.
- **Defining vs. citing declarations (`outer-envelope-*`, `outer-contract-defined-twice`, `outer-contract-undefined`):** `#loaded`, `#kept`, `#registered`, and `#premise` require their `#` prefix; `#loaded: TAG.field... |-> #kept|#registered` and `#loaded: #premise ID` envelope clauses may appear **only on the defining declaration** (which carries the `formula`), with unique keys, exactly one definition per `NAME` across the selection, and no citation of an undefined `NAME`. Pins (`quirin-audit pin --outer`) bind `text_sha256` to the dynamic loader's C body.

**Implementation & Provenance:** `crates/qcore-contract/src/outer_contract.rs`, `crates/qcore-frontend/src/premise_formula.rs`; contract spec [§C04.20](QCORE_CONTRACT_SPEC_r0_3.md#c0420-outer-contracts-code-loaded-at-runtime), gateway rule [31](agents/gateway.md#constraint-31); cards `OUTER-CONTRACT`, `SP8-OUTER-CONTRACT-SYMBOLIC-STATEMENTS`.

---

<a id="g05h-local-condition-and-registered-lifetime-clauses"></a>

## G05h. Local condition and registered-lifetime clauses

### Consumer and Scope

`G05h` extends `resource_contract_v1` (`G05`) with guarded `[[local]]` prerequisite blocks and cross-function `quant registered` lifetime links (`crates/qcore-contract/src/local_condition.rs`; contract specification [§C04.21](QCORE_CONTRACT_SPEC_r0_3.md)). These local contexts remain completely distinct from `G05g`'s dynamic-loading `#outer` scenarios.

### Consolidated Grammar

```ebnf
condition_binding  = "quant", "#", IDENT, "=", "(", "#", IDENT, ".",
                     member_path, "!=", "0", ")", ";" ;
condition_block    = "quant", [ "not" ], "#", IDENT, "in", "{",
                     local_clause, { local_clause }, "}", ":", "[[", "local", "]]", ";" ;
local_clause       = "requires", local_prerequisite, ";" | registered_clause ;
local_prerequisite = named_premise
                   | "resource_loop_owner", "(", IDENT, ",", ( "read" | "mutate" ), ")"
                   | "resource_thread_sync", "(", IDENT, [ ",", IDENT ], ",",
                     ( "cond_wait" | "cond_timedwait" | "cond_signal"
                     | "cond_init" | "cond_destroy" | "join" ), ")"
                   | "resource_lifecycle", "(", IDENT, ",", ( "start" | "stop" | "destroy" ), ")" ;
registered_clause  = "quant", "registered", "#", IDENT, ":", IDENT, ".",
                     member_path, MAPS_TO, IDENT, ".", member_path, ";" ;
```

### Semantic Side Conditions

- **Condition root and polarity:** The `#IDENT` inside `(#ROOT.member_path != 0)` `MUST` name a depth-one pointer formal of the host declaration whose final member Clang proves to be a non-`volatile` integer. Condition names are unique, every declared binding `MUST` be referenced, and at most one `[[local]]` block per polarity (`#c` and `not #c`) is admitted.
- **Registered paths and bounds:** Both sides of `|->` in `quant registered` `MUST` name the same record tag with no free/implicit record binders escaping. Nested blocks, empty blocks, unknown attributes, and unlisted inner clauses reject; each local clause is bounded at **64 KiB** and each `member_path` at **64 segments**. Currently, only the exact guarded named-entry-premise subset discharges from source; other active local obligations stay `Unknown`.

**Implementation & Provenance:** `crates/qcore-contract/src/local_condition.rs`; contract spec `§C04.21`; card `FB-EVENT-LOOP-CROSS-FUNCTION-LIFETIME` (S3).

---

<a id="g05i-declared-cell-protocols"></a>

## G05i. Declared cell protocols (`quant cell #B: ...`)

### Consumer and Purpose

`G05i` extends `resource_contract_v1` (`SourceContractClauseKind::QuantCell`, `crates/qcore-contract/src/local_condition.rs`) to declare record-level descriptor-cell protocols—such as pipe/socketpair channel ends stored in struct fields or fixed-index arrays and guarded either by a record-wide flag or by per-end descriptor sentinels ([contract §C04.6.3](QCORE_CONTRACT_SPEC_r0_3.md#c0463-declared-cell-protocols), gateway rules [38c](agents/gateway.md#constraint-38c), [38f](agents/gateway.md#constraint-38f)).

### Consolidated Grammar

```ebnf
cell_clause  = "quant", "cell", "#", IDENT, ":", IDENT, ".", cell_path, MAPS_TO,
               ( "#", "fresh", "(", "descriptor", ")" | "#", "same", "(", cell_path, ")" ),
               "when", "#", IDENT, ".", cell_path, cell_guard,
               [ "live_set", "(", set, ")" ], [ "absent_set", "(", set, ")" ], ";" ;
cell_path    = cell_segment, { ".", cell_segment } ;
cell_segment = IDENT, [ "[", DECIMAL, "]" ] ;      (* canonical decimal: no leading zero *)
cell_guard   = "!=", "-", "1" | ">", "0" | "==", "1" ;
```

`set` reuses `G05j`'s integer set grammar (`|`, `&`, and `^` are admitted by the `Q038` operator gate inside suffixed `quant cell` clauses).

### Semantic Side Conditions

Every violation rejects the **entire module** (`quant-cell-*`):

1. **Binder and path limits:** One record uses a single non-reserved `#IDENT` binder (`untouched`, `fresh`, `same`, etc. are reserved `#` words), and each `cell_path` has at most **64 segments**.
2. **Record-level vs. per-end sentinel guards:** Clauses on a record group by their `when #B.path OP V` guard. A record may state **at most one non-per-end guard** (`== 1`, whose path `MUST NOT` be any cell of the record) alongside any number of **per-end sentinels** (`!= -1` or `> 0` guarding `#B.END` over that exact `#fresh(descriptor)` cell and its `#same(...)` aliases; `>= 0` is not admitted).
3. **Cell uniqueness and `#same` targets:** Each cell path appears at most once in a record; every guard group has at least one `#fresh(descriptor)` cell; every `#same(target)` points to a `#fresh` cell of the **same** guard group.
4. **Optional `live_set` / `absent_set` suffixes (`quant-cell-set`, `quant-cell-live-set`, `quant-cell-absent-set`):** Admitted *only* on a per-end sentinel's own `#fresh` clause, in `live_set` then `absent_set` order (at most one of each). `live_set` and `absent_set` `MUST` be disjoint, and `absent_set` `MUST` be disjoint from the guard's live edge.
5. **Cross-declaration agreement:** A declaration carrying `quant cell` becomes a withheld lifetime host; `ResourceContracts::cell_protocols` retains a record's protocols only when every declaration mentioning that record states the identical protocol set.

### Concrete Example

```qv
quant cell #ins: flb_input_instance.ch_manager[0] |-> #fresh(descriptor)
    when #ins.ch_manager[0] > 0 live_set([1, MAX]) absent_set([-1, 0]);
```

**Implementation & Provenance:** `crates/qcore-contract/src/local_condition.rs`, `providers/quirin-provider-fluent-bit/contracts/fluent_bit/teardown_destroy.qv`; contract spec [§C04.6.3](QCORE_CONTRACT_SPEC_r0_3.md#c0463-declared-cell-protocols), gateway rules `38c`, `38f`; cards `MK-EVENT-CELL-CROSS-FUNCTION`, `CELL-PROTOCOL-PER-END-GUARDS`, `CELL-RESULT-SET-ALIGNMENT`.

---

<a id="g05j-c-library-result-sets"></a>

## G05j. C library result sets and slot `written_set` clauses

### Consumer and Operator Precedence

`G05j` extends `bounds_contract_v1` (`G05b`) with integer `result_set(...)` and conditional output-slot `written_set(...)` postconditions under `requires bounds_revision(flexible);` ([contract §C09.4](QCORE_CONTRACT_SPEC_r0_3.md#c094-declared-c-library-result-sets), gateway rules [38e](agents/gateway.md#constraint-38e), [38f](agents/gateway.md#constraint-38f)). Only the null-dereference and cell/resource result-derivation passes read these clauses (`BoundsContracts::for_bounds()` strips them, and `resource_*` loaders reject the module).

### Consolidated Grammar

```ebnf
bounds_pre  = … | "bounds_revision", "(", "flexible", ")" ;
bounds_post = … | "result_set", "(", set, ")"
                | "written_set", "(", IDENT, [ "[", DIGITS, "]" ], ",", set, ")",
                  "when", "#", "result", "==", signed ;

set         = xor_set, { "|", xor_set } ;          (* set union, loosest *)
xor_set     = and_set, { "^", xor_set } ;          (* symmetric difference *)
and_set     = atom, { "&", atom } ;                (* set intersection, tightest *)
atom        = bound | "[", bound, ",", bound, "]" | "(", set, ")" ;
bound       = signed | "MIN" | "MAX" ;
signed      = [ "-" ], DIGITS ;                    (* canonical: no leading zero, never -0 *)
```

### Semantic Side Conditions

- **Set algebra and operator gate (`result-set-*`):** `|`, `^`, and `&` are left-associative with standard C bitwise precedence and admitted by the `Q038` operator gate inside `result_set`/`written_set`/`live_set`/`absent_set` spans (`||`, `&&`, `[|`, `|]`, and non-ASCII tokens are rejected). The host function `MUST` return an integer type (`result-set-integer`), every literal bound `MUST` fit that return/element type (`result-set-outside-type`), every interval `MUST` satisfy `lo <= hi`, and every sub-expression and total set `MUST` be non-empty (`result-set-empty`).
- **Flexible revision scope (`bounds-revision-*`, `result-set-needs-flexible`):** `bounds_revision(flexible)` appears at most once, is mandatory whenever `result_set` is present, and is permitted *only* on an unheld declaration whose clauses consist solely of its single `ensures normal result_set(...)` and zero or more `ensures normal written_set(...)` slot clauses.
- **Conditional slot `written_set` (`written-set-*`):** `IDENT` names a pointer parameter (with optional constant element index `[DIGITS]`, at most one clause per slot element), stating the integer set written into that slot when `#result == signed`.

### Concrete Example

```qv
module contract;

extern fn pipe(pipefd: ptr<mut, u8>) -> i32
contract {
    requires bounds_contract_v1;
    requires bounds_revision(flexible);
    ensures normal result_set([-1, 0]);
    ensures normal written_set(pipefd[0], [0, MAX]) when #result == 0;
    ensures normal written_set(pipefd[1], [0, MAX]) when #result == 0;
};
```

**Implementation & Provenance:** `crates/qcore-contract/src/bounds_contract.rs`, `crates/qcore-core/src/result_set.rs`, `providers/quirin-provider-libc/contracts/libc/result_sets.qv`; contract spec [§C09.4](QCORE_CONTRACT_SPEC_r0_3.md#c094-declared-c-library-result-sets), gateway rules `38e`, `38f`; cards `CELL-GUARD-RESULT-RANGE-CHECKED`, `CELL-RESULT-SET-ALIGNMENT`.

---

<a id="g05k-callee-frames-over-a-declared-cell-protocol"></a>

## G05k. Callee frames over a declared cell protocol (`quant cell CALLEE: ...`)

### Consumer and Disambiguation from `G05i`

`G05k` adds a second `quant cell` form to `resource_contract_v1` (`crates/qcore-contract/src/local_condition.rs`; [contract §C04.6.4](QCORE_CONTRACT_SPEC_r0_3.md#c0464-callee-frames-over-a-declared-cell-protocol), gateway rule [38g](agents/gateway.md#constraint-38g)) declaring that an external helper (such as `mk_list_del`) leaves all descriptor cells of a protocol-governed record untouched during a release window. It is distinguished from `G05i` by opening with a bare `IDENT` (the callee name) rather than a `#IDENT` binder.

### Consolidated Grammar

```ebnf
frame_clause = "quant", "cell", IDENT, ":", IDENT, MAPS_TO, "#", "untouched", ";" ;
```

### Semantic Side Conditions

- **Shape and co-location (`quant-frame-shape`, `quant-frame-record`, `quant-frame-duplicate`):** First `IDENT` is the callee function name, second `IDENT` is the C struct tag, and the right-hand side is `#untouched` (`untouched` is reserved against `G05i` binders). The **same declaration** `MUST` also declare a `G05i` cell protocol for that struct tag, and each `(callee, record)` pair may appear at most once per declaration.
- **Cross-declaration agreement:** `ResourceContracts::cell_frames` retains frames for a record only if `cell_protocols` retains that record's protocol *and* every declaration stating that record's protocol declares the identical set of frame callees. The clause alters nothing about its host function's own call summary.

### Concrete Example

```qv
quant cell mk_list_del: flb_input_instance |-> #untouched;
```

**Implementation & Provenance:** `crates/qcore-contract/src/local_condition.rs`, `providers/quirin-provider-fluent-bit/contracts/fluent_bit/teardown_destroy.qv`; contract spec [§C04.6.4](QCORE_CONTRACT_SPEC_r0_3.md#c0464-callee-frames-over-a-declared-cell-protocol), gateway rule `38g`; card `CELL-PROTOCOL-RELEASE-WINDOW-FRAMES`.

---

<a id="g05l-list-finiteness-premises"></a>

## G05l. List-finiteness premises (`quant list TAG.MEMBER |-> #finite;`)

### Consumer and Standalone File Boundary

Unlike `G05`–`G05k`, a list-finiteness file is **not** a `module contract;` bundle member: it is a standalone line-oriented premise file selected via `QCORE_LIST_FINITENESS_PREMISES` and parsed total-or-reject by `crates/qcore-contract/src/list_finiteness.rs` (`parse_list_finiteness_premises`; [contract §C04.22](QCORE_CONTRACT_SPEC_r0_3.md#c0422-the-list-finiteness-premise), gateway rule [48](agents/gateway.md#constraint-48)). It is excluded from provider packages and resource-bundle digests.

### Consolidated Grammar

```ebnf
premise_file = { line } ;
line         = blank | "//", { any } | premise ;
premise      = "quant", "list", IDENT, ".", IDENT, MAPS_TO, "#", "finite", ";" ;
```

### Semantic Side Conditions

- Each `premise` `MUST` appear on a single line, naming `TAG.MEMBER` (`struct TAG` and its intrusive list-head field) at most once across the file.
- Any malformed non-comment line or duplicate `TAG.MEMBER` rejects the **entire file**, leaving zero premises in force so every list-length loop refuses with `premise-not-declared` (recording the loader failure in its detail).

### Concrete Example

```qv
// contract/profiles/fluent-bit/list-finiteness.qv
quant list flb_config.inputs |-> #finite;
quant list flb_config.outputs |-> #finite;
```

**Implementation & Provenance:** `crates/qcore-contract/src/list_finiteness.rs`, `contract/profiles/fluent-bit/list-finiteness.qv`; contract spec [§C04.22](QCORE_CONTRACT_SPEC_r0_3.md#c0422-the-list-finiteness-premise), gateway rule `48`; card `W45-FB-LIST-FINITENESS-PREMISE`.

---

<a id="g05m-contract-linkage-v1"></a>

## G05m. Contract linkage v1 (`linkage_contract_v1`)

### Consumer and Vocabulary

Card CONNECT-ADOPT-DECLARED-DEPENDENCIES (user decisions of 2026-11-26, "Every reliance" and "Accept all"; contract specification [§C15.2](QCORE_CONTRACT_SPEC_r0_3.md#c152-declared-contract-dependencies)). A resource v1 declaration states each other crate's declaration a `Safe` or `Unsafe` verdict using it can rest on as a **link**. The resource v1 loader (`crates/qcore-contract/src/contract_loader.rs`) reads links beside the per-declaration marker `requires linkage_contract_v1;`; every other consumer rejects them. Version 1 fixes the three forms below and their elaboration to one C15.2 dependency each; a change is `linkage_contract_v2`, never a widening of v1. (`G05l` was already allocated, so the linkage vocabulary is `G05m`.)

### Consolidated Grammar

```ebnf
linkage_clause = link_clause | premises_block ;             (* inside contract { ... } *)
link_clause    = "quant", "#", "link", link_target, ";" ;
link_target    = NAMESPACE, "::", NAME                      (* the target crate's derived namespace, C15 *)
               | NAME ;                                      (* only when a link import names it *)
premises_block = "premises", "{", premises_entry, { premises_entry }, "}", ";" ;
premises_entry = link_block | resource_clause ;             (* any clause the contract admits, except premises_block *)
link_block     = "link", "{", link_clause, { link_clause }, "}", ";" ;

link_import    = "#", "include", NAMESPACE, "::", NAME, "as", "quant", "#", "link", ";" ;
                                                            (* module level, among the #include directives *)
NAMESPACE      = /[a-z][a-z0-9_]*/ ;
NAME           = /[A-Za-z_][A-Za-z0-9_]*/ ;
```

### Semantic Side Conditions

Every violation rejects the **entire module** (total-or-reject):

- **Marker (`QL001`, `QL002`):** a declaration with a link states `requires linkage_contract_v1;` exactly once, and a declaration stating the marker has at least one link (`cx_link_needs_marker`, `cx_marker_needs_link`).
- **Target (`QL003`, `QL004`):** a link names one declaration as `NAMESPACE::NAME`; a bare `NAME` only when a module-level link import names it (the inclusion elaborator writes it qualified before any reader runs, so the digest of a bare and a qualified link is one); no target is linked twice by one declaration.
- **Blocks:** `premises { ... };` stands only at a contract's top level and groups clauses without changing their reading (its clauses are read in place; no existing premise is moved into one); `link { ... };` stands only inside `premises` and holds only links; a block is not empty and ends `};`, as a `[[local]]` condition block does. A trailing `.` is no terminator: inside a balanced clause region `.` builds member paths.
- **Reserved word:** `#link` directly after `quant` at the start of a clause is a link; `quant #link = (...)` and `quant [not] #link in { ... }` are refused (no condition is named `link`). `#link` as a binder and `link` as a sort inside a formula are unaffected (constraint 14: no reserved word elsewhere).
- **Placement:** no link inside a `[[local]]` block (links are unconditional); a `contract inherit` declaration carries no clause (`QI002`); its directive is itself its link.
- **Module-level imports (`QL006`):** each is used by a bare link of the module, names a function the module does not declare, and is not repeated; it links nothing by itself (only the per-declaration forms are dependencies, and only they are what the enforcement check counts).
- **Load path (`QL007`–`QL009`, `QI014`):** `providers/contracts.py sync` (or `quirin-audit plugin views --write`) records each link, and each cross-crate inherit directive, in the load-path manifest's `contract_dependencies` with the target crate (the load-path crate deriving `NAMESPACE`), its version as `since` and the target declaration's lock digest as `sha256`, and `vocabulary: "linkage_contract_v1"`. A namespace no load-path crate derives, a target its crate's lock does not record (`QL007`), a libc/POSIX name under another crate (`QL008`), a missing identity (`QL009`), an entry that differs from the module texts and a cycle (`QI014`) are refused.

### Concrete Example

```qv
module contract;
#include libc::free as quant #link;

extern fn acme_new(size: usize) -> ptr<mut, u8>
contract {
    requires resource_contract_v1;
    requires linkage_contract_v1;
    premises {
        link { quant #link libc::malloc; };
        requires resource_revision(flexible);
    };
    ensures normal resource_borrowed(size);
    ensures normal resource_return_owned(nullable);
};

extern fn acme_free(p: ptr<mut, u8>) -> unit
contract {
    requires resource_contract_v1;
    requires linkage_contract_v1;
    quant #link free;
    ensures normal resource_released(p, nullable);
    ensures normal resource_return_none();
};
```

**Implementation & Provenance:** `crates/qcore-frontend/src/frontend.rs` (`QuantLink`, `premises`/`link` blocks), `crates/qcore-frontend/src/contract_inclusion.rs` (link imports), `crates/qcore-contract/src/contract_dependency.rs` (`QL` codes, `scan_module`), `crates/qcore-contract/src/load_path_manifest.rs`, `providers/contracts.py` (`contract_dependencies`, `check_linkage`); Lean `QCore.MeasureConnect` (`elaboration_is_the_dependency_set`); card `CONNECT-ADOPT-DECLARED-DEPENDENCIES`.

---

# Part VI — Observation Policies, Service Configuration, Existing-C Subsets, and Extended BNF

<a id="g06-observation-predicate-grammar"></a>

## G06. Observation predicate grammar (`contract.observation`)

### Consumer and Evaluation Semantics

`crates/qcore-contract/src/contract_observation.rs` evaluates arithmetic and Boolean contract clauses over an explicitly supplied `IDENT -> i128` integer fact map (`contract.observation`, contract specification [§C07](QCORE_CONTRACT_SPEC_r0_3.md)). It evaluates **eagerly** (no short-circuiting: `true or missing_fact` still rejects on the missing fact, and malformed postconditions reject even when preconditions evaluate false) using checked signed `i128` arithmetic and nonzero truthiness.

### Consolidated Grammar

```ebnf
predicate  = or_expr ;
or_expr    = and_expr, { "or", and_expr } ;
and_expr   = comparison, { "and", comparison } ;
comparison = sum, { relation, sum } ;
sum        = product, { ( "+" | "-" ), product } ;
product    = unary, { "*", unary } ;
unary      = ( "-" | "!" | "not" ), unary | primary ;
primary    = NAT | IDENT | "true" | "false" | "(", predicate, ")" ;
relation   = "==" | "!=" | "<" | "<=" | ">" | ">=" ;
```

### Semantic Side Conditions and Bounds

- **Envelope restrictions:** Only `requires` and `ensures normal` clauses are admitted, with **at least one** `ensures normal` postcondition. Function calls, `old(...)`, `resource_*` / `bounds_*` predicates, `quant` clauses, `&&`/`||`, `error(E)`, and `any` selectors are rejected.
- **Associativity and overflow:** All binary operators—including relational comparisons—associate left-to-right, so `a < b < c` parses as `(a < b) < c` rather than mathematical chaining; write `(a < b) and (b < c)`. Signed decimal literals support the full `i128` range (including `i128::MIN`, repair `R07`), while arithmetic overflow (such as negating `i128::MIN` in a supplied fact) rejects.
- **Structural bounds:** At most **512 non-EOF tokens** and **parser depth 64**.

---

<a id="g06a-bounded-observed-timed-wait-contract-consumer"></a>

### G06a. Bounded observed timed-wait contract consumer

`quirin-audit observed-wait` (`crates/qcore-verification/src/monkey_wait_projection.rs`) uses the structural contract parser to load the exact five declarations `_mk_event_loop_create`, `_mk_event_wait_2`, `_mk_event_wait_with_flags`, `_mk_event_loop_destroy`, and `cb_wait_2_timeout` from `monkey/wrapper_paths.qv` and `monkey/composition.qv`, validating their signatures, ownership/usage annotations, and `requires` / `ensures normal` clauses (with no body, `unsafe`, or `effects` block) against `wait_source_policy.qv`. Missing, duplicate, added, or altered clauses reject. These named predicates bind typed C paths to native event-kernel replay obligations rather than entering `G06`'s arithmetic evaluator, and contract acceptance alone publishes no safety verdict ([observed integration report](reports/cycle18-observed-integration/README.md)).

---

<a id="g06b-typed-executable-policy-predicates"></a>

### G06b. Typed executable policy predicates (`i128` and UTF-8 text)

`check_typed_contract_facts` extends `G06` for explicitly selected zero-parameter policy declarations of the exact lexed form `extern fn NAME() -> unit contract { ... };` (rejecting parameters, generics, `effects`, `unsafe`, bodies, and non-`unit` returns):

- **Two-sorted facts (`i128` | UTF-8 text):** Admits double-quoted string literals alongside `G06` expressions, decoding `source.string.v1` escapes (`\\`, `\"`, `\n`, `\t`, `\r`, `\0`, `\xHH`) prior to comparison. Text values support only same-type `==` and `!=` (no ordering, arithmetic, Boolean truthiness, or text/integer coercion), and Boolean connectives remain eager.
- **Standalone literal extraction:** `contract_fact_literal(source, function, fact)` extracts a single `ensures normal FACT == LITERAL;` (text or signed integer) while still requiring callers to check the full policy against independently derived source/runtime facts (`monkey/wait_source_policy.qv`, `monkey/wait_identity_policy.qv`).
- **Consumer bounds:** Source text ≤ **64 KiB**, facts ≤ **256**, function/fact identifiers ≤ **256 bytes**, decoded text values ≤ **4,096 bytes**, predicate ≤ **512 tokens** and **depth 64** ([report](reports/cycle18-contract-extraction/README.md)).

---

<a id="g08-source-contract-service-configuration-v1"></a>

## G08. Source-contract service configuration (`v1` & `v2`) and audit composition

### Schema Version 1: Explicit Package and Policy Selection

This JSON schema (`crates/qcore-audit/src/source_contract.rs`) configures `quirin-audit`'s bounded Boolean-field source-contract service without adding `.qr` syntax (unknown JSON keys reject):

```json
{
  "schema_version": 1,
  "packages": ["packages/example"],
  "bindings": [
    {
      "library": "example",
      "provider": "example.boolean",
      "contract": "policy.qv",
      "frame_helpers": [
        {
          "source": "reset.c",
          "function": "reset_metadata",
          "record": "widget",
          "contract": "reset.qv"
        }
      ]
    }
  ]
}
```

- **Bounds and validation:** Configuration ≤ **1 MiB**; `1..=256` distinct package paths; `1..=4,096` bindings with unique library names and valid provider IDs; optional `frame_helpers` ≤ **256** per binding (unique C function names ≤ 128 ASCII bytes, contained relative `source` paths, exact single compile-db entry with strict preprocessing option preservation and ≤ 8 MiB per source / 32 MiB imported helper source); each UTF-8 `.qv` policy ≤ **8 MiB** and total snapshotted policies ≤ **32 MiB**.
- **Trust boundary:** `SourceContractRegistry::load` validates package manifests, target compatibility, and Boolean-field capability before dispatch, and `verify` publishes a result only after trusted core witness replay succeeds over sealed Clang imports ([report](reports/archive/early-waves/w5-configured-source-contracts/README.md), [frame helpers](reports/w5-configured-frame-helpers/README.md)).

<a id="version-2-pinned-contract-bundle-selection"></a>

### Version 2: Pinned Contract Bundle Selection

`"schema_version": 2` accepts either v1 string policy paths or a pinned bundle-asset selector object on any binding or `frame_helpers` `"contract"` field (`qcore_contract::bundle::ContractBundle::load`):

```json
{
  "manifest": "policy-bundle/contracts.json",
  "sha256": "<64 lowercase hexadecimal digits pinned by the consumer>",
  "owner": "qcore.cmetrics",
  "path": "contracts/cmetrics/counter_reset.qv"
}
```

- **Total-or-reject bundle verification:** Validates the provider inventory (`schema_version: 1`, `plugin_id == owner`, `sha256` match, unique forward-slash keys under `contracts/` ending in `.qv` or `.qcore` with no `.`/`..`/backslash/colon segments, zero symlinks or special files, exact recursive directory tree match, and every per-file SHA-256).
- **Limits:** Manifest ≤ **1 MiB**, `1..=4,096` files, ≤ **8,192** tree entries, ≤ **32** directory levels, ≤ **8 MiB** per non-empty UTF-8 file, and ≤ **32 MiB** per bundle (sharing the service's 32 MiB policy snapshot cap). Bundle loading establishes byte identity only—never predicate truth or implicit process registration.

<a id="configured-c-audit-composition"></a>

### Configured C Audit Composition

`qcore-audit` schema-1 audit configurations accept an optional `"source_contracts"` block containing `"configuration"`, `"source_rules"`, and `"libraries"` (`{"library", "source_root"}`), all required when the block is present (`crates/qcore-audit/README.md#configured-source-contract-processes`). It runs bounded Boolean source import and evidence-v2 replay alongside C resource auditing without promoting raw C resource verdicts.

---

<a id="g10-existing-c-generic-selections"></a>

## G10. Existing C: selection-invariant `_Generic` expressions

### Consumer and Token-Level Lowering Rule

In preprocessed C11 (`§6.5.1.1`) analyzed by the generic custody gateway, a `_Generic` selection inside a function body is admitted by **selection invariance** over the gateway's token stream:

```ebnf
selection    = "_Generic", "(", control, ",", association, { ",", association }, ")" ;
association  = ( type_name | "default" ), ":", expression ;
expression   = { "(" }, { pointer_cast }, core, { ")" } ;
pointer_cast = "(", IDENT, { IDENT }, "*", { "*" | qualifier }, ")" ;
qualifier    = "const" | "volatile" | "restrict" | "__restrict" | "__restrict__" ;
core         = IDENT, "(", arguments, ")" | IDENT | other_expression ;
```

- **Invariance condition:** At most one association is `default`, and after stripping outer parentheses and outer object-pointer casts (permitted only around a parenthesized group, direct call, or single identifier—and if any cast is stripped, `core` `MUST` be a direct call `IDENT(...)` declared in the same TU as returning an object pointer), **every association `MUST` reduce to the exact same token sequence `core`**.
- **Lowering:** Nested `_Generic` selections inside associations lower inner-first; the unevaluated `control` operand is dropped; and the entire `_Generic(...)` rewrites to `core` (parenthesized unless a call or identifier) across statements, initializers, conditions, returns, and call arguments (covering glibc 2.43's C23 `__glibc_const_generic` wrappers for `memchr`, `strchr`, `strstr`, `bsearch`, etc.). Non-invariant selections fall through to route `G`'s Clang-resolved `CG07` fallback (`11g`) or refuse as uncontracted `_Generic`.

**Implementation & Provenance:** `crates/qcore-c-gateway/`, `proof/QCore/GenericSelection.lean`; [report](reports/archive/waves-40-47/w44-c11-generic-selection/README.md) (complemented by `CG07`, gateway rule `11g`).

---

<a id="g11-existing-c-indexed-childnext-lookup"></a>

## G11. Existing C: bounded read-only indexed `child`/`next` lookup

### Matched C Function Shape

The C gateway's read-only indexed-lookup summary class matches the following complete preprocessed function body (up to alpha-renaming of function, parameter, local, and struct/typedef identifiers):

```c
static T *F(const T *P, U N) {
    T *C = 0;
    if (P == 0) { return 0; }
    C = P->child;
    while ((C != 0) && (N > 0)) { N--; C = C->next; }
    return C;
}
```

### Semantic Side Conditions and Caller Refinement Boundary

- **Callee body constraints:** `static` and `const` on `P` are optional; `T` `MUST` resolve to a complete, unambiguous non-union `struct TAG` (or its unique defining typedef); both `child` and `next` `MUST` be unqualified `T *` members; `U` `MUST` be a plain unsigned integer type (not pointer, `bool`, enum, float, opaque, `volatile`, or `_Atomic`); only `0` and parenthesized/`(void *)0` `NULL` spellings normalize; any extra call, write, guard variation, or deeper indirection rejects.
- **Inferred summary vs. caller identity refinement:** Validates as a nullable `Borrowed` (`parent0`, `const_dropped`) derived result—never a fresh allocation. Only a direct assignment `D = F(P, DECIMAL);` with a canonical decimal index **`0..=255`** (avoiding unsigned width truncation) receives private caller identity refinement, and only when the caller holds unique live parent custody, matching canonical `struct TAG` across callee/actual/link-owner, and source-derived must-held physical link facts (currently covering exact stored-head `0` and explicit empty-head `NULL` cases; general successor closure and `Removed` wrapper composition remain `Unknown`).

**Implementation & Provenance:** `crates/qcore-c-gateway/`; gateway rule [13a](agents/gateway.md#constraint-13a), [report](reports/detach-lookup-membership/README.md).

---

<a id="g12-existing-c-checked-call-free-condition-composition"></a>

## G12. Existing C: checked call-free condition composition

### Admitted Controlling-Expression Shape

In `if`, `while`, `do`, and `for` controlling expressions, the resource gateway retains its complete Boolean control-flow plan across parentheses, `!`, short-circuit `&&` and `||`, and ternary `A ? B : C` (with C precedence and short-circuit edge placement) whenever **every leaf already classifies completely** as a call-free check and at least one node is a checked `IndirectScalarAccess` (for example, `if ((cf->fd != -1) && (cf->map != NULL))`).

### Semantic Side Conditions

- **Zero leaf widening:** Adds no new leaf syntax, cast, qualifier, pointer-value, scalar-value, or call-effect rule; every original per-hop non-null, liveness, and alias obligation remains enforced at its evaluation point, and any unclassifiable leaf rejects the entire Boolean plan and restores the original refusal probe.

**Implementation & Provenance:** `crates/qcore-c-gateway/`, `proof/QCore/CallFreeConditionReads.lean`; gateway rule [5h](agents/gateway.md#constraint-5h), [report](reports/archive/wave-48/w48-bottom-up-batch4/README.md).

---

<a id="g07-extended-grammar-and-conformance"></a>

## G07. Relation to the extended design grammar and conformance boundary

[`QCORE_DESIGN_GRAMMAR_r0_3.bnf`](QCORE_DESIGN_GRAMMAR_r0_3.bnf) consolidates the nineteen target-language BNF blocks from revision 0.2 together with the optional `module` header (`IDENTIFIER-TOKEN`, `INTEGER-TOKEN`, `FLOAT-TOKEN`, `STRING-TOKEN`, `CHAR-TOKEN`). It serves solely as the **long-term surface-design reference**, not as a compiler parser-generator input or a statement of implemented lowering.

In Quirin revision 0.3:
1. **Consumer grammars are authoritative for current support:** Each `.qr` execution route (`Parts II–IV`), `.qv` contract/property loader (`Part V`), observation/configuration reader (`G06`, `G08`), and existing-C gateway subset (`G10`–`G12`) admits **only** the productions and side conditions documented in its own section above.
2. **Structural acceptance ≠ semantic conformance:** Retained token spans in `G01` do not make arbitrary expressions executable or arbitrary postconditions proved; any construct outside the selected consumer's implemented grammar `MUST` fail closed (`Q***` compilation refusal or `Unknown` audit verdict).
