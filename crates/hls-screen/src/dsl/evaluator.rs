use std::cmp::Ordering;

use hls_core::{HlsError, HlsResult};

use crate::{
    dsl::ast::{CmpOp, Expr, SortDirection, SortField, ValueExpr},
    row::{FieldValue, ScreenRow},
};

impl Expr {
    pub fn matches(&self, row: ScreenRow<'_>) -> HlsResult<bool> {
        match self {
            Self::Compare { left, op, right } => {
                compare_values(left.eval(row), *op, right.eval(row))
            }
            Self::And(left, right) => Ok(left.matches(row)? && right.matches(row)?),
            Self::Or(left, right) => Ok(left.matches(row)? || right.matches(row)?),
        }
    }
}

impl ValueExpr {
    pub fn eval(&self, row: ScreenRow<'_>) -> FieldValue {
        match self {
            Self::Field(field) => row.value(field),
            Self::Number(value) => FieldValue::Number(*value),
            Self::String(value) => FieldValue::String(value.clone()),
            Self::Bool(value) => FieldValue::Bool(*value),
            Self::Abs(field) => match row.value(field) {
                FieldValue::Number(value) => FieldValue::Number(value.abs()),
                _ => FieldValue::Missing,
            },
        }
    }
}

impl SortField {
    pub fn compare(&self, left: ScreenRow<'_>, right: ScreenRow<'_>) -> Ordering {
        let left = self.value.eval(left);
        let right = self.value.eval(right);
        let missing_ordering = compare_missing(&left, &right);
        if missing_ordering != Ordering::Equal {
            return missing_ordering;
        }

        let ordering = compare_present_sort_values(left, right);
        match self.direction {
            SortDirection::Asc => ordering,
            SortDirection::Desc => ordering.reverse(),
        }
    }
}

fn compare_values(left: FieldValue, op: CmpOp, right: FieldValue) -> HlsResult<bool> {
    match (left, right) {
        (FieldValue::Missing, _) | (_, FieldValue::Missing) => Ok(false),
        (FieldValue::Number(left), FieldValue::Number(right)) => compare_ordered(left, op, right),
        (FieldValue::String(left), FieldValue::String(right)) => compare_eq(left == right, op),
        (FieldValue::Bool(left), FieldValue::Bool(right)) => compare_eq(left == right, op),
        (left, right) => Err(HlsError::Config(format!(
            "type-incompatible comparison between {left:?} and {right:?}"
        ))),
    }
}

fn compare_ordered(left: f64, op: CmpOp, right: f64) -> HlsResult<bool> {
    if !left.is_finite() || !right.is_finite() {
        return Ok(false);
    }

    Ok(match op {
        CmpOp::Gt => left > right,
        CmpOp::Gte => left >= right,
        CmpOp::Lt => left < right,
        CmpOp::Lte => left <= right,
        CmpOp::Eq => left == right,
        CmpOp::Ne => left != right,
    })
}

fn compare_eq(equal: bool, op: CmpOp) -> HlsResult<bool> {
    match op {
        CmpOp::Eq => Ok(equal),
        CmpOp::Ne => Ok(!equal),
        _ => Err(HlsError::Config(
            "ordered comparisons require numeric values".to_owned(),
        )),
    }
}

fn compare_missing(left: &FieldValue, right: &FieldValue) -> Ordering {
    match (left, right) {
        (FieldValue::Missing, FieldValue::Missing) => Ordering::Equal,
        (FieldValue::Missing, _) => Ordering::Greater,
        (_, FieldValue::Missing) => Ordering::Less,
        _ => Ordering::Equal,
    }
}

fn compare_present_sort_values(left: FieldValue, right: FieldValue) -> Ordering {
    match (left, right) {
        (FieldValue::Number(left), FieldValue::Number(right)) => {
            left.partial_cmp(&right).unwrap_or(Ordering::Equal)
        }
        (FieldValue::String(left), FieldValue::String(right)) => left.cmp(&right),
        (FieldValue::Bool(left), FieldValue::Bool(right)) => left.cmp(&right),
        (left, right) => format!("{left:?}").cmp(&format!("{right:?}")),
    }
}
