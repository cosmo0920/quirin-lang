//! Card **OP-APP-RK-CALLBACKS-VIA-POLL-CB** (gateway rule 12n, amended in
//! place; contract specification C04.19.3): the run's member facts, read by a
//! pre-pass before any unit is analysed, so a unit decides a member invocation
//! over the run's closed set instead of its own.
//!
//! USER DECISION 2026-10-14 ("Build the agent's recommended set"): **R1**, a
//! facts pre-pass so a unit decides over the run's set, the fold staying
//! demote-only; **F1**, member facts read from every Fluent Bit production
//! unit; **E1**, a run-closed empty set decides "invokes nothing"; **Q2**,
//! a declared setter's registration
//! (`resource_callback_member_registered(conf, TAG.FIELD, cb)`) is a store of
//! its actual at every call the run makes; **D1**, a constructor's stated
//! default (`quant live #result: TAG.FIELD |-> f;`) is a store of `f`.
//!
//! The pre-pass itself is `qcore_audit::member_prepass`; this module holds
//! what it read ([`RunMemberFacts`]) and installs it on the analysing thread
//! ([`scoped`]) for [`crate::legacy_c`]'s callback route to read
//! ([`installed`]). Card MEMBER-PREPASS-OTHER-ROUTES: the pinned, current
//! and no-build routes all install it when a declaration invokes a member,
//! and a checkpoint carries the run's closures for its composer. With
//! nothing installed (an analysis outside those routes) a unit decides over
//! its own facts as before and an empty set is refused.
//!
//! Card **CALLBACK-MEMBER-NON-C-UNITS** (rule 12n, amended in place): an
//! unread C++ unit whose preprocessed text is the whole unit is scoped
//! ([`RunMemberFacts::scoped`]): it opens only a member whose closure-graph
//! tokens it names, the reading by which the pre-pass skips a C unit naming
//! no token, applied to each member's own graph
//! (`QCore.MemberNonCUnits.a_scoped_unit_naming_no_graph_token_keeps_the_run_set`).
//! Every other unread unit still opens every member.

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, OnceLock};

use crate::callback_member_closure::{Closure, Flow, MemberFacts, Prepared, closure_prepared};

/// What the pre-pass read: every read unit's facts, the declared
/// registrations and stated defaults as one more unit, and the units it could
/// not read although they may name a token of a member's closure.
#[derive(Debug, Default)]
pub struct RunMemberFacts {
    /// Every unit read for its facts, then the declarations' unit.
    pub units: Vec<MemberFacts>,
    /// A unit that may name a token of a member's closure but whose facts
    /// were not read (a C++ or assembly unit, a failed preprocessing or
    /// Clang run), with why. Such a unit opens every member, except a unit
    /// of [`Self::scoped`].
    pub unread: BTreeMap<String, String>,
    /// Card CALLBACK-MEMBER-NON-C-UNITS: the unread C++ units whose
    /// preprocessed text is the whole unit, each with the tokens of
    /// [`Self::checked`] it names. Such a unit opens only a member whose
    /// closure-graph tokens it names (or a member with a graph token outside
    /// `checked`, which was never looked for).
    pub scoped: BTreeMap<String, BTreeSet<String>>,
    /// The tokens every unit of `scoped` was checked against: the pre-pass's
    /// final tokens.
    pub checked: BTreeSet<String>,
    /// The units' stores by normal key, built once.
    prepared: OnceLock<Prepared>,
    /// Every function the facts name as a value, built once.
    named_functions: OnceLock<BTreeSet<String>>,
}

impl RunMemberFacts {
    /// The facts the pre-pass read.
    #[must_use]
    pub fn new(units: Vec<MemberFacts>, unread: BTreeMap<String, String>) -> Self {
        Self {
            units,
            unread,
            scoped: BTreeMap::new(),
            checked: BTreeSet::new(),
            prepared: OnceLock::new(),
            named_functions: OnceLock::new(),
        }
    }

