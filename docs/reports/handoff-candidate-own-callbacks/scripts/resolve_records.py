#!/usr/bin/env python3
"""resolve_records.py -- HANDOFF-CANDIDATE-OWN-CALLBACKS: resolve this card's
record conflicts after a rebase onto a main that landed another card's
records. docs/STATUS.md: the "Projected functions" row is this card's figure
with main's row as its "before it", the "Contract dependencies per verdict"
row is main's, and the "Registered follow-ups" row is main's count plus one
with main's row as its "before it". The increment record keeps this card's
paragraph first, then main's. The acceptance matrix is main's with this
card's ledger update applied (scripts/update_ledger.py)."""
import pathlib
import re
import subprocess
import sys

REPO = pathlib.Path(__file__).resolve().parents[4]
CONFLICT = re.compile(r"<<<<<<< [^\n]*\n(.*?)=======\n(.*?)>>>>>>> [^\n]*\n", re.S)
COUNT = re.compile(r"^\| Registered follow-ups \| \*\*(\d+)\*\* \(")


def status(match):
    main, mine = match.group(1).splitlines(), match.group(2).splitlines()
    if main[0].startswith("| Projected functions"):
        prefix = mine[0][: mine[0].index("before it: ") + len("before it: ")]
        rows = "".join(line + "\n" for line in main[1:] if line)
        return prefix + main[0][len("| Projected functions | "):] + "\n" + rows
    if main[0].startswith("| Registered follow-ups"):
        count = int(COUNT.match(main[0]).group(1))
        mine_count = int(COUNT.match(mine[0]).group(1))
        body = mine[0][len(COUNT.match(mine[0]).group(0)):]
        body = body[: body.index("; before it **")]
        rest = main[0][len(COUNT.match(main[0]).group(0)):]
        assert mine_count >= count - 1
        return (f"| Registered follow-ups | **{count + 1}** (" + body
                + f"; before it **{count}** (" + rest + "\n")
    raise SystemExit(f"unexpected STATUS conflict: {main[0][:80]}")


def main():
    path = REPO / "docs/STATUS.md"
    text = path.read_text()
    path.write_text(CONFLICT.sub(status, text))
    path = REPO / "docs/development/increments/wave-48-part8.md"
    text = path.read_text()
    path.write_text(CONFLICT.sub(lambda m: m.group(2) + "\n" + m.group(1), text))
    matrix = "docs/reviews/r0_3/acceptance_matrix.json"
    subprocess.run(["git", "checkout", "--ours", matrix], cwd=REPO, check=True)
    subprocess.run([sys.executable, str(pathlib.Path(__file__).parent / "update_ledger.py")],
                   cwd=REPO, check=True)


if __name__ == "__main__":
    main()
