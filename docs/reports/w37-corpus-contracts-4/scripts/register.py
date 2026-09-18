#!/usr/bin/env python3
"""Card W37-CORPUS-CONTRACTS-4: register this card's residuals in the matrix.

`named_followups` is `hand_maintained` per the matrix's own
`section_ownership`: waves add entries to the committed matrix and the
generator carries them. W36 named nine residuals in its report alone and
registered none, which cost the wave-36 close a registration pass; this card
registers every one it introduces.

Refuses to write if any id already exists, so a collision is reported rather
than merged silently.
"""
import json
import pathlib
import sys

SRC = "docs/reports/w37-corpus-contracts-4/README.md"

NEW = [
    (
        "LOADER-ABI-INLINE-FNPTR-DECLARATOR",
        "The declared-ABI reader parses a function-pointer TYPEDEF but not an INLINE function-pointer parameter declarator, and withdraws the declaration",
        "Card W37-CORPUS-CONTRACTS-4 (wave 37) declared `rd_kafka_topic_partition_list_sort`, its highest-yield callee at two projected functions. The loader WITHDREW it in every calling translation unit with `abi-unreadable-declaration`, so both targets stayed Unknown. The cause was diagnosed rather than guessed: `crates/qcore-c-gateway/src/legacy_declared_abi.rs` records that card CONSTRUCTOR-RESIDUALS-ABI-TYPEDEF taught the reader the function-pointer TYPEDEF shape, which is exactly why `msgpack_packer_init`'s `callback: fnptr` works (its C parameter is `msgpack_packer_write callback`, a named typedef, at lib/msgpack-c/include/msgpack/pack.h:36 and :43). This callee is spelled with an INLINE declarator and no typedef: `int (*cmp)(const void *a, const void *b, void *cmp_opaque)` at lib/librdkafka-2.15.0/src/rdkafka.h:1163-1166. The reader fails CLOSED, which is constraint 8 working; this is the first measurement of that boundary from production text. The withdrawal is LEAF-NEUTRAL - all 11 rows are same-position substitutions and no function's leaf count changed - which is why the module was kept rather than reverted: reverting it would delete the only evidence for this residual.",
        "Extend the parameter declarator reader to the inline function-pointer form, on CONSTRUCTOR-RESIDUALS-ABI-TYPEDEF's own precedent. The projection it needs already exists (FUNCTION_POINTER_ABI_TYPE), so no new vocabulary is required.",
    ),
    (
        "RDKAFKA-SORT-WITHDRAWN",
        "Two projected functions stay Unknown because their declaration is withdrawn, at zero leaf cost",
        "rd_kafka_topic_partition_list_sort_by_topic and rd_kafka_topic_partition_list_sort_by_topic_id (lib/librdkafka-2.15.0/src/rdkafka_partition.c:3675-3687) each hold exactly one leaf naming rd_kafka_topic_partition_list_sort. Card W37-CORPUS-CONTRACTS-4 declared that callee and the loader withdrew the declaration (see LOADER-ABI-INLINE-FNPTR-DECLARATOR), so both functions stayed Unknown against a prediction of Safe. The declaration itself was read line-cited and is faithful - all three formals borrowed, result none, permuting an array the caller owns whole changes no custody - so nothing about the reading is in doubt; only the reader's coverage is.",
        "Discharging LOADER-ABI-INLINE-FNPTR-DECLARATOR should move both functions to Safe with no other change, which is a sharp test of that repair.",
    ),
    (
        "W37-COVER-FRONTIER-IS-STATIC",
        "After wave 36's harvest the sole-blocker cover frontier is dominated by unit-internal `static` helpers, where declaration is structurally the wrong lever",
        "Card W37-CORPUS-CONTRACTS-4 (wave 37) re-derived the cover and surveyed 22 candidate callees for linkage, headers and definition counts. Ten are `static`/`static inline` DEFINED IN THE VERY UNIT THAT CALLS THEM (rd_kafka_topic_partition_destroy0, rd_kafka_destroy_app, sds_cat_safe, set_result, otlp_kvpair_list_destroy, destroy_context_list, crc32c_zeros, pack_kvlist, pack_context_metrics, rd_kafka_dump0) and three more have several definitions with DISAGREEING signatures. A resource contract is keyed by NAME, so for these a declaration is either meaningless (no extern C declaration for constraint 8's ABI gate to agree with) or wrong (it would speak for a neighbour's shape), and constraint 4's division of labour puts a body defined in its own calling unit on W4's side. This is why the card lands ten CUSTODY-NEUTRAL declarations and not one release - the exact inverse of W36's five releases plus one neutral - and the asymmetry is recorded in compiler/tests/contract_loader.rs's own pinned-table comment so it survives as a fact rather than as report rhetoric.",
        "A W4-directed card attacking the same functions through summary derivation, or acceptance as a measured boundary on how far the declaration lever reaches in this corpus.",
    ),
    (
        "NESTED-CALL-IN-ARGUMENT-POSITION",
        "A call nested in a modeled call's argument list is refused, and this is now the commonest way a decided leaf is exchanged rather than discharged",
        "Generalises CJSON-DELETE-NESTED-DETACH (wave 36) to a third library. Card W37-CORPUS-CONTRACTS-4 PREDICTED one instance ex ante and was exactly right: pack_string (lib/ctraces/src/ctr_encode_msgpack.c:35-38) is `mpack_write_str(writer, str, cfl_sds_len(str));`, and cfl_sds_len is a real external function (lib/cfl/src/cfl_sds.c:193, header lib/cfl/include/cfl/cfl_sds.h:69) that emits no leaf today only because the outer call refuses first. Once mpack_write_str was modeled the projection reached the inner call and refused there. The card also found two unlooked-for instances in cmt_encode_msgpack.c :: pack_metric (cmt_metric_hist_get_sum_value, cmt_metric_get_int64_value). Reading the target body ex ante converts W36-COVER-HIDDEN-BLOCKER from a surprise into a prediction, which is the method this residual recommends to any later cover card.",
        "Admit the nested-call-in-argument-position shape in the resource projection, or declare/derive the inner callees. Note that declaring the inner callee alone does NOT help: the refusal is positional, not about the inner callee being uncontracted.",
    ),
    (
        "CTRACES-FORMATTER-CHAIN",
        "Four ctraces formatters are blocked behind a `static` helper that becomes W4-derivable once two external callees are declared",
        "sds_cat_safe (lib/ctraces/src/ctr_encode_text.c:22) is `static inline`, has exactly one definition in the tree, and is defined in the unit that calls it, so card W37-CORPUS-CONTRACTS-4 declined to declare it (W37-COVER-FRONTIER-IS-STATIC). It blocks format_bool, format_double, format_int64 and format_string. Its own body calls only strlen and cfl_sds_cat_safe, both external and header-declared; W4 cannot derive a summary for it while those are uncontracted. The card's greedy cover measured that snprintf plus sds_cat_safe would free five functions where sole-blocker counting sees one, which is the set-yield effect a per-callee ranking misses.",
        "Declare strlen and cfl_sds_cat_safe (and, for three of the four, snprintf - see LIBC-PRINTF-FAMILY), then re-measure whether W4 derives sds_cat_safe and the four formatters move.",
    ),
    (
        "LIBC-PRINTF-FAMILY",
        "snprintf/sprintf/fprintf/vsnprintf are faithful in v1 today and carry ~3,200 leaves at near-zero function-cover yield: a LEAF-directed card, not a cover card",
        "Card W37-CORPUS-CONTRACTS-4 read them and declined them deliberately, on blast radius rather than faithfulness. There is NO vocabulary gap, and the reason is the exact inverse of LIBC-MEM-RESULT-ALIAS: their result is `int`, a SCALAR, so resource_return_none() is literally faithful rather than a conservative approximation; their buffer parameter is written through, which msgpack_packer_init's `pk` already establishes as borrowed; and their variadic tail is mk_print's resource_variadic_data_tail(). snprintf alone carries 1,044 leaves against a function-cover yield of ONE (rd_kafka_mk_nodename, itself forecast to exchange - its tail passes a `const char *` formal, the shape MK-PRINT-TAIL-ACTUALS measured refusing at 26 sites); sprintf's single target cJSON_Version returns a `static` local array, an independent exchange risk. Deciding 1,044 leaves inside a cover arbiter whose other declarations must each be attributable leaf by leaf would have made that card's movement unreadable, for one function.",
        "A dedicated leaf-directed card landing the family together with its own leaf-by-leaf attribution, rather than smuggling one member into a cover card.",
    ),
    (
        "LOADER-ABI-RESTRICT-QUALIFIER",
        "Narrows LIBC-MEM-RESULT-ALIAS's ABI half: glibc DECORATION is already exercised in production; what is untested is specifically `__restrict` on a pointer parameter",
        "Card W36-CORPUS-CONTRACTS-3 declined memcpy/memset on two grounds, one of them that surviving glibc's decorated prototypes is an untested ABI path. Card W37-CORPUS-CONTRACTS-4 re-read that ground and found it narrower than recorded: libc/memory.qcore already declares malloc, whose glibc prototype is `extern void *malloc (size_t __size) __THROW __attribute_malloc__ __attribute_alloc_size__ ((1)) __wur;` (/usr/include/stdlib.h:676), manifest-selected and NOT withdrawn, so the decoration class as such is exercised. What memcpy/memset add over malloc/free is specifically `__restrict` on the pointer parameters (/usr/include/string.h:46-48 and :65), which no corpus declaration has yet met. The RESULT-MODE half of LIBC-MEM-RESULT-ALIAS is untouched and still stands.",
        "One measurement: declare any `__restrict`-qualified libc callee and observe whether the declared-ABI reader withdraws it.",
    ),
    (
        "CONDITIONAL-RELEASE-BY-LITERAL",
        "A callee that releases its parameter only when a second, literal argument is nonzero cannot be declared in v1 without conditioning on that literal",
        "rd_kafka_topic_partition_destroy0 (lib/librdkafka-2.15.0/src/rdkafka_partition.c:2969-2982) ends `if (do_free) rd_free(rktpar);`. Its two target call sites (:3105, :3109) pass 1/rd_true, but five further sites in the same unit (:3098, :3122, :3600) pass 0. A contract is per-NAME and not per-site, so declaring `released` would be FALSE at those five sites and would move in the admitting direction, which invariant 1 forbids. Card W37-CORPUS-CONTRACTS-4 therefore declined it - also, independently, because it is `static` with no header declaration inside its own calling unit (W37-COVER-FRONTIER-IS-STATIC). The corpus does carry a `resource_literal_argument(PARAMETER, LITERAL)` clause that could in principle condition a contract on `do_free == 1`, which is why this is recorded as an avenue rather than as an impossibility.",
        "A literal-conditioned release contract exercised on a real callee, or acceptance that W4 is the lever for unit-internal statics and this shape needs no declaration vocabulary.",
    ),
    (
        "CPROFILES-STRAY-DUPLICATE-TREE",
        "lib/cprofiles/*.c duplicates lib/cprofiles/src/*.c and the two copies are NOT identical",
        "Found incidentally by card W37-CORPUS-CONTRACTS-4's definition-count survey: lib/cprofiles/cprof_encode_opentelemetry.c and lib/cprofiles/src/cprof_encode_opentelemetry.c are both present and differ in size (92,051 vs 82,980 bytes), and the same stray-duplicate pattern applies to the whole lib/cprofiles/*.c set. Nothing in that card depends on it and no claim is made about which tree the pinned build compiles. It is recorded because a resource contract is keyed by NAME, so a future declaration naming anything defined in that library would meet two candidate definitions that disagree - the ADD-NAME-COLLISION hazard, latent.",
        "Determine which of the two trees the pinned compile database actually builds, and record it beside the corpus conventions.",
    ),
    (
        "MPACK-WRITER-INTERIOR-REALLOC",
        "A growable mpack writer's flush callback reallocates writer->buffer, so interior addresses are unstable across the declared calls",
        "Recorded by card W37-CORPUS-CONTRACTS-4 in contract/qcore/mpack/writer.qcore's own NOT CLAIMED list rather than hidden. Under mpack_writer_init_growable the flush callback reallocs writer->buffer, so a byte address inside the writer is not stable across mpack_write_cstr/str/i64/double. That is a fact about a FIELD of the borrowed object, not about the parameter's custody: the caller holds the mpack_writer_t *, that pointer is unchanged, and the caller's mpack_writer_destroy obligation is unchanged, which is all the declaration states. The v1 projection models parameter custody and has no interior-address vocabulary at all.",
        "Acceptance as a stated boundary of the v1 projection, or an interior-address model - none exists today and none is proposed here.",
    ),
]


def main() -> int:
    p = pathlib.Path("docs/reviews/r0_3/acceptance_matrix.json")
    d = json.loads(p.read_text())
    have = {e["id"] for e in d["named_followups"]}
    before = len(d["named_followups"])
    collisions = [i for i, *_ in NEW if i in have]
    if collisions:
        print(f"REFUSING: ids already present: {collisions}", file=sys.stderr)
        return 1
    for ident, title, why, discharge in NEW:
        d["named_followups"].append(
            {
                "id": ident,
                "title": title,
                "why": why + " Discharge condition: " + discharge,
                "source": SRC,
            }
        )
    p.write_text(json.dumps(d, indent=1, ensure_ascii=False) + "\n")
    print(f"named_followups {before} -> {len(d['named_followups'])} (+{len(NEW)})")
    return 0


if __name__ == "__main__":
    sys.exit(main())
