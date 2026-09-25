# Wave 44 increment records

[Development table of contents](../../../DEVELOPMENT.md) · [History index](README.md)

> Counts and observations describe their recorded dates. Current rules and
> headline status live in [AGENTS.md](../../../AGENTS.md) and
> [STATUS.md](../../STATUS.md); each report owns its evidence. The preceding
> [record](wave-43.md) holds the wave-43 paragraphs and stays unchanged.

**`W44-COLLECTION-ALLOCATION` (2026-09-25).** Admitted the collection allocation `v = malloc(n * sizeof(*v))`, discharging `W43-COLLECTION-SIZEOF-DEREFERENCE-ALLOCATION`. The census came first ([report](../../reports/w44-collection-allocation/README.md), frozen [prediction](../../reports/w44-collection-allocation/PREDICTION.md)): 288 `(*NAME)` operands and 4 `(**NAME)`; 226 `indirect memory access` leaves whose only access is such an operand; and the `C operator sizeof` family (465 leaves) turned out to be `(size_t)(sizeof(_v))` casts, not dereferences. The new module `sizeof_operand.rs` treats `*NAME`, `**NAME` and `NAME[LIT]` as unevaluated (C11 6.5.3.4p2) when every declaration of `NAME` is a pointer declarator with enough stars, no array dimension and a specifier `never_variably_modified` resolves. Block-scope VLA typedefs, nested declarators, arrays and unresolved names refuse. Two thin hooks in `legacy_c.rs` apply only in a statement whose first call is an ordinary callee: the lexical gate reads the operand as a marker, and the protected-name mention of an argument skips it. No transfer rule or instruction is added. `QCore.SizeofOperand` (23 theorems) proves an admitted operand is never evaluated under the faithful-extraction premise, and that the gate view only withdraws refusals. On its own base the card raised the leak pin 271 → 396, with 109 of the 125 added leaks resting on identities handed to refused statements; it was held until W44-LEAK-PIN-AUDIT's refused hand-on rule landed. Merged with main `70ce6b44` (merge `aa86cc39`), BEFORE reproduced the W44-LIBC-RESIDUALS baseline byte for byte. AFTER withdrew 204 operand refusals (230 leaves removed, 249 added), moved functions 300 / 9 / 4,525 → 304 / 11 / 4,519 and units 3 / 8 / 170 / 4 → 3 / 9 / 169 / 4, and moved leak 19 → 21. A rule-off leak-witness census of the merged tree finds 124 leaves on the card's allocations: the rule withdraws 122, and the 2 left have no refused hand-on. Both are unit-test assertion-failure returns that leak a `rd_calloc`'d fake `rk`, read as real. Collections: 9 of 14 `T **` allocations are no longer refused, yet 0 of the 27 element operations resolves and 0 functions are Safe through an element node. Ledger 377 → 379 (one retired, three registered).

**W44-INDIRECT-CALL-ARGUMENT-ACCESS** (2026-09-25, base `c1e69bb5`, merged
with main `70ce6b44` as `9c8b7809`). A census first: the call-argument family
is 11,069 leaves on 2,675 Unknown functions at the card's base, and 7,953 of
them hand the read to a callee with no contract. A simple statement the lexical
gate refused for an actual of a contracted call is classified again with every
typed read at a declared `Borrowed` position or in a declared variadic data
tail admitted (scalar values, pointer reads through `T **` bases and pointer
members, non-pointer sub-object addresses, `sizeof` of a type name as a
constant, no chain); when that is a contracted call whose named destination is
its whole left-hand side, it lowers to W43's typed read of every base, a new
`ArgumentReads` obligation and the call. Custody-taking positions and
uncontracted callees keep the refusal. Lean `QCore.CallArgumentAccess` (26
theorems). On the card's base it exposed eleven false leaks and found the
suffix-destination hole main has since repaired; on the combined tree the
refused hand-on rule withdraws all eleven. Combined-tree arbiter: 195 leaves
removed and 38 added, all attributed; `cio_file_hash_print` Unknown → Safe;
`mk_rconf_read` and its unit Unknown → provisional Unsafe on one class (c)
leak (`MK-LIST-ADD-BORROWED-PUBLICATION`, now modeled rather than refused);
leak 19 → 20; 0 use_after_release. Ledger 377 → 384.
[Report](../../reports/w44-indirect-call-argument-access/README.md).

