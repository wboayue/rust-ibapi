use super::*;
use crate::contracts::{LegAction, SecurityType};
use crate::orders::{Execution, ExecutionSide, OptionExerciseType};

const MAX_WAIT: Duration = Duration::from_secs(5);

fn execution(execution_id: &str) -> ExecutionData {
    ExecutionData {
        request_id: 9001,
        execution: Execution {
            execution_id: execution_id.to_string(),
            shares: 3.0,
            price: 5812.25,
            ..Default::default()
        },
        ..Default::default()
    }
}

fn exercise(execution_id: &str) -> ExecutionData {
    let mut data = execution(execution_id);
    data.execution.option_exercise_type = OptionExerciseType::Exercise;
    data
}

fn combo(execution_id: &str) -> ExecutionData {
    let mut data = execution(execution_id);
    data.contract.security_type = Some(SecurityType::Spread);
    data
}

fn no_report_for_combos(data: &ExecutionData) -> bool {
    data.contract.security_type != Some(SecurityType::Spread)
}

fn report(execution_id: &str, commission_and_fees: f64) -> CommissionAndFeesReport {
    CommissionAndFeesReport {
        execution_id: execution_id.to_string(),
        commission_and_fees,
        currency: "USD".to_string(),
        ..Default::default()
    }
}

fn assert_fill(fill: &Fill, execution_id: &str, commission_and_fees: Option<f64>) {
    assert_eq!(fill.execution.execution.execution_id, execution_id, "fill execution_id");
    assert_eq!(fill.execution.request_id, 9001, "fill request_id");
    assert_eq!(fill.execution.execution.shares, 3.0, "fill shares");
    match (&fill.commission, commission_and_fees) {
        (Some(report), Some(expected)) => {
            assert_eq!(report.execution_id, execution_id, "report execution_id");
            assert_eq!(report.commission_and_fees, expected, "report commission_and_fees");
            assert_eq!(report.currency, "USD", "report currency");
        }
        (None, None) => {}
        (actual, expected) => panic!("commission {actual:?}, expected {expected:?}"),
    }
}

fn ids(executions: &[ExecutionData]) -> Vec<&str> {
    executions.iter().map(|data| data.execution.execution_id.as_str()).collect()
}

fn report_ids(reports: &[CommissionAndFeesReport]) -> Vec<&str> {
    reports.iter().map(|report| report.execution_id.as_str()).collect()
}

fn fill_ids(fills: &[Fill]) -> Vec<(&str, Option<f64>)> {
    fills
        .iter()
        .map(|fill| {
            (
                fill.execution.execution.execution_id.as_str(),
                fill.commission.as_ref().map(|report| report.commission_and_fees),
            )
        })
        .collect()
}

fn assert_nothing_waits(matcher: &FillMatcher) {
    assert!(matcher.executions.is_empty(), "executions held: {:?}", matcher.executions);
    assert!(matcher.reports.is_empty(), "reports held: {:?}", matcher.reports);
}

fn assert_late(result: ReportMatch, execution_id: &str, commission_and_fees: f64) {
    match result {
        ReportMatch::Late(report) => {
            assert_eq!(report.execution_id, execution_id);
            assert_eq!(report.commission_and_fees, commission_and_fees);
        }
        other => panic!("expected Late, got {other:?}"),
    }
}

fn assert_duplicate(result: ReportMatch, execution_id: &str) {
    match result {
        ReportMatch::Duplicate(report) => assert_eq!(report.execution_id, execution_id),
        other => panic!("expected Duplicate, got {other:?}"),
    }
}

#[test]
fn execution_then_report_pairs() {
    let now = Instant::now();
    let mut matcher = FillMatcher::new();

    let pushed = matcher.push_execution_at(execution("e1"), now);
    let paired = matcher.push_report_at(report("e1", 1.25), now);

    assert!(pushed.is_none(), "execution alone must wait, got {pushed:?}");
    match paired {
        ReportMatch::Paired(fill) => assert_fill(&fill, "e1", Some(1.25)),
        other => panic!("expected Paired, got {other:?}"),
    }
    assert_nothing_waits(&matcher);
}

#[test]
fn report_then_execution_pairs() {
    let now = Instant::now();
    let mut matcher = FillMatcher::new();

    let pushed = matcher.push_report_at(report("e1", 1.25), now);
    let fill = matcher.push_execution_at(execution("e1"), now).expect("report was waiting");

    assert!(matches!(pushed, ReportMatch::Waiting), "report alone must wait, got {pushed:?}");
    assert_fill(&fill, "e1", Some(1.25));
    assert_nothing_waits(&matcher);
}

#[test]
fn interleaved_ids_pair_by_execution_id() {
    let now = Instant::now();
    let mut matcher = FillMatcher::new();

    assert!(matcher.push_execution_at(execution("e1"), now).is_none());
    assert!(matcher.push_execution_at(execution("e2"), now).is_none());
    let second = matcher.push_report_at(report("e2", 2.5), now);
    let first = matcher.push_report_at(report("e1", 1.25), now);

    match (second, first) {
        (ReportMatch::Paired(second), ReportMatch::Paired(first)) => {
            assert_fill(&second, "e2", Some(2.5));
            assert_fill(&first, "e1", Some(1.25));
        }
        other => panic!("expected two Paired, got {other:?}"),
    }
    assert_nothing_waits(&matcher);
}

#[test]
fn take_expired_waits_for_max_wait() {
    let start = Instant::now();
    let mut matcher = FillMatcher::new();
    assert!(matcher.push_execution_at(execution("e1"), start).is_none());
    assert!(matches!(matcher.push_report_at(report("r1", 0.5), start), ReportMatch::Waiting));
    assert!(matcher.push_execution_at(execution("e2"), start + Duration::from_secs(2)).is_none());

    let early = matcher.take_expired_at(MAX_WAIT, start + MAX_WAIT - Duration::from_millis(1));
    let due = matcher.take_expired_at(MAX_WAIT, start + MAX_WAIT);

    assert!(early.executions.is_empty(), "nothing has waited 5 s yet: {:?}", ids(&early.executions));
    assert!(early.reports.is_empty(), "nothing has waited 5 s yet: {:?}", report_ids(&early.reports));
    assert_eq!(ids(&due.executions), vec!["e1"], "only e1 has waited 5 s");
    assert_eq!(report_ids(&due.reports), vec!["r1"], "r1 has waited 5 s");
    assert_eq!(matcher.executions.len(), 1, "e2 still waits");
    assert_eq!(matcher.executions[0].0.execution.execution_id, "e2");
}

#[test]
fn take_expired_zero_returns_everything() {
    let now = Instant::now();
    let mut matcher = FillMatcher::new();
    assert!(matcher.push_execution_at(execution("e1"), now).is_none());
    assert!(matches!(matcher.push_report_at(report("r1", 0.5), now), ReportMatch::Waiting));

    let unpaired = matcher.take_expired_at(Duration::ZERO, now);

    assert_eq!(ids(&unpaired.executions), vec!["e1"]);
    assert_eq!(report_ids(&unpaired.reports), vec!["r1"]);
    assert_nothing_waits(&matcher);
}

#[test]
fn report_after_take_is_late_then_duplicate_however_late() {
    let start = Instant::now();
    let mut matcher = FillMatcher::new();
    assert!(matcher.push_execution_at(execution("e1"), start).is_none());
    let taken = matcher.take_expired_at(MAX_WAIT, start + MAX_WAIT);
    let much_later = start + Duration::from_secs(3600);
    matcher.take_expired_at(MAX_WAIT, much_later);

    let late = matcher.push_report_at(report("e1", 1.25), much_later);
    let again = matcher.push_report_at(report("e1", 1.25), much_later);
    let third = matcher.push_report_at(report("e1", 1.25), much_later + MAX_WAIT);

    assert_eq!(ids(&taken.executions), vec!["e1"]);
    assert_late(late, "e1", 1.25);
    assert_duplicate(again, "e1");
    assert_duplicate(third, "e1");
    assert_nothing_waits(&matcher);
}

#[test]
fn repeated_report_for_paired_execution_is_duplicate_and_never_held() {
    let now = Instant::now();
    let mut matcher = FillMatcher::new();
    assert!(matcher.push_execution_at(execution("e1"), now).is_none());
    assert!(matches!(matcher.push_report_at(report("e1", 1.25), now), ReportMatch::Paired(_)));

    let repeated = matcher.push_report_at(report("e1", 1.25), now);
    let leftover = matcher.take_expired_at(Duration::ZERO, now);

    assert_duplicate(repeated, "e1");
    assert!(
        leftover.reports.is_empty(),
        "a repeated report is never handed out: {:?}",
        leftover.reports
    );
    assert!(leftover.executions.is_empty());
}

#[test]
fn second_copy_of_a_waiting_report_is_duplicate() {
    let now = Instant::now();
    let mut matcher = FillMatcher::new();

    let first = matcher.push_report_at(report("e1", 1.25), now);
    let second = matcher.push_report_at(report("e1", 1.25), now);
    let fill = matcher.push_execution_at(execution("e1"), now).expect("report was waiting");

    assert!(matches!(first, ReportMatch::Waiting), "{first:?}");
    assert_duplicate(second, "e1");
    assert_fill(&fill, "e1", Some(1.25));
    assert_nothing_waits(&matcher);
}

#[test]
fn memory_drops_the_oldest_handout_first() {
    let now = Instant::now();
    let mut matcher = FillMatcher::new().memory(2);
    for execution_id in ["e1", "e2", "e3"] {
        assert!(matcher.push_execution_at(execution(execution_id), now).is_none());
        assert!(matches!(matcher.push_report_at(report(execution_id, 1.0), now), ReportMatch::Paired(_)));
    }

    let forgotten = matcher.push_report_at(report("e1", 1.0), now);
    let remembered = matcher.push_report_at(report("e2", 1.0), now);

    assert!(matches!(forgotten, ReportMatch::Waiting), "e1 was dropped from memory: {forgotten:?}");
    assert_duplicate(remembered, "e2");
    assert_eq!(matcher.handouts.len(), 2, "memory holds at most 2 ids");
}

#[test]
fn memory_without_a_bound_remembers_every_handout() {
    let now = Instant::now();
    let mut matcher = FillMatcher::new().memory(usize::MAX);

    assert!(matcher.push_execution_at(execution("e1"), now).is_none());
    let paired = matcher.push_report_at(report("e1", 1.25), now);
    let copy = matcher.push_report_at(report("e1", 1.25), now);

    assert!(matches!(paired, ReportMatch::Paired(_)), "{paired:?}");
    assert_duplicate(copy, "e1");
}

#[test]
fn memory_zero_remembers_nothing() {
    let now = Instant::now();
    let mut matcher = FillMatcher::new().memory(0);
    assert!(matcher.push_execution_at(execution("e1"), now).is_none());
    matcher.take_expired_at(Duration::ZERO, now);

    let report = matcher.push_report_at(report("e1", 1.25), now);

    assert!(matches!(report, ReportMatch::Waiting), "{report:?}");
    assert!(matcher.handouts.is_empty() && matcher.handout_order.is_empty());
}

#[test]
fn shrinking_memory_drops_the_oldest_at_once() {
    let now = Instant::now();
    let mut matcher = FillMatcher::new();
    for execution_id in ["e1", "e2", "e3"] {
        assert!(matcher.push_execution_at(execution(execution_id), now).is_none());
    }
    matcher.take_expired_at(Duration::ZERO, now);

    let mut matcher = matcher.memory(1);

    assert!(matches!(matcher.push_report_at(report("e2", 1.0), now), ReportMatch::Waiting));
    assert_late(matcher.push_report_at(report("e3", 1.0), now), "e3", 1.0);
}

