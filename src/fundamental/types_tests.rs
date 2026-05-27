use super::*;
use crate::common::test_utils::wire_enum::{check_wire_enum_rejects_unknown, check_wire_enum_round_trip};

#[test]
fn report_type_display_round_trip() {
    check_wire_enum_round_trip(&[
        (FundamentalReportType::ReportsFinSummary, "ReportsFinSummary"),
        (FundamentalReportType::ReportSnapshot, "ReportSnapshot"),
        (FundamentalReportType::ReportRatios, "ReportRatios"),
        (FundamentalReportType::ReportsFinStatements, "ReportsFinStatements"),
        (FundamentalReportType::RESC, "RESC"),
        (FundamentalReportType::CalendarReport, "CalendarReport"),
    ]);
}

#[test]
fn report_type_from_str_rejects_unknown() {
    check_wire_enum_rejects_unknown::<FundamentalReportType>(&["", "ReportsFinStmts", "snapshot", "X"]);
}
