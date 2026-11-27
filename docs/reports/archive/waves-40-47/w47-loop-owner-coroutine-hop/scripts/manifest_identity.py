#!/usr/bin/env python3
"""manifest_identity.py RUN_DIR ARCHIVE_DIR [--root-alias ROOT=CANONICAL]...

Compare each file of a run with a filename-keyed archive manifest
(`{"NAME": {"file": "NAME.gz", "sha256": RAW_SHA256, "bytes": N}}`): the raw
SHA-256 of RUN_DIR/NAME against the manifest's, and, for a file that differs,
whether rewriting each ROOT to CANONICAL makes it equal to the archived
content (a checkout-path-only difference, disclosed, never a substitute).
Prints one line per file; exits 0 only when resource-diagnostics.json is
byte-identical. (baseline_identity.py reads label-keyed manifests only.)
"""
import gzip
import hashlib
import json
import pathlib
import sys


def main():
    run, archive = pathlib.Path(sys.argv[1]), pathlib.Path(sys.argv[2])
    aliases = []
    args = sys.argv[3:]
    while args:
        if args[0] == "--root-alias" and len(args) > 1:
            root, canonical = args[1].split("=", 1)
            aliases.append((root.encode(), canonical.encode()))
            args = args[2:]
        else:
            raise SystemExit(f"unexpected argument {args[0]}")
    manifest = json.loads((archive / "manifest.json").read_text())
    verdict_identical = False
    for name, entry in sorted(manifest.items()):
        path = run / name
        if not path.exists():
            print(f"{name}: missing from the run")
            continue
        data = path.read_bytes()
        sha = hashlib.sha256(data).hexdigest()
        same = sha == entry["sha256"]
        line = f"{name}: run {sha[:16]} archive {entry['sha256'][:16]} {'identical' if same else 'DIFFERENT'}"
        if not same and aliases:
            rewritten = data
            for root, canonical in aliases:
                rewritten = rewritten.replace(root, canonical)
            archived = gzip.decompress((archive / entry["file"]).read_bytes())
            line += f"; after root alias {'identical' if rewritten == archived else 'still different'}"
        print(line)
        if name == "resource-diagnostics.json":
            verdict_identical = same
    return 0 if verdict_identical else 1


if __name__ == "__main__":
    raise SystemExit(main())
