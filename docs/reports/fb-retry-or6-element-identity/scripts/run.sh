#!/bin/sh
# run.sh NAME TAR -- extract TAR (a `git archive` of the measured revision)
# into side-NAME (with the pinned libevent submodule cloned in), build its
# release quirin-audit, run the opt-in current route into run-NAME.
# RETURN-FAMILY-UNTRACKED-MEMBER-CAST's run.sh with this card's worktree.
W=/home/cosmo/Documents/GitHub/quirin-lang/.claude/worktrees/agent-a89285394b59a4534
N=$1
T=$W/private/m/side-$N
O=$W/private/m/run-$N
mkdir -p "$T" "$O"
tar -x -C "$T" -f "$2" || exit 2
if [ ! -e "$T/third_party/libevent/CMakeLists.txt" ]; then
  rmdir "$T/third_party/libevent" 2>/dev/null
  git clone -q "$W/third_party/libevent" "$T/third_party/libevent" || exit 2
  git -C "$T/third_party/libevent" checkout -q d82464a277d0f42703702c4dfd9af6af38595a83 || exit 2
fi
cd "$T" || exit 2
sh /home/cosmo/Documents/GitHub/quirin-lang/private/toolchain/env.sh cargo build --release -p qcore-audit --bin quirin-audit > "$O/build.log" 2>&1
echo $? > "$O/build.exit"
sha256sum "$T/target/release/quirin-audit" > "$O/binary.sha256"
sh "$W/docs/reports/fb-retry-or6-element-identity/scripts/opt-in-current.sh" "$T" "$O" "$T/target/release/quirin-audit" /home/cosmo/Documents/GitHub/quirin-lang/private/fb-046accf6
