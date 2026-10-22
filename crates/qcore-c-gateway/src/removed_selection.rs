//! Card DETACH-RESULT-WRAPPER-COMPOSITION (gateway rule 33): the pure
//! decisions of the checked removed-selection composition.
//!
//! A selection wrapper's selected child is `sel(P, W) = (W < 0) ? NULL :
//! L(P, (size_t)W)`, the value its exactly checked frame computes. The child
//! is a **prerequisite** the caller transports: membership is never derived
//! from borrowed provenance or from the detach representation premise. The
//! G hooks are in `legacy_selection.rs`; `QCore.RemovedSelection` proves the
//! finite composition and binds both tables below by shared vectors. Source
//! extraction and Rust/Lean correspondence remain boundary-28 premises.

/// Facts at the one checked selection lookup point of a wrapper analysed
/// under its selection entry.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MaterializeFacts {
    /// The node is the frame's own lookup call into the reserved item place,
    /// whose parent actual is the entry's owner formal.
    pub selection_node: bool,
    /// The owner binding is one settled, live, non-NULL identity.
    pub parent_live_identity: bool,
    /// No call or write precedes the lookup on any path from entry.
    pub pristine: bool,
    /// The selected child has not already appeared on this path.
    pub fresh: bool,
    /// The ordinary call left its borrowed-from result binding in place and
    /// the path is modeled.
    pub borrowed_result: bool,
}

/// The selected child appears only when every fact holds.
#[must_use]
pub const fn materialize(f: MaterializeFacts) -> bool {
    f.selection_node && f.parent_live_identity && f.pristine && f.fresh && f.borrowed_result
}

/// What the wrapper hands back to its caller.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Disposition {
    /// `F`: the selected child is returned, detached.
    Returned,
    /// `G`: the selected child is released, detached.
    Released,
}

/// The selected child's custody at one exit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Selected {
    /// Never selected on any path reaching this exit.
    Absent,
    Contained,
    Live,
    Escaped,
    Released,
    Mixed,
}

/// What one exit returns.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Returned {
    /// No resource place, or a null constant.
    Nothing,
    /// Exactly the selected child (possibly NULL).
    Selected,
    /// Anything else.
    Other,
}

/// One reachable exit of a validation run.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ExitFacts {
    /// The owner formal is still live and no other formal custody moved.
    pub parent_live: bool,
    /// No caller-held containment, out slot or pending replacement remains.
    pub frame_clear: bool,
    pub selected: Selected,
    pub returned: Returned,
}

/// An exit agrees with the summary. The absent-selection exit relies on the
/// exact frame: it is reached only by the `W < 0` arm, where `sel` is NULL.
#[must_use]
pub const fn accept_exit(disposition: Disposition, f: ExitFacts) -> bool {
    if !f.parent_live || !f.frame_clear {
        return false;
    }
    match disposition {
        Disposition::Returned => matches!(
            (f.selected, f.returned),
            (Selected::Absent, Returned::Nothing) | (Selected::Escaped, Returned::Selected)
        ),
        Disposition::Released => {
            matches!(f.selected, Selected::Absent | Selected::Released)
                && matches!(f.returned, Returned::Nothing)
        }
    }
}

/// A run validates only when it is fully modeled, diagnostic-free, reaches
/// an exit, selects on some path and accepts every exit.
#[must_use]
pub fn validates(
    disposition: Disposition,
    clean: bool,
    exits: &[ExitFacts],
    selected_somewhere: bool,
) -> bool {
    clean
        && selected_somewhere
        && !exits.is_empty()
        && exits.iter().all(|exit| accept_exit(disposition, *exit))
}

/// The selection value the frame computes, over a finite child list.
#[must_use]
pub fn selection(children: &[usize], which: i64) -> Option<usize> {
    if which < 0 {
        return None;
    }
    usize::try_from(which)
        .ok()
        .and_then(|index| crate::list_lookup::lookup(children, index))
}

/// Shared with `QCore.RemovedSelection.materializeVectors`.
pub const MATERIALIZE_VECTORS: [(MaterializeFacts, bool); 6] = [
    (
        MaterializeFacts {
            selection_node: true,
            parent_live_identity: true,
            pristine: true,
            fresh: true,
            borrowed_result: true,
        },
        true,
    ),
    (
        MaterializeFacts {
            selection_node: false,
            parent_live_identity: true,
            pristine: true,
            fresh: true,
            borrowed_result: true,
        },
        false,
    ),
    (
        MaterializeFacts {
            selection_node: true,
            parent_live_identity: false,
            pristine: true,
            fresh: true,
            borrowed_result: true,
        },
        false,
    ),
    (
        MaterializeFacts {
            selection_node: true,
            parent_live_identity: true,
            pristine: false,
            fresh: true,
            borrowed_result: true,
        },
        false,
    ),
    (
        MaterializeFacts {
            selection_node: true,
            parent_live_identity: true,
            pristine: true,
            fresh: false,
            borrowed_result: true,
        },
        false,
    ),
    (
        MaterializeFacts {
            selection_node: true,
            parent_live_identity: true,
            pristine: true,
            fresh: true,
            borrowed_result: false,
        },
        false,
    ),
];

#[cfg(test)]
mod tests {
    use super::*;

