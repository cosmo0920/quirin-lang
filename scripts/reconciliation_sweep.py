#!/usr/bin/env python3
"""Integrator reconciliation sweep (card `W32-RECONCILE-SWEEP`, wave 32).

Cards close with an explicit section addressed to the integrator — "Integrator
reconciliation list", "For the integrator", "Integrator carry list", "Proposed
amendment text for standing constraint N", "Proposed proof-stage evidence (the
integrator's flip)", "… left for the integrator". Three consecutive wave closes
applied those sections' *headline numbers* and left the rest
(`DOCS-COMPACT-4-TABLE-SYNC`, then `DOCS-COMPACT-5-RECONCILIATION-LIST`, whose
concrete casualty was an entire unlanded standing-constraint amendment). Both
findings failed for the same reason: the remedy was advice, and advice is
skippable.

This module is the mechanical half. It does exactly two things:

  * `sweep(root, cards)` — find every integrator-addressed section in the named
    card reports and split it into individually addressable **items**, each with
    a stable id `<card>#<section-slug>#<n>`.
  * `check(root, record_path)` — refuse a wave-close reconciliation record that
    does not carry a verdict for every swept item.

**What this tool does NOT do, stated plainly because the alternative would be
dishonest:** it does not decide whether an item was applied, and it cannot. An
item is a paragraph of English addressed to a person; only a person can measure
the tree against it. A green `--check` therefore means *"every item has a
recorded human verdict"* and never *"every item was applied"*. It is a
completeness gate on the close record, not on the repository.

The verdict vocabulary is deliberately small and includes a refusal arm, so a
close can say "no" out loud instead of by silence:

  * `applied`     — the item landed; `note` should say where, so the claim is
                    checkable against the tree.
  * `declined`    — deliberately not applied; `note` carries the ground.
  * `superseded`  — a later card replaced the item; `note` names that card.
  * `deferred`    — carried to a named later wave; `note` says which and why.
  * `noted`       — the block carries no action: an introductory sentence, a
                    suggested reading, or a statement of what the card itself
                    already did. `note` says what was checked to conclude that.
                    It exists so that `applied` never has to stretch, and it
                    demands a note for exactly the same reason.

Every verdict except `applied` requires a note. `applied` does not, but a bare
`applied` is worth little: say where it landed so the claim is checkable.

Record shape — `docs/reports/wave<N>-close/reconciliation.json`:

    {
      "wave": 32,
      "cards": ["docs-compact-5", "w32-reconcile-sweep"],
      "note": "free prose",
      "verdicts": {
        "<item id>": {"verdict": "applied", "note": "AGENTS.md constraint 15h"}
      },
      "no_section": {"<card>": "why this card owed the integrator nothing"}
    }

A swept card with no recognised section is a complaint, not a pass: either the
heading is a new spelling this module has not learned, or the card genuinely
owed nothing — and a person has to say which, in `no_section`. That map is
checked in both directions, so it cannot be used to wave away a card whose
section the sweep *did* find.

`--residuals` is the second half of the same failure, found by this card's own
sweep: a card names a residual in its follow-on table, the close applies the
headline numbers, and the name reaches no tracked ledger at all. It lists every
`SCREAMING-KEBAB` name a swept report writes in backticks that appears in
**neither** `docs/reviews/r0_3/acceptance_matrix.json`'s `named_followups`
**nor** `AGENTS.md`/`CLAUDE.md`. These are candidates for a human, not defects:
plenty of such names are test names, diagnostic classes or one-off emphasis.
`DOCS-COMPACT-4-TABLE-SYNC` and `DOCS-COMPACT-5-RECONCILIATION-LIST` — the two
findings that named this very defect — were both in that untracked set.

Usage:
    python3 scripts/reconciliation_sweep.py [ROOT] --cards NAME [NAME ...]
    python3 scripts/reconciliation_sweep.py [ROOT] --since ORIGIN/MAIN
    python3 scripts/reconciliation_sweep.py [ROOT] --check RECORD [--cards ...]
    python3 scripts/reconciliation_sweep.py [ROOT] --cards NAME --residuals

`--since REF` asks git which report directories changed since REF and sweeps
those; it needs a git checkout and fails loudly rather than silently sweeping
nothing.
"""

