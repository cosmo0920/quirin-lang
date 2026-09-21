# Shared contract assets

`contracts/` owns the 24 language, support-library and application contract
modules shared by repository integrations. These are canonical sources, not
copies of provider-crate contracts. The [corpus index](../../contract/corpus.json)
combines this directory with the five provider-owned contract roots and rejects
duplicate logical names.

This directory is a data bundle, not an executable provider or a Cargo crate.
Consumers select the required `.qcore` modules explicitly. The repository's
[resource manifest](../../cycle19-resource-contracts.json) selects the resource-v1
subset together with the provider-specific modules. The five executable package
inventories describe their own 72 assets; they do not implicitly select or
install these shared modules. See [corpus conventions](../../contract/README.md).
