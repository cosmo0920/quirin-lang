#!/usr/bin/env python3
"""Re-spell the pinned route inputs after the checkout moved.

Copied unchanged in behaviour from card W48-BU-SINGLE-LEAF-BODIES
(docs/reports/w48-bu-single-leaf-bodies/scripts/mirror_inputs.py, commit
1a2ffed1d on its branch) as a path-preparation reference; no other card's verdicts are reused.

Usage: mirror_inputs.py MIRROR_DIR

On 2026-10-23 the primary checkout moved to /media/Data/GitHub/quirin-lang and
/home/cosmo/Documents/GitHub/qcore-c became a symlink to it. Every pinned
compile database still spells its paths under the old prefix, while
run_pinned_audit.py resolves its inputs root to the real path and the audit
selects units by comparing path strings (pinned_route.rs `plan_library`,
current.rs `export_cross_unit_home`). No unit then matches its checkout and
every route fails before analysis (run 4, a retained failed attempt).

This script copies file content and explicitly changes compile-database path
spellings. Such spellings can affect preprocessing (for example __FILE__);
BASE and TIP must use the same prepared paths. It writes:

* corpus: a copy of `build/cycle19/c-gateway-wave-pin/{fluent-bit-clang-source,
  monkey}` and `build/cycle19/provider-packages-wave20` under MIRROR_DIR, with
  the two compile databases re-spelled so the source and Monkey trees name the
  mirror copy and every other old-prefix path names the real checkout;
* system and slice: a re-spelled copy of each prepared compile database,
  whose paths name the real checkout (the route takes the database and the
  checkout as arguments, so no tree is copied).

The only rewrite is the textual prefix substitution recorded in the manifest,
with the sha256 of every original and rewritten database. Source trees are
copied byte for byte (`cp -a`), and the manifest records a content digest of
each copy beside that of its original.
"""
import hashlib
import json
import pathlib
import subprocess
import sys

REAL = '/media/Data/GitHub/quirin-lang'
OLD = '/home/cosmo/Documents/GitHub/qcore-c'
PIN = 'build/cycle19/c-gateway-wave-pin'


def sha(path):
    return hashlib.sha256(pathlib.Path(path).read_bytes()).hexdigest()


def tree_digest(root):
    digest = hashlib.sha256()
    for path in sorted(p for p in pathlib.Path(root).rglob('*') if p.is_file() and not p.is_symlink()):
        relative = str(path.relative_to(root))
        if relative == 'build/compile_commands.json' or relative == 'monkey-build/compile_commands.json':
            continue
        digest.update(relative.encode() + b'\0' + sha(path).encode() + b'\n')
    return digest.hexdigest()


def rewrite(source, target, rules):
    text = pathlib.Path(source).read_text()
    for old, new in rules:
        text = text.replace(old, new)
    assert OLD not in text, f'{source}: old prefix left after rewrite'
    pathlib.Path(target).write_text(text)
    return dict(original=str(source), original_sha256=sha(source), rewritten=str(target),
                rewritten_sha256=sha(target), rules=[list(rule) for rule in rules])


def main():
    mirror = pathlib.Path(sys.argv[1]).resolve()
    mirror.mkdir(parents=True, exist_ok=False)
    real = pathlib.Path(REAL)
    manifest = dict(old_prefix=OLD, real_prefix=REAL, mirror=str(mirror), trees={}, databases={})
    (mirror / PIN.split('/', 2)[2]).mkdir(parents=True)
    for part in ('c-gateway-wave-pin/fluent-bit-clang-source', 'c-gateway-wave-pin/monkey',
                 'provider-packages-wave20'):
        original = real / 'build/cycle19' / part
        copy = mirror / part
        subprocess.run(['cp', '-a', str(original), str(copy)], check=True)
        manifest['trees'][part] = dict(original=str(original), copy=str(copy),
                                       original_digest=tree_digest(original), copy_digest=tree_digest(copy))
        assert manifest['trees'][part]['original_digest'] == manifest['trees'][part]['copy_digest'], part
    fb = 'c-gateway-wave-pin/fluent-bit-clang-source'
    manifest['databases']['corpus-fluent-bit'] = rewrite(
        real / 'build/cycle19' / fb / 'build/compile_commands.json', mirror / fb / 'build/compile_commands.json',
        [(f'{OLD}/build/cycle19/{fb}', str(mirror / fb)), (OLD, REAL)])
    mk = 'c-gateway-wave-pin/monkey'
    manifest['databases']['corpus-monkey'] = rewrite(
        real / 'build/cycle19' / mk / 'monkey-build/compile_commands.json',
        mirror / mk / 'monkey-build/compile_commands.json',
        [(f'{OLD}/build/cycle19/{mk}', str(mirror / mk)), (OLD, REAL)])
    for route, prepared in (('system', 'private/w45-fb/w46-external-callee-contracts/attempt-1'),
                            ('slice', 'private/w47-tail/attempt-1/prep')):
        manifest['databases'][route] = rewrite(
            real / prepared / 'fluent-bit-clang-production/compile_commands.json',
            mirror / f'{route}-compile_commands.json', [(OLD, REAL)])
    (mirror / 'manifest.json').write_text(json.dumps(manifest, indent=1) + '\n')
    print(json.dumps({k: v['rewritten_sha256'] for k, v in manifest['databases'].items()}, indent=1))
    return 0


if __name__ == '__main__':
    raise SystemExit(main())