from __future__ import annotations

import argparse
import json
import pathlib
import re
import subprocess
import sys
from dataclasses import dataclass

#: Headings that address the integrator. Matched case-insensitively against the
#: heading text of a `##`/`###` line. Kept as substrings rather than exact
#: titles because the eleven wave-29..31 cards spelled the same intent six ways;
#: a new spelling is a one-line addition here, and an unrecognised spelling is
#: visible as a card contributing no items (which `--check` reports, since a
#: swept card with zero items is itself worth a human glance).
SECTION_MARKERS = (
    "for the integrator",
    "integrator reconciliation",
    "integrator carry list",
    "integrator's flip",
    "left for the integrator",
    "owed to the integrator",
    "proposed amendment",
    "proposed proof-stage evidence",
)

VERDICTS = ("applied", "declined", "superseded", "deferred", "noted")

HEADING = re.compile(r"^(#{2,6})\s+(.*?)\s*$")
NUMBERED = re.compile(r"^(\d+)\.\s+(.*)$")
BULLET = re.compile(r"^[*-]\s+(.*)$")
FENCE = re.compile(r"^\s*```")


@dataclass(frozen=True)
class Item:
    """One addressable thing a card asked the integrator to do."""

    card: str
    section: str
    index: int
    text: str

    @property
    def id(self) -> str:
        return f"{self.card}#{slug(self.section)}#{self.index}"


def slug(heading: str) -> str:
    """A stable, filename-safe key for a section heading.

    Leading section numbers are dropped so that renumbering a report's sections
    does not silently orphan a recorded verdict; the words carry the identity.
    """
    text = re.sub(r"^\d+(\.\d+)*\.?\s*", "", heading.lower())
    text = re.sub(r"[^a-z0-9]+", "-", text)
    return text.strip("-") or "section"


def _is_integrator_heading(text: str) -> bool:
    lowered = text.lower()
    return any(marker in lowered for marker in SECTION_MARKERS)


def _first_sentence(text: str, limit: int = 160) -> str:
    flat = " ".join(text.split())
    if len(flat) <= limit:
        return flat
    return flat[: limit - 1].rstrip() + "…"


def items_in_report(card: str, body: str) -> list[Item]:
    """Split one report's integrator-addressed sections into items.

    A section's items are its top-level blocks in document order: each numbered
    entry, each top-level bullet, and each top-level prose paragraph. Prose
    counts because `CT22-WIRE` and `CT-WIRE-LANE-3` put the owed half in a
    closing paragraph *after* a bulleted list of files the lane itself changed —
    read list-only, those sections yield nine items none of which is the one
    thing owed. Continuation lines (indented text, block quotes, fenced code)
    attach to the block they follow, so a proposed JSON or Rust record cannot be
    read as a list of its own. A section ends at the next heading of the same or
    higher level.
    """
    lines = body.splitlines()
    found: list[Item] = []
    index = 0
    while index < len(lines):
        match = HEADING.match(lines[index])
        if not match or not _is_integrator_heading(match.group(2)):
            index += 1
            continue
        level, heading = len(match.group(1)), match.group(2)
        start = index + 1
        end = start
        while end < len(lines):
            nested = HEADING.match(lines[end])
            if nested and len(nested.group(1)) <= level:
                break
            end += 1
        found.extend(_section_items(card, heading, lines[start:end]))
        index = end
    return found


def _section_items(card: str, heading: str, lines: list[str]) -> list[Item]:
    blocks: list[list[str]] = []
    current: list[str] | None = None
    fenced = False
    for line in lines:
        if FENCE.match(line):
            fenced = not fenced
            if current is None:
                current = []
                blocks.append(current)
            current.append(line.strip())
            continue
        if fenced:
            if current is None:
                current = []
                blocks.append(current)
            current.append(line.strip())
            continue
        if not line.strip():
            current = None
            continue
        number = NUMBERED.match(line)
        bullet = BULLET.match(line)
        if number and not line.startswith((" ", "\t")):
            current = [number.group(2)]
            blocks.append(current)
        elif bullet and not line.startswith((" ", "\t")):
            current = [bullet.group(1)]
            blocks.append(current)
        elif current is not None:
            current.append(line.strip())
        else:
            current = [line.strip()]
            blocks.append(current)
    return [
        Item(card, heading, position, _first_sentence(" ".join(chunk)))
        for position, chunk in enumerate(blocks, start=1)
        if " ".join(chunk).strip()
    ]


