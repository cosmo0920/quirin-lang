#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
"""Card QUIRIN-AUDIT-CONNECT: write `proof/AuditMeasureConnect.lean`, one
`#print axioms` per theorem of `QCore.MeasureConnect`, in file order."""

import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[4]
LEAN = ROOT / "proof/QCore/MeasureConnect.lean"
AUDIT = ROOT / "proof/AuditMeasureConnect.lean"
HEADER = """-- SPDX-License-Identifier: Apache-2.0
--
-- Licensed under the Apache License, Version 2.0 (the "License");
-- you may not use this file except in compliance with the License.
-- You may obtain a copy of the License at
--
--     http://www.apache.org/licenses/LICENSE-2.0
--
-- Unless required by applicable law or agreed to in writing, software
-- distributed under the License is distributed on an "AS IS" BASIS,
-- WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
-- See the License for the specific language governing permissions and
-- limitations under the License.

import QCore.MeasureConnect

/-! Audited companion to QUIRIN-AUDIT-CONNECT (measure chain, measure compose,
declared contract dependencies). -/
"""

text = LEAN.read_text()
names = []
namespace = []
for line in text.splitlines():
    m = re.match(r"namespace (\S+)", line)
    if m:
        namespace.append(m.group(1))
        continue
    m = re.match(r"end (\S+)", line)
    if m and namespace and namespace[-1] == m.group(1):
        namespace.pop()
        continue
    m = re.match(r"theorem ([A-Za-z_][A-Za-z0-9_']*)", line)
    if m:
        names.append(".".join(namespace + [m.group(1)]))
AUDIT.write_text(HEADER + "\n" + "\n".join(f"#print axioms {name}" for name in names) + "\n")
print(len(names), "theorems")
