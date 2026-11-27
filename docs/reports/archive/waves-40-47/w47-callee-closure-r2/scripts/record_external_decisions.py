#!/usr/bin/env python3
"""Record the user's 2026-09-28 decision on OpenSSL and libcurl contracts (idempotent).

Usage: record_external_decisions.py   (then docs/reports/remaining-d02/build_matrix.py .)

* amends W47-EXTERNAL-RESIDUAL-RANKING-R2 with the decision;
* registers W48-PLUGIN-VERDICT-PRESERVATION, W48-OPENSSL-CONTRACT-PLUGIN and
  W48-LIBCURL-CONTRACT-PLUGIN.
"""
import json
import pathlib

ROOT = pathlib.Path(__file__).resolve().parents[4]
MATRIX = ROOT / "docs/reviews/r0_3/acceptance_matrix.json"
MARK = "USER DECISION (2026-09-28, OpenSSL/libcurl):"
AMEND = (
    " " + MARK + " OpenSSL and libcurl contracts are provided through the quirin-audit plugin "
    "crate mechanism (crates/quirin-audit-plugin SDK, rule 10h), each library with its own "
    "contracts and its own checking/proof system, integrated through the existing four-value "
    "verdict semigroup (QCore.VerdictSemigroup, contract specification C11.1) so that no "
    "Safe/Pending/Unknown/Unsafe information or reason from those checkers is erased. Not a "
    "flexible mode in a provider crate. Registered as W48-PLUGIN-VERDICT-PRESERVATION, "
    "W48-OPENSSL-CONTRACT-PLUGIN and W48-LIBCURL-CONTRACT-PLUGIN. The other declined classes "
    "(callback/retention vocabulary, in-place lifecycle and reallocation, counted references, "
    "handle kinds and no-return, compiler builtins, Monkey backend bodies) remain undecided."
)
COMMON = (
    " Composition: the plugin's per-binding verdict and reasons enter the host only through the "
    "plugin firewall (rule 10h) and the reasoned-verdict join (C11.1, QCore.VerdictSemigroup), "
    "never lowering a host verdict and never erasing a plugin verdict or reason; until a trusted "
    "checker for the plugin's certificates exists (W45-SUBCOMMAND-CERTIFICATE-ROUTE, "
    "W45-PLUGIN-CERTIFICATE-CHECKER) an uncertified plugin Safe or Unsafe is published as "
    "Pending, which is not Safe. Fail-closed: spawn failure, timeout, quota breach, version or "
    "pin mismatch make the binding Unknown."
)
NEW = [
    {
        "id": "W48-PLUGIN-VERDICT-PRESERVATION",
        "title": "Composing external contract-checker plugins erases no verdict or reason",
        "why": (
            "User decision 2026-09-28: OpenSSL and libcurl contracts are checked by their own "
            "quirin-audit plugins and integrated through the verdict semigroup without erasing any "
            "Safe/Pending/Unsafe/Unknown information from their checkers. Before the library "
            "plugins land, establish what the host already keeps (the reasoned verdict's reason "
            "multiset, the sidecar's plugin_checks records, Pending markers) and close any point "
            "where a plugin's verdict, reason, identity or evidence is dropped, collapsed or "
            "overwritten between the plugin's answer and the published row, report, checkpoint "
            "(.qo) composition and sidecars. DISCHARGE: an end-to-end trace, for each of the four "
            "values, from a test plugin's answer to the published artifacts; a projection that "
            "recovers each plugin's own verdict and reasons from a composed row; audited Lean in "
            "QCore.VerdictSemigroup that the join preserves every contributor's (source, verdict, "
            "reasons) record and that the published value is their join; negative controls "
            "(two plugins disagreeing, a plugin Unsafe beside a host Safe, a failing plugin)."
        ),
        "source": "docs/reports/w47-callee-closure-r2/README.md",
        "recorded_by": "INTEGRATOR (user decision on W47-CALLEE-CLOSURE-R2)",
    },
    {
        "id": "W48-OPENSSL-CONTRACT-PLUGIN",
        "title": "OpenSSL contracts through a quirin-audit plugin with its own checker",
        "why": (
            "User decision 2026-09-28. OpenSSL has no audited source in the checkout to pin "
            "(W46-EXTERNAL-RESIDUAL-RANKING: ERR_clear_error blocks a Fluent Bit function alone). "
            "Build a quirin-audit-plugin-openssl crate on the SDK (model: "
            "crates/quirin-audit-plugin-libevent) that prepares a pinned OpenSSL source revision "
            "(the one Fluent Bit builds against, recorded), carries OpenSSL's own contracts, and "
            "checks them with its own checker/proof system, answering per binding with a verdict, "
            "reasons and evidence." + COMMON + " DISCHARGE: the plugin, its pinned preparation, "
            "contracts for the ranked callees (ERR_clear_error first, then the OpenSSL callees the "
            "round-2 ranking names), Rust tests and audited Lean for the plugin-specific premises, "
            "the provider/plugin gate, and a measured corpus/system arbiter attributing every move."
        ),
        "source": "docs/reports/w47-callee-closure-r2/README.md",
        "recorded_by": "INTEGRATOR (user decision on W47-CALLEE-CLOSURE-R2)",
    },
    {
        "id": "W48-LIBCURL-CONTRACT-PLUGIN",
        "title": "libcurl contracts through a quirin-audit plugin with its own checker",
        "why": (
            "User decision 2026-09-28. libcurl has no audited source in the checkout to pin "
            "(W46-EXTERNAL-RESIDUAL-RANKING: curl_global_init blocks a function alone). Build a "
            "quirin-audit-plugin-libcurl crate on the SDK (model: "
            "crates/quirin-audit-plugin-libevent) that prepares a pinned libcurl source revision, "
            "carries libcurl's own contracts and checks them with its own checker/proof system, "
            "answering per binding with a verdict, reasons and evidence." + COMMON + " DISCHARGE: "
            "the plugin, its pinned preparation, contracts for the ranked callees "
            "(curl_global_init first), Rust tests and audited Lean for the plugin-specific "
            "premises, the provider/plugin gate, and a measured corpus/system arbiter attributing "
            "every move."
        ),
        "source": "docs/reports/w47-callee-closure-r2/README.md",
        "recorded_by": "INTEGRATOR (user decision on W47-CALLEE-CLOSURE-R2)",
    },
]

matrix = json.loads(MATRIX.read_text())
entries = matrix["named_followups"]
ids = {entry["id"]: entry for entry in entries}
before = len(entries)
entry = ids["W47-EXTERNAL-RESIDUAL-RANKING-R2"]
if MARK not in entry["why"]:
    entry["why"] += AMEND
for new in NEW:
    if new["id"] not in ids:
        entries.append(new)
MATRIX.write_text(json.dumps(matrix, indent=1, ensure_ascii=False) + "\n")
print(f"follow-ups {before} -> {len(entries)}")
