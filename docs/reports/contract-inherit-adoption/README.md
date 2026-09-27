# CONTRACT-INHERIT-ADOPTION: contract inclusion and inheritance in the real provider crates

Card CONTRACT-INHERIT-ADOPTION, 2026-09-24, on base `dfe10c92` (main after the
CONTRACT-INHERIT integration). [CONTRACT-INHERIT](../contract-inherit/README.md)
added `#include "CRATE:MODULE";` and `contract inherit [NS::]NAME;` and tested
them against fixture crates only. This card wires the four real-crate pieces
that report's §1.3 lists, converts the provider contracts that restate a libc
contract where the rules admit it, and declares two Fluent Bit string wrappers
by inheritance at the user's direction.

## Headline

* **Wiring, all four pieces.** Each of the six `quirin-provider-*` crates
  records `NAMESPACE` and `PUBLISHED` beside `CONTRACTS`. A new generated file,
  `providers/contract-load-path.json`, carries crate names, versions and
  contract-use edges from Cargo metadata, plus each published module's SHA-256.
  Core crates read it as data (`qcore_contract::load_path_manifest`) and link
  no provider crate. The resource manifest and the corpus index move to
  **schema 2**, which can select `crate:module` references; path entries
  resolve and are retained exactly as before. `providers/contracts.py sync`
  computes package include pins, `package.py build` installs them, and
  `verify` and the Rust `ContractBundle` both accept them. One Cargo edge is
  added: `quirin-provider-fluent-bit → quirin-provider-libc`.
* **Conversions, zero verdict movement.** `flb_malloc`, `flb_calloc` and
  `flb_free` now read `contract inherit libc::malloc` / `calloc` / `free`.
  Their elaboration equals the hand-written contracts token for token, and the
  loaded records are equal. The arbiter's verdict-bearing diagnostics are
  **byte-identical** to the W43 baseline before the card and after each of its
  three stages.
* **New declarations.** `libc::strndup` passes the declared-ABI gate in all 181
  units that declare it. `flb_strdup` and `flb_strndup` inherit `libc::strdup`
  and `libc::strndup` in a new module, `fluent_bit/resource_string.qcore`,
  which the Cycle 19 manifest selects by reference. These are **trusted
  assumptions over `static inline` bodies the analyzer could read**. Only one
  pinned unit includes their header, and it has no call site. They move
  nothing.
* **Census.** 46 provider declarations outside libc have clauses equal to a
  libc contract under positional renaming. 35 are coincidences of the neutral
  element. Of the 11 custody-bearing matches (13 at base), the rules admit one,
  `flb_sds_destroy`, which is Fluent Bit's own function and stays written out.
  Nine are refused because their crates are `qcore-provider-*` (`QI017`),
  including `rd_strdup`. The naming decision is laid out with its costs in §6,
  and not taken.
* **Lean.** `proof/QCore/ContractInherit.lean` gains a load-path-from-metadata
  section: 8 theorems, 38 → 46, audited, with no placeholder and no project
  axiom.
* **Follow-ups 294 → 296.** `CONTRACT-INHERIT-ADOPTION` is retired.
  `CONTRACT-INHERIT-QCORE-PROVIDER-NAMING`,
  `CONTRACT-INHERIT-FLB-STRING-ASSUMPTIONS` and
  `CONTRACT-INHERIT-ALIAS-RESULT-PARENTS` are registered. The last answers the
  coordinator's question about the W43-LIBC-RESULT-ALIAS result predicates
  (§12).

## 1. Real-crate wiring

### 1.1 Recorded namespace and published list

Each of `quirin-provider-{libc,cfl,mpack,msgpack,fluent-bit,shared}` now has:

```rust
pub const NAMESPACE: &str = "libc";
pub const PUBLISHED: &[&str] = &["libc/memory.qcore", "libc/string.qcore"];
```

Every crate test `namespace_is_derived_and_published_modules_are_owned` checks
two things. The recorded namespace must equal the derivation of its own
`CARGO_PKG_NAME` (`quirin-provider-fluent-bit` → `fluent_bit`), and every
published module must be an owned one, sorted and listed once. Only libc
publishes anything, and it publishes the two modules that are included today.
Publication is deliberately minimal. A published module's text becomes part of
every inheriting contract's identity, and its digest is pinned (§1.2), so each
extra publication would be one more pin to regenerate on every edit.
`quirin-provider-shared` joins the load path with namespace `shared` and
publishes nothing. It is not a re-exporter: `libc::malloc` stays `libc::malloc`.

The Cargo edge `quirin-provider-fluent-bit → quirin-provider-libc` is real and
load-bearing. The fluent-bit crate test `every_include_names_a_published_libc_module`
links `quirin_provider_libc::PUBLISHED` and checks every directive against it.
`providers/Cargo.lock` gains exactly that dependency line.

### 1.2 The load-path manifest, read as data

`python3 providers/contracts.py sync` writes `providers/contract-load-path.json`
(`check` compares and never writes). It holds one record per
`quirin-provider-*` workspace package:

