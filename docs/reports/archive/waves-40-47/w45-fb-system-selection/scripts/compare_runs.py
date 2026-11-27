import hashlib
import pathlib
import sys

root = pathlib.Path(sys.argv[1])
left, right = root / sys.argv[2], root / sys.argv[3]
for name in sorted({p.name for p in left.iterdir()} | {p.name for p in right.iterdir()}):
    a, b = left / name, right / name
    if not a.exists() or not b.exists():
        print(f"{name}: only in {'left' if a.exists() else 'right'}")
        continue
    x, y = a.read_bytes(), b.read_bytes()
    if name == 'console.log':
        x = b"\n".join(l for l in x.split(b"\n") if not l.startswith(b"preparation:"))
        y = b"\n".join(l for l in y.split(b"\n") if not l.startswith(b"preparation:"))
        name += " (without preparation: timing line)"
    print(f"{name}: {'identical' if x == y else 'DIFFERS'} {hashlib.sha256(a.read_bytes()).hexdigest()[:16]} {hashlib.sha256(b.read_bytes()).hexdigest()[:16]}")
