use super::{analyzable::AnalyzableField, halo2_proofs_libs::*};
use anyhow::{anyhow,Context, Result};
use std::collections::HashSet;

// abstract interpretation of expressions

// simplest possible abstract domain for expressions
#[derive(Debug, Eq, PartialEq, PartialOrd, Ord)]
pub enum AbsResult {
    Variable,
    NonZero,
    Zero,
}
/// Extracts columns and rotations from an expression.
///
/// This function traverses an expression tree and extracts the columns and rotations used within the expression.
/// It recursively examines the expression and adds any encountered `Expression::Advice` columns and their corresponding rotations
/// to the resulting set.
pub fn extract_columns<F: Field>(expr: &Expression<F>) -> HashSet<(Column<Any>, Rotation)> {
    fn recursion<F: Field>(dst: &mut HashSet<(Column<Any>, Rotation)>, expr: &Expression<F>) {
        match expr {
            #[cfg(feature = "use_zcash_halo2_proofs")]
            Expression::Advice(advice_query) => {
                let column = Column {
                    index: advice_query.column_index,
                    column_type: Advice {},
                };
                dst.insert((column.into(), advice_query.rotation));
            }
            #[cfg(any(feature = "use_pse_halo2_proofs", feature = "use_axiom_halo2_proofs",))]
            Expression::Advice(advice_query) => {
                let column = Column {
                    index: advice_query.column_index,
                    column_type: Advice{ phase: advice_query.phase },
                };
                dst.insert((column.into(), advice_query.rotation));
            }
            Expression::Sum(left, right) => {
                recursion(dst, left);
                recursion(dst, right);
            }
            Expression::Product(left, right) => {
                recursion(dst, left);
                recursion(dst, right);
            }
            Expression::Negated(expr) => recursion(dst, expr),
            Expression::Scaled(expr, _) => recursion(dst, expr),
            _ => (),
        }
    }
    let mut set = HashSet::new();
    recursion(&mut set, expr);
    set
}
/// Evaluates an abstract expression and returns the abstract result.
///
/// This function evaluates an abstract expression and returns an abstract result based on the provided selectors.
/// It recursively traverses the expression tree and applies the corresponding evaluation rules to determine the result.
/// The abstract result can be one of the following: `AbsResult::Zero`, `AbsResult::NonZero`, or `AbsResult::Variable`.
///
pub fn eval_abstract<F: AnalyzableField>(
    expr: &Expression<F>,
    selectors: &HashSet<Selector>,
    region_begin: usize,
    region_end: usize,
    row_num: i32,
    fixed: &Vec<Vec<CellValue<F>>>,
) -> Result<AbsResult> {
    match expr {
        Expression::Constant(v) => {
            if v.is_zero().into() {
                Ok(AbsResult::Zero)
            } else {
                Ok(AbsResult::NonZero)
            }
        }
        Expression::Selector(selector) => match selectors.contains(selector) {
            true => Ok(AbsResult::NonZero),
            false => Ok(AbsResult::Zero),
        },
        Expression::Fixed(fixed_query) 
        => 
        {
            let col = fixed_query.column_index;
            let row = (fixed_query.rotation.0 + row_num) as usize + region_begin;

            let mut t = 0;
            if let CellValue::Assigned(fixed_val) = fixed[col][row] {
                t  = u64::from_str_radix(format!("{:?}",fixed_val).strip_prefix("0x").unwrap(), 16).unwrap();
            }
            if t == 0 {
                Ok(AbsResult::Zero)
            } else {
                Ok(AbsResult::Variable)
            }
        }
        Expression::Advice { .. } => Ok(AbsResult::Variable),
        Expression::Instance { .. } => Ok(AbsResult::Variable),
        Expression::Negated(expr) => eval_abstract(expr, selectors,region_begin,region_end,row_num,fixed),
        Expression::Sum(left, right) => {
            let res1 = eval_abstract(left, selectors,region_begin,region_end,row_num,fixed).with_context(|| format!(
                                    "Failed to run abstract evaluation for polynomial at region from row: {} to {}, , at row: {}.",
                                    region_begin, region_end,row_num
                                ))?;
            let res2 = eval_abstract(right, selectors,region_begin,region_end,row_num,fixed).with_context(|| format!(
                                    "Failed to run abstract evaluation for polynomial at region from row: {} to {}, , at row: {}.",
                                    region_begin, region_end, row_num
                                ))?;
            match (res1, res2) {
                (AbsResult::Variable, _) => Ok(AbsResult::Variable),
                (_, AbsResult::Variable) => Ok(AbsResult::Variable),
                (AbsResult::NonZero, AbsResult::NonZero) => Ok(AbsResult::Variable),
                (AbsResult::Zero, AbsResult::Zero) => Ok(AbsResult::Zero),
                (AbsResult::Zero, AbsResult::NonZero) => Ok(AbsResult::NonZero),
                (AbsResult::NonZero, AbsResult::Zero) => Ok(AbsResult::NonZero),
            }
        }
        Expression::Product(left, right) => {
            let res1 = eval_abstract(left, selectors,region_begin,region_end,row_num,fixed).with_context(|| format!(
                                    "Failed to run abstract evaluation for polynomial at region from row: {} to {}, at row: {}.",
                                    region_begin, region_end, row_num
                                ))?;
            let res2 = eval_abstract(right, selectors,region_begin,region_end,row_num,fixed).with_context(|| format!(
                                    "Failed to run abstract evaluation for polynomial at region from row: {} to {}, at row: {}.",
                                    region_begin, region_end, row_num
                                ))?;
            match (res1, res2) {
                (AbsResult::Zero, _) => Ok(AbsResult::Zero),
                (_, AbsResult::Zero) => Ok(AbsResult::Zero),
                (AbsResult::NonZero, AbsResult::NonZero) => Ok(AbsResult::NonZero),
                _ => Ok(AbsResult::Variable),
            }
        }
        Expression::Scaled(expr, scale) => {
            if scale.is_zero().into() {
                Ok(AbsResult::Zero)
            } else {
                eval_abstract(expr, selectors,region_begin,region_end,row_num,fixed)
            }
        }
        #[cfg(any(feature = "use_pse_halo2_proofs", feature = "use_axiom_halo2_proofs",feature = "use_scroll_halo2_proofs"))]
        Expression::Challenge(_) => Err(anyhow!("Challenge expression in abstract evaluation resulted in Invalid Expression")),
    }
}