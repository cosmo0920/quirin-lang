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

"""Validate and apply a card's `integration-amendments.json` (pluggable-cards
seam D, card AMENDMENTS-SCHEMA).

This script is the single authority for the amendments schema
`quirin-amendments/1`; `docs/agents/operations.md` routes to it. A document is
one JSON object with exactly these top-level keys:

- `schema` (required): the string `quirin-amendments/1`.
- `card` (required): the card id, upper case (`[A-Z0-9][A-Z0-9-]*`).
- `base`, `measured_tip` (required): git revisions, 7 to 40 hex digits.
- `increment_paragraph` (required): one paragraph (non-empty, no blank line,
  no surrounding whitespace). It is prepended, newest first, to the current
  increment record named by `docs/development/increments/README.md`, directly
  below the record's checkpoint notice (the `> ` block).
- `status_paragraph` (required, may be `null`): one paragraph, or `null` when
  no current-status fact changes. When present it is prepended below the
  title of `docs/STATUS.md`.
- `ledger` (required, may be empty): edits of the acceptance matrix's
  `named_followups` (`docs/reviews/r0_3/acceptance_matrix.json`), applied in
  order. Each entry has `id`, `operation` and exactly the fields its operation
  takes:
    - `register`: `id`, `title`, `text`, `source`, and optionally
      `recorded_by` (a card id). Adds `{"id", "title", "why": text, "source"}`,
      plus `recorded_by` when given, at the end; refused when the id is
      already registered. A new id is `[A-Z0-9][A-Z0-9_.-]*`.
    - `amend`, `append`: `id`, `text`, `source`. Appends `" " + text` to the
      entry's `why`, and `", " + source` to its `source` unless that path is
      already listed; refused when the id is absent or the text is already
      present. The two operations write the same bytes: `amend` says the text
      qualifies the entry's claim, `append` that it adds an observation.
    - `retire`: `id`, `why`, `source`. Removes the entry; refused when the id
      is absent. `why` is the DISCHARGE record and stays in the card's report
      and amendments file; it is not written to the matrix.
- `profiles` (optional): a list of distinct evaluation-profile names
  (`[a-z0-9][a-z0-9-]*`), the live evaluation profiles whose baseline the
  card's measured tip records (card EVALUATION-PROFILES defines the profile
  records under `contract/evaluations/`; `scripts/evaluation_profiles.py` owns
  their schema and this check). Absent means the empty list. Whenever the tree
  holds `contract/evaluations/`, every run (`--check`, `--dry-run` and a write)
  calls `evaluation_profiles.check_amendments` with the document's `base`, or
  `--profiles-base REV` when given: every name is a profile, none is frozen,
  and the list equals the set of live profiles whose baseline changed between
  that revision and the working tree, so a card that moves a baseline without
  listing it, or lists one it did not move, is refused. Run it on the card's
  tree; on a merge with a newer main pass `--profiles-base` the main the card
  was merged into, so other cards' baselines are not attributed to this one. A
  revision absent from the object store, or a legacy document with no `base`
  and no `--profiles-base`, is refused. In a tree without
  `contract/evaluations/` there is no profile, so a non-empty list is refused
  and an empty one passes. The field writes nothing: the baselines are in the
  card's own commits.
- `contribution` (optional): `null`, or `{"ledgers": [path, ...]}`, a
  non-empty list of distinct repository paths of the per-rule contribution
  ledgers the card commits (card CONTRIBUTION-LEDGERS;
  `scripts/contribution_ledgers.py` owns the ledger schema, its layout
  `contract/contributions/<rule-id>/<profile>.json` and
  `contract/contributions/interactions/<key>/<profile>.json`, and this
  check). Every run calls `contribution_ledgers.check_contribution`: each
  ledger exists, is valid, matches the digest of the native-comparison record
  it was written from and is an entry of that record's contribution slot;
  each rule id it names is in `crates/gateway-rules/registry.toml` once that
  registry exists, and until then has a rule document
  `docs/agents/rules/<id>.md` with a router row; its profile is live; every
  other ledger of the same measurement and profile is listed too, and the
  listed group adds up (base plus deltas equals the TIP totals); and the tree passes
  `contribution_ledgers.consistency` (every current group re-derives its
  profile's real-run total; a disagreement is refused, never repaired). A ledger
  measured on an older profile baseline than the tree's is noted, not
  refused: the profile consistency check lists it as stale until it is
  re-measured. Applying the field regenerates the contributions page
  `docs/development/contributions.md` (`contribution_ledgers.py render`) when
  it differs; the ledgers themselves are in the card's own commits. (Before
  this card the field was reserved as any object; no committed document used
  it, so no existing valid document changes meaning.)

Every refusal is fail-closed: an unknown top-level key, a missing or mistyped
field, an unknown operation, a field an operation does not take, registering
an existing id, or amending, appending to or retiring an absent id stops the
run with a message and exit 1 before anything is written. The text edits are
computed in memory and written only after every check passes.

Usage:

    python3 scripts/apply_amendments.py <amendments.json> [--check] [--dry-run]
                                        [--legacy] [--root DIR]
                                        [--profiles-base REV]

`--check` validates the document and its applicability to the tree and writes
nothing; `--dry-run` also prints the diffs it would write. `--legacy` reads a
pre-schema file (rules 32-35 and earlier cards shipped several shapes) through
`normalise_legacy`, prints what it derived, then validates and applies the
normalised document; historical files are never rewritten (each report owns its
evidence). The script never regenerates the matrix: after a write it prints the
post-gate landing sequence, whose authority is `scripts/land.py`.
"""
from __future__ import annotations

