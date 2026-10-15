#!/usr/bin/env python3
"""resolve_records.py RUN -- HANDOFF-DIRECT-CALL-READING: re-apply this card's
records on main's after a rebase (main's version of each conflicted file
checked out first): the ledger (update_ledger.py RUN), the two STATUS rows
(prepended to main's current rows as "before it") and the increment
paragraph (newest first). The texts are records/status-corpus.txt,
records/status-ledger.txt and records/increment.txt beside this script's
directory. Idempotent: a record already present is not added again."""
import pathlib
import subprocess
import sys

HERE = pathlib.Path(__file__).resolve().parent
REPO = HERE.parents[3]
RECORDS = HERE.parent / "records"
STATUS = REPO / "docs/STATUS.md"
INCREMENT = REPO / "docs/development/increments/wave-48-part8.md"
CARD = "HANDOFF-DIRECT-CALL-READING"


def prepend_row(text: str, label: str, record: str, joint: str) -> str:
    marker = f"| {label} | "
    at = text.index(marker) + len(marker)
    if text[at:].startswith(record.split(" (")[0]) and CARD in text[at : at + 600]:
        return text
    return text[:at] + record + joint + text[at:]


def main() -> None:
    subprocess.run([sys.executable, str(HERE / "update_ledger.py"), sys.argv[1]], check=True)
    status = STATUS.read_text(encoding="utf-8")
    status = prepend_row(
        status, "Projected functions", (RECORDS / "status-corpus.txt").read_text().strip(),
        "; before it: ",
    )
    status = prepend_row(
        status, "Registered follow-ups", (RECORDS / "status-ledger.txt").read_text().strip(),
        "; before it ",
    )
    STATUS.write_text(status, encoding="utf-8")
    increment = INCREMENT.read_text(encoding="utf-8")
    paragraph = (RECORDS / "increment.txt").read_text().strip()
    if f"**`{CARD}`" not in increment:
        at = increment.index("\n**`") + 1
        increment = increment[:at] + paragraph + "\n\n" + increment[at:]
    INCREMENT.write_text(increment, encoding="utf-8")


if __name__ == "__main__":
    main()
