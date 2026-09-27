# DOCS-STRUCTURE-COMPRESSION — routed rules and a single documentation hub

> Historical checkpoint: source filenames, runnable references or discovery scripts in this archive were updated by the [extension migration](../extension-physical-migration/README.md). Recorded results, hashes and gate counts describe the original revision, not a new run of the maintained files. Use [current status](../../STATUS.md) for present measurements.


User-requested structural compaction, 2026-09-19. Base:
`463fa6e8d0c44c23e402dd78bdc146963f0244ca`. This supersedes the old policy
that all current rules/status must physically live in root `AGENTS.md` and
that ordinary compaction leaves root `README.md` alone. The user explicitly
requested extreme compression and restructuring. Rule meanings, evidence
boundaries, numbering and frozen reports are preserved.

## Result and authority map

| Before | Now |
|---|---|
| 176 KB `AGENTS.md` with rules, status, history, architecture and diagnostics | Small startup router and six hard invariants; full numbered rules in [topic references](../../agents/README.md) |
| 29 KB `CLAUDE.md` with operational and platform detail | Compatibility entry; [operations](../../agents/operations.md) and [platform evidence](../../development/platform-evidence.md) |
| Repeated status in root guides | [One concise status page](../../STATUS.md), re-derived from primary records |
| Several competing navigation pages, report folders without an index | [Documentation hub](../../README.md), task-to-rule router and [report lookup](../INDEX.md) |
| Unique context removed by editing prose | [Dated checkpoints](../../development/increments/structure-checkpoints/README.md), each below 32 KiB |

This is **startup and retrieval compression**, not deletion of the evidence
corpus. Topic files preserve all 50 standing constraints and operations in full;
the six hard invariants remain in the startup guide. The router is not permission
to skip cross-topic rules. Existing root heading links are retained. A reference
to “AGENTS constraint N” now resolves through the stable numbered router.
Every live policy page is below 32 KiB; the largest is 32,262 bytes.

| Reading surface | Before | After |
|---|---:|---:|
| Four root guides | 226,193 bytes | 14,379 bytes (**93.64% smaller**) |
| Assumed injected AGENTS + CLAUDE | 205,431 bytes | 6,163 bytes |
| Explicit documentation-work packet | Full former AGENTS + CLAUDE: 205,431 bytes | 54,360 bytes |
| Explicit gateway-work packet | Same former briefing | 118,884 bytes |
| Explicit buffer/task-work packet | Same former briefing | 126,967 bytes |

Each packet reads its listed files once, so cumulative and unique bytes are
equal by construction. A repeated read must be counted again in cumulative
retrieval. These fixtures do not measure this session's actual retrieval or
claim that every task needs only its packet; cross-topic work needs the union.
The report index covers **392** report directories at this checkpoint.

## Preservation and measurements

[migration.json](migration.json) partitions every character of the four original
root guides into preserved spans. [verify.py](verify.py) reconstructs each base
file from its destinations and compares both complete content and SHA-256.
Only Markdown link rebasing is reversed; it also checks that the rebased links
resolve to the same path and fragment. No old report, acceptance matrix,
specification, contract, Rust module or Lean file is changed by this migration.

The one-shot [migration script](migrate.py) documents the transformation from
the pinned base. It is not a future rule generator: edit live topic pages
directly. Preservation checks are evidence at this card's checkpoint; legitimate
later rule edits need not reproduce it, just as frozen reconciliation records
are not expected to stay green against later trees.

[preservation.json](preservation.json) records before/after root bytes,
assumed injected startup bytes, explicit retrieval packets, cumulative and
unique source bytes, legacy heading checks and local-link findings. Packet
metrics are reproducible reading examples, **not runtime telemetry or token
billing**. No reduction of total repository bytes is claimed: retaining historical
context adds archival bytes while making the current reading surface smaller.
[status-sources.json](status-sources.json), reproducible with
[rederive_status.py](rederive_status.py), contains the independently re-derived
CT, corpus, contract and tracker values; the wave-40 gate is attributed to that
close, never relabeled as this card's measurement.

## Residual sweep compatibility

Moving the rules would otherwise hide names from `reconciliation_sweep.py`, which
previously read `AGENTS.md` and `CLAUDE.md` directly. The tool now discovers the
canonical `docs/agents/*.md` files and platform evidence as **mention-only**
sources. The acceptance matrix remains the sole tracker. Two new tests exercise
names appearing only in moved rules/platform text, their retirement into the
real tracker, and exclusion of historical checkpoints from live-rule discovery.
This tooling change does not alter any language/checker semantics or require a
new model theorem; it preserves the existing residual-discovery policy.
[The before/after discovery comparison](residual-discovery.json), reproduced by
[check_residuals.py](check_residuals.py), retains all 49 original candidates
from rules/increment records, with zero lost names. The new increment's own
card name is the one additional candidate, not an unresolved finding.

## Validation

The complete gate passed: **4,727 Rust passed, 0 failed, 10 ignored, 238 result
summaries; 141 Lean audits; 305 Python tests**. Fmt, clippy and both probes
exited 0; Clang 21.1.8 was recorded in this checkout. All five workspace rows
from constraint 16 and the additional forced-override row passed; no isolated
retry was needed. The sixth named constraint-16 row is in the separate provider
workspace and was not run. The two added Python tests account for 303 → 305;
the old wave-40 gate remains attributed to its own tree on the status page.

Commands and final outcomes are recorded in [validation.json](validation.json).
Raw logs are private to this checkout under `private/docs-structure-compression/`.
Required checks: migration reconstruction and local links, generated report
index consistency, CT consistency, the Python suite, and the unchanged complete
standing Rust/Lean/probe/fmt/clippy gate. Provider packages are not re-measured:
their sources and schema are unchanged. No new corpus or conformance claim is
made by this documentation change.

## Integrator reconciliation list

1. Treat `AGENTS.md` as the startup router, `docs/agents/` as current rule text,
   and `docs/STATUS.md` as current summary. Keep original rule numbers. Amend
   the canonical file and quote that location in new reconciliation records.
2. Old frozen close records quoting root guide prose can go red after this
   authorized move. Do not rewrite them; use this migration's span map and the
   existing supersession policy.
3. Keep the report index generated, and include `build_docs_index.py --check`
   when adding a report. New policy files remain discoverable by the residual
   sweep; do not use rule mentions as tracker registrations.
4. Keep one increment narrative in the current wave file. No matrix entry is
   added or retired, no semantic card is declared complete, and CT/corpus
   counts do not move. Future compaction applies to the routed policy pages,
   not merely to already-small root files.
