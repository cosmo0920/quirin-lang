#!/usr/bin/env python3
"""Compare two preparation output directories file by file.

Card W44-QUIRIN-AUDIT-PREPARE. Every regular file under LEFT and RIGHT is
compared by bytes; a file that differs is compared again after the single
normalisation of replacing LEFT's absolute path with RIGHT's. `.git`
directories of the source clones are compared by `git rev-parse HEAD` and
`git status --porcelain` instead, since clone metadata (index timestamps,
packed refs, reflogs) records when the clone was made.

A file that still differs gets a second pass (`explain`): the named per-run
artefact or path-derived digest under which it is equal, or `null`.

Usage: compare_preparations.py LEFT RIGHT OUT.json [--no-git]
"""
import hashlib
import json
import os
import re
from pathlib import Path
import subprocess
import sys


def files(root):
    found = {}
    for directory, subdirectories, names in os.walk(root):
        subdirectories[:] = sorted(name for name in subdirectories if name != '.git')
        for name in names:
            path = Path(directory) / name
            if path.is_file() and not path.is_symlink():
                found[path.relative_to(root).as_posix()] = path
    return found


def sha(data):
    return hashlib.sha256(data).hexdigest()


def guard(path):
    """Bison's include-guard spelling of an absolute path."""
    return re.sub(rb'[^A-Z0-9]+', b'_', path.upper()).strip(b'_')


def explain(name, a, b, left_root, right_root):
    """Second pass for a file that differs after path normalisation: the
    named normalisation under which it is equal, or None. Each is a
    per-run CMake/make artefact or a digest of a path-bearing file."""
    left_bytes, right_bytes = str(left_root).encode(), str(right_root).encode()
    a = a.replace(left_bytes, right_bytes)
    if a.replace(guard(left_bytes), guard(right_bytes)) == b:
        return 'bison include guard spells the output path in upper case'
    noise = [(rb'cmTC_[0-9a-f]+', b'cmTC_*'), (rb'TryCompile-[0-9A-Za-z]+', b'TryCompile-*'),
             (rb'\((\d+\.\d+)s\)', b'(*s)')]
    x, y = a, b
    for pattern, replacement in noise:
        x, y = re.sub(pattern, replacement, x), re.sub(pattern, replacement, y)
    if x == y:
        return 'CMake try-compile names (random cmTC_*, TryCompile-*) and step timings'
    progress = lambda data: sorted(re.sub(rb'^\[ *\d+%\] ', b'', line) for line in data.splitlines())
    if progress(x) == progress(y):
        return 'same lines in another order, progress percentages aside (-j2 parallel make)'
    bare = lambda data: sorted(re.sub(rb'\[ *\d+%\] ', b'', data))
    if name.endswith('native-build.log') and bare(x) == bare(y):
        # Two -j2 jobs (bison and flex) wrote into the one shared pipe
        # mid-line: the same bytes, interleaved at another point.
        return ('same bytes, progress percentages aside, interleaved differently by -j2 jobs '
                'writing to one pipe')
    if name.endswith('CMakeRuleHashes.txt'):
        strip = lambda data: [line.split(b' ', 1)[1] for line in data.splitlines() if b' ' in line]
        if strip(a) == strip(b):
            return 'same rules; each MD5 digests rule text that contains the output path'
    if name.endswith(('build-provenance.json', 'qcore-monkey-inputs.json')):
        # Replace every digest of a path-bearing file with the digest of the
        # LEFT file after path normalisation.
        record, other = json.loads(a), json.loads(b)
        for key, relative in (('compile_database_sha256', None), ('cmake_cache_sha256', 'CMakeCache.txt'),
                              ('static_library_sha256', 'library/libcmetrics.a')):
            if key not in record:
                continue
            if relative is None:
                target = Path(record.get('compile_database',
                                         str(Path(record['build']) / 'compile_commands.json')))
            else:
                target = Path(record['build']) / relative
            source = Path(str(target).replace(str(right_root), str(left_root)))
            data = source.read_bytes().replace(left_bytes, right_bytes)
            if sha(data) != sha(target.read_bytes()):
                return None
            record[key] = other[key]
        if (json.dumps(record, indent=2) + '\n').encode() == b:
            return 'digests of path-bearing files; each equals the digest of the normalised LEFT file'
    return None


