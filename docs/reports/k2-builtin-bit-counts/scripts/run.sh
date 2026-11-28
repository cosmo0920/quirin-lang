#!/bin/sh
# run.sh NAME TAR [system] -- K2-BUILTIN-BIT-COUNTS: extract TAR (a `git
# archive` of the measured revision) into side-NAME (with the pinned libevent
# submodule cloned in), build its release quirin-audit, then run the opt-in
# current route (FB-RETRY-OR6-ELEMENT-IDENTITY's opt-in-current.sh) into
# run-NAME, and, with `system`, the fluent-bit-system route (its system.sh)
# into sys-NAME. FB-RETRY-OR6-ELEMENT-IDENTITY's run.sh with this card's
# worktree and output directory.
W=/home/cosmo/Documents/GitHub/quirin-lang/.claude/worktrees/agent-ae71e0b9f85fdedfd
S=$W/docs/reports/fb-retry-or6-element-identity/scripts
N=$1
T=$W/private/k2bc/m/side-$N
O=$W/private/k2bc/m/run-$N
mkdir -p "$T" "$O"
if [ ! -e "$T/Cargo.toml" ]; then
  tar -x -C "$T" -f "$2" || exit 2
fi
if [ ! -e "$T/third_party/libevent/CMakeLists.txt" ]; then
  rmdir "$T/third_party/libevent" 2>/dev/null
  git clone -q "$W/third_party/libevent" "$T/third_party/libevent" || exit 2
  git -C "$T/third_party/libevent" checkout -q d82464a277d0f42703702c4dfd9af6af38595a83 || exit 2
fi
cd "$T" || exit 2
sh /home/cosmo/Documents/GitHub/quirin-lang/private/toolchain/env.sh cargo build --release -p qcore-audit --bin quirin-audit > "$O/build.log" 2>&1
echo $? > "$O/build.exit"
sha256sum "$T/target/release/quirin-audit" > "$O/binary.sha256"
sh "$S/opt-in-current.sh" "$T" "$O" "$T/target/release/quirin-audit" /home/cosmo/Documents/GitHub/quirin-lang/private/fb-046accf6
if [ "$3" = system ]; then
  sh "$S/system.sh" "$T" "$W/private/k2bc/m/sys-$N"
fi
