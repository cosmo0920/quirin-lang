#!/usr/bin/env python3
"""Print the profile's default contract manifest (cycle19-resource-contracts.json)
as `--provider CRATE:MODULE` arguments, one word per line, so a provider-route
run selects exactly the modules the default route reads."""
import json
import pathlib
import sys

root = pathlib.Path(sys.argv[1])
manifest = json.loads((root / "cycle19-resource-contracts.json").read_text())
for module in manifest["modules"]:
    if module.startswith("providers/"):
        _, crate, contracts, rest = module.split("/", 3)
        assert contracts == "contracts", module
        selection = f"{crate}:{rest}"
    else:
        selection = module
    print("--provider")
    print(selection)
