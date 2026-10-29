#!/usr/bin/env python3
"""Compare the executed test set of a plain `cargo test` log with runner runs.

Card TEST-STORAGE-BUDGET's parity check. A test is the pair (executable stem,
test name): the stem is the test executable's file name without cargo's hash
(``ct_corpus-26056d77914fe904`` -> ``ct_corpus``; ``doc-tests`` for doc-tests).
Two runs agree when

* the multisets of (stem, name) are equal: the same tests ran in the same
  executables, each as often;
* the multisets of (stem, name) reported ``ignored`` are equal, and so are
  those reported ``FAILED``.

Names are read from every ``test <name> ...`` fragment of a line, not only
from whole lines: a test that re-executes its own binary (the gateway rule
switches) has its children's output interleaved with its own, so one line can
hold several fragments (``test a ... test b ... ok``); the outcome then belongs
to the last fragment and the others' outcomes are unreadable, which is why
the outcome sets compare only the readable ones. For the same reason the
plain log's ``test result:`` totals can miss a summary that a child's output
split (§6 of the report); the runner's totals come from each unit's own
summaries. Exit 0 only when every runner run equals the reference.

    compare_tests.py BASE_CARGO_TEST_LOG RUN [RUN ...]

A RUN is a runner's ``summary.json`` or another plain ``cargo test`` log.
"""

from __future__ import annotations

from collections import Counter
import json
from pathlib import Path
import re
import sys

RUNNING = re.compile(r"^\s+Running .*\((?P<path>[^)]*)\)$")
DOC = re.compile(r"^\s+Doc-tests ")
FRAGMENT = re.compile(r"test (.+?) \.\.\.(?= |$)")
OUTCOME = re.compile(r" \.\.\. (?P<outcome>ok|FAILED|ignored(?:, .*)?)$")


def stem(path: str) -> str:
    return re.sub(r"-[0-9a-f]{16}(\.exe)?$", "", Path(path).name)


class Tests:
    def __init__(self) -> None:
        self.names: Counter = Counter()
        self.ignored: Counter = Counter()
        self.failed: Counter = Counter()

    def add_line(self, unit: str, line: str) -> None:
        if not line.startswith("test ") or line.startswith("test result:"):
            return
        names = FRAGMENT.findall(line)
        for name in names:
            self.names[(unit, name)] += 1
        match = OUTCOME.search(line)
        if names and match:
            if match["outcome"].startswith("ignored"):
                self.ignored[(unit, names[-1])] += 1
            elif match["outcome"] == "FAILED":
                self.failed[(unit, names[-1])] += 1


def from_log(path: Path) -> Tests:
    tests = Tests()
    current = None
    for line in path.read_text(encoding="utf-8", errors="replace").splitlines():
        match = RUNNING.match(line)
        if match:
            current = stem(match["path"])
        elif DOC.match(line):
            current = "doc-tests"
        elif current is not None:
            tests.add_line(current, line)
    return tests


def from_summary(path: Path) -> tuple[Tests, dict]:
    document = json.loads(path.read_text(encoding="utf-8"))
    tests = Tests()
    for unit in document["units"]:
        name = "doc-tests" if unit["uid"] == "doc-tests" else stem(unit["executable"])
        stdout = Path(unit["stdout_log"])
        lines = (stdout.read_text(encoding="utf-8", errors="replace").splitlines()
                 if stdout.exists() else unit["tests"])
        for line in lines:
            tests.add_line(name, line.rstrip("\r"))
    return tests, document["totals"]


def log_totals(path: Path) -> dict:
    """The `test result:` totals of a plain log (a split summary is missed)."""
    totals = {"passed": 0, "failed": 0, "ignored": 0}
    pattern = re.compile(r"^test result: \w+\. (\d+) passed; (\d+) failed; (\d+) ignored;")
    for line in path.read_text(encoding="utf-8", errors="replace").splitlines():
        match = pattern.match(line)
        if match:
            for key, value in zip(("passed", "failed", "ignored"), match.groups()):
                totals[key] += int(value)
    return totals


def describe(tests: Tests) -> str:
    return (f"{sum(tests.names.values())} tests, {sum(tests.ignored.values())} ignored, "
            f"{sum(tests.failed.values())} failed")


def main(argv: list[str]) -> int:
    if len(argv) < 2:
        print(__doc__, file=sys.stderr)
        return 2
    reference = from_log(Path(argv[0]))
    print(f"reference {argv[0]}: {describe(reference)}")
    code = 0
    for summary in argv[1:]:
        if summary.endswith(".json"):
            tests, totals = from_summary(Path(summary))
        else:  # a second plain `cargo test` log
            tests, totals = from_log(Path(summary)), log_totals(Path(summary))
        differences = []
        for label, ours, theirs in (("ran", reference.names, tests.names),
                                    ("ignored", reference.ignored, tests.ignored),
                                    ("FAILED", reference.failed, tests.failed)):
            for key, count in sorted((ours - theirs).items()):
                differences.append(f"  {label} only in reference ({count}x): {key[0]}: {key[1]}")
            for key, count in sorted((theirs - ours).items()):
                differences.append(f"  {label} only in this run ({count}x): {key[0]}: {key[1]}")
        verdict = "EQUAL" if not differences else "DIFFERENT"
        print(f"{summary}: {describe(tests)}; runner totals {totals['passed']} passed, "
              f"{totals['failed']} failed, {totals['ignored']} ignored; {verdict}")
        print("\n".join(differences)) if differences else None
        if differences:
            code = 1
    return code


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
