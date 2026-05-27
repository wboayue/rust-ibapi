use super::*;
use crate::common::test_utils::wire_enum::{check_wire_enum_rejects_unknown, check_wire_enum_round_trip};

#[test]
fn report_type_display_round_trip() {
    check_wire_enum_round_trip(&[
        (FundamentalReportType::ReportsFinSummary, "ReportsFinSummary"),
        (FundamentalReportType::ReportSnapshot, "ReportSnapshot"),
        (FundamentalReportType::RESC, "RESC"),
        (FundamentalReportType::CalendarReport, "CalendarReport"),
    ]);
}

#[test]
fn report_type_from_str_rejects_unknown() {
    // ReportRatios and ReportsFinStatements appeared in older IBKR docs but
    // current TWS rejects them with code 430 — verify they round-trip as unknown.
    check_wire_enum_rejects_unknown::<FundamentalReportType>(&["", "ReportRatios", "ReportsFinStatements", "snapshot", "X"]);
}
