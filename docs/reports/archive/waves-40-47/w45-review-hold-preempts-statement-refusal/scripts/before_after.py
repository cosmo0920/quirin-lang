#!/usr/bin/env python3
"""before_after.py BEFORE AFTER -- raw per-file comparison of two run
directories of the same route (card W45-REVIEW-HOLD-PREEMPTS-STATEMENT-REFUSAL).

For each file present in either: `same` (bytes equal), `same-sorted` (equal
once its lines are sorted: a stream written in completion order), `differs`
with the first differing line of each side trimmed, or `missing`. Lines that
name the run's own directory are compared with that directory replaced by
`RUN`, and reported as `same-modulo-run-dir` when that alone makes them equal.
"""
import pathlib
import sys


def lines(path: pathlib.Path, run_dir: pathlib.Path):
    text = path.read_text(encoding="utf-8", errors="replace")
    return text.replace(str(run_dir), "RUN").splitlines()


def main() -> int:
    before, after = pathlib.Path(sys.argv[1]), pathlib.Path(sys.argv[2])
    names = sorted({p.relative_to(before).as_posix() for p in before.rglob("*") if p.is_file()}
                   | {p.relative_to(after).as_posix() for p in after.rglob("*") if p.is_file()})
    for name in names:
        b, a = before / name, after / name
        if not b.is_file() or not a.is_file():
            print(f"missing   {name}")
            continue
        if b.read_bytes() == a.read_bytes():
            print(f"same      {name}")
            continue
        bl, al = lines(b, before), lines(a, after)
        if bl == al:
            print(f"same-modulo-run-dir {name}")
            continue
        if sorted(bl) == sorted(al):
            print(f"same-sorted {name}")
            continue
        diff = [(x, y) for x, y in zip(bl, al) if x != y]
        first = diff[0] if diff else (f"<{len(bl)} lines>", f"<{len(al)} lines>")
        print(f"differs   {name}: {len(diff)} line(s); first:\n    - {first[0][:200]}\n    + {first[1][:200]}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