    /// Card **CURRENT-RUN-WORKER-SPIN** (rule 4c, amended): every function
    /// any member's run closure can deliver. A closure reads a member's
    /// stores (every unit's, the declarations' stated defaults included) and,
    /// for a stored formal of a setter, that setter's actuals at the same
    /// position at every call, transitively through further formals
    /// ([`crate::callback_member_closure::closure`]'s `Member` and `Formal`
    /// nodes). This set is the union of every such reading over every member
    /// and every unit, ignoring the closure's scope and opening conditions,
    /// so it holds every candidate of every closure: a function outside it
    /// and unspelled in a unit is never a candidate of that unit's member
    /// invocations.
    #[must_use]
    pub fn named_functions(&self) -> &BTreeSet<String> {
        self.named_functions.get_or_init(|| {
            let mut names = BTreeSet::new();
            let mut formals = Vec::new();
            let mut seen = BTreeSet::new();
            let mut read = |flow: &Flow, formals: &mut Vec<(String, usize)>| match flow {
                Flow::Function { name } => {
                    names.insert(name.clone());
                }
                Flow::Formal { function, index } => {
                    formals.push((function.clone(), *index));
                }
                Flow::Null | Flow::Member { .. } | Flow::Open { .. } => {}
            };
            for unit in &self.units {
                for flow in unit.stores.values().flatten() {
                    read(flow, &mut formals);
                }
            }
            while let Some(formal) = formals.pop() {
                if !seen.insert(formal.clone()) {
                    continue;
                }
                let (function, index) = formal;
                for unit in &self.units {
                    if let Some(flows) = unit.calls.get(&function).and_then(|at| at.get(&index)) {
                        for flow in flows {
                            read(flow, &mut formals);
                        }
                    }
                }
            }
            names
        })
    }

    /// Card CALLBACK-MEMBER-NON-C-UNITS: the same facts, with the unread
    /// units whose reading is scoped to the members they name. A unit of
    /// `scoped` that is not in `unread` is ignored (it opens nothing it
    /// would not open unread).
    #[must_use]
    pub fn with_scoped(
        mut self,
        checked: BTreeSet<String>,
        scoped: BTreeMap<String, BTreeSet<String>>,
    ) -> Self {
        self.checked = checked;
        self.scoped = scoped
            .into_iter()
            .filter(|(unit, _)| self.unread.contains_key(unit))
            .collect();
        self
    }

    /// Why the unread `unit` opens a member whose closure graph names
    /// `tokens`, or `None` when its reading keeps the member's set (rule 12n,
    /// card CALLBACK-MEMBER-NON-C-UNITS): a unit outside [`Self::scoped`]
    /// opens every member; a scoped unit opens it when it names one of the
    /// tokens, or when a token was never checked against it.
    #[must_use]
    pub fn opens(&self, unit: &str, tokens: &BTreeSet<String>) -> Option<String> {
        let why = self.unread.get(unit)?;
        let Some(named) = self.scoped.get(unit) else {
            return Some(why.clone());
        };
        let unchecked = tokens.difference(&self.checked).next();
        if let Some(token) = unchecked {
            return Some(format!(
                "{why}, not checked for the member's token `{token}`"
            ));
        }
        let hits = named.intersection(tokens).take(3).collect::<Vec<_>>();
        if hits.is_empty() {
            return None;
        }
        Some(format!(
            "{why}, naming {}",
            hits.iter()
                .map(|token| format!("`{token}`"))
                .collect::<Vec<_>>()
                .join(", ")
        ))
    }

    /// The unread units that open `member`, with why (card
    /// CALLBACK-MEMBER-NON-C-UNITS: evidence for the pre-pass section).
    #[must_use]
    pub fn opening(&self, member: &str) -> BTreeMap<String, String> {
        let views = self.units.iter().collect::<Vec<_>>();
        let tokens = closure_prepared(&views, self.prepared(), member).1;
        self.opening_over(&tokens)
    }