#[test]
fn handout_order_stays_bounded_through_put_back_cycles() {
    let start = Instant::now();
    let mut matcher = FillMatcher::new().memory(4);
    assert!(matcher.push_execution_at(execution("e1"), start).is_none());

    for _ in 0..100 {
        let taken = matcher.take_expired_at(Duration::ZERO, start);
        let execution = taken.executions.into_iter().next().expect("e1 is due");
        assert!(matcher.put_back(execution).is_none());
    }

    assert!(matcher.handout_order.len() <= 9, "queue length {}", matcher.handout_order.len());
    assert!(matcher.handouts.is_empty(), "e1 waits again, so it is not a handout");
}

#[test]
fn put_back_keeps_the_original_arrival_time() {
    let start = Instant::now();
    let mut matcher = FillMatcher::new();
    assert!(matcher.push_execution_at(execution("e1"), start).is_none());
    assert!(matcher.push_execution_at(execution("e2"), start + Duration::from_secs(1)).is_none());
    let taken = matcher.take_expired_at(MAX_WAIT, start + MAX_WAIT);

    let put_back = matcher.put_back(taken.executions.into_iter().next().expect("e1 is due"));
    let order: Vec<String> = matcher.executions.iter().map(|(data, _)| data.execution.execution_id.clone()).collect();
    let again = matcher.take_expired_at(MAX_WAIT, start + MAX_WAIT);

    assert!(put_back.is_none());
    assert_eq!(order, vec!["e1", "e2"], "put back in arrival order");
    assert_eq!(ids(&again.executions), vec!["e1"], "its wait did not restart");
}

#[test]
fn put_back_execution_pairs_with_its_report() {
    let start = Instant::now();
    let mut matcher = FillMatcher::new();
    assert!(matcher.push_execution_at(execution("e1"), start).is_none());
    let taken = matcher.take_expired_at(MAX_WAIT, start + MAX_WAIT);
    assert!(matcher.put_back(taken.executions.into_iter().next().expect("e1 is due")).is_none());

    let paired = matcher.push_report_at(report("e1", 1.25), start + MAX_WAIT);

    match paired {
        ReportMatch::Paired(fill) => assert_fill(&fill, "e1", Some(1.25)),
        other => panic!("expected Paired, got {other:?}"),
    }
    assert_nothing_waits(&matcher);
}

#[test]
fn put_back_gives_back_what_it_cannot_restore() {
    let now = Instant::now();
    let mut matcher = FillMatcher::new();
    assert!(matcher.push_execution_at(execution("e1"), now).is_none());
    let taken = matcher.take_expired_at(Duration::ZERO, now).executions;
    assert_late(matcher.push_report_at(report("e1", 1.25), now), "e1", 1.25);

    let reported = matcher.put_back(taken.into_iter().next().expect("e1 is due"));
    let never_taken = matcher.put_back(execution("e9"));

    assert_eq!(reported.map(|data| data.execution.execution_id), Some("e1".to_string()));
    assert_eq!(never_taken.map(|data| data.execution.execution_id), Some("e9".to_string()));
    assert_nothing_waits(&matcher);
}

#[test]
fn exercise_is_a_fill_without_commission_at_once() {
    let now = Instant::now();
    let mut matcher = FillMatcher::new();

    let fill = matcher.push_execution_at(exercise("x1"), now).expect("an exercise gets no report");
    let late = matcher.push_report_at(report("x1", 0.0), now);

    assert_fill(&fill, "x1", None);
    assert_eq!(fill.execution.execution.option_exercise_type, OptionExerciseType::Exercise);
    assert_late(late, "x1", 0.0);
}

#[test]
fn exercise_pairs_with_a_report_that_came_first() {
    let now = Instant::now();
    let mut matcher = FillMatcher::new();
    assert!(matches!(matcher.push_report_at(report("x1", 0.75), now), ReportMatch::Waiting));

    let fill = matcher.push_execution_at(exercise("x1"), now).expect("report was waiting");

    assert_fill(&fill, "x1", Some(0.75));
    assert_nothing_waits(&matcher);
}

#[test]
fn caller_rule_returns_the_fill_at_once_and_remembers_it() {
    let now = Instant::now();
    let mut matcher = FillMatcher::new().expects_report(no_report_for_combos);

    let fill = matcher.push_execution_at(combo("b1"), now).expect("the rule expects no report");
    let normal = matcher.push_execution_at(execution("e1"), now);
    let exercise_waits = matcher.push_execution_at(exercise("x1"), now);
    let late = matcher.push_report_at(report("b1", 0.5), now);
    let repeated = matcher.push_report_at(report("b1", 0.5), now);

    assert_fill(&fill, "b1", None);
    assert!(normal.is_none(), "{normal:?}");
    assert!(exercise_waits.is_none(), "the caller's rule replaces the default: {exercise_waits:?}");
    assert_late(late, "b1", 0.5);
    assert_duplicate(repeated, "b1");
}

#[test]
fn expects_commission_report_is_false_for_exercise_and_combo() {
    let cases = [
        (OptionExerciseType::None, SecurityType::Future, true),
        (OptionExerciseType::Exercise, SecurityType::Option, false),
        (OptionExerciseType::Lapse, SecurityType::Option, true),
        (OptionExerciseType::DoNothing, SecurityType::Option, true),
        (OptionExerciseType::Assigned, SecurityType::Option, true),
        (OptionExerciseType::AutoexerciseClearing, SecurityType::Option, true),
        (OptionExerciseType::Expired, SecurityType::Option, true),
        (OptionExerciseType::AutoexerciseTrading, SecurityType::Option, true),
        (OptionExerciseType::Netting, SecurityType::Option, true),
        (OptionExerciseType::Unknown(7), SecurityType::Option, true),
        (OptionExerciseType::None, SecurityType::Spread, false),
    ];
    for (option_exercise_type, security_type, expected) in cases {
        let mut data = execution("e1");
        data.execution.option_exercise_type = option_exercise_type;
        data.contract.security_type = Some(security_type.clone());
        assert_eq!(data.expects_commission_report(), expected, "{option_exercise_type:?} {security_type:?}");
    }
}

#[test]
fn copy_of_a_paired_execution_is_dropped_before_or_after_its_report_copy() {
    let now = Instant::now();
    let mut matcher = FillMatcher::new();
    assert!(matcher.push_execution_at(execution("e1"), now).is_none());
    let first = matcher.push_report_at(report("e1", 1.25), now);
    assert!(matcher.push_execution_at(execution("e2"), now).is_none());
    let second = matcher.push_report_at(report("e2", 2.5), now);

    let e1_execution_copy = matcher.push_execution_at(execution("e1"), now);
    let e1_report_copy = matcher.push_report_at(report("e1", 1.25), now);
    let e2_report_copy = matcher.push_report_at(report("e2", 2.5), now);
    let e2_execution_copy = matcher.push_execution_at(execution("e2"), now);

    assert!(matches!(first, ReportMatch::Paired(_)), "{first:?}");
    assert!(matches!(second, ReportMatch::Paired(_)), "{second:?}");
    assert!(e1_execution_copy.is_none(), "{e1_execution_copy:?}");
    assert_duplicate(e1_report_copy, "e1");
    assert_duplicate(e2_report_copy, "e2");
    assert!(e2_execution_copy.is_none(), "{e2_execution_copy:?}");
    assert_nothing_waits(&matcher);
    let leftovers = matcher.take_expired_at(Duration::ZERO, now);
    assert!(leftovers.executions.is_empty() && leftovers.reports.is_empty(), "{leftovers:?}");
}

#[test]
fn copy_of_a_waiting_execution_is_dropped() {
    let now = Instant::now();
    let mut matcher = FillMatcher::new();
    assert!(matcher.push_execution_at(execution("e1"), now).is_none());

    let copy = matcher.push_execution_at(execution("e1"), now);
    let held = matcher.executions.len();
    let paired = matcher.push_report_at(report("e1", 1.25), now);

    assert!(copy.is_none(), "{copy:?}");
    assert_eq!(held, 1, "the copy is not held");
    match paired {
        ReportMatch::Paired(fill) => assert_fill(&fill, "e1", Some(1.25)),
        other => panic!("expected Paired, got {other:?}"),
    }
    assert_nothing_waits(&matcher);
}

#[test]
fn copy_of_an_execution_handed_out_without_report_is_dropped() {
    let now = Instant::now();
    let mut matcher = FillMatcher::new();
    assert!(matcher.push_execution_at(execution("e1"), now).is_none());
    let taken = matcher.take_expired_at(Duration::ZERO, now);
    let exercise_fill = matcher.push_execution_at(exercise("x1"), now);

    let e1_copy = matcher.push_execution_at(execution("e1"), now);
    let x1_copy = matcher.push_execution_at(exercise("x1"), now);
    let late = matcher.push_report_at(report("e1", 1.25), now);

    assert_eq!(ids(&taken.executions), vec!["e1"]);
    assert_fill(&exercise_fill.expect("an exercise expects no report"), "x1", None);
    assert!(e1_copy.is_none(), "{e1_copy:?}");
    assert!(x1_copy.is_none(), "{x1_copy:?}");
    assert_late(late, "e1", 1.25);
    assert_nothing_waits(&matcher);
}

#[test]
fn copy_of_a_forgotten_execution_is_new() {
    let now = Instant::now();
    let mut matcher = FillMatcher::new().memory(1);
    assert!(matcher.push_execution_at(execution("e1"), now).is_none());
    assert!(matches!(matcher.push_report_at(report("e1", 1.25), now), ReportMatch::Paired(_)));
    assert!(matcher.push_execution_at(execution("e2"), now).is_none());
    assert!(matches!(matcher.push_report_at(report("e2", 2.5), now), ReportMatch::Paired(_)));

    let copy = matcher.push_execution_at(execution("e1"), now);

    assert!(copy.is_none(), "{copy:?}");
    assert_eq!(matcher.executions.len(), 1, "held again");
    assert_eq!(matcher.executions[0].0.execution.execution_id, "e1");
}

#[test]
fn pair_executions_returns_fills_in_execution_order_and_leftovers() {
    let rows = vec![
        Executions::CommissionAndFeesReport(report("e2", 2.5)),
        Executions::ExecutionData(execution("e1")),
        Executions::ExecutionData(execution("e2")),
        Executions::ExecutionData(execution("e3")),
        Executions::CommissionAndFeesReport(report("e1", 1.25)),
        Executions::CommissionAndFeesReport(report("e1", 1.25)),
        Executions::ExecutionData(exercise("x1")),
        Executions::CommissionAndFeesReport(report("r9", 0.5)),
        Executions::CommissionAndFeesReport(report("x1", 0.25)),
    ];
    let mut matcher = FillMatcher::new();

    let (fills, unpaired) = matcher.pair_executions(rows);

    assert_eq!(
        fill_ids(&fills),
        vec![("e1", Some(1.25)), ("e2", Some(2.5)), ("x1", Some(0.25))],
        "execution order; the exercise's report fills in its commission; the repeated e1 report is dropped"
    );
    assert_eq!(ids(&unpaired.executions), vec!["e3"]);
    assert_eq!(report_ids(&unpaired.reports), vec!["r9"]);
    assert_nothing_waits(&matcher);
}

#[test]
fn pair_executions_applies_the_caller_rule() {
    let rows = vec![
        Executions::ExecutionData(combo("b1")),
        Executions::ExecutionData(execution("e1")),
        Executions::CommissionAndFeesReport(report("e1", 1.25)),
    ];
    let mut matcher = FillMatcher::new().expects_report(no_report_for_combos);

    let (fills, unpaired) = matcher.pair_executions(rows);

    assert_eq!(fill_ids(&fills), vec![("b1", None), ("e1", Some(1.25))]);
    assert!(unpaired.executions.is_empty() && unpaired.reports.is_empty());
}

