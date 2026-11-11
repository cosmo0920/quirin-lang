#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
#
# Licensed under the Apache License, Version 2.0 (the "License");
# you may not use this file except in compliance with the License.
# You may obtain a copy of the License at
#
#     http://www.apache.org/licenses/LICENSE-2.0
#
# Unless required by applicable law or agreed to in writing, software
# distributed under the License is distributed on an "AS IS" BASIS,
# WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
# See the License for the specific language governing permissions and
# limitations under the License.

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

This module is the mechanical half. It does these things:

  * `sweep(root, cards)` — find every integrator-addressed section in the named
    card reports and split it into individually addressable **items**, each with
    a stable id `<card>#<section-slug>#<n>`.
  * `check(root, record_path)` — refuse a wave-close reconciliation record that
    does not carry a verdict for every swept item, and re-list every deferred
    decision an earlier close left open.
  * `untracked_residuals(root, cards)` — list residual-shaped names, from the
    card reports *and* the other documents that can introduce one, that the
    follow-on ledger does not carry.
  * `uncarded_commits(root, base, head)` — list commits in a range that write no
    card report at all, so the seam between two waves' records is visible.

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

Every verdict except `applied` requires a note. `applied` requires something
stronger, since card `STABILITY-REEVAL` #2 (wave 36) found a wave-35 verdict
recorded `applied` whose asserted text **was not at the named location**
(`RECONCILIATION-APPLIED-ASSERTS-UNVERIFIED`): the `where` field is prose, and
prose naming a file is not a measurement of that file. An `applied` entry must
therefore carry **one of two things**, and `--check` reads both:

  * `verify` — a list of `{"file": <path>, "contains": <text>}` or
    `{"file": <path>, "absent": <text>}` claims. Each is opened and read at
    check time. Whitespace is normalised on both sides, so a quotation the tree
    wraps at its own column still matches the words a person would verify by
    eye; nothing else is normalised, so a changed word is a changed claim.
    `absent` is the other half — "this stale sentence now occurs zero times" is
    exactly as checkable as its replacement's presence, and a close that only
    ever asserts presence cannot catch a duplicated amendment.
  * `unverifiable` — a ground, in words, for why this `applied` claim has no
    checkable text: it landed as a regenerated artifact, a run that happened, a
    deletion with no successor sentence. The escape hatch is deliberate and
    deliberately visible: `--check` counts these and prints the count, so a
    close that discharges its evidence by declaring it cannot be checked says so
    out loud rather than by omission.

Carrying both is refused, and so is `unverifiable` on any verdict that is not
`applied`, a non-`applied` verdict asserting no landing. A `verify` list is
honoured on any verdict, so a `superseded` entry can assert the superseding
text.

The **demand** begins at wave `VERIFY_APPLIED_FROM_WAVE`; a `verify` list is
read at every wave. See that constant for why the two halves are separated —
the short version is that the wave-33..35 records are frozen archives
(constraint 21) and an unconditional demand would have left only two moves,
both of them the failure this repository names.

This arm does not make the tool decide anything. It decides whether a **string
the close itself wrote down** is where the close said it is. Choosing the
string, and judging that it is the right string, stays the person's.

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
`SCREAMING-KEBAB` name written in backticks that the follow-on ledger does not
carry. These are candidates for a human, not defects: plenty of such names are
test names, diagnostic classes or one-off emphasis.
`DOCS-COMPACT-4-TABLE-SYNC` and `DOCS-COMPACT-5-RECONCILIATION-LIST` — the two
findings that named this very defect — were both in that untracked set.

