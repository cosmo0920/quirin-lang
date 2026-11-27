#!/usr/bin/env python3
"""CONTRACT-SUPERSESSION re-pin review receipt.

For every provider pin lock (`contracts.lock`, `bounds-contracts.lock`,
`outer-contracts.lock`), compare the BASE revision's bytes with the working
tree's and show that the ONLY change is the addition of the pin stamps
(`pinned_at` on every pinned, flexible and tip entry), the Fluent Bit revision
tie (`fluent_bit_revision`) and the format version (`lock_version` to 4): with
those three removed, the two documents are equal as JSON, every entry keeps
its function, module, contract digest and identities, and every entry carries
the one reviewed stamp.

Usage: review_repin.py BASE_REV STAMP REVISION OUT
"""
import hashlib
import json
import subprocess
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[4]
LOCKS = ("contracts.lock", "bounds-contracts.lock", "outer-contracts.lock")


def strip(document):
    document = json.loads(json.dumps(document))
    document.pop("lock_version", None)
    document.pop("fluent_bit_revision", None)
    for kind in ("pinned", "flexible", "tip"):
        for entry in document.get(kind, []):
            entry.pop("pinned_at", None)
    return document


def main():
    base, stamp, revision, out = sys.argv[1:5]
    rows = []
    ok = True
    for path in sorted((REPO / "providers").glob("*/*.lock")):
        if path.name not in LOCKS:
            continue
        relative = path.relative_to(REPO).as_posix()
        before_bytes = subprocess.run(["git", "-C", str(REPO), "show", f"{base}:{relative}"],
                                      check=True, capture_output=True).stdout
        after_bytes = path.read_bytes()
        before, after = json.loads(before_bytes), json.loads(after_bytes)
        entries = [e for kind in ("pinned", "flexible", "tip") for e in after.get(kind, [])]
        stamps = sorted({e.get("pinned_at") for e in entries}, key=str)
        same_content = strip(before) == strip(after)
        keys_before = {k: [sorted(e) for e in before.get(k, [])] for k in ("pinned", "flexible", "tip")}
        keys_after = {k: [sorted(set(e) - {"pinned_at"}) for e in after.get(k, [])]
                      for k in ("pinned", "flexible", "tip")}
        row = {
            "lock": relative,
            "base_sha256": hashlib.sha256(before_bytes).hexdigest(),
            "tip_sha256": hashlib.sha256(after_bytes).hexdigest(),
            "base_version": before.get("lock_version"),
            "tip_version": after.get("lock_version"),
            "entries": {k: len(after.get(k, [])) for k in ("pinned", "flexible", "tip")},
            "content_equal_without_stamps_tie_and_version": same_content,
            "entry_fields_unchanged_but_pinned_at": keys_before == keys_after,
            "stamps": stamps,
            "fluent_bit_revision": after.get("fluent_bit_revision"),
            "base_had_stamps_or_tie": any("pinned_at" in e for k in ("pinned", "flexible", "tip")
                                          for e in before.get(k, [])) or "fluent_bit_revision" in before,
        }
        row["ok"] = (same_content and keys_before == keys_after and stamps == [stamp]
                     and row["fluent_bit_revision"] == revision and row["tip_version"] == 4
                     and not row["base_had_stamps_or_tie"])
        ok &= row["ok"]
        rows.append(row)
    record = {
        "card": "CONTRACT-SUPERSESSION",
        "base": base,
        "stamp": stamp,
        "fluent_bit_revision": revision,
        "rule": "the only change allowed is the addition of pin stamps and the revision tie (with the format version 4 that expresses them); every clause, digest, module and identity is unchanged",
        "locks": rows,
        "lock_count": len(rows),
        "entry_count": sum(sum(r["entries"].values()) for r in rows),
        "ok": ok,
    }
    Path(out).write_text(json.dumps(record, indent=2) + "\n")
    print(f"{len(rows)} locks, {record['entry_count']} entries, ok={ok}")
    return 0 if ok else 1


if __name__ == "__main__":
    raise SystemExit(main())