| Field | Source |
|---|---|
| `name`, `version` | `cargo metadata --no-deps --offline --locked` |
| `dependencies` | the package's **normal** dependencies that are themselves load-path crates (dev and build dependencies are not contract edges) |
| `namespace`, published list | the crate's `NAMESPACE` and `PUBLISHED`, parsed in one admitted spelling, anything else refused; the namespace must equal the derived one |
| `contracts`, `modules` | the crate's `contracts/` directory and every `.qcore` under it |
| `published` | each published module → SHA-256 of its text (the **pin**) |

`qcore_contract::load_path_manifest::load_contract_load_path_manifest` reads it
total-or-refuse (`QI014`). It checks the shape (`deny_unknown_fields`); a
relative contracts directory with no traversal; sorted, unique, admitted module
spellings that equal the `.qcore` files on disk **exactly**; and every
published module's text against its pin. Only then does it build a
`ContractLoadPath`, which applies the unchanged `QI014`/`QI017` rules. A stale
manifest is refused, never read: a parent edited without `sync` cannot be
inherited from. The meaning of `QI014` is extended to say so in
`docs/agents/diagnostics.md`, and no new code is allocated.

The layering pin (`compiler/tests/workspace_layering.rs`) is untouched, because
no core crate links a provider crate. Three checks keep the data honest:

* `contracts.py check`, which regenerates the file from Cargo metadata;
* the shared crate's `load_path_manifest_matches_the_linked_crates`, which
  compares namespace, modules and pins with the six linked crates' constants;
* the core test `the_real_load_path_reads_and_records_what_the_crates_declare`.

### 1.3 Schema 2 of the resource manifest and the corpus index

`load_resource_contract_manifest` accepts two schemas:

* **Schema 1** is unchanged: paths only, and no `load_path` key.
* **Schema 2** requires `load_path` and admits `CRATE:MODULE` entries, spelled
  and validated exactly like an `#include` target. An entry that contains `:`
  is a reference, and a path entry may not contain `:`.

Path entries are read by the same code under the same key as in schema 1.
References are elaborated and pass through the v1 agreement gate, then are
retained under `CRATE:MODULE`. One total-or-reject load covers both kinds
(`load_resource_contract_selection`, which `load_published_resource_contracts`
now delegates to). Tests in `compiler/tests/contract_inherit_adoption.rs` cover
three things:

* **Parity.** `schema_two_path_entries_load_exactly_as_schema_one` compares
  environments byte for byte.
* **Mixed selection.** A path entry and a reference load together, and the
  inherited contract equals its parent's.
* **Refusals.** Ten negative controls each fail closed: a missing `load_path`,
  `load_path` under schema 1, an unknown schema, two malformed references,
  `QI004`, `QI005`, `QI013` for an inclusion selected by path, one module
  selected twice, and load-path traversal. So do ten load-path mutations and
  the stray-module and changed-parent cases.

`cycle19-resource-contracts.json` is now schema 2. `fluent_bit/resource_memory.qcore`
moved from a path entry to a reference, and `fluent_bit/resource_string.qcore`
was appended. The other 43 entries are byte-identical.

`contract/corpus.json` schema 2 adds `load_path` beside the unchanged roots. The
corpus tests read an inheriting module as its elaboration (`compiler/tests/support/contract_corpus.rs::text`),
which is the text every loader parses. `scripts/contract_corpus.py` accepts
both schemas. Both corpus readers refuse an unknown key set.

The audit sidecar `resource-diagnostics.contracts.json` now publishes
`contract_inclusion` (the chains) when the environment used inclusion, as the
configured audit's assumptions block already did. It is absent otherwise, so a
selection without inclusion writes the same bytes.

### 1.4 Package include pins

* **Pins.** `providers/contracts.py sync` computes each provider inventory's
  `includes` pins (crate, version, module, SHA-256) transitively on the load
  path, using `package.py`'s own directive scanner. It refuses an include of an
  unpublished or unknown module.
* **Installation.** `package.py build` installs each pinned text under
  `includes/<crate>/<module>` and refuses a text that changed after pinning.
  `verify` accepts the result, as its CONTRACT-INHERIT rule already required.
* **Rust.** `qcore_contract::bundle::ContractBundle` now admits the optional
  `includes` object (`deny_unknown_fields` per pin) and applies the same rule
  in Rust. It refuses an unpinned directive, an unused pin, a changed, missing
  or stray text, and an `includes/` tree without pins. `ContractBundle::included`
  serves the pinned text.

**No real package includes anything, and none can yet.** Every packaged
contract belongs to a `qcore-provider-*` crate, which cannot be on a load path
(§6). All five inventories are unchanged and `includes` is 0 for each. The
mechanism is exercised end to end on synthetic packages only:

* `providers/tests/test_contract_load_path.py` (7 tests);
* `crates/qcore-contract/tests/contract_bundle_includes.rs` (2 tests).

## 2. Converted duplicates: parity

`fluent_bit/resource_memory.qcore` now reads:

```qcore
module contract;
#include "quirin-provider-libc:libc/memory.qcore";
extern fn flb_malloc(size: usize) -> ptr<mut, u8> contract inherit libc::malloc;
extern fn flb_calloc(count: usize, size: usize) -> ptr<mut, u8> contract inherit libc::calloc;
extern fn flb_free(ptr: ptr<mut, u8>) -> unit contract inherit libc::free;
```

`flb_sds_create_len` and `flb_sds_destroy` stay written out. `flb_sds_destroy`'s
clauses coincide with `libc::free`'s, but its body frees the header in front of
the string (`src/flb_sds.c:381`). An inherited contract tracks its parent, and
this one must not. `fluent_bit/memory.qcore`, the allocator profile, is not
touched.