import argparse
import difflib
import json
import os
import re
import sys
from pathlib import Path

SCHEMA = "quirin-amendments/1"
ROOT = Path(__file__).resolve().parents[1]
MATRIX = Path("docs/reviews/r0_3/acceptance_matrix.json")
STATUS = Path("docs/STATUS.md")
INCREMENTS = Path("docs/development/increments")
INCREMENT_INDEX = INCREMENTS / "README.md"

TOP_LEVEL_REQUIRED = ("schema", "card", "base", "measured_tip",
                      "increment_paragraph", "status_paragraph", "ledger")
TOP_LEVEL_OPTIONAL = ("profiles", "contribution")
OPTIONAL_FIELDS = {"register": ("recorded_by",)}
OPERATION_FIELDS = {
    "register": ("id", "operation", "title", "text", "source"),
    "amend": ("id", "operation", "text", "source"),
    "append": ("id", "operation", "text", "source"),
    "retire": ("id", "operation", "why", "source"),
}
CARD_RE = re.compile(r"[A-Z0-9][A-Z0-9-]*")
REVISION_RE = re.compile(r"[0-9a-f]{7,40}")
NEW_ID_RE = re.compile(r"[A-Z0-9][A-Z0-9_.-]*")
# Registered ids include one historical lower-case id (`wave3-CT`).
EXISTING_ID_RE = re.compile(r"[A-Za-z0-9][A-Za-z0-9_.+-]*")
PROFILE_RE = re.compile(r"[a-z0-9][a-z0-9-]*")
MIGRATED_START = "<!-- migrated-section:start -->"
EVALUATIONS = Path("contract/evaluations")
CONTRIBUTIONS_PREFIX = "contract/contributions/"
SCRIPTS = Path(__file__).resolve().parent


class AmendmentError(Exception):
    """A refusal. Nothing has been written when it is raised."""


# --------------------------------------------------------------------------
# Schema validation


def _paragraph(value, where):
    if not isinstance(value, str) or not value.strip():
        raise AmendmentError(f"{where} must be a non-empty string")
    if value != value.strip():
        raise AmendmentError(f"{where} has leading or trailing whitespace")
    if "\n\n" in value or "\r" in value:
        raise AmendmentError(f"{where} must be one paragraph (no blank line)")


def _text(value, where):
    if not isinstance(value, str) or not value.strip():
        raise AmendmentError(f"{where} must be a non-empty string")
    if value != value.strip():
        raise AmendmentError(f"{where} has leading or trailing whitespace")


