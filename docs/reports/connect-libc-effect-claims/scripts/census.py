#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
#
# Licensed under the Apache License, Version 2.0 (the "License");
# you may not use this file except in compliance with the License.
# You may obtain a copy of the License at
#
#     http://www.apache.org/licenses/LICENSE-2.0
#
# Unless required by applicable law or agreed to in writing, software
# distributed under the License is distributed on an "AS IS" BASIS,
# WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
# See the License for the specific language governing permissions and
# limitations under the License.

"""Card CONNECT-LIBC-EFFECT-CLAIMS census (read-only).

Every clause of every module of every crate on the shipped load path
(`providers/contract-load-path.json`) and the user load path
(`providers/user-crates-load-path.json`), quirin-provider-libc excepted, that
names a libc/POSIX function (`providers/quirin-provider-libc/libc-names.txt`,
the list rule 10j's corpus test reads) in any of three positions:

  literal  inside a string literal of the clause (`callee == "send"`,
           `path_calls == "send,..."`);
  bare     as a bare identifier compared with `==`/`!=`;
  frame    as the framed callee of `quant cell F: T |-> #untouched;`.

Each hit is printed with its declaration, the `KEY == "..."` keys of its
clause, and the libc names it carries. Classification and consumers are the
report's (README.md), not this script's. Usage:
    python3 docs/reports/connect-libc-effect-claims/scripts/census.py [ROOT]
writes JSON to stdout.
"""
import json
import re
import sys
from pathlib import Path

root = Path(sys.argv[1] if len(sys.argv) > 1 else ".").resolve()
providers = root / "providers"
names = {line.strip() for line in
         (providers / "quirin-provider-libc" / "libc-names.txt").read_text().splitlines()
         if line.strip() and not line.lstrip().startswith("#")}


def strip(text):
    return re.sub(r"/\*.*?\*/", " ", re.sub(r"//[^\n]*", "", text), flags=re.S)


crates = {}
for manifest, path in (("shipped", "contract-load-path.json"), ("user", "user-crates-load-path.json")):
    for crate in json.loads((providers / path).read_text())["crates"]:
        crates.setdefault(crate["name"], {"contracts": crate["contracts"], "modules": crate["modules"],
                                          "load_paths": []})["load_paths"].append(manifest)

rows = []
scanned = {"crates": 0, "modules": 0, "declarations": 0}
for name in sorted(crates):
    if name == "quirin-provider-libc":
        continue
    crate = crates[name]
    scanned["crates"] += 1
    for module in crate["modules"]:
        scanned["modules"] += 1
        text = strip((providers / crate["contracts"] / module).read_text())
        for decl in re.finditer(r"\bextern\s+fn\s+(\w+)(.*?)\};", text, re.S):
            scanned["declarations"] += 1
            function, body = decl.group(1), decl.group(2)
            for clause in re.findall(r'(?:[^;"]|"[^"]*")+', body):
                clause = " ".join(clause.split())
                literals = re.findall(r'"([^"]*)"', clause)
                literal = sorted({token for lit in literals for token in re.findall(r"[A-Za-z_]\w*", lit)
                                  if token in names})
                unquoted = re.sub(r'"[^"]*"', '""', clause)
                bare = sorted({m.group(2) for m in re.finditer(r"(==|!=)\s*([A-Za-z_]\w*)", unquoted)
                               if m.group(2) in names})
                frame = sorted({m.group(1) for m in re.finditer(
                    r"\bquant\s+cell\s+([A-Za-z_]\w*)\s*:\s*[A-Za-z_]\w*\s*\|->\s*#\s*untouched", unquoted)
                                if m.group(1) in names})
                for position, hit in (("literal", literal), ("bare", bare), ("frame", frame)):
                    if hit:
                        rows.append({
                            "crate": name, "load_paths": crate["load_paths"], "module": module,
                            "declaration": function, "position": position,
                            "keys": sorted(set(re.findall(r"\b([a-z_][a-z0-9_]*)\s*==\s*\"", clause))),
                            "libc_names": hit,
                            "clause": clause if len(clause) <= 240 else clause[:237] + "...",
                        })

json.dump({"schema": "connect-libc-effect-claims-census/1", "scanned": scanned, "hits": rows},
          sys.stdout, indent=1)
sys.stdout.write("\n")
