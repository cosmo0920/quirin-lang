#!/bin/sh
# Non-vacuity of the ninth arm: do not remove the join edge, and TSan must
# report nothing. Restores the file with its mtime advanced.
set -u
root=$(pwd)
out="$root/$1"
mkdir -p "$out"
TASK=compiler/tests/task_route_tsan.rs
name=nojoin

cp "$root/$TASK" "$out/$name.orig"
sed 's|let unjoined = c.replace(|let unjoined = c.clone(); let _ = (|' "$out/$name.orig" > "$root/$TASK"
if cmp -s "$out/$name.orig" "$root/$TASK"; then
    echo "MUTATION-DID-NOT-APPLY" > "$out/$name.verdict"
else
    touch "$root/$TASK"
    cargo test -p qcore-compiler --test task_route_tsan removing_the_join_edge \
        > "$out/$name.log" 2>&1
    echo "$?" > "$out/$name.rc"
    if [ "$(cat "$out/$name.rc")" = 0 ]; then
        echo "VACUOUS-ARM-STILL-GREEN" > "$out/$name.verdict"
    else
        echo "RED-AS-REQUIRED" > "$out/$name.verdict"
    fi
fi
cp "$out/$name.orig" "$root/$TASK"
touch "$root/$TASK"
cat "$out/$name.verdict"
