#!/usr/bin/env python3
"""DOCS-COMPACT-9 inventory: rules, invariants, anchors, names, pointers and links.

Usage: inventory.py REV|WORKTREE OUT.json

REV reads every file through `git show REV:path`; WORKTREE reads the working
tree (tracked plus untracked-not-ignored Markdown). Output is one JSON object;
`compare.py` diffs two of them.
"""
import hashlib
import json
import re
import subprocess
import sys
from pathlib import Path
from urllib.parse import unquote, urlsplit

rev, out = sys.argv[1], sys.argv[2]
RULE_FILES = ['AGENTS.md', 'docs/STATUS.md'] + [
    f'docs/agents/{n}' for n in (
        'README.md', 'buffers-records.md', 'concurrency.md', 'diagnostics.md', 'effects.md',
        'evidence.md', 'gateway.md', 'io.md', 'operations.md', 'providers.md',
        'source-identity.md', 'source-routes.md', 'validation.md')]
NAME = re.compile(r'([A-Z][A-Z0-9]*(?:-[A-Z0-9]+)+)')
REPORTS = {p.name for p in Path('docs/reports').iterdir() if p.is_dir()}


def files():
    if rev == 'WORKTREE':
        listed = subprocess.check_output(['git', 'ls-files', '--cached', '--others', '--exclude-standard', '*.md']).decode().split('\n')
        return sorted({p for p in listed if p and Path(p).is_file()})
    return subprocess.check_output(['git', 'ls-tree', '-r', '--name-only', rev]).decode().split('\n')


def read(path):
    if rev == 'WORKTREE':
        p = Path(path)
        return p.read_text() if p.is_file() else None
    try:
        return subprocess.check_output(['git', 'show', f'{rev}:{path}'], stderr=subprocess.DEVNULL).decode()
    except subprocess.CalledProcessError:
        return None


def anchors(text):
    result = set(re.findall(r'<a\s+(?:id|name)="([^"]+)"', text))
    counts = {}
    fenced = False
    for line in text.splitlines():
        if line.startswith(('```', '~~~')):
            fenced = not fenced
        if fenced or not re.match(r'^#{1,6} ', line):
            continue
        title = re.sub(r'^#+ ', '', line)
        slug = re.sub(r'[^\w\s-]', '', title.lower()).replace(' ', '-')
        n = counts.get(slug, 0)
        result.add(slug + (f'-{n}' if n else ''))
        counts[slug] = n + 1
    return result


def rule_blocks(text):
    """Split at each constraint anchor; a block runs to the next anchor or H2."""
    blocks = {}
    parts = re.split(r'(<a id="constraint-[^"]+"></a>)', text)
    for i in range(1, len(parts), 2):
        rid = re.search(r'constraint-([^"]+)', parts[i])[1]
        body = parts[i + 1]
        body = re.split(r'\n(?=## |### Diagnostics namespaces|<!-- preserved:diagnostic-namespaces|<a id="(?!constraint-))', body)[0]
        blocks[rid] = body
    return blocks


def pointers(text):
    found = set()
    for tok in re.findall(r'`([a-z0-9][a-z0-9-]*[a-z0-9])`', text):
        if tok in REPORTS:
            found.add(tok)
    for tok in re.findall(r'reports/([a-z0-9][a-z0-9-]*[a-z0-9])', text):
        if tok in REPORTS:
            found.add(tok)
    return found


result = {'rev': rev, 'rules': {}, 'invariants': {}, 'files': {}, 'anchors': {}, 'links': []}
all_md = [f for f in files() if f.endswith('.md')]
texts = {f: read(f) for f in all_md}
texts = {f: t for f, t in texts.items() if t is not None}
for f in RULE_FILES:
    t = texts.get(f)
    if t is None:
        continue
    result['files'][f] = {'bytes': len(t.encode()), 'names': sorted(set(NAME.findall(t))),
                          'pointers': sorted(pointers(t))}
    for rid, body in rule_blocks(t).items():
        label = re.search(r'^\s*(\d+[a-z]?)\. ', body, re.M)
        result['rules'][rid] = {'file': f, 'label': label[1] if label else None,
                                'bytes': len(body.encode()),
                                'sha256': hashlib.sha256(body.encode()).hexdigest(),
                                'names': sorted(set(NAME.findall(body))),
                                'pointers': sorted(pointers(body))}
agents = texts['AGENTS.md']
inv = agents.split('## Hard invariants', 1)[1].split('\n## ', 1)[0]
for m in re.finditer(r'^(\d+)\. (.*?)(?=^\d+\. |\Z)', inv, re.M | re.S):
    result['invariants'][m[1]] = hashlib.sha256(m[0].encode()).hexdigest()
for f, t in texts.items():
    result['anchors'][f] = sorted(anchors(t))
for f, t in texts.items():
    if '/structure-checkpoints/' in f:
        continue
    unfenced, fenced = [], False
    for line in t.split('\n'):
        if line.startswith(('```', '~~~')):
            fenced = not fenced
            continue
        if not fenced:
            unfenced.append(line)
    for dest in re.findall(r'\]\(([^)\s]+)\)', '\n'.join(unfenced)):
        url = urlsplit(dest)
        if url.scheme or url.netloc:
            continue
        target = (Path(f).parent / unquote(url.path)).as_posix() if url.path else f
        target = str(Path(target)) if not target.startswith('..') else target
        # normalise ../
        norm = Path(f).parent.joinpath(unquote(url.path)) if url.path else Path(f)
        parts = []
        for part in norm.as_posix().split('/'):
            if part == '..':
                if parts:
                    parts.pop()
            elif part and part != '.':
                parts.append(part)
        target = '/'.join(parts)
        if target.endswith('.md'):
            ok_path = target in texts
        else:
            ok_path = Path(target).exists()
        ok_anchor = True
        if url.fragment and target.endswith('.md') and ok_path:
            ok_anchor = unquote(url.fragment) in anchors(texts[target])
        result['links'].append([f, dest, target, bool(ok_path), bool(ok_anchor)])
Path(out).write_text(json.dumps(result, indent=0, sort_keys=True))
bad = [l for l in result['links'] if not (l[3] and l[4])]
print(f"{rev}: {len(result['rules'])} rule blocks, {len(result['invariants'])} invariants, "
      f"{len(result['links'])} local links, {len(bad)} unresolved")
