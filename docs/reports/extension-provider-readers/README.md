# EXT-01 provider package readers

Base: `64ddac3f60fe9b848dd789e895d67b809cce93e2`.

## Prediction, recorded before implementation

This card changes provider Python path readers and inventory validation only. Existing `.qcore` source, manifest, published-list, load-path, lock and package artifact bytes are not renamed or rewritten. The expected delta for all five existing provider package inventories, package digests, contract counts and checker verdicts is **zero**. Synthetic `.qv` contract-mode modules will be accepted at their literal paths and hashes; a `.qcore`/`.qv` pair with the same logical module stem will be rejected before search-order selection. Legacy `.qcore` modules remain readable and emit a visible path-specific warning stating detected contract mode and `.qv` recommendation. No pin is regenerated and no unsupported source form becomes Safe. If actual parser mode validation cannot be reached through this Python boundary, the report will name that integration dependency; it will not substitute a lexical guess.