Card `SWEEP-BLIND-SPOTS` (wave 35) repaired four measured blind spots in the
above, each found by `STABILITY-REEVAL` #1 (wave 34) and each carrying its own
named residual. They are recorded here because each one is a rule about what
this tool can and cannot see, and a future refactor that quietly restores the
old behaviour would restore the defect with it.

  1. **`SWEEP-RESIDUALS-SCANS-REPORTS-ONLY`.** `--residuals` read only
     `docs/reports/`, so a residual introduced anywhere else was invisible *by
     construction*. It now also reads `docs/development/increments/` and the
     standing-constraints text of `AGENTS.md` and `CLAUDE.md` (see
     `RESIDUAL_SOURCES`). The occasioning miss: `CT-SHARED-FIXTURE-DISCLOSURE`,
     which the wave-33 close's own run of this tool did not return.
  2. **`RESIDUAL-NAMED-ONLY-IN-A-CONSTRAINT-IS-UNTRACKED-WORK`.** `TRACKERS`
     counted an `AGENTS.md` or `CLAUDE.md` mention as tracking. It no longer
     does: **the follow-on ledger — `named_followups` in the acceptance matrix
     — is the only tracker.** A constraint paragraph states a *rule that must
     survive refactors*; the ledger states *work that is open*, and a name can
     be in the first and still be untracked as the second. That is not a
     hypothetical: `RETURN-POSITION-ESCAPE-UNREACHABLE` and
     `RETURN-POSITION-ACCUSING-UNEXERCISED-IN-CORPUS` sat in constraint 5b with
     no ledger entry and this tool was silent about both. A name found only in
     those files is now reported untracked **with where it was found**, because
     "untracked, but AGENTS.md discusses it" and "untracked, and nothing
     anywhere mentions it" want different human answers.
  3. **`WAVE-BASE-UNSWEPT`.** The `--since` range was exclusive of its own base
     commit, so a commit that *is* the declared base was invisible to the close
     that declared it. `37bc9004` is the live instance: the wave-33 base, after
     the wave-32 close, described by neither wave's increment record, indexed by
     no report, and swept by nothing — while moving two pinned F02 digests and
     removing four tests from the Windows provider gate. `--since` is therefore
     **inclusive of its base** now, `--range A..B` takes a seam explicitly, and
     `--uncarded` lists the commits in a range that write no card report at all.
     `cards_changed_since` keeps its exclusive meaning as a primitive; the
     inclusive range is `cards_changed_in_range(..., include_base=True)`.
  4. **`WAVE-33-DEFERRED-DECISIONS-UNTRACKED`.** A close may defer a decision —
     a valid outcome, and `CLAUDE.md` says so. But `check()` refuses a verdict
     for an item it cannot find, so a deferral could only ever live in the one
     JSON file that recorded it, and expired in silence. A close's
     `deferred_decisions` (in the record, or in a `deferred_decisions.json`
     beside it, so a past close's record need not be edited — constraint 21) is
     now **re-listed at every later close** until it carries `resolved` or
     `withdrawn` with a ground. A deferral that nobody re-states is a complaint,
     not a pass.

**What this tool does NOT do, and none of the four above changes it:** it does
not decide whether an item was applied, whether a residual-shaped name is a
residual, or whether an uncarded commit needed a card. It cannot. An item is a
paragraph of English addressed to a person; only a person can measure the tree
against it. A green `--check` means *"every item has a recorded human verdict
and every open deferral was re-stated"* and never *"every item was applied"*.

Usage:
    python3 scripts/reconciliation_sweep.py [ROOT] --cards NAME [NAME ...]
    python3 scripts/reconciliation_sweep.py [ROOT] --since PREVIOUS/CLOSE
    python3 scripts/reconciliation_sweep.py [ROOT] --range A..B
    python3 scripts/reconciliation_sweep.py [ROOT] --check RECORD [--cards ...]
    python3 scripts/reconciliation_sweep.py [ROOT] --cards NAME --residuals
    python3 scripts/reconciliation_sweep.py [ROOT] --range A..B --uncarded

`--since REF` asks git which report directories changed in `REF^..HEAD` —
**including REF's own commit** — and sweeps those; it needs a git checkout and
fails loudly rather than silently sweeping nothing.
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
    "integrator flip",
    "left for the integrator",
    "owed to the integrator",
    "proposed amendment",
    "proposed proof-stage evidence",
)

VERDICTS = ("applied", "declined", "superseded", "deferred", "noted")

