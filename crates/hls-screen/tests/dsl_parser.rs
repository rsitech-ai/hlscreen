use hls_screen::dsl::ast::{CmpOp, Expr, Field, SortDirection, SortField, ValueExpr};
use hls_screen::dsl::parser::{parse_filter, parse_sort};

#[test]
fn parses_boolean_comparisons_literals_and_abs_field() {
    let parsed = parse_filter(r#"symbol == "@107" or abs(ret_5m) > 0.01 and spread_bps < 30"#)
        .expect("filter parses");

    assert_eq!(
        parsed,
        Expr::Or(
            Box::new(Expr::Compare {
                left: ValueExpr::Field(Field::Symbol),
                op: CmpOp::Eq,
                right: ValueExpr::String("@107".to_owned()),
            }),
            Box::new(Expr::And(
                Box::new(Expr::Compare {
                    left: ValueExpr::Abs(Field::Ret5m),
                    op: CmpOp::Gt,
                    right: ValueExpr::Number(0.01),
                }),
                Box::new(Expr::Compare {
                    left: ValueExpr::Field(Field::SpreadBps),
                    op: CmpOp::Lt,
                    right: ValueExpr::Number(30.0),
                }),
            )),
        )
    );
}

#[test]
fn parses_sort_fields_and_rejects_unknown_identifiers() {
    assert_eq!(
        parse_sort("abs(ret_5m):desc").expect("sort parses"),
        SortField {
            value: ValueExpr::Abs(Field::Ret5m),
            direction: SortDirection::Desc,
        }
    );

    let err = parse_filter("unknown_score > 10").expect_err("unknown field rejected");
    assert!(err.to_string().contains("unknown field"));

    let err = parse_filter("sqrt(ret_5m) > 0").expect_err("unknown function rejected");
    assert!(err.to_string().contains("unknown function"));
}

#[test]
fn parses_score_component_fields_with_dot_notation() {
    assert_eq!(
        parse_filter("score_component.spread_cost < 0").expect("filter parses"),
        Expr::Compare {
            left: ValueExpr::Field(Field::ScoreComponent("spread_cost".to_owned())),
            op: CmpOp::Lt,
            right: ValueExpr::Number(0.0),
        }
    );
    assert_eq!(
        parse_sort("score_total:desc").expect("sort parses"),
        SortField {
            value: ValueExpr::Field(Field::ScoreTotal),
            direction: SortDirection::Desc,
        }
    );
}

#[test]
fn rejects_non_finite_numeric_literals() {
    let huge = "9".repeat(400);
    let error =
        parse_filter(&format!("price > {huge}")).expect_err("filter literals must remain finite");

    assert!(error.to_string().contains("finite"));
}
