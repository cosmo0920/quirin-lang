//! The descriptor-cell table of one abstract path (rule 38).
//!
//! Every operation refuses rather than approximates: a key the host could
//! not resolve (`None`), a name that may be another, or a state no witness
//! decided keeps the call `Unknown`, never `Safe`, never `Unsafe`.

use std::collections::BTreeMap;

/// What a name says of its identity on this path.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Status {
    /// A live descriptor this function owes.
    Live,
    /// Created in a class no witness has decided, or joined from paths that
    /// disagree: it may be live.
    Maybe,
    /// Released through some name (or never live: a creation's failure
    /// class); a later release refuses.
    Stale,
}

/// The identity a joined name may stand for when the paths disagree.
pub const JOINED: u64 = u64::MAX;

/// A refusal, with its stable reason key.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refusal {
    /// The actual names no cell key, or a release may name a tracked cell it
    /// cannot match, or the cell's class is undecided.
    Undecided,
    /// A creation into a cell that already names a live descriptor.
    Overwritten,
    /// A release of a stale cell: the descriptor is already closed.
    DoubleRelease,
    /// A live descriptor named only by cells that outlive the frame.
    Escapes,
    /// The object holding a live cell is released.
    OwnerReleased,
    /// A member-equality guard the body does not decide.
    GuardUndecided,
    /// Rule 38c: a record with a declared cell protocol does not keep it at
    /// an exit or a call, or a store the protocol cannot read.
    ProtocolBroken,
    /// Rule 38f (card CELL-PROTOCOL-PER-END-GUARDS): the holder of a cell a
    /// `> 0` test's dead edge left undecided is released, or does not keep
    /// its protocol at an exit: the end may be descriptor 0, a success value
    /// the test read as absent (a candidate defect, never `Unsafe`).
    ZeroUndecided,
}

impl Refusal {
    /// The stable reason key.
    #[must_use]
    pub const fn key(self) -> &'static str {
        match self {
            Self::Undecided => "descriptor-cell-undecided",
            Self::Overwritten => "descriptor-cell-overwritten",
            Self::DoubleRelease => "descriptor-cell-double-release",
            Self::Escapes => "descriptor-cell-escapes",
            Self::OwnerReleased => "descriptor-cell-owner-released",
            Self::GuardUndecided => "descriptor-cell-guard-undecided",
            Self::ProtocolBroken => "descriptor-cell-protocol-broken",
            Self::ZeroUndecided => "descriptor-cell-zero-undecided",
        }
    }
}

/// What a release did.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Released {
    /// The identity the name named is released; every name of it is stale.
    Identity,
    /// No cell of this function may be named: the call is the identity on
    /// custody (the state is on an object this function does not track,
    /// premise `P-CELL-STATE-NOT-FORWARDED`).
    NotTracked,
}

/// A member-equality guard's decision.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Equality {
    /// Both sides name the same live identity.
    Equal,
    /// They name two distinct live identities (two open descriptors never
    /// share a number).
    Distinct,
    /// Anything else.
    Undecided,
}

/// The key the host's cells are recorded under: the cell's spelling (unique
/// per cell), the tracked identity its object is rooted at, and whether it
/// is a local of the frame.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Key {
    pub name: String,
    pub site: Option<usize>,
    pub local: bool,
}

impl Key {
    /// Whether the two keys are known to be distinct storage: different
    /// locals, a local and an object, or objects of two distinct tracked
    /// identities. Everything else may be one cell.
    #[must_use]
    pub fn distinct(&self, other: &Self) -> bool {
        match (self.local, other.local) {
            (true, true) => self.name != other.name,
            (true, false) | (false, true) => true,
            (false, false) => {
                matches!((self.site, other.site), (Some(left), Some(right)) if left != right)
            }
        }
    }
}

/// The cells of one path: each name, its identity and its status.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Cells<K: Ord> {
    names: BTreeMap<K, (u64, Status)>,
}

impl<K: Ord> Default for Cells<K> {
    fn default() -> Self {
        Self {
            names: BTreeMap::new(),
        }
    }
}