def validate(document, *, legacy=False):
    """Check `document` against `quirin-amendments/1`; raise on any defect.

    `legacy=True` is used only for documents produced by `normalise_legacy`:
    it accepts `null` for `base` and `measured_tip` (several pre-schema files
    never recorded them) and for an amend/append `source` (the integrator then
    left the entry's `source` untouched, as it did by hand). Every other check
    is the same.
    """
    if not isinstance(document, dict):
        raise AmendmentError("the document must be a JSON object")
    unknown = sorted(set(document) - set(TOP_LEVEL_REQUIRED) - set(TOP_LEVEL_OPTIONAL))
    if unknown:
        raise AmendmentError(
            f"unknown top-level key(s) {unknown}; {SCHEMA} takes "
            f"{list(TOP_LEVEL_REQUIRED) + list(TOP_LEVEL_OPTIONAL)} "
            "(a pre-schema file is read with --legacy)")
    missing = [key for key in TOP_LEVEL_REQUIRED if key not in document]
    if missing:
        raise AmendmentError(f"missing top-level key(s) {missing}")
    if document["schema"] != SCHEMA:
        raise AmendmentError(f"schema is {document['schema']!r}; this tool reads {SCHEMA!r}")
    card = document["card"]
    if not isinstance(card, str) or not CARD_RE.fullmatch(card):
        raise AmendmentError(f"card {card!r} must match {CARD_RE.pattern}")
    for key in ("base", "measured_tip"):
        value = document[key]
        if value is None and legacy:
            continue
        if not isinstance(value, str) or not REVISION_RE.fullmatch(value):
            raise AmendmentError(f"{key} {value!r} must be a git revision of 7-40 hex digits")
    _paragraph(document["increment_paragraph"], "increment_paragraph")
    if document["status_paragraph"] is not None:
        _paragraph(document["status_paragraph"], "status_paragraph")
    ledger = document["ledger"]
    if not isinstance(ledger, list):
        raise AmendmentError("ledger must be a list")
    for index, entry in enumerate(ledger):
        where = f"ledger[{index}]"
        if not isinstance(entry, dict):
            raise AmendmentError(f"{where} must be an object")
        operation = entry.get("operation")
        if operation not in OPERATION_FIELDS:
            raise AmendmentError(
                f"{where} has operation {operation!r}; expected one of {sorted(OPERATION_FIELDS)}")
        fields = OPERATION_FIELDS[operation]
        missing = [field for field in fields if field not in entry]
        extra = sorted(set(entry) - set(fields) - set(OPTIONAL_FIELDS.get(operation, ())))
        if missing:
            raise AmendmentError(f"{where} ({operation}) lacks {missing}")
        if extra:
            raise AmendmentError(f"{where} ({operation}) takes only {list(fields)}; unexpected {extra}")
        identifier = entry["id"]
        pattern = NEW_ID_RE if operation == "register" else EXISTING_ID_RE
        if not isinstance(identifier, str) or not pattern.fullmatch(identifier):
            raise AmendmentError(f"{where} id {identifier!r} must match {pattern.pattern}")
        for field in fields:
            if field in ("id", "operation"):
                continue
            if (legacy and field == "source" and operation in ("amend", "append")
                    and entry[field] is None):
                continue
            _text(entry[field], f"{where}.{field}")
        if "recorded_by" in entry and (not isinstance(entry["recorded_by"], str)
                                       or not CARD_RE.fullmatch(entry["recorded_by"])):
            raise AmendmentError(f"{where}.recorded_by must be a card id")
    if "profiles" in document:
        profiles = document["profiles"]
        if not isinstance(profiles, list):
            raise AmendmentError("profiles must be a list of profile names")
        for name in profiles:
            if not isinstance(name, str) or not PROFILE_RE.fullmatch(name):
                raise AmendmentError(f"profile name {name!r} must match {PROFILE_RE.pattern}")
        if len(set(profiles)) != len(profiles):
            raise AmendmentError("profiles lists a name twice")
    if "contribution" in document:
        contribution = document["contribution"]
        if contribution is not None:
            if not isinstance(contribution, dict) or set(contribution) != {"ledgers"}:
                raise AmendmentError(
                    'contribution must be null or {"ledgers": [path, ...]}')
            paths = contribution["ledgers"]
            if (not isinstance(paths, list) or not paths
                    or not all(isinstance(path, str) and path.startswith(CONTRIBUTIONS_PREFIX)
                               and path.endswith(".json") for path in paths)
                    or len(set(paths)) != len(paths)):
                raise AmendmentError(
                    f"contribution.ledgers must be a non-empty list of distinct ledger paths "
                    f"under {CONTRIBUTIONS_PREFIX}")


# --------------------------------------------------------------------------
# Application


