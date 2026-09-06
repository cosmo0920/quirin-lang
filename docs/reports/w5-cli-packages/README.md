# W5 CLI package selection

The CLI adds read-only `plugins verify`, explicit config-file `enable` and
`disable`, configured list/inspect and `check`/`emit --plugins CONFIG`.
`qcore-verifier-host::verify_executable_package` checks manifest syntax,
canonical path containment and the declared executable SHA-256 without launching
it. The CLI pins the full manifest in each enabled selection and checks it again
before registering a process provider. Missing/malformed/changed enabled entries
reject before source compilation; no default registry fallback occurs. Disabled
entries do not enter the registry and need not remain installed.

Enable validates before any config mutation. Configuration version, absolute
package paths, unique IDs and unknown fields are checked. A sibling create-new
temporary file plus rename avoids partial replacement. Re-enable explicitly
selects a new version/path for upgrade or rollback. These are local selection
files, not a concurrent-writer transaction protocol. No implicit home-directory
configuration or global installation is introduced.

Four CLI process regressions cover no-execution verification with a deliberately
non-runnable file, version pinning/re-enable, disable and source-scope absence,
config/missing-package/tamper rejection without mutation, duplicate IDs,
unsupported config fields/version, and Unix symlink escape. Real packaged
Libevent verify/enable/list/disable was also exercised; both enabled and disabled
listing outputs are archived. The complete executable-package conformance suite
remains in the separate provider workspace.

Verification establishes snapshot identity, not signatures, source authenticity,
runtime compatibility, embedded-template identity or immunity to later filesystem
mutation. First-party `package.py verify` performs the additional executable
self-check. Protect package directories against mutation between verification and
dispatch if immutable identity is required. Existing process containment and
certificate admission rules are unchanged. No Lean semantic definition changed;
no new source/native/formal refinement theorem is claimed.

This completes the bounded CLI lifecycle tooling item. Full W5 acceptance still
requires specialized provider model extraction, registry-based Fluent Bit parity,
removal of old routes and the remaining compiler consolidation.
