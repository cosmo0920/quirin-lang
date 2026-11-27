#!/usr/bin/env python3
"""Copied from docs/reports/rule35-pilot/scripts/substitution_ids.py for
RULE34-PILOT, with one change: a 40-hex token (the analyzer revision, which
differs because BASE is the published `baseline:80c8aea98` and TIP is built
from the card's revision) is a substitutable token too, blanked as `R`.

RULE35-PILOT: a stricter form of GATEWAY-RULE-API's `substitution.py` for
the corpus artifacts that differ outside 64-hex tokens.

The working-directory-dependent unit build identity (finding
MEASURE-CORPUS-SIDE-CWD-IN-BUILD-IDENTITY) also moves the 24-hex obligation
ids `resource_contracts:<24 hex>` derived from it, and evidence kept sorted by
those ids can change order. For each BEFORE/AFTER pair this script:

1. collects every token that is a 64-hex digest or a 24-hex
   `resource_contracts:` id by walking the two parsed documents in parallel:
   list elements are matched by their text with every such token blanked,
   and elements sharing a blanked text are matched afterwards by the token
   map the rest of the document establishes (an element with no compatible
   partner, or a group that stays ambiguous, fails);
2. requires the before->after token map and its inverse to be functions (one
   to one);
3. applies the map to BEFORE's parsed document and requires equality with
   AFTER's, every list compared as a multiset.

Prints one JSON line per pair; exit 1 when any pair fails.
Usage: substitution_ids.py BEFORE AFTER [BEFORE AFTER ...]
"""
import hashlib
import json
import re
import sys

TOKEN = re.compile(r"(?<![0-9a-f])[0-9a-f]{64}(?![0-9a-f])|resource_contracts:[0-9a-f]{24}(?![0-9a-f])"
                   r"|(?<![0-9a-f])[0-9a-f]{40}(?![0-9a-f])")


def blank(text):
    def one(match):
        token = match.group(0)
        if token.startswith("r"):
            return "resource_contracts:H"
        return "R" if len(token) == 40 else "H"
    return TOKEN.sub(one, text)


def key(value):
    return blank(json.dumps(value, sort_keys=True))


def normal(value):
    """Lists sorted by their blanked text, recursively."""
    if isinstance(value, dict):
        return {k: normal(v) for k, v in value.items()}
    if isinstance(value, list):
        return sorted((normal(v) for v in value), key=key)
    return value


class NotOneToOne(Exception):
    pass


def pair_tokens(before, after, forward, backward, deferred):
    """Walk two documents in parallel, recording token pairs. Within a list,
    elements are matched by their text with every token blanked; elements
    whose blanked text is shared by several elements are deferred, to be
    matched by the token map the rest of the document establishes."""
    if isinstance(before, dict) and isinstance(after, dict):
        bk = sorted(before, key=blank)
        ak = sorted(after, key=blank)
        if [blank(k) for k in bk] != [blank(k) for k in ak]:
            raise NotOneToOne("object keys differ beyond tokens")
        for kb, ka in zip(bk, ak):
            pair_text(kb, ka, forward, backward)
            pair_tokens(before[kb], after[ka], forward, backward, deferred)
    elif isinstance(before, list) and isinstance(after, list):
        if len(before) != len(after):
            raise NotOneToOne("list lengths differ")
        groups = {}
        for side, items in ((0, before), (1, after)):
            for item in items:
                groups.setdefault(key(item), ([], []))[side].append(item)
        for blanked, (bs, as_) in groups.items():
            if len(bs) != len(as_):
                raise NotOneToOne(f"list element counts differ: {blanked[:80]}")
            if len(bs) == 1:
                pair_tokens(bs[0], as_[0], forward, backward, deferred)
            else:
                deferred.append((bs, as_))
    elif isinstance(before, str) and isinstance(after, str):
        pair_text(before, after, forward, backward)
    elif before != after:
        raise NotOneToOne(f"values differ: {before!r} != {after!r}")