The wrappers are what they claim to be. In `include/fluent-bit/flb_mem.h`,
`flb_malloc` is `size == 0 → NULL`, then `malloc(size)`, and `flb_calloc` is
`size == 0 → NULL`, then `calloc(n, size)`. The OSS-Fuzz arms are absent from
the pinned build's configuration. `flb_free` is `free(ptr)`. The zero-size
`NULL` is covered by the parent's `nullable` owned result.

**Parity evidence**, all in `compiler/tests/contract_inherit_adoption.rs`,
against `fixtures/contract_inherit_adoption/resource_memory_hand_written.qcore`,
the file byte for byte as it stood at `dfe10c92`:

* **Tokens.** `adopted_inheritance_elaborates_to_the_hand_written_contracts`:
  the elaborated module declares the same five functions in order. Each
  declaration's token sequence equals the hand-written one, so the only
  differences are whitespace and comments.
* **Loaded records.** The loaded functions, ABI projections, literal premises
  and variadic tails are equal.
* **Chains.** Each converted declaration has one link to
  `quirin-provider-libc@0.1.0:libc/memory.qcore#{malloc,calloc,free}`, whose
  digest is the libc file's.
* **Whole environment.** `the_resource_manifest_selects_the_adopted_module_with_equal_meaning`
  loads the Cycle 19 manifest, then replaces the adopted module with the
  hand-written one. The functions and every rederived fact (ABI, premises,
  tails) are equal, `check_source_assumptions` replays the evidence, and
  `inclusion_provenance` publishes it.

**Pins count elaborated clauses.** `EXPECTED_CONTRACTS` in
`compiler/tests/qcore_contract_corpus.rs` still pins `flb_malloc` 3,
`flb_calloc` 4 and `flb_free` 3. Those are the clauses the loader loads and the
environment retains, so a conversion is parity by construction there. The
written form is pinned separately by a new `EXPECTED_INHERITANCE` table
(`inherited_contracts_are_pinned_in_their_written_form`), so a conversion is
still a deliberate edit. `contract_loader.rs` counts elaborated declarations for
the same reason.

## 3. Census: provider contracts equal to a libc contract

[`scripts/census.py`](scripts/census.py) reads every `extern fn … contract`
under `providers/`. It compares parameter and result types exactly, and
clauses after each parameter word is replaced by its position. That is the
renaming `contract inherit` applies. Results:
[census.json](census.json) (after) and [census-base.json](census-base.json)
(base, run on an export of `dfe10c92`).

| | Base | After |
|---|---:|---:|
| libc declarations | 61 | 62 (+`strndup`) |
| provider declarations outside libc | 229 | 231 (+2 string wrappers) |
| already inheriting | 0 | 5 |
| clause-equal matches | 48 | 46 |
| — neutral element (all borrowed, no custody result): a coincidence of shape | 35 | 35 |
| — custody-bearing | 13 | 11 |
| admitted by the rules | 0 | 1 |

The 35 neutral matches are refused as candidates. A match such as
`cio_chunk_is_up ≡ pthread_mutex_lock` is two unrelated functions that both
borrow a pointer and return a scalar. Inheriting would record a false
provenance, and it would make the child change whenever the parent does.

| Custody-bearing match | Crate | Rules | Wrapper? | Disposition |
|---|---|---|---|---|
| `flb_malloc`, `flb_calloc`, `flb_free` ≡ `malloc`, `calloc`, `free` | fluent-bit | admitted after the edge | yes (§2) | **converted** |
| `flb_sds_destroy` ≡ `free` | fluent-bit | admitted | no: frees the header | written out (brief) |
| `cfl_sds_destroy` ≡ `free` | cfl | `QI004`: no libc edge | no: CFL's own release | written out |
| `rd_malloc`, `rd_calloc`, `rd_strdup`, `rd_strndup`, `rd_free` ≡ `malloc`, `calloc`, `strdup`, `strndup`, `free` | librdkafka | `QI017` | yes: `rd.h` forwards, plus `rd_assert` | **blocked on §6** |
| `mk_mem_alloc`, `mk_mem_alloc_z`, `mk_mem_free` ≡ `malloc`, `malloc`, `free` | libevent (monkey) | `QI017` | yes: `mk_memory.h` forwards to `malloc`, `calloc(1, n)`, `free` | **blocked on §6** |
| `cJSON_Delete` ≡ `free` | librdkafka | `QI017` | no: a recursive delete | written out |

`rd_strndup` becomes a match only once `strndup` is declared. The base's 13
custody-bearing matches are the three converted, the eight blocked, and
`flb_sds_destroy` and `cfl_sds_destroy`. `mk_mem_alloc_z` calls
`calloc(1, size)` but matches `libc::malloc` by signature. Its conversion
would name `malloc` as the parent, which the §6 card should record.

## 4. `libc::strndup` and the ABI agreement gate

`strndup(s: ptr<const, u8>, n: usize) -> ptr<mut, u8>` is appended to
`providers/quirin-provider-libc/contracts/libc/string.qcore`: borrowed `s`,
borrowed `n`, `resource_return_owned(nullable)`. It carries the glibc
prototype and the "what no declaration says" note. Nothing else in the file
changed, per the coordination rule.

