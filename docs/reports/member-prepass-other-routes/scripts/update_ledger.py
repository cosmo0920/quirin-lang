#!/usr/bin/env python3
"""update_ledger.py RUN -- MEMBER-PREPASS-OTHER-ROUTES: the acceptance
matrix's named follow-ups (indent=1, ensure_ascii=False;
`dimensions.named_followups` = list length). Idempotent. RUN is the
measured-run sentence the entries quote.

* Retired: `MEMBER-PREPASS-OTHER-ROUTES` (its DISCHARGE holds: the pre-pass
  on the current and no-build routes over their compile databases, a
  checkpoint carrying the run's facts, measured on the system and slice
  routes).
* Registered: `MEMBER-SCOPE-RECHECK-RAW-TEXT` (the re-check reads raw text
  and generator sources, not the excluded unit's preprocessed text) and
  `CHECKPOINT-COMPOSER-DEMOTING-FOLD-REFUSES` (a composer that would demote
  refuses rather than rewriting rows)."""
import json
import pathlib
import sys

REPO = pathlib.Path(__file__).resolve().parents[4]
MATRIX = REPO / "docs/reviews/r0_3/acceptance_matrix.json"
CARD = "MEMBER-PREPASS-OTHER-ROUTES"
SOURCE = "docs/reports/member-prepass-other-routes/README.md"

RETIRE = ["MEMBER-PREPASS-OTHER-ROUTES"]


def register(run):
    return [
        {
            "id": "MEMBER-SCOPE-RECHECK-RAW-TEXT",
            "title": "The member-facts scope re-check reads raw text, not the excluded unit's preprocessed text",
            "why": (
                f"Registered by {CARD} ({run}). The pre-pass re-verifies each "
                "`member-facts-scope.json` exclusion at run time (rule 12n): a tokens-absent "
                "exclusion holds when the unit's raw text, every file Clang resolves for it "
                "under its own command (`-M -MG`) and the named generator sources name no "
                "final token. An excluded unit is excluded because its own command does not "
                "preprocess (a header generated at build time is absent) or it is itself "
                "generated, so its preprocessed text is not available: a token formed by "
                "`##` pasting or by a macro defined in an absent generated header, and the "
                "generated part of a flex/bison or buildvm output beyond its generator's "
                "text, are not seen. Sizes: 14 entries, 100 units (12 not-linked "
                "rdkafka++ units, 88 tokens-absent), all held on the corpus (66910c10), "
                "system (046accf6) and slice (34155955) trees. DISCHARGE: the excluded units' "
                "generated inputs produced (or the generated headers stubbed with their "
                "generators' declarations) so each excluded C unit is preprocessed and read "
                "like any other, or a stated premise accepted by the user for the remaining "
                "text-level check."
            ),
            "source": SOURCE,
            "recorded_by": CARD,
        },
        {
            "id": "CHECKPOINT-COMPOSER-DEMOTING-FOLD-REFUSES",
            "title": "A checkpoint composer refuses, rather than publishes, a member fold that demotes rows",
            "why": (
                f"Registered by {CARD} ({run}). A checkpoint (schema 5) carries the run's "
                "member facts, the fold's environment and each unit's member and state facts, "
                "and the composer folds them as the whole run does (rule 12n, "
                "`callback_member_fold::fold_in`). It writes rows as the checkpoints hold "
                "them, so a fold that would demote a row is refused. With the run's facts "
                "installed the member part demotes nothing "
                "(`QCore.MemberPrepassRoutes.a_run_decided_over_its_facts_composes`); a "
                "constructor's result-state part (rules 12o, 13w, 14e) can, and then a "
                "checkpointed run of those contracts does not compose. Sizes: the system "
                "route composes byte-identically with 0 demotions (this card's measurement); "
                "the corpus is not run with checkpoints. DISCHARGE: the composer applies the "
                "fold's demotions to the rows, tallies and diagnostic counts it streams (and "
                "their null and bounds columns, once checkpoints carry those rows), proved "
                "equal to the whole run's."
            ),
            "source": SOURCE,
            "recorded_by": CARD,
        },
    ]


def main():
    run = sys.argv[1]
    matrix = json.loads(MATRIX.read_text())
    entries = matrix["named_followups"]
    before = len(entries)
    entries = [entry for entry in entries if entry["id"] not in RETIRE]
    known = {entry["id"] for entry in entries}
    for entry in register(run):
        if entry["id"] not in known:
            entries.append(entry)
    matrix["named_followups"] = entries
    matrix["dimensions"]["named_followups"] = len(entries)
    MATRIX.write_text(json.dumps(matrix, indent=1, ensure_ascii=False) + "\n")
    print(f"named_followups {before} -> {len(entries)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