impl<K: Ord + Clone> Cells<K> {
    /// Whether no cell is recorded.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.names.is_empty()
    }

    /// The identity and status a name has.
    #[must_use]
    pub fn get(&self, key: &K) -> Option<(u64, Status)> {
        self.names.get(key).copied()
    }

    /// `#fresh(K)`: a new identity named by `key`. `witnessed`: its class is
    /// decided by a witness later (`Maybe` until then).
    ///
    /// # Errors
    /// [`Refusal::Undecided`] for an unresolved key, [`Refusal::Overwritten`]
    /// for a key naming a descriptor that may be live.
    pub fn create(
        &mut self,
        key: Option<&K>,
        identity: u64,
        witnessed: bool,
    ) -> Result<(), Refusal> {
        let key = key.ok_or(Refusal::Undecided)?;
        if matches!(self.names.get(key), Some((_, Status::Live | Status::Maybe))) {
            return Err(Refusal::Overwritten);
        }
        // The same call reached again (a loop) would give a second descriptor
        // the first one's identity: refused, never merged.
        if self.names.values().any(|(named, _)| *named == identity) {
            return Err(Refusal::Undecided);
        }
        let status = if witnessed {
            Status::Maybe
        } else {
            Status::Live
        };
        self.names.insert(key.clone(), (identity, status));
        Ok(())
    }

    /// Rule 38c: record a protocol's pending name under its key, as the
    /// test that introduced it decided (a key already recorded is kept).
    pub fn adopt(&mut self, key: K, identity: u64, status: Status) {
        self.names.entry(key).or_insert((identity, status));
    }

    /// Rule 38c: drop every name `forget` selects (a record forwarded under
    /// its protocol at a call: its cells are read again from the protocol).
    pub fn forget(&mut self, forget: impl Fn(&K) -> bool) {
        self.names.retain(|key, _| !forget(key));
    }

    /// Every recorded name with its identity and status.
    pub fn entries(&self) -> impl Iterator<Item = (&K, &(u64, Status))> {
        self.names.iter()
    }

    /// `#same(S)`: `key` becomes a second name of the identity `of` names.
    ///
    /// # Errors
    /// As [`Self::create`]; [`Refusal::Undecided`] when `of` names nothing.
    pub fn alias(&mut self, key: Option<&K>, of: &K) -> Result<(), Refusal> {
        let key = key.ok_or(Refusal::Undecided)?;
        let named = self.names.get(of).copied().ok_or(Refusal::Undecided)?;
        if matches!(self.names.get(key), Some((_, Status::Live | Status::Maybe))) {
            return Err(Refusal::Overwritten);
        }
        self.names.insert(key.clone(), named);
        Ok(())
    }

    /// A witness decided `identity`'s class on this edge: `holds` (the
    /// creation's class) makes its names live, the complement makes them
    /// stale (the cell names no live descriptor; it may hold a closed one).
    pub fn witness(&mut self, identity: u64, holds: bool) {
        if identity == JOINED {
            return;
        }
        for (named, status) in self.names.values_mut() {
            if *named == identity && *status == Status::Maybe {
                *status = if holds { Status::Live } else { Status::Stale };
            }
        }
    }

    /// `#stale(K)` through `key`, or a guarded release through the cell an
    /// integer actual reads. `may_name(k)`: whether the release may name the
    /// cell `k` when `key` is unresolved or names nothing recorded.
    ///
    /// # Errors
    /// [`Refusal::Undecided`] for a name that may be live, or a release that
    /// may name a recorded cell it does not match; [`Refusal::DoubleRelease`]
    /// for a stale name.
    pub fn release(
        &mut self,
        key: Option<&K>,
        may_name: impl Fn(&K) -> bool,
    ) -> Result<Released, Refusal> {
        if let Some(key) = key
            && let Some((identity, status)) = self.names.get(key).copied()
        {
            return match status {
                Status::Live if identity != JOINED => {
                    for (named, status) in self.names.values_mut() {
                        if *named == identity {
                            *status = Status::Stale;
                        }
                    }
                    Ok(Released::Identity)
                }
                Status::Stale => Err(Refusal::DoubleRelease),
                Status::Live | Status::Maybe => Err(Refusal::Undecided),
            };
        }
        if self
            .names
            .keys()
            .any(|named| key.is_none() || may_name(named))
        {
            return Err(Refusal::Undecided);
        }
        Ok(Released::NotTracked)
    }

    /// The member-equality guard: `member` and `against` name the same live
    /// identity, two distinct live ones, or anything else.
    #[must_use]
    pub fn equality(&self, member: Option<&K>, against: Option<&K>) -> Equality {
        match (
            member.and_then(|key| self.names.get(key)),
            against.and_then(|key| self.names.get(key)),
        ) {
            (Some((left, Status::Live)), Some((right, Status::Live)))
                if *left != JOINED && *right != JOINED =>
            {
                if left == right {
                    Equality::Equal
                } else {
                    Equality::Distinct
                }
            }
            _ => Equality::Undecided,
        }
    }

    /// Join an arriving path: agreeing names stay; a live or undecided name
    /// on one side only, or names that disagree, may be live.
    pub fn join(&mut self, incoming: &Self) {
        let keys = self
            .names
            .keys()
            .chain(incoming.names.keys())
            .cloned()
            .collect::<Vec<_>>();
        for key in keys {
            let joined = match (self.names.get(&key), incoming.names.get(&key)) {
                (Some(left), Some(right)) if left == right => *left,
                (Some((_, Status::Stale)), None) | (None, Some((_, Status::Stale))) => {
                    (JOINED, Status::Stale)
                }
                (Some((left, _)), Some((right, _))) if left == right => (*left, Status::Maybe),
                _ => (JOINED, Status::Maybe),
            };
            self.names.insert(key, joined);
        }
    }

    /// Card CELL-PROTOCOL-PER-END-GUARDS (rule 38f): [`Self::join`], except
    /// that a name `keep` selects (a declared protocol's cell) that both paths
    /// left stale stays stale, whatever identities the paths name: neither
    /// path leaves a live descriptor in it.
    pub fn join_keeping_stale(&mut self, incoming: &Self, keep: impl Fn(&K) -> bool) {
        let both = self
            .names
            .iter()
            .filter(|(key, (_, status))| {
                *status == Status::Stale
                    && keep(key)
                    && matches!(incoming.names.get(*key), Some((_, Status::Stale)))
            })
            .map(|(key, _)| key.clone())
            .collect::<Vec<_>>();
        self.join(incoming);
        for key in both {
            if let Some(named) = self.names.get_mut(&key)
                && named.1 != Status::Stale
            {
                *named = (JOINED, Status::Stale);
            }
        }
    }

    /// The names that may still be live: at an exit each refuses
    /// ([`Refusal::Escapes`]).
    pub fn escaping(&self) -> impl Iterator<Item = &K> {
        self.names
            .iter()
            .filter(|(_, (_, status))| *status != Status::Stale)
            .map(|(key, _)| key)
    }

    /// Whether a name `held` selects may still be live (the release of the
    /// object holding it refuses, [`Refusal::OwnerReleased`]).
    pub fn holds(&self, held: impl Fn(&K) -> bool) -> bool {
        self.names
            .iter()
            .any(|(key, (_, status))| *status != Status::Stale && held(key))
    }
}