**Gate** ([abi-census.txt](abi-census.txt)). This is the W43 route. It ran
`target/release/examples/declared_abi_census libc/string.qcore resource_string(written out) -- UNIT.i…`
over the 185 pinned units' line-marker-stripped preprocessing, copied from the
W43-LIBC-CONTRACTS capture. The unit set's digest list is
[abi-units.sha256](abi-units.sha256), whose own SHA-256 is `05dcea18…c07`.

| Name | Agreed | Withdrawn |
|---|---:|---:|
| `strndup` | **181** | **0** |
| `strdup`, `strcmp`, `strlen` | 181 each | 0 |
| `strerror_r` (unchanged, as W43 recorded) | 68 | 113 |
| `flb_strdup`, `flb_strndup` | 1 each | 0 |

No unit disagrees, so nothing had to be withdrawn. The only calls to `strndup`
in the corpus are inside `rd_strndup`'s header body. That body is not a
projected function, so the arbiter moves nothing (§7, class b).

## 5. Fluent Bit string wrappers, and the wrapper census

`fluent_bit/resource_string.qcore` declares `flb_strdup` and `flb_strndup` by
inheritance. The brief directs these as assumptions for later corpus scope.
**They are trusted assumptions over code the analyzer could read.** Both are
`static inline` in `include/fluent-bit/flb_str.h`, so every including unit
carries their bodies. Wherever this module is selected, the contract stands in
for the body at every call site. `flb_strndup` is also not a byte-exact
forward of `strndup`: it copies exactly `n` bytes with `memcpy` through
`flb_malloc`, rather than stopping at a terminator. What is equal is the
custody statement, which is all resource v1 says. The module's comment says
the same.

**Selection decision: selected.** Of the 185 pinned units, exactly one,
`flb_input_chunk.c`, includes `flb_str.h`. In that unit the two names occur
only in the header's own inline bodies (`flb_strdup` → `flb_strndup`,
`flb_rtrim` → `flb_strndup`), and no projected function calls either. The
arbiter measures **zero** movement (class c). Selecting the module is what
makes its chains appear in the published assumptions. The corpus pin tests
also require every v1 declaration in the corpus to be manifest-selected
(`contract_loader.rs`: manifest functions = v1 declarations). The
counter-argument is the W43 refusal to contract checkable header-inline glibc
bodies. It is recorded rather than resolved:
`CONTRACT-INHERIT-FLB-STRING-ASSUMPTIONS` asks for the arbiter attribution and
a W4 comparison once the scope reaches a caller.

**Wrapper census.** [`scripts/wrapper_census.py`](scripts/wrapper_census.py)
covers the `flb_*` `static inline`/`FLB_INLINE` functions in
`include/fluent-bit/**/*.h` that call a declared libc function; see
[wrapper-census.json](wrapper-census.json). The OSS-Fuzz arms are removed,
because the pinned build does not define that macro.

| Wrapper | Shape | Reading | Recommendation |
|---|---|---|---|
| `flb_free` | exact forward | `free(ptr)` | converted |
| `flb_malloc`, `flb_calloc` | single callee | zero-size guard, then forward | converted (guard within `nullable`) |
| `flb_strdup`, `flb_rtrim` | composite | `flb_strndup(s, strlen(s))`; trim loop | `flb_strdup` user-directed (above); `flb_rtrim` not declared |
| `flb_task_acquire_lock` / `_release_lock` | single callee | `pthread_mutex_lock(&task->lock)`, `void` result | not a forward (member address, result differs; `QI010`) |
| `flb_stream_release_lock`, `flb_stream_acquire_lock` | single callee / composite | guarded by `thread_safety_flag`, result remapped to `FLB_TRUE/FALSE` | not a forward |
| `flb_str_emptyval`, `flb_opentelemetry_error_code` | single callee | `strcmp` inside a test or loop | not a forward |
| `flb_parser_tm2time`, `flb_input_coro_create`, `flb_output_flush_create` | composite | many callees | not a forward |

The census reads 144 inline `flb_*` functions in 203 headers and finds 14
candidates: 1 exact forward, 7 single-callee and 6 composite. **No further
exact forwarding wrapper exists**, so nothing more is declared. `flb_realloc`
forwards to `realloc`, but `realloc` has no libc declaration, so it is outside
this census.

## 6. The `qcore-provider-*` naming decision (not taken)

The prefix rule (`QI017`) refuses the five artifact crates
`qcore-provider-{chunkio,cmetrics,ctraces,libevent,librdkafka}` on a load path.
Their eight genuine allocator wrappers therefore keep restating libc
contracts. Those are `rd_malloc`, `rd_calloc`, `rd_strdup`, `rd_strndup` and
`rd_free`, plus `mk_mem_alloc`, `mk_mem_alloc_z` and `mk_mem_free`. As the
brief directs, this card does not rename crates or change the rule. The
options follow; the counts are measured at `dfe10c92`.

