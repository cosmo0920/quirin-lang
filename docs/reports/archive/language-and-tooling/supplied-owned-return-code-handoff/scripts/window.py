#!/usr/bin/env python3
"""window.py PATH FROM[-TO] [FROM[-TO]...] -- SUPPLIED-OWNED-RETURN-CODE-HANDOFF:
print numbered line windows of PATH (relative to the corpus tree root,
66910c10) and say whether the same lines are byte-identical in the 046accf6
tree. A reading aid for the census; nothing is inferred from it."""
import pathlib
import sys

OLD = pathlib.Path("/home/cosmo/Documents/GitHub/qcore-c/build/cycle19/c-gateway-wave-pin/fluent-bit-clang-source")
NEW = pathlib.Path("/home/cosmo/Documents/GitHub/qcore-c/private/w45-fb/w46-external-callee-contracts/attempt-1/fluent-bit-clang-source")


def main():
    path = sys.argv[1]
    old = (OLD / path).read_text(errors="replace").splitlines()
    new_path = NEW / path
    new = new_path.read_text(errors="replace").splitlines() if new_path.exists() else None
    for spec in sys.argv[2:]:
        start, _, end = spec.partition("-")
        start = int(start)
        end = int(end) if end else start + 12
        text = old[start - 1:end]
        same = new is not None and new[start - 1:end] == text
        print(f"==== {path}:{start}-{end} ({'identical' if same else 'DIFFERS'} at 046accf6)")
        for number, line in enumerate(text, start):
            print(f"{number:6} {line}")


if __name__ == "__main__":
    main()
