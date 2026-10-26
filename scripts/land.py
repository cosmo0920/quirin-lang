#!/usr/bin/env python3
"""The integrator's post-gate landing sequence, and a check that the
regenerated records are fresh before the Python suite reruns (card
RECORDS-TOOLING; the sequence's single authority).

After `scripts/apply_amendments.py` writes a card's records and the landing
gate has run, the integrator:

1. writes the landing gate record `docs/reports/<card>/integration/gate.json`
   (only when the merged measurement differs from the card's own gate, as
   `docs/agents/operations.md` says; otherwise the card's `gate.json`);
2. regenerates the matrix: `python3 docs/reports/remaining-d02/build_matrix.py
   . --gate <gate.json>`;
3. regenerates the report index: `python3 scripts/build_docs_index.py`;
4. regenerates the per-rule contributions page:
   `python3 scripts/contribution_ledgers.py render` (card CONTRIBUTION-LEDGERS;
   the page names each live profile's baseline, so a landing that moves a
   baseline or adds a ledger changes it);
5. runs `python3 scripts/land.py check`, which must exit 0;
6. commits the records;
7. reruns the Python suite;
8. measures the committed landing revision on all live routes and publishes
   its immutable baseline with `python3 scripts/land.py publish`.

The recurring landing failure this guards is the Python suite running before
step 2: a ledger edit changes `len(named_followups)` while
`dimensions.named_followups` still holds the old count, so
`test_build_matrix_sections` fails. `check` predicts that (and a stale
`dimensions.requirements` or report index) without running the suite, and
names the step to run first; likewise a stale contributions page.

Usage:

    python3 scripts/land.py            # print the sequence
    python3 scripts/land.py check [--root DIR]
    python3 scripts/land.py publish [--root DIR] [--previous SHA] [--inputs-base DIR]

`check` exits 0 when the records are fresh, 1 naming each stale record and the
command that regenerates it.
"""
from __future__ import annotations

import argparse
import json
import subprocess
import sys
from datetime import datetime, timezone
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
MATRIX = Path("docs/reviews/r0_3/acceptance_matrix.json")
MATRIX_COMMAND = "python3 docs/reports/remaining-d02/build_matrix.py . --gate <gate.json>"
INDEX_COMMAND = "python3 scripts/build_docs_index.py"
CONTRIBUTIONS_COMMAND = "python3 scripts/contribution_ledgers.py render"
# The dimensions whose name is a section they count (test_build_matrix_sections).
COUNTED_SECTIONS = ("requirements", "named_followups")
SEQUENCE = (
    "write the landing gate record docs/reports/<card>/integration/gate.json "
    "(when the merged measurement differs from the card's gate)",
    MATRIX_COMMAND,
    INDEX_COMMAND,
    CONTRIBUTIONS_COMMAND,
    "python3 scripts/land.py check",
    "git commit the records",
    "python3 -m unittest discover -s scripts/tests",
    "python3 scripts/land.py publish (live routes; after the full gate and records commit)",
)


def git_revision(root, revision):
    result = subprocess.run(["git", "rev-parse", "--verify", f"{revision}^{{commit}}"],
                            cwd=root, capture_output=True, text=True, check=True)
    return result.stdout.strip()


def previous_published(root, sha):
    ancestors = subprocess.run(["git", "rev-list", "--first-parent", f"{sha}^"],
                               cwd=root, capture_output=True, text=True, check=True)
    for ancestor in ancestors.stdout.splitlines():
        if (root / "private/baselines" / ancestor).is_dir():
            return ancestor
    return None