#: Verdicts a *deferred decision* may carry at a close. `deferred` re-states the
#: deferral, which is valid and keeps the decision visible; `resolved` and
#: `withdrawn` end it. All three demand a ground — a deferral restated without
#: one is how a decision expires quietly.
#:
#: The vocabulary and the record shape are **read from the wave-33 close, not
#: invented here**: that close already wrote `deferred_decisions` with a `note`,
#: an `items` list and a `ground` per entry, having reasoned correctly that a
#: decision is not a swept item and so cannot go in `verdicts`. It got
#: everything right except that nothing would ever read it again, which is the
#: half this card supplies. Its file is therefore valid unchanged (constraint
#: 21) and was not edited.
DECISION_VERDICTS = ("deferred", "resolved", "withdrawn")

#: The verdicts that leave a decision outstanding, to be re-listed at the next
#: close.
DECISION_OPEN = ("deferred",)

#: The two claim kinds an `applied` verdict's `verify` list may carry. Exactly
#: one per claim: a claim asserting both presence and absence of the same text
#: is a confusion, not a stronger claim.
VERIFY_KINDS = ("contains", "absent")

#: The first wave whose close must give every `applied` verdict either checkable
#: text or a written ground.
#:
#: **Why a wave number and not an unconditional rule.** A `verify` list is read
#: wherever it appears, at every wave; what this number gates is the *demand*.
#: The wave-32 retrospective record and the wave-33..35 closes are **frozen
#: archives** (constraint 21) and a live test pins one of them green. An
#: unconditional demand would leave exactly two moves: edit a frozen archive, or
#: weaken the test around it — and both are the failure this repository names.
#: So the demand begins at the close that landed the arm, history keeps the
#: standard it was written to, and no archive is touched. A record may also set
#: `"verify_applied": true` to opt in earlier; it may **not** set it false to
#: opt out, which would restore the hole.
VERIFY_APPLIED_FROM_WAVE = 36

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

#: **The** tracker, singular, and the whole of it
#: (`RESIDUAL-NAMED-ONLY-IN-A-CONSTRAINT-IS-UNTRACKED-WORK`). The follow-on
#: ledger is the list of work that is open; nothing else in the tree is. An
#: `AGENTS.md` constraint paragraph is a *rule*, and a rule is not a tracker.
MATRIX = "docs/reviews/r0_3/acceptance_matrix.json"
TRACKERS = (f"{MATRIX}:named_followups",)

#: Documents that are *not* trackers but whose mention of a name is worth
#: reporting beside it, because "untracked, and AGENTS.md states a rule about
#: it" and "untracked, and nothing anywhere mentions it" want different answers.
MENTION_ONLY = ("AGENTS.md", "CLAUDE.md")


def rule_documents(root: pathlib.Path) -> tuple[str, ...]:
    """Include routed policies after documentation restructuring.

    These remain mention-only evidence, never tracker registrations. Limit the
    discovery to the canonical policy directory, not archived checkpoints.
    """
    routed = sorted(
        path.relative_to(root).as_posix()
        for path in (root / "docs" / "agents").glob("*.md")
        if path.is_file()
    )
    platform = "docs/development/platform-evidence.md"
    if (root / platform).is_file():
        routed.append(platform)
    return (*MENTION_ONLY, *routed)


@dataclass(frozen=True)
class ResidualName:
    """One residual-shaped name the ledger does not carry."""

    name: str
    #: Documents that *write* the name, relative to the root.
    sources: tuple[str, ...]
    #: Non-tracker documents that mention it (see `rule_documents`).
    mentions: tuple[str, ...]


def ledger_text(root: pathlib.Path) -> str:
    """The follow-on ledger's own text — the only surface that counts as tracking.

    Serialized rather than reduced to ids on purpose: a residual is legitimately
    tracked by being named inside another entry's prose, which is how
    `WAVE-33-DEFERRED-DECISIONS-UNTRACKED` deliberately registers four names as
    one entry.
    """
    matrix = root / MATRIX
    if not matrix.is_file():
        return ""
    try:
        loaded = json.loads(matrix.read_text(encoding="utf-8"))
    except json.JSONDecodeError:
        return ""
    return json.dumps(loaded.get("named_followups", []), indent=1)


