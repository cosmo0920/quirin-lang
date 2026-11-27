#!/usr/bin/env python3
"""Card REFUSED-STORE-OF-CALL-RESULT-ESCAPE: write one unit's `-E -P` text.

Usage: preprocess_unit.py COMPILE_COMMANDS.json FILE_SUFFIX OUT.i

Takes the first compile-database record whose file ends with FILE_SUFFIX,
drops `-c` and `-o OUT`, inserts `-E -P`, and runs it in the record's
directory. The census compared the result's SHA-256 with the report row's
`source_identity` (equal for cmt_map.c), so the text is the analysed text.
"""

import json
import shlex
import subprocess
import sys


def main() -> int:
    database = json.load(open(sys.argv[1], encoding="utf-8"))
    for entry in database:
        if not entry["file"].endswith(sys.argv[2]):
            continue
        argv = []
        skip = False
        for argument in shlex.split(entry["command"]):
            if skip:
                skip = False
                continue
            if argument == "-o":
                skip = True
                continue
            if argument == "-c":
                continue
            argv.append(argument)
        argv[1:1] = ["-E", "-P"]
        result = subprocess.run(argv, cwd=entry["directory"], capture_output=True, text=True)
        with open(sys.argv[3], "w", encoding="utf-8") as out:
            out.write(result.stdout)
        print(result.returncode, result.stderr[:500])
        return result.returncode
    print("no compile record", file=sys.stderr)
    return 1


if __name__ == "__main__":
    sys.exit(main())
