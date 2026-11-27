#!/usr/bin/env python3
"""Whether a run's artifact equals an archived one once the checkout root is
aliased and every lowercase hex run of 24 or more characters (the path-bound
digests every W45 card discloses in the source-effects sidecar) is masked.

Usage: mask_digests.py RUN_FILE ARCHIVED_FILE.gz ROOT=CANONICAL

A disclosure beside `baseline_identity.py`, never a substitute for it.
"""
import gzip
import json
import re
import sys

run = open(sys.argv[1], 'rb').read()
base = gzip.decompress(open(sys.argv[2], 'rb').read())
root, canonical = sys.argv[3].split('=', 1)
run = run.replace(root.encode(), canonical.encode())
pattern = re.compile(rb'[0-9a-f]{24,}')
digests = set(pattern.findall(run)) ^ set(pattern.findall(base))
print(json.dumps({
    "identical_after_masking_digests": pattern.sub(b'D', run) == pattern.sub(b'D', base),
    "distinct_differing_digests": len(digests),
}))
