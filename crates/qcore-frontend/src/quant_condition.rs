//! Typed entry/exit assertions. No predicate is evaluated or source-bound here.
use qcore_schemas::quant_condition::{Definition, Selector};
fn selector_valid(selector: &Selector) -> bool {
    match selector {
        Selector::Error(name) => {
            !name.trim().is_empty() && name.len() <= 1024 && !name.chars().any(char::is_control)
        }
        _ => true,
    }
}
/// Parse the expression after the explicit selector, excluding the semicolon.
pub fn parse(selector: Selector, expression: &str) -> Result<Definition, String> {
    if !selector_valid(&selector) {
        return Err("quant-condition: invalid error selector".into());
    }
    let proposition = crate::quant_proposition::parse(expression)?;
    Ok(Definition {
        selector,
        proposition,
    })
}
pub fn validate(definition: &Definition) -> Result<(), String> {
    if !selector_valid(&definition.selector) {
        return Err("quant-condition: invalid error selector".into());
    }
    crate::quant_proposition::validate(&definition.proposition)
}
