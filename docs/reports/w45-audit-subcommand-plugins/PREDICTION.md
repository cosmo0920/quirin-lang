# W45-AUDIT-SUBCOMMAND-PLUGINS: prediction (frozen before implementation)

Written on the base `37e1b82d` ("Record the w45-refuted-contract-withdrawal
integration gate") before any code change, while the BEFORE arbiter binary was
building. It is not edited afterwards; the report compares against it.

## The design being predicted

`quirin-audit NAME …` gains cargo-style external subcommands: a name that is no
built-in runs an executable `quirin-audit-NAME` found only on a declared
search path (`--plugin-path`, `QUIRIN_AUDIT_PLUGIN_PATH`; never `PATH`), out
of process, with a cleared environment, a versioned JSON request on stdin and a
JSON response on stdout, under wall-time and output quotas, its working
directory its declared output. Its verdict claims enter the result only through
the checker-plugin firewall (`qcore_verifier_host::PluginClaim`) and are joined
with the host's verdicts. The first plugin, `quirin-audit-plugin-libevent`,
ports `scripts/prepare_libevent_core_audit.py`.

## Predictions

1. **Corpus: no movement.** No analysis code changes: the new module is reached
   only when `quirin-audit`'s first argument is an external subcommand name or
   a host option, and the arbiter (`cycle19_fluent_bit_libraries_audit
   --pinned-evidence`) never calls it. BEFORE reproduces the
   W45-CONTAINER-ROOT-REACH integration baseline (functions 398 / 9 / 4,427,
   units 5 / 8 / 168 / 4, leak 34, Pending 0; the contracts sidecar's one
   additive empty `refuted_contracts` leaf), and AFTER is byte-identical to
   BEFORE in every artifact except the worktree-bound paths. **No Safe →
   non-Safe move, no leak or violation leaf added.**
2. **Fluent Bit slice: not run.** Nothing the slice route executes changes;
   the prediction is identity by construction, stated, not measured.
3. **Parity.** On the pinned `third_party/libevent` (`d82464a2`), the plugin
   writes `compile_commands.json`, `libevent-core-audit.json.config`,
   `libevent-build-provenance.json`, `CMakeCache.txt`, the static library and
   every object byte-identical to the script's in a same-path run; only CMake's
   and make's logs may differ (random try-compile names, timings, `-j2`
   interleaving), each difference named.
4. **Plugin verdicts.** The libevent plugin makes no verdict claim. No
   subcommand plugin can reach `Safe` from the command line: an uncertified
   `Safe` or `Unsafe` claim is `Pending` (`plugin-claim-uncertified`), every
   failure `Unknown`, and the join never lowers a host verdict.
5. **Ledger:** nothing is retired; new follow-ups are registered for the
   subcommand certificate route and any confinement gap measured.
