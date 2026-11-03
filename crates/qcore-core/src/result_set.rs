//! Declared integer result sets and the class a caller's test reads under them
//! (card **CELL-GUARD-RESULT-RANGE-CHECKED**, contract specification C09.4,
//! grammar G05j, gateway rule 38e).
//!
//! USER DECISION 2026-11-03 (verbatim): "C: a new result-set form that allows
//! unions, such as result_set(-1 | [0, MAX]). It is the only option that can
//! also state open's and timerfd_create's results (needed by
//! RULE-38-VALIDITY-READING-RESULT-SET). It is also the largest grammar
//! change. I choose this because we must not loosen the custody of the
//! contracts if we cannot select other options." and (verbatim) "Also, we
//! need to use mathematical symbols on them and we can use XOR or AND or OR
//! with the normal symbols of programming languages."
//!
//! # Sets
//!
//! A [`ResultSet`] is a set of integers inside the **universe** of a C integer
//! type (`MIN..=MAX` of `int`, `long`, ...), held as sorted, disjoint,
//! non-adjacent closed intervals, so two equal sets have one representation
//! (and one canonical spelling, [`ResultSet::canonical`]).
//!
//! # Expressions
//!
//! [`SetExpr`] is the written form: a literal `k` (`{k}`), a closed interval
//! `[a, b]` (`a <= b`), the type's `MIN` / `MAX` as a literal or a bound, and
//! three operators with C's precedence and left associativity: `&`
//! (intersection, binds tightest), `^` (symmetric difference), `|` (union,
//! loosest). [`SetExpr::eval`] refuses a literal outside the universe, an
//! interval whose bounds are reversed, and any operand or result that denotes
//! the empty set (an empty or contradictory set is never loaded).
//!
//! # The class a test reads
//!
//! A contract splits a call's results into a class (`#result == 0`, the
//! descriptor-cell guard of §C04.6.2) and its complement. A caller's test
//! `v OP k` reads that class **exactly** under a set `S` the result is known
//! to lie in when, on `S`, the test holds on exactly the values where the
//! class holds ([`Class::Holds`]) or on exactly those where it fails
//! ([`Class::Fails`]); otherwise it reads nothing ([`Class::Undecided`]).
//! With no declared set in force `S` is the whole universe, so a test that
//! decides there decides under every narrower set
//! ([`narrowing_never_undecides`]) and a set out of force decides nothing a
//! universe-exact test would not.
//!
//! The Lean model is `QCore.ResultSet`; [`SET_VECTORS`], [`MEMBER_VECTORS`]
//! and [`CLASS_VECTORS`] are its `setVectors`, `memberVectors` and
//! `classVectors`, entry for entry (`crates/qcore-core/tests/result_set.rs`).

use std::fmt;

/// A closed interval `[lo, hi]`, `lo <= hi`.
pub type Interval = (i128, i128);

/// The universe of a C integer type, by the contract vocabulary's spelling of
/// it (`i32` is C `int`). `None` for a type with no integer universe.
#[must_use]
pub fn universe(ty: &str) -> Option<Interval> {
    Some(match ty {
        "bool" => (0, 1),
        "i8" => (i128::from(i8::MIN), i128::from(i8::MAX)),
        "i16" => (i128::from(i16::MIN), i128::from(i16::MAX)),
        "i32" => (i128::from(i32::MIN), i128::from(i32::MAX)),
        "i64" | "isize" => (i128::from(i64::MIN), i128::from(i64::MAX)),
        "u8" => (0, i128::from(u8::MAX)),
        "u16" => (0, i128::from(u16::MAX)),
        "u32" => (0, i128::from(u32::MAX)),
        "u64" | "usize" => (0, i128::from(u64::MAX)),
        _ => return None,
    })
}

/// A set of integers inside a universe: sorted, disjoint, non-adjacent
/// closed intervals. The empty set has no intervals.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ResultSet {
    universe: Interval,
    intervals: Vec<Interval>,
}

