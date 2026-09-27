#!/usr/bin/env python3
"""Place and remove the W45-TEST-HYGIENE decoys in a delegated worktree.

    decoys.py place  ROOT SET   # SET: a-untracked, a-ignored, b-untracked, b-ignored
    decoys.py status ROOT SET
    decoys.py remove ROOT SET

Every decoy is a file the tree-walking tests would read if they walked the
filesystem. `*-untracked` decoys sit in new, unignored directories; `*-ignored`
decoys sit beside a new `.gitignore` that ignores them (or under `/private/`).
Set `a` is read only by test-side walks; set `b` lies in contract roots, which
production loaders also read. `place` refuses to overwrite anything and records
the exact paths it created in `private/w45-test-hygiene/decoys-SET.json`;
`remove` deletes exactly those paths (files, then the directories it created,
deepest first) and nothing else. Refuses to run outside `.claude/worktrees/`,
so the user's main checkout is never touched.
"""
import json
import subprocess
import sys
from pathlib import Path

SOCKET_V4 = "examples/sockets/inet_v4_echo.qr"
SOCKET_V6 = "examples/sockets/inet_v6_echo.qr"
CT_CASE = "compiler/tests/fixtures/ct/CT01/case.qr"

# (relative path, content: str literal, or ("copy", repository path))
SETS = {
    "a-untracked": [
        # std_socket_inet: a second host-span claimant outside the two examples.
        ("examples/sockets/w45-th-untracked/inet_v4_echo.qcore", ("copy", SOCKET_V4)),
        # examples_suite / examples_cli: a family directory with no page.
        ("examples/w45-th-untracked/decoy.qcore", "fn main() -> i32 {\n    return 0;\n}\n"),
        # ct_corpus: an undisclosed byte-identical copy of a CT fixture.
        ("compiler/tests/fixtures/w45-th-untracked/case_copy.qcore", ("copy", CT_CASE)),
        # proof_leg_digests: a Lean digest theorem for an unpinned case.
        ("proof/QCore/W45ThUntrackedDecoy.lean",
         "theorem ct98_w45_untracked_digest : (1 : Nat) = 1 := rfl\n"),
        # proof_leg_digests: a Rust digest pin with no Lean twin.
        ("compiler/tests/w45_th_untracked_decoy.rs",
         "const CT97_LEAN_W45_UNTRACKED_DIGEST: u64 = 1;\n"),
        # composition_bounds: a production-looking count statement.
        ("crates/qcore-verification/src/w45_th_untracked_decoy.rs",
         "fn decoy() { let _ = Bounds { schedules_explored: Some(1) }; }\n"),
        # qcore-c-gateway census_sink_inventory: a sink with an unlisted variable.
        ("crates/qcore-c-gateway/src/w45_th_untracked_decoy.rs",
         "static SINK: Mutex<Option<BufWriter<File>>>> = Mutex::new(None);\n"
         "pub const W45_DECOY: &str = \"QCORE_W45_TH_UNTRACKED\";\n"),
        # conformance packages: a crate directory carrying a package template.
        ("providers/w45-th-untracked/provider-template.json", "{}\n"),
        # test_retired_scripts: a live-looking file naming a retired script.
        ("scripts/w45-th-untracked/notes.md", "Run prepare_cycle19_monkey.py first.\n"),
    ],
    "a-ignored": [
        # The 2026-09-26 incident: an export under /private/.
        ("private/w45-th-decoys/export/examples/sockets/inet_v6_echo.qr", ("copy", SOCKET_V6)),
        ("examples/w45-th-ignored/.gitignore", "*\n"),
        ("examples/w45-th-ignored/decoy.qcore", "fn main() -> i32 {\n    return 0;\n}\n"),
        ("compiler/tests/fixtures/w45-th-ignored/.gitignore", "*\n"),
        ("compiler/tests/fixtures/w45-th-ignored/case_copy.qcore", ("copy", CT_CASE)),
        ("proof/QCore/.gitignore", "W45ThIgnoredDecoy.lean\n.gitignore\n"),
        ("proof/QCore/W45ThIgnoredDecoy.lean",
         "theorem ct96_w45_ignored_digest : (1 : Nat) = 1 := rfl\n"),
        ("compiler/tests/.gitignore", "w45_th_ignored_decoy.rs\n.gitignore\n"),
        ("compiler/tests/w45_th_ignored_decoy.rs",
         "const CT95_LEAN_W45_IGNORED_DIGEST: u64 = 1;\n"),
        ("compiler/src/w45-th-ignored/.gitignore", "*\n"),
        ("compiler/src/w45-th-ignored/decoy.rs",
         "fn decoy() { let _ = Bounds { max_operations: Some(1) }; }\n"),
        ("crates/qcore-c-gateway/src/.gitignore", "w45_th_ignored_decoy.rs\n.gitignore\n"),
        ("crates/qcore-c-gateway/src/w45_th_ignored_decoy.rs",
         "static SINK: Mutex<Option<BufWriter<File>>>> = Mutex::new(None);\n"
         "pub const W45_DECOY: &str = \"QCORE_W45_TH_IGNORED\";\n"),
        ("providers/w45-th-ignored/.gitignore", "*\n"),
        ("providers/w45-th-ignored/provider-template.json", "{}\n"),
        ("scripts/w45-th-ignored/.gitignore", "*\n"),
        ("scripts/w45-th-ignored/notes.md", "Run prepare_cycle19_monkey.py first.\n"),
    ],
    "b-untracked": [
        # Contract roots: the test-side corpus walk (count pins), the provider
        # crates' index tests and test_contract_corpus's libc-name check.
        ("providers/quirin-provider-libc/contracts/w45-th-untracked/decoy.qcore",
         "module contract;\nextern fn w45_th_not_a_libc_name() -> i32\n"),
    ],
    "b-ignored": [
        ("providers/qcore-provider-chunkio/contracts/w45-th-ignored/.gitignore", "*\n"),
        ("providers/qcore-provider-chunkio/contracts/w45-th-ignored/decoy.qcore",
         "module contract;\nextern fn w45_th_ignored_decoy() -> i32\n"),
    ],
}


