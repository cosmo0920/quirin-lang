#!/usr/bin/env python3
"""CG11-POINTER-VALUED-STORES: the wave-48 increment paragraph, applied once."""

import pathlib

ROOT = pathlib.Path(__file__).resolve().parents[4]
PATH = ROOT / "docs/development/increments/wave-48.md"
ANCHOR = "**`CG02-ELABORATION-CORE` (2026-09-29"
PARAGRAPH = (
    "**`CG11-POINTER-VALUED-STORES` (2026-09-30, base `f19204fa`).** Card CG11 of the Clang gap queue, route G, "
    "the unsupported coverage queue's FIELD-CUSTODY (P5) ([report](../../reports/cg11-pointer-valued-stores/README.md); "
    "gateway rule 10v; `QCore.PointerStore`, new `AuditPointerStore.lean`, 18 theorems, its decision vectors bound to "
    "the Rust table, to Clang's classification and to the route's outcome by shared-vector tests). A pointer-typed "
    "store the typed statement route refused is read from the Clang AST (`pointer_store.rs`, flow in "
    "`legacy_pointer_store.rs`) with the value store, overwrite, old obligation and escape kept apart, and admitted "
    "only for a value with no identity (null, string literal, function, a function-pointer name bound to nothing, the "
    "destination's own object) into an owner whose root and member object are untracked (`chain_owner`) and whose "
    "rooted bindings carry nothing; every failing premise is the gate's refusal byte for byte, and a body with such a "
    "store publishes no summary. A formal's, local's or owned pointer stays refused: Q36's example C is Unknown, its "
    "missing premise an owner identity for `*s` bounding `*q`'s lifetime or a derived Stored summary mode. BEFORE "
    "byte-identical to CG02's AFTER (`963d7c9f…`); corpus **548 / 10 / 4,276** (+5, exactly the frozen prediction: "
    "`cio_options_init` and four `TAILQ_INIT` bodies), system +1 (`map_free_task_id`, missed by the prediction), "
    "tail-pause slice 0; no correction, no leak or violation added, `unknown_alias` unchanged, 0 of "
    "CG10-BOUND-UNTRACKED-ROOTS' leaves touched. CG00's M04 `s->cb = f` control and W45's `r->next = r` and "
    "`h->rec->name = 0` pins flip by name; refusal controls for an owning-field overwrite, a double release through a "
    "stored alias and example C. Follow-ups 623 → 627: its own entry and the ten absorbed ones amended, none retired; "
    "registers `CG11-UNRESOLVED-OWNER-POINTER-STORES`, `CG11-POINTER-STORE-SUMMARY-MODE`, "
    "`CG11-FUNCTION-POINTER-PARAMETER-CLOSURE` (a probe finding, fail-closed) and `CG11-UNREAD-POINTER-PLACES`.\n\n"
)


def main() -> int:
    text = PATH.read_text()
    if "**`CG11-POINTER-VALUED-STORES`" in text:
        raise SystemExit("already applied")
    if text.count(ANCHOR) != 1:
        raise SystemExit("anchor not found once")
    PATH.write_text(text.replace(ANCHOR, PARAGRAPH + ANCHOR))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