| Option | What changes | Cost |
|---|---|---|
| **A. Rename** to `quirin-provider-*` | five crate directories and package names, `Cargo.toml`/`Cargo.lock`, `contracts.py` `OWNERS`, `package.py`'s `qcore-provider-*` template glob, every path spelling | **113** tracked files and **233** occurrences outside `docs/reports` and `docs/plans`; **51** files in **27** frozen report directories keep the old paths and may not be edited; archived contract-source keys (`providers/qcore-provider-*/contracts/...`) change spelling in every later arbiter; plugin ids `qcore.*` can stay, but the executables are named after their crates, so package digests are expected to move (not measured). The data crates would also share a prefix with executables, which blurs what `quirin-provider-*` has meant (data only). |
| **B. Widen the rule** to a second fixed prefix, `qcore-provider-NAME` → `NAME` | `derive_contract_namespace`, the Lean `deriveNamespace` (two prefixes; `namespace_names_one_crate` still holds under the distinctness check), C15 and `QI017` text; a `NAMESPACE`/`PUBLISHED` record for **bin** crates (they have no `lib.rs`; for example `[package.metadata.quirin]`, which `contracts.py` would read from Cargo metadata); a Cargo edge `qcore-provider-{librdkafka,libevent} → quirin-provider-libc` | small and local; the namespace stays mechanically derivable from the package name. Namespaces measured today: `chunkio`, `cmetrics`, `ctraces`, `libevent`, `librdkafka`, none colliding with `libc`, `cfl`, `mpack`, `msgpack`, `fluent_bit` or `shared`. A future same-named crate under both prefixes is refused by the existing uniqueness check. The libevent crate owns `monkey/` modules, so its parents would be written `libevent::…`. |
| **C. Recorded namespace** accepted without derivation | the load path would trust the record, checking only identifier shape and uniqueness | weakest: a reviewer can no longer read the qualifier off the package name, which was the point of the user's amendment. |

**Recommendation: B.** It is the smallest change that keeps derivation
mechanical. It leaves every pinned package id and every frozen report path
alone. It also exercises §1.4's include pins on a real package. Registered as
`CONTRACT-INHERIT-QCORE-PROVIDER-NAMING` for the user's decision. `rd_strdup`
stays hand-written.

## 7. Corpus arbiter (constraint 7)

Every run used [`../w43-return-expression/scripts/run_audit.py`](../w43-return-expression/scripts/run_audit.py)
in this worktree. BEFORE ran on a release binary built at `dfe10c92`
(sha256 `5b7201ad…7797`). Stages A to C ran on one release binary built at
`b0a91e17` (sha256 in `arbiter/after-*-run.json`). Between `b0a91e17` and
`fb9e9146`, the only source change in a crate that binary links is `rustfmt`
(`git diff --stat b0a91e17 fb9e9146 -- crates/*/src compiler/src compiler/examples`
lists formatting hunks only). The audit reads the manifest, the load path and
every contract at runtime, so each stage is a data delta over one binary.

| Run | Tree | What it adds | Exit | Record |
|---|---|---|---:|---|
| BEFORE | `dfe10c92` | nothing | 0 | [before-run.json](arbiter/before-run.json), [before-vs-w43.json](arbiter/before-vs-w43.json) |
| AFTER-A | `b0a91e17` | wiring, schema 2, the three conversions | 0 | [after-a-vs-before.json](arbiter/after-a-vs-before.json) |
| AFTER-B | `1de8999b` | `libc::strndup` | 0 | [after-b-vs-after-a.json](arbiter/after-b-vs-after-a.json) |
| AFTER-C | `fb9e9146` | `resource_string.qcore`, selected | 0 | [after-c-vs-after-b.json](arbiter/after-c-vs-after-b.json), [after-c-vs-w43.json](arbiter/after-c-vs-w43.json), [after-c-vs-before.json](arbiter/after-c-vs-before.json) |

**BEFORE reproduces the baseline.** The diagnostics, chunk-contracts and source
are byte-identical to `docs/reports/w43-libc-contracts/artifacts/`
(diagnostics `58a657b5…c955`). `contracts` is identical after the root alias.
`source-effects` differs by the checkout-path digests the brief lists; that
difference is pre-existing and identical at base.

**Attribution, leaf by leaf.** Comparisons are made with
[`scripts/arbiter_compare.py`](scripts/arbiter_compare.py).

| Class | Verdict-bearing diagnostics | Function / unit status moves | Other artifacts |
|---|---|---|---|
| (a) wiring and conversions | **byte-identical**: 0 leaves removed, 0 added | 0 / 0 | chunk-contracts, source-effects and source byte-identical. `contracts`: the manifest text, one source key moved from the `resource_memory.qcore` path to `quirin-provider-fluent-bit:fluent_bit/resource_memory.qcore` (elaborated text), and `contract_inclusion` added with three chains |
| (b) `strndup` | **byte-identical** | 0 / 0 | `contracts`: the `libc/string.qcore` source text only |
| (c) `resource_string.qcore` | **byte-identical** | 0 / 0 | `contracts`: the manifest text, one source added, two more chains |

AFTER-C is byte-identical to the W43 baseline's diagnostics,
`verdict_bearing_identical: true`. The leak pin holds at **272**. No new
baseline is archived, because nothing verdict-bearing moved.
`docs/reports/w43-libc-contracts/artifacts/` stays the baseline. The published
chains, with each link's crate, crate version, module and digest, are in
[after-c-contract-inclusion.json](arbiter/after-c-contract-inclusion.json):

