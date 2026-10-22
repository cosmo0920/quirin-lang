#!/usr/bin/env python3
"""W48-RK-LOG-CALLER-FRAME: move the module-count pins the new
librdkafka/log_caller_frame.qv module moves (selected resource modules
65 -> 66, corpus modules 137 -> 138, owned qcore-provider modules 83 -> 84).
Each edit replaces exactly one line and adds one explaining comment; a line
that is not found exactly once refuses. Run once from the repository root.
"""
import pathlib

ROOT = pathlib.Path(__file__).resolve().parents[4]
RUST = "    // 64 -> 65 ... -> 66 by card W48-RK-LOG-CALLER-FRAME: `librdkafka/log_caller_frame.qv`.\n"
EDITS = [
    ("crates/qcore-c-gateway/tests/prod_contracts.rs",
     "    assert_eq!(contracts.source_assumptions().len(), 65);\n}\n",
     RUST + "    assert_eq!(contracts.source_assumptions().len(), 66);\n}\n"),
    ("crates/qcore-c-gateway/tests/neighbours.rs",
     "    assert_eq!(contracts.source_assumptions().len(), 65);\n}\n",
     RUST + "    assert_eq!(contracts.source_assumptions().len(), 66);\n}\n"),
    ("crates/qcore-audit/tests/provider_selection.rs",
     "    assert_eq!(entries.len(), 65);\n",
     RUST + "    assert_eq!(entries.len(), 66);\n"),
    ("crates/qcore-audit/tests/provider_selection.rs",
     "    assert_eq!(total, 65);\n",
     RUST + "    assert_eq!(total, 66);\n"),
    ("crates/qcore-audit/tests/provider_selection.rs",
     "    assert_eq!(entries(&manifest_text).1.len(), 65);\n",
     RUST + "    assert_eq!(entries(&manifest_text).1.len(), 66);\n"),
    ("crates/qcore-audit/tests/provider_selection.rs",
     "    assert_eq!(from_selection.source_assumptions().len(), 65);\n",
     RUST + "    assert_eq!(from_selection.source_assumptions().len(), 66);\n"),
    ("compiler/tests/contract_loader.rs",
     "    assert_eq!(modules.len(), 137, \"corpus module count\");\n",
     "    // 137 -> 138 by card W48-RK-LOG-CALLER-FRAME: `librdkafka/log_caller_frame.qv`.\n"
     "    assert_eq!(modules.len(), 138, \"corpus module count\");\n"),
    ("compiler/tests/contract_loader.rs",
     "    assert_eq!(contracts.source_assumptions().len(), 65);\n    check_source_assumptions",
     RUST + "    assert_eq!(contracts.source_assumptions().len(), 66);\n    check_source_assumptions"),
    ("compiler/tests/contract_loader.rs",
     "    assert_eq!(certificate.contracts.source_assumptions().len(), 65);\n",
     RUST + "    assert_eq!(certificate.contracts.source_assumptions().len(), 66);\n"),
    ("scripts/tests/test_contract_corpus.py",
     "        self.assertEqual(len(modules), 137)\n"
     "        self.assertEqual(sum(any(part.startswith(\"qcore-provider-\") for part in path.parts) for path in modules.values()), 83)\n",
     "        # 137 -> 138: W48-RK-LOG-CALLER-FRAME's librdkafka/log_caller_frame.qv\n"
     "        # in qcore-provider-librdkafka (83 -> 84 owned).\n"
     "        self.assertEqual(len(modules), 138)\n"
     "        self.assertEqual(sum(any(part.startswith(\"qcore-provider-\") for part in path.parts) for path in modules.values()), 84)\n"),
]


def main():
    for path, old, new in EDITS:
        file = ROOT / path
        text = file.read_text()
        if text.count(old) != 1:
            raise SystemExit(f"{path}: expected exactly one `{old.strip()}`")
        file.write_text(text.replace(old, new.replace("64 -> 65 ... -> 66", "65 -> 66")))
        print("edited", path)


if __name__ == "__main__":
    main()
