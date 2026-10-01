# CG30-DIALECTS-FLAGS-TYPES: dialect, target and type identities, and refused extensions

Card `CG30-DIALECTS-FLAGS-TYPES` of the
[Clang gap card queue](../../plans/clang-gap-cards/README.md) (wave 48, round 10),
entries I01 I02 I03 I05 N12 N13 N14, routes G and T and the compile-database
reader. Base: local `main` `b6a3dc42`.

- **Rule:** [gateway 11y](../../agents/gateway.md#constraint-11y).
- **Lean:** `QCore.DialectTargetIdentity`, `proof/AuditDialectTargetIdentity.lean`
  (18 theorems).
- **Prediction:** [PREDICTION.md](PREDICTION.md), frozen in `85b3efdc` before any
  AFTER run (the code was written while the BEFORE build and census ran; the
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
way (`arm64`, `x86_64` admitted; `i386`, `armv7*`, `arm64_32` refused). There
is no passthrough. `compiler_options_compatibility` (the pinned Cycle 19 route)
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

MEASUREMENTS_PLACEHOLDER
