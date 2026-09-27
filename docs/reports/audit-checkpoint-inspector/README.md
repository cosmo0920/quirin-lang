# Binary checkpoint inspector

`quirin-dump-checkpoint FILE.qo` validates the container and every section before writing a JSON summary: format/schema versions, content digest, source/configuration identity, result and section manifest. `--full` prints every decoded field; `--section NAME` prints one section after validating the entire file. Neither mode executes analysis or changes the checkpoint. An integrity check is not a new safety verdict. Invalid schemas, corrupt digests, truncation and invalid options exit nonzero without a decoded dump on stdout.

The binary is installed with the `qcore-audit` package. `default-run = "quirin-audit"` preserves existing `cargo run -p qcore-audit -- …` commands after adding the second binary. Run the inspector from source with:

```sh
cargo run --release -p qcore-audit --bin quirin-dump-checkpoint -- FILE.qo
cargo run --release -p qcore-audit --bin quirin-dump-checkpoint -- --section verdicts FILE.qo
```

The tool reads only the schema supported by its build. Older checkpoints remain untouched and are refused; use their matching older inspector/decomposer or recompute with the new audit build. Full dumps can be large. The numeric binary schema and per-configuration persistence belong to [the checkpoint card](../audit-checkpoint-files/README.md).

Validation is pending the final integrated gate. Focused CLI tests cover help, bad options and refusing truncated input without partial output; the companion checkpoint integration test checks the decoded contents against a freshly generated complete checkpoint.
