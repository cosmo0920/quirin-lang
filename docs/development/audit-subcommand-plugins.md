# Audit subcommand plugins

`quirin-audit` runs external subcommands the way `cargo` does: for a name that
is not built in, `quirin-audit NAME ARGS…` runs an executable named
`quirin-audit-NAME`. The SDK for writing one is the crate
[`quirin-audit-plugin`](../../crates/quirin-audit-plugin/src/lib.rs); the
first plugin is [`quirin-audit-plugin-libevent`](../../crates/quirin-audit-plugin-libevent/src/lib.rs)
(`quirin-audit libevent prepare`). The normative text is contract
specification [C11.2](../QCORE_CONTRACT_SPEC_r0_3.md#c112-audit-subcommand-plugins);
the host is `crates/qcore-audit/src/subcommand_plugins.rs`; the evidence is
[W45-AUDIT-SUBCOMMAND-PLUGINS](../reports/w45-audit-subcommand-plugins/README.md).

## Running one

```sh
quirin-audit [--plugin-path DIR]... --output DIR [HOST-OPTIONS] NAME [ARGUMENTS...]
quirin-audit [--plugin-path DIR]... --list-plugins
```

Host options come before `NAME`; everything after it goes to the plugin
verbatim.

| Option | Meaning |
|---|---|
| `--plugin-path DIR` | A directory to search, in order; repeatable |
| `--output DIR` | **Required.** The plugin's working directory and the only place its artifacts may lie; created if missing |
| `--report FILE` | The host's report (default `<output>/quirin-audit-NAME.report.json`); `<report>.invocation.json` and `<report>.stderr.log` go beside it |
| `--audit-report FILE` | An existing audit report whose unit statuses are the host's verdicts (below) |
| `--qo FILE` | A `.qo` checkpoint, verified whole and passed by content digest; all must come from one run; repeatable |
| `--load-path FILE`, `--provider NAME` | A contract load path, validated, and crates on it; passed to the plugin |
| `--timeout-ms N` | Wall-time quota (default 30 minutes) |
| `--stdout-limit-bytes N`, `--stderr-limit-bytes N` | Output quotas (default 16 MiB each) |
| `--plugin-env NAME` | Pass one more environment variable by name; repeatable |

**Search.** Only the declared path is searched: every `--plugin-path`, then
the directories in `QUIRIN_AUDIT_PLUGIN_PATH`. `PATH` is never searched, so a
program that happens to be called `quirin-audit-something` is never run by
accident. A built-in name (`current`, `prepare`, `plugin`, …) always runs the
built-in; an external executable of that name is never run, and
`--list-plugins` marks it as shadowed. An unknown name is a usage error that
lists the search path. A configuration path — the other form of the first
argument, `quirin-audit CONFIG.json` — contains a `.` or a `/`.

**Environment.** The plugin's environment is cleared except `PATH`, `HOME`,
`TMPDIR`, `LANG`, `LC_ALL`, `LC_CTYPE` and `TZ` (when set) and each
`--plugin-env NAME`. `PATH` is passed so a plugin can run `git`, `cmake` or
`clang`; it is not used to find the plugin. A relative `PATH` entry resolves
against the plugin's working directory, the output.

**Quotas and confinement.** The plugin runs in its own process group. A
wall-time breach, a response larger than the standard-output quota or
standard error (captured to `<report>.stderr.log`) larger than its quota kills
the whole group; so does the end of the run, so nothing the plugin started
outlives it. Every artifact must resolve inside the output and have the
SHA-256 it declares. This is containment of a process group and of what the
host accepts, **not an operating-system sandbox**: nothing stops a plugin
writing elsewhere, and a descendant that leaves the group escapes the kill
(standing constraint 26).

**Exit status.** 0: the plugin completed and the report is written. 1: the run
failed (the report is written, and its verdict is `Unknown`). 2: a usage error
or a refused input; nothing ran.

## Verdicts

A plugin may *claim* verdicts. A claim is the full tuple of hard invariant 2 —
artifact, property, semantic profile, scope, bounds, assumptions and evidence
version — with a proposed `Safe`, `Unsafe` or `Unknown`, a reason and any
certificate references. The host classifies each claim with the checker-plugin
firewall (contract spec C11.1):

| Claim | Verdict |
|---|---|
| `Safe` with a certificate the audit's own checker accepts | `Safe` — unreachable today: no checker accepts a subcommand plugin's certificate |
| `Safe` without one, or with references no checker reads (`QP008`, `QP016`) | `Pending` (`plugin-claim-uncertified`) |
| `Unsafe` without a witness the audit validates (`QP010`) | `Pending` (`plugin-claim-uncertified`) |
| `Unknown` | `Unknown` |

A spawn failure, a timeout, a quota breach, a nonzero exit, a malformed
response, a protocol-version mismatch (`protocol-version-mismatch`), a failure
the plugin reports, or an artifact that does not verify makes the **run**
`Unknown`, and a failed run's claims are recorded but joined nowhere.

**The join.** Each row of the report is one artifact: the join
(`Safe < Pending < Unknown < Unsafe`) of the host's verdict for it (the
`--audit-report` unit row whose `file` is the artifact), the run's verdict and
the classified claims about it. The report's `verdict` is the join of every row
and the run, or `null` when a completed run has no row (a plugin that claims
nothing publishes no verdict). The join never lowers a host verdict: an
`Unsafe` row stays `Unsafe`, and a failed plugin makes a `Safe` row `Unknown`.
`QCore.SubcommandPlugins` proves these properties of the model.

