#!/usr/bin/env python3
"""Register card STABILITY-REEVAL-5's findings in `named_followups`, once.

`named_followups` is hand-maintained (the generator carries it verbatim), so
this appends in place, on the `stability-reeval-{1..4}` precedent, and then
the caller regenerates the derived sections:

    python3 -B docs/reports/remaining-d02/build_matrix.py . \
        --gate docs/reports/w45-audit-subcommand-plugins/integration/gate.json

(the gate file the committed validation block already names, so that block
stays byte-identical; this card's own gate, with equal figures, is
docs/reports/stability-reeval-5/gate.json).

It retires nothing: no registered entry was measured discharged by this card.
It amends exactly one entry, GATE-SUMMARY-COUNT-LOOSE-GREP-ARTIFACT (this
cadence's own, firing #4), by appending a dated recurrence sentence.

Every new entry carries the five keys (`id`, `title`, `why`, `source`,
`recorded_by`). Idempotent: an id already present is skipped, and the
amendment is skipped when its marker is already in the `why`.
`--show` prints what would change and writes nothing.
"""

import json
import pathlib
import sys

ROOT = pathlib.Path(__file__).resolve().parents[4]
MATRIX = ROOT / "docs/reviews/r0_3/acceptance_matrix.json"
REPORT = "docs/reports/stability-reeval-5/README.md"
CARD = "STABILITY-REEVAL-5"


def entry(id_, title, why):
    return {"id": id_, "title": title, "why": why, "source": REPORT, "recorded_by": CARD}


M = "MEASURED by card STABILITY-REEVAL-5 (wave-45 close, base 8c1f15eb)"
I = "INFERRED by card STABILITY-REEVAL-5 (wave-45 close, base 8c1f15eb) from code read at the cited lines, no fixture run"
P = ("MEASURED by card STABILITY-REEVAL-5 (wave-45 close, base 8c1f15eb) with a one-off probe over the synthetic "
     "q_*/r_* test contracts (docs/reports/stability-reeval-5/probes/, run once and removed; no test landed)")