def current_increment_record(root, *, must_exist=True):
    """The record file `docs/development/increments/README.md` names."""
    index = (root / INCREMENT_INDEX).read_text(encoding="utf-8")
    found = re.findall(r"^Current record file: \[([^\]]+)\]\(([^)]+)\)", index, re.M)
    if len(found) != 1 or found[0][0] != found[0][1]:
        raise AmendmentError(
            f"{INCREMENT_INDEX} must name exactly one 'Current record file: [X](X)'; found {found}")
    name = found[0][1]
    if "/" in name or not name.endswith(".md"):
        raise AmendmentError(f"current record {name!r} is not a file in {INCREMENTS}")
    path = INCREMENTS / name
    if must_exist and not (root / path).is_file():
        raise AmendmentError(f"current record {path} does not exist")
    return path


def prepend_increment(text, paragraph, path):
    """Insert `paragraph` below the checkpoint notice, above every paragraph."""
    if paragraph in text:
        raise AmendmentError(f"{path} already contains the increment paragraph")
    lines = text.split("\n")
    notice = next((i for i, line in enumerate(lines) if line.startswith("> ")), None)
    if notice is None:
        raise AmendmentError(f"{path} has no checkpoint notice ('> ' line) to insert below")
    end = notice
    while end + 1 < len(lines) and lines[end + 1].startswith(">"):
        end += 1
    if any(line.strip() == MIGRATED_START for line in lines[: end + 1]):
        raise AmendmentError(f"{path} has a migrated-section block above its checkpoint notice")
    head = "\n".join(lines[: end + 1])
    rest = "\n".join(lines[end + 1:]).lstrip("\n")
    if rest:
        return f"{head}\n\n{paragraph}\n\n{rest}"
    return f"{head}\n\n{paragraph}\n"


def prepend_status(text, paragraph):
    if paragraph in text:
        raise AmendmentError(f"{STATUS} already contains the status paragraph")
    first, sep, rest = text.partition("\n")
    if not first.startswith("# ") or not sep:
        raise AmendmentError(f"{STATUS} must begin with its '# ' title line")
    return f"{first}\n\n{paragraph}\n\n{rest.lstrip(chr(10))}"


def apply_ledger(followups, ledger):
    """Apply `ledger` to a copy of `named_followups`; return (result, log)."""
    result = [dict(entry) for entry in followups]
    log = []
    for index, edit in enumerate(ledger):
        where = f"ledger[{index}] {edit['operation']} {edit['id']}"
        position = next((i for i, entry in enumerate(result) if entry.get("id") == edit["id"]), None)
        operation = edit["operation"]
        if operation == "register":
            if position is not None:
                raise AmendmentError(f"{where}: the id is already registered")
            entry = {"id": edit["id"], "title": edit["title"],
                     "why": edit["text"], "source": edit["source"]}
            if "recorded_by" in edit:
                entry["recorded_by"] = edit["recorded_by"]
            result.append(entry)
            log.append(f"registered {edit['id']}")
            continue
        if position is None:
            raise AmendmentError(f"{where}: no registered follow-up has this id")
        if operation == "retire":
            del result[position]
            log.append(f"retired {edit['id']}")
            continue
        entry = result[position]
        why = entry.get("why")
        if not isinstance(why, str):
            raise AmendmentError(f"{where}: the registered entry has no 'why' text to extend")
        if edit["text"] in why:
            raise AmendmentError(f"{where}: the entry's 'why' already contains this text")
        entry["why"] = f"{why} {edit['text']}"
        source = entry.get("source")
        if edit["source"] is None:  # legacy only: the source is left as it is
            pass
        elif source is None:
            entry["source"] = edit["source"]
        elif edit["source"] not in [part.strip() for part in source.split(",")]:
            entry["source"] = f"{source}, {edit['source']}"
        log.append(f"{operation}ed {edit['id']}")
    return result, log


def _evaluation_profiles():
    # Imported here so `python3 -I` (no script directory on sys.path) works.
    if str(SCRIPTS) not in sys.path:
        sys.path.insert(0, str(SCRIPTS))
    import evaluation_profiles
    return evaluation_profiles


