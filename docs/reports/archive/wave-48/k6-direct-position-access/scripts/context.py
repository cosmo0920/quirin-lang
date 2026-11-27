#!/usr/bin/env python3
"""Card K6: print the original-source context of FILE:LINE origins in the prepared Fluent Bit tree.

usage: context.py BEFORE AFTER FILE:LINE...
Reads only (the prepared tree is the user's; nothing is written there).
"""
import sys

TREE = ("/media/Data/GitHub/quirin-lang/build/cycle19/clang/046accf6aded7906f66a1baf0202da349e521736/"
        "fluent-bit-clang-source/fluent-bit-clang-source/")


def main():
    before, after = int(sys.argv[1]), int(sys.argv[2])
    for loc in sys.argv[3:]:
        path, line = loc.rsplit(":", 1)
        line = int(line)
        lines = open(TREE + path, errors="replace").read().split("\n")
        print("==", loc)
        for i in range(max(1, line - before), min(len(lines), line + after) + 1):
            print((">" if i == line else " ") + lines[i - 1])


if __name__ == "__main__":
    main()