impl ResultSet {
    /// The whole universe.
    #[must_use]
    pub fn full(universe: Interval) -> Self {
        Self {
            universe,
            intervals: vec![universe],
        }
    }

    /// The empty set of a universe.
    #[must_use]
    pub fn empty(universe: Interval) -> Self {
        Self {
            universe,
            intervals: Vec::new(),
        }
    }

    /// `[lo, hi]` clipped to the universe (empty when they do not meet).
    #[must_use]
    pub fn interval(universe: Interval, lo: i128, hi: i128) -> Self {
        let (lo, hi) = (lo.max(universe.0), hi.min(universe.1));
        if lo > hi {
            Self::empty(universe)
        } else {
            Self {
                universe,
                intervals: vec![(lo, hi)],
            }
        }
    }

    /// A set from any intervals of the universe (normalised).
    #[must_use]
    pub fn from_intervals(universe: Interval, intervals: &[Interval]) -> Self {
        let mut out = Self::empty(universe);
        for (lo, hi) in intervals {
            out = out.union(&Self::interval(universe, *lo, *hi));
        }
        out
    }

    #[must_use]
    pub fn universe(&self) -> Interval {
        self.universe
    }

    #[must_use]
    pub fn intervals(&self) -> &[Interval] {
        &self.intervals
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.intervals.is_empty()
    }

    #[must_use]
    pub fn contains(&self, value: i128) -> bool {
        self.intervals
            .iter()
            .any(|(lo, hi)| *lo <= value && value <= *hi)
    }

    /// The least and greatest member.
    #[must_use]
    pub fn hull(&self) -> Option<Interval> {
        Some((self.intervals.first()?.0, self.intervals.last()?.1))
    }

    /// Every member of `self` is a member of `other`.
    #[must_use]
    pub fn is_subset(&self, other: &Self) -> bool {
        self.difference(other).is_empty()
    }

    #[must_use]
    pub fn union(&self, other: &Self) -> Self {
        self.combine(other, |a, b| a || b)
    }

    #[must_use]
    pub fn intersection(&self, other: &Self) -> Self {
        self.combine(other, |a, b| a && b)
    }

    #[must_use]
    pub fn symmetric_difference(&self, other: &Self) -> Self {
        self.combine(other, |a, b| a != b)
    }

    #[must_use]
    pub fn difference(&self, other: &Self) -> Self {
        self.combine(other, |a, b| a && !b)
    }

    /// The universe's members outside `self`.
    #[must_use]
    pub fn complement(&self) -> Self {
        Self::full(self.universe).difference(self)
    }

    /// The one combination algorithm (the Lean model's `combine`): cut the
    /// universe at every interval boundary of both sets, keep each segment
    /// whose first point `keep` admits, and merge adjacent segments. Every
    /// point of a segment has the same membership in both sets, so the
    /// segment's first point decides it.
    fn combine(&self, other: &Self, keep: impl Fn(bool, bool) -> bool) -> Self {
        let universe = (
            self.universe.0.min(other.universe.0),
            self.universe.1.max(other.universe.1),
        );
        let mut cuts = vec![universe.0];
        for (lo, hi) in self.intervals.iter().chain(&other.intervals) {
            cuts.push(*lo);
            if *hi < universe.1 {
                cuts.push(hi + 1);
            }
        }
        cuts.sort_unstable();
        cuts.dedup();
        let mut intervals: Vec<Interval> = Vec::new();
        for (index, start) in cuts.iter().enumerate() {
            let end = cuts.get(index + 1).map_or(universe.1, |next| next - 1);
            if keep(self.contains(*start), other.contains(*start)) {
                match intervals.last_mut() {
                    Some(last) if last.1 + 1 == *start => last.1 = end,
                    _ => intervals.push((*start, end)),
                }
            }
        }
        Self {
            universe,
            intervals,
        }
    }

