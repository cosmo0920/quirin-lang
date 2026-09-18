#!/usr/bin/env python3
"""Assemble the DOCS-COMPACT-8 verbatim checkpoint appendix from the blocks
extracted before each splice. The blocks are copied byte for byte; the only
text this script writes is the framing."""
import os

W = "/home/cosmo/Documents/GitHub/qcore-c/.claude/worktrees/agent-a9936abd188804098"
M = os.path.join(W, "measure")
OUT = os.path.join(W, "docs/development/increments/wave-40-compaction-checkpoints.md")

BLOCKS = [
    ("c14-verbatim.txt",
     "`AGENTS.md` standing constraint 14 — the per-card keyword-census ledger",
     "The whole entry as it stood at `d6a57f66`, 19,638 bytes. The compacted "
     "entry keeps the rule, every standing decision and the method; what is "
     "here is the card-by-card census narrative, which each card's own report "
     "and each reserved word's source comment in "
     "`qcore_frontend::frontend::is_reserved_source_word` also carry."),
    ("qspace-verbatim.txt",
     "`AGENTS.md` Q-space note — the eighteen-instance enumeration",
     "The whole note as it stood at `d6a57f66`, 6,644 bytes. The compacted "
     "note keeps the icebox decision, the operative consequence, the tally "
     "(re-derived, 18 instances on 10 bands), the two undercounting findings "
     "and the three cards that are not instances; what is here is the "
     "per-instance enumeration those totals were derived from."),
    ("headline-verbatim.txt",
     "`AGENTS.md` ledger — the headline and arbiter-chain paragraph",
     "As it stood at `d6a57f66`, 2,680 bytes."),
    ("corpusrow-verbatim.txt",
     "`AGENTS.md` ledger — the *Cycle 19 raw corpus* row",
     "As it stood at `d6a57f66`, 3,064 bytes. Every figure in it was "
     "re-derived from `docs/reports/w37-corpus-contracts-4/artifacts/` by "
     "`DOCS-COMPACT-8` and none moved."),
    ("ctrow-verbatim.txt",
     "`AGENTS.md` ledger — the *CT01–CT50 conformance* row",
     "As it stood at `d6a57f66`, 5,277 bytes. Every figure in it was "
     "re-derived from `docs/reevaluation/ct_manifest.json` by "
     "`DOCS-COMPACT-8` and none moved."),
    ("gaterow-verbatim.txt",
     "`AGENTS.md` ledger — the *Merged gate* row",
     "As it stood at `d6a57f66`, 4,995 bytes: the wave-39 close's measurement "
     "and provenance. The row now carries `DOCS-COMPACT-8`'s own detached run "
     "instead. Its passed/failed/ignored triple (4,727 / 0 / 10) reproduced "
     "exactly; its summary arithmetic did not, and the correction is in "
     "`docs/reports/docs-compact-8/README.md` §4 rather than applied here."),
    ("followups-verbatim.txt",
     "`AGENTS.md` ledger — the follow-on count paragraph",
     "As it stood at `d6a57f66`, 2,213 bytes, i.e. after `STABILITY-REEVAL-4` "
     "re-wrote it, including the wave-39 registration chain the paragraph's own "
     "next sentence assigns to the increment records."),
    ("c15o-verbatim.txt",
     "`AGENTS.md` standing constraint 15o — the mutex crossing's TSan sentence",
     "As it stood at `d6a57f66`, 517 bytes. Its last clause (\"no TSan row "
     "exists anywhere in this repository\") was false since wave 37; the "
     "correction is in `docs/reports/docs-compact-8/README.md` §4.9 and the "
     "theorem and residual the sentence carries are untouched."),
    ("platform-verbatim.txt",
     "`CLAUDE.md` platform records — the Windows open-residual list",
     "As it stood at `d6a57f66`, 2,201 bytes. Every residual name, report "
     "pointer and tracking status in it is kept in the compacted list."),
]

head = """# Wave 40 compaction checkpoints (`DOCS-COMPACT-8`)

[Development table of contents](../../../DEVELOPMENT.md) · [History index](README.md) · [Wave 40 records](wave-40.md)

> **Preserved checkpoint text. These blocks are not a current rule authority.**
> They are the verbatim text `DOCS-COMPACT-8` compacted out of `AGENTS.md` and
> `CLAUDE.md` at base `d6a57f66`, kept so that nothing the compaction removed
> is lost. Where a block and the current file disagree, **the current file
> wins**; where a block states something the card measured to be false, the
> correction is in `docs/reports/docs-compact-8/README.md` §4 and is
> deliberately *not* applied here, since that would defeat a verbatim copy.

This appendix is separate from [`wave-40.md`](wave-40.md) so that the wave's
record page stays a record page: `wave-37.md` reached 64,613 bytes after
`DOCS-COMPACT-7` put its checkpoints inline, against the retrieval guidance
that a new history page stay below 32 KiB where a complete record boundary
permits it, and that card flagged the split as the close's call to make.

"""

parts = [head]
total = 0
for fname, title, note in BLOCKS:
    text = open(os.path.join(M, fname), encoding="utf-8").read()
    total += len(text.encode("utf-8"))
    parts.append(f"## {title}\n\n{note}\n\n```text\n{text.rstrip(chr(10))}\n```\n\n")
parts.append(
    f"---\n\nNine blocks, **{total:,} bytes** of verbatim checkpoint text.\n"
)
open(OUT, "w", encoding="utf-8").write("".join(parts))
print(OUT, os.path.getsize(OUT), "bytes;", total, "verbatim")
