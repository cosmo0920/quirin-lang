#!/usr/bin/env python3
"""Backfill the `contracts` section into the archived corpus diff documents.

Card `GEN-CTREASONS`, follow-on `DIFF-TOOL-CONTRACTS-BACKFILL`. Card HYGIENE-16
added the fifth artifact — `contracts.json` — to
`docs/reports/corpus-repin/diff_artifacts.py`, which until then never opened that
file at all. Every consolidation diff document written before that landed
therefore describes four artifacts, and the contract-inventory movements those
waves made were recorded only in prose: the wave-14 consolidation verified
corpus 61 -> 64 by direct comparison past the tool, and wave 15 repeated that by
hand for 64 -> 67.

This tool regenerates *only* that section, from each document's **own committed
artifact pair**, and splices it in. Three rules make it a backfill rather than a
rewrite of archived evidence:

  * **Self-describing inputs.** The before-side is read from the document's own
    `archived` field and the after-side is the artifact set committed beside it;
    no pair is supplied on the command line.
  * **The document is its own oracle.** The four entries it already carries must
    reproduce byte-identically from that pair with the current tool, or this
    tool refuses and writes nothing. That is what makes the pair provably the
    inputs of the recorded run rather than a plausible substitute.
  * **Append-only, and said so.** Every pre-existing key keeps its exact value —
    checked by removing the two added paths and comparing the serialized text to
    the original byte for byte — and a `backfilled_sections` record names the
    tool, its section schema, the date, the inputs and the original document's
    digest. Nothing is silently rewritten and `schema_version` is left alone: a
    reader holding a schema-1 document with a `contracts` section can see from
    that record exactly where it came from.

Idempotent: re-running reproduces the same bytes, and a document that already
carries the section (the wave-16 consolidation wrote its diff with the current
tool) is reported and left untouched.

    python3 -B docs/reports/wave17-gen-ctreasons/backfill_contracts.py [--check]

`--check` writes nothing and exits nonzero if any document is not exactly what
this tool would produce.
"""
from __future__ import annotations

import gzip
import hashlib
import json
import pathlib
import subprocess
import sys
import tempfile

ROOT = pathlib.Path(__file__).resolve().parents[3]
TOOL = ROOT / "docs/reports/corpus-repin/diff_artifacts.py"
CARD = "GEN-CTREASONS"
REPORT = "docs/reports/wave17-gen-ctreasons/README.md"
FOLLOW_ON = "DIFF-TOOL-CONTRACTS-BACKFILL"
#: The date this backfill was produced. A constant, not `today`, so re-running
#: the tool reproduces the committed bytes.
DATE = "2026-09-10"
#: The commit the original documents are recoverable from, for a reader who
#: wants the pre-backfill bytes rather than this tool's word for them.
ORIGINAL_COMMIT = "8ec8402"

SECTION = "contracts"
PROVENANCE_KEY = "backfilled_sections"
SECTION_PATH = "artifacts.contracts"

#: The four entries every one of these documents already carries. They are the
#: oracle: reproducing them from the same inputs is what proves the pair.
PRE_EXISTING = ["chunk-contracts", "diagnostics", "source", "source-effects"]

#: The consolidation chain, newest last. Each entry is only the diff document:
#: its before-side comes out of the document itself and its after-side is the
#: artifact set committed in the same report directory.
DOCUMENTS = [
    "docs/reports/wave9-corpus/diff-e01f1-to-wave9.json.gz",
    "docs/reports/wave10-corpus/diff-wave9-to-wave10.json.gz",
    "docs/reports/wave11-corpus/diff-wave10-to-wave11.json.gz",
    "docs/reports/wave12-corpus/diff-wave11-to-wave12.json.gz",
    "docs/reports/wave13-corpus/diff-wave12-to-wave13.json.gz",
    "docs/reports/wave14-corpus/diff-wave13-to-wave14.json.gz",
    "docs/reports/wave15-corpus/diff-wave14-to-wave15.json.gz",
    # Written with the current tool by the wave-16 consolidation; carried here so
    # the chain is complete and the "already itemized" case is exercised.
    "docs/reports/wave16-corpus/diff-wave15-to-wave16.json.gz",
]


def serialize(document) -> str:
    """The byte form `diff_artifacts.py` itself writes."""
    return json.dumps(document, indent=2, sort_keys=True) + "\n"


def digest(text: str) -> str:
    return hashlib.sha256(text.encode("utf-8")).hexdigest()


def read_document(path: pathlib.Path):
    return json.loads(gzip.decompress(path.read_bytes()).decode("utf-8"))


def unpack(archive: pathlib.Path, target: pathlib.Path):
    """Decompress an archived artifact set under the file names a run produces."""
    manifest = json.loads((archive / "manifest.json").read_text())
    for stem, entry in manifest.items():
        blob = gzip.decompress((archive / f"{stem}.json.gz").read_bytes())
        (target / entry["produced_file"]).write_bytes(blob)


