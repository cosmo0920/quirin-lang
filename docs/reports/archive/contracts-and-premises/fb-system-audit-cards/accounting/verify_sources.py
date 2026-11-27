#!/usr/bin/env python3
"""Read-only source pin check for FB-AUDIT-03."""
import hashlib
import pathlib
import subprocess
import sys

PIN = "864c3d97f3589515285a19396ed8dfc85cda08ae"
HASHES = {
    "plugins/in_storage_backlog/sb.c": "705012c176a27a34d0d101c40bc4b28eb7e5ef35595a40b20c2a3a2b8670cf04",
    "src/flb_input_chunk.c": "a537fe347ec1be6c2af7971b55579284e0402f351334f3cfcf0c2c800fe0977e",
    "src/flb_storage.c": "740c66e2ee3a05394c77c738099a4a6b6e6cbb7307cc3c92f2d5099b2b1a608b",
    "include/fluent-bit/flb_output.h": "815dc3774c93718b95b5b9474f93d790accb2c0bb9d0c45a15cb40c8649cd51d",
    "include/fluent-bit/flb_input_chunk.h": "fc456c464b52cdcb9e59d6b5782d34e928c47c052b42e08a3e56c1904ac01b64",
    "lib/chunkio/src/cio_file.c": "1c3eb285307108deb2b22c68cf282c0121619a35bbe36adc216f276d51925257",
    "lib/chunkio/src/cio_chunk.c": "8f6064c742b3fcc53d75b66dbd07d5c7f03d2f245ec7716f8605a2c3a6efd6c8",
    "lib/chunkio/src/cio_stream.c": "5226efb40578e3fc78de3c57cdb9e9e6adfa17b5d3aa2252c9545a7b777a2634",
    "lib/chunkio/src/cio_file_unix.c": "29a392dbf68676bea741d579f8b4cc6c5bcfc0ba3b3e8b3a11fb185443a07fb0",
}

root = pathlib.Path(sys.argv[1] if len(sys.argv) > 1 else ".").resolve()
head = subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=root, text=True).strip()
status = subprocess.check_output(["git", "status", "--porcelain=v1"], cwd=root, text=True)
ok = head == PIN and not status
print(f"HEAD {head} {'OK' if head == PIN else 'MISMATCH'}")
print(f"tracked worktree {'clean' if not status else 'dirty'}")
for name, want in HASHES.items():
    got = hashlib.sha256((root / name).read_bytes()).hexdigest()
    match = got == want
    ok &= match
    print(f"{name} {got} {'OK' if match else 'MISMATCH'}")
sys.exit(0 if ok else 1)