    fn lean() -> &'static str {
        include_str!("../../../proof/QCore/RemovedSelection.lean")
    }

    fn table<'a>(source: &'a str, head: &str) -> Vec<&'a str> {
        source
            .split(head)
            .nth(1)
            .expect("vector table")
            .split("\n]")
            .next()
            .unwrap()
            .lines()
            .map(str::trim)
            .filter(|line| line.starts_with('('))
            .collect()
    }

    fn bools(text: &str) -> Vec<bool> {
        text.split(',')
            .map(|value| match value.trim() {
                "true" => true,
                "false" => false,
                other => panic!("{other}"),
            })
            .collect()
    }

    #[test]
    fn lean_materialize_vectors_match_the_rust_decision() {
        let rows = table(
            lean(),
            "def materializeVectors : List (MaterializeFacts × Bool) := [",
        );
        assert_eq!(rows.len(), MATERIALIZE_VECTORS.len());
        for (row, (facts, expected)) in rows.iter().zip(MATERIALIZE_VECTORS) {
            let (inside, result) = row.trim_start_matches("(⟨").split_once("⟩,").unwrap();
            let values = bools(inside);
            assert_eq!(
                values,
                [
                    facts.selection_node,
                    facts.parent_live_identity,
                    facts.pristine,
                    facts.fresh,
                    facts.borrowed_result
                ]
            );
            let result = result.trim().trim_end_matches(',').trim_end_matches(')');
            assert_eq!(result == "true", expected);
            assert_eq!(materialize(facts), expected);
        }
    }

    #[test]
    fn lean_exit_vectors_match_the_rust_decision() {
        let rows = table(
            lean(),
            "def exitVectors : List (Disposition × ExitFacts × Bool) := [",
        );
        assert_eq!(rows.len(), 12);
        for row in rows {
            let body = row.trim_start_matches('(');
            let (disposition, rest) = body.split_once(", ⟨").unwrap();
            let (inside, result) = rest.split_once("⟩,").unwrap();
            let disposition = match disposition.trim() {
                ".returned" => Disposition::Returned,
                ".released" => Disposition::Released,
                other => panic!("{other}"),
            };
            let parts = inside.split(',').map(str::trim).collect::<Vec<_>>();
            let facts = ExitFacts {
                parent_live: parts[0] == "true",
                frame_clear: parts[1] == "true",
                selected: match parts[2] {
                    ".absent" => Selected::Absent,
                    ".contained" => Selected::Contained,
                    ".live" => Selected::Live,
                    ".escaped" => Selected::Escaped,
                    ".released" => Selected::Released,
                    ".mixed" => Selected::Mixed,
                    other => panic!("{other}"),
                },
                returned: match parts[3] {
                    ".nothing" => Returned::Nothing,
                    ".selected" => Returned::Selected,
                    ".other" => Returned::Other,
                    other => panic!("{other}"),
                },
            };
            let result = result.trim().trim_end_matches(',').trim_end_matches(')');
            assert_eq!(accept_exit(disposition, facts), result == "true", "{row}");
        }
    }

    #[test]
    fn lean_selection_vectors_match_the_rust_model() {
        let rows = table(
            lean(),
            "def selectionVectors : List (List Nat × Int × Option Nat) := [",
        );
        assert_eq!(rows.len(), 8);
        for row in rows {
            let (list, rest) = row.trim_start_matches('(').split_once("],").unwrap();
            let children = list
                .trim_start_matches('[')
                .split(',')
                .map(str::trim)
                .filter(|x| !x.is_empty())
                .map(|x| x.parse::<usize>().unwrap())
                .collect::<Vec<_>>();
            let (which, result) = rest.split_once(',').unwrap();
            let which = which.trim().trim_matches(|c| c == '(' || c == ')');
            let which = which.parse::<i64>().unwrap();
            let result = result
                .trim()
                .trim_end_matches(',')
                .trim_end_matches(')')
                .trim();
            let expected = (result != "none").then(|| {
                result
                    .strip_prefix("some ")
                    .unwrap()
                    .parse::<usize>()
                    .unwrap()
            });
            assert_eq!(selection(&children, which), expected, "{row}");
        }
    }

    #[test]
    fn every_selected_child_is_an_existing_member_and_negative_selects_nothing() {
        for len in 0..7 {
            let children = (20..20 + len).collect::<Vec<_>>();
            for which in -3..9 {
                match selection(&children, which) {
                    Some(child) => {
                        assert!(which >= 0 && children.contains(&child));
                        assert_eq!(
                            Some(child),
                            crate::list_lookup::lookup(&children, which as usize)
                        );
                    }
                    None => assert!(which < 0 || which as usize >= children.len()),
                }
            }
        }
    }

    #[test]
    fn validation_needs_a_clean_selecting_run_and_every_exit() {
        let good = ExitFacts {
            parent_live: true,
            frame_clear: true,
            selected: Selected::Escaped,
            returned: Returned::Selected,
        };
        let early = ExitFacts {
            selected: Selected::Absent,
            returned: Returned::Nothing,
            ..good
        };
        assert!(validates(Disposition::Returned, true, &[early, good], true));
        assert!(!validates(
            Disposition::Returned,
            false,
            &[early, good],
            true
        ));
        assert!(!validates(Disposition::Returned, true, &[early], false));
        assert!(!validates(Disposition::Returned, true, &[], true));
        let kept = ExitFacts {
            selected: Selected::Contained,
            returned: Returned::Nothing,
            ..good
        };
        assert!(!validates(Disposition::Returned, true, &[kept, good], true));
        assert!(!validates(Disposition::Released, true, &[good], true));
    }
}
