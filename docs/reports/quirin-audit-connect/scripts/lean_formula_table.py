#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
"""Card QUIRIN-AUDIT-CONNECT: (re)write `formulaTable` at the end of
`proof/QCore/MeasureConnect.lean` from `qcore_schemas::connect::FORMULAS`
(`crates/qcore-schemas/src/connect.rs`), every formula as written. The
Rust test `connect_formulas.rs` checks that each formula appears in the Lean
file as written and has its `formula_<id>` theorem."""

import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[4]
RUST = ROOT / "crates/qcore-schemas/src/connect.rs"
LEAN = ROOT / "proof/QCore/MeasureConnect.lean"
MARK = "/-- The formulas of `qcore_schemas::connect::FORMULAS`, as written. -/"

text = RUST.read_text()
block = text[text.index("pub const FORMULAS"):text.index("];", text.index("pub const FORMULAS"))]
pairs = re.findall(r'\(\s*"([a-z0-9_]+)",\s*"((?:[^"\\]|\\.)*)",\s*\)', block)
assert pairs, "no formulas"
lean = LEAN.read_text()
if MARK in lean:
    lean = lean[:lean.index(MARK)].rstrip() + "\n"
entries = ",\n".join(f'  ("{fid}",\n   "{formula}")' for fid, formula in pairs)
lean += (f"\n{MARK}\ndef formulaTable : List (String × String) := [\n{entries}\n]\n\n"
         f"theorem formulaTable_count : formulaTable.length = {len(pairs)} := by decide\n\n"
         "end QCore.MeasureConnect\n")
LEAN.write_text(lean)
print(len(pairs), "formulas")
