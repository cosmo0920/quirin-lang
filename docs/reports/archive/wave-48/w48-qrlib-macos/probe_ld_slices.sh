#!/bin/sh
# Card W48-QRLIB-MACOS: which `.tbd` slice Apple `ld` reads for an arm64 link.
# Usage (on a Mac, from the repository root):
#   sh docs/reports/w48-qrlib-macos/probe_ld_slices.sh OUT_DIR
# Each case links `int main(void){return qr_probe(1);}` for arm64 against a
# one-document stub and prints the link's exit and the binding `nm -m` shows.
# `qr_probe` is not a builtin, so no call is folded or inlined away.
set -u
out=${1:?usage: probe_ld_slices.sh OUT_DIR}
mkdir -p "$out"
printf 'int qr_probe(int);int main(void){return qr_probe(1);}\n' > "$out/q.c"
stub() { # NAME TARGETS EXPORTED_FOR
    cat > "$out/$1.tbd" <<STUB
--- !tapi-tbd
tbd-version:     4
targets:         [ $2 ]
install-name:    '/usr/lib/libSystem.B.dylib'
exports:
  - targets:         [ $3 ]
    symbols:         [ _qr_probe ]
...
STUB
}
stub only-arm64e 'arm64e-macos' 'arm64e-macos'
stub only-arm64 'arm64-macos' 'arm64-macos'
stub only-x86_64 'x86_64-macos' 'x86_64-macos'
stub both-exported-arm64e 'arm64-macos, arm64e-macos' 'arm64e-macos'
for case in only-arm64e only-arm64 only-x86_64 both-exported-arm64e; do
    clang -arch arm64 -nostdlib -Wl,-e,_main "$out/q.c" "$out/$case.tbd" \
        -o "$out/$case" > "$out/$case.log" 2>&1
    status=$?
    binding=$(nm -m "$out/$case" 2>/dev/null | grep qr_probe)
    echo "$case exit=$status ${binding:-no binary}"
done