**2026-09-25 — W44-TEST-SUITE-WALL-TIME: the gate's Rust suite, from 22 minutes to about 4½.**
Plain `cargo test --workspace --no-fail-fast` took 22:13 from a clean dev
profile: 125 s of build and 1,208 s of test binaries run one after another.
Three changes, each measured on its own, bring it to 3:59–5:35 over three runs
at 1-minute loads of 8–38, and the executed tests and outcomes stay identical
to the base:

* `[profile.dev] debug = "line-tables-only"` about halves the build and shrinks
  `target/` from 62 to 21 GB. Opt-level and the debug assertions are unchanged.
* The gateway now resolves and probes Clang once per `QCORE_CLANG`/`PATH`
  environment. Discovery had launched `clang --version` before every Clang
  run, 5,411 of the heaviest binary's 9,603 launches. The serial test phase
  falls from 1,345 s to 506 s. A failure is never cached, and no verdict can
  change.
* `scripts/run_test_binaries.py` replaces the serial run. It is std-only. It
  captures cargo's own launch of every test executable (argv, cwd and the
  complete environment, the same 114 variables strace observes) and runs the
  executables eight at a time, longest first. Constraint 16's load-sensitive
  rows, and every binary that launches a process provider under a timeout, run
  alone afterwards; none ever overlapped another unit. The runner fails closed
  on a crash, a signal, a missing or unlaunched binary, or an unparsable
  result.

`docs/agents/validation.md` now names the runner as the gate's Rust step and
keeps `cargo test --workspace --no-fail-fast` as the reference command. The
only test additions are the cache's two unit tests. Three measured residuals
are registered:

* `W44-CLANG-IDENTITY-RECAPTURE`;
* `W44-TEST-CRITICAL-PATH-ROUTE-PARITY`: the heaviest binary, 53 s alone, sets
  the end of the parallel phase;
* `W44-PROVIDER-SUITE-SERIAL`: the provider suite's 370 s is now the gate's
  longest Rust step. See [the report](../../reports/w44-test-suite-wall-time/README.md).

**`W44-C11-GENERIC-SELECTION` (2026-09-25).** The C gateway reads C11 generic selections (§6.5.1.1). A census of the 187 captured `-E -P` texts found **85** `_Generic` occurrences in 36 functions of 25 units, every one glibc 2.43's C23 const-preserving `__glibc_const_generic` expansion (`strchr` 51, `bsearch` 13, `strstr` 13, `strrchr` 6, `memchr` 2), none nested; 14 were the `call to uncontracted function _Generic` leaves and 71 sat in statements refused for other reasons, mostly conditions. Route 1 (selection invariance) is built in `legacy_generic_selection.rs`, called once from `build_function_cfg_with`: a selection whose associations are one expression up to enclosing parentheses and pointer casts is lowered to it and its control dropped, a removed cast requiring a direct call the unit declares with an object-pointer result (`declared_abi::declares_object_pointer_result`); anything else is left byte for byte. Route 2 (Clang's selection) was not built: it would read the typed AST that certificates must not depend on and admits nothing route 1 refuses here. Lean `QCore.GenericSelection` (`resolve_preserves_outcomes`, `the_lowered_effect_is_independent_of_the_selection`, `the_control_is_unevaluated`; `AuditGenericSelection.lean`). Arbiter on `70ce6b44`: BEFORE reproduces the W44-LIBC-RESIDUALS integration diagnostics byte for byte; 37 leaves removed and 34 added, all attributed; functions 300 / 9 / 4,525 → **301 / 9 / 4,524** (`mk_string_char_search`); units unchanged; `unsupported` 43,885 → 43,854, `unknown_alias` 3,357 → 3,370, **leak 19 → 34**: the 15 added are class (a) test-failure-path leaks of the already provisional-Unsafe `ut_sasl_oauthbearer_oidc_assertion_create`, whose `strchr` assertion conditions are now desugared; no violation candidate. Registered `W44-BSEARCH-UNDECLARED`; 377 → 378. [Report](../../reports/w44-c11-generic-selection/README.md).

