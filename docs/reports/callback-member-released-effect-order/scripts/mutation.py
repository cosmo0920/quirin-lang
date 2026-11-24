#!/usr/bin/env python3
"""mutation.py WORKTREE LOGDIR -- CALLBACK-MEMBER-RELEASED-EFFECT-ORDER's one
mutation: the liveness read the late order relies on.

`legacy_member_path_route::apply` releases a linked object a candidate states
`#released`/`#moved` only while it is live (`if single && live`). The card's
ordering argument is that this read is what makes the composition after the
release refuse a linked object the release itself released. The mutation drops
the read (`if single`), runs the effect-order fixture, then restores the file
by rewriting its bytes, asserts the round trip and runs the fixture again (the
green baseline after the last mutation). Exit 0 only when the mutated run
fails and the restored run passes.
"""
import hashlib
import os
import subprocess
import sys

TARGET = "crates/qcore-c-gateway/src/legacy_member_path_route.rs"
OLD = b"                    if single && live {\n                        for child in &children {"
NEW = b"                    if single {\n                        for child in &children {"


def run(worktree: str, log: str) -> int:
    script = os.path.join(worktree, "docs", "reports", "callback-member-released-effect-order", "scripts", "run-test.sh")
    subprocess.run(
        ["bash", script, log, "callback_member_released_effect_order", "--test-threads=1"],
        check=False,
    )
    with open(log + ".exit") as handle:
        return int(handle.read().strip())


def main() -> int:
    worktree, logdir = sys.argv[1], sys.argv[2]
    os.makedirs(logdir, exist_ok=True)
    path = os.path.join(worktree, TARGET)
    with open(path, "rb") as handle:
        original = handle.read()
    digest = hashlib.sha256(original).hexdigest()
    if original.count(OLD) != 1:
        print("mutation site not found exactly once", file=sys.stderr)
        return 2
    try:
        with open(path, "wb") as handle:
            handle.write(original.replace(OLD, NEW))
        mutated = run(worktree, os.path.join(logdir, "mutant-1.log"))
    finally:
        with open(path, "wb") as handle:
            handle.write(original)
        os.utime(path)
    with open(path, "rb") as handle:
        restored = hashlib.sha256(handle.read()).hexdigest()
    assert restored == digest, "round trip failed"
    baseline = run(worktree, os.path.join(logdir, "baseline-1.log"))
    print(f"sha256 {digest} restored; mutant exit {mutated}; baseline exit {baseline}")
    return 0 if mutated != 0 and baseline == 0 else 1


if __name__ == "__main__":
    sys.exit(main())
