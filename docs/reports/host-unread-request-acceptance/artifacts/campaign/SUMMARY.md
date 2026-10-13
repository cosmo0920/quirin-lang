| Binary | Condition | Runs | silent small | silent small, late exit | closed input | partial (4 B) | silent large | EOF reader small | EOF reader large | exact reader small | exact reader large |
|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| base `d18d4cf8` | idle | 20 | 20 | 20 | 18 | 20 | 0 | 0 | 0 | 0 | 0 |
| base `d18d4cf8` | 20 ms host stall | 20 | 20 | 20 | 20 | 20 | 0 | 0 | 0 | 0 | 0 |
| base `d18d4cf8` | 50 ms host stall | 20 | 20 | 20 | 19 | 20 | 0 | 0 | 0 | 0 | 0 |
| base `d18d4cf8` | first write +30 ms | 20 | 0 | 20 | 0 | 20 | 0 | 0 | 0 | 0 | 0 |
| base `d18d4cf8` | close +30 ms | 20 | 0 | 0 | 0 | 20 | 0 | 0 | 0 | 0 | 0 |
| base `d18d4cf8` | both delays + 20 ms stall | 20 | 0 | 0 | 0 | 20 | 0 | 0 | 0 | 0 | 0 |
| after `e1f20731` | idle | 50 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| after `e1f20731` | 20 ms host stall | 50 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| after `e1f20731` | 50 ms host stall | 50 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| after `e1f20731` | first write +30 ms | 50 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| after `e1f20731` | close +30 ms | 50 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| after `e1f20731` | both delays + 20 ms stall | 50 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |

base: 120 runs, 317 deviating case-runs

after: 300 runs, 0 deviating case-runs