    /// The canonical spelling: maximal intervals in increasing order, joined
    /// by ` | `; a singleton as its literal, an interval as `[a, b]`, a
    /// universe bound as `MIN` / `MAX`; `{}` for the empty set (never loaded).
    #[must_use]
    pub fn canonical(&self) -> String {
        if self.intervals.is_empty() {
            return "{}".to_owned();
        }
        let bound = |value: i128| {
            if value == self.universe.0 {
                "MIN".to_owned()
            } else if value == self.universe.1 {
                "MAX".to_owned()
            } else {
                value.to_string()
            }
        };
        self.intervals
            .iter()
            .map(|(lo, hi)| {
                if lo == hi {
                    bound(*lo)
                } else {
                    format!("[{}, {}]", bound(*lo), bound(*hi))
                }
            })
            .collect::<Vec<_>>()
            .join(" | ")
    }
}

impl fmt::Display for ResultSet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.canonical())
    }
}

/// One bound of a written set: a literal or a universe bound.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Bound {
    Literal(i128),
    Min,
    Max,
}

impl Bound {
    fn value(self, universe: Interval) -> Result<i128, SetError> {
        match self {
            Self::Literal(value) if value < universe.0 || value > universe.1 => {
                Err(SetError::OutsideType(value))
            }
            Self::Literal(value) => Ok(value),
            Self::Min => Ok(universe.0),
            Self::Max => Ok(universe.1),
        }
    }
}

/// A written set (G05j's `set_expr`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SetExpr {
    /// `k`, `MIN` or `MAX`: one value.
    Value(Bound),
    /// `[a, b]`.
    Interval(Bound, Bound),
    /// `A | B`: union.
    Or(Box<SetExpr>, Box<SetExpr>),
    /// `A & B`: intersection.
    And(Box<SetExpr>, Box<SetExpr>),
    /// `A ^ B`: symmetric difference.
    Xor(Box<SetExpr>, Box<SetExpr>),
}

/// Why a written set is refused.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SetError {
    /// A literal outside the result type's universe.
    OutsideType(i128),
    /// `[a, b]` with `a > b`.
    ReversedInterval(i128, i128),
    /// An operand or the whole expression denotes the empty set.
    Empty(String),
}

impl fmt::Display for SetError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::OutsideType(value) => write!(
                f,
                "result-set-outside-type: the literal {value} lies outside the result type"
            ),
            Self::ReversedInterval(lo, hi) => write!(
                f,
                "result-set-empty: the interval [{lo}, {hi}] has its bounds reversed"
            ),
            Self::Empty(part) => write!(
                f,
                "result-set-empty: `{part}` denotes the empty set (an empty or contradictory set is never loaded)"
            ),
        }
    }
}

impl SetExpr {
    /// The set this expression denotes in `universe`; every operand and the
    /// whole expression must be non-empty.
    ///
    /// # Errors
    ///
    /// [`SetError`] for a literal outside the universe, a reversed interval
    /// or an empty operand or result.
    pub fn eval(&self, universe: Interval) -> Result<ResultSet, SetError> {
        let set = match self {
            Self::Value(bound) => {
                let value = bound.value(universe)?;
                ResultSet::interval(universe, value, value)
            }
            Self::Interval(lo, hi) => {
                let (lo, hi) = (lo.value(universe)?, hi.value(universe)?);
                if lo > hi {
                    return Err(SetError::ReversedInterval(lo, hi));
                }
                ResultSet::interval(universe, lo, hi)
            }
            Self::Or(a, b) => a.eval(universe)?.union(&b.eval(universe)?),
            Self::And(a, b) => a.eval(universe)?.intersection(&b.eval(universe)?),
            Self::Xor(a, b) => a.eval(universe)?.symmetric_difference(&b.eval(universe)?),
        };
        if set.is_empty() {
            return Err(SetError::Empty(self.to_string()));
        }
        Ok(set)
    }

    fn precedence(&self) -> u8 {
        match self {
            Self::Value(_) | Self::Interval(..) => 3,
            Self::And(..) => 2,
            Self::Xor(..) => 1,
            Self::Or(..) => 0,
        }
    }
}