def sweep(root: pathlib.Path, cards: list[str]) -> list[Item]:
    """Every integrator-addressed item across the named card reports."""
    found: list[Item] = []
    for card in cards:
        report = root / "docs" / "reports" / card / "README.md"
        if not report.is_file():
            raise SystemExit(f"no such card report: {report}")
        found.extend(items_in_report(card, report.read_text(encoding="utf-8")))
    return found


#: A residual name as the reports spell them: backticked SCREAMING-KEBAB with at
#: least one hyphen and at least two segments, so `AGENTS` or `JSON` cannot match.
RESIDUAL = re.compile(r"`([A-Z][A-Z0-9]*(?:-[A-Z0-9]+)+)`")

#: Trackers a residual name may legitimately live in instead of the matrix.
TRACKERS = (
    "docs/reviews/r0_3/acceptance_matrix.json",
    "AGENTS.md",
    "CLAUDE.md",
)


def untracked_residuals(root: pathlib.Path, cards: list[str]) -> dict[str, list[str]]:
    """Residual-shaped names a swept report writes that no tracker mentions.

    Candidates for a human eye, never a verdict: the regex cannot tell a residual
    from a profile name or a test id, and deliberately does not try.
    """
    tracked = "\n".join(
        (root / name).read_text(encoding="utf-8") for name in TRACKERS if (root / name).is_file()
    )
    found: dict[str, list[str]] = {}
    for card in cards:
        report = root / "docs" / "reports" / card / "README.md"
        if not report.is_file():
            raise SystemExit(f"no such card report: {report}")
        names = sorted(
            {
                match.group(1)
                for match in RESIDUAL.finditer(report.read_text(encoding="utf-8"))
                if match.group(1) not in tracked
            }
        )
        if names:
            found[card] = names
    return found


def cards_changed_since(root: pathlib.Path, ref: str) -> list[str]:
    """Report directories git says changed since `ref`.

    Fails loudly: a sweep that quietly covered nothing is the failure mode this
    whole module exists to remove.
    """
    finished = subprocess.run(
        ["git", "diff", "--name-only", ref, "HEAD", "--", "docs/reports"],
        cwd=root,
        capture_output=True,
        text=True,
        check=False,
    )
    if finished.returncode != 0:
        raise SystemExit(f"git diff against {ref!r} failed: {finished.stderr.strip()}")
    names = sorted(
        {
            line.split("/")[2]
            for line in finished.stdout.splitlines()
            if line.startswith("docs/reports/") and len(line.split("/")) > 3
        }
    )
    return [
        name for name in names if (root / "docs" / "reports" / name / "README.md").is_file()
    ]


