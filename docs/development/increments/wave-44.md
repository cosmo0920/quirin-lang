# Wave 44 increment records

[Development table of contents](../../../DEVELOPMENT.md) · [History index](README.md)

> Counts and observations describe their recorded dates. Current rules and
> headline status live in [AGENTS.md](../../../AGENTS.md) and
> [STATUS.md](../../STATUS.md); each report owns its evidence. The preceding
> [record](wave-43.md) holds the wave-43 paragraphs and stays unchanged.

**`W44-LEAK-PIN-AUDIT` (2026-09-25).** Audited every one of the 271 baseline leak leaves on base `c1e69bb5`. A feature-gated leak-witness census re-runs each settled CFG and searches the analyzer's own witness paths; its diagnostics stay byte-identical to the baseline, and all 271 leaves join. With source reading ([reading.json](../../reports/w44-leak-pin-audit/reading.json)) the leaves classify as 139 validated (a), 31 projection artifacts (b), 101 premise or approximation gaps (c) and 0 unclassifiable, frozen before any rule. A carrier-preserving cast return is now decided as the uncast return and a carrier-crossing one is refused (`RETURN-CAST-PLACE-NOT-ESCAPED`, retired; `QCore.CastReturnEscape`); a first AFTER run was withdrawn because its cast rule admitted member chains the uncast return refuses. On the coordinator's direction (`W44-REFUSED-HANDOFF-LEAK-CLAIMS`) the card then implemented the refused hand-on rule: a live identity named in a refused statement, as a store value, return operand, call argument, operand or address, carries an unresolved effect, so no leak is claimed through it; it discharges nothing and makes nothing Safe (`QCore.RefusedHandOnCustody`). A rule-off census with the union-join handed path found 252 of the 270 other leaves named in a refused statement before their exit; the overlay and its prediction were committed before the rule. The arbiter withdrew exactly the 253 predicted leaves (leak 271 → 18, 0 collateral, 0 missed): 76 functions and 35 units provisional Unsafe → Unknown, 3 accessor functions Unknown → Safe through the cast return; unknown_alias +19 / −3, the three removals not localised. The cost is 128 withdrawn leaves the reading validated, registered as `REFUSED-HANDOFF-WITHDRAWN-ACCUSATIONS`; 11 class (a) leaves in 3 functions remain. The three leak semantics W44-INDIRECT-CALL-ARGUMENT-ACCESS exposed (an unsanctioned `*out = p`, a link into a returned owner, a release through an uncontracted helper) are pinned and fall under the same rule, and CT43's owned-local control moves from a provisional Unsafe to Unknown (`CT-CTWIN-UNRESOLVED-LEAK` discharged). Eleven residuals registered, three entries amended, ledger 345 → 355. [Report](../../reports/w44-leak-pin-audit/README.md).

**`W44-BRANCH-CONDITION-CALLS` (2026-09-25).** Made calls inside `if`, `while`, `for`, `do`-`while` and `switch` conditions decidable where every call is modeled. A condition every existing route refused is desugared (`legacy_condition_desugar.rs`, a child module of `legacy_c` behind thin hooks): each call becomes its own `ContractCall` from the ordinary statement classifier, in C evaluation order, each operand decides through one branch, and `&&`, `||` and `?:` become edges, so a later operand's calls run only on the edge where C evaluates them; `!` swaps exits and `__builtin_expect(E, K)` reads as `E`. `(NAME = CALL(...))` into a plain declared scalar or pointer local becomes the statement and a test of `NAME`. Uncontracted calls, custody results, same-component local callees, unsequenced effectful pairs, type operators and unadmitted operands keep the refusal, and `if ((p = malloc(n)) == NULL)` keeps today's exact lowering. The census came first (`df5edb61`, rule compiled in and not consulted, byte-identical): most call-bearing conditions fail on a call the classifier refuses (an indirect argument, 1,686 leaves at AFTER; an uncontracted callee, 1,000) or on a call-free operand (1,211), not on a connective. Lean `QCore.ConditionCallDesugar` (25 audited theorems) proves the plan performs exactly C's short-circuit evaluation for every operand valuation. The arbiter removed **292** condition leaves (276 forecast on Unknown functions, exact), exchanged 36 into same-line `unknown_alias` (`unlikely(!x)` on an unresolved name) and decided one false leak on `rd_malloc`'s null edge in `snappy.c`: **0 function or unit moves, leak 271 → 270, 0 violation candidates**. It retired `W43-INDEX-TRAVERSAL-CONDITION-CALL` and registered eight blocker-sized residuals, a derived-result decision and a B02 descriptor-read finding (345 → 354 on the card branch, 358 → 367 on integration). [Report](../../reports/w44-branch-condition-calls/README.md).

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
