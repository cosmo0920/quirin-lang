#!/usr/bin/env bash
# W48-RK-LOG-CALLER-FRAME: the TIP with sibling rule 35's two arms off
# (DECAY_ARM, LITERAL_ARM false: the switch W48-BU-SINGLE-LEAF-BODIES used
# for its own tipoff side), on the same mirrored eight-home corpus route, so
# BASE -> TIP35OFF holds rules 32 and 34 only and TIP35OFF -> TIP rule 35 only.
# Run under
#   MIRROR=... flock /tmp/qcore-c-card-validation-20261022.lock off35.sh RUN TIP_REV
set -u
wt=/media/Data/GitHub/quirin-lang/.claude/worktrees/w48-rk-log-caller-frame
run=$1
tip_rev=$(git -C "$wt" rev-parse "$2")
mirror="${MIRROR:?MIRROR unset}"
mkdir "$run" || exit 2
date -u +%FT%TZ >"$run/lock-acquired"
free=$(df --output=avail -BG /media/Data | tail -1 | tr -dc 0-9)
echo "$free" >"$run/free-gib-start"
if [ "$free" -lt 40 ]; then echo "below 40 GiB: $free" >"$run/refused-disk"; exit 3; fi
export CARGO_INCREMENTAL=0 CARGO_BUILD_JOBS=6
echo "$tip_rev" >"$run/tip.rev"
snap="$run/snap-tip35off"
mkdir -p "$snap"
git -C "$wt" archive "$tip_rev" | tar -x -C "$snap" || exit 4
pin=$(git -C "$wt" ls-tree "$tip_rev" third_party/libevent | awk '{print $3}')
rmdir "$snap/third_party/libevent" 2>/dev/null || true
git clone -q "$wt/third_party/libevent" "$snap/third_party/libevent" &&
  git -C "$snap/third_party/libevent" checkout -q "$pin" || exit 4
sed -i "s/pub(crate) const DECAY_ARM: bool = true;/pub(crate) const DECAY_ARM: bool = false;/" "$snap/crates/qcore-c-gateway/src/array_member_decay.rs"
sed -i "s/pub(crate) const LITERAL_ARM: bool = true;/pub(crate) const LITERAL_ARM: bool = false;/" "$snap/crates/qcore-c-gateway/src/literal_dereference.rs"
mkdir -p "$run/pristine"
git -C "$wt" archive "$tip_rev" crates | tar -x -C "$run/pristine"
diff -r "$run/pristine/crates" "$snap/crates" >"$run/off.diff"
echo $? >"$run/off.diff.exit"
(cd "$snap" && cargo build --release -p qcore-audit --bin quirin-audit >"$run/build.log" 2>&1; echo $? >"$run/build.exit")
[ "$(cat "$run/build.exit")" = 0 ] || exit 5
sha256sum "$snap/target/release/quirin-audit" >"$run/binary.sha256"
python3 "$wt/docs/reports/w48-rk-log-caller-frame/scripts/measure_mirror.py" corpus "$run/tip35off-corpus" \
  --link-map "$wt/docs/reports/w48-sched-cleanup-production/link-map.corpus.json" --inputs-snapshot "$snap" \
  --binary-source "$tip_rev+rule35-arms-off" --jobs 4 --inputs-mirror "$mirror" >"$run/corpus.driver.log" 2>&1
echo $? >"$run/corpus.exit"
df --output=avail -BG /media/Data | tail -1 >"$run/free-gib-end"
date -u +%FT%TZ >"$run/done"
