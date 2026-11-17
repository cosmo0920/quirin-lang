#!/usr/bin/env python3
"""Print the `--provider` arguments that select, on the user load path
`providers/user-crates-load-path.json`, exactly the modules the Fluent Bit
profile's resource-contract manifest (`cycle19-resource-contracts.json`)
selects, plus every module of `quirin-provider-openssl`: the contract
selection of the opt-in `current` run (card OPENSSL-PROVIDER-PLUGIN).

    python3 opt_in_providers.py [REPOSITORY]   # one argument per line
"""
import json
import re
import sys
from pathlib import Path

root = Path(sys.argv[1] if len(sys.argv) > 1 else ".")
manifest = json.loads((root / "cycle19-resource-contracts.json").read_text())
selections = []
for module in manifest["modules"]:
    match = re.fullmatch(r"providers/([^/]+)/contracts/(.+)", module)
    selections.append(f"{match.group(1)}:{match.group(2)}" if match else module)
selections.append("quirin-provider-openssl")
for selection in selections:
    print("--provider")
    print(selection)
