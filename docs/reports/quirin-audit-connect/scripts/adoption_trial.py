#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
"""Card QUIRIN-AUDIT-CONNECT: what adopting one declared dependency in a
shipped contract would change, computed in a scratch copy, never in the tree.

    python3 docs/reports/quirin-audit-connect/scripts/adoption_trial.py SCRATCH

It adds `requires resource_contract_dependency("quirin-provider-libc",
libc::malloc, since "0.1.0");` to `onig_new` (quirin-provider-onigmo,
`onigmo/regex.qv`) in SCRATCH, prints the module diff, the load-path entry
`providers/contracts.py`'s generator writes for it, and the files whose
pinned identities follow. Nothing under the repository is written.
"""

import difflib
import importlib.util
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[4]
CRATE = ROOT / "providers/quirin-provider-onigmo"
MODULE = "onigmo/regex.qv"
CLAUSE = '    requires resource_contract_dependency("quirin-provider-libc", libc::malloc, since "0.1.0");\n'


def main():
    scratch = Path(sys.argv[1]).resolve()
    if scratch.is_relative_to(ROOT.resolve()) and not scratch.is_relative_to(ROOT / "private"):
        sys.exit("the scratch directory must lie outside the tree (or under private/)")
    original = (CRATE / "contracts" / MODULE).read_text()
    lines = original.splitlines(keepends=True)
    start = next(i for i, line in enumerate(lines) if line.startswith("extern fn onig_new("))
    anchor = next(i for i in range(start, len(lines)) if "resource_library_version" in lines[i])
    changed = lines[:anchor + 1] + [CLAUSE] + lines[anchor + 1:]
    target = scratch / "contracts" / MODULE
    target.parent.mkdir(parents=True, exist_ok=True)
    target.write_text("".join(changed))
    sys.stdout.writelines(difflib.unified_diff(lines, changed, f"a/{MODULE}", f"b/{MODULE}"))
    spec = importlib.util.spec_from_file_location("contracts", ROOT / "providers/contracts.py")
    contracts = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(contracts)
    entry = contracts.contract_dependencies("quirin-provider-onigmo", {MODULE: target})
    print("\ncontract_dependencies entry of quirin-provider-onigmo:")
    print(json.dumps(entry, indent=2))
    lock = json.loads((CRATE / "contracts.lock").read_text())
    pinned = next(e for e in lock["pinned"] if e["function"] == "onig_new")
    print("\nfollows:")
    print(f"- contracts.lock: onig_new contract_sha256 {pinned['contract_sha256']} no longer covers "
          "the text (contract-changed, Pending) until `quirin-audit pin` re-pins it after review")
    print("- quirin-provider-onigmo/Cargo.toml: a [dependencies] entry on quirin-provider-libc "
          "(cx_declared_crate), so providers/Cargo.lock and the user load path's `dependencies`")
    print("- providers/user-crates-load-path.json: `contract_dependencies` "
          "(`quirin-audit plugin views --write`)")
    print("- quirin-plugin.json's checker binding of onig_new: the module digest the checker task "
          "is bound to")


if __name__ == "__main__":
    main()