impl fmt::Display for Bound {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Literal(value) => write!(f, "{value}"),
            Self::Min => f.write_str("MIN"),
            Self::Max => f.write_str("MAX"),
        }
    }
}

impl fmt::Display for SetExpr {
    /// The expression as written, fully determined by the precedence: an
    /// operand is parenthesised only where its operator binds more loosely
    /// than its parent's, or equally on the right (left associativity).
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (a, b, op) = match self {
            Self::Value(bound) => return write!(f, "{bound}"),
            Self::Interval(lo, hi) => return write!(f, "[{lo}, {hi}]"),
            Self::Or(a, b) => (a, b, "|"),
            Self::And(a, b) => (a, b, "&"),
            Self::Xor(a, b) => (a, b, "^"),
        };
        let own = self.precedence();
        if a.precedence() < own {
            write!(f, "({a})")?;
        } else {
            write!(f, "{a}")?;
        }
        write!(f, " {op} ")?;
        if b.precedence() <= own {
            write!(f, "({b})")
        } else {
            write!(f, "{b}")
        }
    }
}

/// A token of a written set, as the contract lexer produces them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SetToken {
    /// A canonical decimal literal, optionally negative.
    Literal(i128),
    /// A word (`MIN`, `MAX`; anything else is refused).
    Word(String),
    Or,
    And,
    Xor,
    LeftBracket,
    RightBracket,
    LeftParen,
    RightParen,
    Comma,
}

/// Parse a written set (G05j): `set = xor { "|" xor }`, `xor = and { "^" and }`,
/// `and = atom { "&" atom }`, `atom = bound | "[" bound "," bound "]" | "("
/// set ")"`, `bound = literal | MIN | MAX`. Nothing else is accepted.
///
/// # Errors
///
/// `result-set-shape` for anything outside the grammar.
pub fn parse(tokens: &[SetToken]) -> Result<SetExpr, String> {
    let mut parser = SetParser { tokens, at: 0 };
    let expr = parser.or()?;
    if parser.at != tokens.len() {
        return Err(shape("trailing tokens after the set"));
    }
    Ok(expr)
}

fn shape(detail: &str) -> String {
    format!(
        "result-set-shape: {detail}; a result set is literals, `[a, b]`, `MIN`, `MAX`, `|`, `^`, `&` and parentheses"
    )
}

struct SetParser<'a> {
    tokens: &'a [SetToken],
    at: usize,
}

impl SetParser<'_> {
    fn peek(&self) -> Option<&SetToken> {
        self.tokens.get(self.at)
    }

    fn next(&mut self) -> Option<&SetToken> {
        let token = self.tokens.get(self.at);
        self.at += 1;
        token
    }

    fn or(&mut self) -> Result<SetExpr, String> {
        let mut left = self.xor()?;
        while self.peek() == Some(&SetToken::Or) {
            self.at += 1;
            left = SetExpr::Or(Box::new(left), Box::new(self.xor()?));
        }
        Ok(left)
    }

    fn xor(&mut self) -> Result<SetExpr, String> {
        let mut left = self.and()?;
        while self.peek() == Some(&SetToken::Xor) {
            self.at += 1;
            left = SetExpr::Xor(Box::new(left), Box::new(self.and()?));
        }
        Ok(left)
    }

    fn and(&mut self) -> Result<SetExpr, String> {
        let mut left = self.atom()?;
        while self.peek() == Some(&SetToken::And) {
            self.at += 1;
            left = SetExpr::And(Box::new(left), Box::new(self.atom()?));
        }
        Ok(left)
    }

    fn bound(&mut self) -> Result<Bound, String> {
        match self.next() {
            Some(SetToken::Literal(value)) => Ok(Bound::Literal(*value)),
            Some(SetToken::Word(word)) if word == "MIN" => Ok(Bound::Min),
            Some(SetToken::Word(word)) if word == "MAX" => Ok(Bound::Max),
            _ => Err(shape("a bound is a canonical decimal literal, `MIN` or `MAX`")),
        }
    }

    fn atom(&mut self) -> Result<SetExpr, String> {
        match self.peek() {
            Some(SetToken::LeftBracket) => {
                self.at += 1;
                let lo = self.bound()?;
                if self.next() != Some(&SetToken::Comma) {
                    return Err(shape("an interval is `[a, b]`"));
                }
                let hi = self.bound()?;
                if self.next() != Some(&SetToken::RightBracket) {
                    return Err(shape("an interval is `[a, b]`"));
                }
                Ok(SetExpr::Interval(lo, hi))
            }
            Some(SetToken::LeftParen) => {
                self.at += 1;
                let inner = self.or()?;
                if self.next() != Some(&SetToken::RightParen) {
                    return Err(shape("unbalanced parenthesis"));
                }
                Ok(inner)
            }
            _ => Ok(SetExpr::Value(self.bound()?)),
        }
    }
}