**`W44-AUDIT-PARALLEL-UNITS` (2026-09-25).** Parallelised the corpus audit's per-translation-unit work without changing a byte. A base profile (frozen with the prediction before any parallel run) put 99.3 % of the pinned arbiter's run in its 185-unit loop, 87 % of that in-process analysis, and the largest unit, `librdkafka/rdkafka_admin.c`, at 46.4 s. `qcore_audit::parallel::run_ordered` computes units on `--jobs N` / `QCORE_AUDIT_JOBS` threads (default: available parallelism, capped at 16; `--jobs 1` is the sequential loop) and folds report fragments, tallies, sidecar results and console buffers in unit order, stopping at the first error in that order; the pinned route runs library preludes, units, postludes and the Monkey gate as ordered steps, and the configured route splits `CurrentAudit::audit_entry` into `evaluate_entry` and `absorb_entry`. Any active census sink forces one job and process-provider verification stays serial. Measured on a shared 8-core/16-thread host: 424–590 s sequential, 129 s at 4 jobs, 96 s at 8, 77–87 s at 16 (the configured route 466–546 s to 79 s); every archived artifact is byte-identical at jobs 1, 4, 8 and 16 (16 four times), `audit.log` `cmp`-identical, census streams identical at 1 and 16, and the configured route identical except its wall-clock `preparation:` figures. The 8- and 16-job predictions missed on the slow side (per-unit slowdown 2.35× under SMT, not 1.3–1.5×); one measured residual is registered, `W44-AUDIT-CRITICAL-UNIT` (that unit spans 84 of 86 s at 16 jobs). No analyzer semantics changed; no Lean change. [Report](../../reports/w44-audit-parallel-units/README.md).

