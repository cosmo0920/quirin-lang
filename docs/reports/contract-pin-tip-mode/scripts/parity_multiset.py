"""Compare the BEFORE and AFTER typed-path parity streams as multisets of
records, with Clang AST node addresses (`"0x…"` values, process memory) erased."""
import collections
import hashlib
import re
P = '/home/cosmo/Documents/GitHub/qcore-c/.claude/worktrees/agent-abb94a55c5f4a07cd/private/contract-pin-tip-mode/'
ADDRESS = re.compile(r'"0x[0-9a-f]+"')


def load(run):
    lines = open(P + run + '/parity.jsonl').read().splitlines()
    return collections.Counter(ADDRESS.sub('"0x"', line) for line in lines)


b, a = load('before'), load('after')
digest = lambda c: hashlib.sha256('\n'.join(sorted(c.elements())).encode()).hexdigest()
print('records', sum(b.values()), sum(a.values()))
print('normalised multiset sha256 before', digest(b))
print('normalised multiset sha256 after ', digest(a))
print('only before', sum((b - a).values()), 'only after', sum((a - b).values()))