NEW = [
    # ---- Gateway soundness (steps 1 and 4) ----
    entry(
        "MEMBER-RELEASE-BEFORE-SUMMARY-RELEASE",
        "A member release followed by a callee whose summary releases the same member again is Safe (rule 5k)",
        f"{P}: with the unit-test destructor `node_destroy` (`if (n->buf) q_nullable_free(n->buf); q_nullable_free(n);`), "
        "`void subject(struct node *n) { q_nullable_free(n->buf); node_destroy(n); }` is Safe at function level AND as the "
        "unit verdict of analyze_unrestricted_c11 (probe rows 5k-member-then-destroy, 5k-unit-member-then-destroy) although "
        "`n->buf` is released twice. Mechanism (read): P4's R(B) accepts any callee whose contract in force releases its only "
        "parameter (crates/qcore-c-gateway/src/legacy_member_release.rs:377 releases_its_only_parameter, used at :423-430 and :473-481), and derived summaries "
        "are in that environment (legacy_summary.rs:274-275); the call-site guard (:300-327) refuses only a tracked "
        "sub-custody. tests/unit/legacy_member_release.rs:201 pins only the opposite order. Lean MemberRelease.lean:222 is "
        "over one body's occurrence list. This is the admitting direction (invariant 1); corpus reach unmeasured. "
        "DISCHARGE: the probe's shape pinned != Safe by a test, and P4's R restricted to a callee that does not itself "
        "release the member (a declared contract, or a callee outside member_releasing_functions), with the Lean model "
        "carrying the interprocedural case.",
    ),
    entry(
        "PUBLICATION-ROOT-RELEASED",
        "A body that publishes into a formal's container and then releases that formal is Safe; container teardown is an unstated premise (10d/10f)",
        f"{P}: `void subject(struct queue *q) {{ struct item *e = q_alloc(); mk_list_add(&e->_head, &q->items); q_free(q); }}` "
        "is Safe (probe row 10d-root-released-after-publication): the element's custody moved into a container whose root "
        "the same body then releases. A CallerFormal root is stable only lexically (legacy_container_publication.rs:187-197); "
        "Lean ContainerPublication has no lifetime and ContainerRootReach's `lifetime` is true by definition (:57, :81). "
        "Whether this is a leak depends on a container-teardown premise no rule states. DISCHARGE: either state the premise "
        "in gateway 10d with its ground, or withdraw published sites when a release (direct or by summary) reaches their "
        "root, with the probe's shape pinned by a test either way.",
    ),
    entry(
        "COUNTED-USER-CONDITIONAL-OPERAND",
        "The counted-user walker applies calls and counter writes behind ?:, && and || on every path, and reads f(a)(b) as non-opaque (10e)",
        f"{I}: Walker::expression (crates/qcore-c-gateway/src/legacy_counted_users.rs:1113-1160, :1162-1215) ignores "
        "conditional operators; opaque_callee (:776-795) treats `) (` after an argument list as not opaque and its unit test "
        "pins `f ( a ) ( b )` so (:1753-1754), against 10c's rule 2. verify_declared accepts a declaration at callers on "
        "records only (:500-540), so a false Held would hide a refutation from 10g. (The custody classifier's expression "
        "statements were probed and refuse these operators: probe rows 10c-ternary/-and/-or are Unknown.) DISCHARGE: the "
        "walker undecides a call or counter write behind a top-level ?:, && or ||, f(a)(b) is opaque, and a fixture "
        "`u_ref(t); c ? u_unref(t,0) : (void)0; return 0;` under resource_user_exit is not Held.",
    ),
    entry(
        "CONTAINER-OF-FORMAL-PUBLICATION-LAUNDER",
        "Publishing an element bound by container_of over a formal skips the formal-publication rule (10d with 5l(2))",
        f"{I}: 5l(2) binds `ctx = container_of(arg, ...)` over a never-bound formal and leaves ctx unbound "
        "(legacy_list_iteration.rs:947-957); a 10d publication of `&ctx->_head` reaches `Ok(None) => continue` "
        "(legacy_container_publication.rs:518-519) because the formal check reads only the entry's own spelling (:405), so "
        "the summary can say `arg` Borrowed while it is linked - the launder 10d forbids for a literal formal. DISCHARGE: a "
        "publication whose entry 5l bound over a formal is treated as that formal (Consumed/UnknownAlias) or refused, with a "
        "fixture (a link helper over `void *arg` and a caller that frees after linking) pinned != Safe.",
    ),
    entry(
        "DECLARED-NEUTRAL-UNVERIFIED-AT-CALL",
        "A declared resource_user_neutral() is applied at call sites before, and without, checking the callee's analysed body",
        f"{M} by reading crates/qcore-c-gateway/src/legacy_counted_users.rs:645-647 (neutral returned before the body is "
        "examined) and :1283-1288 (verify_declared runs for effects and exits only); the defining row records Undecided "
        "(:432-446), which 10g does not withdraw. Narrower than W45-UNCHECKED-DECLARATIONS-TRUSTED: neutrality is not "
        "undecided by the body, it is never compared with it. In the Fluent Bit slice the provider declares neutral for "
        "flb_task_retry_destroy and flb_output_thread_pool_flush (task_users.qcore:81-108). DISCHARGE: declared neutrality "
        "is routed through verify_declared where the unit defines the body, with a fixture whose body writes the counter "
        "and whose caller clause is then not Held.",
    ),
    entry(
        "SCALAR-READ-PARENTHESISED-DOUBLE-DEREF",
        "Rule 5j's scalar read may admit (*q)->len, (*q)[0] and **q as one dereference",
        f"{I}: tracing legacy_return_family.rs:402-447, a starred root in parentheses followed by -> or [, or a repeated "
        "unary *, passes as one dereference with base q, while the equivalent *q->len, q[0]->len and q->next->len are "
        "refused (pinned at tests/unit/legacy_return_family.rs:109-114). DISCHARGE: unit pins asserting None for the three "
        "shapes, and the reader refuses them if any is admitted.",
    ),
    entry(
        "GATEWAY-5L-ELEMENT-TRACKED-AFTER-FOREIGN-UNLINK",
        "Rule 5l's premise that an iterated element is not a tracked identity is stale since the foreign unlink (10f) binds it",
        f"{I}: docs/agents/gateway.md:426-429 reads that an element of a borrowed container is untracked and refuses on its "
        "own; W45-CONTAINER-ROOT-REACH's take_foreign_custody binds the element at a fresh site "
        "(legacy_container_publication.rs:627-640), which 5l's lexical guard does not see. (The two-name alias of one "
        "foreign element was probed and is Unknown, UnknownAlias f/e: probe row 10f-two-names.) DISCHARGE: 5l's text "
        "revised to the post-10f state, and a fixture that re-publishes a foreign-unlinked element into the iterated list "
        "under a different head spelling pinned != Safe.",
    ),
    entry(
        "STATIC-ROOT-SHADOW-READER",
        "10f's static-root set is a lexical reader that skips a shadowing declaration containing '(' (low)",
        f"{I}: statics come from the CTWIN-PUBLISH destination reader (legacy_c.rs:2907; publication.rs:475-489), which "
        "skips a block-scope declaration containing '(' (publication.rs:169), so a global shadowed by such a declaration "
        "could be read as a static root. Masked today by the classifier refusing __attribute__/_Alignas/typeof. "
        "DISCHARGE: a fixture shadowing a global inside a declaration containing '(' pinned != Safe, or the reader refuses "
        "the unit.",
    ),
    entry(
        "GATEWAY-W45-REFUSAL-ARMS-UNREACHED",
        "Wave-45 gateway refusal arms that no fixture drives (10d, 10e, 5l) and one test whose premise drifted",
        "Arm table in STABILITY-REEVAL-5's report section 2.3, attributed to its gateway sub-audit and spot-checked "
        "(PATH_BOUND at legacy_counted_users.rs:1003-1004 has no test naming it). Undriven by message: 10e 'unattributed "
        "counted-user operation' (:1311, :1418), 'an exit no clause covers' (:1692, :1703), clauses 'this position does not "
        "apply' (:654), 'recursive call' (:669), 'verified through itself' (:507), PATH_BOUND (:1004), 'a direct counter "
        "write' (:1187), the jump/label/switch/unbalanced arms (:877-949, only `for` unit-tested); 10d 'projection cannot "
        "name' (legacy_container_publication.rs:487-498) and the four container-effect guards on borrowed-identity routes "
        "(neutral_position.rs:376, variadic_nested.rs:213, legacy_condition_desugar.rs:430,443, legacy_c.rs:6411); 5l "
        "checked-spec Unreadable (legacy_checked_spec.rs:56). All fail closed by reading; deleting any turns no test red. "
        "tests/refused_call_hand_on.rs:113 is named a local-container case but its refusal comes from nullability. "
        "DISCHARGE: one fixture per arm asserting its reason text, and the :113 test renamed or re-aimed.",
    ),
    entry(
        "CONDITION-DESUGAR-REFUSAL-ARMS-UNPINNED",
        "Six W44 condition-desugar refusal variants have no test, and six 'unexplained' residual leaves are unreported",
        f"{M}: the variants IndirectCallee, Braces, AssignmentDestination, ResidualRefused, CallFreeLeafRefused and "
        "SwitchShape (crates/qcore-c-gateway/src/legacy_condition_desugar.rs:140-184, returned at :538, :554, :589, "
        ":718-730, :808) have reason keys named in no file outside that source (rg over crates, compiler, tests). "
        "docs/reports/w44-branch-condition-calls/residuals.txt:17-21 lists 6 unexplained leaves in 5 functions that the "
        "card's README does not mention. Each falls back to the pre-existing refusal (fail-closed). DISCHARGE: one test per "
        "variant, and the 6 leaves explained or registered.",
    ),
    # ---- Pending and plugins (steps 1 and 2) ----
    entry(
        "HELD-PREFIX-SOURCE-FORGES-PENDING",
        "A source that itself spells a held-call prefix publishes Pending with a human-resolvable reason instead of Unknown",
        f"{P}: `qcore_pin_mismatch__zz(0);` and `qcore_review_pending__zz(0);` are Pending (reasons contract-pin-mismatch, "
        "awaiting-human-review) where the control `zz(0);` is Unknown, and a unit holding only the first is a Pending unit "
        "verdict (probe rows held-pin-prefix, held-review-prefix, unit-control-held-alone). The held classification is "
        "lexical: legacy_c.rs:7156-7171 and :7641-7654 test only the callee prefix (legacy_pin_hold.rs:40, "
        "legacy_review_hold.rs:61), and legacy_verdict.rs:76-78 maps the description to Pending; nothing checks membership "
        "in the withheld set. The comment at legacy_pin_hold.rs:37-39 ('which is Unknown') and W45-REVIEW-PREFIX-COLLISION-"
        "UNPINNED's 'so no verdict moves' became false when W45-USER-CONTRACT-PLUGINS made held calls Pending. Never Safe. "
        "DISCHARGE: a source identifier carrying a held prefix the rewrite did not produce is Unknown (or the held "
        "description is emitted only for a member of the withheld set), both texts corrected, and the probe's rows pinned.",
    ),
    entry(
        "PENDING-BYPASSES-UNIT-SAFE-GATES",
        "A Pending unit skips the Safe arm's unread-definition, certificate-scope and inventory gates",
        f"{I}: crates/qcore-c-gateway/src/legacy_c.rs:2111-2122 publishes Unknown|Pending through the Unknown variant, and "
        "legacy_verdict.rs:124-127 reads Pending back from the summary; the unread selected-definition gate (:2153-2170, "
        "whose comment :2145-2147 predates Pending), the certificate target scope (:2171-2186) and the audit's inventory "
        "withholding (projection.rs:419-429) run on the Safe arm only. A unit with a held call and an unread definition is "
        "then Pending although a re-pin could not make it Safe, against spec §L21.1 ('withheld for one of them alone'). The "
        "probe could not engage the gate: analyze_unrestricted_c11 without a selection gives Safe for an `#if 0` "
        "definition too (probe rows unit-control-if0-safe, unit-if0-plus-held). DISCHARGE: the same gates applied to a "
        "Pending unit (any failure Unknown), with a fixture of a held call beside an unread selected definition on the "
        "audit route.",
    ),
    entry(
        "PLUGIN-UNSAFE-MISBOUND-EVIDENCE-PENDING",
        "An Unsafe plugin claim whose evidence does not bind is Pending, not Unknown, when no counterexample checker exists",
        f"{M} by reading crates/qcore-verifier-host/src/plugin_firewall.rs:218-241: the no-witness and no-checker returns "
        "(UnvalidatedUnsafe, Pending) come before check_envelope (:236), against the module's own table row 'evidence that "
        "does not bind ... Unknown' (:18) and the Safe path, where binding (QP009) precedes QP016. No production "
        "counterexample checker exists, so every misbound Unsafe is Pending today. Related: QP016 'core derived no "
        "obligations' is classified uncertified (Pending) though no certificate can resolve an empty inventory. Never "
        "Safe. DISCHARGE: check_envelope before the checker-absence test for Unsafe and empty-inventory QP016 to Unknown, "
        "with a firewall test that a misbound Unsafe without a checker is Unknown.",
    ),
    entry(
        "PENDING-UNTYPED-OUTSIDE-REASONEDVERDICT",
        "Spec §L21.2's 'no other Pending can be constructed' holds for ReasonedVerdict only; the gateway and the read-back paths use a bare Verdict::Pending",
        f"{M}: docs/QCORE_SPEC_r0_3.md:1295 says ReasonedVerdict::pending takes a PendingReason 'so no other Pending can be "
        "constructed'. crates/qcore-schemas/src/verdict.rs:51-57 derives Deserialize on the bare Verdict and Verdict::Pending "
        "is public; no production serde-deserializes it (rg), but the gateway emits a bare Pending from a description string "
        "(legacy_verdict.rs:76-78), and checkpoint.rs:443-449 (unit_verdict), compose.rs, user_plugins.rs:459-462 and "
        "revision_compare.rs:188-191 accept a status 'Pending' with no reason code, while subcommand_plugins.rs:616-623 "
        "refuses one (Unknown). Only the non-Safe direction is affected; HELD-PREFIX-SOURCE-FORGES-PENDING is the measured "
        "consequence. DISCHARGE: the held diagnostic carries a typed PendingReason and every read-back requires a "
        "human-resolvable reason code beside Pending (or states .qo integrity as the premise), or the spec sentence is "
        "narrowed to ReasonedVerdict with the gateway's lexical premise stated in QCore.VerdictSemigroup's unproved list.",
    ),
    entry(
        "PIN-NOT-IN-LOCK-ARM-UNREACHED",
        "Revision mode's NotInLock arms, two pin --check stale arms and the audit route's unknown lock version are undriven",
        f"{M} (rg for NotInLock/not-in-lock over tests finds none; attributed to the card's Pending sub-audit and spot-"
        "checked): no lock entry for a pinned function (crates/qcore-audit/src/contract_revision.rs:269-274) and an entry "
        "with `identities: []`, which validate does not reject (:287-292); pin --check's '~ flexible, not listed' and '- in "
        "the lock but no longer declared' (contract_pin.rs:529-553); an unknown lock_version on the audit route (only pin "
        "--check drives it). All withhold or refuse by reading. DISCHARGE: fixtures for each.",
    ),
    entry(
        "SUBCOMMAND-FAILURE-ARMS-UNREACHED",
        "Three quirin-audit subcommand failure arms have no host-level test",
        f"{M} (rg over crates/qcore-audit/tests/subcommand_plugins.rs and crates/quirin-audit-plugin/tests): plugin-spawn "
        "(crates/qcore-audit/src/subcommand_plugins.rs:942), plugin-escaped-descendant (:977-987) and an exit-0 "
        "plugin-reported-failure (:1017-1019). Each maps to Unknown by reading. DISCHARGE: a test per arm asserting "
        "Unknown and that its claims join nothing.",
    ),
    # ---- Checkpoints and rule 10g (steps 1, 2, 4) ----
    entry(
        "CHECKPOINT-REFUTED-DIGEST-MISMATCH-UNDRIVEN",
        "No test drives a .qo identity mismatch on the refuted-table digest, nor most other resume and compose refusal arms",
        f"{M}: the digest is in the identity (crates/qcore-audit/src/checkpoint.rs:191-196, filled checkpoint_run.rs:396-398, "
        "compared :530; schema 2 at checkpoint.rs:47-49), but W45-REFUTED-CONTRACT-WITHDRAWAL changed "
        "crates/qcore-audit/tests/checkpoint_cli.rs only in its schema string and no test names refuted/run_two_phases; the "
        "slice evidence ran without --resume. Also undriven (sub-audit grep, 0 hits each): resume 'group label differs', "
        "'group-wide refusal differs', 'unit count differs', unit file/preparation, every run_difference field but the "
        "contract set; compose 'outside the plan', 'sits at the plan position', 'run-wide publication inputs', 'groups must "
        "be disjoint', 'no checkpoint to compose'; the census refusal, 'checkout HEAD changed' and a schema-1 file on "
        "resume. DISCHARGE: a checkpoint_cli fixture with a self-refuting body showing phase-1 and phase-2 .qo digests "
        "differ, --resume recomputing on the mismatch and a mixed-phase compose refused, plus one fixture per listed arm.",
    ),
    entry(
        "CHECKPOINT-TWO-PHASE-RESUME-NEVER-REUSES",
        "Both phases of a checkpointed two-phase run write the same .qo paths, so a resumed run with a non-empty table reuses nothing",
        f"{M} by reading crates/qcore-audit/src/checkpoint_run.rs:117 (path = directory/label.qo) and "
        "current_route.rs:273-279 (checkpoint_run::run called twice with the same options): phase 2 overwrites phase 1's "
        "seals, so on --resume each phase finds the other's table digest and recomputes. "
        "docs/development/audit-checkpoints.md:134-136 ('a resumed second phase reuses only groups sealed under the same "
        "table') is true only vacuously. Cost, not soundness. DISCHARGE: phase-qualified checkpoint paths with a test "
        "showing phase-2 reuse on a no-change resume under a non-empty table, or the document corrected.",
    ),
    entry(
        "CHECKPOINT-PHASE-ONE-REPORT-PUBLISHED",
        "In checkpoint mode a phase-2 failure leaves phase 1's composed report at the published path",
        f"{M} by reading crates/qcore-audit/src/checkpoint_run.rs:169-180 (every run composes into route.diagnostic_report) "
        "and current_route.rs:273-279: phase 1 publishes before phase 2 runs; if phase 2 then errs (e.g. 'checkout HEAD "
        "changed', an IO or compose refusal) the route exits nonzero with a report in which callers of a refuted "
        "declaration are not held and may read Safe. The whole-run route writes nothing on error (current_route.rs:323). "
        "Recognisable only from the report's refuted_contracts section. INFERRED consequence; no fixture. DISCHARGE: "
        "phase 1 composes to a temporary path and only the final phase renames into place (or a phase-2 failure removes "
        "the report), with a test forcing a phase-2 error and asserting no report exists.",
    ),
    entry(
        "REFUTED-TABLE-FIXPOINT-UNCHECKED",
        "Rule 10g's second phase is not checked to publish no refutation outside the table it applied",
        f"{M}: the premise 'a held call can only make a refutation undecided, never create one' is stated unproved "
        "(docs/agents/gateway.md:877-879; crates/qcore-audit/src/refuted_contracts.rs:177-183); run_two_phases (:108-125) "
        "stops after phase 2, the checkpoint route discards phase 2's own table (current_route.rs:279 `.map(|(failed, _)|`), "
        "and the section only publishes published_refutations_sha256 beside table_sha256 (:215-218). The Lean model's "
        "`refutes` does not depend on the table (RefutedContractWithdrawal.lean:155-157). If the premise failed, a "
        "declaration refuted only in phase 2 would keep serving callers, possibly as Safe. DISCHARGE: after phase 2 the "
        "route checks published refutations are a subset of the applied table and otherwise iterates (bounded, Unknown past "
        "the bound) or refuses the run, with a fixture or a Lean theorem bound to the rule.",
    ),
    entry(
        "REFUTED-TWO-PHASE-ROUTE-UNTESTED",
        "No integration test drives the two-phase route; --jobs independence with a non-empty table is observed on the slice only",
        f"{M}: the two-phase path through current_route/checkpoint_run and compose's applied_table read have unit tests "
        "only (refuted_contracts.rs tests; tests/refuted_contract_withdrawal.rs is gateway-level); jobs 8 vs 1 byte identity "
        "is docs/reports/w45-refuted-contract-withdrawal/artifacts/slice/jobs-independence.json (observation), the Lean "
        "the_table_is_independent_of_unit_order is under the stated byMembership premise. applied_table returns the empty "
        "table when the key is missing (refuted_contracts.rs:243-256) and compose does not cross-check it with the identity "
        "digest. DISCHARGE: a qcore-audit fixture (a self-refuting body and a cross-unit caller) asserting two phases, the "
        "held caller, and byte identity at jobs 1 vs N and whole vs checkpointed.",
    ),
    # ---- Model proof versus observation (step 2) ----
    entry(
        "LEAN-PARITY-TRANSCRIPTION-CENSUS",
        "Of 70 Lean modules added since 6dc484ad, 2 are mechanically bound to Rust, 67 are transcriptions and 1 names no counterpart",
        "MEASURED by card STABILITY-REEVAL-5's Lean-census sub-audit and re-verified on the two mechanical bindings "
        "(compiler/tests/source_scalar_calls.rs:428-430 include_str!s ScalarCalls.lean; compiler/tests/proof_leg_digests.rs:42 "
        "cross-checks Ct50LifetimeIdentity's digest) and on the count (git diff --name-status --diff-filter=A 6dc484ad HEAD "
        "-- proof/QCore: 70). Modified LoopWindows is also mechanical (composed_window_reach.rs:715). 40 transcriptions are "
        "named by a Rust doc comment, 27 only by a report sentence (IndexTraversal and IndirectScalarAccess among them), and "
        "EventObservation by nothing. Only the two Cycle 18 routes disclose 'review-maintained' in their verdict "
        "assumptions (cycle18_callback_obligations.rs:1956-1957, cycle18_composition.rs:1121-1122); no gateway or corpus "
        "verdict does. Successor of the family firing #4 named (LOOPWINDOWS-LEAN-IS-A-TRANSCRIPTION, "
        "CT42-CELL-DERIVER-LEAN-PARITY-UNBOUND, PROOF-LEG-DIGESTS-UNCROSSCHECKED). DISCHARGE: per module, a mechanical binding "
        "(a shared-vector or digest table a Rust test reads) or 'review-maintained correspondence' carried in the verdict "
        "assumptions of every route whose decision the module models.",
    ),
    entry(
        "LEAN-MODELS-TAKE-RECOGNISER-OUTPUT-AS-PREMISE",
        "Lean models take the Rust recogniser's classification as an input flag, so a recogniser hole is invisible to the proof",
        f"{M}: proof/QCore/IndexTraversal.lean:484-508 models the repaired out[m++] hole with `indexWritesOtherwise : Bool` "
        "as an input and proves the refusal by simp; the defect lived in computing that flag from tokens "
        "(crates/qcore-c-gateway/src/typed_field_access.rs:2162-2175), and the introducing card's own Lean module "
        "(IndirectScalarAccess) did not catch it. AssignmentDestinationSuffix.lean:58-121 models tokens, the level at which "
        "W44's second hole lived. DISCHARGE: a census of the Lean modules whose admission predicate takes a recogniser's "
        "classification as a Bool/enum input, and for recognisers behind Unknown to Safe moves a token-level model or a "
        "shared token-vector table a Rust test reads.",
    ),
    entry(
        "LOOPWINDOWS-LEND-CLASS-MAPPING-UNCHECKED",
        "The LoopWindows agreement test's shared grammar never builds a Lend, Load, Extent or Release event",
        f"{M}: compiler/tests/composed_window_reach.rs:746-790 builds only Store, FieldStore, Bind and Repeat from the Lean "
        "spelling; proof/QCore/LoopWindows.lean's LendEvent collapses the 16 Rust BufferEvent constructors into three "
        "classes by a hand-stated mapping no test checks. Not named by the successor LOOPWINDOWS-BINDING-IS-FINITE. "
        "DISCHARGE: the shared grammar spells at least one leaf per Rust constructor of each class (Lend among them), or the "
        "class mapping is itself a table a Rust test reads.",
    ),
    entry(
        "AUDIT-ROW-ASSUMPTIONS-UNRECORDED",
        "Per-function and per-unit audit rows carry no assumptions: declared libc premises and Fluent Bit reachability premises live only in run-level text",
        f"{M}: the W45 slice's resource-diagnostics.json (docs/reports/w45-fb-retry-lifetime/artifacts/slice-after-"
        "providers-046accf6/) has top-level keys diagnostic_counts, enumeration_meaning, fluent_bit_revision, "
        "line_coordinates, not_projected_meaning, origin_meaning, reason_code_meaning, schema_version, scope, units, and unit "
        "rows carry no assumptions key, although OR-1's provisional Unsafe rests on the synchronous-proxy premise and OR-7's "
        "on unchecked neutrality (W45-FB-CROSS-UNIT-USER-PREMISES); unit assumptions are written only for no-build units "
        "(crates/qcore-audit/src/report.rs:301-304). The corpus's 22 W43 libc moves to Safe rest on declared, unchecked "
        "contracts disclosed at run level only (contracts.json trust string). Hard invariant 2 requires assumptions in every "
        "published result. DISCHARGE: each published row (or a linked sidecar keyed by row) names the premises and declared "
        "contracts it used, or the verdict contract states that a run-level assumption set suffices and every report says "
        "so.",
    ),
    # ---- Semantic edits without a card (step 3) ----
    entry(
        "SIZEOF-TYPE-FOLD-NO-LEAN",
        "Integrator fix 7edfd6b9 changed the sizeof type fold with a unit test but no Lean",
        f"{M}: fold_type_sizes folds a sizeof name only as a typedef the reader read or a tag "
        "(crates/qcore-c-gateway/src/typed_field_access.rs:497; test legacy_call_argument_access.rs:1212; recorded "
        "docs/reports/w44-collection-allocation/integration/integration-confirmation.md:96-116). Refusing direction. No "
        "Lean term: CallArgumentAccess.lean has no sizeof/typedef/tag, SizeofOperand models another route. DISCHARGE: a "
        "Lean definition of the fold admission with a theorem that a non-typedef, non-tag name never folds, audited.",
    ),
    entry(
        "TRAVERSAL-DERIVED-EXIT-STEPPED-ANY-NO-LEAN",
        "Merge b89e0a45 added the rule that an Any-stepped exit is not a published derived result, with test re-pins and no Lean",
        f"{M}: crates/qcore-c-gateway/src/legacy_c.rs:10191-10194 (stepped: Stepped::No | Stepped::Forward), re-pinned by "
        "b79aa07e (derived_result_returned.rs, tests/unit/legacy_derived_summary.rs; wmut and w refused to Safe, x to "
        "Unsafe invalid_release), recorded in docs/reports/w43-index-traversal/integration/integration-confirmation.md:26-80. "
        "IndexTraversal.lean has no exit/summary term and DerivedResultInference.lean no stepped term. DISCHARGE: a Lean "
        "theorem that an Any-stepped exit is never published derived and that a Forward-stepped derived result is interior "
        "after its base.",
    ),
    entry(
        "CHECKOUT-CRLF-DIGEST-ACCEPTANCE-UNSCOPED",
        "Commit fa7ab464 relaxed production integrity digests for CRLF, on some checks Windows-only and on others on every host, with no record naming them",
        f"{M}: crates/qcore-contract/src/bundle.rs:82-92 matches_checkout_digest (Windows only); "
        "compiler/src/verifier_plugins/chunkio/pinned_lifecycle.rs:242 normalises the analysed vendored source on every host "
        "(and :275 Windows only); crates/qcore-audit/src/native_platform_matrix.rs:585-592 historical_validation_digest "
        "normalises on every host. docs/reports/windows-test-portability/README.md mentions only in-memory CRLF handling. No "
        "current impact found (no CR in fixtures or the history record, per the sub-audit). DISCHARGE: a record naming each "
        "relaxed integrity check and its host scope, one consistent rule (cfg!(windows) or stated all-host), and a control "
        "that a CR-altered file is refused where intended.",
    ),
    entry(
        "WINDOWS-F02-RUN-REVISION-UNBOUND",
        "The Windows run that let the F02 pin-agreement test be un-ignored records no revision",
        f"{M}: fa7ab464 re-pinned compiler/tests/windows_threads.rs:31,36 to equal compiler/tests/f02_trace_relation.rs:96-97 "
        "and removed the #[ignore]; docs/reports/windows-test-portability/{windows-f02.log,f02-agreement.log} carry no commit "
        "or tree identity. DISCHARGE: the Windows run re-recorded with its revision (or its tree digest) beside the logs.",
    ),
    entry(
        "FEATURE-GATED-SINK-GUARD-OUTSIDE-STANDING-GATE",
        "The census-sink guard runs only under --features indirect-census in CI, and main was red on it for a stretch of waves 44-45",
        f"{M}: every_declared_sink_forces_one_job (crates/qcore-c-gateway/src/lib.rs, cfg(all(test, "
        "feature=\"indirect-census\")), added by the integrator at fc0fafcb) runs in .github/workflows/ci.yml:37 only; "
        "docs/agents/validation.md:14-20 runs no feature build. Per the step-3 sub-audit, the sink added at 66ce7c7f was "
        "unlisted until 5b7942bb, noticed only by card gates (w45-return-expression-families/README.md:305, "
        "w45-sub-object-stores/README.md:315,:362, w45-callee-summary-cascade/README.md:498-502). DISCHARGE: the standing "
        "gate or runner includes `cargo test -p qcore-c-gateway --features indirect-census --lib`, or the rule says it is "
        "CI-only and a close checks CI.",
    ),
    entry(
        "CARD-TIP-CITATION-SQUASHED-WIP",
        "Wave-45 card reports cite squashed WIP tips that are not ancestors of main, one measuring an arbiter on a tree that differs from the merged one",
        f"{M}: all 44 SHAs in the 21 wave-45 integration/gate.json files are ancestors of HEAD, but "
        "docs/reports/w45-user-contract-plugins/README.md:183-186 and docs/development/increments/wave-45.md:26 cite "
        "62978cf3/32c7ced3/e65226ca/ef560f09 (unreachable, disclosed); the arbiter ran at 62978cf3, which differs from the "
        "final tip in 23 code files, and the README calls the difference formatting, documentation 'and one test-only "
        "clippy fix' though that fix changes a production return type in crates/qcore-verifier-host/src/lib.rs "
        "(check_request_v2 to Box<VerificationOutcome>; no verdict effect found). Same disclosed shape in "
        "w45-contract-deferred-review (3071477f, 5beff34f, ff12f687) and w45-contract-revision-mode (2501709c, 9f02f741). "
        "DISCHARGE: cards cite reachable commits (a retained ref or tag) or archive the tree identity, and the 'test-only' "
        "wording corrected by an appended note.",
    ),
    entry(
        "INTEGRATION-GATE-PATH-RULE-DRIFT",
        "The integration rule names docs/reports/<card>/integration-gate.json; every integration gate since wave 43 is at integration/gate.json",
        f"{M}: docs/agents/operations.md:119 names integration-gate.json; ls finds 54 docs/reports/*/integration/gate.json "
        "and 15 older */integration-gate.json. A close sweep following the rule's path finds none of wave 45's. DISCHARGE: "
        "the rule text, and any close tooling, name the path in use.",
    ),
    entry(
        "STATUS-LATEST-GATE-STALE",
        "STATUS's 'Latest validated gate' still publishes the 2026-09-21 task-context gate, five integration gates later",
        f"{M}: docs/STATUS.md:95-104 cites docs/reports/task-context-contract-bundle/gate.json (5,043 / 165 Lean / 336 "
        "Python / 123 provider); the latest integration gate, docs/reports/w45-audit-subcommand-plugins/integration/gate.json, "
        "records 5,995 / 0 / 11, 207 Lean, 364 Python, 129 provider, and this card's gate reproduced the core figures at "
        "equal code. The same measurement shape as MERGED-GATE-ROW-LEAN-HAND-EDIT: no check ties STATUS to a gate file. "
        "Prose owned by DOCS-COMPACT-9 this wave; not edited here. DISCHARGE: the section names the latest integration gate "
        "and a check (build_matrix.py or a unit test) ties its figures to that gate.json.",
    ),
    entry(
        "WAVE-45-RECORD-ERRATA",
        "Six wave-45 record statements that do not match their own evidence (no verdict effect)",
        f"{M}: (1) w45-audit-subcommand-plugins/integration/gate.json's note attributes the contracts-sidecar "
        "refuted_contracts difference to the new crates' empty table, but the key was introduced by "
        "W45-REFUTED-CONTRACT-WITHDRAWAL after the baseline; (2) w45-user-contract-plugins/README.md:196 and "
        "wave-45.md:26 say the sidecar differs by one meaning string, the integration diff shows two "
        "(contract_revision.meaning, pending_human_review.meaning); (3) w45-fb-retry-lifetime/README.md:34-35 calls OR-7 "
        "'reachable by any threaded output' while its :403-406 says that feasibility is not proved; "
        "(4) docs/reports/loops-on-composed-routes/README.md:179 still cites the removed LoopWindows theorem "
        "the_admitted_body_shapes_are_exactly_these; (5) matrix entry FB-SINGLEPLEX-RETRY-FLUSH-FAULT (C3, observed at "
        "864c3d97) is not amended although w45-fb-retry-lifetime/README.md:83-89 says that code no longer exists as it was "
        "at 046accf6; (6) FB-AUDIT-03-ZERO-PROGRESS (persistent CIO_ERROR) and W45-FB-BACKLOG-TRANSIENT-STICKY-ERROR (one "
        "transient failure suffices) do not cross-reference. DISCHARGE: an appended erratum beside each frozen record "
        "(constraint 21) and the two matrix entries amended by their owners.",
    ),
    # ---- Fluent Bit evidence policy (step 4) ----
    entry(
        "NATIVE-OBSERVATION-UPSTREAM-WORDING",
        "STATUS and two W43 records call local native observations 'upstream double closes' without an observed-local qualifier",
        f"{M}: docs/STATUS.md:44 'two source-confirmed upstream double closes' (the row says observation, not proof, but "
        "not 'not an upstream defect claim'); docs/reports/w43-descriptor-native-check/README.md:71 'from two upstream "
        "double closes' (qualified only by the section preamble :214-217); docs/development/increments/wave-45.md:28 "
        "names mk_http_error's unclosed descriptor as 'the one added leak' without the provisional qualifier its report "
        "and STATUS:25 carry. Every wave-45 Fluent Bit record examined is otherwise qualified. DISCHARGE: the rows read "
        "observed-local with a source-read mechanism, not an upstream defect claim (by appended note where frozen), unless "
        "the user decides to report upstream.",
    ),
    entry(
        "PROVISIONAL-UNSAFE-VALIDATION-STANDARD-UNSTATED",
        "What 'validated' means for a provisional Unsafe (projection path under named premises, or proved feasibility) is written nowhere normative",
        f"{I} from docs/reports/w45-fb-retry-lifetime/README.md:403-406 (OR-1/OR-7 branch feasibility not proved) and "
        "docs/reports/w45-fb-backlog-custody/README.md:475-476 ('a validated reachable violation in the resource "
        "projection') against hard invariant 2 ('Unsafe only for a validated reachable violation') and spec §L20. A user "
        "policy decision. DISCHARGE: spec §L20 or docs/agents/operations.md states the standard.",
    ),
    entry(
        "W45-FB-SYNCHRONOUS-PREMISE-UNEVIDENCED",
        "The premise 'no in-tree output plugin sets FLB_OUTPUT_SYNCHRONOUS' has no archived search",
        f"{M}: asserted at docs/reports/w45-fb-retry-fault-injection/README.md:91-92 ('confirmed by the Codex report') and "
        "for 046accf6 in w45-fb-retry-lifetime/README.md:205-208; rg over docs/reports/fb-system-audit-cards finds no such "
        "statement and no search command or output is archived at either revision. OR-1's provisional Unsafe is stated "
        "under a synchronous-proxy premise that depends on it. DISCHARGE: an archived source search (command, exit, "
        "matches) at 046accf6 and 864c3d97 bound into a report.",
    ),
    entry(
        "EP-CENSUS-PREMISE-ON-UNDECIDED-OBLIGATION",
        "The engine loop census counts EP-L011 bounded under premise PW, which is obligation OB-3 that W45-FB-BACKLOG-CUSTODY left Unknown",
        f"{I}, per the Fluent Bit sub-audit: docs/reports/w45-fb-engine-progress/census/census.json:33 defines PW as OB-3, "
        ":770-785 classifies EP-L011 bounded under it, and docs/reports/w45-fb-backlog-custody/README.md:466 leaves OB-3 "
        "Unknown; the premise text does not say so. No verdict rests on it (the census is measurement only). DISCHARGE: the "
        "census marks PW undecided, or OB-3 is decided.",
    ),
    entry(
        "FB-PLAN-CARDS-UNTRACKED",
        "Two Fluent Bit plan cards are tracked only in plan prose and report follow-ons",
        f"{M}: none of the six card ids in docs/plans/fluent-bit-system-coverage/README.md:60-65 is a named_followups id; "
        "FB-SYSTEM-EXTENSIONS appears only in the plan, FB-BACKLOG-ACCOUNTING only in reports' does_not_close lists; "
        "FB-BACKLOG-CUSTODY's status cell is empty although W45-FB-BACKLOG-CUSTODY merged at 2736f30c. A mention is not "
        "tracking (operations.md). DISCHARGE: the wave-close reconciliation updates the plan's status and registers the two "
        "cards, or records that report follow-ons are their tracker.",
    ),
    # ---- Platform ----
    entry(
        "MACOS-GATE-FAILURES-UNTRACKED",
        "A macOS standing-gate run recorded 13 Rust and 15 Python failures that no follow-up tracks",
        f"{M}: docs/development/increments/wave-45.md:20 (macOS CLI fixture paths, 2026-09-26) records "
        "run_test_binaries.py exit 101 with 5,833 passed / 13 failed (native HTTP/socket tests) and "
        "`python3 -m unittest discover -s scripts/tests` exit 1 with 15 failures ('including macOS path/environment "
        "assumptions'), logs in that host's private/; named_followups has no entry for either (id/title/why search for "
        "macOS, Apple, /private/var, HTTP/socket) and docs/development/platform-evidence.md was last changed 2026-09-19. "
        "DISCHARGE: each failing macOS row classified (platform limit, fixture assumption or defect) in the platform "
        "record, with the fixture ones repaired or registered by name.",
    ),
]

