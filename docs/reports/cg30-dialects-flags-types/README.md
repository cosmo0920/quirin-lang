# CG30-DIALECTS-FLAGS-TYPES: dialect, target and type identities, and refused extensions

Card `CG30-DIALECTS-FLAGS-TYPES` of the
[Clang gap card queue](../../plans/clang-gap-cards/README.md) (wave 48, round 10),
entries I01 I02 I03 I05 N12 N13 N14, routes G and T and the compile-database
reader. Base: local `main` `b6a3dc42`; BEFORE measured there. Rebased onto
main `8e8ed41f` (CG17), `06ad81cb` (CG20) and `b934c560` (CG25), where the
published AFTER was measured (§3), then onto `f0bbc0eb` (CG26-VARIADICS),
where the tip was compiled, tested and gated but **not re-measured** (the
arbiter is serialised under the shared lock; CG26's changes are its own).
Commit names below are those at measurement time.

- **Rule:** [gateway 11y](../../agents/gateway.md#constraint-11y).
- **Lean:** `QCore.DialectTargetIdentity`, `proof/AuditDialectTargetIdentity.lean`
  (18 theorems).
- **Prediction:** [PREDICTION.md](PREDICTION.md), frozen in `85b3efdc` (`526cabc1`
  after the rebase) before any AFTER run (the code was written while the BEFORE build and census ran; the
  file says so).
- **Published guarantee (Phase 4 choice).** The property this card publishes is
  *evidence-identity separation*: a verdict, certificate or cached result is
  bound to the exact dialect (with its registered AST differences), the
  explicit target triple and ABI, and is never reused under another. Its scope
  is the configured route (`compiler_options`) and the gateway's certificate
  request digest; the pinned Cycle 19 route is outside it (it reads no target
  word, unchanged). No extension semantics is published: every extension
  family is refused by name.

## 1. What changed

All new logic is in `crates/qcore-c-gateway/src/c_frontend_identity.rs`, with
thin hooks. Shared items, named for other cards: **`CTargetIdentity`** (with
`CTargetTriple`, `CAbiName`, `CDataModel`; card CG20 keys its integer widths on
it, rule 11q), **`CTypeId`**, **`CExtensionFamily`**, **`DialectDifference`**,
`dialect_identity_bytes`, `CCompilerDialect::{std_name, from_std_name}`.

**Dialects (I01, I05).** `CCompilerDialect` gains `C17`, `C23` and `C2y`, named
exactly (aliases such as `c18`, `c2x`, `gnu23` stay refused). Each dialect's AST
differences from the C11/GNU baseline are registered (`C23_DIFFERENCES`, 17:
`nullptr`, `nullptr_t`, `true`, `false`, `bool`, `constexpr`, `typeof`,
`typeof_unqual`, `auto`, `static_assert`, `alignas`, `alignof`, `thread_local`,
`[[…]]`, `_Decimal32/64/128`; `C2Y_DIFFERENCES`, 6 more: `_Countof`, `countof`,
`0o` literals, named `break`/`continue`, a declaration in an `if`/`switch`
condition, `_Generic`). The registry joins the evidence identity:
`dialect_identity_bytes` replaces the request digest's dialect byte (the four
baseline dialects keep their historical byte exactly; C17 = 4, C23 = 5 + the
registry, C2y = 6 + the registry), and `CompileTaskIdentity` gains
`dialect_differences_sha256`, serialised only when present, so no earlier
digest moves. None is adopted:

- *Route G* (`legacy_dialect_hook.rs`, hooked right after the reserved-spelling
  escape in `analyze_translation_unit_under_holds`): a definition whose
  declaration or body spells a registered difference has its body replaced by
  one call to `qcore_dialect_refused__KEY` (a new reserved prefix, escaped in
  source) before the holds, the ABI gate, W4 or replay read the stream. The row
  is `Unknown` with a named leaf and no summary. Measured reason for the
  withdrawal rather than an added leaf: under C23 the token reader reads
  `nullptr` as an unknown identifier, so `if (p == nullptr) return; q_free(p);`
  would publish a leak `Unsafe`; withdrawn, it is `Unknown`
  (`c23_nullptr_is_refused_by_name_and_never_misread`). A digit separator
  (`1'000`), which the token reader would lex as a character literal, refuses
  the unit.
- *Route T* (`typed_source.rs`): the registered node kinds
  (`CXXNullPtrLiteralExpr`, `CXXBoolLiteralExpr`), the `constexpr` flag, the
  `_Countof` trait and the registered type spellings are refused by name. `nullptr`
  is a registered C23 difference, not excluded for its `CXX` name (I05).

**Targets (I02).** `compiler_options` reads `--target=`/`-target` and `-mabi=`
into closed vocabularies (`CTargetTriple::parse`: 2–4 plain components, a
recorded architecture, OS and environment; `CAbiName::parse`: 18 names with
their data-model consequence), each at most once, and transports them as
`CFrontendFlag::Target`/`Abi` on every Clang invocation. Being frontend flags,
they are serialised into the request digest and the compile-task identity: a
changed target is a different identity, and a certificate minted for one
target fails `check_legacy_safety_certificate_for` against another's expected
context (`a_target_changes_the_evidence_identity`). Admission is fail-closed
on the data model: only LP64 (or no explicit selection, the default target's
standing premise) is admitted, because the readers take LP64 as a premise
(the ABI reader's projection table states it). ILP32 (`i686`, `x32`,
`-mabi=ilp32`), LLP64 (`*-windows-*`), an unrecorded component and a doubled
selection are refused as `unsupported compilation target`/`ABI`
(environment category, `REASON_RULES`). The existing `-arch` is checked the same
way (`arm64`, `x86_64` admitted; `i386`, `armv7*`, `arm64_32` refused). Route
G (in its first hook, before any reader) and route T admit a unit's own
frontend flags again (`CTargetIdentity::admit_flags`), so a caller building a
`C11TranslationUnit` directly is held to the same premise
(`a_unit_selecting_a_non_lp64_target_is_refused_by_the_gateway`). After the
rebase onto CG20 (`c_numeric`, rule 11q): CG20's numeric route already queries
Clang with the unit's frontend flags, so it keys on the transported target;
its one default-target consumer, the ABI reader's `plain_long_is_lp64`, is
never asked about a unit whose own selection fixes another data model, which
discharges `CG20-ABI-READER-ARCH-FLAG` (retired here; an `-arch i386` unit is
refused, by Clang on a Linux host and by the gateway's admission elsewhere).
There is no passthrough. `compiler_options_compatibility` (the pinned Cycle 19 route)
is unchanged.

**Types (I03).** `CTypeId` is Clang's canonical spelling, whitespace collapsed,
`bool` read as `_Bool`, an arithmetic multiset in Clang's own order (`long
unsigned int` = `unsigned long`). Route G (`typed_access_path::type_id`) and
route T (`typed_source::canonical_type_id`) read it from the AST node; the
lexical ABI reader (`declared_abi::declared_type_id`) reaches the same identity
from a specifier multiset through its own typedef resolution
(`routes_g_and_t_read_the_identity_clang_prints`: `my_size` → `size_t` →
`long unsigned int` is one identity on all three). The identity is a key:
Clang still decides compatibility, and no reader's admission changes (T's
`result typedef outside unsigned scalar profile` stands).

**Extensions (N12, N13, N14).** `CExtensionFamily` classifies complex
(`_Complex`, `_Imaginary`, `__real__`/`__imag__`, imaginary literals),
vector (vector/ext-vector/matrix/sizeless types, shuffle/convert/element
nodes), bit-precise (`_BitInt`, `_ExtInt`) and fixed point. Route G adds
``C extension type `…` (FAMILY) is not adopted (CG30)`` to a selected
definition whose typed AST (`StatementIndex`) holds one, read through G's own
`type_id`; the AST is read only where the unit's tokens spell an extension, and
a definition whose AST cannot be read there is refused too. Every other leaf is
kept (an existing validated `Unsafe` stays). Route T refuses first, by name.
None is adopted.

## 2. The cited claims

CG00's controls for these entries passed on the base tree (`clang_gap_phase4`
33 passed, `clang_gap_flags` 4 passed, release profile,
`private/cg30dft/before-controls-*.log`), so each refusal reproduced:

| Entry | Claim | At the base |
|---|---|---|
| I01 | the dialect set is C11 and three GNU modes | reproduced (`-std=c17/c23/gnu23/c2y` refused; `nullptr` refused by Clang under C11/GNU17) |
| I02 | `CFrontendFlag` is an allowlist without target/ABI | reproduced (`--target=`, `-target`, `-mabi=lp64`, `-m32` refused) |
| I03 | readers do not canonicalise types; T is limited to its tables | reproduced (`typedef int myint` result refused by T) |
| I05 | no C23 dialect; T's allowlists do not support C23 forms | partly corrected by CG00 (T admits GNU `__typeof__`; G refuses it); reproduced as recorded |
| N12 | T lacks complex nodes; G's scalar classification excludes `_Complex` | T half reproduced; G half is a **correction**: G's refusal is the assignment route's (`unsupported assignment expression`), not a scalar-type classification |
| N13 | vector nodes are outside T's and G's allowlists | reproduced (G: `unsupported assignment expression`) |
| N14 | `_BitInt` refused by the scalar table | **correction** (recorded by CG00): G's refusal is the indirect-access family, not a scalar classification |

The census ([artifacts/census.json](artifacts/census.json),
`scripts/census.py`) found no C17/C23/C2y unit and no target, ABI, `-m32`/`-m64`
or `-arch` word in any measured compile database (C99 and GNU90 commands stay
refused). The analysis's suggestion to add `--target`/`-mabi` "as needed" has
no measured demand in these inputs; they are admitted for the identity
property, not for coverage.

## 3. Measurements

`scripts/measure.sh` (corpus `66910c10` `--jobs 4`, system, tail-pause slice),
`scripts/compare.py` (status per `(file, function)`, as the earlier cards
count, and leaf deltas); [artifacts/](artifacts/). The BEFORE binary was built
from an exported tree of the base (an archive of `b6a3dc42` under
`private/cg30dft/base-tree/`), because this worktree's sources were already
changing; the system and slice reports embed that tree's path in provider
paths, so their digests are compared after removing it (`normalised`).

| Run | Binary | Corpus | System | Slice |
|---|---|---|---|---|
| BEFORE, base `b6a3dc42` (`before2`, `before-system`, `cg30dft-before`) | `2ce2a9f0…` | 629 / 10 / 4,195 `3d53a5d0…` (= baseline) | 44 / 2 / 188 (normalised `048f4b6c…`) | 29 / 1 / 239 / 1 P (normalised `0221a527…`; function-for-function identical to CG05's `after3`) |
| AFTER, `85b3efdc` (`after1`, superseded) | `d7b61fde…` | 626 / 10 / 4,198 `204bf259…` | not completed | not completed |
| AFTER, `91b46c9d` on `b6a3dc42` (`after3-system`, `cg30dft-after3s`) | `85dfce8e…` | lost (disk exhausted, then stopped for the lock) | 44 / 2 / 188, no function row changed | 29 / 1 / 239 / 1 P, +1 leaf (below) |
| **AFTER, tip `01b19037` on main `b934c560` (`final`, `final-system`, `cg30dft-final`)** | `13bb0797…` | **630 / 10 / 4,194** `638947d5…` | **44 / 2 / 188**, no function row changed (`1ccaab6a…`) | **29 / 1 / 239 / 1 P**, +1 leaf (`48b12d16…`) |

**Attempts kept, not evidence.** The first BEFORE corpus run (`before`) failed
the libevent pin check (`Pin(GitUnavailable)`: the exported tree's submodule
had no readable gitdir); its system and slice runs completed and are the
BEFORE above. `before2` re-ran the corpus with the submodule's gitdir named
absolutely. One AFTER build (`build-after2-enospc.log`) failed with `No space
left on device` while the host's disk was exhausted by other work; it was
rebuilt as `after3` once the disk recovered above the floor, and `after3`'s
corpus was lost to the disk again (`after3-enospc`: a parity sink write failed)
and its re-run stopped when the coordinator serialised heavy steps under a
shared lock. The published AFTER (`final`) was measured on the last rebase, in
one acquisition of that lock.

**The superseded AFTER (`after1`) and the fix it forced.** The first AFTER
moved three corpus functions Safe → Unknown (`rdhttp.c`:
`rd_http_error_destroy`, `rd_http_is_failure_temporary`,
`rd_http_req_get_code`) and added `C extension types could not be excluded:
the definition's typed AST is unavailable (CG30)` to all 21 of that unit's
definitions. None of the three uses an extension; the cause is that
`rdhttp.c`'s translation-unit JSON nests 223 levels deep, beyond
`serde_json`'s recursion limit of 128, so the unit has no AST index at all
(every typed route of G declines there too). That was an over-refusal, not a
correction, so the hook now reads such a definition's own filtered AST
(`typed_access_path::definition_ast`, Clang's `-ast-dump-filter`), and refuses
only when that fails too (`a_unit_too_deep_to_index_is_read_per_definition`).
Prediction item 4 ("0 unavailable") was wrong on `after1` for exactly this
reason; it is recorded, not amended.

**The published AFTER and its attribution.** Main moved under the card three
times (CG17 `8e8ed41f`, CG20 `06ad81cb`, CG25 `b934c560`). CG17 and CG20 each
measured their routes byte-identical to their base; CG25 published its
corpus changes ([its corpus-changed.json](../cg25-resource-kinds/artifacts/corpus-changed.json):
5 functions, `rd_regex_match` Unknown → Safe) and no system or slice row
change. `final` against this card's BEFORE on `b6a3dc42`
([artifacts/corpus-compare.json](artifacts/corpus-compare.json)) is exactly
CG25's five rows plus this card's own:

- **Corpus:** no status move of this card's; **+4 leaves** `C extension types
  could not be excluded: the definition's typed AST is unavailable (CG30)` in
  four already-Unknown `rdhttp.c` definitions (`rd_http_get`,
  `rd_http_post_expect_json`, `rd_http_req_init`, `rd_http_ssl_configure`),
  whose own filtered AST also nests beyond the 128 limit. Totals 630 / 10 /
  4,194, equal to main's.
- **System** ([artifacts/system-compare.json](artifacts/system-compare.json)):
  no function row changed; the only differing fields are the per-unit
  `resource_contract_sources` and `inventory_sha256` provenance digests (the
  contract sources' absolute paths, and CG25's libc contract edits).
- **Slice** ([artifacts/slice-compare.json](artifacts/slice-compare.json)): no
  status move; **+1 leaf** in `flb_skip_leading_zeros_simd` (Unknown before
  and after): ``C extension type `__attribute__((__vector_size__(2 *
  sizeof(long long)))) long long` (vector) is not adopted (CG30)``, plus the
  same provenance digests.

**No correction:** no Safe → non-Safe move, no Unknown → Safe, no leak or
violation added. The extension and dialect paths add refusals only; no
measured unit uses a C17/C23/C2y dialect or a target word, so those paths are
exercised by the tests alone. The wall time of the corpus arbiter did not
change measurably (79 and 81 minutes for `before2`/`after1` at load ~80;
`final` took 9 minutes at load ~5).

## 4. Prediction against outcome

| Prediction | Outcome |
|---|---|
| 1. Dialects and targets move nothing | held: no measured unit asks for either |
| 2. The extension prefilter fires almost everywhere | held (glibc's `_Complex` typedefs) |
| 3. Vector policy by measured demand: none of the five SIMD definitions Safe ⇒ refuse, leaves only | held: none is in the corpus or system selections; the slice's one (`flb_skip_leading_zeros_simd`) is Unknown and gains the one leaf; vectors refused |
| 4. Complex/`_BitInt`/fixed point move nothing; **0** unavailable-AST refusals | **missed**: `after1` refused all 21 `rdhttp.c` definitions (3 Safe → Unknown) for an unreadable unit AST; after the per-definition fallback, 4 remain (all Unknown, leaves only) |
| 5. Totals stay; no Safe → non-Safe | held on the published AFTER (the main's totals, which moved only by CG25) |

## 5. Tests

- `crates/qcore-c-gateway/tests/cg30_dialects_flags_types.rs` (9, Clang):
  a unit reads alike under C11/GNU17/C17/C23/C2y with five distinct request
  digests; C23 `nullptr` refused by name and never a misread `Unsafe`; C23
  keywords and attributes; C2y `_Countof`; a digit separator refuses the unit;
  `_BitInt`, `_Complex` and a vector refused by name on G and T; a sibling's
  extension leaves a definition alone; a too-deep unit read per definition; a
  target changes the request digest and a certificate for one target fails
  another's expected context.
- `crates/qcore-c-gateway/tests/unit/c_frontend_identity.rs` (10): dialect
  bytes and registry, token patterns, digit separators, target and ABI
  admission, `CTypeId` across spellings, the ABI reader's typedef resolution,
  G/T/ABI agreement on Clang's spellings, extension classification.
- `crates/qcore-audit/tests/cg30_target_identity.rs` (3, Clang): structured
  selections transported and preprocessed; every refusal by name and
  classified by `ReasonCategory`; seven commands, seven compile-task digests.
- Flipped CG00 controls (BEFORE → AFTER): `i01_the_dialect_set_is_seven_explicit_identities`,
  `i01_dialects_outside_the_seven_are_refused`, `i02_target_and_abi_flags_are_structured`;
  `n12_complex_values_are_refused` and `n14_bit_precise_integers_are_refused`
  now also assert the named refusal (T's text changed; G keeps its old leaf
  beside the new one). `reserved_spelling`'s every-prefix control covers the new
  marker; `reason_codes` covers the new reason strings.

## 6. Axes (the analysis's §8)

Covered: dialect ∈ {C11, GNU99, GNU11, GNU17, C17, C23, C2y} (named; the three
new ones refuse every registered difference); target ∈ {default, LP64 triple,
LP64 `-mabi`, LP64 `-arch`} admitted, {ILP32, LLP64, unrecorded} refused;
type identity over arithmetic multisets, qualifiers, pointers, tagged types and
typedef chains on G, T and the ABI reader; extension ∈ {complex, vector,
bit-precise, fixed point} refused on G and T. Still refused: every C23/C2y
difference, every extension, every non-LP64 target, `-m32`/`-m64`, GNU23 and
the C99/GNU90 commands of the measured databases.

## 7. Lean

`QCore.DialectTargetIdentity` (`proof/QCore/DialectTargetIdentity.lean`,
registered in `proof/QCore.lean`; `proof/AuditDialectTargetIdentity.lean`, 18
theorems): `dialectByte_injective`, `distinct_dialect_distinct_bytes`,
`baseline_bytes_unchanged`, `c2y_extends_c23`; `lookup_minted_under_key`,
`no_reuse_across_dialects`, `no_reuse_across_targets` (a cache keyed by the
evidence identity answers only with a verdict minted under that key);
`admit_iff`, `ilp32_abi_refused`, `llp64_target_refused`;
`withdrawn_is_unknown`; `refusal_never_safe`, `refusal_keeps_unsafe`,
`refusal_is_monotone`; `perm_same_typeId`, `bool_is_Bool`,
`long_unsigned_int_vector`, `distinct_counts_distinct`. No placeholder, no
project axiom (all 264 audits pass). Premises stated, not proved: SHA-256
collision freedom for the Rust digests and faithful Clang extraction (trust
boundary 28); the Lean names are cited by the Rust unit tests that pin the same
tables.

## 8. Follow-ups

`CG30-DIALECTS-FLAGS-TYPES` is amended (not retired) with the
residual adoptions and their sizes (`scripts/update_ledger.py`): C23/C2y
differences (0 units ask for those dialects), extension families (vectors in 5
Fluent Bit definitions, none Safe; `_Complex` only in glibc typedefs;
`_BitInt`/fixed point 0), non-LP64 targets (wait for CG20; 0 target words),
reader decisions by `CTypeId` (T's typedef refusal stands) and the pinned
route's unread target words (constraint 11). Registered, out of scope:
`G-TYPED-AST-INDEX-DEPTH-LIMIT` (route G has no typed AST index for a unit
whose JSON nests beyond 128: `rdhttp.c`, 21 definitions, every typed route of
G declines there; this card only reads its own check per definition).

GATE_PLACEHOLDER
