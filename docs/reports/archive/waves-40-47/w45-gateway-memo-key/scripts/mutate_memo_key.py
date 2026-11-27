#!/usr/bin/env python3
"""Weaken the member-release memo's hit check and require the tests to notice.

Usage: mutate_memo_key.py LOG_DIR

Each mutation rewrites `ReleasingEntry::answers` in
`crates/qcore-c-gateway/src/legacy_member_release.rs`, runs the module's unit
tests (`cargo test -p qcore-c-gateway --lib member_release`, CARGO_INCREMENTAL=0)
with stdout/stderr in LOG_DIR/<name>.log and the exit in LOG_DIR/<name>.exit,
then restores the original bytes, asserts the round trip and touches the file.
A mutation is killed when the run exits nonzero. Exits 0 only when every
mutation is killed and the restored file is byte-identical.
"""

import hashlib
import os
import pathlib
import subprocess
import sys

ROOT = pathlib.Path(__file__).resolve().parents[4]
SOURCE = ROOT / "crates/qcore-c-gateway/src/legacy_member_release.rs"
CHECK = """        self.digest == digest
            && self.texts.len() == tokens.len()
            && self
                .texts
                .iter()
                .zip(tokens)
                .all(|(cached, token)| *cached == token.text)
"""
MUTATIONS = {
    # The rule the card replaced: a digest match alone serves the entry.
    "digest-only": "        self.digest == digest && !self.texts.is_empty() && !tokens.is_empty()\n",
    # The digest and the length, but no text comparison.
    "digest-and-length": "        self.digest == digest && self.texts.len() == tokens.len()\n",
}


def main() -> int:
    logs = pathlib.Path(sys.argv[1]).resolve()
    logs.mkdir(parents=True, exist_ok=False)
    original = SOURCE.read_bytes()
    digest = hashlib.sha256(original).hexdigest()
    if original.decode().count(CHECK) != 1:
        raise SystemExit("the hit check is not where the harness expects it")
    killed = {}
    environment = dict(os.environ, CARGO_INCREMENTAL="0")
    try:
        for name, replacement in MUTATIONS.items():
            SOURCE.write_bytes(original.decode().replace(CHECK, replacement).encode())
            with open(logs / f"{name}.log", "wb") as log:
                code = subprocess.run(
                    ["cargo", "test", "-p", "qcore-c-gateway", "--lib", "member_release"],
                    cwd=ROOT,
                    env=environment,
                    stdout=log,
                    stderr=log,
                ).returncode
            (logs / f"{name}.exit").write_text(f"{code}\n")
            killed[name] = code != 0
    finally:
        SOURCE.write_bytes(original)
        SOURCE.touch()
    restored = hashlib.sha256(SOURCE.read_bytes()).hexdigest() == digest
    summary = {"mutations": len(MUTATIONS), "killed": killed, "restored": restored}
    (logs / "summary.txt").write_text(f"{summary}\n")
    print(summary)
    return 0 if restored and all(killed.values()) else 1


if __name__ == "__main__":
    sys.exit(main())