## The protocol

Version 1 of `quirin-audit-plugin`: one JSON request on the plugin's standard
input, one JSON response on its standard output, nothing else on standard
output. Both carry `"protocol": "quirin-audit-plugin"` and
`"protocol_version": 1`; the host checks them before the shape, so another
version is a named mismatch. Unknown fields are refused.

The **request** carries the subcommand and its arguments; the host's working
directory (`invocation_directory`, against which a relative path argument is
meant); `output`; `report`; the load path and providers; the audit identity
(revision, contract-set digest and refutation-table digest, from the `.qo`
checkpoints); the audit report and checkpoints, each with its digest; and the
quotas.

The **response** carries `outcome` (`"completed"` or `{"failed": REASON}`),
`artifacts` (path, SHA-256, kind), `diagnostics` (severity, code, message) and
`claims`.

## Writing one in Rust

```toml
[dependencies]
quirin-audit-plugin = { path = "…/crates/quirin-audit-plugin" }

[[bin]]
name = "quirin-audit-mytool"
path = "src/main.rs"
```

```rust
use quirin_audit_plugin::{Request, Response, run};

fn answer(request: &Request) -> Result<Response, String> {
    let output = request.output_directory();
    std::fs::write(output.join("result.txt"), "done\n").map_err(|e| e.to_string())?;
    let mut response = Response::completed();
    response.push_artifact(&output, std::path::Path::new("result.txt"), "result")?;
    Ok(response)
}

fn main() -> std::process::ExitCode {
    run(answer)
}
```

`run` reads the request, refuses another protocol version, and writes the
response; an `Err` becomes a failed response and exit 1. Write diagnostics for
a person to standard error. `quirin_audit_plugin::testing::invoke` runs a built
plugin binary on a request as the host would (cleared environment, working
directory the output) and `Invocation::verified` checks its response and
artifacts; `quirin_audit_plugin::python_json` writes `json.dumps(indent=2)`
bytes for a port of a Python script. The crate's own example,
`quirin-audit-example`, is a complete plugin:

```sh
cargo build -p qcore-audit -p quirin-audit-plugin
target/debug/quirin-audit --plugin-path target/debug --output /tmp/out example --claim safe
```

Its report records the artifact and the claim, classified `Pending`.

## The libevent plugin

```sh
cargo build -p qcore-audit -p quirin-audit-plugin-libevent
target/debug/quirin-audit --plugin-path target/debug --output ../out/libevent \
  libevent prepare --clang clang --clangxx clang++
```

It prepares the pinned `libevent-core` configured-audit inputs from the
repository's `third_party/libevent` (`--repository DIR`, else the load path's
repository, else the working directory): CMake with Clang, the
`event_core_static` build, and `libevent-clang-build/` holding CMake's
unmodified `compile_commands.json`, `libevent-core-audit.json.config` and
`libevent-build-provenance.json`, byte for byte as the retired script wrote
them. It refuses an output inside `build/cycle19` or `third_party`, and
`--write-profile` writes the regenerated audited-file table to
`libevent-clang-build/profile.json` for a person to review and copy, rather
than editing the committed profile. It claims no verdict.
