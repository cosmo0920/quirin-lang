#!/usr/bin/env python3
"""Mutation control: disable the generic-selection hook, run the card's tests,
restore the exact bytes and assert the round trip.

Usage: mutate_hook.py OUT_DIR MUTATION
"""
import hashlib
import os
import pathlib
import subprocess
import sys

ROOT = pathlib.Path(__file__).resolve().parents[4]
MUTATIONS = {
    "hook": (
        "crates/qcore-c-gateway/src/legacy_c.rs",
        "    let resolved = generic_selection::resolve_in_body(tokens, function);\n",
        "    let resolved: Option<Vec<Token>> = None; // MUTATION: hook disabled\n",
    ),
    "pointer-result": (
        "crates/qcore-c-gateway/src/legacy_generic_selection.rs",
        "        if !declarations.declared(texts[0]) {\n",
        "        if !(declarations.declared(texts[0]) || true) {\n",
    ),
    "invariance": (
        "crates/qcore-c-gateway/src/legacy_generic_selection.rs",
        "    if cores.iter().any(|(core, _)| !same(core, first)) {\n",
        "    if cores.iter().any(|(core, _)| !same(core, first)) && false {\n",
    ),
}


def main() -> int:
    out = pathlib.Path(sys.argv[1])
    relative, ORIGINAL, MUTANT = MUTATIONS[sys.argv[2]]
    TARGET = ROOT / relative
    out.mkdir(parents=True, exist_ok=False)
    before = TARGET.read_bytes()
    digest = hashlib.sha256(before).hexdigest()
    text = before.decode()
    assert text.count(ORIGINAL) == 1
    TARGET.write_bytes(text.replace(ORIGINAL, MUTANT).encode())
    env = dict(os.environ, CARGO_INCREMENTAL="0")
    try:
        with open(out / "mutant-test.log", "wb") as log:
            code = subprocess.run(
                ["cargo", "test", "-p", "qcore-c-gateway", "--test", "generic_selection",
                 "--no-fail-fast"],
                cwd=ROOT, env=env, stdout=log, stderr=log,
            ).returncode
    finally:
        TARGET.write_bytes(before)
        os.utime(TARGET)
    restored = hashlib.sha256(TARGET.read_bytes()).hexdigest()
    (out / "mutant-test.exit").write_text(f"{code}\n")
    (out / "round-trip.txt").write_text(
        f"before {digest}\nrestored {restored}\nidentical {digest == restored}\n"
    )
    print(f"mutant exit {code}; round trip identical {digest == restored}")
    return 0 if digest == restored else 1


if __name__ == "__main__":
    sys.exit(main())
