| Binary | Condition | Runs | held silent | held detached | held large | held after reading | held briefly | descendant reads | reads late | reads large | background /dev/null | earlier nine |
|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| base `6c3cd4d7` | idle | 20 | 0 | 0 | 0 | 13 | 0 | 4 | 20 | 0 | 0 | 0 |
| base `6c3cd4d7` | 20 ms host stall | 20 | 0 | 0 | 0 | 17 | 0 | 0 | 20 | 0 | 0 | 0 |
| base `6c3cd4d7` | 50 ms host stall | 20 | 0 | 0 | 0 | 18 | 0 | 0 | 20 | 0 | 0 | 0 |
| base `6c3cd4d7` | first write +30 ms | 20 | 0 | 0 | 0 | 19 | 0 | 3 | 20 | 0 | 0 | 0 |
| base `6c3cd4d7` | close +30 ms | 20 | 0 | 0 | 0 | 20 | 0 | 0 | 0 | 0 | 0 | 0 |
| base `6c3cd4d7` | both delays + 20 ms stall | 20 | 0 | 0 | 0 | 20 | 0 | 0 | 0 | 0 | 0 | 0 |
| after `dbc9d3f2` | idle | 50 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| after `dbc9d3f2` | 20 ms host stall | 50 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| after `dbc9d3f2` | 50 ms host stall | 50 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| after `dbc9d3f2` | first write +30 ms | 50 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| after `dbc9d3f2` | close +30 ms | 50 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| after `dbc9d3f2` | both delays + 20 ms stall | 50 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |

base: 120 runs, 194 deviating case-runs

after: 300 runs, 0 deviating case-runs
