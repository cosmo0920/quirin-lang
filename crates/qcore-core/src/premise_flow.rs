//! Pure three-valued logical flow. No evidence or source authority is admitted.
use crate::hypothesis::Disposition;
use Disposition::{Open, Refuted, Supported};
#[must_use]
pub fn and(left: Disposition, right: Disposition) -> Disposition {
    match (left, right) {
        (Refuted, _) | (_, Refuted) => Refuted,
        (Supported, Supported) => Supported,
        _ => Open,
    }
}
#[must_use]
pub fn or(left: Disposition, right: Disposition) -> Disposition {
    match (left, right) {
        (Supported, _) | (_, Supported) => Supported,
        (Refuted, Refuted) => Refuted,
        _ => Open,
    }
}
#[must_use]
pub fn except(left: Disposition, forbidden: Disposition) -> Disposition {
    match (left, forbidden) {
        (Refuted, _) | (_, Supported) => Refuted,
        (Supported, Refuted) => Supported,
        _ => Open,
    }
}
#[must_use]
pub fn terminal(left: Disposition, condition: Disposition) -> (Disposition, bool) {
    let status = and(left, condition);
    (status, status == Supported)
}
