# Numbered rule router

These pages are the canonical full text of the standing constraints formerly
embedded in `AGENTS.md`; references to “AGENTS constraint N” retain their meaning.
Read all applicable rules, not just this table. The rule text is preserved
verbatim apart from Markdown link rebasing. Root guides are navigation only.

## Generated task packet

`python3 scripts/task_context.py --base <revision> --topic <topic> --path
<planned-path>` selects operative sections and complete applicable topic pages.
Use `--render` for their canonical text. Path routes are additive; unknown paths
select every topic. Add topics for claims or semantics that filenames cannot
identify. Recompute on scope changes. The tool reports exact line ranges, hashes,
rule IDs, reasons and reading-byte estimates; it does not certify rule compliance
or measure token billing. Tests cover committed/working/untracked paths and both
sides of renames. The manual union below remains the fallback if generation fails.

Operations history is preserved in the [pilot checkpoint](../reports/task-context-contract-bundle/operations-before.txt).
Read rationale when changing a rule; routine edits need the operative text.
Status is selected for work planning and coverage claims. Broad implementation
paths intentionally keep conservative packets until narrower routing is justified.

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

Shared-file changes require the union. Relevant paths include the committed
diff from the declared base and all staged, unstaged, renamed, deleted,
untracked and planned files. If relevance is uncertain, inspect the authoritative
rules. This table routes reading; it neither exempts a constraint nor proves it
was read. Backticked code paths throughout these pages are repository-relative.

## Stable rule index

| Rule | Full text | Opening sentence (navigation only) |
|---|---|---|
| 1 | [diagnostics](diagnostics.md#constraint-1) | docs/QCORE_SPEC_r0_3.md §L21 is the canonical per-code ownership table. A… |
| 2 | [diagnostics](diagnostics.md#constraint-2) | Q020 (task route) and Q021 (sequential route) both refuse an unenforced… |
| 3 | [diagnostics](diagnostics.md#constraint-3) | §L21.1's reason vocabulary is implemented as qcore_schemas::reason, but… |
| 4 | [gateway](gateway.md#constraint-4) | A call into an internal C function is modeled only through a declared… |
| 5 | [gateway](gateway.md#constraint-5) | The typed scalar-field whitelist's last premise — the base is a name this… |
| 5a | [gateway](gateway.md#constraint-5a) | A fixed C sizeof/_Alignof query's operand-class marker is never a… |
| 5b | [gateway](gateway.md#constraint-5b) | A return admits what a condition admits, and the difference escapes… |
| 5c | [gateway](gateway.md#constraint-5c) | A constant do … while (0) is sequential composition, and the key is… |
| 5d | [gateway](gateway.md#constraint-5d) | B01–B03 effectful calls preserve evaluation frequency and CFG successors. |
| 5e | [gateway](gateway.md#constraint-5e) | Opaque return evaluation does not establish custody identity. |
| 5f | [gateway](gateway.md#constraint-5f) | Every neutral nested sibling retains liveness and alias obligations. |
| 5g | [gateway](gateway.md#constraint-5g) | W42-INDIRECT-ACCESS adds one route disjoint from the existing unbound scalar-field no-op. |
| 5h | [gateway](gateway.md#constraint-5h) | A desugared condition applies a call exactly where C evaluates it. |
| 6 | [gateway](gateway.md#constraint-6) | C02-P-ALIAS-LEAK-ORDER: report_leaks on intermediate iterates is… |
| 7 | [gateway](gateway.md#constraint-7) | Leak diagnostics are 272 across the pinned 185-unit corpus. A change… |
| 8 | [gateway](gateway.md#constraint-8) | A declared contract must agree with the C the unit declares — in types… |
| 9 | [gateway](gateway.md#constraint-9) | A fixture prelude is not a production declaration. A fixture result is… |
| 10 | [gateway](gateway.md#constraint-10) | A store into an object that outlives the call is refused, not escaped.… |
| 10a | [gateway](gateway.md#constraint-10a) | A refused statement invalidates only the bindings it could have… |
| 10b | [gateway](gateway.md#constraint-10b) | Non-nullness does not establish lookup identity. A null guard on an… |
| 11 | [source-identity](source-identity.md#constraint-11) | Do not normalize the spelled input path (that changes __FILE__), do… |
| 12 | [source-identity](source-identity.md#constraint-12) | A #line directive in analysed source must not be able to move the audited… |
| 12a | [source-identity](source-identity.md#constraint-12a) | A presented coordinate is derived, and no file spelling is ever read from… |
| 13 | [source-routes](source-routes.md#constraint-13) | Concurrency is opt-in: a file declares import std.concurrency;, the… |
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
| 27 | [concurrency](concurrency.md#constraint-27) | The mutex model supports initially-unlocked non-recursive mutexes only.… |
| 28 | [evidence](evidence.md#constraint-28) | Declared profiles, observations_trusted, GuardFootprintAuthority,… |
