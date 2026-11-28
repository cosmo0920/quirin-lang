#!/bin/sh
# run.sh NAME TAR [census] -- extract TAR (a `git archive` of the measured
# revision) into side-NAME, build its release quirin-audit, run the opt-in
# current route into run-NAME. RETURN-FAMILY-UNTRACKED-MEMBER-CAST's run.sh
# with this card's worktree; with a third argument `census` the binary is built
# with `qcore-c-gateway/indirect-census` and QCORE_MARK_CENSUS names
# run-NAME/mark-census.jsonl (card SETTLED-LEAK-MARK-WITHOUT-UNKNOWN-LEAF;
# reused by SETTLED-LEAK-MARK-LEAFLESS with this worktree).
W=/home/cosmo/Documents/GitHub/quirin-lang/.claude/worktrees/agent-a76c14c1af0304385
N=$1
T=$W/private/slf/side-$N
O=$W/private/slf/run-$N
mkdir -p "$T" "$O"
tar -x -C "$T" -f "$2" || exit 2
# The pinned libevent submodule, from `git -C third_party/libevent archive HEAD`.
mkdir -p "$T/third_party/libevent"
tar -x -C "$T/third_party/libevent" -f "$W/private/slf/libevent.tar" || exit 2
cd "$T" || exit 2
FEATURES=
if [ "$3" = census ]; then
  FEATURES="--features qcore-c-gateway/indirect-census"
  QCORE_MARK_CENSUS="$O/mark-census.jsonl"
  export QCORE_MARK_CENSUS
fi
sh /home/cosmo/Documents/GitHub/quirin-lang/private/toolchain/env.sh cargo build --release -p qcore-audit --bin quirin-audit $FEATURES > "$O/build.log" 2>&1
echo $? > "$O/build.exit"
sha256sum "$T/target/release/quirin-audit" > "$O/binary.sha256"
sh "$W/docs/reports/settled-leak-mark-leafless/scripts/opt-in-current.sh" "$T" "$O" "$T/target/release/quirin-audit" /home/cosmo/Documents/GitHub/quirin-lang/private/fb-046accf6