/// Read a written set from its text (the vectors' and the locks' spelling;
/// the contract loader reads the lexer's tokens instead).
///
/// # Errors
///
/// `result-set-shape` for a character or word outside the grammar.
pub fn parse_text(text: &str) -> Result<SetExpr, String> {
    let mut tokens = Vec::new();
    let chars = text.chars().collect::<Vec<_>>();
    let mut at = 0;
    while at < chars.len() {
        let ch = chars[at];
        let token = match ch {
            ' ' => {
                at += 1;
                continue;
            }
            '|' => SetToken::Or,
            '&' => SetToken::And,
            '^' => SetToken::Xor,
            '[' => SetToken::LeftBracket,
            ']' => SetToken::RightBracket,
            '(' => SetToken::LeftParen,
            ')' => SetToken::RightParen,
            ',' => SetToken::Comma,
            '-' | '0'..='9' => {
                let start = at;
                at += 1;
                while at < chars.len() && chars[at].is_ascii_digit() {
                    at += 1;
                }
                let spelled = chars[start..at].iter().collect::<String>();
                tokens.push(SetToken::Literal(
                    canonical_literal(&spelled)
                        .ok_or_else(|| shape("a literal is a canonical decimal"))?,
                ));
                continue;
            }
            c if c.is_ascii_alphabetic() => {
                let start = at;
                while at < chars.len() && chars[at].is_ascii_alphanumeric() {
                    at += 1;
                }
                tokens.push(SetToken::Word(chars[start..at].iter().collect()));
                continue;
            }
            _ => return Err(shape("an unexpected character")),
        };
        tokens.push(token);
        at += 1;
    }
    parse(&tokens)
}

/// A canonical decimal literal, optionally negative: no leading zero, no `-0`.
#[must_use]
pub fn canonical_literal(spelled: &str) -> Option<i128> {
    let digits = spelled.strip_prefix('-').unwrap_or(spelled);
    if digits.is_empty()
        || !digits.bytes().all(|b| b.is_ascii_digit())
        || (digits.len() > 1 && digits.starts_with('0'))
        || spelled == "-0"
    {
        return None;
    }
    spelled.parse().ok()
}

/// A comparison operator of a caller's test `v OP k`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cmp {
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
}

impl Cmp {
    /// `k OP v` is `v OP' k`.
    #[must_use]
    pub const fn mirrored(self) -> Self {
        match self {
            Self::Lt => Self::Gt,
            Self::Le => Self::Ge,
            Self::Gt => Self::Lt,
            Self::Ge => Self::Le,
            Self::Eq => Self::Eq,
            Self::Ne => Self::Ne,
        }
    }
}