    fn opening_over(&self, tokens: &BTreeSet<String>) -> BTreeMap<String, String> {
        self.unread
            .keys()
            .filter_map(|unit| Some((unit.clone(), self.opens(unit, tokens)?)))
            .collect()
    }

    fn prepared(&self) -> &Prepared {
        self.prepared.get_or_init(|| {
            let views = self.units.iter().collect::<Vec<_>>();
            Prepared::of(&views)
        })
    }

    /// The normal forms of a declared member over the run's records (rule
    /// 12n, Q3). `Err` when it is not a function-pointer member of a record
    /// the run defines: such a member is never decided over the run's set,
    /// so a misspelled member can never read as "invokes nothing".
    ///
    /// # Errors
    /// Why the member is not read.
    pub fn normalise(&self, member: &str) -> Result<BTreeSet<String>, String> {
        self.prepared().tables.normalise(member)
    }

    /// The run's closure of `member`: open while any unread unit opens it
    /// ([`Self::opens`]).
    pub fn closure(&self, member: &str) -> Closure {
        let views = self.units.iter().collect::<Vec<_>>();
        let (result, tokens) = closure_prepared(&views, self.prepared(), member);
        let opening = self.opening_over(&tokens);
        if opening.is_empty() {
            return result;
        }
        let reason = format!(
            "{} unit(s) of the run may name it but were not read (first: {})",
            opening.len(),
            opening
                .iter()
                .next()
                .map(|(unit, why)| format!("{unit}: {why}"))
                .unwrap_or_default()
        );
        Err(match result {
            Ok(_) => BTreeSet::from([reason]),
            Err(mut open) => {
                open.insert(reason);
                open
            }
        })
    }
}

/// A declared registration: `function`'s parameter `index` is stored into
/// `member` at every call (`resource_callback_member_registered(P, TAG.FIELD,
/// F)` with `P` a parameter), or `function`'s result holds `value` in
/// `member` (a stated default, `quant live #result: TAG.FIELD |-> f;`).
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum DeclaredStore {
    /// A setter's parameter.
    Parameter {
        member: String,
        function: String,
        index: usize,
    },
    /// A constructor's stated function default.
    Default { member: String, function: String },
}

/// The declarations' unit: each registration a store of the setter's formal
/// (so the closure reads the setter's actuals at every call the run makes,
/// opening on an address-taken setter), each stated default a store of the
/// named function.
#[must_use]
pub fn declarations_unit(stores: &[DeclaredStore]) -> MemberFacts {
    let mut facts = MemberFacts::default();
    for store in stores {
        let (member, flow) = match store {
            DeclaredStore::Parameter {
                member,
                function,
                index,
            } => (
                member,
                Flow::Formal {
                    function: function.clone(),
                    index: *index,
                },
            ),
            DeclaredStore::Default { member, function } => (
                member,
                Flow::Function {
                    name: function.clone(),
                },
            ),
        };
        facts.stores.entry(member.clone()).or_default().insert(flow);
    }
    facts
}

thread_local! {
    static INSTALLED: RefCell<Option<Arc<RunMemberFacts>>> = const { RefCell::new(None) };
}

/// Run `f` with `facts` installed on this thread (the pinned route wraps each
/// unit's analysis), restoring what was installed before.
pub fn scoped<T>(facts: Option<Arc<RunMemberFacts>>, f: impl FnOnce() -> T) -> T {
    struct Restore(Option<Arc<RunMemberFacts>>);
    impl Drop for Restore {
        fn drop(&mut self) {
            let previous = self.0.take();
            INSTALLED.with(|cell| *cell.borrow_mut() = previous);
        }
    }
    let previous = INSTALLED.with(|cell| std::mem::replace(&mut *cell.borrow_mut(), facts));
    let _restore = Restore(previous);
    f()
}

/// The run's facts installed on this thread, if any.
#[must_use]
pub fn installed() -> Option<Arc<RunMemberFacts>> {
    INSTALLED.with(|cell| cell.borrow().clone())
}

