import os, re, glob, hashlib
root = '/media/Data/GitHub/quirin-lang/build/cycle19/clang'
for f in sorted(glob.glob(root + '/*/**/mk_server/mk_lib.c', recursive=True)):
    s = open(f, encoding='utf-8', errors='replace').read()
    m = re.search(r'int mk_vhost_set\(mk_ctx_t \*ctx, int vid, \.\.\.\)\n\{.*?\n\}\n', s, re.S)
    if not m:
        print(f, 'NO DEF'); continue
    body = m.group(0)
    fixed = 'va_end(va);\n                return -1;' in body or re.search(r'if \(!value\) \{\s*/\* Wrong parameter \*/\s*va_end', body) is not None
    print(f.replace(root, ''), 'fixed' if fixed else 'DEFECT', hashlib.sha256(body.encode()).hexdigest()[:12])
