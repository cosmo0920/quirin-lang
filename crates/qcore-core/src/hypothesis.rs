//! Pure conjunction rule. A provenance label supplies no truth or authority.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Disposition {
    Open,
    Supported,
    Refuted,
}
/// Reporting composition only: no caller may use this as certificate admission.
#[must_use]
pub fn conjunction(atoms: &[(bool, Disposition)]) -> Disposition {
    if atoms.is_empty() || atoms.iter().any(|(same, _)| !same) {
        Disposition::Open
    } else if atoms
        .iter()
        .any(|(_, status)| *status == Disposition::Refuted)
    {
        Disposition::Refuted
    } else if atoms
        .iter()
        .all(|(_, status)| *status == Disposition::Supported)
    {
        Disposition::Supported
    } else {
        Disposition::Open
    }
}
