#!/usr/bin/env bash
# identity.sh OUT -- build identity of attempt-1: toolchain banners, the two
# source copies' file manifests (SHA-256 of every file, sorted by path, and
# the manifest's own SHA-256), and the SHA-256 of every library, harness and
# interposer. Read-only on the attempt directory except OUT.
set -u
out="$1"
root=/home/cosmo/Documents/GitHub/qcore-c/private/w47-dstop/attempt-1
mkdir -p "$out" || exit 2
{
  /usr/lib/llvm-21/bin/clang --version | head -1
  cmake --version | head -1
  ninja --version
  ldd --version | head -1
  gdb --version | head -1
  addr2line --version | head -1
  uname -srm
} > "$out/toolchain.txt" 2>&1
for tree in 90978ac6 046accf6; do
  (cd "$root/src-$tree" && find . -type f -print0 | LC_ALL=C sort -z | xargs -0 sha256sum) > "$out/manifest-$tree.sha256"
  sha256sum "$out/manifest-$tree.sha256" | cut -c1-64 > "$out/manifest-$tree.digest"
  wc -l < "$out/manifest-$tree.sha256" > "$out/manifest-$tree.count"
done
sha256sum "$root"/build-*/lib/libfluent-bit.so "$root"/harness-*/harness "$root"/harness-*/joinlog.so > "$out/binaries.sha256"
cat "$out/toolchain.txt" "$out"/manifest-*.digest "$out"/manifest-*.count "$out/binaries.sha256"
