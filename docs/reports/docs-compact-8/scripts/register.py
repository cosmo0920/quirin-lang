#!/usr/bin/env python3
"""Append DOCS-COMPACT-8's two registrations to the carried named_followups
section of the committed matrix, refusing if either id is already present.
The derived dimensions.named_followups is then re-derived by build_matrix.py."""
import json
import pathlib

P = pathlib.Path(
    "/home/cosmo/Documents/GitHub/qcore-c/.claude/worktrees/agent-a9936abd188804098"
    "/docs/reviews/r0_3/acceptance_matrix.json"
)
NEW = [
    {
        "id": "GATE-ROW-SUMMARY-COUNT-UNCHECKED",
        "title": "The merged-gate row's result-summary arithmetic has no machine-readable source and drifted for one wave",
        "why": (
            "Measured by DOCS-COMPACT-8 (wave 40). The wave-38 close made lean_audits and clang "
            "required keys that build_matrix.py --gate checks, which was the first arm of "
            "GATE-ROW-FIGURE-ATTRIBUTION-DRIFT and closed the two figures that had drifted before. "
            "The summary arithmetic beside them is still unchecked: the wave-39 close's row states "
            "'240 result summaries over 236 test targets (227 Running plus 9 doc-test sections; four "
            "of those sections print two summaries each)'. DOCS-COMPACT-8's own detached run, on a "
            "tree differing from that close's tip in documentation only (git diff --name-only names "
            "six .md files, and no test in the tree reads any of the three compacted files), measures "
            "238 summaries over the same 236 sections with exactly TWO printing two summaries each "
            "(Doc-tests qcore_core and Doc-tests qcore_verifier_host), by measure/summary_shape.py. "
            "The passed/failed/ignored triple agrees exactly (4,727 / 0 / 10), so nothing load-bearing "
            "moved, which is precisely why nothing caught it. This is the same family as "
            "GATE-SUMMARY-PARSER-OK-ONLY: a count of test-result lines nobody derives. DISCHARGE: the "
            "gate file carries the summary and target counts as keys build_matrix.py --gate derives "
            "from the run's own log (or refuses), so the row cannot state an arithmetic no log "
            "supports; until then a summary/target count in this row is read as one run's arithmetic "
            "and not as a property of the suite."
        ),
        "source": "docs/reports/docs-compact-8/README.md",
        "recorded_by": "DOCS-COMPACT-8",
    },
    {
        "id": "AGENTS-MAP-FIGURES-UNDERIVED",
        "title": "The repository map's counts are hand-kept beside a ledger that derives the same figures, and two of them went stale for waves",
        "why": (
            "Measured by DOCS-COMPACT-8 (wave 40). AGENTS.md's repository-map table called "
            "contract/qcore a 76-module corpus while the ledger row two sections above it said 88 and "
            "the tree has 88 (find contract/qcore -name '*.qcore' | wc -l) - stale since wave 31, "
            "across five compaction-eligible waves and two compaction passes, because the map's copy "
            "is hand-kept and the pinned test qcore_contract_corpus.rs guards the corpus and not the "
            "prose. In the same row the gateway's 'six load-bearing modules' omitted the four landed "
            "since wave 31 that standing constraints 5b, 8 and 10b rest on (legacy_return_call.rs, "
            "provenance_rounds.rs, legacy_declared_abi.rs, legacy_declared_modes.rs), against 36 "
            "modules on disk. Both are corrected. The general defect is that the map restates figures "
            "the ledger already derives, and a second copy of a number is a second thing to go stale - "
            "the same shape as Q-SPACE-EXTENSION-TALLY-HAND-KEPT and "
            "CONSTRAINT-14-WAVE-32-CENSUS-INCOMPLETE. DISCHARGE: either the map stops carrying "
            "countable figures and points at the ledger row that derives them, or a test derives them "
            "the way qcore_contract_corpus.rs pins the corpus itself; a compaction pass finding one of "
            "these stale again is this residual, not a new one."
        ),
        "source": "docs/reports/docs-compact-8/README.md",
        "recorded_by": "DOCS-COMPACT-8",
    },
]

d = json.loads(P.read_text(encoding="utf-8"))
have = {e["id"] for e in d["named_followups"]}
for e in NEW:
    assert e["id"] not in have, f"already registered: {e['id']}"
before = len(d["named_followups"])
d["named_followups"].extend(NEW)
P.write_text(json.dumps(d, indent=1, ensure_ascii=False) + "\n", encoding="utf-8")
print(f"named_followups {before} -> {len(d['named_followups'])}")