/// The values of a universe for which `v OP k` holds.
#[must_use]
pub fn comparison_set(universe: Interval, cmp: Cmp, k: i128) -> ResultSet {
    let (min, max) = universe;
    match cmp {
        Cmp::Eq => ResultSet::interval(universe, k, k),
        Cmp::Ne => ResultSet::interval(universe, k, k).complement(),
        Cmp::Lt => ResultSet::interval(universe, min, k.saturating_sub(1)),
        Cmp::Le => ResultSet::interval(universe, min, k),
        Cmp::Gt => ResultSet::interval(universe, k.saturating_add(1), max),
        Cmp::Ge => ResultSet::interval(universe, k, max),
    }
}

/// What a test's **true** edge says of a contract class, within a set.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Class {
    /// On the set, the test holds exactly where the class holds.
    Holds,
    /// On the set, the test holds exactly where the class fails.
    Fails,
    /// Neither: the test classifies some value of the set otherwise.
    Undecided,
}

/// The class `test` reads of `class` within `within` (sets of one universe).
#[must_use]
pub fn test_class(within: &ResultSet, test: &ResultSet, class: &ResultSet) -> Class {
    let tested = within.intersection(test);
    if tested == within.intersection(class) {
        Class::Holds
    } else if tested == within.difference(class) {
        Class::Fails
    } else {
        Class::Undecided
    }
}

/// What one edge of the test says of the class: `Some(true)` the class holds,
/// `Some(false)` it fails, `None` nothing.
#[must_use]
pub const fn edge(class: Class, true_edge: bool) -> Option<bool> {
    match class {
        Class::Holds => Some(true_edge),
        Class::Fails => Some(!true_edge),
        Class::Undecided => None,
    }
}

/// The set a reading uses: the declared set while it is in force, otherwise
/// the whole universe (a set out of force is never read).
#[must_use]
pub fn reading_set(universe: Interval, in_force: Option<&ResultSet>) -> ResultSet {
    in_force.map_or_else(|| ResultSet::full(universe), Clone::clone)
}

/// A test that reads a class exactly on a set reads it the same way on every
/// non-empty subset: a narrower known set never takes a decision away (the
/// Lean model's `narrowing_keeps_the_class`).
#[must_use]
pub fn narrowing_never_undecides(
    within: &ResultSet,
    narrower: &ResultSet,
    test: &ResultSet,
    class: &ResultSet,
) -> bool {
    let wide = test_class(within, test, class);
    !narrower.is_subset(within)
        || narrower.is_empty()
        || wide == Class::Undecided
        || test_class(narrower, test, class) == wide
}

/// `i32`'s universe, the vectors' type.
pub const INT: Interval = (-2_147_483_648, 2_147_483_647);

/// `(written set, canonical spelling or the refusal's code)` in `int`, shared
/// with `QCore.ResultSet.setVectors` entry for entry. Every operator appears,
/// alone and mixed, with the empty and out-of-type refusals.
pub const SET_VECTORS: &[(&str, &str)] = &[
    ("-1 | 0", "[-1, 0]"),
    ("-1 | [0, MAX]", "[-1, MAX]"),
    ("[MIN, -2] | [1, MAX]", "[MIN, -2] | [1, MAX]"),
    ("[-1, 5] & [0, MAX]", "[0, 5]"),
    ("[-1, 0] ^ 0", "-1"),
    ("[MIN, MAX] ^ 0", "[MIN, -1] | [1, MAX]"),
    ("-1 | [0, 3] & [2, 9]", "-1 | [2, 3]"),
    ("(-1 | [0, 3]) & [2, 9]", "[2, 3]"),
    ("-1 | 0 ^ 0", "result-set-empty"),
    ("(-1 | 0) ^ 0", "-1"),
    ("[0, 9] ^ [5, 12] & [0, 7]", "[0, 4] | [8, 9]"),
    ("MIN | MAX", "MIN | MAX"),
    ("[0, 5] & [6, 9]", "result-set-empty"),
    ("0 ^ 0", "result-set-empty"),
    ("[5, 1]", "result-set-empty"),
    ("2147483648", "result-set-outside-type"),
    ("[0, MAX] | ([3, 4] & 9)", "result-set-empty"),
];

