# Wave 48 compaction checkpoint: status page, part 11 of 11 (`STATUS-COMPACT`)

[Development table of contents](../../../DEVELOPMENT.md) · [History index](README.md) · [Current status](../../STATUS.md) · [Part 10](wave-48-compaction-status-part10.md)

> **Preserved checkpoint text, not a current authority.** The fenced block is
> `docs/STATUS.md` lines 422–570 at `f77e97ae6`, verbatim (21,081 bytes), part 11 of the
> whole page that `STATUS-COMPACT` replaced with a current-state page.
> `git show f77e97ae6:docs/STATUS.md` reproduces it byte for byte, and
> [`checkpoint.py check`](../../reports/status-compact/scripts/checkpoint.py) asserts
> that the parts concatenated equal that page. Where this text and the current
> page disagree, **the current page wins**; figures here describe their recorded
> checkpoints. Relative links inside keep their original base, `docs/`.

~~~~~text
| Return-position correctness (rules 15x–15z) | GAPO-RETURN-CORRECTNESS (base `1a399e6a1`): every declined classification probe restores the whole probe state (one `ParserProbeState`; every parser field classified by construction, monotone source facts kept); a declared no-return call ends its path at every lowered CFG node where rule 11o's premises hold and is refused at every other node (**119** `rd_kafka_crash` nodes in condition plans of 29 corpus functions made path ends); the W4, B01 and W45-K return probes ask rule 12h's H-U0 (**91** returned calls re-read uncast, all still admitted), rule 12i declines a returned call with a hidden cast, and 11w/10u at the return position are out of scope with a census (3 `10u-k03` returns). Corpus, system, slice and null-property reports byte-identical (**935 / 9 / 3,890**, `bdf5f1c8…`); no verdict moved | [report](reports/gapo-return-correctness/README.md) |
| Cast semantics (rule 12h) | one conversion table (`cast_semantics::CONVERSION_TABLE`, 17 classes: the Clang kind refined by both type classes) every route-G position consults: identity-preserving pointer conversions are their operand at value positions (a call actual's cast of a place is that place, a soundness repair), typed and byte views are rule 11j's views at access positions, a qualifier conversion is read through, a punned view refuses; corpus census on main `9c5f2079c`: **16,743** explicit casts in 3,225 bodies, 10,371 leaves (7,732 refusals) on a cast's line after the card, an upper bound (`CAST-FORMALIZATION-RESIDUAL-CENSUS`); Lean `QCore.CastSemantics` ([report](reports/cast-formalization/README.md)) |
| Indirect-access sole blockers (rules 12i, 12j, 14a–14d) | Round 2 of `CG10-INDIRECT-ACCESS-SOLE-RESIDUALS` (main `84ec54d1d`): the population re-measured at **228** functions (380 leaves), ranked by functions one fix frees alone (no reader lift frees a function of the uncontracted classes: each also needs its callee's contract); rules 14a–14d and two declarations free 24; **201** remain (333 leaves), every class above 3 a design refusal, a decision class or a contract question ([round 2](reports/cg10-indirect-access-sole-residuals-r2/README.md)). Before it: of the **513** Unknown functions the access refusal blocked alone on `cdc2fa878` (977 leaves), classified by the readers' own records (`scripts/classify.py`): **321** hide an uncontracted callee behind the gate (218 call statements, 103 whole-call returns; the P4 callee closure), and the two liftable sub-classes were read: rule 12i (a refused `return`'s one call read with rule W44, 41 of 317 consultations admitted) and rule 12j (`.`-only scalar places on automatic record objects, 544 place decisions); **481** remain (916 leaves), ranked in `CG10-INDIRECT-ACCESS-SOLE-RESIDUALS`: uncontracted callees 227 + 102 alone, refused pointer-store values 26, custody positions 14, call results stored into members 10 | [report](reports/cg10-indirect-access-sole-blockers/README.md), [classification](reports/cg10-indirect-access-sole-blockers/artifacts/classify-after.json) |
| C linkage (`.qrlib`) | `.qrlib` is the FFI route's C linkage manifest, **generated on every build** (`native_library manifest v2`, `quirin qrlib generate`/`check`, `quirin build`) from the artifact's own exported symbol table — read from bytes for ELF (`.a` index and objects, `.so` with versions, weakness and `DT_SONAME`, GNU `ld` scripts), Mach-O (`__.SYMDEF` archives, `.tbd` v3/v4/v5, dylib export trie) and COFF/PE (`.lib` linker members, short import records, the PE export directory with forwarders) — and the build's declared inputs; a stale stored manifest refuses and the build hashes every pinned file before linking exactly the pinned artifact. Every FFI call resolves through a generated manifest the file binds with `extern library NAME;`; `BuilderIncludesAndLinks` is replaced by one `linked:NAME` premise per call (static artifact@sha256, or stub@sha256 plus the runtime premise), under the unchanged unsafe-FFI assumption. Manifest `C_ABI_TYPE` gains `float`/`double`, `struct TAG`, `struct TAG *`, `const struct TAG *`; the manifest header digest must equal the `struct!` header. Linux runs: libm static (through `libm.a`'s script) and dynamic (through `libm.so`'s, weak, `GLIBC_2.2.5`, `floor` `ifunc`), a card-built C archive and a `#![no_std]` Rust `staticlib`, three builds each under UBSan and plain `-O2`, the linker's `--trace` naming every pinned file, 0 sanitizer reports. Windows (MSVC `.lib`, one `ntdll.dll` function) **wired into `msvc-core`, not run**; macOS (card 5m, `W48-QRLIB-MACOS`): **run green on one arm64 Mac** (macOS 27.0.1) — libm through the SDK's `libm.tbd`, read through its `arm64e-macos` slice as Apple `ld` does, under the three builds, plus a Mach-O C archive and a Rust `staticlib`; `macos` CI job wired, first run not yet read; Intel macOS out of scope (user decision). Limitations §L14.8. Lean `QCore.QrlibLinkage` (W48-QRLIB-LINKAGE) | [Report](reports/w48-qrlib-linkage/README.md), [§G02d.2.1](QCORE_IMPLEMENTED_GRAMMARS_r0_3.md#g02d21-the-generated-manifest-v2), [§L14.8](QCORE_SPEC_r0_3.md#l148-c-linkage-through-generated-manifests-and-its-limitations) |
| Nested and sibling call effects (rule 10y) | `nested_call_effects::read` reads a call tree (plain, bound, data actuals, any depth) under `EvalGraph::check_footprints`; each call goes through `instantiate_call` inside-out (Lean `QCore.NestedCallEffects`); on the corpus the statement consumer is consulted **197** times (**17** admitted; 128 refused for an uncontracted inner callee, 25 for non-commuting footprints) and the return consumer for **113** returns holding calls (**3** admitted); **+2** corpus Safe, system and tail-pause slice unmoved | [CG05-NESTED-CALL-EFFECTS](reports/cg05-nested-call-effects/README.md) |
| Elaborator call footprints (rule 11n) | the elaborator's call events carry their contract footprint (rule 10y's cells; `EvalGraph::check_with_footprints`, CG18's `check_with` when no footprint is read); a refused piece holding calls is lowered with each call applied through `instantiate_call` (use `Tested`; Lean `QCore.ElaboratorCallFootprints`); on the corpus **18** elaborator statements admitted over 9 functions, 603 rows still refused for a call with no footprint (515 with the callee's own uncontracted leaf); the address boundary (an identity is never proved distinct from an address-taken automatic) kept as measured, **25** consultations over 14 functions, no premise; **+1** corpus Safe, system and tail-pause slice move no status | [CG05-ELABORATOR-CALL-FOOTPRINTS](reports/cg05-elaborator-call-footprints/README.md) |
| C integer arithmetic (rule 11q) | A **separate property beside custody**, never in the custody denominator: a function's integer operations, increments, compound assignments, recorded conversions and bit-field stores reach no undefined and no unstated implementation-defined behaviour. Widths come from the analysing Clang's target macros (LP64 `x86_64-pc-linux-gnu` and LLP64 `x86_64-pc-windows-msvc` by test), conversions are the ones Clang recorded, the core's checked table is reached only through `c_numeric`; obligations are decided on intervals (`Discharged`/`Open`/`Violated`), verdict `Safe` or `Unknown`, never `Unsafe`. No corpus census yet (`CG20-NUMERIC-CORPUS-CENSUS`); custody figures unchanged on all three routes. The lexical ABI reader reads plain `long` as 64-bit only under an LP64 Clang target (its LP64 premise removed); Windows gate owed. Lean `QCore.CNumeric` (37) | [CG20-NUMERIC-DOMAIN](reports/cg20-numeric-domain/README.md) |
| Layout-derived extents (rule 11u) | an automatic VLA of builtin scalar elements whose bound only reads and whose every use is an element access or `sizeof` operand is lowered to a `ForgetScalar` (no custody event; `bound-defined`/`bound-positive`/`size-representable` left to CG20); any other use keeps the refusal. `container_of` names its enclosing object only for a pointer established as `&x->m`. Flexible-array extents (`(size − offsetof) / sizeof`) and derived-address alignment obligations come from Clang's numbers via `layout_extents::layout_report`, which publishes `layout.alignment.v1` (Safe only when every obligation is aligned; misaligned joins to Unknown; `P-CG17-POINTER-ALIGNED` when used); Lean `QCore.LayoutExtents`. Corpus census (66910c10, main `b6a3dc42`): **0** VLAs, **355** `container_of` shapes over 189 functions (all list lookups, unestablished), **441** derived-address conversions over 232 functions (41 aligned, 0 misaligned, 400 undecided), **16** flexible-array accesses over 12 functions (size not established); no function moved on the corpus, system or tail-pause slice. [Report](reports/cg17-layout-derived-extents/README.md). |
| Loops, recursion and budgets (rule 11x) | the counted-user pass summarizes a `while`/`for`/`do` as the identity on every count only when no token of it performs a counted-user operation (no counter write, no clause-declaring call, every other call neutral, no expression callee, no `return`/`goto`), forgetting the values of the names it spells; any other loop keeps the refusal. A recursive component no other route publishes may carry a signed threshold rank (one `int`/`long`/`long long` control type, sign guards `n <= c` / `n > c` at one literal threshold, actuals `n - k` with `1 <= k <= 1024` or `0`, several ranks, mutual). Census at base `b6a3dc42`: **0** corpus, system or slice functions carry the counted-user loop refusal or exhaust a budget; **22** corpus functions name a recursive partner as uncontracted (1 blocked by nothing else, `cio_os_mkpath`, structural), the system's `flb_output_task_singleplex_flush_next` likewise (a heap-measured mutual pair). Corpus **byte-identical** (629 / 10 / 4,195, `3d53a5d0…`), system and slice totals unchanged; Lean `QCore.LoopInvariants` (32 theorems) | [CG27-LOOPS-RECURSION-BUDGETS](reports/cg27-loops-recursion-budgets/README.md) |

**Mapping is not conformance.** CT01/CT03/CT37/CT50 bind their model proofs
through the F02 trace/digest relation; CT07 binds through the scoped-task lease
ledger, not F02, whose lease refusal remains intentional. CT50's binding grew
no alphabet, and its closure over admitted renderings is the degenerate case —
a single-actor run, proved to admit nothing but itself rather than assumed to.
Every proof row is evidence about one case's artifact; **CT50's covers the
capability half of its scenario only**, because the numeric address reuse that
case is named after is invisible to an alphabet carrying generations and stays
native observation (`CT50-PROOF-ADDRESS-REUSE-UNPROVED`). No F02-shaped
candidate remains among the mapped cases (`CT-PROOF-F02-CANDIDATES-EXHAUSTED`,
measured). Since wave 41 each leg's pinned digest is compared with the value its
Lean side decides, total-or-reject both ways. Native execution is observed,
not proved. The sanitizer count carries its host condition; emission assertions
can run on other hosts without implying sanitizer execution there.

**Raw units remain unpromoted.** The unit Safe count is 6; each is a
function-level roll-up, not a promotion. `flb_input_chunk_create` is
function-level Unknown since W45-FB-RETRY-LIFETIME, and the production
`input_chunk_get` residual remains Unknown. Function-level safety and the chunk
**lifecycle** result are separate dimensions: no selected chunk function's
lifecycle post-state is established, and a checked field contract is not a
lifecycle result. Provisional Unsafe findings are not validated upstream
defects. Unknowns stay in the denominator. The fresh baseline and contract-only predecessor are in
`docs/reports/w46-contract-access-expansion/artifacts/`; the fresh baseline
reproduces the preceding contract expansion byte for byte. Its corrected
combined audit owns the current counts. Earlier integration archives remain
frozen; each subsequent arbiter must reproduce its predecessor's
verdict-bearing artifacts and attribute every delta. The chain and leaf identities are
preserved in the individual reports, [wave history](development/increments/README.md),
and [gateway rules](agents/gateway.md#constraint-7).

## Milestones and trust

Audit tooling now uses `quirin-audit pinned` and the shared `quirin-source-analysis` crate; the legacy compiler example was retired after [pinned parity](reports/audit-pinned-replay-cli/README.md), with no coverage movement. Checkpointed current audits seal one binary schema-3 `.qo` per compile configuration; `quirin-dump-checkpoint` inspects validated content, and preparation prints live log paths. The [combined gate](reports/audit-checkpoint-files/integration/README.md) passed 6,059 Rust tests, 365 Python tests and 213 Lean audits. The [extension migration](plans/quirin-extension-migration/README.md) has passing compatibility readers; physical renames, including archived inputs and their runnable references, passed the combined gate. Legacy external `.qcore` inputs continue to receive outdated-name warnings.

Cycles 1–17 are complete only at their bounded milestones. Cycle 18 has bounded
implementation with aggregate acceptance pending. Cycle 19 is in progress;
per-path identity imports and observation evidence gate promotion. Cycle 20 is
active: all v1 Safe proposals are refused, including empty scopes; unchecked
Unsafe proposals downgrade to Unknown (`QP010`).

Lean gateway theorems are abstract rule theorems, not extraction of the Rust
analyzer or whole-C correctness. Faithful C extraction and Rust–Lean
correspondence remain premises maintained by review. See
[trust boundary 28](agents/evidence.md#constraint-28). Report model proof,
artifact binding, source coverage, native observations and host conditions
separately.

The historical Cycle 19 pinned replay is now the built-in
`quirin-audit pinned`; the compiler example is retired after the
[185-unit archive parity](reports/audit-pinned-replay-cli/README.md). Its
source-analysis adapters live in `quirin-source-analysis`, shared by the
compiler and audit crates. All pinned verdicts, coverage and contract text
match the completed W46 predecessor; three source-effect identity hashes
change solely because the new worktree is a different current directory,
with the exact hash cascade accounted in the parity record. The relocation gate is complete; the later runtime-import gate below is the
latest completed integration measurement.

## Latest validated gate

The [collector integration gate](reports/event-collector-selection/integration/gate.json) validates committed source `a010c6dfc9f410b674358c99b5dfd58075d3d640`. Root Rust: **8,927 passed / 0 failed / 15 ignored**, 584 units / 617 summaries; **396 Lean audits** under pinned 4.35.0 and **568 Python tests**, no skips. Formatting, Clippy, both reevaluation examples and the pinned submodule state exit 0. Provider **133 / 0 / 1** is carried from the documented `dc5d07c95` provider rerun; this records-only continuation selects no new provider gate. Actual Clang and every command/exit/private log are in the receipt. The separately labelled published-baseline native comparison includes the intervening pilot integrations, matches all three predictions and retains full-payload attribution with zero remaining semantic deltas. It is not isolated attribution or backend-body conformance. Final committed-tip publication follows the records commit.

The five provider crates own **86** canonical contract modules; **54** shared or
application-level modules live in six `providers/quirin-provider-*` crates: five
per-library data crates (libc only in `quirin-provider-libc`) and
`quirin-provider-shared`, which also holds the providers' shared runtime and
bundles the five. `contract/corpus.json` indexes all **140** without duplicate
snapshots. Retained integration adapters embed the provider-owned data directly;
generic core crates retain no provider executable dependency;
package inventories bind supplied contract bytes to their executables, and
policy selection and independent checking remain explicit. Contract modules can
take a declaration's contract from another published module (`#include` /
`contract inherit NS::NAME`, [CONTRACT-INHERIT](reports/contract-inherit/README.md)),
wired into the provider crates through a generated, digest-pinned load path
([adoption](reports/contract-inherit-adoption/README.md),
[naming](reports/contract-inherit-qcore-provider-naming/README.md)). Every
function declared in more than one module is a
[checked view relation](reports/contract-view-projection-check/README.md).
Resource custody is kind-indexed: a release discharges only an obligation of its
own kind, and a file descriptor is tracked as an `int` with sentinel validity
([W43-DESCRIPTOR-RESOURCE-TRACKING](reports/w43-descriptor-resource-tracking/README.md),
Lean `QCore.KindedCustody`).

The three broader Cycle 18 follow-ups remain registered
(`CYCLE18-CALLBACK-TASK-HANDOFF`, `CYCLE18-NATIVE-PLATFORM-MATRIX`,
`CYCLE18-AGGREGATE-POLICY-OBLIGATION-SYNC`). Production callback composition,
task/cancellation/event-generation coverage and current foreign-host
observations remain outstanding. Callback, CT and full-conformance coverage are
unchanged.

The F02 Windows native run and the host-unconditional pin-agreement test now
pass with the same two emitted-C digests. Host/load-sensitive tests and their reporting
requirements remain in [rule 16](agents/providers.md#constraint-16). Its unnamed seventh
row, `execution_quota`, and the verifier-host fixtures' `Io(Broken pipe)` and `ETXTBSY`
shapes are repaired, and a spurious per-execution cgroup name collision (`EEXIST`, a
fail-closed host refusal) is fixed. The base binaries failed under each race's amplifier
(2 of 20 to 18 of 20); the fixed ones failed 0 of 250 runs for `execution_quota` and 0 of 150
for each sibling ([EXECUTION-QUOTA-LOAD-INDEPENDENCE](reports/execution-quota-load-independence/README.md)).
The per-execution quota join is fail-closed: a failed duplicate of the cgroup's
`cgroup.procs` descriptor now refuses before spawning instead of running the child
outside its cgroup under a record naming the quota, and a record carries a quota
observation only for a spawned child, which has joined
([QUOTA-PROCS-DESCRIPTOR-DUP-FAIL-OPEN](reports/quota-procs-descriptor-dup-fail-open/README.md),
[rule 26](agents/providers.md#constraint-26)).
A process provider that exits successfully without consuming its whole request is now refused
deterministically (`RequestUnconsumed`), decided from the bytes left in the request pipe rather
than from whether the host's write or the provider's exit came first. The base binary gave the
same non-reading provider both outcomes (317 deviating case-runs in 120 runs); the fixed one
gave 0 in 300 runs under host stalls and write and close delays
([HOST-UNREAD-REQUEST-ACCEPTANCE-BY-SCHEDULING](reports/host-unread-request-acceptance/README.md)).
Every executable test fixture in the workspace (16 files) is now written by the shared
out-of-process writer, so a sibling test's fork cannot make its `execve` fail with `ETXTBSY`.
Under a close-delay shim with a forked-child stall, the base binaries deviated in 15 of 50
(`subcommand_plugins`) and 1 of 50 (`cycle20_verifier_plugins`) runs; the fixed ones in 0 of 50
([PROCESS-FIXTURES-FS-WRITE-OUTSIDE-HOST-CRATE](reports/process-fixtures-fs-write/README.md)).
The host now takes that decision only as the request pipe's last holder: a descendant that keeps
the pipe with both outputs closed is waited for, up to the deadline, and a pipe still held then is
refused (`RequestHeldByDescendant`). The base binary gave such a descendant both outcomes (194
deviating case-runs in 120 runs); the fixed one gave 0 in 300 runs. Off Linux the other holders
cannot be observed, and the host decides there as before, so that shape stays timing-dependent
there (never executed)
([HOST-REQUEST-READ-BY-DETACHED-DESCENDANT](reports/host-request-detached-descendant/README.md)).
The plugin SDK's `package::build` now writes the packaged executable through a separate
`tee` process (user decision 2026-10-14: `tee` on `PATH` on Unix), so a multi-threaded caller
can execute the package as soon as it returns. Release test binaries under the same harness
(parallel child stall): base 10 of 50 and 9 of 50 runs with `ETXTBSY`, tip 0 of 50 and 0 of 50;
package bytes, digest and manifest unchanged
([PLUGIN-PACKAGE-BUILD-WRITES-EXECUTABLE-IN-PROCESS](reports/plugin-package-build-write/README.md)).
The summary-count and Clang recording mechanisms still have their disclosed
limitations; see [validation policy](agents/validation.md). This integration
does not discharge those measurement-mechanism limitations.

## Where to go next

[Planning and registered work](README.md#planning) ·
[Rule router](agents/README.md) · [Platform evidence](development/platform-evidence.md).
The pre-restructure ledger narrative is preserved as a
[historical checkpoint](development/increments/structure-checkpoints/status.md),
not a second current status authority.
~~~~~