/// The shared vectors (`QCore.DescriptorCells.decision_vectors_hold`): a
/// trace of operations on one cell `c` (and a second `d`), and whether the
/// exit refuses or the trace refuses earlier. Each operation: `fresh c`,
/// `witness ok`, `witness fail`, `stale c`, `same d`, `join` (with the empty
/// path).
pub const DECISION_VECTORS: &[(&str, &str)] = &[
    ("fresh c; witness ok; stale c", "safe"),
    ("fresh c; witness ok", "escapes"),
    ("fresh c; witness fail", "safe"),
    ("fresh c; stale c", "undecided"),
    ("fresh c; witness ok; stale c; stale c", "double-release"),
    (
        "fresh c; witness ok; same d; stale d; stale c",
        "double-release",
    ),
    ("fresh c; witness ok; same d; stale d", "safe"),
    ("fresh c; witness ok; join", "escapes"),
    ("stale c", "safe"),
    ("fresh c; witness ok; fresh c", "overwritten"),
];

/// The classification arm `descriptor-cell-release` (registry rule `38`): a
/// declared `#stale(K)` state, or a guarded release, through `key`; an
/// unresolved or unrecorded key may name every recorded cell it is not known
/// to be distinct from.
///
/// # Errors
/// As [`Cells::release`].
pub fn release(cells: &mut Cells<Key>, key: Option<&Key>) -> Result<Released, Refusal> {
    let probe = key.cloned();
    cells.release(key, |recorded| {
        probe.as_ref().is_none_or(|key| !key.distinct(recorded))
    })
}