#[test]
fn stream_copies_after_pair_executions_are_recognized() {
    let rows = vec![
        Executions::ExecutionData(execution("e1")),
        Executions::CommissionAndFeesReport(report("e1", 1.25)),
        Executions::ExecutionData(execution("e2")),
    ];
    let mut matcher = FillMatcher::new();
    let (_fills, unpaired) = matcher.pair_executions(rows);
    let now = Instant::now();

    let execution_copy = matcher.push_execution_at(execution("e1"), now);
    let copy = matcher.push_report_at(report("e1", 1.25), now);
    let late = matcher.push_report_at(report("e2", 2.5), now);

    assert_eq!(ids(&unpaired.executions), vec!["e2"]);
    assert!(execution_copy.is_none(), "{execution_copy:?}");
    assert_duplicate(copy, "e1");
    assert_late(late, "e2", 2.5);
    assert_nothing_waits(&matcher);
}

#[test]
fn pair_executions_leaves_earlier_holdings_alone() {
    let start = Instant::now();
    let mut matcher = FillMatcher::new();
    assert!(matcher.push_execution_at(execution("old"), start).is_none());
    matcher.take_expired_at(Duration::ZERO, start);
    assert!(matcher.push_execution_at(execution("live"), start).is_none());
    let rows = vec![
        Executions::ExecutionData(execution("e1")),
        Executions::CommissionAndFeesReport(report("old", 0.5)),
    ];

    let (fills, unpaired) = matcher.pair_executions(rows);

    assert!(fills.is_empty(), "{fills:?}");
    assert_eq!(ids(&unpaired.executions), vec!["e1"], "the live execution is not part of the reply");
    assert_eq!(
        report_ids(&unpaired.reports),
        vec!["old"],
        "first report for an execution handed out before the call"
    );
    assert_eq!(matcher.executions.len(), 1);
    assert_eq!(matcher.executions[0].0.execution.execution_id, "live");
}

#[test]
fn pair_executions_returns_only_executions_not_handed_out_before() {
    let now = Instant::now();
    let mut matcher = FillMatcher::new();
    assert!(matcher.push_execution_at(execution("e1"), now).is_none());
    assert!(matches!(matcher.push_report_at(report("e1", 1.25), now), ReportMatch::Paired(_)));
    let rows = vec![
        Executions::ExecutionData(execution("e1")),
        Executions::CommissionAndFeesReport(report("e1", 1.25)),
        Executions::ExecutionData(execution("e2")),
        Executions::CommissionAndFeesReport(report("e2", 2.5)),
    ];

    let (fills, unpaired) = matcher.pair_executions(rows);

    assert_eq!(fill_ids(&fills), vec![("e2", Some(2.5))]);
    assert!(unpaired.executions.is_empty() && unpaired.reports.is_empty(), "{unpaired:?}");
    assert_nothing_waits(&matcher);
}

#[test]
fn pair_executions_keeps_a_live_report_when_the_reply_repeats_it() {
    let start = Instant::now();
    let mut matcher = FillMatcher::new();
    let live = report("live", 1.25);
    assert!(matches!(matcher.push_report_at(live.clone(), start), ReportMatch::Waiting));

    let (fills, unpaired) = matcher.pair_executions([Executions::CommissionAndFeesReport(live.clone())]);
    let paired = matcher.push_execution_at(execution("live"), start);
    let copy = matcher.push_report_at(live, start);

    assert!(fills.is_empty(), "{fills:?}");
    assert!(
        unpaired.executions.is_empty() && unpaired.reports.is_empty() && unpaired.fills.is_empty(),
        "{unpaired:?}"
    );
    assert_fill(&paired.expect("the live execution pairs with the original report"), "live", Some(1.25));
    assert_duplicate(copy, "live");
    assert_nothing_waits(&matcher);
}

#[test]
fn pair_executions_pairs_a_live_report_with_the_reply_execution_once() {
    let start = Instant::now();
    let mut matcher = FillMatcher::new();
    let live = report("live", 1.25);
    assert!(matches!(matcher.push_report_at(live.clone(), start), ReportMatch::Waiting));

    let (fills, unpaired) = matcher.pair_executions([
        Executions::CommissionAndFeesReport(report("live", 2.5)),
        Executions::ExecutionData(execution("live")),
    ]);
    let execution_copy = matcher.push_execution_at(execution("live"), start);
    let report_copy = matcher.push_report_at(live, start);

    assert_eq!(fill_ids(&fills), vec![("live", Some(1.25))]);
    assert!(
        unpaired.executions.is_empty() && unpaired.reports.is_empty() && unpaired.fills.is_empty(),
        "{unpaired:?}"
    );
    assert!(execution_copy.is_none(), "{execution_copy:?}");
    assert_duplicate(report_copy, "live");
    assert_nothing_waits(&matcher);
}

#[test]
fn pair_executions_keeps_a_live_execution_waiting_for_its_live_report() {
    let start = Instant::now();
    let mut matcher = FillMatcher::new();
    assert!(matcher.push_execution_at(execution("live"), start).is_none());

    let (fills, unpaired) = matcher.pair_executions(vec![Executions::ExecutionData(execution("live"))]);
    let held = matcher.executions.len();
    let live_report = matcher.push_report(report("live", 1.25));

    assert!(fills.is_empty(), "{fills:?}");
    assert!(unpaired.executions.is_empty(), "not handed out, nor twice: {:?}", unpaired.executions);
    assert_eq!(held, 1, "one copy stays held");
    match live_report {
        ReportMatch::Paired(fill) => assert_fill(&fill, "live", Some(1.25)),
        other => panic!("expected the live report to pair, got {other:?}"),
    }
    assert_nothing_waits(&matcher);
}

#[test]
fn pair_executions_pairs_a_live_execution_with_the_reply_report_once() {
    let start = Instant::now();
    let mut matcher = FillMatcher::new();
    assert!(matcher.push_execution_at(execution("live"), start).is_none());
    let rows = vec![
        Executions::ExecutionData(execution("live")),
        Executions::CommissionAndFeesReport(report("live", 1.25)),
    ];

    let (fills, unpaired) = matcher.pair_executions(rows);

    assert_eq!(fill_ids(&fills), vec![("live", Some(1.25))]);
    assert!(unpaired.executions.is_empty() && unpaired.reports.is_empty(), "{unpaired:?}");
    assert_nothing_waits(&matcher);
    assert_duplicate(matcher.push_report(report("live", 1.25)), "live");
}

#[test]
fn pair_executions_of_empty_reply() {
    let (fills, unpaired) = FillMatcher::new().pair_executions(Vec::new());

    assert!(fills.is_empty());
    assert!(unpaired.executions.is_empty());
    assert!(unpaired.reports.is_empty());
}

const PERM_ID: i64 = 1290491156;
const ESZ6: i32 = 515416632;
const ESH7: i32 = 649180684;

fn combo_legs(near_ratio: i32, far_ratio: i32) -> Vec<ComboLeg> {
    vec![
        ComboLeg {
            contract_id: ESZ6,
            ratio: near_ratio,
            action: LegAction::Buy,
            ..Default::default()
        },
        ComboLeg {
            contract_id: ESH7,
            ratio: far_ratio,
            action: LegAction::Sell,
            ..Default::default()
        },
    ]
}

fn combo_matcher(near_ratio: i32, far_ratio: i32) -> FillMatcher {
    let mut matcher = FillMatcher::new();
    matcher.add_combo(PERM_ID, &combo_legs(near_ratio, far_ratio));
    matcher
}

fn bag(execution_id: &str, shares: f64) -> ExecutionData {
    let mut data = combo(execution_id);
    data.contract.contract_id = 16472979;
    data.execution.perm_id = PERM_ID;
    data.execution.shares = shares;
    data.execution.price = -69.65;
    data
}

fn leg(execution_id: &str, contract_id: i32, shares: f64) -> ExecutionData {
    let mut data = execution(execution_id);
    data.contract.security_type = Some(SecurityType::Future);
    data.contract.contract_id = contract_id;
    data.execution.perm_id = PERM_ID;
    data.execution.shares = shares;
    data
}

/// (execution id, contract id, shares, commission) of each leg fill.
fn leg_fills(fill: &Fill) -> Vec<(&str, i32, f64, Option<f64>)> {
    fill.legs
        .iter()
        .map(|leg| {
            (
                leg.execution.execution.execution_id.as_str(),
                leg.execution.contract.contract_id,
                leg.execution.execution.shares,
                leg.commission.as_ref().map(|report| report.commission_and_fees),
            )
        })
        .collect()
}

fn assert_combo_fill(fill: &Fill, execution_id: &str, shares: f64) {
    assert_eq!(fill.execution.execution.execution_id, execution_id, "combo execution_id");
    assert_eq!(fill.execution.contract.security_type, Some(SecurityType::Spread));
    assert_eq!(fill.execution.execution.shares, shares, "combo shares");
    assert_eq!(fill.execution.execution.price, -69.65, "combo price");
    assert!(fill.commission.is_none(), "a combo-level execution has no report: {:?}", fill.commission);
}

fn assert_no_combo_parts(matcher: &FillMatcher) {
    assert_nothing_waits(matcher);
    assert!(matcher.combo_parts.is_empty(), "combo parts held: {:?}", matcher.combo_parts);
}

/// Feeds `events` at `now`; returns the fills handed out, and how many `ComboPending`.
fn feed(matcher: &mut FillMatcher, events: Vec<Executions>, now: Instant) -> (Vec<Fill>, usize) {
    let mut fills = Vec::new();
    let mut pending = 0;
    for event in events {
        match event {
            Executions::ExecutionData(execution) => fills.extend(matcher.push_execution_at(execution, now)),
            Executions::CommissionAndFeesReport(report) => match matcher.push_report_at(report, now) {
                ReportMatch::Paired(fill) => fills.push(fill),
                ReportMatch::ComboPending => pending += 1,
                ReportMatch::Waiting => {}
                other => panic!("unexpected {other:?}"),
            },
        }
    }
    (fills, pending)
}

/// The calendar fill as TWS sent it live: combo-level execution, far leg, near leg, then the
/// leg reports.
fn calendar_events() -> Vec<Executions> {
    vec![
        Executions::ExecutionData(bag("0000e1a7.6b0717ad.01.01", 1.0)),
        Executions::ExecutionData(leg("0000e1a7.6b0717ad.02.01", ESH7, 1.0)),
        Executions::ExecutionData(leg("0000e1a7.6b0717ad.03.01", ESZ6, 1.0)),
        Executions::CommissionAndFeesReport(report("0000e1a7.6b0717ad.02.01", 2.16)),
        Executions::CommissionAndFeesReport(report("0000e1a7.6b0717ad.03.01", 2.17)),
    ]
}

#[test]
fn combo_fill_in_the_order_tws_sends_it() {
    let now = Instant::now();
    let mut matcher = combo_matcher(1, 1);

    let (fills, pending) = feed(&mut matcher, calendar_events(), now);

    assert_eq!(fills.len(), 1, "{fills:?}");
    assert_combo_fill(&fills[0], "0000e1a7.6b0717ad.01.01", 1.0);
    assert_eq!(
        leg_fills(&fills[0]),
        vec![
            ("0000e1a7.6b0717ad.02.01", ESH7, 1.0, Some(2.16)),
            ("0000e1a7.6b0717ad.03.01", ESZ6, 1.0, Some(2.17)),
        ]
    );
    assert_eq!(pending, 1, "the first leg report pairs and waits for the second");
    assert_no_combo_parts(&matcher);
}

