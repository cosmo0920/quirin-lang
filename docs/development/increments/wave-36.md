# Wave 36 increment records

[Development table of contents](../../../DEVELOPMENT.md) · [History index](README.md)

> Preserved checkpoint narrative. Counts, platform observations and old
> instructions describe their recorded dates. Current agent rules and headline
> status live in [AGENTS.md](../../../AGENTS.md); each card's report owns its evidence.

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


