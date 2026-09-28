# Numbered rule router

These pages hold the canonical full text of the standing constraints formerly
embedded in `AGENTS.md`; references to “AGENTS constraint N” retain their meaning.
Root guides are navigation only.

## Generated task packet

`scripts/task_context.py` computes the reading packet; the
[task reading protocol](operations.md#task-reading-protocol) is its authority.
The table below is the manual fallback if generation fails. Rule history is in
the checkpoints named at the top of [operations](operations.md).

## Choose by task

| Work | Read |
|---|---|
| Every change | [Operations](operations.md), [validation](validation.md), [evidence/trust](evidence.md) |
| C gateway, summaries, contracts or corpus | [Gateway](gateway.md), [source identity](source-identity.md), [diagnostics](diagnostics.md) |
| Parsing, lowering, keywords, source profiles | [Source routes](source-routes.md), [diagnostics](diagnostics.md), then affected feature below |
| Buffers, strings, slices, leases, records | [Buffers/records](buffers-records.md), [concurrency](concurrency.md) when crossing tasks |
| Effects, error regions, epoch credit | [Effects](effects.md), [source routes](source-routes.md) |
| Events, sockets, HTTP, rendering | [I/O](io.md), [source routes](source-routes.md) |
| Threads, mutex sharing, traces, F02 | [Concurrency](concurrency.md), [buffers/records](buffers-records.md), [source routes](source-routes.md) |
| Provider host, protocol, packages, transport | [Providers](providers.md), [source identity](source-identity.md) |
| Documentation, CT, acceptance, archive or tracker | [Evidence](evidence.md), [operations](operations.md); add topic rules if claims change |
| Writing card briefs, delegating, integrating, closing a wave | [Operations](operations.md): card briefs, integration and maintenance sections |

Shared-file changes require the union. Relevant paths include the committed
diff from the declared base and all staged, unstaged, renamed, deleted,
untracked and planned files. If relevance is uncertain, inspect the authoritative
rules. This table routes reading; it neither exempts a constraint nor proves it
was read. Read every applicable rule's full text, cross-topic rules included;
the index below is navigation only. Backticked code paths throughout these pages are repository-relative.

## Stable rule index

| Rule | Full text | Opening sentence (navigation only) |
|---|---|---|
| 1 | [diagnostics](diagnostics.md#constraint-1) | docs/QCORE_SPEC_r0_3.md §L21 is the canonical per-code ownership table. A… |
| 2 | [diagnostics](diagnostics.md#constraint-2) | Q020 (task route) and Q021 (sequential route) both refuse an unenforced… |
| 3 | [diagnostics](diagnostics.md#constraint-3) | §L21.1's reason vocabulary is implemented as qcore_schemas::reason, but… |
| 4 | [gateway](gateway.md#constraint-4) | A call into an internal C function is modeled only through a declared… |
| 4a | [gateway](gateway.md#constraint-4a) | The W4 derivation scope is not the reported scope, and linkage decides what it may add. |
| 4b | [gateway](gateway.md#constraint-4b) | A W4 summary is validated on the premises its function's reported row reads: the candidate is read as the function's own contract (protected formals) in a re-check. |
| 5 | [gateway](gateway.md#constraint-5) | The typed scalar-field whitelist's last premise — the base is a name this… |
| 5a | [gateway](gateway.md#constraint-5a) | A fixed C sizeof/_Alignof query's operand-class marker is never a… |
| 5b | [gateway](gateway.md#constraint-5b) | A return admits what a condition admits, and the difference escapes… |
| 5c | [gateway](gateway.md#constraint-5c) | A constant do … while (0) is sequential composition, and the key is… |
| 5d | [gateway](gateway.md#constraint-5d) | B01–B03 effectful calls preserve evaluation frequency and CFG successors. |
| 5e | [gateway](gateway.md#constraint-5e) | Opaque return evaluation does not establish custody identity. |
| 5f | [gateway](gateway.md#constraint-5f) | Every neutral nested sibling retains liveness and alias obligations. |
| 5g | [gateway](gateway.md#constraint-5g) | W42-INDIRECT-ACCESS adds one route disjoint from the existing unbound scalar-field no-op. |
| 5h | [gateway](gateway.md#constraint-5h) | A desugared condition applies a call exactly where C evaluates it. |
| 5i | [gateway](gateway.md#constraint-5i) | An assignment's destination is its whole left-hand side. |
| 5j | [gateway](gateway.md#constraint-5j) | A scalar-result return decides a read; a cast of a whole call is the call. |
| 5k | [gateway](gateway.md#constraint-5k) | A member release is admitted only as the last use of its place; its summaries are guarded at callers. |
| 5l | [gateway](gateway.md#constraint-5l) | An intrusive-list iteration is modelled on its original spelling; a specification of an audited body is checked, never trusted. |
| 6 | [gateway](gateway.md#constraint-6) | C02-P-ALIAS-LEAK-ORDER: report_leaks on intermediate iterates is… |
| 7 | [gateway](gateway.md#constraint-7) | Leak diagnostics are 34 across the pinned 185-unit corpus. A change… |
| 8 | [gateway](gateway.md#constraint-8) | A declared contract must agree with the C the unit declares — in types… |
| 9 | [gateway](gateway.md#constraint-9) | A fixture prelude is not a production declaration. A fixture result is… |
| 10 | [gateway](gateway.md#constraint-10) | A store into an object that outlives the call is refused, not escaped.… |
| 10a | [gateway](gateway.md#constraint-10a) | A refused statement invalidates only the bindings it could have… |
| 10b | [gateway](gateway.md#constraint-10b) | Non-nullness does not establish lookup identity. A null guard on an… |
| 10c | [gateway](gateway.md#constraint-10c) | An expression statement is a classified effect or a refusal, never an unrecorded no-op; `(f)(x)` is `f(x)`. |
| 10d | [gateway](gateway.md#constraint-10d) | A publication into a caller's container moves custody to it; a published object released is accused, returned or unlinked unpublished is refused. |
| 10e | [gateway](gateway.md#constraint-10e) | A counted user is an obligation checked per path; exit clauses of a defined body are verified, never assumed, and an opaque call is undecided. |
| 10f | [gateway](gateway.md#constraint-10f) | A container outlives the call only by a stated root; an unlinked element of a formal's container is custody taken once, and such a body publishes no summary. |
| 10g | [gateway](gateway.md#constraint-10g) | A declared contract its own analysed body refutes is withdrawn from every caller, cross-unit through a two-phase refutation table; the held call is Unknown. |
| 10h | [gateway](gateway.md#constraint-10h) | An external audit subcommand is found only on the declared plugin path, never PATH or a built-in's name; its claims reach a verdict only through the plugin firewall. |
| 10i | [gateway](gateway.md#constraint-10i) | A held or withdrawn call is only ever tool-minted (a source-spelled reserved prefix is escaped, Unknown); a Pending unit passes every Safe scope gate; a Pending row is joined with every reason its function carries with the held declarations in force, so a held call never masks another reason. |
| 10j | [gateway](gateway.md#constraint-10j) | An external callee is declared only for its body's effect at every audited revision, in its owner's crate (libc flexible, others pinned); a multiplexer only at one literal command; inexpressible effects stay undeclared; a pointer result keeps its dependence on the argument whose storage it lives in (interior, never none). |
| 10k | [gateway](gateway.md#constraint-10k) | A typed access path is read in a condition operand, a return or a call actual only after the position's older routes refused and only where C evaluates it; pointer returns need never-bound, non-may_bind roots. |
| 10l | [gateway](gateway.md#constraint-10l) | A typed position's root premises come from the Clang AST only where the lexical declaration reader answers nothing, joined with the lexical closure; every lexical answer stands. |
| 10m | [gateway](gateway.md#constraint-10m) | A pointer value is read by its unqualified type (only a lone top-level const is erased), and an implicit void * bit-cast into a declared object pointer is the same pointer; volatile, restrict, _Atomic and every other cast stay refused. |
| 10n | [gateway](gateway.md#constraint-10n) | A P3 residual shape (an address, an ambiguous operand, an anonymous inline enum member, a scalar connective return) is read only after every older route and 10k refused, only where C evaluates it and never handing a linked place to a callee. |
| 10o | [gateway](gateway.md#constraint-10o) | The never-bound closure reads every assignment wherever it stands (nested, `for` clause, later declarator, parenthesised, brace initialiser) beside its old reading; typed positions serve its join with the AST closure. |
| 10p | [gateway](gateway.md#constraint-10p) | A loop-owned collector operation (`resource_loop_owner`) is admitted only where the thread-entry census derives every caller context on its loop's owner with no unresolved hop; every route that derives no thread refuses it. A coroutine runs on its derived resumer (hop C); a body that may yield after it is unresolved; a conditional arm is dropped only when the build configuration decides it. A field dispatch reaches only registrants of the Clang record type it goes through (untyped sites field-keyed, unmatched ones refused); a plugin copy's instance drops the coroutine route only when both its creating plugin's and the copy's flags derive none. |
| 10q | [gateway](gateway.md#constraint-10q) | A condition-variable call (`resource_thread_sync`) is admitted only where the census derives the lifted shape (predicate-loop wait, locked signal, quiescent init/destroy), and retained storage is released only after a checked join of every live thread; every route that derives neither refuses. |
| 11 | [source-identity](source-identity.md#constraint-11) | Do not normalize the spelled input path (that changes __FILE__), do… |
| 12 | [source-identity](source-identity.md#constraint-12) | A #line directive in analysed source must not be able to move the audited… |
| 12a | [source-identity](source-identity.md#constraint-12a) | A presented coordinate is derived, and no file spelling is ever read from… |
| 13 | [source-routes](source-routes.md#constraint-13) | Concurrency is opt-in: a file declares #import std.concurrency;, the… |
| 14 | [source-routes](source-routes.md#constraint-14) | New keywords are a recorded compatibility hazard: every candidate is… |
| 15 | [source-routes](source-routes.md#constraint-15) | Lock ranks and protection growth are opt-in disciplines: unranked… |
| 15a | [source-routes](source-routes.md#constraint-15a) | A source line in a composed task module stands in the parent's body… |
| 15b | [source-routes](source-routes.md#constraint-15b) | A route reads its statement tokens from the parsed program's… |
| 15c | [io](io.md#constraint-15c) | An event program's streams are compared byte for byte, and that is a… |
| 15d | [io](io.md#constraint-15d) | Windows std.event selects AF_UNIX by capability, not by compilation.… |
| 15e | [io](io.md#constraint-15e) | A rendered string's observation is the view's exact bytes, and the… |
| 15f | [io](io.md#constraint-15f) | An HTTP message's framing is constructed, and its Content-Length is… |
| 15g | [io](io.md#constraint-15g) | A reactor registration is a loan, and the reactor owns no descriptor of… |
| 15h | [buffers-records](buffers-records.md#constraint-15h) | A concat result is static storage the compiler computed, and a… |
| 15i | [io](io.md#constraint-15i) | A declared address is a check-time constant parsed once, and the two… |
| 15j | [effects](effects.md#constraint-15j) | A flat epoch's credit is conserved across the creator's own error edges,… |
| 15k | [buffers-records](buffers-records.md#constraint-15k) | A record of scalars is a plain copyable value; a record with a bytes… |
| 15l | [effects](effects.md#constraint-15l) | An admitted quantitative region erases from emission, and the sealed pack… |
| 15m | [effects](effects.md#constraint-15m) | A composed region on the epoch task route opens its pack after every… |
| 15n | [concurrency](concurrency.md#constraint-15n) | The F02 trace boundary refuses a range write lease, and refusing is the… |
| 15o | [concurrency](concurrency.md#constraint-15o) | (card MUTEX-SHARE-CROSS.) A protected N payload crosses a task… |
| 15p | [io](io.md#constraint-15p) | A socket program's protocol cells are derived, never declared, and the… |
| 16 | [providers](providers.md#constraint-16) | P02-FLAKE: six provider/host rows are load- or parallelism-sensitive… |
| 17 | [providers](providers.md#constraint-17) | EXTERN-F1-STORE-REACH: the abandonment gate refuses a second live… |
| 18 | [providers](providers.md#constraint-18) | Provider executables embed their manifest template and installed manifests… |
| 19 | [providers](providers.md#constraint-19) | E03-B deliberately supersedes E03's QP012 unequal-premise-set refusal… |
| 20 | [providers](providers.md#constraint-20) | P04's in-process counter route was deliberately not removed (live… |
| 21 | [evidence](evidence.md#constraint-21) | Never edit a frozen archive in place. Card T00's coverage.json was… |
| 22 | [evidence](evidence.md#constraint-22) | contracts.json is the fifth artifact diff_artifacts.py itemizes… |
| 23 | [evidence](evidence.md#constraint-23) | CT coverage grows by mapping, not conformance. A grown denominator is… |
| 24 | [evidence](evidence.md#constraint-24) | REASON-CODES-B3's sidecar reason_code tripwire test is flipped at the… |
| 25 | [evidence](evidence.md#constraint-25) | The acceptance-matrix generator carries a section_ownership block:… |
| 26 | [providers](providers.md#constraint-26) | Process providers have process-group containment plus the opt-in… |
| 27 | [concurrency](concurrency.md#constraint-27) | The mutex model supports initially-unlocked non-recursive mutexes only; condvars are lifted only for rule 10q's derived shapes (predicate-loop wait, locked signal, quiescent init/destroy).… |
| 28 | [evidence](evidence.md#constraint-28) | Declared profiles, observations_trusted, GuardFootprintAuthority,… |
| 29 | [validation](validation.md#constraint-29) | A test discovers repository inputs from tracked files only; ignored and untracked content never changes a result. |
