#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
"""Card QUIRIN-AUDIT-CONNECT: append the CX block (cx_block.py) after the K6
block of `crates/qcore-frontend/src/premise_formula.rs`, once."""

import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[4]
TARGET = ROOT / "crates/qcore-frontend/src/premise_formula.rs"
SCRIPT = Path(__file__).resolve().parent / "cx_block.py"


def block(flag):
    return subprocess.run([sys.executable, str(SCRIPT), flag], check=True,
                          capture_output=True, text=True).stdout.rstrip("\n")


text = TARGET.read_text()
if '"cx_step"' in text:
    sys.exit("the CX block is already present")
sort_anchor = ('        "what a place reader answers for an expression: a root object with its '
               'member, subscript and dereference steps, or a refusal",\n    ),\n')
term_anchor = ("rule 10n's `arrow_through_address` shape\",\n        block: \"K6\",\n    },\n")
assert text.count(sort_anchor) == 1 and text.count(term_anchor) == 1
text = text.replace(sort_anchor, sort_anchor + block("--rust-sorts") + "\n")
text = text.replace(term_anchor, term_anchor + block("--rust-terms") + "\n")
TARGET.write_text(text)
print("inserted")