fn permutations(items: Vec<usize>) -> Vec<Vec<usize>> {
    if items.len() <= 1 {
        return vec![items];
    }
    let mut all = Vec::new();
    for index in 0..items.len() {
        let mut rest = items.clone();
        let first = rest.remove(index);
        for mut tail in permutations(rest) {
            tail.insert(0, first);
            all.push(tail);
        }
    }
    all
}

#[test]
fn combo_fill_in_every_arrival_order() {
    let now = Instant::now();
    for order in permutations((0..5).collect()) {
        let mut matcher = combo_matcher(1, 1);
        let mut events: Vec<Option<Executions>> = calendar_events().into_iter().map(Some).collect();
        let ordered: Vec<Executions> = order.iter().map(|&index| events[index].take().expect("each event once")).collect();

        let (fills, _pending) = feed(&mut matcher, ordered, now);

        assert_eq!(fills.len(), 1, "order {order:?}: {fills:?}");
        assert_combo_fill(&fills[0], "0000e1a7.6b0717ad.01.01", 1.0);
        let mut legs = leg_fills(&fills[0]);
        legs.sort_by_key(|leg| leg.1);
        assert_eq!(
            legs,
            vec![
                ("0000e1a7.6b0717ad.03.01", ESZ6, 1.0, Some(2.17)),
                ("0000e1a7.6b0717ad.02.01", ESH7, 1.0, Some(2.16)),
            ],
            "order {order:?}"
        );
        assert_no_combo_parts(&matcher);
    }
}

#[test]
fn ratio_leg_completes_at_quantity_times_ratio() {
    let now = Instant::now();
    let mut short = combo_matcher(1, 2);
    let mut full = combo_matcher(1, 2);
    let events = |far_shares: f64| {
        vec![
            Executions::ExecutionData(bag("0000e1a7.6b071816.01.01", 1.0)),
            Executions::ExecutionData(leg("0000e1a7.6b071816.02.01", ESH7, far_shares)),
            Executions::ExecutionData(leg("0000e1a7.6b071816.03.01", ESZ6, 1.0)),
            Executions::CommissionAndFeesReport(report("0000e1a7.6b071816.02.01", 4.32)),
            Executions::CommissionAndFeesReport(report("0000e1a7.6b071816.03.01", 2.16)),
        ]
    };

    let (short_fills, _) = feed(&mut short, events(1.0), now);
    let (full_fills, _) = feed(&mut full, events(2.0), now);

    assert!(short_fills.is_empty(), "1 of 2 far-leg shares is not complete: {short_fills:?}");
    assert_eq!(short.combo_parts.len(), 1);
    assert_eq!(full_fills.len(), 1, "{full_fills:?}");
    assert_eq!(
        leg_fills(&full_fills[0]),
        vec![
            ("0000e1a7.6b071816.02.01", ESH7, 2.0, Some(4.32)),
            ("0000e1a7.6b071816.03.01", ESZ6, 1.0, Some(2.16)),
        ]
    );
}

#[test]
fn each_partial_fill_is_its_own_combo_fill() {
    let now = Instant::now();
    let mut matcher = combo_matcher(1, 1);
    let mut events = Vec::new();
    for key in ["0000e1a7.6b0719db", "0000e1a7.6b0719de"] {
        events.push(Executions::ExecutionData(bag(&format!("{key}.01.01"), 2.0)));
        events.push(Executions::ExecutionData(leg(&format!("{key}.02.01"), ESH7, 2.0)));
        events.push(Executions::ExecutionData(leg(&format!("{key}.03.01"), ESZ6, 2.0)));
        events.push(Executions::CommissionAndFeesReport(report(&format!("{key}.02.01"), 4.32)));
        events.push(Executions::CommissionAndFeesReport(report(&format!("{key}.03.01"), 4.33)));
    }

    let (fills, pending) = feed(&mut matcher, events, now);

    let summary: Vec<(&str, Vec<&str>)> = fills
        .iter()
        .map(|fill| {
            (
                fill.execution.execution.execution_id.as_str(),
                fill.legs.iter().map(|leg| leg.execution.execution.execution_id.as_str()).collect(),
            )
        })
        .collect();
    assert_eq!(
        summary,
        vec![
            ("0000e1a7.6b0719db.01.01", vec!["0000e1a7.6b0719db.02.01", "0000e1a7.6b0719db.03.01"]),
            ("0000e1a7.6b0719de.01.01", vec!["0000e1a7.6b0719de.02.01", "0000e1a7.6b0719de.03.01"]),
        ]
    );
    assert_eq!(pending, 2);
    assert_no_combo_parts(&matcher);
}

#[test]
fn take_expired_hands_out_an_incomplete_combo_with_the_legs_that_came() {
    let start = Instant::now();
    let mut matcher = combo_matcher(1, 1);
    assert!(matcher.push_execution_at(bag("0000e1a7.6b0717ad.01.01", 1.0), start).is_none());
    assert!(matcher.push_execution_at(leg("0000e1a7.6b0717ad.02.01", ESH7, 1.0), start).is_none());
    assert!(matches!(
        matcher.push_report_at(report("0000e1a7.6b0717ad.02.01", 2.16), start),
        ReportMatch::ComboPending
    ));
    let later = start + Duration::from_secs(1);
    assert!(matcher.push_execution_at(leg("0000e1a7.6b0717ad.03.01", ESZ6, 1.0), later).is_none());

    let early = matcher.take_expired_at(MAX_WAIT, start + MAX_WAIT - Duration::from_millis(1));
    let due = matcher.take_expired_at(MAX_WAIT, start + MAX_WAIT);
    let late = matcher.push_report_at(report("0000e1a7.6b0717ad.03.01", 2.17), start + MAX_WAIT);

    assert!(early.fills.is_empty() && early.executions.is_empty(), "{early:?}");
    assert_eq!(due.fills.len(), 1, "{:?}", due.fills);
    assert_combo_fill(&due.fills[0], "0000e1a7.6b0717ad.01.01", 1.0);
    assert_eq!(
        leg_fills(&due.fills[0]),
        vec![
            ("0000e1a7.6b0717ad.02.01", ESH7, 1.0, Some(2.16)),
            ("0000e1a7.6b0717ad.03.01", ESZ6, 1.0, None),
        ]
    );
    assert!(due.executions.is_empty() && due.reports.is_empty(), "{due:?}");
    assert_late(late, "0000e1a7.6b0717ad.03.01", 2.17);
    assert_no_combo_parts(&matcher);
}

#[test]
fn combo_expiry_uses_a_younger_leg_correction_without_its_report() {
    let start = Instant::now();
    let mut matcher = combo_matcher(1, 1);
    let mut events = calendar_events();
    let near_report = events.pop().expect("near-leg report");
    let (fills, pending) = feed(&mut matcher, events, start);
    assert!(fills.is_empty(), "{fills:?}");
    assert_eq!(pending, 1);
    let corrected = "0000e1a7.6b0717ad.02.02";
    let shortly_before_expiry = start + MAX_WAIT - Duration::from_millis(100);
    assert!(matcher.push_execution_at(leg(corrected, ESH7, 0.75), shortly_before_expiry).is_none());
    let (fills, pending) = feed(&mut matcher, vec![near_report], shortly_before_expiry);
    assert!(fills.is_empty(), "{fills:?}");
    assert_eq!(pending, 1);

    let early = matcher.take_expired_at(MAX_WAIT, shortly_before_expiry);
    let due = matcher.take_expired_at(MAX_WAIT, start + MAX_WAIT);
    let late = matcher.push_report_at(report(corrected, 2.25), start + MAX_WAIT);
    let copy = matcher.push_report_at(report(corrected, 2.25), start + MAX_WAIT);
    let later = matcher.take_expired_at(MAX_WAIT, start + MAX_WAIT + MAX_WAIT);

    assert!(
        early.executions.is_empty() && early.reports.is_empty() && early.fills.is_empty(),
        "{early:?}"
    );
    assert_eq!(due.fills.len(), 1, "{due:?}");
    assert!(due.executions.is_empty() && due.reports.is_empty(), "{due:?}");
    assert_combo_fill(&due.fills[0], "0000e1a7.6b0717ad.01.01", 1.0);
    assert_eq!(
        leg_fills(&due.fills[0]),
        vec![("0000e1a7.6b0717ad.03.01", ESZ6, 1.0, Some(2.17)), (corrected, ESH7, 0.75, None),]
    );
    assert_late(late, corrected, 2.25);
    assert_duplicate(copy, corrected);
    assert!(
        later.executions.is_empty() && later.reports.is_empty() && later.fills.is_empty(),
        "{later:?}"
    );
    assert_no_combo_parts(&matcher);
}

#[test]
fn leg_without_report_completes_its_combo_when_taken() {
    let start = Instant::now();
    let mut matcher = combo_matcher(1, 1);
    let events = vec![
        Executions::ExecutionData(bag("0000e1a7.6b0717ad.01.01", 1.0)),
        Executions::ExecutionData(leg("0000e1a7.6b0717ad.02.01", ESH7, 1.0)),
        Executions::ExecutionData(leg("0000e1a7.6b0717ad.03.01", ESZ6, 1.0)),
        Executions::CommissionAndFeesReport(report("0000e1a7.6b0717ad.02.01", 2.16)),
    ];
    let (fills, _) = feed(&mut matcher, events, start);

    let due = matcher.take_expired_at(MAX_WAIT, start + MAX_WAIT);
    let late = matcher.push_report_at(report("0000e1a7.6b0717ad.03.01", 2.17), start + MAX_WAIT);

    assert!(fills.is_empty(), "{fills:?}");
    assert_eq!(due.fills.len(), 1, "{:?}", due.fills);
    assert_combo_fill(&due.fills[0], "0000e1a7.6b0717ad.01.01", 1.0);
    assert_eq!(
        leg_fills(&due.fills[0]),
        vec![
            ("0000e1a7.6b0717ad.02.01", ESH7, 1.0, Some(2.16)),
            ("0000e1a7.6b0717ad.03.01", ESZ6, 1.0, None),
        ]
    );
    assert!(due.executions.is_empty(), "a combo leg goes into its combo fill: {:?}", due.executions);
    assert_late(late, "0000e1a7.6b0717ad.03.01", 2.17);
    assert_no_combo_parts(&matcher);
}

#[test]
fn legs_without_their_combo_execution_come_out_one_by_one() {
    let start = Instant::now();
    let mut matcher = combo_matcher(1, 1);
    let events = vec![
        Executions::ExecutionData(leg("0000e1a7.6b0717ad.02.01", ESH7, 1.0)),
        Executions::ExecutionData(leg("0000e1a7.6b0717ad.03.01", ESZ6, 1.0)),
        Executions::CommissionAndFeesReport(report("0000e1a7.6b0717ad.02.01", 2.16)),
        Executions::CommissionAndFeesReport(report("0000e1a7.6b0717ad.03.01", 2.17)),
    ];
    let (fills, pending) = feed(&mut matcher, events, start);

    let due = matcher.take_expired_at(MAX_WAIT, start + MAX_WAIT);

    assert!(fills.is_empty(), "{fills:?}");
    assert_eq!(pending, 2);
    let legs: Vec<(&str, Option<f64>, usize)> = due
        .fills
        .iter()
        .map(|fill| {
            (
                fill.execution.execution.execution_id.as_str(),
                fill.commission.as_ref().map(|report| report.commission_and_fees),
                fill.legs.len(),
            )
        })
        .collect();
    assert_eq!(
        legs,
        vec![("0000e1a7.6b0717ad.02.01", Some(2.16), 0), ("0000e1a7.6b0717ad.03.01", Some(2.17), 0)]
    );
    assert_no_combo_parts(&matcher);
}

