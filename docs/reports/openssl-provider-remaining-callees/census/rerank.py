#!/usr/bin/env python3
"""Re-rank OPENSSL-PROVIDER-PLUGIN's census after this card's contracts.

    python3 rerank.py > rerank.md

Reads `docs/reports/openssl-provider-plugin/census/census.json` (that card's
census, unchanged) and the contracted functions of
`providers/quirin-provider-openssl` (every `extern fn` of its modules), and
prints, per route that has OpenSSL rows, the rows still blocked by an
uncontracted OpenSSL callee (sole: no other diagnostic) and the callees left,
ranked by the rows they block alone, then by pairs. A census row's other
diagnostics are counted as the census counted them; whether a row moves is
the measured opt-in run's to say, not this file's.
"""
import json
import re
import sys
from pathlib import Path

root = Path(__file__).resolve().parents[4]
census = json.loads((root / "docs/reports/openssl-provider-plugin/census/census.json").read_text())
contracted = set()
for module in (root / "providers/quirin-provider-openssl/contracts/openssl").glob("*.qv"):
    text = "\n".join(line.split("//")[0] for line in module.read_text().splitlines())
    contracted.update(re.findall(r"^extern fn (\w+)\(", text, re.M))

out = [f"Contracted by `quirin-provider-openssl`: {len(contracted)} functions.", ""]
for route in census["routes"]:
    if not route["rows"]:
        continue
    rows = route["rows"]
    left = {c["callee"]: c for c in route["callees"] if c["callee"] not in contracted}
    remaining_rows = [r for r in rows if set(r["openssl_callees"]) - contracted]
    sole_after = [r for r in remaining_rows if r["other_diagnostics"] == 0]
    clear = [r for r in rows if not set(r["openssl_callees"]) - contracted]
    out += [f"### {route['route']}", "",
            f"Rows with an OpenSSL callee: {len(rows)}; every OpenSSL callee contracted: "
            f"{len(clear)} ({sum(r['other_diagnostics'] == 0 for r in clear)} with no other "
            f"diagnostic); still blocked by an uncontracted OpenSSL callee: {len(remaining_rows)} "
            f"({len(sole_after)} sole). Uncontracted callees: {len(left)} of "
            f"{route['openssl_callees']}, {sum(c['pairs'] for c in left.values())} of "
            f"{route['openssl_pairs']} pairs.", ""]
    sole_by = {}
    for r in sole_after:
        for callee in set(r["openssl_callees"]) - contracted:
            sole_by[callee] = sole_by.get(callee, 0) + 1
    ranked = sorted(left.values(), key=lambda c: (-sole_by.get(c["callee"], 0), -c["pairs"],
                                                 c["callee"]))
    out += ["| Callee | Rows it blocks with no other diagnostic | Pairs | Sites |",
            "|---|---:|---:|---:|"]
    out += [f"| `{c['callee']}` | {sole_by.get(c['callee'], 0)} | {c['pairs']} | {c['sites']} |"
            for c in ranked]
    out += ["", "| Row still blocked | Other diagnostics | Uncontracted OpenSSL callees |",
            "|---|---:|---|"]
    out += [f"| `{Path(r['file']).name}::{r['function']}` | {r['other_diagnostics']} | "
            + ", ".join(f"`{c}`" for c in sorted(set(r["openssl_callees"]) - contracted)) + " |"
            for r in sorted(remaining_rows, key=lambda r: (r["other_diagnostics"], r["function"]))]
    out.append("")
sys.stdout.write("\n".join(out))
