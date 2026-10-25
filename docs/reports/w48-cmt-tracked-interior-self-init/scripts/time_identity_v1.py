#!/usr/bin/env python3
"""Replay only the exact empty-summary __TIME__ key correspondence.

Raw artifacts stay intact. The descriptor prefix is verified against the
unchanged loop_lifetime_source::read_unit and member_prepass fallback.
"""
from pathlib import Path
import argparse, datetime, gzip, hashlib, json, re, shlex, subprocess, time

p = argparse.ArgumentParser(description=__doc__)
p.add_argument('route', choices=['corpus', 'system', 'slice'])
p.add_argument('--artifacts-root', type=Path)
a = p.parse_args()
repo = Path.cwd()
report = repo / 'private/w48-cmt-tracked-interior'
out = (a.artifacts_root or report / 'comparisons') / a.route
load = lambda side: json.loads(gzip.decompress((out / side / 'resource-diagnostics.contracts.json.gz').read_bytes()))
before, after = [load(side) for side in ['base', 'tip']]
paths = [('callback_members', 'lifetime_protocols', 'evidence', 'summaries'),
         ('callback_members', 'prepass', 'lifetime_protocols', 'summaries')]
def get(x, path):
    for part in path:
        x = x[part]
    return x
maps = []
for path in paths:
    x, y = [get(doc, path) for doc in [before, after]]
    if x == y:
        maps.append(None)
        continue
    removed, added = x.keys() - y.keys(), y.keys() - x.keys()
    assert all(x[k] == y[k] for k in x.keys() & y.keys()), ('non-identity summary changed', path)
    assert len(removed) == len(added) == 1, ('expected one exact identity remap', path, removed, added)
    old, new = next(iter(removed)), next(iter(added))
    assert x[old] == y[new] == {}, 'nonempty summary cannot be attributed by this check'
    maps.append((old, new))
if all(item is None for item in maps):
    (out / 'time-identity-review.json').write_text(json.dumps({'route': a.route, 'summary_paths': paths, 'complete_summary_payloads_identical': True, 'identity_remap_needed': False}, indent=2) + '\n')
    print(json.dumps({'route': a.route, 'identity_remap_needed': False}))
    raise SystemExit(0)
assert None not in maps and len(set(maps)) == 1
old, new = maps[0]
assert old != new
inv = {side: json.loads(gzip.decompress((out / side / 'producer-invocation.json.gz').read_bytes())) for side in ['base', 'tip']}
if a.route == 'corpus':
    actual = json.loads(gzip.decompress((out / 'tip/run.json.gz').read_bytes()))['command']
    assert actual[1:3] == ['pinned', '--jobs']
    database = Path(actual[4]) / 'build/compile_commands.json'
else:
    actual = inv['tip']['command']
    database = Path(actual[actual.index('--compile-database') + 1])
raw_db = database.read_bytes()
entries = [e for e in json.loads(raw_db) if e['file'].endswith('/lib/monkey/mk_bin/monkey.c')]
assert len(entries) == 1
entry = entries[0]
words = shlex.split(entry['command']); command = [words[0]]; skip = False
for word in words[1:]:
    if skip:
        skip = False
        continue
    if word in ['-c', '-MD', '-MMD', '-MP', '-M', '-MM', '-MG']:
        continue
    if word in ['-o', '-MF', '-MT', '-MQ']:
        skip = True
        continue
    if word.startswith('-o') and len(word) > 2 or any(word.startswith(flag) and len(word) > 3 for flag in ['-MF', '-MT', '-MQ']):
        continue
    command.append(word)
command += ['-E', '-P']
assert '-fPIE' in command
assert command[-3].endswith('/lib/monkey/mk_bin/monkey.c')
r = subprocess.run(command, cwd=entry['directory'], stdout=subprocess.PIPE, stderr=subprocess.PIPE)
(out / 'time-preprocess-stderr.log').write_bytes(r.stderr)
assert r.returncode == 0, r.stderr.decode()
s = r.stdout
m = re.search(rb'(static const char MONKEY_BUILT\[\] = ")([^"]+)(" " " ")([^"]+)(";)', s)
assert m
prefix = b'AnonymousText\0Gnu17\0[]\0[]\0[]\0' + s[:m.start(4)]
suffix = s[m.end(4):]
sha = lambda value: hashlib.sha256(value).hexdigest()
found = {}; windows = {}
for side in ['base', 'tip']:
    begin, end = [datetime.datetime.fromisoformat(inv[side][key]) for key in ['started', 'ended']]
    windows[side] = [begin.isoformat(), end.isoformat()]
    for sec in range(int(begin.timestamp()), int(end.timestamp()) + 1):
        token = datetime.datetime.fromtimestamp(sec).strftime('%H:%M:%S')
        digest = sha(prefix + token.encode() + suffix)
        if digest in {old, new}:
            found[digest] = token
assert set(found) == {old, new}, (found, {old, new})
wrong_time = next(f'00:00:{sec:02d}' for sec in range(60)
                  if sha(prefix + f'00:00:{sec:02d}'.encode() + suffix) not in {old, new})
wrong_digest = sha(prefix + wrong_time.encode() + suffix)
assert wrong_digest not in {old, new}
receipt = {'meaning': 'Exactly four retained contract artifact leaves are one empty-summary identity remap duplicated in evidence/prepass. Replacing only MONKEY_BUILT __TIME__ in a fresh exact-command preprocess reproduces both complete source/configuration SHA256 keys; no summary value, decision or authority is removed.',
           'paths': paths, 'before_key': old, 'after_key': new, 'matched_time_literals': found,
           'empty_summary_payload_before': {}, 'empty_summary_payload_after': {},
           'fresh_preprocess': {'command': command, 'cwd': entry['directory'], 'exit': r.returncode,
               'compile_database': str(database), 'compile_database_sha256': sha(raw_db),
               'source_sha256': sha(Path(entry['file']).read_bytes()), 'bound_audit_command': actual,
               'bytes': len(s), 'sha256': sha(s)},
           'descriptor_prefix_hex': b'AnonymousText\0Gnu17\0[]\0[]\0[]\0'.hex(),
           'date_literal': m.group(2).decode(), 'host_timezone_names': time.tzname,
           'host_seconds_west_of_utc': time.timezone, 'measurement_windows_utc': windows,
           'wrong_time_negative_control': {'time': wrong_time, 'sha256': wrong_digest, 'rejected': True},
           'source_algorithms': ['crates/qcore-c-gateway/src/loop_lifetime_source.rs:read_unit',
                                 'crates/qcore-audit/src/member_prepass.rs:read_facts_in',
                                 'crates/qcore-audit/src/member_prepass.rs:c_unit',
                                 'crates/qcore-audit/src/compile_db.rs:compiler_options']}
receipt['source_algorithms_sha256'] = {name.split(':')[0]: sha((repo / name.split(':')[0]).read_bytes()) for name in receipt['source_algorithms']}
for algorithm in receipt['source_algorithms_sha256']:
    frozen = subprocess.check_output(['git','show',json.loads((report/'config.json').read_text())['base']+':'+algorithm],cwd=repo)
    assert frozen == (repo/algorithm).read_bytes(), ('producer algorithm changed',algorithm)
receipt['source_algorithms_equal_semantic_base'] = True
(out / 'time-source.c.gz').write_bytes(gzip.compress(s, mtime=0))
(out / 'time-identity-review.json').write_text(json.dumps(receipt, indent=2) + '\n')
print(json.dumps({'route': a.route, 'matched_keys': found, 'empty_payloads_equal': True, 'wrong_time_rejected': True}))