/// `(written set, value, member)` in `int`, shared with
/// `QCore.ResultSet.memberVectors`: the interval implementation agrees with
/// the operators' meaning (∪ ∩ △) at these points.
pub const MEMBER_VECTORS: &[(&str, i128, bool)] = &[
    ("-1 | 0", -1, true),
    ("-1 | 0", 0, true),
    ("-1 | 0", 1, false),
    ("-1 | 0", -2, false),
    ("-1 | [0, MAX]", 2_147_483_647, true),
    ("-1 | [0, MAX]", -2, false),
    ("[-1, 5] & [0, MAX]", -1, false),
    ("[-1, 5] & [0, MAX]", 5, true),
    ("[-1, 0] ^ 0", 0, false),
    ("[-1, 0] ^ 0", -1, true),
    ("[MIN, MAX] ^ 0", 0, false),
    ("[MIN, MAX] ^ 0", -2_147_483_648, true),
    ("[0, 9] ^ [5, 12] & [0, 7]", 6, false),
    ("[0, 9] ^ [5, 12] & [0, 7]", 8, true),
];

/// `(known set or "" for none in force, test operator, test literal, contract
/// class written as a set, expected class)`, shared with
/// `QCore.ResultSet.classVectors`. The class `#result == 0` is `0`;
/// `#result != -1` is `[MIN, -2] | [0, MAX]`.
pub const CLASS_VECTORS: &[(&str, Cmp, i128, &str, Class)] = &[
    // Fluent Bit's `ret != 0` at a channel creator: exact on every int.
    ("", Cmp::Ne, 0, "0", Class::Fails),
    ("-1 | 0", Cmp::Ne, 0, "0", Class::Fails),
    // `ret == -1`: exact only under {-1, 0}.
    ("", Cmp::Eq, -1, "0", Class::Undecided),
    ("-1 | 0", Cmp::Eq, -1, "0", Class::Fails),
    // `ret < 0` (Monkey's scheduler): exact only under the set.
    ("", Cmp::Lt, 0, "0", Class::Undecided),
    ("-1 | 0", Cmp::Lt, 0, "0", Class::Fails),
    ("-1 | 0", Cmp::Ge, 0, "0", Class::Holds),
    ("-1 | 0", Cmp::Eq, 0, "0", Class::Holds),
    // A constant the body never returns: undecided, never smoothed over.
    ("-1 | 0", Cmp::Eq, -2, "0", Class::Undecided),
    ("-1 | 0", Cmp::Ne, -2, "0", Class::Undecided),
    // `timeout_fd > 0` reads 0 (a success) as failure: undecided.
    ("-1 | [0, MAX]", Cmp::Gt, 0, "[MIN, -2] | [0, MAX]", Class::Undecided),
    ("-1 | [0, MAX]", Cmp::Ge, 0, "[MIN, -2] | [0, MAX]", Class::Holds),
    ("-1 | [0, MAX]", Cmp::Lt, 0, "[MIN, -2] | [0, MAX]", Class::Fails),
    ("", Cmp::Eq, -1, "[MIN, -2] | [0, MAX]", Class::Fails),
    ("", Cmp::Lt, 0, "[MIN, -2] | [0, MAX]", Class::Undecided),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn combine_normalises() {
        let set = ResultSet::from_intervals(INT, &[(3, 4), (0, 1), (2, 2), (9, 9)]);
        assert_eq!(set.intervals(), &[(0, 4), (9, 9)]);
        assert_eq!(set.canonical(), "[0, 4] | 9");
        assert!(ResultSet::full(INT).complement().is_empty());
    }

    #[test]
    fn edges_follow_the_class() {
        assert_eq!(edge(Class::Holds, true), Some(true));
        assert_eq!(edge(Class::Holds, false), Some(false));
        assert_eq!(edge(Class::Fails, true), Some(false));
        assert_eq!(edge(Class::Fails, false), Some(true));
        assert_eq!(edge(Class::Undecided, true), None);
    }
}
