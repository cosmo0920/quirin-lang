//! Complete typed condition inspection, not proof/evidence or an installed summary.
use qcore_frontend::frontend::{SourceContractClauseKind as K, SourceDeclaration};
use qcore_schemas::quant_condition::Definition;
use std::collections::BTreeSet;
pub fn declaration(declaration: &SourceDeclaration) -> Result<Vec<Definition>, String> {
    let mut ids = BTreeSet::new();
    let mut conditions = Vec::new();
    for clause in declaration
        .function_contract
        .iter()
        .flat_map(|c| &c.clauses)
    {
        let source = &clause.value.expression_source;
        let (id, condition) = match &clause.value.kind {
            K::QuantCondition(selector) => {
                let d = qcore_frontend::quant_condition::parse(selector.clone(), source)?;
                (d.proposition.id.clone(), Some(d))
            }
            K::QuantProposition => (qcore_frontend::quant_proposition::parse(source)?.id, None),
            K::QuantEffect(_) => (qcore_frontend::quant_effect::parse(source)?.id, None),
            K::QuantTheorem => (qcore_frontend::quant_theorem::parse(source)?.id, None),
            _ => continue,
        };
        if !ids.insert(id) {
            return Err("quant-condition: duplicate formal clause ID".into());
        }
        if let Some(condition) = condition {
            conditions.push(condition);
        }
    }
    Ok(conditions)
}