def constraint_text(root: pathlib.Path, name: str) -> str:
    """The standing-constraints text of a rules file.

    A file with no standing-constraints heading is read whole, and the reader
    says so rather than silently returning nothing — `CLAUDE.md` is that case,
    and its platform records name residuals exactly as a constraint does.
    """
    path = root / name
    if not path.is_file():
        return ""
    lines = path.read_text(encoding="utf-8").splitlines()
    kept: list[str] = []
    level = 0
    for line in lines:
        heading = HEADING.match(line)
        if heading:
            if level and len(heading.group(1)) <= level:
                level = 0
            if "standing constraint" in heading.group(2).lower():
                level = len(heading.group(1))
        if level:
            kept.append(line)
    return "\n".join(kept) if kept else "\n".join(lines)


def residual_sources(root: pathlib.Path, cards: list[str]) -> dict[str, str]:
    """Every document `--residuals` reads, as {label: text}.

    `SWEEP-RESIDUALS-SCANS-REPORTS-ONLY`: the card reports are the first of four
    kinds of document that can introduce a residual name, not the only one.
    """
    sources: dict[str, str] = {}
    for card in cards:
        report = root / "docs" / "reports" / card / "README.md"
        if not report.is_file():
            raise SystemExit(f"no such card report: {report}")
        sources[f"docs/reports/{card}/README.md"] = report.read_text(encoding="utf-8")
    increments = root / "docs" / "development" / "increments"
    if increments.is_dir():
        for record in sorted(increments.glob("*.md")):
            sources[f"docs/development/increments/{record.name}"] = record.read_text(
                encoding="utf-8"
            )
    for name in rule_documents(root):
        text = constraint_text(root, name)
        if text:
            sources[f"{name} (standing constraints)"] = text
    return sources


def untracked_residuals(root: pathlib.Path, cards: list[str]) -> list[ResidualName]:
    """Residual-shaped names the follow-on ledger does not carry.

    Candidates for a human eye, never a verdict: the regex cannot tell a residual
    from a profile name or a test id, and deliberately does not try.
    """
    tracked = ledger_text(root)
    mentions = {name: (root / name).read_text(encoding="utf-8") for name in rule_documents(root) if (root / name).is_file()}
    where: dict[str, set[str]] = {}
    for label, text in residual_sources(root, cards).items():
        for match in RESIDUAL.finditer(text):
            if match.group(1) in tracked:
                continue
            where.setdefault(match.group(1), set()).add(label)
    return [
        ResidualName(
            name=name,
            sources=tuple(sorted(labels)),
            mentions=tuple(sorted(doc for doc, text in mentions.items() if name in text)),
        )
        for name, labels in sorted(where.items())
    ]


def _git(root: pathlib.Path, *args: str) -> str:
    finished = subprocess.run(
        ["git", *args], cwd=root, capture_output=True, text=True, check=False
    )
    if finished.returncode != 0:
        raise SystemExit(f"git {' '.join(args)} failed: {finished.stderr.strip()}")
    return finished.stdout


def cards_changed_in_range(
    root: pathlib.Path, base: str, head: str = "HEAD", include_base: bool = False
) -> list[str]:
    """Report directories git says changed between `base` and `head`.

    `include_base` closes the `WAVE-BASE-UNSWEPT` seam: git's `A B` diff is
    exclusive of `A`'s own commit, so a wave whose declared base is the commit
    that landed some change never sweeps that change. With `include_base` the
    range starts at `base^` and the base commit's own work is in it.

    Fails loudly: a sweep that quietly covered nothing is the failure mode this
    whole module exists to remove.
    """
    start = f"{base}^" if include_base else base
    output = _git(root, "diff", "--name-only", start, head, "--", "docs/reports")
    names = sorted(
        {
            line.split("/")[2]
            for line in output.splitlines()
            if line.startswith("docs/reports/") and len(line.split("/")) > 3
        }
    )
    return [
        name for name in names if (root / "docs" / "reports" / name / "README.md").is_file()
    ]


