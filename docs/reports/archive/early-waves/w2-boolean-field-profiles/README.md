# W2 Boolean field profile routing (H5, bounded)

The single-field typed C importer now takes record names, field names and
operation mappings from loaded `boolean_field_models`. The compatibility entry
point selects the pinned default profile. `import_chunk_source_with_profile`
accepts an explicitly loaded profile and selected library names, checks exact
revision bindings, and rejects unsupported or ambiguous mappings.

The former `flb_input_chunk.busy` and `cio_chunk.lock` routing tables are removed.
The accepted semantic mappings remain SetBusy/SetBusy and Lock/Unlock. Field
layout validation and the complete two-value source execution grammar are
unchanged. Missing fields, other ABIs, aliases, non-Boolean writes, bitfields and
volatile fields still produce Unknown; malformed or unsupported profile routes
reject the import. Duplicate record/field declarations reject during loading.

Every resulting summary now carries the exact declared profile assumptions.
Instantiation requires a live identity, exclusive pointer access, and equality
between the concrete field and the model Boolean state. A profile declaration
establishes none of those premises. This remains a conditional field-effect
summary, never a Safe translation-unit verdict. The Summary enum payload is
boxed to keep enum storage bounded after adding its assumption map.

Tests use an unrelated `widget.held` record and custom source pin, select both
supported mappings through data, and exercise absent, malformed and ambiguous
routes. The existing ChunkIO helper ABI and frame-write grammar remain bounded
to their original record; this increment does not generalize those helpers or
prove their callers' pointer provenance.

## Pinned corpus parity

The full 185-TU audit has byte-identical resource diagnostics to the W4 extension
baseline (SHA-256 recorded in parity.json): 3 Safe, 98 provisional Unsafe,
80 Unknown and 4 data-only. Source projections are byte-identical. Chunk
contracts differ only by three new `profile_assumptions` fields, whose complete
values and paths are archived in parity.json. Removing exactly those fields
reproduces the baseline JSON. Raw units are not promoted.

Compressed current reports, the old chunk report, and exact comparison results
are retained here. The source pin, field models and contract digests remain
visible in the attached assumptions. This is an importer routing change with no
change to the Boolean transition semantics; the existing Lean models apply.

## Validation

All standing gates passed: 505 workspace tests, formatting, Clippy with
warnings denied, both reevaluation probes, and Lean build. Logs and checksums
are archived here.
