#!/usr/bin/env python3
"""Build the pilot's second contract set: the committed contract selection with
the Fluent Bit input-chunk contracts marked `revision flexible`.

usage: pilot_variant.py OUT_DIR      (from the repository root; OUT_DIR must not exist)

Copies the committed `cycle19-resource-contracts.json` and every provider
contract text, inventory, load path and lock into OUT_DIR (so the manifest's
relative paths and the load path's pins resolve exactly as in the
repository), then adds `requires resource_revision(flexible);` to every
declaration of the `fluent_bit/input_chunk_*.qcore` modules, whose functions
are defined in `src/flb_input_chunk.c`, an audited unit of the system slice.
Nothing in the repository is modified. Prints the flipped functions.
"""
import json
import pathlib
import re
import shutil
import sys

SELECTOR = "    requires resource_contract_v1;\n"
MARKER = "    requires resource_revision(flexible);\n"


def main() -> int:
    out = pathlib.Path(sys.argv[1])
    out.mkdir(parents=True, exist_ok=False)
    shutil.copyfile("cycle19-resource-contracts.json", out / "cycle19-resource-contracts.json")
    providers = pathlib.Path("providers")
    for path in sorted(providers.rglob("*")):
        if "target" in path.parts or not path.is_file():
            continue
        if path.suffix in (".qcore", ".qv", ".json") or path.name in ("contracts.lock", "Cargo.toml"):
            destination = out / path
            destination.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(path, destination)
    flipped = []
    modules = sorted(p for p in
                     (out / "providers/quirin-provider-fluent-bit/contracts/fluent_bit")
                     .glob("input_chunk_*") if p.suffix in (".qcore", ".qv"))
    for module in modules:
        text = module.read_text()
        flipped += re.findall(r"^extern fn ([a-z_0-9]+)", text, re.M)
        if text.count(SELECTOR) != len(re.findall(r"^extern fn ", text, re.M)):
            raise SystemExit(f"{module}: not every declaration is written out")
        module.write_text(text.replace(SELECTOR, SELECTOR + MARKER))
    print(json.dumps({"modules": [str(m.relative_to(out)) for m in modules],
                      "flexible": flipped}, indent=2))
    return 0


if __name__ == "__main__":
    sys.exit(main())