def check_profiles(document, root, profiles_base=None):
    """Check `profiles[]` (module doc); return the checked names, or None
    when the tree holds no evaluation profiles and the list is empty."""
    listed = document.get("profiles", [])
    if not (root / EVALUATIONS).is_dir():
        if listed:
            raise AmendmentError(f"profiles lists {listed}, but {EVALUATIONS} does not exist "
                                 f"under {root}: there is no evaluation profile to name")
        return None
    against = profiles_base if profiles_base is not None else document["base"]
    if against is None:
        raise AmendmentError("the document records no base, which the profiles[] check needs; "
                             "pass --profiles-base REV")
    evaluation_profiles = _evaluation_profiles()
    try:
        return evaluation_profiles.check_amendments(root, document, against)
    except evaluation_profiles.ProfileError as error:
        raise AmendmentError(f"profiles[]: {error}") from error


def _contribution_ledgers():
    if str(SCRIPTS) not in sys.path:
        sys.path.insert(0, str(SCRIPTS))
    import contribution_ledgers
    return contribution_ledgers


def check_contribution(document, root):
    """Check `contribution` (module doc); return (log lines, page edit or
    None)."""
    if document.get("contribution") is None:
        return [], None
    ledgers = _contribution_ledgers()
    try:
        notes = ledgers.check_contribution(root, document["contribution"])
        _, inconsistent = ledgers.consistency(root)
        if inconsistent:
            raise AmendmentError("the profile consistency check fails (reported, never "
                                 "repaired): " + "; ".join(inconsistent))
        page = ledgers.render(root)
    except (ledgers.LedgerError, OSError, ValueError, KeyError, TypeError) as error:
        raise AmendmentError(f"contribution: {error}") from error
    path = Path(str(ledgers.PAGE))
    old = (root / path).read_text(encoding="utf-8") if (root / path).is_file() else ""
    return [f"contribution: {note}" for note in notes], (
        None if old == page else (path, old, page))


def plan(document, root, profiles_base=None):
    """Every file edit `document` makes, as {relative path: (old, new)}, and
    the log lines describing them."""
    profiles = check_profiles(document, root, profiles_base)
    contribution_log, page = check_contribution(document, root)
    edits = {}
    record = current_increment_record(root)
    old = (root / record).read_text(encoding="utf-8")
    edits[record] = (old, prepend_increment(old, document["increment_paragraph"], record))
    if document["status_paragraph"] is not None:
        old = (root / STATUS).read_text(encoding="utf-8")
        edits[STATUS] = (old, prepend_status(old, document["status_paragraph"]))
    log = [] if profiles is None else [f"profiles: {profiles} checked against "
                                        f"{profiles_base or document['base']}"]
    if document["ledger"]:
        old = (root / MATRIX).read_text(encoding="utf-8")
        matrix = json.loads(old)
        followups = matrix.get("named_followups")
        if not isinstance(followups, list):
            raise AmendmentError(f"{MATRIX} has no named_followups list")
        matrix["named_followups"], ledger_log = apply_ledger(followups, document["ledger"])
        log += ledger_log
        edits[MATRIX] = (old, json.dumps(matrix, indent=1, ensure_ascii=False) + "\n")
    log += contribution_log
    if page is not None:
        edits[page[0]] = (page[1], page[2])
    return edits, log


def write(edits, root):
    for path, (_, new) in edits.items():
        target = root / path
        target.parent.mkdir(parents=True, exist_ok=True)
        temporary = target.with_name(target.name + ".apply-amendments.tmp")
        temporary.write_text(new, encoding="utf-8")
        os.replace(temporary, target)


# --------------------------------------------------------------------------
# Legacy normalisation (pre-schema files; read only, never rewritten)

LEGACY_OPERATIONS = {"register": "register", "add": "register", "amend": "amend",
                     "append": "append", "retire": "retire"}


def _legacy_operation(raw, where):
    if not isinstance(raw, str):
        raise AmendmentError(f"{where}: no operation")
    word = raw.split()[0].lower() if raw.split() else ""
    if word not in LEGACY_OPERATIONS:
        raise AmendmentError(f"{where}: unknown legacy operation {raw!r}")
    return LEGACY_OPERATIONS[word]


