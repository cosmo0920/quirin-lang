"""Measurement-harness patch, applied identically to both side exports:
run_pinned_audit.py hard-codes QCORE_CLANG=/usr/bin/clang, which this host
lacks; honour the environment's QCORE_CLANG (the measure route sets it)."""
import sys
old = '    environment["QCORE_CLANG"] = "/usr/bin/clang"\n'
new = '    environment["QCORE_CLANG"] = os.environ.get("QCORE_CLANG", "/usr/bin/clang")\n'
for path in sys.argv[1:]:
    text = open(path).read()
    assert text.count(old) == 1, path
    open(path, 'w').write(text.replace(old, new))
    print('patched', path)