def cards_changed_since(root: pathlib.Path, ref: str) -> list[str]:
    """Report directories changed strictly after `ref`.

    Retained with its original exclusive meaning as the primitive; the close's
    own sweep goes through `cards_changed_in_range(..., include_base=True)`.
    """
    return cards_changed_in_range(root, ref, "HEAD", include_base=False)


def parse_range(spec: str) -> tuple[str, str]:
    """`A..B` into its two ends. One end may be empty and defaults sensibly."""
    if ".." not in spec:
        raise SystemExit(f"--range wants A..B, got {spec!r}")
    base, _, head = spec.partition("..")
    return base.strip() or "HEAD^", head.strip() or "HEAD"


def uncarded_commits(
    root: pathlib.Path, base: str, head: str = "HEAD", include_base: bool = True
) -> list[tuple[str, str, int]]:
    """Commits in a range that write no card report at all.

    The mechanical rule, stated so it can be argued with: a commit is *claimed*
    when it writes some `docs/reports/<card>/README.md`. That is a **necessary
    condition for being carded, not a sufficient one**, and it is not the same
    question as "did this commit need a card". A card's own code commits write
    no report and land here; so does a merge that carries one; so does a genuine
    seam commit like `37bc9004`. The tool classifies none of them — it prints
    the sha, the subject and how many files the commit touched, and a person
    reads the list. That is the whole intent: make the seam **visible**, since
    what `WAVE-BASE-UNSWEPT` measured was a commit nothing looked at.
    """
    start = f"{base}^" if include_base else base
    shas = _git(root, "log", "--format=%H", f"{start}..{head}").split()
    found: list[tuple[str, str, int]] = []
    for sha in shas:
        files = [
            line
            for line in _git(
                root,
                "diff-tree",
                "-r",
                "--no-commit-id",
                "--name-only",
                "-m",
                "--first-parent",
                sha,
            ).splitlines()
            if line.strip()
        ]
        claims = any(
            line.startswith("docs/reports/") and line.endswith("/README.md") for line in files
        )
        if claims:
            continue
        subject = _git(root, "log", "-1", "--format=%s", sha).strip()
        found.append((sha[:8], subject, len(files)))
    return found


CLOSE_DIR = re.compile(r"^wave(\d+)-close$")


def decisions_in(block) -> dict[str, dict]:
    """Deferred decisions out of a `deferred_decisions` block, either shape.

    Two shapes are read because the wave-33 close wrote one before this checker
    existed and must not be edited to suit it (constraint 21):

      * `{"note": ..., "items": [{"name": N, "verdict": V, "ground": G}, ...]}`
        — the wave-33 close's own, and the one to write;
      * `{N: {"verdict": V, ...}, ...}` — a plain map, accepted so a close can
        register a decision without ceremony.
    """
    if not isinstance(block, dict):
        return {}
    items = block.get("items")
    if isinstance(items, list):
        return {
            entry["name"]: entry
            for entry in items
            if isinstance(entry, dict) and isinstance(entry.get("name"), str)
        }
    return {
        name: entry
        for name, entry in block.items()
        if isinstance(entry, dict) and name != "note"
    }


def _decisions_of(directory: pathlib.Path) -> dict[str, dict]:
    """One close's deferred decisions, from its record or from a file beside it.

    The sibling `deferred_decisions.json` exists so that a close whose record was
    already written — and which must not be edited (constraint 21) — can have its
    deferrals registered retrospectively in a **new file** beside it.
    """
    found: dict[str, dict] = {}
    for filename, key in (
        ("deferred_decisions.json", None),
        ("reconciliation.json", "deferred_decisions"),
    ):
        path = directory / filename
        if not path.is_file():
            continue
        try:
            loaded = json.loads(path.read_text(encoding="utf-8"))
        except json.JSONDecodeError:
            continue
        if not isinstance(loaded, dict):
            continue
        block = loaded.get(key, {}) if key else loaded.get("deferred_decisions", loaded)
        found.update(decisions_in(block))
    return found