```text
resource_memory.qcore#flb_malloc  -> quirin-provider-libc@0.1.0:libc/memory.qcore#malloc  sha256=c7e477f4…
resource_memory.qcore#flb_calloc  -> …libc/memory.qcore#calloc
resource_memory.qcore#flb_free    -> …libc/memory.qcore#free
resource_string.qcore#flb_strdup  -> quirin-provider-libc@0.1.0:libc/string.qcore#strdup  sha256=1aa549b6…
resource_string.qcore#flb_strndup -> …libc/string.qcore#strndup
```

That is the discharge condition `CONTRACT-INHERIT-ADOPTION` names. The adopted
environment's diagnostics equal the hand-written baseline's, and the
inheritance chains appear in the published assumptions.

## 8. Lean correspondence

`proof/QCore/ContractInherit.lean` §7, audited in `proof/AuditContractInherit.lean`
(38 → 46 `#print axioms`). As constraint 28 says, the correspondence is
maintained by review and is not extraction.

| Rule | Rust / Python | Lean |
|---|---|---|
| contract edges are declared Cargo dependencies that are themselves load-path crates | `contracts.py load_path_manifest` | `contractDeps`, `contractDeps_sound` |
| an include from a crate built from metadata follows a real Cargo edge or stays in-crate | `LoadPathResolver` over the generated load path | `include_follows_a_cargo_edge`, `no_cargo_edge_includes_nothing_foreign` |
| a published module whose text is not its pin refuses the load path | `load_contract_load_path_manifest` (`QI014`) | `admitPinned`, `admitPinned_sound`, `changed_parent_refused` |
| schema-2 path entries load exactly what schema 1 loads | `load_resource_contract_manifest` | `selectSchema2`, `pathsOf_map_path`, `refsOf_map_path`, `schema_two_paths_are_schema_one` |

Not modelled: the byte-level JSON reading, the directory walk that compares
the listed modules with the disk, and the `:` classification of entries. The
Rust tests in §1.3 cover those.

## 9. Validation

Measured in this worktree (`agent-ac5154b5c6b7f17b9`). [`scripts/run_gate.py`](scripts/run_gate.py)
captured each step's exit to a file, never through a pipe
(`CARGO_BUILD_JOBS=6`, `RUST_TEST_THREADS=2`, debug info off). Logs are
worktree-private under `measure/gate/`. [gate.json](gate.json) is the generator
input. Clang was recorded in this checkout ([clang-version.txt](clang-version.txt)).

The core and provider gates ran on `2d6de0d9`. Later commits change the
report, ledger, status, index and matrix only.

| Command | Exit | Result | Log |
|---|---:|---|---|
| `git submodule update --init --recursive` | 0 | libevent at `d82464a2…` | `measure/gate2/submodules.log` |
| `clang --version` | 0 | Ubuntu clang 21.1.8 | `measure/gate2/clang.log` |
| `cargo test --workspace --no-fail-fast` | 0 | **5,187 passed / 0 failed / 11 ignored**, 277 summaries | `measure/gate2/test.log` |
| `cargo fmt --all -- --check` | 0 | clean | `measure/gate2/fmt.log` |
| `cargo clippy --workspace --all-targets -- -D warnings` | 0 | clean | `measure/gate2/clippy.log` |
| `cargo run -p qcore-compiler --example reevaluation_probe` | 0 | | `measure/gate2/probe.log` |
| `cargo run -p qcore-compiler --example reevaluation_followup` | 0 | | `measure/gate2/followup.log` |
| `python3 scripts/check_lean_audits.py --jobs 4` | 0 | **168** audits (`AuditContractInherit` 38 → 46 theorems) | `measure/gate2/lean.log` |
| `cargo test --manifest-path providers/Cargo.toml --workspace --locked --offline --no-fail-fast` | 0 | **117 passed / 0 failed / 0 ignored**, 39 summaries | `measure/gate2/ptest.log` |
| `… -p qcore-provider-conformance --test packages real_packages_preserve_unknown_and_reject_mutations --exact --test-threads=1` | 0 | 1 passed (isolated, constraint 16) | `measure/gate2/pisolated.log` |
| `cargo fmt --manifest-path providers/Cargo.toml --all -- --check` | 0 | clean | `measure/gate2/pfmt.log` |
| `cargo clippy --manifest-path providers/Cargo.toml --workspace --all-targets --locked --offline -- -D warnings` | 0 | clean | `measure/gate2/pclippy.log` |
| `python3 providers/contracts.py check` | 0 | 72 provider-owned, 0 includes, inventories unchanged; load path current | `measure/gate2/pcontracts.log` |
| `python3 providers/package.py build --output measure/gate2/packages` | 0 | 5 packages | `measure/gate2/pbuild.log` |
| `python3 providers/package.py verify …/<id>` (×5) | 0 ×5 | | `measure/gate2/pverify-<id>.log` |
| `python3 -m unittest discover -s providers/tests` | 0 | **12** tests (+7) | `measure/gate2/ppython.log` |
| `python3 docs/reports/remaining-d02/build_matrix.py . --gate docs/reports/contract-inherit-adoption/gate.json` | 0 | 296 follow-ups, 168 Lean audits, 108 contract modules | `measure/build-matrix-3.log` |
| `python3 -m unittest discover -s scripts/tests` (after the final regeneration) | 0 | **330** tests | `measure/gate4/python.log` |
| `python3 scripts/build_docs_index.py`, then `--check` | 0 / 0 | index regenerated and matching | `measure/docs-index-2.log`, `measure/gate4/docs-index.log` |

