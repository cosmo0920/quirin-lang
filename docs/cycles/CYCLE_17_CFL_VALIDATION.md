# Cycle 17 CFL importer validation

The unrestricted-C gateway was exercised against the adjacent CFL source tree
without changing that checkout.

- CFL revision: `6e35cca`
- translation unit: `src/cfl_atomic_generic.c`
- entry: `cfl_atomic_initialize`
- compiler gateway: Clang C11 preprocessing
- include path: `include`

Command, run from the QCore-C root:

```sh
cargo run -p qcore-compiler --example cycle17_legacy_audit -- \
  ../cfl/src/cfl_atomic_generic.c cfl_atomic_initialize ../cfl/include
```

Result:

```text
Unknown: the conservative C11 projection contains an unsupported operation; 5 partially analyzed function(s)
```

This is the expected fail-closed result. CFL's generic atomic implementation
contains pthread calls, indirect memory accesses, global initialization state,
and mutex-mediated atomic behavior for which this importer has no trusted
external summaries. The production preprocessor accepted the translation
unit, but the resource analyzer did not issue `Safe` for semantics it could not
prove.

A second probe of `src/cfl.c` also returned `Unknown`; preprocessing its public
umbrella header expands bundled xxHash inline functions beyond the conservative
projection before a partial program summary can be formed.
