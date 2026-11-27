"""Is the card's prepared configuration the one the user's `current` run reads?

usage: compare_database.py OURS OURS_PREFIX THEIRS THEIRS_PREFIX [OURS_BUILD]

OURS_BUILD, when given, is a build directory of OURS outside OURS_PREFIX; it
is spelled as OURS_PREFIX/fluent-bit-clang-production before comparing.

Each compile database's records are rewritten with its output prefix (the
directory holding fluent-bit-clang-source and fluent-bit-clang-production)
replaced by a placeholder and keyed by (file, output). Prints whether the two
record sets name the same configurations, how many commands are identical
word for word, and the differing words grouped by pattern.
"""
import collections
import json
import shlex
import sys

ours, ours_prefix, theirs, theirs_prefix = sys.argv[1:5]
ours_build = sys.argv[5] if len(sys.argv) > 5 else None


def records(path, prefix, build=None):
    out = {}
    text = open(path).read()
    if build:
        text = text.replace(build, prefix + '/fluent-bit-clang-production')
    for record in json.loads(text):
        key = (record['file'].replace(prefix, '<OUTPUT>'), record.get('output'),
               record['directory'].replace(prefix, '<OUTPUT>'))
        out.setdefault(key, []).append(shlex.split(record['command'].replace(prefix, '<OUTPUT>')))
    return out


a = records(ours, ours_prefix, ours_build)
b = records(theirs, theirs_prefix)
identical = 0
patterns = collections.Counter()
for key, commands in a.items():
    for command, other in zip(commands, b.get(key, [])):
        if command == other:
            identical += 1
        else:
            only_ours = tuple(sorted(set(command) - set(other)))
            only_theirs = tuple(sorted(set(other) - set(command)))
            patterns[(only_ours, only_theirs)] += 1
print(json.dumps({
    'ours_records': sum(map(len, a.values())),
    'theirs_records': sum(map(len, b.values())),
    'same_configuration_keys': sorted(a) == sorted(b),
    'identical_commands': identical,
    'differing_commands': sum(patterns.values()),
    'difference_patterns': [
        {'records': n, 'only_ours': list(o), 'only_theirs': list(t)}
        for (o, t), n in patterns.most_common()
    ],
}, indent=1))
