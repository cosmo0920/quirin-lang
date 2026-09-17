# Wave 36 increment records

[Development table of contents](../../../DEVELOPMENT.md) · [History index](README.md)

> Preserved checkpoint narrative. Counts, platform observations and old
> instructions describe their recorded dates. Current agent rules and headline
> status live in [AGENTS.md](../../../AGENTS.md); each card's report owns its evidence.

**`MUTEX-SHARE-CROSS-EMISSION` (2026-09-18)** discharges the residual of the same
name — wave 32's declared mutex crossing was *checked and never executed*,
because `mutex_c_emit::is_single_task_trace` gave no C to any trace carrying task
structure — and its interest is a mutation that **escaped**, and what had to
change because of it. The backend gains a second rendered shape,
`classify_trace`'s **fork/join**: non-nested `Spawn`/`Join` pairs whose children's
events are *contiguous* between them, which is exactly what the crossing lowers
to (measured: CT46's trace is `CreateMutex, Spawn, ⟨five child events⟩, Join,
⟨creator's own section⟩`). It renders with real `thrd_t` children over the
existing `mtx_t` emission, generations and protected allocations moved to file
scope so a child's own function can name them; the single-task rendering is
untouched and every previously emitted program is byte-identical. CT46's four
`unassessed` stages — reference execution, emission, native execution and the
sanitized run — move to `executed` on the case's **own** artifact, driven by
`run_mutex_crossing`, and CT46 joins `SANITIZED_GATED_CASES`: sanitized-native
**9 of 46 → 10 of 46**, the first movement of that numerator since wave 32, and
evidence about one case's artifact rather than conformance (constraint 23). The
three coupled edits `CT-CTWIN-LANE` §9 item 5 named landed in one change, as it
required. **The escape.** Five of six emitted-C mutations were caught, and three
were not: deleting the real `mtx_lock` while keeping its log line, moving the
real `mtx_unlock` away from its log line, and calling the child function
directly instead of spawning a thread each produced a **byte-identical** run,
exit 0, clean under ASan/UBSan/LSan. The reason is structural and worth stating
plainly: the native comparison compares the program's *logged* event sequence,
so it is evidence about that sequence and not about the C operations beneath it.
Two repairs followed rather than a caveat — `run_mutex_crossing` now asserts, on
the emitted text, that every logged mutex event is *immediately preceded* by its
own `mtx_lock`/`mtx_unlock` call (the adjacency one renderer writes, proved in
Lean as `a_logged_acquisition_is_preceded_by_a_real_one`), and the thread
boundary is pinned by `emitted_c_contains` plus a `main_excludes` check that the
creator's body carries none of the child's section. With those, the three
escapes go red. A fourth mutation — removing the `thrd_join` so the creator
races the child — aborts or hangs, never a silent pass. **What is deliberately
not claimed.** Contention, a data race, the C memory model. The checked trace is
a *total order* whose child segment is contiguous, so the faithful rendering
creates the child and immediately joins it: the creator performs no trace event
while the child runs. That is not a limitation worked around — a rendering that
let the creator run ahead would realise an order the checker never saw — and it
is **proved**, as
`QCore.MutexShareCrossEmission.the_creator_performs_no_event_while_its_child_runs`,
rather than promised in prose (`MUTEX-SHARE-CROSS-EMISSION-NO-CONTENTION`). No
TSan row exists anywhere in this repository and none is added. The Lean is 30
theorems (`AuditMutexShareCrossEmission.lean`, axiom closure `propext` /
`Classical.choice` / `Quot.sound`): a refused trace renders nothing, refusals
agree between backend and interpreter, segment contiguity and its no-contention
corollary, the two renderings agree, and CT46's own shape decided. Ten hand-built
traces drive every way out of the shape in Rust, each refused by classification,
emission and reference execution at one index; all six classification arms were
shown non-vacuous by mutation, one of them only after a **second** attempt — the
"unmatched spawn" case was being caught by the contiguity arm first, so the
end-of-events arm needed a trace that simply ends inside an open segment. Two
pins moved deliberately and neither was deleted: a composed child module now
emits and runs (its old `c_source.is_none()` assertion carried the instruction to
do this in its own failure message), and the boundary it guarded is re-asserted
on a **guard-local** child, which is outside both shapes, reachable from source
and therefore not a vacuous refusal — pinned in the fixture too, as CT46's new
`guard_local.qcore` control. The card allocates **no `Q` code, reserves no word
and widens no condition**; `Q600`'s existing backend-refusal condition is only
*reached* from the new shape, recorded on the `SOCKET-EVENT-COMPOSE` precedent.
Evidence: `docs/reports/mutex-share-cross-emission/README.md`.

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

