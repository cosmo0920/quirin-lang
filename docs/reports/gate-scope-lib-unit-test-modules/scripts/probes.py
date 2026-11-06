#!/usr/bin/env python3
"""The plan page's section 1 scoper probes, old and new, on this tree.

Card GATE-SCOPE-LIB-UNIT-TEST-MODULES.  Usage (repository root)::

    python3 docs/reports/gate-scope-lib-unit-test-modules/scripts/probes.py \
        --old-rev 13337d0af --work DIR > probes.json

The old scoper and walker table are ``--old-rev``'s (``git show``); the new
ones this checkout's.  Both plan each single path over this checkout.  The
first four paths are the plan page's section 1 probes; the rest are the
library unit-test shapes this card narrows.
"""
import argparse
import importlib.util
import json
from pathlib import Path
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[4]
sys.path.insert(0, str(ROOT / "scripts"))
import gate_scope as new  # noqa: E402

PROBES = [
    "crates/qcore-c-gateway/tests/libc_gap_declarations.rs",
    "providers/quirin-provider-libc/contracts/libc/stdlib.qv",
    "proof/QCore/WrittenSetDerivation.lean",
    "compiler/tests/contract_loader.rs",
    # This card's shapes.
    "crates/qcore-contract/tests/unit/contract_observation.rs",
    "crates/qcore-c-gateway/tests/unit/legacy_c.rs",
    "crates/qcore-c-gateway/tests/fixtures/contracts/q_base.qv",
    "crates/qcore-core/tests/unit/usage.rs",
    "proof/QCore/NullDeref.lean",
]


def load_old(rev, work):
    directory = Path(work) / f"old-{rev}"
    directory.mkdir(parents=True, exist_ok=True)
    for name in ("gate_scope.py", "gate_scope_walkers.json"):
        (directory / name).write_bytes(subprocess.run(
            ["git", "-C", str(ROOT), "show", f"{rev}:scripts/{name}"], capture_output=True,
            check=True).stdout)
    spec = importlib.util.spec_from_file_location("gate_scope_old", directory / "gate_scope.py")
    module = importlib.util.module_from_spec(spec)
    sys.modules["gate_scope_old"] = module
    spec.loader.exec_module(module)
    return module, module.load_table(directory / "gate_scope_walkers.json")


def summary(scope):
    targets = scope.get("rust_targets", [])
    return {"mode": scope["mode"], "full_reasons": scope["full_reasons"],
            "packages": scope["rust_packages"], "targets": targets,
            "lib_units": [uid for uid in targets if "::test::" not in uid],
            "full_crates_reached_whole": [c for c in new.FULL_CRATES if c in scope["affected"]],
            "lean": scope["lean"]["mode"], "lean_modules": len(scope["lean"]["modules"]),
            "lean_audits": len(scope["lean"]["audits"]), "provider_gate": scope["provider_gate"]}


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--old-rev", required=True)
    parser.add_argument("--work", required=True)
    args = parser.parse_args()
    old, table = load_old(args.old_rev, args.work)

    def read_text(path):
        return (ROOT / path).read_text(encoding="utf-8", errors="replace")

    old_model = old.Model(old.load_crates(ROOT), old.tracked_files(ROOT), read_text=read_text,
                          walkers=table["walkers"], expansions=table["expansions"])
    new_model = new.real_model()
    rows = []
    for path in PROBES:
        old_scope = old.plan([path], old_model, exists=lambda p: (ROOT / p).exists(),
                             read_text=read_text)
        rows.append({"path": path, "old": summary(old_scope),
                     "new": summary(new.plan([path], new_model))})
    print(json.dumps({"schema": "gate-scope-probes/2", "old_rev": args.old_rev, "rows": rows},
                     indent=1))


if __name__ == "__main__":
    main()
