#!/usr/bin/env python3
"""Read-only identity check for FB-AUDIT-02-RETRY-LIFETIME."""
import hashlib
import json
import pathlib
import subprocess

report = pathlib.Path(__file__).resolve().parent
evidence = json.loads((report / "evidence.json").read_text())
source = pathlib.Path("/home/cosmo/Documents/GitHub/fluent-bit")
revision = subprocess.check_output(["git", "-C", str(source), "rev-parse", "HEAD"], text=True).strip()
assert revision == evidence["source_revision"], (revision, evidence["source_revision"])
for row in evidence["source_files"]:
    path = source / row["path"]
    actual = hashlib.sha256(path.read_bytes()).hexdigest()
    assert actual == row["sha256"], row["path"]
    print(f"MATCH {row['path']} {actual}")
for anchor in evidence["line_anchors"]:
    lines = (source / anchor["path"]).read_text().splitlines()
    assert lines[anchor["line"] - 1] == anchor["text"], anchor
    print(f"ANCHOR_MATCH {anchor['path']}:{anchor['line']}")
for row in evidence["build_database"]["selected_file_match"]:
    assert row["copy_matches_864"]
    path = pathlib.Path(row["copy_path"])
    actual = hashlib.sha256(path.read_bytes()).hexdigest()
    expected = next(x["sha256"] for x in evidence["source_files"] if x["path"] == row["path"])
    assert actual == expected, row["path"]
    print(f"DB_COPY_MATCH {row['path']} {actual}")
print(f"SOURCE_REVISION {revision}")

extracted = (report / "artifacts/c1-extracted-function.c.txt").read_bytes()
assert hashlib.sha256(extracted).hexdigest() == evidence["experiment"]["extracted_function_sha256"]
source_text = (source / "src/flb_scheduler.c").read_text()
start = source_text.index("int flb_sched_request_create(struct flb_config *config, void *data, int tries)")
end = source_text.index("\nint flb_sched_request_destroy(", start)
assert extracted == (source_text[start:end].rstrip() + "\n").encode()
print("EXTRACTED_FUNCTION_MATCH")