/// The classification arm `descriptor-cell-guard` (registry rule `38`): the
/// member-equality guard of a declaration, decided from identities only.
#[must_use]
pub fn decide_equality(
    cells: &Cells<Key>,
    member: Option<&Key>,
    against: Option<&Key>,
) -> Equality {
    cells.equality(member, against)
}

/// Run one vector over a fresh table; the outcome word.
#[must_use]
pub fn run_vector(trace: &str) -> &'static str {
    let mut cells: Cells<&str> = Cells::default();
    for step in trace.split(';').map(str::trim) {
        let outcome = match step.split_whitespace().collect::<Vec<_>>().as_slice() {
            ["fresh", key] => cells.create(Some(key), 1, true),
            ["witness", "ok"] => {
                cells.witness(1, true);
                Ok(())
            }
            ["witness", "fail"] => {
                cells.witness(1, false);
                Ok(())
            }
            ["same", key] => cells.alias(Some(key), &"c"),
            ["stale", key] => cells.release(Some(key), |_| true).map(|_| ()),
            ["join"] => {
                cells.join(&Cells::default());
                Ok(())
            }
            _ => return "malformed",
        };
        if let Err(refusal) = outcome {
            return match refusal {
                Refusal::Undecided => "undecided",
                Refusal::Overwritten => "overwritten",
                Refusal::DoubleRelease => "double-release",
                _ => "other",
            };
        }
    }
    if cells.escaping().next().is_some() {
        "escapes"
    } else {
        "safe"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_shared_vectors_hold() {
        for (trace, expected) in DECISION_VECTORS {
            assert_eq!(run_vector(trace), *expected, "{trace}");
        }
    }

    #[test]
    fn an_unresolved_key_never_decides() {
        let mut cells: Cells<&str> = Cells::default();
        assert_eq!(cells.create(None, 1, false), Err(Refusal::Undecided));
        assert_eq!(cells.release(None, |_| false), Ok(Released::NotTracked));
        cells.create(Some(&"c"), 1, false).expect("creates");
        assert_eq!(cells.release(None, |_| false), Err(Refusal::Undecided));
        assert_eq!(
            cells.release(Some(&"other"), |_| true),
            Err(Refusal::Undecided)
        );
        assert_eq!(
            cells.release(Some(&"other"), |_| false),
            Ok(Released::NotTracked)
        );
    }

    #[test]
    fn the_guard_decides_only_live_identities() {
        let mut cells: Cells<&str> = Cells::default();
        cells.create(Some(&"r"), 1, false).expect("creates");
        cells.alias(Some(&"m"), &"r").expect("aliases");
        cells.create(Some(&"x"), 2, false).expect("creates");
        assert_eq!(cells.equality(Some(&"m"), Some(&"r")), Equality::Equal);
        assert_eq!(cells.equality(Some(&"m"), Some(&"x")), Equality::Distinct);
        assert_eq!(cells.equality(Some(&"m"), None), Equality::Undecided);
        cells.create(Some(&"y"), 3, true).expect("creates");
        assert_eq!(cells.equality(Some(&"m"), Some(&"y")), Equality::Undecided);
    }

    #[test]
    fn a_join_keeps_only_what_both_paths_agree_on() {
        let mut left: Cells<&str> = Cells::default();
        left.create(Some(&"c"), 1, false).expect("creates");
        let mut right = left.clone();
        right.release(Some(&"c"), |_| true).expect("releases");
        left.join(&right);
        assert_eq!(left.get(&"c"), Some((1, Status::Maybe)));
        assert!(left.holds(|key| *key == "c"));
    }
}
