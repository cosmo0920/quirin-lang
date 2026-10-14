| Campaign | Binary sha256 | Runs | Deviating | With ETXTBSY | Child stalls (total) | Wall s (max) |
|---|---|---|---|---|---|---|
| `base-rel-close100000-stall200` | `c8dcbed520aa…` | 50 | **10** | 10 | 697 | 1.72 |
| `base-rel-close20000-stall20` | `c8dcbed520aa…` | 50 | **9** | 9 | 1307 | 0.42 |
| `tip-rel-close100000-stall200` | `0910c996954b…` | 50 | **0** | 0 | 1321 | 2.72 |
| `tip-rel-close20000-stall20` | `0910c996954b…` | 50 | **0** | 0 | 1689 | 0.52 |

Pilots (not the recorded campaign; serial = the copied harness's one-at-a-time child stall):

| Pilot | Binary | Profile | Close delay | Child stall | Runs | Deviating |
|---|---|---|---|---|---|---|
| `pilot-base` | `f78a03dbbd14…` | debug | 20 ms | 20 ms serial | 3 | 0 |
| `pilot2-base` | `f78a03dbbd14…` | debug | 3,000 ms | 6,000 ms serial | 3 | 0 |
| `pilot3-base-rel` | `c8dcbed520aa…` | release | 20 ms | 20 ms serial | 5 | 0 |
| `pilot4-base-rel` | `c8dcbed520aa…` | release | 100 ms | 200 ms serial | 10 | 0 |
| `pilot5-base-rel` | `c8dcbed520aa…` | release | 100 ms | 200 ms parallel | 10 | 3 (all ETXTBSY) |