def regenerate(before: pathlib.Path, after: pathlib.Path):
    """Run the current diff tool over two archived artifact sets."""
    with tempfile.TemporaryDirectory(prefix="qcore-backfill-") as scratch:
        scratch = pathlib.Path(scratch)
        unpack(after, scratch)
        output = scratch / "diff.json"
        finished = subprocess.run(
            [sys.executable, "-B", str(TOOL), "--archived", str(before),
             "--measured", str(scratch), "--output", str(output)],
            capture_output=True, text=True,
        )
        if finished.returncode != 0:
            raise SystemExit("%s failed on %s -> %s: %s" % (TOOL, before, after, finished.stderr))
        return json.loads(output.read_text())


def without_backfill(document):
    """The document as it was: the paths this tool added removed.

    Only a section this tool's own record claims is removed — a document whose
    `contracts` entry was written by the tool that produced it (the wave-16
    consolidation) is returned unchanged, so it is recognised as already
    itemized rather than treated as needing a backfill.
    """
    stripped = json.loads(json.dumps(document))
    record = stripped.pop(PROVENANCE_KEY, None)
    if record and SECTION_PATH in record:
        stripped.get("artifacts", {}).pop(SECTION, None)
    return stripped


def backfill(path: pathlib.Path, original):
    """The backfilled document, or `None` if the section is already there."""
    if SECTION in original.get("artifacts", {}):
        return None
    before = ROOT / original["archived"]
    after = path.parent / "artifacts"
    if not (before / "manifest.json").is_file():
        raise SystemExit("%s: %s carries no artifact manifest" % (path, before))
    produced = regenerate(before, after)
    # The document is its own oracle: what it already says must come back out of
    # the same two archived sets, or these are not the inputs of that run.
    for stem in PRE_EXISTING:
        if json.dumps(produced["artifacts"][stem], sort_keys=True) != json.dumps(
            original["artifacts"][stem], sort_keys=True
        ):
            raise SystemExit(
                "%s: the archived `%s` entry does not reproduce from %s and %s; refusing to "
                "backfill a document whose recorded inputs cannot be confirmed"
                % (path, stem, before, after)
            )
    if set(produced["artifacts"]) != set(PRE_EXISTING) | {SECTION}:
        raise SystemExit(
            "%s: the current tool produced artifacts %s, expected the four pre-existing "
            "entries plus %r" % (path, sorted(produced["artifacts"]), SECTION)
        )
    document = json.loads(json.dumps(original))
    document["artifacts"][SECTION] = produced["artifacts"][SECTION]
    document[PROVENANCE_KEY] = {
        SECTION_PATH: {
            "card": CARD,
            "follow_on": FOLLOW_ON,
            "report": REPORT,
            "date": DATE,
            "tool": "docs/reports/corpus-repin/diff_artifacts.py",
            "tool_section_schema": 2,
            "document_schema_version_unchanged": original["schema_version"],
            "inputs": {"archived": original["archived"], "measured": str(
                after.relative_to(ROOT))},
            "original_commit": ORIGINAL_COMMIT,
            "original_sha256": digest(serialize(original)),
            "verification": (
                "the four artifact entries this document already carried reproduce "
                "byte-identically from the two archived artifact sets named in `inputs` "
                "under the current tool, so the pair is the recorded run's own input; only "
                "this record and artifacts.%s are new, and every pre-existing key keeps its "
                "exact value" % SECTION
            ),
            "note": (
                "the original document was written before card HYGIENE-16 taught "
                "diff_artifacts.py to read contracts.json at all, so its contract-inventory "
                "movement was recorded only in that wave's prose. This section is a "
                "regeneration from committed bytes, not a re-measurement of the corpus: no "
                "audit was re-run and no verdict, count or pin is affected."
            ),
        }
    }
    # Append-only, checked rather than asserted: stripping the two added paths
    # must give back the original document's exact bytes.
    if serialize(without_backfill(document)) != serialize(original):
        raise SystemExit("%s: the backfill is not append-only" % path)
    return document


def main(argv):
    check = "--check" in argv[1:]
    problems, written, skipped = [], [], []
    for relative in DOCUMENTS:
        path = ROOT / relative
        committed = read_document(path)
        original = without_backfill(committed)
        produced = backfill(path, original)
        if produced is None:
            # Either the document was written with the current tool (wave 16) or
            # a previous run of this tool already backfilled it; both are decided
            # by comparing what this tool would write.
            if PROVENANCE_KEY in committed:
                problems.append("%s: carries a backfill record but no %s section"
                                % (relative, SECTION))
            else:
                skipped.append("%s: already itemized by the tool that wrote it" % relative)
            continue
        text = serialize(produced)
        if serialize(committed) == text:
            skipped.append("%s: already backfilled, byte-identical" % relative)
            continue
        if PROVENANCE_KEY in committed:
            problems.append("%s: backfilled section is not what this tool produces" % relative)
            continue
        if check:
            problems.append("%s: carries no %s section" % (relative, SECTION))
            continue
        path.write_bytes(gzip.compress(text.encode("utf-8"), mtime=0))
        written.append(relative)
    for line in skipped:
        print("skip:", line)
    for line in written:
        print("backfilled:", line)
    for line in problems:
        print("PROBLEM:", line)
    return 1 if problems else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