def _legacy_entry(raw, where, default_source, notes):
    if not isinstance(raw, dict) or not isinstance(raw.get("id"), str):
        raise AmendmentError(f"{where}: a ledger entry must be an object with an id")
    operation = _legacy_operation(
        raw.get("operation", raw.get("action", raw.get("mode"))), where)
    text = raw.get("text", raw.get("why"))
    if operation == "retire":
        text = raw.get("why", raw.get("text"))
    if not isinstance(text, str) or not text.strip():
        raise AmendmentError(f"{where}: no text")
    if text != text.strip():
        notes.append(f"{where}: stripped surrounding whitespace from the text")
    entry = {"id": raw["id"], "operation": operation}
    if operation == "register":
        if not isinstance(raw.get("title"), str):
            raise AmendmentError(f"{where}: a registration needs a title")
        entry["title"] = raw["title"]
    entry["why" if operation == "retire" else "text"] = text.strip()
    source = raw.get("source")
    if not isinstance(source, str) or not source.strip():
        if operation in ("amend", "append"):
            source = None
            notes.append(f"{where}: no source; the entry's source is left unchanged")
        else:
            source = default_source
            notes.append(f"{where}: no source; used the report README {source}")
    entry["source"] = source
    if operation == "register" and isinstance(raw.get("recorded_by"), str):
        entry["recorded_by"] = raw["recorded_by"]
    kept = {"id", "operation", "action", "mode", "text", "why", "source"}
    if operation == "register":
        kept |= {"title", "recorded_by"}
    dropped = sorted(set(raw) - kept)
    if dropped:
        notes.append(f"{where}: dropped field(s) {dropped} the schema does not carry")
    return entry


def normalise_legacy(raw, path):
    """Return (document, notes) for a pre-schema file at `path`.

    Each shape is read by an explicit adapter chosen by its keys, never by
    guessing: an unrecognised shape, or a file that proposes no applicable
    text (no increment paragraph), is refused. Derived values (a card id from
    the report directory, a source from the report README, unrecorded
    revisions as null) are listed in `notes`.
    """
    if not isinstance(raw, dict):
        raise AmendmentError(f"{path}: not a JSON object")
    if "schema" in raw:
        raise AmendmentError(f"{path}: carries a schema key; validate it without --legacy")
    report = Path(path).parent.name
    default_source = f"docs/reports/{report}/README.md"
    notes = []
    keys = set(raw)

    def revision(*names):
        for name in names:
            value = raw.get(name)
            if isinstance(value, str) and REVISION_RE.fullmatch(value):
                if name not in ("base", "measured_tip"):
                    notes.append(f"measured_tip taken from {name!r}")
                return value
        return None

    card = raw.get("card")
    if not isinstance(card, str):
        card = report.upper()
        notes.append(f"no card key; card derived from the report directory: {card}")
    base = revision("base")
    if base is None:
        notes.append("base not recorded (null)")
    measured = revision("measured_tip", "measured_source", "measured_joint_source")
    if measured is None:
        notes.append("measured tip not recorded (null)")

    ledger_raw = []
    increment = status = None
    if "tracking" in keys and raw.get("increment_paragraph") is None:
        raise AmendmentError(
            f"{path}: proposal-only legacy file (increment_paragraph is null and 'tracking' holds "
            "conditional instructions, not text to apply); nothing is applicable")
    if "ledger" in keys:  # the rule 33-35 / refactoring-round shape
        increment = raw.get("increment_paragraph")
        status = raw.get("status_paragraph")
        ledger_raw = [(f"ledger[{i}]", e) for i, e in enumerate(raw["ledger"])]
        if isinstance(raw.get("increment"), str):
            notes.append(f"increment target {raw['increment']!r} ignored; the current record is used")
    elif "followup_amendments" in keys:  # logger-effect / rule 32
        increment = raw.get("increment_paragraph", raw.get("increment"))
        status = raw.get("status_paragraph", raw.get("status_append"))
        ledger_raw = [(f"followup_amendments[{i}]", e)
                      for i, e in enumerate(raw["followup_amendments"])]
        if raw.get("retired_followups"):
            raise AmendmentError(f"{path}: non-empty retired_followups is not a recognised shape")
    elif "named_followup_amendments" in keys:  # bottom-up batch 4
        increment = (raw.get("increment") or {}).get("paragraph")
        status = (raw.get("status") or {}).get("text")
        ledger_raw = [(f"named_followup_amendments[{i}]", e)
                      for i, e in enumerate(raw["named_followup_amendments"])]
    elif "residual_plan" in keys:  # bottom-up batch 5
        increment = raw.get("increment_paragraph")
        status = raw.get("status_paragraph")
        plan_ = raw["residual_plan"]
        ledger_raw = [("residual_plan", {"id": plan_.get("retain"), "operation": "append",
                                         "text": plan_.get("append_paragraph")})]
        notes.append("residual_plan.retain/append_paragraph read as one append")
    elif "followup_amendment" in keys:  # guarded/premise/scheduler contract cards
        increment = raw.get("increment")
        if isinstance(increment, dict):
            increment = increment.get("text")
        status = raw.get("status")
        if "status_amendment" in keys:
            status = (raw.get("status_amendment") or {}).get("text")
        amendment = dict(raw["followup_amendment"])
        if "append" in amendment and "text" not in amendment:
            amendment["text"] = amendment.pop("append")
            amendment["operation"] = "append"
        ledger_raw = [("followup_amendment", amendment)]
        if raw.get("additional_retirements"):
            raise AmendmentError(f"{path}: non-empty additional_retirements is not a recognised shape")
    else:
        raise AmendmentError(f"{path}: unrecognised legacy shape (keys {sorted(keys)})")

    if not isinstance(increment, str) or not increment.strip():
        raise AmendmentError(f"{path}: no increment paragraph to apply")
    if isinstance(status, str) and not status.strip():
        status = None
        notes.append("empty status paragraph read as null")
    ledger = [_legacy_entry(entry, where, default_source, notes) for where, entry in ledger_raw]
    document = {
        "schema": SCHEMA,
        "card": card,
        "base": base,
        "measured_tip": measured,
        "increment_paragraph": increment.strip(),
        "status_paragraph": status.strip() if isinstance(status, str) else None,
        "ledger": ledger,
    }
    carried = {"card", "base", "measured_tip", "measured_source", "measured_joint_source",
               "increment_paragraph", "status_paragraph", "ledger", "followup_amendments",
               "named_followup_amendments", "followup_amendment", "residual_plan",
               "increment", "status", "status_append", "status_amendment"}
    informational = sorted(keys - carried)
    if informational:
        notes.append(f"informational key(s) not applied: {informational}")
    return document, notes


