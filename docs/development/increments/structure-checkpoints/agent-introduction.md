# Pre-restructure agent-introduction

> Historical checkpoint at the migration base, not current instructions.
> Original wording is preserved; current rules start at [AGENTS.md](../../../../AGENTS.md).

<!-- preserved:agent-introduction:start -->
# QCore-C agent briefing

> **Current work status:** The 49-card remaining-work plan of 2026-09-07 is
> fully dispositioned. Current work follows the named follow-ons recorded in
> `docs/reviews/r0_3/acceptance_matrix.json` (`named_followups`) and per-report
> follow-on lists. The overall generalization goal is not yet complete.

Compressed project state for LLM agents; this file is the source of truth for
agents. Humans should read [README.md](../../../../README.md) for the summary and
[DEVELOPMENT.md](../../../../DEVELOPMENT.md) for the development table of contents —
repository layout, cycle records, per-increment evidence narratives and the
full build/validation instructions are divided under `docs/development/`.

**Sync rule (steady state since card DOCS-COMPACT, 2026-09-10).** Status
*history* lives under `docs/development/increments/`, indexed by
`DEVELOPMENT.md#increment-records` — one paragraph per increment, nowhere else.
This file carries only the compact ledger, the hard invariants, the standing
subsystem constraints, the per-change gate and the entry points. A new
increment therefore adds **one** paragraph, newest first, to the **current
record file named by** `docs/development/increments/README.md`, updates that
index when a new wave file opens, updates this file's headline numbers (and, if
it lands a standing "when touching X, do not Y" rule, adds it to the
constraints section below), and leaves `README.md` alone. Do not reintroduce
per-increment chronicles here, in `CLAUDE.md` or in the root table of contents.
A recurring compaction pass over these three files runs every 2–3 waves; the
cadence rule is in `CLAUDE.md`.

## Documentation retrieval and compact context

Prospective production-system cards and their relative priority live in
`docs/plans/fluent-bit-system-coverage/README.md`; these supplement the named
follow-ups, without changing CT coverage or assigning an implementation wave.

Use `DEVELOPMENT.md` as a table of contents, then read the relevant setup,
layout, cycle-evidence or increment page; the increment index names the current
record file and every historical wave. Historical counts and old instructions
describe their checkpoints, not current status. This file remains the authority
for agent rules; each report owns its own evidence, and frozen reports stay
untouched.

Retrieve headings and matching card names before reading long bodies: `rg -n`
with an exact card, symbol or heading in the relevant directory, then read the
surrounding section. Select relevant keys from large JSON records (for example
`named_followups` and `section_ownership`) instead of dumping a whole matrix.
Return commands, exits, counts and log paths — raw logs stay worktree-private
per `CLAUDE.md`'s hygiene rule — and fetch failure context only as needed.

A delegated brief supplies the task, worktree, declared base and HEAD, owned
and planned paths, topic, applicable rule references, report sections and exact
checks. Relevance includes **committed changes since the base**, staged and
unstaged edits, renamed/deleted/untracked paths and planned edits. Shared files
need the union of their path and topic rules. Unknown relevance requires more
authoritative reading, never silent omission. Navigation is not a substitute
for reading all applicable standing constraints before editing.

For documentation maintenance, preserve historical text and evidence links and
keep one current authority for each rule; new history pages should stay below
32 KiB where a complete record boundary permits it. Measure startup guidance,
cumulative retrieval and unique source bytes separately: byte counts are not
token billing, and a smaller index alone does not reduce this file's startup
cost. The measured restart plan and remaining packet/router work are in
`docs/plans/resume-2026-09-13/README.md`; the development split's preservation
evidence is in `docs/reports/development-docs-structure/`.

<!-- preserved:agent-introduction:end -->

<!-- preserved:claude-intro:start -->
# Claude Code project notes

@AGENTS.md

`AGENTS.md` is the source of truth: work status, ledger, invariants, per-change
gate, entry points, Lean CI hygiene and diagnostics namespaces. Read its
**standing subsystem constraints** — the "when touching X, do not Y" inventory —
before editing the C gateway, the compile-task transport, the diagnostics
tables, the concurrency routes or the provider host. Per-increment status
history lives in `docs/development/increments/`, reached through
`DEVELOPMENT.md#increment-records`. Follow `AGENTS.md`'s documentation-retrieval
strategy: the root development guide is a table of contents, and setup,
evidence and history pages are read on demand. This file carries only
the operational guidance that is in neither of those.

<!-- preserved:claude-intro:end -->

<!-- preserved:claude-pointer:start -->
## Pointer

Any supplied archive review is older than the merged W2–W5 implementations;
read `docs/reviews/r0_3/acceptance.json` for current repair status. Every other
pointer an agent needs — spec, audit setup, entry points, editor support — is in
`AGENTS.md`.
<!-- preserved:claude-pointer:end -->