#[test]
fn combo_not_announced_comes_as_separate_fills() {
    let now = Instant::now();
    let mut matcher = FillMatcher::new();

    let (fills, pending) = feed(&mut matcher, calendar_events(), now);

    assert_eq!(
        fill_ids(&fills),
        vec![
            ("0000e1a7.6b0717ad.01.01", None),
            ("0000e1a7.6b0717ad.02.01", Some(2.16)),
            ("0000e1a7.6b0717ad.03.01", Some(2.17)),
        ],
        "the default rule expects no report for the combo-level execution"
    );
    assert!(fills.iter().all(|fill| fill.legs.is_empty()));
    assert_eq!(pending, 0);
    assert_no_combo_parts(&matcher);
}

#[test]
fn second_combo_execution_for_the_same_fill_passes_through() {
    let now = Instant::now();
    let mut matcher = combo_matcher(1, 1);
    assert!(matcher.push_execution_at(bag("0000e1a7.6b0717ad.01.01", 1.0), now).is_none());

    let again = matcher
        .push_execution_at(bag("0000e1a7.6b0717ad.04.01", 1.0), now)
        .expect("not held twice");

    assert_combo_fill(&again, "0000e1a7.6b0717ad.04.01", 1.0);
    assert!(again.legs.is_empty());
    assert_eq!(matcher.combo_parts.len(), 1);
}

#[test]
fn copies_of_a_combo_fill_after_it_completes_are_dropped() {
    let now = Instant::now();
    let mut matcher = combo_matcher(1, 1);
    let (fills, pending) = feed(&mut matcher, calendar_events(), now);
    assert_eq!(fills.len(), 1, "{fills:?}");

    let mut copies = Vec::new();
    let mut duplicates = 0;
    for event in calendar_events() {
        match event {
            Executions::ExecutionData(execution) => copies.extend(matcher.push_execution_at(execution, now)),
            Executions::CommissionAndFeesReport(report) => match matcher.push_report_at(report, now) {
                ReportMatch::Duplicate(_) => duplicates += 1,
                other => panic!("expected Duplicate, got {other:?}"),
            },
        }
    }
    let leftovers = matcher.take_expired_at(Duration::ZERO, now);

    assert_eq!(pending, 1, "the first leg report waits for the second leg");
    assert!(copies.is_empty(), "a replayed combo fill comes out once: {copies:?}");
    assert_eq!(duplicates, 2);
    assert!(
        leftovers.executions.is_empty() && leftovers.reports.is_empty() && leftovers.fills.is_empty(),
        "{leftovers:?}"
    );
    assert_no_combo_parts(&matcher);
}

#[test]
fn copy_of_a_leg_execution_does_not_count_toward_its_combo() {
    let now = Instant::now();
    let mut matcher = combo_matcher(1, 2);
    assert!(matcher.push_execution_at(bag("0000e1a7.6b0717ad.01.01", 1.0), now).is_none());
    assert!(matcher.push_execution_at(leg("0000e1a7.6b0717ad.03.01", ESH7, 1.0), now).is_none());
    assert!(matches!(
        matcher.push_report_at(report("0000e1a7.6b0717ad.03.01", 1.0), now),
        ReportMatch::ComboPending
    ));

    let copy = matcher.push_execution_at(leg("0000e1a7.6b0717ad.03.01", ESH7, 1.0), now);
    let report_copy = matcher.push_report_at(report("0000e1a7.6b0717ad.03.01", 1.0), now);

    assert!(copy.is_none(), "{copy:?}");
    assert_duplicate(report_copy, "0000e1a7.6b0717ad.03.01");
    assert_eq!(matcher.combo_parts.len(), 1, "the far leg has filled 1 of 2");
    assert_eq!(matcher.combo_parts[0].legs.len(), 1);
}

#[test]
fn a_retained_combo_leg_rejects_replays_after_handout_memory_eviction() {
    let now = Instant::now();
    let mut matcher = combo_matcher(1, 2).memory(1);
    let (fills, pending) = feed(&mut matcher, calendar_events(), now);
    assert!(fills.is_empty(), "{fills:?}");
    assert_eq!(pending, 2);
    for (id, commission) in [("e1", 1.25), ("e2", 2.5)] {
        assert!(matcher.push_execution_at(execution(id), now).is_none());
        let paired = matcher.push_report_at(report(id, commission), now);
        let ReportMatch::Paired(fill) = paired else {
            panic!("expected the standalone execution to pair, got {paired:?}");
        };
        assert_fill(&fill, id, Some(commission));
    }
    let first_far_leg = "0000e1a7.6b0717ad.02.01";
    let next_far_leg = "0000e1a7.6b0717ad.04.01";

    let replayed_leg = matcher.push_execution_at(leg(first_far_leg, ESH7, 1.0), now);
    let replayed_report = matcher.push_report_at(report(first_far_leg, 2.16), now);
    let next_leg = matcher.push_execution_at(leg(next_far_leg, ESH7, 1.0), now);
    let next_report = matcher.push_report_at(report(next_far_leg, 2.18), now);

    assert!(replayed_leg.is_none(), "{replayed_leg:?}");
    assert_duplicate(replayed_report, first_far_leg);
    assert!(next_leg.is_none(), "{next_leg:?}");
    let ReportMatch::Paired(fill) = next_report else {
        panic!("expected the distinct execution to complete the combo, got {next_report:?}");
    };
    assert_combo_fill(&fill, "0000e1a7.6b0717ad.01.01", 1.0);
    assert_eq!(
        leg_fills(&fill),
        vec![
            (first_far_leg, ESH7, 1.0, Some(2.16)),
            ("0000e1a7.6b0717ad.03.01", ESZ6, 1.0, Some(2.17)),
            (next_far_leg, ESH7, 1.0, Some(2.18)),
        ]
    );
    assert_no_combo_parts(&matcher);
}

#[test]
fn only_the_latest_leg_revision_counts_toward_combo_completion() {
    let now = Instant::now();
    let (original, corrected, distinct) = ("0000e1a7.6b0717ad.02.01", "0000e1a7.6b0717ad.02.02", "0000e1a7.6b0717ad.04.01");
    for revisions in [
        [(original, 0.75, 1.5), (corrected, 1.25, 2.5)],
        [(corrected, 1.25, 2.5), (original, 0.75, 1.5)],
    ] {
        let mut matcher = combo_matcher(1, 2);
        assert!(matcher.push_execution_at(bag("0000e1a7.6b0717ad.01.01", 1.0), now).is_none());
        for (id, shares, commission) in revisions {
            assert!(matcher.push_execution_at(leg(id, ESH7, shares), now).is_none());
            assert!(matches!(matcher.push_report_at(report(id, commission), now), ReportMatch::ComboPending));
        }
        let near = "0000e1a7.6b0717ad.03.01";
        assert!(matcher.push_execution_at(leg(near, ESZ6, 1.0), now).is_none());

        let near_report = matcher.push_report_at(report(near, 2.17), now);
        let final_leg = matcher.push_execution_at(leg(distinct, ESH7, 0.75), now);
        let final_report = matcher.push_report_at(report(distinct, 1.75), now);

        assert!(matches!(near_report, ReportMatch::ComboPending), "{near_report:?}");
        assert!(final_leg.is_none(), "{final_leg:?}");
        let ReportMatch::Paired(fill) = final_report else {
            panic!("expected the distinct leg to complete the combo, got {final_report:?}");
        };
        assert_combo_fill(&fill, "0000e1a7.6b0717ad.01.01", 1.0);
        assert_eq!(
            leg_fills(&fill),
            vec![
                (corrected, ESH7, 1.25, Some(2.5)),
                (near, ESZ6, 1.0, Some(2.17)),
                (distinct, ESH7, 0.75, Some(1.75)),
            ]
        );
        assert_eq!(
            fill.combo_legs().iter().map(|leg| (leg.contract_id, leg.ratio)).collect::<Vec<_>>(),
            vec![(ESZ6, 1), (ESH7, 2)]
        );
        assert_duplicate(matcher.push_report_at(report(original, 1.5), now), original);
        assert_no_combo_parts(&matcher);
    }
}

#[test]
fn a_known_leg_correction_waits_for_its_own_report() {
    let now = Instant::now();
    let mut matcher = combo_matcher(1, 1);
    let mut events = calendar_events();
    let near_report = events.pop().expect("near-leg report");
    let (fills, pending) = feed(&mut matcher, events, now);
    assert!(fills.is_empty(), "{fills:?}");
    assert_eq!(pending, 1);
    let corrected = "0000e1a7.6b0717ad.02.02";
    assert!(matcher.push_execution_at(leg(corrected, ESH7, 1.0), now).is_none());

    let (premature, pending) = feed(&mut matcher, vec![near_report], now);
    let result = matcher.push_report_at(report(corrected, 2.25), now);

    assert!(premature.is_empty(), "{premature:?}");
    assert_eq!(pending, 1);
    let ReportMatch::Paired(fill) = result else {
        panic!("expected the corrected leg to complete the combo, got {result:?}");
    };
    assert_combo_fill(&fill, "0000e1a7.6b0717ad.01.01", 1.0);
    assert_eq!(
        leg_fills(&fill),
        vec![("0000e1a7.6b0717ad.03.01", ESZ6, 1.0, Some(2.17)), (corrected, ESH7, 1.0, Some(2.25)),]
    );
    assert_no_combo_parts(&matcher);
}

#[test]
fn memory_bounds_the_combos_remembered() {
    let now = Instant::now();
    let mut matcher = FillMatcher::new().memory(1);
    matcher.add_combo(PERM_ID - 1, &combo_legs(1, 1));
    matcher.add_combo(PERM_ID, &combo_legs(1, 1));
    matcher.add_combo(PERM_ID, &combo_legs(1, 2));
    let mut forgotten = bag("0000e1a7.6b071700.01.01", 1.0);
    forgotten.execution.perm_id = PERM_ID - 1;
    let mut none = FillMatcher::new().memory(0);
    none.add_combo(PERM_ID, &combo_legs(1, 1));

    let passed = matcher.push_execution_at(forgotten, now);
    let held = matcher.push_execution_at(bag("0000e1a7.6b071816.01.01", 1.0), now);
    let unannounced = none.push_execution_at(bag("0000e1a7.6b0717ad.01.01", 1.0), now);

    assert!(passed.is_some_and(|fill| fill.legs.is_empty()), "the oldest combo was dropped");
    assert!(held.is_none(), "the newest combo is remembered");
    assert_eq!(
        matcher.combos.get(&PERM_ID).map(AsRef::as_ref),
        Some(&[(ESZ6, 1.0), (ESH7, 2.0)][..]),
        "legs replaced"
    );
    assert_eq!(matcher.combo_order.len(), 1);
    assert!(unannounced.is_some(), "memory(0) remembers no combo");
    assert!(none.combos.is_empty());
}

#[test]
fn a_pending_combo_keeps_its_ratios_after_announcement_eviction() {
    let now = Instant::now();
    let mut matcher = combo_matcher(1, 2).memory(1);
    let mut events = calendar_events();
    let bag_event = events.remove(0);
    let (fills, pending) = feed(&mut matcher, vec![bag_event], now);
    assert!(fills.is_empty(), "{fills:?}");
    assert_eq!(pending, 0);
    matcher.add_combo(PERM_ID + 1, &combo_legs(1, 1));

    let (partial, pending) = feed(&mut matcher, events, now);
    let distinct = "0000e1a7.6b0717ad.04.01";
    let (completed, final_pending) = feed(
        &mut matcher,
        vec![
            Executions::ExecutionData(leg(distinct, ESH7, 1.0)),
            Executions::CommissionAndFeesReport(report(distinct, 1.25)),
        ],
        now,
    );

    assert!(partial.is_empty(), "{partial:?}");
    assert_eq!(pending, 2);
    assert_eq!(final_pending, 0);
    assert_eq!(completed.len(), 1, "{completed:?}");
    assert_combo_fill(&completed[0], "0000e1a7.6b0717ad.01.01", 1.0);
    assert_eq!(
        leg_fills(&completed[0]),
        vec![
            ("0000e1a7.6b0717ad.02.01", ESH7, 1.0, Some(2.16)),
            ("0000e1a7.6b0717ad.03.01", ESZ6, 1.0, Some(2.17)),
            (distinct, ESH7, 1.0, Some(1.25)),
        ]
    );
    assert_no_combo_parts(&matcher);
}

