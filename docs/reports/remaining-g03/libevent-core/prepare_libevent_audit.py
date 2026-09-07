#!/usr/bin/env python3
"""Prepare the pinned `libevent-core` second-source-set audit inputs.

The audited checkout is the repository's own pinned `third_party/libevent`
submodule. CMake is configured with Clang so that Libevent's generated
`event2/event-config.h` and `evconfig-private.h` exist, and `event_core_static`
is built natively from the same tree.

CMake's unmodified compilation database is retained verbatim as
`libevent-cmake-database.json`. It is NOT the audit input: Libevent's CMake
files append warning and code-generation options that the configured gateway
cannot preserve (`docs/reports/remaining-g03/README.md`). The audit database
written here is a hand-written database in the supported schema; every command
it contains is executed with the same Clang before it is published, so the
recorded commands are real compilations, not a paper projection.

This prepares inputs. It does not certify Libevent.
"""
import argparse
import hashlib
import json
from pathlib import Path
import shutil
import subprocess

ROOT = Path(__file__).resolve().parents[1]

# Preprocessing-relevant options are carried over verbatim from the CMake
# command. Everything else is either transported explicitly (below) or dropped
# and recorded in the provenance file.
CARRIED_PREFIXES = ('-I', '-D', '-isystem', '-iquote', '-U')
# The four frontend flags the gateway can transport into preprocessing, syntax
# and typed AST. Retained explicitly so the audit exercises that transport.
TRANSPORTED_FLAGS = ['-Wall', '-g', '-fPIC']


def capture(*command):
    return subprocess.run(command, check=True, text=True, capture_output=True).stdout.strip()