def load(path, *, legacy):
    try:
        raw = json.loads(Path(path).read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        raise AmendmentError(f"{path}: cannot read JSON ({error})") from error
    if legacy:
        document, notes = normalise_legacy(raw, path)
        validate(document, legacy=True)
        return document, notes
    validate(raw)
    return raw, []


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("amendments")
    parser.add_argument("--check", action="store_true",
                        help="validate the document and its applicability; write nothing")
    parser.add_argument("--dry-run", action="store_true",
                        help="print the diffs that would be written; write nothing")
    parser.add_argument("--legacy", action="store_true",
                        help="read a pre-schema file through the normaliser")
    parser.add_argument("--root", type=Path, default=ROOT,
                        help="repository root to apply to (default: this checkout)")
    parser.add_argument("--profiles-base", metavar="REV",
                        help="check profiles[] against REV instead of the document's base "
                             "(the main a card was merged into)")
    args = parser.parse_args(argv)
    root = args.root.resolve()
    try:
        document, notes = load(args.amendments, legacy=args.legacy)
        for note in notes:
            print(f"legacy: {note}")
        edits, log = plan(document, root, args.profiles_base)
    except AmendmentError as error:
        print(f"apply_amendments: refused: {error}", file=sys.stderr)
        print("apply_amendments: nothing was written", file=sys.stderr)
        return 1
    for line in log:
        print(line if line.startswith(("profiles: ", "contribution: ")) else f"ledger: {line}")
    if args.dry_run:
        for path, (old, new) in edits.items():
            sys.stdout.writelines(difflib.unified_diff(
                old.splitlines(True), new.splitlines(True), f"a/{path}", f"b/{path}", n=1))
    if args.check or args.dry_run:
        print(f"apply_amendments: {document['card']}: valid; would write "
              f"{', '.join(str(p) for p in edits)}")
        return 0
    write(edits, root)
    print(f"apply_amendments: {document['card']}: wrote {', '.join(str(p) for p in edits)}")
    print("follow-up (not run by this tool; scripts/land.py prints and checks it):")
    for number, step in enumerate(_land().SEQUENCE, 1):
        print(f"  {number}. {step}")
    return 0


def _land():
    if str(SCRIPTS) not in sys.path:
        sys.path.insert(0, str(SCRIPTS))
    import land
    return land


if __name__ == "__main__":
    sys.exit(main())
