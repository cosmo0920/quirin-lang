# Verification & card reports hub

[Documentation hub](../README.md) · [Alphabetical card lookup (`INDEX.md`)](INDEX.md) · [Current status](../STATUS.md) · [Increment history](../development/increments/README.md)

Card and integration reports are organized into two tiers so active/test-pinned landmarks stay immediately accessible while historical wave reports are grouped by era and subsystem:

- **Top-level active & mechanically pinned reports (`docs/reports/<card>/`)**: **272** directories referenced directly by Rust/Python test harnesses, evaluation baselines, contribution ledgers, or rule gates.
- **Categorized historical archive (`docs/reports/archive/<category>/<card>/`)**: **700** directories grouped into era and subsystem buckets below.

## Historical archive categories

| Archive directory | Reports | Scope |
|---|---:|---|

| [`archive/early-waves/`](archive/early-waves/README.md) | 135 | Waves 1–19 and early `remaining-*` conformance/remediation cards |

| [`archive/waves-20-39/`](archive/waves-20-39/README.md) | 21 | Waves 20–39 source profiles, standard modules, and close records |

| [`archive/cycles-18-19/`](archive/cycles-18-19/README.md) | 24 | Cycle 18 & Cycle 19 evaluations, custody, and OS failure suites |

| [`archive/waves-40-47/`](archive/waves-40-47/README.md) | 80 | Waves 40–47 C-gateway, audit tooling, and provider cards |

| [`archive/wave-48/`](archive/wave-48/README.md) | 37 | Completed Wave 48 historical cards and integration landings |

| [`archive/gateway-and-clang/`](archive/gateway-and-clang/README.md) | 86 | C-gateway rules, Clang-gap (`CG*`), and alias/access cards |

| [`archive/contracts-and-premises/`](archive/contracts-and-premises/README.md) | 68 | Contract loaders, symbolic premises (`SP*`), bounds, null-deref, and cell protocols |

| [`archive/language-and-tooling/`](archive/language-and-tooling/README.md) | 249 | Quirin `.qr` language profiles, concurrency, FFI, gate/audit tooling, and compactions |


## Artifact hygiene & provenance manifests

Unreferenced multi-megabyte raw corpus dumps, per-step execution logs, and superseded attempt directories are pruned from git working trees into per-card `pruned-artifacts.json` manifests (`schema: "quirin-pruned-report-artifacts/1"`) recording each file's original relative path, byte count, and SHA-256 digest. New reports are checked by `python3 scripts/check_report_hygiene.py` (wired into `python3 scripts/land.py check`).
