#!/usr/bin/env python3
"""Prepare a revision-selected Clang CMake build for the configured CMetrics audit.

The compile database is CMake's unmodified output. The isolated source checkout
absorbs upstream configure-time writes. This does not assert GCC equivalence.
"""
import argparse
import hashlib
import json
from pathlib import Path
import shutil
import subprocess

ROOT = Path(__file__).resolve().parents[1]


def capture(*command):
    return subprocess.run(command, check=True, text=True, capture_output=True).stdout.strip()


def logged(command, path):
    result = subprocess.run(command, text=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
    path.write_text(result.stdout)
    if result.returncode:
        raise RuntimeError(f'command failed ({result.returncode}); see {path}')


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--fluent-bit', type=Path, default=ROOT.parent / 'fluent-bit')
    parser.add_argument('--revision', default='HEAD',
                        help='local Git revision to audit (default: HEAD); use the profile commit to reproduce historical evidence')
    parser.add_argument('--output', type=Path,
                        help='build directory (default: build/cycle19/revisions/<resolved commit>)')
    parser.add_argument('--clang', default='clang')
    parser.add_argument('--clangxx', default='clang++')
    parser.add_argument('--cmake', default='cmake')
    args = parser.parse_args()
    upstream = args.fluent_bit.resolve(strict=True)
    profile_dir = ROOT / 'contract/profiles/cmetrics-counter'
    profile = json.loads((profile_dir / 'profile.json').read_text())
    pin = capture('git', '-C', str(upstream), 'rev-parse', '--verify',
                  '--end-of-options', f'{args.revision}^{{commit}}')
    historical = pin == profile['pins']['fluent-bit']['git_commit']
    output = (args.output or ROOT / 'build/cycle19/revisions' / pin).resolve()
    source = output / 'fluent-bit-clang-source'
    build = output / 'fluent-bit-clang-production'
    if output.is_relative_to(upstream) or source == upstream:
        raise ValueError('use an output outside the upstream checkout')
    clang = shutil.which(args.clang)
    clangxx = shutil.which(args.clangxx)
    if not clang or not clangxx:
        raise ValueError('both Clang C and C++ compilers are required')
    compiler_version = capture(clang, '--version')
    if 'clang version' not in compiler_version:
        raise ValueError('C compiler does not identify itself as Clang')
    output.mkdir(parents=True, exist_ok=True)
    if not source.exists():
        # No shared object alternates: the persistent checkout is independent.
        capture('git', 'clone', '--no-hardlinks', '--no-checkout', str(upstream), str(source))
        capture('git', '-C', str(source), 'checkout', '--detach', pin)
    if not (source / '.git').is_dir():
        raise ValueError('existing source is not an independent Git checkout')
    if capture('git', '-C', str(source), 'rev-parse', 'HEAD') != pin:
        raise ValueError('existing source has a different revision; choose a fresh output')
    if capture('git', '-C', str(source), 'status', '--porcelain', '--untracked-files=no'):
        raise ValueError('existing source has tracked changes; choose a fresh output')
    if (source / '.git/objects/info/alternates').exists():
        raise ValueError('existing source borrows Git objects; use an independent clone')
    build.mkdir(parents=True, exist_ok=True)
    cache = build / 'CMakeCache.txt'
    if cache.exists():
        values = dict(line.split('=', 1) for line in cache.read_text().splitlines()
                      if '=' in line and not line.startswith(('#', '//')))
        if values.get('CMAKE_HOME_DIRECTORY:INTERNAL') != str(source):
            raise ValueError('build cache belongs to another source; choose a fresh output')
        cached_compiler = values.get('CMAKE_C_COMPILER:FILEPATH', values.get('CMAKE_C_COMPILER:STRING'))
        if not cached_compiler or Path(cached_compiler).resolve() != Path(clang).resolve():
            raise ValueError('build cache uses another compiler; choose a fresh output')
    configure = [args.cmake, '-S', str(source), '-B', str(build),
                 f'-DCMAKE_C_COMPILER={clang}', f'-DCMAKE_CXX_COMPILER={clangxx}',
                 '-DCMAKE_C_STANDARD=17', '-DCMAKE_EXPORT_COMPILE_COMMANDS=ON',
                 '-DFLB_DEV=ON', '-DFLB_EXAMPLES=ON']
    logged(configure, build / 'configure.log')
    native = [args.cmake, '--build', str(build), '--target', 'cmetrics-static', '-j2']
    logged(native, build / 'native-build.log')
    database = build / 'compile_commands.json'
    entries = json.loads(database.read_text())
    selected_source = source / 'lib/cmetrics/src/cmt_counter.c'
    selected = [entry for entry in entries if Path(entry['file']).resolve() == selected_source]
    if len(selected) != 1:
        raise ValueError('expected exactly one compiled cmt_counter.c entry')
    if capture('git', '-C', str(source), 'status', '--porcelain', '--untracked-files=no'):
        raise ValueError('native build changed tracked source files')
    config = json.loads((profile_dir / 'audit.json').read_text())
    config.update(profile_directory=str(profile_dir), checkout=str(source),
                  compile_database=str(database), report=str(build / 'cmetrics-audit.json'))
    if historical:
        config['source_contracts']['configuration'] = str(profile_dir / 'source-contracts.json')
        config['source_contracts']['source_rules'] = str(ROOT / 'contract/qcore/library/source_projection.qcore')
    else:
        config['source_revision_policy'] = 'checkout_head'
        del config['source_contracts']
    config_path = build / 'cmetrics-audit.json.config'
    config_path.write_text(json.dumps(config, indent=2) + '\n')
    provenance = dict(schema_version=1, source_revision=pin, requested_revision=args.revision,
                      source_revision_policy='profile_pin' if historical else 'checkout_head', upstream=str(upstream),
                      source=str(source), build=str(build), compiler_version=compiler_version,
                      configure_command=configure, native_build_command=native,
                      compile_database_sha256=digest(database), compile_entry=selected[0],
                      source_sha256=digest(selected_source), cmake_cache_sha256=digest(cache),
                      static_library_sha256=digest(build / 'library/libcmetrics.a'),
                      scope='Unmodified Clang GNU17 CMake command; bounded C resource/function audit, not GCC, code generation, executable-stack or whole-library safety equivalence.')
    (build / 'build-provenance.json').write_text(json.dumps(provenance, indent=2) + '\n')
    print(config_path)


if __name__ == '__main__':
    main()
