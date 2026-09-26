#!/bin/sh
# Decompose each corpus checkpoint to YAML and TOML, recompose, and compare bytes;
# then show that an edited text is refused. Writes under MEASURE_DIR/roundtrip.
cd /home/cosmo/Documents/GitHub/qcore-c/.claude/worktrees/agent-a99e1ae248e9aa1bf || exit 2
M=${1:?usage: roundtrip.sh MEASURE_DIR (holding qo-j16/)}
O=$M/roundtrip
mkdir -p $O
Q=target/release/quirin-audit
for g in cmetrics ctraces librdkafka chunkio fluent-bit-chunk monkey-libevent; do
  for f in yaml toml; do
    $Q checkpoint decompose $M/qo-j16/$g.qo --format $f --output $O/$g.$f; d=$?
    $Q checkpoint recompose $O/$g.$f --format $f --output $O/$g-$f.qo; r=$?
    if cmp -s $M/qo-j16/$g.qo $O/$g-$f.qo; then same=identical; else same=DIFFERENT; fi
    echo "$g $f decompose=$d recompose=$r bytes=$same text_bytes=$(wc -c < $O/$g.$f)"
  done
done
# an edited text is refused
sed 's/cmt_atomic/cmt_atomix/' $O/cmetrics.yaml > $O/edited.yaml
$Q checkpoint recompose $O/edited.yaml --format yaml --output $O/edited.qo; echo "edited recompose exit $?"
