# Parity declaration: rule 33's DETACH comparison, replayed

Written by card MEASURE-TOOL before the replay ran. It transcribes, without
change of substance, the measured outcome card DETACH-RESULT-WRAPPER-COMPOSITION
recorded for its pinned-corpus BASE `e282aaccc` and TIP `9ecc941d5`
([report](../../detach-result-wrapper-composition/README.md),
[corpus summary](../../detach-result-wrapper-composition/artifacts/corpus-summary.json)):
975 → 977 Safe, 3,850 → 3,848 Unknown, 9 provisional Unsafe unchanged, and
exactly `cJSON_DetachItemFromArray` and `cJSON_DeleteItemFromArray` move
Unknown → Safe. That card's own frozen prediction
([PREDICTION.md](../../detach-result-wrapper-composition/PREDICTION.md)) is
prose and is not edited; this file is the same claim in the block the tool
reads. The replay passes only if the tool reproduces these moves exactly: any
other moved row is named as unpredicted, and a declared move that does not
happen is named as missing.

```measure-prediction
route corpus
move librdkafka::cJSON.c::cJSON_DetachItemFromArray Unknown -> Safe
move librdkafka::cJSON.c::cJSON_DeleteItemFromArray Unknown -> Safe
delta Safe +2
delta Unknown -2
delta Unsafe 0
```
