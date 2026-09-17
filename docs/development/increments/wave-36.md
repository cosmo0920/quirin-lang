# Wave 36 increment records

[Development table of contents](../../../DEVELOPMENT.md) · [History index](README.md)

> Preserved checkpoint narrative. Counts, platform observations and old
> instructions describe their recorded dates. Current agent rules and headline
> status live in [AGENTS.md](../../../AGENTS.md); each card's report owns its evidence.

**`WRITE-LEND-TASKS-EXERCISE` (2026-09-18, wave 36).** Discharges the residual
of the same name, landing in one card the three things wave 31's sentence
demanded together: checker rule, emission and sanitized native comparison. A
worker body now **exercises** its range write lease —
`qcore_buffer_write(LEASE, INDEX, VALUE)` and `qcore_buffer_read(NAME, INDEX)`,
the **arity** selecting the form on the `borrow_utf8` precedent, so **+0 words,
+0 tokens, +0 `Q` codes** (censused at zero occurrences of either new arity
across 514 `.qcore`/`.qc` fixtures). The rebasing law is decided **in the IR and
split across the two places its halves are known**: the worker's body states the
exclusive bound it touches through its lease, because a body is checked without
knowing where its lease sits; the creator's spawn site, the only place the real
`[low, high)` exists, decides the fit. Those are proved *equivalent*
(`the_footprint_fit_is_exactly_the_rebasing_law`), which is what licenses
checking a summary once instead of every write at every creation. Exclusivity is
re-proved about the **writes themselves** rather than about the permissions
(`no_byte_is_written_by_two_parties`), and exhibited natively: **two threads
writing distinct bytes of one `int32_t` under ASan/UBSan/LSan, stdout
byte-identical to the reference run**, one allocation released once. Leg 4
**decides** `TRACE-ALPHABET-LEASE-GROWTH-UNDECIDED`: the F02 observable alphabet
does **not** grow, because `AdmittedCausalOrder` is a *sequence* and two
disjoint leases' writes have no causal edge between them, so any order it could
carry would assert an ordering the run does not have — the reachability
argument, that a lease event always refuses first, is published beside it as a
measurement rather than as the reason. Three boundaries are measured rather than
claimed. **ThreadSanitizer cannot run any program this route emits** — wave 31's
own held-only program and the published lease-free `examples/threads/two_workers`
die at the identical address and pc, so the failure is the route's and not the
exercise's (`TASK-ROUTE-TSAN-SEGV`). A stray byte at `payload + 4` lands in
`struct qcore_buffer`'s **own tail padding**, which the allocation owns, so ASan
is silent — the measured reason the emitted helper carries its own extent guard
instead of delegating to a sanitizer, with the overflow named only at `+64`. And
a program mixing whole-payload and byte access is host-dependent
(`WRITE-LEND-EXERCISE-BYTE-ORDER`); the flagship is representation-independent by
C11 7.20.1.1 rather than by luck. Eleven mutations, ten caught and one surviving
**as recorded** — breaking the frontend's read-share arm leaves the IR refusing
with the same sentence, constraint 15h's "decided in the IR, not by a route rule"
made visible. The campaign found a real defect in the test set: three IR arms
were backstops **no test reached**, the
`TRACE-ALPHABET-LEASE-REFUSAL-UNEXERCISED` shape again, now driven by five
IR-level tests. 16 card tests, 133 Lean audits, no Windows or Apple run claimed
(`WRITE-LEND-EXERCISE-WINDOWS-RUN`). CT07 is deliberately **not** remapped
(constraint 23); `CT07-TWO-THREAD-REMAP` names the lane work.
`docs/reports/write-lend-tasks-exercise/README.md`.

