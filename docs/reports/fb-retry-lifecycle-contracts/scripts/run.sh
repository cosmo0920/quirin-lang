#!/bin/sh
# run.sh NAME TAR -- FB-RETRY-LIFECYCLE-CONTRACTS: extract TAR (a `git archive`
# of the measured revision) into side-NAME (the pinned libevent submodule
# cloned in), build its release quirin-audit, then run the fluent-bit-system
# route into sys-NAME and the opt-in current --monkey-linked route into
# run-NAME. FB-RETRY-OR6-ELEMENT-IDENTITY's run.sh with this card's worktree.
W=${W:-$(cd "$(dirname "$0")/../../../.." && pwd)}
S=$W/docs/reports/fb-retry-lifecycle-contracts/scripts
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
sh "$S/system.sh" "$T" "$W/private/m/sys-$N"
sh "$S/opt-in-current.sh" "$T" "$O" "$T/target/release/quirin-audit" /home/cosmo/Documents/GitHub/quirin-lang/private/fb-046accf6
