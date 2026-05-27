use super::*;
use crate::Error;
use std::str::FromStr;

#[test]
fn report_type_display_round_trip() {
    let cases = [
        (FundamentalReportType::ReportsFinSummary, "ReportsFinSummary"),
        (FundamentalReportType::ReportSnapshot, "ReportSnapshot"),
        (FundamentalReportType::ReportRatios, "ReportRatios"),
        (FundamentalReportType::ReportsFinStatements, "ReportsFinStatements"),
        (FundamentalReportType::RESC, "RESC"),
        (FundamentalReportType::CalendarReport, "CalendarReport"),
    ];
    for (variant, wire) in cases {
        assert_eq!(variant.to_string(), wire);
        assert_eq!(FundamentalReportType::from_str(wire).unwrap(), variant);
    }
}

#[test]
fn report_type_from_str_rejects_unknown() {
    match FundamentalReportType::from_str("ReportsFinStmts") {
        Err(Error::Parse(_, raw, msg)) => {
            assert_eq!(raw, "ReportsFinStmts");
            assert!(msg.contains("unknown FundamentalReportType"));
        }
        other => panic!("expected Error::Parse, got {other:?}"),
    }
}

#[test]
fn report_type_from_str_rejects_empty() {
    assert!(matches!(FundamentalReportType::from_str(""), Err(Error::Parse(_, _, _))));
}
