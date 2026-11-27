#!/usr/bin/env python3
"""Set the rule-10n step flags of a checkout for one measured step.

Usage: step_flags.py ROOT STEP      (STEP 0..4; 4 is the committed code)

Step k turns on the first k shapes, in the report's order: 1 address values
(`ADDRESS_ARM`), 2 controlling-scope node matching (`SCOPE_MATCH`), 3 the
anonymous inline enumeration member (`ANONYMOUS_ENUM_ARM`), 4 connective
returns (`RETURN_CONNECTIVES`). Each step's binary is built from the named
commit with exactly these edits; restore with STEP 4.
"""

import pathlib
import re
import sys

FLAGS = [
    ("crates/qcore-c-gateway/src/typed_position_shapes.rs", "ADDRESS_ARM"),
    ("crates/qcore-c-gateway/src/typed_position_shapes.rs", "SCOPE_MATCH"),
    ("crates/qcore-c-gateway/src/typed_position_shapes.rs", "ANONYMOUS_ENUM_ARM"),
    ("crates/qcore-c-gateway/src/legacy_typed_residuals.rs", "RETURN_CONNECTIVES"),
]


def main():
    root, step = pathlib.Path(sys.argv[1]), int(sys.argv[2])
    for number, (path, name) in enumerate(FLAGS, 1):
        file = root / path
        text = file.read_text()
        pattern = re.compile(rf"(pub\(crate\) const {name}: bool = )(true|false);")
        if len(pattern.findall(text)) != 1:
            raise SystemExit(f"{name} not found exactly once in {path}")
        text = pattern.sub(rf"\g<1>{'true' if number <= step else 'false'};", text)
        file.write_text(text)
        print(name, number <= step)


if __name__ == "__main__":
    main()