RECURRENCE_ID = "GATE-SUMMARY-COUNT-LOOSE-GREP-ARTIFACT"
RECURRENCE_MARKER = "RECURRED (STABILITY-REEVAL-5)"
RECURRENCE = (
    " RECURRED (STABILITY-REEVAL-5, wave-45 close): commit 8c1f15eb corrected the W45-AUDIT-SUBCOMMAND-PLUGINS "
    "integration gate note to '333 summaries', the loose '^test result' count that also matches the two test names "
    "beginning result_; this card's gate at equal code measured 331 strict summaries, the runner's own totals line, and "
    "the integrator corrected the note to 331 at 33558c67. The process still has no guard: the note and the matrix copy "
    "are hand-summed. DISCHARGE (unchanged in aim, now named as a guard): integration and card gate records take the "
    "summary figure only from scripts/run_test_binaries.py's totals line or summary.json, and build_matrix.py --gate "
    "refuses a note whose summary figure disagrees with a summaries key checked against that file."
)


def main() -> int:
    show = "--show" in sys.argv
    matrix = json.loads(MATRIX.read_text(encoding="utf-8"))
    entries = matrix["named_followups"]
    ids = {e["id"] for e in entries}
    before = len(entries)
    added = []
    for e in NEW:
        if e["id"] in ids:
            print(f"skip (present): {e['id']}")
            continue
        added.append(e["id"])
        if not show:
            entries.append(e)
    amended = False
    for e in entries:
        if e["id"] == RECURRENCE_ID and RECURRENCE_MARKER not in e["why"]:
            amended = True
            if not show:
                e["why"] += RECURRENCE
    if RECURRENCE_ID not in ids:
        raise SystemExit(f"{RECURRENCE_ID} is not registered; refusing to amend")
    print(f"named_followups {before} -> {before + len(added)} (+{len(added)} -0); amended {RECURRENCE_ID}: {amended}")
    for name in added:
        print(f"  + {name}")
    if not show:
        MATRIX.write_text(json.dumps(matrix, indent=1, ensure_ascii=False) + "\n", encoding="utf-8")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