**`W44-AGENT-GUIDANCE-OPUS55` (2026-09-25).** The agent-loop guides now follow
Anthropic's Opus 5 and 5.5 prompting guidance. `AGENTS.md`, `CLAUDE.md`, the
rule router, `operations.md`, `validation.md` and the topic-page headers lose
nine kinds of scaffolding: repeated reading instructions (the "read every
applicable rule" header was in 11 copies), bold emphasis stacking and duplicated
tool mechanics. Ten embedded incident histories move verbatim to a new
[checkpoint](structure-checkpoints/agent-cycle-guidance.md). Operations gains a
scope statement, a verification boundary (the gates are the verification), a
subagent delegation policy, report length calibration and a one-screen
card-brief standard. The standard covers completion conditions, legitimate
stops, continuation of unattended cards and time budgets. `task_context.py` lists rule history
on demand for agent-guidance paths instead of reading it; `--topic rationale`,
unknown paths and empty scope still read it. All 59 rule anchors and the
hard-invariant block are byte-identical to the base, and every unnumbered
requirement keeps its force. Injected startup falls 6,538 → 5,817 bytes and an
agent-guidance packet 47,747 → 23,788. Gateway and provider packets move −0.3%,
since the numbered bodies they read were out of scope. An integration packet
grows 4.4% for the brief standard. Source bytes, not tokens. No pin, count or
ledger entry moves; three task-context tests are added.
[Report](../../reports/w44-agent-guidance-opus55/README.md).

**`W44-LIBC-RESIDUALS` (2026-09-25).** Worked the four libc-side residuals beside the largest libc leaf masses, measuring each before building it, and re-measured on main `5355998d` after merging W44-INDIRECT-CHAIN-ACCESS and W44-CORPUS-CALLEE-SUMMARIES. glibc's header-inline `__bswap_16/32/64` and `__uint16/32/64_identity` are **checked, not declared**: W44-CORPUS-CALLEE-SUMMARIES measured that their `static __inline` bodies validate under W4, so this card lifted that card's reserved-name filter in `legacy_callee_scope.rs`. A reserved-name `static` definition the unit carries enters the derivation scope on the linkage premise alone, and a name the unit only declares never does (Lean `QCore.CalleeSummaryScope`: `reservation_is_not_read`, `an_undefined_name_is_never_in_scope`). An earlier revision declared them in a `libc/byteswap.qcore` and taught the reader `__extension__`; both were dropped, and that route's arbiter is kept under the report's `route-a/`. The declared-ABI reader projects `__builtin_va_list` at a parameter to a new parameter-only, borrowed-only loader spelling `va_list` that names the type rather than its x86-64 or AArch64 layout (Lean `QCore.GnuBuiltinReader`; `AbiType` gains `vaList`); it moves 0 of 13,101 manifest census rows. A new tail premise, T6 (`variadic_nested::is_literal_or_predefined_actual`, Lean `QCore.LiteralTailActual`), admits a tail actual that is only literals and C11 6.4.2.2 predefined identifiers whatever the function's declarations or C02-P's closure say; the closure had marked literals risky where one reached `__assert_fail`. Declared in `providers/quirin-provider-libc`: `vsnprintf` and `vprintf` (`libc/stdio.qcore`, 168/168 units agree) and `strerror` (`libc/errno.qcore`, 181 units; kept out of the published `libc/string.qcore`). Staged arbiter, every leaf attributed and the stages composing exactly: T6 −974, header bodies −627/+5, `va_list` −26/+1, `strerror` −2; 1,629 removed and 6 added. Functions 296 / 85 / 4,453 → **299 / 85 / 4,450** (`ut_put_be16/32/64`); units, leak 271 and `unknown_alias` 4,420 unchanged; `unsupported` 45,721 → 44,098; no violation candidate. Retired `W43-LIBC-HEADER-INLINE-BODIES`, `W44-CALLEE-SCOPE-RESERVED-BODIES`, `LIBC-PRINTF-VA-LIST`, `LIBC-PRINTF-TAIL-ACTUALS` and `LIBC-PRINTF-NESTED-TAIL-CALLEES`; registered `W44-TAIL-NAME-ACTUALS` (226), `W44-PRINTF-NESTED-INNER-CALLEES` (87) and `W44-VA-START-END-BUILTINS` (127 in 51 functions); 358 → 356. [Report](../../reports/w44-libc-residuals/README.md).

**2026-09-25 — W44-ASSIGNMENT-DESTINATION-SUFFIX-PLACE: a destination is the whole left-hand side.**
This is a soundness repair. `direct_assignment_destination` named the first
place-spelling suffix of a left-hand side, so `recs[i].name = x_dup(s);
x_free(name);` bound the local `name` and certified `Safe`. A destination is now
named only when the suffix is the whole left-hand side or follows a declarator
prefix (identifiers, `*`, `,`). Every other such left-hand side is a store into
a sub-object, and the lexical gate refuses it.

The reach census of the base found 10,979 distinct suffix-named records:

* 7,853 declarations, whose names are unchanged;
* 3,126 sub-object stores, of which 1,450 had been admitted under a misnamed
  destination, in 173 functions.

Only two Safe functions relied on it, `LZ4_write16` and `LZ4_write32`, and
both move to Unknown. `unpack_basic_type` moves from provisional Unsafe to
Unknown when its two false leaks, accusations about the misnamed `opts`, are
withdrawn. Leak 271 → 269, `unknown_alias` −1,116, no unit moves, and every
delta is attributed. The forecast missed only nine caller leaves of the two
functions that lost their W4 summaries.

The descriptor guard's missed index read is fixed by the same reader. The
protected-formal closure keeps the old over-approximating reader. Lean
`QCore.AssignmentDestinationSuffix` (17 theorems, including "the repair never
promotes") is audited by `AuditAssignmentDestinationSuffix.lean`. The card
registers three residuals; its own ledger name lives on the call-argument
branch, for the integrator to retire. See
[the report](../../reports/w44-assignment-destination-suffix-place/README.md).

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
