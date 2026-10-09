"""census.py UNITS.txt OUT.txt -- RDHTTP-HEADER-SUMMARY-GAP: the Clang JSON depth
(depth.py's scan, serde_json's count) of every unit unit_evidence.py lists with
an `unvalidated` outcome, from the pinned route's own inputs
(build/cycle19/c-gateway-wave-pin: the Fluent Bit compile database and the
Monkey one). A unit is located by its file suffix under its library's source
directory; none is reported, and every one of several matches (a file two
targets compile) is measured and reported."""
import json
import shlex
import subprocess
import sys

sys.path.insert(0, __file__.rsplit('/', 1)[0])
from depth import scan  # noqa: E402

PIN = '/home/cosmo/Documents/GitHub/qcore-c/build/cycle19/c-gateway-wave-pin'
DATABASES = {
    'fluent-bit': PIN + '/fluent-bit-clang-production/compile_commands.json',
    'monkey': PIN + '/monkey/monkey-build/compile_commands.json',
}
PREFIX = {
    'librdkafka': ('fluent-bit', 'lib/librdkafka-2.15.0/src/'),
    'chunkio': ('fluent-bit', 'lib/chunkio/src/'),
    'fluent-bit-chunk': ('fluent-bit', 'fluent-bit-clang-source/src/'),
    'monkey-libevent': ('monkey', '/'),
}


def command(entry):
    args = shlex.split(entry['command']) if 'command' in entry else list(entry['arguments'])
    kept, skip = [], False
    for arg in args[1:]:
        if skip:
            skip = False
            continue
        if arg in ('-o', '-c'):
            skip = True
            continue
        if arg.startswith('-Wa,'):
            continue
        kept.append(arg)
    return ['clang'] + kept + ['-fsyntax-only', '-Xclang', '-ast-dump=json', entry['file']]


def main():
    databases = {name: json.load(open(path)) for name, path in DATABASES.items()}
    units = set()
    for line in open(sys.argv[1]):
        parts = line.split()
        if len(parts) == 4 and parts[3] == 'unvalidated':
            units.add((parts[1], parts[2]))
    with open(sys.argv[2], 'w') as out:
        for library, file in sorted(units):
            database, prefix = PREFIX[library]
            matches = [e for e in databases[database] if e['file'].endswith(prefix + file)]
            if not matches:
                print(library, file, 'matches 0', file=out, flush=True)
            for index, entry in enumerate(matches):
                proc = subprocess.run(command(entry), cwd=entry['directory'], capture_output=True)
                depth, _ = scan(proc.stdout)
                tag = f' (match {index + 1} of {len(matches)})' if len(matches) > 1 else ''
                print(library, file + tag, 'clang-exit', proc.returncode, 'depth', depth,
                      file=out, flush=True)


if __name__ == '__main__':
    main()