def check(root: pathlib.Path, record_path: pathlib.Path, cards: list[str] | None = None):
    """Complaints about a wave-close reconciliation record, as strings.

    An empty list means every swept item carries a verdict from the recognised
    vocabulary — not that any item was applied. See this module's docstring.
    """
    complaints: list[str] = []
    try:
        record = json.loads(record_path.read_text(encoding="utf-8"))
    except FileNotFoundError:
        return [f"no reconciliation record at {record_path}"]
    except json.JSONDecodeError as error:
        return [f"{record_path} is not JSON: {error}"]
    if not isinstance(record, dict):
        return [f"{record_path} is not a JSON object"]
    listed = cards if cards is not None else record.get("cards")
    if not listed:
        return [f"{record_path} names no cards and none were supplied"]
    if not isinstance(listed, list) or not all(isinstance(name, str) for name in listed):
        return [f"{record_path} 'cards' is not a list of strings"]
    verdicts = record.get("verdicts")
    if not isinstance(verdicts, dict):
        return [f"{record_path} carries no 'verdicts' object"]

    silent = record.get("no_section", {})
    if not isinstance(silent, dict):
        return [f"{record_path} 'no_section' is not an object"]

    items = sweep(root, list(listed))
    swept = {item.id: item for item in items}
    for card in listed:
        if any(item.card == card for item in items):
            continue
        if str(silent.get(card, "")).strip():
            continue
        complaints.append(
            f"{card}: no integrator-addressed section found "
            "(an unrecognised heading, or genuinely nothing owed - say which "
            "in the record's 'no_section' map)"
        )
    for card in sorted(set(silent) - set(listed)):
        complaints.append(f"{card}: 'no_section' entry for a card that was not swept")
    for card in sorted(set(silent) & {item.card for item in items}):
        complaints.append(
            f"{card}: 'no_section' entry for a card that does have a section - "
            "remove it and give its items verdicts"
        )
    for item_id, item in swept.items():
        entry = verdicts.get(item_id)
        if entry is None:
            complaints.append(f"{item_id}: no verdict recorded - {item.text}")
            continue
        if not isinstance(entry, dict):
            complaints.append(f"{item_id}: verdict entry is not an object")
            continue
        verdict = entry.get("verdict")
        if verdict not in VERDICTS:
            complaints.append(
                f"{item_id}: verdict {verdict!r} is not one of {', '.join(VERDICTS)}"
            )
            continue
        if verdict != "applied" and not str(entry.get("note", "")).strip():
            complaints.append(f"{item_id}: verdict {verdict!r} carries no note")
    for item_id in sorted(set(verdicts) - set(swept)):
        complaints.append(f"{item_id}: verdict recorded for an item the sweep does not find")
    return complaints


def _render(items: list[Item]) -> str:
    lines = []
    card = None
    for item in items:
        if item.card != card:
            card = item.card
            lines.append(f"\n{card}")
        lines.append(f"  {item.id}")
        lines.append(f"      [{item.section}] {item.text}")
    return "\n".join(lines).lstrip("\n")


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("root", nargs="?", default=".")
    parser.add_argument("--cards", nargs="+", default=None)
    parser.add_argument("--since", default=None, help="git ref; sweep reports changed since it")
    parser.add_argument("--check", default=None, help="a wave-close reconciliation record")
    parser.add_argument(
        "--residuals",
        action="store_true",
        help="list residual-shaped names the swept reports write that no tracker mentions",
    )
    parser.add_argument("--json", action="store_true")
    args = parser.parse_args(argv)
    root = pathlib.Path(args.root).resolve()

    cards = args.cards
    if cards is None and args.since is not None:
        cards = cards_changed_since(root, args.since)
        if not cards:
            raise SystemExit(f"no card reports changed since {args.since!r}")

    if args.check:
        complaints = check(root, pathlib.Path(args.check), cards)
        if complaints:
            print(f"reconciliation record incomplete ({len(complaints)}):")
            for complaint in complaints:
                print(f"  {complaint}")
            return 1
        print(
            "every swept item carries a recorded verdict. "
            "This says nothing about whether any item was applied - "
            "read the notes."
        )
        return 0

    if cards is None:
        raise SystemExit("supply --cards or --since")

    if args.residuals:
        found = untracked_residuals(root, cards)
        if args.json:
            print(json.dumps(found, indent=2))
            return 0
        total = 0
        for card, names in found.items():
            print(f"\n{card}")
            for name in names:
                print(f"  {name}")
                total += 1
        print(f"\n{total} residual-shaped names in no tracker ({', '.join(TRACKERS)}).")
        print("Candidates for a human eye; this tool classifies none of them.")
        return 0

    items = sweep(root, cards)
    if args.json:
        print(
            json.dumps(
                [
                    {
                        "id": item.id,
                        "card": item.card,
                        "section": item.section,
                        "text": item.text,
                    }
                    for item in items
                ],
                indent=2,
            )
        )
    else:
        print(_render(items))
        print(f"\n{len(items)} integrator-addressed items across {len(cards)} cards.")
        print("Each needs a human verdict; this tool decides none of them.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
