#!/bin/sh
# export.sh REV DIR: a git-archive export of REV at DIR (worktree-private),
# third_party/libevent cloned at its pinned commit, and the one driver
# adaptation every side shares on this host (run_pinned_audit.py's QCORE_CLANG
# -> /usr/lib/llvm-21/bin/clang; the host has no /usr/bin/clang).
R=/home/cosmo/Documents/GitHub/quirin-lang/.claude/worktrees/agent-a5d0fa65ce9b509f2
REV=$1; D=$2
cd "$R" || exit 2
mkdir -p "$D" || exit 2
git archive --format=tar -o "$D.tar" "$REV" || exit 3
tar -xf "$D.tar" -C "$D" && rm "$D.tar" || exit 4
rmdir "$D/third_party/libevent" 2>/dev/null
git clone -q "$R/third_party/libevent" "$D/third_party/libevent" || exit 5
git -C "$D/third_party/libevent" checkout -q d82464a277d0f42703702c4dfd9af6af38595a83 || exit 6
sed -i 's#environment\["QCORE_CLANG"\] = "/usr/bin/clang"#environment["QCORE_CLANG"] = "/usr/lib/llvm-21/bin/clang"#' "$D/scripts/run_pinned_audit.py"
grep -q '/usr/lib/llvm-21/bin/clang' "$D/scripts/run_pinned_audit.py" || exit 7
echo "exported $REV to $D"