def logged(command, path):
    result = subprocess.run(command, text=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
    path.write_text(result.stdout)
    if result.returncode:
        raise RuntimeError(f'command failed ({result.returncode}); see {path}')


def digest(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def selected_files(library):
    files = []
    for root in library['source_roots']:
        selection = root['selection']
        if not isinstance(selection, dict) or 'exact' not in selection:
            raise ValueError('this source set selects exact files only')
        files.append(selection['exact']['file'])
    if len(set(files)) != len(files):
        raise ValueError('duplicate selected source file')
    return files


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output', type=Path, default=ROOT / 'build/cycle19')
    parser.add_argument('--clang', default='clang')
    parser.add_argument('--clangxx', default='clang++')
    parser.add_argument('--cmake', default='cmake')
    args = parser.parse_args()

    profile_dir = ROOT / 'contract/profiles/libevent-core'
    profile = json.loads((profile_dir / 'profile.json').read_text())
    pin = profile['pins']['libevent']['git_commit']
    library = profile['libraries']['libevent-core']
    files = selected_files(library)

    checkout = (ROOT / 'third_party/libevent').resolve(strict=True)
    if capture('git', '-C', str(checkout), 'rev-parse', 'HEAD') != pin:
        raise ValueError('third_party/libevent HEAD does not match the configured source pin')
    if capture('git', '-C', str(checkout), 'status', '--porcelain', '--untracked-files=no'):
        raise ValueError('third_party/libevent has tracked changes; the pin is not clean')
    for audited in library['audited_files']:
        if digest(checkout / audited['path']) != audited['sha256']:
            raise ValueError(f"audited file {audited['path']} does not match its declared digest")

    clang = shutil.which(args.clang)
    clangxx = shutil.which(args.clangxx)
    if not clang or not clangxx:
        raise ValueError('both Clang C and C++ compilers are required')
    compiler_version = capture(clang, '--version')
    if 'clang version' not in compiler_version:
        raise ValueError('C compiler does not identify itself as Clang')

    build = args.output.resolve() / 'libevent-clang-build'
    build.mkdir(parents=True, exist_ok=True)
    cache = build / 'CMakeCache.txt'
    if cache.exists():
        values = dict(line.split('=', 1) for line in cache.read_text().splitlines()
                      if '=' in line and not line.startswith(('#', '//')))
        if values.get('CMAKE_HOME_DIRECTORY:INTERNAL') != str(checkout):
            raise ValueError('build cache belongs to another source; choose a fresh output')
        cached = values.get('CMAKE_C_COMPILER:FILEPATH', values.get('CMAKE_C_COMPILER:STRING'))
        if not cached or Path(cached).resolve() != Path(clang).resolve():
            raise ValueError('build cache uses another compiler; choose a fresh output')

    configure = [args.cmake, '-S', str(checkout), '-B', str(build),
                 f'-DCMAKE_C_COMPILER={clang}', f'-DCMAKE_CXX_COMPILER={clangxx}',
                 '-DCMAKE_C_STANDARD=17', '-DCMAKE_EXPORT_COMPILE_COMMANDS=ON',
                 '-DEVENT__LIBRARY_TYPE=STATIC', '-DEVENT__DISABLE_OPENSSL=ON',
                 '-DEVENT__DISABLE_MBEDTLS=ON', '-DEVENT__DISABLE_TESTS=ON',
                 '-DEVENT__DISABLE_REGRESS=ON', '-DEVENT__DISABLE_SAMPLES=ON',
                 '-DEVENT__DISABLE_BENCHMARK=ON']
    logged(configure, build / 'configure.log')
    native = [args.cmake, '--build', str(build), '--target', 'event_core_static', '-j2']
    logged(native, build / 'native-build.log')
    if capture('git', '-C', str(checkout), 'status', '--porcelain', '--untracked-files=no'):
        raise ValueError('the native build changed tracked source files')

    cmake_database = build / 'compile_commands.json'
    retained = build / 'libevent-cmake-database.json'
    retained.write_bytes(cmake_database.read_bytes())
    entries = json.loads(cmake_database.read_text())

    objects = build / 'qcore-audit-objects'
    objects.mkdir(exist_ok=True)
    records = []
    dropped = {}
    for name in files:
        source = checkout / name
        matching = [entry for entry in entries if Path(entry['file']).resolve() == source]
        if not matching:
            raise ValueError(f'{name} is not compiled by this CMake configuration')
        commands = {entry.get('command') or ' '.join(entry['arguments']) for entry in matching}
        preprocessing, ignored = [], []
        for command in sorted(commands):
            words = command.split()
            cursor = 1
            while cursor < len(words):
                word = words[cursor]
                if word in ('-o', '-c') or Path(word).resolve() == source:
                    cursor += 2 if word == '-o' else 1
                    continue
                if word.startswith(CARRIED_PREFIXES):
                    if word in ('-I', '-D', '-isystem', '-iquote', '-U'):
                        cursor += 1
                        word = f'{word}{words[cursor]}'
                    if word not in preprocessing:
                        preprocessing.append(word)
                elif word not in ignored:
                    ignored.append(word)
                cursor += 1
        dropped[name] = sorted(flag for flag in ignored if flag not in TRANSPORTED_FLAGS)
        arguments = [clang, '-std=gnu17', *TRANSPORTED_FLAGS, *preprocessing,
                     '-c', str(source), '-o', str(objects / f'{Path(name).stem}.o')]
        # A published record must be a command that really compiles this file.
        logged(arguments, build / f'qcore-audit-{Path(name).stem}.compile.log')
        records.append({'directory': str(build), 'file': str(source), 'arguments': arguments})

    database = build / 'qcore-audit-compile-commands.json'
    database.write_text(json.dumps(records, indent=2) + '\n')

    config = json.loads((profile_dir / 'audit.json').read_text())
    config.update(profile_directory=str(profile_dir), checkout=str(checkout),
                  compile_database=str(database),
                  report=str(build / 'libevent-core-audit.json'))
    config_path = build / 'libevent-core-audit.json.config'
    config_path.write_text(json.dumps(config, indent=2) + '\n')

    provenance = dict(
        schema_version=1, source_set='libevent-core', source_revision=pin,
        checkout=str(checkout), build=str(build), compiler_version=compiler_version,
        configure_command=configure, native_build_command=native,
        selected_files=files,
        cmake_database_sha256=digest(retained), cmake_database_entries=len(entries),
        audit_database_sha256=digest(database),
        dropped_cmake_options=dropped,
        static_library_sha256=digest(build / 'lib/libevent_core.a'),
        source_sha256={name: digest(checkout / name) for name in files},
        scope=('Hand-written compilation database in the supported schema, executed with the '
               'configured Clang. Preprocessing options are carried over from the unmodified '
               'CMake command; warning and code-generation options the gateway cannot preserve '
               'are dropped and listed. Bounded C resource projection only: not GCC equivalence, '
               'not code generation, not whole-library safety.'))
    (build / 'libevent-build-provenance.json').write_text(json.dumps(provenance, indent=2) + '\n')
    print(config_path)


if __name__ == '__main__':
    main()
