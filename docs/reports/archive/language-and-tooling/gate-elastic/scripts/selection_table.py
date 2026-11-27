#!/usr/bin/env python3
"""Card GATE-ELASTIC: the scoped gate's selection for representative paths.

Run from the repository root:
    python3 docs/reports/gate-elastic/scripts/selection_table.py > selection.json
Prints JSON, one row per path and reader granularity; plans only, runs nothing.
"""
import json
from pathlib import Path
import sys

sys.path.insert(0, str(Path(__file__).resolve().parents[4] / "scripts"))
import gate_scope as g  # noqa: E402

PATHS = [
    "docs/agents/validation.md",
    "docs/reports/gate-elastic/README.md",
    "scripts/reconciliation_sweep.py",
    "editors/README.md",
    "examples/records/owned_field.qr",
    "compiler/tests/fixtures/ct/CT07/case.qr",
    "compiler/tests/fixtures/ct/CT07/expected.json",
    "crates/qcorec/src/main.rs",
    "crates/qcore-audit/src/lib.rs",
    "crates/qcore-verifier-host/src/lib.rs",
    "crates/quirin-audit-plugin-libevent/src/lib.rs",
    "proof/QCore/RkLogCallerFrame.lean",
    "proof/AuditRkLogCallerFrame.lean",
    "providers/quirin-provider-libc/contracts/libc/errno.qv",
    "providers/package.py",
    "tests/support/tracked_files.rs",
    "crates/qcore-c-gateway/src/lib.rs",
    "Cargo.lock",
]


def main():
    crates, files = g.load_crates(), g.tracked_files()
    rows = []
    for granularity in g.GRANULARITIES:
        model = g.Model(crates, files, granularity=granularity)
        for path in PATHS:
            scope = g.plan([path], model)
            scoped = scope["mode"] == "scoped"
            rows.append({
                "path": path, "granularity": granularity, "mode": scope["mode"],
                "rust_packages": scope["rust_packages"] if scoped else "all",
                "lean": scope["lean"]["mode"], "provider_gate": scope["provider_gate"],
                "full_reasons": scope["full_reasons"],
            })
    print(json.dumps({"schema": "gate-elastic-selection/1", "rows": rows}, indent=1))


if __name__ == "__main__":
    main()