#[test]
fn a_combo_leg_waiting_for_commission_keeps_its_ratios_after_announcement_eviction() {
    let now = Instant::now();
    let mut matcher = combo_matcher(1, 2).memory(1);
    let far = "0000e1a7.6b0717ad.02.01";
    let distinct = "0000e1a7.6b0717ad.04.01";
    let near = "0000e1a7.6b0717ad.03.01";
    let combo = "0000e1a7.6b0717ad.01.01";
    assert!(matcher.push_execution_at(leg(far, ESH7, 1.25), now).is_none());
    matcher.add_combo(PERM_ID + 1, &combo_legs(1, 1));

    let first_report = matcher.push_report_at(report(far, 3.25), now);
    let bag_result = matcher.push_execution_at(bag(combo, 1.0), now);
    let (partial, pending) = feed(
        &mut matcher,
        vec![
            Executions::ExecutionData(leg(near, ESZ6, 1.0)),
            Executions::CommissionAndFeesReport(report(near, 2.17)),
        ],
        now,
    );
    let final_leg = matcher.push_execution_at(leg(distinct, ESH7, 0.75), now);
    let final_report = matcher.push_report_at(report(distinct, 1.75), now);

    assert!(matches!(first_report, ReportMatch::ComboPending), "{first_report:?}");
    assert!(bag_result.is_none(), "{bag_result:?}");
    assert!(partial.is_empty(), "{partial:?}");
    assert_eq!(pending, 1);
    assert!(final_leg.is_none(), "{final_leg:?}");
    let ReportMatch::Paired(fill) = final_report else {
        panic!("expected the distinct execution to complete the combo, got {final_report:?}");
    };
    assert_combo_fill(&fill, combo, 1.0);
    assert_eq!(
        leg_fills(&fill),
        vec![
            (far, ESH7, 1.25, Some(3.25)),
            (near, ESZ6, 1.0, Some(2.17)),
            (distinct, ESH7, 0.75, Some(1.75)),
        ]
    );
    assert_no_combo_parts(&matcher);
}

#[test]
fn a_restored_combo_leg_keeps_its_ratios_after_announcement_eviction() {
    let start = Instant::now();
    let due = start + MAX_WAIT;
    let mut matcher = combo_matcher(1, 2).memory(1);
    let far = "0000e1a7.6b0717ad.02.01";
    let near = "0000e1a7.6b0717ad.03.01";
    let combo = "0000e1a7.6b0717ad.01.01";
    assert!(matcher.push_execution_at(leg(far, ESH7, 2.0), start).is_none());
    let expired = matcher.take_expired_at(MAX_WAIT, due);
    assert!(expired.executions.is_empty() && expired.reports.is_empty(), "{expired:?}");
    assert_eq!(fill_ids(&expired.fills), vec![(far, None)]);
    assert_no_combo_parts(&matcher);

    let expired_leg = expired.fills.into_iter().next().expect("the uncommissioned leg is due");
    let restored = matcher.put_back(expired_leg.execution);
    let restored_arrived = matcher.executions.first().map(|(_, arrived)| *arrived);
    matcher.add_combo(PERM_ID + 1, &combo_legs(1, 1));
    let far_report = matcher.push_report_at(report(far, 3.25), due);
    let bag_result = matcher.push_execution_at(bag(combo, 1.0), due);
    let near_result = matcher.push_execution_at(leg(near, ESZ6, 1.0), due);
    let near_report = matcher.push_report_at(report(near, 2.17), due);

    assert!(restored.is_none(), "{restored:?}");
    assert_eq!(restored_arrived, Some(start));
    assert!(matches!(far_report, ReportMatch::ComboPending), "{far_report:?}");
    assert!(bag_result.is_none(), "{bag_result:?}");
    assert!(near_result.is_none(), "{near_result:?}");
    let ReportMatch::Paired(fill) = near_report else {
        panic!("expected the restored combo to complete, got {near_report:?}");
    };
    assert_combo_fill(&fill, combo, 1.0);
    assert_eq!(leg_fills(&fill), vec![(far, ESH7, 2.0, Some(3.25)), (near, ESZ6, 1.0, Some(2.17))]);
    assert_no_combo_parts(&matcher);
}

#[test]
fn combo_expiry_takes_waiting_legs_after_announcement_eviction() {
    let now = Instant::now();
    let mut matcher = combo_matcher(1, 2).memory(2);
    let far = "0000e1a7.6b0717ad.02.01";
    let near = "0000e1a7.6b0717ad.03.01";
    assert!(matcher.push_execution_at(leg(far, ESH7, 2.0), now).is_none());
    matcher.add_combo(PERM_ID + 1, &combo_legs(1, 1));
    matcher.add_combo(PERM_ID + 2, &combo_legs(1, 1));
    let near_arrived = now + MAX_WAIT - Duration::from_millis(100);
    assert!(matcher.push_execution_at(leg(near, ESZ6, 1.0), near_arrived).is_none());

    let early = matcher.take_expired_at(MAX_WAIT, near_arrived);
    let due = matcher.take_expired_at(MAX_WAIT, now + MAX_WAIT);
    let far_report = matcher.push_report_at(report(far, 3.25), now + MAX_WAIT);
    let near_report = matcher.push_report_at(report(near, 2.17), now + MAX_WAIT);
    let later = matcher.take_expired_at(MAX_WAIT, near_arrived + MAX_WAIT);

    assert!(
        early.executions.is_empty() && early.reports.is_empty() && early.fills.is_empty(),
        "{early:?}"
    );
    assert!(due.executions.is_empty() && due.reports.is_empty(), "{due:?}");
    assert_eq!(fill_ids(&due.fills), vec![(far, None), (near, None)]);
    assert_eq!(
        due.fills
            .iter()
            .map(|fill| (fill.execution.contract.contract_id, fill.execution.execution.shares))
            .collect::<Vec<_>>(),
        vec![(ESH7, 2.0), (ESZ6, 1.0)]
    );
    assert_late(far_report, far, 3.25);
    assert_late(near_report, near, 2.17);
    assert!(
        later.executions.is_empty() && later.reports.is_empty() && later.fills.is_empty(),
        "{later:?}"
    );
    assert_no_combo_parts(&matcher);
}

#[test]
fn a_commission_without_its_execution_does_not_keep_a_combo_announcement() {
    let now = Instant::now();
    let mut matcher = combo_matcher(1, 1).memory(1);
    let far = "0000e1a7.6b0717ad.02.01";
    let combo = "0000e1a7.6b0717ad.01.01";
    let waiting = matcher.push_report_at(report(far, 3.25), now);
    matcher.add_combo(PERM_ID + 1, &combo_legs(1, 1));

    let paired = matcher.push_execution_at(leg(far, ESH7, 3.0), now);
    let bag_result = matcher.push_execution_at(bag(combo, 1.0), now);

    assert!(matches!(waiting, ReportMatch::Waiting), "{waiting:?}");
    let paired = paired.expect("the leg pairs as a standalone fill");
    assert_fill(&paired, far, Some(3.25));
    assert!(paired.legs.is_empty(), "{paired:?}");
    let bag_fill = bag_result.expect("the combo-level execution arrives as a standalone fill");
    assert_combo_fill(&bag_fill, combo, 1.0);
    assert!(bag_fill.legs.is_empty(), "{bag_fill:?}");
    assert_no_combo_parts(&matcher);
}

#[test]
fn pair_executions_assembles_combo_fills_of_a_reply() {
    let mut other = leg("0000e1a7.6b06c16e.01.01", ESH7, 1.0);
    other.execution.perm_id = 1094417381;
    let rows = vec![
        Executions::ExecutionData(other),
        Executions::ExecutionData(bag("0000e1a7.6b0717ad.01.01", 1.0)),
        Executions::ExecutionData(leg("0000e1a7.6b0717ad.02.01", ESH7, 1.0)),
        Executions::ExecutionData(leg("0000e1a7.6b0717ad.03.01", ESZ6, 1.0)),
        Executions::ExecutionData(bag("0000e1a7.6b0717ee.01.01", 1.0)),
        Executions::ExecutionData(leg("0000e1a7.6b0717ee.02.01", ESH7, 1.0)),
        Executions::ExecutionData(leg("0000e1a7.6b0717ee.03.01", ESZ6, 1.0)),
        Executions::CommissionAndFeesReport(report("0000e1a7.6b06c16e.01.01", 2.24)),
        Executions::CommissionAndFeesReport(report("0000e1a7.6b0717ad.02.01", 2.16)),
        Executions::CommissionAndFeesReport(report("0000e1a7.6b0717ad.03.01", 2.17)),
        Executions::CommissionAndFeesReport(report("0000e1a7.6b0717ee.02.01", 2.18)),
    ];
    let mut matcher = combo_matcher(1, 1);

    let (fills, unpaired) = matcher.pair_executions(rows);

    assert_eq!(
        fill_ids(&fills),
        vec![("0000e1a7.6b06c16e.01.01", Some(2.24)), ("0000e1a7.6b0717ad.01.01", None)],
        "execution order; the combo fill stands at its combo-level execution"
    );
    assert_eq!(
        leg_fills(&fills[1]),
        vec![
            ("0000e1a7.6b0717ad.02.01", ESH7, 1.0, Some(2.16)),
            ("0000e1a7.6b0717ad.03.01", ESZ6, 1.0, Some(2.17)),
        ]
    );
    assert_eq!(unpaired.fills.len(), 1, "{:?}", unpaired.fills);
    assert_combo_fill(&unpaired.fills[0], "0000e1a7.6b0717ee.01.01", 1.0);
    assert_eq!(
        leg_fills(&unpaired.fills[0]),
        vec![
            ("0000e1a7.6b0717ee.02.01", ESH7, 1.0, Some(2.18)),
            ("0000e1a7.6b0717ee.03.01", ESZ6, 1.0, None),
        ],
        "the near leg's report is missing from the reply"
    );
    assert!(unpaired.executions.is_empty() && unpaired.reports.is_empty(), "{unpaired:?}");
    assert_no_combo_parts(&matcher);
}

/// An `executions` reply as TWS sends it (fills in time order, each combo-level execution
/// then its legs, then the reports), modelled on the 2026-11-23 replay: a 1x2 calendar filled
/// 2 + 2, a plain fill of another order, and a 1x1 calendar whose near leg's report is missing.
fn combo_reply_rows() -> Vec<Executions> {
    let mut plain = leg("0000e1a7.6b06c16e.01.01", ESH7, 1.0);
    plain.execution.perm_id = 1094417381;
    let mut other_bag = bag("0000e1a7.6b0717ee.01.01", 1.0);
    let mut other_far = leg("0000e1a7.6b0717ee.02.01", ESH7, 1.0);
    let mut other_near = leg("0000e1a7.6b0717ee.03.01", ESZ6, 1.0);
    for data in [&mut other_bag, &mut other_far, &mut other_near] {
        data.execution.perm_id = PERM_ID + 1;
    }
    vec![
        Executions::ExecutionData(bag("0000e1a7.6b0719db.01.01", 2.0)),
        Executions::ExecutionData(leg("0000e1a7.6b0719db.02.01", ESH7, 4.0)),
        Executions::ExecutionData(leg("0000e1a7.6b0719db.03.01", ESZ6, 2.0)),
        Executions::ExecutionData(plain),
        Executions::ExecutionData(bag("0000e1a7.6b0719de.01.01", 2.0)),
        Executions::ExecutionData(leg("0000e1a7.6b0719de.02.01", ESH7, 4.0)),
        Executions::ExecutionData(leg("0000e1a7.6b0719de.03.01", ESZ6, 2.0)),
        Executions::ExecutionData(other_bag),
        Executions::ExecutionData(other_far),
        Executions::ExecutionData(other_near),
        Executions::CommissionAndFeesReport(report("0000e1a7.6b0719db.02.01", 8.64)),
        Executions::CommissionAndFeesReport(report("0000e1a7.6b0719db.03.01", 4.32)),
        Executions::CommissionAndFeesReport(report("0000e1a7.6b06c16e.01.01", 2.24)),
        Executions::CommissionAndFeesReport(report("0000e1a7.6b0719de.02.01", 8.65)),
        Executions::CommissionAndFeesReport(report("0000e1a7.6b0719de.03.01", 4.33)),
        Executions::CommissionAndFeesReport(report("0000e1a7.6b0717ee.02.01", 2.18)),
    ]
}

