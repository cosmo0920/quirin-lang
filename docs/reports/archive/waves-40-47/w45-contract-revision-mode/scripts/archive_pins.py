#!/usr/bin/env python3
"""Copy the `quirin-audit pin` logs of this card from private/rm-pin/ into the
report's locks/ directory, with the worktree root removed from every path, and
write the lock census beside them.

usage: archive_pins.py NAME...      (from the repository root)
"""
import json
import pathlib
import subprocess
import sys


def main() -> int:
    root = str(pathlib.Path.cwd()) + "/"
    out = pathlib.Path("docs/reports/w45-contract-revision-mode/locks")
    out.mkdir(parents=True, exist_ok=True)
    for name in sys.argv[1:]:
        text = pathlib.Path(f"private/rm-pin/{name}.log").read_text().replace(root, "")
        exit_code = pathlib.Path(f"private/rm-pin/{name}.exit").read_text().strip()
        (out / f"{name}.log").write_text(text + f"exit {exit_code}\n")
    census = subprocess.run(
        [sys.executable, "docs/reports/w45-contract-revision-mode/scripts/lock_census.py"],
        check=True, capture_output=True, text=True).stdout
    (out / "census.json").write_text(census)
    print(json.dumps({"archived": sys.argv[1:], "pinned": json.loads(census)["pinned"]}))
    return 0


if __name__ == "__main__":
    sys.exit(main())
