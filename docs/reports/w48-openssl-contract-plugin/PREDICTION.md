# W48-OPENSSL-CONTRACT-PLUGIN prediction

Committed on the card branch before any implementation, at base `d7a497822`.
Sides: `rev:d7a497822` (base) and `rev:<card tip>` (tip), both measured on
this card's own `quirin-audit prepare` preparations under the worktree's
`private/` (the host has no published baseline or prepared live-route inputs),
so absolute figures are not the published baselines'.

**What changes.** A new external subcommand crate,
`crates/quirin-audit-plugin-openssl` (`quirin-audit openssl prepare|check`),
which prepares the pinned OpenSSL release and checks OpenSSL's own contracts
with its own checker, answering per binding through the subcommand host's
plugin firewall (rule 10h, contract specification C11.2). It adds Rust tests,
a Lean module and its audit, documentation and records. It changes no
analyzer code, no provider contract, no lock, no profile, no selection and no
link map, and no measured route invokes a subcommand plugin: the pinned
corpus route (`run_pinned_audit.py`) and the configured selections
(`quirin-audit current` over a selection) never spawn `quirin-audit-NAME`.

**Rows that can move.** None. The plugin's per-binding verdicts are published
in its own `quirin-audit-openssl.report.json`, one row per binding
(`openssl@3.5.5::<function>`), never in a route's diagnostics or sidecars.
Bringing those verdicts to the Fluent Bit callers of the bound functions
(`rd_kafka_transport_ssl_clear_error` and the other census callers) needs a
host route that reads a plugin report into the analysis, which is host
composition and out of this card's scope; it is recorded as a follow-up, not
predicted.

The only expected differences are environment-only: each side's binary and
build commit.

```measure-prediction
route corpus
none
route system
none
route slice
none
```