def outstanding_decisions(root: pathlib.Path, before_wave: int) -> dict[str, dict]:
    """Decisions an earlier close deferred and no later close has ended.

    A decision's state is whatever the **latest** close to mention it says. Once
    that is `resolved` or `withdrawn` it drops out; while it is `open` it is
    re-listed at every close after it, which is the whole point
    (`WAVE-33-DEFERRED-DECISIONS-UNTRACKED`): a deferral is a valid outcome, and
    an outcome nobody ever restates is indistinguishable from a forgotten one.
    """
    reports = root / "docs" / "reports"
    latest: dict[str, tuple[int, dict]] = {}
    if not reports.is_dir():
        return {}
    for directory in sorted(reports.iterdir()):
        match = CLOSE_DIR.match(directory.name) if directory.is_dir() else None
        if not match:
            continue
        wave = int(match.group(1))
        if wave >= before_wave:
            continue
        for name, entry in _decisions_of(directory).items():
            if name not in latest or latest[name][0] < wave:
                latest[name] = (wave, {**entry, "wave": wave})
    return {
        name: entry
        for name, (_, entry) in sorted(latest.items())
        if entry.get("verdict", DECISION_OPEN[0]) in DECISION_OPEN
    }


def _check_decisions(root: pathlib.Path, record: dict, record_path: pathlib.Path) -> list[str]:
    complaints: list[str] = []
    block = record.get("deferred_decisions", {})
    if not isinstance(block, dict):
        return [f"{record_path} 'deferred_decisions' is not an object"]
    mine = decisions_in(block)
    for name, entry in sorted(mine.items()):
        verdict = entry.get("verdict")
        if verdict not in DECISION_VERDICTS:
            complaints.append(
                f"deferred decision {name}: verdict {verdict!r} is not one of "
                f"{', '.join(DECISION_VERDICTS)}"
            )
            continue
        ground = f"{entry.get('ground', '')} {entry.get('note', '')}"
        if not ground.strip():
            complaints.append(f"deferred decision {name}: verdict {verdict!r} carries no ground")
    wave = record.get("wave")
    if not isinstance(wave, int):
        return complaints
    for name, entry in outstanding_decisions(root, wave).items():
        if name in mine:
            continue
        complaints.append(
            f"deferred decision {name}: left open by the wave-{entry['wave']} close and not "
            "re-listed here - restate it as 'open' with a ground, or end it with "
            "'resolved'/'withdrawn'"
        )
    return complaints


def _normalise(text: str) -> str:
    """Collapse every run of whitespace to a single space.

    A close quotes an amendment out of a card's report; the tree carries it
    wrapped at the target file's own column — `AGENTS.md` at 76, a regenerated
    JSON artifact at none. A literal substring search would therefore report a
    *landed* amendment missing purely because a newline fell inside the
    quotation, which is the failure mode that makes a verification arm get
    switched off. Normalising both sides makes the assertion about the words,
    which is what a person verifying an amendment reads. Nothing else is
    normalised: case, punctuation and every word stay significant, so a softened
    amendment (invariant 1's hazard) is still a changed claim.
    """
    return re.sub(r"\s+", " ", text)


def _excerpt(text: str, limit: int = 140) -> str:
    flat = _normalise(text).strip()
    return repr(flat if len(flat) <= limit else flat[:limit] + " ...")


