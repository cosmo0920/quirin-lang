#!/usr/bin/env python3
"""The plan page's section 1 scoper probes, old and new, on this tree.

Card GATE-SCOPE-PRECISE-SELECTION.  Usage (repository root)::

    python3 docs/reports/gate-scope-precise-selection/scripts/probes.py OLD_GATE_SCOPE.py

Prints, per probe path, each scoper's mode, packages, test targets and the
reasons a FULL_CRATES member or full mode was chosen, as JSON.
"""
import importlib.util
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[4]
sys.path.insert(0, str(ROOT / "scripts"))
import gate_scope as new  # noqa: E402

PROBES = [
    "crates/qcore-c-gateway/tests/libc_gap_declarations.rs",
    "providers/quirin-provider-libc/contracts/libc/stdlib.qv",
    "proof/QCore/WrittenSetDerivation.lean",
    "compiler/tests/contract_loader.rs",
]


def load_old(path):
    spec = importlib.util.spec_from_file_location("gate_scope_old", path)
    module = importlib.util.module_from_spec(spec)
    sys.modules["gate_scope_old"] = module
    spec.loader.exec_module(module)
    return module


def summary(scope):
    return {"mode": scope["mode"], "full_reasons": scope["full_reasons"],
            "packages": scope["rust_packages"], "targets": scope.get("rust_targets", []),
            "full_crates_reached_whole": [c for c in new.FULL_CRATES if c in scope["affected"]],
            "lean": scope["lean"]["mode"], "provider_gate": scope["provider_gate"]}


def main():
    old = load_old(sys.argv[1])
    def read_text(path):
        return (ROOT / path).read_text(encoding="utf-8", errors="replace")

    old_model = old.Model(old.load_crates(ROOT), old.tracked_files(ROOT), read_text=read_text)
    new_model = new.real_model()
    rows = []
    for path in PROBES:
        old_scope = old.plan([path], old_model, exists=lambda p: (ROOT / p).exists(),
                             read_text=read_text)
        rows.append({"path": path, "old": summary(old_scope),
                     "new": summary(new.plan([path], new_model))})
    print(json.dumps(rows, indent=1))


if __name__ == "__main__":
    main()