def ledger(root, name):
    return root / "private/w45-test-hygiene" / f"decoys-{name}.json"


def status(root, paths):
    lines = []
    for relative in paths:
        result = subprocess.run(
            ["git", "-C", str(root), "status", "--porcelain", "--ignored", "--untracked-files=all",
             "--", relative], capture_output=True, text=True, check=True)
        state = result.stdout.strip().split(" ", 1)[0] if result.stdout.strip() else "tracked-clean"
        lines.append(f"{state:>13}  {relative}")
    return lines


def place(root, name):
    record = ledger(root, name)
    if record.exists():
        sys.exit(f"{record} exists: remove the previous placement first")
    files, directories = [], []
    for relative, content in SETS[name]:
        target = root / relative
        if target.exists() or target.is_symlink():
            sys.exit(f"refusing to overwrite {target}")
        missing = []
        parent = target.parent
        while not parent.exists():
            missing.append(parent)
            parent = parent.parent
        for directory in reversed(missing):
            directory.mkdir()
            directories.append(str(directory.relative_to(root)))
        data = (root / content[1]).read_bytes() if isinstance(content, tuple) else content.encode()
        target.write_bytes(data)
        files.append(relative)
    record.parent.mkdir(parents=True, exist_ok=True)
    record.write_text(json.dumps({"files": files, "directories": directories}, indent=2) + "\n")
    print("\n".join(status(root, files)))


def remove(root, name):
    record = ledger(root, name)
    placed = json.loads(record.read_text())
    for relative in placed["files"]:
        (root / relative).unlink()
    for relative in sorted(placed["directories"], key=lambda p: -p.count("/")):
        (root / relative).rmdir()
    record.unlink()
    leftovers = [relative for relative in placed["files"] + placed["directories"]
                 if (root / relative).exists()]
    if leftovers:
        sys.exit(f"left behind: {leftovers}")
    print(f"removed {len(placed['files'])} files and {len(placed['directories'])} directories")


def main():
    action, root, name = sys.argv[1], Path(sys.argv[2]).resolve(), sys.argv[3]
    if "/.claude/worktrees/" not in root.as_posix() + "/":
        sys.exit(f"{root} is not a delegated worktree")
    if name not in SETS:
        sys.exit(f"unknown set {name}")
    if action == "place":
        place(root, name)
    elif action == "remove":
        remove(root, name)
    elif action == "status":
        print("\n".join(status(root, [relative for relative, _ in SETS[name]])))
    else:
        sys.exit(f"unknown action {action}")


if __name__ == "__main__":
    main()