**Where the deltas come from.**

* **Core Rust**, +11 over the CONTRACT-INHERIT gate's 5,176:
  * 8 in `contract_inherit_adoption.rs`;
  * 2 in `contract_bundle_includes.rs`;
  * 1 corpus inheritance-pin test.

  The two new test binaries account for summaries 275 → 277.
* **Provider Rust**, +8 over 109:
  * six `namespace_is_derived_and_published_modules_are_owned`;
  * `every_include_names_a_published_libc_module`;
  * `load_path_manifest_matches_the_linked_crates`.
* **Python.** +3 root tests in `test_contract_corpus.py` (327 at base).
  Provider Python +7 in `test_contract_load_path.py`.
* **Load-sensitive rows.** No `P02-FLAKE` row failed in either full run. The
  isolated package run above is the constraint-16 record, not a retry.

**Failed attempts, disclosed.** The first full gate (`measure/gate/`, on
`fb9e9146`) passed the core tests (5,187 / 0 / 11), Lean and the probes. It
failed in four places:

* `fmt` (exit 1): the stage-C pin edits had not been formatted;
* `clippy` (101): `type_complexity` in two new tests;
* provider `fmt`/`clippy` (1 / 101), for the same two reasons;
* provider `test` (101): `artifact_dependencies_are_only_schemas_and_serialization`
  pins that data contract crates have **no** dependency, and the brief's
  fluent-bit → libc edge contradicts that pin.

The pin is **deliberately updated**, not weakened. A data crate may depend
only on another data crate, never on the shared bundle or anything else. libc
stays dependency-free in effect, and the exact edge set is pinned to the one
edge. All four were repaired in `2d6de0d9`, and the gate was rerun **in full**
(`measure/gate2/`). A pre-final Python run recorded 328 in `gate.json` against
a measured 330. The file was corrected, the matrix regenerated, and the suite
rerun (`measure/gate3/`). Registering the third residual after that moved the matrix once more, so it was regenerated again and the suite and index check rerun (`measure/gate4/`, 330 tests, exit 0).

**Confirmation arbiter.** The stage runs used a binary built at `b0a91e17`
(`00185b63…929f`). A fresh release binary built at the gated tip `2d6de0d9`
(`d7839a31…a01c`, [final-run.json](arbiter/final-run.json), exit 0)
reproduces AFTER-C **byte for byte in all five artifacts**
([final-vs-after-c.json](arbiter/final-vs-after-c.json)). So the rustfmt-only
source changes after the stage binary were confirmed, not assumed, to move
nothing.

## 10. Records and pins, each deliberate

* `compiler/tests/qcore_contract_corpus.rs` has three changes:
  * corpus 107 → 108 modules;
  * two clause pins, `flb_strdup` 3 and `flb_strndup` 4, plus `strndup` 4, all
    elaborated counts;
  * the new `EXPECTED_INHERITANCE` table (5 rows).
* `compiler/tests/contract_loader.rs` has four changes:
  * modules 107 → 108;
  * (selected, v1 declarations, rejected) (44, 114, 207) → (45, 117, 207);
  * both `source_assumptions` pins 44 → 45;
  * the old-table parity list gains `strndup`, `flb_strdup` and `flb_strndup`.
* `crates/qcore-c-gateway/tests/{neighbours,prod_contracts}.rs`: 44 → 45.
* `scripts/tests/test_contract_corpus.py` has two changes:
  * 107 → 108 modules, and the `quirin-provider-*` share 35 → 36;
  * three tests: libc taken by reference, the refusals of that check, and the
    schema-2 index.
* `scripts/contract_corpus.py` has two changes:
  * a schema-2 reader;
  * `libc_inheritance`, the placement rule's inheritance half. Outside the libc
    crate, a module may take a libc contract only by including a **published**
    libc module and inheriting a name that module declares. The existing
    placement test still refuses a libc name declared outside the crate.
* Indexes: the libc `CONTRACTS` is unchanged at 14 modules (strndup is in an
  existing module); the fluent-bit `CONTRACTS` goes 12 → 13.
* `providers/contracts.py check` still counts 72 provider-owned contracts, and
  the five inventories are byte-identical.

No assertion is weakened.

## 11. Ledger and status

* **Retired: `CONTRACT-INHERIT-ADOPTION`**, preserved verbatim in
  [retired-followups.json](retired-followups.json). Its discharge asked for
  four things. (1)–(4) are wired: §1. At least one real module adopts the
  forms: §2 and §5. The arbiter shows equal diagnostics with chains in the
  published assumptions: §7. Item (5) asked for a decision, which the brief
  reserves for the user. It is therefore rehomed, not dropped.
* **Registered** ([followups.json](followups.json)):
  * `CONTRACT-INHERIT-QCORE-PROVIDER-NAMING`: the §6 decision, the eight
    blocked wrappers, and real-package include pins;
  * `CONTRACT-INHERIT-FLB-STRING-ASSUMPTIONS`: §5's trusted assumptions over
    readable bodies, with no call site yet;
  * `CONTRACT-INHERIT-ALIAS-RESULT-PARENTS`: inheriting a parent whose result
    is `resource_return_alias`/`resource_return_interior` (W43-LIBC-RESULT-ALIAS,
    on main) is analysed in §12 but not exercised on this base.
