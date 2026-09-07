#!/usr/bin/env python3
"""Prepare current-checkout Monkey sources, generated headers and compile commands."""
import argparse
import hashlib
import json
from pathlib import Path
import shutil
import subprocess

REPO = Path(__file__).resolve().parents[1]


def run(*args):
    return subprocess.run(args, check=True, text=True, capture_output=True).stdout.strip()


GENERATED_SOURCE_FILES = {'include/monkey/mk_static_plugins.h', 'monkey.service'}


def file_hashes(root, names):
    return {name: hashlib.sha256((root / name).read_bytes()).hexdigest()
            for name in sorted(names)}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--fluent-bit', type=Path, required=True)
    parser.add_argument('--output', type=Path, default=REPO / 'build/cycle19')
    parser.add_argument('--cmake', default='cmake')
    args = parser.parse_args()
    upstream = args.fluent_bit.resolve(strict=True)
    out = args.output.resolve()
    revision = run('git', '-C', str(upstream), 'rev-parse', 'HEAD')
    if run('git', '-C', str(upstream), 'status', '--porcelain', '--', 'lib/monkey'):
        raise ValueError('Fluent Bit lib/monkey has local changes; source revision would be ambiguous')
    marker = out / 'qcore-monkey-inputs.json'
    if out.exists() and any(out.iterdir()) and not marker.exists():
        raise ValueError(f'{out} is not a managed QCore Monkey input directory; choose a fresh output')
    out.mkdir(parents=True, exist_ok=True)
    source = out / 'monkey-source'
    build = out / 'monkey-build'
    tracked = subprocess.run(['git', '-C', str(upstream), 'ls-files', '-z', '--', 'lib/monkey'],
                             check=True, capture_output=True, text=True).stdout
    names = [str(Path(name).relative_to('lib/monkey')) for name in tracked.split('\0') if name]
    if not names:
        raise ValueError('checkout has no tracked Monkey source files')
    source_hashes = file_hashes(upstream / 'lib/monkey', names)
    metadata = dict(schema_version=1, source_revision=revision, upstream=str(upstream),
                    source=str(source), build=str(build), source_sha256=source_hashes,
                    status='configuring')
    if marker.exists():
        previous = json.loads(marker.read_text())
        if previous.get('source_revision') != revision or {name: digest for name, digest in previous.get('source_sha256', {}).items() if name not in GENERATED_SOURCE_FILES} != source_hashes:
            raise ValueError('managed input identity changed; use a fresh output directory')
    if source.exists():
        if file_hashes(source, names) != source_hashes:
            raise ValueError('persistent source copy changed; use a fresh output directory')
    else:
        marker.write_text(json.dumps(metadata, indent=2) + '\n')
        for name in names:
            destination = source / name
            destination.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(upstream / 'lib/monkey' / name, destination)
    extras = {str(p.relative_to(source)) for p in source.rglob('*') if p.is_file()} - set(names)
    if extras - GENERATED_SOURCE_FILES:
        raise ValueError('persistent source copy has unexpected extra files')
    marker.write_text(json.dumps(metadata, indent=2) + '\n')
    command = [args.cmake, '-S', str(source), '-B', str(build),
               '-DCMAKE_BUILD_TYPE=Debug', '-DCMAKE_EXPORT_COMPILE_COMMANDS=ON',
               '-DMK_EVENT_LOOP_LIBEVENT=ON', '-DMK_EVENT_LOOP_EPOLL=OFF',
               '-DMK_EVENT_LOOP_KQUEUE=OFF', '-DMK_EVENT_LOOP_POLL=OFF',
               '-DMK_EVENT_LOOP_SELECT=OFF', '-DMK_WITHOUT_BIN=ON',
               '-DMK_WITHOUT_CONF=ON', '-DMK_TLS=OFF', '-DMK_TESTS=OFF']
    command += [f'-DMK_PLUGIN_{name}=OFF' for name in
                ('AUTH', 'CGI', 'CHEETAH', 'DIRLISTING', 'FASTCGI', 'LOGGER', 'MANDRIL')]
    result = subprocess.run(command, text=True, capture_output=True)
    (out / 'configure.log').write_text(result.stdout + result.stderr)
    if result.returncode:
        raise RuntimeError(f'CMake configuration failed; see {out / "configure.log"}')
    database = build / 'compile_commands.json'
    entries = json.loads(database.read_text())
    event = [e for e in entries if Path(e['file']).as_posix().endswith('/mk_core/mk_event.c')]
    if len(event) != 1 or '-DMK_EVENT_LOOP_LIBEVENT' not in event[0].get('command', ''):
        raise ValueError('generated database lacks the unique Libevent mk_event.c entry')
    if any(not Path(e['file']).is_file() or not Path(e['directory']).is_dir() for e in entries):
        raise ValueError('generated compile database has missing source or working directories')
    metadata.update(status='configured', cmake_command=command, compile_entries=len(entries),
                    generated_source_sha256=file_hashes(source, sorted(
                        name for name in GENERATED_SOURCE_FILES if (source / name).is_file())),
                    compile_database=str(database),
                    compile_database_sha256=hashlib.sha256(database.read_bytes()).hexdigest())
    marker.write_text(json.dumps(metadata, indent=2) + '\n')
    print(database)


if __name__ == '__main__':
    main()
