#!/usr/bin/env python3
"""base-submodule.py -- MEMBER-PREPASS-OTHER-ROUTES: `git archive` leaves
`third_party/libevent` empty in private/mpor/base-src, and the pinned route
checks that submodule's revision (constraint: the pinned libevent revision).
Copy this worktree's checked-out submodule files into the base tree and point
the copy's `.git` file at the same module git directory by absolute path, so
the BEFORE binary's check reads the pinned revision. Reads only this
worktree's own submodule; writes only under private/mpor/base-src."""
import pathlib
import shutil

REPO = pathlib.Path(__file__).resolve().parents[4]
SOURCE = REPO / "third_party/libevent"
TARGET = REPO / "private/mpor/base-src/third_party/libevent"


def main():
    gitdir = (SOURCE / (SOURCE / ".git").read_text().split(":", 1)[1].strip()).resolve()
    if TARGET.exists():
        shutil.rmtree(TARGET)
    shutil.copytree(SOURCE, TARGET, symlinks=True, ignore=shutil.ignore_patterns(".git"))
    (TARGET / ".git").write_text(f"gitdir: {gitdir}\n")
    print(f"{TARGET} -> {gitdir}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