**`C-GATEWAY-INDIRECT-CENSUS-2` (2026-09-18, wave 36).** A measurement card:
no analyzer rule changed, no verdict moved, no re-pin owned. It censuses the
corpus's largest refusal family, `indirect memory access is outside the resource
projection` — **18,555 leaves across 3,239 functions in 171 units** — from the
analyzer's own projection rather than from source text, which is the error that
stalled the abandoned wave-33 census (refusals are recorded at the preprocessed
line, constraint 12a, and wave 35 measured the source-vs-preprocessed gap at
~57x for macro idioms). Instrumentation sits behind the off-by-default
`indirect-census` cargo feature on `qcore-c-gateway`, enabled by no workspace
member; with the feature **on** and the census actively streaming 88,001
statement records, the pinned corpus audit still reproduces
`docs/reports/c-gateway-constant-do-while/artifacts/` **byte for byte** on both
verdict-bearing artifacts. The census explains **18,554 of 18,555 leaves
(99.99%)**, joins **171/171** units exactly, and independently reproduces the
integrator's wave-33 figure — **284 sole-blocked functions holding 833 leaves
(4.5%)**. Its result is **negative and reported as one**: ranking shapes by leaf
mass and by *decided functions* give nearly disjoint orders, the top eight shapes
by leaf mass (6,699 leaves, 36% of the family) decide **two** functions between
them, the best single shape decides **11**, the best pair **17**, and all 403
shapes admitted at once **45**. Constraint 5's premise — the base is a name this
function never binds to a resource identity — **fails for 95.7%** of the family
and 87.2% of the sole-blocked leaves, which bounds any arbiter that does not
propose to weaken it. Because 72.2% of leaves end at a member the declaration
reader will not classify, the card also instrumented `read_record_body` and found
it total-or-reject over the whole body: **173 of 909 record tags (19.0%)** are
refused entirely on one `{`, `(` or `:`, so a function-pointer member or bitfield
makes every sibling scalar unclassifiable — 2,713 leaves, measured ceiling **2
sole-blocked functions**. The reader's dominant answer, though, is that **78.5%
of unresolved members were never a question**: the *base*'s declared type was not
committed, a different reader nobody has measured
(`BASE-TYPE-READER-UNMEASURED`). Nine residuals are named, including
`CENSUS-TWO-PREMISE-READINGS` (the two implementations of constraint 5's premise
disagree on 16.0% of the family, and the constraint's own wording is the stricter
one) and `POISON-UNMASK-ON-ADMISSION` (any admission spares bindings from the
refusal poison and can unmask a stale scalar fact, constraint 10a's already
recorded hazard, so a wave-37 re-pin must attribute those moves separately).
Gate: **4,580 passed / 0 failed / 8 ignored** over 230 result summaries,
fmt/clippy clean (plus an extra clippy pass with the census feature **on**), both
probes exit 0, **132 Lean audits**, every exit captured to its own file — all
eight steps exit 0. One constraint-16 observation is recorded with **both**
facts: a separate run deliberately concurrent with two corpus audits failed
`a_forced_override_never_weakens_an_execution` with `Io(Broken pipe)`
(`qcore-verifier-host/tests/execution_quota.rs:777`) and the same test **passes**
in the isolated gate; that is constraint 16's signature and crate but **not** one
of its six named rows, and the card files it as `LOAD-SENSITIVE-ROW-UNNAMED`
rather than proposing a seventh, because one observation under a heavy
self-inflicted load carries no measured load figure. Report:
`docs/reports/c-gateway-indirect-census-2/README.md`.

`STABILITY-REEVAL` #2 (2026-09-18), the cadence's **second firing**, audited
waves 34–35 by `CLAUDE.md`'s four steps and closed with **31 registered
residuals and zero retirements** (`named_followups` 121 → **152**), the
zero deliberate: a retirement is what this firing found doing the damage. Over
firing #1's 27 residuals the tally is **15 discharged · 3 partial · 10 open**,
and the pattern is that every residual needing a **test, a control or a
mechanism** is open while almost every one needing a **sentence** is
discharged. Two findings are about the mechanisms the project relies on rather
than any card. `AGENTS.md`'s Cycle 19 raw corpus row carries "**44** corpus
functions" and "**21** sites" where card `JOINED-PROVENANCE-CYCLES` §11 said
"apply in these words" and the words were **40** and **14**; the 21 is the
**superseded `a300a39d` census** that card's own §2 exists to stop anyone
quoting, and the string 44 occurs nowhere in its report or logs
(`JPC-LEDGER-44-21-STALE`, measured independently by two sub-audits). And
wave 35's `reconciliation.json` records that very item **`applied`**, naming
the row as the place and asserting the 40 and the 14 "is present" — which they
are not. The close rule already demands an `applied` verdict say *where*, and
this one does; nothing checks what it **asserts** against what is **there**, and
a scan of all 103 wave-34/35 verdicts shows the defect invisible to any check
that does not read the named location (`RECONCILIATION-APPLIED-ASSERTS-UNVERIFIED`).
Three Lean statements in one module are described by more than they prove:
`an_off_cycle_node_does_not_return`'s only hypothesis asserts **whole-graph
acyclicity**, so it is vacuous exactly where its card operates, while the
shipped `acyclic_sites` tests per-node off-cycle-ness inside a cyclic graph and
the property claimed ("runs at most once per call") is about execution, which
`Steps` cannot express; `fixed_point_is_sound` is described by the acyclicity
hypothesis it **lacks** rather than the `TransferSound` premise it **carries**,
a name that appears in no report, record or ledger; and `acyclic_solution_is_unique`
holds only among solutions agreeing at the entry. Constraint **5c** is false
about the shipped tree for `for (;;)`: `condition.is_empty()` has dropped that
header's **exit** edge since Cycle 19 (`490f5151`), with neither the Lean
statement nor the re-pin 5c says such a step needs — exact, not unsound, but the
neighbour is unowned (`FOREVER-FOR-EXIT-EDGE-UNOWNED`). The sentence 5c records
the integrator as having caught, because landing it "would have put a statement
contradicting 10b into this file", is landed verbatim in the committed script
5c sends readers to. Wave 35's own record shows a **gate exit swallowed by a
pipe** — a merge reached origin with the Python suite red — "the second time
this wave". Measured **clean** and reported as such: both `C-GATEWAY-RETURN-CALL-W4`
arms go red under mutation and both mutations produce a false `Safe`, so the
premises are load-bearing; all three unread `AGENTS.md` merge resolutions
dropped nothing; **zero** weakened tests across waves 34–35, with three pinned
expectations moved and all three disclosed in both report and commit message;
and an `AbstractPointer` with empty targets, `unknown` and `may_be_null` false
provably cannot reach `state.pointers`, by exhaustive enumeration of fourteen
write sites — though that invariant is asserted nowhere and the struct's derived
`Default` **is** the forbidden triple. Forward: the 64-round fixed-point bound
is driven by **no test**, counted by **no census**, and its decline returns the
**same value** as "nothing to derive", so the rate is unmeasurable without a code
change; `release_site` has **seven** refusal disjuncts and three are
instrumented; and both wave-36 arbiters are chartered against numbers that exist
only outside the tree. `docs/reports/stability-reeval-2/README.md`.

**`W36-CORPUS-CONTRACTS-3` (2026-09-18), wave 36's second arbiter — six
declarations chosen by function-cover yield, and the two places the method
broke.** Declarative: six `.qcore` modules and six manifest rows, no crate
source touched, so **one audit binary served both stages** and the only
difference between them is which modules the manifest selects. The BEFORE
reproduces `docs/reports/joined-provenance-cycles/artifacts/` byte for byte in
both verdict-bearing artifacts — the card brief named
`c-gateway-constant-do-while` instead, which was **stale by one**, checked
rather than assumed (the two archives' diagnostics and chunk-contracts
decompress identically, so the cover is the same either way).
The cover was re-derived rather than carried: **193** projected functions are
blocked *only* by `call to uncontracted function` leaves, and **122 of them
have exactly one blocker**, which is what makes a single declaration able to
move a function. Selection was by sole-blocker count and deliberately not by
leaf mass — `fprintf` (1,138 leaves) and `snprintf` (1,044) are the sole
blocker of one function and none, measured. Landed, each read line-cited from
the pinned checkout before it was declared:
`cfl/sds_lifetime.qcore` (`cfl_sds_destroy`, 8 functions),
`librdkafka/kafka_mem.qcore` (`rd_kafka_mem_free`, 4),
`cmetrics/context_lifetime.qcore` (`cmt_destroy`, 3),
`librdkafka/cjson.qcore` (`cJSON_Delete`, 3),
`monkey/logging.qcore` (`mk_print`, 3, the card's one variadic tail) and
`ctraces/context_lifetime.qcore` (`ctr_destroy`, 2) — the corpus's **first
`ctraces` family**. The nullability markers are deliberately **not uniform**:
`nullable` for `cfl_sds_destroy` (first statement `if (!s) return;`) and
`cJSON_Delete` (`while (item != NULL)` precedes every dereference, and all
three call sites *reach* that null path), `nonnull` for `cmt_destroy` and
`ctr_destroy` (no guard, first executable statement dereferences the
parameter). Prediction committed **before the AFTER run finished**: 204 leaves
and 23 functions. Measured: **204 leaves exactly**, and **20** functions, not
23. The three-way split — **20 leaf-free** (all `Unknown → Safe`), **3
exchanged for another refusal**, **0 unmoved**, so no declaration failed to
decide its own leaf and **none of the six was withdrawn** by constraint 8's
gates. The two breakages are the card's real findings. The three `cJSON_Delete`
functions are `cJSON_Delete(cJSON_DetachItemFromArray(...))`; decided leaf,
then a *new* refusal — `nested call … inside a modeled call is outside the
resource projection` — that was unreachable while the outer call was
uncontracted (`CJSON-DELETE-NESTED-DETACH`), which makes a sole-blocker cover
an **upper bound** and never a forecast (`W36-COVER-HIDDEN-BLOCKER`). And
`rd_kafka_mem_free`'s own defining function moved **`Safe → Unknown`** on the
corpus's first **`summary_conflict`**: W4 derives `Released{nonnull}`, the
declaration says `nullable`, and the declaration is the faithful one —
`rd_free(NULL)` reaches `free(NULL)`, and declaring `nonnull` to silence the
conflict would assert a precondition the body does not have, which constraint 4
forbids. That is constraint 8's gate working (reported, never silently
resolved), in the conservative direction, at a net of +4/−1 for that
declaration (`KAFKA-MEM-FREE-SUMMARY-CONFLICT`). Corpus-wide: **217 leaves
removed, 128 added, net −89**; projected functions **186 → 205 Safe**, Unsafe
**84 unchanged**, Unknown 4,564 → 4,545; **zero unit status moves**; `leak`
**272 → 272, unmoved**, so constraint 7's pin needed no re-pin at all;
`unsupported` 50,106 → 49,922, `unknown_alias` 4,214 → 4,308 — every added leaf
in the accusing direction, and the rising-alias/falling-unsupported shape is
what modelling a release *does*, not a regression. `chunk-contracts.json` is
byte-identical between the two stages. Read and **deliberately not declared**,
with the readings published: `add` (3 functions) — three distinct definitions
with three signatures, the cmetrics one `static inline` with no header
declaration for constraint 8's ABI gate to agree with (`ADD-NAME-COLLISION`);
and `memcpy`/`memset` (9 functions, 130 leaves) — both return `void *` aliasing
their first parameter and v1 has no result mode that says so, a **vocabulary
gap** rather than a judgement that they are undeclarable
(`LIBC-MEM-RESULT-ALIAS`). No Lean module is added, on
`W31-CORPUS-CONTRACTS-2`'s recorded ground and stated rather than assumed: a
grown premise set is not a semantic change, and the vocabulary, loader and
checker are all unedited. Corpus 76 → **82** modules, manifest-selected 21 →
**27**, resource-v1 declarations 43 → **49**, every constraint-5 pin updated
deliberately (`qcore_contract_corpus.rs`, `contract_loader.rs`'s
`(21, 43, 145) → (27, 49, 145)` and its explicit old-table parity rows,
`prod_contracts.rs`, `neighbours.rs`). Gate
`submodule=0 test=101 fmt=0 clippy=0 probe=0 followup=0 lean=0 pytests=0` —
**4,579 passed, 1 failed, 8 ignored**, 132 Lean audits, both probes exit 0. The
one red row is the one the card brief flagged in advance,
`closed_input_does_not_mask_descendant_timeout_or_accept_a_partial_request`,
and it failed on a **one-second wall-clock assertion**
(`standalone_host.rs:175`) over an 8 MiB pipe write and a process spawn against
a 40 ms declared timeout. Reported with load and repetitions as the brief
requires: it failed at load average **~40 on 16 cores** with sibling wave-36
cards building (`rust-lld` at 300%/150%/141%), and passed **12 of 12** re-run
in isolation at load 23.7–27.6. The card changes no crate source and
`qcore-verifier-host` never reads the contract manifest, so the row is offered
as constraint 16's **sixth** load-sensitive member — or as evidence that
`e3d3a2ce`'s repair is incomplete, the integrator's call
(`STANDALONE-HOST-CLOSED-INPUT-LOAD`).
`docs/reports/w36-corpus-contracts-3/README.md`.

**`CARD-SCOPE-GUARD` (2026-09-18)** answers `CARD-SCOPE-SELF-MERGE` with a
mechanism rather than another instruction, and its interest is how much of the
obvious design was **falsified by measurement**. The wave-33 defect was a *local*
ref update — `git -C <main checkout> merge` from an agent worktree — so a
`pre-push` hook never sees it and the git process's own directory is the main
checkout. The guard is therefore a **`reference-transaction`** hook (git ≥ 2.28;
host 2.53.0), which runs on every ref update from every worktree and aborts by
exiting non-zero in the `prepared` state, with `pre-push` beside it for the
remote door. The first design read the caller's exported `PWD`, reasoning that
`git -C` moves the process but not the variable; measured, **git rewrites `PWD`
for its own hooks**, and measured again, **a POSIX shell re-derives `PWD` from
`getcwd()` at startup**, so that arm could not fire on any input at all — it was
removed as vacuous rather than shipped green, with a test now pinning the
measurement so it is not re-added. What does survive `git -C` is **process
ancestry**: the card's own shell and session remain ancestors of that git
process, so the arm that catches the incident shape reads `/proc` upward for an
ancestor whose directory is a delegated worktree (Linux only, stated). A third
measurement: `update-ref -d refs/heads/main` reaches the hook as
`0000…→0000…`, and the first draft's `old == new` no-op filter therefore
**deleted `main` and reported success** — red before green. A fourth decided the
veto's scope: all 15 Agent-tool worktrees on this host are `agent-<hex>` while
the delegating session sits in a *named* one, so vetoing every
`.claude/worktrees/` path would have refused the integrator and left nobody able
to advance `main`. Two layers stand: a provenance veto no authorisation
overrides, and a token making refusal the default for everyone, with the
`CLAUDE_CODE_CHILD_SESSION` signal **calibrated at activation** against the
integrator's own environment instead of assumed. 27 tests drive it in throwaway
repositories, and **11 of 11 mutants die, no survivors**. Two things are not
claimed: the hooks are **inert** until the integrator runs
`python3 scripts/install_main_guard.py` once in the main checkout — a step a card
cannot take, which is why the residual is amended and kept open rather than
retired — and a hook is **bypassable by intent**, so this guards a card's mistake
and nothing more. `CLAUDE.md`'s worktree hygiene also gains the harness hazard
that surfaced mid-card: an isolation worktree with an unchanged tree is removed
even while its agent's background jobs run (`WORKTREE-AUTOREMOVAL-PRISTINE-TREE`).
No crate source, no `proof/`, no pinned artifact: the workspace is byte-identical
to `main`'s at `60958bf0`. `docs/reports/card-scope-guard/README.md`.

