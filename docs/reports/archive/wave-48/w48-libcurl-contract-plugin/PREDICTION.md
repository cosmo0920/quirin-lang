# W48-LIBCURL-CONTRACT-PLUGIN: prediction

Committed before any implementation code, on base `d7a497822`.

## What the card adds

A new workspace crate, `crates/quirin-audit-plugin-libcurl` (the
`quirin-audit libcurl` subcommand plugin and a C11.1 checker package built
from the same crate). It prepares a pinned libcurl release (8.18.0, upstream
tarball, sha256-pinned), carries libcurl contracts and checks them against
that source with its own checker. Its per-binding answers enter the host only
through the plugin firewall: on the subcommand route (C11.2,
`classify_claim`) and, through a user load path with a checker binding, on
the `contracts`/`current` route (C11.1, `user_plugins::run` and
`fold_unit`). No analyzer crate (`qcore-audit`, `qcore-c-gateway`,
`qcore-contract`, `qcore-schemas`, `qcore-verifier-host`) changes, and no
shipped contract or load-path entry changes.

## Measured routes

The live routes the arbiter measures are the pinned corpus (`quirin-audit
pinned`) and the configured selections (`quirin-audit CONFIG.json`). Neither
runs a subcommand plugin, and neither reads a user load path
(`W45-PLUGIN-ROUTE-COVERAGE`), so no libcurl contract or plugin verdict can
reach them. Prediction: **no status move and no content change on any row**
of either route; the only differences are environment-only fields that spell
the build commit.

```measure-prediction
route corpus
none
route system
none
```

## The route the plugin does reach (not an arbiter route)

On `quirin-audit contracts` over librdkafka's `rdhttp.c` (Fluent Bit
`046accf6`, configured with libcurl 8.18.0 headers), with the plugin's user
load path and checker binding:

* `rd_http_global_init`, whose only blocker is the uncontracted call to
  `curl_global_init`, moves **Unknown -> Pending**
  (`plugin-claim-uncertified`): the contract applies, and the binding's
  uncertified `Safe` claim joins in as `Pending`. Never `Safe`.
* No function moves to `Safe`. A function that applies a bound contract and
  is otherwise `Safe` becomes `Pending`; a function with any other blocker
  stays `Unknown`.
* The 44 `Wcurl_easy_setopt_err_*` / `Wcurl_easy_getinfo_err_*` stubs get no
  contract and no binding, so every function calling them is unchanged.
