#!/bin/sh
# side.sh NAME TAR -- extract TAR (a `git archive` of the measured revision)
# into private/m/side-NAME (with the pinned libevent submodule cloned in),
# build its release quirin-audit, and run the fluent-bit-system route
# (system.sh) into private/m/sys-NAME.
W=/home/cosmo/Documents/GitHub/quirin-lang/.claude/worktrees/agent-abd604af97d27e262
N=$1
T=$W/private/m/side-$N
O=$W/private/m/sys-$N
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
sh "$W/docs/reports/leri-held-exit-class-precondition/scripts/system.sh" "$T" "$O"