def compatible(before, after, forward, backward):
    """Whether pairing `before` with `after` agrees with the map so far."""
    tb = TOKEN.findall(json.dumps(before, sort_keys=True))
    ta = TOKEN.findall(json.dumps(after, sort_keys=True))
    if len(tb) != len(ta):
        return False
    return all(forward.get(b, a) == a and backward.get(a, b) == b for b, a in zip(tb, ta))


def resolve(deferred, forward, backward):
    """Match deferred groups by the token map, until no group moves."""
    pending = list(deferred)
    while pending:
        progress = False
        remaining = []
        for bs, as_ in pending:
            bs, as_ = list(bs), list(as_)
            moved = True
            while moved and bs:
                moved = False
                for b in list(bs):
                    candidates = [a for a in as_ if compatible(b, a, forward, backward)]
                    if len(candidates) == 1:
                        inner = []
                        pair_tokens(b, candidates[0], forward, backward, inner)
                        remaining.extend(inner)
                        bs.remove(b)
                        as_.remove(candidates[0])
                        moved = progress = True
                    elif not candidates:
                        raise NotOneToOne("a deferred element matches nothing under the map")
            if bs:
                remaining.append((bs, as_))
        pending = remaining
        if not progress and pending:
            raise NotOneToOne(f"{len(pending)} list groups stay ambiguous under the map")


def pair_text(before, after, forward, backward):
    tb, ta = TOKEN.findall(before), TOKEN.findall(after)
    if len(tb) != len(ta) or TOKEN.split(before) != TOKEN.split(after):
        raise NotOneToOne(f"text differs outside tokens: {before[:80]!r}")
    for b, a in zip(tb, ta):
        if forward.setdefault(b, a) != a or backward.setdefault(a, b) != b:
            raise NotOneToOne(f"token {b[:20]} maps to two values")


def exact(value):
    """Lists sorted by their exact text, recursively (a multiset compare)."""
    if isinstance(value, dict):
        return {k: exact(v) for k, v in value.items()}
    if isinstance(value, list):
        return sorted((exact(v) for v in value), key=lambda v: json.dumps(v, sort_keys=True))
    return value


def substitute(value, forward):
    if isinstance(value, dict):
        return {substitute(k, forward): substitute(v, forward) for k, v in value.items()}
    if isinstance(value, list):
        return [substitute(v, forward) for v in value]
    if isinstance(value, str):
        return TOKEN.sub(lambda m: forward.get(m.group(0), m.group(0)), value)
    return value


def parse(path):
    text = open(path, encoding="utf-8").read()
    if path.endswith(".jsonl"):
        return [json.loads(line) for line in text.splitlines() if line.strip()]
    return json.loads(text)


def check(before_path, after_path):
    raw_b, raw_a = open(before_path, "rb").read(), open(after_path, "rb").read()
    record = {"before": before_path, "after": after_path,
              "before_sha256": hashlib.sha256(raw_b).hexdigest(),
              "after_sha256": hashlib.sha256(raw_a).hexdigest()}
    if raw_b == raw_a:
        record.update(result="byte-identical", substituted=0)
        return record, True
    before, after = normal(parse(before_path)), normal(parse(after_path))
    forward, backward = {}, {}
    try:
        deferred = []
        pair_tokens(before, after, forward, backward, deferred)
        resolve(deferred, forward, backward)
    except NotOneToOne as error:
        record.update(result="not a one-to-one token substitution", why=str(error))
        return record, False
    ok = exact(substitute(before, forward)) == exact(after)
    record.update(result="equal under one-to-one token substitution, lists as multisets"
                  if ok else "differs after substitution",
                  tokens=len(forward), substituted=sum(1 for b, a in forward.items() if b != a),
                  digests_64=sum(1 for b in forward if len(b) == 64),
                  revisions_40=sum(1 for b in forward if len(b) == 40),
                  contract_ids_24=sum(1 for b in forward if b.startswith("r")))
    return record, ok


def main(arguments):
    if len(arguments) < 2 or len(arguments) % 2:
        print(__doc__, file=sys.stderr)
        return 2
    ok = True
    for before, after in zip(arguments[::2], arguments[1::2]):
        record, passed = check(before, after)
        print(json.dumps(record, sort_keys=True))
        ok = ok and passed
    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
