# Wave 44 increment records

[Development table of contents](../../../DEVELOPMENT.md) · [History index](README.md)

> Counts and observations describe their recorded dates. Current rules and
> headline status live in [AGENTS.md](../../../AGENTS.md) and
> [STATUS.md](../../STATUS.md); each report owns its evidence. The preceding
> [record](wave-43.md) holds the wave-43 paragraphs and stays unchanged.

**`W44-LIBC-RESIDUALS` (2026-09-25).** Worked the four libc-side residuals beside the largest libc leaf masses, measuring each before building it, and re-measured on main `5355998d` after merging W44-INDIRECT-CHAIN-ACCESS and W44-CORPUS-CALLEE-SUMMARIES. glibc's header-inline `__bswap_16/32/64` and `__uint16/32/64_identity` are **checked, not declared**: W44-CORPUS-CALLEE-SUMMARIES measured that their `static __inline` bodies validate under W4, so this card lifted that card's reserved-name filter in `legacy_callee_scope.rs`. A reserved-name `static` definition the unit carries enters the derivation scope on the linkage premise alone, and a name the unit only declares never does (Lean `QCore.CalleeSummaryScope`: `reservation_is_not_read`, `an_undefined_name_is_never_in_scope`). An earlier revision declared them in a `libc/byteswap.qcore` and taught the reader `__extension__`; both were dropped, and that route's arbiter is kept under the report's `route-a/`. The declared-ABI reader projects `__builtin_va_list` at a parameter to a new parameter-only, borrowed-only loader spelling `va_list` that names the type rather than its x86-64 or AArch64 layout (Lean `QCore.GnuBuiltinReader`; `AbiType` gains `vaList`); it moves 0 of 13,101 manifest census rows. A new tail premise, T6 (`variadic_nested::is_literal_or_predefined_actual`, Lean `QCore.LiteralTailActual`), admits a tail actual that is only literals and C11 6.4.2.2 predefined identifiers whatever the function's declarations or C02-P's closure say; the closure had marked literals risky where one reached `__assert_fail`. Declared in `providers/quirin-provider-libc`: `vsnprintf` and `vprintf` (`libc/stdio.qcore`, 168/168 units agree) and `strerror` (`libc/errno.qcore`, 181 units; kept out of the published `libc/string.qcore`). Staged arbiter, every leaf attributed and the stages composing exactly: T6 −974, header bodies −627/+5, `va_list` −26/+1, `strerror` −2; 1,629 removed and 6 added. Functions 296 / 85 / 4,453 → **299 / 85 / 4,450** (`ut_put_be16/32/64`); units, leak 271 and `unknown_alias` 4,420 unchanged; `unsupported` 45,721 → 44,098; no violation candidate. Retired `W43-LIBC-HEADER-INLINE-BODIES`, `W44-CALLEE-SCOPE-RESERVED-BODIES`, `LIBC-PRINTF-VA-LIST`, `LIBC-PRINTF-TAIL-ACTUALS` and `LIBC-PRINTF-NESTED-TAIL-CALLEES`; registered `W44-TAIL-NAME-ACTUALS` (226), `W44-PRINTF-NESTED-INNER-CALLEES` (87) and `W44-VA-START-END-BUILTINS` (127 in 51 functions); 358 → 356. [Report](../../reports/w44-libc-residuals/README.md).

**`W44-CORPUS-CALLEE-SUMMARIES` (2026-09-25).** Measured why corpus-defined callees get no validated W4 summary, then closed the one route gap. A capture of every unit's preprocessing and a census build that re-derives W4 over a widened scope on a private copy classify the 9,547 `call to uncontracted function` leaves of Unknown functions: 7,780 sit on callees whose own body does not validate (same unit, another pinned unit, or a header `static` body), 950 on callees with no pinned body (cfl, builtins, OpenSSL), 586 on the glibc `__bswap_*` bodies W44-LIBC-RESIDUALS owns, 104 on another unit's `Safe` body (no cross-unit route), 86 on external-linkage `mpack_*` inline definitions, and 41 on header `static` helpers whose bodies validate but which W4 never saw, because under main-source selection the derivation scope was the reported scope. `legacy_callee_scope.rs` now adds to the derivation scope, and only to it, every unselected definition the selection reaches that is `static` and not `extern` on its own definition and not reserved in every context; reported rows, the entry and the whole-unit `Safe` scope are unchanged, summaries still pass `observations_match_contract`, and declared contracts still win (gateway constraint 4a; `QCore.CalleeSummaryScope`, 11 audited theorems). BEFORE reproduces the baseline byte for byte; AFTER changes 70 leaves, all attributed (42 decided at 16 validated callees, 13 same-line `unknown_alias` at effectful call sites, 11 condition or indirect-access refusals on statements calling a decided callee, 4 re-attributions), moves `mk_event_init` and `mk_event_backend` Unknown → Safe as predicted, moves no unit, and keeps leak 271. A releases-its-formal destructor summary was not admitted, since only 8 of 230 destructor-shaped callees have a body blocked by calls alone; it and four other causes are registered with their sizes, and the reserved `__bswap_*` bodies, which validate, are handed to W44-LIBC-RESIDUALS. Follow-ups 345 → 351 on the card branch, 352 → 358 on integration. [Report](../../reports/w44-corpus-callee-summaries/README.md).

**2026-09-25 — W44-INDIRECT-CHAIN-ACCESS: typed record-pointer member chains.**
W43's typed statement route and typed condition read now accept a member chain
`r->l1->…->f`, `(*r).l1->…->f` or `s.l1->…->f` as an access. A chain has two
to four selections, every link exactly `struct POINTEE *`, and a scalar final
member. The chain rides on `IndirectScalarAccess` (a `chains` field, no new
variant), and each pointer it reads through carries an obligation:

* a root that is never bound and unbound or non-null is untracked;
* a root that is one live identity is tracked;
* a link of an untracked owner is untracked;
* a link of a tracked owner must be the member place an admitted store bound
  to one live linked child. Otherwise it records `UnknownAlias`.

The census of the instrumentation commit reproduces BEFORE byte for byte, and
W43's classifiers reproduce both ledger sizes at base (5,565 statement and 1,248
condition leaves). The census also shows that 3,564 of those 6,813 leaves carry
no member chain. The rule's recogniser admits 74 leaves. The measured outcome
is exactly as forecast:

* 24 leaves are removed;
* 50 are exchanged for 49 root refusals;
* 0 are downstream;
* 0 function or unit moves;
* leak 271 unchanged;
* 0 violation candidates.

Lean `QCore.IndirectChainAccess` (36 theorems) is audited by
`AuditIndirectChainAccess.lean`. Six gateway pins move deliberately. The card
retires `W43-INDIRECT-CHAIN-ACCESS` and `W43-INDEX-TRAVERSAL-CONDITION-CHAIN`
and registers nine measured residuals; the largest real one is nested
aggregates (1,287 leaves). See
[the card report](../../reports/w44-indirect-chain-access/README.md).