def _verify_claim(root: pathlib.Path, item_id: str, position: int, claim) -> list[str]:
    """Complaints about one `verify` claim, measured against the tree."""
    where = f"{item_id} verify[{position}]"
    if not isinstance(claim, dict):
        return [f"{where}: claim is not an object"]
    path = claim.get("file")
    if not isinstance(path, str) or not path.strip():
        return [f"{where}: claim names no 'file'"]
    kinds = [kind for kind in VERIFY_KINDS if kind in claim]
    if len(kinds) != 1:
        return [
            f"{where}: a claim carries exactly one of {', '.join(VERIFY_KINDS)}; "
            f"this one carries {len(kinds)}"
        ]
    kind = kinds[0]
    needle = claim[kind]
    if not isinstance(needle, str) or not needle.strip():
        return [f"{where}: {kind!r} is not a non-empty string"]
    target = (root / path).resolve()
    try:
        target.relative_to(root)
    except ValueError:
        return [f"{where}: 'file' {path!r} escapes the repository root"]
    try:
        body = target.read_text(encoding="utf-8")
    except FileNotFoundError:
        return [f"{where}: no file at {path} - the named location does not exist"]
    except (IsADirectoryError, UnicodeDecodeError, PermissionError) as error:
        return [f"{where}: {path} is not readable text ({type(error).__name__})"]
    present = _normalise(needle).strip() in _normalise(body)
    if kind == "contains" and not present:
        return [
            f"{where}: {path} does NOT carry the asserted text {_excerpt(needle)} - "
            "the verdict says 'applied' and the named location disagrees"
        ]
    if kind == "absent" and present:
        return [
            f"{where}: {path} STILL carries the text asserted absent {_excerpt(needle)}"
        ]
    return []


def demands_verified_claims(record: dict) -> bool:
    """Whether this record must give every `applied` verdict evidence.

    True from `VERIFY_APPLIED_FROM_WAVE` on, and for any record that opts in.
    Setting the flag false does not opt out: a close may adopt the standard
    early and may not abandon it late.
    """
    if record.get("verify_applied") is True:
        return True
    wave = record.get("wave")
    return isinstance(wave, int) and wave >= VERIFY_APPLIED_FROM_WAVE


def _check_claims(root: pathlib.Path, verdicts: dict, demand: bool = True) -> list[str]:
    """Complaints about every entry's evidence for its own verdict.

    This is the arm `RECONCILIATION-APPLIED-ASSERTS-UNVERIFIED` asks for. It
    verifies a string the close wrote down against the location the close named,
    and it decides nothing else. A `verify` list is read whether or not `demand`
    is set — an assertion a record volunteers is measured at every wave.
    """
    complaints: list[str] = []
    for item_id in sorted(verdicts):
        entry = verdicts[item_id]
        if not isinstance(entry, dict):
            continue  # already reported by `check`
        verdict = entry.get("verdict")
        claims = entry.get("verify")
        ground = str(entry.get("unverifiable", "")).strip()
        if claims is not None and not isinstance(claims, list):
            complaints.append(f"{item_id}: 'verify' is not a list of claims")
            claims = None
        if verdict == "applied":
            if claims and ground:
                complaints.append(
                    f"{item_id}: carries both 'verify' and 'unverifiable' - an applied claim is "
                    "either checkable text at a named location or declared unverifiable with a "
                    "ground, never both"
                )
            elif demand and not claims and not ground:
                complaints.append(
                    f"{item_id}: verdict 'applied' carries neither 'verify' (checkable text at a "
                    "named location) nor 'unverifiable' (a ground for why the claim is not "
                    "textual) - RECONCILIATION-APPLIED-ASSERTS-UNVERIFIED"
                )
        elif ground:
            complaints.append(
                f"{item_id}: 'unverifiable' on a {verdict!r} verdict, which asserts no landing"
            )
        for position, claim in enumerate(claims or [], start=1):
            complaints.extend(_verify_claim(root, item_id, position, claim))
    return complaints