#[test]
fn pair_executions_assembles_combo_fills_without_add_combo() {
    let mut matcher = FillMatcher::new();

    let (fills, unpaired) = matcher.pair_executions(combo_reply_rows());

    assert_eq!(
        fill_ids(&fills),
        vec![
            ("0000e1a7.6b0719db.01.01", None),
            ("0000e1a7.6b06c16e.01.01", Some(2.24)),
            ("0000e1a7.6b0719de.01.01", None),
        ],
        "execution order; each combo fill stands at its combo-level execution"
    );
    assert_combo_fill(&fills[0], "0000e1a7.6b0719db.01.01", 2.0);
    assert_eq!(
        leg_fills(&fills[0]),
        vec![
            ("0000e1a7.6b0719db.02.01", ESH7, 4.0, Some(8.64)),
            ("0000e1a7.6b0719db.03.01", ESZ6, 2.0, Some(4.32)),
        ]
    );
    assert!(fills[1].legs.is_empty(), "a plain fill has no legs: {:?}", fills[1].legs);
    assert_combo_fill(&fills[2], "0000e1a7.6b0719de.01.01", 2.0);
    assert_eq!(
        leg_fills(&fills[2]),
        vec![
            ("0000e1a7.6b0719de.02.01", ESH7, 4.0, Some(8.65)),
            ("0000e1a7.6b0719de.03.01", ESZ6, 2.0, Some(4.33)),
        ],
        "the second fill of the same order joins by its own fill key"
    );
    assert_eq!(unpaired.fills.len(), 1, "{:?}", unpaired.fills);
    assert_combo_fill(&unpaired.fills[0], "0000e1a7.6b0717ee.01.01", 1.0);
    assert_eq!(
        leg_fills(&unpaired.fills[0]),
        vec![
            ("0000e1a7.6b0717ee.02.01", ESH7, 1.0, Some(2.18)),
            ("0000e1a7.6b0717ee.03.01", ESZ6, 1.0, None),
        ],
        "the near leg's report is missing from the reply"
    );
    assert!(unpaired.executions.is_empty() && unpaired.reports.is_empty(), "{unpaired:?}");
    assert_no_combo_parts(&matcher);
    assert_late(
        matcher.push_report(report("0000e1a7.6b0717ee.03.01", 2.19)),
        "0000e1a7.6b0717ee.03.01",
        2.19,
    );
}

#[test]
fn pair_executions_leaves_legs_of_another_order_or_fill_alone() {
    let mut other_order = leg("0000e1a7.6b0717ad.02.01", ESH7, 1.0);
    other_order.execution.perm_id = PERM_ID + 1;
    let rows = vec![
        Executions::ExecutionData(bag("0000e1a7.6b0717ad.01.01", 1.0)),
        Executions::ExecutionData(other_order),
        Executions::ExecutionData(leg("0000e1a7.6b0717ae.03.01", ESZ6, 1.0)),
        Executions::CommissionAndFeesReport(report("0000e1a7.6b0717ad.02.01", 2.16)),
        Executions::CommissionAndFeesReport(report("0000e1a7.6b0717ae.03.01", 2.17)),
    ];

    let (fills, unpaired) = FillMatcher::new().pair_executions(rows);

    assert_eq!(
        fill_ids(&fills),
        vec![
            ("0000e1a7.6b0717ad.01.01", None),
            ("0000e1a7.6b0717ad.02.01", Some(2.16)),
            ("0000e1a7.6b0717ae.03.01", Some(2.17)),
        ],
        "a different perm_id or fill key is not a leg"
    );
    assert!(fills.iter().all(|fill| fill.legs.is_empty()), "{fills:?}");
    assert!(unpaired.executions.is_empty() && unpaired.reports.is_empty() && unpaired.fills.is_empty());
}

/// The live executions of a combo fill whose leg reports have not come yet, then an
/// `executions` reply listing the fill whole (the race `Unpaired::executions` describes).
fn live_then_reply(matcher: &mut FillMatcher, now: Instant) -> (Option<Fill>, Vec<Fill>, Unpaired) {
    let live_combo = matcher.push_execution_at(bag("0000e1a7.6b0717ad.01.01", 1.0), now);
    assert!(matcher.push_execution_at(leg("0000e1a7.6b0717ad.02.01", ESH7, 1.0), now).is_none());
    assert!(matcher.push_execution_at(leg("0000e1a7.6b0717ad.03.01", ESZ6, 1.0), now).is_none());
    let (fills, unpaired) = matcher.pair_executions(calendar_events());
    (live_combo, fills, unpaired)
}

#[test]
fn reply_completes_an_announced_combo_held_from_the_live_stream() {
    let now = Instant::now();
    let mut matcher = combo_matcher(1, 1);

    let (live_combo, fills, unpaired) = live_then_reply(&mut matcher, now);

    assert!(live_combo.is_none(), "the announced combo waits for its legs: {live_combo:?}");
    assert_eq!(fills.len(), 1, "{fills:?}");
    assert_combo_fill(&fills[0], "0000e1a7.6b0717ad.01.01", 1.0);
    assert_eq!(
        leg_fills(&fills[0]),
        vec![
            ("0000e1a7.6b0717ad.02.01", ESH7, 1.0, Some(2.16)),
            ("0000e1a7.6b0717ad.03.01", ESZ6, 1.0, Some(2.17)),
        ]
    );
    assert!(
        unpaired.executions.is_empty() && unpaired.reports.is_empty() && unpaired.fills.is_empty(),
        "{unpaired:?}"
    );
    assert_nothing_waits(&matcher);
    assert_no_combo_parts(&matcher);
}

#[test]
fn reply_after_an_unannounced_live_combo_returns_only_the_legs_not_handed_out() {
    let now = Instant::now();
    let mut matcher = FillMatcher::new();

    let (live_combo, fills, unpaired) = live_then_reply(&mut matcher, now);

    let live_combo = live_combo.expect("an unannounced combo-level execution is a fill at once");
    assert_combo_fill(&live_combo, "0000e1a7.6b0717ad.01.01", 1.0);
    assert!(live_combo.legs.is_empty());
    assert_eq!(
        fill_ids(&fills),
        vec![("0000e1a7.6b0717ad.02.01", Some(2.16)), ("0000e1a7.6b0717ad.03.01", Some(2.17))],
        "the combo-level execution came out live; the reply hands out each held leg once"
    );
    assert!(fills.iter().all(|fill| fill.legs.is_empty()));
    assert!(
        unpaired.executions.is_empty() && unpaired.reports.is_empty() && unpaired.fills.is_empty(),
        "{unpaired:?}"
    );
    assert_nothing_waits(&matcher);
}

#[test]
fn pair_executions_leaves_combo_parts_from_before_alone() {
    let now = Instant::now();
    let mut matcher = combo_matcher(1, 1);
    assert!(matcher.push_execution_at(bag("0000e1a7.6b0717ad.01.01", 1.0), now).is_none());

    let (fills, unpaired) = matcher.pair_executions(vec![Executions::ExecutionData(execution("e1"))]);

    assert!(fills.is_empty());
    assert_eq!(ids(&unpaired.executions), vec!["e1"]);
    assert!(unpaired.fills.is_empty(), "{:?}", unpaired.fills);
    assert_eq!(matcher.combo_parts.len(), 1);
}

#[test]
fn pair_executions_keeps_a_live_combo_when_the_reply_repeats_its_parts() {
    let now = Instant::now();
    let mut matcher = combo_matcher(1, 1);
    let mut live = calendar_events();
    let final_report = live.pop().expect("near-leg report");
    let (live_fills, pending) = feed(&mut matcher, live, now);
    assert!(live_fills.is_empty(), "{live_fills:?}");
    assert_eq!(pending, 1);

    let (fills, unpaired) = matcher.pair_executions(calendar_events().into_iter().take(3));
    let (completed, pending) = feed(&mut matcher, vec![final_report], now);

    assert!(fills.is_empty(), "{fills:?}");
    assert!(
        unpaired.executions.is_empty() && unpaired.reports.is_empty() && unpaired.fills.is_empty(),
        "{unpaired:?}"
    );
    assert_eq!(completed.len(), 1, "{completed:?}");
    assert_eq!(pending, 0);
    assert_combo_fill(&completed[0], "0000e1a7.6b0717ad.01.01", 1.0);
    assert_eq!(
        leg_fills(&completed[0]),
        vec![
            ("0000e1a7.6b0717ad.02.01", ESH7, 1.0, Some(2.16)),
            ("0000e1a7.6b0717ad.03.01", ESZ6, 1.0, Some(2.17)),
        ]
    );
    assert_no_combo_parts(&matcher);
}

#[test]
fn pair_executions_leaves_a_new_uncommissioned_leg_with_its_live_combo() {
    let now = Instant::now();
    let mut matcher = combo_matcher(1, 1);
    let mut events = calendar_events();
    let reply_leg = events.remove(2);
    let final_report = events.pop().expect("near-leg report");
    let (live_fills, pending) = feed(&mut matcher, events, now);
    assert!(live_fills.is_empty(), "{live_fills:?}");
    assert_eq!(pending, 1);

    let (fills, unpaired) = matcher.pair_executions([reply_leg]);
    let (completed, pending) = feed(&mut matcher, vec![final_report], now);

    assert!(fills.is_empty(), "{fills:?}");
    assert!(
        unpaired.executions.is_empty() && unpaired.reports.is_empty() && unpaired.fills.is_empty(),
        "{unpaired:?}"
    );
    assert_eq!(completed.len(), 1, "{completed:?}");
    assert_eq!(pending, 0);
    assert_combo_fill(&completed[0], "0000e1a7.6b0717ad.01.01", 1.0);
    assert_eq!(
        leg_fills(&completed[0]),
        vec![
            ("0000e1a7.6b0717ad.02.01", ESH7, 1.0, Some(2.16)),
            ("0000e1a7.6b0717ad.03.01", ESZ6, 1.0, Some(2.17)),
        ]
    );
    assert_no_combo_parts(&matcher);
}