def publish(root, previous=None, inputs_base=Path("/media/Data/GitHub/quirin-lang"),
            bootstrap=False):
    """Measure a committed landing and publish its verified, immutable routes."""
    sha = git_revision(root, "HEAD")
    published_prior = previous_published(root, sha) if previous is None else None
    prior = git_revision(root, previous or published_prior or "HEAD^1")
    if prior == sha:
        raise ValueError("previous revision equals landing revision")
    ancestor = subprocess.run(["git", "merge-base", "--is-ancestor", prior, sha],
                              cwd=root, check=False)
    if ancestor.returncode != 0:
        raise ValueError(f"previous revision {prior} is not an ancestor of {sha}")
    status = subprocess.run(["git", "status", "--porcelain"], cwd=root,
                            capture_output=True, text=True, check=True)
    if status.stdout.strip():
        raise ValueError("landing tree must be clean before publication")
    executable = root / "target/release/quirin-audit"
    plugin = root / "target/release/quirin-audit-measure"
    if not executable.is_file() or not plugin.is_file():
        raise ValueError("release audit host and measure plugin must be built first")
    stamp = datetime.now(timezone.utc).strftime("%Y%m%dT%H%M%S%fZ")
    output = root / "private/measure" / f"landing-{sha}-{stamp}"
    baseline = root / "private/baselines" / sha
    if baseline.exists():
        raise ValueError(f"landing baseline already exists: {baseline}")
    if not inputs_base.is_dir():
        raise ValueError(f"prepared inputs root is absent: {inputs_base}")
    prior_baseline = root / "private/baselines" / prior
    if not prior_baseline.is_dir() and not bootstrap:
        raise ValueError(f"previous baseline is absent: {prior_baseline}; use --bootstrap only for the first publication")
    if bootstrap and prior_baseline.is_dir():
        raise ValueError(f"previous baseline exists: {prior_baseline}; omit --bootstrap to verify reuse")
    previous_side = f"rev:{prior}" if bootstrap else f"baseline:{prior}"
    command = [str(executable), "--plugin-path", str(executable.parent),
               "--output", str(output), "--timeout-ms", "36000000", "measure",
               "--inputs", str(root / "crates/quirin-audit-plugin-measure/inputs/live.json"),
               "--inputs-base", str(inputs_base),
               "--side", f"prev={previous_side}", "--side", f"tip=rev:{sha}"]
    print(f"land: measuring {prior} -> {sha} at {output}", flush=True)
    subprocess.run(command, cwd=root, check=True)
    side = output / "sides/tip"
    if not side.is_dir() or not (output / "native-comparison.json").is_file():
        raise ValueError("measurement did not produce a tip side and comparison")
    subprocess.run([str(executable), "--plugin-path", str(executable.parent),
                    "--output", str(baseline), "measure", "publish", "--from", str(side)],
                   cwd=root, check=True)
    return baseline


def stale_records(root):
    """Each stale record as `(what, regenerating command)`."""
    stale = []
    try:
        matrix = json.loads((root / MATRIX).read_text(encoding="utf-8"))
        dimensions = matrix["dimensions"]
        for section in COUNTED_SECTIONS:
            if dimensions[section] != len(matrix[section]):
                stale.append((f"{MATRIX} dimensions.{section} is {dimensions[section]} against "
                              f"{len(matrix[section])} entries", MATRIX_COMMAND))
    except (OSError, ValueError, KeyError, TypeError) as error:
        stale.append((f"{MATRIX} cannot be read as the acceptance matrix ({error!r})",
                      MATRIX_COMMAND))
    index = subprocess.run([sys.executable, str(root / "scripts/build_docs_index.py"), "--check"],
                           capture_output=True, text=True)
    if index.returncode != 0:
        stale.append(("docs/reports/INDEX.md: " + (index.stderr or index.stdout).strip(),
                      INDEX_COMMAND))
    ledgers = root / "scripts/contribution_ledgers.py"
    if ledgers.is_file():
        page = subprocess.run([sys.executable, str(ledgers), "--root", str(root), "render",
                               "--check"], capture_output=True, text=True)
        if page.returncode != 0:
            stale.append(((page.stderr or page.stdout).strip(), CONTRIBUTIONS_COMMAND))
    return stale


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("command", nargs="?", choices=("sequence", "check", "publish"), default="sequence")
    parser.add_argument("--root", type=Path, default=ROOT)
    parser.add_argument("--previous", help="previous landing revision (default: HEAD first parent)")
    parser.add_argument("--inputs-base", type=Path,
                        default=Path("/media/Data/GitHub/quirin-lang"),
                        help="root of prepared live route inputs")
    parser.add_argument("--bootstrap", action="store_true",
                        help="measure the previous revision when starting the first baseline")
    args = parser.parse_args(argv)
    if args.command == "sequence":
        for number, step in enumerate(SEQUENCE, 1):
            print(f"{number}. {step}")
        return 0
    if args.command == "publish":
        try:
            baseline = publish(args.root.resolve(), args.previous, args.inputs_base.resolve(),
                               args.bootstrap)
        except (OSError, ValueError, subprocess.CalledProcessError) as error:
            print(f"land: publication failed: {error}", file=sys.stderr)
            return 1
        print(f"land: published {baseline}")
        return 0
    stale = stale_records(args.root.resolve())
    for what, command in stale:
        print(f"land: stale: {what}; run first: {command}", file=sys.stderr)
    if stale:
        print("land: regenerate, then rerun `python3 scripts/land.py check` before the Python "
              "suite", file=sys.stderr)
        return 1
    print("land: matrix dimensions, report index and contributions page are fresh")
    return 0


if __name__ == "__main__":
    sys.exit(main())
