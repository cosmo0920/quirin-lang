# Physical extension migration (EXT-02/03)

In progress after the passing EXT-01 compatibility gate, merged at `10a56bf3`. The user requested all contract sources as `.qv`, runtime sources as `.qr`, and native ABI manifests as `.qrlib`, explicitly including outdated historical inputs. The [prediction](prediction.md) records the initial scope and the user correction before mutation.

The parser map selects 195 contract files, 421 runtime files and five ABI manifests. Contract and runtime/ABI tracks integrate in that order. The gate, declaration/lock identity comparison and runtime emitted-C/source parity remain completion requirements; preparation is not a pass.

The integrator's [link migration](markdown-links.json) records exact old/new Markdown targets. Its [script](migrate_markdown_links.py) resolves each link relative to its document and changes only a target present in the approved source map. Before/after filename maps and measured JSON records remain explicit historical records. Updating a source link does not claim its original measurements were rerun on the new filename; the report's original revision and recorded hashes identify those measurements. No report or verdict archive is deleted.

The separate [runtime import card](../../plans/runtime-imports/README.md) follows these renames. It has not been implemented by this filename migration.