* **Still open:** `CONTRACT-INHERIT-EXTENSION-CLAUSES` and
  `CONTRACT-INHERIT-NON-V1-CONSUMERS` are untouched.
* Follow-ups **294 → 296**. `docs/STATUS.md` is updated from this report's
  evidence: the contract corpus is 108 modules, 117 v1 declarations and 45
  manifest-selected.
* The increment record `wave-41-continuation.md` was already 32,801 bytes, over
  its 32 KiB cap, before this card. As the brief directs, one paragraph is
  prepended to it anyway, and the overrun is disclosed here rather than
  repaired.

## 12. For the integrator

Files likely to conflict:

* **With W43-LIBC-RESULT-ALIAS:**
  * `providers/quirin-provider-libc/contracts/libc/string.qcore`: this card
    appends `strndup` at the end.
  * `providers/contract-load-path.json`: the `libc/string.qcore` pin changes
    whenever that file changes. Regenerate with `python3 providers/contracts.py sync`,
    never by hand. Tests refuse a stale pin.
  * `cycle19-resource-contracts.json`: schema 1 → 2, one entry becomes a
    reference, and one is appended.
  * `compiler/tests/qcore_contract_corpus.rs`: the corpus count, the string
    pins and the new table at the file end.
  * `compiler/tests/contract_loader.rs`, `crates/qcore-c-gateway/tests/{neighbours,prod_contracts}.rs`,
    `scripts/tests/test_contract_corpus.py`: counts.
  * the ledger.
* **With main:** `docs/STATUS.md`,
  `docs/development/increments/wave-41-continuation.md`, `docs/reports/INDEX.md`,
  `docs/reviews/r0_3/acceptance_matrix.json`, `docs/agents/diagnostics.md`
  (the `QI014` line), `docs/agents/providers.md`, and C15/G05a in the specs.
* **After merging,** if W43-LIBC-RESULT-ALIAS adds v1 declarations, run
  `python3 providers/contracts.py sync` so that the libc pin, the counts and
  the "every v1 declaration is manifest-selected" test agree.

**Coordinator's question: can a parent's `resource_return_alias(p, …)` or
`resource_return_interior(p, …)` be inherited?** W43-LIBC-RESULT-ALIAS adds
these two predicates on `main`, so they do not exist on this card's base.
Nothing here declares or tests such a wrapper. By construction the answer
should be yes, under two conditions.

* **Why it should work.**
  * `rename_clauses` replaces every word token equal to a parent parameter in
    every clause expression, whatever the predicate. So `p` in the first
    argument becomes the child's parameter at the same position.
  * The capture check refuses a child parameter name that occurs as any other
    word in the parent's clauses. That includes the predicate's second
    argument, if that argument is a word rather than a number.
  * The v1 agreement gate then requires the loaded child record to **equal**
    the parent's.
* **The same shape is already tested.** The CONTRACT-INHERIT fixture inherits
  `libc::realloc`, whose `resource_return_reallocated(ptr, nullable)` puts a
  parameter word first in a result predicate and loads to an index-bearing
  `ResourceReturnMode::Reallocated { parameter, .. }`.
* **Condition 1: the new modes load by index.** If W43-LIBC-RESULT-ALIAS
  records the aliased parameter **by index**, an equal record means the same
  position, and inheritance is exact.
* **Condition 2: a name must not survive.** If a new mode kept the parameter
  **name** in the loaded record, the gate would refuse every renamed child with
  `QI012`. That fails closed, but no alias wrapper could then be inherited.
* **A second parameter-reading position.** If the second argument `n` can name
  a parameter, it is renamed like any other word. If it is a literal or
  keyword, it is copied unchanged, unless it coincides with a parent parameter
  name. In that case it is renamed, and the gate's structural comparison
  decides, exactly as `positional_reading_can_diverge` describes.

This is analysed, not exercised: the combined tree needs one test. The
integrator's re-arbitration should add a load-path inheritance of a
`resource_return_alias` parent, for example a fixture wrapper of
`libc::memcpy`, with the realloc fixture as its model. Until that test exists,
the claim is recorded here rather than asserted, and registered as
`CONTRACT-INHERIT-ALIAS-RESULT-PARENTS`.

## Reproduction

```sh
python3 providers/contracts.py sync        # load path + inventories (check: never writes)
python3 docs/reports/w43-return-expression/scripts/run_audit.py <out> <commit>
python3 docs/reports/w43-return-expression/scripts/baseline_identity.py <out> docs/reports/w43-libc-contracts/artifacts --root-alias <this-root>=<w43-root>
python3 docs/reports/contract-inherit-adoption/scripts/arbiter_compare.py <left> <right> <out.json>
python3 docs/reports/contract-inherit-adoption/scripts/census.py . <out.json>
python3 docs/reports/contract-inherit-adoption/scripts/wrapper_census.py . <fluent-bit-source> <out.json>
target/release/examples/declared_abi_census providers/quirin-provider-libc/contracts/libc/string.qv <written-out wrappers> -- <185 units>.i
python3 docs/reports/contract-inherit-adoption/scripts/update_ledger.py   # once
```
