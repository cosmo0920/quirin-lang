//! The positions of the frozen route map
//! (`docs/reports/gateway-analysis-path-optimization/ROUTE-MAP.md`).

use std::fmt;
use std::str::FromStr;

/// One position of the route map. A package is placed at a position either
/// as a row of its ordered table (first answer wins) or as an arm that amends
/// one predicate of the host at that position.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Position {
    /// §1: a hold or rewrite of the unit's token stream before the parser
    /// (review and pin holds, rule 32a's caller-frame hold).
    TokenHold,
    /// §2.1 rows 1-3: asked before a statement is classified (11o boundary,
    /// 11h static declaration, 11b pointer test).
    StatementPre,
    /// §2.2: `classify_simple` and its stages, including the lexical gate.
    Classification,
    /// §2.2 and §6: the call holds before the gate (review, pin, refuted
    /// contract, reserved spelling, literal premise, declared ABI, declared
    /// modes).
    CallHold,
    /// §2.1 rows 5-6: replaces the classified instruction (12h actuals, 11o
    /// no-return exit).
    StatementRewrite,
    /// §2.1 rows 7-21: the statement fallbacks of a refusal.
    StatementFallback,
    /// §2.1 rows 22-23: parity records and refusal attribution, which never
    /// admit.
    StatementAttribution,
    /// §3: a condition's routes.
    Condition,
    /// §4 steps 1-2: the return routes of `return_statement`.
    ReturnRoute,
    /// §4 step 3: the return fallbacks of a refused return.
    ReturnFallback,
    /// §7: an arm of a Clang-typed reader (the typed access path, its value
    /// and argument positions).
    TypedReader,
}

impl Position {
    /// Every position, in route-map order.
    pub const ALL: [Self; 11] = [
        Self::TokenHold,
        Self::StatementPre,
        Self::Classification,
        Self::CallHold,
        Self::StatementRewrite,
        Self::StatementFallback,
        Self::StatementAttribution,
        Self::Condition,
        Self::ReturnRoute,
        Self::ReturnFallback,
        Self::TypedReader,
    ];

    /// The registry's spelling.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::TokenHold => "token-hold",
            Self::StatementPre => "statement-pre",
            Self::Classification => "classification",
            Self::CallHold => "call-hold",
            Self::StatementRewrite => "statement-rewrite",
            Self::StatementFallback => "statement-fallback",
            Self::StatementAttribution => "statement-attribution",
            Self::Condition => "condition",
            Self::ReturnRoute => "return-route",
            Self::ReturnFallback => "return-fallback",
            Self::TypedReader => "typed-reader",
        }
    }

    /// The route-map section that documents the position.
    #[must_use]
    pub const fn route_map_section(self) -> &'static str {
        match self {
            Self::TokenHold => "§1",
            Self::StatementPre | Self::StatementRewrite | Self::StatementAttribution => "§2.1",
            Self::StatementFallback => "§2.1 rows 7-21",
            Self::Classification => "§2.2",
            Self::CallHold => "§2.2, §6",
            Self::Condition => "§3",
            Self::ReturnRoute => "§4 steps 1-2",
            Self::ReturnFallback => "§4 step 3",
            Self::TypedReader => "§7",
        }
    }

    /// Whether an admission at this position *narrows*: a hold admits a call
    /// into its held (`Unknown`) spelling, so its refusal is the wider
    /// reading. A package placed here can never be switched off by refusal.
    #[must_use]
    pub const fn is_hold(self) -> bool {
        matches!(self, Self::TokenHold | Self::CallHold)
    }
}

impl fmt::Display for Position {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.name())
    }
}

/// A position name the route map does not hold.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UnknownPosition(pub String);

impl fmt::Display for UnknownPosition {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "unknown route-map position `{}`", self.0)
    }
}

impl FromStr for Position {
    type Err = UnknownPosition;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        Self::ALL
            .into_iter()
            .find(|position| position.name() == text)
            .ok_or_else(|| UnknownPosition(text.to_owned()))
    }
}

#[cfg(test)]
mod tests {
    use super::Position;

    #[test]
    fn every_position_round_trips_its_name_and_unknown_names_are_refused() {
        for position in Position::ALL {
            assert_eq!(position.name().parse::<Position>(), Ok(position));
        }
        assert!("statement_fallback".parse::<Position>().is_err());
        assert!("".parse::<Position>().is_err());
        let holds: Vec<_> = Position::ALL.into_iter().filter(|p| p.is_hold()).collect();
        assert_eq!(holds, [Position::TokenHold, Position::CallHold]);
    }
}
