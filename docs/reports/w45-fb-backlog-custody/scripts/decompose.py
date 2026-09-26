#!/usr/bin/env python3
"""Build one decomposition variant of the arbiter binary and run it once.

Usage: decompose.py VARIANT OUT_DIR TARGET_DIR

VARIANT is one of

  * `declarations-only`: the declaration reader skips element statement
    expressions (criterion 2) but rules 1-4 of the list-iteration module
    admit nothing (every hook returns its "not mine" answer);
  * `iteration-only`: rules 1-4 are active but the declaration reader does
    not skip element statement expressions.

The variant is produced by rewriting the named hook lines of the working
tree, building the arbiter example into TARGET_DIR (never `target/`), and
restoring every rewritten file by rewriting its original bytes; the restore
is asserted by SHA-256 before the run starts. The run then goes through
`docs/reports/w43-return-expression/scripts/run_audit.py`'s command shape
with the variant binary. Measurement aid only: no variant is a product.
"""

import hashlib
import json
import os
import pathlib
import subprocess
import sys

ROOT = pathlib.Path(__file__).resolve().parents[4]
PIN = pathlib.Path("/home/cosmo/Documents/GitHub/qcore-c/build/cycle19")

LIST = ROOT / "crates/qcore-c-gateway/src/legacy_list_iteration.rs"
TYPED = ROOT / "crates/qcore-c-gateway/src/typed_field_access.rs"

REWRITES = {
    "declarations-only": [
        (
            LIST,
            "        let Some(shape) = recognise_header(clauses) else {\n            return Ok(None);\n        };",
            "        let Some(shape) = recognise_header(clauses).filter(|_| false) else {\n            return Ok(None);\n        };",
        ),
        (
            LIST,
            "        let shape = element_shape(texts)?;\n        // A formal's own slot",
            "        let shape = element_shape(texts).filter(|_| false)?;\n        // A formal's own slot",
        ),
    ],
    "iteration-only": [
        (
            TYPED,
            "    let skipped = crate::legacy_c::list_iteration::element_expression_spans(body);\n    for start in 0..body.len() {\n        if start > 0 && !matches!(body[start - 1], \"{\" | \"}\" | \";\" | \"(\" | \":\") {\n            continue;\n        }\n        if skipped.iter().any(|(open, close)| *open < start && start <= *close) {\n            continue;\n        }\n        if !is_declaration_candidate(body, start) {\n            continue;\n        }\n        let Some(end) = statement_end(body, start) else {\n            note_refusal!(",
            "    let skipped: Vec<(usize, usize)> = Vec::new();\n    for start in 0..body.len() {\n        if start > 0 && !matches!(body[start - 1], \"{\" | \"}\" | \";\" | \"(\" | \":\") {\n            continue;\n        }\n        if skipped.iter().any(|(open, close)| *open < start && start <= *close) {\n            continue;\n        }\n        if !is_declaration_candidate(body, start) {\n            continue;\n        }\n        let Some(end) = statement_end(body, start) else {\n            note_refusal!(",
        ),
        (
            TYPED,
            "    // Card W45-FB-BACKLOG-CUSTODY: as in `read_locals`.\n    let skipped = crate::legacy_c::list_iteration::element_expression_spans(body);",
            "    // Card W45-FB-BACKLOG-CUSTODY: as in `read_locals`.\n    let skipped: Vec<(usize, usize)> = Vec::new();",
        ),
    ],
}


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main():
    variant, out, target = sys.argv[1], pathlib.Path(sys.argv[2]).resolve(), pathlib.Path(sys.argv[3]).resolve()
    rewrites = REWRITES[variant]
    originals = {path: path.read_bytes() for path, _, _ in rewrites}
    digests = {path: sha(path) for path in originals}
    try:
        for path, old, new in rewrites:
            text = path.read_text()
            if text.count(old) != 1:
                raise SystemExit(f"{path}: hook not found exactly once")
            path.write_text(text.replace(old, new))
        environment = dict(os.environ, CARGO_INCREMENTAL="0", CARGO_TARGET_DIR=str(target))
        build = subprocess.run(
            ["cargo", "build", "--release", "-p", "qcore-compiler", "--example", "cycle19_fluent_bit_libraries_audit"],
            cwd=ROOT, env=environment, capture_output=True, text=True,
        )
    finally:
        for path, data in originals.items():
            path.write_bytes(data)
            os.utime(path, None)
    for path, digest in digests.items():
        assert sha(path) == digest, f"{path} was not restored"
    if build.returncode != 0:
        print(build.stderr[-4000:])
        raise SystemExit(f"variant build failed: {build.returncode}")
    out.mkdir(parents=True, exist_ok=False)
    binary = target / "release/examples/cycle19_fluent_bit_libraries_audit"
    command = [
        str(binary),
        "--pinned-evidence",
        str(PIN / "c-gateway-wave-pin/fluent-bit-clang-source"),
        str(PIN / "c-gateway-wave-pin/monkey/monkey-build/compile_commands.json"),
        str(out / "monkey-source.json"),
        str(out / "resource-diagnostics.json"),
    ]
    environment = dict(os.environ)
    environment["QCORE_CLANG"] = "/usr/bin/clang"
    environment["QCORE_CHUNKIO_PACKAGE"] = str(PIN / "provider-packages-wave20/qcore.chunkio")
    environment.pop("QCORE_SOURCE_CONTRACTS", None)
    environment["QCORE_AUDIT_JOBS"] = "8"
    with open(out / "audit.log", "wb") as log:
        code = subprocess.run(command, env=environment, stdout=log, stderr=log, cwd=ROOT).returncode
    record = {
        "variant": variant,
        "binary_sha256": sha(binary),
        "restored": {str(path.relative_to(ROOT)): digest for path, digest in digests.items()},
        "exit": code,
    }
    (out / "run.json").write_text(json.dumps(record, indent=2) + "\n")
    print(json.dumps(record, indent=2))


if __name__ == "__main__":
    main()