def claim_summary(verdicts: dict) -> str:
    """One line saying how much of this record was mechanically verified."""
    applied = [
        entry
        for entry in verdicts.values()
        if isinstance(entry, dict) and entry.get("verdict") == "applied"
    ]
    claims = [
        claim
        for entry in verdicts.values()
        if isinstance(entry, dict) and isinstance(entry.get("verify"), list)
        for claim in entry["verify"]
    ]
    files = {
        claim["file"]
        for claim in claims
        if isinstance(claim, dict) and isinstance(claim.get("file"), str)
    }
    absent = sum(1 for claim in claims if isinstance(claim, dict) and "absent" in claim)
    declared = sum(
        1
        for entry in applied
        if str(entry.get("unverifiable", "")).strip()
    )
    return (
        f"{len(claims)} content claims ({len(claims) - absent} present, {absent} absent) "
        f"verified at their named locations across {len(files)} files; "
        f"{len(applied) - declared} of {len(applied)} 'applied' verdicts carry checkable text, "
        f"{declared} declared unverifiable with a ground."
    )


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
    complaints.extend(_check_claims(root, verdicts, demands_verified_claims(record)))
    complaints.extend(_check_decisions(root, record, record_path))
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
    parser.add_argument(
        "--since",
        default=None,
        help="git ref; sweep reports changed in REF^..HEAD - INCLUDING REF's own commit",
    )
    parser.add_argument(
        "--range",
        dest="range_spec",
        default=None,
        help="an explicit A..B range to sweep, for a seam between two closes",
    )
    parser.add_argument("--check", default=None, help="a wave-close reconciliation record")
    parser.add_argument(
        "--residuals",
        action="store_true",
        help="list residual-shaped names the scanned documents write that the ledger does not carry",
    )
    parser.add_argument(
        "--uncarded",
        action="store_true",
        help="list commits in the range that write no card report (needs --since or --range)",
    )
    parser.add_argument("--json", action="store_true")
    args = parser.parse_args(argv)
    root = pathlib.Path(args.root).resolve()

    if args.since is not None and args.range_spec is not None:
        raise SystemExit("--since and --range name the same thing; supply one")

    base, head = None, "HEAD"
    if args.range_spec is not None:
        base, head = parse_range(args.range_spec)
        include_base = False
    elif args.since is not None:
        base, include_base = args.since, True

    cards = args.cards
    if cards is None and base is not None:
        cards = cards_changed_in_range(root, base, head, include_base=include_base)
        if not cards:
            raise SystemExit(
                f"no card reports changed in {base}{'^' if include_base else ''}..{head}"
            )

    if args.uncarded:
        if base is None:
            raise SystemExit("--uncarded needs --since or --range")
        found = uncarded_commits(root, base, head, include_base=include_base)
        if args.json:
            print(
                json.dumps(
                    [{"sha": sha, "subject": subject, "files": count} for sha, subject, count in found],
                    indent=2,
                )
            )
            return 0
        for sha, subject, count in found:
            print(f"  {sha}  {subject}  ({count} files)")
        print(f"\n{len(found)} commits in the range write no docs/reports/<card>/README.md.")
        print(
            "Writing no report is NECESSARY for being uncarded, not sufficient: a card's own "
            "code commits are in this list too. This tool classifies none of them."
        )
        return 0

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
        try:
            record = json.loads(pathlib.Path(args.check).read_text(encoding="utf-8"))
            print(claim_summary(record.get("verdicts", {})))
            if not demands_verified_claims(record):
                print(
                    "This record predates the evidence standard and was NOT held to it: "
                    f"it may carry bare 'applied' verdicts (wave {VERIFY_APPLIED_FROM_WAVE}+ "
                    "records, and any record setting 'verify_applied', are held to it)."
                )
        except (OSError, json.JSONDecodeError, AttributeError):
            pass
        print(
            "A verified claim says the close's own string is at the close's own named "
            "location. Whether it is the RIGHT string stays a person's judgment."
        )
        return 0

    if cards is None:
        raise SystemExit("supply --cards, --since or --range")

    if args.residuals:
        found = untracked_residuals(root, cards)
        if args.json:
            print(
                json.dumps(
                    [
                        {
                            "name": entry.name,
                            "sources": list(entry.sources),
                            "mentioned_in": list(entry.mentions),
                        }
                        for entry in found
                    ],
                    indent=2,
                )
            )
            return 0
        for entry in found:
            print(f"\n  {entry.name}")
            for source in entry.sources:
                print(f"      written in  {source}")
            if entry.mentions:
                print(f"      mentioned in {', '.join(entry.mentions)} - a RULE, not a tracker")
        print(f"\n{len(found)} residual-shaped names the tracker ({TRACKERS[0]}) does not carry.")
        print(
            "Scanned: the named card reports, docs/development/increments/, and the "
            "standing-constraints text of " + ", ".join(rule_documents(root)) + "."
        )
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
