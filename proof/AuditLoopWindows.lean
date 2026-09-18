import QCore.LoopWindows

-- Card `LOOPS-ON-COMPOSED-ROUTES` (wave 39): a loan's window computed **over
-- iterations**, so a `for` beside a borrow or a lend is admitted where the
-- window covers it — lifting `SLICE-PERMISSIONS-LOOP-WINDOW` (buffer route)
-- and `STRING-CONCAT-LOOP-WINDOW` (composed string route), which refused the
-- loop because the window was computed over top-level statements only.

-- The vocabulary the card adds, printed so that a later change to any of it is
-- visible in this surface rather than only inside a proof.
#print axioms QCore.LoopWindows.copies
#print axioms QCore.LoopWindows.tagFrom
#print axioms QCore.LoopWindows.tagBody
#print axioms QCore.LoopWindows.taggedFrom
#print axioms QCore.LoopWindows.tagged
#print axioms QCore.LoopWindows.extend
#print axioms QCore.LoopWindows.lastUse
#print axioms QCore.LoopWindows.unrollStmt
#print axioms QCore.LoopWindows.unrollStmts
#print axioms QCore.LoopWindows.toItem
#print axioms QCore.LoopWindows.toItems
#print axioms QCore.LoopWindows.flatTagged
#print axioms QCore.LoopWindows.shallowTagged
#print axioms QCore.LoopWindows.lastUseShallow
#print axioms QCore.LoopWindows.bodyOnlyUse
#print axioms QCore.LoopWindows.admittedInBody

-- **Soundness relative to the unrolling.** Every use in the unrolled program
-- lies inside the window; unrolled positions are attributed to non-decreasing
-- top-level indices; and so everything between a loan's opening and any of its
-- uses in the unrolled program is attributed inside the iteration window.
#print axioms QCore.LoopWindows.foldl_extend_ge
#print axioms QCore.LoopWindows.foldl_extend_covers
#print axioms QCore.LoopWindows.every_use_lies_inside_the_window
#print axioms QCore.LoopWindows.tagFrom_origin
#print axioms QCore.LoopWindows.tagBody_origin
#print axioms QCore.LoopWindows.taggedFrom_origin_ge
#print axioms QCore.LoopWindows.pairwise_of_one_origin
#print axioms QCore.LoopWindows.origins_are_monotone
#print axioms QCore.LoopWindows.the_unrolled_window_lies_inside_the_iteration_window

-- **The unrolling is `LOOP-V1`'s.** Forgetting the tags gives the untagged
-- unrolling, and on the buffer route's events that is
-- `QCore.SourceLoop.unrollList` of the same program read as that module's items.
#print axioms QCore.LoopWindows.copies_map
#print axioms QCore.LoopWindows.tagFrom_untagged
#print axioms QCore.LoopWindows.tagBody_untagged
#print axioms QCore.LoopWindows.taggedFrom_untagged
#print axioms QCore.LoopWindows.tagged_forgets_to_the_unrolling
#print axioms QCore.LoopWindows.copies_is_repeatAppend
#print axioms QCore.LoopWindows.toItem_unrolls
#print axioms QCore.LoopWindows.toItems_unroll
#print axioms QCore.LoopWindows.the_unrolling_is_loop_v1s

-- **Equal where there is no loop**: a loop-free program's tagged unrolling is
-- the program with its own indices, so no admitted flat program's window moves.
#print axioms QCore.LoopWindows.the_window_is_exact_over_a_flat_program

-- **Non-vacuous**: the replaced top-level-only computation misses a use inside
-- a loop body — the admitting arm `STABILITY-REEVAL-3` §4 measured and the
-- card's Rust mutation re-exposes.
#print axioms QCore.LoopWindows.a_top_level_only_window_misses_a_body_use

-- **A longer live set refuses no less**: the IR's owner-access, lend-admission
-- and drop decisions are monotone in the live loans in the refusing direction,
-- which is what makes an over-approximation of liveness safe.
#print axioms QCore.LoopWindows.a_longer_live_set_refuses_no_less
#print axioms QCore.LoopWindows.a_longer_live_set_admits_no_more_lends
#print axioms QCore.LoopWindows.a_longer_live_set_blocks_the_drop_no_less

-- What a loop body may hold beside a loan, as one list, so a widening changes a
-- theorem rather than slipping past one.
#print axioms QCore.LoopWindows.the_admitted_body_shapes_are_exactly_these
