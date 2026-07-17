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
fn parses_fee_aware_tradeability_fields() {
    let expr = parse_filter(
        "fee_tradeability_state == \"costly\" and fee_expected_round_trip_cost_bps > 25",
    )
    .expect("fee-aware filter parses");
    assert_eq!(
        expr,
        Expr::And(
            Box::new(Expr::Compare {
                left: ValueExpr::Field(Field::FeeTradeabilityState),
                op: CmpOp::Eq,
                right: ValueExpr::String("costly".to_owned()),
            }),
            Box::new(Expr::Compare {
                left: ValueExpr::Field(Field::FeeExpectedRoundTripCostBps),
                op: CmpOp::Gt,
                right: ValueExpr::Number(25.0),
            })
        )
    );

    let sort = parse_sort("fee_expected_round_trip_cost_bps:asc").expect("fee-aware sort parses");
    assert_eq!(
        sort.value,
        ValueExpr::Field(Field::FeeExpectedRoundTripCostBps)
    );
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

#[test]
fn rejects_excessive_parenthesis_nesting_without_overflowing_the_stack() {
    let nesting = 257;
    let filter = format!("{}price > 1{}", "(".repeat(nesting), ")".repeat(nesting));

    let error = parse_filter(&filter).expect_err("deeply nested filters must be bounded");

    assert!(error.to_string().contains("nesting"));
}

#[test]
fn bounds_total_boolean_filter_complexity() {
    let accepted = std::iter::repeat_n("price > 0", 257)
        .collect::<Vec<_>>()
        .join(" and ");
    parse_filter(&accepted).expect("the documented boolean-operator limit is accepted");

    let rejected = std::iter::repeat_n("price > 0", 258)
        .collect::<Vec<_>>()
        .join(" or ");
    let error = parse_filter(&rejected).expect_err("filters above the limit must be rejected");

    assert!(error.to_string().contains("filter complexity exceeds"));
}
