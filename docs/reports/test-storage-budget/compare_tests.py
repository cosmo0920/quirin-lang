#!/usr/bin/env python3
"""Compare the executed test set of a plain `cargo test` log with runner runs.

Card TEST-STORAGE-BUDGET's parity check. A test is the pair (executable stem,
libtest line): the stem is the test executable's file name without cargo's
hash (``ct_corpus-26056d77914fe904`` -> ``ct_corpus``; ``doc-tests`` for the
doc-test unit), the line is ``test <name> ... <outcome>`` with the outcome
(``ok``, ``FAILED``, ``ignored[, reason]``). Two runs agree when the multisets
of pairs are equal: the same names in the same executables with the same
outcomes. Prints the totals of each run and every difference; exit 0 only
when every runner run equals the reference.

    compare_tests.py BASE_CARGO_TEST_LOG SUMMARY_JSON [SUMMARY_JSON ...]
"""

from __future__ import annotations

from collections import Counter
import json
from pathlib import Path
import re
import sys

RUNNING = re.compile(r"^\s+Running .*\((?P<path>[^)]*)\)$")
DOC = re.compile(r"^\s+Doc-tests ")
TEST = re.compile(r"^test (?P<name>.+) \.\.\. (?P<outcome>ok|FAILED|ignored(?:, .*)?)$")
SUMMARY = re.compile(r"^test result: (ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored;")


def stem(path: str) -> str:
    name = Path(path).name
    return re.sub(r"-[0-9a-f]{16}$", "", name)


def from_log(path: Path) -> tuple[Counter, list[int]]:
    tests: Counter = Counter()
    totals = [0, 0, 0]
    current = None
    for line in path.read_text(encoding="utf-8", errors="replace").splitlines():
        match = RUNNING.match(line)
        if match:
            current = stem(match["path"])
            continue
        if DOC.match(line):
            current = "doc-tests"
            continue
        match = TEST.match(line)
        if match and current is not None:
            tests[(current, line)] += 1
            continue
        match = SUMMARY.match(line)
        if match:
            for index in range(3):
                totals[index] += int(match.group(index + 2))
    return tests, totals


def from_summary(path: Path) -> tuple[Counter, list[int]]:
    document = json.loads(path.read_text(encoding="utf-8"))
    tests: Counter = Counter()
    for unit in document["units"]:
        name = "doc-tests" if unit["uid"] == "doc-tests" else stem(unit["executable"])
        for line in unit["tests"]:
            tests[(name, line)] += 1
    total = document["totals"]
    return tests, [total["passed"], total["failed"], total["ignored"]]


def main(argv: list[str]) -> int:
    if len(argv) < 2:
        print(__doc__, file=sys.stderr)
        return 2
    reference, totals = from_log(Path(argv[0]))
    print(f"reference {argv[0]}: {sum(reference.values())} test lines; "
          f"{totals[0]} passed; {totals[1]} failed; {totals[2]} ignored")
    code = 0
    for summary in argv[1:]:
        tests, totals = from_summary(Path(summary))
        missing = reference - tests
        extra = tests - reference
        verdict = "EQUAL" if not missing and not extra else "DIFFERENT"
        print(f"{summary}: {sum(tests.values())} test lines; {totals[0]} passed; "
              f"{totals[1]} failed; {totals[2]} ignored; {verdict}")
        for (unit, line), count in sorted(missing.items()):
            print(f"  only in reference ({count}x): {unit}: {line}")
        for (unit, line), count in sorted(extra.items()):
            print(f"  only in this run ({count}x): {unit}: {line}")
        if verdict != "EQUAL":
            code = 1
    return code


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