#[test]
fn pair_executions_uses_a_later_leg_revision_in_an_announced_combo() {
    let corrected = "0000e1a7.6b0717ad.02.02";
    for original_has_report in [true, false] {
        let mut rows = calendar_events();
        if !original_has_report {
            rows.remove(3);
        }
        rows.push(Executions::ExecutionData(leg(corrected, ESH7, 1.0)));
        rows.push(Executions::CommissionAndFeesReport(report(corrected, 2.25)));
        let mut matcher = combo_matcher(1, 1);

        let (fills, unpaired) = matcher.pair_executions(rows);

        assert_eq!(fills.len(), 1, "{fills:?}");
        assert_combo_fill(&fills[0], "0000e1a7.6b0717ad.01.01", 1.0);
        assert_eq!(
            leg_fills(&fills[0]),
            vec![("0000e1a7.6b0717ad.03.01", ESZ6, 1.0, Some(2.17)), (corrected, ESH7, 1.0, Some(2.25)),]
        );
        assert!(
            unpaired.executions.is_empty() && unpaired.reports.is_empty() && unpaired.fills.is_empty(),
            "{unpaired:?}"
        );
        let original = "0000e1a7.6b0717ad.02.01";
        let returned = matcher
            .put_back(leg(original, ESH7, 1.0))
            .expect("superseded execution was never handed out");
        assert_eq!(returned.execution.execution_id, original);
        assert_duplicate(matcher.push_report(report(original, 2.16)), original);
        assert_no_combo_parts(&matcher);
    }
}

#[test]
fn pair_executions_suppresses_an_older_revision_in_an_unannounced_combo() {
    let now = Instant::now();
    let original = "0000e1a7.6b0717ad.02.01";
    let corrected = "0000e1a7.6b0717ad.02.02";
    for original_was_live in [true, false] {
        let mut matcher = FillMatcher::new();
        if original_was_live {
            assert!(matcher.push_execution_at(leg(original, ESH7, 1.0), now).is_none());
        }
        let mut rows = calendar_events();
        rows.retain(|row| !matches!(row, Executions::CommissionAndFeesReport(report) if report.execution_id == original));
        rows.push(Executions::ExecutionData(leg(corrected, ESH7, 1.0)));
        rows.push(Executions::CommissionAndFeesReport(report(corrected, 2.25)));

        let (fills, unpaired) = matcher.pair_executions(rows);
        let returned = matcher
            .put_back(leg(original, ESH7, 1.0))
            .expect("superseded execution was never handed out");
        let old_report = matcher.push_report_at(report(original, 2.16), now);

        assert_eq!(fills.len(), 1, "{fills:?}");
        assert_combo_fill(&fills[0], "0000e1a7.6b0717ad.01.01", 1.0);
        assert_eq!(
            leg_fills(&fills[0]),
            vec![("0000e1a7.6b0717ad.03.01", ESZ6, 1.0, Some(2.17)), (corrected, ESH7, 1.0, Some(2.25)),]
        );
        assert_eq!(
            fills[0].combo_legs().iter().map(|leg| (leg.contract_id, leg.ratio)).collect::<Vec<_>>(),
            vec![(ESZ6, 1), (ESH7, 1)]
        );
        assert!(
            unpaired.executions.is_empty() && unpaired.reports.is_empty() && unpaired.fills.is_empty(),
            "{unpaired:?}"
        );
        assert_eq!(returned.execution.execution_id, original);
        assert_duplicate(old_report, original);
        assert_no_combo_parts(&matcher);
    }
}

#[test]
fn pair_executions_uses_a_waiting_live_correction_in_an_unannounced_combo() {
    let now = Instant::now();
    let mut matcher = FillMatcher::new();
    let corrected = "0000e1a7.6b0717ad.02.02";
    let unrelated = "0000e1a7.6b0717ad.04.01";
    assert!(matcher.push_execution_at(leg(corrected, ESH7, 0.75), now).is_none());
    assert!(matcher.push_execution_at(leg(unrelated, ESH7, 3.0), now).is_none());
    let mut rows = calendar_events();
    rows.push(Executions::ExecutionData(leg(corrected, ESH7, 0.75)));

    let (fills, unpaired) = matcher.pair_executions(rows);
    let late = matcher.push_report_at(report(corrected, 2.25), now);
    let unrelated_report = matcher.push_report_at(report(unrelated, 1.25), now);

    assert!(fills.is_empty(), "{fills:?}");
    assert_eq!(unpaired.fills.len(), 1, "{unpaired:?}");
    assert!(unpaired.executions.is_empty() && unpaired.reports.is_empty(), "{unpaired:?}");
    assert_combo_fill(&unpaired.fills[0], "0000e1a7.6b0717ad.01.01", 1.0);
    assert_eq!(
        leg_fills(&unpaired.fills[0]),
        vec![("0000e1a7.6b0717ad.03.01", ESZ6, 1.0, Some(2.17)), (corrected, ESH7, 0.75, None),]
    );
    assert_late(late, corrected, 2.25);
    let ReportMatch::Paired(fill) = unrelated_report else {
        panic!("expected the unrelated live leg to pair, got {unrelated_report:?}");
    };
    assert_fill(&fill, unrelated, Some(1.25));
    assert_no_combo_parts(&matcher);
}

#[test]
fn pair_executions_uses_an_unannounced_leg_revision_when_both_revisions_are_waiting() {
    let now = Instant::now();
    let original = "0000e1a7.6b0717ad.02.01";
    let corrected = "0000e1a7.6b0717ad.02.02";
    let unrelated = "0000e1a7.6b0717ad.04.01";
    for include_correction in [true, false] {
        let mut matcher = FillMatcher::new();
        assert!(matcher.push_execution_at(leg(original, ESH7, 1.0), now).is_none());
        assert!(matcher.push_execution_at(leg(corrected, ESH7, 0.75), now).is_none());
        assert!(matcher.push_execution_at(leg(unrelated, ESH7, 3.0), now).is_none());
        let mut rows = calendar_events();
        rows.remove(3);
        if include_correction {
            rows.push(Executions::ExecutionData(leg(corrected, ESH7, 0.75)));
        }

        let (fills, unpaired) = matcher.pair_executions(rows);
        let old_report = matcher.push_report_at(report(original, 2.16), now);
        let current_report = matcher.push_report_at(report(corrected, 2.25), now);
        let expiry = matcher.take_expired_at(MAX_WAIT, now);
        let unrelated_report = matcher.push_report_at(report(unrelated, 1.25), now);

        assert!(fills.is_empty(), "{fills:?}");
        assert_eq!(unpaired.fills.len(), 1, "{unpaired:?}");
        assert!(unpaired.executions.is_empty() && unpaired.reports.is_empty(), "{unpaired:?}");
        assert_combo_fill(&unpaired.fills[0], "0000e1a7.6b0717ad.01.01", 1.0);
        assert_eq!(
            leg_fills(&unpaired.fills[0]),
            vec![("0000e1a7.6b0717ad.03.01", ESZ6, 1.0, Some(2.17)), (corrected, ESH7, 0.75, None),]
        );
        assert_duplicate(old_report, original);
        assert_late(current_report, corrected, 2.25);
        assert!(
            expiry.executions.is_empty() && expiry.reports.is_empty() && expiry.fills.is_empty(),
            "{expiry:?}"
        );
        let ReportMatch::Paired(fill) = unrelated_report else {
            panic!("expected the unrelated live leg to pair, got {unrelated_report:?}");
        };
        assert_fill(&fill, unrelated, Some(1.25));
        assert_no_combo_parts(&matcher);
    }
}

#[test]
fn fill_key_drops_the_last_two_fields() {
    let cases = [
        ("0000e1a7.6b0719db.02.01", "0000e1a7.6b0719db"),
        ("0000e1a7.6b0719db.01.01", "0000e1a7.6b0719db"),
        ("0000e1a7.0001.01", "0000e1a7"),
        ("a.b", "a.b"),
        ("e1", "e1"),
        ("", ""),
    ];
    for (execution_id, expected) in cases {
        assert_eq!(fill_key(execution_id), expected, "{execution_id}");
    }
}

/// One partial fill of a 1x2 ES calendar from an `executions` reply, live shape: 2 combos, the
/// ratio-2 far leg with 4 contracts, legs on `side` and its opposite as TWS executes them.
fn calendar_1x2_rows(side: ExecutionSide) -> Vec<Executions> {
    let opposite = match side {
        ExecutionSide::Bought => ExecutionSide::Sold,
        ExecutionSide::Sold => ExecutionSide::Bought,
    };
    let mut combo = bag("0000e1a7.6b071816.01.01", 2.0);
    combo.execution.side = side;
    let mut far = leg("0000e1a7.6b071816.02.01", ESH7, 4.0);
    far.execution.side = opposite;
    far.contract.exchange = "CME".into();
    let mut near = leg("0000e1a7.6b071816.03.01", ESZ6, 2.0);
    near.execution.side = side;
    near.contract.exchange = "CME".into();
    vec![
        Executions::ExecutionData(combo),
        Executions::ExecutionData(far),
        Executions::ExecutionData(near),
        Executions::CommissionAndFeesReport(report("0000e1a7.6b071816.02.01", 4.32)),
        Executions::CommissionAndFeesReport(report("0000e1a7.6b071816.03.01", 2.16)),
    ]
}

// Live (server 223): the 14 ES combo fills of a 7-day `executions` reply gave the legs TWS
// echoed on each order, bought or sold, partial or whole: the echo's legs are the order's
// legs, sorted by contract id, whichever side the combo executed on.
#[test]
fn combo_legs_of_a_partial_fill_match_the_order_bought_or_sold() {
    let expected: Vec<ComboLeg> = combo_legs(1, 2)
        .into_iter()
        .map(|leg| ComboLeg {
            exchange: "CME".to_string(),
            ..leg
        })
        .collect();

    for side in [ExecutionSide::Bought, ExecutionSide::Sold] {
        let (fills, _) = FillMatcher::new().pair_executions(calendar_1x2_rows(side));

        assert_eq!(fills.len(), 1, "{side:?}: {fills:?}");
        assert_eq!(fills[0].combo_legs(), expected, "{side:?}");
    }
}

fn leg_fill(execution_id: &str, contract_id: i32, side: ExecutionSide, shares: f64, exchange: &str) -> Fill {
    let mut data = leg(execution_id, contract_id, shares);
    data.execution.side = side;
    data.contract.exchange = exchange.into();
    Fill {
        execution: data,
        commission: None,
        legs: Vec::new(),
    }
}

#[test]
fn combo_legs_sums_a_leg_filled_by_several_executions() {
    let mut combo = bag("0000e1a7.6b071816.01.01", 2.0);
    combo.execution.side = ExecutionSide::Bought;
    let fill = Fill {
        execution: combo,
        commission: None,
        legs: vec![
            leg_fill("0000e1a7.6b071816.02.01", ESH7, ExecutionSide::Sold, 3.0, "CME"),
            leg_fill("0000e1a7.6b071816.03.01", ESZ6, ExecutionSide::Bought, 2.0, "CME"),
            leg_fill("0000e1a7.6b071816.04.01", ESH7, ExecutionSide::Sold, 1.0, "XCME"),
        ],
    };

    let legs = fill.combo_legs();

    let expected: Vec<ComboLeg> = combo_legs(1, 2)
        .into_iter()
        .map(|leg| ComboLeg {
            exchange: "CME".to_string(),
            ..leg
        })
        .collect();
    assert_eq!(
        legs, expected,
        "one leg per contract, the far leg's 3 + 1 at ratio 2, its first execution's exchange"
    );
}

#[test]
fn combo_legs_ratio_is_zero_when_the_combo_reports_no_shares() {
    let fill = Fill {
        execution: bag("0000e1a7.6b071816.01.01", 0.0),
        commission: None,
        legs: vec![leg_fill("0000e1a7.6b071816.02.01", ESH7, ExecutionSide::Sold, 2.0, "CME")],
    };

    let legs = fill.combo_legs();

    assert_eq!(legs.len(), 1);
    assert_eq!((legs[0].contract_id, legs[0].ratio), (ESH7, 0));
}

#[test]
fn combo_legs_of_a_single_fill_is_empty() {
    let fill = Fill {
        execution: execution("e1"),
        commission: None,
        legs: Vec::new(),
    };

    assert_eq!(fill.combo_legs(), Vec::<ComboLeg>::new());
}