def main():
    left, right, out = (Path(argument) for argument in sys.argv[1:4])
    left_files, right_files = files(left), files(right)
    left_bytes, right_bytes = str(left).encode(), str(right).encode()
    result = {
        'left': str(left), 'right': str(right),
        'normalisation': f'replace {left} with {right} in LEFT bytes',
        'only_left': sorted(set(left_files) - set(right_files)),
        'only_right': sorted(set(right_files) - set(left_files)),
        'identical': [], 'identical_after_normalisation': [], 'different': [],
    }
    for name in sorted(set(left_files) & set(right_files)):
        a = left_files[name].read_bytes()
        b = right_files[name].read_bytes()
        if a == b:
            result['identical'].append(name)
        elif a.replace(left_bytes, right_bytes) == b:
            result['identical_after_normalisation'].append(name)
        else:
            result['different'].append({'file': name, 'left_sha256': sha(a), 'right_sha256': sha(b),
                                        'normalised_left_sha256': sha(a.replace(left_bytes, right_bytes)),
                                        'size': [len(a), len(b)],
                                        'explained_by': explain(name, a, b, left, right)})
    if '--no-git' not in sys.argv:
        clones = {}
        for name in ('fluent-bit-clang-source',):
            state = []
            for root in (left, right):
                git = lambda *args: subprocess.run(['git', '-C', str(root / name), *args], check=True,
                                                   capture_output=True, text=True).stdout
                state.append({'head': git('rev-parse', 'HEAD').strip(),
                              'status': git('status', '--porcelain', '--untracked-files=all')})
            clones[name] = {'left': state[0], 'right': state[1], 'equal': state[0] == state[1]}
        result['clones'] = clones
    result['counts'] = {key: len(result[key]) for key in
                        ('only_left', 'only_right', 'identical', 'identical_after_normalisation', 'different')}
    result['counts']['different_unexplained'] = sum(1 for entry in result['different']
                                                   if entry['explained_by'] is None)
    Path(out).write_text(json.dumps(result, indent=2) + '\n')
    # The summary the report keeps: counts, every differing file and the
    # brief's named files, without the full identical-file lists.
    key_files = {}
    for name in KEY_FILES:
        state = ('identical' if name in result['identical'] else
                 'identical_after_normalisation' if name in result['identical_after_normalisation'] else
                 next((f"different: {entry['explained_by']}" for entry in result['different']
                       if entry['file'] == name), 'absent'))
        key_files[name] = {'state': state,
                           'left_sha256': sha(left_files[name].read_bytes()) if name in left_files else None,
                           'right_sha256': sha(right_files[name].read_bytes()) if name in right_files else None}
    summary = {key: result[key] for key in ('left', 'right', 'normalisation', 'only_left', 'only_right',
                                            'different', 'counts')}
    summary['key_files'] = key_files
    summary['clones'] = result.get('clones')
    Path(str(out).removesuffix('.json') + '.summary.json').write_text(json.dumps(summary, indent=2) + '\n')
    print(json.dumps(result['counts']))


KEY_FILES = [
    'fluent-bit-clang-production/compile_commands.json',
    'fluent-bit-clang-production/cmetrics-audit.json.config',
    'fluent-bit-clang-production/CMakeCache.txt',
    'fluent-bit-clang-production/build-provenance.json',
    'fluent-bit-clang-production/configure.log',
    'fluent-bit-clang-production/native-build.log',
    'fluent-bit-clang-production/library/libcmetrics.a',
    'monkey/monkey-build/compile_commands.json',
    'monkey/monkey-build/CMakeCache.txt',
    'monkey/qcore-monkey-inputs.json',
    'monkey/configure.log',
    'monkey/monkey-source/include/monkey/mk_static_plugins.h',
    'reports/resource-diagnostics.json',
    'reports/resource-diagnostics.invocation.json',
    'reports/resource-diagnostics.contracts.json',
    'reports/resource-diagnostics.chunk-contracts.json',
    'reports/resource-diagnostics.source-effects.json',
    'reports/monkey-source.json',
]


if __name__ == '__main__':
    main()