/// `RUN_DECISION_VECTORS` (`QCore.AppRkCallbacks.runDecisionVectors`): a run
/// closure (`None` open) and whether a member invocation over identity
/// candidates is decided (E1: a closed empty set invokes nothing).
#[cfg(test)]
pub(crate) const RUN_DECISION_VECTORS: &[(Option<&[&str]>, bool)] = &[
    (None, false),
    (Some(&[]), true),
    (Some(&["a"]), true),
    (Some(&["a", "b"]), true),
];

/// `SCOPED_VECTORS` (`QCore.MemberNonCUnits.scopedVectors`): an unread unit
/// (`None`: not scoped, so its tokens are unknown; `Some(named)`: a scoped
/// C++ unit naming `named`), the checked tokens, a member's graph tokens, and
/// whether the unit opens the member.
#[cfg(test)]
#[allow(clippy::type_complexity)]
pub(crate) const SCOPED_VECTORS: &[(Option<&[&str]>, &[&str], &[&str], bool)] = &[
    (None, &["cmt"], &["cmt"], true),
    (None, &[], &[], true),
    (Some(&[]), &["cmt"], &["cmt"], false),
    (Some(&["event"]), &["cmt", "event"], &["cmt"], false),
    (Some(&["cmt", "event"]), &["cmt", "event"], &["cmt"], true),
    (Some(&[]), &["cmt"], &["cmt", "cio_options"], true),
    (
        Some(&["event"]),
        &["event", "rd_kafka_s"],
        &["rd_kafka_s", "rd_kafka_conf_s"],
        true,
    ),
    (
        Some(&["event"]),
        &["event", "rd_kafka_s", "rd_kafka_conf_s"],
        &["rd_kafka_s", "rd_kafka_conf_s"],
        false,
    ),
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::callback_member_closure::closure_tokens;

    fn set(names: &[&str]) -> BTreeSet<String> {
        names.iter().map(|name| (*name).to_owned()).collect()
    }

    #[test]
    fn scoped_vectors_hold() {
        for (named, checked, graph, opens) in SCOPED_VECTORS {
            let mut run = RunMemberFacts::new(
                Vec::new(),
                BTreeMap::from([(
                    "u.cpp".to_owned(),
                    "a unit that is not C (`cpp`)".to_owned(),
                )]),
            );
            if let Some(named) = named {
                run = run.with_scoped(
                    set(checked),
                    BTreeMap::from([("u.cpp".to_owned(), set(named))]),
                );
            }
            assert_eq!(
                run.opens("u.cpp", &set(graph)).is_some(),
                *opens,
                "{named:?} {checked:?} {graph:?}"
            );
        }
        assert_eq!(SCOPED_VECTORS.len(), 8);
    }

    #[test]
    fn a_scoped_unit_opens_only_the_members_it_names() {
        // Unit 0 stores `h` into `T.f` and `g` into `U.g`; the unread C++
        // unit names `U` only.
        let mut unit = MemberFacts::default();
        unit.stores
            .entry("T.f".into())
            .or_default()
            .insert(Flow::Function { name: "h".into() });
        unit.stores
            .entry("U.g".into())
            .or_default()
            .insert(Flow::Function { name: "g".into() });
        let unread = BTreeMap::from([(
            "x.cpp".to_owned(),
            "a unit that is not C (`cpp`)".to_owned(),
        )]);
        let tokens = closure_tokens(&[&unit], "T.f")
            .union(&closure_tokens(&[&unit], "U.g"))
            .cloned()
            .collect::<BTreeSet<_>>();
        assert!(tokens.contains("T") && tokens.contains("U"), "{tokens:?}");
        let scoped = RunMemberFacts::new(vec![unit.clone()], unread.clone()).with_scoped(
            tokens.clone(),
            BTreeMap::from([("x.cpp".to_owned(), set(&["U"]))]),
        );
        assert_eq!(scoped.closure("T.f"), Ok(set(&["h"])));
        let open = scoped.closure("U.g").unwrap_err();
        assert!(
            open.iter()
                .any(|why| why.contains("x.cpp") && why.contains("naming `U`")),
            "{open:?}"
        );
        assert_eq!(scoped.opening("T.f"), BTreeMap::new());
        // Unscoped, the same unit opens both members, as before.
        let unscoped = RunMemberFacts::new(vec![unit.clone()], unread.clone());
        assert!(unscoped.closure("T.f").is_err());
        assert!(unscoped.closure("U.g").is_err());
        // A token never checked against the unit opens the member.
        let unchecked = RunMemberFacts::new(vec![unit.clone()], unread.clone()).with_scoped(
            set(&["U"]),
            BTreeMap::from([("x.cpp".to_owned(), set(&[]))]),
        );
        let open = unchecked.closure("T.f").unwrap_err();
        assert!(
            open.iter()
                .any(|why| why.contains("not checked for the member's token `T`")),
            "{open:?}"
        );
        // A scoped entry for a unit that is not unread is ignored.
        let stray = RunMemberFacts::new(vec![unit], BTreeMap::new())
            .with_scoped(tokens, BTreeMap::from([("y.cpp".to_owned(), set(&["T"]))]));
        assert!(stray.scoped.is_empty());
        assert_eq!(stray.closure("T.f"), Ok(set(&["h"])));
    }

    #[test]
    fn a_registration_reads_the_setters_actuals_at_every_call() {
        // Unit 0 calls the setter with `h`; the declarations' unit stores the
        // setter's formal 1 into `T.f`.
        let mut caller = MemberFacts::default();
        caller
            .calls
            .entry("set_f".into())
            .or_default()
            .insert(1, BTreeSet::from([Flow::Function { name: "h".into() }]));
        let declared = declarations_unit(&[DeclaredStore::Parameter {
            member: "T.f".into(),
            function: "set_f".into(),
            index: 1,
        }]);
        let run = RunMemberFacts::new(vec![caller.clone(), declared.clone()], BTreeMap::new());
        assert_eq!(run.closure("T.f"), Ok(BTreeSet::from(["h".to_owned()])));
        // A stated default joins the set.
        let with_default = RunMemberFacts::new(
            vec![
                caller.clone(),
                declared.clone(),
                declarations_unit(&[DeclaredStore::Default {
                    member: "T.f".into(),
                    function: "d".into(),
                }]),
            ],
            BTreeMap::new(),
        );
        assert_eq!(
            with_default.closure("T.f"),
            Ok(BTreeSet::from(["d".to_owned(), "h".to_owned()]))
        );
        // An address-taken setter opens it.
        let mut taken = caller.clone();
        taken.address_taken.insert("set_f".into());
        let taken = RunMemberFacts::new(vec![taken, declared.clone()], BTreeMap::new());
        assert!(taken.closure("T.f").is_err());
        // An unread unit opens it.
        let unread = RunMemberFacts::new(
            vec![caller, declared],
            BTreeMap::from([("x.cpp".to_owned(), "a C++ unit".to_owned())]),
        );
        assert!(unread.closure("T.f").is_err());
    }

    #[test]
    fn a_scoped_installation_is_restored() {
        assert!(installed().is_none());
        let facts = Arc::new(RunMemberFacts::default());
        scoped(Some(facts.clone()), || {
            assert!(installed().is_some());
            scoped(None, || assert!(installed().is_none()));
            assert!(installed().is_some());
        });
        assert!(installed().is_none());
    }

    #[test]
    fn run_decision_vectors_hold() {
        use crate::callback_contracts::{Decision, decide_run};
        for (closure, admitted) in RUN_DECISION_VECTORS {
            let classes = closure.map(|names| {
                names
                    .iter()
                    .map(|name| {
                        (
                            (*name).to_owned(),
                            crate::callback_contracts::CandidateClass::Identity,
                        )
                    })
                    .collect::<Vec<_>>()
            });
            let decision = decide_run(classes.as_deref().ok_or(()));
            assert_eq!(decision == Decision::Identity, *admitted, "{closure:?}");
        }
        assert_eq!(RUN_DECISION_VECTORS.len(), 4);
    }
}
