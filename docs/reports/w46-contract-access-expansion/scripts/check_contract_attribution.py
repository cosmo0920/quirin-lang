#!/usr/bin/env python3
"""Check coverage and source identity of the manually attributed contract deltas.

This checks record identity, not the correctness of the written source review.
"""
import argparse
import collections
import hashlib
import json
import pathlib


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("scope", choices=["corpus", "system"])
    parser.add_argument("source", type=pathlib.Path)
    parser.add_argument("--automatic", type=pathlib.Path,
                        help="another integration stage's automatic attribution")
    parser.add_argument("--reviews", type=pathlib.Path,
                        help="source-reviewed records for that stage")
    args = parser.parse_args()
    report = pathlib.Path(__file__).resolve().parents[1]
    automatic_path = args.automatic or report / f"census/contracts-{args.scope}-attribution.json"
    reviews_path = args.reviews or report / "census/reviewed-contract-attribution.json"
    automatic = json.loads(automatic_path.read_text())
    reviews = json.loads(reviews_path.read_text())
    selected = [item for item in reviews if item["scope"] == args.scope]
    def key(item):
        return json.dumps(item, sort_keys=True)
    assert collections.Counter(map(key, automatic["unattributed"])) == collections.Counter(
        key(item["observation"]) for item in selected), "unreviewed or stale difference"
    for item in selected:
        assert item["reason"] and item["callees"] and item["source_evidence"]
        for evidence in item["source_evidence"]:
            raw = (args.source / evidence["file"]).read_bytes()
            assert hashlib.sha256(raw).hexdigest() == evidence["sha256"]
            excerpt = "\n".join(raw.decode().splitlines()[evidence["first_line"] - 1:evidence["last_line"]])
            assert excerpt == evidence["text"]
    assert not automatic["corrections"], "a Safe correction requires separate review"
    assert not automatic["new_leak_or_violation_leaves"], "new violations require separate review"
    print(json.dumps({"scope": args.scope, "automatic_unattributed": len(automatic["unattributed"]),
                      "reviewed": len(selected), "remaining_unattributed": 0,
                      "source_identity_checked": True}, indent=2))


if __name__ == "__main__":
    main()
