#!/usr/bin/env python3
"""Mutation harness for ct_fixture_byte_identity_is_exactly_the_reviewed_disclosure.

Card CT-SHARED-FIXTURE-DISCLOSURE. Each mutation rewrites (or creates) one
input, runs only the identity test, records the exit and the full output under
the log directory, restores the input by rewriting its original bytes, and
asserts the round trip by SHA-256 before the next mutation. A final unmutated
run must pass. Run from the repository root:

    python3 docs/reports/ct-shared-fixture-disclosure/scripts/mutate.py \
        docs/reports/ct-shared-fixture-disclosure/mutations

Exit 0 only when every mutation run failed and the baseline runs passed.
"""

import hashlib
import json
import pathlib
import subprocess
import sys

TEST = "ct_fixture_byte_identity_is_exactly_the_reviewed_disclosure"
MANIFEST = "docs/reevaluation/ct_manifest.json"
FIX = "compiler/tests/fixtures/"


def sha(data):
    return hashlib.sha256(data).hexdigest()


def run(log_dir, name):
    completed = subprocess.run(
        ["cargo", "test", "-p", "qcore-compiler", "--test", "ct_corpus", TEST],
        capture_output=True,
    )
    (log_dir / f"{name}.log").write_bytes(completed.stdout + completed.stderr)
    (log_dir / f"{name}.exit").write_text(f"{completed.returncode}\n")
    return completed.returncode, completed.stdout + completed.stderr


def manifest_edit(edit):
    def apply(original):
        document = json.loads(original)
        edit(document["artifact_identity"])
        return (json.dumps(document, indent=2) + "\n").encode()

    return apply


def group(identity, digest_prefix):
    return next(g for g in identity["shared_fixture_groups"] if g["sha256"].startswith(digest_prefix))


def set_role(identity):
    group(identity, "c775f721")["members"][1]["role"] = "case"


def set_relation(identity):
    group(identity, "c775f721")["relation"] = "shared_artifact"


def drop_group(identity):
    identity["shared_fixture_groups"] = [
        g for g in identity["shared_fixture_groups"] if not g["sha256"].startswith("7c75577b")
    ]


def bump_count(identity):
    identity["distinct_source_fixture_digests"] += 1


# (name, path, transform or None for a created copy, expected text in the failure)
MUTATIONS = [
    ("m1-edit-control-side", FIX + "ct/CT04/write_after_joins.qcore",
     lambda b: b + b"\n", "listed but not measured exactly"),
    ("m2-edit-case-side", FIX + "ct/CT02/case.qcore",
     lambda b: b + b"\n", "listed but not measured exactly"),
    ("m3-new-undisclosed-copy", FIX + "compose/ct05_copy_probe.qcore",
     None, "measured but not listed exactly"),
    ("m4-count-drift", MANIFEST, manifest_edit(bump_count), "as measured"),
    ("m5-role-misrecorded", MANIFEST, manifest_edit(set_role), "is recorded as `case` but is a `control`"),
    ("m6-relation-misrecorded", MANIFEST, manifest_edit(set_relation), "does not match its members' roles"),
    ("m7-group-removed", MANIFEST, manifest_edit(drop_group), "measured but not listed exactly"),
]


def main():
    log_dir = pathlib.Path(sys.argv[1])
    log_dir.mkdir(parents=True, exist_ok=True)
    ok = True
    code, _ = run(log_dir, "m0-baseline")
    print(f"m0-baseline exit {code}")
    ok &= code == 0
    for name, path, transform, needle in MUTATIONS:
        target = pathlib.Path(path)
        if transform is None:
            assert not target.exists(), target
            target.write_bytes(pathlib.Path(FIX + "ct/CT05/case.qcore").read_bytes())
            try:
                code, output = run(log_dir, name)
            finally:
                target.unlink()
            assert not target.exists(), f"{target} was not removed"
        else:
            original = target.read_bytes()
            digest = sha(original)
            mutated = transform(original)
            assert mutated != original, name
            target.write_bytes(mutated)
            try:
                code, output = run(log_dir, name)
            finally:
                target.write_bytes(original)
            assert sha(target.read_bytes()) == digest, f"{target} round trip failed"
        found = needle.encode() in output
        print(f"{name} exit {code} expected-message {'found' if found else 'MISSING'}")
        ok &= code != 0 and found
    code, _ = run(log_dir, "m8-baseline-after")
    print(f"m8-baseline-after exit {code}")
    ok &= code == 0
    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main())
