# quirin-provider-plugin

The SDK for **user-defined contracts and checker plugins** (card
W45-USER-CONTRACT-PLUGINS; contract specification
[C11.1](../../docs/QCORE_CONTRACT_SPEC_r0_3.md#c111-user-defined-contracts-and-checker-plugins),
language specification
[§L21.2](../../docs/QCORE_SPEC_r0_3.md#l212-the-four-value-verdict-and-its-composition)).
It carries **no production contracts** and links no core crate, so it is a
member of the providers workspace but not a crate on
`providers/contract-load-path.json` (it is listed with the conformance harness
as a package that owns no contracts).

## The user workflow

A user keeps contracts in their own crate, outside this repository, laid out
like the `quirin-provider-*` crates:

```text
work/
  user-load-path.json            generated below
  quirin-provider-acme/
    Cargo.toml                   [package] name = "quirin-provider-acme"
    src/lib.rs                   pub const NAMESPACE: &str = "acme";  pub const PUBLISHED: &[&str] = &[];
    contracts/acme/pool.qcore    resource-v1 contracts
    contracts.lock               `quirin-audit pin`, after review (pinned contracts)
    quirin-plugin.json           checker bindings and explicit overrides
    checkers/pool-check/         a checker package (optional)
```

```sh
quirin-plugin new work/quirin-provider-acme --name quirin-provider-acme
# write contracts under contracts/acme/, then:
quirin-plugin check work/quirin-provider-acme
quirin-plugin load-path --extends /path/to/quirin/providers/contract-load-path.json \
    --output work/user-load-path.json work/quirin-provider-acme
quirin-audit plugin views --load-path work/user-load-path.json --write
quirin-audit pin --load-path work/user-load-path.json --source-root SOURCE
quirin-audit contracts --load-path work/user-load-path.json \
    --provider quirin-provider-acme --source-root SOURCE --report out/r.json
```

The template's one contract is held awaiting review
(`resource_review_pending`), so a new crate makes nothing `Safe` until a person
has written and reviewed a real contract. User contracts obey every rule of the
shipped ones: total-or-reject loading, bodies checked against their contracts,
pinned by default (the user's own `contracts.lock`), and the review marker. A
user contract for a function a shipped crate also declares is **refused**
unless `quirin-plugin.json` overrides it explicitly, with a reason; every
applied override is recorded in the audit's `user_load_path` record.

## Checker plugins

`quirin-plugin.json` may bind an out-of-process checker to a module's
contracts:

```json
{ "schema_version": 1,
  "checkers": [ { "module": "acme/pool.qcore",
                  "functions": ["acme_pool_get", "acme_pool_put"],
                  "package": "checkers/pool-check" } ],
  "overrides": [] }
```

A checker is a provider executable written against [`checker`](src/checker.rs)
(the shared provider transport, evidence-v2 requests) with its manifest
template compiled in; `quirin-plugin package --executable PATH --output DIR`
installs it with its digest, and `quirin-plugin verify DIR` checks the package
(the audit re-verifies every package it runs). The worked example is
[`src/bin/example_checker.rs`](src/bin/example_checker.rs).

What a checker says is a **claim**. The audit counts its `Safe` only with a
certificate the audit's own trusted checker replays, and its `Unsafe` only with
a witness the audit's own trusted checker validates; otherwise either is
**`Pending`** (`plugin-claim-uncertified`). A crash, timeout, quota breach,
protocol error, mismatch or refuted evidence is `Unknown`. The command line
has no trusted checker for user contracts yet, so every plugin `Safe` it sees
is `Pending`. A bound contract's verdict is joined into every function that
applies it.

## Modules

| Module | What it does |
|---|---|
| [`user_crate`](src/user_crate.rs) | reads and validates a user crate and its `quirin-plugin.json` |
| [`load_path`](src/load_path.rs) | the user load path, extending a shipped one |
| [`template`](src/template.rs) | a new crate from [`template/`](template/) |
| [`package`](src/package.rs) | build and verify a checker package |
| [`checker`](src/checker.rs) | the checker runtime and response builders |

Tests: [`tests/sdk.rs`](tests/sdk.rs) on the example user crate
([`tests/fixtures/example-user-crate/`](tests/fixtures/example-user-crate/));
the audit's end-to-end tests run the same crate with real checker processes
(`crates/qcore-audit/tests/user_contract_plugins.rs`).
