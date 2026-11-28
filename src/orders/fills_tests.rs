use super::*;
use crate::contracts::SecurityType;
use crate::orders::{Execution, OptionExerciseType};

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
fn expects_commission_report_is_false_only_for_exercise() {
    let cases = [
        (OptionExerciseType::None, true),
        (OptionExerciseType::Exercise, false),
        (OptionExerciseType::Lapse, true),
        (OptionExerciseType::DoNothing, true),
        (OptionExerciseType::Assigned, true),
        (OptionExerciseType::AutoexerciseClearing, true),
        (OptionExerciseType::Expired, true),
        (OptionExerciseType::AutoexerciseTrading, true),
        (OptionExerciseType::Netting, true),
        (OptionExerciseType::Unknown(7), true),
    ];
    for (option_exercise_type, expected) in cases {
        let data = ExecutionData {
            execution: Execution {
                option_exercise_type,
                ..Default::default()
            },
            ..Default::default()
        };
        assert_eq!(data.expects_commission_report(), expected, "{option_exercise_type:?}");
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
    assert!(unpaired.executions.is_empty() && unpaired.reports.is_empty(), "{unpaired:?}");
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
    assert!(unpaired.executions.is_empty() && unpaired.reports.is_empty(), "{unpaired:?}");
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
